# #917 市场冲击的可测代理：平方根律文献正本 + 只用 OHLCV 能标定到什么程度

- **票**：[#917](https://github.com/xy7365527-lang/NewChanlun/issues/917)（`wayfinder:research`，父 map [#787](https://github.com/xy7365527-lang/NewChanlun/issues/787)），喂 [#914](https://github.com/xy7365527-lang/NewChanlun/issues/914) Q4 与 [#915](https://github.com/xy7365527-lang/NewChanlun/issues/915)。
- **日期**：2026-08-05。**基线**：`a301a7fb03`（开工时 worktree HEAD 落在祖传线 `19b4015927`，已 `git reset --hard main` 归正后才取数与核码）。
- **纪律**：一手源优先（论文引发表版/arXiv，代码引仓库本体）；每条断言挂年份+数据集+市场+样本量；穷举断言附检索式（§4）。
- **本件性质**：文献考据 + 一节我自己的推导。**推导那节明确标注是我推的，不是文献结论。**

---

## 0. 执行摘要（大白话）

**能拿到。公式有正本，系数有出处，而且比特币有专门的一手标定。**

一句话的答案：

```
冲击(Q) = Y · σ_日 · sqrt( Q / V_日 )
```

- **Q** = 一整笔要做完的单子（分批打出去的那种，文献叫 metaorder），不是单笔成交；
- **σ_日** = 日波动率，**V_日** = 日成交量，两个都是**日**的量，跟你花多久打完这单**没关系**；
- **Y** ≈ **0.9**，比特币的专门标定（Donier & Bonart 2015，MtGox 全量 1300 万笔成交、重建出 100 万个 metaorder）。这个数跟股票、期货标出来的值（跨市场带 0.5–1.0）**基本一样**；
- **指数 0.5** 是主流，但不是铁板一块：Almgren 等 2005 在美股上**明确否掉了 0.5**、给 0.6；东京交易所 2025 年的全市场普查（约 10⁸ 个 metaorder）又强力支持 0.5。**我建议按 0.5 用，把 0.4–0.7 当敏感性区间。**

**只用 1 分钟 OHLCV 能走多远？——能算，不能自己标。**

- σ_日 和 V_日 **从 1 分钟 OHLCV 直接算得出来**，我已经在本仓 BTC 全史 4,613,599 根 bar 上算了（§2.4）：Garman-Klass 中位日 σ_日 = **2.53%**，日成交额中位 **$1.23B**。而且 2026 年那篇 AAPL 的标定论文用的就是 **Parkinson 高低价估计量**当 σ_D —— **拿 OHLC 估波动率去做冲击标定，是文献自己在干的事，不是我们的将就。**
- **算不出来的是 Y 本身。** 要自己标 Y，必须有**带买卖方向的逐笔成交**（去重建 metaorder）。本仓三条数据通道全是聚合 Bar（票面已核实），**做不到**。⟹ **Y 只能借文献的 0.9，不能自证。**
- ⟹ **结论：算容量能做，标系数做不到。** 差的那一步是「把匿名成交流切成 metaorder」，那需要逐笔+方向，OHLCV 里没有。

**三条对本仓当场可用的具体结论：**

1. **`src/newchan/cost/slippage.py:119` 的 `impact_coeff = 0.1` 偏大 3–6 倍。** 那行代码的式子是 `c·sqrt(Q/ADV)`，**没有 σ** ⟹ 它的 `c` 在数学上就等于 `Y·σ_日`。用 BTC 实测 σ_日 = 2.53% 和文献 Y = 0.9 算，`c` 应该是 **0.023**，不是 0.1（§2.6）。
2. **同一文件 `:118` 的 `threshold_pct = 0.01` 把冲击项在我们最关心的尺寸区间整个关掉了。** 按 §3 推出的容量，L0/L1/L2 分别是日成交量的 0.09% / 0.26% / 0.79%，**全在 1% 门槛以下** ⟹ 冲击恒为 0。而平方根律的核心主张恰恰是**小单的冲击异常地大**（Tóth 等 2011 原文：边际冲击按 Q^{-1/2} 发散）。**这个死区把要实现的定律实现反了。**
3. **不要混用 Y 和 Almgren 的 η。** 两者是不同形状（尺寸 vs 参与率），在同一个 Q/V 上数值差 **3–11 倍**（§2.3 表）。

**我推出来的那个数（§3，标注为我的推导，非文献）**：容量定义成「冲击不超过该级往返波幅的 x」时，
`w(1) = 容量(ℓ-1)/容量(ℓ) = (A_{ℓ-1}/A_ℓ)^{1/δ}` —— **Y、σ、V、x 全部约掉**。用 #907 的真实波幅，δ=0.5：
**w(1) ≈ 0.26**（每往下一级，容量掉到约 1/3.8）。但**它不是常数**（逐级 0.35 / 0.33 / 0.16），且随 δ 与分位数选择在 **[0.13, 0.55]** 之间摆。

---

## 1. 承重的判别（先把最容易搞混的四件事钉死）

### 1.1 metaorder ≠ 单笔订单 —— 承重，而且是第一位承重

**Tóth 等 2011 §2 开篇逐字**（[arXiv:1105.1694](https://arxiv.org/abs/1105.1694) p.2）：

> "One should first carefully distinguish the total impact of a given metaorder of size Q from other measures of impact that have been reported in the literature."

它点名了**三个互不相同的量**，并说「许多作者不当地把两者等同（many authors unduly identify the two quantities）」：

| 量 | 定义 | 形状 |
|---|---|---|
| **metaorder 总冲击 Δ(Q)** | 一个交易决策拆成多个子单打完，首笔到末笔的价格变化 | **Q^0.5**（本文要用的） |
| 单笔市价单即时冲击 | 单个 size q 的市价单 | q^α，**α ≈ 0.2**，甚至 ln q |
| 区间不平衡冲击 Δ_T vs 𝒬_T | 时间窗 T 内全市场有符号成交量之和 | T 变大趋于**线性** |

脚注 1 逐字：「metaorder（或 parent order）＝ 对应单个交易决策的一束订单；metaorder 通常通过若干 child order 增量成交。」

**⟹ 我们要用它定义「一个级别能容纳多少资金」，这个映射对不对？**

**对，但要加两句限定，否则会用错：**

1. **「容量」的对象是一条腿，不是一趟往返。** 建仓是一个 metaorder，平仓是另一个 metaorder。平方根律约束的是**每条腿**。往返总冲击 ≈ 建仓冲击 + 平仓冲击（Donier & Bonart 2015 §3.1 给出 metaorder 结束后冲击回落约 1/3，永久部分 I^∞ ≈ (2/3)·I_peak）。**按一条腿算还是按两条腿算，绝对容量差 4 倍（δ=0.5 时 2^{1/δ}=4），但 §3 的比值 w 不受影响（该因子逐级相同、约掉）。**
2. **持有期不进这个公式。** 平方根律里没有持仓时间这一维。L4 一趟往返中位持有 486 天（#907），这 486 天的 funding/borrow **不是冲击成本**，得另算。**不要把它塞进 Y。**

**一条反向的重要限定（对我们有利）**：在比特币上这个区分**没有股票上那么锋利**。Donier & Bonart 2015 Table I：他们那 100 万个 metaorder 里 **61% 只含 1 笔 child trade**，2–4 笔占 29%，≥10 笔只占 3.5%；而平方根律**从最小尺度起、跨 4 个数量级都成立**（同文 §4.1）。⟹ 在 BTC 上「一次性打出去的单」和「切碎打的单」服从同一条律，我们不必先把订单切碎才能引用它。

### 1.2 σ 与 V 的窗口 —— 三篇一手源**全部用「日」**；推广到任意窗口有形式上的自洽，但**没有一手标定**

**三篇原文逐字**：

| 来源 | 原文措辞 | σ 是什么 | V 是什么 |
|---|---|---|---|
| Tóth 等 2011 Eq.(1) | "σ is the daily volatility of the asset, and V the daily traded volume, **both quantities measured contemporaneously to the trade**" | 日波动率 | 日成交量 |
| Donier & Bonart 2015 Eq.(1) | "V_D the daily traded volume and σ the daily volatility of the stock" | 日 | 日 |
| Vasaikar 2026 §3 | "We non-dimensionalise by daily volatility σ_D (**Parkinson high–low of the mid**) and volume V_D" | 日（Parkinson OHLC 估计量） | 日 |

Almgren 等 2005 是**例外且不同**：V = **10 日移动平均**日成交量（原文 §2.2 逐字「V is a ten-day moving average」），σ = 日波动率但用「an intraday estimator that makes use of every transaction in the day」。

**⟹ 能不能按缠论级别（每级时间尺度差 5–6 倍）逐级换窗口用？**

**先说数学**：若把 σ 与 V 都换成窗口 T（以日为单位）的量，且假定
- 价格扩散：`σ_T = σ_日 · sqrt(T)`（Tóth 等 2011 §3 明确以「prices are approximately diffusive」为立论前提之一）
- 成交量线性累积：`V_T = V_日 · T`

则
```
Y · σ_T · sqrt(Q/V_T) = Y · σ_日·sqrt(T) · sqrt( Q / (V_日·T) ) = Y · σ_日 · sqrt(Q/V_日)
```
**——完全相同，T 消掉了。** 这条公式在「扩散 σ + 线性 V」的标度下是**窗口不变的**。

**这条不变性和经验上的「冲击与执行时长无关」是同一件事的两面**，后者是平方根律的标准陈述（Tóth 等 2011 摘要与 §2 通篇；Donier & Bonart 2015 §3.1 逐字：「the usual rule of thumb (1) which predicts impact to be independent of the execution speed」）。

**但必须写清三件事，否则这条会被用过头：**

1. **没有任何一手源在非日窗口上标定过 Y。** 三篇全在日窗口上标。上面的不变性说的是「换窗口不会改变公式的值」，**不是**「Y 在别的窗口上被验过」。这是**一致性论证，不是实证背书**。
2. **σ_T ∝ sqrt(T) 只是近似。** 波动率聚簇与均值回复会破坏它。本仓 BTC 上我没测这条标度（§4 记为未做）。
3. **T-无关这一条恰恰是平方根律最受攻击的部分，而且反例就出在比特币上。** Donier & Bonart 2015 §5.2 用他们自己的 BTC 数据拟出
   `I^exec(Q, μ_V) ~ Q^δ / μ_V^{δ'}`，**δ ≈ 0.5，δ' ≈ 0.4**，并逐字写「The slower the execution, the larger the measured impact: this strange dependence on μ_V ... at variance with intuition and previous findings」。同文 Fig.11 下排（孤立 metaorder）则显示「impact that decreases when execution time increases」。**两个方向的偏离都在，都不是零。**

**⟹ 结论：逐级套用是可以的，但依据是「公式在扩散标度下自洽 + 文献报告 T-无关」，不是「有人在那些窗口上标过」。δ' ≈ 0.4 那条偏离是已知的、量级不小的风险。**

### 1.3 Y 的量纲与参与率形式 —— **两种形式不等价，Y 与 η 绝对不能互换**

- **尺寸形式**（Tóth / Donier-Bonart / Sato-Kanazawa / Vasaikar）：`Δ = Y · σ · (Q/V_日)^δ`。Y **无量纲**，因为 Δ 与 σ 同量纲（都是相对价格变化），Q/V 无量纲。
- **参与率形式**（Almgren 等 2005 Eq.(8)）：`(1/σ)(J − I/2) = η · sgn(X) · |X/(V T)|^β`。这里 **V·T ＝ 执行期间市场成交量**，`X/(VT)` 是**参与率**（不是尺寸）。

**两者只在 T = 1 日时形状重合。T < 1 日时参与率形式给出的冲击大 (1/T)^β 倍。**

**数值上差多少（我按两篇的拟合参数算的，Θ/V = 263 取 Almgren Table 3 的 IBM 值，单位是 σ_日）**：

| Q/V | T | Almgren J（含 I/2） | Tóth Y=1 | 相差 |
|---|---|---|---|---|
| 0.09% | 0.32 日（Almgren 样本中位） | 0.00475 σ | 0.0300 σ | **6.3×** |
| 0.09% | 1 日 | 0.00268 σ | 0.0300 σ | **11.2×** |
| 1% | 0.32 日 | 0.02407 σ | 0.1000 σ | **4.2×** |
| 1% | 1 日 | 0.01528 σ | 0.1000 σ | **6.5×** |
| 5% | 1 日 | 0.05515 σ | 0.2236 σ | **4.1×** |

**再加一层：两者测的还不是同一个价格点。** Tóth Eq.(1) 前文说的是「first and last trade 之间的平均相对价格变化」＝ **峰值冲击**；Almgren 的 J ＝ `(S̄ − S_0)/S_0` ＝ **成交均价相对开仓前价** ＝ 执行落差。平方根路径下执行落差 ≈ (2/3)×峰值（Donier & Bonart 2015 §3.1）⟹ **还要再乘 ~1.5 才可比**。

**⟹ 硬结论：`Y = 0.9` 与 `η = 0.142` 是两个坐标系里的数，混用会错 3–11 倍。要用哪个，先确定你算的是峰值还是落差、约束的是尺寸还是速率。本仓要的是「一个级别能装多少钱」＝ 尺寸约束 ⟹ 用尺寸形式、用 Y。**

### 1.4 加密 vs 股票 —— **文献自己说可以借，而且是拿 BTC 亲自验的**

Donier & Bonart 2015 §4.1 逐字：

> "Normalizing by Bitcoin average volatility and daily volume gives a Y-ratio (as defined in Eq. 1) of **Y ≈ 0.9**, close to the value reported on 'mature' financial markets, e.g. futures or stocks."

同文 §1 更强：

> "The precise mechanism which is behind the peculiar square-root law appears to be universal across markets which significantly differ with respect to their trade characteristics (latency, daily traded volume, volatility), microstructural parameters (tick or lot sizes) and fee structure (i.e. the high fees on the Bitcoin)."

**这不是类比，是当年在 BTC/USD 上直接标出来的。** ⟹ **系数可以跨市场借，且 BTC 有专属标定值，不必借股票的。**

**但有一条反面证据必须并列**（§2.5 详）：一份 2025 年 Binance 现货数据的开源复现得出 δ ≈ 0.1，**结论是平方根律在 2025 年的加密现货上不成立**。它不是同行评议文献，且用的是重建 metaorder（无真实 trader ID），但它用的正是本仓同源的数据（`data.binance.vision` BTCUSDT）⟹ **不能当噪音略过。**

---

## 2. 逐条回答票面七问

### Q1. 平方根律的一手来源与确切陈述

**结论：正本是 Tóth 等 2011 的 Eq.(1)，σ 和 V 都是日的量，Q 是 metaorder 不是单笔。**

**出处**：B. Tóth, Y. Lempérière, C. Deremble, J. de Lataillade, J. Kockelkoren, J.-P. Bouchaud, *Anomalous price impact and the critical nature of liquidity in financial markets*, **Phys. Rev. X 1, 021006 (2011)**（2011-10-31 发表；[arXiv:1105.1694](https://arxiv.org/abs/1105.1694)）。全体作者时任 Capital Fund Management。

**原文 Eq.(1) 逐字**（p.2）：

> "the average relative price change Δ between the first and the last trade of a metaorder of size Q is well described by the so-called 'square-root' law:
> **Δ(Q) = Y σ sqrt(Q/V)**, (1)
> where σ is the daily volatility of the asset, and V the daily traded volume, both quantities measured contemporaneously to the trade. **The numerical constant Y is of order unity.**"

同页紧接着给出推广形式与指数范围：

> "Published and unpublished data suggest slightly different versions of this law; in particular the sqrt(Q) dependence are more generally described as a power-law relation Δ(Q) ∝ Q^δ, with **δ in the range 0.4 to 0.7**."

**符号定义汇总（三篇一手源一致）**：

| 符号 | 定义 | 谁定的 |
|---|---|---|
| Q | metaorder 总量（一个交易决策的全部成交量，分多个 child order 打出） | Tóth 2011 脚注 1 |
| V | **日**成交量，与该笔交易同期测量 | Tóth 2011 Eq.(1) 后文 |
| σ | **日**波动率，与该笔交易同期测量 | 同上 |
| Δ | 首笔到末笔的平均相对价格变化（＝峰值冲击） | 同上 |
| Y | 无量纲常数，量级为 1 | 同上 |

**一个有用的量级校准**（Tóth 2011 p.3 逐字）：「trading one hundredth of the daily volume moves the price by a tenth of its daily volatility」⟹ 代入 Q/V = 0.01、Δ = 0.1σ ⟹ **Y = 1**。

**Tóth 2011 自己的数据**（Fig.1）：CFM 自营，**期货，近 500,000 笔**，**2007-06 → 2010-12**。冲击定义为 metaorder 的平均执行落差。**小 tick 合约 δ = 0.5，大 tick 合约 δ = 0.6**，Q/V 覆盖 10⁻⁵ 到几个 10⁻³。扣掉了 Q=0 处的截距 Δ/σ = 0.0015。

### Q2. 系数 Y 的标定值

**结论：BTC 有专属标定 Y₀ = 0.9 ± 0.35（一手，1M metaorder）；跨市场带是 0.5–1.0。**

| 来源 | 市场 | 期间 | 样本量 | Y | δ |
|---|---|---|---|---|---|
| **Donier & Bonart 2015**（**BTC，最直接**） | MtGox BTC/USD 现货 | 2011-08 → 2013-11 | **全部 13M 笔成交**，重建 **>1M metaorder**（14M+ 笔归组） | **Y₀ = 0.90，Σ_Y = 0.35**，分布近高斯 N(0.9, 0.35)，「essentially lies in the interval [0,2]」 | **≈ 0.5**，跨 4 个数量级 |
| Tóth 等 2011 | 期货（CFM 自营） | 2007-06 → 2010-12 | ~500,000 笔 | "of order unity"；由 1%↔0.1σ 校准 ⟹ ≈ 1（**未给不确定度**） | 0.5（小 tick）/ 0.6（大 tick） |
| Vasaikar 2026（**预印本，非同行评议**） | AAPL，Nasdaq TotalView-ITCH MBO | 2024-12-02 → 2025-08-19（178 交易日，~5×10⁸ 事件） | 标定窗 **1,012 个重建 metaorder** | **c_raw = 0.69，95% CI [0.64, 0.75]**；偏差订正后 **c_eff = 0.34**；32 次周度滚动重标定 c_raw ∈ [0.63, 1.01] | 固定 1/2；自由拟合 δ̂ = 0.50，bootstrap 95% CI **[0.32, 0.66]** |
| Sato & Kanazawa 2025 | 东京证交所全市场 | 8 年 | **~10⁸ metaorder**，覆盖全部交易账户 | **未报 Y**（该文的题目是指数普适性） | **δ = 1/2**，统计误差内，逐股逐交易者均成立 |
| Almgren 等 2005 | 美股（Citigroup 自营台） | 2001-12 → 2003-06（19 个月） | 682,562 单原始，**过滤后 29,509 单** | **不可比**（η = 0.142 ± 0.0062 是参与率形式，见 §1.3） | **β = 0.600 ± 0.038**，**明确否掉 0.5** |

**Donier & Bonart 的 Y 是怎么标的**（§4.3 原文）：对每个 metaorder i 算 `Ỹ_i = I(Q_i)/sqrt(|Q_i|) × sign(Q_i)`；按日做成交量加权平均得日均 Ỹ；再除以当日 `σ_D/sqrt(V_D)` 得 Y-ratio。原始 Ỹ ≈ **4.5×10⁻²**（有量纲，BTC 单位），归一后 Y₀ = 0.9。他们同时指出这个归一「accounts for the major part of the non-stationariness, particularly during extreme market events（如 2013-04-10 崩盘）」。

**加密货币独立标定 ——「已查到的有 2 处，加 1 处未核实」**（检索式见 §4）：
1. **Donier & Bonart 2015**（MtGox BTC/USD，Y = 0.9）—— 一手，已逐页读过。
2. **SLMolenaar/crypto-market-impact**（GitHub，MIT）—— Binance BTC/USDT + ETH/USDT 现货，2025-08/09 与 2025-11，**结论相反**（δ ≈ 0.1，见 Q3）。非同行评议。
3. **Barone & Lillo 2026**，*Trading in the Sunshine or in the Shade*（[arXiv:2606.15715](https://arxiv.org/abs/2606.15715)，2026-06-14），Hyperliquid 永续，430 万隐性 metaorder + 46.5 万可见 TWAP。**摘要未报 δ 与 Y，我未打开正文核实** ⟹ 只登记为「存在、待核」，**不作为证据引用**。

### Q3. 适用边界与反面证据

**结论：指数不是铁板一块（一手证据同时存在 0.5 与 0.6 且互相否定）；边界在两头都有硬墙；反面证据存在且其中一条正落在我们的市场上。**

**(a) 指数到底是不是 0.5 —— 一手源之间有真实冲突**

- **反对 0.5**：Almgren 等 2005 §4.2 逐字：「**At the 95% confidence level, the square-root model β = 1/2 is rejected.** We will therefore fix on the temporary cost exponent β = 3/5.」拟合值 **β = 0.600 ± 0.038**（误差棒为 1 个标准差，原文明说）。同时 α = 0.891 ± 0.10，δ_liquidity = 0.267 ± 0.22。
  **但**：他们拟的是**参与率形式**（§1.3），且**同文自陈 R² < 1%**（原文 §4.3 逐字：「The R² values for these regressions are typically less than one percent」）。**用 R² < 1% 的回归去 95% 否定一个指数，这件事本身要打折。**
- **支持 0.5**：Sato & Kanazawa 2025（Phys. Rev. Lett. 135, 257401；[arXiv:2411.13965](https://arxiv.org/abs/2411.13965)）用东交所**全市场普查**（全部交易账户、约 10⁸ metaorder）报 δ = 1/2 在统计误差（约 0.1）内成立，**逐股与逐交易者两层都成立**，并否定了 GGPS 与 FGLW 两个「非普适」模型（这两个模型把 δ 绑定到 metaorder 尺寸/时长分布的幂律指数上）。
- **中间**：Tóth 2011 自陈文献带是 **0.4–0.7**，并点名 Almgren δ≈0.6、Moro 等马德里 δ≈0.5、伦敦 δ≈0.7。

**⟹ 我的建议：δ = 0.5 作点估计，0.4–0.7 作敏感性带。理由：支持 0.5 的证据（TSE 全普查 10⁸ 样本、BTC 100 万 metaorder）在样本量与选择偏差控制上都强于反对方（29,509 单、R²<1%、参与率形式）。**

**(b) 小单端的硬墙 —— 低于日成交量的 ~0.1%，律就变了**

Bucci, Benzaquen, Lillo, Bouchaud, *Crossover from linear to square-root market impact*, **Phys. Rev. Lett. 122, 108302 (2019)**（[arXiv:1811.05230](https://arxiv.org/abs/1811.05230)）。数据：**800 万笔美股机构成交**。结论：Q/V 大致在 **0.1% – 10%** 区间是平方根；**0.001% – 0.1% 区间是线性**。

**这条对本仓是承重的**：§3 推出的 L0 容量 ≈ 日成交量的 **0.09%**，**正好压在这条线上或线下** ⟹ **L0 及以下用平方根是超出验证范围的**。

**(c) 大单端的硬墙**

Almgren 等 2005 §2.1 逐字：「**Orders larger than a few percent of daily volume have substantial sources of uncertainty that are not modeled here, and we cannot claim that our model accurately represents them.**」他们样本的 X/V 中位数 0.62%、均值 1.51%、最大 88.62%。Tóth 2011 的 Q/V 覆盖到「a few %」。
⟹ §3 推出的 **L4 容量 = 日成交量的 246%（2.5 倍日成交量）**，**完全在验证范围之外，那一格是废数**。

**(d) 反面证据（主动找的，落在我们的市场上）**

`SLMolenaar/crypto-market-impact`（GitHub，**MIT 许可**）：
- 数据：**Binance aggTrades**（`data.binance.vision`，**与本仓 #303 裁定的数据源同源**），BTC/USDT 与 ETH/USDT **现货**；平静期 2025-08/09（BTC 4440 万笔、ETH 7900 万笔），压力期 2025-11。
- metaorder 用 Maitrier, Loeper & Bouchaud (2025) 的重建算法（合成 trader ID，同符号连续成交归组），得 BTC 92.1 万 / ETH 176 万个。
- 结果：**OLS δ = 0.098 – 0.129；MLP 局部斜率 0.026 – 0.042**。自陈结论「**The square-root law is absent in all four experiments**」「the equity sqrt law does not hold on 2025 crypto spot data」。
- 它自己登记的限制：「**No ground-truth trader IDs.** The reconstruction produces synthetic metaorders whose size distribution may not reflect real institutional flow.」

**怎么读这条**：它与 Donier & Bonart（同为 BTC，δ≈0.5）直接矛盾。**最可能的分歧点是重建**：Donier & Bonart 有**真实唯一 trader ID**（这正是他们反复强调的数据优势），而这份复现只有合成 ID。这一路的偏差方向是**有文献在讨论的**——Naviglio, Bormetti, Campigli, Rodikov, Lillo, *Why is the estimation of metaorder impact with public market data so challenging?*（[arXiv:2501.17096](https://arxiv.org/abs/2501.17096)，**Quantitative Finance 26(4):505–523, 2026**）整篇就在讲从公开数据估 metaorder 冲击为什么会系统性走样。**但那篇我只读了摘要与二手转述，没有逐页核实**（§4 记为低信心）。

**⟹ 保守读法：在 2025 年的 Binance 现货上，用公开数据能不能复现平方根律，是一个开放问题。这不推翻 Donier & Bonart 的 2011–2013 MtGox 结论，但它意味着：如果本仓将来要自证 Y，光有逐笔还不够，还要处理重建偏差。**

**(e) metaorder vs 单笔 是不是承重 —— 是**，见 §1.1。三个量的指数分别是 0.5 / 0.2 / 趋于 1，**用错对象会错到指数级**。

### Q4. 只用 OHLCV 能走多远

**结论：σ_日 和 V_日 都能从 1 分钟 OHLCV 算出来，误差不是瓶颈；瓶颈在 Q 那一侧 —— 但对「算容量」而言 Q 是我们要解的未知数，不是要观测的输入。⟹ 算容量能做；自标 Y 做不到。**

**(a) 公式要的四样东西，各自够不够得着**

| 输入 | 从 1m OHLCV 拿得到吗 | 说明 |
|---|---|---|
| **σ_日** | **能** | 由日 OHLC 聚合后用 Parkinson/GK/RS 估计（见下） |
| **V_日** | **能** | `volume` 字段三处 schema 都有（票面已核实）；按日求和 |
| **Y** | **不能** | 需要重建 metaorder ⟹ 需要带方向的逐笔 |
| **Q** | **不需要观测** | 算容量时 Q 是**解出来的量**，不是输入 |

**⟹ 这是本问最重要的一句：算容量不需要观测 metaorder。** 观测 metaorder 只在「自己标 Y」时才需要。**借文献的 Y ⟹ OHLCV 够用。**

**(b) OHLC 估波动率有没有一手文献支持 —— 有，而且冲击标定文献自己在用**

- **一手源**（三篇原文我**都未能打开**，均在付费墙内，§4 记为限制）：
  - Parkinson, M. (1980), *The extreme value method for estimating the variance of the rate of return*, **Journal of Business 53(1):61–65**
  - Garman, M. B. & Klass, M. J. (1980), *On the estimation of security price volatilities from historical data*, **Journal of Business 53(1):67–78**
  - Rogers, L. C. G. & Satchell, S. E. (1991), *Estimating variance from high, low and closing prices*, **Annals of Applied Probability 1(4):504–512**
- **冲击标定文献自己就用 OHLC 估计量**：Vasaikar 2026 §3 逐字「non-dimensionalise by daily volatility σ_D (**Parkinson high–low of the mid**)」。**⟹ 拿 OHLC 估的 σ 去标定平方根律，是在案做法，不是我们的将就。**

**(c) 误差有多大 —— 我自己做了蒙特卡洛，没有引二手转述的数**

无漂移布朗运动，真值 σ² = 1，日内离散为 N 步（N=1440 对应我们的 1 分钟 bar），30,000–60,000 个模拟日：

| 估计量 | 均值（偏差） | 估计量方差 | 相对 close-to-close 的效率 |
|---|---|---|---|
| close-to-close | 1.0250（**+2.5%**） | 2.109 | 1.00 |
| Parkinson | 0.9752（−2.5%） | 0.408 | **5.18×** |
| Garman-Klass | 0.9560（−4.4%） | 0.257 | **8.21×** |
| Rogers-Satchell | 0.9529（−4.7%） | 0.319 | **6.62×** |

加漂移（drift = 2σ/日，极端情形）后：

| 估计量 | 均值偏差 | 效率 |
|---|---|---|
| close-to-close | **+396%** | 1.00 |
| Parkinson | **+142%** | 7.11 |
| Garman-Klass | **+44%** | 30.9 |
| Rogers-Satchell | **−7%** | 44.2 |

**读法**：
1. 我的 Parkinson 效率 5.18× 与文献常引的 **5.2** 一致；GK 我得 8.21×，**高于常引的 7.4** —— 我打不开 GK 1980 原文，无法确认他们「效率」的定义（可能基线或估计量变体不同）。**⟹ 我只声明我自己算的数，7.4 那个数我标为未核实。**
2. **离散偏差**：N=1440 时区间估计量在方差上低估 2.5–4.7%（σ 上约 1.2–2.4%）。**对我们的 1 分钟 bar，这是小量。**
3. **漂移敏感性**：只有 Rogers-Satchell 在有漂移时无偏。**BTC 是趋势资产 ⟹ 若按级别取长窗口（L3/L4 的时间尺度），漂移会污染 Parkinson/GK。这是选估计量时的实际考量，不是教科书细节。**

**(d) 本仓 BTC 全史实测（我跑的，非 #907 读数）**

数据 `analysis/data_cache/btc_1m_full.json`，**4,613,599 根 1m bar**（与 #907 的 FULL 窗**逐位相同**），2017-08-17 → 2026-05-31，聚合为 **3,210 个日历日**：

| 估计量 | RMS σ_日 | 中位日 σ_日 |
|---|---|---|
| close-to-close | 3.576% | 1.493% |
| Parkinson | 3.847% | 2.403% |
| **Garman-Klass** | **3.946%** | **2.531%** |
| Rogers-Satchell | 4.018% | 2.476% |

| 量 | 中位 | 均值 |
|---|---|---|
| V_日（BTC） | 38,234 | 60,471 |
| **V_日（USD 名义）** | **$1.2255e9** | $1.7510e9 |

最近 730 日：GK 中位日 σ_日 = **1.986%**，V_日 中位 = **$1.6924e9**。

**⟹ 关键的一条：估计量之间的分歧（中位日 2.40 / 2.53 / 2.48，差 5.3%）远小于「取哪个统计量」的分歧（GK 中位日 2.53% vs GK RMS 3.95%，差 56%）。** 容量 ∝ σ^{-1/δ} = σ^{-2} ⟹ 估计量选择带来 **1.11×** 的容量差异，而中位/RMS 选择带来 **2.43×**。**⟹ 别在估计量上纠结，先裁「σ_日 取中位日还是 RMS」。**

### Q5. 可复现的开源实现

**结论：有能跑的，但没有一个是「拿来就能给出可援引系数」的。已查到 4 处（检索式见 §4）。**

| 仓库 | 语言 | 许可 | 标定数据 | 能不能直接跑 | 对本仓的价值 |
|---|---|---|---|---|---|
| **[SLMolenaar/crypto-market-impact](https://github.com/SLMolenaar/crypto-market-impact)** | Python | **MIT** | Binance aggTrades（`data.binance.vision`）BTC/USDT+ETH/USDT 现货，2025-08/09 + 2025-11 | **能**（数据源公开且与本仓同源） | **最高**。它是**我们想做的事的现成实现**，且已给出否定结论 ⟹ **若要自证 Y，先复现它、再找它与 Donier-Bonart 的分歧点** |
| [joshuapjacob/almgren-chriss-optimal-execution](https://github.com/joshuapjacob/almgren-chriss-optimal-execution) | Python | 未核 | 日频历史价（3 个月，两资产） | 能 | 低。是**最优执行轨迹求解器**，不做冲击标定 |
| [IlluvatarEru/ElectronicMarkets](https://github.com/IlluvatarEru/ElectronicMarkets) | Python(notebook) | 未核 | 无自带标定 | 部分 | 低。二次开发 Almgren 2005，**β 取 0.5 是它自己选的，不是标出来的** |
| [braverock/blotter](https://rdrr.io/github/braverock/blotter/man/acOptTxns.html) `acOptTxns` | **R** | GPL（blotter 包） | 无 | 能 | 低。系数全靠调用方传入（`sigma, gamma, eta, lambda`） |

**⟹ 没有一个开源实现自带「可援引的 Y」。可援引的 Y 只在论文里（Donier & Bonart 的 0.9）。开源实现的价值是复现路径，不是系数来源。**

**未核实项**：`joshuapjacob` 与 `IlluvatarEru` 两个仓的许可证我没打开 LICENSE 文件确认，只登记为「未核」。

### Q6. ★ 直接给结论：只有 1 分钟 OHLCV，最站得住的算式是什么

**结论：能做。算式如下。差的一步不是「算不出容量」，而是「标不出 Y」——Y 只能借。**

```
容量(S, ℓ)  =  V_日(S) · [ x · A_ℓ(S) / ( Y · σ_日(S) ) ] ^ (1/δ)
```

逐项：

| 参数 | 取值 | 来源 | 不确定度 | 从 OHLCV 拿得到？ |
|---|---|---|---|---|
| **V_日** | BTC 中位 **$1.2255e9**（全史）／$1.6924e9（近 730 日） | **本仓 1m OHLCV，我算的**（§Q4d） | 全史 vs 近期差 1.38× | ✅ |
| **σ_日** | BTC GK 中位日 **2.531%**（全史）／1.986%（近 730 日）；GK RMS 3.946% | **本仓 1m OHLCV，我算的** | **估计量间 5.3%；中位 vs RMS 56%** ⟹ 容量差 **2.43×** | ✅ |
| **Y** | **0.9** | **Donier & Bonart 2015 §4.3，BTC 专属标定**，Y₀=0.90, Σ_Y=0.35 | 分布 N(0.9, 0.35)，「essentially in [0,2]」⟹ 容量在 ±1σ 内差 **(1.25/0.55)² = 5.2×** | ❌ **必须借** |
| **δ** | **0.5** | Tóth 2011 / Donier & Bonart 2015 / Sato & Kanazawa 2025 | 文献带 0.4–0.7 | ❌ 必须借 |
| **A_ℓ** | #907 ROUNDTRIP 中位：68.5 / 116.3 / 201.9 / 506.6 / 3572.7 bp | **#907 实测**（BTC 全史 100%） | L4 跨窗不稳 46×，**不可用**；分布右偏（均值/中位 2.1–2.9） | ✅（#907 已产） |
| **x** | **未裁** | 本票不裁，归 #914/#915 | — | — |

**这个算式的有效区间（硬约束，不许越界）**：`Q/V_日 ∈ [0.1%, 10%]`（Bucci 等 2019 PRL）。越下界变线性，越上界超出全部一手源的样本范围。

**「必须有逐笔/盘口数据、OHLCV 做不到」的那一半是什么**：

| 想做的事 | OHLCV 够不够 | 差在哪一步 |
|---|---|---|
| **算某个级别的容量** | **够** | — |
| **自己标定 Y**（不借文献） | **不够** | 需把匿名成交流切成 metaorder ⟹ 需**带买卖方向的逐笔**。三篇一手源的做法：Donier & Bonart 用真实唯一 trader ID；Vasaikar 2026 用 Nasdaq **Level-3 MBO**、按 30 秒分桶的成交量加权不平衡符号做「dominant-side runs」（阈值 D>0.3）；SLMolenaar 用 Binance **aggTrades**。**没有一条路只用 OHLCV。** |
| **验证 δ 在本市场是不是 0.5** | **不够** | 同上 |
| **算价差成本 / 盘口深度** | **不够** | 需盘口（`rust/src/theta_v0/venue_fee.rs:58-59` 已自陈「按 bar close 市价成交，无限价挂单语义」） |

**⟹ 一句话交付：**
> **把 `Y·σ_日` 当一个整体来标，σ_日 用本仓 OHLCV 算（GK 或 RS），Y 借 Donier & Bonart 的 0.9，δ 取 0.5。这样得到的容量是「基于文献系数的估算」，名分不能写成实测。要把它升成实测，本仓必须先有带方向的逐笔数据 —— 这是一件数据采集任务，不是算法任务。**

### Q7. 跨标的可比性：比值 w(d) 在文献里被讨论过吗

**结论：这个比值本身文献里没人讨论过（我没检索到）。但从公式可以证明它比绝对容量好得多 —— Y、σ、V、x 全部约掉，只剩 δ 和波幅阶梯。而 δ 恰恰是文献里普适性证据最强的那一项。**

**(a) 文献直接讨论「逐级容量比」的：没有查到。** 检索式见 §4。这不奇怪——「按缠论级别分层」是本仓自造的分层，文献没有对应对象。

**(b) 但间接支持是强的，来自「哪一项更普适」的证据分层：**

| 参数 | 普适性证据 | 强度 |
|---|---|---|
| **δ** | Sato & Kanazawa 2025：TSE 全市场普查 ~10⁸ metaorder，**逐股逐交易者** δ=1/2，且**否定了两个非普适模型**；观测到的 δ 离散**全部归因于有限样本效应** | **最强** |
| **Y** | Donier & Bonart 2015：BTC 的 0.9「close to the value reported on mature markets」；Vasaikar 2026 引「worldwide cross-sectional band [0.5, 1.0]」 | 中（有跨市场带，但带宽 2 倍） |
| **σ, V** | 按定义随标的变 | 无（也不需要） |

**⟹ §3 的推导给出 `w(d) = (A_{ℓ-d}/A_ℓ)^{1/δ}` ——它只依赖 δ 和波幅阶梯。**
- **Y 完全不进来** ⟹ 「本仓那个 0.1 是填的」这件事**对比值不构成障碍**（对绝对容量才构成）。
- **跨标的可比性因此被归约为：不同标的的波幅阶梯 `A_ℓ(S)` 是否形状相同。** 这**不是市场冲击问题，是 #907 那类的测量问题**（#907 只测了 BTC，其余 7 品种探针已支持 `P907_SYMBOL` 但未跑）。

**⟹ 这是本票对 map #787「递归自相似」立图前提最实的一条：比值的标度不变性，等价于波幅阶梯的跨标的一致性。要验它，跑 #907 探针的另外 7 个品种即可，不需要任何新的市场微观数据。**

---

## 3. ★ 我的推导（**不是文献结论**，是我从上面的公式和 #907 的读数推的）

> **本节全部为本报告作者的推导。每一步用了什么假设、哪些假设没有文献背书，逐条标出。**

### 3.1 设定

**容量定义**（照票面/#914 的定式）：容量(S, ℓ) ＝ 使**冲击成本不超过该级别往返波幅的比例 x** 的最大下单量。

**假设 A1**（平方根律成立，**有文献背书**，Tóth 2011 Eq.1）：`Δ(Q) = Y·σ_日·(Q/V_日)^δ`
**假设 A2**（σ_日、V_日 是**日**的量，与级别 ℓ **无关**，**有文献背书**，三篇一手源全用日）
**假设 A3**（**无文献背书 —— 这是本仓自造的映射**）：级别 ℓ 的「冲击预算」＝ `x · A_ℓ`，其中 A_ℓ ＝ #907 的 ROUNDTRIP 波幅。

### 3.2 解出容量

令 `Δ(Q*) = x·A_ℓ`：

```
Y·σ_日·(Q*/V_日)^δ = x·A_ℓ
(Q*/V_日)^δ = x·A_ℓ / (Y·σ_日)
Q*(ℓ) = V_日 · [ x·A_ℓ / (Y·σ_日) ]^(1/δ)            ……(★)
```

### 3.3 比值 —— **Y、σ、V、x 全部约掉**

```
w(d) = 容量(ℓ−d) / 容量(ℓ) = ( A_{ℓ−d} / A_ℓ ) ^ (1/δ)      ……(★★)
```

**这是本节最重要的结构性结论：**
- **与 Y 无关** ⟹ 「系数从来没有来源」这件事**不影响比值**；
- **与 σ_日、V_日 无关** ⟹ 与标的的绝对流动性无关；
- **与 x 无关** ⟹ 裁定方怎么定 x 都不改变比值；
- **与「一条腿还是一趟往返计冲击」无关**（那是逐级相同的常数因子，约掉）；
- **只剩 δ 和波幅阶梯。**

### 3.4 用 #907 的真实数字算出来

**输入**（#907 报告 §3.3，FULL 窗 ＝ BTC 全史 4,613,599 bar ＝ 100%，ROUNDTRIP 主口径）：

| 级别 | 中位波幅 A_ℓ (bp) | n |
|---|---|---|
| L0 | 68.5 | 18,247 |
| L1 | 116.3 | 8,913 |
| L2 | 201.9 | 4,387 |
| L3 | 506.6 | 2,311 |
| L4 | 3572.7 | 676（**#907 自证跨窗不稳 46×，不可用**） |

**逐级 w(1)（δ = 0.5，即指数 1/δ = 2）**：

| 级别对 | A 比 | **w(1)** | 容量倍数 1/w |
|---|---|---|---|
| L0/L1 | 0.5890 | **0.347** | 2.88× |
| L1/L2 | 0.5760 | **0.332** | 3.01× |
| L2/L3 | 0.3985 | **0.159** | 6.30× |
| ~~L3/L4~~ | ~~0.1418~~ | ~~0.020~~ | **不可用（L4 不稳）** |

**在可用的三级上取几何平均**：`w(1) = (68.5/506.6)^2 ^(1/3)` = **0.263**

> ### **⟹ w(1) ≈ 0.26**（每往下一级，容量掉到约 **1/3.8**）

### 3.5 这个数有多硬 —— 三层不确定度，全部报出来

**(a) δ 的不确定度**（文献带 0.4–0.7）：

| δ | w(1)（中位波幅） | 1/w |
|---|---|---|
| 0.4 | 0.189 | 5.30× |
| **0.5** | **0.263** | **3.80×** |
| 0.6 | 0.329 | 3.04× |
| 0.7 | 0.386 | 2.59× |

**(b) 用波幅分布的哪个分位数**（#907 明写分布强右偏，均值/中位 = 2.1–2.9）：

| 统计量 | w(1)（δ=0.5） | 1/w |
|---|---|---|
| p25 | 0.438 | 2.29× |
| **p50（中位）** | **0.263** | **3.80×** |
| 均值 | 0.221 | 4.52× |
| p75 | 0.198 | 5.06× |

**(c) 合并包络**：w(1) ∈ **[0.13, 0.55]**（δ=0.4+p75 到 δ=0.7+p25）。

**(d) ★ 最重要的一条：w(1) 不是常数。** 逐级 0.347 / 0.332 / 0.159 —— 前两级几乎相同（≈1/3），**第三级掉了一半以上**。
**⟹ 在本仓 BTC 数据上，「每下一级容量按固定比例衰减」这个标度不变假设，在 L2→L3 处就不成立了。**
（这一条本身不依赖 δ 或 x：三级的比值离散是波幅阶梯自带的，δ 只改变整体指数，不改变「三级不相等」这件事。）

### 3.6 绝对容量（比值之外，顺带算出来的；不确定度大得多）

取 x = 10%（**这个 x 是我为了给出一个数而选的，本票不裁 x**）、σ_日 = GK 中位日 2.531%、V_日 = $1.2255e9、Y = 0.9、δ = 0.5、**单腿口径**：

| 级别 | Q*/V_日 | **容量（USD）** | **落在 Bucci 等 2019 的哪个区间** |
|---|---|---|---|
| L0 | **0.090%** | $1.11 M | **⚠️ 在 0.1% 线上/线下 ⟹ 线性区，平方根律不适用** |
| L1 | 0.261% | $3.20 M | ✅ 平方根区 |
| L2 | 0.786% | $9.63 M | ✅ 平方根区 |
| L3 | 4.95% | $60.6 M | ✅ 平方根区（上半段） |
| L4 | **246%** | ~~$3.02 B~~ | **❌ 2.5 倍日成交量，超出全部一手源样本范围，废数** |

**若按往返两条腿各付一次冲击，上列容量全部 ÷4**（L0 → $277K）。

**⟹ 一条硬发现：级别阶梯横跨了平方根律的有效边界。** 下端（L0 及 #817 阶段二想去的更低级）掉进**线性区** ⟹ 那里 1/δ 应取 **1** 而不是 2 ⟹ **w(1) = A_{ℓ-1}/A_ℓ ≈ 0.59，而不是 0.35 —— 往下走时容量衰减得比平方根律预测的慢。** 上端 L4 直接出界。

### 3.7 哪些步骤推不过去 —— **照实写，不硬凑**

1. **A3（把「级别的往返波幅」当冲击预算）没有任何文献背书。** 文献从不按「级别」分层。**这是本仓自造的映射，名分应为 `[新缠论:选择]`。** 它可否证：若裁定方认为预算应挂在别的量上（例如该级别的 σ、或该级别的期望收益），(★★) 的形式不变，但 A_ℓ 要换成那个量。
2. **σ_日、V_日 与级别无关（A2）这一步，我用的是「文献都用日」这条事实，不是「有人验过跨窗不变」。** §1.2 已证公式在扩散标度下窗口不变，但**那是自洽性，不是实证**。而且 Donier & Bonart 自己的 BTC 数据给出 `δ' ≈ 0.4` 的执行速度依赖 ⟹ **这一步有已知的、量级不小的偏离。**
3. **σ 与波幅的关系我完全没有用**（我直接拿 A_ℓ 当预算，绕开了「A_ℓ 是几倍 σ」这个转换）。**这是有意的**——那一步才是最容易推不过去的地方，绕开它反而更稳。
4. **中位数不是保守值。** #907 自陈 L0–L3 的 p10 比值全 < 1（0.31/0.38/0.43/0.78）⟹ 用中位波幅算出的容量是**中位情形**的容量，不是下界。**要下界，得用 p10/p25 重算，w(1) 会变（§3.5b）。**
5. **单标的、单粒度。** 全部数字来自 BTC 1m。#907 明写不得外推；本仓在案「跨标的 L3 结论 BTC 独有」。**w(1) ≈ 0.26 是 BTC 的数，不是缠论的数。**
6. **持有成本没进来。** L4 中位持有 486 天的 funding/borrow，#907 已登记为未计入。它不改变 (★★)（不是冲击项），但会改变「这个容量值不值得用」。

### 3.8 顺带：对 `src/newchan/cost/slippage.py` 的两条具体订正建议

（我逐行打开 `src/newchan/cost/slippage.py:104-143` 核过。）

**订正 1 —— `impact_coeff` 的量纲**：`:139` 的 `impact_pct = self.impact_coeff * math.sqrt(participation_rate)` **没有 σ** ⟹ `impact_coeff` 在数学上就等于 `Y·σ_日`：

| σ_日 取法 | Y=0.9（BTC 一手） | Y=0.69（AAPL c_raw） | Y=0.34（AAPL c_eff） |
|---|---|---|---|
| GK 中位日 2.531% | **0.0228** | 0.0175 | 0.0086 |
| GK RMS 3.946% | 0.0355 | 0.0272 | 0.0134 |
| GK 中位日（近 730 日）1.986% | 0.0179 | 0.0137 | 0.0068 |

**⟹ `:119` 硬编码的 `0.1` 比文献值大 2.8×–14.8×，最可比的那格（BTC 一手 Y=0.9 + BTC 实测 σ）是 4.4×。**
**建议**：把 `impact_coeff` 拆成显式的 `Y`（借文献，默认 0.9，标注出处）× `sigma_daily`（从 OHLCV 实算传入）。**这样这个系数就有出处了，而且随标的自动变。**

**订正 2 —— `threshold_pct` 的死区方向错了**：`:118` 的 `threshold_pct = 0.01` 使 `:136-137` 在 `Q/ADV < 1%` 时直接 `return price`（冲击恒 0）。而 §3.6 算出 L0/L1/L2 的容量是 ADV 的 **0.090% / 0.261% / 0.786%** —— **全部在死区内。**
更根本的是：Tóth 等 2011 p.3 逐字「the emphasis should rather be placed on the **anomalous high impact of small trades**... Eq. (1) implies that marginal impact diverges for small volumes as Q^{-1/2}」。**平方根律的核心主张就是小单冲击异常地大；一个「小于 1% ADV 就没有冲击」的门槛，把要实现的定律实现反了。**
**建议**：删掉阈值；若一定要留下界，按 Bucci 等 2019 PRL 在 **0.1% ADV** 处切到**线性**，而不是切到零。

---

## 4. 本报告没做到的

### 4.1 检索式与覆盖

**用过的检索式（WebSearch，全部为英文主检索）**：
1. `Almgren Thum Hauptmann Li "Direct Estimation of Equity Market Impact" 2005 Risk`
2. `Tóth Lemperiere Deremble de Lataillade Kockelkoren Bouchaud "Anomalous price impact and the critical nature of liquidity" Physical Review X 2011 arXiv`
3. `Donier Bonart "million metaorders" bitcoin market impact square root law`
4. `square root law market impact criticism "not universal" exponent estimate bias metaorder linear impact evidence against`
5. `square root law impact "sigma sqrt(Q/V)" invariance time window daily volatility daily volume "does not depend" duration scale invariant Bouchaud`
6. `Garman Klass 1980 "On the estimation of security price volatilities from historical data" Journal of Business efficiency Parkinson 1980 extreme value Rogers Satchell 1991`
7. `github open source implementation market impact model square root law Almgren-Chriss python library calibration`
8. `Bucci Benzaquen Lillo Bouchaud "Crossover from linear to square-root market impact" Physical Review Letters 2019 small metaorder`
9. `Naviglio "bias in metaorder impact reconstruction from anonymous order flow" 2025 overestimate prefactor`

**逐页读过原文的（PDF 全文，非摘要非转述）**：
- Almgren 等 2005（p.1–22，含全部方程与 Table 1/2/3）
- Tóth 等 2011（p.1–8，含 Eq.1–5 与 Fig.1）
- Donier & Bonart 2015（p.1–14，含 Eq.1–9、Table I、Fig.5–11、§4.3 Y-ratio 全节）
- Vasaikar 2026（p.1–5 全文）

**只读了摘要/元数据的（**信心低，已在正文标注**）**：
- Sato & Kanazawa 2025（PRL 135, 257401）—— 只读 arXiv 摘要页
- Bucci 等 2019（PRL 122, 108302）—— **只读搜索摘要，未开原文**；0.1%/10% 那两个边界数字来自二手转述，**这是 §3.6 的承重数字，却是本报告信心最低的一条**
- Naviglio 等 2026（Quantitative Finance 26(4)）—— 只读摘要；「重建高估前置因子 ~2×」来自 Vasaikar 2026 的转引，**我未在 Naviglio 原文核实这个倍数**
- Maitrier, Loeper & Bouchaud 2025（arXiv:2503.18199）—— 只读摘要
- Barone & Lillo 2026（arXiv:2606.15715，Hyperliquid）—— 只读摘要，**未取任何数字**

**完全没打开的（付费墙）**：
- Garman & Klass 1980、Parkinson 1980、Rogers & Satchell 1991 **三篇原文全部未开**（CME 镜像 PDF 两次超时）。**⟹ 文献常引的「GK 效率 7.4×」我未能核实，报告里用的是我自己的蒙特卡洛数（8.21×）。**
- Bouchaud, Bonart, Donier & Gould (2018), *Trades, Quotes and Prices*, Cambridge University Press —— **未开**。它是本领域标准专著，**本报告没有从它取任何数字**。

### 4.2 穷举断言的边界

- **「加密货币独立标定已查到 2 处 + 1 处未核」**（Q2）—— 覆盖 ＝ 上列检索式 3、4、7 的结果页 + Donier & Bonart 的引用网络。**不声明穷举。**
- **「开源实现已查到 4 处」**（Q5）—— 覆盖 ＝ 检索式 7 的结果页。**只搜了英文、只搜了 GitHub/rdrr**，未搜 GitLab、未搜 PyPI/crates.io。**不声明穷举。** 其中 2 处许可证未开 LICENSE 核实。
- **「文献没有讨论过逐级容量比 w(d)」**（Q7）—— 这是**否定式断言，最不可靠**。我只用了检索式 5 的近义检索，**没有专门为 w(d) 设计检索式**。**准确说法是：在我跑过的 9 条检索式的结果里没有出现。**

### 4.3 信心分级

| 断言 | 信心 | 理由 |
|---|---|---|
| 公式形式 `Δ = Y σ sqrt(Q/V)`，σ/V 都是日 | **高** | 三篇原文逐字，我逐页读过 |
| BTC 的 Y ≈ 0.9 | **高** | Donier & Bonart §4.3 全节读过，Y₀=0.9/Σ_Y=0.35 有分布图 |
| δ 的文献带 0.4–0.7、主流 0.5 | **高** | Tóth 原文自陈 + Almgren 原文的 0.600±0.038 + TSE 摘要 |
| Y 与 η 不能互换、差 3–11× | **高** | 两篇原文的方程 + 我自己代数计算 |
| σ_日/V_日 可从本仓 OHLCV 算出（具体数值） | **高** | 我在 4,613,599 根 bar 上实算，bar 数与 #907 逐位一致 |
| OHLC 波动率估计量的效率 | **中** | 我自己的蒙特卡洛可信；**但与 GK 1980 原文的 7.4 对不上，原文未能打开** |
| **平方根区间是 [0.1%, 10%] ADV** | **低** | **二手转述，Bucci 原文未开。而这是 §3.6「L0 掉进线性区」的承重数字** |
| 2025 Binance 现货上 δ≈0.1 | **中** | 仓库 README 自陈，我未跑过；其自陈限制（合成 trader ID）足以解释分歧 |
| w(1) ≈ 0.26 | **中** | 代数正确、输入来自 #907 全史实测；**但 A3 假设无文献背书，且包络 [0.13, 0.55] 很宽** |
| **w(1) 不是常数（L2→L3 掉一半）** | **高** | 只依赖 #907 的波幅阶梯，不依赖任何借来的系数 |
| 文献未讨论过 w(d) | **低** | 否定式断言，检索式未专门设计 |

### 4.4 本报告没做、但下一步该做的

1. **打开 Bucci 等 2019 PRL 原文**，核实 0.1%/10% 两个边界。**这是当前最承重的低信心数字。**
2. **跑 #907 探针的另外 7 个品种**（`P907_SYMBOL` 已支持）。按 (★★)，跨标的可比性**完全归约为波幅阶梯的可比性** ⟹ 这一步不需要任何新数据类型。
3. **裁定 σ_日 取中位日还是 RMS**（2.43× 的容量差异，比估计量选择大 20 倍）。
4. **若要把容量从「借系数的估算」升成「实测」**：需采集带方向的逐笔（Binance aggTrades 公开可得），然后**先复现 `SLMolenaar/crypto-market-impact`**，弄清它与 Donier & Bonart 的分歧是重建偏差还是市场真的变了。
5. **`slippage.py` 的两条订正**（§3.8）。

---

## 5. 一手源清单（发表版）

1. **Almgren, R., Thum, C., Hauptmann, E. & Li, H. (2005).** Direct estimation of equity market impact. *Risk* **18**(7), 58–62. 工作稿 2005-05-10（初版 2004-12-20）。PDF: `courant.nyu.edu/~almgren/papers/costestim.pdf`
2. **Tóth, B., Lempérière, Y., Deremble, C., de Lataillade, J., Kockelkoren, J. & Bouchaud, J.-P. (2011).** Anomalous price impact and the critical nature of liquidity in financial markets. *Physical Review X* **1**, 021006. [arXiv:1105.1694](https://arxiv.org/abs/1105.1694)
3. **Donier, J. & Bonart, J. (2015).** A million metaorder analysis of market impact on the Bitcoin. *Market Microstructure and Liquidity* **1**(2), 1550008. [arXiv:1412.4503](https://arxiv.org/abs/1412.4503)
4. **Bucci, F., Benzaquen, M., Lillo, F. & Bouchaud, J.-P. (2019).** Crossover from linear to square-root market impact. *Physical Review Letters* **122**, 108302. [arXiv:1811.05230](https://arxiv.org/abs/1811.05230) —— **未开原文**
5. **Sato, Y. & Kanazawa, K. (2025).** Strict universality of the square-root law in price impact across stocks. *Physical Review Letters* **135**, 257401. [arXiv:2411.13965](https://arxiv.org/abs/2411.13965) —— **只读摘要**
6. **Naviglio, M., Bormetti, G., Campigli, F., Rodikov, G. & Lillo, F. (2026).** Why is the estimation of metaorder impact with public market data so challenging? *Quantitative Finance* **26**(4), 505–523. [arXiv:2501.17096](https://arxiv.org/abs/2501.17096) —— **只读摘要**
7. **Maitrier, G., Loeper, G. & Bouchaud, J.-P. (2025).** Generating realistic metaorders from public data. [arXiv:2503.18199](https://arxiv.org/abs/2503.18199) —— **只读摘要**
8. **Vasaikar, A. (2026).** Empirical Confirmation of the Square-Root Law of Market Impact in a U.S. Large-Cap Equity. [arXiv:2606.24019](https://arxiv.org/abs/2606.24019) —— **独立研究预印本，非同行评议**
9. **Bouchaud, J.-P., Bonart, J., Donier, J. & Gould, M. (2018).** *Trades, Quotes and Prices: Financial Markets Under the Microscope.* Cambridge University Press. —— **未开，本报告未从中取数**
10. **Parkinson, M. (1980).** *Journal of Business* **53**(1), 61–65. —— **付费墙，未开**
11. **Garman, M. B. & Klass, M. J. (1980).** *Journal of Business* **53**(1), 67–78. —— **付费墙，未开**
12. **Rogers, L. C. G. & Satchell, S. E. (1991).** *Annals of Applied Probability* **1**(4), 504–512. —— **付费墙，未开**

**代码**：`SLMolenaar/crypto-market-impact`（MIT）；`joshuapjacob/almgren-chriss-optimal-execution`；`IlluvatarEru/ElectronicMarkets`；`braverock/blotter::acOptTxns`（R）。

**仓内**：#907 报告（`a257400242`，`.chanlun/review-results/issue907-cost-gate-level-20260805.md`）；`src/newchan/cost/slippage.py:104-143`（逐行核过）；`rust/src/theta_v0/venue_fee.rs:50-59`（逐行核过）；`analysis/data_cache/btc_1m_full.json`（4,613,599 bar，我实算 σ_日/V_日）。
