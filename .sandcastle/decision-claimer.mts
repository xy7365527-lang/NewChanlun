#!/usr/bin/env node
/**
 * decision-claimer.mts —— graph engineering 裁决面（#1547）。
 *
 * 设计正本：.sandcastle/docs/graph-engineering.md「裁决流程」。每张 grilling 票：
 *   分级（Fable 5.1）→ 考古（Fable 5.1，两路搜，逐点 A/B/C）→ 起草（Fable 5.1）→ 独审（Opus 5.5，独立进程，只读草稿与源）
 *   → T 类 PASS：decision/<票号> 分支落正本 + resolution 评论；P 类：挂 needs-approval。
 *   其他 BLOCK：一次升档 Opus 5.5 带审查意见重做；仍 BLOCK：转 P 类。
 *
 *   只有 R1 引用类 BLOCK：一次只改引用的修订轮；总轮次上限 MAX_ROUNDS。起草后先查正本落点，越界退回重写一次。
 * 模式：默认 --dry-run，只写 /Volumes/AgentStorage/decisions/<票号>/，不写 GitHub。
 *       --live 才 claim（assign）、评论、打标签、推 decision 分支。
 * 调用：node --import tsx .sandcastle/decision-claimer.mts [--live] [--loop] [--ticket N ...]
 *       只验考古：node --import tsx .sandcastle/decision-claimer.mts --archaeology-only --ticket N --out <目录>
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
export const RULES = ["R1", "R2", "R3", "R4", "R5", "R6", "R7", "R8"] as const;
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
  /** 只跑考古阶段（须配 --ticket），不看票是否已关，不写 GitHub。 */
  archaeologyOnly: boolean;
}

export function parseArgs(argv: string[]): Args {
  const a: Args = { mode: "dry-run", loop: false, tickets: [], maps: [...DEFAULT_MAPS], outRoot: DEFAULT_OUT_ROOT, concurrency: MAX_CONCURRENT, archaeologyOnly: false };
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
    else if (v === "--archaeology-only") a.archaeologyOnly = true;
    else if (v === "--ticket") a.tickets.push(...next().split(",").map(toTicket));
    else if (v === "--maps") a.maps = next().split(",").map(toTicket);
    else if (v === "--out") a.outRoot = next();
    else if (v === "--concurrency") a.concurrency = Math.min(MAX_CONCURRENT, Math.max(1, Number(next())));
    else throw new Error(`未知参数 ${v}`);
  }
  if (a.archaeologyOnly && (a.mode === "live" || a.loop || a.tickets.length === 0)) throw new Error("--archaeology-only 只能 dry-run、单轮、配 --ticket");
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

/** R1–R8 审查解析：缺项、非法值、无依据一律记 BLOCK。 */
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
  // 输出里找不到任何 R1–R8 条目：这是解析失败，与审查者真给出 BLOCK 分开记。
  const parseError = !j || !RULES.some((r) => byRule.has(r));
  if (parseError) problems.push("审查输出无可解析的 R1–R8 JSON");
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

export type Phase = "classify" | "draft" | "review" | "land_t" | "shadow_p" | "review_parse_error" | "done" | "error" | "skipped";
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

export interface ModelCall { model: string; prompt: string; cwd: string; role: "classify" | "archaeology" | "draft" | "review" | "land"; allowWrite?: boolean }
export interface ModelResult { text: string; durationMs: number; attempts: number }
export type ModelRunner = (c: ModelCall) => Promise<ModelResult>;

/**
 * 上游临时错误：429/5xx、限流、过载、「暂时不可用」、网络超时与断连。其余错误（参数、权限、
 * 模型不存在等）不重试。#1530/#1565 落地步就是被「模型服务暂时不可用」一次打成 error。
 */
const RETRYABLE = new RegExp([
  String.raw`\b(429|500|502|503|504|529)\b`,
  String.raw`rate[ _-]?limit`, "overloaded", "too many requests", "service unavailable", "internal server error",
  "bad gateway", "gateway time-?out", String.raw`timed? ?out`, "ETIMEDOUT", "ECONNRESET", "ECONNREFUSED", "EAI_AGAIN",
  "socket hang up", "network error", "fetch failed",
  "暂时不可用", "稍后重试", "服务繁忙", "限流", "超时",
].join("|"), "i");

export function isRetryable(output: string): boolean {
  return RETRYABLE.test(output);
}

export function backoffMs(attempt: number, baseMs = 20_000, capMs = 300_000): number {
  return Math.min(capMs, baseMs * 2 ** (attempt - 1));
}

/** 只读取证：评论 id 只能用这两条 gh api 查询回查。 */
export const GH_API_READ_TOOLS = [
  `Bash(gh api repos/${GH_REPO}/issues/comments/*)`,
  `Bash(gh api repos/${GH_REPO}/issues/*/comments*)`,
];
export const READ_TOOLS = ["Read", "Grep", "Glob", "Bash(gh issue view:*)", "Bash(gh issue list:*)", "Bash(gh search issues:*)", ...GH_API_READ_TOOLS, "Bash(git log:*)", "Bash(git show:*)", "Bash(rg:*)"];
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

export interface OnceResult { code: number; stdout: string; stderr: string }
export type ClaudeOnce = (c: ModelCall, token: string, timeoutMs: number) => Promise<OnceResult>;

function claudeOnce(c: ModelCall, token: string, timeoutMs: number): Promise<OnceResult> {
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
    let timedOut = false;
    const timer = setTimeout(() => { timedOut = true; child.kill("SIGTERM"); }, timeoutMs);
    child.stdout.on("data", (d) => (stdout += d));
    child.stderr.on("data", (d) => (stderr += d));
    child.on("error", (e) => (stderr += `\nspawn error: ${e.message}`));
    child.on("close", (code) => {
      clearTimeout(timer);
      res({ code: code ?? 1, stdout, stderr: timedOut ? `${stderr}\nclaude 子进程超时（${Math.round(timeoutMs / 1000)}s timed out）` : stderr });
    });
    child.stdin.end(c.prompt);
  });
}

/** 临时错误的总尝试次数下限：首次 + 至少 4 次重试。 */
export const MIN_MODEL_ATTEMPTS = 5;

