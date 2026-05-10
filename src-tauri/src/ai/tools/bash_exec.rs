use async_trait::async_trait;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tokio::time::timeout;

use super::LocalTool;

pub struct BashExecTool {
    workspace_dir: PathBuf,
}

impl BashExecTool {
    pub fn new(workspace_dir: PathBuf) -> Self {
        Self { workspace_dir }
    }

    /// 第一层安全：命令字符串黑名单检查
    fn check_denylist(command: &str) -> Result<()> {
        let normalized = command.to_lowercase();

        // 精确匹配的危险模式
        let exact_patterns = [
            "rm -rf /",
            "rm -rf /*",
            "rm -rf ~",
            "rm -rf ~/",
            ":(){ :|:& };:",
            ": () { : | :& } ; :",
            "> /dev/sda",
            "> /dev/sdb",
            "> /dev/hda",
            "> /dev/hdb",
            "> /dev/nvme",
            "mkfs.ext",
            "mkfs.xfs",
            "mkfs.vfat",
            "mkfs.btrfs",
            "mkfs.ntfs",
            "fdisk /dev/sd",
            "dd if=/dev/zero of=/dev/sd",
            "dd if=/dev/random of=/dev/sd",
            "dd if=/dev/urandom of=/dev/sd",
            "chmod 777 /",
            "chmod -r 777 /",
            "chown root:root /",
        ];

        for pattern in &exact_patterns {
            if normalized.contains(pattern) {
                return Err(anyhow!(
                    "命令包含危险模式，已拒绝执行: '{}'。请使用安全的命令替代。",
                    pattern
                ));
            }
        }

        // 模糊匹配的危险模式（基于字符串包含检查）
        let fuzzy_patterns: [(&str, &str); 6] = [
            ("curl", "|sh"),
            ("curl", "|bash"),
            ("wget", "|sh"),
            ("wget", "|bash"),
            ("curl", "| zsh"),
            ("wget", "| zsh"),
        ];

        for (left, right) in &fuzzy_patterns {
            if normalized.contains(left) && normalized.contains(right) {
                return Err(anyhow!(
                    "命令包含危险模式，已拒绝执行: {} ... | sh 管道执行存在安全风险。请使用安全的命令替代。",
                    left
                ));
            }
        }

        // sudo / su 检查
        if normalized.contains("sudo ") || normalized.starts_with("sudo") {
            return Err(anyhow!(
                "命令包含危险模式，已拒绝执行: sudo 提权命令被禁止。请使用安全的命令替代。"
            ));
        }
        if normalized.contains("su -") || normalized.starts_with("su -") {
            return Err(anyhow!(
                "命令包含危险模式，已拒绝执行: su 切换用户命令被禁止。请使用安全的命令替代。"
            ));
        }

        Ok(())
    }
}

#[async_trait]
impl LocalTool for BashExecTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "在工作区目录下执行 shell 命令（bash -c）。支持 git、npm、cargo、grep、find 等常规命令。命令有超时保护（默认 30 秒，最长 120 秒）和黑名单过滤，极端危险命令会被拒绝。环境变量已清理，不包含敏感信息。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "要执行的 bash 命令字符串。注意：只能在工作区内操作，禁止访问系统敏感路径。"
                },
                "description": {
                    "type": "string",
                    "description": "简要说明这条命令的用途（帮助理解和审计），可选"
                },
                "timeout_secs": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 120,
                    "default": 30,
                    "description": "超时时间（秒），默认 30，最大 120"
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, arguments: Value) -> Result<String> {
        let command = arguments.get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("缺少 command 参数"))?;

        let description = arguments.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("未说明");

        let timeout_secs = arguments.get("timeout_secs")
            .and_then(|v| v.as_u64())
            .map(|v| v.clamp(1, 120))
            .unwrap_or(30);

        if command.trim().is_empty() {
            return Err(anyhow!("command 不能为空"));
        }

        // 第一层：黑名单检查
        Self::check_denylist(command)?;

        // 检查工作区目录存在
        if !self.workspace_dir.exists() {
            return Err(anyhow!(
                "工作区目录不存在: {}。无法执行 bash 命令。",
                self.workspace_dir.display()
            ));
        }

        // 构建隔离的 Command
        let mut cmd = Command::new("bash");
        cmd.arg("-c")
            .arg(command)
            .current_dir(&self.workspace_dir)
            .env_clear()
            .env("PATH", "/usr/local/bin:/usr/bin:/bin:/opt/homebrew/bin:/usr/local/sbin:/usr/sbin:/sbin")
            .env("HOME", std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()))
            .env("TMPDIR", std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string()))
            .env("LANG", "en_US.UTF-8")
            .env("SHELL", "/bin/bash")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let start = Instant::now();

        // 第三层：超时执行
        let output = match timeout(Duration::from_secs(timeout_secs), cmd.output()).await {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                return Err(anyhow!(
                    "命令启动失败: {}。command: {}，工作区: {}",
                    e,
                    command,
                    self.workspace_dir.display()
                ));
            }
            Err(_) => {
                return Err(anyhow!(
                    "命令执行超时（{} 秒），已强制终止。command: {}，工作区: {}",
                    timeout_secs,
                    command,
                    self.workspace_dir.display()
                ));
            }
        };

        let duration_ms = start.elapsed().as_millis() as u64;

        // 第四层：输出截断
        const MAX_OUTPUT_BYTES: usize = 500 * 1024; // 500KB

        let stdout_raw = String::from_utf8_lossy(&output.stdout);
        let stderr_raw = String::from_utf8_lossy(&output.stderr);

        let stdout_truncated = stdout_raw.len() > MAX_OUTPUT_BYTES;
        let stderr_truncated = stderr_raw.len() > MAX_OUTPUT_BYTES;

        let stdout = if stdout_truncated {
            format!(
                "{}\n...[stdout 已截断，共 {} 字节]",
                &stdout_raw[..MAX_OUTPUT_BYTES],
                stdout_raw.len()
            )
        } else {
            stdout_raw.to_string()
        };

        let stderr = if stderr_truncated {
            format!(
                "{}\n...[stderr 已截断，共 {} 字节]",
                &stderr_raw[..MAX_OUTPUT_BYTES],
                stderr_raw.len()
            )
        } else {
            stderr_raw.to_string()
        };

        // 审计日志
        eprintln!(
            "[BashTool] desc={} | cmd={} | timeout={} | exit_code={} | stdout_len={} | stderr_len={} | duration_ms={}",
            description,
            command,
            timeout_secs,
            output.status.code().unwrap_or(-1),
            stdout_raw.len(),
            stderr_raw.len(),
            duration_ms
        );

        let result = json!({
            "success": output.status.success(),
            "exit_code": output.status.code(),
            "stdout": stdout,
            "stderr": stderr,
            "stdout_truncated": stdout_truncated,
            "stderr_truncated": stderr_truncated,
            "execution_time_ms": duration_ms,
            "workspace_dir": self.workspace_dir.display().to_string(),
        });

        Ok(serde_json::to_string_pretty(&result)?)
    }
}
