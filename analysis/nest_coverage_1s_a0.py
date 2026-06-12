"""1s a0 nest 覆盖率对照回测（fusion_v 修正版：1s 塔 vs 1min 塔，同窗同标的）。

上游：
- fusion_v 修正版判决 §4(d)（`fusion_v_no_r4_literal_short_results.md`）：
  瓶颈不是 nest 定位质量，是 nest 覆盖率（ES 40% / QQQ 28% / BTC 48%，全史 1min）。
  confirmed 时点翻空是失血主体，nest 时点空腿 3/3 近打平或正。
- 1s 确认加速测量（`1s_confirmation_acceleration.md`）：同尺度原生滞后不被 a0
  细化压缩（−11%~−24%），加速只来自区间套路径（2.8-3.0×）；
  事件密度锚：1s ladder4 ≈ 1min ladder3（两塔同号 ladder 不是同一尺度）。

══════════════ 预注册判据（先于数据声明）══════════════

对照构造：同一 1s 序列按墙钟分钟桶聚合出 1min 塔（同窗同标的同数据源，
唯一变量 = a0 分辨率）。操作床位对齐：1min 塔 floor=2（LADDER_SEG，在册口径）；
1s 塔 floor=3 与 floor=4 双臂——密度锚给出 1min ladder2 ≈ 1s ladder3.5，
整数错位无解析解，两个相邻整数都报告（不是扫参，是错位的诚实表达）。
1s floor=2 仅在两周窗口作 P4 反例对照（操作床位不对齐时交易爆炸的量化）。

P1：ES 在 1s 塔（f3 或 f4）下空头 P&L 较同窗 1min 塔翻正或大幅改善。
P2：BTC 在 1s 塔下 strat_pct 不劣于同窗 1min 塔（容差：符号不翻 + MDD 不显著加深）。
P3：nest 翻空覆盖率（nest_sell 翻空腿 / 全部翻空腿）从 1min 塔基线提高到 60%+。
P4：匹配床位（f3/f4）下交易次数与 1min 塔同数量级（1s 是观测层不是操作层）；
    f2 反例臂预期交易爆炸。

摩擦口径：零摩擦（主，两塔可比）+ maker 面（BTC 2bps/侧 Binance VIP0；
ES 0.5bps/侧 ≈ 半 tick+费用；CL 1bps/侧 ≈ 半 tick+费用）。

认识论等级：两周窗口 = L2（管线验证 + 方向观测，n=1 无深回调 regime）；
ES 1y / CL 1y = L2→L3 边缘（单标的多 regime 年窗，含 2026-02/04 回调段）。

诚实声明（090号）：两周窗口的 nest 覆盖率与全史在册 28-48% 不同窗不可直接比；
本实验的测量量是**同窗 1s vs 1min 差分**，全史数字仅作背景。

════════════════════════════════════════════════════════════

用法：PYTHONPATH=src .venv/bin/python analysis/nest_coverage_1s_a0.py KEY [KEY ...]
KEY ∈ {BTC2W, ES2W, ES1Y, CL1Y}
输出：analysis/data_cache/nest_cov_1s_<KEY>.json
"""

from __future__ import annotations

import json
import sys
import time
from bisect import bisect_right
from calendar import timegm
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import newchan_rust as nr  # noqa: E402

from fugue_version_i import LADDER_SEG  # noqa: E402
from organic_fugue_rust_check import pack_tape  # noqa: E402
from organic_signals import compute_organic_signals  # noqa: E402
from universal_combination_backtest import analyze, bh_mdd  # noqa: E402

DATA_DIR = ROOT / "analysis" / "data_cache"
INIT = 100_000.0
FLIP_REASONS = ("sellpt", "nest_sell", "t2w_sell")

# 数据集注册：KEY → (文件, 1s格式, maker单侧摩擦率)
DATASETS = {
    "BTC2W": ("btc_1s_2week.json", "binance", 0.0002),
    "ES2W": ("es_1s_2week.json", "databento", 0.00005),
    "ES1Y": ("es_1s_databento_1y.json", "databento", 0.00005),
    "CL1Y": ("cl_1s_databento_1y.json", "databento", 0.0001),
}

YEAR_STARTS_NS = [(y, timegm((y, 1, 1, 0, 0, 0)) * 1_000_000_000)
                  for y in range(2020, 2028)]


