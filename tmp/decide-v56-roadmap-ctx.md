# v56-swarm Roadmap Decide 上下文

## 任务

基于"对象矛盾驱动"原则，派生 v56-swarm 下一阶段 roadmap 目标。

## 系统当前状态

- 14 个定义全部已结算（v1.0-v1.6）
- 216 个谱系已结算
- 1975 测试通过
- A 系统（形式化引擎）已闭环
- B 系统（可视化/交互）部分升级

## v55-swarm 已完成的 8 个目标

1. level_recursion — 级别递归概念设计 ✅
2. real_data_validation — 真实数据端到端验证 ✅
3. buysellpoint_module — 三类买卖点模块 ✅
4. xiaozhuan_da — 小转大机制 ✅
5. definition_status_sync — 定义文件状态同步 ✅
6. bi_mode_new_default — 笔引擎 mode='new' 默认 ✅
7. replay_engine_upgrade — 回放引擎升级到 RecursiveOrchestrator（单TF路径）✅
8. nested_divergence_endpoint — 嵌套背驰端点 + bridge 集成 ✅

## 已识别的系统断点（A→B 桥梁）

### 断点1：BSP 可视化缺失

- `ab_bridge_newchan.py` 第296行返回 `"bsp"` 字段（已实现）
- `b_plot.py` 无任何 BSP 渲染代码（grep 结果为空）
- `b_chart.py` 无任何 BSP 渲染代码（grep 结果为空）
- 结论：买卖点数据存在于 API 响应中，但前端完全不渲染

### 断点2：多TF回放路径未升级

- `replay.py` 单TF路径已升级到 RecursiveOrchestrator（v55完成）
- `replay.py` 多TF路径（TFOrchestrator）grep 无结果——未升级
- `b_timeframe.py` 存在但与 RecursiveOrchestrator 的集成状态未知

### 断点3：递归深度验证上限

- 215号：AV 1min 数据产生 depth=2（Level 1 可达）
- T7 验证需要 depth≥3（Level 2 中枢）
- 当前 20 只美股标的中最大 depth=2，T7 仍为空
- 需要更长时间窗口数据或更多标的

### 断点4：T6/T7 已集成但未在真实数据上大规模验证

- `a_divergence_v1.py` 已实现 T6/T7 OR 逻辑（三维度综合判断）
- 215号 T8 通过率 65.4%（53/81）——T6/T7 贡献未单独统计
- 需要分析 T6/T7 在真实数据上的触发率和准确率

### 断点5：区间套端点缺乏实时性

- `/api/nested_divergence` 端点已实现（v55完成）
- 但每次请求全量重算（无缓存/增量策略）
- 大数据窗口可能超时（nested_divergence 任务 constraints 已记录）

## 定义文件中的未完成项

### zhongshu.md

- 中枢扩展充要条件验证：`待验证`（第434行）
- 问题 #2（扩展充要条件）暂搁待走势类型模块（第480行）
- 走势类型模块已完成（zoushi.md v1.6 已结算）——此依赖已满足

### level_recursion.md

- TF映射标记为工程债（纯展示层需求，核心引擎不依赖）
- 状态：已结算，但 TF 展示层未实现

## gangju 分析结果

- `empty_mu`: "RTAS 未产出新区块"（delta_blocks=0，当前=246）
- `new_mu`: 空
- 含义：系统处于相变后的稳定态，需要新的矛盾驱动

## 最近谱系演化方向（200-216号）

- 211号：递归 direction 语义缺口（已修复）
- 212号：质询深度相变（RTAS 正常节奏）
- 213号：弱质询误报校准
- 214号：递归级别不可达（被215号否定）
- 215号：AV 1min 大规模验证——Level 1 可达确认
- 216号：v55-swarm 元观察——异质诊断相关失败 + 任务粒度默认偏小

## 编排者指令

"沿总纲领一步步前进，从一个目到下一个目"

总纲领方向：A 系统（形式化）→ B 系统（可视化/交互）→ 实战可用产品

## 输出要求

派生 v56-swarm roadmap 目标，格式：

```yaml
tasks:
  - id: xxx
    title: "..."
    priority: P1/P2
    status: active
    source: "v56-swarm Gemini decide（第四阶段派生）"
    description: |
      ...
    constraints:
      - "..."
    validation_cmd: "..."
    relevant_files:
      - "..."
    subtasks:
      - id: xxx
        title: "..."
        description: "..."
        relevant_files:
          - "..."
    decomposition: parallel
```

要求：
- 每个目标必须有 validation_cmd（VDW 机制）
- 每个目标必须有 relevant_files
- 复杂目标必须拆 subtasks + decomposition: parallel
- 不存在"长期"或"后续"任务——RTAS 用数量递归解决复杂性
- 目标来自已存在的概念矛盾或状态跃迁，不是覆盖率/代码行数驱动
