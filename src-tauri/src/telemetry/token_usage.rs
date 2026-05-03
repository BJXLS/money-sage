use anyhow::Result;
use sqlx::{Row, SqlitePool};

use crate::models::{
    TokenUsageEntry, TokenUsageFilter, TokenUsageGroupBy, TokenUsageRecord, TokenUsageSummary,
};

/// Token 用量记录服务：每次 LLM 请求一行
#[derive(Clone)]
pub struct TokenUsageRecorder {
    pool: SqlitePool,
}

impl TokenUsageRecorder {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// 写入一条用量记录·
    pub async fn record(&self, rec: TokenUsageRecord) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO token_usage_logs (
                agent_name, session_id, request_id, round_index,
                config_id, config_name_snapshot, provider, model,
                prompt_tokens, completion_tokens, total_tokens,
                finish_reason, duration_ms, success, error_message
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&rec.agent_name)
        .bind(&rec.session_id)
        .bind(&rec.request_id)
        .bind(rec.round_index)
        .bind(rec.config_id)
        .bind(&rec.config_name_snapshot)
        .bind(&rec.provider)
        .bind(&rec.model)
        .bind(rec.prompt_tokens)
        .bind(rec.completion_tokens)
        .bind(rec.total_tokens)
        .bind(&rec.finish_reason)
        .bind(rec.duration_ms)
        .bind(if rec.success { 1i64 } else { 0i64 })
        .bind(&rec.error_message)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    /// 列表查询（带筛选 + 分页）
    pub async fn list(&self, filter: TokenUsageFilter) -> Result<Vec<TokenUsageEntry>> {
        let mut sql = String::from(
            r#"
            SELECT
                t.id, t.agent_name, t.session_id, t.request_id, t.round_index,
                t.config_id,
                COALESCE(c.config_name, t.config_name_snapshot) AS config_name,
                t.provider, t.model,
                t.prompt_tokens, t.completion_tokens, t.total_tokens,
                t.finish_reason, t.duration_ms, t.success, t.error_message,
                t.created_at
            FROM token_usage_logs t
            LEFT JOIN llm_configs c ON c.id = t.config_id
            WHERE 1=1
            "#,
        );
        let mut binds: Vec<String> = Vec::new();

        if let Some(s) = &filter.start_date {
            sql.push_str(" AND t.created_at >= ?");
            binds.push(s.clone());
        }
        if let Some(e) = &filter.end_date {
            sql.push_str(" AND t.created_at < ?");
            binds.push(e.clone());
        }
        if let Some(cid) = filter.config_id {
            sql.push_str(" AND t.config_id = ?");
            binds.push(cid.to_string());
        }
        if let Some(m) = &filter.model {
            sql.push_str(" AND t.model = ?");
            binds.push(m.clone());
        }
        if let Some(a) = &filter.agent_name {
            sql.push_str(" AND t.agent_name = ?");
            binds.push(a.clone());
        }
        if let Some(sid) = &filter.session_id {
            sql.push_str(" AND t.session_id = ?");
            binds.push(sid.clone());
        }
        if filter.success_only.unwrap_or(false) {
            sql.push_str(" AND t.success = 1");
        }
        sql.push_str(" ORDER BY t.created_at DESC, t.id DESC");
        let limit = filter.limit.unwrap_or(100).clamp(1, 1000);
        let offset = filter.offset.unwrap_or(0).max(0);
        sql.push_str(" LIMIT ? OFFSET ?");

        let mut q = sqlx::query(&sql);
        for b in &binds {
            q = q.bind(b);
        }
        q = q.bind(limit).bind(offset);

        let rows = q.fetch_all(&self.pool).await?;
        let entries = rows
            .into_iter()
            .map(|row| TokenUsageEntry {
                id: row.get("id"),
                agent_name: row.get("agent_name"),
                session_id: row
                    .try_get::<Option<String>, _>("session_id")
                    .ok()
                    .flatten(),
                request_id: row.get("request_id"),
                round_index: row.try_get::<i32, _>("round_index").unwrap_or(0),
                config_id: row.try_get::<Option<i64>, _>("config_id").ok().flatten(),
                config_name: row
                    .try_get::<Option<String>, _>("config_name")
                    .ok()
                    .flatten(),
                provider: row.get("provider"),
                model: row.get("model"),
                prompt_tokens: row.try_get::<i64, _>("prompt_tokens").unwrap_or(0),
                completion_tokens: row.try_get::<i64, _>("completion_tokens").unwrap_or(0),
                total_tokens: row.try_get::<i64, _>("total_tokens").unwrap_or(0),
                finish_reason: row
                    .try_get::<Option<String>, _>("finish_reason")
                    .ok()
                    .flatten(),
                duration_ms: row.try_get::<Option<i64>, _>("duration_ms").ok().flatten(),
                success: row.try_get::<i64, _>("success").unwrap_or(0) != 0,
                error_message: row
                    .try_get::<Option<String>, _>("error_message")
                    .ok()
                    .flatten(),
                created_at: row.try_get::<String, _>("created_at").unwrap_or_default(),
            })
            .collect();
        Ok(entries)
    }

