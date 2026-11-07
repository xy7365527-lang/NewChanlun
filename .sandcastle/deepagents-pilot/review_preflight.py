#!/usr/bin/env python3
"""#1742 正式独审 attempt 的启动前预检。只报告环境能否启动，不是审阅结果。

每项检查对应一种真实发生过的启动失败（对照表见 REVIEW-PREFLIGHT.md）：
  checkout_root_free_space       PR1866 A1/A2、PR1868 A3：双快照空间不足
  checkout_root_container_mount  PR1868 A1：容器里看到的 checkout_root 是空的
  route / roundtrip              PR1883 A1：Opus 被路由到 Antigravity，tool_choice=auto 上游 400
  credential:codex-host          #1323：宿主进程缺 CLIPROXY_LOCAL_KEY
"""
import argparse
import hashlib
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import threading
import time
import urllib.error
import urllib.request
import uuid
from pathlib import Path

import tomllib

# 与 pilot1742.external_review.SPACE_RESERVE 相同；预检不导入控制器，避免改变其加载面。
SPACE_RESERVE = 256 * 1024 * 1024
# 与 pilot1742.external_review.MODELS 相同（测试核对）。
MODELS = ("kimi-k3.1-pro", "claude-opus-5-5")
CLIPROXY_LOG_DIR = Path.home() / "Library/Application Support/CLIProxyLocal/auth/logs"
# 每个模型可接受的上游路线。Sol 走 Codex OAuth（chatgpt.com）；Kimi 走国内 kimi（api.kimi.com）或国际 kimi-ai
# （api.kimi.ai），CLIProxy 在两者间按同型号候选池切换（CLIProxyLocal ROUTING-KIMI-1696.md）；Opus 走 Claude OAuth。
ROUTES = {"gpt-6.1-sol": {("codex", "chatgpt.com")},
          "kimi-k3.1-pro": {("kimi", "api.kimi.com"), ("kimi-ai", "api.kimi.ai")},
          "claude-opus-5-5": {("claude", "api.anthropic.com")}}
# 报告里只回显这些已知路由，其余记作 "other"：日志字段来自上游，不能原样进报告
KNOWN_ROUTES = {*(route for routes in ROUTES.values() for route in routes),
                ("antigravity", "daily-cloudcode-pa.googleapis.com")}
PROVIDER = "cliproxy-local"
SCRUBBED_ENV = "CLIPROXY_LOCAL_KEY"
SHA_RE = re.compile(r"[0-9a-f]{40}")
RESPONSE_STATUSES = ("completed", "incomplete", "failed", "cancelled", "in_progress", "queued")
CLEANUP_RESERVE = 3.0  # 总时限里留给回收与输出报告的秒数
OUTPUT_MARGIN = 0.5  # 回收之后杀进程组、删目录、输出报告所留的秒数（机器忙时也够）
# 上游错误只按固定词表分类，不把原文抄进报告（原文可能回显 key，且转义形态无法穷举）
ERROR_CLASSES = (("tool_choice", "tool_choice_unsupported"), ("预扣额度", "quota"), ("quota", "quota"),
                 ("insufficient", "quota"), ("invalid api key", "auth"), ("invalid bearer", "auth"), ("unauthorized", "auth"),
                 ("authentication", "auth"), ("rate limit", "rate_limit"), ("overloaded", "overloaded"),
                 ("model_not_found", "model_not_found"), ("not supported", "unsupported"))


TERMINAL_EVENTS = ("response.completed", "response.incomplete", "response.failed", "error")
CREDENTIAL_SOURCES = ("auth.command", "experimental_bearer_token", "env_key")
# 报告协议词表：字段名与枚举值都由本脚本写死、与 key 无关，原样输出不透露 key 的任何信息。
# 合法 key 可以是任意可见 ASCII（如 "ok"），与它们相同或是其子串时，脱敏也不能改写它们（调用方按字段与枚举读结论）。
PROTOCOL_FIELDS = frozenset({
    "kind", "is_review_result", "ready", "checks", "wall_seconds", "usage", "usage_error",
    "name", "class", "ok", "detail",
    "checkout_root", "free_bytes", "required_bytes", "per_snapshot_bytes", "snapshots",
    "image", "nonce_match", "docker_rc", "stderr",
    "scrubbed_env", "source", "proven_by", "error",
    "expected", "actual", "log_found", "session",
    "http_status", "terminal", "status", "length", "sha256_12", "exception", "errno", "not_sent"})
