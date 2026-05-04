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
                source      TEXT NOT NULL DEFAULT 'local',
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
                message_type     TEXT NOT NULL DEFAULT 'text',
                tool_calls_json  TEXT,
                tool_call_id     TEXT,
                tool_name        TEXT,
                source           TEXT NOT NULL DEFAULT 'local',
                FOREIGN KEY (session_id) REFERENCES analysis_sessions(id) ON DELETE CASCADE
            )
        "#).execute(pool).await?;

        // 迁移：为已有 analysis_messages 表补齐新列
        let _ = sqlx::query("ALTER TABLE analysis_messages ADD COLUMN message_type TEXT NOT NULL DEFAULT 'text'").execute(pool).await;
        let _ = sqlx::query("ALTER TABLE analysis_messages ADD COLUMN tool_calls_json TEXT").execute(pool).await;
        let _ = sqlx::query("ALTER TABLE analysis_messages ADD COLUMN tool_call_id TEXT").execute(pool).await;
        let _ = sqlx::query("ALTER TABLE analysis_messages ADD COLUMN tool_name TEXT").execute(pool).await;
        // 迁移：M1（多通道接入）—— 标记会话/消息来源
        let _ = sqlx::query("ALTER TABLE analysis_sessions ADD COLUMN source TEXT NOT NULL DEFAULT 'local'").execute(pool).await;
        let _ = sqlx::query("ALTER TABLE analysis_messages ADD COLUMN source TEXT NOT NULL DEFAULT 'local'").execute(pool).await;

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
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_analysis_sessions_source ON analysis_sessions(source, updated_at DESC)").execute(pool).await?;

        // MCP 服务器配置表
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS mcp_servers (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                name        TEXT    NOT NULL DEFAULT '',
                command     TEXT    NOT NULL DEFAULT '',
                args        TEXT    NOT NULL DEFAULT '[]',
                env         TEXT    NOT NULL DEFAULT '{}',
                enabled     INTEGER NOT NULL DEFAULT 1,
                created_at  DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at  DATETIME DEFAULT CURRENT_TIMESTAMP
            )
        "#).execute(pool).await?;

        // 飞书机器人凭据 / 绑定配置（M2：单行 UPSERT-by-name='default'）
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS feishu_configs (
                id                 INTEGER PRIMARY KEY AUTOINCREMENT,
                name               TEXT    NOT NULL DEFAULT 'default' UNIQUE,
                app_id             TEXT    NOT NULL,
                app_secret         TEXT    NOT NULL,
                domain             TEXT    NOT NULL DEFAULT 'feishu',
                bind_llm_config_id INTEGER,
                bind_role_scope    TEXT    NOT NULL DEFAULT 'analysis',
                enabled            INTEGER NOT NULL DEFAULT 0,
                created_at         TEXT    NOT NULL,
                updated_at         TEXT    NOT NULL
            )
        "#).execute(pool).await?;

        // Memory facts（阶段一）
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS memory_facts (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                fact_type       TEXT NOT NULL
                    CHECK (fact_type IN (
                        'classification_rule','recurring_event',
                        'financial_goal','user_profile','agent_role'
                    )),
                key             TEXT,
                value_json      TEXT NOT NULL,
                source          TEXT NOT NULL DEFAULT 'user'
                    CHECK (source IN ('user','quick_note','analysis','recap','import','preset')),
                confidence      REAL NOT NULL DEFAULT 0.7,
                status          TEXT NOT NULL DEFAULT 'active'
                    CHECK (status IN ('active','provisional','superseded','retired')),
                supersedes_id   INTEGER REFERENCES memory_facts(id),
                origin_session  TEXT,
                origin_message  INTEGER,
                usage_count     INTEGER NOT NULL DEFAULT 0,
                last_used_at    DATETIME,
                created_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
        "#).execute(pool).await?;

        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS memory_facts_history (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                fact_id         INTEGER NOT NULL,
                op              TEXT NOT NULL
                    CHECK (op IN ('insert','update','retire','auto_merge','auto_decay','auto_retire','supersede','preset_apply','undo','rejected')),
                actor           TEXT NOT NULL,
                before_json     TEXT,
                after_json      TEXT,
                origin_session  TEXT,
                created_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
        "#).execute(pool).await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_memory_facts_type ON memory_facts(fact_type, status)").execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_memory_facts_key ON memory_facts(fact_type, key)").execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_memory_facts_status ON memory_facts(status, confidence)").execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_memory_history_fact ON memory_facts_history(fact_id)").execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_memory_history_time ON memory_facts_history(created_at DESC)").execute(pool).await?;

        // Quick note drafts（Analysis 内联确认）
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS quick_note_drafts (
                id                      INTEGER PRIMARY KEY AUTOINCREMENT,
                draft_id                TEXT NOT NULL UNIQUE,
                session_id              TEXT NOT NULL,
                source_message_id       INTEGER,
                status                  TEXT NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending','confirmed','cancelled')),
                confirmation_token_hash TEXT NOT NULL,
                created_by_tool_call_id TEXT,
                confirmed_by_message_id INTEGER,
                created_at              DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at              DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (session_id) REFERENCES analysis_sessions(id) ON DELETE CASCADE
            )
        "#).execute(pool).await?;

        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS quick_note_draft_items (
                id                  INTEGER PRIMARY KEY AUTOINCREMENT,
                draft_id            TEXT NOT NULL,
                date                DATE NOT NULL,
                amount              REAL NOT NULL,
                transaction_type    TEXT NOT NULL CHECK (transaction_type IN ('income','expense')),
                category_id         INTEGER,
                budget_id           INTEGER,
                description         TEXT,
                note                TEXT,
                raw_category_name   TEXT,
                confidence          REAL NOT NULL DEFAULT 0.9,
                sort_order          INTEGER NOT NULL DEFAULT 0,
                created_at          DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at          DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (draft_id) REFERENCES quick_note_drafts(draft_id) ON DELETE CASCADE
            )
        "#).execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_qnd_session_status ON quick_note_drafts(session_id, status, updated_at DESC)").execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_qnd_items_draft ON quick_note_draft_items(draft_id, sort_order)").execute(pool).await?;

        // Token 用量日志（每次 LLM 请求一行）
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS token_usage_logs (
                id                    INTEGER PRIMARY KEY AUTOINCREMENT,
                agent_name            TEXT NOT NULL,
                session_id            TEXT,
                request_id            TEXT NOT NULL,
                round_index           INTEGER NOT NULL DEFAULT 0,
                config_id             INTEGER,
                config_name_snapshot  TEXT,
                provider              TEXT NOT NULL,
                model                 TEXT NOT NULL,
                prompt_tokens         INTEGER NOT NULL DEFAULT 0,
                completion_tokens     INTEGER NOT NULL DEFAULT 0,
                total_tokens          INTEGER NOT NULL DEFAULT 0,
                finish_reason         TEXT,
                duration_ms           INTEGER,
                success               INTEGER NOT NULL DEFAULT 1,
                error_message         TEXT,
                created_at            DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
        "#).execute(pool).await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tul_created ON token_usage_logs(created_at DESC)").execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tul_config  ON token_usage_logs(config_id, created_at DESC)").execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tul_model   ON token_usage_logs(model, created_at DESC)").execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tul_request ON token_usage_logs(request_id, round_index)").execute(pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tul_session ON token_usage_logs(session_id, created_at DESC)").execute(pool).await?;

        // ── analysis_messages 全文检索（FTS5 + 触发器，支持中文 trigram） ──────
        // 设计文档 §6.2：SessionSearch 需要 BM25 排序的会话级历史检索。
        // 使用 trigram tokenizer 让中英文都可被切分；contentless 链接到主表 id。
        let fts_create = sqlx::query(
            r#"
            CREATE VIRTUAL TABLE IF NOT EXISTS analysis_messages_fts USING fts5(
                content,
                session_id UNINDEXED,
                role UNINDEXED,
                created_at UNINDEXED,
                content='analysis_messages',
                content_rowid='id',
                tokenize='trigram case_sensitive 0'
            )
            "#,
        )
        .execute(pool)
        .await;

        let fts_available = match fts_create {
            Ok(_) => true,
            Err(e) => {
                eprintln!("[memory] FTS5 trigram 不可用，会话检索将退化为 LIKE：{}", e);
                false
            }
        };

        if fts_available {
            sqlx::query(
                r#"
                CREATE TRIGGER IF NOT EXISTS analysis_messages_ai AFTER INSERT ON analysis_messages BEGIN
                    INSERT INTO analysis_messages_fts(rowid, content, session_id, role, created_at)
                    VALUES (new.id, new.content, new.session_id, new.role, new.created_at);
                END;
                "#,
            )
            .execute(pool)
            .await?;

            sqlx::query(
                r#"
                CREATE TRIGGER IF NOT EXISTS analysis_messages_ad AFTER DELETE ON analysis_messages BEGIN
                    INSERT INTO analysis_messages_fts(analysis_messages_fts, rowid, content, session_id, role, created_at)
                    VALUES ('delete', old.id, old.content, old.session_id, old.role, old.created_at);
                END;
                "#,
            )
            .execute(pool)
            .await?;

            sqlx::query(
                r#"
                CREATE TRIGGER IF NOT EXISTS analysis_messages_au AFTER UPDATE ON analysis_messages BEGIN
                    INSERT INTO analysis_messages_fts(analysis_messages_fts, rowid, content, session_id, role, created_at)
                    VALUES ('delete', old.id, old.content, old.session_id, old.role, old.created_at);
                    INSERT INTO analysis_messages_fts(rowid, content, session_id, role, created_at)
                    VALUES (new.id, new.content, new.session_id, new.role, new.created_at);
                END;
                "#,
            )
            .execute(pool)
            .await?;

            // 启动回填：FTS 行数为 0 且主表非空时，全量灌一次
            let fts_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM analysis_messages_fts")
                .fetch_one(pool)
                .await
                .unwrap_or(0);
            let main_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM analysis_messages")
                .fetch_one(pool)
                .await
                .unwrap_or(0);
            if fts_count == 0 && main_count > 0 {
                println!("[memory] 回填 analysis_messages_fts ({} 行)", main_count);
                sqlx::query(
                    r#"
                    INSERT INTO analysis_messages_fts(rowid, content, session_id, role, created_at)
                    SELECT id, content, session_id, role, created_at FROM analysis_messages
                    "#,
                )
                .execute(pool)
                .await?;
            }
        }

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
            "SELECT id, title, config_id, source, created_at, updated_at FROM analysis_sessions ORDER BY updated_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(|row| {
            use sqlx::Row;
            AnalysisSession {
                id: row.get("id"),
                title: row.get("title"),
                config_id: row.get("config_id"),
                source: row.try_get::<String, _>("source").unwrap_or_else(|_| "local".to_string()),
                created_at: row.try_get::<String, _>("created_at").unwrap_or_default(),
                updated_at: row.try_get::<String, _>("updated_at").unwrap_or_default(),
            }
        }).collect())
    }

    pub async fn get_analysis_messages(&self, session_id: &str) -> Result<Vec<AnalysisMessageRecord>> {
        let rows = sqlx::query(
            "SELECT id, session_id, role, content, created_at, message_type, tool_calls_json, tool_call_id, tool_name, source FROM analysis_messages WHERE session_id = ? ORDER BY id ASC"
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
                message_type: row.try_get::<String, _>("message_type").unwrap_or_else(|_| "text".to_string()),
                tool_calls_json: row.try_get::<Option<String>, _>("tool_calls_json").unwrap_or(None),
                tool_call_id: row.try_get::<Option<String>, _>("tool_call_id").unwrap_or(None),
                tool_name: row.try_get::<Option<String>, _>("tool_name").unwrap_or(None),
                source: row.try_get::<String, _>("source").unwrap_or_else(|_| "local".to_string()),
            }
        }).collect())
    }

    /// 获取最近 N 条 text 消息（用于 LLM 上下文构建），按 id 正序
    /// 仅返回 message_type='text' 的消息，工具调用步骤不传递给后续 LLM 请求
    pub async fn get_recent_analysis_messages(&self, session_id: &str, limit: i64) -> Result<Vec<AnalysisMessageRecord>> {
        let rows = sqlx::query(
            "SELECT * FROM (SELECT id, session_id, role, content, created_at, message_type, tool_calls_json, tool_call_id, tool_name, source FROM analysis_messages WHERE session_id = ? AND message_type = 'text' ORDER BY id DESC LIMIT ?) sub ORDER BY id ASC"
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
                message_type: row.try_get::<String, _>("message_type").unwrap_or_else(|_| "text".to_string()),
                tool_calls_json: row.try_get::<Option<String>, _>("tool_calls_json").unwrap_or(None),
                tool_call_id: row.try_get::<Option<String>, _>("tool_call_id").unwrap_or(None),
                tool_name: row.try_get::<Option<String>, _>("tool_name").unwrap_or(None),
                source: row.try_get::<String, _>("source").unwrap_or_else(|_| "local".to_string()),
            }
        }).collect())
    }

    /// 确保会话存在（首次调用时创建，已存在则忽略）
    /// `source` 标记会话来源：'local'（桌面 UI）/ 'feishu'（飞书入站）/ 后续可扩展
    pub async fn ensure_analysis_session(&self, id: &str, title: &str, config_id: Option<i64>, source: &str) -> Result<()> {
        let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        sqlx::query(
            "INSERT OR IGNORE INTO analysis_sessions (id, title, config_id, source, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)"
        )
        .bind(id)
        .bind(title)
        .bind(config_id)
        .bind(source)
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

    /// 保存一条 text 消息（默认 source='local'）—— 兼容旧调用方
    pub async fn save_analysis_message(&self, session_id: &str, role: &str, content: &str) -> Result<i64> {
        self.save_analysis_message_ext(session_id, role, content, "text", None, None, None, "local").await
    }

    pub async fn save_analysis_message_ext(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
        message_type: &str,
        tool_calls_json: Option<&str>,
        tool_call_id: Option<&str>,
        tool_name: Option<&str>,
        source: &str,
    ) -> Result<i64> {
        let result = sqlx::query(
            "INSERT INTO analysis_messages (session_id, role, content, message_type, tool_calls_json, tool_call_id, tool_name, source) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(session_id)
        .bind(role)
        .bind(content)
        .bind(message_type)
        .bind(tool_calls_json)
        .bind(tool_call_id)
        .bind(tool_name)
        .bind(source)
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

    pub async fn list_pending_drafts_by_session(&self, session_id: &str) -> Result<Vec<QuickNoteDraft>> {
        let draft_rows = sqlx::query(
            "SELECT * FROM quick_note_drafts WHERE session_id = ? AND status = 'pending' ORDER BY updated_at DESC"
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;

        let mut drafts = Vec::new();
        for row in draft_rows {
            let draft_id: String = row.get("draft_id");
            let item_rows = sqlx::query(
                "SELECT * FROM quick_note_draft_items WHERE draft_id = ? ORDER BY sort_order ASC, id ASC"
            )
            .bind(&draft_id)
            .fetch_all(&self.pool)
            .await?;

            let items = item_rows.into_iter().map(|it| QuickNoteDraftItem {
                id: it.get("id"),
                draft_id: it.get("draft_id"),
                date: it.get("date"),
                amount: it.get("amount"),
                transaction_type: it.get("transaction_type"),
                category_id: it.try_get("category_id").ok(),
                budget_id: it.try_get("budget_id").ok(),
                description: it.try_get("description").ok(),
                note: it.try_get("note").ok(),
                raw_category_name: it.try_get("raw_category_name").ok(),
                confidence: it.try_get("confidence").unwrap_or(0.9),
                sort_order: it.try_get("sort_order").unwrap_or(0),
                created_at: it.get("created_at"),
                updated_at: it.get("updated_at"),
            }).collect();

            drafts.push(QuickNoteDraft {
                id: row.get("id"),
                draft_id,
                session_id: row.get("session_id"),
                source_message_id: row.try_get("source_message_id").ok(),
                status: row.get("status"),
                confirmation_token: None,
                created_by_tool_call_id: row.try_get("created_by_tool_call_id").ok(),
                confirmed_by_message_id: row.try_get("confirmed_by_message_id").ok(),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                items,
            });
        }

        Ok(drafts)
    }

    pub async fn create_quick_note_draft(&self, request: &CreateQuickNoteDraftRequest) -> Result<QuickNoteDraft> {
        use sha2::{Digest, Sha256};
        let token_raw = uuid::Uuid::new_v4().to_string();
        let token_hash = format!("{:x}", Sha256::digest(token_raw.as_bytes()));
        let draft_id = uuid::Uuid::new_v4().to_string();

        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO quick_note_drafts (
                draft_id, session_id, source_message_id, status,
                confirmation_token_hash, created_by_tool_call_id
             ) VALUES (?, ?, ?, 'pending', ?, ?)"
        )
        .bind(&draft_id)
        .bind(&request.session_id)
        .bind(request.source_message_id)
        .bind(token_hash)
        .bind(&request.created_by_tool_call_id)
        .execute(&mut *tx)
        .await?;

        for (idx, item) in request.items.iter().enumerate() {
            sqlx::query(
                "INSERT INTO quick_note_draft_items (
                    draft_id, date, amount, transaction_type, category_id, budget_id,
                    description, note, raw_category_name, confidence, sort_order
                 ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&draft_id)
            .bind(&item.date)
            .bind(item.amount)
            .bind(&item.transaction_type)
            .bind(item.category_id)
            .bind(item.budget_id)
            .bind(&item.description)
            .bind(&item.description)
            .bind(&item.description)
            .bind(0.9f64)
            .bind(idx as i64)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        let mut drafts = self.list_pending_drafts_by_session(&request.session_id).await?;
        let mut draft = drafts
            .drain(..)
            .find(|d| d.draft_id == draft_id)
            .ok_or_else(|| anyhow::anyhow!("草稿创建后读取失败"))?;
        draft.confirmation_token = Some(token_raw);
        Ok(draft)
    }

    pub async fn confirm_quick_note_draft(&self, request: &ConfirmQuickNoteDraftRequest) -> Result<SaveTransactionsResult> {
        use sha2::{Digest, Sha256};
        let row = sqlx::query(
            "SELECT status, confirmation_token_hash FROM quick_note_drafts WHERE draft_id = ?"
        )
        .bind(&request.draft_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("草稿不存在"))?;

        let status: String = row.get("status");
        if status == "confirmed" {
            return Ok(SaveTransactionsResult {
                success: true,
                message: "草稿已确认，无需重复保存".to_string(),
                saved_count: 0,
                failed_count: 0,
            });
        }
        if status != "pending" {
            return Err(anyhow::anyhow!("草稿状态不可确认"));
        }

        let token_hash: String = row.get("confirmation_token_hash");
        let req_hash = format!("{:x}", Sha256::digest(request.confirmation_token.as_bytes()));
        if token_hash != req_hash {
            return Err(anyhow::anyhow!("确认令牌无效"));
        }

        let mut tx = self.pool.begin().await?;
        let mut saved_count = 0usize;
        let mut failed_count = 0usize;
        for item in &request.items {
            let date = NaiveDate::parse_from_str(&item.date, "%Y-%m-%d")
                .map_err(|_| anyhow::anyhow!("草稿存在无效日期: {}", item.date))?;
            let new_tx = NewTransaction {
                date,
                r#type: item.transaction_type.clone(),
                amount: item.amount,
                category_id: item.category_id,
                budget_id: item.budget_id,
                description: Some(item.description.clone()),
                note: Some(item.description.clone()),
            };
            let res = sqlx::query(
                "INSERT INTO transactions (date, type, amount, category_id, budget_id, description, note, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(new_tx.date)
            .bind(new_tx.r#type)
            .bind(new_tx.amount)
            .bind(new_tx.category_id)
            .bind(new_tx.budget_id)
            .bind(new_tx.description)
            .bind(new_tx.note)
            .bind(Utc::now())
            .bind(Utc::now())
            .execute(&mut *tx)
            .await;
            if res.is_ok() {
                saved_count += 1;
            } else {
                failed_count += 1;
            }
        }

        if failed_count == 0 && saved_count > 0 {
            sqlx::query(
                "UPDATE quick_note_drafts SET status='confirmed', updated_at=CURRENT_TIMESTAMP WHERE draft_id = ?"
            )
            .bind(&request.draft_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;

        Ok(SaveTransactionsResult {
            success: saved_count > 0 && failed_count == 0,
            message: if failed_count == 0 {
                format!("已保存 {} 条，草稿已确认", saved_count)
            } else {
                format!("已保存 {} 条，失败 {} 条，草稿保持待确认", saved_count, failed_count)
            },
            saved_count,
            failed_count,
        })
    }

    pub async fn cancel_quick_note_draft(&self, draft_id: &str) -> Result<()> {
        sqlx::query(
            "UPDATE quick_note_drafts SET status='cancelled', updated_at=CURRENT_TIMESTAMP WHERE draft_id = ? AND status='pending'"
        )
        .bind(draft_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn refresh_quick_note_draft_token(&self, draft_id: &str) -> Result<String> {
        use sha2::{Digest, Sha256};
        let token = uuid::Uuid::new_v4().to_string();
        let token_hash = format!("{:x}", Sha256::digest(token.as_bytes()));
        let result = sqlx::query(
            "UPDATE quick_note_drafts
             SET confirmation_token_hash = ?, updated_at = CURRENT_TIMESTAMP
             WHERE draft_id = ? AND status = 'pending'",
        )
        .bind(token_hash)
        .bind(draft_id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(anyhow::anyhow!("草稿不存在或已不可确认"));
        }
        Ok(token)
    }

    pub async fn get_quick_note_draft_token(&self, draft_id: &str) -> Result<String> {
        self.refresh_quick_note_draft_token(draft_id).await
    }

    // ── MCP 服务器配置相关方法 ──────────────────────────────────────────

    pub async fn get_mcp_servers(&self) -> Result<Vec<crate::mcp::McpServerConfig>> {
        let rows = sqlx::query(
            "SELECT id, name, command, args, env, enabled, created_at, updated_at FROM mcp_servers ORDER BY created_at ASC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(|row| {
            use sqlx::Row;
            crate::mcp::McpServerConfig {
                id: row.get("id"),
                name: row.try_get("name").unwrap_or_default(),
                command: row.try_get("command").unwrap_or_default(),
                args: row.try_get("args").unwrap_or_else(|_| "[]".to_string()),
                env: row.try_get("env").unwrap_or_else(|_| "{}".to_string()),
                enabled: row.try_get::<i64, _>("enabled").unwrap_or(1) != 0,
                created_at: row.try_get::<String, _>("created_at").unwrap_or_default(),
                updated_at: row.try_get::<String, _>("updated_at").unwrap_or_default(),
            }
        }).collect())
    }

    pub async fn create_mcp_server(&self, config: &crate::mcp::NewMcpServerConfig) -> Result<i64> {
        let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let result = sqlx::query(
            "INSERT INTO mcp_servers (name, command, args, env, enabled, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&config.name)
        .bind(&config.command)
        .bind(&config.args)
        .bind(&config.env)
        .bind(config.enabled as i64)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn update_mcp_server(&self, id: i64, config: &crate::mcp::UpdateMcpServerConfig) -> Result<()> {
        let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        sqlx::query(
            "UPDATE mcp_servers SET
                name    = COALESCE(?, name),
                command = COALESCE(?, command),
                args    = COALESCE(?, args),
                env     = COALESCE(?, env),
                enabled = COALESCE(?, enabled),
                updated_at = ?
             WHERE id = ?"
        )
        .bind(&config.name)
        .bind(&config.command)
        .bind(&config.args)
        .bind(&config.env)
        .bind(config.enabled.map(|v| if v { 1i64 } else { 0i64 }))
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_mcp_server(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM mcp_servers WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_mcp_server_by_id(&self, id: i64) -> Result<Option<crate::mcp::McpServerConfig>> {
        let row = sqlx::query("SELECT id, name, command, args, env, enabled, created_at, updated_at FROM mcp_servers WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.as_ref().map(|row| {
            use sqlx::Row;
            crate::mcp::McpServerConfig {
                id: row.get("id"),
                name: row.try_get("name").unwrap_or_default(),
                command: row.try_get("command").unwrap_or_default(),
                args: row.try_get("args").unwrap_or_else(|_| "[]".to_string()),
                env: row.try_get("env").unwrap_or_else(|_| "{}".to_string()),
                enabled: row.try_get::<i64, _>("enabled").unwrap_or(1) != 0,
                created_at: row.try_get::<String, _>("created_at").unwrap_or_default(),
                updated_at: row.try_get::<String, _>("updated_at").unwrap_or_default(),
            }
        }))
    }

    // ── 飞书机器人配置（M2：单行 UPSERT-by-name='default'）─────────────

    /// 读取唯一一条 `name='default'` 的飞书配置；无则返回 `None`。
    pub async fn get_feishu_config(&self) -> Result<Option<crate::feishu::FeishuConfig>> {
        let row = sqlx::query(
            "SELECT id, name, app_id, app_secret, domain, bind_llm_config_id, bind_role_scope, enabled, created_at, updated_at \
             FROM feishu_configs WHERE name = 'default' LIMIT 1"
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.as_ref().map(|row| crate::feishu::FeishuConfig {
            id: row.get("id"),
            name: row.try_get("name").unwrap_or_else(|_| "default".to_string()),
            app_id: row.try_get("app_id").unwrap_or_default(),
            app_secret: row.try_get("app_secret").unwrap_or_default(),
            domain: row.try_get("domain").unwrap_or_else(|_| "feishu".to_string()),
            bind_llm_config_id: row.try_get("bind_llm_config_id").ok(),
            bind_role_scope: row.try_get("bind_role_scope").unwrap_or_else(|_| "analysis".to_string()),
            enabled: row.try_get::<i64, _>("enabled").unwrap_or(0) != 0,
            created_at: row.try_get::<String, _>("created_at").unwrap_or_default(),
            updated_at: row.try_get::<String, _>("updated_at").unwrap_or_default(),
        }))
    }

    /// UPSERT 单行飞书配置（`name='default'` 唯一）。返回行 id。
    ///
    /// SQLite UPSERT：`INSERT … ON CONFLICT(name) DO UPDATE SET …`。
    pub async fn upsert_feishu_config(
        &self,
        input: &crate::feishu::FeishuConfigInput,
    ) -> Result<i64> {
        let now = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        sqlx::query(
            "INSERT INTO feishu_configs \
                (name, app_id, app_secret, domain, bind_llm_config_id, bind_role_scope, enabled, created_at, updated_at) \
             VALUES ('default', ?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(name) DO UPDATE SET \
                app_id             = excluded.app_id, \
                app_secret         = excluded.app_secret, \
                domain             = excluded.domain, \
                bind_llm_config_id = excluded.bind_llm_config_id, \
                bind_role_scope    = excluded.bind_role_scope, \
                enabled            = excluded.enabled, \
                updated_at         = excluded.updated_at"
        )
        .bind(&input.app_id)
        .bind(&input.app_secret)
        .bind(&input.domain)
        .bind(input.bind_llm_config_id)
        .bind(&input.bind_role_scope)
        .bind(input.enabled as i64)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        // UPSERT 后查回 id（SQLite `last_insert_rowid` 在 UPDATE 路径下可能为 0，需另查）
        let row = sqlx::query("SELECT id FROM feishu_configs WHERE name = 'default' LIMIT 1")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get::<i64, _>("id"))
    }
}