#!/usr/bin/env bash
# PreToolUse Hook — Agent Team 强制（095/096号谱系；562号 + 编排者裁决 2026-06-23 harness 演化更新）
#
# 触发：PreToolUse on "Agent" tool（matcher 在 .claude/settings.json）
#
# ─── harness 真实机制（2026-06-23 实测核实，不照搬 stale TeamCreate 模型） ───
#   1. teammate spawn 工具 = `Agent`（不是已废弃的 `Task` agent-spawn 工具）。
#      旧 hook matcher="Task" 匹配一个不再存在的 agent-spawn 工具 → 完全失效，Lead 用
#      Agent tool spawn 绕过了它。本次修复改 matcher="Agent" + 适配 Agent tool 的参数。
#   2. 单一隐式 team（team_name 已 deprecated/ignored）——无需 TeamCreate，flat roster。
#   3. teammate 模式信号 = `name` 参数存在（实测 harness 报错原文："To spawn a subagent
#      instead, omit the `name` parameter."）。name 存在 = 可寻址 peer teammate（入隐式 team
#      共享 inbox）；name 省略 = 孤立 subagent（只能回报 spawner，非 peer）。
#   4. 实测核实：**teammate 不能 spawn teammate**（harness 报错："Teammates cannot spawn
#      other teammates — the team roster is flat."）。只有 LEAD（main session）能 spawn
#      teammate；teammate 的 Agent 调用只能产出 subagent（必须省略 name）。
#      ⇒ 编排者(c)裁决（2026-06-23）已结算此 gap：teammate 要递归 → **TaskCreate 子任务**
#        （metadata.agent_type 标 subagent_type）→ Lead 循环 spawn 该类型 teammate。递归载体从
#        team 拓扑（roster）改为任务拓扑（TaskList DAG）。这不是未决 gap，是已结算的递归实现
#        （见 skill sub-swarm-ceremony + dispatch-dag fractal_template）。本 hook 对 teammate 的
#        无-name Agent 调用 fail-open + advisory，并显式导向 (c) 路径（TaskCreate）。
#
# ─── 强制逻辑（fail-open：仅在 POSITIVE 确认违规时 block，避免 032号死锁/全蜂群停摆） ───
#   - tool_name != "Agent" → 放行。
#   - subagent_type == "Explore"（只读搜索 subagent）→ 不强制 teammate 模式（advisory：096号
#     优先 Glob/Grep 或 spawn 命名搜索 teammate），放行。
#   - 业务/结构工位 spawn（非 Explore）：
#     * 确认调用者是 LEAD（session_id == 某 team config 的 leadSessionId）且缺 name →
#       BLOCK：Lead 必须 spawn 命名 teammate（095/096 + 562），匿名 subagent 对工位非法。
#     * teammate 调用 / 无法确认 lead 身份 → 不 block name（harness 禁止 teammate 用 name，
#       block 会死锁）；仅做两基因 + 递归判断块 advisory。
#   - 两基因（073a号+274号：topo_address, parent_callback）+ 递归判断块缺失 → advisory（不 block）。
#
# 095号：严格使用 Agent Team，不使用孤立 subagent
# 096号：无例外（在当前 harness 的有效边界内——见上 spec-execution gap）
# 016号：规则没有代码强制就不会被执行
# 073a号+274号：spawn 两基因——topo_address, parent_callback（depth_budget 已废除）
# 562号：结构工位是 teammate（075 扬弃）；teammate spawn = 命名 Agent

set -uo pipefail

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

INPUT=$(timeout 3 cat 2>/dev/null || echo "{}")

