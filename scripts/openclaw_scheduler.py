#!/usr/bin/env python3
"""双重耦合自动化调度器。

在 VPS/OpenClaw 上运行，自动执行：
1. 逢亮 daemon swarm 穿越（fold 端，多实例并行）
2. ceremony_scan 拓扑指标检测（切分端的机器部分）
3. 异常 → escalate 队列（等待人的切分）
4. git sync（双向同步）

用法:
    python scripts/openclaw_scheduler.py              # 单次（3×1000步，适合 cron）
    python scripts/openclaw_scheduler.py --loop       # 持续循环（适合 systemd）
    python scripts/openclaw_scheduler.py --instances 5 --steps 2000  # 5实例×2000步
    python scripts/openclaw_scheduler.py --interval 1800  # 循环间隔秒数（默认30分钟）
    python scripts/openclaw_scheduler.py --dry-run    # 只检测不执行

架构:
    fold 端:  daemon swarm (N instances × M steps) → traversal-events.jsonl → git push
    切分端:  ceremony_scan.py → topo_indicators → escalate 队列
    人的切分: .chanlun/escalate/ 队列 → 编排者消费

谱系依据: 419号(fold/切分对偶性), 421号(SUBLATED=Aufhebung,切分⊂SUBLATED)
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
TOPO_DIR = REPO_ROOT / "topological-computation"
ESCALATE_DIR = REPO_ROOT / ".chanlun" / "escalate"
SCHEDULER_LOG = REPO_ROOT / "tmp" / "openclaw-scheduler.log"


def _log(msg: str) -> None:
    """Log with timestamp."""
    ts = datetime.now(timezone.utc).isoformat(timespec="seconds")
    line = f"[{ts}] {msg}"
    print(line, flush=True)
    try:
        SCHEDULER_LOG.parent.mkdir(parents=True, exist_ok=True)
        with open(SCHEDULER_LOG, "a", encoding="utf-8") as f:
            f.write(line + "\n")
    except OSError:
        pass


def _run_cmd(cmd: list[str], cwd: Path | None = None,
             timeout: int = 600) -> tuple[int, str, str]:
    """Run command, return (returncode, stdout, stderr)."""
    try:
        r = subprocess.run(
            cmd, cwd=cwd, capture_output=True, text=True, timeout=timeout
        )
        return r.returncode, r.stdout, r.stderr
    except subprocess.TimeoutExpired:
        return -1, "", "timeout"
    except Exception as e:
        return -1, "", str(e)


def git_pull() -> bool:
    """Pull latest changes from remote."""
    _log("git pull...")
    rc, out, err = _run_cmd(["git", "pull", "--rebase"], cwd=REPO_ROOT)
    if rc == 0:
        _log(f"git pull ok: {out.strip()}")
        return True
    _log(f"git pull failed: {err.strip()}")
    return False


def git_push(message: str) -> bool:
    """Commit and push changes."""
    # Stage known output files
    files_to_stage = [
        ".chanlun/",
        "topological-computation/.chanlun/",
        "tmp/brn-level-report.json",
    ]
    for f in files_to_stage:
        _run_cmd(["git", "add", f], cwd=REPO_ROOT)

    # Check if there are changes to commit
    rc, out, _ = _run_cmd(["git", "diff", "--cached", "--stat"], cwd=REPO_ROOT)
    if not out.strip():
        _log("nothing to commit")
        return True

    _log(f"committing: {message}")
    rc, out, err = _run_cmd(
        ["git", "commit", "-m", message], cwd=REPO_ROOT
    )
    if rc != 0:
        _log(f"commit failed: {err.strip()}")
        return False

    rc, out, err = _run_cmd(["git", "push"], cwd=REPO_ROOT)
    if rc == 0:
        _log("push ok")
        return True
    _log(f"push failed: {err.strip()}, trying rebase...")
    _run_cmd(["git", "pull", "--rebase"], cwd=REPO_ROOT)
    rc, out, err = _run_cmd(["git", "push"], cwd=REPO_ROOT)
    if rc == 0:
        _log("push ok after rebase")
        return True
    _log(f"push failed after rebase: {err.strip()}")
    return False


def run_daemon(steps: int = 1000, instances: int = 3) -> dict:
    """Run 逢亮 daemon swarm (multi-instance).

    Uses start_fengliang.py for multi-instance traversal.
    Each instance has different hash seed → traversal diversity.
    Instance 0 exposes HTTP/WS API, others are pure traversal.

    steps: 每实例穿越步数（默认1000——2683顶点图需要足够步数覆盖）
    instances: 并行实例数（默认3，与 start_fengliang.py 一致）
    """
    _log(f"starting daemon swarm: {instances} instances × {steps} steps...")

    # Multi-instance wrapper: starts N instances, each runs max_steps, collects summary
    wrapper = f"""
