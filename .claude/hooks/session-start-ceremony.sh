#!/bin/bash
# SessionStart Hook — 自动 ceremony（热启动 bootstrap）
#
# 每次会话开始时自动执行。扫描 session 文件和定义/谱系状态，
# 生成差异报告注入 hookSpecificOutput.additionalContext（可靠 context 通道，
# 被 CC 包裹进 system-reminder 注入模型上下文；systemMessage 仅是用户可见提示，
# 不可靠地进入模型 context——这是 compact 恢复后 ceremony 不自动触发的根因）。
#
# source=compact 分支：用 137号正面格式声明"第一个动作"，
# 显式覆盖 compact summary 的 "resume directly" 软冲突。
#
# 边界（097号 hook 纯化）：SessionStart 本质只能注入 context（无 tool 可阻断/放行），
# 注入运行时状态快照是其 bootstrap 设计目的，不是 097号否定的"提示/索引第四层"
# （那是指向 skill 路径的 PostToolUse 索引，已删除并迁入 CLAUDE.md 声明层）。
#
# 效果等同于用户手动输入 /ceremony，但零人工干预。

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
[ -n "$PYTHON_BIN" ] || exit 0

python() { command "$PYTHON_BIN" "$@"; }

input=$(timeout 3 cat 2>/dev/null || echo "{}")
cwd=$(echo "$input" | python -c "import sys,json; print(json.loads(sys.stdin.read()).get('cwd', '.'))" 2>/dev/null || echo ".")
# source ∈ {startup, resume, clear, compact}；空 matcher 已覆盖全部 source（含 compact）
SOURCE=$(echo "$input" | python -c "import sys,json; print(json.loads(sys.stdin.read()).get('source', ''))" 2>/dev/null || echo "")
cd "$cwd" 2>/dev/null || true

# ─── ceremony 相位标记 ───
# 创建 ceremony-in-progress 标记，由 ceremony-completion-guard (Stop hook) 检查
# 057号谱系：flag 由框架自动管理（stop-guard 第二次 block 时自动清除）
mkdir -p .chanlun 2>/dev/null || true
touch .chanlun/.ceremony-in-progress
rm -f .chanlun/.stop-guard-counter .chanlun/.ceremony-blocked-once .chanlun/.meta-observer-executed .chanlun/.meta-observer-guard-counter 2>/dev/null || true

# ─── 安全 JSON 输出 ───
# 用 python json.dumps 保证转义正确，避免裸拼接导致畸形 JSON
# $1 = additionalContext（可执行内容，注入模型 context 的可靠通道）
# $2 = systemMessage（用户可见的简短提示，advisory，可省略）
emit_json() {
    local ctx="$1"
    local sysmsg="${2:-}"
    python -c "
import json, sys
ctx = sys.argv[1]
sysmsg = sys.argv[2] if len(sys.argv) > 2 else ''
out = {
    'continue': True,
    'suppressOutput': False,
    'hookSpecificOutput': {
        'hookEventName': 'SessionStart',
        'additionalContext': ctx,
    },
}
if sysmsg:
    out['systemMessage'] = sysmsg
print(json.dumps(out, ensure_ascii=False))
" "$ctx" "$sysmsg"
}

