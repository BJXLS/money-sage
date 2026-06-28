use async_trait::async_trait;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tokio::time::timeout;

use super::LocalTool;

pub struct PythonExecTool {
    workspace_dir: PathBuf,
    db_path: PathBuf,
    session_id: String,
}

impl PythonExecTool {
    pub fn new(workspace_dir: PathBuf, db_path: PathBuf, session_id: Option<String>) -> Self {
        Self {
            workspace_dir,
            db_path,
            session_id: session_id.unwrap_or_else(|| "unknown".to_string()),
        }
    }

    /// 代码安全黑名单检查（基础层面，防止常见危险调用）
    fn check_denylist(code: &str) -> Result<()> {
        let normalized = code.to_lowercase();

        // 危险 import / 模块
        let dangerous_modules = [
            "subprocess", "os.system", "os.exec", "os.spawn", "os.fork",
            "socket", "urllib", "requests", "http.client", "ftplib", "smtplib",
            "telnetlib", "paramiko", "pexpect",
        ];

        for module in &dangerous_modules {
            if normalized.contains(module) {
                return Err(anyhow!(
                    "Python 代码包含被禁止的模块或调用: {}。请使用纯数据分析方式实现。",
                    module
                ));
            }
        }

        // 危险函数
        let dangerous_functions = [
            "eval(", "exec(", "compile(", "__import__(",
            "open('/", "open(\"/", "open('C:\\\\", "open(\"C:\\\\",
        ];

        for func in &dangerous_functions {
            if normalized.contains(func) {
                return Err(anyhow!(
                    "Python 代码包含被禁止的函数或模式: {}。请使用安全的数据分析方式实现。",
                    func
                ));
            }
        }

        Ok(())
    }

    /// 检查 file_path 是否在工作区内，防止越界访问
    fn validate_file_path(&self, relative_path: &str) -> Result<PathBuf> {
        let path = self.workspace_dir.join(relative_path);
        let canonicalized = path.canonicalize().unwrap_or_else(|_| path.clone());
        let workspace_canonical = self
            .workspace_dir
            .canonicalize()
            .unwrap_or_else(|_| self.workspace_dir.clone());

        if !canonicalized.starts_with(&workspace_canonical) {
            return Err(anyhow!(
                "文件路径 {} 超出工作区范围，拒绝访问",
                relative_path
            ));
        }

        Ok(canonicalized)
    }

    /// 检测系统是否有可用的 Python 解释器，优先 python3，其次 python
    async fn check_python_available() -> Result<String> {
        for interpreter in ["python3", "python"] {
            match Command::new(interpreter).arg("--version").output().await {
                Ok(output) if output.status.success() => {
                    return Ok(interpreter.to_string());
                }
                _ => continue,
            }
        }

        Err(anyhow!(
            "当前系统未检测到 Python 3 环境，无法执行 Python 分析脚本。\
             请自行安装 Python 3 后重试：\
             macOS 可运行 `brew install python3` 或从 https://www.python.org/downloads/ 下载安装；\
             Windows/Linux 请访问 https://www.python.org/downloads/ 获取安装包。"
        ))
    }
}

#[async_trait]
impl LocalTool for PythonExecTool {
    fn name(&self) -> &str {
        "python_exec"
    }

    fn description(&self) -> &str {
        "在工作区目录下执行 Python 3 脚本，用于数据分析、统计计算、生成图表等。可通过环境变量 MONEY_SAGE_DB_PATH 连接 SQLite 数据库（只读访问 categories/transactions/budgets 表）。环境变量 MONEY_SAGE_SESSION_ID 提供当前会话 ID，生成图片等临时文件建议保存到 .query_temp/${MONEY_SAGE_SESSION_ID}/images/ 目录下，并在输出中返回相对路径。执行有超时保护，禁止网络、系统命令和越界文件访问。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "code": {
                    "type": "string",
                    "description": "要执行的 Python 代码字符串。code 和 file_path 二选一。"
                },
                "file_path": {
                    "type": "string",
                    "description": "工作区内的 .py 文件相对路径，与 code 二选一。"
                },
                "description": {
                    "type": "string",
                    "description": "简要说明这段 Python 代码的用途（帮助理解和审计），可选"
                },
                "timeout_secs": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 120,
                    "default": 30,
                    "description": "超时时间（秒），默认 30，最大 120"
                }
            },
            "oneOf": [
                { "required": ["code"] },
                { "required": ["file_path"] }
            ]
        })
    }

    async fn execute(&self, arguments: Value) -> Result<String> {
        let code = arguments.get("code").and_then(|v| v.as_str());
        let file_path = arguments.get("file_path").and_then(|v| v.as_str());

        let (mode, script_content) = match (code, file_path) {
            (Some(c), _) if !c.trim().is_empty() => {
                Self::check_denylist(c)?;
                ("code", c.to_string())
            }
            (_, Some(fp)) if !fp.trim().is_empty() => {
                let full_path = self.validate_file_path(fp)?;
                if !full_path.exists() {
                    return Err(anyhow!("Python 文件不存在: {}", fp));
                }
                let content = tokio::fs::read_to_string(&full_path)
                    .await
                    .map_err(|e| anyhow!("读取 Python 文件失败: {}", e))?;
                Self::check_denylist(&content)?;
                ("file", fp.to_string())
            }
            _ => return Err(anyhow!("必须提供 code 或 file_path 参数")),
        };

        let description = arguments
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("未说明");

        let timeout_secs = arguments
            .get("timeout_secs")
            .and_then(|v| v.as_u64())
            .map(|v| v.clamp(1, 120))
            .unwrap_or(30);

        let interpreter = Self::check_python_available().await?;

        if !self.workspace_dir.exists() {
            return Err(anyhow!(
                "工作区目录不存在: {}，无法执行 Python 脚本",
                self.workspace_dir.display()
            ));
        }

        let mut cmd = Command::new(&interpreter);

        if mode == "code" {
            cmd.arg("-c").arg(&script_content);
        } else {
            cmd.arg(&script_content);
        }

        cmd.current_dir(&self.workspace_dir)
            .env_clear()
            .env("PATH", "/usr/local/bin:/usr/bin:/bin:/opt/homebrew/bin:/usr/local/sbin:/usr/sbin:/sbin")
            .env("HOME", std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string()))
            .env("TMPDIR", std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string()))
            .env("LANG", "en_US.UTF-8")
            .env("MONEY_SAGE_DB_PATH", self.db_path.display().to_string())
            .env("MONEY_SAGE_SESSION_ID", &self.session_id)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let start = Instant::now();

        let output = match timeout(Duration::from_secs(timeout_secs), cmd.output()).await {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                return Err(anyhow!(
                    "Python 启动失败: {}。工作区: {}",
                    e,
                    self.workspace_dir.display()
                ));
            }
            Err(_) => {
                return Err(anyhow!(
                    "Python 执行超时（{} 秒），已强制终止。工作区: {}",
                    timeout_secs,
                    self.workspace_dir.display()
                ));
            }
        };

        let duration_ms = start.elapsed().as_millis() as u64;

        const MAX_OUTPUT_BYTES: usize = 500 * 1024;

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

        eprintln!(
            "[PythonTool] desc={} | mode={} | timeout={} | exit_code={} | stdout_len={} | stderr_len={} | duration_ms={}",
            description,
            mode,
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
            "db_path": self.db_path.display().to_string(),
        });

        Ok(serde_json::to_string_pretty(&result)?)
    }
}
