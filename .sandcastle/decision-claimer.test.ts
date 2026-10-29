import fs from "node:fs";
import { syncBuiltinESMExports } from "node:module";
import { execFileSync } from "node:child_process";
import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  ADR_FORMAT_LINE, AUTH_LINE, MAX_ROUNDS, isCitationOnlyBlock, landingProblem, revisePrompt, CITATION_SECTION, DRAFT_SECTIONS, GH_API_WRITE_DENY, READ_TOOLS, hasCitationTable, toolArgs, FETCH_URL, MAIN_REF, MODEL_DRAFTER, MODEL_OPUS, MODEL_MAIN, MODEL_REVIEW, DryRunWriteError,
  allocateAdr, applyReclass, draftPrompt, ensureSrc, extractFirstApiKey, guardGh, reclassSuggestion, renumberDraft, reviewPrompt, initialState, isGhWrite, landingTarget, missingDraftSections,
  nextAfterReview, parseArgs, parseClassification, parseReview, processTicket, selectCandidates,
  resolvedReason, pickUnresolved, finishedTickets,
  isRetryable, makeClaudeRunner, loadResume, isSafeguardBlock, safeguardFallbackModel, withSafeguardFallback, reviewerModel, modelFamily, assertDifferentModelFamilies, resumeStaleBecauseReopen,
  RULES, ARCH_SECTION, ARCHAEOLOGY_FILE, parseArchaeology, archaeologyProblems, renderArchaeology, archaeologyCoverageProblems,
  triagePrompt, extractJson, FIXED_CLASS_LINE, loadTriaged, loadEffectiveP, archaeologyAsks, archaeologyOnly, archaeologyOnlyAdmission, shadowComment, flowSectionsIn, stripFlowSections,
  type Deps, type GhRunner, type ModelCall, type OnceResult,
} from "./decision-claimer.mts";

const pass = (basis = "回查成立") => Object.fromEntries(RULES.map((r) => [r, { verdict: "PASS", basis }]));
const fence = (v: unknown) => "说明文字\n```json\n" + JSON.stringify(v) + "\n```\n";
const citeTable = `## ${CITATION_SECTION}\n| 引用 | 核实命令 | 结果摘要 |\n|---|---|---|\n| AGENTS.md:1 | rg -n x AGENTS.md | 见行 |`;
const archTable = `## ${ARCH_SECTION}\n| 点 | 归类 | 出处 | 草稿处置 |\n|---|---|---|---|\n| Q1 | A | docs/adr/0016-x.md:96 | 沿用 |\n| Q2 | B | #1355 评论 1 | 默认值，确认或否决 |\n| Q3 | C | — | 待决 |`;
const goodDraft = DRAFT_SECTIONS.map((s) => `## ${s}\n内容${s === "裁定" ? "\n正本落点：docs/adr/0099-x.md" : ""}`).join("\n\n") + "\n\n" + citeTable + "\n\n" + archTable + "\n";
const trackerQ = { channel: "tracker", query: "共同 cap", command: "gh search issues \"共同 cap\" --repo xy7365527-lang/NewChanlun", hits: 2 };
const repoQ = { channel: "repo", query: "cap", command: "rg -n cap docs/adr", hits: 1 };
const goodArchObj = {
  points: [
    { id: "Q1", question: "重的数量由谁定", class: "A", queries: [trackerQ, repoQ], hits: [{ source: "docs/adr/0016-x.md:96", excerpt: "全部推出来" }], citation: "docs/adr/0016-x.md:96" },
    { id: "Q2", question: "固定 Q 初值", class: "B", queries: [trackerQ, repoQ], hits: [{ source: "#1355 评论 1", excerpt: "夹具 Q=100" }], citation: "#1355 评论 1", reasoning: "夹具在用", recommended: "Q=100" },
    { id: "Q3", question: "多空共用容量", class: "C", queries: [trackerQ, repoQ], hits: [], searched_keywords: ["多空 容量", "long short capacity"], gap: "decision", recommended: "各占一份（依 ADR 0016:70 容量按重测）" },
  ],
};
const goodArch = fence(goodArchObj);
const FIXED = FIXED_CLASS_LINE;
/** 假模型的联合回答：分级回答与考古回答合成一个 JSON；考古不可解析时原样拼接。 */
const joint = (cls: string, arch: string) => {
  const c = extractJson(cls);
  const a = extractJson(arch);
  const obj = (v: unknown) => (v && typeof v === "object" && !Array.isArray(v) ? v as Record<string, unknown> : {});
  return a && typeof a === "object" ? fence({ ...obj(c), ...obj(a) }) : `${cls}\n${arch}`;
};

test("parseArgs 默认 dry-run，--live 才写", () => {
  assert.equal(parseArgs([]).mode, "dry-run");
  const a = parseArgs(["--live", "--ticket", "#1531,1532", "--concurrency", "9"]);
  assert.equal(a.mode, "live");
  assert.deepEqual(a.tickets, [1531, 1532]);
  assert.equal(a.concurrency, 3);
  assert.throws(() => parseArgs(["--bogus"]));
});

test("extractFirstApiKey 取 access.api-keys 首项，忽略其他段", () => {
  const y = "server:\n  api-keys:\n    - wrong\naccess:\n  other: 1\n  api-keys:\n    - \"first-key\"\n    - second\nauth:\n  x: y\n";
  assert.equal(extractFirstApiKey(y), "first-key");
  assert.equal(extractFirstApiKey("access:\n  api-keys: [abc, def]\n"), "abc");
  assert.equal(extractFirstApiKey("access:\n  foo: 1\n"), null);
});

test("分级解析：合法 T/P 照收，坏输出与缺理由一律判 P", () => {
  assert.deepEqual(parseClassification(fence({ class: "T", reason: "测试锁" })), { klass: "T", reason: "测试锁", parsed: true });
  assert.equal(parseClassification('前言 {"class":"p","reason":"资金参数"}').klass, "P");
  assert.equal(parseClassification("我觉得是 T").klass, "P");
  assert.equal(parseClassification(fence({ class: "T" })).klass, "P");
  assert.equal(parseClassification(fence({ class: "X", reason: "?" })).parsed, false);
});

test("审查解析：八项全 PASS 才 PASS；缺项、缺依据、草稿缺段都 BLOCK", () => {
  const ok = parseReview(fence(pass()));
  assert.equal(ok.verdict, "PASS");
  assert.equal(ok.items.length, 8);

  const missing = pass() as Record<string, unknown>;
  delete missing.R6;
  const r1 = parseReview(fence(missing));
  assert.equal(r1.verdict, "BLOCK");
  assert.equal(r1.items.find((i) => i.rule === "R6")?.verdict, "BLOCK");

  const noBasis = { ...pass(), R2: { verdict: "PASS", basis: "" } };
  assert.equal(parseReview(fence(noBasis)).verdict, "BLOCK");

  const arr = Object.entries(pass()).map(([rule, v]) => ({ rule, ...v }));
  assert.equal(parseReview(fence({ rules: arr })).verdict, "PASS");

  assert.equal(parseReview(fence(pass()), ["授权引用"]).verdict, "BLOCK");
  assert.equal(parseReview("没有 JSON").verdict, "BLOCK");
});

test("草稿七段与正本落点校验", () => {
  assert.deepEqual(missingDraftSections(goodDraft), []);
  assert.deepEqual(missingDraftSections("## 问题\n## 裁定\n"), DRAFT_SECTIONS.filter((s) => s !== "问题" && s !== "裁定"));
  assert.equal(landingTarget(goodDraft), "docs/adr/0099-x.md");
  assert.equal(landingTarget("正本落点：`CONTEXT.md`"), "CONTEXT.md");
  assert.equal(landingTarget("正本落点：**docs/adr/0041-level-isomorphism-test-lock.md**"), "docs/adr/0041-level-isomorphism-test-lock.md");
  assert.equal(landingTarget("正本落点：rust/src/lib.rs"), null);
  assert.equal(landingTarget("正本落点：__.chanlun/definitions/zhong_shu.md__"), ".chanlun/definitions/zhong_shu.md");
  assert.equal(landingTarget("正本落点：docs/adr/../../etc"), null);
});

test("状态转移：PASS 落地；BLOCK 升档 Opus 重做；再 BLOCK 转 P", () => {
  const passR = parseReview(fence(pass()));
  const blockR = parseReview(fence({ ...pass(), R3: { verdict: "BLOCK", basis: "验收项不可执行" } }));
  const t = initialState("T");
  assert.equal(t.drafterModel, MODEL_DRAFTER);
  assert.equal(nextAfterReview(t, passR).phase, "land_t");
  assert.equal(nextAfterReview(initialState("P"), passR).phase, "shadow_p");

  const t2 = nextAfterReview(t, blockR);
  assert.equal(t2.phase, "draft");
  assert.equal(t2.attempt, 2);
  assert.equal(t2.drafterModel, MODEL_OPUS);
  assert.match(t2.reviewFeedback ?? "", /R3 BLOCK：验收项不可执行/);

  const t3 = nextAfterReview(t2, blockR);
  assert.equal(t3.phase, "shadow_p");
  assert.equal(t3.klass, "P");
  assert.equal(t3.convertedToP, true);
  assert.equal(nextAfterReview(t2, passR).phase, "land_t");
});

test("isGhWrite / guardGh：dry-run 放行只读、拒绝写", () => {
  assert.equal(isGhWrite(["issue", "view", "1"]), false);
  assert.equal(isGhWrite(["api", "--paginate", "repos/x/issues/1/sub_issues"]), false);
  assert.equal(isGhWrite(["api", "repos/x/issues/1/labels", "-f", "labels[]=a"]), true);
  assert.equal(isGhWrite(["api", "-X", "PATCH", "repos/x/issues/1"]), true);
  for (const w of [["issue", "comment", "1"], ["issue", "edit", "1"], ["issue", "close", "1"], ["pr", "create"], ["label", "create"]]) {
    assert.equal(isGhWrite(w), true, w.join(" "));
  }
  const calls: string[][] = [];
  const g = guardGh("dry-run", (a) => (calls.push(a), "{}"));
  g(["issue", "view", "1"]);
  assert.throws(() => g(["issue", "comment", "1"]), DryRunWriteError);
  assert.deepEqual(calls, [["issue", "view", "1"]]);
});

test("selectCandidates：只取 open、grilling、未认领、无 blocker、未待批的票", () => {
  const g = [{ name: "wayfinder:grilling" }];
  const kids = [
    { number: 1, state: "OPEN", labels: g },
    { number: 2, state: "OPEN", labels: g, assignees: [{ login: "x" }] },
    { number: 3, state: "OPEN", labels: g, issue_dependencies_summary: { blocked_by: 1 } },
    { number: 4, state: "OPEN", labels: [...g, { name: "needs-approval" }] },
    { number: 5, state: "CLOSED", labels: g },
    { number: 6, state: "open", labels: [{ name: "wayfinder:task" }] },
    { number: 7, state: "open", labels: g },
    { number: 1, state: "OPEN", labels: g },
  ];
  assert.deepEqual(selectCandidates(kids, new Set([7])), [1]);
});

function fakeDeps(mode: "dry-run" | "live", replies: string[], archReplies: string[] = []) {
  const ghCalls: string[][] = [];
  const modelCalls: ModelCall[] = [];
  const archCalls: ModelCall[] = [];
  const raw: GhRunner = (args) => {
    ghCalls.push(args);
    if (args[0] === "issue" && args[1] === "view") {
      return JSON.stringify({ number: 1531, title: "FU-09", body: "票面", labels: [{ name: "wayfinder:grilling" }], assignees: [], comments: [] });
    }
    if (args[0] === "api") return "[]";
    return "";
  };
  const d: Deps = {
    mode,
    drainRoot: mkdtempSync(join(tmpdir(), "decision-drain-")),
    gh: guardGh(mode, raw),
    git: () => { throw new Error("dry-run 不应调 git"); },
    model: async (c) => {
      if (c.role === "triage") {
        archCalls.push(c);
        const arch = archReplies.shift() ?? goodArch;
        if (c.prompt.includes(FIXED)) return { text: arch, durationMs: 5, attempts: 1 };
        modelCalls.push(c);
        const cls = replies.shift();
        if (cls === undefined) throw new Error("模型调用次数超出预期");
        return { text: joint(cls, arch), durationMs: 5, attempts: 1 };
      }
      modelCalls.push(c);
      const text = replies.shift();
      if (text === undefined) throw new Error("模型调用次数超出预期");
      return { text, durationMs: 5, attempts: 1 };
    },
    outRoot: mkdtempSync(join(tmpdir(), "decision-claimer-test-")),
    srcDir: "/nonexistent-src",
    repoRoot: "/nonexistent-repo",
    now: Date.now,
  };
  return { d, ghCalls, modelCalls, archCalls };
}

const isWrite = (a: string[]) => isGhWrite(a);

test("dry-run 全流程：T 类 PASS 只落本地三份文件，不调任何 gh 写命令", async () => {
  const { d, ghCalls, modelCalls } = fakeDeps("dry-run", [fence({ class: "T", reason: "测试锁" }), goodDraft, fence(pass())]);
  const s = await processTicket(1531, d);
  assert.equal(s.phase, "land_t");
  assert.equal(s.error, undefined);
  assert.deepEqual(modelCalls.map((c) => [c.role, c.model]), [["triage", MODEL_MAIN], ["draft", MODEL_DRAFTER], ["review", MODEL_REVIEW]]);
  assert.ok(modelCalls.every((c) => !c.allowWrite));
  assert.equal(ghCalls.filter(isWrite).length, 0);
  const dir = join(d.outRoot, "1531");
  for (const f of ["classify.json", "draft.md", "review.json", "summary.json"]) assert.ok(existsSync(join(dir, f)), f);
  assert.equal(existsSync(join(dir, ".lock")), false);
  assert.equal(JSON.parse(readFileSync(join(dir, "review.json"), "utf8")).items.length, 8);
  assert.ok(s.plannedWrites.some((w) => w.startsWith("分支 decision/1531")));
});

