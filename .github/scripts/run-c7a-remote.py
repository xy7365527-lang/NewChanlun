#!/usr/bin/env python3
"""#1406 C7a 的 Linux CI 监督与证据归档；不修改已审驱动或资金预期。"""
from __future__ import annotations

import argparse
import ctypes
from dataclasses import dataclass
import hashlib
import json
import os
from pathlib import Path
import shlex
import shutil
import signal
import stat
import subprocess
import sys
import tarfile
import tempfile
import time
import tomllib

GIB = 1024 ** 3
MIB = 1024 ** 2
# #1406 C5：仅精确已审来源。当前常量为候选占位，最终源审通过后由根重新绑定。
REVIEWED_SOURCE = "b93eee571fb35da3c21a6bd435d5ccef1abbe9bb"
REVIEWED_SOURCE_TREE = "4ceed897108a28ffbd3ded8bcae9ef7ccee9f683"
ALLOWED_CI_DELTA = {".github/workflows/ci.yml", ".github/scripts/run-c7a-remote.py"}
FROZEN = {
    "s_session/tests/tb03c_runtime.py": "aec5abfb1a68354ba5c439128924acc19be96588ba4a26378030d3bd75c4c93b",
    "s_session/tests/fixtures/tb03c_runtime/SEED.json": "5f5931dfba64cec634ca5e4bc8cdef2805aaebdb2bdbe3456ba4711cd4d2ed49",
}
BINARIES = ("e_economic_session", "b_chong_session")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RuntimeError(message)


def save(path: Path, value: object) -> None:
    """证据只写新文件；不覆盖已保存的失败或运行原件。"""
    with path.open("x") as stream:
        json.dump(value, stream, ensure_ascii=False, indent=2)
        stream.write("\n")


