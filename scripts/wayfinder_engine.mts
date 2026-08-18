#!/usr/bin/env node
/**
 * wayfinder_engine.mts —— 图清自动汇入主干管线自动化（#1078 裁定落地，#1084 实装）。
 *
 * 形态（#1078 裁定）：
 *   - DAG 走查器 = 纯函数：输入 tracker 状态（map 子票 + blocking 边 + 标签 + comments）
 *     → 输出触发动作清单；本文件内所有带「纯函数」注释的导出均可离线单测。
 *   - loop 壳 = 4 分钟轮询（watcher.sh / dispatcher 先例），每轮重算 ready 集、天然幂等；
 *   - stateless：不建状态库，tracker 即账本。
 *
 * 自动动作边界（#1078 裁定 4）：
 *   1. 图清（子票全关、frontier 空）→ 派 rlm 子代理起草 spec 草案 → 开图内 spec 票
 *      → @编排者批【闸一】；
 *   2. spec 批准（编排者 comment 批）→ 自动拆 tracer-bullet 实装票
 *      （blocking 边 + 票面逐条挂裁定票）+ 挂 sandcastle 标签 → 工蜂拾取；
 *   3. 实装票全关 + 图关判据满足 → 自动起草图关 comment + close map。
 *
 * 不碰决策票（v1）：决策票（含 Notes N-k 预授权）一律人工；纯决策图（spec 出图）也人工。
 * 人工闸三处不动：spec 批准（闸一）/ 不预授权决策票 / 合入 main（闸二，本引擎不碰 main）。
 *
 * 跑法（仓库根）：
 *   npx tsx scripts/wayfinder_engine.mts --once --dry-run   单轮演练（只打印动作，不执行）
 *   npx tsx scripts/wayfinder_engine.mts                    常驻轮询（默认 dry-run，4 分钟一轮）
 *   npx tsx scripts/wayfinder_engine.mts --live --once      真实执行一轮
 */

