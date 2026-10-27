// #1588 ③：评审工蜂提示的宿主侧构造 + 体积闸。
// 成因：review-prompt.md 曾以 `!`git diff main...branch`` 把整段 diff 塞进提示，#1406（37 提交、
// diff 约 163 万字符 ≈ 39 万 token）直接「Prompt is too long」，main.mts exit=1，workers.jsonl
// 最后一条停在 sandbox/closed，票保持已认领、无任何失败记录与通知（静默结束）。
// 现改为：提示只放变更摘要（diff --stat / 提交列表 / 文件清单 + 原始 issue），评审工蜂在工作区里
// 按需逐文件 / 逐提交读 diff；提示在宿主侧渲染、按估算 token 计体积，超上限显式失败
// （结构化记录进 workers.jsonl + 票面评论通知 + 非零退出码），不再交给 agent 去撞上下文墙。
// 渲染结果以 inline prompt 交给 sandbox.run——sandcastle 对 inline prompt 不做 `!`cmd`` 展开，
// 提交标题 / issue 正文里的反引号不会被当成 shell 执行。
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";

/** 评审提示体积上限（估算 token）。Claude 上下文 200k，留足余量给评审工蜂逐文件读 diff 与多轮续跑。 */
export const REVIEW_PROMPT_MAX_TOKENS = 60_000;
/** 提示超限时 main.mts 的退出码（区别于未捕获异常的 1 与 FAIL-loud 启动校验的 2）。 */
export const REVIEW_PROMPT_TOO_LARGE_EXIT = 4;

export const REVIEW_PROMPT_FILE = "./.sandcastle/review-prompt.md";

/**
 * 保守估算 token：ASCII 按 4 字符 / token（与 sandcastle 展开日志 `~N tokens` 同口径），
 * 非 ASCII（中文等）按 1 字符 / token——中文 issue 正文用 4 字符口径会严重低估。
 */
export function estimateTokens(text: string): number {
  let ascii = 0;
  let other = 0;
  for (const ch of text) {
    if (ch.charCodeAt(0) < 0x80) ascii++;
    else other++;
  }
  return Math.ceil(ascii / 4) + other;
}

export class ReviewPromptTooLargeError extends Error {
  readonly exitCode = REVIEW_PROMPT_TOO_LARGE_EXIT;
  constructor(
    readonly estimatedTokens: number,
    readonly limitTokens: number,
    readonly chars: number,
  ) {
    super(
      `[FAIL-loud #1588] 评审提示估算 ~${estimatedTokens} tokens（${chars} 字符）超上限 ${limitTokens}，拒绝启动评审工蜂`,
    );
    this.name = "ReviewPromptTooLargeError";
  }
}

/** 宿主侧 git 调用（参数数组，不经 shell）。 */
export type GitRunner = (args: string[]) => string;
/** 原始 issue 文本（title + body）。 */
export type IssueFetcher = (issue: number) => string;

export function hostGit(cwd?: string): GitRunner {
  return (args) =>
    execFileSync("git", args, { encoding: "utf8", maxBuffer: 64 * 1024 * 1024, ...(cwd ? { cwd } : {}) });
}

export interface ReviewSummary {
  diffStat: string;
  commits: string;
  files: string;
}

/** 变更摘要三件：`--stat`、提交列表、带 A/M/D/R 的文件清单。都不含 diff 正文。 */
export function collectReviewSummary(git: GitRunner, target: string, branch: string): ReviewSummary {
  const range = `${target}...${branch}`;
  return {
    diffStat: git(["diff", "--stat=200", range]).trimEnd(),
    commits: git(["log", "--oneline", "--no-decorate", `${target}..${branch}`]).trimEnd(),
    files: git(["diff", "--name-status", range]).trimEnd(),
  };
}

