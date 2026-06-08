# 完整版缠论+PH多重赋格操盘系统 — G/H 回测

> 标的：QQQ / OKLO（1min）  |  脚本：`analysis/fugue_complete_v2.py`  |  认识论等级：**L2**（真实数据，含否定性结果）

> 521 号限定：信号属 candidate 层（PH 门控 + MACD 面积力度代理），非 confirmed 买卖点 alpha 证据。E/F 简化实验见 `fugue_alpha_diagnosis.md`。

## 复现校验（磁带超集忠实性）

**第一层 tape-equality（严格证明）**：BarSignalV2 是 `BarSignal` 的严格超集。本脚本在每标的前 30,000 bars 上**逐字段**对比 `compute_signals_v2` 与原始 `fugue_alpha_diagnosis.compute_signals` 的 18 个共享字段——不依赖任何已发布数字，直接证明 V2 磁带 ≡ 原始引擎输出。

| 标的 | 校验 bars | 共享字段 | 不一致 bar | 结论 |
|------|----------|---------|-----------|------|
| QQQ | 30,000 | 18 | 0 | ✅ 逐字段一致 |
| OKLO | 30,000 | 18 | 0 | ✅ 逐字段一致 |

**tape-equality 结论**：✅ V2 磁带逐字段等于原始引擎输出——A/E/F 由复用的 `run_trading`/`run_swing_trading` 在此磁带上重放，故与原始脚本数字同源，G/H 与 A/E/F 直接可比。

**第二层 已发布外部锚点（A/E）**：A/E 与 `fugue_alpha_diagnosis.md` 已发布数字对比。

| 标的 | 模式 | 本脚本% | 已发布% | 偏差 |
|------|------|--------|---------|------|
| QQQ | A_fsm_none | +40.59 | +40.59 | +0.0025 |
| QQQ | E_swing_none | +71.16 | +71.16 | -0.0036 |
| OKLO | A_fsm_none | +242.61 | +242.61 | -0.0031 |
| OKLO | E_swing_none | +455.81 | +455.81 | +0.0010 |

**外部锚点结论**：✅ A/E 与已发布逐位一致（进出场磁带忠实）。注：`.md` 的 F（QQQ 50.28 / OKLO 495.63）**已陈旧**——其后 `fugue_alpha_diagnosis.py` 被修改，当前脚本 F 与本脚本一致（已由 tape-equality 逐字段证明覆盖），故 F 不以陈旧 `.md` 为锚（formalization-validity-domain：陈旧锚点会产生假阳性漂移）。

## QQQ

- 数据：**728,030** bars  |  价格：268.82 → 738.28  |  Buy-and-hold：**+174.64%**

| 版本 | 复利% | BH% | 超额% | 交易数 | 胜率 | 夏普 | 降成本笔 |
|------|-------|-----|--------|--------|------|------|---------|
| A（38课FSM纯进出场） | +40.59 | +174.64 | -134.04 | 17 | 65% | +0.395 | 0 |
| E（背驰定位器简化版） | +71.16 | +174.64 | -103.48 | 11 | 73% | +0.400 | 0 |
| F（E+简化降成本） | +63.39 | +174.64 | -111.25 | 11 | 73% | +0.376 | 9 |
| G（完整版：区间套+中枢趋势门+persistence退出+2买标记） | +86.54 | +174.64 | -88.10 | 39 | 62% | +0.315 | 0 |
| H（完整版+三阶段FSM降成本+挣股数） | +66.74 | +174.64 | -107.90 | 39 | 62% | +0.297 | 38 |
| I（最佳组合：E的L2趋势翻转出场+H进场/持仓） | +59.85 | +174.64 | -114.79 | 11 | 64% | +0.401 | 11 |

- 2买加仓机会（G/H 标记，加仓已移除）：G 88 次，H 88 次。
- H 进入挣股数阶段（cost_basis≤0）的交易：0 笔。

## OKLO

- 数据：**333,613** bars  |  价格：18.07 → 63.48  |  Buy-and-hold：**+251.30%**

