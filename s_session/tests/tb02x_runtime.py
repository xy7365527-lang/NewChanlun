#!/usr/bin/env python3
"""#1493（TB-02-X）：第一种情况候选三态（PENDING_CASE_ONE → CONFIRMED）经正式会话入口的非真空验收。

沿用 tb02w_runtime 的正式路径（launcher 起 S/Q，socket 逐条接纳，HTTP live/AsKnown/重访，公共查询经
tb01c_http_capture.cjs 用浏览器同一份 tb01c-client.js 校验，全 cut 再过 Python 合同），只换设计与手算期望：

- `turn`：票面义务 7 的链补一笔确认笔，端点 [0,10,4,20,5,18,7,15,3,12]。s5 ⊂ s3 且 s3 是峰候选 ⟹ s5 是
  s3 的窗成员（s5 已知：PENDING_CASE_ONE，候选态 OPEN_AT_PREFIX_END）；s7 低 3 < b 低 5 且高 15 ≤ 20 ⟹
  先破判据成立（s7 已知：CASE_ONE，候选态 CONFIRMED，段 s0–s2）。UP/DOWN 两向。
- `e2_real`：`rust/tests/fixtures/tb1490/e2_real_3175_40.json`（BTC 1m 真实笔 3175..3189）前 15 笔的端点价，
  按值保序映射为整数点（稠密秩 ×3；段层只比较端点价的大小与相等，保序映射不改任何比较结果），再由 tb02d
  固定生成器生成整数 OHLC。会话只吃 OHLC，笔级夹具不能直接接纳，这是最小可行入口；端点价本身不进会话。
  手算：s5 确认 U s0–s2；D 候选自 s3，s6 ⊃ s4 且更极端（boundary_pair），s8／s10／s12 ⊂ s6 ⟹ 窗成员
  （s8 已知起 PENDING_CASE_ONE），s14 高 39 > 33 且低 15 ≥ 3 ⟹ 先破判据确认（CASE_ONE，段 s3–s5）。
  64 根逐根一代时，cut 41 起公共采集超过客户端 30s 总期限（首跑实测，#1497 同类规模问题）；本设计改为
  一条 ingest 消息携多根（每代恰多知一笔：首代 7 根，其后每代 4 根，末代补尾 1 根），共 16 代；Q 资源配置
  另把 verify_deadline_ms 放宽到 120000，其余与 tb01c 冻结配置相同（调度预算，不是语义）。

期望值全部来自本文件 EXPECTED（按笔手算，依据 xianduan.md §7.1），不调用生产 reducer。cut 约定同 tb02d：
笔 i 在已接纳 4i+7 根时首次已知。
"""
import argparse
import hashlib
import json
import os
import signal
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / "tests"))
import tb02w_runtime as W  # noqa: E402
from s_service_control import canonical  # noqa: E402

GEN = W.GEN
E2 = ROOT.parent / "rust/tests/fixtures/tb1490/e2_real_3175_40.json"


def e2_points(strokes=15):
    rows = json.loads(E2.read_text())["strokes"][:strokes]
    prices = [rows[0][3]] + [r[4] for r in rows]
    rank = {v: i for i, v in enumerate(sorted(set(prices)))}
    return [3 * rank[p] for p in prices]


DESIGNS = {
    "turn": ([0, 10, 4, 20, 5, 18, 7, 15, 3, 12], ("UP", "DOWN")),
    "e2_real": (e2_points(), ("UP",)),
}
BATCHED = {"e2_real"}  # 一代多知一笔（见模块头）；其余设计一根一代。


def chunks(total, batched):
    """每条消息携带的根数前缀界（累计）。"""
    if not batched:
        return list(range(1, total + 1))
    bounds = list(range(7, total + 1, 4))
    return bounds + ([total] if bounds[-1] != total else [])
# 已知笔数 n → 实体 → 期望。只列判定点。
EXPECTED = {
    "turn": {
        6: {"segment:1:1": {"result": "PENDING_CASE_ONE", "turn": "OPEN_AT_PREFIX_END", "members": ["5"]}},
        7: {"segment:1:1": {"result": "PENDING_CASE_ONE", "turn": "OPEN_AT_PREFIX_END", "members": ["5"]}},
        8: {"segment:1:1": {"result": "CASE_ONE", "turn": "CONFIRMED", "members": ["5"], "end_anchor_stroke": 3}},
    },
    "e2_real": {
        6: {"segment:1:1": {"result": "CASE_ONE", "turn": "CONFIRMED", "members": [], "end_anchor_stroke": 3}},
        9: {"segment:1:13": {"result": "PENDING_CASE_ONE", "turn": "OPEN_AT_PREFIX_END", "members": ["8"]}},
        11: {"segment:1:13": {"result": "PENDING_CASE_ONE", "turn": "OPEN_AT_PREFIX_END", "members": ["8", "10"]}},
        13: {"segment:1:13": {"result": "PENDING_CASE_ONE", "turn": "OPEN_AT_PREFIX_END", "members": ["8", "10", "12"]}},
        15: {"segment:1:1": {"result": "CASE_ONE", "turn": "CONFIRMED"},
             "segment:1:13": {"result": "CASE_ONE", "turn": "CONFIRMED", "members": ["8", "10", "12"], "end_anchor_stroke": 6}},
    },
}


