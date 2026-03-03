---
id: '306'
number: 306
type: meta-rule
title: "元观察——Phase 2 图不变量实现 session（compute_graph_invariants + is_structurally_significant 接口预留）"
date: "2026-03-02"
depends_on: ['305', '304', '302', '273', '218', '137', '090']
status: 已结算
rule_version_baseline:
  claude_md_commit: "97f3ba3186f1919ff6abf02dea4247724d47c79a"
  rules_dir_mtime: "2026-03-01 00:27:49 +0000"
---

# 306号：元观察——Phase 2 图不变量实现 session

## 递归判断

任务不可分解：meta-observer 是单一观察角色，观察过程不可并行分割。扁平退化特例。

## 观察对象

本 session 的核心工作：context compaction 后恢复，完成 Phase 2 最后两个待实现项——Step 5b（compute_graph_invariants）和 Step 5.6（is_structurally_significant 接口预留），117 测试全部通过。

关键活动：
1. 从 context compaction 恢复，继续 Phase 2 实现
2. spawn 1 个 agent 实现 compute_graph_invariants()——iterative Tarjan SCC（圈秩计算）+ BFS 弱连通分量（beta_0 计算），在活跃图和全量图上分别计算
3. 实现 is_structurally_significant()——Phase 2 接口预留，两条路径均返回 None
4. 新增 TOPOLOGICAL_RELATIONS 常量——将拓扑关系（depends_on, negates, related 等 15 种）与非拓扑关系（records, defines, annotates）显式区分
5. 更新 run_all_checks() 集成图不变量，invariant_status 从 "not_computed" 变为 "graph_invariants_computed"
6. 手动添加 10 个新测试——TestGraphInvariants 8 个（空图/链/断连/环/invalidated/非拓扑排除/复杂SCC/报告集成）+ TestIsStructurallySignificant 2 个（无stats/有stats均返回None）
7. 修复旧测试断言（invariant_status 值变更适配）
8. 117/117 测试全部通过（test_concept_topology_check 57 个 + test_concept_extractor 60 个）
9. Phase 2 全部 7 步代码实现完成

### 关键文件变更

- `scripts/concept_topology_check.py`：+compute_graph_invariants()（iterative Tarjan SCC + BFS beta_0）、+is_structurally_significant()（Phase 2 接口预留返回 None）、+TOPOLOGICAL_RELATIONS 常量、run_all_checks() 集成
- `tests/test_concept_topology_check.py`：+TestGraphInvariants（8 测试）、+TestIsStructurallySignificant（2 测试）
- import 变更：从 block_topology 新增 VALIDITY_CAPABLE_RELATIONS 导入

## 观察结果

### 观察1（收敛信号）：单 agent spawn 合理性——任务不可并行

编排者指令"spawn 1 个 agent"完成两个待实现项。两个实现项（compute_graph_invariants 和 is_structurally_significant）存在数据依赖——is_structurally_significant 的 active_graph_stats 参数类型依赖 compute_graph_invariants 的输出结构。这构成了串行的严格数据依赖理由（218号允许的串行条件）。此外两个函数位于同一文件，同时修改同一文件的两个位置比分给两个 agent 更高效。

与 305号观察1（三 agent 并行 spawn）对比：305号是三个独立文件（两个测试文件 + ceremony_scan.py），无数据依赖，因此并行；本轮是同一文件内的两个相关函数，有数据依赖，因此单 agent。218号的判断逻辑一致。

**四分法分类**：定理——218号的并行/串行判断规则在两轮中一致适用，是同一原则的两个实例。

### 观察2（收敛信号）：TOPOLOGICAL_RELATIONS 常量引入——概念区分显式化

新增 TOPOLOGICAL_RELATIONS frozenset（15 种关系类型）将拓扑关系与非拓扑关系（records, defines, annotates）显式区分。这是 273号（有向图范畴裁定）在代码层的进一步落地：不仅区分有向/无向，还区分拓扑/非拓扑。

与 305号观察4（保守方向原则落地）模式一致：概念层裁定→代码层常量→测试验证。TOPOLOGICAL_RELATIONS 的引入遵循了相同的三层落地链条。测试 test_non_topological_relations_excluded 验证了 defines/records 关系不进入图不变量计算。

**四分法分类**：定理——273号裁定"有向图范畴"在代码层必然需要区分哪些关系构成有向图的边，TOPOLOGICAL_RELATIONS 是其逻辑推论。

### 观察3（微观收敛）：iterative Tarjan 而非递归 Tarjan——工程选择合理