| 版本 | 复利% | BH% | 超额% | 交易数 | 胜率 | 夏普 | 降成本笔 |
|------|-------|-----|--------|--------|------|------|---------|
| A（38课FSM纯进出场） | +242.61 | +251.30 | -8.69 | 6 | 100% | +0.971 | 0 |
| E（背驰定位器简化版） | +455.81 | +251.30 | +204.51 | 6 | 50% | +0.438 | 0 |
| F（E+简化降成本） | +413.32 | +251.30 | +162.02 | 6 | 50% | +0.434 | 6 |
| G（完整版：区间套+中枢趋势门+persistence退出+2买标记） | +267.55 | +251.30 | +16.25 | 28 | 50% | +0.248 | 0 |
| H（完整版+三阶段FSM降成本+挣股数） | +238.04 | +251.30 | -13.26 | 28 | 46% | +0.244 | 26 |
| I（最佳组合：E的L2趋势翻转出场+H进场/持仓） | +416.62 | +251.30 | +165.32 | 6 | 50% | +0.444 | 6 |

- 2买加仓机会（G/H 标记，加仓已移除）：G 16 次，H 16 次。
- H 进入挣股数阶段（cost_basis≤0）的交易：0 笔。

## 完整版判定（I/G/H vs E/F vs BH）

### QQQ

- G vs E（完整化增量）：+86.54% vs +71.16% → **完整化提升**（差 +15.38%）
- G vs BH：+86.54% vs +174.64% → **跑输 BH**
- H vs G（三阶段降成本+挣股数净贡献）：+66.74% vs +86.54% → **降成本拖累**（差 -19.80%）
- H vs F（完整 FSM 降成本 vs 简化降成本）：+66.74% vs +63.39%（差 +3.35%）
- **I vs G（出场升级 E 的 L2 趋势翻转）**：+59.85% vs +86.54% → **出场升级未提升**（差 -26.69%）
- **I vs E（加 G/H 完整进场持仓）**：+59.85% vs +71.16% → **完整化未提升**（差 -11.31%）
- **I vs H（同进场持仓，仅出场 E vs L1+persistence）**：+59.85% vs +66.74%（差 -6.89%）
- **QQQ 四版本最优**：G（+86.54%），I=+59.85% 非最优；I vs BH：跑输 BH

### OKLO

- G vs E（完整化增量）：+267.55% vs +455.81% → **完整化未提升**（差 -188.26%）
- G vs BH：+267.55% vs +251.30% → **翻正超额**
- H vs G（三阶段降成本+挣股数净贡献）：+238.04% vs +267.55% → **降成本拖累**（差 -29.51%）
- H vs F（完整 FSM 降成本 vs 简化降成本）：+238.04% vs +413.32%（差 -175.28%）
- **I vs G（出场升级 E 的 L2 趋势翻转）**：+416.62% vs +267.55% → **出场升级提升**（差 +149.08%）
- **I vs E（加 G/H 完整进场持仓）**：+416.62% vs +455.81% → **完整化未提升**（差 -39.19%）
- **I vs H（同进场持仓，仅出场 E vs L1+persistence）**：+416.62% vs +238.04%（差 +178.58%）
- **OKLO 四版本最优**：E（+455.81%），I=+416.62% 非最优；I vs BH：翻正超额 ✅

**跨标的**：G 在 **部分标的** 上相对 E 提升（完整化有增量）；G 在 **部分标的** 上翻正超额（G>BH）。H−G 净贡献：QQQ -19.80%、OKLO -29.51%。

## 补全的 7 模块（设计文档 → 代码映射）

