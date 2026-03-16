---
id: "464"
title: "GPU架构质询R2：CSR fold边重定向与标记无效的物理死锁"
status: 已结算
type: 矛盾发现
resolution: 吸收——GPU架构质询的已知约束，迁移时逐个处理
negation_source: heterogeneous
negation_form: waiting
negation_model: gemini-3.1-pro-preview
created_at: "2026-03-16"
date: 2026-03-16
trigger: v247-swarm/gemini-gpu-r2-challenge
depends_on:
  - "461"   # CSR 静态性矛盾（R1，同一质询链）
  - "231"   # 形式化有效域规则
affects:
  - "topological-computation/engine.py"  # merge_vertices, fold
---

## 矛盾描述

**编排者洞察（待质询的主张）**：fold 时标记顶点无效不重建 CSR，增量更新只追加到末尾缓冲区。

**Gemini 异质质询发现的矛盾**：fold 操作的拓扑语义要求边重定向，与 CSR "标记无效" 方案存在物理死锁。

## 推导链（Gemini 推理摘要）

1. `engine.py` 的 `merge_vertices` 中，fold 不只删除顶点，核心操作是**边重定向**：
   ```python
   src = keep if e.source == remove else e.source
   tgt = keep if e.target == remove else e.target
   ```
2. 在 CSR 格式中，修改边的 `source` = 将该边从 `remove` 行移动到 `keep` 行
3. CSR `col_ind` 是紧凑连续数组，跨行移动元素必须后移所有数据 + 更新 `row_ptr` = O(E) 全局重建
4. 替代方案：别名表（Alias Table，`alias[remove] = keep`）
   - 但每次邻域查询时 GPU 必须做依赖性内存读取（Pointer Chasing）解析别名
   - 彻底破坏 GPU 的内存合并访问（Memory Coalescing）→ 性能崩塌

## 判定（同质质询执行）

**否定成立**，针对"fold 标记无效不重建"的具体方案。

- 定义回溯：`merge_vertices` 中的边重定向代码与 Gemini 描述完全吻合（`engine.py:L约380-420`）
- 反例构造：两条路都是死路——重建 = O(E) PCIe 传输；别名表 = Pointer Chasing 破坏 Coalescing
- 推论检验：与 461号（CSR 不可变性矛盾）互补——461号从 add_edge 方向否定，本号从 fold 方向否定

## 边界条件

如果折叠操作频率极低（如每百万步才有一次 fold），标记无效 + 延迟重建在实践中可能可接受。
当前逢亮 fold 频率未知，但每次 encounter 都可能触发 fold。

## 下游推论

CSR 方案的双重死锁（461号 add_edge + 464号 fold），表明 CSR 对逢亮不适用不是单点问题，而是结构性不匹配。

## 谱系引用

- 461号：CSR 静态性矛盾（R1，add_edge 方向）
- 231号：形式化有效域规则——CSR 的有效域不包含频繁变异的图

## 来源标注

[Gemini 异质质询] gemini-3.1-pro-preview, 2026-03-16

review-results 路径：`.chanlun/review-results/gemini-genealogy-review-2026-03-16T02-14-21.md`