PROTOCOL_VALUES = frozenset({
    "review-preflight", "review-preflight-help",
    "checkout_root_free_space", "checkout_root_container_mount", "credential:codex-host", "probe_cleanup",
    "preflight_internal", *(f"{kind}:{model}" for kind in ("route", "roundtrip") for model in MODELS),
    "space", "mount", "credential", "route", "roundtrip", "cleanup", "tool",
    *TERMINAL_EVENTS, *RESPONSE_STATUSES, "other", *(label for _, label in ERROR_CLASSES), "unclassified",
    *(f"{provider}@{host}" for provider, host in KNOWN_ROUTES), *CREDENTIAL_SOURCES})


class Deadline:
    """一次预检的唯一时间来源：检查用到 work，回收与输出用到 total。"""

    def __init__(self, seconds: float):
        self.total = time.monotonic() + seconds
        self.work = self.total - min(CLEANUP_RESERVE, seconds / 2)

    def work_left(self) -> float:
        return max(0.0, self.work - time.monotonic())

    def total_left(self) -> float:
        return max(0.0, self.total - time.monotonic())


def run_bounded(cmd: list[str], timeout: float, *, data: bytes | None = None,
                env: dict | None = None) -> subprocess.CompletedProcess:
    """预检里唯一的子进程原语：在独立进程组里运行，最多等 timeout 秒（超时抛 TimeoutExpired）；
    无论成败，返回前都杀掉整个进程组，包装脚本留下的后台子进程也一并结束。"""
    if timeout <= 0:
        raise subprocess.TimeoutExpired(cmd, 0)
    proc = subprocess.Popen(cmd, stdin=subprocess.PIPE if data is not None else subprocess.DEVNULL,
                            stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env, start_new_session=True)
    try:
        out, err = proc.communicate(data, timeout=timeout)
    finally:
        try:
            os.killpg(proc.pid, signal.SIGKILL)
        except ProcessLookupError:  # 组内已无进程
            pass
        proc.wait()
        for stream in (proc.stdin, proc.stdout, proc.stderr):
            if stream:
                stream.close()
    return subprocess.CompletedProcess(cmd, proc.returncode, out, err)


def git_environment() -> dict[str, str]:
    """与 external_review.git_environment 相同：不读系统/全局配置、不跟随 replace 引用。"""
    env = {k: os.environ[k] for k in ("PATH", "LANG", "LC_ALL", "TMPDIR") if k in os.environ}
    env.update(GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull, GIT_TERMINAL_PROMPT="0",
               GIT_OPTIONAL_LOCKS="0", GIT_NO_REPLACE_OBJECTS="1")
    return env


def snapshot_bytes(source: Path, base: str, head: str, merge_base: str, deadline: Deadline | None = None) -> int:
    """单个模型审阅快照的所需字节，逐项复刻 external_review.prepare_review 的公式。"""
    deadline = deadline or Deadline(60)

    def git(*args: str, data: bytes | None = None) -> bytes:
        result = run_bounded(["git", "-C", str(source), *args], deadline.work_left(), data=data, env=git_environment())
        if result.returncode:
            raise subprocess.CalledProcessError(result.returncode, result.args)
        return result.stdout

    objects = {line.split(b" ", 1)[0] for line in git("rev-list", "--objects", "--no-walk", base, head, merge_base).splitlines()}
    objects.update(git("rev-list", base, head).splitlines())
    sizes = git("cat-file", "--buffer", "--batch-check=%(objectsize)", data=b"\n".join(sorted(objects)) + b"\n").splitlines()
    worktree = sum(int(line.split(b"\t", 1)[0].split()[-1]) for line in git("ls-tree", "-rl", head).splitlines()
                   if line.split()[1] == b"blob")
    return sum(int(size) for size in sizes) + worktree + SPACE_RESERVE


def check_space(args, deadline: Deadline) -> dict:
    per = snapshot_bytes(Path(args.source), args.base, args.head, args.merge_base, deadline)
    need = per * len(MODELS)
    existing = Path(args.checkout_root).absolute()
    while not existing.exists():
        existing = existing.parent
    free = shutil.disk_usage(existing).free
    return {"name": "checkout_root_free_space", "class": "space", "ok": free >= need,
            "detail": {"required_bytes": need, "per_snapshot_bytes": per, "snapshots": len(MODELS), "free_bytes": free}}


