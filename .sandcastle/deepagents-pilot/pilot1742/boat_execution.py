from __future__ import annotations

import base64
import json
import os
import re
import shlex
import shutil
import stat
import subprocess
import tarfile
import time
from dataclasses import asdict, dataclass, replace
from datetime import datetime
from pathlib import Path
from typing import BinaryIO, Literal, Protocol, cast
from urllib.error import HTTPError
from urllib.parse import urlparse
from urllib.request import Request, urlopen
from uuid import uuid4

from .approval import spec_section
from .cloud_package import (
    MAX_FILE_BYTES,
    build_controller_manuals,
    build_package,
    build_project_context,
    build_skill_bundle,
    git,
    safe_path,
)
from .domain import EffectUnknown, NotStarted, Running, Terminal, WorkerFact, WorkerKey

ADMISSION_SECONDS = 900
CONTROL_EXEC_SECONDS = 30
# 600 是 boat exec 不切换为流式输出的最大有界 timeout。
UNPACK_EXEC_SECONDS = 600
STOP_MARGIN_SECONDS = 120
STOPPED_STATES = {"stopping", "stopped", "archiving", "archived"}
Submission = Literal["creating", "prepared", "submitting", "submitted", "collected"]


class ExecutionUnknown(RuntimeError):
    pass


