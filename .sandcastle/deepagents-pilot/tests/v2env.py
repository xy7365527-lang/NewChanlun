from __future__ import annotations

import json
import subprocess
from pathlib import Path

import httpx
from conftest import FAKE, MODELS, Env, git
from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric import rsa
from fake_github import FakeGitHub

from pilot1742.approval import SignedApprovals
from pilot1742.claims import ClaimStore
from pilot1742.delivery import RepoGateway
from pilot1742.executor import TicketExecutor, load_unit, open_v2_run, register
from pilot1742.github_app import WRITE_PERMISSIONS, AppIdentity, GitHubAppClient
from pilot1742.run import PilotRun, TaskSpec
from pilot1742.workunit import DEDICATED_MARKER, WorkUnit

TICKET = 4242
DEMO = {"demo/a.py": "def f():\n    return 1\n"}
V2_TASK = TaskSpec("补操作员恢复指南", "在 demo/ 下实现函数。", ("demo/",), False)


class SimulatedCrash(BaseException):
    """进程在该点死亡：不经任何 except Exception，锁随 fd 关闭释放。"""


def crash_at(point: str):
    def crash(at: str) -> None:
        if at == point:
            raise SimulatedCrash(at)
    return crash


class V2Env(Env):
    def __init__(self, tmp: Path):
        super().__init__(tmp)
        self.tmp = tmp
        self.bare = tmp / "origin.git"
        subprocess.run(["git", "init", "-q", "--bare", str(self.bare)], check=True)
        git(self.source, "push", "-q", str(self.bare), "main")
        key = rsa.generate_private_key(public_exponent=65537, key_size=2048)
        pem = tmp / "app.pem"
        pem.write_bytes(key.private_bytes(serialization.Encoding.PEM, serialization.PrivateFormat.PKCS8,
                                          serialization.NoEncryption()))
        pem.chmod(0o600)
        self.identity = AppIdentity(5486877, 175617978, 1150335160, "xy7365527-lang/NewChanlun",
                                    "newchanlun-deepagents-1742", pem)
        self.github = FakeGitHub(self.identity, self.bare)
        self.github.add_issue(TICKET, body=f"#1742 首证专用票\n\n{DEDICATED_MARKER}\n")
        self.claims_root = tmp / "claims"
        self.workers_log = tmp / "workers.jsonl"
        self.workers_log.write_text(json.dumps({"ticket": 1715, "branch": "sandcastle/issue-1715"}) + "\n")
        self.approvals = SignedApprovals(trust_path=tmp / "unconfigured-trust.json",
                                         authority_root=tmp / "unconfigured-authority")

    def gateway(self) -> RepoGateway:
        return RepoGateway(GitHubAppClient(self.identity, transport=httpx.MockTransport(self.github),
                                           permissions=WRITE_PERMISSIONS))

    def register(self, run_id: str = f"t{TICKET}-1", state_root: Path | None = None, ticket: int = TICKET,
                 candidate_approval: bool = False) -> WorkUnit:
        return register(state_root or self.home, self.gateway(), ticket=ticket, run_id=run_id, base=self.base,
                        task=V2_TASK, playbook="feature", scope_ref="pytest-scope", workers_log=self.workers_log,
                        source_repo=self.source, candidate_approval=candidate_approval)

    def load(self, run_id: str) -> WorkUnit:
        return load_unit(self.home, run_id)[0]

    def executor(self, unit: WorkUnit, **kw) -> TicketExecutor:
        kw.setdefault("approvals", self.approvals)
        return TicketExecutor(unit, self.gateway(), claims=ClaimStore(self.claims_root, lock_timeout=10),
                              workers_log=self.workers_log, **kw)

    def run(self, unit: WorkUnit, state_root: Path | None = None, **kw) -> PilotRun:
        crash = kw.pop("crash", None)
        executor = self.executor(unit, **({"crash": crash} if crash else {}))
        return open_v2_run(state_root or self.home, unit, V2_TASK, executor, MODELS, launcher=FAKE,
                           source_repo=self.source, **kw)

    def to_pass(self, run: PilotRun, review: str = "VERDICT: PASS\n可以") -> str:
        self.control({"author": {"write": DEMO}, "review": {"summary": review}})
        run.run_worker("author", "实现")
        head = run.commit_candidate().commit
        run.run_worker("review", "审")
        return head

    def claim_record(self) -> dict:
        return json.loads((self.claims_root / f"{TICKET}.json").read_text())

    def writes(self) -> list[tuple[str, str]]:
        return [(m, p) for m, p, _ in self.github.requests if m != "GET" and not p.startswith("/app")]
