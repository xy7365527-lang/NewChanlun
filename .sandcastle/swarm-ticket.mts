/** #1867：单票完整生命周期（普通 swarm 与冻结验收第 13 行共用）与仓库共享工位初始化。
 * 工位初始化在仓库级短锁内：Sandcastle createSandbox 会 prune 未登记的 .sandcastle/worktrees 目录，初始化失败还会
 * `git worktree remove --force` 新工位；本模块先拒绝存在孤儿目录的仓库，再在 provider 初始化失败且工位有未知写入时
 * 用 `git worktree lock` 保留现场（单个 --force 不移除已锁工位）。 */
import * as sandcastle from "@ai-hero/sandcastle";
import type { BindMountCreateOptions, BindMountSandboxHandle, SandboxProvider } from "@ai-hero/sandcastle";
import { execFileSync } from "node:child_process";
import { existsSync, readdirSync, realpathSync, statSync } from "node:fs";
import { isAbsolute, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { withFileLock } from "./drain-control.mts";
import { realpathLoose } from "./acceptance-dir.mts";
import { runManagedSwarmTicket } from "./swarm-managed.mts";
import { runImplementer, type HostGh } from "./swarm-prompts.mts";
import { swarmAgent } from "./swarm-route.mts";
import { hostGit, readReviewTemplate, type GitRunner, type IssueFetcher, type ReviewFailureChannels } from "./review-context.mts";
import type { verifyManagedRust } from "./managed-commit.mts";

export type SwarmSandbox = Awaited<ReturnType<typeof sandcastle.createSandbox>>;
const git = (cwd: string, args: string[]) => execFileSync("git", ["-C", cwd, ...args], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();

/** 同一仓库（含全部 linked worktree）共用的初始化锁：放在 git common dir，任何检出建工位都排到同一把锁上。 */
export function repoInitLock(repo: string): string {
  const common = git(repo, ["rev-parse", "--git-common-dir"]);
  return join(isAbsolute(common) ? common : resolve(repo, common), "sandcastle-worktree-init.lock");
}

/** createSandbox 的 pruneStale 会删除的目录：.sandcastle/worktrees 下未登记为活 worktree、或登记了却缺 .git 指针（prune 会先注销）的目录。 */
export function orphanWorktrees(repo: string): string[] {
  const dir = join(repo, ".sandcastle", "worktrees");
  if (!existsSync(dir)) return [];
  const active = new Set(git(repo, ["worktree", "list", "--porcelain"]).split("\n").filter((l) => l.startsWith("worktree ")).map((l) => realpathLoose(l.slice(9))));
  const real = realpathSync(dir);
  return readdirSync(real).map((n) => join(real, n)).filter((p) => {
    let isDir = false;
    try { isDir = statSync(p).isDirectory(); } catch { return false; }
    return isDir && (!active.has(realpathLoose(p)) || !existsSync(join(p, ".git")));
  });
}

type BindMountShape = { tag: string; create: (options: BindMountCreateOptions) => Promise<BindMountSandboxHandle> };
/** 包住 bind-mount provider 的 create（Sandcastle 公开类型不暴露 create）；其余字段原样保留。 */
export function wrapBindMountCreate(provider: SandboxProvider, wrap: (create: () => Promise<BindMountSandboxHandle>, options: BindMountCreateOptions) => Promise<BindMountSandboxHandle>): SandboxProvider {
  const shape = provider as unknown as BindMountShape;
  if (shape.tag !== "bind-mount") throw new Error("[FAIL-loud #1867] 只支持 bind-mount provider");
  return { ...shape, create: (options: BindMountCreateOptions) => wrap(() => shape.create.call(shape, options), options) } as unknown as SandboxProvider;
}

/** provider 初始化失败：工位里出现任何未知写入（含 untracked/ignored）或无法判定时，加 worktree lock 再抛出原错误；干净的新工位照常由库清理。 */
export function preserveFailedInit(provider: SandboxProvider): SandboxProvider {
  return wrapBindMountCreate(provider, async (create, options) => {
      try { return await create(); }
      catch (error) {
        let status: string;
        try { status = git(options.worktreePath, ["status", "--porcelain=v1", "--untracked-files=all", "--ignored"]); } catch { status = "unknown"; }
        if (status) {
          try { git(options.worktreePath, ["worktree", "lock", "--reason", `sandcastle-1867-preserved: provider init failed with unknown writes: ${String(error).slice(0, 160)}`, options.worktreePath]); }
          catch (lockError) { throw new Error(`provider 初始化失败且无法锁定保留工位 ${options.worktreePath}：${String(lockError).slice(0, 160)}；原错误：${String(error).slice(0, 160)}`); }
        }
        throw error;
      }
  });
}

export interface SandboxInit { repo: string; branch: string; baseBranch: string; provider: SandboxProvider; fresh?: string }
/** 初始化前置检查（须在仓库初始化锁内）：无孤儿目录；fresh＝预绑定工位路径时，分支与工位都必须不存在，杜绝复用别轮现场。 */
export function assertInitClean(o: Pick<SandboxInit, "repo" | "branch" | "fresh">): void {
  const orphans = orphanWorktrees(o.repo);
  if (orphans.length) throw new Error(`[FAIL-loud #1867] ${o.repo}/.sandcastle/worktrees 有未登记目录，Sandcastle 会自动删除，拒绝初始化并保留：${orphans.join(", ")}`);
  if (o.fresh !== undefined) {
    let exists = true;
    try { git(o.repo, ["rev-parse", "--verify", "-q", `refs/heads/${o.branch}`]); } catch { exists = false; }
    if (exists || existsSync(o.fresh)) throw new Error(`[FAIL-loud #1867] 分支 ${o.branch} 或工位 ${o.fresh} 已存在，拒绝复用`);
  }
}
/** 调用方已持仓库初始化锁时使用。 */
export async function createSwarmSandboxLocked(o: SandboxInit): Promise<SwarmSandbox> {
  assertInitClean(o);
  return sandcastle.createSandbox({ cwd: o.repo, branch: o.branch, baseBranch: o.baseBranch, sandbox: preserveFailedInit(o.provider) });
}
export async function createSwarmSandbox(o: SandboxInit): Promise<SwarmSandbox> {
  return withFileLock(repoInitLock(o.repo), () => createSwarmSandboxLocked(o));
}

export type AgentRole = "implementer" | "reviewer";
export interface AgentRun { name: string; maxIterations: number; prompt: string; resumeSession?: string }
export type AgentRunner = (sandbox: SwarmSandbox, role: AgentRole, o: AgentRun) => Promise<{ stdout: string; iterations: Array<{ sessionId?: string }> }>;
export interface TicketChannels extends ReviewFailureChannels {
  /** 读取该票正文（实施 prompt 用）；冻结验收只读专票。 */
  gh: HostGh;
  fetchIssue: IssueFetcher;
  verify?: typeof verifyManagedRust;
  /** 仅测试：替代容器内 agent 执行；生产缺省为 sandbox.run + 每角色独立 swarmAgent。 */
  runAgent?: AgentRunner;
}
export interface SwarmTicket { issue: number; branch: string; baseRef: string; reviewBase: (git: GitRunner) => string; launch: () => Promise<SwarmSandbox> }
const REVIEW_TEMPLATE = fileURLToPath(new URL("./codex-review-prompt.md", import.meta.url));

/** 启动（由调用方在正确锁内完成）→ 独立作者 → 宿主封包 → 精确 head 验证 → 独立 reviewer → finally close。 */
export async function runSingleSwarmTicket(t: SwarmTicket, channels: TicketChannels) {
  const sandbox = await t.launch();
  try {
    const run: AgentRunner = channels.runAgent ?? (() => {
      const agents = { implementer: swarmAgent(), reviewer: swarmAgent() };
      return (sb, role, o) => sb.run({ ...o, agent: agents[role] });
    })();
    const git = hostGit(sandbox.worktreePath);
    let reviewSession: string | undefined;
    return await runManagedSwarmTicket({
      cwd: sandbox.worktreePath, branch: t.branch, issue: t.issue,
      implement: () => runImplementer(String(t.issue), {
        gh: channels.gh, agent: undefined, notify: channels.notify,
        run: (o) => run(sandbox, "implementer", { name: o.name, maxIterations: o.maxIterations, prompt: o.prompt }),
      }),
      review: async (prompt) => {
        const result = await run(sandbox, "reviewer", { name: "reviewer", maxIterations: 1, prompt, ...(reviewSession ? { resumeSession: reviewSession } : {}) });
        reviewSession = result.iterations.at(-1)?.sessionId;
        return result.stdout;
      },
      reviewOptions: { template: readReviewTemplate(REVIEW_TEMPLATE), target: t.baseRef, resolveBase: () => t.reviewBase(git), git, fetchIssue: channels.fetchIssue },
      channels: { logWorker: channels.logWorker, notify: channels.notify },
      ...(channels.verify ? { verify: channels.verify } : {}),
    });
  } finally {
    await sandbox.close();
  }
}
