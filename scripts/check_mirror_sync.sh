#!/bin/sh
# 兼容旧 post-commit 挂法；镜像职责已退役，统一转交 main 同步检查（#1103）。
exec sh "$(dirname "$0")/check_main_sync.sh" "$@"
