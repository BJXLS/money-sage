use anyhow::Result;
use sqlx::{Row, SqlitePool};

/// 记忆变更日志记录器
///
/// 记录每次对 memory/ 文件的写入操作，支持一键撤销。
#[derive(Clone)]
pub struct Changelog {
    pool: SqlitePool,
}

#[derive(Debug, Clone)]
pub struct ChangelogEntry {
    pub id: i64,
    pub file_path: String,
    pub operation: String,
    pub old_content: Option<String>,
    pub new_content: String,
    pub source: String,
    pub session_id: Option<String>,
    pub created_at: String,
}

impl Changelog {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 记录一次写入操作
    pub async fn log_write(
        &self,
        file_path: &str,
        old_content: Option<String>,
        new_content: &str,
        source: &str,
        session_id: Option<&str>,
    ) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO memory_changelog (file_path, operation, old_content, new_content, source, session_id)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(file_path)
        .bind("write")
        .bind(old_content.as_deref())
        .bind(new_content)
        .bind(source)
        .bind(session_id)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// 记录一次编辑操作
    pub async fn log_edit(
        &self,
        file_path: &str,
        old_content: &str,
        new_content: &str,
        source: &str,
        session_id: Option<&str>,
    ) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO memory_changelog (file_path, operation, old_content, new_content, source, session_id)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(file_path)
        .bind("edit")
        .bind(old_content)
        .bind(new_content)
        .bind(source)
        .bind(session_id)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// 列出最近变更
    pub async fn list_recent(&self, limit: i64) -> Result<Vec<ChangelogEntry>> {
        let rows = sqlx::query(
            r#"
            SELECT id, file_path, operation, old_content, new_content, source, session_id, created_at
            FROM memory_changelog
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(ChangelogEntry {
                id: row.get("id"),
                file_path: row.get("file_path"),
                operation: row.get("operation"),
                old_content: row.try_get("old_content").ok(),
                new_content: row.get("new_content"),
                source: row.get("source"),
                session_id: row.try_get("session_id").ok(),
                created_at: row.get("created_at"),
            });
        }

        Ok(entries)
    }

    /// 撤销指定变更
    pub async fn undo(&self, entry_id: i64, store: &super::MemoryStore) -> Result<String> {
        let row = sqlx::query(
            r#"
            SELECT file_path, operation, old_content, new_content
            FROM memory_changelog
            WHERE id = ?
            "#,
        )
        .bind(entry_id)
        .fetch_optional(&self.pool)
        .await?;

        let (file_path, operation, old_content, _new_content) = match row {
            Some(r) => {
                let fp: String = r.get("file_path");
                let op: String = r.get("operation");
                let old: Option<String> = r.try_get("old_content").ok();
                let new: String = r.get("new_content");
                (fp, op, old, new)
            }
            None => return Err(anyhow::anyhow!("找不到变更记录: {}", entry_id)),
        };

        match operation.as_str() {
            "write" | "edit" => {
                if let Some(old) = old_content {
                    store.write_file(&file_path, &old)?;
                    // 标记为已撤销
                    sqlx::query("UPDATE memory_changelog SET undone = 1 WHERE id = ?")
                        .bind(entry_id)
                        .execute(&self.pool)
                        .await?;
                    Ok(format!("已撤销变更 #{}，文件 {} 已恢复", entry_id, file_path))
                } else {
                    // 如果是新建文件且没有旧内容，则删除文件
                    let path = store.memory_dir().join(&file_path);
                    if path.exists() {
                        std::fs::remove_file(&path)?;
                    }
                    sqlx::query("UPDATE memory_changelog SET undone = 1 WHERE id = ?")
                        .bind(entry_id)
                        .execute(&self.pool)
                        .await?;
                    Ok(format!("已撤销变更 #{}，新建文件 {} 已删除", entry_id, file_path))
                }
            }
            _ => Err(anyhow::anyhow!("不支持的操作类型: {}", operation)),
        }
    }
}
