use super::store::MemoryStore;
use super::indexer::MemoryIndexer;
use super::snapshot_generator::SnapshotGenerator;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;

/// 记忆整合器 —— 每次对话结束后，调用 LLM 全面总结并自动写入记忆
///
/// 不只是 episodic 摘要，而是智能判断：
/// - 有没有新的 factual 信息（用户画像、分类规则、财务目标等）
/// - 有没有新的 procedural 经验（工作流技巧）
/// - 有没有值得记的情景（episodic）
/// - 有没有需要更新 MEMORY.md 的快照信息
#[derive(Clone)]
pub struct MemoryConsolidator {
    pool: SqlitePool,
    store: MemoryStore,
    indexer: MemoryIndexer,
    snapshot_gen: SnapshotGenerator,
}

/// LLM 返回的结构化总结
#[derive(Debug, Deserialize)]
pub struct ConsolidationResult {
    /// 是否值得记住（如果对话只是闲聊/查余额，可能为 false）
    pub worth_remembering: bool,
    /// factual 更新列表
    pub factual_updates: Vec<MemoryUpdate>,
    /// episodic 摘要（单条字符串，会追加到当日文件）
    pub episodic_summary: Option<String>,
    /// procedural 更新列表
    pub procedural_updates: Vec<MemoryUpdate>,
    /// MEMORY.md 快照需要更新的行列表
    pub snapshot_updates: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MemoryUpdate {
    /// 目标文件相对路径，如 "factual/user-profile.md"
    pub file: String,
    /// 记忆内容（不含 & 前缀，整合器会自动加时间戳）
    pub content: String,
    /// 可选的 heading，如 "工资信息"
    pub heading: Option<String>,
}

#[derive(Debug, Default)]
pub struct ConsolidationReport {
    pub factual_written: usize,
    pub episodic_written: bool,
    pub procedural_written: usize,
    pub snapshot_updated: bool,
}

impl MemoryConsolidator {
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

    /// 主入口：传入本轮对话历史，调用 LLM 总结并写入记忆
    ///
    /// # Arguments
    /// * `session_id` — 当前会话 ID
    /// * `conversation` — 本轮对话的文本记录，格式为 "user: ...\nassistant: ...\n"
    /// * `llm_client` — HTTP 客户端，用于调用辅助模型
    pub async fn consolidate(
        &self,
        session_id: &str,
        conversation: &str,
    ) -> Result<ConsolidationReport> {
        // 1. 读取当前记忆快照作为上下文
        let current_memory = self.store.read_file("MEMORY.md").unwrap_or_default();

        // 2. 构建 LLM 提示词
        let prompt = build_consolidation_prompt(conversation, &current_memory);

        // 3. 调用辅助模型（这里先用规则引擎降级，后续接入真实 LLM）
        // TODO: 接入辅助模型（使用 AIHttpClient 或配置的 aux model）
        let result = self.rule_based_extract(conversation).await?;

        if !result.worth_remembering {
            return Ok(ConsolidationReport::default());
        }

        let mut report = ConsolidationReport::default();
        let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%:z").to_string();

        // 4. 写入 factual 更新
        for update in &result.factual_updates {
            if let Err(e) = self.append_to_file(&update.file, &update.heading, &update.content, &now).await {
                eprintln!("Consolidator 写入 factual 失败 {}: {}", update.file, e);
            } else {
                report.factual_written += 1;
            }
        }

        // 5. 写入 episodic 摘要
        if let Some(summary) = &result.episodic_summary {
            let date = chrono::Local::now().format("%Y-%m-%d").to_string();
            let year = chrono::Local::now().format("%Y").to_string();
            let month = chrono::Local::now().format("%m").to_string();
            let rel_path = format!("episodic/{}/{}/{}.md", year, month, date);

            // 确保目录存在
            let full_path = self.store.memory_dir().join(&rel_path);
            if let Some(parent) = full_path.parent() {
                if !parent.exists() {
                    let _ = std::fs::create_dir_all(parent);
                }
            }

            let entry = format!("& {} | agent:analysis | {}", now, summary);
            if let Err(e) = self.append_line_to_file(&rel_path, &entry).await {
                eprintln!("Consolidator 写入 episodic 失败: {}", e);
            } else {
                report.episodic_written = true;
            }
        }

        // 6. 写入 procedural 更新
        for update in &result.procedural_updates {
            if let Err(e) = self.append_to_file(&update.file, &update.heading, &update.content, &now).await {
                eprintln!("Consolidator 写入 procedural 失败 {}: {}", update.file, e);
            } else {
                report.procedural_written += 1;
            }
        }

        // 7. 增量更新 MEMORY.md 快照
        if !result.snapshot_updates.is_empty() {
            let current = self.store.read_file("MEMORY.md").unwrap_or_default();
            let mut updated = current;
            for line in &result.snapshot_updates {
                updated = self.snapshot_gen.append_entry(&updated, "关于你", line);
            }
            if let Err(e) = self.store.write_file("MEMORY.md", &updated) {
                eprintln!("Consolidator 更新 MEMORY.md 失败: {}", e);
            } else {
                report.snapshot_updated = true;
            }
        }

        // 8. 同步 FTS5 索引
        if let Err(e) = self.indexer.sync_all().await {
            eprintln!("Consolidator 索引同步失败: {}", e);
        }

        Ok(report)
    }

