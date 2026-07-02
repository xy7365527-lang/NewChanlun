---
id: "658"
number: 658
type: domain   # SCHEMA 缺口：子目标 CHECK_PASS → 顶层 acceptance 的 rollup 映射未定义（选择类）
status: 生成态   # genealogist 结构记录。非实现笔误——goal_reducer.py 逻辑自洽于其自身契约(行81-83:顶层acceptance由sub_goal_id==gid的CHECK_PASS闭合)。真缺口=SCHEMA未定义rollup语义。选择类待编排者/escalate。（待#37 codex 裁定）
date: "2026-06-30"
source: genealogist（Lead 在 goal g-alpha-causal-selector 推进中报「reducer acceptance.passed 键不一致」；核源码确认=SCHEMA rollup 缺口非实现笔误）
depends_on: ["650"]
related: ["630", "036", "g-alpha-causal-selector"]
negation_source: "源码事实 L0：goal_reducer.py 行95-96(写入 passed_accept_ids 键=(sub_goal_id, acc_id)) + 行120(查询键=(gid, acc.id)) + 行81-83 注释契约(顶层 acceptance 的 CHECK_PASS.sub_goal_id 必须=gid)。实证 L2：g-alpha-causal-selector mu-estimator/mutex-interp-test 子目标 CHECK_PASS，sub_goal ready 推进正确(L2 ready=[chi-theta-filter])，但顶层 5 项 acceptance.passed 全 False。"
negation_form: "separation"   # 把「acceptance 闭合」分离为两个未定义对象：(A) gid-scoped CHECK_PASS 独立闭合(当前设计) vs (B) 子目标 CHECK_PASS rollup 冒泡——SCHEMA 未定义哪个
title: "goal_reducer.py acceptance rollup SCHEMA 缺口：子目标 CHECK_PASS 是否冒泡到顶层 acceptance 未定义。代码自洽于自身契约(行81-83:顶层 acceptance 由 sub_goal_id==gid 的 CHECK_PASS 闭合,防任意子目标同名 acceptance_id 误闭合=codex MAJOR-2)。但 SCHEMA 未定义子目标完成→顶层 acceptance 的 rollup 映射 ⟹ 所有子目标 CHECK_PASS 后顶层仍永 False ⟹ goal 永不 terminated(除非独立发 gid-scoped CHECK_PASS)。非实现笔误(选择类:两种 rollup 设计互斥待裁),不阻塞 L2/L3 推进(sub_goal ready 正确)"

contradiction:
  description: |
    Lead 报 goal_reducer.py「acceptance.passed 键不一致」：写入(行95-96)键含 sub_goal_id，
    查询(行120)键含 goal_id，sub_goal_id≠goal_id ⟹ 顶层 acceptance 永 False。

    **核源码后判定：非实现笔误，是 SCHEMA rollup 缺口（选择类）。** 代码逻辑**自洽于其自身契约**——
    注释行81-83 明示设计：「goal acceptance 的 CHECK_PASS.sub_goal_id 必须=gid」。即顶层 acceptance
    设计上要求一个 sub_goal_id==gid 的 CHECK_PASS 事件来闭合，**不**由任意子目标的 CHECK_PASS 自动冒泡。
    这是 codex MAJOR-2 的防碰撞设计：acceptance_id 非全局唯一，任意子目标的同名 acceptance_id CHECK_PASS
    不得误闭合顶层 goal acceptance。所以 gid-scoped 查询是**有意**的（不是把 sub_goal_id 写成 goal_id 的笔误）。

    **真缺口**：SCHEMA 未定义「子目标 CHECK_PASS → 顶层 acceptance」的 rollup 映射。两种合理设计互斥：
    - **(A) 当前设计（gid-scoped 独立闭合）**：顶层 acceptance 须独立发 sub_goal_id==gid 的 CHECK_PASS，
      子目标完成不自动满足顶层。优点=防误闭合(codex MAJOR-2)；缺点=顶层 acceptance 需冗余的 gid-scoped
      CHECK_PASS，子目标全 PASS 不自动 terminate goal。
    - **(B) rollup 冒泡**：定义 acceptance_id↔sub_goal_id 映射（如同名即对应），子目标 CHECK_PASS
      自动满足对应顶层 acceptance。优点=子目标全 PASS → goal 自动 terminate；缺点=须解决 acceptance_id
      跨子目标名碰撞(codex MAJOR-2 的原始担忧)。

    哪个对依赖价值判断（goal 终止语义：顶层 acceptance 是独立验收项 vs 子目标完成的汇总）——选择类，
    待编排者 /escalate。
  layer: 定义   # goal SCHEMA 的 acceptance↔sub_goal rollup 语义层未定义。非实现错误(代码自洽自身契约)。
  trigger: "g-alpha-causal-selector mu-estimator/mutex-interp-test 子目标 CHECK_PASS，sub_goal ready 正确推进(L2 ready=[chi-theta-filter])，但顶层 5 项 acceptance.passed 全 False。Lead 查 reducer 报键不一致。"

