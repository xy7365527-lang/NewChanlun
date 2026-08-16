// Sequential Reviewer（prime-agent 版）—— #1001/#1002/#1003 裁定落地。
// 每票：host 侧 frontier 取票 + claim → createSandbox（确定性分支，baseBranch=main）
//   → Phase 1 实装工蜂 → Phase 2 独立评审工蜂（直接在分支上修正）
//   → 不合 main（人工闸，#1003 裁 3）。
// 跑法：npx tsx .sandcastle/main.mts

import * as sandcastle from "@ai-hero/sandcastle";
import { docker } from "@ai-hero/sandcastle/sandboxes/docker";
import { primeAgent } from "./prime-agent-provider.ts";
import { execSync } from "node:child_process";

// ── 顶部常量（#1002 裁 3：模型面集中在此，升档改这里重跑） ────────────────
const IMPLEMENTER_MODEL = "k3";
const REVIEWER_MODEL = "k3";
const PROVIDER = "kimi-coding";
const MAX_ITERATIONS = 1;

// ── frontier 查询（host 侧）：open + sandcastle label + 未 assign + 无 open blocker ──
// blocker 过滤吃 tracker 原生依赖边（#1005 补齐；与 prompt 层判定叠加，#1007 裁 3）。
const GH_CLEAN_ENV = {
  ...process.env,
  // gh 在 FORCE_COLOR 环境下会给 --json 输出染色，必须洗掉
  NO_COLOR: "1", CLICOLOR: "0", FORCE_COLOR: "0", CLICOLOR_FORCE: "0",
};

function hasOpenBlocker(issue: number): boolean {
  const out = execSync(
    `gh api repos/{owner}/{repo}/issues/${issue} --jq .issue_dependencies_summary.blocked_by`,
    { encoding: "utf8", env: GH_CLEAN_ENV },
  ).trim();
  return Number(out) > 0;
}

function pickIssue(): number | null {
  const out = execSync(
    `gh issue list --label sandcastle --state open --limit 20 --json number,assignees`,
    { encoding: "utf8", env: GH_CLEAN_ENV },
  );
  const open = (JSON.parse(out) as Array<{ number: number; assignees: unknown[] }>)
    .filter((i) => i.assignees.length === 0);
  for (const i of open) {
    if (!hasOpenBlocker(i.number)) return i.number;
  }
  return null;
}

for (let iter = 1; iter <= MAX_ITERATIONS; iter++) {
  const issue = pickIssue();
  if (!issue) {
    console.log("无 ready-for-agent 未认领 issue，停。");
    break;
  }

  // 认领（#1007 裁 3 + #1013 抢票教训）：摘 sandcastle + assign @me；拾取闸用专属 label，不碰 ready-for-agent 共享面
  execSync(`gh issue edit ${issue} --remove-label sandcastle --add-assignee @me`);
  const branch = `sandcastle/issue-${issue}`;
  console.log(`\n=== Iteration ${iter}/${MAX_ITERATIONS}: issue #${issue} → ${branch} ===\n`);

  const sandbox = await sandcastle.createSandbox({
    branch,
    baseBranch: "main", // 与检出分支解耦（#1003：并行会话共存）
    sandbox: docker({ imageName: "sandcastle:newchanlun" }),
  });
  try {
    // Phase 1：实装
    const implement = await sandbox.run({
      name: "implementer",
      maxIterations: 1,
      agent: primeAgent(IMPLEMENTER_MODEL, { provider: PROVIDER }),
      promptFile: "./.sandcastle/implement-prompt.md",
      promptArgs: { ISSUE_NUMBER: String(issue) },
    });
    if (!implement.commits.length) {
      console.log("实装无 commit（被卡或无活），跳评审，留票待查。");
      continue;
    }
    console.log(`实装完成：${implement.commits.length} 个 commit`);

    // Phase 2：独立评审（不同子代理，#1001 裁 1；直接在分支上修正）
    await sandbox.run({
      name: "reviewer",
      maxIterations: 1,
      agent: primeAgent(REVIEWER_MODEL, { provider: PROVIDER }),
      promptFile: "./.sandcastle/review-prompt.md",
      promptArgs: { BRANCH: branch, ISSUE_NUMBER: String(issue) },
    });
    console.log(`两段完成：${branch} 待验收合入（人工闸）。`);
  } finally {
    await sandbox.close();
  }
}
