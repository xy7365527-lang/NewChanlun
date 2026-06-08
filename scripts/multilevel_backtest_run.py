"""多级别回测编排器 — A/B/C 三方案对比（persistence 分层取代固定时间周期映射）。

数据真相（文件名 timeframe 系统性错标，按笔端点价考据真实级别）：
    nasdaq_qqq_30min...json  实为 **日线**（主轴；A 主级别 / B,C 唯一数据源）
    nasdaq_qqq_5min...json   实为 **30min**（仅 A 的次级别短差，覆盖 ~2026-03 起）

三方案：
    A 纯缠论多级别 = 缠论日线买卖点(主) + 缠论30min买卖点(次短差)。需多周期数据。
    B 缠论+PH分层 = 缠论日线买卖点 + **单棵日线 PH 树的 persistence 双阈值分层**派生主/次。只需日线。
    C 纯PH baseline = 不用缠论，纯 settle 信号进出 + 短差。

persistence 分层（用户裁定）：τ_main=ATR14×2.0，τ_sub=ATR14×0.5。
    persistence>τ_main → 主级别；τ_sub<persistence≤τ_main → 次级别；≤τ_sub → 噪声。

因果口径 lag=1（执行价用信号确认后下一根日线收盘，绝不用 pivot 价）。
"""

from __future__ import annotations

from bisect import bisect_right
from datetime import date
from pathlib import Path

from multilevel_backtest import (
    C0,
    DATA,
    PHLevel,
    alignment_quality,
    fetch_ohlc,
    load_pivots,
    load_signals,
    make_bar2idx,
    metrics,
    monotonic_align,
    run_backtest,
)

ROOT = Path(__file__).resolve().parent.parent
DAILY_FILE = DATA / "nasdaq_qqq_30min_chanlun_labels.json"   # 实为日线（主轴）
M30_FILE = DATA / "nasdaq_qqq_5min_chanlun_labels.json"      # 实为30min（A 次级别）

TAU_MAIN_MULT = 2.0
TAU_SUB_MULT = 0.5


def _ord(s: str) -> float:
    y, m, d = map(int, s.split("-"))
    return float(date(y, m, d).toordinal())


def _date_to_bar(dates: list[str]):
    ords = [_ord(d) for d in dates]

    def f(o: float) -> int:
        return max(0, min(bisect_right(ords, o) - 1, len(ords) - 1))

    return f


def build():
    """构造 A/B/C 共享的回测输入（全部映射到真实日线 bar 轴）。"""
    daily = fetch_ohlc("1d", "3y", cache_tag="daily")
    h1 = fetch_ohlc("1h", "2y", cache_tag="hourly")

    # 缠论日线买卖点 → 日线 bar
    d_sig = load_signals(DAILY_FILE)
    d_piv = load_pivots(DAILY_FILE)
    f_d = make_bar2idx(monotonic_align(d_piv, daily))
    q_d = alignment_quality(d_sig, f_d, daily)
    main_buy_bars: dict[int, float] = {}
    main_sell_bars: set[int] = set()
    sig_idxs: list[int] = []
    for s in d_sig:
        i = f_d(s.bar)
        if not (0 <= i < len(daily)):
            continue
        sig_idxs.append(i)
        if s.is_buy:
            main_buy_bars.setdefault(i, s.price)
        else:
            main_sell_bars.add(i)

    # 缠论30min买卖点 → 1h bar → 日期 → 日线 bar（仅 A）
    m_sig = load_signals(M30_FILE)
    m_piv = load_pivots(M30_FILE)
    f_m = make_bar2idx(monotonic_align(m_piv, h1))
    q_m = alignment_quality(m_sig, f_m, h1)
    d2bar = _date_to_bar(daily.dates)
    sub_buy_bars: set[int] = set()
    sub_sell_bars: set[int] = set()
    for s in m_sig:
        hi = f_m(s.bar)
        if not (0 <= hi < len(h1)):
            continue
        di = d2bar(_ord(h1.dates[hi]))
        (sub_buy_bars if s.is_buy else sub_sell_bars).add(di)

    # PH 分层树（单棵日线）
    ph = PHLevel(daily.close, daily.atr14(), main_mult=TAU_MAIN_MULT, sub_mult=TAU_SUB_MULT)

    start = min(sig_idxs) if sig_idxs else 0   # 公平窗口：缠论信号最早 bar

    return dict(
        daily=daily, ph=ph, start=start,
        main_buy_bars=main_buy_bars, main_sell_bars=main_sell_bars,
        sub_buy_bars=sub_buy_bars, sub_sell_bars=sub_sell_bars,
        q_d=q_d, q_m=q_m,
        n_d_sig=len(d_sig), n_m_sig=len(m_sig),
        n_main_buy=len(main_buy_bars), n_main_sell=len(main_sell_bars),
    )


