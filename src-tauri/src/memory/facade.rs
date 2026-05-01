use anyhow::Result;
use sqlx::SqlitePool;

use crate::memory::config::MemoryConfig;
use crate::memory::facts::FactsStore;
use crate::memory::history::HistoryStore;
use crate::memory::role::default_presets;
use crate::memory::search::{MemorySearchBackend, SearchQuery, SearchResult};
use crate::memory::snapshot::{SnapshotAgent, SnapshotBuilder};
use crate::models::{
    Fact, FactFilter, HistoryEntry, RolePreset, RoleScope, RoleValue, UpdateFact, UpsertInput, UpsertOutcome,
};

#[derive(Clone)]
pub struct MemoryFacade {
    facts: std::sync::Arc<FactsStore>,
    history: std::sync::Arc<HistoryStore>,
    snapshots: std::sync::Arc<SnapshotBuilder>,
    search: std::sync::Arc<MemorySearchBackend>,
}

impl MemoryFacade {
    pub fn new(pool: SqlitePool) -> Self {
        let cfg = MemoryConfig::default();
        Self {
            facts: std::sync::Arc::new(FactsStore::new(pool.clone(), cfg.clone())),
            history: std::sync::Arc::new(HistoryStore::new(pool.clone())),
            snapshots: std::sync::Arc::new(SnapshotBuilder::new(pool.clone(), cfg)),
            search: std::sync::Arc::new(MemorySearchBackend::new(pool)),
        }
    }

    pub async fn search(&self, query: SearchQuery) -> Result<SearchResult> {
        self.search.search(query).await
    }

    pub async fn seed_default_roles(&self) -> Result<()> {
        for preset in default_presets() {
            let mut value = preset.value.clone();
            value["preset_id"] = serde_json::Value::String(preset.preset_id);
            let input = UpsertInput {
                fact_type: crate::models::FactType::AgentRole,
                key: Some(format!(
                    "role:{}",
                    value
                        .get("scope")
                        .and_then(|x| x.as_str())
                        .unwrap_or("global")
                )),
                value_json: value,
                source: Some(crate::models::FactSource::Preset),
                confidence_hint: Some(0.9),
                origin_session: None,
                origin_message: None,
            };
            let _ = self.facts.upsert(input).await?;
        }
        Ok(())
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

    pub fn list_role_presets(&self) -> Vec<RolePreset> {
        default_presets()
    }

    pub async fn apply_role_preset(&self, preset_id: String, scope: RoleScope) -> Result<UpsertOutcome> {
        let preset = default_presets()
            .into_iter()
            .find(|p| p.preset_id == preset_id)
            .ok_or_else(|| anyhow::anyhow!("preset_not_found"))?;
        let mut value = preset.value;
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
}
