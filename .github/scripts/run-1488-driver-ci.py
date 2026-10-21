#!/usr/bin/env python3
"""#1488 六条原 driver 成功合同：仅精确远端候选，失败不洗绿。"""
from __future__ import annotations

import hashlib
import importlib.util
import json
import os
from pathlib import Path
import platform
import signal
import stat
import subprocess
import sys
import time

REVIEWED_SOURCE = "9160fc9deec9bb8870c6e757583424b272cf3d69"
REVIEWED_TREE = "660775bac826170882434c84c00e8aa4e5379aa8"
CONTRACT_READY = True  # 两 CI 文件独审、源码绑定和尾差复核后由主控显式开启。
CI_FILES = {".github/scripts/run-1488-driver-ci.py", ".github/workflows/issue-1488-driver-six.yml"}
LEGACY_RUNNER = ".github/scripts/run-1488-delta-ci.py"
LEGACY_SHA256 = "2c94e9e8354bd3fe369067210054db903df019cf9ce8c1345a6ec6d107b297dc"
FEATURES = "s_session,s_capture_native_testonly"
DISK_MIN_BYTES = 2147483648
TESTS = [
    "v3::driver::tests::audit_owner_complete_accept_tail_begin_recover_retry_and_revision_qualification",
    "v3::driver::tests::audit_owner_complete_normal_second_connection_and_new_life_are_cold",
    "v3::driver::tests::audit_owner_incremental_normal_writes_full_oracle_and_suffix",
    "v3::driver::tests::audit_owner_normal_append_keeps_candidate_below_old_multiplier_gate",
    "v3::driver::tests::audit_owner_write_budget_releases_candidate_after_successful_normal_append",
    "v3::driver::tests::audit_owner_staged_normal_append_consumes_actual_nodes_and_suffix",
]
CANCELLATION = {"count": 0, "first_signal": None}
OWNED_GROUPS: dict[int, subprocess.Popen] = {}
FINAL_STATE = {"sealed": False, "artifacts": None}
FINAL_EXCLUSIONS = {
    "FILES.json", "FILES.SEAL.json", "RESULT.json", "LATE.json", "CANCELLED.json",
    "CANCELLED-AFTER-SEAL.json", "FINALIZATION-REJECTED.json",
    "control/inventory.stdout", "control/inventory.stderr", "control/inventory.json",
}

class Cancelled(RuntimeError):
    pass


SOURCE_FILES = {'rust/src/bin/s_session_v3/driver.rs': 'eac29a9962608bb7ae5a2eeae01ac54e0c6b177140b1dd5c99ae23062fb2dc61', 'rust/src/bin/s_session_v3/source_contributions.rs': '55f6bcf121f042950ee36bf67bd0eac208423d35381d59929a5fa5a5f8485cc8', 'rust/src/bin/s_session_v3/audit_owner/effects.rs': '6094df399b27a1982ea781795aacef102736dc2b46c9cd0bd6f0b10ea1afde12'}


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
              "group_reaped": True, "cancelled": False, "cwd": str(cwd),
              "started_monotonic": started, "deadline": deadline,
              "cleanup_deadline": cleanup_deadline, "pgid": None}
    try:
        if allow_cancel:
            check_cancelled()
        require(clock_seconds() < deadline, name + " 启动前期限到达")
        with (artifacts / (name + ".stdout")).open("xb") as out, (artifacts / (name + ".stderr")).open("xb") as err:
            # 信号只记录：Popen 返回与登记之间也不会跳过登记/回收。
            proc = subprocess.Popen(argv, cwd=cwd, env=env, stdout=out, stderr=err, start_new_session=True)
            OWNED_GROUPS[proc.pid] = proc
            result["pid"] = proc.pid
            result["pgid"] = proc.pid
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
        if proc is not None:
            result["exit"] = proc.returncode
        result["ended_monotonic"] = clock_seconds()
        result["elapsed_seconds"] = clock_seconds() - started
        result["signals_observed"] = dict(CANCELLATION)
        save(artifacts / (name + ".json"), result)
    if allow_cancel:
        check_cancelled()
    require(not result.get("error") and not result["timed_out"] and result["exit"] == 0
            and result["group_reaped"], name + " 失败/超时/进程未回收")
    return result


