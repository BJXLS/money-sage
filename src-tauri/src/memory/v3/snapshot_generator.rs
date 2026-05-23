use super::store::MemoryStore;
use anyhow::Result;

const MEMORY_MD_MAX_CHARS: usize = 2000;

/// 从所有记忆文件编译生成 MEMORY.md 快照
#[derive(Clone)]
pub struct SnapshotGenerator {
    store: MemoryStore,
}

impl SnapshotGenerator {
    pub fn new(store: MemoryStore) -> Self {
        Self { store }
    }

    /// 全量重生成 MEMORY.md
    /// 读取所有 factual/ episodic/ procedural/ 下的 .md 文件
    /// 提取最新条目，按分类组织，截断到 2000 字符
    pub fn regenerate(&self) -> Result<String> {
        let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%:z").to_string();

        // 统计信息
        let files = self.store.list_files_recursive()?;
        let total_entries: usize = files
            .iter()
            .filter(|(p, _)| !p.starts_with("meta/") && p != "MEMORY.md")
            .map(|(path, _)| {
                self.store
                    .read_file(path)
                    .unwrap_or_default()
                    .lines()
                    .filter(|l| l.trim_start().starts_with('&'))
                    .count()
            })
            .sum();

        let mut output = format!(
            "# 记忆快照\n> 更新：{} | 条目：{} | 文件：{}\n\n",
            now,
            total_entries,
            files.len()
        );

        // ── 关于你（factual 中的用户画像、财务规则） ──
        let about_you = self.extract_category("factual/", &["user-profile", "finance-rules", "goals"], 5);
        if !about_you.is_empty() {
            output.push_str("## 关于你\n");
            for line in &about_you {
                output.push_str(&format!("- {}\n", line));
            }
            output.push('\n');
        }

        // ── 重要规则（factual 中的分类规则、周期事件） ──
        let rules = self.extract_category("factual/", &["finance-rules"], 5);
        if !rules.is_empty() {
            output.push_str("## 重要规则\n");
            for line in &rules {
                output.push_str(&format!("- {}\n", line));
            }
            output.push('\n');
        }

        // ── 近期对话（episodic 最新 7 天） ──
        let recent = self.extract_recent_episodic(7, 5);
        if !recent.is_empty() {
            output.push_str("## 近期对话\n");
            for line in &recent {
                output.push_str(&format!("- {}\n", line));
            }
            output.push('\n');
        }

        // ── 值得注意（所有类别中标记为重要或矛盾的内容） ──
        let notable = self.extract_notable(3);
        if !notable.is_empty() {
            output.push_str("## 值得注意\n");
            for line in &notable {
                output.push_str(&format!("- {}\n", line));
            }
            output.push('\n');
        }

        // 硬性截断到 2000 字符
        let trimmed = limit_chars(&output, MEMORY_MD_MAX_CHARS);
        Ok(trimmed)
    }

    /// Agent 增量更新入口：在现有 MEMORY.md 中追加/替换一条
    /// 返回更新后的完整内容（不做截断，由 Governor 最终截断）
    pub fn append_entry(&self, current: &str, category: &str, entry: &str) -> String {
        let mut lines: Vec<&str> = current.lines().collect();

        // 找到对应分类的插入位置
        let category_header = format!("## {}", category);
        let mut insert_idx = None;
        let mut next_category_idx = None;

        for (i, line) in lines.iter().enumerate() {
            if line.trim() == category_header {
                insert_idx = Some(i + 1);
            } else if insert_idx.is_some()
                && line.trim().starts_with("## ")
                && line.trim() != category_header
            {
                next_category_idx = Some(i);
                break;
            }
        }

        let new_line = format!("- {}", entry);

        if let Some(idx) = insert_idx {
            // 在已有分类下追加到顶部（idx 是该分类 header 后的第一行）
            lines.insert(idx, &new_line);
        } else {
            // 分类不存在，在文件末尾添加新分类
            lines.push("");
            lines.push(&category_header);
            lines.push(&new_line);
        }

        lines.join("\n")
    }

    // ── 内部提取 ──

