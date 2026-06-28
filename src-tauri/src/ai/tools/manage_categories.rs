use async_trait::async_trait;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use sqlx::SqlitePool;

use crate::ai::tools::LocalTool;
use crate::database::Database;
use crate::models::{NewCategory, UpdateCategory};

pub struct ManageCategoriesTool {
    pool: SqlitePool,
}

impl ManageCategoriesTool {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    async fn create_category(&self, args: &Value) -> Result<String> {
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("缺少 name 参数"))?
            .to_string();

        let category_type = args
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("缺少 type 参数"))?
            .to_string();

        if category_type != "income" && category_type != "expense" {
            return Err(anyhow!("type 必须是 income 或 expense"));
        }

        let icon = args.get("icon").and_then(|v| v.as_str()).map(|s| s.to_string());
        let color = args.get("color").and_then(|v| v.as_str()).map(|s| s.to_string());
        let parent_id = args.get("parent_id").and_then(|v| v.as_i64());

        // 如果指定了 parent_id，校验父分类存在且类型一致
        if let Some(pid) = parent_id {
            let parent = sqlx::query_as::<_, crate::models::Category>(
                "SELECT * FROM categories WHERE id = ?"
            )
            .bind(pid)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| anyhow!("查询父分类失败: {}", e))?;

            if let Some(p) = parent {
                if p.r#type != category_type {
                    return Err(anyhow!("子分类类型必须与父分类一致"));
                }
            } else {
                return Err(anyhow!("父分类不存在"));
            }
        }

        let new_category = NewCategory {
            name,
            icon,
            color,
            r#type: category_type,
            parent_id,
        };

        let db = Database { pool: self.pool.clone() };
        let id = db.create_category(&new_category).await?;

        Ok(json!({
            "ok": true,
            "action": "create",
            "id": id,
            "message": "分类创建成功"
        })
        .to_string())
    }

    async fn update_category(&self, args: &Value) -> Result<String> {
        let id = args
            .get("id")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| anyhow!("缺少 id 参数"))?;

        let name = args.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
        let icon = args.get("icon").and_then(|v| v.as_str()).map(|s| s.to_string());
        let color = args.get("color").and_then(|v| v.as_str()).map(|s| s.to_string());
        let parent_id = args.get("parent_id").and_then(|v| {
            if v.is_null() {
                Some(None)
            } else {
                v.as_i64().map(Some)
            }
        });

        // 获取当前分类信息，用于校验
        let db = Database { pool: self.pool.clone() };
        let current = sqlx::query_as::<_, crate::models::Category>(
            "SELECT * FROM categories WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| anyhow!("查询分类失败: {}", e))?
        .ok_or_else(|| anyhow!("分类不存在"))?;

        // 如果修改 parent_id，校验父分类存在、类型一致且不是自己或自己的后代
        if let Some(Some(pid)) = parent_id {
            if pid == id {
                return Err(anyhow!("不能将分类设置为自己的父分类"));
            }

            let parent = sqlx::query_as::<_, crate::models::Category>(
                "SELECT * FROM categories WHERE id = ?"
            )
            .bind(pid)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| anyhow!("查询父分类失败: {}", e))?
            .ok_or_else(|| anyhow!("父分类不存在"))?;

            if parent.r#type != current.r#type {
                return Err(anyhow!("子分类类型必须与父分类一致"));
            }

            // 检查是否把自己的后代设为了父分类（避免循环）
            if self.is_descendant(pid, id).await? {
                return Err(anyhow!("不能将子分类设置为父分类，否则会造成循环层级"));
            }
        }

        let update = UpdateCategory {
            name,
            icon,
            color,
            parent_id,
        };

        db.update_category(id, &update).await?;

        Ok(json!({
            "ok": true,
            "action": "update",
            "id": id,
            "message": "分类更新成功"
        })
        .to_string())
    }

    async fn delete_category(&self, args: &Value) -> Result<String> {
        let id = args
            .get("id")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| anyhow!("缺少 id 参数"))?;

        let db = Database { pool: self.pool.clone() };

        // 检查是否有子分类
        let sub_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM categories WHERE parent_id = ?")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| anyhow!("查询子分类失败: {}", e))?;

        if sub_count > 0 {
            return Err(anyhow!("该分类下存在子分类，无法删除"));
        }

        // 检查是否有关联交易记录
        let transaction_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM transactions WHERE category_id = ?")
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| anyhow!("查询交易记录失败: {}", e))?;

        if transaction_count > 0 {
            return Err(anyhow!(
                "该分类下存在 {} 条交易记录，无法直接删除。建议先修改这些交易的分类，或把该分类重命名。",
                transaction_count
            ));
        }

        db.delete_category(id).await?;

        Ok(json!({
            "ok": true,
            "action": "delete",
            "id": id,
            "message": "分类删除成功"
        })
        .to_string())
    }

    /// 检查 target_id 是否是 source_id 的后代（用于避免循环层级）
    async fn is_descendant(&self, target_id: i64, source_id: i64) -> Result<bool> {
        let mut current = target_id;
        loop {
            let parent: Option<i64> = sqlx::query_scalar(
                "SELECT parent_id FROM categories WHERE id = ?"
            )
            .bind(current)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| anyhow!("查询分类层级失败: {}", e))?;

            match parent {
                Some(pid) if pid == source_id => return Ok(true),
                Some(pid) => current = pid,
                None => return Ok(false),
            }
        }
    }
}

#[async_trait]
impl LocalTool for ManageCategoriesTool {
    fn name(&self) -> &str {
        "manage_categories"
    }

    fn description(&self) -> &str {
        "管理用户的收支分类。支持新增(create)、修改(update)、删除(delete)分类，包括系统内置分类。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["create", "update", "delete"],
                    "description": "操作类型：create 新增，update 修改，delete 删除"
                },
                "id": {
                    "type": "integer",
                    "description": "分类 ID，update 和 delete 时必填"
                },
                "name": {
                    "type": "string",
                    "description": "分类名称，create 必填，update 可选"
                },
                "type": {
                    "type": "string",
                    "enum": ["income", "expense"],
                    "description": "分类类型，create 必填"
                },
                "parent_id": {
                    "type": ["integer", "null"],
                    "description": "父分类 ID，null 表示顶级分类，不传表示不修改"
                },
                "icon": {
                    "type": "string",
                    "description": "分类图标（emoji 或字符）"
                },
                "color": {
                    "type": "string",
                    "description": "分类颜色（hex 字符串，如 #FF5733）"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, arguments: Value) -> Result<String> {
        let action = arguments
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("缺少 action 参数"))?;

        match action {
            "create" => self.create_category(&arguments).await,
            "update" => self.update_category(&arguments).await,
            "delete" => self.delete_category(&arguments).await,
            _ => Err(anyhow!("不支持的 action: {}", action)),
        }
    }
}
