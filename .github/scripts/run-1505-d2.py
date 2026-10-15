#!/usr/bin/env python3
"""#1505 仅远端 macOS：D2 精确原型构建、单次 suite 与受监督封存。"""
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
REVIEWED_SOURCE = "e1d7e0b6bffb78f757e59cdfe3e35f3618e6790a"
REVIEWED_TREE = "44d075dbd7eb99fbe388ee082acbca6beb9219cb"
CI_FILES = {".github/scripts/run-1505-d2.py", ".github/workflows/prototype-1505-d2.yml"}
# 已按源作者 CI_CONTRACT 对齐；源码 SHA/tree 仍须由主控绑定并做尾差独审。
CONTRACT_READY = True
CASES = [
    "current-clean", "current-cold", "current-hot", "old-clean", "old-cold", "old-hot",
    "unknown-clean", "unknown-cold", "unknown-hot", "original-longer", "empty-journal", "post-reader-kill",
]
NEGATIVES = ["callbacks", "sql-denials", "read-io", "read-short", "read-panic",
             "late-exclusive", "late-exclusive-clean", "close-error", "allocator-failure", "arena-oom", "path-swap"]
SUPERS = ["absolute", "relative", "embedded-nul", "length-512", "length-513", "empty",
          "bad-checksum", "zero-first-name", "length-beyond-file", "bad-magic"]
TAILS = ["truncate-seven", "bad-last-byte"]
SQLITE_SOURCE_ID = "2026-06-03 19:12:13 d6e03d8c777cfa2d35e3b60d8ec3e0187f3e9f99d8e2ee9cac695fd6fcdf1a24"
D2_LIMITS = {
    "max_source_main_bytes": 2097152,
    "max_source_journal_bytes": 4194304,
    "overlay_payload_capacity_bytes": 8388608,
    "overlay_metadata_capacity_bytes": 1048576,
    "evidence_capacity_bytes": 4194304,
    "sqlite_heap_limit_bytes": 8388608,
}
# Handler 只记状态，不在 Popen/回收/写回执的中间抛异常；wait 按 0.1 秒检查。
CANCELLATION = {"count": 0, "first_signal": None}
OWNED_GROUPS: dict[int, subprocess.Popen] = {}
FINAL_EXCLUSIONS = {
    "FILES.json", "FILES.SEAL.json", "RESULT.json", "LATE.json", "CANCELLED.json",
    "inventory.stdout", "inventory.stderr", "inventory.json",
}


class Cancelled(RuntimeError):
    pass


def clock_ns() -> int:
    """与 Rust libc CLOCK_MONOTONIC 使用同一显式时钟域。"""
    return time.clock_gettime_ns(time.CLOCK_MONOTONIC)


def clock_seconds() -> float:
    return clock_ns() / 1_000_000_000


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
    require(os.environ.get("GITHUB_REF") == "refs/heads/proto/1505", "原型分支不符")
    require(os.environ.get("GITHUB_EVENT_NAME") == "push", "仅精确原型分支 push")
    require(CONTRACT_READY and len(REVIEWED_SOURCE) == 40 and len(REVIEWED_TREE) == 40
            and all(c in "0123456789abcdef" for c in REVIEWED_SOURCE + REVIEWED_TREE),
            "D2 源码/输出契约尚未独审绑定，禁止运行")
    head = git(root, "rev-parse", "HEAD")
    require(head == os.environ.get("GITHUB_SHA") == os.environ.get("EXPECTED_SHA"), "执行 SHA 不符")
    require(not git(root, "status", "--porcelain"), "需要干净 checkout")
    require(git(root, "rev-parse", REVIEWED_SOURCE + "^{tree}") == REVIEWED_TREE, "已审来源不符")
    subprocess.run(["git", "merge-base", "--is-ancestor", REVIEWED_SOURCE, head], cwd=root, check=True, timeout=15)
    require(set(git(root, "diff", "--name-only", REVIEWED_SOURCE, head).splitlines()) <= CI_FILES, "存在未审源码尾差")
    return {"head": head, "tree": git(root, "rev-parse", "HEAD^{tree}"),
            "reviewed_source": REVIEWED_SOURCE, "reviewed_tree": REVIEWED_TREE,
            "run_id": os.environ["GITHUB_RUN_ID"], "attempt": os.environ["GITHUB_RUN_ATTEMPT"],
            "runner_sha256": digest(root / ".github/scripts/run-1505-d2.py"),
            "workflow_sha256": digest(root / ".github/workflows/prototype-1505-d2.yml")}


def group_exists(pgid: int) -> bool:
    try:
        os.killpg(pgid, 0)
        return True
    except ProcessLookupError:
        return False