compute_graph_invariants 内部的 Tarjan SCC 使用显式调用栈（iterative）而非递归实现。谱系图的节点数可达数百（当前 330+ 区块），递归深度可能超出 Python 默认栈限制（1000）。iterative 实现避免了 RecursionError 风险。

这是正确的工程选择，但值得记录的原因是：编排者指令"纯图算法，没有概念层面的未决问题"——iterative vs recursive 确实不是概念问题，是纯工程决策，agent 自主做出了合理选择。

**四分法分类**：行动——iterative vs recursive 是不携带信息差的工程实现选择。

### 观察4（收敛信号）：活跃图/全量图双轨计算——273号 + validity 机制的交叉实现

compute_graph_invariants 在活跃图（排除 validity="invalidated" 的边）和全量图（包含所有边）上分别计算 beta_0 和 cycle_rank。这是两个已结算机制的交叉点：
- 273号有向图范畴裁定 → 图不变量的计算对象
- VALIDITY_CAPABLE_RELATIONS 机制 → 活跃图/全量图的区分依据

两个机制的交叉通过 `if rel.get("relation") in VALIDITY_CAPABLE_RELATIONS: if rel.get("validity") == "invalidated": continue` 实现。测试 test_active_vs_full_with_invalidated 验证了 invalidated 的 negates 边在活跃图中被排除、在全量图中保留。

**四分法分类**：定理——活跃图/全量图区分是 validity 机制的逻辑必然推论；在图不变量上区分两者是 273号 + validity 机制的联合推论。

### 观察5（微观收敛）：is_structurally_significant 接口预留——Phase 3 的挂载点

is_structurally_significant() 当前两条路径均返回 None（active_graph_stats 为 None → None；有 stats → 仍 None）。docstring 明确标注"Phase 3 实现时填充实际逻辑（检查边是否为 SCC 桥）"。

这是 Phase 2/Phase 3 的边界标记。接口签名 `(relation: dict, active_graph_stats: dict | None) -> bool | None` 预留了 Phase 3 需要的全部输入信息。与 305号的 invariant_status="not_computed" 类似——用显式占位而非 TODO 注释标记阶段边界。

**四分法分类**：行动——接口预留是计划中的分阶段实现策略，不携带信息差。

### 观察6（收敛信号）：测试数量从 107 → 117 的增长

305号记录 107 测试。本轮新增 10 测试后达到 117。测试分布：
- test_concept_topology_check.py：57 测试（305号时 47 + 本轮新增 10）
- test_concept_extractor.py：60 测试（305号时 60，本轮未变）

新增 10 测试覆盖了本轮新增的全部代码路径（8 个图不变量场景 + 2 个接口预留场景）。测试增长与功能增长比例匹配。与 305号观察3（58→107 增长比例合理）收敛。

**四分法分类**：行动——测试覆盖是正常工程实践。

## 规则触发/违反模式

| 规则 | 触发/违反 | 实例 |
|------|----------|------|
| 218号（Lead 并行化） | 遵守 | 单 agent spawn 有数据依赖理由（观察1），不是串行惯性 |
| 137号（格式约束） | 遵守 | 输出以格式A结尾（→ 接下来：等待编排者指令） |
| 090号（严格性） | 遵守 | 117 测试全部通过，无遗留 TODO，接口预留用显式 None 而非 pass |
| 273号（有向图范畴裁定） | 遵守 | TOPOLOGICAL_RELATIONS 区分拓扑/非拓扑关系，formalization_scope="directed_graph_only" 保持 |
| 275号（局部依赖原则） | 不适用 | 本轮是单 agent 实现，无全局排序场景 |

## 历史候选状态检查

| 候选 | 来源 | 本轮状态 |
|------|------|---------|
| 方向转换检测 | 189号 | 无新实例 |
| 任务粒度默认偏小 | 216号 | 无新实例。本轮单 agent 粒度合理 |
| 278号模式B（辩证INTERRUPT） | 278号/288号 | 无新实例。样本不变 |
| 288号审查后修复模式 | 288号/302号 | 无新实例。样本不变 |
| 302号正则格式覆盖率探查 | 302号/305号 | 无新实例（本轮未新增正则）。样本不变 |
| MCP server 异常 | 303号 | 无新实例。样本不变（2） |
| 纯理论讨论蜂群 | 258号/304号 | 无新实例（本轮是实现，非讨论）。样本不变（3） |

## 自环检查

