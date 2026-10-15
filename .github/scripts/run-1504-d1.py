#!/usr/bin/env python3
"""#1504 仅远端 macOS：受监督的精确原型构建与单次六案例运行。"""
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import platform
import signal
import stat
import subprocess
import sys
import tarfile
import tempfile
import time

# 已通过独立源码/unsafe/C ABI审查的原型；不代表已经实跑通过。
REVIEWED_SOURCE = "73ef47a4f46938b48e07094fb6583d57da2a9d42"
REVIEWED_TREE = "63ca08adeb89879090ae8d6d0ade87b5846e539a"
CI_FILES = {".github/scripts/run-1504-d1.py", ".github/workflows/ci.yml"}
CASES = ["committed", "uncommitted-tail", "bad-tail", "truncated-tail", "pathname-swap", "denials"]
# Handler 只记状态，不在 Popen/回收/写回执的中间抛异常；wait 按 0.1 秒检查。
CANCELLATION = {"count": 0, "first_signal": None}
OWNED_GROUPS: dict[int, subprocess.Popen] = {}
FINAL_EXCLUSIONS = {
    "FILES.json", "FILES.SEAL.json", "RESULT.json", "LATE.json", "CANCELLED.json",
    "inventory.stdout", "inventory.stderr", "inventory.json",
}


class Cancelled(RuntimeError):
    pass


def remember_signal(signum: int, _frame: object) -> None:
    CANCELLATION["count"] += 1
    if CANCELLATION["first_signal"] is None:
        CANCELLATION["first_signal"] = signum


def check_cancelled() -> None:
    if CANCELLATION["count"]:
        raise Cancelled("监督器收到 SIGTERM/SIGINT")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RuntimeError(message)


def save(path: Path, value: object) -> None:
    # 同卷原子且不覆盖已有回执；中断不得暴露半份 PASS。
    temporary = path.with_name("." + path.name + ".tmp")
    created = False
    try:
        with temporary.open("x") as out:
            created = True
            json.dump(value, out, ensure_ascii=False, indent=2)
            out.write("\n")
        os.link(temporary, path)
    finally:
        if created:
            temporary.unlink(missing_ok=True)


def digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def git(root: Path, *args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=root, text=True, timeout=15).strip()


def identity(root: Path) -> dict:
    require(os.environ.get("GITHUB_ACTIONS") == "true", "仅允许 GitHub runner")
    require(platform.system() == "Darwin", "需要 macOS 真实平台")
    require(os.environ.get("GITHUB_REF") == "refs/heads/proto/1504", "原型分支不符")
    require(os.environ.get("GITHUB_EVENT_NAME") == "workflow_dispatch", "仅显式派发")
    head = git(root, "rev-parse", "HEAD")
    require(head == os.environ.get("GITHUB_SHA") == os.environ.get("EXPECTED_SHA"), "执行 SHA 不符")
    require(not git(root, "status", "--porcelain"), "需要干净 checkout")
    require(git(root, "rev-parse", REVIEWED_SOURCE + "^{tree}") == REVIEWED_TREE, "已审来源不符")
    subprocess.run(["git", "merge-base", "--is-ancestor", REVIEWED_SOURCE, head], cwd=root, check=True, timeout=15)
    require(set(git(root, "diff", "--name-only", REVIEWED_SOURCE, head).splitlines()) <= CI_FILES, "存在未审源码尾差")
    return {"head": head, "tree": git(root, "rev-parse", "HEAD^{tree}"),
            "reviewed_source": REVIEWED_SOURCE, "reviewed_tree": REVIEWED_TREE,
            "run_id": os.environ["GITHUB_RUN_ID"], "attempt": os.environ["GITHUB_RUN_ATTEMPT"]}


def group_exists(pgid: int) -> bool:
    try:
        os.killpg(pgid, 0)
        return True
    except ProcessLookupError:
        return False


