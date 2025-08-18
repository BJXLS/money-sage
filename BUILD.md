# MoneyNote Windows 安装包构建指南

## 前置要求

### 1. 安装 Rust
```bash
# 访问 https://rustup.rs/ 下载并安装 Rust
# 或使用以下命令：
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. 安装 Node.js
```bash
# 下载并安装 Node.js 18+ 版本
# https://nodejs.org/
```

### 3. 安装 Windows 构建工具

#### 对于 MSI 安装包：
- 安装 **WiX Toolset 3.11** (不是 4.x 版本)
- 下载地址：https://github.com/wixtoolset/wix3/releases
- 确保 WiX 的 `bin` 目录添加到系统 PATH

#### 对于 NSIS 安装包：
- 安装 **NSIS 3.08+**
- 下载地址：https://nsis.sourceforge.io/Download
- 确保 NSIS 安装目录添加到系统 PATH

### 4. 安装项目依赖
```bash
npm install
```

## 构建步骤

### 1. 开发模式运行
```bash
npm run tauri:dev
```

### 2. 构建生产版本
```bash
# 构建前端 + 打包安装程序
npm run build:installer

# 或分步执行：
npm run build              # 构建前端
npm run tauri:build        # 打包 Tauri 应用
```

### 3. 调试版本构建
```bash
npm run build:installer:debug
```

## 输出文件

构建完成后，安装包将位于：
```
src-tauri/target/release/bundle/
├── msi/                    # MSI 安装包
│   └── money-sage_0.1.0_x64_zh-CN.msi
└── nsis/                   # NSIS 安装包
    └── money-sage_0.1.0_x64-setup.exe
```

## 安装包类型

### MSI 安装包
- **优点**: Windows 原生格式，支持企业部署策略
- **特点**: 可通过组策略分发，支持静默安装
- **文件**: `money-sage_0.1.0_x64_zh-CN.msi`

### NSIS 安装包
- **优点**: 灵活的安装向导，文件更小
- **特点**: 更好的用户体验，支持多语言
- **文件**: `money-sage_0.1.0_x64-setup.exe`

## 版本更新

修改版本号需要同时更新：
1. `package.json` 中的 `version`
2. `src-tauri/tauri.conf.json` 中的 `version`
3. `src-tauri/Cargo.toml` 中的 `version`

## 代码签名（可选）

为了避免 Windows 安全警告，建议对安装包进行代码签名：

1. 获取代码签名证书
2. 在 `tauri.conf.json` 中配置：
```json
{
  "bundle": {
    "windows": {
      "certificateThumbprint": "YOUR_CERTIFICATE_THUMBPRINT",
      "timestampUrl": "http://timestamp.sectigo.com"
    }
  }
}
```

## 故障排除

### 常见问题

1. **WiX 未找到**
   ```
   Error: `wix` not found in PATH
   ```
   - 确保安装了 WiX Toolset 3.11
   - 检查 PATH 环境变量

2. **NSIS 未找到**
   ```
   Error: `makensis` not found in PATH
   ```
   - 确保安装了 NSIS
   - 检查 PATH 环境变量

3. **构建失败**
   ```
   Error: failed to bundle project
   ```
   - 检查所有依赖是否正确安装
   - 确保项目编译无错误
   - 查看详细错误日志

### 清理构建缓存
```bash
# 清理 Node.js 缓存
npm run build -- --clean

# 清理 Rust 缓存
cd src-tauri
cargo clean
```

## 分发建议

1. **测试安装包**: 在干净的 Windows 环境中测试安装
2. **检查依赖**: 确保目标机器有 WebView2 运行时
3. **提供选择**: 同时提供 MSI 和 NSIS 版本
4. **版本说明**: 提供详细的更新日志

## 自动化构建

可以使用 GitHub Actions 等 CI/CD 工具自动构建：

```yaml
# .github/workflows/build.yml
name: Build Windows Installer
on:
  push:
    tags:
      - 'v*'
jobs:
  build:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 18
      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
      - name: Install dependencies
        run: npm ci
      - name: Build installer
        run: npm run build:installer
      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: windows-installer
          path: src-tauri/target/release/bundle/
```
