use async_trait::async_trait;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::path::PathBuf;

use super::{LocalTool, workspace_path};

pub struct FileWriteTool {
    workspace_dir: PathBuf,
}

impl FileWriteTool {
    pub fn new(workspace_dir: PathBuf) -> Self {
        Self { workspace_dir }
    }
}

#[async_trait]
impl LocalTool for FileWriteTool {
    fn name(&self) -> &str {
        "file_write"
    }

    fn description(&self) -> &str {
        "在工作区内创建新文件或完全覆盖已有文件。只能操作工作区目录下的文件，file_path 必须是相对于工作区根目录的路径（如 'notes/2024-01.md'）。会自动创建不存在的父目录。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "文件在工作区内的相对路径，如 'notes/2024-01.md'。必须是相对于工作区根目录的路径，禁止绝对路径和路径遍历。"
                },
                "content": {
                    "type": "string",
                    "description": "要写入的完整文件内容"
                }
            },
            "required": ["file_path", "content"]
        })
    }

    async fn execute(&self, arguments: Value) -> Result<String> {
        let file_path = arguments.get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("缺少 file_path 参数"))?;

        let content = arguments.get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("缺少 content 参数"))?;

        let resolved = workspace_path::resolve_workspace_path(&self.workspace_dir, file_path)?;

        // 创建父目录（如果不存在）
        if let Some(parent) = resolved.parent() {
            if !parent.exists() {
                tokio::fs::create_dir_all(parent).await
                    .map_err(|e| anyhow!(
                        "创建父目录失败: {} (路径: {})。工作区路径: {}",
                        e,
                        file_path,
                        self.workspace_dir.display()
                    ))?;
            }
        }

        let is_overwrite = resolved.exists();

        // 原子写入
        let tmp_path = resolved.with_extension(format!("tmp.{}", uuid::Uuid::new_v4()));
        tokio::fs::write(&tmp_path, content).await
            .map_err(|e| anyhow!(
                "写入临时文件失败: {} (路径: {})。工作区路径: {}",
                e,
                file_path,
                self.workspace_dir.display()
            ))?;

        tokio::fs::rename(&tmp_path, &resolved).await
            .map_err(|e| anyhow!(
                "替换文件失败: {} (路径: {})。工作区路径: {}",
                e,
                file_path,
                self.workspace_dir.display()
            ))?;

        let action = if is_overwrite { "覆盖" } else { "创建" };
        let byte_size = content.len();
        Ok(format!(
            "成功{}文件: {}（{} 字节）。工作区路径: {}",
            action,
            file_path,
            byte_size,
            self.workspace_dir.display()
        ))
    }
}
