---
id: '479'
number: 479
title: "memory 顶点全量清理——从 encounter 子集扩展为 memory: 全域"
type: domain
status: 已结算
date: 2026-03-17
source: "[新缠论] v261-swarm——P0 正反馈循环修复（启动时 + 持久化层）"
depends_on:
  - '480'   # settlement 产物类型约束完整覆盖（480=478扩展，redirect from 478）
  - '401'   # 脚印不是宝藏——轨迹与产物的范畴区分
  - '476'   # JSONL append-only 与长期运行不兼容
epistemological_level: L0
negation_source: homogeneous
negation_form: expansion
negates: '401'
topo_effect: "split:401:downstream"
tensions_with: []
---

# 479号：memory 顶点全量清理——从 encounter 子集扩展为 memory: 全域

## 推导链

1. **401号的清理范围**：
   - 401号实装了 `_purge_encounter_memory_nodes()`：启动时清除 `memory:encounter:` 前缀的顶点
   - 保留了 `memory:settlement:` 和 `memory:residue:` 前缀的顶点
   - 理由：encounter 是轨迹（应清除），settlement/residue 是产物（应保留）

2. **478号发现产物也不应保留在图中**：
   - settlement/residue memory 节点参与 boundary_edge 扫描 → 正反馈循环
   - 478号在运行时层（engine.py/traversal.py）植入了类型约束过滤
   - 但历史遗留的 8.5M memory 顶点仍存在于 JSONL 和 snapshot 中

3. **清理范围必须扩展为全量**：
   - 启动时：`_purge_encounter_memory_nodes()` → `_purge_memory_nodes()`
   - 过滤条件从 `memory:encounter:` 前缀 → `memory:` 前缀（全域）
   - 持久化层：`dump_snapshot()` 同步过滤 memory: 前缀顶点，防止清理后的 memory 节点通过 snapshot 复活

4. **8.5M 历史遗留的归属**：
   - 476号观测到 316步产生 8.5M 顶点（27K/step），99.997% 是 memory 域
   - 这些是 401号修复前正反馈循环的病理产物
   - 全量清理是一次性止血——清除后，478号的运行时类型约束防止新的 memory 顶点参与循环

## 代码变更（启动时 + 持久化层：daemon.py / persistence.py）

| 文件 | 位置 | 变更 | 作用 |
|------|------|------|------|
| daemon.py | L79-111 | `_purge_encounter_memory_nodes` → `_purge_memory_nodes`，过滤 `memory:` 全域 | 启动时一次性清除全部历史遗留 memory 顶点 |
| daemon.py | L467-468 | 调用 `_purge_memory_nodes(self.k_active)` 和 `_purge_memory_nodes(self.k_full)` | K_active 和 K_full 同步清理 |
| persistence.py | L226-246 | `dump_snapshot()` 跳过 memory: 前缀顶点和关联边 | 防止 memory 节点通过 snapshot 持久化后在重启时复活 |

## 与 478/480号的分工

| 层级 | 谱系 | 文件 | 职责 |
|------|------|------|------|
| 运行时层（3处） | 478号 | engine.py, traversal.py | 阻断正反馈循环源头——settlement/encounter/穿越跳过 memory: 顶点 |
| 运行时层（4处补全） | 480号 | engine.py, traversal.py, encounter_log.py, daemon.py | 478号遗漏路径堵漏 |
| 启动时 + 持久化层 | 479号（本号） | daemon.py, persistence.py | 清除历史遗留 + 防止 snapshot 复活 |

478+480号是免疫机制（防止新感染，7路径完整覆盖），479号是清创手术（清除已有感染 + 防止复发）。
本号 depends_on 从 478 redirect 至 480（480 是 478 的严格超集，传递依赖保留 478）。

## 边界条件

| 条件 | 当前 | 翻转阈值 |
|------|------|----------|
| 全量清理是否过度 | 否——478号已证明所有 memory: 顶点都不应参与运行时操作 | 如果未来引入需要持久化的 memory 类型，需在 dump_snapshot 中区分 |
| snapshot 过滤是否与 JSONL replay 一致 | 是——两者都跳过 memory: 前缀 | 如果 JSONL replay 逻辑变更，需同步 snapshot 过滤 |

## 影响声明

- daemon.py 的启动清理从 encounter 子集扩展为 memory: 全域
- persistence.py 的 snapshot 同步过滤 memory: 前缀
- 8.5M 历史遗留 memory 顶点在首次启动时被清除
- 与 478号构成完整的类型约束体系：运行时防新增 + 启动时清历史 + 持久化防复活

## 谱系关联

related_records:
  parent: '480'   # 类型约束完整覆盖（480=478超集）——本号是其启动时/持久化层的对应
  siblings: ['401', '476']   # 401号的扩展 + 476号的止血
  children: []
