#!/bin/bash
# Stop Hook — 通用停机阻断（Universal Stop-Guard）
# 048号谱系：从 044号（ceremony 专用）泛化为全场景覆盖
# 069号更新：废弃 ceremony 计数器状态机，改用显式状态检测
# 075号更新：移除 dominator node 检查（结构能力由 skill 事件驱动，不再是 teammate）
# 155号修复：僵尸工位检测——使用 owner 而非 subject 标识工位；completed 不再报告为空闲
#
# 触发：Stop 事件（agent 即将结束 turn）
# 逻辑：
#   1. 检测"有活干但没人在干"的死寂状态 → 注入强指令启动蜂群
#   2. 蜂群任务队列非空 → 阻止 + 注入具体路由
#   3. 谱系有生成态矛盾 → 阻止 + 注入具体文件名和四分法指令
#   4. @proof-required 标签未验证 → 阻止 + 路由到 Gemini 数学验证
#   5. ceremony 确认请求检测 → 阻止
#   6. 以上均无 → 放行
#
# 熔断机制：
#   - 连续阻止 >= 3 次且状态无变化 → 允许停止（145号智能熔断）
#   - 用户 INTERRUPT → 允许停止

set -uo pipefail

resolve_python() {
    for candidate in python3 python; do
        if command -v "$candidate" >/dev/null 2>&1; then
            if "$candidate" -c "import sys" >/dev/null 2>&1; then
                echo "$candidate"
                return 0
            fi
        fi
    done
    return 1
}

PYTHON_BIN="$(resolve_python || true)"
if [ -z "$PYTHON_BIN" ]; then
    exit 0
fi

python() { command "$PYTHON_BIN" "$@"; }

input=$(cat)
cwd=$(echo "$input" | python -c "import sys,json; print(json.loads(sys.stdin.read()).get('cwd', '.'))" 2>/dev/null || echo ".")
cd "$cwd" 2>/dev/null || true

COUNTER=".chanlun/.stop-guard-counter"

# ─── 熔断检查（145号：智能熔断，计数器格式 COUNT:LAST_ACTIVE_TASKS） ───
COUNT=0
LAST_ACTIVE=0
if [ -f "$COUNTER" ]; then
    COUNTER_DATA=$(cat "$COUNTER" 2>/dev/null || echo "0:0")
    COUNT=$(echo "$COUNTER_DATA" | cut -d: -f1)
    LAST_ACTIVE=$(echo "$COUNTER_DATA" | cut -d: -f2)
    COUNT=$((COUNT + 0))
    LAST_ACTIVE=$((LAST_ACTIVE + 0))
fi

