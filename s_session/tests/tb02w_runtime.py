#!/usr/bin/env python3
"""#1420（TB-02-W）：第二种候选三态终点经正式会话入口的非真空验收。

子命令只写 `--plan` 指定的仓外目录：

- `prepare`：按 tb02d 固定生成器（`fixtures/tb02d/generate-input.py` 的 `raw_rows`，只由手算端点
  生成整数 OHLC，不导入生产代码）把两个设计写成冻结消息、时钟与 s-launcher/2 配置。
  `killed_then_b`：078:52 失效先到（s6 严格越过 P），其后 s8/s10 在旧「只认分型」下会让候选复活；
  `confirmed`：第二特征序列先成分型（067:38）。每个设计 UP/DOWN 两向。
- `session`：每个用例三次正式运行（launcher 起 S/Q，socket 逐条接纳，HTTP live/AsKnown/重访）：
  A 不中断；B 新进程同输入重放；C 在中间消息 SIGKILL S 后 recover 续跑。逐 cut 对手算期望，
  并对拍 A≡B≡C 的结构对象。`--browser` 时在 A 上用真 Chromium 读既有页面。

期望值全部来自本文件 EXPECTED（按笔逐步手算，依据 xianduan.md §5.1、078:52、067:38、065:28），
不调用生产 reducer。cut 约定同 tb02d：笔 i 在已接纳 4i+7 根时首次已知。
"""
import argparse
import contextlib
import hashlib
import importlib.util
import json
import os
import signal
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
sys.path.insert(0, str(ROOT / "tests"))
from s_service_control import canonical  # noqa: E402

_spec = importlib.util.spec_from_file_location("tb02d_gen", ROOT / "tests/fixtures/tb02d/generate-input.py")
GEN = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(GEN)

DESIGNS = {
    # s3 = FIRST 顶分型中心 b（P=40），a/b 缺口 [20,25] 未被 c(s5 低 22) 封闭 ⟹ 第二种候选。
    "killed_then_b": [0, 20, 10, 40, 25, 35, 22, 45, 30, 38, 26, 34, 15, 28, 20, 50, 30, 60],
    "confirmed": [0, 20, 10, 40, 25, 35, 22, 30, 23, 33],
}
# 手算（已知笔数 n → segment:1:1 的状态）。只列判定点；其余 cut 由单调约束覆盖。
EXPECTED = {
    "killed_then_b": {
        # n=6：s5 补成 FIRST 顶分型，c 未封缺口；第二序列仅 s4 一项 ⟹ 未决，不是否定（065:28）。
        6: {"result": "PENDING_CASE_TWO", "semantic": "OPEN_AT_PREFIX_END"},
        # n=7：s6 22→45 先严格越过 P=40（078:52）⟹ 候选失效；原段不终结。
        7: {"result": "KILLED_CASE_TWO", "semantic": "KILLED_BY_NEW_EXTREME", "killed_stroke": 6, "killed_point": 45},
        # n=10：FIRST 中 s7[30,45] 由 s9 补成顶分型，s5 高 35 ≥ s7 低 30 无缺口 ⟹ 第一种情况，A 终于 s6 末端。
        10: {"result": "CASE_ONE", "end_anchor_stroke": 7},
    },
    "confirmed": {
        6: {"result": "PENDING_CASE_TWO", "semantic": "OPEN_AT_PREFIX_END"},
        # n=9：s8 补出第二序列底分型 (35,25)(30,22)(33,23) ⟹ 确认，A 终于 s2 末端（b 顶）。
        9: {"result": "CASE_TWO", "semantic": "CONFIRMED", "end_anchor_stroke": 3},
    },
}
FORBIDDEN_AFTER_KILL = {"CASE_TWO"}
SEGMENT, SECOND = "segment:1:1", "second:segment:1:1"


def dump(path, value):
    Path(path).write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")


def known_strokes(inputs):
    return 0 if inputs < 7 else (inputs - 7) // 4 + 1


def price(point, direction):
    return GEN.transform(point, direction)