def prepare(args):
    plan_dir = args.plan.resolve()
    plan_dir.mkdir(parents=True, exist_ok=False)
    resources = json.loads((ROOT / "tests/fixtures/tb01c/runtime-plan.json").read_text())["s_resources"]
    relaxed = json.loads((ROOT / "tests/fixtures/tb01c/query-resources.json").read_text())
    relaxed["verify_deadline_ms"] = 120000
    W.dump(plan_dir / "query-resources-relaxed.json", relaxed)
    cases, port = {}, args.port
    for design, (points, directions) in DESIGNS.items():
        for direction in directions:
            case = f"{design}_{direction.lower()}"
            rows = GEN.raw_rows({"points": points}, direction)
            sid = "tb02x-" + case.replace("_", "-")
            namespace = "testonly.ohlc." + sid
            case_dir = plan_dir / case
            case_dir.mkdir()
            events, messages = {}, []
            bounds = chunks(len(rows), design in BATCHED)
            for index, (lo, hi) in enumerate(zip([0] + bounds, bounds)):
                op = "op-" + str(index).zfill(4)
                base = 946684800000000000 + index * 10000000000
                events[op] = {"accept_ns": str(base), "attempts": [{"begin_ns": str(base + 100000000), "commit_ns": str(base + 200000000)}]}
                raws = [{"event_id": bar["raw_id"], "revision": "1", "seq": str(bar["raw_index"]), "received_at": "2000-01-01T00:00:00Z",
                         "raw_text": canonical(bar).decode(), "timestamp": str(bar["raw_index"] * 60), "volume": "1",
                         **{k: str(bar[k]) for k in ("open", "high", "low", "close")}} for bar in rows[lo:hi]]
                payload = {"op": "ingest", "target_session_id": sid, "target_session_generation": "1", "writer_epoch": "1",
                           "clock_event_id": op,
                           "raw_input": {"schema_revision": "s-ohlc/1", "session_id": sid, "source_namespace": namespace,
                                         "source_epoch": "1", "instrument": "TEST-OHLC", "profile": "ohlc_integer_tb02a_v1", "events": raws}}
                messages.append({"schema_revision": "s-session/2", "session_id": sid, "session_generation": "1",
                                 "source_namespace": namespace, "source_epoch": "1", "message_id": op, "producer_id": "tb02x-input",
                                 "producer_epoch": "1", "causal_refs": [], "payload_hash": hashlib.sha256(canonical(payload)).hexdigest(),
                                 "payload": payload})
            W.dump(case_dir / "clock.json", {"schema_revision": "s-clock-plan/1", "clock_plan_id": sid,
                                             "origin_utc": "2000-01-01T00:00:00Z", "unit": "ns", "events": events})
            (case_dir / "messages.jsonl").write_bytes(b"".join(canonical(m) + b"\n" for m in messages))
            W.dump(case_dir / "raw-rows.json", rows)
            query = plan_dir / "query-resources-relaxed.json" if design == "e2_real" else ROOT / "tests/fixtures/tb01c/query-resources.json"
            W.dump(case_dir / "config-A.json", {
                "schema_revision": "s-launcher/2", "session_id": sid, "session_generation": "1", "source_namespace": namespace,
                "source_epoch": "1", "producer_id": "tb02x-control", "producer_epoch": "1", "writer_epoch": "1", "query_epoch": "1",
                "delivery_retain_generations": "4", "port": str(port), "startup_timeout_ms": "10000",
                "db": str(case_dir / "A.sqlite"),
                "socket": "/tmp/tb1493-" + hashlib.sha256(str(case_dir / "A").encode()).hexdigest()[:16] + ".sock",
                "binary": str(args.binary.resolve()), "python": str(args.python.resolve()),
                "catalog": str(ROOT / "catalog/signed-catalog.json"), "profile": str(ROOT / "profiles/ohlc_integer_tb02a_v1.json"),
                "clock_plan": str(case_dir / "clock.json"), "browser": str(ROOT / "browser/index.html"),
                "query_resource_config": str(query), "s_resources": resources})
            port += 1
            cases[case] = {"design": design, "direction": direction, "inputs": len(rows), "points": points,
                           "inputs_by_generation": bounds, "generations": len(bounds)}
    plan = {"schema": "tb02x-runtime-plan/1", "ticket": 1493, "cases": cases,
            "binary_sha256": hashlib.sha256(args.binary.read_bytes()).hexdigest(),
            "e2_fixture_sha256": hashlib.sha256(E2.read_bytes()).hexdigest(),
            "generator_sha256": hashlib.sha256((ROOT / "tests/fixtures/tb02d/generate-input.py").read_bytes()).hexdigest(),
            "script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
    W.dump(plan_dir / "PLAN.json", plan)
    return plan


def check_case(meta, known):
    want, problems, checked, timeline = EXPECTED[meta["design"]], [], {}, []
    pending_seen, confirmed_after_pending = {}, {}
    for generation in sorted(known):
        n = W.known_strokes(meta["inputs_by_generation"][generation - 1]) if generation else 0
        row = [generation, n]
        for o in known[generation]["objects"]:
            if o["kind"] != "CC-013.segment":
                continue
            d = o["payload"]["data"]
            row.append([d["entity_id"], d["result"], d["turn_hypothesis_state"], d["window_members"], d["parse_invalid_from"]])
            if d["parse_invalid_from"] is not None or d["parse_invalid"] is not None:
                problems.append(f"cut {generation}: {d['entity_id']} 段端门失败 parse_invalid_from={d['parse_invalid_from']}")
            if d["result"] == "FAILED_GEOMETRY":
                problems.append(f"cut {generation}: {d['entity_id']} FAILED_GEOMETRY")
            if d["result"] == "PENDING_CASE_ONE":
                pending_seen.setdefault(d["entity_id"], generation)
            if d["result"] == "CASE_ONE" and d["entity_id"] in pending_seen:
                confirmed_after_pending.setdefault(d["entity_id"], generation)
        timeline.append(row)
        for entity, exp in want.get(n, {}).items():
            if (n, entity) in checked:
                continue
            checked[(n, entity)] = generation
            seg = W.find(known[generation], "CC-013.segment", entity)
            if seg is None:
                problems.append(f"n={n} cut {generation}: 缺 {entity}")
                continue
            for key, field in (("result", "result"), ("turn", "turn_hypothesis_state"), ("members", "window_members")):
                if key in exp and seg[field] != exp[key]:
                    problems.append(f"n={n} cut {generation} {entity}: {field} {seg[field]} ≠ {exp[key]}")
            if exp.get("result") == "PENDING_CASE_ONE" and "first_kind_third_stroke_within_first" not in seg["waiting_reasons"]:
                problems.append(f"n={n} cut {generation} {entity}: 缺等待原因 first_kind_third_stroke_within_first")
            if "end_anchor_stroke" in exp:
                j = exp["end_anchor_stroke"]
                want_end = {"group_anchor": str(GEN.anchor(j)), "stroke_index": str(j - 1),
                            "price": str(W.price(meta["points"][j], meta["direction"])), "extreme_roots": [str(GEN.anchor(j))]}
                if seg["geometric_end"] != want_end:
                    problems.append(f"n={n} {entity}: geometric_end {seg['geometric_end']} ≠ {want_end}")
    missing = sorted({f"n={n}/{e}" for n, es in want.items() for e in es} - {f"n={n}/{e}" for n, e in checked})
    if missing:
        problems.append(f"未覆盖手算判定点 {missing}")
    if not confirmed_after_pending:
        problems.append("未观察到 PENDING_CASE_ONE → CASE_ONE（CONFIRMED）的候选态变化")
    return {"status": "PASS" if not problems else "FAIL", "problems": problems,
            "pending_first_cut": pending_seen, "confirmed_cut": confirmed_after_pending,
            "decision_cuts": {f"{n}/{e}": g for (n, e), g in checked.items()}, "timeline": timeline}


def make_run_class():
    from tb02a_runtime import Tb02aRun

    class Tb02xRun(Tb02aRun):
        browser_args = None

        def collect_core(self):
            records = super().collect_core()
            if self.browser_args is None:
                return records
            command = [self.node, str(self.root / "tests/tb02x_browser.cjs"), "--base-url",
                       "http://127.0.0.1:" + self.config["port"], "--output", str(self.output / "browser"),
                       "--playwright-module", str(self.browser_args["module"]), "--final-cut", self.browser_args["final"],
                       "--pending-cut", self.browser_args["pending"], "--confirmed-cut", self.browser_args["confirmed"],
                       "--entity", self.browser_args["entity"]]
            with (self.output / "browser.stdout").open("x") as out, (self.output / "browser.stderr").open("x") as err:
                child = subprocess.Popen(command, stdout=out, stderr=err, start_new_session=True)
                try:
                    child.wait(timeout=1800)
                finally:
                    if child.poll() is None:
                        os.killpg(child.pid, signal.SIGKILL)
                        child.wait(timeout=5)
            if child.returncode:
                raise ValueError("真实浏览器验证失败，见 browser.stderr")
            return records + 1
    return Tb02xRun


BROWSER_CUTS = {  # 设计 → (实体, 待定 n, 确认 n)
    "turn": ("segment:1:1", 6, 8),
    "e2_real": ("segment:1:13", 9, 15),
}


def session(args):
    from tb02b_verify import verify
    Run = make_run_class()
    plan_dir = args.plan.resolve()
    plan = json.loads((plan_dir / "PLAN.json").read_text())
    report = {"schema": "tb02x-session-check/1", "cases": {}}
    for case in args.case or list(plan["cases"]):
        meta, case_dir = plan["cases"][case], plan_dir / case
        output = case_dir / "run-A"
        run = Run(config=case_dir / "config-A.json", messages=case_dir / "messages.jsonl", output=output,
                  node=args.node, fault_index=None, fault_stage="after_batch")
        if args.browser:
            entity, pending, confirmed = BROWSER_CUTS[meta["design"]]
            bounds = meta["inputs_by_generation"]
            cut_for = lambda n: str(bounds.index(4 * (n - 1) + 7) + 1)  # noqa: E731 笔 n-1 首次已知的 cut
            run.browser_args = {"module": args.playwright_module, "final": str(meta["generations"]), "entity": entity,
                                "pending": cut_for(pending), "confirmed": cut_for(confirmed)}
        with W.deadline(case, 7200):
            result = run.execute()
        entry = {"status": result["status"], "error": result.get("error")}
        if result["status"] == "CAPTURED_REQUIRES_DOUBLE_RUN_ORACLE_AND_BROWSER":
            try:
                known = W.known_of(output)
                entry["cuts"] = len(known)
                entry["hand_oracle"] = check_case(meta, known)
                entry["contract_all_cuts"] = verify(Path(json.loads((case_dir / "config-A.json").read_text())["db"]), None, args.node)
                if (output / "browser" / "RESULT.json").exists():
                    entry["browser"] = json.loads((output / "browser" / "RESULT.json").read_text())
            except Exception as exc:  # 评估失败不丢已采原件，照实记
                entry["evaluation_error"] = type(exc).__name__ + ": " + str(exc)
        ok = entry.get("hand_oracle", {}).get("status") == "PASS" and "evaluation_error" not in entry and (not args.browser or "browser" in entry)
        entry["case_status"] = "PASS" if ok else "FAIL"
        W.dump(case_dir / "EVAL-A.json", entry)
        report["cases"][case] = entry
    report["status"] = "PASS" if all(c["case_status"] == "PASS" for c in report["cases"].values()) else "FAIL"
    W.dump(plan_dir / ("TB02X-SESSION-CHECK" + ("-" + "-".join(args.case) if args.case else "") + ".json"), report)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    p = sub.add_parser("prepare")
    p.add_argument("--plan", type=Path, required=True)
    for name in ("binary", "python"):
        p.add_argument("--" + name, type=Path, required=True)
    p.add_argument("--port", type=int, default=19160)
    s = sub.add_parser("session")
    s.add_argument("--plan", type=Path, required=True)
    s.add_argument("--node", type=Path, required=True)
    s.add_argument("--case", action="append")
    s.add_argument("--browser", action="store_true")
    s.add_argument("--playwright-module", default="playwright")
    args = parser.parse_args()
    report = {"prepare": prepare, "session": session}[args.command](args)
    summary = report if args.command == "prepare" else {
        "status": report["status"],
        "cases": {k: {"status": v["case_status"], "run": v["status"], "error": v.get("error"), "evaluation_error": v.get("evaluation_error"),
                      "oracle": v.get("hand_oracle", {}).get("status"), "problems": v.get("hand_oracle", {}).get("problems"),
                      "pending_first_cut": v.get("hand_oracle", {}).get("pending_first_cut"),
                      "confirmed_cut": v.get("hand_oracle", {}).get("confirmed_cut"), "browser": v.get("browser")}
                  for k, v in report["cases"].items()}}
    print(json.dumps(summary, ensure_ascii=False))
    return 0 if args.command == "prepare" or report.get("status") == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
