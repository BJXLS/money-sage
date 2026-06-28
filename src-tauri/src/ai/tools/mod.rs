pub mod bash_exec;
pub mod csv_read;
pub mod file_edit;
pub mod file_read;
pub mod file_write;
pub mod get_schema;
pub mod manage_categories;
pub mod memory_search;
pub mod query_database;
pub mod quick_note_parse;
pub mod quick_note_save;
pub mod workspace_path;
pub mod workspace_size;

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::Arc;

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
        workspace_dir: PathBuf,
        memory_dir: Option<PathBuf>,
    ) -> Self {
        let mut registry = Self { tools: Vec::new() };
        registry
            .tools
            .push(Box::new(get_schema::GetDatabaseSchemaTool::new(
                pool.clone(),
            )));
        registry
            .tools
            .push(Box::new(manage_categories::ManageCategoriesTool::new(
                pool.clone(),
            )));
        let session_id_for_db = session_id.clone().unwrap_or_else(|| "unknown".to_string());
        registry
            .tools
            .push(Box::new(query_database::QueryDatabaseTool::new(
                pool.clone(),
                workspace_dir.clone(),
                session_id_for_db,
            )));
        registry
            .tools
            .push(Box::new(quick_note_parse::QuickNoteParseTool::new(
                pool.clone(),
                session_id.clone(),
                token_recorder,
                memory_dir.clone(),
            )));
        registry
            .tools
            .push(Box::new(quick_note_save::QuickNoteSaveTool::new(
                pool.clone(),
                session_id.clone(),
            )));

        let pool_for_file_tools = pool.clone();

        // Memory V3 搜索工具
        if let Some(ref mdir) = memory_dir {
            let memory_store = crate::memory::v3::MemoryStore::new(mdir);
            registry.tools.push(Box::new(memory_search::MemorySearchTool::new(
                pool,
                memory_store,
                session_id.clone(),
            )));
        }

        registry.tools.push(Box::new(file_read::FileReadTool::new(
            workspace_dir.clone(),
            memory_dir.clone(),
        )));
        registry.tools.push(Box::new(file_edit::FileEditTool::new(
            workspace_dir.clone(),
            memory_dir.clone(),
            Some(pool_for_file_tools.clone()),
        )));
        registry.tools.push(Box::new(file_write::FileWriteTool::new(
            workspace_dir.clone(),
            memory_dir.clone(),
            Some(pool_for_file_tools),
        )));
        registry.tools.push(Box::new(csv_read::CsvReadTool::new(
            workspace_dir.clone(),
        )));
        registry.tools.push(Box::new(workspace_size::WorkspaceSizeTool::new(
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
