use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub top_k_facts: i64,
    pub top_k_sessions: i64,
    pub time_range_days: i64,
    pub exclude_session: Option<String>,
    pub include_facts: bool,
    pub include_sessions: bool,
}

impl SearchQuery {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            top_k_facts: 3,
            top_k_sessions: 3,
            time_range_days: 365,
            exclude_session: None,
            include_facts: true,
            include_sessions: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactHit {
    pub id: i64,
    pub fact_type: String,
    pub key: Option<String>,
    pub value_excerpt: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionHit {
    pub session_id: String,
    pub title: String,
    pub snippet: String,
    pub score: f64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub facts: Vec<FactHit>,
    pub sessions: Vec<SessionHit>,
}

pub struct MemorySearchBackend {
    pool: SqlitePool,
}

impl MemorySearchBackend {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn search(&self, q: SearchQuery) -> Result<SearchResult> {
        let trimmed = q.query.trim();
        if trimmed.is_empty() {
            return Ok(SearchResult { facts: vec![], sessions: vec![] });
        }

        let facts = if q.include_facts && q.top_k_facts > 0 {
            self.search_facts(trimmed, q.top_k_facts).await?
        } else {
            vec![]
        };

        let sessions = if q.include_sessions && q.top_k_sessions > 0 {
            self.search_sessions(trimmed, q.top_k_sessions, q.time_range_days, q.exclude_session.as_deref())
                .await
                .unwrap_or_else(|e| {
                    eprintln!("[memory] session 检索失败：{}", e);
                    vec![]
                })
        } else {
            vec![]
        };

        Ok(SearchResult { facts, sessions })
    }

    async fn search_facts(&self, query: &str, top_k: i64) -> Result<Vec<FactHit>> {
        // facts 体量小、字段语义不齐，简单 LIKE 已够用；
        // active+provisional 都进候选，按 confidence、usage_count 排序
        let pat = format!("%{}%", query);
        let rows = sqlx::query(
            r#"
            SELECT id, fact_type, key, value_json, confidence
            FROM memory_facts
            WHERE status IN ('active', 'provisional')
              AND (
                   (key IS NOT NULL AND key LIKE ?1)
                OR value_json LIKE ?1
              )
            ORDER BY confidence DESC, usage_count DESC, updated_at DESC
            LIMIT ?2
            "#,
        )
        .bind(&pat)
        .bind(top_k)
        .fetch_all(&self.pool)
        .await?;

        let mut hits = Vec::with_capacity(rows.len());
        for r in rows {
            let id: i64 = r.get("id");
            let fact_type: String = r.get("fact_type");
            let key: Option<String> = r.try_get("key").ok();
            let value_text: String = r.get("value_json");
            let confidence: f64 = r.try_get("confidence").unwrap_or(0.7);
            hits.push(FactHit {
                id,
                fact_type,
                key,
                value_excerpt: truncate(&value_text, 200),
                confidence,
            });
        }
        Ok(hits)
    }

    async fn search_sessions(
        &self,
        query: &str,
        top_k: i64,
        time_range_days: i64,
        exclude_session: Option<&str>,
    ) -> Result<Vec<SessionHit>> {
        // FTS5 MATCH：把空白拆成 token，规避特殊符号注入；用 NEAR/AND 提高相关性
        let match_expr = build_fts_match(query);
        if match_expr.is_empty() {
            return Ok(vec![]);
        }

        // 先按 session 维度聚合：取每个 session 内 BM25 最佳的一条作为代表
        let exclude = exclude_session.unwrap_or("");
        let rows = sqlx::query(
            r#"
            WITH ranked AS (
              SELECT
                m.session_id,
                m.id as msg_id,
                m.content,
                m.created_at,
                bm25(analysis_messages_fts) AS rank
              FROM analysis_messages_fts
              JOIN analysis_messages m ON m.id = analysis_messages_fts.rowid
              WHERE analysis_messages_fts MATCH ?1
                AND (?3 = '' OR m.session_id != ?3)
                AND m.created_at >= date('now', ?4)
            ),
            best AS (
              SELECT session_id, MIN(rank) AS best_rank
              FROM ranked
              GROUP BY session_id
            )
            SELECT
              r.session_id,
              r.msg_id,
              r.content,
              r.created_at,
              r.rank,
              COALESCE(s.title, '') AS title
            FROM ranked r
            JOIN best b ON b.session_id = r.session_id AND b.best_rank = r.rank
            LEFT JOIN analysis_sessions s ON s.id = r.session_id
            ORDER BY r.rank ASC
            LIMIT ?2
            "#,
        )
        .bind(&match_expr)
        .bind(top_k)
        .bind(exclude)
        .bind(format!("-{} days", time_range_days.max(1)))
        .fetch_all(&self.pool)
        .await?;

        let mut hits = Vec::with_capacity(rows.len());
        for r in rows {
            let session_id: String = r.get("session_id");
            let content: String = r.get("content");
            let created_at: String = r.try_get("created_at").unwrap_or_default();
            let title: String = r.try_get("title").unwrap_or_default();
            let rank: f64 = r.try_get("rank").unwrap_or(0.0);
            hits.push(SessionHit {
                session_id,
                title,
                snippet: truncate(&content, 200),
                score: -rank, // bm25 越小越相关，转换为 score 越大越相关
                created_at,
            });
        }
        Ok(hits)
    }
}

fn truncate(s: &str, max_chars: usize) -> String {
    let count = s.chars().count();
    if count <= max_chars {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max_chars).collect();
    out.push('…');
    out
}

/// 构造 FTS5 MATCH 表达式：剥离会破坏 MATCH 语法的字符，按空白分词，
/// 多个 token 用 AND 连接，强制每个 token 至少 1 字（trigram 下其实 ≥3 才有意义，
/// 但 trigram tokenizer 内部会自动处理短词）。
fn build_fts_match(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .map(|c| match c {
            '"' | '*' | ':' | '(' | ')' | '\'' | '\\' => ' ',
            _ => c,
        })
        .collect();
    let tokens: Vec<String> = cleaned
        .split_whitespace()
        .filter(|t| !t.is_empty())
        .map(|t| format!("\"{}\"", t))
        .collect();
    tokens.join(" AND ")
}
