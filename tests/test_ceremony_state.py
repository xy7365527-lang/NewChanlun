"""tests/test_ceremony_state.py — ceremony_state.py 的单元测试。"""

import json
from pathlib import Path
from unittest.mock import patch

import pytest

from scripts.ceremony_state import (
    _STATE_FILE,
    clear_step,
    is_in_ceremony,
    read_step,
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
