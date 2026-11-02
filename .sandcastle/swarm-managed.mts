/** #1617：swarm 单票作者/独审复用已批准宿主封包；多票 merger 不在此合同内。 */
import {
  beginManagedStage, finishManagedStage, runManagedReview, verifyManagedRust, verificationPrompt,
  type RustVerification,
} from "./managed-commit.mts";
import {
  runGuardedReview, prepareFinalReviewPrompt,
  type BuildReviewPromptOptions, type ReviewFailureChannels,
} from "./review-context.mts";

export interface SwarmTicketOptions {
  cwd: string;
  branch: string;
  issue: number;
  implement: () => Promise<string>;
  review: (prompt: string) => Promise<string>;
  reviewOptions: Omit<BuildReviewPromptOptions, "branch" | "issue">;
  channels: ReviewFailureChannels;
  verify?: typeof verifyManagedRust;
}

/** 返回的是受管宿主提交，不依赖 SDK 对工蜂 git commit 的观察。 */
export async function runManagedSwarmTicket(opts: SwarmTicketOptions): Promise<{
  commits: string[]; verification?: RustVerification;
}> {
  const { cwd, branch, issue, channels } = opts;
  const verify = opts.verify ?? verifyManagedRust;
  const stage = beginManagedStage(cwd, branch, issue, "implementer");
  const sealed = finishManagedStage(stage, await opts.implement());
  if (!sealed.committed) return { commits: [] };
  const initial = verify(stage, sealed);
  const verification = await runGuardedReview(
    { ...opts.reviewOptions, branch, issue }, channels,
    async ({ prompt, base }) => runManagedReview(cwd, branch, issue, initial, async (reviewStage, evidence) => {
      const final = prepareFinalReviewPrompt(
        prompt + `\n本次评审必须读取精确候选 commit ${reviewStage.head}。\n${verificationPrompt(evidence)}`,
        base, { issue, branch, maxTokens: opts.reviewOptions.maxTokens }, channels,
      );
      channels.logWorker({ ticket: issue, branch, phase: "reviewer", status: "started", head: reviewStage.head, prompt_est_tokens: final.estimatedTokens });
      return opts.review(final.prompt);
    }, verify),
  );
  channels.logWorker({ ticket: issue, branch, phase: "reviewer", status: "completed", head: verification.head, verification });
  return { commits: [...new Set([sealed.head, verification.head])], verification };
}