/** `{{KEY}}` 替换；模板里留有未提供的占位符即报错（与 sandcastle promptArgs 校验同口径）。 */
// 单趟替换：替换进来的值（如 issue 正文）里即使含 {{X}} 也不会被二次展开。
export function renderTemplate(template: string, vars: Record<string, string>): string {
  const placeholder = /\{\{([A-Z0-9_]+)\}\}/g;
  const missing = [...new Set([...template.matchAll(placeholder)].map((m) => m[1]!))].filter(
    (k) => !Object.prototype.hasOwnProperty.call(vars, k),
  );
  if (missing.length) throw new Error(`评审提示模板占位符缺值：${missing.map((k) => `{{${k}}}`).join(", ")}`);
  return template.replace(placeholder, (_whole, key: string) => vars[key]!);
}

// ── 评审 diff 基准（#1588 N3） ───────────────────────────────────────────────
// 摘要在宿主 cwd 里按 `<基准>...<分支>` 取。宿主 main 落后 origin/main、而分支从更新的 main 分出时
// （REVIEW_ONLY 补派别处建的分支、宿主久未刷新），`main...分支` 的 merge-base 落在旧 main 上，
// 别线已合入的提交会混进摘要（本机实测：同一 #1406 分支对旧 main 渲染直接超 60k，exit 4）。
// 选法：渲染前现取 origin/main，以其提交 SHA 作基准（而非 fast-forward 宿主 main）。理由：
//  1. 三点 diff 取 merge-base：分支无论分自旧 main 还是新 main，对「刚 fetch 的 origin/main」求
//     merge-base 都落在真实分叉点，摘要只含本分支提交；fast-forward 宿主 main 能达到同样效果，
//     但要求宿主 worktree 干净、会改动别的进程正在用的检出（那是 claim-loop refreshHostMain 的职责）。
//  2. 钉死 SHA：评审工蜂整个运行期 diff 基准不随 ref 移动；工蜂工作区与宿主共享 refs，
//     其 `main` 仍是宿主旧 main，提示里的读 diff 命令一律用这个 SHA，不用 `main`。
//  3. 宿主 main 含 origin/main 没有的提交（本地未推提交 / 分叉）时无法判断哪边是对的基准，
//     FAIL-loud 拒绝渲染（记 reason=prepare_error），不猜。

export const REVIEW_BASE_REMOTE = "origin";
export const REVIEW_BASE_BRANCH = "main";

export interface ResolveReviewBaseOptions {
  /** git fetch 最多尝试次数（含首次）；网络抖动 / 并发 fetch 抢 ref 锁时指数退避重试。 */
  attempts?: number;
  /** 首次重试前等待毫秒数，之后每次翻倍。 */
  baseDelayMs?: number;
  /** 同步等待（测试注入，免真睡）。 */
  sleep?: (ms: number) => void;
  log?: (msg: string) => void;
}

function sleepSync(ms: number): void {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, ms);
}

