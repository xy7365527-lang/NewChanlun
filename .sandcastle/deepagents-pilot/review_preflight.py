#!/usr/bin/env python3
"""#1742 正式独审 attempt 的启动前预检。只报告环境能否启动，不是审阅结果。"""
import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import threading
import time
import uuid
from pathlib import Path

import tomllib

# 与 pilot1742.external_review.SPACE_RESERVE 相同；预检不导入控制器，避免改变其加载面。
SPACE_RESERVE = 256 * 1024 * 1024
MODELS = ("gpt-6.1-sol", "claude-opus-5-5")
CLIPROXY_HOME = Path.home() / "Library/Application Support/CLIProxyLocal"
# 正式路线：Sol 走 Codex OAuth（chatgpt.com），Opus 走 Claude OAuth（api.anthropic.com）。
ROUTES = {"gpt-6.1-sol": ("codex", "chatgpt.com"), "claude-opus-5-5": ("claude", "api.anthropic.com")}
PROBE_TIMEOUT = 45
PROVIDER = "cliproxy-local"
# 未回收的挂载探针（目录、容器名）。总时限到时由主线程在退出前回收，os._exit 不会跑 daemon 线程的 finally。
_PENDING_CLEANUP: dict[str, dict] = {}  # 容器名 -> {"probe": Path, "proc": Popen | None}
_CLEANUP_LOCK = threading.Lock()
_DEADLINE_AT = float("inf")
_TOTAL_END = float("inf")  # 总时限（含回收预留）的绝对时刻
_CLOSING = False  # 主线程开始回收后置位；之后不再建探针、不再起容器（在 _CLEANUP_LOCK 下读写）
_CLEANUP_FAILURES: list[str] = []  # 任一线程回收失败都记在这里，主线程汇总成 probe_cleanup
_RECLAIMING: set[str] = set()  # 已被探针线程认领、正在回收的容器；主线程退出前限时等它们做完
_RECLAIM_DONE = threading.Condition(_CLEANUP_LOCK)
# 上游错误只按固定词表分类，不把原文抄进报告（原文可能回显 key，且转义形态无法穷举）
ERROR_CLASSES = (("tool_choice", "tool_choice_unsupported"), ("预扣额度", "quota"), ("quota", "quota"),
                 ("insufficient", "quota"), ("invalid api key", "auth"), ("invalid bearer", "auth"), ("unauthorized", "auth"),
                 ("authentication", "auth"), ("rate limit", "rate_limit"), ("overloaded", "overloaded"),
                 ("model_not_found", "model_not_found"), ("not supported", "unsupported"))
SCRUBBED_ENV = "CLIPROXY_LOCAL_KEY"
SHA_RE = re.compile(r"[0-9a-f]{40}")


def _remaining() -> float:
    return max(0.0, _DEADLINE_AT - time.monotonic())


def _total_left() -> float:
    return max(0.0, _TOTAL_END - time.monotonic())


class SourceError(RuntimeError):
    def __init__(self, missing: list[str]):
        super().__init__(f"needs_source_sync: {missing}")
        self.missing = missing


def git_environment() -> dict[str, str]:
    """与 external_review.git_environment 相同：不读系统/全局配置、不跟随 replace 引用。"""
    env = {k: os.environ[k] for k in ("PATH", "LANG", "LC_ALL", "TMPDIR") if k in os.environ}
    env.update(GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull, GIT_TERMINAL_PROMPT="0",
               GIT_OPTIONAL_LOCKS="0", GIT_NO_REPLACE_OBJECTS="1")
    return env


def _git(source: Path, *args: str, data: bytes | None = None) -> bytes:
    return subprocess.run(["git", "-C", str(source), *args], input=data, capture_output=True, check=True,
                          env=git_environment(), timeout=_git_timeout()).stdout


def _git_timeout() -> float | None:
    left = _remaining()
    return None if left == float("inf") else max(0.01, left)


def snapshot_bytes(source: Path, base: str, head: str, merge_base: str) -> int:
    """单个模型审阅快照的所需字节，逐项复刻 external_review.prepare_review 的公式。互不依赖的 git 读并发执行。"""
    def start(*args: str, stdin: bool = False) -> subprocess.Popen:
        return subprocess.Popen(["git", "-C", str(source), *args], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
                                stdin=subprocess.PIPE if stdin else None, env=git_environment())

    def finish(proc: subprocess.Popen, data: bytes | None = None) -> bytes:
        out, _ = proc.communicate(data, timeout=_git_timeout())
        if proc.returncode:
            raise subprocess.CalledProcessError(proc.returncode, proc.args)
        return out

    shas = sorted({base, head, merge_base})
    procs = []
    try:
        # 与 prepare_review 相同：三者须是 source 里已有的 commit，否则是 needs_source_sync
        kinds = start("cat-file", "--batch-check", stdin=True)
        procs.append(kinds)
        snapshot_objects = start("rev-list", "--objects", "--no-walk", base, head, merge_base)
        commits = start("rev-list", base, head)
        tree = start("ls-tree", "-rl", head)
        procs += [snapshot_objects, commits, tree]
        lines = finish(kinds, "".join(f"{sha}^{{commit}}\n" for sha in shas).encode()).decode().splitlines()
        missing = [sha for sha, line in zip(shas, lines, strict=True) if line.endswith("missing")]
        if missing:
            raise SourceError(missing)
        objects = {line.split(b" ", 1)[0] for line in finish(snapshot_objects).splitlines()}
        objects.update(finish(commits).splitlines())
        sizes = _git(source, "cat-file", "--buffer", "--batch-check=%(objectsize)",
                     data=b"\n".join(sorted(objects)) + b"\n").splitlines()
        worktree = sum(int(line.split(b"\t", 1)[0].split()[-1]) for line in finish(tree).splitlines()
                       if line.split()[1] == b"blob")
        return sum(int(size) for size in sizes) + worktree + SPACE_RESERVE
    finally:
        for proc in procs:  # 任一步失败或超时，不留下其余 git 子进程
            if proc.poll() is None:
                proc.kill()
                proc.wait()