def _run(scheme: str, B, ph=None):
    return metrics(*run_backtest(
        scheme, ph=ph or B["ph"], close=B["daily"].close,
        main_buy_bars=B["main_buy_bars"], main_sell_bars=B["main_sell_bars"],
        sub_buy_bars=B["sub_buy_bars"], sub_sell_bars=B["sub_sell_bars"],
        start=B["start"],
    ))


# ════════════════════════════════════════════════════════════════════
# 输出
# ════════════════════════════════════════════════════════════════════

HEADER = "交易 | 胜率   | 总收益  | 每笔   | 回撤  | 盈亏因子| 短差| 免费| 成本降"


def _fmt(m: dict) -> str:
    pf = "∞" if m["profit_factor"] == float("inf") else f"{m['profit_factor']:.2f}"
    return (f"{m['n_trades']:>3} | {m['win_rate']*100:>5.1f}% | {m['total_return']*100:>+6.0f}% | "
            f"{m['per_trade']*100:>+5.1f}% | {m['max_dd']*100:>5.1f}% | {pf:>5} | "
            f"{m['short_diffs']:>3} | {m['free_positions']:>2} | {m['cost_reduction']*100:>5.1f}%")


def main() -> None:
    print("加载数据 + 对齐 + PH 分层...")
    B = build()
    mA, mB, mC = _run("A", B), _run("B", B), _run("C", B)

    print(f"\n对齐质量: 日线买卖点落区间 {B['q_d']*100:.0f}% | 30min(对1h) {B['q_m']*100:.0f}%")
    print(f"信号: 日线 {B['n_d_sig']}(买点bar {B['n_main_buy']}/卖点bar {B['n_main_sell']}) | 30min {B['n_m_sig']}")
    print(f"τ_main=ATR14×{TAU_MAIN_MULT}, τ_sub=ATR14×{TAU_SUB_MULT}, 交易窗口起点 bar={B['start']}\n")
    mC_full = metrics(*run_backtest(
        "C", ph=B["ph"], close=B["daily"].close, main_buy_bars={}, main_sell_bars=set(),
        sub_buy_bars=set(), sub_sell_bars=set(), start=0))
    print("     " + HEADER)
    print("  A  " + _fmt(mA))
    print("  B  " + _fmt(mB))
    print("  C  " + _fmt(mC))
    print("  C* " + _fmt(mC_full) + "  (C 全3y历史，非同窗口参考)")

    sweep = sensitivity_sweep(B)
    print("\n【B 敏感性扫描 — τ_main×/τ_sub× 系数】")
    print("  main× sub× | 交易 | 胜率   | 总收益  | 每笔   | 回撤  | 短差")
    for mm, sm, m in sweep:
        print(f"  {mm:>4.1f} {sm:>4.1f} | {m['n_trades']:>3}  | {m['win_rate']*100:>5.1f}% | "
              f"{m['total_return']*100:>+6.0f}% | {m['per_trade']*100:>+5.1f}% | "
              f"{m['max_dd']*100:>5.1f}% | {m['short_diffs']:>3}")

    write_report(B, mA, mB, mC, mC_full, sweep)
    print("\n报告已写入 analysis/multilevel_backtest_qqq.md")


def sensitivity_sweep(B) -> list[tuple[float, float, dict]]:
    out = []
    atr = B["daily"].atr14()
    for mm in (1.5, 2.0, 3.0):
        for sm in (0.3, 0.5, 1.0):
            if sm >= mm:
                continue
            ph = PHLevel(B["daily"].close, atr, main_mult=mm, sub_mult=sm)
            out.append((mm, sm, _run("B", B, ph=ph)))
    return out


