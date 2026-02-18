"""现金边信号消歧（CashSignalAnalysis）测试。

验证 026 号谱系需求：
  - 用纯资产子图校验现金边信号可信度
  - 四种信号类型：neutral / genuine_flow / metric_shift / mixed
  - 守恒约束在消歧后仍成立
"""

from __future__ import annotations

import pytest

from newchan.capital_flow import FlowDirection
from newchan.flow_relation import (
    CashSignalAnalysis,
    EdgeFlowInput,
    aggregate_vertex_flows,
    check_conservation,
    disambiguate_cash_signal,
)
from newchan.matrix_topology import AssetVertex

# ── 简写 ─────────────────────────────────────────────────

V = AssetVertex
D = FlowDirection


def _edge(
    a: AssetVertex, b: AssetVertex, direction: FlowDirection
) -> EdgeFlowInput:
    return EdgeFlowInput(vertex_a=a, vertex_b=b, direction=direction)


def _make_six_edges(
    *,
    eq_cash: FlowDirection = D.EQUILIBRIUM,
    re_cash: FlowDirection = D.EQUILIBRIUM,
    co_cash: FlowDirection = D.EQUILIBRIUM,
    eq_re: FlowDirection = D.EQUILIBRIUM,
    eq_co: FlowDirection = D.EQUILIBRIUM,
    re_co: FlowDirection = D.EQUILIBRIUM,
) -> list[EdgeFlowInput]:
    """构造 6 条边的快捷方式。

    参数名表示边的两个端点，方向语义：
      A_TO_B = 资本从第一个端点流向第二个端点
      B_TO_A = 资本从第二个端点流向第一个端点
    """
    return [
        _edge(V.EQUITY, V.CASH, eq_cash),
        _edge(V.REAL_ESTATE, V.CASH, re_cash),
        _edge(V.COMMODITY, V.CASH, co_cash),
        _edge(V.EQUITY, V.REAL_ESTATE, eq_re),
        _edge(V.EQUITY, V.COMMODITY, eq_co),
        _edge(V.REAL_ESTATE, V.COMMODITY, re_co),
    ]


# ── neutral：全均衡 ──────────────────────────────────────


class TestNeutral:
    """CASH net_flow == 0 → signal_type = "neutral"。"""

    def test_all_equilibrium(self) -> None:
        """六条边全部均衡 → neutral, confidence = 1.0。"""
        edges = _make_six_edges()
        result = disambiguate_cash_signal(edges)
        assert result.signal_type == "neutral"
        assert result.cash_net_flow == 0
        assert result.confidence == 1.0

    def test_cash_zero_but_assets_active(self) -> None:
        """CASH net_flow == 0，但纯资产边有方向 → 仍为 neutral。

        因为 neutral 只看 CASH 的净流量。
        """
        edges = _make_six_edges(
            eq_re=D.A_TO_B,
            eq_co=D.A_TO_B,
            re_co=D.A_TO_B,
        )
        result = disambiguate_cash_signal(edges)
        assert result.signal_type == "neutral"
        assert result.cash_net_flow == 0
        assert result.confidence == 1.0


# ── genuine_flow：真实避险/风险偏好 ──────────────────────


class TestGenuineFlow:
    """CASH |net_flow| > 0 且纯资产边有分化 → genuine_flow。"""

    def test_strong_sink_with_asset_divergence(self) -> None:
        """CASH 强共振汇 + 纯资产边有明显分化 → genuine_flow。

        场景：避险——资金涌入现金，同时纯资产之间也有流转分化。
        """
        edges = _make_six_edges(
            # 三条现金边全部流入 CASH
            eq_cash=D.A_TO_B,
            re_cash=D.A_TO_B,
            co_cash=D.A_TO_B,
            # 纯资产边有分化（不全是均衡）
            eq_re=D.A_TO_B,   # EQUITY → REAL_ESTATE
            eq_co=D.B_TO_A,   # COMMODITY → EQUITY
            re_co=D.A_TO_B,   # REAL_ESTATE → COMMODITY
        )
        result = disambiguate_cash_signal(edges)
        assert result.signal_type == "genuine_flow"
        assert result.cash_net_flow == 3
        assert result.asset_subgraph_variance > 0.0
        assert 0.0 < result.confidence <= 1.0

    def test_cash_outflow_with_asset_divergence(self) -> None:
        """CASH 强共振源 + 纯资产边有分化 → genuine_flow。

        场景：风险偏好——资金从现金涌出，纯资产间有流转分化。
        """
        edges = _make_six_edges(
            # 三条现金边全部流出 CASH
            eq_cash=D.B_TO_A,
            re_cash=D.B_TO_A,
            co_cash=D.B_TO_A,
            # 纯资产边有分化
            eq_re=D.A_TO_B,
            eq_co=D.A_TO_B,
            re_co=D.B_TO_A,
        )
        result = disambiguate_cash_signal(edges)
        assert result.signal_type == "genuine_flow"
        assert result.cash_net_flow == -3
        assert result.asset_subgraph_variance > 0.0


