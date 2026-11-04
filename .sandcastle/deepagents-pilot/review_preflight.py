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
_PENDING_CLEANUP: dict[str, tuple[Path, str]] = {}
_CLEANUP_LOCK = threading.Lock()
_DEADLINE_AT = float("inf")
SCRUBBED_ENV = "CLIPROXY_LOCAL_KEY"


def git_environment() -> dict[str, str]:
    """与 external_review.git_environment 相同：不读系统/全局配置、不跟随 replace 引用。"""
    env = {k: os.environ[k] for k in ("PATH", "LANG", "LC_ALL", "TMPDIR") if k in os.environ}
    env.update(GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull, GIT_TERMINAL_PROMPT="0",
               GIT_OPTIONAL_LOCKS="0", GIT_NO_REPLACE_OBJECTS="1")
    return env


def _git(source: Path, *args: str, data: bytes | None = None) -> bytes:
    return subprocess.run(["git", "-C", str(source), *args], input=data, capture_output=True, check=True,
                          env=git_environment()).stdout


def snapshot_bytes(source: Path, base: str, head: str, merge_base: str) -> int:
    """单个模型审阅快照的所需字节，逐项复刻 external_review.prepare_review 的公式。三条互不依赖的 git 读并发执行。"""
    def start(*args: str) -> subprocess.Popen:
        return subprocess.Popen(["git", "-C", str(source), *args], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
                                env=git_environment())

    def finish(proc: subprocess.Popen) -> bytes:
        out, _ = proc.communicate()
        if proc.returncode:
            raise subprocess.CalledProcessError(proc.returncode, proc.args)
        return out

    snapshot_objects = start("rev-list", "--objects", "--no-walk", base, head, merge_base)
    commits = start("rev-list", base, head)
    tree = start("ls-tree", "-rl", head)
    objects = {line.split(b" ", 1)[0] for line in finish(snapshot_objects).splitlines()}
    objects.update(finish(commits).splitlines())
    sizes = _git(source, "cat-file", "--buffer", "--batch-check=%(objectsize)", data=b"\n".join(sorted(objects)) + b"\n").splitlines()
    worktree = sum(int(line.split(b"\t", 1)[0].split()[-1]) for line in finish(tree).splitlines() if line.split()[1] == b"blob")
    return sum(int(size) for size in sizes) + worktree + SPACE_RESERVE


def _existing(path: Path) -> Path:
    while not path.exists():
        path = path.parent
    return path


def check_space(args) -> dict:
    try:
        per = snapshot_bytes(Path(args.source), args.base, args.head, args.merge_base)
    except subprocess.CalledProcessError:
        # 只在失败时诊断：与 prepare_review 相同，SHA 不在 source 对象库是 needs_source_sync，不是空间问题。
        shas = sorted({args.base, args.head, args.merge_base})
        out = _git(Path(args.source), "cat-file", "--batch-check", data="".join(f"{s}^{{commit}}\n" for s in shas).encode())
        missing = [sha for sha, line in zip(shas, out.decode().splitlines(), strict=True) if line.endswith("missing")]
        if not missing:
            raise
        return {"name": "source_objects", "class": "source", "ok": False,
                "detail": {"needs_source_sync": missing, "source": args.source}}
    need = per * len(MODELS)
    free = shutil.disk_usage(_existing(Path(args.checkout_root))).free
    detail = {"required_bytes": need, "per_snapshot_bytes": per, "snapshots": len(MODELS), "free_bytes": free}
    if free < need:
        detail.update(vm_deleted_open_bytes())
    return {"name": "checkout_root_free_space", "class": "space", "ok": free >= need, "detail": detail}


def vm_deleted_open_bytes() -> dict:
    """空间不足时的诊断：colima/lima 的 VM 进程常持有已删除快照与 worktree 的 inode，空间要等 VM 重启才回收。"""
    pids = subprocess.run(["pgrep", "-x", "com.apple.Virtualization.VirtualMachine"],
                          capture_output=True, text=True).stdout.split()
    held, files = 0, 0
    for pid in pids:
        try:
            out = subprocess.run(["lsof", "-nP", "+L1", "-p", pid, "-Fsin"], capture_output=True, text=True,
                                 timeout=10).stdout
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


