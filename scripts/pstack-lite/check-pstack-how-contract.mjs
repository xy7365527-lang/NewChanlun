#!/usr/bin/env node
// pstack-how：异步 explorer 预算、fallback 与父完成门的确定性契约检查（#1131）。
// 不启动模型，不读会话网络状态；只验证可执行 fixture 与 Skill/prompt 的契约一致。
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "..", "..");
const skillDir = join(root, ".agents", "skills", "pstack-how");
const skill = readFileSync(join(skillDir, "SKILL.md"), "utf8");
const prompt = readFileSync(join(skillDir, "references", "explorer-prompt.md"), "utf8");
const fixture = JSON.parse(readFileSync(join(skillDir, "references", "async-contract-fixtures.json"), "utf8"));
const checks = [];
const check = (name, ok, detail) => checks.push({ name, ok: Boolean(ok), detail });

function isCompletionEvidence(entry) {
  return entry.complete === true && (entry.evidence === "agent_message" || entry.evidence === "result_file");
}

for (const c of fixture.completion_cases) {
  const actual = isCompletionEvidence(c);
  check(`完成门/${c.id}`, actual === c.expect, `actual=${actual} expect=${c.expect}`);
}

const d = fixture.defaults;
check("Skill 固定默认小预算", skill.includes(`最多读 **${d.read_budget} 个文件**`) && skill.includes(`最多 **${d.tool_budget} 次工具调用**`) && skill.includes(`最多 **${d.time_budget_minutes} 分钟**`), JSON.stringify(d));
check("Skill 与 prompt 均锁住 70% context 停探", skill.includes(`**${d.context_stop_percent}% context**`) && prompt.includes(`**${d.context_stop_percent}% context**`), `${d.context_stop_percent}%`);
check("工具预算为消息与 fallback 留出尾部调用", skill.includes(`最后 **${d.return_tool_reserve} 次**预留`) && prompt.includes(`最后 **${d.return_tool_reserve} 次**只预留`) && prompt.includes(`工具调用预算仅剩最后 **${d.return_tool_reserve} 次**`), `reserve=${d.return_tool_reserve}`);
check("prompt 要求 read/tool/time/result 全部由根传入", fixture.required_placeholders.every((p) => prompt.includes(p)), fixture.required_placeholders.join(","));
check("结果文件由根在自己的 session-dir 预分配绝对路径", skill.includes("绝对结果文件路径") && skill.includes("根代理在自己的 session-dir 中") && skill.includes("不能等 admission 后"), "absolute preallocated root session-dir path");
check("fallback 仅在 agent_message 失败或不可用时写同一完整结果", /若 `agent_message\.send` 失败或不可用[\s\S]*同一份完整结果[\s\S]*\{RESULT_FILE\}/.test(prompt), "fallback ordering + same payload + path");
check("fallback 文件是唯一允许写操作", prompt.includes("这是唯一允许的写操作"), "only write");
check("禁止以源码 toolResult 尾停", skill.includes("源码 `toolResult` 尾事件都不满足完成门") && prompt.includes("不得停在源码 `toolResult`"), "parent gate + child termination");
check("未知项必须显式保留且不得编造", prompt.includes("Open Questions") && prompt.includes("不得为了显得完整继续读、猜测或编造"), "honest truncation");
check("父代理只接受消息或结果文件", skill.includes("收到该 explorer 的完整 `agent_message`") && skill.includes("`{RESULT_FILE}` 写入的同一份完整结果"), "two evidence types");
check("汇总同时接受消息与结果文件完成门", skill.includes("等每个 explorer 都通过完成门") && skill.includes("读到指定 `{RESULT_FILE}`"), "aggregation uses the same two evidence types");
check("关键未决问题最多一次 follow-up", skill.includes("最多对原 explorer 做一次 follow-up"), "bounded follow-up");

const report = { mode: "pstack-how-contract", ok: checks.every((c) => c.ok), checks };
console.log(JSON.stringify(report, null, 2));
process.exit(report.ok ? 0 : 1);
