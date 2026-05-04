//! 飞书消息文案的渲染辅助（M5）
//!
//! 这一层与 IM SDK 解耦，纯字符串处理：
//! - [`build_post_content`]：把 markdown 转成飞书 `post` 富文本的 JSON 字符串（结构见
//!   <https://open.feishu.cn/document/server-docs/im-v1/message/create_json>）。
//! - [`strip_markdown_to_plain_text`]：post 渲染失败时的兜底，把 markdown 标记清理掉。
//! - [`chunk_text`]：单条飞书文本 ≤ 8000 字节，长输出按段落 / 行边界拆成多片，给
//!   `send_text_chunked` 使用。
//!
//! 设计要点：实现"小而完备"，仅覆盖 LLM 输出里最常见的 markdown 语法（# 标题 / **粗体** /
//! 列表 / ```代码块```），其它语法直接以纯文本透传，不追求与 GitHub Flavored Markdown 等价。
//! 解析失败 → 上层直接 fallback 走纯文本路径，不影响业务连续性。

use serde_json::{json, Value};

/// 单条飞书文本消息体的安全上限（字符数，按 char 计而非 byte，避免中文截到一半）。
///
/// 飞书 OAPI 文档说 8000 字节；这里取 4000 字符作为安全裕量，给中文 / emoji 多留点空间。
pub const FEISHU_TEXT_CHAR_LIMIT: usize = 4000;

/// 把 markdown 渲染成飞书 post 富文本 JSON 字符串。
///
/// 返回的字符串可以直接喂给 `CreateMessageRequestBody.content`，需配合 `msg_type="post"`。
/// 本函数不会失败：未识别的语法都按 plain text 透传。
///
/// 形如：
/// ```json
/// {"zh_cn":{"title":"","content":[[{"tag":"text","text":"..."}], ...]}}
/// ```
pub fn build_post_content(markdown: &str) -> String {
    let rows = build_post_rows(markdown);
    let payload = json!({
        "zh_cn": {
            "title": "",
            "content": rows,
        }
    });
    payload.to_string()
}

/// 把 markdown 转成 post.content 的二维 node 数组（仅返回中间结构，方便测试）。
fn build_post_rows(markdown: &str) -> Vec<Vec<Value>> {
    let mut rows: Vec<Vec<Value>> = Vec::new();
    let mut in_code_block = false;

    for raw_line in markdown.lines() {
        // 代码块围栏 ```
        if raw_line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            // 围栏行本身不渲染（保持简洁）
            continue;
        }

        if in_code_block {
            // 代码块内：每行原样作为一行普通文本（飞书 post 没有 monospace node，但保留缩进）
            rows.push(vec![text_node(raw_line, &[])]);
            continue;
        }

        // 标题：# / ## / ### 都映射为 bold（飞书 post 没有 heading 概念）
        if let Some(rest) = strip_heading(raw_line) {
            rows.push(vec![text_node(rest, &["bold"])]);
            continue;
        }

        // 无序列表项：- foo / * foo
        if let Some(rest) = strip_bullet(raw_line) {
            let mut nodes = vec![text_node("• ", &[])];
            nodes.extend(parse_inline(rest));
            rows.push(nodes);
            continue;
        }

        // 普通行（包括空行 → 用空字符串保留段落间距）
        rows.push(parse_inline(raw_line));
    }

    rows
}

fn strip_heading(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    for prefix in ["#### ", "### ", "## ", "# "] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return Some(rest.trim_end());
        }
    }
    None
}

fn strip_bullet(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    for prefix in ["- ", "* ", "+ "] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return Some(rest);
        }
    }
    None
}

/// 解析单行内的 `**bold**` 标记，把行切成若干 text node。
///
/// 不支持嵌套。形如 `a **b** c **d**` → `[a, **b**, c, **d**]` 共 4 段。
/// 不识别其它 inline 语法（`code` / [text](url)），直接当 plain text 透传。
fn parse_inline(line: &str) -> Vec<Value> {
    if line.is_empty() {
        // 空行：post 需要一个 node，否则该 row 等于不存在
        return vec![text_node("", &[])];
    }

    let mut nodes: Vec<Value> = Vec::new();
    let bytes = line.as_bytes();
    let mut i = 0;
    let mut buf_start = 0;
    let mut bold_open = false;

    while i + 1 < bytes.len() {
        if bytes[i] == b'*' && bytes[i + 1] == b'*' {
            // 把 buf_start..i 这一段（按当前样式）落盘
            let chunk = &line[buf_start..i];
            if !chunk.is_empty() {
                let style: &[&str] = if bold_open { &["bold"] } else { &[] };
                nodes.push(text_node(chunk, style));
            }
            bold_open = !bold_open;
            i += 2;
            buf_start = i;
            continue;
        }
        i += 1;
    }
    // tail
    let tail = &line[buf_start..];
    if !tail.is_empty() {
        let style: &[&str] = if bold_open { &["bold"] } else { &[] };
        nodes.push(text_node(tail, style));
    }

    if nodes.is_empty() {
        nodes.push(text_node("", &[]));
    }
    nodes
}

