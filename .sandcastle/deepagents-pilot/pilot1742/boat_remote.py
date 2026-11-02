from __future__ import annotations

import fcntl
import json
import os
import shutil
import signal
import subprocess
import sys
import time
from contextlib import contextmanager, suppress
from pathlib import Path

CHECK_IMAGE = 'test "$(docker image inspect "$1" --format "{{.Id}}")" = "$1"'


class Cancelled(Exception):
    pass


class DeadlineReached(Exception):
    pass


def save(path: Path, data: dict) -> None:
    temp = path.with_suffix(".tmp")
    temp.write_text(json.dumps(data, ensure_ascii=False) + "\n")
    os.replace(temp, path)


def identity(request: dict) -> dict:
    return {"identity": request["identity"], "package_id": request["package_id"]}


def process_start(pid: int) -> str:
    # Linux starttime is tied to this process instance, unlike a reusable PID.
    return Path(f"/proc/{pid}/stat").read_text().split(")", 1)[1].split()[19]


def same_process(record: dict, argument: str | None = None) -> bool:
    try:
        return process_start(record["pid"]) == record["start"] and (
            argument is None or argument.encode() in Path(f"/proc/{record['pid']}/cmdline").read_bytes().split(b"\0"))
    except (FileNotFoundError, ProcessLookupError):
        return False


def claim_atomically(root: Path, name: str, data: dict) -> bool:
    """The claim directory appears with its first file already inside, so no reader sees an empty claim."""
    staging = root / f".claim-{os.getpid()}"
    staging.mkdir(mode=0o700)
    save(staging / name, data)
    try:
        os.rename(staging, root / "claim")
        return True
    except OSError:
        shutil.rmtree(staging)
        return False


@contextmanager
def launch_lock(claim: Path):
    with open(claim / "launch.lock", "a") as handle:
        fcntl.flock(handle, fcntl.LOCK_EX)
        yield


def observe(request: dict, request_path: Path) -> dict:
    claim = Path(request["root"]) / "claim"
    if (claim / "receipt.json").exists():
        return {**identity(request), "state": "receipt"}
    if not claim.exists():
        return {**identity(request), "state": "unclaimed"}
    state = claim / "state.json"
    if state.exists() and same_process(json.loads(state.read_text()), str(request_path)):
        return {**identity(request), "state": "running"}
    return {**identity(request), "state": "unknown", "reason": "远端已认领，但监督进程不在且没有终态回执"}


def stage(claim: Path, request_path: Path, argv: list[str], cwd: Path, log_name: str, deadline: float) -> int:
    with launch_lock(claim):
        if (claim / "cancelled").exists():
            raise Cancelled
        with open(claim / log_name, "ab") as log:
            child = subprocess.Popen(argv, cwd=cwd, stdout=log, stderr=log, start_new_session=True,
                                     env={"PATH": os.environ["PATH"], "HOME": os.environ["HOME"]})
        save(claim / "child.json", {"pid": child.pid, "start": process_start(child.pid), "request": str(request_path)})
    try:
        code = child.wait(timeout=max(0.0, deadline - time.time()))
    except subprocess.TimeoutExpired:
        os.killpg(child.pid, signal.SIGTERM)
        try:
            child.wait(timeout=10)
        except subprocess.TimeoutExpired:
            os.killpg(child.pid, signal.SIGKILL)
            child.wait()
        raise DeadlineReached from None
    if (claim / "cancelled").exists():
        raise Cancelled
    return code


def run(request: dict, request_path: Path, deadline: float) -> dict:
    root = Path(request["root"])
    claim = root / "claim"
    if not claim_atomically(root, "state.json", {**identity(request), "pid": os.getpid(),
                                                 "start": process_start(os.getpid()), "deadline": deadline}):
        return observe(request, request_path)
    runtime = root / "runtime"
    failure = {**identity(request), "state": "unknown", "exit_code": None}
    try:
        if code := stage(claim, request_path, ["sh", "-c", CHECK_IMAGE, "check-image", request["image"]], runtime, "image.log", deadline):
            raise subprocess.CalledProcessError(code, "frozen docker image missing")
        if code := stage(claim, request_path, ["npm", "ci", "--ignore-scripts"], runtime, "setup.log", deadline):
            raise subprocess.CalledProcessError(code, "npm ci")
        for manifest in sorted((root / "pstack").rglob("package.json")):
            if "node_modules" in manifest.parts:
                continue
            if not (manifest.parent / "bun.lock").is_file():
                raise OSError("pstack script manifest has no frozen bun lock")
            if code := stage(claim, request_path, ["bun", "install", "--frozen-lockfile", "--ignore-scripts"], manifest.parent,
                             "skills-install.log", deadline):
                raise subprocess.CalledProcessError(code, "bun install")
        code = stage(claim, request_path, ["node", "--import", "tsx", ".sandcastle/boat-worker.mts", str(request_path)],
                     runtime, "runner.log", deadline)
        failure.update(exit_code=code, reason="工蜂执行未交业务回执，保留日志不重派")
    except Cancelled:
        failure.update(exit_code=143, reason="管理者取消；未产生成功业务回执")
    except DeadlineReached:
        failure.update(exit_code=143, timed_out=True, reason="到达VM自动停止前的监督截止；未产生成功业务回执")
    except (OSError, subprocess.SubprocessError) as error:
        if isinstance(error, subprocess.CalledProcessError):
            failure["exit_code"] = error.returncode
        failure.update(reason=f"远端准备或监督失败，保留认领不重派：{type(error).__name__}")
    finally:
        (root.parent / "private/model-key").unlink(missing_ok=True)
    if not (claim / "receipt.json").exists():
        save(claim / "receipt.json", failure)
    return observe(request, request_path)


def cancel(request: dict, request_path: Path) -> dict:
    root = Path(request["root"])
    claim = root / "claim"
    if not claim.exists() and claim_atomically(root, "receipt.json", {
            **identity(request), "state": "unknown", "exit_code": 143, "reason": "取消先于监督器认领；模型任务未启动"}):
        return observe(request, request_path)
    if (claim / "receipt.json").exists():
        return observe(request, request_path)
    with launch_lock(claim):
        (claim / "cancelled").touch(exist_ok=True)
        child = claim / "child.json"
        if child.exists() and same_process(record := json.loads(child.read_text())):
            with suppress(ProcessLookupError):
                os.killpg(record["pid"], signal.SIGTERM)
    return observe(request, request_path)


if __name__ == "__main__":
    action, raw_path, *rest = sys.argv[1:]
    path = Path(raw_path)
    request = json.loads(path.read_text())
    if action == "submit":
        result = run(request, path, float(rest[0]))
    else:
        result = {"observe": observe, "cancel": cancel}[action](request, path)
    print(json.dumps(result, ensure_ascii=False))
