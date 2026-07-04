---
id: 650
date: "2026-06-30"
title: goal 事件系统 reader/writer 契约分裂——GOAL_RESUME 可读不可写 + acceptance 闭合只认 CHECK_PASS 不认 EVIDENCE【已结算 2026-06-30：编排者裁决 B+A2 + #22 实施完成】
status: 已结算
settlement_date: "2026-06-30"
type: 选择类（编排者裁决）   # 立场A/B/A2 三选一=选择类，经编排者裁决（B+A2 审计部分）后下沉为定理实施
layer: 运维/工具层（D′ goal 事件系统，非缠论领域概念）
session: 68088c95
discovered_by: Lead（/goal 协议 commit→rescan→base_head 同步失败）
created: 2026-06-30
related: ['630', '651', '652']   # 630=写路径未实装开口①(本号是其演化态); 651=reader/writer 分裂盲区 meta 观测(降级 spec-execution-gap 实例); 652=局部矛盾误判全局阻塞 meta 观测(降级并入 275)
responsible_agents: [编排者]
rule_version_baseline:
  claude_md_commit: "4f040f0c31bb38a58b1adbbde5708096038eea65"
  rules_dir_mtime: "2026-03-14 23:27:42 +0000"

# ============================================================
# /ritual 结算区块（2026-06-30，genealogist 落实，编排者授权 /ritual）
# ============================================================
settlement:
  from_contradiction: |
    D′ goal 事件系统 reader（goal_reducer.py）与 writer（goal_events.py）+ SCHEMA 三者事件类型契约不一致：
    分裂A——GOAL_RESUME 可读不可写（reader 读它做 base_head 再锚定，writer/SCHEMA 不含，历史靠裸 append）；
    分裂B——acceptance 闭合只认 CHECK_PASS 不认 EVIDENCE（无机制把 EVIDENCE 验证结论提升为 CHECK_PASS）。
  negated_definition: |
    立场A（reducer 对，补 writer 带 base_head，每次 resume 抹平 stale）被否定——会使 base_head_stale
    永 False，失去「投影过时」预警价值。
  new_definition: |
    编排者裁决（2026-06-30「就按codex说的办吧」）= B + A2 审计部分。落地四点：
    (1) 保留不可变 GOAL_SET.base_head，永不被 RESUME 改写。
    (2) reducer 删 RESUME 重锚逻辑；base_head_stale=True 是正确降级信号，保留。
    (3) GOAL_RESUME 合法化为无状态恢复记录（goal_id+note+ts 进 SCHEMA，不碰 base_head/acceptance/closure）。
    (4) 新增 EVIDENCE 稳定 id + 授权 CHECK_PASS 提升协议（method=auto(command+verifier) / manual(judge+
        rationale) + evidence_ids 非空指向 EVIDENCE + 可选 acceptance_id 稳定身份匹配）。
    附带：DECISION/裁决治理事件 schema 缺口一并解决。
  logical_necessity: |
    A2 是 A/B 扬弃而非折中——既给 GOAL_RESUME 合法写路径（消灭裸 append 扩散，满足 630 writer 设计
    目的），又保住 base_head_stale 降级预警价值（立场 B 正确洞察：base_head 语义是 GOAL_SET 时刻锚，
    HEAD 推进后 stale 是正确降级信号不该抹平）。EVIDENCE/CHECK_PASS 二分保留 + 补授权提升路径（禁裸写，
    CHECK_PASS 必带来源），解决「EVIDENCE 不自动 pass 但无合法提升机制」的断层。
  sufficiency: |
    (1) 编排者价值判断裁决（选择类经裁决下沉为定理实施，004号）。
    (2) #22 实施完成：GOAL_AMEND 65 passed（Lead 消息确认）。
    (3) 此前 #19 两次实施被还原 = 裁决前未授权；本裁决后实施为合法。
    ⟹ 选择类已裁决 + 实施完成 + 测试通过 ⟹ 结算依据充分。
  tension_check: |
    vs 630（写路径未实装开口①）：本号是其演化态（writer 已实装但 GOAL_RESUME/CHECK_PASS 契约盲区）。
      结算精确化 630 的有效域分层（区分良性分层 vs 盲区），不破坏 630。
    vs 651（降级 spec-execution-gap 实例）：651 是本号 reader/writer 分裂的 meta 观测，本号结算后 651
      的「盲区」案例消除（reducer 删 RESUME 重锚），651 降级归并不破坏。
    vs 652（降级并入 275）：652 是本轮 Lead 把本号误当全局阻塞的 meta 观测，与本号修复方向正交。
    无 settled 被本号结算破坏（goal 事件系统是工具层，非缠论领域概念）。
  decided_by: 编排者（2026-06-30「就按codex说的办吧」= B + A2 审计部分）
