// Sequential Reviewer（Codex/Astra 版，#1617）—— #1001/#1002/#1003 裁定落地。
// 每票：host 侧 frontier 取票 + claim → createSandbox（确定性分支，baseBranch=main）
//   → Phase 1 实装工蜂 → Phase 2 独立评审工蜂（直接在分支上修正）
//   → 不合 main（人工闸，#1003 裁 3）。
// 跑法：npx tsx .sandcastle/main.mts

import { useScheduler } from "./scheduler-config.mts";
import { withLegacyAdmission, DrainFrozenError } from "./drain-control.mts";
import * as sandcastle from "@ai-hero/sandcastle";
// #1547：沙盒后端暂用 noSandbox（agent 直接在宿主上、于 sandcastle 自建的独立 worktree 内运行）。
// Codex workspace-write 保留 Git metadata 只读；宿主受管步骤封包提交（#1617）。
// Docker namespace 工具执行尚未验收，镜像构建或 API 连通不构成切回条件。
import { noSandbox } from "@ai-hero/sandcastle/sandboxes/no-sandbox";
import type { AgentProvider, Sandbox, SandboxRunResult } from "@ai-hero/sandcastle";
import { execSync } from "node:child_process";
import { appendFileSync, existsSync, mkdirSync } from "node:fs";
import { cargoGuardEnv, installCargoGuard } from "./cargo-guard.mts";
import { codexWorker, codexRouteEnv } from "./codex-route.mts";
import { beginManagedStage, finishManagedStage, verifyManagedRust, runManagedReview, verificationPrompt, type RustVerification } from "./managed-commit.mts";
import { MAIN_MODEL } from "./main-model.mts";
import {
  ghIssueFetcher,
  ghIssueNotifier,
  hostGit,
  readReviewTemplate,
  prepareFinalReviewPrompt,
  resolveReviewBase,
  ReviewPromptTooLargeError,
  runGuardedReview,
} from "./review-context.mts";

const scheduler = useScheduler("main", ["workerEffort", "mainMaxIterations", "mainMaxTickets", "implementerIdleSeconds", "worktreeMaxAttempts", "worktreeRetryDelayMs", "frontierQueryLimit", "backlogQueryLimit"]);

const TOKEN_ROTATION_MARKER = "/Users/silencehan/Projects/NewChanlun/.sandcastle/logs/GH_TOKEN_ROTATION_REQUIRED";
if (existsSync(TOKEN_ROTATION_MARKER)) {
  console.error(`[FAIL-loud #1182] GitHub token 尚未轮换（marker=${TOKEN_ROTATION_MARKER}），拒绝启动`);
  process.exit(78);
}

// ── 顶部常量（迭代与资源面；模型唯一来源是 main-model.mts） ────────────────
// #1617：实装与独审均与主控同型；独立性来自不同 invocation，而非异模型。
const IMPLEMENTER_MODEL = MAIN_MODEL;
const REVIEWER_MODEL = MAIN_MODEL;
const EFFORT = scheduler.workerEffort;
// #1066：每票每阶段（implementer/reviewer）最多续跑轮数；续跑复用原 agent 会话
// （runWithResume），而非 sandcastle 内建 maxIterations 的「多轮各自全新会话」。
const MAX_ITERATIONS = scheduler.mainMaxIterations;
// 每次运行认领并处理的最大票数（与每阶段续跑轮数无关；本票不扩大取票吞吐，#1066 未涉此闸）。
const MAX_TICKETS_PER_RUN = scheduler.mainMaxTickets;
// #1066：implementer 数据拉取/长静默任务不被默认 600s idle fail 误杀（#1044 第一次空转形态）。
const IMPLEMENTER_IDLE_TIMEOUT_SECONDS = scheduler.implementerIdleSeconds;

// ── frontier 查询（host 侧）：open + sandcastle label + 未 assign + 无 open blocker ──
// blocker 过滤吃 tracker 原生依赖边（#1005 补齐；与 prompt 层判定叠加，#1007 裁 3）。
const GH_CLEAN_ENV = {
  ...process.env,
  // gh 在 FORCE_COLOR 环境下会给 --json 输出染色，必须洗掉
  NO_COLOR: "1", CLICOLOR: "0", FORCE_COLOR: "0", CLICOLOR_FORCE: "0",
  GH_REPO: "xy7365527-lang/NewChanlun", // origin 可能是本地路径（clone 场景），gh 靠它认仓
};

