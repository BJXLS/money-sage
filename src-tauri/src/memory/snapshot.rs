use anyhow::Result;
use sqlx::{Row, SqlitePool};

use crate::memory::config::MemoryConfig;
use crate::models::{FactType, RoleScope};

#[derive(Debug, Clone, Copy)]
pub enum SnapshotAgent {
    QuickNote,
    Analysis,
}

pub struct SnapshotBuilder {
    pool: SqlitePool,
    config: MemoryConfig,
}

impl SnapshotBuilder {
    pub fn new(pool: SqlitePool, config: MemoryConfig) -> Self {
        Self { pool, config }
    }

    pub async fn render(&self, agent: SnapshotAgent) -> Result<String> {
        let role_scope = match agent {
            SnapshotAgent::QuickNote => RoleScope::QuickNote,
            SnapshotAgent::Analysis => RoleScope::Analysis,
        };
        let mut sections = vec![self.render_role(role_scope).await?];

        match agent {
            SnapshotAgent::QuickNote => {
                sections.push(self.render_facts(FactType::ClassificationRule, 10).await?);
                sections.push(self.render_facts(FactType::RecurringEvent, 8).await?);
            }
            SnapshotAgent::Analysis => {
                sections.push(self.render_facts(FactType::UserProfile, 6).await?);
                sections.push(self.render_facts(FactType::FinancialGoal, 8).await?);
                sections.push(self.render_facts(FactType::RecurringEvent, 8).await?);
                sections.push(self.render_facts(FactType::ClassificationRule, 10).await?);
            }
        }

        let joined = sections.into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n\n");

        // 更新被引用 facts 的 usage_count 和 last_used_at
        self.touch_facts_usage(agent).await;

        Ok(limit_chars(
            &joined,
            match agent {
                SnapshotAgent::QuickNote => self.config.quick_note_snapshot_char_limit,
                SnapshotAgent::Analysis => self.config.analysis_snapshot_char_limit,
            },
        ))
    }

    async fn render_role(&self, scope: RoleScope) -> Result<String> {
        let scope_text = scope.as_str();
        let row = sqlx::query(
            "SELECT value_json FROM memory_facts
             WHERE fact_type='agent_role' AND status='active'
               AND json_extract(value_json, '$.scope') = ?
             ORDER BY updated_at DESC LIMIT 1",
        )
        .bind(scope_text)
        .fetch_optional(&self.pool)
        .await?;
        let fallback = sqlx::query(
            "SELECT value_json FROM memory_facts
             WHERE fact_type='agent_role' AND status='active'
               AND json_extract(value_json, '$.scope') = 'global'
             ORDER BY updated_at DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?;
        let final_row = row.or(fallback);
        if let Some(r) = final_row {
            let value: String = r.get("value_json");
            Ok(format!("## 角色设定\n{}", value))
        } else {
            Ok("## 角色设定\n你是 MoneySage，语气友好、专业、简洁。".to_string())
        }
    }

    async fn render_facts(&self, fact_type: FactType, limit: i64) -> Result<String> {
        let rows = sqlx::query(
            "SELECT key, value_json, usage_count, status
             FROM memory_facts
             WHERE fact_type = ? AND status IN ('active', 'provisional')
             ORDER BY
               CASE WHEN status = 'active' THEN 0 ELSE 1 END,
               usage_count DESC, confidence DESC, updated_at DESC
             LIMIT ?",
        )
        .bind(fact_type.as_str())
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        if rows.is_empty() {
            return Ok(String::new());
        }

        let title = match fact_type {
            FactType::ClassificationRule => "分类规则",
            FactType::RecurringEvent => "固定事件",
            FactType::FinancialGoal => "财务目标",
            FactType::UserProfile => "用户画像",
            FactType::AgentRole => "角色设定",
        };

        let mut lines = vec![format!("## {}", title)];
        for r in rows {
            let key = r.try_get::<Option<String>, _>("key").ok().flatten().unwrap_or_default();
            let value: String = r.get("value_json");
            let status: String = r.get("status");
            let prefix = if status == "provisional" { "[待确认] " } else { "" };
            let key_prefix = if key.is_empty() { String::new() } else { format!("{} ", key) };
            lines.push(format!("- {}{}{}", prefix, key_prefix, value).trim().to_string());
        }
        Ok(lines.join("\n"))
    }

    async fn touch_facts_usage(&self, agent: SnapshotAgent) {
        let fact_types: &[&str] = match agent {
            SnapshotAgent::QuickNote => &[
                "agent_role",
                "classification_rule",
                "recurring_event",
            ],
            SnapshotAgent::Analysis => &[
                "agent_role",
                "user_profile",
                "financial_goal",
                "recurring_event",
                "classification_rule",
            ],
        };

        for ft in fact_types {
            let _ = sqlx::query(
                "UPDATE memory_facts
                 SET usage_count = usage_count + 1,
                     last_used_at = CURRENT_TIMESTAMP
                 WHERE fact_type = ? AND status IN ('active', 'provisional')"
            )
            .bind(ft)
            .execute(&self.pool)
            .await;
        }
    }
}

fn limit_chars(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }
    input.chars().take(max_chars).collect()
}
