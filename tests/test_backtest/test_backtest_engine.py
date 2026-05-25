"""回测引擎测试 — 合成数据验证。

覆盖：
- 买入信号开仓 + 卖出信号平仓
- 退出条件判定（三种 BSP 类型）
- 短差程序模拟
- 仓位阶段转换
- 无信号时无交易
- 统计指标计算
- 防未来函数：入场价 = bar.close
- allow_short 配置
"""

from __future__ import annotations

from dataclasses import dataclass, field
from datetime import datetime, timedelta
from typing import Literal

from newchan.a_buysellpoint_v1 import BuySellPoint
from newchan.backtest import (
    BacktestConfig,
    BacktestEngine,
    BacktestResult,
    PositionPhase,
    Trade,
)
from newchan.types import Bar


# ── Stub 类型 ──


@dataclass
class _BspSnapshot:
    bar_idx: int = 0
    bar_ts: float = 0.0
    buysellpoints: list[BuySellPoint] = field(default_factory=list)
    events: list = field(default_factory=list)


@dataclass
class _ZsSnapshot:
    """最小化 ZhongshuSnapshot stub。"""
    bar_idx: int = 0
    bar_ts: float = 0.0
    zhongshus: list = field(default_factory=list)
    events: list = field(default_factory=list)


@dataclass
class _FakeSnapshot:
    """最小化 RecursiveOrchestratorSnapshot stub。"""

    bar_idx: int = 0
    bar_ts: float = 0.0
    bsp_snapshot: _BspSnapshot = field(default_factory=_BspSnapshot)
    zs_snapshot: _ZsSnapshot = field(default_factory=_ZsSnapshot)
    recursive_snapshots: list = field(default_factory=list)


def _bar(idx: int, close: float) -> Bar:
    return Bar(
        ts=datetime(2024, 1, 1) + timedelta(minutes=idx),
        open=close,
        high=close + 1,
        low=close - 1,
        close=close,
    )


def _buy_bsp(
    seg_idx: int = 0,
    kind: str = "type1",
    level_id: int = 1,
    price: float = 100.0,
    bar_idx: int = 0,
    confirmed: bool = True,
    center_zg: float = 110.0,
    center_zd: float = 90.0,
) -> BuySellPoint:
    return BuySellPoint(
        kind=kind,
        side="buy",
        level_id=level_id,
        seg_idx=seg_idx,
        move_seg_start=0,
        divergence_key=(0, 0, seg_idx),
        center_zd=center_zd,
        center_zg=center_zg,
        center_seg_start=0,
        price=price,
        bar_idx=bar_idx,
        confirmed=confirmed,
        settled=False,
    )


def _sell_bsp(
    seg_idx: int = 1,
    kind: str = "type1",
    level_id: int = 1,
    price: float = 120.0,
    bar_idx: int = 5,
    confirmed: bool = True,
    center_zg: float = 110.0,
    center_zd: float = 90.0,
) -> BuySellPoint:
    return BuySellPoint(
        kind=kind,
        side="sell",
        level_id=level_id,
        seg_idx=seg_idx,
        move_seg_start=0,
        divergence_key=(0, 0, seg_idx),
        center_zd=center_zd,
        center_zg=center_zg,
        center_seg_start=0,
        price=price,
        bar_idx=bar_idx,
        confirmed=confirmed,
        settled=False,
    )


# ── 基础流程 ──


