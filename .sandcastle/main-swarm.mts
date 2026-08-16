// Parallel Planner with Review（prime-agent 版）—— #1007 裁定落地。
// 官方 parallel-planner-with-review 形态 × 本仓裁定：
//   Phase 1 planner 建依赖图（<plan> 结构化输出）→
//   Phase 2 Promise.allSettled 扇出（MAX_PARALLEL=4 信号量，#1007 裁 2），
//     每 issue：createSandbox 确定性分支（baseBranch=main）→ 实装→独立评审 →
//   Phase 3 单 merger 收口——技术合并进 sandcastle/merge-<ts> 归并分支，
//     **不合 main、不关 issue**（人工闸，#1003 裁 3 有意偏离）。
// 跑法：npx tsx .sandcastle/main-swarm.mts

import * as sandcastle from "@ai-hero/sandcastle";
import { docker } from "@ai-hero/sandcastle/sandboxes/docker";
import { primeAgent } from "./prime-agent-provider.ts";
import { execSync } from "node:child_process";
import { z } from "zod";

// ── 顶部常量（#1002 裁 3） ────────────────────────────────────────────────
const PLANNER_MODEL = "k3";
const IMPLEMENTER_MODEL = "k3";
const REVIEWER_MODEL = "k3";
const MERGER_MODEL = "k3";
const PROVIDER = "kimi-coding";
const MAX_ITERATIONS = 3;
const MAX_PARALLEL = 4; // #1007 裁 2（官方上游自用值；资源不足按实测降额登记）
const IMAGE = "sandcastle:newchanlun";

const GH_CLEAN_ENV = {
  ...process.env,
  NO_COLOR: "1", CLICOLOR: "0", FORCE_COLOR: "0", CLICOLOR_FORCE: "0",
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

// 认领（#1007 裁 3 + #1013 教训：摘专属 label + assign，host 侧先行，避免工蜂侧竞态）
function claim(issue: string) {
  execSync(`gh issue edit ${issue} --remove-label sandcastle --add-assignee @me`,
    { env: GH_CLEAN_ENV, stdio: "ignore" });
}

for (let iteration = 1; iteration <= MAX_ITERATIONS; iteration++) {
  console.log(`\n=== Iteration ${iteration}/${MAX_ITERATIONS} ===\n`);

  // Phase 1: Plan
  // 用 createSandbox 而非顶层 run()：顶层 run（merge-to-head）在本仓 clone/多远端
  // 环境下实测 agent 进程 ~11s 被 SIGKILL（exit 137，在案未决怪癖，见 #1009）；
  // createSandbox 路径两段式全程验证过。planner 无产出，分支随关随弃。
  const planSandbox = await sandcastle.createSandbox({
    branch: `sandcastle/plan-${Date.now()}`,
    baseBranch: "main",
    sandbox: docker({ imageName: IMAGE }),
  });
  let issues: Array<{ id: string; title: string; branch: string }>;
  try {
    // 注：sandbox.run 不支持 Output.object（顶层 run 才支持），手动从 stdout 解析 <plan>。
    const plan = await planSandbox.run({
      name: "planner",
      maxIterations: 1,
      agent: primeAgent(PLANNER_MODEL, { provider: PROVIDER }),
      promptFile: "./.sandcastle/plan-prompt.md",
    });
    const m = plan.stdout.match(/<plan>([\s\S]*?)<\/plan>/);
    if (!m) throw new Error("planner 未输出 <plan> 标签");
    issues = planSchema.parse(JSON.parse(m[1])).issues;
  } finally {
    await planSandbox.close();
  }
  if (issues.length === 0) {
    console.log("无可并行 issue，停。");
    break;
  }
  console.log(`planner 放行 ${issues.length} 张：`);
  for (const i of issues) console.log(`  #${i.id}: ${i.title} → ${i.branch}`);

  // host 侧认领（扇出前一次性完成）
  for (const i of issues) claim(i.id);

  // Phase 2: Execute + Review（信号量限流扇出）
  const withLimit = semaphore(MAX_PARALLEL);
  const settled = await Promise.allSettled(
    issues.map((issue) =>
      withLimit(async () => {
        const sandbox = await sandcastle.createSandbox({
          branch: issue.branch,
          baseBranch: "main", // 与检出分支解耦（#1003）
          sandbox: docker({ imageName: IMAGE }),
        });
        try {
          const implement = await sandbox.run({
            name: "implementer",
            maxIterations: 1,
            agent: primeAgent(IMPLEMENTER_MODEL, { provider: PROVIDER }),
            promptFile: "./.sandcastle/implement-prompt.md",
            promptArgs: { ISSUE_NUMBER: issue.id },
          });
          if (!implement.commits.length) return implement;
          const review = await sandbox.run({
            name: "reviewer",
            maxIterations: 1,
            agent: primeAgent(REVIEWER_MODEL, { provider: PROVIDER }),
            promptFile: "./.sandcastle/review-prompt.md",
            promptArgs: { BRANCH: issue.branch, ISSUE_NUMBER: issue.id },
          });
          return { ...review, commits: [...implement.commits, ...review.commits] };
        } finally {
          await sandbox.close();
        }
      }),
    ),
  );

  for (const [i, outcome] of settled.entries()) {
    if (outcome.status === "rejected") {
      console.error(`  ✗ #${issues[i]!.id} (${issues[i]!.branch}) 失败: ${outcome.reason}`);
    }
  }

  const completed = settled
    .map((outcome, i) => ({ outcome, issue: issues[i]! }))
    .filter((e) => e.outcome.status === "fulfilled" && (e.outcome as PromiseFulfilledResult<{ commits: unknown[] }>).value.commits.length > 0)
    .map((e) => e.issue);

  console.log(`\n执行完成，${completed.length} 条分支有产出。`);
  if (completed.length === 0) continue;

  // Phase 3: 单 merger 收口（技术合并进归并分支；不合 main、不关 issue——人工闸）
  const mergeBranch = `sandcastle/merge-${Date.now()}`;
  const mergeSandbox = await sandcastle.createSandbox({
    branch: mergeBranch,
    baseBranch: "main",
    sandbox: docker({ imageName: IMAGE }),
  });
  try {
    await mergeSandbox.run({
      name: "merger",
      maxIterations: 1,
      agent: primeAgent(MERGER_MODEL, { provider: PROVIDER }),
      promptFile: "./.sandcastle/merge-prompt.md",
      promptArgs: {
        BRANCHES: completed.map((i) => `- ${i.branch}`).join("\n"),
        ISSUES: completed.map((i) => `- ${i.id}: ${i.title}`).join("\n"),
      },
    });
    console.log(`\n归并完成：${mergeBranch}（待验收合入 main，人工闸）。`);
  } finally {
    await mergeSandbox.close();
  }
}

console.log("\nAll done.");
