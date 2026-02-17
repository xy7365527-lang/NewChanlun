"""特征序列构建与包含处理 — 单元测试

覆盖 src/newchan/a_feature_sequence.py 全部公开 API：
  A) FeatureBar 数据类基本构造
  B) build_feature_sequence：正常序列构建、空输入、单笔输入
  C) merge_inclusion_feature：无包含关系、包含合并逻辑、连续包含
"""

from typing import Literal

import pytest

from newchan.a_feature_sequence import FeatureBar, build_feature_sequence, merge_inclusion_feature
from newchan.a_stroke import Stroke


# ── 通用 helper ──────────────────────────────────────────────────────

def _stroke(
    i0: int,
    i1: int,
    direction: Literal["up", "down"],
    high: float,
    low: float,
) -> Stroke:
    """快速构造 Stroke，p0/p1 自动填充。"""
    if direction == "up":
        p0, p1 = low, high
    else:
        p0, p1 = high, low
    return Stroke(i0=i0, i1=i1, direction=direction, high=high, low=low, p0=p0, p1=p1, confirmed=True)


# =====================================================================
# A) FeatureBar 数据类
# =====================================================================

class TestFeatureBar:
    """FeatureBar frozen dataclass 基本行为。"""

    def test_basic_construction(self):
        fb = FeatureBar(idx=3, high=10.0, low=5.0)
        assert fb.idx == 3
        assert fb.high == 10.0
        assert fb.low == 5.0

    def test_frozen(self):
        fb = FeatureBar(idx=0, high=10.0, low=5.0)
        with pytest.raises(AttributeError):
            fb.high = 20.0  # type: ignore[misc]

    def test_equality(self):
        a = FeatureBar(idx=1, high=8.0, low=3.0)
        b = FeatureBar(idx=1, high=8.0, low=3.0)
        assert a == b

    def test_inequality(self):
        a = FeatureBar(idx=1, high=8.0, low=3.0)
        b = FeatureBar(idx=2, high=8.0, low=3.0)
        assert a != b


# =====================================================================
# B) build_feature_sequence
# =====================================================================

class TestBuildFeatureSequence:
    """从笔序列中提取反向笔作为特征序列。"""

    def test_empty_strokes(self):
        result = build_feature_sequence([], start_s=0, end_s=0, direction="up")
        assert result == []

    def test_single_up_stroke_direction_up(self):
        """向上段取 down 笔；只有一根 up 笔时结果为空。"""
        strokes = [_stroke(0, 4, "up", 20.0, 10.0)]
        result = build_feature_sequence(strokes, start_s=0, end_s=0, direction="up")
        assert result == []

    def test_single_down_stroke_direction_up(self):
        """向上段取 down 笔；单根 down 笔应被提取。"""
        strokes = [_stroke(0, 4, "down", 20.0, 10.0)]
        result = build_feature_sequence(strokes, start_s=0, end_s=0, direction="up")
        assert len(result) == 1
        assert result[0] == FeatureBar(idx=0, high=20.0, low=10.0)

    def test_up_segment_extracts_down_strokes(self):
        """向上段：提取所有 down 笔（奇数位置笔）。"""
        strokes = [
            _stroke(0, 4, "up", 15.0, 10.0),    # idx=0, up  → 跳过
            _stroke(4, 8, "down", 14.0, 9.0),    # idx=1, down → 提取
            _stroke(8, 12, "up", 18.0, 12.0),    # idx=2, up  → 跳过
            _stroke(12, 16, "down", 17.0, 11.0),  # idx=3, down → 提取
            _stroke(16, 20, "up", 22.0, 15.0),    # idx=4, up  → 跳过
        ]
        result = build_feature_sequence(strokes, start_s=0, end_s=4, direction="up")
        assert len(result) == 2
        assert result[0] == FeatureBar(idx=1, high=14.0, low=9.0)
        assert result[1] == FeatureBar(idx=3, high=17.0, low=11.0)

    def test_down_segment_extracts_up_strokes(self):
        """向下段：提取所有 up 笔。"""
        strokes = [
            _stroke(0, 4, "down", 20.0, 15.0),   # idx=0, down → 跳过
            _stroke(4, 8, "up", 18.0, 13.0),      # idx=1, up   → 提取
            _stroke(8, 12, "down", 16.0, 10.0),   # idx=2, down → 跳过
            _stroke(12, 16, "up", 14.0, 9.0),     # idx=3, up   → 提取
            _stroke(16, 20, "down", 12.0, 7.0),   # idx=4, down → 跳过
        ]
        result = build_feature_sequence(strokes, start_s=0, end_s=4, direction="down")
        assert len(result) == 2
        assert result[0] == FeatureBar(idx=1, high=18.0, low=13.0)
        assert result[1] == FeatureBar(idx=3, high=14.0, low=9.0)

    def test_subrange_selection(self):
        """start_s / end_s 限定范围只提取子区间内的反向笔。"""
        strokes = [
            _stroke(0, 4, "up", 15.0, 10.0),      # idx=0
            _stroke(4, 8, "down", 14.0, 9.0),      # idx=1
            _stroke(8, 12, "up", 18.0, 12.0),      # idx=2
            _stroke(12, 16, "down", 17.0, 11.0),   # idx=3
            _stroke(16, 20, "up", 22.0, 15.0),      # idx=4
        ]
        # 只看 idx 2..3
        result = build_feature_sequence(strokes, start_s=2, end_s=3, direction="up")
        # idx=2 是 up → 跳过；idx=3 是 down → 提取
        assert len(result) == 1
        assert result[0].idx == 3

    def test_end_s_beyond_length(self):
        """end_s 超出 strokes 长度时安全截断。"""
        strokes = [
            _stroke(0, 4, "down", 20.0, 10.0),
        ]
        result = build_feature_sequence(strokes, start_s=0, end_s=100, direction="up")
        assert len(result) == 1
        assert result[0].idx == 0


