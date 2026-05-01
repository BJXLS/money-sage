use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::Result;
use chrono::{Duration, Utc};
use serde_json::Value;
use sqlx::{Row, SqlitePool};
use tokio::sync::Mutex;

use crate::memory::config::MemoryConfig;
use crate::memory::safety::scan_for_injection;
use crate::models::{
    Fact, FactFilter, FactSource, FactStatus, FactType, UpdateFact, UpsertInput, UpsertOutcome,
};

#[derive(Default)]
struct RateLimiterState {
    session_insert_count: HashMap<String, i32>,
    role_change_set: HashSet<String>,
}

pub struct FactsStore {
    pool: SqlitePool,
    config: MemoryConfig,
    limiter: Arc<Mutex<RateLimiterState>>,
}

impl FactsStore {
    pub fn new(pool: SqlitePool, config: MemoryConfig) -> Self {
        Self {
            pool,
            config,
            limiter: Arc::new(Mutex::new(RateLimiterState::default())),
        }
    }

    pub async fn list(&self, filter: FactFilter) -> Result<Vec<Fact>> {
        let limit = filter.limit.unwrap_or(100);
        let rows = sqlx::query(
            "SELECT * FROM memory_facts
             WHERE (?1 IS NULL OR fact_type = ?1)
               AND (?2 IS NULL OR status = ?2)
               AND (?3 IS NULL OR key = ?3)
             ORDER BY updated_at DESC
             LIMIT ?4",
        )
        .bind(filter.fact_type.map(|f| f.as_str().to_string()))
        .bind(filter.status.map(|s| s.as_str().to_string()))
        .bind(filter.key)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(row_to_fact).collect())
    }

    pub async fn upsert(&self, mut input: UpsertInput) -> Result<UpsertOutcome> {
        let source = input.source.clone().unwrap_or(FactSource::User);
        // 设计文档 §4.1.8(4)：AgentRole 来自 Analysis 时 confidence 钳到 [0, 0.75]，
        // 给 user (1.0) 和 preset (0.9) 留出足够空间，避免 LLM 写入碾压用户偏好。
        if input.fact_type == FactType::AgentRole && source == FactSource::Analysis {
            input.confidence_hint = Some(input.confidence_hint.unwrap_or(0.7).min(0.75).max(0.0));
        }
        if source != FactSource::User {
            if let Err(reason) = scan_for_injection(&input) {
                self.log_rejected(&input, source.as_str(), &reason).await?;
                return Ok(UpsertOutcome::Rejected { reason });
            }
        }

        if let Some(reason) = self.check_rate_limit(&input, &source).await {
            self.log_rejected(&input, source.as_str(), &reason).await?;
            return Ok(UpsertOutcome::Rejected { reason });
        }

        input.source = Some(source.clone());
        let fact_type_text = input.fact_type.as_str().to_string();
        let key = normalize_key(input.key.clone());

        let mut tx = self.pool.begin().await?;
        let candidate = sqlx::query(
            "SELECT * FROM memory_facts
             WHERE fact_type = ?1
               AND status IN ('active', 'provisional')
               AND ((key = ?2) OR (key IS NULL AND ?2 IS NULL))
             ORDER BY updated_at DESC
             LIMIT 1",
        )
        .bind(&fact_type_text)
        .bind(&key)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(row) = candidate {
            let old = row_to_fact(row);
            if semantic_equivalent(&old, &input) {
                let old_conf = old.confidence;
                let new_conf = (old_conf + 0.15 * (1.0 - old_conf)).min(1.0);
                sqlx::query(
                    "UPDATE memory_facts
                     SET usage_count = usage_count + 1,
                         confidence = ?1,
                         last_used_at = CURRENT_TIMESTAMP,
                         updated_at = CURRENT_TIMESTAMP
                     WHERE id = ?2",
                )
                .bind(new_conf)
                .bind(old.id)
                .execute(&mut *tx)
                .await?;
                self.insert_history_with_tx(
                    &mut tx,
                    old.id,
                    "auto_merge",
                    source.as_str(),
                    Some(serde_json::to_value(&old)?),
                    Some(input.value_json.clone()),
                    input.origin_session.clone(),
                )
                .await?;
                tx.commit().await?;
                return Ok(UpsertOutcome::Merged {
                    id: old.id,
                    old_confidence: old_conf,
                    new_confidence: new_conf,
                });
            }

            match self.arbitrate(&input, &old) {
                Arbitrate::Skip(reason) => {
                    self.insert_history_with_tx(
                        &mut tx,
                        old.id,
                        "rejected",
                        source.as_str(),
                        None,
                        Some(serde_json::json!({ "reason": reason })),
                        input.origin_session.clone(),
                    )
                    .await?;
                    tx.commit().await?;
                    return Ok(UpsertOutcome::Rejected { reason });
                }
                Arbitrate::Supersede => {
                    sqlx::query(
                        "UPDATE memory_facts
                         SET status='superseded', updated_at=CURRENT_TIMESTAMP
                         WHERE id = ?",
                    )
                    .bind(old.id)
                    .execute(&mut *tx)
                    .await?;

                    let new_id = self
                        .insert_fact_with_tx(&mut tx, &input, key.clone(), Some(old.id))
                        .await?;
                    self.insert_history_with_tx(
                        &mut tx,
                        old.id,
                        "supersede",
                        source.as_str(),
                        Some(serde_json::to_value(&old)?),
                        None,
                        input.origin_session.clone(),
                    )
                    .await?;
                    self.insert_history_with_tx(
                        &mut tx,
                        new_id,
                        if source == FactSource::Preset {
                            "preset_apply"
                        } else {
                            "insert"
                        },
                        source.as_str(),
                        None,
                        Some(input.value_json.clone()),
                        input.origin_session.clone(),
                    )
                    .await?;
                    tx.commit().await?;
                    self.commit_rate_limit(&input, &source).await;
                    return Ok(UpsertOutcome::Superseded {
                        new_id,
                        old_id: old.id,
                    });
                }
            }
        }

        let id = self.insert_fact_with_tx(&mut tx, &input, key, None).await?;
        self.insert_history_with_tx(
            &mut tx,
            id,
            if source == FactSource::Preset {
                "preset_apply"
            } else {
                "insert"
            },
            source.as_str(),
            None,
            Some(input.value_json.clone()),
            input.origin_session.clone(),
        )
        .await?;
        tx.commit().await?;
        self.commit_rate_limit(&input, &source).await;
        Ok(UpsertOutcome::Inserted { id })
    }

    pub async fn edit_by_user(&self, id: i64, patch: UpdateFact) -> Result<()> {
        let before = sqlx::query("SELECT * FROM memory_facts WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        let Some(before_row) = before else {
            return Ok(());
        };
        let before_fact = row_to_fact(before_row);

        let key = patch.key.clone().or(before_fact.key.clone());
        let value_json = patch
            .value_json
            .clone()
            .unwrap_or(before_fact.value_json.clone());
        let status_value = patch.status.unwrap_or_else(|| before_fact.status.clone());
        let status = status_value.as_str().to_string();
        let confidence = patch.confidence.unwrap_or(before_fact.confidence);

        sqlx::query(
            "UPDATE memory_facts
             SET key = ?1, value_json = ?2, status = ?3, confidence = ?4, updated_at=CURRENT_TIMESTAMP
             WHERE id = ?5",
        )
        .bind(key)
        .bind(value_json.to_string())
        .bind(status)
        .bind(confidence)
        .bind(id)
        .execute(&self.pool)
        .await?;

        self.insert_history(
            id,
            "update",
            "user",
            Some(serde_json::to_value(&before_fact)?),
            Some(value_json),
            None,
        )
        .await?;
        Ok(())
    }

    pub async fn retire(&self, id: i64) -> Result<()> {
        let before = sqlx::query("SELECT * FROM memory_facts WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        let Some(before_row) = before else {
            return Ok(());
        };
        let before_fact = row_to_fact(before_row);
        sqlx::query("UPDATE memory_facts SET status='retired', updated_at=CURRENT_TIMESTAMP WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        self.insert_history(
            id,
            "retire",
            "user",
            Some(serde_json::to_value(&before_fact)?),
            None,
            None,
        )
        .await?;
        Ok(())
    }

    pub async fn undo(&self, history_id: i64) -> Result<()> {
        let row = sqlx::query(
            "SELECT id, fact_id, op, before_json, after_json
             FROM memory_facts_history
             WHERE id = ?",
        )
        .bind(history_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(r) = row else {
            return Ok(());
        };
        let fact_id: i64 = r.get("fact_id");
        let op: String = r.get("op");
        let before_json: Option<String> = r.try_get("before_json").ok();

        let mut tx = self.pool.begin().await?;
        match op.as_str() {
            "insert" | "preset_apply" => {
                // 检查这条 fact 是否由 supersede 创建（supersedes_id 非空）。
                // 若是，需要在退役新条的同时，把被压过的旧条恢复为 active。
                let predecessor_id: Option<i64> = sqlx::query_scalar(
                    "SELECT supersedes_id FROM memory_facts WHERE id = ?",
                )
                .bind(fact_id)
                .fetch_optional(&mut *tx)
                .await?
                .flatten();

                sqlx::query(
                    "UPDATE memory_facts SET status='retired', updated_at=CURRENT_TIMESTAMP WHERE id=?",
                )
                .bind(fact_id)
                .execute(&mut *tx)
                .await?;

                if let Some(pid) = predecessor_id {
                    sqlx::query(
                        "UPDATE memory_facts SET status='active', updated_at=CURRENT_TIMESTAMP WHERE id=?",
                    )
                    .bind(pid)
                    .execute(&mut *tx)
                    .await?;
                }
            }
            "retire" => {
                sqlx::query(
                    "UPDATE memory_facts SET status='active', updated_at=CURRENT_TIMESTAMP WHERE id=?",
                )
                .bind(fact_id)
                .execute(&mut *tx)
                .await?;
            }
            "update" | "auto_merge" => {
                if let Some(before) = before_json {
                    if let Ok(v) = serde_json::from_str::<Value>(&before) {
                        let key = v.get("key").and_then(|x| x.as_str()).map(|s| s.to_string());
                        let value_json = v.get("value_json").cloned().unwrap_or(Value::Null);
                        let confidence = v.get("confidence").and_then(|x| x.as_f64()).unwrap_or(0.7);
                        let status = v.get("status").and_then(|x| x.as_str()).unwrap_or("active");
                        sqlx::query(
                            "UPDATE memory_facts
                             SET key=?1, value_json=?2, confidence=?3, status=?4, updated_at=CURRENT_TIMESTAMP
                             WHERE id=?5",
                        )
                        .bind(key)
                        .bind(value_json.to_string())
                        .bind(confidence)
                        .bind(status)
                        .bind(fact_id)
                        .execute(&mut *tx)
                        .await?;
                    }
                }
            }
            "supersede" => {
                // history.fact_id 指向被压的旧条，需同时把后继(supersedes_id=fact_id)退役
                let successor_id: Option<i64> = sqlx::query_scalar(
                    "SELECT id FROM memory_facts WHERE supersedes_id = ? AND status != 'retired' ORDER BY id DESC LIMIT 1",
                )
                .bind(fact_id)
                .fetch_optional(&mut *tx)
                .await?;

                if let Some(sid) = successor_id {
                    sqlx::query(
                        "UPDATE memory_facts SET status='retired', updated_at=CURRENT_TIMESTAMP WHERE id=?",
                    )
                    .bind(sid)
                    .execute(&mut *tx)
                    .await?;
                }

                sqlx::query(
                    "UPDATE memory_facts SET status='active', updated_at=CURRENT_TIMESTAMP WHERE id=?",
                )
                .bind(fact_id)
                .execute(&mut *tx)
                .await?;
            }
            _ => {}
        }

        self.insert_history_with_tx(
            &mut tx,
            fact_id,
            "undo",
            "user",
            None,
            Some(serde_json::json!({ "history_id": history_id })),
            None,
        )
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn log_rejected(&self, input: &UpsertInput, actor: &str, reason: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO memory_facts_history (fact_id, op, actor, before_json, after_json, origin_session)
             VALUES (0, 'rejected', ?1, NULL, ?2, ?3)",
        )
        .bind(actor)
        .bind(
            serde_json::json!({
                "fact_type": input.fact_type.as_str(),
                "key": input.key,
                "reason": reason
            })
            .to_string(),
        )
        .bind(input.origin_session.clone())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn insert_fact_with_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        input: &UpsertInput,
        key: Option<String>,
        supersedes_id: Option<i64>,
    ) -> Result<i64> {
        let source = input.source.clone().unwrap_or(FactSource::User);
        let confidence = default_confidence(source.clone(), input.confidence_hint);
        let status = default_status(input.fact_type.clone(), source.clone(), confidence);
        let res = sqlx::query(
            "INSERT INTO memory_facts (
                fact_type, key, value_json, source, confidence, status,
                supersedes_id, origin_session, origin_message, usage_count,
                last_used_at, created_at, updated_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 0, NULL, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
        )
        .bind(input.fact_type.as_str())
        .bind(key)
        .bind(input.value_json.to_string())
        .bind(source.as_str())
        .bind(confidence)
        .bind(status.as_str())
        .bind(supersedes_id)
        .bind(input.origin_session.clone())
        .bind(input.origin_message)
        .execute(tx.as_mut())
        .await?;
        Ok(res.last_insert_rowid())
    }

    async fn insert_history_with_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        fact_id: i64,
        op: &str,
        actor: &str,
        before_json: Option<Value>,
        after_json: Option<Value>,
        origin_session: Option<String>,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO memory_facts_history (fact_id, op, actor, before_json, after_json, origin_session)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(fact_id)
        .bind(op)
        .bind(actor)
        .bind(before_json.map(|v| v.to_string()))
        .bind(after_json.map(|v| v.to_string()))
        .bind(origin_session)
        .execute(tx.as_mut())
        .await?;
        Ok(())
    }

    async fn insert_history(
        &self,
        fact_id: i64,
        op: &str,
        actor: &str,
        before_json: Option<Value>,
        after_json: Option<Value>,
        origin_session: Option<String>,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO memory_facts_history (fact_id, op, actor, before_json, after_json, origin_session)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        )
        .bind(fact_id)
        .bind(op)
        .bind(actor)
        .bind(before_json.map(|v| v.to_string()))
        .bind(after_json.map(|v| v.to_string()))
        .bind(origin_session)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn check_rate_limit(&self, input: &UpsertInput, source: &FactSource) -> Option<String> {
        if !matches!(source, FactSource::Analysis | FactSource::QuickNote) {
            return None;
        }

        let session = input.origin_session.clone().unwrap_or_default();
        if session.is_empty() {
            return None;
        }

        let mut lock = self.limiter.lock().await;
        let count = *lock.session_insert_count.get(&session).unwrap_or(&0);
        if count >= self.config.max_facts_per_session {
            return Some("rate_limited_session_quota".to_string());
        }

        if input.fact_type == FactType::AgentRole && source == &FactSource::Analysis {
            let scope = input
                .value_json
                .get("scope")
                .and_then(|x| x.as_str())
                .unwrap_or("global");
            let key = format!("{}:{}", session, scope);
            if lock.role_change_set.contains(&key) {
                return Some("rate_limited_role_change".to_string());
            }
        }

        None
    }

    async fn commit_rate_limit(&self, input: &UpsertInput, source: &FactSource) {
        if !matches!(source, FactSource::Analysis | FactSource::QuickNote) {
            return;
        }
        let session = input.origin_session.clone().unwrap_or_default();
        if session.is_empty() {
            return;
        }
        let mut lock = self.limiter.lock().await;
        *lock.session_insert_count.entry(session.clone()).or_insert(0) += 1;
        if input.fact_type == FactType::AgentRole && source == &FactSource::Analysis {
            let scope = input
                .value_json
                .get("scope")
                .and_then(|x| x.as_str())
                .unwrap_or("global");
            lock.role_change_set.insert(format!("{}:{}", session, scope));
        }
    }

    fn arbitrate(&self, input: &UpsertInput, old: &Fact) -> Arbitrate {
        let source = input.source.clone().unwrap_or(FactSource::User);
        if source == FactSource::User {
            return Arbitrate::Supersede;
        }
        if old.source == FactSource::User {
            let now = Utc::now();
            if let Ok(old_time) = chrono::DateTime::parse_from_rfc3339(&(old.updated_at.clone() + "Z")) {
                if now.signed_duration_since(old_time.with_timezone(&Utc)) < Duration::days(7) {
                    return Arbitrate::Skip("recent_user_edit".to_string());
                }
            }
        }
        if input.fact_type == FactType::AgentRole {
            return Arbitrate::Supersede;
        }
        let score_new = input
            .confidence_hint
            .unwrap_or(default_confidence(source.clone(), None))
            * 0.7
            + 0.3;
        let score_old = old.confidence * 0.7 + 0.3;
        if score_new > score_old + 0.05 {
            Arbitrate::Supersede
        } else {
            Arbitrate::Skip(format!("old_stronger:{:.2}>{:.2}", score_old, score_new))
        }
    }
}

