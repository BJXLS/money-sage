use anyhow::{Context, Result};
use std::collections::HashMap;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use super::types::{JsonRpcRequest, JsonRpcResponse};

/// Stdio 传输层：启动子进程并通过 stdin/stdout 通信
pub struct StdioTransport {
    child: Mutex<Child>,
    stdin: Mutex<tokio::process::ChildStdin>,
    stdout: Mutex<BufReader<tokio::process::ChildStdout>>,
}

impl StdioTransport {
    /// 启动 MCP 服务器子进程
    pub async fn spawn(
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
    ) -> Result<Self> {
        let mut cmd = Command::new(command);
        cmd.args(args)
            .envs(env)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // Windows: 避免弹出控制台窗口
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }

        let mut child = cmd.spawn().context(format!("无法启动 MCP 服务器: {}", command))?;

        let stdin = child
            .stdin
            .take()
            .context("无法获取子进程 stdin")?;
        let stdout = child
            .stdout
            .take()
            .context("无法获取子进程 stdout")?;

        Ok(Self {
            child: Mutex::new(child),
            stdin: Mutex::new(stdin),
            stdout: Mutex::new(BufReader::new(stdout)),
        })
    }

    /// 发送 JSON-RPC 请求并等待响应
    pub async fn send_request(&self, request: &JsonRpcRequest) -> Result<JsonRpcResponse> {
        let json = serde_json::to_string(request)?;
        let message = format!("{}\n", json);

        {
            let mut stdin = self.stdin.lock().await;
            stdin.write_all(message.as_bytes()).await?;
            stdin.flush().await?;
        }

        // 对于通知（无 id），不需要读取响应
        if request.id.is_none() {
            return Ok(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: None,
                result: None,
                error: None,
            });
        }

        self.read_response().await
    }

    /// 发送通知（无需响应）
    pub async fn send_notification(&self, request: &JsonRpcRequest) -> Result<()> {
        let json = serde_json::to_string(request)?;
        let message = format!("{}\n", json);

        let mut stdin = self.stdin.lock().await;
        stdin.write_all(message.as_bytes()).await?;
        stdin.flush().await?;
        Ok(())
    }

    /// 读取一行 JSON-RPC 响应
    async fn read_response(&self) -> Result<JsonRpcResponse> {
        let mut stdout = self.stdout.lock().await;
        let mut line = String::new();

        loop {
            line.clear();
            let bytes_read = tokio::time::timeout(
                std::time::Duration::from_secs(30),
                stdout.read_line(&mut line),
            )
            .await
            .context("读取 MCP 响应超时 (30s)")??;

            if bytes_read == 0 {
                return Err(anyhow::anyhow!("MCP 服务器已关闭连接"));
            }

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            match serde_json::from_str::<JsonRpcResponse>(trimmed) {
                Ok(response) => return Ok(response),
                Err(_) => {
                    // 可能是服务器端的通知或日志行，跳过
                    continue;
                }
            }
        }
    }

    /// 检查子进程是否仍在运行
    pub async fn is_running(&self) -> bool {
        let mut child = self.child.lock().await;
        match child.try_wait() {
            Ok(None) => true,
            _ => false,
        }
    }

    /// 终止子进程
    pub async fn kill(&self) -> Result<()> {
        let mut child = self.child.lock().await;
        child.kill().await.ok();
        child.wait().await.ok();
        Ok(())
    }
}
