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
import { appendFileSync, mkdirSync, readFileSync } from "node:fs";

// ── 顶部常量（周期默认 60s；env CLAIM_LOOP_POLL_SECONDS / --poll-seconds 覆盖） ──
const DEFAULT_POLL_SECONDS = 60;

// #1259 看门狗：claim 事件后无任何后续事件且无 running main.mts 的票，超此时长才回滚
// （覆盖「claim-loop 与 main.mts 同时失联」盲区；每轮附带检查，不新增进程）。
const CLAIM_ROLLBACK_WATCHDOG_MS = 15 * 60 * 1000;

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

// ── #1259：认领回滚判定（纯函数，吃 workers.jsonl 归一化视图，可离线单测） ──────
export interface WorkerEvent {
  ts?: string;
  ticket?: number;
  branch?: string;
  phase?: string;
  status?: string;
  [k: string]: unknown;
}

export interface StaleClaim {
  ticket: number;
  branch: string;
}

/**
 * 单票是否需要回滚认领（纯函数，#1259 补遗判定链）：
 * 1. 该票最后事件 = claim/claimed（此后无任何事件）；
 * 2. 无 implementer started 事件（反向判据：含 orphan 工蜂仍在跑的残局 → 不回滚）；
 * 3. claim 事件年龄 ≥ minAgeMs（主路径传 0 立即回滚；看门狗传 15min）。
 */
export function needsRollback(
  events: WorkerEvent[],
  ticket: number,
  opts: { minAgeMs?: number; now?: number } = {},
): boolean {
  const ticketEvents = events.filter((e) => e.ticket === ticket);
  if (ticketEvents.length === 0) return false;
  const last = ticketEvents[ticketEvents.length - 1]!;
  if (last.phase !== "claim" || last.status !== "claimed") return false;
  if (ticketEvents.some((e) => e.phase === "implementer" && e.status === "started")) return false;
  const minAgeMs = opts.minAgeMs ?? 0;
  if (minAgeMs <= 0) return true;
  const ts = Date.parse(last.ts ?? "");
  // ts 缺失/不可解析：无 running main.mts 时按陈旧处理（回滚），防卡死。
  if (Number.isNaN(ts)) return true;
  return (opts.now ?? Date.now()) - ts >= minAgeMs;
}

/** 找出所有 stale-claimed 票（纯函数），返回 {ticket, branch}
 *  （branch 取 claim 事件记录，缺省按确定性分支名 sandcastle/issue-<号> 补齐）。 */
