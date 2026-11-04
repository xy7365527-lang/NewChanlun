#!/usr/bin/env python3
"""#1742 正式独审 attempt 的启动前预检。只报告环境能否启动，不是审阅结果。"""
import argparse
import json
import os
import re
import secrets
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
import uuid
from concurrent.futures import ThreadPoolExecutor, wait
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
    snapshots = (base, head, merge_base)
    objects = {line.split(b" ", 1)[0] for line in _git(source, "rev-list", "--objects", "--no-walk", *snapshots).splitlines()}
    objects.update(_git(source, "rev-list", base, head).splitlines())
    sizes = _git(source, "cat-file", "--batch-check=%(objectsize)", data=b"\n".join(sorted(objects)) + b"\n").splitlines()
    head_files = _git(source, "ls-tree", "-rl", head).splitlines()
    worktree = sum(int(line.split(b"\t", 1)[0].split()[-1]) for line in head_files if line.split()[1] == b"blob")
    return sum(int(size) for size in sizes) + worktree + SPACE_RESERVE


def _existing(path: Path) -> Path:
    while not path.exists():
        path = path.parent
    return path


def check_space(args) -> dict:
    per = snapshot_bytes(Path(args.source), args.base, args.head, args.merge_base)
    need = per * len(MODELS)
    free = shutil.disk_usage(_existing(Path(args.checkout_root))).free
    return {"name": "checkout_root_free_space", "class": "space", "ok": free >= need,
            "detail": {"required_bytes": need, "per_snapshot_bytes": per, "snapshots": len(MODELS), "free_bytes": free}}


def check_mount(args) -> dict:
    """宿主写一次性 nonce，审阅镜像经同样的只读 bind 挂载 checkout_root 读回；读不到即容器看见的不是真实文件。"""
    root = Path(args.checkout_root)
    root.mkdir(parents=True, exist_ok=True, mode=0o700)
    probe = Path(tempfile.mkdtemp(prefix="preflight-", dir=root))
    nonce = secrets.token_hex(16)
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


def _route_from_log(log_dir: Path, session: str, since: float) -> list[tuple[str, str]] | None:
    """按本次唯一 Session-Id 找 CLIProxy request-log，读出每次上游尝试的 provider 与主机。"""
    deadline = time.monotonic() + 5
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
            return list(zip(providers, hosts, strict=True)) if len(providers) == len(hosts) else []
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
    attempts = _route_from_log(Path(args.proxy_log_dir), session, since)
    provider, host = ROUTES[model]
    routed = bool(attempts) and all(a == (provider, host) for a in attempts)
    return [{"name": f"route:{model}", "class": "route", "ok": routed,
             "detail": {"expected": f"{provider}@{host}", "actual": [f"{p}@{h}" for p, h in attempts or []],
                        "log_found": attempts is not None, "session": session}},
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
    ap.add_argument("--proxy-url", default="http://127.0.0.1:8318")
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
    pool = ThreadPoolExecutor(len(jobs))
    futures = {key: pool.submit(_timed, job) for key, job in jobs.items()}
    wait(futures.values(), timeout=args.deadline)
    checks = [credential]
    for (name, kind), future in futures.items():
        if not future.done():
            checks.append({"name": name, "class": kind, "ok": False, "detail": f"超过 {args.deadline}s 总时限未完成"})
            continue
        try:
            checks.extend(future.result())
        except Exception as error:  # 检查自身失败按未通过处理，不放行。
            checks.append({"name": name, "class": kind, "ok": False, "detail": f"{type(error).__name__}: {error}"[:500]})
    pool.shutdown(wait=False, cancel_futures=True)
    ready = all(c["ok"] for c in checks)
    report = {"kind": "review-preflight", "is_review_result": False, "ready": ready, "checks": checks,
              "wall_seconds": round(time.monotonic() - t0, 3)}
    print(json.dumps(report, ensure_ascii=False), flush=True)
    return 0 if ready else 2


if __name__ == "__main__":
    # 不等待超时后仍挂住的探测线程。
    os._exit(main())
