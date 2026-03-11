#!/usr/bin/env python3
"""双重耦合自动化调度器。

在 VPS/OpenClaw 上运行，自动执行：
1. 查询逢亮 daemon swarm 状态（fold 端，daemon 独立持久运行）
2. ceremony_scan 拓扑指标检测（切分端的机器部分）
3. 异常 → escalate 队列（等待人的切分）
4. git sync（双向同步）

前提：daemon 必须已作为持久进程运行：
    cd topological-computation
    python start_fengliang.py --instances 3 --serve --multiproc --load-experiments

用法:
    python scripts/openclaw_scheduler.py              # 单次（查询daemon + scan + escalate）
    python scripts/openclaw_scheduler.py --loop       # 持续循环（适合 systemd）
    python scripts/openclaw_scheduler.py --interval 1800  # 循环间隔秒数（默认30分钟）
    python scripts/openclaw_scheduler.py --dry-run    # 只检测不执行

架构:
    fold 端:  daemon swarm (持久进程, N instances) → perpetual traversal
    切分端:  scheduler → ceremony_scan → topo_indicators → escalate 队列
    人的切分: .chanlun/escalate/ 队列 → 编排者消费
    同步层:  scheduler → git pull/push 双向同步

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
    """Query 逢亮 daemon swarm status via HTTP API.

    Daemon must be running as a persistent process:
      cd topological-computation
      python start_fengliang.py --instances 3 --serve --multiproc --load-experiments

    This function does NOT start the daemon — it queries the running daemon's status.
    If daemon is not running, returns error status.

    The daemon runs perpetually; each cycle we just check its state.
    """
    _log(f"querying daemon status (expecting {instances} instances)...")

    import urllib.request
    import urllib.error

    daemon_url = os.environ.get("FENGLIANG_API", "http://localhost:9765")

    try:
        req = urllib.request.Request(f"{daemon_url}/status")
        with urllib.request.urlopen(req, timeout=10) as resp:
            status = json.loads(resp.read().decode("utf-8"))

        summary = {
            "instances": instances,
            "beta_1": status.get("beta_1"),
            "position": status.get("position"),
            "total_steps": status.get("total_steps", 0),
            "settled_count": status.get("settled_count", 0),
            "sublated_count": status.get("sublated_count", 0),
            "api_status": "ok",
        }
        _log(f"daemon status: β₁={summary['beta_1']}, "
             f"steps={summary['total_steps']}, "
             f"settled={summary['settled_count']}, "
             f"sublated={summary['sublated_count']}")
        return summary

    except urllib.error.URLError as e:
        _log(f"daemon not reachable: {e}")
        return {"error": f"daemon not reachable: {e}", "api_status": "unreachable"}
    except Exception as e:
        _log(f"daemon query failed: {e}")
        return {"error": str(e), "api_status": "error"}


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

    # 2. Query daemon status (daemon runs as persistent process, not started per-cycle)
    daemon_summary = run_daemon(steps=steps, instances=instances)
    result["daemon"] = daemon_summary

    # 3. Git push any daemon-generated changes (traversal events, block topology)
    if not dry_run and daemon_summary.get("api_status") == "ok":
        git_push(f"auto: daemon query β₁={daemon_summary.get('beta_1')}, "
                 f"steps={daemon_summary.get('total_steps')}")

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
