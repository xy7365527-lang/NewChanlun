/** #1742 方案B 的 Docker 准入：矩阵观测只证明“容器合同成立”，不等于“允许认领”。
 * 默认入口无精确 acceptance 事实一律 FAIL-loud。事实全部存放在宿主固定目录（acceptance-dir.mts），
 * 工蜂 worktree 与容器挂载触不到；receipt 绑定代码 head、合同摘要、镜像和用户批准，
 * 验收行 13/14/15 各自一次性，且绑定到具体运行（13=验收票，14/15=归并 run+检查点），同一绑定的中断后续做不重复占用。 */
import { execFileSync } from "node:child_process";
import { createHash, randomUUID } from "node:crypto";
import { closeSync, existsSync, fsyncSync, linkSync, lstatSync, openSync, readFileSync, readdirSync, renameSync, unlinkSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { acceptanceRoot, assertHomeNotRedirected, controlledDir } from "./acceptance-dir.mts";
import { CONTAINER_ENV_ALLOW, DEFAULT_CAP_MASK, DOCKER_INNER_SANDBOX, PROBE_SCRIPT, PROTECTED_SUBPATHS, imageEnvSecrets } from "./docker-isolation.mts";
import { MAIN_MODEL } from "./main-model.mts";

export const ACCEPTANCE_TICKET = 1742;
export const ACCEPTANCE_ROWS = ["13", "14", "15"] as const;
export type AcceptanceRow = (typeof ACCEPTANCE_ROWS)[number];
export const ROW_ENV = "SANDCASTLE_DOCKER_ACCEPTANCE_ROW";
export const MAX_TTL_HOURS = 720;
export const MAX_RESUMES = 5;
export const MAX_CHECKPOINTS = 16;

/** 合同摘要：改任一项（受保护路径、内层沙箱模式、cap 上界、探针脚本、无 security 选项、容器 env 白名单、绑定规则版本）即新合同，旧 receipt 作废。 */
export function contractDigest(): string {
  const canonical = JSON.stringify({
    id: "1742-docker-plan-b-v3", protectedSubpaths: PROTECTED_SUBPATHS, innerSandbox: DOCKER_INNER_SANDBOX,
    capMask: DEFAULT_CAP_MASK.toString(16), securityOpts: [], containerEnvAllow: CONTAINER_ENV_ALLOW, imageIdBound: true, probe: PROBE_SCRIPT,
  });
  return createHash("sha256").update(canonical).digest("hex");
}

/** 镜像 Id（`sha256:<64 hex>`）：receipt 绑定 Id 而非仅 tag，tag 被重指或镜像被重建即漂移。 */
export type ImageIdOf = (image: string) => string;
export const IMAGE_ID = /^sha256:[0-9a-f]{64}$/;
/** 默认解析：`docker image inspect`；同时拒绝镜像自带 ENV 里名字像凭据且值非空的条目（空占位键允许）。失败即抛，不回退为 tag。 */
export const dockerImageId: ImageIdOf = (image) => {
  let out: string;
  try { out = execFileSync("docker", ["image", "inspect", image, "--format", "{{.Id}}\t{{json .Config.Env}}"], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"], timeout: 30_000, killSignal: "SIGKILL" }).trim(); }
  catch (e) { throw new Error(`无法解析镜像 ${image} 的 Id：${String((e as Error).message).slice(0, 160)}`); }
  return parseImageInspect(out, image);
};
/** `docker image inspect --format '{{.Id}}\t{{json .Config.Env}}'` 的输出：Id 须为 sha256，镜像自带 ENV 不得有非空凭据类键。 */
export function parseImageInspect(out: string, image: string): string {
  const tab = out.indexOf("\t"); const id = tab < 0 ? out : out.slice(0, tab);
  if (!IMAGE_ID.test(id)) throw new Error(`镜像 ${image} 的 Id 格式无效`);
  let env: string[] = [];
  try { env = tab < 0 ? [] : JSON.parse(out.slice(tab + 1)) ?? []; } catch { throw new Error(`镜像 ${image} 的 ENV 无法解析`); }
  const secrets = imageEnvSecrets(env);
  if (secrets.length) throw new Error(`镜像 ${image} 自带非空凭据类 ENV（${secrets.join(", ")}），拒绝`);
  return id;
}
const imageIdOf = (ctx: { imageIdOf?: ImageIdOf } | undefined, image: string): string => {
  const id = (ctx?.imageIdOf ?? dockerImageId)(image);
  if (!IMAGE_ID.test(id)) throw new Error(`镜像 ${image} 的 Id 格式无效`);
  return id;
};

export interface AcceptanceReceipt {
  schema: 2; ticket: number; nonce: string; head: string; contract: string; image: string; imageId: string;
  rows: AcceptanceRow[]; swarmTicket?: number; approval: string; issuedAt: string; expiresAt: string;
}
export interface UsedMarker { schema: 2; nonce: string; row: AcceptanceRow; head: string; contract: string; binding: string; at: string; pid: number }
export interface CrossFamilyReview { file: string; head: string; model: string }
export interface AcceptedRecord {
  schema: 2; ticket: number; nonce: string; head: string; contract: string; image: string; imageId: string; approval: string;
  issuedAt: string; expiresAt: string; acceptedAt: string; evidence: Record<AcceptanceRow, string>; crossFamily: CrossFamilyReview;
  /** 登记时每行已占用的运行绑定快照（第 14/15 行的 run+检查点登记前未知，只能在此固定）；acceptedFor 逐字核对。 */
  bindings: Record<AcceptanceRow, string[]>;
}
export interface CodeIdentity { head: string; dirty: string }

export const swarmBinding = (ticket: number) => `swarm-ticket:${ticket}`;
export interface MergeCheckpoint { runId: string; base: string; instance: string; head: string; incoming: string }
const MERGE_PREFIX = "merge-conflict:v1:";
/** 归并检查点身份：同一 run 实例、同一基线、同一冲突位置（HEAD+入站 SHA）；SIGKILL 后重入得到相同值。
 * instance 是该 run 证据目录创建时生成的随机实例 id：删除检查点/重置后从头重跑得到新 instance，不能冒充恢复。可解析，登记时逐字段核对。 */
export const mergeBinding = (o: MergeCheckpoint) =>
  MERGE_PREFIX + Buffer.from(JSON.stringify({ runId: o.runId, base: o.base, instance: o.instance, head: o.head, incoming: o.incoming })).toString("base64url");
/** 该归并 run 证据目录（宿主控制、不挂进工蜂）里的实例 id：首次需要门禁时原子创建，之后只读；与 runId/base 不符即拒绝。 */
export function mergeInstance(evidenceDir: string, runId: string, base: string): string {
  const p = join(evidenceDir, "gate-instance.json");
  if (!existsSync(p)) {
    try { publishOnce(p, { schema: 1, id: randomUUID(), runId, base }); } catch { /* 并发创建：下面统一读取 */ }
  }
  const raw = readStrict(p);
  if (!isObj(raw) || raw.schema !== 1 || raw.runId !== runId || raw.base !== base) throw new Error("gate-instance.json 与本归并 run 不符");
  return str(raw.id, "instance.id");
}
export function parseMergeBinding(binding: string): MergeCheckpoint {
  if (!binding.startsWith(MERGE_PREFIX)) throw new Error("binding 不是归并检查点格式");
  let raw: unknown;
  try { raw = JSON.parse(Buffer.from(binding.slice(MERGE_PREFIX.length), "base64url").toString("utf8")); } catch { throw new Error("binding 无法解析"); }
  if (!isObj(raw)) throw new Error("binding 结构无效");
  const cp = { runId: str(raw.runId, "binding.runId"), base: hex40(raw.base, "binding.base"), instance: str(raw.instance, "binding.instance"), head: hex40(raw.head, "binding.head"), incoming: hex40(raw.incoming, "binding.incoming") };
  if (binding !== mergeBinding(cp)) throw new Error("binding 不是规范编码");
  return cp;
}
const bindingHash = (binding: string) => createHash("sha256").update(binding).digest("hex").slice(0, 16);

/** 被执行的代码身份：本模块所在 checkout 的 HEAD，以及 .sandcastle 下未提交/未跟踪改动（有则不精确）。 */
export function codeIdentity(dir: string = dirname(fileURLToPath(import.meta.url))): CodeIdentity {
  const git = (args: string[]) => execFileSync("git", ["-C", dir, ...args], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
  const root = git(["rev-parse", "--show-toplevel"]);
  return { head: git(["rev-parse", "HEAD"]), dirty: git(["status", "--porcelain", "--", join(root, ".sandcastle")]) };
}

// ── 目录与文件 ────────────────────────────────────────────────────────────────
export interface AdmissionContext { imageIdOf?: ImageIdOf; dir?: string; env?: NodeJS.ProcessEnv; identity?: CodeIdentity; now?: Date; binding?: string; rows?: readonly AcceptanceRow[]; roots?: readonly string[] }

/** 生产路径只用宿主固定目录；dir 仅供单元测试与主控 CLI 的显式参数，且同样经过 controlledDir 校验。 */
export function resolveDir(ctx: { dir?: string; roots?: readonly string[] } = {}, create = false): string {
  if (ctx.dir === undefined) assertHomeNotRedirected(); // 默认（生产）路径：HOME 被重定向即拒绝
  return controlledDir(ctx.dir ?? acceptanceRoot(), { create, roots: ctx.roots });
}
const receiptPath = (dir: string) => join(dir, "receipt.json");
const acceptedPath = (dir: string) => join(dir, "accepted.json");
const usedPath = (dir: string, nonce: string, row: AcceptanceRow) => join(dir, `used-${nonce}-${row}.json`);
const checkpointPath = (dir: string, nonce: string, row: AcceptanceRow, binding: string) => join(dir, `cp-${nonce}-${row}-${bindingHash(binding)}.json`);
const resumePrefix = (nonce: string, row: AcceptanceRow, binding: string) => `resume-${nonce}-${row}-${bindingHash(binding)}-`;

/** 先写临时文件并 fsync，再用 link 原子发布：发布前最终名不可见，发布后内容必然完整；目标已存在即 EEXIST。 */
export function publishOnce(path: string, value: unknown): void {
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

const imageIdField = (v: unknown): string => { const s = str(v, "imageId"); if (!IMAGE_ID.test(s)) throw new Error("imageId 不是 sha256 镜像 Id"); return s; };
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
    image: str(raw.image, "image"), imageId: imageIdField(raw.imageId), rows, ...(swarmTicket !== undefined ? { swarmTicket: swarmTicket as number } : {}),
    approval: str(raw.approval, "approval"), issuedAt: raw.issuedAt as string, expiresAt: raw.expiresAt as string,
  };
}
export function parseUsed(raw: unknown, receipt: AcceptanceReceipt, row: AcceptanceRow): UsedMarker {
  if (!isObj(raw) || raw.schema !== 2) throw new Error("used 记录结构无效");
  const m: UsedMarker = { schema: 2, nonce: str(raw.nonce, "nonce"), row: rowOf(raw.row), head: hex40(raw.head, "head"), contract: str(raw.contract, "contract"), binding: str(raw.binding, "binding"), at: str(raw.at, "at"), pid: Number(raw.pid) };
  time(m.at, "at");
  if (m.nonce !== receipt.nonce || m.row !== row || m.head !== receipt.head || m.contract !== receipt.contract) throw new Error("used 记录与 receipt 不一致");
  checkBinding(receipt, row, m.binding);
  return m;
}
/** 第 13 行：必须逐字等于 receipt 绑定的专用验收票；第 14/15 行：必须是可解析的归并检查点绑定。 */
function checkBinding(receipt: AcceptanceReceipt, row: AcceptanceRow, binding: string): void {
  if (row === "13") { if (binding !== swarmBinding(receipt.swarmTicket!)) throw new Error(`第 13 行 binding 不是专用验收票 #${receipt.swarmTicket}`); return; }
  parseMergeBinding(binding);
}
/** 同一验收行的检查点记录：必须属于同一 run 与基线（第 14/15 行首个 used 记录是锚点）。 */
function loadCheckpoints(dir: string, receipt: AcceptanceReceipt, row: AcceptanceRow, anchor: UsedMarker): string[] {
  if (row === "13") return [];
  const a = parseMergeBinding(anchor.binding);
  const out: string[] = [];
  for (const f of readdirSync(dir).filter((n) => n.startsWith(`cp-${receipt.nonce}-${row}-`) && !n.includes(".tmp-")).sort()) {
    const raw = readStrict(join(dir, f));
    const m = parseUsed(raw, receipt, row);
    const cp = parseMergeBinding(m.binding);
    if (cp.runId !== a.runId || cp.base !== a.base || cp.instance !== a.instance) throw new Error(`检查点记录 ${f} 不属于第 ${row} 行的同一归并 run`);
    if (f !== `cp-${receipt.nonce}-${row}-${bindingHash(m.binding)}.json`) throw new Error(`检查点记录 ${f} 文件名与 binding 不符`);
    out.push(m.binding);
  }
  return out;
}
/** 行内全部已占用绑定（锚点 + 检查点），排序后用于快照比较。 */
function rowBindings(dir: string, receipt: AcceptanceReceipt, row: AcceptanceRow): string[] | null {
  const anchor = loadUsed(dir, receipt, row);
  if (!anchor) return null;
  return [...new Set([anchor.binding, ...loadCheckpoints(dir, receipt, row, anchor)])].sort();
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
export interface IssueOptions { imageIdOf?: ImageIdOf; dir?: string; roots?: readonly string[]; image: string; approval: string; rows?: readonly AcceptanceRow[]; swarmTicket?: number; ttlHours?: number; now?: Date; identity?: CodeIdentity }

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
    schema: 2, ticket: ACCEPTANCE_TICKET, nonce: randomUUID(), head: id.head, contract: contractDigest(), image: o.image, imageId: imageIdOf(o, o.image), rows,
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
/** 模型名 → 家族：显式前缀表。路由别名与未知名称一律拒绝，不能靠“看起来不同”判异家族。 */
const FAMILIES: ReadonlyArray<[RegExp, string]> = [
  [/^(gpt|codex|chatgpt|o\d)(-|\.|$|\d)/, "openai"], [/^claude(-|$)/, "anthropic"], [/^gemini(-|$)/, "google"],
  [/^kimi(-|$)/, "moonshot"], [/^glm(-|$)/, "zhipu"], [/^qwen/, "alibaba"], [/^deepseek(-|$)/, "deepseek"],
];
const ROUTE_ALIAS = /^(auto|openrouter|router|local)(-|$)|-free$|-auto$/;
export function modelFamily(model: string): string {
  const name = (model.trim().split("/").at(-1) ?? "").toLowerCase().replaceAll("_", "-");
  if (!name) throw new Error("审阅模型名为空");
  if (ROUTE_ALIAS.test(name)) throw new Error(`审阅模型 ${model} 是路由别名，无法确定真实家族，拒绝`);
  const hit = FAMILIES.find(([re]) => re.test(name));
  if (!hit) throw new Error(`审阅模型 ${model} 不在已知家族表内，拒绝`);
  return hit[1];
}

/** 外部异家族独审报告的机读格式（三行，顺序固定）：
 *   VERDICT: PASS
 *   HEAD: <被验收的 40 位候选 SHA，须等于 receipt.head>
 *   REVIEWER_MODEL: <真实模型名，家族须不同于 MAIN_MODEL>
 * 首个非空行必须逐字等于 `VERDICT: PASS`；全文只能有这一行 VERDICT，HEAD 与 REVIEWER_MODEL 各出现一次。其后可跟自由正文。 */
export function parseCrossFamilyReview(text: string, expectedHead: string): { head: string; model: string } {
  const lines = text.split(/\r?\n/);
  const first = lines.find((l) => l.trim() !== "");
  if (first !== "VERDICT: PASS") throw new Error("外部独审报告首个非空行必须逐字等于 `VERDICT: PASS`");
  if (lines.filter((l) => /^\s*VERDICT\s*:/i.test(l)).length !== 1) throw new Error("外部独审报告含多个 VERDICT 行");
  const one = (name: string) => {
    const hits = lines.filter((l) => new RegExp(`^\\s*${name}\\s*:`, "i").test(l));
    if (hits.length !== 1) throw new Error(`外部独审报告 ${name} 必须恰好出现一次（实际 ${hits.length}）`);
    return hits[0]!.replace(/^[^:]*:/, "").trim();
  };
  const head = one("HEAD"); const model = one("REVIEWER_MODEL");
  if (!/^[0-9a-f]{40}$/.test(head)) throw new Error("外部独审报告 HEAD 不是 40 位 SHA");
  if (head !== expectedHead) throw new Error(`外部独审报告 HEAD ${head.slice(0, 12)} 不是被验收的候选 ${expectedHead.slice(0, 12)}`);
  const reviewer = modelFamily(model); const author = modelFamily(MAIN_MODEL);
  if (reviewer === author) throw new Error(`审阅模型 ${model} 与作者 ${MAIN_MODEL} 同属 ${author} 家族，不算异家族独审`);
  return { head, model };
}
/** 登记与每次放行共用的读取入口。 */
export function readCrossFamilyReview(file: string, expectedHead: string): CrossFamilyReview {
  if (!file || !existsSync(file)) throw new Error("缺少外部异家族独审报告文件");
  const parsed = parseCrossFamilyReview(readFileSync(file, "utf8"), expectedHead);
  return { file, ...parsed };
}

export interface RecordOptions {
  imageIdOf?: ImageIdOf; dir?: string; roots?: readonly string[]; image: string; approval?: string; evidence: Record<AcceptanceRow, string>;
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
  if (receipt.imageId !== imageIdOf(o, o.image)) throw new Error("receipt 与当前镜像 Id 不符（镜像已漂移）");
  if ((o.now ?? new Date()).getTime() >= Date.parse(receipt.expiresAt)) throw new Error("receipt 已过期，不能登记验收");
  if (o.approval !== undefined && o.approval !== receipt.approval) throw new Error("批准引用与 receipt 不一致");
  if (!ACCEPTANCE_ROWS.every((r) => receipt.rows.includes(r))) throw new Error("receipt 范围不含 13–15 全部三行，不能登记完整验收");
  const bindings = {} as Record<AcceptanceRow, string[]>;
  for (const row of ACCEPTANCE_ROWS) {
    const b = rowBindings(dir, receipt, row);
    if (!b) throw new Error(`第 ${row} 行尚未经受控入口真实运行`);
    bindings[row] = b;
    if (!o.evidence[row] || !existsSync(o.evidence[row])) throw new Error(`第 ${row} 行缺少证据文件`);
  }
  const crossFamily = readCrossFamilyReview(o.crossFamilyReview, receipt.head);
  if (receipt.head !== id.head) throw new Error("receipt.head 与当前执行代码不一致");
  const record: AcceptedRecord = {
    schema: 2, ticket: ACCEPTANCE_TICKET, nonce: receipt.nonce, head: receipt.head, contract: receipt.contract, image: receipt.image, imageId: receipt.imageId, approval: receipt.approval,
    issuedAt: receipt.issuedAt, expiresAt: receipt.expiresAt, acceptedAt: (o.now ?? new Date()).toISOString(), evidence: o.evidence, crossFamily, bindings,
  };
  publishOnce(acceptedPath(dir), record);
  return record;
}

function parseAccepted(raw: unknown): AcceptedRecord {
  if (!isObj(raw) || raw.schema !== 2 || raw.ticket !== ACCEPTANCE_TICKET || !isObj(raw.evidence) || !isObj(raw.crossFamily) || !isObj(raw.bindings)) throw new Error("accepted 结构无效");
  const evidence = Object.fromEntries(ACCEPTANCE_ROWS.map((r) => [r, str((raw.evidence as Record<string, unknown>)[r], `evidence.${r}`)])) as Record<AcceptanceRow, string>;
  time(raw.issuedAt, "issuedAt"); time(raw.expiresAt, "expiresAt"); time(raw.acceptedAt, "acceptedAt");
  return {
    schema: 2, ticket: ACCEPTANCE_TICKET, nonce: str(raw.nonce, "nonce"), head: hex40(raw.head, "head"), contract: str(raw.contract, "contract"), image: str(raw.image, "image"), imageId: imageIdField(raw.imageId),
    approval: str(raw.approval, "approval"), issuedAt: raw.issuedAt as string, expiresAt: raw.expiresAt as string, acceptedAt: raw.acceptedAt as string, evidence,
    crossFamily: { file: str(raw.crossFamily.file, "crossFamily.file"), head: hex40(raw.crossFamily.head, "crossFamily.head"), model: str(raw.crossFamily.model, "crossFamily.model") },
    bindings: Object.fromEntries(ACCEPTANCE_ROWS.map((r) => {
      const v = (raw.bindings as Record<string, unknown> | undefined)?.[r];
      if (!Array.isArray(v) || !v.length || v.some((x) => typeof x !== "string" || !x)) throw new Error(`bindings.${r} 无效`);
      return [r, [...(v as string[])].sort()];
    })) as Record<AcceptanceRow, string[]>,
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
    if (record.nonce !== receipt.nonce || record.head !== receipt.head || record.contract !== receipt.contract || record.image !== receipt.image || record.imageId !== receipt.imageId
      || record.approval !== receipt.approval || record.expiresAt !== receipt.expiresAt || record.issuedAt !== receipt.issuedAt) return null;
    if (receipt.head !== id.head || receipt.contract !== contractDigest() || receipt.image !== image) return null;
    if (receipt.imageId !== imageIdOf(ctx, image)) return null;
    if ((ctx.now ?? new Date()).getTime() >= Date.parse(receipt.expiresAt)) return null;
    if (!ACCEPTANCE_ROWS.every((r) => receipt.rows.includes(r))) return null;
    for (const row of ACCEPTANCE_ROWS) {
      const b = rowBindings(dir, receipt, row);
      if (!b || JSON.stringify(b) !== JSON.stringify(record.bindings[row])) return null;
      if (!existsSync(record.evidence[row])) return null;
    }
    // 外部独审报告每次重读：VERDICT、异家族、HEAD 与 receipt.head、当前执行代码三者一致，且与登记快照一致。
    const review = readCrossFamilyReview(record.crossFamily.file, receipt.head);
    if (review.head !== id.head || review.model !== record.crossFamily.model || record.crossFamily.head !== receipt.head) return null;
    return record;
  } catch { return null; }
}

// ── 准入 ──────────────────────────────────────────────────────────────────────
/** imageId＝准入核对过的 sha256 镜像 Id：调用方必须原样用于启动容器（docker run 用 Id，不用 tag），否则准入与启动之间 tag 可被重指。 */
export interface Admission { imageId: string; receipt?: AcceptanceReceipt; row?: AcceptanceRow; accepted?: AcceptedRecord; resumed?: boolean; consume: () => void }

function refuse(why: string): never {
  throw new Error(`[FAIL-loud #1617] Docker 路径尚无精确验收 receipt：${why}。默认入口继续拒绝，不认领、不扩大容器权限；13–15 仅限受控一次性入口。`);
}

/** 先认已登记且仍有效的验收；否则只认受控 receipt。两者都不成立即拒绝。 */
export function checkAdmission(image: string, ctx: AdmissionContext = {}): Admission {
  const accepted = acceptedFor(image, ctx);
  if (accepted) return { imageId: accepted.imageId, accepted, consume: () => undefined };
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
  let currentImageId: string;
  try { currentImageId = imageIdOf(ctx, image); } catch (e) { return refuse(`镜像 Id 无法确认：${e instanceof Error ? e.message : String(e)}`); }
  if (receipt.imageId !== currentImageId) refuse(`镜像 Id 漂移（receipt ${receipt.imageId.slice(0, 19)} ≠ 当前 ${currentImageId.slice(0, 19)}）`);
  if (!receipt.rows.includes(row!)) refuse(`批准范围不含第 ${row} 行`);
  if ((ctx.now ?? new Date()).getTime() >= Date.parse(receipt.expiresAt)) refuse("receipt 已过期");
  if (row === "13" && ctx.binding !== swarmBinding(receipt.swarmTicket!)) refuse(`第 13 行只允许专用验收票 #${receipt.swarmTicket}`);
  try { checkBinding(receipt, row!, ctx.binding!); } catch (e) { return refuse(e instanceof Error ? e.message : String(e)); }
  const r = row!; const binding = ctx.binding!;
  let used: UsedMarker | null; let known: string[] = [];
  try { used = loadUsed(dir, receipt, r); if (used) known = [used.binding, ...loadCheckpoints(dir, receipt, r, used)]; }
  catch (e) { return refuse(e instanceof Error ? e.message : String(e)); }
  const marker = (): UsedMarker => ({ schema: 2, nonce: receipt.nonce, row: r, head: receipt.head, contract: receipt.contract, binding, at: new Date().toISOString(), pid: process.pid });
  const resume = (): Admission => {
    const prefix = resumePrefix(receipt.nonce, r, binding);
    const count = readdirSync(dir).filter((f) => f.startsWith(prefix) && !f.includes(".tmp-")).length;
    if (count >= MAX_RESUMES) refuse(`第 ${r} 行同一检查点的恢复次数已达上限 ${MAX_RESUMES}`);
    return { imageId: currentImageId, receipt, row: r, resumed: true, consume: () => {
      try { publishOnce(join(dir, `${prefix}${count + 1}.json`), { schema: 2, nonce: receipt.nonce, row: r, binding, at: new Date().toISOString(), pid: process.pid }); }
      catch { refuse(`第 ${r} 行恢复记录被并发写入`); }
    } };
  };
  if (!used) {
    return { imageId: currentImageId, receipt, row: r, consume: () => {
      try { publishOnce(usedPath(dir, receipt.nonce, r), marker()); }
      catch (e) { refuse(`第 ${r} 行已被并发占用（${e instanceof Error ? e.message : String(e)}）`); }
    } };
  }
  if (r === "13") refuse("第 13 行是一次性的，已被占用；中断后须由主控退役并重新签发 receipt，不允许同票重跑冒充恢复");
  if (known.includes(binding)) return resume(); // 第 14/15 行：同一 run 实例、同一检查点的中断后续做
  // 第 14/15 行：同一归并 run 与基线内的新冲突检查点，单独绑定、单独留痕；别的 run 或基线一律拒绝。
  const anchor = parseMergeBinding(used.binding); const mine = parseMergeBinding(binding);
  if (anchor.runId !== mine.runId || anchor.base !== mine.base || anchor.instance !== mine.instance) refuse(`第 ${r} 行已被另一运行/检查点使用，新运行不得复用`);
  if (known.length >= MAX_CHECKPOINTS) refuse(`第 ${r} 行同一 run 的检查点数已达上限 ${MAX_CHECKPOINTS}`);
  return { imageId: currentImageId, receipt, row: r, consume: () => {
    try { publishOnce(checkpointPath(dir, receipt.nonce, r, binding), marker()); }
    catch (e) { refuse(`第 ${r} 行该检查点已被并发占用（${e instanceof Error ? e.message : String(e)}）`); }
  } };
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
    if (!image) throw new Error("用法：accept --image <镜像> [--approval <引用>] --evidence13 <文件> --evidence14 <文件> --evidence15 <文件> --cross-family-review <文件>（三行机读：首行 `VERDICT: PASS`，`HEAD: <receipt.head>`，`REVIEWER_MODEL: <异家族真实模型>`）");
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
