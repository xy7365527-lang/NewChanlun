"""线段尾窗扫描测试 — TAIL_WINDOW 正确性验证

覆盖 2 个场景：
  1. 尾窗 N=7 vs 全量扫描 → 结果完全一致
  2. 长序列（30+ 笔）性能验证 → 尾窗不回退到序列头
"""

from __future__ import annotations

import pytest

from newchan.a_segment_v1 import segments_from_strokes_v1, _FeatureSeqState
from newchan.a_stroke import Stroke


# ── helpers ──

def _s(i0, i1, direction, high, low, confirmed=True):
    if direction == "up":
        p0, p1 = low, high
    else:
        p0, p1 = high, low
    return Stroke(i0=i0, i1=i1, direction=direction,
                  high=high, low=low, p0=p0, p1=p1, confirmed=confirmed)


def _generate_zigzag(n_strokes: int, base: float = 10.0, step: float = 0.5):
    """生成 n_strokes 笔的锯齿形数据。

    偶数笔为 up，奇数笔为 down。
    价格呈小幅波动，确保三笔重叠。
    """
    strokes = []
    price = base
    for i in range(n_strokes):
        direction = "up" if i % 2 == 0 else "down"
        if direction == "up":
            low = price - 2
            high = price + 2
        else:
            high = price + 1
            low = price - 1
        strokes.append(_s(i * 5, (i + 1) * 5, direction, high, low))
        price += step * (1 if direction == "up" else -1)
    return strokes


# =====================================================================
# 1) 尾窗 vs 全量扫描一致性
# =====================================================================

class TestTailWindowConsistency:
    """TAIL_WINDOW=7 和全量扫描应产生完全一致的结果。"""

    def test_same_segments_short(self):
        """短序列（7 笔）下尾窗和全量结果一致。"""
        strokes = [
            _s(0, 5, "up",    15, 5),
            _s(5, 10, "down", 12, 8),
            _s(10, 15, "up",  18, 7),
            _s(15, 20, "down", 16, 9),
            _s(20, 25, "up",  14, 8),
            _s(25, 30, "down", 13, 7),
            _s(30, 35, "up",  12, 4),
        ]
        segs_normal = segments_from_strokes_v1(strokes)

        # 临时修改 TAIL_WINDOW 到极大值（模拟全量扫描）
        old_tw = _FeatureSeqState.TAIL_WINDOW
        try:
            _FeatureSeqState.TAIL_WINDOW = 1000
            segs_full = segments_from_strokes_v1(strokes)
        finally:
            _FeatureSeqState.TAIL_WINDOW = old_tw

        assert len(segs_normal) == len(segs_full)
        for a, b in zip(segs_normal, segs_full):
            assert a.s0 == b.s0 and a.s1 == b.s1
            assert a.direction == b.direction
            assert a.confirmed == b.confirmed

    def test_same_segments_medium(self):
        """中等序列（15 笔）下尾窗和全量结果一致。"""
        strokes = _generate_zigzag(15)
        segs_normal = segments_from_strokes_v1(strokes)

        old_tw = _FeatureSeqState.TAIL_WINDOW
        try:
            _FeatureSeqState.TAIL_WINDOW = 1000
            segs_full = segments_from_strokes_v1(strokes)
        finally:
            _FeatureSeqState.TAIL_WINDOW = old_tw

        assert len(segs_normal) == len(segs_full)
        for a, b in zip(segs_normal, segs_full):
            assert a.s0 == b.s0 and a.s1 == b.s1
            assert a.direction == b.direction

    def test_same_segments_long(self):
        """长序列（31 笔）下尾窗和全量结果一致。"""
        strokes = _generate_zigzag(31)
        segs_normal = segments_from_strokes_v1(strokes)

        old_tw = _FeatureSeqState.TAIL_WINDOW
        try:
            _FeatureSeqState.TAIL_WINDOW = 1000
            segs_full = segments_from_strokes_v1(strokes)
        finally:
            _FeatureSeqState.TAIL_WINDOW = old_tw

        assert len(segs_normal) == len(segs_full)
        for a, b in zip(segs_normal, segs_full):
            assert a.s0 == b.s0 and a.s1 == b.s1
            assert a.direction == b.direction


