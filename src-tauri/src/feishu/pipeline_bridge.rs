//! 把"飞书入站消息"接到"流水线 + 飞书出站"的桥
//!
//! 这一层是 dispatcher 之后、`pipeline::analysis::run` 之前的胶水：
//! 1. 取该用户 (open_id) 的串行锁，避免同一用户消息并发；
//! 2. 解析命令：`/new` 切会话、`/new <text>` 切完后把 text 作为本轮 user 消息；
//! 3. 路由到 session_id（已有则复用，否则新建）；
//! 4. 用 [`FeishuSink`] 跑 [`pipeline::analysis::run`]，标 `source = SessionSource::Feishu`；
//! 5. 处理完毕后 touch `last_message_at`。
//!
//! 失败处理：所有 IO / SDK 错误都通过 [`outbound::send_text`] 给用户回一条提示，不要 panic。

use std::sync::Arc;

use crate::database::Database;
use crate::mcp::McpManager;
use crate::memory::MemoryFacade;
use crate::models::SessionSource;
use crate::pipeline::{self, AnalysisInput};
use crate::telemetry::TokenUsageRecorder;

use super::client::FeishuClient;
use super::commands::{try_parse_command, Command};
use super::inbound::InboundMessage;
use super::locks::LockMap;
use super::outbound::{send_text, FeishuSink};
use super::routing;

/// 入站消息的处理入口。dispatcher 在 `tokio::spawn` 里调用本函数。
///
/// `config_id` 来自 `feishu_configs.bind_llm_config_id`：若为 `Some`，流水线优先用该 LLM；
/// 若为 `None`，流水线 fallback 到 `is_active=1` 的全局 LLM。
pub async fn handle_inbound(
    db: &Database,
    memory: &Arc<MemoryFacade>,
    token_recorder: &Arc<TokenUsageRecorder>,
    mcp_manager: &McpManager,
    bot_client: Arc<FeishuClient>,
    locks: Arc<LockMap>,
    msg: InboundMessage,
    config_id: Option<i64>,
) {
    // 1. 取串行锁
    let _guard = locks.lock_for(&msg.sender_open_id).await;

    let text = match msg.text.as_deref() {
        Some(s) => s,
        None => return, // dispatcher 已过滤，不应到这；防御性处理
    };

    let open_id = msg.sender_open_id.clone();

    // 2. 命令解析
    let (session_id, user_message) = match try_parse_command(text) {
        Some(Command::New { initial }) => {
            // /new：新建会话
            let new_sid = match routing::new_session(db, &open_id, None).await {
                Ok(sid) => sid,
                Err(e) => {
                    let _ = send_text(&bot_client, &open_id, &format!("⚠️ 新建会话失败：{}", e)).await;
                    return;
                }
            };
            if initial.is_empty() {
                // /new 单独 → 只发回执，不跑流水线
                let _ = send_text(
                    &bot_client,
                    &open_id,
                    "✅ 已开启新会话，您的下一条消息会进入新对话。",
                )
                .await;
                return;
            }
            (new_sid, initial)
        }
        Some(Command::Help) => {
            let _ = send_text(
                &bot_client,
                &open_id,
                "可用命令：\n  /new            开启新会话\n  /new <内容>     开启新会话并直接发出第一条消息",
            )
            .await;
            return;
        }
        Some(Command::Status) => {
            let summary = match db.get_feishu_user_session(&open_id).await {
                Ok(Some(row)) => format!(
                    "当前会话：{}\n最近一条：{}",
                    row.current_session_id,
                    row.last_message_at.unwrap_or_else(|| "(无)".into())
                ),
                Ok(None) => "尚未建立会话，请直接发消息或 /new 开启。".into(),
                Err(e) => format!("⚠️ 读取状态失败：{}", e),
            };
            let _ = send_text(&bot_client, &open_id, &summary).await;
            return;
        }
        None => {
            // 普通对话：复用已有 session_id 或新建
            let sid = match routing::resolve_or_create(db, &open_id, None).await {
                Ok(sid) => sid,
                Err(e) => {
                    let _ = send_text(&bot_client, &open_id, &format!("⚠️ 路由会话失败：{}", e)).await;
                    return;
                }
            };
            (sid, text.to_string())
        }
    };

    // 3. 跑流水线（飞书 sink）
    let sink = FeishuSink::new(bot_client.clone(), open_id.clone());
    let input = AnalysisInput {
        session_id,
        user_message,
        config_id,
        source: SessionSource::Feishu,
    };
    let _ = pipeline::run_analysis(db, memory, token_recorder, mcp_manager, input, &sink).await;

    // 4. 更新 last_message_at（best effort）
    if let Err(e) = routing::touch(db, &open_id).await {
        eprintln!("[feishu bridge] touch last_message_at 失败 (open_id={}): {}", open_id, e);
    }
}
