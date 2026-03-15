use sqlx::{SqlitePool, Row};
use chrono::{NaiveDate, Utc};
use anyhow::Result;
use crate::models::*;

pub struct Database {
    pub pool: SqlitePool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = SqlitePool::connect(database_url).await?;
        
        // 手动创建表结构
        Self::create_tables(&pool).await?;
        
        // 插入默认数据
        Self::insert_default_data(&pool).await?;
        
        Ok(Database { pool })
    }

    async fn create_tables(pool: &SqlitePool) -> Result<()> {
        // 创建分类表
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS categories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                icon TEXT,
                color TEXT,
                type TEXT NOT NULL CHECK (type IN ('income', 'expense')),
                parent_id INTEGER,
                is_system INTEGER DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (parent_id) REFERENCES categories(id) ON DELETE CASCADE
            )
        "#).execute(pool).await?;

        // 检查并添加 parent_id 字段（用于数据库迁移）
        let columns_result = sqlx::query("PRAGMA table_info(categories)")
            .fetch_all(pool)
            .await?;
        
        let has_parent_id = columns_result.iter().any(|row| {
            let column_name: String = row.get("name");
            column_name == "parent_id"
        });
        
        if !has_parent_id {
            println!("添加 parent_id 字段到 categories 表");
            sqlx::query("ALTER TABLE categories ADD COLUMN parent_id INTEGER")
                .execute(pool)
                .await?;
            
            // 添加外键约束（注意：SQLite 的 ALTER TABLE 不支持添加外键，所以我们跳过这步）
            // 在新创建的表中，外键约束已经在 CREATE TABLE 中定义
        }

        // 检查 transactions 表是否存在以及 amount 列的类型
        let table_exists = sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name='transactions'")
            .fetch_optional(pool)
            .await?;
        
        if let Some(_) = table_exists {
            // 检查 amount 列的类型
            let columns_result = sqlx::query("PRAGMA table_info(transactions)")
                .fetch_all(pool)
                .await?;
            
            let amount_column = columns_result.iter().find(|row| {
                let column_name: String = row.get("name");
                column_name == "amount"
            });
            
            // 检查是否有 budget_id 字段
            let has_budget_id = columns_result.iter().any(|row| {
                let column_name: String = row.get("name");
                column_name == "budget_id"
            });
            
            if let Some(column) = amount_column {
                let column_type: String = column.get("type");
                let needs_migration = column_type.to_uppercase().contains("DECIMAL") || 
                                    column_type.to_uppercase().contains("INTEGER") ||
                                    !has_budget_id;
                                    
                if needs_migration {
                    println!("需要迁移 transactions 表结构");
                    
                    // 备份现有数据
                    sqlx::query("CREATE TABLE transactions_backup AS SELECT * FROM transactions")
                        .execute(pool)
                        .await?;
                    
                    // 删除原表
                    sqlx::query("DROP TABLE transactions")
                        .execute(pool)
                        .await?;
                    
                    // 重新创建表
                    sqlx::query(r#"
                        CREATE TABLE transactions (
                            id INTEGER PRIMARY KEY AUTOINCREMENT,
                            date DATE NOT NULL,
                            type TEXT NOT NULL CHECK (type IN ('income', 'expense')),
                            amount REAL NOT NULL,
                            category_id INTEGER NOT NULL,
                            budget_id INTEGER,
                            description TEXT,
                            note TEXT,
                            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                            FOREIGN KEY (category_id) REFERENCES categories(id),
                            FOREIGN KEY (budget_id) REFERENCES budgets(id)
                        )
                    "#).execute(pool).await?;
                    
                    // 恢复数据
                    sqlx::query(r#"
                        INSERT INTO transactions (id, date, type, amount, category_id, budget_id, description, note, created_at, updated_at)
                        SELECT id, date, type, CAST(amount AS REAL), category_id, NULL, description, note, created_at, updated_at
                        FROM transactions_backup
                    "#).execute(pool).await?;
                    
                    // 删除备份表
                    sqlx::query("DROP TABLE transactions_backup")
                        .execute(pool)
                        .await?;
                    
                    println!("transactions 表迁移完成");
                }
            }
        } else {
            // 创建新的 transactions 表
            sqlx::query(r#"
                CREATE TABLE transactions (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    date DATE NOT NULL,
                    type TEXT NOT NULL CHECK (type IN ('income', 'expense')),
                    amount REAL NOT NULL,
                    category_id INTEGER NOT NULL,
                    budget_id INTEGER,
                    description TEXT,
                    note TEXT,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY (category_id) REFERENCES categories(id),
                    FOREIGN KEY (budget_id) REFERENCES budgets(id)
                )
            "#).execute(pool).await?;
        }

        // 检查 budgets 表是否存在以及字段完整性
        let budget_table_exists = sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name='budgets'")
            .fetch_optional(pool)
            .await?;
        
        if let Some(_) = budget_table_exists {
            // 检查表结构
            let columns_result = sqlx::query("PRAGMA table_info(budgets)")
                .fetch_all(pool)
                .await?;
            
            let has_name = columns_result.iter().any(|row| {
                let column_name: String = row.get("name");
                column_name == "name"
            });
            
            let has_budget_type = columns_result.iter().any(|row| {
                let column_name: String = row.get("name");
                column_name == "budget_type"
            });
            
            let amount_column = columns_result.iter().find(|row| {
                let column_name: String = row.get("name");
                column_name == "amount"
            });
            
            let needs_migration = !has_name || !has_budget_type || 
                                (amount_column.map(|col| {
                                    let column_type: String = col.get("type");
                                    column_type.to_uppercase().contains("DECIMAL") || 
                                    column_type.to_uppercase().contains("INTEGER")
                                }).unwrap_or(false));
            
            if needs_migration {
                println!("需要迁移 budgets 表结构，缺少必要字段或数据类型不正确");
                
                // 备份现有数据
                sqlx::query("CREATE TABLE budgets_backup AS SELECT * FROM budgets")
                    .execute(pool)
                    .await?;
                
                // 删除原表
                sqlx::query("DROP TABLE budgets")
                    .execute(pool)
                    .await?;
                
                // 重新创建表
                sqlx::query(r#"
                    CREATE TABLE budgets (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        name TEXT NOT NULL,
                        category_id INTEGER NOT NULL,
                        amount REAL NOT NULL,
                        budget_type TEXT NOT NULL DEFAULT 'time' CHECK (budget_type IN ('time', 'event')),
                        period_type TEXT NOT NULL DEFAULT 'monthly' CHECK (period_type IN ('weekly', 'monthly', 'yearly')),
                        start_date DATE NOT NULL,
                        end_date DATE,
                        is_active INTEGER DEFAULT 1,
                        created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                        updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                        FOREIGN KEY (category_id) REFERENCES categories(id)
                    )
                "#).execute(pool).await?;
                
                // 恢复数据，处理可能缺失的字段
                let backup_columns = sqlx::query("PRAGMA table_info(budgets_backup)")
                    .fetch_all(pool)
                    .await?;
                
                let backup_has_name = backup_columns.iter().any(|row| {
                    let column_name: String = row.get("name");
                    column_name == "name"
                });
                
                let backup_has_budget_type = backup_columns.iter().any(|row| {
                    let column_name: String = row.get("name");
                    column_name == "budget_type"
                });
                
                if backup_has_name && backup_has_budget_type {
                    sqlx::query(r#"
                        INSERT INTO budgets (id, name, category_id, amount, budget_type, period_type, start_date, end_date, is_active, created_at, updated_at)
                        SELECT id, name, category_id, CAST(amount AS REAL), budget_type, period_type, start_date, end_date, is_active, created_at, updated_at
                        FROM budgets_backup
                    "#).execute(pool).await?;
                } else {
                    // 如果备份表缺少字段，使用默认值
                    let name_field = if backup_has_name { "name" } else { "'预算-' || id" };
                    let budget_type_field = if backup_has_budget_type { "budget_type" } else { "'time'" };
                    
                    let query = format!(r#"
                        INSERT INTO budgets (id, name, category_id, amount, budget_type, period_type, start_date, end_date, is_active, created_at, updated_at)
                        SELECT id, {}, category_id, CAST(amount AS REAL), {}, period_type, start_date, end_date, is_active, created_at, updated_at
                        FROM budgets_backup
                    "#, name_field, budget_type_field);
                    
                    sqlx::query(&query).execute(pool).await?;
                }
                
                // 删除备份表
                sqlx::query("DROP TABLE budgets_backup")
                    .execute(pool)
                    .await?;
                
                println!("budgets 表迁移完成");
            }
        } else {
            // 创建新的 budgets 表
            sqlx::query(r#"
                CREATE TABLE budgets (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    category_id INTEGER NOT NULL,
                    amount REAL NOT NULL,
                    budget_type TEXT NOT NULL DEFAULT 'time' CHECK (budget_type IN ('time', 'event')),
                    period_type TEXT NOT NULL DEFAULT 'monthly' CHECK (period_type IN ('weekly', 'monthly', 'yearly')),
                    start_date DATE NOT NULL,
                    end_date DATE,
                    is_active INTEGER DEFAULT 1,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY (category_id) REFERENCES categories(id)
                )
            "#).execute(pool).await?;
        }

        // 创建大模型配置表（泛化版：支持任意 OpenAI 兼容接口）
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS llm_configs (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                config_name TEXT    NOT NULL DEFAULT '',
                provider    TEXT    NOT NULL DEFAULT '',
                base_url    TEXT    NOT NULL DEFAULT '',
                api_key     TEXT    NOT NULL DEFAULT '',
                model       TEXT    NOT NULL DEFAULT '',
                temperature      REAL    NOT NULL DEFAULT 0.7,
                max_tokens       INTEGER NOT NULL DEFAULT 2048,
                enable_thinking  INTEGER NOT NULL DEFAULT 0,
                is_active        INTEGER NOT NULL DEFAULT 0,
                created_at  DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at  DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        "#).execute(pool).await?;

        // 迁移旧版 llm_configs 表：为已有表补充缺失列（忽略已存在的错误）
        let _ = sqlx::query("ALTER TABLE llm_configs ADD COLUMN config_name TEXT NOT NULL DEFAULT ''").execute(pool).await;
        let _ = sqlx::query("ALTER TABLE llm_configs ADD COLUMN provider TEXT NOT NULL DEFAULT ''").execute(pool).await;
        let _ = sqlx::query("ALTER TABLE llm_configs ADD COLUMN base_url TEXT NOT NULL DEFAULT ''").execute(pool).await;
        let _ = sqlx::query("ALTER TABLE llm_configs ADD COLUMN api_key TEXT NOT NULL DEFAULT ''").execute(pool).await;
        let _ = sqlx::query("ALTER TABLE llm_configs ADD COLUMN model TEXT NOT NULL DEFAULT ''").execute(pool).await;
        let _ = sqlx::query("ALTER TABLE llm_configs ADD COLUMN temperature REAL NOT NULL DEFAULT 0.7").execute(pool).await;
        let _ = sqlx::query("ALTER TABLE llm_configs ADD COLUMN max_tokens INTEGER NOT NULL DEFAULT 2048").execute(pool).await;
        let _ = sqlx::query("ALTER TABLE llm_configs ADD COLUMN enable_thinking INTEGER NOT NULL DEFAULT 0").execute(pool).await;
        // 将旧 platform/app_key 列迁移到新列（仅当新列为空时）
        let _ = sqlx::query(
            "UPDATE llm_configs SET provider = platform, api_key = app_key WHERE provider = '' AND platform IS NOT NULL AND platform != ''"
        ).execute(pool).await;

        // 智能分析会话表
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS analysis_sessions (
                id          TEXT PRIMARY KEY,
                title       TEXT NOT NULL DEFAULT '',
                config_id   INTEGER,
                created_at  DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at  DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        "#).execute(pool).await?;

        // 智能分析消息表
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS analysis_messages (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id  TEXT NOT NULL,
                role        TEXT NOT NULL,
                content     TEXT NOT NULL DEFAULT '',
                created_at  DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (session_id) REFERENCES analysis_sessions(id) ON DELETE CASCADE
            )
        "#).execute(pool).await?;

        // 创建索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_transactions_date ON transactions(date)").execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_transactions_category ON transactions(category_id)").execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_transactions_type ON transactions(type)").execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_categories_type ON categories(type)").execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_categories_parent ON categories(parent_id)").execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_budgets_category ON budgets(category_id)").execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_budgets_period ON budgets(start_date, end_date)").execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_llm_configs_active ON llm_configs(is_active)").execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_analysis_messages_session ON analysis_messages(session_id)").execute(pool).await?;

        Ok(())
    }

    async fn insert_default_data(pool: &SqlitePool) -> Result<()> {
        // 检查是否已存在默认分类
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM categories WHERE is_system = 1")
            .fetch_one(pool)
            .await?;

        if count == 0 {
            // 插入系统默认分类
            let categories = [
                // 支出分类
                ("餐饮", "🍽️", "#FF6B6B", "expense"),
                ("交通", "🚗", "#4ECDC4", "expense"),
                ("购物", "🛍️", "#45B7D1", "expense"),
                ("娱乐", "🎬", "#96CEB4", "expense"),
                ("住房", "🏠", "#FFEAA7", "expense"),
                ("医疗", "⚕️", "#DDA0DD", "expense"),
                ("教育", "📚", "#FFB74D", "expense"),
                ("其他支出", "💰", "#A0A0A0", "expense"),
                // 收入分类
                ("工资", "💼", "#52C41A", "income"),
                ("兼职", "💪", "#1890FF", "income"),
                ("投资", "📈", "#722ED1", "income"),
                ("其他收入", "💰", "#52C41A", "income"),
            ];

            for (name, icon, color, category_type) in categories {
                sqlx::query(r#"
                    INSERT OR IGNORE INTO categories (name, icon, color, type, parent_id, is_system, created_at, updated_at) 
                    VALUES (?, ?, ?, ?, NULL, 1, ?, ?)
                "#)
                .bind(name)
                .bind(icon)
                .bind(color)
                .bind(category_type)
                .bind(Utc::now())
                .bind(Utc::now())
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    // 分类相关方法
    pub async fn get_categories(&self) -> Result<Vec<Category>> {
        let categories = sqlx::query_as::<_, Category>(
            "SELECT * FROM categories ORDER BY is_system DESC, name ASC"
        )
        .fetch_all(&self.pool)
        .await?;
        
        Ok(categories)
    }

    pub async fn get_categories_by_type(&self, category_type: &str) -> Result<Vec<Category>> {
        let categories = sqlx::query_as::<_, Category>(
            "SELECT * FROM categories WHERE type = ? ORDER BY is_system DESC, name ASC"
        )
        .bind(category_type)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(categories)
    }

    pub async fn create_category(&self, category: &NewCategory) -> Result<i64> {
        let result = sqlx::query(
            "INSERT INTO categories (name, icon, color, type, parent_id, created_at, updated_at) 
             VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&category.name)
        .bind(&category.icon)
        .bind(&category.color)
        .bind(&category.r#type)
        .bind(&category.parent_id)
        .bind(Utc::now())
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;
        
        Ok(result.last_insert_rowid())
    }

    pub async fn update_category(&self, id: i64, category: &UpdateCategory) -> Result<()> {
        // 检查分类是否存在
        let existing_category = sqlx::query("SELECT id FROM categories WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
            
        if existing_category.is_none() {
            return Err(anyhow::anyhow!("分类不存在"));
        }

        // 简化的更新方法 - 只更新提供的字段
        if let Some(name) = &category.name {
            sqlx::query("UPDATE categories SET name = ?, updated_at = ? WHERE id = ?")
                .bind(name)
                .bind(Utc::now())
                .bind(id)
                .execute(&self.pool)
                .await?;
        }
        
        if let Some(icon) = &category.icon {
            sqlx::query("UPDATE categories SET icon = ?, updated_at = ? WHERE id = ?")
                .bind(icon)
                .bind(Utc::now())
                .bind(id)
                .execute(&self.pool)
                .await?;
        }
        
        if let Some(color) = &category.color {
            sqlx::query("UPDATE categories SET color = ?, updated_at = ? WHERE id = ?")
                .bind(color)
                .bind(Utc::now())
                .bind(id)
                .execute(&self.pool)
                .await?;
        }
        
        if let Some(parent_id) = &category.parent_id {
            sqlx::query("UPDATE categories SET parent_id = ?, updated_at = ? WHERE id = ?")
                .bind(parent_id)
                .bind(Utc::now())
                .bind(id)
                .execute(&self.pool)
                .await?;
        }
        
        Ok(())
    }

    pub async fn delete_category(&self, id: i64) -> Result<()> {
        // 检查是否有子分类
        let sub_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM categories WHERE parent_id = ?")
            .bind(id)
            .fetch_one(&self.pool)
            .await?;
            
        if sub_count > 0 {
            return Err(anyhow::anyhow!("该分类下有子分类，无法删除"));
        }
        
        // 检查是否有关联的交易记录
        let transaction_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM transactions WHERE category_id = ?")
            .bind(id)
            .fetch_one(&self.pool)
            .await?;
            
        if transaction_count > 0 {
            return Err(anyhow::anyhow!("该分类下有交易记录，无法删除"));
        }
        
        sqlx::query("DELETE FROM categories WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
            
        Ok(())
    }

    pub async fn get_parent_categories(&self, category_type: &str) -> Result<Vec<Category>> {
        let categories = sqlx::query_as::<_, Category>(
            "SELECT * FROM categories WHERE type = ? AND parent_id IS NULL ORDER BY is_system DESC, name ASC"
        )
        .bind(category_type)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(categories)
    }

    pub async fn get_sub_categories(&self, parent_id: i64) -> Result<Vec<Category>> {
        let categories = sqlx::query_as::<_, Category>(
            "SELECT * FROM categories WHERE parent_id = ? ORDER BY is_system DESC, name ASC"
        )
        .bind(parent_id)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(categories)
    }

    // 交易记录相关方法
    pub async fn get_transactions(&self, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<TransactionWithCategory>> {
        let limit = limit.unwrap_or(50);
        let offset = offset.unwrap_or(0);
        
        let transactions = sqlx::query_as::<_, TransactionWithCategory>(
            r#"
            SELECT 
                t.id, t.date, t.type, t.amount, t.category_id, t.budget_id, t.description, t.note, 
                t.created_at, t.updated_at,
                c.name as category_name, c.icon as category_icon, c.color as category_color,
                b.name as budget_name
            FROM transactions t
            JOIN categories c ON t.category_id = c.id
            LEFT JOIN budgets b ON t.budget_id = b.id
            ORDER BY t.date DESC, t.created_at DESC
            LIMIT ? OFFSET ?
            "#
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(transactions)
    }

    pub async fn get_transactions_by_date_range(&self, start_date: &NaiveDate, end_date: &NaiveDate) -> Result<Vec<TransactionWithCategory>> {
        let transactions = sqlx::query_as::<_, TransactionWithCategory>(
            r#"
            SELECT 
                t.id, t.date, t.type, t.amount, t.category_id, t.budget_id, t.description, t.note, 
                t.created_at, t.updated_at,
                c.name as category_name, c.icon as category_icon, c.color as category_color,
                b.name as budget_name
            FROM transactions t
            JOIN categories c ON t.category_id = c.id
            LEFT JOIN budgets b ON t.budget_id = b.id
            WHERE t.date BETWEEN ? AND ?
            ORDER BY t.date DESC, t.created_at DESC
            "#
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_all(&self.pool)
        .await?;
        
        Ok(transactions)
    }

    pub async fn create_transaction(&self, transaction: &NewTransaction) -> Result<i64> {
        let result = sqlx::query(
            "INSERT INTO transactions (date, type, amount, category_id, budget_id, description, note, created_at, updated_at) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&transaction.date)
        .bind(&transaction.r#type)
        .bind(transaction.amount)
        .bind(transaction.category_id)
        .bind(transaction.budget_id)
        .bind(&transaction.description)
        .bind(&transaction.note)
        .bind(Utc::now())
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;
        
        Ok(result.last_insert_rowid())
    }

    pub async fn update_transaction(&self, id: i64, transaction: &UpdateTransaction) -> Result<()> {
        let mut query_parts = vec!["UPDATE transactions SET updated_at = ?"];
        let mut bindings: Vec<String> = vec![Utc::now().to_rfc3339()];
        
        if let Some(date) = &transaction.date {
            query_parts.push(", date = ?");
            bindings.push(date.to_string());
        }
        if let Some(transaction_type) = &transaction.r#type {
            query_parts.push(", type = ?");
            bindings.push(transaction_type.clone());
        }
        if let Some(amount) = transaction.amount {
            query_parts.push(", amount = ?");
            bindings.push(amount.to_string());
        }
        if let Some(category_id) = transaction.category_id {
            query_parts.push(", category_id = ?");
            bindings.push(category_id.to_string());
        }
        if let Some(budget_id) = &transaction.budget_id {
            query_parts.push(", budget_id = ?");
            if let Some(bid) = budget_id {
                bindings.push(bid.to_string());
            } else {
                bindings.push("NULL".to_string());
            }
        }
        if let Some(description) = &transaction.description {
            query_parts.push(", description = ?");
            bindings.push(description.clone());
        }
        if let Some(note) = &transaction.note {
            query_parts.push(", note = ?");
            bindings.push(note.clone());
        }
        
        query_parts.push(" WHERE id = ?");
        bindings.push(id.to_string());
        
        let query_str = query_parts.join("");
        let mut query = sqlx::query(&query_str);
        
        for binding in bindings {
            query = query.bind(binding);
        }
        
        query.execute(&self.pool).await?;
        Ok(())
    }

    pub async fn delete_transaction(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM transactions WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // 统计相关方法
    pub async fn get_monthly_stats(&self, months: i32) -> Result<Vec<MonthlyStats>> {
        let stats = sqlx::query(
            r#"
            SELECT 
                strftime('%Y-%m', date) as month,
                COALESCE(SUM(CASE WHEN type = 'income' THEN amount ELSE 0.0 END), 0.0) as income,
                COALESCE(SUM(CASE WHEN type = 'expense' THEN amount ELSE 0.0 END), 0.0) as expense
            FROM transactions 
            WHERE date >= date('now', '-' || ? || ' months')
            GROUP BY strftime('%Y-%m', date)
            ORDER BY month DESC
            "#
        )
        .bind(months)
        .fetch_all(&self.pool)
        .await?;
        
        let monthly_stats: Vec<MonthlyStats> = stats
            .into_iter()
            .map(|row| {
                let income: f64 = row.get("income");
                let expense: f64 = row.get("expense");
                MonthlyStats {
                    month: row.get("month"),
                    income,
                    expense,
                    balance: income - expense,
                }
            })
            .collect();
        
        Ok(monthly_stats)
    }

    pub async fn get_category_stats(&self, start_date: &NaiveDate, end_date: &NaiveDate, transaction_type: &str) -> Result<Vec<CategoryStats>> {
        let total: f64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(amount), 0.0) FROM transactions WHERE date BETWEEN ? AND ? AND type = ?"
        )
        .bind(start_date)
        .bind(end_date)
        .bind(transaction_type)
        .fetch_one(&self.pool)
        .await?;
        
        let stats = sqlx::query(
            r#"
            SELECT 
                c.id as category_id, c.name as category_name, c.icon as category_icon, c.color as category_color,
                COALESCE(SUM(t.amount), 0.0) as amount
            FROM categories c
            LEFT JOIN transactions t ON c.id = t.category_id AND t.date BETWEEN ? AND ? AND t.type = ?
            WHERE c.type = ?
            GROUP BY c.id, c.name, c.icon, c.color
            HAVING amount > 0
            ORDER BY amount DESC
            "#
        )
        .bind(start_date)
        .bind(end_date)
        .bind(transaction_type)
        .bind(transaction_type)
        .fetch_all(&self.pool)
        .await?;
        
        let category_stats: Vec<CategoryStats> = stats
            .into_iter()
            .map(|row| {
                let amount: f64 = row.get("amount");
                let percentage = if total > 0.0 { amount / total * 100.0 } else { 0.0 };
                CategoryStats {
                    category_id: row.get("category_id"),
                    category_name: row.get("category_name"),
                    category_icon: row.get("category_icon"),
                    category_color: row.get("category_color"),
                    amount,
                    percentage,
                }
            })
            .collect();
        
        Ok(category_stats)
    }

    // 预算相关方法
    pub async fn get_budgets(&self) -> Result<Vec<BudgetProgress>> {
        let budgets = sqlx::query(
            r#"
            SELECT 
                b.id, b.name, b.category_id, b.amount, b.budget_type, b.period_type, b.start_date, b.end_date, b.is_active,
                b.created_at, b.updated_at,
                c.name as category_name, c.icon as category_icon, c.color as category_color,
                COALESCE(SUM(t.amount), 0.0) as spent
            FROM budgets b
            JOIN categories c ON b.category_id = c.id
            LEFT JOIN transactions t ON t.budget_id = b.id 
                AND t.type = 'expense'
                AND (
                    (b.budget_type = 'time' AND t.date BETWEEN b.start_date AND COALESCE(b.end_date, date('now')))
                    OR (b.budget_type = 'event')
                )
            WHERE b.is_active = 1
            GROUP BY b.id, b.name, b.category_id, b.amount, b.budget_type, b.period_type, b.start_date, b.end_date, 
                     b.is_active, b.created_at, b.updated_at, c.name, c.icon, c.color
            ORDER BY b.created_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await?;
        
        let budget_progress: Vec<BudgetProgress> = budgets
            .into_iter()
            .map(|row| {
                let amount: f64 = row.get("amount");
                let spent: f64 = row.get("spent");
                let remaining = amount - spent;
                let percentage = if amount > 0.0 { spent / amount * 100.0 } else { 0.0 };
                
                BudgetProgress {
                    // Budget fields
                    id: row.get("id"),
                    name: row.get("name"),
                    category_id: row.get("category_id"),
                    amount,
                    budget_type: row.get("budget_type"),
                    period_type: row.get("period_type"),
                    start_date: row.get("start_date"),
                    end_date: row.get("end_date"),
                    is_active: row.get("is_active"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                    // Category fields
                    category_name: row.get("category_name"),
                    category_icon: row.get("category_icon"),
                    category_color: row.get("category_color"),
                    // Progress fields
                    spent,
                    remaining,
                    percentage,
                }
            })
            .collect();
        
        Ok(budget_progress)
    }

    pub async fn create_budget(&self, budget: &NewBudget) -> Result<i64> {
        let result = sqlx::query(
            "INSERT INTO budgets (name, category_id, amount, budget_type, period_type, start_date, end_date, created_at, updated_at) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&budget.name)
        .bind(budget.category_id)
        .bind(budget.amount)
        .bind(&budget.budget_type)
        .bind(&budget.period_type)
        .bind(&budget.start_date)
        .bind(&budget.end_date)
        .bind(Utc::now())
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;
        
        Ok(result.last_insert_rowid())
    }

    pub async fn update_budget(&self, id: i64, budget: &UpdateBudget) -> Result<()> {
        // 构建动态SQL语句
        let mut updates = Vec::new();
        
        if budget.name.is_some() {
            updates.push("name = ?");
        }
        if budget.category_id.is_some() {
            updates.push("category_id = ?");
        }
        if budget.amount.is_some() {
            updates.push("amount = ?");
        }
        if budget.budget_type.is_some() {
            updates.push("budget_type = ?");
        }
        if budget.period_type.is_some() {
            updates.push("period_type = ?");
        }
        if budget.start_date.is_some() {
            updates.push("start_date = ?");
        }
        if budget.end_date.is_some() {
            updates.push("end_date = ?");
        }
        
        if updates.is_empty() {
            return Ok(()); // 没有要更新的字段
        }
        
        updates.push("updated_at = ?");
        
        let sql = format!("UPDATE budgets SET {} WHERE id = ?", updates.join(", "));
        let mut query = sqlx::query(&sql);
        
        // 按顺序绑定参数
        if let Some(name) = &budget.name {
            query = query.bind(name);
        }
        if let Some(category_id) = budget.category_id {
            query = query.bind(category_id);
        }
        if let Some(amount) = budget.amount {
            query = query.bind(amount);
        }
        if let Some(budget_type) = &budget.budget_type {
            query = query.bind(budget_type);
        }
        if let Some(period_type) = &budget.period_type {
            query = query.bind(period_type);
        }
        if let Some(start_date) = budget.start_date {
            query = query.bind(start_date);
        }
        if let Some(end_date) = &budget.end_date {
            query = query.bind(end_date);
        }
        
        query = query.bind(Utc::now()).bind(id);
        
        query.execute(&self.pool).await?;
        
        Ok(())
    }

    pub async fn delete_budget(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM budgets WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // CSV导入相关
    pub async fn create_transactions_batch(&self, transactions: &[NewTransaction]) -> Result<Vec<i64>> {
        let mut tx = self.pool.begin().await?;
        let mut ids = Vec::new();
        
        for transaction in transactions {
            let result = sqlx::query(
                "INSERT INTO transactions (date, type, amount, category_id, budget_id, description, note, created_at, updated_at) 
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&transaction.date)
            .bind(&transaction.r#type)
            .bind(transaction.amount)
            .bind(transaction.category_id)
            .bind(transaction.budget_id)
            .bind(&transaction.description)
            .bind(&transaction.note)
            .bind(Utc::now())
            .bind(Utc::now())
            .execute(&mut *tx)
            .await?;
            
            ids.push(result.last_insert_rowid());
        }
        
        tx.commit().await?;
        Ok(ids)
    }

    // ── 大模型配置相关方法 ──────────────────────────────────────────────

    fn row_to_llm_config(row: &sqlx::sqlite::SqliteRow) -> LLMConfig {
        use sqlx::Row;
        LLMConfig {
            id:          row.get("id"),
            config_name: row.try_get("config_name").unwrap_or_default(),
            provider:    row.try_get("provider").unwrap_or_else(|_| row.try_get("platform").unwrap_or_default()),
            base_url:    row.try_get("base_url").unwrap_or_default(),
            api_key:     row.try_get("api_key").unwrap_or_else(|_| row.try_get("app_key").unwrap_or_default()),
            model:       row.try_get("model").unwrap_or_default(),
            temperature:      row.try_get::<f64, _>("temperature").unwrap_or(0.7),
            max_tokens:       row.try_get::<i64, _>("max_tokens").unwrap_or(2048),
            enable_thinking:  row.try_get::<i64, _>("enable_thinking").unwrap_or(0) != 0,
            is_active:        row.try_get::<i64, _>("is_active").unwrap_or(0) != 0,
            created_at:  row.try_get::<String, _>("created_at").unwrap_or_default(),
            updated_at:  row.try_get::<String, _>("updated_at").unwrap_or_default(),
        }
    }

    /// 获取所有 LLM 配置（按创建时间倒序）
    pub async fn get_llm_configs(&self) -> Result<Vec<LLMConfig>> {
        let rows = sqlx::query(
            "SELECT * FROM llm_configs ORDER BY is_active DESC, created_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(Self::row_to_llm_config).collect())
    }

    /// 获取当前活跃的 LLM 配置
    pub async fn get_active_llm_config(&self) -> Result<Option<LLMConfig>> {
        let row = sqlx::query(
            "SELECT * FROM llm_configs WHERE is_active = 1 ORDER BY updated_at DESC LIMIT 1"
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.as_ref().map(Self::row_to_llm_config))
    }

    /// 保存新配置（不再强制只有一个活跃；新配置默认设为活跃并停用其他）
    pub async fn save_llm_config(&self, config: &NewLLMConfig) -> Result<i64> {
        let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let temperature = config.temperature.unwrap_or(0.7);
        let max_tokens = config.max_tokens.unwrap_or(2048);
        let enable_thinking = config.enable_thinking.unwrap_or(false) as i64;

        // 先停用所有旧配置
        sqlx::query("UPDATE llm_configs SET is_active = 0, updated_at = ?")
            .bind(&now)
            .execute(&self.pool)
            .await?;

        let result = sqlx::query(
            "INSERT INTO llm_configs (config_name, provider, base_url, api_key, model, temperature, max_tokens, enable_thinking, is_active, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, 1, ?, ?)"
        )
        .bind(&config.config_name)
        .bind(&config.provider)
        .bind(&config.base_url)
        .bind(&config.api_key)
        .bind(&config.model)
        .bind(temperature)
        .bind(max_tokens)
        .bind(enable_thinking)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// 更新已有配置
    pub async fn update_llm_config(&self, id: i64, config: &UpdateLLMConfig) -> Result<()> {
        let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        sqlx::query(
            "UPDATE llm_configs SET
                config_name     = COALESCE(?, config_name),
                provider        = COALESCE(?, provider),
                base_url        = COALESCE(?, base_url),
                api_key         = COALESCE(?, api_key),
                model           = COALESCE(?, model),
                temperature     = COALESCE(?, temperature),
                max_tokens      = COALESCE(?, max_tokens),
                enable_thinking = COALESCE(?, enable_thinking),
                is_active       = COALESCE(?, is_active),
                updated_at      = ?
             WHERE id = ?"
        )
        .bind(&config.config_name)
        .bind(&config.provider)
        .bind(&config.base_url)
        .bind(&config.api_key)
        .bind(&config.model)
        .bind(config.temperature)
        .bind(config.max_tokens)
        .bind(config.enable_thinking.map(|v| if v { 1i64 } else { 0i64 }))
        .bind(config.is_active.map(|v| if v { 1i64 } else { 0i64 }))
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 将指定配置设为活跃，同时停用其他所有配置
    pub async fn set_active_llm_config(&self, id: i64) -> Result<()> {
        let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        sqlx::query("UPDATE llm_configs SET is_active = 0, updated_at = ?")
            .bind(&now)
            .execute(&self.pool)
            .await?;

        sqlx::query("UPDATE llm_configs SET is_active = 1, updated_at = ? WHERE id = ?")
            .bind(&now)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn delete_llm_config(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM llm_configs WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_llm_config_by_id(&self, id: i64) -> Result<Option<LLMConfig>> {
        let row = sqlx::query("SELECT * FROM llm_configs WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.as_ref().map(Self::row_to_llm_config))
    }

    // ── 智能分析会话相关方法 ──────────────────────────────────────────────

    pub async fn get_analysis_sessions(&self) -> Result<Vec<AnalysisSession>> {
        let rows = sqlx::query(
            "SELECT id, title, config_id, created_at, updated_at FROM analysis_sessions ORDER BY updated_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(|row| {
            use sqlx::Row;
            AnalysisSession {
                id: row.get("id"),
                title: row.get("title"),
                config_id: row.get("config_id"),
                created_at: row.try_get::<String, _>("created_at").unwrap_or_default(),
                updated_at: row.try_get::<String, _>("updated_at").unwrap_or_default(),
            }
        }).collect())
    }

    pub async fn get_analysis_messages(&self, session_id: &str) -> Result<Vec<AnalysisMessageRecord>> {
        let rows = sqlx::query(
            "SELECT id, session_id, role, content, created_at FROM analysis_messages WHERE session_id = ? ORDER BY id ASC"
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(|row| {
            use sqlx::Row;
            AnalysisMessageRecord {
                id: row.get("id"),
                session_id: row.get("session_id"),
                role: row.get("role"),
                content: row.get("content"),
                created_at: row.try_get::<String, _>("created_at").unwrap_or_default(),
            }
        }).collect())
    }

    /// 获取最近 N 条消息（用于上下文构建），按 id 正序
    pub async fn get_recent_analysis_messages(&self, session_id: &str, limit: i64) -> Result<Vec<AnalysisMessageRecord>> {
        let rows = sqlx::query(
            "SELECT * FROM (SELECT id, session_id, role, content, created_at FROM analysis_messages WHERE session_id = ? ORDER BY id DESC LIMIT ?) sub ORDER BY id ASC"
        )
        .bind(session_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(|row| {
            use sqlx::Row;
            AnalysisMessageRecord {
                id: row.get("id"),
                session_id: row.get("session_id"),
                role: row.get("role"),
                content: row.get("content"),
                created_at: row.try_get::<String, _>("created_at").unwrap_or_default(),
            }
        }).collect())
    }

    /// 确保会话存在（首次调用时创建，已存在则忽略）
    pub async fn ensure_analysis_session(&self, id: &str, title: &str, config_id: Option<i64>) -> Result<()> {
        let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        sqlx::query(
            "INSERT OR IGNORE INTO analysis_sessions (id, title, config_id, created_at, updated_at) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(id)
        .bind(title)
        .bind(config_id)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn touch_analysis_session(&self, id: &str) -> Result<()> {
        let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        sqlx::query("UPDATE analysis_sessions SET updated_at = ? WHERE id = ?")
            .bind(&now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn save_analysis_message(&self, session_id: &str, role: &str, content: &str) -> Result<i64> {
        let result = sqlx::query(
            "INSERT INTO analysis_messages (session_id, role, content) VALUES (?, ?, ?)"
        )
        .bind(session_id)
        .bind(role)
        .bind(content)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn delete_analysis_session(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM analysis_messages WHERE session_id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM analysis_sessions WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
} 