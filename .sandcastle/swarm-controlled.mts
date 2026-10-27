/** #1742 第 13 行受控入口：只接受 receipt 绑定的专用验收票，不读线上队列。 */
import { execFileSync } from "node:child_process";
import { ROW_ENV, swarmBinding, type Admission } from "./docker-acceptance.mts";

export { swarmBinding };
export const TICKET_ENV = "SANDCASTLE_DOCKER_ACCEPTANCE_TICKET";

/** 未声明验收行 → undefined（普通入口）。声明 13 → 返回显式票号；其它行或缺/非法票号 → 抛错。
 * 票号是否就是 receipt.swarmTicket 由准入核对（binding 不一致即拒绝）。 */
export function controlledSwarmTicket(env: NodeJS.ProcessEnv = process.env): number | undefined {
  const row = env[ROW_ENV];
  if (row === undefined || row === "") return undefined;
  if (row !== "13") throw new Error(`[FAIL-loud #1617] 普通 swarm 入口只接受受控第 13 行（收到 ${row}）；14/15 属于归并冲突入口`);
  const raw = env[TICKET_ENV];
  const ticket = Number(raw);
  if (!raw || !Number.isSafeInteger(ticket) || ticket <= 0 || String(ticket) !== raw) throw new Error(`[FAIL-loud #1617] 受控第 13 行必须显式给出专用验收票（${TICKET_ENV}=<票号>）`);
  return ticket;
}

type Git = (args: string[], cwd: string) => string;
const hostGit: Git = (args, cwd) => execFileSync("git", ["-C", cwd, ...args], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();

/** 受控第 13 行的工蜂必须在被验收的候选上运行：基线固定为 receipt.head（git worktree add 接受 SHA），而不是会漂移的本地 main。
 * 在占用验收行、认领和启动任何工蜂之前核对：候选提交存在；当前 checkout 的 HEAD 就是它；验收票分支若已存在，必须以它为祖先。 */
export function assertControlledBase(admission: Admission, ticket: number, repoDir: string = process.cwd(), git: Git = hostGit): string {
  const head = admission.receipt?.head;
  if (admission.row !== "13" || !head) throw new Error("[FAIL-loud #1617] 受控第 13 行缺少 receipt 候选 head");
  let resolved: string;
  try { resolved = git(["rev-parse", "--verify", "-q", `${head}^{commit}`], repoDir); }
  catch { throw new Error(`[FAIL-loud #1617] 受控第 13 行候选 ${head.slice(0, 12)} 在当前仓库中不存在，拒绝启动工蜂`); }
  if (resolved !== head) throw new Error("[FAIL-loud #1617] 候选引用解析结果与 receipt.head 不一致");
  const current = git(["rev-parse", "HEAD"], repoDir);
  if (current !== head) throw new Error(`[FAIL-loud #1617] 当前 checkout HEAD ${current.slice(0, 12)} 不是被验收候选 ${head.slice(0, 12)}，拒绝启动工蜂`);
  let existing = "";
  try { existing = git(["rev-parse", "--verify", "-q", `refs/heads/sandcastle/issue-${ticket}`], repoDir); } catch { /* 分支尚不存在：由候选 head 新建 */ }
  if (existing) {
    try { git(["merge-base", "--is-ancestor", head, existing], repoDir); }
    catch { throw new Error(`[FAIL-loud #1617] 验收票分支 sandcastle/issue-${ticket} 已存在且不基于候选 ${head.slice(0, 12)}，拒绝复用`); }
  }
  return head;
}