export function findStaleClaims(
  events: WorkerEvent[],
  opts: { minAgeMs?: number; now?: number } = {},
): StaleClaim[] {
  const tickets = new Set<number>();
  for (const e of events) {
    if (typeof e.ticket === "number") tickets.add(e.ticket);
  }
  const result: StaleClaim[] = [];
  for (const ticket of tickets) {
    if (!needsRollback(events, ticket, opts)) continue;
    const claimEvent = events
      .filter((e) => e.ticket === ticket && e.phase === "claim" && e.status === "claimed")
      .at(-1);
    result.push({ ticket, branch: claimEvent?.branch ?? `sandcastle/issue-${ticket}` });
  }
  return result;
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

/** 专用 automation host 在 spawn 前快进到 origin/main；非 main/脏树/网络失败一律 fail-closed。 */
function refreshHostMain(): boolean {
  if (process.env.SANDCASTLE_HOST_AUTO_UPDATE !== "1") return true;
  try {
    const branch = execSync("git branch --show-current", { encoding: "utf8", env: GH_CLEAN_ENV }).trim();
    const dirty = execSync("git status --porcelain", { encoding: "utf8", env: GH_CLEAN_ENV }).trim();
    if (branch !== "main" || dirty) {
      console.error(`[claim-loop] automation host 非干净 main（branch=${branch || "HEAD"}, dirty=${Boolean(dirty)}）→ fail-closed`);
      return false;
    }
    execSync("git fetch origin main --quiet", { stdio: "inherit", env: GH_CLEAN_ENV });
    execSync("git reset --hard origin/main --quiet", { stdio: "inherit", env: GH_CLEAN_ENV });
    return true;
  } catch (e) {
    console.error(`[claim-loop] automation host 同步 origin/main 失败 → fail-closed：${String(e).slice(0, 300)}`);
    return false;
  }
}

// ── #1259：workers.jsonl 读写 + 认领回滚（与 main.mts logWorker 同目录、同 JSONL） ──
const WORKERS_LOG_PATH = ".sandcastle/logs/workers.jsonl";

function readWorkerEvents(): WorkerEvent[] {
  try {
    const raw = readFileSync(WORKERS_LOG_PATH, "utf8");
    return raw
      .split("\n")
      .filter((l) => l.trim().length > 0)
      .map((l) => {
        try {
          return JSON.parse(l) as WorkerEvent;
        } catch {
          return {} as WorkerEvent;
        }
      });
  } catch {
    return [];
  }
}

function appendWorkerEvent(e: Record<string, unknown>): void {
  mkdirSync(".sandcastle/logs", { recursive: true });
  appendFileSync(WORKERS_LOG_PATH, JSON.stringify({ ts: new Date().toISOString(), ...e }) + "\n");
}

/** 认领回滚动作（与 #1007 摘标/assign 对称）：remove-assignee @me + add-label sandcastle，
 *  workers.jsonl 追加 {phase:"claim", status:"rolled_back"}。失败不抛出（记日志，下轮再试）。 */
function rollbackClaim(ticket: number, branch: string): void {
  try {
    execSync(`gh issue edit ${ticket} --remove-assignee @me --add-label sandcastle`, { env: GH_CLEAN_ENV });
    appendWorkerEvent({ ticket, branch, phase: "claim", status: "rolled_back" });
    console.log(`[claim-loop] #${ticket} 认领回滚：unassigned + sandcastle 标签恢复，下轮可重拾。`);
  } catch (e) {
    console.error(`[claim-loop] #${ticket} 认领回滚失败（人工兜底：解除认领 + 重新入队）：${String(e).slice(0, 300)}`);
  }
}

/** 扫描 workers.jsonl 中所有 stale-claimed 票并回滚（dryRun 只打印）。
 *  minAgeMs>0 仅回滚超时的陈旧认领（看门狗）；minAgeMs=0 立即回滚（主路径：main.mts 已确认退出失败）。 */
function rollbackStuckClaims(opts: { dryRun: boolean; minAgeMs: number }): void {
  const stale = findStaleClaims(readWorkerEvents(), { minAgeMs: opts.minAgeMs });
  if (stale.length === 0) return;
  for (const s of stale) {
    if (opts.dryRun) {
      console.log(`[claim-loop] [dry-run] 将回滚 stale-claimed #${s.ticket}（unassign @me + 恢复 sandcastle 标签）`);
      continue;
    }
    rollbackClaim(s.ticket, s.branch);
  }
  if (!opts.dryRun) {
    console.log(`[claim-loop] 本轮回滚 ${stale.length} 张 stale-claimed 票。`);
  }
}

/** spawn main.mts 并等它收尾（单线程，等待期间不进入下一轮判定）。
 *  #1259：区分「spawn 失败（未启动，无认领）」与「被信号杀（code=null，可能已认领）」——
 *  后者按失败处理，触发收尾回滚。 */
interface MainMtsExit {
  code: number | null;
  signal: NodeJS.Signals | null;
  spawnFailed: boolean;
}

function runMainMts(): Promise<MainMtsExit> {
  return new Promise((resolve) => {
    const child = spawn("npx", ["tsx", ".sandcastle/main.mts"], {
      cwd: process.cwd(),
      env: GH_CLEAN_ENV,
      stdio: "inherit",
    });
    child.on("error", (e) => {
      console.error(`[claim-loop] spawn main.mts 失败：${e.message}`);
      resolve({ code: null, signal: null, spawnFailed: true });
    });
    child.on("exit", (code, signal) => resolve({ code, signal, spawnFailed: false }));
  });
}

// ── 单轮 ──────────────────────────────────────────────────────────────────────

async function runOnce(cfg: { dryRun: boolean }): Promise<void> {
  if (isMainMtsRunning()) {
    console.log("[claim-loop] main.mts running 实例在跑，本轮跳过（不并发多实例）");
    return;
  }
  // #1259 看门狗：无 running main.mts 时，每轮附带清掉超时（>15min）无后续事件的 stale-claimed 票。
  // 覆盖「claim-loop 与 main.mts 同时失联」盲区；不新增进程。
  rollbackStuckClaims({ dryRun: cfg.dryRun, minAgeMs: CLAIM_ROLLBACK_WATCHDOG_MS });
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
  if (!refreshHostMain()) return;
  console.log(`[claim-loop] frontier 非空且无 running 实例（可拾取：${runnable}）→ spawn main.mts`);
  const exit = await runMainMts();
  // #1259 主路径：main.mts 已退出且失败（exit≠0 或被信号杀）→ 立即回滚 stale-claimed 票
  // （判定链：最后事件=claim 且无 implementer started；反向判据：有 implementer started 不回滚）。
  const failed = !exit.spawnFailed && (exit.code === null || exit.code !== 0);
  const exitLabel = exit.spawnFailed ? "spawn 失败" : exit.code === null ? `signal ${exit.signal}` : `exit=${exit.code}`;
  console.log(`[claim-loop] main.mts 收尾 ${exitLabel}，下一轮再查`);
  if (failed) {
    rollbackStuckClaims({ dryRun: false, minAgeMs: 0 });
  }
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
