use anyhow::Result;
use async_trait::async_trait;
use chrono::NaiveDate;
use serde_json::{json, Value};
use sqlx::{Row, SqlitePool};

use crate::ai::tools::LocalTool;
use crate::database::Database;
use crate::models::{ConfirmQuickNoteDraftRequest, ConfirmedTransaction};

pub struct QuickNoteSaveTool {
    pool: SqlitePool,
    _session_id: Option<String>,
}

impl QuickNoteSaveTool {
    pub fn new(pool: SqlitePool, session_id: Option<String>) -> Self {
        Self {
            pool,
            _session_id: session_id,
        }
    }
}

#[async_trait]
impl LocalTool for QuickNoteSaveTool {
    fn name(&self) -> &str {
        "quick_note_save"
    }

    fn description(&self) -> &str {
        "保存 quick_note_parse 产生的草稿。必须提供 draft_id 与 confirmation_token。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type":"object",
            "properties":{
                "draft_id":{"type":"string"},
                "confirmation_token":{"type":"string"},
                "items":{"type":"array"}
            },
            "required":["draft_id","confirmation_token"]
        })
    }

    async fn execute(&self, arguments: Value) -> Result<String> {
        let draft_id = arguments
            .get("draft_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少 draft_id"))?
            .to_string();
        let token = arguments
            .get("confirmation_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少 confirmation_token"))?
            .to_string();

        let items: Vec<ConfirmedTransaction> = if let Some(items) = arguments.get("items").and_then(|v| v.as_array()) {
            items
                .iter()
                .map(|it| ConfirmedTransaction {
                    date: it.get("date").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
                    amount: it.get("amount").and_then(|v| v.as_f64()).unwrap_or(0.0),
                    transaction_type: it
                        .get("transaction_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("expense")
                        .to_string(),
                    category_id: it.get("category_id").and_then(|v| v.as_i64()).unwrap_or(0),
                    budget_id: it.get("budget_id").and_then(|v| v.as_i64()),
                    description: it
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string(),
                })
                .collect()
        } else {
            let db_items = sqlx::query(
                "SELECT date, amount, transaction_type, category_id, budget_id, description, note
                 FROM quick_note_draft_items WHERE draft_id = ? ORDER BY sort_order ASC, id ASC",
            )
            .bind(&draft_id)
            .fetch_all(&self.pool)
            .await?;
            db_items
                .into_iter()
                .map(|r| {
                    ConfirmedTransaction {
                        date: r.get::<String, _>("date"),
                        amount: r.get::<f64, _>("amount"),
                        transaction_type: r.get::<String, _>("transaction_type"),
                        category_id: r.try_get::<Option<i64>, _>("category_id").ok().flatten().unwrap_or(0),
                        budget_id: r.try_get::<Option<i64>, _>("budget_id").ok().flatten(),
                        description: r
                            .try_get::<Option<String>, _>("description")
                            .ok()
                            .flatten()
                            .unwrap_or_default(),
                    }
                })
                .collect()
        };
        if items.iter().any(|i| NaiveDate::parse_from_str(&i.date, "%Y-%m-%d").is_err() || i.amount <= 0.0 || i.category_id <= 0) {
            return Ok(json!({"ok":false, "error":"invalid_items"}).to_string());
        }

        let db = Database {
            pool: self.pool.clone(),
        };
        let saved = db
            .confirm_quick_note_draft(&ConfirmQuickNoteDraftRequest {
                draft_id: draft_id.clone(),
                confirmation_token: token,
                items,
            })
            .await?;

        Ok(json!({
            "ok": saved.failed_count == 0 && saved.saved_count > 0,
            "draft_id": draft_id,
            "saved_count": saved.saved_count,
            "failed_count": saved.failed_count,
            "status": if saved.failed_count == 0 && saved.saved_count > 0 { "confirmed" } else { "pending" }
        })
        .to_string())
    }
}
