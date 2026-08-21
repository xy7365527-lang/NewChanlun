#!/usr/bin/env python3
"""机械验证 .sandcastle 官方 GitHub Actions scaffold（#1110）。

验证面（对应 #1110 验收）：
  1. workflow YAML 结构不变式（trigger/label 闸、concurrency、权限最小化、
     checkout/base、确定性分支、Draft PR、force-with-lease、失败回写、防重）。
  2. issue shape 检测脚本（detect-issue-shape.sh）在 mock gh 下四形态对拍。
  3. 禁入模式扫描（不读本机 Prime/Codex auth；不复用 watcher/harvest/main-loop）。

用法：python3 .sandcastle/verify-workflows.py
退出码 0 = 全过；非 0 = 首条失败（附可读信息）。
"""
from __future__ import annotations

import os
import pathlib
import subprocess
import sys
import tempfile

import yaml

ROOT = pathlib.Path(__file__).resolve().parent.parent
WF_DIR = ROOT / ".github" / "workflows"
AW_DIR = ROOT / ".sandcastle" / "agent-workflows"

WORKFLOWS = [
    "agent-implement.yml",
    "agent-review.yml",
    "agent-implement-pr.yml",
    "agent-explore.yml",
    "agent-update-branch.yml",
]

_failures: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        _failures.append(msg)


class GHALoader(yaml.SafeLoader):
    """GitHub Actions 语义的 YAML：`on` 是普通字符串键，而非 YAML 1.1 布尔。"""


def _construct_bool(loader: GHALoader, node: yaml.Node) -> object:
    value = loader.construct_scalar(node)
    if value in ("true", "True", "TRUE"):
        return True
    if value in ("false", "False", "FALSE"):
        return False
    return value  # `on` / `off` / `yes` / `no` 保持字符串


GHALoader.add_constructor("tag:yaml.org,2002:bool", _construct_bool)


def load_wf(name: str) -> dict:
    return yaml.load((WF_DIR / name).read_text(), Loader=GHALoader)


def raw_wf(name: str) -> str:
    return (WF_DIR / name).read_text()


def job(wf: dict, name: str) -> dict:
    return wf["jobs"][name]


