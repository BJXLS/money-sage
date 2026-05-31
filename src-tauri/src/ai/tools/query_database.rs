use async_trait::async_trait;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use sqlx::{Column, Row, SqlitePool, ValueRef};
use std::path::PathBuf;

use super::{LocalTool, ALLOWED_TABLES, workspace_path};

const PREVIEW_ROWS: usize = 20;

const FORBIDDEN_KEYWORDS: &[&str] = &[
    "INSERT", "UPDATE", "DELETE", "DROP", "ALTER", "CREATE",
    "ATTACH", "DETACH", "REPLACE", "PRAGMA", "GRANT", "REVOKE",
];

pub struct QueryDatabaseTool {
    pool: SqlitePool,
    workspace_dir: PathBuf,
    session_id: String,
}

impl QueryDatabaseTool {
    pub fn new(pool: SqlitePool, workspace_dir: PathBuf, session_id: String) -> Self {
        Self { pool, workspace_dir, session_id }
    }

    fn validate_sql(sql: &str) -> Result<()> {
        let normalized = sql.trim().to_uppercase();

        if !normalized.starts_with("SELECT") {
            return Err(anyhow!("只允许 SELECT 查询"));
        }

        for kw in FORBIDDEN_KEYWORDS {
            let pattern = format!(" {} ", kw);
            let at_start = normalized.starts_with(&format!("{} ", kw));
            if normalized.contains(&pattern) || at_start {
                if *kw != "SELECT" {
                    return Err(anyhow!("SQL 中包含禁止的关键字: {}", kw));
                }
            }
        }

        Self::validate_tables(sql)?;

        Ok(())
    }

    fn validate_tables(sql: &str) -> Result<()> {
        let upper = sql.to_uppercase();
        let tokens: Vec<&str> = upper.split_whitespace().collect();

        for (i, token) in tokens.iter().enumerate() {
            let is_table_ref = *token == "FROM" || *token == "JOIN";
            if is_table_ref {
                if let Some(table_token) = tokens.get(i + 1) {
                    let table_name = table_token.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
                    if table_name.is_empty() { continue; }
                    let lower_name = table_name.to_lowercase();
                    if !ALLOWED_TABLES.contains(&lower_name.as_str()) {
                        return Err(anyhow!("不允许查询表: {}，允许的表: {:?}", lower_name, ALLOWED_TABLES));
                    }
                }
            }
        }

        Ok(())
    }

    fn sqlite_row_to_json(row: &sqlx::sqlite::SqliteRow) -> Value {
        let mut map = serde_json::Map::new();
        for (i, col) in row.columns().iter().enumerate() {
            let raw = row.try_get_raw(i).ok();
            let val = match raw {
                Some(r) if !r.is_null() => {
                    row.try_get::<i64, _>(i)
                        .map(|v| json!(v))
                        .or_else(|_| row.try_get::<f64, _>(i).map(|v| json!(v)))
                        .or_else(|_| row.try_get::<String, _>(i).map(|v| json!(v)))
                        .or_else(|_| row.try_get::<bool, _>(i).map(|v| json!(v)))
                        .unwrap_or(json!(null))
                }
                _ => json!(null),
            };
            map.insert(col.name().to_string(), val);
        }
        Value::Object(map)
    }

    fn sqlite_row_to_string_record(row: &sqlx::sqlite::SqliteRow) -> Vec<String> {
        let mut record = Vec::new();
        for i in 0..row.columns().len() {
            let raw = row.try_get_raw(i).ok();
            let cell = match raw {
                Some(r) if !r.is_null() => {
                    row.try_get::<i64, _>(i).map(|v| v.to_string())
                        .or_else(|_| row.try_get::<f64, _>(i).map(|v| v.to_string()))
                        .or_else(|_| row.try_get::<String, _>(i).map(|v| v))
                        .or_else(|_| row.try_get::<bool, _>(i).map(|v| v.to_string()))
                        .unwrap_or_default()
                }
                _ => String::new(),
            };
            record.push(cell);
        }
        record
    }