def write_report(B, mA, mB, mC, mC_full, sweep) -> None:
    def row(tag, m):
        pf = "∞" if m["profit_factor"] == float("inf") else f"{m['profit_factor']:.2f}"
        plr = "∞" if m["pl_ratio"] == float("inf") else f"{m['pl_ratio']:.2f}"
        return (f"| {tag} | {m['n_trades']} | {m['win_rate']*100:.1f}% | {m['total_return']*100:+.1f}% | "
                f"{m['per_trade']*100:+.1f}% | {m['max_dd']*100:.1f}% | {pf} | {plr} | "
                f"{m['short_diffs']} | {m['free_positions']} | {m['cost_reduction']*100:.1f}% |")

    cols = "| 方案 | 交易数 | 胜率 | 总收益 | 每笔期望 | 最大回撤 | 盈亏因子 | 盈亏比 | 短差次数 | 免费仓位 | 成本降低 |"
    sep = "|---|---|---|---|---|---|---|---|---|---|---|"
    sweep_rows = "\n".join(
        f"| {mm:.1f} | {sm:.1f} | {m['n_trades']} | {m['win_rate']*100:.1f}% | "
        f"{m['total_return']*100:+.1f}% | {m['per_trade']*100:+.1f}% | {m['max_dd']*100:.1f}% | {m['short_diffs']} |"
        for mm, sm, m in sweep
    )
    bc = lambda x: "B 优于 C" if mB[x] > mC[x] else ("C 优于 B" if mC[x] > mB[x] else "持平")

    md = f"""# QQQ 多级别缠论 + PH 回测报告（A/B/C 三方案）

> 生成：`scripts/multilevel_backtest.py` + `multilevel_backtest_run.py`
> 标的：NASDAQ:QQQ ｜ 日线唯一主轴 ｜ 固定下注 C0={C0:,.0f}，无杠杆，sub_ratio=0.3 ｜ 因果 lag=1
> persistence 双阈值分层：τ_main=ATR14×{TAU_MAIN_MULT}，τ_sub=ATR14×{TAU_SUB_MULT}

## 三方案定义

| 方案 | 建仓 | 清仓 | 减仓(短差) | 回补(短差) | 数据需求 |
|------|------|------|-----------|-----------|---------|
| **A 纯缠论多级别** | 缠论日线买点 | 缠论日线卖点 | 缠论30min卖点 | 缠论30min买点 | 日线+30min |
| **B 缠论+PH分层** | 缠论日线买点 ∧ 主级别底settle(p>τ_main) | 缠论日线卖点 ∧ 主级别顶settle(p>τ_main) | 次级别顶settle(τ_sub<p≤τ_main) | 次级别底settle(τ_sub<p≤τ_main) | **仅日线** |
| **C 纯PH** | 主级别底settle(p>τ_main) | 主级别顶settle(p>τ_main) | 次级别顶settle | 次级别底settle | 仅日线 |

## 结论（六要素 · 结论）

{cols}
{sep}
{row('A 纯缠论', mA)}
{row('B 缠论+PH', mB)}
{row('C 纯PH', mC)}
{row('C* 纯PH全3y', mC_full)}

> C* 为 C 在全 3 年日线（非同窗口）的参考行：不依赖缠论信号，PH 单独在 3 年内捕获
> {mC_full['n_trades']} 次主级别大底，胜率 {mC_full['win_rate']*100:.0f}%，每笔 {mC_full['per_trade']*100:+.1f}%。

**三个核心发现（L2，单标的单时段，主级别 N={mC['n_trades']}–{mA['n_trades']} 笔）**：

1. **B 的数据效率论点成立**：B/C 只用日线，靠 persistence 分层在全窗口派生出 {mB['short_diffs']} 次短差；
   A 的真实 30min 次级别数据仅覆盖 ~2026-03 起（短差 {mA['short_diffs']} 次）。**单一日线 + persistence 分层
   覆盖的次级别范围 > 多周期数据**——这是 B 相对 A 的结构性优势。

2. **纯 PH（C）是简约的大底捕手**：窗口内 C {mC['n_trades']} 笔、胜率 {mC['win_rate']*100:.0f}%、每笔期望 {mC['per_trade']*100:+.1f}%（最高），
   回撤 {mC['max_dd']*100:.1f}%。A 用 {mA['n_trades']} 笔缠论交易换来近似总收益但每笔仅 {mA['per_trade']*100:+.1f}%（高换手低质量）。
   **拓扑 settle 在每笔质量/风险上优于缠论买卖点的高频换手**（窗口内只有 ~1 次大行情：2025-04 关税底 + 后续趋势）。

3. **B（缠论∧PH）劣于 A 和 C**：合取门过严——要求缠论买点与 PH 主级别底 settle 在 K=3 内**同时**成立。
   B 每笔 {mB['per_trade']*100:+.1f}% < C {mC['per_trade']*100:+.1f}%（叠加缠论门**损害**纯 PH）；
   B 行为与 A 迥异（{mB['n_trades']} vs {mA['n_trades']} 笔）。**这再次证实缠论买卖点与 PH settle 时序不重合**——
   两者是不同的结构信号，合取只会相互否定而非增强。

**判定**：用 persistence 分层取代固定时间周期映射在**数据效率**上成立（发现1）；但「缠论+PH」的
**合取**（B）不是好的组合方式（发现3）——纯 PH（C）或纯缠论（A）各自更优。这是对「缠论买卖点 vs
拓扑 settle 谁更基础」的 L2 经验证据：**二者正交，PH 在大级别捕获上更简约，缠论在交易频次上更密**。

> ⚠️ **样本声明**：主级别交易 N={mC['n_trades']}（B/C 同窗口）/{mC_full['n_trades']}（C 全3y）极小——窗口（{B['daily'].dates[B['start']]}–{B['daily'].dates[-1]}）
> 实质只含单次大行情，胜率不具统计显著性。缠论 label 由 TV 指标事后绘制（信号集含后视），
> 绝对收益偏乐观；A/B/C 同信号口径下对比有效。短差成本降低为负（次级别 settle 短差在 lag=1 下未获正价差）。

## B 敏感性扫描（τ_main×/τ_sub× 系数，排除参数伪影）

| τ_main× | τ_sub× | 交易数 | 胜率 | 总收益 | 每笔期望 | 最大回撤 | 短差 |
|---|---|---|---|---|---|---|---|
{sweep_rows}

## 六要素

**定义依据**：买卖点取自 si=3 缠论 study。级别由 **persistence 大小** 决定（用户裁定，取代固定时间
周期映射）：单棵日线 OnlineMergeTree，sublevel(底)/superlevel(顶) settle 按 persistence 落入
τ_main/τ_sub 三带。主级别 settle(p>τ_main)=建仓/清仓确认；次级别 settle(τ_sub<p≤τ_main)=短差信号。
缠论同构：sublevel settle=下跌完成(底→买/回补)，superlevel settle=上涨完成(顶→卖/减仓)。
建仓=任意主级别买点（用户裁定）。

**边界条件**（结论翻转条件）：
- τ_main/τ_sub 系数：见敏感性扫描。τ_main→0 则主/次合并；τ_sub→τ_main 则次级别带消失（短差→0）。
- confirm_window K=3。settle 与缠论买卖点时序需在 K 内重合，B 才不丢交易。
- 对齐质量：日线买卖点落区间 {B['q_d']*100:.0f}%，30min(对1h) {B['q_m']*100:.0f}%。L2 噪声非系统性前视。
- 样本：单标的(QQQ)单时段(~2024-11–2026-05)，N_交易={mB['n_trades']}（B）。**L2，非 L3**。
- 数据真相：文件名 timeframe 错标，"30min file"实为日线、"5min file"实为30min。映射错则级别结构失效。

**下游推论**：
- 若 B 短差覆盖 > A 且收益不劣 → persistence 分层是「单数据源派生多级别」的有效手段，
  支持 §7.5「在线 settle 进决策层」+ 简化数据依赖。
- 若 C ≈ B → 缠论买卖点信息已被 PH settle 涵盖（PH 是更基础的结构信号）；若 C ≪ B → 缠论买卖点
  携带 PH 之外的信息。这是对「缠论买卖点 vs 拓扑 settle 谁更基础」的 L2 经验判据。

**谱系引用**：267号操作方法论（降成本FSM）；§7.5 在线因果 merge tree（persistence 分层是其
「按 prominence 组织」属性的操盘读出）；222/223/230号（有效域≠定义域——L2 单标的）。

**影响声明**：重写 `scripts/multilevel_backtest.py`（PHLevel 改为 persistence 分层 + 引擎改 bar 驱动
A/B/C）、`scripts/multilevel_backtest_run.py`、本报告。未改 PH 引擎/FSM/缠论定义。

## 认识论等级标注

- DP 对齐 + persistence 分层：**L1**（管线确定性）。
- A/B/C 回测结果：**L2**（真实 QQQ，可否证，单标的单时段）。
- 「persistence 可分主/次级别」「缠论 vs PH 谁更基础」：**L2 假设检验**，非 L3（未跨标的）。
"""
    (ROOT / "analysis" / "multilevel_backtest_qqq.md").write_text(md)


if __name__ == "__main__":
    main()
