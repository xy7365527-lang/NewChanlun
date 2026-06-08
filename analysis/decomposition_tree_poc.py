"""纵向圈闭合概念验证（PoC）——真实数据跑通分解树管线。

认识论等级：**L1（管线正确性验证）**。
  本脚本验证 DecompositionTree → VerticalClosure → SelectionRanker 在真实数据上
  跑得通、产出结构正确。它**不验证选股 alpha**（那是 L2，需完整成分股数据 + 回测
  对照随机/等权基准）。L0→L1 信息增量为零（formalization-validity-domain 规则）。

数据缺口诚实标注（231号 / no-patch）：
  analysis/data_cache/ 中**无单个成分股**（NVDA/AAPL/MSFT…）数据。故 PoC 用可得的
  行业 ETF（SOXX=半导体，QQQ 真实子行业）+ 大盘指数（SPY）代理成分，**权重用示意值
  而非真实市值权重**。成分覆盖不完整 → PoC 严格停留 L1，不外推到"纵向选股有效"。

PoC 配置（日线，2021-06 ~ 2026-05，1255 根）：
  参照系 ref = GLD（商品侧参照，C-fold 观测量）
  宏观 P     = QQQ（生产资本指数代理），macro σ = σ(QQQ/GLD)
  成分       = SOXX（半导体行业）、SPY（大盘权益），σ = σ(成分/GLD)

运行：PYTHONPATH=src python analysis/decomposition_tree_poc.py
"""

from __future__ import annotations

import json
from datetime import datetime
from pathlib import Path

from newchan.orchestrator.recursive import RecursiveOrchestrator
from newchan.topology.config_space import WalkDirection
from newchan.topology.decomposition_tree import (
    ComponentReading,
    DecompositionNode,
    DecompositionTree,
    SelectionRanker,
    TreeLayer,
    VerticalClosure,
    a0_precision_residual,
    sedimentation_signal,
)
from newchan.topology.graph import Vertex
from newchan.types import Bar

_DATA = Path(__file__).resolve().parent / "data_cache"

# 顶点分解配置（示意权重，非真实市值权重——数据缺口）。
REF_FILE = "gld_1d_yf.json"          # 参照系 C（商品侧）
MACRO_FILE = "QQQ_1d_5y.json"        # 宏观 P（QQQ）
COMPONENTS = [                        # (label, file, 示意权重)
    ("SOXX", "SOXX_1d_5y.json", 0.5),
    ("SPY", "spy_1d_yf.json", 0.5),
]


def _load(filename: str) -> dict[str, list]:
    return json.load(open(_DATA / filename))


def _series_reading(name: str, num: dict, ref: dict | None) -> tuple[WalkDirection, int]:
    """对序列跑递归取 (最高级别走势方向 σ, 最高涌现级别 level)（cross_national 同口径）。

    各边从 a0 各自递归，涌现自己的级别体系（526号：三个 L1 不是同一个 L1）——
    level 由本边独立涌现，不跨边对齐，供 Layer B 级别加权置信度。

    ref=None  → 绝对方向 σ(num/$)，用 num 自身真实 OHLC。
    ref 给定  → 比价方向 σ(num/ref)，用正确的比价 OHLC：
                比值最高 = num_high/ref_low、最低 = num_low/ref_high（非退化 bar）。
    """
    if ref is None:
        dates = [d[:10] for d in num["dates"]]
        o, h, l, c = num["opens"], num["highs"], num["lows"], num["closes"]
    else:
        rb = {d[:10]: i for i, d in enumerate(ref["dates"])}
        dates, o, h, l, c = [], [], [], [], []
        for j, d in enumerate(num["dates"]):
            k = d[:10]
            i = rb.get(k)
            if i is None or ref["lows"][i] <= 0 or ref["highs"][i] <= 0:
                continue
            dates.append(k)
            o.append(num["opens"][j] / ref["opens"][i])
            h.append(num["highs"][j] / ref["lows"][i])   # 比值最高
            l.append(num["lows"][j] / ref["highs"][i])    # 比值最低
            c.append(num["closes"][j] / ref["closes"][i])

    orch = RecursiveOrchestrator(stream_id=name, max_levels=6, stroke_mode="wide")
    snap = None
    for i in range(len(c)):
        snap = orch.process_bar(
            Bar(ts=datetime.strptime(dates[i], "%Y-%m-%d"),
                open=o[i], high=h[i], low=l[i], close=c[i])
        )
    assert snap is not None
    level = 1
    for rs in snap.recursive_snapshots:
        level = max(level, rs.level_id)

    moves = snap.move_snapshot.moves
    if not moves or moves[-1].kind == "consolidation":
        return WalkDirection.FLAT, level
    if moves[-1].direction == "up":
        return WalkDirection.UP, level
    if moves[-1].direction == "down":
        return WalkDirection.DOWN, level
    return WalkDirection.FLAT, level


