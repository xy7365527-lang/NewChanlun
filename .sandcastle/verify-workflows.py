#!/usr/bin/env python3
"""机械验证 .sandcastle 官方 GitHub Actions scaffold（#1110）与 #1128 显式模型 registry。

验证面（对应 #1110 + #1128 验收）：
  1. workflow YAML 结构不变式（trigger/label 闸、concurrency、权限最小化、
     checkout/base、确定性分支、Draft PR、force-with-lease、失败回写、防重）。
  2. issue shape 检测脚本（detect-issue-shape.sh）在 mock gh 下四形态对拍。
  3. 禁入模式扫描（不读本机 Prime/Codex auth；不复用 watcher/harvest/main-loop；
     #1002 旧 Kimi 常量/票面模型路由不复活）。
  4. #1128 模型 registry 机械锁：event payload 接线、固定 Secret allowlist、
     票面字符串不成为 env/secret key、remote-child identity 与模型凭据分离、
     事件 fixture 覆盖（默认 Claude / 显式 DeepSeek / 冲突 / 未知 / 缺 secret）。

用法：python3 .sandcastle/verify-workflows.py
退出码 0 = 全过；非 0 = 首条失败（附可读信息）。
"""
from __future__ import annotations

import json
import os
import pathlib
import re
import subprocess
import sys
import tempfile

import yaml

ROOT = pathlib.Path(__file__).resolve().parent.parent
WF_DIR = ROOT / ".github" / "workflows"
AW_DIR = ROOT / ".sandcastle" / "agent-workflows"
FIXTURE_DIR = AW_DIR / "shared" / "fixtures"

WORKFLOWS = [
    "agent-implement.yml",
    "agent-review.yml",
    "agent-implement-pr.yml",
    "agent-explore.yml",
    "agent-update-branch.yml",
]

# #1128 事件 fixture 覆盖表：文件名 → 必须出现的标签集合（含触发标签与模型标签）。
FIXTURE_REQUIREMENTS = {
    "issue-labeled-default-claude.json": {"agent:implement"},
    "issue-labeled-deepseek-v4-pro.json": {
        "agent:implement",
        "agent:model:deepseek-v4-pro",
    },
    "issue-labeled-conflict.json": {
        "agent:implement",
        "agent:model:deepseek-v4-pro",
        "agent:model:DEEPSEEK_API_KEY",
    },
    "issue-labeled-unknown.json": {
        "agent:implement",
        "agent:model:DEEPSEEK_API_KEY",
    },
    "issue-labeled-deepseek-missing-secret.json": {
        "agent:implement",
        "agent:model:deepseek-v4-pro",
    },
    "issue-labeled-deepseek-retry.json": {
        "agent:implement",
        "agent:model:deepseek-v4-pro",
    },
    "pr-labeled-review.json": {
        "agent:review",
        "agent:model:deepseek-v4-pro",
    },
    "pr-labeled-implement-pr-deepseek-v4-pro.json": {
        "agent:implement",
        "agent:model:deepseek-v4-pro",
    },
}