def read_small(path: Path) -> dict:
    with path.open("rb") as stream:
        raw = stream.read(1048577)
    require(len(raw) <= 1048576, "监督器只读取有限元数据")
    value = json.loads(raw)
    require(isinstance(value, dict), "元数据必须为对象")
    return value


def digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def regular_files(directory: Path):
    require(stat.S_ISDIR(directory.lstat().st_mode), "原件根不是目录")
    with os.scandir(directory) as entries:
        children = sorted(entries, key=lambda item: item.name)
    for child in children:
        path = Path(child.path)
        mode = child.stat(follow_symlinks=False).st_mode
        if stat.S_ISDIR(mode):
            yield from regular_files(path)
        else:
            require(stat.S_ISREG(mode), "原件内含链接/特殊文件")
            yield path


def load_legacy(root: Path):
    path = root / LEGACY_RUNNER
    require(digest(path) == LEGACY_SHA256, "旧核验器身份变化")
    spec = importlib.util.spec_from_file_location("driver1488_legacy", path)
    require(spec is not None and spec.loader is not None, "旧核验器无法加载")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def gate() -> None:
    require(CONTRACT_READY, "CI 合同未独审，禁止实际运行")
    require(os.environ.get("GITHUB_ACTIONS") == "true" and platform.system() == "Linux", "仅 GitHub Linux runner")
    require(os.environ.get("GITHUB_REF") == "refs/heads/validate/1488-driver-six", "专用分支不符")
    require(os.environ.get("GITHUB_EVENT_NAME") == "push", "仅一次专用 push")


def git(root: Path, deadline: float, *args: str) -> str:
    remaining = deadline - clock_seconds()
    require(remaining > 0, "git helper 已过期限")
    # helper 的 Git 子进程继承 helper 的组，外层监督器负责整组回收。
    return subprocess.check_output(["git", *args], cwd=root, text=True,
                                   timeout=min(15, remaining)).strip()


def identity(root: Path, deadline: float) -> dict:
    gate()
    head = git(root, deadline, "rev-parse", "HEAD")
    require(head == os.environ.get("GITHUB_SHA") == os.environ.get("EXPECTED_SHA"), "执行 SHA 不符")
    require(not git(root, deadline, "status", "--porcelain", "--untracked-files=all"), "checkout 不干净")
    require(git(root, deadline, "rev-parse", REVIEWED_SOURCE + "^{tree}") == REVIEWED_TREE, "已审来源不符")
    git(root, deadline, "merge-base", "--is-ancestor", REVIEWED_SOURCE, head)
    changed = set(git(root, deadline, "diff", "--name-only", REVIEWED_SOURCE, head).splitlines())
    require(changed == CI_FILES, "源码尾差必须恰为两份 CI 文件")
    sources = {name: digest(root / name) for name in SOURCE_FILES}
    require(sources == SOURCE_FILES, "三份 v4 源码身份漂移")
    require(digest(root / LEGACY_RUNNER) == LEGACY_SHA256, "旧核验器身份漂移")
    return {"head": head, "tree": git(root, deadline, "rev-parse", "HEAD^{tree}"),
            "reviewed_source": REVIEWED_SOURCE, "reviewed_tree": REVIEWED_TREE,
            "source_files": sources, "legacy_runner_sha256": LEGACY_SHA256,
            "ci_files": {name: digest(root / name) for name in sorted(CI_FILES)},
            "run_id": os.environ["GITHUB_RUN_ID"], "attempt": os.environ["GITHUB_RUN_ATTEMPT"]}


def resource_observation(target: Path, artifacts: Path) -> dict:
    disk = os.statvfs(artifacts)
    memory = Path("/proc/meminfo").read_text()
    size = sum(path.stat().st_size for path in regular_files(target)) if target.exists() else 0
    return {"available_disk_bytes": disk.f_bavail * disk.f_frsize,
            "target_observed_bytes": size, "target_observation": "阶段采样，非连续峰值或硬界",
            "memory_raw": memory, "os_memory_hard_cap": False,
            "native_16mib_8mib_budgets": "UNCHANGED"}


