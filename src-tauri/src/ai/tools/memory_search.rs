use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

use super::LocalTool;
use crate::memory::v3::{MemoryIndexer, MemoryStore};
use sqlx::SqlitePool;

pub struct MemorySearchTool {
    indexer: MemoryIndexer,
    session_id: Option<String>,
}

impl MemorySearchTool {
    pub fn new(pool: SqlitePool, store: MemoryStore, session_id: Option<String>) -> Self {
        Self {
            indexer: MemoryIndexer::new(pool, store),
            session_id,
        }
    }
}

#[async_trait]
impl LocalTool for MemorySearchTool {
    fn name(&self) -> &str {
        "memory_search"
    }

    fn description(&self) -> &str {
        "在记忆文件系统中检索相关内容。\
        当用户提到「上次/之前/上个月聊过」「我之前说过」「以前的目标」等需要回忆的语境时调用。\
        返回的内容会被 <memory-context> 包裹，请把它作为上下文参考，不要逐字复述。\
        搜索结果按 factual（事实）、episodic（情景）、procedural（程序）分组返回，互不覆盖。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["query"],
            "properties": {
                "query": {
                    "type": "string",
                    "description": "自然语言检索词，建议 2-15 字"
                },
                "top_k": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 10,
                    "default": 5,
                    "description": "每种类型返回的最大条数"
                }
            }
        })
    }

    async fn execute(&self, arguments: Value) -> Result<String> {
        let query = arguments
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少参数 query"))?
            .to_string();

        let top_k = arguments
            .get("top_k")
            .and_then(|v| v.as_i64())
            .unwrap_or(5)
            .clamp(1, 10);

        // 搜索前先同步索引（确保最新）
        let _ = self.indexer.sync_all().await;

        let result = self.indexer.search(&query, top_k).await?;

        let factual: Vec<Value> = result
            .factual
            .into_iter()
            .map(|item| {
                json!({
                    "file": item.file_path,
                    "heading": item.heading,
                    "text": item.full_text,
                    "timestamp": item.timestamp,
                    "source": item.source,
                })
            })
            .collect();

        let episodic: Vec<Value> = result
            .episodic
            .into_iter()
            .map(|item| {
                json!({
                    "file": item.file_path,
                    "heading": item.heading,
                    "text": item.full_text,
                    "timestamp": item.timestamp,
                    "source": item.source,
                })
            })
            .collect();

        let procedural: Vec<Value> = result
            .procedural
            .into_iter()
            .map(|item| {
                json!({
                    "file": item.file_path,
                    "heading": item.heading,
                    "text": item.full_text,
                    "timestamp": item.timestamp,
                    "source": item.source,
                })
            })
            .collect();

        let payload = json!({
            "query": query,
            "factual": factual,
            "episodic": episodic,
            "procedural": procedural,
            "hint": "以上为可参考的历史上下文（已脱敏裁剪），请按实际需求选择性引用，不要逐字复述。"
        });
        let body = serde_json::to_string_pretty(&payload)?;
        Ok(format!("<memory-context>\n{}\n</memory-context>", body))
    }
}
