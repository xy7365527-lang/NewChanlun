"""BSP 可视化测试 — b_plot / b_chart / overlay 桥接层的 BSP 渲染验证。

测试项：
  A) _build_bsp 桥接：BuySellPoint → 前端 JSON 映射
  B) b_plot BSP 渲染（mock matplotlib，验证调用参数）
  C) b_chart overlay 输出包含 BSP 标注
  D) 空 BSP 列表不报错
  E) 三类买卖点各自的渲染（type1/type2/type3）
  F) overlaps_with 标记在输出中保留
"""

from __future__ import annotations

from unittest.mock import MagicMock, patch

import pytest

from newchan.a_buysellpoint_v1 import BuySellPoint


# ── helpers ──


def _bsp(
    kind: str = "type1",
    side: str = "buy",
    seg_idx: int = 5,
    price: float = 100.0,
    bar_idx: int = 50,
    confirmed: bool = True,
    settled: bool = False,
    overlaps_with=None,
    level_id: int = 1,
) -> BuySellPoint:
    return BuySellPoint(
        kind=kind,
        side=side,
        level_id=level_id,
        seg_idx=seg_idx,
        move_seg_start=0,
        divergence_key=(0, 0, seg_idx) if kind == "type1" else None,
        center_zd=90.0,
        center_zg=100.0,
        center_seg_start=0,
        price=price,
        bar_idx=bar_idx,
        confirmed=confirmed,
        settled=settled,
        overlaps_with=overlaps_with,
    )


# ══════════ A) _build_bsp 桥接层 ══════════


class TestBuildBsp:
    """ab_bridge_newchan._build_bsp 将 BuySellPoint 映射到前端 JSON。"""

    def _call(self, buysellpoints, merged_to_raw=None, raw_index=None):
        import pandas as pd
        from newchan.ab_bridge_newchan import _build_bsp

        if merged_to_raw is None:
            # merged_to_raw: list[tuple[int, int]] — (raw_start, raw_end)
            merged_to_raw = [(i, i) for i in range(100)]
        if raw_index is None:
            raw_index = pd.date_range("2025-01-01", periods=100, freq="1min")
        return _build_bsp(buysellpoints, merged_to_raw, raw_index)

    def test_empty_list(self):
        result = self._call([])
        assert result == []

    def test_type1_buy(self):
        bp = _bsp(kind="type1", side="buy", price=95.0, bar_idx=10)
        result = self._call([bp])
        assert len(result) == 1
        r = result[0]
        assert r["kind"] == "type1"
        assert r["side"] == "buy"
        assert r["price"] == 95.0
        assert r["confirmed"] is True
        assert r["settled"] is False
        assert r["overlaps_with"] is None
        assert "time" in r

    def test_type1_sell(self):
        bp = _bsp(kind="type1", side="sell", price=120.0, bar_idx=20)
        result = self._call([bp])
        assert result[0]["kind"] == "type1"
        assert result[0]["side"] == "sell"
        assert result[0]["price"] == 120.0

    def test_type2_buy(self):
        bp = _bsp(kind="type2", side="buy", price=97.0, bar_idx=15)
        result = self._call([bp])
        assert result[0]["kind"] == "type2"
        assert result[0]["side"] == "buy"

    def test_type2_sell(self):
        bp = _bsp(kind="type2", side="sell", price=118.0, bar_idx=25)
        result = self._call([bp])
        assert result[0]["kind"] == "type2"
        assert result[0]["side"] == "sell"

    def test_type3_buy(self):
        bp = _bsp(kind="type3", side="buy", price=101.0, bar_idx=30)
        result = self._call([bp])
        assert result[0]["kind"] == "type3"
        assert result[0]["side"] == "buy"

    def test_type3_sell(self):
        bp = _bsp(kind="type3", side="sell", price=89.0, bar_idx=35)
        result = self._call([bp])
        assert result[0]["kind"] == "type3"
        assert result[0]["side"] == "sell"

    def test_overlaps_with_preserved(self):
        bp = _bsp(kind="type2", side="buy", overlaps_with="type3")
        result = self._call([bp])
        assert result[0]["overlaps_with"] == "type3"

    def test_multiple_bsp(self):
        bps = [
            _bsp(kind="type1", side="buy", bar_idx=10),
            _bsp(kind="type2", side="buy", bar_idx=20),
            _bsp(kind="type3", side="sell", bar_idx=30),
        ]
        result = self._call(bps)
        assert len(result) == 3
        kinds = [r["kind"] for r in result]
        assert kinds == ["type1", "type2", "type3"]

    def test_all_required_fields_present(self):
        bp = _bsp()
        result = self._call([bp])
        r = result[0]
        required = {"kind", "side", "level_id", "seg_idx", "price", "time",
                     "confirmed", "settled", "overlaps_with"}
        assert required.issubset(r.keys())


