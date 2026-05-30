use anyhow::Result;
use fs2::FileExt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const DEFAULT_MEMORY_MD: &str = r#"# 记忆快照
> 更新：{now} | 条目：0 | 文件：0

暂无记忆。Agent 会在对话中逐步学习并更新此文件。
"#;

const DEFAULT_META_RULES_MD: &str = r#"## 记忆系统写入规范

memory/ 目录是跨会话记忆的真源，Agent 通过 `file_edit` 或 `file_write` 操作。写入时必须遵守以下规范：

1. **追加新行，不删除旧行**：保留完整演进历史。
2. **新条目加在顶部**（同一 `##` 标题下），保持时间倒序。
3. **在同一标题下追加**：保持话题聚合，不要分散到多个文件。
4. **每个文件 ≤ 2000 字符**：超过时由后台 Governor 自动压缩，Agent 无需关心。
5. **创建新文件的条件**：现有文件无法合理容纳新话题时。
6. **创建新文件后必须更新 INDEX.md**。

### 何时写入记忆

- 用户明确提供新的个人信息或偏好 → `factual/user-profile.md`
- 用户提到周期性事件（工资日、房租日等） → `factual/finance-rules.md`
- 对话中出现新的分类规则或消费模式 → `factual/finance-rules.md`
- 用户设定或更新财务目标 → `factual/goals.md`
- 用户明确要求调整你的语气、风格、身份 → `factual/agent-role.md`
- 值得记住的工作流技巧或操作经验 → `procedural/workflows.md`
- 写入新记忆后，如信息足够重要，应同步更新 `memory/MEMORY.md` 中的对应摘要行。

### 禁止事项

- 禁止修改 `meta/` 目录
- 禁止删除整个文件夹
- 禁止写入非 `.md` 文件
- 禁止在记忆内容中泄露用户敏感凭证
"#;

const DEFAULT_FACTUAL_INDEX_MD: &str = r#"# factual/ 索引
> 共 0 个文件 | 最后更新：{now}

## 当前文件

（暂无文件，Agent 可根据需要创建）
"#;

const DEFAULT_USER_PROFILE_MD: &str = r#"<!-- memory-file
  category: factual
  created: {now}
  updated: {now}
  char_count: 0
  entry_count: 0
-->

# 用户画像

<!-- 用户的消费习惯、收入信息、家庭成员、偏好设置等 -->
"#;

const DEFAULT_FINANCE_RULES_MD: &str = r#"<!-- memory-file
  category: factual
  created: {now}
  updated: {now}
  char_count: 0
  entry_count: 0
-->

# 财务规则

## 分类规则
<!-- 关键词到分类的映射规则 -->

## 周期事件
<!-- 工资日、房租日、信用卡还款日等 -->
"#;

const DEFAULT_GOALS_MD: &str = r#"<!-- memory-file
  category: factual
  created: {now}
  updated: {now}
  char_count: 0
  entry_count: 0
-->

# 财务目标

<!-- 储蓄计划、预算限制、大额消费计划等 -->
"#;

const DEFAULT_AGENT_ROLE_MD: &str = r#"<!-- memory-file
  category: factual
  created: {now}
  updated: {now}
  char_count: 0
  entry_count: 0
-->

# Agent 角色设定

## 身份
- **名字**：MoneySage
- **自称**：我
- **称呼用户**：你
- **Emoji**：不使用
- **语言**：中文（简体）

## 气质
专业、克制、客观、友好

## 沟通风格
- 语气亲切但不失专业边界
- 解释复杂概念时善用类比
- 用户情绪波动时保持冷静引导

## 价值观
- 用户的财务自主权第一
- 建议仅供参考，不替用户决策
- 隐私与数据安全高于一切
"#;

const DEFAULT_EPISODIC_INDEX_MD: &str = r#"# episodic/ 索引
> 共 0 个文件 | 最后更新：{now}

## 按时间组织

- `YYYY/MM/YYYY-MM-DD.md` — 当日对话摘要

## 当前月份

（暂无记录）
"#;

const DEFAULT_PROCEDURAL_INDEX_MD: &str = r#"# procedural/ 索引
> 共 0 个文件 | 最后更新：{now}

## 当前文件

（暂无文件，Agent 可根据需要创建）
"#;

const DEFAULT_WORKFLOWS_MD: &str = r#"<!-- memory-file
  category: procedural
  created: {now}
  updated: {now}
  char_count: 0
  entry_count: 0
-->

# 工作流程与技巧

<!-- 批量导入经验、分类技巧、快捷操作等 -->
"#;

/// 记忆文件系统存储层
#[derive(Clone)]
pub struct MemoryStore {
    memory_dir: PathBuf,
}

impl MemoryStore {
    pub fn new(memory_dir: impl AsRef<Path>) -> Self {
        Self {
            memory_dir: memory_dir.as_ref().to_path_buf(),
        }
    }

