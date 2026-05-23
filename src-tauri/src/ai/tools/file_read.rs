use async_trait::async_trait;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::path::PathBuf;

use super::{LocalTool, workspace_path};

pub struct FileReadTool {
    workspace_dir: PathBuf,
    memory_dir: Option<PathBuf>,
}

impl FileReadTool {
    pub fn new(workspace_dir: PathBuf, memory_dir: Option<PathBuf>) -> Self {
        Self { workspace_dir, memory_dir }
    }

    fn resolve_path(&self, base: &str, file_path: &str) -> Result<PathBuf> {
        match base {
            "memory" => {
                match &self.memory_dir {
                    Some(dir) => {
                        // 复用 workspace_path 的安全检查逻辑，但针对 memory 目录
                        let trimmed = file_path.trim();
                        if trimmed.is_empty() {
                            return Err(anyhow!("file_path 不能为空"));
                        }
                        if trimmed.contains("..") {
                            return Err(anyhow!("路径中不允许包含 '..'"));
                        }
                        let p = std::path::Path::new(trimmed);
                        if p.is_absolute() {
                            return Err(anyhow!("file_path 必须是相对路径"));
                        }
                        let resolved = dir.join(trimmed);
                        Ok(resolved)
                    }
                    None => Err(anyhow!("memory 目录未配置")),
                }
            }
            _ => {
                workspace_path::resolve_workspace_path(&self.workspace_dir, file_path)
                    .map_err(|e| anyhow!("路径解析失败: {}", e))
            }
        }
    }
}

#[async_trait]
impl LocalTool for FileReadTool {
    fn name(&self) -> &str {
        "file_read"
    }

    fn description(&self) -> &str {
        "读取工作区或记忆目录内的文件内容。file_path 必须是相对路径。可以通过 base 参数指定读取 workspace（默认）或 memory 目录下的文件。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "文件的相对路径。base=workspace 时如 'AGENTS.md'、'docs/plan.md'；base=memory 时如 'factual/user-profile.md'、'MEMORY.md'。"
                },
                "base": {
                    "type": "string",
                    "enum": ["workspace", "memory"],
                    "default": "workspace",
                    "description": "目录基址。workspace 表示工作区目录，memory 表示记忆目录。"
                },
                "offset": {
                    "type": "integer",
                    "description": "起始行号（1-indexed），可选，默认从第1行开始"
                },
                "limit": {
                    "type": "integer",
                    "description": "最多读取多少行，可选，默认读取整个文件"
                }
            },
            "required": ["file_path"]
        })
    }

    async fn execute(&self, arguments: Value) -> Result<String> {
        let file_path = arguments.get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("缺少 file_path 参数"))?;

        let base = arguments.get("base")
            .and_then(|v| v.as_str())
            .unwrap_or("workspace");

        let resolved = self.resolve_path(base, file_path)?;

        if !resolved.exists() {
            return Err(anyhow!(
                "文件不存在: {}。base={}, 请确认文件路径正确",
                file_path, base
            ));
        }

        if resolved.is_dir() {
            return Err(anyhow!(
                "该路径是目录，无法读取: {}。base={}",
                file_path, base
            ));
        }

        // 二进制文件提示
        if !workspace_path::is_likely_text_file(&resolved) {
            return Err(anyhow!(
                "该文件看起来是二进制文件，不支持直接读取文本内容: {}。base={}",
                file_path, base
            ));
        }

        let content = tokio::fs::read_to_string(&resolved).await
            .map_err(|e| anyhow!(
                "读取文件失败: {} (路径: {}, base: {})",
                e, file_path, base
            ))?;

        let offset = arguments.get("offset")
            .and_then(|v| v.as_u64())
            .unwrap_or(1)
            .saturating_sub(1) as usize;
        let limit = arguments.get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(u64::MAX) as usize;

        let lines: Vec<&str> = content.lines().collect();
        let end = (offset + limit).min(lines.len());
        let slice = if offset < lines.len() {
            &lines[offset..end]
        } else {
            &[] as &[&str]
        };

        let mut result = String::new();
        for (i, line) in slice.iter().enumerate() {
            result.push_str(&format!("{:6}\t{}\n", offset + i + 1, line));
        }

        if result.is_empty() && !content.is_empty() {
            result.push_str("(指定范围无内容，文件共 ");
            result.push_str(&lines.len().to_string());
            result.push_str(" 行)\n");
        }

        Ok(result)
    }
}