    /// 汇总统计：支持按 day / model / config / config+day 分组
    pub async fn summary(
        &self,
        group_by: TokenUsageGroupBy,
        filter: TokenUsageFilter,
    ) -> Result<Vec<TokenUsageSummary>> {
        let (group_expr, label_expr, has_config) = match group_by {
            TokenUsageGroupBy::Day => (
                "date(t.created_at)".to_string(),
                "date(t.created_at)".to_string(),
                false,
            ),
            TokenUsageGroupBy::Model => ("t.model".to_string(), "t.model".to_string(), false),
            TokenUsageGroupBy::Config => (
                "COALESCE(CAST(t.config_id AS TEXT), 'null')".to_string(),
                "COALESCE(c.config_name, t.config_name_snapshot, '未保存的临时配置')".to_string(),
                true,
            ),
            TokenUsageGroupBy::ConfigDay => (
                "COALESCE(CAST(t.config_id AS TEXT), 'null') || '|' || date(t.created_at)"
                    .to_string(),
                "COALESCE(c.config_name, t.config_name_snapshot, '未保存的临时配置') || ' · ' || date(t.created_at)"
                    .to_string(),
                true,
            ),
        };

        let mut sql = format!(
            r#"
            SELECT
                {group_expr} AS group_key,
                {label_expr} AS group_label,
                t.config_id  AS config_id,
                COALESCE(c.config_name, t.config_name_snapshot) AS config_name,
                MIN(t.provider) AS provider,
                MIN(t.model)    AS model,
                COUNT(*)        AS call_count,
                SUM(CASE WHEN t.success = 1 THEN 1 ELSE 0 END) AS success_count,
                SUM(t.prompt_tokens)     AS prompt_tokens,
                SUM(t.completion_tokens) AS completion_tokens,
                SUM(t.total_tokens)      AS total_tokens,
                MAX(t.created_at)        AS last_used_at
            FROM token_usage_logs t
            LEFT JOIN llm_configs c ON c.id = t.config_id
            WHERE 1=1
            "#
        );
        let mut binds: Vec<String> = Vec::new();
        if let Some(s) = &filter.start_date {
            sql.push_str(" AND t.created_at >= ?");
            binds.push(s.clone());
        }
        if let Some(e) = &filter.end_date {
            sql.push_str(" AND t.created_at < ?");
            binds.push(e.clone());
        }
        if let Some(cid) = filter.config_id {
            sql.push_str(" AND t.config_id = ?");
            binds.push(cid.to_string());
        }
        if let Some(m) = &filter.model {
            sql.push_str(" AND t.model = ?");
            binds.push(m.clone());
        }
        if let Some(a) = &filter.agent_name {
            sql.push_str(" AND t.agent_name = ?");
            binds.push(a.clone());
        }
        if let Some(sid) = &filter.session_id {
            sql.push_str(" AND t.session_id = ?");
            binds.push(sid.clone());
        }

        let group_clause = if has_config {
            format!(" GROUP BY {group_expr}, t.config_id")
        } else {
            format!(" GROUP BY {group_expr}")
        };
        sql.push_str(&group_clause);
        sql.push_str(" ORDER BY total_tokens DESC, last_used_at DESC");

        let mut q = sqlx::query(&sql);
        for b in &binds {
            q = q.bind(b);
        }
        let rows = q.fetch_all(&self.pool).await?;
        Ok(rows
            .into_iter()
            .map(|row| TokenUsageSummary {
                group_key: row.try_get::<String, _>("group_key").unwrap_or_default(),
                group_label: row.try_get::<String, _>("group_label").unwrap_or_default(),
                config_id: row.try_get::<Option<i64>, _>("config_id").ok().flatten(),
                config_name: row
                    .try_get::<Option<String>, _>("config_name")
                    .ok()
                    .flatten(),
                provider: row.try_get::<Option<String>, _>("provider").ok().flatten(),
                model: row.try_get::<Option<String>, _>("model").ok().flatten(),
                call_count: row.try_get::<i64, _>("call_count").unwrap_or(0),
                success_count: row.try_get::<i64, _>("success_count").unwrap_or(0),
                prompt_tokens: row.try_get::<i64, _>("prompt_tokens").unwrap_or(0),
                completion_tokens: row.try_get::<i64, _>("completion_tokens").unwrap_or(0),
                total_tokens: row.try_get::<i64, _>("total_tokens").unwrap_or(0),
                last_used_at: row
                    .try_get::<Option<String>, _>("last_used_at")
                    .ok()
                    .flatten(),
            })
            .collect())
    }

    pub async fn purge_before(&self, before: &str) -> Result<u64> {
        let res = sqlx::query("DELETE FROM token_usage_logs WHERE created_at < ?")
            .bind(before)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected())
    }
}
