//! 智能分析流水线（从 `lib.rs::send_analysis_message_stream` 抽出的可复用 service）
//!
//! 该模块只负责"跑一次完整的智能分析对话":
//! 1. 取 LLM 配置（按 `config_id` 优先，否则取 active）；
//! 2. 读历史 + 确保会话存在 + 落库 user 消息（带 `source` 标记）；
//! 3. 构 system prompt（FinancialContext + Memory snapshot + MCP tools 提示）；
//! 4. SSE 流式调用 + 工具调用循环（最多 8 轮）；
//! 5. 把 chunk / tool_status / done / error 事件推给 [`MessageSink`]；
//! 6. 落库 assistant tool_call / tool result / 最终文本（同样带 `source`）；
//! 7. 记录 token 用量。
//!
//! 流水线本身不感知是来自 Vue 还是飞书；通道差异完全由 [`MessageSink`] 的具体实现决定。

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{Datelike, Local, NaiveDate};

use crate::ai::agent::analysis::{AnalysisAgent, FinancialContext, McpToolsContext};
use crate::ai::tools::LocalToolRegistry;
use crate::database::Database;
use crate::mcp::McpManager;
use crate::memory::MemoryFacade;
use crate::models::{self, SessionSource};
use crate::pipeline::sink::{MessageSink, ToolStatus, ToolStatusKind};
use crate::telemetry::TokenUsageRecorder;
use crate::utils::http_client::{
    AIHttpClient, AIMessage, AIProvider, AIRequest, ClientConfig, FunctionCall, ToolCall,
};

const MAX_TOOL_ROUNDS: usize = 8;
const HISTORY_LIMIT: i64 = 20;

/// 入参（与原 `AnalysisStreamRequest` + 来源标记同构）
#[derive(Debug, Clone)]
pub struct AnalysisInput {
    pub session_id: String,
    pub user_message: String,
    pub config_id: Option<i64>,
    pub source: SessionSource,
}