// ── #1183：宿主分支 FAIL-loud 校验（启动即查） ────────────────────────────────
// 由来：评审面曾用 sandcastle 内置 promptArg TARGET_BRANCH（解析自宿主 cwd 当前分支，
// TROUBLESHOOTING #8 / #1008「不可覆盖」）。宿主 worktree 常驻他人 ticket 分支时，评审基线被带偏成
// 「别线 WIP + 本票」混合 diff——#1180 实测：ticket-919-final → ~900179 tokens、评审 agent 14s 空转。
// #1588 起评审提示改在宿主侧渲染，不再依赖内置 TARGET_BRANCH；diff 基准也不再取宿主本地 main，
// 而是渲染前现 fetch 的 origin/main 提交（resolveReviewBase：宿主 main 落后时别线提交会混进摘要，
// 领先/分叉即 FAIL-loud）。本校验仍保留，理由：
//  - 宿主 cwd 是 main 的干净检出这一前提，claim-loop refreshHostMain（reset --hard origin/main）依赖它；
//  - 内置 TARGET_BRANCH 仍按宿主当前分支解析，任何仍走 promptFile 的模板若引用它，须与 main 同源；
//  - .sandcastle/logs/workers.jsonl 等相对路径以宿主 cwd 为根，别线 worktree 下运行会写错地方。
// 非 main 直接 FAIL-loud 退出（不取票、不起沙盒）。
function assertHostBranchIsMain(): void {
  let branch: string;
  try {
    branch = execSync("git rev-parse --abbrev-ref HEAD", { encoding: "utf8" }).trim();
  } catch (e) {
    console.error(
      `[FAIL-loud #1183] 解析宿主分支失败（git rev-parse --abbrev-ref HEAD）：${String(e).slice(0, 300)}`,
    );
    process.exit(2);
  }
  if (branch !== "main") {
    console.error(
      `[FAIL-loud #1183] 宿主分支 = ${branch}（期望 main）。编排须在 main 的干净检出下运行：` +
        `claim-loop 刷新宿主 main、sandcastle 内置 TARGET_BRANCH、workers.jsonl 路径都以宿主 cwd 为准，` +
        `非 main 会把别线 WIP 混进编排面（#1180 实测：ticket-919-final 基线 → 评审空转）。` +
        `请在 main 的干净 worktree（main-pstack）下重跑：npx tsx .sandcastle/main.mts`,
    );
    process.exit(2);
  }
}

function resolveSandboxGithubEnv(): Record<string, string> {
  try {
    const token = execSync("gh auth token", { encoding: "utf8", env: GH_CLEAN_ENV }).trim();
    if (!token) throw new Error("gh auth token returned empty output");
    // Token 仅进入临时 Docker env；不写 .env / plist / 日志。
    return { GH_TOKEN: token, GH_REPO: GH_CLEAN_ENV.GH_REPO! };
  } catch (e) {
    console.error(`[FAIL-loud #1182] 无法从系统 keyring 取得 GitHub token，拒绝启动沙盒：${String(e).slice(0, 300)}`);
    process.exit(2);
  }
}

// #1588 ④：本机 cargo 垫片（固定目录 $TMPDIR/nc-cargo-guard，每次运行原地刷新），前置到工蜂 PATH；
// Rust 验证走 nc-remote / CI。
const CARGO_GUARD = installCargoGuard();

function sandboxProvider() {
  // 路由变量走沙盒后端 env（理由见 claude-route.mts claudeWorker 注释）。
  return noSandbox({ env: { ...resolveSandboxGithubEnv(), ...codexRouteEnv(), ...cargoGuardEnv(CARGO_GUARD) } });
}

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

// ── #1259：claim 成功后、implementer started 之前的任何失败必须自动回滚认领（防卡死） ──
// WorktreeTimeoutError（sandcastle 内建 30s worktree 创建阈值，代理瞬断时高发）先重试一次
// （间隔 ≥5s），仍失败才回滚；其他异常不吞错、不重试，直接回滚后原样上抛。回滚动作与
// #1007 摘标/assign 对称：remove-assignee @me + add-label sandcastle。claim-loop 收尾处还有
// 一道按 workers.jsonl 判定的兜底回滚（覆盖 main.mts 被杀/收不到自己异常的场景）。
const WORKTREE_TIMEOUT_MAX_ATTEMPTS = scheduler.worktreeMaxAttempts;
const WORKTREE_TIMEOUT_RETRY_DELAY_MS = scheduler.worktreeRetryDelayMs;

