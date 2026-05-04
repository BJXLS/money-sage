//! 飞书 IM 事件 dispatcher
//!
//! 把 [`open_lark::event::dispatcher::EventDispatcherHandler`] 与本 crate 的
//! [`pipeline_bridge::handle_inbound`] 接起来：
//! 1. 在 SDK 注册一个同步闭包（要求 `Fn(P2ImMessageReceiveV1) + 'static + Sync + Send`）；
//! 2. 闭包内**不做任何阻塞 / IO**，只把事件 [`inbound::parse_event`] 解析后通过
//!    [`inbound::classify`] 过滤；
//! 3. 通过 `tokio::spawn` 把后续重活（DB / LLM / 出站）扔到独立任务里跑，避免阻塞 SDK
//!    的 IM 主线程。
//!
//! 因为闭包是 `Fn`（不是 `FnOnce`），所有捕获必须实现 `Clone`：
//! - `Database` / `McpManager` 自定义 `impl Clone`（内部都是 `Arc<...>`）
//! - 其它共享状态走 `Arc<...>` 包装

use std::sync::Arc;

use open_lark::event::dispatcher::EventDispatcherHandler;
use open_lark::service::im::v1::p2_im_message_receive_v1::P2ImMessageReceiveV1;

use crate::database::Database;
use crate::mcp::McpManager;
use crate::memory::MemoryFacade;
use crate::telemetry::TokenUsageRecorder;

use super::client::FeishuClient;
use super::dedup::Dedup;
use super::inbound::{self, SkipReason};
use super::locks::LockMap;
use super::pipeline_bridge;

/// 创建一个绑定好 `p2.im.message.receive_v1` 处理器的 [`EventDispatcherHandler`]。
///
/// 参数语义与 [`pipeline_bridge::handle_inbound`] 一致；多出的两个：
/// - `bot_open_id`：用于过滤 bot 自己发出的消息，避免循环；M3 启动阶段先 `identity::probe`
///   拿到再传进来；如果传 `None`，过滤会跳过自身校验（仍由 SDK 保证不该收到自己的消息）。
/// - `config_id`：飞书配置上 `bind_llm_config_id` 的镜像，传给流水线决定走哪个 LLM 模板。
#[allow(clippy::too_many_arguments)]
pub fn build_dispatcher(
    db: Database,
    memory: Arc<MemoryFacade>,
    token_recorder: Arc<TokenUsageRecorder>,
    mcp_manager: McpManager,
    bot_client: Arc<FeishuClient>,
    locks: Arc<LockMap>,
    dedup: Arc<Dedup>,
    bot_open_id: Option<String>,
    config_id: Option<i64>,
) -> Result<EventDispatcherHandler, String> {
    let handler = EventDispatcherHandler::builder()
        .register_p2_im_message_receive_v1(move |event: P2ImMessageReceiveV1| {
            // 解析为我们自己的内部结构
            let msg = inbound::parse_event(event);

            // 过滤：群聊 / bot 自己 / 非文本 / 重复事件 → 直接 return
            match inbound::classify(&msg, bot_open_id.as_deref(), &dedup) {
                Ok(()) => {}
                Err(reason) => {
                    log_skip(&msg, reason);
                    return;
                }
            }

            // 拷贝所有跨任务状态，注入新 spawn 的 async task
            let db = db.clone();
            let memory = memory.clone();
            let token_recorder = token_recorder.clone();
            let mcp_manager = mcp_manager.clone();
            let bot_client = bot_client.clone();
            let locks = locks.clone();
            let cfg_id = config_id;

            tokio::spawn(async move {
                pipeline_bridge::handle_inbound(
                    &db,
                    &memory,
                    &token_recorder,
                    &mcp_manager,
                    bot_client,
                    locks,
                    msg,
                    cfg_id,
                )
                .await;
            });
        })
        .map_err(|e| format!("注册 IM 事件处理器失败: {}", e))?
        .build();
    Ok(handler)
}

fn log_skip(msg: &inbound::InboundMessage, reason: SkipReason) {
    eprintln!(
        "[feishu dispatcher] 跳过消息 (event_id={:?}, message_id={}, sender={}, chat_type={}, message_type={}): {:?}",
        msg.event_id, msg.message_id, msg.sender_open_id, msg.chat_type, msg.message_type, reason
    );
}
