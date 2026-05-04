//! 飞书机器人身份探测
//!
//! 通过 SDK 调 `GET /open-apis/bot/v3/info` 拿回当前 `app_id/app_secret` 对应的机器人身份信息，
//! 用于"测试连接"按钮：成功表示凭据可用、bot 已在飞书开发者后台启用机器人能力。
//!
//! 响应字段说明（见 SDK [`Bot`]）：
//! - `app_name`：机器人显示名（我们映射为 `BotIdentity.name`）
//! - `open_id`：机器人 open_id（我们映射为 `BotIdentity.open_id`，M3 路由会用到）
//! - `app_status`：`normal/disabled/unknown`，本探测仅在 `disabled` 时给出更友好提示
//! - `avatar_url` / `ip_white_list`：M2 暂不返回前端
//!
//! 这层封装故意把 SDK 的 `LarkAPIError` 全部塌缩成 `FeishuError::Sdk(String)`，对外只暴露
//! `FeishuError`。理由：前端只需要可读字符串，不关心具体业务码；如果后续要展示 retryable 信息，
//! 再在这一层 enrich。

use open_lark::service::bot::models::{AppStatus, Bot};

use super::client::FeishuClient;
use super::error::FeishuError;
use super::BotIdentity;

/// 调 `bot/v3/info`，返回最小化的 `BotIdentity { open_id, name }`。
///
/// 失败场景：
/// - SDK / HTTP 层错误 → `FeishuError::Sdk`
/// - 业务 code != 0（例如 99991663 凭据错误、1011002 未开启机器人能力）→ `FeishuError::Sdk`，
///   错误文案来自 SDK 的 `data_or_error()`
/// - 响应 success 但 data 缺 `open_id` / `app_name` → 仍尽力返回（用空字符串占位），
///   不视为错误，让用户在 UI 上看到"机器人能力已启用但配置不全"的细节
pub async fn probe(client: &FeishuClient) -> Result<BotIdentity, FeishuError> {
    let resp = client
        .inner
        .bot
        .v3
        .info
        .get(None)
        .await
        .map_err(|e| FeishuError::Sdk(e.to_string()))?;

    let info = resp.data_or_error().map_err(FeishuError::Sdk)?;
    let bot: Bot = info.bot;

    // 如果机器人能力被关闭，给一个明确的提示，避免用户拿到一个空的 open_id 后困惑。
    if let Some(AppStatus::Disabled) = bot.app_status {
        return Err(FeishuError::Sdk(
            "机器人状态为 disabled：请到飞书开发者后台开启机器人能力".into(),
        ));
    }

    Ok(BotIdentity {
        open_id: bot.open_id.unwrap_or_default(),
        name: bot.app_name.unwrap_or_default(),
    })
}
