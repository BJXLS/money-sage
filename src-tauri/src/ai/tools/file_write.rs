use async_trait::async_trait;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::path::PathBuf;

use super::{LocalTool, workspace_path};

pub struct FileWriteTool {
    workspace_dir: PathBuf,
    memory_dir: Option<PathBuf>,
}

impl FileWriteTool {
    pub fn new(workspace_dir: PathBuf, memory_dir: Option<PathBuf>) -> Self {
        Self { workspace_dir, memory_dir }
    }

    fn resolve_path(&self, base: &str, file_path: &str) -> Result<PathBuf> {
        match base {
            "memory" => {
                match &self.memory_dir {
                    Some(dir) => {
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
                        Ok(dir.join(trimmed))
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
impl LocalTool for FileWriteTool {
    fn name(&self) -> &str {
        "file_write"
    }

    fn description(&self) -> &str {
        "在工作区或记忆目录内创建新文件或完全覆盖已有文件。base=memory 时可以在 factual/、episodic/、procedural/ 下创建或修改 .md 文件。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "文件的相对路径。base=workspace 时如 'notes/2024-01.md'；base=memory 时如 'factual/user-profile.md'、'episodic/2026/05/2026-05-23.md'。"
                },
                "base": {
                    "type": "string",
                    "enum": ["workspace", "memory"],
                    "default": "workspace",
                    "description": "目录基址。workspace 表示工作区目录，memory 表示记忆目录。"
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

        let base = arguments.get("base")
            .and_then(|v| v.as_str())
            .unwrap_or("workspace");

        let content = arguments.get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("缺少 content 参数"))?;

        let resolved = self.resolve_path(base, file_path)?;

        // 创建父目录（如果不存在）
        if let Some(parent) = resolved.parent() {
            if !parent.exists() {
                tokio::fs::create_dir_all(parent).await
                    .map_err(|e| anyhow!(
                        "创建父目录失败: {} (路径: {}, base: {})",
                        e, file_path, base
                    ))?;
            }
        }

        let is_overwrite = resolved.exists();

        // 安全检查：如果是 memory/ 目录的写入，扫描注入
        if base == "memory" {
            // meta/ 目录不可写入
            if file_path.starts_with("meta/") {
                return Err(anyhow!(
                    "禁止修改 meta/ 目录下的系统规范文件。文件: {}",
                    file_path
                ));
            }

            if let Err(e) = crate::memory::v3::safety::scan_memory_write(content) {
                return Err(anyhow!(
                    "记忆写入安全检查未通过: {}。文件: {}，base: {}",
                    e, file_path, base
                ));
            }
        }

        // 原子写入
        let tmp_path = resolved.with_extension(format!("tmp.{}", uuid::Uuid::new_v4()));
        tokio::fs::write(&tmp_path, content).await
            .map_err(|e| anyhow!(
                "写入临时文件失败: {} (路径: {}, base: {})",
                e, file_path, base
            ))?;

        tokio::fs::rename(&tmp_path, &resolved).await
            .map_err(|e| anyhow!(
                "替换文件失败: {} (路径: {}, base: {})",
                e, file_path, base
            ))?;

        let action = if is_overwrite { "覆盖" } else { "创建" };
        let byte_size = content.len();
        Ok(format!(
            "成功{}文件: {}（base: {}，{} 字节）",
            action, file_path, base, byte_size
        ))
    }
}