import { execSync } from "node:child_process";
import { rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

// ── 顶部常量（编排者 / 周期 / 起草子代理模型） ──────────────────────────────
// 环境变量覆盖：WAYFINDER_ORCHESTRATOR / WAYFINDER_POLL_SECONDS /
//               WAYFINDER_SPEC_MODEL / WAYFINDER_SPEC_PROVIDER。
// 默认 dry-run（安全默认）；真实执行须显式 --live。
const DEFAULT_ORCHESTRATOR = "xy7365527-lang";
const DEFAULT_POLL_SECONDS = 240; // 4 分钟（dispatcher 先例）
const DEFAULT_SPEC_MODEL = "k3";
const DEFAULT_SPEC_PROVIDER = "kimi-coding";
// 闸一待批的 spec 票标签：编排者需人工批，故挂 ready-for-human（三态 triage 标签之一）。
const SPEC_AWAIT_LABEL = "ready-for-human";
// 实装票标签：ready-for-agent（已就绪）+ sandcastle（工蜂拾取闸，main.mts 读取）。
const IMPL_LABELS = ["ready-for-agent", "sandcastle"];

// gh 在 FORCE_COLOR 环境下会给 --json 输出染色，必须洗掉（main.mts 同款）。
const GH_ENV: NodeJS.ProcessEnv = {
  ...process.env,
  NO_COLOR: "1",
  CLICOLOR: "0",
  FORCE_COLOR: "0",
  CLICOLOR_FORCE: "0",
  GH_REPO: process.env.GH_REPO ?? "xy7365527-lang/NewChanlun",
};

// ── 类型（tracker 状态的归一化视图，纯函数只吃这些） ─────────────────────────

export interface IssueComment {
  author: string;
  body: string;
  createdAt: string;
}

export interface ChildIssue {
  number: number;
  title: string;
  body: string;
  state: "OPEN" | "CLOSED";
  labels: string[];
  assignees: string[];
  /** open 阻塞边的数量（issue_dependencies_summary.blocked_by，非阻塞票号清单）。 */
  blockedBy: number;
  comments: IssueComment[];
}

export interface MapState {
  number: number;
  title: string;
  body: string;
  state: "OPEN" | "CLOSED";
  labels: string[];
  children: ChildIssue[];
}

export type Action =
  | { kind: "draft_spec"; map: number }
  | { kind: "split_impl_tickets"; map: number; spec: number }
  | { kind: "close_graph"; map: number };

export interface WalkOptions {
  /** 闸一批准人（编排者）login；spec 批准只认此人 comment。 */
  orchestrator: string;
}

export interface Analysis {
  phase: string;
  actions: Action[];
}

// ── 角色分类（纯函数） ────────────────────────────────────────────────────────

const DECISION_LABELS = [
  "wayfinder:grilling",
  "wayfinder:research",
  "wayfinder:prototype",
  "wayfinder:task",
];

/** 决策票：带 wayfinder:<type> 标签的子票（grilling/research/prototype/task）。 */
export function isDecisionTicket(c: Pick<ChildIssue, "labels">): boolean {
  return c.labels.some((l) => DECISION_LABELS.includes(l));
}

/** spec 票：标题以 📋 SPEC 或 [spec] 开头（#775/#1077 惯例）。 */
export function isSpecTicket(c: Pick<ChildIssue, "title">): boolean {
  return /^\s*📋\s*SPEC/i.test(c.title) || /^\s*\[spec\]/i.test(c.title);
}

/** 实装票：标题以 [impl] 开头（#1077 拆出的 S1–S5 惯例）。 */
export function isImplTicket(c: Pick<ChildIssue, "title">): boolean {
  return /^\s*\[impl\]/i.test(c.title);
}

/**
 * 纯决策图（v1 不碰，spec 出图走人工）：Notes 显式声明「本图为纯决策图，不带实装——理由：…」。
 * 判据取 wayfinder-workflow.md 的声明样板，避免误伤正文里泛泛提及「纯决策图」的引用句。
 */
export function isPureDecisionMap(map: Pick<MapState, "body">): boolean {
  return /本图为纯决策图|纯决策图[，,]\s*不带实装/.test(map.body);
}

// ── spec 批准检测（纯函数） ────────────────────────────────────────────────────

const APPROVAL_RE =
  /批准|通过|照此执行|按此拆票|approve|approved|lgtm|ship\s*it|^ack$/i;
const REJECT_RE = /不批准|不通过|打回|驳回|重写|reject|deny|nack|暂不/i;

/**
 * spec 是否已被编排者 comment 批准（闸一）。
 * 只认 orchestrator 的评论；按时间序推进——驳回可推翻先前的批准，反之亦然。
 * 注意 REJECT 先判，避免「不通过」被「通过」子串误命中。
 */
export function isSpecApproved(spec: ChildIssue, orchestrator: string): boolean {
  const mine = spec.comments
    .filter((c) => c.author === orchestrator)
    .sort((a, b) => (a.createdAt < b.createdAt ? -1 : a.createdAt > b.createdAt ? 1 : 0));
  let approved = false;
  for (const c of mine) {
    if (REJECT_RE.test(c.body)) approved = false;
    else if (APPROVAL_RE.test(c.body)) approved = true;
  }
  return approved;
}

// ── 图关判据：残雾必须有去向（纯函数） ─────────────────────────────────────────

/** 截取正文里某个 ## 标题到下一个 ## 标题之间的段落；不存在返回 null。 */
function extractSection(body: string, heading: string): string | null {
  const lines = body.split("\n");
  let start = -1;
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]!;
    if (/^#{1,6}\s+/.test(line) && line.includes(heading)) {
      start = i;
      break;
    }
  }
  if (start < 0) return null;
  const out: string[] = [];
  for (let i = start + 1; i < lines.length; i++) {
    if (/^#{1,6}\s+/.test(lines[i]!)) break;
    out.push(lines[i]!);
  }
  return out.join("\n");
}