ALLOWED_SECRET_REFS = {
    "GITHUB_TOKEN",
    "AGENT_PAT",
    "CLAUDE_CODE_OAUTH_TOKEN",
    "DEEPSEEK_API_KEY",
}

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
        ("kimi-coding", "#1002 旧 Kimi provider 常量不得复活"),
        ("IMPLEMENTER_MODEL", "#1002 旧票面/常量模型路由不得复活"),
        ("REVIEWER_MODEL", "#1002 旧票面/常量模型路由不得复活"),
    ]
    for needle, why in banned:
        check(needle not in all_text, f"禁入模式扫描命中 {needle!r}：{why}")
    check("claudeCode" in all_text, "agent provider 必须使用 Sandcastle 内置 claudeCode()")

    # ── 4. #1128 显式模型 registry 机械锁 ────────────────────────────────────
    agent_ts = (AW_DIR / "shared" / "agent.ts").read_text()
    registry_all_text = all_text

    for name in WORKFLOWS:
        raw = raw_wf(name)
        check(
            "EVENT_PAYLOAD: ${{ toJSON(github.event) }}" in raw,
            f"{name} 必须把 toJSON(github.event) 作为 EVENT_PAYLOAD 传给 agent 步",
        )

    # 固定 Secret allowlist：workflow 里出现的每个 secrets.X 都必须在白名单内。
    secret_refs = set(re.findall(r"secrets\.([A-Z0-9_]+)", "\n".join(raw_wf(n) for n in WORKFLOWS)))
    check(
        secret_refs <= ALLOWED_SECRET_REFS,
        f"workflow Secret 引用必须封闭 allowlist（实为 {sorted(secret_refs)}）",
    )

    # 只实装入口（issue implement / PR implement-pr）注入 DeepSeek secret；
    # review/explore/update-branch 固定 Claude，绝不把 DeepSeek key 放进进程 env。
    for name in ["agent-implement.yml", "agent-implement-pr.yml"]:
        raw = raw_wf(name)
        check(
            "DEEPSEEK_API_KEY" in raw and "agent:model:deepseek-v4-pro" in raw,
            f"{name} 必须支持显式 DeepSeek v4-pro 路线",
        )
        check(
            "prime-agent-0.7.2.tgz" in raw,
            f"{name} 必须安装与 .sandcastle/Dockerfile 同版本的 Prime Agent CLI",
        )
    for name in ["agent-review.yml", "agent-explore.yml", "agent-update-branch.yml"]:
        raw = raw_wf(name)
        check(
            "DEEPSEEK_API_KEY" not in raw,
            f"{name} 固定 Claude 角色，不得注入 DEEPSEEK_API_KEY",
        )

    check("MODEL_REGISTRY" in agent_ts and "agent:model:deepseek-v4-pro" in agent_ts,
          "shared/agent.ts 必须包含显式模型 registry allowlist")
    check("primeAgent" in agent_ts and "claudeCode" in agent_ts,
          "shared/agent.ts 必须同时具备 Claude 内置 provider 与自定义 Prime Agent provider")
    check("REMOTE_CHILD_IDENTITY_ENV_ALLOWLIST" in agent_ts,
          "shared/agent.ts 必须把 remote-child invitation/lease 与模型凭据分开")
    check("MODEL_SECRET_NAMES" in agent_ts,
          "shared/agent.ts 必须集中声明模型 Secret 名 allowlist")
    check("process.env[" not in agent_ts,
          "shared/agent.ts 模型/Secret 映射必须是代码内 allowlist，不得动态索引 env key")
    if "REMOTE_CHILD_IDENTITY_ENV_ALLOWLIST = [" in agent_ts:
        remote_identity_block = agent_ts.split(
            "REMOTE_CHILD_IDENTITY_ENV_ALLOWLIST = [", 1
        )[1].split("] as const;", 1)[0]
        check(
            "DEEPSEEK_API_KEY" not in remote_identity_block
            and "CLAUDE_CODE_OAUTH_TOKEN" not in remote_identity_block,
            "remote-child invitation/lease allowlist 不得包含模型凭据名",
        )

    # 事件 fixture：存在、合法 JSON、覆盖四种必需场景（默认 Claude / DeepSeek / 冲突+未知 / 缺 secret）。
    for filename, required_labels in FIXTURE_REQUIREMENTS.items():
        path = FIXTURE_DIR / filename
        check(path.is_file(), f"缺少 #1128 事件 fixture {filename}")
        if not path.is_file():
            continue
        try:
            event = json.loads(path.read_text())
        except json.JSONDecodeError as exc:
            check(False, f"fixture {filename} 不是合法 JSON：{exc}")
            continue
        check(event.get("action") == "labeled", f"fixture {filename} action 必须为 labeled")
        container = event.get("issue") if "issue" in event else event.get("pull_request")
        labels = (container or {}).get("labels") if isinstance(container, dict) else None
        label_names: set[str] = set()
        if isinstance(labels, list):
            label_names = {
                item.get("name")
                for item in labels
                if isinstance(item, dict) and isinstance(item.get("name"), str)
            }
        check(
            isinstance(labels, list) and label_names >= required_labels,
            f"fixture {filename} labels 必须覆盖 {sorted(required_labels)}（实为 {sorted(label_names)}）",
        )

    required_fixtures = {
        "issue-labeled-default-claude.json",
        "issue-labeled-deepseek-v4-pro.json",
        "issue-labeled-conflict.json",
        "issue-labeled-unknown.json",
        "issue-labeled-deepseek-missing-secret.json",
    }
    check(required_fixtures <= set(FIXTURE_REQUIREMENTS),
          "必需事件 fixture 集合缺失（默认 Claude/DeepSeek/冲突/未知/缺 secret）")

    # 票面字符串不成为 Secret/env key：除了 registry allowlist 与测试反例，
    # 工作流和 agent 代码里不得出现 `secrets.<任意标签>` 或 `process.env[<动态>]`。
    check("secrets.agent:model" not in registry_all_text,
          "票面标签字符串不得拼进 secrets 引用")
    check(
        'env["agent:model' not in registry_all_text
        and "env[`agent:model" not in registry_all_text,
        "票面标签字符串不得拼进 env key",
    )

    # ── 汇总 ───────────────────────────────────────────────────────────────────
    if _failures:
        print(f"verify-workflows: {len(_failures)} 处失败")
        for i, f in enumerate(_failures, 1):
            print(f"  [{i}] {f}")
        return 1
    print(
        "verify-workflows: 全过（"
        f"{len(WORKFLOWS)} 个 workflow 结构 + 4 形态对拍 + 禁入扫描 + "
        f"{len(FIXTURE_REQUIREMENTS)} 个 #1128 事件 fixture 覆盖 + registry 机械锁）"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
