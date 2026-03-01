# 274号：废除 depth_budget 和 max_rescan_depth

- **status**: 已结算
- **settled_at**: 2026-03-01
- **type**: 架构修正
- **negation_source**: human（编排者指令）
- **negation_form**: separation（全局截断与局部依赖的分离）

## 结论

废除 depth_budget 全局截断参数和 max_rescan_depth 安全阀。递归终止条件简化为仅两个结构性判断：

1. **原子性**：当前任务不可分解为 ≥2 个独立子任务 → 直接执行
2. **不动点**：rescan 未产生新工位 → ceremony 终止

无外部计数器。无全局截断。递归深度由任务结构自然决定。

context window 耗尽 → 触发 compaction → 下一轮恢复继续。这是暂停，不是终止。

## 定义依据

- depth_budget 是从全局强加的截断——与272号裁定中否定掉的全局排序是同一类错误
- "附庸的附庸不是我的附庸"（272号）→ 每层只管自己的直接依赖，全局截断越级干预子蜂群内部
- 与缠论同构：笔是原子性终止（不可再分解），不动点是区间套收敛终止

## 边界条件

- context window 耗尽不是终止条件而是 compaction 触发条件——如果 compaction 机制本身失效（平台层故障），递归将被迫停止（但这是平台故障，不是架构设计的截断）
- 视差 Gap 随递归层数累积（约束3 × 约束2 的历时效应，094号），这是结构性代价，不是截断理由

## 下游推论

- ceremony.md 步骤5：递归判断块移除 depth_budget 参数
- ceremony.md 步骤11：移除 max_rescan_depth=3 安全阀，rescan 终止条件仅为不动点
- sub-swarm-ceremony SKILL.md：递归深度章节重写为结构性终止
- agent-team-enforce.sh：spawn 基因检查从三基因降为两基因（topo_address + parent_callback）
- dispatch-dag.yaml：required_genes 移除 depth_budget，recursion_rules 更新终止条件

## 影响声明

修改文件：
- `.claude/commands/ceremony.md` — 移除 depth_budget 模板变量 + 废除 max_rescan_depth
- `.claude/skills/sub-swarm-ceremony/SKILL.md` — 递归深度章节重写
- `.claude/hooks/agent-team-enforce.sh` — 三基因→两基因
- `.chanlun/dispatch-dag.yaml` — required_genes 移除 depth_budget + recursion_rules 更新

## 谱系引用

- 272号：RTAS三主题裁定——"附庸的附庸不是我的附庸"局部依赖原则
- 094号：创世 Gap 递归化——视差 Gap 累积是结构性代价
- 069号：递归拓扑异步自指蜂群定义
- 056号：递归是存在方式，线性是退化特例