def material_diagnostics(directory: Path) -> dict:
    rows = []
    for channel in ["stdout", "stderr"]:
        path = directory / ("test." + channel)
        with path.open("rb") as stream:
            for number, raw in enumerate(stream, 1):
                if b'NativeOwnerMaterialFailure' not in raw:
                    continue
                record = {"channel": channel, "line": number,
                          "raw_line_hex": raw.hex()}
                try:
                    value = json.loads(raw)
                    require(isinstance(value, dict) and value.get("diagnostic") == "NativeOwnerMaterialFailure",
                            "诊断行不是独立 JSON 对象")
                    fields = ["stage", "flags", "resource", "requested_bytes", "cap", "bytes", "peak",
                              "owner_seal", "parent", "scope"]
                    record["values"] = {key: value.get(key) for key in fields}
                    record["missing_fields"] = [key for key in fields if key not in value]
                    record["missing_flags"] = [key for key in ["unexpected", "overflow", "resources", "set", "stages", "autocommit"]
                                               if key not in value.get("flags", {})]
                except (RuntimeError, ValueError, TypeError) as exc:
                    record["parse_error"] = str(exc)
                rows.append(record)
    return {"rows": rows, "count": len(rows), "innermost_first_error": "UNAVAILABLE_UNLESS_RAW_REPORTED",
            "absence_means": "未观测到该行，不推断成功或额度不足"}


def receipt_status(receipt: dict) -> str:
    require(receipt.get("pid") is not None and receipt.get("pgid") == receipt["pid"], "缺真实直属进程身份")
    require(receipt.get("group_reaped") is True and receipt.get("cleanup", {}).get("leader_reaped") is True,
            "残留组或 leader 未回收，必须停止")
    require(not receipt.get("cancelled") and not receipt.get("signals_observed", {}).get("count"), "取消，必须停止")
    require(not receipt.get("error") and not receipt.get("reap_error"), "监督器故障，必须停止")
    require(type(receipt.get("exit")) is int, "没有真实 wait 退出码")
    if receipt.get("timed_out"):
        return "FAIL_TIMEOUT"
    return "PASS" if receipt["exit"] == 0 else "FAIL"


def helper(spec: dict) -> None:
    """所有 Git、完整 hash、大日志/目录遍历均在可回收 helper 内。"""
    root = Path(spec["root"])
    operation = spec["operation"]
    artifacts = Path(spec["artifacts"])
    target = Path(spec["target"])
    deadline = spec["deadline"]
    require(clock_seconds() < deadline, "helper 启动迟到")
    if operation == "inventory":
        entries = []
        for path in regular_files(artifacts):
            relative = path.relative_to(artifacts).as_posix()
            if relative not in FINAL_EXCLUSIONS:
                entries.append({"path": relative, "bytes": path.stat().st_size, "sha256": digest(path)})
        save(artifacts / "FILES.json", {"files": entries, "self_exclusions": sorted(FINAL_EXCLUSIONS)})
        save(artifacts / "FILES.SEAL.json", {"files_sha256": digest(artifacts / "FILES.json"),
                                              "count": len(entries), "self_exclusions": sorted(FINAL_EXCLUSIONS)})
        require(clock_seconds() < deadline, "全原件清单迟到")
        return
    result = {}
    legacy = load_legacy(root)
    if operation in {"identity", "before", "after", "final"}:
        current = identity(root, deadline)
        if "expected_source" in spec:
            require(current == spec["expected_source"], "checkout 与起点身份不同")
        result["source"] = current
    if operation == "identity":
        result["resources"] = resource_observation(target, artifacts)
        require(result["resources"]["available_disk_bytes"] >= DISK_MIN_BYTES, "初始可用磁盘不足 2 GiB")
    if operation == "select":
        result["binary"] = legacy.select_binary((artifacts / "control/build.stdout").read_text(),
                                                 root / "rust", target, "bin", "Native")
    if operation in {"before", "after", "final", "list"}:
        legacy.same_binary(spec["binary"])
        result["binary_unchanged"] = True
    if operation == "list":
        legacy.require_listed((artifacts / "control/list.stdout").read_text(), TESTS)
        result["listed_exact_once"] = TESTS
    if operation == "after":
        directory = Path(spec["test_directory"])
        receipt = read_small(directory / "test.json")
        status = receipt_status(receipt)
        if status == "PASS":
            # stderr 原件另留；libtest 的逐名成功和唯一汇总只能来自 stdout。
            try:
                legacy.require_passed((directory / "test.stdout").read_text(), spec["test_name"])
            except ValueError as exc:
                status = "FAIL_OUTPUT"
                result["output_error"] = str(exc)
        result.update({"name": spec["test_name"], "status": status,
                       "receipt": receipt, "diagnostics": material_diagnostics(directory)})
        result["scratch_remaining"] = [str(p.relative_to(directory / "tmp"))
                                        for p in regular_files(directory / "tmp")]
        result["resources"] = resource_observation(target, artifacts)
    if operation == "final":
        result["resources"] = resource_observation(target, artifacts)
        samples = [result["resources"]["target_observed_bytes"]]
        for index in range(1, len(TESTS) + 1):
            path = artifacts / "control" / f"after-{index:02d}.output.json"
            if path.exists():
                samples.append(read_small(path)["resources"]["target_observed_bytes"])
        result["resources"]["target_peak_sampled_bytes"] = max(samples)
        result["unreaped_groups"] = spec["unreaped_groups"]
        require(not spec["unreaped_groups"], "有残留组，不可验收")
    require(operation in {"identity", "select", "before", "after", "final", "list"}, "未知 helper")
    require(clock_seconds() < deadline, "helper 完成迟到")
    save(Path(spec["output"]), result)