    async fn save_to_csv(
        &self,
        rows: &[sqlx::sqlite::SqliteRow],
        relative_path: &str,
    ) -> Result<()> {
        let file_path = workspace_path::resolve_workspace_path(&self.workspace_dir, relative_path)?;

        if let Some(parent) = file_path.parent() {
            if !parent.exists() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }

        let headers: Vec<String> = rows.first()
            .map(|r| r.columns().iter().map(|c| c.name().to_string()).collect())
            .unwrap_or_default();

        let records: Vec<Vec<String>> = rows.iter().map(Self::sqlite_row_to_string_record).collect();

        tokio::task::spawn_blocking({
            let file_path = file_path.clone();
            let headers = headers.clone();
            move || {
                let file = std::fs::File::create(&file_path)
                    .map_err(|e| anyhow!("创建 CSV 文件失败: {}", e))?;
                let mut writer = csv::Writer::from_writer(file);
                writer.write_record(&headers)
                    .map_err(|e| anyhow!("写入 CSV 表头失败: {}", e))?;
                for record in &records {
                    writer.write_record(record)
                        .map_err(|e| anyhow!("写入 CSV 数据失败: {}", e))?;
                }
                writer.flush().map_err(|e| anyhow!("刷新 CSV 失败: {}", e))?;
                Ok::<_, anyhow::Error>(())
            }
        }).await.map_err(|e| anyhow!("CSV 写入任务失败: {}", e))??;

        Ok(())
    }

    fn build_csv_preview(headers: &[String], records: &[Vec<String>], limit: usize) -> Result<String> {
        let mut preview_lines = Vec::new();
        preview_lines.push(headers.join(","));

        for record in records.iter().take(limit) {
            let mut w = csv::Writer::from_writer(Vec::new());
            w.write_record(record)?;
            let line = String::from_utf8(w.into_inner()?)?;
            preview_lines.push(line.trim_end().to_string());
        }

        Ok(preview_lines.join("\n") + "\n")
    }
}

#[async_trait]
impl LocalTool for QueryDatabaseTool {
    fn name(&self) -> &str {
        "query_database"
    }

    fn description(&self) -> &str {
        "执行 SQL SELECT 查询，获取用户的记账数据。只允许 SELECT 语句，可查询的表: categories(分类), transactions(交易记录), budgets(预算)。当结果超过20行时，会自动保存为CSV文件并返回预览和路径。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "sql": {
                    "type": "string",
                    "description": "要执行的 SQL SELECT 查询语句"
                },
                "explanation": {
                    "type": "string",
                    "description": "简要说明这条查询的目的"
                }
            },
            "required": ["sql"]
        })
    }

    async fn execute(&self, arguments: Value) -> Result<String> {
        let sql = arguments.get("sql")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("缺少 sql 参数"))?;

        println!("🔍 [QueryDB] 验证 SQL: {}", sql);
        Self::validate_sql(sql)?;

        println!("⚡ [QueryDB] 执行查询...");
        let rows = sqlx::query(sql)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| anyhow!("SQL 执行失败: {}", e))?;

        let total = rows.len();

        if total > PREVIEW_ROWS {
            let timestamp = chrono::Utc::now().timestamp_millis();
            let file_name = format!("result_{}_{}.csv", timestamp, uuid::Uuid::new_v4());
            let relative_path = format!(".query_temp/{}/{}", self.session_id, file_name);

            self.save_to_csv(&rows, &relative_path).await?;

            let headers: Vec<String> = rows.first()
                .map(|r| r.columns().iter().map(|c| c.name().to_string()).collect())
                .unwrap_or_default();
            let records: Vec<Vec<String>> = rows.iter().map(Self::sqlite_row_to_string_record).collect();
            let preview = Self::build_csv_preview(&headers, &records, PREVIEW_ROWS)?;

            let result = json!({
                "row_count": total,
                "truncated": false,
                "preview_rows": PREVIEW_ROWS,
                "csv_file_path": relative_path,
                "csv_preview": preview,
            });

            println!("✅ [QueryDB] 返回 {} 行数据，已保存 CSV: {}", total, relative_path);
            Ok(serde_json::to_string(&result)?)
        } else {
            let data: Vec<Value> = rows.iter().map(Self::sqlite_row_to_json).collect();

            let result = json!({
                "row_count": total,
                "truncated": false,
                "data": data,
            });

            println!("✅ [QueryDB] 返回 {} 行数据", total);
            Ok(serde_json::to_string(&result)?)
        }
    }
}