def main() -> int:
    # ── 1. 结构不变式 ─────────────────────────────────────────────────────────
    impl = load_wf("agent-implement.yml")
    impl_raw = raw_wf("agent-implement.yml")
    check(impl.get("on", {}).get("issues", {}).get("types") == ["labeled"],
          "agent-implement.yml 必须以 issues labeled 触发")
    check(job(impl, "implement").get("if") == "github.event.label.name == 'agent:implement'",
          "agent-implement.yml 必须闸住 agent:implement label")
    cc = job(impl, "implement").get("concurrency", {})
    check("github.event.issue.number" in cc.get("group", "") and cc.get("cancel-in-progress") is False,
          "agent-implement.yml concurrency 必须按 issue 分组且 cancel-in-progress=false")
    perms = job(impl, "implement").get("permissions", {})
    check(perms == {"contents": "write", "issues": "write", "pull-requests": "write"},
          f"agent-implement.yml 权限必须最小化（实为 {perms}）")
    check("detect-issue-shape.sh" in impl_raw, "agent-implement.yml 必须调用 detect-issue-shape.sh")
    check(impl_raw.index("name: Checkout main") < impl_raw.index("name: Detect issue shape"),
          "agent-implement.yml 必须先 checkout 再跑 shape 检测（脚本在本仓）")
    check("shape == 'map'" in impl_raw, "agent-implement.yml 必须拒绝 map/PRD（有子票）")
    check("shape == 'blocked'" in impl_raw and '--add-label "agent:blocked"' in impl_raw,
          "agent-implement.yml 必须在 open blocker>0 时拒绝并标 blocked")
    check("Refuse sub-issue" not in impl_raw,
          "agent-implement.yml 不得再拒绝 leaf sub-issue（#1110 AC-4：leaf 允许）")
    check("Preflight existing PR" in impl_raw, "agent-implement.yml 必须有 existing PR 防重 preflight")
    check("ref: main" in impl_raw and "fetch-depth: 0" in impl_raw,
          "agent-implement.yml checkout 必须以 main 为 base")
    check("agent/issue-${ISSUE_NUMBER}-${slug}" in impl_raw,
          "agent-implement.yml 分支名必须是确定性的 agent/issue-<n>-<slug>")
    check("--draft --base main" in impl_raw and "Closes #${ISSUE_NUMBER}" in impl_raw,
          "agent-implement.yml 必须开 Draft PR 且 body 含 Closes #<n>")
    check('git push --force origin "$BRANCH"' in impl_raw,
          "agent-implement.yml 实装必须 push 确定性新分支（--force 幂等，非 force-with-lease）")
    check("Mark blocked on failure" in impl_raw and "failure_reason.txt" in impl_raw,
          "agent-implement.yml 必须有失败回写（Mark blocked on failure）")
    check("npm run typecheck" in impl_raw and "npm run build" not in impl_raw,
          "agent-implement.yml 必须 typecheck 而非 build（本仓 sandcastle 是已装依赖）")

    review = load_wf("agent-review.yml")
    review_raw = raw_wf("agent-review.yml")
    check(review.get("on", {}).get("pull_request_target", {}).get("types") == ["labeled"],
          "agent-review.yml 必须以 pull_request_target labeled 触发")
    check(job(review, "review").get("if") == "github.event.label.name == 'agent:review'",
          "agent-review.yml 必须闸住 agent:review label")
    check("github.event.pull_request.number" in job(review, "review").get("concurrency", {}).get("group", ""),
          "agent-review.yml concurrency 必须按 PR 分组")
    check(job(review, "review").get("permissions", {}) == {"contents": "write", "pull-requests": "write"},
          "agent-review.yml 权限必须最小化（contents/pull-requests write）")
    check("--force-with-lease=" in review_raw and "BRANCH_HEAD_SHA" in review_raw,
          "agent-review.yml 评审 push 必须 force-with-lease 且以 checkout 时 head.sha 为 expected old")
    check('gh api --method POST "repos/{owner}/{repo}/pulls/${PR_NUMBER}/reviews"' in review_raw,
          "agent-review.yml 通过后必须提交 GitHub review")
    check("gh pr ready" in review_raw, "agent-review.yml 通过后必须标 Ready")

    # implement-PR 修正入口（AC-3 至少包含 implement-PR 或明确不纳入理由）
    impl_pr = load_wf("agent-implement-pr.yml")
    impl_pr_raw = raw_wf("agent-implement-pr.yml")
    check(impl_pr.get("on", {}).get("pull_request_target", {}).get("types") == ["labeled"]
          and job(impl_pr, "implement-pr").get("if") == "github.event.label.name == 'agent:implement'",
          "agent-implement-pr.yml 必须提供 implement-PR 修正入口")
    check("--force-with-lease=" in impl_pr_raw and "BRANCH_HEAD_SHA" in impl_pr_raw,
          "agent-implement-pr.yml push 必须 force-with-lease")

    explore = load_wf("agent-explore.yml")
    check(explore.get("on", {}).get("issues", {}).get("types") == ["labeled"],
          "agent-explore.yml 必须以 issues labeled 触发")
    check(job(explore, "explore").get("permissions", {}) == {"issues": "write"},
          "agent-explore.yml 权限必须最小化（只读探索，仅 issues: write 回写评论）")

    update = load_wf("agent-update-branch.yml")
    update_raw = raw_wf("agent-update-branch.yml")
    check(update.get("on", {}).get("pull_request_target", {}).get("types") == ["labeled"],
          "agent-update-branch.yml 必须以 pull_request_target labeled 触发")
    check("--force-with-lease=" in update_raw, "agent-update-branch.yml push 必须 force-with-lease")

    # pull_request_target 风险面：PR 型 workflow 只能由 labeled 事件触发
    # （不可信 PR 内容不会自动跑）；label 是信任闸（需仓库写权限）。
    for name in ["agent-review.yml", "agent-implement-pr.yml", "agent-update-branch.yml"]:
        wf = load_wf(name)
        ptr = wf.get("on", {}).get("pull_request_target", {})
        types = ptr.get("types", [])
        check(types == ["labeled"], f"{name} pull_request_target 必须只响应 labeled（实为 {types}）")

    # ── 2. issue shape 四形态对拍（mock gh）─────────────────────────────────────
    script = AW_DIR / "shared" / "detect-issue-shape.sh"
    with tempfile.TemporaryDirectory() as td:
        mock = pathlib.Path(td) / "gh"
        mock.write_text(
            "#!/usr/bin/env bash\n"
            "# mock gh api 'repos/o/r/issues/<n>' --jq '<expr>'\n"
            "case \"$*\" in\n"
            '  *"/issues/1"*) printf "0\\t0\\t\\n" ;;\n'   # standalone leaf
            '  *"/issues/2"*) printf "5\\t0\\t\\n" ;;\n'   # map（5 子票）
            '  *"/issues/3"*) printf "0\\t2\\t\\n" ;;\n'   # blocked（2 open blockers）
            '  *"/issues/4"*) printf "0\\t0\\thttps://api.github.com/repos/o/r/issues/100\\n" ;;\n'  # leaf sub-issue
            "esac\n"
        )
        mock.chmod(0o755)
        env = dict(os.environ, GH_REPO="o/r", PATH=f"{td}:{os.environ['PATH']}")
        cases = [
            ("1", "leaf", ""),
            ("2", "map", ""),
            ("3", "blocked", ""),
            ("4", "leaf", "100"),
        ]
        for num, want_shape, want_parent in cases:
            r = subprocess.run([str(script)], env=dict(env, ISSUE_NUMBER=num),
                               capture_output=True, text=True)
            out = r.stdout
            got_shape = next((l.split("=", 1)[1] for l in out.splitlines() if l.startswith("shape=")), None)
            got_parent = next((l.split("=", 1)[1] for l in out.splitlines() if l.startswith("parent_number=")), "")
            check(r.returncode == 0 and got_shape == want_shape and got_parent == want_parent,
                  f"shape 对拍 issue={num}: 期望 shape={want_shape} parent={want_parent!r}，"
                  f"实得 shape={got_shape} parent={got_parent!r}（rc={r.returncode}）")

    # ── 3. 禁入模式扫描 ────────────────────────────────────────────────────────
    all_text = "\n".join((WF_DIR / n).read_text() for n in WORKFLOWS)
    all_text += "\n".join(p.read_text() for p in AW_DIR.rglob("*") if p.is_file())
    banned = [
        ("PRIME_API_KEY", "不得绑定 PRIME_API_KEY（认证路线未裁定）"),
        ("prime-inference", "不得绑定 prime-inference"),
        ("~/.prime", "不得读取本机 ~/.prime"),
        ("~/.codex", "不得读取本机 ~/.codex"),
        ("sandcastle.codex(", "不得绑定内置 codex() 作为 agent provider"),
        ("sandcastle.pi(", "不得绑定内置 pi() 作为 agent provider"),
        ("watcher.sh", "不得复用现有 watcher.sh 运行时"),
        ("harvest.sh", "不得复用现有 harvest.sh 运行时"),
        ("main.mts", "不得复用现有 main.mts 主循环"),
        ("main-swarm.mts", "不得复用现有 main-swarm.mts 蜂群"),
    ]
    for needle, why in banned:
        check(needle not in all_text, f"禁入模式扫描命中 {needle!r}：{why}")
    check("claudeCode" in all_text, "agent provider 必须使用 Sandcastle 内置 claudeCode()")

    # ── 汇总 ───────────────────────────────────────────────────────────────────
    if _failures:
        print(f"verify-workflows: {len(_failures)} 处失败")
        for i, f in enumerate(_failures, 1):
            print(f"  [{i}] {f}")
        return 1
    print(f"verify-workflows: 全过（{len(WORKFLOWS)} 个 workflow 结构 + 4 形态对拍 + 禁入扫描）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
