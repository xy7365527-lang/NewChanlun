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

/** namespace 恢复也不等于多票 merge 合同获批；在 planner/claim 前明确停下。 */
export function assertSwarmMergeContract(): void {
  throw new Error("[FAIL-loud #1617] Codex swarm 多票 merger 受管提交合同未批准/未实现；不运行 planner、不认领。单票 main.mts 合同不受此门影响。");
}
