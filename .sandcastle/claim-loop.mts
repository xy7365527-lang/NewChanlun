// claim-loop.mts —— sandcastle 自动拾取缺口补齐（#1182 方向 A：launchd 常驻 claim 循环）。
// 职责单一：常驻轮询——若 frontier（sandcastle 标签 + 未 assign + 无 open blocker）非空
// 且无 main.mts running 实例 → spawn `npx tsx .sandcastle/main.mts` 并等它收尾，下一轮再查。
// 防重复认领靠「无 running 实例」guard（pgrep 判活，TROUBLESHOOTING #9 同款判据）：
// 本循环单线程 spawn+等待，天然不并发多实例；常驻不退出（脚本 for(;;)，launchd KeepAlive 兜底）。
// frontier 查询复用 main.mts 现有逻辑（--label sandcastle、未 assign、无 open blocker）。
//
// 跑法（仓库根，手工调试）：
//   npx tsx .sandcastle/claim-loop.mts --once --dry-run   单轮演练（只查询+打印，不 spawn）
//   npx tsx .sandcastle/claim-loop.mts --live --once      真实执行一轮
//   npx tsx .sandcastle/claim-loop.mts --live             常驻轮询（真实 spawn，默认 60s 一轮）
// 部署（受管常驻，launchd KeepAlive；宿主侧安装由编排者执行 install 脚本）：
//   bash .sandcastle/install-sandcastle-claimer.sh install

import { execSync, spawn } from "node:child_process";

// ── 顶部常量（周期默认 60s；env CLAIM_LOOP_POLL_SECONDS / --poll-seconds 覆盖） ──
const DEFAULT_POLL_SECONDS = 60;

// gh 在 FORCE_COLOR 环境下会给 --json 输出染色，必须洗掉（main.mts 同款）。
const GH_CLEAN_ENV: NodeJS.ProcessEnv = {
  ...process.env,
  NO_COLOR: "1",
  CLICOLOR: "0",
  FORCE_COLOR: "0",
  CLICOLOR_FORCE: "0",
  GH_REPO: process.env.GH_REPO ?? "xy7365527-lang/NewChanlun",
};

// ── frontier 候选（纯函数吃这个归一化视图，可离线单测） ──────────────────────
export interface FrontierCandidate {
  number: number;
  assignees: unknown[];
  /** open blocker 数量；>0 表示被阻塞。 */
  blockedBy: number;
}

/** 单票是否可拾取（纯函数）：未 assign 且无 open blocker。 */
export function isRunnable(c: FrontierCandidate): boolean {
  return c.assignees.length === 0 && c.blockedBy === 0;
}

/** frontier 是否有可拾取票（纯函数）：至少一张合资格。 */
export function hasRunnableIssue(candidates: FrontierCandidate[]): boolean {
  return candidates.some(isRunnable);
}

// ── IO 层：frontier 查询（复用 main.mts 现有逻辑） ────────────────────────────

/** 单票是否有 open blocker（main.mts hasOpenBlocker 同款：jq blocked_by 计数 >0）。 */
function hasOpenBlocker(issue: number): boolean {
  const out = execSync(
    `gh api repos/{owner}/{repo}/issues/${issue} --jq .issue_dependencies_summary.blocked_by`,
    { encoding: "utf8", env: GH_CLEAN_ENV },
  ).trim();
  return Number(out) > 0;
}

/** sandcastle 队列（open + sandcastle 标签）+ 逐票 blocker 查询，归并为 frontier 候选。 */
function fetchFrontier(): FrontierCandidate[] {
  const out = execSync(
    `gh issue list --label sandcastle --state open --limit 20 --json number,assignees`,
    { encoding: "utf8", env: GH_CLEAN_ENV },
  );
  const issues = JSON.parse(out) as Array<{ number: number; assignees: unknown[] }>;
  return issues.map((i) => ({
    ...i,
    // 已 assign 的票与本判定无关，跳过 blocker 查询（省 gh api 调用）。
    blockedBy: i.assignees.length === 0 && hasOpenBlocker(i.number) ? 1 : 0,
  }));
}

