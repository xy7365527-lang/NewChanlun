import json
import os
import ssl
import subprocess
import sys
from dataclasses import replace
from datetime import datetime, timedelta, timezone
from pathlib import Path

import httpx
import jwt
import pytest
from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric import rsa

from pilot1742 import github_app as module
from pilot1742.github_app import (
    INSTALLATION_PERMISSIONS,
    READ_PERMISSIONS,
    AppIdentity,
    GitHubAppClient,
    IdentityError,
)


@pytest.fixture
def identity(tmp_path):
    key = rsa.generate_private_key(public_exponent=65537, key_size=2048)
    path = tmp_path / "fixture.pem"
    path.write_bytes(key.private_bytes(
        serialization.Encoding.PEM, serialization.PrivateFormat.PKCS8, serialization.NoEncryption(),
    ))
    path.chmod(0o600)
    return AppIdentity(5486877, 175617978, 1150335160, "xy7365527-lang/NewChanlun",
                       "newchanlun-deepagents-1742", path)


class GitHubFixture:
    def __init__(self, identity):
        self.identity = identity
        self.requests = []
        self.overrides = {}
        self.token_count = 0
        self.public_key = serialization.load_pem_private_key(
            identity.private_key_path.read_bytes(), password=None,
        ).public_key()

    def __call__(self, request):
        self.requests.append(request)
        assert request.url.host == "api.github.com"
        path = request.url.path
        identity = self.identity
        if path in self.overrides:
            return self.overrides[path]
        if path.startswith("/app"):
            claims = jwt.decode(request.headers["Authorization"].removeprefix("Bearer "),
                                self.public_key, algorithms=["RS256"])
            assert claims["iss"] == str(identity.app_id)
            assert claims["exp"] - claims["iat"] <= 600
        else:
            assert request.headers["Authorization"] == "Bearer fixture-installation-token"
        if path == "/app":
            data = {"id": identity.app_id, "slug": identity.app_slug}
        elif path == f"/app/installations/{identity.installation_id}":
            data = {
                "id": identity.installation_id, "app_id": identity.app_id, "app_slug": identity.app_slug,
                "account": {"login": identity.repository.split("/")[0]}, "repository_selection": "selected",
                "permissions": INSTALLATION_PERMISSIONS, "suspended_at": None,
            }
        elif path.endswith("/access_tokens"):
            self.token_count += 1
            assert request.method == "POST"
            assert json.loads(request.content) == {
                "repository_ids": [identity.repository_id], "permissions": READ_PERMISSIONS,
            }
            data = {
                "token": "fixture-installation-token", "permissions": READ_PERMISSIONS,
                "expires_at": (datetime.now(timezone.utc) + timedelta(hours=1)).isoformat(),
            }
        elif path == "/installation/repositories":
            data = {"total_count": 1, "repositories": [
                {"id": identity.repository_id, "full_name": identity.repository},
            ]}
        elif path == f"/repos/{identity.repository}":
            data = {"id": identity.repository_id, "full_name": identity.repository}
        else:
            pytest.fail(f"Unexpected request {path}")
        return httpx.Response(201 if path.endswith("/access_tokens") else 200, json=data)


def test_verify_and_reuse_short_lived_token(identity, monkeypatch):
    for name in ("GH_TOKEN", "GITHUB_TOKEN", "GH_ENTERPRISE_TOKEN", "GITHUB_ENTERPRISE_TOKEN"):
        monkeypatch.setenv(name, "forbidden-user-token")
    monkeypatch.setenv("NETRC", "/forbidden/netrc")
    fixture = GitHubFixture(identity)
    with GitHubAppClient(identity, transport=httpx.MockTransport(fixture)) as client:
        result = client.verify()
        assert result["verified"] is True
        assert result["credential_source"] == "explicit_private_key_file"
        assert result["token_permissions"] == READ_PERMISSIONS
        assert "fixture-installation-token" not in json.dumps(result)
        client.verify()
        assert fixture.token_count == 1
        client._expires_at = 0
        client.verify()
        assert fixture.token_count == 2
    assert client._token == ""
    assert os.environ["GH_TOKEN"] == "forbidden-user-token"
    assert all("forbidden-user-token" not in str(request.headers) for request in fixture.requests)