# ══════════ B) b_plot BSP 渲染 ══════════


class TestPlotBspRendering:
    """b_plot 的 BSP 渲染测试。

    bsp-plot 工位正在添加渲染函数。测试基于预期接口：
    plot_bsp(ax, buysellpoints) → 在 ax 上绘制 BSP 标记。
    如果函数尚未存在，测试标记为 xfail。
    """

    @staticmethod
    def _try_import_plot_bsp():
        try:
            from newchan.b_plot import plot_bsp
            return plot_bsp
        except ImportError:
            pytest.skip("b_plot.plot_bsp 尚未实现（bsp-plot 工位进行中）")

    def test_empty_bsp_no_error(self):
        plot_bsp = self._try_import_plot_bsp()
        mock_ax = MagicMock()
        plot_bsp(mock_ax, [])
        # 空列表不应报错，也不应调用绑定绘图方法
        # （允许实现调用 ax 方法但不 crash）

    def test_type1_buy_marker(self):
        plot_bsp = self._try_import_plot_bsp()
        mock_ax = MagicMock()
        bps = [_bsp(kind="type1", side="buy", price=95.0, bar_idx=10)]
        plot_bsp(mock_ax, bps)
        # 至少调用了一次绘图方法（scatter/annotate/plot）
        assert (
            mock_ax.scatter.called
            or mock_ax.annotate.called
            or mock_ax.plot.called
        ), "plot_bsp 应在 ax 上绑定至少一个绘图调用"

    def test_type1_sell_marker(self):
        plot_bsp = self._try_import_plot_bsp()
        mock_ax = MagicMock()
        bps = [_bsp(kind="type1", side="sell", price=120.0, bar_idx=20)]
        plot_bsp(mock_ax, bps)
        assert (
            mock_ax.scatter.called
            or mock_ax.annotate.called
            or mock_ax.plot.called
        )

    def test_type2_marker(self):
        plot_bsp = self._try_import_plot_bsp()
        mock_ax = MagicMock()
        bps = [_bsp(kind="type2", side="buy", price=97.0)]
        plot_bsp(mock_ax, bps)
        assert (
            mock_ax.scatter.called
            or mock_ax.annotate.called
            or mock_ax.plot.called
        )

    def test_type3_marker(self):
        plot_bsp = self._try_import_plot_bsp()
        mock_ax = MagicMock()
        bps = [_bsp(kind="type3", side="sell", price=89.0)]
        plot_bsp(mock_ax, bps)
        assert (
            mock_ax.scatter.called
            or mock_ax.annotate.called
            or mock_ax.plot.called
        )

    def test_mixed_types(self):
        plot_bsp = self._try_import_plot_bsp()
        mock_ax = MagicMock()
        bps = [
            _bsp(kind="type1", side="buy", bar_idx=10),
            _bsp(kind="type2", side="buy", bar_idx=20),
            _bsp(kind="type3", side="sell", bar_idx=30),
        ]
        plot_bsp(mock_ax, bps)
        total_calls = (
            mock_ax.scatter.call_count
            + mock_ax.annotate.call_count
            + mock_ax.plot.call_count
        )
        assert total_calls >= 3, "三个 BSP 应产生至少三次绘图调用"


# ══════════ C) b_chart overlay 输出包含 BSP ══════════


class TestChartOverlayBsp:
    """b_chart 通过 overlay API 获取 BSP 数据。

    验证 overlay schema 中 bsp 字段的结构正确性。
    """

    def test_empty_overlay_has_bsp_field(self):
        from newchan.ab_bridge_newchan import _empty_overlay

        result = _empty_overlay("TEST", "1m", "full", 12, 26, 9)
        assert "bsp" in result
        assert result["bsp"] == []

    def test_overlay_bsp_schema(self):
        """验证 _build_bsp 输出的每个条目包含 b_chart JS 所需的字段。"""
        import pandas as pd
        from newchan.ab_bridge_newchan import _build_bsp

        bps = [
            _bsp(kind="type1", side="buy", price=95.0, bar_idx=10),
            _bsp(kind="type3", side="sell", price=89.0, bar_idx=30),
        ]
        m2r = [(i, i) for i in range(50)]
        raw_idx = pd.date_range("2025-01-01", periods=50, freq="1min")
        result = _build_bsp(bps, m2r, raw_idx)

        for entry in result:
            # b_chart JS 渲染所需的最小字段集
            assert "kind" in entry
            assert "side" in entry
            assert "price" in entry
            assert "time" in entry
            assert entry["kind"] in ("type1", "type2", "type3")
            assert entry["side"] in ("buy", "sell")
            assert isinstance(entry["price"], (int, float))