def reap(proc: subprocess.Popen, deadline: float) -> dict:
    """未 reap 的直属 leader 固定 PID/PGID；已回收或消失后绝不补发信号。

    SIGCHLD 必须为默认，单线程且无其他 waitpid 回收者。TERM 后不能 poll/wait，
    保留 leader（包括 zombie）直到最后一次 group signal，防止 PGID 复用。
    正常 wait 已回收的 leader 只查组消失；残留归属不明即失败保留原件。
    """
    receipt = {"group_reaped": False, "leader_reaped": proc.returncode is not None,
               "signals_sent": [], "ownership": "unreaped_direct_child"}
    if proc.returncode is not None:
        receipt["ownership"] = "leader_already_reaped_no_more_signals"
        receipt["group_reaped"] = not group_exists(proc.pid)
        return receipt
    disappeared = False
    for sig, grace in [(signal.SIGTERM, 2.0), (signal.SIGKILL, 8.0)]:
        if disappeared or clock_seconds() >= deadline:
            break
        # 不调用 poll/wait；在这里 returncode 为空的 leader 仍由本监督器唯一持有。
        try:
            os.killpg(proc.pid, sig)
            receipt["signals_sent"].append(int(sig))
        except ProcessLookupError:
            disappeared = True
            break
        except Exception as exc:
            receipt["signal_error"] = str(exc)
            break  # 归属/权限异常后不再发送下一信号；仍尝试 wait 直属子进程。
        if sig == signal.SIGKILL:
            break
        end = min(deadline, clock_seconds() + grace)
        while clock_seconds() < end:
            try:
                if not group_exists(proc.pid):
                    disappeared = True
                    break
            except Exception as exc:
                receipt["signal_error"] = "group observation: " + str(exc)
                break
            time.sleep(min(0.1, max(0.0, end - clock_seconds())))
        if "signal_error" in receipt:
            break
    remaining = deadline - clock_seconds()
    if remaining > 0:
        try:
            proc.wait(timeout=remaining)
        except subprocess.TimeoutExpired:
            receipt["ownership"] = "unreaped_direct_child_cleanup_timeout"
            return receipt
    receipt["leader_reaped"] = proc.returncode is not None
    # 此后只能观测，不能重新对历史 PGID 发送任何信号。
    while receipt["leader_reaped"] and not disappeared and clock_seconds() < deadline:
        if not group_exists(proc.pid):
            disappeared = True
            break
        time.sleep(min(0.1, max(0.0, deadline - clock_seconds())))
    receipt["group_reaped"] = receipt["leader_reaped"] and disappeared and "signal_error" not in receipt
    return receipt


