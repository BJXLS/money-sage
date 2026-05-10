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

    /// 读取指定文件内容（若不存在或无法读取返回 None）
    pub fn read_file(&self, name: &str) -> Option<String> {
        let path = self.workspace_dir.join(name);
        if !path.exists() {
            return None;
        }
        fs::read_to_string(&path).ok()
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
