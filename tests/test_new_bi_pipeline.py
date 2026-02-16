"""新笔 vs 旧笔全管线集成测试 — 谱系 001 退化段假说验证

全管线 = inclusion → fractals → strokes (→ segments)
核心假说：包含压缩导致旧笔漏判 → 线段退化。新笔在原始K线计数，消除此链路。

场景设计：
1. 含包含压缩的序列，旧笔/新笔笔数差异
2. 新笔额外成笔区间的方向/价格有效性
3. 全管线端到端：bar → BiEngine → strokes 比较
4. 新笔+线段：退化段是否减少
"""

from __future__ import annotations

from datetime import datetime, timedelta, timezone

import numpy as np
import pandas as pd

from newchan.a_fractal import fractals_from_merged
from newchan.a_inclusion import merge_inclusion
from newchan.a_segment_v0 import segments_from_strokes_v0
from newchan.a_stroke import strokes_from_fractals
from newchan.bi_engine import BiEngine
from newchan.types import Bar


# ── helpers ──────────────────────────────────────────────────────────

def _bar(ts_offset: int, o: float, h: float, l: float, c: float) -> Bar:
    """从偏移序号创建 Bar（5分钟间隔）。"""
    return Bar(
        ts=datetime(2024, 1, 1, tzinfo=timezone.utc)
        + timedelta(seconds=ts_offset * 300),
        open=o, high=h, low=l, close=c,
    )


def _build_df(bars: list[Bar]) -> pd.DataFrame:
    """从 Bar 列表构造 DataFrame。"""
    if not bars:
        return pd.DataFrame(
            columns=["open", "high", "low", "close"],
            index=pd.DatetimeIndex([], name="time"),
        )
    data = [[b.open, b.high, b.low, b.close] for b in bars]
    arr = np.array(data, dtype=np.float64)
    return pd.DataFrame(
        arr,
        columns=["open", "high", "low", "close"],
        index=pd.DatetimeIndex([b.ts for b in bars], name="time"),
    )


def _full_pipeline(bars: list[Bar], mode: str = "wide"):
    """全管线：bars → inclusion → fractals → strokes。"""
    df = _build_df(bars)
    df_merged, merged_to_raw = merge_inclusion(df)
    fractals = fractals_from_merged(df_merged)
    strokes = strokes_from_fractals(
        df_merged, fractals,
        mode=mode,
        merged_to_raw=merged_to_raw if mode == "new" else None,
    )
    return df_merged, merged_to_raw, fractals, strokes


def _is_degenerate(seg) -> bool:
    """退化段：方向与端点价格矛盾。"""
    if seg.ep0_price == 0.0 or seg.ep1_price == 0.0:
        return False
    if seg.direction == "up":
        return seg.ep1_price < seg.ep0_price
    else:
        return seg.ep1_price > seg.ep0_price


# ── 场景 1：包含压缩导致的新旧笔差异 ──────────────────────────────

