#!/usr/bin/env bash
# watcher.sh —— sandcastle 工蜂状态滚报（宿主侧零模型；可靠性不依赖端点）
# 轮询 .sandcastle/logs/workers.jsonl + 循环日志，把最新状态写入
# .sandcastle/logs/STATUS.md 并同步回写 roster（AGENTS.md 登记面）。
# 用法：nohup bash .sandcastle/watcher.sh &  （或挂 launchd/cron 每 2 分钟跑一轮）
set -euo pipefail
REPO=/Users/silencehan/Projects/NewChanlun
cd "$REPO"
REG=.sandcastle/logs/workers.jsonl
STATUS=.sandcastle/logs/STATUS.md
ROSTER=.chanlun/agent-roster-2026-08-16.md
{
  echo "# Sandcastle 工蜂状态（watcher 滚报，$(date '+%Y-%m-%d %H:%M:%S')）"
  echo
  if [ -f "$REG" ]; then
    # 每票取最新事件
    python3 - "$REG" <<'PYEOF'
import json, sys
latest = {}
for l in open(sys.argv[1]):
    if not l.strip(): continue
    e = json.loads(l)
    latest[(e.get("ticket"), e.get("phase"))] = e
for (t, p), e in sorted(latest.items()):
    print(f"- #**{t}** [{p}] {e.get('status')} @ {e.get('ts','')[:19]}")
PYEOF
  else
    echo "- （尚无事件）"
  fi
  echo
  echo "---"
  tail -4 .sandcastle/logs/main-loop-20260817.log 2>/dev/null | sed 's/^/  /' || true
} > "$STATUS"
# roster 同步：把 workers.jsonl 里最新状态回写对应行
if [ -f "$REG" ]; then
  python3 - "$REG" "$ROSTER" <<'PYEOF'
import json, sys, re
reg = {}
for l in open(sys.argv[1]):
    if not l.strip(): continue
    e = json.loads(l)
    reg[e["ticket"]] = f"{e.get('phase')}:{e.get('status')}"
roster = open(sys.argv[2]).read()
for t, st in reg.items():
    pat = re.compile(rf"(\| 沙盒工蜂 #{re.escape(str(t))} .*? \| )running（2026-08-1[67] 派生）( \|)")
    roster, n = pat.subn(rf"\g<1>{st}\g<2>", roster)
open(sys.argv[2], "w").write(roster)
PYEOF
fi
echo "watcher 一轮完成 → $STATUS"
