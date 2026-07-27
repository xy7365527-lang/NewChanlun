"""trading_system 骨架最小守卫测试。

覆盖：杠杆纯函数、LMT-only 断言、桥接层时间守卫。
（生成态例外不适用——这三块是纯技术层，定义已结算。）
"""

import sys
from pathlib import Path

import pytest

pytest.importorskip("nautilus_trader.model")

REPO_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO_ROOT))

from trading_system.execution.leverage_calculator import LeverageCalculator


class TestLeverageCalculator:
    def test_l_max_formula(self):
        """L_max = 1/(D_struct + mm)；OSC 相位 P_neg = ZD。"""
        calc = LeverageCalculator(maint_margin_rate=0.10)
        # price=100, ZD=90 → D_struct = (100-90)/100 = 0.10 → L_max = 1/0.20 = 5.0
        r = calc.compute(price=100.0, zd=90.0, zg=95.0)
        assert r.d_struct == pytest.approx(0.10)
        assert r.l_max == pytest.approx(5.0)

    def test_up_move_phase_uses_zg(self):
        """MOVE↑ 相位 P_neg = ZG。"""
        calc = LeverageCalculator(maint_margin_rate=0.10)
        r = calc.compute(price=100.0, zd=90.0, zg=95.0, phase_is_up_move=True)
        assert r.d_struct == pytest.approx(0.05)
        assert r.l_max == pytest.approx(1.0 / 0.15)

    def test_price_below_p_neg_floors_at_mm(self):
        """价格已破 P_neg：D_struct=0，L_max=1/mm（仅保证金兜底）。"""
        calc = LeverageCalculator(maint_margin_rate=0.05)
        r = calc.compute(price=80.0, zd=90.0, zg=95.0)
        assert r.d_struct == 0.0
        assert r.l_max == pytest.approx(20.0)

    def test_max_quantity_respects_contract_multiplier(self):
        """期货乘数：BZ 1000桶/手——qty 按每手名义 price×1000 折算。"""
        calc = LeverageCalculator(maint_margin_rate=0.10)
        qty = calc.max_quantity(
            equity=1_000_000.0, price=100.0, zd=90.0, zg=95.0,
            notional_frac=1.0, contract_multiplier=1000.0,
        )
        # intent = 1e6/(100×1000) = 10 手；cap = 5×1e6/1e5 = 50 手 → 10
        assert qty == pytest.approx(10.0)

    def test_invalid_mm_rejected(self):
        with pytest.raises(ValueError):
            LeverageCalculator(maint_margin_rate=1.5)

    def test_short_osc_phase_uses_zg(self):
        """[镜像推导] 空头 OSC 相位 P_neg_short = ZG（结构否定=三买）。"""
        calc = LeverageCalculator(maint_margin_rate=0.10)
        # price=100, ZG=110 → D_struct = (110-100)/100 = 0.10 → L_max = 5.0
        r = calc.compute_short(price=100.0, zd=90.0, zg=110.0)
        assert r.d_struct == pytest.approx(0.10)
        assert r.l_max == pytest.approx(5.0)

    def test_short_down_move_phase_uses_zd(self):
        """[镜像推导] MOVE↓ 锁定相 P_neg_short = 新中枢 ZD。"""
        calc = LeverageCalculator(maint_margin_rate=0.10)
        r = calc.compute_short(
            price=100.0, zd=105.0, zg=120.0, phase_is_down_move=True
        )
        assert r.d_struct == pytest.approx(0.05)
        assert r.l_max == pytest.approx(1.0 / 0.15)

    def test_short_price_above_p_neg_floors_at_mm(self):
        """价格已升破 P_neg_short：D_struct=0，仅 mm 兜底（多头侧镜像）。"""
        calc = LeverageCalculator(maint_margin_rate=0.05)
        r = calc.compute_short(price=120.0, zd=90.0, zg=110.0)
        assert r.d_struct == 0.0
        assert r.l_max == pytest.approx(20.0)

    def test_short_mirror_symmetry_with_long(self):
        """镜像对称：价格映射 p ↦ 200−p 下空头 L_max = 多头 L_max。

        多头 (c=100, P_neg=ZD=90) 与空头 (c=100, P_neg=ZG=110) 的结构
        距离相等 ⇒ 同一 L_max——杠杆三元组定理1 公式极性无关的可执行面。
        """
        calc = LeverageCalculator(maint_margin_rate=0.10)
        long_r = calc.compute(price=100.0, zd=90.0, zg=95.0)
        short_r = calc.compute_short(price=100.0, zd=105.0, zg=110.0)
        assert long_r.l_max == pytest.approx(short_r.l_max)


