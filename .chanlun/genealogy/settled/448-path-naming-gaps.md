---
id: "448"
status: 已吸收
type: 矛盾发现
date: "2026-03-13"
---

# 448号：路径即命名的三个实装缺口

## 来源

[Gemini 异质质询] 编排者对话后三个反转（复合能指结晶被否定 → 路径即命名）的质询。
质询模式：`challenge`（无工具，纯推理）
模型：gemini-3.1-pro-preview

## 质询对象

编排者洞察（反转后）：命名 = 穿越摘要 = 从长穿越路径提取短路径（能指链）。无需结晶操作，无需新节点，"生存论谱系" = 能指链 "生存论" → "谱系"。

## 发现的三个缺口

### 缺口1：线性截断 ≠ 拓扑区域摘要（致命）

**冲突方**：编排者洞察（命名 = 穿越某**区域**后识别核心节点序列）vs `psi_L_constraint.py`（`traversal_path[-8:]` = 时间轴线性截断）

**代码位置**：`psi_L_constraint.py:122-127`

```python
for vid in traversal_path[-8:]:  # 最近 8 步 — 线性截断
    spine_labels.append(_vertex_label(vid, v))
narrative_spine = " → ".join(spine_labels)
```

**问题**：`[-8:]` 对图拓扑盲目。逢亮在密集局部簇中徘徊 10 步，`[-8:]` 提取冗余碎片；逢亮 8 步内穿越 3 个拓扑区域，`[-8:]` 提取跨区域缝合怪。

**实装缺口**：缺少"区域边界（Region Boundary）"定义。无区域边界，线性截断永远无法等价于拓扑摘要。可能的方向：
- 用 SettlementTracker 的 settled_cycles 定义区域
- 用 f(v,w) 地形的梯度定义区域边界
- 用 visit_history 中的返回频率定义核心度

### 缺口2：daemon_api.py 管线截断（致命）

**冲突方**：`psi_L_constraint.py` 提供了穿越路径→narrative_spine 的完整管线 vs `daemon_api.py`（直接用 `context_fragment` 覆盖 narrative_spine）

**代码位置**：`daemon_api.py:780`

```python
cs = ConstraintSet(
    narrative_spine=context_fragment,  # operator 对话上下文，非穿越路径
    ...
)
```

**问题**：`graph_to_llm_prompt(traversal_path=...)` 接口存在但 daemon 不调用。逢亮的穿越路径在传递给 LLM 之前被物理切断，LLM 在"叙事骨架（概念穿越路径）"字段看到的是用户输入，不是逢亮的穿越经验。

**实装缺口**：daemon_api.py 中的 ConstraintSet 构建需要改为从 daemon 的 traversal 引擎获取 `visit_history`（`traversal_engine.visit_history`），而非用 `context_fragment` 填充。

### 缺口3：研究谱系是只写存储（重要）

**冲突方**：编排者洞察（物质层共现带 = 研究谱系路径族，可被识别和使用）vs 代码现实（traversal 写入 S_net，无提取函数）

**代码位置**：`traversal.py:1141-1148`（写入端）

```python
snet_cooc = SignifierEdge(
    source=from_sig, target=to_sig,
    axis=AxisType.SYNTAGMATIC,
    weight=1.0,
    evidence=f"traversal:{self.step}",  # 可区分来源
)
```

**问题**：`syntagmatic_neighbors()`、`degree_normalized_neighbors()` 不区分 evidence 来源。无法从 S_net 中提取"traversal 类型的高权重共现子图"（路径族）。研究谱系（如德勒兹："褶皱"→"平面"→"内在性"→"生成"的反复穿越积累）在 S_net 中是暗物质。

**实装缺口**：需要 `SNet.traversal_path_family(min_weight: float) -> list[SignifierEdge]` 或类似接口，过滤 `evidence.startswith("traversal:")` 的高权重边子集。

## Gemini 推导链摘要

1. 读取上下文中 `traversal_path[-8:]` → 识别"线性截断 vs 拓扑区域"差异 → 矛盾1
2. 读取 `daemon_api.py:780` → 发现穿越路径未传递 → 矛盾2（管线截断）
3. 读取 TRAVERSAL_ASSOCIATION 写入逻辑，注意无提取函数 → 矛盾3（只写存储）
4. 从反转1推导 ConceptCreationSuggestion 不应共存 → 矛盾4（已判定不成立，见 review）

## 矛盾4 的误判记录

Gemini 认为 ConceptCreationSuggestion 与反转1（不造词）本体论互斥。
**判定不成立**：Gemini 混淆了"合成词路径压缩"（反转1否定）和"语料词概念收录"（ConceptCreationSuggestion 所做）。两者目标对象不同，共存合理。详见 review 文件第4节。

## 否定判定

**三个缺口均成立**——路径即命名洞察在当前代码中有三处实装分歧：
1. 区域边界定义缺失（缺口1）
2. 穿越路径→spine 管线未接线（缺口2）
3. 研究谱系路径族提取接口缺失（缺口3）

缺口2 是当前可直接修复的接线问题（daemon_api.py 传入 visit_history）。
缺口1 需要先定义"区域"概念（设计决断）。
缺口3 是接口扩展（增加 traversal 过滤的读取函数）。

## 处置建议

等待编排者扫描决定：
1. **缺口2 接线（可立即执行）**：daemon_api.py 修改，从 traversal 引擎获取 visit_history 传给 ConstraintSet
2. **缺口1 设计决断（需选择）**：区域边界定义方案三选一（上浮，选择类决断）
3. **缺口3 接口扩展（可立即执行）**：`SNet` 增加 `traversal_path_family()` 方法