def test_missing_key_never_uses_user_credentials(identity, monkeypatch):
    monkeypatch.setenv("GH_TOKEN", "forbidden-user-token")
    monkeypatch.setattr(subprocess, "run", lambda *_a, **_kw: pytest.fail("must not invoke gh or git"))
    fixture = GitHubFixture(identity)
    missing = replace(identity, private_key_path=identity.private_key_path.with_name("absent.pem"))
    with GitHubAppClient(missing, transport=httpx.MockTransport(fixture)) as client:
        with pytest.raises(IdentityError, match="无法读取"):
            client.verify()
    assert fixture.requests == []


@pytest.mark.parametrize("status", [401, 403, 302])
def test_api_failure_has_no_fallback_or_secret_output(identity, monkeypatch, status):
    monkeypatch.setenv("GITHUB_TOKEN", "forbidden-user-token")
    fixture = GitHubFixture(identity)
    fixture.overrides["/app"] = httpx.Response(
        status, json={"message": "fixture-installation-token"},
        headers={"location": "https://example.invalid/steal"},
    )
    with GitHubAppClient(identity, transport=httpx.MockTransport(fixture)) as client:
        with pytest.raises(IdentityError, match=f"HTTP {status}") as caught:
            client.verify()
        assert "fixture-installation-token" not in str(caught.value)
    assert len(fixture.requests) == 1


@pytest.mark.parametrize("change", [
    {"app_id": 42}, {"repository_selection": "all"}, {"account": {"login": "other"}},
    {"permissions": {**INSTALLATION_PERMISSIONS, "administration": "write"}},
    {"suspended_at": "2026-01-01T00:00:00Z"}, {"account": None},
])
def test_installation_mismatch_stops_before_token(identity, change):
    fixture = GitHubFixture(identity)
    data = {
        "id": identity.installation_id, "app_id": identity.app_id, "app_slug": identity.app_slug,
        "account": {"login": identity.repository.split("/")[0]}, "repository_selection": "selected",
        "permissions": INSTALLATION_PERMISSIONS, "suspended_at": None,
    }
    data.update(change)
    fixture.overrides[f"/app/installations/{identity.installation_id}"] = httpx.Response(200, json=data)
    with GitHubAppClient(identity, transport=httpx.MockTransport(fixture)) as client:
        with pytest.raises(IdentityError, match="安装身份"):
            client.verify()
    assert fixture.token_count == 0


def test_extra_repository_is_rejected(identity):
    fixture = GitHubFixture(identity)
    fixture.overrides["/installation/repositories"] = httpx.Response(200, json={
        "total_count": 2, "repositories": [
            {"id": identity.repository_id, "full_name": identity.repository},
            {"id": 99, "full_name": "other/repo"},
        ],
    })
    with GitHubAppClient(identity, transport=httpx.MockTransport(fixture)) as client:
        with pytest.raises(IdentityError, match="仓库范围"):
            client.verify()


def test_reject_writable_token(identity):
    fixture = GitHubFixture(identity)
    fixture.overrides[f"/app/installations/{identity.installation_id}/access_tokens"] = httpx.Response(201, json={
        "token": "fixture-installation-token", "permissions": INSTALLATION_PERMISSIONS,
        "expires_at": (datetime.now(timezone.utc) + timedelta(hours=1)).isoformat(),
    })
    with GitHubAppClient(identity, transport=httpx.MockTransport(fixture)) as client:
        with pytest.raises(IdentityError, match="短期只读"):
            client.verify()


@pytest.mark.parametrize("mode", [0o644, 0o640, 0o700, 0o500])
def test_reject_exposed_private_key(identity, mode):
    identity.private_key_path.chmod(mode)
    fixture = GitHubFixture(identity)
    with GitHubAppClient(identity, transport=httpx.MockTransport(fixture)) as client:
        with pytest.raises(IdentityError, match="600 或 400"):
            client.verify()
    assert fixture.requests == []


@pytest.mark.parametrize("mode", [0o400, 0o600])
def test_accept_exact_owner_only_key_modes(identity, mode):
    identity.private_key_path.chmod(mode)
    assert module._read_private_key(identity.private_key_path)


def test_explicit_path_required_and_cli_safe(identity, capsys, monkeypatch):
    with pytest.raises(IdentityError, match="绝对文件路径"):
        replace(identity, private_key_path=identity.private_key_path.relative_to(identity.private_key_path.parent))
    monkeypatch.setattr(sys, "argv", ["github_app"])
    with pytest.raises(SystemExit) as caught:
        module.main([])
    assert caught.value.code == 2
    assert module.main(["--private-key", "/nonexistent/explicit.pem"]) == 2
    output = capsys.readouterr()
    assert "无法读取" in output.err
    assert "Traceback" not in output.err