def supervised(name: str, argv: list[str], cwd: Path, env: dict,
               deadline: float, cleanup_deadline: float, artifacts: Path,
               *, allow_cancel: bool = True) -> dict:
    started = clock_seconds()
    proc = None
    result = {"argv": argv, "pid": None, "exit": None, "timed_out": False,
              "group_reaped": True, "cancelled": False}
    try:
        if allow_cancel:
            check_cancelled()
        require(clock_seconds() < deadline, name + " 启动前期限到达")
        with (artifacts / (name + ".stdout")).open("xb") as out, (artifacts / (name + ".stderr")).open("xb") as err:
            # 信号只记录：Popen 返回与登记之间也不会跳过登记/回收。
            proc = subprocess.Popen(argv, cwd=cwd, env=env, stdout=out, stderr=err, start_new_session=True)
            OWNED_GROUPS[proc.pid] = proc
            result["pid"] = proc.pid
            result["group_reaped"] = False
            while True:
                if allow_cancel:
                    check_cancelled()
                remaining = deadline - clock_seconds()
                if remaining <= 0:
                    result["timed_out"] = True
                    break
                try:
                    result["exit"] = proc.wait(timeout=min(0.1, remaining))
                    result["timed_out"] = clock_seconds() > deadline
                    break
                except subprocess.TimeoutExpired:
                    continue
    except Exception as exc:
        result["error"] = str(exc)
        result["cancelled"] = isinstance(exc, Cancelled)
    finally:
        if proc is not None:
            try:
                cleanup = reap(proc, cleanup_deadline)
                result["cleanup"] = cleanup
                result["group_reaped"] = cleanup["group_reaped"]
            except Exception as exc:
                result["reap_error"] = str(exc)
            if result["group_reaped"]:
                OWNED_GROUPS.pop(proc.pid, None)
        result["elapsed_seconds"] = clock_seconds() - started
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
    if OWNED_GROUPS:
        result.update(verdict="FAIL_D2_COMPONENT", scratch_removed=False,
                      cleanup_error="任务组尚未确认回收，保留原件与 scratch", unreaped_groups=sorted(OWNED_GROUPS))
        return
    preserved = False
    try:
        supervised("preserve", [sys.executable, "-B", str(Path(__file__).resolve()), "--preserve",
                    str(scratch / "evidence"), str(artifacts / "raw-evidence"), str(artifacts / "PRESERVATION.json")],
                   root, env, min(started + 1100, clock_seconds() + 15), started + 1110, artifacts, allow_cancel=False)
        receipt = read_small_json(artifacts / "PRESERVATION.json")
        require(receipt["source_absent"] and not path_present(scratch / "evidence"), "原件仍在 scratch")
        require(receipt["evidence_present"] == (artifacts / "raw-evidence").is_dir(), "原件转移收据不符")
        preserved = True
        supervised("archive", [sys.executable, "-B", str(Path(__file__).resolve()), "--archive",
                    str(artifacts / "raw-evidence"), str(artifacts / "evidence.tar.gz")], root, env,
                   min(started + 1140, clock_seconds() + 60), started + 1150, artifacts, allow_cancel=False)
    except Exception as exc:
        result.update(verdict="FAIL_D2_COMPONENT", preservation_or_archive_error=str(exc))
    if not preserved or OWNED_GROUPS:
        result.update(verdict="FAIL_D2_COMPONENT", scratch_removed=False,
                      cleanup_error="未确认原件已安全迁出或仍有任务组，保留 scratch")
        return
    try:
        require(scratch.parent == temp and scratch.name.startswith("1505-d2-") and not scratch.is_symlink(), "scratch 所属不符")
        supervised("cleanup", [sys.executable, "-B", "-c", "import shutil,sys; shutil.rmtree(sys.argv[1])", str(scratch)],
                   root, env, min(started + 1158, clock_seconds() + 15), started + 1160, artifacts, allow_cancel=False)
        require(not scratch.exists(), "scratch 仍存在")
        result["scratch_removed"] = True
    except Exception as exc:
        result.update(verdict="FAIL_D2_COMPONENT", cleanup_error=str(exc), scratch_removed=False)


def publish_result(root: Path, env: dict, artifacts: Path, started: float, result: dict) -> int:
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
        result.update(verdict="FAIL_D2_COMPONENT", inventory_error=str(exc))
    if CANCELLATION["count"]:
        result.update(verdict="FAIL_D2_COMPONENT", cancellation=dict(CANCELLATION))
    result["unreaped_groups"] = sorted(OWNED_GROUPS)
    result["elapsed_seconds"] = clock_seconds() - started
    if clock_seconds() >= started + 1170:
        result.update(verdict="FAIL_D2_COMPONENT", receipt_late=True)
    try:
        save(artifacts / "RESULT.json", result)
    except Exception as exc:
        print("final receipt unavailable: " + str(exc), file=sys.stderr)
        return 1
    # PASS 的必要条件包含进程 exit=0、无 LATE/CANCELLED、精确来源与完整证据。
    # RESULT 不进入 FILES，反向引用清单 SHA，避免自引用及主进程重新哈希大文件。
    late = clock_seconds() >= started + 1170
    cancelled = bool(CANCELLATION["count"])
    try:
        if late:
            save(artifacts / "LATE.json", {"elapsed_seconds": clock_seconds() - started, "accepted": False})
        if cancelled:
            save(artifacts / "CANCELLED.json", {"signals": dict(CANCELLATION), "accepted": False})
    except Exception as exc:
        print("late/cancel receipt unavailable: " + str(exc), file=sys.stderr)
        return 1
    return 0 if (result["verdict"] == "PASS_D2_COMPONENT_ONLY" and not late and not cancelled
                 and not CANCELLATION["count"] and not OWNED_GROUPS
                 and clock_seconds() < started + 1170) else 1


def archive(source: Path, target: Path) -> None:
    """子进程内封存，因此大文件或异常 I/O 同样受外层截止约束。"""
    rows = []
    with tarfile.open(target, "x:gz") as tar:
        if path_present(source):
            for item in regular_files(source):
                rows.append({"path": str(item.relative_to(source)), "bytes": item.stat().st_size, "sha256": digest(item)})
                tar.add(item, arcname=str(item.relative_to(source)), recursive=False)
    save(target.with_suffix(".manifest.json"), {"files": rows, "archive_sha256": digest(target)})


