---
id: "463"
title: "GPU架构质询：步内计算粒度与CUDA Kernel启动开销的倒挂"
status: 已结算
type: 矛盾发现
resolution: 吸收——GPU架构质询的已知约束，迁移时逐个处理
negation_source: heterogeneous
negation_form: waiting
negation_model: gemini-3.1-pro-preview
created_at: "2026-03-16"
trigger: v247-swarm/gemini-gpu-challenge
depends_on:
  - "461"   # CSR 静态性矛盾
  - "462"   # LLM 伪同构矛盾
affects:
  - "topological-computation/traversal.py"  # run_step 的缓存机制
  - "topological-computation/engine.py"     # compute_beta_1 缓存
---

## 矛盾描述

**编排者洞察（待质询的主张）**：每步穿越内部的邻域查询、约束合力计算、β₁ 可以 GPU 并行化。

**Gemini 异质质询发现的矛盾**：步内实际计算量与 CUDA Kernel 启动开销严重倒挂。

## 推导链（Gemini 推理摘要）

1. `compute_beta_1` 具有 `_cached_beta_1` 永久缓存（Graph 不可变，缓存永远有效）
2. 常规步（无图变异，无 execute_encounter）的 beta_1 是 O(1) 缓存命中
3. `run_step` 中实际计算负载：
   - `detect_encounter`：O(deg) 级别的字典哈希查找
   - `neg_pairs` 构建：O(deg) 邻域扫描
   - `walk()`：随机选邻居，O(deg)
   - 以上均在 deg 数量级（当前逢亮：数十到数百）
4. CUDA Kernel 启动开销：约 5-15 微秒（PCIe 模式）
5. O(deg=数十) 的 dict lookup 在 CPU 上执行时间：约 0.1-1 微秒
6. 倒挂：GPU kernel 启动开销 >> 实际计算时间

## 判定（同质质询执行）

**否定成立**，步内计算量假设不成立。

- 定义回溯：`_cached_beta_1` 缓存机制在代码中确认（engine.py:437）；run_step 的实际计算序列确认为 O(deg) 级别
- 反例构造：当触发 execute_encounter（fold/negate/sublate）时，add_edge 产生新 Graph 实例，beta_1 缓存失效，确实有真实 O(V+E) 计算负载。但这属于 462号矛盾（BFS 不规则访存）的范畴，不是"步内并行"
- 推论检验：Encounter 触发步（有真实计算）是"全局 BFS"而不是"步内并行"；非 Encounter 步（大多数步）计算量 O(deg)，GPU 倒挂

## 补充观察：run_step 的严格串行结构

run_step 的 10 个阶段存在强数据依赖：
- neg_pairs 依赖当前 position
- detect_encounter 依赖 neg_pairs
- execute_encounter 依赖 encounter 结果
- _pull_cooccurrence_edges 依赖新 position
- _check_articulation_encounter 依赖更新后的图
- cycle detection 依赖 visit_history

**步内没有可并行化的独立子计算**——每个阶段都依赖前一阶段的输出。"步内并行"的概念本身与 run_step 的实际结构矛盾。

## 边界条件

在以下条件下 GPU 可能有意义：
- 批量步（批量执行 N 个独立的穿越实例，步之间无共享状态）
- 静态只读查询（对 k_full 历史图的大规模特征提取）

这是与"步内并行"完全不同的架构方向。

## 下游推论

如果三个矛盾均成立，编排者的 GPU 架构建议需要重新定向：
- **推荐路线**：Rust 实现 Graph 核心（adjacency list → cache-local SIMD）
- **GPU 有效域**：批量静态只读图分析（非实时穿越加速）
- **优先级调整**：Rust + PyO3 绑定先于 GPU 探索（已有任务 #8）

## 谱系引用

- 461号、462号：同轮质询互补
- 231号：形式化有效域规则——有效域可能严格小于定义域

## 来源标注

[Gemini 异质质询] gemini-3.1-pro-preview, 2026-03-16

review-results 路径：`.chanlun/review-results/gemini-genealogy-review-2026-03-16T02-04-20.md`