enum Arbitrate {
    Supersede,
    Skip(String),
}

fn default_confidence(source: FactSource, hint: Option<f32>) -> f32 {
    hint.unwrap_or(match source {
        FactSource::User => 1.0,
        FactSource::Preset => 0.9,
        FactSource::Import => 0.8,
        FactSource::Analysis => 0.7,
        FactSource::Recap => 0.6,
        FactSource::QuickNote => 0.4,
    })
}

fn default_status(fact_type: FactType, source: FactSource, confidence: f32) -> FactStatus {
    if fact_type == FactType::AgentRole {
        return FactStatus::Active;
    }
    match source {
        FactSource::User | FactSource::Preset | FactSource::Import => FactStatus::Active,
        FactSource::Analysis | FactSource::Recap if confidence >= 0.7 => FactStatus::Active,
        _ => FactStatus::Provisional,
    }
}

fn normalize_key(key: Option<String>) -> Option<String> {
    key.map(|k| k.trim().to_lowercase()).filter(|k| !k.is_empty())
}

fn semantic_equivalent(old: &Fact, input: &UpsertInput) -> bool {
    match input.fact_type {
        FactType::AgentRole => {
            // agent_role 永不走 merge：scope 单例约束 + value 任意差异都视为冲突，
            // 交给 arbitrate -> Supersede，永远保留旧版本可 undo（设计文档 §4.1.6 D-Rule3）
            false
        }
        FactType::ClassificationRule => {
            let old_pattern = old.value_json.get("pattern").and_then(|x| x.as_str()).unwrap_or("").to_lowercase();
            let new_pattern = input
                .value_json
                .get("pattern")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_lowercase();
            let old_target = old
                .value_json
                .get("target_category_path")
                .and_then(|x| x.as_str())
                .unwrap_or("");
            let new_target = input
                .value_json
                .get("target_category_path")
                .and_then(|x| x.as_str())
                .unwrap_or("");
            old_pattern == new_pattern && old_target == new_target
        }
        _ => old.value_json == input.value_json,
    }
}

