# MoneySage

智能记账桌面应用 — AI 快速记账、对话式财务分析、本地优先

[![License](https://img.shields.io/badge/license-Non--Commercial-red.svg)](LICENSE)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8DB.svg)](https://tauri.app)
[![Vue](https://img.shields.io/badge/Vue-3.5-4FC08D.svg)](https://vuejs.org)
[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://rust-lang.org)

> **⚠️ 开源声明**：本项目采用 [非商业许可证](LICENSE)，仅供个人学习与非商业用途。商业使用需另行授权。

---

## 简介

**MoneySage** 是基于 **Tauri 2 + Vue 3** 的桌面记账应用，主打本地优先、AI 增强的财务管理体验。数据存储在本机 SQLite，无需网络即可使用核心记账功能；接入大模型后，可通过自然语言快速记账、进行对话式财务分析，甚至让 AI 帮你调整分类结构、执行 Python 数据分析脚本。

### 核心亮点

- **AI 快速记账** — 自然语言解析金额、分类、日期；批量解析，确认后入库
- **智能分析 (ChatBI)** — 多会话对话、流式输出、多轮工具调用；可查询数据库、管理分类、执行 Python、读写文件
- **分类管理** — 系统预设 + 自定义分类，支持父子层级；AI 可在对话中直接增删改分类
- **预算跟踪** — 按分类与周期设置预算，实时进度展示
- **记忆系统 V3** — Markdown 原生存储，人类可读、Agent 可操作；双层快照机制保护 LLM 前缀缓存
- **仪表盘与统计** — 月度收支、趋势图表、分类分布、预算执行状态
- **用量统计** — LLM token 用量记录与多维度汇总
- **大模型与 MCP** — 多套 OpenAI 兼容 API 配置；MCP 工具服务器扩展
- **本地优先** — SQLite 存储，可自定义数据与记忆目录

---

## 功能概览

### 仪表盘

本月收入、支出、结余概览，近 3/6/12 个月趋势图，支出分类分布与预算执行状态。

> ![仪表盘截图占位](./docs/screenshots/dashboard.svg)

### 账本

账本视图包含三个标签页：

- **收支记录** — 添加、编辑、删除交易记录；按月筛选浏览；CSV 导入导出
- **分类管理** — 系统预设 + 自定义分类，支持父子层级；支持 AI 在智能分析中直接调整
- **预算设置** — 按分类与周期设置预算，实时进度展示

> ![账本截图占位](./docs/screenshots/ledger.svg)

### AI 快速记账

在顶部「记一笔」按钮打开快速记账对话框，输入自然语言：

```
今天午餐38元，晚上买了杯奶茶15元
昨天收到工资 5000 元
```

AI 会自动解析出金额、分类、收支类型、日期，生成可编辑的确认列表，确认后保存入库。支持在对话框内切换模型配置。

> ![快速记账截图占位](./docs/screenshots/quick-booking.svg)

### 智能分析

持久化多会话对话式财务分析。你可以问：

- "分析本月支出结构"
- "过去三个月餐饮花了多少"
- "把『餐饮』改名为『吃喝玩乐』"
- "用 Python 画一张过去 6 个月的支出趋势图"

特性：

- SSE 流式输出
- 工具调用过程实时展示
- 内置本地工具：数据库查询、分类管理、Python 执行、记忆搜索、文件读写、Shell 执行、快速记账等
- 可联动 MCP 外部工具服务器
- 对话结束后自动运行记忆整合（Consolidator），提取用户画像、财务规则、目标等信息

> ![智能分析截图占位](./docs/screenshots/analysis.svg)

#### 智能分析中的 Python 支持

智能分析内置 `python_exec` 工具，可执行 Python 3 脚本进行复杂数据分析、统计计算、生成图表：

- 通过环境变量 `MONEY_SAGE_DB_PATH` 连接 SQLite 数据库
- 通过环境变量 `MONEY_SAGE_SESSION_ID` 获取当前会话 ID
- 生成图片建议保存到 `.query_temp/{session_id}/images/` 目录，前端会自动识别并展示
- 执行前会检测系统是否安装了 Python 3，未安装时提示用户自行安装

> **注意**：Python 执行依赖用户本机环境，应用不会自动安装 Python。

### 系统

系统视图整合了 AI 相关的配置与管理功能：

- **记忆管理** — 工作区文件编辑、记忆目录设置
- **用量统计** — LLM token 用量记录与多维度汇总
- **模型配置** — 管理多套 OpenAI 兼容 API 配置
- **MCP** — 配置 MCP 工具服务器扩展 AI 能力

> ![系统配置截图占位](./docs/screenshots/system.svg)

### 记忆管理 (V3)

- **三层记忆分类**：
  - `factual/` — 用户画像、财务规则、目标、Agent 角色设定
  - `episodic/` — 按日期组织的对话摘要
  - `procedural/` — 工作流程与操作技巧
- **Markdown 原生**：所有记忆以 `.md` 文件存储，可直接用编辑器查看和修改
- **双层快照**：`memory/MEMORY.md` 作为跨会话快照，每次对话注入 System Prompt
- **后台治理**：Governor 定期巡检，自动压缩超长文件、更新索引、重生成快照
- **FTS5 全文搜索**：增量同步索引，支持按分类分组搜索
- **用户可配置**：支持自定义记忆目录路径，支持重置/复制两种切换模式
- **安全保护**：`meta/` 目录禁止 Agent 修改；记忆写入前进行注入扫描

---

## 界面导航

| 菜单 | 说明 |
|------|------|
| 仪表盘 | 总览 + 内嵌统计图表 |
| 账本 | 收支记录 / 分类管理 / 预算设置（标签页） |
| 智能分析 | 对话式财务分析 |
| 系统 | 记忆管理 / 用量统计 / 模型配置 / MCP |

---

## 快速开始

### 系统要求

- **操作系统**：macOS / Windows 10+ / Linux
- **内存**：建议 4GB 及以上（启用 AI 时）
- **Windows**：需 WebView2 Runtime（Win11 已自带）
- **可选**：Python 3（如需使用智能分析中的 Python 执行功能）

### 安装

在 [Releases](../../releases) 下载最新安装包。常见产物：

- Windows：`.exe` (NSIS) / `.msi` (WiX)
- macOS：`.dmg`
- Linux：`.deb` / `.AppImage`

### 首次运行

1. 进入「系统 → 模型配置」，填写 API 信息（Base URL、API Key、模型名称）
2. 可选：进入「系统 → MCP」配置 MCP 工具服务器
3. 使用「记一笔」或「智能分析」前确认网络与密钥有效

> ![模型配置截图占位](./docs/screenshots/model-config.svg)

---

## 开发指南

### 环境准备

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://rustup.rs/) stable
- [Python 3](https://www.python.org/downloads/)（可选，用于测试 `python_exec` 工具）

### 克隆与启动

```bash
git clone <仓库地址>
cd money-sage
npm install
npm run tauri:dev
```

### 项目结构

```
money-sage/
├── src/                              # Vue 前端
│   ├── views/
│   │   ├── DashboardView.vue           # 仪表盘
│   │   ├── LedgerView.vue              # 账本（收支记录 / 分类 / 预算）
│   │   ├── TransactionsView.vue        # 收支记录
│   │   ├── CategoriesView.vue          # 分类管理
│   │   ├── BudgetView.vue              # 预算管理
│   │   ├── AnalysisView.vue            # 智能分析对话
│   │   ├── SystemView.vue              # 系统（记忆 / 用量 / 模型 / MCP）
│   │   ├── MemoryView.vue              # 记忆管理
│   │   ├── UsageStatsView.vue          # Token 用量统计
│   │   └── ...
│   ├── components/
│   │   ├── QuickBookingDialog.vue      # 快速记账对话框
│   │   ├── LLMConfigPanel.vue          # 大模型配置面板
│   │   ├── LLMConfigDialog.vue         # 大模型配置对话框
│   │   ├── McpConfigPanel.vue          # MCP 配置面板
│   │   └── McpConfigDialog.vue         # MCP 配置对话框
│   ├── stores/index.ts                 # Pinia Store + Tauri invoke
│   └── App.vue
├── src-tauri/
│   ├── src/
│   │   ├── ai/
│   │   │   ├── agent/                  # AnalysisAgent / QuickNoteAgent
│   │   │   └── tools/                  # 本地工具集合
│   │   │       ├── manage_categories.rs  # 分类管理（增删改）
│   │   │       ├── python_exec.rs        # Python 执行
│   │   │       ├── query_database.rs     # 数据库只读查询
│   │   │       ├── get_schema.rs         # 数据库表结构
│   │   │       ├── quick_note_parse.rs   # 快速记账解析
│   │   │       ├── quick_note_save.rs    # 快速记账保存
│   │   │       ├── memory_search.rs      # FTS5 记忆搜索
│   │   │       ├── file_read.rs          # 文件读取
│   │   │       ├── file_edit.rs          # 文件精确替换
│   │   │       ├── file_write.rs         # 文件创建/覆盖
│   │   │       ├── bash_exec.rs          # Shell 命令执行
│   │   │       └── workspace_path.rs     # 工作区路径解析
│   │   ├── memory/
│   │   │   └── v3/                     # 记忆系统 V3
│   │   ├── workspace/                  # 工作区与 System Prompt 构建
│   │   ├── telemetry/                  # Token 用量记录与汇总
│   │   ├── mcp/                        # MCP 工具服务器管理
│   │   ├── data_io/                    # 数据导入导出
│   │   ├── database.rs                 # SQLite 数据库 + 表结构
│   │   ├── models.rs                   # 数据模型
│   │   └── lib.rs                      # Tauri 命令注册 + 启动初始化
│   ├── Cargo.toml
│   └── tauri.conf.json
├── docs/                               # 文档与截图
│   └── screenshots/                    # README 截图目录
├── package.json
└── vite.config.ts
```

### 测试

```bash
cd src-tauri
cargo test
```

---

## 技术栈

### 前端

| 用途 | 技术 |
|------|------|
| 框架 | Vue 3 |
| 构建 | Vite |
| UI 组件 | Element Plus |
| 图表 | ECharts + vue-echarts |
| 状态管理 | Pinia |
| Markdown 渲染 | marked |
| CSV 解析 | PapaParse |
| 工具 | VueUse, Day.js |

### 后端

| 用途 | 技术 |
|------|------|
| 桌面框架 | Tauri 2 |
| 语言 | Rust |
| 数据库 | SQLite + sqlx |
| HTTP 客户端 | reqwest (SSE 流式) |
| 异步运行时 | tokio |
| Excel | calamine + rust_xlsxwriter |

### Tauri 插件

`tauri-plugin-dialog` `tauri-plugin-fs` `tauri-plugin-opener` `tauri-plugin-sql`

---

## 记忆系统架构 (V3)

```
{memory_dir}/
├── MEMORY.md                  # 跨会话快照（注入 System Prompt，冻结单会话不变）
├── meta/
│   └── RULES.md               # 写入规范（动态注入 System Prompt，可自定义）
├── factual/
│   ├── INDEX.md               # 文件索引
│   ├── user-profile.md        # 用户画像（消费习惯、收入、家庭成员等）
│   ├── finance-rules.md       # 财务规则（分类规则、周期事件）
│   ├── goals.md               # 财务目标（储蓄计划、预算限制）
│   └── agent-role.md          # Agent 角色设定（语气、风格、身份）
├── episodic/
│   ├── INDEX.md               # 按时间组织的索引
│   └── YYYY/MM/YYYY-MM-DD.md  # 每日对话摘要
└── procedural/
    ├── INDEX.md               # 技巧索引
    └── workflows.md           # 工作流程与操作经验
```

### 数据流

1. **启动时** — MemoryStore 初始化目录骨架；Migrator 执行 V2→V3 迁移；Indexer 同步 FTS5；Governor 定时巡检
2. **对话时** — SystemPromptBuilder 动态拼装：BOOTSTRAP → AGENTS.md → agent-role → MEMORY.md 快照 → user-profile → 工具指南
3. **工具调用** — Agent 通过 file_read/file_write/file_edit 操作记忆文件（`meta/` 保护）
4. **对话后** — Consolidator 分析对话内容，追加记忆条目
5. **后台** — Governor 每 6 小时压缩超长文件、更新索引、重生成 MEMORY.md

### 快照与缓存保护

为保护 LLM 前缀缓存，System Prompt 中注入的是**对话开始时冻结的 MEMORY.md 快照**，而非实时文件。单个会话内即使 Agent 更新了记忆，快照也不会变化，下个会话才能看到更新。

---

## 本地工具清单

智能分析内置以下本地工具：

| 工具 | 说明 |
|------|------|
| `get_database_schema` | 获取数据库表结构 |
| `query_database` | 执行 SELECT 查询（结果可保存为 CSV） |
| `manage_categories` | 新增 / 修改 / 删除分类 |
| `python_exec` | 执行 Python 3 脚本进行数据分析或生成图表 |
| `quick_note_parse` | 自然语言解析记账草稿 |
| `quick_note_save` | 保存确认的记账草稿 |
| `memory_search` | FTS5 全文搜索记忆文件 |
| `file_read` / `file_write` / `file_edit` | 读写工作区与记忆文件 |
| `bash` | 执行 Shell 命令 |
| `csv_read` / `workspace_size` | CSV 读取 / 工作区大小统计 |

---

## 构建发布

### 一键构建

```bash
npm run build:installer
```

调试版本：

```bash
npm run build:installer:debug
```

### 输出目录

`src-tauri/target/release/bundle/`

### 发版前检查

保持以下文件版本号一致：`package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml`

---

## 贡献

欢迎提交 Issue 和 Pull Request。请确保：

1. 代码通过 `npm run build` 和 `cd src-tauri && cargo test`
2. 提交信息遵循 [Conventional Commits](https://www.conventionalcommits.org/) 规范
3. 新增功能附带必要的测试或文档更新

---

## 许可证

[Non-Commercial License](LICENSE) — 仅供非商业用途使用，商业使用需另行授权。
