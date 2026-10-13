#!/usr/bin/env python3
"""#1414：只由先冻结的笔级端点生成整数 OHLC 与手算账；不导入生产代码。"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path

BASE = 2000
SCALE = 100
MIRROR = 10000
# 每一 EXPECTED 都来自 ORACLE-DESIGN.md 的手算；不从运行输出求期望。
DESIGNS = {
    "second_open": {
        "points": [0, 20, 10, 40, 25, 35, 22, 30, 23, 33],
        "result": "CASE_TWO", "match": [4, 6, 8], "fractal": "BOTTOM",
        "merge": None, "later_gap_closer": None,
    },
    "second_nondual": {
        "points": [0, 20, 10, 40, 25, 35, 22, 38, 26, 39, 24, 37],
        "result": "CASE_TWO", "match": [[4, 6], 8, 10], "fractal": "TOP",
        "merge": [4, 6], "later_gap_closer": None,
    },
    "second_inclusion": {
        "points": [0, 20, 10, 40, 25, 35, 22, 38, 21, 32, 23, 33],
        "result": "CASE_TWO", "match": [[4, 6], 8, 10], "fractal": "BOTTOM",
        "merge": [4, 6], "later_gap_closer": None,
    },
    "second_late_close": {
        "points": [0, 20, 10, 40, 25, 35, 22, 30, 15, 27, 18, 28],
        "result": "CASE_TWO", "match": [6, 8, 10], "fractal": "BOTTOM",
        "merge": None, "later_gap_closer": 7,
    },
    "strict_c_closed": {
        "points": [0, 20, 10, 40, 25, 35, 18],
        "result": "CASE_ONE", "match": None, "fractal": None,
        "merge": None, "later_gap_closer": 5,
    },
    "first_extension_pending": {
        "points": [0, 20, 10, 40, 25, 35, 22, 32, 21, 30],
        "result": "PENDING_CASE_TWO", "match": None, "fractal": None,
        "merge": None, "later_gap_closer": None,
    },
    "no_gap_control": {
        "points": [0, 20, 10, 40, 15, 35, 12],
        "result": "CASE_ONE", "match": None, "fractal": None,
        "merge": None, "later_gap_closer": None,
    },
}


def dump(value: object) -> str:
    return json.dumps(value, ensure_ascii=False, indent=2) + "\n"


def transform(price: int, direction: str) -> int:
    scaled = BASE + SCALE * price
    return scaled if direction == "UP" else MIRROR - scaled


def anchor(j: int) -> int:
    return 1 + 4 * j


def known_cut(j: int) -> int:
    return anchor(j) + 2


def element(prices: list[int], member: int | list[int]) -> dict:
    members = [member] if isinstance(member, int) else member
    ranges = [(min(prices[i], prices[i + 1]), max(prices[i], prices[i + 1])) for i in members]
    # 唯一固定合并组4/6：UP方向的SECOND下降min/min；镜像UP处理max/max。
    downward = prices[0] < prices[1]
    low = min(r[0] for r in ranges) if downward else max(r[0] for r in ranges)
    high = min(r[1] for r in ranges) if downward else max(r[1] for r in ranges)
    return {"low": low, "high": high, "stroke_idx": members[-1], "stroke_indices": members}


def raw_rows(design: dict, direction: str) -> list[dict]:
    if direction == "DOWN":
        return [{**r, "open": MIRROR-r["open"], "high": MIRROR-r["low"],
                 "low": MIRROR-r["high"], "close": MIRROR-r["close"]}
                for r in raw_rows(design, "UP")]
    prices = [transform(x, direction) for x in design["points"]]
    # 首个UP端点是底；DOWN镜像后为顶。用极值±1定义中心，极值是已冻结P。
    centers = [p + (1 if (j % 2 == 0) == (direction == "UP") else -1)
               for j, p in enumerate(prices)]
    q_values = [centers[0] + (200 if direction == "UP" else -200), centers[0]]
    for left, right in zip(centers, centers[1:]):
        q_values.extend(left + (right - left) * k // 4 for k in (1, 2, 3, 4))
    last_is_bottom = ((len(prices) - 1) % 2 == 0) == (direction == "UP")
    delta = 100 if last_is_bottom else -100
    q_values.extend([centers[-1] + delta, centers[-1] + 2 * delta])
    return [{"raw_id": f"raw-{i}", "raw_index": i, "open": q, "high": q + 1,
             "low": q - 1, "close": q} for i, q in enumerate(q_values)]


def second_steps(design: dict, prices: list[int], direction: str) -> list[dict]:
    if design["result"] == "CASE_ONE":
        return []
    steps = []
    for i in range(4, len(prices) - 1, 2):
        incoming = element(prices, i)
        action = "merge" if i == 6 and design["merge"] else "append"
        after = element(prices, [4, 6]) if action == "merge" else incoming
        steps.append({"incoming": incoming, "action": action, "after": after,
                      "merge_direction": ("DOWN" if direction == "UP" else "UP") if action == "merge" else None,
                      "scope": "SECOND_SEQUENCE", "known_input_count": known_cut(i + 1)})
    return steps


def build_oracle(design: dict, direction: str, rows: list[dict]) -> dict:
    prices = [transform(x, direction) for x in design["points"]]
    n = len(prices) - 1
    strokes = [{"stroke": i, "direction": direction if i % 2 == 0 else ("DOWN" if direction == "UP" else "UP"),
                "start_raw": anchor(i), "end_raw": anchor(i + 1),
                "start_price": prices[i], "end_price": prices[i + 1],
                "low": min(prices[i], prices[i + 1]), "high": max(prices[i], prices[i + 1]),
                "known_input_count": known_cut(i + 1)} for i in range(n)]
    first = [element(prices, i) for i in (1, 3, 5)]
    raw_gap = design["points"][4] > design["points"][1]
    closed = design["points"][6] <= design["points"][1] if raw_gap else None
    first_kind = "TOP" if direction == "UP" else "BOTTOM"
    second_kind = design["fractal"]
    if direction == "DOWN" and second_kind:
        second_kind = "TOP" if second_kind == "BOTTOM" else "BOTTOM"
    match = None if design["match"] is None else [element(prices, m) for m in design["match"]]
    match_through = None if match is None else match[-1]["stroke_idx"]
    second_known = None if match is None else known_cut(match_through + 1)
    termination_known = 27 if design["result"] == "CASE_ONE" else second_known
    strict = {"elements": first, "raw_gap": raw_gap, "gap_closed_by_c": closed,
              "effective_case": "CASE_ONE" if not raw_gap or closed else "CASE_TWO",
              "closure_operator": "<=" if direction == "UP" else ">=",
              "closure_left": first[2]["low" if direction == "UP" else "high"],
              "closure_right": first[0]["high" if direction == "UP" else "low"],
              "known_input_count": 27,
              "witness_scope": "首次FIRST check与初始右笔5；后来包含组更新不重写首笔封闭时点"}
    steps = second_steps(design, prices, direction)
    last_cut = len(rows)
    cut_names = {15: "seed_first_known", 26: "before_first_right_bar", 27: "first_first_known", last_cut: "whole_finite_prefix"}
    if second_known:
        cut_names[31] = "second_two_raw_members"
        cut_names[second_known - 1] = "before_second_right_bar"
        cut_names[second_known] = "second_first_known"
    if design["later_gap_closer"] == 7:
        cut_names[35] = "later_gap_first_closed_without_second"
    witness = {"earlier_stroke": 2, "later_stroke": 5,
               "earlier_price": prices[3], "later_price": prices[6],
               "comparison": "<=" if direction == "UP" else ">=",
               "i": 2, "j": 4, "minimum_j": 4,
               "known_input_count": 27, "source": "065:44" if direction == "UP" else "065:46"}
    if not raw_gap:
        witness = {**witness, "earlier_stroke": 0, "later_stroke": 3,
                   "earlier_price": prices[1], "later_price": prices[4],
                   "i": 1, "j": 3, "minimum_j": 3, "known_input_count": 19}
        cut_names[19] = "stroke_break_first_known_before_first"
    cuts = {}
    for cut, name in sorted(cut_names.items()):
        available = [s for s in strokes if s["known_input_count"] <= cut]
        count = len(available)
        first_known = cut >= 27
        terminated = termination_known is not None and cut >= termination_known
        result = "NO_FIRST" if not first_known else (design["result"] if terminated else "PENDING_CASE_TWO")
        if design["result"] == "PENDING_CASE_TWO" and first_known:
            result = "PENDING_CASE_TWO"
        second_applicable = first_known and raw_gap and not closed
        matched = second_applicable and second_known is not None and cut >= second_known
        current_steps = [s for s in steps if s["known_input_count"] <= cut] if second_applicable else []
        current_standard = []
        for step in current_steps:
            if step["action"] == "merge":
                current_standard[-1] = step["after"]
            else:
                current_standard.append(step["after"])
        current_raw = [s["incoming"] for s in current_steps]
        actual_through = match_through if matched else count - 1
        cuts[str(cut)] = {
            "cut_label": name, "input_count": cut, "raw_frontier": cut - 1,
            "known_stroke_count": count, "stroke_anchors": [anchor(i) for i in range(count + 1)],
            "prices": prices[:count + 1], "seed_vector": [True, True, True, True],
            "seed_overlap": [min(prices[1], prices[2]), max(prices[1], prices[2])],
            "first_fractal": first_known, "first_fractal_kind": first_kind if first_known else None,
            "first_members": [s["stroke"] for s in available if s["stroke"] % 2 == 1 and s["stroke"] <= 5],
            "first_window": first if first_known else None,
            "first_window_scope": "首次FIRST具名check的笔1/3/5，不等于当前FIRST原成员全集",
            "all_available_reverse_strokes": [s["stroke"] for s in available if s["stroke"] % 2 == 1],
            "raw_gap": raw_gap if first_known else None,
            "strict": strict if first_known else None,
            "second_applicable": second_applicable,
            "second_sequence_status": "CONFIRMED" if matched else ("PENDING" if second_applicable else "NOT_APPLICABLE"),
            "second_fractal": bool(matched) if second_applicable else None,
            "second_fractal_kind": second_kind if matched else None,
            "second_raw_elements": current_raw, "second_standard": current_standard,
            "second_steps": current_steps, "second_match": match if matched else None,
            "second_scan": {"from_stroke": 4, "through_stroke": actual_through,
                            "input_stroke_count": count, "resource_truncated": False,
                            "complete_current_prefix": not matched,
                            "status": "CONFIRMED" if matched else "COMPLETE"} if second_applicable else None,
            "result": result, "end_stroke": 2 if terminated else None,
            "geometric_end": 13 if terminated else None,
            "termination_first_known_input_count": termination_known if terminated else None,
            "termination_first_known_raw": termination_known - 1 if terminated else None,
            "trigger_stroke": (match_through if match is not None else 5) if terminated else None,
            "fact_vector": {"stroke_broken": cut >= witness["known_input_count"], "new_segment_formed": terminated,
                            "segment_broken": terminated, "stroke_break_witness": witness if cut >= witness["known_input_count"] else None},
            "later_old_gap_closed": (cut >= known_cut(design["later_gap_closer"] + 1)) if design["later_gap_closer"] is not None and raw_gap else False,
        }
    return {"direction": direction, "design_points": design["points"], "endpoint_prices": prices,
            "stroke_anchors": [anchor(i) for i in range(len(prices))], "stroke_ledger": strokes,
            "seed_first_known_input_count": 15, "first_first_known_input_count": 27,
            "geometric_end_raw": 13, "strict": strict, "second_first_known_input_count": second_known,
            "second_match_kind": second_kind, "second_match": match,
            "later_gap_closer": design["later_gap_closer"], "cuts": cuts}


def validate_independent_geometry(raw: dict, oracle: dict) -> dict:
    """只检固定几何主张；不实现/调用生产段识别器。"""
    reports = {}
    for name, rows in raw["cases"].items():
        case = oracle["cases"][name]
        for a, b in zip(rows, rows[1:]):
            assert not (a["high"] >= b["high"] and a["low"] <= b["low"]), (name, a, b)
            assert not (b["high"] >= a["high"] and b["low"] <= a["low"]), (name, a, b)
        highs, lows = [r["high"] for r in rows], [r["low"] for r in rows]
        actual_extrema = [i for i in range(1, len(rows)-1)
                          if (highs[i] > highs[i-1] and highs[i] > highs[i+1])
                          or (lows[i] < lows[i-1] and lows[i] < lows[i+1])]
        assert actual_extrema == case["stroke_anchors"], (name, actual_extrema)
        for j, i in enumerate(actual_extrema):
            is_bottom = (j % 2 == 0) == (case["direction"] == "UP")
            assert (lows[i] if is_bottom else highs[i]) == case["endpoint_prices"][j]
        for a, b in zip(actual_extrema, actual_extrema[1:]):
            assert b-a == 4 and b-a-1 == 3
            assert set((a-1, a, a+1)).isdisjoint((b-1, b, b+1))
        for cut in case["cuts"].values():
            for step in cut["second_steps"]:
                if step["action"] == "merge":
                    earlier = element(case["endpoint_prices"], 4)
                    later = step["incoming"]
                    assert later["low"] <= earlier["low"] and later["high"] >= earlier["high"]
                    assert step["after"]["stroke_indices"] == [4,6]
            standard = cut["second_standard"]
            fractals = []
            for a,b,c in zip(standard, standard[1:], standard[2:]):
                if b["high"] > a["high"] and b["high"] > c["high"]:
                    fractals.append(("TOP",c["stroke_idx"]))
                if b["low"] < a["low"] and b["low"] < c["low"]:
                    fractals.append(("BOTTOM",c["stroke_idx"]))
            if cut["second_applicable"]:
                assert bool(fractals) == cut["second_fractal"], (name,cut["input_count"],fractals)
            if cut["second_fractal"]:
                assert fractals[0] == (cut["second_fractal_kind"],cut["trigger_stroke"])
                assert cut["input_count"] >= case["second_first_known_input_count"]
            if cut["first_fractal"]:
                a,b,c=cut["first_window"]
                assert b["high"]>a["high"] and b["high"]>c["high"] if case["direction"]=="UP" else b["low"]<a["low"] and b["low"]<c["low"]
                s=cut["strict"]
                compared=s["closure_left"]<=s["closure_right"] if case["direction"]=="UP" else s["closure_left"]>=s["closure_right"]
                assert s["gap_closed_by_c"] is None if not s["raw_gap"] else s["gap_closed_by_c"]==compared
                witness=cut["fact_vector"]["stroke_break_witness"]
                assert witness["j"] >= witness["i"]+2
                assert witness["later_price"] <= witness["earlier_price"] if case["direction"]=="UP" else witness["later_price"] >= witness["earlier_price"]
            if name.startswith("second_open") or name.startswith("second_nondual"):
                threshold = case["strict"]["closure_right"]
                later_prices = case["endpoint_prices"][4:]
                assert all(p>threshold for p in later_prices) if case["direction"]=="UP" else all(p<threshold for p in later_prices)
        reports[name] = {"raw_count": len(rows), "known_strokes": len(case["stroke_ledger"]),
                         "natural_ohlc_no_inclusion": True, "new_bi_domain": True,
                         "first_known_input_count": 27, "second_first_known_input_count": case["second_first_known_input_count"],
                         "finite_geometry_only": True}
    return {"schema":"tb02d-independent-geometry-check/1", "cases":reports,
            "scope":"纯整数有限几何自检；不是生产测试、独立评审或端到端验收"}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, help="生成固定输入与账；默认只自检，不写文件")
    args = parser.parse_args()
    raw = {"schema": "tb02d-raw-ledger/1", "cases": {}}
    oracle = {"schema": "tb02d-hand-oracle/1", "ticket": 1414,
              "baseline": "f95a7f3ebc0ce84ec2d2f8a8d71b5030b19f89b9",
              "provenance": "先冻结ORACLE-DESIGN.md笔级账；仅由手算端点生成，未导入生产输出",
              "cut_convention": "cuts键为已接纳raw条数N；raw_frontier=N-1，不是最后raw下标",
              "price_transform": {"base":BASE,"scale":SCALE,"mirror":MIRROR}, "cases": {}}
    for stem, design in DESIGNS.items():
        for direction in ("UP", "DOWN"):
            name = stem + "_" + direction.lower()
            rows = raw_rows(design, direction)
            raw["cases"][name] = rows
            oracle["cases"][name] = build_oracle(design, direction, rows)
    report = validate_independent_geometry(raw, oracle)
    if args.output:
        args.output.mkdir(parents=True, exist_ok=True)
        for filename, value in (("raw-ledger.json",raw),("hand-oracle.json",oracle)):
            (args.output/filename).write_text(dump(value),encoding="utf-8")
        (args.output/"MANIFEST.json").write_text(dump({"schema":"tb02d-fixture-manifest/1",
            "ticket":1414,"baseline":oracle["baseline"],"case_count":len(raw["cases"]),
            "artifacts": {n: hashlib.sha256((args.output/n).read_bytes()).hexdigest()
                          for n in ("raw-ledger.json","hand-oracle.json")},
            "generator_sha256":hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
            "provenance":oracle["provenance"],"scope":report["scope"]}),encoding="utf-8")
    print(dump(report),end="")


if __name__ == "__main__":
    main()
