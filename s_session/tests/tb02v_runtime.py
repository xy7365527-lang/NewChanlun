#!/usr/bin/env python3
"""#1405（TB-02-V）：真实 BTC 1m 切片经正式会话入口，核对公共查询上的同价/极值区字段。

三个子命令，只写 `--plan` 指定的仓外目录：

- `prepare`：把 `load_symbol` 同 schema 的切片（默认 #1343 见证 real_C）按 tick 1e-8 量化成整数
  OHLC 冻结消息、时钟与 s-launcher/2 配置，并写出两条运行命令（PLAN.json）。
- `session`：正式 launcher 起 S/Q，经 socket 逐条接纳、经 HTTP 公共查询取 live/AsKnown 候选
  （`tb02a_runtime.Tb02aRun`，不起浏览器），再按 #1343 `WITNESS-WALK.md` 核对公共投影。
- `check-browser`：根排槽后，用 PLAN.json 里的 `browser_command`（`tb02b_runtime.py`，真 Chromium）
  采页面原件；本子命令只读该原件核对页面上可见的组锚 / 极值根集 / raw_first / raw_last /
  policy 版本 / 同价 KEEP 证据。

公共投影的期望值抄自 #1343 `WITNESS-WALK.md`（real_C 切片内 raw 序号坐标），不调用生产 reducer。
"""
import argparse
import hashlib
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / "tests"))
from s_service_control import canonical  # noqa: E402

POLICY = "standard_raw_gap_3;same-price/keep-earlier+extreme-region/1"
RULE = "new-bi-dual-coordinate/2"
KEEP_EVIDENCE = "same_kind_equal_price_keep_earlier"
RETIRED = {"raw_extreme_identity_tie", "actual_extreme_mapping_missing",
           "same_kind_endpoint_identity_tie", "earlier_endpoint_identity_unsettled"}
BI_KINDS = ("CC-008.endpoint", "CC-008.new_bi_pair", "CC-009.same_kind", "CC-010.bi", "CC-055.relation",
            "CC-055.change_event", "CC-056.version")
DEFAULT_SLICE = ROOT.parent / "rust/tests/fixtures/tb02v_same_price/real_C_bars14_40_multi_root_second_gate.json"
# #1343 WITNESS-WALK.md · real_C（切片内 raw 序号）：(kind, 组锚, 极值根集)。
EXPECTED = {
    "endpoints": [("TOP", 1, [1]), ("BOTTOM", 6, [6]), ("TOP", 7, [9, 10]), ("BOTTOM", 12, [12]),
                  ("TOP", 13, [13]), ("BOTTOM", 15, [15]), ("TOP", 16, [16]), ("BOTTOM", 17, [17]),
                  ("TOP", 21, [21, 22]), ("BOTTOM", 24, [24])],
    "same_price_keep": [("BOTTOM", 12, 15)],
    "strokes": [(1, 12, [12], "5", "10"), (12, 21, [21, 22], "9", "8")],
    "extension": ("bi:1:12", 16, 21),
}


def dump(path, value):
    Path(path).write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")


def quantize(price):
    """同 Rust `types::quantize`（tick 1e-8，四舍五入远离零；价格恒正）。"""
    return int(math.floor(price / 1e-8 + 0.5))


