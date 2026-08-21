// Sequential Reviewer（prime-agent 版）—— #1001/#1002/#1003 裁定落地。
// 每票：host 侧 frontier 取票 + claim → createSandbox（确定性分支，baseBranch=main）
//   → Phase 1 实装工蜂 → Phase 2 独立评审工蜂（直接在分支上修正）
//   → 不合 main（人工闸，#1003 裁 3）。
// 跑法：npx tsx .sandcastle/main.mts

import * as sandcastle from "@ai-hero/sandcastle";
import { docker } from "@ai-hero/sandcastle/sandboxes/docker";
import type { AgentProvider, Sandbox, SandboxRunResult } from "@ai-hero/sandcastle";
import { primeAgent } from "./prime-agent-provider.ts";
import { execSync } from "node:child_process";
import { appendFileSync, mkdirSync } from "node:fs";

// ── 顶部常量（#1002 裁 3：模型面集中在此，升档改这里重跑） ────────────────
// 2026-08-18 升档：kimi-coding 配额耗尽（403 billing cycle）→ deepseek（宿主同款，当日验证可用）
const IMPLEMENTER_MODEL = "deepseek-v4-pro";
const REVIEWER_MODEL = "deepseek-v4-pro";
const PROVIDER = "deepseek";
// #1066：每票每阶段（implementer/reviewer）最多续跑轮数；续跑复用原 prime-agent 会话
// （runWithResume），而非 sandcastle 内建 maxIterations 的「多轮各自全新会话」。
const MAX_ITERATIONS = 3;
// 每次运行认领并处理的最大票数（与每阶段续跑轮数无关；本票不扩大取票吞吐，#1066 未涉此闸）。
const MAX_TICKETS_PER_RUN = 1;
// #1066：implementer 数据拉取/长静默任务不被默认 600s idle fail 误杀（#1044 第一次空转形态）。
const IMPLEMENTER_IDLE_TIMEOUT_SECONDS = 1800;

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
  // logs 目录不入仓（运行时产物），首次写前确保存在——否则 appendFileSync ENOENT 杀主循环。
  mkdirSync(".sandcastle/logs", { recursive: true });
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

// ── 多轮续跑（#1066）：同一票的 implementer/reviewer 续跑必须复用原 prime-agent 会话。
// sandcastle 的 maxIterations>1 是「多轮各自全新会话」，不构成续跑（#1066 评论查实）；
// 故每轮 maxIterations=1，靠 provider 会话捕获 + resumeSession 显式续同一 session。
// 终止：出现完成信号（<promise>COMPLETE</promise>）或拿不到可恢复的 session id。
// sandbox.run 抛错（如工蜂被 SIGKILL、exit 137）按原样上抛：沙盒 close 由主循环 finally
// 兜底，票由宿主 re-queue 纪律接管（TROUBLESHOOTING #10/#11）。
async function runWithResume(
  sandbox: Sandbox,
  options: {
    name: string;
    agent: AgentProvider;
    promptFile: string;
    promptArgs: Record<string, string>;
    idleTimeoutSeconds?: number;
  },
): Promise<SandboxRunResult> {
  let resumeSession: string | undefined;
  let result: SandboxRunResult | undefined;
  for (let round = 1; round <= MAX_ITERATIONS; round++) {
    result = await sandbox.run({
      name: options.name,
      maxIterations: 1,
      agent: options.agent,
      promptFile: options.promptFile,
      promptArgs: options.promptArgs,
      ...(options.idleTimeoutSeconds !== undefined ? { idleTimeoutSeconds: options.idleTimeoutSeconds } : {}),
      ...(resumeSession !== undefined ? { resumeSession } : {}),
    });
    const lastId = result.iterations.at(-1)?.sessionId;
    if (lastId) resumeSession = lastId;
    if (result.completionSignal || resumeSession === undefined) break;
  }
  if (!result) throw new Error(`runWithResume: ${options.name} 未产生运行结果`);
  return result;
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
  await runWithResume(sandbox, {
    name: "reviewer",
    agent: primeAgent(REVIEWER_MODEL, { provider: PROVIDER }),
    promptFile: "./.sandcastle/review-prompt.md",
    promptArgs: { BRANCH: branch, ISSUE_NUMBER: String(issue) },
  });
  logWorker({ ticket: issue, branch, phase: "reviewer", status: "completed" });
  console.log(`REVIEW_ONLY 完成：${branch} 待验收合入（人工闸）。`);
  await sandbox.close();
  process.exit(0);
}

for (let iter = 1; iter <= MAX_TICKETS_PER_RUN; iter++) {
  const issue = pickIssue();
  if (!issue) {
    console.log("无 ready-for-agent 未认领 issue，停。");
    break;
  }

  // 认领（#1007 裁 3 + #1013 抢票教训）：摘 sandcastle + assign @me；拾取闸用专属 label，不碰 ready-for-agent 共享面
  execSync(`gh issue edit ${issue} --remove-label sandcastle --add-assignee @me`);
  const branch = `sandcastle/issue-${issue}`;
  logWorker({ ticket: issue, branch, phase: "claim", status: "claimed" });
  console.log(`\n=== Iteration ${iter}/${MAX_TICKETS_PER_RUN}: issue #${issue} → ${branch} ===\n`);

  const sandbox = await sandcastle.createSandbox({
    branch,
    baseBranch: "main", // 与检出分支解耦（#1003：并行会话共存）
    sandbox: docker({ imageName: "sandcastle:newchanlun" }),
  });
  try {
    // Phase 1：实装（#1066：多轮续跑同一会话 + idle 1800s 防长静默误杀）
    logWorker({ ticket: issue, branch, phase: "implementer", status: "started" });
    const implement = await runWithResume(sandbox, {
      name: "implementer",
      agent: primeAgent(IMPLEMENTER_MODEL, { provider: PROVIDER }),
      promptFile: "./.sandcastle/implement-prompt.md",
      promptArgs: { ISSUE_NUMBER: String(issue) },
      idleTimeoutSeconds: IMPLEMENTER_IDLE_TIMEOUT_SECONDS,
    });
    if (!implement.commits.length) {
      logWorker({ ticket: issue, branch, phase: "implementer", status: "no_commit" });
      console.log("实装无 commit（被卡或无活），跳评审，留票待查。");
      continue;
    }
    logWorker({ ticket: issue, branch, phase: "implementer", status: "completed", commits: implement.commits.length });
    console.log(`实装完成：${implement.commits.length} 个 commit`);

    // Phase 2：独立评审（不同子代理，#1001 裁 1；直接在分支上修正；#1066 多轮续跑）
    logWorker({ ticket: issue, branch, phase: "reviewer", status: "started" });
    await runWithResume(sandbox, {
      name: "reviewer",
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
