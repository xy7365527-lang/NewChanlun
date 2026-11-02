#!/usr/bin/env python3
"""#1488 a5s6-0076 S6（D11）/A⑤ 分析：analyze.py --oracle RUN.json --arm 标签=运行目录 [...] --out 目录

S6：逐条（op-k ⇒ generation k+1）回复耗时与服务端 /proc 计数（S 进程 CPU、syscr、rchar、read_bytes、wchar）
对照 Δ_R（oracle 逐代 counts.total 之差；A⑤ 已核终面与 oracle 逐代一致时成立）与累计 R。
台阶判据：第 46 条（op-0045，g46）邻域 op-0040..0050 内，相邻 reply 比与相邻 CPU/读量比是否同向同幅；
全程 reply 与 R、rchar、syscr、CPU 的 Pearson 相关；10 s 帽内条数。
A⑤：逐代 ok/错误类别/客户端耗时/请求数。
"""
import argparse
import json
import math
from pathlib import Path


def pearson(x, y):
    n = len(x)
    mx, my = sum(x) / n, sum(y) / n
    sx = math.sqrt(sum((a - mx) ** 2 for a in x))
    sy = math.sqrt(sum((b - my) ** 2 for b in y))
    return round(sum((a - mx) * (b - my) for a, b in zip(x, y)) / (sx * sy), 4) if sx and sy else None


def ratio(a, b):
    return round(a / b, 3) if b else None


