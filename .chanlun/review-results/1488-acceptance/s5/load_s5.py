#!/usr/bin/env python3
"""#1488 S5（0075）：各臂宿主负载与计时摘要（读 readings.jsonl、run-manifest.json、HOST-BEFORE/AFTER.txt）。

用法：load_s5.py LABEL=RUN_DIR[:G_DIR] ... [--out JSON]；G_DIR 缺省为 /mnt/g/AgentStorage/1488-accept/s5/LABEL。
"""

import argparse
import json
import statistics
from pathlib import Path


def summary(label, run, g):
    rows = [json.loads(x) for x in (run / "readings.jsonl").read_text().splitlines()]
    m = json.loads((run / "run-manifest.json").read_text())
    loads = [r["host_loadavg_1m"] for r in rows] + [r.get("host_loadavg_1m_before") for r in rows
                                                   if r.get("host_loadavg_1m_before") is not None]
    el = {r["generation"]: r["elapsed_ns"] / 1e9 for r in rows}
    host = {}
    for name in ("HOST-BEFORE.txt", "HOST-AFTER.txt"):
        p = g / name
        if p.exists():
            host[name] = p.read_text().splitlines()[:3]
    return {"label": label, "binary_sha256": m["binary_sha256"], "native": m.get("native"),
            "generations": m["generations"], "failure": m["failure"],
            "loadavg_1m": {"min": min(loads), "median": statistics.median(loads), "max": max(loads)},
            "elapsed_s_total": round(sum(el.values()), 1),
            "elapsed_s_at": {g_: round(el[g_], 1) for g_ in (10, 20, 40, 60, 86) if g_ in el},
            "cpu_ticks_total": sum(r["cpu_ticks"] for r in rows),
            "host_snapshots": host}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("arms", nargs="+")
    ap.add_argument("--out")
    a = ap.parse_args()
    out = []
    for arm in a.arms:
        label, rest = arm.split("=", 1)
        run, _, g = rest.partition(":")
        out.append(summary(label, Path(run), Path(g or f"/mnt/g/AgentStorage/1488-accept/s5/{label}")))
    text = json.dumps(out, indent=1, ensure_ascii=False)
    if a.out:
        Path(a.out).write_text(text + "\n")
    print(text)


if __name__ == "__main__":
    main()
