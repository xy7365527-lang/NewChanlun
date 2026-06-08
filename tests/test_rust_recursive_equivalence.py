"""Rust 递归编排器 ↔ Python 递归编排器 逐位等价 golden 测试。

验证 `newchan_rust.RecursiveOrchestrator` 在逐 bar 流式驱动下，其**结构化快照**
（strokes / segments / zhongshus / moves / buysellpoints / 递归级别 zhongshus+moves）
与 Python `newchan.orchestrator.recursive.RecursiveOrchestrator.process_bar` 返回的
`RecursiveOrchestratorSnapshot` 对应字段 **逐位等价**。

## 移植边界（严格声明，与 bi_engine.rs / orchestrator.rs 一致）

比对范围是**结构化列表**，不含 DomainEvent 流。events 是 diff 的纯副产物，不参与
状态递归（下游引擎只读 snapshot.moves 不读 .events），与既有 parity 测试（比对列表
非事件）契约一致。

## 认识论等级（formalization-validity-domain 规则）

- `test_bitexact_synthetic_full_perbar`：合成确定性数据，**每 bar 全量** bit-exact
  对比全部六层。L1（管线正确性，输入自造）——捕捉每一中间快照的发散。
- `test_bitexact_large_real_streaming`：BZ Brent 原油真实 1min 全量，**每 bar 指纹 +
  周期全量 + 最终全量**。L2（真实数据，可证伪）。

## bit-exact 基础

递归编排无新增浮点算术——逐层只是比较 + max/min 选择 + 减法（persistence 极差）。
所有底层核（bi/segment/zhongshu/move/divergence/bsp/ph/level）已各自 bit-exact 验证；
本测试验证**组合层**（短路缓存 + 递归栈 + 级别引擎状态机）无逻辑发散。
"""

from __future__ import annotations

import math

import pytest

from newchan.core.bar import BarV1
from newchan.orchestrator.recursive import RecursiveOrchestrator as PyOrch

newchan_rust = pytest.importorskip("newchan_rust")


# ════════════════════════════════════════════════════════════
# 逐层比较器（浮点 bit-exact，整数/字符串/bool 用 ==）
# ════════════════════════════════════════════════════════════


def _stroke_eq(s, rs) -> bool:
    return (
        s.i0 == rs[0]
        and s.i1 == rs[1]
        and s.direction == rs[2]
        and s.high == rs[3]
        and s.low == rs[4]
        and s.p0 == rs[5]
        and s.p1 == rs[6]
        and s.confirmed == rs[7]
    )


def _be_eq(py_be, rs_be) -> bool:
    if py_be is None:
        return rs_be is None
    if rs_be is None:
        return False
    return (
        py_be.trigger_stroke_k == rs_be[0]
        and tuple(py_be.fractal_abc) == tuple(rs_be[1])
        and py_be.gap_type == rs_be[2]
    )


def _segment_eq(s, rs) -> bool:
    head, ep, be = rs
    return (
        s.s0 == head[0]
        and s.s1 == head[1]
        and s.i0 == head[2]
        and s.i1 == head[3]
        and s.direction == head[4]
        and s.high == head[5]
        and s.low == head[6]
        and s.confirmed == head[7]
        and s.kind == head[8]
        and s.ep0_i == ep[0]
        and s.ep0_price == ep[1]
        and s.ep0_type == ep[2]
        and s.ep1_i == ep[3]
        and s.ep1_price == ep[4]
        and s.ep1_type == ep[5]
        and s.p0 == ep[6]
        and s.p1 == ep[7]
        and _be_eq(s.break_evidence, be)
    )


def _zs_eq(z, rs) -> bool:
    return (
        z.zd == rs[0]
        and z.zg == rs[1]
        and z.seg_start == rs[2]
        and z.seg_end == rs[3]
        and z.seg_count == rs[4]
        and z.settled == rs[5]
        and z.break_seg == rs[6]
        and z.break_direction == rs[7]
        and z.first_seg_s0 == rs[8]
        and z.last_seg_s1 == rs[9]
        and z.gg == rs[10]
        and z.dd == rs[11]
    )


def _move_eq(m, rs) -> bool:
    head, tail = rs
    return (
        m.kind == head[0]
        and m.direction == head[1]
        and m.seg_start == head[2]
        and m.seg_end == head[3]
        and m.zs_start == head[4]
        and m.zs_end == head[5]
        and m.zs_count == head[6]
        and m.settled == head[7]
        and m.high == tail[0]
        and m.low == tail[1]
        and m.first_seg_s0 == tail[2]
        and m.last_seg_s1 == tail[3]
        and m.zg_max == tail[4]
        and m.zd_min == tail[5]
        and m.persistence == tail[6]
    )


