"""Rust 中枢 v1 ↔ Python 中枢 v1 逐位等价 golden 测试。

验证 `newchan_rust.zhongshu_from_segments` / `zhongshu_from_strokes` 与 Python
`newchan.a_zhongshu_v1` 对应函数在同一输入上输出逐字段相等。

## 契约（批量等价）

中枢是组件列表 → 中枢列表的批量函数。两条路径：
- 线段中枢：组件 = 线段，过滤 `confirmed AND kind=="settled"`，时间锚 s0/s1。
- 笔中枢：组件 = 笔，过滤 `confirmed`，时间锚 i0/i1。

过滤逻辑在 Rust 内复刻——Python 侧传未过滤的全量组件。

## 认识论等级

- 合成数据（经 BiEngine→segments）：L1 全量逐字段 bit-exact。
- BZ 真实数据：L2 全量逐字段 bit-exact。

## bit-exact 基础

中枢构造**无浮点算术**，只有比较与三元 max/min 选择，故 zd/zg/gg/dd 浮点字段
应完全相等（非容差）。
"""

from __future__ import annotations

import math

import pytest

from newchan.a_segment_v1 import segments_from_strokes_v1 as py_segments
from newchan.a_zhongshu_v1 import (
    zhongshu_from_segments as py_zs_segments,
    zhongshu_from_strokes as py_zs_strokes,
)
from newchan.bi_engine import BiEngine as PyBiEngine
from newchan.core.bar import BarV1

newchan_rust = pytest.importorskip("newchan_rust")


# ── 转换 ──


def _seg_to_input(s) -> tuple:
    """Python Segment → Rust zhongshu_from_segments 输入 (s0,s1,high,low,confirmed,kind_settled)。"""
    return (s.s0, s.s1, s.high, s.low, s.confirmed, s.kind == "settled")


def _stroke_to_input(s) -> tuple:
    """Python Stroke → Rust zhongshu_from_strokes 输入 (i0,i1,high,low,confirmed)。"""
    return (s.i0, s.i1, s.high, s.low, s.confirmed)


# ── 中枢对比（bit-exact）──


def _zs_eq(py_z, rs) -> bool:
    """Python Zhongshu 与 Rust 12 元组逐字段相等。

    rs = (zd, zg, seg_start, seg_end, seg_count, settled, break_seg,
          break_direction, first_seg_s0, last_seg_s1, gg, dd)
    浮点字段用 `==`（bit-exact）。
    """
    return (
        py_z.zd == rs[0]
        and py_z.zg == rs[1]
        and py_z.seg_start == rs[2]
        and py_z.seg_end == rs[3]
        and py_z.seg_count == rs[4]
        and py_z.settled == rs[5]
        and py_z.break_seg == rs[6]
        and py_z.break_direction == rs[7]
        and py_z.first_seg_s0 == rs[8]
        and py_z.last_seg_s1 == rs[9]
        and py_z.gg == rs[10]
        and py_z.dd == rs[11]
    )


def _assert_zs_equal(py_zs, rs_zs, ctx: str) -> None:
    assert len(py_zs) == len(rs_zs), (
        f"{ctx}: 中枢数不等 py={len(py_zs)} rust={len(rs_zs)}"
    )
    for idx, (a, b) in enumerate(zip(py_zs, rs_zs)):
        assert _zs_eq(a, b), (
            f"{ctx}: 第 {idx} 中枢分歧\n"
            f"  py  =(zd={a.zd},zg={a.zg},ss={a.seg_start},se={a.seg_end},sc={a.seg_count},"
            f"settled={a.settled},bs={a.break_seg},bd={a.break_direction},"
            f"f0={a.first_seg_s0},l1={a.last_seg_s1},gg={a.gg},dd={a.dd})\n"
            f"  rust={b}"
        )


# ── 数据生成 ──


def _synthetic_bars(n: int) -> list[tuple[float, float, float, float]]:
    bars = []
    for k in range(n):
        base = (
            100.0
            + 10.0 * math.sin(k * 0.05)
            + 3.0 * math.sin(k * 0.17 + 1.0)
            + 1.5 * math.sin(k * 0.31 + 2.0)
        )
        spread = 0.5 + 0.4 * abs(math.sin(k * 0.13))
        o = base + 0.2 * math.sin(k * 0.7)
        c = base + 0.2 * math.cos(k * 0.7)
        bars.append((o, max(o, c) + spread, min(o, c) - spread, c))
    return bars


def _strokes_from_bars(bars):
    eng = PyBiEngine(stroke_mode="new")
    ts0 = 1_700_000_000.0
    for k, (o, h, l, c) in enumerate(bars):
        eng.process_bar(BarV1(bar_time=ts0 + k * 60, open=o, high=h, low=l, close=c))
    return eng.current_strokes


# ── 合成数据（L1）──


def test_zhongshu_from_strokes_synthetic() -> None:
    """笔中枢：合成数据全量逐字段 bit-exact（L1）。"""
    strokes = _strokes_from_bars(_synthetic_bars(6000))
    assert len(strokes) > 20, "笔不足"

    py_zs = py_zs_strokes(strokes)
    rs_zs = newchan_rust.zhongshu_from_strokes([_stroke_to_input(s) for s in strokes])
    _assert_zs_equal(py_zs, rs_zs, "stroke_zs_synthetic")
    assert len(py_zs) > 0, "未产生笔中枢，测试无效"


def test_zhongshu_from_segments_synthetic() -> None:
    """线段中枢：合成数据全量逐字段 bit-exact（L1）。"""
    strokes = _strokes_from_bars(_synthetic_bars(8000))
    segs = py_segments(strokes, min_seg_strokes=3, extend_mode="strict")
    assert len(segs) > 3, "线段不足"

    py_zs = py_zs_segments(segs)
    rs_zs = newchan_rust.zhongshu_from_segments([_seg_to_input(s) for s in segs])
    _assert_zs_equal(py_zs, rs_zs, "segment_zs_synthetic")


# ── 真实数据（L2）──

_BZ_PARQUET = ".cache/BZ_1min_2024_raw.parquet"


def _bz_strokes():
    import os

    if not os.path.exists(_BZ_PARQUET):
        pytest.skip(f"真实数据不存在: {_BZ_PARQUET}")
    import pandas as pd

    df = pd.read_parquet(_BZ_PARQUET)
    eng = PyBiEngine(stroke_mode="new")
    for t, o, hi, lo, cl in zip(
        df.index, df["open"], df["high"], df["low"], df["close"]
    ):
        eng.process_bar(
            BarV1(bar_time=t.timestamp(), open=o, high=hi, low=lo, close=cl)
        )
    return eng.current_strokes


@pytest.mark.slow
def test_zhongshu_real_data() -> None:
    """BZ 真实数据：笔中枢 + 线段中枢全量逐字段 bit-exact（L2）。"""
    strokes = _bz_strokes()
    assert len(strokes) > 1000

    # 笔中枢
    py_sz = py_zs_strokes(strokes)
    rs_sz = newchan_rust.zhongshu_from_strokes([_stroke_to_input(s) for s in strokes])
    _assert_zs_equal(py_sz, rs_sz, "stroke_zs_BZ")

    # 线段中枢
    segs = py_segments(strokes, min_seg_strokes=3, extend_mode="strict")
    py_gz = py_zs_segments(segs)
    rs_gz = newchan_rust.zhongshu_from_segments([_seg_to_input(s) for s in segs])
    _assert_zs_equal(py_gz, rs_gz, "segment_zs_BZ")
    assert len(py_gz) > 10, "线段中枢数不足，L2 验证无效"
