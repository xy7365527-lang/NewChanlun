"""tightness-selection-l2: T(S) 选股效果 L2 验证。

350号下游推论4：高 T(S) 标的是否确实产生更高质量的买卖点？

方法：滚动窗口前向收益法（Rolling Window Forward Return）。
- 对每个标的跑引擎全量历史
- 每个 bar 记录当前结构紧度 T(S) 和买卖点状态
- 当 confirmed BSP 首次出现时，记录触发 bar 和前向收益
- 也收集背驰事件（Divergence）作为"准买卖点"以增加样本量
- 同时记录走势完成事件（Move settled）作为结构性信号

关键改进（相对第一版）：
1. 不做训练/测试切分——T(S) 是瞬时量，每个事件触发时刻的 T 就是分组依据
2. 收集三类事件（BSP/Divergence/MoveSettle）以获得足够样本
3. T(S) 使用连续值（实时计算），不是历史最大值

认识论等级：L2（真实数据验证，单时段多标的）。

谱系引用：350号、350号下游推论4、267号。
"""

from __future__ import annotations

import json
import logging
import math
import sys
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

_project_root = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(_project_root / "src"))

from newchan.orchestrator.recursive import RecursiveOrchestrator
from newchan.types import Bar

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(name)s %(message)s")
logger = logging.getLogger("tightness_selection_l2")

# ═══════════════════════════════════════════════════════════════
# 标的配置
# ═══════════════════════════════════════════════════════════════

SYMBOLS: dict[str, dict] = {
    "600547": {"name": "山东黄金", "sector": "au"},
    "002155": {"name": "湖南黄金", "sector": "au"},
    "600489": {"name": "中金黄金", "sector": "au"},
    "600028": {"name": "中国石化", "sector": "oil"},
    "601857": {"name": "中国石油", "sector": "oil"},
    "600688": {"name": "上海石化", "sector": "oil"},
    "601398": {"name": "工商银行", "sector": "bond"},
    "601288": {"name": "农业银行", "sector": "bond"},
    "600048": {"name": "保利发展", "sector": "re"},
    "000002": {"name": "万科A", "sector": "re"},
    "001979": {"name": "招商蛇口", "sector": "re"},
    "002230": {"name": "科大讯飞", "sector": "tech"},
    "300059": {"name": "东方财富", "sector": "tech"},
    "000725": {"name": "京东方A", "sector": "tech"},
    "601318": {"name": "中国平安", "sector": "finance"},
    "600036": {"name": "招商银行", "sector": "finance"},
    "601166": {"name": "兴业银行", "sector": "finance"},
    "600519": {"name": "贵州茅台", "sector": "consumer"},
    "000858": {"name": "五粮液", "sector": "consumer"},
    "002304": {"name": "洋河股份", "sector": "consumer"},
    "300760": {"name": "迈瑞医疗", "sector": "pharma"},
    "600276": {"name": "恒瑞医药", "sector": "pharma"},
    "601985": {"name": "中国核电", "sector": "energy"},
    "600900": {"name": "长江电力", "sector": "energy"},
}

DATA_DIR = _project_root / "data" / "scanner_l2"
OUTPUT_DIR = _project_root / "data" / "tightness_selection_l2"

FORWARD_WINDOWS = (5, 10, 20, 40)
MIN_WARMUP_BARS = 200  # 引擎预热期，跳过前 200 bars


# ═══════════════════════════════════════════════════════════════
# 数据加载
# ═══════════════════════════════════════════════════════════════


def _load_bars(symbol: str) -> list[Bar]:
    path = DATA_DIR / f"{symbol}_daily.json"
    if not path.exists():
        return []
    data = json.loads(path.read_text(encoding="utf-8"))
    bars = [
        Bar(
            ts=datetime.fromisoformat(d["ts"]),
            open=d["open"],
            high=d["high"],
            low=d["low"],
            close=d["close"],
            volume=d["volume"],
        )
        for d in data
    ]
    bars.sort(key=lambda b: b.ts)
    return bars


# ═══════════════════════════════════════════════════════════════
# 结构紧度计算
# ═══════════════════════════════════════════════════════════════


