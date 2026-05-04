# 飞书远程接入聊天能力 —— 技术方案

> 目标：在 channels-money-sage（Rust + Tauri 桌面应用）中接入飞书机器人，使用户可以在飞书的**私聊**里通过自然语言与本地的 AI 财务助手对话（智能分析、快速记账等），复用现有 LLM、Memory、MCP 工具体系。
>
> 参考：openclaw（TypeScript，`extensions/feishu/`）、hermes-agent（Python，`gateway/platforms/feishu.py`）的成熟实现。
>
> 版本：v0.2（决策已对齐，准备实施）

## 决策摘要（已对齐）

| #  | 决策点                                | 最终选择                                                              |
| -- | ------------------------------------- | --------------------------------------------------------------------- |
| 1  | 选用 SDK                              | **`open-lark`**                                                        |
| 2  | 接收协议                              | **仅 WebSocket**（webhook 不在范围内）                                  |
| 3  | Agent 路由                            | **全部走 `AnalysisAgent`**；默认接续当前会话；`/new` 命令开启新会话         |
| 4  | 群功能                                | **暂不实现**，仅支持私聊（P2P）                                          |
| 5  | （N/A，群已被排除）                   | —                                                                     |
| 6  | Vue 中区分 source="飞书"             | **是**：会话列表与每条消息均标记 `source=feishu`                           |
| 7  | `app_secret` 是否需 keyring 加密      | **否**，与现有 `llm_configs.api_key` 同处理                              |

---

## 0. 阅读说明

- 本文档分两部分：**第 1–4 节**给出问题定义、整体架构与关键技术选型；**第 5–10 节**给出落地细节（模块划分、数据库、配置、命令、流水线、错误处理与里程碑）。
- 顶部"决策摘要"已对齐，文中相关章节已按选定方案落实。
- 实施阶段会按"里程碑"分批合并，每个里程碑都可独立验收。

---

## 1. 目标与范围

### 1.1 用户故事

1. 用户在桌面端 channels-money-sage 中配置飞书应用凭据（App ID / App Secret 等），并启动"飞书机器人"。
2. 用户在飞书 App 里把机器人加为联系人，与它进行**私聊**：
   - "我今天午餐花了 35 元，还买了瓶水 4 元"  → 机器人解析并写入数据库（通过 LLM 调用 quick_note 工具），可选给出确认或直接落库。
   - "我这个月在餐饮上一共花了多少？最高的三天是哪几天？" → 机器人调用 AnalysisAgent，查 SQLite，返回 markdown 分析。
3. **会话延续**：默认所有消息都接续到该用户的"当前会话"，跨多次飞书聊天不丢上下文。
4. **新会话命令**：用户发送 `/new` 即开启新会话（重置上下文）；可选附带初始话题，例如 `/new 帮我做一份本月预算`。
5. 用户可在桌面端面板上看到飞书机器人的连接状态、最近事件、可一键停启；并能在"智能分析"会话列表中看到来自飞书的会话（带 `飞书` 来源标记）。

### 1.2 范围

**MVP 范围（必须实现）**：
- 飞书**私聊**（P2P）文本消息收 / 回；
- `/new` 命令开启新会话；其他文本默认接续"当前会话"；
- 走"智能分析"流水线（含工具调用）回答用户；
- 飞书凭据配置 + 启停控制 + 状态展示；
- 与现有 `analysis_sessions` 体系打通，每个飞书用户的"当前会话"持久化、历史会话可见；
- Vue 端会话列表展示来源标记（`飞书` 与本地）。

**Stretch（视情况推进）**：
- 富文本（post 卡片）回复、流式输出（编辑消息追加内容）；
- 文本批合并（debounce）；
- 图片 / 文件接收 / OCR / 票据识别；
- QuickNote 模式：交互式确认卡片；
- 多账号（多个飞书应用）支持；
- Webhook 模式（HTTP 入站）作为 WebSocket 模式的备选；
- @飞书表情回应（typing / 完成 / 失败）。

**非范围（本期不做）**：
- **群聊**（包括 @bot、群白名单、群权限策略）—— 后续视需求再加；
- 飞书云文档 / 多维表格 / Wiki 工具调用；
- 跨进程的"网关守护进程"（保留为单一桌面进程内运行）；
- 工单审批流。

### 1.3 设计约束

- **不破坏现有桌面 UI 流程**：现有 Vue 端的"智能分析"对话框不变；飞书相当于新增一条入站通道。
- **不引入额外的常驻进程**：飞书 bot 跟随 Tauri 主进程的生命周期。如果用户关闭桌面应用，bot 也会下线（这是用户预期，无需后台保活）。
- **可在用户机器上离线启停**：不依赖任何外部网关 / 中继服务。
- **保持 Rust 单 crate**：不引入新的子 crate（除非 SDK 体积过大需拆分）。

---

## 2. 现状回顾（精炼版）

### 2.1 channels-money-sage 关键能力

