"""Rust 走势类型 v1 ↔ Python 走势类型 v1 逐位等价 golden 测试。

验证 `newchan_rust.moves_from_zhongshus` 与 Python
`newchan.a_move_v1.moves_from_zhongshus` 在同一中枢列表上输出逐字段相等。

## 认识论等级
- 合成数据（BiEngine→segments→zhongshu→moves）：L1 全字段 bit-exact。
- BZ 真实数据：L2 全字段 bit-exact。

## bit-exact 基础
走势构造无浮点算术，只比较 + max/min 选择，故 high/low/zg_max/zd_min bit-exact。
persistence 恒 0.0（PH 层后填，move 层不算）。
"""

from __future__ import annotations

import math

import pytest

from newchan.a_move_v1 import moves_from_zhongshus as py_moves
from newchan.a_segment_v1 import segments_from_strokes_v1 as py_segments
from newchan.a_zhongshu_v1 import zhongshu_from_segments as py_zs_segments
from newchan.bi_engine import BiEngine as PyBiEngine
from newchan.core.bar import BarV1

newchan_rust = pytest.importorskip("newchan_rust")


def _zs_to_input(z) -> tuple:
    """Python Zhongshu → Rust moves_from_zhongshus 输入 12 元组。"""
    return (
        z.zd, z.zg, z.seg_start, z.seg_end, z.seg_count, z.settled,
        z.break_seg, z.break_direction, z.first_seg_s0, z.last_seg_s1,
        z.gg, z.dd,
    )


def _move_eq(py_m, rs) -> bool:
    """Python Move 与 Rust 嵌套元组逐字段相等（浮点 bit-exact）。"""
    head, tail = rs
    # head = (kind, direction, seg_start, seg_end, zs_start, zs_end, zs_count, settled)
    # tail = (high, low, first_seg_s0, last_seg_s1, zg_max, zd_min, persistence)
    return (
        py_m.kind == head[0]
        and py_m.direction == head[1]
        and py_m.seg_start == head[2]
        and py_m.seg_end == head[3]
        and py_m.zs_start == head[4]
        and py_m.zs_end == head[5]
        and py_m.zs_count == head[6]
        and py_m.settled == head[7]
        and py_m.high == tail[0]
        and py_m.low == tail[1]
        and py_m.first_seg_s0 == tail[2]
        and py_m.last_seg_s1 == tail[3]
        and py_m.zg_max == tail[4]
        and py_m.zd_min == tail[5]
        and py_m.persistence == tail[6]
    )


def _assert_moves_equal(py_ms, rs_ms, ctx: str) -> None:
    assert len(py_ms) == len(rs_ms), (
        f"{ctx}: 走势数不等 py={len(py_ms)} rust={len(rs_ms)}"
    )
    for idx, (a, b) in enumerate(zip(py_ms, rs_ms)):
        assert _move_eq(a, b), (
            f"{ctx}: 第 {idx} 走势分歧\n"
            f"  py  =(kind={a.kind},dir={a.direction},ss={a.seg_start},se={a.seg_end},"
            f"zss={a.zs_start},zse={a.zs_end},zsc={a.zs_count},settled={a.settled},"
            f"high={a.high},low={a.low},zg_max={a.zg_max},zd_min={a.zd_min})\n"
            f"  rust={b}"
        )


def _synthetic_bars(n: int):
    # 含慢趋势分量（振幅 40，周期 ~1570 bars）使价格大幅漂移、走出区间，
    # 产生多个独立（不重叠）中枢 → settled 中枢 → moves。纯振荡（无漂移）
    # 会使所有中枢重叠成一个未 settled 大中枢，产生不出 move。
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


def _zhongshus_from_bars(bars):
    eng = PyBiEngine(stroke_mode="new")
    ts0 = 1_700_000_000.0
    for k, (o, h, l, c) in enumerate(bars):
        eng.process_bar(BarV1(bar_time=ts0 + k * 60, open=o, high=h, low=l, close=c))
    strokes = eng.current_strokes
    segs = py_segments(strokes, min_seg_strokes=3, extend_mode="strict")
    return py_zs_segments(segs), len(segs)


@pytest.mark.parametrize("use_num_segments", [False, True])
def test_move_synthetic(use_num_segments: bool) -> None:
    """合成数据：Python vs Rust moves 全字段 bit-exact（含/不含 num_segments）。"""
    zhongshus, n_segs = _zhongshus_from_bars(_synthetic_bars(10000))
    assert len(zhongshus) > 1, "中枢不足"

    ns = n_segs if use_num_segments else None
    py_ms = py_moves(zhongshus, num_segments=ns)
    rs_ms = newchan_rust.moves_from_zhongshus([_zs_to_input(z) for z in zhongshus], ns)
    _assert_moves_equal(py_ms, rs_ms, f"synthetic[num_seg={ns}]")
    assert len(py_ms) > 0, "未产生走势，测试无效"


def test_move_empty() -> None:
    """边界：空中枢列表两实现均返回空。"""
    assert py_moves([]) == []
    assert newchan_rust.moves_from_zhongshus([], None) == []


_BZ_PARQUET = ".cache/BZ_1min_2024_raw.parquet"


@pytest.mark.slow
@pytest.mark.parametrize("use_num_segments", [False, True])
def test_move_real_data(use_num_segments: bool) -> None:
    """BZ 真实数据：moves 全字段 bit-exact（L2）。"""
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
    assert len(zhongshus) > 10

    ns = len(segs) if use_num_segments else None
    py_ms = py_moves(zhongshus, num_segments=ns)
    rs_ms = newchan_rust.moves_from_zhongshus([_zs_to_input(z) for z in zhongshus], ns)
    _assert_moves_equal(py_ms, rs_ms, f"real_BZ[num_seg={ns}]")
