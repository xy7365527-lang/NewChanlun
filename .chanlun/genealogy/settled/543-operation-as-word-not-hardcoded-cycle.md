---
id: '543'
number: 543
title: "操作 = path（D∞ word）≠ 硬编码循环模式"
type: 概念分离
status: settled
date: 2026-06-17
level: "L0（自由幺半群 + 覆盖论，operation_route_exhaustion §1.1）"
负责工位: CC session（fugue_v3 D∞ word 处理器实装）
---

# 543 号：操作 = path（D∞ word）≠ 硬编码循环模式

## 概念分离

**被分离的两个概念**：

| 旧（被否定） | 新（结算） |
|------------|-----------|
| 操作 = 硬编码循环模式（CyclePhase 四步相位机：Holding→ShortActive→Recovering） | 操作 = **path（D∞ 的 word）**——自由幺半群 Σ\*={e,h⁺,h⁻,τ}\* 的生成元序列 |
| 四步循环（平多→开空→平空→做多）是引擎的固定结构 | 四步循环只是**多方视角的一个 word 例子**（还有空方 τ-共轭版、会计双重性）；引擎只有 h/τ 两原子，循环是**涌现序列** |
| 核心仓 H⁰（2/3）/ 机动仓 H¹（1/3）是 Layer 的硬编码字段 | 核心仓是**涌现**——每次 τ 只下沉 f=1/λ，顶层保留多数 ⟹ σ-塔自然分布（f∝λ⁻ᴷ） |

## 分离的依据

1. **operation_route_exhaustion §1.1（裁决用户命题）**：「操作路线=word 在 path（第0层）正确；四步循环不是单个群元素（0维），是 1-cycle（H₁）」。三层不可混：path（Σ\*，操作路线）/ group element（D∞，净效果）/ H₁（不变内容）。
2. **§2 定理 Σ-exh**：单 bar 单级别合法原子恰 4 个（e/h⁺/h⁻/τ），Σ\* 由 Σ 穷尽生成。硬编码四步相位 = 把 Σ\* 的一个特定 word 固化为引擎结构 = 混淆 path 与单一循环。
3. **§7.4 OP_* → Σ 原子映射**：OP_SHORT=τ@k−1 / OP_REBUY=h⁺∘σ / OP_FLIP=τ@φ=0——每个操作是 Σ 原子序列，由信号驱动，非状态机相位。

## 否定关系

- **否定 target**：fugue_v3 旧设计的 CyclePhase 四步相位机（硬编码循环）+ core_units/mobile_units 硬拆分字段。
- **by**：543（本号）。
- **驱动**：用户裁决 2026-06-17「操作不是不变量，操作是路径（D∞ 的 word）。四步循环只是一个 word 的例子」。

## 工程实现（落地证明）

fugue_v3 重写为 **D∞ word 处理器**（11 文件）：
- 引擎不预设循环模式，每 bar 每级别：nf_sell[k]→τ 下沉（σ⁻¹∘τ）/ nf_buy[k]→τ 升回（σ∘τ）/ 无信号→h。
- Layer{ladder, direction(ε), units(M), basis, entry_bar}——无 CyclePhase。
- 手性交替（相邻级别方向相反）⟹ Σ|units| Casimir 守恒（prove_chiral_alternation）。**【544号修正】** 此声明过强且为 non-sequitur——守恒由 |units| 转移独立保证（与方向无关）；相邻交替非不变量（建仓阶段同向合法），守卫已降级为 `count_chiral_violations` 观测。
- 验收：cargo build green + 单测 + 真实数据零 panic。**【544号修正】** 原"OKLO/CL 零 panic"为过早声明（090号声明膨胀）——OKLO/CL/ES/GC 实际 4/8 panic（相邻同向 Long）；经 544 号（chiral 降观测 + sink/recover 方向相容前提）修复后 8/8 零 panic，434 单测 green。

## 上游谱系

- 541号（spiral covering space）：D∞ 覆盖空间结构。
- 542号（spawn allocation σ-invariant）：f=1/λ 配额 σ-不变（本号的 H¹ 系数）。
- 540号（递归结构 identity/difference 双重性）：confirm 读法（向心 vs 当下合取）仍 open，本号 groupoid 轴原样继承不解决。
- 537号（BSP 信号 identity/difference 双重性）。

## 待结算开放轴（诚实标注，no-patch）

1. **recover 配额语义**：本实装 recover 拉 f·u_{k−1}（1/3 of 次级别），是否是用户要的"回收 1/3"——待编排者裁决。
2. **confirm 读法矛盾**（540号/escalation 2026-06-15）：向心读过去 vs 当下合取，本号未触及。
3. **评价判据哲学**（memory project_backtest_negativity_vs_pnl）：P1 跑赢 BH ≠ 缠论否定性本质——独立于本号。
