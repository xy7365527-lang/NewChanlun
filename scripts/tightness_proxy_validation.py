"""tightness-proxy-validation: 扩展 tightness proxy 与 350号严格 T(S) 的一致性检验。

366号边界条件4："如果真实 T(S)（从 NestedDivergence 计算）的排序与 proxy 排序不一致，
L2 结论需重新评估。"

方法：对同一组标的，同时计算：
  1. 扩展版 tightness proxy（结构深度梯度，scanner_l2 使用的版本）
  2. 严格 T(S) = D(S) * L(S) * C(S)（350号定义，从 NestedDivergence 链计算）
然后比较两者的排序是否一致（Spearman rank correlation）。

认识论等级：L2（真实数据验证）。
谱系引用：350号、366号边界条件4。
"""

from __future__ import annotations

import json
import logging
import sys
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path

_project_root = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(_project_root / "src"))

from newchan.a_nested_divergence import NestedDivergence, nested_divergence_search
from newchan.orchestrator.recursive import RecursiveOrchestrator
from newchan.types import Bar

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(name)s %(message)s")
logger = logging.getLogger("tightness_proxy_validation")

# 复用 scanner_l2 缓存
DATA_DIR = _project_root / "data" / "scanner_l2"
OUTPUT_DIR = _project_root / "data" / "tightness_proxy_validation"

SYMBOLS: dict[str, dict] = {
    "600547": {"name": "山东黄金"},
    "002155": {"name": "湖南黄金"},
    "600489": {"name": "中金黄金"},
    "600028": {"name": "中国石化"},
    "601857": {"name": "中国石油"},
    "600688": {"name": "上海石化"},
    "601398": {"name": "工商银行"},
    "601288": {"name": "农业银行"},
    "600048": {"name": "保利发展"},
    "000002": {"name": "万科A"},
    "001979": {"name": "招商蛇口"},
    "002230": {"name": "科大讯飞"},
    "300059": {"name": "东方财富"},
    "000725": {"name": "京东方A"},
    "601318": {"name": "中国平安"},
    "600036": {"name": "招商银行"},
    "601166": {"name": "兴业银行"},
    "600519": {"name": "贵州茅台"},
    "000858": {"name": "五粮液"},
    "002304": {"name": "洋河股份"},
    "300760": {"name": "迈瑞医疗"},
    "600276": {"name": "恒瑞医药"},
    "601985": {"name": "中国核电"},
    "600900": {"name": "长江电力"},
}


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
# 两种 tightness 计算
# ═══════════════════════════════════════════════════════════════


def _compute_proxy_tightness(snap) -> float:
    """扩展版 tightness proxy（复用 scanner_l2 逻辑）。"""
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


def _compute_strict_tightness(snap) -> tuple[float, int, int, float]:
    """严格 T(S) = D(S) * L(S) * C(S)（350号定义）。

    从 NestedDivergence 链计算。取全历史中最深（D最大）的链。

    Returns
    -------
    (T, D, L, C)
    """
    nested_divs = nested_divergence_search(snap)
    if not nested_divs:
        return 0.0, 0, 0, 0.0

    # 选取最深的嵌套链
    best_chain: list[tuple[int, object]] = []
    for nd in nested_divs:
        valid_entries = [(lvl, div) for lvl, div in nd.chain if div is not None]
        if len(valid_entries) > len(best_chain):
            best_chain = valid_entries

    if not best_chain:
        return 0.0, 0, 0, 0.0

    # D(S) = 有背驰的级别数
    d_val = len(best_chain)

    # L(S) = 最高级别的 level_id
    l_val = max(lvl for lvl, _ in best_chain)

    # C(S) = 方向一致率
    directions = [div.direction for _, div in best_chain]
    if not directions:
        return 0.0, d_val, l_val, 0.0
    from collections import Counter
    counts = Counter(directions)
    max_count = counts.most_common(1)[0][1]
    c_val = max_count / d_val if d_val > 0 else 0.0

    t_val = d_val * l_val * c_val
    return t_val, d_val, l_val, c_val


@dataclass(frozen=True, slots=True)
class SymbolComparison:
    """单标的的两种 tightness 对比结果。"""

    symbol: str
    name: str
    proxy_tightness: float
    strict_tightness: float
    strict_d: int
    strict_l: int
    strict_c: float


# ═══════════════════════════════════════════════════════════════
# 排序一致性
# ═══════════════════════════════════════════════════════════════


def _spearman_rank_correlation(xs: list[float], ys: list[float]) -> float | None:
    """Spearman rank correlation（手动实现）。"""
    import math
    n = len(xs)
    if n < 3:
        return None

    def _rank(vals: list[float]) -> list[float]:
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

    rx = _rank(xs)
    ry = _rank(ys)

    d_sq = sum((rx[i] - ry[i]) ** 2 for i in range(n))
    rho = 1 - 6 * d_sq / (n * (n * n - 1))
    return rho


# ═══════════════════════════════════════════════════════════════
# 主程序
# ═══════════════════════════════════════════════════════════════


