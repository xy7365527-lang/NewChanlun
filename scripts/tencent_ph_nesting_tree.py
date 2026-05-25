"""腾讯 700 — H0 merge tree 嵌套结构是否 = 缠论级别递归？

核心问题（2026-05-26 编排者）
----------------------------
PH 能否从 barcode 嵌套结构内在地给出"次级别"关系？若能，递归变成内在的，
不需人为构造（"日线的次级别是 30 分钟"）。

理论张力（实测前先标注，避免确认偏差）
--------------------------------------
H0 sublevel persistence 的 bar 按 [birth, death] **价格**区间嵌套，形成 merge tree
（数学必然：年轻分量 merge 进年长分量 → containment 成立）。但：
  - merge tree 嵌套轴 = 价格 prominence（价值轴）
  - 缠论级别递归轴 = 时间 span（时间轴）
两轴相关但不等同（239号 ker(D)：H0 时间盲）。

**决定性测量**：
  - 若 depth（树深度）与 span 强相关 → 树 ≈ 缠论级别递归（时间轴一致）
  - 若 depth 只与 persistence 相关、与 span 弱相关 → 树是 prominence 层级，
    不是时间层级 → 不能平替缠论级别（H1 也无法并入：单位不可比）

认识论等级
----------
- merge tree 构造：L0（纯算法）。
- "嵌套 = 缠论次级别"经验断言：L2（腾讯 700 单标的日线，可否证）。
"""

from __future__ import annotations

import json
import sys
from dataclasses import dataclass
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))

import pandas as pd  # noqa: E402

from newchan.a_fractal import fractals_from_merged  # noqa: E402
from newchan.a_inclusion import merge_inclusion  # noqa: E402
from newchan.a_stroke import strokes_from_fractals  # noqa: E402

DATA = Path(__file__).resolve().parent.parent / "tmp" / "tencent"


@dataclass
class TreeNode:
    """merge tree 节点 = 一个 H0 prominence 特征。"""

    cid: int          # 分量 id = 诞生局部极小的索引
    birth: float      # 诞生价（局部极小）
    death: float      # 死亡价（merge 处的鞍点/极大）；root = cap
    birth_idx: int    # 诞生时间索引
    death_idx: int | None
    lo: int           # 死亡时分量覆盖的最左索引
    hi: int           # 死亡时分量覆盖的最右索引
    parent: int | None  # 父节点 cid（merge 进的年长分量）

    @property
    def persistence(self) -> float:
        return self.death - self.birth

    @property
    def span(self) -> int:
        return self.hi - self.lo + 1


def merge_tree_h0(values, finite_cap: float | None = None) -> dict[int, TreeNode]:
    """sublevel-set H0 merge tree（带时间 span + parent 指针）。

    扩展 a_persistence_barcode.sublevel_h0_bars：除 (birth, death) 外，追踪
    每个分量的时间索引区间 [lo, hi] 和 merge 进的父分量。persistence=0 的
    奇点（诞生即死）不记为节点，但仍参与 lo/hi 传播。
    """
    n = len(values)
    vals = [float(v) for v in values]
    if n == 0:
        return {}
    cap = max(vals) if finite_cap is None else float(finite_cap)

    parent_uf: dict[int, int] = {}
    bval: dict[int, float] = {}   # root -> birth value
    bidx: dict[int, int] = {}     # root -> birth index (component id)
    lo: dict[int, int] = {}
    hi: dict[int, int] = {}
    active = [False] * n
    nodes: dict[int, TreeNode] = {}

    def find(x: int) -> int:
        while parent_uf[x] != x:
            parent_uf[x] = parent_uf[parent_uf[x]]
            x = parent_uf[x]
        return x

    for i in sorted(range(n), key=lambda k: vals[k]):
        vi = vals[i]
        parent_uf[i] = i
        bval[i] = vi
        bidx[i] = i
        lo[i] = i
        hi[i] = i
        active[i] = True

        nbr: list[int] = []
        for j in (i - 1, i + 1):
            if 0 <= j < n and active[j]:
                r = find(j)
                if r not in nbr:
                    nbr.append(r)
        if not nbr:
            continue  # 局部极小：新分量诞生

        comps = list(dict.fromkeys([find(i), *nbr]))
        elder = min(comps, key=lambda r: bval[r])
        for r in comps:
            if r == elder:
                continue
            b = bval[r]
            if b < vi:  # 真实特征（persistence>0）
                nodes[bidx[r]] = TreeNode(
                    cid=bidx[r], birth=b, death=vi, birth_idx=bidx[r],
                    death_idx=i, lo=lo[r], hi=hi[r], parent=bidx[elder],
                )
            # 合并 lo/hi 进 elder
            lo[elder] = min(lo[elder], lo[r])
            hi[elder] = max(hi[elder], hi[r])
            parent_uf[r] = elder

    # 存活的全局分量（路径图全连通 → 恰好一个），死亡封顶
    for r in {find(i) for i in range(n)}:
        nodes[bidx[r]] = TreeNode(
            cid=bidx[r], birth=bval[r], death=cap, birth_idx=bidx[r],
            death_idx=None, lo=lo[r], hi=hi[r], parent=None,
        )
    return nodes