def _bsp_eq(b, rs) -> bool:
    head, mid, div_key, center_seg_start, overlaps = rs
    return (
        b.kind == head[0]
        and b.side == head[1]
        and b.level_id == head[2]
        and b.seg_idx == head[3]
        and b.move_seg_start == head[4]
        and b.confirmed == head[5]
        and b.settled == head[6]
        and b.center_zd == mid[0]
        and b.center_zg == mid[1]
        and b.price == mid[2]
        and b.bar_idx == mid[3]
        and b.divergence_key == div_key
        and b.center_seg_start == center_seg_start
        and b.overlaps_with == overlaps
    )


def _level_zs_eq(z, rs) -> bool:
    # rs = (zd, zg, comp_start, comp_end, comp_count, settled, break_comp,
    #       break_direction, gg, dd, level_id)
    return (
        z.zd == rs[0]
        and z.zg == rs[1]
        and z.comp_start == rs[2]
        and z.comp_end == rs[3]
        and z.comp_count == rs[4]
        and z.settled == rs[5]
        and z.break_comp == rs[6]
        and z.break_direction == rs[7]
        and z.gg == rs[8]
        and z.dd == rs[9]
        and z.level_id == rs[10]
    )


def _assert_list_eq(py_list, rs_list, eq, ctx: str) -> None:
    assert len(py_list) == len(rs_list), (
        f"{ctx}: 长度不等 py={len(py_list)} rust={len(rs_list)}"
    )
    for idx, (a, b) in enumerate(zip(py_list, rs_list)):
        assert eq(a, b), f"{ctx}: 第 {idx} 项分歧\n  rust={b}"


def _assert_full_equal(py_snap, rs, bar_k: int) -> None:
    """断言一个 bar 的全部六层结构化快照逐位相等。"""
    ctx = f"bar {bar_k}"
    _assert_list_eq(py_snap.bi_snapshot.strokes, rs.current_strokes(), _stroke_eq, f"{ctx}/strokes")
    _assert_list_eq(py_snap.seg_snapshot.segments, rs.current_segments(), _segment_eq, f"{ctx}/segments")
    _assert_list_eq(py_snap.zs_snapshot.zhongshus, rs.current_zhongshus(), _zs_eq, f"{ctx}/zhongshus")
    _assert_list_eq(py_snap.move_snapshot.moves, rs.current_moves(), _move_eq, f"{ctx}/moves")
    _assert_list_eq(
        py_snap.bsp_snapshot.buysellpoints, rs.current_buysellpoints(), _bsp_eq, f"{ctx}/bsps"
    )

    py_rec = py_snap.recursive_snapshots
    rs_rec = rs.current_recursive()
    assert len(py_rec) == len(rs_rec), (
        f"{ctx}/recursive: 级别数不等 py={len(py_rec)} rust={len(rs_rec)}"
    )
    for li, (prs, rrs) in enumerate(zip(py_rec, rs_rec)):
        assert prs.level_id == rrs[0], (
            f"{ctx}/recursive[{li}]: level_id py={prs.level_id} rust={rrs[0]}"
        )
        _assert_list_eq(prs.zhongshus, rrs[1], _level_zs_eq, f"{ctx}/rec[{li}]/zhongshus")
        _assert_list_eq(prs.moves, rrs[2], _move_eq, f"{ctx}/rec[{li}]/moves")


def _fingerprint_equal(py_snap, rs, bar_k: int) -> None:
    """每 bar O(1) 指纹：各层长度 + 末元素 bit-exact。"""
    pairs = [
        (py_snap.bi_snapshot.strokes, rs.current_strokes(), _stroke_eq, "strokes"),
        (py_snap.seg_snapshot.segments, rs.current_segments(), _segment_eq, "segments"),
        (py_snap.zs_snapshot.zhongshus, rs.current_zhongshus(), _zs_eq, "zhongshus"),
        (py_snap.move_snapshot.moves, rs.current_moves(), _move_eq, "moves"),
        (py_snap.bsp_snapshot.buysellpoints, rs.current_buysellpoints(), _bsp_eq, "bsps"),
    ]
    for py_l, rs_l, eq, name in pairs:
        assert len(py_l) == len(rs_l), (
            f"bar {bar_k}/{name}: 长度不等 py={len(py_l)} rust={len(rs_l)}"
        )
        if py_l:
            assert eq(py_l[-1], rs_l[-1]), f"bar {bar_k}/{name}: 末元素分歧"
    # 递归层数 + 末级别 moves 长度指纹
    py_rec = py_snap.recursive_snapshots
    rs_rec = rs.current_recursive()
    assert len(py_rec) == len(rs_rec), (
        f"bar {bar_k}/recursive: 级别数不等 py={len(py_rec)} rust={len(rs_rec)}"
    )
    if py_rec:
        assert len(py_rec[-1].moves) == len(rs_rec[-1][2]), (
            f"bar {bar_k}/recursive: 末级别 moves 数不等"
        )


# ════════════════════════════════════════════════════════════
# 合成确定性数据（L1）
# ════════════════════════════════════════════════════════════


def _synthetic_bars(n: int):
    """含慢趋势分量使价格漂移、走出区间，产生独立中枢 → settled → moves → 递归。"""
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


