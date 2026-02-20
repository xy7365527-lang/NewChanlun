#!/bin/bash
# Stop Hook — 通用停机阻断（Universal Stop-Guard）
# 048号谱系：从 044号（ceremony 专用）泛化为全场景覆盖
# 069号更新：废弃 ceremony 计数器状态机，改用显式状态检测
#
# 触发：Stop 事件（agent 即将结束 turn）
# 逻辑：
#   1. 检测"有活干但没人在干"的死寂状态 → 注入强指令启动蜂群
#   2. 蜂群任务队列非空 → 阻止 + 注入具体路由
#   3. 谱系有生成态矛盾 → 阻止 + 注入具体文件名和四分法指令
#   4. @proof-required 标签未验证 → 阻止 + 路由到 Gemini 数学验证
#   5. 以上均无 → 放行
#
# 熔断机制：
#   - 连续阻止 >= 5 次且无状态变更 → 允许停止
#   - 用户 INTERRUPT → 允许停止

set -euo pipefail

input=$(cat)
cwd=$(echo "$input" | python -c "import sys,json; print(json.loads(sys.stdin.read()).get('cwd', '.'))" 2>/dev/null || echo ".")
cd "$cwd" 2>/dev/null || true

COUNTER=".chanlun/.stop-guard-counter"

# ─── 熔断检查 ───
COUNT=0
if [ -f "$COUNTER" ]; then
    COUNT=$(cat "$COUNTER" 2>/dev/null || echo "0")
    COUNT=$((COUNT + 0))
fi

if [ "$COUNT" -ge 5 ]; then
    rm -f "$COUNTER" 2>/dev/null || true
    python -c "
import json
print(json.dumps({
    'decision': 'allow',
    'reason': '[Stop-Guard 熔断] 连续阻止 5 次无状态变更。允许停止。'
}, ensure_ascii=False))
"
    exit 0
fi

# ─── 检查 1：069号下游推论或 session 中断点有未执行的工作 ───
# 检测"有活干但没人在干"：session 记录有下一轮方向，但没有活跃蜂群
HAS_PENDING_WORK=0
WORK_DESCRIPTION=""

# 检查最新 session 的下一轮方向
LATEST_SESSION=$(ls -t .chanlun/sessions/20*-session.md 2>/dev/null | head -1)
if [ -n "$LATEST_SESSION" ]; then
    # 检查是否有"下一轮方向"或"下一轮 Blocker"章节
    if grep -q "## 下一轮" "$LATEST_SESSION" 2>/dev/null; then
        # 检查是否有活跃的蜂群（team 目录存在）
        ACTIVE_TEAMS=$(ls -d "$HOME/.claude/teams"/*/ 2>/dev/null | wc -l)
        if [ "$ACTIVE_TEAMS" -eq 0 ]; then
            # 有工作但没有蜂群在跑
            WORK_DESCRIPTION=$(python -c "
import sys
with open(sys.argv[1], 'r', encoding='utf-8') as f:
    lines = f.readlines()
in_next = False
items = []
for line in lines:
    if line.startswith('## 下一轮'):
        in_next = True
        continue
    if in_next and line.startswith('## '):
        break
    if in_next and line.strip().startswith(('-', '*', '1', '2', '3', '4', '5')):
        items.append(line.strip().lstrip('-*0123456789. '))
print(' | '.join(items[:5]))
" "$LATEST_SESSION" 2>/dev/null || echo "")
            if [ -n "$WORK_DESCRIPTION" ]; then
                HAS_PENDING_WORK=1
            fi
        fi
    fi
fi

if [ "$HAS_PENDING_WORK" -gt 0 ]; then
    echo $((COUNT + 1)) > "$COUNTER"
    python -c "
import json, sys
work = sys.argv[1]
print(json.dumps({
    'decision': 'block',
    'reason': f'[Stop-Guard] 检测到死寂状态：有待推进工作但无活跃蜂群。立即评估并行工位数，拉取递归拓扑蜂群。待推进: {work}'
}, ensure_ascii=False))
" "$WORK_DESCRIPTION"
    exit 0
fi

# ─── 检查 2：蜂群任务队列 ───
# 扫描活跃任务状态，生成具体路由指令
ACTIVE_TASKS=0
PENDING_TASKS=""
IN_PROGRESS_TASKS=""
COMPLETED_TASKS=""
if [ -d "$HOME/.claude/tasks" ]; then
    for team_dir in "$HOME/.claude/tasks"/*/; do
        [ -d "$team_dir" ] || continue
        for task_file in "$team_dir"*.json; do
            [ -f "$task_file" ] || continue
            # 提取 status 和 subject
            TASK_INFO=$(python -c "
import json, sys
with open(sys.argv[1]) as f:
    d = json.load(f)
print(d.get('status',''), d.get('subject',''))
" "$task_file" 2>/dev/null) || continue
            TASK_STATUS=$(echo "$TASK_INFO" | cut -d' ' -f1)
            TASK_NAME=$(echo "$TASK_INFO" | cut -d' ' -f2-)
            case "$TASK_STATUS" in
                pending)
                    ACTIVE_TASKS=$((ACTIVE_TASKS + 1))
                    PENDING_TASKS="${PENDING_TASKS:+$PENDING_TASKS, }$TASK_NAME"
                    ;;
                in_progress)
                    ACTIVE_TASKS=$((ACTIVE_TASKS + 1))
                    IN_PROGRESS_TASKS="${IN_PROGRESS_TASKS:+$IN_PROGRESS_TASKS, }$TASK_NAME"
                    ;;
                completed)
                    COMPLETED_TASKS="${COMPLETED_TASKS:+$COMPLETED_TASKS, }$TASK_NAME"
                    ;;
            esac
        done
    done
