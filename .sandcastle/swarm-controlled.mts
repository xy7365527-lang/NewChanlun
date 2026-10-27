/** #1742 第 13 行受控入口：只接受 receipt 绑定的专用验收票，不读线上队列。 */
import { ROW_ENV, swarmBinding } from "./docker-acceptance.mts";

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