rule_version_baseline_note: 见上方 rule_version_baseline
---

# 650 goal 事件系统 reader/writer 契约分裂【已结算 2026-06-30】

## 结算一句话

编排者裁决 = **B + A2 审计部分**：保留不可变 GOAL_SET.base_head（永不被 RESUME 改写），reducer 删
RESUME 重锚逻辑（base_head_stale 是正确降级信号），GOAL_RESUME 合法化为无状态恢复记录进 SCHEMA，
新增 EVIDENCE 稳定 id + 授权 CHECK_PASS 提升协议（禁裸写）。#22 实施完成（GOAL_AMEND 65 passed）。

## 裁决落地四点

1. 保留不可变 `GOAL_SET.base_head`，永不被 RESUME 改写。
2. reducer 删 RESUME 重锚逻辑；`base_head_stale=True` 是正确降级信号，保留。
3. `GOAL_RESUME` 合法化为无状态恢复记录（`goal_id+note+ts` 进 SCHEMA，不碰 base_head/acceptance/closure）。
4. 新增 `EVIDENCE` 稳定 id + 授权 `CHECK_PASS` 提升协议（`method=auto(command+verifier)` /
   `manual(judge+rationale)` + `evidence_ids` 非空指向 EVIDENCE + 可选 `acceptance_id`）。

附带：DECISION/裁决治理事件 schema 缺口一并在实施时解决。

## A2 = A/B 扬弃

A2 既给 GOAL_RESUME 合法写路径（消灭裸 append，满足 630 writer 设计目的），又保住 base_head_stale
降级预警（立场 B 正确洞察）——否定（A 的 resume 抹平 stale）+ 保留（B 的 base_head 不可变）+ 提升
（GOAL_RESUME 降为无状态恢复记录）。

---

# （以下为结算前生成态原文，谱系012：发现过程不可压扁，保留作发生史）

## 矛盾描述（原文）

D′ goal 事件系统的 reader（goal_reducer.py）与 writer（goal_events.py）+ SCHEMA（SCHEMA.md）三者事件
类型契约不一致：分裂A（GOAL_RESUME 可读不可写）+ 分裂B（acceptance 闭合只认 CHECK_PASS 不认 EVIDENCE）。

## 三立场后果对比（裁决材料）

| 维度 | 立场A（补 writer 带 base_head） | 立场A2（补 writer，RESUME 不带 base_head） | 立场B（删 reducer RESUME） |
|------|------|------|------|
| base_head_stale 信号 | 每次 resume 抹平→永 False | **保留** | 保留 |
| GOAL_RESUME 写路径 | 合法（带 base_head） | 合法（仅 goal_id+note，恢复记录） | 不存在 |
| /goal 热启动语义 | resume 重锚 | resume 仅留痕，不改锚 | resume 无事件载体 |

裁决采纳 B + A2 审计部分。

## goal g-sigma-complete-l2-nautilus 实际状态（git 真相，独立于契约矛盾）

acceptance[0]/[1]/[4] 已封；[2] L2/L3 全窗 8 品种未封（仅 OKLO 单标的）；[3] O(n)@16K 未封（实测
exp≈2.0 FAILED，真修=task#16 ElementView 大重构需 fresh session，见 654 量身份分离）。契约矛盾不阻塞
git 层真相，只阻塞 reducer 投影正确反映已封状态。

## 谱系关联

- related: 630（演化态根）、651（降级 spec-execution-gap 实例）、652（降级并入 275）、654（acceptance[4]
  O(n) 精确化为引擎事件流下界）
