use super::store::MemoryStore;
use super::snapshot_generator::SnapshotGenerator;
use super::indexer::MemoryIndexer;
use crate::models::LLMConfig;
use crate::utils::http_client::{AIHttpClient, AIProvider, AIRequest, AIMessage, ClientConfig};
use anyhow::Result;
use sqlx::SqlitePool;

/// MemoryGovernor — 异步记忆治理器
///
/// 在独立 tokio task 中运行，负责：
/// 1. 压缩超过 2000 字符的文件（保留最近 10 条，旧条目用 LLM 压缩）
/// 2. 维护各文件夹的 INDEX.md
/// 3. 重新生成 MEMORY.md 快照
///
/// 触发时机：
/// - 每次 AnalysisAgent 对话结束后 30 秒
/// - 应用启动后 60 秒（首次巡检）
/// - 每 6 小时定期巡检
#[derive(Clone)]
pub struct MemoryGovernor {
    pool: SqlitePool,
    store: MemoryStore,
    indexer: MemoryIndexer,
    snapshot_gen: SnapshotGenerator,
}

#[derive(Debug, Default)]
pub struct GovernorReport {
    pub compressed_files: usize,
    pub updated_indices: usize,
    pub snapshot_regenerated: bool,
    pub snapshot_char_count: usize,
}

impl MemoryGovernor {
    pub fn new(pool: SqlitePool, store: MemoryStore) -> Self {
        let indexer = MemoryIndexer::new(pool.clone(), store.clone());
        let snapshot_gen = SnapshotGenerator::new(store.clone());
        Self {
            pool,
            store,
            indexer,
            snapshot_gen,
        }
    }

    /// Governor 主巡检流程
    pub async fn run(&self) -> Result<GovernorReport> {
        let mut report = GovernorReport::default();

        // 1. 扫描超标文件并压缩
        let files = self.store.list_files_recursive()?;
        let mut oversized: Vec<(String, usize)> = files
            .into_iter()
            .filter(|(path, count)| {
                path.ends_with(".md")
                    && !path.starts_with("meta/")
                    && *count > 2000
            })
            .collect();

        // 按超过量排序，最多处理 3 个
        oversized.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        oversized.truncate(3);

        for (path, char_count) in oversized {
            match self.compress_file(&path, char_count).await {
                Ok(()) => report.compressed_files += 1,
                Err(e) => eprintln!("Governor 压缩失败 {}: {}", path, e),
            }
        }

        // 2. 同步 FTS5 索引
        if let Err(e) = self.indexer.sync_all().await {
            eprintln!("Governor 索引同步失败: {}", e);
        }

        // 3. 更新各文件夹 INDEX.md
        if let Err(e) = self.update_indices().await {
            eprintln!("Governor 更新 INDEX 失败: {}", e);
        } else {
            report.updated_indices += 1;
        }

        // 4. 重新生成 MEMORY.md
        match self.regenerate_snapshot().await {
            Ok(char_count) => {
                report.snapshot_regenerated = true;
                report.snapshot_char_count = char_count;
            }
            Err(e) => eprintln!("Governor 快照生成失败: {}", e),
        }

        Ok(report)
    }

    /// 压缩单个文件：保留最近 10 条完整 & 行，旧条目用 LLM 压缩为摘要
    async fn compress_file(&self, rel_path: &str, _char_count: usize) -> Result<()> {
        let content = self.store.read_file(rel_path)?;

        // 解析文件结构：文件头 + 若干 section
        let (header, mut sections) = parse_file(&content);

        let mut changed = false;
        for section in &mut sections {
            if section.entries.len() > 10 {
                // 保留最近 10 条（entries[0..10]，因为新条目插入在 heading 后最前面）
                let recent: Vec<String> = section.entries.iter().take(10).cloned().collect();
                let old: Vec<String> = section.entries.iter().skip(10).cloned().collect();

                // 用 LLM 压缩旧条目（失败则降级为机械摘要）
                let summary_text = match self.summarize_entries(&old).await {
                    Ok(text) => text,
                    Err(e) => {
                        eprintln!(
                            "Governor LLM 压缩失败，使用机械摘要: {}",
                            e
                        );
                        format!("共 {} 条旧记录", old.len())
                    }
                };

                let now = chrono::Local::now()
                    .format("%Y-%m-%dT%H:%M:%S%:z")
                    .to_string();
                let summary_entry = format!(
                    "& {} | agent:governor | [历史压缩] {}",
                    now, summary_text
                );

                section.entries = recent;
                section.entries.push(summary_entry);
                changed = true;
            }
        }

        if !changed {
            return Ok(());
        }

        // 重新组装文件并写回
        let new_content = rebuild_file(&header, &sections);
        self.store.write_file(rel_path, &new_content)?;
        Ok(())
    }

