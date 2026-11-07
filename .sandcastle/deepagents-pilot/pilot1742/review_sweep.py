"""#1742 正式审查的自动触发：PR 推送或追平后，不经人工转发，在最新 head 上发起 review-pr。

每轮 tick 列出指向 main 的开放 PR，按（PR，当前 base，当前 head）决定一件事（审查期间 main 前进时 run 记为
STALE、不发布终态；STALE 不算完成，base 变了就在新 base 下重新发起，否则 PR 会永远停在 pending）：
- 没有任何 run：并发未满时以子进程发起 attempt 1；
- 本 head 的审查进程还活着：等待；
- 有 run 但没有结论、进程已不在（中断）：同一 attempt 续跑一次，再次中断交操作员；
- 已有结论：不动。

发起前 base、head 提交须已在宿主对象库（EXTERNAL-REVIEW.md：入口和 reviewer 都不 fetch）。缺失时调用
owner 注入的 sync(number, base, head)；未注入则报 needs-source-sync，不发起。

合入（可选，传入 gh 端口时）：只在 main 的必需检查同时含 deepagents/review 与 ci-ok 时才开 GitHub
auto-merge——这样合入必等审查与整轮 CI，不会因审查单绿绕过 CI 失败；否则只报告 auto-merge-unsafe。
strict 下落后的 PR，若当前 head 的审查与 ci-ok 都已 success，就追平（系统可证明无冲突的合并，审查按补丁沿用），
每轮只追平一个，免得连锁重跑。

人工确认的窄口子（用户 2026-11-07 选择）：改动触及实盘下单或资金、密钥或凭证、CI 与分支保护配置三类路径
（HUMAN_GATED）的 PR 不自动合入、不自动追平，报 needs-human-confirm；之前已开的 auto-merge 会撤掉。
普通审查器代码不在其中。

范围：只处理 max_age 内有活动的非草稿、同仓 PR，不给陈年 PR 发起审查。
不做的事：同一 head 的显式重审（新 attempt）仍由操作员发起；旧 head 不能覆盖新 head 的状态由
StatusPublisher 负责。审查本身、沿用和并发都在 external_review。
"""
from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import time
from collections import Counter
from collections.abc import Callable
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any, Protocol

from .delivery import RepoGateway
from .external_review import STATE_ROOT, TOKEN_PERMISSIONS, ExternalReview, _git
from .github_app import AppIdentity, GitHubAppClient, IdentityError
from .run import _ps_rows

Rows = list[tuple[int, int, str, str]]
Spawn = Callable[[int, int, Path], int]
Sync = Callable[[int, str, str], None]
GATE = frozenset({"deepagents/review", "ci-ok"})
# 人工确认的边界按仓库实际入口定（2026-11-07 核对），不按名字猜；新增同类入口时更新本表。
#   实盘下单或资金：X 执行域、E 经济、B 域、唯一生产 launcher（会话命名约定 x_/e_/b_/production_），
#     E/B 经济事实库（账本 book、allocation、finance……）、trading、funding_record，Python 实盘 runner 与执行器。
#     不含：diagnostic_fill（头注释：不产生正式成交或持仓）、回测与行情抓取。
#   CI 与分支保护：CI 实际执行或读取的配置（workflows、actions、scripts、devskim 配置、CI 清单）；
#     不含 PR 或 issue 模板、copilot 说明。DevSkim 基线与误报清单按内容判（只在放宽扫描准入时确认）。
#   密钥或凭证：密钥材料与凭证配置文件本身；不含讲到 token 的普通代码或文档。
HUMAN_GATED: dict[str, re.Pattern[str]] = {
    "live-trading-or-funds": re.compile(
        r"^rust/src/bin/(x_|e_|b_|production_)[^/]*session[^/]*\.rs$"
        r"|^rust/src/(economic_session|trading|funding_record)/|^rust/src/funding_record\.rs$"
        r"|^trading_system/(live|execution)/|^src/newchan/trading/|^tws_margin_check\.py$"),
    "ci-or-branch-protection": re.compile(
        r"^\.github/(workflows|actions|scripts|devskim)/|^\.github/[^/]+\.txt$"),
    "secrets-or-credentials": re.compile(
        r"\.(pem|key|p12|pfx)$|(^|/)\.env(\.[^/]*)?$|(^|/)(secrets?|credentials?)\.(json|ya?ml|toml|ini)$",
        re.IGNORECASE),
}
SCAN_ADMISSION = (".github/devskim-baseline.json", ".github/devskim-reviewed-1488.json")


