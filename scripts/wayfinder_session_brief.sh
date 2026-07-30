#!/usr/bin/env bash
# UserPromptSubmit hook：用户输入里出现 wayfinder 时，把本仓会话便条注入上下文。
#
# 两个 harness 共用（issue #784）：
#   - Claude Code：.claude/settings.json 的 hooks.UserPromptSubmit
#   - Kimi Code：  ~/.kimi-code/config.toml 顶级 [[hooks]]，event = "UserPromptSubmit"
# 两边都以 JSON 走 stdin 传入，字段名同为 .prompt；stdout 被追加进上下文。
#
# 静默契约：不匹配就 exit 0 且零输出——hook 不该在无关的每一轮里说话。

set -uo pipefail

REPO="/Users/silencehan/Projects/NewChanlun"
BRIEF="$REPO/docs/agents/wayfinder-session-brief.md"

[ -f "$BRIEF" ] || exit 0

payload="$(cat 2>/dev/null || true)"

# 优先按 JSON 取 .prompt；取不到就退回整个 payload 做匹配（harness 换了传参格式也不致哑火）
prompt="$(
  printf '%s' "$payload" | python3 -c '
import sys, json
raw = sys.stdin.read()
try:
    d = json.loads(raw)
    print(d.get("prompt", "") if isinstance(d, dict) else raw)
except Exception:
    print(raw)
' 2>/dev/null || printf '%s' "$payload"
)"

case "$prompt" in
  *wayfinder*|*WAYFINDER*|*Wayfinder*) ;;
  *) exit 0 ;;
esac

cat "$BRIEF"