/// 跑一次完整的智能分析流水线
///
/// 入参职责切分：
/// - `db / memory / token_recorder / mcp_manager`：现有桌面应用的基础设施；
/// - `input`：本次对话的输入（含来源标记）；
/// - `sink`：负责把流水线事件投递给具体通道（Vue / 飞书 / 测试 mock）。
///
/// 注意：本函数总是返回 `Ok(())`，所有错误都通过 [`MessageSink::on_error`] 暴露给调用方，
/// 这样 Tauri 命令薄壳层可以只 `?` 一次（与改造前 `send_analysis_message_stream` 行为一致）。
pub async fn run(
    db: &Database,
    memory: &Arc<MemoryFacade>,
    token_recorder: &Arc<TokenUsageRecorder>,
    mcp_manager: &McpManager,
    input: AnalysisInput,
    sink: &dyn MessageSink,
) -> Result<(), String> {
    let AnalysisInput {
        session_id: sid,
        user_message,
        config_id,
        source,
    } = input;
    let source_str = source.as_str();

    if user_message.trim().is_empty() {
        sink.on_error("请输入您的问题").await;
        return Ok(());
    }

    // 1. 获取 LLM 配置（按 config_id 优先，否则 fallback 到 active）
    let llm_config = match config_id {
        Some(cid) => match db.get_llm_config_by_id(cid).await {
            Ok(Some(c)) => c,
            _ => match db.get_active_llm_config().await {
                Ok(Some(c)) => c,
                _ => {
                    sink.on_error("请先在设置中配置大模型接口").await;
                    return Ok(());
                }
            },
        },
        None => match db.get_active_llm_config().await {
            Ok(Some(c)) => c,
            _ => {
                sink.on_error("请先在设置中配置大模型接口").await;
                return Ok(());
            }
        },
    };

    // 2. 读取历史（在保存本轮 user 消息之前，20 条 = 10 轮）
    let history_records = db
        .get_recent_analysis_messages(&sid, HISTORY_LIMIT)
        .await
        .unwrap_or_default();
    let history: Vec<AIMessage> = history_records
        .iter()
        .map(|r| AIMessage::text(&r.role, &r.content))
        .collect();

    // 3. 确保会话存在 + 持久化 user 消息（均带 source 标记）
    let title: String = user_message.chars().take(30).collect();
    if let Err(e) = db
        .ensure_analysis_session(&sid, &title, config_id, source_str)
        .await
    {
        sink.on_error(&format!("创建会话失败: {}", e)).await;
        return Ok(());
    }
    let _ = db
        .save_analysis_message_ext(
            &sid,
            "user",
            &user_message,
            "text",
            None,
            None,
            None,
            source_str,
        )
        .await;

    // 4. 构建 HTTP 客户端
    let client_config = ClientConfig {
        provider: AIProvider::Custom(llm_config.provider.clone()),
        base_url: llm_config.base_url.clone(),
        api_key: llm_config.api_key.clone(),
        timeout_secs: 600,
        max_retries: 1,
        headers: HashMap::new(),
    };
    let http_client = match AIHttpClient::new(client_config) {
        Ok(c) => c,
        Err(e) => {
            sink.on_error(&format!("创建 HTTP 客户端失败: {}", e)).await;
            return Ok(());
        }
    };

    // 5. 构建财务上下文（当前月份 + 最近 3 个月统计）
    let today = Local::now();
    let month_start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
    let month_end = {
        let next = if today.month() == 12 {
            NaiveDate::from_ymd_opt(today.year() + 1, 1, 1)
        } else {
            NaiveDate::from_ymd_opt(today.year(), today.month() + 1, 1)
        };
        next.and_then(|d| d.pred_opt()).unwrap_or(month_start)
    };

    let financial_context = FinancialContext {
        monthly_stats: db.get_monthly_stats(3).await.unwrap_or_default(),
        expense_category_stats: db
            .get_category_stats(&month_start, &month_end, "expense")
            .await
            .unwrap_or_default(),
        income_category_stats: db
            .get_category_stats(&month_start, &month_end, "income")
            .await
            .unwrap_or_default(),
    };

    // 6. 收集 MCP 工具上下文
    let mcp_tools = mcp_manager.get_all_tools().await;
    let mcp_ctx = if mcp_tools.is_empty() {
        None
    } else {
        Some(McpToolsContext { tools: mcp_tools })
    };

    // 7. 创建本地工具注册表（含数据库 / 内存 / token 记录器引用）
    let tool_registry = LocalToolRegistry::new(
        db.pool.clone(),
        Some(sid.clone()),
        Some(token_recorder.clone()),
        memory.clone(),
    );
    let tools_json = tool_registry.all_as_openai_tools();
    let tools = if tools_json.is_empty() {
        None
    } else {
        Some(tools_json)
    };

    // 8. 组装初始消息
    let agent = AnalysisAgent::new(
        llm_config.model.clone(),
        llm_config.temperature as f32,
        llm_config.max_tokens as u32,
        llm_config.enable_thinking,
    );

    let analysis_snapshot = memory.render_analysis_snapshot().await.unwrap_or_default();
    let system_prompt = format!(
        "{}\n\n## Memory Snapshot\n{}",
        agent.build_system_prompt_with_tools(&financial_context, mcp_ctx.as_ref()),
        analysis_snapshot
    );
    let mut messages: Vec<AIMessage> = vec![AIMessage::text("system", system_prompt)];
    let max_history = HISTORY_LIMIT as usize;
    let start = if history.len() > max_history {
        history.len() - max_history
    } else {
        0
    };
    messages.extend_from_slice(&history[start..]);
    messages.push(AIMessage::text("user", &user_message));

    // 9. Tool call loop：支持多轮工具调用
    let mut full_content = String::new();
    let request_id = uuid::Uuid::new_v4().to_string();
    let mut round_index: i32 = 0;

    for _round in 0..MAX_TOOL_ROUNDS {
        let ai_request = AIRequest {
            model: llm_config.model.clone(),
            messages: messages.clone(),
            temperature: llm_config.temperature as f32,
            max_tokens: llm_config.max_tokens as u32,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stream: None,
            enable_thinking: llm_config.enable_thinking,
            tools: tools.clone(),
            tool_choice: None,
        };

        let round_started = std::time::Instant::now();
        let mut response = match http_client.chat_completion_stream(ai_request).await {
            Ok(r) => r,
            Err(e) => {
                let duration_ms = round_started.elapsed().as_millis() as i64;
                let rec = models::TokenUsageRecord {
                    agent_name: "AnalysisAgent".into(),
                    session_id: Some(sid.clone()),
                    request_id: request_id.clone(),
                    round_index,
                    config_id: Some(llm_config.id),
                    config_name_snapshot: Some(llm_config.config_name.clone()),
                    provider: llm_config.provider.clone(),
                    model: llm_config.model.clone(),
                    prompt_tokens: 0,
                    completion_tokens: 0,
                    total_tokens: 0,
                    finish_reason: None,
                    duration_ms: Some(duration_ms),
                    success: false,
                    error_message: Some(e.to_string()),
                };
                if let Err(re) = token_recorder.record(rec).await {
                    eprintln!("[Analysis] 记录 token 用量失败: {}", re);
                }
                sink.on_error(&format!("请求失败: {}", e)).await;
                return Ok(());
            }
        };

        // 解析 SSE，同时收集 content / tool_calls / usage
        let mut round_content = String::new();
        let mut tool_calls_acc: HashMap<usize, (String, String, String)> = HashMap::new();
        let mut finish_reason = String::new();
        let mut buffer = String::new();
        let mut usage_prompt: i32 = 0;
        let mut usage_completion: i32 = 0;
        let mut usage_total: i32 = 0;
        let mut response_model = String::new();
        let mut stream_error: Option<String> = None;

        loop {
            match response.chunk().await {
                Ok(Some(chunk)) => {
                    buffer.push_str(&String::from_utf8_lossy(&chunk));

                    while let Some(pos) = buffer.find('\n') {
                        let line = buffer[..pos].trim_end_matches('\r').to_string();
                        buffer = buffer[pos + 1..].to_string();

                        if !line.starts_with("data: ") {
                            continue;
                        }
                        let data = line[6..].trim();
                        if data == "[DONE]" {
                            continue;
                        }

                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                            // 服务端在末尾块返回的模型名 / usage
                            if let Some(m) = json["model"].as_str() {
                                if response_model.is_empty() {
                                    response_model = m.to_string();
                                }
                            }
                            if let Some(usage) = json["usage"].as_object() {
                                usage_prompt = usage
                                    .get("prompt_tokens")
                                    .and_then(|v| v.as_i64())
                                    .unwrap_or(0) as i32;
                                usage_completion = usage
                                    .get("completion_tokens")
                                    .and_then(|v| v.as_i64())
                                    .unwrap_or(0) as i32;
                                usage_total =
                                    usage.get("total_tokens").and_then(|v| v.as_i64()).unwrap_or(
                                        (usage_prompt + usage_completion) as i64,
                                    ) as i32;
                            }

                            // 文本内容 delta
                            if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                                if !content.is_empty() {
                                    round_content.push_str(content);
                                    full_content.push_str(content);
                                    sink.on_chunk(content).await;
                                }
                            }

                            // tool_calls delta（增量累积）
                            if let Some(tcs) =
                                json["choices"][0]["delta"]["tool_calls"].as_array()
                            {
                                for tc in tcs {
                                    let idx = tc["index"].as_u64().unwrap_or(0) as usize;
                                    let entry = tool_calls_acc.entry(idx).or_insert_with(|| {
                                        (String::new(), String::new(), String::new())
                                    });
                                    if let Some(id) = tc["id"].as_str() {
                                        entry.0 = id.to_string();
                                    }
                                    if let Some(name) = tc["function"]["name"].as_str() {
                                        entry.1.push_str(name);
                                    }
                                    if let Some(args) = tc["function"]["arguments"].as_str() {
                                        entry.2.push_str(args);
                                    }
                                }
                            }

                            // finish_reason
                            if let Some(reason) = json["choices"][0]["finish_reason"].as_str() {
                                finish_reason = reason.to_string();
                            }
                        }
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    stream_error = Some(format!("流式读取失败: {}", e));
                    break;
                }
            }
        }

        // 记录本轮 token 用量
        let duration_ms = round_started.elapsed().as_millis() as i64;
        let rec = models::TokenUsageRecord {
            agent_name: "AnalysisAgent".into(),
            session_id: Some(sid.clone()),
            request_id: request_id.clone(),
            round_index,
            config_id: Some(llm_config.id),
            config_name_snapshot: Some(llm_config.config_name.clone()),
            provider: llm_config.provider.clone(),
            model: if response_model.is_empty() {
                llm_config.model.clone()
            } else {
                response_model.clone()
            },
            prompt_tokens: usage_prompt,
            completion_tokens: usage_completion,
            total_tokens: usage_total,
            finish_reason: if finish_reason.is_empty() {
                None
            } else {
                Some(finish_reason.clone())
            },
            duration_ms: Some(duration_ms),
            success: stream_error.is_none(),
            error_message: stream_error.clone(),
        };
        if let Err(re) = token_recorder.record(rec).await {
            eprintln!("[Analysis] 记录 token 用量失败: {}", re);
        }
        round_index += 1;

        if let Some(err_msg) = stream_error {
            sink.on_error(&err_msg).await;
            return Ok(());
        }

        // 判断是否需要执行工具
        if finish_reason == "tool_calls" && !tool_calls_acc.is_empty() {
            let mut sorted_indices: Vec<usize> = tool_calls_acc.keys().cloned().collect();
            sorted_indices.sort();

            let tool_calls: Vec<ToolCall> = sorted_indices
                .iter()
                .map(|idx| {
                    let (id, name, args) = tool_calls_acc.get(idx).unwrap();
                    ToolCall {
                        id: id.clone(),
                        call_type: "function".to_string(),
                        function: FunctionCall {
                            name: name.clone(),
                            arguments: args.clone(),
                        },
                    }
                })
                .collect();

            // 追加 assistant 消息（含 tool_calls）
            messages.push(AIMessage::assistant_tool_calls(
                tool_calls.clone(),
                if round_content.is_empty() {
                    None
                } else {
                    Some(round_content)
                },
            ));

            // 执行每个工具并追加结果
            for tc in &tool_calls {
                // 落表：assistant tool_call（带 source）
                let tool_calls_json_str = serde_json::to_string(&[serde_json::json!({
                    "id": tc.id,
                    "name": tc.function.name,
                    "arguments": tc.function.arguments,
                })])
                .unwrap_or_default();
                let _ = db
                    .save_analysis_message_ext(
                        &sid,
                        "assistant",
                        "",
                        "tool_call",
                        Some(&tool_calls_json_str),
                        None,
                        Some(&tc.function.name),
                        source_str,
                    )
                    .await;

                // 推送 tool_status: calling（附 tool_input）
                sink.on_tool_status(ToolStatus {
                    tool_name: tc.function.name.clone(),
                    kind: ToolStatusKind::Calling,
                    description: Some(format!("正在调用: {}", tc.function.name)),
                    tool_input: Some(tc.function.arguments.clone()),
                    tool_output: None,
                })
                .await;

                let args: serde_json::Value =
                    serde_json::from_str(&tc.function.arguments).unwrap_or(serde_json::json!({}));

                let result = match tool_registry.execute(&tc.function.name, args).await {
                    Ok(r) => r,
                    Err(e) => format!("工具执行失败: {}", e),
                };

                // 落表：tool result（带 source）
                let _ = db
                    .save_analysis_message_ext(
                        &sid,
                        "tool",
                        &result,
                        "tool_result",
                        None,
                        Some(&tc.id),
                        Some(&tc.function.name),
                        source_str,
                    )
                    .await;

                // 推送 tool_status: result（附 tool_output）
                sink.on_tool_status(ToolStatus {
                    tool_name: tc.function.name.clone(),
                    kind: ToolStatusKind::Result,
                    description: None,
                    tool_input: None,
                    tool_output: Some(result.clone()),
                })
                .await;

                messages.push(AIMessage::tool_result(&tc.id, result));
            }

            continue;
        }

        // 正常结束（finish_reason == "stop" 或其他）
        break;
    }

    // 10. 推 done 事件（携带完整文本）
    sink.on_done(&full_content).await;

    // 11. 持久化 assistant 回复 + 更新会话时间戳（带 source）
    if !full_content.is_empty() {
        let _ = db
            .save_analysis_message_ext(
                &sid,
                "assistant",
                &full_content,
                "text",
                None,
                None,
                None,
                source_str,
            )
            .await;
    }
    let _ = db.touch_analysis_session(&sid).await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::sink::test_support::{RecordingSink, SinkEvent};
    use crate::tests::create_test_database_state;

    /// 没有配置 active LLM 时，run 应该立即通过 sink 报错并不写入消息表。
    #[tokio::test]
    async fn run_returns_error_when_no_llm_config() {
        let state = create_test_database_state().await.expect("test state");
        let mcp = McpManager::new();
        let sink = RecordingSink::new();

        // create_test_database 会预置一条激活的 LLM 配置，先把所有配置都置为 inactive，
        // 制造"既没有活跃配置，也没有匹配 config_id 的配置"的边界场景。
        sqlx::query("UPDATE llm_configs SET is_active = 0")
            .execute(&state.db.pool)
            .await
            .expect("deactivate all llm configs");
        let active = state
            .db
            .get_active_llm_config()
            .await
            .expect("active llm");
        assert!(active.is_none(), "测试前提：deactivate 后没有活跃 LLM 配置");

        let input = AnalysisInput {
            session_id: "sess-no-cfg".into(),
            user_message: "你好".into(),
            config_id: Some(99999), // 不存在
            source: SessionSource::Local,
        };

        run(
            &state.db,
            &state.memory,
            &state.token_recorder,
            &mcp,
            input,
            &sink,
        )
        .await
        .expect("run returns Ok even on logical errors");

        let events = sink.snapshot();
        assert_eq!(events.len(), 1, "只应有一个 error 事件: {:?}", events);
        match &events[0] {
            SinkEvent::Error(msg) => assert!(msg.contains("大模型")),
            other => panic!("expected Error event, got {:?}", other),
        }
    }

    /// 空消息直接报错，不创建会话。
    #[tokio::test]
    async fn run_rejects_empty_message() {
        let state = create_test_database_state().await.expect("test state");
        let mcp = McpManager::new();
        let sink = RecordingSink::new();

        let input = AnalysisInput {
            session_id: "sess-empty".into(),
            user_message: "   ".into(),
            config_id: None,
            source: SessionSource::Local,
        };

        run(
            &state.db,
            &state.memory,
            &state.token_recorder,
            &mcp,
            input,
            &sink,
        )
        .await
        .expect("run returns Ok");

        let events = sink.snapshot();
        assert_eq!(events.len(), 1);
        match &events[0] {
            SinkEvent::Error(msg) => assert!(msg.contains("请输入")),
            other => panic!("expected Error event, got {:?}", other),
        }
        // 会话不应被创建（因为空消息提前返回）
        let sessions = state.db.get_analysis_sessions().await.expect("list sessions");
        assert!(
            sessions.iter().all(|s| s.id != "sess-empty"),
            "空消息不应创建会话, 实际: {:?}",
            sessions
        );
    }
}
