"""Rust segment 引擎 ↔ Python segment 引擎 逐位等价 golden 测试。

验证 `newchan_rust.segments_from_strokes_v1` 与 Python
`newchan.a_segment_v1.segments_from_strokes_v1` 在**同一笔列表**上输出逐字段相等。

## ⚠口径变更（#246 裁定，supersede Lead #84 点3；#277 裁路①、#288 落码 2026-07-26）

相切边界（三笔重叠含端点 `<=`、缺口谓词严格 `>`，对齐 Lean Overlaps/HasGap）
两侧同批切换——本 golden 契约在**新口径**下继续成立，相切边界不再 bit-exact
对齐 2026-07-26 前旧口径的历史输出/基线（实测段端点零变化，仅
`break_evidence.gap_type` 标签级翻转：BZ 全年 strict 28 段、optimized 159 段，
second→none）。裁定书：
`chanlun/escalate/tangency-overlap-supersede-84p3-ruling-20260725.md`。
本文件其余声明不受影响。

## 契约：批量等价

segment 是笔列表 → 线段列表的批量函数。本测试的契约是：给定同一 `list[Stroke]`，
两实现输出的 `list[Segment]` 逐字段相等（s0/s1/i0/i1/direction/high/low/confirmed/kind/
端点六元/p0/p1/break_evidence）。Python 的 `_resume` 增量快路径是内部优化（已单独验证
等于批量），不属跨语言契约。

## 认识论等级（formalization-validity-domain 规则）

- `test_segment_bitexact_synthetic`：合成确定性笔（经 Python BiEngine 生成），全量逐字段
  对比。**L1**（管线正确性，输入自造）——验证 Rust 移植无逻辑 bug。
- `test_segment_bitexact_real`：BZ Brent 原油真实 1min 数据经 BiEngine → strokes → segments，
  全量逐字段对比。**L2**（真实数据，可证伪）。

## 逐位等价基础（严格声明）

线段构造**不做任何浮点算术**——只有比较与 min/max 选择（端点价直接取自笔字段）。故
high/low/p0/price 等浮点字段的等价不依赖浮点约简顺序，只依赖比较逻辑与 min/max 并列
取首的语义一致。整数下标与 break_evidence 同理。
"""

from __future__ import annotations

import math

import pytest

from newchan.a_segment_v1 import segments_from_strokes_v1 as py_segments
from newchan.bi_engine import BiEngine as PyBiEngine
from newchan.core.bar import BarV1

newchan_rust = pytest.importorskip("newchan_rust")


# ── 笔 → Rust 输入元组 ──


def _stroke_to_tuple(s) -> tuple:
    """Python Stroke → Rust segments_from_strokes_v1 期望的 8 元组。"""
    return (s.i0, s.i1, s.direction, s.high, s.low, s.p0, s.p1, s.confirmed)


# ── segment 对比（bit-exact）──


def _break_evidence_eq(py_be, rs_be) -> bool:
    """break_evidence 逐字段相等（None ↔ None，或三元组相等）。"""
    if py_be is None:
        return rs_be is None
    if rs_be is None:
        return False
    # py_be: BreakEvidence(trigger_stroke_k, fractal_abc, gap_type)
    # rs_be: (trigger_stroke_k, (a, b, c), gap_type)
    return (
        py_be.trigger_stroke_k == rs_be[0]
        and tuple(py_be.fractal_abc) == tuple(rs_be[1])
        and py_be.gap_type == rs_be[2]
    )


def _segment_eq(py_seg, rs_tuple) -> bool:
    """Python Segment 与 Rust 嵌套元组逐字段相等。

    rs_tuple = (
        (s0, s1, i0, i1, dir, high, low, confirmed, kind),
        (ep0_i, ep0_price, ep0_type, ep1_i, ep1_price, ep1_type, p0, p1),
        break_evidence_or_None,
    )
    浮点字段用 `==`（真正 bit-exact，非容差）。
    """
    head, ep, be = rs_tuple
    return (
        py_seg.s0 == head[0]
        and py_seg.s1 == head[1]
        and py_seg.i0 == head[2]
        and py_seg.i1 == head[3]
        and py_seg.direction == head[4]
        and py_seg.high == head[5]
        and py_seg.low == head[6]
        and py_seg.confirmed == head[7]
        and py_seg.kind == head[8]
        and py_seg.ep0_i == ep[0]
        and py_seg.ep0_price == ep[1]
        and py_seg.ep0_type == ep[2]
        and py_seg.ep1_i == ep[3]
        and py_seg.ep1_price == ep[4]
        and py_seg.ep1_type == ep[5]
        and py_seg.p0 == ep[6]
        and py_seg.p1 == ep[7]
        and _break_evidence_eq(py_seg.break_evidence, be)
    )


