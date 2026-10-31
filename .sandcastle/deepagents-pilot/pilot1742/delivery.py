from __future__ import annotations

import base64
import json
import os
import re
import subprocess
import time
from collections.abc import Callable
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any

from .domain import Sha, parse_sha
from .github_app import (
    GitHubAppClient,
    IdentityError,
    OutcomeUnknown,
    RequestNotSent,
    WriteRejected,
)

EffectResult = dict[str, Any]


class EffectPending(Exception):
    """一次外部写的结果不明，读回也没看到它生效。待认回记录保留，不自动重发，等读回确证或主控对账。"""


class Stale(Exception):
    def __init__(self, observed: str):
        super().__init__(observed)
        self.observed = observed


class Unpublishable(Exception):
    """本地对象无法经 GitHub git 数据 API 原样重建，或远端缺基线；没有创建任何 ref。"""


class Journal:
    """外部副作用的待认回记录。

    每个效果先读远端，读到就认领。prepare 做发送前的幂等准备（不算效果）。send 恰好发一个写请求，
    返回值不用；效果是否成立只由随后的读回判定。

    pending 写下后，只有 send 自己抛出 RequestNotSent（没发出）或 WriteRejected（4xx 拒绝）才撤销。
    其他任何异常，包括响应不可读、写后读回失败、读回内容不合合同，都保留 pending。之后幂等效果
    （同名标签、非强推建 ref）可以再发；非幂等效果（评论、PR、状态）只认读回，不因等待或空读回重发，
    只有主控对账写下 reconciled_not_applied 后才允许再发一次。
    """

    def __init__(self, folder: Path):
        self.folder = folder

    def path(self, effect: str) -> Path:
        return self.folder / f"{effect}.json"

    def load(self, effect: str) -> dict[str, Any] | None:
        path = self.path(effect)
        return json.loads(path.read_text()) if path.exists() else None

    def done(self, effect: str) -> bool:
        record = self.load(effect)
        return record is not None and record["state"] == "done"

    def _save(self, effect: str, record: dict[str, Any]) -> None:
        self.folder.mkdir(parents=True, exist_ok=True)
        path = self.path(effect)
        tmp = path.with_name(f"{path.name}.{os.getpid()}.tmp")
        tmp.write_text(json.dumps(record, ensure_ascii=False, indent=2) + "\n")
        os.replace(tmp, path)

    def _restore(self, effect: str, previous: dict[str, Any] | None) -> None:
        if previous is None:
            self.path(effect).unlink(missing_ok=True)
        else:
            self._save(effect, previous)

    def converge(self, effect: str, probe: Callable[[], EffectResult | None], send: Callable[[], None], *,
                 idempotent: bool, what: dict[str, Any], prepare: Callable[[], None] | None = None,
                 crash: Callable[[str], None] = lambda _p: None) -> EffectResult:
        record = self.load(effect)
        if record is not None and record["state"] == "done":
            return record["result"]
        found = probe()
        if found is not None:
            self._save(effect, {"state": "done", "result": found, "recovered": record is not None, "what": what})
            return found
        if record is not None and record["state"] == "pending" and not idempotent:
            raise EffectPending(f"{effect} 上次请求结果不明，读回未见生效；不自动重发，等读回确证或主控对账：{self.path(effect)}")
        if prepare is not None:
            prepare()
        history = (record or {}).get("history", []) + ([record] if record is not None else [])
        self._save(effect, {"state": "pending", "sent_at": time.time(), "pid": os.getpid(), "what": what,
                            "history": history})
        try:
            send()
        except (RequestNotSent, WriteRejected):
            self._restore(effect, record)
            raise
        except OutcomeUnknown:
            pass
        crash(f"after_{effect.split('-')[0]}_before_receipt")
        found = probe()
        if found is None:
            raise EffectPending(f"{effect} 已发出，读回未见生效，已留待认回：{self.path(effect)}")
        self._save(effect, {"state": "done", "result": found, "recovered": False, "what": what})
        return found

    def reconcile_not_applied(self, effect: str, note: str) -> dict[str, Any]:
        """主控核实 GitHub 上确实没有该效果后调用；下一次 converge 先读回，仍不见才再发一次。"""
        record = self.load(effect)
        if record is None or record["state"] != "pending":
            raise EffectPending(f"{effect} 不是 pending，无需对账")
        reconciled = {"state": "reconciled_not_applied", "note": note, "at": time.time(), "what": record.get("what"),
                      "history": record.get("history", []) + [record]}
        self._save(effect, reconciled)
        return reconciled


def marker(kind: str, *, ticket: int, run_id: str, generation: int, head: str) -> str:
    return f"<!-- deepagents:{kind} ticket={ticket} run={run_id} gen={generation} head={head} -->"


