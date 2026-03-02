---
id: '310'
number: 310
type: meta-rule
title: "元观察——309号异质审查裁定执行 + 关系分层代码实现 + block-topology 302-308 补建"
date: "2026-03-02"
depends_on: ['309', '308', '218', '137', '090', '231']
status: 已结算
rule_version_baseline:
  claude_md_commit: "97f3ba3186f1919ff6abf02dea4247724d47c79a"
  rules_dir_mtime: "2026-03-01 00:27:49 +0000"
---

# 310号：元观察——309号异质审查裁定执行 + 关系分层代码实现 + block-topology 302-308 补建

## 递归判断

任务不可分解：meta-observer 是单一观察角色，观察过程不可并行分割。扁平退化特例。

## 规则版本基线

- `claude_md_commit`: 97f3ba3（与 308号相同——CLAUDE.md 未变更）
- `rules_dir_mtime`: 2026-03-01 00:27:49 +0000（与 308号相同——规则目录未变更）

结论：本观察与 308号处于同一规则版本下，任何差异来自 agent 执行偏差而非规则迭代。

## 观察对象

本轮蜂群产出（commit 489ba76）：
1. 309号谱系——Phase 2 异质审查裁定（关系分层 + 结构标记过滤 + duplicate 阈值统一 + inherits 推迟）
2. 302-308号 block-topology 映射补建
3. 四项代码变更：
   - `concept_extractor.py`：+`_FRONTMATTER_METADATA_FIELDS`（17字段）+ `_STRUCTURAL_MARKERS`（18标记）+ NNN号引用过滤——三层过滤链
   - `concept_topology_check.py`：+`LOGICAL_RELATIONS` / `NAVIGATIONAL_RELATIONS` 分层集合 + `layer1_*` / `layer2_*` 图不变量输出 + duplicate `folding_status` 统一为 "duplicate" + `needs_review` 字段
   - `test_concept_extractor.py`：+`TestFrontmatterMetadataFilter`(8 tests) + `TestGenealogyIdTermFilter`(3 tests) + `TestStructuralMarkersFilter`(6 tests)
   - `test_concept_topology_check.py`：+`TestRelationLayerSets`(5 tests) + `TestLayeredGraphInvariants`(4 tests) + 折叠测试更新
   - `test_gemini_challenger.py`：503 重试逻辑测试更新（适配 `_MAX_RETRIES` 指数退避）
4. 下游推论消化（307/306/305/304号 block-topology 映射已完成）

测试状态：195 passed，0 failed。

## 观察结果

### 观察1（收敛信号）：裁定→代码的执行闭环

309号裁定包含四个议题的具体处置方案。本轮代码变更逐条对应：

| 309号裁定 | 代码实现 | 对应关系 |
|-----------|---------|---------|
| 议题1 inherits 推迟 | 无代码变更（正确——推迟意味着不实现） | 否定性对应 |
| 议题2 duplicate 阈值统一 | `folding_status` 从 potential_birth/confirmed_duplicate 统一为 "duplicate" + `needs_review=True` | 正对应 |
| 议题3 关系分层 Layer 1/2 | `LOGICAL_RELATIONS` + `NAVIGATIONAL_RELATIONS` + `layer1_*`/`layer2_*` 不变量 | 正对应 |
| 议题4 domain 下沉到 concept 级 | 本轮未实现（正确——裁定中明确 domain 枚举定了但实现留给后续） | 否定性对应 |

这是"裁定→执行"闭环的标准模式：裁定说做什么就做什么，说不做的就不做。没有额外的"顺便改进"或"提前实现"。与 090号严格性规则一致——代码实现不膨胀超出裁定范围。

**四分法分类**：定理——090号在裁定执行场景下的逻辑推论。

### 观察2（收敛信号）：结构标记过滤的判断依据层级验证

308号观察1 识别了语法记录候选："过滤操作的判断依据层级——结构语义 > 统计频率 > 无判断依据"。本轮新增的 `_STRUCTURAL_MARKERS` 进一步验证了这个模式：

| 过滤层 | 判断依据 | 来源 |
|-------|---------|------|
| `_VALUE_ASSIGNMENT_RE` | definition 内容的正则模式（赋值形态） | 308号第1轮校准 |
| `_FRONTMATTER_METADATA_FIELDS` | term 名在已知元数据字段集合中（结构语义） | 308号第3轮校准 |
| `_STRUCTURAL_MARKERS` | term 名在文档结构标签集合中（结构语义） | 309号裁定议题2 |
| NNN号引用过滤 | term 匹配 `^\d{3}[a-z]?号$`（格式模式） | 308号第3轮校准 |

