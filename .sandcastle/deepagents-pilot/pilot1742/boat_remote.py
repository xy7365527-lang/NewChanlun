from __future__ import annotations

import json
import os
import signal
import subprocess
import sys
from pathlib import Path


def save(path: Path, data: dict) -> None:
    temp = path.with_suffix(".tmp")
    temp.write_text(json.dumps(data, ensure_ascii=False) + "\n")
    os.replace(temp, path)


def identity(request: dict) -> dict:
    return {"identity": request["identity"], "package_id": request["package_id"]}


def observe(request: dict, request_path: Path) -> dict:
    claim = Path(request["root"]) / "claim"
    result = claim / "receipt.json"
    if result.exists():
        return json.loads(result.read_text())
    state = claim / "state.json"
    if state.exists():
        record = json.loads(state.read_text())
        pid = record.get("pid")
        try:
            cmd = Path(f"/proc/{pid}/cmdline").read_bytes().split(b"\0")
            if str(request_path).encode() in cmd:
                return {**identity(request), "state": "running", "pid": pid}
        except FileNotFoundError:
            pass
    return {**identity(request), "state": "unknown", "reason": "远端认领存在但无可证监督进程或终态"}


def run(request: dict, request_path: Path) -> dict:
    root = Path(request["root"])
    claim = root / "claim"
    try:
        claim.mkdir(mode=0o700)
    except FileExistsError:
        return observe(request, request_path)
    save(claim / "state.json", {**identity(request), "pid": os.getpid()})
    runtime = root / "runtime"
    try:
        with open(claim / "setup.log", "wb") as log:
            child = subprocess.Popen(["npm", "ci", "--ignore-scripts"], cwd=runtime, stdout=log, stderr=log,
                                     start_new_session=True, env={"PATH": os.environ["PATH"], "HOME": os.environ["HOME"]})
            save(claim / "child.json", {"pid": child.pid, "start": process_start(child.pid), "request": str(request_path)})
            try:
                child.wait(timeout=600)
            except subprocess.TimeoutExpired:
                os.killpg(child.pid, signal.SIGKILL)
                child.wait()
                raise
            if child.returncode:
                raise subprocess.CalledProcessError(child.returncode, "npm ci")
        if (claim / "cancelled").exists():
            raise subprocess.CalledProcessError(143, "cancel")
        # Rebuild only declared pstack script dependencies, from their own lockfiles.
        for manifest in sorted((root / "pstack").rglob("package.json")):
            if "node_modules" in manifest.parts:
                continue
            if not (manifest.parent / "bun.lock").is_file():
                raise OSError("pstack script manifest has no frozen bun lock")
            with open(claim / "skills-install.log", "ab") as log:
                subprocess.run(["bun", "install", "--frozen-lockfile", "--ignore-scripts"], cwd=manifest.parent,
                               stdout=log, stderr=log, check=True, timeout=180)
        if (claim / "cancelled").exists():
            raise subprocess.CalledProcessError(143, "cancel")
        with open(claim / "runner.log", "wb") as log:
            child = subprocess.Popen(["node", "--import", "tsx", ".sandcastle/boat-worker.mts", str(request_path)],
                                     cwd=runtime, stdout=log, stderr=log, start_new_session=True,
                                     env={"PATH": os.environ["PATH"], "HOME": os.environ["HOME"]})
            save(claim / "child.json", {"pid": child.pid, "start": process_start(child.pid), "request": str(request_path)})
            try:
                code = child.wait(timeout=request["timeout"])
            except subprocess.TimeoutExpired:
                os.killpg(child.pid, signal.SIGTERM)
                try:
                    child.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    os.killpg(child.pid, signal.SIGKILL)
                    child.wait()
                code = 143
        if (claim / "receipt.json").exists():
            return json.loads((claim / "receipt.json").read_text())
        failure = {**identity(request), "state": "unknown", "exit_code": code,
                   "reason": "工蜂执行未交业务回执，保留日志不重派"}
    except (OSError, subprocess.SubprocessError):
        failure = {**identity(request), "state": "unknown", "exit_code": None,
                   "reason": "远端准备或监督失败，保留认领不重派"}
    if (claim / "cancelled").exists():
        failure["exit_code"] = 143
        failure["reason"] = "管理者取消；未产生成功业务回执"
    save(claim / "receipt.json", failure)
    return failure


def process_start(pid: int) -> str:
    # Linux starttime is tied to this process instance, unlike a reusable PID.
    return Path(f"/proc/{pid}/stat").read_text().split(")", 1)[1].split()[19]


def cancel(request: dict, request_path: Path) -> dict:
    observation = observe(request, request_path)
    if observation["state"] != "running":
        return observation
    claim = Path(request["root"]) / "claim"
    (claim / "cancelled").touch(exist_ok=True)
    child_path = claim / "child.json"
    if child_path.exists():
        child = json.loads(child_path.read_text())
        pid = child["pid"]
        try:
            if child["request"] == str(request_path) and process_start(pid) == child["start"]:
                os.killpg(pid, signal.SIGTERM)
        except (FileNotFoundError, ProcessLookupError):
            pass
    return observe(request, request_path)


if __name__ == "__main__":
    action, raw_path = sys.argv[1:]
    path = Path(raw_path)
    request = json.loads(path.read_text())
    operations = {"submit": run, "observe": observe, "cancel": cancel}
    result = operations[action](request, path)
    if "record" in result:
        result = {**identity(request), "state": result["state"], "receipt_path": request["root"] + "/claim/receipt.json"}
    print(json.dumps(result, ensure_ascii=False))