fn text_node(text: &str, style: &[&str]) -> Value {
    if style.is_empty() {
        json!({ "tag": "text", "text": text })
    } else {
        json!({
            "tag": "text",
            "text": text,
            "style": style,
        })
    }
}

/// 把 markdown 简单清洗成纯文本，作为 post 渲染失败时的兜底。
///
/// 策略：
/// - 去掉 markdown 围栏 ``` 行；
/// - 把行首的标题井号 / 列表符号转换成 plain text（列表用 `• ` 替代）；
/// - 把 `**bold**` 的双星号去掉、`` `code` `` 的反引号去掉；
/// - 其它内容原样保留（链接 `[txt](url)` 也直接保留，让用户能读到）。
pub fn strip_markdown_to_plain_text(markdown: &str) -> String {
    let mut out = String::with_capacity(markdown.len());
    let mut in_code_block = false;

    for raw_line in markdown.lines() {
        if raw_line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }

        if in_code_block {
            // 代码块内：原样保留
            out.push_str(raw_line);
            out.push('\n');
            continue;
        }

        // 列表项 → "• ..."
        if let Some(rest) = strip_bullet(raw_line) {
            out.push_str("• ");
            out.push_str(&clean_inline(rest));
            out.push('\n');
            continue;
        }

        // 标题 → 去井号
        let body = strip_heading(raw_line).unwrap_or(raw_line);
        out.push_str(&clean_inline(body));
        out.push('\n');
    }

    while out.ends_with('\n') {
        out.pop();
    }
    out
}

fn clean_inline(s: &str) -> String {
    s.replace("**", "").replace('`', "")
}

/// 把长文本拆成 ≤ `limit` 字符的多片，尽量在段落 / 行边界切。
///
/// - `limit` 以 char 数计；不会切到 char 中间；
/// - 优先在 `\n\n` 处切；其次在 `\n`；最后兜底硬切；
/// - 输入为空或长度 ≤ limit 时，直接返回单元素（即使为空）。
pub fn chunk_text(text: &str, limit: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    if char_count(text) <= limit {
        return vec![text.to_string()];
    }

    let mut chunks: Vec<String> = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        if char_count(remaining) <= limit {
            chunks.push(remaining.to_string());
            break;
        }

        // 取前 limit 字符的 byte 偏移
        let cut_byte = byte_index_at_char(remaining, limit);
        let head = &remaining[..cut_byte];

        // 优先在 head 末尾找最近的 \n\n
        let cut_at = head
            .rfind("\n\n")
            .map(|p| p + 2)
            .or_else(|| head.rfind('\n').map(|p| p + 1))
            .unwrap_or(cut_byte);

        // 防止 cut_at 退化为 0（开头就没找到换行）→ 硬切
        let cut_at = if cut_at == 0 { cut_byte } else { cut_at };

        let (left, right) = remaining.split_at(cut_at);
        let left_trim = left.trim_end_matches(['\n', '\r', ' ']).to_string();
        if !left_trim.is_empty() {
            chunks.push(left_trim);
        }
        remaining = right.trim_start_matches(['\n', '\r', ' ']);
    }

    if chunks.is_empty() {
        vec![text.to_string()]
    } else {
        chunks
    }
}

fn char_count(s: &str) -> usize {
    s.chars().count()
}

