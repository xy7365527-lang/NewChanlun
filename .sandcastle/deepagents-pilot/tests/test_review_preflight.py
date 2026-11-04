import json
import re
import threading
import time
from collections import namedtuple
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

import pytest
from conftest import git

import review_preflight as preflight
from pilot1742 import external_review
from pilot1742.github_app import IdentityError

KEY = "local-test-key-must-not-leak"
SOL, OPUS = "gpt-6.1-sol", "claude-opus-5-5"
OFFICIAL = {SOL: ("codex", "https://chatgpt.com/backend-api/codex/responses"),
            OPUS: ("claude", "https://api.anthropic.com/v1/messages")}
ANTIGRAVITY = ("antigravity", "https://daily-cloudcode-pa.googleapis.com/v1internal:streamGenerateContent")
Usage = namedtuple("Usage", "total used free")


@pytest.fixture
def repo(tmp_path):
    root = tmp_path / "source"
    root.mkdir()
    git(root, "init", "-q", "-b", "main")
    git(root, "config", "user.name", "Fixture")
    git(root, "config", "user.email", "fixture@example.invalid")
    (root / "a.txt").write_text("base\n" * 100)
    git(root, "add", ".")
    git(root, "commit", "-qm", "base")
    base = git(root, "rev-parse", "HEAD")
    (root / "b.txt").write_text("head\n" * 50)
    git(root, "add", ".")
    git(root, "commit", "-qm", "head")
    return root, base, git(root, "rev-parse", "HEAD")


class FakeProxy:
    """同形 CLIProxy：按模型选 provider，写 request-log，记录收到的请求。"""

    def __init__(self, log_dir, routes, *, status=None, write_log=True, hang=False):
        self.requests = []
        proxy = self

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *args):
                pass

            def do_POST(self):
                body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
                proxy.requests.append({"body": body, "auth": self.headers["Authorization"]})
                if self.headers["Authorization"] != f"Bearer {KEY}":
                    self.send_response(401)
                    self.send_header("Content-Length", "0")
                    self.end_headers()
                    return
                provider, upstream = routes[body["model"]]
                code = (status or {}).get(body["model"], 200)
                if write_log:
                    (log_dir / f"v1-responses-{time.time_ns()}.log").write_text(
                        f"=== HEADERS ===\nSession-Id: {self.headers['Session-Id']}\n\n=== API REQUEST 1 ===\n"
                        f"Upstream URL: {upstream}\nAuth: provider={provider}, auth_id=x, label=x, type=oauth\n")
                if hang:
                    self.send_response(200)
                    self.send_header("Content-Type", "text/event-stream")
                    self.end_headers()
                    self.wfile.flush()
                    time.sleep(4)
                    return
                if code != 200:
                    payload = b'{"error":{"message":"tool_choice: type \\"tool\\" and \\"any\\" are not supported"}}'
                else:
                    payload = b'event: response.completed\ndata: {"type":"response.completed","response":{}}\n\n'
                self.send_response(code)
                self.send_header("Content-Length", str(len(payload)))
                self.end_headers()
                self.wfile.write(payload)

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)  # DevSkim: ignore DS162092 有意使用本机回环（本地 CLIProxy / 测试假服务），非调试残留
        self.url = f"http://127.0.0.1:{self.server.server_address[1]}"  # DevSkim: ignore DS162092 有意使用本机回环（本地 CLIProxy / 测试假服务），非调试残留
        threading.Thread(target=self.server.serve_forever, daemon=True).start()

    def close(self):
        self.server.shutdown()
        self.server.server_close()


@pytest.fixture
def env(tmp_path, monkeypatch, repo):
    """宿主 checkout_root、假 docker（按 bind 源映射读文件）与 proxy key。"""
    root, base, head = repo
    bin_dir, logs, checkouts = tmp_path / "bin", tmp_path / "logs", tmp_path / "checkouts"
    for d in (bin_dir, logs):
        d.mkdir()
    docker = bin_dir / "docker"
    docker.write_text("#!/bin/sh\n"
                      "src=$(printf '%s\\n' \"$@\" | sed -n 's/^type=bind,src=\\(.*\\),dst=\\/checkouts,readonly$/\\1/p')\n"
                      "[ -n \"$FAKE_DOCKER_EMPTY\" ] && src=/nonexistent\n"
                      "for last; do :; done\n"
                      "exec cat \"$src${last#/checkouts}\"\n")
    docker.chmod(0o755)
    monkeypatch.setenv("PATH", f"{bin_dir}:/usr/bin:/bin")
    # 宿主 provider 按 auth.command 从文件取 key；进程环境里的 CLIPROXY_LOCAL_KEY 是错值，必须被忽略。
    monkeypatch.setenv("CLIPROXY_LOCAL_KEY", "stale-env-value")
    codex = tmp_path / "codex"
    codex.mkdir()
    (tmp_path / "key.txt").write_text(KEY + "\n")
    (codex / "config.toml").write_text(
        '[model_providers.cliproxy-local]\nbase_url = "http://127.0.0.1:8318/v1"\nwire_api = "responses"\n'  # DevSkim: ignore DS162092 有意使用本机回环（本地 CLIProxy / 测试假服务），非调试残留
        f'[model_providers.cliproxy-local.auth]\ncommand = "/bin/cat"\nargs = ["{tmp_path / "key.txt"}"]\n')
    proxies = []

    def run(routes=None, capsys=None, extra=(), **proxy_options):
        proxy = FakeProxy(logs, routes or OFFICIAL, **proxy_options)
        proxies.append(proxy)
        code = preflight.main(["--source", str(root), "--base", base, "--head", head, "--merge-base", base,
                               "--checkout-root", str(checkouts), "--proxy-url", proxy.url,
                               "--proxy-log-dir", str(logs), "--codex-home", str(codex), *extra])
        out = capsys.readouterr().out
        return code, json.loads(out), out, proxy

    yield run
    for proxy in proxies:
        proxy.close()


