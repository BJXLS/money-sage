# 文件与 Shell 工具实施方案

> 目标：为 Analysis Agent 新增 `read`、`edit`、`write`、`bash` 四个 LocalTool，使其能够直接操作工作区文件和执行命令，同时保证绝对的安全性。

---

## 1. 背景与目标

当前 Analysis Agent 已具备 `query_database`、`memory_search` 等工具，但缺乏对工作区文件（`workspace/*.md`）和开发环境的操作能力。用户经常需要：

- 让 Agent 读取/修改 `AGENTS.md`、`MEMORY.md` 等工作区配置
- 让 Agent 基于工作区文件进行代码分析或批量替换
- 让 Agent 执行简单的 shell 命令（如 `git status`、`npm run build`）

本方案新增 4 个工具，核心约束：

1. **`read`/`edit`/`write`**：严格只能访问工作区目录（`app_data_dir/workspace/`）内的文件，硬编码限制，拒绝一切路径遍历尝试。
2. **`bash`**：执行用户机器上的 shell 命令，必须有多层安全防御（超时、黑名单、环境隔离、输出截断）。

---

## 2. 设计原则

| 原则 | 说明 |
|------|------|
| **沙箱优先** | 文件工具的路径解析必须 `canonicalize` 后检查前缀，确保落在 `workspace_dir` 内。 |
| **硬编码限制** | `workspace_dir` 在 `LocalToolRegistry` 初始化时注入，工具内部不允许任何方式绕过。 |
| ** fail-fast ** | 任何安全检查不通过立即返回错误，不尝试降级执行。 |
| **防御纵深** | bash 工具在命令字符串过滤、执行环境、超时监控三层设防。 |
| **最小权限** | bash 环境变量仅保留最小必要集合，清除所有可能包含敏感信息的变量。 |

---

## 3. 工作区路径沙箱（核心安全层）

所有文件工具共享一个路径解析函数：

```rust
/// 将用户传入的相对路径解析为绝对路径，并严格校验是否落在工作区内。
/// 规则：
/// 1. 拒绝空路径和绝对路径（以 / 或盘符开头）
/// 2. 拒绝包含 ".." 或空组件的路径（如 "a//b"）
/// 3. 使用 workspace_dir.join(path) 拼接
/// 4. canonicalize 后再次检查是否以 workspace_dir 为前缀
/// 5. 若文件不存在，fallback 为拼接后的路径（但用 workspace_dir.canonicalize 做前缀检查）
fn resolve_workspace_path(workspace_dir: &Path, file_path: &str) -> Result<PathBuf> {
    let trimmed = file_path.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("file_path 不能为空"));
    }

    // 拒绝绝对路径
    let p = Path::new(trimmed);
    if p.is_absolute() {
        return Err(anyhow!("file_path 必须是相对路径（基于工作区根目录）"));
    }

    // 拒绝路径遍历字符
    if trimmed.contains("..") {
        return Err(anyhow!("路径中不允许包含 '..'"));
    }
    for comp in p.components() {
        match comp {
            std::path::Component::Normal(_) => {}
            _ => return Err(anyhow!("路径包含非法组件: {:?}", comp)),
        }
    }

    let resolved = workspace_dir.join(trimmed);

    // canonicalize 已存在路径；对于新建文件，canonicalize 父目录做前缀校验
    let (check_path, check_dir) = if resolved.exists() {
        (std::fs::canonicalize(&resolved)?, std::fs::canonicalize(workspace_dir)?)
    } else {
        let parent = resolved.parent().ok_or_else(|| anyhow!("无法获取父目录"))?;
        if !parent.exists() {
            // 允许父目录不存在（write 工具会创建），但 workspace_dir 必须存在
            (resolved.clone(), std::fs::canonicalize(workspace_dir)?)
        } else {
            (std::fs::canonicalize(&resolved).unwrap_or(resolved.clone()), std::fs::canonicalize(workspace_dir)?)
        }
    };

    if !check_path.starts_with(&check_dir) {
        return Err(anyhow!("只能访问工作区内的文件"));
    }

    Ok(resolved)
}
```

