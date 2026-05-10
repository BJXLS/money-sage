# Workspace 文件注入与 Agent 可修改性设计

## 目标

1. System Prompt 中明确告知 Agent：`USER.md` 和 `MEMORY.md` 的用途，以及 Agent 可以根据对话内容主动修改它们。
2. 每个被读入 System Prompt 的 markdown 文件，内容前都附加来源标签，让 Agent 明确知道这段内容的出处。

---

## 1. 现有机制回顾

`SystemPromptBuilder::build_analysis_prompt()` 按固定顺序读取工作区 markdown 文件并拼接：

1. `BOOTSTRAP.md`（一次性，消费后重命名）
2. `AGENTS.md`
3. `IDENTITY.md`
4. `SOUL.md`
5. `USER.md`
6. `MEMORY.md`
7. 动态内容：`tool_guide` + `time_context`

当前问题：
- 文件内容直接裸拼进 prompt，没有来源标记，Agent 可能分不清 "这段文字来自 AGENTS.md 还是 USER.md"。
- 没有明确告诉 Agent："USER.md 和 MEMORY.md 是你可以修改的"。

---

## 2. 来源标签设计

### 方案：行内前缀标签（推荐）

在每个文件内容前加一行简单的 XML 风格标签，内容后可选加结束标签（若内容可能被截断，结束标签可省略）。

```xml
<src:USER.md>
- **职业**：工程师
- **记账目标**：控制月度餐饮支出在 3000 以内
```

或更明确的语义标签：

```xml
<workspace-file name="USER.md">
- **职业**：工程师
</workspace-file>
```

**推荐用 `<src:NAME.md>`**，理由：
- 极简，不增加太多 token
- LLM 能一眼识别来源
- 不干扰 markdown 渲染（XML 标签在 markdown 中会被当成原始 HTML，但这里只在 system prompt 中使用）

### 截断场景处理

`SystemPromptBuilder` 有 `max_chars_per_file = 2000` 截断逻辑。截断后内容不应有未闭合标签，所以采用**前缀单行标签**而非包裹标签最稳妥：

```
<src:MEMORY.md>
2024-01 用户设定月度预算 5000...
...
（内容过多，截断）
...2024-12 用户偏好使用 emoji 分类
```

无需结束标签，避免截断后标签不完整。

---

## 3. 文件用途与可修改性说明

在 System Prompt 中，紧接在 `SOUL.md` 之后、`USER.md` 之前，插入一段策略说明。这段说明**不属于任何 markdown 文件**，由 `build_analysis_prompt()` 硬编码生成。

### 建议插入的文本

```markdown
## 工作区文件说明

以下工作区文件被注入到当前上下文中。它们的用途和可修改性如下：

- **AGENTS.md / IDENTITY.md / SOUL.md**：系统配置，通常由用户手动维护，Agent 不建议修改。
- **USER.md**：用户画像与偏好。当用户明确提供或更新个人信息、记账偏好时，Agent **应当**使用 `file_edit` 或 `file_write` 更新此文件。
- **MEMORY.md**：长期记忆与规律总结。当对话中出现值得跨会话记住的财务规律、消费模式、目标时，Agent **应当**使用 `file_edit` 或 `file_write` 更新此文件。
- **注意**：修改前建议先用 `file_read` 查看当前内容；只写入用户明确提供或高度可信的信息，禁止编造。
```

### 为什么放在 SOUL.md 之后、USER.md 之前？

因为 AGENTS/IDENTITY/SOUL 是"身份与行为定义"，属于系统层；USER/MEMORY 是"用户数据层"。在两层之间插入"说明层"，逻辑最清晰。

---

## 4. 完整 System Prompt 结构（修改后）

```
[BOOTSTRAP.md 内容]

<src:AGENTS.md>
[AGENTS.md 内容]

<src:IDENTITY.md>
[IDENTITY.md 内容]

<src:SOUL.md>
[SOUL.md 内容]

## 工作区文件说明
（硬编码的用途与可修改性说明，见第 3 节）

<src:USER.md>
[USER.md 内容]

<src:MEMORY.md>
[MEMORY.md 内容]

## 工具使用策略
（tool_guide 动态内容）

## 当前时间
（time_context 动态内容）
```

---

## 5. 实现点

### 修改 `workspace/builder.rs`

1. **`read_and_format()`**：在返回内容前加 `<src:NAME.md>\n` 前缀。

   ```rust
   fn read_and_format(&self, name: &str, allow_missing: bool) -> Option<String> {
       match self.workspace.read_file(name) {
           Some(content) => {
               let trimmed = content.trim();
               if trimmed.is_empty() {
                   // ...
               } else {
                   Some(format!("<src:{}>\n{}", name, self.truncate(trimmed)))
               }
           }
           // ...
       }
   }
   ```

2. **`build_analysis_prompt()`**：在 SOUL.md 和 USER.md 之间插入硬编码说明。

   ```rust
   // 4. SOUL.md
   if let Some(section) = self.read_and_format("SOUL.md", false) {
       parts.push(section);
   }

   // 5. 文件用途说明（硬编码）
   parts.push(self.build_workspace_files_guide());

   // 6. USER.md
   if let Some(section) = self.read_and_format("USER.md", false) {
       parts.push(section);
   }
   ```

3. 新增 `build_workspace_files_guide()` 私有方法，返回第 3 节的说明文本。

### 不修改的内容

- `workspace/mod.rs` 的 `WorkspaceManager` 保持不变。
- `analysis.rs` 的 `build_tool_guide()` 保持不变（它已经足够精简）。
- 工具本身的 `description()` 和 schema 保持不变。

---

## 6. 风险与缓解

| 风险 | 缓解 |
|------|------|
| `<src:>` 标签增加 token 消耗 | 每文件只增加 1 行（约 10-20 tokens），6 个文件总计 < 100 tokens，可忽略 |
| Agent 误改 AGENTS.md/IDENTITY.md | 说明中明确标注这些文件"不建议修改"；同时 `file_edit` 的精确匹配机制也限制了误操作范围 |
| Agent 过度写入 MEMORY.md | 说明中强调"只写入用户明确提供或高度可信的信息"；`file_edit` 要求精确匹配 old_string，不会随意追加 |
| 截断导致 `<src:>` 和内容分离 | `read_and_format()` 中标签在截断逻辑之外，不受 `max_chars_per_file` 影响 |

---

## 7. 与现有记忆系统的分工

当前已有 `memory_search` / `memory_fact_upsert` 工具（基于数据库的结构化记忆）。

- **数据库记忆（memory_facts）**：结构化、可检索、适合事实型记录（如"用户每月餐饮预算 3000"）。
- **MEMORY.md**：非结构化、适合段落式总结（如"用户 2024 年的消费习惯总结"）。

两者不冲突。System Prompt 中只需说明 "MEMORY.md 是 Agent 可修改的工作区文件"，具体写入策略由 Agent 自行决定（用 memory_fact_upsert 还是 file_edit，取决于内容类型）。
