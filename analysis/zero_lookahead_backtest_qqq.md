# 严格零前视回测：QQQ 5分钟缠论信号（消除三层前视偏差）

**认识论等级**：L2 —— 真实 TV Replay 信号 + 真实 AlphaVantage OHLCV 执行价，单标的（QQQ）单时段，结果可否证。

## 0. 三层前视偏差消除（本回测的核心命题）

| 层 | 前视类型 | 消除手段 | 本回测落实 |
|----|---------|---------|-----------|
| 1 | 执行价前视 | 用真实 OHLCV 收盘价，不用 TV 标注价 | AV 5min `TIME_SERIES_INTRADAY`，执行价 = 信号 bar 的**下一根 bar 收盘价**（lag=1）|
| 2 | 信号确认前视 | 用 Replay '信号首次出现' step | 预期→确认转换 / 新 frontier 确认 |
| 3 | 信号集前视 | Replay 逐 bar 产出信号 | 每步只见当时图上 label，非事后全图 |

## 1. 数据元信息

| 字段 | 值 |
|------|---|
| 品种 | NASDAQ:QQQ |
| 周期 | 5 分钟（每 replay step = 1 根 5min K 线）|
| Replay 起始 | 2026-04-13 14:59 UTC |
| Replay 末端 | 2026-04-21 23:09 UTC |
| Replay 步数 | 1251 |
| AV OHLCV 源 | AlphaVantage TIME_SERIES_INTRADAY 5min 2026-04（4032 bars, EDT）|
| 末端估值价（AV 真实） | 646.8437 @ 2026-04-21 23:05 UTC |

## 2. 执行价前视偏差量化（第 1 层消除的实证）

下表对每个零前视信号，对比 **TV 标注价**（旧口径，含执行价前视）与 **AV 真实 lag=1 收盘价**（新口径）。偏差 = AV − TV标注。

- 信号数（可执行）：**22**
- 平均偏差（含符号）：**-0.1600**
- 平均绝对偏差：**1.3189**（约 0.204% of 价）
- 最大绝对偏差：**4.6400**

| 步 | 时间(UTC) | 信号 | 事件 | TV标注价 | AV真实执行价(lag=1) | 偏差 |
|----|----------|------|------|---------|-------------------|------|
| 127 | 2026-04-14 09:34 UTC | 1卖(盘整) 1 | SUB_LEVEL_SELL_POINT | 620.64 | 619.9700 | -0.6700 |
| 162 | 2026-04-14 12:29 UTC | 1卖 0 | SUB_LEVEL_SELL_POINT | 621.15 | 620.6627 | -0.4873 |
| 242 | 2026-04-14 19:09 UTC | 3买 -1 | BUY_POINT_CONFIRMED | 625.68 | 627.4300 | +1.7500 |
| 267 | 2026-04-14 21:14 UTC | 1卖 1 | SUB_LEVEL_SELL_POINT | 628.60 | 628.2900 | -0.3100 |
| 318 | 2026-04-15 09:29 UTC | 1卖(趋势) 0 | MAIN_LEVEL_SELL_POINT | 629.50 | 628.4371 | -1.0629 |
| 352 | 2026-04-15 12:19 UTC | 1卖(趋势) 1 | MAIN_LEVEL_SELL_POINT | 629.60 | 628.0902 | -1.5098 |
| 415 | 2026-04-15 17:34 UTC | 3买 -2 | BUY_POINT_CONFIRMED | 630.95 | 633.3300 | +2.3800 |
| 457 | 2026-04-15 21:04 UTC | 1卖 3 | SUB_LEVEL_SELL_POINT | 637.67 | 637.0100 | -0.6600 |
| 461 | 2026-04-15 21:24 UTC | 2卖 0 | SUB_LEVEL_SELL_POINT | 637.23 | 637.0100 | -0.2200 |
| 487 | 2026-04-15 23:34 UTC | 1卖(趋势) 0 | MAIN_LEVEL_SELL_POINT | 638.14 | 638.1500 | +0.0100 |
| 491 | 2026-04-15 23:54 UTC | 3买 0 | BUY_POINT_CONFIRMED | 637.81 | 638.3350 | +0.5250 |
| 500 | 2026-04-16 08:39 UTC | 1卖 0 | SUB_LEVEL_SELL_POINT | 639.80 | 639.0693 | -0.7307 |
| 514 | 2026-04-16 09:49 UTC | 2卖 0 | SUB_LEVEL_SELL_POINT | 639.68 | 639.4400 | -0.2400 |
| 565 | 2026-04-16 14:04 UTC | 2卖 1 | SUB_LEVEL_SELL_POINT | 639.56 | 636.9400 | -2.6200 |
| 636 | 2026-04-16 19:59 UTC | 3买 -1 | BUY_POINT_CONFIRMED | 635.26 | 639.6000 | +4.3400 |
| 795 | 2026-04-17 17:14 UTC | 1卖(趋势) 0 | MAIN_LEVEL_SELL_POINT | 650.00 | 648.4500 | -1.5500 |
| 860 | 2026-04-17 22:39 UTC | 2卖 2 | SUB_LEVEL_SELL_POINT | 649.22 | 648.8450 | -0.3750 |
| 922 | 2026-04-20 11:49 UTC | 2买 0 | SUB_LEVEL_BUY_POINT | 645.23 | 645.7800 | +0.5500 |
| 978 | 2026-04-20 16:29 UTC | 2买 0 | SUB_LEVEL_BUY_POINT | 644.10 | 645.4225 | +1.3225 |
| 1152 | 2026-04-21 14:59 UTC | 2卖 1 | SUB_LEVEL_SELL_POINT | 650.20 | 645.5600 | -4.6400 |
| 1203 | 2026-04-21 19:14 UTC | 2买 0 | SUB_LEVEL_BUY_POINT | 644.78 | 646.6500 | +1.8700 |
| 1222 | 2026-04-21 20:49 UTC | 1卖(盘整) 0 | SUB_LEVEL_SELL_POINT | 647.97 | 646.7785 | -1.1915 |

