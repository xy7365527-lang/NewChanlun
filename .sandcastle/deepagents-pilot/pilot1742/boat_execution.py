from __future__ import annotations

import base64
import json
import os
import re
import shlex
import shutil
import subprocess
import tarfile
from dataclasses import dataclass
from pathlib import Path
from typing import Protocol
from urllib.parse import urlparse

from .cloud_package import build_package, git, safe_path, MAX_FILE_BYTES
from .domain import EffectUnknown, NotStarted, Running, Terminal, WorkerFact, WorkerKey


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
    key_source_sandbox: str
    key_source_path: str
    pstack_source: str
    model_rules: str
    image_source_sandbox: str | None = None
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
        if config.image_source_sandbox is not None and not re.fullmatch(r"bx_[a-z0-9]+", config.image_source_sandbox):
            raise ValueError("冻结镜像来源sandbox无效")
        if not re.fullmatch(r"bx_[a-z0-9]+", config.key_source_sandbox):
            raise ValueError("模型能力来源sandbox无效")
        if not re.fullmatch(r"/[A-Za-z0-9_./-]+", config.key_source_path) or ".." in Path(config.key_source_path).parts:
            raise ValueError("模型能力来源路径无效")
        if not 1 <= config.ttl <= 1800 or not 1 <= config.max_standard_seconds <= 500000:
            raise ValueError("首次Boat验证超出5美元/30分钟合同")
        return config


def new_execution(*, test_launcher: bool = False, source_repo: Path | None = None) -> dict:
    if test_launcher:
        return {"backend": "test-launcher"}
    source = os.environ.get("PILOT_BOAT_CONFIG")
    if not source:
        raise ValueError("新run默认Boat，需PILOT_BOAT_CONFIG；不会回退本机")
    raw = json.loads(Path(source).read_text())
    config = BoatConfig.parse(raw)
    return {"backend": "boat", "config": config.__dict__,
            "runtime_head": git(source_repo, "rev-parse", "HEAD").decode().strip()}


class WorkerExecution(Protocol):
    def submit(self, key: WorkerKey) -> None: ...
    def observe(self, key: WorkerKey) -> WorkerFact: ...
    def cancel(self, key: WorkerKey) -> WorkerFact: ...
    def collect(self, key: WorkerKey, payload: dict) -> None: ...


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
        if result.returncode:
            raise ExecutionUnknown(f"Boat {args[0]}返回{result.returncode}，须按原绑定对账")
        return rows

    def exec(self, sandbox: str, command: str) -> dict:
        rows = self.call("exec", sandbox, "--timeout", "30", "--", "bash", "-lc", command)
        row = rows[-1]
        if row.get("stdoutTruncated") or row.get("stderrTruncated") or row.get("exitCode") != 0 or row.get("timedOut") or row.get("signal"):
            raise ExecutionUnknown("远端监督输出不完整或执行非成功")
        output = row.get("stdout", "")
        if row.get("stdoutEncoding") == "base64":
            output = base64.b64decode(output).decode()
        try:
            return json.loads(output)
        except (TypeError, ValueError):
            raise ExecutionUnknown("远端监督器未交有效JSON") from None


