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
OTHER_VALID = "second-accepted-client-key"
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
                 echo_auth=False, terminal=None, header_first=0.0, split_at=None, error_body=None,
                 stream_model=None):
        self.requests = []
        proxy = self

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *args):
                pass

            def do_GET(self):
                code = 200 if self.headers.get("Authorization") in (f"Bearer {KEY}", f"Bearer {OTHER_VALID}") else 401
                self.send_response(code)
                self.send_header("Content-Length", "0")
                self.end_headers()

            def do_POST(self):
                body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
                proxy.requests.append({"body": body, "auth": self.headers["Authorization"]})
                if self.headers["Authorization"] not in (f"Bearer {KEY}", f"Bearer {OTHER_VALID}"):
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
                    text += "\n=== RESPONSE ===\nStatus: 200\n"
                    if split_at and split_at in text:
                        # 分段写入：先落盘到 split_at 之前（可能截在一行中间），0.3 秒后补全
                        path.write_text(text[:text.index(split_at)])
                        threading.Timer(0.3, path.write_text, args=(text,)).start()
                    elif header_first:
                        # 先只落盘会话头，过一会儿再补全上游段（分段写入）
                        path.write_text(text.split("\n\n=== API REQUEST")[0] + "\n")
                        threading.Timer(header_first, path.write_text, args=(text,)).start()
                    elif log_delay:
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
                if code != 200 and error_body is not None:
                    payload = error_body(self.headers["Authorization"]).encode()
                elif code != 200 and echo_auth:
                    payload = json.dumps({"error": {"message": f"bad header {self.headers['Authorization']}"}}).encode()
                elif code != 200:
                    payload = b'{"error":{"message":"tool_choice: type \\"tool\\" and \\"any\\" are not supported"}}'
                else:
                    payload = (terminal or b'event: response.completed\ndata: {"type":"response.completed","response":{"model":"{model}"}}\n\n')
                    payload = payload.replace(b"{model}", (stream_model or {}).get(body["model"], body["model"]).encode())
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
                      "[ \"$1\" = rm ] && exit 0\n"
                      "[ \"$1\" = ps ] && exit 0\n"
                      "src=$(printf '%s\\n' \"$@\" | sed -n 's/^type=bind,src=\\(.*\\),dst=\\/checkouts,readonly$/\\1/p')\n"
                      "case \"$src\" in /*) ;; *) echo 'mount path must be absolute' >&2; exit 125;; esac\n"
                      "[ -n \"$FAKE_DOCKER_EMPTY\" ] && src=/nonexistent\n"
                      "for last; do :; done\n"
                      "exec cat \"$src${last#/checkouts}\"\n")
    docker.chmod(0o755)
    monkeypatch.setenv("PATH", f"{bin_dir}:/usr/bin:/bin")
    # 测试不依赖宿主真实剩余空间；专测空间的用例自行覆盖。
    monkeypatch.setattr(preflight.shutil, "disk_usage", lambda _: Usage(0, 0, 1 << 50))
    # 正式 worker 从进程环境取 key（launcher 传入）；宿主 provider 按 auth.command 从文件取同一个 key。
    monkeypatch.setenv("CLIPROXY_LOCAL_KEY", KEY)
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
        config = codex / "config.toml"
        previous = [f"{p.url}/v1" for p in proxies[:-1]] + ["http://127.0.0.1:8318/v1"]  # DevSkim: ignore DS162092 有意使用本机回环（本地 CLIProxy / 测试假服务），非调试残留
        data = config.read_bytes()  # 按字节替换：有的用例故意写入非 UTF-8 配置
        for old_url in previous:
            data = data.replace(old_url.encode(), f"{proxy.url}/v1".encode())
        config.write_bytes(data)  # DevSkim: ignore DS162092 有意使用本机回环（本地 CLIProxy / 测试假服务），非调试残留
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
    # 往返失败没有服务模型证据，identity 按无从核实一并拦（独审五）
    assert code == 2 and failing(report) == {f"route:{OPUS}", f"roundtrip:{OPUS}", f"identity:{OPUS}"}
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
    assert code == 2 and failing(report) == {"credential:codex-host"}
    credential = next(c for c in report["checks"] if c["name"] == "credential:codex-host")
    assert "Missing environment variable: CLIPROXY_LOCAL_KEY" in credential["detail"]["error"]
    assert KEY not in out