def _existing(path: Path) -> Path:
    while not path.exists():
        path = path.parent
    return path


def check_space(args) -> dict:
    try:
        per = snapshot_bytes(Path(args.source), args.base, args.head, args.merge_base)
    except SourceError as error:
        return {"name": "source_objects", "class": "source", "ok": False,
                "detail": {"needs_source_sync": error.missing, "source": args.source}}
    need = per * len(MODELS)
    free = shutil.disk_usage(_existing(Path(args.checkout_root).absolute())).free
    detail = {"required_bytes": need, "per_snapshot_bytes": per, "snapshots": len(MODELS), "free_bytes": free}
    if free < need:
        try:  # 诊断失败不能丢掉已算出的空间明细
            detail.update(vm_deleted_open_bytes())
        except (OSError, subprocess.SubprocessError, ValueError) as error:
            detail["vm_diagnosis_error"] = type(error).__name__
    return {"name": "checkout_root_free_space", "class": "space", "ok": free >= need, "detail": detail}


def vm_deleted_open_bytes() -> dict:
    """空间不足时的诊断：colima/lima 的 VM 进程常持有已删除快照与 worktree 的 inode，空间要等 VM 重启才回收。"""
    pids = subprocess.run(["pgrep", "-x", "com.apple.Virtualization.VirtualMachine"],
                          capture_output=True, text=True, timeout=max(0.01, min(5.0, _remaining()))).stdout.split()
    held, files = 0, 0
    for pid in pids:
        try:
            # -a：选择条件取交集。缺了它 lsof 会把其它进程的已删除文件也算进来（实测 144GB vs VM 自身 11GB）
            out = subprocess.run(["lsof", "-nP", "-a", "+L1", "-p", pid, "-Fsin"], capture_output=True, text=True,
                                 timeout=max(0.01, min(10.0, _remaining()))).stdout
        except (OSError, subprocess.TimeoutExpired):
            continue
        seen, size = set(), None
        for line in out.splitlines():
            if line[:1] == "f":
                size = None
            elif line[:1] == "s":
                size = int(line[1:])
            elif line[:1] == "i" and size is not None and line[1:] not in seen:
                seen.add(line[1:])
                held += size
        files += len(seen)
    if not held:
        return {}
    return {"vm_deleted_open_bytes": held, "vm_deleted_open_files": files,
            "hint": "VM 进程仍持有已删除文件；先停掉其中运行的容器，再重启 colima 才能回收这部分空间"}


def _bounded(fn, timeout: float):
    """在守护线程里跑可能卡住的文件系统操作，最多等 timeout 秒；超时返回 (False, None)，主线程不被拖住。"""
    box: dict = {}

    def run() -> None:
        try:
            box["value"] = fn()
        except OSError as error:
            box["error"] = error

    worker = threading.Thread(target=run, daemon=True)
    worker.start()
    worker.join(max(0.0, timeout))
    if worker.is_alive():
        return False, None
    return True, box.get("value", box.get("error"))


def _remove_probe_dir(probe: Path, timeout: float) -> str | None:
    """限时删除探针目录并确认已不存在；超时或删不净都返回失败说明。"""
    def remove() -> bool:
        shutil.rmtree(probe, ignore_errors=True)
        return probe.exists()

    done, leftover = _bounded(remove, timeout)
    if not done:
        return f"探针目录 {probe} 删除在时限内未完成"
    if leftover is not False:
        return f"探针目录 {probe} 未删净"
    return None


def _container_present(container: str, timeout: float) -> bool | None:
    try:
        result = subprocess.run(["docker", "ps", "-aq", "--filter", f"name=^/{container}$"], capture_output=True,
                                text=True, timeout=timeout)
    except (OSError, subprocess.TimeoutExpired):
        return None
    return bool(result.stdout.strip()) if result.returncode == 0 else None


