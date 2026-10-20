#!/usr/bin/env bash
# install-health-monitor.sh —— health-monitor launchd 定时任务的安装/卸载（#1547）。
# 对齐 install-sandcastle-claimer.sh 模式；宿主侧安装由编排者执行。
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"
PLIST_SRC="$SCRIPT_DIR/launchd/com.newchanlun.health-monitor.plist"
PLIST_DST="$HOME/Library/LaunchAgents/com.newchanlun.health-monitor.plist"

case "${1:-install}" in
  install)
    mkdir -p "$HOME/Library/LaunchAgents"
    cp "$PLIST_SRC" "$PLIST_DST"
    launchctl unload "$PLIST_DST" 2>/dev/null || true
    launchctl load "$PLIST_DST"
    echo "已安装并加载 com.newchanlun.health-monitor（每 300s，--heal）"
    ;;
  uninstall)
    launchctl unload "$PLIST_DST" 2>/dev/null || true
    rm -f "$PLIST_DST"
    echo "已卸载 com.newchanlun.health-monitor"
    ;;
  status)
    launchctl list | grep health-monitor || echo "com.newchanlun.health-monitor 未加载"
    ;;
  *)
    echo "用法: $0 [install|uninstall|status]" >&2
    exit 2
    ;;
esac
