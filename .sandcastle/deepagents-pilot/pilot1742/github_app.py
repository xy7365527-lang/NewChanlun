from __future__ import annotations

import argparse
import json
import os
import re
import ssl
import stat
import sys
import time
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

import certifi
import httpx
import jwt

API = "https://api.github.com"
INSTALLATION_PERMISSIONS = {
    "actions": "read", "metadata": "read", "contents": "write",
    "issues": "write", "pull_requests": "write", "statuses": "write",
}
READ_PERMISSIONS = dict.fromkeys(INSTALLATION_PERMISSIONS, "read")
WRITE_PERMISSIONS = {"metadata": "read", "contents": "write", "issues": "write", "pull_requests": "write",
                     "statuses": "write"}
MAX_PAGES = 50


class IdentityError(RuntimeError):
    pass


class OutcomeUnknown(IdentityError):
    """请求可能已在 GitHub 生效：发出后传输失败或 5xx。写操作只能读回认领，不能凭等待重发。"""


class RequestNotSent(IdentityError):
    """连接未建立，请求没有发出，GitHub 上不可能有副作用。"""


class WriteRejected(IdentityError):
    """GitHub 对这一次写请求明确返回 4xx：请求被拒，没有副作用。"""


@dataclass(frozen=True)
class ApiResponse:
    status: int
    data: Any
    has_next: bool


def _require_fields(label: str, checks: dict[str, bool]) -> None:
    failed = [field for field, valid in checks.items() if not valid]
    if failed:
        raise IdentityError(f"{label}不符，字段 " + ", ".join(failed))


@dataclass(frozen=True)
class AppIdentity:
    app_id: int
    installation_id: int
    repository_id: int
    repository: str
    app_slug: str
    private_key_path: Path

    def __post_init__(self) -> None:
        if any(type(value) is not int or value <= 0 for value in
               (self.app_id, self.installation_id, self.repository_id)):
            raise IdentityError("App、installation 和 repository ID 必须为正整数")
        if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", self.repository):
            raise IdentityError("repository 必须是 owner/name")
        if not re.fullmatch(r"[a-z0-9-]+", self.app_slug):
            raise IdentityError("App slug 格式不正确")
        if not self.private_key_path.is_absolute():
            raise IdentityError("私钥必须通过绝对文件路径明确指定")


def _read_private_key(path: Path) -> bytes:
    try:
        directory = path.resolve().parent
        if any((ancestor / ".git").exists() for ancestor in (directory, *directory.parents)):
            raise IdentityError("私钥必须保存在所有 Git 检出之外")
        descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        with os.fdopen(descriptor, "rb") as stream:
            info = os.fstat(stream.fileno())
            if (not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid()
                    or stat.S_IMODE(info.st_mode) not in (0o600, 0o400)):
                raise IdentityError("私钥须由当前用户持有且权限为 600 或 400")
            return stream.read()
    except OSError:
        raise IdentityError("无法读取明确指定的私钥文件") from None