- Tauri 2 + tokio runtime（`tauri::async_runtime::spawn` 已在 setup 中使用）。
- 已有 `AnalysisAgent`（`src-tauri/src/ai/agent/analysis.rs`）：流式对话、工具调用循环（最多 8 轮）、可注入 `FinancialContext` + `McpToolsContext` + Memory snapshot 到 system prompt。
- 已有 `LocalToolRegistry`（6 个本地工具：`get_database_schema`、`query_database`、`quick_note_parse`、`quick_note_save`、`memory_search`、`memory_fact_upsert`）。
- 已有 `AIHttpClient`（OpenAI 协议 SSE 流式、function calling、SSE 解析）。
- 已有 `MemoryFacade`（人设、长期事实、跨会话记忆）。
- 已有 `TokenUsageRecorder`（按 agent_name + session_id 维度记账）。
- 现有 `analysis_sessions` / `analysis_messages` 表（单 session_id 主键，会话 + 消息列表）。
- 配置数据全部写在 SQLite（如 `llm_configs`、`mcp_server_configs`），通过 Tauri 命令暴露给 Vue。

### 2.2 现有不足（为飞书接入需要补的）

- 无任何远程入站通道（无 webhook、无 SDK 客户端）；
- 无 channel/adapter 抽象；
- `send_analysis_message_stream`（`lib.rs:960`）把 LLM 调用 + 工具循环 + `app.emit` 推前端这些逻辑揉在一个函数里，需要抽出成可复用模块（让飞书 bot 和 Vue 前端共享）；
- 会话 id 现状只与 Vue 端 UUID 绑定，无法表达"飞书用户 X 在群 G 里的会话"。

### 2.3 参考项目精要

|       | openclaw                                       | hermes-agent                              |
| ----- | ---------------------------------------------- | ----------------------------------------- |
| 语言  | TypeScript / Node.js                            | Python / asyncio                          |
| SDK   | `@larksuiteoapi/node-sdk`                       | `lark-oapi`                               |
| 入站  | WebSocket（默认） + Webhook（HTTP 备选）        | WebSocket（默认） + Webhook（HTTP 备选）   |
| 出站  | REST：`im.message.create / reply / patch`       | REST：`im.v1.message.create / reply`       |
| 鉴权  | 由 SDK 内部维护 `tenant_access_token`          | 由 SDK 内部维护 `tenant_access_token`     |
| 加解密 | webhook 模式 SHA256 签名 + URL verification     | 同左                                      |
| 群策略 | open / allowlist / disabled + per-group 配置    | 同左                                      |
| 去重  | 内存 + 持久化                                   | 内存（LRU）+ 持久化 JSON                  |
| 批合并 | 输入 debounce + 顺序队列                        | 文本 0.6s / 媒体 0.8s 合并                 |

我们将参考这两者的设计骨架，在 Rust 端做一个**精简**版本（先满足 MVP，然后渐进）。

---

## 3. 整体架构

### 3.1 架构图

```
┌──────────────────────────────────────────────────────────────────┐
│                        Tauri 主进程（单进程）                       │
│                                                                    │
│  ┌─────────────────┐   invoke    ┌──────────────────────────────┐  │
│  │  Vue 前端 (UI)   │ ─────────► │ Tauri Commands  (lib.rs)      │  │
│  │  - 设置面板       │             │  - feishu_save_config         │  │
│  │  - 智能分析对话   │ ◄──────── │  - feishu_start / stop        │  │
│  └─────────────────┘   emit      │  - feishu_get_status          │  │
│           ▲                       │  - send_analysis_message_…    │  │
│           │                       └────────────┬─────────────────┘  │
│           │                                    │                     │
│           │ analysis-stream-chunk              │ 触发                 │
│           │                                    ▼                     │
│  ┌────────┴────────────────────────────────────────────────────┐    │
│  │           AnalysisPipeline (新抽象, 复用核心)                │    │
│  │  fn run(input, sink) -> 协调 LLM + tools + memory           │    │
│  └────────────────────────────────────────────────────────────┘    │
│         ▲                                            ▲             │
│         │                                            │             │
│  ┌──────┴────────┐                          ┌────────┴───────────┐  │
│  │  VueSink      │                          │  FeishuSink         │  │
│  │ (推 Vue UI)   │                          │  (推飞书 IM)        │  │
│  └───────────────┘                          └─────────▲──────────┘  │
│                                                        │             │
│                                                        │ 双向        │
│                                                        ▼             │
│                                  ┌──────────────────────────────┐   │
│                                  │   FeishuChannel (新模块)      │   │
│                                  │   ├── FeishuClient (REST)     │   │
│                                  │   ├── FeishuWsConn (WebSocket)│   │
│                                  │   ├── EventDispatcher         │   │
│                                  │   ├── InboundRouter (会话路由) │   │
│                                  │   ├── Dedup / Policy / Locks  │   │
│                                  │   └── BotIdentity (探测)      │   │
│                                  └──────────────┬───────────────┘   │
│                                                 │ open-lark crate     │
└─────────────────────────────────────────────────┼──────────────────┘
                                                  │
                                                  ▼
                                        ┌──────────────────┐
                                        │  飞书开放平台     │
                                        │  (open.feishu.cn) │
                                        └──────────────────┘
```

### 3.2 组件职责

1. **FeishuChannel**（新模块）
   - 唯一对外的远程通道。维护一个 `FeishuClient`（REST 调用）和一个 `FeishuConnection`（WebSocket 长连接）。
   - 启动时探测 bot 自身身份（open_id / bot_name），用于自我消息过滤。
   - 接收事件 → 解析 → 命令拦截 → 路由会话 → 调 `AnalysisPipeline`。
   - 执行回复（私聊 reply / create）。