class TestChanlunBridge:
    def _bar(self, ts_ns: int, px: float = 100.0):
        """构造最小 Nautilus Bar。"""
        from nautilus_trader.model.data import Bar, BarType
        from nautilus_trader.model.objects import Price, Quantity

        bt = BarType.from_str("BZ.GLBX-1-MINUTE-LAST-EXTERNAL")
        p = Price(px, 2)
        return Bar(
            bar_type=bt, open=p, high=p, low=p, close=p,
            volume=Quantity.from_int(1), ts_event=ts_ns, ts_init=ts_ns,
        )

    def test_feed_monotonic_and_duplicate(self):
        from trading_system.strategy.signal_bridge import ChanlunBridge, FeedResult

        bridge = ChanlunBridge()
        m = 60_000_000_000  # 1min ns
        assert bridge.feed(self._bar(1 * m)) is FeedResult.ACCEPTED
        assert bridge.feed(self._bar(2 * m)) is FeedResult.ACCEPTED
        # 重复 ts → DUPLICATE 丢弃计数
        assert bridge.feed(self._bar(2 * m)) is FeedResult.DUPLICATE
        assert bridge.dup_count == 1
        assert bridge.bar_count == 2

    def test_feed_backwards_ts_fails_fast(self):
        from trading_system.strategy.signal_bridge import ChanlunBridge

        bridge = ChanlunBridge()
        m = 60_000_000_000
        bridge.feed(self._bar(2 * m))
        with pytest.raises(RuntimeError, match="倒退"):
            bridge.feed(self._bar(1 * m))

    def test_gap_counted_not_filled(self):
        from trading_system.strategy.signal_bridge import ChanlunBridge

        bridge = ChanlunBridge()
        m = 60_000_000_000
        bridge.feed(self._bar(1 * m))
        bridge.feed(self._bar(2 * m))   # 建立 interval
        bridge.feed(self._bar(10 * m))  # gap
        assert bridge.gap_count == 1
        assert bridge.bar_count == 3    # 不填充


class TestPersistence:
    def _make_db(self, tmp_path):
        from trading_system.persistence import TradingDatabase

        return TradingDatabase(tmp_path / "test.db")

    def test_bar_cache_roundtrip_and_dedup(self, tmp_path):
        from trading_system.persistence import BarCache

        with self._make_db(tmp_path) as db:
            cache = BarCache(db, commit_interval=2)
            cache.append("BTC-USD-PERP.HYPERLIQUID", 1000, 1, 2, 0.5, 1.5, 10)
            cache.append("BTC-USD-PERP.HYPERLIQUID", 2000, 1.5, 2.5, 1, 2, 20)
            cache.append("BTC-USD-PERP.HYPERLIQUID", 2000, 9, 9, 9, 9, 99)  # dup ts 忽略
            cache.flush()
            rows = list(cache.iter_bars("BTC-USD-PERP.HYPERLIQUID"))
            assert len(rows) == 2
            assert rows[1][4] == 2  # close 保留首次写入值（dup 被忽略）
            assert cache.last_ts("BTC-USD-PERP.HYPERLIQUID") == 2000

    def test_crash_recovery_replay_matches_snapshot(self, tmp_path):
        """关键场景：崩溃→重启→缓存重放→结构与崩溃前快照一致。"""
        import math

        from trading_system.persistence import BarCache, EngineStateStore
        from trading_system.strategy.signal_bridge import ChanlunBridge

        iid = "BZ.GLBX"
        with self._make_db(tmp_path) as db:
            cache = BarCache(db)
            store = EngineStateStore(db)

            # 第一段生命：喂 500 根合成波动 bar，同步缓存，停机前快照
            bridge1 = ChanlunBridge()
            m = 60_000_000_000
            for i in range(500):
                px = 100 + 10 * math.sin(i / 7) + 3 * math.sin(i / 3)
                o, c = px, px + math.sin(i)
                h, l = max(o, c) + 0.5, min(o, c) - 0.5
                bridge1.feed_ohlc((i + 1) * m, o, h, l, c)
                cache.append(iid, (i + 1) * m, o, h, l, c)
            cache.flush()
            store.save_snapshot(iid, bridge1)
            snap_before = bridge1.structure_snapshot()

            # 崩溃 → 新进程：新 bridge 从缓存重放，守卫对账
            bridge2 = ChanlunBridge()
            n = store.recover(bridge2, cache, iid)
            assert n == 500
            assert bridge2.structure_snapshot() == snap_before
            assert bridge2.watermark_ns == bridge1.watermark_ns

    def test_recovery_mismatch_fails_fast(self, tmp_path):
        """缓存缺损（水位线落后于快照）→ RecoveryMismatch，禁止继续交易。"""
        from trading_system.persistence import BarCache, EngineStateStore
        from trading_system.persistence.engine_state import RecoveryMismatch
        from trading_system.strategy.signal_bridge import ChanlunBridge

        iid = "BZ.GLBX"
        m = 60_000_000_000
        with self._make_db(tmp_path) as db:
            cache = BarCache(db)
            store = EngineStateStore(db)
            bridge1 = ChanlunBridge()
            for i in range(10):
                bridge1.feed_ohlc((i + 1) * m, 100, 101, 99, 100.5)
                if i < 5:  # 模拟缓存只写了一半（缺损）
                    cache.append(iid, (i + 1) * m, 100, 101, 99, 100.5)
            cache.flush()
            store.save_snapshot(iid, bridge1)
            with pytest.raises(RecoveryMismatch, match="缺损"):
                store.recover(ChanlunBridge(), cache, iid)

    def test_trade_journal_signal_and_order_lifecycle(self, tmp_path):
        from trading_system.persistence import TradeJournal
        from trading_system.strategy.signal_bridge import ChanlunSignal

        with self._make_db(tmp_path) as db:
            j = TradeJournal(db)
            sig = ChanlunSignal(
                action="BUY", level=1, notional_frac=1.0, price=100.0,
                kind="type1", reason="test", bar_index=42, ts_event_ns=1000,
            )
            j.log_signal("BZ.GLBX", sig)
            j.log_order_placed("O-1", 1000, "BZ.GLBX", "BUY", 2.0, 100.0)
            j.log_fill("O-1", 2000, 99.9, 2.0)
            j.commit()
            status = db.conn.execute(
                "SELECT status FROM orders WHERE client_order_id='O-1'",
            ).fetchone()[0]
            assert status == "FILLED"
            assert db.conn.execute("SELECT COUNT(*) FROM signals").fetchone()[0] == 1


