#!/usr/bin/env bash
# install-sandcastle-claimer.sh —— sandcastle claim 循环 launchd 常驻任务的安装/卸载（#1182 方向 A）。
# 安装后 launchd KeepAlive 常驻；循环脚本 60s 轮询、无合资格票 sleep 不退出。
# 对齐 install-wayfinder-engine.sh 模式；宿主侧安装由编排者执行。
set -euo pipefail
PLIST_SRC=/Users/silencehan/Projects/NewChanlun/.sandcastle/launchd/com.newchanlun.sandcastle-claimer.plist
PLIST_DST="$HOME/Library/LaunchAgents/com.newchanlun.sandcastle-claimer.plist"

case "${1:-install}" in
  install)
    mkdir -p "$HOME/Library/LaunchAgents"
    cp "$PLIST_SRC" "$PLIST_DST"
    launchctl unload "$PLIST_DST" 2>/dev/null || true
    launchctl load "$PLIST_DST"
    echo "已安装并加载 com.newchanlun.sandcastle-claimer（常驻 claim 循环，--live）"
    ;;
  uninstall)
    launchctl unload "$PLIST_DST" 2>/dev/null || true
    rm -f "$PLIST_DST"
    echo "已卸载 com.newchanlun.sandcastle-claimer"
    ;;
  status)
    launchctl list | grep sandcastle-claimer || echo "com.newchanlun.sandcastle-claimer 未在运行"
    ;;
  *)
    echo "用法: $0 [install|uninstall|status]" >&2
    exit 2
    ;;
esac
