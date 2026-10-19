import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  ADR_FORMAT_LINE, AUTH_LINE, CITATION_SECTION, DRAFT_SECTIONS, GH_API_WRITE_DENY, READ_TOOLS, hasCitationTable, toolArgs, FETCH_URL, MAIN_REF, MODEL_FABLE, MODEL_OPUS, DryRunWriteError,
  allocateAdr, applyReclass, draftPrompt, ensureSrc, extractFirstApiKey, guardGh, reclassSuggestion, renumberDraft, reviewPrompt, initialState, isGhWrite, landingTarget, missingDraftSections,
  nextAfterReview, parseArgs, parseClassification, parseReview, processTicket, selectCandidates,
  type Deps, type GhRunner, type ModelCall,
} from "./decision-claimer.mts";

const pass = (basis = "回查成立") => Object.fromEntries(["R1", "R2", "R3", "R4", "R5", "R6", "R7"].map((r) => [r, { verdict: "PASS", basis }]));
const fence = (v: unknown) => "说明文字\n```json\n" + JSON.stringify(v) + "\n```\n";
const citeTable = `## ${CITATION_SECTION}\n| 引用 | 核实命令 | 结果摘要 |\n|---|---|---|\n| AGENTS.md:1 | rg -n x AGENTS.md | 见行 |`;
const goodDraft = DRAFT_SECTIONS.map((s) => `## ${s}\n内容${s === "裁定" ? "\n正本落点：docs/adr/0099-x.md" : ""}`).join("\n\n") + "\n\n" + citeTable + "\n";

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

test("审查解析：七项全 PASS 才 PASS；缺项、缺依据、草稿缺段都 BLOCK", () => {
  const ok = parseReview(fence(pass()));
  assert.equal(ok.verdict, "PASS");
  assert.equal(ok.items.length, 7);

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
  assert.equal(t.drafterModel, MODEL_FABLE);
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

function fakeDeps(mode: "dry-run" | "live", replies: string[]) {
  const ghCalls: string[][] = [];
  const modelCalls: ModelCall[] = [];
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
    gh: guardGh(mode, raw),
    git: () => { throw new Error("dry-run 不应调 git"); },
    model: async (c) => {
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
  return { d, ghCalls, modelCalls };
}

const isWrite = (a: string[]) => isGhWrite(a);

test("dry-run 全流程：T 类 PASS 只落本地三份文件，不调任何 gh 写命令", async () => {
  const { d, ghCalls, modelCalls } = fakeDeps("dry-run", [fence({ class: "T", reason: "测试锁" }), goodDraft, fence(pass())]);
  const s = await processTicket(1531, d);
  assert.equal(s.phase, "land_t");
  assert.equal(s.error, undefined);
  assert.deepEqual(modelCalls.map((c) => [c.role, c.model]), [["classify", MODEL_FABLE], ["draft", MODEL_FABLE], ["review", MODEL_OPUS]]);
  assert.ok(modelCalls.every((c) => !c.allowWrite));
  assert.equal(ghCalls.filter(isWrite).length, 0);
  const dir = join(d.outRoot, "1531");
  for (const f of ["classify.json", "draft.md", "review.json", "summary.json"]) assert.ok(existsSync(join(dir, f)), f);
  assert.equal(existsSync(join(dir, ".lock")), false);
  assert.equal(JSON.parse(readFileSync(join(dir, "review.json"), "utf8")).items.length, 7);
  assert.ok(s.plannedWrites.some((w) => w.startsWith("分支 decision/1531")));
});

test("dry-run 两轮 BLOCK：第二稿由 Opus 起草并带审查意见，最终转 P，仍不写 GitHub", async () => {
  const block = fence({ ...pass(), R1: { verdict: "BLOCK", basis: "行号漂移" } });
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
  const draft = DRAFT_SECTIONS.map((x) => `## ${x}\n内容${x === "裁定" ? "\n正本落点：docs/adr/NNNN-x.md\n依 ADR NNNN" : ""}`).join("\n\n") + "\n\n" + citeTable;
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
