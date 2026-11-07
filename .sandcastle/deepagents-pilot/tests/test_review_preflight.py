import json
import re
import subprocess
import sys
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
SHORT_VALID = "short7k"  # 正式 worker 接受的 7 字符 key
OTHER_VALID = f"outer-{KEY}-tail"  # 含 KEY：两把 key 互为子串
# 与报告字段名、枚举值相同或是其子串的合法 key（_clean_key 允许任意可见 ASCII）
PROTOCOL_LIKE_KEYS = ("ok", "ready", "completed", "re", "name", "codex")
KIMI, OPUS = "kimi-k3.1-pro", "claude-opus-5-5"
OFFICIAL = {KIMI: ("kimi", "https://api.kimi.com/coding/v1/responses"),
            OPUS: ("claude", "https://api.anthropic.com/v1/messages")}
ANTIGRAVITY = ("antigravity", "https://daily-cloudcode-pa.googleapis.com/v1internal:streamGenerateContent")
MODELFLARE = ("openai-compatible-modelflare-codex", "https://modelflare.dev/v1/responses")
LOCAL = "http://localhost:9"  # DevSkim: ignore DS162092 有意使用本机回环（本地 CLIProxy / 测试假服务），非调试残留
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

    def __init__(self, log_dir, routes, *, status=None, write_log=True, hang=False, log_delay=0.0,
                 echo_auth=False, terminal=None, header_first=0.0, split_at=None, error_body=None):
        self.requests = []
        proxy = self

        class Handler(BaseHTTPRequestHandler):
            def log_message(self, *args):
                pass

            def do_POST(self):
                body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
                proxy.requests.append({"body": body, "auth": self.headers["Authorization"]})
                if self.headers["Authorization"] not in {f"Bearer {k}" for k in (KEY, OTHER_VALID, SHORT_VALID, *PROTOCOL_LIKE_KEYS)}:
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
                            "\n=== API RESPONSE 1 ===\nStatus: 200\n\n=== RESPONSE ===\nStatus: 200\n")
                    if split_at and split_at in text:
                        # 分段写入：先落盘到 split_at 之前（可能截在一行中间），0.3 秒后补全
                        path.write_text(text[:text.index(split_at)])
                        threading.Timer(0.3, path.write_text, args=(text,)).start()
                    elif header_first:
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
                    payload = terminal or b'event: response.completed\ndata: {"type":"response.completed","response":{}}\n\n'
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
    """宿主 checkout_root、假 docker（按 bind 源映射读文件）与宿主 provider 配置。"""
    root, base, head = repo
    bin_dir, logs, checkouts = tmp_path / "bin", tmp_path / "logs", tmp_path / "checkouts"
    for d in (bin_dir, logs):
        d.mkdir()
    docker = bin_dir / "docker"
    docker.write_text("#!/bin/sh\n"
                      "[ \"$1\" = rm ] && exit 0\n"
                      "src=$(printf '%s\\n' \"$@\" | sed -n 's/^type=bind,src=\\(.*\\),dst=\\/checkouts,readonly$/\\1/p')\n"
                      "case \"$src\" in /*) ;; *) echo 'mount path must be absolute' >&2; exit 125;; esac\n"
                      "[ -n \"$FAKE_DOCKER_EMPTY\" ] && src=/nonexistent\n"
                      "for last; do :; done\n"
                      "exec cat \"$src${last#/checkouts}\"\n")
    docker.chmod(0o755)
    _fake_codex(bin_dir)
    monkeypatch.setenv("PATH", f"{bin_dir}:/usr/bin:/bin")
    # 测试不依赖宿主真实剩余空间；专测空间的用例自行覆盖。
    monkeypatch.setattr(preflight.shutil, "disk_usage", lambda _: Usage(0, 0, 1 << 50))
    # 预检进程里有这个变量（launcher 传入）；宿主 provider 按 auth.command 从文件取 key。
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
        code = preflight.main(["--source", str(root), "--base", base, "--head", head, "--merge-base", base,
                               "--checkout-root", str(checkouts), "--proxy-url", proxy.url,
                               "--proxy-log-dir", str(logs), "--codex-home", str(codex), *extra])
        out = capsys.readouterr().out
        return code, json.loads(out), out, proxy

    yield run
    for proxy in proxies:
        proxy.close()


def _fake_codex(bin_dir, stderr: str = "", rc: int = 0) -> None:
    """假 codex：`codex features list` 按给定退出码与 stderr 返回（真实 codex 的校验另有专门用例）。"""
    (bin_dir / "codex").write_text(f"#!/bin/sh\nprintf '%s' '{stderr}' >&2\nexit {rc}\n")
    (bin_dir / "codex").chmod(0o755)


def failing(report):
    return {c["name"] for c in report["checks"] if not c["ok"]}


def check(report, name):
    return next(c for c in report["checks"] if c["name"] == name)


def _cli(tmp_path, repo, docker_script: str, deadline: str, codex_text: str = (
        '[model_providers.cliproxy-local]\nexperimental_bearer_token = "key12345"\n')):
    """真实 CLI 退出路径（os._exit）；docker 换成给定脚本。"""
    root, base, head = repo
    bin_dir = tmp_path / "bin2"
    bin_dir.mkdir(exist_ok=True)
    (bin_dir / "docker").write_text(docker_script)
    (bin_dir / "docker").chmod(0o755)
    _fake_codex(bin_dir)
    codex = tmp_path / "codex2"
    codex.mkdir(exist_ok=True)
    (codex / "config.toml").write_text(codex_text)
    start = time.monotonic()
    result = subprocess.run(
        [sys.executable, "-I", preflight.__file__, "--source", str(root), "--base", base, "--head", head,
         "--merge-base", base, "--checkout-root", str(tmp_path / "chk"), "--proxy-url", LOCAL,
         "--proxy-log-dir", str(tmp_path), "--codex-home", str(codex), "--deadline", deadline],
        capture_output=True, text=True, timeout=20, env={"PATH": f"{bin_dir}:/usr/bin:/bin", "HOME": str(tmp_path)})
    return result.returncode, json.loads(result.stdout), time.monotonic() - start


def _leftovers(tmp_path):
    chk = tmp_path / "chk"
    return list(chk.iterdir()) if chk.exists() else []


# ---------------- 空间：PR1866 A1/A2、PR1868 A3 ----------------