function sleep(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms));
}

/**
 * sandcastle 的 WorktreeTimeoutError 未从包公共 API 导出，且经 Effect.runPromise 抛出时被
 * 包成 FiberFailure（name 形如 "(FiberFailure) WorktreeTimeoutError"、message 为原始超时文案），
 * 故按 name / message 双重判读（不依赖 _tag——FiberFailure 不透传内层 _tag）。
 */
function isWorktreeTimeoutError(e: unknown): boolean {
  if (typeof e !== "object" || e === null) return false;
  const name = (e as { name?: unknown }).name;
  const message = (e as { message?: unknown }).message;
  return (
    (typeof name === "string" && name.includes("WorktreeTimeoutError")) ||
    (typeof message === "string" &&
      (message.includes("Worktree creation timed out") || message.includes("Worktree prune timed out")))
  );
}

/** 回滚认领（best-effort）：unassign @me + 恢复 sandcastle 标签 + workers.jsonl 追加 rolled_back。
 *  回滚本身失败不吞原错（原异常照常上抛、exit 1），claim-loop 收尾兜底会按 workers.jsonl 再试。 */
async function rollbackClaim(issue: number, branch: string): Promise<void> {
  try {
    await withLegacyAdmission(() => execSync(`gh issue edit ${issue} --remove-assignee @me --add-label sandcastle`, { env: GH_CLEAN_ENV }));
    logWorker({ ticket: issue, branch, phase: "claim", status: "rolled_back" });
    console.error(`[#1259] 票 #${issue} 认领已回滚：unassigned + sandcastle 标签恢复，下轮可重拾。`);
  } catch (e) {
    console.error(`[#1259] 票 #${issue} 认领回滚失败（人工兜底：解除认领 + 重新入队）：${String(e).slice(0, 300)}`);
  }
}

/** createSandbox + WorktreeTimeoutError 一次重试（间隔 ≥5s）；其他异常直接上抛。 */
async function createSandboxWithWorktreeRetry(opts: { branch: string }): Promise<Sandbox> {
  for (let attempt = 1; ; attempt++) {
    try {
      return await sandcastle.createSandbox({
        branch: opts.branch,
        baseBranch: "main", // 与检出分支解耦（#1003：并行会话共存）
        sandbox: sandboxProvider(),
      });
    } catch (e) {
      if (attempt < WORKTREE_TIMEOUT_MAX_ATTEMPTS && isWorktreeTimeoutError(e)) {
        console.error(
          `[#1259] createSandbox 抛 WorktreeTimeoutError（第 ${attempt}/${WORKTREE_TIMEOUT_MAX_ATTEMPTS} 次），` +
            `${WORKTREE_TIMEOUT_RETRY_DELAY_MS}ms 后重试：${String(e).slice(0, 300)}`,
        );
        await sleep(WORKTREE_TIMEOUT_RETRY_DELAY_MS);
        continue;
      }
      throw e;
    }
  }
}

function pickIssue(): number | null {
  const out = execSync(
    `gh issue list --label sandcastle --state open --limit ${scheduler.frontierQueryLimit} --json number,assignees`,
    { encoding: "utf8", env: GH_CLEAN_ENV },
  );
  const open = (JSON.parse(out) as Array<{ number: number; assignees: unknown[] }>)
    .filter((i) => i.assignees.length === 0);
  for (const i of open) {
    if (!hasOpenBlocker(i.number)) return i.number;
  }
  return null;
}

// #1084 追加：sandcastle 专属队列空 ≠ 全仓 ready-for-agent 空。分开报，
// 状态页不再把「专属队列为空」误写成「全仓 AFK 做完」。
function repoReadyForAgentCount(): number {
  const out = execSync(
    `gh issue list --label ready-for-agent --state open --limit ${scheduler.backlogQueryLimit} --json number --jq length`,
    { encoding: "utf8", env: GH_CLEAN_ENV },
  ).trim();
  return Number(out);
}

