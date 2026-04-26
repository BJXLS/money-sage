# MoneySage 项目功能审计报告（2026-04-17）

## 1. 审计目标

本次审计围绕以下目标展开：

- 全面梳理当前项目的实际功能能力（前端、后端、数据层、AI 能力）
- 排查各功能是否存在 bug、缺陷、行为不一致
- 提出可落地的优化建议与优先级
- 形成可持续跟踪的项目质量文档

## 2. 审计范围与方法

### 2.1 范围

- 前端页面：`DashboardView`、`TransactionsView`、`CategoriesView`、`BudgetView`、`StatisticsView`、`AnalysisView`、`ImportExportView`
- 前端组件：`QuickBookingDialog`、`AddBudgetDialog`、`LLMConfigDialog`、`McpConfigDialog`、`EditTransactionDialog`
- 状态与调用：`src/stores/index.ts`
- 后端与数据库：`src-tauri/src/lib.rs`、`src-tauri/src/database.rs`、`src-tauri/src/models.rs`

### 2.2 方法

- 静态代码审查：逐模块检查功能闭环、字段一致性、命令一致性、文案与行为一致性
- 构建验证：
  - `npm run build`（前端类型检查 + 构建）
  - `cargo check`（Rust 编译检查）
  - `cargo test`（后端测试）

## 3. 当前功能全景（已具备能力）

### 3.1 交易与分类

- 交易记录增删改查（按日期范围查询）
- 日历式记账交互（按日查看、编辑、删除）
- 收入/支出分类管理
- 分类层级（大类/小类）
- 分类图标、颜色、自定义管理

### 3.2 预算

- 预算管理（创建、更新、删除、查看）
- 预算进度统计（spent/remaining/percentage）
- 时间预算与事件预算两种模式
- 交易可关联预算（`budget_id`）

### 3.3 数据分析与仪表盘

- 月度收入/支出/结余统计
- 分类占比统计（支出/收入）
- 仪表盘概览（本月收入支出、最近交易、预算执行）
- 趋势图和饼图展示

### 3.4 AI 能力

- AI 快速记账（自然语言 -> 结构化交易）
- AI 解析后人工确认再落库
- 智能分析（会话化、流式输出、工具调用事件）
- LLM 配置管理（多配置、激活、测试连接）
- MCP 服务管理（增删改查、启停、状态和工具列表）

### 3.5 数据层与后端能力

- SQLite 本地持久化
- 初始化建表与迁移逻辑（categories / transactions / budgets / llm_configs / analysis_sessions / analysis_messages / mcp_servers）
- Tauri 命令接口覆盖基础业务

## 4. 发现的问题与缺陷

以下按优先级分级（P0 > P1 > P2），并给出证据与影响。

---

### 4.1 P0（高优先级，影响核心可用性）

#### P0-1 导入功能命令名不一致，前端导入不可用

- 现象：
  - 前端调用 `import_csv_transactions`
  - 后端实际暴露命令是 `import_transactions`
- 证据：
  - `src/stores/index.ts` 调用：`invoke('import_csv_transactions', ...)`
  - `src-tauri/src/lib.rs` 命令定义：`async fn import_transactions(...)`
- 影响：CSV 导入功能调用失败，属于功能级不可用。

#### P0-2 预算周期“daily”与数据库约束不一致，创建/更新预算可失败

- 现象：
  - 前端允许 `daily`（每日预算）
  - 数据库 `budgets.period_type` 约束只允许 `weekly/monthly/yearly`
- 证据：
  - `src/components/AddBudgetDialog.vue` 存在 `daily` 选项和逻辑
  - `src-tauri/src/database.rs` 约束：`CHECK (period_type IN ('weekly', 'monthly', 'yearly'))`
- 影响：用户选择每日预算后，提交可能失败，属于核心业务规则冲突。

#### P0-3 前端构建失败（TypeScript 类型错误）

- 现象：`npm run build` 失败。
- 错误：`src/views/AnalysisView.vue(360,5): error TS2322`
- 影响：前端无法通过正式构建流程，影响发布。

#### P0-4 后端测试集不可通过（接口结构升级后测试未同步）

- 现象：`cargo test` 失败，报 17 个错误。
- 主要问题：
  - `AIMessage` 结构新增字段后，测试初始化缺字段
  - `AIRequest` 新增 `tools/tool_choice` 字段，测试未补
  - `content` 从 `String` 变 `Option<String>` 后断言未改
- 影响：回归保障失效，CI/质量门禁不可用。

---

### 4.2 P1（中优先级，功能行为错误或体验显著不一致）

#### P1-1 分类删除提示与真实行为不一致