class TestInclusionCompressionEffect:
    """构造含包含关系的K线序列，验证新笔能识别旧笔漏掉的笔。

    设计思路：
    - 在价格转折区域放置多根相互包含的K线
    - 包含合并后 merged gap 缩小，旧笔不成笔
    - 但原始K线间距足够，新笔成笔
    """

    @staticmethod
    def _bars_with_inclusion() -> list[Bar]:
        """构造含包含压缩的K线序列。

        模式：
        - bar 0-5: 上升（高点递增），无包含
        - bar 6-9: 包含区域（bar 6 包含 7, bar 8 包含 9）→ 合并后变 2 根
        - bar 10-15: 下降（低点递降），无包含
        - bar 16-19: 包含区域 → 合并后变 2 根
        - bar 20-25: 上升

        包含合并后merged序列缩短，某些转折点之间merged gap不足4，
        但原始K线间距足够。
        """
        bars = []
        ts = 0

        # 上升段 (0-5): 每bar高低各升4
        for i in range(6):
            base = 50 + i * 4
            bars.append(_bar(ts, base + 0.5, base + 2, base - 2, base - 0.5))
            ts += 1

        # 包含区域 (6-9): bar6包含bar7, bar8包含bar9
        # bar6: high=74, low=70 → 大区间
        bars.append(_bar(ts, 73, 74, 70, 71)); ts += 1
        # bar7: high=73, low=71 → 被bar6包含
        bars.append(_bar(ts, 72, 73, 71, 72)); ts += 1
        # bar8: high=72, low=68 → 大区间
        bars.append(_bar(ts, 71, 72, 68, 69)); ts += 1
        # bar9: high=71, low=69 → 被bar8包含
        bars.append(_bar(ts, 70, 71, 69, 70)); ts += 1

        # 下降段 (10-15): 每bar高低各降4
        for i in range(6):
            base = 66 - i * 4
            bars.append(_bar(ts, base + 0.5, base + 2, base - 2, base - 0.5))
            ts += 1

        # 包含区域 (16-19): 类似处理
        bars.append(_bar(ts, 41, 42, 38, 39)); ts += 1
        bars.append(_bar(ts, 40, 41, 39, 40)); ts += 1
        bars.append(_bar(ts, 43, 44, 40, 41)); ts += 1
        bars.append(_bar(ts, 42, 43, 41, 42)); ts += 1

        # 上升段 (20-25)
        for i in range(6):
            base = 46 + i * 4
            bars.append(_bar(ts, base + 0.5, base + 2, base - 2, base - 0.5))
            ts += 1

        return bars

    def test_inclusion_reduces_merged_count(self):
        """包含处理确实减少了bar数量。"""
        bars = self._bars_with_inclusion()
        df = _build_df(bars)
        df_merged, merged_to_raw = merge_inclusion(df)
        assert len(df_merged) < len(bars), (
            f"包含处理应减少bar数：merged={len(df_merged)} vs raw={len(bars)}"
        )

    def test_new_bi_geq_old_bi_strokes(self):
        """新笔产出的笔数 >= 旧笔。

        因为新笔在原始K线间距上判断，不受包含压缩影响，
        能识别旧笔因merged gap不足而漏掉的笔。
        """
        bars = self._bars_with_inclusion()
        _, _, _, strokes_wide = _full_pipeline(bars, mode="wide")
        _, _, _, strokes_new = _full_pipeline(bars, mode="new")
        assert len(strokes_new) >= len(strokes_wide), (
            f"新笔应 >= 旧笔: new={len(strokes_new)} vs wide={len(strokes_wide)}"
        )

    def test_new_bi_strokes_valid(self):
        """新笔产出的每一笔都方向/价格有效。"""
        bars = self._bars_with_inclusion()
        _, _, _, strokes = _full_pipeline(bars, mode="new")
        for s in strokes:
            if s.direction == "up":
                assert s.p1 > s.p0, f"up stroke must have p1>p0: p0={s.p0} p1={s.p1}"
            else:
                assert s.p1 < s.p0, f"down stroke must have p1<p0: p0={s.p0} p1={s.p1}"

    def test_strokes_direction_alternates(self):
        """新笔产出的笔方向交替。"""
        bars = self._bars_with_inclusion()
        _, _, _, strokes = _full_pipeline(bars, mode="new")
        for i in range(1, len(strokes)):
            assert strokes[i].direction != strokes[i - 1].direction, (
                f"strokes[{i-1}]={strokes[i-1].direction} == strokes[{i}]={strokes[i].direction}"
            )


# ── 场景 2：BiEngine 集成 ──────────────────────────────────────────