class RepoGateway:
    """新执行器在 GitHub 上的全部读写。只经 App installation token，不碰用户凭据。"""

    def __init__(self, client: GitHubAppClient):
        self.client = client
        self.repo = client.identity.repository
        self.owner = self.repo.split("/")[0]
        self.bot = client.bot_login

    def _p(self, suffix: str) -> str:
        return f"/repos/{self.repo}{suffix}"

    def issue(self, number: int) -> dict[str, Any]:
        data = self.client.api("GET", self._p(f"/issues/{number}")).data
        if not isinstance(data, dict) or data.get("number") != number:
            raise IdentityError("票面读回不符，字段 number")
        return data

    @staticmethod
    def label_names(issue: dict[str, Any]) -> frozenset[str]:
        labels = issue.get("labels")
        if not isinstance(labels, list) or not all(isinstance(x, dict) and isinstance(x.get("name"), str)
                                                    for x in labels):
            raise IdentityError("票面标签读回不完整")
        return frozenset(x["name"] for x in labels)

    def timeline(self, number: int) -> list[dict[str, Any]]:
        return self.client.paginate(self._p(f"/issues/{number}/timeline"))

    def parent(self, number: int) -> int | None:
        result = self.client.api("GET", self._p(f"/issues/{number}/parent"), tolerate=frozenset({404}))
        if result.status == 404:
            return None
        parent_number = (result.data or {}).get("number")
        if type(parent_number) is not int or parent_number <= 0:
            raise IdentityError("父票读回不完整")
        return parent_number

    def queue(self, label: str) -> list[dict[str, Any]]:
        return self.client.paginate(self._p("/issues"), {"labels": label, "state": "open"})

    def has_parent(self, number: int) -> bool:
        result = self.client.api("GET", self._p(f"/issues/{number}/parent"), tolerate=frozenset({404}))
        return result.status != 404

    def find_marker(self, number: int, text: str) -> dict[str, Any] | None:
        for comment in self.client.paginate(self._p(f"/issues/{number}/comments")):
            user = comment.get("user") or {}
            if user.get("login") == self.bot and user.get("type") == "Bot" and text in (comment.get("body") or ""):
                return {"id": comment["id"], "url": comment.get("html_url")}
        return None

    def post_comment(self, number: int, body: str) -> None:
        self.client.api("POST", self._p(f"/issues/{number}/comments"), json={"body": body})

    def add_label(self, number: int, name: str) -> None:
        self.client.api("POST", self._p(f"/issues/{number}/labels"), json={"labels": [name]})

    def remove_label(self, number: int, name: str) -> None:
        self.client.api("DELETE", self._p(f"/issues/{number}/labels/{name}"), tolerate=frozenset({404}))

    def commit_exists(self, sha: str) -> bool:
        return self.client.api("GET", self._p(f"/git/commits/{sha}"), tolerate=frozenset({404})).status != 404

    def get_ref(self, branch: str) -> str | None:
        result = self.client.api("GET", self._p(f"/git/ref/heads/{branch}"), tolerate=frozenset({404}))
        if result.status == 404:
            return None
        obj = (result.data or {}).get("object") or {}
        if result.data.get("ref") != f"refs/heads/{branch}" or obj.get("type") != "commit":
            raise IdentityError("远端分支读回不符，字段 ref")
        return parse_sha(obj.get("sha"))

    def create_ref(self, branch: str, sha: str) -> None:
        self.client.api("POST", self._p("/git/refs"), json={"ref": f"refs/heads/{branch}", "sha": sha})

    def create_object(self, kind: str, payload: dict[str, Any]) -> str:
        return str(self.client.api("POST", self._p(f"/git/{kind}"), json=payload).data["sha"])

    def pulls_for(self, branch: str) -> list[dict[str, Any]]:
        return self.client.paginate(self._p("/pulls"), {"head": f"{self.owner}:{branch}", "state": "all"})

    def create_pr(self, branch: str, title: str, body: str) -> None:
        self.client.api("POST", self._p("/pulls"), json={
            "title": title, "head": branch, "base": "main", "body": body, "draft": True})

    def get_pr(self, number: int) -> dict[str, Any]:
        return self.client.api("GET", self._p(f"/pulls/{number}")).data

    def statuses(self, sha: str) -> list[dict[str, Any]]:
        return self.client.paginate(self._p(f"/commits/{sha}/statuses"))

    def create_status(self, sha: str, context: str, description: str) -> None:
        self.client.api("POST", self._p(f"/statuses/{sha}"), json={
            "state": "success", "context": context, "description": description[:140]})