// ── 多轮续跑（#1066）：同一票的 implementer/reviewer 续跑必须复用原 agent 会话。
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
    // promptFile + promptArgs（sandcastle 展开 `!`cmd``），或宿主侧已渲染好的 inline prompt（#1588 评审面）。
    prompt: { promptFile: string; promptArgs: Record<string, string> } | { prompt: string };
    idleTimeoutSeconds?: number;
    resumeSession?: string;
  },
): Promise<SandboxRunResult> {
  let resumeSession: string | undefined = options.resumeSession;
  let result: SandboxRunResult | undefined;
  for (let round = 1; round <= MAX_ITERATIONS; round++) {
    result = await sandbox.run({
      name: options.name,
      maxIterations: 1,
      agent: options.agent,
      ...options.prompt,
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

// ── #1588 ③：评审阶段。提示在宿主侧渲染（只放变更摘要，不放整段 diff）并过体积闸；
// 渲染与运行包在同一失败通道（runGuardedReview）：超限 / 渲染失败（gh、git、fetch 基准）/ 运行失败
// 都显式落 workers.jsonl（phase=reviewer status=failed reason=prompt_too_large|prepare_error|run_error）
// + 票面评论，再原样上抛。旧形态失败时 workers.jsonl 停在 reviewer/started → sandbox/closed，
// 票保持认领、无人知晓（#1406）。
// diff 基准不用宿主本地 main，而是渲染前现取的 origin/main 提交（resolveReviewBase，理由见其注释，#1588 N3）。
const REVIEW_TARGET_BRANCH = "main";

async function runReviewer(sandbox: Sandbox, issue: number, branch: string, authorVerification?: RustVerification): Promise<void> {
  const git = hostGit();
  await runGuardedReview(
    {
      template: readReviewTemplate("./.sandcastle/codex-review-prompt.md"),
      branch,
      target: REVIEW_TARGET_BRANCH,
      resolveBase: () => resolveReviewBase(git),
      issue,
      git,
      fetchIssue: ghIssueFetcher(GH_CLEAN_ENV),
    },
    { logWorker, notify: ghIssueNotifier(GH_CLEAN_ENV) },
    async ({ prompt, base }) => {
      const first = beginManagedStage(sandbox.worktreePath, branch, issue, "reviewer");
      // REVIEW_ONLY 没有作者内存证据：按整个候选 diff 决定远端验证，不能按“独审无改动”跳过。
      const verification = authorVerification ?? verifyManagedRust(first, {
        head: first.head, committed: false,
        report: { status: "ready", files: hostGit(first.cwd)(["diff", "--name-only", "--no-renames", "-z", `${base}...${first.head}`]).split("\0").filter(Boolean), verification: [] },
      });
      const agent = codexWorker({ effort: EFFORT });
      let resumeSession: string | undefined;
      const result = await runManagedReview(sandbox.worktreePath, branch, issue, verification, async (stage, evidence) => {
        const final = prepareFinalReviewPrompt(
          prompt + `\n本次评审必须读取精确候选 commit ${stage.head}。\n${verificationPrompt(evidence)}`,
          base, { issue, branch }, { logWorker, notify: ghIssueNotifier(GH_CLEAN_ENV) },
        );
        logWorker({ ticket: issue, branch, phase: "reviewer", status: "started", prompt_est_tokens: final.estimatedTokens, review_base: base });
        const review = await runWithResume(sandbox, {
          name: "reviewer", agent, resumeSession,
          prompt: { prompt: final.prompt },
        });
        resumeSession = review.iterations.at(-1)?.sessionId;
        return review.stdout;
      });
      logWorker({ ticket: issue, branch, phase: "reviewer", status: "sealed", head: result.head, verification: result });

    },
  );
  logWorker({ ticket: issue, branch, phase: "reviewer", status: "completed" });
}

// ── REVIEW_ONLY 模式（#879 补派 Phase 2 引入）：env SANDCASTLE_REVIEW_ONLY="分支名:票号" 时
// 跳过 pickIssue 与 Phase 1，对既有分支直接跑 Phase 2 评审（宿主驱动中断后的补派路径，
// TROUBLESHOOTING #11）。分支须已存在且含实装 commit。
assertHostBranchIsMain();
const REVIEW_ONLY = process.env.SANDCASTLE_REVIEW_ONLY;
if (REVIEW_ONLY) {
  const [roBranch, roIssue] = REVIEW_ONLY.split(":");
  if (!roBranch || !roIssue) {
    console.error("SANDCASTLE_REVIEW_ONLY 格式须为 分支名:票号");
    process.exit(2);
  }
  const issue = Number(roIssue);
  const branch = roBranch;
  const sandbox = await withLegacyAdmission(() => sandcastle.createSandbox({
    branch,
    baseBranch: "main",
    sandbox: sandboxProvider(),
  }));
  try {
    await runReviewer(sandbox, issue, branch);
  } catch (e) {
    await sandbox.close();
    if (e instanceof ReviewPromptTooLargeError) process.exit(e.exitCode);
    throw e;
  }
  console.log(`REVIEW_ONLY 完成：${branch} 待验收合入（人工闸）。`);
  await sandbox.close();
  process.exit(0);
}

for (let iter = 1; iter <= MAX_TICKETS_PER_RUN; iter++) {
  let issue: number | null;
  try {
    issue = await withLegacyAdmission(() => {
      const picked = pickIssue();
      if (picked) {
        execSync(`gh issue edit ${picked} --remove-label sandcastle --add-assignee @me`);
        logWorker({ ticket: picked, branch: `sandcastle/issue-${picked}`, phase: "claim", status: "claimed" });
      }
      return picked;
    });
  } catch (error) {
    if (!(error instanceof DrainFrozenError)) throw error;
    console.log(error.message);
    break;
  }
  if (!issue) {
    let readyForAgent: number | null = null;
    try {
      readyForAgent = repoReadyForAgentCount();
    } catch (e) {
      console.error(`sandcastle 专属队列空；全仓 ready-for-agent 计数查询失败（忽略，不阻断收尾）：${String(e).slice(0, 200)}`);
    }
    console.log(
      `sandcastle 专属队列空（无可拾取 issue），停。全仓 ready-for-agent open ≈ ${
        readyForAgent === null ? "未知" : readyForAgent
      } 张（含未交棒/blocked/claimed，由控制面 wayfinder_engine 交棒，非全空）。`,
    );
    break;
  }

  // 认领（#1007 裁 3 + #1013 抢票教训）：摘 sandcastle + assign @me；拾取闸用专属 label，不碰 ready-for-agent 共享面
  const branch = `sandcastle/issue-${issue}`;
  console.log(`\n=== Iteration ${iter}/${MAX_TICKETS_PER_RUN}: issue #${issue} → ${branch} ===\n`);

  // #1259：claim 成功后、implementer started 之前的失败都回滚认领（WorktreeTimeoutError 先重试一次）。
  let sandbox: Sandbox;
  try {
    sandbox = await createSandboxWithWorktreeRetry({ branch });
  } catch (e) {
    await rollbackClaim(issue, branch);
    throw e;
  }
  try {
    // Phase 1：实装（#1066：多轮续跑同一会话 + idle 1800s 防长静默误杀）
    logWorker({ ticket: issue, branch, phase: "implementer", status: "started" });
    const stage = beginManagedStage(sandbox.worktreePath, branch, issue, "implementer");
    const implement = await runWithResume(sandbox, {
      name: "implementer",
      agent: codexWorker({ effort: EFFORT }),
      prompt: { promptFile: "./.sandcastle/codex-implement-prompt.md", promptArgs: { ISSUE_NUMBER: String(issue) } },
      idleTimeoutSeconds: IMPLEMENTER_IDLE_TIMEOUT_SECONDS,
    });
    const sealed = finishManagedStage(stage, implement.stdout);
    const authorVerification = verifyManagedRust(stage, sealed);
    if (!sealed.committed) {
      logWorker({ ticket: issue, branch, phase: "implementer", status: "no_commit" });
      console.log("实装无 commit（被卡或无活），跳评审，留票待查。");
      continue;
    }
    logWorker({ ticket: issue, branch, phase: "implementer", status: authorVerification.status === "failed" ? "verification_failed" : "completed", commits: 1, head: sealed.head, verification: authorVerification });
    console.log(`实装受管提交完成：${sealed.head}`);

    // Phase 2：独立评审（不同子代理，#1001 裁 1；直接在分支上修正；#1066 多轮续跑）
    // #1588：提示超限 → 已落失败记录与票面通知，以专用退出码收尾（票保持认领，不回滚重跑实装）。
    try {
      await runReviewer(sandbox, issue, branch, authorVerification);
    } catch (e) {
      if (!(e instanceof ReviewPromptTooLargeError)) throw e;
      process.exitCode = e.exitCode;
      break;
    }
    console.log(`两段完成：${branch} 待验收合入（人工闸）。`);
  } catch (error) {
    logWorker({ ticket: issue, branch, phase: "managed-stage", status: "failed", reason: String(error).slice(0, 500) });
    throw error;
  } finally {
    logWorker({ ticket: issue, branch, phase: "sandbox", status: "closed" });
    await sandbox.close();
  }
}
