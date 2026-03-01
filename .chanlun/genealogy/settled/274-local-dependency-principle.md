---
id: 274
type: 语法记录
status: 已结算
date: "2026-02-28"
source: "v76-swarm/local-dep-discuss（编排者272号裁定下游推论）"
negation_source: "编排者 INTERRUPT——否定 Lead 全局排序所有工位优先级的行为"
negates: ["218号（扩展：218割掉串行尾巴，274割掉全局排序思维）"]
tensions_with: []
topo_effect: ""
downstream_inferences:
  - id: 1
    description: "ceremony_scan.py 的 priority 字段语义更新为来源标记——如未来有消费者需要排序语义，必须新建字段而非复用 priority"
    status: "resolved（本谱系已执行）"
  - id: 2
    description: "Lead spawn 工位时的全部并行要求已写入 lead-parallel-dispatch.md——与218号并行化规则合并"
    status: "resolved（本谱系已执行）"
  - id: 3
    description: "Gemini 矛盾点2（Lead 管理 depends_on 仍是中心化调度）在当前平台约束下的处理：工位无法自挂起等待前置条件，Lead 管理直接子工位的 spawn 是平台约束下的合法行为，但 Lead 不越级管理子蜂群内部的依赖"
    status: "resolved（274号边界条件：平台约束下的局部依赖 = Lead 只管直接子工位，不管子蜂群内部）"
  - id: 4
    description: "Gemini 矛盾点3（资源受限时扫描顺序坍缩为全局排序）——资源受限时由平台排队处理，Lead 不做二次过滤"
    status: "resolved（写入 lead-parallel-dispatch.md 消费规则第4条）"
---

# 274号：局部依赖原则——割掉全局排序思维

## 核心命题

**"附庸的附庸不是我的附庸"**——每个节点只管自己的直接依赖。全局 DAG 是局部依赖自然涌现的结果，不是事先规划的。

## 否定对象

编排者否定了 Lead 试图全局排序所有工位优先级的行为。这是218号（Lead并行化）的根本形式：
- 218号：即使你决定了顺序，也不应该串行执行
- 274号：你根本不应该决定全局顺序

274号比218号更根本——如果你不做全局排序，串行就自然消失了。

## 操盘方法同构

缠论操盘方法中的级别隔离原则：
- 主级别操作者只关注本级别买卖点（直接依赖）
- 次级别的短差操作由次级别走势自行管理（次级别节点的局部依赖）
- 主级别不越级干预次次级别
- 整个多级别协同是每层各管各的直接下级的涌现结果

## Gemini 异质质询摘要

Gemini（gemini-3.1-pro-preview）提出三个矛盾点：

1. **priority 字段的"自描述"语义是逻辑上的自欺欺人**：无消费者的字段是死代码或全局排序后门。verdict=contradictory。
   - 本谱系处理：承认 Gemini 的批评有效。priority 字段保留但语义明确为"来源标记"（标记工位来自 roadmap/session/structural/fallback），不产生排序效力。如果未来需要排序语义，必须新建字段。

2. **Lead 管理 depends_on 仍是中心化调度**：真正的局部依赖应该让工位自行等待前置条件。verdict=contradictory。
   - 本谱系处理：Gemini 的批评在理论上有效，但在当前 Claude Code Agent Teams 平台约束下，工位被 spawn 后立即执行，没有"挂起等待"机制。因此局部依赖原则的可实现形式是：Lead 只管直接子工位的 spawn/shutdown，不越级管理子蜂群内部。这与操盘方法同构更准确。

3. **扫描顺序在资源受限时坍缩为全局排序**：先被扫描到的 roadmap 任务会优先占用资源。verdict=needs_work。
   - 本谱系处理：承认副作用存在。处理方式：资源受限时由平台排队处理，Lead 不做二次过滤/截断——这比 Lead 做二次过滤更符合局部依赖原则（Lead 不做全局评估）。

## 代码变更

1. `scripts/ceremony_scan.py`：
   - 注释更新：priority 字段语义从"全局优先级"明确为"来源标记"
   - "扫描顺序（优先级递减）" → "扫描来源（按发现策略排列，不是工位间的执行排序）"
   - "最高优先级" → "首先扫描"

2. `.claude/rules/lead-parallel-dispatch.md`：
   - 标题更新：增加"局部依赖"和274号引用
   - 新增"局部依赖原则"完整章节
   - 新增"禁止的全局排序行为"4条
   - 新增"ceremony_scan 输出的消费规则"4条

## 边界条件

- 当平台支持工位自挂起机制时，Gemini 矛盾点2 需要重新评估——彼时 Lead 应该将依赖管理权完全下放给工位
- priority 字段如果未来有新的消费者需要排序语义，必须新建字段（如 execution_order），不能复用 priority
- 资源受限时的降级行为由平台决定，Lead 不介入——但如果平台的排队策略系统性地饿死某类工位，需要新的谱系记录

## 影响声明

- 修改了 `.claude/rules/lead-parallel-dispatch.md`（语法规则更新）
- 修改了 `scripts/ceremony_scan.py`（注释语义澄清，无逻辑变更）
- 不影响任何代码运行行为——本谱系是语法记录（行为规则的显式化），不是功能变更
