---
id: "465"
title: "GPU架构质询R2：detect_encounter碎片化访存与Python-GPU往返延迟倒挂"
status: 已结算
type: 矛盾发现
resolution: 吸收——GPU架构质询的已知约束，迁移时逐个处理
negation_source: heterogeneous
negation_form: waiting
negation_model: gemini-3.1-pro-preview
created_at: "2026-03-16"
trigger: v247-swarm/gemini-gpu-r2-challenge
depends_on:
  - "463"   # 步内计算量倒挂（R1，同一质询链）
  - "462"   # LLM 伪同构矛盾（R1）
affects:
  - "topological-computation/traversal.py"  # detect_encounter, run_step
---

## 矛盾描述

**编排者洞察（待质询的主张）**：每步穿越内部的邻域查询可以 GPU 并行化，Python 做步间决策。

**Gemini 异质质询发现的矛盾**：`detect_encounter` 的高度串行、碎片化查询结构，导致 Python-GPU 往返开销远超计算收益。

## 推导链（Gemini 推理摘要）

1. `detect_encounter` 的执行需要多个**上下文依赖的图查询**，在单步内形成链式依赖：
   - 查询当前位置邻居 → GPU 返回 Python
   - 过滤合成顶点（Python 端字符串判断）
   - 检查双向边和 terrain 标记（多次 GPU 查询）
   - 计算 `f_val`（局部 lower_link 的连通分量 BFS）
   - 检查过去 15 步 visit_history 的共享邻居
2. 每个查询依赖上一个查询的结果（不可批量化）
3. 在 CPU 上对数千顶点规模：每步 < 1 微秒
4. 在 GPU 上：每步触发数十次 Python-GPU 往返（PCIe 延迟 ~1-2μs/次）+ Kernel 启动（~5-10μs/次）= 数百微秒
5. 倒挂比例：GPU 慢 100-1000 倍

## 判定（同质质询执行）

**否定成立**，步内 GPU 化方案在碎片化访存结构下是纯粹负优化。

- 定义回溯：`traversal.py` 中 `detect_encounter` 的查询链与 Gemini 描述一致（`_step_neg_pairs`、`terrain`、`f_val`、`visit_history`）
- 反例构造：若将全部决策逻辑 CUDA 化（消除往返）——但这要求 Python 字符串逻辑、visit_history、blocked_at 全部移到 GPU，难度等同完全重写
- 推论检验：即使批量化部分查询，链式依赖仍无法消除往返——与 463号（步内串行）矛盾互补

## 边界条件

若 detect_encounter 的查询逻辑大幅简化（仅做邻域查询，不依赖历史和 terrain），往返次数可以减少。
当前实现的查询复杂度是逢亮拓扑语义的必然要求，不可随意简化。

## 下游推论

Python-GPU 往返问题表明，GPU 加速需要"整体移植"（全部计算迁移到 GPU，Python 仅做控制流），而不是"部分查询 GPU 化"。整体移植的代价与 Rust 完全重写等级相当。

## 谱系引用

- 463号：步内计算量倒挂（R1——计算量 O(deg) << kernel 启动开销）
- 462号：LLM 伪同构——步内并行类比的概念渗入
- 436号：概率范式与拓扑穿越范式概念分离

## 来源标注

[Gemini 异质质询] gemini-3.1-pro-preview, 2026-03-16

review-results 路径：`.chanlun/review-results/gemini-genealogy-review-2026-03-16T02-14-21.md`