def prepare(args):
    plan_dir = args.plan.resolve()
    plan_dir.mkdir(parents=True, exist_ok=False)
    resources = json.loads((ROOT / "tests/fixtures/tb01c/runtime-plan.json").read_text())["s_resources"]
    cases, port = {}, args.port
    for design, points in DESIGNS.items():
        for direction in ("UP", "DOWN"):
            case = f"{design}_{direction.lower()}"
            rows = GEN.raw_rows({"points": points}, direction)
            sid = "tb02w-" + case.replace("_", "-")
            namespace = "testonly.ohlc." + sid
            case_dir = plan_dir / case
            case_dir.mkdir()
            events, messages = {}, []
            for index, bar in enumerate(rows):
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
                                 "source_namespace": namespace, "source_epoch": "1", "message_id": op, "producer_id": "tb02w-input",
                                 "producer_epoch": "1", "causal_refs": [], "payload_hash": hashlib.sha256(canonical(payload)).hexdigest(),
                                 "payload": payload})
            dump(case_dir / "clock.json", {"schema_revision": "s-clock-plan/1", "clock_plan_id": sid,
                                           "origin_utc": "2000-01-01T00:00:00Z", "unit": "ns", "events": events})
            (case_dir / "messages.jsonl").write_bytes(b"".join(canonical(m) + b"\n" for m in messages))
            dump(case_dir / "raw-rows.json", rows)
            for arm in ("A", "B", "C"):
                dump(case_dir / f"config-{arm}.json", {
                    "schema_revision": "s-launcher/2", "session_id": sid, "session_generation": "1", "source_namespace": namespace,
                    "source_epoch": "1", "producer_id": "tb02w-control", "producer_epoch": "1", "writer_epoch": "1", "query_epoch": "1",
                    "delivery_retain_generations": "4", "port": str(port), "startup_timeout_ms": "10000",
                    "db": str(case_dir / f"{arm}.sqlite"),
                    "socket": "/tmp/tb1420-" + hashlib.sha256(str(case_dir / arm).encode()).hexdigest()[:16] + ".sock",
                    "binary": str(args.binary.resolve()), "python": str(args.python.resolve()),
                    "catalog": str(ROOT / "catalog/signed-catalog.json"), "profile": str(ROOT / "profiles/ohlc_integer_tb02a_v1.json"),
                    "clock_plan": str(case_dir / "clock.json"), "browser": str(ROOT / "browser/index.html"),
                    "query_resource_config": str(ROOT / "tests/fixtures/tb01c/query-resources.json"), "s_resources": resources})
                port += 1
            cases[case] = {"design": design, "direction": direction, "inputs": len(rows),
                           "fault_index": len(rows) // 2}
    plan = {"schema": "tb02w-runtime-plan/1", "ticket": 1420, "cases": cases,
            "binary_sha256": hashlib.sha256(args.binary.read_bytes()).hexdigest(),
            "generator_sha256": hashlib.sha256((ROOT / "tests/fixtures/tb02d/generate-input.py").read_bytes()).hexdigest(),
            "script_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
    dump(plan_dir / "PLAN.json", plan)
    return plan


def find(snapshot, kind, entity):
    for o in snapshot["objects"]:
        if o["kind"] == kind and o["payload"]["data"].get("entity_id") == entity:
            return o["payload"]["data"]
    return None


def structural(snapshot):
    """跨运行对拍的结构面：TB-02 对象的 (kind, fact_key, payload)；与写者换代、进程身份无关。"""
    items = sorted((o["kind"], o["fact_key"], canonical(o["payload"]).decode())
                   for o in snapshot["objects"] if o["kind"].startswith("CC-"))
    return hashlib.sha256(canonical(items)).hexdigest(), len(items)


def check_case(case, meta, known):
    design, direction = meta["design"], meta["direction"]
    want = EXPECTED[design]
    up = direction == "UP"
    timeline, seen_kill, problems, checked = [], False, [], {}
    for generation in sorted(known):
        n = known_strokes(generation)
        seg = find(known[generation], "CC-013.segment", SEGMENT)
        second = find(known[generation], "CC-012.feature_sequence", SECOND)
        result = seg and seg["result"]
        semantic = second and second.get("scan", {}).get("semantic_state")
        timeline.append([generation, n, result, semantic])
        if result == "KILLED_CASE_TWO":
            seen_kill = True
        if seen_kill and result in FORBIDDEN_AFTER_KILL:
            problems.append(f"cut {generation}: 失效候选复活为 {result}")
        if seen_kill and semantic == "CONFIRMED":
            problems.append(f"cut {generation}: 失效后第二序列被确认")
        exp = want.get(n)
        if exp is None or n in checked:
            continue
        checked[n] = generation
        if result != exp["result"]:
            problems.append(f"n={n} cut {generation}: result {result} ≠ {exp['result']}")
        if "semantic" in exp and semantic != exp["semantic"]:
            problems.append(f"n={n} cut {generation}: semantic_state {semantic} ≠ {exp['semantic']}")
        if "killed_stroke" in exp:
            k = (second or {}).get("facts", {}).get("killed_by") or {}
            want_price = price(exp["killed_point"], direction)
            got = (k.get("stroke_idx"), k.get("price"), k.get("comparison"))
            if got != (str(exp["killed_stroke"]), str(want_price), ">" if up else "<") and \
               got != (exp["killed_stroke"], want_price, ">" if up else "<"):
                problems.append(f"n={n}: killed_by {got} ≠ ({exp['killed_stroke']}, {want_price}, {'>' if up else '<'})")
            if "second_candidate_killed_by_new_extreme" not in (seg or {}).get("waiting_reasons", []):
                problems.append(f"n={n}: 段缺 second_candidate_killed_by_new_extreme")
        if "end_anchor_stroke" in exp:
            end = GEN.anchor(exp["end_anchor_stroke"])
            if str((seg or {}).get("geometric_end")) != str(end):
                problems.append(f"n={n}: geometric_end {(seg or {}).get('geometric_end')} ≠ {end}")
    missing = sorted(set(want) - set(checked))
    if missing:
        problems.append(f"未覆盖手算判定点 n={missing}")
    final = timeline[-1]
    terminal = max(want)
    if final[2] != want[terminal]["result"]:
        problems.append(f"末 cut result {final[2]} ≠ {want[terminal]['result']}")
    if design == "killed_then_b" and not seen_kill:
        problems.append("未出现 KILLED_CASE_TWO")
    return {"status": "PASS" if not problems else "FAIL", "problems": problems,
            "decision_cuts": {str(k): v for k, v in checked.items()}, "timeline": timeline}


@contextlib.contextmanager
def deadline(label, seconds):
    previous = signal.getsignal(signal.SIGALRM)

    def expired(_s, _f):
        raise TimeoutError(f"{label} 超过 {seconds}s")
    signal.signal(signal.SIGALRM, expired)
    signal.setitimer(signal.ITIMER_REAL, seconds)
    try:
        yield
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        signal.signal(signal.SIGALRM, previous)


def make_run_class():
    from tb02a_runtime import Tb02aRun

    class Tb02wRun(Tb02aRun):
        browser_args = None

        def collect_core(self):
            records = super().collect_core()
            if self.browser_args is None:
                return records
            command = [self.node, str(self.root / "tests/tb02w_browser.cjs"), "--base-url",
                       "http://127.0.0.1:" + self.config["port"], "--output", str(self.output / "browser"),
                       "--playwright-module", str(self.browser_args["module"]),
                       "--final-cut", self.browser_args["final"], "--killed-cut", self.browser_args["killed"],
                       "--open-cut", self.browser_args["open"], "--design", self.browser_args["design"]]
            with (self.output / "browser.stdout").open("x") as out, (self.output / "browser.stderr").open("x") as err:
                child = subprocess.Popen(command, stdout=out, stderr=err, start_new_session=True)
                try:
                    child.wait(timeout=90)
                finally:
                    if child.poll() is None:
                        os.killpg(child.pid, signal.SIGKILL)
                        child.wait(timeout=5)
            if child.returncode:
                raise ValueError("真实浏览器验证失败，见 browser.stderr")
            return records + 1
    return Tb02wRun


def known_of(output):
    known = {}
    for line in (output / "semantic-core.jsonl").read_text().splitlines():
        r = json.loads(line)
        if r.get("kind") == "public_candidate" and r["command"]["id"].startswith("known-"):
            known[int(r["command"]["generation"])] = r["candidate"]["state"]["snapshot"]
    return known


def session(args):
    from tb02b_verify import verify
    Run = make_run_class()
    plan_dir = args.plan.resolve()
    plan = json.loads((plan_dir / "PLAN.json").read_text())
    report = {"schema": "tb02w-session-check/1", "cases": {}}
    selected = args.case or list(plan["cases"])
    for case in selected:
        meta = plan["cases"][case]
        case_dir = plan_dir / case
        arms = {}
        for arm in ("A", "B", "C"):
            output = case_dir / f"run-{arm}"
            run = Run(config=case_dir / f"config-{arm}.json", messages=case_dir / "messages.jsonl", output=output,
                      node=args.node, fault_index=meta["fault_index"] if arm == "C" else None, fault_stage="after_batch")
            if arm == "A" and args.browser:
                want = EXPECTED[meta["design"]]
                cut_for = lambda n: str(4 * (n - 1) + 7)  # noqa: E731 笔 n-1 首次已知的 cut
                killed_n = next((n for n, e in want.items() if e["result"] == "KILLED_CASE_TWO"), max(want))
                run.browser_args = {"module": args.playwright_module, "final": str(meta["inputs"]),
                                    "killed": cut_for(killed_n), "open": cut_for(6), "design": meta["design"]}
            with deadline(f"{case}/{arm}", 900):
                result = run.execute()
            entry = {"status": result["status"], "error": result.get("error")}
            if result["status"] == "CAPTURED_REQUIRES_DOUBLE_RUN_ORACLE_AND_BROWSER":
                known = known_of(output)
                entry["cuts"] = len(known)
                entry["structural"] = {str(g): structural(s)[0] for g, s in sorted(known.items())}
                entry["hand_oracle"] = check_case(case, meta, known)
                entry["contract_all_cuts"] = verify(Path(json.loads((case_dir / f"config-{arm}.json").read_text())["db"]), None, args.node)
                if arm == "C":
                    entry["lifecycle"] = json.loads((output / "RESULT.json").read_text()).get("status")
            arms[arm] = entry
        a, b, c = (arms[x].get("structural") for x in "ABC")
        same_ab = a is not None and a == b
        same_ac = a is not None and a == c
        oracle_ok = all(arms[x].get("hand_oracle", {}).get("status") == "PASS" for x in "ABC")
        report["cases"][case] = {"arms": arms, "replay_new_process_equal": same_ab,
                                 "sigkill_recover_equal": same_ac,
                                 "status": "PASS" if (same_ab and same_ac and oracle_ok) else "FAIL"}
    report["status"] = "PASS" if all(c["status"] == "PASS" for c in report["cases"].values()) else "FAIL"
    dump(plan_dir / "TB02W-SESSION-CHECK.json", report)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    p = sub.add_parser("prepare")
    p.add_argument("--plan", type=Path, required=True)
    for name in ("binary", "python"):
        p.add_argument("--" + name, type=Path, required=True)
    p.add_argument("--port", type=int, default=18960)
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
        "cases": {k: {"status": v["status"], "replay": v["replay_new_process_equal"], "recover": v["sigkill_recover_equal"],
                      "arms": {a: {"status": e["status"], "error": e.get("error"),
                                   "oracle": e.get("hand_oracle", {}).get("status"),
                                   "problems": e.get("hand_oracle", {}).get("problems"),
                                   "decision_cuts": e.get("hand_oracle", {}).get("decision_cuts")}
                               for a, e in v["arms"].items()}}
                  for k, v in report["cases"].items()}}
    print(json.dumps(summary, ensure_ascii=False))
    return 0 if args.command == "prepare" or report.get("status") == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