class GitHubAppClient:
    def __init__(self, identity: AppIdentity, *, transport: httpx.BaseTransport | None = None,
                 permissions: dict[str, str] | None = None):
        self.identity = identity
        self._permissions = dict(permissions or READ_PERMISSIONS)
        grantable = {"read": ("read", "write"), "write": ("write",)}
        if any(INSTALLATION_PERMISSIONS.get(name) not in grantable.get(level, ())
               for name, level in self._permissions.items()):
            raise IdentityError("请求的 token 权限超出 App 安装权限")
        tls = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
        tls.load_verify_locations(cafile=certifi.where())
        self._http = httpx.Client(
            base_url=API, trust_env=False, follow_redirects=False, timeout=20,
            transport=transport, verify=tls,
            headers={"Accept": "application/vnd.github+json", "X-GitHub-Api-Version": "2026-03-10"},
        )
        self._token = ""
        self._expires_at = 0.0

    def __enter__(self) -> GitHubAppClient:
        return self

    def __exit__(self, *_: object) -> None:
        self.close()

    def close(self) -> None:
        self._token = ""
        self._expires_at = 0.0
        self._http.close()

    @property
    def bot_login(self) -> str:
        return f"{self.identity.app_slug}[bot]"

    def _send(self, method: str, path: str, bearer: str, **kwargs: Any) -> httpx.Response:
        try:
            response = self._http.request(method, path, headers={"Authorization": f"Bearer {bearer}"}, **kwargs)
        except (httpx.ConnectError, httpx.ConnectTimeout, httpx.PoolTimeout):
            raise RequestNotSent("GitHub 请求失败（连接未建立），未切换凭据来源") from None
        except httpx.HTTPError:
            raise OutcomeUnknown("GitHub 请求失败，未切换凭据来源") from None
        if response.status_code >= 500:
            raise OutcomeUnknown(f"GitHub 返回 HTTP {response.status_code}，未切换凭据来源")
        return response

    def _request(self, method: str, path: str, bearer: str, **kwargs: Any) -> dict[str, Any]:
        response = self._send(method, path, bearer, **kwargs)
        if response.status_code not in (200, 201):
            raise IdentityError(f"GitHub 返回 HTTP {response.status_code}，未切换凭据来源")
        try:
            data = response.json()
        except ValueError:
            raise IdentityError("GitHub 返回了非 JSON 对象") from None
        if not isinstance(data, dict):
            raise IdentityError("GitHub 返回了非 JSON 对象")
        return data

    def _refresh(self) -> None:
        identity = self.identity
        try:
            signed = jwt.encode(
                {"iat": int(time.time()) - 60, "exp": int(time.time()) + 540, "iss": str(identity.app_id)},
                _read_private_key(identity.private_key_path), algorithm="RS256",
            )
        except (ValueError, TypeError, jwt.PyJWTError):
            raise IdentityError("指定文件不是可用的 GitHub App RSA 私钥") from None
        app = self._request("GET", "/app", signed)
        _require_fields("GitHub App 身份", {
            "id": app.get("id") == identity.app_id,
            "slug": app.get("slug") == identity.app_slug,
        })
        installation = self._request("GET", f"/app/installations/{identity.installation_id}", signed)
        account = installation.get("account")
        _require_fields("安装身份", {
            "id": installation.get("id") == identity.installation_id,
            "app_id": installation.get("app_id") == identity.app_id,
            "app_slug": installation.get("app_slug") == identity.app_slug,
            "account.login": isinstance(account, dict) and account.get("login") == identity.repository.split("/")[0],
            "repository_selection": installation.get("repository_selection") == "selected",
            "suspended_at": installation.get("suspended_at") is None,
            "permissions": installation.get("permissions") == INSTALLATION_PERMISSIONS,
        })
        grant = self._request(
            "POST", f"/app/installations/{identity.installation_id}/access_tokens", signed,
            json={"repository_ids": [identity.repository_id], "permissions": self._permissions},
        )
        label = "短期只读授权" if self._permissions == READ_PERMISSIONS else "短期写授权"
        token = grant.get("token")
        try:
            expiry = datetime.fromisoformat(grant["expires_at"].replace("Z", "+00:00"))
            expires_at = expiry.timestamp() if expiry.tzinfo is not None else 0
        except (KeyError, ValueError, TypeError, AttributeError):
            raise IdentityError(f"{label}不符，字段 expires_at") from None
        if not isinstance(token, str) or not token:
            raise IdentityError(f"{label}不符，字段 token")
        _require_fields(label, {
            "expires_at": expires_at > time.time() + 60,
            "permissions": grant.get("permissions") == self._permissions,
        })
        self._token = token
        self._expires_at = expires_at

    def _ensure_token(self) -> str:
        if self._expires_at <= time.time() + 60:
            self._refresh()
        return self._token

    def api(self, method: str, path: str, *, json: Any = None, params: dict[str, Any] | None = None,
            tolerate: frozenset[int] = frozenset()) -> ApiResponse:
        """仓库内 REST 调用。只接受相对路径；tolerate 中的 4xx 作为确定结果返回，其余非 2xx 抛错。

        写请求只有两种结果能证明没有副作用：RequestNotSent（没发出）与 WriteRejected（4xx）。
        写请求得到 2xx 但正文不可读时报 OutcomeUnknown，因为写可能已经生效。"""
        if not path.startswith("/repos/" + self.identity.repository + "/"):
            raise IdentityError("App 客户端只访问已核验的仓库")
        writes = method != "GET"
        response = self._send(method, path, self._ensure_token(), json=json, params=params)
        if response.status_code not in (200, 201, 204) and response.status_code not in tolerate:
            if writes and 400 <= response.status_code < 500:
                raise WriteRejected(f"GitHub 以 HTTP {response.status_code} 拒绝写请求，未切换凭据来源")
            raise IdentityError(f"GitHub 返回 HTTP {response.status_code}，未切换凭据来源")
        try:
            data = response.json() if response.content else None
        except ValueError:
            if writes:
                raise OutcomeUnknown("GitHub 接受了写请求但返回了非 JSON 内容") from None
            raise IdentityError("GitHub 返回了非 JSON 内容") from None
        return ApiResponse(response.status_code, data, "next" in response.links)

    def paginate(self, path: str, params: dict[str, Any] | None = None) -> list[Any]:
        """读完全部分页；任何一页失败或超过上限都抛错，调用方不能把缺页当成空集。"""
        items: list[Any] = []
        for page in range(1, MAX_PAGES + 1):
            result = self.api("GET", path, params={**(params or {}), "per_page": 100, "page": page})
            if not isinstance(result.data, list):
                raise IdentityError("GitHub 分页结果不是列表")
            items.extend(result.data)
            if not result.has_next:
                return items
        raise IdentityError("分页超过上限，证据不完整")

    def verify(self) -> dict[str, Any]:
        self._ensure_token()
        identity = self.identity
        listing = self._request("GET", "/installation/repositories?per_page=100", self._token)
        repos = listing.get("repositories")
        if not isinstance(repos, list) or len(repos) != 1 or not isinstance(repos[0], dict):
            raise IdentityError("installation token 的仓库范围不符，字段 repositories")
        _require_fields("installation token 的仓库范围", {"total_count": listing.get("total_count") == 1})
        repo = repos[0]
        _require_fields("installation token 的仓库范围", {
            "repositories.id": repo.get("id") == identity.repository_id,
            "repositories.full_name": repo.get("full_name") == identity.repository,
        })
        repository = self._request("GET", f"/repos/{identity.repository}", self._token)
        _require_fields("GitHub 仓库身份", {
            "id": repository.get("id") == identity.repository_id,
            "full_name": repository.get("full_name") == identity.repository,
        })
        return {
            "verified": True, "app_id": identity.app_id, "app_slug": identity.app_slug,
            "installation_id": identity.installation_id, "repository_id": identity.repository_id,
            "repository": identity.repository, "credential_source": "explicit_private_key_file",
            "token_permissions": dict(self._permissions),
            "expires_at": datetime.fromtimestamp(self._expires_at, timezone.utc).isoformat(),
        }


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="pilot1742.github_app", description="只读核验独立 App 身份，不启动工蜂")
    parser.add_argument("--app-id", type=int, default=5486877)
    parser.add_argument("--installation-id", type=int, default=175617978)
    parser.add_argument("--repository-id", type=int, default=1150335160)
    parser.add_argument("--repository", default="xy7365527-lang/NewChanlun")
    parser.add_argument("--app-slug", default="newchanlun-deepagents-1742")
    parser.add_argument("--private-key", required=True, type=Path, help="仓库外 PEM 的绝对路径，不接受内容或环境 token")
    args = parser.parse_args(argv)
    try:
        identity = AppIdentity(args.app_id, args.installation_id, args.repository_id,
                               args.repository, args.app_slug, args.private_key)
        with GitHubAppClient(identity) as client:
            print(json.dumps(client.verify(), ensure_ascii=False, indent=2))
    except IdentityError as error:
        print(str(error), file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
