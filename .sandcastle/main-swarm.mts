// Parallel Planner with Review（主控同型 Codex 版）—— #1007 裁定落地。
// 官方 parallel-planner-with-review 形态 × 本仓裁定：
//   Phase 1 planner 建依赖图（<plan> 结构化输出）→
//   Phase 2 Promise.allSettled 扇出（MAX_PARALLEL=4 信号量，#1007 裁 2），
//     每 issue：createSandbox 确定性分支（baseBranch=main）→ 实装→独立评审 →
//   Phase 3 候选交主控验收；accepted 清单由独立入口受管归并到 sandcastle/merge-<run-id>，
//     **不合 main、不关 issue**（人工闸，#1003 裁 3 有意偏离）。
// 跑法：npx tsx .sandcastle/main-swarm.mts

import { drainRequested, withLegacyAdmission, DrainFrozenError } from "./drain-control.mts";
import * as sandcastle from "@ai-hero/sandcastle";
import { assertDockerCliProxyReachable, assertSwarmExecutionReady, swarmAgent, swarmSandbox } from "./swarm-route.mts";
import { assertControlledBase, controlledSwarmTicket, swarmBinding } from "./swarm-controlled.mts";
import { execSync } from "node:child_process";
import { appendFileSync, mkdirSync, writeFileSync } from "node:fs";
import { z } from "zod";
import { resolve } from "node:path";
import { runAcceptedSwarmMerge } from "./swarm-merge.mts";
import { runManagedSwarmTicket } from "./swarm-managed.mts";
import {
  ghIssueFetcher,
  ghIssueNotifier,
  hostGit,
  readReviewTemplate,
  resolveReviewBase,
  ReviewPromptTooLargeError,
} from "./review-context.mts";
// 单票作者/评审只编辑文件；宿主封包后由宿主 nc-remote 验精确 SHA。
// 多票归并只消费主控已验收固定 SHA 清单；默认运行仅产候选，不自动替代单票验收。

// ── 顶部常量（并发与资源面；模型唯一来源是 main-model.mts） ───────────────
const MAX_ITERATIONS = 3;
const MAX_PARALLEL = 4; // #1007 裁 2（官方上游自用值；资源不足按实测降额登记）
const IMAGE = "sandcastle:newchanlun";

const GH_CLEAN_ENV = {
  ...process.env,
  NO_COLOR: "1", CLICOLOR: "0", FORCE_COLOR: "0", CLICOLOR_FORCE: "0",
  GH_REPO: "xy7365527-lang/NewChanlun", // origin 可能是本地路径（clone 场景），gh 靠它认仓
};

const planSchema = z.object({
  issues: z.array(z.object({ id: z.string(), title: z.string(), branch: z.string() })),
});

// 极简信号量
function semaphore(limit: number) {
  let active = 0;
  const queue: Array<() => void> = [];
  const acquire = () => new Promise<void>((res) => {
    if (active < limit) { active++; res(); } else queue.push(res);
  });
  const release = () => { active--; queue.shift()?.(); if (active < 0) active = 0; };
  return async <T,>(fn: () => Promise<T>): Promise<T> => {
    await acquire();
    try { return await fn(); } finally { release(); }
  };
}

// 工蜂登记面（与 main.mts logWorker 同一 JSONL，watcher.sh 滚报 STATUS.md）
function logWorker(e: Record<string, unknown>) {
  mkdirSync(".sandcastle/logs", { recursive: true });
  appendFileSync(".sandcastle/logs/workers.jsonl", JSON.stringify({ ts: new Date().toISOString(), ...e }) + "\n");
}

// 认领（#1007 裁 3 + #1013 教训：摘专属 label + assign，host 侧先行，避免工蜂侧竞态）
function claim(issue: string) {
  execSync(`gh issue edit ${issue} --remove-label sandcastle --add-assignee @me`,
    { env: GH_CLEAN_ENV, stdio: "ignore" });
}