class TestBasicFlow:
    """买入 → 卖出完整流程。"""

    def test_buy_then_sell(self):
        """买入信号开仓，卖出信号平仓，产生一笔盈利交易。"""
        engine = BacktestEngine()

        # bar 0: 买入信号确认，close=100
        snap0 = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[_buy_bsp(seg_idx=0, bar_idx=0)]),
        )
        engine.process_snapshot(snap0, _bar(0, 100.0))

        # bar 1-4: 无新信号，价格上涨
        for i in range(1, 5):
            snap = _FakeSnapshot(
                bar_idx=i,
                bsp_snapshot=_BspSnapshot(buysellpoints=[_buy_bsp(seg_idx=0, bar_idx=0)]),
            )
            engine.process_snapshot(snap, _bar(i, 100.0 + i * 5))

        # bar 5: 卖出信号确认，close=130
        snap5 = _FakeSnapshot(
            bar_idx=5,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, bar_idx=0),
                _sell_bsp(seg_idx=1, bar_idx=5),
            ]),
        )
        engine.process_snapshot(snap5, _bar(5, 130.0))

        result = engine.result()
        assert result.trade_count == 1
        t = result.trades[0]
        assert t.side == "long"
        assert t.entry_price == 100.0
        assert t.exit_price == 130.0
        assert t.pnl == 30.0
        assert t.hold_bars == 5
        assert t.exit_reason == "reverse_bsp"

    def test_entry_price_is_bar_close(self):
        """防未来函数：入场价 = bar.close，不是 BSP.price。"""
        engine = BacktestEngine()
        # BSP.price=95（段极值），但 bar.close=100
        snap = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, price=95.0, bar_idx=0),
            ]),
        )
        engine.process_snapshot(snap, _bar(0, 100.0))
        assert engine.has_open_position
        # 通过平仓验证
        snap1 = _FakeSnapshot(
            bar_idx=1,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, price=95.0, bar_idx=0),
                _sell_bsp(seg_idx=1, bar_idx=1),
            ]),
        )
        engine.process_snapshot(snap1, _bar(1, 110.0))
        result = engine.result()
        assert result.trades[0].entry_price == 100.0
        assert result.trades[0].exit_price == 110.0


class TestNoSignal:
    """无信号时不产生交易。"""

    def test_no_bsp_no_trade(self):
        engine = BacktestEngine()
        for i in range(10):
            snap = _FakeSnapshot(
                bar_idx=i,
                bsp_snapshot=_BspSnapshot(buysellpoints=[]),
            )
            engine.process_snapshot(snap, _bar(i, 100.0 + i))
        result = engine.result()
        assert result.trade_count == 0
        assert result.total_bars == 10

    def test_unconfirmed_bsp_ignored(self):
        """未确认的 BSP 不触发开仓。"""
        engine = BacktestEngine()
        snap = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, confirmed=False),
            ]),
        )
        engine.process_snapshot(snap, _bar(0, 100.0))
        assert not engine.has_open_position


# ── 退出条件（替代止损） ──


@dataclass
class _FakeZhongshu:
    """最小化 Zhongshu stub，用于退出条件测试。"""
    zd: float = 90.0
    zg: float = 110.0
    seg_start: int = 0
    seg_end: int = 2