# ─── 检测模式 ───
SESSION_FILE=""
if [ -d ".chanlun/sessions" ]; then
    SESSION_FILE=$(ls -t .chanlun/sessions/*-session.md 2>/dev/null | head -1 || true)
fi

if [ -z "$SESSION_FILE" ]; then
    # 冷启动：无 session，输出基本状态
    DEF_COUNT=0
    [ -d ".chanlun/definitions" ] && DEF_COUNT=$(ls .chanlun/definitions/*.md 2>/dev/null | wc -l)
    SETTLED=0
    [ -d ".chanlun/genealogy/settled" ] && SETTLED=$(ls .chanlun/genealogy/settled/*.md 2>/dev/null | wc -l)
    PENDING=0
    [ -d ".chanlun/genealogy/pending" ] && PENDING=$(ls .chanlun/genealogy/pending/*.md 2>/dev/null | wc -l)

    # 137号正面格式：声明第一个动作，而非"请执行"软请求
    # 562号/编排者裁决(2026-06-23)：第一动作必须是 invoke /ceremony 命令序列，
    # 不是手动 git diff/状态快照对比。ceremony_scan.py 的确定性输出是工位的唯一来源。
    MSG="[Ceremony/冷启动] 本回合第一个动作=invoke /ceremony 序列：运行 \`python scripts/ceremony_state.py write 1 initial\` → \`python scripts/ceremony_scan.py\`，由 scan 确定性输出（非手动 git diff）决定工位。当前 定义${DEF_COUNT}条 | 谱系${SETTLED}已结算/${PENDING}生成态。"
    emit_json "$MSG" "[Ceremony] 冷启动状态已注入 context"
    exit 0
fi

# ─── 热启动：从 session 恢复 ───
SESSION_TIME=$(grep '^\*\*时间\*\*' "$SESSION_FILE" 2>/dev/null | sed 's/.*: *//' || echo "?")
SESSION_COMMIT=$(grep '^\*\*最新提交\*\*' "$SESSION_FILE" 2>/dev/null | sed 's/^\*\*最新提交\*\*: *//' | cut -d' ' -f1 || echo "?")

# 当前状态
CURRENT_COMMIT=$(git log --oneline -1 2>/dev/null || echo "unknown")
GIT_BRANCH=$(git branch --show-current 2>/dev/null || echo "unknown")

# 定义差异（快速版：只比较版本号）
DEF_DIFF=""
CHANGED=0
if [ -d ".chanlun/definitions" ]; then
    for f in .chanlun/definitions/*.md; do
        [ -f "$f" ] || continue
        name=$(basename "$f" .md)
        cur_ver=$(grep -m1 '^\*\*版本\*\*' "$f" 2>/dev/null | sed 's/.*: *//; s/\*\*//g' || echo "?")
        # 从 session 表格提取该定义的版本（第2列）
        ses_ver=$(grep "| ${name} |" "$SESSION_FILE" 2>/dev/null | awk -F'|' '{gsub(/^ *| *$/,"",$3); print $3}' | head -1 || true)
        [ -z "$ses_ver" ] && ses_ver="?"
        if [ "$cur_ver" = "$ses_ver" ]; then
            mark="="
        else
            mark="↑"
            CHANGED=$((CHANGED + 1))
        fi
        DEF_DIFF="${DEF_DIFF} ${name}(${mark}${cur_ver})"
    done
fi

# 谱系统计
SETTLED=0
[ -d ".chanlun/genealogy/settled" ] && SETTLED=$(ls .chanlun/genealogy/settled/*.md 2>/dev/null | wc -l)
PENDING=0
PENDING_LIST=""
if [ -d ".chanlun/genealogy/pending" ]; then
    for f in .chanlun/genealogy/pending/*.md; do
        [ -f "$f" ] || continue
        PENDING=$((PENDING + 1))
        PENDING_LIST="${PENDING_LIST} $(basename "$f" .md)"
    done
fi

# 中断点
INTERRUPTS=$(sed -n '/^## 中断点$/,/^## /{ /^## 中断点$/d; /^## /d; p; }' "$SESSION_FILE" 2>/dev/null | head -5 | tr '\n' ' ' || echo "无")

# 活跃蜂群检测
SWARM_INFO=""
TEAMS_DIR="$HOME/.claude/teams"
if [ -d "$TEAMS_DIR" ]; then
    for team_dir in "$TEAMS_DIR"/*/; do
        [ -d "$team_dir" ] || continue
        team_name=$(basename "$team_dir")
        config="$team_dir/config.json"
        [ -f "$config" ] || continue
        member_count=$(python -c "
import json, sys
with open(sys.argv[1]) as f:
    c = json.load(f)
print(len(c.get('members', [])))
" "$config" 2>/dev/null || echo "?")
        SWARM_INFO="${SWARM_INFO} ${team_name}(${member_count}成员)"
    done
fi

# team 拓扑持久化检测
TOPOLOGY_FILE=".claude/team-topology.json"
TEAM_BOOTSTRAP_MSG=""
if [ -f "$TOPOLOGY_FILE" ] && [ -z "$SWARM_INFO" ]; then
    # 有拓扑定义但无活跃 team → bootstrap 指令由 agent-team-bootstrap.sh 注入
    TEAM_BOOTSTRAP_MSG=" | [Team Bootstrap 待执行] 蜂群拓扑已持久化，ceremony 后自动重建 Agent Team"
fi

# 构建消息
if [ -n "$SWARM_INFO" ]; then
    SWARM_MSG=" | 活跃蜂群:${SWARM_INFO} → 立即用TaskList恢复工位追踪，继续蜂群循环"
else
    SWARM_MSG="${TEAM_BOOTSTRAP_MSG}"
fi
# 角色边界锚点（407号修复1：compact后行为层恢复）
ROLE_BOUNDARY="⛔Lead角色边界：Lead只做spawn/shutdown/commit/scan/session写入。代码文件(src/,scripts/,tests/,topological-computation/)的阅读、分析、修复全部通过spawn工位执行。Lead读取代码后唯一合法出口=创建Task+spawn工位。"

# 纠正记录注入（407号修复2：compact后纠正效果恢复）
CORRECTIONS_MSG=""
if [ -f ".chanlun/.lead-corrections.log" ]; then
    RECENT_CORRECTIONS=$(tail -5 .chanlun/.lead-corrections.log 2>/dev/null || true)
    if [ -n "$RECENT_CORRECTIONS" ]; then
        CORRECTIONS_MSG=" | ⚠历史纠正: $(echo "$RECENT_CORRECTIONS" | tr '\n' '; ')"
    fi
fi

# ─── 137号正面格式：第一动作前置（562号/编排者裁决 2026-06-23） ───
# 137号：行为执行层规则必须是"正面输出格式"而非否定性禁令。
# 编排者裁决：第一动作必须是 invoke /ceremony 热启动序列，不是手动 git diff 对比。
# 机制强制的双前提（072号）：
#   - 提示前提（本 hook，SessionStart 仅能注入 context，无 tool 可阻断——097号纯化边界）：
#     正面声明第一动作 = 运行 ceremony_scan.py 命令序列。
#   - 机制前提（ceremony-completion-guard.sh Stop hook 检查 1.5，562号）：Lead 跳过 ceremony
#     → 缺常设结构工位 → Stop 被 block，机制性地强制 ceremony-等价行为。两前提共同构成强制。
# 状态快照仅供参考，工位的唯一确定性来源是 ceremony_scan.py（非 LLM 对 git diff 的认知判断）。
FIRST_ACTION="本回合第一个动作=invoke /ceremony 热启动序列：运行 \`python scripts/ceremony_state.py write 1 initial\` → \`python scripts/ceremony_scan.py\`，由 scan 的确定性输出决定本轮工位。禁止用手动 git diff/下方状态快照对比代替 ceremony_scan——快照仅供参考，工位来源是 scan。scan 输出 workstations 后，用 Agent tool（name=工位名 + run_in_background=true，teammate 模式；harness 已演化：单一隐式 team，无需 TeamCreate，team_name 已废弃）并行 spawn。ceremony/热启动完成后 Lead 默认进 /goal 运行协议循环（.claude/commands/goal.md 步骤1-7：评估→scan→spawn→监控→真封→commit→回步骤1，不停在手动 (c) 等 Stop-Guard 推动）——若 .chanlun/goals/events.jsonl 有 active goal（scan 输出 current_goal 非 null 且未 terminated）则 append GOAL_RESUME 续跑；无 active goal 则从 roadmap/中断点推导候选 GOAL_SET（acceptance 项须 falsifiable）后 append 进循环。编排者裁定(2026-06-29)：Lead 默认 goal 驱动持续自主运行，goal 达成/真实矛盾(/escalate)/资源耗尽是仅有的三种合法停止。"

# compact 恢复专用框定：显式声明 summary 的 resume-directly 不覆盖本动作
if [ "$SOURCE" = "compact" ]; then
    HEADER="[Ceremony/compact恢复] 本会话由上下文压缩(compact)恢复。compact summary 的 'Resume directly — do not acknowledge the summary' 不覆盖本 ceremony 序列——${FIRST_ACTION}"
else
    HEADER="[Ceremony/热启动L2] ${FIRST_ACTION}"
fi

# additionalContext = 可执行内容（可靠注入通道，根因修复）
# systemMessage = 用户可见简短提示（advisory）
CTX="${HEADER}

── 状态快照 ──
恢复自:${SESSION_FILE} (${SESSION_TIME}) | 分支:${GIT_BRANCH} | session提交:${SESSION_COMMIT} | 当前:${CURRENT_COMMIT} | 定义变更:${CHANGED}条 | 谱系:${SETTLED}settled/${PENDING}pending | 中断点:${INTERRUPTS}${SWARM_MSG}

${ROLE_BOUNDARY}${CORRECTIONS_MSG}"

emit_json "$CTX" "[Ceremony/${SOURCE:-start}] 状态快照已注入 context，执行 ceremony 差异检查"
