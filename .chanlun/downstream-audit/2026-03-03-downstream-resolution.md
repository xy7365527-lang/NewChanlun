---
date: "2026-03-03"
session: v138-swarm downstream-resolver
resolved_by: downstream-resolver 工位
---

# 下游推论批量结算报告

## 处理依据

四分法分类（no-unnecessary-escalation.md）：
- **定理**：已结算原则的逻辑必然推论 → 自动结算
- **行动**：不携带信息差的操作性事件 → 直接执行或确认执行状态
- **选择**：多种合理方案 → 标记为需要编排者决策
- **语法记录**：运作中但未显式化的规则 → 标记

---

## 306-1：compute_graph_invariants 在真实谱系数据上运行

**分类**：行动（部分可执行，部分阻塞）

**状态**：blocked — 存在已知 bug，无法直接运行

**调查结果**：
- concept_topology_check.py 运行时抛出 `TypeError: 'int' object is not subscriptable`
- 根因：318号谱系的 `depends_on` 关系在 relations.jsonl 中 from/to 为 int 类型（源于 dag.yaml 中 YAML 数字解析未强制字符串化），共 5 条关系受影响
- 具体条目：`{"from": 318, "to": 317/316/315/314/90, "relation": "depends_on"}`
- 修复路径：`read_all_relations` 需对 from/to 字段做 `str()` 强制转换，或 dag.yaml 写入时强制字符串格式

**历史记录**（来自306号谱系下游推论解决记录）：
- 306号谱系文件末尾已记录"推论1（图不变量真实数据验证）：resolved — 实测结果：active_beta_0=2, active_cycle_rank=1404, full_beta_0=2, full_cycle_rank=1404"
- 该记录是 v133-swarm session 添加的，说明此前已成功运行过

**结论**：306号推论1曾经已结算（v133-swarm），当前的 bug 是 318号谱系数据写入引入的新问题。

**结算状态**：`deferred_until_condition` — 等待 relations.jsonl 中 int 类型 bug 修复

---

## 306-2：TOPOLOGICAL_RELATIONS 与 RELATION_TYPES 语义重叠统一

**分类**：行动（部分已完成）

**状态**：resolved（已在 v133-swarm 完成）

**调查结果**：
- concept_topology_check.py 第 490-501 行：TOPOLOGICAL_RELATIONS 已有显式注释标注集合关系
- 第 497-501 行："语义一致性说明：LOGICAL_RELATIONS ⊂ NAVIGATIONAL_RELATIONS ⊂ TOPOLOGICAL_RELATIONS ⊂ RELATION_TYPES"
- 306号谱系文件末尾下游推论解决记录："推论2（TOPOLOGICAL_RELATIONS 整合）：resolved — 已显式说明集合关系 + 代码层正确处理"

**结论**：已由 v133-swarm 结算，不需要进一步操作。

---

## 305-1：content_enrichment_migration 实际执行验证

**分类**：行动（已完成）

**状态**：resolved（已在 v133-swarm 完成）

**调查结果**：
- `.chanlun/block-topology/meta.json` 中存在 `content_enrichment` 字段：
  ```json
  "content_enrichment": {
    "blocks_created": 313,
    "relations_written": 1309,
    "concepts_defined": 1148
  }
  ```
- 305号谱系文件末尾下游推论解决记录："推论1（enrichment 执行）：resolved — 本 session 三轮校准完成（308号 session：停用词→赋值过滤→元数据字段过滤），duplicates 110→71，concepts 1881→1140"

**结论**：已由 v133-swarm 结算，meta.json 中有执行证据。

---

## 302-3：Codex 审查 prompt 版本锁定

**分类**：定理（302号观察6 的推论：审查 prompt 必须基于最新代码）

**状态**：deferred_until_condition — 等待 Codex 审查触发场景

**分析**：
- 302号观察6 已结算规则："审查 prompt 必须基于审查时的最新代码，并标注代码版本（git ref 或文件 hash）"
- 这是工程规范要求，不是当前 session 需要执行的操作
- 触发条件：下次构造 Codex 审查 prompt 时执行

**结算状态**：`deferred_until_condition` — 触发条件：构造 Codex 审查 prompt 时

---

## 306-3：iterative Tarjan 规模增长（当前 330+ 无问题）

**分类**：定理（条件未满足）

**状态**：deferred_until_condition