def _reclaim(probe: Path, container: str, timeout: float) -> str | None:
    """回收一个挂载探针；目录删除总会执行。返回失败说明，成功返回 None。"""
    error = None
    try:
        result = subprocess.run(["docker", "rm", "-f", container], capture_output=True, text=True,
                                timeout=max(0.1, timeout))
        if result.returncode and "No such container" not in result.stderr:
            error = f"docker rm -f {container} 退出 {result.returncode}"
    except (OSError, subprocess.TimeoutExpired) as exc:
        error = f"docker rm -f {container} {type(exc).__name__}"
    finally:
        shutil.rmtree(probe, ignore_errors=True)
    if probe.exists():
        error = f"{error + '；' if error else ''}探针目录 {probe} 未删净"
    return error


def reclaim_pending(budget: float) -> list[str]:
    """总时限到时回收仍登记的探针，耗时不超过 budget（留在总时限之内）。"""
    end = time.monotonic() + budget
    with _CLEANUP_LOCK:
        pending = list(_PENDING_CLEANUP.values())
        _PENDING_CLEANUP.clear()
    failures = []
    for probe, container in pending:
        error = _reclaim(probe, container, end - time.monotonic())
        if error:
            failures.append(error)
    return failures


def check_mount(args) -> dict:
    """宿主写一次性 nonce，审阅镜像经同样的只读 bind 挂载 checkout_root 读回；读不到即容器看见的不是真实文件。"""
    root = Path(args.checkout_root)
    root.mkdir(parents=True, exist_ok=True, mode=0o700)
    nonce = os.urandom(16).hex()
    container = f"preflight-{nonce[:16]}"
    probe = root / container
    probe.mkdir(mode=0o700)
    with _CLEANUP_LOCK:
        _PENDING_CLEANUP[container] = (probe, container)
    try:
        (probe / "nonce").write_text(nonce)
        command = ["docker", "run", "--rm", "--name", container, "--network", "none", "--read-only",
                   "--mount", f"type=bind,src={root},dst=/checkouts,readonly",
                   "--entrypoint", "cat", args.image, f"/checkouts/{probe.name}/nonce"]
        try:
            result = subprocess.run(command, capture_output=True, text=True,
                                    timeout=max(1.0, min(30.0, _DEADLINE_AT - time.monotonic())))
        except subprocess.TimeoutExpired:
            error = _reclaim(probe, container, 3.0)
            with _CLEANUP_LOCK:
                _PENDING_CLEANUP.pop(container, None)
            raise TimeoutError(f"docker run 超时{'；' + error if error else ''}") from None
    finally:
        with _CLEANUP_LOCK:
            owned = _PENDING_CLEANUP.pop(container, None)
        if owned:
            shutil.rmtree(probe, ignore_errors=True)
            if probe.exists():  # 删不掉就重新登记，由主线程回收并在报告里写明
                with _CLEANUP_LOCK:
                    _PENDING_CLEANUP[container] = owned
    seen = result.stdout.strip()
    return {"name": "checkout_root_container_mount", "class": "mount", "ok": result.returncode == 0 and seen == nonce,
            "detail": {"checkout_root": str(root), "image": args.image, "docker_rc": result.returncode,
                       "nonce_match": seen == nonce, "stderr": result.stderr.strip()[-300:]}}


class CredentialError(RuntimeError):
    pass


def provider_key(codex_home: Path, provider: str = PROVIDER, budget: float | None = None) -> tuple[str, dict]:
    """按 Codex 宿主的 provider 配置取 key，且进程环境去掉 CLIPROXY_LOCAL_KEY（#1323 宿主就缺这个变量）。"""
    config = tomllib.loads((codex_home / "config.toml").read_text())
    entry = (config.get("model_providers") or {}).get(provider)
    if entry is None:
        raise CredentialError(f"{codex_home}/config.toml 没有 model_providers.{provider}")
    env = {k: v for k, v in os.environ.items() if k != SCRUBBED_ENV}
    auth = entry.get("auth")
    if isinstance(auth, dict) and auth.get("command"):
        if entry.get("env_key"):
            # Codex 离线校验：provider auth cannot be combined with env_key，宿主回合起不来。
            raise CredentialError("provider auth cannot be combined with env_key")
        timeout = auth.get("timeout_ms", 5000) / 1000
        if budget is not None:
            timeout = min(timeout, max(budget, 0.0))
        try:
            result = subprocess.run([auth["command"], *auth.get("args", [])], env=env, capture_output=True, text=True,
                                    timeout=timeout)
        except subprocess.TimeoutExpired:
            raise CredentialError(f"auth.command 超过 {timeout:g}s 未返回") from None
        except OSError as error:
            raise CredentialError(f"auth.command 无法执行（{type(error).__name__}）") from None
        key = result.stdout.strip()
        if result.returncode != 0 or not key:
            raise CredentialError(f"auth.command 退出 {result.returncode} 或输出为空")
        return key, {"source": "auth.command", "command": auth["command"], "base_url": entry.get("base_url")}
    if entry.get("experimental_bearer_token"):
        return entry["experimental_bearer_token"], {"source": "experimental_bearer_token", "base_url": entry.get("base_url")}
    if entry.get("env_key"):
        if not env.get(entry["env_key"]):
            raise CredentialError(f"Missing environment variable: {entry['env_key']}")
        return env[entry["env_key"]], {"source": "env_key", "env_key": entry["env_key"], "base_url": entry.get("base_url")}
    raise CredentialError(f"model_providers.{provider} 未配置任何凭据来源")