四层过滤均基于结构语义或内容模式，无一基于统计频率。308号语法记录候选在实践中持续验证。

**四分法分类**：定理——308号观察1 的收敛确认。

### 观察3（发散信号）：LOGICAL_RELATIONS ⊄ TOPOLOGICAL_RELATIONS 的集合关系异常

测试 `TestRelationLayerSets.test_navigational_subset_of_topological` 中有一个值得注意的细节：

```python
def test_navigational_subset_of_topological(self):
    # NAVIGATIONAL_RELATIONS 中的 "defines" 不在 TOPOLOGICAL_RELATIONS 中
    nav_topo = NAVIGATIONAL_RELATIONS - {"defines"}
    assert nav_topo <= TOPOLOGICAL_RELATIONS
```

`NAVIGATIONAL_RELATIONS` 包含 `defines`，但 `TOPOLOGICAL_RELATIONS` 不包含 `defines`。这意味着 309号裁定中声明的分层关系 `LOGICAL ⊂ NAVIGATIONAL ⊂ TOPOLOGICAL` **在代码中不严格成立**——`NAVIGATIONAL ⊄ TOPOLOGICAL`。

测试通过的方式是排除 `defines` 后再验证子集关系。这不是测试 bug——它反映了一个概念张力：`defines` 在导航层有意义（知道哪个区块定义了哪个概念），但在拓扑计算中不参与（不携带依赖/否定/修正语义）。

**分析**：309号裁定中"Layer 2 = Layer 1 + references + defines + refines"的声明与代码中 `TOPOLOGICAL_RELATIONS` 不含 `defines` 之间存在声明偏差。裁定没有显式声明 Layer 2 ⊂ TOPOLOGICAL_RELATIONS，但分层的隐含语义是"Layer 2 是 TOPOLOGICAL 中导航性关系的集合"。`defines` 打破了这个隐含假设。

**四分法分类**：定理——这是 309号裁定的逻辑推论。`defines` 的双重身份（导航有意义 + 拓扑不参与）在 309号裁定中未被显式处理。不构成矛盾（代码正确处理了它），但构成一个声明精度不足的标记。后续如果 `defines` 关系需要参与某种拓扑计算（如概念注册表的 inherits 检测），分层关系的集合定义需要重新审视。

### 观察4（收敛信号）：并行工位分派持续一致

本轮蜂群包含至少 4 个工位（audit-executor、topo-mapper、genealogist、downstream-resolver + meta-observer），全部并行完成。与 308号观察2（6 agent）、307号（2 agent）、305号（3 agent）对比：并行分派持续由数据依赖驱动。

audit-executor 和 topo-mapper 标记为 in_progress 说明它们仍在执行中——meta-observer 不等待它们完成再写观察，这符合并行原则（meta-observer 的输入是已 commit 的产出，不依赖正在执行的审计/映射结果）。

**四分法分类**：定理——218号并行原则的持续验证。

### 观察5（自环检查）：与历史 meta-observation 的交叉对比

| 编号 | 核心发现 | 本轮状态 |
|------|---------|---------|
| 308 观察1 | 过滤判断依据层级（结构语义 > 统计频率） | 收敛——`_STRUCTURAL_MARKERS` 进一步验证（观察2） |
| 308 观察3 | 分层剥离模式（校准迭代） | 收敛——四层过滤链稳定，无新噪声层发现 |
| 308 观察2 | 并行化持续一致 | 收敛——4+ agent 并行（观察4） |
| 302 观察1 | 218号串行违反 | 未再发生——本轮无串行违反迹象 |
| 302 观察3 | declares 缺失测试的代码路径 | 未在本轮重现 |

无新方法论洞察需要写入谱系。观察3（`defines` 集合关系异常）是定理类，标注在本记录中备查，不走 `/escalate`。

308号观察1 语法记录候选（过滤判断依据层级）第二次收敛确认。如果在下一个含过滤操作的 session 中继续一致，建议结晶为规则。

## 结论

本轮核心产出是 309号裁定的忠实代码实现——四议题逐条对应，无范围膨胀。195 测试全通过。一个发散信号（观察3：`NAVIGATIONAL ⊄ TOPOLOGICAL` 的集合关系异常）是定理类，标注备查。308号语法记录候选（过滤判断依据层级）第二次收敛确认。

无规则触发/违反异常。元规则一致性确认——218号并行、137号格式、090号严格性在本轮持续执行。
