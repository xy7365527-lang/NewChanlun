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

    def __init__(self, log_dir, routes, *, status=None, write_log=True, hang=False, log_delay=0.0, served=None,
                 echo_auth=False):
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
                    path = log_dir / f"v1-responses-{time.time_ns()}.log"
                    text = (f"=== HEADERS ===\nSession-Id: {self.headers['Session-Id']}\n\n=== API REQUEST 1 ===\n"
                            f"Upstream URL: {upstream}\nAuth: provider={provider}, auth_id=x, label=x, type=oauth\n"
                            "\n=== API RESPONSE 1 ===\nStatus: 200\n")
                    model_served = (served or {}).get(body["model"])
                    if model_served:
                        text += 'data: {"type":"message_start","message":{"model":"%s"}}\n' % model_served
                    if log_delay:
                        threading.Timer(log_delay, path.write_text, args=(text,)).start()
                    else:
                        path.write_text(text)
                if hang:
                    self.send_response(200)
                    self.send_header("Content-Type", "text/event-stream")
                    self.end_headers()
                    self.wfile.flush()
                    time.sleep(4)
                    return
                if code != 200 and echo_auth:
                    payload = json.dumps({"error": {"message": f"bad header {self.headers['Authorization']}"}}).encode()
                elif code != 200:
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
                               "--proxy-log-dir", str(logs), "--codex-home", str(codex),
                               "--proxy-config", str(tmp_path / "cliproxy-config.yaml"), *extra])
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


MODELFLARE = ("openai-compatible-modelflare-codex", "https://modelflare.dev/v1/responses")


def test_sol_on_third_party_codex_route_blocks(env, capsys):
    """main.log 出现过 gpt-6.1-sol 被选到 modelflare-codex 并 403 预扣额度失败。"""
    code, report, *_ = env(routes={**OFFICIAL, SOL: MODELFLARE}, status={SOL: 403}, capsys=capsys)
    assert code == 2 and failing(report) == {f"route:{SOL}", f"roundtrip:{SOL}"}


def test_proxy_down_blocks_without_waiting_out_the_log_window(env, capsys):
    start = time.monotonic()
    # 后给的 --proxy-url 覆盖 fixture 的假 proxy，指向无人监听的端口。
    code, report, *_ = env(capsys=capsys, extra=("--proxy-url", "http://127.0.0.1:9"))  # DevSkim: ignore DS162092 有意使用本机回环（本地 CLIProxy / 测试假服务），非调试残留
    assert code == 2 and {f"roundtrip:{SOL}", f"roundtrip:{OPUS}"} <= failing(report)
    assert time.monotonic() - start < 2


def test_request_log_written_one_second_late_still_passes(env, capsys):
    code, report, *_ = env(log_delay=1.0, capsys=capsys)
    assert code == 0 and failing(report) == set()


def test_request_log_disabled_in_proxy_config_blocks_at_once(env, capsys, tmp_path):
    (tmp_path / "cliproxy-config.yaml").write_text("observability:\n    logs:\n        request-log: false\n")
    start = time.monotonic()
    code, report, *_ = env(write_log=False, capsys=capsys)
    assert time.monotonic() - start < 1.5
    assert code == 2 and failing(report) == {f"route:{SOL}", f"route:{OPUS}"}
    assert all("request-log=false" in c["detail"]["error"] for c in report["checks"] if c["name"].startswith("route:"))


@pytest.mark.parametrize("text, expected", [
    ("observability:\n    logs:\n        request-log: true\n", True),
    ("observability:\n    logs:\n        request-log: false  # off\n", False),
    ("logs:\n  request-log: false\n", None),
    ("observability:\n    pprof:\n        request-log: false\n", None),
])
def test_request_log_setting_is_read_from_its_own_section(tmp_path, text, expected):
    config = tmp_path / "config.yaml"
    config.write_text(text)
    assert preflight.request_log_enabled(config) is expected
    assert preflight.request_log_enabled(tmp_path / "missing.yaml") is None


def test_opus_silently_served_by_fallback_model_blocks(env, capsys):
    """proxy 给 Claude 请求注入 fallbacks:[claude-opus-4-8]；上游实际由 4.8 服务时必须拦。"""
    code, report, *_ = env(served={OPUS: "claude-opus-4-8", SOL: SOL}, capsys=capsys)
    assert code == 2 and failing(report) == {f"identity:{OPUS}"}
    identity = next(c for c in report["checks"] if c["name"] == f"identity:{OPUS}")
    assert identity["detail"] == {"requested": OPUS, "served": ["claude-opus-4-8"], "verified": True}


def test_served_model_matching_request_passes_and_is_marked_verified(env, capsys):
    code, report, *_ = env(served={OPUS: OPUS, SOL: SOL}, capsys=capsys)
    assert code == 0
    assert all(c["detail"]["verified"] for c in report["checks"] if c["class"] == "identity")