class TestDataFeedRouting:
    def test_crypto_routes_to_hyperliquid(self):
        from trading_system.data.feed_abstraction import FeedRole, feed_for

        spec = feed_for("crypto", FeedRole.REALTIME)
        assert spec.name == "Hyperliquid WS"
        assert spec.nautilus_adapter == "nautilus_trader.adapters.hyperliquid"

    def test_hyperliquid_no_exchange_1s(self):
        """HL 无交易所侧 1s K线（adapter 源码核对）——1s 走 tick→INTERNAL。"""
        from trading_system.data.feed_abstraction import Granularity, supports

        assert not supports("HYPERLIQUID", Granularity.SEC_1, realtime=True)
        assert supports("HYPERLIQUID", Granularity.TICK, realtime=True)

    def test_alphavantage_not_realtime(self):
        """AV 无推送能力——实时粒度声明为空（防声明膨胀）。"""
        from trading_system.data.feed_abstraction import FEEDS

        assert FEEDS["ALPHAVANTAGE"].realtime_granularities == ()
        assert FEEDS["ALPHAVANTAGE"].nautilus_adapter is None

    def test_hyperliquid_adapter_importable(self):
        """1.228.0 官方 HL adapter 存在且配置类可构造（testnet）。"""
        from trading_system.config.broker_config import hyperliquid_config

        data_cfg, exec_cfg = hyperliquid_config(testnet=True)
        assert type(data_cfg).__name__ == "HyperliquidDataClientConfig"
        assert type(exec_cfg).__name__ == "HyperliquidExecClientConfig"


class TestLmtOnly:
    def test_market_order_forbidden(self):
        """非 LIMIT 订单 submit 前抛异常（fail-fast，不是过滤）。"""
        from unittest.mock import MagicMock

        from trading_system.execution.lmt_executor import (
            LmtExecutor,
            MarketOrderForbidden,
        )
        from nautilus_trader.model.enums import OrderType

        executor = LmtExecutor(strategy=MagicMock(), instrument=MagicMock())
        fake_mkt = MagicMock()
        fake_mkt.order_type = OrderType.MARKET
        with pytest.raises(MarketOrderForbidden):
            executor.submit(fake_mkt)