def save(path: Path, data: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temp = path.with_suffix(".tmp")
    with temp.open("w") as handle:
        handle.write(json.dumps(data, ensure_ascii=False, indent=2) + "\n")
        handle.flush()
        os.fsync(handle.fileno())
    os.replace(temp, path)


@dataclass(frozen=True)
class BoatConfig:
    endpoint: str
    image: str
    base_sandbox: str
    key_source_sandbox: str
    key_source_path: str
    pstack_source: str
    model_rules: str
    ttl: int = 1800
    max_standard_seconds: int = 500000

    @classmethod
    def parse(cls, raw: dict) -> "BoatConfig":
        if not isinstance(raw, dict):
            raise ValueError("Boat配置必须是对象")
        try:
            config = cls(**raw)
        except TypeError:
            raise ValueError("Boat配置字段缺失或未知") from None
        if any(not isinstance(getattr(config, name), str) for name in (
                "endpoint", "image", "base_sandbox", "key_source_sandbox", "key_source_path", "pstack_source", "model_rules")):
            raise ValueError("Boat配置路径和身份必须是字符串")
        if type(config.ttl) is not int or type(config.max_standard_seconds) is not int:
            raise ValueError("Boat资源限制必须是整数")
        url = urlparse(config.endpoint)
        if url.scheme != "https" or not url.hostname or url.username or url.password or url.query or url.fragment:
            raise ValueError("Boat模型入口必须是无凭据HTTPS URL")
        if not config.endpoint.endswith("/v1") or not re.fullmatch(r"sha256:[0-9a-f]{64}", config.image):
            raise ValueError("Boat需要完整/v1入口与冻结Docker image ID")
        if not re.fullmatch(r"bx_[a-z0-9]+", config.base_sandbox):
            raise ValueError("冻结镜像所在的基底sandbox无效")
        if not re.fullmatch(r"bx_[a-z0-9]+", config.key_source_sandbox):
            raise ValueError("模型能力来源sandbox无效")
        if not re.fullmatch(r"/[A-Za-z0-9_./-]+", config.key_source_path) or ".." in Path(config.key_source_path).parts:
            raise ValueError("模型能力来源路径无效")
        if not ADMISSION_SECONDS + STOP_MARGIN_SECONDS <= config.ttl <= 1800 or not 1 <= config.max_standard_seconds <= 500000:
            raise ValueError("首次Boat验证超出5美元/30分钟合同")
        return config


@dataclass(frozen=True)
class HistoricalLocal:
    backend: Literal["historical-local", "test-launcher"] = "historical-local"


@dataclass(frozen=True)
class BoatRun:
    config: BoatConfig
    runtime_head: str
    backend: Literal["boat"] = "boat"


RunExecution = HistoricalLocal | BoatRun


def parse_execution(raw: dict | None) -> RunExecution:
    if raw is None:
        return HistoricalLocal()
    if raw == {"backend": "test-launcher"}:
        return HistoricalLocal("test-launcher")
    if not isinstance(raw, dict) or raw.get("backend") != "boat" or set(raw) != {"backend", "config", "runtime_head"}:
        raise ValueError("未知run执行绑定")
    if not isinstance(raw["runtime_head"], str) or not re.fullmatch(r"[0-9a-f]{40}", raw["runtime_head"]):
        raise ValueError("运行时身份需要完整SHA")
    return BoatRun(BoatConfig.parse(raw["config"]), raw["runtime_head"])


def new_execution(*, source_repo: Path) -> BoatRun:
    source = os.environ.get("PILOT_BOAT_CONFIG")
    if not source:
        raise ValueError("新run默认Boat，需PILOT_BOAT_CONFIG；不会回退本机")
    config = BoatConfig.parse(json.loads(Path(source).read_text()))
    return BoatRun(config, git(source_repo, "rev-parse", "HEAD").decode().strip())


@dataclass(frozen=True)
class ForkIntent:
    source_sandbox: str
    idempotency_key: str
    body: str
    requested_at: float


class BoatForkAPI:
    def fork(self, intent: ForkIntent) -> str:
        token = os.environ.get("BOAT_API_KEY")
        if not token:
            source = os.environ.get("PILOT_BOAT_API_KEY_FILE")
            if not source:
                raise ValueError("Boat fork需要BOAT_API_KEY或PILOT_BOAT_API_KEY_FILE，仅供控制端使用")
            path = Path(source)
            if path.resolve().is_relative_to(Path(__file__).resolve().parents[3]):
                raise ValueError("Boat控制端key文件必须位于运行时源码仓库外")
            try:
                descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
                with os.fdopen(descriptor) as stream:
                    info = os.fstat(stream.fileno())
                    if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid() or info.st_mode & 0o077:
                        raise ValueError("Boat控制端key须为当前用户持有且无group/other权限的普通文件")
                    token = stream.read().strip()
            except OSError:
                raise ValueError("无法读取明确指定的Boat控制端key文件") from None
        request = Request("https://boat.dev/api/v1/sandboxes/" + intent.source_sandbox + "/fork",
                          data=intent.body.encode(), method="POST",
                          headers={"Authorization": "Bearer " + token, "Content-Type": "application/json",
                                   "Idempotency-Key": intent.idempotency_key})
        attempt = 0
        while True:
            try:
                with urlopen(request, timeout=60) as response:
                    result = json.load(response)
                sandbox = result.get("id") if isinstance(result, dict) else None
                if not isinstance(sandbox, str) or not re.fullmatch(r"bx_[a-z0-9]+", sandbox):
                    raise ExecutionUnknown("Boat fork未返回有效sandbox身份，按原intent恢复")
                return sandbox
            except HTTPError as error:
                try:
                    payload = json.load(error)
                    code = payload.get("code") or payload.get("error", {}).get("code")
                except (ValueError, AttributeError):
                    code = "invalid_error"
                if error.code == 409 and code == "idempotency_in_progress" and attempt < 2:
                    attempt += 1
                    time.sleep(2)
                    continue
                raise ExecutionUnknown(f"Boat fork HTTP {error.code} ({code})；保留原intent") from None
            except (OSError, ValueError):
                raise ExecutionUnknown("Boat fork响应丢失或无效，须用原intent恢复") from None


@dataclass(frozen=True)
class BoatBinding:
    """One worker key owns one VM. Only the run's owner process writes this record."""
    identity: dict
    sandbox_name: str
    package_id: str
    remote_root: str
    submission: Submission
    sandbox_id: str | None = None
    deadline: float | None = None
    command_id: str | None = None
    process_id: int | None = None
    stop_id: str | None = None
    key_erased: bool = False
    cancel_requested: bool = False
    create_requested: bool = False
    fork_intent: ForkIntent | None = None
    stop_confirmed: bool = False  # 旧 stop_id 可能只是异步受理，不能据此放行。

    @property
    def request(self) -> str:
        return self.remote_root + "/request.json"

    @property
    def key_file(self) -> str:
        return str(Path(self.remote_root).parent / "private/model-key")


class WorkerExecution(Protocol):
    def submit(self, key: WorkerKey) -> None: ...
    def observe(self, key: WorkerKey, *, reconcile: bool = True) -> WorkerFact: ...
    def cancel(self, key: WorkerKey) -> WorkerFact: ...
    def collect(self, key: WorkerKey, receipt: dict) -> None: ...


class BoatCLI:
    def __init__(self, executable: str = "boat"):
        self.executable = executable

    def call(self, *args: str) -> list[dict]:
        result = subprocess.run([self.executable, "--json", *args], capture_output=True, text=True)
        if args[0] == "scp":
            if result.returncode:
                raise ExecutionUnknown("Boat artifact传输失败，未推断副作用")
            return []
        rows = []
        for line in result.stdout.splitlines():
            if line.strip():
                try:
                    rows.append(json.loads(line))
                except ValueError:
                    raise ExecutionUnknown("Boat输出不是JSON，原动作可能已发生") from None
        if result.returncode or not rows:
            raise ExecutionUnknown(f"Boat {args[0]}返回{result.returncode}，须按原绑定对账")
        return rows

    def exec(self, sandbox: str, command: str, *, timeout: int = CONTROL_EXEC_SECONDS) -> dict:
        row = self.call("exec", sandbox, "--timeout", str(timeout), "--", "bash", "-lc", command)[-1]
        if row.get("stdoutTruncated") or row.get("stderrTruncated") or row.get("exitCode") != 0 or row.get("timedOut") or row.get("signal"):
            raise ExecutionUnknown("远端命令输出不完整或执行非成功")
        output = row.get("stdout", "")
        if row.get("stdoutEncoding") == "base64":
            output = base64.b64decode(output).decode()
        try:
            return json.loads(output)
        except (TypeError, ValueError):
            raise ExecutionUnknown("远端监督器未交有效JSON") from None

    def info(self, sandbox: str) -> dict:
        return self.call("info", sandbox)[-1]["sandbox"]


def _epoch(stamp: str) -> float:
    return datetime.fromisoformat(stamp.replace("Z", "+00:00")).timestamp()


class BoatExecution(WorkerExecution):
    def __init__(self, run, cli: BoatCLI | None = None, fork_api: BoatForkAPI | None = None):
        self.run = run
        if not isinstance(run.spec.execution, BoatRun):
            raise TypeError("Boat执行器需要BoatRun绑定")
        self.config = run.spec.execution.config
        self.cli = cli or BoatCLI()
        self.fork_api = fork_api or BoatForkAPI()

    @property
    def generation(self) -> int:
        return self.run.generation if self.run.spec.v == 2 else 0

    def _identity(self, key: WorkerKey) -> dict:
        return {"run_id": key.run_id, "role": key.role, "target_head": key.target_head, "generation": self.generation}

    def _path(self, key: WorkerKey, suffix: str = ".json") -> Path:
        return self.run.dir / "boat" / (key.name + suffix)

    def load(self, key: WorkerKey) -> BoatBinding | None:
        path = self._path(key)
        if not path.exists():
            return None
        data = json.loads(path.read_text())
        if data.get("fork_intent") is not None:
            data["fork_intent"] = ForkIntent(**data["fork_intent"])
        binding = BoatBinding(**data)
        if binding.identity != self._identity(key):
            raise ExecutionUnknown("Boat绑定的持有代次或派工身份漂移")
        return binding

    def _store(self, key: WorkerKey, binding: BoatBinding) -> BoatBinding:
        save(self._path(key), asdict(binding))
        return binding

    def _command(self, binding: BoatBinding, action: str) -> str:
        return " ".join(shlex.quote(part) for part in
                        ("python3", binding.remote_root + "/controller/boat_remote.py", action, binding.request))

    def describe(self) -> dict:
        bindings = {}
        for path in sorted((self.run.dir / "boat").glob("*.json")):
            data = json.loads(path.read_text())
            if "sandbox_name" in data:
                bindings[path.stem] = {k: v for k, v in data.items() if k != "identity"}
        return bindings

    def submit(self, key: WorkerKey) -> None:
        from .run import require_free_space

        binding = self.load(key)
        if binding is not None and binding.submission in ("submitting", "submitted", "collected"):
            return
        require_free_space(self.run.dir)
        binding = binding or self._store(key, self._intent(key))
        if binding.stop_id or binding.cancel_requested:
            raise ExecutionUnknown("已取消的Boat绑定不得重新派工")
        if binding.sandbox_id is None:
            self._create_sandbox(key, binding)
            binding = cast(BoatBinding, self.load(key))
        if binding.submission == "creating":
            self._wait_ready(cast(str, binding.sandbox_id))
            self._upload(key, binding)
            binding = self._store(key, replace(binding, submission="prepared"))
        self._require_package_spec(binding)
        deadline = self._admit(cast(str, binding.sandbox_id))
        self._provision_key(binding)
        self._launch(key, self._store(key, replace(binding, submission="submitting", deadline=deadline)))

    def _intent(self, key: WorkerKey) -> BoatBinding:
        runtime = self.run.spec.execution.runtime_head
        package_id = f"{key.target_head}-g{self.generation}-{key.role}-{runtime[:12]}-{self._skill_bundle()[:12]}"
        name = f"newchanlun-{key.run_id}-{key.role}-{key.target_head[:12]}-g{self.generation}"
        return BoatBinding(self._identity(key), name[:120], package_id,
                           f"/home/user/newchanlun-{key.run_id}/{package_id}", "creating")

    def _skill_bundle(self) -> str:
        bundle = self.run.dir / "boat/skill-bundle"
        if not (bundle / "bundle.json").exists():
            if bundle.exists():
                shutil.rmtree(bundle)
            build_skill_bundle(Path(self.config.pstack_source), Path(self.config.model_rules), bundle)
        return json.loads((bundle / "bundle.json").read_text())["id"]

    def controller_context(self) -> Path:
        self._skill_bundle()
        context = self.run.dir / "boat/controller-context"
        if not (context / "context.json").exists():
            if context.exists():
                shutil.rmtree(context)
            build_project_context(self.run.source_repo, self.run.spec.base, context,
                                  Path(self.config.pstack_source), Path(self.config.model_rules))
        return context

    def controller_manuals(self) -> Path:
        """主控只读的冻结基线手册；与 controller-context 分开，旧 run 的已冻结上下文不改。"""
        manuals = self.run.dir / "controller-manuals"
        if not (manuals / "manuals.json").exists():
            if manuals.exists():
                shutil.rmtree(manuals)
            build_controller_manuals(self.run.source_repo, self.run.spec.base, manuals)
        return manuals

    def _controller_head(self) -> str:
        return git(self.run.source_repo, "rev-parse", "HEAD").decode().strip()

    def controller_upgrades(self) -> list[dict]:
        path = self.run.dir / "controller-upgrades.jsonl"
        return [json.loads(line) for line in path.read_text().splitlines() if line] if path.exists() else []

    def audit_controller(self) -> None:
        runtime, head = self.run.spec.execution.runtime_head, self._controller_head()
        upgrades = self.controller_upgrades()
        if head == (upgrades[-1]["controller_head"] if upgrades else runtime):
            return
        record = {"at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), "run_id": self.run.spec.run_id,
                  "generation": self.generation, "runtime_head": runtime, "controller_head": head, "pid": os.getpid()}
        with open(self.run.dir / "controller-upgrades.jsonl", "a") as stream:
            stream.write(json.dumps(record, ensure_ascii=False) + "\n")
            stream.flush()
            os.fsync(stream.fileno())
        self.run.trail("controller_upgrade", runtime_head=runtime, controller_head=head)

    def _create_sandbox(self, key: WorkerKey, binding: BoatBinding) -> str:
        if binding.fork_intent is None:
            if binding.create_requested:
                matches = [row for row in self.cli.call("list", "--all")[-1].get("sandboxes", [])
                           if row.get("name") == binding.sandbox_name]
                if len(matches) == 1:
                    self._store(key, replace(binding, sandbox_id=matches[0]["id"]))
                    return matches[0]["id"]
                raise ExecutionUnknown("旧fork缺少幂等intent，不能安全重建")
            limits = self.cli.call("limits")[-1]
            if not limits.get("canStart") or limits.get("creditUsedSeconds", 0) + self.config.ttl > self.config.max_standard_seconds:
                raise ExecutionUnknown("Boat额度或首次5美元预算不足")
            new_intent = ForkIntent(self.config.base_sandbox, str(uuid4()), json.dumps({
                "noEnv": True, "type": "small", "ttlSeconds": self.config.ttl,
                "labels": {"newchanlun.worker": binding.sandbox_name}}, sort_keys=True), time.time())
            binding = self._store(key, replace(binding, fork_intent=new_intent, create_requested=True))
        intent = cast(ForkIntent, binding.fork_intent)
        if time.time() >= intent.requested_at + 24 * 3600:
            raise ExecutionUnknown("Boat fork幂等保留期已过，不再请求创建；保留原intent对账")
        sandbox = self.fork_api.fork(intent)
        self.run._crash("after_boat_fork_before_record")
        self._store(key, replace(binding, sandbox_id=sandbox))
        self.cli.call("rename", sandbox, binding.sandbox_name)
        return sandbox

    def _wait_ready(self, sandbox: str) -> None:
        for _ in range(90):
            info = self.cli.info(sandbox)
            if info.get("state") in STOPPED_STATES | {"error", "cancelled"}:
                raise ExecutionUnknown("Boat VM未就绪，按原sandbox对账")
            if info.get("hydrated") is True and info.get("state") in {"ready", "idle", "running"}:
                return
            time.sleep(2)
        raise ExecutionUnknown("Boat VM等待hydrated超时，保留原sandbox")

    def _build(self, key: WorkerKey, binding: BoatBinding) -> Path:
        local = self.run.dir / "boat/packages" / binding.package_id
        if (local / ".complete.json").exists():
            if json.loads((local / ".complete.json").read_text()) != binding.identity:
                raise ExecutionUnknown("完整工蜂包身份不符")
            return local
        if local.exists():
            shutil.rmtree(local)
        staging = local.with_name(f".{local.name}.{uuid4()}.tmp")
        try:
            context = self.controller_context()
            manifest = build_package(self.run.source_repo, key.target_head, staging, self.run.dir / "boat/skill-bundle",
                                     Path(self.config.pstack_source), Path(self.config.model_rules), project_context=context)
            if manifest["project_skill_id"] != json.loads((context / "context.json").read_text())["id"]:
                raise ExecutionUnknown("工蜂与主控的冻结项目技能身份不同")
            runtime_head = self.run.spec.execution.runtime_head

            def show(path: str) -> bytes:
                return git(self.run.source_repo, "show", runtime_head + ":" + path)

            (staging / "controller").mkdir()
            (staging / "controller/boat_remote.py").write_bytes(show(".sandcastle/deepagents-pilot/pilot1742/boat_remote.py"))
            (staging / "runtime/.sandcastle").mkdir(parents=True)
            for name in ("package.json", "package-lock.json"):
                (staging / "runtime" / name).write_bytes(show(name))
            runtime_sources = {name: show(".sandcastle/" + name) for name in (
                "boat-worker.mts", "codex-child.ts", "docker-isolation.mts", "acceptance-dir.mts",
                "git-environment.ts", "main-model.mts")}
            if any(b"./scheduler-config.mts" in source for source in runtime_sources.values()):
                for name in ("scheduler-config.mts", "scheduler-config.json"):
                    runtime_sources[name] = show(".sandcastle/" + name)
            for name, source in runtime_sources.items():
                (staging / "runtime/.sandcastle" / name).write_bytes(source)
            intent = json.loads((self.run.dir / "intents" / f"{key.name}.json").read_text())
            prompt = (Path(intent["prompt_file"]).read_text().replace(self.config.pstack_source, "/opt/newchanlun/pstack")
                      .replace(str(self.run.source_repo), "/workspace/repo"))
            spec = self.run.signed_spec()
            if self.run.spec_issue is not None and spec is None:
                raise ExecutionUnknown(f"缺已签 SPEC #{self.run.spec_issue} 正文，不能封包派工")
            if spec is not None:
                prompt += "\n\n" + spec_section(spec)
            save(staging / "request.json", {
                "spec": spec.target if spec is not None else None, "controller_head": self._controller_head(),
                "identity": binding.identity, "package_id": binding.package_id, "root": binding.remote_root,
                "ticket": self.run.spec.ticket, "model": intent["model"], "mode": intent["mode"],
                **({"output_protocol": intent["output_protocol"]} if "output_protocol" in intent else {}),
                "endpoint": self.config.endpoint, "image": self.config.image,
                "allowed_prefixes": list(self.run.spec.task.allowed_prefixes),
                "prompt": prompt})
            save(staging / ".complete.json", binding.identity)
            os.rename(staging, local)
            return local
        finally:
            if staging.exists():
                shutil.rmtree(staging)

    def _upload(self, key: WorkerKey, binding: BoatBinding) -> None:
        local = self._build(key, binding)
        archive = local.with_suffix(".tar.gz")
        with tarfile.open(archive, "w:gz") as package:
            package.add(local, arcname=".")
        remote = shlex.quote(binding.remote_root)
        self.cli.exec(cast(str, binding.sandbox_id), f"set -e; test ! -e {remote}/claim; mkdir -p {remote}; printf '{{}}'")
        # `boat push` omits .git; the archive carries the exported credential-free Git metadata.
        self.cli.call("scp", str(archive), cast(str, binding.sandbox_id) + ":" + binding.remote_root + "/package.tar.gz")
        self.cli.exec(cast(str, binding.sandbox_id), f"set -e; test ! -e {remote}/claim; tar -xzf {remote}/package.tar.gz -C {remote}; "
                                          f"rm {remote}/package.tar.gz; printf '{{}}'", timeout=UNPACK_EXEC_SECONDS)

    def _admit(self, sandbox: str) -> float:
        info = self.cli.info(sandbox)
        for _ in range(30):
            if info.get("hydrated", True) or info.get("state") in STOPPED_STATES:
                break
            time.sleep(10)
            info = self.cli.info(sandbox)
        else:
            raise ExecutionUnknown("工蜂VM磁盘仍在恢复，暂不派工；可稍后重试")
        if info.get("state") in STOPPED_STATES or not info.get("archiveAfter"):
            raise ExecutionUnknown("工蜂VM未运行或没有自动停止期限；不续期、不派工")
        stop_at = _epoch(info["archiveAfter"])
        if stop_at - time.time() < ADMISSION_SECONDS + STOP_MARGIN_SECONDS:
            raise ExecutionUnknown("工蜂VM剩余寿命不足以完成一次工蜂；不自动续期，可显式恢复该VM后重试")
        return stop_at - STOP_MARGIN_SECONDS

    def _key_source(self) -> list[str]:
        return [self.cli.executable, "ssh", self.config.key_source_sandbox, "cat " + shlex.quote(self.config.key_source_path)]

    def _provision_key(self, binding: BoatBinding) -> None:
        source = subprocess.Popen(self._key_source(), stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        folder = shlex.quote(str(Path(binding.key_file).parent))
        destination = subprocess.run([self.cli.executable, "ssh", cast(str, binding.sandbox_id),
                                      f"umask 077; mkdir -p {folder}; cat > {shlex.quote(binding.key_file)}"],
                                     stdin=source.stdout, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        cast(BinaryIO, source.stdout).close()
        if source.wait() or destination.returncode:
            raise ExecutionUnknown("模型能力受管传输失败，未启动工蜂")

    def packaged_spec(self, binding: BoatBinding) -> str | None:
        """该绑定持久封包所附的 SPEC 签批目标；派工、收集与发布都以它判定工蜂产物属于哪份 SPEC。"""
        request = self.run.dir / "boat/packages" / binding.package_id / "request.json"
        return json.loads(request.read_text()).get("spec") if request.exists() else None

    def _package_spec_problem(self, binding: BoatBinding) -> str | None:
        if self.run.spec_issue is None:
            return None
        spec = self.run.signed_spec()
        packaged = self.packaged_spec(binding)
        if spec is None or packaged != spec.target:
            return (f"封包 SPEC {packaged} 与当前签批目标 {spec.target if spec else None} 不符；"
                    "保留原包、回执与绑定，不派工也不收集，经 cancel-worker 废弃后排空换代")
        return None

    def _require_package_spec(self, binding: BoatBinding) -> None:
        """封包所附的 SPEC 摘要必须就是当前签批目标；只在 dispatch 动作锁内调用，此时目标已在锁内重核。"""
        problem = self._package_spec_problem(binding)
        if problem is not None:
            raise ExecutionUnknown(problem)

    def _launch(self, key: WorkerKey, binding: BoatBinding) -> None:
        from .run import require_free_space

        require_free_space(self.run.dir)
        self._require_package_spec(binding)
        launch = self.cli.call("exec", cast(str, binding.sandbox_id), "--detach", "--", "bash", "-lc",
                               self._command(binding, "submit") + " " + str(int(cast(float, binding.deadline))))[-1]
        self.run._crash("after_boat_submit_before_record")
        self._store(key, replace(binding, submission="submitted", command_id=launch.get("commandId"),
                                 process_id=launch.get("processId")))

    def observe(self, key: WorkerKey, *, reconcile: bool = True) -> WorkerFact:
        binding = self.load(key)
        if binding is None:
            return NotStarted()
        if binding.submission in ("creating", "prepared"):
            if reconcile and binding.cancel_requested:
                return EffectUnknown("Boat工蜂已取消；不得恢复派工，待原绑定清理或换代")
            if binding.cancel_requested and not self.is_drained(key):
                return EffectUnknown("Boat准备阶段仍有远端资源或未确认的fork，尚未排空")
            return NotStarted()
        if binding.submission == "collected":
            return self._terminal(key)
        if binding.cancel_requested:
            return EffectUnknown("Boat工蜂已经取消废弃；回执保留，不收集、不重派，排空后换代")
        cache = self._path(key, ".remote.json")
        if not reconcile:
            return EffectUnknown("远端终态已取回，待控制端收集") if cache.exists() else Running()
        try:
            if not cache.exists():
                fact = self._fetch(key, binding)
                if fact is not None:
                    return fact
            self._reclaim(key, cast(BoatBinding, self.load(key)))
            with self.run._fence("dispatch"):
                self.collect(key, json.loads(cache.read_text()))
        except (ExecutionUnknown, ValueError, KeyError, TypeError, OSError) as error:
            return EffectUnknown(str(error))
        return self._terminal(key)

    def is_drained(self, key: WorkerKey) -> bool:
        """只读排空证据；失败回执不成为业务 Terminal，也不进入 collect。"""
        binding = self.load(key)
        if binding is None:
            return True
        if binding.submission in ("creating", "prepared"):
            return bool((binding.stop_id and binding.stop_confirmed is True and binding.key_erased)
                        or (binding.submission == "creating" and not binding.sandbox_id
                            and not binding.create_requested and binding.fork_intent is None))
        if (binding.submission not in ("submitting", "submitted") or binding.stop_confirmed is not True
                or not isinstance(binding.stop_id, str) or not binding.stop_id or not binding.key_erased):
            return False
        cache = self._path(key, ".remote.json")
        if not cache.exists():
            return False
        receipt = json.loads(cache.read_text())
        if not isinstance(receipt, dict):
            return False
        if receipt.get("identity") != binding.identity or receipt.get("package_id") != binding.package_id:
            raise ExecutionUnknown("排空回执身份或 package 不符")
        code = receipt.get("exit_code")
        if type(code) is not int:
            return False
        if "record" not in receipt:
            # boat_remote 的监督失败/取消回执沿用 unknown，非零退出码才证明监督已终止。
            return receipt.get("state") == "unknown" and code != 0
        record = receipt["record"]
        statuses = ("failed", "cancelled", "completed") if binding.cancel_requested else ("failed", "cancelled")
        if receipt.get("state") != "terminal" or not isinstance(record, dict) or record.get("status") not in statuses:
            return False
        process_exit = record.get("process_exit")
        return (isinstance(process_exit, dict) and type(process_exit.get("code")) is int
                and process_exit["code"] == code)

    def _terminal(self, key: WorkerKey) -> Terminal:
        out = self.run._key_dir(key)
        return Terminal(self.run._parse_worker_result(key, json.loads((out / "result.json").read_text())))

    def _fetch(self, key: WorkerKey, binding: BoatBinding, *, allow_launch: bool = True) -> WorkerFact | None:
        if cast(str, self.cli.info(cast(str, binding.sandbox_id)).get("state")) in STOPPED_STATES:
            raise ExecutionUnknown("工蜂VM已停止；须显式恢复同一VM后对账，不自动续费或重派")
        remote = self.cli.exec(cast(str, binding.sandbox_id), self._command(binding, "observe"))
        if remote.get("identity") != binding.identity or remote.get("package_id") != binding.package_id:
            raise ExecutionUnknown("Boat远端身份或package不符")
        state = remote.get("state")
        if state == "running":
            if binding.submission == "submitting":
                self._adopt(key, binding, self._submit_records(binding))
            return Running()
        if state == "unclaimed":
            return self._unclaimed(key, binding, allow_launch=allow_launch)
        if state != "receipt":
            raise ExecutionUnknown(remote.get("reason", "Boat工蜂没有可证终态"))
        part = self._path(key, ".remote.part")
        self.cli.call("scp", cast(str, binding.sandbox_id) + ":" + binding.remote_root + "/claim/receipt.json", str(part))
        if part.stat().st_size > 96 * 1024 * 1024:
            raise ExecutionUnknown("远端回执超过批次界限")
        receipt = json.loads(part.read_text())
        if receipt.get("identity") != binding.identity or receipt.get("package_id") != binding.package_id:
            raise ExecutionUnknown("远端回执身份或package不符")
        session = (receipt.get("record") or {}).get("external_session_log")
        if session:
            if session != binding.remote_root + "/claim/session.jsonl":
                raise ExecutionUnknown("远端session日志路径不符合绑定")
            target = self.run._key_dir(key) / "session.jsonl"
            target.parent.mkdir(parents=True, exist_ok=True)
            self.cli.call("scp", cast(str, binding.sandbox_id) + ":" + session, str(target))
            receipt["record"]["external_session_log"] = str(target)
        save(self._path(key, ".remote.json"), receipt)
        part.unlink()
        return None

    def _submit_records(self, binding: BoatBinding) -> list[dict]:
        rows = self.cli.call("history", cast(str, binding.sandbox_id), "--mode", "detached", "--limit", "100")[-1].get("commands", [])
        return [row for row in rows if self._command(binding, "submit") in (row.get("command") or "")]

    def _adopt(self, key: WorkerKey, binding: BoatBinding, records: list[dict]) -> None:
        running = [row for row in records if row.get("status") == "running"]
        if binding.submission == "submitting" and len(running) == 1:
            self._store(key, replace(binding, submission="submitted", command_id=running[0].get("commandId"),
                                     process_id=running[0].get("processId")))

    def _unclaimed(self, key: WorkerKey, binding: BoatBinding, *, allow_launch: bool = True) -> WorkerFact:
        mine = self._submit_records(binding)
        if any(row.get("status") == "running" for row in mine):
            self._adopt(key, binding, mine)
            return Running()
        if allow_launch and binding.submission == "submitting" and not mine:
            with self.run._fence("dispatch"):
                self._launch(key, binding)
            return Running()
        raise ExecutionUnknown("监督命令已结束但远端没有认领；模型任务未启动，需主控处置")

    def _reclaim(self, key: WorkerKey, binding: BoatBinding) -> None:
        if binding.stop_id and binding.stop_confirmed is True and binding.key_erased:
            return
        info = self.cli.info(cast(str, binding.sandbox_id))
        if info.get("state") in STOPPED_STATES:
            if info["state"] not in ("stopped", "archived"):
                raise ExecutionUnknown("Boat停止请求已受理但VM停止未确认，保留排空占位")
            if not binding.key_erased:
                raise ExecutionUnknown("Boat VM已停止但模型key删除未确认；须显式恢复同一VM后清理")
            stop = (info.get("stop") or {}).get("id") or info["state"]
        else:
            key_file = shlex.quote(binding.key_file)
            self.cli.exec(cast(str, binding.sandbox_id), f"rm -f {key_file} && test ! -e {key_file} && printf '{{}}'")
            binding = self._store(key, replace(binding, key_erased=True))
            stop = self.cli.call("stop", cast(str, binding.sandbox_id))[-1]["operation"]["id"]
            if self.cli.info(cast(str, binding.sandbox_id)).get("state") not in ("stopped", "archived"):
                raise ExecutionUnknown("Boat停止请求已受理但VM停止未确认，保留排空占位")
        self._store(key, replace(binding, stop_id=stop, stop_confirmed=True))

    def collect(self, key: WorkerKey, receipt: dict) -> None:
        binding = self.load(key)
        if not binding or receipt["identity"] != self._identity(key) or receipt["package_id"] != binding.package_id:
            raise ExecutionUnknown("拒绝收集其他身份产物")
        self._require_package_spec(binding)
        if receipt.get("exit_code") != 0 or not isinstance(receipt.get("record"), dict):
            raise ExecutionUnknown(receipt.get("reason") or "远端退出失败或未知，不能产生业务终态")
        record = receipt["record"]
        if (record.get("expected_head") != key.target_head or record.get("model") != self.run.spec.model_for(key.role)
                or (record.get("status") == "completed" and record.get("actual_model") != record.get("model"))):
            raise ExecutionUnknown("远端模型或head不符")
        result_path = self.run._key_dir(key) / "result.json"
        if binding.submission == "collected" and result_path.exists():
            if json.loads(result_path.read_text()) != record:
                raise ExecutionUnknown("已收集业务回执发生漂移")
            return
        checkout = self.run.spec.checkout_for(key.role)
        if git(checkout, "rev-parse", "HEAD").decode().strip() != key.target_head:
            raise ExecutionUnknown("收集目标head已漂移")
        from .run import _porcelain_paths
        parsed = self.run._parse_worker_result(key, record)
        files = receipt.get("files", [])
        if not isinstance(files, list):
            raise ExecutionUnknown("产物清单不是列表")
        if key.role != "author" and files:
            raise ExecutionUnknown("只读独审产生文件改动")
        artifact = self._decode(checkout, files)
        if set(parsed.changed_paths_after) != set(artifact):
            raise ExecutionUnknown("业务回执与产物清单不同")
        intent_path = self._path(key, ".collect.json")
        intent = {"identity": binding.identity, "files": files}
        recovering = intent_path.exists()
        dirty = set(_porcelain_paths(checkout))
        if recovering:
            if json.loads(intent_path.read_text()) != intent or dirty - set(artifact):
                raise ExecutionUnknown("恢复收集的清单或其他写者状态漂移")
        elif dirty:
            raise ExecutionUnknown("收集前checkout不干净")
        remove_directories, expanded = _removable_and_expanding_directories(checkout, key.target_head, artifact, recovering)
        for path, (data, mode) in artifact.items():
            destination = checkout / path
            if destination.is_dir() and path in remove_directories | expanded:
                continue
            if recovering and path in remove_directories and not destination.exists():
                continue
            if not _at_base(checkout, key.target_head, path) and not (recovering and _matches(destination, data, mode)):
                raise ExecutionUnknown("收集路径存在不属于本产物的并发内容或模式")
        save(intent_path, intent)
        if record["status"] == "completed":
            deletions = sorted((path for path, (data, _) in artifact.items() if data is None),
                               key=lambda path: (-len(Path(path).parts), path))
            for path in deletions:
                destination = checkout / path
                if (path in expanded and destination.is_dir()) or _matches(destination, None, "deleted"):
                    continue
                if not _at_base(checkout, key.target_head, path):
                    raise ExecutionUnknown("收集删除前路径发生并发修改")
                destination.unlink()
            for path in sorted(remove_directories, key=lambda path: (-len(Path(path).parts), path)):
                destination = checkout / path
                if destination.is_dir() and not destination.is_symlink():
                    destination.rmdir()
            for path, (data, mode) in artifact.items():
                if data is None:
                    continue
                destination = checkout / path
                destination.parent.mkdir(parents=True, exist_ok=True)
                temporary = self._path(key, ".collect-content.tmp")
                temporary.write_bytes(data)
                temporary.chmod(0o755 if mode == "100755" else 0o644)
                if not ((path in remove_directories and not destination.exists())
                        or _at_base(checkout, key.target_head, path)
                        or (recovering and _matches(destination, data, mode))):
                    raise ExecutionUnknown("收集写入前路径发生并发修改")
                os.replace(temporary, destination)
        save(result_path, record)
        self._store(key, replace(binding, submission="collected"))

    def _decode(self, checkout: Path, files: list) -> dict[str, tuple[bytes | None, str]]:
        artifact: dict[str, tuple[bytes | None, str]] = {}
        checkout_root = checkout.resolve()
        total = 0
        for entry in files:
            path = str(safe_path(entry["path"]))
            if path in artifact or not any(path.startswith(prefix) for prefix in self.run.spec.task.allowed_prefixes):
                raise ExecutionUnknown("远端产物重复或越出写集")
            if entry.get("mode") not in {"100644", "100755", "deleted"}:
                raise ExecutionUnknown("远端产物模式或链接不允许")
            data = base64.b64decode(entry.get("content", ""), validate=True)
            total += len(data)
            if len(data) > MAX_FILE_BYTES or total > 64 * 1024 * 1024:
                raise ExecutionUnknown("远端产物超过文件或批次界限")
            destination = checkout / path
            if destination.is_symlink() or destination.resolve() != checkout_root / path:
                raise ExecutionUnknown("本地收集路径与声明路径不一致或含链接")
            artifact[path] = (None if entry["mode"] == "deleted" else data, entry["mode"])
        return artifact

    def _cancel_prelaunch(self, key: WorkerKey, binding: BoatBinding) -> WorkerFact:
        if not binding.cancel_requested:
            binding = self._store(key, replace(binding, cancel_requested=True))
        if binding.stop_id and binding.stop_confirmed is True and binding.key_erased:
            return NotStarted()
        if binding.sandbox_id is None:
            if binding.submission == "creating" and not binding.create_requested and binding.fork_intent is None:
                return NotStarted()
            matches = [row for row in self.cli.call("list", "--all")[-1].get("sandboxes", [])
                       if row.get("name") == binding.sandbox_name
                       or (row.get("labels") or {}).get("newchanlun.worker") == binding.sandbox_name]
            if len(matches) != 1 or not re.fullmatch(r"bx_[a-z0-9]+", str(matches[0].get("id", ""))):
                raise ExecutionUnknown("取消未能唯一确认原Boat VM，保留fork intent；不创建新资源")
            binding = self._store(key, replace(binding, sandbox_id=matches[0]["id"]))
        self._reclaim(key, binding)
        return NotStarted()

    def cancel(self, key: WorkerKey) -> WorkerFact:
        binding = self.load(key)
        if binding is None:
            return NotStarted()
        with self.run._fence("cancel"):
            if binding.submission == "collected":
                return self._terminal(key)
            try:
                if binding.submission in ("creating", "prepared"):
                    return self._cancel_prelaunch(key, binding)
                if not self._path(key, ".remote.json").exists():
                    remote = self.cli.exec(cast(str, binding.sandbox_id), self._command(binding, "observe"))
                    if remote.get("identity") != binding.identity or remote.get("package_id") != binding.package_id:
                        raise ExecutionUnknown("取消前 Boat 远端身份或 package 不符")
                    self.cli.exec(cast(str, binding.sandbox_id), self._command(binding, "cancel"))
                    fact = self._fetch(key, binding, allow_launch=False)
                    if fact is not None:
                        return fact
                receipt = json.loads(self._path(key, ".remote.json").read_text())
                if receipt.get("identity") != binding.identity or receipt.get("package_id") != binding.package_id:
                    raise ExecutionUnknown("取消回收前 Boat 回执身份或 package 不符")
                self._reclaim(key, cast(BoatBinding, self.load(key)))
                binding = cast(BoatBinding, self.load(key))
                record = receipt.get("record")
                problem = self._package_spec_problem(binding)
                if (problem is not None and receipt.get("exit_code") == 0 and isinstance(record, dict)
                        and record.get("status") == "completed"):
                    self._store(key, replace(binding, cancel_requested=True))
                    self.run.trail("boat_receipt_abandoned", key=key.name, reason=problem)
                    return EffectUnknown("旧 SPEC 的完成回执已废弃：原回执保留，不收集、不记成功；排空后换代")
            except (ExecutionUnknown, ValueError, KeyError, TypeError, OSError) as error:
                return EffectUnknown(str(error))
            # 取消不准把已撤销准入的产物导入 checkout；回执保留给原控制入口对账。
            return EffectUnknown("远端终态已取回并停止 VM，取消入口不收集业务产物")


def _removable_and_expanding_directories(checkout: Path, head: str, artifact: dict[str, tuple[bytes | None, str]],
                            recovering: bool) -> tuple[set[str], set[str]]:
    writes = {path for path, (data, _) in artifact.items() if data is not None}
    if any(str(parent) in writes for path in writes for parent in Path(path).parents):
        raise ExecutionUnknown("产物文件不能同时作为其他产物文件的目录")
    remove_directories: set[str] = set()
    expanded = {path for path, (data, _) in artifact.items()
                if data is None and any(write.startswith(path + "/") for write in writes)}
    for path in writes:
        rows = git(checkout, "ls-tree", "-r", "-t", "-z", head, "--", path).decode().split("\0")
        entries = {name: metadata.split()[1] for row in rows if row
                   for metadata, name in [row.split("\t", 1)]
                   if name == path or name.startswith(path + "/")}
        if entries.get(path) != "tree":
            continue
        directories = {name for name, kind in entries.items() if kind == "tree"}
        children = set(entries) - directories
        if any(name not in artifact or artifact[name][0] is not None for name in children):
            raise ExecutionUnknown("目录改为文件必须在同一产物中删除全部跟踪子文件")
        remove_directories.update(directories)
        if (checkout / path).is_dir():
            _check_collection_tree(checkout, path, directories, children, artifact, completed_files=False)
    for path in expanded:
        if not (checkout / path).is_dir():
            continue
        if not recovering:
            raise ExecutionUnknown("文件改为目录前路径已有其他写者内容")
        children = {write for write in writes if write.startswith(path + "/")}
        directories = {str(parent) for write in children for parent in Path(write).parents
                       if str(parent) == path or str(parent).startswith(path + "/")}
        _check_collection_tree(checkout, path, directories, children, artifact, completed_files=True)
    return remove_directories, expanded


def _check_collection_tree(checkout: Path, root: str, directories: set[str], files: set[str],
                           artifact: dict[str, tuple[bytes | None, str]], *, completed_files: bool) -> None:
    pending = [checkout / root]
    while pending:
        for child in pending.pop().iterdir():
            name = str(child.relative_to(checkout))
            if child.is_symlink():
                raise ExecutionUnknown("收集目录包含未授权链接")
            if child.is_dir():
                if name not in directories:
                    raise ExecutionUnknown("收集目录包含未授权子目录")
                pending.append(child)
            elif name not in files or not child.is_file():
                raise ExecutionUnknown("收集目录包含未授权文件")
            elif completed_files and not _matches(child, *artifact[name]):
                raise ExecutionUnknown("恢复收集的目录包含并发内容或模式")


def _matches(path: Path, data: bytes | None, mode: str) -> bool:
    if path.is_symlink():
        return False
    if not path.is_file():
        return data is None and not path.exists()
    current_mode = "100755" if path.stat().st_mode & 0o111 else "100644"
    return data is not None and path.read_bytes() == data and current_mode == mode


def _at_base(checkout: Path, head: str, path: str) -> bool:
    tree = git(checkout, "ls-tree", head, "--", path).decode()
    if not tree:
        return _matches(checkout / path, None, "deleted")
    if tree.split()[0] == "040000":
        return ((checkout / path).is_dir() and not (checkout / path).is_symlink()
                and not git(checkout, "status", "--porcelain=v1", "-z", "--untracked-files=all", "--", path))
    return _matches(checkout / path, git(checkout, "show", head + ":" + path), tree.split()[0])