def _reclaim(entry: dict, container: str, budget: float) -> str | None:
    """回收一个挂载探针：先杀掉 docker run 的整个进程组，再 rm -f 并确认容器已不存在；目录总会删。返回失败说明。"""
    import signal

    end = time.monotonic() + budget
    errors = []
    proc = entry.get("proc")
    if proc is not None:
        # 不以父进程是否存活为前提：包装脚本可能已退出、却留下后台子进程继续建容器（独审九）。
        # start_new_session=True 使 pgid == 父 pid；组内已无成员时 killpg 报 ProcessLookupError，属正常。
        # 残余风险：组内全部成员退出后的数秒内，该 pgid 恰被无关进程组复用——概率极低，接受。
        try:
            if proc.pid <= 1:
                raise ProcessLookupError(proc.pid)
            os.killpg(proc.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        except OSError:
            pass
        if proc.poll() is None:
            proc.kill()  # 父进程仍在（或进程组信号不可用）时直接结束它
            left = min(1.0, end - time.monotonic())
            if left > 0:
                try:
                    proc.wait(timeout=left)
                except subprocess.TimeoutExpired:
                    pass
        if proc.poll() is None:
            errors.append(f"docker run 进程未退出，容器 {container} 可能仍会被创建")
    # 先删宿主侧探针目录：不依赖容器是否已删（容器只读挂载它）；限时执行，文件系统卡住也不拖过总时限
    dir_error = _remove_probe_dir(entry["probe"], max(0.0, min(1.0, end - time.monotonic())))
    if end - time.monotonic() <= 0:
        errors.append(f"回收预算耗尽，未能确认容器 {container} 已删除")
    else:
        try:
            subprocess.run(["docker", "rm", "-f", container], capture_output=True, text=True,
                           timeout=end - time.monotonic())
        except (OSError, subprocess.TimeoutExpired) as exc:
            errors.append(f"docker rm -f {container} {type(exc).__name__}")
        else:
            left = end - time.monotonic()
            if left <= 0:
                errors.append(f"回收预算耗尽，未能确认容器 {container} 已删除")
            else:
                present = _container_present(container, left)
                if present is not False:
                    errors.append(f"容器 {container} {'仍存在' if present else '状态无法确认'}")
    if dir_error:
        errors.append(dir_error)
    return "；".join(errors) or None


def _claim(container: str) -> dict | None:
    """回收所有权：谁从登记里取走条目，谁负责回收；同一探针只会被回收一次（独审·gemini2）。
    认领后记入 _RECLAIMING，做完调用 _release；主线程退出前会限时等这些在途回收结束。"""
    with _CLEANUP_LOCK:
        entry = _PENDING_CLEANUP.pop(container, None)
        if entry is not None:
            _RECLAIMING.add(container)
        return entry


def _release(container: str) -> None:
    with _CLEANUP_LOCK:
        _RECLAIMING.discard(container)
        _RECLAIM_DONE.notify_all()


def _record_failure(error: str | None) -> None:
    if error:
        with _CLEANUP_LOCK:
            _CLEANUP_FAILURES.append(error)


def reclaim_pending(budget: float) -> list[str]:
    """总时限到时回收仍登记的探针，耗时不超过 budget（留在总时限之内）。返回本次与此前登记的全部回收失败。"""
    global _CLOSING
    end = time.monotonic() + budget
    # 限时取锁：探针线程可能正持锁做文件 I/O 或起容器；等不到就如实报告，不让主线程越过总时限（独审七）
    if not _CLEANUP_LOCK.acquire(timeout=max(0.0, budget)):
        return ["回收锁在时限内未取得：挂载探针仍在建立，探针目录或容器可能遗留，回收未能确认"]
    try:
        _CLOSING = True  # 此后 check_mount 不再登记、建目录或起容器
        pending = list(_PENDING_CLEANUP.items())
        _PENDING_CLEANUP.clear()
    finally:
        _CLEANUP_LOCK.release()
    for container, entry in pending:
        _record_failure(_reclaim(entry, container, max(0.0, end - time.monotonic())))
    # 探针线程已认领、仍在回收的条目：限时等它们做完，避免 os._exit 把回收掐在半路
    if not _CLEANUP_LOCK.acquire(timeout=max(0.0, end - time.monotonic())):
        return ["回收锁在时限内未取得：在途回收未能确认"]
    try:
        _RECLAIM_DONE.wait_for(lambda: not _RECLAIMING, timeout=max(0.0, end - time.monotonic()))
        failures = list(_CLEANUP_FAILURES)
        failures += [f"容器 {c} 的回收在时限内未完成，探针目录或容器可能遗留" for c in sorted(_RECLAIMING)]
        return failures
    finally:
        _CLEANUP_LOCK.release()


def check_mount(args) -> dict:
    """宿主写一次性 nonce，审阅镜像经同样的只读 bind 挂载 checkout_root 读回；读不到即容器看见的不是真实文件。"""
    if _remaining() <= 0:
        raise TimeoutError("总时限已到，未启动挂载探针")
    root = Path(args.checkout_root).absolute()  # docker bind 只接受绝对路径
    nonce = os.urandom(16).hex()
    container = f"preflight-{nonce[:16]}"
    probe = root / container
    entry = {"probe": probe, "proc": None}
    command = ["docker", "run", "--rm", "--name", container, "--network", "none", "--read-only",
               "--mount", f"type=bind,src={root},dst=/checkouts,readonly",
               "--entrypoint", "cat", args.image, f"/checkouts/{probe.name}/nonce"]
    # 登记、建目录、起容器、发布进程句柄在同一把锁内完成，并以 closing 为准：
    # 主线程一旦开始回收，这里要么已整体登记可被回收，要么根本不建任何东西（独审六）。
    root.mkdir(parents=True, exist_ok=True, mode=0o700)  # checkout_root 本身不是探针产物，放在锁外
    with _CLEANUP_LOCK:
        if _CLOSING or _remaining() <= 0:
            raise TimeoutError("总时限已到，未启动挂载探针")
        _PENDING_CLEANUP[container] = entry
        try:
            probe.mkdir(mode=0o700)
            (probe / "nonce").write_text(nonce)
        except OSError:
            _PENDING_CLEANUP.pop(container, None)  # 没建成就不登记，免得误报“未删净”
            shutil.rmtree(probe, ignore_errors=True)
            raise
        if _CLOSING or _remaining() <= 0:  # 慢 I/O 后再查一次：时限已过就不起容器
            _PENDING_CLEANUP.pop(container, None)
            shutil.rmtree(probe, ignore_errors=True)
            raise TimeoutError("总时限已到，未启动挂载容器")
        entry["proc"] = proc = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
                                                start_new_session=True)
    try:
        out, err = proc.communicate(timeout=min(30.0, _remaining()) or 0.001)
    except subprocess.TimeoutExpired:
        owned = _claim(container)
        if owned is not None:
            try:
                _record_failure(_reclaim(owned, container, min(3.0, max(0.0, _total_left() - 0.1))))
            finally:
                _release(container)
        raise TimeoutError("docker run 超时") from None
    owned = _claim(container)
    if owned is not None:
        try:
            if proc.returncode != 0:  # 容器可能已建但未启动，--rm 不会清理：一并 rm -f
                _record_failure(_reclaim(owned, container, min(3.0, max(0.0, _total_left() - 0.1))))
            else:
                _record_failure(_remove_probe_dir(probe, min(1.0, max(0.0, _total_left() - 0.1))))
        finally:
            _release(container)
    seen = out.strip()
    return {"name": "checkout_root_container_mount", "class": "mount", "ok": proc.returncode == 0 and seen == nonce,
            "detail": {"checkout_root": str(root), "image": args.image, "docker_rc": proc.returncode,
                       "nonce_match": seen == nonce, "stderr": err.strip()}}


