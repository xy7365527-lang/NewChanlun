# Phase 6 Roadmap Derive — 异质质询代理产出

## 推导依据

- 第五阶段全部完成（4/4）：concept_strict_closure / backtest_evaluation_framework / live_data_integration / engineering_robustness
- 当前能力边界：A系统全链路闭环 + 回测引擎基础版 + 实时数据流基础版 + B系统可视化
- 缺口来源：decide-phase6-ctx.md 四维度分析

## 优先级判定原则

- P1 = 没有它，实战使用存在根本性风险或功能缺失（不可绕过）
- P2 = 提升质量、扩展覆盖，但当前系统已可运行

---

## YAML tasks 节

```yaml
  # ─── 第六阶段任务（异质质询代理 derive，2026-02-26）──────────────────────
  # 来源：第五阶段全部完成后，基于四维度缺口分析派生
  # 诊断：回测框架已有但缺乏实战必要的风控/成本模型；实时系统缺乏生产级健壮性；
  #       前端交互停留在静态；数据源局限于美股
  # 方向：风控+成本模型（P1）→ 实时生产加固（P1）→ 前端交互（P2）→ 数据源扩展（P2）

  - id: risk_management_module
    title: "风控与仓位管理模块"
    priority: P1
    status: active
    source: "维度1（回测→实战缺口）：无风控模块，实战存在无限风险敞口"
    description: |
      当前 backtest.py 仅统计信号胜率/盈亏比，无止损逻辑和仓位管理。
      实战使用必须具备：
      1. 止损策略：固定点数止损、ATR倍数止损、结构止损（跌破前低/前高）
      2. 仓位管理：固定比例（每笔风险 = 账户 N%）、Kelly 公式
      3. 最大回撤控制：触发阈值后暂停开仓
      4. 与 backtest.py 集成：回测时应用风控规则，输出风控后的净值曲线
      风控模块必须与 A 系统解耦——仅消费 BuySellPoint 输出，不修改引擎逻辑。
    constraints:
      - "风控参数必须可配置（不硬编码），支持参数扫描"
      - "止损触发时间点不可使用未来数据（防未来函数）"
      - "与 backtest.py 集成时不修改现有回测引擎接口"
      - "结构止损依赖 A 系统中枢/笔数据，通过 RecursiveOrchestrator 输出获取"
    subtasks:
      - id: stop_loss_strategies
        title: "止损策略实现"
        description: |
          实现三种止损：
          - FixedStopLoss：入场价 ± 固定点数
          - ATRStopLoss：入场价 ± N * ATR(period)
          - StructuralStopLoss：跌破/突破最近笔的极值点
          新建 src/newchan/risk/stop_loss.py
      - id: position_sizing
        title: "仓位管理实现"
        description: |
          实现两种仓位计算：
          - FixedRiskSizing：每笔风险 = 账户 * risk_pct，仓位 = 风险金额 / 止损距离
          - KellySizing：基于历史胜率/盈亏比计算 Kelly 比例（含半 Kelly 保守版）
          新建 src/newchan/risk/position_sizing.py
      - id: drawdown_control
        title: "最大回撤控制"
        description: |
          实现账户级别的回撤监控：
          - 实时计算当前回撤（相对历史最高净值）
          - 触发阈值（如 -15%）后暂停开仓，直到净值恢复
          新建 src/newchan/risk/drawdown_guard.py
      - id: backtest_risk_integration
        title: "风控模块与回测引擎集成"
        description: |
          扩展 backtest.py，支持传入 RiskConfig（止损策略 + 仓位管理 + 回撤控制）。
          输出增加：风控后净值曲线、止损触发次数、仓位分布统计。
          新增 tests/test_risk_management.py
    decomposition: parallel

  - id: slippage_cost_model
    title: "滑点与手续费模型"
    priority: P1
    status: active
    source: "维度1（回测→实战缺口）：无成本模型，回测结果与实战存在系统性偏差"
    description: |
      当前 backtest.py 假设零滑点零手续费，导致回测结果过于乐观。
      实战必须具备：
      1. 滑点模型：固定点数滑点、百分比滑点、市场冲击模型（大单影响价格）
      2. 手续费模型：固定费率（如 0.03%）、阶梯费率（按交易量分档）
      3. 回测引擎集成：每笔交易扣除成本后计算净盈亏
      4. 成本敏感性分析：在不同成本假设下输出净值曲线对比
    constraints:
      - "成本参数必须可配置，支持不同市场（美股/期货/A股）的默认预设"
      - "市场冲击模型仅在仓位超过日均成交量 N% 时激活（避免过度复杂化）"
      - "与 risk_management_module 并行开发，最终统一集成到 backtest.py"
    subtasks:
      - id: slippage_models
        title: "滑点模型实现"
        description: |
          实现三种滑点：
          - FixedSlippage：每笔固定点数
          - PercentSlippage：成交价 * slippage_pct
          - MarketImpactSlippage：基于成交量占比的冲击模型
          新建 src/newchan/cost/slippage.py
      - id: commission_models
        title: "手续费模型实现"
        description: |
          实现两种手续费：
          - FixedRateCommission：成交金额 * rate
          - TieredCommission：按月交易量分档费率
          新建 src/newchan/cost/commission.py
      - id: cost_backtest_integration
        title: "成本模型与回测引擎集成"
        description: |
          扩展 backtest.py，支持传入 CostConfig（滑点 + 手续费）。
          输出增加：总成本统计、成本占盈利比例、成本敏感性对比图。
          新增 tests/test_cost_model.py
    decomposition: parallel

  - id: live_production_hardening
    title: "实时系统生产加固"
    priority: P1
    status: active
    source: "维度2（实时系统→生产缺口）：断线重连/多标的并发缺失，生产环境不可用"
    description: |
      当前 live_engine.py 在网络中断时无法自动恢复，多标的场景无并发管理。
      生产环境必须具备：
      1. 断线重连：指数退避策略（1s→2s→4s→...→60s），重连后状态恢复
      2. 数据缺失补偿：重连后检测 gap，从历史数据 API 补充缺失 K 线
      3. 多标的并发管理：asyncio task pool，每个标的独立 live_engine 实例
      4. 背压控制：WebSocket 推送队列满时丢弃旧帧（不阻塞引擎计算）
      5. 连接状态监控：/api/live/status 端点，返回各标的连接状态
    constraints:
      - "重连逻辑必须保证状态连续性——重连后 RecursiveOrchestrator 状态不重置"
      - "多标的并发不超过系统资源上限（默认 max_symbols=10，可配置）"
      - "背压控制不得丢弃买卖点信号帧（仅丢弃中间状态帧）"
      - "不引入新的消息格式，复用现有 WsSnapshot 结构"
    subtasks:
      - id: reconnect_with_backoff
        title: "断线重连（指数退避）"
        description: |
          在 live_engine.py 中实现断线检测和指数退避重连。
          重连后调用 data_databento_live.py 的历史补充接口填充 gap。
          修改 src/newchan/live_engine.py
      - id: multi_symbol_concurrency
        title: "多标的并发管理"
        description: |
          在 gateway.py 中实现 asyncio task pool，支持同时订阅多个标的的实时数据。
          每个标的独立 LiveEngine 实例，共享 WebSocket 广播通道。
          修改 src/newchan/gateway.py，新增 src/newchan/live_pool.py
      - id: backpressure_control
        title: "背压控制"
        description: |
          在 WebSocket 推送路径中实现有界队列（maxsize=100）。
          队列满时：丢弃非关键帧（中间状态），保留买卖点信号帧。
          修改 src/newchan/gateway.py
      - id: live_status_endpoint
        title: "实时状态监控端点"
        description: |
          新增 GET /api/live/status 端点，返回各标的连接状态、最后更新时间、重连次数。
          修改 src/newchan/server.py
    decomposition: parallel

  - id: frontend_realtime_interaction
    title: "前端实时交互升级"
    priority: P2
    status: active
    source: "维度3（前端交互缺口）：当前前端无实时更新能力，交互式回放缺失"
    description: |
      当前 server.py 提供静态文件服务，前端无法实时更新图表或控制回放进度。
      需要实现：
      1. 实时图表更新：WebSocket 驱动的 K 线/买卖点增量渲染（基于现有 WsSnapshot）
      2. 交互式回放控制：进度条拖拽、速度调节（0.5x/1x/2x/5x）、暂停/继续
      3. 多标的切换：下拉菜单切换标的，保持当前回放进度
      4. 级别切换：切换显示的递归级别（Level 0/1/2/...）
      技术选型：轻量级（原生 JS + Canvas 或 lightweight-charts），不引入重型框架。
    constraints:
      - "前端代码放入 src/newchan/static/，不引入 Node.js 构建流程"
      - "实时更新基于现有 WsSnapshot 结构，不修改后端消息格式"
      - "交互式回放复用现有 gateway.py 的 replay 逻辑，不重写后端"
      - "多标的切换不重新加载页面（SPA 模式）"
    subtasks:
      - id: realtime_chart_update
        title: "实时图表 WebSocket 驱动"
        description: |
          在前端实现 WebSocket 客户端，订阅 /ws/live/{symbol}，
          增量更新 K 线图（新增 K 线、更新最后一根 K 线）和买卖点标注。
          修改 src/newchan/static/index.html 及相关 JS
      - id: replay_interactive_control
        title: "交互式回放控制"
        description: |
          实现回放控制 UI：进度条（基于 K 线索引）、速度选择器、暂停/继续按钮。
          后端新增 POST /api/replay/control 端点（暂停/继续/跳转）。
          修改 src/newchan/server.py + src/newchan/static/
      - id: multi_symbol_switch
        title: "多标的切换 UI"
        description: |
          实现标的选择下拉菜单，切换时重新订阅对应 WebSocket 频道。
          后端新增 GET /api/symbols 端点，返回可用标的列表。
      - id: level_switch_ui
        title: "递归级别切换 UI"
        description: |
          实现级别选择器（Level 0/1/2/...），切换时过滤 WsSnapshot 中对应级别的数据渲染。
          纯前端过滤，不需要后端改动。
    decomposition: parallel

  - id: data_source_expansion
    title: "数据源扩展（A股 + 加密货币）"
    priority: P2
    status: active
    source: "维度4（数据源扩展）：当前仅美股/期货，A股和加密货币市场无法接入"
    description: |
      当前数据源：AlphaVantage（美股历史）+ Databento（期货实时）。
      扩展目标：
      1. A股数据源：akshare（免费，支持日线/分钟线）或 tushare（需 token）
         - 历史数据：data_astock.py（复用 DataProvider 接口）
         - 实时数据：A股无 tick 级实时，使用分钟级轮询（60s 间隔）
      2. 加密货币数据源：ccxt（统一接口，支持 Binance/OKX 等）
         - 历史数据：data_crypto.py（OHLCV 接口）
         - 实时数据：WebSocket 订阅（Binance WS API）
      接口约束：新数据源必须实现 DataProvider 协议（与现有 data_av.py 同接口）。
    constraints:
      - "新数据源必须实现 DataProvider 协议（get_bars/get_live_bars），不修改 A 系统"
      - "A股数据需处理停牌/除权复权问题（前复权为默认）"
      - "加密货币数据需处理 24/7 无休市场的特殊性（无开盘/收盘概念）"
      - "数据源切换通过配置（symbol 前缀或 exchange 参数），不修改引擎代码"
    subtasks:
      - id: astock_data_provider
        title: "A股数据源实现"
        description: |
          新建 src/newchan/data_astock.py，实现 DataProvider 协议。
          使用 akshare 获取 A 股历史分钟线数据（前复权）。
          处理停牌日（跳过）和除权（使用复权价格）。
          新增 tests/test_data_astock.py（使用 mock 数据，不依赖网络）
      - id: crypto_data_provider
        title: "加密货币数据源实现"
        description: |
          新建 src/newchan/data_crypto.py，使用 ccxt 实现 DataProvider 协议。
          支持 Binance OHLCV 历史数据和 WebSocket 实时订阅。
          新增 tests/test_data_crypto.py（使用 mock 数据）
      - id: data_source_e2e_validation
        title: "新数据源端到端验证"
        description: |
          在 A 股标的（如 000001.SZ）和加密货币标的（如 BTC/USDT）上运行
          RecursiveOrchestrator 全链路，验证缠论引擎对不同市场数据的适应性。
          输出验证报告（谱系记录）。
    decomposition: parallel

  - id: backtest_advanced_analytics
    title: "回测高级分析"
    priority: P2
    status: active
    source: "维度1（回测→实战缺口）：多标的横向对比和信号时效性分析缺失"
    description: |
      当前 signal_quality_report.py 仅支持单标的统计。
      扩展目标：
      1. 多标的横向对比：在 N 个标的上批量运行回测，输出对比表格（胜率/盈亏比/最大回撤）
      2. 信号时效性分析：买卖点信号从确认到最优入场的时间窗口分析
      3. 蒙特卡洛模拟：随机打乱交易顺序，评估策略稳健性（排除运气成分）
      4. 参数敏感性分析：在不同止损/仓位参数下的净值曲线对比
      这些分析工具为策略优化提供数据支撑，不修改 A 系统逻辑。
    constraints:
      - "批量回测必须支持并行执行（multiprocessing），避免串行过慢"
      - "所有分析结果输出为结构化数据（JSON/CSV），支持外部工具消费"
      - "蒙特卡洛模拟次数默认 1000 次，可配置"
    subtasks:
      - id: multi_symbol_backtest
        title: "多标的批量回测"
        description: |
          新建 scripts/batch_backtest.py，支持传入标的列表，
          并行运行 backtest.py，输出横向对比报告（CSV + 可视化）。
      - id: signal_timeliness_analysis
        title: "信号时效性分析"
        description: |
          分析买卖点信号确认后 N 根 K 线内的价格走势分布，
          确定最优入场窗口（信号确认后立即入场 vs 等待回调）。
          新建 scripts/signal_timeliness.py
      - id: monte_carlo_simulation
        title: "蒙特卡洛模拟"
        description: |
          对回测交易序列进行随机重排，生成净值曲线分布，
          计算策略在随机条件下的期望收益和风险区间。
          新建 src/newchan/analysis/monte_carlo.py
    decomposition: parallel
```