/** fetch origin main 后返回 origin/main 的提交 SHA，作评审 diff 基准；宿主 main 领先/分叉即抛。 */
export function resolveReviewBase(git: GitRunner, opts: ResolveReviewBaseOptions = {}): string {
  const attempts = opts.attempts ?? 4;
  const baseDelayMs = opts.baseDelayMs ?? 2000;
  const sleep = opts.sleep ?? sleepSync;
  const log = opts.log ?? ((msg: string) => console.error(msg));
  const remoteRef = `refs/remotes/${REVIEW_BASE_REMOTE}/${REVIEW_BASE_BRANCH}`;
  const localRef = `refs/heads/${REVIEW_BASE_BRANCH}`;
  for (let attempt = 1; ; attempt++) {
    try {
      git(["fetch", "--quiet", REVIEW_BASE_REMOTE, `+${localRef}:${remoteRef}`]);
      break;
    } catch (e) {
      if (attempt >= attempts) {
        throw new Error(
          `[FAIL-loud #1588] git fetch ${REVIEW_BASE_REMOTE} ${REVIEW_BASE_BRANCH} 连续 ${attempts} 次失败，` +
            `拿不到最新基准，拒绝渲染评审提示：${String(e).slice(0, 300)}`,
        );
      }
      const delay = baseDelayMs * 2 ** (attempt - 1);
      log(`[#1588] git fetch ${REVIEW_BASE_REMOTE} ${REVIEW_BASE_BRANCH} 第 ${attempt}/${attempts} 次失败，${delay}ms 后重试`);
      sleep(delay);
    }
  }
  const base = git(["rev-parse", "--verify", `${remoteRef}^{commit}`]).trim();
  const ahead = Number(git(["rev-list", "--count", `${remoteRef}..${localRef}`]).trim());
  if (ahead > 0) {
    throw new Error(
      `[FAIL-loud #1588] 宿主 ${REVIEW_BASE_BRANCH} 含 ${ahead} 个 ${REVIEW_BASE_REMOTE}/${REVIEW_BASE_BRANCH} 没有的提交` +
        `（本地未推 / 分叉），评审基准无法判定，拒绝渲染评审提示；请先核对宿主 ${REVIEW_BASE_BRANCH}。`,
    );
  }
  const behind = Number(git(["rev-list", "--count", `${localRef}..${remoteRef}`]).trim());
  if (behind > 0) {
    log(
      `[#1588] 宿主 ${REVIEW_BASE_BRANCH} 落后 ${REVIEW_BASE_REMOTE}/${REVIEW_BASE_BRANCH} ${behind} 个提交；` +
        `评审基准改用 ${REVIEW_BASE_REMOTE}/${REVIEW_BASE_BRANCH} ${base.slice(0, 10)}（不改宿主检出）`,
    );
  }
  return base;
}

export interface BuildReviewPromptOptions {
  template: string;
  branch: string;
  /** 评审目标分支名（提示里的称呼，如 main）。 */
  target: string;
  /** diff 基准解析（如 resolveReviewBase）；缺省直接用 target。在渲染内调用，失败归 prepare_error。 */
  resolveBase?: () => string;
  issue: number;
  git: GitRunner;
  fetchIssue: IssueFetcher;
  maxTokens?: number;
}

export interface BuiltReviewPrompt {
  prompt: string;
  estimatedTokens: number;
  chars: number;
  /** 实际用作 diff 基准的 ref / SHA。 */
  base: string;
}

/**
 * issue 原文放进 `<issue>…</issue>`（#1588 N2：旧模板用 ``` 围栏，正文自带围栏会提前闭合）。
 * 正文里字面的 `</issue` 改写成 `<\/issue`，免得提前闭合标签。
 */
export function neutralizeIssueText(text: string): string {
  return text.replace(/<\/(issue)/gi, "<\\/$1");
}

/** 渲染评审提示并过体积闸；超限抛 ReviewPromptTooLargeError（不截断——截断会静默漏掉文件）。 */
export function buildReviewPrompt(opts: BuildReviewPromptOptions): BuiltReviewPrompt {
  const base = opts.resolveBase ? opts.resolveBase() : opts.target;
  const summary = collectReviewSummary(opts.git, base, opts.branch);
  const prompt = renderTemplate(opts.template, {
    BRANCH: opts.branch,
    TARGET_BRANCH: opts.target,
    BASE: base,
    ISSUE_NUMBER: String(opts.issue),
    DIFF_STAT: summary.diffStat || "（无改动）",
    COMMITS: summary.commits || "（无提交）",
    FILES: summary.files || "（无文件）",
    ISSUE: neutralizeIssueText(opts.fetchIssue(opts.issue).trimEnd()),
  });
  const estimatedTokens = estimateTokens(prompt);
  const limit = opts.maxTokens ?? REVIEW_PROMPT_MAX_TOKENS;
  if (estimatedTokens > limit) throw new ReviewPromptTooLargeError(estimatedTokens, limit, prompt.length);
  return { prompt, estimatedTokens, chars: prompt.length, base };
}

/** 失败通道（沿用仓内现成的两条：workers.jsonl → watcher.sh → STATUS.md；票面 gh issue comment）。 */
export interface ReviewFailureChannels {
  logWorker: (e: Record<string, unknown>) => void;
  notify: (issue: number, body: string) => void;
}

