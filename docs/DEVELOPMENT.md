# 开发指南

本文档提供 MoneySage 的详细开发指南。

## 📋 目录

- [环境搭建](#环境搭建)
- [项目结构](#项目结构)
- [开发工作流](#开发工作流)
- [技术栈详解](#技术栈详解)
- [数据库设计](#数据库设计)
- [API 文档](#api-文档)
- [测试](#测试)
- [调试](#调试)
- [性能优化](#性能优化)

---

## 环境搭建

### 前置要求

1. **Node.js 18+**

   ```bash
   node --version  # 应该 >= 18.0.0
   npm --version
   ```

2. **Rust 1.70+**

   ```bash
   rustc --version  # 应该 >= 1.70
   cargo --version
   ```

3. **Git**
   ```bash
   git --version
   ```

### 克隆项目

```bash
git clone https://github.com/yourusername/money-sage.git
cd money-sage
```

### 安装依赖

```bash
# 安装前端依赖
npm install

# Rust 依赖会在首次构建时自动下载
```

### IDE 设置

#### VS Code (推荐)

安装推荐的扩展：

- Vue - Official (Volar)
- rust-analyzer
- Tauri
- Prettier - Code formatter
- ESLint

配置已包含在 `.vscode/settings.json` 中。

---

## 项目结构

```
money-sage/
├── src/                          # Vue 前端源码
│   ├── components/               # Vue 组件
│   │   ├── QuickBookingDialog.vue
│   │   ├── LLMConfigDialog.vue
│   │   ├── EditTransactionDialog.vue
│   │   └── AddBudgetDialog.vue
│   ├── views/                    # 页面视图
│   │   ├── DashboardView.vue
│   │   ├── TransactionsView.vue
│   │   ├── CategoriesView.vue
│   │   ├── BudgetView.vue
│   │   ├── StatisticsView.vue
│   │   └── ImportExportView.vue
│   ├── stores/                   # Pinia 状态管理
│   │   └── index.ts
│   ├── App.vue                   # 根组件
│   └── main.ts                   # 入口文件
│
├── src-tauri/                    # Rust 后端源码
│   ├── src/
│   │   ├── ai/                   # AI 模块
│   │   │   ├── agent/
│   │   │   │   ├── base.rs       # Agent 基础接口
│   │   │   │   ├── quick_note.rs # 快速记账 Agent
│   │   │   │   └── analysis.rs   # 分析 Agent
│   │   │   └── mod.rs
│   │   ├── database.rs           # 数据库操作
│   │   ├── models.rs             # 数据模型
│   │   ├── utils/                # 工具函数
│   │   │   ├── http_client.rs
│   │   │   └── mod.rs
│   │   ├── lib.rs                # 库入口
│   │   └── main.rs               # 主程序
│   ├── Cargo.toml                # Rust 依赖
│   └── tauri.conf.json           # Tauri 配置
│
├── public/                       # 静态资源
├── dist/                         # 构建输出
├── docs/                         # 文档
├── .github/                      # GitHub 配置
│   ├── workflows/                # CI/CD
│   └── ISSUE_TEMPLATE/           # Issue 模板
├── package.json                  # Node.js 配置
├── vite.config.ts                # Vite 配置
├── tsconfig.json                 # TypeScript 配置
└── README.md                     # 项目说明
```

---

## 开发工作流

### 启动开发服务器

```bash
# 同时启动前端和后端（推荐）
npm run tauri:dev

# 或分别启动
npm run dev          # 仅前端 (localhost:1420)
npm run tauri        # Tauri 开发模式
```

### 常用命令

```bash
# 前端相关
npm run dev          # 启动 Vite 开发服务器
npm run build        # 构建前端

# Tauri 相关
npm run tauri:dev    # Tauri 开发模式
npm run tauri:build  # 构建生产版本

# 完整构建
npm run build:installer  # 前端 + 后端 + 安装包
```

### 代码风格

```bash
# 前端
npm run lint         # ESLint 检查
npm run format       # Prettier 格式化

# 后端
cd src-tauri
cargo fmt            # 格式化代码
cargo clippy         # Lint 检查
```

### Git 工作流

```bash
# 1. 创建特性分支
git checkout -b feature/your-feature

# 2. 开发并提交
git add .
git commit -m "feat: your feature description"

# 3. 推送并创建 PR
git push origin feature/your-feature
```

---

## 技术栈详解

### 前端架构

#### Vue 3 组合式 API

```typescript
<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'

const count = ref(0)
const double = computed(() => count.value * 2)

onMounted(() => {
  console.log('组件已挂载')
})
</script>
```

#### Pinia 状态管理

```typescript
// stores/index.ts
export const useAppStore = defineStore('app', {
  state: () => ({
    transactions: [],
    categories: [],
  }),
  actions: {
    async fetchTransactions() {
      // 调用 Tauri 命令
    },
  },
})
```

#### Element Plus 组件

```vue
<template>
  <el-button type="primary" @click="handleClick"> 点击 </el-button>
</template>
```

### 后端架构

#### Tauri 命令系统

```rust
#[tauri::command]
async fn add_transaction(
    transaction: Transaction,
    state: State<'_, AppState>
) -> Result<Transaction, String> {
    // 业务逻辑
    Ok(transaction)
}
```

#### 数据库操作

```rust
use sqlx::SqlitePool;

pub async fn create_transaction(
    pool: &SqlitePool,
    transaction: &Transaction
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO transactions (...) VALUES (...)",
        // ...
    )
    .execute(pool)
    .await?;

    Ok(())
}
```

#### AI Agent 模式

```rust
#[async_trait]
pub trait Agent {
    async fn process(&self, input: &str) -> Result<AgentResponse>;
}

pub struct QuickNoteAgent {
    client: HttpClient,
    config: LLMConfig,
}

impl Agent for QuickNoteAgent {
    async fn process(&self, input: &str) -> Result<AgentResponse> {
        // AI 处理逻辑
    }
}
```

---

## 数据库设计

### 表结构

#### transactions (交易记录)

```sql
CREATE TABLE transactions (
    id TEXT PRIMARY KEY,
    amount REAL NOT NULL,
    type TEXT NOT NULL,           -- 'income' | 'expense'
    category_id TEXT NOT NULL,
    description TEXT,
    date TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (category_id) REFERENCES categories(id)
);
```

#### categories (分类)

```sql
CREATE TABLE categories (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    type TEXT NOT NULL,           -- 'income' | 'expense'
    icon TEXT NOT NULL,
    color TEXT NOT NULL,
    is_system INTEGER NOT NULL,   -- 0 | 1
    created_at TEXT NOT NULL
);
```

#### budgets (预算)

```sql
CREATE TABLE budgets (
    id TEXT PRIMARY KEY,
    category_id TEXT NOT NULL,
    amount REAL NOT NULL,
    period TEXT NOT NULL,         -- 'week' | 'month' | 'year'
    start_date TEXT NOT NULL,
    end_date TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (category_id) REFERENCES categories(id)
);
```

### 索引

```sql
CREATE INDEX idx_transactions_date ON transactions(date);
CREATE INDEX idx_transactions_category ON transactions(category_id);
CREATE INDEX idx_transactions_type ON transactions(type);
```

---

## API 文档

### Tauri 命令

#### 交易相关

```typescript
// 获取所有交易
await invoke('get_transactions'): Promise<Transaction[]>

// 添加交易
await invoke('add_transaction', { transaction }): Promise<Transaction>

// 更新交易
await invoke('update_transaction', { id, transaction }): Promise<Transaction>

// 删除交易
await invoke('delete_transaction', { id }): Promise<void>
```

#### 分类相关

```typescript
await invoke('get_categories'): Promise<Category[]>
await invoke('add_category', { category }): Promise<Category>
await invoke('delete_category', { id }): Promise<void>
```

#### 预算相关

```typescript
await invoke('get_budgets'): Promise<Budget[]>
await invoke('add_budget', { budget }): Promise<Budget>
await invoke('update_budget', { id, budget }): Promise<Budget>
await invoke('delete_budget', { id }): Promise<void>
```

#### AI 相关

```typescript
await invoke('quick_booking_parse', {
  text: string
}): Promise<ParsedTransaction[]>

await invoke('save_llm_config', {
  config: LLMConfig
}): Promise<void>

await invoke('get_llm_config'): Promise<LLMConfig | null>
```

### 数据类型

```typescript
interface Transaction {
  id: string
  amount: number
  type: 'income' | 'expense'
  categoryId: string
  description: string
  date: string // ISO 8601
  createdAt: string
  updatedAt: string
}

interface Category {
  id: string
  name: string
  type: 'income' | 'expense'
  icon: string
  color: string
  isSystem: boolean
  createdAt: string
}

interface Budget {
  id: string
  categoryId: string
  amount: number
  period: 'week' | 'month' | 'year'
  startDate: string
  endDate: string
  createdAt: string
}
```

---

## 测试

### 后端测试

```bash
cd src-tauri

# 运行所有测试
cargo test

# 运行特定测试
cargo test quick_booking

# 显示输出
cargo test -- --nocapture

# 测试覆盖率
cargo tarpaulin
```

### 编写测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_transaction() {
        // 测试逻辑
    }
}
```

---

## 调试

### 前端调试

1. **Chrome DevTools**

   - 在开发模式下，应用内置 DevTools
   - 快捷键: `F12` 或 `Ctrl+Shift+I`

2. **Vue DevTools**

   - 安装浏览器扩展
   - 查看组件树和状态

3. **Console 日志**
   ```typescript
   console.log('debug info', data)
   ```

### 后端调试

1. **打印调试**

   ```rust
   println!("Debug: {:?}", value);
   dbg!(&variable);
   ```

2. **日志系统**

   ```rust
   use log::{info, warn, error};

   info!("Transaction added: {:?}", transaction);
   error!("Database error: {}", err);
   ```

3. **VS Code 调试**
   - 设置断点
   - 使用 rust-analyzer 的调试功能

---

## 性能优化

### 前端优化

1. **组件懒加载**

   ```typescript
   const DashboardView = defineAsyncComponent(() => import('./views/DashboardView.vue'))
   ```

2. **虚拟滚动**

   - 对于大列表使用虚拟滚动
   - Element Plus 的 `el-virtual-list`

3. **计算属性缓存**
   ```typescript
   const expensiveComputed = computed(() => {
     // 只在依赖变化时重新计算
   })
   ```

### 后端优化

1. **数据库索引**

   - 为常查询字段添加索引
   - 避免全表扫描

2. **批量操作**

   ```rust
   // 使用事务批量插入
   let mut tx = pool.begin().await?;
   for item in items {
       // insert
   }
   tx.commit().await?;
   ```

3. **异步并发**

   ```rust
   use tokio::try_join;

   let (result1, result2) = try_join!(
       async_operation1(),
       async_operation2()
   )?;
   ```

---

## 构建和发布

### 开发构建

```bash
npm run build:installer:debug
```

### 生产构建

```bash
npm run build:installer
```

### 更新版本

```bash
# 1. 更新版本号
# - package.json
# - src-tauri/tauri.conf.json
# - src-tauri/Cargo.toml

# 2. 构建
npm run build:installer

# 3. 创建 tag
git tag -a v0.1.0 -m "Release v0.1.0"
git push origin v0.1.0
```

---

## 常见问题

### 前端

**Q: 热重载不工作？**

- 检查 Vite 配置
- 重启开发服务器

**Q: Element Plus 样式不生效？**

- 确认已导入样式
- 检查深色主题配置

### 后端

**Q: 编译错误？**

- 清理缓存: `cargo clean`
- 更新依赖: `cargo update`

**Q: 数据库迁移？**

- 查看 `database.rs` 中的初始化代码
- 使用 SQLx 迁移工具

---

## 资源链接

- [Vue 3 文档](https://vuejs.org/)
- [Tauri 文档](https://tauri.app/)
- [Rust 官方书](https://doc.rust-lang.org/book/)
- [Element Plus 文档](https://element-plus.org/)
- [SQLx 文档](https://github.com/launchbadge/sqlx)

---

## 贡献

欢迎贡献！查看 [CONTRIBUTING.md](../CONTRIBUTING.md) 了解详情。

如有问题，欢迎在 GitHub Issues 中提问。
