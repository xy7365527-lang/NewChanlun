/** #1742 方案B 的 Docker 准入：矩阵观测只证明“容器合同成立”，不等于“允许认领”。
 * 默认入口无精确 acceptance 事实一律 FAIL-loud。事实全部存放在宿主固定目录（acceptance-dir.mts），
 * 工蜂 worktree 与容器挂载触不到；receipt 绑定代码 head、合同摘要、镜像和用户批准，
 * 验收行 13/14/15 各自一次性，且绑定到具体运行（13=验收票，14/15=归并 run+检查点），同一绑定的中断后续做不重复占用。 */
import { execFileSync } from "node:child_process";
import { createHash, randomUUID } from "node:crypto";
import { closeSync, existsSync, fsyncSync, linkSync, lstatSync, mkdirSync, openSync, readFileSync, readdirSync, realpathSync, renameSync, unlinkSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { isDeepStrictEqual } from "node:util";
import { acceptanceRoot, assertHomeNotRedirected, controlledDir } from "./acceptance-dir.mts";
import { CONTAINER_ENV_ALLOW, DEFAULT_CAP_MASK, DOCKER_INNER_SANDBOX, PROBE_SCRIPT, PROTECTED_SUBPATHS, imageEnvSecrets } from "./docker-isolation.mts";
import { DrainFrozenError, withFileLock, withFrozenAdmission } from "./drain-control.mts";
import { MAIN_MODEL } from "./main-model.mts";
import { MERGE_CHECK_ID, validateMergeManifest, type MergeManifest, type MergeState } from "./managed-merge.mts";

export const ACCEPTANCE_TICKET = 1742;
export const ACCEPTANCE_ROWS = ["13", "14", "15"] as const;
export type AcceptanceRow = (typeof ACCEPTANCE_ROWS)[number];
export const MERGE_ROWS = ["14", "15"] as const;
export type MergeRow = (typeof MERGE_ROWS)[number];
export const ROW_ENV = "SANDCASTLE_DOCKER_ACCEPTANCE_ROW";
export const MAX_TTL_HOURS = 720;
export const MAX_RESUMES = 5;
export const MAX_CHECKPOINTS = 16;

export const CONTRACT_ID = "1742-docker-plan-b-v4-frozen-deployment-v1";
export const BINDING_RULES = ["receipt-purpose-v1", "frozen-row13-fresh-branch-v1", "merge-plan-sealed-v1", "merge-run-admission-then-conflict-v1", "row-recovery-checkpoint-v1"] as const;
/** 合同摘要：改任一项（受保护路径、内层沙箱模式、cap 上界、探针脚本、无 security 选项、容器 env 白名单、绑定规则版本）即新合同，旧 receipt 作废。 */
export function contractDigest(): string {
  const canonical = JSON.stringify({
    id: CONTRACT_ID, protectedSubpaths: PROTECTED_SUBPATHS, innerSandbox: DOCKER_INNER_SANDBOX,
    capMask: DEFAULT_CAP_MASK.toString(16), securityOpts: [], containerEnvAllow: CONTAINER_ENV_ALLOW, imageIdBound: true, probe: PROBE_SCRIPT,
    receiptSchema: 3, bindingRules: BINDING_RULES,
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

export interface PartnerFixture { issue: number; branch: string; sha: string; parent: string; review: string; approval: string }
export type ReceiptPurpose =
  | { kind: "controlled" }
  | { kind: "frozen-deployment"; generation: number; repo: string; partner: PartnerFixture };
export interface AcceptanceReceipt {
  schema: 3; ticket: number; nonce: string; head: string; contract: string; image: string; imageId: string;
  rows: AcceptanceRow[]; swarmTicket?: number; approval: string; issuedAt: string; expiresAt: string; purpose: ReceiptPurpose;
}
export interface UsedMarker { schema: 3; nonce: string; row: AcceptanceRow; head: string; contract: string; binding: string; at: string; pid: number }
export interface CrossFamilyReview { file: string; head: string; model: string }
export interface FrozenRoundSummary { candidate: string; merges: Record<MergeRow, { runId: string; instance: string; head: string }> }
export interface AcceptedRecord {
  schema: 3; ticket: number; nonce: string; head: string; contract: string; image: string; imageId: string; approval: string;
  issuedAt: string; expiresAt: string; acceptedAt: string; evidence: Record<AcceptanceRow, string>; crossFamily: CrossFamilyReview;
  /** 登记时每行已占用的运行绑定快照（第 14/15 行的 run+检查点登记前未知，只能在此固定）；acceptedFor 逐字核对。 */
  bindings: Record<AcceptanceRow, string[]>;
  round?: FrozenRoundSummary;
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
export interface AdmissionContext { imageIdOf?: ImageIdOf; dir?: string; env?: NodeJS.ProcessEnv; identity?: CodeIdentity; now?: Date; binding?: string; rows?: readonly AcceptanceRow[]; roots?: readonly string[]; repo?: string }

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
export const controlLockPath = (dir: string) => join(dir, "control.lock");

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
const posInt = (v: unknown, name: string): number => { if (!Number.isSafeInteger(v) || (v as number) <= 0) throw new Error(`${name} 不是正整数`); return v as number; };
export function parsePartner(raw: unknown): PartnerFixture {
  if (!isObj(raw)) throw new Error("固定伙伴输入结构无效");
  const p: PartnerFixture = { issue: posInt(raw.issue, "partner.issue"), branch: str(raw.branch, "partner.branch"), sha: hex40(raw.sha, "partner.sha"),
    parent: hex40(raw.parent, "partner.parent"), review: str(raw.review, "partner.review"), approval: str(raw.approval, "partner.approval") };
  if (p.branch !== `sandcastle/issue-${p.issue}`) throw new Error("固定伙伴分支必须为 sandcastle/issue-<票号>");
  if (Object.keys(raw).some((k) => !(k in p))) throw new Error("固定伙伴输入含未知字段");
  return p;
}
function parsePurpose(raw: unknown): ReceiptPurpose {
  if (!isObj(raw)) throw new Error("receipt.purpose 缺失");
  if (raw.kind === "controlled" && Object.keys(raw).length === 1) return { kind: "controlled" };
  if (raw.kind === "frozen-deployment") {
    const repo = str(raw.repo, "purpose.repo");
    if (!repo.startsWith("/")) throw new Error("purpose.repo 必须是绝对真实路径");
    return { kind: "frozen-deployment", generation: posInt(raw.generation, "purpose.generation"), repo, partner: parsePartner(raw.partner) };
  }
  throw new Error("receipt.purpose 未知");
}
export function parseReceipt(raw: unknown): AcceptanceReceipt {
  if (!isObj(raw) || raw.schema !== 3 || raw.ticket !== ACCEPTANCE_TICKET) throw new Error("receipt 结构无效");
  if (!Array.isArray(raw.rows) || !raw.rows.length) throw new Error("receipt.rows 无效");
  const rows = raw.rows.map(rowOf);
  const swarmTicket = raw.swarmTicket;
  if (swarmTicket !== undefined && (!Number.isSafeInteger(swarmTicket) || (swarmTicket as number) <= 0)) throw new Error("receipt.swarmTicket 无效");
  if (rows.includes("13") && swarmTicket === undefined) throw new Error("第 13 行必须绑定专用验收票");
  time(raw.issuedAt, "issuedAt"); time(raw.expiresAt, "expiresAt");
  const purpose = parsePurpose(raw.purpose);
  if (purpose.kind === "frozen-deployment" && (swarmTicket === undefined || !ACCEPTANCE_ROWS.every((r) => rows.includes(r)) || rows.length !== ACCEPTANCE_ROWS.length)) throw new Error("冻结验收 receipt 必须绑定专票且恰含 13–15 三行");
  return {
    schema: 3, ticket: ACCEPTANCE_TICKET, nonce: str(raw.nonce, "nonce"), head: hex40(raw.head, "head"), contract: str(raw.contract, "contract"),
    image: str(raw.image, "image"), imageId: imageIdField(raw.imageId), rows, ...(swarmTicket !== undefined ? { swarmTicket: swarmTicket as number } : {}),
    approval: str(raw.approval, "approval"), issuedAt: raw.issuedAt as string, expiresAt: raw.expiresAt as string, purpose,
  };
}
export function parseUsed(raw: unknown, receipt: AcceptanceReceipt, row: AcceptanceRow): UsedMarker {
  if (!isObj(raw) || raw.schema !== 3) throw new Error("used 记录结构无效");
  const m: UsedMarker = { schema: 3, nonce: str(raw.nonce, "nonce"), row: rowOf(raw.row), head: hex40(raw.head, "head"), contract: str(raw.contract, "contract"), binding: str(raw.binding, "binding"), at: str(raw.at, "at"), pid: Number(raw.pid) };
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
    schema: 3, ticket: ACCEPTANCE_TICKET, nonce: randomUUID(), head: id.head, contract: contractDigest(), image: o.image, imageId: imageIdOf(o, o.image), rows,
    ...(o.swarmTicket !== undefined ? { swarmTicket: o.swarmTicket } : {}),
    approval: o.approval.trim(), issuedAt: now.toISOString(), expiresAt: new Date(now.getTime() + ttl * 3_600_000).toISOString(), purpose: { kind: "controlled" },
  };
  const dir = resolveDir(o, true);
  publishOnce(receiptPath(dir), receipt);
  return receipt;
}

export async function retireReceipt(ctx: { dir?: string; roots?: readonly string[] } = {}): Promise<string | null> {
  const dir = resolveDir(ctx);
  return withFileLock(controlLockPath(dir), () => retireNow(dir));
}
function retireNow(dir: string): string | null {
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
  const round = receipt.purpose.kind === "frozen-deployment" ? verifyFrozenRound(dir, receipt, bindings) : undefined;
  const record: AcceptedRecord = {
    schema: 3, ticket: ACCEPTANCE_TICKET, nonce: receipt.nonce, head: receipt.head, contract: receipt.contract, image: receipt.image, imageId: receipt.imageId, approval: receipt.approval,
    issuedAt: receipt.issuedAt, expiresAt: receipt.expiresAt, acceptedAt: (o.now ?? new Date()).toISOString(), evidence: o.evidence, crossFamily, bindings,
    ...(round ? { round } : {}),
  };
  publishOnce(acceptedPath(dir), record);
  return record;
}

function parseAccepted(raw: unknown): AcceptedRecord {
  if (!isObj(raw) || raw.schema !== 3 || raw.ticket !== ACCEPTANCE_TICKET || !isObj(raw.evidence) || !isObj(raw.crossFamily) || !isObj(raw.bindings)) throw new Error("accepted 结构无效");
  const evidence = Object.fromEntries(ACCEPTANCE_ROWS.map((r) => [r, str((raw.evidence as Record<string, unknown>)[r], `evidence.${r}`)])) as Record<AcceptanceRow, string>;
  time(raw.issuedAt, "issuedAt"); time(raw.expiresAt, "expiresAt"); time(raw.acceptedAt, "acceptedAt");
  return {
    ...(raw.round !== undefined ? { round: raw.round as FrozenRoundSummary } : {}),
    schema: 3, ticket: ACCEPTANCE_TICKET, nonce: str(raw.nonce, "nonce"), head: hex40(raw.head, "head"), contract: str(raw.contract, "contract"), image: str(raw.image, "image"), imageId: imageIdField(raw.imageId),
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
    const live = {} as Record<AcceptanceRow, string[]>;
    for (const row of ACCEPTANCE_ROWS) {
      const b = rowBindings(dir, receipt, row);
      if (!b || JSON.stringify(b) !== JSON.stringify(record.bindings[row])) return null;
      if (!existsSync(record.evidence[row])) return null;
      live[row] = b;
    }
    if (receipt.purpose.kind === "frozen-deployment" ? !isDeepStrictEqual(verifyFrozenRound(dir, receipt, live), record.round) : record.round !== undefined) return null;
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
  return admitRow(image, ctx, row!, ctx.binding!, "controlled");
}

export function frozenRowAdmission(image: string, ctx: AdmissionContext, row: AcceptanceRow, binding: string): Admission {
  return admitRow(image, ctx, row, binding, "frozen-deployment");
}

function admitRow(image: string, ctx: AdmissionContext, row: AcceptanceRow, binding: string, purpose: ReceiptPurpose["kind"]): Admission {
  let dir: string, receipt: AcceptanceReceipt;
  try { dir = resolveDir(ctx); } catch (e) { return refuse(`验收目录不合格：${e instanceof Error ? e.message : String(e)}`); }
  try { receipt = loadReceipt(dir); } catch (e) { return refuse(e instanceof Error ? e.message : String(e)); }
  if (receipt.purpose.kind !== purpose) refuse(purpose === "controlled" ? "当前 receipt 是冻结部署验收用途，只能经 deployment-acceptance 专用入口消费" : "当前 receipt 不是冻结部署验收用途");
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
  if (row === "13" && binding !== swarmBinding(receipt.swarmTicket!)) refuse(`第 13 行只允许专用验收票 #${receipt.swarmTicket}`);
  try { checkBinding(receipt, row, binding); } catch (e) { return refuse(e instanceof Error ? e.message : String(e)); }
  const r = row;
  let used: UsedMarker | null; let known: string[] = [];
  try { used = loadUsed(dir, receipt, r); if (used) known = [used.binding, ...loadCheckpoints(dir, receipt, r, used)]; }
  catch (e) { return refuse(e instanceof Error ? e.message : String(e)); }
  const marker = (): UsedMarker => ({ schema: 3, nonce: receipt.nonce, row: r, head: receipt.head, contract: receipt.contract, binding, at: new Date().toISOString(), pid: process.pid });
  const resume = (): Admission => {
    const prefix = resumePrefix(receipt.nonce, r, binding);
    const count = readdirSync(dir).filter((f) => f.startsWith(prefix) && !f.includes(".tmp-")).length;
    if (count >= MAX_RESUMES) refuse(`第 ${r} 行同一检查点的恢复次数已达上限 ${MAX_RESUMES}`);
    return { imageId: currentImageId, receipt, row: r, resumed: true, consume: () => {
      try { publishOnce(join(dir, `${prefix}${count + 1}.json`), { schema: 3, nonce: receipt.nonce, row: r, binding, at: new Date().toISOString(), pid: process.pid }); }
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

export interface DeploymentSelection { ticket: number; head: string; nonce: string; row: AcceptanceRow }
export interface RoundIdentity { nonce: string; head: string; contract: string; imageId: string; swarmTicket: number; generation: number; expiresAt: string }
type FrozenPurpose = Extract<ReceiptPurpose, { kind: "frozen-deployment" }>;
export interface FrozenRound { dir: string; receipt: AcceptanceReceipt; purpose: FrozenPurpose; ticket: number; roundDir: string; identity: RoundIdentity; control: string }
export type RoundContext = Pick<AdmissionContext, "dir" | "roots" | "identity" | "imageIdOf" | "now" | "repo">;

const NONCE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
export const roundDirOf = (dir: string, nonce: string) => join(dir, "rounds", nonce);
export const ROUND_FILES = {
  candidate: "row13-candidate.json", plan: "merge-plan.json", log: join("logs", "workers.jsonl"), notes: "notes.jsonl",
  admission: (row: MergeRow) => `merge-${row}-admission.json`, result: (row: MergeRow) => `merge-${row}-result.json`,
  evidence: (row: MergeRow) => `merge-${row}`, runLock: (row: AcceptanceRow) => `row${row}.lock`,
} as const;
export const ticketBranch = (ticket: number) => `sandcastle/issue-${ticket}`;
export const mergeRunId = (row: MergeRow, nonce: string) => `1742-acc-r${row}-${nonce}`;

export function parseSelection(raw: { ticket?: string; head?: string; nonce?: string; row?: string }): DeploymentSelection {
  const ticket = Number(raw.ticket);
  if (!raw.ticket || !Number.isSafeInteger(ticket) || ticket <= 0 || String(ticket) !== raw.ticket) throw new Error("--ticket 必须是正整数票号");
  if (!/^[0-9a-f]{40}$/.test(raw.head ?? "")) throw new Error("--head 必须是 40 位部署 SHA");
  if (!NONCE.test(raw.nonce ?? "")) throw new Error("--nonce 必须是本轮 receipt 的 UUID");
  return { ticket, head: raw.head!, nonce: raw.nonce!, row: rowOf(raw.row) };
}

export function moduleRepo(dir: string = dirname(fileURLToPath(import.meta.url))): string {
  return realpathSync(execFileSync("git", ["-C", dir, "rev-parse", "--show-toplevel"], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim());
}
const repoGit = (repo: string, args: string[]) => execFileSync("git", ["-C", repo, ...args], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
export function branchExists(repo: string, branch: string): boolean {
  try { repoGit(repo, ["rev-parse", "--verify", "-q", `refs/heads/${branch}`]); return true; } catch { return false; }
}

export function prebindPath(root: string, segments: readonly string[]): string {
  if (realpathSync(root) !== root) throw new Error(`预绑定根 ${root} 不是真实路径`);
  let cur = root;
  for (const [n, seg] of segments.entries()) {
    if (!/^[A-Za-z0-9.][A-Za-z0-9._-]*$/.test(seg) || seg === "." || seg === "..") throw new Error(`路径段 ${JSON.stringify(seg)} 不合规`);
    cur = join(cur, seg);
    let st;
    try { st = lstatSync(cur); } catch (e) { if ((e as NodeJS.ErrnoException).code === "ENOENT") return join(cur, ...segments.slice(n + 1)); throw e; }
    if (st.isSymbolicLink()) throw new Error(`预绑定路径 ${cur} 是符号链接，拒绝`);
    if (n < segments.length - 1 && !st.isDirectory()) throw new Error(`预绑定路径 ${cur} 不是目录，拒绝`);
  }
  return cur;
}
export function assertBoundPath(path: string): void {
  if (realpathSync(path) !== path) throw new Error(`实际位置 ${realpathSync(path)} 与预绑定 ${path} 不符`);
}
export const ticketWorktree = (repo: string, ticket: number) => prebindPath(repo, [".sandcastle", "worktrees", ticketBranch(ticket).replaceAll("/", "-")]);
const mergeWorktree = (repo: string, branch: string) => prebindPath(repo, [".sandcastle", "worktrees", branch.replaceAll("/", "-")]);

function roundOf(dir: string, receipt: AcceptanceReceipt): FrozenRound {
  if (receipt.purpose.kind !== "frozen-deployment" || receipt.swarmTicket === undefined) throw new Error("当前 receipt 不是冻结部署验收用途");
  const purpose = receipt.purpose;
  return {
    dir, receipt, purpose, ticket: receipt.swarmTicket, roundDir: roundDirOf(dir, receipt.nonce), control: controlLockPath(dir),
    identity: { nonce: receipt.nonce, head: receipt.head, contract: receipt.contract, imageId: receipt.imageId, swarmTicket: receipt.swarmTicket, generation: purpose.generation, expiresAt: receipt.expiresAt },
  };
}

export function inspectFrozenRound(sel: Omit<DeploymentSelection, "row">, ctx: Pick<RoundContext, "dir" | "roots"> = {}): FrozenRound {
  const dir = resolveDir(ctx);
  const round = roundOf(dir, loadReceipt(dir));
  if (sel.nonce !== round.receipt.nonce || sel.ticket !== round.ticket || sel.head !== round.receipt.head) throw new Error("选择与当前冻结验收轮不符");
  return round;
}
export function roundClaims(round: FrozenRound): Record<AcceptanceRow, { bindings: string[]; resumes: number } | null> {
  return Object.fromEntries(ACCEPTANCE_ROWS.map((row) => {
    const bindings = rowBindings(round.dir, round.receipt, row);
    const resumes = readdirSync(round.dir).filter((f) => f.startsWith(`resume-${round.receipt.nonce}-${row}-`) && !f.includes(".tmp-")).length;
    return [row, bindings ? { bindings, resumes } : null];
  })) as Record<AcceptanceRow, { bindings: string[]; resumes: number } | null>;
}

export function loadFrozenRound(sel: DeploymentSelection, ctx: RoundContext = {}): FrozenRound {
  const dir = resolveDir(ctx);
  const round = roundOf(dir, loadReceipt(dir));
  const { receipt } = round;
  if (sel.nonce !== receipt.nonce) throw new Error(`nonce ${sel.nonce} 不是当前冻结验收轮`);
  if (sel.ticket !== round.ticket) throw new Error(`专票 #${sel.ticket} 不是本轮绑定的 #${round.ticket}`);
  if (sel.head !== receipt.head) throw new Error(`部署 head ${sel.head.slice(0, 12)} 不是本轮 receipt.head ${receipt.head.slice(0, 12)}`);
  if (!receipt.rows.includes(sel.row)) throw new Error(`本轮范围不含第 ${sel.row} 行`);
  const id = ctx.identity ?? codeIdentity();
  if (id.dirty) throw new Error(".sandcastle 有未提交改动，代码不精确");
  if (id.head !== receipt.head) throw new Error(`执行代码 ${id.head.slice(0, 12)} 不是本轮部署 head`);
  if (receipt.contract !== contractDigest()) throw new Error("隔离合同已变化");
  if (realpathSync(ctx.repo ?? moduleRepo()) !== round.purpose.repo) throw new Error("执行仓库不是本轮绑定仓库");
  if (imageIdOf(ctx, receipt.image) !== receipt.imageId) throw new Error("镜像 Id 漂移");
  if ((ctx.now ?? new Date()).getTime() >= Date.parse(receipt.expiresAt)) throw new Error("冻结验收 receipt 已过期");
  controlledDir(round.roundDir, { roots: ctx.roots });
  return round;
}

const readFact = (path: string): unknown | null => (existsSync(path) ? readStrict(path) : null);
function sameRound(raw: Record<string, unknown>, round: FrozenRound, what: string): void {
  if (!isDeepStrictEqual(raw.round, round.identity)) throw new Error(`${what} 不属于本冻结验收轮`);
}

export interface RoundCandidate {
  schema: 1; round: RoundIdentity; ticket: number; base: string; branch: string; head: string; commits: string[];
  verification: { head: string; status: string; evidencePath?: string }; log: string; at: string;
}
export function publishCandidate(round: FrozenRound, c: Pick<RoundCandidate, "head" | "commits" | "verification">): RoundCandidate {
  const value: RoundCandidate = { schema: 1, round: round.identity, ticket: round.ticket, base: round.receipt.head, branch: ticketBranch(round.ticket), ...c,
    log: join(round.roundDir, ROUND_FILES.log), at: new Date().toISOString() };
  publishOnce(join(round.roundDir, ROUND_FILES.candidate), value);
  return value;
}
export function loadCandidate(round: FrozenRound): RoundCandidate | null {
  const raw = readFact(join(round.roundDir, ROUND_FILES.candidate));
  if (raw === null) return null;
  if (!isObj(raw) || raw.schema !== 1) throw new Error("候选结果结构无效");
  sameRound(raw, round, "候选结果");
  if (raw.ticket !== round.ticket || raw.base !== round.receipt.head || raw.branch !== ticketBranch(round.ticket)) throw new Error("候选结果与本轮专票/基线不符");
  hex40(raw.head, "candidate.head");
  return raw as unknown as RoundCandidate;
}

export interface BoundMergeRun { row: MergeRow; manifest: MergeManifest; repo: string; cwd: string; evidenceDir: string; checkpoint: string; instance: string; checkId: string }
export interface FrozenMergePlan { schema: 1; round: RoundIdentity; candidate: { head: string; review: CrossFamilyReview; controllerAdmission: string }; runs: Record<MergeRow, BoundMergeRun> }
export function loadMergePlan(round: FrozenRound): FrozenMergePlan | null {
  const raw = readFact(join(round.roundDir, ROUND_FILES.plan));
  if (raw === null) return null;
  if (!isObj(raw) || raw.schema !== 1 || !isObj(raw.runs) || !isObj(raw.candidate)) throw new Error("归并计划结构无效");
  sameRound(raw, round, "归并计划");
  const plan = raw as unknown as FrozenMergePlan;
  for (const row of MERGE_ROWS) {
    const run = plan.runs[row];
    if (!run || run.row !== row || run.manifest.runId !== mergeRunId(row, round.receipt.nonce) || run.repo !== round.purpose.repo || run.checkId !== MERGE_CHECK_ID
      || run.evidenceDir !== join(round.roundDir, ROUND_FILES.evidence(row)) || run.checkpoint !== join(run.evidenceDir, "checkpoint.json") || !run.instance) throw new Error(`归并计划第 ${row} 行身份无效`);
    validateMergeManifest(run.manifest);
  }
  return plan;
}

export interface MergeRunAdmission { schema: 1; round: RoundIdentity; run: BoundMergeRun; at: string; pid: number }
export function publishMergeAdmission(round: FrozenRound, run: BoundMergeRun): void {
  publishOnce(join(round.roundDir, ROUND_FILES.admission(run.row)), { schema: 1, round: round.identity, run, at: new Date().toISOString(), pid: process.pid } satisfies MergeRunAdmission);
}
export function loadMergeAdmission(round: FrozenRound, run: BoundMergeRun): boolean {
  const raw = readFact(join(round.roundDir, ROUND_FILES.admission(run.row)));
  if (raw === null) return false;
  if (!isObj(raw) || raw.schema !== 1) throw new Error(`第 ${run.row} 行 run 准入事实结构无效`);
  sameRound(raw, round, `第 ${run.row} 行 run 准入事实`);
  if (!isDeepStrictEqual(raw.run, run)) throw new Error(`第 ${run.row} 行 run 准入事实与封定计划不符`);
  return true;
}

export const rowConsumed = (round: FrozenRound, row: AcceptanceRow): boolean => loadUsed(round.dir, round.receipt, row) !== null;
function planBindings(round: FrozenRound, run: BoundMergeRun): string[] | null {
  const bindings = rowBindings(round.dir, round.receipt, run.row);
  for (const b of bindings ?? []) {
    const cp = parseMergeBinding(b);
    if (cp.runId !== run.manifest.runId || cp.base !== run.manifest.base || cp.instance !== run.instance) throw new Error(`第 ${run.row} 行占用绑定不属于封定 run/实例`);
  }
  return bindings;
}
export interface MergeRowResult {
  schema: 1; round: RoundIdentity; row: MergeRow; runId: string; instance: string; head: string;
  entries: Array<{ sha: string; head: string; kind: string }>; bindings: string[]; at: string;
}
export function publishMergeResult(round: FrozenRound, run: BoundMergeRun, state: MergeState): MergeRowResult {
  if (state.phase !== "done" || !isDeepStrictEqual(state.manifest, run.manifest)) throw new Error(`第 ${run.row} 行检查点不是本 run 的完成态`);
  const bindings = planBindings(round, run);
  if (!bindings) throw new Error(`第 ${run.row} 行归并已完成但未出现真实冲突消费，不满足该行`);
  const value: MergeRowResult = { schema: 1, round: round.identity, row: run.row, runId: run.manifest.runId, instance: run.instance, head: state.snapshot.head,
    entries: state.entries.map((e) => ({ sha: e.sha, head: e.head, kind: e.kind })), bindings, at: new Date().toISOString() };
  publishOnce(join(round.roundDir, ROUND_FILES.result(run.row)), value);
  return value;
}
export function loadMergeResult(round: FrozenRound, row: MergeRow): MergeRowResult | null {
  const raw = readFact(join(round.roundDir, ROUND_FILES.result(row)));
  if (raw === null) return null;
  if (!isObj(raw) || raw.schema !== 1 || raw.row !== row || !Array.isArray(raw.bindings)) throw new Error(`第 ${row} 行完成结果结构无效`);
  sameRound(raw, round, `第 ${row} 行完成结果`);
  return raw as unknown as MergeRowResult;
}

/** 不快照：每次读取当前文件。 */
export function currentCandidateSource(round: FrozenRound, c: { head: string; reviewFile: string; controllerAdmission: string }): CrossFamilyReview {
  try {
    const review = readCrossFamilyReview(c.reviewFile, c.head);
    if (!existsSync(c.controllerAdmission)) throw new Error("缺少主控准入记录");
    parseControllerAdmission(readFileSync(c.controllerAdmission, "utf8"), c.head, round.receipt.nonce);
    return review;
  } catch (e) { throw new Error(`候选来源证据当前无效：${e instanceof Error ? e.message : String(e)}`); }
}
export const verifyPlanSource = (round: FrozenRound, plan: FrozenMergePlan): CrossFamilyReview =>
  currentCandidateSource(round, { head: plan.candidate.head, reviewFile: plan.candidate.review.file, controllerAdmission: plan.candidate.controllerAdmission });

function verifyFrozenRound(dir: string, receipt: AcceptanceReceipt, bindings: Record<AcceptanceRow, string[]>): FrozenRoundSummary {
  const round = roundOf(dir, receipt);
  const candidate = loadCandidate(round);
  if (!candidate) throw new Error("本轮第 13 行没有已发布的候选结果");
  const plan = loadMergePlan(round);
  if (!plan || plan.candidate.head !== candidate.head) throw new Error("本轮归并计划缺失或不是基于本轮候选");
  verifyPlanSource(round, plan);
  if (!isDeepStrictEqual(bindings["13"], [swarmBinding(round.ticket)])) throw new Error("第 13 行绑定不是本轮专票");
  const merges = {} as FrozenRoundSummary["merges"];
  for (const row of MERGE_ROWS) {
    const run = plan.runs[row]; const result = loadMergeResult(round, row);
    if (!result || result.runId !== run.manifest.runId || result.instance !== run.instance) throw new Error(`第 ${row} 行没有本 run 的完成结果`);
    if (!isDeepStrictEqual(result.bindings, bindings[row]) || !isDeepStrictEqual(planBindings(round, run), bindings[row])) throw new Error(`第 ${row} 行完成结果与真实冲突绑定不符`);
    if (!result.entries.some((e) => e.sha === candidate.head) || result.entries.length !== run.manifest.inputs.length) throw new Error(`第 ${row} 行完成结果未覆盖封定输入`);
    merges[row] = { runId: result.runId, instance: result.instance, head: result.head };
  }
  return { candidate: candidate.head, merges };
}

function verifyPartner(repo: string, p: PartnerFixture, ticket: number): void {
  if (p.issue === ticket) throw new Error("固定伙伴不能是本轮专票");
  let parents: string;
  try { parents = repoGit(repo, ["rev-list", "--parents", "-n", "1", `${p.sha}^{commit}`]); } catch { throw new Error(`固定伙伴 ${p.sha.slice(0, 12)} 在本仓库不存在`); }
  if (parents !== `${p.sha} ${p.parent}`) throw new Error("固定伙伴的唯一父与批准输入不符");
  if (!existsSync(p.review)) throw new Error("固定伙伴独审报告不存在");
  const text = readFileSync(p.review, "utf8");
  if (text.split(/\r?\n/).find((l) => l.trim() !== "") !== "VERDICT: PASS" || !text.includes(p.sha)) throw new Error("固定伙伴独审报告不是指向该 SHA 的 PASS");
}

export interface FrozenIssueOptions {
  image: string; approval: string; swarmTicket: number; generation: number; partner: unknown; ttlHours?: number;
  dir?: string; roots?: readonly string[]; now?: Date; identity?: CodeIdentity; imageIdOf?: ImageIdOf; repo?: string; drainRoot?: string;
}
export async function issueFrozenReceipt(o: FrozenIssueOptions): Promise<AcceptanceReceipt> {
  const id = o.identity ?? codeIdentity();
  if (id.dirty) throw new Error(`.sandcastle 存在未提交改动，不能给不精确的代码签发 receipt：${id.dirty.slice(0, 200)}`);
  if (!o.approval.trim()) throw new Error("签发 receipt 必须带用户批准引用");
  if (!Number.isSafeInteger(o.swarmTicket) || o.swarmTicket <= 0) throw new Error("必须指定专用验收票（--swarm-ticket N）");
  if (!Number.isSafeInteger(o.generation) || o.generation <= 0) throw new Error("必须指定当前冻结代次（--generation N）");
  const ttl = o.ttlHours ?? 12;
  if (!(ttl > 0 && ttl <= MAX_TTL_HOURS)) throw new Error(`ttlHours 必须在 (0, ${MAX_TTL_HOURS}] 内`);
  const repo = realpathSync(o.repo ?? moduleRepo());
  const partner = parsePartner(o.partner);
  verifyPartner(repo, partner, o.swarmTicket);
  if (branchExists(repo, ticketBranch(o.swarmTicket)) || existsSync(ticketWorktree(repo, o.swarmTicket))) throw new Error(`专票分支 ${ticketBranch(o.swarmTicket)} 或其工位已存在，不能作为本轮新候选；先按原证据归档处置`);
  const imageId = imageIdOf(o, o.image);
  const dir = resolveDir(o, true);
  return withFrozenAdmission(() => {
    const now = o.now ?? new Date();
    const receipt: AcceptanceReceipt = {
      schema: 3, ticket: ACCEPTANCE_TICKET, nonce: randomUUID(), head: id.head, contract: contractDigest(), image: o.image, imageId, rows: [...ACCEPTANCE_ROWS],
      swarmTicket: o.swarmTicket, approval: o.approval.trim(), issuedAt: now.toISOString(), expiresAt: new Date(now.getTime() + ttl * 3_600_000).toISOString(),
      purpose: { kind: "frozen-deployment", generation: o.generation, repo, partner },
    };
    if (existsSync(receiptPath(dir))) throw new Error("当前已有未退役 receipt，须先 retire");
    mkdirSync(roundDirOf(dir, receipt.nonce), { recursive: true, mode: 0o700 });
    controlledDir(join(dir, "rounds"), { roots: o.roots });
    publishOnce(receiptPath(dir), receipt);
    return receipt;
  }, { generation: o.generation, control: controlLockPath(dir), controlMode: "ex", root: o.drainRoot });
}

export function parseControllerAdmission(text: string, head: string, nonce: string): void {
  const lines = text.split(/\r?\n/).filter((l) => l.trim() !== "");
  if (lines[0] !== "VERDICT: ADMIT") throw new Error("主控准入记录首个非空行必须逐字等于 `VERDICT: ADMIT`");
  const one = (name: string) => {
    const hits = lines.filter((l) => l.startsWith(`${name}:`));
    if (hits.length !== 1) throw new Error(`主控准入记录 ${name} 必须恰好出现一次`);
    return hits[0]!.slice(name.length + 1).trim();
  };
  if (one("HEAD") !== head) throw new Error("主控准入记录 HEAD 不是本轮候选");
  if (one("NONCE") !== nonce) throw new Error("主控准入记录 NONCE 不是本轮");
}

export interface BindOptions extends RoundContext { ticket: number; head: string; nonce: string; candidateReview: string; controllerAdmission: string }
export async function bindMerges(o: BindOptions): Promise<FrozenMergePlan> {
  const sel: DeploymentSelection = { ticket: o.ticket, head: o.head, nonce: o.nonce, row: "14" };
  const first = loadFrozenRound(sel, o);
  return withFileLock(first.control, () => {
    const round = loadFrozenRound(sel, o);
    const repo = round.purpose.repo;
    const candidate = loadCandidate(round);
    if (!candidate) throw new Error("本轮第 13 行尚无已发布候选（candidate_pending），不能封定归并");
    if (repoGit(repo, ["rev-parse", "--verify", "-q", `refs/heads/${candidate.branch}`]) !== candidate.head) throw new Error("候选分支已不在本轮候选 SHA");
    try { repoGit(repo, ["merge-base", "--is-ancestor", round.receipt.head, candidate.head]); } catch { throw new Error("候选不是从本轮部署 head 派生"); }
    const review = currentCandidateSource(round, { head: candidate.head, reviewFile: o.candidateReview, controllerAdmission: o.controllerAdmission });
    const partner = round.purpose.partner;
    verifyPartner(repo, partner, round.ticket);
    const verification = join(round.roundDir, ROUND_FILES.candidate);
    const inputs: MergeManifest["inputs"] = [
      { issue: round.ticket, branch: candidate.branch, sha: candidate.head, acceptance: { head: candidate.head, status: "accepted", reviewEvidence: review.file, verificationEvidence: `${verification}#head=${candidate.head}` } },
      { issue: partner.issue, branch: partner.branch, sha: partner.sha, acceptance: { head: partner.sha, status: "accepted", reviewEvidence: partner.review, verificationEvidence: partner.approval } },
    ];
    // git merge-tree --write-tree 有冲突时退出码为 1，只写对象库，不动 ref 与工作区。
    try { execFileSync("git", ["-C", repo, "merge-tree", "--write-tree", "--name-only", candidate.head, partner.sha], { stdio: "pipe" }); throw new Error("候选与固定伙伴不冲突，rows14/15 无法产生真实冲突"); }
    catch (e) { if ((e as { status?: number }).status !== 1) throw e; }
    const derive = (row: MergeRow, instance: string): BoundMergeRun => {
      const runId = mergeRunId(row, round.receipt.nonce);
      const manifest: MergeManifest = { runId, branch: `sandcastle/merge-${runId}`, base: round.receipt.head, inputs };
      validateMergeManifest(manifest);
      const evidenceDir = join(round.roundDir, ROUND_FILES.evidence(row));
      return { row, manifest, repo, cwd: mergeWorktree(repo, manifest.branch), evidenceDir, checkpoint: join(evidenceDir, "checkpoint.json"), instance, checkId: MERGE_CHECK_ID };
    };
    const existing = loadMergePlan(round);
    const candidateFact = { head: candidate.head, review, controllerAdmission: o.controllerAdmission };
    if (existing) {
      const same = isDeepStrictEqual(existing.candidate, candidateFact) && MERGE_ROWS.every((row) => isDeepStrictEqual(existing.runs[row], derive(row, existing.runs[row].instance)));
      if (!same) throw new Error("本轮归并计划已封定且与本次输入不同，拒绝改写");
      return existing;
    }
    const runs = { "14": derive("14", randomUUID()), "15": derive("15", randomUUID()) };
    for (const run of Object.values(runs)) if (branchExists(repo, run.manifest.branch) || existsSync(run.cwd)) throw new Error(`归并分支或工位 ${run.manifest.branch} 已存在，不能作为本轮新 run`);
    const plan: FrozenMergePlan = { schema: 1, round: round.identity, candidate: candidateFact, runs };
    publishOnce(join(round.roundDir, ROUND_FILES.plan), plan);
    return plan;
  });
}

// ── 主控 CLI（仅宿主主控使用；目录固定，--dir 只用于演练并同样受 controlledDir 校验）────────
async function main(argv: string[]) {
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
  } else if (cmd === "issue-frozen") {
    const image = opt("image"), approval = opt("approval"), partnerInput = opt("partner-input");
    if (!image || !approval || !partnerInput || !opt("swarm-ticket") || !opt("generation")) throw new Error("用法：issue-frozen --image <镜像> --swarm-ticket N --generation G --approval <用户批准引用> --partner-input <固定伙伴 JSON> [--ttl-hours 12]");
    const r = await issueFrozenReceipt({ ...ctx, image, approval, swarmTicket: Number(opt("swarm-ticket")), generation: Number(opt("generation")),
      partner: JSON.parse(readFileSync(partnerInput, "utf8")), ttlHours: opt("ttl-hours") ? Number(opt("ttl-hours")) : undefined });
    const p = r.purpose as FrozenPurpose;
    console.log(JSON.stringify({ issued: receiptPath(resolveDir(ctx)), nonce: r.nonce, head: r.head, imageId: r.imageId, swarmTicket: r.swarmTicket, generation: p.generation, repo: p.repo, partner: p.partner.sha, expiresAt: r.expiresAt }));
  } else if (cmd === "bind-merges") {
    const sel = parseSelection({ ticket: opt("ticket"), head: opt("head"), nonce: opt("nonce"), row: "14" });
    const candidateReview = opt("candidate-review"), controllerAdmission = opt("controller-admission");
    if (!candidateReview || !controllerAdmission) throw new Error("用法：bind-merges --ticket N --head <部署 SHA> --nonce <本轮 nonce> --candidate-review <精确候选异家族独审> --controller-admission <主控准入记录>");
    const plan = await bindMerges({ ...ctx, ticket: sel.ticket, head: sel.head, nonce: sel.nonce, candidateReview, controllerAdmission });
    console.log(JSON.stringify({ candidate: plan.candidate.head, runs: Object.fromEntries(MERGE_ROWS.map((r) => [r, { runId: plan.runs[r].manifest.runId, cwd: plan.runs[r].cwd, checkpoint: plan.runs[r].checkpoint, instance: plan.runs[r].instance }])) }));
  } else if (cmd === "retire") {
    console.log(JSON.stringify({ retired: await retireReceipt(ctx) }));
  } else if (cmd === "status") {
    const dir = resolveDir(ctx);
    let receipt: AcceptanceReceipt | null = null; let invalid: string | undefined;
    if (existsSync(receiptPath(dir))) { try { receipt = loadReceipt(dir); } catch (e) { invalid = String(e); } }
    const used = receipt ? ACCEPTANCE_ROWS.map((r) => { try { return [r, loadUsed(dir, receipt!, r)?.binding ?? null]; } catch (e) { return [r, `INVALID ${e}`]; } }) : [];
    console.log(JSON.stringify({ dir, receipt, invalid, used: Object.fromEntries(used), accepted: receipt ? existsSync(acceptedPath(dir)) : false, current: codeIdentity() }, null, 2));
  } else throw new Error("命令：issue | issue-frozen | bind-merges | accept | status | retire");
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try { await main(process.argv.slice(2)); }
  catch (error) { console.error(error instanceof Error ? error.message : String(error)); process.exitCode = error instanceof DrainFrozenError ? 75 : 1; }
}
