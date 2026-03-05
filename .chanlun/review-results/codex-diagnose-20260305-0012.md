# Codex diagnose — 2026-03-05 00:12:29 UTC

## 元数据

- **mode**: diagnose
- **subject**: 为什么新的并行研究线没有出现——ceremony_scan 能否自动生成新研究线，还是只能消费已有 gangmu.yaml 条目？当活跃纲目全部关闭时，蜂群如何继续生长？
- **model**: gpt-5.3-codex
- **timestamp**: 2026-03-05 00:12:29 UTC
- **context-file**: tmp/codex-new-lines-ctx.md

## Prompt

## 诊断目标

为什么新的并行研究线没有出现——ceremony_scan 能否自动生成新研究线，还是只能消费已有 gangmu.yaml 条目？当活跃纲目全部关闭时，蜂群如何继续生长？

## 上下文

# Codex 诊断上下文：为什么新的并行研究线没有出现？

## 诊断问题

v158-swarm 关闭了 3 条纲目（operational-methodology, block-topology-eng, discrete-morse-experiment），活跃研究线从 4 条降到 1 条（scanner-l2-validation 需真实数据）。v159 的 ceremony_scan 只输出修复工位（356映射、stagnation等重复项），没有新的研究线或工位出现。

蜂群进入"等待真实数据"的稳态，编排者认为应该有新的并行线可以推进。

---

## 1. ceremony_scan.py 的工位生成逻辑

ceremony_scan.py 的输入源（按优先级排序）：

