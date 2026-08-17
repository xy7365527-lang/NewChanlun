// Sequential Reviewer（prime-agent 版）—— #1001/#1002/#1003 裁定落地。
// 每票：host 侧 frontier 取票 + claim → createSandbox（确定性分支，baseBranch=main）
//   → Phase 1 实装工蜂 → Phase 2 独立评审工蜂（直接在分支上修正）
//   → 不合 main（人工闸，#1003 裁 3）。
// 跑法：npx tsx .sandcastle/main.mts

import * as sandcastle from "@ai-hero/sandcastle";
import { docker } from "@ai-hero/sandcastle/sandboxes/docker";
import { primeAgent } from "./prime-agent-provider.ts";
import { execSync } from "node:child_process";
import { appendFileSync, mkdirSync } from "node:fs";

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
  GH_REPO: "xy7365527-lang/NewChanlun", // origin 可能是本地路径（clone 场景），gh 靠它认仓
};

function hasOpenBlocker(issue: number): boolean {
  const out = execSync(
    `gh api repos/{owner}/{repo}/issues/${issue} --jq .issue_dependencies_summary.blocked_by`,
    { encoding: "utf8", env: GH_CLEAN_ENV },
  ).trim();
  return Number(out) > 0;
}

// ── 工蜂登记面（2026-08-17：宿主 harness 不可见问题的补偿——跨进程无原生注册通道，
//    以 JSONL registry 供宿主/roster 读取；每事件带 ts/ticket/branch/phase/status）──
function logWorker(e: Record<string, unknown>) {
<<<<<<< HEAD
  // logs/ 目录可能不存在（gitignore 后新克隆）——mkdir 兜底（dispatcher 轮 2 坐实的 ENOENT）
  try { execSync("mkdir -p .sandcastle/logs"); } catch {}
=======
  // logs 目录不入仓（运行时产物），首次写前确保存在——否则 appendFileSync ENOENT 杀主循环。
  mkdirSync(".sandcastle/logs", { recursive: true });
>>>>>>> sandcastle/issue-879
  appendFileSync(
    ".sandcastle/logs/workers.jsonl",
    JSON.stringify({ ts: new Date().toISOString(), ...e }) + "\n",
  );
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

// ── REVIEW_ONLY 模式（#879 补派 Phase 2 引入）：env SANDCASTLE_REVIEW_ONLY="分支名:票号" 时
// 跳过 pickIssue 与 Phase 1，对既有分支直接跑 Phase 2 评审（宿主驱动中断后的补派路径，
// TROUBLESHOOTING #11）。分支须已存在且含实装 commit。
const REVIEW_ONLY = process.env.SANDCASTLE_REVIEW_ONLY;
if (REVIEW_ONLY) {
  const [roBranch, roIssue] = REVIEW_ONLY.split(":");
  if (!roBranch || !roIssue) {
    console.error("SANDCASTLE_REVIEW_ONLY 格式须为 分支名:票号");
    process.exit(2);
  }
  const issue = Number(roIssue);
  const branch = roBranch;
  const sandbox = await sandcastle.createSandbox({
    branch,
    baseBranch: "main",
    sandbox: docker({ imageName: "sandcastle:newchanlun" }),
  });
  logWorker({ ticket: issue, branch, phase: "reviewer", status: "started" });
  await sandbox.run({
    name: "reviewer",
    maxIterations: 1,
    agent: primeAgent(REVIEWER_MODEL, { provider: PROVIDER }),
    promptFile: "./.sandcastle/review-prompt.md",
    promptArgs: { BRANCH: branch, ISSUE_NUMBER: String(issue) },
  });
  logWorker({ ticket: issue, branch, phase: "reviewer", status: "completed" });
  console.log(`REVIEW_ONLY 完成：${branch} 待验收合入（人工闸）。`);
  await sandbox.close();
  process.exit(0);
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
  logWorker({ ticket: issue, branch, phase: "claim", status: "claimed" });
  console.log(`\n=== Iteration ${iter}/${MAX_ITERATIONS}: issue #${issue} → ${branch} ===\n`);

  const sandbox = await sandcastle.createSandbox({
    branch,
    baseBranch: "main", // 与检出分支解耦（#1003：并行会话共存）
    sandbox: docker({ imageName: "sandcastle:newchanlun" }),
  });
  try {
    // Phase 1：实装
    logWorker({ ticket: issue, branch, phase: "implementer", status: "started" });
    const implement = await sandbox.run({
      name: "implementer",
      maxIterations: 1,
      agent: primeAgent(IMPLEMENTER_MODEL, { provider: PROVIDER }),
      promptFile: "./.sandcastle/implement-prompt.md",
      promptArgs: { ISSUE_NUMBER: String(issue) },
    });
    if (!implement.commits.length) {
      logWorker({ ticket: issue, branch, phase: "implementer", status: "no_commit" });
      console.log("实装无 commit（被卡或无活），跳评审，留票待查。");
      continue;
    }
    logWorker({ ticket: issue, branch, phase: "implementer", status: "completed", commits: implement.commits.length });
    console.log(`实装完成：${implement.commits.length} 个 commit`);

    // Phase 2：独立评审（不同子代理，#1001 裁 1；直接在分支上修正）
    logWorker({ ticket: issue, branch, phase: "reviewer", status: "started" });
    await sandbox.run({
      name: "reviewer",
      maxIterations: 1,
      agent: primeAgent(REVIEWER_MODEL, { provider: PROVIDER }),
      promptFile: "./.sandcastle/review-prompt.md",
      promptArgs: { BRANCH: branch, ISSUE_NUMBER: String(issue) },
    });
    logWorker({ ticket: issue, branch, phase: "reviewer", status: "completed" });
    console.log(`两段完成：${branch} 待验收合入（人工闸）。`);
  } finally {
    logWorker({ ticket: issue, branch, phase: "sandbox", status: "closed" });
    await sandbox.close();
  }
}
