#!/usr/bin/env node
// design-an-interface：Prime RLM 扇出、结果回流与完成门的确定性契约检查（#1136）。
// 不启动模型；只验证 fixture 与现役 Skills/prompt 的文本契约。
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..", "..");
const designDir = join(root, ".agents", "skills", "design-an-interface");
const skill = readFileSync(join(designDir, "SKILL.md"), "utf8");
const prompt = readFileSync(join(designDir, "references", "design-candidate-prompt.md"), "utf8");
const fixture = JSON.parse(readFileSync(join(designDir, "references", "async-contract-fixtures.json"), "utf8"));
const deepSkill = readFileSync(join(root, ".agents", "skills", "codebase-design", "SKILL.md"), "utf8");
const checks = [];
const check = (name, ok, detail) => checks.push({ name, ok: Boolean(ok), detail });

const completionChannels = new Set(["agent_message", "result_file", "final_jsonl"]);
function isComplete(entry) {
  return entry.complete === true && completionChannels.has(entry.channel);
}

for (const c of fixture.completion_cases) {
  const actual = isComplete(c);
  check(`完成门/${c.id}`, actual === c.expect, `actual=${actual} expect=${c.expect}`);
}
for (const c of fixture.candidate_sets) {
  const actual = c.candidates.length >= fixture.defaults.minimum_candidates && c.candidates.every(isComplete);
  check(`候选集合/${c.id}`, actual === c.expect_compare, `actual=${actual} expect=${c.expect_compare}`);
}

const d = fixture.defaults;
check("旧同步 Task 语义已清除", !skill.includes("Task tool"), "no legacy Task tool wording");
check("根代理持有至少三个独立 RLM admission", skill.includes("at least three independent children") && skill.includes("separate `await rlm('sub-task'") && skill.includes("Keep every returned handle"), `minimum=${d.minimum_candidates}`);
check("admission handle 明确不是结果", skill.includes("not a design result") && skill.includes("admission handles are not results"), "handle-only rejected");
check("根 session-dir 预分配绝对结果文件", skill.includes("absolute result-file path in the root session directory") && skill.includes("Before admission"), "preallocated absolute root path");
check("prompt 占位符完整", fixture.required_placeholders.every((p) => prompt.includes(p)), fixture.required_placeholders.join(","));
check("默认预算与回流预留", skill.includes(`**${d.read_budget} files**`) && skill.includes(`**${d.tool_budget} tool calls**`) && skill.includes(`**${d.time_budget_minutes} minutes**`) && skill.includes(`**1,200 words**`) && skill.includes(`**${d.return_tool_reserve} tool calls**`) && skill.includes(`**${d.context_stop_percent}% context**`), JSON.stringify(d));
check("child 先消息后同内容文件 fallback", prompt.includes("is unavailable or not imported") && prompt.includes("same complete result**") && prompt.includes("only permitted write"), "message -> result file");
check("final JSONL 为第三回收通道", skill.includes("complete final response in its final session JSONL located through the retained handle/session directory") && prompt.includes("final session JSONL"), "message/file/final JSONL");
check("父完成门覆盖三个真实通道", skill.includes("complete parent message, the assigned result file, or the child's complete final response"), "three completion channels");
check("缺结果时降级且父不得代写", skill.includes("stop before comparison") && skill.includes("do not invent it") && skill.includes("root-authored work"), "no incomplete comparison or parent substitute");
check("caller-first 输出契约保留", prompt.includes("Caller's usage") && prompt.includes("Interface signature") && prompt.includes("What it hides") && prompt.includes("Trade-offs"), "existing output shape");
check("三种方案须真正不同", skill.includes("genuinely different design pressure") && skill.includes("does not count as a different design"), "not cosmetic variants");
check("四类设计红旗完整", ["shallow module", "information leakage", "temporal decomposition", "pass-through method"].every((x) => prompt.includes(x)), "four red flags");
check("不重复调用 arena", skill.includes("must not invoke `pstack-arena`") && skill.includes("root must not delegate this fan-out"), "single owner");
check("codebase-design 保持单方案不二次扇出", deepSkill.includes("single-scheme deep-module design") && !deepSkill.includes("rlm('sub-task'") && !deepSkill.includes("Task tool"), "single scheme only");
check("普通小改动仍跳过", skill.includes("small, obvious change") && deepSkill.includes("small, obvious change"), "trivial control");

const report = { mode: "design-interface-async-contract", ok: checks.every((c) => c.ok), checks };
console.log(JSON.stringify(report, null, 2));
process.exit(report.ok ? 0 : 1);
