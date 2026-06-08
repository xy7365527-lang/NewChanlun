# 第二阶段：修复后引擎 streaming 回测（QQQ 日线）

> 数据：QQQ 日线最近 1000 根（yfinance，2022-06-03→2026-05-29），零前视逐根喂入  
> 初始资金：$10,000，无杠杆  
> 认识论等级：L2（真实数据，可否证）

## 0. 核心对比

| 组 | 门控 | 买信号 | 被否定 | 卖信号 | 成交笔 | 胜率 | 总收益% | 末值 |
|----|------|--------|--------|--------|--------|------|---------|------|
| A | 无 | 0 | — | 0 | 0 | 0.0% | 0.0% | $10,000 |
| B | PH settle | 0 | 0 | 0 | 0 | 0.0% | 0.0% | $10,000 |
| 买入持有 | — | — | — | — | 1 | — | 141.12% | $24,112 |

## 1. A组交易明细（无门控）

（无交易）

## 2. B组交易明细（PH settle 门控）

（无交易）

## 3. 对比 TV 137 个买卖点

- 引擎日线 confirmed 买卖信号（A组）：买 0 + 卖 0 = 0
- TV CZSC 基准：137 个买卖点

## 4. 结果包六要素

**结论**：见 §0 对比表。这是天然零前视的因果回测。

**定义依据**：confirmed 买卖点 = 所在 Move.settled（maimai #2，a_buysellpoint_v1）。Type3 = 中枢突破回试 + Move.settled（不依赖背驰）。PH settle 门控 = OnlineMergeTree.trend_health 否定性必要条件（426号）。

**边界条件**：
- 若 confirmed 买卖点数 = 0，则 A/B 组无交易，等同空仓 0% 收益——结论翻转条件是引擎在该窗口产出 ≥1 个 confirmed 买点。
- 若改用更细级别数据（30min/5min），笔/线段/中枢数量级上升，confirmed 买卖点应随之增加（见 §5 根因）。

**下游推论**：日线单级别引擎的 confirmed 信号密度由该尺度的笔→线段几何压缩比（≈12:1）决定，与窗口长度无关。要达到 TV 137 量级需多级别递归或更细级别数据。

**谱系引用**：267号（cost_reduction_fsm）；426号（PH 对象否定对象）；engine_vs_tv_comparison.md（5项修复）；formalization-validity-domain（L2）。

**影响声明**：新建 scripts/streaming_backtest.py、scripts/_qqq_data.py、scripts/streaming_engine_verify.py；引擎改动见 §6。

## 5. 根因分析（为何日线 confirmed 信号稀疏）

修复后日线 1000 根产出：72 笔 → 6 线段 → 1 中枢 → 1 走势 → 1 递归层。

1. **几何压缩**：日线笔跨度大（中位 raw_gap≈7），1000 根 ≈ 72 笔，每线段需 ≥3 笔重叠 → 仅 6 线段。6 线段只够定义 1 个中枢，不足以形成多中枢趋势 → confirmed 走势稀少。
2. **Type3 触发缺口（独立发现，非本任务 5 问题）**：存在一个 settled 中枢（[342.35, 387.98]，向上突破），seg4 回试 low=402.39 > zg 未跌回中枢——结构上是标准第三类买点，但 `_detect_type3` 因 `break_seg` 指向回试段本身（而非突破段）从 break_seg+1 找回试段失败。此疑点涉及 a_zhongshu_v1 的中枢延伸/突破段定义，按 no-workaround 规则未在本任务擅自修改，已另行标记独立诊断。

## 6. 引擎改动清单（本任务）

| 文件 | 改动 | 对应问题 |
|------|------|---------|
| a_stroke.py | `_check_gap`/`strokes_from_fractals` 加 `new_raw_gap_min` 参数（默认3） | #4 笔gap |
| bi_engine.py | BiEngine 透传 `new_raw_gap_min`；Snapshot 暴露 `merged_to_raw` | #3 #4 |
| core/recursion/buysellpoint_engine.py | `process_snapshots` 接 `df_macd`/`merged_to_raw` 透传背驰 | #3 MACD |
| orchestrator/recursive.py | 透传 `new_raw_gap_min`；`enable_macd_divergence` + 增量 OnlineMacdState | #3 #4 |
| （调用层）窗口截断 1000 根 | streaming_backtest.py / streaming_engine_verify.py | #1 尺度 |
| （调用层）`reset_dir_on_fractal=True` | 同上 | #2 包含 |
| （已存在）RecursiveStack 递归层 | 无需改 | #5 递归 |