export function reviewFailureComment(issue: number, branch: string, err: ReviewPromptTooLargeError): string {
  return [
    `> *This was generated by AI (sandcastle main.mts, #1588).*`,
    ``,
    `**[FAIL-loud] #${issue} 独立评审未启动：评审提示超体积上限。**`,
    ``,
    `- 分支：\`${branch}\`（实装 commit 保留在分支上，未评审）`,
    `- 提示估算：~${err.estimatedTokens} tokens / ${err.chars} 字符；上限 ${err.limitTokens} tokens`,
    `- 提示已只含变更摘要（diff --stat / 提交列表 / 文件清单），仍超限说明改动面过大（文件数过多）。`,
    `- 结构化记录：\`.sandcastle/logs/workers.jsonl\` 中 \`phase=reviewer status=failed reason=prompt_too_large\`。`,
    `- 本票保持认领，不自动回滚（回滚会重跑实装再撞同一处）。处置：拆票 / 拆分支后用`,
    `  \`SANDCASTLE_REVIEW_ONLY=${branch}:${issue} npx tsx .sandcastle/main.mts\` 补派评审，或转人工评审。`,
  ].join("\n");
}

/**
 * 评审失败原因：prepare_error = 渲染提示阶段失败（宿主侧 git / gh 调用、基准解析），工蜂未启动；
 * run_error = 评审工蜂运行期失败（agent 非零退出、被杀等）。
 */
export type ReviewerFailureReason = "prepare_error" | "run_error";

const REVIEWER_FAILURE_HEADLINE: Record<ReviewerFailureReason, string> = {
  prepare_error: "独立评审未启动：宿主侧渲染评审提示失败（git / gh 调用或 diff 基准解析出错）。",
  run_error: "独立评审工蜂运行失败。",
};

/** 失败的票面评论：不贴原始错误文本（错误串可能带命令行 / 路径，只留本地 workers.jsonl）。 */
export function reviewerRunFailureComment(
  issue: number,
  branch: string,
  reason: ReviewerFailureReason = "run_error",
): string {
  return [
    `> *This was generated by AI (sandcastle main.mts, #1588).*`,
    ``,
    `**[FAIL-loud] #${issue} ${REVIEWER_FAILURE_HEADLINE[reason]}**`,
    ``,
    `- 分支：\`${branch}\`（实装 commit 保留在分支上，未评审）`,
    `- 错误摘要见宿主 \`.sandcastle/logs/workers.jsonl\`（\`phase=reviewer status=failed reason=${reason}\`）与 \`.sandcastle/logs/\` 下的 reviewer 日志。`,
    `- 补派评审：\`SANDCASTLE_REVIEW_ONLY=${branch}:${issue} npx tsx .sandcastle/main.mts\`。`,
  ].join("\n");
}

/** 先通知、后落结构化记录（记录里带 notified 真值）；通知失败只记日志，不吞调用方的原错。 */
function recordReviewerFailure(
  issue: number,
  branch: string,
  comment: string,
  record: Record<string, unknown>,
  channels: ReviewFailureChannels,
): void {
  let notified = true;
  try {
    channels.notify(issue, comment);
  } catch (ne) {
    notified = false;
    console.error(`[#1588] 评审失败通知（gh issue comment）失败：${String(ne).slice(0, 300)}`);
  }
  channels.logWorker({ ticket: issue, branch, phase: "reviewer", status: "failed", ...record, notified });
}

/**
 * 构造评审提示；超限时写结构化失败记录并通知，然后原样上抛 ReviewPromptTooLargeError，
 * 由调用方按 err.exitCode 非零退出。
 */
export function prepareReviewPrompt(
  opts: BuildReviewPromptOptions,
  channels: ReviewFailureChannels,
): BuiltReviewPrompt {
  try {
    return buildReviewPrompt(opts);
  } catch (e) {
    if (!(e instanceof ReviewPromptTooLargeError)) throw e;
    recordReviewerFailure(
      opts.issue,
      opts.branch,
      reviewFailureComment(opts.issue, opts.branch, e),
      { reason: "prompt_too_large", est_tokens: e.estimatedTokens, limit_tokens: e.limitTokens, chars: e.chars },
      channels,
    );
    console.error(e.message);
    throw e;
  }
}

