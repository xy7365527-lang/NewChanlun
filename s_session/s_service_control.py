#!/usr/bin/env python3
"""#1372：正式 launcher 的 S/Q 独立生命周期控制，不解释或生成结构事实。"""

import argparse
import errno
import fcntl
import hashlib
import json
import os
import re
import signal
import socket
import stat
import subprocess
import sys
import time
import uuid
from pathlib import Path

from s_socket_client import decode_frame, exchange, validate_reply


def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"),
                      allow_nan=False).encode("utf-8")


def positive(value, field):
    if type(value) is not str or not value.isascii() or not value.isdigit() or value.startswith("0"):
        raise ValueError(field + " 必须为规范正整数文本")
    result = int(value)
    if result > 2**63 - 1:
        raise ValueError(field + " 超过 i64")
    return result


def read_config(path):
    config = decode_frame(path.read_bytes())
    if config.get("schema_revision") != "s-launcher/2":
        raise ValueError("启动配置必须显式为 s-launcher/2")
    for field in ("session_id", "session_generation", "source_namespace", "source_epoch",
                  "producer_id", "producer_epoch", "writer_epoch"):
        if type(config.get(field)) is not str or not config[field]:
            raise ValueError("缺少启动身份 " + field)
    for field in ("writer_epoch", "query_epoch", "delivery_retain_generations", "port", "startup_timeout_ms"):
        positive(config.get(field), field)
    if int(config["port"]) > 65535:
        raise ValueError("端口越域")
    for field in ("db", "socket", "binary", "python", "catalog", "profile", "clock_plan",
                  "browser", "query_resource_config"):
        value = config.get(field)
        if type(value) is not str or not value:
            raise ValueError("缺少启动路径 " + field)
        config[field] = str((path.parent / value).resolve())
    if len(os.fsencode(config["socket"])) > 100:
        raise ValueError("Unix socket 路径超过本 launcher 的跨平台 100 字节上限")
    for field in ("max_frame_bytes", "read_timeout_ms", "write_timeout_ms", "response_timeout_ms",
                  "audit_cache_source_bytes", "queue_capacity", "max_connections"):
        if type(config.get("s_resources")) is not dict:
            raise ValueError("s_resources 必须为资源对象")
        positive(config["s_resources"].get(field), "s_resources." + field)
    return config


def remaining(deadline):
    value = deadline - time.monotonic()
    if value <= 0:
        raise TimeoutError("生命周期动作超过声明总期限；已执行的动作不回退为未执行")
    return value