const acceptedManifest = process.env.SANDCASTLE_SWARM_MERGE_MANIFEST;
if (acceptedManifest) {
  try {
    await runAcceptedSwarmMerge(acceptedManifest, IMAGE, logWorker);
  } catch (error) {
    if (!(error instanceof DrainFrozenError)) throw error;
    console.error(error.message);
    process.exitCode = 75;
  }
} else {
// 普通 swarm 先验容器路由，再进入 planner/claim（#1617）。
// 受控验收（SANDCASTLE_DOCKER_ACCEPTANCE_ROW=13）只处理 receipt 绑定的专用验收票：不读线上 sandcastle 队列、不认领任何票。
const controlledTicket = controlledSwarmTicket();
// 受控第 13 行以 receipt.head 为工蜂基线（不是会漂移的本地 main），核对发生在占用验收行、认领和启动工蜂之前。
let baseRef = "main";
assertSwarmExecutionReady(IMAGE, {
  rows: ["13"], binding: controlledTicket ? swarmBinding(controlledTicket) : undefined,
  verify: controlledTicket ? (admission) => { baseRef = assertControlledBase(admission, controlledTicket); } : undefined,
});
assertDockerCliProxyReachable(IMAGE);

/** 普通路径：planner 读线上队列 → 放行 → host 侧认领。返回 null 表示本轮收束。 */
async function planAndClaim(): Promise<Array<{ id: string; title: string; branch: string }> | null> {
  // 用 createSandbox 而非顶层 run()：顶层 run（merge-to-head）在本仓 clone/多远端
  // 环境下实测 agent 进程 ~11s 被 SIGKILL（exit 137，在案未决怪癖，见 #1009）；
  // createSandbox 路径两段式全程验证过。planner 无产出，分支随关随弃。
  const planSandbox = await sandcastle.createSandbox({
    branch: `sandcastle/plan-${Date.now()}`,
    baseBranch: "main",
    sandbox: swarmSandbox(IMAGE),
  });
  let planned: Array<{ id: string; title: string; branch: string }>;
  try {
    // 注：sandbox.run 不支持 Output.object（顶层 run 才支持），手动从 stdout 解析 <plan>。
    const plan = await planSandbox.run({
      name: "planner",
      maxIterations: 1,
      agent: swarmAgent(),
      promptFile: "./.sandcastle/plan-prompt.md",
    });
    const m = plan.stdout.match(/<plan>([\s\S]*?)<\/plan>/);
    if (!m) throw new Error("planner 未输出 <plan> 标签");
    planned = planSchema.parse(JSON.parse(m[1])).issues;
  } finally {
    await planSandbox.close();
  }
  if (planned.length === 0) {
    console.log("无可并行 issue，停。");
    return null;
  }
  console.log(`planner 放行 ${planned.length} 张：`);
  for (const i of planned) console.log(`  #${i.id}: ${i.title} → ${i.branch}`);

  // host 侧认领（扇出前一次性完成）
  try {
    await withLegacyAdmission(() => { for (const i of planned) claim(i.id); });
  } catch (error) {
    if (!(error instanceof DrainFrozenError)) throw error;
    console.log(error.message);
    return null;
  }
  return planned;
}

for (let iteration = 1; iteration <= (controlledTicket ? 1 : MAX_ITERATIONS); iteration++) {
  if (drainRequested()) break;
  console.log(`\n=== Iteration ${iteration}/${controlledTicket ? 1 : MAX_ITERATIONS} ===\n`);

  // Phase 1: Plan（受控验收：输入就是 receipt 绑定的专用验收票，其余票一律不碰；工蜂基线=receipt.head，不写回 main）
  const issues = controlledTicket
    ? [{ id: String(controlledTicket), title: `#1742 Docker 受控验收票 #${controlledTicket}`, branch: `sandcastle/issue-${controlledTicket}` }]
    : await planAndClaim();
  if (!issues) break;

  // Phase 2: Execute + Review（信号量限流扇出）
  const withLimit = semaphore(MAX_PARALLEL);
  const settled = await Promise.allSettled(
    issues.map((issue) =>
      withLimit(async () => {
        const sandbox = await sandcastle.createSandbox({
          branch: issue.branch,
          baseBranch: baseRef, // 普通 swarm=main（与检出分支解耦，#1003）；受控第 13 行=receipt.head
          sandbox: swarmSandbox(IMAGE),
        });
        try {
          const git = hostGit(sandbox.worktreePath);
          // 作者与评审各持有独立 provider；评审修复轮只续自己的 session。
          const implementer = swarmAgent();
          const reviewer = swarmAgent();
          let reviewSession: string | undefined;
          return await runManagedSwarmTicket({
            cwd: sandbox.worktreePath, branch: issue.branch, issue: Number(issue.id),
            implement: async () => (await sandbox.run({
              name: "implementer", maxIterations: 1, agent: implementer,
              promptFile: "./.sandcastle/codex-implement-prompt.md",
              promptArgs: { ISSUE_NUMBER: issue.id },
            })).stdout,
            review: async (prompt) => {
              const result = await sandbox.run({
                name: "reviewer", maxIterations: 1, agent: reviewer, prompt,
                ...(reviewSession ? { resumeSession: reviewSession } : {}),
              });
              reviewSession = result.iterations.at(-1)?.sessionId;
              return result.stdout;
            },
            reviewOptions: {
              template: readReviewTemplate("./.sandcastle/codex-review-prompt.md"),
              target: "main", resolveBase: () => resolveReviewBase(git), git,
              fetchIssue: ghIssueFetcher(GH_CLEAN_ENV),
            },
            channels: { logWorker, notify: ghIssueNotifier(GH_CLEAN_ENV) },
          });
        } finally {
          await sandbox.close();
        }
      }),
    ),
  );

  for (const [i, outcome] of settled.entries()) {
    if (outcome.status === "rejected") {
      console.error(`  ✗ #${issues[i]!.id} (${issues[i]!.branch}) 失败: ${outcome.reason}`);
      if (outcome.reason instanceof ReviewPromptTooLargeError) process.exitCode = outcome.reason.exitCode;
    }
  }

  const completed = settled
    .map((outcome, i) => ({ outcome, issue: issues[i]! }))
    .filter((e) => e.outcome.status === "fulfilled" && (e.outcome as PromiseFulfilledResult<{ commits: unknown[] }>).value.commits.length > 0)
    .map((e) => ({ ...e.issue, result: (e.outcome as PromiseFulfilledResult<Awaited<ReturnType<typeof runManagedSwarmTicket>>>).value }));

  console.log(`\n执行完成，${completed.length} 条分支有产出。`);
  if (completed.length === 0) continue;

  // Phase 3 的输入先交主控验收：不能把 reviewer/completed 自动改成 accepted。
  const runId = String(Date.now());
  const draftPath = `.sandcastle/logs/merge-candidates-${runId}.json`;
  const draft = {
    runId, branch: `sandcastle/merge-${runId}`, base: hostGit()(["rev-parse", "main"]).trim(),
    inputs: completed.map(i => ({ issue: Number(i.id), branch: i.branch, sha: i.result.verification!.head,
      acceptance: { head: i.result.verification!.head, status: "pending",
        reviewEvidence: resolve(".sandcastle/logs/workers.jsonl"),
        verificationEvidence: `${resolve(draftPath)}#head=${i.result.verification!.head}` },
      verification: i.result.verification,
    })),
  };
  mkdirSync(".sandcastle/logs", { recursive: true });
  writeFileSync(draftPath, JSON.stringify(draft, null, 2));
  console.log(`单票候选清单：${draftPath}；主控核验后形成 accepted 清单，再以 SANDCASTLE_SWARM_MERGE_MANIFEST 指向它执行/恢复归并。`);
}
}

console.log("\nAll done.");