/** 一条残雾是否有去向：毕业成票（#号）/ 明写搁置 / 移 Out of scope。 */
function fogItemResolved(item: string): boolean {
  if (/#\d+/.test(item)) return true;
  if (/issues\/\d+/.test(item)) return true;
  if (/搁置/.test(item)) return true;
  if (/out of scope/i.test(item)) return true;
  return false;
}

/**
 * 返回「## Not yet specified」段中尚无去向的残雾条目（列表项）。
 * 空数组 = 残雾全部有去向（或没有残雾段）。关图前必须为空。
 */
export function unresolvedFog(body: string): string[] {
  const section = extractSection(body, "Not yet specified");
  if (section === null) return [];
  const items: string[] = [];
  for (const line of section.split("\n")) {
    const t = line.trim();
    if (/^[-*•]\s+/.test(t) || /^\d+[.、]\s+/.test(t)) items.push(t);
  }
  return items.filter((i) => !fogItemResolved(i));
}

// ── spec 拆票计划解析（纯函数） ────────────────────────────────────────────────

export interface ImplTicketPlan {
  id: string;
  title: string;
  summary?: string;
  dependsOn: string[];
  rulings: number[];
}

function extractJsonFences(text: string): string[] {
  const out: string[] = [];
  const re = /```(?:json)?\s*\n([\s\S]*?)```/g;
  let m: RegExpExecArray | null;
  while ((m = re.exec(text)) !== null) {
    if (m[1]) out.push(m[1]!);
  }
  return out;
}

function tryParsePlan(text: string): ImplTicketPlan[] | null {
  let obj: unknown;
  try {
    obj = JSON.parse(text);
  } catch {
    return null;
  }
  if (!obj || typeof obj !== "object") return null;
  const tickets = (obj as { tickets?: unknown }).tickets;
  if (!Array.isArray(tickets) || tickets.length === 0) return null;
  const out: ImplTicketPlan[] = [];
  for (const t of tickets) {
    if (!t || typeof t !== "object") return null;
    const o = t as Record<string, unknown>;
    if (typeof o.id !== "string" || typeof o.title !== "string") return null;
    out.push({
      id: o.id,
      title: o.title,
      summary: typeof o.summary === "string" ? o.summary : undefined,
      dependsOn: Array.isArray(o.depends_on)
        ? (o.depends_on as unknown[]).filter((x): x is string => typeof x === "string")
        : [],
      rulings: Array.isArray(o.rulings)
        ? (o.rulings as unknown[]).filter((x): x is number => typeof x === "number")
        : [],
    });
  }
  return out;
}

/**
 * 从 spec 正文解析机器可读拆票计划（由 spec 起草子代理在
 * 「## 实施票拆分（机器可读）」节内用 wayfinder-split 标记 + json 围栏产出）。
 * 优先取标记后的围栏；兜底扫全文 json 围栏。解析不出返回 null（拆票走人工）。
 */
export function parseSplitPlan(specBody: string): ImplTicketPlan[] | null {
  const candidates: string[] = [];
  const marker = specBody.indexOf("wayfinder-split");
  if (marker >= 0) {
    const after = specBody.slice(marker);
    const first = extractJsonFences(after)[0];
    if (first) candidates.push(first);
  }
  candidates.push(...extractJsonFences(specBody));
  for (const text of candidates) {
    const plan = tryParsePlan(text);
    if (plan) return plan;
  }
  return null;
}

// ── DAG 走查器（纯函数）：tracker 状态 → 触发动作 ─────────────────────────────

/**
 * 单张图的走查。状态机（带实装图的完整生命周期）：
 *   走图中 → 图清(draft_spec) → spec 待批 → spec 批准(split) → 实装中 → 实装全关(close)。
 * 每个动作的触发条件在该动作执行后必然失效（stateless 幂等的根基）：
 *   draft_spec 后 spec 子票出现；split 后实装票出现；close 后图变 CLOSED。
 */
export function analyzeMap(map: MapState, opts: WalkOptions): Analysis {
  if (map.state !== "OPEN") return { phase: "已关（跳过）", actions: [] };
  if (isPureDecisionMap(map)) return { phase: "纯决策图（v1 不碰，spec 出图人工）", actions: [] };

  const spec = map.children.filter(isSpecTicket);
  const impl = map.children.filter(isImplTicket);
  const decision = map.children.filter(isDecisionTicket);

  // 决策票未全关：还在走图，机器不替人裁，也不越过未决决策去开 spec。
  if (decision.some((c) => c.state === "OPEN")) {
    return { phase: "走图中（决策票未全关）", actions: [] };
  }

  // 图清：无 spec、无实装票，且全部子票（决策票）已关。
  if (spec.length === 0 && impl.length === 0) {
    if (decision.length > 0 && map.children.every((c) => c.state === "CLOSED")) {
      return {
        phase: "图清 → 起草 spec 草案并开图内 spec 票（闸一）",
        actions: [{ kind: "draft_spec", map: map.number }],
      };
    }
    return { phase: "无决策子票（新图/空图，无动作）", actions: [] };
  }

  // spec 已存在：先判是否该拆票（幂等——拆到一半的图按计划补拆）。
  if (spec.length > 0) {
    const s = spec[0]!;
    if (s.state === "OPEN" && isSpecApproved(s, opts.orchestrator)) {
      const plan = parseSplitPlan(s.body);
      if (plan === null) {
        // 无机器可读拆票计划：actor 拿不到计划就不建票（只报「留人工」），impl 恒空 ⟹
        // 每轮重触发（stateless 无「已触发过」记忆），直到人工把计划补进 spec 正文或
        // 手动拆出实装票（impl 非空后本分支不再命中）——诚实标注，非「触发一次即停」。
        if (impl.length === 0) {
          return {
            phase: "spec 已批准 → 拆 tracer-bullet 实装票",
            actions: [{ kind: "split_impl_tickets", map: map.number, spec: s.number }],
          };
        }
      } else {
        const complete = plan.every((t) =>
          impl.some((i) => i.title.startsWith(`[impl] ${t.id} `)),
        );
        if (!complete) {
          return {
            phase: "spec 已批准 → 拆 tracer-bullet 实装票（补拆缺失票）",
            actions: [{ kind: "split_impl_tickets", map: map.number, spec: s.number }],
          };
        }
      }
    } else if (s.state === "OPEN") {
      return { phase: "spec 待批准（闸一）", actions: [] };
    }
    // spec 已关（或已批准且拆齐）→ 落到实装票判定。
  }

  // 实装票判定。
  if (impl.length > 0) {
    const allImplClosed = impl.every((c) => c.state === "CLOSED");
    if (allImplClosed) {
      const fog = unresolvedFog(map.body);
      if (fog.length === 0) {
        return {
          phase: "实装票全关 + 图关判据满足 → 起草图关 comment 并 close map",
          actions: [{ kind: "close_graph", map: map.number }],
        };
      }
      return {
        phase: `实装票全关但残雾无去向（${fog.length} 条）→ 不关图，留人工`,
        actions: [],
      };
    }
    return { phase: "实装中（工蜂拾取 / 评审 / 人工闸合入）", actions: [] };
  }

  // 有 spec、无实装票、且 spec 已关（异常）。
  return { phase: "spec 已关但无实装票（异常，人工处置）", actions: [] };
}

/** 纯函数走查器：单图 → 动作清单。 */
export function walkGraph(map: MapState, opts: WalkOptions): Action[] {
  return analyzeMap(map, opts).actions;
}

/** 纯函数走查器：多图 → 动作清单（每轮重算 ready 集）。 */
export function walkGraphs(maps: MapState[], opts: WalkOptions): Action[] {
  return maps.flatMap((m) => analyzeMap(m, opts).actions);
}

// ── gh 封装（IO 层，非纯） ─────────────────────────────────────────────────────

function gh(args: string): string {
  return execSync(`gh ${args}`, {
    encoding: "utf8",
    env: GH_ENV,
    maxBuffer: 32 * 1024 * 1024,
  }).trim();
}

function shellQuote(s: string): string {
  return `'${s.replace(/'/g, `'\\''`)}'`;
}

function issueUrl(number: number): string {
  const repo = GH_ENV.GH_REPO ?? "xy7365527-lang/NewChanlun";
  return `https://github.com/${repo}/issues/${number}`;
}

function normalizeState(s: string): "OPEN" | "CLOSED" {
  return s.toLowerCase() === "closed" ? "CLOSED" : "OPEN";
}

interface RawChild {
  number: number;
  title: string;
  body?: string | null;
  state: string;
  labels?: Array<{ name: string }>;
  assignees?: Array<{ login: string }>;
  issue_dependencies_summary?: { blocked_by?: number };
}

interface RawComment {
  user: { login: string };
  body?: string | null;
  created_at: string;
}

function fetchChildren(mapNumber: number): ChildIssue[] {
  const raw = JSON.parse(
    gh(`api --paginate repos/${GH_ENV.GH_REPO}/issues/${mapNumber}/sub_issues`),
  ) as RawChild[];
  return raw.map((c) => {
    const child: ChildIssue = {
      number: c.number,
      title: c.title,
      body: c.body ?? "",
      state: normalizeState(c.state),
      labels: (c.labels ?? []).map((l) => l.name),
      assignees: (c.assignees ?? []).map((a) => a.login),
      blockedBy: c.issue_dependencies_summary?.blocked_by ?? 0,
      comments: [],
    };
    // 只对 spec 子票拉评论（批准检测需要）；决策票的评论在起草 spec 时才按需拉。
    if (isSpecTicket(child)) child.comments = fetchComments(child.number);
    return child;
  });
}

function fetchComments(issueNumber: number): IssueComment[] {
  const raw = JSON.parse(
    gh(`api --paginate repos/${GH_ENV.GH_REPO}/issues/${issueNumber}/comments`),
  ) as RawComment[];
  return raw.map((c) => ({
    author: c.user.login,
    body: c.body ?? "",
    createdAt: c.created_at,
  }));
}

function fetchOpenMaps(): MapState[] {
  const out = gh(
    `issue list --label wayfinder:map --state open --limit 100 --json number,title,body,state,labels`,
  );
  const raw = JSON.parse(out) as Array<{
    number: number;
    title: string;
    body?: string | null;
    state: string;
    labels: Array<{ name: string }>;
  }>;
  return raw.map((m) => ({
    number: m.number,
    title: m.title,
    body: m.body ?? "",
    state: normalizeState(m.state),
    labels: m.labels.map((l) => l.name),
    children: fetchChildren(m.number),
  }));
}

// ── 动作执行器（IO 层；dry-run 下不调用） ──────────────────────────────────────

export interface EngineConfig {
  orchestrator: string;
  dryRun: boolean;
  pollSeconds: number;
  specModel: string;
  specProvider: string;
  once: boolean;
}

function createIssue(
  title: string,
  body: string,
  labels: string[],
): { number: number; id: number } {
  const tmp = join(
    tmpdir(),
    `wayfinder_issue_${Date.now()}_${Math.random().toString(36).slice(2)}.md`,
  );
  writeFileSync(tmp, body, "utf8");
  try {
    const labelArgs = labels.map((l) => `--label ${shellQuote(l)}`).join(" ");
    const out = execSync(
      `gh issue create --title ${shellQuote(title)} --body-file ${shellQuote(tmp)} ${labelArgs}`,
      { encoding: "utf8", env: GH_ENV, maxBuffer: 32 * 1024 * 1024 },
    ).trim();
    const m = out.match(/issues\/(\d+)\s*$/);
    if (!m) throw new Error(`无法从 gh issue create 输出解析票号：${out}`);
    const number = Number(m[1]);
    const id = Number(gh(`api repos/${GH_ENV.GH_REPO}/issues/${number} --jq .id`));
    return { number, id };
  } finally {
    rmSync(tmp, { force: true });
  }
}

function addSubIssue(mapNumber: number, childId: number): void {
  // sub_issue_id 须为整型（数据库 id），用 -F（typed）而非 -f（string），否则 422。
  gh(
    `api --method POST repos/${GH_ENV.GH_REPO}/issues/${mapNumber}/sub_issues -F sub_issue_id=${childId}`,
  );
}

function addBlockingEdge(childNumber: number, blockerDbId: number): void {
  gh(
    `api --method POST repos/${GH_ENV.GH_REPO}/issues/${childNumber}/dependencies/blocked_by -F issue_id=${blockerDbId}`,
  );
}

function issueDbId(number: number): number {
  return Number(gh(`api repos/${GH_ENV.GH_REPO}/issues/${number} --jq .id`));
}

/** 起草 spec 的 rlm 子代理：无头 prime-agent，只读、无工具，输出即 spec 正文。 */
function runSpecDrafter(prompt: string, cfg: EngineConfig): string {
  const out = execSync(
    `prime-agent -p --mode text --thinking off -nt --no-session ` +
      `--provider ${shellQuote(cfg.specProvider)} --model ${shellQuote(cfg.specModel)}`,
    { encoding: "utf8", input: prompt, env: process.env, maxBuffer: 32 * 1024 * 1024, timeout: 15 * 60 * 1000 },
  );
  return out.trim();
}

/** 拉一张图的全量上下文（供起草 spec / 组装图关 comment）。 */
function fetchMapContext(mapNumber: number): {
  number: number;
  title: string;
  body: string;
  children: Array<{ number: number; title: string; state: string; body: string; lastComment: string }>;
} {
  const map = JSON.parse(
    gh(`issue view ${mapNumber} --json number,title,body,state`),
  ) as { number: number; title: string; body: string; state: string };
  const children = fetchChildren(mapNumber);
  const ctx = children.map((c) => {
    const comments = fetchComments(c.number);
    return {
      number: c.number,
      title: c.title,
      state: c.state,
      body: c.body,
      lastComment: comments.length > 0 ? comments[comments.length - 1]!.body : "",
    };
  });
  return { number: map.number, title: map.title, body: map.body, children: ctx };
}

function truncate(s: string, n: number): string {
  return s.length <= n ? s : `${s.slice(0, n)}\n…（截断）`;
}

function buildSpecDraftPrompt(mapNumber: number): string {
  const ctx = fetchMapContext(mapNumber);
  const decisionList = ctx.children
    .filter((c) => c.state === "CLOSED")
    .map((c) => {
      return (
        `- #${c.number} ${c.title}\n` +
        `  正文：${truncate(c.body, 3000)}\n` +
        `  决议（末条评论）：${truncate(c.lastComment, 3000)}`
      );
    })
    .join("\n");

  return `你是一名 spec 起草子代理（只读文书，#1000 例外类）。为下面这张 wayfinder 图起草「图内实施总单 spec」（不出图，实施票在图内）。

## 硬性规则

1. 只读：不得创建/评论/关闭任何 issue，不得写任何仓库文件；只输出 spec 正文。
2. 输出格式（严格）：
   - 第一行必须是 H1 标题：\`# 📋 SPEC：<一句话主题>（map #${mapNumber} 实施总单）\`
   - 第二行声明：\`本 spec 是 map #${mapNumber} 的实施总单，实施票在图内。Part of #${mapNumber}。\`
   - 后续按需分节：\`## Problem Statement\` / \`## Solution\` / \`## Implementation Decisions\` / \`## Testing Decisions\` / \`## Out of Scope\`。
   - 末尾必须带「## 实施票拆分（机器可读）」节：先一行 \`<!-- wayfinder-split -->\`，然后一个 json 代码围栏，内容为：
     \`{"tickets":[{"id":"S1","title":"…","summary":"…","depends_on":[],"rulings":[<裁定票号>]},…]}\`
     id 用 S1/S2/…；depends_on 填依赖的其他 id（无则 []）；rulings 填该票逐条挂的裁定票号（数字数组）。
   - 除以上内容外，不输出任何前言/后语。
3. 口径：spec 是实施总单不是判决书——每条 Implementation Decision 挂裁定票号（正文用 #<号> 链接），决策正本在各自票内；本图已全关的决策票在下方给出。
4. 语言：简体中文。

## 图的现状（map #${mapNumber}）

${truncate(ctx.body, 20000)}

## 已关决策子票（决议在各自末条评论）

${decisionList || "（无）"}`;
}

/** 动作 1：图清 → 起草 spec → 开图内 spec 票 → @编排者批【闸一】。 */
function actDraftSpec(a: Extract<Action, { kind: "draft_spec" }>, cfg: EngineConfig): void {
  const mapNumber = a.map;
  const draft = runSpecDrafter(buildSpecDraftPrompt(mapNumber), cfg);

  // 拆 H1 标题与正文；声明行（本 spec 是 map #N 的实施总单）由本函数统一前置，
  // 起草稿里的同款声明行若存在则剥掉，避免重复。
  const lines = draft.split("\n");
  const h1Idx = lines.findIndex((l) => l.startsWith("# "));
  const title =
    h1Idx >= 0
      ? lines[h1Idx]!.replace(/^#\s+/, "").trim()
      : `📋 SPEC：实施总单（map #${mapNumber}）`;
  const rest = h1Idx >= 0 ? lines.slice(h1Idx + 1) : lines;
  while (rest.length > 0 && rest[0]!.trim() === "") rest.shift();
  if (rest.length > 0 && /实施总单/.test(rest[0]!) && /Part of #\d+/.test(rest[0]!)) rest.shift();
  while (rest.length > 0 && rest[0]!.trim() === "") rest.shift();
  const draftBody = rest.join("\n");

  const finalBody =
    `本 spec 是 map [#${mapNumber}](${issueUrl(mapNumber)}) 的实施总单，实施票在图内。Part of #${mapNumber}。\n\n` +
    `${draftBody}\n\n---\n@${cfg.orchestrator} 请批【闸一】：批准后引擎自动拆实装票（blocking 边 + 逐条挂裁定票 + sandcastle 标签）。\n`;

  const created = createIssue(title, finalBody, [SPEC_AWAIT_LABEL]);
  addSubIssue(mapNumber, created.id);
  console.log(`  ✓ 已开图内 spec 票 #${created.number}（挂 ${SPEC_AWAIT_LABEL}）→ @${cfg.orchestrator} 批【闸一】`);
}

/** 动作 2：spec 批准 → 拆 tracer-bullet 实装票 + blocking 边 + sandcastle 标签。 */
function actSplitImpl(a: Extract<Action, { kind: "split_impl_tickets" }>, _cfg: EngineConfig): void {
  const mapNumber = a.map;
  const specNumber = a.spec;
  const specBody = JSON.parse(
    gh(`issue view ${specNumber} --json body`),
  ).body as string;
  const plan = parseSplitPlan(specBody);
  if (!plan) {
    console.error(
      `  ✗ spec #${specNumber} 无机器可读拆票节（wayfinder-split），无法自动拆票 → 留人工。`,
    );
    return;
  }

  const existingImpl = fetchChildren(mapNumber).filter(isImplTicket);
  const existingTitle = new Set(existingImpl.map((c) => c.title));
  const createdByPlanId = new Map<string, { number: number; id: number }>();

  // 幂等：已存在的实装票（按标题含 [impl] <id> 判定）跳过，补建缺失的。
  for (const t of plan) {
    const expected = `[impl] ${t.id} `;
    const already = [...existingTitle].some((title) => title.startsWith(expected));
    if (already) {
      console.log(`  · 实装票 ${t.id} 已存在，跳过（幂等）`);
      const match = existingImpl.find((c) => c.title.startsWith(expected));
      if (match) createdByPlanId.set(t.id, { number: match.number, id: issueDbId(match.number) });
      continue;
    }
    const rulings = t.rulings
      .map((n) => `[#${n}](${issueUrl(n)})`)
      .join(" + ");
    const ticketBody =
      `Part of [SPEC #${specNumber}](${issueUrl(specNumber)})（map [#${mapNumber}](${issueUrl(mapNumber)}) 图内实施票）。` +
      (rulings ? `决策链：${rulings}。` : "") +
      `\n\n## 目标（spec ${t.id}）\n\n${t.summary ?? "（见 spec 对应条目）"}\n`;
    const created = createIssue(`[impl] ${t.id} ${t.title}`, ticketBody, IMPL_LABELS);
    addSubIssue(mapNumber, created.id);
    createdByPlanId.set(t.id, created);
    console.log(`  ✓ 已开实装票 #${created.number}（${t.id}，${IMPL_LABELS.join("+")}）`);
  }

  // blocking 边：depends_on 引用本计划内的 id。
  for (const t of plan) {
    const child = createdByPlanId.get(t.id);
    if (!child) continue;
    for (const depId of t.dependsOn) {
      const dep = createdByPlanId.get(depId);
      if (!dep) {
        console.error(`  ✗ ${t.id} depends_on 未知 id ${depId}，跳过该边（人工补接）`);
        continue;
      }
      addBlockingEdge(child.number, dep.id);
      console.log(`  ✓ blocking 边：#${child.number} blocked by #${dep.number}（${depId}）`);
    }
  }
}

/** 动作 3：实装票全关 + 判据满足 → 起草图关 comment + close map（合入 main 人工闸不动）。 */
function actCloseGraph(a: Extract<Action, { kind: "close_graph" }>, _cfg: EngineConfig): void {
  const mapNumber = a.map;
  const ctx = fetchMapContext(mapNumber);
  const spec = ctx.children.find((c) => isSpecTicket({ title: c.title } as ChildIssue));
  const impl = ctx.children.filter((c) => isImplTicket({ title: c.title } as ChildIssue));
  // 决策票全关只列决策票（spec/实装票各有独立行，混入会让「决策票全关」名不副实）。
  const decisionClosed = ctx.children.filter(
    (c) =>
      c.state === "CLOSED" &&
      !isSpecTicket({ title: c.title }) &&
      !isImplTicket({ title: c.title }),
  );
  const fog = unresolvedFog(ctx.body);

  const lines: string[] = [];
  lines.push(`## 图关闭（${new Date().toISOString().slice(0, 10)}）——到达 destination`);
  lines.push("");
  lines.push(`- **决策票全关**：${decisionClosed.map((c) => `[#${c.number}](${issueUrl(c.number)})`).join(" / ") || "（无）"}`);
  if (spec) lines.push(`- **spec**：[#${spec.number}](${issueUrl(spec.number)})（图内实施总单）`);
  lines.push(`- **实装票全关**：${impl.map((c) => `[#${c.number}](${issueUrl(c.number)})`).join(" / ") || "（无）"}`);
  lines.push(`- **残雾去向核实**：${fog.length === 0 ? "无残留（Not yet specified 已清空或全部有去向）" : `⚠ 仍有 ${fog.length} 条（不应发生）`}`);
  lines.push("");
  lines.push("（本提交由 wayfinder_engine 自动起草；合入 main 为人工闸【闸二】，本引擎不碰。）");

  const comment = lines.join("\n");
  // spec 非持久：实装落地即关。
  if (spec && spec.state === "OPEN") {
    gh(`issue close ${spec.number} --comment "实施票已全关，spec 落地即关（非持久件）。"`);
    console.log(`  ✓ 关闭 spec #${spec.number}`);
  }
  gh(`issue close ${mapNumber} --comment ${shellQuote(comment)}`);
  console.log(`  ✓ 关闭 map #${mapNumber}（comment 已附）`);
}

function executeAction(action: Action, cfg: EngineConfig): void {
  switch (action.kind) {
    case "draft_spec":
      return actDraftSpec(action, cfg);
    case "split_impl_tickets":
      return actSplitImpl(action, cfg);
    case "close_graph":
      return actCloseGraph(action, cfg);
  }
}

function describeAction(a: Action): string {
  switch (a.kind) {
    case "draft_spec":
      return `起草 spec 草案（rlm 子代理）→ 开图内 spec 票 → @编排者批【闸一】  (map #${a.map})`;
    case "split_impl_tickets":
      return `拆 tracer-bullet 实装票（blocking 边 + 逐条挂裁定票）+ sandcastle 标签  (map #${a.map}, spec #${a.spec})`;
    case "close_graph":
      return `起草图关 comment + close map  (map #${a.map})`;
  }
}

// ── 壳：轮询 + 每轮重算 ready 集 ───────────────────────────────────────────────

async function runOnce(cfg: EngineConfig): Promise<void> {
  const stamp = new Date().toISOString().slice(0, 19).replace("T", " ");
  console.log(`\n[wayfinder_engine] ${stamp} 一轮开始（${cfg.dryRun ? "dry-run" : "live"}）`);
  let maps: MapState[];
  try {
    maps = fetchOpenMaps();
  } catch (e) {
    console.error(`[wayfinder_engine] 拉取 tracker 状态失败（网络/鉴权？）：${e}`);
    return;
  }
  console.log(`  open 的 wayfinder:map 共 ${maps.length} 张`);
  for (const m of maps) {
    const analysis = analyzeMap(m, { orchestrator: cfg.orchestrator });
    console.log(`  · #${m.number} ${m.title} — ${analysis.phase}`);
    for (const action of analysis.actions) {
      if (cfg.dryRun) {
        console.log(`    [dry-run] 将执行：${describeAction(action)}`);
      } else {
        try {
          executeAction(action, cfg);
        } catch (e) {
          console.error(`    ✗ 执行失败（${action.kind}）：${e}`);
        }
      }
    }
  }
  console.log(`[wayfinder_engine] 一轮结束`);
}

function parseArgs(argv: string[]): EngineConfig {
  let dryRun = true;
  let once = false;
  let pollSeconds = Number(process.env.WAYFINDER_POLL_SECONDS ?? DEFAULT_POLL_SECONDS);
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i]!;
    if (a === "--live") dryRun = false;
    else if (a === "--dry-run") dryRun = true;
    else if (a === "--once") once = true;
    else if (a === "--poll-seconds") {
      const n = Number(argv[i + 1]);
      if (Number.isFinite(n) && n > 0) {
        pollSeconds = n;
        i++;
      }
    }
  }
  return {
    orchestrator: process.env.WAYFINDER_ORCHESTRATOR ?? DEFAULT_ORCHESTRATOR,
    dryRun,
    pollSeconds,
    specModel: process.env.WAYFINDER_SPEC_MODEL ?? DEFAULT_SPEC_MODEL,
    specProvider: process.env.WAYFINDER_SPEC_PROVIDER ?? DEFAULT_SPEC_PROVIDER,
    once,
  };
}

const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

async function main(): Promise<void> {
  const cfg = parseArgs(process.argv.slice(2));
  console.log(
    `[wayfinder_engine] 启动：dry-run=${cfg.dryRun} poll=${cfg.pollSeconds}s orchestrator=${cfg.orchestrator} spec 起草子代理=${cfg.specProvider}/${cfg.specModel}`,
  );
  if (cfg.once) {
    await runOnce(cfg);
    return;
  }
  for (;;) {
    try {
      await runOnce(cfg);
    } catch (e) {
      console.error(`[wayfinder_engine] 一轮异常：${e}`);
    }
    await sleep(cfg.pollSeconds * 1000);
  }
}

if ((import.meta as { main?: boolean }).main) {
  main();
}
