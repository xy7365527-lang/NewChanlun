#!/usr/bin/env bash
# install-wayfinder-engine.sh —— wayfinder_engine 控制面 launchd 常驻任务的安装/卸载（#1084 追加）。
# 安装后 launchd KeepAlive 常驻；引擎内部 4 分钟轮询、无 runnable 票 sleep 不退出。
# 对齐 install-sandcastle-claimer.sh：plist 模板按本脚本位置取；模板内路径指 sandcastle-host（#1588）。
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"
PLIST_SRC="$SCRIPT_DIR/launchd/com.newchanlun.wayfinder-engine.plist"
PLIST_DST="$HOME/Library/LaunchAgents/com.newchanlun.wayfinder-engine.plist"

case "${1:-install}" in
  install)
    mkdir -p "$HOME/Library/LaunchAgents"
    cp "$PLIST_SRC" "$PLIST_DST"
    launchctl unload "$PLIST_DST" 2>/dev/null || true
    launchctl load "$PLIST_DST"
    echo "已安装并加载 com.newchanlun.wayfinder-engine（常驻控制面，--live）"
    ;;
  uninstall)
    launchctl unload "$PLIST_DST" 2>/dev/null || true
    rm -f "$PLIST_DST"
    echo "已卸载 com.newchanlun.wayfinder-engine"
    ;;
  status)
    launchctl list | grep wayfinder-engine || echo "com.newchanlun.wayfinder-engine 未在运行"
    ;;
  *)
    echo "用法: $0 [install|uninstall|status]" >&2
    exit 2
    ;;
esac