## 3. A 组（纯缠论）：cost_reduction_fsm，全部信号执行

初始资金 100,000 USD，无融资，sub_ratio=0.3。执行价全部为 AV 真实 lag=1 收盘价。

### 信号处理日志

| 步 | 事件 | 文本 | 执行价 | FSM 新状态 |
|----|------|------|--------|-----------|
| 127 | SUB_LEVEL_SELL_POINT | 1卖(盘整) 1 | 619.9700 | SKIP(IllegalTransitionError) |
| 162 | SUB_LEVEL_SELL_POINT | 1卖 0 | 620.6627 | SKIP(IllegalTransitionError) |
| 242 | BUY_POINT_CONFIRMED | 3买 -1 | 627.4300 | POSITION_OPEN |
| 267 | SUB_LEVEL_SELL_POINT | 1卖 1 | 628.2900 | COST_REDUCING |
| 318 | MAIN_LEVEL_SELL_POINT | 1卖(趋势) 0 | 628.4371 | SCANNING(reset,线段转空退出) |
| 352 | MAIN_LEVEL_SELL_POINT | 1卖(趋势) 1 | 628.0902 | SKIP(IllegalTransitionError) |
| 415 | BUY_POINT_CONFIRMED | 3买 -2 | 633.3300 | POSITION_OPEN |
| 457 | SUB_LEVEL_SELL_POINT | 1卖 3 | 637.0100 | COST_REDUCING |
| 461 | SUB_LEVEL_SELL_POINT | 2卖 0 | 637.0100 | SKIP(IllegalTransitionError) |
| 487 | MAIN_LEVEL_SELL_POINT | 1卖(趋势) 0 | 638.1500 | SCANNING(reset,线段转空退出) |
| 491 | BUY_POINT_CONFIRMED | 3买 0 | 638.3350 | POSITION_OPEN |
| 500 | SUB_LEVEL_SELL_POINT | 1卖 0 | 639.0693 | COST_REDUCING |
| 514 | SUB_LEVEL_SELL_POINT | 2卖 0 | 639.4400 | SKIP(IllegalTransitionError) |
| 565 | SUB_LEVEL_SELL_POINT | 2卖 1 | 636.9400 | SKIP(IllegalTransitionError) |
| 636 | BUY_POINT_CONFIRMED | 3买 -1 | 639.6000 | SKIP(IllegalTransitionError) |
| 795 | MAIN_LEVEL_SELL_POINT | 1卖(趋势) 0 | 648.4500 | SCANNING(reset,线段转空退出) |
| 860 | SUB_LEVEL_SELL_POINT | 2卖 2 | 648.8450 | SKIP(IllegalTransitionError) |
| 922 | SUB_LEVEL_BUY_POINT | 2买 0 | 645.7800 | SKIP(IllegalTransitionError) |
| 978 | SUB_LEVEL_BUY_POINT | 2买 0 | 645.4225 | SKIP(IllegalTransitionError) |
| 1152 | SUB_LEVEL_SELL_POINT | 2卖 1 | 645.5600 | SKIP(IllegalTransitionError) |
| 1203 | SUB_LEVEL_BUY_POINT | 2买 0 | 646.6500 | SKIP(IllegalTransitionError) |
| 1222 | SUB_LEVEL_SELL_POINT | 1卖(盘整) 0 | 646.7785 | SKIP(IllegalTransitionError) |

### 已完成交易