def prepare(args):
    plan_dir = args.plan.resolve()
    plan_dir.mkdir(parents=True, exist_ok=False)
    data = json.loads(args.slice.read_bytes())
    bars = [{"raw_index": i, "raw_id": f"tb02v-{i:04d}", **{k: quantize(data[k + "s"][i]) for k in ("open", "high", "low", "close")}}
            for i in range(len(data["closes"]))]
    sid = "tb02v-" + args.slice.stem.replace("_", "-")[:40]
    namespace = "testonly.ohlc." + sid
    events, messages = {}, []
    for index, bar in enumerate(bars):
        op = "op-" + str(index).zfill(4)
        base = 946684800000000000 + index * 10000000000
        events[op] = {"accept_ns": str(base), "attempts": [{"begin_ns": str(base + 100000000), "commit_ns": str(base + 200000000)}]}
        raw = {"event_id": bar["raw_id"], "revision": "1", "seq": str(bar["raw_index"]), "received_at": "2000-01-01T00:00:00Z",
               "raw_text": canonical(bar).decode(), "timestamp": str(bar["raw_index"] * 60), "volume": "1",
               **{k: str(bar[k]) for k in ("open", "high", "low", "close")}}
        payload = {"op": "ingest", "target_session_id": sid, "target_session_generation": "1", "writer_epoch": "1",
                   "clock_event_id": op,
                   "raw_input": {"schema_revision": "s-ohlc/1", "session_id": sid, "source_namespace": namespace,
                                 "source_epoch": "1", "instrument": "TEST-OHLC", "profile": "ohlc_integer_tb02a_v1", "events": [raw]}}
        messages.append({"schema_revision": "s-session/2", "session_id": sid, "session_generation": "1",
                         "source_namespace": namespace, "source_epoch": "1", "message_id": op, "producer_id": "tb02v-input",
                         "producer_epoch": "1", "causal_refs": [], "payload_hash": hashlib.sha256(canonical(payload)).hexdigest(),
                         "payload": payload})
    dump(plan_dir / "clock.json", {"schema_revision": "s-clock-plan/1", "clock_plan_id": sid,
                                   "origin_utc": "2000-01-01T00:00:00Z", "unit": "ns", "events": events})
    (plan_dir / "messages.jsonl").write_bytes(b"".join(canonical(m) + b"\n" for m in messages))
    resources = json.loads((ROOT / "tests/fixtures/tb01c/runtime-plan.json").read_text())["s_resources"]
    for arm, port in (("session", args.port), ("browser", args.port + 1)):
        dump(plan_dir / f"config-{arm}.json", {
            "schema_revision": "s-launcher/2", "session_id": sid, "session_generation": "1", "source_namespace": namespace,
            "source_epoch": "1", "producer_id": "tb02v-control", "producer_epoch": "1", "writer_epoch": "1", "query_epoch": "1",
            "delivery_retain_generations": "4", "port": str(port), "startup_timeout_ms": "10000",
            "db": str(plan_dir / f"{arm}.sqlite"),
            "socket": "/private/tmp/tb1405-" + hashlib.sha256(str(plan_dir / arm).encode()).hexdigest()[:16] + ".sock",
            "binary": str(args.binary.resolve()), "python": str(args.python.resolve()),
            "catalog": str(ROOT / "catalog/signed-catalog.json"), "profile": str(ROOT / "profiles/ohlc_integer_tb02a_v1.json"),
            "clock_plan": str(plan_dir / "clock.json"), "browser": str(ROOT / "browser/index.html"),
            "query_resource_config": str(ROOT / "tests/fixtures/tb01c/query-resources.json"), "s_resources": resources})
    browser_command = [str(args.python.resolve()), str(ROOT / "tests/tb02b_runtime.py"), "--config", str(plan_dir / "config-browser.json"),
                       "--messages", str(plan_dir / "messages.jsonl"), "--output", str(plan_dir / "browser-run"),
                       "--node", str(args.node.resolve()), "--playwright-module", str(args.playwright_module)]
    plan = {"schema": "tb02v-runtime-plan/1", "ticket": 1405, "slice": str(args.slice.resolve()),
            "slice_sha256": hashlib.sha256(args.slice.read_bytes()).hexdigest(), "bars": len(bars),
            "slice_meta": data.get("meta"), "binary_sha256": hashlib.sha256(args.binary.read_bytes()).hexdigest(),
            "session_command": [str(args.python.resolve()), str(Path(__file__).resolve()), "session", "--plan", str(plan_dir),
                                "--node", str(args.node.resolve())],
            "browser_command": browser_command,
            "check_browser_command": [str(args.python.resolve()), str(Path(__file__).resolve()), "check-browser", "--plan", str(plan_dir)]}
    dump(plan_dir / "PLAN.json", plan)
    return plan