def sha(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def git(root: Path, *args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=root, text=True, timeout=15).strip()


def source_identity(root: Path) -> dict[str, str]:
    """绑定精确已审源 HEAD/tree；后续仅允许两个 CI 文件改变。"""
    head = git(root, "rev-parse", "HEAD")
    require(not git(root, "status", "--porcelain"), "需要干净 checkout")
    require(head == os.environ["GITHUB_SHA"], "checkout 与实际 workflow SHA 不同")
    require(git(root, "rev-parse", REVIEWED_SOURCE + "^{tree}") == REVIEWED_SOURCE_TREE,
            "已审源 tree 与冻结资格不同")
    subprocess.run(["git", "merge-base", "--is-ancestor", REVIEWED_SOURCE, head],
                   cwd=root, check=True, timeout=15)
    changed = set(git(root, "diff", "--name-only", REVIEWED_SOURCE, head).splitlines())
    require(changed <= ALLOWED_CI_DELTA, "相对已审 C5 来源存在未批准源码尾差")
    for name, expected in FROZEN.items():
        require(sha(root / name) == expected, "冻结字节改变：" + name)
    seed = json.loads((root / "s_session/tests/fixtures/tb03c_runtime/SEED.json").read_text())
    for binding in seed["bindings"]:
        path = (root / "s_session/tests" / binding["path"]).resolve()
        require(path.is_relative_to(root / "s_session/tests"), "冻结绑定路径越界")
        require(sha(path) == binding["sha256"], "冻结输入改变：" + binding["path"])
    cargo = tomllib.loads((root / "rust/Cargo.toml").read_text())
    for name in BINARIES:
        target = next(item for item in cargo["bin"] if item["name"] == name)
        require(target["required-features"] == ["s_session"], "bin feature 改变：" + name)
    return {"source_sha": head, "source_tree": git(root, "rev-parse", "HEAD^{tree}")}


def memory_available() -> int:
    """取 Linux 实际可用内存，不以总内存替代余量。"""
    for line in Path("/proc/meminfo").read_text().splitlines():
        if line.startswith("MemAvailable:"):
            return int(line.split()[1]) * 1024
    raise RuntimeError("缺少 MemAvailable")


def group_members(group: int) -> list[dict[str, int | str]]:
    """只观察本次新建进程组；不按进程名或陈旧 PID 清理。"""
    members = []
    for path in Path("/proc").iterdir():
        if not path.name.isdigit():
            continue
        try:
            fields = (path / "stat").read_text().rsplit(") ", 1)[1].split()
        except (FileNotFoundError, ProcessLookupError):
            continue
        if int(fields[2]) == group:
            members.append({"pid": int(path.name), "state": fields[0],
                            "rss_bytes": int(fields[21]) * os.sysconf("SC_PAGE_SIZE")})
    return members


def become_subreaper() -> None:
    """父驱动退出后仍由本监督器回收同组后代，避免留下孤儿。"""
    libc = ctypes.CDLL(None, use_errno=True)
    libc.prctl.argtypes = [ctypes.c_int, ctypes.c_ulong, ctypes.c_ulong, ctypes.c_ulong, ctypes.c_ulong]
    libc.prctl.restype = ctypes.c_int
    if libc.prctl(36, 1, 0, 0, 0) != 0:  # PR_SET_CHILD_SUBREAPER
        raise OSError(ctypes.get_errno(), "PR_SET_CHILD_SUBREAPER")


def cleanup_group(proc: subprocess.Popen[bytes]) -> dict[str, object]:
    """leader 尚未 reap，组身份不会被重用；TERM/KILL 后 wait 全部本组子进程。"""
    events, errors = [], []
    forced = False
    try:
        for sig in (signal.SIGTERM, signal.SIGKILL):
            live = [row for row in group_members(proc.pid) if row["state"] != "Z"]
            if not live:
                break
            forced = True
            events.append({"signal": int(sig), "members": live})
            try:
                os.killpg(proc.pid, sig)
            except ProcessLookupError:
                pass
            deadline = time.monotonic() + 3
            while time.monotonic() < deadline:
                if not any(row["state"] != "Z" for row in group_members(proc.pid)):
                    break
                time.sleep(0.05)
        proc.wait(timeout=1)
        deadline = time.monotonic() + 2
        while True:
            try:
                child, code = os.waitpid(-proc.pid, os.WNOHANG)
            except ChildProcessError:
                child = 0
            if child:
                events.append({"reaped_pid": child, "wait_status": code})
                continue
            remaining = group_members(proc.pid)
            if not remaining or time.monotonic() >= deadline:
                break
            time.sleep(0.05)
        require(not remaining, "本次进程组回收后仍有成员：" + repr(remaining))
    except (OSError, RuntimeError, subprocess.SubprocessError) as error:
        errors.append({"type": type(error).__name__, "error": str(error)})
    return {"forced": forced, "events": events, "errors": errors, "complete": not errors}


@dataclass(frozen=True)
class Limits:
    timeout_seconds: int
    max_group_rss_bytes: int
    filesystem: Path
    initial_free_bytes: int
    min_free_bytes: int = GIB
    min_available_memory_bytes: int = 512 * MIB
    max_filesystem_growth_bytes: int = 6 * GIB


def supervise(command: list[str], cwd: Path, env: dict[str, str], prefix: Path,
              limits: Limits) -> dict[str, object]:
    """真实进程退出、主失败及回收失败分别保留；任一失败不得签为成功。"""
    receipt: dict[str, object] = {"argv": command, "cwd": str(cwd), "exit_code": None,
                                "failure": None, "cleanup": None, "status": "FAIL"}
    started = time.monotonic()
    proc = None
    print(json.dumps({"phase": prefix.name, "command": command, "timeout_seconds": limits.timeout_seconds},
                     ensure_ascii=False), flush=True)
    with prefix.with_suffix(".stdout").open("xb") as out, prefix.with_suffix(".stderr").open("xb") as err:
        with prefix.with_suffix(".resources.jsonl").open("x") as resources:
            try:
                proc = subprocess.Popen(command, cwd=cwd, env=env, stdout=out, stderr=err,
                                        start_new_session=True)
                receipt["pid"] = receipt["process_group"] = proc.pid
                while True:
                    elapsed = time.monotonic() - started
                    free = shutil.disk_usage(limits.filesystem).free
                    available = memory_available()
                    rss = sum(int(row["rss_bytes"]) for row in group_members(proc.pid))
                    resources.write(json.dumps({"elapsed_seconds": elapsed, "free_bytes": free,
                                                "available_memory_bytes": available, "group_rss_bytes": rss}) + "\n")
                    resources.flush()
                    require(elapsed <= limits.timeout_seconds, "外层进程期限耗尽")
                    require(free >= limits.min_free_bytes, "磁盘余量低于保留量")
                    require(limits.initial_free_bytes - free <= limits.max_filesystem_growth_bytes,
                            "本 job 文件系统增长超过上界")
                    require(available >= limits.min_available_memory_bytes, "实际可用内存低于保留量")
                    require(rss <= limits.max_group_rss_bytes, "本次进程组 RSS 超过上界")
                    # WNOWAIT 保留 leader 的 PID，直到同组后代均已回收；不提前 poll/reap。
                    if os.waitid(os.P_PID, proc.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT) is not None:
                        break
                    time.sleep(0.25)
            except BaseException as error:
                receipt["failure"] = {"type": type(error).__name__, "error": str(error)}
            finally:
                if proc is not None:
                    receipt["cleanup"] = cleanup_group(proc)
                    receipt["exit_code"] = proc.returncode
    cleanup = receipt["cleanup"]
    if (receipt["failure"] is None and receipt["exit_code"] == 0 and isinstance(cleanup, dict)
            and cleanup["complete"] and not cleanup["forced"]):
        receipt["status"] = "PASS"
    receipt["elapsed_seconds"] = time.monotonic() - started
    receipt["logs"] = {suffix: {"sha256": sha(prefix.with_suffix(suffix)),
                                "bytes": prefix.with_suffix(suffix).stat().st_size}
                       for suffix in (".stdout", ".stderr", ".resources.jsonl")}
    save(prefix.with_suffix(".receipt.json"), receipt)
    return receipt


def verify_run(output: Path, identity: dict[str, str]) -> dict[str, object]:
    """核完整驱动证据，不自行替换其资金 oracle 或归一化逐表比较。"""
    result = json.loads((output / "RESULT.json").read_text())
    expected = {"status": "PASS_C7A", "trajectories": 6, "full_table_double_run": True,
                "all_step_oracle": True, "actual_resolve_kills": 4}
    require(all(result.get(key) == value for key, value in expected.items()), "六轨结果门未通过")
    provenance = json.loads((output / "provenance.json").read_text())
    require(all(provenance["build"][key] == value for key, value in identity.items()), "运行来源不符")
    kills = 0
    for phase in ("normal", "before_commit", "after_commit"):
        previous = None
        for repeat in (1, 2):
            folder = output / f"{phase}-{repeat}"
            trajectory = json.loads((folder / "result.json").read_text())
            require(trajectory["status"] == "PASS_C7A_TRAJECTORY", "轨迹结果未通过")
            require(trajectory["phase"] == (None if phase == "normal" else phase), "轨迹 phase 不符")
            tables = (folder / "final-tables.json").read_bytes()
            if previous is not None:
                require(tables == previous, "同场景十表原始封包不同")
            previous = tables
            require(len(list(folder.glob("step-*-oracle.json"))) == 20, "逐步 oracle 数量不符")
            require(len(list(folder.glob("*-recovery.json"))) == (2 if phase == "normal" else 4),
                    "恢复证据数量不符")
            require((folder / "commands.jsonl").stat().st_size > 0, "缺少消息/回收记录")
            require(not (folder / "cleanup-failure.json").exists(), "存在驱动回收失败")
            if phase != "normal":
                fault = json.loads((folder / "fault.json").read_text())
                require(fault["kill"]["exit_code"] == -signal.SIGKILL, "缺真实 SIGKILL 回执")
                require(fault["marker"]["phase"] == phase and fault["marker"]["message_id"] == "step-014",
                        "故障 marker 不符")
                kills += 1
    require(kills == 4, "缺少四次 Resolve SIGKILL")
    return result


def archive_run(output: Path, artifacts: Path) -> None:
    """每个普通文件完整保留并散列；socket 等非普通条目显式列账，绝不收 target。"""
    entries, special = [], []
    paths = sorted(output.rglob("*")) if output.exists() else []
    archive_path = artifacts / "runtime-evidence.tar.gz"
    for path in paths:
        relative = str(path.relative_to(output))
        mode = path.lstat().st_mode
        if stat.S_ISREG(mode):
            entries.append({"path": relative, "bytes": path.stat().st_size, "sha256": sha(path)})
        elif not stat.S_ISDIR(mode):
            special.append({"path": relative, "mode": mode, "socket": stat.S_ISSOCK(mode)})
    # 压缩前封完整清单；归档中途失败时仍能定位未收入包的原始证据。
    save(artifacts / "archive-inventory.json", {"regular_files": entries, "non_regular_entries": special})
    with tarfile.open(archive_path, "x:gz", dereference=True) as archive:
        for entry in entries:
            archive.add(output / entry["path"], arcname=entry["path"], recursive=False)
    save(artifacts / "archive-manifest.json", {"regular_files": entries, "non_regular_entries": special,
                                              "archive_sha256": sha(archive_path), "archive_bytes": archive_path.stat().st_size})
    require(all(row["socket"] for row in special), "存在非 socket 的特殊文件，须审阅归档边界")


def interrupted(signum: int, _frame: object) -> None:
    raise RuntimeError(f"CI supervision interrupted by signal {signum}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifacts", type=Path, required=True)
    args = parser.parse_args()
    artifacts = args.artifacts.resolve()
    artifacts.mkdir(parents=True, exist_ok=False)
    root = Path(__file__).resolve().parents[2]
    work = Path(tempfile.mkdtemp(prefix="c7a-", dir="/tmp"))
    output = work / "run"
    final: dict[str, object] = {"status": "FAIL", "work": str(work), "runtime_output": str(output),
                               "failure": None, "archive_failure": None}
    try:
        require(sys.platform == "linux", "本监督器只在 Linux CI 执行")
        become_subreaper()
        for sig in (signal.SIGINT, signal.SIGTERM):
            signal.signal(sig, interrupted)
        identity = source_identity(root)
        final.update(identity)
        require(artifacts.stat().st_dev == work.stat().st_dev, "证据和任务目录必须处于同一受监测文件系统")
        free, available = shutil.disk_usage(work).free, memory_available()
        require(free >= 8 * GIB and available >= 3 * GIB, "开跑需实际空闲磁盘 >=8GiB、可用内存 >=3GiB")
        require(len(str(output / "runtime/e.sock").encode()) < 100, "socket 路径过长")
        env = dict(os.environ, CARGO_TARGET_DIR=str(work / "target"), CARGO_INCREMENTAL="0")
        save(artifacts / "configuration.json", {**identity, "work": str(work), "initial_free_bytes": free,
             "initial_available_memory_bytes": available, "build_timeout_seconds": 1200,
             "runtime_timeout_seconds": 900, "build_max_group_rss_bytes": 5 * GIB,
             "runtime_max_group_rss_bytes": 2 * GIB, "min_free_bytes": GIB,
             "min_available_memory_bytes": 512 * MIB, "max_filesystem_growth_bytes": 6 * GIB,
             "driver_seed_timeouts_unchanged": True, "CARGO_TARGET_DIR": env["CARGO_TARGET_DIR"]})
        command = ["cargo", "build", "--locked", "--features", "s_session", "--jobs", "2",
                   "--bin", BINARIES[0], "--bin", BINARIES[1]]
        build = supervise(command, root / "rust", env, artifacts / "build", Limits(1200, 5 * GIB, work, free))
        evidence = {**identity, "command": shlex.join(command), "argv": command,
                    "cwd": str(root / "rust"), "CARGO_TARGET_DIR": env["CARGO_TARGET_DIR"],
                    "exit_code": build["exit_code"], "supervision_status": build["status"],
                    "binaries": {}, "logs": build["logs"]}
        if build["status"] == "PASS":
            evidence["binaries"] = {name: sha(work / "target/debug" / name) for name in BINARIES}
        save(artifacts / "build-evidence.json", evidence)
        require(build["status"] == "PASS", "真实构建/资源/回收门失败")
        require(source_identity(root) == identity, "构建期间源码改变")
        command = [sys.executable, "-B", str(root / "s_session/tests/tb03c_runtime.py"),
                   "--binaries", str(work / "target/debug"), "--build-evidence", str(artifacts / "build-evidence.json"),
                   "--output", str(output)]
        runtime = supervise(command, root, env, artifacts / "runtime", Limits(900, 2 * GIB, work, free))
        require(runtime["status"] == "PASS", "真实六轨进程/资源/回收门失败")
        require(source_identity(root) == identity, "运行期间源码改变")
        require({name: sha(work / "target/debug" / name) for name in BINARIES} == evidence["binaries"],
                "运行期间二进制改变")
        final["driver_result"] = verify_run(output, identity)
        final["status"] = "PASS_C7A_CI"
    except BaseException as error:
        final["failure"] = {"type": type(error).__name__, "error": str(error)}
        final["status"] = "FAIL"
    finally:
        try:
            archive_run(output, artifacts)
        except BaseException as error:
            final["archive_failure"] = {"type": type(error).__name__, "error": str(error)}
            final["status"] = "FAIL"
        save(artifacts / "CI-RESULT.json", final)
    print(json.dumps(final, ensure_ascii=False))
    return 0 if final["status"] == "PASS_C7A_CI" else 1


if __name__ == "__main__":
    raise SystemExit(main())
