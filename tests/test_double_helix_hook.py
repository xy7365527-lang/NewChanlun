"""Regression tests for the double helix verification hook."""

from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path


HOOK_PATH = Path(__file__).resolve().parents[1] / ".claude/hooks/double-helix-verify.sh"


def _run(command: list[str], cwd: Path) -> None:
    subprocess.run(command, cwd=cwd, check=True, capture_output=True, text=True)


def test_hook_does_not_execute_staged_diff_as_python(tmp_path: Path) -> None:
    repo = tmp_path / "repo"
    repo.mkdir()
    _run(["git", "init"], repo)

    gemini_dir = repo / "src/newchan/gemini"
    gemini_dir.mkdir(parents=True)
    (repo / "src/newchan/__init__.py").write_text("", encoding="utf-8")
    (gemini_dir / "__init__.py").write_text("", encoding="utf-8")
    (gemini_dir / "modes.py").write_text(
        "\n".join(
            [
                "from pathlib import Path",
                "",
                "class Result:",
                "    response = 'CLEAN'",
                "",
                "def decide(subject, context):",
                "    Path('gemini_context.txt').write_text(context, encoding='utf-8')",
                "    return Result()",
                "",
            ]
        ),
        encoding="utf-8",
    )

    malicious = "'''\n__import__('pathlib').Path('pwned').write_text('x')\n'''"
    (repo / "danger.py").write_text(malicious, encoding="utf-8")
    _run(["git", "add", "danger.py"], repo)

    payload = {
        "tool_input": {"command": "git commit -m hook-test"},
        "cwd": str(repo),
    }
    env = os.environ.copy()
    env["GOOGLE_API_KEY"] = "fake-key"

    result = subprocess.run(
        [str(HOOK_PATH)],
        cwd=repo,
        input=json.dumps(payload),
        env=env,
        check=True,
        capture_output=True,
        text=True,
    )

    assert "pwned" not in result.stdout
    assert not (repo / "pwned").exists()
    context = (repo / "gemini_context.txt").read_text(encoding="utf-8")
    assert "danger.py" in context
    assert "__import__" not in context
    assert "Staged diff 摘要" in context
