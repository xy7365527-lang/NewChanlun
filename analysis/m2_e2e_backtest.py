"""M2 选股层 → M1 背驰定位器（E 版本）端到端回测。

管线全链贯通验证（用户任务："先用 QQQ + OKLO 验证管线连通"）：

  dated ES/GC/CL 1min
      │ 60min 聚合 + RecursiveOrchestrator（level-0 move 方向，按日采样）
      ▼
  σ(E/$), σ(Au/$), σ(Oil/$)  →  Configuration Γ(date)
      │ k4_integration.build_pool_from_config  ←── 本次集成的胶水层
      ▼
  品种池+方向表（含 K4 极性 S、ω regime、逐标的置信度）
      │ k4_integration.long_allowed（LOW 置信 → 拦截做多入场）
      ▼
  E 版本（背驰定位器，run_swing_trading MODE_NONE，纯多头）
      │ E+M2 = 在 K4 risk-off 日掩码入场触发（信号磁带掩码，引擎逐位复用）
      ▼
  对比表：E（无选股） vs E+M2（K4 选股驱动方向） vs Buy&Hold

═══ 认识论标注（formalization-validity-domain 规则）═══
  - 管线连通性（本脚本能跑通、各层对接正确）：L1。
  - E vs E+M2 的真实数据收益对比：L2（可产生否定性结果）。
  - regime_from_config 的方向假设（ω→regime）曾被反向证伪
    （project_omega_regime_falsified, p=0.069）→ 预期 E+M2 很可能**逊于** E
    （门控在股票牛市中削减暴露 = project_divergence_gate_entry_exit_falsified 模式）。
    若结果如此，是**确认既有否定性结论**，不得包装为 M2 选股"有效"。

═══ 无前视保证 ═══
  date X 的盘中 bar 只使用 date < X（严格早于）的 K4 regime（滞后一日）。

═══ 边界条件 ═══
  - QQQ 仅有带时间戳 1min ≈ 1 个月（qqq_1m_3mo，2026-03~04）→ 小样本，统计功效低。
    OKLO 带时间戳 1min 覆盖 2024-05~2026-06 → 主力严格检验标的。
  - 期货 dates 为 UTC，股票 ts 为本地时区 → 日界处理用"最近的更早日"regime，
    对慢变宏观 regime 鲁棒（误差 <1 日，影响 <1% bar）。
  - σ_r 边按用户 M2 规格绑定为原油(CL)，区别于 k4_scanner 的利率(TLT)绑定。

谱系引用：project_omega_regime_falsified、project_divergence_locator_entry_exit、
project_divergence_gate_entry_exit_falsified、project_config_sigma_ontology（527号）、
project_backtest_benchmark_falsifiability。
"""

from __future__ import annotations

import bisect
import json
import sys
import time
from dataclasses import replace
from datetime import datetime, timedelta
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "src"))
sys.path.insert(0, str(ROOT / "analysis"))

from newchan.orchestrator.recursive import RecursiveOrchestrator  # noqa: E402
from newchan.strategy.k4_integration import (  # noqa: E402
    build_pool_from_config,
    long_allowed,
)
from newchan.topology.config_space import Configuration, WalkDirection  # noqa: E402
from newchan.topology.k4_scanner import walk_direction_from_snapshot  # noqa: E402
from newchan.types import Bar  # noqa: E402

import fugue_alpha_diagnosis as ef  # noqa: E402
from fugue_alpha_diagnosis import (  # noqa: E402
    INITIAL_CAPITAL,
    MODE_NONE,
    BarSignal,
    compute_metrics,
    compute_signals,
    run_swing_trading,
)

DATA_DIR = ROOT / "analysis" / "data_cache"
OUTPUT_MD = ROOT / "analysis" / "m2_e2e_backtest.md"
OUTPUT_JSON = DATA_DIR / "m2_e2e_backtest_results.json"

