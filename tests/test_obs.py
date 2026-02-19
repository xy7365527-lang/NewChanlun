"""obs/logger.py + obs/metrics.py 单元测试。"""
from __future__ import annotations

import json
import logging
import time

import pytest

from newchan.obs.logger import StructuredLogger
from newchan.obs.metrics import EngineMetrics


# ══════════ StructuredLogger ══════════


class TestStructuredLogger:
    def test_create_logger(self):
        log = StructuredLogger("test.logger.create")
        assert log._logger.name == "test.logger.create"

    def test_event_emits_record(self, caplog):
        log = StructuredLogger("test.logger.event")
        with caplog.at_level(logging.DEBUG, logger="test.logger.event"):
            log.event("bar processed", bar_idx=42, tf="5m", event_count=3)
        assert len(caplog.records) >= 1
        rec = caplog.records[-1]
        assert rec.message == "bar processed"
        assert rec.levelno == logging.INFO

    def test_event_structured_extra(self, caplog):
        log = StructuredLogger("test.logger.extra")
        with caplog.at_level(logging.DEBUG, logger="test.logger.extra"):
            log.event("test", bar_idx=10, tf="1m", event_count=5, custom_key="val")
        rec = caplog.records[-1]
        extra = getattr(rec, "_structured_extra", {})
        assert extra["bar_idx"] == 10
        assert extra["tf"] == "1m"
        assert extra["event_count"] == 5
        assert extra["custom_key"] == "val"

    def test_warning_level(self, caplog):
        log = StructuredLogger("test.logger.warn")
        with caplog.at_level(logging.DEBUG, logger="test.logger.warn"):
            log.warning("something wrong", detail="info")
        rec = caplog.records[-1]
        assert rec.levelno == logging.WARNING
        assert rec.message == "something wrong"

    def test_error_level(self, caplog):
        log = StructuredLogger("test.logger.err")
        with caplog.at_level(logging.DEBUG, logger="test.logger.err"):
            log.error("crash", code=500)
        rec = caplog.records[-1]
        assert rec.levelno == logging.ERROR
        extra = getattr(rec, "_structured_extra", {})
        assert extra["code"] == 500

    def test_idempotent_handler_setup(self):
        """Creating multiple loggers with same name should not duplicate handlers."""
        log1 = StructuredLogger("test.logger.idem")
        n1 = len(log1._logger.handlers)
        log2 = StructuredLogger("test.logger.idem")
        n2 = len(log2._logger.handlers)
        assert n1 == n2


# ══════════ EngineMetrics ══════════


class TestEngineMetrics:
    def test_initial_state(self):
        m = EngineMetrics(tf="5m")
        assert m.tf == "5m"
        assert m.events_total == 0
        assert m.violations_total == 0
        assert m.bars_processed == 0
        assert m.last_bar_ts == 0.0

    def test_record_bar(self):
        m = EngineMetrics(tf="1m")
        m.record_bar(event_count=3, bar_ts=1000.0)
        assert m.bars_processed == 1
        assert m.events_total == 3
        assert m.last_bar_ts == 1000.0
        m.record_bar(event_count=2, bar_ts=1060.0)
        assert m.bars_processed == 2
        assert m.events_total == 5
        assert m.last_bar_ts == 1060.0

    def test_record_violation(self):
        m = EngineMetrics()
        m.record_violation()
        assert m.violations_total == 1
        m.record_violation(count=3)
        assert m.violations_total == 4

    def test_timer(self):
        m = EngineMetrics()
        m.start_timer()
        time.sleep(0.05)
        m.stop_timer()
        assert m.last_process_duration_ms >= 0
        assert m._timer_start == 0.0

    def test_stop_timer_without_start(self):
        m = EngineMetrics()
        m.stop_timer()
        assert m.last_process_duration_ms == 0.0

    def test_snapshot(self):
        m = EngineMetrics(tf="5m")
        m.record_bar(event_count=10, bar_ts=2000.0)
        m.record_violation(count=2)
        snap = m.snapshot()
        assert snap["tf"] == "5m"
        assert snap["events_total"] == 10
        assert snap["violations_total"] == 2
        assert snap["bars_processed"] == 1
        assert snap["last_bar_ts"] == 2000.0
        assert "last_process_duration_ms" in snap

    def test_reset(self):
        m = EngineMetrics(tf="1m")
        m.record_bar(event_count=5, bar_ts=100.0)
        m.record_violation(count=2)
        m.reset()
        assert m.events_total == 0
        assert m.violations_total == 0
        assert m.bars_processed == 0
        assert m.last_bar_ts == 0.0
        assert m.last_process_duration_ms == 0.0
        # tf should not be reset
        assert m.tf == "1m"

    def test_snapshot_duration_rounded(self):
        m = EngineMetrics()
        m.start_timer()
        time.sleep(0.005)
        m.stop_timer()
        snap = m.snapshot()
        # Should be rounded to 3 decimal places
        dur_str = str(snap["last_process_duration_ms"])
        if "." in dur_str:
            assert len(dur_str.split(".")[1]) <= 3
