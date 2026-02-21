#!/bin/bash
# Stop Hook — 二阶反馈强制（meta-observer guard）
# 016号谱系：规则没有代码强制就不会被执行
# dispatch-dag constraint: meta_observation
#
# 触发：Stop 事件
# 逻辑：检查 .chanlun/.meta-observer-executed 标记是否包含当前 session ID。
#        若未执行，阻止一次并注入二阶观察指令。
# 熔断：仅阻止 1 次（二阶反馈是观察性的，不阻塞关键路径）
#
# 2026-02-21 hotfix:
# 默认改为"自动落标+放行"模式，避免在某些会话里触发
# Invalid `signature` in `thinking` block 的上游错误。
# 如需恢复强制阻断，设置环境变量 META_OBSERVER_GUARD_STRICT=1。
#
# 2026-02-21 087号谱系修复:
# advisory 模式现在产生实际的 systemMessage 提示（D策略要求：hooks 提示 + Lead 认领）
# 原始 hotfix 直接 exit 0 不产生任何输出——这不是 advisory，是完全失效。

set -uo pipefail

input=$(cat)
cwd=$(echo "$input" | python -c "import sys,json; print(json.loads(sys.stdin.read()).get('cwd', '.'))" 2>/dev/null || echo ".")
cd "$cwd" 2>/dev/null || true

MARKER=".chanlun/.meta-observer-executed"
COUNTER=".chanlun/.meta-observer-guard-counter"
STRICT_MODE="${META_OBSERVER_GUARD_STRICT:-0}"

# ─── 熔断：已阻止过 1 次则放行 ───
if [ -f "$COUNTER" ]; then
    rm -f "$COUNTER" 2>/dev/null || true
    exit 0
fi

# ─── 检查当前 session 是否已执行二阶观察 ───
CURRENT_SESSION=""
if [ -d ".chanlun/sessions" ]; then
    CURRENT_SESSION=$(ls -t .chanlun/sessions/*-session.md 2>/dev/null | head -1 | xargs basename 2>/dev/null || true)
fi
[ -z "$CURRENT_SESSION" ] && exit 0

# ─── 默认模式：Advisory — 落标 + 输出提示（087号谱系修复）───
if [ "$STRICT_MODE" != "1" ]; then
    mkdir -p .chanlun 2>/dev/null || true
    echo "$CURRENT_SESSION" > "$MARKER"
    rm -f "$COUNTER" 2>/dev/null || true
    # Advisory 输出：提示 Lead 可选择执行二阶观察（D策略：hooks 提示 + Lead 认领）
    python -c "
import json
print(json.dumps({
    'continue': True,
    'systemMessage': '[meta-observer advisory] 本 session 二阶观察已跳过（STRICT_MODE=0）。如需执行：读取 .claude/agents/meta-observer.md 并对本 session 执行二阶观察（规则触发/违反模式、语法记录候选、元规则一致性）。'
}, ensure_ascii=False))
" 2>/dev/null || true
    exit 0
fi

if [ -f "$MARKER" ]; then
    MARKER_SESSION=$(cat "$MARKER" 2>/dev/null || true)
    if [ "$MARKER_SESSION" = "$CURRENT_SESSION" ]; then
        exit 0
    fi
fi

# ─── 未执行：阻止一次 ───
echo "1" > "$COUNTER"
python -c "
import json, sys
session = sys.argv[1]
print(json.dumps({
    'decision': 'block',
    'reason': f'[meta-observer-guard] 二阶反馈未执行（016号谱系强制）。执行指令：读取 .claude/agents/meta-observer.md，对本 session 执行二阶观察（规则触发/违反模式、语法记录候选、元规则一致性），观察结果写入谱系（type: meta-rule）或确认无新发现，然后写入标记: echo \"{session}\" > .chanlun/.meta-observer-executed'
}, ensure_ascii=False))
" "$CURRENT_SESSION"
