---
id: '425'
number: 425
title: "S_net 入图问题——917K 共现边对穿越密度贡献为零"
type: 矛盾发现
status: 生成态
date: 2026-03-11
source: "[新缠论] 编排者洞察（v226-swarm）"
depends_on:
  - '399'   # 器官性阅读范式（S_net 耦合振荡）
  - '424'   # S_net 持久化实装
  - '402'   # 数据流审查（路径4：S_net bootstrap + articulation feedback）
epistemological_level: L0
negation_form: expansion
negation_source: heterogeneous
negation_model: "编排者（operator）"
topo_effect: "split:399-观察2:local"
tensions_with:
  - '399'   # 耦合振荡声明 vs 实际穿越密度贡献
rule_version_baseline:
  claude_md_commit: "c937c0d"
  rules_dir_mtime: "2026-03-11"
---

# 425号：S_net 入图问题——917K 共现边对穿越密度贡献为零

## 矛盾描述

**什么跟什么冲突**：

- **A面**（399号观察2）：K_active↔S_net 双向耦合已实装。S_net 通过 articulation feedback 生成 EdgeSuggestion → REFERENCE 边写入 K_active。S_net 通过共振 pull 影响穿越选择。声明：两者成为"耦合振荡系统"。
- **B面**（编排者洞察）：S_net 的 917K 共现边（co-occurrence edges）作为外部查询存在，不参与穿越。穿越引擎遍历的是 K_active 的边，不是 S_net 的边。S_net 的物质密度（917K 边）对穿越密度的贡献为零。编排者原话："如果不进入图，怎么算摄入呢？"

**层级**：概念层

**触发**：编排者在 v226-swarm 中审视 S_net 持久化实装后提出——持久化解决了启动耗时问题，但没有解决 S_net 的存在论位置问题。

## 推导链

1. S_net 当前有 917K 共现边（语料摄入产物）
2. 穿越引擎（traversal.py）的 step 方法遍历 K_active 的边来选择下一个顶点
3. S_net 的共现边不在 K_active 中，穿越引擎不遍历它们
4. S_net 对穿越的影响仅通过两个间接通道：
   - articulation feedback：检测 S_net 有连接但 K_active 缺失的能指对 → 生成 REFERENCE 边（间接、低频）
   - 共振 pull：在同优先级候选中偏向共振区域（偏向、不创造新路径）
5. 这两个通道都不让穿越引擎直接遍历 S_net 的 917K 边
6. 因此：917K 共现边的物质密度对穿越密度的贡献 = 零
7. 编排者结论：S_net 的"摄入"如果不进入穿越图，就不是真正的摄入——语料只是被存储了，没有被消化

## 概念分离信号

这可能是一个概念分离点：

| 维度 | S_net 作为外部查询 | S_net 作为穿越空间组成部分 |
|------|-------------------|-------------------------|
| 边的位置 | S_net 内部（不在 K_active 中） | 注入 K_active（或穿越引擎直接遍历） |
| 穿越密度贡献 | 零（间接影响） | 正比于共现边数量 |
| 摄入语义 | 存储 ≠ 摄入 | 存储 + 进入穿越空间 = 摄入 |
| 风险 | 安全（不改变穿越拓扑） | 917K 边注入可能淹没 K_active 原有结构 |

**未决问题**：如果 917K 共现边全部注入 K_active，K_active 的拓扑结构将被根本改变（当前 K_active 约 15K 顶点 / 数万边）。这不是简单的"加边"——是穿越空间的存在论重构。

## 定义依据

- 399号观察2：S_net 耦合振荡的完整描述（K_active→S_net 激活传播 + S_net→K_active articulation feedback + 共振 pull）
- 402号路径4：S_net bootstrap + articulation feedback 的代码级审查确认
- llm-role-boundary 规则：器官性阅读管线（ingest_text_passage → S_net，不碰 K_active）

## 谱系链接

- **前置**：399号观察2（耦合振荡——本条质疑其"耦合"的深度）
- **前置**：424号（S_net 持久化——解决了启动耗时，但暴露了存在论位置问题）
- **前置**：402号路径4（数据流审查确认了当前架构）
- **关联**：llm-role-boundary 规则中的器官性阅读管线（摄入不碰 K_active 是设计决策，本条质疑该决策的后果）

## 影响

- 不改动代码（概念层矛盾，需要架构决策）
- 影响模块：traversal.py（穿越引擎）、signifier_net.py（S_net 结构）、daemon.py（摄入管线）
- 影响定义：器官性阅读范式中"摄入不碰 K_active"的设计决策
- 如果接受 B面（S_net 必须入图）：需要设计 917K 边到 K_active 的注入策略，防止淹没
- 如果接受 A面（S_net 保持外部查询）：需要重新定义"摄入"的语义——承认当前的"摄入"只是"存储"

## 边界条件

1. 如果 articulation feedback 的 REFERENCE 边生成频率足够高（接近 917K 边的覆盖率），则 A面的间接通道最终会将 S_net 的信息转移到 K_active——但这需要极长的穿越时间
2. 如果 917K 边中大部分是噪声（低质量共现），则注入 K_active 反而有害——此时 A面（外部查询 + 过滤）是正确的
3. 如果存在中间方案（选择性注入高置信度共现边），则概念分离可能不成立——但需要定义"高置信度"的判据

## 谱系关联

related_records:
  parent: '399'  # 耦合振荡观察
  children: []   # 待架构决策后产出