def human_gated(files: list[str], widened: Callable[[str], bool] = lambda path: True) -> dict[str, list[str]]:
    """命中人工确认三类的文件，按类别列出；空表示不需要人工确认。DevSkim 基线与误报清单只在
    widened(path) 为真（本 PR 新增了被放行的发现）时算 CI 配置改动。"""
    hits: dict[str, list[str]] = {}
    for name in files:
        if name in SCAN_ADMISSION:
            if widened(name):
                hits.setdefault("ci-or-branch-protection", []).append(name)
            continue
        for category, pattern in HUMAN_GATED.items():
            if pattern.search(name):
                hits.setdefault(category, []).append(name)
    return hits


def admission_keys(path: str, text: str | None) -> Counter[str]:
    """扫描准入文件里被放行的发现计数。基线按（规则，文件）计数：行号迁移不算放宽，同一文件同一规则
    多放行一条才算；误报清单按（文件，内容哈希，规则结果）计：文件内容变了还继续放行也算放宽。"""
    if not text:
        return Counter()
    data = json.loads(text)
    if path.endswith("devskim-baseline.json"):
        return Counter(key.rsplit("|", 1)[0] for key in data.get("findings", []))
    # 误报清单：files[].results[] 每条是对某个文件内容（sha256）某条规则的放行。
    return Counter(json.dumps([entry["path"], entry["sha256_bytes"], result], sort_keys=True)
                   for entry in data["files"] for result in entry.get("results", []))


class MergePort(Protocol):
    """写操作端口（宿主 gh 凭据）；App token 保持只读。"""
    def required_checks(self) -> set[str]: ...
    def pr_state(self, number: int) -> dict[str, Any]: ...
    def enable_auto_merge(self, number: int, head: str) -> None: ...
    def disable_auto_merge(self, number: int) -> None: ...
    def update_branch(self, number: int, head: str) -> None: ...
    def pr_files(self, number: int) -> list[str]: ...
    def file_at(self, ref: str, path: str) -> str | None: ...


class GhCli:
    def __init__(self, repository: str = "xy7365527-lang/NewChanlun"):
        self.repository = repository

    def _gh(self, *args: str) -> str:
        return subprocess.run(["gh", *args], capture_output=True, text=True, check=True).stdout

    def required_checks(self) -> set[str]:
        out = self._gh("api", f"repos/{self.repository}/branches/main/protection",
                       "--jq", ".required_status_checks.contexts[]")
        return set(out.split())

    def pr_state(self, number: int) -> dict[str, Any]:
        data = json.loads(self._gh("pr", "view", str(number), "-R", self.repository, "--json",
                                   "headRefOid,mergeStateStatus,autoMergeRequest,statusCheckRollup"))
        checks = {(c.get("name") or c.get("context")): (c.get("conclusion") or c.get("state") or "").lower()
                  for c in data["statusCheckRollup"]}
        return {"head": data["headRefOid"], "merge_state": data["mergeStateStatus"],
                "auto_merge": data["autoMergeRequest"] is not None, "checks": checks}

    def enable_auto_merge(self, number: int, head: str) -> None:
        self._gh("pr", "merge", str(number), "-R", self.repository, "--auto", "--merge", "--match-head-commit", head)

    def disable_auto_merge(self, number: int) -> None:
        self._gh("pr", "merge", str(number), "-R", self.repository, "--disable-auto")

    def update_branch(self, number: int, head: str) -> None:
        self._gh("api", "-X", "PUT", f"repos/{self.repository}/pulls/{number}/update-branch",
                 "-f", f"expected_head_sha={head}")

    def file_at(self, ref: str, path: str) -> str | None:
        try:
            return self._gh("api", f"repos/{self.repository}/contents/{path}?ref={ref}",
                            "-H", "Accept: application/vnd.github.raw")
        except subprocess.CalledProcessError:
            return None

    def pr_files(self, number: int) -> list[str]:
        out = self._gh("api", "--paginate", f"repos/{self.repository}/pulls/{number}/files?per_page=100",
                       "--jq", ".[] | .filename, (.previous_filename // empty)")
        return out.split()


def review_command(number: int, attempt: int, private_key: Path) -> list[str]:
    return [sys.executable, "-m", "pilot1742.external_review", "review-pr",
            "--pr", str(number), "--attempt", str(attempt), "--private-key", str(private_key)]


