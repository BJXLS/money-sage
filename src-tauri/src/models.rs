use serde::{Deserialize, Serialize};
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub r#type: String, // 'income' or 'expense'
    pub parent_id: Option<i64>,
    pub is_system: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewCategory {
    pub name: String,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub r#type: String,
    pub parent_id: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateCategory {
    pub name: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub parent_id: Option<Option<i64>>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Transaction {
    pub id: i64,
    pub date: NaiveDate,
    pub r#type: String, // 'income' or 'expense'
    pub amount: f64,
    pub category_id: i64,
    pub budget_id: Option<i64>, // 关联的预算ID
    pub description: Option<String>,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewTransaction {
    pub date: NaiveDate,
    pub r#type: String,
    pub amount: f64,
    pub category_id: i64,
    pub budget_id: Option<i64>,
    pub description: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateTransaction {
    pub date: Option<NaiveDate>,
    pub r#type: Option<String>,
    pub amount: Option<f64>,
    pub category_id: Option<i64>,
    pub budget_id: Option<Option<i64>>,
    pub description: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Budget {
    pub id: i64,
    pub name: String,
    pub category_id: i64,
    pub amount: f64,
    pub budget_type: String, // 'time' or 'event'
    pub period_type: String, // 'weekly', 'monthly', 'yearly' for time budgets
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewBudget {
    pub name: String,
    pub category_id: i64,
    pub amount: f64,
    pub budget_type: String,
    pub period_type: String,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateBudget {
    pub name: Option<String>,
    pub category_id: Option<i64>,
    pub amount: Option<f64>,
    pub budget_type: Option<String>,
    pub period_type: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<Option<NaiveDate>>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct TransactionWithCategory {
    // Transaction fields
    pub id: i64,
    pub date: NaiveDate,
    pub r#type: String,
    pub amount: f64,
    pub category_id: i64,
    pub budget_id: Option<i64>,
    pub description: Option<String>,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Category fields
    pub category_name: String,
    pub category_icon: Option<String>,
    pub category_color: Option<String>,
    // Budget fields (optional)
    pub budget_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MonthlyStats {
    pub month: String,
    pub income: f64,
    pub expense: f64,
    pub balance: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CategoryStats {
    pub category_id: i64,
    pub category_name: String,
    pub category_icon: Option<String>,
    pub category_color: Option<String>,
    pub amount: f64,
    pub percentage: f64,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct BudgetProgress {
    // Budget fields
    pub id: i64,
    pub name: String,
    pub category_id: i64,
    pub amount: f64,
    pub budget_type: String,
    pub period_type: String,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Category fields
    pub category_name: String,
    pub category_icon: Option<String>,
    pub category_color: Option<String>,
    // Progress fields
    pub spent: f64,
    pub remaining: f64,
    pub percentage: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DateRange {
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
}

// 大模型配置相关结构（泛化重设计：支持任意 OpenAI 兼容接口）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LLMConfig {
    pub id: i64,
    /// 用户自定义配置名称，如 "我的 GPT-4"
    pub config_name: String,
    /// 供应商名称，如 "OpenAI"、"阿里百炼"、"Ollama"
    pub provider: String,
    /// API Base URL，如 "https://api.openai.com/v1"
    pub base_url: String,
    /// API Key（本地服务可为空）
    pub api_key: String,
    /// 模型名称，如 "gpt-4o-mini"、"qwen-plus"
    pub model: String,
    /// 温度参数 0.0-2.0
    pub temperature: f64,
    /// 最大 token 数
    pub max_tokens: i64,
    /// 是否开启深度思考（仅阿里百炼等支持该参数的供应商生效）
    pub enable_thinking: bool,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewLLMConfig {
    pub config_name: String,
    pub provider: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i64>,
    pub enable_thinking: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateLLMConfig {
    pub config_name: Option<String>,
    pub provider: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i64>,
    pub enable_thinking: Option<bool>,
    pub is_active: Option<bool>,
}

/// 连接测试请求（不要求先保存）
#[derive(Debug, Serialize, Deserialize)]
pub struct TestConnectionRequest {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

// 快速记账相关结构
#[derive(Debug, Serialize, Deserialize)]
pub struct QuickBookingRequest {
    pub text: String,
}

// AI解析后的结果，用于前端展示和编辑
#[derive(Debug, Serialize, Deserialize)]
pub struct QuickBookingResult {
    pub success: bool,
    pub message: String,
    pub parsed_transactions: Vec<ParsedTransaction>, // 解析出的交易（未保存）
    pub failed_lines: Vec<FailedLine>,
}

// AI解析出的交易信息（供前端展示和编辑）
#[derive(Debug, Serialize, Deserialize)]
pub struct ParsedTransaction {
    pub original_text: String,
    pub date: String,           // YYYY-MM-DD格式
    pub amount: f64,
    pub transaction_type: String, // income/expense
    pub category_name: String,   // AI识别的分类名称（可能是父分类-子分类格式）
    pub category_id: Option<i64>, // 映射后的分类ID（如果能找到匹配的）
    pub description: String,     // 备注
    pub confidence: f32,         // AI识别的置信度
}

// 用户确认保存的请求
#[derive(Debug, Serialize, Deserialize)]
pub struct SaveTransactionsRequest {
    pub transactions: Vec<ConfirmedTransaction>,
}

// 用户确认后的交易记录
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConfirmedTransaction {
    pub date: String,           // YYYY-MM-DD格式
    pub amount: f64,
    pub transaction_type: String, // income/expense  
    pub category_id: i64,       // 分类ID
    pub budget_id: Option<i64>, // 关联的预算ID（可选）
    pub description: String,    // 备注
}

// 保存结果
#[derive(Debug, Serialize, Deserialize)]
pub struct SaveTransactionsResult {
    pub success: bool,
    pub message: String,
    pub saved_count: usize,
    pub failed_count: usize,
}

// ─────────────────────────────────────────────────────────────────────────────
// 数据导入导出
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum DataExportFormat {
    Excel,
    MoneySage,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExportDataRequest {
    pub file_path: String,
    pub format: DataExportFormat,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExportDataResult {
    pub file_path: String,
    pub categories: usize,
    pub budgets: usize,
    pub transactions: usize,
    pub memory_facts: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImportConflictStrategy {
    Skip,
    Upsert,
    ReplaceAll,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImportPreviewItem {
    pub table: String,
    pub rows: usize,
    pub estimated_insert: usize,
    pub estimated_update: usize,
    pub estimated_skip: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImportPreviewResult {
    pub file_type: String,
    pub schema_version: Option<i32>,
    pub checksum_valid: Option<bool>,
    pub items: Vec<ImportPreviewItem>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImportDataRequest {
    pub file_path: String,
    pub strategy: ImportConflictStrategy,
    #[serde(default)]
    pub skip_checksum: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ImportDataResult {
    pub categories: usize,
    pub budgets: usize,
    pub transactions: usize,
    pub memory_facts: usize,
    pub inserted: usize,
    pub updated: usize,
    pub skipped: usize,
    pub warnings: Vec<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// 智能分析 (ChatBI) 相关结构
// ─────────────────────────────────────────────────────────────────────────────

/// 持久化的分析会话
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnalysisSession {
    pub id: String,
    pub title: String,
    pub config_id: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

/// 持久化的分析消息
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnalysisMessageRecord {
    pub id: i64,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
    pub message_type: String,
    pub tool_calls_json: Option<String>,
    pub tool_call_id: Option<String>,
    pub tool_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuickNoteDraft {
    pub id: i64,
    pub draft_id: String,
    pub session_id: String,
    pub source_message_id: Option<i64>,
    pub status: String,
    pub confirmation_token: Option<String>,
    pub created_by_tool_call_id: Option<String>,
    pub confirmed_by_message_id: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
    pub items: Vec<QuickNoteDraftItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QuickNoteDraftItem {
    pub id: i64,
    pub draft_id: String,
    pub date: String,
    pub amount: f64,
    pub transaction_type: String,
    pub category_id: Option<i64>,
    pub budget_id: Option<i64>,
    pub description: Option<String>,
    pub note: Option<String>,
    pub raw_category_name: Option<String>,
    pub confidence: f64,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateQuickNoteDraftRequest {
    pub session_id: String,
    pub source_message_id: Option<i64>,
    pub items: Vec<ConfirmedTransaction>,
    pub created_by_tool_call_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ConfirmQuickNoteDraftRequest {
    pub draft_id: String,
    pub confirmation_token: String,
    pub items: Vec<ConfirmedTransaction>,
}

/// 流式分析请求（前端 → 后端）
#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisStreamRequest {
    pub message: String,
    pub session_id: String,
    pub config_id: Option<i64>,
}

/// 工具调用状态载荷
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolStatusPayload {
    pub tool_name: String,
    pub status: String,
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_input: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_output: Option<String>,
}

/// 流式 chunk 事件载荷（后端 → 前端，通过 Tauri 事件推送）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StreamChunkPayload {
    pub session_id: String,
    pub chunk: String,
    pub done: bool,
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_status: Option<ToolStatusPayload>,
}

/// AI 生成的图表配置（ECharts 数据格式，暂保留供后续使用）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChartDataItem {
    pub name: String,
    pub value: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChartConfig {
    pub chart_type: String,
    pub title: String,
    pub x_axis: Option<Vec<String>>,
    pub data: Vec<ChartDataItem>,
    pub unit: Option<String>,
}

/// 分析响应（非流式，保留兼容）
#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisResponse {
    pub success: bool,
    pub message: String,
    pub text: String,
    pub chart: Option<ChartConfig>,
}

// 遗留结构体（兼容性）
#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessedTransaction {
    pub original_text: String,
    pub transaction: NewTransaction,
    pub confidence: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FailedLine {
    pub line_number: usize,
    pub original_text: String,
    pub error_reason: String,
} 

// ─────────────────────────────────────────────────────────────────────────────
// Memory (阶段一)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FactType {
    ClassificationRule,
    RecurringEvent,
    FinancialGoal,
    UserProfile,
    AgentRole,
}

impl FactType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ClassificationRule => "classification_rule",
            Self::RecurringEvent => "recurring_event",
            Self::FinancialGoal => "financial_goal",
            Self::UserProfile => "user_profile",
            Self::AgentRole => "agent_role",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FactStatus {
    Active,
    Provisional,
    Superseded,
    Retired,
}

impl FactStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Provisional => "provisional",
            Self::Superseded => "superseded",
            Self::Retired => "retired",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FactSource {
    User,
    QuickNote,
    Analysis,
    Recap,
    Import,
    Preset,
}

impl FactSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::QuickNote => "quick_note",
            Self::Analysis => "analysis",
            Self::Recap => "recap",
            Self::Import => "import",
            Self::Preset => "preset",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RoleScope {
    Global,
    QuickNote,
    Analysis,
}

impl RoleScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::QuickNote => "quick_note",
            Self::Analysis => "analysis",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RoleTone {
    pub style: Option<String>,
    pub emoji: Option<bool>,
    pub verbosity: Option<String>,
    pub language_flavor: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RoleValue {
    pub scope: RoleScope,
    pub display_name: Option<String>,
    pub self_reference: Option<String>,
    pub user_address: Option<String>,
    pub tone: Option<RoleTone>,
    pub traits: Option<Vec<String>>,
    pub r#do: Option<Vec<String>>,
    pub dont: Option<Vec<String>>,
    pub preset_id: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RolePreset {
    pub preset_id: String,
    pub display_name: String,
    pub summary: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Fact {
    pub id: i64,
    pub fact_type: FactType,
    pub key: Option<String>,
    pub value_json: serde_json::Value,
    pub source: FactSource,
    pub confidence: f32,
    pub status: FactStatus,
    pub supersedes_id: Option<i64>,
    pub origin_session: Option<String>,
    pub origin_message: Option<i64>,
    pub usage_count: i64,
    pub last_used_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UpsertInput {
    pub fact_type: FactType,
    pub key: Option<String>,
    pub value_json: serde_json::Value,
    pub source: Option<FactSource>,
    pub confidence_hint: Option<f32>,
    pub origin_session: Option<String>,
    pub origin_message: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UpsertOutcome {
    Inserted { id: i64 },
    Merged {
        id: i64,
        old_confidence: f32,
        new_confidence: f32,
    },
    Superseded { new_id: i64, old_id: i64 },
    Rejected { reason: String },
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct FactFilter {
    pub fact_type: Option<FactType>,
    pub status: Option<FactStatus>,
    pub key: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct UpdateFact {
    pub key: Option<String>,
    pub value_json: Option<serde_json::Value>,
    pub confidence: Option<f32>,
    pub status: Option<FactStatus>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HistoryEntry {
    pub id: i64,
    pub fact_id: i64,
    pub op: String,
    pub actor: String,
    pub before_json: Option<serde_json::Value>,
    pub after_json: Option<serde_json::Value>,
    pub origin_session: Option<String>,
    pub created_at: String,
}

// ─── Token 用量统计 ─────────────────────────────────────────────────────────

/// 写入 token_usage_logs 时使用
#[derive(Debug, Clone)]
pub struct TokenUsageRecord {
    pub agent_name: String,
    pub session_id: Option<String>,
    pub request_id: String,
    pub round_index: i32,
    pub config_id: Option<i64>,
    pub config_name_snapshot: Option<String>,
    pub provider: String,
    pub model: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub total_tokens: i32,
    pub finish_reason: Option<String>,
    pub duration_ms: Option<i64>,
    pub success: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TokenUsageEntry {
    pub id: i64,
    pub agent_name: String,
    pub session_id: Option<String>,
    pub request_id: String,
    pub round_index: i32,
    pub config_id: Option<i64>,
    pub config_name: Option<String>,
    pub provider: String,
    pub model: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub finish_reason: Option<String>,
    pub duration_ms: Option<i64>,
    pub success: bool,
    pub error_message: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct TokenUsageFilter {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub config_id: Option<i64>,
    pub model: Option<String>,
    pub agent_name: Option<String>,
    pub session_id: Option<String>,
    pub success_only: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TokenUsageSummary {
    pub group_key: String,
    pub group_label: String,
    pub config_id: Option<i64>,
    pub config_name: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub call_count: i64,
    pub success_count: i64,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum TokenUsageGroupBy {
    Day,
    Model,
    Config,
    ConfigDay,
}