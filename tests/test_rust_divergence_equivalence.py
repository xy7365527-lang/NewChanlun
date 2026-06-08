"""Rust 背驰 v1 ↔ Python 背驰 v1 逐位等价 golden 测试（fallback 路径）。

验证 `newchan_rust.divergences_from_moves_v1` 与 Python
`newchan.a_divergence_v1.divergences_from_moves_v1`（df_macd=None）在同一管线输出上
逐字段相等。

## 有效域（formalization-validity-domain.md）
Rust 背驰**只实装价格振幅 fallback 路径**（MACD 三维度属后续 MACD 层）。因此本测试
仅在 `df_macd=None` 有效域上对比——这是 Rust 背驰的完整定义域，非削减后的子集。

## 认识论等级
- 合成数据（BiEngine→segments→zhongshu→moves→divergences）：L1 全字段 bit-exact。
- BZ 真实数据：L2 全字段 bit-exact。

## bit-exact 基础
fallback force 唯一浮点算术是 `(high-low) * duration`（单次乘法，IEEE-754 bit-exact）。
high/low 由 max/min 顺序约简，bit-exact。
"""

from __future__ import annotations

import math

import pytest

from newchan.a_divergence_v1 import divergences_from_moves_v1 as py_divs
from newchan.a_move_v1 import moves_from_zhongshus as py_moves
from newchan.a_segment_v1 import segments_from_strokes_v1 as py_segments
from newchan.a_zhongshu_v1 import zhongshu_from_segments as py_zs_segments
from newchan.bi_engine import BiEngine as PyBiEngine
from newchan.core.bar import BarV1

newchan_rust = pytest.importorskip("newchan_rust")


def _seg_to_input(s) -> tuple:
    """Python Segment → Rust divergence 段输入 (direction, high, low, i0, i1)。"""
    return (s.direction, s.high, s.low, s.i0, s.i1)


def _zs_to_div_input(z) -> tuple:
    """Python Zhongshu → Rust divergence 中枢输入 (zd, zg, seg_start, seg_end, settled)。"""
    return (z.zd, z.zg, z.seg_start, z.seg_end, z.settled)


def _move_to_input(m) -> tuple:
    """Python Move → Rust 走势输入 8 元组。"""
    return (
        m.kind, m.direction, m.seg_start, m.seg_end,
        m.zs_start, m.zs_end, m.zs_count, m.settled,
    )


def _div_eq(py_d, rs) -> bool:
    """Python Divergence 与 Rust 嵌套元组逐字段相等（浮点 bit-exact）。"""
    head, tail = rs
    # head = (kind, direction, level_id, seg_a_start, seg_a_end, seg_c_start, seg_c_end, center_idx)
    # tail = (force_a, force_c, confirmed, dif_peak_a, dif_peak_c, hist_peak_a, hist_peak_c)
    return (
        py_d.kind == head[0]
        and py_d.direction == head[1]
        and py_d.level_id == head[2]
        and py_d.seg_a_start == head[3]
        and py_d.seg_a_end == head[4]
        and py_d.seg_c_start == head[5]
        and py_d.seg_c_end == head[6]
        and py_d.center_idx == head[7]
        and py_d.force_a == tail[0]
        and py_d.force_c == tail[1]
        and py_d.confirmed == tail[2]
        and py_d.dif_peak_a == tail[3]
        and py_d.dif_peak_c == tail[4]
        and py_d.hist_peak_a == tail[5]
        and py_d.hist_peak_c == tail[6]
    )


def _assert_divs_equal(py_ds, rs_ds, ctx: str) -> None:
    assert len(py_ds) == len(rs_ds), (
        f"{ctx}: 背驰数不等 py={len(py_ds)} rust={len(rs_ds)}"
    )
    for idx, (a, b) in enumerate(zip(py_ds, rs_ds)):
        assert _div_eq(a, b), (
            f"{ctx}: 第 {idx} 背驰分歧\n"
            f"  py  =(kind={a.kind},dir={a.direction},aS={a.seg_a_start},aE={a.seg_a_end},"
            f"cS={a.seg_c_start},cE={a.seg_c_end},ci={a.center_idx},"
            f"fa={a.force_a},fc={a.force_c},conf={a.confirmed})\n"
            f"  rust={b}"
        )