def assign_depth(nodes: dict[int, TreeNode]) -> dict[int, int]:
    """每个节点到 root 的深度（root=0）。"""
    depth: dict[int, int] = {}

    def d(cid: int) -> int:
        if cid in depth:
            return depth[cid]
        p = nodes[cid].parent
        depth[cid] = 0 if p is None else d(p) + 1
        return depth[cid]

    for cid in nodes:
        d(cid)
    return depth


def children_of(nodes: dict[int, TreeNode]) -> dict[int, list[int]]:
    ch: dict[int, list[int]] = {cid: [] for cid in nodes}
    for cid, nd in nodes.items():
        if nd.parent is not None:
            ch[nd.parent].append(cid)
    return ch


def strokes_spans(df: pd.DataFrame) -> list[int]:
    df_m, m2r = merge_inclusion(df)
    fr = fractals_from_merged(df_m)
    strokes = strokes_from_fractals(df_m, fr, mode="new", merged_to_raw=m2r)
    spans = []
    for s in strokes:
        r0, r1 = m2r[s.i0][0], m2r[s.i1][1]
        spans.append(abs(r1 - r0) + 1)
    return spans


def pearson(x: list[float], y: list[float]) -> float:
    import numpy as np
    ax, ay = np.array(x, float), np.array(y, float)
    if ax.std() == 0 or ay.std() == 0:
        return float("nan")
    return float(np.corrcoef(ax, ay)[0, 1])


