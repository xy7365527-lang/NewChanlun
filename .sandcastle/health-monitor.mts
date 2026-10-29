/**
 * health-monitor —— graph engineering 观测面（#1547，正本 .sandcastle/docs/graph-engineering.md）。
 *
 * 五项检查：wayfinder_engine 日志新鲜度、claim-loop 存活与宿主脏树 fail-closed、
 * stale claim（workers.jsonl 非终态且 >48h 无后续事件）、外盘可用空间、CLIProxy 8318 可达。
 *
 * 默认只读：写 .sandcastle/logs/health-report.json 后退出。
 * --heal：wayfinder/claim-loop 停摆 → launchctl kickstart -k，服务未加载则 bootstrap
 * .sandcastle/launchd/<label>.plist 并复判运行态（#1678）；stale claim → 与 claim-loop
 * rollbackClaim 同款回滚（unassign @me + 恢复 sandcastle 标签 + workers.jsonl 记 rolled_back）。
 * 宿主脏树不自动清理（需人工或 PR 合入后 reset），外盘只报告不删文件。
 *
 * 跑法：npx tsx .sandcastle/health-monitor.mts [--heal] [--report <path>]
 */
import { drainRequested, withLegacyAdmission, DrainFrozenError } from "./drain-control.mts";
import { execFileSync } from "node:child_process";
import { appendFileSync, existsSync, mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";

// ── 阈值与路径（环境变量可覆盖） ─────────────────────────────────────────────
export const WAYFINDER_STALE_MS = 10 * 60 * 1000;
export const CLAIM_STALE_MS = 48 * 60 * 60 * 1000;
export const DISK_MIN_GIB = Number(process.env.HEALTH_DISK_MIN_GIB ?? 100);

const MAIN_REPO = process.env.HEALTH_MAIN_REPO ?? "/Users/silencehan/Projects/NewChanlun";
const HOST_REPO = process.env.SANDCASTLE_HOST_REPO ?? "/Users/silencehan/Projects/NewChanlun-sandcastle-host";
const WAYFINDER_LOG = `${MAIN_REPO}/.sandcastle/logs/wayfinder-engine.stdout.log`;
const CLAIMER_STDOUT = `${HOST_REPO}/.sandcastle/logs/sandcastle-claimer.stdout.log`;
const CLAIMER_STDERR = `${HOST_REPO}/.sandcastle/logs/sandcastle-claimer.stderr.log`;
const WORKERS_LOG = `${HOST_REPO}/.sandcastle/logs/workers.jsonl`;
const DISK_PATH = process.env.HEALTH_DISK_PATH ?? "/Volumes/AgentStorage";
const CLIPROXY_URL = process.env.HEALTH_CLIPROXY_URL ?? "http://127.0.0.1:8318/v1/models";
const CLIPROXY_CONFIG =
  process.env.HEALTH_CLIPROXY_CONFIG ?? "/Users/silencehan/Library/Application Support/CLIProxyLocal/config.yaml";
const LABEL_WAYFINDER = "com.newchanlun.wayfinder-engine";
const LABEL_CLAIMER = "com.newchanlun.sandcastle-claimer";
const PROC_WAYFINDER = "scripts/[w]ayfinder_engine[.]mts";
const PROC_CLAIMER = "[.]sandcastle/[c]laim-loop[.]mts";

const GH_ENV: NodeJS.ProcessEnv = {
  ...process.env,
  NO_COLOR: "1",
  CLICOLOR: "0",
  FORCE_COLOR: "0",
  CLICOLOR_FORCE: "0",
  GH_REPO: process.env.GH_REPO ?? "xy7365527-lang/NewChanlun",
};

export type Status = "healthy" | "red";
export interface CheckResult {
  name: string;
  status: Status;
  message: string;
  [k: string]: unknown;
}

// ── 纯函数判定（health-monitor.test.ts 覆盖） ─────────────────────────────────

/** 日志新鲜度：mtime 缺失或距今超过阈值即 red。 */
export function judgeFreshness(mtimeMs: number | null, now: number, staleMs = WAYFINDER_STALE_MS): Status {
  if (mtimeMs === null) return "red";
  return now - mtimeMs > staleMs ? "red" : "healthy";
}

/**
 * claim-loop 状态：进程不在 → stopped；stderr 末行是 fail-closed 且比 stdout 新 → fail_closed；否则 ok。
 * stdout 每轮都写（「本轮无事」等），fail-closed 只写 stderr 并提前返回，因此「stderr 更新且末行 fail-closed」
 * 即当前仍卡在脏树门。
 */
export function judgeClaimLoop(input: {
  running: boolean;
  stderrTail: string;
  stderrMtimeMs: number | null;
  stdoutMtimeMs: number | null;
}): "ok" | "stopped" | "fail_closed" {
  if (!input.running) return "stopped";
  const lines = input.stderrTail.split("\n").map((l) => l.trim()).filter(Boolean);
  const last = lines.at(-1) ?? "";
  const newer = (input.stderrMtimeMs ?? 0) > (input.stdoutMtimeMs ?? 0);
  if (newer && last.includes("fail-closed")) return "fail_closed";
  return "ok";
}

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
  lastPhase: string;
  lastTs: string;
  ageHours: number;
}

