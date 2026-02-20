# 走势分解定理二（Trend Decomposition Theorem II）

**状态**: verified
**来源**: `.chanlun/definitions/zoushi.md` v1.6 / 第17课

## 陈述

任何级别的任何走势类型，都至少由三段以上次级别走势类型构成。

形式化：

```
∀ level k, ∀ trend_type T at level k:
  ∃ sub-types [S_1, S_2, ..., S_m] at level k-1, m ≥ 3:
    T is composed of S_1, S_2, ..., S_m
```

## 证明

由中枢定义和走势类型定义推出：

1. 中枢 = 至少三个连续次级别走势类型的重叠部分（第17课定义）
2. 盘整 = 包含1个中枢 → 至少3段次级别走势类型
3. 趋势 = 包含≥2个中枢 → 至少3段次级别走势类型（第一个中枢3段 + 中枢间连接 + 第二个中枢的部分）

∴ 任何走势类型至少由3段次级别走势类型构成。

原文（第17课）："缠中说禅走势分解定理二：任何级别的任何走势类型，都至少由三段以上次级别走势类型构成。"

**验证依据**：级别递归实现（level_recursion.md v1.0）中 RecursiveStack 终止条件 = settled moves < 3。