def _synthetic_bars(n: int):
    """与 move 等价测试同款合成数据（含慢趋势分量，产生多 settled 中枢→moves）。"""
    bars = []
    for k in range(n):
        base = (
            100.0
            + 40.0 * math.sin(k * 0.004)
            + 10.0 * math.sin(k * 0.05)
            + 3.0 * math.sin(k * 0.17 + 1.0)
            + 1.5 * math.sin(k * 0.31 + 2.0)
        )
        spread = 0.5 + 0.4 * abs(math.sin(k * 0.13))
        o = base + 0.2 * math.sin(k * 0.7)
        c = base + 0.2 * math.cos(k * 0.7)
        bars.append((o, max(o, c) + spread, min(o, c) - spread, c))
    return bars


def _pipeline_from_bars(bars):
    """bars → (segments, zhongshus, moves)。"""
    eng = PyBiEngine(stroke_mode="new")
    ts0 = 1_700_000_000.0
    for k, (o, h, l, c) in enumerate(bars):
        eng.process_bar(BarV1(bar_time=ts0 + k * 60, open=o, high=h, low=l, close=c))
    strokes = eng.current_strokes
    segs = py_segments(strokes, min_seg_strokes=3, extend_mode="strict")
    zhongshus = py_zs_segments(segs)
    moves = py_moves(zhongshus, num_segments=len(segs))
    return segs, zhongshus, moves


def test_divergence_synthetic() -> None:
    """合成数据：Python vs Rust 背驰全字段 bit-exact（fallback）。"""
    segs, zhongshus, moves = _pipeline_from_bars(_synthetic_bars(10000))
    assert len(moves) > 0, "未产生走势，测试无效"

    py_ds = py_divs(segs, zhongshus, moves, 1)
    rs_ds = newchan_rust.divergences_from_moves_v1(
        [_seg_to_input(s) for s in segs],
        [_zs_to_div_input(z) for z in zhongshus],
        [_move_to_input(m) for m in moves],
        1,
    )
    _assert_divs_equal(py_ds, rs_ds, "synthetic")
    assert len(py_ds) > 0, "未产生背驰，测试无效（fallback 有效域未覆盖）"


def test_divergence_empty() -> None:
    """边界：空输入两实现均返回空。"""
    assert py_divs([], [], [], 1) == []
    assert newchan_rust.divergences_from_moves_v1([], [], [], 1) == []


_BZ_PARQUET = ".cache/BZ_1min_2024_raw.parquet"


@pytest.mark.slow
def test_divergence_real_data() -> None:
    """BZ 真实数据：背驰全字段 bit-exact（L2）。"""
    import os

    if not os.path.exists(_BZ_PARQUET):
        pytest.skip(f"真实数据不存在: {_BZ_PARQUET}")
    import pandas as pd

    df = pd.read_parquet(_BZ_PARQUET)
    eng = PyBiEngine(stroke_mode="new")
    for t, o, hi, lo, cl in zip(
        df.index, df["open"], df["high"], df["low"], df["close"]
    ):
        eng.process_bar(BarV1(bar_time=t.timestamp(), open=o, high=hi, low=lo, close=cl))
    strokes = eng.current_strokes
    segs = py_segments(strokes, min_seg_strokes=3, extend_mode="strict")
    zhongshus = py_zs_segments(segs)
    moves = py_moves(zhongshus, num_segments=len(segs))
    assert len(moves) > 10

    py_ds = py_divs(segs, zhongshus, moves, 1)
    rs_ds = newchan_rust.divergences_from_moves_v1(
        [_seg_to_input(s) for s in segs],
        [_zs_to_div_input(z) for z in zhongshus],
        [_move_to_input(m) for m in moves],
        1,
    )
    _assert_divs_equal(py_ds, rs_ds, "real_BZ")
