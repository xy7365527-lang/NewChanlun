---
id: '476'
number: 476
title: "JSONL append-only 持久化与 daemon 长期运行不兼容——快照+增量架构需求"
type: domain
status: 已结算
date: 2026-03-17
source: "[新缠论] v260-swarm/genealogist 工位——编排者诊断 + 症状链分析"
depends_on:
  - '423'   # v225-swarm 元观察——候选5（S_net 持久化边界模糊）
  - '424'   # S_net 持久化缓存实装（423号候选5的 S_net 侧闭合）
  - '401'   # 轨迹 vs 产物范畴区分（memory 注入止血）
  - '178'   # block topology 迁移（声明为 primary 存储）
  - '347'   # content-addressed block topology（block = source of truth）
epistemological_level: L0
negation_form: none
negation_source: ""
topo_effect: ""
tensions_with:
  - '178'   # block topology 声明为 primary 存储，但 JSONL 实际仍是 primary——迁移断裂
---

# 476号：JSONL append-only 持久化与 daemon 长期运行不兼容

## 推导链

1. **现象观测**（v260-swarm 编排者诊断）：
   - K_full JSONL：24GB（本地）/ 18GB（VPS）
   - daemon 启动时全量 replay JSONL 进内存，耗时极长
   - 316步产生 8.5M 顶点（27K/step），99.997% 是 memory 域
   - 穿越在 `memory:residue:expression_pressure` 区域打转

2. **迁移断裂识别**：
   - 178号（2026-02-24）声明 block topology 为 primary 存储
   - 347号（2026-03-04）升级为 content-addressed block topology（block_id = SHA256(full_text)）
   - 但 K_active/K_full 的 **运行时持久化** 仍然依赖 JSONL append-only（`persistence.py` 中的 `PersistentKFull`）
   - block topology 管辖的是谱系/概念层的不可变记录，不是运行时图状态
   - **断裂点**：block topology 声明了 primary 地位，但运行时图的持久化机制未迁移——两套存储并存，JSONL 作为运行时实际 primary

3. **JSONL append-only 的结构性缺陷**：
   - append-only 设计使得文件只增不减
   - 无 compaction/snapshot 机制：历史增量永远保留
   - 启动时 replay = 线性扫描全量历史 → O(N) 启动时间随运行时间线性增长
   - 长期运行后文件膨胀到 GB 级别 → 内存消耗与文件大小成正比

4. **424号只解决了 S_net 侧**：
   - S_net 持久化缓存（`snet_cache.py`）：三路分支（完全命中/增量/全量），启动从 30-60min 降至数秒
   - K_active/K_full JSONL 持久化：**未触及**
   - 这是 423号候选5（"持久化边界模糊"）的 **K 侧残留**

5. **与 401号的关系**：
   - 401号解决了 memory 注入的 **范畴错误**（轨迹不是产物）
   - 但 JSONL 中已有的历史膨胀数据 + JSONL 架构本身的无限增长问题 ≠ 范畴错误
   - 即使 401号完美执行（不再注入垃圾 memory 节点），JSONL 仍会随时间无限增长（每步的合法 diff 仍然追加）

## 症状链（完整）

```
daemon 长期运行
  → K_full JSONL append-only 持续增长（每步写入合法 diff）
    → 文件膨胀到 24GB/18GB
      → 启动时 replay 24GB JSONL → 内存 + 时间爆炸
        → 8.5M 顶点重建进内存
          → 穿越在 memory:residue:expression_pressure 打转
            → 系统穿越自己的副产物（与 terror_moral_worldview 循环同构）
```

## 解决方案方向（非实装——方向记录）

**snapshot + 增量 JSONL 循环**：

| 步骤 | 操作 | 效果 |
|------|------|------|
| 1 | 定期将 K_active/K_full 当前状态 dump 为 snapshot 文件 | 全量快照，类似 424号 S_net 缓存的 pickle.gz |
| 2 | snapshot 写入成功后，truncate 对应 JSONL | JSONL 从零开始，只记录 snapshot 之后的增量 |
| 3 | 启动时先加载 snapshot，再 replay 增量 JSONL | O(1) snapshot + O(delta) replay，替代 O(N) 全量 replay |

这与 424号 S_net 缓存的三路分支策略同构：完全命中 = snapshot 新鲜，增量 = snapshot + delta，全量 = 无 snapshot。

## 谱系链接

- **423号候选5**（持久化边界模糊）→ 424号闭合了 S_net 侧 → 本号识别 K 侧残留
- **401号**（轨迹 vs 产物）→ 止血了 memory 注入膨胀 → 但未止住 JSONL 架构本身的增长
- **178号/347号**（block topology）→ 声明 block 为 primary → 运行时 JSONL 仍是实际 primary → 迁移断裂

## 27K/step memory vertex 退化的归属

**暂不创建独立谱系**。理由：

1. 401号已修复了 encounter memory 注入（30863 → 224 memory 节点）
2. 当前 27K/step 的数据来自 JSONL replay 的历史数据——可能是 401号修复前的历史残留
3. 如果 snapshot + truncate 止血后 27K/step 问题消失 → 确认是历史残留
4. 如果 snapshot + truncate 止血后 27K/step 问题仍存 → 说明存在 401号之后的新膨胀路径 → 此时创建独立谱系

**处置**：标记为"止血后观测"——persist-snapshot 工位的实装结果将决定此问题是否需要独立谱系。

## 边界条件

| 条件 | 当前 | 翻转阈值 |
|------|------|----------|
| JSONL 增长是否可接受 | 不可接受（24GB） | snapshot + truncate 后 JSONL 保持在 MB 级别 |
| 27K/step 是否是独立问题 | 待观测 | persist-snapshot 止血后如仍 27K/step → 独立谱系 |
| block topology 能否替代 JSONL | 尚不能（block topology 管谱系，不管运行时状态） | block topology 扩展为运行时状态管理 |

## 影响声明

- 创建 476号谱系（domain，已结算——问题和方向已明确，实装由 persist-snapshot 工位执行）
- 423号候选5 的 K 侧残留被识别并记录
- 与 178号/347号 存在张力（迁移断裂）——block topology 声明 primary 但运行时 JSONL 仍是实际 primary
- 27K/step 退化标记为"止血后观测"，不创建独立谱系

## 谱系关联

related_records:
  parent: '423'   # 候选5 的来源（S_net + K 两侧持久化边界）
  siblings: ['424']   # S_net 侧的闭合
  children: []   # persist-snapshot 工位实装后可能产出
