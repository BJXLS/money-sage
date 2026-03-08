<div align="center">

# 💰 MoneySage

一款现代化的智能记账应用，采用 AI 辅助快速记账

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Tauri](https://img.shields.io/badge/Tauri-2.0-24C8DB.svg)](https://tauri.app)
[![Vue](https://img.shields.io/badge/Vue-3.5-4FC08D.svg)](https://vuejs.org)
[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://rust-lang.org)

[功能特性](#功能特性) • [快速开始](#快速开始) • [开发指南](#开发指南) • [技术栈](#技术栈) • [构建发布](#构建发布)

</div>

---

## 📖 简介

**MoneySage** 是一款功能强大的桌面记账应用，专为追求效率的用户设计。通过 AI 自然语言处理技术，让记账变得前所未有的简单。只需用自然语言描述你的收支，AI 会自动识别金额、分类和时间，大幅提升记账效率。

### ✨ 核心亮点

- 🤖 **AI 智能记账** - 自然语言输入，AI 自动解析金额、分类、日期
- 📊 **数据可视化** - 精美的图表展示收支趋势和支出分布
- 💼 **预算管理** - 设置预算目标，实时监控执行情况
- 🎨 **现代化 UI** - 深色主题界面，优雅流畅的用户体验
- 🔐 **隐私优先** - 本地数据存储，保护个人财务隐私
- ⚡ **高性能** - Rust 后端 + Vue 前端，极速运行
- 📦 **轻量级** - 安装包小巧，占用资源少

---

## 🎯 功能特性

### 📊 仪表盘概览

- **收支汇总** - 实时显示本月收入、支出、结余和交易笔数
- **趋势分析** - 可视化展示近 3/6/12 个月的收支变化趋势
- **支出分布** - 饼图显示各分类支出占比，一目了然
- **最近交易** - 快速查看最新的交易记录
- **预算执行** - 监控各分类预算使用情况和进度

### 🤖 AI 快速记账

```
输入: "今天中午在餐厅花了38元吃午饭"
AI识别: ✓ 金额:38元 ✓ 分类:餐饮 ✓ 类型:支出 ✓ 日期:今天
```

- 支持自然语言输入，无需填写复杂表单
- 自动识别金额、分类、日期、收支类型
- 支持批量输入，一次添加多条记录
- 识别结果可编辑确认，确保准确性

### 💰 交易记录管理

- **快速添加** - 一键"记一笔"，快速记录收支
- **智能筛选** - 按类型、分类、日期范围快速筛选
- **灵活编辑** - 支持修改和删除交易记录
- **分页浏览** - 高效管理大量历史数据
- **多维排序** - 按日期、金额等字段排序

### 🏷️ 分类管理

**预设分类体系**:

- 支出：🍽️ 餐饮 | 🚗 交通 | 🛍️ 购物 | 🎬 娱乐 | 🏠 住房 | ⚕️ 医疗 | 📚 教育 | 💰 其他
- 收入：💼 工资 | 💪 兼职 | 📈 投资 | 💰 其他

**自定义能力**:

- 添加自定义分类
- 自定义图标和颜色
- 管理和删除非系统分类

### 📈 预算管理

- **灵活设置** - 为各支出分类设置周/月/年度预算
- **实时监控** - 动态显示预算使用百分比和剩余额度
- **超支提醒** - 接近或超过预算时智能提醒
- **图表展示** - 可视化预算执行情况

### 📊 统计分析

- **趋势图表** - 查看历史收支变化趋势
- **分类统计** - 深入分析各分类支出分布
- **时间维度** - 灵活选择统计时间段
- **数据洞察** - 帮助发现消费模式，优化财务规划

### 📁 数据导入导出

- **CSV 导入** - 批量导入交易记录
- **CSV 导出** - 导出数据进行备份或深度分析
- **标准格式** - 兼容常见表格软件

---

## 🚀 快速开始

### 安装使用

#### Windows 用户

1. 前往 [Releases](../../releases) 页面
2. 下载最新版本的安装包：
   - `money-sage_x.x.x_x64-setup.exe` (NSIS 安装程序，推荐)
   - `money-sage_x.x.x_x64_zh-CN.msi` (MSI 安装包)
3. 双击运行安装程序
4. 启动 MoneySage 开始记账

#### 系统要求

- **操作系统**: Windows 10 / Windows 11
- **内存**: 至少 2GB RAM
- **磁盘空间**: 约 100MB
- **WebView2**: Windows 11 自带，Windows 10 会自动安装

---

## 💻 开发指南

### 环境准备

#### 1. 安装必需工具

```bash
# Node.js 18+
https://nodejs.org/

# Rust (通过 rustup)
https://rustup.rs/
```

#### 2. 克隆项目

```bash
git clone https://github.com/yourusername/money-sage.git
cd money-sage
```

#### 3. 安装依赖

```bash
npm install
```

### 开发运行

```bash
# 启动开发服务器（热重载）
npm run tauri:dev

# 或分开运行
npm run dev          # 启动前端开发服务器
npm run tauri        # 启动 Tauri 开发模式
```

### 代码结构

```
money-sage/
├── src/                      # Vue 前端代码
│   ├── components/           # 组件
│   │   ├── QuickBookingDialog.vue    # AI 快速记账
│   │   ├── LLMConfigDialog.vue       # AI 模型配置
│   │   ├── EditTransactionDialog.vue # 编辑交易
│   │   └── AddBudgetDialog.vue       # 添加预算
│   ├── views/                # 页面视图
│   │   ├── DashboardView.vue         # 仪表盘
│   │   ├── TransactionsView.vue      # 交易记录
│   │   ├── CategoriesView.vue        # 分类管理
│   │   ├── BudgetView.vue            # 预算设置
│   │   ├── StatisticsView.vue        # 统计分析
│   │   └── ImportExportView.vue      # 导入导出
│   ├── stores/               # Pinia 状态管理
│   ├── App.vue               # 主应用组件
│   └── main.ts               # 入口文件
├── src-tauri/                # Rust 后端代码
│   ├── src/
│   │   ├── ai/               # AI 功能模块
│   │   │   ├── agent/        # AI Agent 实现
│   │   │   │   ├── quick_note.rs    # 快速记账 AI
│   │   │   │   ├── analysis.rs      # 数据分析 AI
│   │   │   │   └── base.rs          # AI 基础接口
│   │   │   └── mod.rs
│   │   ├── database.rs       # 数据库操作
│   │   ├── models.rs         # 数据模型
│   │   ├── utils/            # 工具函数
│   │   ├── lib.rs            # 库入口
│   │   └── main.rs           # 主程序
│   ├── icons/                # 应用图标
│   ├── Cargo.toml            # Rust 依赖配置
│   └── tauri.conf.json       # Tauri 配置
├── package.json              # Node.js 依赖
└── vite.config.ts            # Vite 配置
```

### 测试

```bash
# 运行后端测试
cd src-tauri
cargo test

# 查看测试覆盖率
cargo test -- --nocapture
```

---

## 🛠️ 技术栈

### 前端

- **框架**: [Vue 3](https://vuejs.org/) - 渐进式 JavaScript 框架
- **构建工具**: [Vite](https://vitejs.dev/) - 下一代前端构建工具
- **UI 库**: [Element Plus](https://element-plus.org/) - Vue 3 组件库
- **图表**: [ECharts](https://echarts.apache.org/) + [vue-echarts](https://github.com/ecomfe/vue-echarts) - 数据可视化
- **状态管理**: [Pinia](https://pinia.vuejs.org/) - Vue 状态管理
- **工具库**: [VueUse](https://vueuse.org/) - Vue 组合式工具集
- **日期处理**: [Day.js](https://day.js.org/) - 轻量级日期库
- **CSV 处理**: [PapaParse](https://www.papaparse.com/) - CSV 解析器

### 后端

- **框架**: [Tauri 2.0](https://tauri.app/) - 构建桌面应用
- **语言**: [Rust](https://www.rust-lang.org/) - 高性能系统语言
- **数据库**: [SQLite](https://www.sqlite.org/) + [sqlx](https://github.com/launchbadge/sqlx) - 轻量级数据库
- **HTTP 客户端**: [reqwest](https://github.com/seanmonstar/reqwest) - AI API 调用
- **序列化**: [serde](https://serde.rs/) - Rust 序列化框架
- **异步运行时**: [tokio](https://tokio.rs/) - 异步运行时
- **错误处理**: [anyhow](https://github.com/dtolnay/anyhow) + [thiserror](https://github.com/dtolnay/thiserror)

### 开发工具

- **语言**: TypeScript + Rust
- **代码格式化**: Prettier + rustfmt
- **包管理**: npm + Cargo

---

## 📦 构建发布

### 快速构建

#### 方法一：使用自动化脚本（推荐）

```bash
# Windows
.\build-installer.bat

# 或 PowerShell
.\build-installer.ps1
```

#### 方法二：手动命令

```bash
# 安装依赖
npm install

# 构建前端并打包
npm run build:installer

# 或分步执行
npm run build        # 构建前端
npm run tauri:build  # 打包应用
```

### 构建环境要求

#### Windows 安装包构建

需要安装以下工具之一：

**MSI 安装包**（企业级）:

- [WiX Toolset 3.11](https://github.com/wixtoolset/wix3/releases)
- 添加到系统 PATH 环境变量

**NSIS 安装包**（推荐）:

- [NSIS 3.08+](https://nsis.sourceforge.io/Download)
- 添加到系统 PATH 环境变量

### 输出文件

构建成功后，安装包位于：

```
src-tauri/target/release/bundle/
├── msi/
│   └── money-sage_0.1.0_x64_zh-CN.msi
└── nsis/
    └── money-sage_0.1.0_x64-setup.exe
```

### 版本发布流程

1. **更新版本号** - 同步修改三个文件：

   ```bash
   package.json             # "version": "x.x.x"
   src-tauri/tauri.conf.json # "version": "x.x.x"
   src-tauri/Cargo.toml     # version = "x.x.x"
   ```

2. **构建安装包**:

   ```bash
   npm run build:installer
   ```

3. **测试安装包** - 在干净的 Windows 环境中测试

4. **创建 GitHub Release** - 上传安装包并附带更新说明

详细构建说明请查看 [BUILD.md](BUILD.md) 和 [打包说明.md](打包说明.md)

---

## 📚 使用文档

详细使用说明请查看 [使用说明.md](使用说明.md)

### 快速上手

1. **首次使用**

   - 启动应用，系统自动创建数据库
   - 默认分类已预设，可直接开始记账

2. **快速记账**

   - 点击右上角"快速记账"按钮
   - 用自然语言描述收支，如："今天午饭花了 35 元"
   - AI 自动识别并填充信息，确认后保存

3. **查看数据**

   - **仪表盘** - 查看整体财务状况
   - **记账** - 浏览和管理所有交易记录
   - **数据分析** - 查看收支趋势和分类统计

4. **设置预算**
   - 进入"预算设置"
   - 为各支出分类设置预算金额和周期
   - 系统自动跟踪预算执行情况

---

## 🎨 截图预览

<details>
<summary>点击展开查看应用截图</summary>

> 注：待添加实际应用截图

### 仪表盘

![仪表盘](docs/screenshots/dashboard.png)

### AI 快速记账

![快速记账](docs/screenshots/quick-booking.png)

### 交易记录

![交易记录](docs/screenshots/transactions.png)

### 数据分析

![数据分析](docs/screenshots/statistics.png)

</details>

---

## 🤝 贡献指南

欢迎贡献代码、报告问题或提出建议！

### 贡献方式

1. **Fork** 本仓库
2. **创建特性分支** (`git checkout -b feature/AmazingFeature`)
3. **提交更改** (`git commit -m 'Add some AmazingFeature'`)
4. **推送到分支** (`git push origin feature/AmazingFeature`)
5. **开启 Pull Request**

### 代码规范

- **前端**: 遵循 Vue 3 官方风格指南
- **后端**: 遵循 Rust 官方代码规范 (`cargo fmt`)
- **提交信息**: 使用清晰的提交信息描述更改内容

### 报告问题

如果发现 Bug 或有功能建议，请：

1. 搜索已有 [Issues](../../issues) 避免重复
2. 创建新 Issue，详细描述问题或建议
3. 提供复现步骤、截图或日志（如适用）

---

## 📄 许可证

本项目采用 [MIT License](LICENSE) 开源许可证。

这意味着你可以自由地：

- ✅ 商业使用
- ✅ 修改
- ✅ 分发
- ✅ 私人使用

但需要保留版权声明和许可证声明。

---

## 🙏 致谢

感谢以下优秀的开源项目：

- [Tauri](https://tauri.app/) - 让我们能够用 Web 技术构建高性能桌面应用
- [Vue.js](https://vuejs.org/) - 优雅的渐进式 JavaScript 框架
- [Element Plus](https://element-plus.org/) - 精美的 Vue 3 组件库
- [Rust](https://www.rust-lang.org/) - 高性能且内存安全的系统语言
- 所有其他依赖的开源项目

---

## 📮 联系方式

- **Issues**: [GitHub Issues](../../issues)
- **Discussions**: [GitHub Discussions](../../discussions)
- **Email**: your-email@example.com

---

## 🗺️ 路线图

### v0.2.0 (计划中)

- [ ] 支持多账户管理
- [ ] 添加周期性交易自动记录
- [ ] 数据云同步功能
- [ ] 移动端适配

### v0.3.0 (计划中)

- [ ] 支持多币种
- [ ] 高级财务报表
- [ ] 数据加密功能
- [ ] macOS / Linux 版本

### 长期目标

- [ ] 投资组合管理
- [ ] 账单提醒
- [ ] 家庭账本模式
- [ ] 更多 AI 智能分析功能

---

## ⭐ Star History

如果这个项目对你有帮助，请给它一个 Star ⭐️

[![Star History Chart](https://api.star-history.com/svg?repos=yourusername/money-sage&type=Date)](https://star-history.com/#yourusername/money-sage&Date)

---

<div align="center">

**用 ❤️ 和 ☕ 构建**

[⬆ 回到顶部](#-moneysage)

</div>
