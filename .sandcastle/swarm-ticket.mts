import { fileURLToPath } from "node:url";
import { runManagedSwarmTicket } from "./swarm-managed.mts";
import { runImplementer, type HostGh } from "./swarm-prompts.mts";
import { swarmAgent } from "./swarm-route.mts";
import { hostGit, readReviewTemplate, type GitRunner, type IssueFetcher, type ReviewFailureChannels } from "./review-context.mts";
import type { verifyManagedRust } from "./managed-commit.mts";
import type { SwarmSandbox } from "./worktree-init.mts";

export type AgentRole = "implementer" | "reviewer";
export interface AgentRun { name: string; maxIterations: number; prompt: string; resumeSession?: string }
export type AgentRunner = (sandbox: SwarmSandbox, role: AgentRole, o: AgentRun) => Promise<{ stdout: string; iterations: Array<{ sessionId?: string }> }>;
export interface TicketChannels extends ReviewFailureChannels {
  gh: HostGh;
  fetchIssue: IssueFetcher;
  verify?: typeof verifyManagedRust;
  runAgent?: AgentRunner;
}
export interface SwarmTicket { issue: number; branch: string; baseRef: string; reviewBase: (git: GitRunner) => string; launch: () => Promise<SwarmSandbox> }
const REVIEW_TEMPLATE = fileURLToPath(new URL("./codex-review-prompt.md", import.meta.url));

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