def analyze(name: str, raw: list[dict]) -> None:
    closes = [float(b["close"]) for b in raw]
    df = pd.DataFrame({k: [float(b[k]) for b in raw] for k in ("open", "high", "low", "close")})

    nodes = merge_tree_h0(closes)
    depth = assign_depth(nodes)
    ch = children_of(nodes)
    max_depth = max(depth.values())

    print("#" * 88)
    print(f"# 腾讯 700 {name} — H0 merge tree 嵌套结构 (n_bars={len(closes)}, n_nodes={len(nodes)})")
    print("#" * 88)

    # ---- 1. 时间嵌套一致性检验（child [lo,hi] ⊆ parent [lo,hi]?）----
    violations = 0
    for cid, nd in nodes.items():
        if nd.parent is None:
            continue
        p = nodes[nd.parent]
        if not (p.lo <= nd.lo and nd.hi <= p.hi):
            violations += 1
    print(f"\n[1] 时间嵌套一致性: child[lo,hi]⊆parent[lo,hi] 违例 = {violations}/{len(nodes)-1}")
    print("    （0 违例 = 价格嵌套同时保证时间区间嵌套 → 树是真正的区间套）")

    # ---- 2. 树形打印（按深度缩进）----
    print(f"\n[2] 嵌套树 (root→leaf, 最大深度={max_depth})")
    print(f"    {'depth':>5} {'价格区间[birth,death]':>22} {'pers':>7} {'时间[lo,hi]':>14} {'span':>5} {'子':>3}")
    root = next(cid for cid, nd in nodes.items() if nd.parent is None)

    def show(cid: int, indent: int) -> None:
        nd = nodes[cid]
        bar = f"[{nd.birth:.1f},{nd.death:.1f}]"
        tspan = f"[{nd.lo},{nd.hi}]"
        pad = "  " * indent
        print(f"    {depth[cid]:>5} {pad}{bar:>{22-len(pad)}} {nd.persistence:>7.1f} "
              f"{tspan:>14} {nd.span:>5} {len(ch[cid]):>3}")
        # 子节点按 persistence 降序
        for c in sorted(ch[cid], key=lambda c: nodes[c].persistence, reverse=True):
            if depth[c] <= 4:  # 控制打印深度
                show(c, indent + 1)

    show(root, 0)

    # ---- 3. 决定性测量：depth/persistence/span 相关性 ----
    cids = list(nodes.keys())
    deps = [float(depth[c]) for c in cids]
    pers = [nodes[c].persistence for c in cids]
    spans = [float(nodes[c].span) for c in cids]
    print(f"\n[3] ★决定性测量：树结构的轴是价格还是时间？(n_nodes={len(cids)})")
    print(f"    corr(depth, persistence) = {pearson(deps, pers):+.3f}  (强负→深度=价格层级)")
    print(f"    corr(depth, span)        = {pearson(deps, spans):+.3f}  (强负→深度=时间层级=缠论级别)")
    print(f"    corr(persistence, span)  = {pearson(pers, spans):+.3f}  (强正→两轴一致；弱→两轴分裂)")

    # ---- 4. 按深度分层的 span 分布（缠论级别应随深度单调）----
    print(f"\n[4] 按深度分层 span 统计（缠论级别 = span 随深度递减且分层清晰）")
    import numpy as np
    print(f"    {'depth':>5} {'n':>4} {'span均值':>9} {'span中位':>9} {'span范围':>14} {'pers均值':>9}")
    for d_ in range(max_depth + 1):
        grp = [c for c in cids if depth[c] == d_]
        if not grp:
            continue
        sp = [nodes[c].span for c in grp]
        pe = [nodes[c].persistence for c in grp]
        print(f"    {d_:>5} {len(grp):>4} {np.mean(sp):>9.1f} {np.median(sp):>9.1f} "
              f"{f'[{min(sp)},{max(sp)}]':>14} {np.mean(pe):>9.1f}")

    # ---- 5. 与缠论笔 span 对比 ----
    spans_stroke = strokes_spans(df)
    leaf_spans = [nodes[c].span for c in cids if not ch[c]]
    print(f"\n[5] 与缠论新笔 span 对比")
    print(f"    缠论笔: n={len(spans_stroke)}, span 均值={np.mean(spans_stroke):.1f}, "
          f"中位={np.median(spans_stroke):.1f}, 范围=[{min(spans_stroke)},{max(spans_stroke)}]")
    print(f"    PH叶节点: n={len(leaf_spans)}, span 均值={np.mean(leaf_spans):.1f}, "
          f"中位={np.median(leaf_spans):.1f}, 范围=[{min(leaf_spans)},{max(leaf_spans)}]")
    print(f"    （叶节点 ≈ 笔? 若 span 分布接近 → PH 最细层 = 缠论笔级）")


def main() -> None:
    daily = json.loads((DATA / "daily_ohlcv.json").read_text())["bars"]
    analyze("日线", daily)
    print()
    m30 = json.loads((DATA / "m30_ohlcv.json").read_text())["bars"]
    analyze("30分钟", m30)


if __name__ == "__main__":
    main()
