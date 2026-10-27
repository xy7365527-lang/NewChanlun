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
from collections.abc import Iterator
from contextlib import contextmanager
from dataclasses import asdict, dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import TYPE_CHECKING, Any

import yaml

from .domain import (
    DESIGN_SLOTS,
    MODEL_FAMILY,
    ROLE_MODE,
    TICKET,
    TRAILER_KEY,
    V2_TRAILER_KEY,
    ApprovalRecord,
    Approved,
    Awaiting,
    CheckoutState,
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
    candidate_conflict,
    chain_head,
    classify_worker,
    current_head,
    decide_gate,
    declared_base,
    derive_phase,
    dispatch_refusal,
    outside_allowed,
    parse_key_name,
    parse_model,
    parse_sha,
    parse_verdict,
    reconcile_commit,
    synthesis_refusal,
)
from .workunit import WorkUnit, parse_work_unit

if TYPE_CHECKING:
    from .executor import TicketExecutor

SOURCE_REPO = Path(__file__).resolve().parents[3]
DEFAULT_HOME = Path("/Volumes/AgentStorage/issue1742-deepagents-pilot-20261027")
PSTACK_MODELS = Path.home() / ".agents" / "pstack-models.md"
SKILLS_ROOT = "/Users/silencehan/.codex/pstack-upstream-source/installed/skills/"
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
    require_design: bool


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
    require_design = head.get("require_design", False)
    if not isinstance(require_design, bool):
        raise ValueError("require_design 只能是 true 或 false")
    return TaskSpec(str(head["title"]), match.group(2).strip(), prefixes, require_design)


@dataclass(frozen=True)
class RolePlan:
    author: ModelSlug
    reviewer: ModelSlug
    brain: ModelSlug
    design: tuple[ModelSlug, ModelSlug]
    judge: ModelSlug


def plan_roles(text: str) -> RolePlan:
    """从 pstack-models.md 派生：feature 行作者、judgment 行独审、hardest 行主控、architect runners 行前两个做设计候选、
    arena cross-judge pool 里第一个与主控不同家族的模型做交叉裁判。"""
    rows = {key.strip(): [m.strip() for m in value.split(",") if m.strip()]
            for key, value in (line.split(":", 1) for line in text.splitlines()
                               if ":" in line and not line.lstrip().startswith("#"))}
    missing = [name for name in ("feature, refactoring", "judgment and prose", "hardest tasks", "architect runners",
                                 "arena cross-judge pool") if not rows.get(name)]
    if missing:
        raise ValueError(f"pstack-models.md 缺少角色行：{missing}")
    author, reviewer, brain = (parse_model(rows[name][0])
                               for name in ("feature, refactoring", "judgment and prose", "hardest tasks"))
    runners = [parse_model(m) for m in rows["architect runners"][:2]]
    judges = [m for m in rows["arena cross-judge pool"] if MODEL_FAMILY.get(m) not in (None, MODEL_FAMILY[brain])]
    if MODEL_FAMILY[author] == MODEL_FAMILY[reviewer]:
        raise ValueError("pstack-models 的作者与独审是同一家族，不能构成异家族独审")
    if len(runners) < 2 or MODEL_FAMILY[runners[0]] == MODEL_FAMILY[runners[1]]:
        raise ValueError("architect runners 需要两个不同家族的模型")
    if not judges:
        raise ValueError("arena cross-judge pool 里没有与主控不同家族的模型")
    return RolePlan(author, reviewer, brain, (runners[0], runners[1]), parse_model(judges[0]))


