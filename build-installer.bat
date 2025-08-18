@echo off
echo ====================================
echo MoneyNote Windows 安装包构建脚本
echo ====================================
echo.

:: 检查 Node.js
where node >nul 2>nul
if %errorlevel% neq 0 (
    echo [错误] 未找到 Node.js，请先安装 Node.js
    pause
    exit /b 1
)

:: 检查 Rust
where cargo >nul 2>nul
if %errorlevel% neq 0 (
    echo [错误] 未找到 Rust，请先安装 Rust
    pause
    exit /b 1
)

:: 检查 WiX (MSI 构建需要)
where candle >nul 2>nul
if %errorlevel% neq 0 (
    echo [警告] 未找到 WiX Toolset，将无法构建 MSI 安装包
    echo 请安装 WiX Toolset 3.11: https://github.com/wixtoolset/wix3/releases
)

:: 检查 NSIS (NSIS 构建需要)
where makensis >nul 2>nul
if %errorlevel% neq 0 (
    echo [警告] 未找到 NSIS，将无法构建 NSIS 安装包
    echo 请安装 NSIS: https://nsis.sourceforge.io/Download
)

echo.
echo [信息] 开始构建过程...
echo.

:: 安装依赖
echo [1/3] 安装项目依赖...
call npm install
if %errorlevel% neq 0 (
    echo [错误] 依赖安装失败
    pause
    exit /b 1
)

:: 构建前端
echo.
echo [2/3] 构建前端代码...
call npm run build
if %errorlevel% neq 0 (
    echo [错误] 前端构建失败
    pause
    exit /b 1
)

:: 构建安装包
echo.
echo [3/3] 构建 Windows 安装包...
call npm run tauri:build
if %errorlevel% neq 0 (
    echo [错误] 安装包构建失败
    pause
    exit /b 1
)

echo.
echo ====================================
echo 构建完成！
echo ====================================
echo.
echo 安装包位置：
echo   MSI:  src-tauri\target\release\bundle\msi\
echo   NSIS: src-tauri\target\release\bundle\nsis\
echo.

:: 打开输出目录
if exist "src-tauri\target\release\bundle\" (
    echo 正在打开输出目录...
    start "" "src-tauri\target\release\bundle\"
)

echo 按任意键退出...
pause >nul
