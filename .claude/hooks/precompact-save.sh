#!/bin/bash
# PreCompact Hook — 统一 session 快照
#
# 当 Claude Code 上下文即将被压缩时，自动保存蜂群状态。
# session = 指针，不是叙事。内容活在文件系统里，session 只存引用。
# 141号修复：增加蜂群状态采集（teams/ + tasks/），闭合 compact 后蜂群追踪丢失的缺口。
#
# 输入：JSON (stdin) — session_id, transcript_path, cwd 等
# 输出：JSON (stdout) — continue=true, systemMessage=状态摘要

set -euo pipefail

resolve_python() {
    local bin
    for bin in python python3; do
        if command -v "$bin" >/dev/null 2>&1 && "$bin" -c "import sys" >/dev/null 2>&1; then
            echo "$bin"
            return 0
        fi
    done
    return 1
}

PYTHON_BIN="$(resolve_python || true)"
if [ -z "$PYTHON_BIN" ]; then
    exit 0
fi

python() { command "$PYTHON_BIN" "$@"; }

# 读取 stdin 的 JSON 输入
input=$(timeout 3 cat 2>/dev/null || echo "{}")
cwd=$(echo "$input" | python -c "import sys,json; print(json.loads(sys.stdin.read()).get('cwd', '.'))" 2>/dev/null || echo ".")

# 确保在项目目录内
cd "$cwd" 2>/dev/null || cd "$(git rev-parse --show-toplevel 2>/dev/null)" 2>/dev/null || true

# 调用独立 session 写入脚本
SCRIPT_DIR_REL="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
SESSION_FILE=$(bash "$SCRIPT_DIR_REL/write_session.sh" 2>/dev/null || true)

if [ -z "$SESSION_FILE" ]; then
    # write_session.sh 失败或无 python，使用回退路径
    SESSION_FILE=".chanlun/sessions/$(date +"%Y-%m-%d-%H%M")-session.md"
fi

# 生成系统消息（从 session 文件提取摘要）
MSG="[Session] 状态已保存: ${SESSION_FILE}"
if [ -f "$SESSION_FILE" ]; then
    # 从已写入的 session 文件提取关键数字
    SUMMARY=$(python -c "
import sys, re
with open(sys.argv[1], encoding='utf-8') as f:
    text = f.read()
def_count = text.count('|') // 4  # 粗略计算定义行数
pending_m = re.search(r'生成态: (\d+)', text)
settled_m = re.search(r'已结算: (\d+)', text)
pending = pending_m.group(1) if pending_m else '?'
settled = settled_m.group(1) if settled_m else '?'
swarm_count = text.count('### 蜂群:')
print(f'定义{def_count}条 | 谱系{pending}生成态/{settled}已结算 | 蜂群{swarm_count}活跃')
" "$SESSION_FILE" 2>/dev/null || echo "")
    [ -n "$SUMMARY" ] && MSG="${MSG} | ${SUMMARY}"
fi

# 输出 JSON 响应（使用 python json.dumps 保证转义安全）
python -c "
import json, sys
msg = sys.argv[1]
print(json.dumps({'continue': True, 'suppressOutput': False, 'systemMessage': msg}, ensure_ascii=False))
" "$MSG"