def process_alive(record: dict | None, rows: Rows) -> bool:
    """身份 = 记录的 pid 且命令行仍是同一 PR、同一 attempt 的 review-pr；僵尸和 pid 复用都不算。"""
    if not record:
        return False
    needle = f"review-pr --pr {record['pr']} --attempt {record['attempt']} "
    return any(pid == record["pid"] and not stat.startswith("Z") and needle in command + " "
               for pid, _, stat, command in rows)


class Sweep:
    def __init__(self, gateway: RepoGateway, lane: ExternalReview, spawn: Spawn, *,
                 state_root: Path = STATE_ROOT, max_running: int = 2, rows: Callable[[], Rows] = _ps_rows,
                 sync: Sync | None = None, merge: MergePort | None = None, max_age: timedelta = timedelta(hours=24),
                 now: Callable[[], datetime] = lambda: datetime.now(timezone.utc)):
        self.gateway, self.lane, self.spawn, self.rows, self.sync = gateway, lane, spawn, rows, sync
        self.merge, self.max_age, self.now = merge, max_age, now
        self.folder = state_root / "sweep"
        self.max_running = max_running

    def open_pulls(self) -> list[dict[str, Any]]:
        return self.gateway.client.paginate(self.gateway._p("/pulls"), {"state": "open", "base": "main"})

    def _record_path(self, number: int, base: str, head: str) -> Path:
        return self.folder / f"p{number}-{base[:12]}-{head}.json"

    def _read(self, number: int, base: str, head: str) -> dict | None:
        path = self._record_path(number, base, head)
        return json.loads(path.read_text()) if path.exists() else None

    def _has_commits(self, *shas: str) -> bool:
        try:
            for sha in shas:
                _git(self.lane.source, "cat-file", "-e", f"{sha}^{{commit}}")
        except subprocess.CalledProcessError:
            return False
        return True

    def running(self, rows: Rows) -> int:
        """所有仍活着的审查进程，含旧 head 上尚未结束的。"""
        if not self.folder.exists():
            return 0
        return sum(process_alive(json.loads(p.read_text()), rows) for p in self.folder.glob("p*.json"))

    def _launch(self, number: int, base: str, head: str, attempt: int, resumes: int) -> dict:
        self.folder.mkdir(parents=True, exist_ok=True, mode=0o700)
        pid = self.spawn(number, attempt, self.folder / "logs" / f"p{number}-{head[:12]}-a{attempt}.log")
        record = {"pr": number, "base": base, "head": head, "attempt": attempt, "pid": pid, "resumes": resumes,
                  "started_at": time.time()}
        tmp = self._record_path(number, base, head).with_suffix(".tmp")
        tmp.write_text(json.dumps(record))
        os.replace(tmp, self._record_path(number, base, head))
        return record

    def decide(self, pr: dict[str, Any], rows: Rows, running: int) -> tuple[str, dict | None]:
        number, head, base = pr["number"], pr["head"]["sha"], pr["base"]["sha"]
        if pr.get("draft"):
            return "draft", None
        if (pr["head"].get("repo") or {}).get("id") != pr["base"]["repo"]["id"]:
            return "foreign-head", None
        if not self._recent(pr):
            return "stale-pr", None
        record = self._read(number, base, head)
        if process_alive(record, rows):
            return "running", None
        runs = [row for row in self.lane.status(number)
                if row["target"].get("head_sha") == head and row["target"].get("base_sha") == base]
        if any(row["outcome"] and row["outcome"].get("verdict") != "STALE" for row in runs):
            return "done", None
        live = [row for row in runs if not row["outcome"]]  # STALE 不续跑，只看未到终态的
        if running >= self.max_running:
            return "queued", None
        if not self._has_commits(base, head):
            if self.sync is None:
                return "needs-source-sync", None
            self.sync(number, base, head)
            if not self._has_commits(base, head):
                return "needs-source-sync", None
        if not live:
            # 本 base/head 没有在途 run：首次发起，或此前只有 STALE（同一 base 下发布前 head/目标漂移），取下一个 attempt。
            attempt = max((row["target"]["attempt"] for row in runs), default=0) + 1
            if record is None or record["attempt"] < attempt:
                return "launch", self._launch(number, base, head, attempt, 0)
        # 有登记无结论、进程已不在：中断。只续跑记录过的那个 attempt，且只续一次。
        attempt = record["attempt"] if record else max(row["target"]["attempt"] for row in live)
        resumes = record["resumes"] if record else 0
        if resumes >= 1:
            return "needs-operator", None
        return "resume", self._launch(number, base, head, attempt, resumes + 1)

    def _recent(self, pr: dict[str, Any]) -> bool:
        updated = pr.get("updated_at")
        return updated is None or datetime.fromisoformat(updated.replace("Z", "+00:00")) >= self.now() - self.max_age

    def merge_actions(self, pulls: list[dict[str, Any]]) -> list[dict[str, Any]]:
        if self.merge is None:
            return []
        if not GATE <= self.merge.required_checks():
            return [{"action": "auto-merge-unsafe", "reason": f"main 必需检查须含 {sorted(GATE)}"}]
        actions, updated = [], False
        for pr in pulls:
            if pr.get("draft") or (pr["head"].get("repo") or {}).get("id") != pr["base"]["repo"]["id"] \
                    or not self._recent(pr):
                continue
            number, head = pr["number"], pr["head"]["sha"]
            state = self.merge.pr_state(number)
            if state["head"] != head:
                continue  # 本轮读到的 head 已变，下轮再说
            port = self.merge
            def widened(path: str, base: str = pr["base"]["sha"], head: str = head, port: MergePort = port) -> bool:
                try:
                    return bool(admission_keys(path, port.file_at(head, path))
                                - admission_keys(path, port.file_at(base, path)))
                except (ValueError, AttributeError, TypeError):
                    return True  # 读不懂就按放宽处理，交人工确认
            gated = human_gated(self.merge.pr_files(number), widened)
            if gated:
                if state["auto_merge"]:
                    self.merge.disable_auto_merge(number)
                actions.append({"pr": number, "head": head, "action": "needs-human-confirm", "gated": gated})
                continue
            if not state["auto_merge"]:
                self.merge.enable_auto_merge(number, head)
                actions.append({"pr": number, "head": head, "action": "auto-merge-on"})
            checks = state["checks"]
            if (not updated and state["merge_state"] == "BEHIND"
                    and all(checks.get(name) == "success" for name in GATE)):
                self.merge.update_branch(number, head)
                updated = True
                actions.append({"pr": number, "head": head, "action": "update-branch"})
        return actions

    def tick(self) -> list[dict[str, Any]]:
        rows = self.rows()
        pulls = sorted(self.open_pulls(), key=lambda pr: pr["number"])
        running = self.running(rows)
        actions = []
        for pr in pulls:
            action, record = self.decide(pr, rows, running)
            running += action in ("launch", "resume")
            actions.append({"pr": pr["number"], "head": pr["head"]["sha"], "action": action,
                            **({"pid": record["pid"], "attempt": record["attempt"]} if record else {})})
        return actions + self.merge_actions(pulls)


