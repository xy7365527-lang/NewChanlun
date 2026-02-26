"""嵌套背驰端点 + 管线集成测试。

测试三层：
  A. run_nested_search() 单元测试（补充 test_nested_pipeline.py 未覆盖的边界）
  B. /api/nested_divergence 端点测试（mock，nde-api 并行开发中）
  C. build_overlay_newchan() nested_divergence 集成（mock，nde-bridge 并行开发中）

概念溯源: [旧缠论] 第27课 区间套（精确大转折点寻找程序定理）
"""

from __future__ import annotations

import json
from datetime import datetime, timedelta
from unittest.mock import MagicMock, patch

import pandas as pd
import pytest

from newchan.a_nested_divergence import NestedDivergence, nested_divergence_search
from newchan.nested_pipeline import run_nested_search
from newchan.types import Bar


# ── 测试数据工厂 ──────────────────────────────


def _make_bars(n: int, base_price: float = 100.0, amplitude: float = 3.0) -> list[Bar]:
    """生成 n 根模拟 K 线（锯齿波动）。"""
    bars: list[Bar] = []
    t0 = datetime(2025, 1, 1, 9, 30)
    for i in range(n):
        cycle = i % 10
        if cycle < 5:
            offset = cycle * amplitude
        else:
            offset = (10 - cycle) * amplitude
        c = base_price + offset + i * 0.02
        bars.append(Bar(
            ts=t0 + timedelta(minutes=i),
            open=c - 0.2,
            high=c + amplitude * 0.5,
            low=c - amplitude * 0.5,
            close=c,
            volume=1000.0,
        ))
    return bars


def _make_flat_bars(n: int, price: float = 100.0) -> list[Bar]:
    """生成 n 根几乎无波动的 K 线（不应产生任何结构）。"""
    bars: list[Bar] = []
    t0 = datetime(2025, 1, 1, 9, 30)
    for i in range(n):
        bars.append(Bar(
            ts=t0 + timedelta(minutes=i),
            open=price,
            high=price + 0.01,
            low=price - 0.01,
            close=price,
            volume=100.0,
        ))
    return bars


# ═══════════════════════════════════════════════
# A. run_nested_search 单元测试（补充边界）
# ═══════════════════════════════════════════════


class TestRunNestedSearchEdgeCases:
    """run_nested_search 边界情况。"""

    def test_single_bar(self):
        """1 根 bar → ([], None)。"""
        bars = _make_bars(1)
        results, snap = run_nested_search(bars)
        assert results == []
        assert snap is None

    def test_exactly_three_bars(self):
        """恰好 3 根 bar → snap 非 None，但可能无背驰。"""
        bars = _make_bars(3)
        results, snap = run_nested_search(bars)
        assert isinstance(results, list)
        # 3 根 bar 不足以产生递归层级
        assert snap is not None

    def test_flat_bars_no_divergence(self):
        """无波动数据 → 无嵌套背驰。"""
        bars = _make_flat_bars(100)
        results, snap = run_nested_search(bars)
        assert results == []

    def test_results_are_nested_divergence_type(self):
        """返回值中每个元素都是 NestedDivergence。"""
        bars = _make_bars(200, amplitude=5.0)
        results, snap = run_nested_search(bars)
        for nd in results:
            assert isinstance(nd, NestedDivergence)

    def test_chain_levels_descending(self):
        """每条嵌套链中 level_id 严格递减。"""
        bars = _make_bars(300, amplitude=5.0)
        results, snap = run_nested_search(bars, max_levels=4)
        for nd in results:
            levels = [lv for lv, _ in nd.chain]
            for i in range(1, len(levels)):
                assert levels[i] < levels[i - 1], (
                    f"level_id 应递减: {levels}"
                )

    def test_bar_range_non_negative(self):
        """bar_range 始终非负。"""
        bars = _make_bars(200, amplitude=5.0)
        results, snap = run_nested_search(bars)
        for nd in results:
            assert nd.bar_range[0] >= 0
            assert nd.bar_range[1] >= 0

    def test_max_levels_respected(self):
        """max_levels 参数限制递归深度。"""
        bars = _make_bars(200)
        _, snap = run_nested_search(bars, max_levels=2)
        assert snap is not None
        assert len(snap.recursive_snapshots) <= 1  # max_levels=2 → 最多 1 个递归层

    def test_stroke_mode_parameter(self):
        """不同 stroke_mode 不报错。"""
        bars = _make_bars(50)
        results_wide, _ = run_nested_search(bars, stroke_mode="wide")
        results_strict, _ = run_nested_search(bars, stroke_mode="strict")
        assert isinstance(results_wide, list)
        assert isinstance(results_strict, list)

    def test_custom_macd_params(self):
        """自定义 MACD 参数不报错。"""
        bars = _make_bars(60)
        results, snap = run_nested_search(
            bars, macd_fast=8, macd_slow=17, macd_signal=5,
        )
        assert isinstance(results, list)
        assert snap is not None

    def test_idempotent(self):
        """同一输入两次调用结果相同。"""
        bars = _make_bars(100)
        r1, s1 = run_nested_search(bars)
        r2, s2 = run_nested_search(bars)
        assert len(r1) == len(r2)
        for nd1, nd2 in zip(r1, r2):
            assert nd1.bar_range == nd2.bar_range
            assert len(nd1.chain) == len(nd2.chain)