2. **AnalysisPipeline**（新抽象）
   - 把 `lib.rs:960` 的 `send_analysis_message_stream` 抽成可复用的 service：
     - 输入：`session_id, user_text, optional config_id, MessageSink`
     - 内部：取 LLM 配置、构上下文、跑 tool-call 循环、把 chunk 推给 sink、最后落盘 message。
     - 输出：每个分块通过 `MessageSink` trait 推送。
   - 两个实现：
     - `VueSink`：调 `app.emit("analysis-stream-chunk", ...)`（保持向后兼容）。
     - `FeishuSink`：累积 chunk，到完整消息或一定阈值后调 `FeishuClient.send_message(...)`。

3. **MessageSink trait**（新抽象，async）
   ```rust
   #[async_trait]
   pub trait MessageSink: Send + Sync {
       async fn on_chunk(&self, chunk: &str);
       async fn on_tool_status(&self, status: ToolStatus);
       async fn on_done(&self, full_text: &str);
       async fn on_error(&self, err: &str);
   }
   ```

4. **InboundRouter**（在 FeishuChannel 内）
   - 负责 `飞书 open_id → 内部 session_id` 映射（仅私聊）。
   - 维护"当前会话指针"：每个 open_id 一个 `current_session_id`；`/new` 时刷新。
   - 自我消息过滤（sender == bot_open_id 时丢弃）。
   - 群消息直接丢弃（chat_type != "p2p"）。

5. **CommandRouter**（在 FeishuChannel 内，新增）
   - 解析消息文本是否为内置命令（如 `/new`），命令直接处理（不走 LLM）；
   - 非命令文本透传到 `AnalysisPipeline`。

6. **现有 `send_analysis_message_stream` 命令**
   - 保持外部接口不变（向后兼容）；
   - 内部改为：构造 `VueSink` → 调 `AnalysisPipeline::run(...)`。

---

## 4. 关键技术选型

### 4.1 Rust 飞书 SDK：`open-lark` ✅ 已确定

