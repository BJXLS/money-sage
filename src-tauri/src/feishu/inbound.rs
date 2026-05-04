//! 飞书 IM 入站事件解析与过滤
//!
//! 负责把 SDK 推过来的 `P2ImMessageReceiveV1` 事件落到一个内部 [`InboundMessage`]：
//! - 把 `{"text":"..."}` 这种文本消息内容解出来；
//! - 暴露过滤入口 [`SkipReason`]，让 dispatcher 把"群消息 / bot 自己发的 / 重复事件 /
//!   非文本"统一丢掉，仅在 debug 日志里留痕；
//! - 不在这一层做 IO（DB / 网络），保持纯函数便于单测。

use open_lark::service::im::v1::p2_im_message_receive_v1::P2ImMessageReceiveV1;
use serde::Deserialize;

use super::dedup::Dedup;

/// 进站消息的最小化结构（其它字段需要时再加）。
#[derive(Debug, Clone)]
pub struct InboundMessage {
    /// 事件 id（可选，仅用于日志）
    pub event_id: Option<String>,
    /// 消息 id（用作去重 key + 出站 reply 的锚点）
    pub message_id: String,
    /// 发送者 open_id
    pub sender_open_id: String,
    /// 会话类型："p2p" / "group"
    pub chat_type: String,
    /// 消息类型："text" / "post" / "image" ...
    pub message_type: String,
    /// 文本内容（仅当 `message_type == "text"` 时为 `Some`，否则为 `None`）
    pub text: Option<String>,
    /// 群组 id（M3 不用，留个字段方便日志）
    #[allow(dead_code)]
    pub chat_id: String,
}

/// 跳过原因。dispatcher 据此决定是否处理。
#[derive(Debug, PartialEq, Eq)]
pub enum SkipReason {
    /// 群消息（M3 仅响应 p2p 单聊）
    NotP2p,
    /// 机器人自己发的消息（避免循环）
    SelfMessage,
    /// 该 message_id 已经处理过（飞书重投递）
    Duplicate,
    /// 非文本消息（M3 仅处理纯文本）
    NotText,
    /// 文本为空（解出来 text 字段缺失或全是空白）
    EmptyText,
}

/// 把 SDK 事件解析为 [`InboundMessage`]。
///
/// 不会失败：始终返回；后续过滤逻辑由 [`classify`] 决定。
pub fn parse_event(event: P2ImMessageReceiveV1) -> InboundMessage {
    let event_id = event.header.event_id;
    let sender_open_id = event.event.sender.sender_id.open_id;
    let msg = event.event.message;
    let text = if msg.message_type == "text" {
        extract_text(&msg.content)
    } else {
        None
    };
    InboundMessage {
        event_id,
        message_id: msg.message_id,
        sender_open_id,
        chat_type: msg.chat_type,
        message_type: msg.message_type,
        text,
        chat_id: msg.chat_id,
    }
}

/// 飞书文本消息的 `content` 是 `{"text":"..."}` 这种 JSON 字符串。
/// 若解析失败或字段缺失，返回 `None`。
pub fn extract_text(content_json: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct TextEnvelope {
        text: Option<String>,
    }
    serde_json::from_str::<TextEnvelope>(content_json)
        .ok()
        .and_then(|env| env.text)
}

/// 决定是否跳过该消息。`bot_open_id` 为机器人自身的 open_id；`dedup` 为去重缓存。
///
/// 副作用：在判定为非重复时会调用 `dedup.mark_seen_or_dup` 把 `message_id` 写入缓存。
pub fn classify(
    msg: &InboundMessage,
    bot_open_id: Option<&str>,
    dedup: &Dedup,
) -> Result<(), SkipReason> {
    if msg.chat_type != "p2p" {
        return Err(SkipReason::NotP2p);
    }
    if let Some(bot_id) = bot_open_id {
        if !bot_id.is_empty() && msg.sender_open_id == bot_id {
            return Err(SkipReason::SelfMessage);
        }
    }
    if msg.message_type != "text" {
        return Err(SkipReason::NotText);
    }
    let txt = match msg.text.as_deref() {
        Some(s) if !s.trim().is_empty() => s,
        _ => return Err(SkipReason::EmptyText),
    };
    let _ = txt; // text 已校验，下面靠 message_id 去重

    if dedup.mark_seen_or_dup(&msg.message_id) {
        return Err(SkipReason::Duplicate);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn fake_msg(message_type: &str, chat_type: &str, text: Option<&str>, mid: &str, sender: &str) -> InboundMessage {
        InboundMessage {
            event_id: Some("e1".into()),
            message_id: mid.into(),
            sender_open_id: sender.into(),
            chat_type: chat_type.into(),
            message_type: message_type.into(),
            text: text.map(|s| s.to_string()),
            chat_id: "c1".into(),
        }
    }

    #[test]
    fn extract_text_handles_normal_payload() {
        assert_eq!(extract_text("{\"text\":\"hi\"}"), Some("hi".into()));
    }

    #[test]
    fn extract_text_returns_none_on_missing_field() {
        assert_eq!(extract_text("{}"), None);
        assert_eq!(extract_text("{\"image_key\":\"abc\"}"), None);
    }

    #[test]
    fn extract_text_returns_none_on_invalid_json() {
        assert_eq!(extract_text("not json"), None);
    }

    #[test]
    fn classify_rejects_group_chat() {
        let dedup = Dedup::new(8, Duration::from_secs(60));
        let m = fake_msg("text", "group", Some("hi"), "m1", "ou_user");
        assert_eq!(classify(&m, Some("ou_bot"), &dedup), Err(SkipReason::NotP2p));
    }

    #[test]
    fn classify_rejects_bot_self_messages() {
        let dedup = Dedup::new(8, Duration::from_secs(60));
        let m = fake_msg("text", "p2p", Some("loop"), "m1", "ou_bot");
        assert_eq!(classify(&m, Some("ou_bot"), &dedup), Err(SkipReason::SelfMessage));
    }

    #[test]
    fn classify_rejects_non_text() {
        let dedup = Dedup::new(8, Duration::from_secs(60));
        let m = fake_msg("image", "p2p", None, "m1", "ou_user");
        assert_eq!(classify(&m, Some("ou_bot"), &dedup), Err(SkipReason::NotText));
    }

    #[test]
    fn classify_rejects_empty_text() {
        let dedup = Dedup::new(8, Duration::from_secs(60));
        let m1 = fake_msg("text", "p2p", None, "m1", "ou_user");
        assert_eq!(classify(&m1, Some("ou_bot"), &dedup), Err(SkipReason::EmptyText));
        let m2 = fake_msg("text", "p2p", Some("   "), "m2", "ou_user");
        assert_eq!(classify(&m2, Some("ou_bot"), &dedup), Err(SkipReason::EmptyText));
    }

    #[test]
    fn classify_dedups_repeated_message_id() {
        let dedup = Dedup::new(8, Duration::from_secs(60));
        let m = fake_msg("text", "p2p", Some("hi"), "m-dup", "ou_user");
        assert!(classify(&m, Some("ou_bot"), &dedup).is_ok());
        assert_eq!(
            classify(&m, Some("ou_bot"), &dedup),
            Err(SkipReason::Duplicate)
        );
    }

    #[test]
    fn classify_passes_when_bot_id_unknown() {
        // bot_open_id 未知（启动早期未加载到）应当不阻塞业务消息
        let dedup = Dedup::new(8, Duration::from_secs(60));
        let m = fake_msg("text", "p2p", Some("hello"), "m1", "ou_user");
        assert!(classify(&m, None, &dedup).is_ok());
    }
}