# K4 三条边的连续期货（dated）。σ_e=ES, σ_c=GC(金), σ_r=CL(油)。
K4_FILES = {
    "E": DATA_DIR / "es_1m_databento.json",
    "Au": DATA_DIR / "gc_1m_databento.json",
    "Oil": DATA_DIR / "cl_1m_databento.json",
}

# 交易标的（带时间戳 1min；bars[].ts 格式）。
SYMBOL_DATED = {
    "OKLO": DATA_DIR / "oklo_1m_databento.json",
    "QQQ": DATA_DIR / "qqq_1m_3mo.json",
}

# K4 regime 计算的工作周期（分钟）。60min L0 move 方向分布健康
# （日线全 FLAT；240min 过稀）——见探索记录。
K4_TF_MINUTES = 60


# ════════════════════════════════════════════════════════════
# 1) 期货 → 60min 聚合 → 逐日 σ（level-0 走势方向，无前视）
# ════════════════════════════════════════════════════════════


def _aggregate_60min(path: Path) -> list[tuple[str, float, float, float, float]]:
    """1min 期货 → 60min bar。返回 [(date_str, o, h, l, c)]，按时间序。

    date_str 取 UTC 日期（dates 字段前 10 字符），用于后续按日采样 σ。
    """
    d = json.loads(path.read_text())
    dates, o, h, lo, c = (
        d["dates"], d["opens"], d["highs"], d["lows"], d["closes"]
    )
    out: list[tuple[str, float, float, float, float]] = []
    cur_key: str | None = None
    cur: list[float] | None = None
    cur_day: str = ""
    for i, ts in enumerate(dates):
        key = ts[:13]  # 'YYYY-MM-DD HH'
        if key != cur_key:
            if cur is not None:
                out.append((cur_day, cur[0], cur[1], cur[2], cur[3]))
            cur_key = key
            cur = [o[i], h[i], lo[i], c[i]]
            cur_day = ts[:10]
        else:
            assert cur is not None
            cur[1] = max(cur[1], h[i])
            cur[2] = min(cur[2], lo[i])
            cur[3] = c[i]
    if cur is not None:
        out.append((cur_day, cur[0], cur[1], cur[2], cur[3]))
    return out


def _sigma_by_date(path: Path, tag: str) -> dict[str, WalkDirection]:
    """流式跑引擎，返回 date_str → 该日收盘时的 level-0 走势方向。

    引擎逐 60min bar 处理，每根 bar 后快照 walk_direction；同一日多根 bar 的最后一根
    的读数作为该日 σ（= 该日收盘时已知的走势方向，无前视）。
    """
    bars = _aggregate_60min(path)
    orch = RecursiveOrchestrator(stream_id=f"k4_{tag}", max_levels=2)
    base = datetime(2020, 1, 1)
    by_date: dict[str, WalkDirection] = {}
    for j, (day, oo, hh, ll, cc) in enumerate(bars):
        bar = Bar(
            ts=base + timedelta(hours=j),
            open=oo, high=hh, low=ll, close=cc, volume=0.0,
        )
        snap = orch.process_bar(bar)
        by_date[day] = walk_direction_from_snapshot(snap, level=0)
    return by_date


def build_regime_timeline() -> list[tuple[str, Configuration]]:
    """构造逐日 K4 配置时间线：[(date_str, Configuration)]，按日期升序。

    三条边各自的 σ(date) 用 carry-forward 对齐（某市场休市当日沿用最近已知 σ）。
    """
    t0 = time.time()
    sig_e = _sigma_by_date(K4_FILES["E"], "E")
    sig_au = _sigma_by_date(K4_FILES["Au"], "Au")
    sig_oil = _sigma_by_date(K4_FILES["Oil"], "Oil")
    print(f"  K4 σ 计算完成（{time.time() - t0:.1f}s）")

    all_dates = sorted(set(sig_e) | set(sig_au) | set(sig_oil))

    def _carry(series: dict[str, WalkDirection], dates: list[str]) -> dict[str, WalkDirection]:
        out: dict[str, WalkDirection] = {}
        last = WalkDirection.FLAT
        for dd in dates:
            if dd in series:
                last = series[dd]
            out[dd] = last
        return out

    ce = _carry(sig_e, all_dates)
    ca = _carry(sig_au, all_dates)
    co = _carry(sig_oil, all_dates)

    timeline = [
        (dd, Configuration(sigma_e=ce[dd], sigma_c=ca[dd], sigma_r=co[dd]))
        for dd in all_dates
    ]
    return timeline