def test_snapshot_bytes_equal_to_controller_formula(repo, tmp_path, monkeypatch):
    root, base, head = repo
    target = external_review.ReviewTarget(1, 7, 1, "main", base, 1, "topic", head, base, "run", 1,
                                          external_review.POLICY, head, 0)
    monkeypatch.setattr(external_review.shutil, "disk_usage", lambda _: Usage(0, 0, 0))
    with pytest.raises(IdentityError) as error:
        external_review.prepare_review(target, root, tmp_path / "checkout")
    required = int(re.search(r"需要 (\d+) bytes", str(error.value)).group(1))
    assert preflight.snapshot_bytes(root, base, head, base) == required


def test_snapshot_bytes_match_controller_with_replace_refs(repo, tmp_path, monkeypatch):
    """控制器不跟随 replace 引用；预检若跟随，同一输入字节数会不同。"""
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


def test_healthy_environment_is_ready_and_not_a_review(env, capsys):
    code, report, out, proxy = env(capsys=capsys)
    assert code == 0 and report["ready"] and failing(report) == set()
    assert report["is_review_result"] is False and report["kind"] == "review-preflight"
    assert KEY not in out
    assert sorted(r["body"]["model"] for r in proxy.requests) == sorted([KIMI, OPUS])
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


# ---------------- 挂载：PR1868 A1 ----------------

def test_container_seeing_empty_mount_blocks(env, capsys, monkeypatch):
    monkeypatch.setenv("FAKE_DOCKER_EMPTY", "1")
    code, report, *_ = env(capsys=capsys)
    assert code == 2 and failing(report) == {"checkout_root_container_mount"}


def test_probe_leaves_no_files_in_checkout_root(env, capsys, tmp_path):
    env(capsys=capsys)
    assert list((tmp_path / "checkouts").iterdir()) == []


def test_relative_checkout_root_is_mounted_as_absolute(env, capsys, tmp_path, monkeypatch):
    monkeypatch.chdir(tmp_path)
    code, report, *_ = env(capsys=capsys, extra=("--checkout-root", "checkouts"))
    assert code == 0 and failing(report) == set()


def test_failed_docker_run_container_is_removed(env, capsys, tmp_path):
    """docker run 非 0 退出时容器可能已建，--rm 不会清，须 rm -f。"""
    calls = tmp_path / "docker-calls"
    (tmp_path / "bin" / "docker").write_text(f'#!/bin/sh\necho "$@" >> {calls}\n[ "$1" = rm ] && exit 0\nexit 125\n')
    code, report, *_ = env(capsys=capsys)
    assert code == 2 and any(line.startswith("rm -f preflight-") for line in calls.read_text().splitlines())
    # docker 自身出错（125）时 daemon 可能稍后才建出容器，rm -f 成功也只能报回收未确认（独审十五）
    assert any("daemon" in item for item in check(report, "probe_cleanup")["detail"])


def test_container_command_failing_inside_a_finished_container_is_not_a_cleanup_failure(env, capsys, tmp_path):
    """容器里的命令非 0 退出（如 PR1868 A1 空挂载时 cat 失败）：容器已跑完、--rm 已删，不报回收失败。"""
    (tmp_path / "bin" / "docker").write_text('#!/bin/sh\n[ "$1" = rm ] && exit 0\nexit 1\n')
    code, report, *_ = env(capsys=capsys)
    assert code == 2 and failing(report) == {"checkout_root_container_mount"}


def test_probe_dir_that_cannot_be_deleted_is_reported(env, capsys, tmp_path, monkeypatch):
    monkeypatch.setattr(preflight.shutil, "rmtree", lambda *a, **k: None)
    code, report, *_ = env(capsys=capsys)
    assert code == 2 and "probe_cleanup" in failing(report)
    for leftover in (tmp_path / "checkouts").iterdir():
        (leftover / "nonce").unlink(missing_ok=True)
        leftover.rmdir()


# ---------------- 有界原语 run_bounded：超时、杀进程组、清理只在这一处 ----------------

def test_run_bounded_kills_a_background_child_left_by_an_exited_parent(tmp_path):
    marker = tmp_path / "late"
    result = preflight.run_bounded(["/bin/sh", "-c", f"(sleep 0.5; touch {marker}) >/dev/null 2>&1 & exit 0"], 5)
    time.sleep(1)
    assert result.returncode == 0 and not marker.exists()


def test_run_bounded_timeout_kills_the_whole_group_in_time(tmp_path):
    marker = tmp_path / "late"
    start = time.monotonic()
    with pytest.raises(subprocess.TimeoutExpired):
        preflight.run_bounded(["/bin/sh", "-c", f"(sleep 0.8; touch {marker}) & wait"], 0.3)
    assert time.monotonic() - start < 0.6
    time.sleep(1)
    assert not marker.exists()


def test_run_bounded_with_no_time_left_starts_nothing(tmp_path, monkeypatch):
    monkeypatch.setattr(preflight.subprocess, "Popen", lambda *a, **k: pytest.fail("不应启动子进程"))
    with pytest.raises(subprocess.TimeoutExpired):
        preflight.run_bounded(["true"], 0)


def test_hanging_docker_is_killed_removed_and_reported_within_deadline(tmp_path, repo):
    """docker run 与 rm 都卡住：总耗时不越过 --deadline，探针目录删掉，回收失败如实报告。"""
    calls = tmp_path / "docker-calls"
    code, report, elapsed = _cli(tmp_path, repo, f'#!/bin/sh\necho "$@" >> {calls}\nsleep 30\n', "4")
    lines = calls.read_text().splitlines() if calls.exists() else []
    assert code == 2 and elapsed < 4.6 and report["wall_seconds"] <= 4.0 and _leftovers(tmp_path) == []
    assert "probe_cleanup" in failing(report)
    runs = [a.split()[a.split().index("--name") + 1] for a in lines if a.startswith("run ")]
    assert all(f"rm -f {name}" in lines for name in runs)  # 机器忙到 docker run 没起来时，没有要删的容器


def test_report_is_printed_within_the_deadline_when_docker_hangs(tmp_path, repo):
    code, report, _ = _cli(tmp_path, repo, "#!/bin/sh\nsleep 30\n", "2")
    assert code == 2 and report["wall_seconds"] <= 2.0


