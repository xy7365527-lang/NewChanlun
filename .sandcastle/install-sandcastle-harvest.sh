#!/usr/bin/env bash
# install-sandcastle-harvest.sh —— sandcastle 收割 launchd 定时任务的安装/卸载（#1660）。
# 安装后 launchd 每 15 分钟跑一轮 harvest.sh：推分支 + 开 PR + 票面登记；不合入、不关票。
# 对齐 install-wayfinder-engine.sh 模式；宿主侧安装由编排者执行。
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"
PLIST_SRC="$SCRIPT_DIR/launchd/com.newchanlun.sandcastle-harvest.plist"
PLIST_DST="$HOME/Library/LaunchAgents/com.newchanlun.sandcastle-harvest.plist"

case "${1:-install}" in
  install)
    if [ -f "/Users/silencehan/Projects/NewChanlun/.sandcastle/logs/GH_TOKEN_ROTATION_REQUIRED" ]; then
      echo "拒绝安装：GitHub token 尚未轮换（见 /Users/silencehan/Projects/NewChanlun/.sandcastle/logs/GH_TOKEN_ROTATION_REQUIRED）" >&2
      exit 78
    fi
    mkdir -p "$HOME/Library/LaunchAgents"
    cp "$PLIST_SRC" "$PLIST_DST"
    launchctl unload "$PLIST_DST" 2>/dev/null || true
    launchctl load "$PLIST_DST"
    echo "已安装并加载 com.newchanlun.sandcastle-harvest（每 15 分钟一轮收割：推分支+开 PR，不合入不关票）"
    ;;
  uninstall)
    launchctl unload "$PLIST_DST" 2>/dev/null || true
    rm -f "$PLIST_DST"
    echo "已卸载 com.newchanlun.sandcastle-harvest"
    ;;
  status)
    launchctl list | grep sandcastle-harvest || echo "com.newchanlun.sandcastle-harvest 未在运行"
    ;;
  *)
    echo "用法: $0 [install|uninstall|status]" >&2
    exit 2
    ;;
esac