class TestBiEngineNewMode:
    """通过 BiEngine 逐bar驱动，比较 mode="new" 和 mode="wide"。"""

    @staticmethod
    def _zigzag_with_inclusion(n: int = 40) -> list[Bar]:
        """生成含少量包含的锯齿序列。

        每16根一个完整周期，在转折点附近插入包含对。
        """
        bars: list[Bar] = []
        ts = 0
        for i in range(n):
            cycle_pos = i % 16
            if cycle_pos < 8:
                base = 100 - cycle_pos * 5
            else:
                base = 60 + (cycle_pos - 8) * 5

            # 在转折区域(pos=7,8)和(pos=15,0)制造包含
            if cycle_pos in (7, 15):
                # 大区间bar：包含下一根
                h = base + 4
                l = base - 4
            elif cycle_pos in (0, 8):
                # 小区间bar：被上一根包含
                h = base + 1
                l = base - 1
            else:
                h = base + 1.5
                l = base - 1.5

            o = (h + l) / 2 + 0.3
            c = (h + l) / 2 - 0.3
            bars.append(_bar(ts, o, h, l, c))
            ts += 1
        return bars

    def test_engine_new_mode_runs(self):
        """BiEngine mode='new' 能正常运行完所有bar。"""
        bars = self._zigzag_with_inclusion()
        assert len(bars) > 0
        engine = BiEngine(stroke_mode="new")
        snap = engine.process_bar(bars[0])
        for bar in bars[1:]:
            snap = engine.process_bar(bar)
        assert snap.bar_idx == len(bars) - 1

    def test_engine_new_vs_wide_stroke_count(self):
        """BiEngine: mode='new' 笔数 >= mode='wide'。"""
        bars = self._zigzag_with_inclusion(n=64)
        assert len(bars) > 0
        engine_wide = BiEngine(stroke_mode="wide")
        engine_new = BiEngine(stroke_mode="new")

        snap_wide = engine_wide.process_bar(bars[0])
        snap_new = engine_new.process_bar(bars[0])
        for bar in bars[1:]:
            snap_wide = engine_wide.process_bar(bar)
            snap_new = engine_new.process_bar(bar)

        assert len(snap_new.strokes) >= len(snap_wide.strokes), (
            f"new={len(snap_new.strokes)} < wide={len(snap_wide.strokes)}"
        )

    def test_engine_new_mode_strokes_connected(self):
        """BiEngine mode='new'：笔首尾相连。"""
        bars = self._zigzag_with_inclusion(n=48)
        assert len(bars) > 0
        engine = BiEngine(stroke_mode="new")
        snap = engine.process_bar(bars[0])
        for bar in bars[1:]:
            snap = engine.process_bar(bar)
        strokes = snap.strokes
        for i in range(1, len(strokes)):
            assert strokes[i].i0 == strokes[i - 1].i1, (
                f"strokes[{i}].i0={strokes[i].i0} != strokes[{i-1}].i1={strokes[i-1].i1}"
            )


# ── 场景 3：新笔 + 线段 → 退化段减少（001假说核心） ─────────────────