class CredentialError(RuntimeError):
    pass


def _clean_key(value, source: str) -> str:
    """去掉首尾空白；只接受可见 ASCII（0x21–0x7E）且至少 8 个字符的 key（HTTP 头安全，脱敏可靠）。不回显值。"""
    if not isinstance(value, str):
        raise CredentialError(f"{source} 的 key 不是字符串")
    key = value.strip()
    if not key:
        raise CredentialError(f"{source} 的 key 为空")
    if len(key) < 8 or any(not 0x21 <= ord(ch) <= 0x7E for ch in key):
        raise CredentialError(f"{source} 的 key 含非可见 ASCII 字符或少于 8 个字符")
    return key


def provider_key(codex_home: Path, provider: str = PROVIDER, budget: float | None = None) -> tuple[str, dict]:
    """按 Codex 宿主的 provider 配置取 key，且进程环境去掉 CLIPROXY_LOCAL_KEY（#1323 宿主就缺这个变量）。"""
    config = tomllib.loads((codex_home / "config.toml").read_text(encoding="utf-8"))
    providers = config.get("model_providers")
    entry = providers.get(provider) if isinstance(providers, dict) else None
    if not isinstance(entry, dict):
        raise CredentialError(f"{codex_home}/config.toml 没有 model_providers.{provider}")
    if entry.get("base_url") is not None and not isinstance(entry.get("base_url"), str):
        raise CredentialError(f"model_providers.{provider}.base_url 不是字符串")
    env = {k: v for k, v in os.environ.items() if k != SCRUBBED_ENV}
    auth = entry.get("auth")
    if isinstance(auth, dict) and auth.get("command"):
        if entry.get("env_key"):
            # Codex 离线校验：provider auth cannot be combined with env_key，宿主回合起不来。
            raise CredentialError("provider auth cannot be combined with env_key")
        args = auth.get("args", [])
        timeout_ms = auth.get("timeout_ms", 5000)
        if (not isinstance(auth["command"], str) or not isinstance(args, list) or not all(isinstance(a, str) for a in args)
                or isinstance(timeout_ms, bool) or not isinstance(timeout_ms, int | float) or timeout_ms <= 0):
            raise CredentialError("model_providers.auth 的 command/args/timeout_ms 类型无效")
        timeout = timeout_ms / 1000
        if budget is not None:
            timeout = min(timeout, max(budget, 0.0))
        if timeout <= 0:
            raise CredentialError("总时限已到，未执行 auth.command")
        try:
            result = subprocess.run([auth["command"], *args], env=env, capture_output=True, timeout=timeout)
        except subprocess.TimeoutExpired:
            raise CredentialError(f"auth.command 超过 {timeout:g}s 未返回") from None
        except OSError as error:
            raise CredentialError(f"auth.command 无法执行（{type(error).__name__}）") from None
        if result.returncode != 0:
            raise CredentialError(f"auth.command 退出 {result.returncode}")
        key = _clean_key(result.stdout.decode("utf-8", "replace"), "auth.command")
        return key, {"source": "auth.command", "command": auth["command"], "base_url": entry.get("base_url")}
    if entry.get("experimental_bearer_token"):
        return (_clean_key(entry["experimental_bearer_token"], "experimental_bearer_token"),
                {"source": "experimental_bearer_token", "base_url": entry.get("base_url")})
    if entry.get("env_key"):
        if not isinstance(entry["env_key"], str) or not env.get(entry["env_key"]):
            raise CredentialError(f"Missing environment variable: {entry['env_key']}")
        return (_clean_key(env[entry["env_key"]], "env_key"),
                {"source": "env_key", "env_key": entry["env_key"], "base_url": entry.get("base_url")})
    raise CredentialError(f"model_providers.{provider} 未配置任何凭据来源")


def check_credential(args, budget: float | None = None) -> tuple[dict, str | None]:
    try:
        key, source = provider_key(Path(args.codex_home), budget=budget)
    except CredentialError as error:
        message = str(error)
    except OSError as error:
        message = f"读不到 Codex 配置（{type(error).__name__}）"
    except (tomllib.TOMLDecodeError, UnicodeDecodeError):
        message = "Codex config.toml 解析失败"
    except (TypeError, ValueError, AttributeError) as error:
        message = f"Codex provider 配置无效（{type(error).__name__}）"
    else:
        return {"name": f"credential:{PROVIDER}", "class": "credential", "ok": True,
                "detail": {"scrubbed_env": SCRUBBED_ENV, **source}}, key
    return {"name": f"credential:{PROVIDER}", "class": "credential", "ok": False,
            "detail": {"scrubbed_env": SCRUBBED_ENV, "error": message}}, None