| 买入步 | 买入价 | 卖出步 | 卖出价 | P&L% | 退出原因 |
|--------|--------|--------|--------|------|---------|
| 242 | 627.4300 | 318 | 628.4371 | +0.16% | EXIT(线段转空,from=COST_REDUCING,cost_basis=627.43) |
| 415 | 633.3300 | 487 | 638.1500 | +0.76% | EXIT(线段转空,from=COST_REDUCING,cost_basis=633.33) |
| 491 | 638.3350 | 795 | 648.4500 | +1.58% | EXIT(线段转空,from=COST_REDUCING,cost_basis=638.34) |

## 4. B 组（缠论 + PH settle）：买点前 online settle 过滤

初始资金 100,000 USD，无融资，sub_ratio=0.3。执行价全部为 AV 真实 lag=1 收盘价。
PH settle 否决买点数：**0** / 3 个 3买。

### PH settle 与缠论 3买的共振（独立交叉确认）

PH 近端下跌腿 settle 屏障价（拓扑学，online merge tree）vs 缠论 3买执行价（形态学）。二者识别同一个底则价位应重合。

| 步 | 缠论3买执行价 | PH近端settle价 | gap | settle_pers | 判定 |
|----|------------|--------------|-----|------------|------|
| 242 | 627.43 | 627.42 | +0.180 | 0.01 | ALLOW |
| 415 | 633.33 | 634.37 | +0.843 | 2.84 | ALLOW |
| 491 | 638.34 | 638.41 | +0.012 | 0.01 | ALLOW |

**共振发现**：缠论 3买执行价与 PH 近端 settle 屏障价平均绝对偏差 **0.345** 点（约 0.053% of 价）。二者近乎重合——缠论形态学的「三买」与 PH 拓扑学的「近端下跌腿因果死亡确认」在 5min 尺度上识别同一转折点，构成独立交叉确认（persistence_theory 「settle屏障 = 缠论分型确认」同构的弱经验支持，L2）。

**尺度诚实声明**：本窗口近端下跌腿 settle_persistence 普遍 < 3.0 点（5min 尺度下摆动幅度比日线小一个量级），故 PH gate 在此窗口几乎不主动否决（全部经「噪声」豁免通过）。PH settle gate 的过滤能力需在**日线/更大级别**（settle 屏障为多点级）窗口验证——本 5min 结果不能外推 PH 过滤有效性，只支持「价位重合」这一更弱的共振结论。

### 信号处理日志

| 步 | 事件 | 文本 | 执行价 | FSM 新状态 |
|----|------|------|--------|-----------|
| 127 | SUB_LEVEL_SELL_POINT | 1卖(盘整) 1 | 619.9700 | SKIP(IllegalTransitionError) |
| 162 | SUB_LEVEL_SELL_POINT | 1卖 0 | 620.6627 | SKIP(IllegalTransitionError) |
| 242 | BUY_POINT_CONFIRMED | 3买 -1 | 627.4300 | POSITION_OPEN |
| 267 | SUB_LEVEL_SELL_POINT | 1卖 1 | 628.2900 | COST_REDUCING |
| 318 | MAIN_LEVEL_SELL_POINT | 1卖(趋势) 0 | 628.4371 | SCANNING(reset,线段转空退出) |
| 352 | MAIN_LEVEL_SELL_POINT | 1卖(趋势) 1 | 628.0902 | SKIP(IllegalTransitionError) |
| 415 | BUY_POINT_CONFIRMED | 3买 -2 | 633.3300 | POSITION_OPEN |
| 457 | SUB_LEVEL_SELL_POINT | 1卖 3 | 637.0100 | COST_REDUCING |
| 461 | SUB_LEVEL_SELL_POINT | 2卖 0 | 637.0100 | SKIP(IllegalTransitionError) |
| 487 | MAIN_LEVEL_SELL_POINT | 1卖(趋势) 0 | 638.1500 | SCANNING(reset,线段转空退出) |
| 491 | BUY_POINT_CONFIRMED | 3买 0 | 638.3350 | POSITION_OPEN |
| 500 | SUB_LEVEL_SELL_POINT | 1卖 0 | 639.0693 | COST_REDUCING |
| 514 | SUB_LEVEL_SELL_POINT | 2卖 0 | 639.4400 | SKIP(IllegalTransitionError) |
| 565 | SUB_LEVEL_SELL_POINT | 2卖 1 | 636.9400 | SKIP(IllegalTransitionError) |
| 636 | BUY_POINT_CONFIRMED | 3买 -1 | 639.6000 | SKIP(IllegalTransitionError) |
| 795 | MAIN_LEVEL_SELL_POINT | 1卖(趋势) 0 | 648.4500 | SCANNING(reset,线段转空退出) |
| 860 | SUB_LEVEL_SELL_POINT | 2卖 2 | 648.8450 | SKIP(IllegalTransitionError) |
| 922 | SUB_LEVEL_BUY_POINT | 2买 0 | 645.7800 | SKIP(IllegalTransitionError) |
| 978 | SUB_LEVEL_BUY_POINT | 2买 0 | 645.4225 | SKIP(IllegalTransitionError) |
| 1152 | SUB_LEVEL_SELL_POINT | 2卖 1 | 645.5600 | SKIP(IllegalTransitionError) |
| 1203 | SUB_LEVEL_BUY_POINT | 2买 0 | 646.6500 | SKIP(IllegalTransitionError) |
| 1222 | SUB_LEVEL_SELL_POINT | 1卖(盘整) 0 | 646.7785 | SKIP(IllegalTransitionError) |

