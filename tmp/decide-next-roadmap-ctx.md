# 纲领下一目决策上下文

## 系统当前状态

### roadmap.yaml 所有任务已 completed
- level_recursion ✅ (RecursiveLevelEngine + RecursiveStack + RecursiveOrchestrator, 106 tests)
- real_data_validation ✅ (AAPL 1min 端到端验证)
- buysellpoint_module ✅ (a_buysellpoint_v1.py, 79 tests, 三类买卖点)
- xiaozhuan_da ✅ (a_xiaozhuan_da.py, 11 tests)
- definition_status_sync ✅
- bi_mode_new_default ✅ (mode='new' 为默认, 1435 tests passed)
- recursion_termination_consistency ✅

### 形式化层状态
- 14 个定义全部已结算：baohan, fenxing, bi, bijia, dengjia, level_recursion, liuzhuan, maimai, qushi, xianduan, zhongshu, zoushi, beichi
- 215 个谱系已结算（最新：215号 AV 1min 递归 Level 1 可达确认）
- Layer 1-3 审核通过（T6/T7/T8 拓扑化）
- gauge 经验验证 120/120 通过
- 真实数据验证：20 只美股，T8 通过率 65.4%（53/81），11/20 标的达到 depth=2

### 代码模块清单（src/newchan/）
- a_inclusion.py — 包含关系
- a_fractal.py — 分型
- a_stroke.py — 笔引擎（mode='new' 默认）
- a_segment_v1.py — 线段
- a_zhongshu_v1.py — 中枢
- a_move_v1.py — 走势类型
- a_divergence_v1.py — 背驰
- a_buysellpoint_v1.py — 三类买卖点（336行）
- a_xiaozhuan_da.py — 小转大
- a_recursive_engine.py — 递归引擎
- a_topology.py — 拓扑不变量（T6/T7/T8）
- a_macd.py — MACD 辅助
- b_chart.py — 可视化（HTML/JS）
- b_plot.py — 图表输出
- ab_bridge_newchan.py — 桥接层（含 signal 相关）
- nested_pipeline.py — 嵌套管线
- flow_relation.py — 资金流向（含 signal）
- indicators.py — 指标（含 signal）
- data_av.py — Alpha Vantage 数据源

### 测试规模
- 118 个测试文件，1435+ 测试

### 已知 TBD（生成态，来自 a_buysellpoint_v1.py）
- [TBD-1] 下跌确立条件（严格 vs 宽松口径）
- [TBD-4] 盘整背驰与买卖点
- [TBD-5] 中枢范围（ZG/ZD 固定 vs 动态）

### 最新谱系（215号）下游推论状态
- T6/T7 验证：SPY 1min 产生首个 T6 结果（inconclusive，bottleneck_bounded=false）
- T7 仍为空（需要 depth≥3）
- 递归引擎在真实 1min 数据上正确运行

## 问题

### 1. 下一个目应该是什么？
从缠论形式化系统的整体逻辑看，完成了：
- 定义偏序链（14个定义）
- 递归引擎（RecursiveOrchestrator）
- 拓扑化验证（T6/T7/T8）
- 三类买卖点模块
- 真实数据验证（20只美股）

系统的自然下一步是什么？

### 2. 拖延的结构性原因
为什么系统在 215 个谱系后停滞？RTAS delta_genealogy=0，循环未产出新谱系。
可能原因：
a. 缺少明确的下一个业务目标？
b. 蜂群在自指循环中消耗资源（元层过重）？
c. 形式化和实际应用之间的鸿沟？
d. 其他？

### 3. 具体的 roadmap 提案
给出 2-3 个具体的 P1 任务，以 roadmap.yaml 格式。

## 约束
- 不要提出蜂群架构改进（元层够了）
- 聚焦业务价值——缠论系统的用户需要什么
- 从代码仓库已有的能力出发

## 缠论业务背景
缠论是一套完整的股票/期货技术分析体系，核心价值：
1. 识别走势结构（笔→线段→中枢→走势类型）
2. 判断背驰（趋势力度衰竭）
3. 给出买卖点（三类买卖点）
4. 多级别联动分析（小转大）

用户需要的最终产出：在真实市场数据上，给出可操作的买卖信号，并能可视化展示走势结构。
