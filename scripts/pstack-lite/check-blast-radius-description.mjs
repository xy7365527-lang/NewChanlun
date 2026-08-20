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
const triggerSection = triggerDocs.match(
  /(?:^|\n)## pstack-blast-radius 降级记录（#1138）\n[\s\S]*?(?=\n## |$)/,
)?.[0] ?? "";
const customizationRow = customizations
  .split("\n")
  .find((line) => line.startsWith("| `pstack-blast-radius` |")) ?? "";
const behaviorBody = skill.split("\n## 本地 Prime 改写")[0];
const failures = [];
const requireCheck = (condition, message) => {
  if (!condition) failures.push(message);
};

// user-only 路由契约：flag + 显式入口，description 不再承诺自动加载。
requireCheck(/^disable-model-invocation: true$/m.test(frontmatter), "frontmatter 缺 disable-model-invocation: true");
requireCheck(description.includes("仅限用户显式调用（user-only）"), "description 未声明 user-only");
requireCheck(description.includes("/skill:pstack-blast-radius"), "description 缺显式 /skill 入口");
requireCheck(description.includes("已禁用模型自动调用"), "description 未声明禁用自动模型调用");
for (const promise of ["MUST load", "必须加载", "自动加载本 Skill", "可自动调用", "自动语义加载"]) {
  requireCheck(!description.includes(promise), `description 仍承诺自动加载：${promise}`);
}
requireCheck(skill.includes("仅通过 `/skill:pstack-blast-radius` 显式调用"), "正文缺显式调用入口");

// 原 3+3 保留为 draft，重启条件必须写死；快照防止只保留数量却替换语料。
const expectedCases = {
  positive: [
    ["blast-pos-1", "我改了这个返回类型的字段，帮我找出 diff 之外还会被影响的调用者和依赖。", "loaded"],
    ["blast-pos-2", "这个非小型行为变更会影响哪些其他路径？跑一条能证明安全性的靶向验证。", "loaded"],
    ["blast-pos-3", "改了错误码的语义，列出所有依赖这个错误码的位置，并给出跨模块的证明。", "loaded"],
  ],
  negative: [
    ["blast-neg-1", "把这个函数重命名，批量替换一下引用。", "skipped"],
    ["blast-neg-2", "按 rustfmt 格式化这个文件。", "skipped"],
    ["blast-neg-3", "给这段注释改个错别字。", "skipped"],
  ],
};
const caseSnapshot = (cases) => cases?.map(({ id, prompt, expect }) => [id, prompt, expect]);
requireCheck(fixture?.skill === "pstack-blast-radius", "fixture skill 指向错误");
requireCheck(fixture?.skill_dir === "../../../.agents/skills/pstack-blast-radius", "fixture skill_dir 指向错误");
requireCheck(fixture?.status === "draft", "pstack-blast-radius fixture 必须为 draft");
requireCheck(JSON.stringify(caseSnapshot(fixture?.positive)) === JSON.stringify(expectedCases.positive), "原 3 条正例 fixture 未逐字保留");
requireCheck(JSON.stringify(caseSnapshot(fixture?.negative)) === JSON.stringify(expectedCases.negative), "原 3 条反例 fixture 未逐字保留");
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
  requireCheck(behaviorBody.includes(phrase), `行为正文缺契约：${phrase}`);
}
requireCheck(customizationRow.includes("已实现、user-only（#1133/#1138）"), "本地定制表对应行未记录 user-only 降级");
requireCheck(customizationRow.includes("fixture 降为 draft") && customizationRow.includes("行为能力不回滚"), "本地定制表对应行未记录 draft 或能力保留");
requireCheck(triggerSection.includes("`disable-model-invocation: true` 的 user-only 能力"), "TRIGGER-FIXTURES 降级节未记录 user-only flag");
requireCheck(triggerSection.includes("至少两个模型上重复稳定通过"), "TRIGGER-FIXTURES 降级节缺双模型重启条件");
requireCheck(triggerSection.includes("#1139 十任务试点不计其自动触发"), "TRIGGER-FIXTURES 降级节未排除 #1139 自动触发");

const report = { mode: "pstack-blast-radius-user-only-contract", ok: failures.length === 0, failures };
console.log(JSON.stringify(report, null, 2));
process.exit(report.ok ? 0 : 1);