/** 每次发送前重计完整提示（含精确 head、验证诊断及续跑反馈），沿用原上限和失败通道。 */
export function prepareFinalReviewPrompt(
  prompt: string, base: string,
  opts: Pick<BuildReviewPromptOptions, "issue" | "branch" | "maxTokens">,
  channels: ReviewFailureChannels,
): BuiltReviewPrompt {
  const estimatedTokens = estimateTokens(prompt);
  const limit = opts.maxTokens ?? REVIEW_PROMPT_MAX_TOKENS;
  if (estimatedTokens > limit) {
    const error = new ReviewPromptTooLargeError(estimatedTokens, limit, prompt.length);
    recordReviewerFailure(opts.issue, opts.branch, reviewFailureComment(opts.issue, opts.branch, error),
      { reason: "prompt_too_large", est_tokens: estimatedTokens, limit_tokens: limit, chars: prompt.length }, channels);
    throw error;
  }
  return { prompt, estimatedTokens, chars: prompt.length, base };
}

/** 评审阶段的非超限失败（渲染期 prepare_error / 运行期 run_error）：同一失败通道，调用方随后原样上抛。 */
export function recordReviewerRunFailure(
  issue: number,
  branch: string,
  error: unknown,
  channels: ReviewFailureChannels,
  reason: ReviewerFailureReason = "run_error",
): void {
  recordReviewerFailure(
    issue,
    branch,
    reviewerRunFailureComment(issue, branch, reason),
    { reason, error: String(error).slice(0, 300) },
    channels,
  );
}

/**
 * 评审阶段整段（渲染提示 + 跑评审工蜂）包进同一失败通道（#1588 N1）：
 * - 超限：prepareReviewPrompt 已记 prompt_too_large 并通知，这里只原样上抛；
 * - 渲染期其它异常（gh issue view / git diff / fetch 基准失败）→ reason=prepare_error；
 * - run 回调抛错 → reason=run_error。
 * 都写 reviewer/failed + 票面评论（不贴原始错误串），然后原样上抛；main.mts / main-swarm.mts 共用。
 */
export async function runGuardedReview<T>(
  opts: BuildReviewPromptOptions,
  channels: ReviewFailureChannels,
  run: (built: BuiltReviewPrompt) => Promise<T>,
): Promise<T> {
  let reason: ReviewerFailureReason = "prepare_error";
  try {
    const built = prepareReviewPrompt(opts, channels);
    reason = "run_error";
    return await run(built);
  } catch (e) {
    if (!(e instanceof ReviewPromptTooLargeError)) recordReviewerRunFailure(opts.issue, opts.branch, e, channels, reason);
    throw e;
  }
}

export function readReviewTemplate(path: string = REVIEW_PROMPT_FILE): string {
  return readFileSync(path, "utf8");
}

// ── 宿主侧 gh IO（main.mts / main-swarm.mts 共用；env 由调用方传入洗过颜色的 GH_CLEAN_ENV） ──

/** gh `--json title,body` 是单行 JSON（换行转义成 \n），还原成可读的标题 + 正文再进 `<issue>` 块。 */
export function formatIssueJson(raw: string): string {
  const { title, body } = JSON.parse(raw) as { title?: unknown; body?: unknown };
  return `标题：${String(title ?? "")}\n\n${String(body ?? "")}`;
}

export function ghIssueFetcher(env: NodeJS.ProcessEnv): IssueFetcher {
  return (issue) =>
    formatIssueJson(
      execFileSync("gh", ["issue", "view", String(issue), "--json", "title,body"], { encoding: "utf8", env }),
    );
}

export function ghIssueNotifier(env: NodeJS.ProcessEnv): ReviewFailureChannels["notify"] {
  return (issue, body) => {
    execFileSync("gh", ["issue", "comment", String(issue), "--body-file", "-"], {
      input: body,
      env,
      stdio: ["pipe", "ignore", "pipe"],
    });
  };
}