def check_mount(args, deadline: Deadline) -> tuple[dict, list[str]]:
    """宿主写一次性 nonce，审阅镜像经同样的只读 bind 挂载 checkout_root 读回；读不到即容器看见的不是真实文件。
    返回 (检查结果, 回收失败列表)。"""
    root = Path(args.checkout_root).absolute()  # docker bind 只接受绝对路径
    nonce = os.urandom(16).hex()
    container = f"preflight-{nonce[:16]}"
    probe = root / container
    detail: dict = {"checkout_root": str(root), "image": args.image}
    ok, cleanup, docker_called, container_done = False, [], False, False
    try:
        root.mkdir(parents=True, exist_ok=True, mode=0o700)
        probe.mkdir(mode=0o700)
        (probe / "nonce").write_text(nonce)
        docker_called = True
        # 与正式审阅容器相同，以宿主 uid:gid 运行（docker-isolation.mts）：探针目录是 0700，镜像用户与宿主不同时
        # 不带 --user 会因 EACCES 误拦健康环境
        result = run_bounded(["docker", "run", "--rm", "--name", container, "--network", "none", "--read-only",
                              "--user", f"{os.getuid()}:{os.getgid()}",
                              "--mount", f"type=bind,src={root},dst=/checkouts,readonly",
                              "--entrypoint", "cat", args.image, f"/checkouts/{container}/nonce"],
                             deadline.work_left())  # 只受总工作时限约束，不另设固定上限
        seen = result.stdout.decode("utf-8", "replace").strip()
        # docker 约定：125 是 docker 自身出错（容器可能没跑完）；其余退出码来自容器里的命令，容器已跑完、--rm 已删
        container_done = result.returncode >= 0 and result.returncode != 125
        ok = result.returncode == 0 and seen == nonce
        detail.update(docker_rc=result.returncode, nonce_match=seen == nonce,
                      # daemon/registry 的自由文本不进报告，与上游错误正文同样只给分类、长度、摘要
                      stderr=_error_info(result.stderr.decode("utf-8", "replace").strip()))
    except subprocess.TimeoutExpired:
        detail["error"] = "docker run 超时"
    except Exception as error:
        detail["error"] = type(error).__name__
    finally:
        # docker run 被杀或非 0 退出时容器可能已建，--rm 不会清：总是 rm -f（容器不存在时也返回 0）。
        try:
            removed = run_bounded(["docker", "rm", "-f", container], deadline.total_left() - OUTPUT_MARGIN)
            if removed.returncode:
                cleanup.append(f"docker rm -f {container} 退出 {removed.returncode}")
        except (OSError, subprocess.TimeoutExpired) as error:
            cleanup.append(f"docker rm -f {container} {type(error).__name__}")
        if docker_called and not container_done:
            # 客户端超时被杀或 docker 自身出错时，daemon 可能已收下创建请求并在 rm -f 之后才建出容器：如实报未确认
            cleanup.append(f"docker run 未正常结束，daemon 可能稍后仍建出容器 {container}；请稍后 docker rm -f {container} 确认")
        shutil.rmtree(probe, ignore_errors=True)
        if probe.exists():
            cleanup.append(f"探针目录 {probe} 未删净")
    return {"name": "checkout_root_container_mount", "class": "mount", "ok": ok, "detail": detail}, cleanup


class CredentialError(RuntimeError):
    pass


def _clean_key(value, source: str) -> str:
    """去掉首尾空白；只接受可见 ASCII（0x21–0x7E）的 key（HTTP 头安全）。不设长度或字符子集门槛——
    正式 worker 不设，预检多设就是误报；短 key、含转义的 key 由脱敏形态覆盖。不回显值。"""
    if not isinstance(value, str):
        raise CredentialError(f"{source} 的 key 不是字符串")
    key = value.strip()
    if not key:
        raise CredentialError(f"{source} 的 key 为空")
    if any(not 0x21 <= ord(ch) <= 0x7E for ch in key):
        raise CredentialError(f"{source} 的 key 含非可见 ASCII 字符")
    return key


