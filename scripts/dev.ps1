#requires -Version 5.1
<#
.SYNOPSIS
    一键启动开发环境：Bottle (8765) + FastAPI (8766) + Vite dev (5173)
.DESCRIPTION
    并行启动三个进程，Ctrl+C 后全部停止。
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# 项目根目录
$ProjectRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
if (-not $ProjectRoot) {
    $ProjectRoot = (git rev-parse --show-toplevel 2>$null)
}
if (-not $ProjectRoot) {
    $ProjectRoot = $PSScriptRoot | Split-Path -Parent
}

Write-Host "=== NewChan Dev Environment ===" -ForegroundColor Cyan
Write-Host "Project root: $ProjectRoot"
Write-Host ""

# ── 1. Bottle server (port 8765) ──
Write-Host "[1/3] Starting Bottle server on :8765 ..." -ForegroundColor Green
$bottleJob = Start-Job -ScriptBlock {
    param($root)
    Set-Location $root
    & python -m newchan.cli serve --port 8765 --no-browser 2>&1
} -ArgumentList $ProjectRoot

# ── 2. FastAPI gateway (port 8766) ──
Write-Host "[2/3] Starting FastAPI gateway on :8766 ..." -ForegroundColor Green
$fastapiJob = Start-Job -ScriptBlock {
    param($root)
    Set-Location $root
    & python -m uvicorn newchan.gateway:app --host 127.0.0.1 --port 8766 --reload 2>&1
} -ArgumentList $ProjectRoot

# ── 3. Vite dev server (port 5173) ──
Write-Host "[3/3] Starting Vite dev server on :5173 ..." -ForegroundColor Green
$viteJob = Start-Job -ScriptBlock {
    param($root)
    Set-Location (Join-Path $root "frontend")
    & npm run dev 2>&1
} -ArgumentList $ProjectRoot

Write-Host ""
Write-Host "All services starting:" -ForegroundColor Cyan
Write-Host "  Bottle   -> http://localhost:8765"
Write-Host "  FastAPI  -> http://localhost:8766"
Write-Host "  Vite     -> http://localhost:5173"
Write-Host ""
Write-Host "Press Ctrl+C to stop all services." -ForegroundColor Yellow
Write-Host ""

# 持续输出日志，直到 Ctrl+C
$jobs = @($bottleJob, $fastapiJob, $viteJob)
$names = @("Bottle ", "FastAPI", "Vite   ")
$colors = @("DarkYellow", "DarkCyan", "DarkGreen")

try {
    while ($true) {
        for ($i = 0; $i -lt $jobs.Count; $i++) {
            $output = Receive-Job -Job $jobs[$i] -ErrorAction SilentlyContinue
            if ($output) {
                foreach ($line in $output) {
                    Write-Host "[$($names[$i])] $line" -ForegroundColor $colors[$i]
                }
            }
        }
        Start-Sleep -Milliseconds 500
    }
}
finally {
    Write-Host ""
    Write-Host "Stopping all services..." -ForegroundColor Red
    $jobs | ForEach-Object {
        Stop-Job -Job $_ -ErrorAction SilentlyContinue
        Remove-Job -Job $_ -Force -ErrorAction SilentlyContinue
    }
    Write-Host "All services stopped." -ForegroundColor Red
}
