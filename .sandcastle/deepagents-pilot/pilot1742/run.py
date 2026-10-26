"""#1742 唯一副作用拥有者 PilotRun。

每个副作用都是“读现存事实 -> domain 纯函数决策 -> 只做差额 -> 写一次性回执”。
LangGraph checkpoint 只保存模型上下文；派工去重依据是 codex-child 的排他 outputDir，提交去重依据是 git 本身。
"""
from __future__ import annotations

import fcntl
import json
import os
import re
import shutil
import signal
import subprocess
import threading
import time
from dataclasses import asdict, dataclass
from datetime import datetime, timezone
from pathlib import Path

import yaml

from .domain import (
    MODEL_FAMILY,
    ROLE_MODE,
    TICKET,
    TRAILER_KEY,
    ApprovalRecord,
    Approved,
    Awaiting,
    CommitIntent,
    CommitReceipt,
    EffectUnknown,
    GateDecision,
    HeadObservation,
    HumanSource,
    ModelSlug,
    NotStarted,
    Role,
    RunFacts,
    RunId,
    Running,
    Scope,
    Sha,
    Terminal,
    WorkerFact,
    WorkerKey,
    WorkerObservation,
    WorkerReceipt,
    classify_worker,
    current_head,
    decide_gate,
    derive_phase,
    dispatch_refusal,
    outside_allowed,
    parse_model,
    parse_sha,
    parse_verdict,
    reconcile_commit,
)

SOURCE_REPO = Path(__file__).resolve().parents[3]
DEFAULT_HOME = Path("/Volumes/AgentStorage/issue1742-deepagents-pilot-20261027")
PSTACK_MODELS = Path.home() / ".agents" / "pstack-models.md"
CRASH_POINTS = ("after_intent_before_spawn", "after_spawn_before_launch_record", "after_commit_before_receipt")
WORKER_IDLE_SECONDS = 900
WORKER_TOTAL_SECONDS = 5400
_ENV_KEEP = ("PATH", "HOME", "USER", "LOGNAME", "SHELL", "TMPDIR", "LANG", "LC_ALL", "LC_CTYPE", "CODEX_HOME",
             "CLIPROXY_LOCAL_KEY", "HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "NO_PROXY",
             "http_proxy", "https_proxy", "all_proxy", "no_proxy")


class Halted(Exception):
    """外部事实无法证据收敛，停止该动作，等主控处置。"""


class ToolRefusal(Exception):
    """阶段前置条件不满足；没有发生任何副作用。"""


class Busy(Exception):
    """同一 run 已有控制进程持锁。"""


def _now() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds")