definitions_involved:
  - name: "goal_reducer.py acceptance 闭合契约（行81-83/95-96/119-123）"
    version: "scripts/goal_reducer.py（HEAD）"
    role: "缺口所在。代码自洽于「顶层 acceptance 由 sub_goal_id==gid 的 CHECK_PASS 闭合」契约（codex MAJOR-2 防误闭合）。但 SCHEMA 未定义子目标 CHECK_PASS 是否 rollup 到顶层。"
  - name: "650 稳定身份 + reader/writer 契约（settled）"
    version: ".chanlun/genealogy/settled/650"
    role: "约束来源。650 定 acceptance 稳定身份(acceptance_id 优先)+ reader 忠实历史。本号是 650 未覆盖的维度：稳定身份解决「同一 acceptance 跨事件匹配」，未解决「子目标 acceptance 是否冒泡顶层」(rollup 是不同问题)。"

resolution:
  type: 未解决   # SCHEMA rollup 语义未定义(选择类)。哪种 rollup 设计(A 独立闭合 vs B 冒泡)=价值判断待编排者/escalate。genealogist 不裁定、不改代码。
  description: |
    判定：**非实现错误（代码自洽自身契约），是 SCHEMA rollup 缺口（选择类）。** 按 testing-override.md
    判据：修复要么改 SCHEMA 定义子目标→顶层 rollup 映射(接受 B)，要么确认当前设计正确并补「顶层 acceptance
    须独立发 gid-scoped CHECK_PASS」的 goal 编写约定(接受 A)——均改变定义/适用范围 ⟹ 定义缺口(选择类)。
    区别于纯实现错误(若只需把行120的 gid 改 sub_goal_id 即修，但那会引入 codex MAJOR-2 的误闭合,
    不是无副作用的修复)。

    **不阻塞当前推进**：sub_goal ready 推进正确(L2 ready=[chi-theta-filter] 已解锁)，本缺口只影响
    goal 能否**自动 terminated**。在 /escalate 裁决前，goal 可由独立发 gid-scoped CHECK_PASS 手动闭合
    (当前设计 A 的路径)。genealogist 不改代码、不裁定哪种 rollup——选择类待编排者。
  decided_by: 待裁决   # SCHEMA rollup 设计(A 独立闭合 vs B 冒泡)=选择类,编排者/escalate

negated:
  description: "(1) reducer 行120 是实现笔误(把 sub_goal_id 误写 goal_id)，改 gid→sub_goal_id 即修。(2) 子目标全 CHECK_PASS 应自动 terminate goal(rollup 是已定义行为)。"
  why_negated: "(1) 行81-83 注释明示「顶层 acceptance 的 CHECK_PASS.sub_goal_id 必须=gid」是有意设计(codex MAJOR-2 防误闭合)——gid 查询非笔误。把 gid 改 sub_goal_id 会让任意子目标的同名 acceptance_id 误闭合顶层(MAJOR-2 担忧复活)。(2) SCHEMA 未定义子目标→顶层 rollup 映射——当前是设计 A(独立闭合),B(冒泡)未定义。哪个对=选择类未裁。"

