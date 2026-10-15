#!/usr/bin/env python3
"""#1504 仅远端：原监督器真实取消补验；不运行 Rust/SQLite。"""
from __future__ import annotations

import hashlib
import importlib.util
import json
import os
from pathlib import Path
import platform
import signal
import subprocess
import sys
import tarfile
import tempfile
import time

BASE = "b9f38106a5c0cc6f320798cf146e3bc7a6677c1b"
RUNNER_HASH = "e4308e3355d396b5e00dd4a97338180fcc2e517823e36ba9baa82b4f33bcc7bd"
PROBE = ".github/scripts/test-1504-d1-cancellation.py"
RUNNER = ".github/scripts/run-1504-d1.py"
BRANCH = "refs/heads/proto/1504-cancel-probe"
ORIGINAL = b"1504 cancellation: preserve the original evidence\n"


def require(ok: bool, message: str) -> None:
    if not ok:
        raise RuntimeError(message)


def remote_only() -> None:
    require(os.environ.get("GITHUB_ACTIONS") == "true" and platform.system() == "Darwin",
            "仅允许 GitHub macOS runner；禁止本机真实信号探针")


def load_runner():
    path = Path(__file__).with_name("run-1504-d1.py")
    require(hashlib.sha256(path.read_bytes()).hexdigest() == RUNNER_HASH, "原监督器字节变化")
    spec = importlib.util.spec_from_file_location("d1_original_runner", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def identity(runner, root: Path) -> dict:
    remote_only()
    require(os.environ.get("GITHUB_REF") == BRANCH, "补验分支不符")
    require(os.environ.get("GITHUB_EVENT_NAME") == "push", "必须精确候选分支 push")
    head = runner.git(root, "rev-parse", "HEAD")
    require(head == os.environ.get("GITHUB_SHA") == os.environ.get("EXPECTED_SHA"), "精确 SHA 不符")
    require(not runner.git(root, "status", "--porcelain"), "需要干净 checkout")
    subprocess.run(["git", "merge-base", "--is-ancestor", BASE, head], cwd=root, check=True, timeout=15)
    require(set(runner.git(root, "diff", "--name-only", BASE, head).splitlines())
            <= {PROBE, RUNNER, ".github/workflows/prototype-1504-cancel.yml"}, "存在范围外尾差")
    return {"head": head, "tree": runner.git(root, "rev-parse", "HEAD^{tree}"),
            "base": BASE, "runner_sha256": RUNNER_HASH,
            "probe_sha256": runner.digest(root / PROBE),
            "run_id": os.environ["GITHUB_RUN_ID"], "attempt": os.environ["GITHUB_RUN_ATTEMPT"]}


def wait_for(path: Path, proc: subprocess.Popen, deadline: float) -> dict:
    while not path.exists():
        require(proc.poll() is None, "监督器提前退出：" + path.name)
        require(time.monotonic() < deadline, "握手期限到达：" + path.name)
        time.sleep(0.02)
    return json.loads(path.read_text())


def process_snapshot(pids: list[int]) -> list[dict]:
    # 独立 OS 读数；kill(pid,0) 对 zombie 也成立，故另核状态。
    output = subprocess.check_output(
        ["/bin/ps", "-o", "pid=,ppid=,pgid=,stat=", "-p", ",".join(map(str, pids))],
        text=True, timeout=2)
    rows = []
    for line in output.splitlines():
        pid, ppid, pgid, state = line.split()
        rows.append({"pid": int(pid), "ppid": int(ppid), "pgid": int(pgid), "state": state})
    require({row["pid"] for row in rows} == set(pids), "自有进程缺失")
    require(all(not row["state"].startswith("Z") for row in rows), "自有进程已为 zombie")
    return rows


def require_owned(rows: list[dict], supervisor: int, stage: int, child: int) -> None:
    by_pid = {row["pid"]: row for row in rows}
    require(by_pid[supervisor]["pgid"] == supervisor, "监督器会话不符")
    require(by_pid[stage]["ppid"] == supervisor and by_pid[stage]["pgid"] == stage, "阶段组不属监督器")
    require(by_pid[child]["ppid"] == stage and by_pid[child]["pgid"] == stage, "后代不属阶段组")


def payload(control: Path, mode: str, grandchild: bool) -> int:
    remote_only()
    runner = load_runner()
    name = "grandchild" if grandchild else "stage"

    def term_received(signum, _frame):
        marker = control / (name + "-TERM.json")
        if not marker.exists():
            runner.save(marker, {"signal": signum, "pid": os.getpid(), "pgid": os.getpgrp(),
                                 "monotonic": time.monotonic()})

    signal.signal(signal.SIGTERM, term_received)
    signal.signal(signal.SIGINT, signal.SIG_IGN)
    if not grandchild:
        runner.save(control / "STAGE.json", {"pid": os.getpid(), "ppid": os.getppid(), "pgid": os.getpgrp()})
    if grandchild:
        runner.save(control / "CHILD.json", {"pid": os.getpid(), "ppid": os.getppid(), "pgid": os.getpgrp()})
        if mode == "normal":
            return 0
    else:
        child = subprocess.Popen([sys.executable, "-B", str(Path(__file__).resolve()),
                                  "--grandchild", str(control), mode])
        child_info = wait_for(control / "CHILD.json", child, time.monotonic() + 30)
        runner.save(control / "READY.json", {"pid": os.getpid(), "ppid": os.getppid(),
                     "pgid": os.getpgrp(), "child": child_info})
        if mode == "normal":
            require(child.wait(timeout=30) == 0, "正常后代失败")
            return 0
    # 两层都留在原 reap 的 TERM grace 内，最后由原 SIGKILL 回收。
    while True:
        time.sleep(0.1)


def supervisor(case: Path, temp: Path, mode: str) -> int:
    remote_only()
    runner = load_runner()
    root = Path(os.environ["GITHUB_WORKSPACE"]).resolve()
    artifacts = case / "artifacts"
    control = case / "control"
    started = time.monotonic()
    trace = []
    overflow = False

    def observe(frame, event, _arg):
        nonlocal overflow
        if event not in ("call", "return"):
            return
        code = frame.f_code
        if code not in (runner.reap.__code__, runner.remember_signal.__code__):
            return
        if len(trace) >= 100:
            overflow = True
            return
        row = {"function": code.co_name, "event": event, "monotonic": time.monotonic()}
        if code == runner.reap.__code__:
            row["pid"] = frame.f_locals["proc"].pid
        else:
            row["signal"] = frame.f_locals["signum"]
        trace.append(row)

    signal.signal(signal.SIGTERM, runner.remember_signal)
    signal.signal(signal.SIGINT, runner.remember_signal)
    sys.setprofile(observe)  # 只观察原函数；不替换 handler/reap，不发信号。
    scratch = Path(tempfile.mkdtemp(prefix="1504-d1-cancel-", dir=temp))
    (scratch / "evidence").mkdir()
    (scratch / "evidence" / "ORIGINAL.bin").write_bytes(ORIGINAL)
    runner.save(control / "SCRATCH.json", {"path": str(scratch), "started": started,
                "active_end": started + 900, "active_reap_end": started + 1080,
                "receipt_end": started + 1170, "pid": os.getpid(), "pgid": os.getpgrp()})
    result = {"verdict": "FAIL_D1_COMPONENT", "scope": "1504 supervisor cancellation only", "mode": mode}
    env = os.environ.copy()
    try:
        runner.supervised("probe", [sys.executable, "-B", str(Path(__file__).resolve()),
                          "--payload", str(control), mode], root, env,
                          started + 900, started + 1080, artifacts)
        result["verdict"] = "PASS_D1_COMPONENT_ONLY"
    except Exception as exc:
        result["error"] = str(exc)
    finally:
        runner.finish_evidence(scratch, temp, root, env, artifacts, started, result)
    code = runner.publish_result(root, env, artifacts, started, result)
    sys.setprofile(None)
    runner.save(control / "CALLS.json", {"events": trace, "overflow": overflow})
    return code


def verify_case(runner, case: Path, mode: str, exit_code: int) -> dict:
    artifacts = case / "artifacts"
    control = case / "control"
    result = json.loads((artifacts / "RESULT.json").read_text())
    stage = json.loads((artifacts / "probe.json").read_text())
    ready = json.loads((control / "READY.json").read_text())
    scratch = json.loads((control / "SCRATCH.json").read_text())
    expected_cancel = mode == "cancel"
    require(exit_code == (1 if expected_cancel else 0), "监督器退出码不符")
    require(result["verdict"] == ("FAIL_D1_COMPONENT" if expected_cancel else "PASS_D1_COMPONENT_ONLY"), "结论不符")
    require(not result["unreaped_groups"] and result["scratch_removed"], "任务组/scratch 未清理")
    require(stage["group_reaped"] and not stage["timed_out"] and not stage.get("reap_error")
            and stage["cancelled"] == expected_cancel, "阶段回执不符")
    require(not (artifacts / "LATE.json").exists() and result["elapsed_seconds"] < 1170, "超过原期限")
    require((artifacts / "CANCELLED.json").exists() == expected_cancel, "取消终结回执不符")
    require(not Path(scratch["path"]).exists(), "scratch 仍在")
    require(not runner.group_exists(ready["pgid"]), "阶段组仍在")
    require(not runner.group_exists(scratch["pgid"]), "监督器组仍在")
    for name in ("preserve", "archive", "cleanup", "inventory"):
        receipt = json.loads((artifacts / (name + ".json")).read_text())
        require(receipt["exit"] == 0 and receipt["group_reaped"] and not receipt["timed_out"]
                and not receipt.get("error") and not receipt.get("reap_error"), "终结阶段失败：" + name)
        require(not runner.group_exists(receipt["pid"]), "终结阶段组仍在：" + name)
    preservation = json.loads((artifacts / "PRESERVATION.json").read_text())
    require(preservation == {"evidence_present": True, "source": str(Path(scratch["path"]) / "evidence"),
                "destination": str(artifacts / "raw-evidence"), "preserved": True, "source_absent": True}, "原件迁移回执不符")
    require((artifacts / "raw-evidence" / "ORIGINAL.bin").read_bytes() == ORIGINAL, "原件丢失/变化")
    with tarfile.open(artifacts / "evidence.tar.gz", "r:gz") as archive:
        require(archive.getnames() == ["ORIGINAL.bin"], "归档成员不符")
        require(archive.getmembers()[0].isfile(), "归档不是普通原件")
        require(archive.extractfile("ORIGINAL.bin").read() == ORIGINAL, "归档原件变化")
    files = artifacts / "FILES.json"
    seal = json.loads((artifacts / "FILES.SEAL.json").read_text())
    require(seal == result["files_manifest"] and seal["sha256"] == runner.digest(files), "最终封条不符")
    manifest = json.loads(files.read_text())
    require(len(manifest["files"]) == seal["file_count"], "清单计数不符")
    listed = {row["path"] for row in manifest["files"]}
    actual = {str(path.relative_to(artifacts)) for path in runner.regular_files(artifacts)
              if str(path.relative_to(artifacts)) not in runner.FINAL_EXCLUSIONS}
    require(len(listed) == seal["file_count"] and listed == actual, "原件清单集合不完整")
    for row in manifest["files"]:
        path = artifacts / row["path"]
        require(path.is_relative_to(artifacts) and ".." not in path.parts, "清单越界")
        require(path.stat().st_size == row["bytes"] and runner.digest(path) == row["sha256"], "原件清单不符")
    calls = json.loads((control / "CALLS.json").read_text())
    require(not calls["overflow"], "调用观察溢出")
    signals = [row for row in calls["events"] if row["function"] == "remember_signal" and row["event"] == "call"]
    if expected_cancel:
        require([row["signal"] for row in signals] == [signal.SIGTERM, signal.SIGINT], "真实 handler 次序不符")
        require(stage["signals_observed"] == {"count": 2, "first_signal": signal.SIGTERM}, "阶段未观察两信号")
        require(result["cancellation"] == stage["signals_observed"], "终结取消状态不符")
        cancelled = json.loads((artifacts / "CANCELLED.json").read_text())
        require(cancelled == {"signals": stage["signals_observed"], "accepted": False}, "取消收据不符")
        reap_events = [row for row in calls["events"] if row["function"] == "reap" and row["pid"] == ready["pid"]]
        require([row["event"] for row in reap_events] == ["call", "return"], "阶段回收观察不完整")
        require(reap_events[0]["monotonic"] < signals[1]["monotonic"] < reap_events[1]["monotonic"],
                "第二信号没有发生在原 reap 内")
        sent = json.loads((control / "INT-SENT.json").read_text())
        term = json.loads((control / "stage-TERM.json").read_text())
        require(term["pid"] == ready["pid"] and term["signal"] == signal.SIGTERM
                and term["monotonic"] < sent["before"] <= sent["after"], "实际 TERM/INT 时序不符")
        for rows in (sent["live_before"], sent["live_after"]):
            require_owned(rows, scratch["pid"], ready["pid"], ready["child"]["pid"])
    else:
        require(not signals and stage["signals_observed"]["count"] == 0 and stage["exit"] == 0, "正常控制遭取消")
    require(not (case / "WATCHDOG.json").exists(), "外层看门狗介入")
    return {"mode": mode, "accepted": True, "supervisor_exit": exit_code,
            "signals": signals, "elapsed_seconds": result["elapsed_seconds"], "original_sha256": hashlib.sha256(ORIGINAL).hexdigest()}


def signal_direct_child(proc: subprocess.Popen, sig: int) -> None:
    # 此 Popen 由本进程创建并一直持有；唯一线程负责 wait/poll。
    # 未 wait/poll 回收的子 PID（即使已为 zombie）不会被 OS 复用。
    require(proc.returncode is None, "子进程已回收，禁止向历史 PID 发信号")
    os.kill(proc.pid, sig)


def cleanup_direct_child(proc: subprocess.Popen, deadline: float, term_grace: float = 2.0) -> dict:
    """仅处理仍由 Popen 保留身份的直接子进程；从不补发历史 PGID。"""
    receipt = {"pid": proc.pid, "signals": [], "errors": [], "reaped": proc.returncode is not None}
    if receipt["reaped"]:
        receipt["exit"] = proc.returncode
        return receipt
    for sig, grace in ((signal.SIGTERM, term_grace), (signal.SIGKILL, 8.0)):
        if proc.returncode is not None:
            break
        disappeared = False
        try:
            signal_direct_child(proc, sig)
            receipt["signals"].append(int(sig))
        except ProcessLookupError:
            # 身份/存在性已有疑点；只 wait，绝不继续 KILL 或另找组。
            disappeared = True
        except Exception as exc:
            receipt["errors"].append("signal: " + str(exc))
        try:
            proc.wait(timeout=max(0.0, min(deadline - time.monotonic(), grace)))
        except subprocess.TimeoutExpired:
            if disappeared:
                break
            continue
        except Exception as exc:
            receipt["errors"].append("wait: " + str(exc))
            break
    receipt["reaped"] = proc.returncode is not None
    receipt["exit"] = proc.returncode
    receipt["finished"] = time.monotonic()
    receipt["within_deadline"] = receipt["finished"] < deadline
    return receipt


def failed_child(runner, path: Path, proc: subprocess.Popen, deadline: float, error: Exception,
                 *, supervisor_child: bool = False) -> None:
    # 回收先于任何可能 ENOSPC/EEXIST 的记录操作；清理异常也不能转成成功。
    receipt = {"accepted": False, "error": str(error),
               "descendants_reaped": not supervisor_child,
               "descendants_state": "UNKNOWN_OR_UNREAPED" if supervisor_child else "NO_DESCENDANTS_BY_DESIGN",
               "unproven_descendants": "未确认归属的后代只记录遗留，不按历史 PGID 发信号"}
    try:
        grace = max(0.0, deadline - time.monotonic() - 2.0) if supervisor_child else 2.0
        receipt["cleanup"] = cleanup_direct_child(proc, deadline, grace)
    except Exception as exc:
        receipt["cleanup_error"] = str(exc)
        receipt["cleanup"] = {"pid": proc.pid, "reaped": False}
    finally:
        try:
            runner.save(path, receipt)
        except Exception as exc:
            # 调用者无论能否写盘都重新抛出原失败；stderr 只作尽力告知。
            try:
                print("failure receipt unavailable: " + type(exc).__name__, file=sys.stderr)
            except Exception:
                pass


def bounded_worker(runner, root: Path, argv: list[str], logs: Path,
                   deadline: float, cleanup_deadline: float) -> int:
    """验证/哈希子进程没有后代；父进程只保留其直接子身份。"""
    require(time.monotonic() < deadline, "验证/封存启动前截止")
    with logs.with_suffix(".stdout").open("xb") as out, logs.with_suffix(".stderr").open("xb") as err:
        proc = subprocess.Popen(argv, cwd=root, stdout=out, stderr=err, start_new_session=True)
        try:
            code = proc.wait(timeout=max(0.0, deadline - time.monotonic()))
            require(time.monotonic() < deadline and code == 0, "验证/封存失败或迟到")
            return code
        except Exception as exc:
            failed_child(runner, logs.with_suffix(".WATCHDOG.json"), proc, cleanup_deadline, exc)
            raise


def verification_worker(case: Path, mode: str, exit_code: int, deadline: float) -> int:
    runner = load_runner()
    require(time.monotonic() < deadline, "验证启动前截止")
    result = verify_case(runner, case, mode, exit_code)
    require(time.monotonic() < deadline, "验证完成已迟到")
    # 子进程不授予最终 accepted；父进程另核退出码与绝对截止。
    result.update(accepted=False, checks_passed=True)
    runner.save(case / "control/VERIFIED.json", result)
    require(time.monotonic() < deadline, "验证收据写出已迟到")
    return 0


def run_case(runner, root: Path, temp: Path, destination: Path, mode: str, overall_end: float) -> dict:
    case = destination / mode
    case.mkdir()
    (case / "artifacts").mkdir()
    (case / "control").mkdir()
    started = time.monotonic()
    hard_end = started + 1180
    require(hard_end < overall_end - 10, "不足完整案例窗口，拒绝启动；不缩原 1170 秒")
    # supervisor 的原 1170 秒保持；剩余时间供独立验证及外层回收。
    with (case / "supervisor.stdout").open("xb") as out, (case / "supervisor.stderr").open("xb") as err:
        proc = subprocess.Popen([sys.executable, "-B", str(root / PROBE), "--supervisor", str(case), str(temp), mode],
                                cwd=root, stdout=out, stderr=err, start_new_session=True)
        try:
            stage_identity = wait_for(case / "control/STAGE.json", proc, started + 30)
            require(stage_identity["ppid"] == proc.pid and stage_identity["pgid"] == stage_identity["pid"], "未知阶段组")
            stage = stage_identity["pid"]
            ready = wait_for(case / "control/READY.json", proc, started + 30)
            require(ready["pid"] == stage and ready["ppid"] == proc.pid and ready["pgid"] == stage, "阶段身份变化")
            if mode == "cancel":
                pids = [proc.pid, stage, ready["child"]["pid"]]
                rows = process_snapshot(pids)
                require_owned(rows, *pids)
                before = time.monotonic()
                signal_direct_child(proc, signal.SIGTERM)
                runner.save(case / "control/TERM-SENT.json", {"before": before, "after": time.monotonic(), "live_before": rows})
                wait_for(case / "control/stage-TERM.json", proc, min(hard_end, time.monotonic() + 30))
                rows = process_snapshot(pids)
                require_owned(rows, *pids)
                before = time.monotonic()
                signal_direct_child(proc, signal.SIGINT)
                after = time.monotonic()
                live_after = process_snapshot(pids)
                require_owned(live_after, *pids)
                runner.save(case / "control/INT-SENT.json", {"before": before, "after": after,
                            "live_before": rows, "live_after": live_after})
            code = proc.wait(timeout=max(0.0, started + 1172 - time.monotonic()))
        except Exception as exc:
            failed_child(runner, case / "WATCHDOG.json", proc, hard_end, exc, supervisor_child=True)
            raise
    # wait 已回收 supervisor；后续验证失败不会向它或任何旧 PGID 发信号。
    bounded_worker(runner, root, [sys.executable, "-B", str(root / PROBE), "--verify", str(case), mode,
                   str(code), str(hard_end - 2)], case / "verification", hard_end - 2, hard_end)
    require(time.monotonic() < hard_end, "验证读取前迟到")
    receipt = runner.read_small_json(case / "control/VERIFIED.json")
    require(time.monotonic() < hard_end and receipt["checks_passed"] and not receipt["accepted"], "验证收据迟到或未通过")
    receipt.update(accepted=True, outer_elapsed_seconds=time.monotonic() - started)
    require(time.monotonic() < hard_end, "案例终结迟到")
    return receipt


def seal_worker(destination: Path, root: Path, deadline: float) -> int:
    runner = load_runner()
    require(time.monotonic() < deadline, "最终清单启动前截止")
    source = runner.read_small_json(destination / "SOURCE.json")
    # 只读字节复核，无 git 子进程/后代；HEAD/tree 已在准入时绑定。
    for key, name in (("runner_sha256", RUNNER), ("probe_sha256", PROBE),
                      ("workflow_sha256", ".github/workflows/prototype-1504-cancel.yml")):
        require(runner.digest(root / name) == source[key], "执行后源码字节变化")
    exclusions = {"sealing.stdout", "sealing.stderr", "PROBE-FILES.json", "PROBE-SEAL.json", "PROBE-RESULT.json", "PROBE-LATE.json"}
    rows = [{"path": str(path.relative_to(destination)), "bytes": path.stat().st_size,
             "sha256": runner.digest(path)} for path in runner.regular_files(destination)
            if str(path.relative_to(destination)) not in exclusions]
    require(time.monotonic() < deadline, "最终哈希迟到")
    manifest = destination / "PROBE-FILES.json"
    runner.save(manifest, {"files": rows, "excluded_finalization_files": sorted(exclusions)})
    runner.save(destination / "PROBE-SEAL.json", {"path": manifest.name, "sha256": runner.digest(manifest),
                "bytes": manifest.stat().st_size, "file_count": len(rows)})
    require(time.monotonic() < deadline, "最终封条写出迟到")
    return 0


def publish_probe(runner, destination: Path, result: dict, deadline: float) -> int:
    if time.monotonic() >= deadline:
        result.update(verdict="FAIL_CANCEL_PROBE", receipt_late=True)
    runner.save(destination / "PROBE-RESULT.json", result)
    late = time.monotonic() >= deadline
    if late:
        try:
            runner.save(destination / "PROBE-LATE.json", {"accepted": False, "deadline": deadline,
                        "finished": time.monotonic()})
        except Exception:
            pass
    # 文件中的 PASS 只是候选：必须同时 exit0、无 LATE/任何 WATCHDOG、完整清单。
    return 0 if result["verdict"] == "PASS_CANCEL_PROBE_ONLY" and not late and time.monotonic() < deadline else 1


def main() -> int:
    overall_end = time.monotonic() + 2380
    require(signal.getsignal(signal.SIGCHLD) == signal.SIG_DFL, "需要 wait 保留子进程身份，禁止 SIGCHLD 自动回收")
    runner = load_runner()
    root = Path(os.environ["GITHUB_WORKSPACE"]).resolve()
    source = identity(runner, root)
    source["workflow_sha256"] = runner.digest(root / ".github/workflows/prototype-1504-cancel.yml")
    temp = Path(os.environ["RUNNER_TEMP"]).resolve()
    destination = temp / "1504-cancel-artifacts"
    destination.mkdir(mode=0o700)
    runner.save(destination / "SOURCE.json", source)
    result = {"verdict": "FAIL_CANCEL_PROBE", "source": source, "cases": [], "outer_deadline": overall_end,
              "scope": "仅 #1504 监督器；无 SQLite/Rust；恢复 v4 仍 BLOCK_NOT_MERGEABLE"}
    try:
        for mode in ("normal", "cancel"):
            result["cases"].append(run_case(runner, root, temp, destination, mode, overall_end))
        bounded_worker(runner, root, [sys.executable, "-B", str(root / PROBE), "--seal", str(destination),
                       str(root), str(overall_end - 3)], destination / "sealing", overall_end - 3, overall_end - 1)
        require(time.monotonic() < overall_end - 1, "最终封条读取前截止")
        result["files_manifest"] = runner.read_small_json(destination / "PROBE-SEAL.json")
        require(time.monotonic() < overall_end - 1, "最终封条读取迟到")
        result["verdict"] = "PASS_CANCEL_PROBE_ONLY"
    except Exception as exc:
        result["error"] = str(exc)
    return publish_probe(runner, destination, result, overall_end)


if __name__ == "__main__":
    remote_only()
    if len(sys.argv) == 4 and sys.argv[1] in ("--payload", "--grandchild"):
        raise SystemExit(payload(Path(sys.argv[2]), sys.argv[3], sys.argv[1] == "--grandchild"))
    if len(sys.argv) == 5 and sys.argv[1] == "--supervisor":
        raise SystemExit(supervisor(Path(sys.argv[2]), Path(sys.argv[3]), sys.argv[4]))
    if len(sys.argv) == 6 and sys.argv[1] == "--verify":
        raise SystemExit(verification_worker(Path(sys.argv[2]), sys.argv[3], int(sys.argv[4]), float(sys.argv[5])))
    if len(sys.argv) == 5 and sys.argv[1] == "--seal":
        raise SystemExit(seal_worker(Path(sys.argv[2]), Path(sys.argv[3]), float(sys.argv[4])))
    raise SystemExit(main())