# ═══════════════════════════════════════════════
# B. /api/nested_divergence 端点测试（mock）
# ═══════════════════════════════════════════════


def _make_query(params: dict | None = None):
    """构造 bottle.request.query mock。"""
    store = params or {}
    mock = MagicMock()
    mock.get = lambda key, default="": store.get(key, default)
    return mock


def _make_ohlcv_df(n: int = 20) -> pd.DataFrame:
    """构造小 OHLCV DataFrame。"""
    idx = pd.date_range("2025-01-01", periods=n, freq="1min")
    return pd.DataFrame(
        {
            "open": [100 + i * 0.1 for i in range(n)],
            "high": [102 + i * 0.1 for i in range(n)],
            "low": [98 + i * 0.1 for i in range(n)],
            "close": [101 + i * 0.1 for i in range(n)],
            "volume": [1000] * n,
        },
        index=idx,
    )


class TestNestedDivergenceEndpoint:
    """端点路由测试（nde-api 并行开发中，用 mock 验证预期行为）。"""

    def test_endpoint_function_importable(self):
        """如果端点已注册，应能从 server 模块导入。

        nde-api 尚未完成时此测试 skip。
        """
        import newchan.server as srv
        fn = getattr(srv, "api_nested_divergence", None)
        if fn is None:
            pytest.skip("api_nested_divergence 尚未实现（nde-api 并行开发中）")
        assert callable(fn)

    def test_missing_symbol_returns_error(self):
        """缺少 symbol 参数 → 400 错误。"""
        import newchan.server as srv
        fn = getattr(srv, "api_nested_divergence", None)
        if fn is None:
            pytest.skip("api_nested_divergence 尚未实现")

        with patch.object(srv, "request") as mock_req:
            mock_req.query = _make_query({"symbol": ""})
            result = json.loads(fn())
            assert "error" in result

    def test_no_cache_returns_error(self):
        """无缓存数据 → 404 错误。"""
        import newchan.server as srv
        fn = getattr(srv, "api_nested_divergence", None)
        if fn is None:
            pytest.skip("api_nested_divergence 尚未实现")

        with patch.object(srv, "request") as mock_req, \
             patch.object(srv, "load_df", return_value=None):
            mock_req.query = _make_query({"symbol": "CL", "interval": "1min"})
            result = json.loads(fn())
            assert "error" in result

    def test_success_returns_results(self):
        """正常请求 → 返回嵌套背驰结果。"""
        import newchan.server as srv
        fn = getattr(srv, "api_nested_divergence", None)
        if fn is None:
            pytest.skip("api_nested_divergence 尚未实现")

        df = _make_ohlcv_df(50)
        from newchan.a_divergence import Divergence
        mock_div = Divergence(
            kind="trend", direction="top", level_id=2,
            seg_a_start=1, seg_a_end=2, seg_c_start=4, seg_c_end=5,
            center_idx=0, force_a=20.0, force_c=5.0, confirmed=False,
        )
        mock_nd = NestedDivergence(
            chain=[(2, mock_div)],
            bar_range=(10, 50),
        )

        with patch.object(srv, "request") as mock_req, \
             patch.object(srv, "load_df", return_value=df), \
             patch.object(srv, "resample_ohlc", return_value=df), \
             patch("newchan.nested_pipeline.run_nested_search",
                   return_value=([mock_nd], MagicMock())):
            mock_req.query = _make_query({
                "symbol": "CL", "interval": "1min", "tf": "1m",
            })
            result = json.loads(fn())
            assert "data" in result
            assert result["count"] == 1
            assert result["data"][0]["bar_range"] == [10, 50]
            chain_entry = result["data"][0]["chain"][0]
            assert chain_entry["level_id"] == 2
            assert chain_entry["divergence"]["kind"] == "trend"
            assert chain_entry["divergence"]["force_a"] > chain_entry["divergence"]["force_c"]