def test_in_flight_docker_run_is_killed_before_it_creates_a_container(tmp_path, repo):
    created = tmp_path / "containers"
    created.mkdir()
    docker = (f'#!/bin/sh\nif [ "$1" = run ]; then sleep 1.5; touch "{created}/$4"; exit 0; fi\n'
              f'if [ "$1" = rm ]; then rm -f "{created}/$3"; exit 0; fi\n')
    code, _, _ = _cli(tmp_path, repo, docker, "1")
    time.sleep(2)
    assert code == 2 and list(created.iterdir()) == [] and _leftovers(tmp_path) == []


@pytest.mark.parametrize("spawn", [
    "(sleep 1.5; touch \"{created}/$4\") & wait; exit 0",  # 包装脚本等子进程
    "(sleep 2.5; touch \"{created}/$4\") & exit 0",  # 父进程先退出，子进程占着 stdout
    "(sleep 2.5; touch \"{created}/$4\") >/dev/null 2>&1 & exit 0",  # 关闭标准流后放后台，以 0 退出
])
def test_container_created_late_by_a_docker_wrapper_child_never_appears(tmp_path, repo, spawn):
    created = tmp_path / "containers"
    created.mkdir()
    docker = (f'#!/bin/sh\nif [ "$1" = run ]; then {spawn.format(created=created)}; fi\n'
              f'if [ "$1" = rm ]; then rm -f "{created}/$3"; exit 0; fi\n')
    code, _, _ = _cli(tmp_path, repo, docker, "2")
    time.sleep(3)
    assert code == 2 and list(created.iterdir()) == []


def test_no_probe_dir_is_left_when_credentials_use_up_the_deadline(tmp_path, repo):
    slow = ('[model_providers.cliproxy-local]\n'
            '[model_providers.cliproxy-local.auth]\ncommand = "/bin/sh"\nargs = ["-c", "sleep 2; echo key12345"]\n')
    code, _, _ = _cli(tmp_path, repo, "#!/bin/sh\nexit 0\n", "1", slow)
    assert code == 2 and _leftovers(tmp_path) == []


# ---------------- 路由与往返：PR1883 A1 ----------------

def test_opus_on_antigravity_with_upstream_400_blocks(env, capsys):
    code, report, *_ = env(routes={**OFFICIAL, OPUS: ANTIGRAVITY}, status={OPUS: 400}, capsys=capsys)
    assert code == 2 and failing(report) == {f"route:{OPUS}", f"roundtrip:{OPUS}"}
    assert check(report, f"route:{OPUS}")["detail"]["actual"] == ["antigravity@daily-cloudcode-pa.googleapis.com"]
    assert check(report, f"roundtrip:{OPUS}")["detail"]["error"]["class"] == "tool_choice_unsupported"


def test_wrong_provider_blocks_even_when_round_trip_succeeds(env, capsys):
    code, report, *_ = env(routes={**OFFICIAL, OPUS: ANTIGRAVITY}, capsys=capsys)
    assert code == 2 and failing(report) == {f"route:{OPUS}"}


def test_kimi_on_third_party_route_blocks(env, capsys):
    code, report, *_ = env(routes={**OFFICIAL, KIMI: MODELFLARE}, status={KIMI: 403}, capsys=capsys)
    assert code == 2 and failing(report) == {f"route:{KIMI}", f"roundtrip:{KIMI}"}


def test_kimi_international_upstream_is_an_official_route(env, capsys):
    """CLIProxy 在国内 kimi 与国际 kimi-ai 之间按同型号候选池切换，两条都是正式路线。"""
    code, report, *_ = env(routes={**OFFICIAL, KIMI: ("kimi-ai", "https://api.kimi.ai/coding/v1/responses")}, capsys=capsys)
    assert code == 0 and failing(report) == set()
    assert check(report, f"route:{KIMI}")["detail"]["actual"] == ["kimi-ai@api.kimi.ai"]
    assert check(report, f"route:{KIMI}")["detail"]["expected"] == ["kimi-ai@api.kimi.ai", "kimi@api.kimi.com"]


def test_kimi_routed_to_codex_blocks(env, capsys):
    code, report, *_ = env(routes={**OFFICIAL, KIMI: ("codex", "https://chatgpt.com/backend-api/codex/responses")},
                           capsys=capsys)
    assert code == 2 and failing(report) == {f"route:{KIMI}"}


def test_preflight_probes_the_controller_reviewer_pair():
    assert preflight.MODELS == external_review.MODELS
    assert set(preflight.MODELS) <= set(preflight.ROUTES)


def test_route_unknown_without_request_log_blocks(env, capsys):
    code, report, *_ = env(write_log=False, capsys=capsys, extra=("--deadline", "6"))
    assert code == 2 and failing(report) == {f"route:{KIMI}", f"route:{OPUS}"}
    assert all(not c["detail"]["log_found"] for c in report["checks"] if c["name"].startswith("route:"))


def test_request_log_written_one_second_late_still_passes(env, capsys):
    code, report, *_ = env(log_delay=1.0, capsys=capsys)
    assert code == 0 and failing(report) == set()


def test_log_written_in_two_steps_is_not_misread(env, capsys):
    code, report, *_ = env(header_first=0.3, capsys=capsys)
    assert code == 0 and failing(report) == set()


def test_provider_line_written_halfway_is_not_misread(env, capsys):
    """日志截在 provider=clau 时不能判失败。"""
    code, report, *_ = env(split_at="ude, auth_id", capsys=capsys)
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


def test_unreadable_log_dir_does_not_crash(env, capsys, monkeypatch):
    monkeypatch.setattr(preflight.Path, "glob", lambda self, pattern: (_ for _ in ()).throw(PermissionError(13, "denied")))
    code, report, *_ = env(capsys=capsys, extra=("--deadline", "6"))
    assert code == 2 and {f"route:{KIMI}", f"route:{OPUS}"} <= failing(report)


def test_log_with_non_utf8_byte_is_still_read(tmp_path):
    session = "preflight-utf"
    (tmp_path / "v1-responses-1.log").write_bytes(
        f"=== HEADERS ===\nSession-Id: {session}\n\n=== API REQUEST 1 ===\n"
        "Upstream URL: https://chatgpt.com/backend-api/codex/responses\nAuth: provider=codex, auth_id=x\n\n"
        "=== API RESPONSE 1 ===\nStatus: 200\n".encode() + b"# byte \xff\n\n=== RESPONSE ===\nStatus: 200\n")
    assert preflight.route_from_log(tmp_path, session, time.time() - 5, wait=0.5) == [("codex", "chatgpt.com")]