**关键点**：
- `workspace_dir` 由 `LocalToolRegistry::new()` 传入，工具内部不可修改。
- 即使传入 `"a/../../b"`，也会被 `".."` 检查拦截。
- 对于符号链接，`canonicalize` 会将其解析为真实路径，如果指向工作区外，则 `starts_with` 检查失败。

---

## 4. `read` 工具设计

### 功能
读取工作区内指定文件的文本内容，以 `cat -n` 格式返回（带行号）。

### 参数 Schema
```json
{
  "type": "object",
  "properties": {
    "file_path": {
      "type": "string",
      "description": "文件在工作区内的相对路径，如 'AGENTS.md' 或 'docs/plan.md'"
    },
    "offset": {
      "type": "integer",
      "description": "起始行号（1-indexed），可选，默认从第1行开始"
    },
    "limit": {
      "type": "integer",
      "description": "最多读取多少行，可选，默认读取整个文件"
    }
  },
  "required": ["file_path"]
}
```

### 执行逻辑
1. 调用 `resolve_workspace_path()` 解析并校验路径。
2. 检查文件是否存在且为普通文件（非目录）。
3. 读取全部内容到内存（工作区文件通常 < 1MB，可控）。
4. 按行切分，根据 `offset`/`limit` 切片。
5. 每行前面加上行号（`cat -n` 格式），拼接返回。
6. 如果文件是二进制（通过内容或扩展名判断），返回提示信息而非原始字节。

### 错误处理
- 路径越界 → `"只能访问工作区内的文件"`
- 文件不存在 → `"文件不存在: <path>"`
- 路径是目录 → `"该路径是目录，无法读取"`
- 二进制文件 → `"二进制文件无法直接读取文本内容"`

---

## 5. `edit` 工具设计

### 功能
对工作区内已有文件进行精确的字符串替换。

### 参数 Schema
```json
{
  "type": "object",
  "properties": {
    "file_path": {
      "type": "string",
      "description": "文件在工作区内的相对路径"
    },
    "old_string": {
      "type": "string",
      "description": "文件中要替换的精确文本，必须完全匹配（包括缩进）"
    },
    "new_string": {
      "type": "string",
      "description": "替换后的新文本"
    },
    "replace_all": {
      "type": "boolean",
      "description": "若为 true，替换所有匹配项；默认 false（要求 old_string 唯一）"
    }
  },
  "required": ["file_path", "old_string", "new_string"]
}
```

### 执行逻辑
1. 调用 `resolve_workspace_path()` 解析并校验路径。
2. 读取文件完整内容。
3. 统计 `old_string` 出现次数：
   - 若 `replace_all == false` 且出现次数 ≠ 1，返回错误（"`old_string` 在文件中出现 N 次，不唯一"）。
   - 若 `replace_all == true`，全部替换。
4. 执行替换，写回文件（原子写入：先写临时文件，再 `rename`）。
5. 返回替换结果摘要（替换次数、文件路径）。

### 错误处理
- `old_string` 未找到 → `"未找到 old_string，请确认文本内容完全匹配"`
- `old_string` 不唯一且未设置 `replace_all` → 明确提示出现次数
- 写回失败 → 保留原始文件不动（原子写入保证）

---

## 6. `write` 工具设计

### 功能
在工作区内创建新文件或完全覆盖已有文件。

### 参数 Schema
```json
{
  "type": "object",
  "properties": {
    "file_path": {
      "type": "string",
      "description": "文件在工作区内的相对路径，如 'notes/2024-01.md'"
    },
    "content": {
      "type": "string",
      "description": "要写入的完整文件内容"
    }
  },
  "required": ["file_path", "content"]
}
```