def load_1s(key: str) -> tuple[list, list, list, list, list, list, dict]:
    """→ (opens, highs, lows, closes, years, minute_keys, meta)。"""
    fname, fmt, _ = DATASETS[key]
    raw = json.loads((DATA_DIR / fname).read_text())
    o, h, l, c = raw["opens"], raw["highs"], raw["lows"], raw["closes"]
    if fmt == "binance":
        dates = raw["dates"]
        years = [int(d[:4]) for d in dates]
        minutes = [d[:16] for d in dates]
        rng = [dates[0], dates[-1]]
    else:
        ts = raw["timestamps_ns"]
        bounds = [ns for _, ns in YEAR_STARTS_NS]
        years = [YEAR_STARTS_NS[bisect_right(bounds, t) - 1][0] for t in ts]
        minutes = [t // 60_000_000_000 for t in ts]
        rng = [raw["start"], raw["end"]]
    return o, h, l, c, years, minutes, {"file": fname, "range": rng}


def aggregate_1min(o, h, l, c, years, minutes):
    """墙钟分钟桶聚合（同数据源唯一变量 = a0；桶内 first/max/min/last）。"""
    ao, ah, al, ac, ay = [], [], [], [], []
    cur = None
    for i in range(len(c)):
        if minutes[i] != cur:
            cur = minutes[i]
            ao.append(o[i])
            ah.append(h[i])
            al.append(l[i])
            ac.append(c[i])
            ay.append(years[i])
        else:
            ah[-1] = max(ah[-1], h[i])
            al[-1] = min(al[-1], l[i])
            ac[-1] = c[i]
    return ao, ah, al, ac, ay


def friction_face(trades, closes, side: float) -> dict:
    """双极性 NAV 重建（摩擦率参数化）：多头买付/卖收，空头开收/补付。"""
    n = len(closes)
    dshares = [0.0] * (n + 1)
    dcash = [0.0] * (n + 1)
    for (lad, eb, ep, xb, xp, sh, w, dfr, part, reason, pol) in trades:
        if pol == "long":
            dshares[eb] += sh
            dshares[xb] -= sh
            dcash[eb] -= sh * ep * (1 + side)
            dcash[xb] += sh * xp * (1 - side)
        else:
            dshares[eb] -= sh
            dshares[xb] += sh
            dcash[eb] += sh * ep * (1 - side)
            dcash[xb] -= sh * xp * (1 + side)
    shares = 0.0
    pool = INIT
    nav = INIT
    peak = mdd = 0.0
    for i in range(n):
        shares += dshares[i]
        pool += dcash[i]
        nav = pool + shares * closes[i]
        peak = max(peak, nav)
        mdd = min(mdd, nav / peak - 1.0)
    return {"strat_pct": round((nav / INIT - 1) * 100, 2),
            "mdd_pct": round(mdd * 100, 2)}


def nest_coverage(trades) -> dict:
    """翻空腿按入场触发词汇分桶（覆盖率 = nest_sell 腿占全部翻空腿）。

    空腿只由翻转断面产生（unified_voice.rs 阶段A：平多与开空同 bar 同价
    两行 trade）⇒ 空腿入场触发 = 同 (ladder, bar) 多头出场行的 exit_reason。
    """
    flip_at = {}
    for (lad, eb, ep, xb, xp, sh, w, dfr, part, reason, pol) in trades:
        if pol == "long" and reason in FLIP_REASONS:
            flip_at[(lad, xb)] = reason
    buckets: dict = defaultdict(
        lambda: {"n": 0, "pnl": 0.0, "win": 0, "hold_bars": 0})
    for (lad, eb, ep, xb, xp, sh, w, dfr, part, reason, pol) in trades:
        if pol != "short":
            continue
        trig = flip_at.get((lad, eb), "unknown")
        b = buckets[trig]
        b["n"] += 1
        b["pnl"] += sh * (ep - xp)
        b["win"] += sh * (ep - xp) > 0
        b["hold_bars"] += xb - eb
    buckets = dict(buckets)
    total = sum(b["n"] for b in buckets.values())
    nest_n = buckets["nest_sell"]["n"] if "nest_sell" in buckets else 0
    out = {
        "n_short_legs": total,
        "coverage_nest": round(nest_n / total, 3) if total else None,
        "by_trigger": {
            k: {"n": v["n"], "pnl": round(v["pnl"], 0),
                "win_rate": round(v["win"] / v["n"], 3),
                "avg_hold_bars": round(v["hold_bars"] / v["n"], 0)}
            for k, v in sorted(buckets.items())},
        "short_pnl_total": round(sum(b["pnl"] for b in buckets.values()), 0),
    }
    return out


def build_tape(bars) -> tuple:
    """信号层一次计算，跨 floor 臂复用（1y 窗口的计算量约束）。"""
    opens, highs, lows, closes = bars
    t0 = time.time()
    dir_flips: list = []
    trend_flips: list = []
    tape = compute_organic_signals(opens, highs, lows, closes,
                                   dir_flips=dir_flips,
                                   trend_flips=trend_flips)
    fp = {"bsp": sum(len(s.bsp_events[lv]) for s in tape if s.bsp_events
                     for lv in range(11)),
          "div": sum(len(s.div_events[lv]) for s in tape if s.div_events
                     for lv in range(11)),
          "flips": len(dir_flips), "tflips": len(trend_flips)}
    rtape = pack_tape(tape, dir_flips=dir_flips, trend_flips=trend_flips)
    return rtape, fp, round(time.time() - t0, 1)


def run_arm(label: str, floor: int, rtape, fp, t_sig, closes, years,
            maker_side: float, bar_seconds: int) -> dict:
    n = len(closes)
    t1 = time.time()
    res = nr.run_positional_rust(rtape, floor_ladder=floor, mode="fusion_v")
    t_run = time.time() - t1
    a = analyze(res, closes, years)
    cov = nest_coverage(res["trades"])
    lead = a["gates"]["nest_lead_avg_bars"]
    out = {
        "label": label, "floor": floor, "n_bars": n, "tape_fp": fp,
        "strat_pct": a["strat_pct"], "mdd_pct": a["mdd_pct"],
        "n_trades": a["n_trades"],
        "short_cash_total": a["short_cash_total"],
        "exit_reasons": a["exit_reasons"],
        "yearly": a["yearly"], "regime": a["regime"],
        "gates": a["gates"],
        "nest_lead_avg_min": (round(lead * bar_seconds / 60, 1)
                              if lead else None),
        "nest_coverage": cov,
        "friction": {
            "book_0p05": friction_face(res["trades"], closes, 0.0005),
            "maker": {"side": maker_side,
                      **friction_face(res["trades"], closes, maker_side)},
        },
        "timing_s": {"signals": t_sig, "fsm": round(t_run, 1)},
    }
    print(f"  [{label}] bars={n:,} sig={t_sig:.0f}s strat={a['strat_pct']:+.1f}% "
          f"mdd={a['mdd_pct']}% trades={a['n_trades']} "
          f"short_pnl={cov['short_pnl_total']} cov={cov['coverage_nest']} "
          f"triggers={ {k: v['n'] for k, v in cov['by_trigger'].items()} }",
          flush=True)
    return out


def run_key(key: str) -> dict:
    fname, fmt, maker_side = DATASETS[key]
    print(f"[{key}] 载入 {fname}", flush=True)
    o, h, l, c, years, minutes, meta = load_1s(key)
    ao, ah, al, ac, ay = aggregate_1min(o, h, l, c, years, minutes)
    del minutes
    bh_1s = (c[-1] / c[0] - 1) * 100
    print(f"[{key}] 1s bars={len(c):,} → 1min bars={len(ac):,} "
          f"BH={bh_1s:+.2f}% 范围 {meta['range']}", flush=True)

    out = {"key": key, **meta,
           "bh_pct": round(bh_1s, 2),
           "bh_mdd_pct": round(bh_mdd(c) * 100, 2),
           "design": "1s vs 1min 同窗对照（判据见 docstring）",
           "arms": {}}
    floors_1s = [3, 4] + ([LADDER_SEG] if key.endswith("2W") else [])
    # P4 反例对照臂 f2 仅两周窗口（床位不对齐时交易爆炸的量化）
    groups = [((ao, ah, al, ac), ay, 60, [("1min_f2", LADDER_SEG)]),
              ((o, h, l, c), years, 1,
               [(f"1s_f{fl}" + ("_naive" if fl == LADDER_SEG else ""), fl)
                for fl in floors_1s])]
    for bars, yrs, bs, floor_arms in groups:
        rtape, fp, t_sig = build_tape(bars)
        closes_g = bars[3]
        for label, floor in floor_arms:
            out["arms"][label] = run_arm(label, floor, rtape, fp, t_sig,
                                         closes_g, yrs, maker_side, bs)
            (DATA_DIR / f"nest_cov_1s_{key}.json").write_text(
                json.dumps(out, ensure_ascii=False, indent=1))
        del rtape

    base = out["arms"]["1min_f2"]
    verd = {}
    for lab in ("1s_f3", "1s_f4"):
        arm = out["arms"][lab]
        verd[lab] = {
            "P1_short_pnl_delta": round(
                arm["nest_coverage"]["short_pnl_total"]
                - base["nest_coverage"]["short_pnl_total"], 0),
            "P1_short_pnl_sign_flip": (
                base["nest_coverage"]["short_pnl_total"] < 0
                <= arm["nest_coverage"]["short_pnl_total"]),
            "P2_strat_delta_pp": round(
                arm["strat_pct"] - base["strat_pct"], 1),
            "P3_coverage": [base["nest_coverage"]["coverage_nest"],
                            arm["nest_coverage"]["coverage_nest"]],
            "P3_ge_60pct": (arm["nest_coverage"]["coverage_nest"] or 0) >= 0.6,
            "P4_trades_ratio": round(
                arm["n_trades"] / max(1, base["n_trades"]), 2),
        }
    out["verdicts"] = verd
    (DATA_DIR / f"nest_cov_1s_{key}.json").write_text(
        json.dumps(out, ensure_ascii=False, indent=1))
    print(f"[{key}] verdicts={json.dumps(verd)}", flush=True)
    return out


def main() -> None:
    keys = sys.argv[1:] or ["BTC2W", "ES2W"]
    for key in keys:
        if key not in DATASETS:
            sys.exit(f"未知 KEY：{key}（可选 {sorted(DATASETS)}）")
        if not (DATA_DIR / DATASETS[key][0]).exists():
            print(f"[{key}] 数据文件缺失，跳过：{DATASETS[key][0]}", flush=True)
            continue
        run_key(key)


if __name__ == "__main__":
    main()
