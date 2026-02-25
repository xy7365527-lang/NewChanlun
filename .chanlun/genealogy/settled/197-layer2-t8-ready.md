---
id: '197'
title: 纲举目张自动检测——Layer 2 T8 就绪
type: 概念发现
status: 已结算
date: 2026-02-25
source: gangju_analysis.py
audit_targets:
  - 'Layer 2 T8 就绪'
settlement_evidence:
  - 'T8 实现: src/newchan/a_topology.py (check_divergence_topology)'
  - 'Gemini Round 2 APPROVED: .chanlun/review-results/plan-review-layer2-gemini-round2.md'
  - 'Codex Round 2 APPROVED: .chanlun/review-results/plan-review-layer2-codex-round2.md'
  - '1938 测试全通过'
---

# 197号：纲举目张自动检测——Layer 2 T8 就绪

## 来源标注

[gangju_analysis.py 自动生成] audit_needed=true, new_mu=1 项

## 待质询目标

### Layer 2 T8 就绪

**触发原因**: Layer 1 审核共识 + gauge 经验验证均 filled → T8 背驰拓扑化可启动

**质询结果**: T8 已实现并通过 Gemini×Codex 2轮严格审核。

实现内容：
- T8Result dataclass + 4 辅助函数 + check_divergence_topology 主函数
- 背驰 ⇔ W₁(Dgm(C)) ≤ W₁(Dgm(A)) - η（Wasserstein-1 带容差单调下降）
- inconclusive 判定：任一侧无中枢 → 数据不足（Gemini×Codex 共识：理解B）
- gauge_equivalence_report 增强：t7_recursive_order + t8_divergence_topology

审核历程：
- Round 1: NOT APPROVED（inconclusive 判定 and→or + 索引空间一致性）
- Round 2: 双方 APPROVED

## 边界条件

- T8 是后验验证层，不替代现有 MACD 三维度 OR 判定
- 任一侧无中枢时标记 inconclusive（非 passed/failed）
- 仿射规范化 span ≤ 0 时退化为原始条形码
- 级别 > 0 时索引空间需保持一致（已修复）

## 下游推论

- Layer 2 完成 → 纲举目张体系中 T8 从"就绪"变为"已实现"
- 下一阶段：Layer 3 (T6 Leray) 或真实市场数据验证（195号-5 长期项）