def reap(proc: subprocess.Popen, deadline: float) -> bool:
    """仅在直系 child 尚未收割时保留组身份；wait/poll 后不再发组信号。"""
    if proc.returncode is not None:
        # success wait 已经释放 PID。旧编号只能只读观察，残留必须 FAIL。
        return not group_exists(proc.pid) and time.monotonic() < deadline
    identity_uncertain = False
    for sig, grace in ((signal.SIGTERM, 2.0), (signal.SIGKILL, 8.0)):
        if time.monotonic() >= deadline:
            break
        # 整段只有本线程持有 Popen，未 poll/wait 的 child PID 不会复用。
        if (proc.returncode is not None or OWNED_GROUPS.get(proc.pid) is not proc
                or signal.getsignal(signal.SIGCHLD) != signal.SIG_DFL):
            identity_uncertain = True
            break
        try:
            if os.getpgid(proc.pid) != proc.pid:
                identity_uncertain = True
                break
            os.killpg(proc.pid, sig)
        except ProcessLookupError:
            identity_uncertain = True
            break
        if sig == signal.SIGKILL:
            break  # 最后一个信号已发；此后才允许 wait 释放身份。
        end = min(deadline, time.monotonic() + grace)
        while time.monotonic() < end:
            if not group_exists(proc.pid):
                break
            time.sleep(min(0.1, max(0.0, end - time.monotonic())))
        else:
            continue
        break  # TERM 后组已消失：直接 wait，不继续向历史编号补 KILL。
    final_end = min(deadline, time.monotonic() + 8.0)
    try:
        proc.wait(timeout=max(0.0, final_end - time.monotonic()))
    except subprocess.TimeoutExpired:
        return False
    # wait 之后不再发信号；只等自有后代退出/被系统收割。
    while time.monotonic() < final_end:
        if not group_exists(proc.pid):
            return not identity_uncertain
        time.sleep(min(0.1, max(0.0, final_end - time.monotonic())))
    return False


def recheck_owned_groups() -> list[dict]:
    """终结时逐项非阻塞复核；不按历史 PGID 补杀，也不添加回收窗口。"""
    rows = []
    for pid, proc in list(OWNED_GROUPS.items()):
        row = {"pid": pid, "leader_reaped": proc.returncode is not None, "group_absent": False}
        try:
            require(pid == proc.pid, "登记 PID 与 Popen 不符")
            if proc.returncode is None:
                try:
                    proc.wait(timeout=0)
                except subprocess.TimeoutExpired:
                    pass
            row["leader_reaped"] = proc.returncode is not None
            row["group_absent"] = not group_exists(pid)
            if row["leader_reaped"] and row["group_absent"]:
                OWNED_GROUPS.pop(pid, None)
        except Exception as exc:
            row["error"] = str(exc)
        rows.append(row)
    return rows


def record_owned_recheck(result: dict, phase: str) -> None:
    if OWNED_GROUPS:
        result.setdefault("owned_group_checks", []).append({"phase": phase, "groups": recheck_owned_groups()})


def supervised(name: str, argv: list[str], cwd: Path, env: dict,
               deadline: float, cleanup_deadline: float, artifacts: Path,
               *, allow_cancel: bool = True) -> dict:
    started = time.monotonic()
    proc = None
    result = {"argv": argv, "pid": None, "exit": None, "timed_out": False,
              "group_reaped": True, "cancelled": False}
    try:
        if allow_cancel:
            check_cancelled()
        require(time.monotonic() < deadline, name + " 启动前期限到达")
        require(signal.getsignal(signal.SIGCHLD) == signal.SIG_DFL, "需要 wait 保留子进程身份")
        with (artifacts / (name + ".stdout")).open("xb") as out, (artifacts / (name + ".stderr")).open("xb") as err:
            # 信号只记录：Popen 返回与登记之间也不会跳过登记/回收。
            proc = subprocess.Popen(argv, cwd=cwd, env=env, stdout=out, stderr=err, start_new_session=True)
            OWNED_GROUPS[proc.pid] = proc
            result["pid"] = proc.pid
            result["group_reaped"] = False
            while True:
                if allow_cancel:
                    check_cancelled()
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    result["timed_out"] = True
                    break
                try:
                    result["exit"] = proc.wait(timeout=min(0.1, remaining))
                    result["timed_out"] = time.monotonic() > deadline
                    break
                except subprocess.TimeoutExpired:
                    continue
    except Exception as exc:
        result["error"] = str(exc)
        result["cancelled"] = isinstance(exc, Cancelled)
    finally:
        if proc is not None:
            try:
                result["group_reaped"] = reap(proc, cleanup_deadline)
            except Exception as exc:
                result["reap_error"] = str(exc)
            if result["group_reaped"]:
                OWNED_GROUPS.pop(proc.pid, None)
        result["elapsed_seconds"] = time.monotonic() - started
        result["signals_observed"] = dict(CANCELLATION)
        save(artifacts / (name + ".json"), result)
    if allow_cancel:
        check_cancelled()
    require(not result.get("error") and not result["timed_out"] and result["exit"] == 0
            and result["group_reaped"], name + " 失败/超时/进程未回收")
    return result


