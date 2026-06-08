"""Rust PH 层 ↔ Python PH 层逐位等价 golden 测试。

验证 `newchan_rust` 的 `compute_move_persistence` / `attach_persistence` /
`should_stop_recursion` 与 Python `newchan.ph_layer` 同名函数在同一输入上
输出 bit-exact 相等。

## 认识论等级
- 合成数据（BiEngine→segments→zhongshu→moves→PH）：L1 全字段 bit-exact。
- BZ 真实数据：L2 全字段 bit-exact。

## bit-exact 基础
persistence = 闭式 max(prices) - min(prices)（1D sublevel H0 最大特征 = 全局价格
极差，ph_layer.py 已证 L0 恒等式）。唯一算术是减法，极值扫描顺序两实现一致（按
zhongshu 索引 + dd/gg 顺序），故 bit-exact。should_stop_recursion 只做比较，无算术。
"""

from __future__ import annotations

import math

import pytest

from newchan.a_move_v1 import moves_from_zhongshus as py_moves
from newchan.a_segment_v1 import segments_from_strokes_v1 as py_segments
from newchan.a_zhongshu_v1 import zhongshu_from_segments as py_zs_segments
from newchan.bi_engine import BiEngine as PyBiEngine
from newchan.core.bar import BarV1
from newchan.ph_layer import (
    attach_persistence as py_attach,
    compute_move_persistence as py_compute,
    should_stop_recursion as py_should_stop,
)

newchan_rust = pytest.importorskip("newchan_rust")


# --------------------------------------------------------------------------
# Python 对象 → Rust 输入元组
# --------------------------------------------------------------------------


def _zs_to_input(z) -> tuple:
    """Python Zhongshu → Rust 12 元组（PH 仅用 dd/gg，但保持与 move 测试同构）。"""
    return (
        z.zd, z.zg, z.seg_start, z.seg_end, z.seg_count, z.settled,
        z.break_seg, z.break_direction, z.first_seg_s0, z.last_seg_s1,
        z.gg, z.dd,
    )


def _move_to_input(m) -> tuple:
    """Python Move → Rust MoveTupleIn（嵌套元组，kind/direction 为字符串）。"""
    return (
        (m.kind, m.direction, m.seg_start, m.seg_end,
         m.zs_start, m.zs_end, m.zs_count, m.settled),
        (m.high, m.low, m.first_seg_s0, m.last_seg_s1,
         m.zg_max, m.zd_min, m.persistence),
    )


def _move_eq_full(py_m, rs) -> bool:
    """Python Move 与 Rust 嵌套元组逐字段 bit-exact 相等（含 persistence）。"""
    head, tail = rs
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


# --------------------------------------------------------------------------
# 数据生成（复用 move 测试的合成器）
# --------------------------------------------------------------------------


def _synthetic_bars(n: int):
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


# --------------------------------------------------------------------------
# 测试：compute_move_persistence（单个）
# --------------------------------------------------------------------------


def test_compute_persistence_synthetic() -> None:
    """合成数据：逐个 Move 的 persistence bit-exact。"""
    zhongshus, n_segs = _zhongshus_from_bars(_synthetic_bars(10000))
    moves = py_moves(zhongshus, num_segments=n_segs)
    assert len(moves) > 0, "未产生走势，测试无效"

    zs_in = [_zs_to_input(z) for z in zhongshus]
    for idx, m in enumerate(moves):
        py_val = py_compute(m, zhongshus)
        rs_val = newchan_rust.compute_move_persistence(_move_to_input(m), zs_in)
        assert py_val == rs_val, (
            f"move[{idx}] persistence 分歧 py={py_val!r} rust={rs_val!r}"
        )


def test_compute_persistence_empty_zhongshus() -> None:
    """边界：空中枢列表 → high - low。"""
    # zs_start/zs_end 任意，high-low 应被返回
    m_in = (("trend", "up", 0, 0, 0, 0, 1, True), (110.0, 90.0, 0, 0, 0.0, 0.0, 0.0))
    assert newchan_rust.compute_move_persistence(m_in, []) == 20.0


# --------------------------------------------------------------------------
# 测试：attach_persistence（批量，全字段）
# --------------------------------------------------------------------------


