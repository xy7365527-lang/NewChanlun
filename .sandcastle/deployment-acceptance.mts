/** #1867 冻结部署验收专用入口。
 *
 *   npx --no-install tsx .sandcastle/deployment-acceptance.mts run --ticket N --head <部署 SHA> --nonce <本轮 nonce> --row 13|14|15
 *   npx --no-install tsx .sandcastle/deployment-acceptance.mts status --ticket N --head <部署 SHA> --nonce <本轮 nonce>
 *
 * 不读业务队列、不 claim、不 push、不合 main、不关票、不写业务 workers.jsonl、不调用 gh 写操作。 */
import type { SandboxProvider } from "@ai-hero/sandcastle";
import { execFileSync } from "node:child_process";
import { appendFileSync, existsSync, mkdirSync, readFileSync, realpathSync } from "node:fs";
import { dirname, join } from "node:path";
import { pathToFileURL } from "node:url";
import {
  ROUND_FILES, MERGE_ROWS, branchExists, frozenRowAdmission, inspectFrozenRound, loadCandidate, loadFrozenRound, loadMergeAdmission, loadMergePlan, loadMergeResult,
  parseSelection, publishCandidate, roundClaims, rowConsumed, swarmBinding, ticketBranch, ticketWorktree,
  type DeploymentSelection, type FrozenRound, type MergeRow, type RoundContext,
} from "./docker-acceptance.mts";
import { DrainFrozenError, withFileLock, withFrozenAdmission } from "./drain-control.mts";
import { assertContainerIsolationContract } from "./docker-isolation.mts";
import { assertDockerCliProxyReachable, swarmSandbox } from "./swarm-route.mts";
import { runSingleSwarmTicket, type AgentRunner } from "./swarm-ticket.mts";
import { assertInitClean, createSwarmSandboxLocked, repoInitLock } from "./worktree-init.mts";
import { runDeploymentSwarmMerge, type DeploymentMergeDeps } from "./swarm-merge.mts";
import { hostGh, type HostGh } from "./swarm-prompts.mts";
import type { verifyManagedRust } from "./managed-commit.mts";
import type { IssueFetcher } from "./review-context.mts";

export type DeploymentOutcome =
  | { row: "13"; state: "candidate_pending"; head: string; evidence: string }
  | { row: MergeRow; state: "merge_done"; head: string; evidence: string };

export interface DeploymentDeps extends Omit<DeploymentMergeDeps, "logWorker" | "provider" | "barrier"> {
  provider?: (imageId: string) => SandboxProvider;
  mergeProvider?: DeploymentMergeDeps["provider"];
  runAgent?: AgentRunner;
  gh?: HostGh;
  fetchIssue?: IssueFetcher;
  verify?: typeof verifyManagedRust;
  barrier?: (point: string) => Promise<void> | void;
}

const GH_ENV = { ...process.env, NO_COLOR: "1", CLICOLOR: "0", FORCE_COLOR: "0", CLICOLOR_FORCE: "0", GH_REPO: "Newchanlun/NewChanlun" };
function ticketOnlyGh(ticket: number, gh: HostGh = hostGh(GH_ENV)): HostGh {
  return (args) => {
    if (args[0] !== "issue" || args[1] !== "view" || args[2] !== String(ticket)) throw new Error(`[FAIL-loud #1867] 冻结验收只允许读取专票 #${ticket}，拒绝 gh ${args.slice(0, 3).join(" ")}`);
    return gh(args);
  };
}
function roundChannels(round: FrozenRound) {
  const log = join(round.roundDir, ROUND_FILES.log);
  mkdirSync(dirname(log), { recursive: true, mode: 0o700 });
  const notes = join(round.roundDir, ROUND_FILES.notes);
  return {
    log,
    logWorker: (e: Record<string, unknown>) => appendFileSync(log, JSON.stringify({ ts: new Date().toISOString(), round: round.receipt.nonce, ...e }) + "\n", { mode: 0o600 }),
    notify: (issue: number, body: string) => appendFileSync(notes, JSON.stringify({ ts: new Date().toISOString(), issue, body }) + "\n", { mode: 0o600 }),
  };
}