def verify_build(log: Path, target: Path, root: Path, source_file: Path, receipt: Path) -> None:
    binaries = []
    with log.open() as stream:
        for line in stream:
            if not line.startswith("{"):
                continue
            row = json.loads(line)
            if (row.get("reason") == "compiler-artifact"
                    and row.get("target", {}).get("name") == "p1505_fd_journal"
                    and row.get("executable")):
                require(row.get("fresh") is False, "拒绝复用已有可执行产物")
                binaries.append(Path(row["executable"]))
    require(len(binaries) == 1, "需要唯一 cargo JSON 可执行产物")
    binary = binaries[0].resolve()
    require(binary.is_relative_to(target.resolve()) and binary.is_file(), "构建产物越界")
    source = read_small_json(source_file)
    require(identity(root) == source, "构建后源码身份变化")
    save(receipt, {"path": str(binary), "sha256": digest(binary), "source": source})


def evidence_json(evidence: Path, name: str) -> object:
    path = evidence / name
    require(stat.S_ISREG(path.lstat().st_mode), "证据非普通文件: " + name)
    return json.loads(path.read_text())


def names_exact(rows: object, names: list[str], key: str = "name") -> None:
    require(isinstance(rows, list) and [row[key] for row in rows] == names, "案例集合/顺序不符: " + key)


def check_process(row: dict, suite_deadline: int) -> None:
    require(row["success"] is True and row["reaped"] is True and row["exit_code"] == 0
            and row["signal"] is None and row["pid"] > 0 and row["pgid"] > 0,
            "子进程真实退出/回收不符")
    require(row["started_ns"] <= row["ended_ns"] < suite_deadline, "子进程回执迟到")


def check_kill(row: dict, mode: str, suite_deadline: int) -> None:
    require(row["mode"] == mode and row["signal_sent"] == 9 and row["wait_signal"] == 9
            and row["wait_exit_code"] is None and row["reaped"] is True,
            "真实 SIGKILL/wait 回执不符")
    require(row["pid"] > 0 and row["pgid"] > 0 and row["sent_ns"] <= row["reaped_ns"] < suite_deadline,
            "SIGKILL 身份/时序不符")
    ready = row["ready"]
    require(ready["ready"] is True, "缺真实 READY")
    if mode == "writer":
        observed = row["prekill_observation"]
        require(ready["source_id"] == SQLITE_SOURCE_ID and ready["uncommitted"] is True
                and len(ready["aligned_header_magic_offsets"]) >= 2
                and ready["main_before_hash"] != observed["main_sha256"]
                and ready["main_uncommitted_hash"] == observed["main_sha256"]
                and ready["main_uncommitted_len"] == observed["main_len"]
                and ready["journal_len"] == observed["journal_len"] > 512, "writer spill 实证不符")
    else:
        require(ready["query_ok"] is True and ready["overlay_write_calls"] > 0
                and row["prekill_observation"]["reader_recovery_observed"] is True,
                "reader 未在真实恢复后被 kill")


def check_reader(reader: dict, hot: bool) -> None:
    life = reader["lifecycle"]
    report = life["report"]
    require(life["unregistered"] is True and life["source_fds_closed"] is True
            and report["active_files"] == 0 and report["source_unchanged"] is True
            and report["trace_overflow"] is False, "reader 生命周期/原件不符")
    require(reader["owned_allocation_ledger"]["actual_owned_layout_current_bytes"] == 0,
            "owned allocation 未释放")
    allocator = reader["allocator"]
    require(allocator["after_shutdown_xsize_bytes"] == 0
            and allocator["before_shutdown"]["configured_limit_bytes"] == D2_LIMITS["sqlite_heap_limit_bytes"]
            and allocator["before_shutdown"]["observed_xsize_within_configured_limit"] is True,
            "SQLite heap 账不符")
    armed = False
    writes = 0
    for event in report["events"]:
        if event["op"] == "exclusive-armed":
            armed = True
        if event["op"] == "read" and not armed:
            require(event["kind"] == 1 and event["a"] == 0 and event["b"] == 100,
                    "EXCLUSIVE 之前读取 schema/数据")
        if event["op"] in {"write-overlay", "truncate-overlay", "delete-virtual-journal"}:
            require(armed, "EXCLUSIVE 之前恢复")
            writes += event["op"] == "write-overlay" and event["rc"] == 0
    require(not hot or writes > 0, "hot 正控缺实际 overlay write")


def check_boundary(evidence: Path, prefix: str, row: dict, *, compare_bytes: bool) -> None:
    oracle = evidence_json(evidence, prefix + "-oracle.json")
    reader = evidence_json(evidence, prefix + "-reader.json")
    require(reader["source_id"] == oracle["source_id"] == SQLITE_SOURCE_ID
            and reader.get("registration_or_harness_error") is not True
            and type(oracle["ok"]) is bool and reader["ok"] is oracle["ok"]
            and row["sqlite_succeeded"] is oracle["ok"], "边界 SQLite 状态/身份不符")
    if oracle["ok"]:
        require(reader["data"] == oracle["data"], "边界完整 data 不等")
        if compare_bytes:
            require(reader["virtual_main_sha256"] == oracle["main_sha256"]
                    and reader["virtual_main_len"] == oracle["main_len"], "边界虚拟字节不等")
    else:
        require(type(reader["extended_code"]) is int and reader["extended_code"] == oracle["extended_code"],
                "边界双失败没有相同明确 SQLite 扩展码")