def _git_bytes(checkout: Path, *args: str) -> bytes:
    return subprocess.run(["git", "-C", str(checkout), *args], check=True, capture_output=True).stdout


_IDENT = re.compile(rb"^(.*) <(.*)> ([0-9]+) ([+-])([0-9]{2})([0-9]{2})$")


def _ident(raw: bytes) -> dict[str, str]:
    match = _IDENT.match(raw)
    if not match:
        raise Unpublishable("提交身份行无法解析")
    name, email, epoch, sign, hh, mm = match.groups()
    offset = timedelta(hours=int(hh), minutes=int(mm)) * (-1 if sign == b"-" else 1)
    moment = datetime.fromtimestamp(int(epoch), timezone(offset))
    date = moment.strftime("%Y-%m-%dT%H:%M:%SZ") if offset == timedelta(0) else moment.isoformat()
    return {"name": name.decode(), "email": email.decode(), "date": date}


def commit_payload(raw: bytes) -> dict[str, Any]:
    """把本地提交对象翻成 git 数据 API 的请求体。只接受 tree/parent/author/committer 四种头。"""
    head, sep, message = raw.partition(b"\n\n")
    if not sep:
        raise Unpublishable("提交对象缺正文分隔")
    fields: dict[bytes, list[bytes]] = {}
    for line in head.split(b"\n"):
        key, _, value = line.partition(b" ")
        if key not in (b"tree", b"parent", b"author", b"committer"):
            raise Unpublishable(f"提交含 API 无法重建的头：{key.decode(errors='replace')}")
        fields.setdefault(key, []).append(value)
    if len(fields.get(b"tree", [])) != 1 or len(fields.get(b"author", [])) != 1 or len(fields.get(b"committer", [])) != 1:
        raise Unpublishable("提交头不完整")
    try:
        text = message.decode()
    except UnicodeDecodeError:
        raise Unpublishable("提交说明不是 UTF-8") from None
    return {"message": text, "tree": fields[b"tree"][0].decode(),
            "parents": [p.decode() for p in fields.get(b"parent", [])],
            "author": _ident(fields[b"author"][0]), "committer": _ident(fields[b"committer"][0])}


@dataclass
class ExactUpload:
    """把 base..h 的对象逐个经 git 数据 API 重建，每个返回 SHA 必须等于本地 SHA。"""

    gateway: RepoGateway
    checkout: Path
    base: Sha

    def __post_init__(self) -> None:
        listing = _git_bytes(self.checkout, "ls-tree", "-r", "-t", "-z", self.base).split(b"\0")
        self.known = {entry.split(b"\t")[0].split()[2].decode() for entry in listing if entry}
        self.known.add(self.base)

    def _check(self, kind: str, expected: str, got: str) -> None:
        if got != expected:
            raise Unpublishable(f"GitHub 重建的 {kind} 与本地不同：期望 {expected}，得到 {got}")
        self.known.add(expected)

    def _blob(self, sha: str) -> None:
        if sha in self.known:
            return
        content = _git_bytes(self.checkout, "cat-file", "blob", sha)
        self._check("blob", sha, self.gateway.create_object(
            "blobs", {"content": base64.b64encode(content).decode(), "encoding": "base64"}))

    def _tree(self, sha: str) -> None:
        if sha in self.known:
            return
        entries = []
        for entry in _git_bytes(self.checkout, "ls-tree", "-z", sha).split(b"\0"):
            if not entry:
                continue
            meta, _, name = entry.partition(b"\t")
            mode, kind, obj = (part.decode() for part in meta.split())
            if kind == "tree":
                self._tree(obj)
            elif kind == "blob":
                self._blob(obj)
            entries.append({"path": name.decode(), "mode": mode, "type": kind, "sha": obj})
        self._check("tree", sha, self.gateway.create_object("trees", {"tree": entries}))

    def upload(self, head: Sha) -> None:
        if self.gateway.commit_exists(head):
            return
        if not self.gateway.commit_exists(self.base):
            raise Unpublishable(f"远端没有基线 {self.base}")
        commits = _git_bytes(self.checkout, "rev-list", "--reverse", "--topo-order", f"{self.base}..{head}").split()
        for raw_sha in commits:
            sha = raw_sha.decode()
            payload = commit_payload(_git_bytes(self.checkout, "cat-file", "commit", sha))
            for parent in payload["parents"]:
                if parent not in self.known and not self.gateway.commit_exists(parent):
                    raise Unpublishable(f"父提交 {parent} 不在远端也不在本次发布范围")
            self._tree(payload["tree"])
            self._check("commit", sha, self.gateway.create_object("commits", payload))
