use super::store::MemoryStore;
use anyhow::Result;
use sqlx::{Row, SqlitePool};

/// V2 → V3 迁移器
///
/// 从 memory_facts 表读取数据，转换为 Markdown 文件格式写入 memory/ 目录。
/// 迁移是一次性的，完成后建议在数据库中记录标记。
pub struct Migrator {
    pool: SqlitePool,
    store: MemoryStore,
}

#[derive(Debug)]
pub struct MigrationReport {
    pub classification_rules: usize,
    pub recurring_events: usize,
    pub financial_goals: usize,
    pub user_profiles: usize,
    pub agent_roles: usize,
    pub total_entries: usize,
}

impl Migrator {
    pub fn new(pool: SqlitePool, store: MemoryStore) -> Self {
        Self { pool, store }
    }

    /// 检查是否已迁移过（通过检查 memory/ 下是否有非骨架内容）
    pub fn already_migrated(&self) -> Result<bool> {
        match self.store.read_file("factual/finance-rules.md") {
            Ok(content) => {
                // 如果文件中有 & 开头的行，说明已有内容
                Ok(content.lines().any(|l| l.trim_start().starts_with('&')))
            }
            Err(_) => Ok(false),
        }
    }

    /// 执行迁移
    pub async fn migrate(&self) -> Result<MigrationReport> {
        let rows = sqlx::query(
            "SELECT fact_type, key, value_json, source, status, created_at, updated_at
             FROM memory_facts
             WHERE status IN ('active', 'provisional', 'superseded')
             ORDER BY fact_type, updated_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut classification_rules = Vec::new();
        let mut recurring_events = Vec::new();
        let mut financial_goals = Vec::new();
        let mut user_profiles = Vec::new();
        let mut agent_roles = Vec::new();

        for row in &rows {
            let fact_type: String = row.get("fact_type");
            let key: Option<String> = row.try_get("key").ok().flatten();
            let value: String = row.get("value_json");
            let source: String = row.get("source");
            let status: String = row.get("status");
            let updated_at: String = row.get("updated_at");

            let source_tag = match source.as_str() {
                "user" => "user",
                "analysis" => "agent:analysis",
                "quick_note" => "agent:analysis",
                "recap" => "agent:analysis",
                "import" => "user",
                "preset" => "user",
                _ => "agent:analysis",
            };

            let status_prefix = if status == "provisional" {
                "[待确认] "
            } else if status == "superseded" {
                "[已取代] "
            } else {
                ""
            };

            let content = format!(
                "{}{} {}",
                status_prefix,
                key.as_deref().unwrap_or(""),
                value
            );

            let line = format!("& {} | {} | {}", updated_at, source_tag, content.trim());

            match fact_type.as_str() {
                "classification_rule" => classification_rules.push(line),
                "recurring_event" => recurring_events.push(line),
                "financial_goal" => financial_goals.push(line),
                "user_profile" => user_profiles.push(line),
                "agent_role" => agent_roles.push(line),
                _ => {}
            }
        }

        // 写入文件
        if !classification_rules.is_empty() || !recurring_events.is_empty() {
            let mut content = String::new();
            content.push_str("<!-- memory-file\n  category: factual\n  created: ");
            content.push_str(&chrono::Local::now().to_rfc3339());
            content.push_str("\n  updated: ");
            content.push_str(&chrono::Local::now().to_rfc3339());
            content.push_str("\n  char_count: 0\n  entry_count: 0\n-->\n\n# 财务规则\n\n## 分类规则\n");
            for line in &classification_rules {
                content.push_str(line);
                content.push('\n');
            }
            content.push_str("\n## 周期事件\n");
            for line in &recurring_events {
                content.push_str(line);
                content.push('\n');
            }
            self.store.write_file("factual/finance-rules.md", &content)?;
        }

        if !financial_goals.is_empty() {
            let mut content = String::new();
            content.push_str("<!-- memory-file\n  category: factual\n  created: ");
            content.push_str(&chrono::Local::now().to_rfc3339());
            content.push_str("\n  updated: ");
            content.push_str(&chrono::Local::now().to_rfc3339());
            content.push_str("\n  char_count: 0\n  entry_count: 0\n-->\n\n# 财务目标\n\n");
            for line in &financial_goals {
                content.push_str(line);
                content.push('\n');
            }
            self.store.write_file("factual/goals.md", &content)?;
        }

        if !user_profiles.is_empty() {
            let mut content = String::new();
            content.push_str("<!-- memory-file\n  category: factual\n  created: ");
            content.push_str(&chrono::Local::now().to_rfc3339());
            content.push_str("\n  updated: ");
            content.push_str(&chrono::Local::now().to_rfc3339());
            content.push_str("\n  char_count: 0\n  entry_count: 0\n-->\n\n# 用户画像\n\n");
            for line in &user_profiles {
                content.push_str(line);
                content.push('\n');
            }
            self.store.write_file("factual/user-profile.md", &content)?;
        }

        if !agent_roles.is_empty() {
            let mut content = String::new();
            content.push_str("<!-- memory-file\n  category: factual\n  created: ");
            content.push_str(&chrono::Local::now().to_rfc3339());
            content.push_str("\n  updated: ");
            content.push_str(&chrono::Local::now().to_rfc3339());
            content.push_str("\n  char_count: 0\n  entry_count: 0\n-->\n\n# Agent 角色设定\n\n");
            for line in &agent_roles {
                content.push_str(line);
                content.push('\n');
            }
            self.store.write_file("factual/agent-role.md", &content)?;
        }

        Ok(MigrationReport {
            classification_rules: classification_rules.len(),
            recurring_events: recurring_events.len(),
            financial_goals: financial_goals.len(),
            user_profiles: user_profiles.len(),
            agent_roles: agent_roles.len(),
            total_entries: rows.len(),
        })
    }
}
