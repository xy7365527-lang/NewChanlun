"""P3 判决实验 — 随机门控对照（判决整条门控 alpha 链）。

## 命题 P3

门控（MACD 背驰进出场）产生的 alpha 是否携带择时信息？还是仅仅是"在场暴露"
（exposure）这一守恒量的体现？

## 三组对照（OKLO 447K bars，含盘前盘后，无交易成本）

  (a) 真实门控 = E 版本：MACD 底背驰进场 / L2 顶背驰出场
      （`compute_e_signals_rust` + `run_swing_trading(MODE_NONE)`）。
  (b) 随机门控：与 (a) 相同的交易笔数 N，但 entry 时点从全时间窗口均匀随机采样，
      每笔持有时长 = E 版本平均持有时长。bootstrap 10 次取均值±std。
  (c) Buy-hold：全程持有。

三组同数据、同（零）成本，唯一差异是 entry 时点的来源（背驰定位 vs 均匀随机）。

## 判决标准

  - (a) ≈ (b)：门控无信息增量，alpha = 暴露守恒          → P3 成立
  - (a) > (b)：门控有真实择时 alpha                      → P3 否证
  - (a) < (b) < (c)：门控负 alpha                        → P3 强化

## 认识论等级

L2（真实数据，单标的单时段假设检验，可产生否定性结果）。随机门控对照是
`project_backtest_benchmark_falsifiability` 记忆指出的"可证伪性补强"——它给出了
"相同暴露下的随机基线"，使"E 版本是否真有择时 alpha"成为可证伪命题。
"""

from __future__ import annotations

import json
import random
import sys
import time
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

import fugue_alpha_diagnosis as F  # noqa: E402
import m1_e_rust_engine as RE  # noqa: E402

DATA_FILE = ROOT / "analysis" / "data_cache" / "oklo_1m_databento.json"
OUT_FILE = ROOT / "analysis" / "p3_random_gate_control.md"

N_BOOTSTRAP = 10        # 用户指定的明细展示轮数（mean±std 表）
N_BOOTSTRAP_LARGE = 5000  # 百分位/p 值的稳定估计轮数（随机门控分布重尾，10 次不足以刻画）
BASE_SEED = 20260609  # 固定基种子 → 实验可复现（每次 bootstrap 用 BASE_SEED + run_idx）


def load_ohlc(
    path: Path,
) -> tuple[list[float], list[float], list[float], list[float]]:
    """加载 databento bars 列表格式（含 ts/open/high/low/close/volume）。

    与 m1_e_rust_backtest.load_ohlc 同款 nan 清洗（删除任一 OHLC 为 nan 的整根 bar）——
    nan 污染 MACD/PH/背驰下游。本文件无 nan（实测），清洗为防御性。
    """
    raw = json.loads(path.read_text())
    bars = raw["bars"]
    opens: list[float] = []
    highs: list[float] = []
    lows: list[float] = []
    closes: list[float] = []
    for b in bars:
        o, h, l, c = float(b["open"]), float(b["high"]), float(b["low"]), float(b["close"])
        if o != o or h != h or l != l or c != c:  # nan != nan
            continue
        opens.append(o)
        highs.append(h)
        lows.append(l)
        closes.append(c)
    return opens, highs, lows, closes


def compound_and_maxdd(returns_chrono: list[float]) -> tuple[float, float]:
    """给定按时间顺序排列的逐笔收益率（小数，如 0.05=+5%），返回 (复利%, 最大回撤%)。

    复利对顺序不敏感（乘法可交换），但最大回撤强依赖顺序——故输入**必须**已按
    entry_bar 时间排序。逐字复用 `compute_metrics` 的权益曲线/回撤算法以保证可比。
    """
    eq = 1.0
    peak = 1.0
    max_dd = 0.0
    for r in returns_chrono:
        eq *= 1 + r
        peak = max(peak, eq)
        max_dd = min(max_dd, (eq - peak) / peak)
    return (eq - 1) * 100, max_dd * 100


@dataclass(frozen=True)
class GateResult:
    n_trades: int
    compound: float   # %
    max_dd: float     # %
    excess_vs_bh: float  # %