new_output:
  definitions:
    - "reducer acceptance 闭合现状=设计 A(gid-scoped 独立闭合)：顶层 acceptance 须 sub_goal_id==gid 的 CHECK_PASS，子目标 CHECK_PASS 不冒泡。"
    - "SCHEMA 缺口：子目标 CHECK_PASS → 顶层 acceptance 的 rollup 映射未定义(A 独立闭合 vs B 冒泡互斥,待裁)。"
    - "影响：goal 永不自动 terminated(子目标全 PASS 后)，除非独立发 gid-scoped CHECK_PASS。不阻塞 sub_goal ready 推进(L2/L3 正确)。"
  code_changes: "无(genealogist 不改代码,624 工具有效域)。裁决后修复=行动类(A:补 goal 编写约定+保持代码 / B:改 SCHEMA + reducer rollup 逻辑 + 解 MAJOR-2 名碰撞),待 Lead 派工位。"
  orchestration_changes: "方法论：①「键不一致」类报告须核代码是否自洽于自身契约——本号代码自洽(行81-83),不一致是 SCHEMA 缺口非笔误。把 SCHEMA 缺口当笔误直接改(gid→sub_goal_id)=引入 codex MAJOR-2 误闭合(no-workaround:不可硬改一端绕过未定义语义)。②goal 终止语义(顶层 acceptance vs 子目标汇总)是 SCHEMA 层概念,rollup 映射须显式定义。"

impact:
  affected_modules:
    - "scripts/goal_reducer.py 行119-123 → acceptance 闭合现状=设计 A;裁决后(若 B)须改 rollup 逻辑 + 解 acceptance_id 跨子目标名碰撞。"
    - "goal SCHEMA(acceptance↔sub_goal rollup)→ 未定义,待 /escalate 裁 A/B。"
    - "g-alpha-causal-selector goal → 顶层 5 项 acceptance 当前永 False(设计 A 未发 gid-scoped CHECK_PASS);不阻塞 L2/L3 sub_goal 推进。"
  affected_definitions:
    - "650(settled)：本号是其未覆盖维度(稳定身份解决跨事件匹配,未解决子目标→顶层 rollup)。不否定,补充。维持 settled。"
  downstream_implications:
    - "goal 自动 terminated 依赖 rollup 语义裁决——裁决前 goal 不会因子目标全 PASS 自动关闭(需独立 gid-scoped CHECK_PASS)。"
    - "若裁 B(冒泡)：须同时解 codex MAJOR-2(acceptance_id 跨子目标名碰撞)——rollup 映射须明确「哪个子目标的 CHECK_PASS 满足哪个顶层 acceptance」,不能纯靠同名。"
    - "不阻塞 g-alpha-causal-selector 的 L2/L3 推进(sub_goal ready 链正确)——只影响顶层自动终止。"

related_records:
  parent: "650(稳定身份+reader/writer 契约)——本号是其未覆盖的 rollup 维度"
  children: []
  related:
    - "650(settled)：稳定身份(跨事件匹配)≠rollup(子目标→顶层冒泡),正交补充。"
    - "630(settled,goal 持久化开口)：rollup 是 goal 终止语义的一部分,630 开口①(reducer 驱动)的下游。"
    - "036(spec-execution-gap)：SCHEMA 未定义 rollup = 声明(goal 应自动 terminate)与能力(reducer 无 rollup)缺口的变体。"

