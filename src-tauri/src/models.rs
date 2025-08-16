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

// 大模型配置相关结构
#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct LLMConfig {
    pub id: i64,
    pub platform: String,
    pub app_key: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewLLMConfig {
    pub platform: String,
    pub app_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateLLMConfig {
    pub platform: Option<String>,
    pub app_key: Option<String>,
    pub is_active: Option<bool>,
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
#[derive(Debug, Serialize, Deserialize)]
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