def bi_objects(snapshot):
    out = {}
    for o in snapshot["objects"]:
        if o["kind"] in BI_KINDS:
            out.setdefault(o["kind"], []).append(o["payload"])
    return out


def check_state(snapshot, final):
    """单个公共 cut 的通用不变式；final=True 时再逐项对 WITNESS-WALK。"""
    objs = bi_objects(snapshot)
    for kind, payloads in objs.items():
        for p in payloads:
            assert (p["schema_revision"], p["semantic_version"], p["policy_version"]) == ("s-new-bi/2", RULE, POLICY), kind
            text = json.dumps(p["data"], ensure_ascii=False)
            assert not any(tag in text for tag in RETIRED), (kind, "退役的同价等待态复现")
    for p in objs.get("CC-009.same_kind", []):
        d = p["data"]
        assert d["selection"] in ("KEEP", "REPLACE")
        assert d["evidence"] == ([KEEP_EVIDENCE] if d["comparison"] == "EQ" else [])
    if not final:
        return objs
    endpoints = sorted(((d["kind"], int(d["group_anchor"]), [int(r) for r in d["extreme_roots"]], int(d["raw_first"]), int(d["raw_last"]))
                        for d in (p["data"] for p in objs["CC-008.endpoint"])), key=lambda e: e[1])
    want = [(k, a, r, min(r), max(r)) for k, a, r in EXPECTED["endpoints"]]
    assert endpoints == want, ("端点组锚/极值根集/首末序位不符 WITNESS-WALK", endpoints)
    keeps = sorted((d["old"]["kind"], int(d["old"]["group_anchor"]), int(d["new"]["group_anchor"]))
                   for d in (p["data"] for p in objs["CC-009.same_kind"]) if d["comparison"] == "EQ")
    assert keeps == EXPECTED["same_price_keep"], ("同价 KEEP 不符", keeps)
    for d in (p["data"] for p in objs["CC-009.same_kind"]):
        if d["comparison"] == "EQ":
            assert (d["selection"], d["retained_anchors"]) == ("KEEP", [d["old"]["group_anchor"]])
    strokes = sorted((int(d["start"]["group_anchor"]), int(d["end"]["group_anchor"]), [int(r) for r in d["end"]["extreme_roots"]],
                      d["formation"]["merged_gap"], d["formation"]["raw_between_actual_extrema"])
                     for d in (p["data"] for p in objs["CC-010.bi"]))
    assert strokes == [tuple(s) for s in EXPECTED["strokes"]], ("笔不符", strokes)
    end = next(d["end"] for d in (p["data"] for p in objs["CC-010.bi"]) if d["entity_id"] == EXPECTED["extension"][0])
    assert (end["raw_first"], end["raw_last"]) == ("21", "22"), "多根笔终点首末 raw 序位"
    return objs