def test_completed_event_with_failed_status_is_not_success(env, capsys):
    bad = b'data: {"type":"response.completed","response":{"status":"failed","error":{"message":"overloaded"}}}\n\n'
    code, report, *_ = env(terminal=bad, capsys=capsys)
    assert code == 2 and {f"roundtrip:{KIMI}", f"roundtrip:{OPUS}"} <= failing(report)


def test_incomplete_without_error_still_passes(env, capsys):
    ok = b'data: {"type":"response.incomplete","response":{"status":"incomplete","incomplete_details":{"reason":"max_output_tokens"}}}\n\n'
    code, report, *_ = env(terminal=ok, capsys=capsys)
    assert code == 0


def test_hanging_upstream_is_cut_at_deadline(env, capsys, monkeypatch):
    start = time.monotonic()
    code, report, *_ = env(hang=True, extra=("--deadline", "2"), capsys=capsys)
    assert time.monotonic() - start < 4
    assert code == 2 and {f"roundtrip:{KIMI}", f"roundtrip:{OPUS}"} <= failing(report)


def test_proxy_down_blocks_without_waiting_out_the_log_window(env, capsys):
    start = time.monotonic()
    code, report, *_ = env(capsys=capsys, extra=("--proxy-url", LOCAL))  # 后给的 --proxy-url 覆盖假 proxy
    assert code == 2 and {f"roundtrip:{KIMI}", f"roundtrip:{OPUS}"} <= failing(report)
    assert time.monotonic() - start < 2


def test_no_probe_is_sent_once_the_work_deadline_has_passed(monkeypatch):
    import urllib.request
    monkeypatch.setattr(urllib.request, "urlopen", lambda *a, **k: pytest.fail("不应发送"))
    deadline = preflight.Deadline(0.002)
    time.sleep(0.01)
    assert preflight.round_trip(LOCAL, KEY, KIMI, "s", deadline)["error"] == {"not_sent": "总时限已到"}


# ---------------- 凭据：#1323 宿主缺 CLIPROXY_LOCAL_KEY ----------------

def test_env_key_provider_without_variable_blocks_before_any_probe(env, capsys, tmp_path, monkeypatch):
    """宿主 provider 只认 env_key 时，去掉 CLIPROXY_LOCAL_KEY 即报 Missing environment variable。"""
    (tmp_path / "codex/config.toml").write_text(
        '[model_providers.cliproxy-local]\nbase_url = "http://127.0.0.1:8318/v1"\nenv_key = "CLIPROXY_LOCAL_KEY"\n')  # DevSkim: ignore DS162092 有意使用本机回环（本地 CLIProxy / 测试假服务），非调试残留
    monkeypatch.setenv("CLIPROXY_LOCAL_KEY", KEY)  # 预检进程里有也不算：宿主进程里没有
    code, report, out, proxy = env(capsys=capsys)
    assert code == 2 and failing(report) == {"credential:codex-host", f"roundtrip:{KIMI}", f"roundtrip:{OPUS}"}
    assert "Missing environment variable: CLIPROXY_LOCAL_KEY" in check(report, "credential:codex-host")["detail"]["error"]
    assert KEY not in out and proxy.requests == []


def test_auth_command_failure_blocks(env, capsys, tmp_path):
    (tmp_path / "key.txt").unlink()
    code, report, *_ = env(capsys=capsys)
    assert code == 2 and "credential:codex-host" in failing(report)


def test_config_rejected_by_codex_blocks_and_only_the_field_name_is_reported(env, capsys, tmp_path):
    """provider 配置以宿主 codex 自己的加载结果为准；报错里只取字段名，其余文本不进报告。"""
    _fake_codex(tmp_path / "bin", "model_providers.cliproxy-local: provider auth cannot be combined with env_key "
                "value=local-test-key-must-not-leak", rc=1)
    code, report, out, proxy = env(capsys=capsys)
    error = check(report, "credential:codex-host")["detail"]["error"]
    assert code == 2 and "cannot be combined with env_key" in error and KEY not in out and proxy.requests == []


REAL_CODEX = __import__("shutil").which("codex")


@pytest.mark.skipif(REAL_CODEX is None, reason="本机没有 codex")
@pytest.mark.parametrize("extra, auth_extra", [
    ('env_key = ""', ""), ('experimental_bearer_token = ""', ""), ("requires_openai_auth = true", ""),
    ("", "timeout_ms = 12.5"), ("", "refresh_interval_ms = -1"), ('wire_api = "chat"', ""),
])
def test_configs_the_real_codex_refuses_are_blocked(tmp_path, monkeypatch, extra, auth_extra):
    """独审二十 A1/A2：真实 codex 拒绝加载的 provider 配置，预检也必须拦（不再自己抄校验规则）。"""
    home = tmp_path / "codex"
    home.mkdir()
    (tmp_path / "key.txt").write_text("k\n")
    wire = "" if "wire_api" in extra else 'wire_api = "responses"\n'
    (home / "config.toml").write_text(
        f'[model_providers.cliproxy-local]\nname = "x"\nbase_url = "http://127.0.0.1:9/v1"\n{wire}{extra}\n'  # DevSkim: ignore DS162092 有意使用本机回环（本地 CLIProxy / 测试假服务），非调试残留
        f'[model_providers.cliproxy-local.auth]\ncommand = "/bin/cat"\nargs = ["{tmp_path / "key.txt"}"]\n{auth_extra}\n')
    with pytest.raises(preflight.CredentialError, match="Codex 拒绝加载"):
        preflight.provider_key(home, preflight.Deadline(10), REAL_CODEX)


@pytest.mark.skipif(REAL_CODEX is None, reason="本机没有 codex")
def test_valid_auth_command_config_is_accepted_by_the_real_codex(tmp_path):
    home = tmp_path / "codex"
    home.mkdir()
    (tmp_path / "key.txt").write_text("k\n")
    (home / "config.toml").write_text(
        '[model_providers.cliproxy-local]\nname = "x"\nbase_url = "http://127.0.0.1:9/v1"\nwire_api = "responses"\n'  # DevSkim: ignore DS162092 有意使用本机回环（本地 CLIProxy / 测试假服务），非调试残留
        'requires_openai_auth = false\n'
        f'[model_providers.cliproxy-local.auth]\ncommand = "/bin/cat"\nargs = ["{tmp_path / "key.txt"}"]\n')
    assert preflight.provider_key(home, preflight.Deadline(10), REAL_CODEX) == ("k", "auth.command")