### 执行逻辑
1. 调用 `resolve_workspace_path()` 解析并校验路径。
2. 如果父目录不存在，自动创建（`fs::create_dir_all`）。
3. 原子写入：先写入 `<file>.tmp.<random>`，成功后再 `rename` 到目标路径。
4. 返回写入结果（字节数、文件路径）。

### 与现有 `WorkspaceManager::write_file` 的区别
- 现有方法限制在白名单（只允许 `AGENTS.md` 等 7 个文件名）。
- 新工具**放宽到工作区内任意文件**，但**保持同样的路径沙箱检查**。
- 现有方法保留给前端 UI 使用（如配置面板），新工具仅给 Agent 使用。

---

## 7. `bash` 工具设计（多层安全防御）

### 功能
在工作区目录下执行一条 bash 命令，返回 stdout/stderr/exit_code。

### 参数 Schema
```json
{
  "type": "object",
  "properties": {
    "command": {
      "type": "string",
      "description": "要执行的 bash 命令字符串"
    },
    "description": {
      "type": "string",
      "description": "简要说明这条命令的用途（帮助理解和审计）"
    },
    "timeout_secs": {
      "type": "integer",
      "description": "超时时间（秒），可选，默认 30，最大 120"
    }
  },
  "required": ["command"]
}
```

### 安全架构（五层防御）

#### 第一层：命令字符串黑名单（静态过滤）
在将命令送入 `std::process::Command` 前，进行字符串模式匹配。以下模式**直接拒绝**：

```rust
const DENIED_PATTERNS: &[&str] = &[
    "rm -rf /",           // 删除根目录
    "rm -rf /*",          // 删除根目录内容
    ":(){ :|:& };:",      // fork bomb
    "> /dev/sda",         // 覆写磁盘
    "mkfs",               // 格式化文件系统
    "dd if=/dev/zero of=/dev",
    "chmod 777 /",
    "chown root:root /",
];

const DENIED_REGEX_PATTERNS: &[&str] = &[
    r"curl\s+.*\|\s*(sh|bash)",   // curl | sh
    r"wget\s+.*\|\s*(sh|bash)",  // wget | bash
    r"sudo\s+",                    // sudo 提权
    r"su\s+-",
];
```

**注意**：不过度限制正常的 `rm ./temp.txt`、`npm install`、`cargo build`、`git push` 等操作。

#### 第二层：执行环境隔离
```rust
let mut cmd = std::process::Command::new("bash");
cmd.arg("-c").arg(command)
    .current_dir(&workspace_dir)          // 锁定工作目录
    .env_clear()                           // 清空所有环境变量
    .env("PATH", "/usr/local/bin:/usr/bin:/bin:/opt/homebrew/bin") // 最小 PATH
    .env("HOME", home_dir)
    .env("TMPDIR", tmp_dir)
    .env("LANG", "en_US.UTF-8")
    .env("SHELL", "/bin/bash")
    .stdin(std::process::Stdio::null())    // 禁止交互输入
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::piped());
```

- `env_clear()` 清除所有环境变量，防止泄露 `API_KEY`、`AWS_SECRET` 等敏感信息。
- `stdin(null())` 确保命令不会阻塞等待用户输入。

#### 第三层：超时强制终止
```rust
let timeout = timeout_secs.clamp(1, 120);
let result = tokio::time::timeout(
    Duration::from_secs(timeout),
    tokio::process::Command::from(cmd).output()
).await;

match result {
    Ok(Ok(output)) => { /* 正常处理 */ }
    Ok(Err(e)) => return Err(anyhow!("命令执行失败: {}", e)),
    Err(_) => return Err(anyhow!("命令执行超时（{} 秒），已强制终止", timeout)),
}
```

#### 第四层：输出截断
```rust
const MAX_OUTPUT_BYTES: usize = 500 * 1024; // 500KB

let stdout = String::from_utf8_lossy(&output.stdout);
let stderr = String::from_utf8_lossy(&output.stderr);

let stdout = if stdout.len() > MAX_OUTPUT_BYTES {
    format!("{}\n...[输出已截断，共 {} 字节]", &stdout[..MAX_OUTPUT_BYTES], stdout.len())
} else {
    stdout.to_string()
};
```

