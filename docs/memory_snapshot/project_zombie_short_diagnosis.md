---
name: zombie-short-diagnosis
description: 僵尸空头三条路径(A根仓做空方向错/B高层sink回补难/C强平)，σ-ascend无关，伤害=regime函数
metadata:
  type: project
  originSessionId: cowork-2026-06-17
---

## 2026-06-17 僵尸空头产生机制追查

### σ-ascend无罪
σ-ascend只搬核心仓多头层（operate.rs:193-202），对空头零作用。僵尸空头不是被推高的。

### 三条产生路径
- **路径A**：核心仓直接在高层F做空（ES bar=870220做空价1404→涨到7368=+425%，-146K）
- **路径B**：高层sink子层空头，recover需父层nf_buy极稀疏（rL2以上recover率=0%）
- **路径C**：低层空头撞2×basis强平线（ES seg liq_short -118K）

### nf_buy级别衰减
move(3)≈5000次 → recL2(4)≈150次 → recL3(5)≈12次 → recL4(6)≈1次
每升一级少一个数量级。recover率：seg层CL93%/BTC85%，rL2及以上全标的0%。

### 伤害=regime函数
CL（震荡BH+28%）：short PnL +1477（正），同样结构但震荡中空头赚钱
ES（强牛BH+594%）：short PnL -257K（灾难），单边涨空头结构性亏损

### 关键洞察链（用户推动）
1. "笔的走势类型是线段" → 考证：第65课否定（严格意义上不对）→ PENDING_LO=3正确
2. "停泊层≠触发层" → 空头停segment，recover用父层move信号，不死锁
3. "σ-ascend不推空头" → 代码确认，空头高层是sink在高层直接产生的
4. "多头也不该升级？" → 不，核心仓跟随涌现级别是缠论的（清仓等高级别卖点）
5. "涌现级别不可能减小" → 对，无论涨跌，级别涌现单调

### 零操作参数裁决
engine.rs:44 断言 floor_ladder==FIRST_BSP_LADDER，引擎是"零操作参数引擎"。加sink_ceiling等参数违背裁决。

**Why:** 从"ES -418%为什么"追查到σ-ascend无罪→三条路径→regime函数
**How to apply:** 修复应从路径A入手（根仓做空方向判断），路径B/C是结构性的regime依赖

Related: [[emergent-direction]], [[session5-spiral-reinterpretation]]
