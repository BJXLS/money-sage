use crate::mcp::McpTool;
use crate::models::{CategoryStats, MonthlyStats};
use crate::utils::http_client::{AIMessage, AIRequest};
use chrono::Local;

/// 财务上下文快照（注入 system prompt）
pub struct FinancialContext {
    pub monthly_stats: Vec<MonthlyStats>,
    pub expense_category_stats: Vec<CategoryStats>,
    pub income_category_stats: Vec<CategoryStats>,
}

/// MCP 工具上下文（让 Agent 知道有哪些外部工具可用）
pub struct McpToolsContext {
    pub tools: Vec<(String, McpTool)>,
}

/// 智能分析 Agent —— 负责构建 prompt 和组装请求，实际调用由 lib.rs 处理
pub struct AnalysisAgent {
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub enable_thinking: bool,
}

impl AnalysisAgent {
    pub fn new(model: String, temperature: f32, max_tokens: u32, enable_thinking: bool) -> Self {
        Self {
            model,
            temperature,
            max_tokens,
            enable_thinking,
        }
    }

    #[deprecated(note = "使用 workspace::SystemPromptBuilder 构建 system prompt")]
    pub fn build_system_prompt(&self, _ctx: &FinancialContext) -> String {
        format!(
            "{}\n\n{}",
            self.build_tool_guide(None),
            self.build_time_context()
        )
    }

    #[deprecated(note = "使用 workspace::SystemPromptBuilder 构建 system prompt")]
    pub fn build_system_prompt_with_tools(
        &self,
        _ctx: &FinancialContext,
        mcp_ctx: Option<&McpToolsContext>,
    ) -> String {
        format!(
            "{}\n\n{}",
            self.build_tool_guide(mcp_ctx),
            self.build_time_context()
        )
    }

    /// 构建动态工具指南部分（由 workspace builder 拼接在静态文件之后）
    ///
    /// 这里只保留**策略性内容**：跨工具调用顺序、协作策略等。
    /// 单个工具的功能定义和参数说明由工具自身的 `description()` 和 `parameters_schema` 提供。
    pub fn build_tool_guide(&self, mcp_ctx: Option<&McpToolsContext>) -> String {
        let mut p = String::new();

        // 如果有 MCP 工具可用，在 system prompt 中告知 Agent
        if let Some(mcp) = mcp_ctx {
            if !mcp.tools.is_empty() {
                p.push_str("## 可用的外部工具\n\n");
                p.push_str("以下外部工具已通过 MCP 连接，你可以调用，解决用户的问题：\n\n");
                for (server, tool) in &mcp.tools {
                    p.push_str(&format!(
                        "- **{}** (来自 {}): {}\n",
                        tool.name,
                        server,
                        tool.description.as_deref().unwrap_or("无描述")
                    ));
                }
                p.push('\n');
            }
        }

        p.push_str(
            "## 工具使用策略\n\n\
             当用户提出需要具体数据才能回答的问题时，请按以下顺序调用工具：\n\
             1. 先调用 get_database_schema 了解表结构\n\
             2. 然后调用 query_database 执行 SQL 查询获取数据\n\
             3. 基于查询结果给出分析和建议\n\n\
             ### 跨工具协作策略\n\
             - 使用 file_edit 修改文件前，建议先调用 file_read 查看当前内容，确保 old_string 精确匹配。\n\
             - file_write 可以完全覆盖已有文件，覆盖前请确认意图。\n\
             - bash 命令只能在工作区内操作，禁止访问系统敏感路径。\n"
        );

        p
    }

    /// 构建当前时间上下文
    pub fn build_time_context(&self) -> String {
        let now_local = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        format!(
            "## 当前时间\n\n\
             以下是用户设备上的当前本地时间（用于理解「今天」「本月」「上周」「最近 30 天」等表述，并在 SQL 中编写正确的日期条件）：\n\
             - **本地时间**：{now_local}\n"
        )
    }

    /// 组装完整的消息列表：system + history(最近 20 条) + 当前 user
    pub fn build_messages(
        &self,
        user_message: &str,
        history: &[AIMessage],
        ctx: &FinancialContext,
    ) -> Vec<AIMessage> {
        let mut msgs = vec![AIMessage::text("system", self.build_system_prompt(ctx))];

        let max_history = 20;
        let start = if history.len() > max_history {
            history.len() - max_history
        } else {
            0
        };
        for msg in &history[start..] {
            msgs.push(msg.clone());
        }

        msgs.push(AIMessage::text("user", user_message));

        msgs
    }

    /// 构建 AIRequest（由 lib.rs 拿去调 http_client）
    pub fn build_request(
        &self,
        user_message: &str,
        history: &[AIMessage],
        ctx: &FinancialContext,
    ) -> AIRequest {
        self.build_request_with_tools(user_message, history, ctx, None)
    }

    /// 构建带工具定义的 AIRequest
    pub fn build_request_with_tools(
        &self,
        user_message: &str,
        history: &[AIMessage],
        ctx: &FinancialContext,
        tools: Option<Vec<serde_json::Value>>,
    ) -> AIRequest {
        AIRequest {
            model: self.model.clone(),
            messages: self.build_messages(user_message, history, ctx),
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stream: None,
            enable_thinking: self.enable_thinking,
            tools,
            tool_choice: None,
        }
    }
}