def spawner(private_key: Path) -> Spawn:
    def spawn(number: int, attempt: int, log: Path) -> int:
        log.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
        with open(log, "ab", opener=lambda p, f: os.open(p, f, 0o600)) as out:
            return subprocess.Popen(review_command(number, attempt, private_key), stdin=subprocess.DEVNULL,
                                    stdout=out, stderr=out, start_new_session=True,
                                    cwd=Path(__file__).resolve().parents[1]).pid
    return spawn


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="pilot1742.review_sweep")
    parser.add_argument("--private-key", required=True, type=Path)
    parser.add_argument("--max-running", type=int, default=2)
    parser.add_argument("--loop", type=float, default=0, help="大于 0 时按此秒数循环；0 只跑一轮")
    parser.add_argument("--auto-merge", action="store_true", help="用宿主 gh 凭据开 auto-merge 与追平（须 ci-ok 为必需检查）")
    args = parser.parse_args(argv)
    identity = AppIdentity(5486877, 175617978, 1150335160, "xy7365527-lang/NewChanlun",
                           "newchanlun-deepagents-1742", args.private_key)
    try:
        with GitHubAppClient(identity, permissions=TOKEN_PERMISSIONS) as client:
            client.verify()
            gateway = RepoGateway(client)
            sweep = Sweep(gateway, ExternalReview(gateway), spawner(args.private_key), max_running=args.max_running,
                          merge=GhCli() if args.auto_merge else None)
            while True:
                for action in sweep.tick():
                    if action["action"] not in ("done", "draft", "stale-pr"):
                        print(json.dumps(action, ensure_ascii=False), flush=True)
                if args.loop <= 0:
                    return 0
                time.sleep(args.loop)
    except (IdentityError, OSError, ValueError) as error:
        print(str(error), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
