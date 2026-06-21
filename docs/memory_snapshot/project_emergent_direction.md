---
name: emergent-direction
description: Root方向由涌现最高级别决定，四步循环是跨级别+会计双重性，F建仓ε=+1硬编码是D∞截断，已实装ε对称但需regime门控
metadata:
  type: project
  originSessionId: cowork-2026-06-17
---

## 2026-06-17 级别涌现决定Root方向

### 核心洞察链（用户推动）

1. **四步循环是跨级别的**：本级别卖点→平多 / 次级别做空 / 次级别买点→平空 / 本级别买点→做多。每步有会计双重性（一个级别的了结=另一级别的开始）。

2. **同级别只有三类买卖点**：1买/2买/3买或1卖/2卖/3卖。没有同级别"翻转"操作。

3. **走势类型=上一级别的一笔**：级别递归定义。root方向的翻转→联系到上一级别。

4. **最大级别不是选的，是涌现的**：从1s/1min笔自下而上逐层涌现。Root方向=已涌现最高级别走势方向。

5. **根voice不应该存在**：没有预设root，方向从涌现结构中读取。

### ε对称性实装结果（2026-06-17）

F建仓从硬编码Long改为双向（buy_source→Long, sell_source→Short），结果：
- OKLO +116%→+495%（大幅改善，跑赢BH）
- ES +594%→-213%（崩了——Short root在强牛市做空）
- GC +219%→-122%（同上）
- 其余也变差

**诊断**：sell_source fire不等于"翻空"。正确的逻辑：sell_source=本级别走势结束=平多，下一步做什么取决于涌现的新走势方向，不是机械翻转。

### 架构改动

- docs/emergent_level_direction.md（形式化文档）
- axis.rs: root_direction() 从形态学轴读取
- operate.rs: prove_f_source_direction 守卫
- 形态学轴的真正级别涌现实装仍pending

**Why:** 解决了"root方向从哪里来"的根本问题——不是硬编码也不是信号翻转，是级别涌现的副产品
**How to apply:** MorphologyAxis需要实装真正的走势类型涌现（笔→段→走势类型递归），输出级别+方向

Related: [[session5-spiral-reinterpretation]], [[bidirectional-always-in]], [[orbit-enumeration-duality]]