def session(args):
    from tb02a_runtime import Tb02aRun
    from tb02b_verify import verify
    plan_dir = args.plan.resolve()
    output = plan_dir / "session-run"
    result = Tb02aRun(config=plan_dir / "config-session.json", messages=plan_dir / "messages.jsonl", output=output,
                      node=args.node, fault_index=None, fault_stage="after_batch").execute()
    report = {"run_status": result["status"], "run_error": result.get("error")}
    if result["status"] != "CAPTURED_REQUIRES_DOUBLE_RUN_ORACLE_AND_BROWSER":
        report["status"] = "FAIL"
        dump(plan_dir / "TB02V-SESSION-CHECK.json", report)
        return report
    report["contract_all_cuts"] = verify(output.parent / "session.sqlite", None, args.node)
    # 实际 HTTP 公共候选（Tb02aRun 记录的 known-<i> = AsKnown 第 i+1 代），逐 cut 核不变式与延伸替代链。
    records = [json.loads(line) for line in (output / "semantic-core.jsonl").read_text().splitlines()]
    known = {}
    for r in records:
        if r.get("kind") == "public_candidate" and r["command"]["id"].startswith("known-"):
            known[int(r["command"]["generation"])] = r["candidate"]["state"]["snapshot"]
    entity, before_end, after_end = EXPECTED["extension"]
    seen_ends, first_bi_ends = [], []
    for generation in sorted(known):
        final = generation == max(known)
        objs = check_state(known[generation], final)
        bis = {p["data"]["entity_id"]: p["data"] for p in objs.get("CC-010.bi", [])}
        if entity in bis:
            seen_ends.append((generation, int(bis[entity]["end"]["group_anchor"]), bis[entity]["entity_revision"]))
        if "bi:1:1" in bis:
            first_bi_ends.append((generation, int(bis["bi:1:1"]["end"]["group_anchor"])))
    ends = [e for _, e, _ in seen_ends]
    assert before_end in ends and ends[-1] == after_end and ends == sorted(ends), ("末笔延伸替代链不符", seen_ends)
    assert first_bi_ends and all(e == 12 for _, e in first_bi_ends), ("首笔被同价弹出或改写", first_bi_ends)
    assert first_bi_ends[-1][0] == max(known), "首笔在末 cut 缺失"
    report.update(status="PASS", cuts=len(known), extension_chain=seen_ends, first_stroke_cuts=len(first_bi_ends),
                  scope="正式 launcher + socket 接纳 + HTTP 公共查询（live/AsKnown/重访）；浏览器另由 check-browser 核")
    dump(plan_dir / "TB02V-SESSION-CHECK.json", report)
    return report


def check_browser(args):
    plan_dir = args.plan.resolve()
    run = plan_dir / "browser-run"
    result = json.loads((run / "RESULT.json").read_text())
    browser = json.loads((run / "browser" / "RESULT.json").read_text())
    texts = {axis: (run / "browser" / f"{axis}.txt").read_text() for axis in ("CC-008", "CC-009", "CC-010", "CC-056")}
    must = {"CC-008": ["group_anchor", "extreme_roots", "raw_first", "raw_last", POLICY, RULE],
            "CC-009": ["comparison", "EQ", "KEEP", "evidence", KEEP_EVIDENCE, "retained_anchors", POLICY],
            "CC-010": ["raw_first", "raw_last", "extreme_roots", POLICY],
            "CC-056": [POLICY]}
    missing = {axis: [m for m in items if m not in texts[axis]] for axis, items in must.items()}
    retired = {axis: [t for t in RETIRED if t in text] for axis, text in texts.items()}
    ok = browser.get("status") == "passed" and not any(missing.values()) and not any(retired.values())
    report = {"status": "PASS" if ok else "FAIL", "runtime_status": result.get("status"), "browser": browser,
              "missing_visible_fields": missing, "retired_waiting_visible": retired,
              "text_sha256": {a: hashlib.sha256(t.encode()).hexdigest() for a, t in texts.items()}}
    dump(plan_dir / "TB02V-BROWSER-CHECK.json", report)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    p = sub.add_parser("prepare")
    p.add_argument("--plan", type=Path, required=True)
    p.add_argument("--slice", type=Path, default=DEFAULT_SLICE)
    for name in ("binary", "python", "node"):
        p.add_argument("--" + name, type=Path, required=True)
    p.add_argument("--playwright-module", default="playwright")
    p.add_argument("--port", type=int, default=18950)
    s = sub.add_parser("session")
    s.add_argument("--plan", type=Path, required=True)
    s.add_argument("--node", type=Path, required=True)
    b = sub.add_parser("check-browser")
    b.add_argument("--plan", type=Path, required=True)
    args = parser.parse_args()
    report = {"prepare": prepare, "session": session, "check-browser": check_browser}[args.command](args)
    print(json.dumps(report, ensure_ascii=False))
    return 0 if args.command == "prepare" or report.get("status") == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
