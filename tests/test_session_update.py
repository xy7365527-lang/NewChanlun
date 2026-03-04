"""tests/test_session_update.py — session_update.py + completion_persist_guard.py 测试

验证：
1. session_update.py 的 append_output 写入内容正确
2. append_output 同时更新 .last-session-append 标记
3. completion_persist_guard.py 的 check_append / mark_append / clear_marker 逻辑
4. finalize_session 更新 HEAD 和追加收尾标记
"""

import time
from pathlib import Path

import pytest

from scripts.session_update import append_output, create_session, finalize_session
from scripts.completion_persist_guard import (
    STALE_THRESHOLD_SECONDS,
    check_append,
    clear_marker,
    mark_append,
)


@pytest.fixture
def session_dir(tmp_path):
    """创建临时 session 目录结构。"""
    sessions = tmp_path / ".chanlun" / "sessions"
    sessions.mkdir(parents=True)
    return sessions


@pytest.fixture
def session_file(session_dir):
    """创建一个基本 session 文件。"""
    path = session_dir / "2026-03-04-1200-session.md"
    path.write_text(
        "# Session\n\n"
        "**时间**: 2026-03-04-1200\n"
        "**分支**: main\n"
        "**最新提交**: abc1234\n"
        "**模式**: roadmap\n\n"
        "## 工位\n\n"
        "### ws-alpha\n"
        "- 状态: pending\n\n"
        "## 产出记录\n\n"
        "（工位完成后增量追加）\n",
        encoding="utf-8",
    )
    return path


# ═══════════════════════════════════════════════════════════════
# session_update.py tests
# ═══════════════════════════════════════════════════════════════


class TestAppendOutput:
    def test_append_inserts_after_marker(self, session_file):
        append_output(session_file, "ws-alpha: 完成 3 个测试")
        content = session_file.read_text(encoding="utf-8")
        assert "- ws-alpha: 完成 3 个测试" in content
        # 原标记仍在
        assert "## 产出记录" in content

    def test_append_multiple(self, session_file):
        append_output(session_file, "ws-alpha: 产出A")
        append_output(session_file, "ws-beta: 产出B")
        content = session_file.read_text(encoding="utf-8")
        assert "- ws-alpha: 产出A" in content
        assert "- ws-beta: 产出B" in content

    def test_append_creates_marker_section_if_missing(self, session_dir):
        path = session_dir / "2026-03-04-1300-session.md"
        path.write_text("# Session\n\n**时间**: 2026-03-04-1300\n", encoding="utf-8")
        append_output(path, "ws-gamma: 新产出")
        content = path.read_text(encoding="utf-8")
        assert "## 产出记录" in content
        assert "- ws-gamma: 新产出" in content

    def test_append_updates_timestamp_marker(self, session_file, tmp_path):
        # .last-session-append 在 .chanlun/ 下（session_file 在 .chanlun/sessions/）
        append_output(session_file, "ws-alpha: 产出")
        marker = session_file.parent.parent / ".last-session-append"
        assert marker.exists()
        ts = float(marker.read_text(encoding="utf-8").strip())
        assert time.time() - ts < 5  # 5秒内


class TestFinalizeSession:
    def test_finalize_updates_head(self, session_file):
        finalize_session(session_file)
        content = session_file.read_text(encoding="utf-8")
        assert "## 收尾" in content
        assert "完成时间:" in content

    def test_finalize_idempotent(self, session_file):
        finalize_session(session_file)
        finalize_session(session_file)
        content = session_file.read_text(encoding="utf-8")
        # 只有一个收尾章节
        assert content.count("## 收尾") == 1


class TestCreateSession:
    def test_create_session_basic(self, tmp_path):
        scan = {
            "mode": "roadmap",
            "definitions": 5,
            "settled": 10,
            "pending": 2,
            "workstations": [
                {"name": "ws-test", "status": "new", "priority": "roadmap"},
            ],
        }
        # 需要 sessions 目录
        (tmp_path / ".chanlun" / "sessions").mkdir(parents=True)
        import scripts.session_update as su
        original_root = su.get_root

        def fake_root():
            return tmp_path

        su.get_root = fake_root
        try:
            path = create_session(tmp_path, scan)
            assert path.exists()
            content = path.read_text(encoding="utf-8")
            assert "ws-test" in content
            assert "roadmap" in content
        finally:
            su.get_root = original_root


# ═══════════════════════════════════════════════════════════════
# completion_persist_guard.py tests
# ═══════════════════════════════════════════════════════════════


class TestMarkAppend:
    def test_mark_creates_file(self, tmp_path):
        result = mark_append(tmp_path)
        marker = tmp_path / ".chanlun" / ".last-session-append"
        assert marker.exists()
        assert result["action"] == "mark_append"
        ts = float(marker.read_text(encoding="utf-8").strip())
        assert time.time() - ts < 5

    def test_mark_overwrites(self, tmp_path):
        mark_append(tmp_path)
        time.sleep(0.05)
        mark_append(tmp_path)
        marker = tmp_path / ".chanlun" / ".last-session-append"
        ts = float(marker.read_text(encoding="utf-8").strip())
        assert time.time() - ts < 1


class TestCheckAppend:
    def test_no_marker_returns_warning(self, tmp_path):
        result = check_append(tmp_path, "session.md")
        assert result["has_recent_append"] is False
        assert result["warning"] is not None
        assert "session_append.sh" in result["warning"]

    def test_recent_marker_returns_ok(self, tmp_path):
        mark_append(tmp_path)
        result = check_append(tmp_path, "session.md")
        assert result["has_recent_append"] is True
        assert result["warning"] is None

    def test_stale_marker_returns_warning(self, tmp_path):
        marker = tmp_path / ".chanlun" / ".last-session-append"
        marker.parent.mkdir(parents=True, exist_ok=True)
        stale_ts = time.time() - STALE_THRESHOLD_SECONDS - 10
        marker.write_text(str(stale_ts), encoding="utf-8")

        result = check_append(tmp_path, "session.md")
        assert result["has_recent_append"] is False
        assert result["warning"] is not None
        assert "批量处理" in result["warning"]

    def test_corrupted_marker(self, tmp_path):
        marker = tmp_path / ".chanlun" / ".last-session-append"
        marker.parent.mkdir(parents=True, exist_ok=True)
        marker.write_text("not a number", encoding="utf-8")

        result = check_append(tmp_path, "session.md")
        assert result["has_recent_append"] is False
        assert result["warning"] is not None


class TestClearMarker:
    def test_clear_removes_file(self, tmp_path):
        mark_append(tmp_path)
        marker = tmp_path / ".chanlun" / ".last-session-append"
        assert marker.exists()

        clear_marker(tmp_path)
        assert not marker.exists()

    def test_clear_nonexistent_no_error(self, tmp_path):
        result = clear_marker(tmp_path)
        assert result["action"] == "clear_marker"