# ════════════════════════════════════════════════════════════
# 2) 无前视 regime 查询 + 逐日做多放行表
# ════════════════════════════════════════════════════════════


def allow_long_by_date(
    symbol: str,
    bar_dates: list[str],
    timeline: list[tuple[str, Configuration]],
) -> dict[str, bool]:
    """对该标的，逐"唯一交易日"判定 K4 是否放行做多入场（无前视，滞后一日）。

    date X 使用 timeline 中 date < X（严格早于）的最近配置。X 早于时间线起点 →
    无 K4 信息 → 默认放行（不凭空门控）。
    """
    tl_dates = [d for d, _ in timeline]
    cache: dict[str, bool] = {}
    for dd in dict.fromkeys(bar_dates):  # 去重保序
        idx = bisect.bisect_left(tl_dates, dd) - 1  # 严格早于 dd 的最近一日
        if idx < 0:
            cache[dd] = True
            continue
        config = timeline[idx][1]
        pool = build_pool_from_config((symbol,), config)
        entry = pool.entries[0]
        cache[dd] = long_allowed(entry)
    return cache


# ════════════════════════════════════════════════════════════
# 3) 标的数据加载 + 信号磁带门控
# ════════════════════════════════════════════════════════════


def load_symbol_dated(
    path: Path,
) -> tuple[list[float], list[float], list[float], list[float], list[str]]:
    """加载带时间戳 1min（bars[].ts/open/high/low/close）→ OHLC 数组 + 逐 bar 日期。"""
    d = json.loads(path.read_text())
    bars = d["bars"]
    opens = [float(b["open"]) for b in bars]
    highs = [float(b["high"]) for b in bars]
    lows = [float(b["low"]) for b in bars]
    closes = [float(b["close"]) for b in bars]
    bdates = [str(b["ts"])[:10] for b in bars]
    return opens, highs, lows, closes, bdates


def gate_signals(
    signals: list[BarSignal],
    bar_dates: list[str],
    allow_map: dict[str, bool],
) -> tuple[list[BarSignal], int]:
    """E+M2 信号磁带：K4 不放行的日，掩码该 bar 的入场触发（down_move_settled=False）。

    MODE_NONE 下 down_move_settled 仅用于 FLAT→LONG 入场判定，故掩码它精确等价于
    "该日不开新多仓"，不触碰任何离场/持仓逻辑（保守入场门控）。引擎逐位复用。
    返回 (门控后磁带, 被掩码的入场触发数)。
    """
    masked = 0
    out: list[BarSignal] = []
    for i, sig in enumerate(signals):
        if not allow_map.get(bar_dates[i], True) and sig.down_move_settled:
            out.append(replace(sig, down_move_settled=False))
            masked += 1
        else:
            out.append(sig)
    return out, masked


def buy_and_hold_pct(closes: list[float]) -> float:
    """全程买入持有收益%（基准）。"""
    if len(closes) < 2 or closes[0] <= 0:
        return 0.0
    return (closes[-1] / closes[0] - 1.0) * 100.0


# ════════════════════════════════════════════════════════════
# 4) 单标的回测
# ════════════════════════════════════════════════════════════