| 本次观察 | 历史对应 | 关系 |
|---------|---------|------|
| 观察1（单 agent 合理性） | 305号观察1（三 agent 并行） | 收敛——218号判断逻辑一致（有依赖→串行，无依赖→并行） |
| 观察2（TOPOLOGICAL_RELATIONS 常量） | 305号观察4（保守方向原则落地） | 收敛——概念层裁定→代码层常量→测试验证的三层落地模式 |
| 观察3（iterative Tarjan） | 无直接对应 | 微观收敛——工程选择合理，不构成新候选 |
| 观察4（活跃图/全量图双轨） | 302号/305号 validity 机制 | 收敛——validity 机制在图不变量层的自然延伸 |
| 观察5（接口预留） | 305号 invariant_status 占位 | 收敛——显式占位而非 TODO 的阶段边界标记模式 |
| 观察6（测试增长） | 305号观察3（58→107） | 收敛——107→117 增长比例合理 |

## 总结

**全部收敛，无需 `/escalate`。**

六个观察全部与历史模式对应。无新候选产生。Phase 2 实现的最后一步（图不变量 + 接口预留）在 218号并行/串行判断、273号有向图范畴裁定、090号严格性三个维度上与前轮一致。TOPOLOGICAL_RELATIONS 常量的引入是 273号裁定从"有向图范畴"到"拓扑/非拓扑关系区分"的自然细化。

Phase 2 七步实现全部完成。下一步进入 Phase 3 讨论阶段（编排者已指定与 Gemini/Codex 讨论）。

## 下游推论

1. Phase 2 完整结束后，compute_graph_invariants 在真实谱系数据上的运行结果（当前 330+ 区块的 beta_0 和 cycle_rank）可作为 Phase 3 设计 is_structurally_significant 算法的输入参考——已知图的实际拓扑特征比理论假设更可靠
2. TOPOLOGICAL_RELATIONS 常量与 block_topology.py 中的 RELATION_TYPES 存在语义重叠但划分不同（TOPOLOGICAL_RELATIONS 是 RELATION_TYPES 的真子集）。如果后续新增关系类型，需同步更新两处——这是一个潜在的一致性维护点
3. iterative Tarjan 的实现选择在当前规模（330+ 区块）下无性能问题，但如果谱系图增长至数千节点，BFS 的 O(V+E) 复杂度保证仍然成立。无需预优化

## 边界条件

- compute_graph_invariants 依赖 read_all_relations()——如果 relations.jsonl 格式变更，图不变量计算需适配
- is_structurally_significant 当前返回 None——Phase 3 填充逻辑时需保持接口签名不变（relation: dict, active_graph_stats: dict | None -> bool | None）
- 117 测试中 TestGraphInvariants 的 8 个测试使用 topology_dir fixture 创建临时目录，不依赖真实谱系文件
- TOPOLOGICAL_RELATIONS 常量硬编码了 15 种关系类型——如果 block_topology.py 新增 validity-capable 关系，需同步更新 TOPOLOGICAL_RELATIONS

### 下游推论解决记录（v133-swarm session）

- 推论1（图不变量真实数据验证）：**resolved** — 实测结果：active_beta_0=2, active_cycle_rank=1404, full_beta_0=2, full_cycle_rank=1404。active/full 一致表明当前无 invalidated 边。beta_0=2 确认图有 2 个连通分量。cycle_rank=1404 确认环路丰富度。认识论等级：L2（真实数据验证）
- 推论2（TOPOLOGICAL_RELATIONS 整合）：**resolved** — concept_topology_check.py 第497-501行已显式说明集合关系：LOGICAL ⊂ NAVIGATIONAL ⊂ TOPOLOGICAL ⊂ RELATION_TYPES。310号观察3 记录了 defines 的集合异常（NAVIGATIONAL 包含 defines 但 TOPOLOGICAL 不包含），代码层已正确处理（测试排除 defines 后验证子集关系）。一致性维护点已标注
- 推论3（性能问题）：**deferred** — 当前规模（330+ 区块）下 iterative Tarjan O(V+E) 无性能瓶颈

### 补充记录（v138-swarm downstream-resolver）

- 推论1 状态更新：当前运行 concept_topology_check.py 报 `TypeError: 'int' object is not subscriptable`。根因：318号谱系写入 dag.yaml 时 depends_on 未强制字符串化，relations.jsonl 中出现 5 条 int 类型 from/to 字段（from=318 到 317/316/315/314/90）。v133-swarm 的 resolved 结果是在此 bug 引入前取得的。修复路径：`block_topology.py:read_all_relations` 对 from/to 做 `str()` 强制转换。当前状态：**re-blocked**（阻塞于 int 类型 bug）
