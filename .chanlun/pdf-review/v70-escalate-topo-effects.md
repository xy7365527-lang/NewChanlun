# v70-swarm /escalate 报告：topo_effects 规范性矛盾

**时间**: 2026-04-24
**分支**: claude/dazzling-kowalevski-52a297
**蜂群**: v70-swarm
**触发**: 14 条 pending_topo_effects 的静态分析揭示两类 id_mapping 覆盖缺口

## 矛盾声明

`block-topology/meta.json` 的 `id_mapping` 只注册**主 id**（单一数字或单一字符串，如 `"482"`），但谱系 topo_effect 字段大量使用**复合 id**（主 id + 限定符，如 `"436-下游推论3"`、`"概率范式概念群"`、`"425-单层K_active"`）。

这造成 `topology_operator.py:resolve_genealogy_id` 严格 key 匹配时，**9 条 topo_effect 无 target**。

## 四分法分类

**语法记录类**（不属于定理/行动/选择）——这不是"绕过矛盾"的选择，而是**揭示了现有 id 规范的覆盖缺口**，需要编排者决定如何扩展规范。

## 9 条具体 topo_effect（target 未在 id_mapping）

| # | source | topo_effect | 复合 target | 问题 |
|---|--------|-------------|-------------|------|
| 1 | 437 | split:436-下游推论3:downstream | `436-下游推论3` | 主 id 436 存在，但"下游推论3"是子节点，未独立注册 |
| 2 | 480 | split:478:local | `478` (实际 `480` split `478`) | 目标 id 明确，但 split 类型语义需验证 |
| 3 | 440 | split:425-单层K_active:downstream | `425-单层K_active` | 主 id 425 存在，"单层 K_active" 是内部观察 |
| 4 | 436 | sever:概率范式概念群:downstream | `概率范式概念群` | **纯字符串 target**，未绑定任何主 id |
| 5 | 441 | split:437-回流管道:downstream | `437-回流管道` | 同 #1 模式 |
| 6 | 442 | split:439-S_net参数区分:local | `439-S_net参数区分` | 同 #1 模式 |
| 7 | 439 | split:438-S_net位置:local | `438-S_net位置` | 同 #1 模式 |
| 8 | 434 | sever:LLM-thinking-analogy:local | `LLM-thinking-analogy` | 纯字符串 target，类似 #4 |
| 9 | 435 | split:434-终止条件:downstream | `434-终止条件` | 同 #1 模式 |

## 两类问题

### 类型 A：复合 id（主 id + 限定符，7 条：#1/#2/#3/#5/#6/#7/#9）

谱系作者在 topo_effect 中用 `{主 id}-{内部限定符}` 表达"对某号谱系内某子节点的操作"。这是一个**实际在使用的隐式语法**，但未被 id_mapping 规范化。

### 类型 B：纯字符串 target（2 条：#4/#8）

`概率范式概念群` 和 `LLM-thinking-analogy` 不是谱系号而是**概念簇的名称**。这与 id_mapping 的"单一 id 映射"模式根本冲突。

## 编排者必须决定的方向

三个候选路径（**这是"选择类"**——需价值判断）：

**路径 I：扩展 id_mapping 允许复合 id**
- 在 meta.json 增加 `compound_ids` 字段，形如 `{"436-下游推论3": "436_sub_downstream_3"}`
- 需要谱系作者在新增 topo_effect 时显式注册
- **代价**：id_mapping 膨胀，维护成本增加

**路径 II：谱系扁平化（谱系作者规范升级）**
- 要求所有 topo_effect 的 target 必须是 id_mapping 中已注册的主 id
- 子节点操作必须升级为新谱系（如 "436-下游推论3" 升级为 485 号独立谱系）
- "概率范式概念群" 必须升级为一个具体谱系号
- **代价**：谱系号膨胀，发现密度降低

**路径 III：修改 topo_effect 字段规范**
- 不改 id_mapping，在 topo_effect 中引入分隔符明确 "主 id + 内部路径" 结构
- 如 `split:436#下游推论3:downstream`，主 id 与子路径分开解析
- `topology_operator.py` 增加 subpath 解析逻辑
- 纯字符串 target（类型 B）需要先升级为某号谱系的子路径
- **代价**：规范变更需迁移所有历史 topo_effect

## 本轮处理

- **未执行任何 topo_effect**：topo-operator-v70 合理拒绝硬塞
- **2 条 scope 非法（#1 trigger-conditions / #3 independent）**：同样等编排者裁定是扩展 VALID_SCOPES 还是修改谱系
- **3 条可执行（#12/#13/#14 即 478→401/479→401/480→478 路径）**：待 Bash classifier 稳定后下一轮 spawn topo-operator-v71 执行

## 谱系依据

- 090号：严格性语法规则——不允许硬塞不合法的 target
- 136号：成本收益消解禁止——不允许用"14 条中修 3 条算了"论证绕过
- 161号：务实否定——把矛盾和缺口留到后面 = 先验上限
- 275号：局部依赖——Lead 不做全局排序，这些条目的处理顺序由编排者决定

## 影响声明

本报告不修改任何代码或谱系文件。仅揭示规范性缺口，等待编排者决断。
