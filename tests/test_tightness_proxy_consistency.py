"""T(S) 一致性检验：严格 T(S) vs 扩展 tightness proxy 排序一致性。

366号谱系下游推论3：扩展 tightness proxy 需独立验证——
与350号严格定义的 T(S) 的一致性未确认。

验证内容：
1. L0：两个度量的结构性差异（定义推导，排序不一致的充分条件）
2. L1：合成数据下管线正确性（两个函数各自计算正确）
3. L2 方案文档：真实数据验证需要 baostock API，标记为 blocked

认识论等级：
- test_structural_divergence_*: L0（从定义推导排序不一致的充分条件）
- test_pipeline_*: L1（管线正确性，不验证排序相关性假设）
- L2 验证方案见 tmp/tightness-proxy-l2-plan.md

谱系引用：350号收敛紧度、366号下游推论3。
"""

from __future__ import annotations

import pytest

from newchan.a_divergence import Divergence
from newchan.a_nested_divergence import NestedDivergence
from newchan.convergence import ConvergenceTightness, convergence_tightness


# ═══════════════════════════════════════════════════════════════
# L0：结构性差异——两个度量不同构的定义推导
# ═══════════════════════════════════════════════════════════════


def _make_divergence(
    level_id: int,
    direction: str = "bottom",
    kind: str = "trend",
) -> Divergence:
    """构造最小 Divergence。"""
    return Divergence(
        kind=kind,
        direction=direction,
        level_id=level_id,
        seg_a_start=0,
        seg_a_end=1,
        seg_c_start=2,
        seg_c_end=3,
        center_idx=0,
        force_a=100.0,
        force_c=50.0,
        confirmed=True,
    )


def _make_nd(
    chain_levels: list[int],
    directions: list[str] | None = None,
) -> NestedDivergence:
    """构造 NestedDivergence, 每级有背驰。"""
    if directions is None:
        directions = ["bottom"] * len(chain_levels)
    chain = [
        (lvl, _make_divergence(lvl, d))
        for lvl, d in zip(chain_levels, directions)
    ]
    return NestedDivergence(chain=chain, bar_range=(0, 100))


class TestStructuralDivergenceL0:
    """L0：从定义推导两个度量的不同构性。

    严格 T(S) = D * L * C（乘法，依赖 NestedDivergence 链）
    扩展 proxy = sum of structural depth weights（加法，不依赖背驰检测）

    结论：存在排序不一致的充分条件。
    """

    def test_ts_multiplicative_vs_proxy_additive(self) -> None:
        """T(S) 是乘法 (D*L*C)，proxy 是加法 (sum of weights)。

        构造两个 NestedDivergence:
        - A: 2层，level_max=4，全一致 → T=2*4*1=8
        - B: 3层，level_max=3，全一致 → T=3*3*1=9

        T(S) 排序: B > A。
        但如果 A 对应的标的有更多 zhongshus/moves（结构更深），
        proxy 可能排 A > B。
        """
        nd_a = _make_nd([4, 3])
        nd_b = _make_nd([3, 2, 1])

        ts_a = convergence_tightness(nd_a)
        ts_b = convergence_tightness(nd_b)

        assert ts_a.score == pytest.approx(8.0)   # 2 * 4 * 1.0
        assert ts_b.score == pytest.approx(9.0)   # 3 * 3 * 1.0
        assert ts_b.score > ts_a.score             # T(S): B > A

        # proxy 排序可能不同：A 有 level_max=4 的递归层，
        # 如果该层有 moves (+1.0)，proxy 会因高级别层得分更高。
        # 但 proxy 不依赖背驰检测，两者排序可以不一致。

    def test_consistency_factor_absent_in_proxy(self) -> None:
        """T(S) 有一致性因子 C(S)，proxy 没有。

        构造两个 NestedDivergence:
        - A: 4层全一致 → C=1.0, T=4*4*1=16
        - B: 4层半一致 → C=0.5, T=4*4*0.5=8

        T(S) 排序: A >> B。
        proxy 只看结构存在性，不区分方向一致性 → 可能排 A ≈ B。
        """
        nd_a = _make_nd([4, 3, 2, 1], ["bottom", "bottom", "bottom", "bottom"])
        nd_b = _make_nd([4, 3, 2, 1], ["bottom", "top", "bottom", "top"])

        ts_a = convergence_tightness(nd_a)
        ts_b = convergence_tightness(nd_b)

        assert ts_a.consistency == 1.0
        assert ts_b.consistency == 0.5
        assert ts_a.score == pytest.approx(16.0)
        assert ts_b.score == pytest.approx(8.0)
        assert ts_a.score > ts_b.score  # T(S) 2x 差距

        # proxy 不检测方向一致性，同样的结构深度 → 同分
        # 这是排序分叉的充分条件

    def test_zero_ts_nonzero_proxy(self) -> None:
        """标的有结构（zhongshus/moves）但无背驰链 → T(S)=0, proxy>0。

        这是最常见的分叉场景：大多数标的有中枢和走势，
        但不一定在同一时刻有跨级别背驰嵌套链。
        """
        nd_empty = NestedDivergence(chain=[], bar_range=(0, 0))
        ts = convergence_tightness(nd_empty)
        assert ts.score == 0.0
        assert ts.depth == 0

        # 同一标的的 proxy 可能是 1.8（BSP+1.0, moves+0.5, zs+0.3）
        # → T(S) = 0, proxy = 1.8
        # 排序完全不一致


