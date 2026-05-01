use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

use super::LocalTool;
use crate::memory::MemoryFacade;
use crate::models::{FactSource, FactType, UpsertInput, UpsertOutcome};

pub struct MemoryFactUpsertTool {
    memory: Arc<MemoryFacade>,
    session_id: Option<String>,
}

impl MemoryFactUpsertTool {
    pub fn new(memory: Arc<MemoryFacade>, session_id: Option<String>) -> Self {
        Self { memory, session_id }
    }

    fn parse_fact_type(s: &str) -> Result<FactType> {
        match s {
            "classification_rule" => Ok(FactType::ClassificationRule),
            "recurring_event" => Ok(FactType::RecurringEvent),
            "financial_goal" => Ok(FactType::FinancialGoal),
            "user_profile" => Ok(FactType::UserProfile),
            "agent_role" => Ok(FactType::AgentRole),
            other => Err(anyhow::anyhow!("未知 fact_type: {}", other)),
        }
    }
}

#[async_trait]
impl LocalTool for MemoryFactUpsertTool {
    fn name(&self) -> &str {
        "memory_fact_upsert"
    }

    fn description(&self) -> &str {
        "把对话里值得长期沉淀的事实写入记忆库（来源固定为 analysis）。\
        适用于：分类规则 / 周期事件 / 财务目标 / 用户画像 / agent_role 人格调整。\
        agent_role 用于用户明确改称谓/语气时（value_json.scope 必填，建议 analysis）。\
        非明确意图请勿调用，避免污染长期记忆。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["fact_type", "value_json"],
            "properties": {
                "fact_type": {
                    "type": "string",
                    "enum": [
                        "classification_rule",
                        "recurring_event",
                        "financial_goal",
                        "user_profile",
                        "agent_role"
                    ],
                    "description": "事实类型"
                },
                "key": {
                    "type": ["string", "null"],
                    "description": "去重键。规则用 pattern；事件用 title；目标用 metric:title；profile 可省略；agent_role 自动生成 role:<scope>"
                },
                "value_json": {
                    "type": "object",
                    "description": "事实的结构化内容。结构需符合该 fact_type 的 schema（见设计文档 §4.1.6）。"
                },
                "confidence_hint": {
                    "type": "number",
                    "minimum": 0,
                    "maximum": 1,
                    "description": "置信度提示，0-1。analysis 写 agent_role 时会被钳到 0.75 以内。"
                }
            }
        })
    }

    async fn execute(&self, arguments: Value) -> Result<String> {
        let fact_type_str = arguments
            .get("fact_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少参数 fact_type"))?;
        let fact_type = Self::parse_fact_type(fact_type_str)?;

        let value_json = arguments
            .get("value_json")
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("缺少参数 value_json"))?;
        if !value_json.is_object() {
            return Err(anyhow::anyhow!("value_json 必须是 object"));
        }

        let key_arg = arguments
            .get("key")
            .and_then(|v| if v.is_null() { None } else { v.as_str() })
            .map(|s| s.to_string());

        // agent_role 的 key 由 scope 强制生成，避免 LLM 写错
        let key = if fact_type == FactType::AgentRole {
            let scope = value_json
                .get("scope")
                .and_then(|v| v.as_str())
                .unwrap_or("analysis");
            Some(format!("role:{}", scope))
        } else {
            key_arg
        };

        let confidence_hint = arguments
            .get("confidence_hint")
            .and_then(|v| v.as_f64())
            .map(|v| v.clamp(0.0, 1.0) as f32);

        let input = UpsertInput {
            fact_type: fact_type.clone(),
            key,
            value_json: value_json.clone(),
            // 工具调用始终以 analysis 身份写入：facts.upsert 内部会触发
            // 注入扫描、速率限制、agent_role 钳值、冲突仲裁
            source: Some(FactSource::Analysis),
            confidence_hint,
            origin_session: self.session_id.clone(),
            origin_message: None,
        };

        let outcome = self.memory.upsert_fact(input).await?;

        let user_visible_hint = match (&outcome, fact_type) {
            (UpsertOutcome::Superseded { .. }, FactType::AgentRole) => {
                Some("已记住，可在『记忆管理 → 人格』撤销")
            }
            (UpsertOutcome::Inserted { .. }, FactType::AgentRole) => {
                Some("已记住，可在『记忆管理 → 人格』撤销")
            }
            _ => None,
        };

        let mut payload = json!({ "outcome": outcome });
        if let Some(hint) = user_visible_hint {
            payload["user_visible_hint"] = json!(hint);
        }
        Ok(serde_json::to_string(&payload)?)
    }
}
