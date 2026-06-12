# PINS（Pinterest）1min 全历史回测 — BH / V0(P5) / V2oa25 / V2oa25_ht

日期：2026-06-11 ｜ 认识论等级：**L2**（单标的真实数据，新资产域）
脚本：`analysis/pins_backtest.py` ｜ 在册 JSON：`analysis/data_cache/pins_backtest.json`

## 1. 结论

| 变体 | 复利 | Δ(BH) | Δ(V2oa25) | 笔数 | 胜率 | 夏普 | MDD | 暴露 | 均持仓bar | trend_holds |
|------|-----:|------:|----------:|-----:|-----:|-----:|----:|-----:|----------:|------------:|
| BH | **−6.92%** | — | — | 1 | — | — | — | 1.00 | — | — |
| V0 (P5) | −21.14% | −14.2pp | — | 142 | 57.7% | −0.000 | −54.3% | 0.91 | 5043 | 0 |
| V2oa25 | −7.36% | −0.4pp | 基线 | 142 | 57.7% | +0.019 | −52.6% | 0.91 | 5043 | 0 |
| **V2oa25_ht** | **+52.32%** | **+59.2pp** | **+59.7pp** | 310 | 58.1% | +0.052 | −67.8% | 0.80 | 2013 | 42 |

三层增量分解：
- **P5 → V2oa25（配对 REV + θ自适应 + 成本门 + 41课门）**：−21.1% → −7.4%，**+13.8pp**。
  PINS 进入 REV 正域（十标的基准正域 6/负域 4 之后的第 7 个正域样本）。
  短差腿 746 → 1884 次，master 出场结构零变化（exit_reasons 逐键相同）——增量全部来自 REV 配对短差。
- **V2oa25 → V2oa25_ht（HoldTrend 出场单轴）**：−7.4% → +52.3%，**+59.7pp**。
  PINS 是 HoldTrend 第五个 Δ 为正的标的（OKLO +176.9 / BRN +495 / CL +25 / BTC +14.5pp 之后），
  且首次在**长熊+泡沫崩塌域**（IPO 破发 → 2021 顶 ~$89 → −90% 崩塌 → 低位震荡）为正。
- 唯一跑赢 BH 的变体是 V2oa25_ht；V2oa25 与 BH 打平（−0.4pp），V0 显著跑输。

### 行为面反转（与前四标的方向相反，开放观察项）

前四标的上 HoldTrend 的表型是"持有更久、笔数更少"；PINS 上相反：
笔数 142 → 310（×2.2），平均持仓 5043 → 2013 bar（腰斩），暴露 0.91 → 0.80（下降）。
出场理由全部带 `_trendexh` 后缀（264 segment + 41 move(L1) + 4 recL2），拦截计数 trend_holds=42。
机制未诊断——仅记录观测：HoldTrend 在长熊域改写的是 FLAT 窗口时序（出场延后 →
后续入场窗口重切分），而非单纯延长持仓。**不在此声明因果机制**（090号：声明=能力）。

代价面：MDD −52.6% → −67.8%（加深 15.2pp）——持仓穿越崩塌段是 HoldTrend 的固有代价，
与 BTC 案（MDD 换趋势主体段利润）同构。

## 2. 数据与守卫

- **数据**：Databento `XNAS.ITCH` ohlcv-1m，`PINS` 2019-04-18（IPO 日）→ 2026-06-09，
  volume>0 过滤后 **784,963 bars**（`pins_1m_databento_full.json`，49.6MB）。
  PINS 无拆股 ⇒ 原始未复权价直接作回测基底（Mag7 复权陷阱不适用）。
  Databento 标注 3 个 degraded 日（2021-07-07 / 2021-10-26 / 2022-09-19），未剔除。
- **磁带指纹**：bsp 22,395 / div 16,561 / flips 41,246（增量续跑守卫在册）。
- **首笔入场同一性**：三变体 first_entry_bar = 1032 逐位相同 ⇒ exit_mode/θ 轴对入场触发零泄漏。
- **链路**：`load_ohlc → compute_organic_signals →（5.6s 信号层）→ pack_tape → nr.run_organic_rust`，
  与 master 出场消融任务同一链路同一 floor（LADDER_SEG=2）。
- **口径**：与十标的基准同口径——master 腿无逐笔摩擦扣减，REV 腿成本门
  （θ_eff ≥ 2×10bps）仅作开腿门槛。复利数字与实盘净值之间隔一层执行摩擦。

## 3. 定义依据

- **V2oa25**：`rust/src/trading/config.rs` 在册定义——`ThetaMode::AdaptiveQuantile{q:0.25,window:50,min_obs:10}`
  + `rev_l41_gate=true` + 成本门下界，继承 V2of（配对 REV）。满仓入场 = 有机赋格 master 默认。
- **V2oa25_ht**：`exit_mode=ExitMode::HoldTrend` 单轴（49课"利润最大化"持币不动 +
  41课"大级别走势没有衰竭时不做反向"——sell1 ∧ 父级别上行趋势衰竭才出）。
- **V0≡P5**：cargo 守卫在册（trades+trace+counters 三面逐位）。

## 4. 边界条件（结论何时翻转）

1. **单标的 L2**——PINS 一例不升级"HoldTrend 全资产为正"为 L3 结论；若后续标的出现
   Δ(base) 为负（尤其慢熊阴跌域，趋势衰竭信号迟到 → 深套），5/5 全正即破。
2. **MDD 约束**：若以 MDD ≤ −60% 为风控硬约束，V2oa25_ht 在 PINS 上不可采纳（−67.8%）。
3. **摩擦敏感**：_ht 笔数 ×2.2 + 短差 1429 次，若计入逐笔摩擦（股票 ~1-2bps + 滑点），
   +59.7pp 增量会被侵蚀；REV 腿成本门只防负期望开腿，不消除已开腿摩擦。
4. **degraded 日**：3 个降质日若含异常 bar，磁带指纹随数据重拉漂移（OKLO E 基线漂移先例）。

## 5. 下游推论

- HoldTrend 出场 5/5 全正且 regime 覆盖扩至长熊崩塌域 → "新默认候选"证据增强一档，
  距 L3 判决还差负样本搜索（慢熊阴跌标的，如 2021 后中概）。
- PINS 入 REV 正域 → REV 标的白名单（十标的基准结论：REV 应为白名单非全局开关）新增一行。
- 行为面反转（笔数翻倍）提示 HoldTrend 的作用机制是 regime 依赖的双通道
  （强趋势域=延长持仓；震荡/熊域=重切 FLAT 窗口）——值得单独诊断任务，本报告不预判。

## 6. 谱系引用

- master 出场状态驱动消融（HoldTrend 四标的全正，状态范畴第四例；HighestOnly 伪装 buy-hold 否证）
- master 出场爬梯否证（出场判据级别不可高于入场级别——HoldTrend 维持默认候选的对照臂）
- θ 自适应深度门三标的判决 + REV 腿配对修正（V2oa25 的两个构成轴）
- V2oa25 十标的全量基准（REV 正/负域两极分化 → 白名单结论）
- 231号形式化有效域规则（本报告 L2 标注）；090号严格性（机制未诊断处不声明因果）

## 7. 影响声明

新增：`analysis/fetch_pins_databento.py` 产出的 `pins_1m_databento_full.json`（数据）、
`analysis/pins_backtest.py`（脚本）、`analysis/data_cache/pins_backtest.json`（在册结果）、
本报告。零改动：引擎、信号层、`config.rs` 变体表、任何在册回测 JSON。
