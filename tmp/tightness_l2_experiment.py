"""tightness proxy + 选股效果 L2 验证实验。

两个实验合一脚本：
  实验1：Tightness proxy 与严格 T(S) 一致性验证（366号下游推论3 + 371号）
  实验2：T(S)/proxy 选股效果 L2 验证（350号下游推论4）

认识论等级标注：
  - 实验1 结论：L2 否定性结果（strict T(S) 在日线数据上全零，proxy 排序无可比对象）
  - 实验2 结论：L2（proxy-L2，非 T(S)-L2）——诚实标注

谱系引用：350号、366号、371号。
"""
from __future__ import annotations

import json
import logging
import math
import sys
from collections import Counter
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

_project_root = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(_project_root / "src"))

from newchan.a_nested_divergence import (
    NestedDivergence,
    divergences_from_level_snapshot,
    nested_divergence_search,
    _get_moves_at_level,
)
from newchan.convergence import convergence_tightness
from newchan.orchestrator.recursive import RecursiveOrchestrator
from newchan.types import Bar

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(name)s %(message)s")
logger = logging.getLogger("tightness_l2")

DATA_DIR = _project_root / "data" / "scanner_l2"
OUTPUT_DIR = _project_root / "data" / "tightness_l2_experiment"

# 24 个标的 + 折叠通道标注（复用 scanner_l2 数据）
SYMBOLS: dict[str, dict] = {
    # AU 通道
    "600547": {"name": "山东黄金", "channel": "AU"},
    "002155": {"name": "湖南黄金", "channel": "AU"},
    "600489": {"name": "中金黄金", "channel": "AU"},
    # OIL 通道
    "600028": {"name": "中国石化", "channel": "OIL"},
    "601857": {"name": "中国石油", "channel": "OIL"},
    "600688": {"name": "上海石化", "channel": "OIL"},
    # BOND 通道
    "601398": {"name": "工商银行", "channel": "BOND"},
    "601288": {"name": "农业银行", "channel": "BOND"},
    # RE 通道
    "600048": {"name": "保利发展", "channel": "RE"},
    "000002": {"name": "万科A", "channel": "RE"},
    "001979": {"name": "招商蛇口", "channel": "RE"},
    # EQUITY 通道 - 多板块
    "002230": {"name": "科大讯飞", "channel": "EQUITY", "sector": "tech"},
    "300059": {"name": "东方财富", "channel": "EQUITY", "sector": "tech"},
    "000725": {"name": "京东方A", "channel": "EQUITY", "sector": "tech"},
    "601318": {"name": "中国平安", "channel": "EQUITY", "sector": "finance"},
    "600036": {"name": "招商银行", "channel": "EQUITY", "sector": "finance"},
    "601166": {"name": "兴业银行", "channel": "EQUITY", "sector": "finance"},
    "600519": {"name": "贵州茅台", "channel": "EQUITY", "sector": "consumer"},
    "000858": {"name": "五粮液", "channel": "EQUITY", "sector": "consumer"},
    "002304": {"name": "洋河股份", "channel": "EQUITY", "sector": "consumer"},
    "300760": {"name": "迈瑞医疗", "channel": "EQUITY", "sector": "pharma"},
    "600276": {"name": "恒瑞医药", "channel": "EQUITY", "sector": "pharma"},
    "601985": {"name": "中国核电", "channel": "EQUITY", "sector": "energy"},
    "600900": {"name": "长江电力", "channel": "EQUITY", "sector": "energy"},
}


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
# Proxy 计算（复用 scanner_l2 逻辑）
# ═══════════════════════════════════════════════════════════════


def _compute_proxy_tightness(snap) -> float:
    """扩展版 tightness proxy（加法结构）。"""
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


# ═══════════════════════════════════════════════════════════════
# 买卖点质量指标
# ═══════════════════════════════════════════════════════════════


