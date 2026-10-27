/** #1742 U6：归并输入的来源身份。旧 sandcastle 单票输入保持原合同；DeepAgents 产物按 run 记录接入，不改名冒充。 */
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { basename, isAbsolute, join } from "node:path";

export interface Acceptance { head: string; status: "accepted"; reviewEvidence: string; verificationEvidence: string }
/** 原 #1701 合同：无 source 字段，分支固定 sandcastle/issue-N。 */
export interface SandcastleMergeInput { source?: undefined; issue: number; branch: string; sha: string; acceptance: Acceptance }
/** DeepAgents 执行器交付：分支 codex/deepagents/tN，PASS(head) 与交付记录取自该 run 目录。 */
export interface DeepAgentsMergeInput {
  source: "deepagents"; issue: number; branch: string; sha: string;
  run: { id: string; generation: number; dir: string };
  acceptance: Acceptance;
}
export type MergeInput = SandcastleMergeInput | DeepAgentsMergeInput;

const SHA = /^[a-f0-9]{40}$/;
const deepAgentsBranch = (ticket: number) => `codex/deepagents/t${ticket}`;
const reviewReceiptPath = (dir: string, sha: string) => join(dir, "workers", `review-${sha}`, "result.json");

function acceptanceBound(i: { sha: string; acceptance?: Acceptance }): boolean {
  const a = i.acceptance;
  return a?.head === i.sha && a.status === "accepted"
    && typeof a.reviewEvidence === "string" && !!a.reviewEvidence.trim()
    && typeof a.verificationEvidence === "string" && !!a.verificationEvidence.trim();
}

/** 纯形状校验，不读文件。任一来源都要求票号、固定 SHA 及绑定该 SHA 的验收证据。 */
export function validateMergeInputSource(i: MergeInput): void {
  if (!i || !Number.isSafeInteger(i.issue) || i.issue <= 0 || !SHA.test(i.sha)) throw new Error("归并输入必须为固定单票 SHA");
  if (i.source === undefined) {
    if (i.branch !== `sandcastle/issue-${i.issue}`) throw new Error("归并输入必须为固定单票 SHA");
  } else if (i.source === "deepagents") {
    if (i.branch !== deepAgentsBranch(i.issue)) throw new Error("DeepAgents 输入分支必须为 codex/deepagents/t<票号>");
    const r = i.run;
    const m = /^t(\d+)-[1-9]\d*$/.exec(r?.id ?? "");
    if (!m || Number(m[1]) !== i.issue || !Number.isSafeInteger(r.generation) || r.generation <= 0) throw new Error("DeepAgents run 身份与票号不符");
    if (typeof r.dir !== "string" || !isAbsolute(r.dir) || basename(r.dir) !== r.id) throw new Error("DeepAgents run 目录必须为该 run 的绝对路径");
    if (i.acceptance?.reviewEvidence !== reviewReceiptPath(r.dir, i.sha)) throw new Error("DeepAgents 独审证据必须指向该 SHA 的 review 回执");
  } else throw new Error("未知归并输入来源");
  if (!acceptanceBound(i)) throw new Error("缺少绑定精确 SHA 的单票验收/独审证据");
}

function readJson(path: string): Record<string, any> {
  try { return JSON.parse(readFileSync(path, "utf8")); }
  catch (e) { throw new Error(`DeepAgents 记录不可读：${path}：${String(e)}`); }
}

/** 从 run 目录读出 PASS(head) 与交付状态；任一项不绑定该 SHA 即拒绝。执行器只在 PASS(head) 后写 deepagents/review 状态。 */
export function readDeepAgentsDelivery(dir: string, sha: string): { ticket: number; runId: string; generation: number; branch: string; pr: string } {
  const run = readJson(join(dir, "run.json"));
  const claim = readJson(join(dir, "claim.json"));
  const state = readJson(join(dir, "delivery", "state.json"));
  const status = readJson(join(dir, "delivery", `status-${sha}.json`));
  const review = readJson(reviewReceiptPath(dir, sha));
  const ticket = run.ticket, runId = run.run_id;
  if (!Number.isSafeInteger(ticket) || runId !== basename(dir) || claim.ticket !== ticket || claim.run_id !== runId || !Number.isSafeInteger(claim.generation)) throw new Error("DeepAgents run/claim 记录不一致");
  if (state.phase !== "awaiting_merge" || state.head !== sha || state.branch !== deepAgentsBranch(ticket) || state.pr?.head !== sha) throw new Error(`DeepAgents 交付不在该 SHA 的 awaiting_merge：phase=${state.phase} head=${state.head}`);
  if (status.state !== "done" || status.result?.context !== "deepagents/review" || status.result?.sha !== sha) throw new Error("DeepAgents 缺少该 SHA 的 deepagents/review 状态");
  const summary = typeof review.result?.summary === "string" ? review.result.summary : "";
  if (review.status !== "completed" || review.mode !== "review" || review.expected_head !== sha || review.checkout_after?.head !== sha
    || review.result?.status !== "completed" || summary.split("\n")[0] !== "VERDICT: PASS") throw new Error("DeepAgents review 回执不是该 SHA 的 VERDICT: PASS");
  return { ticket, runId, generation: claim.generation, branch: state.branch, pr: String(state.pr?.url ?? "") };
}

/** 由主控生成 DeepAgents 输入：身份和 PASS(head) 取自 run 记录，主控只补单票验证证据。 */
export function deepAgentsMergeInput(dir: string, sha: string, verificationEvidence: string): DeepAgentsMergeInput {
  const d = readDeepAgentsDelivery(dir, sha);
  const input: DeepAgentsMergeInput = {
    source: "deepagents", issue: d.ticket, branch: d.branch, sha,
    run: { id: d.runId, generation: d.generation, dir },
    acceptance: { head: sha, status: "accepted", reviewEvidence: reviewReceiptPath(dir, sha), verificationEvidence },
  };
  validateMergeInputSource(input);
  return input;
}

/** 每次归并该输入前复核：run 记录仍交付这一 SHA，仓内来源分支也未前进到其他 head。旧输入不受影响。 */
export function admitMergeInput(repo: string, i: MergeInput): void {
  if (i.source !== "deepagents") return;
  const d = readDeepAgentsDelivery(i.run.dir, i.sha);
  if (d.ticket !== i.issue || d.runId !== i.run.id || d.generation !== i.run.generation || d.branch !== i.branch) throw new Error("DeepAgents run 记录与清单身份不符");
  const refs = execFileSync("git", ["for-each-ref", "--format=%(objectname) %(refname)", "refs/heads", "refs/remotes"], { cwd: repo, encoding: "utf8" })
    .split("\n").filter(Boolean).map(l => l.split(" ") as [string, string])
    .filter(([, ref]) => ref === `refs/heads/${i.branch}` || (ref.startsWith("refs/remotes/") && ref.split("/").slice(3).join("/") === i.branch));
  const moved = refs.find(([sha]) => sha !== i.sha);
  if (moved) throw new Error(`DeepAgents 来源分支已不在已审 SHA：${moved[1]}=${moved[0]}`);
}
