/**
 * 每票 MAIN 会话（#1742 F4）：分级考古与起草、引用专修、重做共用同一个具名 Claude CLI 会话。
 * 会话绑定本票与取证提交；独审不经过这里，始终另起无状态调用。副作用仍全部由 Node 宿主执行。
 */
import { existsSync, readdirSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import type { ModelResult, ModelRunner } from "./decision-claimer.mts";

export const MAIN_SESSION_FILE = "main-session.json";
export type SessionRef = { kind: "start" } | { kind: "resume"; id: string };
export type MainRole = "triage" | "draft";
export interface MainTurn { role: MainRole; model: string; ref: SessionRef["kind"] }
export interface MainSession {
  version: 1;
  ticket: number;
  sessionId: string;
  sourceCommit: string;
  /** 开会话时票上的评论条数；其后的编排者退回作废本会话。 */
  commentsSeen: number;
  turns: MainTurn[];
}
export interface MainSessionLoad { prior: MainSession | null; retired?: string }

const SHA = /^[0-9a-f]{40,64}$/;

function parse(raw: string, ticket: number): MainSession | string {
  let j: Partial<MainSession>;
  try { j = JSON.parse(raw) as Partial<MainSession>; } catch { return "会话记录不可解析"; }
  if (j.version !== 1 || typeof j.sessionId !== "string" || !j.sessionId || typeof j.sourceCommit !== "string" || !SHA.test(j.sourceCommit) ||
      !Number.isSafeInteger(j.commentsSeen) || (j.commentsSeen as number) < 0 || !Array.isArray(j.turns) || j.turns.length === 0) return "会话记录形状非法";
  if (j.ticket !== ticket) return `会话记录属于 #${j.ticket}，不是 #${ticket}`;
  return j as MainSession;
}

/** 旧会话不能再续时改名留档，下一次调用另起新会话；原因写进 summary。 */
function retire(dir: string, reason: string): string {
  const k = readdirSync(dir).filter((f) => f.startsWith("main-session.retired-")).length + 1;
  renameSync(join(dir, MAIN_SESSION_FILE), join(dir, `main-session.retired-${k}.json`));
  return reason;
}

/** 读回本票会话。取证提交变了、属于别票、记录坏了或其后有退回，都不续接旧历史。 */
export function loadMainSession(dir: string, ticket: number, sourceCommit: string, reopened: (s: MainSession) => boolean): MainSessionLoad {
  const path = join(dir, MAIN_SESSION_FILE);
  if (!existsSync(path)) return { prior: null };
  const rec = parse(readFileSync(path, "utf8"), ticket);
  if (typeof rec === "string") return { prior: null, retired: retire(dir, rec) };
  if (rec.sourceCommit !== sourceCommit) return { prior: null, retired: retire(dir, `取证提交已从 ${rec.sourceCommit.slice(0, 10)} 变为 ${sourceCommit.slice(0, 10)}`) };
  if (reopened(rec)) return { prior: null, retired: retire(dir, "会话之后编排者退回") };
  return { prior: rec };
}

export interface MainSessionHandle {
  /** MAIN 角色（triage/draft）专用：首轮 start，之后按记录的 id resume。 */
  run: ModelRunner;
  current(): MainSession | null;
  /** 最近一轮实际应答的模型。 */
  model(): string | undefined;
}

export function openMainSession(o: {
  dir: string; ticket: number; sourceCommit: string; commentsSeen: number;
  head: () => string; run: ModelRunner; prior: MainSession | null; onSave?: (s: MainSession) => void;
}): MainSessionHandle {
  let rec = o.prior;
  const save = (s: MainSession) => {
    const path = join(o.dir, MAIN_SESSION_FILE);
    writeFileSync(path + ".tmp", JSON.stringify(s, null, 2) + "\n");
    renameSync(path + ".tmp", path);
    o.onSave?.(s);
  };
  const run: ModelRunner = async (c): Promise<ModelResult> => {
    if (c.role !== "triage" && c.role !== "draft") throw new Error(`MAIN 会话不承担 ${c.role}`);
    const head = o.head().trim();
    if (head !== o.sourceCommit) throw new Error(`取证检出已从 ${o.sourceCommit.slice(0, 10)} 变为 ${head.slice(0, 10)}，拒绝在本会话上续接`);
    const ref: SessionRef = rec ? { kind: "resume", id: rec.sessionId } : { kind: "start" };
    const r = await o.run({ ...c, session: ref });
    if (!r.sessionId) throw new Error("模型通道未回报会话身份，拒绝把起草交给无状态上下文");
    if (rec && r.sessionId !== rec.sessionId) throw new Error(`续接会话身份不符：期望 ${rec.sessionId}，得到 ${r.sessionId}`);
    const turn: MainTurn = { role: c.role, model: r.model ?? c.model, ref: ref.kind };
    rec = rec
      ? { ...rec, turns: [...rec.turns, turn] }
      : { version: 1, ticket: o.ticket, sessionId: r.sessionId, sourceCommit: o.sourceCommit, commentsSeen: o.commentsSeen, turns: [turn] };
    save(rec);
    return r;
  };
  return { run, current: () => rec, model: () => rec?.turns.at(-1)?.model };
}
