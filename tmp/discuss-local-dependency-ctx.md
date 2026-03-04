# 局部依赖原则讨论上下文

## 编排者原话

272号裁定："附庸的附庸不是我的附庸"——每个节点只管自己的直接依赖。全局 DAG 是局部依赖自然涌现的结果，不是事先规划的。

编排者指出这和操盘方法同构：主级别持仓不管次次级别的短差。次级别管次次级别，主级别只管次级别。每层只看自己的直接下级。

## 操盘方法同构论证

缠论操盘方法中的级别隔离原则：
- 主级别操作者只关注**本级别买卖点**（直接依赖）
- 次级别的短差操作由**次级别走势**自行管理（次级别节点的局部依赖）
- 主级别不越级干预次次级别（"附庸的附庸不是我的附庸"）
- 整个多级别协同是每层各管各的直接下级的**涌现结果**

映射到蜂群架构：
- Lead（主级别）只管 spawn 直接 teammate（次级别），不管 teammate 的子蜂群
- Teammate（次级别）自主决定是否 spawn 子蜂群，以及子蜂群内部的工位分配
- 全局蜂群 DAG 是各层局部依赖的涌现结果，Lead 不需要（也不应该）知道完整的全局拓扑

## 当前蜂群架构中的全局排序残余分析

### 残余 1：ceremony_scan.py 的硬编码优先级线性扫描

文件头部自己声明（第12行、第18行）：
```
089号声明：当前为硬编码优先级扫描，不是 DAG 拓扑排序。
本脚本以优先级线性扫描实现（roadmap → session → fallback）——这是有意的工程选择。
```

第912-913行：
```python
# 扫描顺序（优先级递减）：
# 1. roadmap.yaml（最高优先级，结构化业务目标）
# 2. session 遗留项 + pending 谱系
# 3. no_work_fallback（测试失败等）
```

**分析**：这实际上是**扫描来源**的优先级，不是**工位之间**的排序。roadmap → session → fallback 是"从哪里发现工位"的顺序，不是"工位之间谁先谁后"。发现完所有工位后，它们被合并到一个 workstations 列表中，Lead spawn 时应该并行 spawn 所有工位——每个工位只有与自己相关的局部依赖。

**判定**：扫描来源的优先级不违反局部依赖原则。但 priority 字段（P0/P1/P2/P3）暗示了全局排序——如果 Lead 按 priority 排序后串行 spawn，则违反。如果 Lead 并行 spawn 所有工位（218号已要求），priority 只是工位的元数据标签，不产生排序效力。

### 残余 2：priority 字段的语义歧义

ceremony_scan.py 输出的每个工位都有 priority 字段：
- P0：topo_effect 待执行、谱系编号异常
- P1：roadmap active 任务、meta-observer、gangju-audit
- P2：topology-analyst、session遗留、下游推论
- P3：session 长期项

**问题**：priority 字段是给谁看的？
- 如果是给 Lead 做全局排序用 → 违反局部依赖原则（Lead 越级排序所有工位）
- 如果是工位的自描述元数据（"我认为自己的紧急程度"）→ 不违反，但需要明确语义

### 残余 3：dispatch-dag.yaml 的 ceremony_sequence

ceremony_sequence 定义了 cold_start 和 warm_start 的 DAG（scan-definitions → scan-genealogy → ... → derive-work → spawn-tasks）。
这是 **ceremony 自身的执行序列**，不是工位之间的排序。ceremony 完成后，spawn 的工位之间没有全局排序。

**判定**：ceremony_sequence 不违反局部依赖原则。

### 残余 4：depth_budget 的全局参数性质

depth_budget 是从顶层传入的整数，每层递归减1。

**分析**：depth_budget 的传递是局部的——父节点传给子节点 `depth_budget - 1`，子节点不知道祖父节点的 depth_budget 是多少。这已经是局部依赖的形式。

**但**：depth_budget 的初始值是全局决定的（由 ceremony 或编排者设定）。这个初始值是全局参数吗？按局部依赖原则，每层应该自行决定"还要不要继续递归"——不是因为预算用完了，而是因为任务已经不可再分。

