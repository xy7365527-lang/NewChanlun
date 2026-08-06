"""Security regression tests for the double-helix Bash hook."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
from pathlib import Path


HOOK_PATH = Path(__file__).resolve().parents[1] / ".claude/hooks/double-helix-verify.sh"


def _run(command: list[str], cwd: Path) -> None:
    subprocess.run(command, cwd=cwd, check=True, capture_output=True, text=True)


def test_double_helix_hook_treats_staged_diff_as_data(tmp_path: Path) -> None:
    """A staged diff containing Python syntax must not execute inside the hook."""
    repo = tmp_path / "repo"
    repo.mkdir()
    _run(["git", "init"], repo)
    _run(["git", "config", "user.email", "test@example.com"], repo)
    _run(["git", "config", "user.name", "Test User"], repo)

    modes_dir = repo / "src/newchan/gemini"
    modes_dir.mkdir(parents=True)
    (modes_dir / "modes.py").write_text(
        "class Result:\n"
        "    response = 'CLEAN'\n\n"
        "def decide(subject, context):\n"
        "    return Result()\n",
        encoding="utf-8",
    )

    marker = tmp_path / "pwned"
    payload = (
        "x = '''';import pathlib;"
        f"pathlib.Path({str(marker)!r}).write_text('owned');x='''\n"
    )
    (repo / "payload.py").write_text(payload, encoding="utf-8")
    _run(["git", "add", "payload.py"], repo)

    hook_input = {
        "tool_input": {"command": "git commit -m safe"},
        "cwd": str(repo),
    }
    env = {
        **os.environ,
        "GOOGLE_API_KEY": "dummy-key",
    }
    result = subprocess.run(
        [str(HOOK_PATH)],
        input=json.dumps(hook_input),
        cwd=repo,
        env=env,
        check=False,
        capture_output=True,
        text=True,
    )

    assert result.returncode == 0
    assert not marker.exists()
    assert "permissionDecision" in result.stdout or result.stdout == ""


def test_double_helix_hook_is_available() -> None:
    """The hook must be present and executable when registered in .claude/settings.json."""
    assert HOOK_PATH.exists()
    assert os.access(HOOK_PATH, os.X_OK)
    assert shutil.which("git") is not None
