//! 飞书 WebSocket 长连接生命周期
//!
//! 这一层是 Tauri 命令 `feishu_start` / `feishu_stop` 的实质实现：
//! - [`start`]：构造 [`FeishuClient`] → 探测机器人身份 → 起一条独立 OS 线程，线程内
//!   构造 dispatcher 并跑 `LarkWsClient::open` → 把 oneshot shutdown sender 存进
//!   `FeishuState.connection` → 更新 `FeishuStatus`；
//! - [`stop`]：通过 oneshot 通知线程退出 + 重置 [`FeishuStatus`] 为 `running=false`。
//!
//! ## 为什么用独立 OS 线程而不是 `tokio::spawn`
//!
//! `open_lark::event::dispatcher::EventDispatcherHandler` 内部是
//! `HashMap<String, Box<dyn EventHandler>>`，trait 上没有 `Send + Sync` bound，导致
//! `EventDispatcherHandler` 既不是 `Send` 也不是 `Sync`：
//! - 不能 `tokio::spawn`（多线程 runtime 要求 future `Send`）；
//! - 不能跨 `std::thread::spawn` 边界（thread `spawn` 也要 `Send`）。
//!
//! 解决：把 dispatcher 的构造移到目标 OS 线程内部，使它"出生"就在那个线程上。
//! `build_dispatcher` 的入参（`Database` / `Arc<MemoryFacade>` / `Arc<...>`）都是
//! `Send + Sync`，可以安全 move 进 thread closure；dispatcher 构造完后只在当前线程上被使用。
//!
//! 实际网络循环跑在 OS 线程的 `current_thread` runtime 里；`stop` 时通过 oneshot 信号让
//! `tokio::select!` 早退即可让 dispatcher 被 drop、WebSocket 关闭，线程随之退出。

use std::sync::Arc;

use chrono::Utc;
use open_lark::client::ws_client::LarkWsClient;
use tokio::sync::oneshot;

use crate::database::Database;
use crate::mcp::McpManager;
use crate::memory::MemoryFacade;
use crate::telemetry::TokenUsageRecorder;

use super::client::FeishuClient;
use super::dispatcher::build_dispatcher;
use super::error::FeishuError;
use super::identity;
use super::{FeishuConfig, FeishuState};

