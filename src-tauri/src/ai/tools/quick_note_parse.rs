use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use sqlx::{Row, SqlitePool};
use std::path::PathBuf;
use std::sync::Arc;

use crate::ai::agent::quick_note::QuickNoteAgent;
use crate::ai::tools::LocalTool;
use crate::database::Database;
use crate::models::{
    Category, ConfirmedTransaction, CreateQuickNoteDraftRequest, TokenUsageRecord,
};
use crate::telemetry::TokenUsageRecorder;
use crate::utils::http_client::{AIHttpClient, AIProvider, ClientConfig};

/// 根据 AI 返回的分类名称匹配系统中的分类ID
/// AI 可能返回 "大类-小类" 格式，需要拆分后优先匹配小类
/// 优先匹配与 transaction_type 一致的分类，增强类型安全性
fn match_category_name(categories: &[Category], ai_category: &str, tx_type: &str) -> i64 {
    let ai_trimmed = ai_category.trim();
    if ai_trimmed.is_empty() {
        return 0;
    }
    let ai_lower = ai_trimmed.to_lowercase();

    // 按类型过滤分类（优先同类型，允许跨类型兜底）
    let same_type_cats: Vec<&Category> = categories.iter().filter(|c| c.r#type == tx_type).collect();
    let candidates: &[&Category] = if !same_type_cats.is_empty() {
        &same_type_cats
    } else {
        // 需要把 categories 转成引用数组，这里直接用 categories.iter() 但不太好处理
        // 简单做法：直接用 same_type_cats（可能为空），后面再兜底
        &same_type_cats
    };

    // 1. 先尝试精确匹配（优先同类型）
    if let Some(c) = categories.iter().find(|c| c.name == ai_trimmed && c.r#type == tx_type) {
        return c.id;
    }
    if let Some(c) = categories.iter().find(|c| c.name == ai_trimmed) {
        return c.id;
    }

    // 2. 拆分 "大类-小类" 格式，优先匹配小类（最后一部分）
    let parts: Vec<&str> = ai_trimmed.split('-').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if parts.len() > 1 {
        let sub_name = parts.last().unwrap();
        // 优先在同类型中匹配
        if let Some(c) = categories.iter().find(|c| c.name == *sub_name && c.r#type == tx_type) {
            return c.id;
        }
        if let Some(c) = categories.iter().find(|c| c.name == *sub_name) {
            return c.id;
        }
    }

    // 3. 尝试匹配大类（第一部分）
    if parts.len() > 1 {
        let parent_name = parts.first().unwrap();
        if let Some(c) = categories.iter().find(|c| c.name == *parent_name && c.r#type == tx_type) {
            return c.id;
        }
        if let Some(c) = categories.iter().find(|c| c.name == *parent_name) {
            return c.id;
        }
    }

    // 4. 模糊匹配：AI 名称包含分类名，或分类名包含 AI 名称（优先同类型）
    if let Some(c) = categories.iter().find(|c| {
        let c_lower = c.name.to_lowercase();
        (ai_lower.contains(&c_lower) || c_lower.contains(&ai_lower)) && c.r#type == tx_type
    }) {
        return c.id;
    }

    // 5. 跨类型兜底模糊匹配
    if let Some(c) = categories.iter().find(|c| {
        let c_lower = c.name.to_lowercase();
        ai_lower.contains(&c_lower) || c_lower.contains(&ai_lower)
    }) {
        return c.id;
    }

    0
}

pub struct QuickNoteParseTool {
    pool: SqlitePool,
    session_id: Option<String>,
    token_recorder: Option<Arc<TokenUsageRecorder>>,
    memory_dir: Option<PathBuf>,
}

impl QuickNoteParseTool {
    pub fn new(
        pool: SqlitePool,
        session_id: Option<String>,
        token_recorder: Option<Arc<TokenUsageRecorder>>,
        memory_dir: Option<PathBuf>,
    ) -> Self {
        Self { pool, session_id, token_recorder, memory_dir }
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

        // V3 记忆快照：读取 MEMORY.md
        let memory_snapshot = if let Some(ref mdir) = self.memory_dir {
            let store = crate::memory::v3::MemoryStore::new(mdir);
            store.read_file("MEMORY.md").unwrap_or_default()
        } else {
            String::new()
        };

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

        let db = Database {
            pool: self.pool.clone(),
        };
        // 查询所有分类用于匹配 AI 识别的分类名称
        let categories = db.get_categories().await.unwrap_or_default();

        let mut candidates = Vec::new();
        let mut items = Vec::new();
        for t in parsed.transactions {
            // 根据 AI 识别的分类名称尝试匹配分类ID
            // AI 可能返回 "大类-小类" 格式，需要拆分后分别匹配
            let category_id = match_category_name(&categories, &t.category, &t.transaction_type);

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
                category_id,
                budget_id: None,
                description: t.remark,
                raw_category_name: Some(t.category),
            });
        }
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
