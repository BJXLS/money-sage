use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::Result;

use crate::utils::http_client::{AIHttpClient, AIRequest, AIResponse, AIMessage};

/// Agent执行上下文
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContext {
    /// 用户ID（可选）
    pub user_id: Option<String>,
    /// 会话ID
    pub session_id: String,
    /// 上下文数据
    pub data: HashMap<String, serde_json::Value>,
    /// 消息历史
    pub message_history: Vec<AIMessage>,
}

impl AgentContext {
    pub fn new(session_id: String) -> Self {
        Self {
            user_id: None,
            session_id,
            data: HashMap::new(),
            message_history: Vec::new(),
        }
    }
    
    /// 添加数据到上下文
    pub fn add_data(&mut self, key: String, value: serde_json::Value) {
        self.data.insert(key, value);
    }
    
    /// 获取上下文数据
    pub fn get_data(&self, key: &str) -> Option<&serde_json::Value> {
        self.data.get(key)
    }
    
    /// 添加消息到历史
    pub fn add_message(&mut self, message: AIMessage) {
        self.message_history.push(message);
    }
    
    /// 清空消息历史
    pub fn clear_history(&mut self) {
        self.message_history.clear();
    }
    
    /// 获取最近的N条消息
    pub fn get_recent_messages(&self, count: usize) -> Vec<AIMessage> {
        let start = if self.message_history.len() > count {
            self.message_history.len() - count
        } else {
            0
        };
        self.message_history[start..].to_vec()
    }
}

/// Agent配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Agent名称
    pub name: String,
    /// Agent描述
    pub description: String,
    /// 使用的模型
    pub model: String,
    /// 温度参数
    pub temperature: f32,
    /// 最大token数
    pub max_tokens: u32,
    /// 系统提示词
    pub system_prompt: String,
    /// 是否启用记忆
    pub enable_memory: bool,
    /// 最大记忆条数
    pub max_memory_size: usize,
    /// 自定义参数
    pub custom_params: HashMap<String, serde_json::Value>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            name: "DefaultAgent".to_string(),
            description: "A basic AI agent".to_string(),
            model: "gpt-3.5-turbo".to_string(),
            temperature: 0.7,
            max_tokens: 1000,
            system_prompt: "You are a helpful AI assistant.".to_string(),
            enable_memory: true,
            max_memory_size: 10,
            custom_params: HashMap::new(),
        }
    }
}

/// Agent执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    /// 是否成功
    pub success: bool,
    /// 响应内容
    pub content: String,
    /// 元数据
    pub metadata: HashMap<String, serde_json::Value>,
    /// 错误信息（如果有）
    pub error: Option<String>,
    /// Token使用情况
    pub token_usage: Option<TokenUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl AgentResult {
    /// 创建成功结果
    pub fn success(content: String) -> Self {
        Self {
            success: true,
            content,
            metadata: HashMap::new(),
            error: None,
            token_usage: None,
        }
    }
    
    /// 创建失败结果
    pub fn error(error: String) -> Self {
        Self {
            success: false,
            content: String::new(),
            metadata: HashMap::new(),
            error: Some(error),
            token_usage: None,
        }
    }
    
    /// 添加元数据
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }
    
    /// 设置Token使用情况
    pub fn with_token_usage(mut self, usage: TokenUsage) -> Self {
        self.token_usage = Some(usage);
        self
    }
}

/// Agent接口特征
/// 所有AI Agent都必须实现这个接口
#[async_trait]
pub trait Agent: Send + Sync {
    /// 获取Agent名称
    fn name(&self) -> &str;
    
    /// 获取Agent描述
    fn description(&self) -> &str;
    
    /// 获取Agent配置
    fn config(&self) -> &AgentConfig;
    
    /// 更新Agent配置
    fn update_config(&mut self, config: AgentConfig);
    
    /// 处理用户输入
    /// 这是主要的执行方法，接收用户消息和上下文，返回处理结果
    async fn process(&self, input: &str, context: &mut AgentContext) -> Result<AgentResult>;
    
    /// 初始化Agent
    /// 在Agent开始工作前调用，用于加载必要的资源或配置
    async fn initialize(&mut self) -> Result<()> {
        Ok(())
    }
    
    /// 清理资源
    /// 在Agent停止工作后调用，用于清理资源
    async fn cleanup(&mut self) -> Result<()> {
        Ok(())
    }
    
    /// 验证输入
    /// 在处理前验证输入是否合法
    fn validate_input(&self, input: &str) -> Result<()> {
        if input.trim().is_empty() {
            return Err(anyhow::anyhow!("Input cannot be empty"));
        }
        Ok(())
    }
    
    /// 预处理输入
    /// 在发送给AI前对输入进行预处理
    async fn preprocess_input(&self, input: &str, context: &AgentContext) -> Result<String> {
        Ok(input.to_string())
    }
    
    /// 后处理输出
    /// 对AI响应进行后处理
    async fn postprocess_output(&self, output: &str, context: &AgentContext) -> Result<String> {
        Ok(output.to_string())
    }
    
    /// 构建提示词
    /// 根据输入和上下文构建完整的提示词
    async fn build_prompt(&self, input: &str, context: &AgentContext) -> Result<Vec<AIMessage>> {
        let mut messages = Vec::new();
        
        // 添加系统提示词
        if !self.config().system_prompt.is_empty() {
            messages.push(AIMessage {
                role: "system".to_string(),
                content: self.config().system_prompt.clone(),
            });
        }
        
        // 添加历史消息（如果启用记忆）
        if self.config().enable_memory {
            let history = context.get_recent_messages(self.config().max_memory_size);
            messages.extend(history);
        }
        
        // 添加当前用户输入
        messages.push(AIMessage {
            role: "user".to_string(),
            content: input.to_string(),
        });
        
        Ok(messages)
    }
    
