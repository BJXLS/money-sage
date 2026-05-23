use super::store::MemoryStore;
use anyhow::Result;
use sha2::{Sha256, Digest};
use sqlx::{Row, SqlitePool};

/// Memory 文件 → SQLite FTS5 索引同步器
///
/// 增量同步策略：
/// 1. 扫描所有 .md 文件，提取 `&` 开头的记忆行
/// 2. 计算每行的 SHA-256 hash
/// 3. 与数据库中的 `line_hash` 比对，新增/删除差异
/// 4. 更新 `memory_index` 表（FTS5 自动同步）
#[derive(Clone)]
pub struct MemoryIndexer {
    pool: SqlitePool,
    store: MemoryStore,
}

#[derive(Debug, Default)]
pub struct SyncReport {
    pub scanned_files: usize,
    pub scanned_entries: usize,
    pub added: usize,
    pub removed: usize,
    pub unchanged: usize,
}

impl MemoryIndexer {
    pub fn new(pool: SqlitePool, store: MemoryStore) -> Self {
        Self { pool, store }
    }

    /// 全量同步：扫描所有 md 文件，与数据库比对后增量更新
    pub async fn sync_all(&self) -> Result<SyncReport> {
        let files = self.store.list_files_recursive()?;
        let mut report = SyncReport {
            scanned_files: files.len(),
            ..Default::default()
        };

        // 收集当前文件中的所有条目 hash
        let mut current_hashes = std::collections::HashSet::new();
        let mut to_insert = Vec::new();

        for (rel_path, _char_count) in &files {
            if rel_path == "MEMORY.md" {
                continue; // MEMORY.md 是快照索引，不入索引
            }
            if rel_path.starts_with("meta/") {
                continue; // meta/ 是系统规范，不入索引
            }

            let content = self.store.read_file(rel_path)?;
            let mut current_heading = String::new();

            for line in content.lines() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("## ") {
                    current_heading = trimmed.trim_start_matches("## ").trim().to_string();
                    continue;
                }
                if trimmed.starts_with('&') {
                    report.scanned_entries += 1;
                    let hash = hash_line(trimmed);
                    current_hashes.insert(hash.clone());

                    // 解析 & 行
                    if let Some(parsed) = parse_entry_line(trimmed) {
                        to_insert.push((
                            rel_path.clone(),
                            current_heading.clone(),
                            hash,
                            parsed.full_text,
                            parsed.timestamp,
                            parsed.source,
                        ));
                    }
                }
            }
        }

        // 获取数据库中现有的所有 hash
        let existing_hashes: Vec<String> = sqlx::query_scalar(
            "SELECT line_hash FROM memory_index"
        )
        .fetch_all(&self.pool)
        .await?;

        let existing_set: std::collections::HashSet<String> =
            existing_hashes.into_iter().collect();

        // 需要删除的：在数据库中但不在当前文件中的
        let to_delete: Vec<String> = existing_set
            .difference(&current_hashes)
            .cloned()
            .collect();

        // 需要插入的：在当前文件中但不在数据库中的
        let to_insert_filtered: Vec<_> = to_insert
            .into_iter()
            .filter(|(_, _, hash, _, _, _)| !existing_set.contains(hash))
            .collect();

        report.added = to_insert_filtered.len();
        report.removed = to_delete.len();
        report.unchanged = current_hashes.len() - report.added;

        // 执行删除
        for hash in &to_delete {
            let _ = sqlx::query("DELETE FROM memory_index WHERE line_hash = ?")
                .bind(hash)
                .execute(&self.pool)
                .await;
        }

        // 执行插入（FTS5 自动同步）
        for (file_path, heading, hash, full_text, timestamp, source) in to_insert_filtered {
            let _ = sqlx::query(r#"
                INSERT INTO memory_index (file_path, heading, line_hash, full_text, timestamp, source)
                VALUES (?, ?, ?, ?, ?, ?)
            "#)
            .bind(file_path)
            .bind(heading)
            .bind(hash)
            .bind(full_text)
            .bind(timestamp)
            .bind(source)
            .execute(&self.pool)
            .await;
        }

        Ok(report)
    }

