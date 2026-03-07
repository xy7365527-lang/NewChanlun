/**
 * classifyNode.ts
 *
 * 将 TopologyNode 映射到域分类标签。
 * 分类逻辑基于节点 ID 前缀和 label 内容。
 */

import type { TopologyNode } from "../types";

export type DomainKey =
  | "philosophy"
  | "code"
  | "genealogy"
  | "chanlun"
  | "deepseek"
  | "mineru"
  | "system"
  | "other";

// 哲学家/哲学概念前缀列表
const PHILOSOPHY_PREFIXES = [
  "derrida_", "deleuze_", "nietzsche_", "foucault_",
  "hegel_", "lacan_", "marx_", "spinoza_", "schelling_",
  "wittgenstein_", "heidegger_", "mao_", "lenin_",
  "holderlin_", "simmel_", "merleau_", "postman_",
  "zizek_", "adorno_", "benjamin_",
  // 哲学概念节点
  "sense_", "perception_", "lord_", "unhappy_", "absolute_",
  "being_", "nothing_", "becoming_", "objet_", "jouissance_",
  "commodity_", "surplus_", "capital_", "bourgeois_", "proletariat_",
];

export function classifyNode(node: TopologyNode): DomainKey {
  const id = node.id;

  // 代码类：NewChanlun 代码 + topo-computation 自己的代码
  if (id.includes("newchanlun:") || id.includes("topo-self:")) return "code";

  // MinerU
  if (id.includes("MinerU:")) return "mineru";

  // DeepSeek（包含 deepseek-v3, deepseek-r1 等）
  if (id.toLowerCase().includes("deepseek")) return "deepseek";

  // synthesis 顶点
  if (id.startsWith("syn_") || id.startsWith("anti_")) return "system";

  // 哲学家/哲学概念
  if (PHILOSOPHY_PREFIXES.some((p) => id.startsWith(p))) return "philosophy";

  // phi_L 提取的文本节点（c_ 前缀）
  if (id.startsWith("c_")) return "chanlun";

  // 谱系区块（label 中包含 [event] 或 [tension]）
  if (node.label?.includes("[event]") || node.label?.includes("[tension]")) {
    return "genealogy";
  }

  return "other";
}

// ── 过滤器元数据（UI 展示用） ─────────────────────────────────────

export interface DomainFilterMeta {
  key: DomainKey;
  label: string;
  color: string;
}

export const DOMAIN_FILTERS: DomainFilterMeta[] = [
  { key: "philosophy", label: "哲学",    color: "#a855f7" },
  { key: "code",       label: "代码",    color: "#3b82f6" },
  { key: "genealogy",  label: "谱系",    color: "#f97316" },
  { key: "chanlun",    label: "缠论",    color: "#ef4444" },
  { key: "deepseek",   label: "DeepSeek", color: "#06b6d4" },
  { key: "mineru",     label: "MinerU",  color: "#22c55e" },
  { key: "system",     label: "合题",    color: "#6366f1" },
  { key: "other",      label: "其他",    color: "#6b7280" },
];
