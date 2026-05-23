use super::WorkspaceManager;

pub struct SystemPromptBuilder<'a> {
    workspace: &'a WorkspaceManager,
    memory_store: Option<&'a crate::memory::v3::MemoryStore>,
    max_chars_per_file: usize,
    trunc_marker: &'a str,
}

impl<'a> SystemPromptBuilder<'a> {
    pub fn new(workspace: &'a WorkspaceManager) -> Self {
        Self {
            workspace,
            memory_store: None,
            max_chars_per_file: 2000,
            trunc_marker: "（内容过多，截断）",
        }
    }

    pub fn with_memory_store(mut self, store: &'a crate::memory::v3::MemoryStore) -> Self {
        self.memory_store = Some(store);
        self
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

        // 3. Agent 角色设定（factual/agent-role.md）
        if let Some(store) = self.memory_store {
            if let Ok(role) = store.read_file("factual/agent-role.md") {
                let trimmed = role.trim();
                if !trimmed.is_empty() && trimmed.len() > 30 {
                    parts.push(format!("<src:factual/agent-role.md>\n{}\n\n[System note: Agent 角色设定。当用户明确要求调整你的语气、风格、身份时，通过 file_edit/file_write 更新 factual/agent-role.md。]", self.truncate(trimmed)));
                }
            }
        }

        // 4. 工作区文件用途与可修改性说明
        parts.push(self.build_workspace_files_guide());

        // 5. V3 记忆快照（memory/MEMORY.md）
        if let Some(store) = self.memory_store {
            let loader = crate::memory::v3::SnapshotLoader::new(store.clone());
            let snapshot = loader.load();
            if !snapshot.trim().is_empty() {
                parts.push(format!("## 记忆 (Memory Snapshot)\n{}\n\n[System note: 以上是跨会话记忆的快照。当前会话中你新学到的信息，应通过 file_edit/file_write 写入 memory/ 目录，同时更新 memory/MEMORY.md 快照文件。]", snapshot));
            }

            // 6. 用户画像（factual/user-profile.md）
            if let Ok(profile) = store.read_file("factual/user-profile.md") {
                let trimmed = profile.trim();
                if !trimmed.is_empty() && trimmed.len() > 20 {
                    parts.push(format!("<src:factual/user-profile.md>\n{}\n\n[System note: 用户画像信息。当用户明确更新个人信息时，通过 file_edit/file_write 更新 factual/user-profile.md。]", self.truncate(trimmed)));
                }
            }
        }

        // 7. workspace/MEMORY.md（允许缺失，为空也跳过）
        if let Some(section) = self.read_and_format("MEMORY.md", true) {
            if !section.trim().is_empty() {
                parts.push(section);
            }
        }

        // 8. 动态内容：工具指南 + 当前时间（放在最后，减少前缀变动）
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

    /// 构建工作区文件用途与可修改性说明
    /// 优先从 memory/meta/RULES.md 读取，不存在则使用 fallback
    fn build_workspace_files_guide(&self) -> String {
        let mut guide = String::new();
        guide.push_str("## 工作区文件说明\n\n");
        guide.push_str("以下文件被注入到当前上下文中。它们的用途和可修改性如下：\n\n");
        guide.push_str("- **AGENTS.md**：系统行为准则，通常由用户手动维护，Agent 不建议修改。\n\n");

        // 动态加载 meta/RULES.md
        if let Some(store) = self.memory_store {
            if let Ok(rules) = store.read_file("meta/RULES.md") {
                let trimmed = rules.trim();
                if !trimmed.is_empty() {
                    guide.push_str(trimmed);
                    guide.push_str("\n\n");
                    return guide;
                }
            }
        }

        // fallback：硬编码核心规则
        guide.push_str("## 记忆系统写入规范\n\n");
        guide.push_str("memory/ 目录是跨会话记忆的真源。写入时必须遵守以下规范：\n\n");
        guide.push_str("1. **追加新行，不删除旧行**：保留完整演进历史。\n");
        guide.push_str("2. **新条目加在顶部**（同一 `##` 标题下），保持时间倒序。\n");
        guide.push_str("3. **在同一标题下追加**：保持话题聚合。\n");
        guide.push_str("4. **每个文件 ≤ 2000 字符**：超过时由后台自动压缩。\n");
        guide.push_str("5. **创建新文件后必须更新 INDEX.md**。\n\n");
        guide.push_str("### 何时写入记忆\n\n");
        guide.push_str("- 用户明确提供新的个人信息或偏好 → `factual/user-profile.md`\n");
        guide.push_str("- 用户提到周期性事件 → `factual/finance-rules.md`\n");
        guide.push_str("- 新的分类规则或消费模式 → `factual/finance-rules.md`\n");
        guide.push_str("- 用户设定或更新财务目标 → `factual/goals.md`\n");
        guide.push_str("- 用户调整语气、风格、身份 → `factual/agent-role.md`\n");
        guide.push_str("- 工作流技巧或操作经验 → `procedural/workflows.md`\n\n");
        guide.push_str("### 禁止事项\n\n");
        guide.push_str("- 禁止修改 `meta/` 目录\n");
        guide.push_str("- 禁止删除整个文件夹\n");
        guide.push_str("- 禁止写入非 `.md` 文件\n");
        guide.push_str("- 禁止泄露用户敏感凭证\n\n");
        guide.push_str("- **注意**：修改前建议先用 `file_read` 查看当前内容；只写入用户明确提供或高度可信的信息，禁止编造。\n");

        guide
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