test("dry-run 两轮 BLOCK：第二稿由 Opus 起草并带审查意见，最终转 P，仍不写 GitHub", async () => {
  const block = fence({ ...pass(), R1: { verdict: "BLOCK", basis: "行号漂移" }, R3: { verdict: "BLOCK", basis: "验收项不可执行" } });
  const { d, ghCalls, modelCalls } = fakeDeps("dry-run", [fence({ class: "T", reason: "接口" }), goodDraft, block, goodDraft, block]);
  const s = await processTicket(1531, d);
  assert.equal(s.phase, "shadow_p");
  assert.equal(s.klass, "P");
  assert.equal(s.convertedToP, true);
  assert.equal(modelCalls[3].model, MODEL_OPUS);
  assert.match(modelCalls[3].prompt, /R1 BLOCK：行号漂移/);
  assert.equal(ghCalls.filter(isWrite).length, 0);
  assert.ok(existsSync(join(d.outRoot, "1531", "review.attempt1.json")));
  assert.ok(s.plannedWrites.includes("标签 #1531：+needs-approval"));
});

test("live P 类：claim、分级评论、影子评论、needs-approval 依次写入", async () => {
  const { d, ghCalls } = fakeDeps("live", [fence({ class: "P", reason: "资金参数" }), goodDraft, fence(pass())]);
  let views = 0;
  const base = d.gh;
  d.gh = (args, input) => {
    if (args[0] === "issue" && args[1] === "view" && ++views > 1) {
      ghCalls.push(args);
      return JSON.stringify({ number: 1531, title: "FU-09", body: "", labels: [], assignees: [{ login: "me" }], comments: [] });
    }
    return base(args, input);
  };
  d.me = "me";
  const s = await processTicket(1531, d);
  assert.equal(s.phase, "shadow_p");
  const writes = ghCalls.filter(isWrite).map((a) => a.slice(0, 4).join(" "));
  assert.deepEqual(writes, [
    "issue edit 1531 --add-assignee",
    "issue comment 1531 --body-file",
    "issue comment 1531 --body-file",
    "issue edit 1531 --add-label",
    "issue edit 1531 --remove-assignee",
  ]);
});

// ───────── #1547 dry-run 批测缺陷回归 ─────────

/** #1536 dry-run 草稿原文节选：落点写在 ### 标题下一行。 */
const DRAFT_1536 = `## 裁定

采纳候选 B「完整域扩展」，同时补充以下约束与执行顺序：

### 正本落点

.chanlun/definitions/history-profile.md

（新建教义正本，记录完整历史数据六族 schema、数据质量三态语义与 ModelOrderUnspecified 分支的教义归属）
`;
/** #1537 dry-run 草稿原文节选：粗体标签 + 反引号路径。 */
const DRAFT_1537 = `## 裁定

**正本落点**：\`docs/adr/0038-dual-run-determinism-recovery-seek-asknown.md\`

### 1. 双跑确定性协议（候选 A）
`;

test("正本落点：#1536 标题下一行、#1537 粗体反引号、纯文本三种写法都能解析", () => {
  assert.equal(landingTarget(DRAFT_1536), ".chanlun/definitions/history-profile.md");
  assert.equal(landingTarget(DRAFT_1537), "docs/adr/0038-dual-run-determinism-recovery-seek-asknown.md");
  assert.equal(landingTarget("x\n正本落点：docs/adr/NNNN-foo.md\n"), "docs/adr/NNNN-foo.md");
  assert.equal(landingTarget("- 正本落点：`CONTEXT.md`。"), "CONTEXT.md");
  assert.equal(landingTarget("### 正本落点\n\n### 下一节\n"), null);
  assert.equal(landingTarget("### 正本落点\n\nrust/src/lib.rs\n"), null);
});

test("起草提示要求落点单独一行纯文本、ADR 写占位编号", () => {
  const t = { number: 1, title: "x", body: "", labels: [], assignees: [], comments: [], blockers: [] };
  const p = draftPrompt(t, { klass: "T", reason: "r", parsed: true });
  assert.match(p, /单独一行写纯文本/);
  assert.match(p, /docs\/adr\/NNNN-/);
  assert.match(p, /改判建议/);
  const pp = draftPrompt(t, { klass: "P", reason: "r", parsed: true });
  assert.ok(!pp.includes(`写「${AUTH_LINE}」`));
});

test("改判建议：转 P 即转，转 T 忽略并记录；无建议不动", () => {
  assert.deepEqual(reclassSuggestion("## 授权引用\n改判建议：P — 涉及真实账户"), { to: "P", reason: "涉及真实账户" });
  assert.equal(reclassSuggestion("无"), null);
  assert.deepEqual(applyReclass("T", { to: "P", reason: "x" }).klass, "P");
  const keep = applyReclass("P", { to: "T", reason: "纯 schema" });
  assert.equal(keep.klass, "P");
  assert.equal(keep.converted, false);
  assert.match(keep.note ?? "", /已忽略/);
  assert.deepEqual(applyReclass("T", null), { klass: "T", converted: false });
});

test("审查提示：R7 只审分级器结论，材料缺失也必须 BLOCK 并输出 JSON", () => {
  const t = { number: 1, title: "x", body: "", labels: [], assignees: [], comments: [], blockers: [] };
  const p = reviewPrompt(t, { klass: "P", reason: "账户", parsed: true }, "草稿");
  assert.match(p, /R7 分级器结论（P 类）正确/);
  assert.match(p, /材料缺失.*BLOCK/);
});

test("审查解析：无 JSON 记 parseError，与真 BLOCK 分开", () => {
  assert.equal(parseReview("没有 JSON").parseError, true);
  assert.equal(parseReview(fence({ foo: 1 })).parseError, true);
  const real = parseReview(fence({ ...pass(), R4: { verdict: "BLOCK", basis: "越界" } }));
  assert.equal(real.parseError, false);
  assert.equal(real.verdict, "BLOCK");
});

test("dry-run：审查首次解析失败重试一次，仍失败记 review_parse_error 不升档", async () => {
  const { d, modelCalls } = fakeDeps("dry-run", [fence({ class: "T", reason: "接口" }), goodDraft, "抱歉无法完成", "还是没有"]);
  const s = await processTicket(1531, d);
  assert.equal(s.phase, "review_parse_error");
  assert.deepEqual(s.reviewCounts, { pass: 0, block: 0, parseError: 1 });
  assert.equal(modelCalls.length, 4);
  assert.match(modelCalls[3].prompt, /无法解析/);
  const rv = JSON.parse(readFileSync(join(d.outRoot, "1531", "review.json"), "utf8"));
  assert.equal(rv.verdict, "review_parse_error");
  assert.deepEqual(rv.items, []);
});

test("dry-run：解析失败后重试成功照常走 PASS", async () => {
  const { d } = fakeDeps("dry-run", [fence({ class: "T", reason: "接口" }), goodDraft, "坏输出", fence(pass())]);
  const s = await processTicket(1531, d);
  assert.equal(s.phase, "land_t");
  assert.deepEqual(s.reviewCounts, { pass: 1, block: 0, parseError: 0 });
});

test("dry-run：T 类草稿建议转 P，编排转 P，审查按 P 类审", async () => {
  const draft = goodDraft + "\n改判建议：P — 含真实账户参数\n";
  const { d, modelCalls } = fakeDeps("dry-run", [fence({ class: "T", reason: "接口" }), draft, fence(pass())]);
  const s = await processTicket(1531, d);
  assert.equal(s.phase, "shadow_p");
  assert.equal(s.klass, "P");
  assert.equal(s.convertedToP, true);
  assert.match(modelCalls[2].prompt, /分级器结论：P 类/);
  assert.match(s.reclassNotes?.[0] ?? "", /建议转 P，已转/);
});

test("dry-run：P 类草稿建议转 T 被忽略；P 类草稿自写预授权句判 BLOCK", async () => {
  const draft = goodDraft + `\n${AUTH_LINE}\n改判建议：T — 纯技术\n`;
  const { d } = fakeDeps("dry-run", [fence({ class: "P", reason: "账户" }), draft, fence(pass()), draft, fence(pass())]);
  const s = await processTicket(1531, d);
  assert.equal(s.klass, "P");
  assert.match(s.reclassNotes?.[0] ?? "", /已忽略/);
  assert.deepEqual(s.reviewCounts, { pass: 0, block: 2, parseError: 0 });
});

test("ADR 编号：占位与草稿自编号都按 main 最大号顺序分配，并发不撞号", async () => {
  const root = mkdtempSync(join(tmpdir(), "adr-alloc-"));
  const adrDir = join(root, "adr");
  mkdirSync(adrDir);
  for (const f of ["0036-a.md", "0037-b.md", "README.md"]) writeFileSync(join(adrDir, f), "");
  const o = { adrDir, stateDir: join(root, "state"), reserve: true };
  const [a, b] = await Promise.all([
    allocateAdr("docs/adr/0038-dual-run.md", 1537, o),
    allocateAdr("docs/adr/0038-venue.md", 1540, o),
  ]);
  assert.deepEqual([a.number, b.number].sort(), [38, 39]);
  assert.equal((await allocateAdr("docs/adr/NNNN-dual-run.md", 1537, o)).number, a.number);
  assert.equal((await allocateAdr("docs/adr/0037-b.md", 1, o)).renumbered, false);
  assert.equal((await allocateAdr(".chanlun/definitions/x.md", 1, o)).number, null);
  const preview = await allocateAdr("docs/adr/NNNN-z.md", 9, { ...o, reserve: false });
  assert.equal(preview.number, 40);
  assert.equal((await allocateAdr("docs/adr/NNNN-z.md", 9, { ...o, reserve: false })).number, 40);
  assert.equal(renumberDraft("见 ADR NNNN；落点 docs/adr/NNNN-z.md", "docs/adr/NNNN-z.md", preview), "见 ADR 0040；落点 docs/adr/0040-z.md");
});

test("dry-run：T 类 ADR 落点按 main 顺序改号并写回草稿", async () => {
  const draft = DRAFT_SECTIONS.map((x) => `## ${x}\n内容${x === "裁定" ? "\n正本落点：docs/adr/NNNN-x.md\n依 ADR NNNN" : ""}`).join("\n\n") + "\n\n" + citeTable + "\n\n" + archTable;
  const { d } = fakeDeps("dry-run", [fence({ class: "T", reason: "接口" }), draft, fence(pass())]);
  d.srcDir = mkdtempSync(join(tmpdir(), "src-"));
  mkdirSync(join(d.srcDir, "docs/adr"), { recursive: true });
  writeFileSync(join(d.srcDir, "docs/adr/0037-b.md"), "");
  const s = await processTicket(1531, d);
  assert.equal(s.phase, "land_t");
  assert.match(s.landing ?? "", /^docs\/adr\/0038-x\.md/);
  assert.match(readFileSync(join(d.outRoot, "1531", "draft.md"), "utf8"), /ADR 0038/);
});

test("ensureSrc：在 srcDir 的库里用 ssh 地址 fetch，HEAD 与远端 main 不等即拒绝", () => {
  const src = mkdtempSync(join(tmpdir(), "src-"));
  const calls: Array<[string, string[]]> = [];
  let head = "abc123";
  const git = (cwd: string, args: string[]) => {
    calls.push([cwd, args]);
    if (args[0] === "status") return "";
    if (args[0] === "rev-parse" && args[1] === MAIN_REF) return "abc123\n";
    if (args[0] === "rev-parse") return head + "\n";
    return "";
  };
  assert.equal(ensureSrc("/repo", src, git), "abc123");
  assert.deepEqual(calls[1], [src, ["fetch", FETCH_URL, `+main:${MAIN_REF}`]]);
  assert.deepEqual(calls.find(([, a]) => a[0] === "checkout")?.[1], ["checkout", "--detach", "abc123"]);
  head = "stale99";
  assert.throws(() => ensureSrc("/repo", src, git), /拒绝运行/);
});

test("取证工具：起草/审查可只读 gh api 查评论，写形态一律拒", () => {
  assert.ok(READ_TOOLS.includes("Bash(gh api repos/xy7365527-lang/NewChanlun/issues/comments/*)"));
  const ro = toolArgs(false);
  const dis = ro.slice(ro.indexOf("--disallowedTools") + 1);
  for (const w of ["Edit", "Write", ...GH_API_WRITE_DENY]) assert.ok(dis.includes(w), w);
  assert.ok(GH_API_WRITE_DENY.some((x) => x.includes("-X")) && GH_API_WRITE_DENY.some((x) => x.includes("--input")));
  const rw = toolArgs(true);
  assert.ok(rw.includes("Edit") && rw.slice(rw.indexOf("--disallowedTools") + 1).every((x) => !["Edit", "Write"].includes(x)));
});