def test_fifo_config_cannot_stretch_past_the_deadline(env, capsys, tmp_path):
    """独审二十 C1：config.toml 是 FIFO（写端迟迟不来）时，凭据检查按时限失败，不越过 --deadline。"""
    import os as _os
    config = tmp_path / "codex/config.toml"
    config.unlink()
    _os.mkfifo(config)
    (tmp_path / "bin" / "codex").write_text(f"#!/bin/sh\ncat '{config}' >/dev/null\n")  # 真实 codex 读 FIFO 也会卡住
    start = time.monotonic()
    code, report, *_ = env(capsys=capsys, extra=("--deadline", "1"))
    assert code == 2 and "credential:codex-host" in failing(report) and time.monotonic() - start < 1.5


def test_request_log_written_late_within_the_deadline_still_passes(env, capsys):
    """独审二十 B1：日志晚于 2 秒落盘也不是路由失败，等到工作时限为止。"""
    code, report, *_ = env(log_delay=2.4, capsys=capsys)
    assert code == 0 and failing(report) == set()


def test_host_key_rejected_by_proxy_blocks(env, capsys, tmp_path):
    (tmp_path / "key.txt").write_text("revoked-key\n")
    code, report, *_ = env(capsys=capsys)
    assert code == 2 and {f"roundtrip:{KIMI}", f"roundtrip:{OPUS}"} <= failing(report)


def _auth_command(tmp_path, command: str, args: list[str], timeout_ms: int = 5000) -> None:
    (tmp_path / "codex/config.toml").write_text(
        '[model_providers.cliproxy-local]\n'
        f'[model_providers.cliproxy-local.auth]\ncommand = "{command}"\nargs = {json.dumps(args)}\ntimeout_ms = {timeout_ms}\n')


def test_credential_helper_respects_total_deadline(env, capsys, tmp_path):
    _auth_command(tmp_path, "/bin/sh", ["-c", "sleep 3; echo key"], timeout_ms=120000)
    start = time.monotonic()
    code, report, *_ = env(capsys=capsys, extra=("--deadline", "0.5"))
    assert time.monotonic() - start < 2 and code == 2


# ---------------- 密钥不外泄（全部保留） ----------------

def test_credential_helper_timeout_does_not_leak_its_arguments(env, capsys, tmp_path):
    _auth_command(tmp_path, "/bin/sh", ["-c", f"sleep 5 # {KEY}"], timeout_ms=200)
    code, report, out, _ = env(capsys=capsys)
    assert code == 2 and KEY not in out
    assert "未返回" in check(report, "credential:codex-host")["detail"]["error"]


def test_key_echoed_by_upstream_error_is_redacted(env, capsys):
    code, report, out, _ = env(status={OPUS: 400}, echo_auth=True, capsys=capsys)
    trip = check(report, f"roundtrip:{OPUS}")["detail"]
    # 上游正文不进报告：只有分类、长度和摘要
    assert code == 2 and KEY not in out and set(trip["error"]) == {"class", "length", "sha256_12"}


def test_key_crossing_the_truncation_point_leaves_no_prefix(env, capsys):
    """先截断后脱敏会留下 key 前缀。"""
    def body(auth: str) -> str:
        head = json.dumps({"error": {"message": ""}})[:-3]
        pad = 290 - len(head) - len("Bearer ")  # 让 key 从第 290 个字符开始，跨过 300 的截断点
        return head + "x" * pad + auth + ' tail"}}'
    assert body(f"Bearer {KEY}").index(KEY) == 290
    code, report, out, _ = env(status={OPUS: 400}, error_body=body, capsys=capsys)
    assert code == 2 and KEY[:8] not in out


def test_host_key_from_file_is_used_for_both_probes_and_never_printed(env, capsys, tmp_path):
    (tmp_path / "key.txt").write_text(OTHER_VALID + "\n")
    code, report, out, proxy = env(capsys=capsys)
    assert code == 0 and {r["auth"] for r in proxy.requests} == {f"Bearer {OTHER_VALID}"}
    assert OTHER_VALID not in out


def test_nested_keys_echoed_by_upstream_never_split_each_other(env, capsys, monkeypatch):
    """请求用短 key（宿主），错误正文却回显包含它的长 key（环境变量）；只按短 key 先脱敏会把长 key 拆开。"""
    monkeypatch.setenv("CLIPROXY_LOCAL_KEY", OTHER_VALID)
    body = lambda auth: json.dumps({"error": {"message": f"other credential {OTHER_VALID} rejected"}})  # noqa: E731
    code, report, out, _ = env(status={KIMI: 400}, error_body=body, capsys=capsys)
    assert code == 2 and KEY not in out and "outer-" not in out and "-tail" not in out


def test_key_with_inner_control_char_is_rejected_without_leaking(env, capsys, tmp_path):
    (tmp_path / "key.txt").write_text("abc\ndef-secret-tail")
    code, report, out, proxy = env(capsys=capsys)
    host = check(report, "credential:codex-host")
    assert code == 2 and not host["ok"] and "非可见 ASCII" in host["detail"]["error"]
    assert "def-secret-tail" not in out and proxy.requests == []


def test_key_with_surrounding_whitespace_is_stripped_like_a_header_value(env, capsys, tmp_path):
    (tmp_path / "key.txt").write_text(f"  {KEY}  \n\n")
    code, report, out, _ = env(capsys=capsys)
    assert code == 0 and KEY not in out


def test_short_valid_key_passes_in_a_healthy_environment(env, capsys, tmp_path):
    """7 字符的有效 key 在健康环境不能被误拦，也不能出现在输出里。"""
    (tmp_path / "key.txt").write_text(SHORT_VALID + "\n")
    code, report, out, proxy = env(capsys=capsys)
    assert code == 0 and len(proxy.requests) == 2 and SHORT_VALID not in out


@pytest.mark.parametrize("config", [b"\xff\xfe", b'[model_providers]\ncliproxy-local = 3\n',
                                    b'[model_providers.cliproxy-local]\n'
                                    b'experimental_bearer_token = ["local-test-key-must-not-leak"]\n'])
def test_broken_codex_config_never_crashes_the_report(env, capsys, tmp_path, config):
    (tmp_path / "codex/config.toml").write_bytes(config)
    code, report, out, _ = env(capsys=capsys)
    assert code == 2 and report["kind"] == "review-preflight" and KEY not in out