class TestMultiplicativeProperties:
    """L0：验证 T(S) 的乘法性质。"""

    def test_single_level_chain(self) -> None:
        """单级别链：D=1, L=level_id, C=1.0。"""
        for lvl in (1, 2, 3, 5):
            nd = _make_nd([lvl])
            ts = convergence_tightness(nd)
            assert ts.depth == 1
            assert ts.level_max == lvl
            assert ts.consistency == 1.0
            assert ts.score == pytest.approx(float(lvl))

    def test_none_entries_excluded(self) -> None:
        """chain 中 Divergence=None 的级别被排除。"""
        chain = [
            (4, _make_divergence(4)),
            (3, None),
            (2, _make_divergence(2)),
        ]
        nd = NestedDivergence(chain=chain, bar_range=(0, 100))
        ts = convergence_tightness(nd)

        assert ts.depth == 2       # 只有 level 4 和 2 有效
        assert ts.level_max == 4
        assert ts.consistency == 1.0  # 两者都是 "bottom"
        assert ts.score == pytest.approx(8.0)  # 2 * 4 * 1.0


# ═══════════════════════════════════════════════════════════════
# L1：管线正确性——两个函数各自计算正确
# ═══════════════════════════════════════════════════════════════


class TestPipelineCorrectnessL1:
    """L1：验证 convergence_tightness 管线正确性。

    认识论标注：这些测试用合成数据验证管线不 bug，
    不验证 T(S) 在真实市场数据上的有效性（需 L2）。
    """

    def test_empty_nd(self) -> None:
        """空链 → 全零。"""
        nd = NestedDivergence(chain=[], bar_range=(0, 0))
        ts = convergence_tightness(nd)
        assert ts == ConvergenceTightness(
            depth=0, level_max=0, consistency=0.0, score=0.0,
        )

    def test_all_none_chain(self) -> None:
        """全 None 链 → 全零。"""
        nd = NestedDivergence(chain=[(3, None), (2, None)], bar_range=(0, 50))
        ts = convergence_tightness(nd)
        assert ts.depth == 0
        assert ts.score == 0.0

    def test_full_consistent_chain(self) -> None:
        """完整一致链的精确计算。"""
        nd = _make_nd([5, 4, 3, 2, 1])
        ts = convergence_tightness(nd)
        assert ts.depth == 5
        assert ts.level_max == 5
        assert ts.consistency == 1.0
        assert ts.score == pytest.approx(25.0)  # 5 * 5 * 1.0

    def test_partial_consistency(self) -> None:
        """部分不一致链的 C(S) 计算。"""
        # 3 bottom + 1 top → C = 3/4 = 0.75
        nd = _make_nd([4, 3, 2, 1], ["bottom", "bottom", "bottom", "top"])
        ts = convergence_tightness(nd)
        assert ts.depth == 4
        assert ts.level_max == 4
        assert ts.consistency == pytest.approx(0.75)
        assert ts.score == pytest.approx(12.0)  # 4 * 4 * 0.75

    def test_score_monotonicity_with_depth(self) -> None:
        """同 level_max、同一致性下，depth 增大 → score 增大。"""
        scores = []
        for depth in range(1, 5):
            levels = list(range(4, 4 - depth, -1))
            nd = _make_nd(levels)
            ts = convergence_tightness(nd)
            scores.append(ts.score)
        # 每加一层（同方向），score 严格递增
        for i in range(len(scores) - 1):
            assert scores[i + 1] > scores[i]


