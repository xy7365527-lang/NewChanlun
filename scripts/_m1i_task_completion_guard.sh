#!/bin/bash
# m1_i 回测任务完成守护（detached 兜底，解除潜在死锁）
#
# 背景：task 3635505d/2.json（监控 m1_i 回测汇总）owner=None, in_progress。
# 回测进程 82437 是 detached 孤儿（父=launchd），owning session 死活未知。
# 若 owning session 已退出，回测跑完无人标 task completed → Stop-Guard 永久死锁。
#
# 本守护：鲁棒等待 m1_i 回测真正完成（results.json 生成）后标 task completed。
# 门控用文件存在 + pgrep 重探测（吸取 PID 门控脆弱教训，不依赖固定 PID）。
# 幂等：owning session 若活着也标 completed，无冲突。
# 严格：仅当 results.json 真实生成才标 completed，绝不假标。

set -u
TASK="$HOME/.claude/tasks/3635505d-4345-4816-b23d-c676d3a4bb20/2.json"
RESULTS="/Users/silencehan/Projects/NewChanlun/analysis/data_cache/m1_i_rust_backtest_results.json"
RESULTS_MD="/Users/silencehan/Projects/NewChanlun/analysis/m1_i_rust_backtest_results.md"
LOG="/Users/silencehan/Projects/NewChanlun/analysis/logs/_m1i_completion_guard.log"
DEADLINE=$(( $(date +%s) + 5*3600 ))   # 5h 超时保护

log() { echo "[$(date '+%H:%M:%S')] $1" >> "$LOG"; }
log "守护启动：盯 m1_i 回测完成 → 标 task completed"

mark_completed() {
  local reason="$1"
  python3 - "$TASK" "$reason" <<'PY'
import json, sys, time
p, reason = sys.argv[1], sys.argv[2]
try:
    t = json.load(open(p))
except Exception:
    sys.exit(0)  # task 文件不存在/被删，无需处理
new = dict(t)
new["status"] = "completed"
new["owner"] = "_m1i_completion_guard"
new["note"] = f"{reason} (守护标记 {time.strftime('%Y-%m-%d %H:%M')})；结果见 m1_i_rust_backtest_results.json/.md"
json.dump(new, open(p, "w"), ensure_ascii=False, indent=1)
PY
  log "已标 task completed：$reason"
}

while true; do
  # 1) results.json 真实生成 → 回测完成，标 completed
  if [ -f "$RESULTS" ] && [ -s "$RESULTS" ]; then
    # 校验非空 JSON（含至少一个标的）
    if python3 -c "import json,sys; d=json.load(open('$RESULTS')); sys.exit(0 if d else 1)" 2>/dev/null; then
      mark_completed "回测完成，results.json 已生成"
      exit 0
    fi
  fi

  # 2) 重探测 m1_i 进程（鲁棒：进程可能被外部重启换 PID）
  if ! pgrep -f "m1_i_rust_backtest.py" >/dev/null 2>&1; then
    # 进程消失但无 results.json → 等 30s 确认（可能正在写结果）
    sleep 30
    if [ -f "$RESULTS" ] && [ -s "$RESULTS" ]; then
      mark_completed "回测进程结束，results.json 已生成"
      exit 0
    fi
    log "m1_i 进程消失且无 results.json（回测失败/被杀）；不假标 completed，记录后退出"
    python3 - "$TASK" <<'PY'
import json,sys,time
p=sys.argv[1]
try: t=json.load(open(p))
except Exception: sys.exit(0)
new=dict(t)
new["note"]=f"守护观测：m1_i 回测进程于 {time.strftime('%H:%M')} 消失且未生成 results.json（疑失败/被杀）；任务未完成，保持 in_progress 待人工裁决"
json.dump(new,open(p,"w"),ensure_ascii=False,indent=1)
PY
    exit 1
  fi

  # 3) 超时保护
  if [ "$(date +%s)" -gt "$DEADLINE" ]; then
    log "5h 超时，守护退出（回测仍在跑，任务保持 in_progress）"
    exit 2
  fi

  sleep 60
done
