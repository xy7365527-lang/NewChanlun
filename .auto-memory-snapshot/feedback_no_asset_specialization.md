---
name: no-asset-specialization
description: 缠论必须在任何标的任何市场下统一适用，per-asset配置=过拟合风险，regime切换必须从走势结构涌现
metadata:
  type: feedback
  originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---

用户2026-06-12明确表述："缠论应该在任何情形下的市场都适用，否则会有过拟合的风险。它不能被标的所特化。"

当前系统的三域三配置矩阵（趋势域fusion_t / 震荡域ht / BH域）本质是per-asset特化——虽然每个配置有原文课号，但"哪个标的用哪个配置"是回测盈亏拟合，无原文依据。

**Why:** 缠论第一性原理：走势完全分类（趋势/盘整），任何级别操作遵循同一套买卖点规则。不应存在per-asset配置选择。49课二相的切换是走势自身决定的，不是人决定的。

**How to apply:** 下一个session的最高优先级=找到统一配置，regime切换完全由走势结构驱动（运行时内生涌现）。不继续优化各域收益，回到根上。所有"白名单"类判决（osc域、fusion_t域、H1/H3域）都是形式化不到位的信号，不是最终形态。

Related: [[session3-fusion]], [[recursive-nested-fugue]]