class TestExitConditions:
    """退出条件判定——基于 BSP 类型的结构性退出。"""

    def test_type1_exit_new_zhongshu(self):
        """Type1 买点：上涨中形成新同级别中枢 → bsp_negated 退出。

        入场时 zs_snapshot 有 1 个中枢，后续出现第 2 个 → 退出。
        """
        engine = BacktestEngine()

        # bar 0: Type1 买点开仓，当前 1 个中枢
        snap0 = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, kind="type1", bar_idx=0),
            ]),
            zs_snapshot=_ZsSnapshot(zhongshus=[_FakeZhongshu()]),
        )
        engine.process_snapshot(snap0, _bar(0, 100.0))
        assert engine.has_open_position

        # bar 1: 价格上涨，仍然 1 个中枢 → 不退出
        snap1 = _FakeSnapshot(
            bar_idx=1,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, kind="type1", bar_idx=0),
            ]),
            zs_snapshot=_ZsSnapshot(zhongshus=[_FakeZhongshu()]),
        )
        engine.process_snapshot(snap1, _bar(1, 110.0))
        assert engine.has_open_position

        # bar 2: 出现第 2 个中枢 → bsp_negated 退出
        snap2 = _FakeSnapshot(
            bar_idx=2,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, kind="type1", bar_idx=0),
            ]),
            zs_snapshot=_ZsSnapshot(zhongshus=[
                _FakeZhongshu(),
                _FakeZhongshu(zd=105.0, zg=120.0, seg_start=2, seg_end=4),
            ]),
        )
        engine.process_snapshot(snap2, _bar(2, 115.0))
        assert not engine.has_open_position

        result = engine.result()
        assert result.trade_count == 1
        assert result.trades[0].exit_reason == "bsp_negated"

    def test_type2_exit_break_low(self):
        """Type2 买点：跌破前一下跌趋势最低点 → bsp_negated 退出。

        Type2 买点的 center_zd 作为前一下跌趋势最低点的近似。
        """
        engine = BacktestEngine()

        # bar 0: Type2 买点开仓，center_zd=90
        snap0 = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, kind="type2", bar_idx=0, center_zd=90.0),
            ]),
        )
        engine.process_snapshot(snap0, _bar(0, 100.0))
        assert engine.has_open_position

        # bar 1: 价格 91 > 90 → 不退出
        snap1 = _FakeSnapshot(
            bar_idx=1,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, kind="type2", bar_idx=0, center_zd=90.0),
            ]),
        )
        engine.process_snapshot(snap1, _bar(1, 91.0))
        assert engine.has_open_position

        # bar 2: 价格 89 < 90 → bsp_negated 退出
        snap2 = _FakeSnapshot(
            bar_idx=2,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, kind="type2", bar_idx=0, center_zd=90.0),
            ]),
        )
        engine.process_snapshot(snap2, _bar(2, 89.0))
        assert not engine.has_open_position

        result = engine.result()
        assert result.trades[0].exit_reason == "bsp_negated"

    def test_type3_exit_break_zg(self):
        """Type3 买点：回试跌破 ZG → bsp_negated 退出。"""
        engine = BacktestEngine()

        # bar 0: Type3 买点开仓，center_zg=110
        snap0 = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, kind="type3", bar_idx=0, center_zg=110.0),
            ]),
        )
        engine.process_snapshot(snap0, _bar(0, 115.0))
        assert engine.has_open_position

        # bar 1: 价格 111 > 110 → 不退出
        snap1 = _FakeSnapshot(
            bar_idx=1,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, kind="type3", bar_idx=0, center_zg=110.0),
            ]),
        )
        engine.process_snapshot(snap1, _bar(1, 111.0))
        assert engine.has_open_position

        # bar 2: 价格 109 < 110 → bsp_negated 退出
        snap2 = _FakeSnapshot(
            bar_idx=2,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, kind="type3", bar_idx=0, center_zg=110.0),
            ]),
        )
        engine.process_snapshot(snap2, _bar(2, 109.0))
        assert not engine.has_open_position

        result = engine.result()
        assert result.trades[0].exit_reason == "bsp_negated"

    def test_exit_condition_not_triggered(self):
        """退出条件未满足时持仓不变。"""
        engine = BacktestEngine()

        # Type1 买点开仓，1 个中枢
        snap0 = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, kind="type1", bar_idx=0),
            ]),
            zs_snapshot=_ZsSnapshot(zhongshus=[_FakeZhongshu()]),
        )
        engine.process_snapshot(snap0, _bar(0, 100.0))

        # 5 个 bar，中枢数量不变 → 持仓不变
        for i in range(1, 6):
            snap = _FakeSnapshot(
                bar_idx=i,
                bsp_snapshot=_BspSnapshot(buysellpoints=[
                    _buy_bsp(seg_idx=0, kind="type1", bar_idx=0),
                ]),
                zs_snapshot=_ZsSnapshot(zhongshus=[_FakeZhongshu()]),
            )
            engine.process_snapshot(snap, _bar(i, 100.0 + i * 2))

        assert engine.has_open_position
        assert engine.result().trade_count == 0


# ── 短差程序 ──


