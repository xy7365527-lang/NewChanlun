import json
import subprocess
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from pilot1742.domain import parse_run_id, parse_sha  # noqa: E402
from pilot1742.run import PilotRun, TaskSpec  # noqa: E402

FAKE = [sys.executable, str(Path(__file__).with_name("fake_codex_child.py"))]
MODELS = "feature, refactoring: gpt-6.1-sol\njudgment and prose: claude-opus-5-5\nhardest tasks: claude-opus-5-5\n"
TASK = TaskSpec("测试任务", "在 demo/ 下实现函数。", ("demo/",))


def git(root, *args):
    return subprocess.run(["git", "-C", str(root), *args], check=True, capture_output=True, text=True).stdout.strip()


class Env:
    def __init__(self, tmp: Path):
        self.home = tmp / "home"
        self.source = tmp / "source"
        self.source.mkdir()
        git(self.source, "init", "-q", "-b", "main")
        git(self.source, "config", "user.name", "Fixture")
        git(self.source, "config", "user.email", "fixture@example.invalid")
        (self.source / "README").write_text("x\n")
        (self.source / ".gitignore").write_text("__pycache__/\n")
        git(self.source, "add", ".")
        git(self.source, "commit", "-qm", "base")
        self.base = parse_sha(git(self.source, "rev-parse", "HEAD"))
        self.home.mkdir()
        self.control({})

    def control(self, data: dict) -> None:
        (self.home / "fake-control.json").write_text(json.dumps(data))

    def claims(self) -> list[str]:
        path = self.home / "fake-claims.log"
        return path.read_text().split() if path.exists() else []

    def create(self, run_id="r1", models=MODELS, **kw) -> PilotRun:
        return PilotRun.create(self.home, parse_run_id(run_id), TASK, self.base, models, launcher=FAKE,
                               source_repo=self.source, **kw)

    def reopen(self, run_id="r1") -> PilotRun:
        run = PilotRun.open(self.home, parse_run_id(run_id), launcher=FAKE, source_repo=self.source)
        run.acquire()
        return run

    def driver(self, action: str, crash: str = "", run_id="r1") -> subprocess.Popen:
        return subprocess.Popen([sys.executable, str(Path(__file__).with_name("driver.py")), str(self.home), run_id,
                                 action, crash, str(self.source)])


@pytest.fixture
def env(tmp_path):
    return Env(tmp_path)
