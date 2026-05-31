use async_trait::async_trait;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::path::PathBuf;

use super::{LocalTool, workspace_path};

pub struct CsvReadTool {
    workspace_dir: PathBuf,
}

impl CsvReadTool {
    pub fn new(workspace_dir: PathBuf) -> Self {
        Self { workspace_dir }
    }
}

#[async_trait]
impl LocalTool for CsvReadTool {
    fn name(&self) -> &str {
        "csv_read"
    }

    fn description(&self) -> &str {
        "读取工作区内的 CSV 文件，支持分页。返回 Markdown 表格格式，便于查看数据。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "CSV 文件的相对路径，如 '.query_temp/session-xxx/result_xxx.csv'"
                },
                "offset": {
                    "type": "integer",
                    "description": "起始数据行号（1-indexed，不含表头），可选，默认从第1行开始"
                },
                "limit": {
                    "type": "integer",
                    "description": "最多读取多少行数据，可选，默认100行"
                }
            },
            "required": ["file_path"]
        })
    }

    async fn execute(&self, arguments: Value) -> Result<String> {
        let file_path = arguments.get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("缺少 file_path 参数"))?;

        let offset = arguments.get("offset")
            .and_then(|v| v.as_u64())
            .unwrap_or(1)
            .saturating_sub(1) as usize;

        let limit = arguments.get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(100) as usize;

        let resolved = workspace_path::resolve_workspace_path(&self.workspace_dir, file_path)
            .map_err(|e| anyhow!("路径解析失败: {}", e))?;

        if !resolved.exists() {
            return Err(anyhow!(
                "文件不存在: {}。请确认文件路径正确",
                file_path
            ));
        }

        if resolved.is_dir() {
            return Err(anyhow!(
                "该路径是目录，无法读取: {}",
                file_path
            ));
        }

        let content = tokio::fs::read_to_string(&resolved).await
            .map_err(|e| anyhow!(
                "读取文件失败: {} (路径: {})",
                e, file_path
            ))?;

        let mut reader = csv::Reader::from_reader(content.as_bytes());

        let headers = reader.headers()
            .map_err(|e| anyhow!("解析 CSV 表头失败: {}", e))?
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();

        let mut records: Vec<csv::StringRecord> = Vec::new();
        for result in reader.records() {
            let record = result.map_err(|e| anyhow!("解析 CSV 行失败: {}", e))?;
            records.push(record);
        }

        let total = records.len();
        let end = (offset + limit).min(total);
        let slice = if offset < total {
            &records[offset..end]
        } else {
            &[] as &[csv::StringRecord]
        };

        // 构建 Markdown 表格
        let mut output = String::new();
        output.push_str(&format!("文件: {} (共 {} 行数据)\n", file_path, total));
        if offset >= total {
            output.push_str("指定范围无内容\n");
            return Ok(output);
        }
        output.push_str(&format!("显示第 {}-{} 行：\n\n", offset + 1, end));

        // 表头
        output.push_str("| ");
        for h in &headers {
            output.push_str(&format!("{} | ", h));
        }
        output.push('\n');

        // 分隔线
        output.push_str("|");
        for _ in &headers {
            output.push_str(" --- |");
        }
        output.push('\n');

        // 数据行
        for record in slice {
            output.push_str("| ");
            for cell in record.iter() {
                output.push_str(&format!("{} | ", cell));
            }
            output.push('\n');
        }

        Ok(output)
    }
}