def path_present(path: Path) -> bool:
    # PermissionError/I/O 错误不能被 exists() 折成“没有原件”。
    try:
        path.lstat()
        return True
    except FileNotFoundError:
        return False


def regular_files(directory: Path):
    # scandir 的读目录失败必须传播；rglob 可能静默跳过无权限子目录。
    require(stat.S_ISDIR(directory.lstat().st_mode), "证据根不是普通目录")
    with os.scandir(directory) as entries:
        children = sorted(entries, key=lambda item: item.name)
    for child in children:
        path = Path(child.path)
        mode = child.stat(follow_symlinks=False).st_mode
        if stat.S_ISDIR(mode):
            yield from regular_files(path)
        else:
            require(stat.S_ISREG(mode), "证据包含链接/特殊文件")
            yield path


def preserve(source: Path, target: Path, receipt: Path) -> None:
    """先转移原件再封存；同卷 rename 失败时绝不删除 source。"""
    require(not path_present(target), "原件目的目录已存在")
    present = path_present(source)
    if present:
        require(stat.S_ISDIR(source.lstat().st_mode), "证据目录类型不符")
        require(source.stat().st_dev == target.parent.stat().st_dev, "原件必须同卷原子转移")
        source.rename(target)
    save(receipt, {"evidence_present": present, "source": str(source), "destination": str(target),
                   "preserved": present, "source_absent": not path_present(source)})


def read_small_json(path: Path) -> object:
    # 终结主进程只读固定小收据；大清单及所有内容哈希留在受监督子进程。
    with path.open("rb") as stream:
        content = stream.read(16385)
    require(len(content) <= 16384, "终结收据超出 16 KiB")
    return json.loads(content)


def file_manifest(artifacts: Path) -> None:
    rows = []
    for item in regular_files(artifacts):
        relative = str(item.relative_to(artifacts))
        if relative not in FINAL_EXCLUSIONS:
            rows.append({"path": relative, "bytes": item.stat().st_size, "sha256": digest(item)})
    manifest = artifacts / "FILES.json"
    save(manifest, {"files": rows, "excluded_finalization_files": sorted(FINAL_EXCLUSIONS)})
    save(artifacts / "FILES.SEAL.json", {"path": "FILES.json", "bytes": manifest.stat().st_size,
                                       "sha256": digest(manifest), "file_count": len(rows)})