export function makeClaudeRunner(opts: {
  maxAttempts?: number; timeoutMs?: number; sleep?: (ms: number) => Promise<void>;
  once?: ClaudeOnce; token?: () => string;
} = {}): ModelRunner {
  const maxAttempts = Math.max(MIN_MODEL_ATTEMPTS, opts.maxAttempts ?? 6);
  const timeoutMs = opts.timeoutMs ?? 30 * 60 * 1000;
  const sleep = opts.sleep ?? ((ms) => new Promise((r) => setTimeout(r, ms)));
  const once = opts.once ?? claudeOnce;
  return async (c) => {
    const token = (opts.token ?? readProxyToken)();
    const t0 = Date.now();
    for (let attempt = 1; ; attempt++) {
      const r = await once(c, token, timeoutMs);
      let parsed: { result?: string; is_error?: boolean } | undefined;
      try { parsed = JSON.parse(r.stdout); } catch { /* 非 JSON */ }
      const ok = r.code === 0 && parsed && !parsed.is_error && typeof parsed.result === "string";
      if (ok) return { text: parsed!.result!, durationMs: Date.now() - t0, attempts: attempt };
      const blob = `${parsed?.result ?? r.stdout}\n${r.stderr}`;
      if (attempt < maxAttempts && isRetryable(blob)) {
        const wait = backoffMs(attempt);
        log(`${c.role} ${c.model} 第 ${attempt} 次临时错误，${Math.round(wait / 1000)}s 后重试：${redact(blob).trim().slice(0, 160)}`);
        await sleep(wait);
        continue;
      }
      throw new Error(`${c.role} ${c.model} 调用失败（exit ${r.code}，第 ${attempt} 次）：${redact(blob).slice(0, 600)}`);
    }
  };
}