@pytest.mark.parametrize("use_num_segments", [False, True])
def test_attach_persistence_synthetic(use_num_segments: bool) -> None:
    """合成数据：attach 后全字段 bit-exact（persistence 填充，其余不变）。"""
    zhongshus, n_segs = _zhongshus_from_bars(_synthetic_bars(10000))
    ns = n_segs if use_num_segments else None
    moves = py_moves(zhongshus, num_segments=ns)
    assert len(moves) > 0

    py_attached = py_attach(moves, zhongshus)
    rs_attached = newchan_rust.attach_persistence(
        [_move_to_input(m) for m in moves],
        [_zs_to_input(z) for z in zhongshus],
    )

    assert len(py_attached) == len(rs_attached)
    for idx, (a, b) in enumerate(zip(py_attached, rs_attached)):
        assert _move_eq_full(a, b), (
            f"attach[{idx}] 分歧\n  py.persistence={a.persistence!r}\n  rust={b}"
        )
    # 至少一个 move 拿到了非退化 persistence（multi-center 路径被覆盖）
    assert any(m.persistence > 0 for m in py_attached)


def test_attach_persistence_empty() -> None:
    """边界：空 move 列表两实现均返回空。"""
    assert py_attach([], []) == []
    assert newchan_rust.attach_persistence([], []) == []


# --------------------------------------------------------------------------
# 测试：should_stop_recursion
# --------------------------------------------------------------------------


def test_should_stop_recursion_synthetic() -> None:
    """合成数据派生的多场景：should_stop bool bit-exact。

    用真实 attached moves 构造若干 (curr, prev) 组合，覆盖 True/False/空集分支。
    """
    zhongshus, n_segs = _zhongshus_from_bars(_synthetic_bars(10000))
    moves = py_attach(py_moves(zhongshus, num_segments=n_segs), zhongshus)
    assert len(moves) >= 2

    # 把 moves 切两半模拟相邻递归层
    mid = len(moves) // 2
    lo_half = moves[:mid]
    hi_half = moves[mid:]

    scenarios = [
        ("same", moves, moves),
        ("lo_vs_hi", lo_half, hi_half),
        ("hi_vs_lo", hi_half, lo_half),
        ("empty_curr", [], moves),
        ("empty_prev", moves, []),
        ("both_empty", [], []),
    ]
    for name, curr, prev in scenarios:
        py_r = py_should_stop(curr, prev)
        rs_r = newchan_rust.should_stop_recursion(
            [_move_to_input(m) for m in curr],
            [_move_to_input(m) for m in prev],
        )
        assert py_r == rs_r, f"scenario={name}: py={py_r} rust={rs_r}"


# --------------------------------------------------------------------------
# BZ 真实数据（L2）
# --------------------------------------------------------------------------

_BZ_PARQUET = ".cache/BZ_1min_2024_raw.parquet"


@pytest.mark.slow
def test_ph_real_data() -> None:
    """BZ 真实数据：compute + attach + should_stop 全部 bit-exact（L2）。"""
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

    moves = py_moves(zhongshus, num_segments=len(segs))
    assert len(moves) > 0

    zs_in = [_zs_to_input(z) for z in zhongshus]

    # compute 逐个
    for idx, m in enumerate(moves):
        assert py_compute(m, zhongshus) == newchan_rust.compute_move_persistence(
            _move_to_input(m), zs_in
        ), f"real move[{idx}] compute 分歧"

    # attach 全字段
    py_attached = py_attach(moves, zhongshus)
    rs_attached = newchan_rust.attach_persistence(
        [_move_to_input(m) for m in moves], zs_in
    )
    assert len(py_attached) == len(rs_attached)
    for idx, (a, b) in enumerate(zip(py_attached, rs_attached)):
        assert _move_eq_full(a, b), f"real attach[{idx}] 分歧 py={a.persistence} rust={b}"

    # should_stop（多场景）
    mid = len(py_attached) // 2
    for curr, prev in [
        (py_attached, py_attached),
        (py_attached[:mid], py_attached[mid:]),
        (py_attached[mid:], py_attached[:mid]),
    ]:
        assert py_should_stop(curr, prev) == newchan_rust.should_stop_recursion(
            [_move_to_input(m) for m in curr],
            [_move_to_input(m) for m in prev],
        )
