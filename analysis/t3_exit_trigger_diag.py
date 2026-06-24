#!/usr/bin/env python3
"""#164 R2 T3 出场 leak 可约性诊断（observation-only）。

对 Face A 基线（默认，无 env），跑 RecTStream.finish_full()，用 leg_trades + exit_trigger_log
（同序）+ level_segments 分类每笔 r≤0 交易为 {T0,T1,TC,T2,T3}（复用 capture_ratio_matrix
的决策树），然后对 T3 笔统计**出场触发类型分布**（type1/type2/type3/止损/翻转/强平）。

判据（574 floor 可约性）：
- T3 主要被 type1（=走势完成=最小确认滞后 574 floor）平 ⇒ leak 在 floor 内**不可约**。
- T3 主要被 type2/type3（确认滞后 ⊋ type1）平 ⇒ floor 之上**可约**（应优先 type1 出场）。

用法：
    python analysis/t3_exit_trigger_diag.py --symbols BTC,CL,GC,ES
"""
import argparse
import sys
from pathlib import Path
from collections import Counter

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from analysis.capture_ratio_matrix import (  # noqa: E402
    load_clean_ohlc, SYMBOLS, MAIN_DATA_DIR, DATA_DIR, CHURN_HOLD_BARS,
    _seg_by_level, _find_entry_seg,
)
import newchan_rust as nr  # noqa: E402

TRIG = {0: "type1(走势完成/floor)", 1: "type2", 2: "type3", 3: "止损",
        4: "节点翻转", 5: "强平", 6: "其他"}


def run(o, h, l, c, mode="structural"):
    pm = {"structural": "structural", "and": "and", "or": "or"}[mode]
    s = nr.RecTStream(pm)
    for a, b, d, e in zip(o, h, l, c):
        s.push_bar(a, b, d, e)
    return s.finish_full()


def classify_and_diag(res):
    trades = res["leg_trades"]
    trig = res["exit_trigger_log"]
    assert len(trades) == len(trig), f"同序断言失败 trades={len(trades)} trig={len(trig)}"
    seg_by = _seg_by_level(res["level_segments"])
    top_level = max(seg_by) if seg_by else 0
    EPS = 1e-9

    t3_trig = Counter()      # T3 笔出场触发分布
    t3_trig_pnl = Counter()  # T3 笔出场触发 pnl 聚合
    all_trig = Counter()
    n_t3 = 0
    for (k, eb, xb, ep, xp, u, is_short, pnl), tg in zip(trades, trig):
        per_unit = pnl / u if u > EPS else pnl
        if is_short:
            mn = ep  # 简化：covered 需 H/L，这里只用 sign(pnl)=sign(r) 判 r≤0
        r_le0 = pnl <= 0
        all_trig[tg] += 1
        if not r_le0:
            continue
        k = int(k); eb = int(eb); xb = int(xb)
        favorable_up = not is_short
        hold = xb - eb
        arr = seg_by.get(k, [])
        seg = _find_entry_seg(arr, eb)
        if seg is None or seg == "OUT":
            continue  # T0
        rs, re, hi, lo, up = seg
        aligned = (up and favorable_up) or ((not up) and (not favorable_up))
        if not aligned:
            continue  # T1
        if hold <= CHURN_HOLD_BARS:
            continue  # TC
        if xb > re:
            # T3
            n_t3 += 1
            t3_trig[tg] += 1
            t3_trig_pnl[tg] += pnl
        # else T2
    return n_t3, t3_trig, t3_trig_pnl, all_trig


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--symbols", default="BTC")
    ap.add_argument("--mode", default="structural")
    args = ap.parse_args()
    file_of = dict(SYMBOLS)
    for sym in args.symbols.split(","):
        sym = sym.strip()
        fn = file_of[sym]
        path = MAIN_DATA_DIR / fn
        if not path.exists():
            path = DATA_DIR / fn
        o, h, l, c = load_clean_ohlc(path)
        res = run(o, h, l, c, args.mode)
        n_t3, t3_trig, t3_trig_pnl, all_trig = classify_and_diag(res)
        print(f"\n=== {sym} ({args.mode}) Face A 基线 T3={n_t3} ===")
        for tg in sorted(t3_trig):
            print(f"  T3 出场触发 [{TRIG[tg]:18}] = {t3_trig[tg]:4} 笔  pnl={t3_trig_pnl[tg]:+.0f}")
        floor = t3_trig.get(0, 0)
        reducible = sum(v for kk, v in t3_trig.items() if kk in (1, 2))
        print(f"  → 574 floor(type1)={floor} ({floor/max(n_t3,1):.1%}) | "
              f"可约(type2/3)={reducible} ({reducible/max(n_t3,1):.1%}) | "
              f"止损={t3_trig.get(3,0)} 翻转={t3_trig.get(4,0)} 强平={t3_trig.get(5,0)} 其他={t3_trig.get(6,0)}")


if __name__ == "__main__":
    raise SystemExit(main())
