---
id: '480'
number: 480
title: "memory 泄漏四路径堵漏——478号类型约束的遗漏路径补全"
type: domain
status: 已结算
date: 2026-03-17
source: "[新缠论] v262-swarm——memory 泄漏堵漏 + autocompact 实装"
depends_on:
  - '478'   # settlement 产物类型约束（运行时层——engine.py / traversal.py 三处过滤）
  - '479'   # memory 顶点全量清理（启动时 + 持久化层）
epistemological_level: L0
negation_source: homogeneous
negation_form: expansion
negates: '478'
topo_effect: "split:478:local"
tensions_with: []
---

# 480号：memory 泄漏四路径堵漏——478号类型约束的遗漏路径补全

## 推导链

1. **478号覆盖了三条主路径**：
   - engine.py `_compute_residue()` 的 boundary_edge 扫描
   - traversal.py encounter 检测
   - traversal.py 穿越步进的邻居过滤

2. **四条遗漏路径被发现**：
   478号的类型约束（memory: 前缀顶点不参与运行时操作）正确，但实施不完整——同一约束在代码中有多个施加点，478号只覆盖了三个。以下四条路径未被过滤：

   a. **engine.py `backfill_residue()`**：daemon 启动时为历史 settled cycle 补充 residue。boundary_edge 计算时未过滤 memory: 前缀目标顶点。与 `_compute_residue()` 的 boundary_edge 逻辑同构，但 478号只修了后者。

   b. **traversal.py `_try_residue_escape()`**：穿越被 settled cycle 阻塞时，从 residue 中提取 escape 目标。boundary_targets 和 nachtraeg_targets 均未过滤 memory: 前缀顶点——穿越可能跳入 memory 节点，触发后续正反馈。

   c. **encounter_log.py `inject_settlement_memory_node()`**：注入 settlement memory 节点时，referenced_vids 收集 residue 引用的概念顶点。未过滤 memory: 前缀——导致 settlement 产物通过 REFERENCE 边自引用其他 memory 节点，形成 memory→memory 循环。

   d. **daemon.py `_inject_cross_domain_incremental()`**：增量跨域边检测扫描 new_vids 时未过滤 memory: 前缀——memory 顶点参与跨域匹配会产生虚假的跨域边。

3. **与 478号的关系**：
   - 478号确立了正确的类型约束：memory: 前缀 ∈ 输出域，不属于输入域
   - 480号将同一约束扩展到 478号遗漏的四条路径
   - 同一个不变量（产物不回流为输入），更完整的实施

## 代码变更

| 文件 | 位置 | 变更 | 作用 |
|------|------|------|------|
| engine.py | `backfill_residue()` L1282-1288 | boundary_edge 目标加 `not e.target.startswith("memory:")` 过滤 | 启动时 backfill 不把 memory 节点纳入 boundary residue |
| traversal.py | `_try_residue_escape()` L1007 | boundary_targets 加 `not ev.startswith("memory:")` 过滤 | escape 不跳入 memory 节点 |
| traversal.py | `_try_residue_escape()` L1025 | nachtraeg_targets 加 `not v.startswith("memory:")` 过滤 | nachtraeglichkeit escape 不跳入 memory 节点 |
| encounter_log.py | `inject_settlement_memory_node()` L316 | referenced_vids 集合过滤 `not v.startswith("memory:")` | settlement 产物不通过 REFERENCE 边引用其他 memory 节点 |
| daemon.py | `_inject_cross_domain_incremental()` L1849 | new_vids 集合过滤 `not v.startswith("memory:")` | memory 顶点不参与跨域边检测 |

## 478号类型约束的完整覆盖表（478+480 联合）

| 路径 | 文件 | 谱系 |
|------|------|------|
| `_compute_residue()` boundary_edge | engine.py | 478号 |
| encounter 检测 | traversal.py | 478号 |
| 穿越步进邻居 | traversal.py | 478号 |
| `backfill_residue()` boundary_edge | engine.py | **480号**（本号） |
| `_try_residue_escape()` boundary + nachtraeg | traversal.py | **480号**（本号） |
| `inject_settlement_memory_node()` referenced_vids | encounter_log.py | **480号**（本号） |
| `_inject_cross_domain_incremental()` new_vids | daemon.py | **480号**（本号） |

## 边界条件

| 条件 | 当前 | 翻转阈值 |
|------|------|----------|
| 是否还有遗漏路径 | 478+480 联合覆盖了所有已知的 memory: 顶点消费路径 | 如果新增消费 K_active 顶点的代码路径，需检查是否需要 memory: 过滤 |
| 过滤条件是否一致 | 全部使用 `startswith("memory:")` | 如果 memory 前缀命名规则变更，需同步更新所有 7 处过滤 |

## 下游推论

- 478号的类型约束现在完整覆盖所有已知路径——正反馈循环的免疫机制无缝隙
- 新增的 K_active 消费路径应作为约定检查 memory: 前缀过滤（防御性编程模式）

## 影响声明

- engine.py / traversal.py / encounter_log.py / daemon.py 各增加 1 处 memory: 前缀过滤
- 正反馈循环的 4 条遗漏泄漏路径被堵住
- 与 478号（3 处）、479号（启动+持久化）构成完整的三层防御：运行时 7 路径过滤 + 启动时全量清理 + 持久化层过滤

## 谱系关联

related_records:
  parent: '478'   # 类型约束（运行时层）——本号是其遗漏路径的补全
  siblings: ['479']   # 启动时+持久化层（另一个维度的补全）
  children: []
