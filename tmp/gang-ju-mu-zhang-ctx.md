# 纲举目张质询上下文

## 纲

递归运动在计算质料上产生它自己的亏格。

## 总方针关键§目（已实现/未实现对照）

### 已实现

| § | 内容 | 实现形式 |
|---|------|---------|
| §10-11 | 区块拓扑三层架构声明 | `scripts/block_topology.py` — Event layer 完整实现 |
| §12 | Event/Relation/Reflexive 三层 | Event layer: 182个区块; Relation layer: 958条关系; Reflexive layer: 0 rewrite |
| §13 | 区块类型 event/consensus/residue/tension/rewrite | 类型定义在代码中，但只有 event 类型有实例 |
| §17 | CC/Gemini/Codex/Serena 四主体 | CLI 入口存在，API key 已配置 |
| §18 | 双重质询循环 | `stance_parser.py` + `consensus_trigger.py` 立场差分架构已实现 |
| §21 | 共识仪式三区块原子写入 | `consensus_ceremony.py` + `trigger_ceremony()` 代码就绪 |
| §14 | 否定不消除 | Event layer 不可变已实现 |
| §26 | 缠论14个已结算定义 | `缠论知识库.md` + `.chanlun/definitions/` |

### 未实现（关键缺口）

| § | 内容 | 缺口性质 |
|---|------|---------|
| §9 | Nachträglichkeit：事件不变，意义追溯改写 | **Relation layer 从未被追溯改写。** 958条关系全部是迁移时一次性写入的 depends_on，没有任何 negates/supersedes/reopens 关系 |
| §12 Reflexive | 关系变更自动回写为 rewrite 区块 | **0个 rewrite 区块。** Reflexive layer 完全未激活 |
| §20 | Real 在共识处——共识生产剩余物 | **0个 consensus/residue/tension 区块。** 代码存在但从未被调用 |
| §22 | CC 面对共识和剩余，捡起剩余物 | 无剩余物可捡——循环从未转过 |
| §23 | 剩余物积累 → 系统性结构问题 | 无剩余物积累 |
| §24 | 奇点检测：找反复 rewrite 的区块簇 | 无 rewrite → 无可检测对象 |
| §25-30 | 核心回路：缠论识别→交易→利润→物质条件 | 交易谱系不存在 |
| §31-33 | 交易谱系 | 完全未启动 |
| §63 | 共识仪式形式化 | 代码写了，但"从未被真实质询循环调用" |

### 诊断

**根本张力不是某个§目没填，而是系统只运行了 Event layer。**

纲说"递归运动产生亏格"。当前系统的运动是：
1. CC 生产判断（谱系条目）→ 写入 Event layer → 完成
2. 没有质询循环（Gemini/Codex 质询代码存在但从未在生产环境执行）
3. 没有共识仪式（代码存在但从未被调用）
4. 没有追溯改写（Relation layer 是静态的迁移产物）
5. 没有 Reflexive 回写

这意味着系统当前是**单层线性谱系**，不是**三层递归拓扑**。纲要求的递归运动（每一轮审查上一轮、追溯改写、产生剩余物）从未发生。

## 质询问题

1. **从纲到新目的推导**：区块拓扑的 Event layer 已有 182 个区块和 958 条关系。要让 Relation layer 和 Reflexive layer "活起来"，最小的驱动力是什么？是先让一个真实的质询循环跑通（产生 consensus/residue/tension），还是先让一个 Nachträglichkeit 发生（追溯改写一条已有关系）？

2. **管道连通的优先序**：ceremony_scan 当前从旧 dag.yaml 读取。block_topology API 已就绪。consensus_trigger 代码已就绪。三者之间的连接点在哪里？什么是让数据第一次流过新管道的最短路径？

3. **纲的自我验证**：总方针 §9 说"t 审查 t-1 时，t-1 已不可修改，但 t 可以改写 t-1 与其他区块之间的关系"。当前 182 个 event 区块中，是否存在应该被追溯改写但尚未被改写的关系？换言之，181 轮谱系运动中，是否有后来的发现否定了先前的判断，但这个否定只存在于对话历史中，没有被写入 Relation layer？

4. **共识仪式的第一次执行**：`trigger_ceremony()` 需要一个 `InquiryCycleResult`（来自真实质询循环的立场差分）。当前 Gemini/Codex CLI 已就绪。什么是让共识仪式第一次在生产环境执行的最小场景？

5. **ceremony_scan 迁移**：ceremony_scan 当前仍读 dag.yaml。已有 plan 将 `get_frozen_nodes()` 和 `detect_genealogy_anomalies()` 迁移到读 block-topology。这个迁移是否是管道连通的前置条件？