# ═══════════════════════════════════════════════
# C. build_overlay_newchan nested_divergence 集成
# ═══════════════════════════════════════════════


class TestOverlayNestedDivergenceIntegration:
    """build_overlay_newchan 的 nested_divergence 字段集成测试。

    nde-bridge 并行开发中，验证集成后的预期行为。
    """

    def _make_zigzag_df(self, n: int = 40) -> pd.DataFrame:
        """构造锯齿 OHLC。"""
        dates = pd.date_range("2025-01-01", periods=n, freq="h")
        highs, lows = [], []
        for i in range(n):
            cycle = i % 20
            if cycle < 10:
                h = 100 - cycle * 3
                l = h - 5
            else:
                h = 70 + (cycle - 10) * 3
                l = h - 5
            highs.append(float(h))
            lows.append(float(l))
        opens = [(h + l) / 2 for h, l in zip(highs, lows)]
        closes = [(h + l) / 2 for h, l in zip(highs, lows)]
        return pd.DataFrame(
            {"open": opens, "high": highs, "low": lows, "close": closes},
            index=dates,
        )

    def test_overlay_has_nested_divergence_key(self):
        """集成后 overlay 输出应包含 nested_divergence 键。

        nde-bridge 尚未完成时 skip。
        """
        from newchan.ab_bridge_newchan import build_overlay_newchan

        df = self._make_zigzag_df()
        result = build_overlay_newchan(df, symbol="TEST", tf="1h")
        if "nested_divergence" not in result:
            pytest.skip("nested_divergence 集成尚未完成（nde-bridge 并行开发中）")
        assert isinstance(result["nested_divergence"], list)

    def test_overlay_nested_divergence_structure(self):
        """nested_divergence 列表中每项应有 chain 和 bar_range。"""
        from newchan.ab_bridge_newchan import build_overlay_newchan

        df = self._make_zigzag_df()
        result = build_overlay_newchan(df, symbol="TEST", tf="1h")
        if "nested_divergence" not in result:
            pytest.skip("nested_divergence 集成尚未完成（nde-bridge 并行开发中）")
        for nd in result["nested_divergence"]:
            assert "chain" in nd
            assert "bar_range" in nd

    def test_overlay_empty_df_no_nested_divergence(self):
        """空 DataFrame → nested_divergence 为空列表（如果字段存在）。"""
        from newchan.ab_bridge_newchan import build_overlay_newchan

        df = pd.DataFrame(columns=["open", "high", "low", "close"])
        result = build_overlay_newchan(df)
        if "nested_divergence" not in result:
            pytest.skip("nested_divergence 集成尚未完成（nde-bridge 并行开发中）")
        assert result["nested_divergence"] == []


# ═══════════════════════════════════════════════
# D. NestedDivergence 数据结构验证
# ═══════════════════════════════════════════════


class TestNestedDivergenceDataclass:
    """NestedDivergence frozen dataclass 的基本属性。"""

    def test_frozen(self):
        """NestedDivergence 是不可变的。"""
        nd = NestedDivergence(chain=[], bar_range=(0, 10))
        with pytest.raises(AttributeError):
            nd.bar_range = (5, 15)  # type: ignore[misc]

    def test_equality(self):
        """相同字段 → 相等。"""
        nd1 = NestedDivergence(chain=[(2, None)], bar_range=(10, 50))
        nd2 = NestedDivergence(chain=[(2, None)], bar_range=(10, 50))
        assert nd1 == nd2

    def test_inequality(self):
        """不同字段 → 不等。"""
        nd1 = NestedDivergence(chain=[], bar_range=(0, 10))
        nd2 = NestedDivergence(chain=[], bar_range=(0, 20))
        assert nd1 != nd2