def backtest_symbol(
    symbol: str,
    timeline: list[tuple[str, Configuration]],
) -> dict:
    print(f"\n── {symbol} ──")
    path = SYMBOL_DATED[symbol]
    opens, highs, lows, closes, bdates = load_symbol_dated(path)
    n = len(closes)
    print(f"  bars={n}  日期 {bdates[0]} → {bdates[-1]}")

    t0 = time.time()
    signals = compute_signals(opens, highs, lows, closes)
    print(f"  信号计算完成（{time.time() - t0:.1f}s）")

    # E：无选股，纯背驰定位器
    trades_e, _ = run_swing_trading(signals, MODE_NONE)
    m_e = compute_metrics(trades_e)

    # E+M2：K4 选股门控入场
    allow_map = allow_long_by_date(symbol, bdates, timeline)
    n_block_days = sum(1 for v in allow_map.values() if not v)
    gated, masked = gate_signals(signals, bdates, allow_map)
    trades_m2, _ = run_swing_trading(gated, MODE_NONE)
    m_m2 = compute_metrics(trades_m2)

    bh = buy_and_hold_pct(closes)

    print(f"  放行日={len(allow_map) - n_block_days}/{len(allow_map)}  "
          f"拦截日={n_block_days}  掩码入场触发={masked}")
    print(f"  E      : n={m_e['n']:3d}  收益={m_e['total_compound']:+8.2f}%  "
          f"胜率={m_e['win_rate']:.1f}%")
    print(f"  E+M2   : n={m_m2['n']:3d}  收益={m_m2['total_compound']:+8.2f}%  "
          f"胜率={m_m2['win_rate']:.1f}%")
    print(f"  Buy&Hold: {bh:+.2f}%")

    return {
        "symbol": symbol,
        "bars": n,
        "date_start": bdates[0],
        "date_end": bdates[-1],
        "unique_days": len(allow_map),
        "blocked_days": n_block_days,
        "masked_entries": masked,
        "buy_hold_pct": round(bh, 4),
        "E": m_e,
        "E_M2": m_m2,
    }


# ════════════════════════════════════════════════════════════
# 5) 报告
# ════════════════════════════════════════════════════════════