fi

if [ "$ACTIVE_TASKS" -gt 0 ]; then
    echo $((COUNT + 1)) > "$COUNTER"
    python -c "
import json, sys
active = int(sys.argv[1])
pending = sys.argv[2]
in_progress = sys.argv[3]
completed = sys.argv[4]

# 构建具体路由指令
instructions = []

# 已完成的工位 = idle，可以分配新任务或关闭
if completed:
    idle_names = completed.split(', ')
    for name in idle_names:
        instructions.append(f'工位 {name} 空闲，分配下一个任务或发送 shutdown_request 关闭')

# 有 pending 任务但没有对应工位在运行
if pending:
    instructions.append(f'待分配任务: [{pending}]，启动工位或分配给空闲工位')

# 所有活跃任务都在运行中（无 pending、无 completed idle）
if in_progress and not pending and not completed:
    instructions.append(f'所有工位运行中 [{in_progress}]。检查是否有概念层问题可以同步处理（Gemini 质询、谱系检查）')

# 有运行中的工位，列出以便跟踪
if in_progress and (pending or completed):
    instructions.append(f'运行中: [{in_progress}]，等待汇报')

route = ' | '.join(instructions) if instructions else '请检查 TaskList 并推进未完成任务'

print(json.dumps({
    'decision': 'block',
    'reason': f'[Stop-Guard] 蜂群任务队列有 {active} 个活跃任务。不允许停止。路由指令: {route}'
}, ensure_ascii=False))
" "$ACTIVE_TASKS" "$PENDING_TASKS" "$IN_PROGRESS_TASKS" "$COMPLETED_TASKS"
    exit 0
fi

# ─── 检查 3：生成态谱系矛盾 ───
PENDING_COUNT=0
PENDING_FILES=""
if [ -d ".chanlun/genealogy/pending" ]; then
    while IFS= read -r f; do
        [ -z "$f" ] && continue
        PENDING_COUNT=$((PENDING_COUNT + 1))
        BASENAME=$(basename "$f")
        PENDING_FILES="${PENDING_FILES:+$PENDING_FILES, }$BASENAME"
    done < <(find .chanlun/genealogy/pending -name "*.md" -type f 2>/dev/null)
fi

if [ "$PENDING_COUNT" -gt 0 ]; then
    echo $((COUNT + 1)) > "$COUNTER"
    python -c "
import json, sys
n = sys.argv[1]
files = sys.argv[2]
# 取第一个文件作为优先推进目标
first = files.split(', ')[0] if files else ''
print(json.dumps({
    'decision': 'block',
    'reason': f'[Stop-Guard] 谱系有 {n} 个生成态矛盾待处理: [{files}]。不允许停止。推进 {first} 的结算：读取文件，判断四分法分类（吸收/修正/分裂/废弃），执行对应动作。'
}, ensure_ascii=False))
" "$PENDING_COUNT" "$PENDING_FILES"
    exit 0
fi

# ─── 检查 4：@proof-required 标签扫描 ───
PROOF_REQUIRED=0
PROOF_LOCATIONS=""
if [ -d "spec" ] || [ -d "src" ] || [ -d ".chanlun" ]; then
    PROOF_SCAN=$(grep -rn "@proof-required" spec/ src/ .chanlun/ 2>/dev/null | grep -v '/genealogy/settled/' | grep -v 'README.md' | grep -v 'dispatch-spec.yaml' | grep -v 'tags:.*@proof-required' | grep -v '/sessions/' || true)
    if [ -n "$PROOF_SCAN" ]; then
        PROOF_REQUIRED=$(echo "$PROOF_SCAN" | wc -l)
        PROOF_LOCATIONS=$(echo "$PROOF_SCAN" | head -5 | while IFS= read -r line; do
            FILE=$(echo "$line" | cut -d: -f1)
            LINE_NUM=$(echo "$line" | cut -d: -f2)
            echo "$FILE:$LINE_NUM"
        done | tr '\n' ', ' | sed 's/, $//')
    fi
fi

if [ "$PROOF_REQUIRED" -gt 0 ]; then
    echo $((COUNT + 1)) > "$COUNTER"
    python -c "
import json, sys
n = sys.argv[1]
locs = sys.argv[2]
print(json.dumps({
    'decision': 'block',
    'reason': f'[Stop-Guard] 发现 {n} 个 @proof-required 标签未验证: [{locs}]。不允许停止。路由指令: 路由到 Gemini 数学验证，或标记为已验证。'
}, ensure_ascii=False))
" "$PROOF_REQUIRED" "$PROOF_LOCATIONS"
    exit 0
fi

# ─── 全部检查通过：允许停止 ───
rm -f "$COUNTER" 2>/dev/null || true
python -c "
import json
print(json.dumps({
    'decision': 'allow',
    'reason': ''
}, ensure_ascii=False))
"
exit 0
