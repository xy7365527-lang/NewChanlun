---
id: 650
title: goal 事件系统 reader/writer 契约分裂——GOAL_RESUME 可读不可写 + acceptance 闭合只认 CHECK_PASS 不认 EVIDENCE
status: 生成态
type: 语法记录候选 + 工具层契约矛盾
layer: 运维/工具层（D′ goal 事件系统，非缠论领域概念）
session: 68088c95
discovered_by: Lead（/goal 协议 commit→rescan→base_head 同步失败）
created: 2026-06-30
related: ['630'（写路径未实装开口①——writer 整体未实装，本号是其演化态：writer 已实装但 GOAL_RESUME/CHECK_PASS 契约盲区）]
rule_version_baseline:
  claude_md_commit: "4f040f0c31bb38a58b1adbbde5708096038eea65"
  rules_dir_mtime: "2026-03-14 23:27:42 +0000"
---

# 矛盾报告

## 矛盾描述

D′ goal 事件系统的 **reader（`goal_reducer.py`）与 writer（`goal_events.py`）+ SCHEMA（`SCHEMA.md`）三者的事件类型契约不一致**，导致 /goal 运行协议的 base_head 再锚定与 goal 闭合**无法通过合法写路径完成**，只能靠裸 append 绕过 writer（而裸 append 正是 writer 设计要消灭的"退化写法扩散"）。

两处具体分裂：

### 分裂 A：GOAL_RESUME 可读不可写
- **reader 侧**（`goal_reducer.py:52-58`）：明确 **读** `GOAL_RESUME` 做 base_head 再锚定，注释称"RESUME 是合法的再锚定事件，复用 SET 的 stale 逻辑"，并论证"不认 RESUME → base_head 永久停在最初 GOAL_SET，与真实 HEAD 永久 stale"。
- **writer 侧**（`goal_events.py` `_REQUIRED_FIELDS`）+ **SCHEMA**（`SCHEMA.md` 7 种事件表）：**都不含 GOAL_RESUME**。writer 显式拒绝："未知 event 类型 'GOAL_RESUME'（仅接受 SCHEMA.md 定义的 7 种）"。
- **后果**：历史 2 条 GOAL_RESUME（events line128/132，含本 session 热启动写的）全是**裸 append 绕过 writer** 写入的。reducer 依赖一个 writer 不产生、SCHEMA 未定义的事件类型完成核心功能。

### 分裂 B：acceptance 闭合只认 CHECK_PASS，不认 EVIDENCE
- **reader 侧**（`goal_reducer.py:70-94`）：acceptance 的 `passed` **只由 `CHECK_PASS` 事件驱动**（`(gid, check)` 精确文本匹配）。`EVIDENCE` 事件**完全不被消费**于 passed 判定。
- **行为后果**：Lead 在 /goal 循环中持续写 `EVIDENCE`（叙事证据），期望反映验收进展，但 reducer 重算时 acceptance 全 `passed=False`。goal 永远无法经 /goal 协议正常闭合，除非每次额外裸写 `CHECK_PASS`（check 文本须与 GOAL_SET 逐字一致）。
- 这不是 bug——`SCHEMA.md` 注释明示"叙事归 EVIDENCE，结构归 CHECK_PASS"。但**没有任何机制把 EVIDENCE 的验证结论提升为 CHECK_PASS**，二者之间是人工裸写的断层。

## 双方论证

**立场A（reducer 正确，writer/SCHEMA 缺失）**：GOAL_RESUME 是 /goal 协议的语义必需事件（624 裁决 Lead 默认 goal，热启动必然 resume + 推进 HEAD → 必然需要再锚定）。reducer 已正确实现它。缺的是 writer/SCHEMA 没补上 GOAL_RESUME。修复 = SCHEMA 加第 8 种事件 GOAL_RESUME(goal_id, base_head, note?, ts) + writer `_REQUIRED_FIELDS` 加对应项。CHECK_PASS 同理需要一个"EVIDENCE→CHECK_PASS 提升"的合法路径（谁有权把 L2 验证结论封为 CHECK_PASS）。

