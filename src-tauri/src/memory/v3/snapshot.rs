use super::store::MemoryStore;

/// MEMORY.md 快照加载器
///
/// 负责从磁盘读取 MEMORY.md，并生成会话级冻结快照。
/// 快照在会话开始时加载，整个会话期间不变（保护 LLM prefix cache）。
#[derive(Clone)]
pub struct SnapshotLoader {
    store: MemoryStore,
}

impl SnapshotLoader {
    pub fn new(store: MemoryStore) -> Self {
        Self { store }
    }

    /// 从磁盘加载 MEMORY.md 内容
    /// 如果文件不存在或为空，返回默认提示文本
    pub fn load(&self) -> String {
        match self.store.read_file("MEMORY.md") {
            Ok(content) => {
                let trimmed = content.trim();
                if trimmed.is_empty() {
                    Self::default_snapshot()
                } else {
                    trimmed.to_string()
                }
            }
            Err(_) => Self::default_snapshot(),
        }
    }

    /// 默认快照内容（当 MEMORY.md 不存在或为空时）
    fn default_snapshot() -> String {
        "# 记忆快照\n\n暂无跨会话记忆。".to_string()
    }
}