@dataclass(frozen=True)
class RunSpec:
    run_id: RunId
    ticket: int
    scope: Scope | None
    base: Sha
    task: TaskSpec
    author_model: ModelSlug
    reviewer_model: ModelSlug
    brain_model: ModelSlug
    author_checkout: str
    review_checkout: str
    created_at: str
    design_models: tuple[ModelSlug, ModelSlug] | None
    judge_model: ModelSlug | None
    v: int = 1
    work_unit: dict | None = None

    def to_json(self) -> dict:
        data = asdict(self)
        if self.v == 1:
            data.pop("v")
            data.pop("work_unit")
        return data

    @property
    def unit(self) -> WorkUnit:
        if self.work_unit is None:
            raise ToolRefusal("v1 run 没有 WorkUnit")
        return parse_work_unit(self.work_unit)

    @staticmethod
    def from_json(data: dict) -> "RunSpec":
        raw_task = data["task"]
        task = TaskSpec(raw_task["title"], raw_task["requirement"], tuple(raw_task["allowed_prefixes"]),
                        bool(raw_task.get("require_design", False)))
        design = data.get("design_models")
        judge = data.get("judge_model")
        version = data.get("v", 1)
        if version not in (1, 2):
            raise ValueError(f"未知 RunSpec 版本：{version!r}")
        unit = data.get("work_unit") if version == 2 else None
        if version == 2:
            parsed = parse_work_unit(unit or {})
            if (parsed.run_id != data["run_id"] or parsed.ticket != data["ticket"] or parsed.base != data["base"]
                    or data.get("scope") is not None or parsed.allowed_prefixes != task.allowed_prefixes):
                raise ValueError("v2 RunSpec 与 WorkUnit 不一致")
        elif data["ticket"] != TICKET:
            raise ValueError("v1 RunSpec 只属于 #1742")
        return RunSpec(RunId(data["run_id"]), data["ticket"], data["scope"], parse_sha(data["base"]), task,
                       parse_model(data["author_model"]), parse_model(data["reviewer_model"]),
                       parse_model(data["brain_model"]), data["author_checkout"], data["review_checkout"],
                       data["created_at"], (parse_model(design[0]), parse_model(design[1])) if design else None,
                       parse_model(judge) if judge else None, version, unit)

    def model_for(self, role: Role) -> ModelSlug:
        if role == "author":
            return self.author_model
        if role == "review":
            return self.reviewer_model
        if self.design_models is None or self.judge_model is None:
            raise ToolRefusal("本 run 创建时没有冻结设计通道的模型，不能派设计或裁判工蜂")
        return self.judge_model if role == "judge" else self.design_models[DESIGN_SLOTS.index(role)]

    def checkout_for(self, role: Role) -> Path:
        author = Path(self.author_checkout)
        return {"author": author, "review": Path(self.review_checkout)}.get(role, author.parent / role)

    def branch_for(self, role: Role) -> str:
        if self.v == 2:
            return f"codex/deepagents-run/{self.run_id}-{role}"
        return f"codex/deepagents-pilot-1742-{self.run_id}-{role}"

    @property
    def trailer_key(self) -> str:
        return V2_TRAILER_KEY if self.v == 2 else TRAILER_KEY


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


def parse_result(key: WorkerKey, data: dict, output_dir: Path, ticket: int = TICKET) -> WorkerReceipt:
    if data.get("ticket") != ticket or data.get("status") not in ("completed", "failed", "cancelled"):
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


def lock_run_dir(run_dir: Path) -> int:
    """run 锁的唯一入口：控制进程与主控对账都经这里。锁文件不存在就创建，缺失不算无锁。返回持锁 fd。"""
    run_dir.mkdir(parents=True, exist_ok=True)
    fd = os.open(run_dir / "owner.lock", os.O_RDWR | os.O_CREAT, 0o600)
    try:
        fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        os.close(fd)
        raise Busy(f"run {run_dir.name} 已有控制进程持锁，本进程未做任何副作用") from None
    os.ftruncate(fd, 0)
    os.write(fd, f"{os.getpid()}\n".encode())
    return fd


