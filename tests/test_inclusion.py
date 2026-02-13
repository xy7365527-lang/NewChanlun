"""包含关系处理 — 单元测试 (Step8)

手工构造 K 线样本，覆盖 docs/chan_spec.md §2 全部规则:
  - §2.1 包含判定
  - §2.2 向上合并（high=max, low=max）/ 向下合并（high=min, low=min）
  - §2.3 方向判定：双条件规则 + dir=None 默认 UP
  - §2.4 先左后右递推、连续包含递推合并、合并后无残留包含
  - merged_to_raw 映射正确（闭区间、覆盖全部原始行、单调递增）
  - assert_inclusion_no_residual 集成验证
"""

from __future__ import annotations

import pandas as pd
import pytest

from newchan.a_inclusion import merge_inclusion
from newchan.a_assertions import assert_inclusion_no_residual


# =====================================================================
# 主测试数据：10 根 K 线（同时覆盖向上/向下/链式合并）
# =====================================================================
#
# 逐 bar 推演（§2.3 双条件方向规则）：
#
# Bar 0: H=10, L=5   → buf[0]=(10,5)            dir=None
# Bar 1: H=12, L=7   → 无包含                   dir 更新: 12>10 AND 7>5 → UP
#                       buf[1]=(12,7)
# Bar 2: H=11, L=8   → 包含(12>=11, 7<=8)       dir=UP → 向上合并
#                       H=max(12,11)=12, L=max(7,8)=8
#                       buf[1]=(12,8) raw=(1,2)
# Bar 3: H=15, L=10  → 无包含                   dir 更新: 15>12 AND 10>8 → UP
#                       buf[2]=(15,10)
# Bar 4: H=13, L=9   → 无包含                   dir 更新: 13<15 AND 9<10 → DOWN
#                       buf[3]=(13,9)
# Bar 5: H=11, L=7   → 无包含                   dir 更新: 11<13 AND 7<9 → DOWN
#                       buf[4]=(11,7)
# Bar 6: H=10, L=8   → 包含(11>=10, 7<=8)       dir=DOWN → 向下合并
#                       H=min(11,10)=10, L=min(7,8)=7
#                       buf[4]=(10,7) raw=(5,6)
# Bar 7: H=12, L=9   → 无包含                   dir 更新: 12>10 AND 9>7 → UP
#                       buf[5]=(12,9)
# Bar 8: H=14, L=8   → 包含(14>=12, 8<=9)       dir=UP → 向上合并
#                       H=max(12,14)=14, L=max(9,8)=9
#                       buf[5]=(14,9) raw=(7,8)
# Bar 9: H=13, L=10  → 包含(14>=13, 9<=10)      dir=UP → 向上合并（链式）
#                       H=max(14,13)=14, L=max(9,10)=10
#                       buf[5]=(14,10) raw=(7,9)
#
# 最终 merged (6 根):
#  [0] H=10, L=5   raw=(0,0)
#  [1] H=12, L=8   raw=(1,2)  ← UP 合并
#  [2] H=15, L=10  raw=(3,3)
#  [3] H=13, L=9   raw=(4,4)
#  [4] H=10, L=7   raw=(5,6)  ← DOWN 合并
#  [5] H=14, L=10  raw=(7,9)  ← 链式 UP 合并 (3→1)

def _make_df() -> pd.DataFrame:
    """构造 10 根手工 K 线。"""
    return pd.DataFrame({
        "open":  [6,  8,  9, 11, 10,  8,  9, 10,  9, 11],
        "high":  [10, 12, 11, 15, 13, 11, 10, 12, 14, 13],
        "low":   [5,   7,  8, 10,  9,  7,  8,  9,  8, 10],
        "close": [9,  11, 10, 14, 12,  9,  9, 11, 13, 12],
    })


EXPECTED_HIGHS   = [10.0, 12.0, 15.0, 13.0, 10.0, 14.0]
EXPECTED_LOWS    = [5.0,   8.0, 10.0,  9.0,  7.0, 10.0]
EXPECTED_RAW_MAP = [(0, 0), (1, 2), (3, 3), (4, 4), (5, 6), (7, 9)]


# =====================================================================
# TestMergeInclusion — 核心正确性
# =====================================================================

