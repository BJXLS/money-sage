//! 飞书集成错误类型
//!
//! 在 M2 阶段只覆盖凭据 / SDK / DB / 域三类错误。M3 接入 WebSocket 后会再加变体（如
//! `Connection`、`Dispatch`）。所有错误都派生 `Debug` + 实现 `Display`，便于直接 `to_string()` 给前端。

use std::fmt;

#[derive(Debug)]
pub enum FeishuError {
    /// 调用 SDK / OAPI 失败（含网络、HTTP 状态、业务 code != 0）
    Sdk(String),
    /// 凭据缺失（app_id / app_secret 为空等）
    MissingCredential,
    /// 不识别的 domain（仅支持 "feishu" / "lark"）
    BadDomain(String),
    /// 数据库相关错误
    Db(String),
}

impl fmt::Display for FeishuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FeishuError::Sdk(msg) => write!(f, "飞书 SDK 调用失败: {}", msg),
            FeishuError::MissingCredential => write!(f, "缺少必要的飞书凭据 (app_id / app_secret)"),
            FeishuError::BadDomain(d) => {
                write!(f, "未知的飞书 domain: '{}'，仅支持 'feishu' 或 'lark'", d)
            }
            FeishuError::Db(msg) => write!(f, "飞书配置数据库错误: {}", msg),
        }
    }
}

impl std::error::Error for FeishuError {}

impl From<sqlx::Error> for FeishuError {
    fn from(e: sqlx::Error) -> Self {
        FeishuError::Db(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_format_is_human_readable() {
        let cases = [
            (
                FeishuError::Sdk("token 过期".into()),
                "飞书 SDK 调用失败: token 过期",
            ),
            (FeishuError::MissingCredential, "缺少必要的飞书凭据"),
            (
                FeishuError::BadDomain("xxx".into()),
                "未知的飞书 domain: 'xxx'",
            ),
            (FeishuError::Db("disk i/o".into()), "飞书配置数据库错误: disk i/o"),
        ];
        for (err, expected_substr) in cases {
            let s = err.to_string();
            assert!(
                s.contains(expected_substr),
                "expected '{}' to contain '{}'",
                s,
                expected_substr
            );
        }
    }
}
