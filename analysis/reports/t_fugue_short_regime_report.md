# T 赋格引擎 · 空头交易分段盈亏分析（按 price regime 归因）

- 数据源：`trading_system/data_cache/t_fugue_<SYM>_<MODE>.json`（T 赋格，多空双向）
- regime 分段：ZigZag 峰谷，主阈值 **theta=15%**（敏感性见末节 (0.08, 0.15, 0.3)）
- pnl 口径：已实现现金 / 初始资本 100,000 × 100 = 对 NAV 贡献%（可加总，与 strat_pct 同口径）
- 认识论等级：**L3**（真实数据逐笔归因，可否证）

## 1. 各标的 price regime 段落分布（ZigZag）

| 标的 | bars | BH% | 段数 | 上涨段 | 下跌段 | up占bar% | down占bar% |
|------|-----:|----:|----:|-------:|-------:|--------:|----------:|
| CL | 5,528,156 | +28 | 114 | 57 | 57 | 61 | 39 |
| BRN | 2,418,058 | +87 | 58 | 29 | 29 | 57 | 43 |
| DX | 2,046,008 | +4 | 2 | 1 | 1 | 50 | 50 |
| GC | 5,544,556 | +257 | 23 | 12 | 11 | 72 | 28 |
| ES | 5,589,928 | +594 | 15 | 8 | 7 | 86 | 14 |
| QQQ | 728,030 | +175 | 5 | 3 | 2 | 94 | 6 |
| BTC | 4,625,119 | +1380 | 187 | 93 | 94 | 64 | 36 |
| OKLO | 447,738 | +307 | 132 | 66 | 66 | 53 | 47 |

## 2. 空头交易按 regime 分段（顺势=下跌段 / 逆势=上涨段）

| 标的 | 模式 | 空头@下跌段(顺势) | 空头@上涨段(逆势) | 空头净 |
|------|------|---|---|---:|
| CL | structural | n=63    pnl=   +0.9%NAV win=  76% held=278605b | n=1748  pnl=   -6.0%NAV win=  57% held=1364779b | -5.1%NAV |
| CL | and | n=71    pnl=   +2.0%NAV win=  79% held=265311b | n=1718  pnl=  +14.7%NAV win=  56% held=1371046b | +16.8%NAV |
| CL | or | n=46    pnl=   +0.1%NAV win=  91% held=239591b | n=1977  pnl=   -4.8%NAV win=  59% held=1436738b | -4.7%NAV |
| BRN | structural | n=186   pnl=  -58.3%NAV win=  70% held=584965b | n=97    pnl=  -58.0%NAV win=  24% held=216543b | -116.3%NAV |
| BRN | and | n=155   pnl=  -90.2%NAV win=  65% held=630559b | n=113   pnl=  -24.8%NAV win=  35% held=209985b | -115.0%NAV |
| BRN | or | n=402   pnl=  +24.4%NAV win=  71% held=455099b | n=125   pnl=  -30.1%NAV win=  30% held=190962b | -5.7%NAV |
| DX | structural |                              — | n=643   pnl=   -2.0%NAV win=  59% held=604549b | -2.0%NAV |
| DX | and |                              — | n=626   pnl=   -1.7%NAV win=  60% held=604456b | -1.7%NAV |
| DX | or |                              — | n=755   pnl=   -1.0%NAV win=  63% held=618304b | -1.0%NAV |
| GC | structural | n=6     pnl=   +1.2%NAV win= 100% held=  6071b | n=1137  pnl=  -96.1%NAV win=  52% held=914957b | -94.9%NAV |
| GC | and | n=1237  pnl=  -98.6%NAV win=  59% held=2142327b | n=278   pnl=  -27.3%NAV win=  55% held=401365b | -125.8%NAV |
| GC | or | n=7     pnl=   +0.7%NAV win= 100% held=  6538b | n=1337  pnl=  -85.4%NAV win=  55% held=905192b | -84.6%NAV |
| ES | structural | n=338   pnl=  -19.7%NAV win=  51% held=784271b | n=1088  pnl= -129.9%NAV win=  41% held=1219395b | -149.6%NAV |
| ES | and | n=332   pnl=  -14.6%NAV win=  52% held=789509b | n=1052  pnl= -110.9%NAV win=  40% held=1268625b | -125.6%NAV |
| ES | or | n=218   pnl=  -58.0%NAV win=  58% held=912211b | n=1275  pnl=  -47.1%NAV win=  47% held=382262b | -105.1%NAV |
| QQQ | structural |                              — | n=219   pnl=  -40.4%NAV win=  20% held= 86141b | -40.4%NAV |
| QQQ | and |                              — | n=309   pnl=  -31.1%NAV win=  22% held= 79556b | -31.1%NAV |
| QQQ | or |                              — | n=361   pnl=  -19.5%NAV win=  27% held= 75984b | -19.5%NAV |
| BTC | structural | n=368   pnl=  +52.3%NAV win=  74% held= 70790b | n=942   pnl= -219.8%NAV win=  57% held=149598b | -167.4%NAV |
| BTC | and | n=176   pnl=  +86.6%NAV win=  72% held=295293b | n=1310  pnl= -178.1%NAV win=  60% held=668435b | -91.5%NAV |
| BTC | or | n=474   pnl=  +84.0%NAV win=  72% held= 74632b | n=908   pnl= -284.9%NAV win=  60% held=138484b | -200.8%NAV |
| OKLO | structural | n=13    pnl=  -68.3%NAV win=  69% held=  8915b | n=17    pnl= -269.6%NAV win=  47% held= 31384b | -337.9%NAV |
| OKLO | and | n=22    pnl=  -46.4%NAV win=  50% held=  9559b | n=108   pnl=  -95.0%NAV win=  73% held= 54973b | -141.3%NAV |
| OKLO | or | n=26    pnl=  -29.6%NAV win=  73% held= 11144b | n=147   pnl= -124.1%NAV win=  60% held= 78684b | -153.7%NAV |