test("起草提示：先核实再引用、附引用自证表、ADR NNNN 与落点格式同一行带反例", () => {
  const t = { number: 1, title: "x", body: "", labels: [], assignees: [], comments: [], blockers: [] };
  const p = draftPrompt(t, { klass: "T", reason: "r", parsed: true });
  assert.match(p, /核实不到的路径、行号、条款、评论 id 一律不引/);
  assert.match(p, /\| 引用 \| 核实命令 \| 结果摘要 \|/);
  assert.ok(p.includes(`## ${CITATION_SECTION}`));
  const line = p.split("\n").find((l) => l.includes("单独一行写纯文本"));
  assert.ok(line && line.includes(ADR_FORMAT_LINE), "ADR 编号要求须与落点格式说明同一行");
  assert.match(ADR_FORMAT_LINE, /编号永远写 NNNN/);
  assert.match(ADR_FORMAT_LINE, /反例（禁止）：`正本落点：docs\/adr\/0041-/);
});

test("审查提示：R1 先对照引用自证表，表缺失即 BLOCK，可 gh api 回查评论", () => {
  const t = { number: 1, title: "x", body: "", labels: [], assignees: [], comments: [], blockers: [] };
  const p = reviewPrompt(t, { klass: "T", reason: "r", parsed: true }, "草稿");
  assert.match(p, /R1 .*引用自证」表：表缺失或为空，R1 直接 BLOCK/);
  assert.match(p, /gh api repos\/xy7365527-lang\/NewChanlun\/issues\/comments\/<id>/);
});

test("引用自证表：缺段或只有表头不算；缺表的草稿即使审查全 PASS 也 BLOCK", async () => {
  assert.equal(hasCitationTable(goodDraft), true);
  assert.equal(hasCitationTable(goodDraft.replace(citeTable, "")), false);
  assert.equal(hasCitationTable(`## ${CITATION_SECTION}\n| 引用 | 核实命令 | 结果摘要 |\n|---|---|---|\n`), false);
  const noTable = goodDraft.replace(citeTable, "");
  const { d } = fakeDeps("dry-run", [fence({ class: "T", reason: "接口" }), noTable, fence(pass()), noTable, fence(pass())]);
  const r = await processTicket(1, d);
  assert.notEqual(r.phase, "land_t");
});

// ───────── #1547 fix3：落点硬检查与只改引用的修订轮 ─────────

const r1Block = (basis: string) => fence({ ...pass(), R1: { verdict: "BLOCK", basis } });
const r3Block = fence({ ...pass(), R3: { verdict: "BLOCK", basis: "验收项不可执行" } });

test("落点硬检查：缺失与越界给出原因，合法返回 null", () => {
  assert.equal(landingProblem(goodDraft), null);
  assert.match(landingProblem("## 裁定\n无落点")!, /未声明/);
  assert.match(landingProblem("正本落点：rust/src/lib.rs")!, /rust\/src\/lib\.rs 越界/);
});

test("isCitationOnlyBlock：只有 R1 且依据指向具体引用才算", () => {
  assert.equal(isCitationOnlyBlock(parseReview(r1Block("replay.rs:283 核实命令没覆盖该行"))), true);
  assert.equal(isCitationOnlyBlock(parseReview(r1Block("CONTEXT.md 行号差一行"))), true);
  assert.equal(isCitationOnlyBlock(parseReview(r1Block("裁定思路有问题"))), false);
  assert.equal(isCitationOnlyBlock(parseReview(fence({ ...pass(), R1: { verdict: "BLOCK", basis: "行号差一行" }, R6: { verdict: "BLOCK", basis: "缺票" } }))), false);
  assert.equal(isCitationOnlyBlock(parseReview(r1Block("行号差一行"), ["授权引用"])), false);
});

test("状态转移：R1 引用 BLOCK 进修订轮，其他 BLOCK 升档，总轮次上限 3", () => {
  const cite = parseReview(r1Block("x.md:12 行号差一行"));
  const other = parseReview(r3Block);
  let s = nextAfterReview(initialState("T"), cite);
  assert.deepEqual([s.phase, s.attempt, s.draftMode, s.drafterModel], ["draft", 2, "revise", MODEL_OPUS]);
  s = nextAfterReview(s, other);
  assert.deepEqual([s.phase, s.attempt, s.draftMode], ["draft", 3, "redo"]);
  assert.equal(nextAfterReview(s, cite).phase, "shadow_p");
  assert.equal(MAX_ROUNDS, 3);
  // 升档后剩 R1 引用 BLOCK：第三轮修订；修订后仍 BLOCK 转 P
  let u = nextAfterReview(initialState("T"), other);
  assert.equal(u.draftMode, "redo");
  u = nextAfterReview(u, cite);
  assert.deepEqual([u.attempt, u.draftMode], [3, "revise"]);
  const end = nextAfterReview(u, cite);
  assert.deepEqual([end.phase, end.klass, end.convertedToP], ["shadow_p", "P", true]);
  // 修订用过后再遇 R1 引用 BLOCK 不再修订，走升档
  const v = nextAfterReview(nextAfterReview(initialState("T"), cite), cite);
  assert.equal(v.draftMode, "redo");
});

test("dry-run 修订轮：只拿上一稿与引用意见，不改裁定，再审 PASS 落地", async () => {
  const { d, modelCalls } = fakeDeps("dry-run", [fence({ class: "T", reason: "接口" }), goodDraft, r1Block("AGENTS.md:1 行号差一行"), goodDraft, fence(pass())]);
  const s = await processTicket(7, d);
  assert.equal(s.phase, "land_t");
  assert.equal(s.attempts, 2);
  const rev = modelCalls[3];
  assert.equal(rev.model, MODEL_OPUS);
  assert.match(rev.prompt, /只改引用」的修订轮/);
  assert.match(rev.prompt, /===== 上一稿开始 =====/);
  assert.match(rev.prompt, /R1 BLOCK：AGENTS\.md:1 行号差一行/);
  assert.deepEqual(s.rounds!.map((r) => [r.mode, r.verdict]), [["initial", "BLOCK"], ["revise", "PASS"]]);
});

test("dry-run 三轮仍 BLOCK 转 P：首稿 R3→升档→R1 引用→修订→仍 BLOCK", async () => {
  const cite = r1Block("x.md:3 转述多出内容");
  const { d, modelCalls } = fakeDeps("dry-run", [fence({ class: "T", reason: "接口" }), goodDraft, r3Block, goodDraft, cite, goodDraft, cite]);
  const s = await processTicket(8, d);
  assert.equal(s.phase, "shadow_p");
  assert.equal(s.convertedToP, true);
  assert.deepEqual(s.rounds!.map((r) => r.mode), ["initial", "redo", "revise"]);
  assert.equal(modelCalls.length, 7);
});

test("dry-run 落点越界：退回起草者重写一次，原因写进反馈", async () => {
  const bad = goodDraft.replace("docs/adr/0099-x.md", "rust/src/lib.rs");
  const { d, modelCalls } = fakeDeps("dry-run", [fence({ class: "T", reason: "接口" }), bad, goodDraft, fence(pass())]);
  const s = await processTicket(9, d);
  assert.equal(s.phase, "land_t");
  assert.match(modelCalls[2].prompt, /正本落点退回：正本落点 rust\/src\/lib\.rs 越界/);
  assert.equal(s.landingRedrafts!.length, 1);
  assert.ok(existsSync(join(d.outRoot, "9", "draft.attempt1.landing-rejected.md")));
});

test("提示：自证表看不到原文整条删；R4 落点越界即 BLOCK；修订提示保留落点", () => {
  const t = { number: 1, title: "x", body: "", labels: [], assignees: [], comments: [], blockers: [] };
  const c = { klass: "T" as const, reason: "r", parsed: true };
  assert.match(draftPrompt(t, c), /写不出亲眼看到的原文就整条删掉.*不许写「未直接看到但结构确认」/);
  assert.match(reviewPrompt(t, c, "草稿"), /R4 .*正本落点越界.*即 BLOCK/);
  assert.match(revisePrompt(t, c, "上稿", "- R1 BLOCK：x"), /正本落点保持原样/);
});

// ───────── #1547 fix4：临时错误重试与审查后断点续做 ─────────

test("isRetryable：429/5xx/暂时不可用/超时算临时，参数与权限错误不算", () => {
  for (const t of ["API Error: 模型服务暂时不可用，请稍后重试。", "API Error: 529 overloaded", "503 Service Unavailable",
    "Rate limit exceeded", "request timed out", "ECONNRESET", "API Error: 500 Internal Server Error"]) assert.equal(isRetryable(t), true, t);
  for (const t of ["Invalid model name", "permission denied: Write", "API Error: 400 bad request", "API Error: 401 unauthorized"]) assert.equal(isRetryable(t), false, t);
});

const okOnce = (text: string): OnceResult => ({ code: 0, stdout: JSON.stringify({ result: text, is_error: false }), stderr: "" });
const errOnce = (msg: string): OnceResult => ({ code: 1, stdout: JSON.stringify({ result: msg, is_error: true }), stderr: "" });
const call: ModelCall = { model: MODEL_OPUS, prompt: "p", cwd: "/", role: "land" };

test("makeClaudeRunner：临时错误指数退避重试直到成功", async () => {
  const seq = [errOnce("API Error: 模型服务暂时不可用，请稍后重试。"), errOnce("API Error: 503"), { code: 143, stdout: "", stderr: "claude 子进程超时（1800s timed out）" }, okOnce("好")];
  const waits: number[] = [];
  const run = makeClaudeRunner({ once: async () => seq.shift()!, token: () => "t", sleep: async (ms) => { waits.push(ms); } });
  const r = await run(call);
  assert.equal(r.text, "好");
  assert.equal(r.attempts, 4);
  assert.deepEqual(waits, [20_000, 40_000, 80_000]);
});

test("makeClaudeRunner：至少重试 4 次；非临时错误首次即失败", async () => {
  let n = 0;
  const flaky = makeClaudeRunner({ maxAttempts: 2, once: async () => (n++, errOnce("overloaded")), token: () => "t", sleep: async () => {} });
  await assert.rejects(flaky(call), /第 5 次/);
  assert.equal(n, 5);
  let m = 0;
  const hard = makeClaudeRunner({ once: async () => (m++, errOnce("Invalid model name")), token: () => "t", sleep: async () => {} });
  await assert.rejects(hard(call), /Invalid model name/);
  assert.equal(m, 1);
});

/** live 假环境：gh 维护 assignees 状态，git 按落地提示里的目标回报改动。 */
function liveDeps(replies: Array<string | Error>, opts: { failGh?: (a: string[]) => boolean; postedThenFail?: (a: string[]) => boolean } = {}) {
  const ghCalls: string[][] = [];
  const modelCalls: ModelCall[] = [];
  const fixedCalls: ModelCall[] = [];
  let assignees: string[] = [];
  let target = "";
  const comments: Array<{ author: { login: string }; body: string }> = [];
  const gh: GhRunner = (args, input) => {
    ghCalls.push(args);
    // postedThenFail：评论已发出、gh 却报错（#1565 那轮 GraphQL EOF 的同型）。
    if (opts.postedThenFail?.(args) && input !== undefined) comments.push({ author: { login: "xy7365527-lang" }, body: input });
    if (opts.failGh?.(args)) throw new Error(`gh 失败：${args.join(" ")}`);
    if (args[0] === "issue" && args[1] === "view") {
      return JSON.stringify({ number: 1565, title: "FU-x", body: "票面", labels: [{ name: "wayfinder:grilling" }], assignees: assignees.map((login) => ({ login })), comments });
    }
    if (args[0] === "issue" && args[1] === "comment" && input !== undefined) comments.push({ author: { login: "xy7365527-lang" }, body: input });
    if (args[0] === "issue" && args[1] === "edit" && args[3] === "--add-assignee") assignees = ["me"];
    if (args[0] === "issue" && args[1] === "edit" && args[3] === "--remove-assignee") assignees = [];
    if (args[0] === "api") return "[]";
    return "";
  };
  const gitCalls: string[][] = [];
  const d: Deps = {
    mode: "live", drainRoot: mkdtempSync(join(tmpdir(), "decision-drain-live-")), gh, me: "me",
    git: (_cwd, args) => {
      gitCalls.push(args);
      if (args[0] === "status") return `?? ${target}\n`;
      if (args[0] === "rev-parse") return "abcdef1234567890\n";
      return "";
    },
    model: async (c) => {
      if (c.role === "triage" && c.prompt.includes(FIXED)) {
        fixedCalls.push(c);
        return { text: goodArch, durationMs: 5, attempts: 1 };
      }
      modelCalls.push(c);
      if (c.role === "land") target = /写进正本 (\S+?)。/.exec(c.prompt)![1];
      const r = replies.shift();
      if (r === undefined) throw new Error("模型调用次数超出预期");
      if (r instanceof Error) throw r;
      return { text: c.role === "triage" ? joint(r, goodArch) : r, durationMs: 5, attempts: 1 };
    },
    outRoot: mkdtempSync(join(tmpdir(), "decision-claimer-live-")),
    srcDir: "/nonexistent-src",
    repoRoot: "/nonexistent-repo",
    now: Date.now,
  };
  return {
    d, replies, ghCalls, modelCalls, fixedCalls, gitCalls, comments,
    assign: (who: string[]) => { assignees = who; },
    fail: (f: (a: string[]) => boolean, posted = false) => { opts.failGh = f; if (posted) opts.postedThenFail = f; },
    stopFail: () => { opts.failGh = undefined; opts.postedThenFail = undefined; },
  };
}

const writesOf = (calls: string[][]) => calls.filter(isWrite).map((a) => a.slice(0, 4).join(" "));

test("live T 类：审查 PASS 后落地失败记 reviewed_pass，下一轮只做落地不重审", async () => {
  const { d, replies, ghCalls, modelCalls, gitCalls } = liveDeps([
    fence({ class: "T", reason: "接口" }), goodDraft, fence(pass()),
    new Error("land claude-opus-5-5 调用失败（exit 1，第 6 次）：API Error: 模型服务暂时不可用"),
  ]);
  const s1 = await processTicket(1565, d);
  assert.equal(s1.phase, "error");
  assert.equal(s1.stage, "reviewed_pass");
  const dir = join(d.outRoot, "1565");
  assert.equal(JSON.parse(readFileSync(join(dir, "summary.json"), "utf8")).stage, "reviewed_pass");
  assert.ok(loadResume(dir));
  assert.equal(writesOf(ghCalls).at(-1), "issue edit 1565 --remove-assignee");

  ghCalls.length = 0;
  modelCalls.length = 0;
  gitCalls.length = 0;
  replies.push("已写入");
  const s2 = await processTicket(1565, d);
  assert.equal(s2.phase, "land_t");
  assert.equal(s2.resumedFrom, "reviewed_pass");
  assert.equal(s2.stage, undefined);
  assert.deepEqual(modelCalls.map((c) => c.role), ["land"]);
  assert.equal(modelCalls[0].model, MODEL_MAIN, "落地是工具侧，必须用主控模型");
  assert.deepEqual(writesOf(ghCalls), ["issue edit 1565 --add-assignee", "issue comment 1565 --body-file"]);
  assert.ok(gitCalls.some((a) => a[0] === "push"));
  assert.equal(JSON.parse(readFileSync(join(dir, "summary.json"), "utf8")).stage, undefined);
  assert.equal(loadResume(dir), null);
});

test("live T 类：已推送只差评论时，续做只补 resolution 评论", async () => {
  const h = liveDeps([fence({ class: "T", reason: "接口" }), goodDraft, fence(pass()), "已写入"]);
  // 第一条评论是分级，第二条是 resolution：让 resolution 评论失败。
  h.fail((a) => a[1] === "comment" && h.ghCalls.filter((x) => x[1] === "comment").length > 1);
  const s1 = await processTicket(1565, h.d);
  assert.equal(s1.phase, "error");
  assert.equal(s1.landedSha, "abcdef1234567890");
  h.stopFail();
  h.ghCalls.length = 0;
  h.modelCalls.length = 0;
  h.gitCalls.length = 0;
  const s2 = await processTicket(1565, h.d);
  assert.equal(s2.phase, "land_t");
  assert.deepEqual(h.modelCalls, []);
  assert.ok(!h.gitCalls.some((a) => a[0] === "push"));
  assert.deepEqual(writesOf(h.ghCalls), ["issue edit 1565 --add-assignee", "issue comment 1565 --body-file"]);
});

test("live P 类：贴标签失败只记断点，下一轮只补贴标签与取消认领", async () => {
  const h = liveDeps([fence({ class: "P", reason: "资金参数" }), goodDraft, fence(pass())]);
  h.fail((a) => a[3] === "--add-label");
  const s1 = await processTicket(1565, h.d);
  assert.equal(s1.phase, "error");
  assert.equal(s1.stage, "shadow_p_pending");
  assert.deepEqual(s1.pDone, ["comment"]);
  h.stopFail();
  h.ghCalls.length = 0;
  h.modelCalls.length = 0;
  const s2 = await processTicket(1565, h.d);
  assert.equal(s2.phase, "shadow_p");
  assert.equal(s2.stage, undefined);
  assert.deepEqual(h.modelCalls, []);
  assert.deepEqual(writesOf(h.ghCalls), ["issue edit 1565 --add-assignee", "issue edit 1565 --add-label", "issue edit 1565 --remove-assignee"]);
});

// ───────── 图审计 2026-10-20 第 13 条：已裁定 / 已关闭票不再进裁决面 ─────────

test("resolvedReason：已关闭、resolution 评论、用户批复都跳过；待用户批复与正文提及不算", () => {
  const c = (body: string) => ({ author: "xy7365527-lang", body });
  assert.equal(resolvedReason({ state: "CLOSED", comments: [] }), "票已关闭");
  assert.equal(resolvedReason({ state: "OPEN", comments: [c(`${AUTH_LINE}\n\n**resolution**：T 类`)] }), "票上已有 resolution");
  assert.equal(resolvedReason({ state: "OPEN", comments: [c("用户 2026-10-20 批复：「同意」——采纳方案 A")] }), "票上已有用户批复");
  assert.equal(resolvedReason({ state: "OPEN", comments: [c("用户 2026-10-20 批复\n\n批复已落成正本")] }), "票上已有用户批复");
  // #1565 的影子评论：首行「待用户批复」，草稿里还可能写「需用户批复」——都不是批复。
  assert.equal(resolvedReason({ state: "OPEN", comments: [c("**P 类影子模式，待用户批复**（原判 T）\n草稿：此项需用户批复")] }), null);
  assert.equal(resolvedReason({ state: "OPEN", comments: [c("分级：T — 理由")] }), null);
  assert.equal(resolvedReason({ comments: [] }), null);
  // Markdown 前缀不挡识别。
  for (const b of ["**用户 2026-10-20 批复**：同意", "> 用户 2026-10-20 批复：同意", "## 用户 2026-10-20 批复", "- 用户批复：同意"]) {
    assert.equal(resolvedReason({ comments: [c(b)] }), "票上已有用户批复", b);
  }
  // 未批复 / 待批复 / 退回重写都不算已裁定；后来的退回推翻先前批复，再批复又生效。
  assert.equal(resolvedReason({ comments: [c("用户尚未批复，先挂着")] }), null);
  assert.equal(resolvedReason({ comments: [c("## 退回重写（用户 2026-10-20 批复：「同意」）")] }), null);
  assert.equal(resolvedReason({ comments: [c("用户 10-20 批复：退回重写")] }), null);
  assert.equal(resolvedReason({ comments: [c("用户 10-19 批复：同意"), c("## 退回重写（用户 10-20 批复）")] }), null);
  assert.equal(resolvedReason({ comments: [c("## 退回重写"), c("用户 10-21 批复：同意")] }), "票上已有用户批复");
  // 首行中间提到「重写」、批复后的簿记评论都不重开；「还没 / 需」批复不算已批复。
  assert.equal(resolvedReason({ comments: [c("用户 2026-10-20 批复：「同意」——按草稿重写 §3 后落地")] }), "票上已有用户批复");
  assert.equal(resolvedReason({ comments: [c("用户 10-20 批复：同意"), c("已按用户批复重写正本 baohan.md")] }), "票上已有用户批复");
  assert.equal(resolvedReason({ comments: [c("用户还没批复")] }), null);
  assert.equal(resolvedReason({ comments: [c("用户需批复后再落地")] }), null);
  assert.equal(resolvedReason({ comments: [c("用户 10-20 批复：「不同意」")] }), null);
  assert.equal(resolvedReason({ comments: [c("用户 10-20 批复：**不同意**")] }), null);
  assert.equal(resolvedReason({ comments: [c("用户 10-20 批复：否决")] }), null);
  // 非编排者账号不算；影子评论草稿里夹一行 **resolution**： 也不算。
  assert.equal(resolvedReason({ comments: [{ author: "someone", body: "用户批复：同意" }] }), null);
  assert.equal(resolvedReason({ comments: [c("**P 类影子模式，待用户批复**\n\n<details>\n**resolution**：草稿里的一行\n</details>")] }), null);
});

test("resolvedReason：首行带前导强调符号的批复（#1644 spec 批准评论）算已裁定，强调包着的待批复不算", () => {
  const c = (body: string) => ({ author: "xy7365527-lang", body });
  // #1644 真实评论首行，按时间序（2026-10-21，gh issue view 1644 --json comments）。
  const seq = [
    "分级：P — 票面自标 spec 级（ARCH.md:28/:168–169）：新增持久台账表……依「拿不准一律判 P」判 P。",
    "**P 类影子模式，待用户批复**",
    "**P 类影子模式：#1644 spec 修订草案（负责人稿，待用户 spec 批准闸）**",
    "**用户批复（#1644 spec 批准）**：用户 2026-10-21 原话「都同意」——四点均按推荐：① 台账单独文件；② 每次重启从原件重核全部历史。",
    "分级：P — 票面自标 spec 级……且已走「用户批复（#1644 spec 批准）」流程，依 graph-engineering.md「拿不准一律判 P」判 P。",
    "**草稿名分说明**：本票评论 decision-claimer 自动草稿已被取代：用户批复（评论 6308923666）批准的是负责人稿。",
  ];
  const expected = [null, null, null, "票上已有用户批复", "票上已有用户批复", "票上已有用户批复"];
  seq.forEach((_, k) => assert.equal(resolvedReason({ comments: seq.slice(0, k + 1).map(c) }), expected[k], seq[k]));
  // 斜体、粗斜体、下划线强调、引用叠加粗都认。
  for (const b of ["*用户 10-21 批复*：同意", "***用户批复***：同意", "__用户批复__：同意", "_用户批复_：同意", "> **用户批复（spec 批准）**：同意"]) {
    assert.equal(resolvedReason({ comments: [c(b)] }), "票上已有用户批复", b);
  }
  // 强调包着的待 / 未 / 需批复、强调包着的退回仍不算已裁定。
  for (const b of ["**待用户批复**", "*用户待批复*", "***用户尚未批复***", "__用户需批复后再落地__", "**用户批复：退回重写**"]) {
    assert.equal(resolvedReason({ comments: [c(b)] }), null, b);
  }
  // 列表符后跟空白仍按列表剥离。
  assert.equal(resolvedReason({ comments: [c("* 用户批复：同意")] }), "票上已有用户批复");
  // 批复冒号后用斜体 / 粗斜体 / 下划线包着的退回是退回，不是批准；首行斜体的退回同样重开。
  for (const b of ["用户批复：*退回重写*", "用户批复：***退回***", "用户批复：_退回_", "用户批复：__不同意__", "*退回重写*（用户批复：同意）", "**用户批复**：退回", "**用户 10-20 批复**：*退回*"]) {
    assert.equal(resolvedReason({ comments: [c("用户 10-20 批复：同意"), c(b)] }), null, b);
  }
  // 加粗的 resolution 标题行也算 resolution。
  assert.equal(resolvedReason({ comments: [c("**resolution**：T 类")] }), "票上已有 resolution");
});

test("pickUnresolved：逐张读票，跳过已裁定票不占并发槽，读票失败本轮跳过", () => {
  const views: Record<number, unknown> = {
    1565: { number: 1565, state: "CLOSED", title: "a", body: "", labels: [], assignees: [], comments: [] },
    1566: { number: 1566, state: "OPEN", title: "b", body: "", labels: [], assignees: [], comments: [{ author: { login: "xy7365527-lang" }, body: "用户 2026-10-20 批复：同意" }] },
    1567: { number: 1567, state: "OPEN", title: "c", body: "", labels: [], assignees: [], comments: [] },
    1569: { number: 1569, state: "OPEN", title: "e", body: "", labels: [], assignees: [], comments: [] },
  };
  const gh: GhRunner = (args) => {
    if (args[0] === "issue" && args[1] === "view") {
      const v = views[Number(args[2])];
      if (!v) throw new Error("EOF");
      return JSON.stringify(v);
    }
    return "[]";
  };
  const skipped: number[] = [];
  assert.deepEqual(pickUnresolved(gh, [1565, 1566, 1568, 1567, 1569], 1, (n) => skipped.push(n)), [1567]);
  assert.deepEqual(skipped, [1565, 1566, 1568]);
  assert.deepEqual(pickUnresolved(gh, [1565, 1567, 1569], 3, () => {}), [1567, 1569]);
});

test("finishedTickets：终态跳过，error 与带断点的不跳过", () => {
  const root = mkdtempSync(join(tmpdir(), "decision-finished-"));
  const put = (n: number, v: object) => { mkdirSync(join(root, String(n)), { recursive: true }); writeFileSync(join(root, String(n), "summary.json"), JSON.stringify(v)); };
  put(1, { phase: "done" });
  put(2, { phase: "shadow_p" });
  put(3, { phase: "error" });
  put(4, { phase: "land_t", stage: "reviewed_pass" });
  put(5, { phase: "review_parse_error" });
  assert.deepEqual([...finishedTickets(root, [1, 2, 3, 4, 5, 6].map((number) => ({ number })))].sort(), [1, 2]);
});

test("live processTicket：已有用户批复的票不 claim、不调模型、不覆盖 summary.json", async () => {
  const { d, ghCalls, modelCalls } = fakeDeps("live", []);
  const base = d.gh;
  d.gh = (args, input) => {
    if (args[0] === "issue" && args[1] === "view") {
      ghCalls.push(args);
      return JSON.stringify({ number: 1565, state: "OPEN", title: "W", body: "", labels: [{ name: "wayfinder:grilling" }], assignees: [], comments: [{ author: { login: "xy7365527-lang" }, body: "用户 2026-10-20 批复：「同意」" }] });
    }
    return base(args, input);
  };
  d.me = "me";
  mkdirSync(join(d.outRoot, "1565"), { recursive: true });
  writeFileSync(join(d.outRoot, "1565", "summary.json"), JSON.stringify({ phase: "error", error: "EOF" }));
  const s = await processTicket(1565, d);
  assert.equal(s.phase, "skipped");
  assert.equal(ghCalls.filter(isWrite).length, 0);
  assert.equal(modelCalls.length, 0);
  assert.equal(JSON.parse(readFileSync(join(d.outRoot, "1565", "summary.json"), "utf8")).phase, "error");
});

test("live processTicket：已关闭票直接跳过", async () => {
  const { d, ghCalls } = fakeDeps("live", []);
  const base = d.gh;
  d.gh = (args, input) => {
    if (args[0] === "issue" && args[1] === "view") return JSON.stringify({ number: 9, state: "CLOSED", title: "x", body: "", labels: [], assignees: [], comments: [] });
    return base(args, input);
  };
  const s = await processTicket(9, d);
  assert.equal(s.phase, "skipped");
  assert.equal(ghCalls.filter(isWrite).length, 0);
});

test("断点续做 × 跳过：reviewed_pass 且 resolution 评论其实已发出 → 下一轮 skipped，不调模型不写", async () => {
  const h = liveDeps([fence({ class: "T", reason: "接口" }), goodDraft, fence(pass()), "已写入"]);
  // 第二条评论（resolution）发出后 gh 报错。
  h.fail((a) => a[1] === "comment" && h.ghCalls.filter((x) => x[1] === "comment").length > 1, true);
  const s1 = await processTicket(1565, h.d);
  assert.equal(s1.phase, "error");
  assert.ok(h.comments.some((c) => c.body.startsWith(AUTH_LINE)));
  h.stopFail();
  h.ghCalls.length = 0;
  h.modelCalls.length = 0;
  const s2 = await processTicket(1565, h.d);
  assert.equal(s2.phase, "skipped");
  assert.deepEqual(h.modelCalls, []);
  assert.deepEqual(writesOf(h.ghCalls), []);
  // 断点原件保留，未被 skipped 覆盖。
  assert.equal(JSON.parse(readFileSync(join(h.d.outRoot, "1565", "summary.json"), "utf8")).stage, "reviewed_pass");
});

test("断点续做 × 退回：reviewed_pass 之后出现退回，下一轮整链重跑，不沿用旧 PASS 落地", async () => {
  const h = liveDeps([
    fence({ class: "T", reason: "接口" }), goodDraft, fence(pass()),
    new Error("land claude-opus-5-5 调用失败（exit 1，第 6 次）：API Error: 模型服务暂时不可用"),
  ]);
  const s1 = await processTicket(1565, h.d);
  assert.equal(s1.stage, "reviewed_pass");
  assert.equal(s1.commentsSeen, 0);
  h.comments.push({ author: { login: "xy7365527-lang" }, body: "## 退回重写\n这稿不能落地" });
  h.replies.push(fence({ class: "T", reason: "按退回重写" }), goodDraft, fence(pass()), "已写入");
  h.modelCalls.length = 0;
  h.gitCalls.length = 0;
  const s2 = await processTicket(1565, h.d);
  assert.equal(s2.resumedFrom, undefined);
  assert.deepEqual(h.modelCalls.map((c) => c.role), ["triage", "draft", "review", "land"]);
  assert.equal(s2.phase, "land_t");
  assert.ok(h.gitCalls.some((a) => a[0] === "push"));
});

test("断点续做：退回出现在断点之前，或之后只有普通评论，仍只补落地", async () => {
  const h = liveDeps([
    fence({ class: "T", reason: "接口" }), goodDraft, fence(pass()),
    new Error("land claude-opus-5-5 调用失败（exit 1，第 6 次）：API Error: 模型服务暂时不可用"),
  ]);
  h.comments.push({ author: { login: "xy7365527-lang" }, body: "## 退回重写\n上一稿不要" });
  const s1 = await processTicket(1565, h.d);
  assert.equal(s1.stage, "reviewed_pass");
  assert.equal(s1.commentsSeen, 1);
  h.comments.push({ author: { login: "xy7365527-lang" }, body: "已按用户批复重写正本 baohan.md" });
  h.comments.push({ author: { login: "someone" }, body: "## 退回重写" });
  h.replies.push("已写入");
  h.modelCalls.length = 0;
  h.gitCalls.length = 0;
  const s2 = await processTicket(1565, h.d);
  assert.equal(s2.phase, "land_t");
  assert.equal(s2.resumedFrom, "reviewed_pass");
  assert.deepEqual(h.modelCalls.map((c) => c.role), ["land"]);
  assert.ok(h.gitCalls.some((a) => a[0] === "push"));
});

test("resumeStaleBecauseReopen：只认断点水印之后、编排者账号的退回", () => {
  const reopen = { author: "xy7365527-lang", body: "用户 10-20 批复：「不同意」" };
  const keep = { author: "xy7365527-lang", body: "分级：T — 接口" };
  assert.equal(resumeStaleBecauseReopen({ commentsSeen: 1 }, [reopen, keep]), false);
  assert.equal(resumeStaleBecauseReopen({ commentsSeen: 1 }, [keep, reopen]), true);
  assert.equal(resumeStaleBecauseReopen({}, [reopen]), true);
  assert.equal(resumeStaleBecauseReopen({ commentsSeen: 0 }, [{ author: "someone", body: "## 退回重写" }]), false);
});

test("断点续做 × 跳过：shadow_p_pending 票面已有影子评论 → 照常补贴标签与取消认领", async () => {
  const h = liveDeps([fence({ class: "P", reason: "资金参数" }), goodDraft, fence(pass())]);
  h.fail((a) => a[3] === "--add-label");
  await processTicket(1565, h.d);
  assert.ok(h.comments.some((c) => c.body.startsWith("**P 类影子模式，待用户批复**")));
  h.stopFail();
  h.ghCalls.length = 0;
  const s2 = await processTicket(1565, h.d);
  assert.equal(s2.phase, "shadow_p");
  assert.deepEqual(writesOf(h.ghCalls), ["issue edit 1565 --add-assignee", "issue edit 1565 --add-label", "issue edit 1565 --remove-assignee"]);
});

// ───────────────────────── 考古阶段 ─────────────────────────

test("考古解析与机械校验：合格产物无问题；两路缺一、未覆盖已关票、A/B 缺出处、B 缺默认值、C 关键词不足都报", () => {
  const a = parseArchaeology(goodArch)!;
  assert.equal(a.points.length, 3);
  assert.deepEqual(a.points.map((p) => p.klass), ["A", "B", "C"]);
  assert.equal(a.points[1].recommended, "Q=100");
  assert.deepEqual(archaeologyProblems(a), []);
  assert.equal(parseArchaeology("没有 JSON"), null);
  const odd = parseArchaeology(fence({ points: [null, 3, { id: "Q1", class: "A", queries: [null], hits: [null, { source: "x" }] }] }))!;
  assert.equal(odd.points.length, 1);
  assert.deepEqual(odd.points[0].hits, [{ source: "x", excerpt: "" }]);

  const bad = structuredClone(goodArchObj) as typeof goodArchObj & { points: Array<Record<string, unknown>> };
  bad.points[0].queries = [repoQ];
  bad.points[1].queries = [{ ...trackerQ, command: "gh issue list --search cap" }, repoQ];
  bad.points[1].recommended = "";
  bad.points[2].searched_keywords = ["只搜一个"];
  bad.points.push({ id: "Q4", question: "x", class: "A", queries: [trackerQ, repoQ], hits: [], citation: "", conflict: "ADR 与代码不符" });
  bad.points.push({ id: "Q5", question: "y", class: "C", queries: [trackerQ, repoQ], hits: [], searched_keywords: ["a", "b"] });
  bad.points.push({ id: "Q6", question: "z", class: "C", queries: [{ ...trackerQ, command: "gh search issues z --state open" }, repoQ], hits: [], searched_keywords: ["a", "b"], gap: "decision" });
  const probs = archaeologyProblems(parseArchaeology(fence(bad))!);
  for (const re of [/Q1：缺 tracker/, /Q2：tracker 查询未覆盖已关闭票/, /Q2：B 类缺默认值/, /Q3：C 类须列出至少两个/, /Q4：A 类缺出处/, /Q4：A 类缺命中/, /Q4：有冲突的点不能归 A/, /Q5：C 类须写 gap/, /Q6：tracker 查询未覆盖已关闭票/, /Q6：C 类 decision 缺推荐答案/]) {
    assert.ok(probs.some((x) => re.test(x)), `${re} 未报：${probs.join(" / ")}`);
  }
});

test("archaeology.md：逐点写查询词、命令、命中与归类，B 带默认值、C 带已搜关键词", () => {
  const md = renderArchaeology({ number: 1355, title: "量档" }, parseArchaeology(goodArch)!, []);
  assert.match(md, /A 已裁 1 · B 有先例未正式裁 1 · C 确未设计 1/);
  assert.match(md, /## Q1 \[A\] 重的数量由谁定/);
  assert.match(md, /\| tracker \| 共同 cap \| `gh search issues/);
  assert.match(md, /- 默认值：Q=100/);
  assert.match(md, /- 已搜关键词：多空 容量、long short capacity/);
  assert.match(md, /## 机械校验\n\n通过。/);
});

test("分级考古联合提示：分级口径与考古两路搜、A/B/C 归类、联合 JSON 形状都在同一提示", () => {
  const t = { number: 1355, title: "量档", body: "", labels: [], assignees: [], comments: [], blockers: [] };
  const p = triagePrompt(t);
  for (const s of ["T 类（技术取舍）", "P 类（政策）", "资金与费用参数", "拿不准一律判 P", '"class":"T|P"', '"points"', "--state all", "in:comments", "gh search issues", "docs/adr/", ".chanlun/definitions/", "CONTEXT.md", "analysis/", "测试夹具", "A 已裁", "B 有先例未正式裁", "C 确未设计", "searched_keywords", ".agents/skills/grilling/SKILL.md", ".agents/skills/domain-modeling/SKILL.md", "research 票", "探针票", "adr/0012", "conflict"]) {
    assert.ok(p.includes(s), s);
  }
});

test("dry-run 全流程：分级后先考古，archaeology.md 落盘，起草与审查都拿到考古，审查含 R8", async () => {
  const { d, modelCalls, archCalls, ghCalls } = fakeDeps("dry-run", [fence({ class: "T", reason: "测试锁" }), goodDraft, fence(pass())]);
  const s = await processTicket(1531, d);
  assert.equal(s.phase, "land_t");
  assert.equal(archCalls.length, 1);
  assert.equal(archCalls[0].model, MODEL_MAIN);
  assert.ok(!archCalls[0].allowWrite);
  assert.deepEqual(s.archaeology, { A: 1, B: 1, C: 1, problems: [] });
  const dir = join(d.outRoot, "1531");
  assert.ok(existsSync(join(dir, ARCHAEOLOGY_FILE)));
  assert.ok(existsSync(join(dir, "archaeology.json")));
  const [draftCall, reviewCall] = [modelCalls.find((c) => c.role === "draft")!, modelCalls.find((c) => c.role === "review")!];
  assert.match(draftCall.prompt, /## Q3 \[C\] 多空共用容量/);
  assert.match(draftCall.prompt, /只有 C 类 decision 列为真正待决/);
  assert.match(draftCall.prompt, /不交用户填数/);
  assert.match(reviewCall.prompt, /R8 考古成立。.*至少三条命中.*复跑/);
  assert.match(reviewCall.prompt, /凭空提供数值/);
  assert.match(reviewCall.prompt, /archaeology\.md 开始/);
  assert.equal(ghCalls.filter(isWrite).length, 0);
});

test("考古不合格：带问题重跑一次；仍不合格则问题交审查，草稿即使审查全 PASS 也 BLOCK", async () => {
  const lazy = structuredClone(goodArchObj);
  (lazy.points[2] as Record<string, unknown>).searched_keywords = ["一个"];
  const { d, archCalls } = fakeDeps("dry-run", [fence({ class: "T", reason: "接口" }), goodDraft, fence(pass()), goodDraft, fence(pass())], [fence(lazy), fence(lazy), fence(lazy), fence(lazy)]);
  const s = await processTicket(1531, d);
  assert.match(archCalls[1].prompt, /上一次考古产物没过机械校验[\s\S]*Q3：C 类须列出至少两个/);
  const rv = JSON.parse(readFileSync(join(d.outRoot, "1531", "review.attempt1.json"), "utf8"));
  assert.equal(rv.verdict, "BLOCK");
  assert.ok(rv.problems.some((x: string) => /^考古：Q3/.test(x)));
  assert.equal(archCalls.length, 4, "升档重做时重跑考古");
  assert.match(archCalls[2].prompt, /上一次考古产物没过机械校验[\s\S]*Q3：C 类须列出至少两个/, "重跑首次即带上轮问题");
  assert.equal(archCalls[2].model, MODEL_MAIN, "工具步骤始终走主控模型");
  assert.notEqual(s.phase, "land_t");
});

test("考古输出两次都解析不出：票记 error，不起草", async () => {
  const { d, modelCalls } = fakeDeps("dry-run", [fence({ class: "T", reason: "接口" })], ["无", "仍无"]);
  const s = await processTicket(1531, d);
  assert.equal(s.phase, "error");
  assert.match(s.error ?? "", /考古输出两次均无法解析/);
  assert.equal(modelCalls.filter((c) => c.role === "draft").length, 0);
});

test("考古对照：草稿缺段或漏点即 BLOCK", async () => {
  const a = parseArchaeology(goodArch)!;
  assert.deepEqual(archaeologyCoverageProblems(goodDraft, a), []);
  assert.match(archaeologyCoverageProblems(goodDraft.replace(archTable, ""), a)[0], /缺「考古对照」段/);
  assert.match(archaeologyCoverageProblems(goodDraft.replace("| Q3 | C | — | 待决 |", ""), a)[0], /漏了考古点 Q3/);
  const noArch = goodDraft.replace(archTable, "");
  const { d } = fakeDeps("dry-run", [fence({ class: "T", reason: "接口" }), noArch, fence(pass()), noArch, fence(pass())]);
  const s = await processTicket(1531, d);
  assert.equal(s.phase, "shadow_p");
  assert.ok(s.rounds!.every((r) => r.problems.some((x) => /考古对照/.test(x))));
});

test("影子评论：只问 C 类与 B 类确认，每问附出处与推荐值；A 类列为已沿用", () => {
  const a = parseArchaeology(goodArch)!;
  const asks = archaeologyAsks(a);
  assert.match(asks, /A 已裁 1 点直接沿用（Q1（docs\/adr\/0016-x\.md:96））/);
  assert.match(asks, /\| Q2 \| B 确认或否决 \| 固定 Q 初值 \| #1355 评论 1 \| Q=100 \|/);
  assert.match(asks, /\| Q3 \| C 待决 \| 多空共用容量 \| 已搜：多空 容量、long short capacity \| 各占一份（依 ADR 0016:70 容量按重测） \|/);
  assert.doesNotMatch(asks, /\| Q1 \|/);
  const withGap = parseArchaeology(fence({ points: [...goodArchObj.points, { id: "Q4", question: "交易所费率", class: "C", queries: [trackerQ, repoQ], hits: [], searched_keywords: ["fee", "费率"], gap: "research" }] }))!;
  const g = archaeologyAsks(withGap);
  assert.doesNotMatch(g, /\| Q4 \|/);
  assert.match(g, /事实缺口（不问你，开票补）：\n- Q4 交易所费率 → 开 research 票/);
  const review = parseReview(fence(pass()));
  const body = shadowComment(1355, goodDraft, review, "/x/1355", { ticket: 1355, mode: "dry-run", phase: "shadow_p", attempts: 1, durationsMs: {}, plannedWrites: [] }, a);
  assert.ok(body.indexOf("需要你回答的只有下表") < body.indexOf("独立审查"));
});

test("--archaeology-only：已关闭的决策票也照跑考古，只落 archaeology 两份文件，不分级不起草不写 GitHub", async () => {
  assert.throws(() => parseArgs(["--archaeology-only"]), /配 --ticket/);
  assert.throws(() => parseArgs(["--archaeology-only", "--live", "--ticket", "1"]), /只能 dry-run/);
  assert.equal(parseArgs(["--archaeology-only", "--ticket", "1355"]).archaeologyOnly, true);
  const { d, modelCalls, archCalls, ghCalls } = fakeDeps("dry-run", []);
  const base = d.gh;
  d.gh = (args, input) => (args[0] === "issue" && args[1] === "view"
    ? JSON.stringify({ number: 1355, state: "CLOSED", title: "量档", body: "票面", labels: [], assignees: [], comments: [{ author: { login: "xy7365527-lang" }, body: "用户 2026-10-13 批复：同意" }] })
    : base(args, input));
  const r = await archaeologyOnly(1355, d);
  assert.deepEqual(r.counts, { A: 1, B: 1, C: 1 });
  assert.deepEqual(r.problems, []);
  assert.equal(archCalls.length, 1);
  assert.equal(modelCalls.length, 0);
  assert.equal(ghCalls.filter(isWrite).length, 0);
  assert.ok(existsSync(join(d.outRoot, "1355", ARCHAEOLOGY_FILE)));
  assert.equal(existsSync(join(d.outRoot, "1355", "summary.json")), false);
});

test("考古有 research/probe 事实缺口：T 类不自动落地，转 P 起草，影子评论列缺口", async () => {
  const gapped = structuredClone(goodArchObj) as { points: Array<Record<string, unknown>> };
  gapped.points.push({ id: "Q4", question: "交易所费率", class: "C", queries: [trackerQ, repoQ], hits: [], searched_keywords: ["fee", "费率"], gap: "research" });
  const draft = goodDraft + "| Q4 | C research | — | 开 research 票 |\n";
  const { d, modelCalls } = fakeDeps("dry-run", [fence({ class: "T", reason: "接口" }), draft, fence(pass())], [fence(gapped)]);
  const s = await processTicket(1531, d);
  assert.equal(s.phase, "shadow_p");
  assert.equal(s.klass, "P");
  assert.equal(s.convertedToP, true);
  assert.match(s.reclassNotes![0], /事实缺口 Q4/);
  assert.match(modelCalls.find((c) => c.role === "draft")!.prompt, /本票分级：P 类/);
  assert.ok(s.plannedWrites.includes("标签 #1531：+needs-approval"));
  assert.ok(s.plannedWrites.some((w) => /^评论 #1531：考古有事实缺口 Q4/.test(w)));
  const body = shadowComment(1531, draft, parseReview(fence(pass())), "/x/1531", s, parseArchaeology(fence(gapped)));
  assert.match(body.split("\n")[0], /原判 T，考古有事实缺口 Q4/);
  assert.doesNotMatch(body.split("\n")[0], /审查仍 BLOCK/);
});

test("升档重跑考古后新出现事实缺口：T 类同样转 P，不落地", async () => {
  const noGap = structuredClone(goodArchObj) as { points: Array<Record<string, unknown>> };
  noGap.points[2].gap = "";
  const probe = structuredClone(goodArchObj) as { points: Array<Record<string, unknown>> };
  probe.points[2].gap = "probe";
  const { d } = fakeDeps("dry-run", [fence({ class: "T", reason: "接口" }), goodDraft, fence(pass()), goodDraft, fence(pass())], [fence(noGap), fence(noGap), fence(probe)]);
  const s = await processTicket(1531, d);
  assert.notEqual(s.phase, "land_t");
  assert.equal(s.klass, "P");
  assert.match(s.factGapNote ?? "", /Q3/);
});

test("flowSectionsIn：正本里混入引用自证或考古对照即报", () => {
  assert.deepEqual(flowSectionsIn("## 裁定\n内容"), []);
  assert.deepEqual(flowSectionsIn(goodDraft), ["引用自证", "考古对照"]);
  const stripped = stripFlowSections(goodDraft);
  assert.deepEqual(flowSectionsIn(stripped), []);
  assert.deepEqual(missingDraftSections(stripped), []);
});

test("影子评论：本地无考古时首行提示先跑 --archaeology-only", () => {
  const body = shadowComment(1355, goodDraft, parseReview(fence(pass())), "/x/decisions/1355", { ticket: 1355, mode: "dry-run", phase: "shadow_p", attempts: 1, durationsMs: {}, plannedWrites: [] }, null);
  assert.match(body, /本票无考古.*--archaeology-only --ticket 1355 --out \/x\/decisions/);
});

test("--archaeology-only 与 processTicket 共用目录锁", async () => {
  const { d } = fakeDeps("dry-run", []);
  mkdirSync(join(d.outRoot, "1531"), { recursive: true });
  writeFileSync(join(d.outRoot, "1531", ".lock"), "1");
  await assert.rejects(archaeologyOnly(1531, d), /另一进程正在处理/);
});

const drainHelper = join(process.cwd(), ".sandcastle", "drain-control.py");
const drainCli = (d: Deps, command: "freeze" | "resume", ...args: string[]) =>
  execFileSync("python3", [drainHelper, command, "--root", d.drainRoot!, ...args]);

test("--archaeology-only 入口受 drain 准入保护", async () => {
  const { d } = fakeDeps("dry-run", [], [goodArch]);
  drainCli(d, "freeze");
  await assert.rejects(archaeologyOnlyAdmission(1355, d), /冻结|未新增动作/);
  assert.equal(existsSync(join(d.outRoot, "1355")), false);
});

test("冻结保留 T 类已推送断点及认领，解冻只补 resolution 不重复提交", async () => {
  const h = liveDeps([fence({ class: "T", reason: "接口" }), goodDraft, fence(pass()), "已写入"]);
  h.fail((a) => (a[1] === "comment" && h.comments.length > 0) || a[3] === "--remove-assignee");
  const first = await processTicket(1565, h.d);
  assert.equal(first.stage, "reviewed_pass");
  assert.equal(first.landedSha, "abcdef1234567890");
  h.stopFail();
  const dir = join(h.d.outRoot, "1565");
  const before = readFileSync(join(dir, "summary.json"), "utf8");
  h.ghCalls.length = h.modelCalls.length = h.gitCalls.length = 0;
  drainCli(h.d, "freeze");
  const frozen = await processTicket(1565, h.d);
  assert.equal(frozen.phase, "skipped");
  assert.equal(readFileSync(join(dir, "summary.json"), "utf8"), before);
  assert.equal(loadResume(dir)?.prev.stage, "reviewed_pass");
  assert.deepEqual(writesOf(h.ghCalls), []);
  assert.deepEqual(h.modelCalls, []);
  drainCli(h.d, "resume", "--generation", "1");
  const resumed = await processTicket(1565, h.d);
  assert.equal(resumed.resumedFrom, "reviewed_pass");
  assert.equal(resumed.phase, "land_t");
  assert.deepEqual(writesOf(h.ghCalls), ["issue comment 1565 --body-file"]);
  assert.deepEqual(h.modelCalls, []);
  assert.deepEqual(h.gitCalls, []);
});

test("冻结保留 P 类 pDone，解冻只补标签和取消认领而不重复影子评论", async () => {
  const h = liveDeps([fence({ class: "P", reason: "资金参数" }), goodDraft, fence(pass())]);
  h.fail((a) => a[3] === "--add-label");
  const first = await processTicket(1565, h.d);
  assert.equal(first.stage, "shadow_p_pending");
  assert.deepEqual(first.pDone, ["comment"]);
  h.stopFail();
  const dir = join(h.d.outRoot, "1565");
  const before = readFileSync(join(dir, "summary.json"), "utf8");
  h.ghCalls.length = h.modelCalls.length = h.gitCalls.length = 0;
  drainCli(h.d, "freeze");
  const frozen = await processTicket(1565, h.d);
  assert.equal(frozen.phase, "skipped");
  assert.equal(readFileSync(join(dir, "summary.json"), "utf8"), before);
  assert.deepEqual(loadResume(dir)?.prev.pDone, ["comment"]);
  assert.deepEqual(writesOf(h.ghCalls), []);
  drainCli(h.d, "resume", "--generation", "1");
  const resumed = await processTicket(1565, h.d);
  assert.equal(resumed.phase, "shadow_p");
  assert.equal(resumed.resumedFrom, "shadow_p_pending");
  assert.deepEqual(writesOf(h.ghCalls), ["issue edit 1565 --add-assignee", "issue edit 1565 --add-label", "issue edit 1565 --remove-assignee"]);
  assert.equal(h.comments.filter((c) => c.body.includes("P 类影子模式")).length, 1);
  assert.deepEqual(h.modelCalls, []);
  assert.deepEqual(h.gitCalls, []);
});

test("锁：旧入口主循环每轮开始检查 drainRequested", () => {
  const source = readFileSync(join(process.cwd(), ".sandcastle", "decision-claimer.mts"), "utf8");
  assert.match(source, /for \(;;\) \{\s*if \(drainRequested\(d\.drainRoot\)\)/);
});

// ───────────── #1785：安全拦截改派与异家族独审 ─────────────

/** 真实 runner 的安全拦截错误形状；安全拦截不是临时错误，不能退避重发同一模型。 */
const SAFEGUARD_ERROR = (role: string, model = MODEL_DRAFTER) =>
  `${role} ${model} 调用失败（exit 1，第 1 次）：API Error: ${model} safeguards flagged this message (https://www.anthropic.com/legal/aup). This sometimes happens with safe, normal conversations.`;

/** 按 (role, model) 脚本化的假模型：命中 block 集合的指定模型调用抛安全拦截错误。 */
function blockingDeps(block: Set<string>, opts: { reviewText?: string; hardError?: string; blockedModel?: string } = {}) {
  const { d } = fakeDeps("dry-run", []);
  const calls: ModelCall[] = [];
  const blockedModel = opts.blockedModel ?? MODEL_DRAFTER;
  d.model = async (c) => {
    calls.push(c);
    if (opts.hardError && c.role === "triage") throw new Error(opts.hardError);
    if (block.has(c.role) && c.model === blockedModel) throw new Error(SAFEGUARD_ERROR(c.role, c.model));
    const text = c.role === "triage" ? (c.prompt.includes(FIXED) ? goodArch : fence({ class: "T", reason: "测试锁", ...goodArchObj }))
      : c.role === "draft" ? goodDraft
      : opts.reviewText ?? fence(pass());
    return { text, durationMs: 5, attempts: 1, model: c.model };
  };
  return { d, calls };
}
const seq = (calls: ModelCall[]) => calls.map((c) => [c.role, c.model]);

test("isSafeguardBlock：只认安全拦截文本；临时错误、参数错误、泛泛的 safeguard 字样都不算", () => {
  assert.ok(isSafeguardBlock(SAFEGUARD_ERROR("draft")));
  assert.ok(isSafeguardBlock("API Error: claude-opus-5-5's safeguards flagged this message"));
  for (const x of ["API Error: 429 Too Many Requests", "Invalid model name", "safeguards", "Opus safeguards flagged", "模型服务暂时不可用"]) {
    assert.equal(isSafeguardBlock(x), false, x);
  }
  assert.equal(isRetryable(SAFEGUARD_ERROR("draft")), false, "拦截不是临时错误，runner 不退避重发");
  assert.equal(safeguardFallbackModel(MODEL_DRAFTER), MODEL_REVIEW);
  assert.equal(safeguardFallbackModel(MODEL_REVIEW), MODEL_DRAFTER);
});

test("工具步骤被安全拦截：主控模型改派 Opus，summary 与 archaeology.json 记录切换", async () => {
  const { d, calls } = blockingDeps(new Set(["triage"]), { blockedModel: MODEL_MAIN });
  const s = await processTicket(1783, d);
  assert.equal(s.phase, "land_t", s.error ?? "");
  assert.deepEqual(seq(calls), [
    ["triage", MODEL_MAIN], ["triage", MODEL_DRAFTER], ["draft", MODEL_DRAFTER], ["review", MODEL_REVIEW],
  ]);
  const want = [{ role: "triage", from: MODEL_MAIN, to: MODEL_DRAFTER, reason: "safeguard" }];
  assert.deepEqual(s.modelSwitches, want);
  const aj = JSON.parse(readFileSync(join(d.outRoot, "1783", "archaeology.json"), "utf8"));
  assert.deepEqual(aj.modelSwitches, want);
  assert.equal(aj.model, MODEL_DRAFTER);
});

test("主控工具步骤被拦截后粘住改派模型，不再先撞一次拦截", async () => {
  const { d, calls } = blockingDeps(new Set(["triage"]), { blockedModel: MODEL_MAIN });
  const s = await processTicket(1783, d);
  assert.equal(s.phase, "land_t", s.error ?? "");
  assert.deepEqual(seq(calls).slice(0, 2), [["triage", MODEL_MAIN], ["triage", MODEL_DRAFTER]]);
  const inner: ModelCall[] = [];
  const run = withSafeguardFallback(async (c) => {
    inner.push(c);
    if (c.model === MODEL_MAIN) throw new Error(SAFEGUARD_ERROR(c.role, c.model));
    return { text: "ok", durationMs: 1, attempts: 1, model: c.model };
  });
  const a = await run({ model: MODEL_MAIN, prompt: "p", cwd: "/", role: "triage" });
  const b = await run({ model: MODEL_MAIN, prompt: "p2", cwd: "/", role: "triage" });
  assert.equal(a.model, MODEL_DRAFTER);
  assert.equal(b.model, MODEL_DRAFTER);
  assert.deepEqual(inner.map((c) => c.model), [MODEL_MAIN, MODEL_DRAFTER, MODEL_DRAFTER], "第二次直接走 Opus");
});

test("起草被拦截改主控：独审改用 Opus，review.json 与 rounds 记实际起草/独审模型", async () => {
  const { d, calls } = blockingDeps(new Set(["draft"]));
  const s = await processTicket(1783, d);
  assert.equal(s.phase, "land_t", s.error ?? "");
  assert.deepEqual(seq(calls).slice(1), [["draft", MODEL_DRAFTER], ["draft", MODEL_REVIEW], ["review", MODEL_DRAFTER]]);
  const rv = JSON.parse(readFileSync(join(d.outRoot, "1783", "review.json"), "utf8"));
  assert.equal(rv.drafter, MODEL_REVIEW);
  assert.equal(rv.reviewer, MODEL_DRAFTER);
  assert.deepEqual(s.rounds!.map((r) => [r.drafter, r.reviewer]), [[MODEL_REVIEW, MODEL_DRAFTER]]);
});

test("起草改主控后独审 Opus 也被拦截：无异家族可选时 FAIL-loud，不静默放行", async () => {
  const { d, calls } = blockingDeps(new Set(["draft", "review"]));
  const s = await processTicket(1783, d);
  assert.equal(s.phase, "error");
  assert.match(s.error ?? "", /同家族|FAIL/);
  assert.deepEqual(seq(calls).slice(1), [["draft", MODEL_DRAFTER], ["draft", MODEL_REVIEW], ["review", MODEL_DRAFTER], ["review", MODEL_REVIEW]]);
  assert.deepEqual(s.modelSwitches!.map((x) => [x.role, x.from, x.to]), [["draft", MODEL_DRAFTER, MODEL_REVIEW], ["review", MODEL_DRAFTER, MODEL_REVIEW]]);
  const rv = JSON.parse(readFileSync(join(d.outRoot, "1783", "review.json"), "utf8"));
  assert.deepEqual(rv.modelSwitches, [{ role: "review", from: MODEL_DRAFTER, to: MODEL_REVIEW, reason: "safeguard" }]);
});

test("锁：起草与独审必须异家族，计划与改派两种轮次都覆盖", async () => {
  assert.equal(modelFamily(MODEL_DRAFTER), "claude");
  assert.equal(modelFamily(MODEL_REVIEW), "gpt");
  assert.notEqual(reviewerModel(MODEL_DRAFTER, MODEL_DRAFTER), MODEL_DRAFTER);
  assert.equal(reviewerModel(MODEL_DRAFTER, MODEL_DRAFTER), MODEL_REVIEW);
  assert.equal(reviewerModel(MODEL_DRAFTER, MODEL_REVIEW), MODEL_DRAFTER);
  assert.doesNotThrow(() => assertDifferentModelFamilies(MODEL_DRAFTER, MODEL_REVIEW));
  assert.throws(() => assertDifferentModelFamilies(MODEL_REVIEW, MODEL_REVIEW), /同家族/);
  for (const block of [new Set<string>(), new Set(["draft"])]) {
    const { d } = blockingDeps(block);
    const s = await processTicket(1783, d);
    for (const r of s.rounds!) assert.notEqual(modelFamily(r.drafter), modelFamily(r.reviewer ?? MODEL_REVIEW), `block=${[...block]}`);
  }
});

test("负控：拦截以外的非临时错误不改派，照原逻辑整票失败；落地模型也不改派", async () => {
  const hard = blockingDeps(new Set(), { hardError: `triage ${MODEL_MAIN} 调用失败（exit 1，第 1 次）：Invalid model name` });
  const s = await processTicket(1783, hard.d);
  assert.equal(s.phase, "error");
  assert.match(s.error ?? "", /Invalid model name/);
  assert.deepEqual(seq(hard.calls), [["triage", MODEL_MAIN]]);
  assert.equal(s.modelSwitches, undefined);
  const run = withSafeguardFallback(async () => { throw new Error(SAFEGUARD_ERROR("land", MODEL_OPUS)); });
  await assert.rejects(run({ model: MODEL_OPUS, prompt: "p", cwd: "/", role: "land" }), /safeguards flagged/);
});

test("F-1：分级考古联合调用改派后两份记录都写实际模型与 modelSwitches，未改派默认主控模型", async () => {
  const { d } = blockingDeps(new Set(["triage"]), { blockedModel: MODEL_MAIN });
  const s = await processTicket(1783, d);
  assert.equal(s.phase, "land_t", s.error ?? "");
  const dir = join(d.outRoot, "1783");
  const cj = JSON.parse(readFileSync(join(dir, "classify.json"), "utf8"));
  assert.equal(cj.model, MODEL_DRAFTER);
  assert.deepEqual(cj.modelSwitches, [{ role: "triage", from: MODEL_MAIN, to: MODEL_DRAFTER, reason: "safeguard" }]);
  const aj = JSON.parse(readFileSync(join(dir, "archaeology.json"), "utf8"));
  assert.equal(aj.model, MODEL_DRAFTER);
  assert.deepEqual(aj.modelSwitches, [{ role: "triage", from: MODEL_MAIN, to: MODEL_DRAFTER, reason: "safeguard" }]);
  const plain = blockingDeps(new Set());
  await processTicket(1783, plain.d);
  const pj = JSON.parse(readFileSync(join(plain.d.outRoot, "1783", "archaeology.json"), "utf8"));
  assert.equal(pj.model, MODEL_MAIN);
  assert.equal(pj.modelSwitches, undefined);
  assert.equal(JSON.parse(readFileSync(join(plain.d.outRoot, "1783", "classify.json"), "utf8")).model, MODEL_MAIN);
});

test("F-1：--archaeology-only 无 summary 时，考古改派照样写进 archaeology.json", async () => {
  const { d, calls } = blockingDeps(new Set(["triage"]), { blockedModel: MODEL_MAIN });
  const r = await archaeologyOnly(1355, d);
  assert.deepEqual(r.problems, []);
  assert.deepEqual(seq(calls), [["triage", MODEL_MAIN], ["triage", MODEL_DRAFTER]]);
  const dir = join(d.outRoot, "1355");
  const aj = JSON.parse(readFileSync(join(dir, "archaeology.json"), "utf8"));
  assert.equal(aj.model, MODEL_DRAFTER);
  assert.deepEqual(aj.modelSwitches, [{ role: "triage", from: MODEL_MAIN, to: MODEL_DRAFTER, reason: "safeguard" }]);
  assert.equal(existsSync(join(dir, "summary.json")), false);
});

test("F-2：断点续做带回 modelSwitches，续做后重写的 summary.json 不丢改派记录", async () => {
  const { d, replies } = liveDeps([
    new Error(SAFEGUARD_ERROR("triage", MODEL_MAIN)), fence({ class: "T", reason: "接口" }), goodDraft, fence(pass()),
    new Error("land claude-opus-5-5 调用失败（exit 1，第 6 次）：API Error: 模型服务暂时不可用"),
  ]);
  const want = [{ role: "triage", from: MODEL_MAIN, to: MODEL_DRAFTER, reason: "safeguard" }];
  const s1 = await processTicket(1565, d);
  assert.equal(s1.stage, "reviewed_pass", s1.error ?? "");
  assert.deepEqual(s1.modelSwitches, want);
  replies.push("已写入");
  const s2 = await processTicket(1565, d);
  assert.equal(s2.phase, "land_t", s2.error ?? "");
  assert.equal(s2.resumedFrom, "reviewed_pass");
  assert.deepEqual(s2.modelSwitches, want);
  assert.deepEqual(JSON.parse(readFileSync(join(d.outRoot, "1565", "summary.json"), "utf8")).modelSwitches, want);
});

// ───────────── #1742 M1：分级+考古一次联合调用 ─────────────

const triageCalls = (calls: ModelCall[]) => calls.filter((c) => c.role === "triage");
const classifyComments = (comments: Array<{ body: string }>) => comments.filter((c) => c.body.startsWith("分级："));

test("M1 新票：正常路径只有一次分级+考古模型调用，classify.json 与 archaeology.json 同源落盘", async () => {
  const { d } = fakeDeps("dry-run", []);
  const calls: ModelCall[] = [];
  d.model = async (c) => {
    calls.push(c);
    const text = c.role === "triage" ? fence({ class: "T", reason: "接口", ...goodArchObj }) : c.role === "draft" ? goodDraft : fence(pass());
    return { text, durationMs: 5, attempts: 1 };
  };
  const s = await processTicket(1531, d);
  assert.equal(s.phase, "land_t", s.error ?? "");
  assert.deepEqual(calls.map((c) => c.role), ["triage", "draft", "review"]);
  assert.ok(!calls[0].prompt.includes(FIXED_CLASS_LINE));
  const dir = join(d.outRoot, "1531");
  assert.equal(JSON.parse(readFileSync(join(dir, "classify.json"), "utf8")).klass, "T");
  assert.equal(JSON.parse(readFileSync(join(dir, "archaeology.json"), "utf8")).parsed, true);
  assert.equal(s.plannedWrites.filter((w) => w.startsWith("评论 #1531：分级：")).length, 1);
});

test("M1 联合输出缺 class 或 reason：分级按「拿不准一律判 P」，考古照收，不自动落地", async () => {
  const { d } = fakeDeps("dry-run", []);
  d.model = async (c) => ({ text: c.role === "triage" ? fence({ ...goodArchObj }) : c.role === "draft" ? goodDraft : fence(pass()), durationMs: 5, attempts: 1 });
  const s = await processTicket(1531, d);
  assert.equal(s.klass, "P");
  assert.equal(s.phase, "shadow_p");
  assert.deepEqual(s.archaeology, { A: 1, B: 1, C: 1, problems: [] });
  assert.ok(s.plannedWrites.includes("标签 #1531：+needs-approval"));
});

test("M1 P 类账户未知：联合调用保留 P 与 C 类 decision/research 缺口，影子评论列待决与要开的票，不落地", async () => {
  const pArch = { points: [
    { id: "Q1", question: "Q 字段映射", class: "C", queries: [trackerQ, repoQ], hits: [], searched_keywords: ["Q_total", "账户权益"], gap: "decision", recommended: "AwaitingPolicy，签署前不代填（ADR 0049 金额与数量分维）" },
    { id: "Q2", question: "目标账户初值", class: "C", queries: [trackerQ, repoQ], hits: [], searched_keywords: ["账户初值", "initial equity"], gap: "research" },
  ] };
  const { d } = fakeDeps("dry-run", []);
  const calls: ModelCall[] = [];
  d.model = async (c) => {
    calls.push(c);
    const draft = goodDraft.replace(archTable, `## ${ARCH_SECTION}\n| 点 | 归类 | 出处 | 草稿处置 |\n|---|---|---|---|\n| Q1 | C | — | 待决 |\n| Q2 | C research | — | 开 research 票 |`);
    return { text: c.role === "triage" ? fence({ class: "P", reason: "资金参数", ...pArch }) : c.role === "draft" ? draft : fence(pass()), durationMs: 5, attempts: 1 };
  };
  const s = await processTicket(1531, d);
  assert.equal(s.klass, "P");
  assert.equal(s.phase, "shadow_p");
  assert.equal(triageCalls(calls).length, 1);
  assert.match(calls.find((c) => c.role === "draft")!.prompt, /本票分级：P 类/);
  assert.ok(!s.plannedWrites.some((w) => w.startsWith("分支 decision/")));
  const body = shadowComment(1531, "", parseReview(fence(pass())), "/x/1531", s, parseArchaeology(fence(pArch)));
  assert.match(body, /\| Q1 \| C 待决 \| Q 字段映射 \|.*AwaitingPolicy/);
  assert.match(body, /- Q2 目标账户初值 → 开 research 票/);
});

test("M1 失败重入：联合调用考古两次解析不出，分级已落盘且评论一次；下一轮只补考古，不重分级、不重发分级评论", async () => {
  const h = liveDeps([goodDraft, fence(pass()), "已写入"]);
  const base = h.d.model;
  let bad = 1;
  h.d.model = async (c) => {
    if (c.role === "triage" && !c.prompt.includes(FIXED_CLASS_LINE)) {
      h.modelCalls.push(c);
      return { text: fence({ class: "T", reason: "接口" }), durationMs: 5, attempts: 1 };
    }
    if (c.role === "triage" && bad > 0) { bad--; h.fixedCalls.push(c); return { text: "无", durationMs: 5, attempts: 1 }; }
    return base(c);
  };
  const s1 = await processTicket(1565, h.d);
  assert.equal(s1.phase, "error");
  assert.match(s1.error ?? "", /考古输出两次均无法解析/);
  assert.equal(triageCalls(h.modelCalls).length, 1);
  assert.equal(classifyComments(h.comments).length, 1);
  assert.equal(writesOf(h.ghCalls).at(-1), "issue edit 1565 --remove-assignee");
  h.modelCalls.length = 0;
  h.fixedCalls.length = 0;
  h.ghCalls.length = 0;
  const s2 = await processTicket(1565, h.d);
  assert.equal(s2.phase, "land_t", s2.error ?? "");
  assert.equal(s2.resumedFrom, "triaged");
  assert.equal(s2.klass, "T");
  assert.equal(h.fixedCalls.length, 1);
  assert.match(h.fixedCalls[0].prompt, /分级已定为 T 类 —— 接口/);
  assert.deepEqual(h.modelCalls.map((c) => c.role), ["draft", "review", "land"]);
  assert.equal(classifyComments(h.comments).length, 1, "分级评论不重发");
  assert.deepEqual(writesOf(h.ghCalls), ["issue edit 1565 --add-assignee", "issue comment 1565 --body-file"]);
});

test("M1 旧两段式断点：只有 classify.json（旧角色名）且票上已有分级评论 → 沿用分级只补考古，认领只一次", async () => {
  const h = liveDeps([goodDraft, fence(pass()), "已写入"]);
  const dir = join(h.d.outRoot, "1565");
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, "classify.json"), JSON.stringify({ ticket: 1565, model: MODEL_MAIN, klass: "T", reason: "接口", parsed: true, durationMs: 9, raw: "x" }));
  writeFileSync(join(dir, "summary.json"), JSON.stringify({ ticket: 1565, mode: "live", phase: "error", attempts: 0, durationsMs: { classify: 9 }, plannedWrites: [], error: "archaeology gpt-6.1-sol 调用失败", modelSwitches: [{ role: "classify", from: MODEL_MAIN, to: MODEL_DRAFTER, reason: "safeguard" }] }));
  h.comments.push({ author: { login: "xy7365527-lang" }, body: "分级：T — 接口" });
  const s = await processTicket(1565, h.d);
  assert.equal(s.phase, "land_t", s.error ?? "");
  assert.equal(s.resumedFrom, "triaged");
  assert.equal(h.fixedCalls.length, 1);
  assert.deepEqual(h.modelCalls.map((c) => c.role), ["draft", "review", "land"]);
  assert.equal(classifyComments(h.comments).length, 1);
  assert.equal(writesOf(h.ghCalls).filter((w) => w.includes("--add-assignee")).length, 1);
  assert.equal(JSON.parse(readFileSync(join(dir, "classify.json"), "utf8")).raw, "x", "旧分级原件不被覆盖");
});

test("M1 旧断点：已认领在自己名下也照常续，P 类旧分级保持 P；考古已完成或水印后退回则整链联合重跑", async () => {
  const h = liveDeps([goodDraft, fence(pass())]);
  const dir = join(h.d.outRoot, "1565");
  mkdirSync(dir, { recursive: true });
  writeFileSync(join(dir, "classify.json"), JSON.stringify({ klass: "P", reason: "资金参数", parsed: true }));
  h.assign(["me"]);
  const s = await processTicket(1565, h.d);
  assert.equal(s.klass, "P");
  assert.equal(s.phase, "shadow_p");
  assert.match(h.modelCalls[0].prompt, /本票分级：P 类/);
  assert.equal(writesOf(h.ghCalls).filter((w) => w.includes("--add-assignee")).length, 0);
  assert.equal(classifyComments(h.comments).length, 1, "票上没有分级评论时补发一次");

  const done = mkdtempSync(join(tmpdir(), "m1-done-"));
  writeFileSync(join(done, "classify.json"), JSON.stringify({ klass: "T", reason: "r", parsed: true }));
  writeFileSync(join(done, "archaeology.json"), JSON.stringify({ parsed: true, points: [] }));
  assert.equal(loadTriaged(done), null);
  writeFileSync(join(done, "archaeology.json"), JSON.stringify({ parsed: false }));
  assert.deepEqual(loadTriaged(done), { klass: "T", reason: "r", parsed: true });
  writeFileSync(join(done, "classify.json"), JSON.stringify({ klass: "X", reason: "r" }));
  assert.equal(loadTriaged(done), null);

  const r = liveDeps([fence({ class: "T", reason: "重分" }), goodDraft, fence(pass()), "已写入"]);
  const rdir = join(r.d.outRoot, "1565");
  mkdirSync(rdir, { recursive: true });
  writeFileSync(join(rdir, "classify.json"), JSON.stringify({ klass: "T", reason: "旧", parsed: true }));
  r.comments.push({ author: { login: "xy7365527-lang" }, body: "## 退回重写" });
  const s3 = await processTicket(1565, r.d);
  assert.equal(s3.resumedFrom, undefined);
  assert.deepEqual(r.modelCalls.map((c) => c.role), ["triage", "draft", "review", "land"]);
});

test("M1 联合调用不改变独审异家族与有效 BLOCK：起草 Opus、独审主控，BLOCK 不变 PASS", async () => {
  const block = fence({ ...pass(), R1: { verdict: "BLOCK", basis: "rg 或运算写成 \\| 无输出" }, R3: { verdict: "BLOCK", basis: "验收 7 退出 128" }, R4: { verdict: "BLOCK", basis: "越界改外部票" }, R6: { verdict: "BLOCK", basis: "依赖未提供" } });
  const { d, modelCalls } = fakeDeps("dry-run", [fence({ class: "T", reason: "接口" }), goodDraft, block, goodDraft, block]);
  const s = await processTicket(1531, d);
  assert.notEqual(s.phase, "land_t");
  assert.equal(s.reviewCounts!.pass, 0);
  for (const r of s.rounds!) assert.notEqual(modelFamily(r.drafter), modelFamily(r.reviewer!));
  assert.equal(triageCalls(modelCalls).length, 1);
});

test("M1 转 P 不回退：先 T、起草建议转 P、升档重做考古两次解析不出 → 下一轮沿用 P，不得 land_t；classify.json 原件不改", async () => {
  const badArch = fence({ points: [{ ...goodArchObj.points[0], queries: [] }] });
  const { d } = fakeDeps("dry-run", []);
  const block = fence({ ...pass(), R3: { verdict: "BLOCK", basis: "验收项不可执行" } });
  let run = 1;
  let fixedN = 0;
  const calls: ModelCall[] = [];
  d.model = async (c) => {
    calls.push(c);
    if (c.role === "triage") {
      if (run === 1) {
        if (!c.prompt.includes(FIXED)) return { text: fence({ class: "T", reason: "接口", points: [{ ...goodArchObj.points[0], queries: [] }] }), durationMs: 5, attempts: 1 };
        fixedN++;
        return { text: fixedN === 1 ? badArch : "无 JSON", durationMs: 5, attempts: 1 };
      }
      return { text: goodArch, durationMs: 5, attempts: 1 };
    }
    if (c.role === "draft") return { text: goodDraft + (run === 1 ? "\n改判建议：P — 涉及真实账户\n" : ""), durationMs: 5, attempts: 1 };
    return { text: run === 1 ? block : fence(pass()), durationMs: 5, attempts: 1 };
  };
  const s1 = await processTicket(1531, d);
  assert.equal(s1.phase, "error");
  assert.match(s1.reclassNotes?.join(" ") ?? "", /建议转 P，已转/);
  const dir = join(d.outRoot, "1531");
  assert.equal(JSON.parse(readFileSync(join(dir, "classify.json"), "utf8")).klass, "T", "分级原件不改写");
  run = 2;
  calls.length = 0;
  const s2 = await processTicket(1531, d);
  assert.equal(s2.resumedFrom, "triaged");
  assert.equal(s2.klass, "P");
  assert.equal(s2.phase, "shadow_p");
  assert.match(calls[0].prompt, /分级已定为 P 类/);
  assert.ok(!s2.plannedWrites.some((w) => w.startsWith("分支 decision/")));
});

test("M1 转 P 不回退：考古事实缺口转 P 后整链重跑重判 T 仍沿用 P；水印后退回则作废转 P 记录", async () => {
  const dir0 = mkdtempSync(join(tmpdir(), "m1-effp-"));
  writeFileSync(join(dir0, "classify.json"), JSON.stringify({ klass: "T", reason: "接口", parsed: true }));
  writeFileSync(join(dir0, "classify.effective.json"), JSON.stringify({ klass: "P", reason: "接口（考古有事实缺口 Q3）", commentsSeen: 0 }));
  assert.deepEqual(loadEffectiveP(dir0), { klass: "P", reason: "接口（考古有事实缺口 Q3）", commentsSeen: 0 });

  const r = liveDeps([fence({ class: "T", reason: "重分" }), goodDraft, fence(pass())]);
  const rdir = join(r.d.outRoot, "1565");
  mkdirSync(rdir, { recursive: true });
  writeFileSync(join(rdir, "classify.json"), JSON.stringify({ klass: "T", reason: "接口", parsed: true }));
  writeFileSync(join(rdir, "archaeology.json"), JSON.stringify({ parsed: true, points: [] }));
  writeFileSync(join(rdir, "classify.effective.json"), JSON.stringify({ klass: "P", reason: "接口（转 P）", commentsSeen: 0 }));
  const s = await processTicket(1565, r.d);
  assert.equal(s.klass, "P");
  assert.equal(s.phase, "shadow_p");
  assert.equal(s.convertedToP, true);

  const q = liveDeps([fence({ class: "T", reason: "重分" }), goodDraft, fence(pass()), "已写入"]);
  const qdir = join(q.d.outRoot, "1565");
  mkdirSync(qdir, { recursive: true });
  writeFileSync(join(qdir, "classify.effective.json"), JSON.stringify({ klass: "P", reason: "旧转 P", commentsSeen: 0 }));
  q.comments.push({ author: { login: "xy7365527-lang" }, body: "## 退回重写" });
  const s3 = await processTicket(1565, q.d);
  assert.equal(s3.phase, "land_t", s3.error ?? "");
});


test("M1 损坏或不可读的有效 P 记录必须停止，不得重新判 T 自动落地", async () => {
  for (const value of ['{"klass":"P",', '{"klass":"T","reason":"错误"}', 'null']) {
    const { d, modelCalls } = fakeDeps("dry-run", [fence({ class: "T", reason: "接口" }), goodDraft, fence(pass())]);
    const dir = join(d.outRoot, "1531");
    mkdirSync(dir, { recursive: true });
    writeFileSync(join(dir, "classify.effective.json"), value);
    const s = await processTicket(1531, d);
    assert.equal(s.phase, "error", value);
    assert.match(s.error ?? "", /有效 P/);
    assert.equal(modelCalls.length, 0);
    assert.ok(!s.plannedWrites.some(w => w.startsWith("分支 decision/")));
  }
  const { d, modelCalls } = fakeDeps("dry-run", []);
  mkdirSync(join(d.outRoot, "1531", "classify.effective.json"), { recursive: true });
  const s = await processTicket(1531, d);
  assert.equal(s.phase, "error");
  assert.match(s.error ?? "", /有效 P/);
  assert.equal(modelCalls.length, 0);
});

test("M1 实际转 P 短写与首次创建中断后必须停止，不能把临时记录当旧记录缺失", async () => {
  for (const fault of ["short-write", "after-create", "before-rename"] as const) {
    const { d } = fakeDeps("dry-run", []);
    d.model = async c => ({ text: c.role === "triage"
      ? fence({ class: "T", reason: "接口", ...goodArchObj })
      : c.role === "draft" ? goodDraft + "\n改判建议：P - 涉及真实账户\n" : fence(pass()), durationMs: 1, attempts: 1 });
    const actual = { open: fs.openSync, write: fs.writeFileSync, close: fs.closeSync, rename: fs.renameSync };
    const effective = join(d.outRoot, "1531", "classify.effective.json");
    const pending = effective + ".pending";
    let injected = false;
    let pendingFd: number | undefined;
    fs.openSync = ((path, ...args) => {
      const fd = actual.open(path, ...args);
      if (String(path) === pending) {
        pendingFd = fd;
        if (fault === "after-create") {
          injected = true;
          actual.close(fd);
          throw Object.assign(new Error("注入首次 P 文件创建中断"), { code: "ENOSPC" });
        }
      }
      return fd;
    }) as typeof fs.openSync;
    fs.writeFileSync = ((path, data, options) => {
      if (fault === "short-write" && (String(path) === effective || String(path) === pending || path === pendingFd)) {
        injected = true;
        actual.write(path, '{"klass":"P",');
        throw Object.assign(new Error("注入 P 记录短写失败"), { code: "ENOSPC" });
      }
      return actual.write(path, data, options);
    }) as typeof fs.writeFileSync;
    fs.renameSync = ((from, to) => {
      if (fault === "before-rename" && String(to) === effective) {
        injected = true;
        throw Object.assign(new Error("注入 P 文件发布中断"), { code: "EIO" });
      }
      return actual.rename(from, to);
    }) as typeof fs.renameSync;
    syncBuiltinESMExports();
    let first;
    try { first = await processTicket(1531, d); }
    finally {
      fs.openSync = actual.open;
      fs.writeFileSync = actual.write;
      fs.renameSync = actual.rename;
      syncBuiltinESMExports();
    }
    assert.equal(injected, true, `${fault} 必须命中实际持久化路径`);
    assert.equal(first.phase, "error");
    if (fault === "short-write") {
      const path = existsSync(pending) ? pending : effective;
      assert.equal(readFileSync(path, "utf8"), '{"klass":"P",');
    }
    const calls: ModelCall[] = [];
    d.model = async c => {
      calls.push(c);
      return { text: c.role === "triage" ? fence({ class: "T", reason: "重分", ...goodArchObj }) : c.role === "draft" ? goodDraft : fence(pass()), durationMs: 1, attempts: 1 };
    };
    const second = await processTicket(1531, d);
    assert.equal(second.phase, "error", `${fault} 后不得按旧记录缺失重分`);
    assert.match(second.error ?? "", /有效 P/);
    assert.equal(calls.length, 0);
    assert.ok(!second.plannedWrites.some(w => w.startsWith("分支 decision/")));
  }
});
