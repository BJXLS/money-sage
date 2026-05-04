//! 飞书集成模块
//!
//! M2 范围：
//! - DTOs：`FeishuConfig` / `FeishuConfigInput` / `FeishuStatus` / `BotIdentity`
//! - 子模块：[`error`] [`client`] [`identity`]
//! - 状态：[`FeishuState`]，目前只持 `RwLock<FeishuStatus>`，M3 接入连接句柄时再扩
//!
//! M3 增量：
//! - DTOs：[`FeishuUserSession`]（一个 open_id 一行的"当前会话"映射）
//! - 子模块：[`commands`] [`dedup`] [`locks`] [`inbound`] [`routing`] [`outbound`]
//!   [`pipeline_bridge`] [`dispatcher`] [`connection`]
//! - [`FeishuState`] 扩为持有连接句柄 / 串行锁 / 去重缓存

pub mod client;
pub mod commands;
pub mod connection;
pub mod dedup;
pub mod dispatcher;
pub mod error;
pub mod identity;
pub mod inbound;
pub mod locks;
pub mod outbound;
pub mod pipeline_bridge;
pub mod routing;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{oneshot, RwLock};

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

/// 飞书用户的"当前会话"映射（M3）。
///
/// 一个 `user_open_id` 对应一行；`current_session_id` 指向 `analysis_sessions.id`。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeishuUserSession {
    pub id: i64,
    pub user_open_id: String,
    pub user_name: Option<String>,
    pub current_session_id: String,
    pub last_message_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 飞书运行时状态（M3：由 `FeishuState` 在连接生命周期中维护）。
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
/// M3 持有：
/// - `status`：连接 / 错误 / 时间戳，前端可读
/// - `connection`：当前活跃的连接 join handle，`feishu_stop` 时 abort
/// - `locks`：per-open_id 串行锁，避免同一用户并发触发流水线
/// - `dedup`：消息 message_id 去重缓存，规避 SDK 重投递
pub struct FeishuState {
    /// 用 `Arc<RwLock<...>>` 而不是裸 `RwLock<...>`：长连接任务（独立 OS 线程上跑）需要拿到一份句柄，
    /// 在 WebSocket 自然退出时把 `running` 翻回 false / 记录错误。
    pub status: Arc<RwLock<FeishuStatus>>,
    /// 飞书 WebSocket 不能被 tokio 多线程 runtime spawn（SDK 的 `EventDispatcherHandler` 含
    /// `Box<dyn EventHandler>`，没有 `Send + Sync` bound）；解决：每次 `start` spawn 一条独立
    /// OS 线程，里面跑一个 `current_thread` runtime。本字段持有给该线程的 oneshot shutdown sender；
    /// `stop` 时 send 信号，线程内 `tokio::select!` 命中后丢弃 dispatcher 让连接结束。
    pub connection: RwLock<Option<oneshot::Sender<()>>>,
    pub locks: Arc<locks::LockMap>,
    pub dedup: Arc<dedup::Dedup>,
}

impl FeishuState {
    pub fn new() -> Self {
        Self {
            status: Arc::new(RwLock::new(FeishuStatus::default())),
            connection: RwLock::new(None),
            locks: Arc::new(locks::LockMap::new()),
            dedup: Arc::new(dedup::Dedup::new(2048, std::time::Duration::from_secs(86_400))),
        }
    }
}

impl Default for FeishuState {
    fn default() -> Self {
        Self::new()
    }
}