TOOL_NAME=$(echo "$INPUT" | python -c "
import sys, json
try:
    data = json.loads(sys.stdin.read())
    print(data.get('tool_name', ''))
except Exception:
    print('')
" 2>/dev/null || echo "")

# 只拦截 Agent 工具（harness 真实 teammate spawn 工具）
if [ "$TOOL_NAME" != "Agent" ]; then
    exit 0
fi

TEAMS_DIR="$HOME/.claude/teams"

# INPUT/TEAMS_DIR 经环境变量传递（脚本经 heredoc 占用 stdin，避免双 stdin 重定向冲突）。
# 直接调用 "$PYTHON_BIN"（非 python() 函数）——env 前缀对外部命令保证导出到子进程环境。
HOOK_INPUT="$INPUT" HOOK_TEAMS_DIR="$TEAMS_DIR" "$PYTHON_BIN" <<'PYEOF'
import sys, json, os

teams_dir = os.environ.get('HOOK_TEAMS_DIR', '') or ''
try:
    data = json.loads(os.environ.get('HOOK_INPUT', '{}') or '{}')
except Exception:
    data = {}
tool_input = data.get('tool_input', {}) or {}
session_id = data.get('session_id', '') or ''

subagent_type = (tool_input.get('subagent_type', '') or '')
name = (tool_input.get('name', '') or '')
prompt = (tool_input.get('prompt', '') or tool_input.get('description', '') or '')

def emit_block(reason):
    print(json.dumps({
        'hookSpecificOutput': {
            'hookEventName': 'PreToolUse',
            'permissionDecision': 'deny',
            'permissionDecisionReason': reason,
        }
    }, ensure_ascii=False))

def emit_advisory(messages):
    if messages:
        print(json.dumps({'systemMessage': ' | '.join(messages)}, ensure_ascii=False))

# --- Explore 只读搜索 subagent：不强制 teammate 模式（096号 advisory，放行） ---
if subagent_type == 'Explore':
    emit_advisory([
        '[096号] Agent(Explore) 是孤立搜索 subagent。优先：(1) 简单搜索 → 直接 Glob/Grep/Read；'
        '(2) 多轮自主搜索且需 peer 协作 → 由 LEAD spawn 命名搜索 teammate（name=…）。'
    ])
    sys.exit(0)

# --- 判定调用者是否 LEAD（session_id == 某 team config 的 leadSessionId） ---
is_lead = False
if session_id and os.path.isdir(teams_dir):
    for nm in sorted(os.listdir(teams_dir)):
        cfg = os.path.join(teams_dir, nm, 'config.json')
        if not os.path.isfile(cfg):
            continue
        try:
            with open(cfg) as f:
                d = json.load(f)
        except Exception:
            continue
        if (d.get('leadSessionId', '') or '') == session_id:
            is_lead = True
            break

# --- 硬 block：LEAD spawn 工位但缺 name（= 孤立 subagent，对工位非法） ---
# fail-open：仅在 POSITIVE 确认 is_lead 时 block；无法确认 lead 身份（teammate 或 config 未就绪）
# 不 block name——teammate 用 name 会被 harness 拒绝，block 会死锁（no-workaround 上浮的 gap）。
if is_lead and not name:
    emit_block(
        '[095/096+562号 teammate 模式强制] Lead 的工位 spawn 缺少 name 参数 = 孤立 subagent。'
        'harness 实测：name 存在 = 可寻址 peer teammate（入隐式 team 共享 inbox）；name 省略 = 孤立 subagent。'
        '结构/业务工位必须是 teammate（562号扬弃075——结构=teammate）。'
        '请加 name="<工位名>" + run_in_background=true（teammate 模式）。'
        '单一隐式 team：无需 TeamCreate，team_name 已废弃，不要再传。'
    )
    sys.exit(0)

# --- advisory：两基因 + 递归判断块（lead 与 teammate 的工位 spawn 都应携带） ---
# 137号机制化边界（090号声明精度）：两基因/递归判断块只能存活于 prompt 自由文本——
#   harness Agent 工具无结构化基因字段（只有 prompt/name/subagent_type 等），故 hook 仅能
#   启发式 substring 检测（'topo_address' in prompt），检的是字符串存在性，**非语义有效性**
#   （含未填充占位符 "<swarm/工位名>" 的 template 也会通过）。启发式 block 会误杀同义表述 →
#   违反本 hook 的 fail-open 不变量（032号：仅 POSITIVE 结构性违规才 block）。故基因保持 advisory。
#   ⚠ 诚实声明：基因无 hard structural 机制化路径（harness 无结构化字段）。实际形式 =
#   present-by-instruction（bootstrap 注入 template）+ advisory-detected（本 hook 字符串检测），
#   **非** present-by-construction（template 含占位符，不保证 Lead 填充语义有效值）。
messages = []
missing_genes = []
if 'topo_address' not in prompt.lower() and '拓扑坐标' not in prompt:
    missing_genes.append('topo_address')
# 274号废除 depth_budget——递归终止由原子性和不动点决定，不由计数器决定
if 'parent_callback' not in prompt.lower() and '父节点回调' not in prompt:
    missing_genes.append('parent_callback')
if missing_genes:
    messages.append(
        '[073a号+274号 spawn 两基因] Agent prompt 缺少基因: ' + ', '.join(missing_genes) + '。'
        '每个衍生节点须携带 topo_address（拓扑坐标）, parent_callback（父节点回调）。'
    )

has_recursion_block = ('递归判断' in prompt or 'sub-swarm-ceremony' in prompt.lower())
if not has_recursion_block:
    messages.append(
        '[递归判断缺失] Agent prompt 未含递归判断块（原则15：真递归是默认模式）。'
    )

# --- (c) 路径 advisory：teammate 的无-name 非-Explore Agent 调用 = 应改为 TaskCreate 子任务 ---
# (c)裁决（2026-06-23 编排者）：teammate 识别可分解子工作 → TaskCreate 子任务（Lead spawn），
#   而非 Agent-spawn。harness flat roster：teammate 不能 spawn teammate；teammate 的 Agent(无name)
#   只产孤立 subagent（违 096 号），除非是只读 Explore 搜索（已在上方豁免）。
if (not is_lead) and (not name):
    messages.append(
        '[(c)裁决 递归路径] 你（非 Lead）的 Agent(无 name) 只产孤立 subagent（违 096号）。'
        '要递归请 TaskCreate 子任务（在 metadata.agent_type 标 subagent_type）→ Lead 循环 spawn 该类型 teammate。'
        'harness 实测 teammate 不能 spawn teammate（flat roster）——递归载体是任务拓扑（TaskList DAG），'
        '非 team 拓扑（见 skill sub-swarm-ceremony，(c)裁决已结算，非未决 gap）。'
        '若确需只读搜索 subagent，用 subagent_type="Explore"。'
    )

emit_advisory(messages)
sys.exit(0)
PYEOF