# ── metric_shift：度量基准变动 ───────────────────────────


class TestMetricShift:
    """CASH |net_flow| > 0 且纯资产边全部均衡 → metric_shift。"""

    def test_strong_sink_no_asset_divergence(self) -> None:
        """CASH 强共振汇 + 纯资产边全均衡 → metric_shift。

        场景：USD 升值，三类资产以美元计价同时下跌，
        但纯资产间比价未变——不是真实避险，是度量基准变动。
        """
        edges = _make_six_edges(
            eq_cash=D.A_TO_B,
            re_cash=D.A_TO_B,
            co_cash=D.A_TO_B,
            # 纯资产边全部均衡
        )
        result = disambiguate_cash_signal(edges)
        assert result.signal_type == "metric_shift"
        assert result.cash_net_flow == 3
        assert result.asset_subgraph_variance == 0.0
        assert 0.0 < result.confidence <= 1.0

    def test_weak_cash_signal_no_asset_divergence(self) -> None:
        """CASH net_flow = 2（弱共振）+ 纯资产边全均衡 → metric_shift。"""
        edges = _make_six_edges(
            eq_cash=D.A_TO_B,
            re_cash=D.A_TO_B,
            co_cash=D.EQUILIBRIUM,
            # 纯资产边全部均衡
        )
        result = disambiguate_cash_signal(edges)
        assert result.signal_type == "metric_shift"
        assert result.cash_net_flow == 2
        assert result.asset_subgraph_variance == 0.0


# ── mixed：中间情况 ──────────────────────────────────────


class TestMixed:
    """CASH |net_flow| > 0 且纯资产边部分分化 → mixed。"""

    def test_cash_signal_with_partial_asset_divergence(self) -> None:
        """CASH 有信号 + 纯资产边部分分化 → mixed。

        场景：现金流入，但纯资产间只有一条边有方向，
        分化不完全——信号有混合成分。
        """
        edges = _make_six_edges(
            eq_cash=D.A_TO_B,
            re_cash=D.A_TO_B,
            co_cash=D.A_TO_B,
            # 纯资产边只有一条有方向（部分分化）
            eq_re=D.A_TO_B,
            eq_co=D.EQUILIBRIUM,
            re_co=D.EQUILIBRIUM,
        )
        result = disambiguate_cash_signal(edges)
        assert result.signal_type == "mixed"
        assert result.cash_net_flow == 3
        assert result.asset_subgraph_variance > 0.0
        assert result.confidence == 0.5


# ── 守恒约束 ─────────────────────────────────────────────


class TestConservationStillHolds:
    """消歧是分析层——不改变底层拓扑计算，守恒仍成立。"""

    @pytest.mark.parametrize(
        "label,edges",
        [
            (
                "neutral",
                _make_six_edges(),
            ),
            (
                "genuine_flow",
                _make_six_edges(
                    eq_cash=D.A_TO_B,
                    re_cash=D.A_TO_B,
                    co_cash=D.A_TO_B,
                    eq_re=D.A_TO_B,
                    eq_co=D.B_TO_A,
                    re_co=D.A_TO_B,
                ),
            ),
            (
                "metric_shift",
                _make_six_edges(
                    eq_cash=D.A_TO_B,
                    re_cash=D.A_TO_B,
                    co_cash=D.A_TO_B,
                ),
            ),
            (
                "mixed",
                _make_six_edges(
                    eq_cash=D.A_TO_B,
                    re_cash=D.A_TO_B,
                    co_cash=D.A_TO_B,
                    eq_re=D.A_TO_B,
                ),
            ),
        ],
    )
    def test_conservation(self, label: str, edges: list[EdgeFlowInput]) -> None:
        """所有消歧场景下底层守恒约束 Σnet(V) = 0。"""
        states = aggregate_vertex_flows(edges)
        assert check_conservation(states), f"守恒失败：{label}"


