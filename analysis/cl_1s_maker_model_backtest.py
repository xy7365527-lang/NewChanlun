"""CL 秒级 a0 maker 执行模型回测 — 摩擦×fill_rate 可行域 + 杠杆分析。

═══════════════════════════════════════════════════════════════════════
任务（2026-06-12 编排者）：验证 maker 执行能否解封 1s a0 的短差 alpha
═══════════════════════════════════════════════════════════════════════
前置判决（cl_1s_a0_backtest_results.md）：1s 结构 alpha 真实（零摩擦优
1min +12pp），但 taker 1.45bps/侧 下全负；盈亏平衡 ≈0.58bps/侧。
本脚本回答：maker 限价成交（低/负摩擦 × 非100% fill rate）下可行域在哪。

严格缠论约束：信号层零改动。模型作用在 run_organic_rust diag 的逐 fill
折算上；磁带指纹 fail-fast（与 cl_1s_a0_backtest.py 在册值逐键一致）。

执行模型（每个 fill 侧独立）：
  以概率 fr 成交为 maker：摩擦 = f_fill（bps/侧，可负=rebate）
  以概率 1−fr 未成交：
    worst（追价）：下一 bar close 处 taker 成交——
      摩擦 = F_TAKER + chase；chase = max(0, 不利漂移)。
      （限价单未成交 ⟺ 价格向不利方向移动，故漂移下截断于 0——
       这是 maker 逆向选择成本的保守下界表达。）
    best（撤单）：开仓侧撤单 → 该单元（主笔/diff对）以概率 fr 存在；
      平仓侧不可撤（持仓必须了结）→ 与 worst 同样追价。
      逐笔取期望值复利（期望值近似声明：忽略成交伯努利方差对复利的
      二阶效应；fills≈5K/年，大数下近似误差远小于报告精度）。

杠杆：日级 mark-to-market 权益曲线（仓位事件+日终收盘标记，与逐笔
现金 pnl 对账 fail-fast），日收益 ×L 后复利；equity≤0 记 RUIN。

费率事实锚（报告对照用）：
  CL 1 tick = $0.01 = 1.43bps @ $70；点差成本（taker 过手）≈1 tick。
  IBKR 固定佣金 + NYMEX 交易所费 ≈ $2.4/手/侧 ≈ 0.035bps（$70k 名义）。
  → IBKR maker（限价不过手点差）：f_fill ≈ +0.04bps/侧，无 rebate。
  CME maker rebate（做市商计划，非散户）：约 −0.2~−0.5bps/侧。
  CL 初始保证金 ≈ $6,000/手 → 最大杠杆 ≈ 名义/保证金 ≈ 11×。

输出：analysis/data_cache/cl_1s_maker_fills.json（stage1 引擎产物缓存）
      analysis/data_cache/cl_1s_maker_model.json（stage2 网格结果）
      analysis/cl_1s_maker_heatmap.png
用法：PYTHONPATH=src .venv/bin/python analysis/cl_1s_maker_model_backtest.py
认识论等级：L2（CL 单标的单时段真实数据；执行层是模型而非逐档撮合——
fill rate 本身未被数据验证，是参数轴）。
"""

from __future__ import annotations

import json
import math
import sys
import time
from bisect import bisect_left
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

DATA_DIR = ROOT / "analysis" / "data_cache"
F_1S = DATA_DIR / "cl_1s_databento_1y.json"
F_1M_WIN = DATA_DIR / "cl_1m_1y_window.json"
F_PREV = DATA_DIR / "cl_1s_a0_backtest.json"
FILLS_JSON = DATA_DIR / "cl_1s_maker_fills.json"
OUT_JSON = DATA_DIR / "cl_1s_maker_model.json"
PNG = ROOT / "analysis" / "cl_1s_maker_heatmap.png"

VARIANT = "V2oa25_ht"
F_TAKER = 1.45e-4  # 追价侧：1 tick 点差 + 佣金（在册物理下限）
F_FILL_BPS = [-0.3, -0.2, -0.1, 0.0, 0.1, 0.25, 0.5, 0.75, 1.0, 1.5]
FILL_RATES = [0.30, 0.50, 0.70, 0.80, 0.90, 0.95, 1.00]
LEVERAGES = [1, 2, 3, 5, 10]
CL_MARGIN = 6000.0  # USD/手（初始保证金，量级锚）

