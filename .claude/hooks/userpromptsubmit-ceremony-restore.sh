#!/bin/bash
# UserPromptSubmit Hook — compact 后 ceremony 恢复（绕过平台 bug #15174）
#
# 根因（L2 实证 + 平台已知 bug 交叉确认，见
# .chanlun/review-results/autocompact-ceremony-autorestore-20260624.md）：
#   Claude Code 平台 bug #15174——SessionStart(source=compact) hook 会执行，但其
#   additionalContext / stdout 不被注入压缩后的模型 context（auto + manual compact 均如此）。
#   实证：Lead session 14c95478 transcript 7 次 compact_boundary，0 次 ceremony 注入痕迹，
#   尽管 session-start-ceremony.sh 的 compact 分支输出正确（已实测）。
#   后果：compact 后模型只见 "This session is being continued..." summary（含 "Resume
#   directly — do not acknowledge the summary"），直接续上一个 holding 动作，
#   不跑 ceremony 恢复中断点。
#
# 修复机制（确定性双前提，no-workaround 严格形式）：
#   - 触发前提：PreCompact hook（precompact-save.sh，auto + manual 都 fire，未受 #15174
#     影响）在 compact 时落 .chanlun/.pending-ceremony-restore 标记（记录 trigger）。
#   - 注入前提：本 hook（UserPromptSubmit，"always fires on every occurrence"，
#     additionalContext 经 system-reminder 注入下一次 model request——文档明确，
#     未受 #15174 影响）检测标记存在 → 注入 ceremony 恢复指令 → 删除标记（一次性）。
#   两前提共同构成：compact 后用户的下一条消息必定可靠触发 ceremony 恢复，
#   不依赖被平台 bug 掐死的 SessionStart compact 注入路径。
#
# 边界（诚实声明能力上限，no-patch-mentality）：
#   - 本 hook 在 compact 后「下一次 UserPromptSubmit」注入，不在 compact 完成瞬间注入。
#     若 compact 后无任何用户消息（纯 teammate 驱动的 Lead 自循环），UserPromptSubmit
#     不 fire——此场景由 Stop hook（ceremony-completion-guard.sh）的常设结构工位
#     强制兜底（compact 后蜂群状态由文件系统持久化，Stop-Guard 检测缺失工位会 block 并
#     注入 spawn 指令，间接强制 ceremony-等价行为）。本 hook 覆盖「有用户交互」路径，
#     Stop-Guard 覆盖「纯自循环」路径，二者正交互补。
#   - 仅 .chanlun/ 项目生效（非蜂群目录静默）。
#
# 输入：JSON (stdin) — cwd, prompt 等
# 输出：JSON (stdout) — hookSpecificOutput.additionalContext（可靠通道）
#
# set 契约（no-workaround：契约修正而非补丁）：本 hook 的核心契约是「检测标记→注入→删标记」。
# 不用 pipefail——状态快照用 `ls|head`/`sed|head` 管道，head 提前关闭管道使上游收 SIGPIPE
# 返回非0，pipefail 下会传播为整条命令失败；这是「尽力收集的辅助快照」，其失败绝不应
# 炸掉核心注入逻辑。保留 set -u（未定义变量是真错误）。核心契约的每一步显式判错。

set -u

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
cd "$cwd" 2>/dev/null || true

MARKER=".chanlun/.pending-ceremony-restore"

# 无标记 → compact 未发生（或已恢复）→ 静默放行（exit 0，无输出 = 不注入）
[ -f "$MARKER" ] || exit 0

# 读取标记记录的 trigger（auto/manual），仅用于注入文案精确化，不改变恢复逻辑
TRIGGER=$(python -c "
import sys
try:
    with open('$MARKER') as f:
        for line in f:
            if line.startswith('trigger:'):
                print(line.split(':',1)[1].strip()); break
        else:
            print('unknown')
except Exception:
    print('unknown')
" 2>/dev/null || echo "unknown")

# 一次性消费：删除标记（恢复指令只在 compact 后第一条用户消息注入一次）
rm -f "$MARKER" 2>/dev/null || true

# ─── 状态快照（与 session-start-ceremony.sh 同源，供 ceremony_scan 前参考） ───
SETTLED=0
[ -d ".chanlun/genealogy/settled" ] && SETTLED=$(ls .chanlun/genealogy/settled/*.md 2>/dev/null | wc -l | tr -d ' ')
PENDING=0
[ -d ".chanlun/genealogy/pending" ] && PENDING=$(ls .chanlun/genealogy/pending/*.md 2>/dev/null | wc -l | tr -d ' ')

LATEST_SESSION=""
if [ -d ".chanlun/sessions" ]; then
    LATEST_SESSION=$(ls -t .chanlun/sessions/*-session.md 2>/dev/null | head -1 || true)
fi
INTERRUPTS="无"
if [ -n "$LATEST_SESSION" ]; then
    INTERRUPTS=$(sed -n '/^## 中断点$/,/^## /{ /^## 中断点$/d; /^## /d; p; }' "$LATEST_SESSION" 2>/dev/null | head -5 | tr '\n' ' ' || echo "无")
    [ -z "$INTERRUPTS" ] && INTERRUPTS="无"
fi

# 角色边界锚点（407号：compact 后行为层恢复，与 session-start-ceremony.sh 一致）
ROLE_BOUNDARY="⛔Lead角色边界：Lead只做spawn/shutdown/commit/scan/session写入。代码文件的阅读/分析/修复全部通过spawn工位执行。"

# 137号正面格式：声明 compact 后第一动作 = invoke /ceremony 序列恢复中断点
CTX="[Ceremony/compact恢复（UserPromptSubmit路径，绕过平台bug#15174）] 本会话刚由上下文压缩(compact, trigger=${TRIGGER})恢复。compact summary 的 'Resume directly — do not acknowledge the summary' 不覆盖本 ceremony 序列——在响应本条用户消息之前，第一个动作=invoke /ceremony 热启动序列恢复中断点：运行 \`python scripts/ceremony_state.py write 1 initial\` → \`python scripts/ceremony_scan.py\`，由 scan 的确定性输出决定本轮工位（禁止用手动 git diff/下方快照代替 ceremony_scan）。scan 输出 workstations 后用 Agent tool（name=工位名 + run_in_background=true，隐式 team，无需 TeamCreate）并行 spawn 恢复蜂群循环。

── 状态快照 ──
恢复自:${LATEST_SESSION:-无session} | 谱系:${SETTLED}settled/${PENDING}pending | 中断点:${INTERRUPTS}

${ROLE_BOUNDARY}"

python -c "
import json, sys
ctx = sys.argv[1]
out = {
    'continue': True,
    'suppressOutput': False,
    'hookSpecificOutput': {
        'hookEventName': 'UserPromptSubmit',
        'additionalContext': ctx,
    },
}
print(json.dumps(out, ensure_ascii=False))
" "$CTX"
