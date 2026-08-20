#!/usr/bin/env node
// #1133/#1138 确定性 user-only 契约：不调用真实模型。
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const read = (...parts) => readFileSync(join(root, ...parts), "utf8");
const skill = read(".agents", "skills", "pstack-blast-radius", "SKILL.md");
const frontmatter = skill.match(/^---\n([\s\S]*?)\n---/)?.[1] ?? "";
const descriptionLine = frontmatter.split("\n").find((line) => line.startsWith("description: "));
let description = "";
try {
  description = JSON.parse(descriptionLine?.slice("description: ".length) ?? "");
} catch {}

const fixtures = JSON.parse(read("docs", "agents", "pstack-lite", "fixtures.json"));
const fixture = fixtures.items?.find((item) => item.id === "pstack-blast-radius");
const customizations = read("docs", "agents", "skill-local-customizations.md");
const triggerDocs = read("docs", "agents", "pstack-lite", "TRIGGER-FIXTURES.md");
const failures = [];
const requireCheck = (condition, message) => {
  if (!condition) failures.push(message);
};

// user-only 路由契约：flag + 显式入口，description 不再承诺自动加载。
requireCheck(/^disable-model-invocation: true$/m.test(frontmatter), "frontmatter 缺 disable-model-invocation: true");
requireCheck(description.includes("User-only"), "description 未声明 User-only");
requireCheck(description.includes("/skill:pstack-blast-radius"), "description 缺显式 /skill 入口");
requireCheck(description.includes("automatic model invocation is disabled"), "description 未声明禁用自动模型调用");
requireCheck(!/MUST load|自动调用|自动语义加载/.test(description), "description 仍承诺自动加载");
requireCheck(skill.includes("仅通过 `/skill:pstack-blast-radius` 显式调用"), "正文缺显式调用入口");

// 原 3+3 保留为 draft，重启条件必须写死。
requireCheck(fixture?.status === "draft", "pstack-blast-radius fixture 必须为 draft");
requireCheck(fixture?.positive?.length === 3 && fixture?.negative?.length === 3, "原 3+3 fixture 未完整保留");
requireCheck(fixture?.positive?.every((c) => c.expect === "loaded"), "正例预期应继续保留 loaded");
requireCheck(fixture?.negative?.every((c) => c.expect === "skipped"), "反例预期应继续保留 skipped");
requireCheck(fixture?.note?.includes("#1138") && fixture.note.includes("至少两个模型") && fixture.note.includes("重复稳定通过"), "fixture note 缺 #1138 降级原因或双模型重启条件");

// 降级只改路由，不回滚行为能力。
for (const phrase of [
  "Serena",
  "codebase-memory",
  "代码搜索（grep/ripgrep）",
  "潜在破坏面（未证实的主张）",
  "实际验证证据（已运行的命令）",
  "靶向验证命令要在错时大声失败",
  "grep 零命中（未发现，非行为证明）",
]) {
  requireCheck(skill.includes(phrase), `行为正文缺契约：${phrase}`);
}
requireCheck(customizations.includes("已实现、user-only（#1133/#1138）"), "本地定制表未记录 user-only 降级");
requireCheck(triggerDocs.includes("pstack-blast-radius 降级记录（#1138）") && triggerDocs.includes("至少两个模型上重复稳定通过"), "TRIGGER-FIXTURES 未记录降级或重启条件");

const report = { mode: "pstack-blast-radius-user-only-contract", ok: failures.length === 0, failures };
console.log(JSON.stringify(report, null, 2));
process.exit(report.ok ? 0 : 1);