def test_reject_invalid_rsa_file(identity):
    identity.private_key_path.write_text("not-a-private-key")
    with GitHubAppClient(identity) as client:
        with pytest.raises(IdentityError, match="RSA 私钥"):
            client.verify()


def test_tls_context_never_creates_environment_keylog(identity, tmp_path, monkeypatch):
    keylog = tmp_path / "tls-secrets.log"
    monkeypatch.setenv("SSLKEYLOGFILE", str(keylog))
    monkeypatch.setenv("HTTPS_PROXY", "http://127.0.0.1:1")
    monkeypatch.setenv("ALL_PROXY", "http://127.0.0.1:1")
    with GitHubAppClient(identity) as client:
        assert client._http._mounts == {}
        context = client._http._transport._pool._ssl_context
        assert context.verify_mode == ssl.CERT_REQUIRED
        assert context.check_hostname is True
        assert context.minimum_version >= ssl.TLSVersion.TLSv1_2
        assert context.keylog_filename is None
        assert context.cert_store_stats()["x509_ca"] > 0
    assert not keylog.exists()


@pytest.mark.parametrize("marker_kind", ["directory", "file"])
def test_reject_private_key_in_any_git_checkout(identity, tmp_path, marker_kind):
    checkout = tmp_path / "other-checkout"
    checkout.mkdir()
    marker = checkout / ".git"
    if marker_kind == "directory":
        marker.mkdir()
    else:
        marker.write_text("gitdir: /fixture/worktrees/other\n")
    nested = checkout / "nested"
    nested.mkdir()
    key = nested / "fixture.pem"
    key.write_bytes(identity.private_key_path.read_bytes())
    key.chmod(0o600)
    with pytest.raises(IdentityError, match="所有 Git 检出之外"):
        module._read_private_key(key)


def test_fifo_private_key_returns_without_writer(identity, tmp_path):
    fifo = tmp_path / "not-a-key"
    os.mkfifo(fifo, 0o600)
    script = (
        "from pathlib import Path; import sys; "
        "from pilot1742.github_app import _read_private_key, IdentityError\n"
        "try: _read_private_key(Path(sys.argv[1]))\n"
        "except IdentityError as error: print(str(error)); raise SystemExit(2)\n"
    )
    result = subprocess.run(
        [sys.executable, "-c", script, str(fifo)], capture_output=True, text=True, timeout=5,
        env={"PATH": os.defpath, "PYTHONPATH": str(Path(module.__file__).resolve().parents[1])},
    )
    assert result.returncode == 2
    assert "600 或 400" in result.stdout
    assert "Traceback" not in result.stderr


def test_permission_result_cannot_mutate_next_grant(identity):
    fixture = GitHubFixture(identity)
    with GitHubAppClient(identity, transport=httpx.MockTransport(fixture)) as client:
        result = client.verify()
        result["token_permissions"]["contents"] = "write"
        client._expires_at = 0
        next_result = client.verify()
    assert next_result["token_permissions"]["contents"] == "read"
    assert READ_PERMISSIONS["contents"] == "read"
    assert fixture.token_count == 2


def test_mismatch_diagnostics_only_name_fields(identity):
    fixture = GitHubFixture(identity)
    fixture.overrides[f"/app/installations/{identity.installation_id}"] = httpx.Response(200, json={
        "id": identity.installation_id, "app_id": identity.app_id, "app_slug": identity.app_slug,
        "account": {"login": "must-not-appear"}, "repository_selection": "must-not-appear",
        "permissions": INSTALLATION_PERMISSIONS, "suspended_at": None,
    })
    with GitHubAppClient(identity, transport=httpx.MockTransport(fixture)) as client:
        with pytest.raises(IdentityError) as caught:
            client.verify()
    assert str(caught.value) == "安装身份不符，字段 account.login, repository_selection"
    assert "must-not-appear" not in str(caught.value)


def test_token_diagnostic_only_names_failed_field(identity):
    fixture = GitHubFixture(identity)
    fixture.overrides[f"/app/installations/{identity.installation_id}/access_tokens"] = httpx.Response(201, json={
        "token": "must-not-appear", "permissions": {"contents": "write"},
        "expires_at": (datetime.now(timezone.utc) + timedelta(hours=1)).isoformat(),
    })
    with GitHubAppClient(identity, transport=httpx.MockTransport(fixture)) as client:
        with pytest.raises(IdentityError) as caught:
            client.verify()
    assert str(caught.value) == "短期只读授权不符，字段 permissions"
    assert "must-not-appear" not in str(caught.value)