def arm(label, d, totals):
    rows = []
    for line in (d / "INGEST.jsonl").read_text().splitlines():
        r = json.loads(line)
        g = int(r["publication"]["generation"]) if r.get("publication") else r["index"] + 1
        s = r["s"]
        rows.append({"op": r["index"], "g": g, "reply_ms": r["reply_ms"], "s_cpu_ms": r["s_cpu_ms"],
                     "cpu_over_wall": r.get("s_cpu_over_wall"), "syscr": s["syscr"], "rchar": s["rchar"],
                     "read_bytes": s["read_bytes"], "wchar": s["wchar"], "host_busy_cores": r.get("host_busy_cores"),
                     "load1": r.get("load1_before"), "R": totals.get(g),
                     "dR": (totals[g] - totals.get(g - 1, 0)) if g in totals else None})
    near = []
    for r in rows:
        if 40 <= r["op"] <= 50 and r["op"] > 0:
            p = rows[r["op"] - 1]
            near.append({"op": r["op"], "g": r["g"], "reply_ms": r["reply_ms"],
                         "reply_x": ratio(r["reply_ms"], p["reply_ms"]), "cpu_x": ratio(r["s_cpu_ms"], p["s_cpu_ms"]),
                         "rchar_x": ratio(r["rchar"], p["rchar"]), "syscr_x": ratio(r["syscr"], p["syscr"]),
                         "wchar_x": ratio(r["wchar"], p["wchar"]), "dR": r["dR"], "R": r["R"],
                         "cpu_over_wall": r["cpu_over_wall"], "load1": r["load1"]})
    full = [r for r in rows if r["R"] is not None]
    reply = [r["reply_ms"] for r in full]
    corr = {k: pearson(reply, [r[k] for r in full]) for k in ("R", "dR", "s_cpu_ms", "rchar", "syscr", "read_bytes", "wchar")}
    a5 = {}
    names = ["A5-formal", "A5-functional"] + sorted((f.stem for f in d.glob("A5-formal-prefix-g*.jsonl")),
                                                    key=lambda n: int(n.rsplit("g", 1)[1]))
    for name in names:
        f = d / f"{name}.jsonl"
        if f.exists():
            a5[name] = [{"mode": x["mode"], "g": x["generation"], "ok": x["ok"], "client_ms": x["client_ms"],
                         "requests": x["http"]["requests"], "bytes": x["http"]["bytes"],
                         "non200": [h["status"] for h in x["http"]["non200"]],
                         "error_code": (x.get("last_failure") or {}).get("code"),
                         "checks": {k: x.get(k) for k in ("reencode_root_equal", "qcut3_equals_server", "oracle_digest_equal",
                                                         "oracle_counts_equal", "oracle_structure_cut_equal",
                                                         "oracle_rule_revision_equal")}}
                        for x in map(json.loads, f.read_text().splitlines())]
    runj = json.loads((d / "RUN.json").read_text()) if (d / "RUN.json").exists() else {}
    probes = [json.loads(x) for x in (d / "READY-PROBE.jsonl").read_text().splitlines()] if (d / "READY-PROBE.jsonl").exists() else []
    return {"label": label, "binary_sha256": runj.get("binary_sha256"), "ops": len(rows),
            "over_10s": [r["op"] for r in rows if r["reply_ms"] > 10000],
            "max_reply_ms": max(r["reply_ms"] for r in rows), "sum_reply_s": round(sum(reply) / 1000, 1),
            "min_cpu_over_wall": min((r["cpu_over_wall"] for r in rows if r["cpu_over_wall"]), default=None),
            "load1_range": [min(r["load1"] for r in rows), max(r["load1"] for r in rows)],
            "corr_reply_with": corr, "near46": near, "rows": rows, "a5": a5,
            "ready_probe": [{"g": p["generation"], "http": p["http"], "s": p["seconds"], "kind": p["kind"],
                             "q_cpu_ms": p["q"] and p["q"]["utime_ms"] + p["q"]["stime_ms"], "q_rchar": p["q"] and p["q"]["rchar"]}
                            for p in probes],
            "stop_exit": runj.get("stop_exit"), "stop_attempts": len(runj.get("stop_attempts", [])),
            "tb03a_economic_ready_http": runj.get("tb03a_economic_ready_http")}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--oracle", required=True, type=Path)
    ap.add_argument("--arm", action="append", required=True)
    ap.add_argument("--out", required=True, type=Path)
    a = ap.parse_args()
    totals = {q["generation"]: int(q["counts"]["total"]) for q in json.loads(a.oracle.read_text())["queries"]}
    arms = [arm(label, Path(path), totals) for label, path in (spec.split("=", 1) for spec in a.arm)]
    a.out.mkdir(parents=True, exist_ok=True)
    (a.out / "ANALYSIS.json").write_text(json.dumps({"schema": "1488-a5s6-analysis/1", "arms": arms}, ensure_ascii=False, indent=1) + "\n")
    md = []
    for x in arms:
        md += [f"## {x['label']}（bin {(x['binary_sha256'] or '?')[:12]}）", "",
               f"- 条数 {x['ops']}；>10 s：{x['over_10s'] or '无'}；最大 {x['max_reply_ms']:.0f} ms；合计 {x['sum_reply_s']} s；"
               f"min CPU/墙钟 {x['min_cpu_over_wall']}；load1 {x['load1_range']}",
               f"- reply 与各量 Pearson：{x['corr_reply_with']}", "",
               "| op | g | reply ms | ×reply | ×cpu | ×rchar | ×syscr | ×wchar | ΔR | R | cpu/wall | load1 |",
               "|---|---|---|---|---|---|---|---|---|---|---|---|"]
        md += [f"| {n['op']} | {n['g']} | {n['reply_ms']:.0f} | {n['reply_x']} | {n['cpu_x']} | {n['rchar_x']} | {n['syscr_x']} | "
               f"{n['wchar_x']} | {n['dR']} | {n['R']} | {n['cpu_over_wall']} | {n['load1']} |" for n in x["near46"]]
        md += ["", "全程逐条：", "", "| op | reply ms | S cpu ms | syscr | rchar | read_bytes | wchar | ΔR | R | cpu/wall |",
               "|---|---|---|---|---|---|---|---|---|---|"]
        md += [f"| {r['op']} | {r['reply_ms']:.0f} | {r['s_cpu_ms']} | {r['syscr']} | {r['rchar']} | {r['read_bytes']} | {r['wchar']} | "
               f"{r['dR']} | {r['R']} | {r['cpu_over_wall']} |" for r in x["rows"]]
        if x["ready_probe"]:
            md += ["", "冻结 Q3 /api/ready 逐条探测（每条提交后、下一条发出前）：", "",
                   "| g | http | 秒 | 类别 | Q cpu ms | Q rchar |", "|---|---|---|---|---|---|"]
            md += [f"| {p['g']} | {p['http']} | {p['s']} | {p['kind']} | {p['q_cpu_ms']} | {p['q_rchar']} |" for p in x["ready_probe"]]
        for name, recs in x["a5"].items():
            md += ["", f"{name}：", "", "| mode | g | ok | client ms | 请求 | 字节 | 非200 | 失败码 |", "|---|---|---|---|---|---|---|---|"]
            md += [f"| {r['mode']} | {r['g']} | {r['ok']} | {r['client_ms']} | {r['requests']} | {r['bytes']} | {r['non200']} | {r['error_code']} |"
                   for r in recs]
        md += [""]
    (a.out / "ANALYSIS.md").write_text("\n".join(md) + "\n")
    print("\n".join(md[:40]))


if __name__ == "__main__":
    main()