class TestMergeInclusion:
    """merge_inclusion 核心测试（10 根 K 线主样本）。"""

    @pytest.fixture(autouse=True)
    def setup(self):
        self.df = _make_df()
        self.df_merged, self.raw_map = merge_inclusion(self.df)

    def test_merged_count(self):
        """合并后 K 线数量正确。"""
        assert len(self.df_merged) == 6

    def test_highs(self):
        """合并后 high 序列正确。"""
        assert self.df_merged["high"].tolist() == EXPECTED_HIGHS

    def test_lows(self):
        """合并后 low 序列正确。"""
        assert self.df_merged["low"].tolist() == EXPECTED_LOWS

    def test_raw_map(self):
        """merged_to_raw 映射与预期一致。"""
        assert self.raw_map == EXPECTED_RAW_MAP

    def test_raw_map_length(self):
        """merged_to_raw 长度与 merged 行数一致。"""
        assert len(self.raw_map) == len(self.df_merged)

    def test_no_residual_inclusion(self):
        """§2.4: 合并后相邻 K 线不再存在包含关系。"""
        highs = self.df_merged["high"].values
        lows = self.df_merged["low"].values
        for i in range(len(highs) - 1):
            h1, l1 = highs[i], lows[i]
            h2, l2 = highs[i + 1], lows[i + 1]
            assert not (h1 >= h2 and l1 <= l2), (
                f"残留包含(左包右): [{i}]=({h1},{l1}) vs [{i+1}]=({h2},{l2})")
            assert not (h2 >= h1 and l2 <= l1), (
                f"残留包含(右包左): [{i}]=({h1},{l1}) vs [{i+1}]=({h2},{l2})")

    def test_raw_map_covers_all(self):
        """映射区间覆盖原始全部行（0~n-1），不重叠不遗漏。"""
        covered = set()
        for start, end in self.raw_map:
            for j in range(start, end + 1):
                assert j not in covered, f"原始索引 {j} 被多次覆盖"
                covered.add(j)
        assert covered == set(range(len(self.df)))

    def test_raw_map_monotonic(self):
        """映射区间 start 严格递增。"""
        starts = [s for s, _ in self.raw_map]
        assert starts == sorted(starts)
        assert len(set(starts)) == len(starts), "start 有重复"


# =====================================================================
# TestDirectionRule — §2.3 双条件方向规则
# =====================================================================

class TestDirectionRule:
    """专门验证 §2.3 的方向判定逻辑。"""

    def test_up_direction_merge(self):
        """dir=UP 建立后，包含按向上合并（high=max, low=max）。"""
        # Bar0→Bar1: 12>10 AND 7>5 → dir=UP
        # Bar2 包含于 Bar1: up merge → H=max(12,11)=12, L=max(7,8)=8
        df = pd.DataFrame({
            "open":  [5, 7, 8],
            "high":  [10, 12, 11],
            "low":   [5, 7, 8],
            "close": [9, 11, 10],
        })
        m, r = merge_inclusion(df)
        assert len(m) == 2
        assert m["high"].iloc[1] == 12.0
        assert m["low"].iloc[1] == 8.0
        assert r == [(0, 0), (1, 2)]

    def test_down_direction_merge(self):
        """dir=DOWN 建立后，包含按向下合并（high=min, low=min）。"""
        # Bar0→Bar1: 13<15 AND 8<10 → dir=DOWN
        # Bar2 包含于 Bar1(13,8): 13>=12 AND 8<=9 → down merge
        # H=min(13,12)=12, L=min(8,9)=8
        df = pd.DataFrame({
            "open":  [12, 9, 8],
            "high":  [15, 13, 12],
            "low":   [10, 8, 9],
            "close": [13, 10, 10],
        })
        m, r = merge_inclusion(df)
        assert len(m) == 2
        assert m["high"].iloc[1] == 12.0
        assert m["low"].iloc[1] == 8.0
        assert r == [(0, 0), (1, 2)]

    def test_dir_none_defaults_up(self):
        """§2.3: dir=None 时遇到包含，默认按 UP 合并。"""
        # Bar0 和 Bar1 直接包含（从未出现无包含对来确定 dir）
        # dir=None → 按 UP: H=max(20,19)=20, L=max(1,2)=2
        df = pd.DataFrame({
            "open":  [1, 2],
            "high":  [20, 19],
            "low":   [1, 2],
            "close": [10, 10],
        })
        m, _ = merge_inclusion(df)
        assert len(m) == 1
        assert m["high"].iloc[0] == 20.0
        assert m["low"].iloc[0] == 2.0  # max(1,2)=2, 不是 min

    def test_chain_default_up(self):
        """dir=None 全程包含链 → 持续按 UP 合并。"""
        df = pd.DataFrame({
            "open":  [1, 2, 3, 4],
            "high":  [20, 19, 18, 17],
            "low":   [1, 2, 3, 4],
            "close": [10, 10, 10, 10],
        })
        m, r = merge_inclusion(df)
        assert len(m) == 1
        assert m["high"].iloc[0] == 20.0
        assert m["low"].iloc[0] == 4.0   # max(1,2,3,4)
        assert r == [(0, 3)]

    def test_direction_switch(self):
        """方向在 UP→DOWN 之间切换时，合并行为随之切换。"""
        # Bar0→1: UP(12>10, 7>5)
        # Bar1→2: UP(15>12, 10>7) — no inclusion
        # Bar2→3: DOWN(13<15, 9<10) — no inclusion
        # Bar3→4: inclusion(13>=12, 9<=10), dir=DOWN → down merge
        #         H=min(13,12)=12, L=min(9,10)=9
        df = pd.DataFrame({
            "open":  [5, 7, 11, 10, 10],
            "high":  [10, 12, 15, 13, 12],
            "low":   [5, 7, 10, 9, 10],
            "close": [9, 11, 14, 12, 11],
        })
        m, _ = merge_inclusion(df)
        # 合并后最后一根应该是 down merge 的结果
        assert m["high"].iloc[-1] == 12.0
        assert m["low"].iloc[-1] == 9.0