def test_structured_sse_error_never_reaches_the_report(monkeypatch):
    """SSE 错误体（含二次转义的 key）不抄进报告，只留分类与摘要。"""
    import io
    import urllib.request

    key = 'secret"key"12345'
    line = "data: " + json.dumps({"type": "error", "error": {"message": json.dumps({"e": f"invalid Bearer {key}"})}}) + "\n\n"

    class FakeResp(io.BytesIO):
        status = 200

        def __enter__(self):
            return self

        def __exit__(self, *a):
            return False

    monkeypatch.setattr(urllib.request, "urlopen", lambda req, timeout=None: FakeResp(line.encode()))
    trip = preflight.round_trip(LOCAL, key, KIMI, "s", preflight.Deadline(30))
    out = json.dumps(preflight.sanitize(trip, [key]), ensure_ascii=True)
    assert "secret" not in out and "12345" not in out and trip["error"]["class"] == "auth"


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


def test_truncation_leaves_no_multi_char_key_prefix():
    key = "MySecretKey-must-not-leak"
    assert not preflight.redact("x" * 497 + key[:20], [key], limit=500).endswith("MyS")


def test_untruncated_protocol_fields_are_never_rewritten():
    """key 首尾两个字符碰巧与字段首尾重合时，未截断的协议字段不能被改写（#1742 调用方按字段匹配）。"""
    report = {"kind": "review-preflight", "checks": [{"name": "credential:codex-host", "class": "credential"}]}
    assert preflight.sanitize(report, ["st-valid-key-ending-re"]) == report


@pytest.mark.parametrize("raw", ["\x80-secret-tail-12345", "café-secret-key"])
def test_non_ascii_keys_are_rejected(raw):
    with pytest.raises(preflight.CredentialError):
        preflight._clean_key(raw, "env")


def test_short_and_escape_containing_keys_are_accepted_and_still_redacted():
    """正式 worker 不限长度与字符子集，预检也不能多拦；短 key 与含转义的 key 由脱敏覆盖。"""
    for raw in ("short7k", "abc", "secret-\\u00e9-12345", 'secret"quoted-1234'):
        assert preflight._clean_key(raw, "env") == raw
    escaped = "secret-\\u00e9-12345"
    decoded = json.loads(f'"{escaped}"')
    out = json.dumps(preflight.sanitize({"status": decoded, "short": "Bearer abc"}, [escaped, "abc"]), ensure_ascii=True)
    assert "secret-" not in out and "12345" not in out and "abc" not in out


def test_surrogate_key_is_redacted_in_replace_decoded_text():
    secret = "secret-\udcff-key-12345"
    text = b"secret-\xff-key-12345".decode("utf-8", "replace")
    out = json.dumps(preflight.sanitize({"detail": text}, [secret]), ensure_ascii=True)
    assert "secret-" not in out and "-key-12345" not in out


def test_usage_error_redacts_the_environment_key(capsys, monkeypatch):
    monkeypatch.setenv("CLIPROXY_LOCAL_KEY", "MySecretKey-must-not-leak")
    with pytest.raises(SystemExit):
        preflight.main(["--deadline", "MySecretKey-must-not-leak"])
    captured = capsys.readouterr()
    assert "MySecretKey" not in captured.out and "MySecretKey" not in captured.err


def test_output_is_ascii_json_even_with_surrogates(tmp_path, repo, capsys):
    root, base, head = repo
    code = preflight.main(["--source", str(root), "--base", base, "--head", head, "--merge-base", base,
                           "--checkout-root", str(tmp_path / "chk"), "--image", "img-\udcff",
                           "--proxy-url", LOCAL, "--codex-home", str(tmp_path), "--deadline", "1"])
    out = capsys.readouterr().out
    assert code == 2 and out.isascii()
    json.loads(out)


# ---------------- 用法 ----------------

@pytest.mark.parametrize("value", ["0", "-5", "61", "1e9", "nan"])
def test_out_of_range_deadline_is_a_usage_error(value, capsys):
    with pytest.raises(SystemExit) as exit_info:
        preflight.main(["--source", ".", "--base", "a", "--head", "b", "--merge-base", "a", "--checkout-root", ".",
                        "--deadline", value])
    assert exit_info.value.code == 3


@pytest.mark.parametrize("flag, value", [("--base", "--all"), ("--merge-base", "--max-count=0"), ("--head", "not-a-sha")])
def test_non_sha_arguments_are_usage_errors(flag, value, capsys):
    argv = ["--source", ".", "--base", "a" * 40, "--head", "b" * 40, "--merge-base", "a" * 40, "--checkout-root", "."]
    argv[argv.index(flag) + 1] = value
    with pytest.raises(SystemExit) as exit_info:
        preflight.main(argv)
    assert exit_info.value.code == 3


def test_usage_error_and_help_print_json(capsys):
    with pytest.raises(SystemExit) as exit_info:
        preflight.main(["--deadline", "0"])
    assert exit_info.value.code == 3 and "usage_error" in json.loads(capsys.readouterr().out)
    with pytest.raises(SystemExit) as exit_info:
        preflight.main(["--help"])
    assert exit_info.value.code == 0 and "usage" in json.loads(capsys.readouterr().out)


# ---------------- 独审（缩审查面后）----------------

def test_upstream_status_object_keyed_by_the_key_never_reaches_the_report(monkeypatch, capsys):
    """上游把 key 放进 response.status 字典的键里：status 只收字符串，报告的键也脱敏。"""
    import io
    import urllib.request

    key = "isolated-fake-host-key"
    event = {"type": "response.completed", "response": {"status": {key: "diagnostic"}}}

    class FakeResp(io.BytesIO):
        status = 200

        def __enter__(self):
            return self

        def __exit__(self, *a):
            return False

    monkeypatch.setattr(urllib.request, "urlopen",
                        lambda req, timeout=None: FakeResp(("data: " + json.dumps(event) + "\n\n").encode()))
    trip = preflight.round_trip(LOCAL, key, KIMI, "s", preflight.Deadline(5))
    preflight._emit(trip, [key])
    assert key not in capsys.readouterr().out and trip["status"] == "other"


def test_sanitize_redacts_dict_keys_too():
    out = json.dumps(preflight.sanitize({"local-secret-key-1": {"inner-local-secret-key-1": 1}}, ["local-secret-key-1"]))
    assert "local-secret-key-1" not in out


