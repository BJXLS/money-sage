# 贡献指南

感谢你考虑为 MoneySage 做出贡献！

## 📋 目录

- [行为准则](#行为准则)
- [如何贡献](#如何贡献)
- [开发流程](#开发流程)
- [代码规范](#代码规范)
- [提交规范](#提交规范)
- [问题反馈](#问题反馈)

## 行为准则

我们致力于为每个人提供友好、安全和热情的环境。参与本项目即表示你同意遵守以下准则：

- 使用友好和包容的语言
- 尊重不同的观点和经验
- 优雅地接受建设性批评
- 关注对社区最有利的事情
- 对其他社区成员表示同理心

## 如何贡献

### 报告 Bug

如果你发现了一个 Bug，请：

1. **检查已有 Issues** - 确认该问题是否已被报告
2. **创建详细的 Issue**，包含：
   - 清晰的标题
   - 详细的问题描述
   - 复现步骤
   - 预期行为 vs 实际行为
   - 系统环境（操作系统、版本等）
   - 截图或日志（如果适用）

### 建议新功能

如果你有新功能的想法：

1. **检查路线图和已有 Issues** - 避免重复建议
2. **创建 Feature Request Issue**，包含：
   - 功能的清晰描述
   - 为什么需要这个功能
   - 可能的实现方式
   - 示例或原型（如果有）

### 提交代码

1. **Fork 仓库**
2. **创建特性分支**

   ```bash
   git checkout -b feature/your-feature-name
   # 或
   git checkout -b fix/your-bug-fix
   ```

3. **进行开发**

   - 遵循代码规范
   - 编写清晰的代码注释
   - 添加必要的测试
   - 确保所有测试通过

4. **提交更改**

   ```bash
   git add .
   git commit -m "feat: add your feature description"
   ```

5. **推送到 Fork**

   ```bash
   git push origin feature/your-feature-name
   ```

6. **创建 Pull Request**
   - 清晰描述你的更改
   - 关联相关的 Issue
   - 等待代码审查

## 开发流程

### 环境搭建

```bash
# 1. 克隆你的 Fork
git clone https://github.com/your-username/money-sage.git
cd money-sage

# 2. 添加上游仓库
git remote add upstream https://github.com/original-owner/money-sage.git

# 3. 安装依赖
npm install

# 4. 启动开发服务器
npm run tauri:dev
```

### 保持同步

```bash
# 获取上游更改
git fetch upstream

# 合并到你的分支
git checkout main
git merge upstream/main
```

### 运行测试

```bash
# 前端测试
npm run test

# 后端测试
cd src-tauri
cargo test

# 运行所有测试
cargo test -- --nocapture
```

### 代码检查

```bash
# 前端代码检查
npm run lint

# Rust 代码格式化
cd src-tauri
cargo fmt

# Rust 代码检查
cargo clippy
```

## 代码规范

### TypeScript/Vue 规范

- 使用 **2 空格**缩进
- 使用 **单引号**表示字符串
- 组件名使用 **PascalCase**
- 文件名使用 **PascalCase** (组件) 或 **camelCase** (工具)
- 遵循 [Vue 官方风格指南](https://vuejs.org/style-guide/)
- 使用 TypeScript 类型注解

**示例**:

```typescript
// Good
export interface Transaction {
  id: string
  amount: number
  category: string
}

// Bad
export interface transaction {
  id: string
  amount: number
  category: string
}
```

### Rust 规范

- 使用 **4 空格**缩进
- 使用 `cargo fmt` 自动格式化
- 遵循 [Rust 官方风格指南](https://doc.rust-lang.org/1.0.0/style/)
- 为公共 API 编写文档注释
- 使用 `cargo clippy` 检查代码质量

**示例**:

```rust
// Good
/// 添加一笔交易记录
pub async fn add_transaction(transaction: Transaction) -> Result<()> {
    // 实现...
}

// Bad
pub async fn add_transaction(transaction:Transaction)->Result<()>{
    //实现...
}
```

## 提交规范

我们使用 [Conventional Commits](https://www.conventionalcommits.org/) 规范：

### 提交格式

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Type 类型

- `feat`: 新功能
- `fix`: Bug 修复
- `docs`: 文档更新
- `style`: 代码格式调整（不影响功能）
- `refactor`: 代码重构
- `perf`: 性能优化
- `test`: 测试相关
- `chore`: 构建过程或辅助工具的变动

### 示例

```bash
# 新功能
git commit -m "feat(ai): add natural language parsing for transactions"

# Bug 修复
git commit -m "fix(database): resolve transaction duplicate issue"

# 文档
git commit -m "docs(readme): update installation instructions"

# 重构
git commit -m "refactor(ui): improve dashboard layout structure"
```

## Pull Request 流程

1. **确保通过所有检查**

   - 代码格式化
   - 所有测试通过
   - 无 linter 错误

2. **填写 PR 模板**

   - 描述更改内容
   - 说明为什么需要这些更改
   - 关联相关 Issue

3. **等待审查**
   - 维护者会审查你的代码
   - 根据反馈进行调整
   - 通过审查后会被合并

## 问题反馈

### Bug 报告模板

```markdown
**描述问题**
清晰简洁地描述问题

**复现步骤**

1. 打开 '...'
2. 点击 '...'
3. 滚动到 '...'
4. 看到错误

**预期行为**
描述你期望发生什么

**实际行为**
描述实际发生了什么

**截图**
如果适用，添加截图帮助解释问题

**环境信息**

- OS: [例如 Windows 11]
- 版本: [例如 0.1.0]
- 浏览器: [如果适用]

**额外信息**
添加任何其他相关信息
```

### Feature Request 模板

```markdown
**功能描述**
清晰简洁地描述你想要的功能

**问题场景**
描述这个功能解决什么问题
例如：我总是很沮丧当 [...]

**期望的解决方案**
描述你希望如何实现

**替代方案**
描述你考虑过的其他方案

**额外信息**
添加任何其他相关信息或截图
```

## 文档贡献

文档同样重要！如果你发现文档有误或需要改进：

- 直接编辑 Markdown 文件
- 遵循相同的 PR 流程
- 确保语言清晰、准确

## 社区

- **GitHub Issues**: 用于 Bug 报告和功能请求
- **GitHub Discussions**: 用于一般讨论和问题
- **Pull Requests**: 用于代码贡献

## 获得帮助

如果你在贡献过程中遇到问题：

1. 查看现有文档和 Issues
2. 在 Discussions 中提问
3. 联系维护者

## 感谢

感谢你的贡献！每一个贡献都让 MoneySage 变得更好。

---

**记住**: 没有贡献是太小的。无论是修复拼写错误、改进文档，还是添加新功能，我们都非常感激！