def worker_key(budget: float = 5.0) -> tuple[str | None, str]:
    """与 codex-child.ts providerEnvironment 相同：先进程环境，再 launchctl getenv。正式 worker 只认这两处。"""
    if os.environ.get(SCRUBBED_ENV):
        return os.environ[SCRUBBED_ENV], "env"
    if budget <= 0:
        return None, "budget-exhausted"
    try:
        value = subprocess.run(["launchctl", "getenv", SCRUBBED_ENV], capture_output=True,
                               timeout=min(5.0, budget)).stdout.decode("utf-8", "replace")
    except (OSError, subprocess.TimeoutExpired):
        value = ""
    return (value, "launchctl") if value.strip() else (None, "missing")


def check_worker_credential(budget: float = 5.0) -> tuple[dict, str | None]:
    key, source = worker_key(budget)
    detail = {"source": source, "consumer": "external-review-worker (providerEnvironment)"}
    if key is None:
        detail["error"] = f"缺少 {SCRUBBED_ENV}：进程环境与 launchctl 均未提供，正式 worker 会在启动前失败"
    else:
        try:
            key = _clean_key(key, source)
        except CredentialError as error:
            key = None
            detail["error"] = str(error)
    return {"name": "credential:review-worker", "class": "credential", "ok": key is not None, "detail": detail}, key


def _endpoint(url: str) -> tuple[str, str, int | None, str]:
    """把 base_url 归一成 (scheme, host, port, path)；回环主机名、IPv4 与 IPv6 回环视为同一端点。"""
    from urllib.parse import (
        urlsplit,  # 用标准解析：手写切分会把带端口的 IPv6 字面量解析错（独审四）
    )

    parts = urlsplit(url.strip().rstrip("/"))
    scheme = parts.scheme.lower()
    host = (parts.hostname or "").lower()
    if host in ("localhost", "127.0.0.1", "::1"):  # DevSkim: ignore DS162092 有意使用本机回环（本地 CLIProxy / 测试假服务），非调试残留
        host = "loopback"
    try:
        port = parts.port or {"http": 80, "https": 443}.get(scheme)
    except ValueError:
        port = "invalid"
    return scheme, host, port, parts.path or ""


def _secret_forms(secrets) -> list[str]:
    forms = set()
    for secret in secrets:
        if not secret:
            continue
        forms.update({secret, json.dumps(secret, ensure_ascii=False)[1:-1], json.dumps(secret)[1:-1],
                      repr(secret)[1:-1], secret.strip()})
    return sorted((f for f in forms if len(f) >= 4), key=len, reverse=True)


def redact(text: str, secrets, limit: int | None = None) -> str:
    """抹掉 key 的各种形态（原文、JSON/repr 转义），长者优先；再截断，并清掉两端残留的 key 片段（防御纵深）。"""
    if isinstance(secrets, str) or secrets is None:
        secrets = [secrets]
    forms = _secret_forms(secrets)
    for form in forms:
        text = text.replace(form, "<redacted>")
    if limit is not None and len(text) > limit:
        text = text[:limit]
    tail = max((k for form in forms for k in range(len(form) - 1, 3, -1) if text.endswith(form[:k])), default=0)
    if tail:
        text = text[:-tail] + "<redacted>"
    head = max((k for form in forms for k in range(len(form) - 1, 3, -1) if text.startswith(form[-k:])), default=0)
    if head:
        text = "<redacted>" + text[head:]
    return text


def sanitize(value, secrets, limit: int = 500):
    """对报告逐字段脱敏后截断；在序列化之前做，保证输出仍是合法 JSON。"""
    if isinstance(value, dict):
        return {k: sanitize(v, secrets, limit) for k, v in value.items()}
    if isinstance(value, list):
        return [sanitize(v, secrets, limit) for v in value]
    if isinstance(value, str):
        return redact(value, secrets, limit)
    return value


def _round_trip(url: str, key: str, model: str, session: str) -> dict:
    # 不在这里按单把 key 脱敏或截断：两把 key 互为子串时会互相拆坏。统一由报告出门前的 sanitize 用完整 secret 集合处理（独审六）。
    return _round_trip_raw(url, key, model, session)