/**
 * main.mts 是否有 running 实例（pgrep 判活；exit 0 = 有匹配 = 在跑）。
 * 模式用 `[.]…/[m]ain[.]mts` 自避括号：pgrep -f 会匹配到「执行本命令的 sh -c 父壳」，
 * 若模式字面出现在命令串里会永久误报为「在跑」；括号字符类让正则不命中自身命令串
 * （与 TROUBLESHOOTING #9 的 `[t]sx` 同款手法）。
 */
function isMainMtsRunning(): boolean {
  try {
    execSync(`pgrep -f "[.]sandcastle/[m]ain[.]mts"`, { encoding: "utf8", env: GH_CLEAN_ENV });
    return true;
  } catch {
    return false;
  }
}

/** spawn main.mts 并等它收尾（单线程，等待期间不进入下一轮判定）。 */
function runMainMts(): Promise<number | null> {
  return new Promise((resolve) => {
    const child = spawn("npx", ["tsx", ".sandcastle/main.mts"], {
      cwd: process.cwd(),
      env: GH_CLEAN_ENV,
      stdio: "inherit",
    });
    child.on("error", (e) => {
      console.error(`[claim-loop] spawn main.mts 失败：${e.message}`);
      resolve(null);
    });
    child.on("exit", (code) => resolve(code));
  });
}

// ── 单轮 ──────────────────────────────────────────────────────────────────────

async function runOnce(cfg: { dryRun: boolean }): Promise<void> {
  if (isMainMtsRunning()) {
    console.log("[claim-loop] main.mts running 实例在跑，本轮跳过（不并发多实例）");
    return;
  }
  const candidates = fetchFrontier();
  if (!hasRunnableIssue(candidates)) {
    console.log(
      `[claim-loop] frontier 无可拾取票（sandcastle 队列 ${candidates.length} 张：均已 assign 或仍有 open blocker），本轮无事`,
    );
    return;
  }
  const runnable = candidates.filter(isRunnable).map((c) => `#${c.number}`).join(" ");
  if (cfg.dryRun) {
    console.log(`[claim-loop] [dry-run] 将 spawn main.mts（可拾取：${runnable}）`);
    return;
  }
  console.log(`[claim-loop] frontier 非空且无 running 实例（可拾取：${runnable}）→ spawn main.mts`);
  const status = await runMainMts();
  console.log(`[claim-loop] main.mts 收尾 exit=${status ?? "spawn 失败"}，下一轮再查`);
}

// ── 参数与循环壳 ──────────────────────────────────────────────────────────────

function parseArgs(argv: string[]): { dryRun: boolean; once: boolean; pollSeconds: number } {
  let dryRun = true; // 安全默认：真实 spawn 须显式 --live（wayfinder_engine 同款口径）
  let once = false;
  const envPoll = Number(process.env.CLAIM_LOOP_POLL_SECONDS ?? DEFAULT_POLL_SECONDS);
  let pollSeconds = Number.isFinite(envPoll) && envPoll > 0 ? envPoll : DEFAULT_POLL_SECONDS;
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i]!;
    if (a === "--live") dryRun = false;
    else if (a === "--dry-run") dryRun = true;
    else if (a === "--once") once = true;
    else if (a === "--poll-seconds") {
      const n = Number(argv[i + 1]);
      if (Number.isFinite(n) && n > 0) {
        pollSeconds = n;
        i++;
      }
    }
  }
  return { dryRun, once, pollSeconds };
}

const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

async function main(): Promise<void> {
  const cfg = parseArgs(process.argv.slice(2));
  console.log(`[claim-loop] 启动：dry-run=${cfg.dryRun} poll=${cfg.pollSeconds}s`);
  if (cfg.once) {
    await runOnce(cfg);
    return;
  }
  for (;;) {
    try {
      await runOnce(cfg);
    } catch (e) {
      // 单次 gh 网络失败不永久停摆：记日志后按周期重试（wayfinder_engine 同款口径）。
      console.error(`[claim-loop] 一轮异常（不退出，下轮重试）：${String(e).slice(0, 300)}`);
    }
    await sleep(cfg.pollSeconds * 1000);
  }
}

if ((import.meta as { main?: boolean }).main) {
  main();
}