def _layer_a_data_quality() -> None:
    """Layer A：a0 精确闭合（指数复制残差）——数据质量门（设计文档 §3.1）。

    检查成分加权能否复制宏观指数。PoC 用 ETF 代理（SOXX/SPY 非 QQQ 真成分），
    预期 is_clean=False——这是 Layer A 正确履职：在数据层报告"成分无法复制宏观"，
    自动阻止 Layer B 选股结论被误信。诚实暴露"缺真实成分股数据"。
    """
    macro = _load(MACRO_FILE)
    macro_by_date = {d[:10]: c for d, c in zip(macro["dates"], macro["closes"])}
    comp_loaded = [(lbl, w, _load(f)) for lbl, f, w in COMPONENTS]

    # 公共日期对齐（QQQ ∩ 各成分）。
    common = set(macro_by_date)
    comp_by_date = []
    for lbl, w, d in comp_loaded:
        bd = {dd[:10]: cc for dd, cc in zip(d["dates"], d["closes"])}
        comp_by_date.append((w, bd))
        common &= set(bd)
    dates = sorted(common)

    macro_prices = [macro_by_date[d] for d in dates]
    weighted = tuple((w, [bd[d] for d in dates]) for w, bd in comp_by_date)
    r = a0_precision_residual(macro_prices, weighted)

    print(f"\n{'─' * 70}\nLayer A：a0 精确闭合（数据质量门，L1）\n{'─' * 70}")
    print(f"  ρ(t)=log(QQQ)−log(Σ wᵢ·成分ᵢ) over {r.n_bars} bars")
    print(f"  mean|ρ|={r.mean_abs_residual:.4f}  max|ρ|={r.max_abs_residual:.4f}  "
          f"is_clean={r.is_clean}")
    if not r.is_clean:
        print("  → 数据质量门未通过（预期）：SOXX+SPY 非 QQQ 真成分，无法复制指数。")
        print("    Layer B 结论仅作管线验证（L1），不可外推选股有效性。需真实成分股数据。")


def _build_tree() -> DecompositionTree:
    """构建分解树（Layer 2 US → Layer 1 P 顶点 → Layer 0 成分叶）。"""
    leaves = tuple(
        DecompositionNode(label=lbl, layer=TreeLayer.INSTRUMENT, weight=w, symbol=lbl)
        for lbl, _, w in COMPONENTS
    )
    p_node = DecompositionNode(
        label="P", layer=TreeLayer.VERTEX_DECOMPOSITION, weight=1.0,
        vertex=Vertex.P, symbol="QQQ", children=leaves,
    )
    return DecompositionTree(
        DecompositionNode(label="US", layer=TreeLayer.NATIONAL_K4, weight=1.0,
                          children=(p_node,))
    )


def _run_scenario(title: str, ref_file: str | None) -> None:
    """跑一个参照系场景：ref_file=None 绝对方向(ref=M)；否则比价(ref=该文件)。"""
    ref = _load(ref_file) if ref_file else None
    ref_label = "$" if ref is None else "GLD"
    macro = _load(MACRO_FILE)

    print(f"\n{'─' * 70}\n场景：{title}  (参照系 ref = {ref_label})\n{'─' * 70}")

    macro_sigma, macro_level = _series_reading(f"QQQ/{ref_label}", macro, ref)
    print(f"[宏观] σ(QQQ/{ref_label}) = {macro_sigma.name} (level={macro_level})")

    comp_readings = []
    for lbl, fname, w in COMPONENTS:
        sig, lvl = _series_reading(f"{lbl}/{ref_label}", _load(fname), ref)
        comp_readings.append(ComponentReading(symbol=lbl, weight=w, sigma=sig, level=lvl))
        print(f"[成分] σ({lbl}/{ref_label}) = {sig.name} (权重={w} level={lvl})")

    closure = VerticalClosure.check(macro_sigma, tuple(comp_readings), macro_level=macro_level)
    print(f"[Layer B 软闭合] synth={closure.synth:+.3f} compatible={closure.compatible} "
          f"incompat={closure.incompatibility:.3f} "
          f"concentration={closure.concentration:+.3f} "
          f"({'集中→脆弱' if closure.concentration > 0.05 else '广泛参与/稳健'})")

    # 资本病理学：纵向沉没检测（§8.3，对任何宏观方向都可读）。
    sed = sedimentation_signal(closure)
    if sed.entries:
        sed_str = ", ".join(f"{e.symbol}={e.score:.3f}(L{e.level})" for e in sed.entries)
        print(f"[沉没] total={sed.total:.3f} | {sed_str}（低级别 FLAT=死水）")
    else:
        print("[沉没] 无（所有成分有向，资本流转中）")

    ranking = SelectionRanker.rank(closure)
    if not ranking.entries:
        print("[排序] 宏观 FLAT → 空排序（292号区间套：上层闭合是下层分解前提）")
        return
    print("[排序] (contribution 降序；alpha 属 L2 未验证)")
    for i, e in enumerate(ranking.entries, 1):
        print(f"  {i}. {e.symbol:6s} contrib={e.contribution:+.3f} "
              f"residual={e.residual} role={e.role.value}")


def main() -> None:
    print("=" * 70)
    print("纵向圈闭合 PoC（L1 管线验证，非 alpha 验证）")
    print(f"宏观 P = QQQ | 成分 = {[c[0] for c in COMPONENTS]}")
    print("⚠ 权重为示意值（无真实市值权重数据）；成分用 ETF 代理（无个股数据）。")
    print("=" * 70)

    tree = _build_tree()
    print(f"[树结构] 叶标的: {[n.symbol for n in tree.leaves()]} | "
          f"P 顶点成分: {[n.label for n in tree.children_of('P')]}")

    # Layer A：数据质量门（指数复制残差）。
    _layer_a_data_quality()

    # Layer B 场景1：相对商品参照（ref=C），覆盖比价路径。
    _run_scenario("相对商品强弱（成分/GLD）", REF_FILE)
    # Layer B 场景2：绝对方向（ref=M=$），用真实 OHLC，覆盖完整排序路径。
    _run_scenario("绝对方向（成分自身价格）", None)

    print("\n" + "=" * 70)
    print("✓ 管线贯通（L1）：两层闭合（A 数据质量门 + B 软闭合）+ 级别加权 +")
    print("  排序 + 空排序门控均产出正确结构。")
    print("  选股有效性需 L2 回测（个股数据补齐 + 对照随机/等权基准）。")
    print("=" * 70)


if __name__ == "__main__":
    main()
