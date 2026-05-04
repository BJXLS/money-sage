//! 飞书出站消息（M3：仅文本）
//!
//! 两层 API：
//! - [`send_text`]：直接调 SDK 把一条文本消息发给指定 `open_id`，给 dispatcher 错误回执
//!   / `/new` 提示这种"非流水线"的固定文案使用；
//! - [`FeishuSink`]：实现 [`MessageSink`]，把流水线事件桥接到飞书 IM。M3 简化策略：
//!     * `on_chunk` / `on_tool_status` 不发消息，避免刷屏（飞书 IM 没有 SSE 体验，每个分片
//!        都发会导致几十条短消息，工具调用细节也不适合给最终用户看）；
//!     * `on_done` 把 `full_text` 整段发给用户；
//!     * `on_error` 把错误文本作为一条普通消息发回。
//!
//! 后续 M5 如果想做"边写边发"，可以在 sink 里维护一个 batching buffer + update_message API；
//! M3 优先把端到端跑通。

use std::sync::Arc;

use async_trait::async_trait;
use open_lark::service::im::v1::message::builders::{
    CreateMessageRequest, CreateMessageRequestBody,
};

use crate::pipeline::sink::{MessageSink, ToolStatus};

use super::client::FeishuClient;
use super::error::FeishuError;

/// 把一段纯文本作为新消息发给某个 `open_id`。
///
/// 失败原因：网络 / SDK / 业务 code != 0 → `FeishuError::Sdk`。调用方应记日志但不要崩溃。
pub async fn send_text(
    client: &FeishuClient,
    open_id: &str,
    text: &str,
) -> Result<(), FeishuError> {
    if text.is_empty() {
        return Ok(());
    }
    // 飞书文本消息内容是个 JSON 字符串，注意转义换行 / 引号
    let content = serde_json::json!({ "text": text }).to_string();

    let req = CreateMessageRequest::builder()
        .receive_id_type("open_id")
        .request_body(
            CreateMessageRequestBody {
                receive_id: open_id.to_string(),
                msg_type: "text".to_string(),
                content,
                ..Default::default()
            },
        )
        .build();

    client
        .inner
        .im
        .v1
        .message
        .create(req, None)
        .await
        .map_err(|e| FeishuError::Sdk(e.to_string()))?;
    Ok(())
}

/// 把流水线事件桥接到飞书 IM 的 sink。
///
/// 由 [`pipeline::analysis::run`] 持有引用驱动；本 struct 自身只持 `Arc<FeishuClient>` + 目标
/// `open_id`，不持任何状态，跨 await 安全。
pub struct FeishuSink {
    client: Arc<FeishuClient>,
    open_id: String,
}

impl FeishuSink {
    pub fn new(client: Arc<FeishuClient>, open_id: String) -> Self {
        Self { client, open_id }
    }
}

#[async_trait]
impl MessageSink for FeishuSink {
    async fn on_chunk(&self, _chunk: &str) {
        // M3 简化：流式分片不实时发飞书，避免刷屏。整段在 on_done 一并发送。
    }

    async fn on_tool_status(&self, _status: ToolStatus) {
        // M3 不把工具调用细节透给最终用户。
    }

    async fn on_done(&self, full_text: &str) {
        let trimmed = full_text.trim();
        if trimmed.is_empty() {
            return;
        }
        if let Err(e) = send_text(&self.client, &self.open_id, trimmed).await {
            eprintln!("[feishu sink] 发送 done 消息失败 (open_id={}): {}", self.open_id, e);
        }
    }

    async fn on_error(&self, message: &str) {
        let body = format!("⚠️ 处理失败：{}", message);
        if let Err(e) = send_text(&self.client, &self.open_id, &body).await {
            eprintln!(
                "[feishu sink] 发送 error 消息失败 (open_id={}): {}",
                self.open_id, e
            );
        }
    }
}
