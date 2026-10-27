import json
import subprocess
import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from pilot1742.domain import parse_run_id, parse_sha  # noqa: E402
from pilot1742.run import PilotRun, TaskSpec  # noqa: E402

FAKE = [sys.executable, str(Path(__file__).with_name("fake_codex_child.py"))]
MODELS = ("feature, refactoring: gpt-6.1-sol\njudgment and prose: claude-opus-5-5\nhardest tasks: claude-opus-5-5\n"
          "architect runners: claude-opus-5-5, gpt-6.1-sol\narena cross-judge pool: claude-opus-5-5, gpt-6.1-sol\n")
TASK = TaskSpec("测试任务", "在 demo/ 下实现函数。", ("demo/",), False)
DESIGN_TASK = TaskSpec("测试任务", "在 demo/ 下实现函数。", ("demo/",), True)


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

    def create(self, run_id="r1", models=MODELS, task=TASK, **kw) -> PilotRun:
        return PilotRun.create(self.home, parse_run_id(run_id), task, self.base, models, launcher=FAKE,
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


@pytest.fixture(autouse=True)
def no_operator_credential(monkeypatch):
    """测试从不读取操作员的 CLIProxy 凭据（本机 launchctl 里是真值，CI 上没有 launchctl）。
    需要构造生产模型时拿到占位值；真去调用模型会失败。"""
    from pydantic import SecretStr

    from pilot1742 import agent

    monkeypatch.setattr(agent, "managed_cliproxy_key", lambda: SecretStr("pytest"))


@pytest.fixture(autouse=True)
def isolated_skill_source(tmp_path, monkeypatch):
    """这里只验证技能文件加载机制，真实上游内容由 real4 集成记录验收。"""
    from pilot1742 import agent

    root = tmp_path / "skills"
    skill = root / "poteto-mode" / "SKILL.md"
    skill.parent.mkdir(parents=True)
    skill.write_text("---\nname: pstack-upstream-poteto-mode\ndescription: CI fixture\n---\n# Poteto mode\n")
    monkeypatch.setattr(agent, "SKILLS_ROOT", str(root) + "/")
    monkeypatch.setattr(agent, "POTETO_MODE", str(skill))