fn byte_index_at_char(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(b, _)| b)
        .unwrap_or(s.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_post_rows_handles_plain_paragraphs() {
        let rows = build_post_rows("hello\nworld");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0][0]["text"], "hello");
        assert_eq!(rows[1][0]["text"], "world");
    }

    #[test]
    fn build_post_rows_marks_headings_bold() {
        let rows = build_post_rows("# 标题\n正文");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0][0]["text"], "标题");
        assert_eq!(rows[0][0]["style"][0], "bold");
        assert_eq!(rows[1][0].get("style"), None);
    }

    #[test]
    fn build_post_rows_replaces_bullet_with_dot() {
        let rows = build_post_rows("- item1\n* item2");
        assert_eq!(rows.len(), 2);
        // 第一段是 "• " 前缀
        assert_eq!(rows[0][0]["text"], "• ");
        assert_eq!(rows[0][1]["text"], "item1");
        assert_eq!(rows[1][0]["text"], "• ");
        assert_eq!(rows[1][1]["text"], "item2");
    }

    #[test]
    fn build_post_rows_splits_bold_runs() {
        let rows = build_post_rows("a **b** c");
        // 共 1 row，4 段：("a ", ""), ("b", bold), (" c", "")
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.len(), 3);
        assert_eq!(row[0]["text"], "a ");
        assert_eq!(row[1]["text"], "b");
        assert_eq!(row[1]["style"][0], "bold");
        assert_eq!(row[2]["text"], " c");
    }

    #[test]
    fn build_post_rows_strips_code_fences() {
        // 围栏行不出现，代码块内每行作为普通 row
        let rows = build_post_rows("```rust\nlet x = 1;\n```");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][0]["text"], "let x = 1;");
    }

    #[test]
    fn build_post_content_returns_valid_json() {
        let s = build_post_content("# Title\nbody");
        let v: Value = serde_json::from_str(&s).expect("valid json");
        assert!(v["zh_cn"]["content"].is_array());
        assert_eq!(v["zh_cn"]["title"], "");
    }

    #[test]
    fn strip_markdown_removes_basic_formatting() {
        let plain = strip_markdown_to_plain_text("# 标题\n**强调** `code`\n- a\n- b");
        // 不强求精确等价，只保证关键字符消失
        assert!(!plain.contains("# "));
        assert!(!plain.contains("**"));
        assert!(!plain.contains('`'));
        assert!(plain.contains("• a"));
        assert!(plain.contains("• b"));
    }

    #[test]
    fn strip_markdown_skips_code_fences() {
        let plain = strip_markdown_to_plain_text("```\nlet x = 1;\n```");
        assert_eq!(plain, "let x = 1;");
    }

    #[test]
    fn chunk_text_passthrough_for_short_input() {
        let chunks = chunk_text("hello", 100);
        assert_eq!(chunks, vec!["hello".to_string()]);
    }

    #[test]
    fn chunk_text_handles_empty_string() {
        let chunks = chunk_text("", 100);
        assert_eq!(chunks, vec!["".to_string()]);
    }

    #[test]
    fn chunk_text_splits_on_paragraph_boundary() {
        let text = "para1\n\npara2\n\npara3";
        let chunks = chunk_text(text, 7); // "para1" 5 chars 通过；加上 \n\n=7 边界
        assert!(chunks.len() >= 2);
        // 每片不应包含中间 \n\n（拆分时已 trim）
        for c in &chunks {
            assert!(!c.starts_with('\n'));
            assert!(!c.ends_with('\n'));
        }
        // 拼回去（用 \n\n 重连）应包含原文核心
        let joined = chunks.join("\n\n");
        assert!(joined.contains("para1"));
        assert!(joined.contains("para2"));
        assert!(joined.contains("para3"));
    }

    #[test]
    fn chunk_text_falls_back_to_line_boundary() {
        let text = "line1\nline2\nline3";
        let chunks = chunk_text(text, 7);
        assert!(chunks.len() >= 2);
        let joined = chunks.join("\n");
        assert!(joined.contains("line1"));
        assert!(joined.contains("line3"));
    }

    #[test]
    fn chunk_text_hard_cuts_when_no_newline() {
        let text = "0123456789abcdefghij"; // 20 chars
        let chunks = chunk_text(text, 6);
        assert!(chunks.len() >= 3);
        // 还原应等价（无 trim 损耗，因为没有 \n / 空格）
        let joined = chunks.concat();
        assert_eq!(joined, text);
    }

    #[test]
    fn chunk_text_respects_char_boundary_on_chinese() {
        // 6 个中文字 = 18 bytes；按字符切应等于按字
        let text = "你好世界天气好";
        let chunks = chunk_text(text, 3);
        assert_eq!(chunks.iter().map(|s| char_count(s)).max(), Some(3));
        assert_eq!(chunks.concat(), text);
    }
}
