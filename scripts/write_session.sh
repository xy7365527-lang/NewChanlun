#!/usr/bin/env bash
# write_session.sh — 独立 session 采集+写入（从 precompact-save.sh 提取）
#
# 用法: bash scripts/write_session.sh [session_file_path]
#
# 参数:
#   $1 — 可选的 session 文件路径。默认自动生成带时间戳的路径。
#
# 行为:
#   - 采集定义基底、谱系状态、git 状态、蜂群状态、中断点、checkpoints
#   - 写入 session 文件（markdown 格式，与 precompact-save.sh 完全一致）
#   - 幂等：同一分钟内多次调用更新同一文件（基于时间戳匹配）
#   - 输出 session 文件路径到 stdout
#
# 调用方:
#   - precompact-save.sh（PreCompact hook）
#   - ceremony_push_and_rescan.sh（ceremony 步骤7 commit 前）
#   - ceremony-completion-guard.sh（Stop hook 放行前）

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT" || exit 1

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
    echo "[write_session] 无可用 python，跳过 session 写入" >&2
    exit 0
fi

python() { command "$PYTHON_BIN" "$@"; }

# 确保 sessions 目录存在
mkdir -p .chanlun/sessions

TIMESTAMP=$(date +"%Y-%m-%d-%H%M")

# 确定 session 文件路径
if [ -n "${1:-}" ]; then
    SESSION_FILE="$1"
else
    SESSION_FILE=".chanlun/sessions/${TIMESTAMP}-session.md"
fi

# ─── 采集定义状态 ───
DEFINITIONS=""
if [ -d ".chanlun/definitions" ]; then
    for f in .chanlun/definitions/*.md; do
        [ -f "$f" ] || continue
        name=$(basename "$f" .md)
        version=$(grep -m1 '^\*\*版本\*\*' "$f" 2>/dev/null | sed 's/.*: *//' || true)
        [ -z "$version" ] && version="?"
        status=$(grep -m1 '^\*\*状态\*\*' "$f" 2>/dev/null | sed 's/.*: *//' || true)
        [ -z "$status" ] && status="?"
        DEFINITIONS="${DEFINITIONS}\n| ${name} | ${version} | ${status} |"
    done
fi