防止 LLM 因超大输出耗尽上下文窗口，也防止恶意命令通过 stdout 洪水攻击。

#### 第五层：审计日志
每条 bash 命令执行都记录结构化日志：
```rust
eprintln!("[BashTool] command={} | desc={} | timeout={} | exit_code={} | stdout_len={} | stderr_len={}",
    command, description, timeout, exit_code, stdout.len(), stderr.len());
```

### 执行逻辑
1. 参数校验：`command` 非空，`timeout_secs` 在 [1, 120] 范围内。
2. 黑名单过滤。
3. 构建隔离的 `tokio::process::Command`。
4. `tokio::time::timeout` 包裹执行。
5. 收集 stdout/stderr/exit_code，截断超长输出。
6. 返回 JSON：
   ```json
   {
     "success": true,
     "exit_code": 0,
     "stdout": "...",
     "stderr": "",
     "truncated": false,
     "execution_time_ms": 1234
   }
   ```

### 错误处理
- 命中黑名单 → `"命令包含危险模式，已拒绝执行"`
- 超时 → `"命令执行超时，已强制终止"`
- 非零退出码 → 仍返回 stdout/stderr，但 `success: false`，让 LLM 自行判断

---

## 8. Rust 实现架构

### 8.1 文件结构

```
src-tauri/src/ai/tools/
├── mod.rs                    # LocalTool trait / Registry（修改）
├── get_schema.rs
├── query_database.rs
├── quick_note_parse.rs
├── quick_note_save.rs
├── memory_search.rs
├── memory_fact_upsert.rs
├── file_read.rs              # 新增
├── file_edit.rs              # 新增
├── file_write.rs             # 新增
├── bash_exec.rs              # 新增
└── workspace_path.rs         # 新增：共享路径沙箱函数
```

### 8.2 `workspace_path.rs`（共享模块）

```rust
use std::path::{Path, PathBuf};
use anyhow::{anyhow, Result};

pub fn resolve_workspace_path(workspace_dir: &Path, file_path: &str) -> Result<PathBuf> {
    // ... 详见第 3 节 ...
}

pub fn is_text_file(path: &Path) -> bool {
    // 基于扩展名判断：允许 .md .rs .js .ts .json .txt .yaml .toml 等
    // 拒绝 .exe .dll .so .dylib .bin .jpg .png .pdf 等
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let ext = ext.to_lowercase();
        let binary_exts = ["exe", "dll", "so", "dylib", "bin", "o", "a",
                           "jpg", "jpeg", "png", "gif", "bmp", "webp",
                           "mp3", "mp4", "avi", "mov", "pdf", "zip", "gz", "tar", "7z"];
        if binary_exts.contains(&ext.as_str()) {
            return false;
        }
    }
    true
}
```

### 8.3 工具实现示例（`file_read.rs`）