def phase(t0: float, work: float, cleanup: float, cap: float) -> tuple[float, float]:
    deadline = min(t0 + work, clock_seconds() + cap)
    return deadline, min(t0 + cleanup, deadline + 10)


def run_helper(name: str, operation: str, context: dict, work: float, cleanup: float, **extra) -> dict:
    deadline, cleanup_deadline = phase(context["t0"], work, cleanup, 30)
    artifacts = Path(context["artifacts"])
    spec = {**context, **extra, "operation": operation, "deadline": deadline,
            "output": str(artifacts / "control" / (name + ".output.json"))}
    path = artifacts / "control" / (name + ".spec.json")
    save(path, spec)
    supervised(name, [sys.executable, "-B", str(Path(__file__).resolve()), "--helper", str(path)],
               Path(context["root"]), command_env(context), deadline, cleanup_deadline, artifacts / "control")
    return read_small(Path(spec["output"])) if operation != "inventory" else read_small(artifacts / "FILES.SEAL.json")


def command_env(context: dict, tmpdir: Path | None = None) -> dict:
    env = os.environ.copy()
    env.update({"CARGO_INCREMENTAL": "0", "CARGO_TARGET_DIR": context["target"],
                "PYTHONDONTWRITEBYTECODE": "1", "RUST_BACKTRACE": "1"})
    if tmpdir is not None:
        env["TMPDIR"] = str(tmpdir)
        env["TMP"] = str(tmpdir)
        env["TEMP"] = str(tmpdir)
    return env


def run_six(context: dict, source: dict, binary: dict, states: list[dict]) -> None:
    for index, name in enumerate(TESTS):
        check_cancelled()
        require(not OWNED_GROUPS, "存在未回收组，停止新测试")
        directory = Path(context["artifacts"]) / "tests" / f"{index + 1:02d}"
        directory.mkdir(parents=True)
        (directory / "tmp").mkdir()
        run_helper(f"before-{index + 1:02d}", "before", context, 2280, 2290,
                   expected_source=source, binary=binary)
        deadline, cleanup_deadline = phase(context["t0"], 2280, 2290, 180)
        states[index] = {"name": name, "status": "FAIL_INCOMPLETE", "directory": f"tests/{index + 1:02d}"}
        try:
            receipt = supervised("test", [binary["path"], name, "--exact", "--format", "pretty", "--color", "never", "--show-output"],
                       Path(context["root"]) / "rust", command_env(context, directory / "tmp"),
                       deadline, cleanup_deadline, directory)
        except RuntimeError:
            # 只有真实保存且已回收的普通非零/超时才继续；不捕获写回执错误或取消。
            check_cancelled()
            receipt = read_small(directory / "test.json")
            status = receipt_status(receipt)
            require(status in {"FAIL", "FAIL_TIMEOUT"}, "不是可继续的普通测试失败")
        actual_status = receipt_status(receipt)
        states[index]["exit"] = receipt["exit"]
        if actual_status != "PASS":
            # 总阶段耗尽导致 post helper 未运行时，仍保留实际 FAIL/TIMEOUT。
            states[index]["status"] = actual_status
        require(not OWNED_GROUPS, "失败后仍有残留组")
        observed = run_helper(f"after-{index + 1:02d}", "after", context, 2280, 2290,
                              expected_source=source, binary=binary, test_name=name, test_directory=str(directory))
        states[index].update({"status": observed["status"], "exit": observed["receipt"]["exit"],
                              "evidence": f"control/after-{index + 1:02d}.output.json"})