# ─── 采集谱系状态 ───
PENDING_COUNT=0
SETTLED_COUNT=0
PENDING_LIST=""
if [ -d ".chanlun/genealogy/pending" ]; then
    for f in .chanlun/genealogy/pending/*.md; do
        [ -f "$f" ] || continue
        PENDING_COUNT=$((PENDING_COUNT + 1))
        id=$(basename "$f" .md)
        PENDING_LIST="${PENDING_LIST}\n  - ${id}"
    done
fi
if [ -d ".chanlun/genealogy/settled" ]; then
    SETTLED_COUNT=$(find .chanlun/genealogy/settled -name "*.md" 2>/dev/null | wc -l || true)
fi

# ─── 采集 git 状态 ───
GIT_BRANCH=$(git branch --show-current 2>/dev/null || echo "unknown")
GIT_COMMIT=$(git log --oneline -1 2>/dev/null || echo "unknown")
GIT_DIRTY=$(git diff --stat 2>/dev/null | tail -1 || true)
[ -z "$GIT_DIRTY" ] && GIT_DIRTY="clean"

# ─── 采集活跃蜂群状态 ───
SWARM_STATUS=""
TEAMS_DIR="$HOME/.claude/teams"
TASKS_DIR="$HOME/.claude/tasks"
if [ -d "$TEAMS_DIR" ]; then
    for team_dir in "$TEAMS_DIR"/*/; do
        [ -d "$team_dir" ] || continue
        team_name=$(basename "$team_dir")
        config="$team_dir/config.json"
        [ -f "$config" ] || continue
        members=$(python -c "
import json, sys
with open(sys.argv[1]) as f:
    c = json.load(f)
for m in c.get('members', []):
    print(f\"  - {m['name']} ({m['agentType']})\")
" "$config" 2>/dev/null || echo "  - (读取失败)")
        task_summary=""
        task_dir="$TASKS_DIR/$team_name"
        if [ -d "$task_dir" ]; then
            task_summary=$(python -c "
import json, os, sys
d = sys.argv[1]
for f in sorted(os.listdir(d)):
    if not f.endswith('.json'): continue
    with open(os.path.join(d, f)) as fh:
        t = json.load(fh)
    owner = t.get('owner', '')
    blocked = t.get('blockedBy', [])
    s = t.get('status', '?')
    subj = t.get('subject', '?')
    line = f\"  - [{s}] {subj}\"
    if owner: line += f\" (owner:{owner})\"
    if blocked: line += f\" (blocked:{blocked})\"
    print(line)
" "$task_dir" 2>/dev/null || echo "  - (任务读取失败)")
        fi
        SWARM_STATUS="${SWARM_STATUS}
### 蜂群: ${team_name}
成员:
${members}
任务:
${task_summary:-  - (无任务)}"
    done
fi

# ─── 采集中断点 ───
PREV_SESSION=""
PREV_INTERRUPTS=""
LATEST_SESSION=$(ls -t .chanlun/sessions/*-session.md 2>/dev/null | head -1 || true)
# 如果最新 session 就是当前要写的文件，取第二新的
if [ -n "$LATEST_SESSION" ] && [ "$LATEST_SESSION" = "$SESSION_FILE" ]; then
    LATEST_SESSION=$(ls -t .chanlun/sessions/*-session.md 2>/dev/null | sed -n '2p' || true)
fi
if [ -n "$LATEST_SESSION" ] && [ -f "$LATEST_SESSION" ]; then
    PREV_SESSION="$LATEST_SESSION"
    PREV_INTERRUPTS=$(sed -n '/^## 中断点$/,/^## /{/^## /d; p}' "$LATEST_SESSION" 2>/dev/null | head -20 || true)
fi

# G1修复：检测中断点是否过时
if [ -n "$PREV_SESSION" ]; then
    SESSION_MTIME=$(python -c "import os,sys; print(int(os.path.getmtime(sys.argv[1])))" "$PREV_SESSION" 2>/dev/null || echo 0)
    LATEST_COMMIT_TIME=$(git log -1 --format=%ct 2>/dev/null || echo 0)
    if [ "$LATEST_COMMIT_TIME" -gt "$SESSION_MTIME" ]; then
        RECENT=$(git log --oneline -5 --after="@${SESSION_MTIME}" 2>/dev/null || true)
        if [ -n "$RECENT" ]; then
            STALE_SUPPLEMENT="$(echo "$RECENT" | sed 's/^/  - /')"
            PREV_INTERRUPTS="${PREV_INTERRUPTS}
- ⚠ session后新增提交（中断点可能过时）:
${STALE_SUPPLEMENT}"
        fi
    fi
fi

# ─── 283号缺口A：采集工位 checkpoints 摘要 ───
CHECKPOINT_SUMMARY=""
if [ -d ".chanlun/checkpoints" ]; then
    CHECKPOINT_SUMMARY=$(python -c "
import json, os, glob
entries = []
for f in sorted(glob.glob('.chanlun/checkpoints/*/*.json')):
    if f.endswith('.tmp'): continue
    try:
        with open(f, encoding='utf-8') as fh:
            d = json.load(fh)
        entries.append(f\"| {d.get('team_name','?')} | {d.get('agent_name','?')} | {d.get('phase','?')} | {d.get('updated_at','?')[:19]} |\")
    except Exception:
        pass
if entries:
    print('| team | agent | phase | updated_at |')
    print('|------|-------|-------|------------|')
    for e in entries:
        print(e)
else:
    print('（无活跃 checkpoint）')
" 2>/dev/null || echo "（checkpoint 采集失败）")
fi

# ─── 写入 session ───
cat > "$SESSION_FILE" << SESSION_EOF
# Session

**时间**: ${TIMESTAMP}
**分支**: ${GIT_BRANCH}
**最新提交**: ${GIT_COMMIT}
**工作树**: ${GIT_DIRTY}

## 定义基底
| 名称 | 版本 | 状态 |
|------|------|------|$(echo -e "$DEFINITIONS")
→ 来源: .chanlun/definitions/*.md

## 谱系状态
- 生成态: ${PENDING_COUNT} 个${PENDING_LIST:+$(echo -e "$PENDING_LIST")}
- 已结算: ${SETTLED_COUNT} 个
→ 来源: .chanlun/genealogy/{pending,settled}/

## 活跃蜂群
${SWARM_STATUS:-"（无活跃蜂群）"}

## 工位 Checkpoints
${CHECKPOINT_SUMMARY}

## 中断点
${PREV_INTERRUPTS:-"（自动快照，中断点待 CC 下次写入）"}

## 恢复指引
1. 读取此文件获取状态指针
2. 如有活跃蜂群：先用 TaskList 恢复工位追踪，继续蜂群循环
3. 扫描 definitions/ 和 genealogy/ 获取当前状态
4. 按中断点评估可并行工位，直接进入蜂群循环
SESSION_EOF

# 输出 session 文件路径
echo "$SESSION_FILE"
