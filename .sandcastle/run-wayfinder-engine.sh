#!/usr/bin/env bash
# run-wayfinder-engine.sh —— wayfinder_engine 控制面常驻壳（#1084 追加）。
# launchd KeepAlive 调用本脚本；前台常驻（launchd 不需要 nohup）。
# 引擎内部自带 4 分钟轮询 for(;;)，无 runnable 票 sleep 后重查、不退出；
# 单次 GitHub 网络失败经重试/退避后 fail-loud，进程不因单次失败停摆。
#
# 用法：
#   bash .sandcastle/run-wayfinder-engine.sh --live       # 真实执行（服务入口）
#   bash .sandcastle/run-wayfinder-engine.sh --once --dry-run  # 单轮演练
set -euo pipefail
REPO=/Users/silencehan/Projects/NewChanlun
cd "$REPO"
mkdir -p .sandcastle/logs
export NO_COLOR=1 CLICOLOR=0 FORCE_COLOR=0 CLICOLOR_FORCE=0
export GH_REPO="${GH_REPO:-xy7365527-lang/NewChanlun}"
exec npx tsx scripts/wayfinder_engine.mts "$@"
