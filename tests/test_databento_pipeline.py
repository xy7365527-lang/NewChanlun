"""Databento → Nautilus 管线守卫测试。

离线可跑：BarCounter / 配置口径用纯构造；catalog 端到端用 .cache/dbn/ 已缓存
DBN 文件（缺文件 skip，不触网不计费）。
"""

from __future__ import annotations

import os
import sys
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO_ROOT))

DBN_DIR = REPO_ROOT / ".cache" / "dbn"
DEF_DBN = DBN_DIR / "GLBX.MDP3_ESM6_definition_2026-05-12_2026-06-11.dbn"
BARS_DBN = DBN_DIR / "GLBX.MDP3_ESM6_ohlcv-1m_2026-05-12_2026-06-11.dbn"


class TestBarCounter:
    def _make_bar(self, bar_type, ts: int):
        from nautilus_trader.model.data import Bar
        from nautilus_trader.model.objects import Price, Quantity

        return Bar(
            bar_type=bar_type,
            open=Price(100.0, 2),
            high=Price(101.0, 2),
            low=Price(99.0, 2),
            close=Price(100.5, 2),
            volume=Quantity(10.0, 0),
            ts_event=ts,
            ts_init=ts,
        )

    def test_counts_per_bar_type_and_reset(self):
        from nautilus_trader.model.data import BarType

        from trading_system.strategy.bar_counter import BarCounter, BarCounterConfig

        bt_str = "ESM6.GLBX-1-MINUTE-LAST-EXTERNAL"
        bar_type = BarType.from_str(bt_str)
        strategy = BarCounter(BarCounterConfig(bar_types=[bt_str], log_first_n=0))
        strategy.bar_counts[bar_type] = 0  # on_start 需 engine 上下文，直接铺初值
        for i in range(3):
            strategy.on_bar(self._make_bar(bar_type, ts=i + 1))
        assert strategy.bar_counts[bar_type] == 3
        strategy.on_reset()
        assert strategy.bar_counts[bar_type] == 0


class TestBrokerConfigDatabento:
    def test_missing_key_fails_fast(self, monkeypatch):
        from trading_system.config.broker_config import databento_live_config

        monkeypatch.delenv("DATABENTO_API_KEY", raising=False)
        with pytest.raises(EnvironmentError):
            databento_live_config(["ESM6.GLBX"])

    def test_venue_pinned_to_glbx(self, monkeypatch):
        """use_exchange_as_venue 必须显式 False（与 catalog 口径一致）。"""
        from trading_system.config.broker_config import databento_live_config

        monkeypatch.setenv("DATABENTO_API_KEY", "db-test-key")
        cfg = databento_live_config(["ESM6.GLBX", "CLN6.GLBX"])
        assert cfg.use_exchange_as_venue is False
        assert [str(i) for i in cfg.instrument_ids] == ["ESM6.GLBX", "CLN6.GLBX"]
        assert cfg.api_key is None  # 凭据不落配置对象，adapter 从环境变量读


class TestCatalogSmokeConfig:
    def test_run_config_shape(self):
        from trading_system.backtest.catalog_smoke import build_run_config

        cfg = build_run_config("/tmp/cat", "ESM6.GLBX")
        assert cfg.venues[0].name == "GLBX"  # venue 从 instrument id 推导
        assert cfg.dispose_on_completion is False  # run 后读策略计数依赖此项
        assert cfg.data[0].bar_spec == "1-MINUTE-LAST"


@pytest.mark.skipif(
    not (DEF_DBN.exists() and BARS_DBN.exists()),
    reason="本地 DBN 缓存缺失（先跑 databento_catalog.py 拉数）",
)
class TestCatalogEndToEnd:
    def test_write_catalog_dedupes_instruments(self, tmp_path):
        """27 日 definition 快照 → 去重后单 instrument；bars 全量写入。"""
        from trading_system.data.databento_loader import load_dbn_to_catalog

        summary = load_dbn_to_catalog(DEF_DBN, [BARS_DBN], tmp_path / "catalog")
        assert summary["instruments"] == 1
        assert summary["data_counts"][BARS_DBN.name] > 0

        from nautilus_trader.persistence.catalog import ParquetDataCatalog

        catalog = ParquetDataCatalog(str(tmp_path / "catalog"))
        instruments = catalog.instruments()
        assert [str(i.id) for i in instruments] == ["ESM6.GLBX"]
        bars = catalog.bars()
        assert len(bars) == summary["data_counts"][BARS_DBN.name]
        # 时间戳单调（bars_timestamp_on_close 口径下仍须严格递增）
        ts = [b.ts_event for b in bars]
        assert all(a < b for a, b in zip(ts, ts[1:]))
