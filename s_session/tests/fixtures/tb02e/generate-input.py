#!/usr/bin/env python3
"""#1419 S9（根裁 D-E (B)）：TB-02-E 正式运行的冻结输入与手算账；不导入生产代码、不读生产输出。

输入取 12 笔最小流 MIN12（设计点 10,30,20,50,25,45,15,35,20,40,28,38,22，×10 后照 TB-02-D 变换）的前 43 根。
原始 K 线由 TB-02-D 生成器的 raw_rows 落地（每笔 4 根 = 新笔下限 raw_between≥3、merged_gap≥3）。
(B) 口径：S 摄入代价随历史超线性，正式会话单条回复在第 48 条超过 10 s 帽（S0b/MIN12 实测），
所以正式运行只到第 43 条：u0、u1 两个完成段加进行中的 u2，TB-02-E 只发布单元归属与版本游标，不出 seed/中枢。
seed、角色、续扫的断言由单元测试、oracle 全切面核对与真实数据 L3 承担（交付报告「正式运行覆盖边界」）。

手算规则（schema v1.3 §1/§3）：
- 段 k 由端点 3k..3k+3 组成，起笔锚 raw = 1+12k ⟹ 实体 id `segment:1:<1+12k>`；
  第一种情况结算要等后一段三笔：第 3k+5 笔确认于 known_cut = anchor(3k+6)+2 = 12k+27 条。
- 单元区间 = 成员笔方向极值；本流端点即极值，所以 = 段两端价的 [min, max]。
- 完成单元不足三个 ⟹ 无 seed 尝试、无中枢；完成单元归属 NOT_YET_ASSIGNED/NO_LEFT_CENTER_IN_WINDOW，
  进行中单元 NOT_YET_ASSIGNED/UNIT_NOT_COMPLETED，seed_from_self 一律 NOT_EVALUATED。
- 游标 SEEKING_SEED，seed 起点位置 0（u0，完成或进行中都填其 id）；等待位置 2：有进行中单元 ⟹ awaiting_unit_id，
  否则 awaiting_after_unit_id = 末个完成单元、awaiting_offset = 2 − after 位置。
"""
from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent
POINTS = [10, 30, 20, 50, 25, 45, 15, 35, 20, 40, 28, 38, 22]
RES = 10
INPUTS = 43
CUTS = (26, 27, 38, 39, 43)
RECOVERY_CUTS = (27, 43)