function redact(s: string): string {
  return s.replace(/(token|key|authorization)["':=\s]+[A-Za-z0-9_\-.]{16,}/gi, "$1=<redacted>");
}

// ───────────────────────── 票面上下文 ─────────────────────────

export interface TicketContext {
  number: number;
  /** OPEN / CLOSED；旧夹具可缺省，缺省按 OPEN。 */
  state?: string;
  title: string;
  body: string;
  labels: string[];
  assignees: string[];
  comments: Array<{ author: string; body: string }>;
  blockers: Array<{ number: number; state: string; title: string }>;
}

export function loadTicket(gh: GhRunner, n: number): TicketContext {
  const v = JSON.parse(gh(["issue", "view", String(n), "--json", "number,state,title,body,labels,assignees,comments"])) as {
    number: number; state?: string; title: string; body: string;
    labels: Array<{ name: string }>; assignees: Array<{ login: string }>;
    comments: Array<{ author?: { login?: string }; body: string }>;
  };
  let blockers: TicketContext["blockers"] = [];
  try {
    blockers = (JSON.parse(gh(["api", `repos/${GH_REPO}/issues/${n}/dependencies/blocked_by`])) as Array<{ number: number; state: string; title: string }>)
      .map((b) => ({ number: b.number, state: b.state, title: b.title }));
  } catch { /* 依赖接口不可用时留空 */ }
  return {
    number: v.number, state: v.state, title: v.title, body: v.body ?? "",
    labels: v.labels.map((l) => l.name), assignees: v.assignees.map((a) => a.login),
    comments: v.comments.map((c) => ({ author: c.author?.login ?? "?", body: c.body })),
    blockers,
  };
}

/** 用户批复与 resolution 只认编排者账号的评论（全仓代理与用户共用此账号，外部账号的评论不算）。 */
export const ORCHESTRATOR = process.env.DECISION_ORCHESTRATOR ?? "xy7365527-lang";

/** 去掉评论首行的 Markdown 前缀（引用、标题、加粗、列表），只留正文。 */
function firstLine(body: string): string {
  const line = body.trimStart().split("\n", 1)[0] ?? "";
  return line.replace(/^(?:\s*(?:>|#{1,6}|\*\*|__|[-*]\s))+\s*/, "").replace(/^\*\*|\*\*$/g, "").trim();
}

/**
 * 重开：首行以退回 / 驳回 / 打回 / 不批准 / 不同意开头（#1530「## 退回重写（用户 … 批复：「同意」）」），
 * 或批复冒号后的内容以这些词开头（「用户 … 批复：退回重写」）。首行中间提到「重写」不算（「批复：同意——按草稿重写 §3」）。
 */
const REOPEN_HEAD = "(?:退回|驳回|打回|否决|不批准|不同意)";
const REOPEN_RE = new RegExp(`^${REOPEN_HEAD}|批复\\s*[：:]\\s*(?:\\*\\*|[「“"])*\\s*${REOPEN_HEAD}`);

/**
 * 首行「用户 … 批复」且不是「未 / 待」批复（#1565 先例「用户 2026-10-20 批复：「同意」…」）。
 * 影子评论「**P 类影子模式，待用户批复**」首行以「P 类」开头，不命中。
 */
function isUserResolution(line: string): boolean {
  const m = /^用户([^\n]{0,40}?)批复/.exec(line);
  return !!m && !/[未待没需]/.test(m[1]!);
}

/**
 * 已裁定 / 已关闭的票不得再进裁决面（图审计 2026-10-20 第 13 条：#1565 已按用户批复落正本，
 * claimer live 仍把它选进本轮）。按时间序扫编排者评论，最新一条有效：
 *   - 首行是 AUTH_LINE 或「**resolution**：」→ 已裁定；
 *   - 首行「用户 … 批复」→ 已裁定；但若同一首行或之后的评论首行带退回 / 驳回 / 重写，则重新打开。
 * 返回跳过原因；null = 可处理。纯函数。
 */
export function resolvedReason(t: Pick<TicketContext, "state" | "comments">, orchestrator = ORCHESTRATOR): string | null {
  if ((t.state ?? "OPEN").toUpperCase() === "CLOSED") return "票已关闭";
  let reason: string | null = null;
  for (const c of t.comments) {
    if (c.author !== orchestrator) continue;
    const line = firstLine(c.body);
    if (line.startsWith("P 类影子模式")) continue;
    if (c.body.trimStart().startsWith(AUTH_LINE) || /^resolution\s*[：:]/i.test(line)) reason = "票上已有 resolution";
    else if (REOPEN_RE.test(line)) reason = null;
    else if (isUserResolution(line)) reason = "票上已有用户批复";
    // 其余评论（簿记、进度）不改变结论。
  }
  return reason;
}

function renderTicket(t: TicketContext): string {
  const comments = t.comments.slice(-12).map((c) => `--- 评论 @${c.author} ---\n${c.body.slice(0, 4000)}`).join("\n\n");
  const blockers = t.blockers.map((b) => `- #${b.number} [${b.state}] ${b.title}`).join("\n") || "（无）";
  return `# #${t.number} ${t.title}\n标签：${t.labels.join(", ")}\n\n## 票面\n${t.body}\n\n## 阻塞本票的票\n${blockers}\n\n## 最近评论\n${comments || "（无）"}`;
}

// ───────────────────────── 考古（起草前） ─────────────────────────

/**
 * 考古阶段（用户 2026-10-20 令「所有的裁决票都要先考古」）。不是新机制，是三条既有规矩在无人值守裁决面上的执行：
 *   - /grilling「If a *fact* can be found by exploring the environment … look it up rather than asking me.
 *     The *decisions*, though, are mine」——事实查环境，决定才问人；
 *   - /domain-modeling（/grill-with-docs = /grilling + /domain-modeling）：对照 CONTEXT.md 与 docs/adr，
 *     已裁的直接用，与代码或词汇表冲突的当场指出；
 *   - AGENTS.md「开之前先搜（两路）」与 ADR 0012 裁定一/四（推论层主张可援引但推理链须重过；
 *     代价第 4 条「每张裁定票前多一个 AFK 子代理」——本阶段就是那个子代理）。
 * 本文件只补胶水：逐点 A/B/C 归类、可机械校验的查询证据、C 类缺口去向。
 *   A 已裁（正本层或已关票 resolution）——沿用，不再列为待决；
 *   B 有先例未正式裁（推论层、票评论、代码常量/夹具）——默认候选，以「默认值，确认或否决」呈现；
 *   C 确未设计——gap 指去向：decision 交用户（附推荐答案），research 开 research 票（/research 子代理），
 *     probe 缺数字派探针票（wayfinder-workflow 反面判据③）。
 */
export const ARCHAEOLOGY_FILE = "archaeology.md";
/** 草稿须附的「考古对照」段：每个考古点号一行，写明归类与草稿里怎么处置。 */
export const ARCH_SECTION = "考古对照";
export type ArchClass = "A" | "B" | "C";
export const ARCH_CLASSES: ArchClass[] = ["A", "B", "C"];
export interface ArchQuery { channel: "tracker" | "repo"; query: string; command: string; hits: number }
export interface ArchHit { source: string; excerpt: string }
export interface ArchPoint {
  id: string;
  question: string;
  klass: ArchClass;
  queries: ArchQuery[];
  hits: ArchHit[];
  /** A/B：出处（票号评论链接、path:line）。 */
  citation: string;
  /** B：从先例推到本票的推理链。 */
  reasoning: string;
  /** B：默认值；C：推荐值（有在案读数时）。 */
  recommended: string;
  /** C：已搜关键词。 */
  searchedKeywords: string[];
  /** C：缺口去向。 */
  gap: ArchGap | "";
  /** 先例之间或先例与代码互相冲突（/domain-modeling「Cross-reference with code」）。 */
  conflict: string;
}
export type ArchGap = "decision" | "research" | "probe";
export const ARCH_GAPS: ArchGap[] = ["decision", "research", "probe"];
export interface Archaeology { points: ArchPoint[] }

const str = (v: unknown) => (typeof v === "string" ? v.trim() : "");

/** 解析考古 JSON：{"points":[...]}；解析不出返回 null。 */
export function parseArchaeology(text: string): Archaeology | null {
  const j = extractJson(text) as { points?: unknown } | undefined;
  if (!j || !Array.isArray(j.points)) return null;
  const points = (j.points as Array<Record<string, unknown>>).map((p): ArchPoint => ({
    id: str(p.id),
    question: str(p.question),
    klass: str(p.class ?? p.klass).toUpperCase() as ArchClass,
    queries: (Array.isArray(p.queries) ? p.queries : []).map((q: Record<string, unknown>) => ({
      channel: str(q.channel) as ArchQuery["channel"], query: str(q.query), command: str(q.command), hits: Number(q.hits ?? 0) || 0,
    })),
    hits: (Array.isArray(p.hits) ? p.hits : []).map((h: Record<string, unknown>) => ({ source: str(h.source), excerpt: str(h.excerpt) })),
    citation: str(p.citation),
    reasoning: str(p.reasoning),
    recommended: str(p.recommended ?? p.default),
    searchedKeywords: (Array.isArray(p.searched_keywords ?? p.searchedKeywords) ? (p.searched_keywords ?? p.searchedKeywords) as unknown[] : []).map(str).filter(Boolean),
    gap: str(p.gap).toLowerCase() as ArchGap | "",
    conflict: str(p.conflict),
  }));
  return { points };
}

/** tracker 那一路必须覆盖已关闭票：`--state all` 或 `gh search issues`（默认不限状态）。 */
const TRACKER_ALL_STATES = /--state[\s=]+all|gh search issues/;

/** 考古产物的机械校验：返回问题清单，空 = 形状合格。 */
export function archaeologyProblems(a: Archaeology): string[] {
  const out: string[] = [];
  if (a.points.length === 0) out.push("考古没有列出任何待决点");
  const seen = new Set<string>();
  for (const p of a.points) {
    const id = p.id || "(无编号)";
    if (!/^Q\d+$/.test(p.id)) out.push(`${id}：编号须为 Q1、Q2…`);
    else if (seen.has(p.id)) out.push(`${id}：编号重复`);
    seen.add(p.id);
    if (!p.question) out.push(`${id}：缺待决点表述`);
    if (!ARCH_CLASSES.includes(p.klass)) out.push(`${id}：归类须为 A/B/C`);
    const tracker = p.queries.filter((q) => q.channel === "tracker" && q.query && q.command);
    const repo = p.queries.filter((q) => q.channel === "repo" && q.query && q.command);
    if (tracker.length === 0) out.push(`${id}：缺 tracker 一路查询证据`);
    else if (!tracker.some((q) => TRACKER_ALL_STATES.test(q.command))) out.push(`${id}：tracker 查询未覆盖已关闭票（须 --state all 或 gh search issues）`);
    if (repo.length === 0) out.push(`${id}：缺仓内一路查询证据`);
    if (p.hits.some((h) => !h.source)) out.push(`${id}：有命中缺出处`);
    if (p.klass === "A" || p.klass === "B") {
      if (!p.citation) out.push(`${id}：${p.klass} 类缺出处`);
      if (p.hits.length === 0) out.push(`${id}：${p.klass} 类缺命中`);
    }
    if (p.klass === "B") {
      if (!p.reasoning) out.push(`${id}：B 类缺推理链`);
      if (!p.recommended) out.push(`${id}：B 类缺默认值`);
    }
    if (p.klass === "C" && new Set(p.searchedKeywords).size < 2) out.push(`${id}：C 类须列出至少两个已搜关键词`);
    if (p.klass === "C" && !ARCH_GAPS.includes(p.gap as ArchGap)) out.push(`${id}：C 类须写 gap（decision / research / probe）`);
    if (p.klass === "A" && p.conflict) out.push(`${id}：有冲突的点不能归 A`);
  }
  return out;
}

const GAP_LABEL: Record<ArchGap, string> = { decision: "交用户决定", research: "开 research 票", probe: "派探针票测数" };
const cell = (s: string) => s.replace(/\|/g, "\\|").replace(/\n/g, " ");

/** archaeology.md：逐点 查询词 / 命中 / 归类，末尾附机械校验结果。 */
export function renderArchaeology(t: Pick<TicketContext, "number" | "title">, a: Archaeology, problems: string[]): string {
  const count = (k: ArchClass) => a.points.filter((p) => p.klass === k).length;
  const head = `# #${t.number} 考古\n\n${t.title}\n\n归类：A 已裁 ${count("A")} · B 有先例未正式裁 ${count("B")} · C 确未设计 ${count("C")}。只有 C 类可列为真正待决；B 类以「默认值，确认或否决」呈现；A 类直接沿用。\n`;
  const body = a.points.map((p) => {
    const qs = p.queries.map((q) => `| ${q.channel} | ${cell(q.query)} | \`${cell(q.command)}\` | ${q.hits} |`).join("\n");
    const hs = p.hits.map((h) => `- ${h.source}${h.excerpt ? `：${h.excerpt}` : ""}`).join("\n") || "- （无）";
    const tail = [
      p.citation && `- 出处：${p.citation}`,
      p.reasoning && `- 推理链：${p.reasoning}`,
      p.recommended && `- ${p.klass === "B" ? "默认值" : "推荐值"}：${p.recommended}`,
      p.searchedKeywords.length > 0 && `- 已搜关键词：${p.searchedKeywords.join("、")}`,
      p.gap && `- 缺口去向：${GAP_LABEL[p.gap]}`,
      p.conflict && `- 冲突：${p.conflict}`,
    ].filter(Boolean).join("\n");
    return `## ${p.id} [${p.klass}] ${p.question}\n\n| 路 | 查询词 | 命令 | 命中数 |\n|---|---|---|---|\n${qs}\n\n命中：\n${hs}\n\n${tail}\n`;
  }).join("\n");
  const check = problems.length ? `## 机械校验\n\n${problems.map((x) => `- ${x}`).join("\n")}\n` : "## 机械校验\n\n通过。\n";
  return `${head}\n${body}\n${check}`;
}

/** 草稿「考古对照」段须逐个点名考古点号。返回问题清单。 */
export function archaeologyCoverageProblems(draft: string, a: Archaeology): string[] {
  const m = new RegExp(`^#{1,3}\\s*(?:\\d+[.、]\\s*)?${ARCH_SECTION}\\s*$`, "m").exec(draft);
  if (!m) return [`草稿缺「${ARCH_SECTION}」段`];
  const rest = draft.slice(m.index + m[0].length).split(/^#{1,3}\s/m)[0];
  const missing = a.points.filter((p) => !new RegExp(`\\b${p.id}\\b`).test(rest)).map((p) => p.id);
  return missing.length ? [`「${ARCH_SECTION}」段漏了考古点 ${missing.join("、")}`] : [];
}

export function archaeologyPrompt(t: TicketContext, feedback?: string): string {
  return `你是 NewChanlun 仓裁决面的考古子代理，替起草者把本票的「事实」查完，只把「决定」留下。当前目录是 origin/main 的只读检出；禁止修改任何文件。
用户 2026-10-20 令：「所有的裁决票都要先考古」。先读这几处，按它们做：
- .agents/skills/grilling/SKILL.md：事实查环境，决定才问人；每个要问的决定附推荐答案。
- .agents/skills/domain-modeling/SKILL.md：对照 CONTEXT.md 与 docs/adr；「Challenge against the glossary」「Cross-reference with code」发现的冲突当场写出。
- .agents/skills/wayfinder/SKILL.md「Ticket Types」：仓外事实缺口是 research 票；docs/agents/wayfinder-workflow.md 反面判据③：缺数字派探针票。
- AGENTS.md「开之前先搜（两路，缺一不可）」与 docs/adr/0012-inference-layer-document-naming.md 裁定一、四：推论层主张可援引，推理链必须搬进来重过并现场打溯源标签。

${renderTicket(t)}
${feedback ? `\n## 上一次考古产物没过机械校验，必须逐条补齐：\n${feedback}\n` : ""}
本阶段要你交的（胶水，以上 skill 没规定的部分）：
1. 把票面与评论里的待决点逐个列出，编号 Q1、Q2…，一点一问。
2. 每点两路各至少一条查询，记下实际命令与命中数。tracker 路用 \`gh search issues "<词>" --repo ${GH_REPO}\` 或 \`gh issue list --repo ${GH_REPO} --state all --search "<词> in:comments"\`（覆盖已关闭票与评论）；仓内路用 rg -n 搜正本层 docs/adr/、.chanlun/definitions/、CONTEXT.md，推论层 analysis/、docs/、.chanlun/，以及代码常量与测试夹具（rust/、src/、tests/、trading_system/、scripts/）。同一概念换中文词、英文标识符、票号各搜一次。
3. 归类：A 已裁（正本层或已关票 resolution 明写）→ citation；B 有先例未正式裁（推论层、票评论、代码常量或夹具在用）→ citation + reasoning（ADR 0012 标签开头的推理链）+ recommended 默认值；C 确未设计 → searched_keywords（至少两个）+ gap：decision（交用户决定，recommended 写推荐答案）、research（仓外事实，开 research 票）、probe（缺读数，派探针票）。先例互相冲突或与代码不符时写 conflict，该点不归 A。

最后只输出一个 JSON 代码块：
{"points":[{"id":"Q1","question":"一句话待决点","class":"A|B|C",
  "queries":[{"channel":"tracker","query":"查询词","command":"实际命令","hits":3},{"channel":"repo","query":"查询词","command":"rg -n ...","hits":0}],
  "hits":[{"source":"#1348 评论 6262469214 或 docs/adr/0016-x.md:96","excerpt":"逐字摘一句"}],
  "citation":"A/B 必填","reasoning":"B 必填","recommended":"B 必填，C 可选","searched_keywords":["C 必填"],"gap":"C 必填","conflict":"有则填"}]}`;
}

/**
 * 呈现给用户的部分（/grilling：决定才问人，每问附推荐答案）：只问 B 类确认与 C 类 decision；
 * A 类列为已沿用；C 类 research/probe 是事实缺口，列为要开的票，不问用户。
 */
export function archaeologyAsks(a: Archaeology): string {
  const adopted = a.points.filter((p) => p.klass === "A").map((p) => `${p.id}（${p.citation}）`);
  const asks = a.points.filter((p) => p.klass === "B" || (p.klass === "C" && p.gap === "decision"));
  const gaps = a.points.filter((p) => p.klass === "C" && p.gap !== "decision");
  const rows = asks.map((p) => p.klass === "B"
    ? `| ${p.id} | B 确认或否决 | ${cell(p.question)} | ${cell(p.citation)} | ${cell(p.recommended)} |`
    : `| ${p.id} | C 待决 | ${cell(p.question)} | 已搜：${cell(p.searchedKeywords.join("、"))} | ${cell(p.recommended || "见草稿「候选」")} |`);
  const conflicts = a.points.filter((p) => p.conflict).map((p) => `- ${p.id} 冲突：${p.conflict}`);
  return `**考古**：A 已裁 ${adopted.length} 点直接沿用${adopted.length ? `（${adopted.join("；")}）` : ""}；需要你回答的只有下表。\n\n` +
    (rows.length ? `| 点 | 问法 | 待决点 | 出处 | 推荐值 |\n|---|---|---|---|---|\n${rows.join("\n")}` : "（无需用户回答的点）") +
    (gaps.length ? `\n\n事实缺口（不问你，开票补）：\n${gaps.map((p) => `- ${p.id} ${p.question} → ${GAP_LABEL[p.gap as ArchGap] ?? p.gap}`).join("\n")}` : "") +
    (conflicts.length ? `\n\n${conflicts.join("\n")}` : "");
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

export function draftPrompt(t: TicketContext, c: Classification, feedback?: string, arch?: string): string {
  return `你是 NewChanlun 仓 #1323 图的裁决起草者。当前目录是 origin/main 的只读检出；禁止修改任何文件，只读取证。
先读 AGENTS.md（教义正本、#799 收敛通则、#804 总缝规则、引原文四步核验）、CONTEXT.md、.sandcastle/docs/graph-engineering.md，以及票面点名的票（gh issue view <号> --comments）。评论 id 用 gh api repos/${GH_REPO}/issues/comments/<id> 或 gh api repos/${GH_REPO}/issues/<号>/comments 取，只读。
本票分级：${c.klass} 类 —— ${c.reason}
分级由分级器决定，你无权改判：按 ${c.klass} 类起草。若你认为分级有误，在「授权引用」段末另起一行写 \`改判建议：T 或 P — 理由\`，由编排逻辑处置；其余部分仍按 ${c.klass} 类写。

${renderTicket(t)}
${arch ? `\n## 考古结论（${ARCHAEOLOGY_FILE}，起草前已两路搜过）\n${arch}\n` : ""}${feedback ? `\n## 上一稿被独立审查 BLOCK，必须逐条处理：\n${feedback}\n` : ""}
输出一份 resolution 草稿（Markdown），先是以下七个二级标题，顺序一致、标题逐字相同；七段之后再附 \`## ${CITATION_SECTION}\`，最后附 \`## ${ARCH_SECTION}\`：
${DRAFT_SECTIONS.map((s) => `## ${s}`).join("\n")}
## ${CITATION_SECTION}
## ${ARCH_SECTION}

硬要求：
1. 权威依据逐条带可回查位置：docs/chanlun/text/blog/<课>.md:<行>、ADR 路径、.chanlun/definitions/ 路径或票评论链接。引缠师原文须核行号、逐字、作者归属（界前界后＋行首注）、行内注四步；核不过就不引。
2. 每条引用写进草稿前必须先用工具核实：文件引用用 Read 或 rg 看到该路径该行的原文；ADR 与定义文件引用先确认文件存在、所引条款在文件里逐字可见；票评论引用先用 gh api repos/${GH_REPO}/issues/comments/<id> 取到该评论并核对作者与内容。核实不到的路径、行号、条款、评论 id 一律不引，也不得凭记忆补写。
3. 「裁定」段必须单独一行写纯文本 \`正本落点：<路径>\`（行首就是「正本落点：」，路径不加反引号、粗体或其他修饰，行内不写别的）。路径只能在 .chanlun/definitions/ 下、docs/adr/ 下，或为 CONTEXT.md。${ADR_FORMAT_LINE}
4. 验收清单每项必须是以下之一：可执行命令、测试名、文件存在、票状态；判据二值。
5. 不越出 #1323 Destination；T 类草稿不得夹带 P 类取舍；不得新增同一判断的宽严两档或失败兜底。
6. 「受影响票与 blocking 边变化」列出全部受影响票号和每条要增删的 blocking 边。
7. 「授权引用」段：${c.klass === "T" ? `写「${AUTH_LINE}」并引用户 2026-10-18 原话` : "写「待用户批复（影子模式）」，不得写 T 类预授权句"}。
8. 「${CITATION_SECTION}」段是一张 Markdown 表，表头为 \`| 引用 | 核实命令 | 结果摘要 |\`，草稿里每一条引用（文件:行、ADR、定义文件条款、票号、评论 id）各占一行：写出你实际执行的核实命令与看到的结果摘要。表里没有的引用视为未核实。自证表里写不出亲眼看到的原文就整条删掉（连同正文里那条引用），不许写「未直接看到但结构确认」之类。
9. 按 /grill-with-docs 的做法用考古结论（.agents/skills/grilling/SKILL.md、.agents/skills/domain-modeling/SKILL.md）：A 类沿用其出处写进「权威依据」与「裁定」，不再列为待决；B 类在「候选」写成「默认值 <值>（出处 <位置>）——确认或否决」，推理链照考古重过；只有 C 类 decision 列为真正待决，每问附推荐答案。C 类 research/probe 是事实缺口，写进「受影响票与 blocking 边变化」作为要开的 research 票或探针票，不交用户填数。考古标出的冲突写进「问题」段。
10. 「${ARCH_SECTION}」段是一张表 \`| 点 | 归类 | 出处 | 草稿处置 |\`，考古里每个点号（Q1、Q2…）各占一行。
只输出草稿正文，不要前言或结语。`;
}

/** 修订轮：只改被审查点名的引用与自证表，裁定内容不动。 */
export function revisePrompt(t: TicketContext, c: Classification, prevDraft: string, feedback: string, arch?: string): string {
  return `${draftPrompt(t, c, undefined, arch)}

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

export function reviewPrompt(t: TicketContext, c: Classification, draft: string, arch?: string): string {
  return `你是 NewChanlun 仓 #1323 图裁决面的独立审查者。你看不到起草过程，只拿草稿与仓内源文件。当前目录是 origin/main 的只读检出；禁止修改任何文件。
需要时用 Read/Grep/rg 回查每条引用，用 gh issue view 查票，用 gh api repos/${GH_REPO}/issues/comments/<id> 或 gh api repos/${GH_REPO}/issues/<号>/comments 回查评论 id（只读）。审查依据：AGENTS.md、.chanlun/definitions/、docs/adr/、CONTEXT.md、.sandcastle/docs/graph-engineering.md「4. 独审」。

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
R8 考古成立。从下方 ${ARCHAEOLOGY_FILE} 抽查至少三条命中（不足三条全查），亲自复跑对应查询命令，命中与表述不符即 BLOCK；另自选一个 C 类点换一个说法再搜一次，搜出在案先例即 BLOCK。草稿把 A 类点当待决或交用户选、B/C 类点缺查询证据、「${ARCH_SECTION}」段漏点、C 类 research/probe 事实缺口被当成问题交给用户，或 P 类问题要求用户凭空提供数值而在案先例（考古 B 类或你复搜到的值）未列出，均 BLOCK。考古缺失按 BLOCK。

材料缺失（引用文件不存在、票读不到、工具调用失败）也必须照常输出：对受影响的条目判 BLOCK 并在依据里写明缺什么，其余条目照常审。任何情况下都要输出 R1–R8 全部八项。

${arch ? `===== ${ARCHAEOLOGY_FILE} 开始 =====\n${arch}\n===== ${ARCHAEOLOGY_FILE} 结束 =====\n\n` : `（本票没有 ${ARCHAEOLOGY_FILE}）\n\n`}===== 草稿开始 =====
${draft}
===== 草稿结束 =====

最后只输出一个 JSON 代码块：{"R1":{"verdict":"PASS|BLOCK","basis":"..."}, ... ,"R8":{...}}`;
}

export function reviewRetryPrompt(base: string): string {
  return `${base}

上一次输出无法解析为 R1–R8 JSON。这次回答的最后一段必须是一个 \`\`\`json 代码块，八个键 R1–R8 齐全，每项含 verdict（PASS 或 BLOCK）与 basis。`;
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
  /** 考古归类计数与机械校验问题。 */
  archaeology?: { A: number; B: number; C: number; problems: string[] };
  /** 落点硬检查退回记录。 */
  landingRedrafts?: string[];
  landing?: string;
  error?: string;
  /**
   * 断点：审查已过、写 GitHub 前落盘，写完才清。
   * reviewed_pass —— T 类审查全 PASS，落地未完成；下一轮跳过分级/起草/审查，从落地续做。
   * shadow_p_pending —— P 类结论已定，评论/贴标签/取消认领未全部完成；下一轮只补没做完的写入。
   */
  stage?: ResumeStage;
  /** 落地已推送的 commit；续做时只补 resolution 评论。 */
  landedSha?: string;
  /** shadow_p_pending 下已完成的写入步。 */
  pDone?: PStep[];
  /** 本次从哪个断点续做。 */
  resumedFrom?: ResumeStage;
}

export type ResumeStage = "reviewed_pass" | "shadow_p_pending";
export const P_STEPS = ["comment", "label", "unassign"] as const;
export type PStep = (typeof P_STEPS)[number];

/** 读上次 summary.json 的断点；草稿或审查原件缺失、审查不是 PASS 的 reviewed_pass 一律不认。 */
export function loadResume(dir: string): { prev: Summary; draft: string; review: Review } | null {
  const sp = join(dir, "summary.json");
  if (!existsSync(sp)) return null;
  let prev: Summary;
  try { prev = JSON.parse(readFileSync(sp, "utf8")) as Summary; } catch { return null; }
  if (prev.stage !== "reviewed_pass" && prev.stage !== "shadow_p_pending") return null;
  const dp = join(dir, "draft.md");
  const rp = join(dir, "review.json");
  if (!existsSync(dp) || !existsSync(rp)) return null;
  let rv: { verdict?: string; items?: ReviewItem[]; problems?: string[]; klass?: Klass };
  try { rv = JSON.parse(readFileSync(rp, "utf8")); } catch { return null; }
  if (prev.stage === "reviewed_pass" && (rv.verdict !== "PASS" || prev.klass !== "T")) return null;
  const review: Review = { items: rv.items ?? [], verdict: rv.verdict === "PASS" ? "PASS" : "BLOCK", problems: rv.problems ?? [], parseError: false };
  return { prev, draft: readFileSync(dp, "utf8"), review };
}

function writeJson(path: string, v: unknown) {
  writeFileSync(path, JSON.stringify(v, null, 2) + "\n");
}

function isGitWrite(args: string[]): boolean {
  return args[0] === "push";
}

export interface ArchRun { arch: Archaeology; md: string; problems: string[] }

/**
 * 跑考古：解析不出或机械校验不过时带问题重跑一次；仍解析不出抛错，仍有问题照常返回
 * （问题清单交审查，草稿因此 BLOCK）。写 archaeology.md 与 archaeology.json。
 */
export async function runArchaeology(t: TicketContext, d: Pick<Deps, "model" | "srcDir">, dir: string, summary?: Summary): Promise<ArchRun> {
  let fb: string | undefined;
  let arch: Archaeology | null = null;
  let problems: string[] = [];
  let raw = "";
  for (let i = 1; i <= 2; i++) {
    const r = await d.model({ model: MODEL_FABLE, prompt: archaeologyPrompt(t, fb), cwd: d.srcDir, role: "archaeology" });
    if (summary) summary.durationsMs[`archaeology${i}`] = r.durationMs;
    raw = r.text;
    arch = parseArchaeology(r.text);
    problems = arch ? archaeologyProblems(arch) : ["考古输出无可解析的 points JSON"];
    if (problems.length === 0) break;
    fb = problems.map((x) => `- ${x}`).join("\n");
  }
  if (!arch) {
    writeJson(join(dir, "archaeology.json"), { ticket: t.number, model: MODEL_FABLE, parsed: false, problems, raw });
    throw new Error("考古输出两次均无法解析为 points JSON");
  }
  const md = renderArchaeology(t, arch, problems);
  writeFileSync(join(dir, ARCHAEOLOGY_FILE), md);
  writeJson(join(dir, "archaeology.json"), { ticket: t.number, model: MODEL_FABLE, parsed: true, problems, ...arch, raw });
  if (summary) {
    const c = (k: ArchClass) => arch!.points.filter((p) => p.klass === k).length;
    summary.archaeology = { A: c("A"), B: c("B"), C: c("C"), problems };
  }
  return { arch, md, problems };
}

/** 读本地 archaeology.json（续做时给影子评论用）。 */
function loadArchaeology(dir: string): Archaeology | null {
  const p = join(dir, "archaeology.json");
  if (!existsSync(p)) return null;
  try {
    const j = JSON.parse(readFileSync(p, "utf8")) as { parsed?: boolean; points?: ArchPoint[] };
    return j.parsed && Array.isArray(j.points) ? { points: j.points } : null;
  } catch { return null; }
}

/** 只跑考古（不分级、不起草、不写 GitHub），不看票是否已关：用来对已关决策票验证考古产物形状。 */
export async function archaeologyOnly(n: number, d: Deps): Promise<{ ticket: number; dir: string; problems: string[]; counts: Record<ArchClass, number> }> {
  const dir = join(d.outRoot, String(n));
  mkdirSync(dir, { recursive: true });
  const t = loadTicket(d.gh, n);
  const r = await runArchaeology(t, d, dir);
  const c = (k: ArchClass) => r.arch.points.filter((p) => p.klass === k).length;
  return { ticket: n, dir, problems: r.problems, counts: { A: c("A"), B: c("B"), C: c("C") } };
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
  let skipPersist = false;
  // live 写 GitHub；dry-run 只登记意图，不触 gh 写命令。
  const write = (desc: string, args: string[], input?: string) => {
    if (d.mode === "dry-run") summary.plannedWrites.push(desc);
    else d.gh(args, input);
  };
  try {
    const t = loadTicket(d.gh, n);
    const resolved = resolvedReason(t);
    if (resolved) {
      // 不覆盖既有 summary.json（它记录的是上次真实处理的结局）。
      skipPersist = true;
      summary.phase = "skipped";
      summary.error = resolved;
      log(`#${n} 跳过：${resolved}`);
      return summary;
    }
    const resume = loadResume(dir);

    // claim 先行防重复；续做时允许票仍挂在自己名下（上次没来得及取消认领）。
    if (d.mode === "live") {
      const mine = resume && d.me && t.assignees.length === 1 && t.assignees[0] === d.me;
      if (t.assignees.length > 0 && !mine) throw new Error(`票已被 ${t.assignees.join(",")} 认领`);
      if (!mine) d.gh(["issue", "edit", String(n), "--add-assignee", "@me"]);
      claimed = true;
      const after = loadTicket(d.gh, n).assignees;
      if (after.length !== 1 || (d.me && after[0] !== d.me)) throw new Error(`claim 竞争：assignees=${after.join(",")}`);
    } else summary.plannedWrites.push(`claim：assign #${n}`);

    const persist = () => writeJson(join(dir, "summary.json"), summary);
    let s: RunState;
    let draft = "";
    let review: Review | undefined;
    if (resume) {
      // 续做：沿用上次的分级、轮次与审查结论，不再调用分级/起草/审查模型。
      const p = resume.prev;
      Object.assign(summary, {
        klass: p.klass, convertedToP: p.convertedToP, attempts: p.attempts, durationsMs: { ...p.durationsMs },
        reclassNotes: p.reclassNotes, reviewCounts: p.reviewCounts, rounds: p.rounds, landingRedrafts: p.landingRedrafts,
        stage: p.stage, landedSha: p.landedSha, pDone: p.pDone ?? [], resumedFrom: p.stage,
      });
      delete summary.durationsMs.total;
      draft = resume.draft;
      review = resume.review;
      const klass: Klass = p.stage === "reviewed_pass" ? "T" : "P";
      s = { ...initialState(klass), phase: p.stage === "reviewed_pass" ? "land_t" : "shadow_p", attempt: p.attempts, convertedToP: !!p.convertedToP };
      log(`#${n} 从断点 ${p.stage} 续做${p.landedSha ? `（已推送 ${p.landedSha.slice(0, 10)}）` : ""}`);
    } else {
    // 1. 分级
    const cr = await d.model({ model: MODEL_FABLE, prompt: classifyPrompt(t), cwd: d.srcDir, role: "classify" });
    const cls = parseClassification(cr.text);
    summary.durationsMs.classify = cr.durationMs;
    summary.klass = cls.klass;
    writeJson(join(dir, "classify.json"), { ticket: n, model: MODEL_FABLE, ...cls, durationMs: cr.durationMs, raw: cr.text });
    write(`评论 #${n}：分级：${cls.klass} — ${cls.reason}`, ["issue", "comment", String(n), "--body-file", "-"], `分级：${cls.klass} — ${cls.reason}`);

    // 1b. 考古：起草前两路搜，逐点 A/B/C。机械校验不过的问题交审查（草稿因此 BLOCK），升档重做时重跑考古。
    let ar = await runArchaeology(t, d, dir, summary);

    // 2–3. 起草 / 独审，至多两轮。分级由分级器定，起草者只能提改判建议。
    s = initialState(cls.klass);
    let effCls: Classification = cls;
    const counts = { pass: 0, block: 0, parseError: 0 };
    summary.reviewCounts = counts;
    summary.reclassNotes = [];
    summary.rounds = [];
    summary.landingRedrafts = [];
    while (s.phase === "draft") {
      summary.attempts = s.attempt;
      if (s.draftMode === "redo" && ar.problems.length) ar = await runArchaeology(t, d, dir, summary);
      const prompt = (fb?: string) =>
        s.draftMode === "revise" ? revisePrompt(t, effCls, draft, fb ?? s.reviewFeedback ?? "", ar.md) : draftPrompt(t, effCls, fb ?? s.reviewFeedback, ar.md);
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
      const extra = [
        ...draftClassProblems(draft, s.klass),
        ...ar.problems.map((x) => `考古：${x}`),
        ...archaeologyCoverageProblems(draft, ar.arch),
      ];
      const basePrompt = reviewPrompt(t, effCls, draft, ar.md);
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
        summary.error = "审查输出两次均无法解析为 R1–R8 JSON（review_parse_error），未计入 BLOCK";
        break;
      }
      s = nextAfterReview(s, review);
    }
    summary.klass = s.klass;
    summary.convertedToP = s.convertedToP;
    }

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
        // 审查已过：先落盘断点，落地任何一步失败，下一轮从这里续做。
        summary.stage = "reviewed_pass";
        persist();
        if (d.mode === "dry-run") {
          summary.plannedWrites.push(`分支 decision/${n}：写 ${target} 并推送`, `评论 #${n}：resolution（首行「${AUTH_LINE}」）`);
        } else {
          await landT(t, draft, target, dir, d, summary, persist);
        }
        delete summary.stage;
        delete summary.landedSha;
      }
    }
    if (s.phase === "shadow_p") {
      const done = new Set<PStep>(summary.pDone ?? []);
      summary.stage = "shadow_p_pending";
      summary.pDone = [...done];
      persist();
      const steps: Record<PStep, [string, string[], string?]> = {
        comment: [`评论 #${n}：P 类草稿与审查结论`, ["issue", "comment", String(n), "--body-file", "-"], shadowComment(n, draft, review!, dir, summary, loadArchaeology(dir))],
        label: [`标签 #${n}：+needs-approval`, ["issue", "edit", String(n), "--add-label", "needs-approval"]],
        unassign: [`取消认领 #${n}`, ["issue", "edit", String(n), "--remove-assignee", "@me"]],
      };
      for (const step of P_STEPS) {
        if (done.has(step)) continue;
        const [desc, args, input] = steps[step];
        write(desc, args, input);
        done.add(step);
        summary.pDone = [...done];
        persist();
        if (step === "unassign") claimed = false;
      }
      delete summary.stage;
      delete summary.pDone;
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
    if (!skipPersist) writeJson(join(dir, "summary.json"), summary);
    rmSync(lock, { force: true });
  }
  return summary;
}

export function shadowComment(n: number, draft: string, r: Review, dir: string, s: Summary, arch: Archaeology | null = null): string {
  const table = r.items.map((i) => `| ${i.rule} | ${i.verdict} | ${i.basis.replace(/\|/g, "\\|").replace(/\n/g, " ")} |`).join("\n");
  return `**P 类影子模式，待用户批复**${s.convertedToP ? `（原判 T，${s.attempts} 轮审查仍 BLOCK 后转 P）` : ""}\n\n` +
    (arch ? `${archaeologyAsks(arch)}\n\n考古原件：\`${dir}/${ARCHAEOLOGY_FILE}\`\n\n` : "") +
    `独立审查（${MODEL_OPUS}）：**${r.verdict}**\n\n| 条 | 结论 | 依据 |\n|---|---|---|\n${table}\n\n` +
    `草稿与审查原件：\`${dir}/\`\n\n<details><summary>草稿全文</summary>\n\n${draft}\n</details>`;
}

async function landT(t: TicketContext, draft: string, target: string, dir: string, d: Deps, s: Summary, persist: () => void) {
  const n = t.number;
  const branch = `decision/${n}`;
  if (s.landedSha) {
    // 上次已推送，只差 resolution 评论。
    d.gh(["issue", "comment", String(n), "--body-file", "-"], resolutionBody(n, target, branch, s.landedSha, dir, draft));
    return;
  }
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
  d.git(wt, ["commit", "-m", `decision(#${n}): ${t.title}\n\n${AUTH_LINE}；独审 ${MODEL_OPUS} R1–R8 PASS。`]);
  d.git(wt, ["push", FETCH_URL, `HEAD:refs/heads/${branch}`]);
  const sha = d.git(wt, ["rev-parse", "HEAD"]).trim();
  s.landedSha = sha;
  persist();
  d.gh(["issue", "comment", String(n), "--body-file", "-"], resolutionBody(n, target, branch, sha, dir, draft));
}

function resolutionBody(n: number, target: string, branch: string, sha: string, dir: string, draft: string): string {
  return `${AUTH_LINE}\n\n**resolution**：T 类，独立审查 R1–R8 全 PASS。正本 \`${target}\` 已写入分支 \`${branch}\`（${sha.slice(0, 10)}），待按日批量 PR 合入 main 后关票。\n\n` +
    `草稿：\`${dir}/draft.md\`；审查：\`${dir}/review.json\`\n\n<details><summary>草稿全文</summary>\n\n${draft}\n</details>`;
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

/**
 * 本地已走完的终态：落地、影子待批。带断点（stage）的一律不算，留给续做。
 * 要重跑一张已到终态的票（例如主控退回重写），先把本地目录改名（先例 decisions/1530.returned-20261020）。
 */
const FINISHED_PHASES: Phase[] = ["done", "land_t", "shadow_p"];

/** 本地 summary.json 已到终态的票（dry-run 与 live 都跳过）。 */
export function finishedTickets(outRoot: string, children: Array<{ number: number }>): Set<number> {
  const done = new Set<number>();
  for (const c of children) {
    const p = join(outRoot, String(c.number), "summary.json");
    if (!existsSync(p)) continue;
    try {
      const s = JSON.parse(readFileSync(p, "utf8")) as Summary;
      if (FINISHED_PHASES.includes(s.phase) && !s.stage) done.add(c.number);
    } catch { /* 坏文件视为未完成 */ }
  }
  return done;
}

/**
 * 从候选里逐张读票面，跳过已关闭 / 已有 resolution 或用户批复的票，凑满 limit 张。
 * 读票失败的候选本轮跳过（下一轮重试），不占并发槽。
 */
export function pickUnresolved(gh: GhRunner, candidates: number[], limit: number, onSkip: (n: number, why: string) => void): number[] {
  const out: number[] = [];
  for (const n of candidates) {
    if (out.length >= limit) break;
    let why: string | null;
    try { why = resolvedReason(loadTicket(gh, n)); } catch (e) { why = `读票失败：${e instanceof Error ? e.message.split("\n")[0] : String(e)}`; }
    if (why) onSkip(n, why);
    else out.push(n);
  }
  return out;
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
    // live 与 dry-run 同口径：本地已到终态的票、已关闭或已有 resolution/用户批复的票都不再选。
    const candidates = selectCandidates(children, finishedTickets(a.outRoot, children));
    tickets = pickUnresolved(d.gh, candidates, a.concurrency, (n, why) => log(`#${n} 跳过：${why}`));
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
  if (a.archaeologyOnly) {
    const sha = ensureSrc(REPO_ROOT, srcDir, git);
    log(`考古 only：取证检出对齐远端 main ${sha.slice(0, 10)}`);
    for (const n of a.tickets) {
      try {
        const r = await archaeologyOnly(n, d);
        log(`#${n} 考古 A ${r.counts.A} / B ${r.counts.B} / C ${r.counts.C}；机械校验${r.problems.length ? `问题 ${r.problems.length} 条：${r.problems.join("；")}` : "通过"}；${r.dir}/${ARCHAEOLOGY_FILE}`);
      } catch (e) {
        log(`#${n} 考古失败：${e instanceof Error ? e.message : String(e)}`);
        process.exitCode = 1;
      }
    }
    return;
  }
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