def codex_accepts_config(codex_bin: str, codex_home: Path, env: dict, deadline: Deadline) -> None:
    """让宿主自己的 codex 离线加载这份配置（`codex features list`，约 50ms、不发请求）：
    provider/auth 字段组合与取值以 Codex 为准，预检不再自己抄一份校验规则。"""
    try:
        result = run_bounded([codex_bin, "features", "list"], deadline.work_left(), env=env)
    except subprocess.TimeoutExpired:
        raise CredentialError("codex 加载配置超时") from None
    except OSError as error:
        raise CredentialError(f"无法运行 codex（{type(error).__name__}）") from None
    if result.returncode != 0:
        # 只取 Codex 报错里的字段名，其余文本可能含配置取值，不进报告
        stderr = result.stderr.decode("utf-8", "replace")
        field = re.search(r"cannot be combined with ([A-Za-z_]+)", stderr)
        reason = f"provider auth cannot be combined with {field.group(1)}" if field else "配置无效"
        raise CredentialError(f"Codex 拒绝加载该配置（退出 {result.returncode}）：{reason}")


def provider_key(codex_home: Path, deadline: Deadline, codex_bin: str = "codex") -> tuple[str, str]:
    """按 Codex 宿主的 provider 配置取 key，且进程环境去掉 CLIPROXY_LOCAL_KEY（#1323 宿主就缺这个变量）。"""
    # 校验与 auth.command 用同一个环境：去掉 CLIPROXY_LOCAL_KEY、CODEX_HOME 指向被检查的配置目录
    env = {**{k: v for k, v in os.environ.items() if k != SCRUBBED_ENV}, "CODEX_HOME": str(codex_home)}
    codex_accepts_config(codex_bin, codex_home, env, deadline)
    path = codex_home / "config.toml"
    if not path.is_file():  # FIFO 等特殊文件读起来会无界阻塞
        raise CredentialError("Codex config.toml 不是普通文件")
    try:
        config = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, tomllib.TOMLDecodeError) as error:
        raise CredentialError(f"读不到或解析不了 Codex config.toml（{type(error).__name__}）") from None
    providers = config.get("model_providers")
    entry = providers.get(PROVIDER) if isinstance(providers, dict) else None
    if not isinstance(entry, dict):
        raise CredentialError(f"{codex_home}/config.toml 没有 model_providers.{PROVIDER}")
    auth = entry.get("auth")
    if isinstance(auth, dict) and auth.get("command"):
        command, args, timeout_ms = auth["command"], auth.get("args", []), auth.get("timeout_ms", 5000)
        if (not isinstance(command, str) or not isinstance(args, list) or not all(isinstance(a, str) for a in args)
                or isinstance(timeout_ms, bool) or not isinstance(timeout_ms, int | float) or timeout_ms <= 0):
            raise CredentialError("model_providers.auth 的 command/args/timeout_ms 类型无效")
        timeout = min(timeout_ms / 1000, deadline.work_left())
        try:
            result = run_bounded([command, *args], timeout, env=env)
        except subprocess.TimeoutExpired:
            raise CredentialError(f"auth.command 超过 {timeout:g}s 未返回") from None
        except OSError as error:
            raise CredentialError(f"auth.command 无法执行（{type(error).__name__}）") from None
        if result.returncode != 0:
            raise CredentialError(f"auth.command 退出 {result.returncode}")
        return _clean_key(result.stdout.decode("utf-8", "replace"), "auth.command"), "auth.command"
    if entry.get("experimental_bearer_token"):
        return _clean_key(entry["experimental_bearer_token"], "experimental_bearer_token"), "experimental_bearer_token"
    if entry.get("env_key"):
        if not isinstance(entry["env_key"], str) or not env.get(entry["env_key"]):
            # 只回显已知变量名：误填的 key 本身也可能长得像变量名
            name = SCRUBBED_ENV if entry["env_key"] == SCRUBBED_ENV else "<配置的 env_key，不回显>"
            raise CredentialError(f"Missing environment variable: {name}")
        return _clean_key(env[entry["env_key"]], "env_key"), "env_key"
    raise CredentialError(f"model_providers.{PROVIDER} 未配置任何凭据来源")


def check_credential(args, deadline: Deadline) -> tuple[dict, str | None]:
    detail: dict = {"scrubbed_env": SCRUBBED_ENV}
    try:
        key, detail["source"] = provider_key(Path(args.codex_home), deadline, args.codex_bin)
    except CredentialError as error:
        key, detail["error"] = None, str(error)
    else:
        detail["proven_by"] = "两家 roundtrip 都用这把 key"
    return {"name": "credential:codex-host", "class": "credential", "ok": key is not None, "detail": detail}, key