const TERMINAL = new Set(["sandbox/closed", "claim/rolled_back", "implementer/no_commit"]);

/** stale claim：某票最后一条事件非终态，且距今 > staleMs。mainRunning 时一律不判（正在跑的不算僵尸）。 */
export function findStaleClaims(
  events: WorkerEvent[],
  opts: { now: number; staleMs?: number; mainRunning?: boolean },
): StaleClaim[] {
  if (opts.mainRunning) return [];
  const staleMs = opts.staleMs ?? CLAIM_STALE_MS;
  const last = new Map<number, WorkerEvent>();
  const branch = new Map<number, string>();
  for (const e of events) {
    if (typeof e.ticket !== "number") continue;
    last.set(e.ticket, e);
    if (e.phase === "claim" && e.status === "claimed" && typeof e.branch === "string") branch.set(e.ticket, e.branch);
  }
  const out: StaleClaim[] = [];
  for (const [ticket, e] of last) {
    if (TERMINAL.has(`${e.phase}/${e.status}`)) continue;
    const ts = Date.parse(e.ts ?? "");
    if (Number.isNaN(ts)) continue;
    const age = opts.now - ts;
    if (age <= staleMs) continue;
    out.push({
      ticket,
      branch: branch.get(ticket) ?? `sandcastle/issue-${ticket}`,
      lastPhase: `${e.phase}/${e.status}`,
      lastTs: e.ts ?? "",
      ageHours: Math.round(age / 3_600_000),
    });
  }
  return out.sort((a, b) => a.ticket - b.ticket);
}

/** df -k 输出 → 可用 GiB。解析失败返回 null。 */
export function parseDfAvailGiB(dfOutput: string): number | null {
  const lines = dfOutput.trim().split("\n");
  const cols = (lines.at(-1) ?? "").trim().split(/\s+/);
  const avail = Number(cols[3]);
  if (!Number.isFinite(avail)) return null;
  return Math.round((avail / 1024 / 1024) * 10) / 10;
}

export function judgeDisk(availGiB: number | null, minGiB = DISK_MIN_GIB): Status {
  return availGiB !== null && availGiB >= minGiB ? "healthy" : "red";
}

/** 从 CLIProxyLocal config.yaml 取 access.api-keys 首项（只在内存里用，不写日志）。 */
export function extractAccessKey(yaml: string): string | null {
  const m = yaml.match(/^access:\s*\n(?:[ \t]+.*\n)*?[ \t]+api-keys:\s*\n[ \t]+-\s*["']?([^"'\s]+)["']?/m);
  return m ? m[1]! : null;
}