def write_report(results: list[dict], timeline: list[tuple[str, Configuration]]) -> None:
    from collections import Counter

    from newchan.strategy.k4_integration import polarity_from_config, regime_from_config

    reg_dist = Counter(regime_from_config(c).value for _, c in timeline)
    pol_dist = Counter(polarity_from_config(c) for _, c in timeline)

    lines: list[str] = []
    lines.append("# M2 选股层 → M1 背驰定位器（E）端到端回测")
    lines.append("")
    lines.append(f"工作周期（K4 regime）：{K4_TF_MINUTES}min 聚合期货，level-0 走势方向。")
    lines.append(f"K4 时间线天数：{len(timeline)}（{timeline[0][0]} → {timeline[-1][0]}）。")
    lines.append("")
    lines.append("## K4 regime / 极性分布（全时间线）")
    lines.append("")
    lines.append("| regime | 天数 |")
    lines.append("|--------|------|")
    for k, v in reg_dist.most_common():
        lines.append(f"| {k} | {v} |")
    lines.append("")
    lines.append("| 极性 S | 天数 |")
    lines.append("|--------|------|")
    for k in sorted(pol_dist):
        lines.append(f"| {k:+d} | {pol_dist[k]} |")
    lines.append("")
    lines.append("## 对比表")
    lines.append("")
    lines.append("| 标的 | 版本 | 交易数 | 复利收益% | 胜率% | Sharpe | MaxDD% |")
    lines.append("|------|------|--------|-----------|-------|--------|--------|")
    for r in results:
        for ver, key in (("E（无选股，纯背驰）", "E"), ("E+M2（K4选股驱动方向）", "E_M2")):
            m = r[key]
            lines.append(
                f"| {r['symbol']} | {ver} | {m['n']} | "
                f"{m['total_compound']:+.2f} | {m['win_rate']:.1f} | "
                f"{m['sharpe']:.3f} | {m['max_dd']:.2f} |"
            )
        lines.append(
            f"| {r['symbol']} | Buy&Hold | — | {r['buy_hold_pct']:+.2f} | — | — | — |"
        )
    lines.append("")
    lines.append("## 门控统计")
    lines.append("")
    lines.append("| 标的 | bars | 起 | 止 | 交易日 | K4拦截日 | 掩码入场 |")
    lines.append("|------|------|----|----|--------|----------|----------|")
    for r in results:
        lines.append(
            f"| {r['symbol']} | {r['bars']} | {r['date_start']} | {r['date_end']} | "
            f"{r['unique_days']} | {r['blocked_days']} | {r['masked_entries']} |"
        )
    lines.append("")
    lines.append("## 结果包六要素")
    lines.append("")
    lines.append("**结论**：见上述对比表（E vs E+M2 vs Buy&Hold）。")
    lines.append("")
    lines.append("**定义依据**：")
    lines.append("- K4 配置 Γ=(σ(E/$),σ(Au/$),σ(Oil/$))，σ=RecursiveOrchestrator level-0 "
                 "settled-move 方向（缠论走势方向，第31课盘整→FLAT）。")
    lines.append("- 极性 S=polarity_index(Γ)；regime=regime_from_config（金/油相对方向→ω）。")
    lines.append("- 方向表=selection_pool.apply_regime_adjustment（regime+S→置信度）；"
                 "门控=long_allowed（LOW→拦截做多）。")
    lines.append("- E=run_swing_trading(MODE_NONE)：底背驰买入、L2趋势顶背驰清仓，纯多头。")
    lines.append("")
    lines.append("**边界条件**：")
    lines.append("- QQQ 带时间戳 1min 仅约 1 个月（小样本，统计功效低）；OKLO 覆盖约 2 年。")
    lines.append("- K4 工作周期=60min（日线在 move 层级全 FLAT，无法产生 regime）。")
    lines.append("- 无前视：date X 仅用 date<X 的 regime（滞后一日）。")
    lines.append("- σ_r 绑定为原油(CL)（用户 M2 规格），区别于 k4_scanner 的利率(TLT)。")
    lines.append("- 门控仅作用于入场，不强制平仓；改 selection_pool 置信度逻辑会翻转结果。")
    lines.append("")
    lines.append("**下游推论**：")
    lines.append("- 若 E+M2 < E：确认门控在股票牛市削减暴露的既有否定性结论"
                 "（project_divergence_gate_entry_exit_falsified / omega_regime_falsified）。")
    lines.append("- 若 E+M2 > E：K4 金/油 regime 对股票方向有择时价值——但需 L3 多标的/多时段交叉验证才可声明。")
    lines.append("- 管线本身贯通（各层对接无误）属 L1，与收益方向无关。")
    lines.append("")
    lines.append("**谱系引用**：project_omega_regime_falsified、"
                 "project_divergence_locator_entry_exit、"
                 "project_divergence_gate_entry_exit_falsified、527号（config σ 走势方向态）。")
    lines.append("")
    lines.append("**影响声明**：新建胶水层 src/newchan/strategy/k4_integration.py + 本回测脚本；"
                 "不修改引擎/selection_pool/config_space。")
    lines.append("")
    lines.append("**认识论等级**：管线连通 L1；收益对比 L2（单标的，可证伪；非 L3 交叉验证）。")
    lines.append("")

    OUTPUT_MD.write_text("\n".join(lines))
    print(f"\n报告 → {OUTPUT_MD}")


# ════════════════════════════════════════════════════════════
# 主入口
# ════════════════════════════════════════════════════════════


def main() -> None:
    print("═" * 60)
    print("  M2 端到端回测：K4 选股 → E 背驰定位器")
    print("═" * 60)
    print("\n[1] 构造 K4 regime 时间线（ES/GC/CL → 60min → 逐日 σ）")
    timeline = build_regime_timeline()

    results: list[dict] = []
    for symbol in ("QQQ", "OKLO"):
        results.append(backtest_symbol(symbol, timeline))

    write_report(results, timeline)
    OUTPUT_JSON.write_text(json.dumps(results, indent=2, ensure_ascii=False))
    print(f"结果 JSON → {OUTPUT_JSON}")
    print("\n" + "═" * 60)
    print("  完成")
    print("═" * 60)


if __name__ == "__main__":
    main()
