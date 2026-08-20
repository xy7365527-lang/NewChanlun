#!/usr/bin/env node
// #1137 确定性契约锁：只读 fixture 与文档，不调用 rlm/prime-agent/真实模型。
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const fixture = JSON.parse(readFileSync(join(root, "docs/agents/pstack-lite/adversarial-gate-fixtures.json"), "utf8"));
const mode = readFileSync(join(root, ".agents/skills/code-review/ADVERSARIAL-MODE.md"), "utf8");
const skill = readFileSync(join(root, ".agents/skills/code-review/SKILL.md"), "utf8");

const retryForResult = (reviewer) =>
  `${reviewer} 能经 agent_message、session-dir 结果文件或最终 JSONL 返回可解析真实 child 产物后重试`;

const aggregateProviders = new Set(["prime-inference"]);

// 直连 provider 自己就是模型所有者；聚合 provider 按其承载的基础模型所有者分桶。
const ownerBucket = ({ provider, base_family }) =>
  aggregateProviders.has(provider) ? base_family : provider;

function decide(c) {
  const candidates = c.candidates ?? [];
  const uniqueBySelector = new Map();
  let conflictingMetadata = false;
  const malformed = candidates.some(({ selector, provider, base_family }) =>
    !selector || !provider || !base_family);

  // 完全相同的 selector + metadata 只计一次；同 selector 的 metadata 冲突则不可准入。
  for (const candidate of candidates) {
    const previous = uniqueBySelector.get(candidate.selector);
    if (!previous) {
      uniqueBySelector.set(candidate.selector, candidate);
    } else if (previous.provider !== candidate.provider ||
               previous.base_family !== candidate.base_family) {
      conflictingMetadata = true;
    }
  }
  const uniqueCandidates = [...uniqueBySelector.values()];
  const ownerBuckets = new Set(uniqueCandidates.map(ownerBucket));

  // 发现集必须提供至少三个 selector、两个基础模型所有者桶。
  if (malformed || conflictingMetadata || uniqueCandidates.length < 3 || ownerBuckets.size < 2) {
    return { status: "unavailable" };
  }

  // 只评估实际选中的三名 reviewer；未选候选不应被误判为启动失败。
  const launchItems = c.launches ?? [];
  const selectedSelectors = new Set(launchItems.map(({ selector }) => selector));
  const selectedCandidates = launchItems.map(({ selector }) => uniqueBySelector.get(selector));
  const selectedOwnerBuckets = new Set(
    selectedCandidates.filter(Boolean).map(ownerBucket),
  );
  if (launchItems.length !== 3 || selectedSelectors.size !== 3 ||
      selectedCandidates.some((candidate) => !candidate) || selectedOwnerBuckets.size < 2) {
    return { status: "unavailable" };
  }

  const launches = new Map(launchItems.map((item) => [item.selector, item]));
  for (const candidate of selectedCandidates) {
    const item = launches.get(candidate.selector);
    if (!item?.ok) {
      const reviewer = item?.reviewer ?? `未分配 reviewer（${candidate.selector}）`;
      return {
        status: "degraded",
        detail: {
          reviewer,
          selector: candidate.selector,
          stage: "launch",
          channel: "rlm admission",
          retry_when: `${reviewer} 的 ${candidate.selector} 可成功启动后重试`,
        },
      };
    }
  }

  const results = new Map((c.results ?? []).map((item) => [item.selector, item]));
  for (const candidate of selectedCandidates) {
    const launch = launches.get(candidate.selector);
    const item = results.get(candidate.selector);
    if (!item?.ok || item.reviewer !== launch.reviewer) {
      const reviewer = item?.reviewer ?? launch.reviewer ?? `未知 reviewer（${candidate.selector}）`;
      const channel = item?.channel ?? "agent_message/session-dir/final-jsonl（均未回流）";
      return {
        status: "degraded",
        detail: {
          reviewer,
          selector: candidate.selector,
          stage: "result",
          channel,
          retry_when: retryForResult(reviewer),
        },
      };
    }
  }
  return { status: "adversarial" };
}

const failures = [];
for (const c of fixture.cases) {
  const actual = decide(c);
  if (actual.status !== c.expect) {
    failures.push(`${c.id}: expect=${c.expect} actual=${actual.status}`);
  }
  if (c.expect_detail && JSON.stringify(actual.detail) !== JSON.stringify(c.expect_detail)) {
    failures.push(`${c.id}: degraded detail mismatch: ${JSON.stringify(actual.detail)}`);
  }
}
for (const token of [
  "await rlm.find_models(limit=8)", "至少 3 个不同、可启动的 selector", "至少 2 个 vendor",
  "三个 OpenAI selector 仍只有一个桶", "重复 selector",
  "Adversarial status: unavailable", "Adversarial status: degraded", "标准 Standards/Spec 双轴",
  "显式请求不豁免", "具体 reviewer", "尝试过的回流通道", "稍后重试条件", "真实 child 产物",
  "session_dir", "最终 JSONL", "admission handle 不是结果",
]) {
  if (!mode.includes(token) && !skill.includes(token)) failures.push(`文档缺契约：${token}`);
}
if (!/only when its model-diversity gate is satisfied/.test(skill.split("---")[1] ?? "")) {
  failures.push("description 未声明多样性降级门");
}
const report = { ok: failures.length === 0, cases: fixture.cases.length, failures };
console.log(JSON.stringify(report, null, 2));
process.exit(report.ok ? 0 : 1);
