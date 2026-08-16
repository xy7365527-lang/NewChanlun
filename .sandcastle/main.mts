import { run } from "@ai-hero/sandcastle";
import { docker } from "@ai-hero/sandcastle/sandboxes/docker";
import { primeAgent } from "./prime-agent-provider.ts";

// Simple loop（prime-agent 版，#996）。跑法：npx tsx .sandcastle/main.mts
// 冒烟期 maxIterations=1；正式使用按需调大，并把 prompt.md 换成正式版。

await run({
  name: "worker",
  sandbox: docker(),
  agent: primeAgent("k3", { provider: "kimi-coding" }),
  promptFile: "./.sandcastle/prompt.md",
  maxIterations: 1,
  branchStrategy: { type: "merge-to-head" },
});