# 杠杆/日级分析的场景单元（f_fill_bps, fill_rate, 标签）
LEV_SCENARIOS = [
    (0.04, 1.00, "IBKR_maker_fr100"),
    (0.04, 0.95, "IBKR_maker_fr95"),
    (0.04, 0.90, "IBKR_maker_fr90"),
    (0.04, 0.80, "IBKR_maker_fr80"),
    (0.04, 0.70, "IBKR_maker_fr70"),
    (-0.30, 0.95, "MM_rebate_fr95"),
    (1.45, 1.00, "taker_ref"),
]


# ───────────────────────── stage 1：引擎 → fills 缓存 ─────────────────────────

def load_with_days(path: Path):
    """复刻 fugue_v2_full_backtest.load_ohlc 的过滤（nan/≤0 + spike-revert），
    额外返回逐 bar 日键。复刻正确性由磁带指纹守卫兜底。"""
    raw = json.loads(path.read_text())
    o_in, h_in = raw["opens"], raw["highs"]
    l_in, c_in = raw["lows"], raw["closes"]
    if "timestamps_ns" in raw:
        d_in = raw["timestamps_ns"]
        day_of = lambda d: int(d // 86_400_000_000_000)  # noqa: E731
    else:
        d_in = raw["dates"]
        day_of = lambda d: str(d)[:10]  # noqa: E731
    opens, highs, lows, closes, days = [], [], [], [], []
    for i in range(len(c_in)):
        o, h, l, c = (float(o_in[i]), float(h_in[i]),
                      float(l_in[i]), float(c_in[i]))
        if any(math.isnan(x) or x <= 0 for x in (o, h, l, c)):
            continue
        opens.append(o); highs.append(h); lows.append(l); closes.append(c)
        days.append(day_of(d_in[i]))
    drop = {i for i in range(1, len(closes) - 1)
            if abs(closes[i] / closes[i - 1] - 1) > 0.5
            and abs(closes[i + 1] / closes[i - 1] - 1) < 0.05}
    if drop:
        keep = [i for i in range(len(closes)) if i not in drop]
        opens = [opens[i] for i in keep]; highs = [highs[i] for i in keep]
        lows = [lows[i] for i in keep]; closes = [closes[i] for i in keep]
        days = [days[i] for i in keep]
    return opens, highs, lows, closes, days


def extract_trades(diag, closes) -> list[dict]:
    """diag → 自含 fill 记录。每 fill：bar/价/股数/方向/类别 + 追价参照
    p_next（下一 bar close）与单位 chase（不利漂移截断于0）。"""
    n = len(closes)
    trades = []
    for header, diffs in diag:
        eb, ep, xb, xp, _rs, _lad, pnl_pct, _cb, sh, _earn = header
        cap = ep * sh

        def mk(bar, price, qty, side, kind):  # side: +1 buy / −1 sell
            pn = closes[min(bar + 1, n - 1)]
            chase = max(0.0, side * (pn - price))  # $/单位，不利方向截断
            return {"bar": int(bar), "p": price, "sh": qty, "side": side,
                    "kind": kind, "pn": pn, "chase": chase}

        pairs = []
        pf_sum = 0.0
        for (_k, leg, sb, sp, bb, bp), (psh, _df, pf, *_r) in diffs:
            pairs.append({"leg": leg, "pf": pf,
                          "open": mk(sb, sp, psh, -1, "pair_open"),
                          "close": mk(bb, bp, psh, +1, "pair_close")})
            pf_sum += pf
        trades.append({
            "eb": int(eb), "xb": int(xb), "cap": cap,
            "pnl_pct": pnl_pct,
            "master_pnl_pct": pnl_pct - pf_sum / cap * 100,
            "entry": mk(eb, ep, sh, +1, "entry"),
            "exit": mk(xb, xp, sh, -1, "exit"),
            "pairs": pairs,
        })
    return trades


def day_marks(days):
    """逐 bar 日键 → (日终 bar 索引列表, 日键列表)。"""
    de_bars, keys = [], []
    for i in range(len(days) - 1):
        if days[i + 1] != days[i]:
            de_bars.append(i); keys.append(days[i])
    de_bars.append(len(days) - 1); keys.append(days[-1])
    return de_bars, keys


def trade_daily_gross(tr, closes, de_bars) -> dict[int, float]:
    """单笔交易的逐日 mark-to-market 毛 pnl（占该笔资本的比例）。
    与 pnl_pct 现金对账（±0.01pp fail-fast）。"""
    evs = sorted(
        [(tr["entry"]["bar"], +tr["entry"]["sh"], tr["entry"]["p"]),
         (tr["exit"]["bar"], -tr["exit"]["sh"], tr["exit"]["p"])]
        + [(p["open"]["bar"], -p["open"]["sh"], p["open"]["p"])
           for p in tr["pairs"]]
        + [(p["close"]["bar"], +p["close"]["sh"], p["close"]["p"])
           for p in tr["pairs"]])
    out: dict[int, float] = {}
    cash = pos = 0.0
    prev_val, ei = 0.0, 0
    di = bisect_left(de_bars, tr["eb"])
    while True:
        m = de_bars[di]
        while ei < len(evs) and evs[ei][0] <= m:
            _b, q, pr = evs[ei]
            cash -= q * pr; pos += q; ei += 1
        val = cash + pos * closes[m]
        if val != prev_val:
            out[di] = out.get(di, 0.0) + (val - prev_val) / tr["cap"]
        prev_val = val
        if ei == len(evs) and m >= tr["xb"]:
            break
        di += 1
    total = sum(out.values()) * 100
    if abs(total - tr["pnl_pct"]) > 0.01:
        raise SystemExit(f"日级对账失败：Σ日pnl {total:.4f} ≠ "
                         f"pnl_pct {tr['pnl_pct']:.4f}")
    return out


def build_stage1() -> dict:
    """信号层 + Rust 交易层 → 自含 fills/日级缓存（重网格无需再跑引擎）。"""
    if FILLS_JSON.exists():
        print("[stage1] 缓存命中，跳过引擎", flush=True)
        return json.loads(FILLS_JSON.read_text())
    import newchan_rust as nr
    from fugue_version_i import LADDER_SEG, PAIRING_VARIANTS, run_version_i
    from organic_fugue_rust_check import pack_tape
    from organic_signals import compute_organic_signals

    prev = json.loads(F_PREV.read_text())
    out = {}
    for tag, path in (("CL_1S", F_1S), ("CL_1M_WIN", F_1M_WIN)):
        print(f"[stage1] {tag} — {path.name}", flush=True)
        opens, highs, lows, closes, days = load_with_days(path)
        t0 = time.time()
        dir_flips: list = []
        tape = compute_organic_signals(opens, highs, lows, closes,
                                       dir_flips=dir_flips)
        fp = {"bsp": sum(len(s.bsp_events[l]) for s in tape if s.bsp_events
                         for l in range(11)),
              "div": sum(len(s.div_events[l]) for s in tape if s.div_events
                         for l in range(11)),
              "flips": len(dir_flips)}
        fp_expect = prev[tag]["tape_fp"]
        if fp != fp_expect:
            raise SystemExit(f"{tag} 磁带指纹不匹配：{fp} ≠ 在册 {fp_expect}"
                             "（信号层被改动或数据漂移——严格缠论约束违例）")
        print(f"  信号层 {time.time() - t0:.1f}s  指纹 PASS {fp}", flush=True)
        # P5 守卫（O0≡P5 在册链）
        run_version_i(tape, floor_ladder=LADDER_SEG,
                      pairing=PAIRING_VARIANTS["P5"])
        rtape = pack_tape(tape, dir_flips=dir_flips)
        res = nr.run_organic_rust(rtape, VARIANT, floor_ladder=LADDER_SEG,
                                  stop_mode="none", diag=True)
        trades = extract_trades(res["diag"], closes)
        # 零摩擦复利对账（在册 −3.84% / −15.86%）
        comp = 1.0
        for tr in trades:
            comp *= 1 + tr["pnl_pct"] / 100
        zero = (comp - 1) * 100
        reg = prev[tag]["variants"][VARIANT]["friction"]["0.0bps"][
            "total_compound"]
        if abs(zero - reg) > 0.05:
            raise SystemExit(f"{tag} 零摩擦复利 {zero:.2f} ≠ 在册 {reg}")
        de_bars, keys = day_marks(days)
        gross: dict[int, float] = {}
        for tr in trades:
            for di, frac in trade_daily_gross(tr, closes, de_bars).items():
                gross[di] = gross.get(di, 0.0) + frac
        # 年跨度（用于年化）
        if isinstance(days[0], int):
            years = (days[-1] - days[0] + 1) / 365.25
        else:  # 日期字符串 → 物理日跨度（CME 周日-周五 6天/周，不能用 252）
            from datetime import date
            d0 = date.fromisoformat(days[0][:10])
            d1 = date.fromisoformat(days[-1][:10])
            years = ((d1 - d0).days + 1) / 365.25
        out[tag] = {
            "tape_fp": fp, "zero_friction_compound": round(zero, 2),
            "bh": prev[tag]["bh"], "n_bars": len(closes),
            "n_days": len(keys), "years": round(years, 4),
            "mean_price": round(sum(closes) / len(closes), 3),
            "trades": trades,
            "gross_by_day": {str(k): v for k, v in gross.items()},
            "de_bars": de_bars,
        }
        print(f"  trades={len(trades)}  零摩擦={zero:+.2f}%（在册 PASS）"
              f"  天数={len(keys)}", flush=True)
    FILLS_JSON.write_text(json.dumps(out))
    print(f"[stage1] → {FILLS_JSON.name}", flush=True)
    return out


# ───────────────────────── stage 2：执行模型网格 ─────────────────────────

def fill_drag(f, f_fill: float, fr: float) -> float:
    """单 fill 期望摩擦（$，worst=未成交追价）。"""
    maker = f_fill * f["p"] * f["sh"]
    chase = (F_TAKER * f["pn"] + f["chase"]) * f["sh"]
    return fr * maker + (1 - fr) * chase


def cell_worst(trades, f_fill_bps: float, fr: float) -> dict:
    """worst case：全部信号执行，未 fill 侧追价。逐笔复利 + 分腿净现金。"""
    f_fill = f_fill_bps * 1e-4
    comp, wins = 1.0, 0
    leg_net: dict[str, float] = {}
    for tr in trades:
        drag = fill_drag(tr["entry"], f_fill, fr) \
            + fill_drag(tr["exit"], f_fill, fr)
        for p in tr["pairs"]:
            pd = fill_drag(p["open"], f_fill, fr) \
                + fill_drag(p["close"], f_fill, fr)
            drag += pd
            leg_net[p["leg"]] = leg_net.get(p["leg"], 0.0) + p["pf"] - pd
        adj = tr["pnl_pct"] - drag / tr["cap"] * 100
        comp *= 1 + adj / 100
        wins += adj > 0
    n = len(trades)
    return {"compound": round((comp - 1) * 100, 2),
            "win_rate": round(wins / n * 100, 1) if n else None,
            "leg_net_cash": {k: round(v, 1)
                             for k, v in sorted(leg_net.items())}}


def cell_best(trades, f_fill_bps: float, fr: float) -> dict:
    """best case：开仓侧未 fill 即撤单（单元以概率 fr 存在），平仓侧追价。
    期望值复利（近似声明见模块 docstring）。"""
    f_fill = f_fill_bps * 1e-4
    comp = 1.0
    for tr in trades:
        d_entry = f_fill * tr["entry"]["p"] * tr["entry"]["sh"]  # 成交才有笔
        d_exit = fill_drag(tr["exit"], f_fill, fr)
        pair_ev = 0.0
        for p in tr["pairs"]:
            d_open = f_fill * p["open"]["p"] * p["open"]["sh"]
            d_close = fill_drag(p["close"], f_fill, fr)
            pair_ev += fr * (p["pf"] - d_open - d_close)
        adj = fr * (tr["master_pnl_pct"]
                    + (pair_ev - d_entry - d_exit) / tr["cap"] * 100)
        comp *= 1 + adj / 100
    return {"compound": round((comp - 1) * 100, 2)}


def daily_returns(data, f_fill_bps: float, fr: float) -> list[float]:
    """worst 模型的逐日净收益序列（毛日收益 − 当日 fill 期望摩擦占比）。"""
    f_fill = f_fill_bps * 1e-4
    drag_by_day: dict[int, float] = {}
    de = data["_de_lookup"]
    for tr in data["trades"]:
        fills = [tr["entry"], tr["exit"]] + [
            x for p in tr["pairs"] for x in (p["open"], p["close"])]
        for f in fills:
            di = de(f["bar"])
            drag_by_day[di] = drag_by_day.get(di, 0.0) \
                + fill_drag(f, f_fill, fr) / tr["cap"]
    gross = {int(k): v for k, v in data["gross_by_day"].items()}
    n_days = data["n_days"]
    return [gross.get(d, 0.0) - drag_by_day.get(d, 0.0)
            for d in range(n_days)]


def curve_metrics(rets: list[float], lev: float, years: float) -> dict:
    """杠杆 L 下日级复利曲线指标。equity≤0 → RUIN。"""
    eq, peak, mdd = 1.0, 1.0, 0.0
    for r in rets:
        eq *= 1 + lev * r
        if eq <= 0:
            return {"ann_ret": -100.0, "sharpe": None, "max_dd": -100.0,
                    "ruin": True}
        peak = max(peak, eq)
        mdd = min(mdd, eq / peak - 1)
    n = len(rets)
    mu = sum(rets) / n
    var = sum((r - mu) ** 2 for r in rets) / n
    ann_days = n / years
    sharpe = (lev * mu / (lev * math.sqrt(var)) * math.sqrt(ann_days)
              if var > 0 else None)
    return {"ann_ret": round((eq ** (1 / years) - 1) * 100, 2),
            "sharpe": round(sharpe, 3) if sharpe is not None else None,
            "max_dd": round(mdd * 100, 2), "ruin": False}


def run_grid(data: dict) -> dict:
    out: dict = {"worst": {}, "best": {}}
    trades = data["trades"]
    for fb in F_FILL_BPS:
        for fr in FILL_RATES:
            key = f"{fb}|{fr}"
            out["worst"][key] = cell_worst(trades, fb, fr)
            out["best"][key] = cell_best(trades, fb, fr)
    return out


def run_leverage(data: dict) -> dict:
    """场景单元 × 杠杆档的日级指标 + 交易频率。"""
    years = data["years"]
    n_fills = sum(2 + 2 * len(t["pairs"]) for t in data["trades"])
    out = {"n_fills_per_day": round(n_fills / data["n_days"], 1),
           "round_turns_per_day": round(n_fills / 2 / data["n_days"], 1),
           "n_trades": len(data["trades"]),
           "max_margin_leverage": round(
               data["mean_price"] * 1000 / CL_MARGIN, 1),
           "scenarios": {}}
    for fb, fr, label in LEV_SCENARIOS:
        rets = daily_returns(data, fb, fr)
        cell = cell_worst(data["trades"], fb, fr)
        sc = {"f_fill_bps": fb, "fill_rate": fr,
              "compound_trade_level": cell["compound"], "levered": {}}
        for lv in LEVERAGES:
            sc["levered"][f"{lv}x"] = curve_metrics(rets, lv, years)
        out["scenarios"][label] = sc
    return out


def frontier(grid_worst: dict, bench: float) -> dict[str, float | None]:
    """每个 f_fill 行：复利 > bench 所需的最低 fill_rate（线性内插）。"""
    out = {}
    for fb in F_FILL_BPS:
        vals = [(fr, grid_worst[f"{fb}|{fr}"]["compound"])
                for fr in FILL_RATES]
        req = None
        for (fr0, c0), (fr1, c1) in zip(vals, vals[1:]):
            if c0 < bench <= c1:
                req = fr0 + (fr1 - fr0) * (bench - c0) / (c1 - c0)
                break
        if req is None and vals[0][1] >= bench:
            req = vals[0][0]
        out[str(fb)] = round(req, 3) if req is not None else None
    return out


def plot_heatmap(results: dict) -> None:
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    plt.rcParams["font.sans-serif"] = ["Hiragino Sans GB", "Arial Unicode MS",
                                       "DejaVu Sans"]
    plt.rcParams["axes.unicode_minus"] = False
    panels = [("CL_1S", "worst", "1s worst（未fill追价）"),
              ("CL_1S", "best", "1s best（开仓侧撤单）"),
              ("CL_1M_WIN", "worst", "1min worst（对照）")]
    fig, axes = plt.subplots(1, 3, figsize=(19, 6.5))
    for ax, (tag, model, title) in zip(axes, panels):
        grid = results[tag]["grid"][model]
        z = [[grid[f"{fb}|{fr}"]["compound"] for fr in FILL_RATES]
             for fb in F_FILL_BPS]
        vmax = max(40.0, max(abs(v) for row in z for v in row))
        im = ax.imshow(z, cmap="RdYlGn", aspect="auto",
                       vmin=-vmax, vmax=vmax)
        ax.set_xticks(range(len(FILL_RATES)),
                      [f"{fr:.0%}" for fr in FILL_RATES])
        ax.set_yticks(range(len(F_FILL_BPS)),
                      [f"{fb:+.2f}" for fb in F_FILL_BPS])
        ax.set_xlabel("fill rate")
        ax.set_ylabel("f_fill (bps/side, maker)")
        bh = results[tag]["bh"]
        ax.set_title(f"{title}\n复利% vs BH {bh:+.1f}%")
        for i, fb in enumerate(F_FILL_BPS):
            for j, fr in enumerate(FILL_RATES):
                v = z[i][j]
                ax.text(j, i, f"{v:+.0f}", ha="center", va="center",
                        fontsize=7,
                        color="black" if abs(v) < vmax * 0.6 else "white")
        fig.colorbar(im, ax=ax, shrink=0.8)
    fig.suptitle(f"CL {VARIANT} maker 执行模型：复利%（2024-06→2025-05，"
                 f"追价侧 taker={F_TAKER * 1e4:.2f}bps）")
    fig.tight_layout()
    fig.savefig(PNG, dpi=130)
    print(f"热力图 → {PNG.name}", flush=True)


def main() -> None:
    stage1 = build_stage1()
    results: dict = {}
    for tag in ("CL_1S", "CL_1M_WIN"):
        data = stage1[tag]
        # 日终 bar → 日索引查找（fills 的 bar 落在哪一天）
        # gross_by_day 的日索引即 de_bars 序号；fills 用同一 bisect。
        # de_bars 不在缓存里——由 trades/gross 的索引域重建不可行，
        # 故 stage1 缓存补充：用 bar→day 通过 de_bars 持久化。
        de_bars = data.get("de_bars")
        if de_bars is None:
            # 旧缓存升级路径：从原始数据重建日界
            _o, _h, _l, _c, days = load_with_days(
                F_1S if tag == "CL_1S" else F_1M_WIN)
            de_bars, _ = day_marks(days)
            data["de_bars"] = de_bars
            stage1[tag]["de_bars"] = de_bars
            FILLS_JSON.write_text(json.dumps(stage1))
        data["_de_lookup"] = lambda b, _de=de_bars: bisect_left(_de, b)

        print(f"\n[stage2] {tag}：{len(F_FILL_BPS)}×{len(FILL_RATES)} 网格",
              flush=True)
        grid = run_grid(data)
        lev = run_leverage(data)
        results[tag] = {
            "bh": data["bh"], "zero": data["zero_friction_compound"],
            "n_days": data["n_days"], "years": data["years"],
            "grid": grid, "leverage": lev,
            "frontier_vs_zero": frontier(grid["worst"], 0.0),
            "frontier_vs_bh": frontier(grid["worst"], data["bh"]),
        }
        for fb in (0.0, 0.04, -0.3):
            key_fb = fb if fb in F_FILL_BPS else None
            if key_fb is None:
                continue
            row = "  ".join(
                f"{fr:.0%}→{grid['worst'][f'{key_fb}|{fr}']['compound']:+.1f}"
                for fr in FILL_RATES)
            print(f"  f_fill={fb:+.2f}bps worst: {row}", flush=True)
    # 1s vs 1min 同 cell 优势前沿
    adv = {}
    for fb in F_FILL_BPS:
        for fr in FILL_RATES:
            k = f"{fb}|{fr}"
            adv[k] = round(
                results["CL_1S"]["grid"]["worst"][k]["compound"]
                - results["CL_1M_WIN"]["grid"]["worst"][k]["compound"], 2)
    results["adv_1s_vs_1m_worst"] = adv
    OUT_JSON.write_text(json.dumps(results, indent=1, ensure_ascii=False))
    print(f"\n结果 → {OUT_JSON.name}", flush=True)
    plot_heatmap(results)


if __name__ == "__main__":
    main()