    /// 规则引擎降级：当 LLM 不可用时，用简单规则提取关键信息
    async fn rule_based_extract(&self, conversation: &str) -> Result<ConsolidationResult> {
        let lower = conversation.to_lowercase();
        let mut factual_updates = Vec::new();
        let mut episodic_summary = None;
        let mut snapshot_updates = Vec::new();

        // 简单规则：检测用户提到的新信息
        if lower.contains("工资") || lower.contains("收入") || lower.contains("月薪") {
            if let Some(info) = extract_salary_info(conversation) {
                factual_updates.push(MemoryUpdate {
                    file: "factual/user-profile.md".to_string(),
                    content: info.clone(),
                    heading: Some("工资信息".to_string()),
                });
                snapshot_updates.push(info);
            }
        }

        if lower.contains("分类") && (lower.contains("应该") || lower.contains("归到")) {
            if let Some(rule) = extract_classification_rule(conversation) {
                factual_updates.push(MemoryUpdate {
                    file: "factual/finance-rules.md".to_string(),
                    content: rule.clone(),
                    heading: Some("分类规则".to_string()),
                });
                snapshot_updates.push(rule);
            }
        }

        if lower.contains("目标") || lower.contains("存钱") || lower.contains("预算") {
            if let Some(goal) = extract_goal_info(conversation) {
                factual_updates.push(MemoryUpdate {
                    file: "factual/goals.md".to_string(),
                    content: goal.clone(),
                    heading: None,
                });
                snapshot_updates.push(goal);
            }
        }

        // 如果检测到任何更新，生成 episodic 摘要
        if !factual_updates.is_empty() || !snapshot_updates.is_empty() {
            let summary = generate_episodic_summary(conversation);
            episodic_summary = Some(summary);
        }

        let worth_remembering = !factual_updates.is_empty()
            || episodic_summary.is_some()
            || !snapshot_updates.is_empty();

        Ok(ConsolidationResult {
            worth_remembering,
            factual_updates,
            episodic_summary,
            procedural_updates: Vec::new(),
            snapshot_updates,
        })
    }

    /// 在指定文件的指定 heading 下追加内容
    async fn append_to_file(
        &self,
        rel_path: &str,
        heading: &Option<String>,
        content: &str,
        timestamp: &str,
    ) -> Result<()> {
        let entry = format!("& {} | agent:analysis | {}", timestamp, content);

        let current = self.store.read_file(rel_path).unwrap_or_default();
        let mut lines: Vec<String> = current.lines().map(|s| s.to_string()).collect();

        if let Some(h) = heading {
            let target = format!("## {}", h);
            let mut idx = None;
            for (i, line) in lines.iter().enumerate() {
                if line.trim() == target {
                    idx = Some(i + 1);
                } else if idx.is_some() && line.trim().starts_with("## ") {
                    // 找到了下一个 heading，插在它前面
                    idx = Some(i);
                    break;
                }
            }

            if let Some(i) = idx {
                lines.insert(i, entry);
            } else {
                // heading 不存在，在末尾添加
                lines.push(String::new());
                lines.push(target);
                lines.push(entry);
            }
        } else {
            // 没有指定 heading，追加到文件末尾
            lines.push(String::new());
            lines.push(entry);
        }

        let new_content = lines.join("\n");
        self.store.write_file(rel_path, &new_content)?;
        Ok(())
    }

