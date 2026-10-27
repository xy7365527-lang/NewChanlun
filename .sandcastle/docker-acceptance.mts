/** #1742 方案B 的 Docker 准入：矩阵观测只证明“容器合同成立”，不等于“允许认领”。
 * 默认入口无精确 acceptance receipt 一律 FAIL-loud；receipt 绑定代码 head、合同摘要、镜像和用户批准范围，
 * 每个验收行（13 普通 swarm / 14 首次冲突工蜂 / 15 中断恢复）一次性，用后或漂移即拒。 */
import { execFileSync } from "node:child_process";
import { createHash, randomUUID } from "node:crypto";
import { closeSync, existsSync, mkdirSync, openSync, readFileSync, renameSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { DEFAULT_CAP_MASK, DOCKER_INNER_SANDBOX, PROBE_SCRIPT, PROTECTED_SUBPATHS } from "./docker-isolation.mts";

export const ACCEPTANCE_TICKET = 1742;
export const ACCEPTANCE_ROWS = ["13", "14", "15"] as const;
export type AcceptanceRow = (typeof ACCEPTANCE_ROWS)[number];
export const ROW_ENV = "SANDCASTLE_DOCKER_ACCEPTANCE_ROW";
export const DIR_ENV = "SANDCASTLE_DOCKER_ACCEPTANCE_DIR";

/** 合同内容摘要：改任一项（受保护路径、内层沙箱模式、cap 上界、探针脚本、不加 security 选项）即视为新合同，旧 receipt 作废。 */
export function contractDigest(): string {
  const canonical = JSON.stringify({
    id: "1742-docker-plan-b", protectedSubpaths: PROTECTED_SUBPATHS, innerSandbox: DOCKER_INNER_SANDBOX,
    capMask: DEFAULT_CAP_MASK.toString(16), securityOpts: [], probe: PROBE_SCRIPT,
  });
  return createHash("sha256").update(canonical).digest("hex");
}

export interface AcceptanceReceipt {
  schema: 1; ticket: number; nonce: string; head: string; contract: string; image: string;
  rows: AcceptanceRow[]; approval: string; issuedAt: string; expiresAt: string;
}
export interface CodeIdentity { head: string; dirty: string }

/** 被执行的代码身份：本模块所在 checkout 的 HEAD，以及 .sandcastle 下未提交/未跟踪改动（有则不精确）。 */
export function codeIdentity(dir: string = dirname(fileURLToPath(import.meta.url))): CodeIdentity {
  const git = (args: string[]) => execFileSync("git", ["-C", dir, ...args], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
  const root = git(["rev-parse", "--show-toplevel"]);
  return { head: git(["rev-parse", "HEAD"]), dirty: git(["status", "--porcelain", "--", join(root, ".sandcastle")]) };
}

export function acceptanceDir(env: NodeJS.ProcessEnv = process.env, cwd: string = process.cwd()): string {
  return env[DIR_ENV] || join(cwd, ".sandcastle", "logs", "docker-acceptance");
}
const receiptPath = (dir: string) => join(dir, "receipt.json");
const usedPath = (dir: string, nonce: string, row: AcceptanceRow) => join(dir, `used-${nonce}-${row}.json`);

export interface IssueOptions { dir: string; image: string; approval: string; rows?: readonly AcceptanceRow[]; ttlHours?: number; now?: Date; identity?: CodeIdentity }

/** 签发由主控在用户批准后执行；工位已有未退役 receipt 时拒绝覆盖（须先 retire，保留历史）。 */
export function issueReceipt(o: IssueOptions): AcceptanceReceipt {
  const id = o.identity ?? codeIdentity();
  if (id.dirty) throw new Error(`.sandcastle 存在未提交改动，不能给不精确的代码签发 receipt：${id.dirty.slice(0, 200)}`);
  if (!o.approval.trim()) throw new Error("签发 receipt 必须带用户批准引用");
  const rows = [...(o.rows ?? ACCEPTANCE_ROWS)];
  if (!rows.length || rows.some((r) => !ACCEPTANCE_ROWS.includes(r))) throw new Error("rows 只能是 13/14/15");
  const now = o.now ?? new Date();
  const receipt: AcceptanceReceipt = {
    schema: 1, ticket: ACCEPTANCE_TICKET, nonce: randomUUID(), head: id.head, contract: contractDigest(), image: o.image,
    rows, approval: o.approval.trim(), issuedAt: now.toISOString(), expiresAt: new Date(now.getTime() + (o.ttlHours ?? 12) * 3_600_000).toISOString(),
  };
  mkdirSync(o.dir, { recursive: true });
  const fd = openSync(receiptPath(o.dir), "wx", 0o600); // 已存在即 EEXIST
  try { writeFileSync(fd, JSON.stringify(receipt, null, 2) + "\n"); } finally { closeSync(fd); }
  return receipt;
}

/** 退役当前 receipt（改名保留，不删历史）。 */
export function retireReceipt(dir: string): string | null {
  if (existsSync(acceptedPath(dir))) renameSync(acceptedPath(dir), join(dir, `accepted-retired-${randomUUID()}.json`));
  if (!existsSync(receiptPath(dir))) return null;
  const nonce = (JSON.parse(readFileSync(receiptPath(dir), "utf8")) as { nonce?: string }).nonce ?? randomUUID();
  const target = join(dir, `retired-${nonce}.json`);
  renameSync(receiptPath(dir), target);
  return target;
}

const acceptedPath = (dir: string) => join(dir, "accepted.json");
export interface AcceptedRecord {
  schema: 1; ticket: number; head: string; contract: string; image: string; approval: string; nonce: string; acceptedAt: string;
  evidence: Record<AcceptanceRow, string>;
}

/** 13–15 受控验收完成后由主控登记：必须三行都真实占用过、证据文件存在，且与当前 head/合同/镜像一致。
 * 登记后普通入口凭此记录加实时观测放行；head、合同或镜像漂移即失效，需重新受控验收。 */
export function recordAcceptance(o: { dir: string; image: string; approval: string; evidence: Record<AcceptanceRow, string>; identity?: CodeIdentity; now?: Date }): AcceptedRecord {
  const id = o.identity ?? codeIdentity();
  if (id.dirty) throw new Error("代码不精确，拒绝登记验收");
  if (!existsSync(receiptPath(o.dir))) throw new Error("没有受控 receipt，无法登记验收");
  const receipt = JSON.parse(readFileSync(receiptPath(o.dir), "utf8")) as AcceptanceReceipt;
  if (receipt.head !== id.head || receipt.contract !== contractDigest() || receipt.image !== o.image) throw new Error("receipt 与当前 head/合同/镜像不符");
  for (const row of ACCEPTANCE_ROWS) {
    if (!existsSync(usedPath(o.dir, receipt.nonce, row))) throw new Error(`第 ${row} 行尚未经受控入口真实运行`);
    if (!o.evidence[row] || !existsSync(o.evidence[row])) throw new Error(`第 ${row} 行缺少证据文件`);
  }
  if (!o.approval.trim()) throw new Error("登记验收必须带批准引用");
  const record: AcceptedRecord = { schema: 1, ticket: ACCEPTANCE_TICKET, head: id.head, contract: receipt.contract, image: o.image, approval: o.approval.trim(), nonce: receipt.nonce, acceptedAt: (o.now ?? new Date()).toISOString(), evidence: o.evidence };
  const fd = openSync(acceptedPath(o.dir), "wx", 0o600);
  try { writeFileSync(fd, JSON.stringify(record, null, 2) + "\n"); } finally { closeSync(fd); }
  return record;
}

/** 已登记的验收记录是否对当前代码/合同/镜像有效。 */
export function acceptedFor(image: string, ctx: AdmissionContext = {}): AcceptedRecord | null {
  const dir = ctx.dir ?? acceptanceDir(ctx.env ?? process.env);
  if (!existsSync(acceptedPath(dir))) return null;
  let r: AcceptedRecord;
  try { r = JSON.parse(readFileSync(acceptedPath(dir), "utf8")) as AcceptedRecord; } catch { return null; }
  const id = ctx.identity ?? codeIdentity();
  const complete = ACCEPTANCE_ROWS.every((row) => r.evidence?.[row] && existsSync(r.evidence[row]));
  return r.schema === 1 && r.ticket === ACCEPTANCE_TICKET && !id.dirty && r.head === id.head && r.contract === contractDigest() && r.image === image && complete ? r : null;
}

export interface Admission { receipt?: AcceptanceReceipt; row?: AcceptanceRow; accepted?: AcceptedRecord; consume: () => void }
export interface AdmissionContext { env?: NodeJS.ProcessEnv; dir?: string; identity?: CodeIdentity; now?: Date }

function refuse(why: string): never {
  throw new Error(`[FAIL-loud #1617] Docker 路径尚无精确验收 receipt：${why}。默认入口继续拒绝，不认领、不扩大容器权限；13–15 仅限受控一次性入口。`);
}

/** 先认已登记的验收记录（无需占用）；否则只认受控一次性 receipt。两者都不成立即拒绝。 */
export function checkAdmission(image: string, ctx: AdmissionContext = {}): Admission {
  const accepted = acceptedFor(image, ctx);
  if (accepted) return { receipt: undefined, row: undefined, accepted, consume: () => undefined };
  return checkControlledAdmission(image, ctx);
}

/** 核对受控 receipt 与本次调用完全一致；通过后返回 consume（原子占用该行，重复占用即拒）。 */
export function checkControlledAdmission(image: string, ctx: AdmissionContext = {}): Admission {
  const env = ctx.env ?? process.env;
  const row = env[ROW_ENV] as AcceptanceRow | undefined;
  if (!row || !ACCEPTANCE_ROWS.includes(row)) refuse(`未声明受控验收行（${ROW_ENV}=13|14|15）`);
  const dir = ctx.dir ?? acceptanceDir(env);
  if (!existsSync(receiptPath(dir))) refuse("receipt 缺失");
  let receipt: AcceptanceReceipt;
  try { receipt = JSON.parse(readFileSync(receiptPath(dir), "utf8")) as AcceptanceReceipt; } catch { return refuse("receipt 无法解析"); }
  if (receipt.schema !== 1 || receipt.ticket !== ACCEPTANCE_TICKET || !receipt.nonce || !receipt.approval) refuse("receipt 结构或批准引用无效");
  const id = ctx.identity ?? codeIdentity();
  if (id.dirty) refuse(".sandcastle 有未提交改动，代码不精确");
  if (receipt.head !== id.head) refuse(`代码 head 漂移（receipt ${receipt.head.slice(0, 12)} ≠ 当前 ${id.head.slice(0, 12)}）`);
  if (receipt.contract !== contractDigest()) refuse("隔离合同已变化");
  if (receipt.image !== image) refuse(`镜像不一致（${receipt.image} ≠ ${image}）`);
  if (!receipt.rows.includes(row)) refuse(`批准范围不含第 ${row} 行`);
  if ((ctx.now ?? new Date()).getTime() > Date.parse(receipt.expiresAt)) refuse("receipt 已过期");
  if (existsSync(usedPath(dir, receipt.nonce, row))) refuse(`第 ${row} 行已使用，重复拒绝`);
  const consume = () => {
    let fd: number;
    try { fd = openSync(usedPath(dir, receipt.nonce, row), "wx", 0o600); }
    catch { return refuse(`第 ${row} 行已被并发占用，重复拒绝`); }
    try { writeFileSync(fd, JSON.stringify({ nonce: receipt.nonce, row, head: receipt.head, at: new Date().toISOString(), pid: process.pid }) + "\n"); } finally { closeSync(fd); }
  };
  return { receipt, row, consume };
}

function main(argv: string[]) {
  const [cmd, ...rest] = argv;
  const opt = (name: string) => { const i = rest.indexOf(`--${name}`); return i >= 0 ? rest[i + 1] : undefined; };
  const dir = opt("dir") ?? acceptanceDir();
  if (cmd === "issue") {
    const image = opt("image"), approval = opt("approval");
    if (!image || !approval) throw new Error("用法：issue --image <镜像> --approval <用户批准引用> [--rows 13,14,15] [--ttl-hours 12] [--dir <目录>]");
    const rows = opt("rows")?.split(",") as AcceptanceRow[] | undefined;
    const r = issueReceipt({ dir, image, approval, rows, ttlHours: opt("ttl-hours") ? Number(opt("ttl-hours")) : undefined });
    console.log(JSON.stringify({ issued: receiptPath(dir), nonce: r.nonce, head: r.head, rows: r.rows, expiresAt: r.expiresAt }));
  } else if (cmd === "accept") {
    const image = opt("image"), approval = opt("approval");
    if (!image || !approval) throw new Error("用法：accept --image <镜像> --approval <引用> --evidence13 <文件> --evidence14 <文件> --evidence15 <文件> [--dir <目录>]");
    const evidence = { "13": opt("evidence13") ?? "", "14": opt("evidence14") ?? "", "15": opt("evidence15") ?? "" } as Record<AcceptanceRow, string>;
    const r = recordAcceptance({ dir, image, approval, evidence });
    console.log(JSON.stringify({ accepted: acceptedPath(dir), head: r.head, nonce: r.nonce }));
  } else if (cmd === "retire") {
    console.log(JSON.stringify({ retired: retireReceipt(dir) }));
  } else if (cmd === "status") {
    const present = existsSync(receiptPath(dir));
    const receipt = present ? (JSON.parse(readFileSync(receiptPath(dir), "utf8")) as AcceptanceReceipt) : null;
    console.log(JSON.stringify({ dir, present, receipt, used: receipt ? ACCEPTANCE_ROWS.filter((r) => existsSync(usedPath(dir, receipt.nonce, r))) : [], current: codeIdentity() }, null, 2));
  } else throw new Error("命令：issue | accept | status | retire");
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try { main(process.argv.slice(2)); } catch (error) { console.error(error instanceof Error ? error.message : String(error)); process.exitCode = 1; }
}
