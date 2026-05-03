use anyhow::{anyhow, Result};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::memory::role::{
    default_preset_value, DEFAULT_PRESET_DISPLAY_NAME, DEFAULT_PRESET_ID, DEFAULT_PRESET_SUMMARY,
};
use crate::models::{NewRolePreset, RolePreset, UpdateRolePreset};

pub struct RolePresetsStore {
    pool: SqlitePool,
}

impl RolePresetsStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list(&self) -> Result<Vec<RolePreset>> {
        let rows = sqlx::query(
            "SELECT preset_id, display_name, summary, value_json, is_builtin, sort_order
             FROM role_presets
             ORDER BY is_builtin DESC, sort_order ASC, created_at ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(row_to_preset).collect())
    }

    pub async fn get(&self, preset_id: &str) -> Result<Option<RolePreset>> {
        let row = sqlx::query(
            "SELECT preset_id, display_name, summary, value_json, is_builtin, sort_order
             FROM role_presets WHERE preset_id = ?",
        )
        .bind(preset_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(row_to_preset))
    }

    pub async fn create(&self, input: NewRolePreset) -> Result<RolePreset> {
        let display_name = input.display_name.trim();
        if display_name.is_empty() {
            return Err(anyhow!("display_name 不能为空"));
        }
        let preset_id = Uuid::new_v4().to_string();
        let summary = input.summary.unwrap_or_default();
        let sort_order = input.sort_order.unwrap_or(0);
        let value_text = input.value.to_string();

        sqlx::query(
            "INSERT INTO role_presets (preset_id, display_name, summary, value_json, is_builtin, sort_order)
             VALUES (?, ?, ?, ?, 0, ?)",
        )
        .bind(&preset_id)
        .bind(display_name)
        .bind(&summary)
        .bind(&value_text)
        .bind(sort_order)
        .execute(&self.pool)
        .await?;

        self.get(&preset_id)
            .await?
            .ok_or_else(|| anyhow!("preset 创建后读取失败"))
    }

    pub async fn update(&self, preset_id: &str, patch: UpdateRolePreset) -> Result<()> {
        let existing = self
            .get(preset_id)
            .await?
            .ok_or_else(|| anyhow!("preset_not_found"))?;
        let display_name = patch
            .display_name
            .map(|s| s.trim().to_string())
            .unwrap_or(existing.display_name);
        if display_name.is_empty() {
            return Err(anyhow!("display_name 不能为空"));
        }
        let summary = patch.summary.unwrap_or(existing.summary);
        let value_text = patch
            .value
            .unwrap_or(existing.value)
            .to_string();
        let sort_order = patch.sort_order.unwrap_or(existing.sort_order);

        sqlx::query(
            "UPDATE role_presets
             SET display_name = ?, summary = ?, value_json = ?, sort_order = ?,
                 updated_at = CURRENT_TIMESTAMP
             WHERE preset_id = ?",
        )
        .bind(display_name)
        .bind(summary)
        .bind(value_text)
        .bind(sort_order)
        .bind(preset_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete(&self, preset_id: &str) -> Result<()> {
        let existing = self
            .get(preset_id)
            .await?
            .ok_or_else(|| anyhow!("preset_not_found"))?;
        if existing.is_builtin {
            return Err(anyhow!("内置预设不可删除"));
        }
        sqlx::query("DELETE FROM role_presets WHERE preset_id = ?")
            .bind(preset_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// 仅对内置预设生效：把 value/display_name/summary 重置回常量
    pub async fn reset_builtin(&self, preset_id: &str) -> Result<()> {
        let existing = self
            .get(preset_id)
            .await?
            .ok_or_else(|| anyhow!("preset_not_found"))?;
        if !existing.is_builtin {
            return Err(anyhow!("仅内置预设可重置"));
        }
        sqlx::query(
            "UPDATE role_presets
             SET display_name = ?, summary = ?, value_json = ?,
                 updated_at = CURRENT_TIMESTAMP
             WHERE preset_id = ?",
        )
        .bind(DEFAULT_PRESET_DISPLAY_NAME)
        .bind(DEFAULT_PRESET_SUMMARY)
        .bind(default_preset_value().to_string())
        .bind(preset_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// 启动时调用：表为空时插入唯一内置默认预设
    pub async fn ensure_default_seeded(&self) -> Result<()> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM role_presets")
            .fetch_one(&self.pool)
            .await?;
        if count > 0 {
            return Ok(());
        }
        sqlx::query(
            "INSERT INTO role_presets (preset_id, display_name, summary, value_json, is_builtin, sort_order)
             VALUES (?, ?, ?, ?, 1, 0)",
        )
        .bind(DEFAULT_PRESET_ID)
        .bind(DEFAULT_PRESET_DISPLAY_NAME)
        .bind(DEFAULT_PRESET_SUMMARY)
        .bind(default_preset_value().to_string())
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

fn row_to_preset(row: sqlx::sqlite::SqliteRow) -> RolePreset {
    let value_text: String = row.get("value_json");
    let value = serde_json::from_str(&value_text).unwrap_or(serde_json::Value::Null);
    RolePreset {
        preset_id: row.get("preset_id"),
        display_name: row.get("display_name"),
        summary: row.try_get("summary").unwrap_or_default(),
        value,
        is_builtin: row.try_get::<i64, _>("is_builtin").unwrap_or(0) != 0,
        sort_order: row.try_get("sort_order").unwrap_or(0),
    }
}