def _round_trip_raw(url: str, key: str, model: str, session: str) -> dict:
    """一次最小往返，请求形状对齐正式独审：responses + tools + tool_choice=auto + JSON 输出格式。"""
    # 只在探测线程里导入，与 docker/git 等待重叠，不占启动关键路径。
    import urllib.error
    import urllib.request

    body = {"model": model, "stream": True, "store": False, "reasoning": {"effort": "low"},
            "instructions": "Preflight connectivity check. Do not call tools.",
            "input": [{"role": "user", "content": [{"type": "input_text", "text": 'Return {"ok": true}.'}]}],
            "tools": [{"type": "function", "name": "noop", "description": "Never call this.", "strict": False,
                       "parameters": {"type": "object", "properties": {}, "additionalProperties": False}}],
            "tool_choice": "auto", "parallel_tool_calls": True,
            "text": {"format": {"type": "json_schema", "name": "preflight", "strict": True,
                                "schema": {"type": "object", "properties": {"ok": {"type": "boolean"}},
                                           "required": ["ok"], "additionalProperties": False}}}}
    request = urllib.request.Request(url.rstrip("/") + "/v1/responses", data=json.dumps(body).encode(), method="POST",
                                     headers={"Authorization": f"Bearer {key}", "Content-Type": "application/json",
                                              "Accept": "text/event-stream", "Session-Id": session,
                                              "X-Client-Request-Id": session})
    # 在真正发送的边界再查一次（独审八）：入口检查之后还有导入、构造请求与线程调度。检查与系统调用之间
    # 剩下的几条指令之内若越过截止，这次探测也不会延长总墙钟（探测线程是 daemon，退出时被放弃），也不留下产物。
    if _remaining() <= 0 or _CLOSING:
        return {"http_status": None, "terminal": None, "ok": False, "error": {"not_sent": "总时限已到"}}
    deadline = time.monotonic() + PROBE_TIMEOUT
    try:
        with urllib.request.urlopen(request, timeout=PROBE_TIMEOUT) as response:
            terminal, detail, status, streamed = None, "流在终态事件前结束", None, set()
            for raw in response:
                if time.monotonic() > deadline:
                    detail = f"超过 {PROBE_TIMEOUT}s 仍无终态事件"
                    break
                line = raw.decode("utf-8", "replace").strip()
                if line.startswith("data:"):
                    event = json.loads(line[5:]) if line[5:].strip().startswith("{") else {}
                    # 客户端流里自报的服务模型（response.* 的 response.model / message_start 的 message.model）
                    for holder in (event.get("response"), event.get("message")):
                        if isinstance(holder, dict) and isinstance(holder.get("model"), str):
                            streamed.add(holder["model"])
                    if event.get("type") in ("response.completed", "response.incomplete", "response.failed", "error"):
                        terminal = event.get("type")
                        inner = event.get("response") if isinstance(event.get("response"), dict) else {}
                        status = inner.get("status")
                        detail = inner.get("error") or event.get("error") or event.get("message")
                        break
            # 事件名成功还不够：响应体自报 failed/cancelled 或带 error 也算失败。
            ok = (terminal in ("response.completed", "response.incomplete") and not detail
                  and status not in ("failed", "cancelled"))
            body = detail if isinstance(detail, str) else json.dumps(detail, ensure_ascii=False) if detail else ""
            return {"http_status": response.status, "terminal": terminal, "status": status, "ok": ok,
                    "streamed_models": sorted(streamed), "error": None if ok else _error_info(body)}
    except urllib.error.HTTPError as error:
        return {"http_status": error.code, "terminal": None, "ok": False,
                "error": _error_info(error.read(65536).decode("utf-8", "replace"))}
    except Exception as error:  # 请求构造、连接、协议等任何异常都只记类型与 errno，不带异常原文
        reason = getattr(error, "reason", None)
        return {"http_status": None, "terminal": None, "ok": False,
                "error": {"exception": type(error).__name__, "errno": getattr(reason, "errno", getattr(error, "errno", None))}}


def _error_info(body: str) -> dict:
    """上游错误正文只给长度、sha256 前 12 位与固定词表分类，不抄原文。"""
    import hashlib

    lowered = body.lower()
    kind = next((label for needle, label in ERROR_CLASSES if needle in lowered), "unclassified")
    return {"class": kind, "length": len(body), "sha256_12": hashlib.sha256(body.encode("utf-8", "replace")).hexdigest()[:12]}


def request_log_enabled(config: Path) -> bool | None:
    """只读 CLIProxy 配置 observability.logs.request-log；读不到返回 None（按开启处理）。"""
    try:
        lines = config.read_bytes().decode("utf-8", "replace").splitlines()
    except OSError:
        return None
    path, want = [], ("observability", "logs", "request-log")
    for raw in lines:
        if not raw.strip() or raw.lstrip().startswith("#") or ":" not in raw:
            continue
        indent = len(raw) - len(raw.lstrip())
        key, _, value = raw.strip().partition(":")
        while path and path[-1][0] >= indent:
            path.pop()
        path.append((indent, key))
        if tuple(k for _, k in path) == want:
            return value.split("#")[0].strip().lower() not in ("false", "no", "off", "0")
    return None


def served_models(text: str) -> set[str]:
    """从各 API RESPONSE 段读上游实际服务的模型：Anthropic message_start.message.model、Responses response.model。"""
    served = set()
    for section in text.split("=== API RESPONSE ")[1:]:
        section = section.split("\n=== RESPONSE ===", 1)[0]
        for line in section.splitlines():
            line = line.strip().removeprefix("data:").strip()
            if not line.startswith("{"):
                continue
            try:
                event = json.loads(line)
            except ValueError:
                continue
            kind = event.get("type", "") if isinstance(event, dict) else ""
            if kind == "message_start" and isinstance(event.get("message"), dict):
                served.add(event["message"].get("model"))
            elif kind.startswith("response.") and isinstance(event.get("response"), dict) and event["response"].get("model"):
                served.add(event["response"]["model"])
    served.discard(None)
    return served