class PilotRun:
    """一个 run 目录的唯一写者；进程内工具调用经 mutex 串行，进程间经 owner.lock。"""

    def __init__(self, home: Path, spec: RunSpec, *, launcher: list[str] | None = None, crash_at: str | None = None,
                 source_repo: Path = SOURCE_REPO, executor: TicketExecutor | None = None):
        self.home = home
        self.executor = executor
        self.source_repo = source_repo
        self.spec = spec
        self.dir = home / "runs" / spec.run_id
        self.launcher = launcher or [shutil.which("node") or "node", "--import", "tsx", ".sandcastle/codex-child.mts"]
        self.crash_at = crash_at
        self._mutex = threading.Lock()
        self._tools = threading.Condition()
        self._tools_active = 0
        self._lock_fd: int | None = None
        self._children: dict[str, subprocess.Popen] = {}

    @staticmethod
    def run_dir(home: Path, run_id: RunId) -> Path:
        return home / "runs" / run_id

    @classmethod
    def create(cls, home: Path, run_id: RunId, task: TaskSpec, base: Sha, models_text: str, **kw) -> "PilotRun":
        plan = plan_roles(models_text)
        root = cls.run_dir(home, run_id)
        root.mkdir(parents=True, exist_ok=True)
        checkouts = home / "checkouts" / run_id
        spec = RunSpec(run_id, TICKET, "pilot_acceptance", base, task, plan.author, plan.reviewer, plan.brain,
                       str(checkouts / "author"), str(checkouts / "review"), _now(), plan.design, plan.judge)
        run = cls(home, spec, **kw)
        run.acquire()
        if not (root / "run.json").exists():
            _write_once(root / "run.json", spec.to_json())
        run.spec = RunSpec.from_json(json.loads((root / "run.json").read_text()))
        run.trail("run_created", spec=run.spec.to_json())
        run.ensure_checkouts()
        return run

    @classmethod
    def create_v2(cls, home: Path, unit: WorkUnit, task: TaskSpec, models_text: str, executor: TicketExecutor,
                  **kw) -> "PilotRun":
        """v2：先取 run 锁，再在票级锁内持有票，最后才冻结 run.json 与建 checkout。"""
        if task.allowed_prefixes != unit.allowed_prefixes or task.require_design != unit.require_design:
            raise ValueError("任务文件的写集或设计要求与 WorkUnit 不一致")
        plan = plan_roles(models_text)
        checkouts = home / "checkouts" / unit.run_id
        spec = RunSpec(unit.run_id, unit.ticket, None, unit.base, task, plan.author, plan.reviewer, plan.brain,
                       str(checkouts / "author"), str(checkouts / "review"), _now(), plan.design, plan.judge,
                       2, unit.to_json())
        run = cls(home, spec, executor=executor, **kw)
        run.acquire()
        try:
            run.bind_claim()
            if not (run.dir / "run.json").exists():
                _write_once(run.dir / "run.json", spec.to_json())
            run.spec = RunSpec.from_json(json.loads((run.dir / "run.json").read_text()))
            if run.spec.unit != unit:
                raise Halted("run.json 已冻结的 WorkUnit 与本次登记不同")
            run.trail("run_created", spec=run.spec.to_json(), generation=run.generation)
            run.ensure_checkouts()
        except BaseException:
            run.release()
            raise
        return run

    def bind_claim(self) -> None:
        """v2 run 的持有代次写进 run 目录；恢复时必须与票级记录一致。"""
        assert self.executor is not None
        claim = self.executor.claim(self.dir)
        path = self.dir / "claim.json"
        bound = {"ticket": claim.ticket, "run_id": claim.run_id, "generation": claim.generation}
        if path.exists() and json.loads(path.read_text()) != bound:
            raise Halted(f"run 目录记录的持有代次与票级记录不符：{path}")
        if not path.exists():
            _write_once(path, bound)
        self.trail("claim_bound", **bound)

    @property
    def generation(self) -> int:
        return int(json.loads((self.dir / "claim.json").read_text())["generation"])

    @contextmanager
    def _fence(self, effect: str) -> Iterator[None]:
        if self.executor is None:
            if self.spec.v == 2:
                raise Halted("v2 run 的副作用只能经新执行器的票级动作锁执行")
            yield
            return
        with self.executor.action(effect, self.dir, self.generation):
            yield

    @classmethod
    def open(cls, home: Path, run_id: RunId, **kw) -> "PilotRun":
        path = cls.run_dir(home, run_id) / "run.json"
        if not path.exists():
            raise FileNotFoundError(f"run {run_id} 不存在：{path}")
        return cls(home, RunSpec.from_json(json.loads(path.read_text())), **kw)

    def acquire(self) -> None:
        self._lock_fd = lock_run_dir(self.dir)

    def release(self) -> None:
        if self._lock_fd is not None:
            os.close(self._lock_fd)
            self._lock_fd = None

    @contextmanager
    def tool_call(self) -> Iterator[None]:
        with self._tools:
            self._tools_active += 1
        try:
            yield
        finally:
            with self._tools:
                self._tools_active -= 1
                self._tools.notify_all()

    def wait_tools_idle(self) -> None:
        """官方 run 被取消时任务先退出、同步工具仍在线程里跑；等它们返回才放 run 锁。"""
        with self._tools:
            self._tools.wait_for(lambda: self._tools_active == 0)

    def interrupt_requested(self) -> dict | None:
        path = self.dir / "interrupt.request"
        return json.loads(path.read_text()) if path.exists() else None

    def request_interrupt(self, by: str) -> dict:
        """操作员中断请求：只写一个文件，控制循环在下一次工具调用前停在 checkpoint。不需要锁，也不杀工蜂。"""
        request = self.interrupt_requested() or {"at": _now(), "by": by}
        if not (self.dir / "interrupt.request").exists():
            _write_once(self.dir / "interrupt.request", request)
            self.trail("operator_interrupt_requested", by=by)
        return request

    def clear_interrupt(self) -> None:
        """只由持有 run 锁的恢复入口调用。"""
        assert self._lock_fd is not None
        (self.dir / "interrupt.request").unlink(missing_ok=True)

    def trail(self, event: str, **fields) -> None:
        with open(self.dir / "trail.jsonl", "a") as fh:
            fh.write(json.dumps({"at": _now(), "pid": os.getpid(), "event": event, **fields}, ensure_ascii=False) + "\n")

    def _crash(self, point: str) -> None:
        if self.crash_at == point:
            self.trail("crash_injected", point=point)
            os.kill(os.getpid(), signal.SIGKILL)

    def ensure_checkouts(self) -> None:
        for role in ("author", "review"):
            self._ensure_checkout(role, self.spec.base)

    def _ensure_checkout(self, role: Role, head: Sha) -> None:
        path, branch = self.spec.checkout_for(role), self.spec.branch_for(role)
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
        names: set[str] = set()
        for sub in ("intents", "workers"):
            folder = self.dir / sub
            if folder.exists():
                names.update(p.name.removesuffix(".json") for p in folder.iterdir() if not p.name.endswith(".tmp"))
        return [parse_key_name(self.spec.run_id, name) for name in sorted(names)]

    def _observe(self, key: WorkerKey, rows: list[tuple[int, int, str, str]]) -> WorkerFact:
        out = self._key_dir(key)
        result = None
        state = None
        try:
            if (out / "result.json").exists():
                result = parse_result(key, json.loads((out / "result.json").read_text()), out, self.spec.ticket)
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
        return CommitIntent(parse_key_name(self.spec.run_id, data["author"]), parse_sha(data["parent"]),
                            data["tree"], tuple(data["paths"]), data.get("generation"))

    def _intent_json(self, intent: CommitIntent) -> dict:
        data: dict[str, Any] = {"author": intent.author.name, "parent": intent.parent, "tree": intent.tree,
                                "paths": list(intent.paths),
                                "trailer": f"{self.spec.trailer_key}: {intent.trailer}"}
        if intent.generation is not None:
            data["generation"] = intent.generation
        return data

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
        author = self.spec.checkout_for("author")
        observed = CheckoutState(_git(author, "branch", "--show-current"), parse_sha(_git(author, "rev-parse", "HEAD")))
        recorded = chain_head(self.spec.base, commits)
        intent_path = self.dir / "commits" / f"author-{recorded}.intent.json"
        pending_landed = (observed.head != recorded and intent_path.exists() and reconcile_commit(
            self._parse_intent(json.loads(intent_path.read_text())), self._observe_head(author)) == "committed")
        return RunFacts(self.spec.run_id, self.spec.base, self.spec.task.require_design, commits, workers, verdicts,
                        self.spec.branch_for("author"), observed, pending_landed)

    def run_design(self, brief: str) -> list[WorkerReceipt]:
        with self._mutex:
            head = current_head(self.facts())
            return self._dispatch([WorkerKey(self.spec.run_id, slot, head) for slot in DESIGN_SLOTS], brief)

    def run_worker(self, role: Role, brief: str) -> WorkerReceipt:
        with self._mutex:
            head = current_head(self.facts())
            return self._dispatch([WorkerKey(self.spec.run_id, role, head)], brief)[0]

    def _dispatch(self, keys: list[WorkerKey], brief: str) -> list[WorkerReceipt]:
        facts = self.facts()
        fresh = [key for key in keys if key not in facts.workers]
        for key in fresh:
            refusal = dispatch_refusal(facts, key) or synthesis_refusal(facts, key, brief)
            if refusal:
                self.trail("refused", key=key.name, reason=refusal)
                raise ToolRefusal(refusal)
        for key in keys:
            if key not in fresh:
                self.trail("adopt", key=key.name, fact=type(facts.workers[key]).__name__)
        for key in fresh:
            base = declared_base(brief) if key.role == "author" else None
            judge = facts.workers.get(WorkerKey(self.spec.run_id, "judge", key.target_head))
            if base is not None and isinstance(judge, Terminal):
                self.trail("synthesis", key=key.name, declared_base=base,
                           judge_first_line=(judge.receipt.summary.splitlines() or [""])[0])
            self._prepare_dispatch(key, brief, facts)
        return self._drive_all(keys)

    def _prepare_dispatch(self, key: WorkerKey, brief: str, facts: RunFacts) -> None:
        model = self.spec.model_for(key.role)
        checkout = self.spec.checkout_for(key.role)
        if key.role != "author":
            self._ensure_checkout(key.role, key.target_head)
            if _git(checkout, "rev-parse", "HEAD") != key.target_head:
                if _porcelain_paths(checkout):
                    raise Halted(f"{key.role} checkout 不干净，不能切到 {key.target_head}")
                _git(checkout, "switch", "-q", "-C", self.spec.branch_for(key.role), key.target_head)
        prompt = self.dir / "prompts" / f"{key.name}.md"
        prompt.parent.mkdir(parents=True, exist_ok=True)
        prompt.write_text(self._prompt(key, brief, facts))
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
        thread, manager = ((f"deepagents-{self.spec.run_id}", f"/deepagents/exec/{self.spec.run_id}")
                           if self.spec.v == 2 else
                           (f"pilot1742-{self.spec.run_id}", f"/deepagents/pilot1742/{self.spec.run_id}"))
        argv = [*self.launcher, "--ticket", str(self.spec.ticket), "--parent-thread-id", thread,
                "--manager-agent-id", manager, "--checkout", intent["checkout"],
                "--expected-head", key.target_head, "--mode", intent["mode"], "--model", intent["model"],
                "--idle-timeout-seconds", str(WORKER_IDLE_SECONDS), "--timeout-seconds", str(WORKER_TOTAL_SECONDS),
                "--prompt-file", intent["prompt_file"], "--output-dir", str(out)]
        log_dir = self.dir / "launch"
        log_dir.mkdir(exist_ok=True)
        env = {k: os.environ[k] for k in _ENV_KEEP if os.environ.get(k)}
        with self._fence("dispatch"):
            with open(log_dir / f"{key.name}.log", "ab") as log:
                child = subprocess.Popen(argv, cwd=self.source_repo, stdin=subprocess.DEVNULL, stdout=log,
                                         stderr=log, env=env, start_new_session=True)
            self._children[key.name] = child
            self._crash("after_spawn_before_launch_record")
            _write_once(log_dir / f"{key.name}.{child.pid}.json", {"pid": child.pid, "argv": argv, "at": _now()})
        self.trail("dispatched", key=key.name, supervisor_pid=child.pid, model=intent["model"])

    def _drive_all(self, keys: list[WorkerKey]) -> list[WorkerReceipt]:
        launched: set[str] = set()
        while True:
            rows = _ps_rows()
            done: dict[WorkerKey, WorkerReceipt] = {}
            for key in keys:
                fact = self._observe(key, rows)
                if isinstance(fact, Terminal):
                    done[key] = fact.receipt
                elif isinstance(fact, EffectUnknown):
                    self.trail("halted", key=key.name, reason=fact.reason)
                    raise Halted(f"{key.name}：{fact.reason}")
                elif isinstance(fact, NotStarted):
                    child = self._children.get(key.name)
                    if key.name in launched and child is not None and child.poll() is not None:
                        self.trail("halted", key=key.name, reason="监督进程在认领 outputDir 前退出")
                        raise Halted(f"{key.name}：codex-child 在认领 outputDir 前退出，见 launch/{key.name}.log")
                    if key.name not in launched:
                        self._launch(key)
                        launched.add(key.name)
            if len(done) == len(keys):
                for key in keys:
                    receipt = done[key]
                    self.trail("terminal", key=key.name, status=receipt.status, model=receipt.model,
                               session=receipt.external_session_id)
                return [done[key] for key in keys]
            for child in self._children.values():
                child.poll()
            time.sleep(2)

    @property
    def _label(self) -> str:
        return "交付" if self.spec.v == 2 else "试点"

    def _prompt(self, key: WorkerKey, brief: str, facts: RunFacts) -> str:
        task = self.spec.task
        scope = "、".join(task.allowed_prefixes)
        architect = SKILLS_ROOT + "architect"
        if key.role in DESIGN_SLOTS:
            return (f"# #{self.spec.ticket} 设计候选（architect Phase B 的 arena runner）\n\nrun {self.spec.run_id}，基线 {key.target_head}。"
                    "你在自己的独立只读 checkout 里工作，不写任何文件。\n\n"
                    f"先完整读 `{architect}/SKILL.md` 与 `{architect}/references/runner-prompt.md`：前者是你所在的流程，"
                    f"后者是本候选的合同。设计说明按 `{architect}/references/rationale-template.md` 组织，"
                    f"并用 `{architect}/references/design-red-flags.md` 自检。\n\n"
                    f"## 需求原文\n{task.requirement}\n\n## 写集\n{scope}\n\n"
                    f"## 主控给所有候选的同一设计任务\n{brief.strip()}\n\n## 交付\n"
                    "summary 写完整设计包：调用方用法、类型与函数签名草图、模块边界，以及按 rationale-template 写的 rationale"
                    "（含 Alternatives considered）。findings 逐条写你考虑过并拒绝的替代形状及理由。validation 写你实际核对过的事实。"
                    "不要向其他候选折中，status 填 completed 表示设计包已完成。\n")
        if key.role == "judge":
            packages = []
            for label, slot in zip(("候选 1", "候选 2"), DESIGN_SLOTS, strict=True):
                fact = facts.workers[WorkerKey(self.spec.run_id, slot, key.target_head)]
                assert isinstance(fact, Terminal)
                rejected = "\n".join(f"- {f}" for f in fact.receipt.findings)
                packages.append(f"## {label}\n{fact.receipt.summary}\n\n### {label}拒绝的替代形状\n{rejected}\n")
            return (f"# #{self.spec.ticket} arena 交叉裁判（只读）\n\nrun {self.spec.run_id}，基线 {key.target_head}。"
                    f"先完整读 `{SKILLS_ROOT}arena/SKILL.md`，你是其中 Phase C 的只读裁判，不写任何文件。"
                    "下面是同一任务的两个候选设计包，只按标签区分。按主控给的 rubric 逐条给每个候选打分，推荐一个 base 并写理由，"
                    "再指出各候选值得嫁接或应当拒绝的部分。\n\n"
                    f"## 需求原文\n{task.requirement}\n\n## rubric（主控给出）\n{brief.strip()}\n\n" + "\n".join(packages)
                    + "\n## 交付\nsummary 第一行写 `BASE: 候选 1` 或 `BASE: 候选 2`，之后写逐条评分与理由。"
                    "findings 逐条写建议嫁接或拒绝的点。status 填 completed 表示裁判已完成。\n")
        if key.role == "author":
            rework = ""
            verdict = facts.verdicts.get(key.target_head)
            review = facts.workers.get(WorkerKey(self.spec.run_id, "review", key.target_head))
            if verdict is not None and verdict.kind == "BLOCK" and isinstance(review, Terminal):
                findings = "\n".join(f"- {f}" for f in review.receipt.findings)
                rework = (f"\n## 返修依据\n候选 {key.target_head} 被独审（{verdict.reviewer_model}）判 BLOCK。\n"
                          f"{review.receipt.summary}\n{findings}\n")
            return (f"# #{self.spec.ticket} {self._label}作者任务：{task.title}\n\nrun {self.spec.run_id}，基线 {key.target_head}。\n\n"
                    f"## 需求原文\n{task.requirement}\n\n## 写集\n只允许改动 {scope} 下的文件；"
                    f"写集外的改动会让宿主拒绝提交。不要 commit，宿主负责受管提交。\n{rework}\n"
                    f"## 主控 brief\n{brief.strip()}\n")
        author_commit = next(c for c in facts.commits if c.commit == key.target_head)
        return (f"# #{self.spec.ticket} {self._label}独立审阅：{task.title}\n\nrun {self.spec.run_id}。审阅候选提交 {key.target_head}"
                f"（父提交 {author_commit.intent.parent}）。用 `git show {key.target_head}` 与 "
                f"`git diff {author_commit.intent.parent} {key.target_head}` 读改动，可以运行只读检查，不得写 checkout。\n\n"
                f"## 需求原文\n{task.requirement}\n\n## 写集\n{scope}\n\n## 结论协议\n"
                "summary 第一行必须严格是 `VERDICT: PASS` 或 `VERDICT: BLOCK`，第二行起写理由。"
                "BLOCK 时 findings 逐条写必须修复的缺陷。status 填 completed 表示审阅已完成，与结论无关。\n\n"
                f"## 主控关注点（不能指定结论）\n{brief.strip()}\n")

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
            checkout = self.spec.checkout_for("author")
            intent = self._parse_intent(json.loads(intent_path.read_text())) if intent_path.exists() else None
            if intent is not None:
                state = reconcile_commit(intent, self._observe_head(checkout))
                if state == "committed":
                    return self._receipt(intent, checkout, receipt_path, recovered=True)
                if state == "conflict":
                    self.trail("halted", tool="commit_candidate", reason="HEAD 已变但与提交意图不符")
                    raise Halted("作者 checkout 的 HEAD 已变化，但 parent/tree/trailer 与提交意图不符")
            if _git(checkout, "branch", "--show-current") != self.spec.branch_for("author"):
                raise Halted("作者 checkout 不在本 run 的作者分支")
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
                intent = CommitIntent(akey, head, tree, paths, self.generation if self.spec.v == 2 else None)
                _write_once(intent_path, self._intent_json(intent))
                self.trail("commit_intent", author=akey.name, parent=head, tree=tree)
            message = (f"#{self.spec.ticket} {self._label}候选：{self.spec.task.title}\n\n作者工蜂 {fact.receipt.model}，"
                       f"run {self.spec.run_id}，基线 {head}。")
            trailer = f"{self.spec.trailer_key}: {intent.trailer}"
            with self._fence("commit"):
                if self.spec.v == 2:
                    _git(checkout, "-c", "commit.gpgsign=false", "commit", "-q", "-m", message, "--trailer", trailer,
                         env={**os.environ, "TZ": "UTC"})
                else:
                    _git(checkout, "commit", "-q", "-m", message, "--trailer", trailer)
            self._crash("after_commit_before_receipt")
            if reconcile_commit(intent, self._observe_head(checkout)) != "committed":
                raise Halted("提交后观测不到与意图一致的提交")
            return self._receipt(intent, checkout, receipt_path, recovered=False)

    def _observe_head(self, checkout: Path) -> HeadObservation:
        head = parse_sha(_git(checkout, "rev-parse", "HEAD"))
        parents = _git(checkout, "rev-list", "--parents", "-n", "1", head).split()
        trailers = _git(checkout, "log", "-1", "--format=%(trailers:key=" + self.spec.trailer_key + ",valueonly)", head)
        return HeadObservation(head, parse_sha(parents[1]) if len(parents) > 1 else None,
                               _git(checkout, "rev-parse", f"{head}^{{tree}}"),
                               tuple(t.strip() for t in trailers.splitlines() if t.strip()))

    def _receipt(self, intent: CommitIntent, checkout: Path, path: Path, *, recovered: bool) -> CommitReceipt:
        receipt = CommitReceipt(intent, parse_sha(_git(checkout, "rev-parse", "HEAD")), recovered)
        _write_once(path, {"intent": self._intent_json(intent), "commit": receipt.commit, "recovered": recovered,
                           "at": _now()})
        self.trail("commit_receipt", commit=receipt.commit, parent=intent.parent, recovered=recovered)
        return receipt

    def publish_candidate(self) -> dict[str, Any]:
        if self.executor is None or self.spec.v != 2:
            raise ToolRefusal("只有 v2 run 经新执行器发布")
        with self._mutex:
            return self.executor.publish(self)

    def evaluate_gate(self, facts: RunFacts | None = None) -> tuple[GateDecision, list[str]]:
        if self.spec.v != 1 or self.spec.scope is None:
            raise ToolRefusal("v2 run 不消费 v1 批准账本")
        facts = facts or self.facts()
        ledger = self.dir / "approvals.jsonl"
        records, problems = parse_approval_ledger(ledger.read_text() if ledger.exists() else "")
        head = current_head(facts)
        return decide_gate(ticket=TICKET, scope=self.spec.scope, candidate=head, conflict=candidate_conflict(facts),
                           verdict=facts.verdicts.get(head), records=records), problems

    def gate(self) -> GateDecision:
        with self._mutex:
            decision, problems = self.evaluate_gate()
            self.trail("gate", decision=gate_json(decision), ledger_problems=problems)
            return decision

    def status(self) -> dict:
        facts = self.facts()
        head = current_head(facts)
        phase = derive_phase(facts)
        v1 = self.spec.v == 1
        gate, problems = self.evaluate_gate(facts) if v1 and phase in ("gate", "conflict") else (None, [])
        conflict = candidate_conflict(facts)
        extra: dict[str, Any] = {}
        if not v1:
            from .approval import pending_view
            from .executor import delivery_status

            extra = {"v": 2, "work_unit": self.spec.work_unit, "generation": self.generation,
                     "delivery": delivery_status(self.dir), "approvals": pending_view(self.dir / "approvals"),
                     "interrupt_requested": self.interrupt_requested()}
            if phase == "gate" and extra["delivery"]["phase"] is not None:
                phase = extra["delivery"]["phase"]
        return {
            "run_id": self.spec.run_id, "ticket": self.spec.ticket, "scope": self.spec.scope, "base": self.spec.base,
            "recorded_head": head, "observed_head": facts.observed.head, "observed_branch": facts.observed.branch,
            "conflict": conflict.code if conflict else None, "phase": phase,
            "require_design": self.spec.task.require_design,
            "models": {"brain": self.spec.brain_model, "author": self.spec.author_model,
                       "reviewer": self.spec.reviewer_model, "design": self.spec.design_models,
                       "judge": self.spec.judge_model},
            "workers": {k.name: _fact_json(f) for k, f in facts.workers.items()},
            "commits": [{"commit": c.commit, "parent": c.intent.parent, "tree": c.intent.tree,
                         "recovered": c.recovered} for c in facts.commits],
            "verdicts": {h: v.kind for h, v in facts.verdicts.items()},
            "gate": gate_json(gate) if gate is not None else None, "ledger_problems": problems, **extra,
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