# 注意：ACTIVE_TASKS 在下方检查2中计算，此处先做预计算以支持智能熔断
PRE_ACTIVE_TASKS=0
if [ -d "$HOME/.claude/tasks" ]; then
    for td in "$HOME/.claude/tasks"/*/; do
        [ -d "$td" ] || continue
        for tf in "$td"*.json; do
            [ -f "$tf" ] || continue
            case "$tf" in *.lock) continue ;; esac
            ts=$(python -c "
import json, sys
with open(sys.argv[1]) as f: d=json.load(f)
s=d.get('status','')
if s in ('pending','in_progress'): print('1')
else: print('0')
" "$tf" 2>/dev/null || echo "0")
            PRE_ACTIVE_TASKS=$((PRE_ACTIVE_TASKS + ts))
        done
    done
fi

if [ "$COUNT" -ge 3 ] && [ "$PRE_ACTIVE_TASKS" -eq "$LAST_ACTIVE" ]; then
    # 状态停滞 3 次，放行
    rm -f "$COUNTER" 2>/dev/null || true
    exit 0
fi

# ─── 检查 1：069号下游推论或 session 中断点有未执行的工作 ───
# 检测"有活干但没人在干"：session 记录有下一轮方向，但没有活跃蜂群
HAS_PENDING_WORK=0
WORK_DESCRIPTION=""

# 检查最新 session 的下一轮方向
LATEST_SESSION=$(ls -t .chanlun/sessions/20*-session.md 2>/dev/null | head -1) || true
if [ -n "$LATEST_SESSION" ]; then
    # 检查是否有"下一轮方向"或"下一轮 Blocker"章节
    if grep -q "## 下一轮" "$LATEST_SESSION" 2>/dev/null; then
        # 检查是否有活跃的蜂群（team 目录存在）
        ACTIVE_TEAMS=$(ls -d "$HOME/.claude/teams"/*/ 2>/dev/null | wc -l) || ACTIVE_TEAMS=0
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
    echo "$((COUNT + 1)):$PRE_ACTIVE_TASKS" > "$COUNTER"
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
# 145号修复：排除被 blockedBy 阻塞的 pending 任务 + 僵尸任务检测
ACTIVE_TASKS=0
PENDING_TASKS=""
IN_PROGRESS_TASKS=""
if [ -d "$HOME/.claude/tasks" ]; then
    for team_dir in "$HOME/.claude/tasks"/*/; do
        [ -d "$team_dir" ] || continue
        team_name=$(basename "$team_dir")
        for task_file in "$team_dir"*.json; do
            [ -f "$task_file" ] || continue
            # 跳过 .lock 文件
            case "$task_file" in *.lock) continue ;; esac
            # 提取 status, owner, subject, blockedBy
            # 155号修复：使用 owner（agent name）而非 subject 作为工位标识
            TASK_INFO=$(python -c "
import json, sys
with open(sys.argv[1]) as f:
    d = json.load(f)
status = d.get('status','')
owner = d.get('owner','')
subject = d.get('subject','')
blocked = d.get('blockedBy', [])
# 只有当 blockedBy 中所有任务都已完成时，才算未阻塞
has_open_blockers = False
if blocked:
    import os
    task_dir = os.path.dirname(sys.argv[1])
    for bid in blocked:
        bf = os.path.join(task_dir, f'{bid}.json')
        if os.path.exists(bf):
            with open(bf) as bfh:
                bs = json.load(bfh).get('status','')
            if bs != 'completed':
                has_open_blockers = True
                break
        else:
            has_open_blockers = True
            break
# 输出格式: status<TAB>blocked<TAB>owner<TAB>subject
# 使用 TAB 分隔避免 subject 中含空格导致 cut 错位
print(f'{status}\t{\"BLOCKED\" if has_open_blockers else \"UNBLOCKED\"}\t{owner}\t{subject}')
" "$task_file" 2>/dev/null) || continue
            TASK_STATUS=$(echo "$TASK_INFO" | cut -f1)
            TASK_BLOCKED=$(echo "$TASK_INFO" | cut -f2)
            TASK_OWNER=$(echo "$TASK_INFO" | cut -f3)
            TASK_SUBJECT=$(echo "$TASK_INFO" | cut -f4-)
            # 155号修复：用 owner 标识工位，subject 仅作描述
            # completed 任务不再报告为"空闲工位"——工位退出后不应被追踪
            DISPLAY_NAME="${TASK_OWNER:-未分配}"
            case "$TASK_STATUS" in
                pending)
                    # 145号：被阻塞的 pending 不计为活跃——它们无法被分配
                    if [ "$TASK_BLOCKED" = "UNBLOCKED" ]; then
                        ACTIVE_TASKS=$((ACTIVE_TASKS + 1))
                        PENDING_TASKS="${PENDING_TASKS:+$PENDING_TASKS, }${TASK_SUBJECT}(→${DISPLAY_NAME})"
                    fi
                    ;;
                in_progress)
                    ACTIVE_TASKS=$((ACTIVE_TASKS + 1))
                    IN_PROGRESS_TASKS="${IN_PROGRESS_TASKS:+$IN_PROGRESS_TASKS, }${DISPLAY_NAME}:${TASK_SUBJECT}"
                    ;;
                completed)
                    # 155号修复：completed 任务不计入活跃，不报告为空闲
                    # 已完成 = 工位已交付，不需要路由指令
                    ;;
            esac
        done
    done
fi

if [ "$ACTIVE_TASKS" -gt 0 ]; then
    # 145号智能熔断：写入 COUNT:ACTIVE_TASKS 格式
    if [ "$ACTIVE_TASKS" -ne "$LAST_ACTIVE" ]; then
        # 状态发生变化，重置计数器
        echo "1:$ACTIVE_TASKS" > "$COUNTER"
    else
        echo "$((COUNT + 1)):$ACTIVE_TASKS" > "$COUNTER"
    fi
    # 155号修复：移除僵尸工位报告（completed 不再参与路由）
    python -c "
import json, sys
active = int(sys.argv[1])
pending = sys.argv[2]
in_progress = sys.argv[3]

# 构建具体路由指令
instructions = []

# 有 pending 任务但没有对应工位在运行
if pending:
    instructions.append(f'待分配任务: [{pending}]，启动工位或分配给空闲工位')

# 所有活跃任务都在运行中（无 pending）
if in_progress and not pending:
    instructions.append(f'所有工位运行中 [{in_progress}]。检查是否有概念层问题可以同步处理（Gemini 质询、谱系检查）')

# 有运行中的工位且有 pending
if in_progress and pending:
    instructions.append(f'运行中: [{in_progress}]，等待汇报')

route = ' | '.join(instructions) if instructions else '请检查 TaskList 并推进未完成任务'

print(json.dumps({
    'decision': 'block',
    'reason': f'[Stop-Guard] 蜂群任务队列有 {active} 个活跃任务。不允许停止。路由指令: {route}'
}, ensure_ascii=False))
" "$ACTIVE_TASKS" "$PENDING_TASKS" "$IN_PROGRESS_TASKS"
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
    echo "$((COUNT + 1)):$PRE_ACTIVE_TASKS" > "$COUNTER"
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
    PROOF_SCAN=$(grep -rn "@proof-required" spec/ src/ .chanlun/ 2>/dev/null | grep -v '/genealogy/settled/' | grep -v 'README.md' | grep -v 'dispatch-spec.yaml' | grep -v 'dispatch-dag.yaml' | grep -v 'tags:.*@proof-required' | grep -v '/sessions/' || true)
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
    echo "$((COUNT + 1)):$PRE_ACTIVE_TASKS" > "$COUNTER"
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

# ─── 检查 5：ceremony 确认请求检测（058号谱系） ───
if [ -f ".chanlun/.ceremony-in-progress" ]; then
    CONFIRM_PATTERNS='待确认|以上理解是否正确|如有偏差请指出|是否现在处理|是否有新的|请确认|等待.*确认'
    STOP_CONTENT=$(echo "$input" | python -c "
import sys, json
try:
    d = json.loads(sys.stdin.read())
    print(d.get('stop_hook_content', d.get('content', '')))
except: pass
" 2>/dev/null || true)
    if echo "$STOP_CONTENT" | grep -qP "$CONFIRM_PATTERNS" 2>/dev/null; then
        python -c "
import json
print(json.dumps({
    'decision': 'block',
    'reason': '[Stop-Guard] ceremony 阶段检测到确认请求（违反058号谱系）。不允许停止。路由指令: 删除确认请求，直接输出行动声明并执行。'
}, ensure_ascii=False))
"
        exit 0
    fi
fi

# ─── 全部检查通过：允许停止（静默退出） ───
rm -f "$COUNTER" 2>/dev/null || true
exit 0