def check_credential(args, budget: float | None = None) -> tuple[dict, str | None]:
    try:
        key, source = provider_key(Path(args.codex_home), budget=budget)
    except CredentialError as error:
        message = str(error)
    except OSError as error:
        message = f"读不到 Codex 配置（{type(error).__name__}）"
    except tomllib.TOMLDecodeError:
        message = "Codex config.toml 解析失败"
    else:
        return {"name": f"credential:{PROVIDER}", "class": "credential", "ok": True,
                "detail": {"scrubbed_env": SCRUBBED_ENV, **source}}, key
    return {"name": f"credential:{PROVIDER}", "class": "credential", "ok": False,
            "detail": {"scrubbed_env": SCRUBBED_ENV, "error": message[:300]}}, None


def worker_key(budget: float = 5.0) -> tuple[str | None, str]:
    """与 codex-child.ts providerEnvironment 相同：先进程环境，再 launchctl getenv。正式 worker 只认这两处。"""
    if os.environ.get(SCRUBBED_ENV):
        return os.environ[SCRUBBED_ENV], "env"
    try:
        value = subprocess.run(["launchctl", "getenv", SCRUBBED_ENV], capture_output=True, text=True,
                               timeout=max(0.1, min(5.0, budget))).stdout.strip()
    except (OSError, subprocess.TimeoutExpired):
        value = ""
    return (value, "launchctl") if value else (None, "missing")


def check_worker_credential(budget: float = 5.0) -> tuple[dict, str | None]:
    key, source = worker_key(budget)
    detail = {"source": source, "consumer": "external-review-worker (providerEnvironment)"}
    if key is None:
        detail["error"] = f"缺少 {SCRUBBED_ENV}：进程环境与 launchctl 均未提供，正式 worker 会在启动前失败"
    return {"name": "credential:review-worker", "class": "credential", "ok": key is not None, "detail": detail}, key


def _endpoint(url: str) -> tuple[str, str, int | None, str]:
    """把 base_url 归一成 (scheme, host, port, path)；回环主机名、IPv4 与 IPv6 回环视为同一端点。"""
    from urllib.parse import (
        urlsplit,  # 用标准解析：手写切分会把带端口的 IPv6 字面量解析错（独审四）
    )

    parts = urlsplit(url.strip().rstrip("/"))
    scheme = parts.scheme.lower()
    host = (parts.hostname or "").lower()
    if host in ("localhost", "::1") or host.startswith("127."):  # DevSkim: ignore DS162092 有意使用本机回环（本地 CLIProxy / 测试假服务），非调试残留
        host = "loopback"
    port = parts.port or {"http": 80, "https": 443}.get(scheme)
    return scheme, host, port, parts.path or ""


def redact(text: str, secret: str | None) -> str:
    """报告出门前抹掉 key 的原文与 JSON 转义形态（错误正文可能回显 Authorization）。"""
    if not secret:
        return text
    for form in {secret, json.dumps(secret, ensure_ascii=False)[1:-1], json.dumps(secret)[1:-1]}:
        text = text.replace(form, "<redacted>")
    return text