@pytest.mark.parametrize("enable_macd", [False, True])
def test_bitexact_synthetic_full_perbar(enable_macd: bool) -> None:
    """合成数据，每 bar 全量 bit-exact 对比全部六层（L1，含/不含 MACD 背驰）。

    n=12000 使慢趋势分量走出足够多独立中枢，level-2 产生 ≥1 个泛化中枢 +
    ≥1 个递归走势（含 persistence），从而 bit-exact 验证递归核（zhongshu_from_
    components / moves_from_level_zhongshus / attach_persistence）。
    """
    n = 12000
    bars = _synthetic_bars(n)
    ts0 = 1_700_000_000.0

    py = PyOrch(stroke_mode="wide", enable_macd_divergence=enable_macd)
    rs = newchan_rust.RecursiveOrchestrator(
        stroke_mode="wide", enable_macd_divergence=enable_macd
    )

    saw_moves = False
    saw_recursive_zs = False
    saw_recursive_move = False
    for k, (o, h, l, c) in enumerate(bars):
        snap = py.process_bar(BarV1(bar_time=ts0 + k * 60, open=o, high=h, low=l, close=c))
        rs.process_bar(o, h, l, c)
        _assert_full_equal(snap, rs, k)
        if snap.move_snapshot.moves:
            saw_moves = True
        if snap.recursive_snapshots:
            if any(r.zhongshus for r in snap.recursive_snapshots):
                saw_recursive_zs = True
            if any(r.moves for r in snap.recursive_snapshots):
                saw_recursive_move = True

    assert saw_moves, "合成数据未产生任何 level-1 走势，测试无效"
    assert saw_recursive_zs, "合成数据未产生任何递归级别中枢，递归中枢核未被验证"
    assert saw_recursive_move, "合成数据未产生任何递归级别走势，递归走势核未被验证"


def test_reset_restores_initial_state() -> None:
    """reset 后重新驱动，结构化快照与首次一致（回放 seek 正确性）。"""
    bars = _synthetic_bars(1500)
    ts0 = 1_700_000_000.0
    rs = newchan_rust.RecursiveOrchestrator(stroke_mode="wide")
    for k, (o, h, l, c) in enumerate(bars):
        rs.process_bar(o, h, l, c)
    first_moves = rs.current_moves()
    first_segs = rs.current_segments()

    rs.reset()
    assert rs.current_strokes() == []
    assert rs.current_segments() == []
    assert rs.current_moves() == []
    assert rs.current_recursive() == []

    for k, (o, h, l, c) in enumerate(bars):
        rs.process_bar(o, h, l, c)
    assert rs.current_moves() == first_moves
    assert rs.current_segments() == first_segs


# ════════════════════════════════════════════════════════════
# 真实数据（L2）
# ════════════════════════════════════════════════════════════

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
    """BZ 真实数据全量，每 bar 指纹 + 周期全量 + 最终全量 bit-exact（L2）。

    ## canonical 参考（已记录的未提交回归，2026-06-08 发现）

    本测试对 **canonical 线段语义**（full 重算，= HEAD 行为）比对 Rust 编排器。
    Python 端通过 `_seg_engine._try_resume = lambda *_: None` 禁用 SegmentEngine
    的 resume 增量——因为当前**未提交**的 segment_engine.py resume 优化在真实
    数据上**与 full 不等价**（BZ 前 6000 bar：resume=40 段 vs full=42 段，从
    第 9 段起发散）。根因：resume 从"稳定"段断点续算产生的尾部分段 ≠ 全量
    （bisect 确认：禁 resume→42 正确；禁续扫优化仍 40，故 bug 在 resume 机制
    本身，非续扫 O(1) 校验）。

    Rust 编排器用 full 重算（= canonical = HEAD），故本测试验证 Rust 移植在真实
    数据上逐位等价于 canonical 管线。**Python resume 回归是独立于本移植的
    pre-existing WIP bug**，已上报（见会话结论），修复后可移除此 monkeypatch。
    """
    o, h, l, c, ts = _load_bz_bars()
    n = len(o)

    py = PyOrch(stroke_mode="wide")
    # 强制 canonical full 线段路径（绕过未提交 resume 回归——见 docstring）。
    py._seg_engine._try_resume = lambda *_a, **_k: None  # type: ignore[attr-defined]
    rs = newchan_rust.RecursiveOrchestrator(stroke_mode="wide")

    full_check_every = 25_000
    last_snap = None
    for k in range(n):
        snap = py.process_bar(
            BarV1(bar_time=ts[k], open=o[k], high=h[k], low=l[k], close=c[k])
        )
        rs.process_bar(o[k], h[k], l[k], c[k])
        _fingerprint_equal(snap, rs, k)
        if k % full_check_every == 0:
            _assert_full_equal(snap, rs, k)
        last_snap = snap

    _assert_full_equal(last_snap, rs, n - 1)
    assert n > 100_000, "真实数据规模不足以构成 L2 验证"