def _secret_forms(secrets) -> list[str]:
    forms = set()
    for secret in secrets:
        if not secret:
            continue
        forms.add(secret.encode("utf-8", "surrogateescape").decode("utf-8", "replace"))
        try:  # key 里的 \uXXXX 之类转义被上游 JSON/TOML 解码后的形态
            decoded = json.loads(f'"{secret}"')
            if isinstance(decoded, str):
                forms.add(decoded)
        except ValueError:
            pass
        forms.update({secret, json.dumps(secret, ensure_ascii=False)[1:-1], json.dumps(secret)[1:-1],
                      repr(secret)[1:-1], secret.strip()})
    # 不按长度过滤：短 key 宁可过度脱敏也不漏（报告里已不含上游原文，剩下的几乎都是本脚本自己的文字）
    return sorted((f for f in forms if f), key=len, reverse=True)


def redact(text: str, secrets, limit: int | None = None) -> str:
    """抹掉 key 的各种形态（原文、JSON/repr 转义），长者优先；再截断，并清掉截断处残留的 key 前缀。"""
    if isinstance(secrets, str) or secrets is None:
        secrets = [secrets]
    forms = _secret_forms(secrets)
    for form in forms:
        text = text.replace(form, "<redacted>")
    if limit is not None and len(text) > limit:
        text = text[:limit]
        # 只有真的截断了，末尾才可能残留 key 前缀；未截断的协议字段不动。
        tail = max((k for form in forms for k in range(len(form) - 1, 1, -1) if text.endswith(form[:k])), default=0)
        if tail:
            text = text[:-tail] + "<redacted>"
    return text


def sanitize(value, secrets, limit: int = 500):
    """对报告逐字段脱敏后截断；在序列化之前做，保证输出仍是合法 JSON。"""
    if isinstance(value, dict):
        return {k if k in PROTOCOL_FIELDS else redact(str(k), secrets, limit): sanitize(v, secrets, limit)
                for k, v in value.items()}
    if isinstance(value, list):
        return [sanitize(v, secrets, limit) for v in value]
    if isinstance(value, str):
        return value if value in PROTOCOL_VALUES else redact(value, secrets, limit)
    return value


def _error_info(body: str) -> dict:
    """上游错误正文只给长度、sha256 前 12 位与固定词表分类，不抄原文。"""
    lowered = body.lower()
    kind = next((label for needle, label in ERROR_CLASSES if needle in lowered), "unclassified")
    return {"class": kind, "length": len(body), "sha256_12": hashlib.sha256(body.encode("utf-8", "replace")).hexdigest()[:12]}


def round_trip(url: str, key: str, model: str, session: str, deadline: Deadline) -> dict:
    """一次最小往返，请求形状对齐正式独审：responses + tools + tool_choice=auto + JSON 输出格式。"""
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
    timeout = deadline.work_left()  # 只受总工作时限约束，不另设更短的固定上限
    if timeout <= 0:
        return {"http_status": None, "terminal": None, "ok": False, "error": {"not_sent": "总时限已到"}}
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            terminal, detail, status = None, "流在终态事件前结束", None
            for raw in response:
                if deadline.work_left() <= 0:
                    detail = "总时限内无终态事件"
                    break
                line = raw.decode("utf-8", "replace").strip()
                if line.startswith("data:"):
                    event = json.loads(line[5:]) if line[5:].strip().startswith("{") else {}
                    if event.get("type") in TERMINAL_EVENTS:
                        terminal = event.get("type")
                        inner = event.get("response") if isinstance(event.get("response"), dict) else {}
                        # 只报已知枚举：上游任意值（可能以各种编码夹带 key）一律归一成 "other"，不进报告
                        status = inner.get("status") if inner.get("status") in RESPONSE_STATUSES else (
                            None if inner.get("status") is None else "other")
                        detail = inner.get("error") or event.get("error") or event.get("message")
                        break
            # 事件名成功还不够：响应体自报 failed/cancelled 或带 error 也算失败。
            ok = (terminal in ("response.completed", "response.incomplete") and not detail
                  and status not in ("failed", "cancelled"))
            text = detail if isinstance(detail, str) else json.dumps(detail, ensure_ascii=False) if detail else ""
            return {"http_status": response.status, "terminal": terminal, "status": status, "ok": ok,
                    "error": None if ok else _error_info(text)}
    except urllib.error.HTTPError as error:
        return {"http_status": error.code, "terminal": None, "ok": False,
                "error": _error_info(error.read(65536).decode("utf-8", "replace"))}
    except Exception as error:  # 连接、协议等任何异常都只记类型与 errno，不带异常原文
        reason = getattr(error, "reason", None)
        return {"http_status": None, "terminal": None, "ok": False,
                "error": {"exception": type(error).__name__, "errno": getattr(reason, "errno", getattr(error, "errno", None))}}