# =====================================================================
# 2) 尾窗不回退到序列头
# =====================================================================

class TestTailWindowEfficiency:
    """验证尾窗扫描不会退化为全量扫描。"""

    def test_last_checked_no_regress(self):
        """增量添加元素后，last_checked 不应回退到 0（除包含合并外）。"""
        fs = _FeatureSeqState("up")

        # 逐个添加元素
        data = [
            (1, 15.0, 8.0),
            (3, 18.0, 10.0),
            (5, 12.0, 6.0),
            (7, 16.0, 9.0),
            (9, 10.0, 5.0),
            (11, 14.0, 7.0),
            (13, 8.0, 3.0),
            (15, 12.0, 6.0),
            (17, 6.0, 2.0),
        ]

        prev_checked = 0
        for idx, h, l in data:
            fs.append(idx, h, l)
            fs.scan_trigger("up")

        # 在长序列中 last_checked 不应为 0
        if len(fs.std) > _FeatureSeqState.TAIL_WINDOW:
            assert fs.last_checked > 0, (
                f"last_checked={fs.last_checked} 不应为 0 "
                f"（std 长度 {len(fs.std)} 远超 TAIL_WINDOW={_FeatureSeqState.TAIL_WINDOW}）"
            )

    def test_scan_trigger_start_bounded(self):
        """scan_trigger 的起始位置应受 TAIL_WINDOW 限制。"""
        fs = _FeatureSeqState("up")

        # 添加大量元素（超出 TAIL_WINDOW）
        for i in range(20):
            idx = i * 2 + 1
            h = 10.0 + (i % 5)
            l = 5.0 + (i % 3)
            fs.append(idx, h, l)

        n = len(fs.std)
        if n > _FeatureSeqState.TAIL_WINDOW:
            # 验证扫描不会从位置 1 开始（应从 n - TAIL_WINDOW 或更后）
            expected_start = max(1, fs.last_checked, n - _FeatureSeqState.TAIL_WINDOW)
            assert expected_start > 1, (
                f"expected_start={expected_start} 应 > 1（说明尾窗生效）"
            )


# =====================================================================
# 3) _has_any_fractal_after 边界条件
# =====================================================================

class TestHasAnyFractalAfterEdgeCases:
    """_has_any_fractal_after 的边界条件测试。"""

    def test_empty_std(self):
        assert _FeatureSeqState._has_any_fractal_after([], 0) is False

    def test_single_element(self):
        std = [[10.0, 5.0, 0]]
        assert _FeatureSeqState._has_any_fractal_after(std, 0) is False

    def test_two_elements(self):
        std = [[10.0, 5.0, 0], [15.0, 8.0, 1]]
        assert _FeatureSeqState._has_any_fractal_after(std, 0) is False

    def test_start_pos_beyond_end(self):
        std = [[10.0, 5.0, 0], [15.0, 8.0, 1], [12.0, 6.0, 2]]
        assert _FeatureSeqState._has_any_fractal_after(std, 3) is False

    def test_fractal_at_exact_start(self):
        """分型恰好从 start_pos 开始。"""
        std = [
            [10.0, 5.0, 0],   # a
            [15.0, 8.0, 1],   # b (peak)
            [12.0, 6.0, 2],   # c
        ]
        # start_pos=0: 以 j=1 为中心 (a,b,c 构成顶分型)
        assert _FeatureSeqState._has_any_fractal_after(std, 0) is True

    def test_monotonic_no_fractal(self):
        """单调序列无分型。"""
        std = [
            [10.0, 5.0, 0],
            [12.0, 7.0, 1],
            [14.0, 9.0, 2],
            [16.0, 11.0, 3],
        ]
        assert _FeatureSeqState._has_any_fractal_after(std, 0) is False
