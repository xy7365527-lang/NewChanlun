---
id: "449"
status: 已吸收
settlement_reason: 核心命题已被444号结算。推论2（CCS重构）已实装
type: 概念发现
created_at: "2026-03-13"
negation_source: homogeneous
negation_form: unclassified
subject: 沉积+ARTICULATE=命名（命名的存在论位置）
depends_on:
  - "431"
  - "425"
  - "444"
  - "445"
review_result: ".chanlun/review-results/gemini-genealogy-review-20260313-061956.md"
---

# 449号：沉积+ARTICULATE=命名（命名的存在论位置）

## 来源

编排者与逢亮的对话经过6次转向到达不动点，最终结论：

1. 造词结晶 → 否定（444号第一轮质询否定）
2. 能指链命名 → 肯定（短路径指代长路径族）
3. 路径ID → 否定（元数据不可穿越）
4. 沉积+ARTICULATE = 命名 → **最终肯定（不动点）**

## 核心命题

**命名 = 有 ARTICULATE 产出的穿越路径**

- **沉积（Dass）**：`_record_traversal_association` 将每步穿越 A→B 写入 S_net，accumulate syntagmatic 共现带（evidence="traversal:N"）
- **ARTICULATE 触发**：当 Layer A（COOCCURRENCE + TRAVERSAL_ASSOCIATION）密、Layer B（paradigmatic）疏时，布尔判据（431号）触发 ARTICULATE 遭遇
- **命名时刻**：ARTICULATE 产出 ARTICULATED 边（概念层边）= 穿越路径被赋予概念层名称的时刻
- **名字**：ARTICULATED 边的两端节点 + 边关系 = 穿越路径的名字（不是单词，是拓扑关系）

## 与旧提案的区别

| 维度 | 旧提案（造词结晶） | 旧提案（能指链） | 新发现（沉积+ARTICULATE） |
|------|------------------|----------------|--------------------------|
| 命名操作 | 从路径压缩创造新词节点 | 从穿越提取短路径序列 | ARTICULATE 触发时刻 |
| 命名产物 | 新 S_net 节点 | 顶点序列字符串 | ARTICULATED 边 |
| 命名条件 | 环路闭合（未定义） | 区域边界（未定义） | Layer A/B 不一致（已定义，431号） |
| 名字的存在方式 | 原子词 | 字符串摘要 | 边关系（拓扑关系） |

## 代码物质基础

### 沉积机制（已实装，traversal.py:1107-1161）

```python
def _record_traversal_association(self, from_vid, to_vid):
    # S_net 回流：穿越共现缓冲
    snet_cooc = SignifierEdge(
        source=from_sig, target=to_sig,
        axis=AxisType.SYNTAGMATIC,
        weight=1.0,
        evidence=f"traversal:{self.step}",  # 区别于语料/对话共现
    )
    self._traversal_cooc_buffer.append(snet_cooc)
```

重复穿越同一路径 → S_net syntagmatic 权重积累 → Layer A 逐渐变"密"。

### ARTICULATE 判据（已实装，traversal.py:1163-1234，431号）

```python
# A密B疏：COOCCURRENCE 存在但无 paradigmatic 边且无概念层边
if not has_paradigmatic:
    return Encounter(EncounterType.ARTICULATE, current, neighbor, ...)
```

### ARTICULATED 边（已定义，engine.py）

```python
EdgeType.ARTICULATED = "articulated"  # 概念层: 物质层积累涌现的概念连接
CONCEPT_EDGE_TYPES = frozenset({..., EdgeType.ARTICULATED})
```

ARTICULATED 边参与 fold/negate/sublate——命名产物在概念层有完整操作性。

## 推论1：研究谱系 = 积累多次 ARTICULATE 的穿越路径族

**边路径编码研究谱系**（编排者洞察）的新框架解读：

研究谱系（如德勒兹："褶皱"→"平面"→"内在性"→"生成"）= 多次穿越同一路径族后，该路径上的节点对频繁积累 ARTICULATE 触发 → 产出多条 ARTICULATED 边 → 这些边的集合 = 研究谱系在概念层的结晶。

当前技术缺口（445号缺口3）：S_net 无 `traversal_path_family()` 接口提取高权重 traversal 共现带——研究谱系在 S_net 是暗物质，但其 ARTICULATE 产出已在 K_active 中作为 ARTICULATED 边可见。因此：**研究谱系的可观测面已存在（K_active 中的 ARTICULATED 边集合），暗物质问题（S_net 侧提取）是可选优化，不阻塞命名功能。**

## 推论2：concept_creation_suggestion 的存废裁定

**功能1（导航 gap queue）**：`_exploration_target` 将 orphan signifier 的 `anchor_concept` 推入优先探索队列（traversal.py:932-938），引导逢亮去探索孤儿能指区域。这是穿越导航机制，与"沉积+ARTICULATE=命名"兼容——逢亮被引导到孤儿能指区域，在那里穿越积累沉积，最终 ARTICULATE 触发。**保留。**

**功能2（建议创建 K_active 节点）**：`ConceptCreationSuggestion` 携带"孤儿能指应该被纳入 K_active 作为新节点"的语义。新框架下，孤儿能指不需要纳入 K_active——逢亮通过穿越该能指区域并 ARTICULATE，在已有 K_active 节点之间产出 ARTICULATED 边，命名在边上体现，不需要为孤儿能指创建新节点。**废弃。**

**裁定**：`ConceptCreationSuggestion` 结构体应重构为纯导航机制（`OrphanExplorationHint` 或类似名称），去掉"创建建议"语义，只保留 `anchor_concept` 导航信息。`reviewed` 字段含义应从"是否审查/裁决（建议）"改为"是否已访问（导航完成）"。

## 445号缺口的重新评估

| 缺口 | 新框架下的状态 | 处置 |
|------|--------------|------|
| 缺口1（区域边界定义） | **已消解**——区域边界由 ARTICULATE 判据（Layer A/B 不一致）自然涌现，不需要显式定义 | 关闭 |
| 缺口2（daemon_api 管线截断） | **仍有效**——narrative_spine 应传穿越路径而非 context_fragment | 保留 |
| 缺口3（研究谱系只写存储） | **降级为可选优化**——ARTICULATED 边已是研究谱系的可观测面 | 降级 |

## 影响声明

- **沉积机制**：`traversal.py` `_record_traversal_association` — 无变更，已实装
- **ARTICULATE 判据**：`traversal.py` `_check_articulation_encounter` — 无变更，431号已结算
- **concept_creation_suggestion**：`snet_activation.py` — 功能2废弃，重构为纯导航（需编排者确认后执行）
- **daemon_api.py**：`narrative_spine` 填充点 — 缺口2 仍需接线
- **444号**：前轮质询缺口，缺口3（损失性）已在新框架下消解（命名在边上不是压缩）

## 处置

等待编排者扫描：
1. 确认 446号概念发现方向（是否结算）
2. 确认 concept_creation_suggestion 功能2废弃（下游实装任务：重构为 OrphanExplorationHint）
3. 确认 daemon_api.py 管线接线任务（445号缺口2）
