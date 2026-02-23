# 否定类型→拓扑操作 工程实现设计

## 概述

本文档设计 147号谱系（矛盾具有拓扑效力）的工程实现方案。核心：否定事件触发时，根据否定形式（040号）自动执行对应的拓扑操作。

## 现有基础设施

### dispatch-dag.yaml 中的 topology-mutator（141号已声明）

```yaml
- id: topology-mutator
  skill_type: structural
  agent: ".claude/agents/topology-manager.md"
  purpose: "检测到否定事件时执行拓扑操作"
  triggers:
    - event: genealogy_settlement
      condition: "新结算谱系包含 negates 边或 topo_effect 标注"
```

topology-mutator 已在 event_skill_map 中声明，但其具体的操作逻辑尚未定义。

### dag_add_node.py 现有接口

```
python scripts/dag_add_node.py --id 143 --title "标题" --type "定理" \
  --file "settled/143-xxx.md" \
  --depends_on 140,141 --related 069,093 --negates 042
```

已支持：添加节点、depends_on 边、related 边、tensions_with 边、negates 边。
缺少：冻结路径、分裂节点、切断连接的拓扑操作。

## 设计方案

### 1. 谱系文件中的 topo_effect 标注

每条携带否定边的谱系文件，在 YAML front matter 中增加 `topo_effect` 字段：

```yaml
---
id: "148"
negation_form: "waiting"
topo_effect:
  type: "freeze"           # freeze | split | sever
  target: "148"            # 被操作的节点/边 ID
  scope: "downstream"      # downstream | node | edge
---
```

三种 topo_effect 类型与 040号否定形式的对应：

| negation_form | topo_effect.type | 操作语义 |
|---------------|-----------------|---------|
| waiting | freeze | 标记目标节点的所有下游边为 frozen，ceremony_scan 跳过 frozen 路径 |
| expansion | split | 目标节点分裂为两个新节点（原ID-a, 原ID-b），各自保留部分边 |
| separation | sever | 将目标节点的某条连接边切断，产生两条独立路径 |
| unclassified | (无自动操作) | 作为生成态谱系条目存在，等待系统递归运动辨认其形式 |

### 2. dag_add_node.py 扩展

在现有 `dag_add_node.py` 基础上增加拓扑操作能力：

```
python scripts/dag_add_node.py --id 148 --title "标题" --type "语法记录" \
  --file "settled/148-xxx.md" \
  --depends_on 147 \
  --topo_effect "freeze:148:downstream"
```

`--topo_effect` 参数格式：`{type}:{target_id}:{scope}`

实现逻辑：
- **freeze**：在 dag.yaml 的目标节点上添加 `frozen: true` 标记，在所有以该节点为起点的 depends_on 边上添加 `frozen_by: "148"`
- **split**：将目标节点复制为两个（{id}-a, {id}-b），原来的入边连向两者，原来的出边根据谱系描述分配
- **sever**：在指定边上添加 `severed_by: "148"`，ceremony_scan 不再沿此边传播

### 3. dag.yaml 中的冻结/切断标记

dag.yaml edges 部分已有 `valid_until` 字段（132号引入）。拓扑操作在同一 schema 上扩展：

```yaml
edges:
  depends_on:
    - from: "040"
      to: "029"
    - from: "148"
      to: "040"
      frozen_by: "149"    # 被 149号谱系冻结
      frozen_at: "2026-02-24"
```

```yaml
nodes:
  - id: "050"
    title: "..."
    frozen: true           # 被冻结——下游路径不可推进
    frozen_by: "149"
```

### 4. ceremony_scan.py 整合

ceremony_scan.py 在扫描谱系时需要：

1. 读取每条新结算谱系的 `topo_effect` 字段
2. 如果存在 topo_effect，调用 topology-mutator 执行对应操作
3. topo_effect 执行结果记录在 ceremony_scan 输出的 `topo_mutations` 字段中
4. 冻结的路径在后续 ceremony 中被跳过（不产生工位）
5. 分裂的节点在后续 ceremony 中作为两个独立节点处理

### 5. 084-4 实现：基因组修改触发谱系写入 + 异步审查

当 Lead 修改基因组文件（CLAUDE.md、dispatch-dag.yaml、ceremony_scan.py）时：

1. **现有机制**：`definition-write-guard.sh`（PreToolUse hook）已在监控这些文件的写入
2. **新增行为**：hook 检测到基因组文件修改后，在 `systemMessage` 中注入提示："基因组修改已触发——请将本次修改作为分离型否定写入谱系（147号拓扑效力）"
3. **谱系写入**：Lead 将修改写入一条新的谱系条目，`negation_form: separation`，`topo_effect: { type: sever, ... }`
4. **异步审查**：该谱系条目进入下一轮 ceremony_scan 的扫描范围，如果审查发现与已结算谱系矛盾，矛盾作为新的生成态谱系条目存在

关键：审查结果**不回到编排者**，而是回到系统的下一轮 ceremony_scan。编排者看谱系，但不裁决。

### 6. 与现有 topology-mutator 的衔接

topology-mutator 已在 dispatch-dag event_skill_map 中声明（141号）。147号为其定义了具体的操作映射。

topology-mutator 的执行流程：
1. genealogy_settlement 事件触发
2. 检查新结算谱系是否携带 `negates` 边或 `topo_effect` 标注
3. 如果有 `topo_effect`，执行对应的拓扑操作（调用 dag_add_node.py 的扩展接口）
4. 如果只有 `negates` 边但无 `topo_effect`，检查 `negation_form` 推断拓扑操作类型
5. 操作结果写入 dag.yaml

## 实现优先级

| 阶段 | 内容 | 依赖 |
|------|------|------|
| P0 | dag.yaml 中 frozen/severed 标记的 schema 定义 | 无 |
| P0 | dag_add_node.py 增加 --topo_effect 参数 | 无 |
| P1 | ceremony_scan.py 读取并执行 topo_effect | dag.yaml schema |
| P1 | definition-write-guard.sh 增加基因组修改拓扑效力提示 | 147号谱系 |
| P2 | split 操作的完整实现（节点分裂逻辑较复杂） | dag_add_node.py 扩展 |

## 不需要的

- 不需要新的 agent 文件——topology-mutator 复用 topology-manager.md 的 agent 指令
- 不需要编排者审批流程——这是 147号的核心否定点
- 不需要回滚机制——拓扑操作本身是否定事件，逆操作也是否定事件，走同样的路径
