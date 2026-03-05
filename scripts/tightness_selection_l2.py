"""tightness-selection-l2: T(S) 选股效果 L2 验证。

350号下游推论4：高 T(S) 标的是否确实产生更高质量的买卖点？

方法：前向收益窗口法（Forward Return Window）。
- 对每个标的跑引擎，收集所有 confirmed 买卖点
- 记录触发时刻的 bar 索引和价格
- 测量触发后 k 根 bar 的前向收益
- 按 T(S) 分组比较 high-T vs low-T 的买卖点质量

认识论等级：L2（真实数据验证，单时段多标的）。

谱系引用：350号、350号下游推论4、267号。
"""

from __future__ import annotations

import json
import logging
import sys
from dataclasses import asdict, dataclass
from datetime import datetime
from pathlib import Path
from typing import Literal

# 确保项目 src 在 path 上
_project_root = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(_project_root / "src"))

from newchan.orchestrator.recursive import RecursiveOrchestrator
from newchan.types import Bar

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(name)s %(message)s")
logger = logging.getLogger("tightness_selection_l2")

# ═══════════════════════════════════════════════════════════════
# 数据配置（复用 scanner_l2 缓存）
# ═══════════════════════════════════════════════════════════════

SYMBOLS: dict[str, dict] = {
    # Au
    "600547": {"name": "山东黄金", "sector": "au"},
    "002155": {"name": "湖南黄金", "sector": "au"},
    "600489": {"name": "中金黄金", "sector": "au"},
    # Oil
    "600028": {"name": "中国石化", "sector": "oil"},
    "601857": {"name": "中国石油", "sector": "oil"},
    "600688": {"name": "上海石化", "sector": "oil"},
    # Bond
    "601398": {"name": "工商银行", "sector": "bond"},
    "601288": {"name": "农业银行", "sector": "bond"},
    # RE
    "600048": {"name": "保利发展", "sector": "re"},
    "000002": {"name": "万科A", "sector": "re"},
    "001979": {"name": "招商蛇口", "sector": "re"},
    # Equity
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
TRAIN_RATIO = 0.8


# ═══════════════════════════════════════════════════════════════
# 数据加载（复用 scanner_l2 缓存）
# ═══════════════════════════════════════════════════════════════


def _load_bars(symbol: str) -> list[Bar]:
    """从 scanner_l2 缓存加载 bars。"""
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
# 买卖点采集 + 前向收益测量
# ═══════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class BSPRecord:
    """单个买卖点的记录 + 前向收益。"""

    symbol: str
    kind: str           # type1 / type2 / type3
    side: str           # buy / sell
    bar_idx: int        # 触发 bar 索引
    price: float        # 触发价格
    tightness: float    # 触发时刻的结构紧度
    fwd_returns: dict[int, float | None]   # {k: return}，None=数据不足
    mfe: dict[int, float | None]           # max favorable excursion
    mae: dict[int, float | None]           # max adverse excursion


def _compute_structural_tightness(snap) -> float:
    """从快照中计算结构紧度。复用 scanner_l2 的扩展版逻辑。"""
    tightness = 0.0

    has_bsp = any(bp.confirmed for bp in snap.bsp_snapshot.buysellpoints)
    if has_bsp:
        tightness += 1.0

    if len(snap.move_snapshot.moves) > 0:
        tightness += 0.5

    if len(snap.zs_snapshot.zhongshus) > 0:
        tightness += 0.3

    for rs in snap.recursive_snapshots:
        layer_score = 0.0
        if len(rs.moves) > 0:
            layer_score = 1.0
        elif len(rs.zhongshus) > 0:
            layer_score = 0.5
        if layer_score > 0:
            tightness += layer_score

    return tightness


def _forward_metrics(
    bars: list[Bar],
    trigger_bar_idx: int,
    trigger_price: float,
    side: str,
    windows: tuple[int, ...],
) -> tuple[dict[int, float | None], dict[int, float | None], dict[int, float | None]]:
    """计算前向收益、MFE、MAE。"""
    fwd = {}
    mfe = {}
    mae = {}
    sign = 1.0 if side == "buy" else -1.0

    for k in windows:
        end_idx = trigger_bar_idx + k
        if end_idx >= len(bars):
            fwd[k] = None
            mfe[k] = None
            mae[k] = None
            continue

        fwd_close = bars[end_idx].close
        fwd[k] = sign * (fwd_close - trigger_price) / trigger_price if trigger_price > 0 else None

        # MFE/MAE over [trigger+1, trigger+k]
        best = 0.0
        worst = 0.0
        for j in range(trigger_bar_idx + 1, end_idx + 1):
            if j >= len(bars):
                break
            if side == "buy":
                excursion_high = (bars[j].high - trigger_price) / trigger_price if trigger_price > 0 else 0.0
                excursion_low = (bars[j].low - trigger_price) / trigger_price if trigger_price > 0 else 0.0
                best = max(best, excursion_high)
                worst = min(worst, excursion_low)
            else:
                excursion_high = (trigger_price - bars[j].low) / trigger_price if trigger_price > 0 else 0.0
                excursion_low = (trigger_price - bars[j].high) / trigger_price if trigger_price > 0 else 0.0
                best = max(best, excursion_high)
                worst = min(worst, excursion_low)

        mfe[k] = best
        mae[k] = worst

    return fwd, mfe, mae


def collect_bsp_records(
    symbol: str,
    bars: list[Bar],
) -> tuple[float, list[BSPRecord]]:
    """对单个标的跑引擎，收集所有 confirmed 买卖点的前向收益。

    使用时间切分：前 80% 训练期计算最终 T(S)，后 20% 测试期收集买卖点。
    但买卖点触发需要引擎处理全部历史，所以引擎跑全量，
    T(S) 在训练期末固定，仅收集测试期内触发的买卖点。

    Returns
    -------
    (max_tightness_at_split, bsp_records)
    """
    split_idx = int(len(bars) * TRAIN_RATIO)
    orch = RecursiveOrchestrator(stream_id=symbol, max_levels=4)

    max_tightness = 0.0
    records: list[BSPRecord] = []

    # 已见过的 BSP 身份键，避免重复记录
    seen_bsp_keys: set[tuple] = set()

    for i, bar in enumerate(bars):
        snap = orch.process_bar(bar)

        # 更新训练期 tightness
        if i < split_idx:
            t = _compute_structural_tightness(snap)
            if t > max_tightness:
                max_tightness = t
            continue

        # 测试期：收集新出现的 confirmed 买卖点
        for bp in snap.bsp_snapshot.buysellpoints:
            if not bp.confirmed:
                continue
            bsp_key = (bp.seg_idx, bp.kind, bp.side, bp.level_id)
            if bsp_key in seen_bsp_keys:
                continue
            seen_bsp_keys.add(bsp_key)

            trigger_price = bp.price
            trigger_bar_idx = i  # 当前 bar 是首次看到此 BSP

            if trigger_price <= 0:
                continue

            fwd, mfe, mae = _forward_metrics(
                bars, trigger_bar_idx, trigger_price, bp.side, FORWARD_WINDOWS,
            )

            records.append(BSPRecord(
                symbol=symbol,
                kind=bp.kind,
                side=bp.side,
                bar_idx=trigger_bar_idx,
                price=trigger_price,
                tightness=max_tightness,
                fwd_returns=fwd,
                mfe=mfe,
                mae=mae,
            ))

    return max_tightness, records


# ═══════════════════════════════════════════════════════════════
# 分组比较 + 统计检验
# ═══════════════════════════════════════════════════════════════


@dataclass
class GroupStats:
    """一组标的的汇总统计。"""

    group_name: str
    n_symbols: int
    n_bsp_total: int
    n_buy: int
    n_sell: int
    avg_tightness: float
    per_window: dict[int, dict]  # k -> {mean_fwd, win_rate, mean_mfe, mean_mae, n_valid}


def _compute_group_stats(
    group_name: str,
    records: list[BSPRecord],
    symbols_in_group: list[str],
    windows: tuple[int, ...],
) -> GroupStats:
    """计算一组标的的汇总统计。"""
    buy_records = [r for r in records if r.side == "buy"]
    sell_records = [r for r in records if r.side == "sell"]
    all_tightness = [r.tightness for r in records]

    per_window: dict[int, dict] = {}
    for k in windows:
        valid_fwd = [r.fwd_returns[k] for r in records if r.fwd_returns.get(k) is not None]
        valid_mfe = [r.mfe[k] for r in records if r.mfe.get(k) is not None]
        valid_mae = [r.mae[k] for r in records if r.mae.get(k) is not None]

        n_valid = len(valid_fwd)
        mean_fwd = sum(valid_fwd) / n_valid if n_valid > 0 else 0.0
        win_rate = sum(1 for v in valid_fwd if v > 0) / n_valid if n_valid > 0 else 0.0
        mean_mfe = sum(valid_mfe) / len(valid_mfe) if valid_mfe else 0.0
        mean_mae = sum(valid_mae) / len(valid_mae) if valid_mae else 0.0

        per_window[k] = {
            "mean_fwd_return": round(mean_fwd, 6),
            "win_rate": round(win_rate, 4),
            "mean_mfe": round(mean_mfe, 6),
            "mean_mae": round(mean_mae, 6),
            "n_valid": n_valid,
        }

    return GroupStats(
        group_name=group_name,
        n_symbols=len(symbols_in_group),
        n_bsp_total=len(records),
        n_buy=len(buy_records),
        n_sell=len(sell_records),
        avg_tightness=round(sum(all_tightness) / len(all_tightness), 4) if all_tightness else 0.0,
        per_window=per_window,
    )


def _wilcoxon_rank_sum(xs: list[float], ys: list[float]) -> dict:
    """Wilcoxon rank-sum test（手动实现，避免 scipy 依赖）。

    返回 U 统计量、z 值、近似 p 值（正态近似，n1+n2>=20 时有效）。
    n1+n2<20 时仅返回 U 统计量，不做正态近似。
    """
    import math

    n1, n2 = len(xs), len(ys)
    if n1 == 0 or n2 == 0:
        return {"U": None, "z": None, "p_approx": None, "n1": n1, "n2": n2}

    # 合并并排序
    combined = [(v, "x") for v in xs] + [(v, "y") for v in ys]
    combined.sort(key=lambda t: t[0])

    # 分配排名（处理 ties 用平均排名）
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

    # R1 = sum of ranks for xs
    r1 = sum(ranks[k] for k in range(len(combined)) if combined[k][1] == "x")
    u1 = r1 - n1 * (n1 + 1) / 2.0
    u2 = n1 * n2 - u1
    u = min(u1, u2)

    result: dict = {"U": round(u, 2), "n1": n1, "n2": n2}

    if n1 + n2 >= 20:
        mu = n1 * n2 / 2.0
        sigma = math.sqrt(n1 * n2 * (n1 + n2 + 1) / 12.0)
        z = (u - mu) / sigma if sigma > 0 else 0.0
        # 双侧 p 值（正态近似）
        p = 2 * (1 - _normal_cdf(abs(z)))
        result["z"] = round(z, 4)
        result["p_approx"] = round(p, 6)
    else:
        result["z"] = None
        result["p_approx"] = None
        result["note"] = "sample too small for normal approximation"

    return result


def _normal_cdf(x: float) -> float:
    """标准正态 CDF 近似（Abramowitz & Stegun 26.2.17）。"""
    import math
    if x < 0:
        return 1 - _normal_cdf(-x)
    t = 1.0 / (1.0 + 0.2316419 * x)
    poly = t * (0.319381530 + t * (-0.356563782 + t * (1.781477937 + t * (-1.821255978 + t * 1.330274429))))
    return 1.0 - poly * math.exp(-x * x / 2.0) / math.sqrt(2.0 * math.pi)


# ═══════════════════════════════════════════════════════════════
# 主程序
# ═══════════════════════════════════════════════════════════════


def main() -> None:
    logger.info("=== T(S) Selection L2 Validation ===")
    logger.info("标的数: %d, 前向窗口: %s, 训练比例: %.0f%%",
                len(SYMBOLS), FORWARD_WINDOWS, TRAIN_RATIO * 100)

    # 1. 加载数据 + 跑引擎 + 收集买卖点
    all_records: list[BSPRecord] = []
    symbol_tightness: dict[str, float] = {}

    for symbol, info in SYMBOLS.items():
        bars = _load_bars(symbol)
        if len(bars) < 200:
            logger.warning("数据不足: %s (%s) — %d bars, 跳过", symbol, info["name"], len(bars))
            continue

        logger.info("处理: %s (%s) — %d bars (训练 %d / 测试 %d)",
                    symbol, info["name"], len(bars),
                    int(len(bars) * TRAIN_RATIO),
                    len(bars) - int(len(bars) * TRAIN_RATIO))

        max_t, records = collect_bsp_records(symbol, bars)
        symbol_tightness[symbol] = max_t
        all_records.extend(records)

        logger.info("  → T(S)=%.2f, %d 买卖点 (buy=%d, sell=%d)",
                    max_t,
                    len(records),
                    sum(1 for r in records if r.side == "buy"),
                    sum(1 for r in records if r.side == "sell"))

    logger.info("总买卖点数: %d (buy=%d, sell=%d)",
                len(all_records),
                sum(1 for r in all_records if r.side == "buy"),
                sum(1 for r in all_records if r.side == "sell"))

    if not all_records:
        logger.error("无买卖点产出，无法进行 L2 验证")
        sys.exit(1)

    # 2. 按 T(S) 排序分组
    sorted_symbols = sorted(symbol_tightness.items(), key=lambda x: -x[1])
    n = len(sorted_symbols)
    third = max(1, n // 3)

    high_t_symbols = [s for s, _ in sorted_symbols[:third]]
    low_t_symbols = [s for s, _ in sorted_symbols[-third:]]
    mid_t_symbols = [s for s, _ in sorted_symbols[third:-third]]

    logger.info("分组: high-T %d 标的 (T>=%.2f), low-T %d 标的 (T<=%.2f)",
                len(high_t_symbols),
                symbol_tightness[high_t_symbols[-1]] if high_t_symbols else 0,
                len(low_t_symbols),
                symbol_tightness[low_t_symbols[0]] if low_t_symbols else 0)

    high_t_records = [r for r in all_records if r.symbol in set(high_t_symbols)]
    low_t_records = [r for r in all_records if r.symbol in set(low_t_symbols)]

    # 3. 计算组统计
    high_stats = _compute_group_stats("high-T", high_t_records, high_t_symbols, FORWARD_WINDOWS)
    low_stats = _compute_group_stats("low-T", low_t_records, low_t_symbols, FORWARD_WINDOWS)

    # 4. 统计检验
    tests: dict[int, dict] = {}
    for k in FORWARD_WINDOWS:
        high_fwd = [r.fwd_returns[k] for r in high_t_records if r.fwd_returns.get(k) is not None]
        low_fwd = [r.fwd_returns[k] for r in low_t_records if r.fwd_returns.get(k) is not None]
        tests[k] = _wilcoxon_rank_sum(high_fwd, low_fwd)

    # 5. 组装报告
    report = {
        "meta": {
            "hypothesis": "H0: high-T BSP quality == low-T BSP quality",
            "method": "Forward Return Window + Wilcoxon rank-sum",
            "train_ratio": TRAIN_RATIO,
            "forward_windows": list(FORWARD_WINDOWS),
            "total_symbols": len(symbol_tightness),
            "total_bsp": len(all_records),
            "epistemology_level": "L2",
        },
        "symbol_tightness": {
            s: {"name": SYMBOLS[s]["name"], "tightness": round(t, 4)}
            for s, t in sorted_symbols
        },
        "groups": {
            "high_t": {
                "symbols": high_t_symbols,
                "n_symbols": high_stats.n_symbols,
                "n_bsp": high_stats.n_bsp_total,
                "n_buy": high_stats.n_buy,
                "n_sell": high_stats.n_sell,
                "avg_tightness": high_stats.avg_tightness,
                "per_window": high_stats.per_window,
            },
            "low_t": {
                "symbols": low_t_symbols,
                "n_symbols": low_stats.n_symbols,
                "n_bsp": low_stats.n_bsp_total,
                "n_buy": low_stats.n_buy,
                "n_sell": low_stats.n_sell,
                "avg_tightness": low_stats.avg_tightness,
                "per_window": low_stats.per_window,
            },
        },
        "statistical_tests": {str(k): v for k, v in tests.items()},
        "bsp_details": [
            {
                "symbol": r.symbol,
                "name": SYMBOLS.get(r.symbol, {}).get("name", "?"),
                "kind": r.kind,
                "side": r.side,
                "bar_idx": r.bar_idx,
                "price": round(r.price, 4),
                "tightness": round(r.tightness, 4),
                "fwd_returns": {str(k): round(v, 6) if v is not None else None for k, v in r.fwd_returns.items()},
                "mfe": {str(k): round(v, 6) if v is not None else None for k, v in r.mfe.items()},
                "mae": {str(k): round(v, 6) if v is not None else None for k, v in r.mae.items()},
            }
            for r in all_records
        ],
    }

    # 6. 输出
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    report_path = OUTPUT_DIR / "tightness_selection_l2_report.json"
    report_path.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    logger.info("报告已保存: %s", report_path)

    # 7. 打印摘要
    print("\n" + "=" * 70)
    print("T(S) Selection L2 Validation 结果摘要")
    print("=" * 70)

    print(f"\n标的数: {len(symbol_tightness)}, 总买卖点: {len(all_records)}")
    print(f"High-T 组: {len(high_t_symbols)} 标的, {high_stats.n_bsp_total} BSP (avg T={high_stats.avg_tightness:.2f})")
    print(f"Low-T  组: {len(low_t_symbols)} 标的, {low_stats.n_bsp_total} BSP (avg T={low_stats.avg_tightness:.2f})")

    print("\n标的 T(S) 排序:")
    for s, t in sorted_symbols:
        group = "HIGH" if s in set(high_t_symbols) else ("LOW" if s in set(low_t_symbols) else "MID")
        print(f"  {s} ({SYMBOLS[s]['name']:6s}) T={t:.2f} [{group}]")

    print("\n前向收益对比 (buy + sell):")
    print(f"{'窗口':>4s} | {'High-T mean':>12s} {'win%':>6s} {'n':>4s} | {'Low-T mean':>12s} {'win%':>6s} {'n':>4s} | {'p-value':>8s}")
    print("-" * 75)
    for k in FORWARD_WINDOWS:
        hw = high_stats.per_window.get(k, {})
        lw = low_stats.per_window.get(k, {})
        t = tests.get(k, {})
        h_mean = hw.get("mean_fwd_return", 0)
        h_wr = hw.get("win_rate", 0)
        h_n = hw.get("n_valid", 0)
        l_mean = lw.get("mean_fwd_return", 0)
        l_wr = lw.get("win_rate", 0)
        l_n = lw.get("n_valid", 0)
        p = t.get("p_approx")
        p_str = f"{p:.4f}" if p is not None else "N/A"
        sig = " *" if p is not None and p < 0.05 else ""
        print(f"  k={k:2d} | {h_mean:>11.4%} {h_wr:>5.1%} {h_n:>4d} | {l_mean:>11.4%} {l_wr:>5.1%} {l_n:>4d} | {p_str:>8s}{sig}")

    print("\nMFE/MAE 对比 (买点):")
    high_buy = [r for r in high_t_records if r.side == "buy"]
    low_buy = [r for r in low_t_records if r.side == "buy"]
    print(f"{'窗口':>4s} | {'High-T MFE':>10s} {'MAE':>10s} {'n':>4s} | {'Low-T MFE':>10s} {'MAE':>10s} {'n':>4s}")
    print("-" * 65)
    for k in FORWARD_WINDOWS:
        h_mfe_vals = [r.mfe[k] for r in high_buy if r.mfe.get(k) is not None]
        h_mae_vals = [r.mae[k] for r in high_buy if r.mae.get(k) is not None]
        l_mfe_vals = [r.mfe[k] for r in low_buy if r.mfe.get(k) is not None]
        l_mae_vals = [r.mae[k] for r in low_buy if r.mae.get(k) is not None]

        h_mfe = sum(h_mfe_vals) / len(h_mfe_vals) if h_mfe_vals else 0
        h_mae = sum(h_mae_vals) / len(h_mae_vals) if h_mae_vals else 0
        l_mfe = sum(l_mfe_vals) / len(l_mfe_vals) if l_mfe_vals else 0
        l_mae = sum(l_mae_vals) / len(l_mae_vals) if l_mae_vals else 0

        print(f"  k={k:2d} | {h_mfe:>9.4%} {h_mae:>9.4%} {len(h_mfe_vals):>4d} | {l_mfe:>9.4%} {l_mae:>9.4%} {len(l_mfe_vals):>4d}")

    # 8. 结论
    any_significant = any(
        tests[k].get("p_approx") is not None and tests[k]["p_approx"] < 0.05
        for k in FORWARD_WINDOWS
    )
    sufficient_sample = all(
        (high_stats.per_window.get(k, {}).get("n_valid", 0) >= 10
         and low_stats.per_window.get(k, {}).get("n_valid", 0) >= 10)
        for k in FORWARD_WINDOWS[:2]  # 至少 k=5,10 有足够样本
    )

    print("\n" + "=" * 70)
    print("认识论等级: L2（真实数据验证，单时段多标的）")
    print("验证命题: 高 T(S) 标的的买卖点后续走势质量 > 低 T(S) 标的")

    if not sufficient_sample:
        print("验证结果: 不确定 — 样本量不足，无法得出可靠结论")
        print("建议: 增加标的数量或使用更细粒度时间周期（如60分钟线）")
    elif any_significant:
        # 确认方向：high-T 是否真的更好
        k_sig = [k for k in FORWARD_WINDOWS
                 if tests[k].get("p_approx") is not None and tests[k]["p_approx"] < 0.05]
        high_better = all(
            high_stats.per_window[k]["mean_fwd_return"] > low_stats.per_window[k]["mean_fwd_return"]
            for k in k_sig
        )
        if high_better:
            print("验证结果: 肯定性 — 高 T(S) 标的买卖点质量显著优于低 T(S)")
            print(f"显著窗口: k={k_sig}")
        else:
            print("验证结果: 否定性 — 存在显著差异但方向相反（低 T 优于高 T）")
            print(f"显著窗口: k={k_sig}")
            print("解读: T(S) 可能不是有效选股指标，或扩展版 tightness proxy 不合适")
    else:
        print("验证结果: 否定性 — 无显著差异，H0 未被否决")
        print("解读: T(S) 在日线粒度下可能不具备选股预测力")
        print("可能原因: (1) 样本量不足 (2) 日线买卖点太稀疏 (3) tightness proxy 不精确")

    print("=" * 70)


if __name__ == "__main__":
    main()
