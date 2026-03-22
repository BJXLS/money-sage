pub mod get_schema;
pub mod query_database;

use async_trait::async_trait;
use anyhow::Result;
use serde_json::Value;
use sqlx::SqlitePool;

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
    pub fn new(pool: SqlitePool) -> Self {
        let mut registry = Self { tools: Vec::new() };
        registry.tools.push(Box::new(get_schema::GetDatabaseSchemaTool::new(pool.clone())));
        registry.tools.push(Box::new(query_database::QueryDatabaseTool::new(pool)));
        registry
    }

    pub fn get(&self, name: &str) -> Option<&dyn LocalTool> {
        self.tools.iter().find(|t| t.name() == name).map(|t| t.as_ref())
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