@dataclass
class _FakeSubBspSnapshot:
    """次级别 BSP 快照 stub。"""
    bar_idx: int = 0
    bar_ts: float = 0.0
    buysellpoints: list[BuySellPoint] = field(default_factory=list)
    events: list = field(default_factory=list)


@dataclass
class _FakeRecursiveLevelSnapshot:
    """次级别递归快照 stub。"""
    bar_idx: int = 0
    bar_ts: float = 0.0
    level_id: int = 2
    zhongshus: list = field(default_factory=list)
    moves: list = field(default_factory=list)
    zhongshu_events: list = field(default_factory=list)
    move_events: list = field(default_factory=list)
    bsp_snapshot: _BspSnapshot | None = None


class TestShortDiff:
    """短差程序模拟。"""

    def test_short_diff_reduce_in_cost_reduction_phase(self):
        """成本>0阶段：次级别卖点触发减仓，记录短差。"""
        config = BacktestConfig(
            operation_level=1,
            short_diff_level=0,
            position_fraction=0.1,
        )
        engine = BacktestEngine(config)

        # bar 0: 操作级别买点开仓
        snap0 = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, kind="type1", bar_idx=0, level_id=1),
            ]),
        )
        engine.process_snapshot(snap0, _bar(0, 100.0))
        assert engine.has_open_position

        # bar 1: 次级别卖点出现 → 短差减仓
        sub_sell = _sell_bsp(seg_idx=10, kind="type1", level_id=0, bar_idx=1, price=108.0)
        snap1 = _FakeSnapshot(
            bar_idx=1,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, kind="type1", bar_idx=0, level_id=1),
            ]),
            recursive_snapshots=[
                _FakeRecursiveLevelSnapshot(
                    level_id=0,
                    bsp_snapshot=_BspSnapshot(buysellpoints=[sub_sell]),
                ),
            ],
        )
        engine.process_snapshot(snap1, _bar(1, 108.0))

        # 仍然持仓（短差不平仓）
        assert engine.has_open_position
        # 检查短差记录
        assert engine._position is not None
        assert len(engine._position.short_diffs) == 1
        assert engine._position.short_diffs[0].side == "reduce"

    def test_short_diff_disabled_by_default(self):
        """默认不启用短差（short_diff_level=None）。"""
        engine = BacktestEngine()

        snap0 = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, bar_idx=0),
            ]),
        )
        engine.process_snapshot(snap0, _bar(0, 100.0))
        assert engine.has_open_position
        assert engine._position is not None
        assert len(engine._position.short_diffs) == 0


# ── 仓位阶段转换 ──


class TestPositionPhase:
    """仓位阶段状态机。"""

    def test_initial_phase_is_cost_reduction(self):
        """开仓后初始阶段为 COST_REDUCTION。"""
        engine = BacktestEngine()
        snap = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[_buy_bsp(seg_idx=0, bar_idx=0)]),
        )
        engine.process_snapshot(snap, _bar(0, 100.0))
        assert engine._position is not None
        assert engine._position.phase == PositionPhase.COST_REDUCTION

    def test_phase_at_exit_recorded(self):
        """平仓时记录当时的阶段。"""
        engine = BacktestEngine()

        snap0 = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[_buy_bsp(seg_idx=0, bar_idx=0)]),
        )
        engine.process_snapshot(snap0, _bar(0, 100.0))

        snap1 = _FakeSnapshot(
            bar_idx=1,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, bar_idx=0),
                _sell_bsp(seg_idx=1, bar_idx=1),
            ]),
        )
        engine.process_snapshot(snap1, _bar(1, 120.0))

        result = engine.result()
        assert result.trades[0].phase_at_exit == PositionPhase.COST_REDUCTION


# ── 统计指标 ──