def run_real_gate(
    opens: list[float],
    highs: list[float],
    lows: list[float],
    closes: list[float],
) -> tuple[GateResult, int, float]:
    """(a) 真实门控 = E 版本。返回 (结果, 交易笔数 N, 平均持有时长 H)。

    复用 m1_e_rust_engine.compute_e_signals_rust + run_swing_trading(MODE_NONE)，
    与 m1_e_rust_backtest 逐字一致——本实验不修改任何信号/交易逻辑。
    """
    bh = (closes[-1] - closes[0]) / closes[0] * 100
    signals = RE.compute_e_signals_rust(opens, highs, lows, closes)
    trades, _ = F.run_swing_trading(signals, F.MODE_NONE)
    m = F.compute_metrics(trades)
    n = len(trades)
    avg_hold = (
        sum(t.exit_bar - t.entry_bar for t in trades) / n if n else 0.0
    )
    res = GateResult(
        n_trades=n,
        compound=m["total_compound"],
        max_dd=m["max_dd"],
        excess_vs_bh=m["total_compound"] - bh,
    )
    return res, n, avg_hold


def simulate_one_random_gate(
    closes: list[float],
    n_trades: int,
    hold_bars: int,
    rng: random.Random,
) -> tuple[float, float]:
    """(b) 随机门控单次模拟 —— P3 判决的心脏。

    [TODO — 由用户实现]

    目标：在**相同暴露**（n_trades 笔，每笔持有 hold_bars 根 bar）但**随机择时**
    （entry 时点均匀随机）下，模拟出一条交易序列并返回 (复利%, 最大回撤%)。

    返回值必须可与真实门控的 (compound, max_dd) 直接比较 → 故须复用
    `compound_and_maxdd(returns_chrono)`，且传入的逐笔收益率**已按 entry_bar 排序**。

    需要你做的价值判断（这些选择决定实验有效性，不是 boilerplate）：

      1. entry 采样范围：从哪里采到哪里？每笔须有完整的 hold_bars 才能算出
         close[entry + hold_bars]。合法范围 = [0, len(closes) - hold_bars - 1]。
         越界的 entry 会让某些交易没有完整持有期 → 暴露不守恒 → 判决失真。

      2. 单笔收益率：r_i = close[entry_i + hold_bars] / close[entry_i] - 1
         （与 E 版本 pnl = exit/entry - 1 同构，零成本）。

      3. 顺序：复利对顺序不敏感，但 max_dd 依赖顺序 → 必须按 entry_bar 升序排列
         收益率序列再喂给 compound_and_maxdd（否则随机门控的回撤数字无意义）。

      4. 重叠：本设计允许时间重叠（纯均匀采样 N 个 entry，不做 non-overlap 拒绝）——
         因为 P3 检验的是"相同总暴露下随机择时的期望表现"，重叠不破坏暴露守恒
         （∑ hold_bars 不变）。如果你认为应改为 non-overlap，这是一个会改变
         暴露语义的决策，需在 .md 中显式声明并论证。

    用 `rng`（已按 BASE_SEED + run_idx 播种）做所有随机采样——保证可复现，
    且符合 llm-role-boundary 规则（random 用于性能/统计采样，非概率范式命名）。

    采用推荐实现：有放回 + 允许重叠 + 统一持有时长——对"暴露守恒"最干净的检验
    （∑hold_bars 严格不变，无拒绝采样引入的边界偏差）。
    """
    # 合法 entry 范围：每笔须有完整 hold_bars 才能取到 close[entry + hold_bars]。
    max_entry = len(closes) - hold_bars - 1
    if max_entry < 0 or n_trades <= 0:
        return 0.0, 0.0
    # 均匀随机采样 n_trades 个 entry（有放回），用 rng 保证可复现。
    entries = sorted(rng.randint(0, max_entry) for _ in range(n_trades))
    # 逐笔收益 r_i = close[e + hold] / close[e] - 1（与 E 版本 exit/entry-1 同构，零成本）；
    # entries 已按 entry_bar 升序 → max_dd 时间顺序正确。
    returns = [closes[e + hold_bars] / closes[e] - 1 for e in entries]
    return compound_and_maxdd(returns)


