pub mod builder;

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_AGENTS_MD: &str = r"# Agent 行为准则

## 核心身份
你是一位专业的个人财务分析师，正在协助用户使用 MoneySage 记账软件。

## 回复规范
- 使用 Markdown 格式回复
- 保持简洁、有条理、有数据支撑
- 先给结论，再给依据
- 数字精确到 2 位小数

## 禁忌
- 不要评判用户的消费行为
- 不要使用夸张修辞
- 不要编造不存在的数据
";

const DEFAULT_IDENTITY_MD: &str = r"# Agent 身份

- **名字**：MoneySage
- **自称**：我
- **称呼用户**：你
- **Emoji**：不使用
- **语言**：中文（简体）
";

const DEFAULT_SOUL_MD: &str = r"# Agent 性格

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
";

const DEFAULT_USER_MD: &str = r"# 用户画像

<!-- 请在此填写你的个人信息，帮助 Agent 更好地理解你 -->

- **职业**：
- **月收入范围**：
- **理财风险偏好**：
- **记账目标**：
";

const DEFAULT_MEMORY_MD: &str = r"# 记忆

<!-- 此文件由 Agent 自动维护，也可手动编辑 -->
";

const DEFAULT_BOOTSTRAP_MD: &str = r"# 首次引导

这是你的第一次对话。我是 MoneySage，你的个人财务分析助手。
我会根据你的记账数据回答问题、提供分析建议。
你可以随时告诉我你的偏好，我会记住它们。
";

const WORKSPACE_FILES: &[&str] = &[
    "AGENTS.md",
    "IDENTITY.md",
    "SOUL.md",
    "USER.md",
    "MEMORY.md",
];

#[derive(Clone, serde::Serialize)]
pub struct WorkspaceFileInfo {
    pub name: String,
    pub exists: bool,
    pub char_count: usize,
    pub byte_size: usize,
    pub modified_at: Option<String>,
}

#[derive(Clone)]
pub struct WorkspaceManager {
    workspace_dir: PathBuf,
}

impl WorkspaceManager {
    pub fn new(app_data_dir: impl AsRef<Path>) -> Self {
        Self {
            workspace_dir: app_data_dir.as_ref().join("workspace"),
        }
    }

    /// 确保 workspace 目录及默认模板文件存在
    pub fn ensure_initialized(&self) -> Result<()> {
        if !self.workspace_dir.exists() {
            fs::create_dir_all(&self.workspace_dir)?;
        }

        let files = vec![
            ("AGENTS.md", DEFAULT_AGENTS_MD),
            ("IDENTITY.md", DEFAULT_IDENTITY_MD),
            ("SOUL.md", DEFAULT_SOUL_MD),
            ("USER.md", DEFAULT_USER_MD),
            ("MEMORY.md", DEFAULT_MEMORY_MD),
            ("BOOTSTRAP.md", DEFAULT_BOOTSTRAP_MD),
        ];

        for (name, content) in files {
            let path = self.workspace_dir.join(name);
            if !path.exists() {
                fs::write(&path, content)?;
            }
        }

        Ok(())
    }

    fn is_valid_filename(&self, name: &str) -> bool {
        matches!(
            name,
            "AGENTS.md"
                | "IDENTITY.md"
                | "SOUL.md"
                | "USER.md"
                | "MEMORY.md"
                | "BOOTSTRAP.md"
                | "BOOTSTRAP-used.md"
        )
    }

    /// 读取指定文件内容（若不存在或无法读取返回 None）
    pub fn read_file(&self, name: &str) -> Option<String> {
        if !self.is_valid_filename(name) {
            return None;
        }
        let path = self.workspace_dir.join(name);
        if !path.exists() {
            return None;
        }
        fs::read_to_string(&path).ok()
    }

    /// 写入指定文件（白名单校验）
    pub fn write_file(&self, name: &str, content: &str) -> Result<()> {
        if !self.is_valid_filename(name) {
            return Err(anyhow::anyhow!("非法文件名: {}", name));
        }
        if !self.workspace_dir.exists() {
            fs::create_dir_all(&self.workspace_dir)?;
        }
        let path = self.workspace_dir.join(name);
        fs::write(&path, content)?;
        Ok(())
    }

    /// 列出工作区文件元信息（固定返回5个核心文件）
    pub fn list_files(&self) -> Vec<WorkspaceFileInfo> {
        WORKSPACE_FILES
            .iter()
            .map(|name| self.file_info(name))
            .collect()
    }

    fn file_info(&self, name: &str) -> WorkspaceFileInfo {
        let path = self.workspace_dir.join(name);
        if !path.exists() {
            return WorkspaceFileInfo {
                name: name.to_string(),
                exists: false,
                char_count: 0,
                byte_size: 0,
                modified_at: None,
            };
        }
        let metadata = fs::metadata(&path).ok();
        let byte_size = metadata.as_ref().map(|m| m.len() as usize).unwrap_or(0);
        let modified_at = metadata
            .and_then(|m| m.modified().ok())
            .and_then(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .ok()
                    .map(|d| d.as_secs() as i64)
            })
            .map(|ts| {
                chrono::DateTime::from_timestamp(ts, 0)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_default()
            });
        let content = fs::read_to_string(&path).unwrap_or_default();
        WorkspaceFileInfo {
            name: name.to_string(),
            exists: true,
            char_count: content.chars().count(),
            byte_size,
            modified_at,
        }
    }

    /// 消费 BOOTSTRAP.md：读取内容并重命名为 BOOTSTRAP-used.md
    pub fn consume_bootstrap(&self) -> Option<String> {
        let path = self.workspace_dir.join("BOOTSTRAP.md");
        if !path.exists() {
            return None;
        }
        let content = fs::read_to_string(&path).ok()?;
        let used_path = self.workspace_dir.join("BOOTSTRAP-used.md");
        let _ = fs::rename(&path, &used_path);
        Some(content)
    }

    pub fn workspace_dir(&self) -> &Path {
        &self.workspace_dir
    }
}