def final_handler(signum: int, frame: object) -> None:
    remember_signal(signum, frame)
    if FINAL_STATE["sealed"]:
        # 封口后不可改写 RESULT；独立取消标记加非零退出共同拒绝候选。
        try:
            save(Path(FINAL_STATE["artifacts"]) / "CANCELLED-AFTER-SEAL.json", dict(CANCELLATION))
        finally:
            raise SystemExit(1)


def publish_result(context: dict, states: list[dict], result: dict, seal: dict | None) -> int:
    artifacts = Path(context["artifacts"])
    result.update({"tests": states, "failed": sum(s["status"].startswith("FAIL") for s in states),
                   "not_run": sum(s["status"] == "NOT_RUN" for s in states),
                   "passed": sum(s["status"] == "PASS" for s in states),
                   "unreaped_groups": sorted(OWNED_GROUPS), "signals_observed": dict(CANCELLATION),
                   "supervisor_sigterm_sigint_actual": "OBSERVED_NOT_FAULT_INJECTION_VALIDATED" if CANCELLATION["count"] else "NOT_RUN",
                   "test_timeouts_observed": sum(s["status"] == "FAIL_TIMEOUT" for s in states),
                   "files_sha256": seal.get("files_sha256") if seal else None,
                   "candidate_only": True, "acceptance_requires": ["process exit 0", "job success", "all original evidence",
                     "no late/cancel/finalization rejection markers", "independent remote evidence review"]})
    good = (all(s["status"] == "PASS" for s in states) and not result.get("error")
            and seal is not None and not OWNED_GROUPS and not CANCELLATION["count"]
            and clock_seconds() < context["t0"] + 2370)
    result["verdict"] = "CANDIDATE_SIX_PASS" if good else "FAIL_DRIVER_SIX"
    result["finished_monotonic"] = clock_seconds()
    try:
        if CANCELLATION["count"]:
            save(artifacts / "CANCELLED.json", dict(CANCELLATION))
        save(artifacts / "RESULT.json", result)
        FINAL_STATE.update({"artifacts": str(artifacts), "sealed": True})
        # 原子写仍可能迟到/收到信号，不能把封口前的判定带过最后一门。
        if CANCELLATION["count"] or clock_seconds() >= context["t0"] + 2370:
            name = "CANCELLED.json" if CANCELLATION["count"] else "LATE.json"
            if not (artifacts / name).exists():
                save(artifacts / name, {"signals_observed": dict(CANCELLATION), "at": clock_seconds()})
            return 1
        return 0 if good else 1
    except Exception as exc:
        try:
            save(artifacts / "FINALIZATION-REJECTED.json", {"error": str(exc), "at": clock_seconds()})
        except Exception:
            pass
        return 1


