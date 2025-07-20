use std::time::Duration;
use std::collections::HashMap;
use reqwest::{Client, Response, Error as ReqwestError};
use serde_json::{Value, json};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use anyhow::{Result, anyhow};

/// AI服务提供商类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AIProvider {
    OpenAI,
    Claude,
    Gemini,
    Local,
    Custom(String),
}

/// AI模型配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub top_p: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            model: "gpt-3.5-turbo".to_string(),
            temperature: 0.7,
            max_tokens: 1000,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
        }
    }
}

/// AI请求消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIMessage {
    pub role: String,
    pub content: String,
}

/// AI请求体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIRequest {
    pub model: String,
    pub messages: Vec<AIMessage>,
    pub temperature: f32,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

/// AI响应选择
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIChoice {
    pub index: u32,
    pub message: AIMessage,
    pub finish_reason: Option<String>,
}

/// AI响应体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<AIChoice>,
    pub usage: Option<AIUsage>,
}

/// Token使用情况
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// HTTP客户端配置
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub provider: AIProvider,
    pub base_url: String,
    pub api_key: String,
    pub timeout_secs: u64,
    pub max_retries: u32,
    pub headers: HashMap<String, String>,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            provider: AIProvider::OpenAI,
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: String::new(),
            timeout_secs: 30,
            max_retries: 3,
            headers: HashMap::new(),
        }
    }
}

/// API错误响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct APIError {
    pub error: ErrorDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetails {
    pub message: String,
    pub r#type: Option<String>,
    pub param: Option<String>,
    pub code: Option<String>,
}

/// AI HTTP客户端工具类
/// 支持多种AI服务商的API调用，包括重试机制、错误处理和流式响应
pub struct AIHttpClient {
    client: Client,
    config: ClientConfig,
}

impl AIHttpClient {
    /// 创建新的HTTP客户端实例
    pub fn new(config: ClientConfig) -> Result<Self> {
        let mut headers = reqwest::header::HeaderMap::new();
        
        // 设置默认headers
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        
        // 根据服务提供商设置认证header
        match &config.provider {
            AIProvider::OpenAI => {
                let auth_value = format!("Bearer {}", config.api_key);
                headers.insert(
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(&auth_value)
                        .map_err(|e| anyhow!("Invalid API key format: {}", e))?,
                );
            }
            AIProvider::Claude => {
                headers.insert(
                    "x-api-key",
                    reqwest::header::HeaderValue::from_str(&config.api_key)
                        .map_err(|e| anyhow!("Invalid API key format: {}", e))?,
                );
                headers.insert(
                    "anthropic-version",
                    reqwest::header::HeaderValue::from_static("2023-06-01"),
                );
            }
            AIProvider::Gemini => {
                // Gemini 使用 query parameter 方式传递 API key
            }
            AIProvider::Local | AIProvider::Custom(_) => {
                // 本地或自定义服务，可能不需要认证或使用自定义认证方式
                if !config.api_key.is_empty() {
                    let auth_value = format!("Bearer {}", config.api_key);
                    headers.insert(
                        reqwest::header::AUTHORIZATION,
                        reqwest::header::HeaderValue::from_str(&auth_value)
                            .map_err(|e| anyhow!("Invalid API key format: {}", e))?,
                    );
                }
            }
        }
        
        // 添加自定义headers
        for (key, value) in &config.headers {
            headers.insert(
                reqwest::header::HeaderName::from_bytes(key.as_bytes())
                    .map_err(|e| anyhow!("Invalid header name '{}': {}", key, e))?,
                reqwest::header::HeaderValue::from_str(value)
                    .map_err(|e| anyhow!("Invalid header value for '{}': {}", key, e))?,
            );
        }
        
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .default_headers(headers)
            .build()
            .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;
        
        Ok(Self { client, config })
    }
    
    /// 发送聊天完成请求
    pub async fn chat_completion(&self, request: AIRequest) -> Result<AIResponse> {
        let url = self.build_url("/chat/completions")?;
        let body = self.build_request_body(request)?;
        
        self.send_request_with_retry(&url, &body).await
    }
    
    /// 发送聊天完成请求（流式）
    pub async fn chat_completion_stream(&self, mut request: AIRequest) -> Result<reqwest::Response> {
        request.stream = Some(true);
        let url = self.build_url("/chat/completions")?;
        let body = self.build_request_body(request)?;
        
        self.send_stream_request(&url, &body).await
    }
    