def check_late_exclusive(value: dict, mode: str, proof: dict) -> None:
    """从原 trace 重建两处不同拒绝的证明，不把任意 SQLite 错误算命中。"""
    require(mode in {"late-exclusive", "late-exclusive-clean"}, "未知 exclusive 负控")
    life = value["lifecycle"]
    report = life["report"]
    before = value["before_source"]
    events = report["events"]
    require(value["ok"] is False and all(value.get(key) is not True for key in
            ("registration_or_harness_error", "connect_error", "close_error")), "未触达查询拒绝")
    require(life["unregistered"] is True and life["source_fds_closed"] is True
            and type(report["active_files"]) is int and report["active_files"] == 0
            and report["source_unchanged"] is True and report["trace_overflow"] is False
            and isinstance(before, dict) and "main.sqlite" in before
            and before == value["after_source"] == value["whole_reader_source_before"]
            == value["whole_reader_source_after"], "exclusive 负控来源/生命周期不符")
    require(value["owned_allocation_ledger"]["actual_owned_layout_current_bytes"] == 0
            and value["allocator"]["after_shutdown_xsize_bytes"] == 0, "exclusive 负控账未释放")
    require(isinstance(events, list) and all(
            event["exclusive_armed"] is False and event["op"] not in
            {"exclusive-armed", "write-overlay", "truncate-overlay", "delete-virtual-journal"}
            and all(type(event[key]) is int for key in ("kind", "a", "b", "rc"))
            for event in events), "exclusive 负控提前 arm/改写或 trace 字段类型错误")
    require(all(type(report["overlay"][key]) is int and report["overlay"][key] == 0
            for key in ("write_calls", "truncate_calls", "delete_calls")), "exclusive 负控发生虚拟写入")
    reads = [event for event in events if event["op"] == "read"]
    denied = [event for event in events if event["op"] == "open-denied"]
    require(type(value["extended_code"]) is int, "缺 SQLite 扩展码")
    if mode == "late-exclusive":
        require(set(before) == {"main.sqlite", "main.sqlite-journal"}
                and type(before["main.sqlite-journal"]["len"]) is int
                and before["main.sqlite-journal"]["len"] > 512, "hot 负控失去热 journal")
        require(value["extended_code"] == 14 and bool(reads)
                and all((e["kind"], e["a"], e["b"], e["rc"]) == (1, 0, 100, 0)
                        for e in reads), "hot 未在非头读取之前拒绝")
        require([(e["kind"], e["a"], e["b"], e["rc"]) for e in denied]
                == [(0, 2049, 0, 14), (0, 2050, 0, 14)], "hot journal 打开拒绝顺序不符")
        rejection = "HOT_JOURNAL_OPEN_CANTOPEN"
    else:
        require(set(before) == {"main.sqlite"} and value["extended_code"] == 266
                and not denied and any(e["kind"] == 1 and e["rc"] == 266
                and (e["a"], e["b"]) != (0, 100) for e in reads), "clean 非头读取拒绝未命中")
        require(all((e["kind"], e["a"], e["b"]) == (1, 0, 100)
                    for e in reads if e["rc"] == 0), "clean 放过非头读取")
        rejection = "CLEAN_MAIN_READ_IOERR_READ"
    rebuilt = {"mode": mode, "rejection": rejection, "extended_code": value["extended_code"],
               "read_events": reads, "open_denied_events": denied, "source_before": before,
               "source_after": value["after_source"], "no_arm": True, "no_overlay_mutation": True,
               "source_unchanged": True, "active_files": 0, "unregistered": True,
               "source_fds_closed": True, "all_expected": True}
    require(json.dumps(proof, sort_keys=True) == json.dumps(rebuilt, sort_keys=True),
            "exclusive 具名证明与原件不符")


