/**
 * 你是一个记账记录实体识别机器人。你的目标是将用户输入的句子，进行结构化的解析，识别出其中的关键信息，并以结构化的格式进行返回。
## 规则
1. 用户输入的会是多个句子，每个句子用结构化的list分割，你需要将每个独立的句子进行实体识别
2. 你需要先识别用户句子中的时间，记录为dateTime
3. 你需要识别用户句子中花费或赚取的金额，记录为money
4. 你需要识别用户句子中的花费或赚取的事件，并对此事件进行分类，可选的分类值为···房租、吃饭、教育···，记录为category
5. 你需要将此次花费的主题进行总结，记录为remark

## output
{
"transactions":[
{"id":"id","dateTime":"2025-07-20", "money":10,"category":"吃饭","remark":"周末吃饭"}
]
}
 */

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::Result;
use chrono::{Local, NaiveDate};

use crate::utils::http_client::AIHttpClient;
use super::base::{Agent, AgentConfig, AgentContext, AgentResult};

/// 快速记账交易记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickTransaction {
    /// 日期时间
    pub date_time: String,
    /// 金额
    pub amount: f64,
    /// 交易类型 (income/expense)
    pub transaction_type: String,
    /// 分类
    pub category: String,
    /// 备注/描述
    pub remark: String,
}

/// 快速记账解析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickNoteResult {
    /// 解析的交易记录列表
    pub transactions: Vec<QuickTransaction>,
    /// 解析说明
    pub explanation: Option<String>,
}

/// 快速记账AI代理
/// 用于解析自然语言输入并提取记账信息
pub struct QuickNoteAgent {
    config: AgentConfig,
}

impl QuickNoteAgent {
    /// 创建新的快速记账代理
    pub fn new() -> Self {
        let config = AgentConfig {
            name: "QuickNoteAgent".to_string(),
            description: "AI agent for parsing natural language into accounting transactions".to_string(),
            model: "qwen-turbo".to_string(),
            temperature: 0.3, // 降低温度以获得更一致的结果
            max_tokens: 1000,
            system_prompt: Self::build_system_prompt(),
            enable_memory: false, // 记账解析不需要记忆
            max_memory_size: 0,
            custom_params: HashMap::new(),
        };
        
        Self { config }
    }
    
    /// 构建系统提示词
    fn build_system_prompt() -> String {
        r#"你是一个专业的记账记录实体识别机器人。你的任务是将用户输入的自然语言文本解析为结构化的记账信息。

## 核心规则

1. **输入解析**: 用户可能输入多个记账记录，每个记录包含时间、金额、类型和描述信息
2. **时间识别**: 识别并解析时间信息，格式为 YYYY-MM-DD。如果没有明确时间，使用今天的日期
3. **金额识别**: 识别数字金额，支持小数点，去除货币符号
4. **类型判断**: 判断是收入(income)还是支出(expense)
   - 收入关键词: 收入、赚了、工资、奖金、红包、转账收到等
   - 支出关键词: 花了、买了、支付、消费、转账给等
5. **分类识别**: 将消费场景映射到标准分类
   - 餐饮: 吃饭、喝咖啡、外卖、餐厅等
   - 交通: 打车、地铁、公交、加油等  
   - 购物: 买衣服、超市、网购等
   - 娱乐: 电影、游戏、KTV等
   - 住房: 房租、水电费、物业费等
   - 医疗: 看病、买药、体检等
   - 教育: 培训、买书、学费等
   - 其他支出: 无法明确分类的支出
   - 工资: 工资收入
   - 其他收入: 其他类型收入

## 输出格式

严格按照以下JSON格式输出，不要添加任何额外的文字说明：

```json
{
  "transactions": [
    {
      "date_time": "2025-01-20",
      "amount": 28.5,
      "transaction_type": "expense",
      "category": "餐饮",
      "remark": "午餐"
    }
  ],
  "explanation": "解析了1条记录"
}
```

## 示例

用户输入: "今天中午花了28.5元吃午餐，晚上打车回家15元"
输出:
```json
{
  "transactions": [
    {
      "date_time": "2025-01-20",
      "amount": 28.5,
      "transaction_type": "expense", 
      "category": "餐饮",
      "remark": "中午午餐"
    },
    {
      "date_time": "2025-01-20",
      "amount": 15.0,
      "transaction_type": "expense",
      "category": "交通", 
      "remark": "晚上打车回家"
    }
  ],
  "explanation": "解析了2条支出记录"
}
```"#.to_string()
    }
    
    /// 解析快速记账文本
    pub async fn parse_quick_note(&self, input: &str, client: &AIHttpClient) -> Result<QuickNoteResult> {
        // 验证输入
        self.validate_input(input)?;
        
        // 创建上下文
        let mut context = AgentContext::new("quick_note_session".to_string());
        
        // 添加当前日期到上下文
        let today = Local::now().format("%Y-%m-%d").to_string();
        context.add_data("current_date".to_string(), serde_json::Value::String(today));
        
        // 预处理输入，添加日期信息
        let processed_input = format!("当前日期: {}\n用户输入: {}", 
            Local::now().format("%Y-%m-%d"), input);
        
        // 构建消息
        let messages = self.build_prompt(&processed_input, &context).await?;
        
        // 调用AI模型
        let response = self.call_ai(messages, client).await?;
        
        // 解析AI响应
        let content = response.choices.get(0)
            .ok_or_else(|| anyhow::anyhow!("AI响应中没有choices"))?
            .message.content.clone();
        self.parse_ai_response(&content).await
    }
    
