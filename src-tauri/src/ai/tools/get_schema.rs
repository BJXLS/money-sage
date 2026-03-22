use async_trait::async_trait;
use anyhow::Result;
use serde_json::{json, Value};
use sqlx::{Row, SqlitePool};

use super::{LocalTool, ALLOWED_TABLES};

pub struct GetDatabaseSchemaTool {
    pool: SqlitePool,
}

impl GetDatabaseSchemaTool {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    fn column_description(table: &str, column: &str) -> &'static str {
        match (table, column) {
            ("categories", "id") => "主键",
            ("categories", "name") => "分类名称",
            ("categories", "icon") => "图标",
            ("categories", "color") => "颜色",
            ("categories", "type") => "'income' 或 'expense'",
            ("categories", "parent_id") => "父分类 ID，为空表示顶级分类",
            ("categories", "is_system") => "是否系统内置分类",

            ("transactions", "id") => "主键",
            ("transactions", "date") => "交易日期 (YYYY-MM-DD)",
            ("transactions", "type") => "'income' 或 'expense'",
            ("transactions", "amount") => "金额 (REAL)",
            ("transactions", "category_id") => "关联 categories.id",
            ("transactions", "budget_id") => "关联 budgets.id，可为空",
            ("transactions", "description") => "交易描述/备注",
            ("transactions", "note") => "附加备注",

            ("budgets", "id") => "主键",
            ("budgets", "name") => "预算名称",
            ("budgets", "category_id") => "关联 categories.id",
            ("budgets", "amount") => "预算金额 (REAL)",
            ("budgets", "budget_type") => "'time' (周期) 或 'event' (事件)",
            ("budgets", "period_type") => "'weekly' / 'monthly' / 'yearly'",
            ("budgets", "start_date") => "起始日期",
            ("budgets", "end_date") => "结束日期，可为空",
            ("budgets", "is_active") => "是否启用",

            _ => "",
        }
    }

    fn table_description(table: &str) -> &'static str {
        match table {
            "categories" => "收支分类表，支持父子层级结构",
            "transactions" => "交易记录表，记录所有收入和支出",
            "budgets" => "预算表，支持按时间周期或事件设置预算",
            _ => "",
        }
    }

    async fn get_table_schema(&self, table: &str) -> Result<Value> {
        let query = format!("PRAGMA table_info({})", table);
        let rows = sqlx::query(&query).fetch_all(&self.pool).await?;

        let columns: Vec<Value> = rows.iter().map(|row| {
            let name: String = row.get("name");
            let col_type: String = row.get("type");
            let notnull: i32 = row.get("notnull");
            let pk: i32 = row.get("pk");
            let desc = Self::column_description(table, &name);

            let mut col = json!({
                "name": name,
                "type": col_type,
                "nullable": notnull == 0,
                "primary_key": pk == 1,
            });
            if !desc.is_empty() {
                col["description"] = json!(desc);
            }
            col
        }).collect();

        Ok(json!({
            "name": table,
            "description": Self::table_description(table),
            "columns": columns,
        }))
    }
}

#[async_trait]
impl LocalTool for GetDatabaseSchemaTool {
    fn name(&self) -> &str {
        "get_database_schema"
    }

    fn description(&self) -> &str {
        "获取记账数据库的表结构信息。在执行 query_database 之前应先调用此工具了解可用的表和字段。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "table_name": {
                    "type": "string",
                    "description": "可选，指定要查看的表名。不传则返回所有可查询表的结构。",
                    "enum": ALLOWED_TABLES,
                }
            }
        })
    }

    async fn execute(&self, arguments: Value) -> Result<String> {
        let table_name = arguments.get("table_name").and_then(|v| v.as_str());

        let tables = match table_name {
            Some(name) => {
                if !ALLOWED_TABLES.contains(&name) {
                    return Err(anyhow::anyhow!("不允许访问表: {}", name));
                }
                vec![self.get_table_schema(name).await?]
            }
            None => {
                let mut result = Vec::new();
                for &table in ALLOWED_TABLES {
                    result.push(self.get_table_schema(table).await?);
                }
                result
            }
        };

        let output = json!({ "tables": tables });
        Ok(serde_json::to_string(&output)?)
    }
}
