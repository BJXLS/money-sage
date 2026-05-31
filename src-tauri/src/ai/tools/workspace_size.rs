use async_trait::async_trait;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::path::PathBuf;

use super::LocalTool;

pub struct WorkspaceSizeTool {
    workspace_dir: PathBuf,
}

impl WorkspaceSizeTool {
    pub fn new(workspace_dir: PathBuf) -> Self {
        Self { workspace_dir }
    }
}

#[async_trait]
impl LocalTool for WorkspaceSizeTool {
    fn name(&self) -> &str {
        "workspace_size"
    }

    fn description(&self) -> &str {
        "获取当前工作区的磁盘占用大小。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    async fn execute(&self, _arguments: Value) -> Result<String> {
        let dir = self.workspace_dir.clone();

        let size = tokio::task::spawn_blocking(move || {
            fn dir_size(path: &std::path::Path) -> std::io::Result<u64> {
                let mut size = 0u64;
                for entry in std::fs::read_dir(path)? {
                    let entry = entry?;
                    let meta = entry.metadata()?;
                    if meta.is_file() {
                        size += meta.len();
                    } else if meta.is_dir() {
                        size += dir_size(&entry.path())?;
                    }
                }
                Ok(size)
            }
            dir_size(&dir)
        })
        .await
        .map_err(|e| anyhow!("计算任务失败: {}", e))?
        .map_err(|e| anyhow!("遍历目录失败: {}", e))?;

        let readable = if size < 1024 {
            format!("{} B", size)
        } else if size < 1024 * 1024 {
            format!("{:.2} KB", size as f64 / 1024.0)
        } else if size < 1024 * 1024 * 1024 {
            format!("{:.2} MB", size as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.2} GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
        };

        let result = json!({
            "bytes": size,
            "readable": readable,
        });

        Ok(serde_json::to_string(&result)?)
    }
}