- 现象：
  - UI 文案提示“删除大类会同时删除小类”
  - 后端实际逻辑是：若存在子分类则拒绝删除
- 证据：
  - `src/views/CategoriesView.vue` 删除确认文案
  - `src-tauri/src/database.rs` `delete_category` 中先检查 `sub_count > 0` 并返回错误
- 影响：用户预期与行为冲突，降低可信度。

#### P1-2 交易表单存在“账户/时间”字段，但并未落库

- 现象：
  - 前端有 `account`、`time` 输入
  - 后端模型与数据库无对应字段
  - 保存时未提交账户/时间
- 证据：
  - `src/views/TransactionsView.vue` 中 `transactionForm.account/time`
  - `src-tauri/src/models.rs` 的 `Transaction/NewTransaction` 无 `account/time`
- 影响：用户误以为记录了账户/时间，实际数据丢失。

#### P1-3 导入导出页面仍为占位页，与系统能力不匹配

- 现象：`ImportExportView` 仅显示占位文案。
- 证据：`src/views/ImportExportView.vue` 仅有“这里将显示CSV导入导出功能”。
- 影响：功能入口存在但不可用，造成“假完成”。

#### P1-4 仪表盘“查看全部/管理预算”按钮无跳转逻辑

- 现象：按钮点击无实际行为。
- 证据：`src/views/DashboardView.vue` 中 `goToTransactions()`/`goToBudget()` 为空实现。
- 影响：交互断点。

---

### 4.3 P2（低优先级，技术债/可维护性/一致性优化）

#### P2-1 Rust 警告较多，存在未使用代码与变量

- 现象：`cargo check` 输出大量 warning（unused import/unused fn 等）。
- 影响：长期降低可维护性，掩盖真实告警。

#### P2-2 前端主题风格存在多套并行样式体系

- 现象：部分页面使用“全局深色玻璃风”，部分页面使用“独立深色卡片风”。
- 影响：视觉一致性与维护成本不佳。

#### P2-3 状态管理与页面本地查询边界可进一步收敛

- 现象：部分页面直接拉大范围交易再本地过滤，存在重复逻辑。
- 影响：当数据量增大时，性能与可读性下降。

## 5. 构建与测试结果汇总

### 5.1 `npm run build`

- 结果：失败
- 原因：`AnalysisView.vue` 的 TypeScript 类型不匹配（第 360 行附近）

### 5.2 `cargo check`

- 结果：通过（但有较多 warning）

### 5.3 `cargo test`

- 结果：失败
- 原因：测试代码未同步最新 HTTP/AI 数据结构（`AIMessage`、`AIRequest`、`Option<String>` 等）

## 6. 优化建议（按阶段）

### 6.1 第一阶段（建议 1-2 天内）

- 修复导入命令名不一致（统一为一个命令名）
- 统一预算周期规则（前后端一致：要么支持 `daily`，要么前端移除）
- 修复 `AnalysisView.vue` 类型错误，恢复前端构建
- 同步修复后端测试用例结构，恢复 `cargo test`

### 6.2 第二阶段（建议 3-5 天）

- 完成 `ImportExportView` 真正功能闭环（文件选择、预览、错误提示、结果反馈）
- 修正分类删除文案或后端逻辑（两者保持一致）
- 处理交易表单“账户/时间”：
  - 若暂不支持，移除字段与文案
  - 若要支持，补齐 DB + model + command + store + UI
- 完成仪表盘按钮跳转（切换菜单或路由）

### 6.3 第三阶段（建议持续进行）

- 清理 Rust warnings，减少死代码
- 抽离统计口径与过滤逻辑，集中到 store 或后端查询层
- 建立最小自动化质量门禁：
  - PR 必须通过 `npm run build`
  - PR 必须通过 `cargo check` + `cargo test`

## 7. 建议的修复优先级清单（可直接转任务）

- [P0] 修复 CSV 导入命令名不一致
- [P0] 对齐预算周期 `daily` 规则
- [P0] 修复前端 TS 构建错误（AnalysisView）
- [P0] 修复后端测试用例结构并恢复通过
- [P1] 完成导入导出页面实现
- [P1] 修复分类删除文案/行为不一致
- [P1] 账户/时间字段能力对齐（删或补齐）
- [P1] 补全仪表盘按钮跳转
- [P2] 清理 warning 和冗余代码
- [P2] 统一 UI 设计规范与复用组件

## 8. 结论

项目当前已经具备较强的“桌面记账 + AI 扩展”能力基础，但仍存在若干关键一致性问题和发布阻断项。  
建议优先完成 P0 项，先恢复“可构建、可测试、功能闭环可用”，再推进体验与工程质量优化。