/// 启动飞书 WebSocket 连接。重复调用会先停掉旧连接。
///
/// 流程：
/// 1. 若已有活跃连接，先 [`stop`]；
/// 2. 用 `config.app_id/app_secret/domain` 构造 [`FeishuClient`]，调 `identity::probe` 拿
///    `bot_open_id` / `bot_name`（失败立即 `Err`，不入连接循环）；
/// 3. 起一条独立 OS 线程：把 db / memory / mcp_manager / bot_client 等 `Send + Sync` 的状态
///    move 进去，线程内构造 dispatcher 后立即调 `LarkWsClient::open`；
/// 4. 把 oneshot sender 写入 `state.connection`；线程退出时会更新 `state.status`。
#[allow(clippy::too_many_arguments)]
pub async fn start(
    state: &FeishuState,
    config: &FeishuConfig,
    db: Database,
    memory: Arc<MemoryFacade>,
    token_recorder: Arc<TokenUsageRecorder>,
    mcp_manager: McpManager,
) -> Result<(), FeishuError> {
    // 1. 先停掉旧连接（best effort）
    stop(state).await;

    // 2. 构造 SDK 客户端 + 探测身份（在调用线程上做，失败立即返回前端）
    let bot_client = Arc::new(FeishuClient::new(
        &config.app_id,
        &config.app_secret,
        &config.domain,
    )?);
    let identity_info = identity::probe(&bot_client).await?;

    // 3. 起一条独立 OS 线程跑长连接（dispatcher 在线程内构造，避免跨线程 send）
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
    let status_arc = state.status.clone();
    let locks = state.locks.clone();
    let dedup = state.dedup.clone();
    let bot_open_id = identity_info.open_id.clone();
    let bot_name_for_log = identity_info.name.clone();
    let bot_open_id_for_log = identity_info.open_id.clone();
    let bind_llm_id = config.bind_llm_config_id;

    std::thread::Builder::new()
        .name("feishu-ws".into())
        .spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!(
                        "[feishu connection] 无法构造 current_thread runtime: {}",
                        e
                    );
                    // 无法跑 async 写状态，靠一个临时多线程 runtime 把 status 翻回去
                    if let Ok(aux) = tokio::runtime::Runtime::new() {
                        aux.block_on(async move {
                            let mut s = status_arc.write().await;
                            s.running = false;
                            s.connected_since = None;
                            s.last_error = Some(format!("runtime build error: {}", e));
                        });
                    }
                    return;
                }
            };

            rt.block_on(async move {
                // 在线程内构造 dispatcher：dispatcher 是 !Send/!Sync，但永远不离开本线程
                let dispatcher = match build_dispatcher(
                    db,
                    memory,
                    token_recorder,
                    mcp_manager,
                    bot_client.clone(),
                    locks,
                    dedup,
                    Some(bot_open_id.clone()),
                    bind_llm_id,
                ) {
                    Ok(d) => d,
                    Err(e) => {
                        eprintln!("[feishu connection] 构造 dispatcher 失败: {}", e);
                        let mut s = status_arc.write().await;
                        s.running = false;
                        s.connected_since = None;
                        s.last_error = Some(format!("dispatcher build error: {}", e));
                        return;
                    }
                };

                let ws_config = Arc::new(bot_client.inner.config.clone());

                let exit_reason = tokio::select! {
                    res = LarkWsClient::open(ws_config, dispatcher) => match res {
                        Ok(()) => Ok(()),
                        Err(e) => Err(format!("WebSocket 连接退出: {}", e)),
                    },
                    _ = shutdown_rx => {
                        // 用户调 stop / 重启时通过 oneshot 通知；正常退出
                        Ok(())
                    }
                };

                let mut status = status_arc.write().await;
                status.running = false;
                status.connected_since = None;
                match exit_reason {
                    Ok(()) => {
                        eprintln!(
                            "[feishu connection] WebSocket 已退出 (bot_open_id={}, name={})",
                            bot_open_id_for_log, bot_name_for_log
                        );
                    }
                    Err(msg) => {
                        eprintln!("[feishu connection] {}", msg);
                        status.last_error = Some(msg);
                    }
                }
            });
        })
        .map_err(|e| FeishuError::Sdk(format!("无法 spawn feishu-ws 线程: {}", e)))?;

    // 4. 存任务句柄 + 写入状态
    {
        let mut conn = state.connection.write().await;
        *conn = Some(shutdown_tx);
    }
    {
        let mut status = state.status.write().await;
        status.running = true;
        status.bot_open_id = Some(identity_info.open_id);
        status.bot_name = Some(identity_info.name);
        status.last_error = None;
        status.connected_since = Some(Utc::now().to_rfc3339());
    }

    Ok(())
}

/// 停止飞书 WebSocket 连接（无连接时是 no-op）。
///
/// 通过 oneshot send 信号；不等待线程实际退出（线程几毫秒内会被 select! 唤醒并 drop dispatcher）。
pub async fn stop(state: &FeishuState) {
    let sender_opt = {
        let mut conn = state.connection.write().await;
        conn.take()
    };
    if let Some(sender) = sender_opt {
        // 接收端可能已经 drop（线程已自行退出）；忽略 SendError。
        let _ = sender.send(());
    }
    let mut status = state.status.write().await;
    status.running = false;
    status.connected_since = None;
    // 保留 bot_open_id / bot_name 不清，便于 UI 显示"上次连接的 bot"；
    // 用户重启或修改配置后下次 start 会覆盖。
}