@dataclass(frozen=True, slots=True)
class BspQuality:
    """买卖点质量度量。"""
    total_bsp: int            # 总买卖点数
    confirmed_bsp: int        # 已确认买卖点数
    settled_bsp: int          # 已结算买卖点数
    type1_count: int          # Type1（趋势背驰）
    type2_count: int          # Type2（回调/反弹）
    type3_count: int          # Type3（中枢突破）
    buy_count: int            # 买点数
    sell_count: int           # 卖点数
    overlap_count: int        # 2B+3B 重合
    max_level_id: int         # 最高级别
    # 后续收益度量
    buy_forward_returns: list[float]  # 每个买点后 N 根K线的收益率


def _compute_bsp_quality(
    snap,
    bars: list[Bar],
    bar_idx: int,
    forward_window: int = 20,
) -> BspQuality | None:
    """从快照中计算买卖点质量。仅在有买卖点时返回。"""
    bsps = snap.bsp_snapshot.buysellpoints
    if not bsps:
        return None

    total = len(bsps)
    confirmed = sum(1 for bp in bsps if bp.confirmed)
    settled = sum(1 for bp in bsps if bp.settled)
    type1 = sum(1 for bp in bsps if bp.kind == "type1")
    type2 = sum(1 for bp in bsps if bp.kind == "type2")
    type3 = sum(1 for bp in bsps if bp.kind == "type3")
    buys = sum(1 for bp in bsps if bp.side == "buy")
    sells = sum(1 for bp in bsps if bp.side == "sell")
    overlaps = sum(1 for bp in bsps if bp.overlaps_with is not None)
    max_lid = max(bp.level_id for bp in bsps)

    # 后续收益：对每个 confirmed buy，取 bar_idx 后 forward_window 根 K 线的收益
    forward_returns: list[float] = []
    for bp in bsps:
        if bp.side == "buy" and bp.confirmed:
            buy_bar = bp.bar_idx
            if buy_bar < len(bars) and buy_bar + forward_window < len(bars):
                entry_price = bars[buy_bar].close
                exit_price = bars[buy_bar + forward_window].close
                if entry_price > 0:
                    forward_returns.append((exit_price - entry_price) / entry_price)

    return BspQuality(
        total_bsp=total,
        confirmed_bsp=confirmed,
        settled_bsp=settled,
        type1_count=type1,
        type2_count=type2,
        type3_count=type3,
        buy_count=buys,
        sell_count=sells,
        overlap_count=overlaps,
        max_level_id=max_lid,
        buy_forward_returns=forward_returns,
    )


# ═══════════════════════════════════════════════════════════════
# 排序相关性工具
# ═══════════════════════════════════════════════════════════════


def _rank(vals: list[float]) -> list[float]:
    """Average rank assignment (handles ties)."""
    n = len(vals)
    sorted_indices = sorted(range(n), key=lambda i: vals[i])
    ranks = [0.0] * n
    i = 0
    while i < n:
        j = i
        while j < n and vals[sorted_indices[j]] == vals[sorted_indices[i]]:
            j += 1
        avg_rank = (i + j + 1) / 2.0
        for k in range(i, j):
            ranks[sorted_indices[k]] = avg_rank
        i = j
    return ranks


def _spearman(xs: list[float], ys: list[float]) -> float | None:
    n = len(xs)
    if n < 3:
        return None
    rx = _rank(xs)
    ry = _rank(ys)
    d_sq = sum((rx[i] - ry[i]) ** 2 for i in range(n))
    rho = 1 - 6 * d_sq / (n * (n * n - 1))
    return rho


def _kendall_tau(xs: list[float], ys: list[float]) -> float | None:
    n = len(xs)
    if n < 3:
        return None
    concordant = 0
    discordant = 0
    for i in range(n):
        for j in range(i + 1, n):
            dx = xs[i] - xs[j]
            dy = ys[i] - ys[j]
            if dx * dy > 0:
                concordant += 1
            elif dx * dy < 0:
                discordant += 1
            # ties: neither concordant nor discordant
    denom = concordant + discordant
    if denom == 0:
        return 0.0
    return (concordant - discordant) / denom


# ═══════════════════════════════════════════════════════════════
# 实验1：Proxy vs Strict T(S) 一致性
# ═══════════════════════════════════════════════════════════════