## 3. 多头交易按 regime 分段（顺势=上涨段 / 逆势=下跌段）

| 标的 | 模式 | 多头@上涨段(顺势) | 多头@下跌段(逆势) | 多头净 |
|------|------|---|---|---:|
| CL | structural | n=1158  pnl=  -22.5%NAV win=  64% held=1305014b | n=434   pnl=  -11.9%NAV win=  73% held=382246b | -34.3%NAV |
| CL | and | n=1147  pnl=   -9.6%NAV win=  64% held=1310880b | n=420   pnl=  -16.6%NAV win=  72% held=383653b | -26.2%NAV |
| CL | or | n=1342  pnl=  -27.1%NAV win=  66% held=1311888b | n=489   pnl=  -12.3%NAV win=  72% held=382100b | -39.4%NAV |
| BRN | structural | n=159   pnl=  +30.5%NAV win=  69% held= 88898b | n=798   pnl=   -7.7%NAV win=  74% held=610450b | +22.7%NAV |
| BRN | and | n=225   pnl=  +28.0%NAV win=  65% held= 86905b | n=715   pnl=   -6.7%NAV win=  76% held=656423b | +21.3%NAV |
| BRN | or | n=193   pnl=  +25.5%NAV win=  69% held= 71493b | n=642   pnl=  +15.6%NAV win=  79% held=428674b | +41.1%NAV |
| DX | structural | n=883   pnl=   +3.2%NAV win=  69% held=499753b | n=176   pnl=   -0.0%NAV win=  62% held=471028b | +3.2%NAV |
| DX | and | n=999   pnl=   +3.6%NAV win=  67% held=560165b |                              — | +3.6%NAV |
| DX | or | n=1037  pnl=   +3.2%NAV win=  69% held=491441b | n=188   pnl=   -0.0%NAV win=  60% held=465740b | +3.2%NAV |
| GC | structural | n=1299  pnl=  +76.1%NAV win=  78% held=900811b |                              — | +76.1%NAV |
| GC | and | n=429   pnl=  -12.6%NAV win=  75% held=529708b | n=1747  pnl=  +56.0%NAV win=  66% held=2110755b | +43.4%NAV |
| GC | or | n=1508  pnl=  +83.5%NAV win=  78% held=924696b |                              — | +83.5%NAV |
| ES | structural | n=1719  pnl= +101.2%NAV win=  79% held=1178485b | n=451   pnl=  +22.8%NAV win=  78% held=834237b | +124.0%NAV |
| ES | and | n=1681  pnl=  +65.0%NAV win=  79% held=1192036b | n=453   pnl=  +14.1%NAV win=  77% held=827058b | +79.1%NAV |
| ES | or | n=1253  pnl=  +51.4%NAV win=  76% held=360633b | n=600   pnl=   +9.8%NAV win=  90% held=978152b | +61.2%NAV |
| QQQ | structural | n=255   pnl=  +55.4%NAV win=  88% held= 28323b | n=15    pnl=   -9.9%NAV win=  47% held=  7793b | +45.5%NAV |
| QQQ | and | n=211   pnl=  +74.8%NAV win=  87% held= 28186b | n=15    pnl=  -11.6%NAV win=  47% held=  7793b | +63.1%NAV |
| QQQ | or | n=272   pnl=  +85.1%NAV win=  85% held= 24233b | n=16    pnl=  -10.2%NAV win=  62% held=  7888b | +74.9%NAV |
| BTC | structural | n=756   pnl= +137.8%NAV win=  71% held= 90953b | n=466   pnl=  -66.3%NAV win=  50% held= 72293b | +71.4%NAV |
| BTC | and | n=304   pnl= +325.2%NAV win=  82% held=186334b | n=486   pnl=  -42.0%NAV win=  63% held=506279b | +283.2%NAV |
| BTC | or | n=772   pnl= +278.9%NAV win=  72% held= 82775b | n=680   pnl= -172.8%NAV win=  55% held= 81292b | +106.1%NAV |
| OKLO | structural | n=29    pnl=  -44.0%NAV win=  72% held= 30802b | n=12    pnl=  -24.1%NAV win=  58% held=  6725b | -68.1%NAV |
| OKLO | and | n=81    pnl=  +44.3%NAV win=  62% held= 52954b | n=26    pnl=  +71.0%NAV win=  73% held= 31674b | +115.4%NAV |
| OKLO | or | n=38    pnl=  +30.1%NAV win=  87% held= 25053b | n=59    pnl=  +50.5%NAV win=  59% held= 42726b | +80.6%NAV |