    /// 调用 LLM 压缩旧条目，保留关键信息和时间信息
    async fn summarize_entries(&self, entries: &[String]) -> Result<String> {
        if entries.is_empty() {
            return Ok(String::new());
        }

        let llm_config = self.fetch_llm_config().await?;
        let prompt = build_compression_prompt(entries);

        let client = build_http_client(&llm_config)?;
        let request = AIRequest {
            model: llm_config.model,
            messages: vec![
                AIMessage::text(
                    "system",
                    "你是一个记忆压缩助手。你的任务是将多条旧记忆条目压缩为一条简洁的摘要，保留关键事实和时间演化脉络。"
                ),
                AIMessage::text("user", prompt),
            ],
            temperature: (llm_config.temperature * 0.5) as f32, // 低温度，更稳定
            max_tokens: 512,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stream: None,
            enable_thinking: false,
            tools: None,
            tool_choice: None,
        };

        let response = client.chat_completion(request).await?;
        let text = response
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content_text().to_string())
            .unwrap_or_default()
            .trim()
            .to_string();

        if text.is_empty() {
            return Ok(format!("共 {} 条旧记录", entries.len()));
        }
        Ok(text)
    }

    /// 从数据库获取活跃 LLM 配置
    async fn fetch_llm_config(&self) -> Result<LLMConfig> {
        let row = sqlx::query(
            "SELECT * FROM llm_configs WHERE is_active = 1 ORDER BY updated_at DESC LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("没有活跃的 LLM 配置"))?;

        use sqlx::Row;
        Ok(LLMConfig {
            id: row.get("id"),
            config_name: row.try_get("config_name").unwrap_or_default(),
            provider: row
                .try_get("provider")
                .unwrap_or_else(|_| row.try_get("platform").unwrap_or_default()),
            base_url: row.try_get("base_url").unwrap_or_default(),
            api_key: row
                .try_get("api_key")
                .unwrap_or_else(|_| row.try_get("app_key").unwrap_or_default()),
            model: row.try_get("model").unwrap_or_default(),
            temperature: row.try_get::<f64, _>("temperature").unwrap_or(0.7),
            max_tokens: row.try_get::<i64, _>("max_tokens").unwrap_or(2048),
            enable_thinking: row.try_get::<i64, _>("enable_thinking").unwrap_or(0) != 0,
            is_active: true,
            created_at: row.try_get("created_at").unwrap_or_default(),
            updated_at: row.try_get("updated_at").unwrap_or_default(),
        })
    }

    /// 更新各文件夹的 INDEX.md
    async fn update_indices(&self) -> Result<()> {
        let files = self.store.list_files_recursive()?;
        let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%:z").to_string();

        // 按文件夹分组
        let mut folders: std::collections::HashMap<String, Vec<(String, usize)>> =
            std::collections::HashMap::new();
        for (path, count) in files {
            if path == "MEMORY.md" || path.starts_with("meta/") {
                continue;
            }
            let parent = std::path::Path::new(&path)
                .parent()
                .and_then(|p| p.to_str())
                .unwrap_or("")
                .to_string();
            if parent.is_empty() {
                continue;
            }
            folders.entry(parent).or_default().push((path, count));
        }

        for (folder, items) in folders {
            let index_path = format!("{}/INDEX.md", folder);
            let mut content = format!(
                "# {}/ 索引\n> 共 {} 个文件 | 最后更新：{}\n\n",
                folder,
                items.len(),
                now
            );

            for (path, count) in items {
                let file_name = std::path::Path::new(&path)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                // 尝试读取文件第一行非注释内容作为摘要
                let summary = self
                    .store
                    .read_file(&path)
                    .ok()
                    .and_then(|c| {
                        c.lines()
                            .find(|l| {
                                let t = l.trim();
                                !t.is_empty() && !t.starts_with("<!--") && !t.starts_with('#')
                            })
                            .map(|l| l.trim().to_string())
                    })
                    .unwrap_or_else(|| format!("{} 字符", count));

                content.push_str(&format!("## {}\n{}\n\n", file_name, summary));
            }

            self.store.write_file(&index_path, &content)?;
        }

        Ok(())
    }

    /// 重新生成 MEMORY.md
    async fn regenerate_snapshot(&self) -> Result<usize> {
        let snapshot = self.snapshot_gen.regenerate()?;
        let char_count = snapshot.chars().count();
        self.store.write_file("MEMORY.md", &snapshot)?;
        Ok(char_count)
    }
}

