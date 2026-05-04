//! 飞书客户端封装
//!
//! 把 `open_lark::client::LarkClient` 包一层，提供：
//! - 统一的 `domain` 字符串到 `base_url` 的映射（`feishu` ↔ `open.feishu.cn`，`lark` ↔ `open.larksuite.com`）；
//! - 凭据非空校验（避免 SDK 内部因为空 `app_id` panic 在 token 缓存路径上）；
//! - 一个稳定的内部句柄 `inner: LarkClient`，给 `identity::probe` 使用。
//!
//! 设计：M2 阶段不持有任何全局 client 实例，每次 `feishu_test_credential` 命令现场构造一个；
//! 等到 M3 接入 WebSocket 长连接时再考虑共享句柄到 `FeishuState`。

use open_lark::client::{LarkClient, LarkClientBuilder};
use open_lark::core::constants::AppType;

use super::error::FeishuError;

/// `https://open.feishu.cn` 的 SDK 默认 base_url（`feishu` 域名）。
const FEISHU_BASE_URL: &str = "https://open.feishu.cn";
/// `https://open.larksuite.com`（海外 `lark` 域名）。
const LARK_BASE_URL: &str = "https://open.larksuite.com";

/// 飞书 SDK 客户端的薄封装。
///
/// `inner` 暴露给同 crate 的 `identity::probe` 直接拿 `client.bot.v3.info.get(None)` 调用。
pub struct FeishuClient {
    pub inner: LarkClient,
}

impl FeishuClient {
    /// 构造一个新的飞书客户端。
    ///
    /// - `app_id` / `app_secret`：飞书自建应用凭据；任一为空时返回 `MissingCredential`。
    /// - `domain`：`"feishu"` 或 `"lark"`；其它值返回 `BadDomain`。
    pub fn new(app_id: &str, app_secret: &str, domain: &str) -> Result<Self, FeishuError> {
        if app_id.trim().is_empty() || app_secret.trim().is_empty() {
            return Err(FeishuError::MissingCredential);
        }
        let base_url = match domain {
            "feishu" => FEISHU_BASE_URL,
            "lark" => LARK_BASE_URL,
            other => return Err(FeishuError::BadDomain(other.to_string())),
        };
        let builder: LarkClientBuilder = LarkClient::builder(app_id, app_secret)
            .with_app_type(AppType::SelfBuild)
            .with_enable_token_cache(true)
            .with_open_base_url(base_url.to_string());
        Ok(Self {
            inner: builder.build(),
        })
    }

    /// 暴露 base_url 用于单元测试 / 调试。
    #[allow(dead_code)]
    pub fn base_url(&self) -> &str {
        &self.inner.config.base_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_with_feishu_domain_uses_open_feishu_cn() {
        let client = FeishuClient::new("cli_xxx", "secret_yyy", "feishu").expect("build ok");
        assert_eq!(client.base_url(), "https://open.feishu.cn");
    }

    #[test]
    fn new_with_lark_domain_uses_open_larksuite_com() {
        let client = FeishuClient::new("cli_xxx", "secret_yyy", "lark").expect("build ok");
        assert_eq!(client.base_url(), "https://open.larksuite.com");
    }

    #[test]
    fn new_with_unknown_domain_returns_bad_domain_error() {
        match FeishuClient::new("cli_xxx", "secret_yyy", "google") {
            Err(FeishuError::BadDomain(d)) => assert_eq!(d, "google"),
            Err(other) => panic!("expected BadDomain, got {}", other),
            Ok(_) => panic!("expected BadDomain error, got Ok"),
        }
    }

    #[test]
    fn new_with_empty_credentials_returns_missing_credential() {
        assert!(matches!(
            FeishuClient::new("", "secret_yyy", "feishu"),
            Err(FeishuError::MissingCredential)
        ));
        assert!(matches!(
            FeishuClient::new("cli_xxx", "   ", "feishu"),
            Err(FeishuError::MissingCredential)
        ));
    }
}
