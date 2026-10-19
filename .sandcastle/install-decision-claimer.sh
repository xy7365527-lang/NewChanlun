#!/usr/bin/env bash
# install-decision-claimer.sh —— 裁决面 launchd 常驻任务的安装/卸载（#1547）。
# 安装后 launchd KeepAlive 常驻；脚本内部 10 分钟轮询，以 --live 写 GitHub。
# 安装前先手动跑一次 dry-run：node --import tsx .sandcastle/decision-claimer.mts --ticket <号>
set -euo pipefail
PLIST_SRC=/Users/silencehan/Projects/NewChanlun/.sandcastle/launchd/com.newchanlun.decision-claimer.plist
PLIST_DST="$HOME/Library/LaunchAgents/com.newchanlun.decision-claimer.plist"

case "${1:-install}" in
  install)
    mkdir -p "$HOME/Library/LaunchAgents"
    cp "$PLIST_SRC" "$PLIST_DST"
    launchctl unload "$PLIST_DST" 2>/dev/null || true
    launchctl load "$PLIST_DST"
    echo "已安装并加载 com.newchanlun.decision-claimer（常驻裁决面，--live）"
    ;;
  uninstall)
    launchctl unload "$PLIST_DST" 2>/dev/null || true
    rm -f "$PLIST_DST"
    echo "已卸载 com.newchanlun.decision-claimer"
    ;;
  status)
    launchctl list | grep decision-claimer || echo "com.newchanlun.decision-claimer 未在运行"
    ;;
  *)
    echo "用法: $0 [install|uninstall|status]" >&2
    exit 2
    ;;
esac
