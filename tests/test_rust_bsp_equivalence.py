"""Rust 买卖点 v1 ↔ Python 买卖点 v1 逐位等价 golden 测试。

验证 `newchan_rust.buysellpoints_from_level` 与 Python
`newchan.a_buysellpoint_v1.buysellpoints_from_level` 在同一组（segments, zhongshus,
moves, divergences）上输出逐字段相等（含 type1/type2/type3 + 2B3B 重合标记 + 排序）。

## 等价隔离
背驰由 Python `divergences_from_moves_v1`（fallback）产出，同一组背驰同时喂给 Python
与 Rust 的 buysellpoints_from_level —— 隔离 BSP 层等价（不混入背驰层差异）。

## 认识论等级
- 合成数据（全管线 → divergences → BSP）：L1 全字段 bit-exact。
- BZ 真实数据：L2 全字段 bit-exact。

## bit-exact 基础
confirmed 唯一浮点算术是 `force_c / force_a ≤ 0.9`（单次除法）。price = seg.low/seg.high
（直接取值）。最终 sorted(by seg_idx) 用稳定排序复刻 Python `sorted`。
"""

from __future__ import annotations

import math

import pytest

from newchan.a_buysellpoint_v1 import buysellpoints_from_level as py_bsps
from newchan.a_divergence_v1 import divergences_from_moves_v1 as py_divs
from newchan.a_move_v1 import moves_from_zhongshus as py_moves
from newchan.a_segment_v1 import segments_from_strokes_v1 as py_segments
from newchan.a_zhongshu_v1 import zhongshu_from_segments as py_zs_segments
from newchan.bi_engine import BiEngine as PyBiEngine
from newchan.core.bar import BarV1

newchan_rust = pytest.importorskip("newchan_rust")


def _seg_to_input(s) -> tuple:
    return (s.direction, s.high, s.low, s.i0, s.i1)


def _zs_to_bsp_input(z) -> tuple:
    """Python Zhongshu → Rust BSP 中枢输入 7 元组（含 break 字段）。"""
    return (z.zd, z.zg, z.seg_start, z.seg_end, z.settled, z.break_direction, z.break_seg)


def _move_to_input(m) -> tuple:
    return (
        m.kind, m.direction, m.seg_start, m.seg_end,
        m.zs_start, m.zs_end, m.zs_count, m.settled,
    )


def _div_to_input(d) -> tuple:
    """Python Divergence → Rust BSP 背驰输入 7 元组（BSP 消费的字段子集）。"""
    return (d.kind, d.direction, d.center_idx, d.seg_c_start, d.seg_c_end, d.force_a, d.force_c)


def _bsp_eq(py_b, rs) -> bool:
    """Python BuySellPoint 与 Rust 嵌套元组逐字段相等（浮点 bit-exact）。"""
    head, mid, div_key, center_seg_start, overlaps = rs
    # head = (kind, side, level_id, seg_idx, move_seg_start, confirmed, settled)
    # mid  = (center_zd, center_zg, price, bar_idx)
    return (
        py_b.kind == head[0]
        and py_b.side == head[1]
        and py_b.level_id == head[2]
        and py_b.seg_idx == head[3]
        and py_b.move_seg_start == head[4]
        and py_b.confirmed == head[5]
        and py_b.settled == head[6]
        and py_b.center_zd == mid[0]
        and py_b.center_zg == mid[1]
        and py_b.price == mid[2]
        and py_b.bar_idx == mid[3]
        and py_b.divergence_key == div_key
        and py_b.center_seg_start == center_seg_start
        and py_b.overlaps_with == overlaps
    )


def _assert_bsps_equal(py_bs, rs_bs, ctx: str) -> None:
    assert len(py_bs) == len(rs_bs), (
        f"{ctx}: 买卖点数不等 py={len(py_bs)} rust={len(rs_bs)}"
    )
    for idx, (a, b) in enumerate(zip(py_bs, rs_bs)):
        assert _bsp_eq(a, b), (
            f"{ctx}: 第 {idx} 买卖点分歧\n"
            f"  py  =(kind={a.kind},side={a.side},seg_idx={a.seg_idx},"
            f"move_ss={a.move_seg_start},conf={a.confirmed},settled={a.settled},"
            f"czd={a.center_zd},czg={a.center_zg},price={a.price},bar={a.bar_idx},"
            f"dkey={a.divergence_key},css={a.center_seg_start},ov={a.overlaps_with})\n"
            f"  rust={b}"
        )


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


def _pipeline_from_bars(bars):
    eng = PyBiEngine(stroke_mode="new")
    ts0 = 1_700_000_000.0
    for k, (o, h, l, c) in enumerate(bars):
        eng.process_bar(BarV1(bar_time=ts0 + k * 60, open=o, high=h, low=l, close=c))
    strokes = eng.current_strokes
    segs = py_segments(strokes, min_seg_strokes=3, extend_mode="strict")
    zhongshus = py_zs_segments(segs)
    moves = py_moves(zhongshus, num_segments=len(segs))
    divergences = py_divs(segs, zhongshus, moves, 1)
    return segs, zhongshus, moves, divergences


def _run_both(segs, zhongshus, moves, divergences, level_id=1):
    py_out = py_bsps(segs, zhongshus, moves, divergences, level_id)
    rs_out = newchan_rust.buysellpoints_from_level(
        [_seg_to_input(s) for s in segs],
        [_zs_to_bsp_input(z) for z in zhongshus],
        [_move_to_input(m) for m in moves],
        [_div_to_input(d) for d in divergences],
        level_id,
    )
    return py_out, rs_out


def test_bsp_synthetic() -> None:
    """合成数据：Python vs Rust 买卖点全字段 bit-exact。"""
    segs, zhongshus, moves, divergences = _pipeline_from_bars(_synthetic_bars(10000))
    py_out, rs_out = _run_both(segs, zhongshus, moves, divergences)
    _assert_bsps_equal(py_out, rs_out, "synthetic")
    assert len(py_out) > 0, "未产生买卖点，测试无效"


def test_bsp_empty() -> None:
    """边界：空输入两实现均返回空。"""
    assert py_bsps([], [], [], [], 1) == []
    assert newchan_rust.buysellpoints_from_level([], [], [], [], 1) == []


_BZ_PARQUET = ".cache/BZ_1min_2024_raw.parquet"


@pytest.mark.slow
def test_bsp_real_data() -> None:
    """BZ 真实数据：买卖点全字段 bit-exact（L2）。"""
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
    divergences = py_divs(segs, zhongshus, moves, 1)
    assert len(moves) > 10

    py_out, rs_out = _run_both(segs, zhongshus, moves, divergences)
    _assert_bsps_equal(py_out, rs_out, "real_BZ")
    assert len(py_out) > 0, "真实数据未产生买卖点，测试无效"