```rust
use async_trait::async_trait;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::path::PathBuf;

use super::{LocalTool, workspace_path::resolve_workspace_path};

pub struct FileReadTool {
    workspace_dir: PathBuf,
}

impl FileReadTool {
    pub fn new(workspace_dir: PathBuf) -> Self {
        Self { workspace_dir }
    }
}

#[async_trait]
impl LocalTool for FileReadTool {
    fn name(&self) -> &str { "file_read" }
    fn description(&self) -> &str {
        "读取工作区内的文件内容。只能访问工作区目录下的文件，禁止路径遍历。"
    }
    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_path": { "type": "string", "description": "工作区内的相对路径，如 'AGENTS.md'" },
                "offset": { "type": "integer", "description": "起始行号（1-indexed），可选" },
                "limit": { "type": "integer", "description": "最大读取行数，可选" }
            },
            "required": ["file_path"]
        })
    }
    async fn execute(&self, arguments: Value) -> Result<String> {
        let file_path = arguments["file_path"].as_str().ok_or_else(|| anyhow!("缺少 file_path"))?;
        let resolved = resolve_workspace_path(&self.workspace_dir, file_path)?;

        if !resolved.exists() {
            return Err(anyhow!("文件不存在: {}", file_path));
        }
        if resolved.is_dir() {
            return Err(anyhow!("该路径是目录，无法读取: {}", file_path));
        }

        let content = tokio::fs::read_to_string(&resolved).await
            .map_err(|e| anyhow!("读取文件失败: {}", e))?;

        let offset = arguments["offset"].as_u64().unwrap_or(1).saturating_sub(1) as usize;
        let limit = arguments["limit"].as_u64().unwrap_or(u64::MAX) as usize;

        let lines: Vec<&str> = content.lines().collect();
        let end = (offset + limit).min(lines.len());
        let slice = if offset < lines.len() { &lines[offset..end] } else { &[] };

        let mut result = String::new();
        for (i, line) in slice.iter().enumerate() {
            result.push_str(&format!("{:6}\t{}\n", offset + i + 1, line));
        }

        Ok(result)
    }
}
```

### 8.4 `LocalToolRegistry` 修改

在 `tools/mod.rs` 的 `LocalToolRegistry::new()` 中新增工具注册：

```rust
use std::path::PathBuf;

pub struct LocalToolRegistry {
    tools: Vec<Box<dyn LocalTool>>,
}

impl LocalToolRegistry {
    pub fn new(
        pool: SqlitePool,
        session_id: Option<String>,
        token_recorder: Option<Arc<TokenUsageRecorder>>,
        memory: Arc<MemoryFacade>,
        workspace_dir: PathBuf,          // <-- 新增参数
    ) -> Self {
        let mut registry = Self { tools: Vec::new() };

        // 现有工具...
        registry.tools.push(Box::new(get_schema::GetDatabaseSchemaTool::new(pool.clone())));
        registry.tools.push(Box::new(query_database::QueryDatabaseTool::new(pool.clone())));
        // ...

        // 新增文件与 bash 工具
        registry.tools.push(Box::new(file_read::FileReadTool::new(workspace_dir.clone())));
        registry.tools.push(Box::new(file_edit::FileEditTool::new(workspace_dir.clone())));
        registry.tools.push(Box::new(file_write::FileWriteTool::new(workspace_dir.clone())));
        registry.tools.push(Box::new(bash_exec::BashExecTool::new(workspace_dir.clone())));

        registry
    }
    // ...
}
```

### 8.5 `lib.rs` 调用点修改

在 `send_analysis_message_stream` 中（约 1428 行）：

```rust
let tool_registry = LocalToolRegistry::new(
    db_state.db.pool.clone(),
    Some(sid.clone()),
    Some(db_state.token_recorder.clone()),
    db_state.memory.clone(),
    db_state.workspace.workspace_dir().to_path_buf(),  // <-- 新增
);
```

**注意**：`db_state` 中需要能够访问到 `WorkspaceManager`。当前代码中 `workspace_manager` 在 `setup()` 中创建但未存入 `DatabaseState`。需要：

1. 在 `DatabaseState` 中添加 `pub workspace: WorkspaceManager` 字段；
2. 或在 `setup()` 中将 `workspace_manager` 存入 State。

建议把 `WorkspaceManager` 存到 `AppState` 或现有的 `DatabaseState` 中，因为 `workspace_dir()` 已经存在。

---

## 9. System Prompt 更新

在 `analysis.rs` 的 `build_tool_guide()` 中，追加新工具的说明：

