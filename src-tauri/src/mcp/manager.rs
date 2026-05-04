use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::client::McpClient;
use super::types::*;

/// 活跃的 MCP 服务器连接
struct ActiveServer {
    config: McpServerConfig,
    client: McpClient,
}

/// MCP 服务器管理器：管理多个 MCP 服务器的生命周期
pub struct McpManager {
    servers: Arc<RwLock<HashMap<i64, ActiveServer>>>,
}

impl Clone for McpManager {
    fn clone(&self) -> Self {
        Self {
            servers: self.servers.clone(),
        }
    }
}

impl McpManager {
    pub fn new() -> Self {
        Self {
            servers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 启动一个 MCP 服务器
    pub async fn start_server(&self, config: &McpServerConfig) -> Result<()> {
        // 如果已经连接，先停止
        self.stop_server(config.id).await.ok();

        let args: Vec<String> = if config.args.is_empty() {
            Vec::new()
        } else {
            serde_json::from_str(&config.args).unwrap_or_else(|_| {
                config.args.split_whitespace().map(|s| s.to_string()).collect()
            })
        };

        let env: HashMap<String, String> = if config.env.is_empty() {
            HashMap::new()
        } else {
            serde_json::from_str(&config.env).unwrap_or_default()
        };

        println!(
            "[MCP] 正在启动服务器 '{}': {} {:?}",
            config.name, config.command, args
        );

        let client = McpClient::connect(&config.command, &args, &env).await?;

        let tool_count = client.tools().len();
        println!(
            "[MCP] 服务器 '{}' 已连接，发现 {} 个工具",
            config.name, tool_count
        );

        let mut servers = self.servers.write().await;
        servers.insert(
            config.id,
            ActiveServer {
                config: config.clone(),
                client,
            },
        );

        Ok(())
    }

    /// 停止一个 MCP 服务器
    pub async fn stop_server(&self, server_id: i64) -> Result<()> {
        let mut servers = self.servers.write().await;
        if let Some(active) = servers.remove(&server_id) {
            println!("[MCP] 正在停止服务器 '{}'", active.config.name);
            active.client.shutdown().await?;
        }
        Ok(())
    }

    /// 停止所有服务器
    pub async fn stop_all(&self) -> Result<()> {
        let mut servers = self.servers.write().await;
        for (_, active) in servers.drain() {
            active.client.shutdown().await.ok();
        }
        Ok(())
    }

    /// 获取所有已连接服务器的状态
    pub async fn get_all_status(&self, configs: &[McpServerConfig]) -> Vec<McpServerStatus> {
        let servers = self.servers.read().await;
        let mut result = Vec::new();

        for config in configs {
            let (connected, tools, error) = if let Some(active) = servers.get(&config.id) {
                let alive = active.client.is_alive().await;
                if alive {
                    (true, active.client.tools().to_vec(), None)
                } else {
                    (false, vec![], Some("服务器进程已退出".to_string()))
                }
            } else {
                (false, vec![], None)
            };

            result.push(McpServerStatus {
                id: config.id,
                name: config.name.clone(),
                command: config.command.clone(),
                enabled: config.enabled,
                connected,
                tools,
                error,
            });
        }

        result
    }

    /// 获取某个已连接服务器的工具列表
    pub async fn get_tools(&self, server_id: i64) -> Result<Vec<McpTool>> {
        let servers = self.servers.read().await;
        match servers.get(&server_id) {
            Some(active) => Ok(active.client.tools().to_vec()),
            None => Err(anyhow::anyhow!("服务器未连接")),
        }
    }

    /// 获取所有已连接服务器的工具（合并）
    pub async fn get_all_tools(&self) -> Vec<(String, McpTool)> {
        let servers = self.servers.read().await;
        let mut all_tools = Vec::new();

        for active in servers.values() {
            for tool in active.client.tools() {
                all_tools.push((active.config.name.clone(), tool.clone()));
            }
        }

        all_tools
    }

    /// 调用某个服务器上的工具
    pub async fn call_tool(
        &self,
        server_id: i64,
        tool_name: &str,
        arguments: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<ToolCallResult> {
        let servers = self.servers.read().await;
        match servers.get(&server_id) {
            Some(active) => active.client.call_tool(tool_name, arguments).await,
            None => Err(anyhow::anyhow!("服务器未连接")),
        }
    }

    /// 按服务器名称查找并调用工具（Agent 集成用）
    pub async fn call_tool_by_name(
        &self,
        server_name: &str,
        tool_name: &str,
        arguments: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<ToolCallResult> {
        let servers = self.servers.read().await;
        for active in servers.values() {
            if active.config.name == server_name {
                return active.client.call_tool(tool_name, arguments).await;
            }
        }
        Err(anyhow::anyhow!(
            "未找到名为 '{}' 的已连接服务器",
            server_name
        ))
    }

    /// 将所有已连接的工具转换为 Agent 可用的工具描述（OpenAI function calling 格式）
    pub async fn tools_as_functions(&self) -> Vec<serde_json::Value> {
        let servers = self.servers.read().await;
        let mut functions = Vec::new();

        for active in servers.values() {
            for tool in active.client.tools() {
                functions.push(serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": format!("{}_{}", active.config.name, tool.name),
                        "description": tool.description.clone().unwrap_or_default(),
                        "parameters": tool.input_schema,
                    }
                }));
            }
        }

        functions
    }

    /// 检查某个服务器是否已连接
    pub async fn is_connected(&self, server_id: i64) -> bool {
        let servers = self.servers.read().await;
        if let Some(active) = servers.get(&server_id) {
            active.client.is_alive().await
        } else {
            false
        }
    }
}

impl Default for McpManager {
    fn default() -> Self {
        Self::new()
    }
}
