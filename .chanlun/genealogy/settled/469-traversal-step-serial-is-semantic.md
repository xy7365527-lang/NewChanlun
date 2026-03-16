---
id: "469"
title: "穿越步内串行是语义决定的（非工程选择）"
status: 已结算
type: 已结算
resolution: 吸收——461-468号质询系列的概念层结论
negation_source: heterogeneous
negation_form: separation
negation_model: gemini-3.1-pro-preview
created_at: "2026-03-16"
depends_on:
  - "461"   # CSR 静态性矛盾
  - "462"   # LLM 伪同构
  - "463"   # kernel 开销倒挂
  - "464"   # CSR fold 边重定向死锁
  - "465"   # Python-GPU roundtrip 延迟倒挂
  - "466"   # cuGraph OLAP 不匹配
  - "467"   # K_active 规模幻象与 Rust 转向
  - "468"   # 多实例 GPU 五重死锁
affects:
  - "topological-computation/traversal.py"   # TraversalEngine.run_step
  - "topological-computation/engine.py"      # Graph 变异方法
---

# 469号：穿越步内串行是语义决定的（非工程选择）

## 核心命题

`run_step` 内部的 t0 -> t1 -> t2 -> t3 读写依赖链不是可以拆开并行的工程瓶颈——它是辩证运动的时间结构。

## 推导链

1. `run_step` 每步执行序列：walk() -> detect_encounter() -> execute_encounter() -> _record_traversal_association() -> _pull_cooccurrence_edges()
2. 每个阶段读取前一阶段的写入结果：
   - `detect_encounter` 读取 walk() 到达的位置和实时图状态
   - `execute_encounter` 的 fold/negate/sublate 依赖 detect 的判定结果
   - `_record_traversal_association` 写入遭遇后被修改的图
   - `_pull_cooccurrence_edges` 依赖当前步的完整关联状态
3. 遭遇改变图（fold 删除顶点、negate 翻转边、sublate 合并结构），改变后的图**立刻**影响后续判断（如 `_compute_f` 依赖实时度数、terrain 依赖实时连通分量）
4. 并行化 = 取消阶段间的时间顺序 = 取消"否定必须在被否定者之后、扬弃必须在否定之后"的辩证法时间结构

## 谱系链接——与 461-468 的关系

461-468 是从物理层（CSR、GPU SIMT、PCIe 带宽、kernel 开销）逐条否定 GPU 加速方案。469号是这些否定的**概念层收敛**：物理层的所有瓶颈都指向同一个根因——穿越步内的串行性不是工程实现的偶然，而是语义上不可消除的读写依赖。

| 物理层否定 | 指向的语义根因 |
|-----------|-------------|
| 461（CSR 静态性）| 图每步变异 = 辩证运动改变对象 |
| 462（LLM 伪同构）| 密集张量 vs 不规则图遍历 = 静态权重 vs 动态否定 |
| 463（kernel 开销）| 微秒级计算 + 毫秒级调度 = 步内粒度太细 |
| 464（fold 边重定向）| fold 需要实时遍历全部入边 = 否定的影响是全局的 |
| 465（roundtrip 延迟）| Python-GPU 往返 > 计算本身 = 控制流不可分离 |
| 466（cuGraph OLAP）| OLAP 不支持实时变异 = 分析工具假设静态数据 |
| 467（规模幻象）| K_active 千级顶点 = 规模不足以摊薄并行开销 |
| 468（五重死锁）| 多实例共享图写冲突 = 并行化摧毁拓扑因果性 |

## 边界条件

此结论在以下条件下可能翻转：
- `run_step` 被重构为纯函数式（每步返回新图，不就地修改）**且**图规模扩展到 >100K 顶点——此时步间的图快照可以用 GPU 做只读查询
- 但步内阶段间的读写依赖仍然存在——纯函数式只移动了复制开销，不消除语义依赖

## 下游推论

1. Rust + CPU SIMD 路线的合理性：步内串行不可消除，但可以让每个串行步骤执行得更快（内存局部性、分支预测、SIMD 邻域枚举）
2. 加速方向应从"步内并行"转向"步间优化"：减少每步的 O(E) 开销（terrain 增量化、adj 索引优化）

## 来源标注

[新缠论] 基于 461-468 号 Gemini 异质质询系列的概念层收敛
