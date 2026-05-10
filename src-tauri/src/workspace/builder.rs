use super::WorkspaceManager;

pub struct SystemPromptBuilder<'a> {
    workspace: &'a WorkspaceManager,
    max_chars_per_file: usize,
    trunc_marker: &'a str,
}

impl<'a> SystemPromptBuilder<'a> {
    pub fn new(workspace: &'a WorkspaceManager) -> Self {
        Self {
            workspace,
            max_chars_per_file: 2000,
            trunc_marker: "（内容过多，截断）",
        }
    }

    /// 构建 AnalysisAgent 的完整 system prompt
    pub fn build_analysis_prompt(&self, tool_guide: &str, time_context: &str) -> String {
        let mut parts: Vec<String> = Vec::new();

        // 1. BOOTSTRAP.md（一次性，最前面）
        if let Some(bootstrap) = self.workspace.consume_bootstrap() {
            if !bootstrap.trim().is_empty() {
                parts.push(bootstrap);
            }
        }

        // 2. AGENTS.md（最稳定）
        if let Some(section) = self.read_and_format("AGENTS.md", false) {
            parts.push(section);
        }

        // 3. IDENTITY.md
        if let Some(section) = self.read_and_format("IDENTITY.md", false) {
            parts.push(section);
        }

        // 4. SOUL.md
        if let Some(section) = self.read_and_format("SOUL.md", false) {
            parts.push(section);
        }

        // 5. 工作区文件用途与可修改性说明
        parts.push(self.build_workspace_files_guide());

        // 6. USER.md
        if let Some(section) = self.read_and_format("USER.md", false) {
            parts.push(section);
        }

        // 6. MEMORY.md（允许缺失，为空也跳过）
        if let Some(section) = self.read_and_format("MEMORY.md", true) {
            if !section.trim().is_empty() {
                parts.push(section);
            }
        }

        // 7. 动态内容：工具指南 + 当前时间（放在最后，减少前缀变动）
        if !tool_guide.is_empty() {
            parts.push(tool_guide.to_string());
        }
        if !time_context.is_empty() {
            parts.push(time_context.to_string());
        }

        parts.join("\n\n")
    }

    /// 读取单个文件并应用截断/占位符逻辑
    /// allow_missing = true 时，文件不存在返回 None（不输出占位符）
    fn read_and_format(&self, name: &str, allow_missing: bool) -> Option<String> {
        match self.workspace.read_file(name) {
            Some(content) => {
                let trimmed = content.trim();
                if trimmed.is_empty() {
                    if allow_missing {
                        None
                    } else {
                        Some(format!("<!-- {} 内容为空 -->", name))
                    }
                } else {
                    Some(format!("<src:{}>\n{}", name, self.truncate(trimmed)))
                }
            }
            None => {
                if allow_missing {
                    None
                } else {
                    Some(format!("<!-- {} 未找到 -->", name))
                }
            }
        }
    }

    /// 构建工作区文件用途与可修改性说明（硬编码，插入在 SOUL.md 之后、USER.md 之前）
    fn build_workspace_files_guide(&self) -> String {
        "## 工作区文件说明\n\n\
         以下工作区文件被注入到当前上下文中。它们的用途和可修改性如下：\n\n\
         - **AGENTS.md / IDENTITY.md / SOUL.md**：系统配置，通常由用户手动维护，Agent 不建议修改。\n\
         - **USER.md**：用户画像与偏好。当用户明确提供或更新个人信息、记账偏好时，Agent **应当**使用 `file_edit` 或 `file_write` 更新此文件。\n\
         - **MEMORY.md**：长期记忆与规律总结。当对话中出现值得跨会话记住的财务规律、消费模式、目标时，Agent **应当**使用 `file_edit` 或 `file_write` 更新此文件。\n\
         - **注意**：修改前建议先用 `file_read` 查看当前内容；只写入用户明确提供或高度可信的信息，禁止编造。"
            .to_string()
    }

    /// 截断逻辑：严格保证总长度 <= max_chars_per_file
    fn truncate(&self, content: &str) -> String {
        let char_count = content.chars().count();
        if char_count <= self.max_chars_per_file {
            return content.to_string();
        }

        let marker_len = self.trunc_marker.chars().count();
        let available = self.max_chars_per_file.saturating_sub(marker_len);
        let head_len = available * 3 / 4;
        let tail_len = available.saturating_sub(head_len);

        let head: String = content.chars().take(head_len).collect();
        let tail: String = content
            .chars()
            .skip(char_count.saturating_sub(tail_len))
            .collect();

        format!("{}{}{}", head, self.trunc_marker, tail)
    }
}