### 已完成交易

| 买入步 | 买入价 | 卖出步 | 卖出价 | P&L% | 退出原因 |
|--------|--------|--------|--------|------|---------|
| 242 | 627.4300 | 318 | 628.4371 | +0.16% | EXIT(线段转空,from=COST_REDUCING,cost_basis=627.43) |
| 415 | 633.3300 | 487 | 638.1500 | +0.76% | EXIT(线段转空,from=COST_REDUCING,cost_basis=633.33) |
| 491 | 638.3350 | 795 | 648.4500 | +1.58% | EXIT(线段转空,from=COST_REDUCING,cost_basis=638.34) |

## 5. A vs B 对比

| 指标 | A 组（纯缠论） | B 组（+PH settle） |
|------|--------------|-------------------|
| 已实现 P&L | 2506.16 USD | 2506.16 USD |
| 末端浮盈% | — | — |
| 往返交易数 | 3 | 3 |
| PH 否决买点 | — | 0 |

## 6. 结果包（六要素）

**1. 结论**：QQQ 5分钟 1251 步 replay （2026-04-13 14:59 UTC → 2026-04-21 23:09 UTC，约 8 个交易日）提取 22 个零前视可执行信号。用 AV 真实 lag=1 收盘价执行，A 组已实现 P&L 2506.16 USD，B 组（PH settle 过滤）2506.16 USD。执行价前视偏差平均绝对值 1.3189（0.204% of 价），最大 4.6400——证明 TV 标注价与真实可成交价存在系统性差异，用标注价回测会高估/低估收益。

**2. 定义依据**：零前视三层口径见 §0。信号提取复用 `zero_lookahead_backtest.extract_zero_lookahead_signals`（基于 repaint=0% 验证，见 analysis/tv_repaint_check.md）；线段方向 = 缠论指标 `(趋势)` 标记（§3 分类）；执行价 = AV `TIME_SERIES_INTRADAY` 5min 下一根 bar 收盘（lag=1）。

**3. 边界条件（结论翻转条件）**：
- 时间窗口仅约 8 个交易日的 5min 数据，信号 22 个 / 可执行 22 个——**统计上不可外推**，单一窗口可被另一窗口否证；
- 5min 信号对应**次级别**（非主操作级别），3卖（线段转空）在本窗口为 0，  退出几乎只由 `1卖(趋势)` 触发；
- AV EDT 时区假设（2026-04 全月 EDT/-4）；1 根 04-19 周日 bar AV 缺失，  落到下一根真实 bar；
- PH settle gate 阈值 SETTLE_MIN_PERS=3.0、sub_ratio=0.3 为参数假设（L0）；
- 单标的单时段 = L2；跨标的/跨时段（L3）需全量回放完成后重算。

**4. 下游推论**：
- 执行价前视偏差非零 → 任何用 TV 标注价的历史回测（含本仓库既有   zero_lookahead_backtest.py）都含第 1 层偏差，需用 AV 真实价重算；
- 若 B 组 PH 否决的买点事后被证明确为假买点 → PH settle 作为缠论过滤层有效；  本窗口 PH 否决 0 个，需更长窗口验证过滤质量。

**5. 谱系引用**：
- 267号：满仓满融降成本操作方法论（cost_reduction_fsm）；
- persistence_theory §17.1/§17.3：alive/settled 与规则3（PH settle gate 依据）；
- a_settle_trigger.py：全局最低分量不可反弹 settle（090号声明膨胀禁止）；
- repaint=0% 结论：analysis/tv_repaint_check.md（L2）。
- 本产出未涉及缠论概念定义的新谱系分离；线段方向口径沿用指标既有 `(趋势)` 标记。

**6. 影响声明**：新增 `scripts/zero_lookahead_full_backtest.py`、本报告、`analysis/data_cache/av_qqq_5min_202604.json`、`analysis/data_cache/zero_lookahead_full_result.json`。只读调用 cost_reduction_fsm / a_settle_trigger / a_online_persistence，不修改任何生产模块或缠论定义。相对既有 zero_lookahead_backtest.py 的实质改进：执行价从 TV 标注价改为 AV 真实 lag=1 收盘价（消除第 1 层前视）。