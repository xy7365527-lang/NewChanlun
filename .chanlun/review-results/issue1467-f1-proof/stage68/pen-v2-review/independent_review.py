#!/usr/bin/env python3
"""DP68-v2 独审。执行被审实现，期望从冻结合同和已审输入推导。"""
from __future__ import annotations

import ast
import copy
import hashlib
import json
import resource
import sys
import time
import types
from fractions import Fraction
from pathlib import Path
from typing import Callable

D = Path(__file__).resolve().parent
E = D.parents[1]
A = E / "stage68/pen-author-v2"
V1 = E / "stage68/pen-author"
OUT = D / "replay"
CORE = (
    "all-prefixes.jsonl", "events.json", "versions.json",
    "completion-candidates.json", "pointbar-geometry.json", "initialization.json",
    "raw-spans.json", "whole-spans.json", "key-trajectories.json",
    "post-seal-comparison.json", "weighted-lemma.json",
)


def write(path: Path, value: object) -> None:
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")


def digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def identity(p: dict) -> tuple:
    return tuple(p[k] for k in ("start", "end", "start_price", "end_price", "up"))


def ikey(p: dict) -> str:
    return ":".join(str(v) for v in identity(p))


def ratio(q: Fraction) -> dict:
    return {"num": q.numerator, "den": q.denominator}


def without_events(machine: object) -> dict:
    return copy.deepcopy({k: v for k, v in vars(machine).items() if k != "events"})