## 4. BTC 三模式对比（AND +191.7% vs Structural -96% vs OR -94.8%）

逐 polarity × regime 分解（NAV 贡献%）：

| 模式 | 空头@跌(顺) | 空头@涨(逆) | 多头@涨(顺) | 多头@跌(逆) | 空头净 | 多头净 | Σ已实现 | strat% |
|------|---|---|---|---|---:|---:|---:|---:|
| structural | +52(n368) | -220(n942) | +138(n756) | -66(n466) | -167 | +71 | -96 | -96.0 |
| and | +87(n176) | -178(n1310) | +325(n304) | -42(n486) | -91 | +283 | +192 | +191.7 |
| or | +84(n474) | -285(n908) | +279(n772) | -173(n680) | -201 | +106 | -95 | -94.8 |

## 5. 关键问题作答

**Q1: 下跌段做空到底赚不赚钱？**
- 全 24 组中，下跌段空头净盈利的有 **9/18** 组。
- 全样本下跌段空头 NAV 贡献合计 = **-231%**；上涨段空头合计 = **-1873%**。

**Q2: 整体空头为何负——上涨段亏太多 vs 下跌段赚太少？**
- CL/structural: 跌段+1% + 涨段-6% = -5% → 上涨段失血主导
- CL/and: 跌段+2% + 涨段+15% = +17% → 空头整体为正
- CL/or: 跌段+0% + 涨段-5% = -5% → 上涨段失血主导
- BRN/structural: 跌段-58% + 涨段-58% = -116% → 下跌段未能盈利主导
- BRN/and: 跌段-90% + 涨段-25% = -115% → 下跌段未能盈利主导
- BRN/or: 跌段+24% + 涨段-30% = -6% → 上涨段失血主导
- DX/structural: 跌段+0% + 涨段-2% = -2% → 上涨段失血主导
- DX/and: 跌段+0% + 涨段-2% = -2% → 上涨段失血主导
- DX/or: 跌段+0% + 涨段-1% = -1% → 上涨段失血主导
- GC/structural: 跌段+1% + 涨段-96% = -95% → 上涨段失血主导
- GC/and: 跌段-99% + 涨段-27% = -126% → 下跌段未能盈利主导
- GC/or: 跌段+1% + 涨段-85% = -85% → 上涨段失血主导
- ES/structural: 跌段-20% + 涨段-130% = -150% → 上涨段失血主导
- ES/and: 跌段-15% + 涨段-111% = -126% → 上涨段失血主导
- ES/or: 跌段-58% + 涨段-47% = -105% → 下跌段未能盈利主导
- QQQ/structural: 跌段+0% + 涨段-40% = -40% → 上涨段失血主导
- QQQ/and: 跌段+0% + 涨段-31% = -31% → 上涨段失血主导
- QQQ/or: 跌段+0% + 涨段-19% = -19% → 上涨段失血主导
- BTC/structural: 跌段+52% + 涨段-220% = -167% → 上涨段失血主导
- BTC/and: 跌段+87% + 涨段-178% = -91% → 上涨段失血主导
- BTC/or: 跌段+84% + 涨段-285% = -201% → 上涨段失血主导
- OKLO/structural: 跌段-68% + 涨段-270% = -338% → 上涨段失血主导
- OKLO/and: 跌段-46% + 涨段-95% = -141% → 上涨段失血主导
- OKLO/or: 跌段-30% + 涨段-124% = -154% → 上涨段失血主导