export function overall(checks: CheckResult[]): Status {
  return checks.every((c) => c.status === "healthy") ? "healthy" : "red";
}

// ── 副作用层 ──────────────────────────────────────────────────────────────────

function mtime(path: string): number | null {
  try {
    return statSync(path).mtimeMs;
  } catch {
    return null;
  }
}

function tail(path: string, bytes = 4096): string {
  try {
    const buf = readFileSync(path);
    return buf.subarray(Math.max(0, buf.length - bytes)).toString("utf8");
  } catch {
    return "";
  }
}

function sh(cmd: string, args: string[], env: NodeJS.ProcessEnv = process.env): string {
  return execFileSync(cmd, args, { encoding: "utf8", env, stdio: ["ignore", "pipe", "pipe"] });
}

function pgrep(pattern: string): boolean {
  try {
    sh("pgrep", ["-f", pattern]);
    return true;
  } catch {
    return false;
  }
}

/** launchd 服务停摆自愈；run 失败即抛（同 execFileSync），isRunning 为加载后复判的运行态。 */
export interface HealDeps {
  run: (cmd: string, args: string[]) => string;
  isRunning: () => boolean;
}
export interface HealResult {
  status: Status;
  action: string;
}

function errText(e: unknown): string {
  const stderr = (e as { stderr?: unknown })?.stderr;
  return `${String(e)} ${typeof stderr === "string" ? stderr : ""}`.replace(/\s+/g, " ").trim().slice(0, 200);
}

/**
 * kickstart -k；若 launchd 报「服务不存在/未加载」（#1678，Could not find service）→ bootstrap 对应 plist
 * 再判一次运行态。其它 kickstart 错误只报告，不叠加重试。
 */
export function healService(label: string, plist: string, uid: string, deps: HealDeps): HealResult {
  try {
    deps.run("launchctl", ["kickstart", "-k", `gui/${uid}/${label}`]);
    return { status: deps.isRunning() ? "healthy" : "red", action: `已 kickstart ${label}` };
  } catch (e) {
    const msg = errText(e);
    if (!/Could not find service/i.test(msg)) return { status: "red", action: `kickstart ${label} 失败：${msg}` };
  }
  try {
    deps.run("launchctl", ["bootstrap", `gui/${uid}`, plist]);
  } catch (e) {
    return { status: "red", action: `${label} 未加载，bootstrap ${plist} 失败：${errText(e)}` };
  }
  return deps.isRunning()
    ? { status: "healthy", action: `${label} 未加载，已 bootstrap ${plist}，进程已起` }
    : { status: "red", action: `${label} 未加载，已 bootstrap ${plist}，但进程仍不在` };
}

/** 轮询进程出现（launchd 拉起有延迟），最多 waitMs。 */
function waitForProcess(pattern: string, waitMs = 15_000): boolean {
  const deadline = Date.now() + waitMs;
  for (;;) {
    if (pgrep(pattern)) return true;
    if (Date.now() >= deadline) return false;
    sh("sleep", ["1"]);
  }
}

function healLaunchd(label: string, processPattern: string): string {
  const uid = sh("id", ["-u"]).trim();
  const plist = `${HOST_REPO}/.sandcastle/launchd/${label}.plist`;
  const r = healService(label, plist, uid, { run: sh, isRunning: () => waitForProcess(processPattern) });
  return `${r.action}（复判：${r.status}）`;
}

function checkWayfinder(now: number): CheckResult {
  const m = mtime(WAYFINDER_LOG);
  const status = judgeFreshness(m, now);
  const ageSec = m === null ? null : Math.round((now - m) / 1000);
  return {
    name: "wayfinder_engine",
    status,
    logPath: WAYFINDER_LOG,
    lastActivitySec: ageSec,
    message: m === null ? "日志不存在" : status === "healthy" ? `日志 ${ageSec}s 前更新` : `日志 ${ageSec}s 未更新（>600s）`,
  };
}