# ══════════ D) 空 BSP 列表不报错 ══════════


class TestEmptyBspSafety:
    """空 BSP 列表在所有层级都不应报错。"""

    def test_build_bsp_empty(self):
        import pandas as pd
        from newchan.ab_bridge_newchan import _build_bsp

        result = _build_bsp([], [(i, i) for i in range(10)],
                            pd.date_range("2025-01-01", periods=10, freq="1min"))
        assert result == []

    def test_empty_overlay_bsp(self):
        from newchan.ab_bridge_newchan import _empty_overlay

        result = _empty_overlay("X", "1d", "full", 12, 26, 9)
        assert result["bsp"] == []

    def test_plot_bsp_empty(self):
        """b_plot.plot_bsp([]) 不应报错。"""
        try:
            from newchan.b_plot import plot_bsp
        except ImportError:
            pytest.skip("b_plot.plot_bsp 尚未实现")
        mock_ax = MagicMock()
        plot_bsp(mock_ax, [])  # 不应抛异常


# ══════════ E) 三类买卖点各自的渲染 ══════════


class TestPerTypeBspRendering:
    """验证三类买卖点在 overlay JSON 中的区分。"""

    @staticmethod
    def _build(bps):
        import pandas as pd
        from newchan.ab_bridge_newchan import _build_bsp

        m2r = [(i, i) for i in range(60)]
        raw_idx = pd.date_range("2025-01-01", periods=60, freq="1min")
        return _build_bsp(bps, m2r, raw_idx)

    def test_type1_buy_json(self):
        result = self._build([_bsp(kind="type1", side="buy", price=95.0, bar_idx=5)])
        assert result[0]["kind"] == "type1"
        assert result[0]["side"] == "buy"
        assert result[0]["price"] == 95.0

    def test_type1_sell_json(self):
        result = self._build([_bsp(kind="type1", side="sell", price=120.0, bar_idx=8)])
        assert result[0]["kind"] == "type1"
        assert result[0]["side"] == "sell"

    def test_type2_buy_json(self):
        result = self._build([_bsp(kind="type2", side="buy", price=97.0, bar_idx=12)])
        assert result[0]["kind"] == "type2"
        assert result[0]["side"] == "buy"

    def test_type2_sell_json(self):
        result = self._build([_bsp(kind="type2", side="sell", price=118.0, bar_idx=15)])
        assert result[0]["kind"] == "type2"
        assert result[0]["side"] == "sell"

    def test_type3_buy_json(self):
        result = self._build([_bsp(kind="type3", side="buy", price=101.0, bar_idx=20)])
        assert result[0]["kind"] == "type3"
        assert result[0]["side"] == "buy"

    def test_type3_sell_json(self):
        result = self._build([_bsp(kind="type3", side="sell", price=89.0, bar_idx=25)])
        assert result[0]["kind"] == "type3"
        assert result[0]["side"] == "sell"


# ══════════ F) overlaps_with 标记 ══════════


class TestOverlapMarkerVisualization:
    """2B+3B 重合标记在可视化输出中保留。"""

    @staticmethod
    def _build(bps):
        import pandas as pd
        from newchan.ab_bridge_newchan import _build_bsp

        m2r = [(i, i) for i in range(60)]
        raw_idx = pd.date_range("2025-01-01", periods=60, freq="1min")
        return _build_bsp(bps, m2r, raw_idx)

    def test_overlap_type3_on_type2(self):
        bp = _bsp(kind="type2", side="buy", overlaps_with="type3", bar_idx=10)
        result = self._build([bp])
        assert result[0]["overlaps_with"] == "type3"

    def test_overlap_type2_on_type3(self):
        bp = _bsp(kind="type3", side="buy", overlaps_with="type2", bar_idx=10)
        result = self._build([bp])
        assert result[0]["overlaps_with"] == "type2"

    def test_no_overlap(self):
        bp = _bsp(kind="type1", side="buy", bar_idx=10)
        result = self._build([bp])
        assert result[0]["overlaps_with"] is None