def finish_evidence(scratch: Path | None, temp: Path, root: Path, env: dict,
                    artifacts: Path, started: float, result: dict) -> None:
    if scratch is None:
        return
    record_owned_recheck(result, "finish-entry")
    if OWNED_GROUPS:
        result.update(verdict="FAIL_D1_COMPONENT", scratch_removed=False,
                      cleanup_error="任务组尚未确认回收，保留原件与 scratch", unreaped_groups=sorted(OWNED_GROUPS))
        return
    preserved = False
    try:
        supervised("preserve", [sys.executable, "-B", str(Path(__file__).resolve()), "--preserve",
                    str(scratch / "evidence"), str(artifacts / "raw-evidence"), str(artifacts / "PRESERVATION.json")],
                   root, env, min(started + 1100, time.monotonic() + 15), started + 1110, artifacts, allow_cancel=False)
        receipt = read_small_json(artifacts / "PRESERVATION.json")
        require(receipt["source_absent"] and not path_present(scratch / "evidence"), "原件仍在 scratch")
        require(receipt["evidence_present"] == (artifacts / "raw-evidence").is_dir(), "原件转移收据不符")
        preserved = True
        supervised("archive", [sys.executable, "-B", str(Path(__file__).resolve()), "--archive",
                    str(artifacts / "raw-evidence"), str(artifacts / "evidence.tar.gz")], root, env,
                   min(started + 1140, time.monotonic() + 60), started + 1150, artifacts, allow_cancel=False)
    except Exception as exc:
        result.update(verdict="FAIL_D1_COMPONENT", preservation_or_archive_error=str(exc))
    record_owned_recheck(result, "before-scratch-cleanup")
    if not preserved or OWNED_GROUPS:
        result.update(verdict="FAIL_D1_COMPONENT", scratch_removed=False,
                      cleanup_error="未确认原件已安全迁出或仍有任务组，保留 scratch")
        return
    try:
        require(scratch.parent == temp and scratch.name.startswith("1504-d1-") and not scratch.is_symlink(), "scratch 所属不符")
        supervised("cleanup", [sys.executable, "-B", "-c", "import shutil,sys; shutil.rmtree(sys.argv[1])", str(scratch)],
                   root, env, min(started + 1158, time.monotonic() + 15), started + 1160, artifacts, allow_cancel=False)
        require(not scratch.exists(), "scratch 仍存在")
        result["scratch_removed"] = True
    except Exception as exc:
        result.update(verdict="FAIL_D1_COMPONENT", cleanup_error=str(exc), scratch_removed=False)


def publish_result(root: Path, env: dict, artifacts: Path, started: float, result: dict) -> int:
    record_owned_recheck(result, "publish-entry")
    try:
        require(not OWNED_GROUPS, "任务组未回收，不能封最终清单")
        result["inventory_stage"] = supervised(
            "inventory", [sys.executable, "-B", str(Path(__file__).resolve()), "--inventory", str(artifacts)],
            root, env, started + 1165, started + 1168, artifacts, allow_cancel=False)
        seal = read_small_json(artifacts / "FILES.SEAL.json")
        require(seal["path"] == "FILES.json" and isinstance(seal["sha256"], str)
                and len(seal["sha256"]) == 64, "最终清单封条不符")
        result["files_manifest"] = seal
    except Exception as exc:
        result.update(verdict="FAIL_D1_COMPONENT", inventory_error=str(exc))
    if CANCELLATION["count"]:
        result.update(verdict="FAIL_D1_COMPONENT", cancellation=dict(CANCELLATION))
    record_owned_recheck(result, "publish-final")
    result["unreaped_groups"] = sorted(OWNED_GROUPS)
    result["elapsed_seconds"] = time.monotonic() - started
    if time.monotonic() >= started + 1170:
        result.update(verdict="FAIL_D1_COMPONENT", receipt_late=True)
    save(artifacts / "RESULT.json", result)
    # PASS 的必要条件包含进程 exit=0、无 LATE/CANCELLED、精确来源与完整证据。
    # RESULT 不进入 FILES，反向引用清单 SHA，避免自引用及主进程重新哈希大文件。
    late = time.monotonic() >= started + 1170
    cancelled = bool(CANCELLATION["count"])
    if late:
        save(artifacts / "LATE.json", {"elapsed_seconds": time.monotonic() - started, "accepted": False})
    if cancelled:
        save(artifacts / "CANCELLED.json", {"signals": dict(CANCELLATION), "accepted": False})
    return 0 if (result["verdict"] == "PASS_D1_COMPONENT_ONLY" and not late and not cancelled
                 and not CANCELLATION["count"] and not OWNED_GROUPS
                 and time.monotonic() < started + 1170) else 1


def archive(source: Path, target: Path) -> None:
    """子进程内封存，因此大文件或异常 I/O 同样受外层截止约束。"""
    rows = []
    with tarfile.open(target, "x:gz") as tar:
        if path_present(source):
            for item in regular_files(source):
                rows.append({"path": str(item.relative_to(source)), "bytes": item.stat().st_size, "sha256": digest(item)})
                tar.add(item, arcname=str(item.relative_to(source)), recursive=False)
    save(target.with_suffix(".manifest.json"), {"files": rows, "archive_sha256": digest(target)})