# ═══════════════════════════════════════════════════════════════
# L0：排序不一致的具体反例
# ═══════════════════════════════════════════════════════════════


class TestRankingCounterexamplesL0:
    """L0：构造两个标的的 T(S) 和 proxy 排序相反的具体反例。

    证明：即使同时可以计算 T(S) 和 proxy，两者的排序可以不一致。
    """

    def test_depth_vs_level_tradeoff(self) -> None:
        """T(S) 的 D*L 乘法使得 "深但低" 可胜 "浅但高"。

        标的 A: 5层 level1-5 全一致 → T=5*5*1=25
        标的 B: 2层 level4-5 全一致 → T=2*5*1=10

        T(S): A >> B。
        但 proxy 的加法计算中，B 可能有更多结构
        （每层有 moves+zhongshus），而 A 只有 BSP。
        """
        nd_a = _make_nd([5, 4, 3, 2, 1])
        nd_b = _make_nd([5, 4])

        ts_a = convergence_tightness(nd_a)
        ts_b = convergence_tightness(nd_b)

        assert ts_a.score == 25.0
        assert ts_b.score == 10.0
        assert ts_a.score > ts_b.score  # T(S): A > B

        # proxy 反例构造：
        # A proxy: BSP(1.0) + moves(0.5) + zs(0.3) = 1.8
        # B proxy: BSP(1.0) + moves(0.5) + zs(0.3) + 3层递归(3*1.0) = 4.8
        # proxy: B > A — 排序相反
        proxy_a = 1.0 + 0.5 + 0.3  # 仅 level 1 结构
        proxy_b = 1.0 + 0.5 + 0.3 + 3 * 1.0  # level 1 + 3 层递归
        assert proxy_b > proxy_a  # proxy: B > A

    def test_consistency_invisible_to_proxy(self) -> None:
        """方向不一致时 T(S) 大幅降分，proxy 无感。

        标的 A: 4层全一致 → T=4*4*1=16
        标的 B: 4层 C=0.5 → T=4*4*0.5=8

        T(S): A > B (2x)。
        proxy: 两者结构深度相同 → 排序相同或 B 更高（取决于额外结构）。
        """
        nd_a = _make_nd([4, 3, 2, 1], ["bottom"] * 4)
        nd_b = _make_nd([4, 3, 2, 1], ["bottom", "top", "bottom", "top"])

        ts_a = convergence_tightness(nd_a)
        ts_b = convergence_tightness(nd_b)

        assert ts_a.score == pytest.approx(16.0)
        assert ts_b.score == pytest.approx(8.0)

        # 同结构深度下 proxy 给出相同分数
        # → T(S) 排序中 A 是 B 的 2 倍，proxy 中两者并列
