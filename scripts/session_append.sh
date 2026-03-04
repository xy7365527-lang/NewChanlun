#!/usr/bin/env bash
# session_append.sh — 一行 session 增量追加（降低调用成本）
#
# 用法: bash scripts/session_append.sh "工位名: 产出摘要"
#
# 设计意图（Codex 持存诊断·假设3缓解）：
# Lead 不调用 session_update.py --append 的一个原因是 python 命令太长。
# 本脚本将调用简化为 bash 一行，降低 LLM 的隐性效率优化倾向。
#
# 如果无 session 文件，静默退出（不阻塞 ceremony 流程）。

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

if [ $# -lt 1 ]; then
    echo "[session_append] 用法: bash scripts/session_append.sh \"工位名: 产出摘要\"" >&2
    exit 1
fi

OUTPUT_LINE="$1"

# 找到最新 session 文件
SESSIONS_DIR="$REPO_ROOT/.chanlun/sessions"
if [ ! -d "$SESSIONS_DIR" ]; then
    exit 0
fi

LATEST=$(ls -t "$SESSIONS_DIR"/*-session.md 2>/dev/null | head -1 || true)
if [ -z "$LATEST" ] || [ ! -f "$LATEST" ]; then
    exit 0
fi

# 调用 session_update.py --append
PYTHON_BIN=""
for candidate in python python3; do
    if command -v "$candidate" >/dev/null 2>&1; then
        PYTHON_BIN="$candidate"
        break
    fi
done

if [ -z "$PYTHON_BIN" ]; then
    echo "[session_append] 无可用 python" >&2
    exit 1
fi

"$PYTHON_BIN" "$SCRIPT_DIR/session_update.py" --append "$OUTPUT_LINE"
