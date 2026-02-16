"""真实市场数据 E2E 验证 — AAPL 1分钟 DataBento 数据

目的：验证 v1 全管线在真实数据上的表现，为以下定义结算提供实证：
- bi.md: 笔定义（从生成态→已结算的前提）
- 001 谱系: 退化段在真实数据上的表现
- 003 谱系: v0/v1 线段对比

运行：pytest tests/test_real_data_e2e.py -v -s
"""

from __future__ import annotations

import csv
from datetime import datetime
from pathlib import Path

import pytest

from newchan.types import Bar
from newchan.orchestrator.timeframes import TFOrchestrator

# --- 数据加载 ---

DATA_PATH = Path(__file__).parent.parent / "data" / "AAPL_20260209_20260213_1m.csv"


def _load_bars_from_csv(path: Path) -> list[Bar]:
    """从 CSV 加载 Bar 列表。"""
    bars: list[Bar] = []
    with open(path, "r") as f:
        reader = csv.DictReader(f)
        for row in reader:
            ts = datetime.fromisoformat(row["timestamp"])
            bars.append(Bar(
                ts=ts,
                open=float(row["open"]),
                high=float(row["high"]),
                low=float(row["low"]),
                close=float(row["close"]),
                volume=float(row["volume"]) if row.get("volume") else None,
            ))
    return bars


def _filter_rth_bars(bars: list[Bar]) -> list[Bar]:
    """过滤仅保留美东常规交易时段（9:30-16:00 ET = 14:30-21:00 UTC）。"""
    rth_bars = []
    for bar in bars:
        time_utc = bar.ts.hour * 60 + bar.ts.minute
        # ET = UTC-5: 9:30 ET = 14:30 UTC, 16:00 ET = 21:00 UTC
        if 14 * 60 + 30 <= time_utc < 21 * 60:
            rth_bars.append(bar)
    return rth_bars


# --- 跳过条件 ---

skip_no_data = pytest.mark.skipif(
    not DATA_PATH.exists(),
    reason=f"真实数据不可用: {DATA_PATH}",
)


# --- 测试 ---

@skip_no_data
class TestRealDataFullPipeline:
    """v1 全管线在 AAPL 1分钟数据上的 E2E 验证。"""

    @pytest.fixture(scope="class")
    def all_bars(self) -> list[Bar]:
        return _load_bars_from_csv(DATA_PATH)

    @pytest.fixture(scope="class")
    def rth_bars(self, all_bars: list[Bar]) -> list[Bar]:
        return _filter_rth_bars(all_bars)

    @pytest.fixture(scope="class")
    def pipeline_result(self, rth_bars: list[Bar]):
        """跑完 v1 全管线并收集最终状态。"""
        orch = TFOrchestrator(
            session_id="e2e_aapl",
            base_bars=rth_bars,
            timeframes=["1m"],
            stroke_mode="new",
        )
        orch.step(len(rth_bars))
        return orch

    def test_data_loaded(self, all_bars, rth_bars):
        """数据加载验证。"""
        assert len(all_bars) > 3000, f"全量数据应 > 3000 条，实际 {len(all_bars)}"
        assert len(rth_bars) > 1000, f"RTH 数据应 > 1000 条，实际 {len(rth_bars)}"
        # 5 个交易日 × 390 分钟 = 1950
        assert 1500 < len(rth_bars) < 2500, f"RTH 数据量异常: {len(rth_bars)}"

    def test_bi_layer(self, pipeline_result):
        """笔层产出验证。"""
        # 取最后一个快照（event_log 中最后一条）
        last_snap = pipeline_result.base_session.event_log[-1]
        strokes = last_snap.strokes
        assert len(strokes) > 10, f"笔数应 > 10，实际 {len(strokes)}"
        # 笔方向交替
        for i in range(1, len(strokes)):
            assert strokes[i].direction != strokes[i - 1].direction, (
                f"笔 {i-1} 和笔 {i} 方向相同: {strokes[i].direction}"
            )
        print(f"\n[E2E] 笔数: {len(strokes)}")

    def test_segment_layer(self, pipeline_result):
        """线段层产出验证（v1 管线）。"""
        segments = pipeline_result._segment_engines["1m"].current_segments
        assert len(segments) >= 1, f"线段数应 >= 1，实际 {len(segments)}"
        # 线段方向交替
        for i in range(1, len(segments)):
            assert segments[i].direction != segments[i - 1].direction, (
                f"线段 {i-1} 和线段 {i} 方向相同"
            )
        print(f"\n[E2E] 线段数(v1): {len(segments)}")

    def test_no_degenerate_segments(self, pipeline_result):
        """退化段检查（001 谱系验证）。"""
        segments = pipeline_result._segment_engines["1m"].current_segments
        degenerate_count = 0
        for seg in segments:
            n_strokes = seg.s1 - seg.s0 + 1
            if n_strokes < 3:
                degenerate_count += 1
        degenerate_rate = degenerate_count / max(len(segments), 1)
        print(f"\n[E2E] 线段总数: {len(segments)}, 退化段: {degenerate_count}, 退化率: {degenerate_rate:.2%}")
        # 退化率 < 20% 是合理预期
        assert degenerate_rate < 0.3, f"退化率过高: {degenerate_rate:.2%}"

    def test_zhongshu_layer(self, pipeline_result):
        """中枢层产出验证。"""
        zhongshus = pipeline_result._zhongshu_engines["1m"].current_zhongshus
        print(f"\n[E2E] 中枢总数: {len(zhongshus)}")
        if len(zhongshus) > 0:
            for i, zs in enumerate(zhongshus):
                assert zs.zg > zs.zd, f"中枢 {i}: ZG({zs.zg}) <= ZD({zs.zd})"
                assert zs.gg >= zs.zg, f"中枢 {i}: GG({zs.gg}) < ZG({zs.zg})"
                assert zs.dd <= zs.zd, f"中枢 {i}: DD({zs.dd}) > ZD({zs.zd})"

    def test_move_layer(self, pipeline_result):
        """走势类型层产出验证。"""
        moves = pipeline_result._move_engines["1m"].current_moves
        print(f"\n[E2E] 走势类型总数: {len(moves)}")
        kind_counts: dict[str, int] = {}
        for i, m in enumerate(moves):
            assert m.zs_end >= m.zs_start, f"Move {i}: zs_end < zs_start"
            kind_counts[m.kind] = kind_counts.get(m.kind, 0) + 1
        if moves:
            print(f"[E2E] 走势类型分布: {kind_counts}")

    def test_buysellpoint_layer(self, pipeline_result):
        """买卖点层产出验证。"""
        bsps = pipeline_result._bsp_engines["1m"].current_buysellpoints
        print(f"\n[E2E] 买卖点总数: {len(bsps)}")
        type_counts: dict[str, int] = {}
        for bsp in bsps:
            key = f"{bsp.kind}_{bsp.side}"
            type_counts[key] = type_counts.get(key, 0) + 1
        print(f"[E2E] 买卖点类型分布: {type_counts}")

    def test_pipeline_summary(self, pipeline_result, rth_bars):
        """全管线汇总报告。"""
        last_snap = pipeline_result.base_session.event_log[-1]
        segments = pipeline_result._segment_engines["1m"].current_segments
        zhongshus = pipeline_result._zhongshu_engines["1m"].current_zhongshus
        moves = pipeline_result._move_engines["1m"].current_moves
        bsps = pipeline_result._bsp_engines["1m"].current_buysellpoints

        summary = {
            "bars(RTH)": len(rth_bars),
            "merged_bars": last_snap.n_merged,
            "fractals": last_snap.n_fractals,
            "strokes": len(last_snap.strokes),
            "segments(v1)": len(segments),
            "zhongshus": len(zhongshus),
            "moves": len(moves),
            "buysellpoints": len(bsps),
        }
        print(f"\n{'='*60}")
        print(f"[E2E] AAPL 1分钟 v1 全管线汇总")
        print(f"{'='*60}")
        for k, v in summary.items():
            print(f"  {k:20s}: {v}")
        print(f"{'='*60}")

        # 层层递减
        assert last_snap.n_merged <= len(rth_bars)
        assert len(last_snap.strokes) <= last_snap.n_merged


