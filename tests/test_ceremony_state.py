"""tests/test_ceremony_state.py — ceremony_state.py 的单元测试。"""

import json
from pathlib import Path
from unittest.mock import patch

import pytest

from scripts.ceremony_state import (
    _STATE_FILE,
    _SUSPENDED_FILE,
    clear_step,
    get_suspended_workstations,
    is_in_ceremony,
    read_step,
    suspend_workstation,
    unsuspend_workstation,
    write_step,
)


@pytest.fixture(autouse=True)
def _clean_state(tmp_path):
    """每个测试使用临时目录，避免污染项目根目录。"""
    fake_state = tmp_path / ".ceremony-step"
    with patch("scripts.ceremony_state._STATE_FILE", fake_state):
        yield fake_state


def test_write_and_read(tmp_path):
    fake_state = tmp_path / ".ceremony-step"
    with patch("scripts.ceremony_state._STATE_FILE", fake_state):
        write_step(3, "spawn-workstations")
        data = read_step()

        assert data is not None
        assert data["step"] == 3
        assert data["phase"] == "spawn-workstations"
        assert "timestamp" in data


def test_read_nonexistent(tmp_path):
    fake_state = tmp_path / ".ceremony-step"
    with patch("scripts.ceremony_state._STATE_FILE", fake_state):
        assert read_step() is None


def test_clear(tmp_path):
    fake_state = tmp_path / ".ceremony-step"
    with patch("scripts.ceremony_state._STATE_FILE", fake_state):
        write_step(1, "load-genome")
        assert is_in_ceremony() is True

        clear_step()
        assert is_in_ceremony() is False
        assert read_step() is None


def test_is_in_ceremony(tmp_path):
    fake_state = tmp_path / ".ceremony-step"
    with patch("scripts.ceremony_state._STATE_FILE", fake_state):
        assert is_in_ceremony() is False

        write_step(2, "recursive-entry")
        assert is_in_ceremony() is True


def test_read_corrupted_file(tmp_path):
    fake_state = tmp_path / ".ceremony-step"
    with patch("scripts.ceremony_state._STATE_FILE", fake_state):
        fake_state.write_text("not json", encoding="utf-8")
        assert read_step() is None


def test_clear_nonexistent(tmp_path):
    fake_state = tmp_path / ".ceremony-step"
    with patch("scripts.ceremony_state._STATE_FILE", fake_state):
        # 不应抛异常
        clear_step()


# ═══════════════════════════════════════════════════════════════
# Suspended workstation tests（270号）
# ═══════════════════════════════════════════════════════════════


class TestSuspendedWorkstations:
    @pytest.fixture(autouse=True)
    def _patch_suspended_file(self, tmp_path):
        fake_suspended = tmp_path / ".ceremony-suspended"
        with patch("scripts.ceremony_state._SUSPENDED_FILE", fake_suspended):
            yield fake_suspended

    def test_suspend_and_get(self, tmp_path):
        fake_suspended = tmp_path / ".ceremony-suspended"
        with patch("scripts.ceremony_state._SUSPENDED_FILE", fake_suspended):
            suspend_workstation("审讯:概念X", "stance-repetition")
            suspended = get_suspended_workstations()

            assert "审讯:概念X" in suspended
            assert suspended["审讯:概念X"]["reason"] == "stance-repetition"
            assert "suspended_at" in suspended["审讯:概念X"]

    def test_suspend_multiple(self, tmp_path):
        fake_suspended = tmp_path / ".ceremony-suspended"
        with patch("scripts.ceremony_state._SUSPENDED_FILE", fake_suspended):
            suspend_workstation("ws-A", "reason-a")
            suspend_workstation("ws-B", "reason-b")
            suspended = get_suspended_workstations()

            assert len(suspended) == 2
            assert "ws-A" in suspended
            assert "ws-B" in suspended

    def test_unsuspend_existing(self, tmp_path):
        fake_suspended = tmp_path / ".ceremony-suspended"
        with patch("scripts.ceremony_state._SUSPENDED_FILE", fake_suspended):
            suspend_workstation("ws-A", "reason-a")
            suspend_workstation("ws-B", "reason-b")

            result = unsuspend_workstation("ws-A")
            assert result is True

            suspended = get_suspended_workstations()
            assert "ws-A" not in suspended
            assert "ws-B" in suspended

    def test_unsuspend_nonexistent(self, tmp_path):
        fake_suspended = tmp_path / ".ceremony-suspended"
        with patch("scripts.ceremony_state._SUSPENDED_FILE", fake_suspended):
            result = unsuspend_workstation("not-here")
            assert result is False

    def test_get_empty(self, tmp_path):
        fake_suspended = tmp_path / ".ceremony-suspended"
        with patch("scripts.ceremony_state._SUSPENDED_FILE", fake_suspended):
            suspended = get_suspended_workstations()
            assert suspended == {}

    def test_corrupted_suspended_file(self, tmp_path):
        fake_suspended = tmp_path / ".ceremony-suspended"
        with patch("scripts.ceremony_state._SUSPENDED_FILE", fake_suspended):
            fake_suspended.write_text("not json", encoding="utf-8")
            suspended = get_suspended_workstations()
            assert suspended == {}

    def test_suspend_overwrites_reason(self, tmp_path):
        fake_suspended = tmp_path / ".ceremony-suspended"
        with patch("scripts.ceremony_state._SUSPENDED_FILE", fake_suspended):
            suspend_workstation("ws-A", "old-reason")
            suspend_workstation("ws-A", "new-reason")
            suspended = get_suspended_workstations()

            assert suspended["ws-A"]["reason"] == "new-reason"
