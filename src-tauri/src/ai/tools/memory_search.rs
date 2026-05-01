use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

use super::LocalTool;
use crate::memory::search::SearchQuery;
use crate::memory::MemoryFacade;

pub struct MemorySearchTool {
    memory: Arc<MemoryFacade>,
    session_id: Option<String>,
}

impl MemorySearchTool {
    pub fn new(memory: Arc<MemoryFacade>, session_id: Option<String>) -> Self {
        Self { memory, session_id }
    }
}

#[async_trait]
impl LocalTool for MemorySearchTool {
    fn name(&self) -> &str {
        "memory_search"
    }

    fn description(&self) -> &str {
        "在长期记忆库（memory_facts）和历史会话（analysis_messages）中检索相关内容。\
        当用户提到「上次/之前/上个月聊过」「我之前说过」「以前的目标」等需要回忆的语境时调用。\
        返回的内容会被 <memory-context> 包裹，请把它作为上下文参考，不要逐字复述。"
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
                    "maximum": 5,
                    "default": 3,
                    "description": "facts 与 sessions 各自返回的最大条数（≤5）"
                },
                "time_range_days": {
                    "type": "integer",
                    "minimum": 1,
                    "default": 365,
                    "description": "会话检索的时间窗口（天），默认 365"
                },
                "include": {
                    "type": "array",
                    "items": { "type": "string", "enum": ["facts", "sessions"] },
                    "default": ["facts", "sessions"],
                    "description": "要检索的范围"
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
            .unwrap_or(3)
            .clamp(1, 5);

        let time_range_days = arguments
            .get("time_range_days")
            .and_then(|v| v.as_i64())
            .unwrap_or(365)
            .max(1);

        let include = arguments
            .get("include")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| vec!["facts".to_string(), "sessions".to_string()]);

        let q = SearchQuery {
            query: query.clone(),
            top_k_facts: top_k,
            top_k_sessions: top_k,
            time_range_days,
            exclude_session: self.session_id.clone(),
            include_facts: include.iter().any(|s| s == "facts"),
            include_sessions: include.iter().any(|s| s == "sessions"),
        };

        let result = self.memory.search(q).await?;

        // 按设计文档 §9.2 的「记忆围栏」格式返回，便于模型识别上下文边界
        let payload = json!({
            "query": query,
            "facts": result.facts,
            "sessions": result.sessions,
            "hint": "以上为可参考的历史上下文（已脱敏裁剪），请按实际需求选择性引用，不要逐字复述。"
        });
        let body = serde_json::to_string_pretty(&payload)?;
        Ok(format!("<memory-context>\n{}\n</memory-context>", body))
    }
}
