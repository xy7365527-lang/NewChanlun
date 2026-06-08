"""跨国 K4 管线测试（254号三层结构 + 528/529号折叠通道）。

纯逻辑测试，不依赖 analysis/data_cache 真实数据文件（真实数据验证见
analysis/cross_national_k4_validation.py，属 L2）。用合成 EdgeReading 测装配逻辑。
"""

from newchan.topology.config_space import Configuration, WalkDirection
from newchan.topology.cross_national_pipeline import (
    ECONOMY_SPECS,
    GLOBAL_FOLD_FILES,
    CrossNationalCPathResult,
    CurrencyLayerResult,
    EconomyK4Result,
    EdgeReading,
    _direction_to_sigma,
)
from newchan.topology.graph import Vertex


def _reading(name: str, sigma: WalkDirection, kind: str, direction: str,
             settled: bool = True, price: float = 1.0) -> EdgeReading:
    return EdgeReading(
        name=name, bar_count=100, date_range=("2024-01-01", "2026-01-01"),
        sigma=sigma, move_kind=kind, move_direction=direction,
        move_settled=settled, max_level=2, last_price=price,
    )


class TestEconomySpecs:
    def test_four_economies(self):
        assert set(ECONOMY_SPECS) == {"US", "EU", "JP", "CN"}

    def test_us_has_all_four_vertices(self):
        """美国是唯一四顶点齐全的经济体。"""
        spec = ECONOMY_SPECS["US"]
        assert all(spec.vertex_files[v] for v in Vertex)
        assert spec.fx_to_usd is None  # 美元截面本身

    def test_us_money_is_uup(self):
        assert ECONOMY_SPECS["US"].vertex_files[Vertex.M] == "uup_1m_full.json"

    def test_china_domestic_is_gap(self):
        """中国本土 P/C/R 在 IB/databento 均无源（已验证缺口），仅离岸 FX 可用。"""
        spec = ECONOMY_SPECS["CN"]
        assert spec.vertex_files[Vertex.P] is None
        assert spec.vertex_files[Vertex.C] is None
        assert spec.vertex_files[Vertex.R] is None
        assert spec.fx_to_usd == "usdcnh_1h_databento.json"

    def test_eu_jp_have_equity_only(self):
        """EU/JP 有股指(P)，但 C/R 本币源缺口（254号 OQ2 不动产本地化）。"""
        for eco in ("EU", "JP"):
            spec = ECONOMY_SPECS[eco]
            assert spec.vertex_files[Vertex.P] is not None
            assert spec.vertex_files[Vertex.C] is None
            assert spec.vertex_files[Vertex.R] is None
            assert spec.fx_to_usd is not None

    def test_global_folds_au_oil(self):
        """Au/Oil 全局共享折叠通道（292号），不在各经济体重复。"""
        assert set(GLOBAL_FOLD_FILES) == {"Au", "Oil"}
        assert "gc" in GLOBAL_FOLD_FILES["Au"]
        assert "cl" in GLOBAL_FOLD_FILES["Oil"]


class TestDirectionToSigma:
    def test_trend_up(self):
        assert _direction_to_sigma("up", "trend") is WalkDirection.UP

    def test_trend_down(self):
        assert _direction_to_sigma("down", "trend") is WalkDirection.DOWN

    def test_consolidation_flat(self):
        """盘整 → FLAT，无论 break_direction。"""
        assert _direction_to_sigma("up", "consolidation") is WalkDirection.FLAT
        assert _direction_to_sigma("down", "consolidation") is WalkDirection.FLAT

    def test_none_flat(self):
        assert _direction_to_sigma("none", "none") is WalkDirection.FLAT


class TestEconomyK4Result:
    def test_complete_config(self):
        """三独立边齐全 → 完整 Configuration。"""
        sigma = {
            Vertex.P: WalkDirection.UP,
            Vertex.C: WalkDirection.FLAT,
            Vertex.R: WalkDirection.DOWN,
        }
        cfg = Configuration(sigma_p=sigma[Vertex.P], sigma_c=sigma[Vertex.C], sigma_r=sigma[Vertex.R])
        res = EconomyK4Result("US", "USD", edges={}, missing=(), config=cfg, partial_sigma=sigma)
        assert res.is_complete
        assert res.config.label == "(+,0,-)"

    def test_partial_no_config(self):
        """缺顶点 → config 为 None（部分配置，不伪造）。"""
        res = EconomyK4Result(
            "EU", "EUR", edges={},
            missing=(Vertex.C, Vertex.R),
            config=None,
            partial_sigma={Vertex.P: WalkDirection.UP},
        )
        assert not res.is_complete
        assert res.config is None


class TestCurrencyLayer:
    def test_break_signal_two_same_direction(self):
        """≥2 条货币边同向已结算趋势 → 结算尺断裂候选（254号定理2）。"""
        layer = CurrencyLayerResult(fx_readings={
            "EU": _reading("FX:EUR/USD", WalkDirection.UP, "trend", "up"),
            "JP": _reading("FX:JPY/USD", WalkDirection.UP, "trend", "up"),
        })
        assert layer.synchronized_break_signal()

    def test_no_break_mixed_direction(self):
        layer = CurrencyLayerResult(fx_readings={
            "EU": _reading("FX:EUR/USD", WalkDirection.UP, "trend", "up"),
            "JP": _reading("FX:JPY/USD", WalkDirection.DOWN, "trend", "down"),
        })
        assert not layer.synchronized_break_signal()

    def test_no_break_single_trend(self):
        """单边事件无法区分分子/分母（254号），不触发。"""
        layer = CurrencyLayerResult(fx_readings={
            "EU": _reading("FX:EUR/USD", WalkDirection.UP, "trend", "up"),
            "JP": _reading("FX:JPY/USD", WalkDirection.FLAT, "consolidation", "down"),
        })
        assert not layer.synchronized_break_signal()


class TestCrossNationalCPath:
    def _cpath(self, omega_dir: str) -> CrossNationalCPathResult:
        return CrossNationalCPathResult(
            au_reading=_reading("Au", WalkDirection.UP, "trend", "up", price=2000.0),
            oil_reading=_reading("Oil", WalkDirection.FLAT, "consolidation", "down", price=80.0),
            omega_last=25.0,
            omega_direction=omega_dir,
            economy_c_edges={},
        )

    def test_credit_contraction_omega_up(self):
        """ω↑（金强于油）→ 信用收缩（482号）。"""
        assert "信用收缩" in self._cpath("up").credit_signal

    def test_credit_expansion_omega_down(self):
        assert "信用扩张" in self._cpath("down").credit_signal

    def test_credit_flat(self):
        assert "不明" in self._cpath("flat").credit_signal