def load_tb02d():
    spec = importlib.util.spec_from_file_location("tb02d_generate", HERE.parent / "tb02d/generate-input.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    module.BASE, module.SCALE, module.MIRROR = 2000, 100, 100000
    return module


def dump(value) -> str:
    return json.dumps(value, ensure_ascii=False, indent=2) + "\n"


def units_for(tb02d, direction):
    prices = [tb02d.transform(p * RES, direction) for p in POINTS]
    out = []
    for k in range(4):
        a, b = prices[3 * k], prices[3 * k + 3]
        out.append({"unit_id": f"segment:1:{1 + 12 * k}", "direction": "UP" if b > a else "DOWN",
                    "lo": min(a, b), "hi": max(a, b), "completed_at_input_count": 12 * k + 27})
    return out


def expected_cut(units, n):
    done = [u for u in units if u["completed_at_input_count"] <= n]
    assert len(done) < 3, "本流 (B) 前缀不得出现 seed"
    pending = units[len(done)]
    assignments = [{"unit_id": u["unit_id"], "completed": True, "direction": u["direction"], "lo": str(u["lo"]), "hi": str(u["hi"]),
                    "status": "NOT_YET_ASSIGNED", "reason": "NO_LEFT_CENTER_IN_WINDOW", "seed_from_self": "NOT_EVALUATED"} for u in done]
    assignments.append({"unit_id": pending["unit_id"], "completed": False, "status": "NOT_YET_ASSIGNED",
                        "reason": "UNIT_NOT_COMPLETED", "seed_from_self": "NOT_EVALUATED"})
    if len(done) == 2:      # 等待位置 2 上是进行中单元
        cursor = {"awaiting_after_unit_id": None, "awaiting_offset": None, "awaiting_unit_id": pending["unit_id"]}
    else:                   # 等待位置 2 上没有单元；after 为末个完成单元（无则 null，以 −1 计）
        after = done[-1]["unit_id"] if done else None
        cursor = {"awaiting_after_unit_id": after, "awaiting_offset": str(2 - (len(done) - 1)), "awaiting_unit_id": None}
    cursor.update({"center_id": None, "seed_start_unit_id": units[0]["unit_id"], "state": "SEEKING_SEED"})
    return {"input_count": n, "input_frontier": str(n - 1), "completed_units": [u["unit_id"] for u in done],
            "in_progress_unit": pending["unit_id"], "assignments": assignments, "scan_cursor": cursor,
            "waiting_reasons": sorted({a["reason"] for a in assignments}),
            "center_kinds_absent": ["CC-016.center_seed_attempt", "CC-016.center", "CC-017.center_roles",
                                    "CC-055.center_relation", "CC-055.center_change"]}


def self_check(raw, oracle):
    for name, rows in raw["cases"].items():
        assert len(rows) == INPUTS
        q = [r["open"] for r in rows]
        extrema = [i for i in range(1, len(q) - 1) if (q[i] - q[i - 1]) * (q[i] - q[i + 1]) > 0]
        assert extrema == [1 + 4 * j for j in range(len(extrema))], name   # 端点只在 anchor，相邻间隔 4
        for cut in oracle["cases"][name]["cuts"].values():
            ids = [x["unit_id"] for x in cut["assignments"]]
            assert len(ids) == len(set(ids)) and sum(not x["completed"] for x in cut["assignments"]) == 1
            c = cut["scan_cursor"]
            assert (c["awaiting_unit_id"] is None) != (c["awaiting_offset"] is None)
    return {"cases": len(raw["cases"]), "inputs": INPUTS, "cuts": list(CUTS)}


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--output", type=Path, help="写固定输入与账；默认只自检")
    a = ap.parse_args()
    tb02d = load_tb02d()
    raw = {"schema": "tb02e-raw-ledger/1", "cases": {}}
    oracle = {"schema": "tb02e-hand-oracle/1", "ticket": 1419, "design_points": POINTS, "res": RES,
              "price_transform": {"base": tb02d.BASE, "scale": tb02d.SCALE, "mirror": tb02d.MIRROR},
              "provenance": "手算规则见生成器文档；仅由设计端点生成，未导入生产输出",
              "scope": "根裁 D-E (B)：正式运行只到第 43 条（2 个完成段 + 进行中段），不出 seed/中枢",
              "cut_convention": "cuts 键为已接纳 raw 条数 N；input_frontier = N−1",
              "recovery_cuts": list(RECOVERY_CUTS), "cases": {}}
    for direction in ("UP", "DOWN"):
        name = "min12_" + direction.lower()
        raw["cases"][name] = tb02d.raw_rows({"points": [p * RES for p in POINTS]}, direction)[:INPUTS]
        units = units_for(tb02d, direction)
        oracle["cases"][name] = {"direction": direction, "units": units,
                                 "cuts": {str(n): expected_cut(units, n) for n in CUTS}}
    report = self_check(raw, oracle)
    if a.output:
        a.output.mkdir(parents=True, exist_ok=True)
        for filename, value in (("raw-ledger.json", raw), ("hand-oracle.json", oracle)):
            (a.output / filename).write_text(dump(value), encoding="utf-8")
        (a.output / "MANIFEST.json").write_text(dump({
            "schema": "tb02e-fixture-manifest/1", "ticket": 1419, "inputs": INPUTS, "cases": list(raw["cases"]),
            "artifacts": {n: hashlib.sha256((a.output / n).read_bytes()).hexdigest() for n in ("raw-ledger.json", "hand-oracle.json")},
            "generator_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
            "tb02d_generator_sha256": hashlib.sha256((HERE.parent / "tb02d/generate-input.py").read_bytes()).hexdigest(),
            "provenance": oracle["provenance"], "scope": oracle["scope"]}), encoding="utf-8")
    print(dump(report), end="")


if __name__ == "__main__":
    main()