def test_auth_command_failure_blocks(env, capsys, tmp_path):
    (tmp_path / "key.txt").unlink()
    code, report, *_ = env(capsys=capsys)
    assert code == 2 and "credential:codex-host" in failing(report)


MODELFLARE = ("openai-compatible-modelflare-codex", "https://modelflare.dev/v1/responses")


def test_sol_on_third_party_codex_route_blocks(env, capsys):
    """main.log 出现过 gpt-6.1-sol 被选到 modelflare-codex 并 403 预扣额度失败。"""
    code, report, *_ = env(routes={**OFFICIAL, SOL: MODELFLARE}, status={SOL: 403}, capsys=capsys)
    assert code == 2 and failing(report) == {f"route:{SOL}", f"roundtrip:{SOL}", f"identity:{SOL}"}


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
    assert identity["detail"]["served"] == ["claude-opus-4-8", OPUS] and identity["detail"]["verified"]


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
    assert code == 2 and failing(report) == {"credential:codex-host"}
    credential = next(c for c in report["checks"] if c["name"] == "credential:codex-host")
    assert "cannot be combined with env_key" in credential["detail"]["error"]


def _auth_command(tmp_path, command: str, args: list[str], timeout_ms: int = 5000) -> None:
    (tmp_path / "codex/config.toml").write_text(
        '[model_providers.cliproxy-local]\nbase_url = "http://localhost:8318/v1"\n'  # DevSkim: ignore DS162092 有意使用本机回环（本地 CLIProxy / 测试假服务），非调试残留
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
    (bin_dir / "docker").write_text(f'#!/bin/sh\necho "$@" >> {calls}\n[ "$1" = rm ] && exit 0\nsleep 5\n')
    (bin_dir / "docker").chmod(0o755)
    (bin_dir / "launchctl").write_text("#!/bin/sh\nexit 0\n")  # 不碰本机真实 launchctl
    (bin_dir / "launchctl").chmod(0o755)
    codex = tmp_path / "codex"
    codex.mkdir()
    (codex / "config.toml").write_text('[model_providers.cliproxy-local]\nexperimental_bearer_token = "x"\n')
    result = subprocess.run(
        [sys.executable, "-I", preflight.__file__, "--source", str(root), "--base", base, "--head", head,
         "--merge-base", base, "--checkout-root", str(checkouts), "--proxy-url", "http://localhost:9",  # DevSkim: ignore DS162092 有意使用本机回环（本地 CLIProxy / 测试假服务），非调试残留
         "--proxy-log-dir", str(tmp_path), "--proxy-config", str(tmp_path / "none.yaml"),
         "--codex-home", str(codex), "--deadline", "2"],
        capture_output=True, text=True, timeout=20, env={"PATH": f"{bin_dir}:/usr/bin:/bin", "HOME": str(tmp_path)})
    assert result.returncode == 2
    assert [p.name for p in checkouts.iterdir()] == []
    lines = calls.read_text().splitlines()
    name = next(a.split()[a.split().index("--name") + 1] for a in lines if a.startswith("run "))
    assert f"rm -f {name}" in lines


def test_completed_event_with_failed_status_is_not_success(env, capsys):
    bad = b'data: {"type":"response.completed","response":{"status":"failed","error":{"message":"overloaded"}}}\n\n'
    code, report, *_ = env(terminal=bad, capsys=capsys)
    assert code == 2 and {f"roundtrip:{SOL}", f"roundtrip:{OPUS}"} <= failing(report)


def test_incomplete_without_error_still_passes(env, capsys):
    ok = b'data: {"type":"response.incomplete","response":{"model":"{model}","status":"incomplete","incomplete_details":{"reason":"max_output_tokens"}}}\n\n'
    code, report, *_ = env(terminal=ok, capsys=capsys)
    assert code == 0


def test_log_written_in_two_steps_is_not_misread(env, capsys):
    code, report, *_ = env(header_first=0.3, capsys=capsys)
    assert code == 0 and failing(report) == set()


def test_unrelated_log_vanishing_mid_scan_does_not_block(env, capsys, tmp_path, monkeypatch):
    original = preflight.Path.stat
    def flaky(self, *a, **k):
        if self.name.startswith("v1-responses-gone"):
            raise FileNotFoundError(self)
        return original(self, *a, **k)
    (tmp_path / "logs" / "v1-responses-gone.log").write_text("x")
    monkeypatch.setattr(preflight.Path, "stat", flaky)
    code, report, *_ = env(capsys=capsys)
    assert code == 0 and failing(report) == set()


def test_snapshot_bytes_match_controller_with_replace_refs(repo, tmp_path, monkeypatch):
    """控制器不跟随 replace 引用；预检若跟随，同一输入字节数会不同（独审第 7 条）。"""
    root, base, head = repo
    (root / "big.txt").write_text("y" * 200000)
    git(root, "add", "big.txt")
    git(root, "commit", "-qm", "replacement")
    replacement = git(root, "rev-parse", "HEAD")
    git(root, "reset", "-q", "--hard", head)
    git(root, "replace", head, replacement)
    target = external_review.ReviewTarget(1, 7, 1, "main", base, 1, "topic", head, base, "run", 1,
                                          external_review.POLICY, head, 0)
    monkeypatch.setattr(external_review.shutil, "disk_usage", lambda _: Usage(0, 0, 0))
    with pytest.raises(IdentityError) as error:
        external_review.prepare_review(target, root, tmp_path / "checkout")
    required = int(re.search(r"需要 (\d+) bytes", str(error.value)).group(1))
    assert preflight.snapshot_bytes(root, base, head, base) == required


def test_space_failure_reports_bytes_held_by_vm_deleted_files(env, capsys, monkeypatch, tmp_path):
    bin_dir = tmp_path / "bin"
    (bin_dir / "pgrep").write_text("#!/bin/sh\necho 4242\n")
    # 同一 inode 被两个 fd 打开只算一次
    (bin_dir / "lsof").write_text("#!/bin/sh\nprintf 'p4242\\nf10\\ns1000\\ni7\\nf11\\ns1000\\ni7\\nf12\\ns500\\ni8\\n'\n")
    for name in ("pgrep", "lsof"):
        (bin_dir / name).chmod(0o755)
    monkeypatch.setattr(preflight.shutil, "disk_usage", lambda _: Usage(0, 0, 1))
    code, report, *_ = env(capsys=capsys)
    space = next(c for c in report["checks"] if c["name"] == "checkout_root_free_space")
    assert code == 2 and space["detail"]["vm_deleted_open_bytes"] == 1500 and space["detail"]["vm_deleted_open_files"] == 2


def _fake_launchctl(tmp_path, value: str) -> None:
    script = tmp_path / "bin" / "launchctl"
    script.write_text(f"#!/bin/sh\n[ \"$1 $2\" = \"getenv CLIPROXY_LOCAL_KEY\" ] && printf '%s' '{value}'\n")
    script.chmod(0o755)


def test_worker_key_missing_from_env_and_launchctl_blocks_before_probes(env, capsys, tmp_path, monkeypatch):
    """codex 机器人复核：宿主 auth.command 能取到 key 不代表正式 worker 有 key。"""
    monkeypatch.delenv("CLIPROXY_LOCAL_KEY")
    _fake_launchctl(tmp_path, "")
    code, report, _, proxy = env(capsys=capsys)
    assert code == 2 and "credential:review-worker" in failing(report) and proxy.requests == []
    assert next(c for c in report["checks"] if c["name"] == "credential:codex-host")["ok"]


def test_worker_key_falls_back_to_launchctl_like_provider_environment(env, capsys, tmp_path, monkeypatch):
    monkeypatch.delenv("CLIPROXY_LOCAL_KEY")
    _fake_launchctl(tmp_path, KEY)
    code, report, _, proxy = env(capsys=capsys)
    worker = next(c for c in report["checks"] if c["name"] == "credential:review-worker")
    assert code == 0 and worker["detail"]["source"] == "launchctl" and len(proxy.requests) == 2


def test_host_key_different_from_worker_is_proven_by_its_own_round_trip(env, capsys, tmp_path):
    """独审三·2：key 不同时 Sol 那一次探测走宿主 key，两把 key 各由一次真实往返证明，每家仍只探测一次。"""
    (tmp_path / "key.txt").write_text(OTHER_VALID + "\n")
    code, report, out, proxy = env(capsys=capsys)
    used = {r["body"]["model"]: r["auth"] for r in proxy.requests}
    assert code == 0 and used == {SOL: f"Bearer {OTHER_VALID}", OPUS: f"Bearer {KEY}"}
    assert OTHER_VALID not in out


def test_host_key_rejected_by_proxy_blocks(env, capsys, tmp_path):
    (tmp_path / "key.txt").write_text("revoked-key\n")
    code, report, *_ = env(capsys=capsys)
    assert code == 2 and f"roundtrip:{SOL}" in failing(report) and f"roundtrip:{OPUS}" not in failing(report)


def test_key_crossing_the_truncation_point_leaves_no_prefix(env, capsys):
    """独审二·1：先截断后脱敏会留下 key 前缀。"""
    def body(auth: str) -> str:
        head = json.dumps({"error": {"message": ""}})[:-3]  # 到 message 值开头为止
        pad = 290 - len(head) - len("Bearer ")  # 让 key 从第 290 个字符开始，跨过 300 的截断点
        return head + "x" * pad + auth + ' tail"}}'
    assert body(f"Bearer {KEY}").index(KEY) == 290
    code, report, out, _ = env(status={OPUS: 400}, error_body=body, capsys=capsys)
    assert code == 2 and KEY[:8] not in out


def test_provider_line_written_halfway_is_not_misread(env, capsys):
    """独审二·2：日志截在 provider=clau 时不能判失败。"""
    code, report, *_ = env(split_at="ude, auth_id", capsys=capsys)
    assert code == 0 and failing(report) == set()


def test_served_model_appended_later_is_still_checked(env, capsys):
    """独审二·2：路由头先写完、实际服务模型后补，fallback 仍须拦。"""
    code, report, *_ = env(served={OPUS: "claude-opus-4-8", SOL: SOL}, split_at='data: {"type":"message_start"', capsys=capsys)
    assert code == 2 and f"identity:{OPUS}" in failing(report)


def test_cleanup_stays_within_deadline_and_reports_failure(tmp_path, repo):
    """独审二·3/4：docker rm 卡住时目录仍删除、失败要报告，且总耗时不超总时限。"""
    import subprocess
    import sys
    root, base, head = repo
    bin_dir, checkouts, calls = tmp_path / "bin", tmp_path / "checkouts", tmp_path / "docker-calls"
    bin_dir.mkdir()
    (bin_dir / "docker").write_text(f'#!/bin/sh\necho "$@" >> {calls}\nsleep 5\n')
    (bin_dir / "docker").chmod(0o755)
    (bin_dir / "launchctl").write_text("#!/bin/sh\nexit 0\n")
    (bin_dir / "launchctl").chmod(0o755)
    codex = tmp_path / "codex"
    codex.mkdir()
    (codex / "config.toml").write_text('[model_providers.cliproxy-local]\nexperimental_bearer_token = "x"\n')
    start = time.monotonic()
    result = subprocess.run(
        [sys.executable, "-I", preflight.__file__, "--source", str(root), "--base", base, "--head", head,
         "--merge-base", base, "--checkout-root", str(checkouts), "--proxy-url", "http://localhost:9",  # DevSkim: ignore DS162092 有意使用本机回环（本地 CLIProxy / 测试假服务），非调试残留
         "--proxy-log-dir", str(tmp_path), "--proxy-config", str(tmp_path / "none.yaml"),
         "--codex-home", str(codex), "--deadline", "1.0"],
        capture_output=True, text=True, timeout=20, env={"PATH": f"{bin_dir}:/usr/bin:/bin", "HOME": str(tmp_path)})
    elapsed = time.monotonic() - start
    report = json.loads(result.stdout)
    assert result.returncode == 2 and elapsed < 1.6
    assert [p.name for p in checkouts.iterdir()] == []
    assert "probe_cleanup" in {c["name"] for c in report["checks"] if not c["ok"]}


def test_host_provider_pointing_elsewhere_blocks(env, capsys, tmp_path):
    """独审三·2：宿主 base_url 不是预检代理时，探测证明不了宿主能完成往返。"""
    config = tmp_path / "codex/config.toml"
    config.write_text(config.read_text().replace("http://127.0.0.1:8318/v1", "http://127.0.0.1:9/v1"))  # DevSkim: ignore DS162092 有意使用本机回环（本地 CLIProxy / 测试假服务），非调试残留
    code, report, *_ = env(capsys=capsys)
    host = next(c for c in report["checks"] if c["name"] == "credential:codex-host")
    assert code == 2 and failing(report) == {"credential:codex-host"} and "不是同一端点" in host["detail"]["error"]


def test_credential_stages_share_the_total_deadline(env, capsys, tmp_path, monkeypatch):
    """独审三·1：helper、launchctl 各自的超时都不能叠加到总时限之外。"""
    _auth_command(tmp_path, "/bin/sh", ["-c", "sleep 3; echo key"], timeout_ms=120000)
    monkeypatch.delenv("CLIPROXY_LOCAL_KEY")
    slow = tmp_path / "bin" / "launchctl"
    slow.write_text("#!/bin/sh\nsleep 5\n")
    slow.chmod(0o755)
    start = time.monotonic()
    code, report, *_ = env(capsys=capsys, extra=("--deadline", "1"))
    assert code == 2 and time.monotonic() - start < 1.6


def test_probe_dir_that_cannot_be_deleted_is_reported(env, capsys, tmp_path, monkeypatch):
    """独审三·3：正常路径删目录失败时不能静默丢登记。"""
    monkeypatch.setattr(preflight.shutil, "rmtree", lambda *a, **k: None)
    code, report, *_ = env(capsys=capsys)
    assert code == 2 and "probe_cleanup" in failing(report)
    for leftover in (tmp_path / "checkouts").iterdir():
        (leftover / "nonce").unlink(missing_ok=True)
        leftover.rmdir()


@pytest.mark.parametrize("value", ["0", "-5", "61", "1e9", "nan"])
def test_out_of_range_deadline_is_a_usage_error(value, capsys):
    with pytest.raises(SystemExit) as exit_info:
        preflight.main(["--source", ".", "--base", "a", "--head", "b", "--merge-base", "a", "--checkout-root", ".",
                        "--deadline", value])
    assert exit_info.value.code == 3


@pytest.mark.parametrize("a, b, same", [
    ("http://[::1]:8318/v1", "http://127.0.0.1:8318/v1", True),  # DevSkim: ignore DS162092 有意使用本机回环（本地 CLIProxy / 测试假服务），非调试残留
    ("http://localhost:8318/v1/", "http://127.0.0.1:8318/v1", True),  # DevSkim: ignore DS162092 有意使用本机回环（本地 CLIProxy / 测试假服务），非调试残留
    ("http://LOCALHOST:8318/v1", "http://[::1]:8318/v1", True),  # DevSkim: ignore DS162092 有意使用本机回环（本地 CLIProxy / 测试假服务），非调试残留
    ("http://127.0.0.1:9/v1", "http://127.0.0.1:8318/v1", False),  # DevSkim: ignore DS162092 有意使用本机回环（本地 CLIProxy / 测试假服务），非调试残留
    ("https://127.0.0.1:8318/v1", "http://127.0.0.1:8318/v1", False),  # DevSkim: ignore DS162092 有意使用本机回环（本地 CLIProxy / 测试假服务），非调试残留
    ("http://127.0.0.1:8318/v2", "http://127.0.0.1:8318/v1", False),  # DevSkim: ignore DS162092 有意使用本机回环（本地 CLIProxy / 测试假服务），非调试残留
])
def test_endpoint_equivalence_including_ipv6_loopback(a, b, same):
    """独审四·1：带端口的 IPv6 回环不能被误判为不同端点。"""
    assert (preflight._endpoint(a) == preflight._endpoint(b)) is same


def test_stream_reporting_fallback_model_blocks_even_without_upstream_body(env, capsys):
    """独审五：日志没有上游响应体、客户端流自报 claude-opus-4-8，不能放行。"""
    code, report, *_ = env(stream_model={OPUS: "claude-opus-4-8"}, capsys=capsys)
    identity = next(c for c in report["checks"] if c["name"] == f"identity:{OPUS}")
    assert code == 2 and failing(report) == {f"identity:{OPUS}"} and identity["detail"]["served"] == ["claude-opus-4-8"]


def test_unverifiable_served_model_blocks(env, capsys):
    """独审五：两处都没有服务模型证据时按无从核实拦截，不算通过。"""
    bare = b'data: {"type":"response.completed","response":{"status":"completed"}}\n\n'
    code, report, *_ = env(terminal=bare, capsys=capsys)
    assert code == 2 and failing(report) == {f"identity:{SOL}", f"identity:{OPUS}"}
    assert all(not c["detail"]["verified"] for c in report["checks"] if c["class"] == "identity")


# ---------------- 跨族边界独审（gemini）反例 ----------------

def test_key_with_inner_control_char_is_rejected_without_leaking(env, capsys, monkeypatch):
    monkeypatch.setenv("CLIPROXY_LOCAL_KEY", "abc\ndef-secret-tail")
    code, report, out, proxy = env(capsys=capsys)
    worker = next(c for c in report["checks"] if c["name"] == "credential:review-worker")
    assert code == 2 and not worker["ok"] and "控制字符" in worker["detail"]["error"]
    assert "def-secret-tail" not in out and proxy.requests == []


def test_key_with_trailing_newline_is_stripped_like_a_header_value(env, capsys, monkeypatch):
    monkeypatch.setenv("CLIPROXY_LOCAL_KEY", KEY + "\n")
    code, report, out, _ = env(capsys=capsys)
    assert code == 0 and KEY not in out


@pytest.mark.parametrize("secrets, text", [
    (["subkey", "host-secret-subkey-99999"], "err host-secret-subkey-99999"),
    (['"secret-key-5'], 'bad "secret-key-5 end'),
    (["it's-a-secret-key"], str({"m": "it's-a-secret-key"})),
])
def test_sanitize_hides_every_form_and_keeps_valid_json(secrets, text):
    out = json.dumps(preflight.sanitize({"detail": text}, secrets), ensure_ascii=False)
    json.loads(out)
    assert all(s not in out for s in secrets)
    assert "host-secret-" not in out and "99999" not in out


def test_redact_drops_key_prefix_left_at_any_cut():
    key = "secret-key-" + "a" * 190
    text = (key * 20)[:4000]
    assert key[:20] not in preflight.redact(text, [key], 300)
    assert key[:20] not in preflight.redact("prefix " + key[:150], [key])


def test_log_with_non_utf8_byte_is_still_read(tmp_path):
    session = "preflight-utf"
    (tmp_path / "v1-responses-1.log").write_bytes(
        f"=== HEADERS ===\nSession-Id: {session}\n\n=== API REQUEST 1 ===\n"
        "Upstream URL: https://chatgpt.com/backend-api/codex/responses\nAuth: provider=codex, auth_id=x\n\n"
        '=== API RESPONSE 1 ===\nStatus: 200\ndata: {"type":"response.created","response":{"model":"gpt-6.1-sol"}}\n'.encode()
        + b"# byte \xff\n\n=== RESPONSE ===\nStatus: 200\n")
    assert preflight._route_from_log(tmp_path, session, time.time() - 5, wait=0.5) == (
        [("codex", "chatgpt.com")], {"gpt-6.1-sol"})


def test_unreadable_log_dir_does_not_crash(env, capsys, tmp_path, monkeypatch):
    monkeypatch.setattr(preflight.Path, "glob", lambda self, pattern: (_ for _ in ()).throw(PermissionError(13, "denied")))
    code, report, *_ = env(capsys=capsys)
    assert code == 2 and {f"route:{SOL}", f"route:{OPUS}"} <= failing(report)


def test_relative_checkout_root_is_mounted_as_absolute(env, capsys, tmp_path, monkeypatch):
    monkeypatch.chdir(tmp_path)
    code, report, *_ = env(capsys=capsys, extra=("--checkout-root", "checkouts"))
    assert code == 0 and failing(report) == set()


@pytest.mark.parametrize("flag, value", [("--base", "--all"), ("--merge-base", "--max-count=0"), ("--head", "not-a-sha")])
def test_non_sha_arguments_are_usage_errors(flag, value, capsys):
    argv = ["--source", ".", "--base", "a" * 40, "--head", "b" * 40, "--merge-base", "a" * 40, "--checkout-root", "."]
    argv[argv.index(flag) + 1] = value
    with pytest.raises(SystemExit) as exit_info:
        preflight.main(argv)
    assert exit_info.value.code == 3


def test_blob_given_as_commit_is_not_accepted(env, capsys, repo):
    root, base, head = repo
    blob = git(root, "rev-parse", f"{head}:a.txt")
    code, report, *_ = env(capsys=capsys, extra=("--base", blob, "--merge-base", blob))
    source = next(c for c in report["checks"] if c["class"] == "source")
    assert code == 2 and source["detail"]["needs_source_sync"] == [blob]


@pytest.mark.parametrize("config", [b"\xff\xfe", b'[model_providers]\ncliproxy-local = 3\n',
                                    b'[model_providers.cliproxy-local]\nbase_url = "http://localhost:bad/v1"\n'  # DevSkim: ignore DS162092 有意使用本机回环（本地 CLIProxy / 测试假服务），非调试残留
                                    b'experimental_bearer_token = "local-test-key-must-not-leak"\n'])
def test_broken_codex_config_never_crashes_the_report(env, capsys, tmp_path, config):
    (tmp_path / "codex/config.toml").write_bytes(config)
    code, report, out, _ = env(capsys=capsys)
    assert code == 2 and report["kind"] == "review-preflight" and KEY not in out


def _cli(tmp_path, repo, docker_script: str, deadline: str, codex_text: str):
    import subprocess
    import sys
    root, base, head = repo
    bin_dir = tmp_path / "bin2"
    bin_dir.mkdir(exist_ok=True)
    (bin_dir / "docker").write_text(docker_script)
    (bin_dir / "docker").chmod(0o755)
    for name in ("launchctl", "pgrep"):
        (bin_dir / name).write_text("#!/bin/sh\nexit 0\n")
        (bin_dir / name).chmod(0o755)
    codex = tmp_path / "codex2"
    codex.mkdir(exist_ok=True)
    (codex / "config.toml").write_text(codex_text)
    result = subprocess.run(
        [sys.executable, "-I", preflight.__file__, "--source", str(root), "--base", base, "--head", head,
         "--merge-base", base, "--checkout-root", str(tmp_path / "chk"), "--proxy-url", "http://localhost:9",  # DevSkim: ignore DS162092 有意使用本机回环（本地 CLIProxy / 测试假服务），非调试残留
         "--proxy-log-dir", str(tmp_path), "--proxy-config", str(tmp_path / "none.yaml"),
         "--codex-home", str(codex), "--deadline", deadline],
        capture_output=True, text=True, timeout=20,
        env={"PATH": f"{bin_dir}:/usr/bin:/bin", "HOME": str(tmp_path), "CLIPROXY_LOCAL_KEY": "key12345"})
    return result.returncode, json.loads(result.stdout)


def test_no_probe_dir_is_created_after_the_deadline_is_spent(tmp_path, repo):
    """gemini 3·1：凭据阶段耗尽时限后，不能再建探针目录并在退出时遗留。"""
    slow = ('[model_providers.cliproxy-local]\nbase_url = "http://localhost:9/v1"\n'  # DevSkim: ignore DS162092 有意使用本机回环（本地 CLIProxy / 测试假服务），非调试残留
            '[model_providers.cliproxy-local.auth]\ncommand = "/bin/sh"\nargs = ["-c", "sleep 2; echo key12345"]\n')
    code, report = _cli(tmp_path, repo, "#!/bin/sh\nexit 0\n", "1", slow)
    assert code == 2
    assert not (tmp_path / "chk").exists() or list((tmp_path / "chk").iterdir()) == []


def test_in_flight_docker_run_is_killed_before_rm(tmp_path, repo):
    """gemini 3·2：docker run 在途时回收要先杀掉它，不能让它在 rm 之后才把容器建出来。"""
    created = tmp_path / "containers"
    created.mkdir()
    docker = (f'#!/bin/sh\nif [ "$1" = run ]; then sleep 1.5; touch "{created}/$4"; exit 0; fi\n'
              f'if [ "$1" = ps ]; then ls "{created}"; exit 0; fi\n'
              f'if [ "$1" = rm ]; then rm -f "{created}/$3"; exit 0; fi\n')
    code, report = _cli(tmp_path, repo, docker, "1",
                        '[model_providers.cliproxy-local]\nbase_url = "http://localhost:9/v1"\n'  # DevSkim: ignore DS162092 有意使用本机回环（本地 CLIProxy / 测试假服务），非调试残留
                        'experimental_bearer_token = "key12345"\n')
    time.sleep(2)
    assert code == 2 and list(created.iterdir()) == []
    assert not (tmp_path / "chk").exists() or list((tmp_path / "chk").iterdir()) == []


def test_failed_docker_run_container_is_removed(env, capsys, tmp_path):
    """gemini 3·3：docker run 非 0 退出时容器可能已建，--rm 不会清，须 rm -f。"""
    calls = tmp_path / "docker-calls"
    script = tmp_path / "bin" / "docker"
    script.write_text(f'#!/bin/sh\necho "$@" >> {calls}\n[ "$1" = ps ] && exit 0\n[ "$1" = rm ] && exit 0\nexit 125\n')
    code, report, *_ = env(capsys=capsys)
    assert code == 2 and any(line.startswith("rm -f preflight-") for line in calls.read_text().splitlines())
    assert "probe_cleanup" not in failing(report)
