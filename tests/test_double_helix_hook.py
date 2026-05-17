from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / ".claude/hooks/double-helix-verify.sh"


def _prepare_repo(repo: Path, staged_content: str) -> None:
    subprocess.run(["git", "init"], cwd=repo, check=True, stdout=subprocess.DEVNULL)
    (repo / "sample.txt").write_text(staged_content, encoding="utf-8")
    subprocess.run(["git", "add", "sample.txt"], cwd=repo, check=True)

    gemini_pkg = repo / "src" / "newchan" / "gemini"
    gemini_pkg.mkdir(parents=True)
    (gemini_pkg.parent / "__init__.py").write_text("", encoding="utf-8")
    (gemini_pkg / "__init__.py").write_text("", encoding="utf-8")
    (gemini_pkg / "modes.py").write_text(
        "\n".join([
            "from pathlib import Path",
            "",
            "class Result:",
            "    response = 'CLEAN'",
            "",
            "def decide(subject, context):",
            "    Path('called.txt').write_text(context, encoding='utf-8')",
            "    return Result()",
            "",
        ]),
        encoding="utf-8",
    )


def _run_hook(repo: Path, env_extra: dict[str, str] | None = None) -> subprocess.CompletedProcess[str]:
    payload = {
        "tool_input": {"command": "git commit -m test"},
        "cwd": str(repo),
    }
    env = os.environ.copy()
    env.update(env_extra or {})
    return subprocess.run(
        [str(SCRIPT)],
        input=json.dumps(payload),
        text=True,
        cwd=repo,
        env=env,
        capture_output=True,
        check=False,
    )


def test_gemini_diff_verification_is_opt_in(tmp_path: Path) -> None:
    _prepare_repo(tmp_path, "api_key = 'secret'\n")

    result = _run_hook(tmp_path, {"GOOGLE_API_KEY": "fake"})

    assert result.returncode == 0
    assert not (tmp_path / "called.txt").exists()


def test_opt_in_passes_diff_as_data_not_python_source(tmp_path: Path) -> None:
    marker = "''' ; raise RuntimeError('diff executed as code') ; '''"
    _prepare_repo(tmp_path, f"payload = {marker!r}\n")

    result = _run_hook(
        tmp_path,
        {
            "GOOGLE_API_KEY": "fake",
            "NEWCHAN_ENABLE_GEMINI_DIFF_VERIFY": "1",
        },
    )

    assert result.returncode == 0
    context = (tmp_path / "called.txt").read_text(encoding="utf-8")
    assert marker in context