def main() -> int:
    started = time.monotonic()
    signal.signal(signal.SIGTERM, remember_signal)
    signal.signal(signal.SIGINT, remember_signal)
    root = Path(os.environ["GITHUB_WORKSPACE"]).resolve()
    temp = Path(os.environ["RUNNER_TEMP"]).resolve()
    artifacts = temp / "1504-d1-artifacts"
    artifacts.mkdir(mode=0o700)
    scratch = None
    result = {"verdict": "FAIL_D1_COMPONENT", "local_execution": False}
    env = os.environ.copy()
    try:
        check_cancelled()
        source = identity(root)
        save(artifacts / "SOURCE.json", source)
        scratch = Path(tempfile.mkdtemp(prefix="1504-d1-", dir=temp))
        env["CARGO_TARGET_DIR"] = str(scratch / "target")
        save(artifacts / "ENVIRONMENT.json", {"platform": platform.platform(), "disk": os.statvfs(temp).f_bavail * os.statvfs(temp).f_frsize,
             "build_seconds": 900, "suite_seconds": 120, "active_seconds": 1020,
             "active_reap_end_seconds": 1080, "archive_end_seconds": 1140,
             "scratch_cleanup_end_seconds": 1158, "inventory_end_seconds": 1165,
             "inventory_reap_end_seconds": 1168, "receipt_end_seconds": 1170})
        supervised("build", ["cargo", "build", "--locked", "--no-default-features", "--features", "prototype_1504",
                    "--bin", "p1504_fd_wal", "--message-format=json-render-diagnostics"],
                   root / "rust", env, started + 900, started + 1080, artifacts)
        builds = [json.loads(line) for line in (artifacts / "build.stdout").read_text().splitlines() if line.startswith("{")]
        binaries = [Path(row["executable"]) for row in builds if row.get("reason") == "compiler-artifact"
                    and row.get("target", {}).get("name") == "p1504_fd_wal" and row.get("executable")]
        require(len(binaries) == 1, "需要唯一 cargo JSON 可执行产物")
        binary = binaries[0].resolve()
        require(binary.is_relative_to(scratch / "target") and binary.is_file(), "构建产物越界")
        binary_hash = digest(binary)
        save(artifacts / "BINARY.json", {"path": str(binary), "sha256": binary_hash, "source": source})
        supervised("suite", [str(binary), "suite", str(scratch / "evidence")], root, env,
                   min(started + 1020, time.monotonic() + 120), started + 1080, artifacts)
        require(digest(binary) == binary_hash and identity(root) == source, "执行前后源码/产物变化")
        report = json.loads((scratch / "evidence/RESULT.json").read_text())
        require(report["verdict"] == "PASS_D1_COMPONENT_ONLY", "原型未通过")
        require(report["cases"] == [{"name": name, "status": "PASS"} for name in CASES], "六案例不完整")
        require(report["cleanup"] == {"fixtures_removed": True, "registrations_removed": True}, "原型清理未完成")
        require(report["default_unchanged"] is True and report["production_v4"] == "BLOCK_NOT_MERGEABLE", "越出组件边界")
        result["verdict"] = "PASS_D1_COMPONENT_ONLY"
    except Exception as exc:
        result["error"] = str(exc)
    finally:
        finish_evidence(scratch, temp, root, env, artifacts, started, result)
    return publish_result(root, env, artifacts, started, result)


if __name__ == "__main__":
    if len(sys.argv) == 4 and sys.argv[1] == "--archive":
        archive(Path(sys.argv[2]), Path(sys.argv[3]))
    elif len(sys.argv) == 5 and sys.argv[1] == "--preserve":
        preserve(Path(sys.argv[2]), Path(sys.argv[3]), Path(sys.argv[4]))
    elif len(sys.argv) == 3 and sys.argv[1] == "--inventory":
        file_manifest(Path(sys.argv[2]))
    else:
        raise SystemExit(main())
