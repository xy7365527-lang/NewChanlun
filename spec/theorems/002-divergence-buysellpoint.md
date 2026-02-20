# 背驰-买卖点定理（Divergence-BSP Theorem）

**状态**: verified
**来源**: `.chanlun/definitions/beichi.md` v1.1 / 第24课

## 陈述

任一背驰都必然制造某级别的买卖点，任一级别的买卖点都必然源自某级别走势的背驰。

形式化：

```
∀ divergence D at level k:
  ∃ BSP at some level j: D produces BSP

∀ BSP at level k:
  ∃ divergence D at some level j: BSP is produced by D
```

即：背驰 ⟺ 买卖点（双向蕴含，充要条件）。

## 证明

原文（第24课 L18）直接给出充要关系声明。

推导链：
1. 背驰 = 力度衰竭 → 中枢回拉必然成功（第33课中枢对称回拉特性）→ 走势完成 → 产生买卖点
2. 买卖点 = 走势转折点 → 转折必源于力度衰竭（趋势转折定律）→ 某级别必然存在背驰

**注意**：充要关系中的"某级别"不一定是本级别。本级别走势完成不要求本级别背驰（可由"小转大"触发），但某个级别必然有背驰。

**验证依据**：beichi.md #3 已结算（充分非必要 at 本级别，充要 at 某级别）。