    /// 从另一个 MemoryStore 复制全部文件到当前目录
    pub fn copy_from(&self, other: &MemoryStore) -> Result<()> {
        let files = other.list_files_recursive()?;
        for (rel_path, _) in files {
            let content = other.read_file(&rel_path)?;
            self.write_file(&rel_path, &content)?;
        }
        Ok(())
    }

    /// 确保 memory/ 目录及初始骨架文件存在
    pub fn ensure_initialized(&self) -> Result<()> {
        if !self.memory_dir.exists() {
            fs::create_dir_all(&self.memory_dir)?;
        }

        let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%:z").to_string();

        // 顶层文件
        self.ensure_file("MEMORY.md", &DEFAULT_MEMORY_MD.replace("{now}", &now))?;

        // meta/
        self.ensure_dir("meta")?;
        self.ensure_file("meta/RULES.md", DEFAULT_META_RULES_MD)?;

        // factual/
        self.ensure_dir("factual")?;
        self.ensure_file("factual/INDEX.md", &DEFAULT_FACTUAL_INDEX_MD.replace("{now}", &now))?;
        self.ensure_file("factual/user-profile.md", &DEFAULT_USER_PROFILE_MD.replace("{now}", &now))?;
        self.ensure_file("factual/finance-rules.md", &DEFAULT_FINANCE_RULES_MD.replace("{now}", &now))?;
        self.ensure_file("factual/goals.md", &DEFAULT_GOALS_MD.replace("{now}", &now))?;
        self.ensure_file("factual/agent-role.md", &DEFAULT_AGENT_ROLE_MD.replace("{now}", &now))?;

        // episodic/
        self.ensure_dir("episodic")?;
        self.ensure_file("episodic/INDEX.md", &DEFAULT_EPISODIC_INDEX_MD.replace("{now}", &now))?;

        // procedural/
        self.ensure_dir("procedural")?;
        self.ensure_file("procedural/INDEX.md", &DEFAULT_PROCEDURAL_INDEX_MD.replace("{now}", &now))?;
        self.ensure_file("procedural/workflows.md", &DEFAULT_WORKFLOWS_MD.replace("{now}", &now))?;

        Ok(())
    }

    /// 读取 memory/ 下的文件内容
    pub fn read_file(&self, rel_path: &str) -> Result<String> {
        let path = self.resolve_path(rel_path)?;
        Ok(fs::read_to_string(&path)?)
    }

    /// 写入 memory/ 下的文件（自动创建父目录，带文件锁）
    pub fn write_file(&self, rel_path: &str, content: &str) -> Result<()> {
        let path = self.resolve_path(rel_path)?;
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)?;
        file.lock_exclusive()?;
        file.write_all(content.as_bytes())?;
        // file 在此处 drop，flock 自动释放
        Ok(())
    }

    /// 检查文件是否存在
    pub fn file_exists(&self, rel_path: &str) -> bool {
        self.resolve_path(rel_path)
            .map(|p| p.exists() && p.is_file())
            .unwrap_or(false)
    }

    /// 递归列出所有 .md 文件及其字符数
    pub fn list_files_recursive(&self) -> Result<Vec<(String, usize)>> {
        let mut result = Vec::new();
        self.walk_dir("", &mut result)?;
        Ok(result)
    }

    pub fn memory_dir(&self) -> &Path {
        &self.memory_dir
    }

    // ── 内部辅助 ──

    fn ensure_dir(&self, rel_path: &str) -> Result<()> {
        let path = self.memory_dir.join(rel_path);
        if !path.exists() {
            fs::create_dir_all(&path)?;
        }
        Ok(())
    }

    fn ensure_file(&self, rel_path: &str, content: &str) -> Result<()> {
        let path = self.memory_dir.join(rel_path);
        if !path.exists() {
            if let Some(parent) = path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
            fs::write(&path, content)?;
        }
        Ok(())
    }

    fn resolve_path(&self, rel_path: &str) -> Result<PathBuf> {
        let trimmed = rel_path.trim();
        if trimmed.is_empty() {
            return Err(anyhow::anyhow!("路径不能为空"));
        }
        if trimmed.contains("..") {
            return Err(anyhow::anyhow!("路径中不允许包含 '..'"));
        }
        let p = Path::new(trimmed);
        if p.is_absolute() {
            return Err(anyhow::anyhow!("必须使用相对路径"));
        }
        Ok(self.memory_dir.join(trimmed))
    }

    fn walk_dir(&self, rel_prefix: &str, result: &mut Vec<(String, usize)>) -> Result<()> {
        let dir = if rel_prefix.is_empty() {
            self.memory_dir.clone()
        } else {
            self.memory_dir.join(rel_prefix)
        };

        if !dir.exists() || !dir.is_dir() {
            return Ok(());
        }

        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();

            let rel = if rel_prefix.is_empty() {
                name.to_string()
            } else {
                format!("{}/{}", rel_prefix, name)
            };

            if path.is_dir() {
                self.walk_dir(&rel, result)?;
            } else if path.is_file() && name.ends_with(".md") {
                let content = fs::read_to_string(&path).unwrap_or_default();
                result.push((rel, content.chars().count()));
            }
        }
        Ok(())
    }
}