def read_health(port, deadline):
    """短 HTTP/1.0 控制请求，头和正文共享绝对期限及字节上限。"""
    limit = 1024 * 1024
    with socket.create_connection(("127.0.0.1", int(port)), timeout=remaining(deadline)) as connection:
        connection.settimeout(remaining(deadline))
        connection.sendall(b"GET /api/state HTTP/1.0\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        raw = bytearray()
        header_end = -1
        while True:
            connection.settimeout(remaining(deadline))
            chunk = connection.recv(min(65536, limit + 16384 + 1 - len(raw)))
            if not chunk:
                break
            raw.extend(chunk)
            if header_end < 0:
                header_end = raw.find(b"\r\n\r\n")
                if header_end > 16384 or (header_end < 0 and len(raw) > 16384):
                    raise ValueError("健康回执头超过 16 KiB 控制器上限")
            if len(raw) > limit + 16384 or (header_end >= 0 and len(raw) - header_end - 4 > limit):
                raise ValueError("健康回执超过 1 MiB 控制器上限")
        remaining(deadline)
    if header_end < 0:
        raise ValueError("健康回执缺少完整 HTTP 头")
    lines = bytes(raw[:header_end]).split(b"\r\n")
    status = lines[0].split(b" ")
    if len(status) < 3 or status[0] not in (b"HTTP/1.0", b"HTTP/1.1") or status[1] != b"200":
        raise ValueError("健康回执不是 HTTP 200")
    headers = {}
    for line in lines[1:]:
        if b":" not in line:
            raise ValueError("健康回执头格式无效")
        name, value = line.split(b":", 1)
        name = name.strip().lower()
        if name in headers:
            raise ValueError("健康回执头重复")
        headers[name] = value.strip()
    body = bytes(raw[header_end + 4:])
    if b"transfer-encoding" in headers:
        raise ValueError("控制健康回执不支持 Transfer-Encoding")
    if b"content-length" in headers:
        value = headers[b"content-length"]
        if not value.isdigit() or int(value) != len(body):
            raise ValueError("健康回执正文长度不符")
    result = decode_frame(body)
    remaining(deadline)
    return result


class ProcessObservationUnavailable(ValueError):
    """进程存在，但 ps 暂未给出可核验的完整命令；不是已匹配或已退出。"""

    def __init__(self, start_identity):
        super().__init__("进程命令暂不可读，不能确认完整参数")
        self.start_identity = start_identity


def process_identity(pid, command=None, *, deadline=None):
    """核 PID 启动时间、参数及 UID；macOS Python 启动壳换 exe 时参数必须完全相同。"""
    if type(pid) is not int or pid <= 1:
        return None
    timeout = 1 if deadline is None else min(1, remaining(deadline))
    result = subprocess.run(["ps", "-p", str(pid), "-o", "uid=,stat=,lstart=,command="],
                            capture_output=True, text=True, check=False, timeout=timeout)
    if deadline is not None:
        remaining(deadline)
    if result.returncode == 1 and not result.stdout.strip() and not result.stderr.strip():
        return None
    if result.returncode != 0 or not result.stdout.strip():
        raise ValueError("无法确认进程状态，不能将查询失败当作已退出")
    line = result.stdout.strip()
    columns = line.split(None, 2)
    if columns[0] != str(os.getuid()):
        raise ValueError("拒绝控制不属于本用户的进程")
    if columns[1].startswith("Z"):
        return None
    # stat 会随正常运行在 S/R 间变化，不属于进程身份。
    identity = columns[0] + " " + columns[2]
    if command and re.fullmatch(r"[Pp]ython(?:[0-9]+(?:\.[0-9]+)*)?", Path(command[0]).name):
        timestamp_and_command = columns[2].split(None, 5)
        if len(timestamp_and_command) != 6:
            raise ValueError("无法读取进程启动时间/命令")
        actual = timestamp_and_command[5]
        if re.fullmatch(r"\([Pp]ython(?:[0-9]+(?:\.[0-9]+)*)?\)", actual):
            raise ProcessObservationUnavailable(columns[0] + " " + " ".join(timestamp_and_command[:5]))
        suffix = " " + " ".join(command[1:])
        if not actual.endswith(suffix):
            raise ValueError("Python 服务参数与本次启动命令不符")
        executable = actual[:-len(suffix)]
        if not re.fullmatch(r"[Pp]ython(?:[0-9]+(?:\.[0-9]+)*)?", Path(executable).name):
            raise ValueError("Python 服务实际可执行程序身份不符")
        identity = columns[0] + " " + " ".join(timestamp_and_command[:5]) + " " + " ".join(command)
    return identity


class Controller:
    def __init__(self, config, state_dir):
        self.config = config
        self.db = Path(config["db"])
        if not self.db.is_absolute():
            raise ValueError("生命周期数据库必须为已解析的绝对路径")
        self.state_dir = state_dir.resolve()
        self.state_dir.mkdir(parents=True, exist_ok=True)
        self.db.parent.mkdir(parents=True, exist_ok=True)
        self.db_lock = None
        self.lock = (self.state_dir / ".control.lock").open("a+b")
        try:
            fcntl.flock(self.lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            self.db_lock = Path(str(self.db) + ".control.lock").open("a+b")
            fcntl.flock(self.db_lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            owner_path = Path(str(self.db) + ".control-owner.json")
            owner = {"db": config["db"], "state_dir": str(self.state_dir)}
            if owner_path.exists():
                if decode_frame(owner_path.read_bytes()) != owner:
                    raise ValueError("同一数据库已绑定其他控制目录，不得以新目录采用未知进程")
            else:
                with owner_path.open("x") as output:
                    json.dump(owner, output, ensure_ascii=True)
                    output.flush()
                    os.fsync(output.fileno())
        except (OSError, ValueError) as exc:
            if self.db_lock is not None:
                self.db_lock.close()
            self.lock.close()
            if isinstance(exc, ValueError):
                raise
            raise ValueError("控制目录或数据库已有操作，拒绝并发覆盖进程记录：" + str(exc)) from exc

    def close(self):
        self.db_lock.close()
        self.lock.close()

    def record_path(self, service):
        return self.state_dir / (service + ".process.json")

    def status(self, service):
        path = self.record_path(service)
        if not path.exists():
            return {"service": service, "state": "not_started"}
        record = decode_frame(path.read_bytes())
        fields = {"service", "pid", "db", "process_identity", "command", "control_instance_id"}
        if (not fields.issubset(record) or not set(record).issubset(fields | {"socket_identity"})
                or type(record.get("pid")) is not int or record["pid"] <= 1
                or any(type(record.get(k)) is not str or not record[k]
                       for k in ("service", "db", "process_identity", "control_instance_id"))
                or type(record.get("command")) is not list or not record["command"]
                or any(type(arg) is not str or not arg for arg in record["command"])):
            raise ValueError("进程记录字段不完整或类型无效，状态未知")
        if "socket_identity" in record:
            identity = record["socket_identity"]
            if (service != "s" or type(identity) is not dict
                    or set(identity) != {"path", "device", "inode", "uid"}
                    or identity.get("path") != self.config.get("socket")
                    or any(type(identity.get(k)) is not int for k in ("device", "inode", "uid"))):
                raise ValueError("已登记 socket 身份无效或不属于本次配置")
        if record.get("db") != self.config["db"] or record.get("service") != service:
            raise ValueError("进程记录不属于本次启动配置")
        identity = process_identity(record.get("pid"), record.get("command"))
        if identity is None:
            return {**record, "state": "stopped"}
        if identity != record.get("process_identity"):
            raise ValueError("PID 已复用或进程命令改变，拒绝采用旧记录")
        return {**record, "state": "running"}

    def record(self, service, pid, command, instance_id, *, child=None, deadline=None):
        if child is not None and (pid != child.pid or deadline is None):
            raise ValueError("初始登记必须绑定本次直接子进程及启动总期限")
        observed_start = None
        while True:
            if deadline is not None:
                remaining(deadline)
            if child is not None and child.poll() is not None:
                raise ValueError(service + " 启动后立即退出；请查其日志")
            try:
                identity = process_identity(pid, command, deadline=deadline)
            except ProcessObservationUnavailable as exc:
                # 仅本次 Popen 初始登记等待观测补全；status/Ready/stop 均不进入此分支。
                if child is None:
                    raise
                if observed_start is not None and observed_start != exc.start_identity:
                    raise ValueError("初始登记期间进程启动身份改变") from exc
                observed_start = exc.start_identity
                if child.poll() is not None:
                    raise ValueError(service + " 启动后立即退出；请查其日志") from exc
                time.sleep(min(0.01, remaining(deadline)))
                continue
            if identity is None or (child is not None and child.poll() is not None):
                raise ValueError(service + " 启动后立即退出；请查其日志")
            if observed_start is not None and not identity.startswith(observed_start + " "):
                raise ValueError("初始登记期间进程启动身份改变")
            if deadline is not None:
                remaining(deadline)
            break
        record = {"service": service, "pid": pid, "db": self.config["db"],
                  "process_identity": identity, "command": command, "control_instance_id": instance_id}
        self.save_record(service, record)

    def save_record(self, service, record):
        target = self.record_path(service)
        temporary = target.with_suffix(".tmp")
        temporary.write_text(json.dumps(record, ensure_ascii=True, indent=2))
        os.replace(temporary, target)

    def socket_identity(self):
        path = Path(self.config["socket"])
        current = path.lstat()
        if not stat.S_ISSOCK(current.st_mode) or current.st_uid != os.getuid():
            raise ValueError("通信路径不是本用户的 Unix socket，拒绝采用或删除")
        return {"path": str(path), "device": current.st_dev, "inode": current.st_ino, "uid": current.st_uid}

    def prepare_socket(self, deadline):
        path = Path(self.config["socket"])
        if not os.path.lexists(path):
            return None
        record = self.status("s")
        if record["state"] != "stopped" or record.get("socket_identity") is None:
            raise ValueError("已有 socket 缺少已确认停止的原 S 及其就绪时身份，不接管未知路径")
        expected = record["socket_identity"]
        if self.socket_identity() != expected:
            raise ValueError("socket 已被替换，拒绝删除")
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as connection:
            connection.settimeout(min(0.5, remaining(deadline)))
            try:
                connection.connect(str(path))
            except OSError as exc:
                if exc.errno != errno.ECONNREFUSED:
                    raise ValueError("无法确认残留 socket 无监听者：" + str(exc)) from exc
            else:
                raise ValueError("原路径仍有活监听者，拒绝删除")
        remaining(deadline)
        if self.socket_identity() != expected:
            raise ValueError("删除前 socket 身份改变，拒绝操作")
        path.unlink()
        return {"removed_stopped_service_socket": expected, "old_pid": record["pid"]}

    def start_process(self, service, command, environment=None, *, deadline=None):
        if deadline is None:
            deadline = time.monotonic() + int(self.config["startup_timeout_ms"]) / 1000
        remaining(deadline)
        if self.status(service)["state"] == "running":
            raise ValueError(service + " 已在运行，不覆盖 PID 记录")
        instance_id = str(uuid.uuid4())
        environment = dict(os.environ if environment is None else environment)
        environment["S_CONTROL_INSTANCE_ID"] = instance_id
        with (self.state_dir / (service + ".log")).open("ab") as log:
            remaining(deadline)
            process = subprocess.Popen(command, stdin=subprocess.DEVNULL, stdout=log, stderr=log,
                                       start_new_session=True, env=environment)
        try:
            self.record(service, process.pid, command, instance_id, child=process, deadline=deadline)
        except Exception as exc:
            # 仅善后本次尚未登记的直接子进程，不触碰其他已运行服务。
            if process.poll() is None:
                process.terminate()
            try:
                process.wait(timeout=int(self.config["startup_timeout_ms"]) / 1000)
            except subprocess.TimeoutExpired as timeout:
                raise ValueError(f"{service} 登记失败且本次子进程 pid={process.pid} 尚未退出：{exc}") from timeout
            raise
        return process

    def runtime_environment(self, deadline):
        python = self.config["python"]
        result = subprocess.run([python, "-c", "import json,sysconfig; print(json.dumps([sysconfig.get_config_var('LIBDIR'),sysconfig.get_config_var('LDLIBRARY')]))"],
                                capture_output=True, check=True, timeout=remaining(deadline))
        library_dir, library_name = json.loads(result.stdout)
        if (type(library_dir) is not str or type(library_name) is not str
                or not (Path(library_dir) / library_name).is_file()):
            raise ValueError("显式 Python 不具备可验证的共享运行库")
        environment = dict(os.environ)
        environment["PYO3_PYTHON"] = python
        for key in ("LD_LIBRARY_PATH", "DYLD_LIBRARY_PATH"):
            environment[key] = library_dir + (os.pathsep + environment[key] if environment.get(key) else "")
        return environment

    def ready_request(self, writer_epoch):
        config = self.config
        payload = {"op": "ready", "target_session_id": config["session_id"],
                   "target_session_generation": config["session_generation"],
                   "writer_epoch": writer_epoch}
        return {"schema_revision": "s-session/2", "session_id": config["session_id"],
                "session_generation": config["session_generation"],
                "source_namespace": config["source_namespace"], "source_epoch": config["source_epoch"],
                "message_id": "launcher-ready-" + writer_epoch,
                "producer_id": config["producer_id"], "producer_epoch": config["producer_epoch"],
                "payload_hash": hashlib.sha256(canonical(payload)).hexdigest(),
                "causal_refs": [], "payload": payload}

    def wait_ready(self, service, writer_epoch=None, query_epoch=None, deadline=None):
        if deadline is None:
            deadline = time.monotonic() + int(self.config["startup_timeout_ms"]) / 1000
        expected = self.status(service)
        detail = "启动尚无健康回执"
        while time.monotonic() < deadline:
            if self.status(service)["state"] != "running":
                raise ValueError(service + " 已退出；请查 " + str(self.state_dir / (service + ".log")))
            remaining_ms = max(1, int((deadline - time.monotonic()) * 1000))
            if service == "s":
                request = self.ready_request(writer_epoch)
                reply = exchange(self.config["socket"], request,
                                 timeout_ms=remaining_ms,
                                 max_frame_bytes=int(self.config["s_resources"]["max_frame_bytes"]))
                response = reply.get("response")
                response = response if type(response) is dict else {}
                payload = response.get("payload")
                payload = payload if type(payload) is dict else {}
                # 字段与 S v2 正式响应一致后才将进程文件存在提升为 ready。
                if (reply["transport"] == "Received" and payload.get("kind") == "Ready"
                        and response.get("session_id") == self.config["session_id"]
                        and response.get("session_generation") == self.config["session_generation"]
                        and response.get("control_instance_id") == expected.get("control_instance_id")):
                    validate_reply(request, response, producer="S", producer_epoch=writer_epoch,
                                   allow_control_instance=True)
                    self.confirm_ready_process(service, expected, deadline)
                    # 只有正式Ready nonce与本次活PID同时成立，才登记此路径的清理资格。
                    record = {k: v for k, v in self.status("s").items() if k != "state"}
                    record["socket_identity"] = self.socket_identity()
                    self.save_record("s", record)
                    remaining(deadline)
                    return {"service": service, "state": "ready", "reply": response}
                detail = reply
            else:
                body = None
                try:
                    body = read_health(self.config["port"], deadline)
                except (OSError, ValueError) as exc:
                    detail = str(exc)
                if body is not None:
                    cut = body.get("cut")
                    cut = cut if type(cut) is dict else {}
                    if (body.get("ok") is True and cut.get("session_id") == self.config["session_id"]
                            and cut.get("session_generation") == self.config["session_generation"]
                            and body.get("producer_epoch") == query_epoch
                            and body.get("control_instance_id") == expected.get("control_instance_id")):
                        self.confirm_ready_process(service, expected, deadline)
                        return {"service": service, "state": "ready", "reply": body}
                    detail = body
            time.sleep(min(0.05, max(0, deadline - time.monotonic())))
        raise TimeoutError(f"{service} 未在声明期限内就绪：{detail}")

    def confirm_ready_process(self, service, expected, deadline):
        remaining(deadline)
        current = self.status(service)
        if (current["state"] != "running" or current.get("pid") != expected.get("pid")
                or current.get("process_identity") != expected.get("process_identity")
                or not expected.get("control_instance_id")):
            raise ValueError("健康回执返回时本次启动进程身份已失效")
        remaining(deadline)

    def start_s(self, writer_epoch):
        config = self.config
        deadline = time.monotonic() + int(config["startup_timeout_ms"]) / 1000
        environment = self.runtime_environment(deadline)
        cleanup = self.prepare_socket(deadline)
        initialization = None
        if not Path(config["db"]).exists():
            init = subprocess.run([config["binary"], "init", "--db", config["db"], "--session", config["session_id"],
                                   "--catalog", config["catalog"], "--profile", config["profile"],
                                   "--session-generation", config["session_generation"], "--clock-plan", config["clock_plan"],
                                   "--delivery-retain-generations", config["delivery_retain_generations"]],
                                  capture_output=True, check=True, env=environment, timeout=remaining(deadline))
            initialization = decode_frame(init.stdout)
            remaining(deadline)
        command = [config["binary"], "serve", "--db", config["db"], "--socket", config["socket"],
                   "--writer-epoch", writer_epoch, "--profile", config["profile"], "--clock-plan", config["clock_plan"]]
        for option, field in (("max-frame-bytes", "max_frame_bytes"), ("read-timeout-ms", "read_timeout_ms"),
                              ("write-timeout-ms", "write_timeout_ms"), ("queue-capacity", "queue_capacity"),
                              ("max-connections", "max_connections"), ("response-timeout-ms", "response_timeout_ms"),
                              ("audit-cache-source-bytes", "audit_cache_source_bytes")):
            command += ["--" + option, config["s_resources"][field]]
        remaining(deadline)
        self.start_process("s", command, environment, deadline=deadline)
        return {**self.wait_ready("s", writer_epoch, deadline=deadline), "initialization": initialization,
                "socket_cleanup": cleanup}

    def start_q(self, query_epoch):
        config = self.config
        deadline = time.monotonic() + int(config["startup_timeout_ms"]) / 1000
        script = Path(__file__).with_name("s_readonly_server.py")
        command = [config["python"], str(script), "--db", config["db"], "--port", config["port"],
                   "--browser", config["browser"], "--resource-config", config["query_resource_config"],
                   "--producer-epoch", query_epoch]
        self.start_process("q", command, deadline=deadline)
        return self.wait_ready("q", query_epoch=query_epoch, deadline=deadline)

    def stop(self, service, requested_signal):
        record = self.status(service)
        if record["state"] != "running":
            return record
        # 信号只发送给刚复核的本服务 PID，不扫端口、不发进程组/全局 kill。
        if process_identity(record["pid"], record.get("command")) != record["process_identity"]:
            raise ValueError("发信号前进程身份发生变化")
        os.kill(record["pid"], requested_signal)
        deadline = time.monotonic() + int(self.config["startup_timeout_ms"]) / 1000
        observations = []
        while time.monotonic() < deadline:
            try:
                observed = self.status(service)
            except ValueError as exc:
                # 已发出一次经身份复核的信号。退出期间命令文本可能短暂变化，
                # 只继续观察，不据此采用新身份、再发信号或认定已退出。
                if str(exc) not in observations:
                    observations.append(str(exc))
                observed = {"state": "unknown"}
            if observed["state"] == "stopped":
                return {"service": service, "state": "stopped", "pid": record["pid"],
                        "signal": signal.Signals(requested_signal).name,
                        "exit_observation_warnings": observations}
            time.sleep(0.05)
        raise TimeoutError(service + " 尚未退出，不自动升级信号")

    def recover(self, new_epoch, clock_event_id):
        if self.status("s")["state"] != "stopped":
            raise ValueError("须有本控制器记录且已确认原 S 停止，才可恢复换代")
        positive(new_epoch, "new_epoch")
        config = self.config
        deadline = time.monotonic() + int(config["startup_timeout_ms"]) / 1000
        environment = self.runtime_environment(deadline)
        recovery = subprocess.run([config["binary"], "recover", "--db", config["db"], "--new-epoch", new_epoch,
                                   "--clock-plan", config["clock_plan"], "--clock-event-id", clock_event_id],
                                  capture_output=True, check=True, env=environment, timeout=remaining(deadline))
        reply = decode_frame(recovery.stdout)
        remaining(deadline)
        return {"service": "s", "state": "recovered", "new_epoch": new_epoch,
                "reply": reply, "next_action": "start-s --writer-epoch " + new_epoch}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("action", choices=("start", "start-s", "start-q", "status", "stop", "stop-s", "stop-q", "recover"))
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--state-dir", type=Path, required=True)
    parser.add_argument("--writer-epoch")
    parser.add_argument("--query-epoch")
    parser.add_argument("--new-epoch")
    parser.add_argument("--clock-event-id")
    parser.add_argument("--signal", choices=("TERM", "KILL"), default="TERM")
    args = parser.parse_args()
    control = None
    try:
        config = read_config(args.config.resolve())
        control = Controller(config, args.state_dir)
        epoch = args.writer_epoch or config["writer_epoch"]
        query_epoch = args.query_epoch or config["query_epoch"]
        positive(epoch, "writer_epoch")
        positive(query_epoch, "query_epoch")
        if args.action == "start":
            result = [control.start_s(epoch), control.start_q(query_epoch)]
        elif args.action == "start-s":
            result = control.start_s(epoch)
        elif args.action == "start-q":
            result = control.start_q(query_epoch)
        elif args.action == "status":
            result = [control.status("s"), control.status("q")]
        elif args.action.startswith("stop"):
            targets = ["q", "s"] if args.action == "stop" else [args.action[-1]]
            result = [control.stop(service, getattr(signal, "SIG" + args.signal)) for service in targets]
        else:
            if not args.new_epoch or not args.clock_event_id:
                raise ValueError("recover 必须具名 --new-epoch 和 --clock-event-id")
            result = control.recover(args.new_epoch, args.clock_event_id)
        print(json.dumps({"ok": True, "result": result}, ensure_ascii=True, indent=2))
        return 0
    except (OSError, ValueError, subprocess.SubprocessError) as exc:
        print(json.dumps({"ok": False, "error": str(exc)}, ensure_ascii=True), file=sys.stderr)
        return 1
    finally:
        if control is not None:
            control.close()


if __name__ == "__main__":
    raise SystemExit(main())
