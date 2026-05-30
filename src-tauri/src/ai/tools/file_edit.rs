use async_trait::async_trait;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::path::PathBuf;

use super::{LocalTool, workspace_path};

pub struct FileEditTool {
    workspace_dir: PathBuf,
    memory_dir: Option<PathBuf>,
    pool: Option<SqlitePool>,
}

impl FileEditTool {
    pub fn new(workspace_dir: PathBuf, memory_dir: Option<PathBuf>, pool: Option<SqlitePool>) -> Self {
        Self { workspace_dir, memory_dir, pool }
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
impl LocalTool for FileEditTool {
    fn name(&self) -> &str {
        "file_edit"
    }

    fn description(&self) -> &str {
        "对工作区或记忆目录内的已有文件进行精确的字符串替换。修改前建议先用 file_read 查看当前内容。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "文件的相对路径。base=workspace 时如 'AGENTS.md'；base=memory 时如 'factual/user-profile.md'。"
                },
                "base": {
                    "type": "string",
                    "enum": ["workspace", "memory"],
                    "default": "workspace",
                    "description": "目录基址。workspace 表示工作区目录，memory 表示记忆目录。"
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
                    "description": "若为 true，替换所有匹配项；默认 false"
                }
            },
            "required": ["file_path", "old_string", "new_string"]
        })
    }

    async fn execute(&self, arguments: Value) -> Result<String> {
        let file_path = arguments.get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("缺少 file_path 参数"))?;

        let base = arguments.get("base")
            .and_then(|v| v.as_str())
            .unwrap_or("workspace");

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

        let resolved = self.resolve_path(base, file_path)?;

        // 安全检查：禁止修改 meta/ 目录
        if base == "memory" && file_path.starts_with("meta/") {
            return Err(anyhow!(
                "禁止修改 meta/ 目录下的系统规范文件。文件: {}",
                file_path
            ));
        }

        if !resolved.exists() {
            return Err(anyhow!(
                "文件不存在，无法编辑: {}。base: {}，请确认路径正确",
                file_path, base
            ));
        }

        if resolved.is_dir() {
            return Err(anyhow!(
                "该路径是目录，无法编辑: {}。base: {}",
                file_path, base
            ));
        }

        let content = tokio::fs::read_to_string(&resolved).await
            .map_err(|e| anyhow!(
                "读取文件失败: {} (路径: {}, base: {})",
                e, file_path, base
            ))?;

        let occurrences = content.matches(old_string).count();

        if occurrences == 0 {
            return Err(anyhow!(
                "未找到 old_string，请确认文本内容完全匹配。文件: {}，base: {}",
                file_path, base
            ));
        }

        if !replace_all && occurrences > 1 {
            return Err(anyhow!(
                "old_string 在文件中出现 {} 次，不唯一。如需全部替换，请设置 replace_all: true。文件: {}，base: {}",
                occurrences, file_path, base
            ));
        }

        let new_content = if replace_all {
            content.replace(old_string, new_string)
        } else {
            content.replacen(old_string, new_string, 1)
        };

        // 安全检查：如果是 memory/ 目录的写入，扫描注入
        if base == "memory" {
            if let Err(e) = crate::memory::v3::safety::scan_memory_write(&new_content) {
                return Err(anyhow!(
                    "记忆写入安全检查未通过: {}。文件: {}，base: {}",
                    e, file_path, base
                ));
            }
        }

        // 原子写入
        let tmp_path = resolved.with_extension(format!("tmp.{}", uuid::Uuid::new_v4()));
        tokio::fs::write(&tmp_path, &new_content).await
            .map_err(|e| anyhow!(
                "写入临时文件失败: {}。文件: {}，base: {}",
                e, file_path, base
            ))?;

        tokio::fs::rename(&tmp_path, &resolved).await
            .map_err(|e| anyhow!(
                "替换文件失败: {}。文件: {}，base: {}",
                e, file_path, base
            ))?;

        // memory 文件编辑后，后台同步 FTS5 索引
        if base == "memory" {
            if let (Some(pool), Some(ref mdir)) = (&self.pool, &self.memory_dir) {
                let store = crate::memory::v3::MemoryStore::new(mdir);
                let indexer = crate::memory::v3::MemoryIndexer::new(pool.clone(), store);
                tokio::spawn(async move {
                    if let Err(e) = indexer.sync_all().await {
                        eprintln!("Memory index sync failed after file_edit: {}", e);
                    }
                });
            }
        }

        let replaced_count = if replace_all { occurrences } else { 1 };
        Ok(format!(
            "成功编辑文件: {}（base: {}，替换 {} 处）",
            file_path, base, replaced_count
        ))
    }
}
