<div align="center">

# 💰 MoneySage

一款现代化的智能记账桌面应用，结合 AI 快速记账、对话式财务分析与本地数据存储

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8DB.svg)](https://tauri.app)
[![Vue](https://img.shields.io/badge/Vue-3.5-4FC08D.svg)](https://vuejs.org)
[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://rust-lang.org)

[功能概览](#功能概览) • [界面导航](#界面导航) • [快速开始](#快速开始) • [开发指南](#开发指南) • [技术栈](#技术栈) • [构建发布](#构建发布)

</div>

---

## 📖 简介

**MoneySage** 是一款基于 **Tauri 2 + Vue 3** 的 Windows 桌面记账应用。支持自然语言快速记账、多会话「智能分析」对话（可调用数据库工具）、本地 **SQLite** 持久化，以及 **LLM / MCP** 的可选扩展。数据默认保存在本机应用数据目录，便于离线使用与隐私控制。

### ✨ 核心亮点

- 🤖 **AI 快速记账** — 自然语言解析金额、分类、日期；支持批量解析与确认后入库  
- 💬 **智能分析（ChatBI）** — 多分析会话、流式回复、工具调用状态；可联动快速记账草稿确认  
- 🧠 **记忆管理** — 维护结构化记忆事实与 Agent 人设（角色预设等），供 AI 场景使用  
- 📊 **仪表盘与统计** — 月度汇总、趋势与分类分布；仪表盘内嵌统计分析图表  
- 📈 **用量统计** — 记录每次 LLM 调用的 token 用量（写入本地数据库，可按配置/模型汇总）  
- 🔌 **大模型与 MCP** — 支持多套 OpenAI 兼容 API 配置；可连接 MCP 工具服务器扩展能力  
- 🔐 **本地优先** — 默认 SQLite 文件库（`money_note.db`）；异常情况下可能回退内存库（重启不保留）  

---

## 功能概览

### 仪表盘

- 本月收入、支出、结余与交易概况  
- 近 **3 / 6 / 12** 个月收支趋势  
- 支出分布与预算执行概览  
- 内嵌 **统计分析**（`StatisticsView`），无需单独入口  

### AI 快速记账

```
输入: "今天中午在餐厅花了38元吃午饭"
识别: 金额、分类、收支类型、日期（可编辑确认）
```

- 支持多条文本批量解析  
- 解析结果可修改后再保存  

### 收支记录

- 添加、编辑、删除交易；按条件浏览  
- **数据导入 / 导出**：支持应用自定义格式（Excel、`money_sage` 包等，具体以界面与后端命令为准），用于备份与迁移  

### 分类与预算

- **分类管理**：系统预设与自定义分类；支持父子层级（大类 / 小类）  
- **预算设置**：按分类与周期监控进度（含事件类预算等）  

### 智能分析

- 持久化 **分析会话** 与历史消息  
- **流式输出**、工具调用过程展示  
- 与 **快速记账草稿** 联动，可在对话流中确认入账  

### 记忆管理

- 查看与管理记忆条目、变更历史（含撤销类操作，以实际界面为准）  
- **人设 / 角色**：全局、快速记账、智能分析等范围可分别配置  

### 用量统计

- 按 **LLM 配置**、**模型** 等维度汇总调用次数与 **prompt / completion / total** tokens  
- 每次请求一行日志，**持久化**在本地表 `token_usage_logs`（除非当前运行使用了内存数据库）  
- 支持按日期筛选与清理历史日志（如「清理 90 天前」）  

### 顶部快捷入口

- **快速记账** — 打开 AI 记账对话框  
- **MCP** — 配置 MCP 工具服务器（可选）  
- **大模型配置** — 管理多套 API、模型与连接参数  

---

## 界面导航

与 `src/App.vue` 侧边栏一致：

| 菜单       | 说明 |
|------------|------|
| 仪表盘     | 总览 + 内嵌统计图表 |
| 收支记录   | 交易 CRUD + 导入导出 |
| 分类与预算 | 分类管理 / 预算设置（标签页） |
| 智能分析   | 对话式财务分析 |
| 记忆管理   | 记忆与人设 |
| 用量统计   | LLM token 用量 |

---

## 🚀 快速开始

### 安装使用（Windows）

1. 在 [Releases](../../releases) 下载最新安装包（若已发布）  
2. 常见产物名称示例：`money-sage_*_x64-setup.exe`（NSIS）、`money-sage_*_x64_zh-CN.msi`（MSI）  
3. 安装后启动应用  

### 系统要求

- **操作系统**：Windows 10 / 11（x64）  
- **内存**：建议 4GB 及以上（启用 AI 时）  
- **磁盘**：约 100MB 量级（随数据增长）  
- **WebView2**：Windows 11 通常已自带；Windows 10 若缺失需安装 [WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/)  

---

## 💻 开发指南

### 环境准备

- [Node.js](https://nodejs.org/) 18+  
- [Rust](https://rustup.rs/)（stable）及 Windows 对应构建依赖（用于 `tauri build`）  

### 克隆与安装

```bash
git clone <你的仓库地址>
cd money-note
npm install
```

### 开发运行

```bash
# 推荐：前端 + Tauri 热重载
npm run tauri:dev
```

或拆分：

```bash
npm run dev    # 仅 Vite
npm run tauri  # 需另开终端配合 dev server
```

### 代码结构（摘要）

```
money-note/
├── src/                         # Vue 前端
│   ├── views/
│   │   ├── DashboardView.vue       # 仪表盘（内嵌 StatisticsView）
│   │   ├── TransactionsView.vue    # 收支 + 导入导出
│   │   ├── CategoriesBudgetView.vue# 分类 / 预算标签容器
│   │   ├── CategoriesView.vue
│   │   ├── BudgetView.vue
│   │   ├── StatisticsView.vue
│   │   ├── AnalysisView.vue        # 智能分析
│   │   ├── MemoryView.vue          # 记忆管理
│   │   └── UsageStatsView.vue      # Token 用量统计
│   ├── components/
│   │   ├── QuickBookingDialog.vue
│   │   ├── LLMConfigDialog.vue
│   │   └── McpConfigDialog.vue
│   ├── stores/index.ts             # Pinia + Tauri invoke
│   └── App.vue
├── src-tauri/
│   ├── src/
│   │   ├── ai/                     # Agent、工具与流式分析
│   │   ├── telemetry/              # token_usage 记录与汇总
│   │   ├── database.rs             # SQLite 与迁移
│   │   ├── models.rs
│   │   └── lib.rs                  # Tauri 命令注册
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

## 🛠️ 技术栈

### 前端

| 用途 | 技术 |
|------|------|
| 框架 | [Vue 3](https://vuejs.org/) |
| 构建 | [Vite](https://vitejs.dev/) |
| UI | [Element Plus](https://element-plus.org/) |
| 图表 | [ECharts](https://echarts.apache.org/) + [vue-echarts](https://github.com/ecomfe/vue-echarts) |
| 状态 | [Pinia](https://pinia.vuejs.org/) |
| 工具 | [VueUse](https://vueuse.org/)、[Day.js](https://day.js.org/) |
| 表格 | [PapaParse](https://www.papaparse.com/)（CSV） |
| 正文渲染 | [marked](https://github.com/markedjs/marked)（分析消息等） |

### 后端

| 用途 | 技术 |
|------|------|
| 桌面壳 | [Tauri 2](https://tauri.app/) |
| 语言 | [Rust](https://www.rust-lang.org/) |
| 数据库 | [SQLite](https://www.sqlite.org/) + [sqlx](https://github.com/launchbadge/sqlx) |
| HTTP | [reqwest](https://github.com/seanmonstar/reqwest)（流式响应等） |
| 异步 | [tokio](https://tokio.rs/) |
| Excel | [calamine](https://github.com/tafia/calamine)、[rust_xlsxwriter](https://github.com/jmcnamara/rust_xlsxwriter) |

### Tauri 插件（节选）

- `tauri-plugin-dialog`、`tauri-plugin-fs`、`tauri-plugin-opener` 等（见 `src-tauri/Cargo.toml`）  

---

## 📦 构建发布

### 一键构建安装包

```bash
npm run build:installer
```

等价于 `npm run build` 后执行 `tauri build`。调试包可使用：

```bash
npm run build:installer:debug
```

若仓库内提供脚本，也可使用 `build-installer.bat` / `build-installer.ps1`（视本仓库是否包含而定）。

### Windows 打包依赖（可选）

- **NSIS**：生成 `.exe` 安装向导  
- **WiX**：生成 `.msi`  

详见仓库内 [BUILD.md](BUILD.md)、[打包说明.md](打包说明.md)（若存在）。

### 输出目录

构建成功后，安装包通常位于：

`src-tauri/target/release/bundle/`（`nsis/`、`msi/` 等子目录）

### 发版前同步版本号

保持以下文件中版本号一致：

- `package.json` 的 `version`  
- `src-tauri/tauri.conf.json`  
- `src-tauri/Cargo.toml`  

---

## 📚 使用文档

更详细的操作说明见 [使用说明.md](使用说明.md)（若与本 README 有出入，以当前软件界面为准）。

**首次运行建议**：在「大模型配置」中填写可用的 API；使用「智能分析」「快速记账」前请确认网络与密钥有效。  

---

## 🤝 贡献与问题

欢迎提交 Issue / PR。提交前可本地执行 `cargo fmt`、`cargo test` 与前端构建检查。

---

## 📄 许可证

本项目采用 [MIT License](LICENSE)。

---

## 🙏 致谢

[Tauri](https://tauri.app/)、[Vue.js](https://vuejs.org/)、[Element Plus](https://element-plus.org/)、[Rust](https://www.rust-lang.org/) 及生态中所有依赖项目。

---

<div align="center">

**用 ❤️ 构建**

[⬆ 回到顶部](#-moneysage)

</div>