# =====================================================================
# C) merge_inclusion_feature
# =====================================================================

class TestMergeInclusionFeature:
    """特征序列包含处理 → 标准特征序列。"""

    def test_empty_input(self):
        merged, mapping = merge_inclusion_feature([])
        assert merged == []
        assert mapping == []

    def test_single_element(self):
        seq = [FeatureBar(idx=1, high=10.0, low=5.0)]
        merged, mapping = merge_inclusion_feature(seq)
        assert len(merged) == 1
        assert merged[0] == seq[0]
        assert mapping == [(0, 0)]

    def test_no_inclusion(self):
        """两个元素无包含关系，各自保留。"""
        seq = [
            FeatureBar(idx=1, high=10.0, low=5.0),
            FeatureBar(idx=3, high=15.0, low=8.0),
        ]
        merged, mapping = merge_inclusion_feature(seq)
        assert len(merged) == 2
        assert merged[0] == seq[0]
        assert merged[1] == seq[1]
        assert mapping == [(0, 0), (1, 1)]

    def test_left_inclusion_up_direction(self):
        """左包含右（last 包含 curr）：方向默认 UP → 取 max(high), max(low)。"""
        seq = [
            FeatureBar(idx=1, high=20.0, low=5.0),   # 包含下面的
            FeatureBar(idx=3, high=15.0, low=8.0),   # 被包含
        ]
        merged, mapping = merge_inclusion_feature(seq)
        assert len(merged) == 1
        # UP: max(20, 15)=20, max(5, 8)=8
        assert merged[0].high == 20.0
        assert merged[0].low == 8.0
        assert merged[0].idx == 3  # idx 保留最后一个 stroke idx
        assert mapping == [(0, 1)]

    def test_right_inclusion_up_direction(self):
        """右包含左（curr 包含 last）：方向默认 UP → 取 max(high), max(low)。"""
        seq = [
            FeatureBar(idx=1, high=15.0, low=8.0),   # 被包含
            FeatureBar(idx=3, high=20.0, low=5.0),   # 包含左边的
        ]
        merged, mapping = merge_inclusion_feature(seq)
        assert len(merged) == 1
        # UP: max(15, 20)=20, max(8, 5)=8
        assert merged[0].high == 20.0
        assert merged[0].low == 8.0
        assert mapping == [(0, 1)]

    def test_inclusion_down_direction(self):
        """先建立 DOWN 方向，再触发包含，验证取 min。"""
        seq = [
            FeatureBar(idx=1, high=20.0, low=10.0),
            FeatureBar(idx=3, high=15.0, low=5.0),   # h↓ l↓ → DOWN
            FeatureBar(idx=5, high=18.0, low=3.0),   # 包含 idx=3（18>15, 3<5）
        ]
        merged, mapping = merge_inclusion_feature(seq)
        # 第一对 idx=1→idx=3: h↓ l↓ → dir_state=DOWN，无包含
        # 第二对 idx=3→idx=5: right_inc（18>=15 且 3<=5） → DOWN 取 min
        # min(15,18)=15, min(5,3)=3
        assert len(merged) == 2
        assert merged[0] == FeatureBar(idx=1, high=20.0, low=10.0)
        assert merged[1].high == 15.0
        assert merged[1].low == 3.0
        assert mapping == [(0, 0), (1, 2)]

    def test_consecutive_inclusions(self):
        """连续三个包含关系，应全部合并为一个元素。"""
        seq = [
            FeatureBar(idx=1, high=20.0, low=2.0),   # 最大范围
            FeatureBar(idx=3, high=18.0, low=4.0),   # 被 idx=1 包含
            FeatureBar(idx=5, high=16.0, low=6.0),   # 被合并后元素包含
        ]
        merged, mapping = merge_inclusion_feature(seq)
        assert len(merged) == 1
        # 默认 UP: 连续取 max(high), max(low)
        # 第一次合并: max(20,18)=20, max(2,4)=4 → (20, 4)
        # 第二次合并: max(20,16)=20, max(4,6)=6 → (20, 6)
        assert merged[0].high == 20.0
        assert merged[0].low == 6.0
        assert merged[0].idx == 5
        assert mapping == [(0, 2)]

    def test_mixed_inclusion_and_non_inclusion(self):
        """混合场景：部分包含、部分不包含。"""
        seq = [
            FeatureBar(idx=1, high=10.0, low=5.0),
            FeatureBar(idx=3, high=15.0, low=8.0),   # 无包含，h↑ l↑ → UP
            FeatureBar(idx=5, high=14.0, low=9.0),   # 被 idx=3 包含（15>=14, 8<=9）
            FeatureBar(idx=7, high=20.0, low=12.0),  # 无包含
        ]
        merged, mapping = merge_inclusion_feature(seq)
        # idx=1 → 独立
        # idx=3 与 idx=1: 无包含, h↑ l↑ → UP
        # idx=5 与 idx=3(当前buf末尾): left_inc(15>=14 且 8<=9) → UP 合并
        #   → max(15,14)=15, max(8,9)=9
        # idx=7 与 合并结果(15,9): 无包含, h↑ l↑ → UP
        assert len(merged) == 3
        assert merged[0] == FeatureBar(idx=1, high=10.0, low=5.0)
        assert merged[1].high == 15.0
        assert merged[1].low == 9.0
        assert merged[2] == FeatureBar(idx=7, high=20.0, low=12.0)
        assert mapping == [(0, 0), (1, 2), (3, 3)]

    def test_direction_switch_within_sequence(self):
        """序列内方向从 UP 切换到 DOWN，包含处理方式随之变化。"""
        seq = [
            FeatureBar(idx=1, high=10.0, low=5.0),
            FeatureBar(idx=3, high=15.0, low=8.0),   # h↑ l↑ → UP
            FeatureBar(idx=5, high=20.0, low=12.0),  # h↑ l↑ → UP
            FeatureBar(idx=7, high=16.0, low=9.0),   # h↓ l↓ → DOWN
            FeatureBar(idx=9, high=18.0, low=7.0),   # 包含 idx=7（18>16, 7<9）
        ]
        merged, mapping = merge_inclusion_feature(seq)
        # idx=1..5: 无包含, 各自独立, dir UP→UP
        # idx=7 与 idx=5(20,12): h↓ l↓ → DOWN, 无包含
        # idx=9 与 idx=7(16,9): right_inc(18>=16, 7<=9) → DOWN 取 min
        #   → min(16,18)=16, min(9,7)=7
        assert len(merged) == 4
        assert merged[3].high == 16.0
        assert merged[3].low == 7.0
        assert mapping[3] == (3, 4)

    def test_exact_equality_is_inclusion(self):
        """high 和 low 完全相同视为包含关系。"""
        seq = [
            FeatureBar(idx=1, high=10.0, low=5.0),
            FeatureBar(idx=3, high=10.0, low=5.0),  # 完全相同 → 包含
        ]
        merged, mapping = merge_inclusion_feature(seq)
        assert len(merged) == 1
        assert mapping == [(0, 1)]
