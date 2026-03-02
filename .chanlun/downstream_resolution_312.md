# 312号下游推论验证汇总报告

日期：2026-03-02
验证方式：代码审读 + 运行验证（两次独立执行）

## 总览

| 序号 | 推论 | 验证方法 | 结果 |
|------|------|----------|------|
| 1 | 输入子图：references-only | 代码审读 morse_landscape.py:112 | **verified** |
| 2 | 算法：时序 Kruskal | 代码审读 morse_landscape.py:42-73,116 | **verified** |
| 3 | 临界边数量 862 | 运行 build_morse_landscape() | **verified** |
| 4 | tie-breaking: (from,to) 字典序 | 代码审读 morse_landscape.py:116 | **verified** |
| 5 | 生成树唯一且可重现 | 两次运行比较 edge_marks | **verified** |
| 6 | 全量 997 概念目标 | 运行 load_registry() | **verified** |
| 7 | 15 个 authoritative | 运行 load_registry() | **rejected** |
| 8 | 偶遇催化基于完整注册表 | 代码审读 encounter_guard.py:39-70 | **rejected** |
| 9 | DAG 守护仅 depends_on 子图 | 代码审读 encounter_guard.py:119 | **verified** |
| 10 | tie-breaking 同推论4 | 同推论4 | **verified** |

**8/10 verified，2/10 rejected。**

## rejected 推论详情

### 推论7：15 个 authoritative（实际 981 个）

312号裁定原文："15 个 `type: definition` 区块加 `authoritative: true`"。

实际状态：
- 区块拓扑中不存在 `type: definition` 的区块（0 个）
- `concept_registry.py` 的 authoritative 判定基于 defines 边的 `concept_definition` 字段是否非空
- 1106/1123 条 defines 边携带 concept_definition → 981/997 概念被标记 authoritative
- 仅 16 个概念的所有 defines 边都不携带 concept_definition，标记为非 authoritative

差异根因：裁定描述的"15 个 type: definition 区块"是讨论阶段的数据快照或设计意图，但：
1. 当前区块类型分布为 rewrite(580)、event(409)、consensus(9)、meta-rule(8) 等，无 definition 类型
2. 代码实现选择了不同的 authoritative 判定标准（边属性而非区块类型）

**影响**：authoritative 标记失去区分度（981/997 ≈ 98.4% 都是 authoritative），无法实现"15 个高权重 + 982 个普通"的分层设计。需要后续裁定：是修正 authoritative 判定逻辑，还是修正预期数字。

### 推论8：偶遇催化基于完整注册表触发（实际基于 Morse 地形）

312号裁定原文："偶遇催化基于完整注册表触发"。

实际状态：
- `encounter_guard.py` 的 `check_encounter()` 调用 `build_morse_landscape()` 全量重算
- 偶遇判定依据是新 reference 边在 Morse 地形中是否为 critical
- 概念注册表（concept_registry.py）是独立组件，check_encounter() 不导入也不调用它
- 偶遇记录写入 topo_events.jsonl，不与注册表交互

差异根因：裁定中的"基于完整注册表触发"描述的可能是设计意图（偶遇催化应该在注册表中查找匹配概念），但当前实现的偶遇检测完全基于图拓扑（Morse 地形的 critical 标记），不涉及概念语义匹配。

**影响**：如果偶遇催化需要概念语义匹配（"这条新 reference 边连接了哪些概念"），当前实现缺失这个环节。需要后续裁定：偶遇催化是否需要概念注册表参与。

## 运行数据快照

```
Morse 地形 stats:
  nodes: 296
  edges: 1156
  critical: 862
  tree: 294
  components: 2

概念注册表:
  total entries: 997
  authoritative: 981
  non-authoritative: 16

区块类型分布（前5）:
  rewrite: 580
  event: 409
  consensus: 9
  meta-rule: 8
  概念发现: 5
```
