import { test } from "node:test";
import assert from "node:assert/strict";
import { existsSync, mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  DRAFT_SECTIONS, MODEL_FABLE, MODEL_OPUS, DryRunWriteError,
  extractFirstApiKey, guardGh, initialState, isGhWrite, landingTarget, missingDraftSections,
  nextAfterReview, parseArgs, parseClassification, parseReview, processTicket, selectCandidates,
  type Deps, type GhRunner, type ModelCall,
} from "./decision-claimer.mts";

const pass = (basis = "回查成立") => Object.fromEntries(["R1", "R2", "R3", "R4", "R5", "R6", "R7"].map((r) => [r, { verdict: "PASS", basis }]));
const fence = (v: unknown) => "说明文字\n```json\n" + JSON.stringify(v) + "\n```\n";
const goodDraft = DRAFT_SECTIONS.map((s) => `## ${s}\n内容${s === "裁定" ? "\n正本落点：docs/adr/0099-x.md" : ""}`).join("\n\n");

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