**立场B（reducer 越界，base_head 不该靠事件再锚定）**：base_head 语义是"GOAL_SET 时刻的锚"。HEAD 推进后 stale 是**正确的降级信号**（提示 reducer 投影可能过时，应从 git 重算）——不该用 RESUME 抹平。若每次 commit 都 resume 抹平 stale，base_head_stale 信号永远为 False，失去"投影过时"的预警价值。此立场下 GOAL_RESUME 应从 reducer 删除，base_head 永远等于最初 GOAL_SET。

## 涉及的定义/契约文件

- `scripts/goal_reducer.py:52-58`（reader 读 GOAL_RESUME）、`:70-94`（CHECK_PASS 驱动 passed）
- `scripts/goal_events.py` `_REQUIRED_FIELDS`（writer 7 种，无 GOAL_RESUME）
- `.chanlun/goals/SCHEMA.md`（7 种事件定义表，无 GOAL_RESUME；注释"叙事归 EVIDENCE，结构归 CHECK_PASS"）
- events.jsonl line128/132（历史裸 append 的 GOAL_RESUME 证据）

## 谱系比对结果

- **630 开口①**（"写路径未实装"止血）：reducer 注释多处引用 630，承认 reader 宽容读历史 ⊋ writer 严格守新写是**有意的有效域分层**（formalization-validity-domain）。但 630 的分层针对的是"历史退化事件（sub_goal_id 当 goal id）"，**不覆盖** GOAL_RESUME 这种 reader 依赖但 writer 从未支持的事件——这是分层的**盲区**而非良性分层。
- **升格判定**：本矛盾是 goal 事件系统**内部** reader/writer 契约，属"goal 定义/验收/分解原则语义"，达到升格 genealogy 的门槛（goal 事件系统语义变更才升格 genealogy 的一般原则）。
  - 注（genealogist R2 核查）：初稿误引 575 为"goal events ≠ genealogy 分层防污染"——系虚构（575 实为 clean-slate 重建消除双代共存，与本矛盾无语义关联），已移除。

## 需要决断的问题

**GOAL_RESUME 与 EVIDENCE→CHECK_PASS 提升路径，是补 writer/SCHEMA（立场A）还是改 reducer（立场B）？**

具体二选一（或第三方案）：
1. **立场A**：SCHEMA + writer 增 GOAL_RESUME 为合法第 8 种事件；并定义"EVIDENCE 的 L2 验证结论由谁/如何合法提升为 CHECK_PASS"（candidate：code-verifier/quality-guard 真封后有权写 CHECK_PASS）。
2. **立场B**：reducer 删除 GOAL_RESUME 再锚定，承认 base_head_stale 是正确的降级信号不该抹平。

**当前 goal `g-sigma-complete-l2-nautilus` 的实际状态（独立于本契约矛盾，git 真相）**：
- acceptance[0] Q4 ElementId：**已封**（b14fe2a290 在 HEAD，grep 70 命中，1224 tests）
- acceptance[1] ΔSharpe≠0：**已封**（K_i 激活 OKLO 腿 0→20 + ΔSharpe=−0.0466≠0）
- acceptance[4] Nautilus：**已封**（7c7dce5d7f 真 BacktestEngine）
- acceptance[2] L2/L3 全窗 8 品种：**未封**（仅 OKLO 单标的）
- acceptance[3] O(n)@16K：**未封**（实测 exp≈2.0 FAILED，真修=task#16 ElementView 大重构需 fresh session）

即 goal 实质完成 3/5，剩 2 项中 [3] 明确需 fresh session 专做大重构。本契约矛盾不阻塞 git 层真相，只阻塞 reducer 投影正确反映已封状态。