    /// 解析AI响应为结构化数据
    async fn parse_ai_response(&self, content: &str) -> Result<QuickNoteResult> {
        // 尝试从响应中提取JSON
        let json_content = self.extract_json_from_response(content)?;
        
        // 解析JSON
        let result: QuickNoteResult = serde_json::from_str(&json_content)
            .map_err(|e| anyhow::anyhow!("Failed to parse AI response as JSON: {}", e))?;
        
        // 验证解析结果
        self.validate_parse_result(&result)?;
        
        Ok(result)
    }
    
    /// 从响应文本中提取JSON内容
    pub fn extract_json_from_response(&self, content: &str) -> Result<String> {
        // 查找JSON代码块
        if let Some(start) = content.find("```json") {
            let json_start = start + 7; // "```json".len()
            if let Some(end_pos) = content[json_start..].find("```") {
                let json_end = json_start + end_pos;
                return Ok(content[json_start..json_end].trim().to_string());
            }
        }
        
        // 查找纯JSON块（以{开始，以}结束）
        if let Some(start) = content.find('{') {
            if let Some(end) = content.rfind('}') {
                if end > start {
                    return Ok(content[start..=end].to_string());
                }
            }
        }
        
        // 如果没找到JSON，返回整个内容
        Ok(content.trim().to_string())
    }
    
    /// 验证解析结果
    pub fn validate_parse_result(&self, result: &QuickNoteResult) -> Result<()> {
        if result.transactions.is_empty() {
            return Err(anyhow::anyhow!("No transactions parsed from input"));
        }
        
        for (i, transaction) in result.transactions.iter().enumerate() {
            // 验证日期格式
            if NaiveDate::parse_from_str(&transaction.date_time, "%Y-%m-%d").is_err() {
                return Err(anyhow::anyhow!("Invalid date format in transaction {}: {}", 
                    i + 1, transaction.date_time));
            }
            
            // 验证金额
            if transaction.amount <= 0.0 {
                return Err(anyhow::anyhow!("Invalid amount in transaction {}: {}", 
                    i + 1, transaction.amount));
            }
            
            // 验证交易类型
            if !["income", "expense"].contains(&transaction.transaction_type.as_str()) {
                return Err(anyhow::anyhow!("Invalid transaction type in transaction {}: {}", 
                    i + 1, transaction.transaction_type));
            }
            
            // 验证分类不为空
            if transaction.category.trim().is_empty() {
                return Err(anyhow::anyhow!("Empty category in transaction {}", i + 1));
            }
        }
        
        Ok(())
    }
}

#[async_trait]
impl Agent for QuickNoteAgent {
    fn name(&self) -> &str {
        &self.config.name
    }
    
    fn description(&self) -> &str {
        &self.config.description
    }
    
    fn config(&self) -> &AgentConfig {
        &self.config
    }
    
    fn update_config(&mut self, config: AgentConfig) {
        self.config = config;
    }
    
    async fn process(&self, input: &str, context: &mut AgentContext) -> Result<AgentResult> {
        // 验证输入
        self.validate_input(input)?;
        
        // 预处理输入
        let processed_input = self.preprocess_input(input, context).await?;
        
        // 构建提示词
        let messages = self.build_prompt(&processed_input, context).await?;
        
        // 这里需要HTTP客户端，但Agent trait设计上没有提供
        // 所以直接返回成功，实际处理在parse_quick_note方法中
        Ok(AgentResult::success("快速记账代理已准备就绪".to_string()))
    }
    
    async fn preprocess_input(&self, input: &str, _context: &AgentContext) -> Result<String> {
        // 添加日期上下文
        let today = Local::now().format("%Y-%m-%d").to_string();
        Ok(format!("当前日期: {}\n用户输入: {}", today, input))
    }
    
    async fn postprocess_output(&self, output: &str, _context: &AgentContext) -> Result<String> {
        // 对于快速记账，后处理主要是确保输出格式正确
        Ok(output.to_string())
    }
}

impl Default for QuickNoteAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extract_json_from_response() {
        let agent = QuickNoteAgent::new();
        
        // 测试JSON代码块
        let content = r#"这是一个解析结果：
```json
{"transactions": [{"amount": 10.0}]}
```
"#;
        let result = agent.extract_json_from_response(content).unwrap();
        assert_eq!(result, r#"{"transactions": [{"amount": 10.0}]}"#);
        
        // 测试纯JSON
        let content = r#"{"transactions": [{"amount": 20.0}]}"#;
        let result = agent.extract_json_from_response(content).unwrap();
        assert_eq!(result, r#"{"transactions": [{"amount": 20.0}]}"#);
    }
    
    #[test]
    fn test_validate_parse_result() {
        let agent = QuickNoteAgent::new();
        
        let valid_result = QuickNoteResult {
            transactions: vec![QuickTransaction {
                date_time: "2025-01-20".to_string(),
                amount: 10.0,
                transaction_type: "expense".to_string(),
                category: "餐饮".to_string(),
                remark: "午餐".to_string(),
            }],
            explanation: Some("解析了1条记录".to_string()),
        };
        
        assert!(agent.validate_parse_result(&valid_result).is_ok());
        
        // 测试无效金额
        let invalid_result = QuickNoteResult {
            transactions: vec![QuickTransaction {
                date_time: "2025-01-20".to_string(),
                amount: -10.0,
                transaction_type: "expense".to_string(),
                category: "餐饮".to_string(),
                remark: "午餐".to_string(),
            }],
            explanation: None,
        };
        
        assert!(agent.validate_parse_result(&invalid_result).is_err());
    }
} 