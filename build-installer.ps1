# MoneyNote Windows 安装包构建脚本
# PowerShell 版本

Write-Host "====================================" -ForegroundColor Cyan
Write-Host "MoneyNote Windows 安装包构建脚本" -ForegroundColor Cyan
Write-Host "====================================" -ForegroundColor Cyan
Write-Host

# 检查执行策略
$executionPolicy = Get-ExecutionPolicy
if ($executionPolicy -eq "Restricted") {
    Write-Host "[警告] PowerShell 执行策略被限制" -ForegroundColor Yellow
    Write-Host "请运行: Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser" -ForegroundColor Yellow
    Write-Host
}

# 检查必要工具
function Test-Command($command) {
    try {
        Get-Command $command -ErrorAction Stop | Out-Null
        return $true
    } catch {
        return $false
    }
}

$hasErrors = $false

# 检查 Node.js
if (-not (Test-Command "node")) {
    Write-Host "[错误] 未找到 Node.js，请先安装 Node.js" -ForegroundColor Red
    Write-Host "下载地址: https://nodejs.org/" -ForegroundColor Yellow
    $hasErrors = $true
}

# 检查 Rust
if (-not (Test-Command "cargo")) {
    Write-Host "[错误] 未找到 Rust，请先安装 Rust" -ForegroundColor Red
    Write-Host "下载地址: https://rustup.rs/" -ForegroundColor Yellow
    $hasErrors = $true
}

# 检查 WiX
if (-not (Test-Command "candle")) {
    Write-Host "[警告] 未找到 WiX Toolset，将无法构建 MSI 安装包" -ForegroundColor Yellow
    Write-Host "下载地址: https://github.com/wixtoolset/wix3/releases" -ForegroundColor Yellow
}

# 检查 NSIS
if (-not (Test-Command "makensis")) {
    Write-Host "[警告] 未找到 NSIS，将无法构建 NSIS 安装包" -ForegroundColor Yellow
    Write-Host "下载地址: https://nsis.sourceforge.io/Download" -ForegroundColor Yellow
}

if ($hasErrors) {
    Write-Host
    Write-Host "请安装缺少的工具后重新运行脚本" -ForegroundColor Red
    Read-Host "按 Enter 键退出"
    exit 1
}

Write-Host
Write-Host "[信息] 开始构建过程..." -ForegroundColor Green
Write-Host

try {
    # 安装依赖
    Write-Host "[1/3] 安装项目依赖..." -ForegroundColor Cyan
    npm install
    if ($LASTEXITCODE -ne 0) {
        throw "依赖安装失败"
    }

    # 构建前端
    Write-Host
    Write-Host "[2/3] 构建前端代码..." -ForegroundColor Cyan
    npm run build
    if ($LASTEXITCODE -ne 0) {
        throw "前端构建失败"
    }

    # 构建安装包
    Write-Host
    Write-Host "[3/3] 构建 Windows 安装包..." -ForegroundColor Cyan
    npm run tauri:build
    if ($LASTEXITCODE -ne 0) {
        throw "安装包构建失败"
    }

    Write-Host
    Write-Host "====================================" -ForegroundColor Green
    Write-Host "构建完成！" -ForegroundColor Green
    Write-Host "====================================" -ForegroundColor Green
    Write-Host
    Write-Host "安装包位置：" -ForegroundColor Cyan
    
    $bundlePath = "src-tauri\target\release\bundle"
    if (Test-Path $bundlePath) {
        $msiPath = Join-Path $bundlePath "msi"
        $nsisPath = Join-Path $bundlePath "nsis"
        
        if (Test-Path $msiPath) {
            $msiFiles = Get-ChildItem $msiPath -Filter "*.msi"
            foreach ($file in $msiFiles) {
                Write-Host "  MSI:  $($file.FullName)" -ForegroundColor Green
            }
        }
        
        if (Test-Path $nsisPath) {
            $nsisFiles = Get-ChildItem $nsisPath -Filter "*setup.exe"
            foreach ($file in $nsisFiles) {
                Write-Host "  NSIS: $($file.FullName)" -ForegroundColor Green
            }
        }
        
        Write-Host
        Write-Host "正在打开输出目录..." -ForegroundColor Cyan
        Invoke-Item $bundlePath
    }

} catch {
    Write-Host
    Write-Host "[错误] $($_.Exception.Message)" -ForegroundColor Red
    Write-Host "构建失败！" -ForegroundColor Red
    Read-Host "按 Enter 键退出"
    exit 1
}

Write-Host
Read-Host "按 Enter 键退出"
