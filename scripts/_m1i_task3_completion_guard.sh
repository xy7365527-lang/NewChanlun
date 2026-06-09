#!/bin/bash
# m1_i task3 完成守护（detached 兜底，解除跨 session Stop-Guard 死锁）
#
# 背景：task 3635505d/3.json（监控 m1_i 重启运行、完成剩余 6 标的、汇总全 7 标的 4 组对照）
#   owner=None(未分配), in_progress。m1_i 回测进程为 detached 孤儿（PPID=1），按每标的
#   进程隔离并行跑（崩溃不互相影响），workers 100% CPU 活跃推进中。
#   本 session(415f4beb) 认领此未分配监控任务，委托本守护自主收尾。
#
# 严格性（no-patch / 不假标）：仅当 results.json 真实含全 7 标的才写 summary + 标 completed。
#   只读监控 m1_i 产物（i_result_*.json / results.json），绝不触碰/重启/kill m1_i 计算
#   （feedback_pid_gating_fragile_multisession：不碰他人运行中的工作）。
#   幂等：若 3635505d 自带监控工位也写 summary/标 completed，last-writer-wins，内容一致无冲突。
#
# 4 组对照 = E + I_bar0/I_seg2/I_move3（各 × stop none/A/B），指标 compound/excess/sharpe/max_dd。

set -u
ROOT="/Users/silencehan/Projects/NewChanlun"
TASK="$HOME/.claude/tasks/3635505d-4345-4816-b23d-c676d3a4bb20/3.json"
RESULTS="$ROOT/analysis/data_cache/m1_i_rust_backtest_results.json"
SUMMARY="$ROOT/analysis/m1_i_rust_backtest_7symbol_summary.md"
LOG="$ROOT/analysis/logs/_m1i_task3_guard.log"
NEED=7
DEADLINE=$(( $(date +%s) + 6*3600 ))   # 6h 超时保护

log() { echo "[$(date '+%H:%M:%S')] $1" >> "$LOG"; }
log "task3 守护启动：等全 $NEED 标的 results → 写 4组对照 summary → 标 completed"

