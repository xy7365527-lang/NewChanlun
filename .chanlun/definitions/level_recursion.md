# 级别递归（Level Recursion / 走势级别的递归构造）

**版本**: v1.0
**状态**: 已结算
**创建日期**: 2026-02-16
**结算日期**: 2026-02-17
**溯源**: [旧缠论] 第12课、第17课、第20课

---

## 核心定义

### 级别 = 递归层级（level_id）

**[旧缠论]** 级别不是时间周期（日线/30分/5分），而是走势结构**自下而上递归构造**的层级编号。

> "级别在本 ID 的理论中有着严格的定义，是不能随意僭越的。" — 第12课

**形式化**：

```
level_id : ℕ （自然数，从 0 开始）

Move[0]    = Segment     （线段，归纳基底）
Center[k]  = 三个连续 Move[k-1] 的价格区间重叠部分
Move[k]    = 包含 Center[k] 的走势类型实例（盘整=1中枢，趋势≥2中枢）
```

### 归纳基底：线段不依赖中枢

**[旧缠论]** 递归链的关键在于**线段的自足性**：

```
1分K线（原子）
  ↓ 包含处理（baohan v1.3）
合并K线
  ↓ 分型识别（fenxing v1.0）
顶底分型
  ↓ 笔划分（bi v1.3）
笔（Stroke）
  ↓ 特征序列法（xianduan v1.2）
线段（Segment）= Move[0]
```

线段的构造链中**不涉及中枢定义**，因此 Move[0] 是自足的归纳基底，打破了"中枢需要走势类型、走势类型需要中枢"的看似循环依赖。

### 递归构造链

```
Move[0] = Segment（已有：a_segment_v1.py）
  ↓ 三段重叠
Center[1]（已有：a_zhongshu_v1.py，输入为 Segment）
  ↓ 中枢组合
Move[1] = 1级走势类型（已有：a_move_v1.py，输入为 Zhongshu）
  ↓ 三个 Move[1] 重叠
Center[2] = 2级中枢（❌ 未实现：需要 zhongshu_from_moves）
  ↓ 中枢组合
Move[2] = 2级走势类型（❌ 未实现：需要 moves_from_zhongshus 接受 Move[1]）
  ↓ ...
```

### 递归终止

**[旧缠论]** 当某层 `len(Move[k]) < 3` 时，无法再构造更高级别中枢，递归自然终止。

> "在实际之中，对最后不能分解的级别，其缠中说禅走势中枢就不能用'至少三个连续次级别走势类型所重叠'定义，而定义为至少三个该级别单位K线重叠部分。" — 第17课

---

## 核心概念：次级别走势类型

**[旧缠论]** 中枢的组件必须是**次级别走势类型**（Move[k-1]），不是任意结构。

> "某级别走势类型中，被至少三个连续次级别走势类型所重叠的部分。" — 第17课

**"次级别走势类型"的完成条件**：

一个 Move[k-1] 被视为"完成"的条件：
1. **盘整型 Move[k-1]**：包含至少 1 个 Center[k-1]，且已被后续走势终结（第三类买卖点或新中枢生成）
2. **趋势型 Move[k-1]**：包含至少 2 个 Center[k-1]，且最后一段出现背驰

**当前实现状态**：
- 1级中枢（Center[1]）：组件是 Segment = Move[0] ✅ 正确
- 2级及以上中枢：**未实现**，需要 Move[k-1] 完成判定 + 三段重叠

---

## 两种"级别"口径的区分

### 口径 A：递归级别（recursive_level）[旧缠论]

- **定义**：从 1 分钟 K 线出发，自下而上递归构造
- **level_id 来源**：递归层级 k（0, 1, 2, ...）
- **跨级别关系**：高级别**由**低级别构造，存在严格的包含关系
- **原文对应**：第12课、第17课、第20课的"级别"定义
- **当前实现**：`a_recursive_engine.py`（批处理版本，非事件驱动）

### 口径 B：时间周期级别（tf_level）[旧缠论:选择]

- **定义**：为每个时间周期（5m, 30m, 1h, ...）独立运行完整管线
- **level_id 来源**：TF 在列表中的索引 + 1
- **跨级别关系**：各 TF 独立，互不依赖
- **实战价值**：可以同时监控多个周期，对比信号
- **当前实现**：`TFOrchestrator`（事件驱动，五层引擎链）

### 两者的关系

```
口径 A（递归级别）≠ 口径 B（TF 级别）

口径 A：1分K线 → 笔 → 线段 → 1级走势 → 2级走势 → ...
        （单一数据源，递归向上构造）

口径 B：5mK线 → 笔 → 线段 → 中枢 → 走势 → 买卖点
        30mK线 → 笔 → 线段 → 中枢 → 走势 → 买卖点
        1hK线  → 笔 → 线段 → 中枢 → 走势 → 买卖点
        （多个独立数据源，各自完整管线）
```

**本项目的口径选择（编排者决断 2026-02-16）**：