// ── 文件解析与重组 ──

/// 记忆文件中的一个 section（以 heading 为界）
#[derive(Debug)]
struct Section {
    heading: String,
    /// heading 后的 & 条目（按时间倒序：最新在前）
    entries: Vec<String>,
    /// heading 后的其他行（描述、注释、空行等，被 entries 挤到后面）
    other_lines: Vec<String>,
}

/// 解析文件为「文件头 + sections」
///
/// 策略：
/// - 如果文件包含 `## `，按 `## ` 分割 section，其前为文件头
/// - 如果不含 `## ` 但含 `# `，按第一个 `# ` 分割，其前为文件头
fn parse_file(content: &str) -> (Vec<String>, Vec<Section>) {
    let mut header: Vec<String> = Vec::new();
    let mut sections: Vec<Section> = Vec::new();
    let mut current: Option<Section> = None;

    let has_h2 = content.lines().any(|l| l.trim_start().starts_with("## "));

    for line in content.lines() {
        let trimmed = line.trim_start();
        let is_heading = if has_h2 {
            trimmed.starts_with("## ")
        } else {
            trimmed.starts_with("# ")
        };

        if is_heading {
            if let Some(s) = current.take() {
                sections.push(s);
            }
            current = Some(Section {
                heading: line.to_string(),
                entries: Vec::new(),
                other_lines: Vec::new(),
            });
        } else if let Some(ref mut sec) = current {
            if trimmed.starts_with("& ") {
                sec.entries.push(line.to_string());
            } else {
                sec.other_lines.push(line.to_string());
            }
        } else {
            header.push(line.to_string());
        }
    }

    if let Some(s) = current.take() {
        sections.push(s);
    }

    (header, sections)
}

/// 将文件头和 sections 重新组装为完整文件内容
fn rebuild_file(header: &[String], sections: &[Section]) -> String {
    let mut lines: Vec<String> = header.to_vec();

    for (_i, section) in sections.iter().enumerate() {
        // section 之间保留一个空行（但不要连续多个空行）
        let need_blank = !lines.is_empty()
            && !lines.last().unwrap().is_empty()
            && !section.heading.is_empty();
        if need_blank {
            lines.push(String::new());
        }

        lines.push(section.heading.clone());

        for entry in &section.entries {
            lines.push(entry.clone());
        }

        for other in &section.other_lines {
            lines.push(other.clone());
        }
    }

    let content = lines.join("\n");
    if content.ends_with('\n') {
        content
    } else {
        content + "\n"
    }
}

// ── LLM 提示词与客户端 ──

fn build_compression_prompt(entries: &[String]) -> String {
    let entries_text = entries.join("\n");
    format!(
        r#"请将以下较旧的记忆条目压缩为一条简洁的摘要。

要求：
1. 保留所有关键事实（如具体金额、日期、名称、分类规则等）
2. 保留时间演化脉络（如"从X修正为Y"、"首次记录→后续更新"）
3. 合并重复或相似信息，去除冗余描述
4. 用一到两句话总结，尽量简洁
5. 不要编造，只基于提供的条目

原始条目：
{}

请直接输出摘要文本（不含时间戳前缀，不要加引号）："#,
        entries_text
    )
}

fn build_http_client(config: &LLMConfig) -> Result<AIHttpClient> {
    let client_config = ClientConfig {
        provider: AIProvider::Custom(config.provider.clone()),
        base_url: config.base_url.clone(),
        api_key: config.api_key.clone(),
        timeout_secs: 30,
        max_retries: 1,
        headers: std::collections::HashMap::new(),
    };
    AIHttpClient::new(client_config)
}