def verify_suite(evidence: Path, binary_receipt: Path, root: Path,
                 source_file: Path, receipt: Path) -> None:
    """仅在受监督 worker 中核原件和契约；不执行 SQLite，不把边界双失败记正控。"""
    require(CONTRACT_READY, "D2 suite 契约尚未冻结")
    for _ in regular_files(evidence):
        pass  # 包含所有子目录，拒绝符号链接/特殊文件与不可读目录。
    binary = read_small_json(binary_receipt)
    require(digest(Path(binary["path"])) == binary["sha256"], "执行前后产物变化")
    require(identity(root) == read_small_json(source_file), "执行前后源码变化")
    common_deadline = int(os.environ["P1505_DEADLINE_NS"])
    suite_deadline = int(os.environ["P1505_SUITE_DEADLINE_NS"])
    report = evidence_json(evidence, "RESULT.json")
    require(report["verdict"] == "PASS_D2_COMPONENT_ONLY", "D2 原型未通过")
    require(report["absolute_deadline_ns"] == common_deadline
            and report["suite_deadline_ns"] == suite_deadline <= common_deadline
            and report["started_ns"] <= report["finished_ns"] < suite_deadline, "共同绝对截止不符")
    require(report["production_v4"] == "BLOCK_NOT_MERGEABLE"
            and report["supervisor_sigterm_sigint"] == "NOT_RUN"
            and report["source_rw_opened"] is False
            and report["parent_process_never_initialized_sqlite"] is True, "越出组件边界")
    require(report["limits"] == evidence_json(evidence, "limits.json") == D2_LIMITS, "显式限制漂移")
    names_exact(report["cases"], CASES)
    names_exact(report["negative_cases"], NEGATIVES, "mode")
    names_exact(report["super_journal"], SUPERS)
    names_exact(report["tail_boundaries"], TAILS)
    for case in report["cases"]:
        name = case["name"]
        require(case == evidence_json(evidence, name + "-case.json") and case["all_expected"] is True
                and case["rows"] == 48 and case["source_before"] == case["source_after"], "正控原件摘要不符")
        reader = evidence_json(evidence, name + "-reader.json")
        oracle = evidence_json(evidence, name + "-oracle.json")
        require(reader["source_id"] == oracle["source_id"] == SQLITE_SOURCE_ID, "SQLite source-id 漂移")
        require(reader["ok"] is True and oracle["ok"] is True and reader["data"] == oracle["data"]
                and reader["virtual_main_sha256"] == oracle["main_sha256"]
                and reader["virtual_main_len"] == oracle["main_len"], "正控完整 data/虚拟字节不等")
        require(reader["accepted_rule"] is (not name.startswith(("old-", "unknown-"))), "规则准入不符")
        check_reader(reader, name.endswith("-hot") or name in {"original-longer", "post-reader-kill"})
        for mode in ("oracle", "reader"):
            process = evidence_json(evidence, name + "-" + mode + "-process.process.json")
            require(case[mode + "_process"] == process, "进程摘要与原件不符")
            check_process(process, suite_deadline)
        for copied in case["copy"]["files"]:
            require((copied["source_dev"], copied["source_ino"]) != (copied["copy_dev"], copied["copy_ino"]),
                    "oracle 与 source 同 inode")
        if name.endswith("-hot"):
            check_kill(evidence_json(evidence, name + "-writer.kill.json"), "writer", suite_deadline)
    for item in report["negative_cases"]:
        mode = item["mode"]
        value = evidence_json(evidence, "fault-" + mode + ".json")
        expected = mode in {"callbacks", "sql-denials", "allocator-failure", "path-swap"}
        require(item["all_expected"] is True and item["ok"] is expected and value["ok"] is expected
                and value["source_id"] == SQLITE_SOURCE_ID
                and value["allocator"]["after_shutdown_xsize_bytes"] == 0, "负控未命中/基础设施失败")
        check_process(evidence_json(evidence, "fault-" + mode + "-reader.process.json"), suite_deadline)
        if mode == "callbacks":
            checks = value["callbacks"]
            require(checks["all_expected"] is True and checks["actual_os_eintr"] == "NOT_RUN", "callback 名分不符")
            by_name = {row["name"]: row for row in checks["checks"]}
            require({"thread-owner-and-reentry", "unknown-capability", "delete-live-handle-and-reopen",
                     "zero-null-huge-bounds"} <= by_name.keys(), "callback 边界缺件")
            require(by_name["thread-owner-and-reentry"]["wrong_thread_rc"] == 21
                    and by_name["thread-owner-and-reentry"]["reentry_rc"] == 21, "线程/重入拒绝不符")
        elif mode == "sql-denials":
            proof = value["statement_lifecycle"]
            expected_codes = {"prepare_rc": 0, "step_rc": 100, "close_rc": 5,
                              "authorizer_restored_rc": 0, "finalize_rc": 0}
            require(all(type(proof[key]) is int and proof[key] == code
                        for key, code in expected_codes.items())
                    and all(proof[key] is True for key in ("close_refused",
                            "original_connection_retained", "owner_registration_retained"))
                    and type(proof["active_files_after_refusal"]) is int
                    and proof["active_files_after_refusal"] > 0,
                    "活 statement 关闭拒绝/恢复 owner 证据不符")
            require(type(value["lifecycle"]["report"]["sql_denials"]) is int
                    and value["lifecycle"]["report"]["sql_denials"] >= 7,
                    "SQL 拒绝负控未实抵达")
            check_reader(value, hot=True)
        elif mode in {"late-exclusive", "late-exclusive-clean"}:
            check_late_exclusive(value, mode, evidence_json(evidence, "fault-" + mode + "-case.json"))
        elif mode == "allocator-failure":
            probe = value["allocation_probe"]
            require(all(probe[key] is True for key in ("realloc_old_bytes_preserved", "actual_malloc_failed",
                    "actual_hard_heap_rejection")), "allocator 负控未实抵达")
        elif mode == "arena-oom":
            require(value["owned_allocation_ledger"]["failures"] > 0
                    and value["vfs_counters"]["registrations"] == 0, "arena OOM 未在注册前拒绝")
    require(report["super_journal"] == evidence_json(evidence, "super-journal-cases.json"), "super 原件不符")
    sentinel_hash = digest(evidence / "external-super-sentinel")
    for index, item in enumerate(report["super_journal"]):
        valid = index < 4
        require(item["detector_expected"] is valid and item["detector"] is valid
                and item["sentinel_sha256"] == sentinel_hash, "super footer/外域 sentinel 不符")
        if valid:
            value = evidence_json(evidence, "super-" + item["name"] + ".json")
            require(item["sqlite_boundary"]["preconnection_denial"] is True and value["ok"] is False
                    and value["vfs_counters"]["registrations"] == 0 and value["vfs_counters"]["open_calls"] == 0,
                    "有效 super footer 未前置拒绝")
        else:
            boundary = evidence_json(evidence, "super-" + item["name"] + "-boundary-case.json")
            require(item["sqlite_boundary"] == boundary and boundary["positive_hot_proof"] is False
                    and boundary["status_and_full_data_or_error_equal"] is True
                    and boundary["before"] == boundary["after"], "super 边界对拍不符")
            check_boundary(evidence, "super-" + item["name"] + "-boundary", boundary, compare_bytes=True)
    require(report["tail_boundaries"] == evidence_json(evidence, "tail-boundaries.json"), "tail 原件不符")
    for item in report["tail_boundaries"]:
        require(item["positive_hot_proof"] is False and item["kind"] == "opaque-byte-mutation"
                and item["status_and_full_data_or_error_equal"] is True and item["before"] == item["after"],
                "tail 边界名分/原件不符")
        check_boundary(evidence, "tail-" + item["name"], item, compare_bytes=False)
    mutation = evidence_json(evidence, "original-longer-mutation.json")
    require(mutation["kind"] == "crash-image-mutation" and mutation["journal_from_real_sigkill"] is True
            and mutation["not_natural_crash_length_shape"] is True
            and mutation["mutated_main_len"] < mutation["original_len"], "增长正控名分不符")
    require(report["reader_sigkill"] == evidence_json(evidence, "reader-kill.kill.json"), "reader kill 原件不符")
    check_kill(report["reader_sigkill"], "reader", suite_deadline)
    require(report["byte_contract"] == evidence_json(evidence, "byte-contract.json"), "字节对拍原件不符")
    require(report["byte_contract"]["ordinary_file_oracle"] is True, "字节对拍缺普通文件 oracle")
    parameters = evidence_json(evidence, "limits-contract.json")
    require(report["parameter_checks"] == parameters and parameters["profile"] == "D2-SMALL-01"
            and parameters["unknown_field_rejected"] is True and parameters["defaults"] is False,
            "参数拒绝合同不符")
    require({(item["field"], item["mutation"]) for item in parameters["rejected"]}
            == {(field, mutation) for field in D2_LIMITS for mutation in ("missing", "zero", "overflow", "negative")}
            and len(parameters["rejected"]) == 24, "六键边界拒绝缺件")
    require(clock_ns() < suite_deadline, "核验结束已过 suite 截止")
    save(receipt, {"verified": True, "accepted": False, "report_sha256": digest(evidence / "RESULT.json"),
                   "positive_cases": len(CASES), "negative_cases": len(NEGATIVES),
                   "super_journal_cases": len(SUPERS), "tail_boundaries": len(TAILS)})


