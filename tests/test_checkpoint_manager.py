"""tests/test_checkpoint_manager.py — checkpoint_manager.py 的单元测试（283号缺口A）。"""

import json
from pathlib import Path
from unittest.mock import patch

import pytest

from scripts.checkpoint_manager import (
    _CHECKPOINTS_DIR,
    clear_checkpoints,
    list_checkpoints,
    read_checkpoint,
    write_checkpoint,
)


@pytest.fixture(autouse=True)
def _patch_checkpoints_dir(tmp_path):
    fake_dir = tmp_path / ".chanlun" / "checkpoints"
    with patch("scripts.checkpoint_manager._CHECKPOINTS_DIR", fake_dir):
        yield fake_dir


class TestWriteAndRead:
    def test_write_and_read_checkpoint(self, tmp_path):
        fake_dir = tmp_path / ".chanlun" / "checkpoints"
        with patch("scripts.checkpoint_manager._CHECKPOINTS_DIR", fake_dir):
            write_checkpoint(
                team_name="v128-swarm",
                agent_name="ws-oq1",
                phase="data_fetch_done",
                done=["fetch_spy", "fetch_gld"],
                pending=["run_d_operator"],
                next_action="运行 D 算子",
                artifacts=[{"path": "tmp/raw.json", "sha256": "abc"}],
            )
            data = read_checkpoint("v128-swarm", "ws-oq1")

            assert data is not None
            assert data["agent_name"] == "ws-oq1"
            assert data["team_name"] == "v128-swarm"
            assert data["phase"] == "data_fetch_done"
            assert data["done"] == ["fetch_spy", "fetch_gld"]
            assert data["pending"] == ["run_d_operator"]
            assert data["next_action"] == "运行 D 算子"
            assert len(data["artifacts"]) == 1
            assert "updated_at" in data

    def test_op_id_format(self, tmp_path):
        fake_dir = tmp_path / ".chanlun" / "checkpoints"
        with patch("scripts.checkpoint_manager._CHECKPOINTS_DIR", fake_dir):
            write_checkpoint("team", "agent-x", "phase1", [], [], "next")
            data = read_checkpoint("team", "agent-x")
            assert data["op_id"].startswith("agent-x-run-")

    def test_read_nonexistent(self, tmp_path):
        fake_dir = tmp_path / ".chanlun" / "checkpoints"
        with patch("scripts.checkpoint_manager._CHECKPOINTS_DIR", fake_dir):
            assert read_checkpoint("no-team", "no-agent") is None

    def test_overwrite_checkpoint(self, tmp_path):
        fake_dir = tmp_path / ".chanlun" / "checkpoints"
        with patch("scripts.checkpoint_manager._CHECKPOINTS_DIR", fake_dir):
            write_checkpoint("t", "a", "phase1", ["step1"], ["step2"], "do step2")
            write_checkpoint("t", "a", "phase2", ["step1", "step2"], [], "done")
            data = read_checkpoint("t", "a")
            assert data["phase"] == "phase2"
            assert data["pending"] == []


class TestClear:
    def test_clear_checkpoints(self, tmp_path):
        fake_dir = tmp_path / ".chanlun" / "checkpoints"
        with patch("scripts.checkpoint_manager._CHECKPOINTS_DIR", fake_dir):
            write_checkpoint("team-a", "agent-1", "p", [], [], "n")
            write_checkpoint("team-a", "agent-2", "p", [], [], "n")
            assert len(list_checkpoints("team-a")) == 2

            clear_checkpoints("team-a")
            assert list_checkpoints("team-a") == []

    def test_clear_nonexistent_team(self, tmp_path):
        fake_dir = tmp_path / ".chanlun" / "checkpoints"
        with patch("scripts.checkpoint_manager._CHECKPOINTS_DIR", fake_dir):
            clear_checkpoints("ghost-team")  # 不应抛异常


class TestList:
    def test_list_checkpoints(self, tmp_path):
        fake_dir = tmp_path / ".chanlun" / "checkpoints"
        with patch("scripts.checkpoint_manager._CHECKPOINTS_DIR", fake_dir):
            write_checkpoint("team-a", "agent-1", "p1", [], [], "n1")
            write_checkpoint("team-a", "agent-2", "p2", [], [], "n2")
            result = list_checkpoints("team-a")
            assert len(result) == 2
            names = {r["agent_name"] for r in result}
            assert names == {"agent-1", "agent-2"}

    def test_list_empty_team(self, tmp_path):
        fake_dir = tmp_path / ".chanlun" / "checkpoints"
        with patch("scripts.checkpoint_manager._CHECKPOINTS_DIR", fake_dir):
            assert list_checkpoints("empty-team") == []

    def test_list_ignores_tmp_files(self, tmp_path):
        fake_dir = tmp_path / ".chanlun" / "checkpoints"
        with patch("scripts.checkpoint_manager._CHECKPOINTS_DIR", fake_dir):
            write_checkpoint("team-a", "agent-1", "p", [], [], "n")
            # 手动创建 tmp 文件
            tmp_file = fake_dir / "team-a" / "agent-1.json.tmp"
            tmp_file.write_text("{}", encoding="utf-8")
            result = list_checkpoints("team-a")
            assert len(result) == 1


class TestAtomicWrite:
    def test_no_tmp_residue(self, tmp_path):
        fake_dir = tmp_path / ".chanlun" / "checkpoints"
        with patch("scripts.checkpoint_manager._CHECKPOINTS_DIR", fake_dir):
            write_checkpoint("team", "agent", "phase", [], [], "next")
            team_dir = fake_dir / "team"
            tmp_files = list(team_dir.glob("*.tmp"))
            assert len(tmp_files) == 0

    def test_default_artifacts_empty_list(self, tmp_path):
        fake_dir = tmp_path / ".chanlun" / "checkpoints"
        with patch("scripts.checkpoint_manager._CHECKPOINTS_DIR", fake_dir):
            write_checkpoint("t", "a", "p", [], [], "n")
            data = read_checkpoint("t", "a")
            assert data["artifacts"] == []


class TestCorrupted:
    def test_corrupted_checkpoint_returns_none(self, tmp_path):
        fake_dir = tmp_path / ".chanlun" / "checkpoints"
        with patch("scripts.checkpoint_manager._CHECKPOINTS_DIR", fake_dir):
            target = fake_dir / "team" / "agent.json"
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text("not json", encoding="utf-8")
            assert read_checkpoint("team", "agent") is None