def route_from_log(log_dir: Path, session: str, since: float, wait: float) -> list[tuple[str, str]] | None:
    """按本次唯一 Session-Id 找 CLIProxy request-log，读出每次上游尝试的 (provider, 主机)。
    proxy 的日志以 `=== RESPONSE ===` 段收尾；只在该段已出现、且相邻两次轮询文件大小不变时解析，
    避免把写到一半的 `provider=clau` 当成定论。找不到或一直没写完返回 None。"""
    end = time.monotonic() + wait
    sizes: dict = {}
    while time.monotonic() <= end:
        try:
            paths = list(log_dir.glob("v1-responses-*.log"))
        except OSError:  # 日志目录不可读：按找不到日志处理
            paths = []
        candidates = []
        for path in paths:
            try:  # 其他会话的日志可能在 glob 与 stat 之间被轮转删除
                candidates.append((path.stat().st_mtime, path))
            except OSError:
                continue
        for mtime, path in sorted(candidates, reverse=True):
            if mtime < since - 1:
                break
            try:
                raw = path.read_bytes()
            except OSError:
                continue
            text = raw.decode("utf-8", "replace")
            if f"Session-Id: {session}" not in text:
                continue
            stable = sizes.get(path) == len(raw)
            sizes[path] = len(raw)
            if "\n=== RESPONSE ===" in text and stable:
                # 只读各 API REQUEST 段（到下一个 === 段头为止）：响应正文里伪造的同名行不算路由
                requests = re.findall(r"^=== API REQUEST \d+ ===\n(.*?)(?=^=== )", text, re.M | re.S)
                attempts = []
                for section in requests:
                    provider = re.search(r"^Auth: provider=([\w.-]+)(?=,|\s*$)", section, re.M)
                    host = re.search(r"^Upstream URL: https?://([^/:?\s]+)", section, re.M)
                    if not (provider and host):
                        return []
                    attempts.append((provider.group(1), host.group(1)))
                return attempts
            break
        time.sleep(0.05)
    return None


def probe_model(args, model: str, key: str, deadline: Deadline) -> list[dict]:
    session = f"preflight-{uuid.uuid4()}"
    since = time.time()
    trip = round_trip(args.proxy_url, key, model, session, deadline)
    # 往返成功时日志一定会写（实测响应后 14–85ms），等到工作时限前 0.25 秒（留给线程交回结果）；
    # 往返已失败时日志只作诊断，等 0.3 秒。
    wait = max(0.0, deadline.work_left() - 0.25) if trip["ok"] else 0.3
    attempts = route_from_log(Path(args.proxy_log_dir), session, since, wait)
    expected = ROUTES[model]
    return [{"name": f"route:{model}", "class": "route", "ok": bool(attempts) and all(a in expected for a in attempts),
             "detail": {"expected": sorted("@".join(route) for route in expected), "actual": [f"{p}@{h}" if (p, h) in KNOWN_ROUTES else "other" for p, h in attempts or []],
                        "log_found": attempts is not None, "session": session}},
            {"name": f"roundtrip:{model}", "class": "roundtrip", "ok": trip["ok"], "detail": trip}]


def _guarded(name: str, kind: str, fn) -> dict:
    """检查自身异常按未通过处理，只记异常类型（不带可能含参数的原文）。"""
    try:
        return fn()
    except Exception as error:
        return {"name": name, "class": kind, "ok": False, "detail": {"error": type(error).__name__}}


def _deadline(text: str) -> float:
    value = float(text)
    if not 0 < value <= 60:  # 承诺单次墙钟 ≤60 秒；0 或负数只会让所有检查超时
        raise argparse.ArgumentTypeError("必须满足 0 < deadline ≤ 60")
    return value


def _sha(text: str) -> str:
    if not SHA_RE.fullmatch(text):
        raise argparse.ArgumentTypeError("必须是 40 位小写十六进制 commit SHA")
    return text