def main() -> int:
    started_ns = clock_ns()
    started = started_ns / 1_000_000_000
    signal.signal(signal.SIGTERM, remember_signal)
    signal.signal(signal.SIGINT, remember_signal)
    require(signal.getsignal(signal.SIGCHLD) == signal.SIG_DFL, "直属子身份要求 SIGCHLD 默认处理")
    root = Path(os.environ["GITHUB_WORKSPACE"]).resolve()
    temp = Path(os.environ["RUNNER_TEMP"]).resolve()
    artifacts = temp / "1505-d2-artifacts"
    artifacts.mkdir(mode=0o700)
    scratch = None
    result = {"verdict": "FAIL_D2_COMPONENT", "local_execution": False,
              "production_v4": "BLOCK_NOT_MERGEABLE", "supervisor_sigterm_sigint": "NOT_RUN"}
    env = os.environ.copy()
    # 构建之前生成，同一原点贯穿 build/suite/归档；子进程不得重置。
    env["P1505_DEADLINE_NS"] = str(started_ns + 1170 * 1_000_000_000)
    env["P1505_SUITE_DEADLINE_NS"] = str(started_ns + 1020 * 1_000_000_000)
    runner = str(Path(__file__).resolve())
    try:
        check_cancelled()
        supervised("identity", [sys.executable, "-B", runner, "--identity", str(root), str(artifacts / "SOURCE.json")],
                   root, env, min(started + 900, clock_seconds() + 60), started + 1080, artifacts)
        source = read_small_json(artifacts / "SOURCE.json")
        scratch = Path(tempfile.mkdtemp(prefix="1505-d2-", dir=temp))
        env["CARGO_TARGET_DIR"] = str(scratch / "target")
        require(not path_present(scratch / "target"), "构建目标目录必须全新")
        limits = artifacts / "D2-SMALL-01.json"
        save(limits, D2_LIMITS)
        save(artifacts / "ENVIRONMENT.json", {
            "platform": platform.platform(), "disk": os.statvfs(temp).f_bavail * os.statvfs(temp).f_frsize,
            "clock": "CLOCK_MONOTONIC", "started_ns": started_ns,
            "P1505_DEADLINE_NS": int(env["P1505_DEADLINE_NS"]),
            "P1505_SUITE_DEADLINE_NS": int(env["P1505_SUITE_DEADLINE_NS"]),
            "limits_name": "D2-SMALL-01", "limits": D2_LIMITS,
            "build_seconds": 900, "suite_seconds": 120, "active_seconds": 1020,
            "active_reap_end_seconds": 1080, "archive_end_seconds": 1140,
            "scratch_cleanup_end_seconds": 1158, "inventory_end_seconds": 1165,
            "inventory_reap_end_seconds": 1168, "receipt_end_seconds": 1170})
        supervised("build", ["cargo", "build", "--locked", "--no-default-features", "--features", "prototype_1505",
                    "--bin", "p1505_fd_journal", "--message-format=json-render-diagnostics"],
                   root / "rust", env, started + 900, started + 1080, artifacts)
        supervised("verify-build", [sys.executable, "-B", runner, "--verify-build",
                    str(artifacts / "build.stdout"), str(scratch / "target"), str(root),
                    str(artifacts / "SOURCE.json"), str(artifacts / "BINARY.json")],
                   root, env, min(started + 1020, clock_seconds() + 30), started + 1080, artifacts)
        binary = read_small_json(artifacts / "BINARY.json")
        supervised("suite", [binary["path"], "suite", str(scratch / "evidence"), str(limits)], root, env,
                   min(started + 1020, clock_seconds() + 120), started + 1080, artifacts)
        supervised("verify-suite", [sys.executable, "-B", runner, "--verify-suite", str(scratch / "evidence"),
                    str(artifacts / "BINARY.json"), str(root), str(artifacts / "SOURCE.json"),
                    str(artifacts / "SUITE-VERIFIED.json")],
                   root, env, min(started + 1020, clock_seconds() + 30), started + 1080, artifacts)
        verified = read_small_json(artifacts / "SUITE-VERIFIED.json")
        require(verified["verified"] is True and verified["accepted"] is False, "核验回执不符")
        require(clock_ns() < int(env["P1505_SUITE_DEADLINE_NS"]), "suite 核验迟到")
        result["source"] = source
        result["verdict"] = "PASS_D2_COMPONENT_ONLY"
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
    elif len(sys.argv) == 4 and sys.argv[1] == "--identity":
        save(Path(sys.argv[3]), identity(Path(sys.argv[2])))
    elif len(sys.argv) == 7 and sys.argv[1] == "--verify-build":
        verify_build(*(Path(arg) for arg in sys.argv[2:]))
    elif len(sys.argv) == 7 and sys.argv[1] == "--verify-suite":
        verify_suite(*(Path(arg) for arg in sys.argv[2:]))
    else:
        raise SystemExit(main())