def test_timed_out_docker_run_reports_cleanup_unconfirmed_even_if_rm_succeeds(tmp_path, repo):
    """杀掉 docker 客户端撤不回 daemon 已收的创建请求；rm -f 成功也不能报告无遗留。"""
    docker = '#!/bin/sh\n[ "$1" = rm ] && exit 0\nsleep 30\n'
    code, report, _ = _cli(tmp_path, repo, docker, "2")
    cleanup = next(c for c in report["checks"] if c["name"] == "probe_cleanup")
    assert code == 2 and any("daemon" in item for item in cleanup["detail"])


def test_container_created_by_the_daemon_after_a_failed_client_and_rm_is_reported(tmp_path, repo):
    """独审十五：独立会话里的假 daemon 在客户端以 125 退出、rm -f 已返回之后才建出容器；报告必须写明回收未确认。"""
    created = tmp_path / "containers"
    created.mkdir()
    ready = tmp_path / "daemon-ready"
    daemon = (f"import os, sys, time; os.setsid(); open('{ready}', 'w').close(); "
              "time.sleep(0.6); open(sys.argv[1], 'w').close()")
    # 客户端等假 daemon 脱离进程组（已收下创建请求）后才以 125 退出
    docker = (f'#!/bin/sh\nif [ "$1" = run ]; then python3 -c "{daemon}" "{created}/$4" >/dev/null 2>&1 & '
              f'while [ ! -e "{ready}" ]; do sleep 0.02; done; exit 125; fi\n'
              f'if [ "$1" = rm ]; then rm -f "{created}/$3"; exit 0; fi\n')
    code, report, _ = _cli(tmp_path, repo, docker, "4")
    time.sleep(1)
    assert list(created.iterdir())  # 容器确实在 rm -f 之后出现：killpg 与 rm -f 都管不到 daemon
    assert code == 2 and any("daemon" in item for item in check(report, "probe_cleanup")["detail"])


@pytest.mark.parametrize("status", [lambda k: json.dumps(json.dumps(k)), lambda k: k[::-1], lambda k: {k: 1}])
def test_unknown_upstream_status_is_normalised_and_never_carries_the_key(monkeypatch, status):
    """独审十六：status 只报已知枚举；双重 JSON 编码、倒序等任意形态的 key 都不进报告。"""
    import io
    import urllib.request

    key = 'isolated"host"key1742'
    event = {"type": "response.completed", "response": {"status": status(key)}}

    class FakeResp(io.BytesIO):
        status = 200

        def __enter__(self):
            return self

        def __exit__(self, *a):
            return False

    monkeypatch.setattr(urllib.request, "urlopen",
                        lambda req, timeout=None: FakeResp(("data: " + json.dumps(event) + "\n\n").encode()))
    trip = preflight.round_trip(LOCAL, key, KIMI, "s", preflight.Deadline(5))
    assert trip["status"] == "other" and "1742" not in json.dumps(preflight.sanitize(trip, [key]))


def test_route_lines_forged_inside_a_response_body_are_ignored_and_never_echoed(tmp_path, monkeypatch, capsys):
    """独审十七：响应正文里伪造的 Upstream URL / Auth 行（主机里夹带编码过的 key）不算路由、不进报告。"""
    from types import SimpleNamespace

    key = 'isolated"host"key1742'
    encoded = json.dumps(json.dumps(key))
    (tmp_path / "v1-responses-demo.log").write_text(
        "Session-Id: preflight-demo\n\n=== API REQUEST 1 ===\n"
        "Upstream URL: https://api.anthropic.com/v1/messages\nAuth: provider=claude, auth_id=x\n\n"
        "=== API RESPONSE 1 ===\nStatus: 400\nBody:\n"
        f"Upstream URL: https://{encoded}\nAuth: provider=claude, auth_id=echo\n\n"
        "=== RESPONSE ===\nStatus: 400\n")
    monkeypatch.setattr(preflight.uuid, "uuid4", lambda: "demo")
    monkeypatch.setattr(preflight, "round_trip", lambda *a: {"ok": True})
    args = SimpleNamespace(proxy_url=LOCAL, proxy_log_dir=str(tmp_path))
    checks = preflight.probe_model(args, OPUS, key, preflight.Deadline(5))
    preflight._emit({"checks": checks}, [key])
    out = capsys.readouterr().out
    assert checks[0]["detail"]["actual"] == ["claude@api.anthropic.com"] and checks[0]["ok"]
    assert "1742" not in out


def test_unknown_route_is_reported_as_other(tmp_path):
    session = "preflight-unknown"
    (tmp_path / "v1-responses-1.log").write_text(
        f"Session-Id: {session}\n\n=== API REQUEST 1 ===\nUpstream URL: https://secret-ish.example/v1\n"
        "Auth: provider=mystery, auth_id=x\n\n=== RESPONSE ===\nStatus: 200\n")
    from types import SimpleNamespace
    attempts = preflight.route_from_log(tmp_path, session, time.time() - 5, wait=0.5)
    assert attempts == [("mystery", "secret-ish.example")]
    args = SimpleNamespace(proxy_url=LOCAL, proxy_log_dir=str(tmp_path))
    import unittest.mock as mock
    with mock.patch.object(preflight.uuid, "uuid4", lambda: "unknown"), \
         mock.patch.object(preflight, "round_trip", lambda *a: {"ok": True}):
        route = preflight.probe_model(args, KIMI, "k", preflight.Deadline(5))[0]
    assert route["detail"]["actual"] == ["other"] and not route["ok"]


def test_docker_stderr_is_summarised_and_never_carries_the_key(env, capsys, tmp_path):
    """独审十八：docker/daemon 的 stderr 是自由文本，双重编码的 key 也不能经它进报告。"""
    (tmp_path / "key.txt").write_text('isolated"host"key1742\n')
    encoded = json.dumps(json.dumps('isolated"host"key1742')).replace("'", "'\\''")
    (tmp_path / "bin" / "docker").write_text(f"#!/bin/sh\n[ \"$1\" = rm ] && exit 0\necho 'daemon diagnostic: {encoded}' >&2\nexit 125\n")
    code, report, out, _ = env(capsys=capsys)
    stderr = check(report, "checkout_root_container_mount")["detail"]["stderr"]
    assert code == 2 and set(stderr) == {"class", "length", "sha256_12"} and "1742" not in out