- **口径 A（递归级别）是唯一正式路径**。所有定义文件中"级别"一词均指递归级别。
- **口径 B（多周期独立管线）不在本项目中使用**，除非能找到多周期配合自下而上递归的方式。
- TFOrchestrator 保留为工程参考/调试工具，但不作为核心引擎路径。
- **理由**：多周期下钻本质上是信息丢失（高 TF 聚合了低 TF 的细节），与递归构造从最低级别保留全部信息的思路相矛盾。

---

## "真中枢"与"假中枢"

### 定义 [新缠论]

本项目引入"真中枢"和"假中枢"概念，用于区分递归构造的严格性：

**真中枢（Authentic Center）**：
- 组件是三个**已完成的**次级别走势类型（Move[k-1]）
- 每个 Move[k-1] 自身包含至少一个 Center[k-1]
- 递归链完整：从 Move[0]（Segment）逐级向上构造

**假中枢（Apparent Center）**：
- 在较高时间周期图表上**看起来像**中枢（三段重叠）
- 但组件未经递归验证——不确认其是否为完成的次级别走势类型
- 典型来源：TFOrchestrator 口径 B 产出的中枢

### 对应关系

| 属性 | 真中枢 | 假中枢 |
|------|--------|--------|
| 组件类型 | Move[k-1]（已完成的次级别走势类型） | 三段重叠但组件未经递归验证 |
| 递归链 | 完整可追溯至 Move[0] | 断链（组件级别归属不确定） |
| 有效性 | 100%（原文定义保证） | 不确定（可能有效但无法证明） |
| 买卖点定理 | 完全适用 | 不保证适用 |

### 在纯递归口径下的意义

由于本项目不使用多周期方案（编排者决断），"假中枢"的来源不再是"TF 口径产出"，而是：
1. **递归链未完成时的候选中枢**：低级别走势类型尚未 settled，导致高级别中枢是"暂态"的
2. **组件完成判定不严格**：用 Segment 直接构造中枢时，未验证 Segment 是否构成完整的次级别走势类型

因此"真/假"的区分在于**递归链的完整性和组件完成状态**，而非来源口径的差异。

---

## 区间套与级别递归的关系

**[旧缠论]** 区间套（第27课）是级别递归的直接应用：

```
在 k 级别找到背驰段 → 范围 [a, b]
在 k-1 级别的 [a, b] 内找背驰段 → 范围 [a', b'] ⊂ [a, b]
...
在最低级别找到精确转折点
```

区间套依赖**多级别同时存在**的递归构造结果。当前实现（beichi #5）被阻塞在级别递归未实现。

---

## 当前实现状态

| 组件 | 状态 | 文件 | 说明 |
|------|------|------|------|
| Move[0] = Segment | ✅ 完整 | a_segment_v1.py | 特征序列法 |
| Center[1] | ✅ 完整 | a_zhongshu_v1.py | 三段重叠（输入=Segment） |
| Move[1] | ✅ 完整 | a_move_v1.py | 盘整/趋势分类 |
| Center[k≥2] | ✅ 泛化接口已实现 | a_zhongshu_level.py | `zhongshu_from_components(MoveProtocol列表)` |
| Move[k≥2] | ✅ 泛化接口已实现 | a_zhongshu_level.py | `moves_from_level_zhongshus(LevelZhongshu列表)` |
| MoveProtocol | ✅ 已实现 | a_level_protocol.py | Protocol + SegmentAsComponent + MoveAsComponent |
| 递归调度（批处理） | ⚠️ 有雏形 | a_recursive_engine.py | 类型混用（duck typing） |
| 递归调度（事件驱动） | ✅ P4完成 | recursive_level_engine.py + recursive_level_state.py | RecursiveLevelEngine（全量重算+diff，21个测试全GREEN） |
| 递归栈（多层自动调度） | ✅ P5完成 | recursive_stack.py | RecursiveStack（懒创建引擎，max_levels=6，终止条件=moves<3，16测试全GREEN） |
| 事件level_id扩展 | ✅ P6完成 | events.py + bus.py + recursive_level_state.py | 6事件类+level_id:int=1，EventBus push_level/drain_by_level，25测试全GREEN |
| 口径A编排器 | ✅ P8完成 | orchestrator/recursive.py | RecursiveOrchestrator.process_bar() 五引擎链+RecursiveStack，9测试全GREEN |
| 口径A交叉验证 | ✅ P9完成 | test_p9_cross_validation.py | 编排器 vs 手动管线链 level=1 一致性，7测试全GREEN |

---

## 未结算问题

### ~~1. Move 的统一接口~~ → ✅ 已实现（P1-P3）

- **解决方案**：`MoveProtocol`（Protocol，结构化子类型） + `SegmentAsComponent` / `MoveAsComponent` 适配器
- **代码**：`a_level_protocol.py`（Protocol + 适配器）、`a_zhongshu_level.py`（LevelZhongshu + zhongshu_from_components + moves_from_level_zhongshus）
- **测试**：22个测试全GREEN（10 protocol + 12 zhongshu_level），含 zhongshu_from_components 与 zhongshu_from_segments 交叉验证
- **设计规范**：`docs/spec/level_recursion_interface_v1.md`

### ~~2. Move[k-1] 完成判定~~ → ✅ 已结算（Option B: settled 标记）