def main() -> None:
    logger.info("=== Tightness Proxy Validation ===")
    logger.info("验证目标: 扩展 proxy 排序 vs 严格 T(S) 排序一致性")

    comparisons: list[SymbolComparison] = []

    for symbol, info in SYMBOLS.items():
        bars = _load_bars(symbol)
        if len(bars) < 200:
            logger.warning("数据不足: %s — %d bars, 跳过", symbol, len(bars))
            continue

        logger.info("处理: %s (%s) — %d bars", symbol, info["name"], len(bars))
        orch = RecursiveOrchestrator(stream_id=symbol, max_levels=4)

        max_proxy = 0.0
        max_strict = 0.0
        max_d = 0
        max_l = 0
        max_c = 0.0

        for bar in bars:
            snap = orch.process_bar(bar)

            proxy_t = _compute_proxy_tightness(snap)
            if proxy_t > max_proxy:
                max_proxy = proxy_t

            strict_t, d, l, c = _compute_strict_tightness(snap)
            if strict_t > max_strict:
                max_strict = strict_t
                max_d = d
                max_l = l
                max_c = c

        comparisons.append(SymbolComparison(
            symbol=symbol,
            name=info["name"],
            proxy_tightness=max_proxy,
            strict_tightness=max_strict,
            strict_d=max_d,
            strict_l=max_l,
            strict_c=max_c,
        ))

        logger.info("  → proxy=%.2f, strict T=%.2f (D=%d, L=%d, C=%.2f)",
                     max_proxy, max_strict, max_d, max_l, max_c)

    if len(comparisons) < 3:
        logger.error("有效标的不足（<3），无法计算排序相关性")
        sys.exit(1)

    # 计算排序相关性
    proxy_vals = [c.proxy_tightness for c in comparisons]
    strict_vals = [c.strict_tightness for c in comparisons]
    rho = _spearman_rank_correlation(proxy_vals, strict_vals)

    # 排序对比
    proxy_order = sorted(range(len(comparisons)),
                         key=lambda i: -comparisons[i].proxy_tightness)
    strict_order = sorted(range(len(comparisons)),
                          key=lambda i: -comparisons[i].strict_tightness)

    proxy_rank = {i: rank for rank, i in enumerate(proxy_order)}
    strict_rank = {i: rank for rank, i in enumerate(strict_order)}

    # 非零子集分析
    nonzero_strict = [c for c in comparisons if c.strict_tightness > 0]
    if len(nonzero_strict) >= 3:
        nz_proxy = [c.proxy_tightness for c in nonzero_strict]
        nz_strict = [c.strict_tightness for c in nonzero_strict]
        rho_nonzero = _spearman_rank_correlation(nz_proxy, nz_strict)
    else:
        rho_nonzero = None

    # 组装报告
    report = {
        "meta": {
            "purpose": "验证扩展 tightness proxy 与严格 T(S)=D*L*C 的排序一致性",
            "epistemology_level": "L2",
            "genealogy_ref": "366号边界条件4 + 350号",
            "total_symbols": len(comparisons),
            "nonzero_strict": len(nonzero_strict),
        },
        "spearman_rho": round(rho, 4) if rho is not None else None,
        "spearman_rho_nonzero_subset": round(rho_nonzero, 4) if rho_nonzero is not None else None,
        "interpretation": _interpret_rho(rho, len(nonzero_strict)),
        "per_symbol": [
            {
                "symbol": c.symbol,
                "name": c.name,
                "proxy_tightness": round(c.proxy_tightness, 4),
                "strict_tightness": round(c.strict_tightness, 4),
                "strict_d": c.strict_d,
                "strict_l": c.strict_l,
                "strict_c": round(c.strict_c, 4),
                "proxy_rank": proxy_rank[i] + 1,
                "strict_rank": strict_rank[i] + 1,
                "rank_delta": strict_rank[i] - proxy_rank[i],
            }
            for i, c in enumerate(comparisons)
        ],
    }

    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    report_path = OUTPUT_DIR / "proxy_validation_report.json"
    report_path.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    logger.info("报告已保存: %s", report_path)

    # 打印摘要
    print("\n" + "=" * 70)
    print("Tightness Proxy Validation 结果摘要")
    print("=" * 70)

    print(f"\n标的数: {len(comparisons)} (严格T>0: {len(nonzero_strict)})")
    print(f"Spearman rho (全集): {rho:.4f}" if rho is not None else "Spearman rho: N/A")
    if rho_nonzero is not None:
        print(f"Spearman rho (T>0 子集): {rho_nonzero:.4f}")

    print("\n排序对比:")
    print(f"{'Symbol':>8s} {'Name':>8s} {'Proxy':>7s} {'P-rank':>7s} {'Strict':>7s} {'S-rank':>7s} {'Delta':>6s}")
    print("-" * 60)
    for i, c in enumerate(comparisons):
        print(f"{c.symbol:>8s} {c.name:>8s} {c.proxy_tightness:>7.2f} #{proxy_rank[i]+1:<6d} "
              f"{c.strict_tightness:>7.2f} #{strict_rank[i]+1:<6d} {strict_rank[i]-proxy_rank[i]:>+5d}")

    print(f"\n{report['interpretation']}")
    print("=" * 70)


def _interpret_rho(rho: float | None, n_nonzero: int) -> str:
    """根据 rho 值给出解读。"""
    if rho is None:
        return "样本不足，无法计算排序相关性"
    if n_nonzero < 5:
        return (f"rho={rho:.4f}，但严格T>0的标的仅{n_nonzero}个，"
                "样本过少。366号L2结论在严格T(S)维度上暂无法交叉验证。"
                "根本原因可能是日线数据对区间套背驰搜索的分辨率不足。")
    if rho >= 0.7:
        return (f"rho={rho:.4f} >= 0.7：排序高度一致。"
                "扩展 proxy 在排序意义上是严格 T(S) 的有效近似。366号L2结论稳健。")
    if rho >= 0.4:
        return (f"rho={rho:.4f}，中等一致。"
                "扩展 proxy 部分反映严格 T(S) 的排序，但存在系统性偏差。"
                "366号L2结论的商空间排序优势可能需要用严格T(S)重新验证。")
    return (f"rho={rho:.4f} < 0.4：排序一致性低。"
            "扩展 proxy 不是严格 T(S) 的可靠代理。"
            "366号L2结论需用严格T(S)重新评估——商空间排序的优势可能不存在于严格定义下。")


if __name__ == "__main__":
    main()
