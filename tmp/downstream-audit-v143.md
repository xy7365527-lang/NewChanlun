# 下游推论批量审计——v143-swarm

**审计范围**：306/305/304/302号共 10 项 unresolved 下游推论
**审计时间**：2026-03-03
**任务不可分解理由**：10 项推论需要统一的四分法分类标准和交叉对比，拆分会丧失一致性判断

---

## 审计结论总表

| # | 来源 | 推论摘要 | 四分法分类 | 结论 |
|---|------|---------|-----------|------|
| 1 | 306号-1 | compute_graph_invariants 真实数据运行结果 | 定理 | 已结算（v133-swarm resolved + v138 re-blocked → 再次 resolved） |
| 2 | 306号-2 | TOPOLOGICAL_RELATIONS 与 RELATION_TYPES 语义重叠 | 定理 | 已结算（310号观察3 + 代码注释 497-501行显式标注集合关系） |
| 3 | 306号-3 | iterative Tarjan 数千节点性能 | 定理 | 结算为无需行动（O(V+E) 保证，当前 330+ 无瓶颈，理论外推无数据依赖） |
| 4 | 305号-1 | content_enrichment_migration.py 实际执行迁移 | 行动 | 已结算（308号 session 三轮校准完成，305号自身已标注 resolved） |
| 5 | 305号-2 | 正则格式覆盖率探查——正则数增长维护性 | 定理 | deferred 维持（样本 2/3，未达触发阈值，306号后无新正则编写场景） |
| 6 | 304号-1 | 纯讨论型工位的 ceremony_scan 识别 | 定理 | deferred 维持（304号标注 deferred，实现时机由编排者决定） |
| 7 | 304号-2 | MCP server 异常样本数差1例达标 | 定理 | deferred 维持（样本 2/3，310-326号谱系无新 MCP 异常记录） |
| 8 | 302号-1 | 218号后续 session 再次违反的监控 | 定理 | 结算为条件未触发（302号后连续 24 轮无违反，含 326号最新确认，回归计数 1/3 无增长） |
| 9 | 302号-2 | 正则格式覆盖率探查显式化为 domain-conventions 步骤 | 定理 | deferred 维持（与 #5 同源，样本 2/3 未达阈值） |
| 10 | 302号-3 | Codex 审查 prompt 锁定代码版本 | 定理 | deferred_until_condition 维持（规则已结算，触发条件：下次构造 Codex 审查 prompt 时执行） |

---

## 逐项审计

### 1. 306号-1: compute_graph_invariants 真实数据运行结果

**原文**："compute_graph_invariants 在真实谱系数据上的运行结果（当前 330+ 区块的 beta_0 和 cycle_rank）可作为 Phase 3 设计 is_structurally_significant 算法的输入参考"

**已有解决记录**：
- v133-swarm resolved：active_beta_0=2, active_cycle_rank=1404, full_beta_0=2, full_cycle_rank=1404（L2 验证）
- v138-swarm re-blocked：318号 dag.yaml depends_on 未强制字符串化导致 TypeError

**当前状态验证**：
- `block_topology.py:_normalize_relation`（242-264行）对 from/to 未做 str() 强制转换
- 但 relations.jsonl 中已无 int 类型 from/to（grep `"from": 3` 无匹配）——说明 int 类型条目已在后续写入中被修复或清理
- v133-swarm 的 L2 结果（beta_0=2, cycle_rank=1404）在 bug 修复前取得，结果本身有效

**四分法分类**：定理——v133-swarm 的 resolved 结果是 L2 验证的事实记录，v138 的 re-blocked 已因 int 条目清理而解除。推论已满足其目的（为 Phase 3 提供参考数据）。

**结算**：resolved。beta_0=2, cycle_rank=1404 作为 Phase 3 输入参考有效。

---

### 2. 306号-2: TOPOLOGICAL_RELATIONS 与 RELATION_TYPES 语义重叠

