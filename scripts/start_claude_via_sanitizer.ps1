param(
  [string]$Upstream = "",
  [int]$Port = 8787,
  [string]$ListenHost = "127.0.0.1"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Resolve-Upstream {
  param([string]$Provided)
  if ($Provided) { return $Provided }

  $settingsPath = Join-Path $env:USERPROFILE ".claude\settings.json"
  if (Test-Path $settingsPath) {
    try {
      $settings = Get-Content $settingsPath -Raw | ConvertFrom-Json
      $candidate = $settings.env.ANTHROPIC_BASE_URL
      if ($candidate -and $candidate -notmatch "127\.0\.0\.1|localhost") {
        return $candidate
      }
    } catch {
      # Fall through to default.
    }
  }
  return "https://cc.zhihuiapi.top"
}

$repoRoot = Split-Path -Parent $PSScriptRoot
$proxyScript = Join-Path $repoRoot "scripts\anthropic_thinking_sanitizer_proxy.py"
if (-not (Test-Path $proxyScript)) {
  throw "Missing proxy script: $proxyScript"
}

$resolvedUpstream = Resolve-Upstream -Provided $Upstream
$localBase = "http://$ListenHost`:$Port"

Write-Host "Starting thinking-sanitizer proxy..."
Write-Host "  Upstream: $resolvedUpstream"
Write-Host "  Local:    $localBase"

$proxyProc = Start-Process `
  -FilePath "python" `
  -ArgumentList @($proxyScript, "--upstream", $resolvedUpstream, "--host", $ListenHost, "--port", "$Port") `
  -PassThru `
  -WindowStyle Hidden

Start-Sleep -Seconds 2

if ($proxyProc.HasExited) {
  throw "Failed to start local proxy process."
}

try {
  $env:ANTHROPIC_BASE_URL = $localBase
  Write-Host "Launching claude with ANTHROPIC_BASE_URL=$localBase"
  claude
} finally {
  if ($proxyProc -and -not $proxyProc.HasExited) {
    Write-Host "Stopping local proxy..."
    Stop-Process -Id $proxyProc.Id -Force
  }
}
