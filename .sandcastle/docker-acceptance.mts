/** #1742 方案B 的 Docker 准入：矩阵观测只证明“容器合同成立”，不等于“允许认领”。
 * 默认入口无精确 acceptance 事实一律 FAIL-loud。事实全部存放在宿主固定目录（acceptance-dir.mts），
 * 工蜂 worktree 与容器挂载触不到；receipt 绑定代码 head、合同摘要、镜像和用户批准，
 * 验收行 13/14/15 各自一次性，且绑定到具体运行（13=验收票，14/15=归并 run+检查点），同一绑定的中断后续做不重复占用。 */
import { execFileSync } from "node:child_process";
import { createHash, randomUUID } from "node:crypto";
import { closeSync, existsSync, fsyncSync, linkSync, lstatSync, openSync, readFileSync, readdirSync, renameSync, unlinkSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { acceptanceRoot, controlledDir } from "./acceptance-dir.mts";
import { DEFAULT_CAP_MASK, DOCKER_INNER_SANDBOX, PROBE_SCRIPT, PROTECTED_SUBPATHS } from "./docker-isolation.mts";
import { MAIN_MODEL } from "./main-model.mts";

export const ACCEPTANCE_TICKET = 1742;
export const ACCEPTANCE_ROWS = ["13", "14", "15"] as const;
export type AcceptanceRow = (typeof ACCEPTANCE_ROWS)[number];
export const ROW_ENV = "SANDCASTLE_DOCKER_ACCEPTANCE_ROW";
export const MAX_TTL_HOURS = 720;
export const MAX_RESUMES = 5;

/** 合同摘要：改任一项（受保护路径、内层沙箱模式、cap 上界、探针脚本、无 security 选项、绑定规则版本）即新合同，旧 receipt 作废。 */
export function contractDigest(): string {
  const canonical = JSON.stringify({
    id: "1742-docker-plan-b-v2", protectedSubpaths: PROTECTED_SUBPATHS, innerSandbox: DOCKER_INNER_SANDBOX,
    capMask: DEFAULT_CAP_MASK.toString(16), securityOpts: [], probe: PROBE_SCRIPT,
  });
  return createHash("sha256").update(canonical).digest("hex");
}

export interface AcceptanceReceipt {
  schema: 2; ticket: number; nonce: string; head: string; contract: string; image: string;
  rows: AcceptanceRow[]; swarmTicket?: number; approval: string; issuedAt: string; expiresAt: string;
}
export interface UsedMarker { schema: 2; nonce: string; row: AcceptanceRow; head: string; contract: string; binding: string; at: string; pid: number }
export interface CrossFamilyReview { file: string; head: string; model: string }
export interface AcceptedRecord {
  schema: 2; ticket: number; nonce: string; head: string; contract: string; image: string; approval: string;
  issuedAt: string; expiresAt: string; acceptedAt: string; evidence: Record<AcceptanceRow, string>; crossFamily: CrossFamilyReview;
}
export interface CodeIdentity { head: string; dirty: string }

export const swarmBinding = (ticket: number) => `swarm-ticket:${ticket}`;
/** 归并检查点身份：同一 run、同一基线、同一冲突位置（HEAD+入站 SHA）；中断后重入得到相同值，新运行不同。 */
export const mergeBinding = (o: { runId: string; base: string; head: string; incoming: string }) =>
  "merge-conflict:" + createHash("sha256").update(JSON.stringify([o.runId, o.base, o.head, o.incoming])).digest("hex");

/** 被执行的代码身份：本模块所在 checkout 的 HEAD，以及 .sandcastle 下未提交/未跟踪改动（有则不精确）。 */
export function codeIdentity(dir: string = dirname(fileURLToPath(import.meta.url))): CodeIdentity {
  const git = (args: string[]) => execFileSync("git", ["-C", dir, ...args], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
  const root = git(["rev-parse", "--show-toplevel"]);
  return { head: git(["rev-parse", "HEAD"]), dirty: git(["status", "--porcelain", "--", join(root, ".sandcastle")]) };
}

// ── 目录与文件 ────────────────────────────────────────────────────────────────
export interface AdmissionContext { dir?: string; env?: NodeJS.ProcessEnv; identity?: CodeIdentity; now?: Date; binding?: string; rows?: readonly AcceptanceRow[]; roots?: readonly string[] }

/** 生产路径只用宿主固定目录；dir 仅供单元测试与主控 CLI 的显式参数，且同样经过 controlledDir 校验。 */
export function resolveDir(ctx: { dir?: string; roots?: readonly string[] } = {}, create = false): string {
  return controlledDir(ctx.dir ?? acceptanceRoot(), { create, roots: ctx.roots });
}
const receiptPath = (dir: string) => join(dir, "receipt.json");
const acceptedPath = (dir: string) => join(dir, "accepted.json");
const usedPath = (dir: string, nonce: string, row: AcceptanceRow) => join(dir, `used-${nonce}-${row}.json`);
const resumePath = (dir: string, nonce: string, row: AcceptanceRow, n: number) => join(dir, `resume-${nonce}-${row}-${n}.json`);

/** 先写临时文件并 fsync，再用 link 原子发布：发布前最终名不可见，发布后内容必然完整；目标已存在即 EEXIST。 */
function publishOnce(path: string, value: unknown): void {
  const tmp = `${path}.tmp-${process.pid}-${randomUUID()}`;
  const fd = openSync(tmp, "wx", 0o600);
  try { writeFileSync(fd, JSON.stringify(value, null, 2) + "\n"); fsyncSync(fd); } finally { closeSync(fd); }
  try { linkSync(tmp, path); } finally { unlinkSync(tmp); }
  const dfd = openSync(dirname(path), "r");
  try { fsyncSync(dfd); } finally { closeSync(dfd); }
}

/** 严格读取：普通文件、属于当前用户、无组/其他写位，再解析 JSON。 */
function readStrict(path: string): unknown {
  const st = lstatSync(path);
  if (!st.isFile()) throw new Error(`${path} 不是普通文件`);
  const uid = process.getuid?.();
  if (uid !== undefined && st.uid !== uid) throw new Error(`${path} 不属于当前用户`);
  if ((st.mode & 0o022) !== 0) throw new Error(`${path} 权限过宽`);
  return JSON.parse(readFileSync(path, "utf8"));
}

const isObj = (v: unknown): v is Record<string, unknown> => typeof v === "object" && v !== null && !Array.isArray(v);
const str = (v: unknown, name: string): string => { if (typeof v !== "string" || !v.trim()) throw new Error(`${name} 缺失或非字符串`); return v; };
const hex40 = (v: unknown, name: string): string => { const s = str(v, name); if (!/^[0-9a-f]{40}$/.test(s)) throw new Error(`${name} 不是 40 位 SHA`); return s; };
const time = (v: unknown, name: string): number => { const t = Date.parse(str(v, name)); if (!Number.isFinite(t)) throw new Error(`${name} 不是合法时间`); return t; };
const rowOf = (v: unknown): AcceptanceRow => { if (!ACCEPTANCE_ROWS.includes(v as AcceptanceRow)) throw new Error("验收行非法"); return v as AcceptanceRow; };

export function parseReceipt(raw: unknown): AcceptanceReceipt {
  if (!isObj(raw) || raw.schema !== 2 || raw.ticket !== ACCEPTANCE_TICKET) throw new Error("receipt 结构无效");
  if (!Array.isArray(raw.rows) || !raw.rows.length) throw new Error("receipt.rows 无效");
  const rows = raw.rows.map(rowOf);
  const swarmTicket = raw.swarmTicket;
  if (swarmTicket !== undefined && (!Number.isSafeInteger(swarmTicket) || (swarmTicket as number) <= 0)) throw new Error("receipt.swarmTicket 无效");
  if (rows.includes("13") && swarmTicket === undefined) throw new Error("第 13 行必须绑定专用验收票");
  time(raw.issuedAt, "issuedAt"); time(raw.expiresAt, "expiresAt");
  return {
    schema: 2, ticket: ACCEPTANCE_TICKET, nonce: str(raw.nonce, "nonce"), head: hex40(raw.head, "head"), contract: str(raw.contract, "contract"),
    image: str(raw.image, "image"), rows, ...(swarmTicket !== undefined ? { swarmTicket: swarmTicket as number } : {}),
    approval: str(raw.approval, "approval"), issuedAt: raw.issuedAt as string, expiresAt: raw.expiresAt as string,
  };
}
export function parseUsed(raw: unknown, receipt: AcceptanceReceipt, row: AcceptanceRow): UsedMarker {
  if (!isObj(raw) || raw.schema !== 2) throw new Error("used 记录结构无效");
  const m: UsedMarker = { schema: 2, nonce: str(raw.nonce, "nonce"), row: rowOf(raw.row), head: hex40(raw.head, "head"), contract: str(raw.contract, "contract"), binding: str(raw.binding, "binding"), at: str(raw.at, "at"), pid: Number(raw.pid) };
  time(m.at, "at");
  if (m.nonce !== receipt.nonce || m.row !== row || m.head !== receipt.head || m.contract !== receipt.contract) throw new Error("used 记录与 receipt 不一致");
  return m;
}
function loadReceipt(dir: string): AcceptanceReceipt {
  if (!existsSync(receiptPath(dir))) throw new Error("receipt 缺失");
  try { return parseReceipt(readStrict(receiptPath(dir))); } catch (e) { throw new Error(`receipt 无效：${e instanceof Error ? e.message : String(e)}`); }
}
function loadUsed(dir: string, receipt: AcceptanceReceipt, row: AcceptanceRow): UsedMarker | null {
  const p = usedPath(dir, receipt.nonce, row);
  if (!existsSync(p)) return null;
  try { return parseUsed(readStrict(p), receipt, row); } catch (e) { throw new Error(`第 ${row} 行 used 记录无效：${e instanceof Error ? e.message : String(e)}`); }
}

// ── 签发 / 退役 ───────────────────────────────────────────────────────────────
export interface IssueOptions { dir?: string; roots?: readonly string[]; image: string; approval: string; rows?: readonly AcceptanceRow[]; swarmTicket?: number; ttlHours?: number; now?: Date; identity?: CodeIdentity }

/** 由主控在用户批准后执行；工位已有未退役 receipt 时拒绝覆盖（须先 retire，保留历史）。 */
export function issueReceipt(o: IssueOptions): AcceptanceReceipt {
  const id = o.identity ?? codeIdentity();
  if (id.dirty) throw new Error(`.sandcastle 存在未提交改动，不能给不精确的代码签发 receipt：${id.dirty.slice(0, 200)}`);
  if (!o.approval.trim()) throw new Error("签发 receipt 必须带用户批准引用");
  const rows = [...(o.rows ?? ACCEPTANCE_ROWS)];
  if (!rows.length || rows.some((r) => !ACCEPTANCE_ROWS.includes(r))) throw new Error("rows 只能是 13/14/15");
  if (rows.includes("13") && !(Number.isSafeInteger(o.swarmTicket) && o.swarmTicket! > 0)) throw new Error("第 13 行必须指定专用验收票（--swarm-ticket N）");
  const ttl = o.ttlHours ?? 12;
  if (!(ttl > 0 && ttl <= MAX_TTL_HOURS)) throw new Error(`ttlHours 必须在 (0, ${MAX_TTL_HOURS}] 内`);
  const now = o.now ?? new Date();
  const receipt: AcceptanceReceipt = {
    schema: 2, ticket: ACCEPTANCE_TICKET, nonce: randomUUID(), head: id.head, contract: contractDigest(), image: o.image, rows,
    ...(o.swarmTicket !== undefined ? { swarmTicket: o.swarmTicket } : {}),
    approval: o.approval.trim(), issuedAt: now.toISOString(), expiresAt: new Date(now.getTime() + ttl * 3_600_000).toISOString(),
  };
  const dir = resolveDir(o, true);
  publishOnce(receiptPath(dir), receipt);
  return receipt;
}

/** 退役当前 receipt 与 accepted 记录（改名保留，不删历史）。 */
export function retireReceipt(ctx: { dir?: string; roots?: readonly string[] } = {}): string | null {
  const dir = resolveDir(ctx);
  if (existsSync(acceptedPath(dir))) renameSync(acceptedPath(dir), join(dir, `accepted-retired-${randomUUID()}.json`));
  if (!existsSync(receiptPath(dir))) return null;
  let nonce: string = randomUUID();
  try { nonce = parseReceipt(readStrict(receiptPath(dir))).nonce; } catch { /* 损坏的 receipt 也要能退役 */ }
  const target = join(dir, `retired-${nonce}-${randomUUID().slice(0, 8)}.json`);
  renameSync(receiptPath(dir), target);
  return target;
}

// ── 登记验收 ──────────────────────────────────────────────────────────────────
const family = (model: string) => (model.split("/").at(-1) ?? model).split("-")[0]!.toLowerCase();

/** 第 13 行的外部异家族独审：swarm 自身作者与评审同为 MAIN_MODEL，不能给出异家族结论，须外部审阅精确候选 head。 */
export function readCrossFamilyReview(file: string): CrossFamilyReview {
  if (!file || !existsSync(file)) throw new Error("缺少外部异家族独审报告文件");
  const text = readFileSync(file, "utf8");
  const field = (name: string) => text.match(new RegExp(`^${name}:\\s*(\\S.*?)\\s*$`, "m"))?.[1];
  const head = field("HEAD"); const model = field("REVIEWER_MODEL");
  if (!/^VERDICT:\s*PASS\s*$/m.test(text)) throw new Error("外部独审报告缺少 `VERDICT: PASS`");
  if (!head || !/^[0-9a-f]{40}$/.test(head)) throw new Error("外部独审报告缺少 `HEAD: <40位SHA>`");
  if (!model) throw new Error("外部独审报告缺少 `REVIEWER_MODEL:`");
  if (family(model) === family(MAIN_MODEL)) throw new Error(`审阅模型 ${model} 与作者 ${MAIN_MODEL} 同家族，不算异家族独审`);
  return { file, head, model };
}

export interface RecordOptions {
  dir?: string; roots?: readonly string[]; image: string; approval?: string; evidence: Record<AcceptanceRow, string>;
  crossFamilyReview: string; identity?: CodeIdentity; now?: Date;
}
/** 13–15 受控验收完成后由主控登记：receipt 未过期、三行 used 记录都真实有效、证据与外部独审存在，且与当前 head/合同/镜像一致。
 * approval 若给出必须逐字等于 receipt.approval；记录保存 receipt 的有效期，到期自动失效。 */
export function recordAcceptance(o: RecordOptions): AcceptedRecord {
  const id = o.identity ?? codeIdentity();
  if (id.dirty) throw new Error("代码不精确，拒绝登记验收");
  const dir = resolveDir(o);
  const receipt = loadReceipt(dir);
  if (receipt.head !== id.head || receipt.contract !== contractDigest() || receipt.image !== o.image) throw new Error("receipt 与当前 head/合同/镜像不符");
  if ((o.now ?? new Date()).getTime() >= Date.parse(receipt.expiresAt)) throw new Error("receipt 已过期，不能登记验收");
  if (o.approval !== undefined && o.approval !== receipt.approval) throw new Error("批准引用与 receipt 不一致");
  if (!ACCEPTANCE_ROWS.every((r) => receipt.rows.includes(r))) throw new Error("receipt 范围不含 13–15 全部三行，不能登记完整验收");
  for (const row of ACCEPTANCE_ROWS) {
    if (!loadUsed(dir, receipt, row)) throw new Error(`第 ${row} 行尚未经受控入口真实运行`);
    if (!o.evidence[row] || !existsSync(o.evidence[row])) throw new Error(`第 ${row} 行缺少证据文件`);
  }
  const crossFamily = readCrossFamilyReview(o.crossFamilyReview);
  const record: AcceptedRecord = {
    schema: 2, ticket: ACCEPTANCE_TICKET, nonce: receipt.nonce, head: receipt.head, contract: receipt.contract, image: receipt.image, approval: receipt.approval,
    issuedAt: receipt.issuedAt, expiresAt: receipt.expiresAt, acceptedAt: (o.now ?? new Date()).toISOString(), evidence: o.evidence, crossFamily,
  };
  publishOnce(acceptedPath(dir), record);
  return record;
}

function parseAccepted(raw: unknown): AcceptedRecord {
  if (!isObj(raw) || raw.schema !== 2 || raw.ticket !== ACCEPTANCE_TICKET || !isObj(raw.evidence) || !isObj(raw.crossFamily)) throw new Error("accepted 结构无效");
  const evidence = Object.fromEntries(ACCEPTANCE_ROWS.map((r) => [r, str((raw.evidence as Record<string, unknown>)[r], `evidence.${r}`)])) as Record<AcceptanceRow, string>;
  time(raw.issuedAt, "issuedAt"); time(raw.expiresAt, "expiresAt"); time(raw.acceptedAt, "acceptedAt");
  return {
    schema: 2, ticket: ACCEPTANCE_TICKET, nonce: str(raw.nonce, "nonce"), head: hex40(raw.head, "head"), contract: str(raw.contract, "contract"), image: str(raw.image, "image"),
    approval: str(raw.approval, "approval"), issuedAt: raw.issuedAt as string, expiresAt: raw.expiresAt as string, acceptedAt: raw.acceptedAt as string, evidence,
    crossFamily: { file: str(raw.crossFamily.file, "crossFamily.file"), head: hex40(raw.crossFamily.head, "crossFamily.head"), model: str(raw.crossFamily.model, "crossFamily.model") },
  };
}

/** 已登记的验收是否对当前代码/合同/镜像仍然成立：每次重新读取 receipt 与三份 used 记录逐项核对，任何一项缺失、非法、不一致或过期都返回 null。 */
export function acceptedFor(image: string, ctx: AdmissionContext = {}): AcceptedRecord | null {
  try {
    const dir = resolveDir(ctx);
    if (!existsSync(acceptedPath(dir))) return null;
    const record = parseAccepted(readStrict(acceptedPath(dir)));
    const receipt = loadReceipt(dir);
    const id = ctx.identity ?? codeIdentity();
    if (id.dirty) return null;
    if (record.nonce !== receipt.nonce || record.head !== receipt.head || record.contract !== receipt.contract || record.image !== receipt.image
      || record.approval !== receipt.approval || record.expiresAt !== receipt.expiresAt || record.issuedAt !== receipt.issuedAt) return null;
    if (receipt.head !== id.head || receipt.contract !== contractDigest() || receipt.image !== image) return null;
    if ((ctx.now ?? new Date()).getTime() >= Date.parse(receipt.expiresAt)) return null;
    if (!ACCEPTANCE_ROWS.every((r) => receipt.rows.includes(r))) return null;
    for (const row of ACCEPTANCE_ROWS) {
      if (!loadUsed(dir, receipt, row)) return null;
      if (!existsSync(record.evidence[row])) return null;
    }
    if (!existsSync(record.crossFamily.file)) return null;
    return record;
  } catch { return null; }
}

// ── 准入 ──────────────────────────────────────────────────────────────────────
export interface Admission { receipt?: AcceptanceReceipt; row?: AcceptanceRow; accepted?: AcceptedRecord; resumed?: boolean; consume: () => void }

function refuse(why: string): never {
  throw new Error(`[FAIL-loud #1617] Docker 路径尚无精确验收 receipt：${why}。默认入口继续拒绝，不认领、不扩大容器权限；13–15 仅限受控一次性入口。`);
}

/** 先认已登记且仍有效的验收；否则只认受控 receipt。两者都不成立即拒绝。 */
export function checkAdmission(image: string, ctx: AdmissionContext = {}): Admission {
  const accepted = acceptedFor(image, ctx);
  if (accepted) return { accepted, consume: () => undefined };
  return checkControlledAdmission(image, ctx);
}

/** 受控 receipt 核对：入口声明验收行与运行绑定；该行未占用 → 首次（consume 原子占用）；已占用且绑定相同 → 同一运行的中断后续做（有上限，留痕）；
 * 已占用且绑定不同 → 拒绝（任何新运行不得复用）。 */
export function checkControlledAdmission(image: string, ctx: AdmissionContext = {}): Admission {
  const env = ctx.env ?? process.env;
  const row = env[ROW_ENV] as AcceptanceRow | undefined;
  if (!row || !ACCEPTANCE_ROWS.includes(row)) refuse(`未声明受控验收行（${ROW_ENV}=13|14|15）`);
  const allowed = ctx.rows ?? ACCEPTANCE_ROWS;
  if (!allowed.includes(row!)) refuse(`第 ${row} 行不能用于此入口（此入口只接受 ${allowed.join("/")}）`);
  if (!ctx.binding) refuse("缺少运行绑定");
  let dir: string, receipt: AcceptanceReceipt;
  try { dir = resolveDir(ctx); } catch (e) { return refuse(`验收目录不合格：${e instanceof Error ? e.message : String(e)}`); }
  try { receipt = loadReceipt(dir); } catch (e) { return refuse(e instanceof Error ? e.message : String(e)); }
  const id = ctx.identity ?? codeIdentity();
  if (id.dirty) refuse(".sandcastle 有未提交改动，代码不精确");
  if (receipt.head !== id.head) refuse(`代码 head 漂移（receipt ${receipt.head.slice(0, 12)} ≠ 当前 ${id.head.slice(0, 12)}）`);
  if (receipt.contract !== contractDigest()) refuse("隔离合同已变化");
  if (receipt.image !== image) refuse(`镜像不一致（${receipt.image} ≠ ${image}）`);
  if (!receipt.rows.includes(row!)) refuse(`批准范围不含第 ${row} 行`);
  if ((ctx.now ?? new Date()).getTime() >= Date.parse(receipt.expiresAt)) refuse("receipt 已过期");
  if (row === "13" && ctx.binding !== swarmBinding(receipt.swarmTicket!)) refuse(`第 13 行只允许专用验收票 #${receipt.swarmTicket}`);
  let used: UsedMarker | null;
  try { used = loadUsed(dir, receipt, row!); } catch (e) { return refuse(e instanceof Error ? e.message : String(e)); }
  const r = row!;
  if (used) {
    if (used.binding !== ctx.binding) refuse(`第 ${r} 行已被另一运行/检查点使用，新运行不得复用`);
    const resumeCount = readdirSync(dir).filter((f) => f.startsWith(`resume-${receipt.nonce}-${r}-`) && !f.includes(".tmp-")).length;
    if (resumeCount >= MAX_RESUMES) refuse(`第 ${r} 行同一运行的恢复次数已达上限 ${MAX_RESUMES}`);
    const consume = () => {
      try { publishOnce(resumePath(dir, receipt.nonce, r, resumeCount + 1), { schema: 2, nonce: receipt.nonce, row: r, binding: ctx.binding, at: new Date().toISOString(), pid: process.pid }); }
      catch { refuse(`第 ${r} 行恢复记录被并发写入`); }
    };
    return { receipt, row: r, resumed: true, consume };
  }
  const consume = () => {
    const marker: UsedMarker = { schema: 2, nonce: receipt.nonce, row: r, head: receipt.head, contract: receipt.contract, binding: ctx.binding!, at: new Date().toISOString(), pid: process.pid };
    try { publishOnce(usedPath(dir, receipt.nonce, r), marker); }
    catch (e) { refuse(`第 ${r} 行已被并发占用（${e instanceof Error ? e.message : String(e)}）`); }
  };
  return { receipt, row: r, consume };
}

// ── 主控 CLI（仅宿主主控使用；目录固定，--dir 只用于演练并同样受 controlledDir 校验）────────
function main(argv: string[]) {
  const [cmd, ...rest] = argv;
  const opt = (name: string) => { const i = rest.indexOf(`--${name}`); return i >= 0 ? rest[i + 1] : undefined; };
  const dirOpt = opt("dir");
  const ctx = dirOpt ? { dir: dirOpt } : {};
  if (cmd === "issue") {
    const image = opt("image"), approval = opt("approval");
    if (!image || !approval) throw new Error("用法：issue --image <镜像> --approval <用户批准引用> [--rows 13,14,15] [--swarm-ticket N] [--ttl-hours 12]");
    const rows = opt("rows")?.split(",") as AcceptanceRow[] | undefined;
    const r = issueReceipt({ ...ctx, image, approval, rows, swarmTicket: opt("swarm-ticket") ? Number(opt("swarm-ticket")) : undefined, ttlHours: opt("ttl-hours") ? Number(opt("ttl-hours")) : undefined });
    console.log(JSON.stringify({ issued: receiptPath(resolveDir(ctx)), nonce: r.nonce, head: r.head, rows: r.rows, swarmTicket: r.swarmTicket, expiresAt: r.expiresAt }));
  } else if (cmd === "accept") {
    const image = opt("image");
    if (!image) throw new Error("用法：accept --image <镜像> [--approval <引用>] --evidence13 <文件> --evidence14 <文件> --evidence15 <文件> --cross-family-review <文件>");
    const evidence = { "13": opt("evidence13") ?? "", "14": opt("evidence14") ?? "", "15": opt("evidence15") ?? "" } as Record<AcceptanceRow, string>;
    const r = recordAcceptance({ ...ctx, image, approval: opt("approval"), evidence, crossFamilyReview: opt("cross-family-review") ?? "" });
    console.log(JSON.stringify({ accepted: acceptedPath(resolveDir(ctx)), head: r.head, nonce: r.nonce, expiresAt: r.expiresAt }));
  } else if (cmd === "retire") {
    console.log(JSON.stringify({ retired: retireReceipt(ctx) }));
  } else if (cmd === "status") {
    const dir = resolveDir(ctx);
    let receipt: AcceptanceReceipt | null = null; let invalid: string | undefined;
    if (existsSync(receiptPath(dir))) { try { receipt = loadReceipt(dir); } catch (e) { invalid = String(e); } }
    const used = receipt ? ACCEPTANCE_ROWS.map((r) => { try { return [r, loadUsed(dir, receipt!, r)?.binding ?? null]; } catch (e) { return [r, `INVALID ${e}`]; } }) : [];
    console.log(JSON.stringify({ dir, receipt, invalid, used: Object.fromEntries(used), accepted: receipt ? existsSync(acceptedPath(dir)) : false, current: codeIdentity() }, null, 2));
  } else throw new Error("命令：issue | accept | status | retire");
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try { main(process.argv.slice(2)); } catch (error) { console.error(error instanceof Error ? error.message : String(error)); process.exitCode = 1; }
}
