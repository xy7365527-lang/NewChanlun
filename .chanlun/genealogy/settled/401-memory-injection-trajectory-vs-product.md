---
id: "401"
number: "401"
type: meta-rule
status: settled
date: 2026-03-08
title: "脚印不是宝藏 — 轨迹与产物的范畴区分"
rule_version_baseline:
  claude_md_commit: "09bc3999062d55d369dde9ebedd37c9efe4ce0e3"
  rules_dir_mtime: "2026-03-04 19:53:04 +0000"
parent: ["396"]
---

# 401号 脚印不是宝藏 — K_active 中轨迹与产物的范畴区分

## 发现

encounter_log 将每一步穿越的 fold/sublate/negate 操作记录为 K_active 中的 domain:memory 节点。
98020 条 JSONL 记录 → 30863 个 memory 节点淹没 225 个 core 概念节点。
beta_1 从 131 膨胀到 62819。所有 fold/negate 目标被 blocked。

## 范畴区分（语法记录）

**轨迹（trajectory）**：穿越过程中发生的事件序列。每步的 fold/sublate/negate 是穿越走过的路，不是穿越发现的东西。
- 存储位置：traversal-events.jsonl（append-only 事件历史）、visit_history
- 不进入 K_active

**产物（product）**：穿越产出的结构性事件。改变了概念图的拓扑结构。
- settlement（裁决）、residue（边界/张力/Nachtraglichkeit）、code_settlement_request
- 进入 K_active 作为 domain:memory 节点

编排者原话："你走路时留下的脚印不是你发现的宝藏——如果你把每个脚印都当宝藏捡起来放进背包，你很快就走不动了。"

## 幽灵 settlement（下游推论）

memory 膨胀期间产生的 settlement 保护了包含 memory 节点的 cycles。清除 memory 节点后，这些 settlement 变成幽灵——保护的 cycle 不存在了，但 settlement 仍然 block 操作。

- 本地：201 个 settlement 全部是幽灵，清除后 0 个保留
- VPS：76 个 settlement 全部是幽灵，清除后 0 个保留

修复：SettlementTracker.purge_invalid_cycles() 在启动时清除 cycle 边不完整的 settled cycles。

## 修复

1. encounter_log.py: rebuild_memory_from_jsonl() 不再注入 encounter memory 节点
2. daemon.py: 实时路径不再将 encounter 注入 K_active
3. daemon.py: 启动时 _purge_encounter_memory_nodes() 清除已有 encounter 节点
4. engine.py: purge_invalid_cycles() 清除幽灵 settlement
5. traversal.py: sublate 调用加 try/except 防御 negation edge race

## 修复后数据

| 指标 | 修复前 | 修复后 |
|------|--------|--------|
| vertices | 31088 | 442 |
| memory | 30863 | 224 |
| beta_1 | 62819 | 189 |
| settled | 201 (全幽灵) | 2 (真实) |
| fold | 0 (blocked) | 6 |
| negate | 0 (blocked) | 4 |

## 元规则

**轨迹不是产物**是一条范畴规则，与 005b 号（对象否定对象）属于同一层级。
将轨迹当成产物注入概念图 = 范畴错误 = 与 222/223/230 号的有效域膨胀同构。

## 谱系引用

- 396号：settlement 热寂定理（本修复的前置诊断）
- 222号：有效域膨胀的第一例
- 090号：严格性语法规则
- 005b号：对象否定对象（范畴规则先例）
