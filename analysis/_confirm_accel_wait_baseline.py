"""嵌套确认的严格随机基线 + 尾部 moved 分布（读事件缓存，免重跑信号层）。

修正两处：
1. random_gap（中位到达间隔）低估随机等待——事件成簇时 inspection paradox 使
   随机时点的等待远大于中位间隔。严格基线 = 全体 bar 到下一事件的等待分布
   （时间平均等待），与 T_B(true) 同分布族可比。
2. moved_A/moved_B 补 p90/max——任务关心的"等确认时回调已走完"是尾部场景。
"""

from __future__ import annotations

import bisect
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

from confirmation_acceleration_1s import aggregate_1min, load_btc, load_es  # noqa: E402

CACHE = ROOT / "analysis" / "data_cache"


def q(xs, p):
    if not xs:
        return float("nan")
    xs = sorted(xs)
    return float(xs[min(len(xs) - 1, int(p * len(xs)))])


for sym in (sys.argv[1:] or ["BTC", "ES"]):
    d1s = load_btc() if sym == "BTC" else load_es()
    d1m = aggregate_1min(d1s)
    ev1s = json.loads((CACHE / f"_confirm_events_full_{sym.lower()}_1s.json").read_text())
    ev1m = json.loads((CACHE / f"_confirm_events_full_{sym.lower()}_1m.json").read_text())
    ts1s, closes1s, last1s = d1s["ts"], d1s["closes"], d1m["last1s"]
    n = len(ts1s)

    print(f"===== {sym} =====")
    for K in (3, 4):
        idxs = sorted(b for (b, lad, _k, side, _sg, conf) in ev1s
                      if lad >= K and side == "sell" and conf)
        # 时间平均等待：每个 bar 到下一事件的秒数（无后续事件的尾段丢弃）
        waits = []
        j = 0
        for i in range(0, n, 7):           # 1/7 采样足够（分布估计）
            while j < len(idxs) and idxs[j] < i:
                j += 1
            if j >= len(idxs):
                break
            waits.append(ts1s[idxs[j]] - ts1s[i])
        print(f"  K>={K}: 随机时点等待 p50={q(waits,.5):.0f}s "
              f"p90={q(waits,.9):.0f}s (事件数={len(idxs)})")

    # 高层 candidate 的 moved 尾部
    for hl in (2, 3):
        cand, conf = {}, {}
        for (bar, lad, kind, side, seg_idx, confirmed) in ev1m:
            if lad != hl or side != "sell":
                continue
            key = (kind, seg_idx)
            (conf if confirmed else cand).setdefault(key, bar)
        ma, mb3, ta_list = [], [], []
        nest3 = sorted(b for (b, lad, _k, side, _sg, c) in ev1s
                       if lad >= 3 and side == "sell" and c)
        for key, cbar in cand.items():
            if key not in conf or conf[key] < cbar:
                continue
            a1s = last1s[cbar]
            apx = closes1s[a1s]
            cf1s = last1s[conf[key]]
            ma.append((apx - closes1s[cf1s]) / apx * 100)
            ta_list.append(ts1s[cf1s] - ts1s[a1s])
            jj = bisect.bisect_right(nest3, a1s)
            if jj < len(nest3):
                mb3.append((apx - closes1s[nest3[jj]]) / apx * 100)
        print(f"  1min ladder{hl} (n={len(ma)}): "
              f"T_A p50={q(ta_list,.5):.0f}s p90={q(ta_list,.9):.0f}s")
        print(f"    moved_A%: p50={q(ma,.5):.3f} p90={q(ma,.9):.3f} "
              f"max={max(ma) if ma else float('nan'):.3f}")
        print(f"    moved_B%(K>=3): p50={q(mb3,.5):.3f} p90={q(mb3,.9):.3f} "
              f"max={max(mb3) if mb3 else float('nan'):.3f}")