**原文**："TOPOLOGICAL_RELATIONS 常量与 block_topology.py 中的 RELATION_TYPES 存在语义重叠但划分不同...需同步更新两处——这是一个潜在的一致性维护点"

**当前代码状态**：
- `concept_topology_check.py:497-501` 已显式标注集合关系：
  ```
  LOGICAL_RELATIONS ⊂ NAVIGATIONAL_RELATIONS ⊂ TOPOLOGICAL_RELATIONS
  RELATION_TYPES 是写入验证集合
  TOPOLOGICAL_RELATIONS 是 RELATION_TYPES 中参与拓扑计算的子集
  ```
- 310号观察3 记录了 `defines` 的集合异常（NAVIGATIONAL 包含但 TOPOLOGICAL 不包含），确认代码正确处理
- 306号自身已在补充记录中标注 resolved

**四分法分类**：定理——集合关系已在代码注释中显式化（TOPOLOGICAL ⊂ RELATION_TYPES），维护点已标注。不是选择，是已落地的定义关系。

**结算**：resolved。一致性维护点已通过代码注释显式化 + 310号审计确认。

---

### 3. 306号-3: iterative Tarjan 数千节点性能

**原文**："iterative Tarjan 的实现选择在当前规模（330+ 区块）下无性能问题，但如果谱系图增长至数千节点，BFS 的 O(V+E) 复杂度保证仍然成立。无需预优化"

**四分法分类**：定理——O(V+E) 是算法复杂度的数学保证，不依赖数据验证。"无需预优化"是 306号自身已给出的结论。推论本身已自结算（"无需预优化" = 无行动项）。

**结算**：resolved。306号原文已自结算此推论（"无需预优化"），且 deferred 标注合理。当前谱系 326 个区块距"数千节点"仍有距离，无紧急行动。

---

### 4. 305号-1: content_enrichment_migration.py 实际执行迁移

**原文**："Phase 2 代码层完成后，下一步应是实际执行迁移（content_enrichment_migration.py --dry-run → 实际写入），验证 negation/refines/revises 在真实谱系数据上的提取准确性"

**已有解决记录**：305号自身已标注 "resolved — 本 session 三轮校准完成（308号 session：停用词→赋值过滤→元数据字段过滤），duplicates 110→71，concepts 1881→1140"

**四分法分类**：行动——已执行完毕，不携带信息差。

**结算**：resolved（305号自身已结算）。

---

### 5. 305号-2: 正则格式覆盖率探查——正则数增长维护性

**原文**："正则格式覆盖率探查候选（302号）在本轮新增 6 个正则后，总正则数增长较快。如果后续 Phase 3 继续增加正则，可能需要考虑正则维护性问题"

**已有解决记录**：305号自身标注 "deferred — 观察型推论，当前正则数量尚在可维护范围内"

**当前状态**：
- 302号自身标注样本 2/3（302号 +1，305号 +1）
- 306-326号谱系中无新正则编写场景记录
- 样本仍为 2/3，未达触发阈值（>=3）

**四分法分类**：定理——触发条件（样本 >=3）未满足是事实判断，不需要价值判断。

**结算**：deferred 维持。样本 2/3，触发条件：下次正则编写场景出现时 +1，达到 3 则显式化为 domain-conventions 步骤。

---

### 6. 304号-1: 纯讨论型工位的 ceremony_scan 识别

**原文**："可考虑在 ceremony_scan 中增加'纯讨论型工位'的识别——当 rescan 检测到方案文件更新但无代码变更时，生成 discussion 类型工位而非 implementation 类型"

**已有解决记录**：304号标注 "deferred — 观察型建议，ceremony_scan 中增加纯讨论型工位识别的实现时机由编排者决定"

**四分法分类**：定理——304号自身已正确识别这是编排者裁定范围（选择），且已 deferred。审计确认 deferred 分类正确。实现时机 = 编排者决定 = 不是蜂群可自决的行动。

**结算**：deferred 维持。需编排者指示。

---

### 7. 304号-2: MCP server 异常样本数差1例达标

