use anyhow::Result;
use sqlx::{Row, SqlitePool};

use crate::models::HistoryEntry;

pub struct HistoryStore {
    pool: SqlitePool,
}

impl HistoryStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list_recent_changes(&self, limit: i64) -> Result<Vec<HistoryEntry>> {
        let rows = sqlx::query(
            "SELECT id, fact_id, op, actor, before_json, after_json, origin_session, created_at
             FROM memory_facts_history
             ORDER BY id DESC
             LIMIT ?",
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            items.push(HistoryEntry {
                id: row.get("id"),
                fact_id: row.get("fact_id"),
                op: row.get("op"),
                actor: row.get("actor"),
                before_json: row
                    .try_get::<Option<String>, _>("before_json")
                    .ok()
                    .flatten()
                    .and_then(|v| serde_json::from_str(&v).ok()),
                after_json: row
                    .try_get::<Option<String>, _>("after_json")
                    .ok()
                    .flatten()
                    .and_then(|v| serde_json::from_str(&v).ok()),
                origin_session: row.try_get("origin_session").ok(),
                created_at: row.get("created_at"),
            });
        }
        Ok(items)
    }
}