# ── 返回值不可变性 ───────────────────────────────────────


class TestImmutability:
    """CashSignalAnalysis 是 frozen dataclass。"""

    def test_frozen(self) -> None:
        """尝试修改属性 → 报错。"""
        edges = _make_six_edges()
        result = disambiguate_cash_signal(edges)
        with pytest.raises(AttributeError):
            result.signal_type = "hacked"  # type: ignore[misc]


# ── 输入验证 ─────────────────────────────────────────────


class TestInputValidation:
    """消歧函数应对不合法输入报错。"""

    def test_wrong_edge_count(self) -> None:
        """非 6 条边 → 报错。"""
        with pytest.raises(ValueError, match="6"):
            disambiguate_cash_signal([])

    def test_duplicate_edge(self) -> None:
        """重复边 → 报错。"""
        edges = [
            _edge(V.EQUITY, V.CASH, D.A_TO_B),
            _edge(V.EQUITY, V.CASH, D.B_TO_A),  # 重复
            _edge(V.REAL_ESTATE, V.CASH, D.EQUILIBRIUM),
            _edge(V.COMMODITY, V.CASH, D.EQUILIBRIUM),
            _edge(V.EQUITY, V.REAL_ESTATE, D.EQUILIBRIUM),
            _edge(V.EQUITY, V.COMMODITY, D.EQUILIBRIUM),
        ]
        with pytest.raises(ValueError, match="重复"):
            disambiguate_cash_signal(edges)


# ── confidence 数值精确性 ─────────────────────────────────


class TestConfidenceValues:
    """confidence 的数值边界。"""

    def test_genuine_flow_max_confidence(self) -> None:
        """纯资产边全部有方向且方差最大 → confidence 接近 1.0。

        方差最大场景：三条边方向值为 +1, -1, +1（或其排列）。
        方差 = var([1, -1, 1]) = 2/3 * (2/3) ...
        """
        edges = _make_six_edges(
            eq_cash=D.A_TO_B,
            re_cash=D.A_TO_B,
            co_cash=D.A_TO_B,
            eq_re=D.A_TO_B,     # +1
            eq_co=D.B_TO_A,     # -1
            re_co=D.A_TO_B,     # +1
        )
        result = disambiguate_cash_signal(edges)
        assert result.signal_type == "genuine_flow"
        # confidence = variance / max_variance，max_variance 也是同样模式
        # 此处三条边方向值 [+1, -1, +1]，方差 = 8/9
        # 最大方差也是 8/9（取 +1/-1 的极端分化组合）
        # 所以 confidence 应该 = 1.0
        assert result.confidence == pytest.approx(1.0, abs=1e-9)

    def test_metric_shift_max_confidence(self) -> None:
        """纯资产边全部均衡 → metric_shift, confidence = 1.0。"""
        edges = _make_six_edges(
            eq_cash=D.A_TO_B,
            re_cash=D.A_TO_B,
            co_cash=D.A_TO_B,
        )
        result = disambiguate_cash_signal(edges)
        assert result.signal_type == "metric_shift"
        assert result.confidence == pytest.approx(1.0, abs=1e-9)

    def test_mixed_confidence_always_half(self) -> None:
        """mixed 情况下 confidence 恒为 0.5。"""
        edges = _make_six_edges(
            eq_cash=D.A_TO_B,
            re_cash=D.A_TO_B,
            co_cash=D.A_TO_B,
            eq_re=D.A_TO_B,
        )
        result = disambiguate_cash_signal(edges)
        assert result.signal_type == "mixed"
        assert result.confidence == 0.5
