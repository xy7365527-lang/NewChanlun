#!/usr/bin/env bash
# install-arch-hotzone.sh —— 架构热区监控 launchd 常驻任务的安装/卸载（#1197 追加）。
# 安装后 launchd KeepAlive 常驻；监控脚本内部 24h 一轮（ARCH_HOTZONE_INTERVAL_SECONDS 可调）、
# 无命中 sleep 不退出。与 install-wayfinder-engine.sh 同款 launchd 安装模式。
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"
PLIST_SRC="$SCRIPT_DIR/launchd/com.newchanlun.arch-hotzone.plist"
PLIST_DST="$HOME/Library/LaunchAgents/com.newchanlun.arch-hotzone.plist"

case "${1:-install}" in
  install)
    mkdir -p "$HOME/Library/LaunchAgents"
    cp "$PLIST_SRC" "$PLIST_DST"
    launchctl unload "$PLIST_DST" 2>/dev/null || true
    launchctl load "$PLIST_DST"
    echo "已安装并加载 com.newchanlun.arch-hotzone（常驻热区监控，--live，24h 一轮）"
    ;;
  uninstall)
    launchctl unload "$PLIST_DST" 2>/dev/null || true
    rm -f "$PLIST_DST"
    echo "已卸载 com.newchanlun.arch-hotzone"
    ;;
  status)
    launchctl list | grep arch-hotzone || echo "com.newchanlun.arch-hotzone 未在运行"
    ;;
  *)
    echo "用法: $0 [install|uninstall|status]" >&2
    exit 2
    ;;
esac