    /// 调用AI模型
    /// 通用的AI调用方法
    async fn call_ai(&self, messages: Vec<AIMessage>, client: &AIHttpClient) -> Result<AIResponse> {
        let request = AIRequest {
            model: self.config().model.clone(),
            messages,
            temperature: self.config().temperature,
            max_tokens: self.config().max_tokens,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stream: None,
        };
        
        client.chat_completion(request).await
    }
}

/// 基础Agent实现
/// 提供了Agent接口的默认实现，其他Agent可以继承这个基础实现
pub struct BaseAgent {
    config: AgentConfig,
    client: AIHttpClient,
}

impl BaseAgent {
    /// 创建新的基础Agent
    pub fn new(config: AgentConfig, client: AIHttpClient) -> Self {
        Self { config, client }
    }
    
    /// 获取HTTP客户端
    pub fn client(&self) -> &AIHttpClient {
        &self.client
    }
    
    /// 更新HTTP客户端
    pub fn update_client(&mut self, client: AIHttpClient) {
        self.client = client;
    }
}

#[async_trait]
impl Agent for BaseAgent {
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
    
    // agent入口
    async fn process(&self, input: &str, context: &mut AgentContext) -> Result<AgentResult> {
        // 验证输入
        self.validate_input(input)?;
        
        // 预处理输入
        let processed_input = self.preprocess_input(input, context).await?;
        
        // 构建提示词
        let messages = self.build_prompt(&processed_input, context).await?;
        
        // 调用AI模型
        let response = self.call_ai(messages, &self.client).await?;
        
        // 提取响应内容
        let content = response.choices
            .first()
            .map(|choice| choice.message.content.clone())
            .unwrap_or_default();
        
        // 后处理输出
        let processed_output = self.postprocess_output(&content, context).await?;
        
        // 更新上下文
        context.add_message(AIMessage {
            role: "user".to_string(),
            content: input.to_string(),
        });
        context.add_message(AIMessage {
            role: "assistant".to_string(),
            content: processed_output.clone(),
        });
        
        // 创建结果
        let mut result = AgentResult::success(processed_output);
        
        // 添加Token使用情况
        if let Some(usage) = response.usage {
            result = result.with_token_usage(TokenUsage {
                prompt_tokens: usage.prompt_tokens,
                completion_tokens: usage.completion_tokens,
                total_tokens: usage.total_tokens,
            });
        }
        
        Ok(result)
    }
}

/// Agent工厂
/// 用于创建和管理不同类型的Agent
pub struct AgentFactory {
    agents: HashMap<String, Box<dyn Agent>>,
}

impl AgentFactory {
    /// 创建新的Agent工厂
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }
    
    /// 注册Agent
    pub fn register(&mut self, agent: Box<dyn Agent>) {
        let name = agent.name().to_string();
        self.agents.insert(name, agent);
    }
    
    /// 获取Agent
    pub fn get(&self, name: &str) -> Option<&dyn Agent> {
        self.agents.get(name).map(|agent| agent.as_ref())
    }
    
    /// 获取可变Agent（通过移除和重新插入来修改）
    pub fn take_agent(&mut self, name: &str) -> Option<Box<dyn Agent>> {
        self.agents.remove(name)
    }
    
    /// 重新插入Agent
    pub fn insert_agent(&mut self, agent: Box<dyn Agent>) {
        let name = agent.name().to_string();
        self.agents.insert(name, agent);
    }
    
    /// 列出所有Agent
    pub fn list_agents(&self) -> Vec<&str> {
        self.agents.keys().map(|k| k.as_str()).collect()
    }
    
    /// 移除Agent
    pub fn remove(&mut self, name: &str) -> Option<Box<dyn Agent>> {
        self.agents.remove(name)
    }
}

impl Default for AgentFactory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::http_client::{ClientConfig, AIProvider};
    
    #[test]
    fn test_agent_context() {
        let mut context = AgentContext::new("test-session".to_string());
        
        // 测试添加数据
        context.add_data("key1".to_string(), serde_json::json!("value1"));
        assert_eq!(context.get_data("key1"), Some(&serde_json::json!("value1")));
        
        // 测试添加消息
        context.add_message(AIMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
        });
        assert_eq!(context.message_history.len(), 1);
    }
    
    #[test]
    fn test_agent_result() {
        let result = AgentResult::success("Test response".to_string())
            .with_metadata("key".to_string(), serde_json::json!("value"));
        
        assert!(result.success);
        assert_eq!(result.content, "Test response");
        assert_eq!(result.metadata.get("key"), Some(&serde_json::json!("value")));
    }
    
    #[test]
    fn test_agent_factory() {
        let mut factory = AgentFactory::new();
        
        let config = AgentConfig::default();
        let client_config = ClientConfig::default();
        let client = AIHttpClient::new(client_config).unwrap();
        let agent = BaseAgent::new(config, client);
        
        factory.register(Box::new(agent));
        
        assert!(factory.get("DefaultAgent").is_some());
        assert_eq!(factory.list_agents(), vec!["DefaultAgent"]);
    }
}