import sys, os, json, time
os.chdir(r'{TOPO_DIR}')
sys.path.insert(0, r'{TOPO_DIR}')

from daemon import TopologicalDaemon
import threading

results = {{}}
lock = threading.Lock()

def run_instance(instance_idx, seed):
    \"\"\"Run one daemon instance.\"\"\"
    try:
        d = TopologicalDaemon()
        d.load_experiments()
        # Different hash seed per instance for traversal diversity
        if d.engine:
            import hashlib
            d.engine._rng_seed = seed
        d.run(max_steps={steps})
        summary = {{
            'beta_1': d.engine.beta_1 if d.engine else None,
            'position': d.engine.position if d.engine else None,
            'settled_count': len(d.engine.settled_cycles) if d.engine else 0,
            'sublated_count': sum(
                1 for sc in (d.engine.settled_cycles if d.engine else [])
                if getattr(sc, 'status', 'active') == 'sublated'
            ),
        }}
    except Exception as e:
        summary = {{'error': str(e)}}
    with lock:
        results[f'instance_{{instance_idx}}'] = summary

seeds = [42, 137, 271, 409, 547, 683, 821, 953, 1087, 1223]
threads = []
for i in range({instances}):
    t = threading.Thread(target=run_instance, args=(i, seeds[i % len(seeds)]))
    threads.append(t)
    t.start()

for t in threads:
    t.join()

# Aggregate
total_settled = max(r.get('settled_count', 0) for r in results.values())
total_sublated = max(r.get('sublated_count', 0) for r in results.values())
# beta_1 should converge across instances (shared graph)
beta_1_vals = [r.get('beta_1') for r in results.values() if r.get('beta_1') is not None]
final_beta_1 = beta_1_vals[0] if beta_1_vals else None