def _route_from_log(log_dir: Path, session: str, since: float, wait: float = 2) -> tuple[list[tuple[str, str]], set[str]] | None:
    """按本次唯一 Session-Id 找 CLIProxy request-log，读出每次上游尝试的 provider 与主机，以及实际服务模型。

    proxy 的日志以 `=== RESPONSE ===` 段收尾（实测 92/92）。只在该段已出现、且相邻两次轮询文件大小不变时解析，
    避免把写到一半的 `provider=clau` 或尚未写入的实际服务模型当成定论。"""
    deadline = time.monotonic() + wait
    partial, sizes = None, {}
    while True:
        entries = []
        try:
            candidates = list(log_dir.glob("v1-responses-*.log"))
        except OSError:  # 日志目录不可读：按找不到日志处理（route 判失败），不中断探测
            candidates = []
        for path in candidates:
            try:  # 其他会话的日志可能在 glob 与 stat 之间被轮转删除
                stat = path.stat()
            except OSError:
                continue
            entries.append((stat.st_mtime, stat.st_size, path))
        for mtime, size, path in sorted(entries, reverse=True):
            if mtime < since - 1:
                break
            try:
                raw = path.read_bytes()
            except OSError:
                continue
            text = raw.decode("utf-8", "replace")
            if f"Session-Id: {session}" not in text:
                continue
            stable = sizes.get(path) == size == len(raw)  # 按字节比较，日志含非 UTF-8 字节也能判稳定
            sizes[path] = size
            if "\n=== RESPONSE ===" in text and stable:
                providers = re.findall(r"^Auth: provider=([\w.-]+)(?=,|\s*$)", text, re.M)
                hosts = re.findall(r"^Upstream URL: https?://([^/:?\s]+)", text, re.M)
                attempts = list(zip(providers, hosts, strict=True)) if providers and len(providers) == len(hosts) else []
                return attempts, served_models(text)
            partial = ([], set())  # 找到了但还没写完；窗口结束仍未完成即按“无法核实”判失败
            break
        if time.monotonic() > deadline:
            return partial
        time.sleep(0.05)


def probe_model(args, model: str, key: str | None) -> list[dict]:
    if key is None:
        return [{"name": f"roundtrip:{model}", "class": "roundtrip", "ok": False,
                 "detail": "provider 凭据不可用，未发探测"}]
    if _remaining() <= 0:
        return [{"name": f"roundtrip:{model}", "class": "roundtrip", "ok": False, "detail": "总时限已到，未发探测"}]
    session = f"preflight-{uuid.uuid4()}"
    since = time.time()
    trip = _round_trip(args.proxy_url, key, model, session)
    if request_log_enabled(Path(args.proxy_config)) is False:
        provider, host = ROUTES[model]
        return [{"name": f"route:{model}", "class": "route", "ok": False,
                 "detail": {"expected": f"{provider}@{host}", "actual": [], "log_found": False,
                            "error": "CLIProxy observability.logs.request-log=false，无法核实实际路由"}},
                {"name": f"roundtrip:{model}", "class": "roundtrip", "ok": trip["ok"], "detail": trip}]
    # 实测 proxy 在响应结束后 14–85ms 写日志；成功时等 2 秒（约 23 倍余量），往返已失败时日志只作诊断，等 0.3 秒。
    found = _route_from_log(Path(args.proxy_log_dir), session, since, 2 if trip["ok"] else 0.3)
    attempts, served = found if found is not None else (None, set())
    provider, host = ROUTES[model]
    routed = bool(attempts) and all(a == (provider, host) for a in attempts)
    # proxy 会给 Claude 请求注入 fallbacks（如 claude-opus-4-8）。服务模型取上游日志与客户端流两处之并，
    # 必须至少有一处证据且全部等于请求的模型；无从核实也算失败（独审五）。
    served = served | set(trip.get("streamed_models", []))
    return [{"name": f"route:{model}", "class": "route", "ok": routed,
             "detail": {"expected": f"{provider}@{host}", "actual": [f"{p}@{h}" for p, h in attempts or []],
                        "log_found": found is not None, "session": session}},
            {"name": f"identity:{model}", "class": "identity", "ok": bool(served) and served <= {model},
             "detail": {"requested": model, "served": sorted(served), "verified": bool(served)}},
            {"name": f"roundtrip:{model}", "class": "roundtrip", "ok": trip["ok"], "detail": trip}]


def _timed(job) -> list[dict]:
    start = time.monotonic()
    checks = job()
    for check in checks:
        check["elapsed_s"] = round(time.monotonic() - start, 3)
    return checks


def _deadline(text: str) -> float:
    value = float(text)
    if not 0 < value <= 60:  # 承诺单次墙钟 ≤60 秒；0 或负数只会让所有检查超时
        raise argparse.ArgumentTypeError("必须满足 0 < deadline ≤ 60")
    return value


def _emit(report: dict, secrets=()) -> None:
    # ensure_ascii：孤立代理项、surrogateescape 参数都转成 \uXXXX，stdout 永远是合法 ASCII JSON
    print(json.dumps(sanitize(report, list(secrets)), ensure_ascii=True), flush=True)


class _Parser(argparse.ArgumentParser):
    def error(self, message):  # 退出码 2 专指“拦截”，用法错误用 3；stdout 仍给 JSON
        print(f"review_preflight: {message}", file=sys.stderr)
        _emit({"kind": "review-preflight", "is_review_result": False, "ready": False, "usage_error": message})
        sys.exit(3)

    def print_help(self, file=None):
        _emit({"kind": "review-preflight-help", "is_review_result": False, "usage": self.format_help()})


def _sha(text: str) -> str:
    if not SHA_RE.fullmatch(text):
        raise argparse.ArgumentTypeError("必须是 40 位小写十六进制 commit SHA")
    return text