    /// 在文件末尾追加一行
    async fn append_line_to_file(&self, rel_path: &str, line: &str) -> Result<()> {
        let current = self.store.read_file(rel_path).unwrap_or_default();
        let mut new_content = current;
        if !new_content.ends_with('\n') && !new_content.is_empty() {
            new_content.push('\n');
        }
        new_content.push_str(line);
        new_content.push('\n');
        self.store.write_file(rel_path, &new_content)?;
        Ok(())
    }
}

/// 构建 LLM 总结提示词（用于后续接入真实 LLM）
fn build_consolidation_prompt(conversation: &str, current_memory: &str) -> String {
    format!(
        r#"你是一位记忆整理助手。请分析以下对话，判断是否有值得跨会话记住的信息。

## 当前记忆快照
{}

## 本轮对话
{}

## 任务
请输出 JSON 格式：
{{
  "worth_remembering": true/false,
  "factual_updates": [
    {{"file": "factual/user-profile.md", "heading": "工资信息", "content": "用户每月15号发工资"}}
  ],
  "episodic_summary": "今天讨论了预算调整",
  "procedural_updates": [],
  "snapshot_updates": ["每月15号发工资，约15000元"]
}}

规则：
- worth_remembering：只有对话中包含用户明确提供的新信息时才为 true
- factual_updates：写入 factual/ 下的具体文件，content 不含时间戳
- episodic_summary：一句话概括本轮对话主题（可选）
- snapshot_updates：MEMORY.md 中需要新增或更新的摘要行（可选）
- 不要编造，只基于对话内容
- 如果对话只是查余额、闲聊，worth_remembering 应为 false"#,
        current_memory, conversation
    )
}

// ── 简单规则提取函数（降级方案）──

fn extract_salary_info(text: &str) -> Option<String> {
    // 简单规则：找 "工资" 后面的数字和描述
    let lower = text.to_lowercase();
    if let Some(idx) = lower.find("工资") {
        let after = &text[idx..];
        if let Some(num) = find_first_number(after) {
            return Some(format!("用户工资约{}元", num));
        }
    }
    None
}

fn extract_classification_rule(text: &str) -> Option<String> {
    // 简单规则：找 "A 归到 B" 或 "A 属于 B"
    let lower = text.to_lowercase();
    if let Some(idx) = lower.find("归到") {
        let before = &text[..idx];
        let after = &text[idx + 4..];
        let from = before.split(|c| c == '，' || c == '。').last().unwrap_or("").trim();
        let to = after.split(|c| c == '，' || c == '。').next().unwrap_or("").trim();
        if !from.is_empty() && !to.is_empty() {
            return Some(format!("\"{}\" → {}", from, to));
        }
    }
    if let Some(idx) = lower.find("属于") {
        let before = &text[..idx];
        let after = &text[idx + 4..];
        let from = before.split(|c| c == '，' || c == '。').last().unwrap_or("").trim();
        let to = after.split(|c| c == '，' || c == '。').next().unwrap_or("").trim();
        if !from.is_empty() && !to.is_empty() {
            return Some(format!("\"{}\" → {}", from, to));
        }
    }
    None
}

fn extract_goal_info(text: &str) -> Option<String> {
    // 简单规则：找 "目标" "存钱" 后面的数字
    let lower = text.to_lowercase();
    let keywords = ["目标", "存钱", "储蓄"];
    for kw in keywords {
        if let Some(idx) = lower.find(kw) {
            let after = &text[idx..];
            if let Some(num) = find_first_number(after) {
                return Some(format!("储蓄目标：{}元", num));
            }
        }
    }
    None
}

fn find_first_number(text: &str) -> Option<String> {
    let mut result = String::new();
    let mut started = false;
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            started = true;
            result.push(ch);
        } else if ch == ',' || ch == '.' {
            if started {
                result.push(ch);
            }
        } else if started {
            break;
        }
    }
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

fn generate_episodic_summary(text: &str) -> String {
    // 取对话的前 100 字作为摘要
    let trimmed = text.trim();
    if trimmed.len() > 100 {
        format!("{}", &trimmed[..100])
    } else {
        trimmed.to_string()
    }
}
