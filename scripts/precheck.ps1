<#
================================================================================
 Weft 开源仓库 commit 前检查
================================================================================
 用途：在 git commit / push 前自检，拦截两类事故：
   1) 敏感/污染文件混入（密钥、嵌套 .git、构建产物、第三方合集、模型权重）
   2) 残留的 relik 品牌关键字（应替换为 weft）

 用法：
   pwsh -File scripts\precheck.ps1            # 检查，发现问题非零退出
   pwsh -File scripts\precheck.ps1 -Brand     # 仅检查 relik 品牌关键字

 退出码：0 = 通过；1 = 发现问题。可接入 git pre-commit hook。
================================================================================
#>
param([switch]$Brand)

$ErrorActionPreference = 'Stop'
$fail = 0

function Fail($msg) { Write-Host "[FAIL] $msg" -ForegroundColor Red; $script:fail = 1 }
function Pass($msg) { Write-Host "[ OK ] $msg" -ForegroundColor Green }

# 只检查 git 已跟踪的文件
$tracked = git ls-files

if (-not $Brand) {
    # ---- 1) 敏感/污染文件 -----------------------------------------------------
    $bad = $tracked | Select-String -Pattern @(
        '(^|/)\.git/',            # 嵌套版本库
        '/target/', '\.wasm$',    # Rust 构建产物
        '/build/', '\.dart_tool/',# Flutter 产物
        '__pycache__', '\.pyc$',  # Python 缓存
        '(^|/)lock\.toml$', 'generation-store\.toml$', # 包运行时态
        '(^|/)config\.toml$', 'config\.dev\.toml$', '\.env$', # 密钥实配
        '\.onnx$', '\.gguf$', '\.safetensors$', '\.pt$', '\.pth$', # 模型权重
        'openclaw-skills/',       # 第三方社区合集
        'companion-core/',        # 不开源的包(用户要求排除)
        '(^|/)CLAUDE\.md$', '(^|/)AGENT\.md$', '(^|/)AGENTS\.md$', # 私有代理指令(绝不公开)
        '\.claude/',              # 私有代理配置
        '\.migration/'            # 迁移工具
    ) -Raw
    if ($bad) { Fail "跟踪了不该提交的文件：`n$($bad -join "`n")" }
    else { Pass "无敏感/污染文件" }

    # ---- 2) 文件内容密钥扫描 --------------------------------------------------
    $secret = git grep -nI -E 'sk-[a-f0-9]{32}|gitea_token\s*=\s*["''][a-f0-9]|exa_api_key\s*=\s*["''][0-9a-f]' 2>$null
    if ($secret) { Fail "疑似真实密钥：`n$secret" }
    else { Pass "无硬编码密钥" }
}

# ---- 3) relik 品牌关键字 ------------------------------------------------------
# 排除：本检查脚本自身、CLAUDE.md（它们合法地提及 relik 作为说明）
# 项目已彻底改名 RELIK -> Weft：源码、capability id、数据目录、crate 名均为 weft，
# 因此任何 relik 残留都视为违规。
# 3a) 文件内容里的 relik
$hits = git grep -nIi 'relik' -- ':!scripts/precheck.ps1' ':!CLAUDE.md' 2>$null
if ($hits) {
    Fail "发现 $(@($hits).Count) 处 relik 关键字内容（应替换为 weft）。前 30 处："
    $hits | Select-Object -First 30 | ForEach-Object { Write-Host "    $_" -ForegroundColor DarkYellow }
} else {
    Pass "无 relik 关键字内容残留"
}

# 3b) 文件名/路径里的 relik（内容检查查不到，例如孤儿夹具目录名）
$pathHits = $tracked | Where-Object { $_ -match 'relik' -and $_ -ne 'CLAUDE.md' -and $_ -ne 'scripts/precheck.ps1' }
if ($pathHits) {
    Fail "发现 $(@($pathHits).Count) 个含 relik 的文件路径（应改名为 weft）："
    $pathHits | Select-Object -First 30 | ForEach-Object { Write-Host "    $_" -ForegroundColor DarkYellow }
} else {
    Pass "无 relik 文件名/路径残留"
}

if ($fail) { Write-Host "`n检查未通过，请修复后再 commit。" -ForegroundColor Red; exit 1 }
else { Write-Host "`n全部通过。" -ForegroundColor Green; exit 0 }
