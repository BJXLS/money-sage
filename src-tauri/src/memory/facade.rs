use anyhow::Result;
use sqlx::SqlitePool;

use crate::memory::config::MemoryConfig;
use crate::memory::facts::FactsStore;
use crate::memory::history::HistoryStore;
use crate::memory::role_presets_store::RolePresetsStore;
use crate::memory::search::{MemorySearchBackend, SearchQuery, SearchResult};
use crate::memory::snapshot::{SnapshotAgent, SnapshotBuilder};
use crate::models::{
    Fact, FactFilter, HistoryEntry, NewRolePreset, RolePreset, RoleScope, RoleValue, UpdateFact,
    UpdateRolePreset, UpsertInput, UpsertOutcome,
};

#[derive(Clone)]
pub struct MemoryFacade {
    facts: std::sync::Arc<FactsStore>,
    history: std::sync::Arc<HistoryStore>,
    snapshots: std::sync::Arc<SnapshotBuilder>,
    search: std::sync::Arc<MemorySearchBackend>,
    presets: std::sync::Arc<RolePresetsStore>,
}

impl MemoryFacade {
    pub fn new(pool: SqlitePool) -> Self {
        let cfg = MemoryConfig::default();
        Self {
            facts: std::sync::Arc::new(FactsStore::new(pool.clone(), cfg.clone())),
            history: std::sync::Arc::new(HistoryStore::new(pool.clone())),
            snapshots: std::sync::Arc::new(SnapshotBuilder::new(pool.clone(), cfg)),
            search: std::sync::Arc::new(MemorySearchBackend::new(pool.clone())),
            presets: std::sync::Arc::new(RolePresetsStore::new(pool)),
        }
    }

    pub async fn search(&self, query: SearchQuery) -> Result<SearchResult> {
        self.search.search(query).await
    }

    /// 启动时调用：仅在 role_presets 表为空时插入唯一内置预设
    pub async fn ensure_default_role_seed(&self) -> Result<()> {
        self.presets.ensure_default_seeded().await
    }

    pub async fn list_facts(&self, filter: FactFilter) -> Result<Vec<Fact>> {
        self.facts.list(filter).await
    }

    pub async fn upsert_fact(&self, input: UpsertInput) -> Result<UpsertOutcome> {
        self.facts.upsert(input).await
    }

    pub async fn edit_fact(&self, id: i64, patch: UpdateFact) -> Result<()> {
        self.facts.edit_by_user(id, patch).await
    }

    pub async fn retire_fact(&self, id: i64) -> Result<()> {
        self.facts.retire(id).await
    }

    pub async fn undo(&self, history_id: i64) -> Result<()> {
        self.facts.undo(history_id).await
    }

    pub async fn list_recent_changes(&self, limit: i64) -> Result<Vec<HistoryEntry>> {
        self.history.list_recent_changes(limit).await
    }

    pub async fn render_quick_note_snapshot(&self) -> Result<String> {
        self.snapshots.render(SnapshotAgent::QuickNote).await
    }

    pub async fn render_analysis_snapshot(&self) -> Result<String> {
        self.snapshots.render(SnapshotAgent::Analysis).await
    }

    pub async fn list_role_presets(&self) -> Result<Vec<RolePreset>> {
        self.presets.list().await
    }

    pub async fn create_role_preset(&self, input: NewRolePreset) -> Result<RolePreset> {
        self.presets.create(input).await
    }

    pub async fn update_role_preset(&self, preset_id: String, patch: UpdateRolePreset) -> Result<()> {
        self.presets.update(&preset_id, patch).await
    }

    pub async fn delete_role_preset(&self, preset_id: String) -> Result<()> {
        self.presets.delete(&preset_id).await
    }

    pub async fn reset_role_preset(&self, preset_id: String) -> Result<()> {
        self.presets.reset_builtin(&preset_id).await
    }

    pub async fn apply_role_preset(&self, preset_id: String, scope: RoleScope) -> Result<UpsertOutcome> {
        let preset = self
            .presets
            .get(&preset_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("preset_not_found"))?;
        let mut value = preset.value;
        // 注入 scope 与 preset_id 元信息
        if !value.is_object() {
            value = serde_json::json!({});
        }
        value["scope"] = serde_json::Value::String(scope.as_str().to_string());
        value["preset_id"] = serde_json::Value::String(preset_id);
        self.facts
            .upsert(UpsertInput {
                fact_type: crate::models::FactType::AgentRole,
                key: Some(format!("role:{}", scope.as_str())),
                value_json: value,
                source: Some(crate::models::FactSource::Preset),
                confidence_hint: Some(0.9),
                origin_session: None,
                origin_message: None,
            })
            .await
    }

    pub async fn get_role(&self, scope: RoleScope) -> Result<Option<Fact>> {
        let items = self
            .facts
            .list(FactFilter {
                fact_type: Some(crate::models::FactType::AgentRole),
                status: Some(crate::models::FactStatus::Active),
                key: Some(format!("role:{}", scope.as_str())),
                limit: Some(1),
            })
            .await?;
        Ok(items.into_iter().next())
    }

    pub async fn set_role(&self, scope: RoleScope, value: RoleValue) -> Result<UpsertOutcome> {
        let input = UpsertInput {
            fact_type: crate::models::FactType::AgentRole,
            key: Some(format!("role:{}", scope.as_str())),
            value_json: serde_json::to_value(value)?,
            source: Some(crate::models::FactSource::User),
            confidence_hint: Some(1.0),
            origin_session: None,
            origin_message: None,
        };
        self.facts.upsert(input).await
    }

    pub async fn auto_decay(&self) -> Result<usize> {
        self.facts.auto_decay().await
    }
}