    /// 搜索：FTS5 + BM25 排序，分组返回
    pub async fn search(
        &self,
        query: &str,
        top_k_per_category: i64,
    ) -> Result<GroupedSearchResult> {
        // 清理查询：FTS5 特殊字符转义
        let safe_query = sanitize_fts5_query(query);

        // BM25 排序搜索
        let rows = sqlx::query(r#"
            SELECT file_path, heading, full_text, timestamp, source,
                   rank AS score
            FROM memory_index_fts
            WHERE memory_index_fts MATCH ?
            ORDER BY rank
            LIMIT ?
        "#)
        .bind(&safe_query)
        .bind(top_k_per_category * 10) // 多取一些，后续按类型过滤
        .fetch_all(&self.pool)
        .await?;

        let mut factual = Vec::new();
        let mut episodic = Vec::new();
        let mut procedural = Vec::new();

        for row in rows {
            let file_path: String = row.get("file_path");
            let heading: Option<String> = row.try_get("heading").ok();
            let full_text: String = row.get("full_text");
            let timestamp: Option<String> = row.try_get("timestamp").ok();
            let source: Option<String> = row.try_get("source").ok();
            let score: f64 = row.get("score");

            let item = SearchItem {
                file_path: file_path.clone(),
                heading,
                full_text,
                timestamp,
                source,
                score,
            };

            if file_path.starts_with("factual/") && factual.len() < top_k_per_category as usize {
                factual.push(item);
            } else if file_path.starts_with("episodic/") && episodic.len() < top_k_per_category as usize {
                episodic.push(item);
            } else if file_path.starts_with("procedural/") && procedural.len() < top_k_per_category as usize {
                procedural.push(item);
            }
        }

        Ok(GroupedSearchResult {
            factual,
            episodic,
            procedural,
        })
    }
}

#[derive(Debug, Clone)]
pub struct SearchItem {
    pub file_path: String,
    pub heading: Option<String>,
    pub full_text: String,
    pub timestamp: Option<String>,
    pub source: Option<String>,
    pub score: f64,
}

#[derive(Debug, Default)]
pub struct GroupedSearchResult {
    pub factual: Vec<SearchItem>,
    pub episodic: Vec<SearchItem>,
    pub procedural: Vec<SearchItem>,
}

fn hash_line(line: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(line.as_bytes());
    format!("{:x}", hasher.finalize())[..16].to_string()
}

struct ParsedEntry {
    full_text: String,
    timestamp: Option<String>,
    source: Option<String>,
}

fn parse_entry_line(line: &str) -> Option<ParsedEntry> {
    // 格式: & <timestamp> | <source> | <content>
    let without_prefix = line.trim_start().trim_start_matches('&').trim_start();
    let parts: Vec<&str> = without_prefix.splitn(3, " | ").collect();
    if parts.len() < 3 {
        return Some(ParsedEntry {
            full_text: without_prefix.to_string(),
            timestamp: None,
            source: None,
        });
    }
    Some(ParsedEntry {
        timestamp: Some(parts[0].trim().to_string()),
        source: Some(parts[1].trim().to_string()),
        full_text: parts[2].trim().to_string(),
    })
}

/// 转义 FTS5 查询中的特殊字符
fn sanitize_fts5_query(query: &str) -> String {
    // FTS5 需要转义: " * ( ) 以及引号
    let mut result = String::new();
    for ch in query.chars() {
        match ch {
            '"' | '*' | '(' | ')' => {
                result.push('"');
                result.push(ch);
                result.push('"');
            }
            _ => result.push(ch),
        }
    }
    if result.is_empty() {
        result.push_str("*");
    }
    result
}
