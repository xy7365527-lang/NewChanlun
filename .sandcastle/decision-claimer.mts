#!/usr/bin/env node
/**
 * decision-claimer.mts —— graph engineering 裁决面（#1547）。
 *
 * 设计正本：.sandcastle/docs/graph-engineering.md「裁决流程」。每张 grilling 票：
 *   分级（Fable 5.1）→ 起草（Fable 5.1）→ 独审（Opus 5.5，独立进程，只读草稿与源）
 *   → T 类 PASS：decision/<票号> 分支落正本 + resolution 评论；P 类：挂 needs-approval。
 *   其他 BLOCK：一次升档 Opus 5.5 带审查意见重做；仍 BLOCK：转 P 类。
 *
 *   只有 R1 引用类 BLOCK：一次只改引用的修订轮；总轮次上限 MAX_ROUNDS。起草后先查正本落点，越界退回重写一次。
 * 模式：默认 --dry-run，只写 /Volumes/AgentStorage/decisions/<票号>/，不写 GitHub。
 *       --live 才 claim（assign）、评论、打标签、推 decision 分支。
 * 调用：node --import tsx .sandcastle/decision-claimer.mts [--live] [--loop] [--ticket N ...]
 *
 * 模型通道：claude -p 经本机统一路由 127.0.0.1:8318；令牌运行时从 CLIProxyLocal config
 * 读取，只进子进程环境变量，不打印、不写日志、不落文件。
 */
import { execFileSync, spawn } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

// ───────────────────────── 常量 ─────────────────────────

export const MODEL_FABLE = "claude-fable-5-1";
export const MODEL_OPUS = "claude-opus-5-5";
export const PROXY_BASE_URL = "http://127.0.0.1:8318";
export const PROXY_CONFIG = join(homedir(), "Library/Application Support/CLIProxyLocal/config.yaml");
export const DEFAULT_OUT_ROOT = "/Volumes/AgentStorage/decisions";
export const DEFAULT_MAPS = [1323, 1340];
export const MAX_CONCURRENT = 3;
export const LOOP_INTERVAL_MS = 10 * 60 * 1000;
export const AUTH_LINE = "本条依 #1323 Notes N-1 预授权判定";
export const RULES = ["R1", "R2", "R3", "R4", "R5", "R6", "R7"] as const;
export type Rule = (typeof RULES)[number];
export const DRAFT_SECTIONS = [
  "问题",
  "权威依据",
  "候选",
  "裁定",
  "验收清单",
  "受影响票与 blocking 边变化",
  "授权引用",
] as const;
/** 草稿七段之后必须附的引用自证表段名。 */
export const CITATION_SECTION = "引用自证";
/** T 类落地只准改这些正本位置。 */
export const LANDING_ROOTS = [".chanlun/definitions/", "docs/adr/", "CONTEXT.md"];

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const GH_REPO = process.env.GH_REPO ?? "xy7365527-lang/NewChanlun";
/** 取远端 main 用 ssh 地址：本机 https 拉取会卡死。 */
export const FETCH_URL = process.env.DECISION_FETCH_URL ?? "git@github.com:xy7365527-lang/NewChanlun.git";
export const MAIN_REF = "refs/remotes/origin/main";
/** 草稿里 ADR 一律写占位编号，落地时才分配真实编号。 */
export const ADR_PLACEHOLDER = "NNNN";

// ───────────────────────── 参数 ─────────────────────────

export type Mode = "dry-run" | "live";
export interface Args {
  mode: Mode;
  loop: boolean;
  tickets: number[];
  maps: number[];
  outRoot: string;
  concurrency: number;
}

export function parseArgs(argv: string[]): Args {
  const a: Args = { mode: "dry-run", loop: false, tickets: [], maps: [...DEFAULT_MAPS], outRoot: DEFAULT_OUT_ROOT, concurrency: MAX_CONCURRENT };
  for (let i = 0; i < argv.length; i++) {
    const v = argv[i];
    const next = () => {
      const n = argv[++i];
      if (n === undefined) throw new Error(`${v} 缺参数`);
      return n;
    };
    if (v === "--live") a.mode = "live";
    else if (v === "--dry-run") a.mode = "dry-run";
    else if (v === "--loop") a.loop = true;
    else if (v === "--ticket") a.tickets.push(...next().split(",").map(toTicket));
    else if (v === "--maps") a.maps = next().split(",").map(toTicket);
    else if (v === "--out") a.outRoot = next();
    else if (v === "--concurrency") a.concurrency = Math.min(MAX_CONCURRENT, Math.max(1, Number(next())));
    else throw new Error(`未知参数 ${v}`);
  }
  return a;
}

