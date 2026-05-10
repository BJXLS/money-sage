use async_trait::async_trait;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::path::PathBuf;

use super::{LocalTool, workspace_path};

pub struct FileEditTool {
    workspace_dir: PathBuf,
}

impl FileEditTool {
    pub fn new(workspace_dir: PathBuf) -> Self {
        Self { workspace_dir }
    }
}

#[async_trait]
impl LocalTool for FileEditTool {
    fn name(&self) -> &str {
        "file_edit"
    }

    fn description(&self) -> &str {
        "对工作区内的已有文件进行精确的字符串替换。只能修改工作区目录下的文件，file_path 必须是相对于工作区根目录的路径（如 'AGENTS.md'）。修改前建议先用 file_read 查看当前内容。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "文件在工作区内的相对路径，如 'AGENTS.md'。必须是相对于工作区根目录的路径，禁止绝对路径和路径遍历。"
                },
                "old_string": {
                    "type": "string",
                    "description": "文件中要替换的精确文本，必须完全匹配（包括缩进和空格）"
                },
                "new_string": {
                    "type": "string",
                    "description": "替换后的新文本"
                },
                "replace_all": {
                    "type": "boolean",
                    "description": "若为 true，替换所有匹配项；默认 false（要求 old_string 在文件中只出现一次）"
                }
            },
            "required": ["file_path", "old_string", "new_string"]
        })
    }

    async fn execute(&self, arguments: Value) -> Result<String> {
        let file_path = arguments.get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("缺少 file_path 参数"))?;

        let old_string = arguments.get("old_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("缺少 old_string 参数"))?;

        let new_string = arguments.get("new_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("缺少 new_string 参数"))?;

        let replace_all = arguments.get("replace_all")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if old_string.is_empty() {
            return Err(anyhow!("old_string 不能为空"));
        }

        let resolved = workspace_path::resolve_workspace_path(&self.workspace_dir, file_path)?;

        if !resolved.exists() {
            return Err(anyhow!(
                "文件不存在，无法编辑: {}。工作区路径: {}，请确认路径正确",
                file_path,
                self.workspace_dir.display()
            ));
        }

        if resolved.is_dir() {
            return Err(anyhow!(
                "该路径是目录，无法编辑: {}。工作区路径: {}",
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

        let occurrences = content.matches(old_string).count();

        if occurrences == 0 {
            return Err(anyhow!(
                "未找到 old_string，请确认文本内容完全匹配（包括缩进和空格）。文件: {}，工作区路径: {}",
                file_path,
                self.workspace_dir.display()
            ));
        }

        if !replace_all && occurrences > 1 {
            return Err(anyhow!(
                "old_string 在文件中出现 {} 次，不唯一。如需全部替换，请设置 replace_all: true。文件: {}，工作区路径: {}",
                occurrences,
                file_path,
                self.workspace_dir.display()
            ));
        }

        let new_content = if replace_all {
            content.replace(old_string, new_string)
        } else {
            content.replacen(old_string, new_string, 1)
        };

        // 原子写入：先写临时文件，再 rename
        let tmp_path = resolved.with_extension(format!("tmp.{}", uuid::Uuid::new_v4()));
        tokio::fs::write(&tmp_path, new_content).await
            .map_err(|e| anyhow!(
                "写入临时文件失败: {}。文件: {}，工作区路径: {}",
                e,
                file_path,
                self.workspace_dir.display()
            ))?;

        tokio::fs::rename(&tmp_path, &resolved).await
            .map_err(|e| anyhow!(
                "替换文件失败: {}。文件: {}，工作区路径: {}",
                e,
                file_path,
                self.workspace_dir.display()
            ))?;

        let replaced_count = if replace_all { occurrences } else { 1 };
        Ok(format!(
            "成功编辑文件: {}（替换 {} 处）。工作区路径: {}",
            file_path,
            replaced_count,
            self.workspace_dir.display()
        ))
    }
}