---

## 推导摘要

| id | title | priority | 来源维度 |
|----|-------|----------|---------|
| risk_management_module | 风控与仓位管理模块 | P1 | 维度1：回测→实战缺口 |
| slippage_cost_model | 滑点与手续费模型 | P1 | 维度1：回测→实战缺口 |
| live_production_hardening | 实时系统生产加固 | P1 | 维度2：实时→生产缺口 |
| frontend_realtime_interaction | 前端实时交互升级 | P2 | 维度3：前端交互缺口 |
| data_source_expansion | 数据源扩展（A股+加密货币） | P2 | 维度4：数据源扩展 |
| backtest_advanced_analytics | 回测高级分析 | P2 | 维度1：回测→实战缺口 |

**P1 判定依据：**
- `risk_management_module`：无止损/仓位管理，实战存在无限风险敞口，不可绕过
- `slippage_cost_model`：零成本假设导致回测结果系统性高估，实战决策基础失真
- `live_production_hardening`：断线无法重连，生产环境不可用（单点故障）

**P2 判定依据：**
- `frontend_realtime_interaction`：现有静态图表可用，交互升级提升体验但不阻塞
- `data_source_expansion`：美股已可用，A股/加密货币是覆盖扩展
- `backtest_advanced_analytics`：单标的回测已可运行，高级分析是质量提升

**概念层剩余缺口（baohan.md 初始方向 / bi.md wide/strict）：**
不列入第六阶段——两者均已标注为低优先级且不阻塞实战，待实战数据暴露具体问题后再结算。