async function runRow13(sel: DeploymentSelection, d: DeploymentDeps): Promise<DeploymentOutcome> {
  const ctx = d.ctx ?? {};
  const round = loadFrozenRound(sel, ctx);
  const { ticket } = round;
  const repo = round.purpose.repo;
  const branch = ticketBranch(ticket);
  const cwd = ticketWorktree(repo, ticket);
  const terminal = (r: FrozenRound) => {
    if (loadCandidate(r)) throw new Error("第 13 行已有本轮候选结果，拒绝终态重放；查看用 status");
    if (rowConsumed(r, "13")) throw new Error("第 13 行是一次性的，已被占用；中断后须由主控退役并重新签发 receipt，不允许同票重跑冒充恢复");
  };
  terminal(round);
  if (branchExists(repo, branch) || existsSync(cwd)) throw new Error(`专票分支 ${branch} 或工位已存在，不是本轮新候选，拒绝复用`);
  const channels = roundChannels(round);
  const gh = ticketOnlyGh(ticket, d.gh);
  const issueJson: IssueFetcher = d.fetchIssue ?? ((n) => ticketOnlyGh(ticket)(["issue", "view", String(n), "--json", "number,title,body,labels,comments"]));
  return withFileLock(join(round.roundDir, ROUND_FILES.runLock("13")), async () => {
    const result = await runSingleSwarmTicket({
      issue: ticket, branch, baseRef: round.receipt.head, reviewBase: () => round.receipt.head,
      launch: () => withFrozenAdmission(() => withFileLock(repoInitLock(repo), async () => {
        terminal(loadFrozenRound(sel, ctx));
        assertInitClean({ repo, branch, fresh: cwd });
        await d.barrier?.("row13-after-verify");
        assertContainerIsolationContract(round.receipt.imageId, d.probe);
        (d.modelProbe ?? assertDockerCliProxyReachable)(round.receipt.imageId);
        const launchRound = loadFrozenRound(sel, ctx);
        terminal(launchRound);
        await d.barrier?.("row13-before-consume");
        frozenRowAdmission(launchRound.receipt.image, ctx, "13", swarmBinding(ticket)).consume();
        channels.logWorker({ ticket, branch, phase: "admission", status: "consumed", row: "13", head: launchRound.receipt.head });
        await d.barrier?.("row13-after-consume");
        const sandbox = await createSwarmSandboxLocked({ repo, branch, baseBranch: launchRound.receipt.head, provider: (d.provider ?? swarmSandbox)(launchRound.receipt.imageId), fresh: cwd });
        if (realpathSync(sandbox.worktreePath) !== cwd) throw new Error(`实际工位 ${sandbox.worktreePath} 不是预绑定 ${cwd}`);
        return sandbox;
      }), { generation: round.purpose.generation, control: round.control, root: d.drainRoot }),
    }, { logWorker: channels.logWorker, notify: channels.notify, gh, fetchIssue: issueJson, verify: d.verify, runAgent: d.runAgent });
    if (!result.commits.length || !result.verification) throw new Error("第 13 行没有受管提交产出，不是候选；used-13 保留");
    const head = result.verification.head;
    const git = (args: string[]) => execFileSync("git", ["-C", repo, ...args], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
    if (git(["rev-parse", "--verify", "-q", `refs/heads/${branch}`]) !== head) throw new Error("候选分支不在已验证 head");
    try { git(["merge-base", "--is-ancestor", round.receipt.head, head]); } catch { throw new Error("候选不是从本轮部署 head 派生"); }
    publishCandidate(round, { head, commits: result.commits, verification: { head, status: result.verification.status, ...(result.verification.evidencePath ? { evidencePath: result.verification.evidencePath } : {}) } });
    channels.logWorker({ ticket, branch, phase: "candidate", status: "pending", head });
    return { row: "13" as const, state: "candidate_pending" as const, head, evidence: join(round.roundDir, ROUND_FILES.candidate) };
  }, { nonblock: true });
}

export async function runDeploymentAcceptance(sel: DeploymentSelection, d: DeploymentDeps = {}): Promise<DeploymentOutcome> {
  if (sel.row === "13") return runRow13(sel, d);
  const row = sel.row;
  const round = loadFrozenRound(sel, d.ctx ?? {});
  const result = await runDeploymentSwarmMerge({ ...sel, row }, {
    ctx: d.ctx, drainRoot: d.drainRoot, probe: d.probe, modelProbe: d.modelProbe, provider: d.mergeProvider, runConflictAgent: d.runConflictAgent, docker: d.docker,
    barrier: d.barrier, logWorker: roundChannels(round).logWorker,
  });
  return { row, state: "merge_done", head: result.head, evidence: join(round.roundDir, ROUND_FILES.result(row)) };
}

export function deploymentStatus(sel: Omit<DeploymentSelection, "row">, ctx: Pick<RoundContext, "dir" | "roots"> = {}) {
  const round = inspectFrozenRound(sel, ctx);
  const plan = loadMergePlan(round);
  const checkpointPhase = (path: string) => { try { return (JSON.parse(readFileSync(path, "utf8")) as { phase?: string }).phase ?? null; } catch { return null; } };
  return {
    roundDir: round.roundDir, receipt: { nonce: round.receipt.nonce, head: round.receipt.head, ticket: round.ticket, generation: round.purpose.generation, expiresAt: round.receipt.expiresAt, partner: round.purpose.partner.sha },
    claims: roundClaims(round),
    candidate: loadCandidate(round), log: join(round.roundDir, ROUND_FILES.log), notes: join(round.roundDir, ROUND_FILES.notes),
    merges: plan ? Object.fromEntries(MERGE_ROWS.map((row) => {
      const run = plan.runs[row];
      return [row, { runId: run.manifest.runId, instance: run.instance, cwd: run.cwd, checkpoint: run.checkpoint, evidenceDir: run.evidenceDir,
        admitted: loadMergeAdmission(round, run), phase: checkpointPhase(run.checkpoint), result: loadMergeResult(round, row) }];
    })) : null,
  };
}

export function parseCli(argv: readonly string[]): { cmd: "run" | "status"; sel: DeploymentSelection } {
  const [cmd, ...rest] = argv;
  if (cmd !== "run" && cmd !== "status") throw new UsageError("命令：run | status");
  const allowed = cmd === "run" ? ["ticket", "head", "nonce", "row"] : ["ticket", "head", "nonce"];
  const got: Record<string, string> = {};
  for (let i = 0; i < rest.length; i += 2) {
    const key = rest[i]!.startsWith("--") ? rest[i]!.slice(2) : "";
    if (!allowed.includes(key) || rest[i + 1] === undefined || key in got) throw new UsageError(`不接受的参数 ${rest[i]}（只接受 ${allowed.map((k) => `--${k}`).join(" ")}）`);
    got[key] = rest[i + 1]!;
  }
  try { return { cmd, sel: parseSelection({ ...got, row: cmd === "run" ? got.row : "13" }) }; }
  catch (error) { throw new UsageError(error instanceof Error ? error.message : String(error)); }
}
export class UsageError extends Error {}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    const { cmd, sel } = parseCli(process.argv.slice(2));
    const out = cmd === "run" ? await runDeploymentAcceptance(sel) : deploymentStatus(sel);
    console.log(JSON.stringify(out, null, 2));
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = error instanceof DrainFrozenError ? 75 : error instanceof UsageError ? 64 : 1;
  }
}
