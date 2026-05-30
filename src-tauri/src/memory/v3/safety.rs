/// 对即将写入 memory/ 目录的内容进行安全检查
///
/// 扫描提示词注入、SQL 注入、凭证泄露等危险模式。
/// 返回 Ok(()) 表示通过，Err(msg) 表示拒绝。
pub fn scan_memory_write(content: &str) -> Result<(), String> {
    let lower = content.to_lowercase();

    // 1. 提示词注入模式
    let injection_patterns = [
        "ignore previous instructions",
        "do not tell the user",
        "<system>",
        "</memory-context>",
        "you are now",
        "from now on you are",
        "forget all previous",
        "bypass",
        "developer mode",
        "dan mode",
        "越狱",
        "开发者模式",
    ];
    for pat in &injection_patterns {
        if lower.contains(pat) {
            return Err(format!("检测到潜在的提示词注入模式: '{}'", pat));
        }
    }

    // 2. 凭证泄露模式
    let secret_patterns = [
        "sk-",          // OpenAI API key
        "api_key:",
        "apikey:",
        "password:",
        "secret:",
        "token:",
        "cat .env",     // 命令行读取 .env 的上下文模式
    ];
    for pat in &secret_patterns {
        if lower.contains(pat) {
            return Err(format!("检测到潜在的凭证泄露模式: '{}'", pat));
        }
    }

    // 4. 不可见 Unicode 字符（零宽字符等）
    let invisible_chars: [char; 4] = ['\u{200B}', '\u{200C}', '\u{200D}', '\u{FEFF}'];
    for ch in &invisible_chars {
        if content.contains(*ch) {
            return Err(format!("检测到不可见 Unicode 字符: U+{:04X}", *ch as u32));
        }
    }

    Ok(())
}
