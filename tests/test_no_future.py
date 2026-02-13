"""无未来函数数学证明 — 最重要的集成测试

证明 BiEngine 在 bar_idx=k 的输出 === 仅用 bars[:k+1] 独立全量计算的 strokes。

设计原则：
- 每根 bar 都验证 BiEngine 增量输出与全量重算的一致性
- 使用足够产生多笔的锯齿价格序列
- 覆盖笔的数量、位置、方向、确认状态、价格等全部字段
"""

from __future__ import annotations

from datetime import datetime, timedelta, timezone

import pandas as pd
import pytest

from newchan.a_fractal import fractals_from_merged
from newchan.a_inclusion import merge_inclusion
from newchan.a_stroke import Stroke, strokes_from_fractals
from newchan.bi_engine import BiEngine
from newchan.types import Bar


# ── 测试数据生成 ──────────────────────────────────────────────────


def _generate_test_bars(n: int = 60) -> list[Bar]:
    """生成锯齿形（zigzag）价格序列，确保能产生多笔。

    设计思路：
    - 每 8 根 bar 一个半周期，价格在 60-100 之间大幅振荡
    - 相邻 bar 的 high/low 严格不包含（避免合并导致 merged 数量不足）
    - 下降段：100 → 65（每 bar 降 5）
    - 上升段：65 → 100（每 bar 升 5）
    - 使用固定小振幅（+/- 1.5）确保不包含
    """
    bars: list[Bar] = []
    for i in range(n):
        cycle_pos = i % 16
        if cycle_pos < 8:
            # 下降段
            base = 100 - cycle_pos * 5
        else:
            # 上升段
            base = 65 + (cycle_pos - 8) * 5

        h = base + 1.5
        l = base - 1.5
        o = base + 0.5
        c = base - 0.5
        bars.append(
            Bar(
                ts=datetime(2024, 1, 1, tzinfo=timezone.utc)
                + timedelta(minutes=i * 5),
                open=o,
                high=h,
                low=l,
                close=c,
            )
        )
    return bars


def _full_pipeline(bars: list[Bar]) -> list[Stroke]:
    """对一组 bar 执行完整管线：包含处理 → 分型 → 笔构造。"""
    if not bars:
        return []
    df = pd.DataFrame(
        [[b.open, b.high, b.low, b.close] for b in bars],
        columns=["open", "high", "low", "close"],
        index=pd.DatetimeIndex([b.ts for b in bars]),
    )
    merged, _ = merge_inclusion(df)
    fractals = fractals_from_merged(merged)
    strokes = strokes_from_fractals(merged, fractals)
    return strokes


# =====================================================================
# 测试类
# =====================================================================


class TestNoFutureFunction:
    """数学证明：BiEngine 在 bar_idx=k 的 current_strokes
    === 仅用 bars[:k+1] 独立全量计算的 strokes。"""

    def test_no_future_function(self):
        """逐 bar 验证：增量结果 == 全量重算结果。"""
        bars = _generate_test_bars(60)
        engine = BiEngine()

        for k, bar in enumerate(bars):
            snap = engine.process_bar(bar)

            # 独立全量计算
            strokes_full = _full_pipeline(bars[: k + 1])

            # 断言笔数量一致
            assert len(snap.strokes) == len(strokes_full), (
                f"bar {k}: BiEngine has {len(snap.strokes)} strokes, "
                f"full pipeline has {len(strokes_full)}"
            )

            # 断言每笔字段一致
            for j, (a, b) in enumerate(zip(snap.strokes, strokes_full)):
                assert a.i0 == b.i0, (
                    f"bar {k}, stroke {j}: i0 mismatch: "
                    f"engine={a.i0}, full={b.i0}"
                )
                assert a.i1 == b.i1, (
                    f"bar {k}, stroke {j}: i1 mismatch: "
                    f"engine={a.i1}, full={b.i1}"
                )
                assert a.direction == b.direction, (
                    f"bar {k}, stroke {j}: direction mismatch: "
                    f"engine={a.direction}, full={b.direction}"
                )
                assert a.confirmed == b.confirmed, (
                    f"bar {k}, stroke {j}: confirmed mismatch: "
                    f"engine={a.confirmed}, full={b.confirmed}"
                )
                assert abs(a.p0 - b.p0) < 1e-9, (
                    f"bar {k}, stroke {j}: p0 mismatch: "
                    f"engine={a.p0}, full={b.p0}"
                )
                assert abs(a.p1 - b.p1) < 1e-9, (
                    f"bar {k}, stroke {j}: p1 mismatch: "
                    f"engine={a.p1}, full={b.p1}"
                )
                assert abs(a.high - b.high) < 1e-9, (
                    f"bar {k}, stroke {j}: high mismatch: "
                    f"engine={a.high}, full={b.high}"
                )
                assert abs(a.low - b.low) < 1e-9, (
                    f"bar {k}, stroke {j}: low mismatch: "
                    f"engine={a.low}, full={b.low}"
                )

    def test_produces_multiple_strokes(self):
        """验证测试数据确实能产生多笔（>=3），否则证明力度不足。"""
        bars = _generate_test_bars(60)
        strokes = _full_pipeline(bars)
        assert len(strokes) >= 3, (
            f"Test data only produces {len(strokes)} strokes, need >= 3"
        )

    def test_no_future_short_sequence(self):
        """短序列（20 bar）也满足无未来函数。"""
        bars = _generate_test_bars(20)
        engine = BiEngine()

        for k, bar in enumerate(bars):
            snap = engine.process_bar(bar)
            strokes_full = _full_pipeline(bars[: k + 1])

            assert len(snap.strokes) == len(strokes_full), (
                f"bar {k}: length mismatch"
            )
            for j, (a, b) in enumerate(zip(snap.strokes, strokes_full)):
                assert a.i0 == b.i0
                assert a.i1 == b.i1
                assert a.direction == b.direction
                assert a.confirmed == b.confirmed