    /// 测试连接
    pub async fn test_connection(&self) -> Result<bool> {
        match &self.config.provider {
            AIProvider::OpenAI => {
                let url = self.build_url("/models")?;
                let response = self.client.get(&url).send().await?;
                Ok(response.status().is_success())
            }
            AIProvider::Claude => {
                // Claude没有专门的测试接口，发送一个简单的请求
                let test_request = AIRequest {
                    model: "claude-3-haiku-20240307".to_string(),
                    messages: vec![AIMessage {
                        role: "user".to_string(),
                        content: "Hello".to_string(),
                    }],
                    temperature: 0.1,
                    max_tokens: 10,
                    top_p: None,
                    frequency_penalty: None,
                    presence_penalty: None,
                    stream: None,
                };
                
                match self.chat_completion(test_request).await {
                    Ok(_) => Ok(true),
                    Err(_) => Ok(false),
                }
            }
            _ => {
                // 对于其他服务商，尝试发送一个简单请求
                Ok(true)
            }
        }
    }
    
    /// 构建完整的API URL
    fn build_url(&self, endpoint: &str) -> Result<String> {
        let base_url = self.config.base_url.trim_end_matches('/');
        
        match &self.config.provider {
            AIProvider::Gemini => {
                // Gemini 的 URL 格式不同，需要包含 API key
                Ok(format!("{}{}?key={}", base_url, endpoint, self.config.api_key))
            }
            _ => {
                Ok(format!("{}{}", base_url, endpoint))
            }
        }
    }
    
    /// 构建请求体
    fn build_request_body(&self, request: AIRequest) -> Result<Value> {
        match &self.config.provider {
            AIProvider::OpenAI | AIProvider::Local | AIProvider::Custom(_) => {
                Ok(serde_json::to_value(request)?)
            }
            AIProvider::Claude => {
                // Claude API 格式转换
                let system_message = request.messages.iter()
                    .find(|msg| msg.role == "system")
                    .map(|msg| msg.content.clone());
                
                let user_messages: Vec<_> = request.messages.iter()
                    .filter(|msg| msg.role != "system")
                    .collect();
                
                let mut body = json!({
                    "model": request.model,
                    "max_tokens": request.max_tokens,
                    "temperature": request.temperature,
                    "messages": user_messages
                });
                
                if let Some(system) = system_message {
                    body["system"] = json!(system);
                }
                
                if let Some(top_p) = request.top_p {
                    body["top_p"] = json!(top_p);
                }
                
                Ok(body)
            }
            AIProvider::Gemini => {
                // Gemini API 格式转换
                let contents: Vec<_> = request.messages.iter()
                    .filter(|msg| msg.role == "user")
                    .map(|msg| json!({
                        "parts": [{"text": msg.content}]
                    }))
                    .collect();
                
                Ok(json!({
                    "contents": contents,
                    "generationConfig": {
                        "temperature": request.temperature,
                        "maxOutputTokens": request.max_tokens,
                        "topP": request.top_p
                    }
                }))
            }
        }
    }
    
