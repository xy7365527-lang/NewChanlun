"""GitHub 替身：git 对象与 ref 落在真实临时裸仓库里，SHA 由 git 自己计算；票面、PR、状态在内存里。"""
from __future__ import annotations

import base64
import json
import re
import subprocess
import threading
from dataclasses import dataclass, field
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any

import httpx
import jwt
from cryptography.hazmat.primitives import serialization

from pilot1742.github_app import INSTALLATION_PERMISSIONS


@dataclass
class Fault:
    method: str
    pattern: str
    mode: str
    times: int = 1
    page: int | None = None
    entered: threading.Event = field(default_factory=threading.Event)
    release: threading.Event = field(default_factory=threading.Event)


def _git(bare: Path, *args: str, data: bytes | None = None) -> str:
    return subprocess.run(["git", "-C", str(bare), *args], input=data, check=True,
                          capture_output=True).stdout.decode().strip()


def _exists(bare: Path, sha: str, kind: str | None = None) -> bool:
    result = subprocess.run(["git", "-C", str(bare), "cat-file", "-t", sha], capture_output=True, text=True)
    return result.returncode == 0 and (kind is None or result.stdout.strip() == kind)


class FakeGitHub:
    def __init__(self, identity, bare: Path):
        self.identity = identity
        self.bare = bare
        self.repo = identity.repository
        self.bot = f"{identity.app_slug}[bot]"
        self.public_key = serialization.load_pem_private_key(
            identity.private_key_path.read_bytes(), password=None).public_key()
        self.issues: dict[int, dict[str, Any]] = {}
        self.pulls: dict[int, dict[str, Any]] = {}
        self.statuses: dict[str, list[dict[str, Any]]] = {}
        self.requests: list[tuple[str, str, int | None]] = []
        self.faults: list[Fault] = []
        self.on_request: list[Any] = []
        self.hidden: set[str] = set()
        self.tokens: dict[str, dict[str, str]] = {}
        self.next_number = 4000
        self.next_id = 1
        self.lock = threading.Lock()

    def add_issue(self, number: int, *, body: str, labels: tuple[str, ...] = (), state: str = "open",
                  parent: int | None = None, timeline: list[dict[str, Any]] | None = None) -> dict[str, Any]:
        issue = {"number": number, "state": state, "title": f"票 {number}", "body": body, "labels": list(labels),
                 "comments": [], "timeline": list(timeline or []), "parent": parent, "is_pr": False}
        for name in labels:
            issue["timeline"].append({"event": "labeled", "label": {"name": name}})
        self.issues[number] = issue
        return issue

    def fault(self, method: str, pattern: str, mode: str, *, times: int = 1, page: int | None = None) -> Fault:
        item = Fault(method, pattern, mode, times, page)
        self.faults.append(item)
        return item

    def count(self, method: str, pattern: str) -> int:
        return sum(1 for m, p, _ in self.requests if m == method and re.fullmatch(pattern, p))

    def ref(self, branch: str) -> str | None:
        result = subprocess.run(["git", "-C", str(self.bare), "rev-parse", "--verify", "-q", f"refs/heads/{branch}"],
                                capture_output=True, text=True)
        return result.stdout.strip() if result.returncode == 0 else None

    def _id(self) -> int:
        self.next_id += 1
        return self.next_id

    def _label(self, issue: dict[str, Any], name: str, event: str) -> None:
        issue["timeline"].append({"event": event, "label": {"name": name}})

    def _issue_json(self, issue: dict[str, Any]) -> dict[str, Any]:
        data = {"number": issue["number"], "state": issue["state"], "title": issue["title"], "body": issue["body"],
                "labels": [{"name": n} for n in issue["labels"]]}
        if issue["is_pr"]:
            data["pull_request"] = {"url": f"https://api.github.com/repos/{self.repo}/pulls/{issue['number']}"}
        return data

    def _pr_json(self, pr: dict[str, Any]) -> dict[str, Any]:
        head = self.ref(pr["branch"]) if pr["state"] == "open" else pr["frozen_head"]
        return {"number": pr["number"], "state": pr["state"], "merged": pr["merged"], "draft": pr["draft"],
                "html_url": f"https://github.com/{self.repo}/pull/{pr['number']}", "title": pr["title"],
                "body": pr["body"], "user": pr["user"], "head": {"ref": pr["branch"], "sha": head,
                                                                 "repo": {"full_name": self.repo}},
                "base": {"ref": pr["base"]}}

    def add_pull(self, branch: str, head: str, *, state: str = "closed", body: str = "", base: str = "main",
                 draft: bool = True, user: dict[str, str] | None = None) -> int:
        number = self.next_number = self.next_number + 1
        self.pulls[number] = {"number": number, "branch": branch, "base": base, "state": state, "merged": False,
                              "draft": draft, "title": "旧 PR", "body": body, "frozen_head": head,
                              "user": user or {"login": "xy7365527-lang", "type": "User"}}
        self.issues[number] = {"number": number, "state": state, "title": "旧 PR", "body": body, "labels": [],
                               "comments": [], "timeline": [], "parent": None, "is_pr": True}
        return number

    def merge_pull(self, number: int) -> None:
        self.close_pull(number)
        self.pulls[number]["merged"] = True

    def close_pull(self, number: int) -> None:
        pr = self.pulls[number]
        pr["frozen_head"] = self.ref(pr["branch"])
        pr["state"] = self.issues[number]["state"] = "closed"

    @staticmethod
    def _page(items: list[Any], params: httpx.QueryParams) -> tuple[list[Any], bool]:
        per = int(params.get("per_page", 30))
        page = int(params.get("page", 1))
        chunk = items[(page - 1) * per: page * per]
        return chunk, page * per < len(items)

    def __call__(self, request: httpx.Request) -> httpx.Response:
        assert request.url.host == "api.github.com"
        path, method = request.url.path, request.method
        page = int(request.url.params["page"]) if "page" in request.url.params else None
        with self.lock:
            self.requests.append((method, path, page))
        for hook in list(self.on_request):
            hook(method, path)
        fault = next((f for f in self.faults if f.times > 0 and f.method == method and re.fullmatch(f.pattern, path)
                      and (f.page is None or f.page == page)), None)
        if fault is not None:
            fault.times -= 1
            if fault.mode == "drop_before":
                raise httpx.ConnectError("fixture drop", request=request)
            if fault.mode == "status_500":
                return httpx.Response(500, json={"message": "fixture"})
            if fault.mode == "reject_422":
                return httpx.Response(422, json={"message": "fixture"})
            if fault.mode == "hold":
                fault.entered.set()
                assert fault.release.wait(20)
        response = self._route(request, method, path)
        if fault is not None and fault.mode == "drop_after":
            raise httpx.ReadTimeout("fixture lost response", request=request)
        if fault is not None and fault.mode == "garble_after":
            return httpx.Response(response.status_code, content=b"NOT JSON")
        return response

    def _auth(self, request: httpx.Request, path: str, method: str) -> None:
        bearer = request.headers["Authorization"].removeprefix("Bearer ")
        if path.startswith("/app"):
            claims = jwt.decode(bearer, self.public_key, algorithms=["RS256"])
            assert claims["iss"] == str(self.identity.app_id)
            return
        permissions = self.tokens[bearer]
        if method != "GET":
            area = ("contents" if "/git/" in path else "pull_requests" if "/pulls" in path
                    else "statuses" if "/statuses/" in path else "issues")
            assert permissions.get(area) == "write", f"token lacks {area}:write"

    def _route(self, request: httpx.Request, method: str, path: str) -> httpx.Response:
        self._auth(request, path, method)
        ident = self.identity
        body = json.loads(request.content) if request.content else None
        if path == "/app":
            return httpx.Response(200, json={"id": ident.app_id, "slug": ident.app_slug})
        if path == f"/app/installations/{ident.installation_id}":
            return httpx.Response(200, json={
                "id": ident.installation_id, "app_id": ident.app_id, "app_slug": ident.app_slug,
                "account": {"login": self.repo.split("/")[0]}, "repository_selection": "selected",
                "permissions": INSTALLATION_PERMISSIONS, "suspended_at": None})
        if path.endswith("/access_tokens"):
            token = f"fake-token-{len(self.tokens) + 1}"
            self.tokens[token] = body["permissions"]
            return httpx.Response(201, json={"token": token, "permissions": body["permissions"],
                                             "expires_at": (datetime.now(timezone.utc) + timedelta(hours=1)).isoformat()})
        if path == "/installation/repositories":
            return httpx.Response(200, json={"total_count": 1, "repositories": [
                {"id": ident.repository_id, "full_name": self.repo}]})
        prefix = f"/repos/{self.repo}"
        if path == prefix:
            return httpx.Response(200, json={"id": ident.repository_id, "full_name": self.repo})
        rel = path.removeprefix(prefix)
        for pattern, handler in self._routes():
            match = re.fullmatch(pattern, f"{method} {rel}")
            if match:
                return handler(request, body, *match.groups())
        raise AssertionError(f"unexpected {method} {rel}")

    def _routes(self):
        return [
            (r"GET /issues/(\d+)", self._get_issue),
            (r"GET /issues/(\d+)/timeline", self._timeline),
            (r"GET /issues/(\d+)/parent", self._parent),
            (r"GET /issues/(\d+)/comments", self._comments),
            (r"POST /issues/(\d+)/comments", self._post_comment),
            (r"POST /issues/(\d+)/labels", self._add_labels),
            (r"DELETE /issues/(\d+)/labels/(.+)", self._remove_label),
            (r"GET /git/commits/([0-9a-f]{40})", self._get_commit),
            (r"POST /git/blobs", self._blob),
            (r"POST /git/trees", self._tree),
            (r"POST /git/commits", self._commit),
            (r"GET /git/ref/heads/(.+)", self._get_ref),
            (r"POST /git/refs", self._create_ref),
            (r"GET /pulls", self._list_pulls),
            (r"POST /pulls", self._create_pull),
            (r"GET /pulls/(\d+)", self._get_pull),
            (r"POST /statuses/([0-9a-f]{40})", self._create_status),
            (r"GET /commits/([0-9a-f]{40})/statuses", self._list_statuses),
        ]

    def _issue(self, number: str) -> dict[str, Any] | None:
        return self.issues.get(int(number))

    def _get_issue(self, request, body, number):
        issue = self._issue(number)
        return httpx.Response(200, json=self._issue_json(issue)) if issue else httpx.Response(404)

    def _paged(self, request, items):
        chunk, more = self._page(items, request.url.params)
        headers = {}
        if more:
            page = int(request.url.params.get("page", 1)) + 1
            headers["Link"] = f'<{request.url.copy_set_param("page", page)}>; rel="next"'
        return httpx.Response(200, json=chunk, headers=headers)

    def _timeline(self, request, body, number):
        return self._paged(request, self._issue(number)["timeline"])

    def _parent(self, request, body, number):
        parent = self._issue(number)["parent"]
        return httpx.Response(404) if parent is None else httpx.Response(200, json={"number": parent})

    def _comments(self, request, body, number):
        visible = [c for c in self._issue(number)["comments"] if not any(h in c["body"] for h in self.hidden)]
        return self._paged(request, visible)

    def _post_comment(self, request, body, number):
        comment = {"id": self._id(), "body": body["body"], "user": {"login": self.bot, "type": "Bot"},
                   "html_url": f"https://github.com/{self.repo}/issues/{number}#issuecomment-{self.next_id}"}
        self._issue(number)["comments"].append(comment)
        return httpx.Response(201, json=comment)

    def _add_labels(self, request, body, number):
        issue = self._issue(number)
        for name in body["labels"]:
            if name not in issue["labels"]:
                issue["labels"].append(name)
                self._label(issue, name, "labeled")
        return httpx.Response(200, json=[{"name": n} for n in issue["labels"]])

    def _remove_label(self, request, body, number, name):
        issue = self._issue(number)
        if name not in issue["labels"]:
            return httpx.Response(404)
        issue["labels"].remove(name)
        self._label(issue, name, "unlabeled")
        return httpx.Response(200, json=[{"name": n} for n in issue["labels"]])

    def _get_commit(self, request, body, sha):
        return httpx.Response(200, json={"sha": sha}) if _exists(self.bare, sha, "commit") else httpx.Response(404)

    def _blob(self, request, body):
        sha = _git(self.bare, "hash-object", "-w", "--stdin", data=base64.b64decode(body["content"]))
        return httpx.Response(201, json={"sha": sha})

    def _tree(self, request, body):
        lines = b"".join(f"{e['mode']} {e['type']} {e['sha']}\t{e['path']}".encode() + b"\0" for e in body["tree"])
        return httpx.Response(201, json={"sha": _git(self.bare, "mktree", "-z", data=lines)})

    @staticmethod
    def _ident(person: dict[str, str]) -> str:
        moment = datetime.fromisoformat(person["date"].replace("Z", "+00:00"))
        offset = moment.utcoffset() or timedelta(0)
        minutes = int(offset.total_seconds() // 60)
        sign = "-" if minutes < 0 else "+"
        tz = f"{sign}{abs(minutes) // 60:02d}{abs(minutes) % 60:02d}"
        return f"{person['name']} <{person['email']}> {int(moment.timestamp())} {tz}"

    def _commit(self, request, body):
        assert _exists(self.bare, body["tree"], "tree")
        assert all(_exists(self.bare, p, "commit") for p in body["parents"])
        raw = (f"tree {body['tree']}\n" + "".join(f"parent {p}\n" for p in body["parents"])
               + f"author {self._ident(body['author'])}\ncommitter {self._ident(body['committer'])}\n\n"
               + body["message"])
        sha = _git(self.bare, "hash-object", "-t", "commit", "-w", "--stdin", data=raw.encode())
        return httpx.Response(201, json={"sha": sha})

    def _get_ref(self, request, body, branch):
        sha = self.ref(branch)
        if sha is None:
            return httpx.Response(404)
        return httpx.Response(200, json={"ref": f"refs/heads/{branch}", "object": {"sha": sha, "type": "commit"}})

    def _create_ref(self, request, body):
        ref, sha = body["ref"], body["sha"]
        if not _exists(self.bare, sha, "commit"):
            return httpx.Response(422)
        result = subprocess.run(["git", "-C", str(self.bare), "update-ref", ref, sha, ""], capture_output=True)
        if result.returncode != 0:
            return httpx.Response(422)
        return httpx.Response(201, json={"ref": ref, "object": {"sha": sha, "type": "commit"}})

    def _list_pulls(self, request, body):
        owner, _, branch = request.url.params["head"].partition(":")
        assert owner == self.repo.split("/")[0] and request.url.params["state"] == "all"
        return self._paged(request, [self._pr_json(p) for p in self.pulls.values() if p["branch"] == branch])

    def _create_pull(self, request, body):
        if self.ref(body["head"]) is None or any(p["branch"] == body["head"] and p["state"] == "open"
                                                 for p in self.pulls.values()):
            return httpx.Response(422)
        number = self.next_number = self.next_number + 1
        self.pulls[number] = {"number": number, "branch": body["head"], "base": body["base"], "state": "open",
                              "merged": False, "draft": body["draft"], "title": body["title"], "body": body["body"],
                              "frozen_head": None, "user": {"login": self.bot, "type": "Bot"}}
        self.issues[number] = {"number": number, "state": "open", "title": body["title"], "body": body["body"],
                               "labels": [], "comments": [], "timeline": [], "parent": None, "is_pr": True}
        return httpx.Response(201, json=self._pr_json(self.pulls[number]))

    def _get_pull(self, request, body, number):
        return httpx.Response(200, json=self._pr_json(self.pulls[int(number)]))

    def _create_status(self, request, body, sha):
        status = {"id": self._id(), "state": body["state"], "context": body["context"],
                  "description": body["description"], "creator": {"login": self.bot}}
        self.statuses.setdefault(sha, []).insert(0, status)
        return httpx.Response(201, json=status)

    def _list_statuses(self, request, body, sha):
        return self._paged(request, self.statuses.get(sha, []))