# =====================================================================
# TestEdgeCases — 边界条件
# =====================================================================

class TestEdgeCases:
    """边界情况。"""

    def test_empty_df(self):
        """空 DataFrame。"""
        df = pd.DataFrame(columns=["open", "high", "low", "close"])
        m, r = merge_inclusion(df)
        assert len(m) == 0
        assert r == []

    def test_single_bar(self):
        """单根 K 线直接通过。"""
        df = pd.DataFrame({"open": [5], "high": [10], "low": [3], "close": [8]})
        m, r = merge_inclusion(df)
        assert len(m) == 1
        assert m["high"].iloc[0] == 10.0
        assert m["low"].iloc[0] == 3.0
        assert r == [(0, 0)]

    def test_all_ascending_no_inclusion(self):
        """完全单调上升（无包含），输出与输入等长。"""
        df = pd.DataFrame({
            "open":  [1, 3, 5, 7],
            "high":  [2, 4, 6, 8],
            "low":   [1, 3, 5, 7],
            "close": [2, 4, 6, 8],
        })
        m, r = merge_inclusion(df)
        assert len(m) == 4
        assert r == [(0, 0), (1, 1), (2, 2), (3, 3)]

    def test_datetime_index_preserved(self):
        """DateTimeIndex 被保留到输出。"""
        idx = pd.to_datetime(["2025-01-01", "2025-01-02", "2025-01-03"])
        df = pd.DataFrame({
            "open":  [5, 7, 8],
            "high":  [10, 12, 11],
            "low":   [5, 7, 8],
            "close": [9, 11, 10],
        }, index=idx)
        m, _ = merge_inclusion(df)
        # Bar1 和 Bar2 合并，index 取最后一根 (2025-01-03)
        assert len(m) == 2
        assert m.index[0] == pd.Timestamp("2025-01-01")
        assert m.index[1] == pd.Timestamp("2025-01-03")

    def test_open_close_convention(self):
        """§2.2 SHOULD: open=左K.open, close=右K.close。"""
        df = pd.DataFrame({
            "open":  [100, 200, 300],
            "high":  [10, 12, 11],
            "low":   [5, 7, 8],
            "close": [110, 210, 310],
        })
        m, _ = merge_inclusion(df)
        # Bar1+Bar2 合并: open=Bar1.open=200, close=Bar2.close=310
        assert m["open"].iloc[1] == 200.0
        assert m["close"].iloc[1] == 310.0


# =====================================================================
# TestAssertIntegration — assert_inclusion_no_residual 集成
# =====================================================================

class TestAssertIntegration:
    """验证断言函数与 merge_inclusion 的集成。"""

    def test_merged_output_passes(self):
        """合并后的输出应该通过无残留包含断言。"""
        df = _make_df()
        m, _ = merge_inclusion(df)
        result = assert_inclusion_no_residual(m)
        assert result.ok is True

    def test_raw_data_with_inclusion_fails(self):
        """含包含关系的原始数据应该触发断言失败。"""
        df = pd.DataFrame({
            "open":  [5, 6],
            "high":  [20, 15],
            "low":   [1, 5],
            "close": [10, 10],
        })
        # 20>=15 and 1<=5 → 左包右 → 有残留包含
        result = assert_inclusion_no_residual(df)
        assert result.ok is False
        assert "Residual inclusion" in result.message

    def test_clean_data_passes(self):
        """无包含关系的数据通过断言。"""
        df = pd.DataFrame({
            "open":  [1, 3],
            "high":  [2, 4],
            "low":   [1, 3],
            "close": [2, 4],
        })
        result = assert_inclusion_no_residual(df)
        assert result.ok is True

    def test_empty_input_passes(self):
        """空输入或无参数不报错。"""
        assert assert_inclusion_no_residual().ok is True
        df = pd.DataFrame(columns=["open", "high", "low", "close"])
        assert assert_inclusion_no_residual(df).ok is True