**结算结论** [旧缠论:选择]：采用 Option B — `Move.settled = True` 作为递归构造的组件完成判定。

**理由**：
1. beichi #3 已结算：背驰→走势完成是充分非必要条件，走势可由第三类买卖点或"小转大"终结而无本级背驰
2. Option A（要求背驰才完成）会将结构层与动力学层紧耦合，且与"小转大"终结机制矛盾
3. 分离关注点：`Move.settled` 是结构标记（新实例产生时前一个自动结算），背驰检测是独立管道
4. P9 交叉验证已确认 `RecursiveOrchestrator` 使用 settled 过滤后与手动管线一致
5. `nested_divergence_search` 的递归下降映射也基于 settled 过滤，20测试GREEN

**边界条件**：settled=False 的最后一个 Move 不参与高级别递归构造。这意味着实时数据中最高级别总是不完整的（正确行为：未完成的走势不应影响更高级别判断）

### ~~3. 递归深度的实际限制~~ → ✅ 已结算（可配参数）

**结算结论**：`RecursiveStack.max_levels`（默认6）作为可配置参数已实现。

- **代码**：`recursive_stack.py :: RecursiveStack.__init__(max_levels=6)`
- **终止条件**：`RecursiveStack` 在 settled moves < 3 时自动停止创建新层级（自然终止）
- **估算验证**：1分钟 K 线 240根/天，10年 ≈ 600K 根 → 实际递归深度 4-5 层符合预期
- **设计原则**：`max_levels` 是安全上限，实际深度由数据自然决定（对象否定对象，非人为截断）[新缠论]

### ~~4. 递归级别与 TF 级别的映射~~ → ✅ 已结算（工程债，非概念阻塞）

- **问题**：用户习惯用 TF 描述级别（"30分钟级别的中枢"），但严格定义下级别 ≠ TF
- **方案**：维护近似映射表（如 L1≈5min, L2≈30min），仅用于展示，不用于计算
- **原文依据**："级别在本ID的理论中有着严格的定义，是不能随意僭越的"
- **结算结论**：TF-level 映射是纯展示层需求，核心引擎（`RecursiveLevelEngine`）仅接受 `level_id: int`，不使用 TF 参数。当且仅当 UI 展示层需要"用户友好的级别描述"时实现。不构成定义结算阻塞。

### ~~5. 事件驱动递归引擎的设计~~ → ✅ 已实现（P4）

- **解决方案**：`RecursiveLevelEngine`（事件驱动，全量重算+diff）
- **代码**：`recursive_level_engine.py`（引擎）+ `recursive_level_state.py`（快照+diff）
- **核心流程**：settled Move → adapt_moves → zhongshu_from_components → diff → moves_from_level_zhongshus → diff
- **测试**：21个测试全GREEN（引擎基本、中枢形成、走势形成、增量处理、diff直接测试、settled过滤、reset）
- **level_id语义**：引擎level_id=k → 消费 Move[k-1] → 产出 Center[k] + Move[k]

---

## 谱系关联

- **前置**：zhongshu.md v1.2（中枢定义，组件="至少三个连续次级别走势类型"）
- **前置**：zoushi.md v1.1（走势类型定义，盘整/趋势分类）
- **前置**：005-object-negation-principle.md（对象否定对象——递归是对象产生对象的核心机制）
- **已解除**：~~beichi.md v1.1 #5 区间套（已结算）~~ → ✅ beichi v1.1 已结算（nested_divergence_search 使用 RecursiveStack 递归结构）
- **相关**：003-segment-concept-separation.md（Move[0] = Segment v1 为唯一口径）

---

## 变更历史

- 2026-02-16: v0.1 初始版本，基于原文第12/17/20课 + 现有代码研究 + 编排者启动信号
- 2026-02-16: v0.2 P1-P3 泛化接口实现，P4设计规范
- 2026-02-16: v0.3 P4 RecursiveLevelEngine 事件驱动引擎实现（#1/#5已结算）
- 2026-02-16: v0.4 P5 RecursiveStack 多层自动递归栈实现 + 五引擎 reset() 突变 bug 修复
- 2026-02-16: v0.5 P6 事件level_id扩展（6事件类+level_id，EventBus级别路由，25测试全GREEN）
- 2026-02-16: v0.6 P8 RecursiveOrchestrator口径A编排器 + P9交叉验证（编排器vs手动链level=1一致性，7测试全GREEN）
- 2026-02-16: v0.2 P1-P3已实现：MoveProtocol + 适配器 + zhongshu_from_components + moves_from_level_zhongshus，22测试全GREEN
- 2026-02-16: v0.7 #2 已结算（settled标记=递归完成判定，分离结构层与动力学层）；#3 已结算（max_levels可配参数+自然终止）；beichi阻塞已解除
- 2026-02-17: v1.0 /ritual 结算仪式——生成态→已结算。#4 TF映射标记为工程债（纯展示层需求，核心引擎不依赖）。5/5问题全部已结算。106测试全GREEN，递归引擎核心覆盖率100%。依据元编排原则#6自动结算