def _write_once(path: Path, data: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_name(path.name + ".tmp")
    tmp.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n")
    os.replace(tmp, path)


def _git(root: Path | str, *args: str, env: dict | None = None) -> str:
    return subprocess.run(["git", "-C", str(root), *args], check=True, capture_output=True, text=True,
                          env=env).stdout.strip()


@dataclass(frozen=True)
class TaskSpec:
    title: str
    requirement: str
    allowed_prefixes: tuple[str, ...]


def parse_task(text: str) -> TaskSpec:
    match = re.match(r"---\n(.*?)\n---\n(.*)", text, re.S)
    if not match:
        raise ValueError("任务文件需以 --- 包住的 title/allowed_prefixes 头开始")
    head = yaml.safe_load(match.group(1)) or {}
    prefixes = tuple(head.get("allowed_prefixes") or ())
    if not head.get("title") or not prefixes or not all(isinstance(p, str) and p.endswith("/") and not p.startswith("/")
                                                        and ".." not in p for p in prefixes):
        raise ValueError("任务头需要 title 与以 / 结尾的相对目录 allowed_prefixes")
    if not match.group(2).strip():
        raise ValueError("任务正文为空")
    return TaskSpec(str(head["title"]), match.group(2).strip(), prefixes)


def role_models(text: str) -> tuple[ModelSlug, ModelSlug, ModelSlug]:
    """从 pstack-models.md 取作者、独审与大脑模型：feature 行、judgment 行、hardest 行。"""
    lines = dict(line.split(":", 1) for line in text.splitlines() if ":" in line and not line.startswith("#"))
    pick = {k.strip(): v.strip().split(",")[0].strip() for k, v in lines.items()}
    author = parse_model(pick["feature, refactoring"])
    reviewer = parse_model(pick["judgment and prose"])
    brain = parse_model(pick["hardest tasks"])
    return author, reviewer, brain


@dataclass(frozen=True)
class RunSpec:
    run_id: RunId
    ticket: int
    scope: Scope
    base: Sha
    task: TaskSpec
    author_model: ModelSlug
    reviewer_model: ModelSlug
    brain_model: ModelSlug
    author_checkout: str
    review_checkout: str
    created_at: str

    def to_json(self) -> dict:
        return asdict(self)

    @staticmethod
    def from_json(data: dict) -> "RunSpec":
        task = TaskSpec(data["task"]["title"], data["task"]["requirement"], tuple(data["task"]["allowed_prefixes"]))
        return RunSpec(RunId(data["run_id"]), data["ticket"], data["scope"], parse_sha(data["base"]), task,
                       parse_model(data["author_model"]), parse_model(data["reviewer_model"]),
                       parse_model(data["brain_model"]), data["author_checkout"], data["review_checkout"],
                       data["created_at"])


def _porcelain_paths(root: Path | str) -> tuple[str, ...]:
    raw = subprocess.run(["git", "-C", str(root), "status", "--porcelain=v1", "-z", "--untracked-files=all"],
                         check=True, capture_output=True, text=True).stdout
    entries = raw.split("\0")
    paths: list[str] = []
    i = 0
    while i < len(entries):
        entry = entries[i]
        i += 1
        if not entry:
            continue
        paths.append(entry[3:])
        if entry[0] in "RC":
            paths.append(entries[i])
            i += 1
    return tuple(sorted(set(paths)))


def _status_text_paths(status: str) -> tuple[str, ...]:
    paths: list[str] = []
    for line in status.splitlines():
        if len(line) > 3:
            paths.extend(part.strip('"') for part in line[3:].split(" -> "))
    return tuple(sorted(set(paths)))


def parse_result(key: WorkerKey, data: dict, output_dir: Path) -> WorkerReceipt:
    if data.get("ticket") != TICKET or data.get("status") not in ("completed", "failed", "cancelled"):
        raise ValueError("result.json 票号或终态不符")
    expected = parse_sha(data.get("expected_head"))
    if expected != key.target_head or data.get("mode") != ROLE_MODE[key.role]:
        raise ValueError("result.json 的基线或模式与派工键不符")
    after = data.get("checkout_after") or {}
    result = data.get("result") or {}
    return WorkerReceipt(
        key=key, child_run_id=str(data.get("run_id")), status=data["status"], model=parse_model(data.get("model")),
        expected_head=expected, head_after=parse_sha(after["head"]) if after.get("head") else None,
        changed_paths_after=_status_text_paths(after.get("status") or ""),
        external_session_id=data.get("external_session_id"), external_session_log=data.get("external_session_log"),
        summary=str(result.get("summary") or ""), findings=tuple(result.get("findings") or ()),
        validation=tuple(result.get("validation") or ()), failure_reason=data.get("failure_reason"),
        output_dir=str(output_dir),
    )


_LEDGER_KINDS = ("tracker_comment", "user_message")


def parse_approval_ledger(text: str) -> tuple[list[ApprovalRecord], list[str]]:
    """主控登记的人类批准账本。合成、代理来源或缺出处的行不计入，只进诊断。"""
    records: list[ApprovalRecord] = []
    problems: list[str] = []
    comment_url = re.compile(rf"https://github\.com/xy7365527-lang/NewChanlun/issues/{TICKET}#issuecomment-\d+")
    for n, line in enumerate(text.splitlines(), 1):
        if not line.strip():
            continue
        try:
            row = json.loads(line)
            source = row["source"]
            kind, reference, registered_by = source["kind"], str(source["reference"]), str(source["registered_by"])
            if kind not in _LEDGER_KINDS or not reference.strip() or not registered_by.strip():
                raise ValueError(f"来源不是已核实的人类来源：{kind!r}")
            if kind == "tracker_comment" and not comment_url.fullmatch(reference):
                raise ValueError("tracker 来源必须是本票评论 URL")
            if row["scope"] not in ("pilot_acceptance", "main_merge") or row["decision"] not in ("approve", "reject"):
                raise ValueError("scope 或 decision 无效")
            head = parse_sha(row["head"]) if row.get("head") is not None else None
            if row["decision"] == "approve" and head is None:
                raise ValueError("批准必须绑定完整 head")
            records.append(ApprovalRecord(n, int(row["ticket"]), row["scope"], head, row["decision"],
                                          HumanSource(kind, reference, registered_by)))
        except (KeyError, TypeError, ValueError, json.JSONDecodeError) as error:
            problems.append(f"第 {n} 行不计入：{error}")
    return records, problems


def _ps_rows() -> list[tuple[int, int, str, str]]:
    out = subprocess.run(["ps", "-axww", "-o", "pid=,pgid=,stat=,command="], check=True, capture_output=True,
                         text=True).stdout
    rows = []
    for line in out.splitlines():
        parts = line.split(None, 3)
        if len(parts) == 4:
            rows.append((int(parts[0]), int(parts[1]), parts[2], parts[3]))
    return rows


def supervisor_alive(output_dir: Path, rows: list[tuple[int, int, str, str]]) -> bool:
    """身份 = 命令行里带本键唯一的 --output-dir；僵尸进程不算。"""
    needle = f"--output-dir {output_dir}"
    return any(not stat.startswith("Z") and (cmd.endswith(needle) or f"{needle} " in cmd)
               for _, _, stat, cmd in rows)


def worker_group_alive(state: dict | None, rows: list[tuple[int, int, str, str]]) -> bool:
    """身份 = state.json 记录的进程组首进程，且命令行是 codex。"""
    pid = (state or {}).get("process_pid")
    return isinstance(pid, int) and any(p == pid and g == pid and not s.startswith("Z") and "codex" in c
                                        for p, g, s, c in rows)


class PilotRun:
    """一个 run 目录的唯一写者；进程内工具调用经 mutex 串行，进程间经 owner.lock。"""

    def __init__(self, home: Path, spec: RunSpec, *, launcher: list[str] | None = None, crash_at: str | None = None,
                 source_repo: Path = SOURCE_REPO):
        self.home = home
        self.source_repo = source_repo
        self.spec = spec
        self.dir = home / "runs" / spec.run_id
        self.launcher = launcher or [shutil.which("node") or "node", "--import", "tsx", ".sandcastle/codex-child.mts"]
        self.crash_at = crash_at
        self._mutex = threading.Lock()
        self._lock_fd: int | None = None
        self._children: dict[str, subprocess.Popen] = {}

    @staticmethod
    def run_dir(home: Path, run_id: RunId) -> Path:
        return home / "runs" / run_id

    @classmethod
    def create(cls, home: Path, run_id: RunId, task: TaskSpec, base: Sha, models_text: str, **kw) -> "PilotRun":
        author, reviewer, brain = role_models(models_text)
        if MODEL_FAMILY[author] == MODEL_FAMILY[reviewer]:
            raise ValueError("pstack-models 的作者与独审是同一家族，不能构成异家族独审")
        root = cls.run_dir(home, run_id)
        root.mkdir(parents=True, exist_ok=True)
        checkouts = home / "checkouts" / run_id
        spec = RunSpec(run_id, TICKET, "pilot_acceptance", base, task, author, reviewer, brain,
                       str(checkouts / "author"), str(checkouts / "review"), _now())
        run = cls(home, spec, **kw)
        run.acquire()
        if not (root / "run.json").exists():
            _write_once(root / "run.json", spec.to_json())
        run.spec = RunSpec.from_json(json.loads((root / "run.json").read_text()))
        run.trail("run_created", spec=run.spec.to_json())
        run.ensure_checkouts()
        return run

    @classmethod
    def open(cls, home: Path, run_id: RunId, **kw) -> "PilotRun":
        path = cls.run_dir(home, run_id) / "run.json"
        if not path.exists():
            raise FileNotFoundError(f"run {run_id} 不存在：{path}")
        return cls(home, RunSpec.from_json(json.loads(path.read_text())), **kw)

    def acquire(self) -> None:
        self.dir.mkdir(parents=True, exist_ok=True)
        fd = os.open(self.dir / "owner.lock", os.O_RDWR | os.O_CREAT, 0o600)
        try:
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            os.close(fd)
            raise Busy(f"run {self.spec.run_id} 已有控制进程持锁，本进程未做任何副作用") from None
        os.ftruncate(fd, 0)
        os.write(fd, f"{os.getpid()}\n".encode())
        self._lock_fd = fd

    def release(self) -> None:
        if self._lock_fd is not None:
            os.close(self._lock_fd)
            self._lock_fd = None

    def trail(self, event: str, **fields) -> None:
        with open(self.dir / "trail.jsonl", "a") as fh:
            fh.write(json.dumps({"at": _now(), "pid": os.getpid(), "event": event, **fields}, ensure_ascii=False) + "\n")

    def _crash(self, point: str) -> None:
        if self.crash_at == point:
            self.trail("crash_injected", point=point)
            os.kill(os.getpid(), signal.SIGKILL)

    # ---- 观测 ----------------------------------------------------------------

    def _branch(self, role: Role) -> str:
        return f"codex/deepagents-pilot-1742-{self.spec.run_id}-{role}"

    def _checkout(self, role: Role) -> Path:
        return Path(self.spec.author_checkout if role == "author" else self.spec.review_checkout)

    def ensure_checkouts(self) -> None:
        for role in ("author", "review"):
            self._ensure_checkout(role, self.spec.base)

    def _ensure_checkout(self, role: Role, head: Sha) -> None:
        path, branch = self._checkout(role), self._branch(role)
        if not path.exists():
            path.parent.mkdir(parents=True, exist_ok=True)
            exists = subprocess.run(["git", "-C", str(self.source_repo), "rev-parse", "--verify", "-q",
                                     f"refs/heads/{branch}"], capture_output=True).returncode == 0
            args = ["worktree", "add", "-q", str(path), branch] if exists else \
                ["worktree", "add", "-q", "-b", branch, str(path), head]
            _git(self.source_repo, *args)
            self.trail("checkout_created", role=role, path=str(path), branch=branch)
        if _git(path, "symbolic-ref", "--short", "HEAD") != branch:
            raise Halted(f"{path} 不在预期分支 {branch}")

    def _key_dir(self, key: WorkerKey) -> Path:
        return self.dir / "workers" / key.name

    def _keys(self) -> list[WorkerKey]:
        names = set()
        for sub in ("intents", "workers"):
            folder = self.dir / sub
            if folder.exists():
                names.update(p.name.removesuffix(".json") for p in folder.iterdir() if not p.name.endswith(".tmp"))
        keys = []
        for name in sorted(names):
            role, _, head = name.partition("-")
            keys.append(WorkerKey(self.spec.run_id, role, parse_sha(head)))  # type: ignore[arg-type]
        return keys

    def _observe(self, key: WorkerKey, rows: list[tuple[int, int, str, str]]) -> WorkerFact:
        out = self._key_dir(key)
        result = None
        state = None
        try:
            if (out / "result.json").exists():
                result = parse_result(key, json.loads((out / "result.json").read_text()), out)
            if (out / "state.json").exists():
                state = json.loads((out / "state.json").read_text())
        except (ValueError, KeyError, json.JSONDecodeError) as error:
            return EffectUnknown(f"工蜂回执无法解析：{error}")
        return classify_worker(WorkerObservation(out.exists(), result, supervisor_alive(out, rows),
                                                 worker_group_alive(state, rows)))

    def _commit_receipts(self) -> list[CommitReceipt]:
        folder = self.dir / "commits"
        receipts = []
        if folder.exists():
            for path in sorted(folder.glob("*.receipt.json")):
                data = json.loads(path.read_text())
                receipts.append(CommitReceipt(self._parse_intent(data["intent"]), parse_sha(data["commit"]),
                                              bool(data["recovered"])))
        return receipts

    def _parse_intent(self, data: dict) -> CommitIntent:
        role, _, head = data["author"].partition("-")
        return CommitIntent(WorkerKey(self.spec.run_id, role, parse_sha(head)), parse_sha(data["parent"]),  # type: ignore[arg-type]
                            data["tree"], tuple(data["paths"]))

    @staticmethod
    def _intent_json(intent: CommitIntent) -> dict:
        return {"author": intent.author.name, "parent": intent.parent, "tree": intent.tree,
                "paths": list(intent.paths), "trailer": f"{TRAILER_KEY}: {intent.trailer}"}

    def facts(self) -> RunFacts:
        rows = _ps_rows()
        workers = {key: self._observe(key, rows) for key in self._keys()}
        commits = tuple(self._commit_receipts())
        author_model_of = {}
        for commit in commits:
            fact = workers.get(commit.intent.author)
            if isinstance(fact, Terminal):
                author_model_of[commit.commit] = fact.receipt.model
        verdicts = {}
        for key, fact in workers.items():
            if key.role == "review" and isinstance(fact, Terminal) and key.target_head in author_model_of:
                verdicts[key.target_head] = parse_verdict(fact.receipt, author_model_of[key.target_head])
        return RunFacts(self.spec.run_id, self.spec.base, commits, workers, verdicts)

    # ---- 工具：派工 ------------------------------------------------------------

    def run_worker(self, role: Role, brief: str) -> WorkerReceipt:
        with self._mutex:
            facts = self.facts()
            head = current_head(facts)
            key = WorkerKey(self.spec.run_id, role, head)
            if key not in facts.workers:
                refusal = dispatch_refusal(facts, key)
                if refusal:
                    self.trail("refused", tool="run_worker", key=key.name, reason=refusal)
                    raise ToolRefusal(refusal)
                self._prepare_dispatch(key, brief, facts)
            else:
                self.trail("adopt", key=key.name, fact=type(facts.workers[key]).__name__)
            return self._drive(key)

    def _prepare_dispatch(self, key: WorkerKey, brief: str, facts: RunFacts) -> None:
        checkout = self._checkout(key.role)
        if key.role == "review":
            current = _git(checkout, "rev-parse", "HEAD")
            if current != key.target_head:
                if _porcelain_paths(checkout):
                    raise Halted(f"独审 checkout 不干净，不能切到 {key.target_head}")
                _git(checkout, "switch", "-q", "-C", self._branch("review"), key.target_head)
        prompt = self.dir / "prompts" / f"{key.name}.md"
        prompt.parent.mkdir(parents=True, exist_ok=True)
        prompt.write_text(self._prompt(key, brief, facts))
        model = self.spec.author_model if key.role == "author" else self.spec.reviewer_model
        _write_once(self.dir / "intents" / f"{key.name}.json", {
            "key": key.name, "run_id": self.spec.run_id, "role": key.role, "target_head": key.target_head,
            "mode": ROLE_MODE[key.role], "model": model, "checkout": str(checkout), "prompt_file": str(prompt),
            "created_at": _now()})
        self.trail("intent", key=key.name, model=model)
        self._crash("after_intent_before_spawn")

    def _launch(self, key: WorkerKey) -> None:
        intent = json.loads((self.dir / "intents" / f"{key.name}.json").read_text())
        out = self._key_dir(key)
        out.parent.mkdir(parents=True, exist_ok=True)
        argv = [*self.launcher, "--ticket", str(TICKET), "--parent-thread-id", f"pilot1742-{self.spec.run_id}",
                "--manager-agent-id", f"/deepagents/pilot1742/{self.spec.run_id}", "--checkout", intent["checkout"],
                "--expected-head", key.target_head, "--mode", intent["mode"], "--model", intent["model"],
                "--idle-timeout-seconds", str(WORKER_IDLE_SECONDS), "--timeout-seconds", str(WORKER_TOTAL_SECONDS),
                "--prompt-file", intent["prompt_file"], "--output-dir", str(out)]
        log_dir = self.dir / "launch"
        log_dir.mkdir(exist_ok=True)
        env = {k: os.environ[k] for k in _ENV_KEEP if os.environ.get(k)}
        with open(log_dir / f"{key.name}.log", "ab") as log:
            child = subprocess.Popen(argv, cwd=self.source_repo, stdin=subprocess.DEVNULL, stdout=log, stderr=log,
                                     env=env, start_new_session=True)
        self._children[key.name] = child
        self._crash("after_spawn_before_launch_record")
        _write_once(log_dir / f"{key.name}.{child.pid}.json", {"pid": child.pid, "argv": argv, "at": _now()})
        self.trail("dispatched", key=key.name, supervisor_pid=child.pid, model=intent["model"])

    def _drive(self, key: WorkerKey) -> WorkerReceipt:
        launched_here = False
        while True:
            fact = self._observe(key, _ps_rows())
            if isinstance(fact, Terminal):
                self.trail("terminal", key=key.name, status=fact.receipt.status, model=fact.receipt.model,
                           session=fact.receipt.external_session_id)
                return fact.receipt
            if isinstance(fact, EffectUnknown):
                self.trail("halted", key=key.name, reason=fact.reason)
                raise Halted(f"{key.name}：{fact.reason}")
            if isinstance(fact, NotStarted):
                child = self._children.get(key.name)
                if launched_here and child is not None and child.poll() is not None:
                    self.trail("halted", key=key.name, reason="监督进程在认领 outputDir 前退出")
                    raise Halted(f"{key.name}：codex-child 在认领 outputDir 前退出，见 launch/{key.name}.log")
                if not launched_here:
                    self._launch(key)
                    launched_here = True
            child = self._children.get(key.name)
            if child is not None:
                child.poll()
            time.sleep(2)

    def _prompt(self, key: WorkerKey, brief: str, facts: RunFacts) -> str:
        task = self.spec.task
        scope = "、".join(task.allowed_prefixes)
        if key.role == "author":
            rework = ""
            verdict = facts.verdicts.get(key.target_head)
            review = facts.workers.get(WorkerKey(self.spec.run_id, "review", key.target_head))
            if verdict is not None and verdict.kind == "BLOCK" and isinstance(review, Terminal):
                findings = "\n".join(f"- {f}" for f in review.receipt.findings)
                rework = (f"\n## 返修依据\n候选 {key.target_head} 被独审（{verdict.reviewer_model}）判 BLOCK。\n"
                          f"{review.receipt.summary}\n{findings}\n")
            return (f"# #1742 试点作者任务：{task.title}\n\nrun {self.spec.run_id}，基线 {key.target_head}。\n\n"
                    f"## 需求原文\n{task.requirement}\n\n## 写集\n只允许改动 {scope} 下的文件；"
                    f"写集外的改动会让宿主拒绝提交。不要 commit，宿主负责受管提交。\n{rework}\n"
                    f"## 主控 brief\n{brief.strip()}\n")
        author_commit = next(c for c in facts.commits if c.commit == key.target_head)
        return (f"# #1742 试点独立审阅：{task.title}\n\nrun {self.spec.run_id}。审阅候选提交 {key.target_head}"
                f"（父提交 {author_commit.intent.parent}）。用 `git show {key.target_head}` 与 "
                f"`git diff {author_commit.intent.parent} {key.target_head}` 读改动，可以运行只读检查，不得写 checkout。\n\n"
                f"## 需求原文\n{task.requirement}\n\n## 写集\n{scope}\n\n## 结论协议\n"
                "summary 第一行必须严格是 `VERDICT: PASS` 或 `VERDICT: BLOCK`，第二行起写理由。"
                "BLOCK 时 findings 逐条写必须修复的缺陷。status 填 completed 表示审阅已完成，与结论无关。\n\n"
                f"## 主控关注点（不能指定结论）\n{brief.strip()}\n")

    # ---- 工具：宿主提交 ----------------------------------------------------------

    def commit_candidate(self) -> CommitReceipt:
        with self._mutex:
            facts = self.facts()
            head = current_head(facts)
            akey = WorkerKey(self.spec.run_id, "author", head)
            fact = facts.workers.get(akey)
            if not isinstance(fact, Terminal) or fact.receipt.status != "completed":
                landed = [c for c in facts.commits if c.commit == head]
                if landed:
                    self.trail("commit_replay", commit=head)
                    return landed[0]
                raise ToolRefusal(f"当前 head {head} 没有已完成的作者工蜂可提交")
            folder = self.dir / "commits"
            intent_path, receipt_path = folder / f"{akey.name}.intent.json", folder / f"{akey.name}.receipt.json"
            checkout = self._checkout("author")
            intent = self._parse_intent(json.loads(intent_path.read_text())) if intent_path.exists() else None
            if intent is not None:
                state = reconcile_commit(intent, self._observe_head(checkout))
                if state == "committed":
                    return self._receipt(intent, checkout, receipt_path, recovered=True)
                if state == "conflict":
                    self.trail("halted", tool="commit_candidate", reason="HEAD 已变但与提交意图不符")
                    raise Halted("作者 checkout 的 HEAD 已变化，但 parent/tree/trailer 与提交意图不符")
            if _git(checkout, "rev-parse", "HEAD") != head:
                raise Halted("作者 checkout 不在当前 head")
            if (Path(_git(checkout, "rev-parse", "--absolute-git-dir")) / "index.lock").exists():
                raise Halted("作者 checkout 有 index.lock，git 操作可能仍在进行")
            paths = _porcelain_paths(checkout)
            if not paths:
                raise Halted("作者工蜂完成但没有留下改动")
            outside = outside_allowed(paths, self.spec.task.allowed_prefixes)
            if outside:
                self.trail("halted", tool="commit_candidate", reason="越界改动", paths=list(outside))
                raise Halted(f"改动越出写集：{list(outside)}")
            _git(checkout, "add", "-A", "--", *paths)
            tree = _git(checkout, "write-tree")
            if intent is not None and intent.tree != tree:
                raise Halted("作者 checkout 自提交意图写下后发生变化")
            if intent is None:
                intent = CommitIntent(akey, head, tree, paths)
                _write_once(intent_path, self._intent_json(intent))
                self.trail("commit_intent", author=akey.name, parent=head, tree=tree)
            message = (f"#1742 试点候选：{self.spec.task.title}\n\n作者工蜂 {fact.receipt.model}，"
                       f"run {self.spec.run_id}，基线 {head}。")
            _git(checkout, "commit", "-q", "-m", message, "--trailer", f"{TRAILER_KEY}: {intent.trailer}")
            self._crash("after_commit_before_receipt")
            if reconcile_commit(intent, self._observe_head(checkout)) != "committed":
                raise Halted("提交后观测不到与意图一致的提交")
            return self._receipt(intent, checkout, receipt_path, recovered=False)

    def _observe_head(self, checkout: Path) -> HeadObservation:
        head = parse_sha(_git(checkout, "rev-parse", "HEAD"))
        parents = _git(checkout, "rev-list", "--parents", "-n", "1", head).split()
        trailers = _git(checkout, "log", "-1", "--format=%(trailers:key=" + TRAILER_KEY + ",valueonly)", head)
        return HeadObservation(head, parse_sha(parents[1]) if len(parents) > 1 else None,
                               _git(checkout, "rev-parse", f"{head}^{{tree}}"),
                               tuple(t.strip() for t in trailers.splitlines() if t.strip()))

    def _receipt(self, intent: CommitIntent, checkout: Path, path: Path, *, recovered: bool) -> CommitReceipt:
        receipt = CommitReceipt(intent, parse_sha(_git(checkout, "rev-parse", "HEAD")), recovered)
        _write_once(path, {"intent": self._intent_json(intent), "commit": receipt.commit, "recovered": recovered,
                           "at": _now()})
        self.trail("commit_receipt", commit=receipt.commit, parent=intent.parent, recovered=recovered)
        return receipt

    # ---- 工具：门与状态 ----------------------------------------------------------

    def evaluate_gate(self, facts: RunFacts | None = None) -> tuple[GateDecision, list[str]]:
        facts = facts or self.facts()
        ledger = self.dir / "approvals.jsonl"
        records, problems = parse_approval_ledger(ledger.read_text() if ledger.exists() else "")
        head = current_head(facts)
        return decide_gate(ticket=TICKET, scope=self.spec.scope, candidate=head, verdict=facts.verdicts.get(head),
                           records=records), problems

    def gate(self) -> GateDecision:
        with self._mutex:
            decision, problems = self.evaluate_gate()
            self.trail("gate", decision=gate_json(decision), ledger_problems=problems)
            return decision

    def status(self) -> dict:
        facts = self.facts()
        head = current_head(facts)
        gate, problems = self.evaluate_gate(facts) if derive_phase(facts) == "gate" else (None, [])
        return {
            "run_id": self.spec.run_id, "ticket": TICKET, "scope": self.spec.scope, "base": self.spec.base,
            "current_head": head, "phase": derive_phase(facts),
            "models": {"brain": self.spec.brain_model, "author": self.spec.author_model,
                       "reviewer": self.spec.reviewer_model},
            "workers": {k.name: _fact_json(f) for k, f in facts.workers.items()},
            "commits": [{"commit": c.commit, "parent": c.intent.parent, "tree": c.intent.tree,
                         "recovered": c.recovered} for c in facts.commits],
            "verdicts": {h: v.kind for h, v in facts.verdicts.items()},
            "gate": gate_json(gate) if gate is not None else None, "ledger_problems": problems,
        }


def _fact_json(fact: WorkerFact) -> dict:
    if isinstance(fact, Terminal):
        r = fact.receipt
        return {"fact": "terminal", "status": r.status, "model": r.model, "session": r.external_session_id,
                "summary": r.summary[:400], "findings": list(r.findings), "changed_paths": list(r.changed_paths_after),
                "output_dir": r.output_dir}
    if isinstance(fact, EffectUnknown):
        return {"fact": "effect_unknown", "reason": fact.reason}
    return {"fact": "running" if isinstance(fact, Running) else "not_started"}


def gate_json(decision: GateDecision) -> dict:
    if isinstance(decision, Approved):
        return {"decision": "approved", "head": decision.record.head, "scope": decision.record.scope,
                "source": asdict(decision.record.source)}
    return {"decision": "awaiting_approval" if isinstance(decision, Awaiting) else "refused",
            "head": decision.head, "reasons": list(decision.reasons)}