| 模块 | 设计要求 | 实现 |
|------|---------|------|
| 1. 4层级别架构 | L2趋势/L1操作/L0次操作 | L2=move-level PH(l2_flip)；L1=move settle+背驰(进出场)；L0=segment PH+背驰(降成本)。L2 为段切换语境非1买门 |
| 2. 区间套精确入场 | L1确认→L0定位 | ARMED 两阶段：L1趋势底背驰 ARM → L0(l0_r1_down/type1) 定位精确价；递归展开非独立过滤 |
| 3. 2买 | 回调不破1买低点 | **结构化判定**(引擎不发type2,只type1/type3)：回调段低点≥1买低点 ∧ 买点候选(回调底确认) → 标记(加仓已移除,267号D2无加仓FSM,只计数) |
| 4. Persistence过滤 | 高persistence触发退出 | up-move settle persistence≥近期中位数 → 退出评估；低persistence settle 不退出(穿越噪声,H中触发降成本) |
| 5. 中枢判定 | 1买要求趋势下跌(≥2中枢) | down-move zs_count≥2 门控入场；**经验观察(L2)**：本数据全部 settle 的 down-move 均为趋势(zs_count≥2)，盘整(zs_count==1)不作方向性settle出现 → 门语义正确但此数据上非限制性(诚实标注,非声明膨胀) |
| 6. 三阶段降成本FSM | 接入cost_reduction_fsm | H 用真实 CostReductionFSM：POSITION_OPEN→COST_REDUCING→PRINCIPAL_WITHDRAWN/EARNING_SHARES |
| 7. 次级别买卖点触发降成本 | 顶背驰卖/底背驰买 | sub_sell_signal(segment顶背驰)→SUB_LEVEL_SELL_POINT；sub_buy_signal(segment底背驰/买点)→SUB_LEVEL_BUY_POINT |

## 结果包（六要素）

**1. 结论**：见主对照表（BH/A/E/F/G/H）。G=完整版进出场（区间套+中枢趋势门+persistence退出+2买标记），H=G+三阶段FSM降成本+挣股数。

**2. 定义依据**：38课操盘程式（底背驰买/顶背驰卖/没背驰不动）；37课背驰（相邻同向走势力度衰减）；29课区间套（大级别往小级别看，敏感进场+鲁棒出场）；中枢趋势定义（`a_move_v1.py`：趋势=2+同向中枢，盘整=1中枢）；267号满仓满融降成本（`cost_reduction_fsm.py`）；缠师31/43课挣股数（cost_basis≤0后金额守恒先卖后买）。

**3. 边界条件**：(i) 入场要求 down-move zs_count≥2 — 趋势背驰买点在1min上稀少，若引擎中枢判定阈值变化则入场频率翻转；(ii) persistence 高/低以近期中位数(窗口50)分类，窗口变化影响退出频率；(iii) sub_ratio=0.3、trim阈值0.05 隐含；(iv) 力度用MACD面积代理(521号PH纯拓扑无动量)，换真实走势区间力度可能翻转；(v) G/H>BH 仍须随机门控对照排除趋近buy-hold同义反复(project_backtest_benchmark_falsifiability)。

**4. 下游推论**：若 G≤BH → 完整化的进出场信号层仍无确立 alpha，须先修信号层再谈降成本；若 G>E → 区间套+中枢趋势门+persistence退出确有完整化增量；若 H>G → 三阶段FSM降成本在完整框架下增益(可上推confirmed层)，若 H<G → 降成本仍是负alpha源(project_ph_settle_usage_boundary)。

**5. 谱系引用**：521号(PH须MACD闸)；267号(满仓满融降成本+D2无加仓)；268a(own_capital独立核算)；project_divergence_locator_entry_exit(E/F背驰定位器,G/H的前身)；project_divergence_gate_entry_exit_falsified(D门控被证伪,故L2不作1买门)；project_costreduction_moneyprinter_bug(截断bug已去)；project_ph_settle_usage_boundary(PH settle是candidate必要非充分)；project_backtest_benchmark_falsifiability(BH基准可证伪性)。

**6. 影响声明**：脚本 `analysis/fugue_complete_v2.py`(本文件)——compute_signals_v2(BarSignalV2超集磁带)+run_complete(G)+run_complete_h(H,接入cost_reduction_fsm.py)+run_complete_i(I,E出场+H进场持仓)。I 与 H 唯一差异是出场判定行(L2趋势翻转替换L1+persistence)，进场/FSM降成本/2买逐行复用 H 逻辑。**不修改引擎代码、不修改 cost_reduction_fsm.py、不修改 fugue_alpha_diagnosis.py**(A/E/F 经复现校验逐位复用)。新增报告 `fugue_complete_v2_results.md`。

**认识论等级**：L2（真实数据 QQQ/OKLO 1min；G 若跑输 BH 的否定性结果缩小有效域边界，比确认性结果更有价值——缺 L3 多标的交叉验证与随机门控对照）。