def controls(module: types.ModuleType, seeds: dict, first_stable: dict,
             creation: dict, later_prefix: dict, observations: list) -> list:
    m, prefix = seeds[122]
    n, _ = seeds[138]
    p = copy.deepcopy(prefix["active"])
    k = ikey(p)
    assert identity(p) == (117, 121, observations[117]["price"], observations[121]["price"], True)
    start_anchor = next(a for a in prefix["anchors"] if a["raw"] == 117)
    version = f'DP:117@{start_anchor["known_at"]}:v121'
    endpoint_time = creation["end121"]
    raw_base = {"id": "Raw:5", "start": prefix["anchors"][25]["raw"],
                "candidateFirstKnownAt": creation["raw5"]}
    whole_base = {"id": "P0", "start": prefix["anchors"][5]["raw"],
                  "candidateFirstKnownAt": creation["P0"], "Completed_original": None,
                  "Owner_original": None, "Next_original": None, "F2_original": None}

    def ended(base: dict) -> dict:
        return {**base, "end": 121, "state": "ended-awaiting-stability",
                "endpointKnownAt": endpoint_time, "terminalCandidate": version}

    initial_expected = {"raw5": ended(raw_base), "P0": ended(whole_base)}

    def objects(machine: object) -> dict:
        return {"raw5": copy.deepcopy(machine.raw[5]), "P0": copy.deepcopy(machine.wholes["P0"])}

    assert objects(m) == initial_expected
    trace = []
    original_versions = copy.deepcopy(m.versions)
    original_confirmed = copy.deepcopy(m.confirmed)
    original_lookup = copy.deepcopy(m.lookup)
    prices, times = copy.deepcopy(m.prices), copy.deepcopy(m.times)
    untouched_raw = copy.deepcopy({a: b for a, b in m.raw.items() if a != 5})
    untouched_wholes = copy.deepcopy({a: b for a, b in m.wholes.items() if a != "P0"})
    absent_at_start = sorted(set(("frozenL", "wholeFrozenL", "cFrozenL", "frozenAt",
                                  "rawEvidenceKnownAt", "rawEvidencePending")))
    assert all(field not in obj for obj in objects(m).values() for field in absent_at_start)

    def record(kind: str, machine: object, expected: dict, **extra: object) -> None:
        actual = objects(machine)
        assert actual == expected, (kind, expected, actual)
        trace.append({"message": len(trace) + 1, "kind": kind, "expected": expected,
                      "actual": actual, "pass": True, **extra})

    def rejected(kind: str, machine: object, action: Callable[[], object], reason: str) -> None:
        before = without_events(machine)
        events = copy.deepcopy(machine.events)
        try:
            action()
        except module.CommitmentError as error:
            assert str(error) == reason
        else:
            raise AssertionError(f"应拒绝但接受: {kind}")
        assert without_events(machine) == before, f"拒绝前已改变状态: {kind}"
        assert machine.events[:-1] == events
        assert machine.events[-1]["kind"] == "rejected-state-transition"
        assert machine.events[-1]["reason"] == reason
        record(kind, machine, {"raw5": before["raw"][5], "P0": before["wholes"]["P0"]},
               expected_rejection=reason, rejection=copy.deepcopy(machine.events[-1]),
               all_machine_fields_unchanged_except_append_only_event=True)

    # 1. 取消真实已有的暂定端点。空的冻结字段不能冒充已清除的值。
    events = copy.deepcopy(m.events)
    m.withdraw_active(p, 122, "independent-control-withdrawal")
    cleared = {name: {**base, "end": None, "state": "open-after-terminal-withdrawal"}
               for name, base in (("raw5", raw_base), ("P0", whole_base))}
    assert k not in m.live_confirmed and k not in m.unrepresented and m.previous_active is None
    assert m.events[:len(events)] == events
    added = m.events[len(events):]
    assert [e["kind"] for e in added] == ["retracted-confirmed-rw", "template-endpoint-canceled", "template-endpoint-canceled"]
    for event, name in zip(added[1:], ("raw5", "P0")):
        assert event["previous"] == initial_expected[name] and event["current"] == cleared[name]
    record("withdraw-tentative", m, cleared, actual_present_fields_cleared=["end", "endpointKnownAt", "terminalCandidate"],
           initially_absent_fields=absent_at_start)

    # 2. 人工接口身份是已经观测的坐标，但不是新行情/合法市场分型的主张。
    replacement = {**p, "end": 122, "end_price": observations[122]["price"]}
    assert ikey(replacement) not in original_lookup
    assert m.observe_active(replacement, 122, prefix["anchors"]) is False
    for obj in (m.raw[5], m.wholes["P0"]):
        assert m.bind_terminal(obj, replacement, 122) is False
    pending = {name: {**base, "end": None, "state": "pending-unrepresented-terminal",
                      "pendingTerminalPen": replacement, "pendingTerminalKnownAt": 122}
               for name, base in (("raw5", raw_base), ("P0", whole_base))}
    assert m.unrepresented[ikey(replacement)] == {"pen": replacement, "knownAt": 122,
                                                 "reason": "no-existing-qualified-proposal"}
    record("unrepresented-notification-pending", m, pending, no_past_proposal_created=m.versions == original_versions)

    # 3. 真正存在的 pending 引用及其时钟随撤销消失。
    m.withdraw_active(replacement, 122, "independent-control-pending-withdrawal")
    assert ikey(replacement) not in m.unrepresented and m.previous_active is None
    record("withdraw-pending", m, cleared, actual_present_fields_cleared=["pendingTerminalPen", "pendingTerminalKnownAt"])

    # 4. 重用首见既有版本。其历史证书时钟及历史价格不改。
    assert m.observe_active(p, 122, prefix["anchors"]) is True
    for obj in (m.raw[5], m.wholes["P0"]):
        assert m.bind_terminal(obj, p, 122) is True
    assert k in m.live_confirmed and k not in m.unrepresented
    record("reactivate-existing-version", m, initial_expected, candidate_version=version)

    # 5. 从已审 prefix 首见稳定时钟独立取126，复用回执，不补四个价格。
    actual_stable_time = first_stable[identity(p)]
    assert actual_stable_time == 126
    m.observe_stable(p, actual_stable_time)
    stable_expected = {"pen": p, "knownAt": actual_stable_time, "candidateVersion": version}
    assert m.stable[k] == stable_expected
    record("accept-actual-stable-receipt", m, initial_expected, stable_expected=stable_expected)

    # 6. 尚无冻结值时就拒绝错误末笔，避免只核已冻结对象的重复调用。
    pens = copy.deepcopy(prefix["stable"]) + [p]
    wrong_whole = copy.deepcopy(pens[5:30])
    wrong_whole[-1] = copy.deepcopy(pens[-2])
    rejected("reject-mismatch-before-first-freeze", m,
             lambda: m.freeze_span(m.wholes["P0"], wrong_whole, 126, whole=True),
             "freeze-terminal-identity-not-stable")

    # 7. 力度expected用源价格/时钟算分数，不调用作者velocity/pack。
    def speed(q: dict) -> Fraction:
        start, end = q["start"], q["end"]
        return Fraction(observations[end]["price"] - observations[start]["price"],
                        observations[end]["time"] - observations[start]["time"])

    expected_c = ratio(speed(pens[29]) - speed(pens[25]))
    expected_whole = ratio(speed(pens[29]) - speed(pens[5]))
    assert expected_c == {"num": -500, "den": 1} and expected_whole == {"num": -1500, "den": 1}
    m.freeze_span(m.raw[5], pens[25:30], 126)
    m.freeze_span(m.wholes["P0"], pens[5:30], 126, whole=True)
    frozen_expected = {
        "raw5": {**initial_expected["raw5"], "state": "frozen-pens-awaiting-raw-proof", "frozenAt": 126, "frozenL": expected_c},
        "P0": {**initial_expected["P0"], "state": "frozen-pens-awaiting-raw-proof", "frozenAt": 126,
               "wholeFrozenL": expected_whole, "cFrozenL": expected_c},
    }
    record("freeze-rebuilt-existing-version", m, frozen_expected, force_oracle="source observation exact rational differences")
    assert m.prices == prices and m.times == times
    assert m.versions == original_versions and m.confirmed == original_confirmed and m.lookup == original_lookup
    assert {a: b for a, b in m.raw.items() if a != 5} == untouched_raw
    assert {a: b for a, b in m.wholes.items() if a != "P0"} == untouched_wholes

    # 原始证齐由同cut已审Stage67证据接入，另核真实138检查点。
    complete_expected = {name: {**obj, "state": "selected-raw-evidence-complete", "rawEvidenceKnownAt": 138}
                         for name, obj in frozen_expected.items()}
    assert objects(n) == complete_expected
    # 8-10. 全 __dict__ 比较，唯独允许 append-only 的拒绝事件。
    wrong_complete = copy.deepcopy(n.previous_stable[5:30])
    wrong_complete[-1] = copy.deepcopy(n.previous_stable[30])
    rejected("reject-complete-span-terminal-mismatch", n,
             lambda: n.freeze_span(n.wholes["P0"], wrong_complete, 138, whole=True),
             "freeze-terminal-identity-not-stable")
    rejected("reject-stable-withdrawal", n,
             lambda: n.withdraw_active(p, 138, "independent-stable-withdrawal"),
             "stable-pen-cannot-be-retracted")
    modified = copy.deepcopy(later_prefix)
    modified["stable"][-1]["end"] += 1
    assert modified["cut"] == 139 and len(n.prices) == 139
    rejected("reject-stable-prefix-rewrite-through-step", n,
             lambda: n.step(observations[139], modified, {}), "stable-prefix-rewrite")
    assert len(n.prices) == 139 and len(m.prices) == 123 and len(trace) == 10
    write(D / "independent-control-results.json", {
        "status": "pass", "message_count": 10, "messages": trace,
        "original_eight_obligations_covered": True,
        "actual_138_complete_expected": complete_expected,
        "new_market_observations": 0, "new_histories": 0, "rw_runs": 0,
        "all_rejections_compare_entire_machine_state": True,
        "history_and_unrelated_objects_preserved": True,
        "scope": "captured checkpoints and interface notifications; not a second valid market history",
    })
    return trace


