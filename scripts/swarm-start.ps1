#requires -Version 7.0
Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# 1) 固定在 git root（避免 Serena 索引/项目根漂移）
$gitRoot = (git rev-parse --show-toplevel) 2>$null
if (-not $gitRoot) { throw "Not a git repo. Please run inside your project root." }
Set-Location $gitRoot

# 2) 确保 Claude Code Agent Teams 开关已写入（不替你编辑文件，只提示）
$settingsPath = Join-Path $env:USERPROFILE ".claude\settings.json"
if (-not (Test-Path $settingsPath)) {
  Write-Host "WARNING: Claude Code settings not found at $settingsPath"
  Write-Host 'Create it with: { "env": { "CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS": "1", "ENABLE_TOOL_SEARCH": "true" } }'
} else {
  $content = Get-Content $settingsPath -Raw
  if ($content -notmatch "CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS") {
    Write-Host "WARNING: Agent Teams flag not found in settings.json"
  } else {
    Write-Host "OK: Agent Teams flag found in settings.json"
  }
  if ($content -notmatch "ENABLE_TOOL_SEARCH") {
    Write-Host "WARNING: ENABLE_TOOL_SEARCH flag not found in settings.json"
  } else {
    Write-Host "OK: ENABLE_TOOL_SEARCH flag found in settings.json"
  }
}

# 3) 安装/注册 Serena MCP（如已存在会失败；失败也无所谓）
try {
  claude mcp add serena -- uvx --from git+https://github.com/oraios/serena serena start-mcp-server --context claude-code --project "$PWD"
  Write-Host "Serena MCP added for this project."
} catch {
  Write-Host "Serena MCP add skipped (maybe already added)."
}

# 4) 预索引一次（可重复跑，成本低；大项目第一次可能久一点）
try {
  serena project index
  Write-Host "Serena project index done."
} catch {
  Write-Host "Serena index skipped/failed. If first time, ensure Serena is installed and on PATH."
}

Write-Host ""
Write-Host "=== Swarm environment ready ==="
Write-Host "Now open Claude Code panel in Cursor and start your team workflow."