@pytest.mark.parametrize("env_key", ['sk-\\"double\\"-1742', "TokenSecret1742abc"])
def test_env_key_misfilled_with_the_key_itself_is_not_echoed(env, capsys, tmp_path, env_key):
    """env_key 误填成 key 本身时，错误信息不回显它——含符合变量名语法的 key（正式 a2 Sol P2）。"""
    (tmp_path / "codex/config.toml").write_text(f'[model_providers.cliproxy-local]\nenv_key = "{env_key}"\n')
    code, report, out, _ = env(capsys=capsys)
    assert code == 2 and "1742" not in out
    assert "Missing environment variable" in check(report, "credential:codex-host")["detail"]["error"]


def test_auth_command_sees_the_checked_codex_home(env, capsys, tmp_path, monkeypatch):
    """独审二十一 B-1：helper 按 $CODEX_HOME 找 key 时，--codex-home 覆盖也要生效（与 codex 校验同一环境）。"""
    monkeypatch.delenv("CODEX_HOME", raising=False)
    (tmp_path / "codex/key.txt").write_text(KEY + "\n")
    (tmp_path / "codex/config.toml").write_text(
        '[model_providers.cliproxy-local]\n'
        '[model_providers.cliproxy-local.auth]\ncommand = "/bin/sh"\nargs = ["-c", "cat \\"$CODEX_HOME/key.txt\\""]\n')
    code, report, *_ = env(capsys=capsys)
    assert code == 0 and failing(report) == set()


def test_slow_but_successful_round_trip_within_the_work_deadline_passes(monkeypatch):
    """独审二十一 B-2：往返只受总工作时限约束；比旧的固定 45 秒更慢但在时限内完成的往返不误报。"""
    import io
    import urllib.request

    seen = {}

    class FakeResp(io.BytesIO):
        status = 200

        def __enter__(self):
            return self

        def __exit__(self, *a):
            return False

    def urlopen(req, timeout=None):
        seen["timeout"] = timeout
        return FakeResp(b'data: {"type":"response.completed","response":{"status":"completed"}}\n\n')

    monkeypatch.setattr(urllib.request, "urlopen", urlopen)
    trip = preflight.round_trip(LOCAL, KEY, KIMI, "s", preflight.Deadline(60))
    assert trip["ok"] and seen["timeout"] > 50


def test_every_subprocess_wait_comes_from_the_deadline_alone(tmp_path, monkeypatch):
    """独审二十二 B-3：docker run、docker rm、codex 校验都不能有低于剩余时限的固定上限（旧实现 30/5/10 秒）。"""
    from types import SimpleNamespace
    seen = {}

    def record(cmd, timeout, **kwargs):
        seen[cmd[1] if cmd[0] != "/bin/cat" else "helper"] = timeout
        out = b"k\n" if cmd[0] == "/bin/cat" else b""
        return subprocess.CompletedProcess(cmd, 0, out, b"")

    monkeypatch.setattr(preflight, "run_bounded", record)
    deadline = preflight.Deadline(60)
    preflight.check_mount(SimpleNamespace(checkout_root=str(tmp_path / "chk"), image="img"), deadline)
    home = tmp_path / "codex"
    home.mkdir()
    (home / "config.toml").write_text('[model_providers.cliproxy-local]\n[model_providers.cliproxy-local.auth]\n'
                                      'command = "/bin/cat"\nargs = ["k"]\ntimeout_ms = 120000\n')
    preflight.provider_key(home, deadline, "codex")
    assert seen["run"] > 50 and seen["rm"] > 50 and seen["features"] > 50 and seen["helper"] > 50



def test_mount_probe_runs_as_the_host_user_like_review_containers(env, capsys, tmp_path):
    """正式审阅 1（Sol）：探针目录是 0700，必须像正式审阅容器一样以宿主 uid:gid 运行，否则镜像用户不同时误拦。"""
    import os
    calls = tmp_path / "docker-calls"
    docker = tmp_path / "bin" / "docker"
    docker.write_text(docker.read_text().replace("#!/bin/sh\n", f'#!/bin/sh\necho "$@" >> {calls}\n', 1))
    code, report, *_ = env(capsys=capsys)
    run = next(line for line in calls.read_text().splitlines() if line.startswith("run "))
    assert code == 0 and f"--user {os.getuid()}:{os.getgid()}" in run


# ---------------- 正式审查 a4 Sol P2：key 与报告字段名、枚举值重合 ----------------

def _walk(value, fields, enums, field=None):
    if isinstance(value, dict):
        for k, v in value.items():
            fields.add(k)
            _walk(v, fields, enums, k)
    elif isinstance(value, list):
        for v in value:
            _walk(v, fields, enums, field)
    elif isinstance(value, str) and field in ("name", "class", "kind", "status", "terminal", "actual", "expected", "source"):
        enums.add(value)


@pytest.mark.parametrize("key", PROTOCOL_LIKE_KEYS)
def test_key_equal_to_a_protocol_token_does_not_rewrite_the_report(env, capsys, tmp_path, key):
    """合法 key 是「ok」「ready」等时，脱敏不能改写报告的字段名与枚举，否则调用方读不到结论。"""
    code, baseline, *_ = env(capsys=capsys)
    (tmp_path / "key.txt").write_text(key + "\n")
    code, report, _, proxy = env(capsys=capsys)
    assert code == 0 and {r["auth"] for r in proxy.requests} == {f"Bearer {key}"}
    assert report["ready"] is True and failing(report) == set()
    fields, enums, base_fields, base_enums = set(), set(), set(), set()
    _walk(report, fields, enums)
    _walk(baseline, base_fields, base_enums)
    assert (fields, enums) == (base_fields, base_enums)


def test_every_report_field_and_enum_is_in_the_protocol_vocabulary(env, capsys, monkeypatch):
    """新增报告字段或枚举必须登记进协议词表，否则 key 与它重合时会被脱敏改写。"""
    reports = [env(capsys=capsys)[1],
               env(routes={**OFFICIAL, OPUS: ANTIGRAVITY}, status={OPUS: 400}, capsys=capsys)[1]]
    monkeypatch.setenv("FAKE_DOCKER_EMPTY", "1")
    reports.append(env(capsys=capsys)[1])
    fields, enums = set(), set()
    for report in reports:
        _walk(report, fields, enums)
    assert fields <= preflight.PROTOCOL_FIELDS and enums <= preflight.PROTOCOL_VALUES
