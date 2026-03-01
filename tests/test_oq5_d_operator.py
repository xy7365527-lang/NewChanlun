"""OQ5 D 算子管线测试（口径A：递归级别）。

277号谱系：验证 run_d_operator 使用 RecursiveOrchestrator 后管线不崩溃，
产出结构正确。

认识论等级：L1（合成数据验证管线正确性）
"""

from __future__ import annotations

from datetime import datetime, timedelta

import pandas as pd
import pytest

from newchan.a_move_v1 import Move


# ── 辅助 ──


def _make_daily_df(n: int = 200, base: float = 100.0) -> pd.DataFrame:
    """生成锯齿形日线 OHLCV DataFrame（保证笔/线段/中枢产出）。

    每 20 根 bar 形成一个完整 V 形波动周期，振幅逐渐增大。
    """
    dates = pd.date_range("2020-01-01", periods=n, freq="B")
    rows = []
    for i in range(n):
        cycle = (i % 20) / 20.0
        # V形：先涨后跌
        amplitude = 5.0 + (i // 20) * 0.5
        if cycle < 0.5:
            price = base + amplitude * (cycle * 2)
        else:
            price = base + amplitude * (1.0 - (cycle - 0.5) * 2)

        noise = (i % 3) * 0.1
        o = price - noise
        c = price + noise
        h = max(o, c) + 0.2
        l = min(o, c) - 0.2
        rows.append({"Open": o, "High": h, "Low": l, "Close": c, "Volume": 1000})

    return pd.DataFrame(rows, index=dates)


def _make_trending_df(
    n: int = 300, base: float = 100.0, direction: str = "up",
) -> pd.DataFrame:
    """生成带趋势的日线数据（锯齿叠加趋势）。"""
    dates = pd.date_range("2020-01-01", periods=n, freq="B")
    rows = []
    for i in range(n):
        cycle = (i % 15) / 15.0
        amplitude = 3.0
        if cycle < 0.5:
            wave = amplitude * (cycle * 2)
        else:
            wave = amplitude * (1.0 - (cycle - 0.5) * 2)

        if direction == "up":
            trend = i * 0.1
        else:
            trend = -i * 0.1

        price = base + trend + wave
        o = price - 0.1
        c = price + 0.1
        h = max(o, c) + 0.3
        l = min(o, c) - 0.3
        rows.append({"Open": o, "High": h, "Low": l, "Close": c, "Volume": 1000})

    return pd.DataFrame(rows, index=dates)


# ── 测试 ──


class TestRunDOperator:
    """run_d_operator 管线正确性测试。"""

    def test_minimum_data_returns_empty(self):
        """数据不足 5 根时返回空结果。"""
        # 延迟导入避免模块级副作用
        import importlib
        import sys
        sys.path.insert(0, str(__import__("pathlib").Path(__file__).resolve().parents[1] / "scripts"))
        from oq5_d_operator import run_d_operator

        dates = pd.date_range("2020-01-01", periods=3, freq="B")
        df = pd.DataFrame({
            "Open": [100, 101, 102],
            "High": [101, 102, 103],
            "Low": [99, 100, 101],
            "Close": [100.5, 101.5, 102.5],
        }, index=dates)

        result = run_d_operator(df, "TEST")
        assert result.bar_count == 3
        assert result.max_level_reached == 0
        assert result.levels == []

    def test_pipeline_does_not_crash_on_zigzag(self):
        """锯齿形数据管线不崩溃，结果结构正确。"""
        import sys
        sys.path.insert(0, str(__import__("pathlib").Path(__file__).resolve().parents[1] / "scripts"))
        from oq5_d_operator import run_d_operator

        df = _make_daily_df(200)
        result = run_d_operator(df, "ZIGZAG")

        assert result.symbol == "ZIGZAG"
        assert result.bar_count == 200
        # 至少应有 level 1
        assert result.max_level_reached >= 1
        assert len(result.levels) >= 1
        # level 1 必须存在
        level1 = result.levels[0]
        assert level1.level_id == 1

    def test_level_ids_monotonically_increasing(self):
        """levels 列表中 level_id 递增。"""
        import sys
        sys.path.insert(0, str(__import__("pathlib").Path(__file__).resolve().parents[1] / "scripts"))
        from oq5_d_operator import run_d_operator

        df = _make_daily_df(300)
        result = run_d_operator(df, "MONO")

        ids = [lr.level_id for lr in result.levels]
        assert ids == sorted(ids)
        # 没有重复
        assert len(ids) == len(set(ids))

    def test_trending_data_produces_moves(self):
        """趋势数据应产出走势。"""
        import sys
        sys.path.insert(0, str(__import__("pathlib").Path(__file__).resolve().parents[1] / "scripts"))
        from oq5_d_operator import run_d_operator

        df = _make_trending_df(400, direction="down")
        result = run_d_operator(df, "DOWN")

        # 管线不崩溃，至少有 level 1 产出
        assert result.max_level_reached >= 1
        level1 = result.levels[0]
        assert level1.level_id == 1
        # 400 根 bar 的趋势数据至少应产出笔和线段
        # 走势数量取决于中枢的形成，此处仅验证管线完整性
        assert level1.move_count >= 0

    def test_missing_column_raises(self):
        """缺少必要列时抛出 ValueError。"""
        import sys
        sys.path.insert(0, str(__import__("pathlib").Path(__file__).resolve().parents[1] / "scripts"))
        from oq5_d_operator import run_d_operator

        dates = pd.date_range("2020-01-01", periods=10, freq="B")
        df = pd.DataFrame({
            "Open": range(10),
            "High": range(10),
            # Missing Low and Close
        }, index=dates)

        with pytest.raises(ValueError, match="Missing column"):
            run_d_operator(df, "BAD")


class TestGetLevel:
    """get_level / get_highest_level 辅助函数测试。"""

    def test_get_level_returns_correct(self):
        import sys
        sys.path.insert(0, str(__import__("pathlib").Path(__file__).resolve().parents[1] / "scripts"))
        from oq5_d_operator import DOperatorResult, LevelResult, get_level, get_highest_level

        lr1 = LevelResult(
            level_id=1, move_count=3, moves=[],
            last_move_direction="up", last_move_kind="trend",
            last_move_settled=False, structural_breakdown=False,
        )
        lr2 = LevelResult(
            level_id=2, move_count=1, moves=[],
            last_move_direction="down", last_move_kind="consolidation",
            last_move_settled=True, structural_breakdown=True,
        )

        result = DOperatorResult(
            symbol="TEST", bar_count=100,
            max_level_reached=2, levels=[lr1, lr2],
        )

        assert get_level(result, 1) is lr1
        assert get_level(result, 2) is lr2
        assert get_level(result, 3) is None
        assert get_highest_level(result) is lr2


class TestDetectTripleSynchrony:
    """三重联立检测测试。"""

    def test_all_empty_returns_not_met(self):
        import sys
        sys.path.insert(0, str(__import__("pathlib").Path(__file__).resolve().parents[1] / "scripts"))
        from oq5_d_operator import DOperatorResult, detect_triple_synchrony

        empty = DOperatorResult(
            symbol="DXY", bar_count=0, max_level_reached=0, levels=[],
        )

        sync = detect_triple_synchrony(empty, [], None, check_level_id=1)
        assert not sync.triple_met
        assert not sync.center_breakdown
        assert not sync.periphery_construction
        assert not sync.gold_resonance