agg = {{
    'instances': {instances},
    'steps_per_instance': {steps},
    'total_steps': {instances} * {steps},
    'beta_1': final_beta_1,
    'settled_count': total_settled,
    'sublated_count': total_sublated,
    'per_instance': results,
}}
print('DAEMON_SUMMARY:' + json.dumps(agg))
"""
    rc, out, err = _run_cmd(
        [sys.executable, "-c", wrapper],
        cwd=TOPO_DIR,
        timeout=3600,  # 1h max for multi-instance
    )

    # Parse summary from output
    summary = {"steps": steps, "instances": instances, "returncode": rc}
    for line in out.splitlines():
        if line.startswith("DAEMON_SUMMARY:"):
            try:
                summary.update(json.loads(line[len("DAEMON_SUMMARY:"):]))
            except json.JSONDecodeError:
                pass

    if rc == 0:
        _log(f"daemon swarm completed: {instances}×{steps} steps, "
             f"β₁={summary.get('beta_1')}, "
             f"settled={summary.get('settled_count')}, "
             f"sublated={summary.get('sublated_count')}")
    else:
        _log(f"daemon swarm failed (rc={rc}): {err[:200]}")

    if err:
        err_path = REPO_ROOT / "tmp" / "daemon-stderr-latest.log"
        err_path.parent.mkdir(parents=True, exist_ok=True)
        with open(err_path, "w", encoding="utf-8") as f:
            f.write(err)

    return summary


def run_ceremony_scan() -> dict:
    """Run ceremony_scan and parse topo_indicators."""
    _log("running ceremony_scan...")
    rc, out, err = _run_cmd(
        [sys.executable, "scripts/ceremony_scan.py"],
        cwd=REPO_ROOT,
        timeout=300,
    )
    if rc != 0:
        _log(f"ceremony_scan failed: {err[:200]}")
        return {"error": err[:500]}

    try:
        scan = json.loads(out)
    except json.JSONDecodeError:
        _log("ceremony_scan output not valid JSON")
        return {"error": "invalid JSON"}

    topo = scan.get("topo_indicators", {})
    workstations = scan.get("workstations", [])

    _log(f"scan result: {len(workstations)} workstations, "
         f"broken_chains={len(topo.get('broken_dependency_chains', []))}, "
         f"unstable_settled={len(topo.get('unstable_settled', []))}")

    return scan


def process_anomalies(scan: dict, dry_run: bool = False) -> list[dict]:
    """Process topo_indicators anomalies → escalate queue."""
    topo = scan.get("topo_indicators", {})
    escalate_items = []

    # Check for high-priority anomalies
    broken = topo.get("broken_dependency_chains", [])
    unstable = topo.get("unstable_settled", [])
    hotzones = [h for h in topo.get("hotzone_residue_density", [])
                if h.get("growth_rate", 0) > 2.0]  # Only very hot zones

    if broken:
        escalate_items.append({
            "level": "L1",  # Auto-closeable
            "type": "broken_dependency_chains",
            "count": len(broken),
            "top_5": [b["block"] for b in broken[:5]],
            "action": "auto-diagnose",
        })

    if unstable:
        escalate_items.append({
            "level": "L2",  # Needs theory review
            "type": "unstable_settled",
            "count": len(unstable),
            "ids": [u["genealogy_id"] for u in unstable],
            "action": "review-premise-validity",
        })

    if hotzones:
        escalate_items.append({
            "level": "L2",
            "type": "residue_hotzone",
            "count": len(hotzones),
            "blocks": [h["block"] for h in hotzones[:3]],
            "action": "investigate-accumulation",
        })

    # Write to escalate queue
    if escalate_items and not dry_run:
        ESCALATE_DIR.mkdir(parents=True, exist_ok=True)
        ts = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S")
        escalate_file = ESCALATE_DIR / f"cycle-{ts}.json"
        with open(escalate_file, "w", encoding="utf-8") as f:
            json.dump({
                "timestamp": ts,
                "items": escalate_items,
                "scan_head": scan.get("head", "unknown"),
            }, f, ensure_ascii=False, indent=2)
        _log(f"wrote {len(escalate_items)} items to {escalate_file.name}")

    return escalate_items


def run_cycle(steps: int = 1000, instances: int = 3, dry_run: bool = False) -> dict:
    """Run one full fold→scan→escalate cycle."""
    cycle_start = time.time()
    result = {"timestamp": datetime.now(timezone.utc).isoformat()}

    # 1. Git pull (sync from remote)
    if not dry_run:
        git_pull()

    # 2. Run daemon swarm (fold 端, multi-instance)
    if not dry_run:
        daemon_summary = run_daemon(steps=steps, instances=instances)
        result["daemon"] = daemon_summary
    else:
        _log("[dry-run] would run daemon")
        result["daemon"] = {"dry_run": True}

    # 3. Git push daemon results
    if not dry_run:
        total = daemon_summary.get('total_steps', steps)
        git_push(f"auto: daemon {instances}×{steps}={total} steps, β₁={daemon_summary.get('beta_1')}")

    # 4. Run ceremony_scan (切分端 - 机器部分)
    scan = run_ceremony_scan()
    result["scan"] = {
        "workstations": len(scan.get("workstations", [])),
        "settled": scan.get("settled", 0),
        "head": scan.get("head", "unknown"),
    }

    # 5. Process anomalies → escalate queue
    escalate_items = process_anomalies(scan, dry_run=dry_run)
    result["escalate"] = escalate_items

    # 6. Git push scan results + escalate
    if not dry_run and escalate_items:
        git_push(f"auto: ceremony scan, {len(escalate_items)} escalate items")

    elapsed = time.time() - cycle_start
    result["elapsed_seconds"] = round(elapsed, 1)
    _log(f"cycle complete in {elapsed:.1f}s")

    return result


def main():
    parser = argparse.ArgumentParser(
        description="双重耦合自动化调度器 (OpenClaw/VPS)"
    )
    parser.add_argument("--loop", action="store_true",
                        help="持续循环模式（适合 systemd service）")
    parser.add_argument("--interval", type=int, default=1800,
                        help="循环间隔秒数（默认1800=30分钟）")
    parser.add_argument("--steps", type=int, default=1000,
                        help="每实例穿越步数（默认1000）")
    parser.add_argument("--instances", type=int, default=3,
                        help="并行 daemon 实例数（默认3）")
    parser.add_argument("--dry-run", action="store_true",
                        help="只检测不执行 daemon")
    args = parser.parse_args()

    _log(f"=== OpenClaw Scheduler started (instances={args.instances}, "
         f"steps={args.steps}, interval={args.interval}s, loop={args.loop}) ===")

    if args.loop:
        cycle_num = 0
        while True:
            cycle_num += 1
            _log(f"--- cycle {cycle_num} ---")
            try:
                result = run_cycle(steps=args.steps, instances=args.instances,
                                   dry_run=args.dry_run)
                _log(f"cycle {cycle_num} result: "
                     f"daemon={result.get('daemon', {}).get('beta_1', '?')}, "
                     f"escalate={len(result.get('escalate', []))}")
            except Exception as e:
                _log(f"cycle {cycle_num} error: {e}")
            _log(f"sleeping {args.interval}s until next cycle...")
            time.sleep(args.interval)
    else:
        result = run_cycle(steps=args.steps, instances=args.instances,
                           dry_run=args.dry_run)
        print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