def _round_trip(url: str, key: str, model: str, session: str) -> dict:
    trip = _round_trip_raw(url, key, model, session)
    if trip.get("error"):
        trip["error"] = redact(trip["error"], key)[:300]  # 先脱敏再截断，key 跨截断点也不留前缀
    return trip


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
            return {"http_status": response.status, "terminal": terminal, "status": status, "ok": ok,
                    "streamed_models": sorted(streamed),
                    "error": None if ok else str(detail or f"status={status}")[:4000]}
    except urllib.error.HTTPError as error:
        return {"http_status": error.code, "terminal": None, "ok": False, "error": error.read(4000).decode("utf-8", "replace")}
    except (urllib.error.URLError, TimeoutError, OSError) as error:
        return {"http_status": None, "terminal": None, "ok": False, "error": f"{type(error).__name__}: {error}"[:4000]}


def request_log_enabled(config: Path) -> bool | None:
    """只读 CLIProxy 配置 observability.logs.request-log；读不到返回 None（按开启处理）。"""
    try:
        lines = config.read_text().splitlines()
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
        for path in log_dir.glob("v1-responses-*.log"):
            try:  # 其他会话的日志可能在 glob 与 stat 之间被轮转删除
                stat = path.stat()
            except OSError:
                continue
            entries.append((stat.st_mtime, stat.st_size, path))
        for mtime, size, path in sorted(entries, reverse=True):
            if mtime < since - 1:
                break
            try:
                text = path.read_text(errors="replace")
            except OSError:
                continue
            if f"Session-Id: {session}" not in text:
                continue
            stable = sizes.get(path) == size == len(text.encode("utf-8", "replace"))
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


class _Parser(argparse.ArgumentParser):
    def error(self, message):  # 退出码 2 专指“拦截”，用法错误用 3。
        self.print_usage(sys.stderr)
        print(f"review_preflight: {message}", file=sys.stderr)
        sys.exit(3)


def main(argv=None) -> int:
    ap = _Parser(description=__doc__)
    ap.add_argument("--source", required=True, help="控制器读取候选对象的仓库（ExternalReview.source）")
    ap.add_argument("--base", required=True)
    ap.add_argument("--head", required=True)
    ap.add_argument("--merge-base", required=True)
    ap.add_argument("--checkout-root", required=True, help="正式 attempt 将传给 ExternalReview 的 checkout_root")
    ap.add_argument("--image", default="sandcastle:newchanlun", help="与 external-review-worker.mts REVIEW_IMAGE 相同")
    ap.add_argument("--proxy-url", default="http://127.0.0.1:8318")  # DevSkim: ignore DS162092 有意使用本机回环（本地 CLIProxy / 测试假服务），非调试残留
    ap.add_argument("--proxy-config", default=str(CLIPROXY_HOME / "config.yaml"), help="只读，用于确认 request-log 已开启")
    ap.add_argument("--proxy-log-dir", default=str(CLIPROXY_HOME / "auth/logs"))
    ap.add_argument("--codex-home", default=os.environ.get("CODEX_HOME", str(Path.home() / ".codex")),
                    help="宿主 Codex 配置目录；按其 model_providers.cliproxy-local 取 key")
    ap.add_argument("--deadline", type=_deadline, default=55.0, help="总墙钟上限（秒，0 < x ≤ 60），超时的检查判失败")
    args = ap.parse_args(argv)
    global _DEADLINE_AT
    t0 = time.monotonic()
    reserve = min(3.0, args.deadline / 2)  # 回收探针的预算留在总时限之内
    _DEADLINE_AT = t0 + args.deadline - reserve
    remaining = lambda: max(0.0, _DEADLINE_AT - time.monotonic())  # noqa: E731  凭据各阶段共用同一截止时间
    credential, host_key = check_credential(args, budget=remaining())
    credential["name"] = "credential:codex-host"
    worker_check, key = check_worker_credential(remaining())
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
            checks.append({"name": name, "class": kind, "ok": False, "detail": f"{type(value).__name__}: {value}"[:500]})
        else:
            checks.append({"name": name, "class": kind, "ok": False, "detail": f"超过 {args.deadline}s 总时限未完成"})
    ready = all(c["ok"] for c in checks)
    report = {"kind": "review-preflight", "is_review_result": False, "ready": ready, "checks": checks,
              "wall_seconds": round(time.monotonic() - t0, 3)}
    print(redact(redact(json.dumps(report, ensure_ascii=False), key), host_key), flush=True)
    return 0 if ready else 2


if __name__ == "__main__":
    # 不等待超时后仍挂住的探测线程。
    os._exit(main())
