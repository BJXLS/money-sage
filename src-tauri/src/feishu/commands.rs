//! 飞书入站文本的命令解析
//!
//! 设计文档 §8.4：仅识别白名单命令（`/new` / `/help` / `/status`），未匹配则视为正常对话。
//! 这是为了避免用户随手敲一个 `/` 开头的内容（比如代码片段）被误识别为命令。

/// 已识别的命令。M3 仅落地 `/new`，其他命令保留枚举但 dispatcher 暂时不处理。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// `/new` 切换到新会话；可携带正文 `/new <message>`，由 dispatcher 把 `initial`
    /// 当作进入新会话后的第一条用户消息发给流水线。
    New { initial: String },
    /// `/help` 展示可用命令（M3 暂未启用）。
    #[allow(dead_code)]
    Help,
    /// `/status` 展示当前会话信息（M3 暂未启用）。
    #[allow(dead_code)]
    Status,
}

/// 白名单解析：
/// - 必须以 `/` 开头；
/// - 命令名取首个空白前的子串（不含 `/`）；
/// - 命令名必须命中白名单，否则返回 `None`（视为普通对话）；
/// - 剩余部分作为参数（`/new` 用作 initial 文本）。
pub fn try_parse_command(text: &str) -> Option<Command> {
    let trimmed = text.trim();
    let mut iter = trimmed.splitn(2, char::is_whitespace);
    let head = iter.next()?;
    if !head.starts_with('/') {
        return None;
    }
    let name = &head[1..];
    let rest = iter.next().unwrap_or("").trim();
    match name {
        "new" => Some(Command::New {
            initial: rest.to_string(),
        }),
        "help" => Some(Command::Help),
        "status" => Some(Command::Status),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_or_plain_text_is_not_a_command() {
        assert!(try_parse_command("").is_none());
        assert!(try_parse_command("hello").is_none());
        assert!(try_parse_command("查询本月支出").is_none());
    }

    #[test]
    fn slash_new_without_payload_returns_empty_initial() {
        match try_parse_command("/new") {
            Some(Command::New { initial }) => assert_eq!(initial, ""),
            other => panic!("expected Command::New, got {:?}", other),
        }
    }

    #[test]
    fn slash_new_with_payload_captures_initial() {
        match try_parse_command("/new 这个月吃饭花了多少") {
            Some(Command::New { initial }) => assert_eq!(initial, "这个月吃饭花了多少"),
            other => panic!("expected Command::New with initial, got {:?}", other),
        }
    }

    #[test]
    fn slash_new_strips_extra_whitespace_in_payload() {
        match try_parse_command("  /new   hello world  ") {
            Some(Command::New { initial }) => assert_eq!(initial, "hello world"),
            other => panic!("unexpected: {:?}", other),
        }
    }

    #[test]
    fn unknown_slash_command_is_treated_as_plain_text() {
        // 用户在代码片段里贴 `/path/to/file` 应当被当作正常对话
        assert!(try_parse_command("/path/to/foo.rs").is_none());
        assert!(try_parse_command("/random unrecognized").is_none());
    }

    #[test]
    fn slash_help_and_status_recognized() {
        assert_eq!(try_parse_command("/help"), Some(Command::Help));
        assert_eq!(try_parse_command("/status"), Some(Command::Status));
    }
}
