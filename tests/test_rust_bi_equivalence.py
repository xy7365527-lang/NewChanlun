"""Rust bi 引擎 ↔ Python bi 引擎 逐位等价 golden 测试。

验证 `newchan_rust.BiEngine` 在逐 bar 流式驱动下，其 strokes 快照与 Python
`newchan.bi_engine.BiEngine` 的 `BiEngineSnapshot.strokes` **逐位等价**。

## 认识论等级（formalization-validity-domain 规则）

- `test_bitexact_small_full_perbar`：合成确定性数据，**每 bar 全量** bit-exact 对比。
  这是 **L1**（管线正确性，输入自造）——验证 Rust 移植无逻辑 bug，信息增量在于
  捕捉每一个中间快照的发散。
- `test_bitexact_large_real_streaming`：BZ Brent 原油真实 1min 数据全量（~51 万 bars），
  **每 bar 指纹（len + 末笔 bit-exact）+ 周期性全量 + 最终全量** bit-exact 对比。
  这是 **L2**（真实数据，可证伪）。

## 验证粒度的权衡（严格声明）

字面"每 bar 全量对比"在 51 万 bars 上是 O(N²)（小时级），不可行。大数据测试采用
**指纹 + 周期全量** 策略。其有效域依赖一个**已陈述的前提**：笔引擎是前缀单调的
确定性增量，任何中间笔发散都会传播并改变末笔身份或列表长度，故"每 bar 末笔 bit-exact
+ 周期全量 + 最终全量"在实践上覆盖每-bar全量。该前提对前缀单调的增量引擎成立；
小数据测试用字面每-bar全量提供独立保险。
"""

from __future__ import annotations

import math

import pytest

from newchan.bi_engine import BiEngine as PyBiEngine
from newchan.core.bar import BarV1

newchan_rust = pytest.importorskip("newchan_rust")


# ── stroke 对比（bit-exact）──


def _stroke_eq(py_stroke, rs_tuple) -> bool:
    """Python Stroke 与 Rust tuple 逐位相等。

    整数/字符串/bool 用 `==`；浮点用 `==`（真正的 bit-exact，非容差）——
    逐位等价要求 high/low/p0/p1 完全相同，不是"接近"。
    """
    return (
        py_stroke.i0 == rs_tuple[0]
        and py_stroke.i1 == rs_tuple[1]
        and py_stroke.direction == rs_tuple[2]
        and py_stroke.high == rs_tuple[3]
        and py_stroke.low == rs_tuple[4]
        and py_stroke.p0 == rs_tuple[5]
        and py_stroke.p1 == rs_tuple[6]
        and py_stroke.confirmed == rs_tuple[7]
    )


def _assert_full_equal(py_strokes, rs_strokes, bar_k: int) -> None:
    """断言整列 strokes 逐位相等，失败时给出首个分歧的精确位置。"""
    assert len(py_strokes) == len(rs_strokes), (
        f"bar {bar_k}: 笔数不等 py={len(py_strokes)} rust={len(rs_strokes)}"
    )
    for idx, (a, b) in enumerate(zip(py_strokes, rs_strokes)):
        assert _stroke_eq(a, b), (
            f"bar {bar_k}: 第 {idx} 笔分歧\n"
            f"  py  ={(a.i0, a.i1, a.direction, a.high, a.low, a.p0, a.p1, a.confirmed)}\n"
            f"  rust={b}"
        )


# ── 合成确定性数据（L1）──


def _synthetic_bars(n: int) -> list[tuple[float, float, float, float]]:
    """确定性合成 OHLC：多频正弦叠加，产生丰富的分型/笔结构。无随机性。"""
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


@pytest.mark.parametrize("mode", ["new", "wide", "strict"])
def test_bitexact_small_full_perbar(mode: str) -> None:
    """合成数据，每 bar 全量 bit-exact 对比（L1，三种笔模式）。"""
    bars = _synthetic_bars(6000)
    ts0 = 1_700_000_000.0

    py = PyBiEngine(stroke_mode=mode)
    rs = newchan_rust.BiEngine(stroke_mode=mode)

    for k, (o, h, l, c) in enumerate(bars):
        snap = py.process_bar(
            BarV1(bar_time=ts0 + k * 60, open=o, high=h, low=l, close=c)
        )
        rs.process_bar(o, h, l, c)
        _assert_full_equal(snap.strokes, rs.current_strokes(), k)

    # 至少要产生一些笔，否则测试是空验证
    assert len(py.current_strokes) > 5, "合成数据未产生足够的笔，测试无效"


# ── 真实数据（L2）──

_BZ_PARQUET = ".cache/BZ_1min_2024_raw.parquet"


def _load_bz_bars():
    import os

    if not os.path.exists(_BZ_PARQUET):
        pytest.skip(f"真实数据不存在: {_BZ_PARQUET}")
    import pandas as pd

    df = pd.read_parquet(_BZ_PARQUET)
    return (
        df["open"].tolist(),
        df["high"].tolist(),
        df["low"].tolist(),
        df["close"].tolist(),
        [t.timestamp() for t in df.index],
    )


@pytest.mark.slow
def test_bitexact_large_real_streaming() -> None:
    """BZ 真实数据全量，每 bar 指纹 + 周期全量 + 最终全量 bit-exact（L2）。"""
    o, h, l, c, ts = _load_bz_bars()
    n = len(o)

    py = PyBiEngine(stroke_mode="new")
    rs = newchan_rust.BiEngine(stroke_mode="new")

    full_check_every = 25_000

    for k in range(n):
        snap = py.process_bar(
            BarV1(bar_time=ts[k], open=o[k], high=h[k], low=l[k], close=c[k])
        )
        rs.process_bar(o[k], h[k], l[k], c[k])
        pys = snap.strokes
        rss = rs.current_strokes()

        # 每 bar O(1) 指纹：长度 + 末笔 bit-exact
        assert len(pys) == len(rss), (
            f"bar {k}: 笔数不等 py={len(pys)} rust={len(rss)}"
        )
        if pys:
            assert _stroke_eq(pys[-1], rss[-1]), f"bar {k}: 末笔分歧"

        # 周期性全量
        if k % full_check_every == 0:
            _assert_full_equal(pys, rss, k)

    # 最终全量
    _assert_full_equal(py.current_strokes, rs.current_strokes(), n - 1)
    assert n > 100_000, "真实数据规模不足以构成 L2 验证"
