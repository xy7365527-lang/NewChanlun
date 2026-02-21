"""真实市场数据 E2E 验证 — RecursiveOrchestrator（口径A递归引擎）

目的：验证口径 A 递归引擎在真实 AAPL 1分钟数据上的表现。
- 逐 bar 处理无崩溃
- 递归层级数在合理范围
- 每层中枢价格区间合法（zg > zd）
- level_id 严格递增
- 事件序列单调递增
- 输出各层统计摘要

运行：pytest tests/test_real_data_recursive_e2e.py -v -s
"""

from __future__ import annotations

import csv
from datetime import datetime
from pathlib import Path

import pytest

from newchan.orchestrator.recursive import RecursiveOrchestrator, RecursiveOrchestratorSnapshot
from newchan.types import Bar

# --- 数据加载（复用已有模式） ---

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
class TestRealDataRecursiveOrchestrator:
    """口径 A 递归引擎在 AAPL 1分钟数据上的 E2E 验证。"""

    @pytest.fixture(scope="class")
    def all_bars(self) -> list[Bar]:
        return _load_bars_from_csv(DATA_PATH)

    @pytest.fixture(scope="class")
    def rth_bars(self, all_bars: list[Bar]) -> list[Bar]:
        return _filter_rth_bars(all_bars)

    @pytest.fixture(scope="class")
    def final_snapshot(self, rth_bars: list[Bar]) -> RecursiveOrchestratorSnapshot:
        """逐 bar 处理全部 RTH 数据，返回最终快照。"""
        orch = RecursiveOrchestrator(
            stream_id="e2e_recursive_aapl",
            max_levels=6,
            stroke_mode="new",
        )
        snap = None
        for bar in rth_bars:
            snap = orch.process_bar(bar)
        assert snap is not None, "无 bar 可处理"
        return snap

    def test_data_loaded(self, all_bars: list[Bar], rth_bars: list[Bar]) -> None:
        """数据加载验证。"""
        assert len(all_bars) > 3000, f"全量数据应 > 3000 条，实际 {len(all_bars)}"
        assert len(rth_bars) > 1000, f"RTH 数据应 > 1000 条，实际 {len(rth_bars)}"
        assert 1500 < len(rth_bars) < 2500, f"RTH 数据量异常: {len(rth_bars)}"

    def test_no_crash_during_processing(self, final_snapshot: RecursiveOrchestratorSnapshot) -> None:
        """逐 bar 处理无崩溃（fixture 成功即通过）。"""
        assert final_snapshot is not None
        assert final_snapshot.bar_idx > 0

    def test_recursive_level_count_reasonable(self, final_snapshot: RecursiveOrchestratorSnapshot) -> None:
        """递归层级数在合理范围（0-5 层递归快照）。"""
        n_recursive = len(final_snapshot.recursive_snapshots)
        print(f"\n[E2E-Recursive] 递归层级数: {n_recursive}")
        assert 0 <= n_recursive <= 5, f"递归层级数异常: {n_recursive}"

    def test_level_ids_strictly_increasing(self, final_snapshot: RecursiveOrchestratorSnapshot) -> None:
        """递归快照的 level_id 严格递增。"""
        snaps = final_snapshot.recursive_snapshots
        if len(snaps) < 2:
            pytest.skip("递归层数不足2，无法验证递增性")
        for i in range(1, len(snaps)):
            assert snaps[i].level_id > snaps[i - 1].level_id, (
                f"level_id 不递增: level[{i-1}]={snaps[i-1].level_id}, "
                f"level[{i}]={snaps[i].level_id}"
            )

    def test_level_ids_start_from_2(self, final_snapshot: RecursiveOrchestratorSnapshot) -> None:
        """递归快照的 level_id 从 2 开始（level=1 由基础管线处理）。"""
        snaps = final_snapshot.recursive_snapshots
        if not snaps:
            pytest.skip("无递归层快照")
        assert snaps[0].level_id == 2, f"首个递归层 level_id 应为 2，实际 {snaps[0].level_id}"

    def test_zhongshu_price_range_valid(self, final_snapshot: RecursiveOrchestratorSnapshot) -> None:
        """每层中枢的价格区间合法：zg > zd。"""
        for snap in final_snapshot.recursive_snapshots:
            for j, zs in enumerate(snap.zhongshus):
                assert zs.zg > zs.zd, (
                    f"level={snap.level_id} 中枢[{j}]: "
                    f"zg({zs.zg}) <= zd({zs.zd})"
                )

    def test_level1_bi_layer(self, final_snapshot: RecursiveOrchestratorSnapshot) -> None:
        """Level=1 笔层产出验证。"""
        strokes = final_snapshot.bi_snapshot.strokes
        assert len(strokes) > 10, f"笔数应 > 10，实际 {len(strokes)}"
        # 笔方向交替
        for i in range(1, len(strokes)):
            assert strokes[i].direction != strokes[i - 1].direction, (
                f"笔 {i-1} 和笔 {i} 方向相同: {strokes[i].direction}"
            )
        print(f"\n[E2E-Recursive] Level=1 笔数: {len(strokes)}")

    def test_level1_segment_layer(self, final_snapshot: RecursiveOrchestratorSnapshot) -> None:
        """Level=1 线段层产出验证。"""
        segments = final_snapshot.seg_snapshot.segments
        assert len(segments) >= 1, f"线段数应 >= 1，实际 {len(segments)}"
        # 线段方向交替
        for i in range(1, len(segments)):
            assert segments[i].direction != segments[i - 1].direction, (
                f"线段 {i-1} 和线段 {i} 方向相同"
            )
        print(f"\n[E2E-Recursive] Level=1 线段数: {len(segments)}")

    def test_level1_zhongshu_valid(self, final_snapshot: RecursiveOrchestratorSnapshot) -> None:
        """Level=1 中枢合法性验证。"""
        zhongshus = final_snapshot.zs_snapshot.zhongshus
        print(f"\n[E2E-Recursive] Level=1 中枢数: {len(zhongshus)}")
        for i, zs in enumerate(zhongshus):
            assert zs.zg > zs.zd, f"Level=1 中枢[{i}]: zg({zs.zg}) <= zd({zs.zd})"

    def test_level1_move_layer(self, final_snapshot: RecursiveOrchestratorSnapshot) -> None:
        """Level=1 走势类型层产出验证。"""
        moves = final_snapshot.move_snapshot.moves
        print(f"\n[E2E-Recursive] Level=1 走势数: {len(moves)}")
        kind_counts: dict[str, int] = {}
        for m in moves:
            kind_counts[m.kind] = kind_counts.get(m.kind, 0) + 1
        if moves:
            print(f"[E2E-Recursive] Level=1 走势类型分布: {kind_counts}")

    def test_events_monotonic(self, rth_bars: list[Bar]) -> None:
        """事件序列的 bar_idx 单调递增。"""
        orch = RecursiveOrchestrator(
            stream_id="e2e_events_check",
            max_levels=6,
            stroke_mode="new",
        )
        prev_bar_idx = -1
        for bar in rth_bars:
            snap = orch.process_bar(bar)
            assert snap.bar_idx >= prev_bar_idx, (
                f"bar_idx 不单调: prev={prev_bar_idx}, curr={snap.bar_idx}"
            )
            prev_bar_idx = snap.bar_idx

    def test_pipeline_summary(
        self,
        final_snapshot: RecursiveOrchestratorSnapshot,
        rth_bars: list[Bar],
    ) -> None:
        """全管线汇总报告。"""
        snap = final_snapshot
        summary: dict[str, object] = {
            "bars(RTH)": len(rth_bars),
            "L1_strokes": len(snap.bi_snapshot.strokes),
            "L1_segments": len(snap.seg_snapshot.segments),
            "L1_zhongshus": len(snap.zs_snapshot.zhongshus),
            "L1_moves": len(snap.move_snapshot.moves),
            "L1_buysellpoints": len(snap.bsp_snapshot.buysellpoints),
            "recursive_levels": len(snap.recursive_snapshots),
        }

        # 每个递归层的统计
        for rs in snap.recursive_snapshots:
            prefix = f"L{rs.level_id}"
            summary[f"{prefix}_zhongshus"] = len(rs.zhongshus)
            summary[f"{prefix}_moves"] = len(rs.moves)

        print(f"\n{'='*60}")
        print("[E2E-Recursive] AAPL 1分钟 口径A递归引擎 全管线汇总")
        print(f"{'='*60}")
        for k, v in summary.items():
            print(f"  {str(k):25s}: {v}")
        print(f"{'='*60}")

        # 基本层层递减检查
        assert len(snap.bi_snapshot.strokes) <= len(rth_bars)