    /// 带重试机制的请求发送
    async fn send_request_with_retry(&self, url: &str, body: &Value) -> Result<AIResponse> {
        let mut last_error = None;
        
        for attempt in 0..=self.config.max_retries {
            match self.send_request(url, body).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    last_error = Some(e);
                    
                    if attempt < self.config.max_retries {
                        let delay = Duration::from_millis(1000 * (2_u64.pow(attempt as u32)));
                        println!("Request failed, retrying in {:?}...", delay);
                        sleep(delay).await;
                    }
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| anyhow!("All retry attempts failed")))
    }
    
    /// 发送单次请求
    async fn send_request(&self, url: &str, body: &Value) -> Result<AIResponse> {
        let response = self.client
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(|e| anyhow!("Request failed: {}", e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            
            // 尝试解析结构化错误
            if let Ok(api_error) = serde_json::from_str::<APIError>(&error_text) {
                return Err(anyhow!("API Error: {}", api_error.error.message));
            }
            
            return Err(anyhow!("HTTP {}: {}", status, error_text));
        }
        
        let response_text = response.text().await
            .map_err(|e| anyhow!("Failed to read response: {}", e))?;
        
        // 根据不同服务商解析响应
        match &self.config.provider {
            AIProvider::OpenAI | AIProvider::Local | AIProvider::Custom(_) => {
                serde_json::from_str(&response_text)
                    .map_err(|e| anyhow!("Failed to parse OpenAI response: {}", e))
            }
            AIProvider::Claude => {
                self.parse_claude_response(&response_text)
            }
            AIProvider::Gemini => {
                self.parse_gemini_response(&response_text)
            }
        }
    }
    
    /// 发送流式请求
    async fn send_stream_request(&self, url: &str, body: &Value) -> Result<Response> {
        let response = self.client
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(|e| anyhow!("Stream request failed: {}", e))?;
        
        if !response.status().is_success() {
            let status = response.status();
            return Err(anyhow!("Stream request failed with HTTP {}", status));
        }
        
        Ok(response)
    }
    
    /// 解析 Claude 响应格式
    fn parse_claude_response(&self, response_text: &str) -> Result<AIResponse> {
        let claude_response: Value = serde_json::from_str(response_text)?;
        
        // 转换为标准格式
        let content = claude_response["content"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();
        
        Ok(AIResponse {
            id: claude_response["id"].as_str().unwrap_or("").to_string(),
            object: "chat.completion".to_string(),
            created: 0,
            model: claude_response["model"].as_str().unwrap_or("").to_string(),
            choices: vec![AIChoice {
                index: 0,
                message: AIMessage {
                    role: "assistant".to_string(),
                    content,
                },
                finish_reason: claude_response["stop_reason"].as_str().map(|s| s.to_string()),
            }],
            usage: claude_response["usage"].as_object().map(|usage| AIUsage {
                prompt_tokens: usage["input_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: usage["output_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: (usage["input_tokens"].as_u64().unwrap_or(0) + 
                             usage["output_tokens"].as_u64().unwrap_or(0)) as u32,
            }),
        })
    }
    
    /// 解析 Gemini 响应格式
    fn parse_gemini_response(&self, response_text: &str) -> Result<AIResponse> {
        let gemini_response: Value = serde_json::from_str(response_text)?;
        
        // 转换为标准格式
        let content = gemini_response["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();
        
        Ok(AIResponse {
            id: "gemini-response".to_string(),
            object: "chat.completion".to_string(),
            created: 0,
            model: "gemini".to_string(),
            choices: vec![AIChoice {
                index: 0,
                message: AIMessage {
                    role: "assistant".to_string(),
                    content,
                },
                finish_reason: gemini_response["candidates"][0]["finishReason"]
                    .as_str().map(|s| s.to_string()),
            }],
            usage: gemini_response["usageMetadata"].as_object().map(|usage| AIUsage {
                prompt_tokens: usage["promptTokenCount"].as_u64().unwrap_or(0) as u32,
                completion_tokens: usage["candidatesTokenCount"].as_u64().unwrap_or(0) as u32,
                total_tokens: usage["totalTokenCount"].as_u64().unwrap_or(0) as u32,
            }),
        })
    }
    
    /// 获取客户端配置
    pub fn config(&self) -> &ClientConfig {
        &self.config
    }
    
    /// 更新API密钥
    pub fn update_api_key(&mut self, new_api_key: String) -> Result<()> {
        self.config.api_key = new_api_key;
        // 重新创建客户端以更新headers
        let new_client = Self::new(self.config.clone())?;
        self.client = new_client.client;
        Ok(())
    }
}

/// 便捷方法：创建不同服务商的客户端
impl AIHttpClient {
    /// 创建 OpenAI 客户端
    pub fn openai(api_key: String) -> Result<Self> {
        let config = ClientConfig {
            provider: AIProvider::OpenAI,
            base_url: "https://api.openai.com/v1".to_string(),
            api_key,
            ..Default::default()
        };
        Self::new(config)
    }
    
    /// 创建 Claude 客户端
    pub fn claude(api_key: String) -> Result<Self> {
        let config = ClientConfig {
            provider: AIProvider::Claude,
            base_url: "https://api.anthropic.com/v1".to_string(),
            api_key,
            ..Default::default()
        };
        Self::new(config)
    }
    
    /// 创建 Gemini 客户端
    pub fn gemini(api_key: String) -> Result<Self> {
        let config = ClientConfig {
            provider: AIProvider::Gemini,
            base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
            api_key,
            ..Default::default()
        };
        Self::new(config)
    }
    
    /// 创建本地服务客户端
    pub fn local(base_url: String, api_key: Option<String>) -> Result<Self> {
        let config = ClientConfig {
            provider: AIProvider::Local,
            base_url,
            api_key: api_key.unwrap_or_default(),
            ..Default::default()
        };
        Self::new(config)
    }
    
    /// 创建自定义服务客户端
    pub fn custom(provider_name: String, base_url: String, api_key: String) -> Result<Self> {
        let config = ClientConfig {
            provider: AIProvider::Custom(provider_name),
            base_url,
            api_key,
            ..Default::default()
        };
        Self::new(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_client_creation() {
        let config = ClientConfig {
            provider: AIProvider::OpenAI,
            api_key: "test-key".to_string(),
            ..Default::default()
        };
        
        let client = AIHttpClient::new(config);
        assert!(client.is_ok());
    }
    
    #[test]
    fn test_url_building() {
        let config = ClientConfig {
            provider: AIProvider::OpenAI,
            base_url: "https://api.openai.com/v1".to_string(),
            ..Default::default()
        };
        
        let client = AIHttpClient::new(config).unwrap();
        let url = client.build_url("/chat/completions").unwrap();
        assert_eq!(url, "https://api.openai.com/v1/chat/completions");
    }
}