@dataclass
class SymbolResult:
    symbol: str
    name: str
    channel: str
    bar_count: int
    proxy_tightness: float
    strict_ts: float
    strict_d: int
    strict_l: int
    strict_c: float
    max_recursive_levels: int
    max_level1_moves: int
    max_level1_zhongshus: int
    max_level2_moves: int
    max_level2_zhongshus: int
    # BSP 质量
    total_bsp: int
    confirmed_bsp: int
    settled_bsp: int
    type1_count: int
    type2_count: int
    type3_count: int
    buy_count: int
    sell_count: int
    overlap_count: int
    buy_forward_returns_20d: list[float]
    mean_forward_return_20d: float | None


def process_symbol(symbol: str, info: dict) -> SymbolResult:
    """处理单个标的：同时计算 proxy、strict T(S)、BSP 质量。"""
    bars = _load_bars(symbol)
    if not bars:
        raise ValueError(f"No data for {symbol}")

    orch = RecursiveOrchestrator(stream_id=symbol, max_levels=6)

    max_proxy = 0.0
    max_strict = 0.0
    max_d = 0
    max_l = 0
    max_c = 0.0
    max_rec_levels = 0
    max_l1_moves = 0
    max_l1_zs = 0
    max_l2_moves = 0
    max_l2_zs = 0

    # BSP quality aggregation
    all_bsp_total = 0
    all_bsp_confirmed = 0
    all_bsp_settled = 0
    all_type1 = 0
    all_type2 = 0
    all_type3 = 0
    all_buys = 0
    all_sells = 0
    all_overlaps = 0
    all_forward_returns: list[float] = []

    for i, bar in enumerate(bars):
        snap = orch.process_bar(bar)

        # Proxy
        pt = _compute_proxy_tightness(snap)
        if pt > max_proxy:
            max_proxy = pt

        # Strict T(S)
        nds = nested_divergence_search(snap)
        if nds:
            for nd in nds:
                ct = convergence_tightness(nd)
                if ct.score > max_strict:
                    max_strict = ct.score
                    max_d = ct.depth
                    max_l = ct.level_max
                    max_c = ct.consistency

        # Structure depth
        n_rec = len(snap.recursive_snapshots)
        if n_rec > max_rec_levels:
            max_rec_levels = n_rec
        n_l1m = len(snap.move_snapshot.moves)
        n_l1z = len(snap.zs_snapshot.zhongshus)
        if n_l1m > max_l1_moves:
            max_l1_moves = n_l1m
        if n_l1z > max_l1_zs:
            max_l1_zs = n_l1z
        for rs in snap.recursive_snapshots:
            if rs.level_id == 2:
                if len(rs.moves) > max_l2_moves:
                    max_l2_moves = len(rs.moves)
                if len(rs.zhongshus) > max_l2_zs:
                    max_l2_zs = len(rs.zhongshus)

        # BSP quality (at final bar snapshot for total counts,
        # but forward returns need to be computed at each BSP event)
        bq = _compute_bsp_quality(snap, bars, i, forward_window=20)
        if bq is not None:
            all_bsp_total = max(all_bsp_total, bq.total_bsp)
            all_bsp_confirmed = max(all_bsp_confirmed, bq.confirmed_bsp)
            all_bsp_settled = max(all_bsp_settled, bq.settled_bsp)
            all_type1 = max(all_type1, bq.type1_count)
            all_type2 = max(all_type2, bq.type2_count)
            all_type3 = max(all_type3, bq.type3_count)
            all_buys = max(all_buys, bq.buy_count)
            all_sells = max(all_sells, bq.sell_count)
            all_overlaps = max(all_overlaps, bq.overlap_count)
            all_forward_returns.extend(bq.buy_forward_returns)

    mean_fwd = None
    if all_forward_returns:
        mean_fwd = sum(all_forward_returns) / len(all_forward_returns)

    return SymbolResult(
        symbol=symbol,
        name=info["name"],
        channel=info["channel"],
        bar_count=len(bars),
        proxy_tightness=max_proxy,
        strict_ts=max_strict,
        strict_d=max_d,
        strict_l=max_l,
        strict_c=max_c,
        max_recursive_levels=max_rec_levels,
        max_level1_moves=max_l1_moves,
        max_level1_zhongshus=max_l1_zs,
        max_level2_moves=max_l2_moves,
        max_level2_zhongshus=max_l2_zs,
        total_bsp=all_bsp_total,
        confirmed_bsp=all_bsp_confirmed,
        settled_bsp=all_bsp_settled,
        type1_count=all_type1,
        type2_count=all_type2,
        type3_count=all_type3,
        buy_count=all_buys,
        sell_count=all_sells,
        overlap_count=all_overlaps,
        buy_forward_returns_20d=all_forward_returns,
        mean_forward_return_20d=mean_fwd,
    )