class TestNewBiReducesDegenerateSegments:
    """001假说：新笔消除退化段的根因链路。

    链路：包含压缩 → 旧笔漏判 → 线段含矛盾区间 → 退化段
    新笔在原始K线计数 → 不受压缩影响 → 多识别关键笔 → 线段更精确

    验证方式：相同K线序列，旧笔+v0线段 vs 新笔+v0线段，比较退化段数。
    """

    @staticmethod
    def _complex_bars() -> list[Bar]:
        """构造一组复杂K线：含包含、有多个转折点、足够产出线段。

        需要足够长的序列（~50+bar）才能产出多段线段。
        """
        bars = []
        ts = 0
        # 大周期锯齿 + 局部包含
        phases = [
            # (方向, 长度, 起始价, 幅度, 包含位置列表)
            ("up",   10, 50, 4, [3, 4]),      # 10 bars上升，bar3,4包含
            ("down",  8, 90, 5, [2, 3]),      # 8 bars下降，bar2,3包含
            ("up",   10, 50, 4, [5, 6]),      # 10 bars上升
            ("down",  8, 90, 5, [1, 2]),      # 8 bars下降
            ("up",   10, 50, 4, [7, 8]),      # 10 bars上升
            ("down",  6, 90, 5, []),           # 6 bars下降，无包含
        ]

        for direction, length, start_price, step, inclusion_pairs in phases:
            for i in range(length):
                if direction == "up":
                    base = start_price + i * step
                else:
                    base = start_price - i * step

                if i in inclusion_pairs and len(inclusion_pairs) >= 2 and i == inclusion_pairs[0]:
                    # 大区间
                    h, l = base + 5, base - 5
                elif i in inclusion_pairs and len(inclusion_pairs) >= 2 and i == inclusion_pairs[1]:
                    # 小区间（被包含）
                    h, l = base + 1, base - 1
                else:
                    h, l = base + 2, base - 2

                o = (h + l) / 2 + 0.5
                c = (h + l) / 2 - 0.5
                bars.append(_bar(ts, o, h, l, c))
                ts += 1

        return bars

    def test_has_enough_strokes_for_segments(self):
        """复杂序列应产出至少3笔（线段最低要求）。"""
        bars = self._complex_bars()
        _, _, _, strokes_wide = _full_pipeline(bars, mode="wide")
        _, _, _, strokes_new = _full_pipeline(bars, mode="new")
        assert len(strokes_wide) >= 3, f"旧笔只有{len(strokes_wide)}笔，不够成段"
        assert len(strokes_new) >= 3, f"新笔只有{len(strokes_new)}笔，不够成段"

    def test_degenerate_segments_new_leq_wide(self):
        """新笔+v0线段的退化段数 <= 旧笔+v0线段。"""
        bars = self._complex_bars()
        _, _, _, strokes_wide = _full_pipeline(bars, mode="wide")
        _, _, _, strokes_new = _full_pipeline(bars, mode="new")

        segs_wide = segments_from_strokes_v0(strokes_wide)
        segs_new = segments_from_strokes_v0(strokes_new)

        degen_wide = sum(1 for s in segs_wide if _is_degenerate(s))
        degen_new = sum(1 for s in segs_new if _is_degenerate(s))

        assert degen_new <= degen_wide, (
            f"001假说违反：新笔退化段({degen_new}) > 旧笔退化段({degen_wide})"
        )

    def test_all_strokes_valid_price(self):
        """两种模式的笔都应满足价格有效性。"""
        bars = self._complex_bars()
        for mode in ("wide", "new"):
            _, _, _, strokes = _full_pipeline(bars, mode=mode)
            for s in strokes:
                if s.direction == "up":
                    assert s.p1 > s.p0, (
                        f"mode={mode}: up stroke p0={s.p0} >= p1={s.p1}"
                    )
                else:
                    assert s.p1 < s.p0, (
                        f"mode={mode}: down stroke p0={s.p0} <= p1={s.p1}"
                    )


# ── 场景 4：边界条件 ───────────────────────────────────────────────

class TestNewBiEdgeCases:
    """新笔模式的边界条件。"""

    def test_no_inclusion_same_as_wide(self):
        """无包含K线时，新笔和旧笔产出相同。"""
        bars = []
        for i in range(30):
            cycle_pos = i % 16
            if cycle_pos < 8:
                base = 100 - cycle_pos * 5
            else:
                base = 60 + (cycle_pos - 8) * 5
            # 固定小振幅，保证无包含
            bars.append(_bar(i, base + 0.5, base + 1.5, base - 1.5, base - 0.5))

        _, _, _, strokes_wide = _full_pipeline(bars, mode="wide")
        _, _, _, strokes_new = _full_pipeline(bars, mode="new")

        assert len(strokes_wide) == len(strokes_new), (
            f"无包含时应相同: wide={len(strokes_wide)} new={len(strokes_new)}"
        )

    def test_empty_bars(self):
        """空输入两种模式都返回空。"""
        for mode in ("wide", "new"):
            _, _, _, strokes = _full_pipeline([], mode=mode)
            assert strokes == []

    def test_single_bar(self):
        """单bar输入两种模式都返回空。"""
        bars = [_bar(0, 50, 52, 48, 50)]
        for mode in ("wide", "new"):
            _, _, _, strokes = _full_pipeline(bars, mode=mode)
            assert strokes == []
