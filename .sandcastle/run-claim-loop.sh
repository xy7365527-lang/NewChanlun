#!/usr/bin/env bash
# run-claim-loop.sh —— sandcastle 自动拾取 claim 循环常驻壳（#1182 方向 A）。
# launchd KeepAlive 调用本脚本；前台常驻（launchd 不需要 nohup）。
# 循环本体 .sandcastle/claim-loop.mts 自带 60s 轮询 for(;;)，无合资格票 sleep 后重查、不退出；
# 单次 gh 网络失败经 runOnce 的 try/catch 兜底，进程不因单次失败停摆。
#
# 用法：
#   bash .sandcastle/run-claim-loop.sh --live        # 真实执行（服务入口）
#   bash .sandcastle/run-claim-loop.sh --once --dry-run  # 单轮演练
set -euo pipefail
if [ -f "/Users/silencehan/Projects/NewChanlun/.sandcastle/logs/GH_TOKEN_ROTATION_REQUIRED" ]; then
  echo "[claim-loop] 拒绝启动：GitHub token 尚未轮换" >&2
  exit 78
fi
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"
REPO="${SANDCASTLE_HOST_REPO:-$(cd "$SCRIPT_DIR/.." && pwd)}"
cd "$REPO"
mkdir -p .sandcastle/logs
export NO_COLOR=1 CLICOLOR=0 FORCE_COLOR=0 CLICOLOR_FORCE=0
export GH_REPO="${GH_REPO:-xy7365527-lang/NewChanlun}"
exec npx tsx .sandcastle/claim-loop.mts "$@"