def _structural_tightness(snap) -> float:
    """实时结构紧度——当前 bar 时刻的瞬时 T(S)。

    与 scanner_l2 的 max-over-history 不同，这是瞬时值。

    分量：
    - has_confirmed_bsp: +1.0
    - has_moves: +0.5
    - has_zhongshus: +0.3
    - 每个有 moves 的递归层: +1.0
    - 每个仅有 zhongshus 的递归层: +0.5
    """
    t = 0.0

    if any(bp.confirmed for bp in snap.bsp_snapshot.buysellpoints):
        t += 1.0

    if len(snap.move_snapshot.moves) > 0:
        t += 0.5

    if len(snap.zs_snapshot.zhongshus) > 0:
        t += 0.3

    for rs in snap.recursive_snapshots:
        if len(rs.moves) > 0:
            t += 1.0
        elif len(rs.zhongshus) > 0:
            t += 0.5

    return t


# ═══════════════════════════════════════════════════════════════
# 事件记录
# ═══════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class SignalRecord:
    """单个信号事件 + 前向收益。"""

    symbol: str
    signal_type: str      # bsp_buy / bsp_sell / div_bottom / div_top / move_settle
    bar_idx: int
    price: float
    tightness: float      # 触发时刻的瞬时 T(S)
    fwd_returns: dict[int, float | None]
    mfe: dict[int, float | None]
    mae: dict[int, float | None]


def _forward_metrics(
    bars: list[Bar],
    trigger_idx: int,
    trigger_price: float,
    is_bullish: bool,
    windows: tuple[int, ...],
) -> tuple[dict[int, float | None], dict[int, float | None], dict[int, float | None]]:
    """计算前向收益、MFE、MAE。"""
    sign = 1.0 if is_bullish else -1.0
    fwd: dict[int, float | None] = {}
    mfe: dict[int, float | None] = {}
    mae: dict[int, float | None] = {}

    if trigger_price <= 0:
        for k in windows:
            fwd[k] = None
            mfe[k] = None
            mae[k] = None
        return fwd, mfe, mae

    for k in windows:
        end_idx = trigger_idx + k
        if end_idx >= len(bars):
            fwd[k] = None
            mfe[k] = None
            mae[k] = None
            continue

        fwd[k] = sign * (bars[end_idx].close - trigger_price) / trigger_price

        best = 0.0
        worst = 0.0
        for j in range(trigger_idx + 1, end_idx + 1):
            if j >= len(bars):
                break
            if is_bullish:
                up = (bars[j].high - trigger_price) / trigger_price
                down = (bars[j].low - trigger_price) / trigger_price
            else:
                up = (trigger_price - bars[j].low) / trigger_price
                down = (trigger_price - bars[j].high) / trigger_price
            best = max(best, up)
            worst = min(worst, down)

        mfe[k] = best
        mae[k] = worst

    return fwd, mfe, mae


def collect_signals(symbol: str, bars: list[Bar]) -> list[SignalRecord]:
    """对单个标的跑引擎全量历史，收集三类信号事件。

    1. BSP（confirmed 买卖点）
    2. Divergence（背驰事件，从 snap 的域事件中提取）
    3. Move settled（走势确认——结构性信号）

    每类事件都附带触发时刻的瞬时 T(S) 和前向收益。
    """
    orch = RecursiveOrchestrator(stream_id=symbol, max_levels=4)
    records: list[SignalRecord] = []

    seen_bsp: set[tuple] = set()
    seen_div: set[tuple] = set()
    seen_move: set[tuple] = set()

    for i, bar in enumerate(bars):
        snap = orch.process_bar(bar)

        if i < MIN_WARMUP_BARS:
            continue

        t = _structural_tightness(snap)

        # 1. BSP 信号
        for bp in snap.bsp_snapshot.buysellpoints:
            if not bp.confirmed:
                continue
            key = (bp.seg_idx, bp.kind, bp.side, bp.level_id)
            if key in seen_bsp:
                continue
            seen_bsp.add(key)

            is_bullish = bp.side == "buy"
            sig_type = f"bsp_{bp.side}"
            price = bp.price if bp.price > 0 else bar.close

            f, m, a = _forward_metrics(bars, i, price, is_bullish, FORWARD_WINDOWS)
            records.append(SignalRecord(
                symbol=symbol, signal_type=sig_type, bar_idx=i,
                price=price, tightness=t,
                fwd_returns=f, mfe=m, mae=a,
            ))

        # 2. Divergence 信号（从域事件中提取）
        for ev in snap.all_events:
            ev_type = getattr(ev, "event_type", "")
            if "divergence" not in ev_type.lower():
                continue

            div_direction = getattr(ev, "direction", None)
            div_seg_c = getattr(ev, "seg_c_end", None)
            div_level = getattr(ev, "level_id", 1)
            if div_direction is None or div_seg_c is None:
                continue

            key = (div_seg_c, div_direction, div_level)
            if key in seen_div:
                continue
            seen_div.add(key)

            is_bullish = div_direction == "bottom"
            sig_type = f"div_{div_direction}"

            f, m, a = _forward_metrics(bars, i, bar.close, is_bullish, FORWARD_WINDOWS)
            records.append(SignalRecord(
                symbol=symbol, signal_type=sig_type, bar_idx=i,
                price=bar.close, tightness=t,
                fwd_returns=f, mfe=m, mae=a,
            ))

        # 3. Move settled 信号
        for ev in snap.all_events:
            ev_type = getattr(ev, "event_type", "")
            if "move_settle" not in ev_type.lower():
                continue

            m_direction = getattr(ev, "direction", None)
            m_seg_start = getattr(ev, "seg_start", None)
            if m_direction is None or m_seg_start is None:
                continue

            key = (m_seg_start, m_direction)
            if key in seen_move:
                continue
            seen_move.add(key)

            # Move settle 后反转概率——下跌走势完成 → 买入信号，上涨走势完成 → 卖出信号
            is_bullish = m_direction == "down"
            sig_type = "move_settle"

            f, m_v, a = _forward_metrics(bars, i, bar.close, is_bullish, FORWARD_WINDOWS)
            records.append(SignalRecord(
                symbol=symbol, signal_type=sig_type, bar_idx=i,
                price=bar.close, tightness=t,
                fwd_returns=f, mfe=m_v, mae=a,
            ))

    return records