1. **roadmap.yaml** — 最高优先级，status=active 的任务
2. **session 遗留项** + **pending 谱系**
3. **pattern-buffer** — candidate 模式计数
4. **review-results/** — 未消费审查结果
5. **block-topology 映射缺口** — 未映射谱系 vs 已映射
6. **偶遇检测** — 跨纲 encounter_candidate
7. **研究线扫描** — `_scan_research_lines()` 读取 gangmu.yaml 中 status=active 的 mu，提取 unblocked next_actions
8. **谱系编号异常检测**
9. **async_self_ref** 审计
10. **meta-rule 谱系消费** — 扫描 type=meta-rule 谱系中的 grammar_record_candidate/divergence_signal
11. **no_work_fallback** — 仅在以上全部为空时触发（且需要环境变量开启）

**关键约束**：`_scan_research_lines()` 只消费 gangmu.yaml 中 `status=active` 的 mu。ceremony_scan **不能自动生成新研究线**——它只能消费已有的 gangmu.yaml 条目。

---

## 2. 当前 gangmu.yaml 状态

### 活跃目（status=active）

| gang | mu | 状态 | 未完成的 next_actions |
|------|----|------|----------------------|
| simatrix-quanyu | multi-target-scanner | active | scanner-l2-validation（需真实数据，blocked_by: scanner-quotient-rank）—— 但 scanner-quotient-rank 已完成(completed_at: 2026-03-04)，所以 scanner-l2-validation 实际是 unblocked |

### 阻塞目（status=blocked）

| gang | mu | 阻塞原因 |
|------|----|---------|
| simatrix-quanyu | k4-regime-analysis | 等待新的全图同步压缩数据事件发生 |

### 已关闭目（status=closed）

- shipen-bihuan: code-pipeline, k4-independence-fiber, operational-methodology
- simatrix-quanyu: multi-economy-ontology, k4-engineering, oq-v2, fold-intrinsic-test
- block-topology: block-topology-eng, fold-topology-ontology
- swarm-infra: swarm-architecture-v2, swarm-architecture-v3
- knowledge-genealogy: traverse-infra
- morse-topology: discrete-morse-experiment

---

## 3. 总方针已定义但纲目未覆盖的阶段

### 阶段A（协议设计）
- 在 gangmu.yaml 中无对应 mu
- 总方针声明当前处于阶段A

### 阶段B-M（离散Morse理论数学研究）
- 总方针描述：在现有谱系数据上构造**最优**离散 Morse 函数（不是 BFS 生成树）
- 产出：类型化离散 Morse 理论（可发表论文）
- 触发条件：在穿越实践中遭遇 BFS 退化版和直觉严重不一致的情况
- **355号给出了 L2 否定性结果，说明 BFS 跳跃幅度 ≠ 概念否定显著性——这正是"退化版和直觉不一致"的证据**
- gangmu.yaml 中 morse-topology 纲下只有 `discrete-morse-experiment`（已关闭），**没有 B-M 数学研究目**

### 阶段C（实盘接入）
- 总方针描述：接入真实市场，核心回路首次闭合
- gangmu.yaml 中无对应 mu
- 当前 scanner-l2-validation 需要的"真实多标的数据"是阶段C的前置条件

### 阶段D（本地模型接入）
- 总方针描述：实盘利润投资算力，本地模型接管执行层
- gangmu.yaml 中无对应 mu

### 阶段I（核心回路全域展开）
- I-1 至 I-5 各子阶段在 gangmu.yaml 中无对应 mu

---

## 4. 已结算谱系的未转化下游推论

### 353号下游推论

| 推论 | 状态 | 说明 |
|------|------|------|
| 1. spec-execution-gap skill 需要更新 | **未执行** | 356号观察5确认 |
| 2. ceremony_scan 增加双向扫描 | **未执行** | 356号观察5确认 |
| 3. dispatch-dag 标注平台边界 | 已完成（v157-swarm） | |

### 355号下游推论（implicit，356号观察2中提及）

| 推论来源 | 内容 | 是否转化为研究线 |
|---------|------|-----------------|
| 356号观察2第3点 | "否定事件的拓扑特征不是幅度，而是配对结构"——引入关系类型权重（negates边权重应高于references边） | **未转化** |
| 355号边界条件 | "引入加权 Morse（按关系类型加权）可能改善 precision" | **未转化** |
| 355号边界条件 | "排除工程工件后重新排名" | **未转化** |

### topo-analyst 报告（356号 session 记录中）

topo-analyst 发现的结构性问题：
- schema 漂移（三格式并存）
- 354号双写问题
- supersedes/reopens/modifies 关系数 = 0
- 51 条张力悬置（tension_with 字段有值但未消费）
- 090号概念-拓扑不匹配

这些发现是否有对应的 gangmu next_actions？**无**。

---

## 5. ceremony_scan 能否自动生成新研究线？

`_scan_research_lines()` 函数的输入：
- 读取 `gangmu.yaml` 中 status=active 的 mu
- 提取每个 mu 的 unblocked next_actions
- 生成工位

**结论**：ceremony_scan **不能**自动生成新研究线。它只能消费已有 gangmu.yaml 中的 active mu。新研究线的引入必须通过：
- 编排者手动修改 gangmu.yaml（添加新 mu）
- 或其他机制将谱系下游推论转化为 gangmu next_actions

---

## 6. 为什么 v159 只输出修复工位

v158-swarm 结束后的状态：
- gangmu.yaml 中只有 1 个 active mu：multi-target-scanner
- multi-target-scanner 的 scanner-l2-validation action 的 blocked_by 是 scanner-quotient-rank
- scanner-quotient-rank 已标注 completed_at: 2026-03-04
- **因此 scanner-l2-validation 在逻辑上是 unblocked 的，应该生成工位**
- 但 scanner-l2-validation 的 completion_check 是 `genealogy_settled: scanner-validation`——该关键词在 settled/ 中不存在
- **所以 scanner-l2-validation 应该出现在 ceremony_scan 输出中**

session 遗留项（2026-03-04-2219-session.md）包含：
- 356映射、stagnation 等重复项

**如果 v159 的 ceremony_scan 只输出修复工位而没有 scanner-l2-validation，原因可能是**：
1. scanner-quotient-rank 的 completion_check 未满足（test_file_exists: tests/test_quotient_rank.py）
2. 或 ceremony_scan 在评估时 scanner-quotient-rank 被视为未完成

---

## 7. 可能的新研究线候选

根据分析，以下是可以立即开启的新研究线：

### A. 353号下游推论（已结算 > 工程）
1. **spec-execution-gap 双向扫描** — 更新 .claude/skills/spec-execution-gap/SKILL.md，增加消费断裂检测模式
2. **ceremony_scan 双向扫描** — 修改 ceremony_scan.py，增加"产出→消费"方向扫描

### B. 加权 Morse 改进（355号边界条件）
- 在 discrete-morse-experiment 已闭合的基础上，开启**加权 Morse 研究线**
- 按关系类型加权（negates > references > depends_on）
- 这是 B-M 阶段（类型化离散 Morse 理论）的具体起点

### C. topo-analyst 发现的结构性问题
- schema 漂移修复（三格式统一）
- supersedes/reopens/modifies=0 问题（拓扑关系缺失）
- 51条张力悬置消化

### D. 穿越基础设施组件三/四（总方针§八.五）
- 组件三：Morse 地形导航（BFS 生成树 + tree/critical 边标记集成进查询接口）
- 组件四：概念注册表（导出所有被 defines 的概念词）
- 这两个组件在 traverse-infra 关闭时是否都完成了？需要确认

---

## 诊断问题核心

**ceremony_scan 无法自动生成新研究线**——它是消费 gangmu.yaml 的工具，而不是生成 gangmu.yaml 的工具。

当所有 active mu 的 next_actions 都完成或阻塞时，ceremony_scan 输出变少，不是 bug，是设计。新研究线的产生需要外部注入（编排者）或内部涌现机制（目前没有实现的从谱系下游推论→gangmu next_actions 的自动转化）。


## 诊断要求

请执行严格根因诊断：
1. 失败现象：观测到什么？
2. 直接原因：哪行代码导致了失败？
3. 根本原因：为什么会写出那行代码？是设计错误还是实现疏忽？
4. 分类判定：实现错误 or 定义冲突？
5. 修复方案（仅限实现错误）：精确的修复步骤

如果是定义冲突，输出：
- **冲突方**：A 说什么 vs B 说什么
- **不可弥合理由**：为什么不能同时满足两边


## Response

## 严格诊断报告

### 1) 失败现象（被否定对象）
- 观测：v158 关闭多条纲目后，v159 的 `ceremony_scan` 仅产出修复/遗留工位，没有出现**新的并行研究线**。
- 期望：蜂群应继续“长出”新研究线，而不是停在“等待真实数据”。

---

### 2) 直接原因（代码级）
在 `ceremony_scan.py` 的研究线扫描逻辑 `_scan_research_lines()` 中，工位来源被硬性限定为：

- 只读取 `gangmu.yaml`
- 只处理 `status=active` 的 mu
- 只从这些 mu 的 `next_actions` 提取可执行工位

等价逻辑是：
```python
if mu.status != "active":
    continue
# only consume existing next_actions
```
并且扫描链路中没有“创建新 mu / 回写 gangmu.yaml”的分支。

**因此：`ceremony_scan` 只能消费已有研究线，不能自动生成新研究线。**

---

### 3) 根本原因（为何会写成这样）
这是**系统角色定义**导致的，不是偶然疏忽：

- 当前定义：`ceremony_scan` 是“消费与调度器”（consume existing agenda）
- 你的期望：`ceremony_scan` 兼任“研究线生成器”（synthesize new agenda）

两者是不同职责边界。当前实现严格遵守前者，所以不会自动长出新纲目。

---

### 4) 分类判定
**定义冲突（不是实现错误）**

---

### 5) 冲突说明（按要求）

#### 冲突方
- **A（现有定义）**：`ceremony_scan` 只消费 `gangmu.yaml` 的 active 条目与既有队列，不负责生成新研究线。
- **B（期望定义）**：当活跃纲目关闭/耗尽时，系统应从已结算谱系或下游推论中自动生成新研究线。

#### 不可弥合理由
- A 是“只读消费”边界；B 要求“自动生成+回写议程”能力。
- 这需要新增“推论→纲目”的规则、优先级、审批与写入机制，已超出现有 `ceremony_scan` 定义。
- 不改变定义，无法同时满足 A 与 B。

---

## 对“活跃纲目全部关闭时如何继续生长”的结论
在现定义下：不会自动生长；只会继续消费遗留/修复类来源，耗尽后进入空转（或 fallback）。  
要继续生长，必须由**外部机制**注入新 `gangmu` 条目（或先变更定义再实现自动转化器）。
