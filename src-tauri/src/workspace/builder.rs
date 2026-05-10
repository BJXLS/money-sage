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

        // 5. USER.md
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
                    Some(self.truncate(trimmed))
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
