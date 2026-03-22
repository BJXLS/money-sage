use async_trait::async_trait;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use sqlx::{Column, Row, SqlitePool, ValueRef};

use super::{LocalTool, ALLOWED_TABLES};

const MAX_ROWS: usize = 200;

const FORBIDDEN_KEYWORDS: &[&str] = &[
    "INSERT", "UPDATE", "DELETE", "DROP", "ALTER", "CREATE",
    "ATTACH", "DETACH", "REPLACE", "PRAGMA", "GRANT", "REVOKE",
];

pub struct QueryDatabaseTool {
    pool: SqlitePool,
}

impl QueryDatabaseTool {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
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
}

#[async_trait]
impl LocalTool for QueryDatabaseTool {
    fn name(&self) -> &str {
        "query_database"
    }

    fn description(&self) -> &str {
        "执行 SQL SELECT 查询，获取用户的记账数据。只允许 SELECT 语句，可查询的表: categories(分类), transactions(交易记录), budgets(预算)。"
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
        let truncated = total > MAX_ROWS;
        let rows = &rows[..total.min(MAX_ROWS)];

        let data: Vec<Value> = rows.iter().map(Self::sqlite_row_to_json).collect();

        let result = json!({
            "row_count": total,
            "truncated": truncated,
            "data": data,
        });

        println!("✅ [QueryDB] 返回 {} 行数据", total);
        Ok(serde_json::to_string(&result)?)
    }
}