- `open-lark`（[crates.io/open-lark](https://crates.io/crates/open-lark)）是社区维护、功能最完整的 Rust 飞书 SDK，已支持：
  - 自建机器人、长连接 WebSocket bot、消息收发、卡片、群、文档；
  - 基于 `tokio` 异步、SDK 内部维护 tenant_access_token；
  - 提供完整的 `websocket_client.rs` 例子。

→ **采用 `open-lark`**。如发现关键能力缺失（比如某个 OAPI 不支持），可以混入直接 `reqwest` 调用兜底。

### 4.2 接收协议：WebSocket（长连接）✅ 已确定

|             | WebSocket（长连接）      | Webhook（HTTP 入站）             |
| ----------- | ----------------------- | ------------------------------- |
| 桌面应用是否合适 | ✅ 不需要公网入口        | ❌ 需要在用户机器上开端口 + 公网映射 |
| 实现复杂度    | 低（SDK 接管心跳/重连）   | 中（要起 axum server + 签名校验）  |
| 飞书侧配置    | 仅需创建应用即可          | 需配置回调 URL（用户必须有公网 URL） |

→ MVP **仅实现 WebSocket**。Webhook 不在范围内。

### 4.3 Agent 路由策略：全部走 AnalysisAgent + 命令分流 ✅ 已确定

- **默认行为**：用户文本消息直接送入 `AnalysisAgent`（含工具调用循环），让 LLM 自行决定是查询还是调 `quick_note_*` 工具记账。
- **会话接续**：默认情况下复用该飞书用户的"当前会话"（current session），**不创建新 session**，跨多次聊天保持上下文。
- **内置斜杠命令**（在 `feishu::inbound` 内提前拦截，不走 LLM）：

  | 命令       | 语义                                                                              |
  | ---------- | --------------------------------------------------------------------------------- |
  | `/new`     | 创建新会话并设为"当前会话"；后续消息接续到新会话。可选携带首条消息：`/new 帮我做月预算` |
  | `/help`    | 列出所有可用命令（Stretch）                                                        |
  | `/status`  | 返回当前会话标题、消息数、bot 身份、连接状态（Stretch）                              |

  其余以 `/` 开头但未注册的输入：原样转给 LLM（避免误判用户的笔记内容，比如 "/月支出"）。命令识别规则在 §8.4 详述。

### 4.4 会话粒度：私聊 + 当前会话指针 ✅ 已确定

- **每个飞书用户**（`open_id`）对应一个"当前会话指针"（`current_session_id`）；
- 默认所有消息接续到 `current_session_id`，跨次聊天上下文不丢；
- `/new` 命令：用 `uuid v4` 生成新 `session_id`，写入 `analysis_sessions`（带 `source='feishu'`），更新指针为新会话；如果命令带初始文本，把首条消息直接送入 LLM；
- 历史会话仍然存于 `analysis_sessions`，可在 Vue 端"智能分析"列表查看（带 `飞书` 标记）。

### 4.5 群消息：本期不实现 ✅ 已确定

不订阅 / 不响应群消息。`im.message.receive_v1` 进来时，先看 `chat_type`：
- `chat_type == "p2p"` → 处理；
- `chat_type == "group"` → **忽略**（仅记 debug 日志，不回复，不建会话）。

### 4.6 启停控制

参考 `mcp_server_configs` 现有模式（一条配置 + `enabled` 字段 + 启停命令）：
- 在 SQLite 里存配置；
- 暴露 `feishu_start / feishu_stop` Tauri 命令；
- 每次桌面 app 启动时，根据 `enabled` 字段决定是否自动启动；
- 状态用 `feishu_get_status` 查询。

---

## 5. 模块划分（Rust 代码组织）

新增顶层模块 `src-tauri/src/feishu/`：

```
src-tauri/src/feishu/
├── mod.rs              # pub use 导出；FeishuState（全局状态结构）
├── config.rs           # FeishuConfig + load/save（数据库 CRUD）
├── client.rs           # FeishuClient（REST 包装），open-lark Client 创建
├── connection.rs       # FeishuConnection trait + WsConnection 实现
├── dispatcher.rs       # EventDispatcher：注册 lark 事件 handler
├── identity.rs         # 启动时探测 bot_open_id / bot_name
├── inbound.rs          # 入站消息解析 + 派发：parse → dedup → drop_group → command? → enqueue
├── commands.rs         # 内置斜杠命令：/new（必备）、/help、/status（Stretch）
├── routing.rs          # InboundRouter：当前会话指针 + 创建/查找 session
├── outbound.rs         # FeishuSink + sender（reply/create/post fallback）
├── pipeline_bridge.rs  # 把入站消息桥接到 AnalysisPipeline
├── locks.rs            # per open_id 串行锁
├── dedup.rs            # 简单 LRU 去重 + 24h TTL
├── error.rs            # FeishuError 类型
└── tests/              # 单元测试（mock SDK / 协议解析 / 命令解析）
```

抽象出来的"流水线"在新模块 `src-tauri/src/pipeline/` 中：

```
src-tauri/src/pipeline/
├── mod.rs              # pub use AnalysisPipeline; pub trait MessageSink
├── analysis.rs         # AnalysisPipeline::run(...) —— 从 send_analysis_message_stream 抽出
├── sink.rs             # MessageSink trait + VueSink + FeishuSink
└── tests/
```

`lib.rs` 增量改动：
- 新增 `mod feishu;` 和 `mod pipeline;`；
- `send_analysis_message_stream` 内部改写为构造 `VueSink` → `pipeline::analysis::run(...)`；
- 新增 5 个 Tauri 命令并加入 `generate_handler!` 列表（详见 §7）；
- `setup` 中：DB 初始化成功后，新增 `app.manage(FeishuState::new(...))`；并在 `feishu_configs.enabled=1` 时自动 `tokio::spawn(channel.start())`。

---

## 6. 数据库改动

### 6.1 新增表 `feishu_configs`（参考 `mcp_server_configs` schema 风格）

```sql
CREATE TABLE IF NOT EXISTS feishu_configs (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT    NOT NULL DEFAULT 'default',
    app_id          TEXT    NOT NULL,
    app_secret      TEXT    NOT NULL,
    domain          TEXT    NOT NULL DEFAULT 'feishu',     -- feishu | lark
    -- 路由
    bind_llm_config_id INTEGER,                            -- 可选：固定使用某个 LLM 配置；否则用 active
    bind_role_scope    TEXT NOT NULL DEFAULT 'analysis',   -- 用于 Memory 场景
    -- 状态
    enabled         INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT    NOT NULL,
    updated_at      TEXT    NOT NULL
);
```

MVP 仅一条记录（`name='default'`），UI 不暴露多账号；为多账号保留扩展空间。
群相关字段（`require_mention`、`group_policy`、`allowed_chats` 等）本期**不引入**，后续若开群功能再迁移加列。

### 6.2 新增表 `feishu_user_sessions`（飞书用户 → 当前会话指针）

```sql
CREATE TABLE IF NOT EXISTS feishu_user_sessions (
    id                  INTEGER PRIMARY KEY AUTOINCREMENT,
    -- 飞书侧坐标（仅 P2P）
    user_open_id        TEXT    NOT NULL,
    user_name           TEXT,                  -- 缓存的显示名（首次拉到后续复用）
    -- 当前会话指针
    current_session_id  TEXT    NOT NULL,      -- 指向 analysis_sessions.session_id
    -- 元信息
    last_message_at     TEXT,
    created_at          TEXT    NOT NULL,
    updated_at          TEXT    NOT NULL,
    UNIQUE(user_open_id)
);

CREATE INDEX IF NOT EXISTS idx_feishu_user_sessions_session
    ON feishu_user_sessions(current_session_id);
```

会话生命周期：

1. 飞书用户首次发消息 → 找不到记录 → 生成新 `session_id`（uuid v4）→ 写 `analysis_sessions`（带 `source='feishu'`）→ 创建 `feishu_user_sessions` 行。
2. 后续消息默认接续 `current_session_id` → 写 `analysis_messages`（带 `source='feishu'`）。
3. 用户发 `/new [可选首条话题]` → 生成新 `session_id` → `UPDATE feishu_user_sessions SET current_session_id=...` → 若命令带正文，立即把首条作为新会话第一条用户消息送入 LLM。
4. 历史会话仍保存于 `analysis_sessions`，可在 Vue UI"智能分析"列表查看。

> 命名为 `feishu_user_sessions` 而非通用的 `channel_sessions`，原因：MVP 仅私聊、仅一个 channel；后续接入更多平台或群功能时，再扩为更通用的 `channel_sessions(channel, chat_id, user_open_id, thread_id, current_session_id)`，迁移成本低。

### 6.3 现有表 `analysis_sessions` / `analysis_messages` 增加 `source` 列

```sql
ALTER TABLE analysis_sessions ADD COLUMN source TEXT NOT NULL DEFAULT 'local';
ALTER TABLE analysis_messages  ADD COLUMN source TEXT NOT NULL DEFAULT 'local';
```

- `source` 取值：`'local'`（来自桌面 UI）或 `'feishu'`（来自飞书）；
- Vue 端读取后用于在会话列表与消息条上展示徽标（`飞书`），并保留筛选/隔离的可能性；
- 写入时机：
  - `pipeline::analysis::run` 接受新参数 `source: SessionSource`，把它带入 `ensure_analysis_session` 与 `save_analysis_message`；
  - 现有 `send_analysis_message_stream` 命令默认传 `local`；
  - 飞书桥接传 `feishu`。

> SQLite `ALTER TABLE ADD COLUMN`（带默认值）是 O(1) 的，安全可靠。

### 6.4 飞书消息去重（MVP 内存即可）

仅用内存 LRU（`lru` crate 或自建 HashMap + 容量截断 + TTL），不持久化。
重启后偶发重复消息可接受（飞书已自带 message_id 单调；SDK 也兜底）。

### 6.5 迁移脚本位置

在 `database.rs` 现有的迁移函数中追加（`CREATE TABLE IF NOT EXISTS` + `ALTER TABLE ... ADD COLUMN` 配合现有迁移版本号机制）。沿用现有风格，不引入额外迁移框架。

---

## 7. Tauri 命令（前端 ↔ 后端）

新增以下命令，都加入 `lib.rs:1596` 的 `generate_handler!` 中：

| 命令名                   | 入参                                | 出参                  | 功能                                  |
| ------------------------ | ----------------------------------- | --------------------- | ------------------------------------- |
| `feishu_get_config`      | —                                   | `FeishuConfig`        | 读取当前配置（无则返回默认空）          |
| `feishu_save_config`     | `FeishuConfigInput`                 | `()`                  | 保存配置（更新或创建唯一一条）          |
| `feishu_test_credential` | `app_id, app_secret, domain`        | `BotIdentity`         | 调 `bot.info` 测试凭据有效性            |
| `feishu_start`           | —                                   | `FeishuStatus`        | 启动 bot（拉起 ws 连接）                |
| `feishu_stop`            | —                                   | `FeishuStatus`        | 停止 bot                                |
| `feishu_get_status`      | —                                   | `FeishuStatus`        | 当前连接状态、最后错误、bot 身份等       |
| `feishu_list_recent_events` | `limit?: u32`                    | `Vec<FeishuEvent>`    | （Stretch）最近事件，便于前端排查        |

向前端发的事件（保持现有命名风格）：

- `feishu-status-change`：`FeishuStatus { running, error?, bot_open_id?, ... }`
- `feishu-event-recv`：（Stretch，调试用）

### 7.1 关键 DTO（精简）

```rust
#[derive(Serialize, Deserialize)]
pub struct FeishuConfig {
    pub app_id: String,
    pub app_secret: String,                   // 与现有 LLM api_key 同处理，明文存 SQLite
    pub domain: String,                       // "feishu" | "lark"
    pub bind_llm_config_id: Option<i64>,
    pub bind_role_scope: String,              // "analysis"
    pub enabled: bool,
}

#[derive(Serialize, Deserialize)]
pub struct FeishuStatus {
    pub running: bool,
    pub bot_open_id: Option<String>,
    pub bot_name: Option<String>,
    pub last_error: Option<String>,
    pub last_event_at: Option<String>,
    pub connected_since: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct BotIdentity {
    pub open_id: String,
    pub name: String,
}
```

### 7.2 凭据存储

参考现有 `llm_configs.api_key` 的处理方式（明文存于 SQLite，应用层认为本地数据库可信）。MVP 不引入额外加密层。

---

## 8. 数据流（端到端）

### 8.1 入站（飞书 → 用户回复）

```
[1] open-lark WsClient 收到事件
      │ p2_im_message_receive_v1
      ▼
[2] feishu::dispatcher 注册的回调 on_message
      │ async move ...
      ▼
[3] feishu::inbound::parse_message
      │  - 从 event.event.message 提取 message_id, message_type, content,
      │    chat_id, chat_type, sender_id (open_id), root_id, parent_id, thread_id
      │  - msg_type=text → JSON {"text": ...}；msg_type=post → 渲染 markdown；其它先忽略 (Stretch)
      ▼
[4] feishu::dedup.try_insert(message_id) → 已见过则丢弃
      ▼
[5] 准入过滤
      │  - chat_type != "p2p" → 直接 drop（本期不实现群）
      │  - sender_open_id == bot_open_id → drop（自我消息）
      ▼
[6] feishu::commands::parse(text)
      │  - 命令前缀检测：text 首字符为 '/' 且首词 ∈ {"/new","/help","/status"}
      │  - 命中 → 执行命令并直接回复，return
      │  - 未命中（包括 '/月支出' 等） → 继续走 LLM 流程
      ▼
[7] feishu::routing.resolve_session(user_open_id)
      │  - SELECT current_session_id FROM feishu_user_sessions WHERE user_open_id=?
      │  - 不存在 → 生成新 session_id（uuid v4），写 analysis_sessions(source='feishu') + feishu_user_sessions
      │  - 返回 session_id
      ▼
[8] feishu::locks.acquire(user_open_id) → per-user asyncio::Mutex（防并发）
      ▼
[9] pipeline::analysis::run(session_id, user_text, source='feishu', sink=FeishuSink{...})
      │  - 读取 LLM 配置（按 bind_llm_config_id 或 active）
      │  - 取最近 20 条 analysis_messages（自动接续上下文）
      │  - 构 system prompt（FinancialContext + Memory snapshot + MCP tools）
      │  - SSE 流式调用 + tool-call 循环（最多 8 轮，复用现有逻辑）
      │  - 把 user 消息以 source='feishu' 入库
      │  - 流式 chunk 调 sink.on_chunk(...)
      │  - 完成后 sink.on_done(full_text)
      │  - assistant 消息以 source='feishu' 入库 + 更新 feishu_user_sessions.last_message_at
      ▼
[10] FeishuSink.on_done → FeishuClient.send_message_text(open_id, full_text, reply_to=msg_id)
      │  - msg_type=text 优先；若文本含 markdown 标记，用 post(md) 兜底
      │  - reply 失败码 230011 / 231003 → fallback 为 create
```

### 8.2 出站（仅由 pipeline 触发）

回到 §3.1 架构图。出站完全由 `FeishuSink` 触发，外部不主动 push。

### 8.3 启动流程

```
Tauri main → setup()
   ├─ app.manage(McpState)
   └─ tokio::spawn(async move {
        Database::new(...).await
        ├─ app.manage(DatabaseState)
        ├─ app.manage(FeishuState::new(db_pool, app_handle))    [NEW]
        └─ if feishu_configs.enabled:
             FeishuChannel::start_in_background(state)            [NEW]
                ├─ FeishuClient::new(app_id, app_secret, domain)
                ├─ identity.probe()  → bot_open_id / bot_name
                ├─ ws_client = WsClient::new(...)
                ├─ ws_client.start(event_handler) (后台任务)
                └─ 状态写入 FeishuState (Arc<RwLock<...>>)
   })
```

### 8.4 命令解析规则

为避免误判用户的笔记内容（"/月支出" / "/etc/passwd" / 路径式记账等），命令识别采用**白名单**而非"凡是 / 开头都拦截"：

```rust
fn try_parse_command(text: &str) -> Option<Command> {
    // 1. trim + 取首词（按空白分隔）
    // 2. 首词必须以 '/' 开头
    // 3. 首词去掉 '/' 后必须是已注册命令名（new / help / status）
    // 4. 命令后正文（split 第二段，可空）
    let trimmed = text.trim();
    let mut iter = trimmed.splitn(2, char::is_whitespace);
    let head = iter.next()?;
    if !head.starts_with('/') { return None; }
    let name = &head[1..];
    let rest = iter.next().unwrap_or("").trim();
    match name {
        "new"    => Some(Command::New { initial: rest.to_string() }),
        "help"   => Some(Command::Help),
        "status" => Some(Command::Status),
        _        => None,   // 未知命令：透传给 LLM，避免误伤用户文本
    }
}
```

`/new` 命令处理：

```
[A] generate new session_id (uuid v4)
[B] insert analysis_sessions(session_id, title="新会话", source='feishu')
[C] UPDATE feishu_user_sessions SET current_session_id=:new WHERE user_open_id=:uid
[D] if rest.is_empty():
        FeishuClient.send_text(open_id, "✅ 已开启新会话，请直接发送你的问题")
    else:
        将 rest 作为新会话第一条用户输入，立即送入 [9] pipeline::analysis::run
```

### 8.5 停止流程

`feishu_stop` 命令：
1. 设置 `FeishuState.running=false`；
2. abort 后台 ws 任务的 JoinHandle；
3. emit `feishu-status-change` 给 Vue。

---

## 9. 错误处理 / 可靠性

### 9.1 重连

- WebSocket：依赖 `open-lark` SDK 自带的心跳 + 重连策略；外层只关心"是否最终失败"。SDK 暴露的回调里收到致命错误 → 写入 `last_error`，emit 状态变更，标记 `running=false`，不阻塞主进程。
- REST 调用：`reqwest` 默认 + 简单 3 次指数退避（参考 hermes-agent 的 `_feishu_send_with_retry`）。

### 9.2 启停一致性

- 用 `tokio::sync::Mutex` 保护 `FeishuState`，避免重复 start。
- 用 `tokio::task::AbortHandle` 实现可中断的后台任务。

### 9.3 消息重复

- 内存 LRU 去重（容量 2048，TTL 24 小时）。
- 飞书会在网络抖动时重发同一 `message_id`（SDK 也会兜底）。

### 9.4 长消息处理

- 飞书单条文本 ≤ 8000 字符；超出按现有 `truncate_message` 策略切片，分多条发送（参考 hermes-agent）。
- post (markdown) 渲染失败时 fallback 为纯文本，避免飞书 API 报错（参考 hermes-agent 的 `_strip_markdown_to_plain_text`）。

### 9.5 错误暴露

- 任何错误都写入 `FeishuState.last_error` 并 emit `feishu-status-change`；
- Vue 设置面板显示状态徽标 + 最近错误；
- 控制台日志走现有 `eprintln!` 风格（与项目现状对齐）。

---

## 10. 实施里程碑

每个里程碑可独立 review、独立合并。每个里程碑都不会破坏现有桌面 app 的可用性。

### M1：基础设施（不接飞书，不破坏现状）

1. 抽出 `pipeline/analysis.rs`：
   - 把 `send_analysis_message_stream` 内部业务逻辑迁移；
   - 定义 `MessageSink` trait + `VueSink`；
   - `pipeline::analysis::run` 接收 `source: SessionSource` 参数，并把它带入 `ensure_analysis_session` / `save_analysis_message`；
   - 改写原 Tauri 命令为薄壳：构造 `VueSink` → `pipeline::analysis::run(..., source=SessionSource::Local)`。
2. 给 `analysis_sessions` / `analysis_messages` 增加 `source` 列（DEFAULT 'local'）。
3. 跑通现有 Vue 端"智能分析"功能（行为完全等价），加单元测试覆盖关键路径。

**验收**：Vue UI 智能分析对话与改造前一致，回归通过；DB 内 `source` 列被正确写入为 `local`。

### M2：飞书凭据 + 连接（不处理消息）

1. 新增 `feishu_configs` 表 + 迁移；
2. 新增 `mod feishu`，引入 `open-lark` 依赖；
3. 实现 `FeishuClient`、`identity.probe`；
4. 实现 `feishu_save_config / feishu_get_config / feishu_test_credential / feishu_get_status`；
5. Vue 设置面板：新增"飞书"页签（参考 `LLMConfigDialog.vue` / `McpConfigDialog.vue` 的模式）。

**验收**：在 Vue 中填入 `app_id` / `app_secret`，点"测试连接"能正确返回 bot 身份。

### M3：飞书入站消息 + 自动回复（MVP 核心）

1. 实现 `FeishuConnection`（WebSocket）；
2. 注册 `im.message.receive_v1` handler；
3. 实现 `inbound.parse_message + dedup + p2p_only_filter + bot_self_filter`；
4. 实现 `commands::parse` + `/new` 命令；
5. 实现 `routing.resolve_session`（current pointer 模式）+ 新增 `feishu_user_sessions` 表；
6. 实现 `FeishuSink` + `outbound.send_message_text`；
7. 把入站消息桥接到 `pipeline::analysis::run(..., source=SessionSource::Feishu)`；
8. `feishu_start / feishu_stop` 命令 + 自动启动逻辑。

**验收**：
- 在飞书私聊里跟 bot 说"本月餐饮花了多少"，能拿到 LLM 调用 SQL 工具后的回复；
- 连续多次消息接续同一会话；
- `/new` 命令能新建会话；带正文的 `/new ...` 也能立刻进入新会话并响应；
- 群消息发给 bot 时不响应（仅 debug 日志）；
- 数据库中 `analysis_sessions.source / analysis_messages.source` 写入 `feishu`。

### M4：Vue 端"飞书会话"区分

1. `get_analysis_sessions` / `get_analysis_messages` 返回 `source` 字段；
2. Vue 会话列表中：来自飞书的会话条目显示徽标"飞书"；
3. 会话内消息条上：来自飞书的消息附带"飞书"小标记（与本地区分开）；
4. 可选：会话筛选 tab（全部 / 本地 / 飞书）。

**验收**：UI 上能清晰区分两种来源，飞书发起的对话内容能在桌面端复盘。

### M5（Stretch）：体验优化

1. typing 表情反馈（开始处理 + 完成 / 失败）；
2. post (markdown) 富文本回复；
3. 文本长消息切片；
4. `/help`、`/status` 命令。

### M6（Stretch）：流式 / 媒体

1. 流式输出（编辑消息追加，参考 openclaw 的 streaming-card）；
2. 文本批合并（debounce 0.6s）；
3. 图片/文件接收（OCR 票据，复用 quick_note 工具）。

### M7（远期）：群功能 / Webhook

1. 群消息（@ 检测、群白名单、按用户独立会话）；
2. Webhook 模式（axum + 签名校验，自部署服务器场景）；
3. 多账号支持。

---

## 11. 风险与对策

| 风险                                      | 对策                                                      |
| ---------------------------------------- | --------------------------------------------------------- |
| open-lark SDK 缺失某个 OAPI / 行为不稳定   | 关键调用兜底用 `reqwest` 直接打 OAPI；锁定 SDK 版本 + 跑通端到端冒烟。 |
| 桌面应用关闭后 bot 自然下线，用户疑惑      | 在 Vue 中明确提示"飞书机器人随应用一起运行；关闭应用后机器人离线"。 |
| Tauri 单进程内长连接抢占 runtime          | 后台任务专门 spawn 在 `tauri::async_runtime`；网络任务都用 async 实现，不会阻塞主线程。 |
| `/new` 命令误伤用户笔记内容                | 命令解析采用白名单（仅识别 `/new /help /status`），未注册命令一律透传给 LLM。 |
| 用户在飞书海聊导致 LLM token 高消耗        | 现有 `token_usage_logs` 已记录；可在 Vue 端按 source='feishu' 绘制用量统计；后续可加日预算。 |
| 飞书 message_id 重复 / 网络抖动           | 内存 LRU 去重，超时 24h；如果实测有遗漏再上持久化。            |
| `app_secret` 明文存 SQLite               | MVP 与现有 LLM api_key 一致；后续若要上 keyring，新增独立能力即可。 |
| 飞书 SDK 协议变更                         | 只用 SDK 暴露的稳定接口；自检脚本里加 `feishu_test_credential` 烟雾。 |
| 当前会话指针错乱                          | `feishu_user_sessions.UNIQUE(user_open_id)` 保证仅一份指针；指针更新在 per-user lock 下进行，不会出现 race。 |

---

## 12. 已对齐决策清单

| #  | 决策点                              | 最终选择                                                                       |
| -- | ----------------------------------- | ----------------------------------------------------------------------------- |
| 1  | 选用 SDK                            | ✅ `open-lark`                                                                 |
| 2  | 接收协议                            | ✅ 仅 WebSocket（webhook 不在范围内，列入远期 M7）                                |
| 3  | Agent 路由                          | ✅ 全部走 AnalysisAgent；默认接续当前会话；`/new` 命令开启新会话                    |
| 4  | 群功能                              | ✅ 本期不实现，仅支持私聊（P2P）                                                  |
| 5  | （N/A，群已被排除）                  | —                                                                             |
| 6  | Vue 中区分 source="飞书"           | ✅ 是；`analysis_sessions / analysis_messages` 加 `source` 列；列表与消息条均标记 |
| 7  | `app_secret` 是否需 keyring 加密     | ✅ 否，与 LLM api_key 同处理                                                    |

---

## 13. 文件改动清单（汇总，便于估算）

### 新增

```
src-tauri/src/feishu/                 (新建模块)
src-tauri/src/pipeline/               (新建模块)
docs/FEISHU_INTEGRATION_DESIGN.md     (本文档)
src/components/FeishuConfigDialog.vue (Vue 配置对话框，参考 McpConfigDialog)
```

### 修改

```
src-tauri/Cargo.toml                  (新增 open-lark 依赖)
src-tauri/src/lib.rs                  (mod 引入 + setup 增量 + 命令注册 + send_analysis_message_stream 改薄)
src-tauri/src/database.rs             (新增 feishu_configs / feishu_user_sessions 表 + CRUD;
                                       analysis_sessions / analysis_messages 增 source 列)
src-tauri/src/models.rs               (新增 FeishuConfig / FeishuStatus / BotIdentity / SessionSource)
src/App.vue                           (设置入口加"飞书"页签)
src/views/AnalysisView.vue            (会话列表与消息条加 source 徽标)
src/stores/index.ts                   (新增飞书相关 Tauri invoke 封装)
```

### 估算

- 增量 Rust 代码：~2000–3000 行（含测试），主要在 `feishu/`；
- 增量 Vue 代码：~300–500 行；
- 增量 SQL：~30 行。

---

## 14. 与现有项目原则的对齐

- **不引入额外进程**：飞书 bot 运行在 Tauri 主进程的 tokio runtime 内 ✅
- **配置走 SQLite**：与 `llm_configs / mcp_server_configs` 一致 ✅
- **Tauri 命令风格一致**：`feishu_*` 前缀 + `Result<T, String>` 返回 ✅
- **错误处理简洁**：`anyhow::Error.to_string()` 转 String，与现有命令一致 ✅
- **无副作用迁移**：M1 仅做内部抽出，外部行为不变 ✅
- **复用现有能力**：LLM 配置、Memory、Token 计数、MCP 工具、本地工具都直接复用 ✅

---

## 15. 后续阅读 / 参考索引

- openclaw：`/Users/gehao/Programs/openclaw/extensions/feishu/`（核心：`monitor.transport.ts`、`bot.ts`、`send.ts`、`channel.ts`）
- hermes-agent：`/Users/gehao/Programs/hermes-agent/gateway/platforms/feishu.py`（核心：`FeishuAdapter._connect_websocket`、`_on_message_event`、`_send_raw_message`）
- open-lark crate：[crates.io/crates/open-lark](https://crates.io/crates/open-lark)
- Lark 官方协议：[open.feishu.cn 文档中心](https://open.feishu.cn/document)（事件订阅、消息卡片、长连接 bot）
- 现有项目流水线：`src-tauri/src/lib.rs:960` `send_analysis_message_stream`（待抽出）

---

> **下一步**：决策已在 §12 对齐，按 §10 里程碑顺序实施。建议先做 **M1（基础设施抽出）** —— 对桌面应用无感改动，可独立验收合并；然后 M2（凭据 + 测试连接）→ M3（MVP 核心）→ M4（Vue 区分飞书来源）。每完成一个里程碑等你 review 后再继续下一个。
