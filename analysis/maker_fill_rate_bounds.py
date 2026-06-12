"""CL 1s maker fill rate 的 OHLCV 可验证上下界。

对 cl_1s_maker_fills.json 中全部 5,120 个 fill（信号磁带产物，零改动）：
在 fill bar 的 close 价 p 挂限价单，问真实 1s 行情在其后 W 秒内：

- touch 率  = P(价格触及挂单价)        —— fill rate 的上界（触及即成交 = 队列首位）
- cross 率  = P(价格穿越挂单价 ≥1 tick) —— fill rate 的下界（穿越必成交 = 队列末位）

真实 fill rate ∈ [cross, touch]，落点由队列位置决定（co-lo/延迟敏感的唯一轴）。
窗口按真实时间（timestamps_ns）计，非 bar 数——1s bar 在静秒缺失。

两种窗口口径：
- fixed：固定 W 秒（执行自由度）
- capped：min(W, 距同一 trade 内下一 fill 的间隔)（策略一致窗口，
  挂单不能跨越下一个信号事件）

输出：analysis/data_cache/cl_1s_fill_rate_bounds.json + stdout 表。
认识论等级：L2（真实 1s OHLCV 上的频率统计）——但仅界定区间端点，
区间内落点（队列位置）仍是 L0，需 MBP-10。
"""
from __future__ import annotations

import json
import math
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
DATA = HERE / "data_cache"
TICK = 0.01
WINDOWS = [1, 2, 5, 10, 30, 60, 120, 300]  # 秒


def load_filtered():
    """复刻 cl_1s_maker_model_backtest.load_with_days 的过滤，额外保留 ts。"""
    raw = json.loads((DATA / "cl_1s_databento_1y.json").read_text())
    o = np.asarray(raw["opens"], dtype=np.float64)
    h = np.asarray(raw["highs"], dtype=np.float64)
    l = np.asarray(raw["lows"], dtype=np.float64)
    c = np.asarray(raw["closes"], dtype=np.float64)
    ts = np.asarray(raw["timestamps_ns"], dtype=np.int64)
    del raw
    ok = ~(np.isnan(o) | np.isnan(h) | np.isnan(l) | np.isnan(c)
           | (o <= 0) | (h <= 0) | (l <= 0) | (c <= 0))
    o, h, l, c, ts = o[ok], h[ok], l[ok], c[ok], ts[ok]
    # spike-revert 过滤（与引擎侧逐字同判据）
    r1 = np.abs(c[1:-1] / c[:-2] - 1) > 0.5
    r2 = np.abs(c[2:] / c[:-2] - 1) < 0.05
    drop = np.zeros(len(c), dtype=bool)
    drop[1:-1] = r1 & r2
    keep = ~drop
    return h[keep], l[keep], c[keep], ts[keep]


def iter_fills(trades):
    for tr in trades:
        fills = [tr["entry"], tr["exit"]]
        for p in tr["pairs"]:
            fills.extend([p["open"], p["close"]])
        fills.sort(key=lambda f: f["bar"])
        for i, f in enumerate(fills):
            nxt = fills[i + 1]["bar"] if i + 1 < len(fills) else None
            yield f, nxt


def main():
    cache = json.loads((DATA / "cl_1s_maker_fills.json").read_text())["CL_1S"]
    highs, lows, closes, ts = load_filtered()
    if len(closes) != cache["n_bars"]:
        raise SystemExit(
            f"bar 对齐失败: filtered={len(closes)} vs cache={cache['n_bars']}")

    fills = list(iter_fills(cache["trades"]))
    # 守卫：fill 价必须等于该 bar close（限价挂在信号价）
    mism = sum(1 for f, _ in fills
               if abs(f["p"] - closes[f["bar"]]) > 1e-9)
    if mism:
        raise SystemExit(f"fill 价 ≠ bar close: {mism}/{len(fills)}")

    out = {"n_fills": len(fills), "windows_s": WINDOWS,
           "fixed": {}, "capped": {}, "by_kind": {}}
    n = len(closes)
    for W in WINDOWS:
        res = {"fixed": [0, 0], "capped": [0, 0]}  # [touch, cross]
        kind_acc: dict[str, list[int]] = {}
        for f, nxt_bar in fills:
            b, p, side = f["bar"], f["p"], f["side"]
            # 固定窗口右端：真实时间 W 秒
            j_fix = int(np.searchsorted(ts, ts[b] + W * 1_000_000_000,
                                        side="right"))
            for mode, j_end in (("fixed", j_fix),
                                ("capped",
                                 min(j_fix, nxt_bar if nxt_bar else n))):
                lo, hi = b + 1, min(j_end, n)
                if lo >= hi:
                    continue
                if side > 0:  # 买单：low 触及/下穿
                    seg = lows[lo:hi]
                    touch = bool((seg <= p).any())
                    cross = bool((seg <= p - TICK + 1e-9).any())
                else:         # 卖单：high 触及/上穿
                    seg = highs[lo:hi]
                    touch = bool((seg >= p).any())
                    cross = bool((seg >= p + TICK - 1e-9).any())
                res[mode][0] += touch
                res[mode][1] += cross
                if mode == "capped":
                    acc = kind_acc.setdefault(f["kind"], [0, 0, 0])
                    acc[0] += touch
                    acc[1] += cross
                    acc[2] += 1
        m = len(fills)
        out["fixed"][W] = {"touch": res["fixed"][0] / m,
                           "cross": res["fixed"][1] / m}
        out["capped"][W] = {"touch": res["capped"][0] / m,
                            "cross": res["capped"][1] / m}
        out["by_kind"][W] = {k: {"touch": v[0] / v[2], "cross": v[1] / v[2],
                                 "n": v[2]}
                             for k, v in kind_acc.items()}

    (DATA / "cl_1s_fill_rate_bounds.json").write_text(
        json.dumps(out, indent=1))

    print(f"n_fills={out['n_fills']}  (touch=上界/队列首位, cross=下界/队列末位)")
    print(f"{'W(s)':>6} | {'fixed touch':>11} {'fixed cross':>11} | "
          f"{'capped touch':>12} {'capped cross':>12}")
    for W in WINDOWS:
        fx, cp = out["fixed"][W], out["capped"][W]
        print(f"{W:>6} | {fx['touch']:>10.1%} {fx['cross']:>11.1%} | "
              f"{cp['touch']:>11.1%} {cp['cross']:>12.1%}")
    print("\nby kind (capped):")
    for W in (5, 30, 60):
        row = "  ".join(f"{k}: {v['touch']:.0%}/{v['cross']:.0%}(n={v['n']})"
                        for k, v in sorted(out["by_kind"][W].items()))
        print(f"  W={W:>3}s  {row}")


if __name__ == "__main__":
    main()