function hostDirtyPaths(): string[] {
  try {
    return sh("git", ["-C", HOST_REPO, "status", "--porcelain"]).split("\n").filter(Boolean);
  } catch {
    return [];
  }
}

function checkClaimLoop(): CheckResult {
  const running = pgrep(PROC_CLAIMER);
  const state = judgeClaimLoop({
    running,
    stderrTail: tail(CLAIMER_STDERR),
    stderrMtimeMs: mtime(CLAIMER_STDERR),
    stdoutMtimeMs: mtime(CLAIMER_STDOUT),
  });
  const dirty = state === "fail_closed" ? hostDirtyPaths() : [];
  const message =
    state === "ok"
      ? "进程在跑，最近一轮未 fail-closed"
      : state === "stopped"
        ? "claim-loop 进程不在"
        : `宿主脏树 fail-closed，派工停摆（${dirty.length} 个改动路径，需经 PR 合入或人工清理宿主）`;
  return { name: "claim_loop", status: state === "ok" ? "healthy" : "red", state, running, hostRepo: HOST_REPO, dirtyPaths: dirty, message };
}

function readWorkerEvents(): WorkerEvent[] {
  try {
    return readFileSync(WORKERS_LOG, "utf8")
      .split("\n")
      .filter((l) => l.trim())
      .map((l) => {
        try {
          return JSON.parse(l) as WorkerEvent;
        } catch {
          return {};
        }
      });
  } catch {
    return [];
  }
}

interface IssueState {
  state: string;
  assignees: string[];
}

function issueState(n: number): IssueState | null {
  try {
    const j = JSON.parse(sh("gh", ["issue", "view", String(n), "--json", "state,assignees"], GH_ENV));
    return { state: j.state, assignees: (j.assignees ?? []).map((a: { login: string }) => a.login) };
  } catch {
    return null;
  }
}

function checkStaleClaims(now: number): { result: CheckResult; actionable: StaleClaim[] } {
  const mainRunning = pgrep("[.]sandcastle/[m]ain[.]mts");
  const stale = findStaleClaims(readWorkerEvents(), { now, mainRunning });
  // 只把「票仍 open 且仍有 assignee」的算真僵尸；已关或已释放的只是日志尾巴。
  const actionable: StaleClaim[] = [];
  const ignored: Array<{ ticket: number; reason: string }> = [];
  for (const s of stale) {
    const st = issueState(s.ticket);
    if (st === null) ignored.push({ ticket: s.ticket, reason: "gh 查询失败" });
    else if (st.state !== "OPEN") ignored.push({ ticket: s.ticket, reason: `票已 ${st.state}` });
    else if (st.assignees.length === 0) ignored.push({ ticket: s.ticket, reason: "已无 assignee" });
    else actionable.push(s);
  }
  return {
    actionable,
    result: {
      name: "stale_claims",
      status: actionable.length === 0 ? "healthy" : "red",
      count: actionable.length,
      stale: actionable,
      ignored,
      message: actionable.length === 0 ? "无 >48h 僵尸认领" : `${actionable.length} 张票认领后 >48h 无后续事件`,
    },
  };
}

function checkDisk(): CheckResult {
  let avail: number | null = null;
  try {
    avail = parseDfAvailGiB(sh("df", ["-k", DISK_PATH]));
  } catch {
    avail = null;
  }
  const status = judgeDisk(avail);
  return {
    name: "disk_space",
    status,
    path: DISK_PATH,
    availableGiB: avail,
    requiredGiB: DISK_MIN_GIB,
    message: avail === null ? `${DISK_PATH} 不可读` : `可用 ${avail} GiB（门 ${DISK_MIN_GIB} GiB）`,
  };
}