def _assert_segments_equal(py_segs, rs_segs, ctx: str) -> None:
    """断言整列 segments 逐字段相等，失败时给出首个分歧的精确位置。"""
    assert len(py_segs) == len(rs_segs), (
        f"{ctx}: 段数不等 py={len(py_segs)} rust={len(rs_segs)}"
    )
    for idx, (a, b) in enumerate(zip(py_segs, rs_segs)):
        assert _segment_eq(a, b), (
            f"{ctx}: 第 {idx} 段分歧\n"
            f"  py  head=({a.s0},{a.s1},{a.i0},{a.i1},{a.direction},{a.high},{a.low},"
            f"{a.confirmed},{a.kind})\n"
            f"        ep=({a.ep0_i},{a.ep0_price},{a.ep0_type},{a.ep1_i},{a.ep1_price},"
            f"{a.ep1_type},{a.p0},{a.p1}) be={a.break_evidence}\n"
            f"  rust={b}"
        )


def _strokes_from_bars(bars, stroke_mode: str = "new"):
    """用 Python BiEngine 流式驱动 bars，返回最终笔列表。"""
    eng = PyBiEngine(stroke_mode=stroke_mode)
    ts0 = 1_700_000_000.0
    for k, (o, h, l, c) in enumerate(bars):
        eng.process_bar(BarV1(bar_time=ts0 + k * 60, open=o, high=h, low=l, close=c))
    return eng.current_strokes


# ── 合成确定性数据（L1）──


def _synthetic_bars(n: int) -> list[tuple[float, float, float, float]]:
    """确定性合成 OHLC：多频正弦叠加，产生丰富的分型/笔/线段结构。无随机性。"""
    bars: list[tuple[float, float, float, float]] = []
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
        hi = max(o, c) + spread
        lo = min(o, c) - spread
        bars.append((o, hi, lo, c))
    return bars


@pytest.mark.parametrize("extend_mode", ["strict", "optimized"])
def test_segment_bitexact_synthetic(extend_mode: str) -> None:
    """合成数据：Python vs Rust segment 全量逐字段 bit-exact（L1，两种延续模式）。"""
    bars = _synthetic_bars(6000)
    strokes = _strokes_from_bars(bars)
    assert len(strokes) > 20, "合成数据未产生足够的笔，segment 测试无效"

    py_segs = py_segments(strokes, min_seg_strokes=3, extend_mode=extend_mode)
    rs_segs = newchan_rust.segments_from_strokes_v1(
        [_stroke_to_tuple(s) for s in strokes], 3, extend_mode
    )

    _assert_segments_equal(py_segs, rs_segs, f"synthetic[{extend_mode}]")
    assert len(py_segs) > 2, "合成数据未产生足够的线段，测试无效"


@pytest.mark.parametrize("min_seg_strokes", [3, 5])
def test_segment_bitexact_min_seg_strokes(min_seg_strokes: int) -> None:
    """合成数据：变化 min_seg_strokes 参数仍逐字段等价（L1）。"""
    bars = _synthetic_bars(4000)
    strokes = _strokes_from_bars(bars)

    py_segs = py_segments(strokes, min_seg_strokes=min_seg_strokes, extend_mode="strict")
    rs_segs = newchan_rust.segments_from_strokes_v1(
        [_stroke_to_tuple(s) for s in strokes], min_seg_strokes, "strict"
    )

    _assert_segments_equal(py_segs, rs_segs, f"synthetic[min={min_seg_strokes}]")


def test_segment_empty_and_tiny() -> None:
    """边界：空/不足三笔输入两实现均返回空列表。"""
    for strokes in ([], None):
        if strokes is None:
            bars = _synthetic_bars(40)
            sk = _strokes_from_bars(bars)
            sk = sk[:2]  # 截到 <3 笔
        else:
            sk = strokes
        py_segs = py_segments(list(sk), min_seg_strokes=3, extend_mode="strict")
        rs_segs = newchan_rust.segments_from_strokes_v1(
            [_stroke_to_tuple(s) for s in sk], 3, "strict"
        )
        _assert_segments_equal(py_segs, rs_segs, f"tiny[n={len(sk)}]")


# ── 真实数据（L2）──

_BZ_PARQUET = ".cache/BZ_1min_2024_raw.parquet"


def _load_bz_bars():
    import os

    if not os.path.exists(_BZ_PARQUET):
        pytest.skip(f"真实数据不存在: {_BZ_PARQUET}")
    import pandas as pd

    df = pd.read_parquet(_BZ_PARQUET)
    return list(
        zip(
            df["open"].tolist(),
            df["high"].tolist(),
            df["low"].tolist(),
            df["close"].tolist(),
        )
    )


@pytest.mark.slow
@pytest.mark.parametrize("extend_mode", ["strict", "optimized"])
def test_segment_bitexact_real(extend_mode: str) -> None:
    """BZ 真实数据：BiEngine → strokes → segments 全量逐字段 bit-exact（L2）。"""
    bars = _load_bz_bars()
    strokes = _strokes_from_bars(bars)
    assert len(strokes) > 1000, "真实数据规模不足以构成 L2 segment 验证"

    py_segs = py_segments(strokes, min_seg_strokes=3, extend_mode=extend_mode)
    rs_segs = newchan_rust.segments_from_strokes_v1(
        [_stroke_to_tuple(s) for s in strokes], 3, extend_mode
    )

    _assert_segments_equal(py_segs, rs_segs, f"real_BZ[{extend_mode}]")
    assert len(py_segs) > 100, "真实数据线段数不足，L2 验证无效"
