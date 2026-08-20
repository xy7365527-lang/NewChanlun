#!/usr/bin/env node
// pstack-arena：解析 Skill 内唯一结构化安全契约，并对危险 mutation 做确定性回归（#1132）。
// 不启动模型；生产载体就是 SKILL.md，本检查不另造一份调度规则。
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..", "..");
const skillPath = join(root, ".agents", "skills", "pstack-arena", "SKILL.md");
const skill = readFileSync(skillPath, "utf8");
const fixtures = JSON.parse(readFileSync(join(root, "docs", "agents", "pstack-lite", "fixtures.json"), "utf8"));
const arenaFixture = fixtures.items.find((item) => item.id === "pstack-arena");
const checks = [];
const check = (name, ok, detail) => checks.push({ name, ok: Boolean(ok), detail });

function parseDocument(text) {
  const frontmatter = text.match(/^---\n([\s\S]*?)\n---/)?.[1] ?? "";
  const blocks = [...text.matchAll(/## 机器可检安全契约（唯一规则块）[\s\S]*?```json\n([\s\S]*?)\n```/g)];
  let contract = null;
  let parseError = null;
  if (blocks.length === 1) {
    try { contract = JSON.parse(blocks[0][1]); } catch (error) { parseError = String(error); }
  }
  const proseWithoutContract = blocks.length === 1 ? text.replace(blocks[0][0], "") : text;
  const contradictoryLines = proseWithoutContract.split("\n").filter((line) =>
    /(?:允许|可以|可)(?:补派|重派|整轮重派|第二轮|直接调用)/.test(line) ||
    /(?:失败|超时).{0,24}(?:允许|可以).{0,12}(?:重跑|补派|重派)/.test(line));
  return { frontmatter, blocks, contract, parseError, contradictoryLines };
}

function validContract(c) {
  return c?.version === 1 &&
    c.invocation === "user-only" &&
    JSON.stringify(c.explicit_entries) === JSON.stringify(["/skill:pstack-arena", "arena this", "扔进竞技场"]) &&
    c.candidate_fanout_rounds === 1 &&
    c.candidate_failure === "drop-out" &&
    c.minimum_valid_candidates === 2 &&
    c.read_only_executor === "RLM" &&
    c.write_executor === "Sandcastle" &&
    c.write_requires_authorization === true &&
    c.direct_cli_fallback === "forbidden" &&
    JSON.stringify(c.result_channels) === JSON.stringify(["agent_message", "preallocated_task_scratch_file", "final_jsonl"]) &&
    JSON.stringify(c.outside_task_scratch_allowlist) === JSON.stringify(["mandatory_roster_registration"]) &&
    c.judge_failure === "root_completes_without_replacement";
}

function validDocument(text) {
  const parsed = parseDocument(text);
  return /^disable-model-invocation:\s*true$/m.test(parsed.frontmatter) &&
    parsed.blocks.length === 1 && !parsed.parseError && validContract(parsed.contract) &&
    parsed.contradictoryLines.length === 0;
}

function decisionFromContract(c, candidates) {
  const valid = candidates.filter((candidate) => candidate.complete === true).length;
  return {
    valid,
    dropped: candidates.length - valid,
    proceedToJudge: valid >= c.minimum_valid_candidates,
    retryAdmissionsAllowed: c.candidate_fanout_rounds > 1 || c.candidate_failure !== "drop-out",
  };
}

const parsed = parseDocument(skill);
check("唯一结构化规则块可解析", parsed.blocks.length === 1 && !parsed.parseError, `blocks=${parsed.blocks.length} error=${parsed.parseError ?? "none"}`);
check("结构化安全契约字段完整", validContract(parsed.contract), JSON.stringify(parsed.contract));
check("user-only frontmatter", /^disable-model-invocation:\s*true$/m.test(parsed.frontmatter), "disable-model-invocation=true");
check("正文无相反例外", parsed.contradictoryLines.length === 0, parsed.contradictoryLines.join(" | ") || "none");
check("自动 fixture 已降 draft", arenaFixture?.status === "draft", `status=${arenaFixture?.status}`);

for (const testCase of [
  { id: "all-complete", candidates: [{ complete: true }, { complete: true }, { complete: true }], proceed: true, dropped: 0 },
  { id: "n-minus-one", candidates: [{ complete: true }, { complete: false }, { complete: true }], proceed: true, dropped: 1 },
  { id: "fewer-than-two", candidates: [{ complete: false }, { complete: true }, { complete: false }], proceed: false, dropped: 2 },
]) {
  const actual = decisionFromContract(parsed.contract, testCase.candidates);
  check(`契约决策/${testCase.id}`, actual.proceedToJudge === testCase.proceed && actual.dropped === testCase.dropped && actual.retryAdmissionsAllowed === false, JSON.stringify(actual));
}

const mutations = [
  ["自动调用", (text) => text.replace("disable-model-invocation: true", "disable-model-invocation: false")],
  ["第二轮", (text) => text.replace('"candidate_fanout_rounds": 1', '"candidate_fanout_rounds": 2')],
  ["直接 CLI fallback", (text) => text.replace('"direct_cli_fallback": "forbidden"', '"direct_cli_fallback": "allowed"')],
  ["写候选免授权", (text) => text.replace('"write_requires_authorization": true', '"write_requires_authorization": false')],
  ["额外 scratch 例外", (text) => text.replace('["mandatory_roster_registration"]', '["mandatory_roster_registration", "temp"]')],
  ["补派评委", (text) => text.replace('"root_completes_without_replacement"', '"replace_judge"')],
  ["矛盾正文", (text) => `${text}\n失败时允许第二轮候选 fan-out。\n`],
];
for (const [name, mutate] of mutations) {
  check(`mutation 必须失败/${name}`, !validDocument(mutate(skill)), "rejected");
}

check("显式入口仍存在", ["/skill:pstack-arena", "arena this", "扔进竞技场"].every((entry) => skill.includes(entry)), "three explicit entries");
check("直接 CLI 禁令同句覆盖三类", /不得直接调用[^。\n]*`codex`[^。\n]*`claude`[^。\n]*`prime-agent`[^。\n]*外部 CLI/.test(skill), "Codex/Claude/Prime + generic");
check("执行面和回流正文对齐契约", skill.includes("只走 RLM") && skill.includes("必须走 **Sandcastle 隔离**") && skill.includes("未获写入授权时不得启动写代码候选") && ["`agent_message`", "预分配的独立结果文件", "final JSONL"].every((text) => skill.includes(text)), "prose alignment");
check("单轮与完成门正文对齐契约", skill.includes("整个 arena 只准这一轮 fan-out") && skill.includes("有效候选少于 2 个时立即停止 arena") && skill.includes("不得只交付「评委评分中」"), "prose alignment");

const report = { mode: "pstack-arena-contract", ok: checks.every((item) => item.ok), checks };
console.log(JSON.stringify(report, null, 2));
process.exit(report.ok ? 0 : 1);