async function checkCliproxy(): Promise<CheckResult> {
  let key: string | null = process.env.HEALTH_CLIPROXY_KEY ?? null;
  if (!key && existsSync(CLIPROXY_CONFIG)) key = extractAccessKey(readFileSync(CLIPROXY_CONFIG, "utf8"));
  try {
    const res = await fetch(CLIPROXY_URL, {
      method: "GET",
      headers: key ? { Authorization: `Bearer ${key}` } : {},
      signal: AbortSignal.timeout(5000),
    });
    let models = 0;
    if (res.ok) {
      const j = (await res.json()) as { data?: unknown[] };
      models = Array.isArray(j.data) ? j.data.length : 0;
    }
    return {
      name: "cliproxy",
      status: res.ok ? "healthy" : "red",
      url: CLIPROXY_URL,
      httpStatus: res.status,
      models,
      message: res.ok ? `GET /v1/models 200，${models} 个模型` : `GET /v1/models 返回 ${res.status}`,
    };
  } catch (e) {
    return { name: "cliproxy", status: "red", url: CLIPROXY_URL, message: `不可达：${String(e).slice(0, 200)}` };
  }
}

function rollbackClaim(s: StaleClaim): string {
  try {
    sh("gh", ["issue", "edit", String(s.ticket), "--remove-assignee", "@me", "--add-label", "sandcastle"], GH_ENV);
    appendFileSync(
      WORKERS_LOG,
      JSON.stringify({ ts: new Date().toISOString(), ticket: s.ticket, branch: s.branch, phase: "claim", status: "rolled_back", by: "health-monitor" }) + "\n",
    );
    return `#${s.ticket} 认领回滚：unassigned + sandcastle 标签恢复`;
  } catch (e) {
    return `#${s.ticket} 认领回滚失败：${String(e).slice(0, 200)}`;
  }
}

async function main(): Promise<void> {
  const argv = process.argv.slice(2);
  const heal = argv.includes("--heal");
  const ri = argv.indexOf("--report");
  const reportPath = ri >= 0 && argv[ri + 1] ? argv[ri + 1]! : `${MAIN_REPO}/.sandcastle/logs/health-report.json`;
  const now = Date.now();

  const wayfinder = checkWayfinder(now);
  const claimLoop = checkClaimLoop();
  const stale = checkStaleClaims(now);
  const disk = checkDisk();
  const cliproxy = await checkCliproxy();
  const checks = [wayfinder, claimLoop, stale.result, disk, cliproxy];

  const actions: string[] = [];
  if (heal && drainRequested()) {
    actions.push("legacy drain：只报告，不重启服务、不回滚认领");
  } else if (heal) {
    try {
      await withLegacyAdmission(() => {
        if (wayfinder.status === "red") actions.push(healLaunchd(LABEL_WAYFINDER, PROC_WAYFINDER));
        if (claimLoop.state === "stopped") actions.push(healLaunchd(LABEL_CLAIMER, PROC_CLAIMER));
        if (claimLoop.state === "fail_closed") actions.push("claim_loop：宿主脏树不自动清理，留人工/PR");
        for (const s of stale.actionable) actions.push(rollbackClaim(s));
        if (disk.status === "red") actions.push("disk_space：只报告，不删除外盘文件");
      });
    } catch (error) {
      if (!(error instanceof DrainFrozenError)) throw error;
      actions.push("legacy drain：冻结已生效，未执行heal");
    }
  }

  const report = {
    timestamp: new Date(now).toISOString(),
    mode: heal ? "heal" : "read-only",
    overallStatus: overall(checks),
    checks,
    actions,
  };
  mkdirSync(dirname(reportPath), { recursive: true });
  writeFileSync(reportPath, JSON.stringify(report, null, 2) + "\n");
  console.log(`[health-monitor] ${report.overallStatus} → ${reportPath}`);
  for (const c of checks) console.log(`  ${c.status === "healthy" ? "ok " : "RED"} ${c.name}: ${c.message}`);
  for (const a of actions) console.log(`  action: ${a}`);
}

if ((import.meta as { main?: boolean }).main) {
  main().catch((e) => {
    console.error(`[health-monitor] 异常：${String(e).slice(0, 300)}`);
    process.exit(1);
  });
}