@skip_no_data
class TestRealDataV0V1Comparison:
    """v0/v1 线段对比（003 谱系真实数据验证）。"""

    @pytest.fixture(scope="class")
    def rth_bars(self) -> list[Bar]:
        all_bars = _load_bars_from_csv(DATA_PATH)
        return _filter_rth_bars(all_bars)

    def test_v0_v1_segment_comparison(self, rth_bars):
        """v0/v1 线段对比。"""
        from newchan.a_segment_v0 import segments_from_strokes_v0
        from newchan.a_segment_v1 import segments_from_strokes_v1

        # 用 new 笔模式获取笔
        orch = TFOrchestrator(
            session_id="seg_cmp",
            base_bars=rth_bars,
            timeframes=["1m"],
            stroke_mode="new",
        )
        orch.step(len(rth_bars))
        last_snap = orch.base_session.event_log[-1]
        strokes = last_snap.strokes

        if len(strokes) < 3:
            pytest.skip(f"笔数不足: {len(strokes)}")

        segs_v0 = segments_from_strokes_v0(strokes)
        segs_v1 = segments_from_strokes_v1(strokes)

        n_v0 = len(segs_v0)
        n_v1 = len(segs_v1)

        # 退化段统计
        degen_v0 = sum(1 for s in segs_v0 if (s.s1 - s.s0 + 1) < 3)
        degen_v1 = sum(1 for s in segs_v1 if (s.s1 - s.s0 + 1) < 3)

        print(f"\n{'='*60}")
        print(f"[CMP] v0/v1 线段对比 (AAPL 1分钟, {len(strokes)} 笔)")
        print(f"{'='*60}")
        print(f"  v0 线段数: {n_v0} (退化段: {degen_v0})")
        print(f"  v1 线段数: {n_v1} (退化段: {degen_v1})")
        if n_v1 > 0:
            print(f"  v0/v1 比值: {n_v0/n_v1:.2f}")
        print(f"{'='*60}")

        # 003 谱系假说验证：v0 段数 >= v1 段数
        assert n_v0 >= n_v1, f"003假说违反: v0({n_v0}) < v1({n_v1})"

    def test_stroke_mode_comparison(self, rth_bars):
        """新旧笔模式对比。"""
        orch_wide = TFOrchestrator(
            session_id="cmp_wide",
            base_bars=rth_bars,
            timeframes=["1m"],
            stroke_mode="wide",
        )
        orch_wide.step(len(rth_bars))
        n_wide = len(orch_wide.base_session.event_log[-1].strokes)

        orch_new = TFOrchestrator(
            session_id="cmp_new",
            base_bars=rth_bars,
            timeframes=["1m"],
            stroke_mode="new",
        )
        orch_new.step(len(rth_bars))
        n_new = len(orch_new.base_session.event_log[-1].strokes)

        print(f"\n[CMP] 宽笔: {n_wide} 笔, 新笔: {n_new} 笔")
        assert n_wide > 0
        assert n_new > 0
