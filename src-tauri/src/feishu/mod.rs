//! 飞书集成模块（M2：凭据 + 身份探测；不含 WebSocket 连接 / 消息处理）
//!
//! M2 范围：
//! - DTOs：`FeishuConfig` / `FeishuConfigInput` / `FeishuStatus` / `BotIdentity`
//! - 子模块：[`error`] [`client`] [`identity`]
//! - 状态：[`FeishuState`]，目前只持 `RwLock<FeishuStatus>`，M3 接入连接句柄时再扩
//!
//! 不在 M2：连接、事件、入站消息、`/new` 命令、串行锁 / 去重 —— 留给 M3。

pub mod client;
pub mod error;
pub mod identity;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

pub use error::FeishuError;

/// 飞书配置完整记录（含 id / 时间戳，从 DB 读出后给前端）。
///
/// `app_secret` 字段直接序列化，与 `LLMConfig.api_key` 处理方式一致；M2 不做 redact，
/// 但保留 `Debug` 实现避免日志意外打印（手动控制 `eprintln!` 不打印整个对象）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuConfig {
    pub id: i64,
    pub name: String,
    pub app_id: String,
    pub app_secret: String,
    pub domain: String,
    pub bind_llm_config_id: Option<i64>,
    pub bind_role_scope: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// 前端保存配置时使用的入参（无 id / 时间戳）。
///
/// 字段顺序 / 命名与 `FeishuConfig` 对齐；Tauri 2 自动 camelCase ↔ snake_case 映射，
/// 前端用 `appId / appSecret / domain / bindLlmConfigId / bindRoleScope / enabled`。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuConfigInput {
    pub app_id: String,
    pub app_secret: String,
    #[serde(default = "default_domain")]
    pub domain: String,
    #[serde(default)]
    pub bind_llm_config_id: Option<i64>,
    #[serde(default = "default_role_scope")]
    pub bind_role_scope: String,
    #[serde(default)]
    pub enabled: bool,
}

fn default_domain() -> String {
    "feishu".to_string()
}

fn default_role_scope() -> String {
    "analysis".to_string()
}

/// 机器人身份（`feishu_test_credential` 的返回 + 状态字段共用结构）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotIdentity {
    pub open_id: String,
    pub name: String,
}

/// 飞书运行时状态（M2：连接相关字段恒为默认值；M3 接入连接后由 `FeishuState` 维护）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FeishuStatus {
    pub running: bool,
    pub bot_open_id: Option<String>,
    pub bot_name: Option<String>,
    pub last_error: Option<String>,
    pub last_event_at: Option<String>,
    pub connected_since: Option<String>,
}

/// Tauri State：`app.manage(FeishuState::new())`。
///
/// 目前只持有一个状态读写锁。M3 会扩为 `connection: RwLock<Option<ConnectionHandle>>`、
/// `pending_locks: ...` 等字段，但 M2 不预先暴露 API，避免误用。
pub struct FeishuState {
    pub status: RwLock<FeishuStatus>,
}

impl FeishuState {
    pub fn new() -> Self {
        Self {
            status: RwLock::new(FeishuStatus::default()),
        }
    }
}

impl Default for FeishuState {
    fn default() -> Self {
        Self::new()
    }
}
