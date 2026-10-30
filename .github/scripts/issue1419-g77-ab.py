#!/usr/bin/env python3
import json
import os
import signal
from pathlib import Path
import statistics
import subprocess
import sys
import time

root = Path(__file__).resolve().parents[2]
out = root / "g77-evidence"
PROOF_CAP = 3 * 1024 * 1024
MATERIAL_CAP = 32 * 1024 * 1024


def configure(executable, started):
    global deadline, exe, source, summary
    out.mkdir(exist_ok=True)
    deadline = started + 35 * 60 - 120
    exe = Path(executable).resolve()
    source = out / "extracted-original/w2-runtime"
    summary = {"status": "running", "cases": [], "pairs": [], "successes": 0,
               "errors": 0, "timeouts": 0, "launched": 0, "exited": 0,
               "reclaimed": 0, "nonzero_exits": 0, "failed_cases": 0,
               "max_measured_pairs": 5, "started_epoch": started,
               "deadline_epoch": deadline, "archive_reserve_seconds": 120,
               "mode": "trace-first", "base_side": "new",
               "previous_slots": {"old": 0, "new": 16384},
               "timing_scope": "Runtime.commit; trace excluded from measured series",
               "features": ["s_session"], "test_threads": 1,
               "cache_policy": "fresh process and copied DB per arm; alternating order; no cache drop",
               "executable": str(exe),
               "source_head": subprocess.check_output(
                   ["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()}


def remaining():
    seconds = deadline - time.time()
    if seconds <= 0:
        raise TimeoutError("shared diagnostic deadline exhausted")
    return seconds


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def case_env(case, side, trace):
    return dict(os.environ, W2_PENDING_COPY=str(case / "w2.run/s.sqlite"),
                W2_BASE_SIDE="new", W2_PREVIOUS_SIDE=side,
                W2_TRACE="1" if trace else "0")


def validate_resources(p, side):
    d, proof, material = p["development"], p["proofs"], p["material"]
    require(proof is not None and material is not None, "missing closure/material evidence")
    require(not proof["disabled"] and proof["retained_bytes"] > 0, "33W closure disabled")
    require(0 < proof["combined_peak_bytes"] <= PROOF_CAP, "combined proof cap exceeded")
    require(proof["auxiliary_retained_bytes"] == 0, "auxiliary lease not released")
    require(material["retained_capacity_highwater"] <= MATERIAL_CAP, "material cap exceeded")
    require(d["retained_bytes"] == 0 and d["released_bytes"] == d["allocated_bytes"],
            "previous memo not fully released")
    require(d["combined_peak_bytes"] <= PROOF_CAP, "earlier audit combined cap exceeded")
    require(d["capacity"] == (0 if side == "old" else 16384), "wrong actual previous capacity")
    require(d["disabled"] == (side == "old"), "previous selector/allocation mismatch")
    if side == "old":
        require(d["peak_bytes"] == d["allocated_bytes"] == d["hits"] == 0, "old memo active")
    else:
        require(0 < d["peak_bytes"] <= d["allocated_bytes"], "new memo not allocated")
        require(d["peak_bytes"] + proof["retained_bytes"] <= PROOF_CAP, "split proof budgets")


def validate_work(a, b):
    x, y = a["development"], b["development"]
    require(x["sessions"] == y["sessions"] > 0, "audit session count mismatch")
    require(x["eligible_batches"] == y["eligible_batches"] > 0, "no matched eligible batches")
    require(x["current_full"] == y["current_full"] > 0, "current work changed")
    require(y["previous_full"] < x["previous_full"] and y["hits"] > 0, "no previous work reduction")
    require(x["previous_full"] == y["previous_full"] + y["hits"], "previous workload changed")
    require(x["misses"] == x["previous_full"] and y["misses"] == y["previous_full"],
            "previous work accounting mismatch")
    require(a["proofs"]["retained_bytes"] == b["proofs"]["retained_bytes"], "closure capacity changed")


def save():
    summary["remaining_seconds"] = deadline - time.time()
    path = out / "summary.json"
    temp = path.with_suffix(".tmp")
    temp.write_text(json.dumps(summary, ensure_ascii=False, indent=2) + "\n")
    temp.replace(path)


def command_output(command):
    result = subprocess.run(command, capture_output=True, text=True, timeout=min(5, remaining()))
    return {"command": command, "returncode": result.returncode,
            "stdout": result.stdout, "stderr": result.stderr}


def machine():
    return {name: command_output(command) for name, command in [
        ("uname", ["uname", "-a"]), ("cores", ["nproc"]), ("uptime", ["uptime"]),
        ("load", ["cat", "/proc/loadavg"]),
        ("processes", ["ps", "-eo", "pid,comm,pcpu,pmem", "--sort=-pcpu"])]}


def run_case(index, side, trace):
    case = out / ("trace" if trace else f"pair{index}") / side
    case.mkdir(parents=True)
    record = {"case": str(case.relative_to(out)), "side": side, "trace": trace,
              "timed_out": False, "launched": False, "stopped": None}
    summary["cases"].append(record)
    try:
        subprocess.run(["cp", "-R", str(source / "a/w2.run"), str(case / "w2.run")],
                       check=True, timeout=remaining())
        (case / "machine-before.json").write_text(json.dumps(machine(), indent=2))
        cmd = [sys.executable, str(root / ".github/scripts/w2-runtime-process.py"),
               "run", "--state", str(case / "process.json"), "--log", str(case / "run.log"), "--",
               "/usr/bin/time", "-v", "-o", str(case / "time.txt"), str(exe),
               "--ignored", "--exact", "v3::financial_session::tests::w2_pending_cost_probe",
               "--nocapture", "--test-threads=1"]
        wait_seconds = remaining() - 15
        require(wait_seconds > 0, "insufficient time for probe and supervised stop")
        started = time.monotonic()
        with (case / "supervisor.stderr").open("wb") as supervisor_stderr:
            proc = subprocess.Popen(cmd, env=case_env(case, side, trace), cwd=root / "rust",
                                    stderr=supervisor_stderr)
        record["launched"] = True
        summary["launched"] += 1
        try:
            code = proc.wait(timeout=wait_seconds)
        finally:
            if proc.poll() is None:
                proc.terminate()
                proc.wait(timeout=15)
            record["process_wall_seconds"] = time.monotonic() - started
            record["returncode"] = proc.returncode
            summary["exited"] += int(proc.returncode is not None)
            summary["nonzero_exits"] += int(proc.returncode != 0)
            statefile = case / "process.json"
            if statefile.exists():
                state = json.loads(statefile.read_text())
                record["process"] = state
                record["stopped"] = state["stopped"]
                summary["reclaimed"] += int(state["stopped"])
        (case / "machine-after.json").write_text(json.dumps(machine(), indent=2))
        log = (case / "run.log").read_text()
        marker = "W2-PENDING-COST "
        matches = [line.split(marker, 1)[1] for line in log.splitlines() if marker in line]
        require(len(matches) == 1, "missing or duplicate probe receipt")
        p = record["probe"] = json.loads(matches[0])
        validate_resources(p, side)
        record["resources_valid"] = True
        require(code == 0 and record["stopped"], "probe exit/reclamation failure")
        require(p["error"] is None and p["generation"] == 77 and p["reply"] is not None,
                "commit failed or generation mismatch")
        require(p["side"] == side and p["base_side"] == "new" and p["trace"] == trace,
                "arm/trace mismatch")
        require(int(p["elapsed_ns"]) > 0 and p["f_calls"] > 0, "missing measured work")
        require("test result: ok. 1 passed;" in log, "probe test did not pass")
        summary["successes"] += 1
        print(json.dumps(record), flush=True)
        return record
    except BaseException as exc:
        record["failure"] = repr(exc)
        record["timed_out"] = isinstance(exc, (TimeoutError, subprocess.TimeoutExpired))
        summary["timeouts"] += int(record["timed_out"])
        summary["failed_cases"] += 1
        raise
    finally:
        save()


def pair(index, trace):
    # 首对 old/new，随后反转每对顺序，减少先后/页缓存偏置。
    order = ["old", "new"] if index % 2 == 0 else ["new", "old"]
    records = {side: run_case(index, side, trace) for side in order}
    a, b = records["old"], records["new"]
    require(a["probe"]["reply"] == b["probe"]["reply"], "complete reply mismatch")
    validate_work(a["probe"], b["probe"])
    folder = out / ("trace" if trace else f"pair{index}")
    compare = subprocess.run([sys.executable, str(root / ".github/scripts/compare-g77-tables.py"),
                              str(folder / "old/w2.run/s.sqlite"),
                              str(folder / "new/w2.run/s.sqlite")], capture_output=True, text=True, timeout=remaining())
    (folder / "comparison.json").write_text(compare.stdout)
    (folder / "comparison.stderr").write_text(compare.stderr)
    require(compare.returncode == 0, "full typed DB mismatch")
    db = json.loads(compare.stdout)
    require(db["all_equal"] and db["schema_equal"] and len(db["tables"]) == 24,
            "incomplete schema/24-table comparison")
    result = {"index": index, "trace": trace, "order": order, "reply_equal": True,
              "work_valid": True, "schema_equal": True, "table_count": len(db["tables"]),
              "row_count": sum(t["rows"] for t in db["tables"]),
              "old_seconds": int(a["probe"]["elapsed_ns"]) / 1e9,
              "new_seconds": int(b["probe"]["elapsed_ns"]) / 1e9,
              "old_logical": a["probe"]["material"]["logical_reads"],
              "new_logical": b["probe"]["material"]["logical_reads"]}
    summary["pairs"].append(result)
    save()
    return result


def summarize_timings():
    measured = [p for p in summary["pairs"] if not p["trace"]]
    summary["measured_pairs"] = len(measured)
    if not measured:
        summary["benefit"] = "未测出收益：无完整无 trace 样本"
        return
    summary["timings"] = {}
    for side in ["old", "new"]:
        values = [p[f"{side}_seconds"] for p in measured]
        summary["timings"][side] = {"n": len(values), "values_seconds": values,
            "median_seconds": statistics.median(values), "min_seconds": min(values),
            "max_seconds": max(values)}
    savings = [p["old_seconds"] - p["new_seconds"] for p in measured]
    fluctuation = max(v["max_seconds"] - v["min_seconds"] for v in summary["timings"].values())
    saving = statistics.median(savings)
    summary["paired_savings_seconds"] = savings
    summary["median_saving_seconds"] = saving
    summary["observed_range_seconds"] = fluctuation
    summary["benefit"] = (
        "观测到收益：配对节省中位数大于两臂各自的观测极差"
        if len(measured) >= 3 and saving > fluctuation
        else "未测出收益：样本不足或配对节省未超过观测波动")
    if summary["status"] == "failed":
        summary["benefit"] = "未判定收益：本轮验证失败，已采样本仅供诊断"


def main():
    save()
    require((source / "source-head.txt").read_text().strip() ==
            "ca887d965a8a30848c4521b5684ae2b046ca5698", "wrong original source head")
    require((source / "run-id.txt").read_text().strip() == "41005347604", "wrong original run")
    require(os.environ.get("G77_MODE", "trace-first") == "trace-first", "trace-first required")
    (out / "runner.json").write_text(json.dumps(machine(), indent=2))
    (out / "lscpu.txt").write_text(command_output(["lscpu"])["stdout"])
    started = time.monotonic()
    pair(0, True)
    pair_cost = time.monotonic() - started
    for index in range(1, 6):
        if deadline - time.time() < pair_cost * 1.25 + 45:
            summary["status"] = "bounded_partial_samples"
            break
        started = time.monotonic()
        pair(index, False)
        pair_cost = max(pair_cost, time.monotonic() - started)
    else:
        summary["status"] = "five_measured_pairs_complete"


def interrupted(signum, _frame):
    raise InterruptedError(f"runner received signal {signum}")


def cli():
    signal.signal(signal.SIGTERM, interrupted)
    signal.signal(signal.SIGHUP, interrupted)
    configure(sys.argv[1], int(os.environ["G77_STARTED"]))
    try:
        main()
    except BaseException as exc:
        summary["status"] = "failed"
        summary["failure"] = repr(exc)
        summary["errors"] += 1
        if isinstance(exc, (TimeoutError, subprocess.TimeoutExpired)) and not summary["timeouts"]:
            summary["timeouts"] += 1
        raise
    finally:
        summarize_timings()
        save()


if __name__ == "__main__":
    cli()