class TestStatistics:
    """统计指标。"""

    def test_two_trades_generated(self):
        """两笔交易（一盈一亏）→ 验证生成 2 笔 Trade 事实。

        策略评价（胜率/盈亏比）已删除（pending-004）；此处只验证交易生成正确。
        """
        engine = BacktestEngine()

        # 第一笔：买100 卖120 → +20
        snap0 = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[_buy_bsp(seg_idx=0, bar_idx=0)]),
        )
        engine.process_snapshot(snap0, _bar(0, 100.0))
        snap1 = _FakeSnapshot(
            bar_idx=1,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, bar_idx=0),
                _sell_bsp(seg_idx=1, bar_idx=1),
            ]),
        )
        engine.process_snapshot(snap1, _bar(1, 120.0))

        # 第二笔：买110 卖100 → -10
        snap2 = _FakeSnapshot(
            bar_idx=2,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, bar_idx=0),
                _sell_bsp(seg_idx=1, bar_idx=1),
                _buy_bsp(seg_idx=2, bar_idx=2),
            ]),
        )
        engine.process_snapshot(snap2, _bar(2, 110.0))
        snap3 = _FakeSnapshot(
            bar_idx=3,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _buy_bsp(seg_idx=0, bar_idx=0),
                _sell_bsp(seg_idx=1, bar_idx=1),
                _buy_bsp(seg_idx=2, bar_idx=2),
                _sell_bsp(seg_idx=3, bar_idx=3),
            ]),
        )
        engine.process_snapshot(snap3, _bar(3, 100.0))

        result = engine.result()
        assert result.trade_count == 2

    # 注：test_max_drawdown 已删除（pending-004）——max_drawdown_pct 是策略评价指标。

    def test_empty_result(self):
        """无交易时 result 安全返回（策略评价指标已删，pending-004）。"""
        result = BacktestResult(trades=(), total_bars=0)
        assert result.trade_count == 0
        assert result.total_bars == 0


# ── 做空 ──


class TestShortSelling:
    """做空逻辑。"""

    def test_short_disabled_by_default(self):
        """默认不允许做空，sell BSP 不开仓。"""
        engine = BacktestEngine()
        snap = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[_sell_bsp(seg_idx=0, bar_idx=0)]),
        )
        engine.process_snapshot(snap, _bar(0, 100.0))
        assert not engine.has_open_position

    def test_short_enabled(self):
        """allow_short=True 时 sell BSP 开空仓。"""
        config = BacktestConfig(allow_short=True)
        engine = BacktestEngine(config)

        # 卖出信号开空仓 close=100
        snap0 = _FakeSnapshot(
            bar_idx=0,
            bsp_snapshot=_BspSnapshot(buysellpoints=[_sell_bsp(seg_idx=0, bar_idx=0)]),
        )
        engine.process_snapshot(snap0, _bar(0, 100.0))
        assert engine.has_open_position

        # 买入信号平空仓 close=90 → 盈利 10
        snap1 = _FakeSnapshot(
            bar_idx=1,
            bsp_snapshot=_BspSnapshot(buysellpoints=[
                _sell_bsp(seg_idx=0, bar_idx=0),
                _buy_bsp(seg_idx=1, bar_idx=1),
            ]),
        )
        engine.process_snapshot(snap1, _bar(1, 90.0))

        result = engine.result()
        assert result.trade_count == 1
        t = result.trades[0]
        assert t.side == "short"
        assert t.pnl == 10.0


# ── 去重 ──


class TestDuplicateBspIgnored:
    """同一 BSP 不重复触发。"""

    def test_same_bsp_across_bars(self):
        """同一 confirmed BSP 在多个 bar 的 snapshot 中出现，只触发一次开仓。"""
        engine = BacktestEngine()
        bsp = _buy_bsp(seg_idx=0, bar_idx=0)

        for i in range(5):
            snap = _FakeSnapshot(
                bar_idx=i,
                bsp_snapshot=_BspSnapshot(buysellpoints=[bsp]),
            )
            engine.process_snapshot(snap, _bar(i, 100.0 + i))

        # 只开了一次仓，未平仓
        assert engine.has_open_position
        assert engine.result().trade_count == 0  # 未平仓不计入