function toTicket(s: string): number {
  const n = Number(s.replace(/^#/, ""));
  if (!Number.isInteger(n) || n <= 0) throw new Error(`票号非法：${s}`);
  return n;
}

// ───────────────────────── 令牌 ─────────────────────────

/** 从 CLIProxyLocal config.yaml 文本取 access.api-keys 首项（纯函数，不依赖 yaml 库）。 */
export function extractFirstApiKey(yamlText: string): string | null {
  const lines = yamlText.split(/\r?\n/);
  let inAccess = false;
  let keysIndent = -1;
  for (const raw of lines) {
    if (/^\s*(#.*)?$/.test(raw)) continue;
    const indent = raw.length - raw.trimStart().length;
    const line = raw.trim();
    if (indent === 0) {
      inAccess = /^access:\s*$/.test(line);
      keysIndent = -1;
      continue;
    }
    if (!inAccess) continue;
    if (keysIndent < 0) {
      const m = /^api-keys:\s*(.*)$/.exec(line);
      if (!m) continue;
      const inline = m[1].trim();
      if (inline.startsWith("[")) {
        const first = inline.replace(/^\[|\]$/g, "").split(",")[0]?.trim();
        return first ? unquote(first) : null;
      }
      keysIndent = indent;
      continue;
    }
    if (indent <= keysIndent && !line.startsWith("-")) return null;
    if (line.startsWith("-")) {
      const v = unquote(line.slice(1).trim());
      return v || null;
    }
  }
  return null;
}

function unquote(s: string): string {
  return s.replace(/\s+#.*$/, "").replace(/^["']|["']$/g, "");
}

function readProxyToken(): string {
  const tok = extractFirstApiKey(readFileSync(PROXY_CONFIG, "utf8"));
  if (!tok) throw new Error("CLIProxyLocal config 未找到 access.api-keys 首项");
  return tok;
}

// ───────────────────────── 模型输出解析 ─────────────────────────

/** 取文本里最后一个 ```json 块，否则取最后一个配平的 {...}。 */
export function extractJson(text: string): unknown {
  const fences = [...text.matchAll(/```(?:json)?\s*\n([\s\S]*?)```/g)];
  for (let i = fences.length - 1; i >= 0; i--) {
    try {
      return JSON.parse(fences[i][1]);
    } catch { /* 继续找 */ }
  }
  for (let end = text.lastIndexOf("}"); end >= 0; end = text.lastIndexOf("}", end - 1)) {
    let depth = 0;
    for (let s = end; s >= 0; s--) {
      if (text[s] === "}") depth++;
      else if (text[s] === "{" && --depth === 0) {
        try {
          return JSON.parse(text.slice(s, end + 1));
        } catch { break; }
      }
    }
  }
  return undefined;
}

export type Klass = "T" | "P";
export interface Classification { klass: Klass; reason: string; parsed: boolean }

/** 分级解析：拿不准一律判 P。 */
export function parseClassification(text: string): Classification {
  const j = extractJson(text) as { class?: unknown; klass?: unknown; reason?: unknown } | undefined;
  const raw = String(j?.class ?? j?.klass ?? "").trim().toUpperCase();
  const reason = typeof j?.reason === "string" ? j.reason.trim() : "";
  if ((raw === "T" || raw === "P") && reason) return { klass: raw, reason, parsed: true };
  return { klass: "P", reason: `分级输出无法解析，按「拿不准一律判 P」处理${reason ? `：${reason}` : ""}`, parsed: false };
}

export type Verdict = "PASS" | "BLOCK";
export interface ReviewItem { rule: Rule; verdict: Verdict; basis: string }
export interface Review { items: ReviewItem[]; verdict: Verdict; problems: string[]; parseError: boolean }

/** R1–R7 审查解析：缺项、非法值、无依据一律记 BLOCK。 */
export function parseReview(text: string, draftMissing: string[] = []): Review {
  const j = extractJson(text) as Record<string, unknown> | undefined;
  const src = (j && typeof j === "object" && "rules" in j ? (j as { rules: unknown }).rules : j) as Record<string, unknown> | unknown[] | undefined;
  const byRule = new Map<string, { verdict?: unknown; basis?: unknown }>();
  if (Array.isArray(src)) {
    for (const it of src as Array<Record<string, unknown>>) byRule.set(String(it.rule ?? it.id ?? ""), it);
  } else if (src && typeof src === "object") {
    for (const [k, v] of Object.entries(src)) byRule.set(k, (v ?? {}) as Record<string, unknown>);
  }
  const problems: string[] = [];
  // 输出里找不到任何 R1–R7 条目：这是解析失败，与审查者真给出 BLOCK 分开记。
  const parseError = !j || !RULES.some((r) => byRule.has(r));
  if (parseError) problems.push("审查输出无可解析的 R1–R7 JSON");
  const items = RULES.map((rule): ReviewItem => {
    const it = byRule.get(rule);
    const verdict = String(it?.verdict ?? "").toUpperCase();
    const basis = typeof it?.basis === "string" ? it.basis.trim() : "";
    if (verdict !== "PASS" && verdict !== "BLOCK") {
      problems.push(`${rule} 缺结论`);
      return { rule, verdict: "BLOCK", basis: basis || "审查输出缺此项" };
    }
    if (!basis) {
      problems.push(`${rule} 缺依据`);
      return { rule, verdict: "BLOCK", basis: "审查未给依据" };
    }
    return { rule, verdict, basis };
  });
  if (draftMissing.length) problems.push(`草稿缺段：${draftMissing.join("、")}`);
  const verdict: Verdict = problems.length === 0 && items.every((i) => i.verdict === "PASS") ? "PASS" : "BLOCK";
  return { items, verdict, problems, parseError };
}

/** 草稿七段校验：返回缺失的段名。 */
export function missingDraftSections(draft: string): string[] {
  const heads = new Set([...draft.matchAll(/^#{1,3}\s*(?:\d+[.、]\s*)?(.+?)\s*$/gm)].map((m) => m[1].trim()));
  return DRAFT_SECTIONS.filter((s) => !heads.has(s));
}

/** 引用自证表：须有「## 引用自证」段且段内至少一行表格数据（表头与分隔行之外）。 */
export function hasCitationTable(draft: string): boolean {
  const m = new RegExp(`^#{1,3}\\s*(?:\\d+[.、]\\s*)?${CITATION_SECTION}\\s*$`, "m").exec(draft);
  if (!m) return false;
  const rest = draft.slice(m.index + m[0].length).split(/^#{1,3}\s/m)[0];
  const rows = rest.split(/\r?\n/).filter((l) => /^\s*\|.*\|\s*$/.test(l) && !/^\s*\|[\s:|-]+\|\s*$/.test(l));
  return rows.length >= 2;
}

/** 清掉路径两侧的 Markdown 修饰（反引号、粗斜体、尖括号、句末标点）。 */
function cleanPath(raw: string): string {
  const tok = raw.trim().replace(/^[*_`<\s]+/, "").split(/[\s`*<>，。；;,（(]/)[0] ?? "";
  return tok.replace(/[*_`>.:：]+$/, "").replace(/^\.\//, "");
}

/**
 * 草稿里声明的正本落点，且必须在 LANDING_ROOTS 内。容许三种写法：
 *   正本落点：path      **正本落点**：`path`      ### 正本落点（下一非空行为路径）
 */
export function landingTarget(draft: string): string | null {
  const p = declaredLanding(draft);
  return p !== null && isLandingPath(p) ? p : null;
}

/** 草稿声明的正本落点（清掉修饰，不校验根）；没写落点返回 null。 */
export function declaredLanding(draft: string): string | null {
  const lines = draft.split(/\r?\n/);
  for (let i = 0; i < lines.length; i++) {
    const m = /^\s*(?:[-+>]\s*)?(#{1,6}\s*)?[*_]*正本落点[*_]*\s*(?:[:：]\s*(.*))?$/.exec(lines[i]);
    if (!m) continue;
    let raw = (m[2] ?? "").trim();
    if (!raw) {
      let k = i + 1;
      while (k < lines.length && !lines[k].trim()) k++;
      raw = lines[k]?.trim() ?? "";
      if (raw.startsWith("#")) return null;
    }
    return cleanPath(raw) || null;
  }
  return null;
}

/** 起草后的落点硬检查：合法返回 null，否则返回退回起草者的原因。 */
export function landingProblem(draft: string): string | null {
  const p = declaredLanding(draft);
  if (p === null) return "草稿未声明「正本落点：<路径>」";
  if (!isLandingPath(p)) return `正本落点 ${p} 越界：只能在 ${LANDING_ROOTS.join("、")} 内`;
  return null;
}

export interface ReclassSuggestion { to: Klass; reason: string }

/** 起草者的改判建议：草稿里单独一行 `改判建议：T|P — 理由`。起草者自己不得改判。 */
export function reclassSuggestion(draft: string): ReclassSuggestion | null {
  const m = /^\s*[*_]*改判建议[*_]*\s*[:：]\s*[*_`]*([TPtp])\b[*_`]*\s*(?:类)?\s*[—\-–:：]*\s*(.*)$/m.exec(draft);
  if (!m) return null;
  return { to: m[1].toUpperCase() as Klass, reason: m[2].trim() };
}

export interface ReclassDecision { klass: Klass; converted: boolean; note?: string }

/** 编排逻辑对改判建议的处置：建议转 P 即转 P；建议转 T 一律忽略并记录。 */
export function applyReclass(klass: Klass, sug: ReclassSuggestion | null): ReclassDecision {
  if (!sug || sug.to === klass) return { klass, converted: false };
  if (sug.to === "P") return { klass: "P", converted: true, note: `起草者建议转 P，已转：${sug.reason}` };
  return { klass, converted: false, note: `起草者建议转 T，已忽略（分级器判 P）：${sug.reason}` };
}

/** P 类草稿自写 T 类授权句 = 起草者越权改判。 */
export function draftClassProblems(draft: string, klass: Klass): string[] {
  return klass === "P" && draft.includes(AUTH_LINE) ? ["P 类草稿写入 T 类预授权句，起草者越权改判"] : [];
}

export function isLandingPath(p: string): boolean {
  if (p.includes("..")) return false;
  return LANDING_ROOTS.some((r) => (r.endsWith("/") ? p.startsWith(r) && p.length > r.length : p === r));
}

// ───────────────────────── ADR 编号 ─────────────────────────

const ADR_RE = /^docs\/adr\/(\d{4}|NNNN)-(.+\.md)$/;

/** 目录里现有 ADR 的最大编号。 */
export function maxAdrNumber(files: string[]): number {
  return files.reduce((mx, f) => {
    const m = /^(\d{4})-.+\.md$/.exec(f);
    return m ? Math.max(mx, Number(m[1])) : mx;
  }, 0);
}

export interface AdrAllocation { path: string; number: number | null; renumbered: boolean }

/**
 * 给落点分配 ADR 编号。非 docs/adr 落点原样返回；已存在于 main 的 ADR 视为修订原件，不改号；
 * 其余（占位 NNNN 或草稿自编的号）按 max(main, 已预留) + 1 顺序分配，加文件锁防并发撞号。
 * reserve=false 只预览（dry-run），不写预留表。
 */
export async function allocateAdr(
  target: string,
  ticket: number,
  o: { adrDir: string; stateDir: string; reserve: boolean; sleep?: (ms: number) => Promise<void> },
): Promise<AdrAllocation> {
  const m = ADR_RE.exec(target);
  if (!m) return { path: target, number: null, renumbered: false };
  const files = existsSync(o.adrDir) ? readdirSync(o.adrDir) : [];
  if (m[1] !== ADR_PLACEHOLDER && files.includes(`${m[1]}-${m[2]}`)) return { path: target, number: Number(m[1]), renumbered: false };
  const slug = m[2];
  const resFile = join(o.stateDir, "adr-reservations.json");
  const lock = join(o.stateDir, ".adr-alloc.lock");
  const sleep = o.sleep ?? ((ms) => new Promise((r) => setTimeout(r, ms)));
  mkdirSync(o.stateDir, { recursive: true });
  for (let i = 0; ; i++) {
    try { writeFileSync(lock, String(process.pid), { flag: "wx" }); break; } catch {
      if (i >= 600) throw new Error(`ADR 编号锁 ${lock} 被占用超时`);
      await sleep(100);
    }
  }
  try {
    const res: Record<string, number> = existsSync(resFile) ? JSON.parse(readFileSync(resFile, "utf8")) : {};
    const mainMax = maxAdrNumber(files);
    // 预留号大于 main 最大号才算仍有效；已被 main 追上的预留作废。
    const live = Object.entries(res).filter(([, v]) => v > mainMax);
    const mine = live.find(([k]) => k === String(ticket))?.[1];
    const n = mine ?? Math.max(mainMax, ...live.map(([, v]) => v)) + 1;
    if (o.reserve) writeJson(resFile, { ...res, [String(ticket)]: n });
    return { path: `docs/adr/${String(n).padStart(4, "0")}-${slug}`, number: n, renumbered: true };
  } finally {
    rmSync(lock, { force: true });
  }
}

/** 把草稿里的旧落点与「ADR NNNN」占位换成分配到的编号。 */
export function renumberDraft(draft: string, from: string, a: AdrAllocation): string {
  if (!a.renumbered || a.number === null) return draft;
  const num = String(a.number).padStart(4, "0");
  return draft.split(from).join(a.path).replace(/ADR[\s-]*NNNN/g, `ADR ${num}`);
}

// ───────────────────────── 状态转移 ─────────────────────────

export type Phase = "classify" | "draft" | "review" | "land_t" | "shadow_p" | "review_parse_error" | "done" | "error";
/** 起草/审查总轮次上限（含升档重做与只改引用的修订轮）。 */
export const MAX_ROUNDS = 3;
/** 起草方式：首稿、升档重做、只改引用的修订。 */
export type DraftMode = "initial" | "redo" | "revise";
export interface RunState {
  phase: Phase;
  klass: Klass;
  attempt: number;
  drafterModel: string;
  convertedToP: boolean;
  draftMode: DraftMode;
  upgraded: boolean;
  revised: boolean;
  reviewFeedback?: string;
}

export function initialState(klass: Klass): RunState {
  return { phase: "draft", klass, attempt: 1, drafterModel: MODEL_FABLE, convertedToP: false, draftMode: "initial", upgraded: false, revised: false };
}

/** BLOCK 依据指向具体引用的特征：行号、文件:行、逐字、转述、核实命令、评论 id、引文。 */
const CITATION_POINTER = /行号|[\w.\/`]:\d+|逐字|转述|核实命令|评论\s*id|引文/i;

/** 只有 R1 BLOCK、且每条依据都指向具体引用，没有其他问题。 */
export function isCitationOnlyBlock(r: Review): boolean {
  if (r.parseError || r.verdict !== "BLOCK" || r.problems.length > 0) return false;
  const blocked = r.items.filter((i) => i.verdict === "BLOCK");
  return blocked.length > 0 && blocked.every((i) => i.rule === "R1" && CITATION_POINTER.test(i.basis));
}

/**
 * 审查后的转移：PASS→落地；只有 R1 引用类 BLOCK→一次只改引用的修订轮（Opus）；
 * 其他 BLOCK→一次升档 Opus 重做；修订与升档都用过或总轮次到 MAX_ROUNDS 仍 BLOCK→转 P。
 */
export function nextAfterReview(s: RunState, r: Review): RunState {
  if (r.verdict === "PASS") return { ...s, phase: s.klass === "T" ? "land_t" : "shadow_p" };
  const toP: RunState = { ...s, phase: "shadow_p", convertedToP: s.convertedToP || s.klass === "T", klass: "P" };
  if (s.attempt >= MAX_ROUNDS) return toP;
  const next = { ...s, phase: "draft" as const, attempt: s.attempt + 1, drafterModel: MODEL_OPUS, reviewFeedback: formatFeedback(r) };
  if (isCitationOnlyBlock(r) && !s.revised) return { ...next, draftMode: "revise", revised: true };
  if (!s.upgraded) return { ...next, draftMode: "redo", upgraded: true };
  return toP;
}

export function formatFeedback(r: Review): string {
  const lines = r.items.filter((i) => i.verdict === "BLOCK").map((i) => `- ${i.rule} BLOCK：${i.basis}`);
  return [...lines, ...r.problems.map((p) => `- ${p}`)].join("\n");
}

// ───────────────────────── GitHub 守卫 ─────────────────────────

export type GhRunner = (args: string[], input?: string) => string;

/** 判断一条 gh 命令是否写 GitHub。 */
export function isGhWrite(args: string[]): boolean {
  const [a, b] = args;
  if (a === "api") {
    const i = args.findIndex((x) => x === "-X" || x === "--method");
    const method = i >= 0 ? args[i + 1]?.toUpperCase() : undefined;
    if (method && method !== "GET") return true;
    if (args.some((x) => x === "-f" || x === "-F" || x === "--field" || x === "--raw-field" || x === "--input")) return method !== "GET";
    return false;
  }
  const readOnly: Record<string, string[]> = {
    issue: ["view", "list", "status"],
    pr: ["view", "list", "status", "diff", "checks"],
    repo: ["view"],
    auth: ["status"],
  };
  if (a in readOnly) return !readOnly[a].includes(b);
  return true;
}

export class DryRunWriteError extends Error {}

export function guardGh(mode: Mode, raw: GhRunner): GhRunner {
  return (args, input) => {
    if (mode === "dry-run" && isGhWrite(args)) throw new DryRunWriteError(`dry-run 拒绝 GitHub 写命令：gh ${args.slice(0, 2).join(" ")}`);
    return raw(args, input);
  };
}

function realGh(args: string[], input?: string): string {
  return execFileSync("gh", args, {
    encoding: "utf8",
    input,
    env: { ...process.env, GH_REPO, NO_COLOR: "1" },
    stdio: ["pipe", "pipe", "pipe"],
    maxBuffer: 64 * 1024 * 1024,
  });
}

// ───────────────────────── 模型调用 ─────────────────────────

export interface ModelCall { model: string; prompt: string; cwd: string; role: "classify" | "draft" | "review" | "land"; allowWrite?: boolean }
export interface ModelResult { text: string; durationMs: number; attempts: number }
export type ModelRunner = (c: ModelCall) => Promise<ModelResult>;

export function isRetryable(output: string): boolean {
  return /\b(429|529)\b|rate[ _-]?limit|overloaded|too many requests/i.test(output);
}

export function backoffMs(attempt: number, baseMs = 20_000, capMs = 300_000): number {
  return Math.min(capMs, baseMs * 2 ** (attempt - 1));
}

/** 只读取证：评论 id 只能用这两条 gh api 查询回查。 */
export const GH_API_READ_TOOLS = [
  `Bash(gh api repos/${GH_REPO}/issues/comments/*)`,
  `Bash(gh api repos/${GH_REPO}/issues/*/comments*)`,
];
export const READ_TOOLS = ["Read", "Grep", "Glob", "Bash(gh issue view:*)", ...GH_API_READ_TOOLS, "Bash(git log:*)", "Bash(git show:*)", "Bash(rg:*)"];
const WRITE_TOOLS = ["Edit", "Write", "MultiEdit", "NotebookEdit"];
/** gh api 一切带方法或请求体的形态都拒，保证上面两条只能 GET。 */
export const GH_API_WRITE_DENY = [
  "Bash(gh api *-X*)", "Bash(gh api *--method*)", "Bash(gh api *-f *)", "Bash(gh api *-F *)",
  "Bash(gh api *--field*)", "Bash(gh api *--raw-field*)", "Bash(gh api *--input*)",
  "Bash(gh api *-f=*)", "Bash(gh api *-F=*)",
];
export function toolArgs(allowWrite: boolean): string[] {
  return [
    "--allowedTools", ...(allowWrite ? [...READ_TOOLS, ...WRITE_TOOLS] : READ_TOOLS),
    "--disallowedTools", ...(allowWrite ? GH_API_WRITE_DENY : [...WRITE_TOOLS, ...GH_API_WRITE_DENY]),
  ];
}

function claudeOnce(c: ModelCall, token: string, timeoutMs: number): Promise<{ code: number; stdout: string; stderr: string }> {
  const env: NodeJS.ProcessEnv = { ...process.env, ANTHROPIC_BASE_URL: PROXY_BASE_URL, ANTHROPIC_AUTH_TOKEN: token };
  delete env.ANTHROPIC_API_KEY;
  const args = [
    "-p", "--bare", "--model", c.model, "--output-format", "json", "--no-session-persistence",
    ...toolArgs(!!c.allowWrite),
  ];
  return new Promise((res) => {
    const child = spawn("claude", args, { cwd: c.cwd, env, stdio: ["pipe", "pipe", "pipe"] });
    let stdout = "";
    let stderr = "";
    const timer = setTimeout(() => child.kill("SIGTERM"), timeoutMs);
    child.stdout.on("data", (d) => (stdout += d));
    child.stderr.on("data", (d) => (stderr += d));
    child.on("close", (code) => {
      clearTimeout(timer);
      res({ code: code ?? 1, stdout, stderr });
    });
    child.stdin.end(c.prompt);
  });
}

export function makeClaudeRunner(opts: { maxAttempts?: number; timeoutMs?: number; sleep?: (ms: number) => Promise<void> } = {}): ModelRunner {
  const maxAttempts = opts.maxAttempts ?? 6;
  const timeoutMs = opts.timeoutMs ?? 30 * 60 * 1000;
  const sleep = opts.sleep ?? ((ms) => new Promise((r) => setTimeout(r, ms)));
  return async (c) => {
    const token = readProxyToken();
    const t0 = Date.now();
    for (let attempt = 1; ; attempt++) {
      const r = await claudeOnce(c, token, timeoutMs);
      let parsed: { result?: string; is_error?: boolean } | undefined;
      try { parsed = JSON.parse(r.stdout); } catch { /* 非 JSON */ }
      const ok = r.code === 0 && parsed && !parsed.is_error && typeof parsed.result === "string";
      if (ok) return { text: parsed!.result!, durationMs: Date.now() - t0, attempts: attempt };
      const blob = `${parsed?.result ?? r.stdout}\n${r.stderr}`;
      if (attempt < maxAttempts && isRetryable(blob)) {
        const wait = backoffMs(attempt);
        log(`${c.role} ${c.model} 第 ${attempt} 次 429/限流，${Math.round(wait / 1000)}s 后重试`);
        await sleep(wait);
        continue;
      }
      throw new Error(`${c.role} ${c.model} 调用失败（exit ${r.code}）：${redact(blob).slice(0, 600)}`);
    }
  };
}

function redact(s: string): string {
  return s.replace(/(token|key|authorization)["':=\s]+[A-Za-z0-9_\-.]{16,}/gi, "$1=<redacted>");
}

// ───────────────────────── 票面上下文 ─────────────────────────

export interface TicketContext {
  number: number;
  title: string;
  body: string;
  labels: string[];
  assignees: string[];
  comments: Array<{ author: string; body: string }>;
  blockers: Array<{ number: number; state: string; title: string }>;
}

export function loadTicket(gh: GhRunner, n: number): TicketContext {
  const v = JSON.parse(gh(["issue", "view", String(n), "--json", "number,title,body,labels,assignees,comments"])) as {
    number: number; title: string; body: string;
    labels: Array<{ name: string }>; assignees: Array<{ login: string }>;
    comments: Array<{ author?: { login?: string }; body: string }>;
  };
  let blockers: TicketContext["blockers"] = [];
  try {
    blockers = (JSON.parse(gh(["api", `repos/${GH_REPO}/issues/${n}/dependencies/blocked_by`])) as Array<{ number: number; state: string; title: string }>)
      .map((b) => ({ number: b.number, state: b.state, title: b.title }));
  } catch { /* 依赖接口不可用时留空 */ }
  return {
    number: v.number, title: v.title, body: v.body ?? "",
    labels: v.labels.map((l) => l.name), assignees: v.assignees.map((a) => a.login),
    comments: v.comments.map((c) => ({ author: c.author?.login ?? "?", body: c.body })),
    blockers,
  };
}

function renderTicket(t: TicketContext): string {
  const comments = t.comments.slice(-12).map((c) => `--- 评论 @${c.author} ---\n${c.body.slice(0, 4000)}`).join("\n\n");
  const blockers = t.blockers.map((b) => `- #${b.number} [${b.state}] ${b.title}`).join("\n") || "（无）";
  return `# #${t.number} ${t.title}\n标签：${t.labels.join(", ")}\n\n## 票面\n${t.body}\n\n## 阻塞本票的票\n${blockers}\n\n## 最近评论\n${comments || "（无）"}`;
}

// ───────────────────────── 提示词 ─────────────────────────

export function classifyPrompt(t: TicketContext): string {
  return `你是 NewChanlun 仓 #1323 图的裁决面分级者。只读工作，禁止修改任何文件。
按 .sandcastle/docs/graph-engineering.md「1. 分级」判本票属于哪一类：
- T 类（技术取舍）：数据结构、接口与 seam、owner 落点、状态机与事件 schema、测试锁、迁移步骤、恢复语义；有权威源或本仓判例可援引。
- P 类（政策）：资金与费用参数（本金、量档、cap、分配）、风险偏好、真实场所/账户/权限、关图与 #1279 去向、Destination 改写、产品 taste、本仓无先例的新取舍。
拿不准一律判 P。可用 Read/Grep/gh issue view 查关联票与仓内正本。

${renderTicket(t)}

最后只输出一个 JSON 代码块：{"class":"T"或"P","reason":"一句中文理由，点名依据"}`;
}

export function draftPrompt(t: TicketContext, c: Classification, feedback?: string): string {
  return `你是 NewChanlun 仓 #1323 图的裁决起草者。当前目录是 origin/main 的只读检出；禁止修改任何文件，只读取证。
先读 AGENTS.md（教义正本、#799 收敛通则、#804 总缝规则、引原文四步核验）、CONTEXT.md、.sandcastle/docs/graph-engineering.md，以及票面点名的票（gh issue view <号> --comments）。评论 id 用 gh api repos/${GH_REPO}/issues/comments/<id> 或 gh api repos/${GH_REPO}/issues/<号>/comments 取，只读。
本票分级：${c.klass} 类 —— ${c.reason}
分级由分级器决定，你无权改判：按 ${c.klass} 类起草。若你认为分级有误，在「授权引用」段末另起一行写 \`改判建议：T 或 P — 理由\`，由编排逻辑处置；其余部分仍按 ${c.klass} 类写。

${renderTicket(t)}
${feedback ? `\n## 上一稿被独立审查 BLOCK，必须逐条处理：\n${feedback}\n` : ""}
输出一份 resolution 草稿（Markdown），先是以下七个二级标题，顺序一致、标题逐字相同；七段之后最后再附一段 \`## ${CITATION_SECTION}\`：
${DRAFT_SECTIONS.map((s) => `## ${s}`).join("\n")}
## ${CITATION_SECTION}

硬要求：
1. 权威依据逐条带可回查位置：docs/chanlun/text/blog/<课>.md:<行>、ADR 路径、.chanlun/definitions/ 路径或票评论链接。引缠师原文须核行号、逐字、作者归属（界前界后＋行首注）、行内注四步；核不过就不引。
2. 每条引用写进草稿前必须先用工具核实：文件引用用 Read 或 rg 看到该路径该行的原文；ADR 与定义文件引用先确认文件存在、所引条款在文件里逐字可见；票评论引用先用 gh api repos/${GH_REPO}/issues/comments/<id> 取到该评论并核对作者与内容。核实不到的路径、行号、条款、评论 id 一律不引，也不得凭记忆补写。
3. 「裁定」段必须单独一行写纯文本 \`正本落点：<路径>\`（行首就是「正本落点：」，路径不加反引号、粗体或其他修饰，行内不写别的）。路径只能在 .chanlun/definitions/ 下、docs/adr/ 下，或为 CONTEXT.md。${ADR_FORMAT_LINE}
4. 验收清单每项必须是以下之一：可执行命令、测试名、文件存在、票状态；判据二值。
5. 不越出 #1323 Destination；T 类草稿不得夹带 P 类取舍；不得新增同一判断的宽严两档或失败兜底。
6. 「受影响票与 blocking 边变化」列出全部受影响票号和每条要增删的 blocking 边。
7. 「授权引用」段：${c.klass === "T" ? `写「${AUTH_LINE}」并引用户 2026-10-18 原话` : "写「待用户批复（影子模式）」，不得写 T 类预授权句"}。
8. 「${CITATION_SECTION}」段是一张 Markdown 表，表头为 \`| 引用 | 核实命令 | 结果摘要 |\`，草稿里每一条引用（文件:行、ADR、定义文件条款、票号、评论 id）各占一行：写出你实际执行的核实命令与看到的结果摘要。表里没有的引用视为未核实。自证表里写不出亲眼看到的原文就整条删掉（连同正文里那条引用），不许写「未直接看到但结构确认」之类。
只输出草稿正文，不要前言或结语。`;
}

/** 修订轮：只改被审查点名的引用与自证表，裁定内容不动。 */
export function revisePrompt(t: TicketContext, c: Classification, prevDraft: string, feedback: string): string {
  return `${draftPrompt(t, c)}

## 本轮是「只改引用」的修订轮
上一稿的裁定内容已过审，独立审查只对以下引用 BLOCK：
${feedback}

逐条处理：重新用工具核实被点名的引用，改正行号、把转述改成与原文一致、让核实命令真正覆盖所引的行；核实不到的引用连同自证表里那一行一起删掉。
只准改这些引用和「${CITATION_SECTION}」表；「裁定」「候选」「验收清单」「受影响票与 blocking 边变化」「授权引用」的取舍与内容保持原样，正本落点保持原样。输出修订后的完整草稿。

===== 上一稿开始 =====
${prevDraft}
===== 上一稿结束 =====`;
}

/** 落点格式说明里关于 ADR 编号的那一句，单测断言提示含此句。 */
export const ADR_FORMAT_LINE = `新建 ADR 时落点一律写 \`正本落点：docs/adr/${ADR_PLACEHOLDER}-<英文短名>.md\`，正文提到它时写「ADR ${ADR_PLACEHOLDER}」，编号永远写 ${ADR_PLACEHOLDER}，落地时按 main 顺序分配；反例（禁止）：\`正本落点：docs/adr/0041-dual-run.md\`、「ADR 0041」、引用一个仓内并不存在的 ADR 编号。`;

export function reviewPrompt(t: TicketContext, c: Classification, draft: string): string {
  return `你是 NewChanlun 仓 #1323 图裁决面的独立审查者。你看不到起草过程，只拿草稿与仓内源文件。当前目录是 origin/main 的只读检出；禁止修改任何文件。
需要时用 Read/Grep/rg 回查每条引用，用 gh issue view 查票，用 gh api repos/${GH_REPO}/issues/comments/<id> 或 gh api repos/${GH_REPO}/issues/<号>/comments 回查评论 id（只读）。审查依据：AGENTS.md、.chanlun/definitions/、docs/adr/、CONTEXT.md、.sandcastle/docs/graph-engineering.md「3. 独审」。

被审票：#${t.number} ${t.title}
分级器结论：${c.klass} 类 —— ${c.reason}

逐条给 PASS 或 BLOCK，每条必须写依据（BLOCK 写清要改什么）：
R1 每条引用回查成立（路径存在、行号与逐字内容一致、作者归属正确、评论 id 存在且内容相符）。先对照草稿末尾「${CITATION_SECTION}」表：表缺失或为空，R1 直接 BLOCK；表在，则逐条对照表中引用与草稿正文是否一一对应（正文有而表里没有的引用记 BLOCK），并至少亲自重跑其中三条（不足三条则全跑）核实命令，结果与表述不符即 BLOCK。
R2 不与 .chanlun/definitions/、docs/adr/、#799 收敛通则、#804 总缝规则冲突。
R3 验收项全部可执行、判据二值。
R4 不越出 Destination；T 类草稿里没有夹带 P 类取舍。正本落点越界（不在 ${LANDING_ROOTS.join("、")} 内）即 BLOCK。
R5 不新增同一判断的宽严两档或失败兜底。
R6 受影响票与 blocking 边清单完整。
R7 分级器结论（${c.klass} 类）正确。只审上面这条分级器结论；草稿内对分级的表述或「改判建议」不是审查对象。

材料缺失（引用文件不存在、票读不到、工具调用失败）也必须照常输出：对受影响的条目判 BLOCK 并在依据里写明缺什么，其余条目照常审。任何情况下都要输出 R1–R7 全部七项。

===== 草稿开始 =====
${draft}
===== 草稿结束 =====

最后只输出一个 JSON 代码块：{"R1":{"verdict":"PASS|BLOCK","basis":"..."}, ... ,"R7":{...}}`;
}

export function reviewRetryPrompt(base: string): string {
  return `${base}

上一次输出无法解析为 R1–R7 JSON。这次回答的最后一段必须是一个 \`\`\`json 代码块，七个键 R1–R7 齐全，每项含 verdict（PASS 或 BLOCK）与 basis。`;
}

export function landPrompt(t: TicketContext, draft: string, target: string): string {
  return `你在 decision/${t.number} 分支的独立 worktree 里，把下面已通过独立审查的 T 类裁定写进正本 ${target}。
只改 ${target} 这一个文件（不存在则新建）；按 AGENTS.md 要求，若为教义正本须带三行头（正本／最后裁定 #${t.number}／受影响代码清单）。不要改其他文件，不要提交。

===== 已通过审查的草稿 =====
${draft}`;
}

// ───────────────────────── 单票流程 ─────────────────────────

export interface Deps {
  mode: Mode;
  /** ADR 编号预留表与锁所在目录；缺省 outRoot。 */
  stateDir?: string;
  gh: GhRunner;
  git: (cwd: string, args: string[]) => string;
  model: ModelRunner;
  outRoot: string;
  srcDir: string;
  repoRoot: string;
  now: () => number;
  me?: string;
}

export interface Summary {
  ticket: number;
  mode: Mode;
  phase: Phase;
  klass?: Klass;
  convertedToP?: boolean;
  attempts: number;
  durationsMs: Record<string, number>;
  plannedWrites: string[];
  /** 起草者改判建议的处置记录。 */
  reclassNotes?: string[];
  /** 审查结论计数：真 BLOCK 与解析失败分开。 */
  reviewCounts?: { pass: number; block: number; parseError: number };
  /** 每轮起草方式与审查结论。 */
  rounds?: Array<{ attempt: number; mode: DraftMode; drafter: string; verdict: string; blocked: Rule[]; problems: string[] }>;
  /** 落点硬检查退回记录。 */
  landingRedrafts?: string[];
  landing?: string;
  error?: string;
}

function writeJson(path: string, v: unknown) {
  writeFileSync(path, JSON.stringify(v, null, 2) + "\n");
}

function isGitWrite(args: string[]): boolean {
  return args[0] === "push";
}

export async function processTicket(n: number, d: Deps): Promise<Summary> {
  const dir = join(d.outRoot, String(n));
  mkdirSync(dir, { recursive: true });
  const lock = join(dir, ".lock");
  const summary: Summary = { ticket: n, mode: d.mode, phase: "classify", attempts: 0, durationsMs: {}, plannedWrites: [] };
  try {
    writeFileSync(lock, String(process.pid), { flag: "wx" });
  } catch {
    return { ...summary, phase: "error", error: "本地锁已存在，另一进程正在处理" };
  }
  const t0 = d.now();
  let claimed = false;
  // live 写 GitHub；dry-run 只登记意图，不触 gh 写命令。
  const write = (desc: string, args: string[], input?: string) => {
    if (d.mode === "dry-run") summary.plannedWrites.push(desc);
    else d.gh(args, input);
  };
  try {
    const t = loadTicket(d.gh, n);

    // claim 先行防重复
    if (d.mode === "live") {
      if (t.assignees.length > 0) throw new Error(`票已被 ${t.assignees.join(",")} 认领`);
      d.gh(["issue", "edit", String(n), "--add-assignee", "@me"]);
      claimed = true;
      const after = loadTicket(d.gh, n).assignees;
      if (after.length !== 1 || (d.me && after[0] !== d.me)) throw new Error(`claim 竞争：assignees=${after.join(",")}`);
    } else summary.plannedWrites.push(`claim：assign #${n}`);

    // 1. 分级
    const cr = await d.model({ model: MODEL_FABLE, prompt: classifyPrompt(t), cwd: d.srcDir, role: "classify" });
    const cls = parseClassification(cr.text);
    summary.durationsMs.classify = cr.durationMs;
    summary.klass = cls.klass;
    writeJson(join(dir, "classify.json"), { ticket: n, model: MODEL_FABLE, ...cls, durationMs: cr.durationMs, raw: cr.text });
    write(`评论 #${n}：分级：${cls.klass} — ${cls.reason}`, ["issue", "comment", String(n), "--body-file", "-"], `分级：${cls.klass} — ${cls.reason}`);

    // 2–3. 起草 / 独审，至多两轮。分级由分级器定，起草者只能提改判建议。
    let s = initialState(cls.klass);
    let effCls: Classification = cls;
    let draft = "";
    let review: Review | undefined;
    const counts = { pass: 0, block: 0, parseError: 0 };
    summary.reviewCounts = counts;
    summary.reclassNotes = [];
    summary.rounds = [];
    summary.landingRedrafts = [];
    while (s.phase === "draft") {
      summary.attempts = s.attempt;
      const prompt = (fb?: string) =>
        s.draftMode === "revise" ? revisePrompt(t, effCls, draft, fb ?? s.reviewFeedback ?? "") : draftPrompt(t, effCls, fb ?? s.reviewFeedback);
      let dr = await d.model({ model: s.drafterModel, prompt: prompt(), cwd: d.srcDir, role: "draft" });
      let next = dr.text.trim() + "\n";
      summary.durationsMs[`draft${s.attempt}`] = dr.durationMs;
      // 落点硬检查：越界或缺失立即退回起草者重写一次。
      const lp = landingProblem(next);
      if (lp) {
        writeFileSync(join(dir, `draft.attempt${s.attempt}.landing-rejected.md`), next);
        summary.landingRedrafts.push(`第 ${s.attempt} 轮：${lp}`);
        const fb = [s.reviewFeedback, `- 正本落点退回：${lp}。「裁定」段单独一行写纯文本 \`正本落点：<路径>\`，路径只能在 ${LANDING_ROOTS.join("、")} 内。`].filter(Boolean).join("\n");
        dr = await d.model({ model: s.drafterModel, prompt: prompt(fb), cwd: d.srcDir, role: "draft" });
        next = dr.text.trim() + "\n";
        summary.durationsMs[`draft${s.attempt}_landing`] = dr.durationMs;
      }
      draft = next;
      if (s.attempt > 1 && existsSync(join(dir, "draft.md"))) renameSync(join(dir, "draft.md"), join(dir, `draft.attempt${s.attempt - 1}.md`));
      writeFileSync(join(dir, "draft.md"), draft);

      const rc = applyReclass(s.klass, reclassSuggestion(draft));
      if (rc.note) summary.reclassNotes.push(`第 ${s.attempt} 稿：${rc.note}`);
      if (rc.converted) {
        s = { ...s, klass: "P", convertedToP: true };
        effCls = { ...cls, klass: "P", reason: `${cls.reason}（${rc.note}）` };
      }

      const missing = [...missingDraftSections(draft), ...(hasCitationTable(draft) ? [] : [CITATION_SECTION])];
      const extra = draftClassProblems(draft, s.klass);
      const basePrompt = reviewPrompt(t, effCls, draft);
      let rr = await d.model({ model: MODEL_OPUS, prompt: basePrompt, cwd: d.srcDir, role: "review" });
      review = parseReview(rr.text, missing);
      let retried = false;
      if (review.parseError) {
        retried = true;
        rr = await d.model({ model: MODEL_OPUS, prompt: reviewRetryPrompt(basePrompt), cwd: d.srcDir, role: "review" });
        review = parseReview(rr.text, missing);
      }
      if (!review.parseError && extra.length) review = { ...review, verdict: "BLOCK", problems: [...review.problems, ...extra] };
      if (review.parseError) counts.parseError++;
      else if (review.verdict === "PASS") counts.pass++;
      else counts.block++;
      summary.durationsMs[`review${s.attempt}`] = rr.durationMs;
      if (s.attempt > 1 && existsSync(join(dir, "review.json"))) renameSync(join(dir, "review.json"), join(dir, `review.attempt${s.attempt - 1}.json`));
      summary.rounds.push({
        attempt: s.attempt, mode: s.draftMode, drafter: s.drafterModel,
        verdict: review.parseError ? "review_parse_error" : review.verdict,
        blocked: review.parseError ? [] : review.items.filter((i) => i.verdict === "BLOCK").map((i) => i.rule),
        problems: review.problems,
      });
      writeJson(join(dir, "review.json"), {
        ticket: n, attempt: s.attempt, mode: s.draftMode, drafter: s.drafterModel, reviewer: MODEL_OPUS, klass: s.klass,
        verdict: review.parseError ? "review_parse_error" : review.verdict, parseError: review.parseError, retried,
        items: review.parseError ? [] : review.items, problems: review.problems, draftMissingSections: missing,
        durationMs: rr.durationMs, raw: rr.text,
      });
      if (review.parseError) {
        s = { ...s, phase: "review_parse_error" };
        summary.error = "审查输出两次均无法解析为 R1–R7 JSON（review_parse_error），未计入 BLOCK";
        break;
      }
      s = nextAfterReview(s, review);
    }
    summary.klass = s.klass;
    summary.convertedToP = s.convertedToP;

    // 4. 落地
    if (s.phase === "land_t") {
      const declared = landingTarget(draft);
      if (!declared) {
        s = { ...s, phase: "shadow_p", klass: "P", convertedToP: true };
        summary.klass = "P";
        summary.convertedToP = true;
        summary.error = "草稿未声明合法正本落点，转 P";
      } else {
        const alloc = await allocateAdr(declared, n, { adrDir: join(d.srcDir, "docs/adr"), stateDir: d.stateDir ?? d.outRoot, reserve: d.mode === "live" });
        const target = alloc.path;
        summary.landing = alloc.renumbered ? `${target}（草稿写 ${declared}，按 main 顺序分配）` : target;
        if (alloc.renumbered) {
          draft = renumberDraft(draft, declared, alloc);
          writeFileSync(join(dir, "draft.md"), draft);
        }
        if (d.mode === "dry-run") {
          summary.plannedWrites.push(`分支 decision/${n}：写 ${target} 并推送`, `评论 #${n}：resolution（首行「${AUTH_LINE}」）`);
        } else {
          await landT(t, draft, target, dir, d, summary);
        }
      }
    }
    if (s.phase === "shadow_p") {
      const body = shadowComment(n, draft, review!, dir, summary);
      write(`评论 #${n}：P 类草稿与审查结论`, ["issue", "comment", String(n), "--body-file", "-"], body);
      write(`标签 #${n}：+needs-approval`, ["issue", "edit", String(n), "--add-label", "needs-approval"]);
      write(`取消认领 #${n}`, ["issue", "edit", String(n), "--remove-assignee", "@me"]);
    }
    if (s.phase === "review_parse_error") write(`取消认领 #${n}（审查解析失败，待重跑）`, ["issue", "edit", String(n), "--remove-assignee", "@me"]);
    summary.phase = s.phase === "land_t" || s.phase === "shadow_p" || s.phase === "review_parse_error" ? s.phase : "done";
  } catch (e) {
    summary.phase = "error";
    summary.error = e instanceof Error ? e.message : String(e);
    if (claimed) {
      try { d.gh(["issue", "edit", String(n), "--remove-assignee", "@me"]); } catch { /* 尽力回收 */ }
    }
  } finally {
    summary.durationsMs.total = d.now() - t0;
    writeJson(join(dir, "summary.json"), summary);
    rmSync(lock, { force: true });
  }
  return summary;
}

function shadowComment(n: number, draft: string, r: Review, dir: string, s: Summary): string {
  const table = r.items.map((i) => `| ${i.rule} | ${i.verdict} | ${i.basis.replace(/\|/g, "\\|").replace(/\n/g, " ")} |`).join("\n");
  return `**P 类影子模式，待用户批复**${s.convertedToP ? `（原判 T，${s.attempts} 轮审查仍 BLOCK 后转 P）` : ""}\n\n` +
    `独立审查（${MODEL_OPUS}）：**${r.verdict}**\n\n| 条 | 结论 | 依据 |\n|---|---|---|\n${table}\n\n` +
    `草稿与审查原件：\`${dir}/\`\n\n<details><summary>草稿全文</summary>\n\n${draft}\n</details>`;
}

async function landT(t: TicketContext, draft: string, target: string, dir: string, d: Deps, s: Summary) {
  const n = t.number;
  const branch = `decision/${n}`;
  const wt = join(dir, "land-wt");
  fetchMain(d.repoRoot, d.git);
  if (existsSync(wt)) d.git(d.repoRoot, ["worktree", "remove", "--force", wt]);
  d.git(d.repoRoot, ["worktree", "add", "-B", branch, wt, MAIN_REF]);
  const lr = await d.model({ model: MODEL_OPUS, prompt: landPrompt(t, draft, target), cwd: wt, role: "land", allowWrite: true });
  s.durationsMs.land = lr.durationMs;
  const changed = d.git(wt, ["status", "--porcelain"]).split("\n").filter(Boolean).map((l) => l.slice(3));
  if (changed.length === 0) throw new Error("落地工蜂未改动正本");
  const stray = changed.filter((p) => p !== target);
  if (stray.length) throw new Error(`落地越界改动：${stray.join(", ")}`);
  d.git(wt, ["add", "--", target]);
  d.git(wt, ["commit", "-m", `decision(#${n}): ${t.title}\n\n${AUTH_LINE}；独审 ${MODEL_OPUS} R1–R7 PASS。`]);
  d.git(wt, ["push", FETCH_URL, `HEAD:refs/heads/${branch}`]);
  const sha = d.git(wt, ["rev-parse", "HEAD"]).trim();
  const body = `${AUTH_LINE}\n\n**resolution**：T 类，独立审查 R1–R7 全 PASS。正本 \`${target}\` 已写入分支 \`${branch}\`（${sha.slice(0, 10)}），待按日批量 PR 合入 main 后关票。\n\n` +
    `草稿：\`${dir}/draft.md\`；审查：\`${dir}/review.json\`\n\n<details><summary>草稿全文</summary>\n\n${draft}\n</details>`;
  d.gh(["issue", "comment", String(n), "--body-file", "-"], body);
}

// ───────────────────────── 选票与并发 ─────────────────────────

interface RawChild {
  number: number; state: string;
  labels?: Array<{ name: string }>; assignees?: Array<{ login: string }>;
  issue_dependencies_summary?: { blocked_by?: number };
}

/** frontier 上的 grilling 票：open、未认领、无 open blocker、未挂 needs-approval。 */
export function selectCandidates(children: RawChild[], skip: Set<number>): number[] {
  const seen = new Set<number>();
  return children
    .filter((c) => c.state.toLowerCase() === "open")
    .filter((c) => (c.labels ?? []).some((l) => l.name === "wayfinder:grilling"))
    .filter((c) => !(c.labels ?? []).some((l) => l.name === "needs-approval"))
    .filter((c) => (c.assignees ?? []).length === 0)
    .filter((c) => (c.issue_dependencies_summary?.blocked_by ?? 0) === 0)
    .map((c) => c.number)
    .filter((n) => !skip.has(n) && !seen.has(n) && seen.add(n));
}

function finishedTickets(outRoot: string, children: RawChild[]): Set<number> {
  const done = new Set<number>();
  for (const c of children) {
    const p = join(outRoot, String(c.number), "summary.json");
    if (!existsSync(p)) continue;
    try {
      const s = JSON.parse(readFileSync(p, "utf8")) as Summary;
      if (s.phase !== "error" && s.phase !== "review_parse_error") done.add(c.number);
    } catch { /* 坏文件视为未完成 */ }
  }
  return done;
}

export async function runPool<T, R>(items: T[], limit: number, fn: (x: T) => Promise<R>): Promise<R[]> {
  const out: R[] = new Array(items.length);
  let next = 0;
  const worker = async () => {
    while (next < items.length) {
      const i = next++;
      out[i] = await fn(items[i]);
    }
  };
  await Promise.all(Array.from({ length: Math.min(limit, items.length) }, worker));
  return out;
}

/** 用 ssh 地址把远端 main 取到 refs/remotes/origin/main，返回取到的 SHA。 */
export function fetchMain(gitDir: string, git: Deps["git"]): string {
  git(gitDir, ["fetch", FETCH_URL, `+main:${MAIN_REF}`]);
  return git(gitDir, ["rev-parse", MAIN_REF]).trim();
}

/** 取证检出对齐最新远端 main；fetch 在 srcDir 所属的库里做，checkout 后 HEAD 必须等于刚取到的 SHA。 */
export function ensureSrc(repoRoot: string, srcDir: string, git: Deps["git"]): string {
  if (!existsSync(srcDir)) {
    const sha = fetchMain(repoRoot, git);
    mkdirSync(dirname(srcDir), { recursive: true });
    git(repoRoot, ["worktree", "add", "--detach", srcDir, sha]);
  } else if (git(srcDir, ["status", "--porcelain"]).trim()) {
    throw new Error(`取证检出 ${srcDir} 不干净，拒绝运行`);
  }
  const sha = fetchMain(srcDir, git);
  git(srcDir, ["checkout", "--detach", sha]);
  const head = git(srcDir, ["rev-parse", "HEAD"]).trim();
  if (head !== sha) throw new Error(`取证检出 HEAD ${head.slice(0, 10)} ≠ 远端 main ${sha.slice(0, 10)}，拒绝运行`);
  return sha;
}

function log(msg: string) {
  process.stdout.write(`[decision-claimer ${new Date().toISOString()}] ${msg}\n`);
}

async function round(a: Args, d: Deps) {
  let tickets = a.tickets;
  if (tickets.length === 0) {
    const children = a.maps.flatMap((m) => JSON.parse(d.gh(["api", "--paginate", `repos/${GH_REPO}/issues/${m}/sub_issues`])) as RawChild[]);
    tickets = selectCandidates(children, d.mode === "dry-run" ? finishedTickets(a.outRoot, children) : new Set()).slice(0, a.concurrency);
  }
  if (tickets.length === 0) {
    log("frontier 上没有可裁决的 grilling 票");
    return;
  }
  log(`${a.mode} 本轮：${tickets.map((n) => `#${n}`).join(" ")}`);
  const results = await runPool(tickets, a.concurrency, (n) => processTicket(n, d));
  for (const r of results) {
    log(`#${r.ticket} → ${r.phase}${r.klass ? ` ${r.klass}` : ""}${r.convertedToP ? "（转 P）" : ""}${r.landing ? ` 落点 ${r.landing}` : ""} 轮数 ${r.attempts} 用时 ${Math.round((r.durationsMs.total ?? 0) / 1000)}s${r.error ? ` 错误：${r.error}` : ""}`);
  }
}

async function main() {
  const a = parseArgs(process.argv.slice(2));
  const git: Deps["git"] = (cwd, args) => {
    if (a.mode === "dry-run" && isGitWrite(args)) throw new DryRunWriteError(`dry-run 拒绝 git ${args[0]}`);
    return execFileSync("git", ["-C", cwd, ...args], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
  };
  const gh = guardGh(a.mode, realGh);
  const srcDir = join(a.outRoot, ".src");
  const d: Deps = {
    mode: a.mode, gh, git, model: makeClaudeRunner(), outRoot: a.outRoot, srcDir, repoRoot: REPO_ROOT, now: Date.now,
    me: a.mode === "live" ? gh(["api", "user", "--jq", ".login"]).trim() : undefined,
  };
  for (;;) {
    try {
      const sha = ensureSrc(REPO_ROOT, srcDir, git);
      log(`取证检出对齐远端 main ${sha.slice(0, 10)}`);
      await round(a, d);
    } catch (e) {
      log(`本轮失败：${e instanceof Error ? e.message : String(e)}`);
      if (!a.loop) process.exitCode = 1;
    }
    if (!a.loop) break;
    await new Promise((r) => setTimeout(r, LOOP_INTERVAL_MS));
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  void main();
}