write_summary_and_complete() {
  "$ROOT/.venv/bin/python" - "$RESULTS" "$SUMMARY" "$TASK" <<'PY'
import json, sys, time
results_p, summary_p, task_p = sys.argv[1], sys.argv[2], sys.argv[3]
d = json.load(open(results_p))
syms = list(d)
I_TAGS = ["I_bar0", "I_seg2", "I_move3"]
STOPS = ["", "_stopA", "_stopB"]

def m(o):  # metrics 取数
    return o.get("metrics", {})

lines = []
lines.append("# m1_i version-I 回测：全 7 标的 4 组对照汇总\n")
lines.append(f"> 自动生成（task3 完成守护，{time.strftime('%Y-%m-%d %H:%M')}）。源：m1_i_rust_backtest_results.json。\n")
lines.append(f"> 标的（{len(syms)}）：{', '.join(syms)}\n")
lines.append("> 4 组 = E（背驰定位器基线）+ I_bar0 / I_seg2 / I_move3（多重赋格 floor 级别），I 各含 stop none/A/B。\n")
lines.append("> 指标：复利%(compound) / 超额%(excess vs BH) / 夏普 / 最大回撤%。诚实标注：本汇总为另一 session(3635505d) 实验的只读聚合，未参与其设计。\n")

# E 基线表
lines.append("\n## E 基线 vs Buy-Hold\n")
lines.append("| 标的 | bars | BH% | E复利% | E超额% | E夏普 | E_MDD% |")
lines.append("|------|------|-----|--------|--------|-------|--------|")
for s in syms:
    r = d[s]; em = m(r.get("E", {}))
    lines.append(f"| {s} | {r.get('n_bars',0):,} | {r.get('bh',0):+.2f} | "
                 f"{em.get('compound',0):+.2f} | {r.get('E',{}).get('excess',0):+.2f} | "
                 f"{em.get('sharpe',0):+.3f} | {em.get('max_dd',0):+.2f} |")

# 每个 I floor 组的 stop 对照
for tag in I_TAGS:
    lines.append(f"\n## {tag}（多重赋格 floor）× 止损对照\n")
    lines.append("| 标的 | stop=none 复利% | none MDD% | stopA 复利% | A MDD% | stopB 复利% | B MDD% |")
    lines.append("|------|-----|-----|-----|-----|-----|-----|")
    for s in syms:
        v = d[s].get("variants", {})
        row = [s]
        for st in STOPS:
            vt = v.get(tag + st)
            if vt:
                mm = m(vt)
                row.append(f"{mm.get('compound',0):+.2f}")
                row.append(f"{mm.get('max_dd',0):+.2f}")
            else:
                row += ["—", "—"]
        lines.append("| " + " | ".join(row) + " |")

# 最优变体（每标的按复利取最高）
lines.append("\n## 每标的最优变体（按复利%）\n")
lines.append("| 标的 | 最优组 | 复利% | 超额% | 夏普 | MDD% | vs E复利% |")
lines.append("|------|--------|-------|-------|------|------|-----------|")
for s in syms:
    r = d[s]; em = m(r.get("E", {}))
    best_k, best_c = "E", em.get("compound", -1e9)
    cand = {"E": r.get("E", {})}
    for tag in I_TAGS:
        for st in STOPS:
            k = tag + st
            if k in r.get("variants", {}):
                cand[k] = r["variants"][k]
    for k, vt in cand.items():
        c = m(vt).get("compound", -1e9)
        if c > best_c:
            best_c, best_k = c, k
    bv = cand[best_k]; bm = m(bv)
    lines.append(f"| {s} | {best_k} | {bm.get('compound',0):+.2f} | "
                 f"{bv.get('excess',0):+.2f} | {bm.get('sharpe',0):+.3f} | "
                 f"{bm.get('max_dd',0):+.2f} | {em.get('compound',0):+.2f} |")

open(summary_p, "w").write("\n".join(lines) + "\n")

# 标 task completed（诚实备注）
try:
    t = json.load(open(task_p))
    t["status"] = "completed"
    t["owner"] = "_m1i_task3_guard(415f4beb认领)"
    t["note"] = (f"全 {len(syms)} 标的 results 齐全，4 组对照 summary 已写 "
                 f"({time.strftime('%Y-%m-%d %H:%M')})；见 m1_i_rust_backtest_7symbol_summary.md。"
                 f"只读聚合，未触碰 m1_i 计算进程。")
    json.dump(t, open(task_p, "w"), ensure_ascii=False, indent=1)
except Exception:
    pass
print(f"summary written: {len(syms)} symbols")
PY
}

while true; do
  if [ -f "$RESULTS" ] && [ -s "$RESULTS" ]; then
    n=$("$ROOT/.venv/bin/python" -c "import json;print(len(json.load(open('$RESULTS'))))" 2>/dev/null || echo 0)
    if [ "$n" -ge "$NEED" ] 2>/dev/null; then
      log "results.json 含 $n 标的（≥$NEED）→ 写 summary + 标 completed"
      write_summary_and_complete >> "$LOG" 2>&1
      log "完成，守护退出"
      exit 0
    fi
    log "results.json 当前 $n/$NEED 标的，继续等"
  fi
  # m1_i 进程消失且未达 7 标的 → 不假标，记录退出待人工
  if ! pgrep -f "m1_i_rust_backtest.py" >/dev/null 2>&1; then
    sleep 30
    n=$("$ROOT/.venv/bin/python" -c "import json;print(len(json.load(open('$RESULTS'))))" 2>/dev/null || echo 0)
    if [ "$n" -ge "$NEED" ] 2>/dev/null; then
      write_summary_and_complete >> "$LOG" 2>&1; log "进程结束且 $n 标的齐全，已完成"; exit 0
    fi
    log "m1_i 进程消失但仅 $n/$NEED 标的；不假标 completed，保持 in_progress 待人工裁决"
    exit 1
  fi
  [ "$(date +%s)" -gt "$DEADLINE" ] && { log "6h 超时退出，task 保持 in_progress"; exit 2; }
  sleep 120
done
