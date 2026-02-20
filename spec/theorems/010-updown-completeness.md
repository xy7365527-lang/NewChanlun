# 升跌完备性定理（Up-Down Completeness Theorem）

**状态**: verified
**来源**: `.chanlun/definitions/maimai.md` v1.0 / 第21课

## 陈述

市场中的任何向上与下跌，都必然从三类缠中说禅买卖点中的某一类开始以及结束。换言之，市场走势完全由这样的线段构成，线段的端点是某级别三类缠中说禅买卖点中的某一类。

形式化：

```
∀ market_movement M (向上或向下):
  ∃ BSP_start, BSP_end:
    BSP_start.type ∈ {Type1, Type2, Type3} at some level
    BSP_end.type ∈ {Type1, Type2, Type3} at some level
    M starts at BSP_start and ends at BSP_end
```

## 证明

由走势分解定理一 + 买卖点完备性定理 + 趋势转折定律推出：

1. 走势分解定理一：任何走势 = 盘整/上涨/下跌的连接
2. 每个走势类型的起止点 = 转折点
3. 趋势转折定律：任何转折由某级别第一类买卖点构成
4. 买卖点定律一：第二类买卖点由次级别第一类买卖点构成
5. ∴ 所有转折点 = 某级别某类买卖点
6. ∴ 所有市场运动的起止点 = 某级别某类买卖点

**验证依据**：maimai.md v1.0 已结算。本定理是买卖点完备性定理的直接推论。
