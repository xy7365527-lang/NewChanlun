# 中心定理一（Center Theorem I / Extension Theorem）

**状态**: verified
**来源**: `.chanlun/definitions/zhongshu.md` v1.3 / 第20课

## 陈述

走势中枢的延伸等价于任意区间 [dn, gn] 与 [ZD, ZG] 有重叠。换言之，若有 Zn，使得 dn > ZG 或 gn < ZD，则必然产生高级别的走势中枢或趋势及延续。

形式化：

```
设中枢 C 的区间为 [ZD, ZG]，后续 Z走势段 Zn 的区间为 [dn, gn]。

延伸 ⟺ [dn, gn] ∩ [ZD, ZG] ≠ ∅
       ⟺ dn ≤ ZG ∧ gn ≥ ZD

中枢终结 ⟺ ∃ Zn: dn > ZG ∨ gn < ZD
```

## 证明

原文（第20课）直接给出充要条件。

推导：
1. 中枢区间 [ZD, ZG] 由前三段确定后固定不变
2. 后续段与中枢有重叠 → 仍在中枢影响范围内 → 延伸
3. 后续段与中枢无重叠 → 脱离中枢 → 必然产生新中枢或趋势延续

**注意**：延伸判定使用弱不等式（≤/≥），突破判定使用严格不等式（>/<）。zhongshu.md v1.1 审计已修复此边界条件。

**验证依据**：zhongshu v1.3 审计通过，5个边界条件测试（TestExtensionBoundaryTouch），36/36全绿。
