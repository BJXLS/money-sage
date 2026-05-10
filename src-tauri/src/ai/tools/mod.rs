pub mod bash_exec;
pub mod file_edit;
pub mod file_read;
pub mod file_write;
pub mod get_schema;
pub mod memory_fact_upsert;
pub mod memory_search;
pub mod query_database;
pub mod quick_note_parse;
pub mod quick_note_save;
pub mod workspace_path;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::Arc;

use crate::memory::MemoryFacade;
use crate::telemetry::TokenUsageRecorder;

pub const ALLOWED_TABLES: &[&str] = &["categories", "transactions", "budgets"];

#[async_trait]
pub trait LocalTool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;
    async fn execute(&self, arguments: Value) -> Result<String>;

    fn as_openai_tool(&self) -> Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name(),
                "description": self.description(),
                "parameters": self.parameters_schema(),
            }
        })
    }
}

pub struct LocalToolRegistry {
    tools: Vec<Box<dyn LocalTool>>,
}

impl LocalToolRegistry {
    pub fn new(
        pool: SqlitePool,
        session_id: Option<String>,
        token_recorder: Option<Arc<TokenUsageRecorder>>,
        memory: Arc<MemoryFacade>,
        workspace_dir: PathBuf,
    ) -> Self {
        let mut registry = Self { tools: Vec::new() };
        registry
            .tools
            .push(Box::new(get_schema::GetDatabaseSchemaTool::new(
                pool.clone(),
            )));
        registry
            .tools
            .push(Box::new(query_database::QueryDatabaseTool::new(
                pool.clone(),
            )));
        registry
            .tools
            .push(Box::new(quick_note_parse::QuickNoteParseTool::new(
                pool.clone(),
                session_id.clone(),
                token_recorder,
            )));
        registry
            .tools
            .push(Box::new(quick_note_save::QuickNoteSaveTool::new(
                pool,
                session_id.clone(),
            )));
        // registry.tools.push(Box::new(memory_search::MemorySearchTool::new(
        //     memory.clone(),
        //     session_id.clone(),
        // )));
        // registry.tools.push(Box::new(memory_fact_upsert::MemoryFactUpsertTool::new(
        //     memory,
        //     session_id,
        // )));
        registry.tools.push(Box::new(file_read::FileReadTool::new(
            workspace_dir.clone(),
        )));
        registry.tools.push(Box::new(file_edit::FileEditTool::new(
            workspace_dir.clone(),
        )));
        registry.tools.push(Box::new(file_write::FileWriteTool::new(
            workspace_dir.clone(),
        )));
        registry
            .tools
            .push(Box::new(bash_exec::BashExecTool::new(workspace_dir)));
        registry
    }

    pub fn get(&self, name: &str) -> Option<&dyn LocalTool> {
        self.tools
            .iter()
            .find(|t| t.name() == name)
            .map(|t| t.as_ref())
    }

    pub fn all_as_openai_tools(&self) -> Vec<Value> {
        self.tools.iter().map(|t| t.as_openai_tool()).collect()
    }

    pub async fn execute(&self, name: &str, arguments: Value) -> Result<String> {
        match self.get(name) {
            Some(tool) => tool.execute(arguments).await,
            None => Err(anyhow::anyhow!("未找到工具: {}", name)),
        }
    }
}
