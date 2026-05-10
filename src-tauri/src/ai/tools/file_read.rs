use async_trait::async_trait;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::path::PathBuf;

use super::{LocalTool, workspace_path};

pub struct FileReadTool {
    workspace_dir: PathBuf,
}

impl FileReadTool {
    pub fn new(workspace_dir: PathBuf) -> Self {
        Self { workspace_dir }
    }
}

#[async_trait]
impl LocalTool for FileReadTool {
    fn name(&self) -> &str {
        "file_read"
    }

    fn description(&self) -> &str {
        "读取工作区内的文件内容。只能访问工作区目录下的文件，file_path 必须是相对于工作区根目录的路径（如 'AGENTS.md' 或 'docs/plan.md'）。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "文件在工作区内的相对路径，如 'AGENTS.md'、'docs/plan.md'。必须是相对于工作区根目录的路径，禁止绝对路径和路径遍历。"
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

        let resolved = workspace_path::resolve_workspace_path(&self.workspace_dir, file_path)?;

        if !resolved.exists() {
            return Err(anyhow!(
                "文件不存在: {}。工作区路径: {}，请确认文件路径正确",
                file_path,
                self.workspace_dir.display()
            ));
        }

        if resolved.is_dir() {
            return Err(anyhow!(
                "该路径是目录，无法读取: {}。工作区路径: {}",
                file_path,
                self.workspace_dir.display()
            ));
        }

        // 二进制文件提示
        if !workspace_path::is_likely_text_file(&resolved) {
            return Err(anyhow!(
                "该文件看起来是二进制文件，不支持直接读取文本内容: {}。工作区路径: {}",
                file_path,
                self.workspace_dir.display()
            ));
        }

        let content = tokio::fs::read_to_string(&resolved).await
            .map_err(|e| anyhow!(
                "读取文件失败: {} (路径: {})。工作区路径: {}",
                e,
                file_path,
                self.workspace_dir.display()
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
