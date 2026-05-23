# MoneySage

智能记账桌面应用 — AI 快速记账、对话式财务分析、本地优先

[![License](https://img.shields.io/badge/license-Non--Commercial-red.svg)](LICENSE)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8DB.svg)](https://tauri.app)
[![Vue](https://img.shields.io/badge/Vue-3.5-4FC08D.svg)](https://vuejs.org)
[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://rust-lang.org)

</div>

---

## 简介

**MoneySage** 是基于 **Tauri 2 + Vue 3** 的桌面记账应用。支持自然语言快速记账、流式对话财务分析、本地 **SQLite** 持久化，以及 **LLM / MCP** 扩展。数据保存在本机应用数据目录，无需网络即可使用核心功能。

### 核心亮点

- **AI 快速记账** — 自然语言解析金额、分类、日期；批量解析，确认后入库
- **智能分析 (ChatBI)** — 多会话对话、流式输出、工具调用循环；可查询数据库、搜索记忆、读写文件
- **记忆系统 V3** — Markdown 原生存储，人类可读、Agent 可操作；双层快照机制保护 LLM 前缀缓存
- **仪表盘与统计** — 月度收支、趋势图表、分类分布、预算进度
- **用量统计** — LLM token 用量记录与多维度汇总
- **大模型与 MCP** — 多套 OpenAI 兼容 API 配置；MCP 工具服务器扩展
- **本地优先** — SQLite 存储，可自定义数据与记忆目录

---

## 功能概览

### 仪表盘

本月收入、支出、结余概览，近 3/6/12 个月趋势图，支出分类分布与预算执行状态。

### AI 快速记账

```
输入: "今天午餐38元，晚餐买了杯奶茶15元"
解析: 2 条记录，含金额、分类、收支类型、日期，可逐条编辑后保存
```

支持下述解析策略：AI 语义理解（需配置大模型），解析后可在确认界面修改分类、金额等信息再保存。

### 收支记录

添加、编辑、删除交易记录；按月筛选浏览；CSV 导入导出；支持应用自定义格式（Excel / money_sage）的备份与迁移。

### 分类与预算

- **分类管理**：系统预设 + 自定义分类，支持父子层级
- **预算设置**：按分类与周期设置预算，实时进度展示

### 智能分析

- 持久化多会话，支持历史回溯
- SSE 流式输出，工具调用过程实时展示
- 内置 9 个本地工具：数据库查询、文件读写、记忆搜索、快速记账解析/保存、表结构查询、Shell 执行
- 可联动 MCP 外部工具服务器
- 对话结束后自动运行记忆整合（Consolidator），提取用户画像、财务规则、目标等信息

### 记忆管理 (V3)

- **三层记忆分类**：
  - `factual/` — 用户画像、财务规则、目标、Agent 角色设定
  - `episodic/` — 按日期组织的对话摘要
  - `procedural/` — 工作流程与操作技巧
- **Markdown 原生**：所有记忆以 `.md` 文件存储，可直接用编辑器查看和修改
- **双层快照**：`memory/MEMORY.md` 作为跨会话快照，每次对话注入 System Prompt
- **后台治理**：Governor 定期巡检，自动压缩超长文件（保留最近 5 条/标题）、更新索引、重生成快照
- **FTS5 全文搜索**：增量同步索引，支持按分类分组搜索
- **用户可配置**：支持自定义记忆目录路径，支持重置/复制两种切换模式
- **安全保护**：`meta/` 目录禁止 Agent 修改；记忆写入前进行注入扫描

### 用量统计

每次 LLM 调用记录 prompt/completion/total tokens，按配置、模型、日期等维度汇总，支持清理历史日志。

### 大模型配置

支持多套 OpenAI 兼容 API 配置，可分别设置 Base URL、API Key、模型、温度等参数，随时切换活跃配置。

---

## 界面导航

| 菜单 | 说明 |
|------|------|
| 仪表盘 | 总览 + 内嵌统计图表 |
| 收支记录 | 交易 CRUD + 导入导出 |
| 分类与预算 | 分类管理 / 预算设置（标签页） |
| 智能分析 | 对话式财务分析 |
| Agent 配置 | 工作区文件编辑 + 记忆目录设置 |
| 用量统计 | LLM token 用量 |

---

## 快速开始

### 系统要求

- **操作系统**：macOS / Windows 10+ / Linux
- **内存**：建议 4GB 及以上（启用 AI 时）
- **Windows**：需 WebView2 Runtime（Win11 已自带）

### 安装

在 [Releases](../../releases) 下载最新安装包。常见产物：

- Windows：`.exe` (NSIS) / `.msi` (WiX)
- macOS：`.dmg`
- Linux：`.deb` / `.AppImage`

### 首次运行

1. 在「大模型配置」中填写 API 信息（Base URL、Key、模型名称）
2. 使用「快速记账」或「智能分析」前确认网络与密钥有效

---

## 开发指南

### 环境准备

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://rustup.rs/) stable

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
│   │   ├── DashboardView.vue           # 仪表盘（内嵌统计）
│   │   ├── TransactionsView.vue        # 收支记录 + 导入导出
│   │   ├── CategoriesBudgetView.vue    # 分类 / 预算标签容器
│   │   ├── CategoriesView.vue          # 分类管理
│   │   ├── BudgetView.vue              # 预算管理
│   │   ├── StatisticsView.vue          # 统计分析
│   │   ├── AnalysisView.vue            # 智能分析对话
│   │   ├── MemoryView.vue              # Agent 配置（工作区 + 记忆目录）
│   │   └── UsageStatsView.vue          # Token 用量统计
│   ├── components/
│   │   ├── QuickBookingDialog.vue      # 快速记账对话框
│   │   ├── LLMConfigDialog.vue         # 大模型配置
│   │   └── McpConfigDialog.vue         # MCP 服务器配置
│   ├── stores/index.ts                 # Pinia Store + Tauri invoke
│   └── App.vue
├── src-tauri/
│   ├── src/
│   │   ├── ai/
│   │   │   ├── agent/                  # AnalysisAgent / QuickNoteAgent
│   │   │   └── tools/                  # 9 个本地工具
│   │   │       ├── file_edit.rs        # 精确字符串替换（workspace/memory）
│   │   │       ├── file_read.rs        # 文件读取
│   │   │       ├── file_write.rs       # 文件创建/覆盖
│   │   │       ├── memory_search.rs    # FTS5 记忆搜索
│   │   │       ├── query_database.rs   # 数据库只读查询
│   │   │       ├── get_schema.rs       # 数据库表结构
│   │   │       ├── quick_note_parse.rs # 快速记账文本解析
│   │   │       ├── quick_note_save.rs  # 快速记账保存
│   │   │       ├── bash_exec.rs        # Shell 命令执行
│   │   │       └── workspace_path.rs   # 工作区路径解析
│   │   ├── memory/
│   │   │   └── v3/
│   │   │       ├── store.rs            # MemoryStore — 文件系统存储层
│   │   │       ├── snapshot.rs         # SnapshotLoader — 快照加载
│   │   │       ├── snapshot_generator.rs # SnapshotGenerator — MEMORY.md 生成
│   │   │       ├── indexer.rs          # FTS5 增量索引同步 + 搜索
│   │   │       ├── migrator.rs         # V2 → V3 数据迁移
│   │   │       ├── safety.rs           # 注入扫描 + 安全检查
│   │   │       ├── changelog.rs        # 变更日志 + 撤销支持
│   │   │       ├── governor.rs         # 后台巡检：压缩文件、更新索引
│   │   │       └── consolidator.rs     # 对话结束后的记忆整合
│   │   ├── workspace/
│   │   │   ├── mod.rs                  # WorkspaceManager — 工作区文件管理
│   │   │   └── builder.rs              # SystemPromptBuilder — 动态拼装提示词
│   │   ├── telemetry/                  # Token 用量记录与汇总
│   │   ├── mcp/                        # MCP 工具服务器管理
│   │   ├── data_io/                    # 数据导入导出
│   │   ├── database.rs                 # SQLite 数据库 + 表结构
│   │   ├── models.rs                   # 数据模型
│   │   └── lib.rs                      # Tauri 命令注册 + 启动初始化
│   ├── Cargo.toml
│   └── tauri.conf.json
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

## 许可证

[Non-Commercial License](LICENSE) — 仅供非商业用途使用，商业使用需另行授权。
