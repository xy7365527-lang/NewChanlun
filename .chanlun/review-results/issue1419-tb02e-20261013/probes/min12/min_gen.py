#!/usr/bin/env python3
"""#1419 D-E 改裁 (A)：12 笔最小流（3 个完成段 + 1 个进行中段），≤52 条单事件内出一个 seed 中枢。

不 import 项目实现；原始 K 线照 TB-02-D 生成器的 raw_rows（每笔 4 根 = 新笔下限：stroke.rs raw_between=y−x−1≥3、merged_gap≥3）。
设计点（设计单位，×RES 后进 TB-02-D 的 BASE+SCALE·p 变换）：
  u0 UP   p0..p3  = 10,30,20,50     特征序列（下行笔）s1=[20,30] s3=[25,50] s5=[15,45]：s3 顶分型，s3.low 25 < s1.high 30 无缺口 ⟹ 第一种情况
  u1 DOWN p3..p6  = 50,25,45,15     特征序列（上行笔）s4=[25,45] s6=[15,35] s8=[20,40]：s6 底分型，s6.high 35 > s4.low 25 无缺口
  u2 UP   p6..p9  = 15,35,20,40     特征序列 s7=[20,35] s9=[28,40] s11=[22,38]：s9 顶分型，s9.low 28 < s7.high 35 无缺口
  u3 DOWN p9..p12 = 40,28,38,22     进行中（其三笔即 u2 的确认笔）
seed（u0,u1,u2）：ZD=max(10,15,15)=15 < ZG=min(50,50,40)=40；Zn = u0,u2 ⟹ GG=max(50,40)=50、DD=min(10,15)=10。
"""
import argparse
import importlib.util
import json
from pathlib import Path

WT = Path("/private/tmp/goal1323-native-20261012/tb1419-wip-wt")
GEN = WT / "s_session/tests/fixtures/tb02d/generate-input.py"
RES = 10
POINTS = [10, 30, 20, 50, 25, 45, 15, 35, 20, 40, 28, 38, 22]
UNITS = [(0, 3), (3, 6), (6, 9)]


def load_tb02d():
    spec = importlib.util.spec_from_file_location("tb02d_generate", GEN)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def iv(p, i):
    """第 i 笔的价格区间。"""
    return (min(p[i], p[i + 1]), max(p[i], p[i + 1]))


def design_checks(p):
    """把文件头的手算条件逐条写成断言（设计自检，不是 parser 的替身）。"""
    c = []
    for (a, b) in UNITS + [(9, 12)]:
        up = p[b] > p[a]
        c.append((f"u[{a}..{b}] 方向交替的三笔", all((p[k + 1] > p[k]) == (up if (k - a) % 2 == 0 else not up) for k in range(a, b))))
    # 三个完成段的特征序列分型（第一种情况：前两元素无缺口；三元素无包含）
    for (a, b), top in zip(UNITS, (True, False, True)):
        e0, e1, e2 = iv(p, a + 1), iv(p, b), iv(p, b + 2)
        if top:
            frac = e1[1] > e0[1] and e1[1] > e2[1] and e1[0] > e0[0] and e1[0] > e2[0]
            nogap = e1[0] < e0[1]
        else:
            frac = e1[0] < e0[0] and e1[0] < e2[0] and e1[1] < e0[1] and e1[1] < e2[1]
            nogap = e1[1] > e0[0]
        inc = lambda x, y: (x[0] <= y[0] and x[1] >= y[1]) or (y[0] <= x[0] and y[1] >= x[1])
        c.append((f"u[{a}..{b}] 特征序列{'顶' if top else '底'}分型", frac))
        c.append((f"u[{a}..{b}] 前两元素无缺口（第一种情况）", nogap))
        c.append((f"u[{a}..{b}] 特征元素无包含", not inc(e0, e1) and not inc(e1, e2)))
    units = [(min(p[a], p[b]), max(p[a], p[b])) for a, b in UNITS]
    zd, zg = max(u[0] for u in units), min(u[1] for u in units)
    c.append(("seed ZD<ZG", zd < zg))
    bad = [n for n, ok in c if not ok]
    assert not bad, bad
    return {"checks": len(c), "units": units, "ZD": zd, "ZG": zg,
            "GG": max(units[0][1], units[2][1]), "DD": min(units[0][0], units[2][0])}


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--out", type=Path, required=True)
    a = ap.parse_args()
    design = design_checks(POINTS)
    tb02d = load_tb02d()
    tb02d.BASE, tb02d.SCALE, tb02d.MIRROR = 2000, 100, 100000
    points = [x * RES for x in POINTS]
    rows = tb02d.raw_rows({"points": points}, "UP")
    endpoint_prices = [tb02d.transform(points[i], "UP") for i in (0, 3, 6, 9, 12)]
    scale = lambda v: tb02d.transform(v * RES, "UP")
    seed = {k: scale(v) for k, v in design.items() if k in ("ZD", "ZG", "GG", "DD")}
    out = {"schema": "tb1419-s0b-input/1", "admission_evidence": False, "stream": "MIN12", "direction": "UP",
           "segment_endpoints": [POINTS[i] for i in (0, 3, 6, 9, 12)], "segment_endpoint_prices": endpoint_prices,
           "stroke_points_design_units": points, "strokes_per_unit": [3, 3, 3, 3], "units_fallback_5_strokes": [],
           "design_checks": design["checks"], "expected_seed_prices": seed,
           "expected": {"completed_segments": 3, "first_formed_seed_at_input": 1 + 4 * 12 + 2,
                        "center_status": "EXTENDING", "in_progress_unit": "u3"},
           "self_check": {"bars": len(rows), "strokes": len(points) - 1},
           "rows": rows,
           "gate": "parser/S 实测：完成段 3 个、端点 = segment_endpoint_prices[:4]；第 51 条起 seed FORMED、中枢 EXTENDING，ZD/ZG/GG/DD = expected_seed_prices"}
    a.out.write_text(json.dumps(out, ensure_ascii=False, indent=1) + "\n")
    print(json.dumps({k: out[k] for k in ("segment_endpoint_prices", "expected_seed_prices", "expected", "self_check", "design_checks")}, ensure_ascii=False))


if __name__ == "__main__":
    main()