epistemological_levels:
  - proposition: "reducer 行120 gid-scoped 查询自洽于行81-83 契约(顶层 acceptance 由 sub_goal_id==gid 闭合,codex MAJOR-2 防误闭合)——非实现笔误"
    level: "L0(源码 + 注释契约一致性判定)"
    increment: "高：判定非笔误(代码自洽),排除「改 gid→sub_goal_id 即修」的错误修复"
  - proposition: "SCHEMA 未定义子目标 CHECK_PASS → 顶层 acceptance 的 rollup 映射(A 独立闭合 vs B 冒泡互斥)"
    level: "L0(SCHEMA 缺失判定)"
    increment: "高：缺口定位(rollup 语义未定义)=选择类"
  - proposition: "g-alpha-causal-selector 子目标 CHECK_PASS 后顶层 5 项 acceptance 全 False,sub_goal ready 推进正确"
    level: "L2(reducer 实测:顶层 False + L2 ready=[chi-theta-filter])"
    increment: "高：缺口的 L2 实证(影响自动终止,不阻塞推进)"
---

# 658 reducer acceptance rollup SCHEMA 缺口：子目标 CHECK_PASS 是否冒泡顶层未定义

## 一句话结论

Lead 报 goal_reducer.py「acceptance.passed 键不一致」(写入键 sub_goal_id / 查询键 goal_id)。**核源码后
判定：非实现笔误，是 SCHEMA rollup 缺口（选择类）。** 代码**自洽于自身契约**(行81-83:顶层 acceptance 由
sub_goal_id==gid 的 CHECK_PASS 闭合，codex MAJOR-2 防任意子目标同名 acceptance_id 误闭合)。真缺口=SCHEMA
未定义「子目标 CHECK_PASS → 顶层 acceptance」的 rollup 映射。**不阻塞 L2/L3 推进**(sub_goal ready 正确)，
只影响 goal 能否自动 terminated。哪种 rollup 设计(A 独立闭合 vs B 冒泡)=价值判断，待编排者 /escalate。

## A vs B（rollup 设计互斥）

| 设计 | 语义 | 优点 | 缺点 |
|------|------|------|------|
| **A 当前(gid-scoped 独立闭合)** | 顶层 acceptance 须 sub_goal_id==gid 的 CHECK_PASS | 防误闭合(codex MAJOR-2) | 子目标全 PASS 不自动 terminate,需冗余 gid-scoped CHECK_PASS |
| **B rollup 冒泡** | 子目标 CHECK_PASS 经 acceptance_id↔sub_goal_id 映射满足顶层 | 子目标全 PASS → goal 自动 terminate | 须解 acceptance_id 跨子目标名碰撞 |

## 为何非实现笔误（排除错误修复）

行120 的 gid 查询**不是**把 sub_goal_id 写成 goal_id 的笔误——行81-83 注释明示这是 codex MAJOR-2 的
有意设计(防任意子目标同名 acceptance_id 误闭合顶层)。直接改 gid→sub_goal_id 会让误闭合复活
(no-workaround:不可硬改一端绕过未定义的 rollup 语义)。

## genealogist 边界

genealogist 判定分类(SCHEMA 缺口/选择类)+ 记录,不改代码、不裁定 A/B。选择类待编排者 /escalate。

## 张力检查（019d/020）

- **vs 650(settled,parent)**：稳定身份(跨事件匹配)≠rollup(子目标→顶层冒泡)。650 未覆盖 rollup 维度,本号补充,不否定。维持 settled。
- **vs 630/036(settled)**：rollup 是 goal 终止语义(630 开口①下游)+ 声明-能力缺口变体(036)。印证,不否定。
- **中断 #1**：无两条定义互斥——是 SCHEMA 未定义 rollup(两种设计待裁,非已有定义冲突)。**不触发中断 #1**(经典 SCHEMA 缺口,走 /escalate 选择类)。
- **递归完成(020)**：第0层本号写入(rollup 缺口+A/B);第1层×650→稳定身份≠rollup(净新发现高);第2层×630/036→终止语义/声明缺口(净新发现降)。背驰∧分型⟹结构完成。选择类待编排者/escalate。

## 回溯扫描（职责3）

- **650(settled)**：未覆盖 rollup 维度,本号补充,不否定。维持 settled。
- **630/036**：rollup 是其下游/变体,印证。
- **无 settled 被破坏。** SCHEMA rollup 设计(A/B)=选择类待编排者 /escalate;裁决后修复=行动类(待 Lead 派工位);genealogist 不改代码不裁定。
