#!/usr/bin/env python3
"""#1742 正式独审 attempt 的启动前预检。只报告环境能否启动，不是审阅结果。"""
import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
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
SCRUBBED_ENV = "CLIPROXY_LOCAL_KEY"


def _git(source: Path, *args: str, data: bytes | None = None) -> bytes:
    return subprocess.run(["git", "-C", str(source), *args], input=data, capture_output=True, check=True).stdout


def snapshot_bytes(source: Path, base: str, head: str, merge_base: str) -> int:
    """单个模型审阅快照的所需字节，逐项复刻 external_review.prepare_review 的公式。三条互不依赖的 git 读并发执行。"""
    def start(*args: str) -> subprocess.Popen:
        return subprocess.Popen(["git", "-C", str(source), *args], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)

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
    return {"name": "checkout_root_free_space", "class": "space", "ok": free >= need,
            "detail": {"required_bytes": need, "per_snapshot_bytes": per, "snapshots": len(MODELS), "free_bytes": free}}


def check_mount(args) -> dict:
    """宿主写一次性 nonce，审阅镜像经同样的只读 bind 挂载 checkout_root 读回；读不到即容器看见的不是真实文件。"""
    root = Path(args.checkout_root)
    root.mkdir(parents=True, exist_ok=True, mode=0o700)
    probe = Path(tempfile.mkdtemp(prefix="preflight-", dir=root))
    nonce = os.urandom(16).hex()
    try:
        (probe / "nonce").write_text(nonce)
        command = ["docker", "run", "--rm", "--network", "none", "--read-only",
                   "--mount", f"type=bind,src={root},dst=/checkouts,readonly",
                   "--entrypoint", "cat", args.image, f"/checkouts/{probe.name}/nonce"]
        result = subprocess.run(command, capture_output=True, text=True, timeout=30)
    finally:
        shutil.rmtree(probe, ignore_errors=True)
    seen = result.stdout.strip()
    return {"name": "checkout_root_container_mount", "class": "mount", "ok": result.returncode == 0 and seen == nonce,
            "detail": {"checkout_root": str(root), "image": args.image, "docker_rc": result.returncode,
                       "nonce_match": seen == nonce, "stderr": result.stderr.strip()[-300:]}}


class CredentialError(RuntimeError):
    pass


def provider_key(codex_home: Path, provider: str = PROVIDER) -> tuple[str, dict]:
    """按 Codex 宿主的 provider 配置取 key，且进程环境去掉 CLIPROXY_LOCAL_KEY（#1323 宿主就缺这个变量）。"""
    config = tomllib.loads((codex_home / "config.toml").read_text())
    entry = (config.get("model_providers") or {}).get(provider)
    if entry is None:
        raise CredentialError(f"{codex_home}/config.toml 没有 model_providers.{provider}")
    env = {k: v for k, v in os.environ.items() if k != SCRUBBED_ENV}
    if isinstance(entry.get("auth"), dict) and entry["auth"].get("command"):
        auth = entry["auth"]
        result = subprocess.run([auth["command"], *auth.get("args", [])], env=env, capture_output=True, text=True,
                                timeout=auth.get("timeout_ms", 5000) / 1000)
        key = result.stdout.strip()
        if result.returncode != 0 or not key:
            raise CredentialError(f"auth.command 退出 {result.returncode} 或输出为空")
        return key, {"source": "auth.command", "command": auth["command"]}
    if entry.get("experimental_bearer_token"):
        return entry["experimental_bearer_token"], {"source": "experimental_bearer_token"}
    if entry.get("env_key"):
        if not env.get(entry["env_key"]):
            raise CredentialError(f"Missing environment variable: {entry['env_key']}")
        return env[entry["env_key"]], {"source": "env_key", "env_key": entry["env_key"]}
    raise CredentialError(f"model_providers.{provider} 未配置任何凭据来源")


def check_credential(args) -> tuple[dict, str | None]:
    try:
        key, source = provider_key(Path(args.codex_home))
    except (CredentialError, OSError, subprocess.TimeoutExpired, tomllib.TOMLDecodeError) as error:
        return {"name": f"credential:{PROVIDER}", "class": "credential", "ok": False,
                "detail": {"scrubbed_env": SCRUBBED_ENV, "error": f"{type(error).__name__}: {error}"[:300]}}, None
    return {"name": f"credential:{PROVIDER}", "class": "credential", "ok": True,
            "detail": {"scrubbed_env": SCRUBBED_ENV, **source}}, key


