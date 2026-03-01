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
    for bin in python3 python; do
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
input=$(cat)
cwd=$(echo "$input" | python -c "import sys,json; print(json.loads(sys.stdin.read()).get('cwd', '.'))" 2>/dev/null || echo ".")

# 确保在项目目录内
cd "$cwd" 2>/dev/null || cd "$(git rev-parse --show-toplevel 2>/dev/null)" 2>/dev/null || true

# 确保 sessions 目录存在
mkdir -p .chanlun/sessions

TIMESTAMP=$(date +"%Y-%m-%d-%H%M")
SESSION_FILE=".chanlun/sessions/${TIMESTAMP}-session.md"

# 采集定义状态
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

# 采集谱系状态
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

# 采集 git 状态
GIT_BRANCH=$(git branch --show-current 2>/dev/null || echo "unknown")
GIT_COMMIT=$(git log --oneline -1 2>/dev/null || echo "unknown")
GIT_DIRTY=$(git diff --stat 2>/dev/null | tail -1 || true)
[ -z "$GIT_DIRTY" ] && GIT_DIRTY="clean"

# 采集活跃蜂群状态（~/.claude/teams/ + ~/.claude/tasks/）
SWARM_STATUS=""
TEAMS_DIR="$HOME/.claude/teams"
TASKS_DIR="$HOME/.claude/tasks"
if [ -d "$TEAMS_DIR" ]; then
    for team_dir in "$TEAMS_DIR"/*/; do
        [ -d "$team_dir" ] || continue
        team_name=$(basename "$team_dir")
        config="$team_dir/config.json"
        [ -f "$config" ] || continue
        # 提取成员列表
        members=$(python -c "
import json, sys
with open(sys.argv[1]) as f:
    c = json.load(f)
for m in c.get('members', []):
    print(f\"  - {m['name']} ({m['agentType']})\")
" "$config" 2>/dev/null || echo "  - (读取失败)")
        # 提取任务状态
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

# 采集中断点：从最近 session 继承 + 过时检测
PREV_SESSION=""
PREV_INTERRUPTS=""
LATEST_SESSION=$(ls -t .chanlun/sessions/*-session.md 2>/dev/null | head -1 || true)
if [ -n "$LATEST_SESSION" ] && [ -f "$LATEST_SESSION" ]; then
    PREV_SESSION="$LATEST_SESSION"
    # 提取中断点章节内容（跳过标题行本身）
    PREV_INTERRUPTS=$(sed -n '/^## 中断点$/,/^## /{/^## /d; p}' "$LATEST_SESSION" 2>/dev/null | head -20 || true)
fi

# G1修复：检测中断点是否过时（session写入后有新提交 = 进度未持久化）
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

# 283号缺口A：采集工位 checkpoints 摘要
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

# 写入 session（统一格式，不再区分 precompact/手动）
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

# 生成系统消息
DEF_COUNT=$(echo -e "$DEFINITIONS" | grep -c '|' || true)
[ -z "$DEF_COUNT" ] && DEF_COUNT=0
# 计算活跃蜂群数
SWARM_COUNT=0
if [ -d "$TEAMS_DIR" ]; then
    SWARM_COUNT=$(ls -d "$TEAMS_DIR"/*/ 2>/dev/null | wc -l || true)
fi
MSG="[Session] 状态已保存: ${SESSION_FILE} | 定义${DEF_COUNT}条 | 谱系${PENDING_COUNT}生成态/${SETTLED_COUNT}已结算 | 蜂群${SWARM_COUNT}活跃"

# 输出 JSON 响应（使用 python json.dumps 保证转义安全）
python -c "
import json, sys
msg = sys.argv[1]
print(json.dumps({'continue': True, 'suppressOutput': False, 'systemMessage': msg}, ensure_ascii=False))
" "$MSG"
