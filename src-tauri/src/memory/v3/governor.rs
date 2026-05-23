use super::store::MemoryStore;
use super::snapshot_generator::SnapshotGenerator;
use super::indexer::MemoryIndexer;
use anyhow::Result;
use sqlx::SqlitePool;

/// MemoryGovernor — 异步记忆治理器
///
/// 在独立 tokio task 中运行，负责：
/// 1. 压缩超过 2000 字符的文件
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

    /// 压缩单个文件：保留最近 5 条完整 & 行，旧条目合并为摘要
    async fn compress_file(&self, rel_path: &str, _char_count: usize) -> Result<()> {
        let content = self.store.read_file(rel_path)?;
        let mut new_lines: Vec<String> = Vec::new();
        let mut current_heading = String::new();
        let mut heading_entries: Vec<String> = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("## ") {
                // 处理上一个 heading 的条目
                if !current_heading.is_empty() && heading_entries.len() > 5 {
                    flush_heading(&mut new_lines, &current_heading, &mut heading_entries)?;
                } else if !current_heading.is_empty() {
                    new_lines.push(current_heading.clone());
                    for e in &heading_entries {
                        new_lines.push(e.clone());
                    }
                }
                current_heading = line.to_string();
                heading_entries.clear();
                continue;
            }
            if trimmed.starts_with('&') {
                heading_entries.push(line.to_string());
            } else {
                // 非记忆行（元数据注释、空行等）保留
                if !current_heading.is_empty() && !heading_entries.is_empty() {
                    // 还在 heading 内，先刷出已有条目
                    if heading_entries.len() > 5 {
                        flush_heading(&mut new_lines, &current_heading, &mut heading_entries)?;
                    } else {
                        new_lines.push(current_heading.clone());
                        for e in &heading_entries {
                            new_lines.push(e.clone());
                        }
                    }
                    current_heading.clear();
                    heading_entries.clear();
                }
                new_lines.push(line.to_string());
            }
        }

        // 处理最后一个 heading
        if !current_heading.is_empty() {
            if heading_entries.len() > 5 {
                flush_heading(&mut new_lines, &current_heading, &mut heading_entries)?;
            } else {
                new_lines.push(current_heading);
                for e in heading_entries {
                    new_lines.push(e);
                }
            }
        }

        let new_content = new_lines.join("\n");
        self.store.write_file(rel_path, &new_content)?;
        Ok(())
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
            let mut content = format!("# {}/ 索引\n> 共 {} 个文件 | 最后更新：{}\n\n", folder, items.len(), now);

            for (path, count) in items {
                let file_name = std::path::Path::new(&path)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                // 尝试读取文件第一行非注释内容作为摘要
                let summary = self.store.read_file(&path)
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

/// 将 heading 下的条目压缩：保留最近 5 条，其余合并为摘要行
fn flush_heading(
    new_lines: &mut Vec<String>,
    heading: &str,
    entries: &mut Vec<String>,
) -> Result<()> {
    new_lines.push(heading.to_string());

    // 保留最近 5 条
    let recent: Vec<String> = entries.iter().rev().take(5).cloned().collect();
    for e in recent.iter().rev() {
        new_lines.push(e.clone());
    }

    // 旧条目合并为摘要
    let old_count = entries.len().saturating_sub(5);
    if old_count > 0 {
        let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%:z").to_string();
        let summary = format!(
            "& {} | agent:governor | [历史压缩] {} 条旧记录已合并（保留最近 5 条）",
            now, old_count
        );
        new_lines.push(summary);
    }

    entries.clear();
    Ok(())
}
