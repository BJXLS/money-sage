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
                name TEXT NOT NULL UNIQUE,
                icon TEXT,
                color TEXT,
                type TEXT NOT NULL CHECK (type IN ('income', 'expense')),
                is_system INTEGER DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        "#).execute(pool).await?;

        // 创建交易记录表
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS transactions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date DATE NOT NULL,
                type TEXT NOT NULL CHECK (type IN ('income', 'expense')),
                amount DECIMAL(10, 2) NOT NULL,
                category_id INTEGER NOT NULL,
                description TEXT,
                note TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (category_id) REFERENCES categories(id)
            )
        "#).execute(pool).await?;

        // 创建预算表
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS budgets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                category_id INTEGER NOT NULL,
                amount DECIMAL(10, 2) NOT NULL,
                period_type TEXT NOT NULL DEFAULT 'monthly' CHECK (period_type IN ('weekly', 'monthly', 'yearly')),
                start_date DATE NOT NULL,
                end_date DATE,
                is_active INTEGER DEFAULT 1,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (category_id) REFERENCES categories(id)
            )
        "#).execute(pool).await?;

        // 创建索引
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_transactions_date ON transactions(date)").execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_transactions_category ON transactions(category_id)").execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_transactions_type ON transactions(type)").execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_budgets_category ON budgets(category_id)").execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_budgets_period ON budgets(start_date, end_date)").execute(pool).await?;

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
                    INSERT OR IGNORE INTO categories (name, icon, color, type, is_system, created_at, updated_at) 
                    VALUES (?, ?, ?, ?, 1, ?, ?)
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
            "INSERT INTO categories (name, icon, color, type, created_at, updated_at) 
             VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(&category.name)
        .bind(&category.icon)
        .bind(&category.color)
        .bind(&category.r#type)
        .bind(Utc::now())
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;
        
        Ok(result.last_insert_rowid())
    }

    pub async fn delete_category(&self, id: i64) -> Result<()> {
        // 检查是否为系统分类
        let category = sqlx::query("SELECT is_system FROM categories WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
            
        if let Some(row) = category {
            let is_system: bool = row.get("is_system");
            if is_system {
                return Err(anyhow::anyhow!("不能删除系统分类"));
            }
        }
        
        // 检查是否有关联的交易记录
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM transactions WHERE category_id = ?")
            .bind(id)
            .fetch_one(&self.pool)
            .await?;
            
        if count > 0 {
            return Err(anyhow::anyhow!("该分类下有交易记录，无法删除"));
        }
        
        sqlx::query("DELETE FROM categories WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
            
        Ok(())
    }

    // 交易记录相关方法
    pub async fn get_transactions(&self, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<TransactionWithCategory>> {
        let limit = limit.unwrap_or(50);
        let offset = offset.unwrap_or(0);
        
        let transactions = sqlx::query_as::<_, TransactionWithCategory>(
            r#"
            SELECT 
                t.id, t.date, t.type, t.amount, t.category_id, t.description, t.note, 
                t.created_at, t.updated_at,
                c.name as category_name, c.icon as category_icon, c.color as category_color
            FROM transactions t
            JOIN categories c ON t.category_id = c.id
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
                t.id, t.date, t.type, t.amount, t.category_id, t.description, t.note, 
                t.created_at, t.updated_at,
                c.name as category_name, c.icon as category_icon, c.color as category_color
            FROM transactions t
            JOIN categories c ON t.category_id = c.id
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
            "INSERT INTO transactions (date, type, amount, category_id, description, note, created_at, updated_at) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&transaction.date)
        .bind(&transaction.r#type)
        .bind(transaction.amount)
        .bind(transaction.category_id)
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
                COALESCE(SUM(CASE WHEN type = 'income' THEN amount ELSE 0 END), 0) as income,
                COALESCE(SUM(CASE WHEN type = 'expense' THEN amount ELSE 0 END), 0) as expense
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
            "SELECT COALESCE(SUM(amount), 0) FROM transactions WHERE date BETWEEN ? AND ? AND type = ?"
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
                COALESCE(SUM(t.amount), 0) as amount
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
                b.id, b.category_id, b.amount, b.period_type, b.start_date, b.end_date, b.is_active,
                b.created_at, b.updated_at,
                c.name as category_name, c.icon as category_icon, c.color as category_color,
                COALESCE(SUM(t.amount), 0) as spent
            FROM budgets b
            JOIN categories c ON b.category_id = c.id
            LEFT JOIN transactions t ON b.category_id = t.category_id 
                AND t.type = 'expense'
                AND t.date BETWEEN b.start_date AND COALESCE(b.end_date, date('now'))
            WHERE b.is_active = 1
            GROUP BY b.id, b.category_id, b.amount, b.period_type, b.start_date, b.end_date, 
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
                    category_id: row.get("category_id"),
                    amount,
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
            "INSERT INTO budgets (category_id, amount, period_type, start_date, end_date, created_at, updated_at) 
             VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(budget.category_id)
        .bind(budget.amount)
        .bind(&budget.period_type)
        .bind(&budget.start_date)
        .bind(&budget.end_date)
        .bind(Utc::now())
        .bind(Utc::now())
        .execute(&self.pool)
        .await?;
        
        Ok(result.last_insert_rowid())
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
                "INSERT INTO transactions (date, type, amount, category_id, description, note, created_at, updated_at) 
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&transaction.date)
            .bind(&transaction.r#type)
            .bind(transaction.amount)
            .bind(transaction.category_id)
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
} 