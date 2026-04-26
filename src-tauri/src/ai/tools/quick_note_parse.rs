use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use sqlx::{Row, SqlitePool};
use std::sync::Arc;

use crate::ai::agent::quick_note::QuickNoteAgent;
use crate::ai::tools::LocalTool;
use crate::database::Database;
use crate::memory::MemoryFacade;
use crate::models::{
    Category, ConfirmedTransaction, CreateQuickNoteDraftRequest, TokenUsageRecord,
};
use crate::telemetry::TokenUsageRecorder;
use crate::utils::http_client::{AIHttpClient, AIProvider, ClientConfig};

pub struct QuickNoteParseTool {
    pool: SqlitePool,
    session_id: Option<String>,
    token_recorder: Option<Arc<TokenUsageRecorder>>,
}

impl QuickNoteParseTool {
    pub fn new(
        pool: SqlitePool,
        session_id: Option<String>,
        token_recorder: Option<Arc<TokenUsageRecorder>>,
    ) -> Self {
        Self { pool, session_id, token_recorder }
    }
}

#[async_trait]
impl LocalTool for QuickNoteParseTool {
    fn name(&self) -> &str {
        "quick_note_parse"
    }

    fn description(&self) -> &str {
        "将自然语言记账内容解析为候选交易草稿。不会直接入账，需用户确认。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type":"object",
            "properties":{
                "input":{"type":"string","description":"自然语言记账文本"}
            },
            "required":["input"]
        })
    }

    async fn execute(&self, arguments: Value) -> Result<String> {
        let input = arguments
            .get("input")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少 input 参数"))?
            .trim()
            .to_string();
        if input.is_empty() {
            return Ok(json!({"error":"input_empty"}).to_string());
        }

        let session_id = self
            .session_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("缺少 session_id 上下文"))?;

        let active_config = sqlx::query(
            "SELECT id, config_name, provider, base_url, api_key, model, temperature, max_tokens, enable_thinking
             FROM llm_configs
             WHERE is_active=1
             ORDER BY updated_at DESC
             LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("未找到可用 LLM 配置"))?;

        let config_id: Option<i64> = active_config.try_get("id").ok();
        let config_name: String = active_config.try_get("config_name").unwrap_or_default();
        let provider: String = active_config.try_get("provider").unwrap_or_default();
        let base_url: String = active_config.try_get("base_url").unwrap_or_default();
        let api_key: String = active_config.try_get("api_key").unwrap_or_default();
        let model: String = active_config.try_get("model").unwrap_or_default();
        let temperature: f32 = active_config.try_get::<f64, _>("temperature").unwrap_or(0.3) as f32;
        let max_tokens: u32 = active_config.try_get::<i64, _>("max_tokens").unwrap_or(2048) as u32;
        let enable_thinking: bool = active_config.try_get::<i64, _>("enable_thinking").unwrap_or(0) != 0;

        let http = AIHttpClient::new(ClientConfig {
            provider: AIProvider::Custom(provider.clone()),
            base_url,
            api_key,
            timeout_secs: 60,
            max_retries: 2,
            headers: std::collections::HashMap::new(),
        })?;

        let categories: Vec<Category> = sqlx::query_as::<_, Category>(
            "SELECT * FROM categories ORDER BY is_system DESC, name ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        let memory = MemoryFacade::new(self.pool.clone());
        let memory_snapshot = memory.render_quick_note_snapshot().await.unwrap_or_default();

        let request_id = uuid::Uuid::new_v4().to_string();
        let started = std::time::Instant::now();
        let quick_note = QuickNoteAgent::new_with_config(model.clone(), temperature, max_tokens, enable_thinking);
        let parse_outcome = quick_note
            .parse_quick_note_with_categories_reported(&input, &categories, &memory_snapshot, &http)
            .await;

        let (parsed, _report_used) = match parse_outcome {
            Ok((parsed, report)) => {
                if let Some(rec) = &self.token_recorder {
                    let usage_record = TokenUsageRecord {
                        agent_name: "AnalysisAgent.tool.quick_note_parse".into(),
                        session_id: self.session_id.clone(),
                        request_id: request_id.clone(),
                        round_index: 0,
                        config_id,
                        config_name_snapshot: Some(config_name.clone()),
                        provider: provider.clone(),
                        model: if report.model.is_empty() { model.clone() } else { report.model.clone() },
                        prompt_tokens: report.usage.as_ref().map(|u| u.prompt_tokens as i32).unwrap_or(0),
                        completion_tokens: report.usage.as_ref().map(|u| u.completion_tokens as i32).unwrap_or(0),
                        total_tokens: report.usage.as_ref().map(|u| u.total_tokens as i32).unwrap_or(0),
                        finish_reason: report.finish_reason.clone(),
                        duration_ms: Some(report.duration_ms),
                        success: true,
                        error_message: None,
                    };
                    if let Err(e) = rec.record(usage_record).await {
                        eprintln!("[quick_note_parse] 记录 token 用量失败: {}", e);
                    }
                }
                (parsed, report)
            }
            Err(e) => {
                if let Some(rec) = &self.token_recorder {
                    let duration_ms = started.elapsed().as_millis() as i64;
                    let usage_record = TokenUsageRecord {
                        agent_name: "AnalysisAgent.tool.quick_note_parse".into(),
                        session_id: self.session_id.clone(),
                        request_id: request_id.clone(),
                        round_index: 0,
                        config_id,
                        config_name_snapshot: Some(config_name.clone()),
                        provider: provider.clone(),
                        model: model.clone(),
                        prompt_tokens: 0,
                        completion_tokens: 0,
                        total_tokens: 0,
                        finish_reason: None,
                        duration_ms: Some(duration_ms),
                        success: false,
                        error_message: Some(e.to_string()),
                    };
                    if let Err(re) = rec.record(usage_record).await {
                        eprintln!("[quick_note_parse] 记录 token 用量失败: {}", re);
                    }
                }
                return Err(e);
            }
        };

        let mut candidates = Vec::new();
        let mut items = Vec::new();
        for t in parsed.transactions {
            candidates.push(json!({
                "date": t.date,
                "amount": t.amount,
                "transaction_type": t.transaction_type,
                "category_name": t.category,
                "description": t.remark
            }));
            items.push(ConfirmedTransaction {
                date: t.date,
                amount: t.amount,
                transaction_type: t.transaction_type,
                category_id: 0,
                budget_id: None,
                description: t.remark,
            });
        }
        let db = Database {
            pool: self.pool.clone(),
        };
        let draft = db.create_quick_note_draft(&CreateQuickNoteDraftRequest {
            session_id,
            source_message_id: None,
            items,
            created_by_tool_call_id: None,
        }).await?;

        Ok(json!({
            "draft_id": draft.draft_id,
            "status": "pending",
            "requires_confirmation": true,
            "items": candidates
        })
        .to_string())
    }
}