def main() -> int:
    t0 = clock_seconds()  # identity/fetch/build 之前，唯一时钟原点。
    gate()
    require(signal.getsignal(signal.SIGCHLD) == signal.SIG_DFL, "要求 SIGCHLD 默认、无外部 waitpid 回收者")
    signal.signal(signal.SIGTERM, final_handler)
    signal.signal(signal.SIGINT, final_handler)
    root = Path(os.environ["GITHUB_WORKSPACE"]).resolve()
    require(root == Path(__file__).resolve().parents[2], "必须执行当前 checkout 内 runner")
    temp = Path(os.environ["RUNNER_TEMP"]).resolve()
    task = temp / ("1488-driver-six-" + os.environ["GITHUB_RUN_ID"] + "-" + os.environ["GITHUB_RUN_ATTEMPT"])
    task.mkdir(mode=0o700)  # 全新目录，不能继承前轮 target/原件。
    artifacts = task / "evidence"
    artifacts.mkdir()
    (artifacts / "control").mkdir()
    context = {"root": str(root), "artifacts": str(artifacts), "target": str(task / "target"), "t0": t0}
    states = [{"name": name, "status": "NOT_RUN"} for name in TESTS]
    result = {"scope": "#1488 six original driver tests only", "started_monotonic": t0,
              "full_234_and_ci": "NOT_REPLACED", "production_v4": "BLOCK_NOT_MERGEABLE",
              "supervisor_os_eintr_actual": "NOT_RUN",
              "synchronous_io_hard_deadline": "NOT_QUALIFIED",
              "resource_qualification": "2GiB initial disk admission only; no OS heap/store global hard cap",
              "deadlines_seconds": {"identity_fetch": 180, "identity_cleanup": 190, "build": 1080,
                "build_cleanup": 1090, "list": 1140, "one_test": 180, "all_tests": 2280,
                "test_cleanup": 2290, "verify": 2340, "verify_cleanup": 2350, "seal": 2370}}
    source = binary = seal = None
    try:
        first = run_helper("identity-start", "identity", context, 180, 190)
        source = first["source"]
        result["source"] = source
        for name, argv in [("cargo-version", ["cargo", "-V"]), ("rustc-version", ["rustc", "-Vv"]),
                           ("rustup-version", ["rustup", "show", "active-toolchain"])]:
            deadline, cleanup = phase(t0, 180, 190, 30)
            supervised(name, argv, root / "rust", command_env(context), deadline, cleanup, artifacts / "control")
        deadline, cleanup = phase(t0, 180, 190, 180)
        supervised("fetch", ["cargo", "fetch", "--locked"], root / "rust", command_env(context), deadline, cleanup, artifacts / "control")
        deadline, cleanup = phase(t0, 1080, 1090, 1080)
        supervised("build", ["cargo", "test", "--offline", "--locked", "-j", "6", "--features", FEATURES,
                    "--no-run", "--message-format=json", "--bin", "s_structure_session"],
                   root / "rust", command_env(context), deadline, cleanup, artifacts / "control")
        binary = run_helper("verify-build", "select", context, 1130, 1140)["binary"]
        result["binary"] = binary
        deadline, cleanup = phase(t0, 1130, 1140, 30)
        supervised("list", [binary["path"], "--list", "--format", "pretty"], root / "rust",
                   command_env(context), deadline, cleanup, artifacts / "control")
        run_helper("verify-list", "list", context, 1130, 1140, binary=binary)
        run_six(context, source, binary, states)
    except Exception as exc:
        result["error"] = str(exc)
    # 每条 supervised 已消费原 10 秒清理窗口；失败后不续租、不再次发信号。
    # 未回收直属组仅留存身份，停止新进程，交 GitHub runner 外层回收。
    cleanup_rows = [{"pid": pid, "returncode": proc.returncode, "group_reaped": False,
                     "action": "STOP_NO_ADDITIONAL_SIGNAL_OR_CLEANUP_WINDOW"}
                    for pid, proc in OWNED_GROUPS.items()]
    try:
        save(artifacts / "FINAL-CLEANUP.json", {"groups": cleanup_rows, "unreaped": sorted(OWNED_GROUPS)})
        if not CANCELLATION["count"] and not OWNED_GROUPS:
            if source is not None and binary is not None:
                run_helper("verify-final", "final", context, 2340, 2350,
                           expected_source=source, binary=binary, unreaped_groups=[])
            else:
                require(result.get("error") is not None, "缺来源/二进制且无失败理由")
            seal = run_helper("inventory", "inventory", context, 2340, 2350)
    except Exception as exc:
        result["finalization_error"] = str(exc)
        result["error"] = result.get("error", "原件封存失败")
    return publish_result(context, states, result, seal)


if __name__ == "__main__":
    if len(sys.argv) == 3 and sys.argv[1] == "--helper":
        gate()
        helper(read_small(Path(sys.argv[2])))
    else:
        raise SystemExit(main())
