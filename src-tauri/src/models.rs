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
    pub description: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateTransaction {
    pub date: Option<NaiveDate>,
    pub r#type: Option<String>,
    pub amount: Option<f64>,
    pub category_id: Option<i64>,
    pub description: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Budget {
    pub id: i64,
    pub category_id: i64,
    pub amount: f64,
    pub period_type: String, // 'weekly', 'monthly', 'yearly'
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewBudget {
    pub category_id: i64,
    pub amount: f64,
    pub period_type: String,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct TransactionWithCategory {
    // Transaction fields
    pub id: i64,
    pub date: NaiveDate,
    pub r#type: String,
    pub amount: f64,
    pub category_id: i64,
    pub description: Option<String>,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Category fields
    pub category_name: String,
    pub category_icon: Option<String>,
    pub category_color: Option<String>,
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
    pub category_id: i64,
    pub amount: f64,
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