# ═══════════════════════════════════════════════════════════════
# 统计分析
# ═══════════════════════════════════════════════════════════════


def _group_stats(
    name: str,
    records: list[SignalRecord],
    windows: tuple[int, ...],
) -> dict:
    """计算一组的汇总统计。"""
    if not records:
        return {
            "group": name, "n": 0, "avg_tightness": 0,
            "per_window": {k: {} for k in windows},
        }

    per_window = {}
    for k in windows:
        valid = [(r.fwd_returns[k], r.mfe.get(k), r.mae.get(k))
                 for r in records if r.fwd_returns.get(k) is not None]
        n_valid = len(valid)
        if n_valid == 0:
            per_window[k] = {"n": 0}
            continue

        fwd_vals = [v[0] for v in valid]
        mfe_vals = [v[1] for v in valid if v[1] is not None]
        mae_vals = [v[2] for v in valid if v[2] is not None]

        mean_fwd = sum(fwd_vals) / n_valid
        median_fwd = sorted(fwd_vals)[n_valid // 2]
        win_rate = sum(1 for v in fwd_vals if v > 0) / n_valid
        mean_mfe = sum(mfe_vals) / len(mfe_vals) if mfe_vals else 0
        mean_mae = sum(mae_vals) / len(mae_vals) if mae_vals else 0

        per_window[k] = {
            "n": n_valid,
            "mean_fwd": round(mean_fwd, 6),
            "median_fwd": round(median_fwd, 6),
            "win_rate": round(win_rate, 4),
            "mean_mfe": round(mean_mfe, 6),
            "mean_mae": round(mean_mae, 6),
        }

    return {
        "group": name,
        "n": len(records),
        "avg_tightness": round(sum(r.tightness for r in records) / len(records), 4),
        "by_type": {
            sig: len([r for r in records if r.signal_type == sig])
            for sig in sorted(set(r.signal_type for r in records))
        },
        "per_window": per_window,
    }


def _wilcoxon_rank_sum(xs: list[float], ys: list[float]) -> dict:
    """Wilcoxon rank-sum test（手动实现）。"""
    n1, n2 = len(xs), len(ys)
    if n1 == 0 or n2 == 0:
        return {"U": None, "z": None, "p_approx": None, "n1": n1, "n2": n2}

    combined = [(v, "x") for v in xs] + [(v, "y") for v in ys]
    combined.sort(key=lambda t: t[0])

    ranks: list[float] = [0.0] * len(combined)
    i = 0
    while i < len(combined):
        j = i
        while j < len(combined) and combined[j][0] == combined[i][0]:
            j += 1
        avg_rank = (i + 1 + j) / 2.0
        for k in range(i, j):
            ranks[k] = avg_rank
        i = j

    r1 = sum(ranks[k] for k in range(len(combined)) if combined[k][1] == "x")
    u1 = r1 - n1 * (n1 + 1) / 2.0
    u2 = n1 * n2 - u1
    u = min(u1, u2)

    result: dict = {"U": round(u, 2), "n1": n1, "n2": n2}

    if n1 + n2 >= 20:
        mu = n1 * n2 / 2.0
        sigma = math.sqrt(n1 * n2 * (n1 + n2 + 1) / 12.0)
        z = (u - mu) / sigma if sigma > 0 else 0.0
        p = 2 * (1 - _normal_cdf(abs(z)))
        result["z"] = round(z, 4)
        result["p_approx"] = round(p, 6)
    else:
        result["z"] = None
        result["p_approx"] = None
        result["note"] = "sample too small for normal approximation"

    return result


def _normal_cdf(x: float) -> float:
    if x < 0:
        return 1 - _normal_cdf(-x)
    t = 1.0 / (1.0 + 0.2316419 * x)
    poly = t * (0.319381530 + t * (-0.356563782 + t * (1.781477937 + t * (-1.821255978 + t * 1.330274429))))
    return 1.0 - poly * math.exp(-x * x / 2.0) / math.sqrt(2.0 * math.pi)


# ═══════════════════════════════════════════════════════════════
# 主程序
# ═══════════════════════════════════════════════════════════════


def main() -> None:
    logger.info("=== T(S) Selection L2 Validation (v2) ===")
    logger.info("标的数: %d, 前向窗口: %s, 预热期: %d bars",
                len(SYMBOLS), FORWARD_WINDOWS, MIN_WARMUP_BARS)

    # 1. 收集所有信号
    all_records: list[SignalRecord] = []

    for symbol, info in SYMBOLS.items():
        bars = _load_bars(symbol)
        if len(bars) < MIN_WARMUP_BARS + 50:
            logger.warning("数据不足: %s (%s) — %d bars, 跳过", symbol, info["name"], len(bars))
            continue

        logger.info("处理: %s (%s) — %d bars", symbol, info["name"], len(bars))
        records = collect_signals(symbol, bars)
        all_records.extend(records)

        by_type: dict[str, int] = {}
        for r in records:
            by_type[r.signal_type] = by_type.get(r.signal_type, 0) + 1

        logger.info("  → %d 信号: %s", len(records),
                    ", ".join(f"{k}={v}" for k, v in sorted(by_type.items())))

    logger.info("总信号数: %d", len(all_records))

    if len(all_records) < 10:
        logger.error("信号不足（<10），无法进行有意义的 L2 验证")
        _write_insufficient_report(all_records)
        sys.exit(1)

    # 2. 按 T(S) 中位数分组（二分法，最大化每组样本量）
    all_t = sorted(set(r.tightness for r in all_records))
    if len(all_t) < 2:
        logger.warning("T(S) 无区分度（仅 %d 个不同值），尝试 T>median vs T<=median", len(all_t))

    t_values = [r.tightness for r in all_records]
    t_values.sort()
    t_median = t_values[len(t_values) // 2]

    high_t = [r for r in all_records if r.tightness > t_median]
    low_t = [r for r in all_records if r.tightness <= t_median]

    # 如果中位数切分导致一组为空，用均值切分
    if not high_t or not low_t:
        t_mean = sum(r.tightness for r in all_records) / len(all_records)
        high_t = [r for r in all_records if r.tightness > t_mean]
        low_t = [r for r in all_records if r.tightness <= t_mean]

    # 如果仍然无法分组（所有 T 相同），用上下半分
    if not high_t or not low_t:
        logger.warning("T(S) 完全相同，退化为前半 vs 后半按 bar_idx 排序")
        sorted_records = sorted(all_records, key=lambda r: r.bar_idx)
        mid = len(sorted_records) // 2
        high_t = sorted_records[:mid]
        low_t = sorted_records[mid:]

    logger.info("分组: high-T=%d (avg T=%.2f), low-T=%d (avg T=%.2f)",
                len(high_t),
                sum(r.tightness for r in high_t) / len(high_t),
                len(low_t),
                sum(r.tightness for r in low_t) / len(low_t))

    # 3. 组统计
    high_stats = _group_stats("high-T", high_t, FORWARD_WINDOWS)
    low_stats = _group_stats("low-T", low_t, FORWARD_WINDOWS)

    # 4. 统计检验（每个窗口）
    tests: dict[str, dict] = {}
    for k in FORWARD_WINDOWS:
        h_fwd = [r.fwd_returns[k] for r in high_t if r.fwd_returns.get(k) is not None]
        l_fwd = [r.fwd_returns[k] for r in low_t if r.fwd_returns.get(k) is not None]
        tests[str(k)] = _wilcoxon_rank_sum(h_fwd, l_fwd)

    # 5. 按信号类型分别检验（更细粒度）
    signal_types = sorted(set(r.signal_type for r in all_records))
    per_type_tests: dict[str, dict] = {}
    for sig_type in signal_types:
        sig_high = [r for r in high_t if r.signal_type == sig_type]
        sig_low = [r for r in low_t if r.signal_type == sig_type]
        per_type_tests[sig_type] = {}
        for k in FORWARD_WINDOWS:
            h_fwd = [r.fwd_returns[k] for r in sig_high if r.fwd_returns.get(k) is not None]
            l_fwd = [r.fwd_returns[k] for r in sig_low if r.fwd_returns.get(k) is not None]
            per_type_tests[sig_type][str(k)] = _wilcoxon_rank_sum(h_fwd, l_fwd)

    # 6. 组装报告
    report = {
        "meta": {
            "hypothesis": "H0: high-T signal quality == low-T signal quality",
            "method": "Rolling Window Forward Return + Wilcoxon rank-sum (v2)",
            "forward_windows": list(FORWARD_WINDOWS),
            "warmup_bars": MIN_WARMUP_BARS,
            "total_symbols": len(SYMBOLS),
            "total_signals": len(all_records),
            "t_median_split": round(t_median, 4),
            "epistemology_level": "L2",
            "signal_types_collected": signal_types,
        },
        "t_distribution": {
            "unique_values": len(all_t),
            "values": [round(v, 4) for v in all_t],
            "median": round(t_median, 4),
        },
        "groups": {
            "high_t": high_stats,
            "low_t": low_stats,
        },
        "aggregate_tests": tests,
        "per_type_tests": per_type_tests,
        "signal_details": [
            {
                "symbol": r.symbol,
                "name": SYMBOLS.get(r.symbol, {}).get("name", "?"),
                "type": r.signal_type,
                "bar_idx": r.bar_idx,
                "price": round(r.price, 4),
                "tightness": round(r.tightness, 4),
                "fwd": {str(k): round(v, 6) if v is not None else None for k, v in r.fwd_returns.items()},
                "mfe": {str(k): round(v, 6) if v is not None else None for k, v in r.mfe.items()},
                "mae": {str(k): round(v, 6) if v is not None else None for k, v in r.mae.items()},
            }
            for r in all_records
        ],
    }

    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    report_path = OUTPUT_DIR / "tightness_selection_l2_report.json"
    report_path.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    logger.info("报告已保存: %s", report_path)

    # 7. 打印摘要
    _print_summary(all_records, high_t, low_t, high_stats, low_stats, tests, per_type_tests, signal_types)


def _write_insufficient_report(records: list[SignalRecord]) -> None:
    """信号不足时写入最小报告。"""
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    report = {
        "meta": {"epistemology_level": "L2", "result": "insufficient_data"},
        "total_signals": len(records),
        "verdict": "blocked_by_data",
    }
    path = OUTPUT_DIR / "tightness_selection_l2_report.json"
    path.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")


def _print_summary(
    all_records, high_t, low_t, high_stats, low_stats, tests, per_type_tests, signal_types,
) -> None:
    print("\n" + "=" * 78)
    print("T(S) Selection L2 Validation v2 — 结果摘要")
    print("=" * 78)

    print(f"\n总信号数: {len(all_records)}")
    by_type: dict[str, int] = {}
    for r in all_records:
        by_type[r.signal_type] = by_type.get(r.signal_type, 0) + 1
    for sig, cnt in sorted(by_type.items()):
        print(f"  {sig}: {cnt}")

    print(f"\nHigh-T 组: {high_stats['n']} 信号, avg T={high_stats['avg_tightness']:.2f}")
    if high_stats.get("by_type"):
        for sig, cnt in sorted(high_stats["by_type"].items()):
            print(f"  {sig}: {cnt}")
    print(f"Low-T  组: {low_stats['n']} 信号, avg T={low_stats['avg_tightness']:.2f}")
    if low_stats.get("by_type"):
        for sig, cnt in sorted(low_stats["by_type"].items()):
            print(f"  {sig}: {cnt}")

    print("\n--- 聚合前向收益对比 ---")
    print(f"{'k':>4s} | {'High-T mean':>11s} {'med':>8s} {'win%':>6s} {'n':>5s} | "
          f"{'Low-T mean':>11s} {'med':>8s} {'win%':>6s} {'n':>5s} | {'p':>8s}")
    print("-" * 85)
    for k in FORWARD_WINDOWS:
        hw = high_stats["per_window"].get(k, {})
        lw = low_stats["per_window"].get(k, {})
        t = tests.get(str(k), {})

        h_m = hw.get("mean_fwd", 0)
        h_md = hw.get("median_fwd", 0)
        h_wr = hw.get("win_rate", 0)
        h_n = hw.get("n", 0)
        l_m = lw.get("mean_fwd", 0)
        l_md = lw.get("median_fwd", 0)
        l_wr = lw.get("win_rate", 0)
        l_n = lw.get("n", 0)
        p = t.get("p_approx")
        p_str = f"{p:.4f}" if p is not None else "N/A"
        sig = " *" if p is not None and p < 0.05 else ""
        print(f"  {k:2d} | {h_m:>10.4%} {h_md:>7.4%} {h_wr:>5.1%} {h_n:>5d} | "
              f"{l_m:>10.4%} {l_md:>7.4%} {l_wr:>5.1%} {l_n:>5d} | {p_str:>8s}{sig}")

    # 按信号类型细分
    for sig_type in signal_types:
        sig_high = [r for r in high_t if r.signal_type == sig_type]
        sig_low = [r for r in low_t if r.signal_type == sig_type]
        if not sig_high and not sig_low:
            continue
        print(f"\n--- {sig_type} 信号前向收益 (high-T={len(sig_high)}, low-T={len(sig_low)}) ---")

        if per_type_tests.get(sig_type):
            for k in FORWARD_WINDOWS:
                tt = per_type_tests[sig_type].get(str(k), {})
                h_fwd = [r.fwd_returns[k] for r in sig_high if r.fwd_returns.get(k) is not None]
                l_fwd = [r.fwd_returns[k] for r in sig_low if r.fwd_returns.get(k) is not None]
                h_mean = sum(h_fwd) / len(h_fwd) if h_fwd else 0
                l_mean = sum(l_fwd) / len(l_fwd) if l_fwd else 0
                p = tt.get("p_approx")
                p_str = f"p={p:.4f}" if p is not None else "p=N/A"
                sig_mark = " *" if p is not None and p < 0.05 else ""
                print(f"  k={k:2d}: H={h_mean:>8.4%} (n={len(h_fwd)}), "
                      f"L={l_mean:>8.4%} (n={len(l_fwd)}), {p_str}{sig_mark}")

    # 结论
    any_significant = any(
        tests.get(str(k), {}).get("p_approx") is not None and tests[str(k)]["p_approx"] < 0.05
        for k in FORWARD_WINDOWS
    )
    sufficient = all(
        high_stats["per_window"].get(k, {}).get("n", 0) >= 10
        and low_stats["per_window"].get(k, {}).get("n", 0) >= 10
        for k in FORWARD_WINDOWS[:2]
    )

    print("\n" + "=" * 78)
    print("认识论等级: L2（真实数据验证，单时段多标的）")
    print("验证命题: 350号下游推论4 — 高 T(S) 标的信号质量 > 低 T(S) 标的")

    if not sufficient:
        total_min = min(
            high_stats["per_window"].get(k, {}).get("n", 0) + low_stats["per_window"].get(k, {}).get("n", 0)
            for k in FORWARD_WINDOWS[:2]
        )
        print(f"验证结果: 不确定 — 有效样本不足 (k=5/10 最小有效={total_min})")
        print("数据限制: 日线缠论 BSP 在 5 年 1400 bars 上极稀疏")
        print("推荐: (1) 增加标的至 100+ (2) 使用 60min/30min 级别 (3) 降低信号阈值")
    elif any_significant:
        k_sig = [k for k in FORWARD_WINDOWS
                 if tests.get(str(k), {}).get("p_approx") is not None
                 and tests[str(k)]["p_approx"] < 0.05]
        high_better = all(
            high_stats["per_window"].get(k, {}).get("mean_fwd", 0)
            > low_stats["per_window"].get(k, {}).get("mean_fwd", 0)
            for k in k_sig
        )
        if high_better:
            print(f"验证结果: 肯定性 — 高 T(S) 信号质量显著优于低 T(S) (k={k_sig})")
        else:
            print(f"验证结果: 否定性 — 差异显著但方向相反 (k={k_sig})")
    else:
        print("验证结果: 否定性 — 无显著差异，H0 未被否决")
        print("解读: T(S) 在日线粒度下不具备信号质量区分力")

    print("=" * 78)


if __name__ == "__main__":
    main()
