from __future__ import annotations

import base64
import json
import os
import re
import shlex
import shutil
import subprocess
import tarfile
import time
from dataclasses import asdict, dataclass, replace
from datetime import datetime
from pathlib import Path
from typing import Literal, Protocol
from urllib.parse import urlparse

from .cloud_package import MAX_FILE_BYTES, build_package, build_skill_bundle, git, safe_path
from .domain import EffectUnknown, NotStarted, Running, Terminal, WorkerFact, WorkerKey

ADMISSION_SECONDS = 900
STOP_MARGIN_SECONDS = 120
STOPPED_STATES = {"stopping", "stopped", "archiving", "archived"}
Submission = Literal["creating", "prepared", "submitting", "submitted", "collected"]


class ExecutionUnknown(RuntimeError):
    pass


def save(path: Path, data: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temp = path.with_suffix(".tmp")
    temp.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n")
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
        config = cls(**raw)
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


def new_execution(*, test_launcher: bool = False, source_repo: Path | None = None) -> dict:
    if test_launcher:
        return {"backend": "test-launcher"}
    source = os.environ.get("PILOT_BOAT_CONFIG")
    if not source:
        raise ValueError("新run默认Boat，需PILOT_BOAT_CONFIG；不会回退本机")
    config = BoatConfig.parse(json.loads(Path(source).read_text()))
    return {"backend": "boat", "config": asdict(config),
            "runtime_head": git(source_repo, "rev-parse", "HEAD").decode().strip()}


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
    create_requested: bool = False

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

    def exec(self, sandbox: str, command: str) -> dict:
        row = self.call("exec", sandbox, "--timeout", "30", "--", "bash", "-lc", command)[-1]
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


class BoatExecution:
    def __init__(self, run, cli: BoatCLI | None = None):
        self.run = run
        self.config = BoatConfig.parse(run.spec.execution["config"])
        self.cli = cli or BoatCLI()

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
        binding = BoatBinding(**json.loads(path.read_text()))
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
        binding = self.load(key) or self._store(key, self._intent(key))
        if binding.submission in ("submitting", "submitted", "collected"):
            return
        if binding.sandbox_id is None:
            binding = self._store(key, replace(binding, sandbox_id=self._create_sandbox(key, binding)))
        if binding.submission == "creating":
            self._upload(key, binding)
            binding = self._store(key, replace(binding, submission="prepared"))
        deadline = self._admit(binding.sandbox_id)
        self._provision_key(binding)
        self._launch(key, self._store(key, replace(binding, submission="submitting", deadline=deadline)))

    def _intent(self, key: WorkerKey) -> BoatBinding:
        runtime = self.run.spec.execution["runtime_head"]
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

    def _create_sandbox(self, key: WorkerKey, binding: BoatBinding) -> str:
        """Fork the stopped base VM (its snapshot carries the frozen image) and name the fork after this key."""
        name = binding.sandbox_name
        matches = [row for row in self.cli.call("list", "--all")[-1].get("sandboxes", []) if row.get("name") == name]
        if len(matches) > 1:
            raise ExecutionUnknown("同名Boat VM不唯一，不能收养")
        if matches:
            return matches[0]["id"]
        if binding.create_requested:
            raise ExecutionUnknown("上次fork的响应丢失且没有同名VM可收养；不再创建，需主控按boat list处置")
        limits = self.cli.call("limits")[-1]
        if not limits.get("canStart") or limits.get("creditUsedSeconds", 0) + self.config.ttl > self.config.max_standard_seconds:
            raise ExecutionUnknown("Boat额度或首次5美元预算不足")
        self._store(key, replace(binding, create_requested=True))
        rows = self.cli.call("fork", self.config.base_sandbox, "--no-env", "--type", "small",
                             "--ttl", str(self.config.ttl), "--wait")
        ready = next((row for row in rows if row.get("event") == "ready" and row.get("id")), None)
        if ready is None:
            raise ExecutionUnknown("Boat fork未返回ready")
        self.cli.call("rename", ready["id"], name)
        return ready["id"]

    def _build(self, key: WorkerKey, binding: BoatBinding) -> Path:
        local = self.run.dir / "boat/packages" / binding.package_id
        if (local / "package.json").exists():
            return local
        if local.exists():
            shutil.rmtree(local)
        build_package(self.run.source_repo, key.target_head, local, self.run.dir / "boat/skill-bundle",
                      Path(self.config.pstack_source), Path(self.config.model_rules))
        runtime_head = self.run.spec.execution["runtime_head"]

        def show(path: str) -> bytes:
            return git(self.run.source_repo, "show", runtime_head + ":" + path)

        (local / "controller").mkdir()
        (local / "controller/boat_remote.py").write_bytes(show(".sandcastle/deepagents-pilot/pilot1742/boat_remote.py"))
        (local / "runtime/.sandcastle").mkdir(parents=True)
        for name in ("package.json", "package-lock.json"):
            (local / "runtime" / name).write_bytes(show(name))
        for name in ("boat-worker.mts", "codex-child.ts", "docker-isolation.mts", "acceptance-dir.mts",
                     "git-environment.ts", "main-model.mts"):
            (local / "runtime/.sandcastle" / name).write_bytes(show(".sandcastle/" + name))
        intent = json.loads((self.run.dir / "intents" / f"{key.name}.json").read_text())
        prompt = Path(intent["prompt_file"]).read_text().replace(self.config.pstack_source, "/opt/newchanlun/pstack")
        save(local / "request.json", {
            "identity": binding.identity, "package_id": binding.package_id, "root": binding.remote_root,
            "ticket": self.run.spec.ticket, "model": intent["model"], "mode": intent["mode"],
            "endpoint": self.config.endpoint, "image": self.config.image,
            "allowed_prefixes": list(self.run.spec.task.allowed_prefixes),
            "prompt": prompt.replace(str(self.run.source_repo), "/workspace/repo")})
        return local

    def _upload(self, key: WorkerKey, binding: BoatBinding) -> None:
        local = self._build(key, binding)
        archive = local.with_suffix(".tar.gz")
        with tarfile.open(archive, "w:gz") as package:
            package.add(local, arcname=".")
        remote = shlex.quote(binding.remote_root)
        self.cli.exec(binding.sandbox_id, f"set -e; test ! -e {remote}/claim; mkdir -p {remote}; printf '{{}}'")
        # `boat push` omits .git; the archive carries the exported credential-free Git metadata.
        self.cli.call("scp", str(archive), binding.sandbox_id + ":" + binding.remote_root + "/package.tar.gz")
        self.cli.exec(binding.sandbox_id, f"set -e; test ! -e {remote}/claim; tar -xzf {remote}/package.tar.gz -C {remote}; "
                                          f"rm {remote}/package.tar.gz; printf '{{}}'")

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
        destination = subprocess.run([self.cli.executable, "ssh", binding.sandbox_id,
                                      f"umask 077; mkdir -p {folder}; cat > {shlex.quote(binding.key_file)}"],
                                     stdin=source.stdout, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        source.stdout.close()
        if source.wait() or destination.returncode:
            raise ExecutionUnknown("模型能力受管传输失败，未启动工蜂")

    def _launch(self, key: WorkerKey, binding: BoatBinding) -> None:
        launch = self.cli.call("exec", binding.sandbox_id, "--detach", "--", "bash", "-lc",
                               self._command(binding, "submit") + " " + str(int(binding.deadline)))[-1]
        self.run._crash("after_boat_submit_before_record")
        self._store(key, replace(binding, submission="submitted", command_id=launch.get("commandId"),
                                 process_id=launch.get("processId")))

    def observe(self, key: WorkerKey, *, reconcile: bool = True) -> WorkerFact:
        binding = self.load(key)
        if binding is None or binding.submission in ("creating", "prepared"):
            return NotStarted()
        if binding.submission == "collected":
            return self._terminal(key)
        cache = self._path(key, ".remote.json")
        if not reconcile:
            return EffectUnknown("远端终态已取回，待控制端收集") if cache.exists() else Running()
        try:
            if not cache.exists():
                fact = self._fetch(key, binding)
                if fact is not None:
                    return fact
            self._reclaim(key, self.load(key))
            with self.run._fence("dispatch"):
                self.collect(key, json.loads(cache.read_text()))
        except (ExecutionUnknown, ValueError, KeyError, TypeError, OSError) as error:
            return EffectUnknown(str(error))
        return self._terminal(key)

    def _terminal(self, key: WorkerKey) -> Terminal:
        from .run import parse_result
        out = self.run._key_dir(key)
        return Terminal(parse_result(key, json.loads((out / "result.json").read_text()), out, self.run.spec.ticket))

    def _fetch(self, key: WorkerKey, binding: BoatBinding) -> WorkerFact | None:
        if self.cli.info(binding.sandbox_id).get("state") in STOPPED_STATES:
            raise ExecutionUnknown("工蜂VM已停止；须显式恢复同一VM后对账，不自动续费或重派")
        remote = self.cli.exec(binding.sandbox_id, self._command(binding, "observe"))
        if remote.get("identity") != binding.identity or remote.get("package_id") != binding.package_id:
            raise ExecutionUnknown("Boat远端身份或package不符")
        state = remote.get("state")
        if state == "running":
            if binding.submission == "submitting":
                self._adopt(key, binding, self._submit_records(binding))
            return Running()
        if state == "unclaimed":
            return self._unclaimed(key, binding)
        if state != "receipt":
            raise ExecutionUnknown(remote.get("reason", "Boat工蜂没有可证终态"))
        part = self._path(key, ".remote.part")
        self.cli.call("scp", binding.sandbox_id + ":" + binding.remote_root + "/claim/receipt.json", str(part))
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
            self.cli.call("scp", binding.sandbox_id + ":" + session, str(target))
            receipt["record"]["external_session_log"] = str(target)
        save(self._path(key, ".remote.json"), receipt)
        part.unlink()
        return None

    def _submit_records(self, binding: BoatBinding) -> list[dict]:
        rows = self.cli.call("history", binding.sandbox_id, "--mode", "detached", "--limit", "100")[-1].get("commands", [])
        return [row for row in rows if self._command(binding, "submit") in (row.get("command") or "")]

    def _adopt(self, key: WorkerKey, binding: BoatBinding, records: list[dict]) -> None:
        running = [row for row in records if row.get("status") == "running"]
        if binding.submission == "submitting" and len(running) == 1:
            self._store(key, replace(binding, submission="submitted", command_id=running[0].get("commandId"),
                                     process_id=running[0].get("processId")))

    def _unclaimed(self, key: WorkerKey, binding: BoatBinding) -> WorkerFact:
        mine = self._submit_records(binding)
        if any(row.get("status") == "running" for row in mine):
            self._adopt(key, binding, mine)
            return Running()
        if binding.submission == "submitting" and not mine:
            with self.run._fence("dispatch"):
                self._launch(key, binding)
            return Running()
        raise ExecutionUnknown("监督命令已结束但远端没有认领；模型任务未启动，需主控处置")

    def _reclaim(self, key: WorkerKey, binding: BoatBinding) -> None:
        if binding.stop_id:
            return
        info = self.cli.info(binding.sandbox_id)
        if info.get("state") in STOPPED_STATES:
            stop = (info.get("stop") or {}).get("id") or info["state"]
        else:
            key_file = shlex.quote(binding.key_file)
            self.cli.exec(binding.sandbox_id, f"rm -f {key_file}; test ! -e {key_file}; printf '{{}}'")
            stop = self.cli.call("stop", binding.sandbox_id)[-1]["operation"]["id"]
        self._store(key, replace(binding, stop_id=stop))

    def collect(self, key: WorkerKey, receipt: dict) -> None:
        binding = self.load(key)
        if not binding or receipt["identity"] != self._identity(key) or receipt["package_id"] != binding.package_id:
            raise ExecutionUnknown("拒绝收集其他身份产物")
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
        from .run import _porcelain_paths, parse_result
        parsed = parse_result(key, record, self.run._key_dir(key), self.run.spec.ticket)
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
        for path, (data, mode) in artifact.items():
            if not _at_base(checkout, key.target_head, path) and not (recovering and _matches(checkout / path, data, mode)):
                raise ExecutionUnknown("收集路径存在不属于本产物的并发内容或模式")
        save(intent_path, intent)
        if record["status"] == "completed":
            for path, (data, mode) in artifact.items():
                destination = checkout / path
                if data is None:
                    destination.unlink(missing_ok=True)
                    continue
                destination.parent.mkdir(parents=True, exist_ok=True)
                temporary = self._path(key, ".collect-content.tmp")
                temporary.write_bytes(data)
                temporary.chmod(0o755 if mode == "100755" else 0o644)
                os.replace(temporary, destination)
        save(result_path, record)
        self._store(key, replace(binding, submission="collected"))

    def _decode(self, checkout: Path, files: list) -> dict[str, tuple[bytes | None, str]]:
        artifact: dict[str, tuple[bytes | None, str]] = {}
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
            if destination.is_symlink() or not destination.resolve().is_relative_to(checkout.resolve()):
                raise ExecutionUnknown("本地收集路径含逃逸链接")
            artifact[path] = (None if entry["mode"] == "deleted" else data, entry["mode"])
        return artifact

    def cancel(self, key: WorkerKey) -> WorkerFact:
        binding = self.load(key)
        if binding is None or binding.submission in ("creating", "prepared"):
            return NotStarted()
        if binding.submission != "collected" and not self._path(key, ".remote.json").exists():
            with self.run._fence("dispatch"):
                self.cli.exec(binding.sandbox_id, self._command(binding, "cancel"))
        return self.observe(key)


def _matches(path: Path, data: bytes | None, mode: str) -> bool:
    if not path.is_file():
        return data is None
    current_mode = "100755" if path.stat().st_mode & 0o111 else "100644"
    return data is not None and path.read_bytes() == data and current_mode == mode


def _at_base(checkout: Path, head: str, path: str) -> bool:
    tree = git(checkout, "ls-tree", head, "--", path).decode()
    if not tree:
        return _matches(checkout / path, None, "deleted")
    return _matches(checkout / path, git(checkout, "show", head + ":" + path), tree.split()[0])