fn row_to_fact(row: sqlx::sqlite::SqliteRow) -> Fact {
    let fact_type_text: String = row.get("fact_type");
    let source_text: String = row.get("source");
    let status_text: String = row.get("status");
    let value_json_text: String = row.get("value_json");
    Fact {
        id: row.get("id"),
        fact_type: match fact_type_text.as_str() {
            "classification_rule" => FactType::ClassificationRule,
            "recurring_event" => FactType::RecurringEvent,
            "financial_goal" => FactType::FinancialGoal,
            "user_profile" => FactType::UserProfile,
            _ => FactType::AgentRole,
        },
        key: row.try_get("key").ok(),
        value_json: serde_json::from_str(&value_json_text).unwrap_or(Value::Null),
        source: match source_text.as_str() {
            "quick_note" => FactSource::QuickNote,
            "analysis" => FactSource::Analysis,
            "recap" => FactSource::Recap,
            "import" => FactSource::Import,
            "preset" => FactSource::Preset,
            _ => FactSource::User,
        },
        confidence: row.try_get::<f32, _>("confidence").unwrap_or(0.7),
        status: match status_text.as_str() {
            "provisional" => FactStatus::Provisional,
            "superseded" => FactStatus::Superseded,
            "retired" => FactStatus::Retired,
            _ => FactStatus::Active,
        },
        supersedes_id: row.try_get("supersedes_id").ok(),
        origin_session: row.try_get("origin_session").ok(),
        origin_message: row.try_get("origin_message").ok(),
        usage_count: row.try_get("usage_count").unwrap_or(0),
        last_used_at: row.try_get("last_used_at").ok(),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}
