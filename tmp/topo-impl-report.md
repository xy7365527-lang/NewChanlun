# 147号下游推论实现报告

**工位**: topo-impl (v148-swarm)
**日期**: 2026-02-23

## 147-1: topology-mutator 验证 — resolved

**结论**: topology-mutator 已完整实现，无需额外修改。

**现有实现**:
- `dispatch-dag.yaml` 第257-264行: topology-mutator skill 定义完整
  - skill_type: structural
  - agent: topology-manager.md（复用拓扑管理 agent）
  - trigger: genealogy_settlement 事件
  - condition: "新结算谱系包含 negates 边或 topo_effect 标注"
- `dag_add_node.py` 第81-139行: `--topo_effect` 参数支持 freeze/split/sever 三种操作
  - freeze: 在目标节点上标记 `frozen: true`，downstream scope 时冻结相关边
  - split: 创建 target-a 和 target-b 分裂节点
  - sever: 在目标节点相关边上标记 `severed_by`

**边界条件**: 当前 topology-mutator 的触发依赖 LLM 解释执行（D策略082号），不是 hook 自动触发。这与 dispatch-dag 的事件驱动设计一致。

## 147-2: ceremony_scan topo_effect 整合 — resolved

**修改**: `scripts/ceremony_scan.py`

新增两个函数:
1. `get_frozen_nodes(root)`: 读取 `dag.yaml` 中 `frozen: true` 的节点 id 集合
2. `get_topo_effects_from_genealogy(root)`: 扫描已结算谱系文件的 `topo_effect` 字段

在 main 扫描流程中新增:
- frozen 节点过滤: 工位名称中包含 frozen 节点 id 的工位不 spawn
- topo_effects 汇总: scan 输出中包含 `topo_effects` 和 `frozen_nodes` 字段

**向后兼容**: 如果 dag.yaml 中没有 frozen 标记，行为不变（`frozen_nodes` 为空集，不触发过滤）。

**验证**: `python ceremony_scan.py --skills` 和全量扫描均通过，topo_effects 正确输出。

## 147-3: 谱系写入 topo_effect 标注 — resolved

**修改**: `.claude/hooks/genealogy-write-guard.sh`

新增检查逻辑（第230-268行）:
- 解析待写入内容的 `negates` 字段（支持行内数组和多行格式）
- 解析 `topo_effect` 字段是否存在
- 如果 `negates` 非空但 `topo_effect` 缺失，输出 advisory 警告
- 警告文本: "147号 advisory: 谱系声明了 negates 但缺少 topo_effect 标注（应标注冻结/分裂/切断之一）"

**原则0 兼容**: 守卫只发出 advisory（`decision: allow`），不阻断写入。

## 147-4: lead-permissions 重定义 — resolved

**修改**: `.chanlun/downstream-action-overrides.yaml`

新增 147号条目:
- 147-1: resolved (topology-mutator 完整)
- 147-2: resolved (ceremony_scan 整合)
- 147-3: resolved (genealogy-write-guard advisory)
- 147-4: resolved (084-4 认识论转化——权限问题→拓扑问题)

## 影响声明

- 修改: `scripts/ceremony_scan.py`（+54行: 两个新函数 + 主流程集成）
- 修改: `.claude/hooks/genealogy-write-guard.sh`（+40行: topo_effect advisory 检查）
- 修改: `.chanlun/downstream-action-overrides.yaml`（+6行: 147号四条标注）
- 新建: `tmp/topo-impl-report.md`（本报告）
