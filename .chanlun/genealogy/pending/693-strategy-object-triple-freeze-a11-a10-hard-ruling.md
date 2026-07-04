---
id: 693
title: 策略对象三分冻结 + A11/A10 硬裁决——待编排者 /ritual 追认
status: pending
type: 概念层裁定（escalate 点）
source_task: goal g-20260704T1756Z-strategy-object-freeze-cbc36d / acc-target-freeze
depends_on: [685, 690]
related: [684, 692, 696]
date: 2026-07-04
---

# 693号：策略对象三分冻结 + A11/A10 硬裁决

## 从哪个矛盾中来

问题1.pdf（16页《推导完全分类》）裁定的核心矛盾：**被检验的策略对象在「裁剪版」与
「完整体」之间漂移**——负结果被解释为「实现没接全」，正结果被怀疑为「口径伪影/beta/
过拟合」，循环无出口。根因是 \(\Pi_{tested} \subsetneq \Pi_{max\text{-}full}\) 且
无人冻结 \(\Pi_{target}\)：A11（声部执行层）处于「诊断层完成但完整策略又暗含它」的
模糊态，A10（保证金/强平/资金费）处于长期 waiver 态，两者都既不算实装完成也不算
显式出域。

## 否定了什么旧口径

1. 否定「A10 = WAIVED（外部数据无所谓）」的长期豁免态——PDF §5 第3条：任何带杠杆、
   多空双开、期货/加密环境都不能长期 waiver；要么 MUST 实装，要么把目标对象改写为
   cash-settled no-margin netting strategy（此时 A10 属定义外，非豁免）。
2. 否定「A11 诊断层完成」可以支撑「多重赋格/多空双开策略已回测」的声称——诊断层
   不改变真实交易时，\(\Pi_{tested} = \Pi_{signal+net\text{-}exec} \ne \Pi_{full\text{-}voice\text{-}exec}\)。
3. 否定一切不带对象声明的 alpha 结论——没有 TARGET_STRATEGY.md 时所有 alpha 结论
   不应接受（PDF §5 第1条）。

## 新裁定内容

三对象冻结（TARGET_STRATEGY.md §1）：\(\Pi_{signal\text{-}full}\) /
\(\Pi_{exec\text{-}full}\) / \(\Pi_{treasury\text{-}full}\)，边界互斥，
负结论互不外推。

本轮 \(\Pi_{target} = \Pi_{signal\text{-}full}\)（PDF 路线A）。

A11/A10 硬裁决（无模糊态）：

| 模块 | Π_signal-full | Π_exec-full | Π_treasury-full |
|------|--------------|-------------|-----------------|
| A11 | OUT_OF_SCOPE | MUST（后续 goal 优先级 #1） | MUST |
| A10 | OUT_OF_SCOPE（定义外，非豁免） | MUST（优先级 #2） | MUST |

## 分离的逻辑必然性

若不冻结对象：设未接模块 \(M^\star\)（如声部执行层）仅在状态子集产正收益
\(E[R(M^\star)] > 0\)，而 \(\Pi_{tested}\) 不调用 \(M^\star\)，则
\(E[R(\Pi_{tested})] \le 0 \not\Rightarrow E[R(\Pi_{full})] \le 0\)（有效域定理1，
docs/formal-chain/有效域定理-20260704.md）。即一切「测过=否证」的推理在
\(\Pi_{tested} \subsetneq \Pi_{full}\) 下逻辑无效。冻结不是管理偏好，而是让
「负结论」这个词有确定语义的必要条件。三分不是任意划分：signal/exec/treasury
分别对应 estimand 三类（方向 μ / 净值 R / 可达性 Reach），合格判据互不蕴含，
故必须三对象而非一对象带开关。

## 结算状态

**概念层裁定——待编排者 /ritual 追认的 escalate 点。**
追认前 TARGET_STRATEGY.md 即为工作口径（goal 授权下的无模糊标注）；
若编排者翻转任一格，按 /ritual 基底刷新流程将受影响产出退回生成态。

## 翻转条件

1. 编排者裁定「多重赋格/短差是策略核心，本轮就必须检验」→ A11 对本轮翻 MUST，
   Π_target 改 Π_exec-full，本轮 alpha 复验作废重跑。
2. 编排者裁定信号层检验必须带杠杆环境 → A10 翻 MUST，K 层重定义。

## settle-sweep 增量扫描交叉引用注记（2026-07-04，三路线对照）

`formal-chain-deepresearch-20260704.md` §4「三条路线落地对照表」是本条目三分冻结
（Π_signal-full/Π_exec-full/Π_treasury-full）的**直接下游消费者**——该报告把本条目
的三对象逐一映射为「路线A/B/C」并给出各自的现状与剩余量级：

- **路线A（Π_signal-full，本条目"本轮 Π_target"）**：P1 区间套/P2 ForceState/P4 六类
  买卖点/P7 typed exit 已收口；(i) d 入状态/μ_R estimand（见 `696` 号，estimand 选择
  待独立 /escalate）、(iii) δ-free 在线主裁决、(iv) AncOK/身份彻底修复（见 `697` 号，
  persistent.rs ceiling 待独立 /escalate）三项为剩余。四项互相独立可并行，"本轮可闭合"。
- **路线B（Π_exec-full）**：A11 声部执行层接真实开平仓（本条目已裁 OUT_OF_SCOPE→MUST，
  优先级#1）+ A10 保证金/强平/资金费（本条目已裁 OUT_OF_SCOPE→MUST，优先级#2）——
  均为"大（架构级新增），未启动"，依赖序在路线A之后。**本条目的 A11/A10 硬裁决表**与
  路线B剩余项**逐字对应**，未见冲突。
- **路线C（Π_treasury-full/GAP3）**：Stage II/III 可达性——L1 已可达（`runner.rs:2439`
  利润桥 assert 通过，**推翻**「架构不可达」旧归因，与 project_gap3_l2_unreachable_architecture
  memory 记录的旧结论不一致，须按本条目最新核实订正该 memory 归因为过期）；终态判据待
  κ 外部风险政策裁定（选择类）。

**交叉核对结论**：deep-research §4 对三路线的现状描述与本条目的三分冻结表/A11-A10
硬裁决表**完全一致**，未发现张力——该报告是本条目冻结表在实装层的**首次系统性落地
盘点**，可视为本条目"追认前 TARGET_STRATEGY.md 即为工作口径"这一过渡状态的具体
执行进度快照。**本条目状态不变**（仍为待编排者 /ritual 追认的 escalate 点），本注记
仅补充：路线A的四项剩余中，两项（d/μ_R estimand、AncOK/身份彻底修复）已各自拆分为
独立谱系条目（696/697）供编排者分别裁决，避免路线A整体被当作单一不可分割的待裁项。
