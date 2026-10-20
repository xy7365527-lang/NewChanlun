#!/usr/bin/env bash
# run-wayfinder-engine.sh —— wayfinder_engine 控制面常驻壳（#1084 追加）。
# launchd KeepAlive 调用本脚本；前台常驻（launchd 不需要 nohup）。
# 引擎内部自带 4 分钟轮询 for(;;)，无 runnable 票 sleep 后重查、不退出；
# 单次 GitHub 网络失败经重试/退避后 fail-loud，进程不因单次失败停摆。
#
# 用法：
#   bash .sandcastle/run-wayfinder-engine.sh --live       # 真实执行（服务入口）
#   bash .sandcastle/run-wayfinder-engine.sh --once --dry-run  # 单轮演练
#
# 宿主仓库 = 本脚本所在仓（.sandcastle/ 的上一级），可用 SANDCASTLE_HOST_REPO 覆盖（run-claim-loop.sh 同款）。
# #1588：原硬编码用户工作区 /Users/silencehan/Projects/NewChanlun，launchd 跑的是用户检出分支上的旧引擎，
# 合入 main 的修复（#1578）从未生效；常驻服务应从 sandcastle-host（跟 main）运行。
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"
REPO="${SANDCASTLE_HOST_REPO:-$(cd "$SCRIPT_DIR/.." && pwd)}"
cd "$REPO"
mkdir -p .sandcastle/logs
# spec 起草子代理调 claude CLI（~/.local/bin），launchd 的 PATH 不含它。
export PATH="$HOME/.local/bin:$PATH"
export NO_COLOR=1 CLICOLOR=0 FORCE_COLOR=0 CLICOLOR_FORCE=0
export GH_REPO="${GH_REPO:-xy7365527-lang/NewChanlun}"
exec npx tsx scripts/wayfinder_engine.mts "$@"