class BoatExecution:
    def __init__(self, run, cli: BoatCLI | None = None):
        self.run = run
        self.config = BoatConfig.parse(run.spec.execution["config"])
        self.cli = cli or BoatCLI()
        self.root = "/home/user/newchanlun-" + run.spec.run_id

    @property
    def generation(self) -> int:
        return self.run.generation if self.run.spec.v == 2 else 0

    def _path(self, key: WorkerKey) -> Path:
        return self.run.dir / "boat" / (key.name + ".json")

    def _identity(self, key: WorkerKey) -> dict:
        return {"run_id": key.run_id, "role": key.role, "target_head": key.target_head,
                "generation": self.generation}

    def _binding(self, key: WorkerKey) -> dict | None:
        path = self._path(key)
        if not path.exists():
            return None
        binding = json.loads(path.read_text())
        if binding["identity"] != self._identity(key):
            raise ExecutionUnknown("Boat绑定的持有代次或派工身份漂移")
        return binding

    def _sandbox(self) -> str:
        path = self.run.dir / "boat/sandbox.json"
        if path.exists():
            prior = json.loads(path.read_text())
            if prior.get("id"):
                info = self.cli.call("info", prior["id"])[-1]["sandbox"]
                if info.get("state") == "stopped":
                    raise ExecutionUnknown("Boat已停止；须显式恢复同sandbox后再观察，不自动续费或重派")
                return prior["id"]
            rows = self.cli.call("list")[-1].get("sandboxes", [])
            matches = [row for row in rows if row.get("name") == prior["name"]]
            if len(matches) == 1:
                save(path, {**prior, "id": matches[0]["id"]})
                return self._sandbox()
            raise ExecutionUnknown("Boat创建响应丢失，无法唯一收养；不再创建")
        limits = self.cli.call("limits")[-1]
        if not limits.get("canStart") or limits.get("creditUsedSeconds", 0) + self.config.ttl > self.config.max_standard_seconds:
            raise ExecutionUnknown("Boat额度或首次5美元预算不足")
        name = "newchanlun-" + self.run.spec.run_id
        save(path, {"name": name, "state": "creating"})
        rows = self.cli.call("new", "--name", name, "--type", "small", "--no-env", "--ttl", str(self.config.ttl))
        ready = next((row for row in rows if row.get("event") == "ready" and row.get("id")), None)
        if ready is None:
            raise ExecutionUnknown("Boat创建未返回ready，须按名称收养")
        save(path, {"name": name, "id": ready["id"]})
        return ready["id"]

    def _command(self, binding: dict, action: str) -> str:
        return "python3 " + shlex.quote(self.root + "/controller/boat_remote.py") + " " + action + " " + shlex.quote(binding["remote_request"])

    def submit(self, key: WorkerKey) -> None:
        binding = self._binding(key)
        if binding and binding["submission"] in {"submitting", "submitted", "collected"}:
            return
        sandbox = self._sandbox()
        runtime_head = self.run.spec.execution["runtime_head"]
        package_id = f"{key.target_head}-g{self.generation}-{key.role}-{runtime_head[:12]}"
        local = self.run.dir / "boat/packages" / package_id
        if not (local / "package.json").exists():
            if local.exists():
                shutil.rmtree(local)
            build_package(self.run.source_repo, key.target_head, local, Path(self.config.pstack_source), Path(self.config.model_rules))
        control = local / "controller"
        control.mkdir(exist_ok=True)
        for name in ("boat_remote.py",):
            (control / name).write_bytes(git(self.run.source_repo, "show", runtime_head + ":.sandcastle/deepagents-pilot/pilot1742/" + name))
        runtime = local / "runtime"
        runtime.mkdir(exist_ok=True)
        for name in ("package.json", "package-lock.json"):
            (runtime / name).write_bytes(git(self.run.source_repo, "show", runtime_head + ":" + name))
        (runtime / ".sandcastle").mkdir(exist_ok=True)
        for name in ("boat-worker.mts", "codex-child.ts", "docker-isolation.mts", "acceptance-dir.mts",
                     "git-environment.ts", "main-model.mts"):
            (runtime / ".sandcastle" / name).write_bytes(git(self.run.source_repo, "show", runtime_head + ":.sandcastle/" + name))
        remote = self.root + "/" + package_id
        intent = json.loads((self.run.dir / "intents" / f"{key.name}.json").read_text())
        request = {"identity": self._identity(key), "package_id": package_id, "root": remote,
                   "ticket": self.run.spec.ticket, "model": intent["model"], "mode": intent["mode"],
                   "endpoint": self.config.endpoint, "image": self.config.image,
                   "allowed_prefixes": list(self.run.spec.task.allowed_prefixes), "timeout": min(1700,  self.config.ttl - 30),
                   "prompt": Path(intent["prompt_file"]).read_text().replace(str(Path(self.config.pstack_source)), "/opt/newchanlun/pstack").replace(str(self.run.source_repo), "/workspace/repo")}
        save(local / "request.json", request)
        binding = {"identity": request["identity"], "sandbox_id": sandbox, "package_id": package_id,
                   "remote_request": remote + "/request.json", "remote_root": remote, "submission": "prepared",
                   "command_id": None, "process_id": None}
        save(self._path(key), binding)
        # `boat push` intentionally omits .git; ship our newly-built, credential-free
        # Git metadata explicitly in one archive instead of changing the target SHA.
        archive = local.with_suffix(".tar.gz")
        with tarfile.open(archive, "w:gz") as package:
            package.add(local, arcname=".")
        self.cli.exec(sandbox, "mkdir -p " + shlex.quote(remote) + "; printf '{}' ")
        self.cli.call("scp", str(archive), sandbox + ":" + remote + "/package.tar.gz")
        self.cli.exec(sandbox, "tar -xzf " + shlex.quote(remote + "/package.tar.gz") + " -C " + shlex.quote(remote) +
                      "; mkdir -p " + shlex.quote(self.root + "/controller") + "; cp " +
                      shlex.quote(remote + "/controller/boat_remote.py") + " " + shlex.quote(self.root + "/controller/boat_remote.py") + "; printf '{}'")
        self._provision_image(sandbox)
        self._provision_key(sandbox)
        save(self._path(key), {**binding, "submission": "submitting"})
        rows = self.cli.call("exec", sandbox, "--detach", "--", "bash", "-lc", self._command(binding, "submit"))
        launch = rows[-1]
        save(self._path(key), {**binding, "submission": "submitted", "command_id": launch.get("commandId"),
                              "process_id": launch.get("pid")})

    def _provision_image(self, sandbox: str) -> None:
        # A sha256 image ID cannot be pulled from a registry by itself. Copy only
        # the prebuilt approved image, never rebuild floating npm/toolchain layers.
        source_id = self.config.image_source_sandbox
        if source_id is None or source_id == sandbox:
            return  # The runner probes exact presence and the hardened contract.
        source = subprocess.Popen([self.cli.executable, "ssh", source_id,
                                   "docker image save " + shlex.quote(self.config.image)],
                                  stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        destination = subprocess.run([self.cli.executable, "ssh", sandbox, "docker image load > /dev/null"],
                                     stdin=source.stdout, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        source.stdout.close()
        if source.wait() or destination.returncode:
            raise ExecutionUnknown("冻结镜像传输失败，未启动工蜂")

    def _provision_key(self, sandbox: str) -> None:
        target = self.root + "/private/model-key"
        source = subprocess.Popen([self.cli.executable, "ssh", self.config.key_source_sandbox,
                                   "cat " + shlex.quote(self.config.key_source_path)], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
        destination = subprocess.run([self.cli.executable, "ssh", sandbox,
                                      "umask 077; mkdir -p " + shlex.quote(self.root + "/private") + "; cat > " + shlex.quote(target)],
                                     stdin=source.stdout, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        source.stdout.close()
        if source.wait() or destination.returncode:
            raise ExecutionUnknown("模型能力受管传输失败，未启动工蜂")

    def observe(self, key: WorkerKey) -> WorkerFact:
        binding = self._binding(key)
        if binding is None:
            return NotStarted()
        if binding["submission"] == "prepared":
            return NotStarted()
        try:
            self._sandbox()
            payload = self.cli.exec(binding["sandbox_id"], self._command(binding, "observe"))
            if payload.get("receipt_path"):
                receipt = binding["remote_root"] + "/claim/receipt.json"
                if payload["receipt_path"] != receipt:
                    raise ExecutionUnknown("远端回执路径不符合绑定")
                local_receipt = self.run.dir / "boat" / (key.name + ".remote.json")
                self.cli.call("scp", binding["sandbox_id"] + ":" + receipt, str(local_receipt))
                if local_receipt.stat().st_size > 96 * 1024 * 1024:
                    raise ExecutionUnknown("远端回执超过批次界限")
                payload = json.loads(local_receipt.read_text())
            if payload.get("identity") != binding["identity"] or payload.get("package_id") != binding["package_id"]:
                return EffectUnknown("Boat远端身份或package不符")
            if payload.get("state") == "running":
                return Running()
            if payload.get("state") != "terminal":
                return EffectUnknown(payload.get("reason", "Boat工蜂没有可证终态"))
            with self.run._fence("dispatch"):
                session = payload.get("record", {}).get("external_session_log")
                if session:
                    expected_session = binding["remote_root"] + "/claim/session.jsonl"
                    if session != expected_session:
                        raise ExecutionUnknown("远端session日志路径不符合绑定")
                    target = self.run._key_dir(key) / "session.jsonl"
                    target.parent.mkdir(parents=True, exist_ok=True)
                    if not target.exists():
                        self.cli.call("scp", binding["sandbox_id"] + ":" + session, str(target))
                    payload["record"]["external_session_log"] = str(target)
                self.collect(key, payload)
            from .run import parse_result
            out = self.run._key_dir(key)
            return Terminal(parse_result(key, json.loads((out / "result.json").read_text()), out, self.run.spec.ticket))
        except (ExecutionUnknown, ValueError, KeyError, TypeError, OSError) as error:
            return EffectUnknown(str(error))

    def collect(self, key: WorkerKey, payload: dict) -> None:
        binding = self._binding(key)
        if not binding or payload["identity"] != self._identity(key) or payload["package_id"] != binding["package_id"]:
            raise ExecutionUnknown("拒绝收集其他身份产物")
        if payload.get("exit_code") != 0 or not isinstance(payload.get("record"), dict):
            raise ExecutionUnknown("远端退出失败或未知，不能产生业务终态")
        record = payload["record"]
        if (record.get("expected_head") != key.target_head or record.get("model") != self.run.spec.model_for(key.role)
                or (record.get("status") == "completed" and record.get("actual_model") != record.get("model"))):
            raise ExecutionUnknown("远端模型或head不符")
        checkout = self.run.spec.checkout_for(key.role)
        if git(checkout, "rev-parse", "HEAD").decode().strip() != key.target_head:
            # An already managed commit is reconciled by PilotRun, never by reapplying a patch.
            if binding["submission"] == "collected" and (self.run._key_dir(key) / "result.json").exists():
                return
            raise ExecutionUnknown("收集目标head已漂移")
        from .run import _porcelain_paths, parse_result
        parsed = parse_result(key, record, self.run._key_dir(key), self.run.spec.ticket)
        files = payload.get("files", [])
        if not isinstance(files, list):
            raise ExecutionUnknown("产物清单不是列表")
        expected_paths: list[str] = []
        decoded: dict[str, bytes | None] = {}
        total = 0
        for entry in files:
            path = str(safe_path(entry["path"]))
            if path in decoded or not any(path.startswith(prefix) for prefix in self.run.spec.task.allowed_prefixes):
                raise ExecutionUnknown("远端产物重复或越出写集")
            if entry.get("mode") not in {"100644", "100755", "deleted"}:
                raise ExecutionUnknown("远端产物模式或链接不允许")
            data = base64.b64decode(entry.get("content", ""), validate=True)
            total += len(data)
            if len(data) > MAX_FILE_BYTES or total > 64 * 1024 * 1024:
                raise ExecutionUnknown("远端产物超过文件或批次界限")
            decoded[path] = None if entry["mode"] == "deleted" else data
            expected_paths.append(path)
            destination = checkout / path
            if destination.is_symlink() or not destination.resolve().is_relative_to(checkout.resolve()):
                raise ExecutionUnknown("本地收集路径含逃逸链接")
        if key.role != "author" and files:
            raise ExecutionUnknown("只读独审产生文件改动")
        if set(parsed.changed_paths_after) != set(expected_paths):
            raise ExecutionUnknown("业务回执与产物清单不同")
        receipt_path = self.run._key_dir(key) / "result.json"
        if binding["submission"] == "collected" and receipt_path.exists():
            if json.loads(receipt_path.read_text()) != record:
                raise ExecutionUnknown("已收集业务回执发生漂移")
            return
        intent_path = self.run.dir / "boat" / (key.name + ".collect.json")
        dirty = _porcelain_paths(checkout)
        intent = {"identity": binding["identity"], "files": files}
        recovering = intent_path.exists()
        if recovering:
            if json.loads(intent_path.read_text()) != intent or set(dirty) - set(expected_paths):
                raise ExecutionUnknown("恢复收集的清单或其他写者状态漂移")
        elif dirty:
            raise ExecutionUnknown("收集前checkout不干净")
        # Each partially-applied path must equal either its frozen base or complete target.
        # A concurrent write on the same path is not permission to overwrite it.
        for entry in files:
            path = entry["path"]
            destination = checkout / path
            current = destination.read_bytes() if destination.is_file() else None
            current_mode = ("100755" if destination.stat().st_mode & 0o111 else "100644") if current is not None else "deleted"
            tree = git(checkout, "ls-tree", key.target_head, "--", path).decode()
            base_mode = tree.split()[0] if tree else "deleted"
            base_data = git(checkout, "show", key.target_head + ":" + path) if tree else None
            at_base = current == base_data and current_mode == base_mode
            at_target = current == decoded[path] and current_mode == entry["mode"]
            if not at_base and not (recovering and at_target):
                raise ExecutionUnknown("收集路径存在不属于本产物的并发内容或模式")
        save(intent_path, intent)
        if record["status"] == "completed":
            for entry in files:
                destination = checkout / entry["path"]
                if entry["mode"] == "deleted":
                    destination.unlink(missing_ok=True)
                else:
                    destination.parent.mkdir(parents=True, exist_ok=True)
                    temporary = self.run.dir / "boat" / (key.name + ".collect-content.tmp")
                    temporary.write_bytes(decoded[entry["path"]])
                    temporary.chmod(0o755 if entry["mode"] == "100755" else 0o644)
                    os.replace(temporary, destination)
        save(receipt_path, record)
        save(self._path(key), {**binding, "submission": "collected"})

    def cancel(self, key: WorkerKey) -> WorkerFact:
        binding = self._binding(key)
        if binding is None:
            return NotStarted()
        with self.run._fence("dispatch"):
            self.cli.exec(binding["sandbox_id"], self._command(binding, "cancel"))
        return self.observe(key)
