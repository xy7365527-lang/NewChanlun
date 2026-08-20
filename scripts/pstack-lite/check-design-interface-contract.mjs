#!/usr/bin/env node
// design-an-interface：Prime RLM 扇出、结果回流与完成门的确定性契约检查（#1136）。
// 不启动模型；用真实文本/JSONL 形状驱动回收判据，并核对现役 Skill/prompt。
import { readFileSync } from "node:fs";
import { dirname, isAbsolute, join, relative, resolve, sep } from "node:path";
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

function completeText(text) {
  if (typeof text !== "string") return false;
  const headings = fixture.required_headings;
  const matches = [...text.matchAll(/^## ([^\n]+)\s*$/gm)];
  let previous = -1;
  for (const heading of headings) {
    const matchIndex = matches.findIndex((m, index) => index > previous && m[1].trim() === heading);
    if (matchIndex < 0) return false;
    const start = matches[matchIndex].index + matches[matchIndex][0].length;
    const end = matches[matchIndex + 1]?.index ?? text.length;
    if (text.slice(start, end).trim().length === 0) return false;
    previous = matchIndex;
  }
  return true;
}

function completeJsonl(records) {
  if (!Array.isArray(records)) return false;
  for (let index = records.length - 1; index >= 0; index -= 1) {
    const message = records[index]?.type === "message" ? records[index].message : undefined;
    if (message?.role !== "assistant" || !Array.isArray(message.content)) continue;
    const text = message.content
      .filter((part) => part?.type === "text")
      .map((part) => part.text ?? fixture.payloads[part.payload_ref])
      .filter((part) => typeof part === "string")
      .join("\n");
    if (completeText(text)) return true;
  }
  return false;
}

function isComplete(entry) {
  if (entry?.channel === "agent_message" || entry?.channel === "result_file") {
    return completeText(entry.payload ?? fixture.payloads[entry.payload_ref]);
  }
  if (entry?.channel === "final_jsonl") return entry.jsonl_file_count === 1 && completeJsonl(entry.jsonl_records);
  return false;
}

function validResultPaths(testCase) {
  const rootDir = resolve(testCase.root_session_dir);
  const resolved = testCase.result_files.map((path) => resolve(path));
  const unique = new Set(resolved).size === resolved.length;
  const insideRoot = testCase.result_files.every((path, index) => {
    if (!isAbsolute(path)) return false;
    const rel = relative(rootDir, resolved[index]);
    return rel !== "" && rel !== ".." && !rel.startsWith(`..${sep}`) && !isAbsolute(rel);
  });
  return unique && insideRoot;
}

for (const c of fixture.completion_cases) {
  const actual = isComplete(c);
  check(`完成门/${c.id}`, actual === c.expect, `actual=${actual} expect=${c.expect}`);
}
for (const c of fixture.candidate_sets) {
  const actual = c.candidates.length >= fixture.defaults.minimum_candidates && c.candidates.every(isComplete);
  check(`候选集合/${c.id}`, actual === c.expect_compare, `actual=${actual} expect=${c.expect_compare}`);
}
for (const c of fixture.result_path_cases) {
  const actual = validResultPaths(c);
  check(`结果路径/${c.id}`, actual === c.expect, `actual=${actual} expect=${c.expect}`);
}
for (const c of fixture.render_cases) {
  let rendered = prompt;
  for (const [name, value] of Object.entries(c.values)) rendered = rendered.replaceAll(`{${name}}`, String(value));
  const remaining = fixture.required_placeholders.filter((placeholder) => rendered.includes(placeholder));
  const actual = remaining.length === 0;
  check(`prompt 渲染/${c.id}`, actual === c.expect_no_placeholders, `remaining=${remaining.join(",") || "none"}`);
}

const d = fixture.defaults;
check("旧同步 Task 语义已清除", !skill.includes("Task tool"), "no legacy Task tool wording");
check("根代理持有至少三个独立 RLM admission", d.minimum_candidates === 3 && skill.includes("至少分别调用三次") && skill.includes("保留每个返回的 handle"), `minimum=${d.minimum_candidates}`);
check("admission handle 明确不是结果", skill.includes("不是设计结果") && skill.includes("admission handle 都不是结果"), "handle-only rejected");
check("根 session-dir 预分配绝对结果文件", skill.includes("根 `session-dir`") && skill.includes("绝对结果文件路径") && skill.includes("admission 前"), "preallocated absolute root path");
check("prompt 占位符完整", fixture.required_placeholders.every((p) => prompt.includes(p)), fixture.required_placeholders.join(","));
check("预算单位与回流预留", prompt.includes("{TIME_BUDGET} minutes（分钟）") && prompt.includes("{OUTPUT_BUDGET} words（词）") && prompt.includes("{RETURN_TOOL_RESERVE}") && prompt.includes("{CONTEXT_STOP_PERCENT}%"), JSON.stringify(d));
check("child 先消息后同内容文件 fallback", prompt.includes("发送失败，把**同一份完整结果**写入") && prompt.includes("唯一允许的写操作"), "message -> result file");
check("最终 assistant response 保留完整结果", skill.includes("最终 assistant response 仍须保留同一份完整结果") && prompt.includes("最终内容必须是完整结果"), "recoverable final JSONL");
check("final JSONL 回收步骤可执行", skill.includes("handle.session_dir") && skill.includes("枚举 `*.jsonl`") && skill.includes('message.role = "assistant"') && skill.includes('type = "text"'), "handle -> one JSONL -> assistant text");
check("完整结果有机械判据", fixture.required_headings.every((heading) => skill.includes(`\`${heading}\``)) && skill.includes("各自正文非空"), fixture.required_headings.join(","));
check("缺结果时只追问原 child 一次", skill.includes("一次 bounded follow-up") && skill.includes("receiver_name=handle.name") && skill.includes("然后再检查一次三个渠道"), "one bounded recovery");
check("caller-first 输出契约保留", ["Caller's usage", "Interface signature", "What it hides", "Trade-offs"].every((x) => prompt.includes(`## ${x}`)), "existing output shape");
check("四类设计红旗完整", ["shallow module", "information leakage", "temporal decomposition", "pass-through method"].every((x) => prompt.includes(x)), "four red flags");
check("不重复调用 arena", skill.includes("不得对同一接口问题调用 `pstack-arena`") && skill.includes("不得把这次扇出再委托"), "single owner");
check("codebase-design 保持单方案不二次扇出", deepSkill.includes("single-scheme deep-module design") && !deepSkill.includes("rlm('sub-task'") && !deepSkill.includes("Task tool"), "single scheme only");
check("普通小改动仍跳过", skill.includes("small, obvious change") && deepSkill.includes("small, obvious change"), "trivial control");

const report = { mode: "design-interface-async-contract", ok: checks.every((c) => c.ok), checks };
console.log(JSON.stringify(report, null, 2));
process.exit(report.ok ? 0 : 1);
