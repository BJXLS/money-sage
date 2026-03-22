use crate::models::{CategoryStats, MonthlyStats};
use crate::mcp::McpTool;
use crate::utils::http_client::{AIMessage, AIRequest};

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

    pub fn build_system_prompt(&self, ctx: &FinancialContext) -> String {
        self.build_system_prompt_with_tools(ctx, None)
    }

    pub fn build_system_prompt_with_tools(
        &self,
        ctx: &FinancialContext,
        mcp_ctx: Option<&McpToolsContext>,
    ) -> String {
        let mut p = String::from(
            "你是一位专业的个人财务分析师。用户正在使用记账软件，你需要基于他们的真实财务数据回答问题、提供分析和建议。\n\
             请使用 Markdown 格式回复，保持简洁、有条理、有数据支撑。\n\n\
             ## 用户财务数据摘要\n\n"
        );

        if !ctx.monthly_stats.is_empty() {
            p.push_str(
                "### 近几月收支\n\n| 月份 | 收入 | 支出 | 结余 |\n|------|------|------|------|\n",
            );
            for s in &ctx.monthly_stats {
                p.push_str(&format!(
                    "| {} | {:.2} | {:.2} | {:.2} |\n",
                    s.month, s.income, s.expense, s.balance
                ));
            }
            p.push('\n');
        } else {
            p.push_str("暂无月度统计数据。\n\n");
        }

        if !ctx.expense_category_stats.is_empty() {
            p.push_str("### 本月支出分类\n\n| 分类 | 金额 | 占比 |\n|------|------|------|\n");
            for s in &ctx.expense_category_stats {
                p.push_str(&format!(
                    "| {} | {:.2} | {:.1}% |\n",
                    s.category_name, s.amount, s.percentage
                ));
            }
            p.push('\n');
        }

        if !ctx.income_category_stats.is_empty() {
            p.push_str("### 本月收入分类\n\n| 分类 | 金额 | 占比 |\n|------|------|------|\n");
            for s in &ctx.income_category_stats {
                p.push_str(&format!(
                    "| {} | {:.2} | {:.1}% |\n",
                    s.category_name, s.amount, s.percentage
                ));
            }
            p.push('\n');
        }

        // 如果有 MCP 工具可用，在 system prompt 中告知 Agent
        if let Some(mcp) = mcp_ctx {
            if !mcp.tools.is_empty() {
                p.push_str("## 可用的外部工具\n\n");
                p.push_str("以下外部工具已通过 MCP 连接，你可以告知用户这些功能可用：\n\n");
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

        p.push_str("请根据以上数据回答用户的问题。如果数据不足以回答，请坦诚说明。\n");
        p
    }

    /// 组装完整的消息列表：system + history(最近 20 条) + 当前 user
    pub fn build_messages(
        &self,
        user_message: &str,
        history: &[AIMessage],
        ctx: &FinancialContext,
    ) -> Vec<AIMessage> {
        let mut msgs = vec![AIMessage {
            role: "system".to_string(),
            content: self.build_system_prompt(ctx),
        }];

        let max_history = 20; // 10 轮
        let start = if history.len() > max_history {
            history.len() - max_history
        } else {
            0
        };
        for msg in &history[start..] {
            msgs.push(msg.clone());
        }

        msgs.push(AIMessage {
            role: "user".to_string(),
            content: user_message.to_string(),
        });

        msgs
    }

    /// 构建 AIRequest（由 lib.rs 拿去调 http_client）
    pub fn build_request(
        &self,
        user_message: &str,
        history: &[AIMessage],
        ctx: &FinancialContext,
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
        }
    }
}