def _emit(report: dict, secrets=()) -> None:
    # ensure_ascii：孤立代理项、surrogateescape 参数都转成 \uXXXX，stdout 永远是合法 ASCII JSON
    print(json.dumps(sanitize(report, list(secrets)), ensure_ascii=True), flush=True)


class _Parser(argparse.ArgumentParser):
    def error(self, message):  # 退出码 2 专指“拦截”，用法错误用 3；stdout 仍给 JSON，stderr 同样脱敏
        secrets = [os.environ.get(SCRUBBED_ENV)]
        print(redact(f"review_preflight: {message}", secrets), file=sys.stderr)
        _emit({"kind": "review-preflight", "is_review_result": False, "ready": False, "usage_error": message}, secrets)
        sys.exit(3)

    def print_help(self, file=None):
        _emit({"kind": "review-preflight-help", "is_review_result": False, "usage": self.format_help()})


def main(argv=None) -> int:
    ap = _Parser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--source", required=True, help="控制器读取候选对象的仓库（ExternalReview.source）")
    ap.add_argument("--base", required=True, type=_sha)
    ap.add_argument("--head", required=True, type=_sha)
    ap.add_argument("--merge-base", required=True, type=_sha)
    ap.add_argument("--checkout-root", required=True, help="正式 attempt 将传给 ExternalReview 的 checkout_root")
    ap.add_argument("--image", default="sandcastle:newchanlun", help="与 external-review-worker.mts REVIEW_IMAGE 相同")
    ap.add_argument("--proxy-url", default="http://127.0.0.1:8318")  # DevSkim: ignore DS162092 有意使用本机回环（本地 CLIProxy / 测试假服务），非调试残留
    ap.add_argument("--proxy-log-dir", default=str(CLIPROXY_LOG_DIR))
    ap.add_argument("--codex-home", default=os.environ.get("CODEX_HOME", str(Path.home() / ".codex")),
                    help="宿主 Codex 配置目录；按其 model_providers.cliproxy-local 取 key")
    ap.add_argument("--codex-bin", default="codex", help="宿主使用的 codex 可执行文件，用于离线校验 provider 配置")
    ap.add_argument("--deadline", type=_deadline, default=55.0, help="总墙钟上限（秒，0 < x ≤ 60），超时的检查判失败")
    args = ap.parse_args(argv)
    t0 = time.monotonic()
    deadline = Deadline(args.deadline)
    secrets = [os.environ.get(SCRUBBED_ENV)]
    try:
        checks = run_checks(args, deadline, secrets)
    except Exception as error:  # 主流程意外异常也要给出合法报告，退出码仍是 2
        checks = [{"name": "preflight_internal", "class": "tool", "ok": False, "detail": type(error).__name__}]
    ready = all(c["ok"] for c in checks)
    _emit({"kind": "review-preflight", "is_review_result": False, "ready": ready, "checks": checks,
           "wall_seconds": round(time.monotonic() - t0, 3)}, secrets)
    return 0 if ready else 2


def run_checks(args, deadline: Deadline, secrets: list) -> list[dict]:
    credential, key = check_credential(args, deadline)
    secrets.append(key)
    checks = [credential]
    # 两次往返不留任何本地产物，放在守护线程里与空间、挂载检查并行；到时未完成即判失败、放弃等待。
    probes: dict = {}
    threads = []
    if key is not None:
        for model in MODELS:
            def probe(m=model):
                probes[m] = _guarded(f"roundtrip:{m}", "roundtrip", lambda: probe_model(args, m, key, deadline))
            threads.append(threading.Thread(target=probe, daemon=True))
            threads[-1].start()
    checks.append(_guarded("checkout_root_free_space", "space", lambda: check_space(args, deadline)))
    mount, cleanup = check_mount(args, deadline)
    checks.append(mount)
    if cleanup:
        checks.append({"name": "probe_cleanup", "class": "cleanup", "ok": False, "detail": cleanup})
    for thread in threads:
        thread.join(deadline.work_left())
    for model in MODELS:
        result = probes.get(model)
        if result is None:
            reason = "凭据不可用，未发探测" if key is None else "总时限内未完成"
            result = [{"name": f"roundtrip:{model}", "class": "roundtrip", "ok": False, "detail": reason}]
        checks.extend(result if isinstance(result, list) else [result])
    return checks


if __name__ == "__main__":
    os._exit(main())  # 不等待超时后仍挂住的探测线程
