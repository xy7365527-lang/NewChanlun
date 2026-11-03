import { z } from "zod";
import { claimIssue, QUEUE_ARGS, queueToJson, renderPlanPrompt, validatePlan, type HostGh } from "./swarm-prompts.mts";

const queueIssueSchema = z.object({
  number: z.number().int().positive(), title: z.string(), labels: z.array(z.string()), body: z.string(),
});
const foldedPlanSchema = z.object({
  queue: z.array(queueIssueSchema),
  issues: z.array(z.object({ id: z.string(), title: z.string(), branch: z.string() })),
});
export type PlanningQueue = z.infer<typeof queueIssueSchema>[];
export type FoldedPlan = z.infer<typeof foldedPlanSchema>;

export function fetchPlanningQueue(gh: HostGh): PlanningQueue {
  const rows = z.array(z.object({
    number: z.number().int().positive(), title: z.string(), body: z.string(),
    labels: z.array(z.object({ name: z.string() })), assignees: z.array(z.unknown()),
  })).parse(JSON.parse(gh(QUEUE_ARGS)));
  return rows.filter(row => row.assignees.length === 0).map(row => ({
    number: row.number, title: row.title, body: row.body, labels: row.labels.map(label => label.name),
  }));
}

export function prepareSwarmPlan(gh: HostGh) {
  const queue = fetchPlanningQueue(gh);
  return { queue, prompt: renderPlanPrompt(queueToJson(queue)) };
}

export function parseFoldedPlan(input: unknown): FoldedPlan {
  return foldedPlanSchema.parse(input);
}

export function validateFoldedPlan(plan: FoldedPlan, freshQueue: PlanningQueue): void {
  const snapshot = (queue: PlanningQueue) => JSON.stringify([...queue].sort((a, b) => a.number - b.number)
    .map(({ number, title, body, labels }) => [number, title, body, [...labels].sort()]));
  if (snapshot(plan.queue) !== snapshot(freshQueue)) {
    throw new Error("主控计划快照已变化，请重新取队列并判断，未认领任何票");
  }
  validatePlan(plan.issues, freshQueue);
}

export async function consumeSwarmPlan(
  input: unknown, gh: HostGh, admit: (claim: () => void) => Promise<void>,
): Promise<FoldedPlan["issues"]> {
  const plan = parseFoldedPlan(input);
  if (plan.issues.length === 0) {
    validateFoldedPlan(plan, fetchPlanningQueue(gh));
  } else {
    await admit(() => {
      validateFoldedPlan(plan, fetchPlanningQueue(gh));
      for (const issue of plan.issues) claimIssue(gh, issue.id);
    });
  }
  return plan.issues;
}