**Q3: AND 为何在 BTC 大幅改善（-96% → +192%）？**
- 上涨段空头（逆势）：structural n=942 -220% → AND n=1310 -178% （笔数Δ=368, NAVΔ=+42%）
- 下跌段空头（顺势）：structural n=368 +52% → AND n=176 +87%
- 上涨段多头（顺势）：structural n=756 +138% → AND n=304 +325%

## 6. theta 敏感性（结论边界条件）

「下跌段空头净盈亏符号」在不同 theta 下是否稳定（+ = 顺势空头赚钱）：

| 标的 | 模式 | θ=8% | θ=15% | θ=30% |
|------|------|---|---|---|
| BTC | structural | -117%(n372) | +52%(n368) | +68%(n574) |
| BTC | and | -96%(n991) | +87%(n176) | +53%(n984) |
| BTC | or | -127%(n436) | +84%(n474) | +120%(n521) |
| CL | structural | +1%(n60) | +1%(n63) | -1%(n74) |
| CL | and | +2%(n68) | +2%(n71) | -1%(n74) |
| CL | or | +1%(n59) | +0%(n46) | -1%(n60) |
| OKLO | structural | +10%(n9) | -68%(n13) | -62%(n19) |
| OKLO | and | -34%(n67) | -46%(n22) | -1%(n17) |
| OKLO | or | -33%(n120) | -30%(n26) | +7%(n21) |
| GC | structural | +1%(n6) | +1%(n6) | -7%(n203) |
| GC | and | -95%(n1234) | -99%(n1237) | -99%(n1281) |
| GC | or | +1%(n7) | +1%(n7) | -7%(n225) |
| ES | structural | -0%(n1) | -20%(n338) | +0%(n0) |
| ES | and | +0%(n2) | -15%(n332) | +0%(n0) |
| ES | or | -58%(n218) | -58%(n218) | +0%(n0) |

---

## 7. 深度解读与方法论边界

### 7.1 AND 改善 BTC 的真因 = 多头长持，不是空头修复
- 多头净 NAV：structural **+71%** → AND **+283%**（Δ **+212%**）
- 空头净 NAV：structural **-167%** → AND **-91%**（Δ +76%）
- 多头改善占总改善的 **73.6%**。机制：顺势多头（上涨段）笔数 756 → **304**（砍 60%），单类贡献 +138% → **+325%**（翻倍）。
- 即：AND 的 MACD 收紧门控**减少了顺势多头的过度短差（churn）**，让多头在 BTC 大牛市中长期持有吃透趋势。空头只是次要改善。

### 7.2 整体空头为负的真凶 = 上涨段逆势空头
- 全样本：上涨段空头合计 **-1873%** vs 下跌段空头合计 **-231%**，比例 **8.1 : 1**。
- 结论：空头亏损绝大部分来自**在上涨大趋势中开逆势空头并死扛**，不是"下跌段赚得太少"。
- 证据：逆势空头持有时长极长（BTC structural 上涨段空头 avg held 15万 bar；OKLO 3万 bar；GC/and 214万 bar）——被趋势持续碾压。

### 7.3 "下跌段做空赚钱"不普适，且强尺度依赖
- θ=15% 主口径：有下跌段空头交易的 18 组中仅 **9 组**净盈利。BTC/CL/BRN-or 顺势空头为正；GC/ES/OKLO 连下跌段空头都亏（强上涨标的的"下跌段"=回调陷阱 + 确认滞后税）。
- **尺度依赖（边界条件）**：BTC 下跌段空头净盈亏符号在 θ 间**翻转**——θ=8% 全负（-117/-96/-127），θ=15%/30% 全正（+52/+87/+84）。
  即：只有在**足够大级别**（≥15%）的下跌段做空才赚；小回调（8%）做空亏。这与缠论"做空要在足够大级别的下跌走势"一致。

### 7.4 方法论边界（诚实标注）
- regime 按 **entry_bar** 归类，对持有时长 ≪ 单段时长的交易有效；对**超长持仓**（如 GC/and 214万 bar，跨越多个 ZigZag 段）entry 标签会失真——这类"跨段死扛仓"的 regime 归属不能完全反映盈亏来源。
- 但这一失真本身即发现：**赋格引擎的部分空头声部持有过久、跨 regime 被碾压**，正是 7.2 的病灶。
- pnl 口径 = 已实现现金 / 初始资本；各标的四象限合计 ≈ strat_pct（OKLO -406% 精确吻合），口径自洽。
- 认识论等级 **L3**（真实数据逐笔归因，可否证）；regime 工具本身 L0；θ 敏感性即有效域边界。