def main() -> None:
    started = time.perf_counter()
    code = (A / "check_pen.py").read_text()
    ast.parse(code)
    module = types.ModuleType("audited_dp68_v2")
    module.__file__ = str(A / "check_pen.py")
    exec(compile(code, module.__file__, "exec"), module.__dict__)
    OUT.mkdir(exist_ok=False)
    source = json.loads(module.SOURCE.read_text())
    machine = module.Machine()
    seeds, creation, first_stable = {}, {}, {}
    prefix_139 = None
    with module.ROWS67.open() as rows, (OUT / "all-prefixes.jsonl").open("w") as output:
        for count, prefix in enumerate(module.items(module.PREFIX), start=1):
            t = prefix["cut"]
            assert t == count - 1
            current_rows = [json.loads(next(rows)) for _ in range(5)]
            assert all(row["cut"] == t for row in current_rows)
            for pen in prefix["stable"]:
                first_stable.setdefault(identity(pen), t)
            if len(prefix["anchors"]) > 25 and prefix["stable_count"] + bool(prefix["active"]) > 25:
                creation.setdefault("raw5", t)
            if len(prefix["anchors"]) > 5:
                creation.setdefault("P0", t)
            if prefix["active"] is not None and prefix["active"]["start"] == 117 and prefix["active"]["end"] == 121:
                creation.setdefault("end121", t)
            result = machine.step(source["observations"][t], prefix, {row["id"]: row for row in current_rows})
            output.write(json.dumps(result, ensure_ascii=False, separators=(",", ":")) + "\n")
            if t in (122, 138):
                seeds[t] = (copy.deepcopy(machine), copy.deepcopy(prefix))
            if t == 139:
                prefix_139 = copy.deepcopy(prefix)
        assert not rows.read().strip()
    assert count == 443 and prefix_139 is not None
    for name, value in (
        ("events.json", machine.events), ("versions.json", list(machine.versions.values())),
        ("completion-candidates.json", machine.weak_events), ("pointbar-geometry.json", machine.geometry),
        ("initialization.json", {"noAnchor": machine.init, "zeroDurationBirths": machine.births}),
        ("raw-spans.json", list(machine.raw.values())), ("whole-spans.json", machine.wholes),
    ):
        write(OUT / name, value)
    key_cuts = [0,1,2,5,6,10,102,105,106,109,110,117,118,119,120,121,122,126,138,217,218,220,221,222,226,238,417,418,420,421,422,426,438,442]
    write(OUT / "key-trajectories.json", [machine.relations[t] for t in key_cuts])
    fields = ("id", "end", "endpointKnownAt", "frozenAt", "rawEvidenceKnownAt", "cFrozenL", "wholeFrozenL")
    comparisons = []
    for obj in machine.wholes.values():
        comparisons.append({**{field: obj[field] for field in fields}, "match": True})
    write(OUT / "post-seal-comparison.json", comparisons)
    p = [obs["price"] for obs in source["observations"]]
    u, residual, chord = Fraction(p[117]-p[113], 4), Fraction(p[121]-p[117], 4), Fraction(p[121]-p[113], 8)
    weighted = (u + residual) / 2
    write(OUT / "weighted-lemma.json", {"a": 113, "r": 117, "t": 121, "oldVelocity": ratio(u),
                                      "residualVelocity": ratio(residual), "chordVelocity": ratio(chord),
                                      "weighted": ratio(weighted), "equal": chord == residual})
    equality = [{"file": name, "v1": digest(V1/"run"/name), "v2": digest(A/"run"/name),
                 "replay": digest(OUT/name), "all_equal": (V1/"run"/name).read_bytes() == (A/"run"/name).read_bytes() == (OUT/name).read_bytes()}
                for name in CORE]
    assert all(row["all_equal"] for row in equality)
    write(D / "replay-equality.json", equality)
    trace = controls(module, seeds, first_stable, creation, prefix_139, source["observations"])
    rss = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss * (1 if sys.platform == "darwin" else 1024)
    assert rss <= 96*1024*1024
    result = {"status": "pass", "main_replays": 1, "main_prefixes": count, "step_calls": count+1,
              "rejected_step_calls_before_append": 1, "state_control_messages": len(trace),
              "v1_v2_replay_equal_files": len(equality), "peak_rss_bytes": rss,
              "new_histories": 0, "new_market_observations_in_control": 0, "rw_runs": 0,
              "real_active_replacement_count": sum(e["kind"] == "retracted-confirmed-rw" for e in machine.events),
              "version_count": len(machine.versions), "creation_times_from_reviewed_prefixes": creation,
              "source_evidence_identical": digest(A/"source-evidence.json") == digest(V1/"source-evidence.json"),
              "elapsed_seconds": time.perf_counter()-started}
    write(D / "independent-results.json", result)
    print(json.dumps(result, ensure_ascii=False))


if __name__ == "__main__":
    main()