def test_served_models_reads_both_stream_shapes():
    text = ("=== API RESPONSE 1 ===\nStatus: 200\n"
            'data: {"type":"message_start","message":{"model":"claude-opus-5-5"}}\n'
            'data: {"type":"response.created","response":{"model":"gpt-6.1-sol","tools":[{"model":"gpt-image-2-codex"}]}}\n'
            "\n=== RESPONSE ===\n"
            'data: {"type":"response.completed","response":{"model":"client-side-echo"}}\n')
    assert preflight.served_models(text) == {"claude-opus-5-5", "gpt-6.1-sol"}


def test_target_missing_from_source_reports_needs_source_sync(env, capsys, repo):
    root, base, head = repo
    absent = "1" * 40
    code, report, *_ = env(capsys=capsys, extra=("--head", absent))
    assert code == 2 and "source_objects" in failing(report)
    source = next(c for c in report["checks"] if c["name"] == "source_objects")
    assert source["class"] == "source" and source["detail"]["needs_source_sync"] == [absent]


def test_auth_command_combined_with_env_key_blocks_like_codex(env, capsys, tmp_path):
    """Codex 拒绝 provider 同时配 auth 与 env_key；预检不能因 helper 能取到 key 就放行。"""
    config = tmp_path / "codex/config.toml"
    config.write_text(config.read_text().replace("[model_providers.cliproxy-local.auth]",
                                                 'env_key = "CLIPROXY_LOCAL_KEY"\n[model_providers.cliproxy-local.auth]'))
    code, report, _, proxy = env(capsys=capsys)
    assert code == 2 and f"credential:{preflight.PROVIDER}" in failing(report) and proxy.requests == []
    credential = next(c for c in report["checks"] if c["class"] == "credential")
    assert "cannot be combined with env_key" in credential["detail"]["error"]


def _auth_command(tmp_path, command: str, args: list[str], timeout_ms: int = 5000) -> None:
    (tmp_path / "codex/config.toml").write_text(
        '[model_providers.cliproxy-local]\nbase_url = "http://localhost:8318/v1"\n'
        f'[model_providers.cliproxy-local.auth]\ncommand = "{command}"\nargs = {json.dumps(args)}\ntimeout_ms = {timeout_ms}\n')


def test_credential_helper_timeout_does_not_leak_its_arguments(env, capsys, tmp_path):
    _auth_command(tmp_path, "/bin/sh", ["-c", f"sleep 5 # {KEY}"], timeout_ms=200)
    code, report, out, _ = env(capsys=capsys)
    assert code == 2 and KEY not in out
    assert "未返回" in next(c for c in report["checks"] if c["class"] == "credential")["detail"]["error"]


def test_credential_helper_respects_total_deadline(env, capsys, tmp_path):
    _auth_command(tmp_path, "/bin/sh", ["-c", "sleep 3; echo key"], timeout_ms=120000)
    start = time.monotonic()
    code, report, *_ = env(capsys=capsys, extra=("--deadline", "0.5"))
    assert time.monotonic() - start < 2 and code == 2


def test_key_echoed_by_upstream_error_is_redacted(env, capsys):
    code, report, out, _ = env(status={OPUS: 400}, echo_auth=True, capsys=capsys)
    assert code == 2 and KEY not in out and "<redacted>" in out


def test_cli_deadline_exit_reclaims_probe_dir_and_container(tmp_path, repo):
    """真实 CLI 退出路径（os._exit）：docker 卡住、总时限先到时，探针目录与容器都要回收。"""
    import subprocess
    import sys
    root, base, head = repo
    bin_dir, checkouts, calls = tmp_path / "bin", tmp_path / "checkouts", tmp_path / "docker-calls"
    bin_dir.mkdir()
    (bin_dir / "docker").write_text(f'#!/bin/sh\necho "$@" >> {calls}\n[ "$1" = rm ] && exit 0\nsleep 3\n')
    (bin_dir / "docker").chmod(0o755)
    codex = tmp_path / "codex"
    codex.mkdir()
    (codex / "config.toml").write_text('[model_providers.cliproxy-local]\nexperimental_bearer_token = "x"\n')
    result = subprocess.run(
        [sys.executable, "-I", preflight.__file__, "--source", str(root), "--base", base, "--head", head,
         "--merge-base", base, "--checkout-root", str(checkouts), "--proxy-url", "http://localhost:9",
         "--proxy-log-dir", str(tmp_path), "--proxy-config", str(tmp_path / "none.yaml"),
         "--codex-home", str(codex), "--deadline", "0.5"],
        capture_output=True, text=True, timeout=20, env={"PATH": f"{bin_dir}:/usr/bin:/bin", "HOME": str(tmp_path)})
    assert result.returncode == 2
    assert [p.name for p in checkouts.iterdir()] == []
    lines = calls.read_text().splitlines()
    name = next(a.split()[a.split().index("--name") + 1] for a in lines if a.startswith("run "))
    assert f"rm -f {name}" in lines