def failing(report):
    return {c["name"] for c in report["checks"] if not c["ok"]}


def test_snapshot_bytes_equal_to_controller_formula(repo, tmp_path, monkeypatch):
    root, base, head = repo
    target = external_review.ReviewTarget(1, 7, 1, "main", base, 1, "topic", head, base, "run", 1,
                                          external_review.POLICY, head, 0)
    monkeypatch.setattr(external_review.shutil, "disk_usage", lambda _: Usage(0, 0, 0))
    with pytest.raises(IdentityError) as error:
        external_review.prepare_review(target, root, tmp_path / "checkout")
    required = int(re.search(r"需要 (\d+) bytes", str(error.value)).group(1))
    assert preflight.snapshot_bytes(root, base, head, base) == required


def test_healthy_environment_is_ready_and_not_a_review(env, capsys):
    code, report, out, proxy = env(capsys=capsys)
    assert code == 0 and report["ready"] and failing(report) == set()
    assert report["is_review_result"] is False and report["kind"] == "review-preflight"
    assert KEY not in out
    assert sorted(r["body"]["model"] for r in proxy.requests) == sorted([SOL, OPUS])
    assert all(r["body"]["tool_choice"] == "auto" and r["body"]["tools"] for r in proxy.requests)
    assert all(r["auth"] == f"Bearer {KEY}" for r in proxy.requests)


def test_space_below_two_snapshots_blocks(env, capsys, repo, monkeypatch):
    root, base, head = repo
    per = preflight.snapshot_bytes(root, base, head, base)
    monkeypatch.setattr(preflight.shutil, "disk_usage", lambda _: Usage(0, 0, 2 * per - 1))
    code, report, *_ = env(capsys=capsys)
    assert code == 2 and failing(report) == {"checkout_root_free_space"}
    monkeypatch.setattr(preflight.shutil, "disk_usage", lambda _: Usage(0, 0, 2 * per))
    assert env(capsys=capsys)[0] == 0


def test_container_seeing_empty_mount_blocks(env, capsys, monkeypatch):
    monkeypatch.setenv("FAKE_DOCKER_EMPTY", "1")
    code, report, *_ = env(capsys=capsys)
    assert code == 2 and failing(report) == {"checkout_root_container_mount"}


def test_probe_leaves_no_files_in_checkout_root(env, capsys, tmp_path):
    env(capsys=capsys)
    assert list((tmp_path / "checkouts").iterdir()) == []


def test_opus_on_antigravity_with_upstream_400_blocks(env, capsys):
    code, report, *_ = env(routes={**OFFICIAL, OPUS: ANTIGRAVITY}, status={OPUS: 400}, capsys=capsys)
    assert code == 2 and failing(report) == {f"route:{OPUS}", f"roundtrip:{OPUS}"}
    route = next(c for c in report["checks"] if c["name"] == f"route:{OPUS}")
    assert route["detail"]["actual"] == ["antigravity@daily-cloudcode-pa.googleapis.com"]


def test_wrong_provider_blocks_even_when_round_trip_succeeds(env, capsys):
    code, report, *_ = env(routes={**OFFICIAL, OPUS: ANTIGRAVITY}, capsys=capsys)
    assert code == 2 and failing(report) == {f"route:{OPUS}"}


def test_route_unknown_without_request_log_blocks(env, capsys):
    code, report, *_ = env(write_log=False, capsys=capsys)
    assert code == 2 and failing(report) == {f"route:{SOL}", f"route:{OPUS}"}
    assert all(not c["detail"]["log_found"] for c in report["checks"] if c["name"].startswith("route:"))


def test_hanging_upstream_is_cut_at_deadline(env, capsys, monkeypatch):
    monkeypatch.setattr(preflight, "PROBE_TIMEOUT", 3)
    start = time.monotonic()
    code, report, *_ = env(hang=True, extra=("--deadline", "2"), capsys=capsys)
    assert time.monotonic() - start < 4
    assert code == 2 and {f"roundtrip:{SOL}", f"roundtrip:{OPUS}"} <= failing(report)


def test_env_key_provider_without_variable_blocks_before_any_probe(env, capsys, tmp_path, monkeypatch):
    """#1323：宿主 provider 只认 env_key 时，去掉 CLIPROXY_LOCAL_KEY 即报 Missing environment variable。"""
    (tmp_path / "codex/config.toml").write_text(
        '[model_providers.cliproxy-local]\nbase_url = "http://127.0.0.1:8318/v1"\nenv_key = "CLIPROXY_LOCAL_KEY"\n')  # DevSkim: ignore DS162092 有意使用本机回环（本地 CLIProxy / 测试假服务），非调试残留
    monkeypatch.setenv("CLIPROXY_LOCAL_KEY", KEY)  # 预检进程里有也不算：宿主进程里没有
    code, report, out, proxy = env(capsys=capsys)
    assert code == 2 and f"credential:{preflight.PROVIDER}" in failing(report)
    credential = next(c for c in report["checks"] if c["class"] == "credential")
    assert "Missing environment variable: CLIPROXY_LOCAL_KEY" in credential["detail"]["error"]
    assert proxy.requests == [] and KEY not in out


def test_auth_command_failure_blocks(env, capsys, tmp_path):
    (tmp_path / "key.txt").unlink()
    code, report, *_ = env(capsys=capsys)
    assert code == 2 and f"credential:{preflight.PROVIDER}" in failing(report)