def run_random_gate(
    closes: list[float],
    n_trades: int,
    hold_bars: int,
    bh: float,
) -> tuple[GateResult, list[float], list[float]]:
    """(b) 随机门控 bootstrap：跑 N_BOOTSTRAP 次取均值。

    返回 (均值结果, 每次复利列表, 每次回撤列表)——后两者用于报告 std。
    """
    compounds: list[float] = []
    maxdds: list[float] = []
    for run_idx in range(N_BOOTSTRAP):
        rng = random.Random(BASE_SEED + run_idx)
        comp, dd = simulate_one_random_gate(closes, n_trades, hold_bars, rng)
        compounds.append(comp)
        maxdds.append(dd)
    mean_comp = sum(compounds) / len(compounds)
    mean_dd = sum(maxdds) / len(maxdds)
    res = GateResult(
        n_trades=n_trades,
        compound=mean_comp,
        max_dd=mean_dd,
        excess_vs_bh=mean_comp - bh,
    )
    return res, compounds, maxdds


def _std(xs: list[float]) -> float:
    if len(xs) < 2:
        return 0.0
    m = sum(xs) / len(xs)
    return (sum((x - m) ** 2 for x in xs) / (len(xs) - 1)) ** 0.5


def _median(xs: list[float]) -> float:
    s = sorted(xs)
    n = len(s)
    if n == 0:
        return 0.0
    if n % 2 == 1:
        return s[n // 2]
    return (s[n // 2 - 1] + s[n // 2]) / 2


@dataclass(frozen=True)
class Percentile:
    """E 在随机门控大样本分布中的秩统计（对重尾免疫，替代失效的 ±1σ 带）。"""
    n_runs: int
    p_random_ge_e: float  # P(随机 ≥ 真实)：≈0.5 无择时；≈0 门控胜；≈1 门控负
    e_percentile: float   # E 在随机分布中的百分位（= 1 - p_random_ge_e）
    rnd_median: float     # 随机门控复利中位数（稳健中心）
    rnd_mean: float
    rnd_std: float


def large_bootstrap_percentile(
    closes: list[float], n_trades: int, hold_bars: int, e_compound: float,
) -> Percentile:
    """大样本 bootstrap → E 在随机门控分布中的秩。

    重尾分布（std > mean）下 mean±1σ 检验功效≈0；秩统计（多少比例随机跑赢 E）
    是稳健替代。N_BOOTSTRAP_LARGE 次独立采样（与明细表共享 BASE_SEED 偏移空间）。
    """
    comps: list[float] = []
    n_ge = 0
    for k in range(N_BOOTSTRAP_LARGE):
        rng = random.Random(BASE_SEED + 1_000 + k)
        comp, _ = simulate_one_random_gate(closes, n_trades, hold_bars, rng)
        comps.append(comp)
        if comp >= e_compound:
            n_ge += 1
    p = n_ge / len(comps)
    return Percentile(
        n_runs=len(comps),
        p_random_ge_e=p,
        e_percentile=(1 - p) * 100,
        rnd_median=_median(comps),
        rnd_mean=sum(comps) / len(comps),
        rnd_std=_std(comps),
    )


def verdict(real: GateResult, pct: Percentile, bh: float, n_trades: int) -> str:
    """三组对照 → P3 判决（秩统计主判 + N 功效警告）。

    ±1σ 带在重尾分布上失效（有效域膨胀，formalization-validity-domain）——改用
    P(随机 ≥ 真实) 的秩检验。p≈0.5 ⟹ E 与随机不可区分；p 小 ⟹ E 真有择时 alpha。
    """
    p = pct.p_random_ge_e
    # 主判决：秩
    if 0.33 <= p <= 0.67:
        core = (
            f"**P3 成立（弱）**：随机门控有 **{p:.0%}** 的概率跑赢真实门控"
            f"（E 处于随机分布第 {pct.e_percentile:.0f} 百分位），E 与相同暴露的随机择时"
            f"**统计不可区分**——门控无可检出的择时信息增量，alpha 主要来自在场暴露。"
        )
    elif p < 0.33:
        core = (
            f"**P3 否证**：仅 **{p:.0%}** 的随机门控跑赢真实门控"
            f"（E 处于第 {pct.e_percentile:.0f} 百分位），门控显著优于相同暴露的随机择时"
            f"——携带真实择时 alpha。"
        )
    else:  # p > 0.67
        tail = "且随机中位数高于 buy-hold" if pct.rnd_median > bh else ""
        core = (
            f"**P3 强化**：**{p:.0%}** 的随机门控跑赢真实门控"
            f"（E 仅处于第 {pct.e_percentile:.0f} 百分位），门控为负择时 alpha"
            f"——劣于相同暴露的随机择时{('，' + tail) if tail else ''}。"
        )
    return core


def main() -> None:
    print("P3 随机门控对照实验 — OKLO 447K bars")
    t0 = time.time()
    opens, highs, lows, closes = load_ohlc(DATA_FILE)
    n_bars = len(closes)
    bh = (closes[-1] - closes[0]) / closes[0] * 100
    print(f"  数据：{n_bars:,} bars | Buy-hold = {bh:+.2f}%")

    # (a) 真实门控
    print("  (a) 真实门控（E 版本，Rust 引擎驱动）...")
    real, n_trades, avg_hold = run_real_gate(opens, highs, lows, closes)
    hold_bars = int(round(avg_hold))
    print(f"      N={n_trades} 笔 | 平均持有={avg_hold:.1f} bars (≈{hold_bars}) "
          f"| 复利={real.compound:+.2f}% | MDD={real.max_dd:+.2f}%")

    # (b) 随机门控 bootstrap
    print(f"  (b) 随机门控 bootstrap ×{N_BOOTSTRAP}...")
    rnd, comps, dds = run_random_gate(closes, n_trades, hold_bars, bh)
    comp_std = _std(comps)
    dd_std = _std(dds)
    print(f"      复利={rnd.compound:+.2f}% ± {comp_std:.2f}% "
          f"| MDD={rnd.max_dd:+.2f}% ± {dd_std:.2f}%")

    # (b') 大样本秩统计（重尾分布下 ±1σ 失效，改用百分位/p 值）
    print(f"  (b') 大样本 bootstrap ×{N_BOOTSTRAP_LARGE} 求秩...")
    pct = large_bootstrap_percentile(closes, n_trades, hold_bars, real.compound)
    print(f"      P(随机≥真实)={pct.p_random_ge_e:.1%} | E 百分位={pct.e_percentile:.0f} "
          f"| 随机中位数={pct.rnd_median:+.2f}% | 随机均值={pct.rnd_mean:+.2f}%")

    # (c) buy-hold 已算
    v = verdict(real, pct, bh, n_trades)
    elapsed = time.time() - t0
    print(f"  判决：{v}")
    print(f"  总耗时 {elapsed:.1f}s")

    _write_report(
        n_bars, bh, real, n_trades, avg_hold, hold_bars,
        rnd, comps, dds, comp_std, dd_std, pct, v, elapsed,
    )
    print(f"  报告已写入：{OUT_FILE}")


def _write_report(
    n_bars: int, bh: float, real: GateResult, n_trades: int,
    avg_hold: float, hold_bars: int, rnd: GateResult,
    comps: list[float], dds: list[float], comp_std: float, dd_std: float,
    pct: Percentile, v: str, elapsed: float,
) -> None:
    exposure_frac = (n_trades * hold_bars) / n_bars * 100
    single_frac = hold_bars / n_bars * 100
    lines: list[str] = []
    lines.append("# P3 判决实验：随机门控对照\n")
    lines.append(f"> 标的 OKLO | {n_bars:,} bars（含盘前盘后，2024-05-10 ~ 2026-06-02）"
                 f" | 零交易成本 | 认识论等级 **L2**\n")

    # ── 统计功效警告（置顶，因为它限定了整个判决的可信度）──
    if n_trades < 30:
        lines.append("> ⚠️ **统计功效警告（先读这条）**：真实门控在本数据上仅产生 "
                     f"**{n_trades} 笔**交易，平均单笔持有 {avg_hold:.0f} bars "
                     f"= 序列长度的 **{single_frac:.0f}%**，名义总暴露 "
                     f"{exposure_frac:.0f}%（含重叠）。N={n_trades} 太小 + 单笔横跨"
                     "巨幅行情 ⟹ 随机门控分布**极重尾**（见下表 std > mean）。"
                     "因此用户原定的 **mean±1σ 带判决在此数据上功效≈0**（带宽 "
                     f"[{rnd.compound - comp_std:+.0f}%, {rnd.compound + comp_std:+.0f}%] "
                     "几乎吞掉一切，落入即「成立」是假确认，违反 formalization-validity-"
                     "domain）。**主判决改用大样本秩统计（百分位/p 值），对重尾免疫。**\n")

    lines.append("## 命题\n")
    lines.append("门控（MACD 背驰进出场）的 alpha 是否携带择时信息，还是仅是"
                 "「在场暴露」这一守恒量的体现？随机门控保持**相同交易笔数 N "
                 "与相同平均持有时长**（即相同总暴露），仅把 entry 时点替换为"
                 "均匀随机——若两者收益相等，则择时无信息增量。\n")

    lines.append("## 三组对照结果\n")
    lines.append("| 组 | 说明 | 交易笔数 | 平均持有 | 复利收益 | 最大回撤 | 超额(vs BH) |")
    lines.append("|----|------|---------|---------|---------|---------|------------|")
    lines.append(f"| (a) 真实门控 | E 版本 MACD 背驰进出场 | {n_trades} | "
                 f"{avg_hold:.0f} bars | {real.compound:+.2f}% | {real.max_dd:+.2f}% | "
                 f"{real.excess_vs_bh:+.2f}% |")
    lines.append(f"| (b) 随机门控 | 相同暴露，均匀随机 entry（{N_BOOTSTRAP} 次均值±std） | "
                 f"{n_trades} | {hold_bars} bars | {rnd.compound:+.2f}% ± {comp_std:.2f}% | "
                 f"{rnd.max_dd:+.2f}% ± {dd_std:.2f}% | {rnd.excess_vs_bh:+.2f}% |")
    lines.append(f"| (b') 随机门控 | 大样本中位数（{pct.n_runs} 次，稳健中心） | "
                 f"{n_trades} | {hold_bars} bars | {pct.rnd_median:+.2f}% (中位) | — | "
                 f"{pct.rnd_median - bh:+.2f}% |")
    lines.append(f"| (c) Buy-hold | 全程持有 | 1 | {n_bars:,} bars | {bh:+.2f}% | — | 0.00% |")
    lines.append("")
    lines.append(f"> 注：(b) 的 std (**{comp_std:.0f}%**) > mean (**{rnd.compound:.0f}%**) "
                 "⟹ 分布重尾，均值不是稳健中心，应看 (b') 中位数与下方秩统计。\n")

    lines.append(f"### 随机门控 bootstrap 明细（用户指定 {N_BOOTSTRAP} 次）\n")
    lines.append("| run | seed | 复利% | 最大回撤% | 是否跑赢真实门控 |")
    lines.append("|-----|------|-------|-----------|-----------------|")
    for i, (c, d) in enumerate(zip(comps, dds)):
        beat = "是" if c >= real.compound else "否"
        lines.append(f"| {i} | {BASE_SEED + i} | {c:+.2f} | {d:+.2f} | {beat} |")
    lines.append(f"| **均值** | — | **{rnd.compound:+.2f} ± {comp_std:.2f}** | "
                 f"**{rnd.max_dd:+.2f} ± {dd_std:.2f}** | — |")
    lines.append("")

    lines.append(f"### 秩统计（大样本 {pct.n_runs} 次，主判决依据）\n")
    lines.append("| 量 | 值 | 含义 |")
    lines.append("|----|----|------|")
    lines.append(f"| P(随机 ≥ 真实) | **{pct.p_random_ge_e:.1%}** | "
                 "≈50% ⟹ 无择时；≈0% ⟹ 门控胜；≈100% ⟹ 门控负 |")
    lines.append(f"| E 所处百分位 | **{pct.e_percentile:.0f}** | "
                 "真实门控在随机分布中的相对位置 |")
    lines.append(f"| 随机中位数 | {pct.rnd_median:+.2f}% | 稳健中心（vs 真实 "
                 f"{real.compound:+.2f}%，vs BH {bh:+.2f}%） |")
    lines.append(f"| 随机均值 ± std | {pct.rnd_mean:+.2f}% ± {pct.rnd_std:.2f}% | "
                 "重尾，均值被极端值拉高 |")
    lines.append("")

    lines.append("## 判决\n")
    lines.append(v + "\n")
    lines.append("### 判决标准（秩统计版，替代失效的 ±1σ）\n")
    lines.append("- P(随机≥真实) ∈ [33%, 67%]：E 与随机不可区分 → **P3 成立（弱）**——"
                 "alpha = 暴露守恒")
    lines.append("- P(随机≥真实) < 33%：E 显著优于随机 → **P3 否证**——门控有择时 alpha")
    lines.append("- P(随机≥真实) > 67%：E 显著劣于随机 → **P3 强化**——门控负 alpha")
    lines.append("")

    lines.append("## 边界条件\n")
    lines.append(f"- **统计功效是第一约束**：N={n_trades} 笔交易内禀地限制了检验功效。"
                 "即便秩统计稳健，其置信区间在 N 如此小时仍宽——本判决的强度上限由 N 决定，"
                 "不是由 bootstrap 轮数决定（增加轮数只稳定百分位估计，不增加底层信息）。")
    lines.append("- **门控近乎满仓**：真实门控总暴露 ≈ "
                 f"{exposure_frac:.0f}%（6 笔 × 占序列 {single_frac:.0f}% 的持有），"
                 "结构上接近 buy-hold——这本身说明该门控在此数据上几乎不「择时」，"
                 "只是带了 6 个进出点的全程持有。")
    lines.append("- 单标的单时段（OKLO 2024-05~2026-06，强趋势上行段）。换标的/换 regime"
                 "（震荡市或下行段）结论可能翻转——上行段任何长持有都赚钱，掩盖择时差异。")
    lines.append("- 随机门控允许时间重叠（纯均匀有放回采样），暴露守恒由 ∑hold_bars 不变保证；"
                 "若改为 non-overlap 采样，暴露语义改变，需重新判决。")
    lines.append("- 平均持有时长取整（avg_hold → hold_bars）引入 <1 bar 的暴露偏差，"
                 "对 447K bar 量级可忽略。")
    lines.append("- 零成本假设：引入交易成本后，交易笔数多的一组会被额外惩罚（本例两组 N 同，无差异）。")
    lines.append("")

    lines.append("## 谱系引用\n")
    lines.append("- `project_backtest_benchmark_falsifiability`：随机门控对照是「可证伪性补强」"
                 "——给出相同暴露的随机基线，使「门控是否有择时 alpha」成为可证伪命题。")
    lines.append("- `project_divergence_gate_entry_exit_falsified`：实验 D（背驰作过滤器）已被证伪；"
                 "本实验判决的是实验 E（背驰作定位器）的择时 alpha 是否真实。")
    lines.append("- `formalization-validity-domain`：本结论有效域 = 单标的单时段，非全 regime。")
    lines.append("")

    lines.append("## 影响声明\n")
    lines.append("- 新增 `analysis/p3_random_gate_control.py`（实验脚本）+ 本报告。")
    lines.append("- 不修改任何信号/引擎/交易逻辑（复用 m1_e_rust_engine / fugue_alpha_diagnosis）。")
    lines.append(f"- 运行耗时 {elapsed:.1f}s。")
    lines.append("")

    OUT_FILE.write_text("\n".join(lines))


if __name__ == "__main__":
    main()
