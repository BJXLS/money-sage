//! 飞书出站消息（M5：post 富文本 + 长消息切片 + 表情反馈）
//!
//! 三层 API：
//! - [`send_text`] / [`send_text_chunked`]：纯文本消息。`send_text_chunked` 会按
//!   `markdown::FEISHU_TEXT_CHAR_LIMIT` 把超长文本切成多片分别发送，给 `/help` `/status`
//!   这种短文案 + 错误回执使用；
//! - [`send_post`]：先把 markdown 渲染成 post 富文本发送，**任何失败 fallback 到
//!   `send_text_chunked` + `strip_markdown_to_plain_text`**，确保业务消息不会因渲染异常丢失；
//! - [`add_reaction`]：在用户原消息上加一个 emoji 反馈（最佳努力，失败仅日志）。
//!
//! [`FeishuSink`] 把流水线事件桥接到飞书 IM：
//! - `on_chunk` / `on_tool_status` 不实时刷消息（飞书 IM 没有 SSE 体验，按 chunk 发会刷屏）；
//! - `on_done`：调 [`send_post`] 把 markdown 输出整段发回；
//! - `on_error`：调 [`send_text_chunked`] 把错误文本作为一条消息发回 + 加 [`EMOJI_ERR`] reaction。
//!
//! reaction 的"开始"反馈不在本文件触发，由 [`pipeline_bridge`] 在 spawn 流水线之前直接调
//! [`add_reaction`]，避免在 sink 内部维护"是否已发出 start reaction"的状态。

use std::sync::Arc;

use async_trait::async_trait;
use open_lark::service::im::v1::message::builders::{
    CreateMessageRequest, CreateMessageRequestBody,
};

use crate::pipeline::sink::{MessageSink, ToolStatus};

use super::client::FeishuClient;
use super::error::FeishuError;
use super::markdown::{
    build_post_content, chunk_text, strip_markdown_to_plain_text, FEISHU_TEXT_CHAR_LIMIT,
};

/// 标准 Feishu emoji_type 字符串：开始处理（👀）。
pub const EMOJI_START: &str = "EYES";
/// 标准 Feishu emoji_type 字符串：处理成功（👌）。
#[allow(dead_code)]
pub const EMOJI_DONE: &str = "OK";
/// 标准 Feishu emoji_type 字符串：处理失败（❓）。
pub const EMOJI_ERR: &str = "QUESTION";

/// 把一段纯文本作为一条消息发给某个 `open_id`（不分片，仅供 < 8000 字符的固定文案使用）。
pub async fn send_text(
    client: &FeishuClient,
    open_id: &str,
    text: &str,
) -> Result<(), FeishuError> {
    if text.is_empty() {
        return Ok(());
    }
    let content = serde_json::json!({ "text": text }).to_string();
    let req = CreateMessageRequest::builder()
        .receive_id_type("open_id")
        .request_body(CreateMessageRequestBody {
            receive_id: open_id.to_string(),
            msg_type: "text".to_string(),
            content,
            ..Default::default()
        })
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

/// 把任意长度的纯文本切片后发给某个 `open_id`，每片 ≤ [`FEISHU_TEXT_CHAR_LIMIT`] 字符。
///
/// 任意一片失败立即返回 Err；前面已发出去的不会回滚（飞书消息一旦发送即不可撤回）。
pub async fn send_text_chunked(
    client: &FeishuClient,
    open_id: &str,
    text: &str,
) -> Result<(), FeishuError> {
    let chunks = chunk_text(text, FEISHU_TEXT_CHAR_LIMIT);
    for chunk in chunks {
        if chunk.is_empty() {
            continue;
        }
        send_text(client, open_id, &chunk).await?;
    }
    Ok(())
}

/// 把 markdown 内容渲染成飞书 post 富文本发送；任何失败 fallback 到 `send_text_chunked`。
///
/// fallback 触发场景：
/// - post JSON 序列化异常；
/// - SDK 调用失败（4xx/5xx/网络）；
/// - post 内容超出 OAPI 限制（飞书 post body 上限远高于 8000 字符，此情况罕见）。
pub async fn send_post(
    client: &FeishuClient,
    open_id: &str,
    markdown: &str,
) -> Result<(), FeishuError> {
    if markdown.trim().is_empty() {
        return Ok(());
    }

    let content = build_post_content(markdown);
    let req = CreateMessageRequest::builder()
        .receive_id_type("open_id")
        .request_body(CreateMessageRequestBody {
            receive_id: open_id.to_string(),
            msg_type: "post".to_string(),
            content,
            ..Default::default()
        })
        .build();

    match client.inner.im.v1.message.create(req, None).await {
        Ok(_) => Ok(()),
        Err(e) => {
            eprintln!(
                "[feishu outbound] post 发送失败，fallback 到纯文本 (open_id={}): {}",
                open_id, e
            );
            let plain = strip_markdown_to_plain_text(markdown);
            send_text_chunked(client, open_id, &plain).await
        }
    }
}

/// 在指定消息上加一个 emoji 反馈（最佳努力，失败仅打印日志）。
///
/// 这是一种轻量的"已读 / 已处理"反馈：用户在飞书里看到自己消息上多了一个 reaction
/// 即知道 bot 已经接收 / 已经完成。
pub async fn add_reaction(client: &FeishuClient, message_id: &str, emoji_type: &str) {
    if message_id.is_empty() || emoji_type.is_empty() {
        return;
    }
    if let Err(e) = client
        .inner
        .im
        .v1
        .message_reaction
        .create(message_id, emoji_type, None, None)
        .await
    {
        eprintln!(
            "[feishu outbound] 添加 reaction 失败 (message_id={}, emoji={}): {}",
            message_id, emoji_type, e
        );
    }
}

/// 把流水线事件桥接到飞书 IM 的 sink。
///
/// 持有：
/// - `client`：发送 OAPI；
/// - `open_id`：消息接收人；
/// - `inbound_message_id`：用户原始消息的 message_id，用于 `on_error` 时叠加 ❓ 反馈，让用户
///   不必读完错误文本就能在原消息上看到失败标识。
pub struct FeishuSink {
    client: Arc<FeishuClient>,
    open_id: String,
    inbound_message_id: String,
}

impl FeishuSink {
    pub fn new(client: Arc<FeishuClient>, open_id: String, inbound_message_id: String) -> Self {
        Self {
            client,
            open_id,
            inbound_message_id,
        }
    }
}

#[async_trait]
impl MessageSink for FeishuSink {
    async fn on_chunk(&self, _chunk: &str) {
        // M5 仍不做边写边发：飞书 IM 没有 SSE 体验，每个分片都发会刷屏。
        // M6 才考虑 update_message + debounce。
    }

    async fn on_tool_status(&self, _status: ToolStatus) {
        // 工具调用细节不透给最终用户。
    }

    async fn on_done(&self, full_text: &str) {
        let trimmed = full_text.trim();
        if trimmed.is_empty() {
            return;
        }
        if let Err(e) = send_post(&self.client, &self.open_id, trimmed).await {
            eprintln!(
                "[feishu sink] 发送 done 消息失败 (open_id={}): {}",
                self.open_id, e
            );
        }
    }

    async fn on_error(&self, message: &str) {
        let body = format!("⚠️ 处理失败：{}", message);
        if let Err(e) = send_text_chunked(&self.client, &self.open_id, &body).await {
            eprintln!(
                "[feishu sink] 发送 error 消息失败 (open_id={}): {}",
                self.open_id, e
            );
        }
        add_reaction(&self.client, &self.inbound_message_id, EMOJI_ERR).await;
    }
}