```rust
p.push_str(
    "\n## 文件与命令工具使用指南\n\n\
     你还可以直接操作工作区文件和执行命令：\n\n\
     ### file_read\n\
     - 读取工作区内的文件内容（只能访问 workspace/ 目录下）。\n\
     - file_path 必须是相对路径，如 'AGENTS.md'、'docs/plan.md'。\n\
     - 支持 offset/limit 分段读取大文件。\n\n\
     ### file_edit\n\
     - 对工作区已有文件进行精确字符串替换。\n\
     - old_string 必须与文件内容**完全匹配**（包括缩进）。\n\
     - 修改前建议先用 file_read 查看当前内容。\n\
     - 如果 old_string 出现多次，可设置 replace_all: true 全部替换。\n\n\
     ### file_write\n\
     - 在工作区内创建新文件或完全覆盖已有文件。\n\
     - 会自动创建不存在的父目录。\n\
     - 覆盖已有文件时请谨慎。\n\n\
     ### bash\n\
     - 在工作区目录下执行 shell 命令。\n\
     - 支持 git、npm、cargo、grep、find 等常规命令。\n\
     - 命令有超时保护（默认 30 秒）和黑名单过滤，极端危险命令会被拒绝。\n\
     - 环境变量已清理，不包含敏感信息。\n"
);
```

---

## 10. 前端交互

当前工具调用状态通过 `analysis-stream-chunk` 事件透传：

```rust
ToolStatusPayload {
    tool_name: "file_edit".to_string(),
    status: "calling".to_string(),
    description: Some("正在编辑: AGENTS.md".to_string()),
    tool_input: Some(json!({ "file_path": "AGENTS.md" }).to_string()),
    tool_output: None,
}
```

新增工具天然复用这套机制，前端无需改动即可看到 "正在调用 file_read"、"正在调用 bash" 等状态。

**可选增强**：
- bash 工具的 `stderr` 如果非空，前端可以用红色高亮展示。
- file_edit/file_write 成功后，可以触发工作区文件列表刷新。

---

## 11. 风险分析与缓解

| 风险 | 可能性 | 影响 | 缓解措施 |
|------|--------|------|----------|
| 路径遍历突破沙箱 | 低 | 高 | `resolve_workspace_path` 硬编码检查：`..` 拦截 + `canonicalize` + `starts_with` 三重校验。 |
| bash 执行 `rm -rf /` | 低 | 极高 | 字符串黑名单直接拒绝；`env_clear` + `current_dir` 锁定。 |
| bash 导致资源耗尽 | 中 | 中 | `tokio::time::timeout` 30 秒强制终止；输出截断 500KB。 |
| bash 泄露环境变量 | 低 | 高 | `env_clear()` 只保留最小 PATH/HOME/TMPDIR。 |
| LLM 误 edit 破坏文件 | 中 | 中 | `file_edit` 要求精确匹配，不会模糊替换；`file_write` 原子写入。 |
| bash 执行网络攻击 | 低 | 中 | 不禁止 curl/wget（开发需要），但禁止 `curl \| sh` 管道模式。 |
| 符号链接逃逸 | 低 | 高 | `canonicalize` 解析 symlink 真实路径后再做 `starts_with` 检查。 |

---

## 12. 实施步骤 checklist

- [ ] 新增 `tools/workspace_path.rs`（共享路径沙箱函数）
- [ ] 新增 `tools/file_read.rs`
- [ ] 新增 `tools/file_edit.rs`
- [ ] 新增 `tools/file_write.rs`
- [ ] 新增 `tools/bash_exec.rs`
- [ ] 修改 `tools/mod.rs`：更新 `LocalToolRegistry::new` 签名，注册 4 个新工具
- [ ] 修改 `lib.rs`：把 `workspace_dir` 传入 `LocalToolRegistry::new`
- [ ] 确保 `DatabaseState` / `AppState` 包含 `WorkspaceManager`（或至少 `workspace_dir`）
- [ ] 修改 `analysis.rs`：`build_tool_guide()` 追加新工具说明
- [ ] 编译测试：验证路径遍历被拦截、bash 超时生效
- [ ] 集成测试：让 Analysis Agent 执行 `file_read(AGENTS.md)` → `bash(ls)` 流程