def _round_trip(url: str, key: str, model: str, session: str) -> dict:
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
            terminal, detail = None, "流在终态事件前结束"
            for raw in response:
                if time.monotonic() > deadline:
                    detail = f"超过 {PROBE_TIMEOUT}s 仍无终态事件"
                    break
                line = raw.decode("utf-8", "replace").strip()
                if line.startswith("data:"):
                    event = json.loads(line[5:]) if line[5:].strip().startswith("{") else {}
                    if event.get("type") in ("response.completed", "response.incomplete", "response.failed", "error"):
                        terminal = event.get("type")
                        detail = (event.get("response") or {}).get("error") or event.get("error") or event.get("message")
                        break
            return {"http_status": response.status, "terminal": terminal,
                    "ok": terminal in ("response.completed", "response.incomplete"),
                    "error": None if terminal in ("response.completed", "response.incomplete") else str(detail)[:300]}
    except urllib.error.HTTPError as error:
        return {"http_status": error.code, "terminal": None, "ok": False, "error": error.read().decode("utf-8", "replace")[:300]}
    except (urllib.error.URLError, TimeoutError, OSError) as error:
        return {"http_status": None, "terminal": None, "ok": False, "error": f"{type(error).__name__}: {error}"[:300]}


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
    """按本次唯一 Session-Id 找 CLIProxy request-log，读出每次上游尝试的 provider 与主机，以及实际服务模型。"""
    deadline = time.monotonic() + wait
    while True:
        for path in sorted(log_dir.glob("v1-responses-*.log"), key=lambda q: q.stat().st_mtime, reverse=True):
            if path.stat().st_mtime < since - 1:
                break
            text = path.read_text(errors="replace")
            if f"Session-Id: {session}" not in text:
                continue
            providers = re.findall(r"^Auth: provider=([\w.-]+)", text, re.M)
            hosts = re.findall(r"^Upstream URL: https?://([^/:?\s]+)", text, re.M)
            # 行数不齐说明日志形状变了，返回空表让路由检查判失败，不截断猜测。
            attempts = list(zip(providers, hosts, strict=True)) if len(providers) == len(hosts) else []
            return attempts, served_models(text)
        if time.monotonic() > deadline:
            return None
        time.sleep(0.25)


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
    # proxy 会给 Claude 请求注入 fallbacks（如 claude-opus-4-8）；实际服务模型与请求不符即拦。日志无响应体时记为未核实。
    return [{"name": f"route:{model}", "class": "route", "ok": routed,
             "detail": {"expected": f"{provider}@{host}", "actual": [f"{p}@{h}" for p, h in attempts or []],
                        "log_found": found is not None, "session": session}},
            {"name": f"identity:{model}", "class": "identity", "ok": served <= {model},
             "detail": {"requested": model, "served": sorted(served), "verified": bool(served)}},
            {"name": f"roundtrip:{model}", "class": "roundtrip", "ok": trip["ok"], "detail": trip}]


def _timed(job) -> list[dict]:
    start = time.monotonic()
    checks = job()
    for check in checks:
        check["elapsed_s"] = round(time.monotonic() - start, 3)
    return checks


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
    ap.add_argument("--deadline", type=float, default=55.0, help="总墙钟上限（秒），超时的检查判失败")
    args = ap.parse_args(argv)
    t0 = time.monotonic()
    credential, key = check_credential(args)
    # 键为该 job 超时或异常时上报的检查名与类别。
    jobs = {("checkout_root_free_space", "space"): lambda: [check_space(args)],
            ("checkout_root_container_mount", "mount"): lambda: [check_mount(args)],
            **{(f"roundtrip:{model}", "roundtrip"): (lambda m=model: probe_model(args, m, key)) for model in MODELS}}
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
        thread.join(max(0.0, args.deadline - (time.monotonic() - t0)))
    checks = [credential]
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
    print(json.dumps(report, ensure_ascii=False), flush=True)
    return 0 if ready else 2


if __name__ == "__main__":
    # 不等待超时后仍挂住的探测线程。
    os._exit(main())
