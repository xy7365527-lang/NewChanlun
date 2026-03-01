"""tests/test_ceremony_state.py — ceremony_state.py 的单元测试。"""

import json
from pathlib import Path
from unittest.mock import patch

import pytest

from scripts.ceremony_state import (
    _STATE_FILE,
    _SUSPENDED_FILE,
    _TEAM_INIT_DIR,
    _WAL_FILE,
    WAL_PHASES,
    clear_ceremony_state,
    clear_step,
    compute_rescan_hash,
    get_incomplete_team_inits,
    get_suspended_workstations,
    is_in_ceremony,
    mark_team_init_complete,
    mark_team_init_started,
    read_ceremony_state,
    read_step,
    suspend_workstation,
    unsuspend_workstation,
    write_ceremony_state,
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


# ═══════════════════════════════════════════════════════════════
# WAL ceremony state tests（283号缺口B）
# ═══════════════════════════════════════════════════════════════


class TestWALCeremonyState:
    @pytest.fixture(autouse=True)
    def _patch_wal(self, tmp_path):
        fake_wal = tmp_path / ".chanlun" / "ceremony_wal.json"
        with patch("scripts.ceremony_state._WAL_FILE", fake_wal):
            yield fake_wal

    def test_write_and_read_ceremony_state(self, tmp_path):
        fake_wal = tmp_path / ".chanlun" / "ceremony_wal.json"
        with patch("scripts.ceremony_state._WAL_FILE", fake_wal):
            write_ceremony_state(1, "SCAN_DONE", rescan_hash="abc123", workstations=["ws-a", "ws-b"])
            data = read_ceremony_state()

            assert data is not None
            assert data["epoch"] == 1
            assert data["phase"] == "SCAN_DONE"
            assert data["rescan_hash"] == "abc123"
            assert data["workstations"] == ["ws-a", "ws-b"]
            assert "last_transition_ts" in data
            assert data["prev_rescan_hash"] is None

    def test_prev_rescan_hash_preserved(self, tmp_path):
        fake_wal = tmp_path / ".chanlun" / "ceremony_wal.json"
        with patch("scripts.ceremony_state._WAL_FILE", fake_wal):
            write_ceremony_state(1, "SCAN_DONE", rescan_hash="hash1")
            write_ceremony_state(1, "RESCAN_DONE", rescan_hash="hash2")
            data = read_ceremony_state()

            assert data["rescan_hash"] == "hash2"
            assert data["prev_rescan_hash"] == "hash1"

    def test_read_nonexistent_wal(self, tmp_path):
        fake_wal = tmp_path / ".chanlun" / "ceremony_wal.json"
        with patch("scripts.ceremony_state._WAL_FILE", fake_wal):
            assert read_ceremony_state() is None

    def test_clear_ceremony_state(self, tmp_path):
        fake_wal = tmp_path / ".chanlun" / "ceremony_wal.json"
        with patch("scripts.ceremony_state._WAL_FILE", fake_wal):
            write_ceremony_state(1, "SCAN_DONE")
            assert read_ceremony_state() is not None

            clear_ceremony_state()
            assert read_ceremony_state() is None

    def test_clear_removes_tmp(self, tmp_path):
        fake_wal = tmp_path / ".chanlun" / "ceremony_wal.json"
        with patch("scripts.ceremony_state._WAL_FILE", fake_wal):
            # 手动创建 tmp 文件
            fake_wal.parent.mkdir(parents=True, exist_ok=True)
            tmp_file = fake_wal.with_suffix(".json.tmp")
            tmp_file.write_text("{}", encoding="utf-8")

            clear_ceremony_state()
            assert not tmp_file.exists()

    def test_invalid_phase_raises(self, tmp_path):
        fake_wal = tmp_path / ".chanlun" / "ceremony_wal.json"
        with patch("scripts.ceremony_state._WAL_FILE", fake_wal):
            with pytest.raises(ValueError, match="Invalid WAL phase"):
                write_ceremony_state(1, "INVALID_PHASE")

    def test_corrupted_wal_file(self, tmp_path):
        fake_wal = tmp_path / ".chanlun" / "ceremony_wal.json"
        with patch("scripts.ceremony_state._WAL_FILE", fake_wal):
            fake_wal.parent.mkdir(parents=True, exist_ok=True)
            fake_wal.write_text("not json", encoding="utf-8")
            assert read_ceremony_state() is None

    def test_atomic_write_no_tmp_residue(self, tmp_path):
        fake_wal = tmp_path / ".chanlun" / "ceremony_wal.json"
        with patch("scripts.ceremony_state._WAL_FILE", fake_wal):
            write_ceremony_state(1, "SCAN_DONE")
            tmp_file = fake_wal.with_suffix(".json.tmp")
            assert not tmp_file.exists()
            assert fake_wal.exists()

    def test_compute_rescan_hash(self):
        output_a = {"workstations": [{"name": "ws-a"}, {"name": "ws-b"}]}
        output_b = {"workstations": [{"name": "ws-b"}, {"name": "ws-a"}]}
        output_c = {"workstations": [{"name": "ws-a"}, {"name": "ws-c"}]}

        hash_a = compute_rescan_hash(output_a)
        hash_b = compute_rescan_hash(output_b)
        hash_c = compute_rescan_hash(output_c)

        # 同内容（排序后）hash 相同
        assert hash_a == hash_b
        # 不同内容 hash 不同
        assert hash_a != hash_c
        # hash 是 16 字符 hex
        assert len(hash_a) == 16

    def test_compute_rescan_hash_empty(self):
        assert compute_rescan_hash({}) == compute_rescan_hash({"workstations": []})


# ═══════════════════════════════════════════════════════════════
# Team init 事务标记 tests（283号缺口C）
# ═══════════════════════════════════════════════════════════════


class TestTeamInit:
    @pytest.fixture(autouse=True)
    def _patch_team_init_dir(self, tmp_path):
        fake_dir = tmp_path / ".chanlun" / "team_init"
        with patch("scripts.ceremony_state._TEAM_INIT_DIR", fake_dir):
            yield fake_dir

    def test_mark_started_and_complete(self, tmp_path):
        fake_dir = tmp_path / ".chanlun" / "team_init"
        with patch("scripts.ceremony_state._TEAM_INIT_DIR", fake_dir):
            mark_team_init_started("v128-swarm")
            data = json.loads((fake_dir / "v128-swarm.json").read_text(encoding="utf-8"))
            assert data["status"] == "started"
            assert data["team_name"] == "v128-swarm"
            assert "started_at" in data

            mark_team_init_complete("v128-swarm")
            data = json.loads((fake_dir / "v128-swarm.json").read_text(encoding="utf-8"))
            assert data["status"] == "complete"
            assert "completed_at" in data
            assert data["team_name"] == "v128-swarm"

    def test_get_incomplete_empty(self, tmp_path):
        fake_dir = tmp_path / ".chanlun" / "team_init"
        with patch("scripts.ceremony_state._TEAM_INIT_DIR", fake_dir):
            assert get_incomplete_team_inits() == []

    def test_get_incomplete_team_inits(self, tmp_path):
        fake_dir = tmp_path / ".chanlun" / "team_init"
        with patch("scripts.ceremony_state._TEAM_INIT_DIR", fake_dir):
            mark_team_init_started("team-a")
            mark_team_init_started("team-b")
            mark_team_init_complete("team-b")

            incomplete = get_incomplete_team_inits()
            assert incomplete == ["team-a"]

    def test_mark_complete_without_started(self, tmp_path):
        fake_dir = tmp_path / ".chanlun" / "team_init"
        with patch("scripts.ceremony_state._TEAM_INIT_DIR", fake_dir):
            mark_team_init_complete("orphan-team")
            data = json.loads((fake_dir / "orphan-team.json").read_text(encoding="utf-8"))
            assert data["status"] == "complete"

    def test_corrupted_init_file_treated_as_incomplete(self, tmp_path):
        fake_dir = tmp_path / ".chanlun" / "team_init"
        with patch("scripts.ceremony_state._TEAM_INIT_DIR", fake_dir):
            fake_dir.mkdir(parents=True, exist_ok=True)
            (fake_dir / "broken-team.json").write_text("not json", encoding="utf-8")
            incomplete = get_incomplete_team_inits()
            assert "broken-team" in incomplete