**分析**：
- 306号谱系观察3 已结算：iterative Tarjan O(V+E) 在当前规模（330+）下无问题
- 触发条件：谱系图增长至数千节点
- 当前区块数：1175（meta.json block_count）

**注意**：当前区块数 1175 > 当时记录的 330+，但仍在无问题范围内（iterative 实现不受栈深度限制）

**结算状态**：`deferred_until_condition` — 触发条件：规模增长至数千节点时评估

---

## 305-2：正则格式覆盖率（当前无紧急行动）

**分类**：定理（条件未满足）

**状态**：deferred_until_condition

**分析**：
- 305号谱系下游推论解决记录："推论2（正则维护性）：deferred — 当前正则数量尚在可维护范围内"
- 302号标识了"正则格式覆盖率探查"作为语法记录候选（样本 1）
- 305号样本 +1（concept_extractor 新增 6 个否定相关正则）
- 触发条件：样本 >= 3 时考虑显式化为 domain-conventions 工作流步骤

**结算状态**：`deferred_until_condition` — 触发条件：正则编写场景累计 >= 3 样本

---

## 304-2：MCP server 异常样本不足（2/3）

**分类**：定理（条件未满足）

**状态**：deferred_until_condition

**分析**：
- 304号谱系：MCP 异常总样本 2（卡死型 1 + 延迟型 1），阈值为 >= 3
- 触发条件：第 3 例 MCP 异常（无论卡死型还是延迟型）出现
- 304号下游推论解决记录："deferred"

**结算状态**：`deferred_until_condition` — 触发条件：MCP 异常样本 >= 3

---

## 302-1：218号违反回归未达 3 次阈值

**分类**：定理（条件未满足）

**状态**：deferred_until_condition

**分析**：
- 302号下游推论1：累计 3 次 218号回归违反时 /escalate
- 302号是第 1 次，后续 303/304/305/306/307/308/309 均显示 218号"遵守"
- 回归计数：1/3

**结算状态**：`deferred_until_condition` — 触发条件：218号违反累计 >= 3 次

---

## 302-2：正则格式探查样本不足

**分类**：定理（与 305-2 重叠）

**状态**：deferred_until_condition（与 305-2 合并跟踪）

**分析**：同 305-2。302号首次出现（样本 1），305号 +1（样本 2），当前样本 2/3。

**结算状态**：`deferred_until_condition` — 与 305-2 合并，触发条件相同

---

## 304-1：纯讨论型工位识别（需要价值判断）

**分类**：选择

**状态**：需要编排者决策

**分析**：
- 304号观察2：纯理论讨论蜂群样本 3 达标
- 304号下游推论1："可考虑在 ceremony_scan 中增加纯讨论型工位的识别"
- 304号边界条件明确标注："达标仅指模式识别的统计显著性，不等于 ceremony_scan 必须立即实现——实现时机由编排者决定"
- 这是实现时机和优先级的价值判断，非可自决的技术问题

**结算状态**：`needs_orchestrator_decision` — 需要编排者决定 ceremony_scan 中是否以及何时实现纯讨论型工位识别

---

## 汇总

| 推论 | 四分法 | 结算状态 |
|------|--------|---------|
| 306-1 compute_graph_invariants 运行 | 行动（阻塞） | deferred — int 类型 bug 阻塞 |
| 306-2 TOPOLOGICAL_RELATIONS 整合 | 行动 | resolved（v133-swarm 已完成） |
| 305-1 enrichment 执行验证 | 行动 | resolved（v133-swarm 已完成） |
| 302-3 Codex prompt 版本锁定 | 定理 | deferred_until_condition（下次审查时） |
| 306-3 Tarjan 规模增长 | 定理 | deferred_until_condition（数千节点时） |
| 305-2 正则覆盖率 | 定理 | deferred_until_condition（样本 2/3） |
| 304-2 MCP 异常样本 | 定理 | deferred_until_condition（样本 2/3） |
| 302-1 218号违反回归 | 定理 | deferred_until_condition（回归 1/3） |
| 302-2 正则探查样本 | 定理（与 305-2 合并） | deferred_until_condition（样本 2/3） |
| 304-1 讨论型工位识别 | 选择 | needs_orchestrator_decision |

**额外发现**：306-1 无法执行的根因——318号谱系 dag.yaml 写入时 depends_on 未强制字符串化，导致 relations.jsonl 中出现 int 类型的 from/to 字段（5条）。修复路径：`block_topology.py:read_all_relations` 对 from/to 做 `str()` 强制转换。
