import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import {
  QUEUE_ARGS, blockedComment, blockedNote, fetchIssueJson, fetchQueueJson, renderImplementPrompt, renderPlanPrompt, runImplementer, type HostGh,
} from "./swarm-prompts.mts";

const ISSUE = { number: 4242, title: "专用验收票", body: "只改 docs/x.md。含 {{ISSUE_NUMBER}} 与 !`echo hi` 字面。", labels: [{ name: "doc" }], comments: [{ body: "评论一" }] };
const QUEUE = [
  { number: 1, title: "A", body: "b", labels: [{ name: "sandcastle" }], assignees: [] },
  { number: 2, title: "B", body: "b", labels: [{ name: "sandcastle" }], assignees: [{ login: "someone" }] },
];
const fakeGh = (calls: string[][] = []): HostGh => (args) => {
  calls.push(args);
  if (args[1] === "view") return JSON.stringify(ISSUE);
  if (args[1] === "list") return JSON.stringify(QUEUE);
  throw new Error(`unexpected gh ${args.join(" ")}`);
};

test("Docker 专用模板不含任何 shell 展开块，也不引用 gh / GitHub 凭据", () => {
  for (const f of ["swarm-implement-prompt.md", "swarm-plan-prompt.md"]) {
    const text = readFileSync(new URL(`./${f}`, import.meta.url), "utf8");
    assert.doesNotMatch(text, /!`/, f);
    assert.doesNotMatch(text, /\bgh\b|GH_TOKEN|GITHUB_TOKEN/, f);
  }
});

test("Docker 路径使用的 inline prompt 来源里没有 !` 块：评审模板、冲突 prompt、专用模板", () => {
  for (const f of ["codex-review-prompt.md", "swarm-implement-prompt.md", "swarm-plan-prompt.md"]) {
    assert.doesNotMatch(readFileSync(new URL(`./${f}`, import.meta.url), "utf8"), /!`/, f);
  }
  const merge = readFileSync(new URL("./swarm-merge.mts", import.meta.url), "utf8");
  assert.match(merge, /prompt: final\.prompt/); assert.doesNotMatch(merge, /promptFile/);
  // 宿主 main.mts 仍用原模板，行为不变
  const main = readFileSync(new URL("./main.mts", import.meta.url), "utf8");
  assert.match(main, /codex-implement-prompt\.md/);
  assert.match(readFileSync(new URL("./codex-implement-prompt.md", import.meta.url), "utf8"), /!`gh issue view/);
});

test("main-swarm 的 planner 与 implementer 都传 inline prompt，不再有 promptFile", () => {
  const src = readFileSync(new URL("./main-swarm.mts", import.meta.url), "utf8");
  assert.doesNotMatch(src, /promptFile/); assert.doesNotMatch(src, /promptArgs/);
  assert.match(src, /prompt: renderPlanPrompt\(fetchQueueJson\(hostGh\(GH_CLEAN_ENV\)\)\)/);
  assert.match(src, /runImplementer\(issue\.id/);
});

test("implementer：宿主取票面并 inline 渲染，包含票面内容；票面里的 {{…}}/!` 字面不被再次展开", () => {
  const calls: string[][] = [];
  const prompt = renderImplementPrompt(4242, fetchIssueJson(fakeGh(calls), 4242));
  assert.deepEqual(calls, [["issue", "view", "4242", "--json", "number,title,body,labels,comments"]]);
  assert.match(prompt, /专用验收票/); assert.match(prompt, /评论一/); assert.match(prompt, /只改 docs\/x\.md/);
  assert.match(prompt, /含 \{\{ISSUE_NUMBER\}\} 与 !`echo hi` 字面/, "正文原样保留，不二次展开");
  assert.doesNotMatch(prompt.replace(/含 \{\{ISSUE_NUMBER\}\} 与 !`echo hi` 字面/, ""), /!`|\{\{[A-Z_]+\}\}/);
  assert.match(prompt, /status":"blocked"/); assert.doesNotMatch(prompt, /gh issue comment/);
});

test("planner：宿主列队列，过滤语义不变（label sandcastle、open、limit 50、无 assignee）", () => {
  assert.deepEqual(QUEUE_ARGS.slice(0, 8), ["issue", "list", "--label", "sandcastle", "--state", "open", "--limit", "50"]);
  const json = fetchQueueJson(fakeGh());
  assert.deepEqual(JSON.parse(json), [{ number: 1, title: "A", labels: ["sandcastle"] }]);
  const prompt = renderPlanPrompt(json);
  assert.match(prompt, /"number":1/); assert.doesNotMatch(prompt, /"number":2/);
  assert.doesNotMatch(prompt, /!`|\{\{[A-Z_]+\}\}/);
  const original = readFileSync(new URL("./plan-prompt.md", import.meta.url), "utf8");
  assert.match(original, /--label sandcastle --state open --limit 50/);
  assert.match(original, /select\(\.assignees \| length == 0\)/);
});

test("runImplementer：传给 sandbox.run 的是 inline prompt（无 promptFile、无 !`），blocked 由宿主回写票面", async () => {
  const seen: Array<Record<string, unknown>> = []; const notes: Array<[number, string]> = [];
  const blocked = '<sandcastle-result>{"status":"blocked","files":[],"verification":["卡点：缺数据","续跑：重跑"]}</sandcastle-result>';
  const ready = '<sandcastle-result>{"status":"ready","files":["docs/x.md"],"verification":["已核对"]}</sandcastle-result>';
  for (const out of [ready, blocked]) {
    seen.length = 0; notes.length = 0;
    const stdout = await runImplementer(4242, { gh: fakeGh(), agent: "A", notify: (i, b) => notes.push([i, b]), run: async (o) => { seen.push(o); return { stdout: out }; } });
    assert.equal(stdout, out);
    assert.equal(seen.length, 1);
    assert.ok(!("promptFile" in seen[0]!) && !("promptArgs" in seen[0]!));
    assert.equal(typeof seen[0]!.prompt, "string");
    assert.match(seen[0]!.prompt as string, /专用验收票/); assert.doesNotMatch(seen[0]!.prompt as string, /^!`/m);
    if (out === blocked) { assert.equal(notes.length, 1); assert.equal(notes[0]![0], 4242); assert.match(notes[0]![1], /卡点：缺数据\n续跑：重跑/); }
    else assert.equal(notes.length, 0);
  }
});

test("blockedNote / blockedComment", () => {
  assert.equal(blockedNote("无报告"), null);
  assert.equal(blockedNote('<sandcastle-result>{"status":"blocked","files":[],"verification":[]}</sandcastle-result>'), "（工蜂报告 blocked，未附说明）");
  assert.match(blockedComment(7, "x"), /#7[\s\S]*x/);
});

test("Docker 路径的所有 prompt 来源均不依赖容器内 gh：.sandcastle 里仍含 !` 的文件只有宿主路径/历史模板", () => {
  const hits = readdirSync(new URL("./", import.meta.url)).filter((f) => f.endsWith(".md") && f !== "README.md").filter((f) => /!`/.test(readFileSync(new URL(`./${f}`, import.meta.url), "utf8"))).sort();
  assert.deepEqual(hits, ["codex-implement-prompt.md", "implement-prompt.md", "plan-prompt.md", "prompt.md.ralph.md"].sort());
});