    /// 从指定类别中提取最新条目（按文件过滤）
    fn extract_category(&self, prefix: &str, file_keywords: &[&str], max_per_file: usize) -> Vec<String> {
        let mut result = Vec::new();

        match self.store.list_files_recursive() {
            Ok(files) => {
                for (path, _) in files {
                    if !path.starts_with(prefix) || path == "MEMORY.md" || path.starts_with("meta/") {
                        continue;
                    }
                    // 文件关键词过滤
                    let file_name = std::path::Path::new(&path)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");
                    if !file_keywords.is_empty() && !file_keywords.iter().any(|kw| file_name.contains(kw)) {
                        continue;
                    }

                    if let Ok(content) = self.store.read_file(&path) {
                        let entries: Vec<String> = content
                            .lines()
                            .filter(|l| l.trim_start().starts_with('&'))
                            .take(max_per_file)
                            .filter_map(|line| parse_summary(line))
                            .collect();
                        result.extend(entries);
                    }
                }
            }
            Err(_) => {}
        }

        result.truncate(max_per_file * file_keywords.len().max(1));
        result
    }

    /// 提取最近 N 天的 episodic 摘要
    fn extract_recent_episodic(&self, days: i64, max_entries: usize) -> Vec<String> {
        let cutoff = chrono::Local::now() - chrono::Duration::days(days);
        let cutoff_str = cutoff.format("%Y-%m-%d").to_string();

        let mut result = Vec::new();

        match self.store.list_files_recursive() {
            Ok(files) => {
                for (path, _) in files {
                    if !path.starts_with("episodic/") {
                        continue;
                    }
                    // 从文件名提取日期：episodic/2026/05/2026-05-23.md
                    let file_name = std::path::Path::new(&path)
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");
                    if file_name < cutoff_str.as_str() {
                        continue;
                    }

                    if let Ok(content) = self.store.read_file(&path) {
                        let entries: Vec<String> = content
                            .lines()
                            .filter(|l| l.trim_start().starts_with('&'))
                            .filter_map(|line| parse_summary(line))
                            .collect();
                        result.extend(entries);
                    }
                }
            }
            Err(_) => {}
        }

        result.truncate(max_entries);
        result
    }

    /// 提取值得注意的内容（重要标记、矛盾、用户纠正）
    fn extract_notable(&self, max_entries: usize) -> Vec<String> {
        let mut result = Vec::new();

        match self.store.list_files_recursive() {
            Ok(files) => {
                for (path, _) in files {
                    if path.starts_with("meta/") || path == "MEMORY.md" {
                        continue;
                    }
                    if let Ok(content) = self.store.read_file(&path) {
                        for line in content.lines() {
                            let trimmed = line.trim_start();
                            if !trimmed.starts_with('&') {
                                continue;
                            }
                            // 标记为重要或矛盾的内容
                            if trimmed.contains("[重要]") || trimmed.contains("[纠正]") || trimmed.contains("[矛盾]") {
                                if let Some(summary) = parse_summary(trimmed) {
                                    result.push(summary);
                                }
                            }
                        }
                    }
                }
            }
            Err(_) => {}
        }

        result.truncate(max_entries);
        result
    }
}

/// 从 & 行提取摘要文本（去掉时间戳和来源）
fn parse_summary(line: &str) -> Option<String> {
    // 格式: & <timestamp> | <source> | <content>
    let without_prefix = line.trim_start().trim_start_matches('&').trim_start();
    let parts: Vec<&str> = without_prefix.splitn(3, " | ").collect();
    if parts.len() < 3 {
        return Some(without_prefix.to_string());
    }
    let content = parts[2].trim();
    if content.is_empty() {
        return None;
    }
    Some(content.to_string())
}

fn limit_chars(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }

    // 保留头部，在最后一个完整段落截断
    let mut count = 0;
    let mut last_newline = 0;
    for (i, ch) in input.char_indices() {
        count += 1;
        if ch == '\n' {
            last_newline = i;
        }
        if count >= max_chars.saturating_sub(50) {
            // 留 50 字符给截断标记
            return format!("{}\n\n（内容过多，已截断）", &input[..last_newline]);
        }
    }
    input.to_string()
}
