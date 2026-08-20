#!/usr/bin/env node
// #1133 确定性路由契约：只检查 pstack-blast-radius 的 frontmatter，不调用真实模型。
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const skill = readFileSync(join(root, ".agents", "skills", "pstack-blast-radius", "SKILL.md"), "utf8");
const frontmatter = skill.match(/^---\n([\s\S]*?)\n---/)?.[1] ?? "";
const descriptionLine = frontmatter.split("\n").find((line) => line.startsWith("description: "));
let description = "";
try {
  description = JSON.parse(descriptionLine?.slice("description: ".length) ?? "");
} catch {}

const required = [
  "MUST load/read this Skill before analysis",
  "observable behavior/contract change",
  "blast radius",
  "diff 之外",
  "证明安全性",
  "跨模块的证明",
  "outside the diff",
  "cross-module proof",
  "可观察行为/契约变化",
  "必须在分析前加载并阅读本 Skill",
];
const forbidden = ["重命名", "格式化", "注释订正", "pstack-how", "pstack-arena", "Serena", "codebase-memory"];
const failures = [];
for (const phrase of required) {
  if (!description.includes(phrase)) failures.push(`description 缺必需词组：${phrase}`);
}
for (const phrase of forbidden) {
  if (description.includes(phrase)) failures.push(`description 含禁止的负向反例或竞争能力名：${phrase}`);
}

const report = { mode: "pstack-blast-radius-description-contract", ok: failures.length === 0, failures };
console.log(JSON.stringify(report, null, 2));
process.exit(report.ok ? 0 : 1);
