use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use super::transport::StdioTransport;
use super::types::*;

/// MCP 客户端：管理与单个 MCP 服务器的连接
pub struct McpClient {
    transport: StdioTransport,
    request_id: AtomicU64,
    server_info: Option<InitializeResult>,
    tools: Vec<McpTool>,
}

impl McpClient {
    /// 连接到 MCP 服务器并完成初始化握手
    pub async fn connect(
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
    ) -> Result<Self> {
        let transport = StdioTransport::spawn(command, args, env).await?;

        let mut client = Self {
            transport,
            request_id: AtomicU64::new(1),
            server_info: None,
            tools: Vec::new(),
        };

        client.initialize().await?;
        Ok(client)
    }

    fn next_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }

    /// MCP 协议握手：initialize → initialized 通知
    async fn initialize(&mut self) -> Result<()> {
        let params = InitializeParams {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ClientCapabilities {
                roots: None,
                sampling: None,
            },
            client_info: ClientInfo {
                name: "MoneySage".to_string(),
                version: "0.1.0".to_string(),
            },
        };

        let request = JsonRpcRequest::new(
            self.next_id(),
            "initialize",
            Some(serde_json::to_value(&params)?),
        );

        let response = self.transport.send_request(&request).await?;

        if let Some(error) = &response.error {
            return Err(anyhow::anyhow!("MCP 初始化失败: {}", error));
        }

        let result: InitializeResult = serde_json::from_value(
            response.result.context("MCP 初始化响应缺少 result 字段")?,
        )?;

        self.server_info = Some(result);

        // 发送 initialized 通知
        let notification = JsonRpcRequest::notification("notifications/initialized", None);
        self.transport.send_notification(&notification).await?;

        // 自动获取工具列表
        self.refresh_tools().await?;

        Ok(())
    }

    /// 获取/刷新工具列表
    pub async fn refresh_tools(&mut self) -> Result<()> {
        let request = JsonRpcRequest::new(self.next_id(), "tools/list", None);
        let response = self.transport.send_request(&request).await?;

        if let Some(error) = &response.error {
            return Err(anyhow::anyhow!("获取工具列表失败: {}", error));
        }

        if let Some(result) = response.result {
            let list: ListToolsResult = serde_json::from_value(result)?;
            self.tools = list.tools;
        }

        Ok(())
    }

    /// 调用工具
    pub async fn call_tool(
        &self,
        name: &str,
        arguments: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<ToolCallResult> {
        let params = ToolCallParams {
            name: name.to_string(),
            arguments,
        };

        let request = JsonRpcRequest::new(
            self.next_id(),
            "tools/call",
            Some(serde_json::to_value(&params)?),
        );

        let response = self.transport.send_request(&request).await?;

        if let Some(error) = &response.error {
            return Err(anyhow::anyhow!("工具调用失败: {}", error));
        }

        let result: ToolCallResult = serde_json::from_value(
            response.result.context("工具调用响应缺少 result 字段")?,
        )?;

        Ok(result)
    }

    /// 获取缓存的工具列表
    pub fn tools(&self) -> &[McpTool] {
        &self.tools
    }

    /// 获取服务器信息
    pub fn server_info(&self) -> Option<&InitializeResult> {
        self.server_info.as_ref()
    }

    /// 检查连接是否存活
    pub async fn is_alive(&self) -> bool {
        self.transport.is_running().await
    }

    /// 关闭连接
    pub async fn shutdown(&self) -> Result<()> {
        self.transport.kill().await
    }
}