def main(argv=None) -> int:
    ap = _Parser(description=__doc__)
    ap.add_argument("--source", required=True, help="控制器读取候选对象的仓库（ExternalReview.source）")
    ap.add_argument("--base", required=True, type=_sha)
    ap.add_argument("--head", required=True, type=_sha)
    ap.add_argument("--merge-base", required=True, type=_sha)
    ap.add_argument("--checkout-root", required=True, help="正式 attempt 将传给 ExternalReview 的 checkout_root")
    ap.add_argument("--image", default="sandcastle:newchanlun", help="与 external-review-worker.mts REVIEW_IMAGE 相同")
    ap.add_argument("--proxy-url", default="http://127.0.0.1:8318")  # DevSkim: ignore DS162092 有意使用本机回环（本地 CLIProxy / 测试假服务），非调试残留
    ap.add_argument("--proxy-config", default=str(CLIPROXY_HOME / "config.yaml"), help="只读，用于确认 request-log 已开启")
    ap.add_argument("--proxy-log-dir", default=str(CLIPROXY_HOME / "auth/logs"))
    ap.add_argument("--codex-home", default=os.environ.get("CODEX_HOME", str(Path.home() / ".codex")),
                    help="宿主 Codex 配置目录；按其 model_providers.cliproxy-local 取 key")
    ap.add_argument("--deadline", type=_deadline, default=55.0, help="总墙钟上限（秒，0 < x ≤ 60），超时的检查判失败")
    args = ap.parse_args(argv)
    secrets: list = []
    t0 = time.monotonic()
    try:
        return _run(args, t0, secrets)
    except Exception as error:  # 主流程意外异常也要给出合法报告，退出码仍是 2，不变成 1；并照样回收探针
        checks = [{"name": "preflight_internal", "class": "tool", "ok": False, "detail": type(error).__name__}]
        try:
            failures = reclaim_pending(_total_left())
        except Exception as cleanup_error:
            failures = [f"回收本身失败：{type(cleanup_error).__name__}"]
        if failures:
            checks.append({"name": "probe_cleanup", "class": "cleanup", "ok": False, "detail": failures})
        _emit({"kind": "review-preflight", "is_review_result": False, "ready": False, "checks": checks,
               "wall_seconds": round(time.monotonic() - t0, 3)}, secrets)
        return 2


def _run(args, t0: float, secrets: list) -> int:
    global _DEADLINE_AT
    reserve = min(3.0, args.deadline / 2)  # 回收探针的预算留在总时限之内
    global _TOTAL_END, _CLOSING
    with _CLEANUP_LOCK:  # 同一进程内多次调用 main()（测试）时重置回收状态
        _CLOSING = False
        _PENDING_CLEANUP.clear()
        _CLEANUP_FAILURES.clear()
        _RECLAIMING.clear()
    _DEADLINE_AT = t0 + args.deadline - reserve
    _TOTAL_END = t0 + args.deadline
    remaining = lambda: max(0.0, _DEADLINE_AT - time.monotonic())  # noqa: E731  凭据各阶段共用同一截止时间
    credential, host_key = check_credential(args, budget=remaining())
    credential["name"] = "credential:codex-host"
    worker_check, key = check_worker_credential(remaining())
    secrets += [key, host_key, os.environ.get(SCRUBBED_ENV)]
    probe_keys = {model: key for model in MODELS}
    if key is None:
        pass  # 正式 worker 缺 key，attempt 起不来，不再花 token 探测
    elif host_key is not None:
        host_url = credential["detail"].get("base_url") or ""
        preflight_url = args.proxy_url.rstrip("/") + "/v1"
        if _endpoint(host_url) != _endpoint(preflight_url):
            # 宿主不经预检代理时，同一次探测证明不了宿主能完成往返
            credential["ok"] = False
            credential["detail"]["error"] = f"宿主 provider base_url={host_url or '未配置'}，与预检代理 {preflight_url} 不是同一端点"
        elif host_key == key:
            credential["detail"]["proven_by"] = "两家往返探测（与 worker 同一 key、同一端点）"
        else:
            # key 不同：Sol 的那一次探测改用宿主 key，两把 key 各由一次真实往返证明，每家仍只探测一次
            probe_keys["gpt-6.1-sol"] = host_key
            credential["detail"]["proven_by"] = "roundtrip:gpt-6.1-sol（宿主 key）"
            worker_check["detail"]["proven_by"] = "roundtrip:claude-opus-5-5（worker key）"
    # 键为该 job 超时或异常时上报的检查名与类别。
    jobs = {("checkout_root_free_space", "space"): lambda: [check_space(args)],
            ("checkout_root_container_mount", "mount"): lambda: [check_mount(args)],
            **{(f"roundtrip:{model}", "roundtrip"): (lambda m=model: probe_model(args, m, probe_keys[m])) for model in MODELS}}
    results: dict = {}

    def run(key, job) -> None:
        try:
            results[key] = ("ok", _timed(job))
        except Exception as error:  # 检查自身失败按未通过处理，不放行。
            results[key] = ("error", error)

    threads = {key: threading.Thread(target=run, args=(key, job), daemon=True) for key, job in jobs.items()}
    for thread in threads.values():
        thread.start()
    for thread in threads.values():
        thread.join(max(0.0, _DEADLINE_AT - time.monotonic()))
    cleanup_failures = reclaim_pending(max(0.0, t0 + args.deadline - time.monotonic()))
    checks = [credential, worker_check]
    if cleanup_failures:
        checks.append({"name": "probe_cleanup", "class": "cleanup", "ok": False, "detail": cleanup_failures})
    for name, kind in jobs:
        status, value = results.get((name, kind), ("timeout", None))
        if status == "ok":
            checks.extend(value)
        elif status == "error":
            checks.append({"name": name, "class": kind, "ok": False, "detail": f"{type(value).__name__}: {value}"})
        else:
            checks.append({"name": name, "class": kind, "ok": False, "detail": f"超过 {args.deadline}s 总时限未完成"})
    ready = all(c["ok"] for c in checks)
    report = {"kind": "review-preflight", "is_review_result": False, "ready": ready, "checks": checks,
              "wall_seconds": round(time.monotonic() - t0, 3)}
    _emit(report, secrets)
    return 0 if ready else 2


if __name__ == "__main__":
    # 不等待超时后仍挂住的探测线程。
    os._exit(main())