**判定**：depth_budget 作为资源约束是合理的（防止无限递归），但它不应该是递归终止的**主要**判据。主要判据应该是：
1. 任务不可分解（局部判断）
2. 020号背驰+分型（局部信号）

depth_budget 只是安全网（铁笼子），不是终止条件（铁壁垒）。164号已经做了这个区分。

### 残余 5：Lead 对工位的全局排序行为

编排者刚刚否定了 Lead 试图全局排序所有工位优先级的行为。这是最直接的违反。

Lead 应该做的：
1. 从 ceremony_scan 获取 workstations 列表
2. 识别每个工位的**局部依赖**（depends_on 关系）
3. 无依赖的工位**全部并行 spawn**
4. 有依赖的工位在依赖完成后自动 spawn

Lead 不应该做的：
- 比较工位之间的 priority 来决定 spawn 顺序
- 对所有工位做全局排序
- 越级判断某个工位"不重要"或"可以后做"

## 讨论问题

### Q1：ceremony_scan.py 的 priority 字段是否应该删除？

选项A：删除 priority 字段——它暗示全局排序，与局部依赖原则矛盾。
选项B：保留 priority 但重定义语义——priority 是工位的自描述紧急程度标签，不产生排序效力。Lead 不按 priority 排序 spawn。
选项C：将 priority 替换为 depends_on——显式声明工位之间的局部依赖关系。

### Q2：Lead 的 RTAS 循环中是否存在隐含的全局排序？

当前 Lead 的行为模式：
1. 读取 ceremony_scan 输出
2. spawn 工位（如果按 priority 排序 spawn = 违反）
3. 等待工位完成
4. rescan

其中步骤2如果 Lead 根据 priority 字段做了排序（先 P0 再 P1 再 P2），则是全局排序残余。218号已要求并行 spawn，但 priority 字段的存在可能诱导 Lead 做排序。

### Q3：depth_budget 是否已经是局部的？

depth_budget 的传递是局部的（父→子减1），但初始值是全局的。这个全局初始值是否可以替换为局部判断？

可能的替代：每层自行判断是否递归——如果任务可分解为 ≥2 个独立子任务，且当前层有资源（可以 spawn agent），则递归。不需要全局 budget。

但 164号已经区分了铁壁垒终止（局部信号）和铁笼子终止（全局约束/资源耗尽）。depth_budget 是铁笼子，合法存在但不应是主要终止条件。

### Q4：与218号（Lead并行化）的关系

218号割掉的是 Lead 的**串行执行尾巴**——Lead 不应该串行处理独立工位。
本原则（272号）割掉的是**全局排序思维**——Lead 不应该对所有工位做优先级排序。

两者是互补的：
- 218号：即使你决定了顺序，也不应该串行执行
- 272号：你根本不应该决定全局顺序

272号比218号更根本——如果你不做全局排序，串行就自然消失了。

## 与缠论定义偏序的关系

定义偏序（dispatch-dag.yaml §9）是**概念之间的逻辑依赖**（baohan → fenxing → bi → ...），不是全局排序。每个概念只声明自己的直接依赖（depends_on），全局偏序是局部依赖的涌现结果。

这与"附庸的附庸不是我的附庸"完全同构：
- baohan 不知道 bi 的存在（bi 通过 fenxing 间接依赖 baohan）
- 每个概念只管自己的直接前置
- 全局偏序图是自然涌现的

## 结论与建议

1. ceremony_scan.py 的扫描来源顺序（roadmap → session → fallback）不违反——这是"在哪里找工位"的策略，不是工位间的排序
2. priority 字段的语义需要明确：它是工位的自描述标签，不产生全局排序效力
3. Lead spawn 工位时不应该按 priority 排序，应该识别 depends_on 关系后并行 spawn 所有无依赖工位
4. depth_budget 作为安全网合法存在，但不是递归终止的主要判据
5. 局部依赖原则是218号（并行化）的更根本形式

---
待质询：Gemini challenge + Codex 代码审查