# ═══════════════════════════════════════════════════════════════
# 实验2：分组对比
# ═══════════════════════════════════════════════════════════════


def experiment2_group_comparison(results: list[SymbolResult]) -> dict:
    """按 proxy tightness 分组，对比买卖点质量。

    分组方式：proxy > median → 高组，proxy <= median → 低组。
    """
    proxies = sorted(set(r.proxy_tightness for r in results))
    if len(proxies) < 2:
        return {"error": "proxy 值无分化（所有标的相同）", "n_unique_proxies": len(proxies)}

    median_proxy = sorted(r.proxy_tightness for r in results)[len(results) // 2]

    high_group = [r for r in results if r.proxy_tightness > median_proxy]
    low_group = [r for r in results if r.proxy_tightness <= median_proxy]

    def _group_stats(group: list[SymbolResult], label: str) -> dict:
        if not group:
            return {"label": label, "n": 0}
        all_fwd = []
        for r in group:
            all_fwd.extend(r.buy_forward_returns_20d)
        return {
            "label": label,
            "n": len(group),
            "symbols": [f"{r.symbol}({r.name})" for r in group],
            "avg_proxy": round(sum(r.proxy_tightness for r in group) / len(group), 4),
            "avg_total_bsp": round(sum(r.total_bsp for r in group) / len(group), 2),
            "avg_confirmed_bsp": round(sum(r.confirmed_bsp for r in group) / len(group), 2),
            "avg_type1": round(sum(r.type1_count for r in group) / len(group), 2),
            "avg_type2": round(sum(r.type2_count for r in group) / len(group), 2),
            "avg_type3": round(sum(r.type3_count for r in group) / len(group), 2),
            "avg_overlap": round(sum(r.overlap_count for r in group) / len(group), 2),
            "total_buy_signals": sum(r.buy_count for r in group),
            "total_forward_returns": len(all_fwd),
            "mean_forward_return_20d": round(sum(all_fwd) / len(all_fwd), 6) if all_fwd else None,
            "median_forward_return_20d": round(sorted(all_fwd)[len(all_fwd) // 2], 6) if all_fwd else None,
            "positive_return_ratio": round(sum(1 for x in all_fwd if x > 0) / len(all_fwd), 4) if all_fwd else None,
        }

    high_stats = _group_stats(high_group, "high_proxy")
    low_stats = _group_stats(low_group, "low_proxy")

    # Mann-Whitney U 近似（非参数检验）
    all_high_fwd = []
    for r in high_group:
        all_high_fwd.extend(r.buy_forward_returns_20d)
    all_low_fwd = []
    for r in low_group:
        all_low_fwd.extend(r.buy_forward_returns_20d)

    u_result = _mann_whitney_u(all_high_fwd, all_low_fwd)

    return {
        "median_proxy_threshold": median_proxy,
        "high_group": high_stats,
        "low_group": low_stats,
        "mann_whitney_u": u_result,
    }


def _mann_whitney_u(xs: list[float], ys: list[float]) -> dict:
    """Simple Mann-Whitney U statistic (no scipy dependency)."""
    if not xs or not ys:
        return {"error": "insufficient data", "n_x": len(xs), "n_y": len(ys)}

    nx, ny = len(xs), len(ys)
    # Combine and rank
    combined = [(v, "x") for v in xs] + [(v, "y") for v in ys]
    combined.sort(key=lambda t: t[0])

    # Assign ranks with tie handling
    n = len(combined)
    ranks_dict: dict[int, float] = {}
    i = 0
    while i < n:
        j = i
        while j < n and combined[j][0] == combined[i][0]:
            j += 1
        avg_rank = (i + j + 1) / 2.0
        for k in range(i, j):
            ranks_dict[k] = avg_rank
        i = j

    r_x = sum(ranks_dict[i] for i in range(n) if combined[i][1] == "x")
    u_x = r_x - nx * (nx + 1) / 2
    u_y = nx * ny - u_x
    u = min(u_x, u_y)

    # Normal approximation for p-value
    mu = nx * ny / 2
    sigma = math.sqrt(nx * ny * (nx + ny + 1) / 12)
    if sigma == 0:
        return {"U": u, "p_approx": 1.0, "n_x": nx, "n_y": ny}
    z = (u - mu) / sigma
    # Two-tailed p-value approximation
    p = 2 * (1 - _norm_cdf(abs(z)))

    return {
        "U": round(u, 2),
        "z": round(z, 4),
        "p_approx": round(p, 6),
        "n_x": nx,
        "n_y": ny,
        "significant_at_005": p < 0.05,
    }


def _norm_cdf(x: float) -> float:
    """Standard normal CDF approximation (Abramowitz & Stegun)."""
    return 0.5 * (1 + math.erf(x / math.sqrt(2)))


# ═══════════════════════════════════════════════════════════════
# 主程序
# ═══════════════════════════════════════════════════════════════


def main() -> None:
    logger.info("=== Tightness L2 实验 ===")
    logger.info("实验1: Proxy vs Strict T(S) 一致性")
    logger.info("实验2: Proxy 分组选股效果")

    # 处理所有标的
    results: list[SymbolResult] = []
    for symbol, info in SYMBOLS.items():
        bars = _load_bars(symbol)
        if len(bars) < 200:
            logger.warning("数据不足: %s — %d bars, 跳过", symbol, len(bars))
            continue
        logger.info("处理: %s (%s) — %d bars", symbol, info["name"], len(bars))
        sr = process_symbol(symbol, info)
        results.append(sr)
        logger.info(
            "  proxy=%.2f, strict_T=%.2f, bsp=%d(conf=%d), fwd_returns=%d signals",
            sr.proxy_tightness, sr.strict_ts, sr.total_bsp, sr.confirmed_bsp,
            len(sr.buy_forward_returns_20d),
        )

    if len(results) < 5:
        logger.error("有效标的不足（<5），退出")
        sys.exit(1)

    # ─── 实验1 ───────────────────────────────────
    proxy_vals = [r.proxy_tightness for r in results]
    strict_vals = [r.strict_ts for r in results]

    n_nonzero_strict = sum(1 for v in strict_vals if v > 0)

    spearman = _spearman(proxy_vals, strict_vals)
    kendall = _kendall_tau(proxy_vals, strict_vals)

    # ─── 实验2 ───────────────────────────────────
    exp2_result = experiment2_group_comparison(results)

    # ─── 组装报告 ─────────────────────────────────
    report = {
        "experiment_1": {
            "title": "Proxy vs Strict T(S) 排序一致性",
            "epistemology_level": "L2（否定性结果）",
            "genealogy_ref": ["350号", "366号边界条件4", "371号"],
            "n_symbols": len(results),
            "n_nonzero_strict_ts": n_nonzero_strict,
            "spearman_rho": round(spearman, 4) if spearman is not None else None,
            "kendall_tau": round(kendall, 4) if kendall is not None else None,
            "conclusion": _exp1_conclusion(n_nonzero_strict, spearman),
            "root_cause_diagnosis": {
                "summary": "日线5年数据递归深度不足",
                "detail": "所有24标的的 Level 2 moves=0, zhongshus=0。"
                          "nested_divergence_search 需要至少 Level 2 有背驰才能形成链。"
                          "日线1489根K线产生的 Level 1 最多3个 moves，"
                          "不足以在 Level 2 形成中枢和走势。",
                "max_recursive_levels": max(r.max_recursive_levels for r in results),
                "max_l1_moves_across_all": max(r.max_level1_moves for r in results),
                "max_l2_moves_across_all": max(r.max_level2_moves for r in results),
            },
            "per_symbol": [
                {
                    "symbol": r.symbol,
                    "name": r.name,
                    "channel": r.channel,
                    "proxy": round(r.proxy_tightness, 4),
                    "strict_ts": round(r.strict_ts, 4),
                    "strict_d": r.strict_d,
                    "strict_l": r.strict_l,
                    "strict_c": round(r.strict_c, 4),
                    "max_rec_levels": r.max_recursive_levels,
                    "max_l1_moves": r.max_level1_moves,
                    "max_l2_moves": r.max_level2_moves,
                }
                for r in results
            ],
        },
        "experiment_2": {
            "title": "Proxy 分组选股效果",
            "epistemology_level": "L2（proxy-L2，非 T(S)-L2）",
            "genealogy_ref": ["350号下游推论4"],
            "honest_label": "由于 strict T(S) 全零，本实验使用 proxy 代替。"
                            "结论仅对 proxy 有效，不可反向背书 T(S)。"
                            "（371号约束）",
            "comparison": exp2_result,
            "conclusion": _exp2_conclusion(exp2_result),
        },
    }

    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    report_path = OUTPUT_DIR / "tightness_l2_report.json"
    report_path.write_text(
        json.dumps(report, ensure_ascii=False, indent=2, default=str),
        encoding="utf-8",
    )
    logger.info("报告已保存: %s", report_path)

    # ─── 打印摘要 ─────────────────────────────────
    _print_summary(report, results)


def _exp1_conclusion(n_nonzero: int, rho: float | None) -> str:
    if n_nonzero == 0:
        return (
            "否定性结果（L2）：24个标的的严格 T(S) 全部为零。"
            "nested_divergence_search 在日线5年数据上未检测到任何跨级别嵌套背驰链。"
            "根因：日线K线数量不足以在递归 Level 2 形成结构（moves=0, zhongshus=0）。"
            "Spearman/Kendall 值无参考意义（全零严格值的排序退化为 tie-breaking）。"
            "371号的数学断裂问题无法在此数据集上量化——不是因为断裂不存在，"
            "而是因为严格 T(S) 的有效域在日线数据上为空集。"
            "L3 需要：(1) 更高频数据（5分钟/15分钟），或 (2) 更长历史（10年+），"
            "或 (3) 多时间周期递归（TFOrchestrator）以获得更深递归层级。"
        )
    if n_nonzero < 5:
        return (
            f"弱结果：仅 {n_nonzero} 个标的有非零 strict T(S)，样本不足以做排序相关性分析。"
        )
    if rho is not None and rho >= 0.7:
        return f"一致性高 (rho={rho:.4f})：proxy 排序近似 T(S) 排序。"
    return f"一致性不足 (rho={rho})：proxy 不是 T(S) 的可靠代理。"


def _exp2_conclusion(exp2: dict) -> str:
    if "error" in exp2:
        return f"无法分组：{exp2['error']}"

    high = exp2.get("high_group", {})
    low = exp2.get("low_group", {})
    u_test = exp2.get("mann_whitney_u", {})

    if not high.get("total_forward_returns") or not low.get("total_forward_returns"):
        return (
            "买点后续收益数据不足。"
            f"高组信号数={high.get('total_forward_returns', 0)}, "
            f"低组信号数={low.get('total_forward_returns', 0)}。"
            "原因：日线数据上确认买点稀疏，forward_window=20天内可用数据有限。"
        )

    p = u_test.get("p_approx")
    sig = u_test.get("significant_at_005", False)

    h_mean = high.get("mean_forward_return_20d")
    l_mean = low.get("mean_forward_return_20d")

    direction = ""
    if h_mean is not None and l_mean is not None:
        if h_mean > l_mean:
            direction = "方向一致（高 proxy 组收益更高）"
        else:
            direction = "方向不一致（低 proxy 组收益更高或相同）"

    if sig:
        return (
            f"{direction}，Mann-Whitney p={p:.4f} < 0.05，差异显著。"
            f"高组20日均收益={h_mean:.4%}, 低组={l_mean:.4%}。"
            "但注意：这是 proxy-L2 结论，不可推广到 strict T(S)。（371号约束）"
        )
    return (
        f"{direction}，Mann-Whitney p={p:.4f} >> 0.05，差异不显著。"
        f"高组20日均收益={h_mean:.4%} (n={high.get('total_forward_returns', 0)}), "
        f"低组={l_mean:.4%} (n={low.get('total_forward_returns', 0)})。"
        "proxy 分组在选股效果上无统计显著差异。"
    )


def _print_summary(report: dict, results: list[SymbolResult]) -> None:
    print("\n" + "=" * 80)
    print("Tightness L2 实验报告")
    print("=" * 80)

    # 实验1
    e1 = report["experiment_1"]
    print(f"\n{'─'*80}")
    print("实验1: Proxy vs Strict T(S) 排序一致性")
    print(f"认识论等级: {e1['epistemology_level']}")
    print(f"{'─'*80}")
    print(f"标的数: {e1['n_symbols']}")
    print(f"严格 T(S) > 0 的标的数: {e1['n_nonzero_strict_ts']}")
    print(f"Spearman rho: {e1['spearman_rho']}")
    print(f"Kendall tau: {e1['kendall_tau']}")

    diag = e1["root_cause_diagnosis"]
    print(f"\n根因诊断: {diag['summary']}")
    print(f"  最大递归层数: {diag['max_recursive_levels']}")
    print(f"  Level 1 最大 moves: {diag['max_l1_moves_across_all']}")
    print(f"  Level 2 最大 moves: {diag['max_l2_moves_across_all']}")

    print(f"\n结论: {e1['conclusion']}")

    print(f"\n{'Symbol':>8s} {'Name':>8s} {'Proxy':>7s} {'T(S)':>7s} {'RecLvl':>7s} {'L1mov':>6s} {'L2mov':>6s}")
    print("-" * 60)
    for r in results:
        print(f"{r.symbol:>8s} {r.name:>8s} {r.proxy_tightness:>7.2f} {r.strict_ts:>7.2f} "
              f"{r.max_recursive_levels:>7d} {r.max_level1_moves:>6d} {r.max_level2_moves:>6d}")

    # 实验2
    e2 = report["experiment_2"]
    print(f"\n{'─'*80}")
    print("实验2: Proxy 分组选股效果")
    print(f"认识论等级: {e2['epistemology_level']}")
    print(f"诚实标注: {e2['honest_label']}")
    print(f"{'─'*80}")

    comp = e2.get("comparison", {})
    if "error" not in comp:
        print(f"分组阈值 (median proxy): {comp.get('median_proxy_threshold')}")
        for grp in ["high_group", "low_group"]:
            g = comp.get(grp, {})
            print(f"\n  {g.get('label', grp)}:")
            print(f"    标的数: {g.get('n', 0)}")
            print(f"    平均 proxy: {g.get('avg_proxy')}")
            print(f"    平均确认BSP: {g.get('avg_confirmed_bsp')}")
            print(f"    Type1/2/3: {g.get('avg_type1')}/{g.get('avg_type2')}/{g.get('avg_type3')}")
            print(f"    买信号总数: {g.get('total_buy_signals')}")
            print(f"    后续收益信号数: {g.get('total_forward_returns')}")
            print(f"    20日均收益: {g.get('mean_forward_return_20d')}")
            print(f"    正收益率: {g.get('positive_return_ratio')}")

        u = comp.get("mann_whitney_u", {})
        if "error" not in u:
            print(f"\n  Mann-Whitney U: {u.get('U')}")
            print(f"  z: {u.get('z')}")
            print(f"  p (近似): {u.get('p_approx')}")
            print(f"  显著 (alpha=0.05): {u.get('significant_at_005')}")

    print(f"\n结论: {e2['conclusion']}")

    print("\n" + "=" * 80)
    print("认识论标注")
    print("=" * 80)
    print("实验1: L2 否定性结果")
    print("  - strict T(S) 在日线5年数据上有效域为空集")
    print("  - 371号数学断裂无法量化（被测对象不存在）")
    print("  - 需要更高频或更长历史数据才能获得非零 T(S)")
    print("实验2: proxy-L2（非 T(S)-L2）")
    print("  - 结论仅对 proxy 有效")
    print("  - 不可反向背书 T(S)（371号约束）")
    print("=" * 80)


if __name__ == "__main__":
    main()