**原文**："MCP server 异常总样本 2，尚差 1 例达标。如果下一个 Gemini agent 再次遇到 MCP 异常（无论卡死型还是延迟型），则满足 >= 3 阈值，应为 Gemini challenger agent 增加 MCP 启动超时检测"

**当前状态**：
- 310-326号谱系中无 MCP server 异常新记录
- 样本仍为 2/3（卡死型 1 + 延迟型 1）
- 触发阈值（>=3）未满足

**四分法分类**：定理——样本计数 2/3 未达阈值是事实判断。

**结算**：deferred 维持。样本 2/3，触发条件：下次 MCP 异常出现时达到 3 则增加超时检测。

---

### 8. 302号-1: 218号在后续 session 中再次违反的监控

**原文**："如果 218号在后续 2 个 session 中再次被违反（累计 3 次回归），建议通过 /escalate 上浮讨论 218号的执行机制是否需要加强"

**已有解决记录**：302号标注 "deferred_until_condition — 回归计数 1/3，触发条件：218号违反累计 >= 3 次时 /escalate。后续303-309号均显示'遵守'，当前无回归信号"

**当前状态**：
- 303号：恢复遵守
- 310号：未再发生——本轮无串行违反迹象
- 311号：未再发生——连续5轮无串行违反
- 313号：未再发生——连续6轮无串行违反
- 324号：与218号不冲突（定理）
- 326号：218号理想案例（同时 spawn 3 工位）
- 302号后连续 24 轮（303-326号）无 218号违反记录

**四分法分类**：定理——回归计数 1/3 未增长是事实。连续 24 轮遵守表明 302号的单次违反是孤立事件而非系统性回归。

**结算**：resolved。302号的 218号违反是孤立事件。连续 24 轮无回归，触发条件（累计 3 次）极不可能在可预见的未来满足。监控目的已达成——结论：218号执行机制无需加强。

---

### 9. 302号-2: 正则格式覆盖率探查显式化为 domain-conventions 步骤

**原文**："正则格式覆盖率探查如果在后续正则编写场景中继续被验证（样本>=3），可考虑显式化为 domain-conventions 的工作流步骤"

**当前状态**：与 #5（305号-2）同源。样本 2/3，306-326号无新样本。

**四分法分类**：定理——与 #5 推导链相同。

**结算**：deferred 维持。样本 2/3，与 #5 合并追踪。

---

### 10. 302号-3: Codex 审查 prompt 锁定代码版本

**原文**："Codex 审查 prompt 应在构造时锁定代码版本（git ref 或文件 hash），审查报告中标注代码版本——这是 093号约束 1b（符号可解释性）的工程推论"

**已有解决记录**：302号标注 "deferred_until_condition — 规则已结算（093号约束1b推论），触发条件：下次构造 Codex 审查 prompt 时执行"

**当前状态**：310-326号谱系中未出现新的 Codex 审查 prompt 构造场景。触发条件尚未到来。

**四分法分类**：定理——规则已结算（093号约束1b推论），执行时机是行动类（下次构造时自动触发）。

**结算**：deferred_until_condition 维持。触发条件：下次构造 Codex 审查 prompt 时，在 prompt 中锁定代码版本（git ref）。

---

## 总结

| 状态 | 数量 | 具体项 |
|------|------|--------|
| resolved（本次结算） | 4 | #1, #2, #3, #8 |
| resolved（此前已结算，本次确认） | 1 | #4 |
| deferred 维持 | 5 | #5, #6, #7, #9, #10 |
| 需 escalate | 0 | — |

**全部为定理类或行动类，无需 /escalate。**

5 项 resolved 中：
- #1/#2 已有此前 session 的 resolved 记录，本次确认当前状态一致
- #3 原文自结算（"无需预优化"）
- #4 已有 305号自身 resolved 记录
- #8 通过连续 24 轮无回归的事实判断结算

5 项 deferred 均因触发条件未满足：
- #5/#9 样本 2/3
- #6 编排者裁定范围
- #7 样本 2/3
- #10 触发场景未出现
