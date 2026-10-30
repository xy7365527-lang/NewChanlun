#!/usr/bin/env python3
"""#1419：固定 g77 诊断副本上的同候选 33W A/B，35 分钟共用帽。"""
import json
import os
from pathlib import Path
import shutil
import statistics
import subprocess
import sys
import time

root = Path(__file__).resolve().parents[2]
out = root / "g77-evidence"
out.mkdir(exist_ok=True)
deadline = int(os.environ["G77_STARTED"]) + 35 * 60 - 120
exe = Path(sys.argv[1]).resolve()
source = out / "extracted-original/w2-runtime"
assert (source / "source-head.txt").read_text().strip() == "ca887d965a8a30848c4521b5684ae2b046ca5698"
assert (source / "run-id.txt").read_text().strip() == "41005347604"
summary = {"status": "running", "cases": [], "pairs": [], "successes": 0,
           "errors": 0, "timeouts": 0, "max_measured_pairs": 5,
           "deadline_epoch": deadline, "timing_scope": "Runtime.commit; trace excluded from measured series",
           "source_head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()}

def save():
    summary["remaining_seconds"] = deadline - time.time()
    path = out / "summary.json"
    temp = path.with_suffix(".tmp")
    temp.write_text(json.dumps(summary, ensure_ascii=False, indent=2) + "\n")
    temp.replace(path)

def command_output(command):
    result = subprocess.run(command, capture_output=True, text=True)
    return {"command": command, "returncode": result.returncode,
            "stdout": result.stdout, "stderr": result.stderr}

def machine():
    return {name: command_output(command) for name, command in [
        ("uname", ["uname", "-a"]), ("cores", ["nproc"]), ("uptime", ["uptime"]),
        ("load", ["cat", "/proc/loadavg"]),
        ("processes", ["ps", "-eo", "pid,comm,pcpu,pmem", "--sort=-pcpu"])]}

def run_case(index, side, trace):
    case = out / ("trace" if trace else f"pair{index}")
    case = case / side
    case.mkdir(parents=True)
    shutil.copytree(source / "a/w2.run", case / "w2.run")
    env = dict(os.environ, W2_PENDING_COPY=str(case / "w2.run/s.sqlite"),
               W2_BASE_SIDE=side, W2_TRACE="1" if trace else "0")
    (case / "machine-before.json").write_text(json.dumps(machine(), indent=2))
    cmd = [sys.executable, str(root / ".github/scripts/w2-runtime-process.py"),
           "run", "--state", str(case / "process.json"), "--log", str(case / "run.log"), "--",
           "/usr/bin/time", "-v", "-o", str(case / "time.txt"), str(exe),
           "--ignored", "--exact", "v3::financial_session::tests::w2_pending_cost_probe",
           "--nocapture", "--test-threads=1"]
    remaining = deadline - time.time()
    if remaining <= 0:
        raise RuntimeError("deadline before case launch")
    started = time.monotonic()
    proc = subprocess.Popen(cmd, env=env, cwd=root / "rust")
    timed_out = False
    try:
        code = proc.wait(timeout=remaining)
    except subprocess.TimeoutExpired:
        timed_out = True
        summary["timeouts"] += 1
        proc.terminate()
        code = proc.wait(timeout=15)
    wall = time.monotonic() - started
    (case / "machine-after.json").write_text(json.dumps(machine(), indent=2))
    state = json.loads((case / "process.json").read_text())
    record = {"case": str(case.relative_to(out)), "side": side, "trace": trace,
              "process_wall_seconds": wall, "returncode": code,
              "timed_out": timed_out, "stopped": state["stopped"]}
    summary["cases"].append(record)
    log = (case / "run.log").read_text()
    marker = "W2-PENDING-COST "
    matches = [line.split(marker, 1)[1] for line in log.splitlines() if marker in line]
    if len(matches) == 1:
        record["probe"] = json.loads(matches[0])
    okay = (code == 0 and state["stopped"] and not timed_out and len(matches) == 1
            and "test result: ok. 1 passed;" in log)
    if okay:
        p = record["probe"]
        okay = (p["error"] is None and p["generation"] == 77 and p["reply"] is not None
                and p["side"] == side and p["trace"] == trace
                and p["material"] is not None
                and ((side == "old" and p["proofs"] is None)
                     or (side == "new" and p["proofs"] is not None)))
    if not okay:
        summary["errors"] += 1
        save()
        raise RuntimeError("case failure: " + record["case"])
    summary["successes"] += 1
    save()
    print(json.dumps(record), flush=True)
    return record

def pair(index, trace):
    # 首对 old/new，随后反转每对顺序，减少先后/页缓存偏置。
    order = ["old", "new"] if index % 2 == 0 else ["new", "old"]
    records = {side: run_case(index, side, trace) for side in order}
    a, b = records["old"], records["new"]
    assert a["probe"]["reply"] == b["probe"]["reply"], "complete reply mismatch"
    folder = out / ("trace" if trace else f"pair{index}")
    compare = subprocess.run([sys.executable, str(root / ".github/scripts/compare-g77-tables.py"),
                              str(folder / "old/w2.run/s.sqlite"),
                              str(folder / "new/w2.run/s.sqlite")], capture_output=True, text=True)
    (folder / "comparison.json").write_text(compare.stdout)
    (folder / "comparison.stderr").write_text(compare.stderr)
    assert compare.returncode == 0, "full typed DB mismatch"
    db = json.loads(compare.stdout)
    assert db["all_equal"] and db["schema_equal"] and len(db["tables"]) == 24
    result = {"index": index, "trace": trace, "order": order, "reply_equal": True,
              "schema_equal": True, "table_count": len(db["tables"]),
              "row_count": sum(t["rows"] for t in db["tables"]),
              "old_seconds": int(a["probe"]["elapsed_ns"]) / 1e9,
              "new_seconds": int(b["probe"]["elapsed_ns"]) / 1e9,
              "old_logical": a["probe"]["material"]["logical_reads"],
              "new_logical": b["probe"]["material"]["logical_reads"]}
    summary["pairs"].append(result)
    save()
    return result

def main():
    save()
    (out / "runner.json").write_text(json.dumps(machine(), indent=2))
    (out / "lscpu.txt").write_text(command_output(["lscpu"])["stdout"])
    mode = os.environ.get("G77_MODE", "trace-first")
    assert mode in ("trace-first", "untraced")
    summary["mode"] = mode
    if mode == "trace-first":
        initial = pair(0, True)
        if initial["new_logical"] >= initial["old_logical"] * 0.9:
            summary["status"] = "stopped_no_clear_repeated_work_reduction"
            return
        pair_cost = initial["old_seconds"] + initial["new_seconds"]
    else:
        # 主控消费 r2 等价和工作量证据后批准独立重复性轮；不再按收益挑样本。
        summary["prior_trace_run"] = 41054015956
        summary["authorization_reason"] = "full equivalence, zero errors, SQL reduction and real proof hits"
        pair_cost = 300.0
    for index in range(1, 6):
        if deadline - time.time() < pair_cost * 1.25 + 45:
            summary["status"] = "bounded_partial_samples"
            break
        started = time.monotonic()
        pair(index, False)
        pair_cost = max(pair_cost, time.monotonic() - started)
    else:
        summary["status"] = "five_measured_pairs_complete"
    measured = [p for p in summary["pairs"] if not p["trace"]]
    summary["measured_pairs"] = len(measured)
    if measured:
        summary["timings"] = {}
        for side in ["old", "new"]:
            values = [p[f"{side}_seconds"] for p in measured]
            summary["timings"][side] = {"n": len(values), "values_seconds": values,
                "median_seconds": statistics.median(values), "min_seconds": min(values),
                "max_seconds": max(values)}

try:
    main()
except Exception as exc:
    summary["status"] = "failed"
    summary["failure"] = repr(exc)
    save()
    raise
finally:
    save()
