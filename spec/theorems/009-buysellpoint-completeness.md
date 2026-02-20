# 买卖点完备性定理（BSP Completeness Theorem）

**状态**: verified
**来源**: `.chanlun/definitions/maimai.md` v1.0 / 第21课

## 陈述

市场必然产生赢利的买卖点，只有第一、二、三类。

形式化：

```
∀ profitable BSP in market:
  BSP.type ∈ {Type1, Type2, Type3}

¬∃ profitable BSP: BSP.type ∉ {Type1, Type2, Type3}
```

三类买卖点定义：
- **Type1（第一类）**：趋势背驰点（下跌趋势背驰=买点，上涨趋势背驰=卖点）
- **Type2（第二类）**：第一类买卖点后第一次次级别回调/反弹的结束点
- **Type3（第三类）**：次级别离开中枢后回试不破 ZG/ZD

互斥/重合关系：
- Type1 与 Type2：不可能重合
- Type1 与 Type3：不可能重合
- Type2 与 Type3：可以重合（V型反转情形）

## 证明

原文（第21课）直接给出完备性声明。

推导链：
1. 走势终完美 → 任何走势类型终将完成
2. 走势完成 → 必然产生转折 → 转折点即买卖点
3. 转折的来源只有三种：趋势背驰（Type1）、回调确认（Type2）、中枢突破（Type3）
4. 三种来源穷尽了所有走势结构上可能的转折机制

**验证依据**：maimai.md v1.0 已结算，5/5 未结算问题全部解决，43测试全通过。
