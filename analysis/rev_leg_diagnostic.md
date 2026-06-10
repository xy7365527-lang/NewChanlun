# REV 腿交易行为诊断 — O1 三标的全负的逐笔根因

> 数据：O1/O1v/P5 逐腿 trace × 磁带事件流离线交叉分类；中枢账本重放采样开腿结构位置。生成脚本 `rev_leg_diagnostic.py`。认识论 **L2**（OKLO/QQQ/BRN 真实 1min，逐笔可否证）。

## 0. 判决摘要

Δ(O1) 全负是**两个独立机制的叠加**（O1v 拆解：master 扩展贡献 = Δ(O1)−Δ(O1v) = OKLO -562.5pp[90%] / QQQ -39.0pp[49%] / BRN -263.6pp[59%]；其余为 voice REV 腿自身贡献）：

1. **master 出场触发扩展砍掉一半暴露（首要机制；OKLO 上压倒性，QQQ 上与 voice 同量级）**：rev_mode 把 master 出场从 sell1（confirmed type1 卖）扩展为段终结三触发后，绝大多数出场由扩展分量触发（ext_type3_conf_sell + ext_div_consolidation_sell），sell1 兼容出场占比 <2%。三标的持仓时间一致地掉到 P5 的 ~50%，P5 持有窗口内的空仓缺口在强趋势标的上错失复合收益数倍于本金。其中 ext_type3_conf_sell 出场是高频割肉循环（平均持有最短、平均 pnl 为负）。
2. **voice REV 腿自身负贡献（O1v 隔离：-63.7/-41.4/-186.2pp）**：两个结构性亏损通道——(a) T1 动作『先 CLOSE_OSC』把高胜率域腿在卖侧信号 bar 强制回补（截断腿胜率 20-31% vs 正常闭腿 66-77%）；(b) t6_pre_type3_buy 关腿槽（REV 卖出后价格不跌反向上离开中枢，candidate type3 买预逃逸高位回补）是 REV 腿内唯一系统性大负槽（三标的胜率一致 ≈30%），把 t5 设计剧本（下跌背驰买回，赚钱）的利润全部吃掉。

## 1. Δ 的分解：亏损载体不是 REV 配对差价，而是暴露缺口

| 标的 | O1 持仓 bars | P5 持仓 bars | 暴露比 | P5 窗口内空仓错失(复合%) |
|------|-------------|-------------|--------|----------------|
| OKLO | 151,051 | 299,763 | 0.504 | +308.9 |
| QQQ | 219,588 | 445,300 | 0.493 | +44.9 |
| BRN | 792,626 | 1,596,640 | 0.496 | +209.0 |

### 空仓缺口按造成缺口的出场触发归因（复合%）

| 标的 | 触发 | 缺口数 | 错失复合% |
|------|------|--------|-----------|
| OKLO | sell1_P5_compatible | 5 | -2.3 |
| OKLO | ext_div_consolidation_sell | 421 | +56.2 |
| OKLO | ext_type3_conf_sell | 359 | +168.0 |
| QQQ | ext_div_trend_sell | 4 | +2.6 |
| QQQ | sell1_P5_compatible | 8 | +6.8 |
| QQQ | ext_type3_conf_sell | 807 | +8.6 |
| QQQ | ext_div_consolidation_sell | 922 | +21.7 |
| BRN | entry_lag | 3 | -2.1 |
| BRN | sell1_P5_compatible | 19 | +0.3 |
| BRN | ext_type3_conf_sell | 2212 | +44.0 |
| BRN | ext_div_consolidation_sell | 2549 | +118.6 |

## 2. O1 master 出场触发归因（哪个扩展分量砍了暴露）

| 标的 | 出场触发 | 笔数 | 平均持有bars | 平均pnl% |
|------|---------|------|--------------|----------|
| OKLO | ext_div_consolidation_sell | 428 | 258 | +0.805 |
| OKLO | ext_type3_conf_sell | 361 | 102 | -0.770 |
| OKLO | sell1_P5_compatible | 14 | 269 | +3.636 |
| QQQ | ext_div_consolidation_sell | 930 | 184 | +0.093 |
| QQQ | ext_type3_conf_sell | 807 | 50 | -0.095 |
| QQQ | sell1_P5_compatible | 33 | 215 | +0.438 |
| QQQ | ext_div_trend_sell | 4 | 144 | +0.417 |
| BRN | ext_div_consolidation_sell | 2576 | 239 | +0.155 |
| BRN | ext_type3_conf_sell | 2222 | 73 | -0.189 |
| BRN | sell1_P5_compatible | 70 | 203 | +1.086 |

## 3a. O1 — REV 腿逐笔聚合

### 开腿触发

| 标的 | 分组 | 对数 | 胜率% | avg_diff% | 净现金 | 平均持有bars |
|------|------|------|-------|-----------|--------|--------------|
| OKLO | t1_conf_sell | 169 | 51.5 | 0.1456 | +25173 | 85 |
| OKLO | t3_conf_sell | 122 | 54.9 | 0.0931 | +10882 | 73 |
| OKLO | div_consolidation_sell | 113 | 45.1 | 0.1168 | +13598 | 51 |
| OKLO | div_trend_sell | 3 | 66.7 | 1.9673 | +5985 | 83 |
| QQQ | t1_conf_sell | 470 | 44.9 | -0.0017 | -798 | 52 |
| QQQ | t3_conf_sell | 190 | 43.7 | -0.0133 | -2560 | 57 |
| QQQ | div_consolidation_sell | 131 | 47.3 | 0.0033 | +426 | 41 |
| QQQ | div_trend_sell | 16 | 25.0 | -0.1031 | -1655 | 27 |
| BRN | t1_conf_sell | 1224 | 48.3 | 0.016 | +19972 | 73 |
| BRN | t3_conf_sell | 645 | 48.7 | -0.0206 | -13174 | 69 |
| BRN | div_consolidation_sell | 521 | 42.6 | -0.0084 | -2220 | 53 |
| BRN | div_trend_sell | 41 | 58.5 | 0.0695 | +3000 | 52 |

### 开腿时结构位置

| 标的 | 分组 | 对数 | 胜率% | avg_diff% | 净现金 | 平均持有bars |
|------|------|------|-------|-----------|--------|--------------|
| OKLO | center_dead | 170 | 51.2 | 0.0945 | +16522 | 69 |
| OKLO | above_zg_uptrend_leg | 135 | 45.9 | 0.0631 | +10192 | 47 |
| OKLO | below_zd_downtrend_leg | 89 | 56.2 | 0.2902 | +24226 | 106 |
| OKLO | inside_center_osc | 13 | 61.5 | 0.3572 | +4698 | 133 |
| QQQ | above_zg_uptrend_leg | 404 | 43.8 | -0.0082 | -3355 | 35 |
| QQQ | center_dead | 213 | 42.7 | -0.0145 | -3108 | 61 |
| QQQ | below_zd_downtrend_leg | 173 | 49.7 | 0.0055 | +965 | 74 |
| QQQ | inside_center_osc | 17 | 35.3 | 0.0536 | +911 | 63 |
| BRN | above_zg_uptrend_leg | 983 | 48.2 | 0.0139 | +14180 | 47 |
| BRN | center_dead | 804 | 47.8 | -0.0131 | -8621 | 66 |
| BRN | below_zd_downtrend_leg | 543 | 45.9 | -0.0086 | -4355 | 102 |
| BRN | inside_center_osc | 101 | 43.6 | 0.0634 | +6376 | 88 |

### 闭腿原因

| 标的 | 分组 | 对数 | 胜率% | avg_diff% | 净现金 | 平均持有bars |
|------|------|------|-------|-----------|--------|--------------|
| OKLO | t5_div_cons_buy | 132 | 50.8 | 0.2 | +27304 | 86 |
| OKLO | t5_type1_conf_buy | 108 | 63.0 | 0.6155 | +64497 | 80 |
| OKLO | t6_pre_type3_buy | 90 | 34.4 | -0.619 | -55737 | 61 |
| OKLO | t7_hard_type3_buy | 40 | 47.5 | 0.1035 | +4297 | 13 |
| OKLO | forced_master_exit | 37 | 59.5 | 0.3717 | +15277 | 90 |
| QQQ | t6_pre_type3_buy | 231 | 32.0 | -0.0812 | -18876 | 36 |
| QQQ | t5_div_cons_buy | 183 | 45.4 | 0.0004 | +140 | 83 |
| QQQ | t5_type1_conf_buy | 163 | 55.8 | 0.1026 | +16709 | 56 |
| QQQ | t7_hard_type3_buy | 162 | 51.9 | -0.0023 | -358 | 14 |
| QQQ | forced_master_exit | 68 | 41.2 | -0.0325 | -2201 | 91 |
| BRN | t5_div_cons_buy | 661 | 49.5 | 0.0609 | +40467 | 102 |
| BRN | t6_pre_type3_buy | 626 | 30.2 | -0.1915 | -120490 | 53 |
| BRN | t5_type1_conf_buy | 580 | 58.1 | 0.1322 | +79166 | 70 |
| BRN | t7_hard_type3_buy | 374 | 56.7 | 0.0222 | +8654 | 16 |
| BRN | forced_master_exit | 190 | 45.3 | -0.0026 | -218 | 84 |

## 3b. O1v（master 不变的隔离对照） — REV 腿逐笔聚合

### 开腿触发

| 标的 | 分组 | 对数 | 胜率% | avg_diff% | 净现金 | 平均持有bars |
|------|------|------|-------|-----------|--------|--------------|
| OKLO | t1_conf_sell | 436 | 49.3 | 0.0868 | +55502 | 90 |
| OKLO | t3_conf_sell | 234 | 50.4 | -0.1123 | -10066 | 85 |
| OKLO | div_consolidation_sell | 233 | 42.5 | 0.0335 | +8404 | 56 |
| OKLO | div_trend_sell | 14 | 50.0 | -0.5892 | -13741 | 124 |
| QQQ | t1_conf_sell | 1033 | 46.4 | -0.0003 | +201 | 57 |
| QQQ | t3_conf_sell | 484 | 50.6 | -0.0149 | -7009 | 60 |
| QQQ | div_consolidation_sell | 362 | 43.9 | -0.0181 | -6281 | 45 |
| QQQ | div_trend_sell | 34 | 50.0 | -0.0235 | -838 | 52 |
| BRN | t1_conf_sell | 2783 | 47.1 | 0.0022 | +1651 | 78 |
| BRN | t3_conf_sell | 1466 | 48.7 | -0.0344 | -45076 | 75 |
| BRN | div_consolidation_sell | 1202 | 42.3 | 0.0033 | -1203 | 55 |
| BRN | div_trend_sell | 87 | 48.3 | 0.0427 | +3196 | 72 |

### 开腿时结构位置

| 标的 | 分组 | 对数 | 胜率% | avg_diff% | 净现金 | 平均持有bars |
|------|------|------|-------|-----------|--------|--------------|
| OKLO | above_zg_uptrend_leg | 352 | 47.4 | 0.1274 | +53442 | 58 |
| OKLO | center_dead | 317 | 47.6 | -0.0257 | +6539 | 80 |
| OKLO | below_zd_downtrend_leg | 219 | 49.3 | -0.0769 | -10350 | 117 |
| OKLO | inside_center_osc | 29 | 44.8 | -0.3007 | -9533 | 97 |
| QQQ | above_zg_uptrend_leg | 907 | 45.4 | -0.0019 | -1579 | 37 |
| QQQ | center_dead | 541 | 48.1 | -0.0242 | -12716 | 64 |
| QQQ | below_zd_downtrend_leg | 411 | 50.6 | -0.0038 | -1245 | 85 |
| QQQ | inside_center_osc | 54 | 37.0 | 0.0284 | +1613 | 68 |
| BRN | above_zg_uptrend_leg | 2290 | 46.5 | 0.0049 | +11636 | 50 |
| BRN | center_dead | 1786 | 46.2 | -0.0308 | -53737 | 73 |
| BRN | below_zd_downtrend_leg | 1272 | 47.6 | 0.0138 | +11358 | 109 |
| BRN | inside_center_osc | 190 | 41.6 | -0.0565 | -10689 | 94 |

### 闭腿原因

| 标的 | 分组 | 对数 | 胜率% | avg_diff% | 净现金 | 平均持有bars |
|------|------|------|-------|-----------|--------|--------------|
| OKLO | t5_div_cons_buy | 302 | 50.3 | 0.2853 | +95330 | 105 |
| OKLO | t5_type1_conf_buy | 242 | 59.1 | 0.7043 | +176470 | 98 |
| OKLO | t6_pre_type3_buy | 232 | 28.9 | -1.033 | -224720 | 67 |
| OKLO | t7_hard_type3_buy | 123 | 54.5 | -0.0705 | -11232 | 14 |
| OKLO | forced_master_exit | 18 | 55.6 | 0.1589 | +4251 | 84 |
| QQQ | t6_pre_type3_buy | 542 | 30.1 | -0.1174 | -63697 | 44 |
| QQQ | t5_type1_conf_buy | 487 | 57.7 | 0.0699 | +34045 | 63 |
| QQQ | t5_div_cons_buy | 478 | 50.4 | 0.0207 | +10893 | 90 |
| QQQ | t7_hard_type3_buy | 370 | 50.8 | 0.001 | +263 | 14 |
| QQQ | forced_master_exit | 36 | 75.0 | 0.1249 | +4570 | 98 |
| BRN | t5_div_cons_buy | 1618 | 48.1 | 0.0509 | +77555 | 106 |
| BRN | t6_pre_type3_buy | 1516 | 30.1 | -0.205 | -302034 | 58 |
| BRN | t5_type1_conf_buy | 1475 | 56.5 | 0.1247 | +177220 | 80 |
| BRN | t7_hard_type3_buy | 855 | 55.3 | 0.0103 | +7718 | 16 |
| BRN | forced_master_exit | 74 | 44.6 | -0.0144 | -1890 | 102 |

## 4. REV 腿亏损集中度

| 标的 | 变体 | 对数 | 净 | 毛亏 | 毛盈 | 亏笔 | 最差10笔占毛亏% | 最差30笔占毛亏% | 中位持有bars |
|------|------|------|----|------|------|------|------|------|------|
| OKLO | O1 | 407 | +55639 | -204128 | +259767 | 191 | 26.5 | 52.0 | 41 |
| OKLO | O1v | 917 | +40100 | -635653 | +675753 | 466 | 16.7 | 33.6 | — |
| QQQ | O1 | 807 | -4587 | -67867 | +63280 | 439 | 13.5 | 29.7 | 26 |
| QQQ | O1v | 1913 | -13926 | -185521 | +171594 | 993 | 10.3 | 20.8 | — |
| BRN | O1 | 2431 | +7579 | -413994 | +421573 | 1205 | 10.1 | 19.1 | 36 |
| BRN | O1v | 5538 | -41431 | -1032393 | +990961 | 2806 | 6.9 | 12.0 | — |

## 5. 域腿 vs REV 腿：T1 CLOSE_OSC 截断证据（O1v 隔离）

| 标的 | osc 闭腿类 | 对数 | 胜率% | avg_diff% | 净现金 |
|------|-----------|------|-------|-----------|--------|
| OKLO | 被 REV 开腿强制截断 | 25 | 20.0 | -1.8802 | -44590 |
| OKLO | 正常闭腿(ZD/死亡/强平) | 31 | 77.4 | 0.8542 | +25656 |
| QQQ | 被 REV 开腿强制截断 | 16 | 31.2 | -0.2607 | -4114 |
| QQQ | 正常闭腿(ZD/死亡/强平) | 62 | 66.1 | 0.0616 | +3896 |
| BRN | 被 REV 开腿强制截断 | 57 | 29.8 | -0.3143 | -16334 |
| BRN | 正常闭腿(ZD/死亡/强平) | 146 | 68.5 | 0.16 | +21440 |

## 6. 41课门：反事实为空集 + 恒开机制

O2 与 O1 逐位相同（门拒=0，门开率 100%）→ **门没有关掉任何一条 REV 腿，反事实集为空**。恒开机制（FatigueMonitor 重放）：

| 标的 | ladder | 证据集非空时间占比% | 卖侧证据事件数 | 向上settle清空次数 |
|------|--------|---------------------|----------------|--------------------|
| OKLO | segment | 99.95 | 9480 | 0 |
| OKLO | move(L1) | 99.42 | 1253 | 96 |
| OKLO | recL2 | 68.61 | 15 | 8 |
| QQQ | segment | 99.98 | 22505 | 0 |
| QQQ | move(L1) | 98.83 | 2644 | 212 |
| QQQ | recL2 | 67.96 | 19 | 16 |
| QQQ | recL3 | 68.7 | 1 | 1 |
| BRN | segment | 99.99 | 54359 | 0 |
| BRN | move(L1) | 99.2 | 6553 | 503 |
| BRN | recL2 | 66.14 | 72 | 38 |
| BRN | recL3 | 81.42 | 8 | 1 |

## 7. O1 REV 腿逐笔磁带样本（最大亏损 12 + 最大盈利 6）

### OKLO

| ladder | 开bar | 开价 | 闭bar | 闭价 | 持有 | 盈亏 | 开腿触发 | 结构位置 | 闭腿原因 | 强平 |
|--------|-------|------|-------|------|------|------|----------|----------|----------|------|
| segment | 108682 | 19.08 | 108702 | 20.61 | 20 | -8270 | t1_conf_sell | above_zg_uptrend_leg | t6_pre_type3_buy |  |
| segment | 432574 | 71.91 | 432817 | 77.3201 | 243 | -7452 | t3_conf_sell | center_dead | t6_pre_type3_buy |  |
| segment | 172842 | 22.33 | 172956 | 23.67 | 114 | -6124 | div_consolidation_sell | center_dead | t5_type1_conf_buy |  |
| segment | 109992 | 20.01 | 110068 | 21.14 | 76 | -6108 | t1_conf_sell | below_zd_downtrend_leg | t5_div_cons_buy |  |
| segment | 413328 | 49.27 | 413707 | 51.669 | 379 | -4837 | t1_conf_sell | below_zd_downtrend_leg | t5_div_cons_buy |  |
| segment | 244214 | 63.68 | 244338 | 66.7121 | 124 | -4758 | div_consolidation_sell | above_zg_uptrend_leg | t5_div_cons_buy |  |
| segment | 324054 | 100.59 | 324085 | 105.11 | 31 | -4624 | t1_conf_sell | below_zd_downtrend_leg | t5_div_cons_buy |  |
| segment | 172400 | 21.71 | 172597 | 22.62 | 197 | -4159 | t3_conf_sell | center_dead | t6_pre_type3_buy |  |
| segment | 225573 | 58.14 | 225688 | 60.39 | 115 | -3889 | t1_conf_sell | below_zd_downtrend_leg | t6_pre_type3_buy |  |
| segment | 150448 | 34.37 | 150714 | 35.62 | 266 | -3869 | t1_conf_sell | below_zd_downtrend_leg | t6_pre_type3_buy |  |
| segment | 317562 | 108.8959 | 317632 | 112.905 | 70 | -3481 | t3_conf_sell | center_dead | t5_type1_conf_buy |  |
| segment | 199487 | 31.21 | 199616 | 32.21 | 129 | -3115 | t3_conf_sell | center_dead | t5_type1_conf_buy |  |
| segment | 317424 | 116.69 | 317488 | 110.0665 | 64 | +5750 | t1_conf_sell | below_zd_downtrend_leg | t5_div_cons_buy |  |
| segment | 27271 | 8.02 | 27534 | 7.48 | 263 | +6818 | div_consolidation_sell | center_dead | t5_type1_conf_buy |  |
| segment | 25270 | 7.74 | 25417 | 7.175 | 147 | +7001 | t1_conf_sell | below_zd_downtrend_leg | t5_type1_conf_buy |  |
| segment | 148707 | 37.785 | 148747 | 34.87 | 40 | +7519 | t3_conf_sell | center_dead | t5_type1_conf_buy |  |
| segment | 341403 | 84.96 | 341768 | 78.4111 | 365 | +7967 | t1_conf_sell | above_zg_uptrend_leg | t5_div_cons_buy |  |
| segment | 146717 | 45.8 | 146899 | 41.935 | 182 | +8591 | t1_conf_sell | above_zg_uptrend_leg | t5_div_cons_buy |  |

### QQQ

| ladder | 开bar | 开价 | 闭bar | 闭价 | 持有 | 盈亏 | 开腿触发 | 结构位置 | 闭腿原因 | 强平 |
|--------|-------|------|-------|------|------|------|----------|----------|----------|------|
| segment | 471825 | 433.8 | 471887 | 439.09 | 62 | -1240 | t1_conf_sell | above_zg_uptrend_leg | t6_pre_type3_buy |  |
| segment | 491317 | 486.16 | 491420 | 491.36 | 103 | -1064 | t3_conf_sell | center_dead | t5_type1_conf_buy |  |
| segment | 612324 | 584.6 | 612409 | 590.45 | 85 | -1003 | t1_conf_sell | below_zd_downtrend_leg | t5_div_cons_buy |  |
| segment | 72259 | 318.4 | 72450 | 321.53 | 191 | -990 | t1_conf_sell | above_zg_uptrend_leg | t6_pre_type3_buy |  |
| segment | 23338 | 300.62 | 23419 | 303.35 | 81 | -906 | t1_conf_sell | below_zd_downtrend_leg | t6_pre_type3_buy |  |
| segment | 34102 | 294.22 | 34221 | 296.87 | 119 | -901 | t1_conf_sell | below_zd_downtrend_leg | t6_pre_type3_buy |  |
| segment | 225378 | 423.34 | 225466 | 426.96 | 88 | -861 | t3_conf_sell | center_dead | t5_div_cons_buy |  |
| segment | 123248 | 374.43 | 123510 | 377.54 | 262 | -836 | t3_conf_sell | center_dead | t6_pre_type3_buy |  |
| segment | 246713 | 439.05 | 246872 | 442.41 | 159 | -765 | t1_conf_sell | below_zd_downtrend_leg | t6_pre_type3_buy |  |
| segment | 165602 | 364.58 | 165633 | 366.89 | 31 | -625 | t3_conf_sell | center_dead | t5_type1_conf_buy |  |
| segment | 577093 | 596.38 | 577281 | 600.1 | 188 | -624 | t1_conf_sell | above_zg_uptrend_leg | t6_pre_type3_buy |  |
| segment | 92230 | 353.53 | 92312 | 355.62 | 82 | -593 | div_consolidation_sell | below_zd_downtrend_leg | t5_div_cons_buy |  |
| segment | 416117 | 512.04 | 416226 | 506.65 | 109 | +1054 | t3_conf_sell | center_dead | t5_type1_conf_buy |  |
| segment | 319082 | 483.5 | 319198 | 477.74 | 116 | +1192 | t1_conf_sell | below_zd_downtrend_leg | t5_type1_conf_buy |  |
| segment | 29826 | 294.44 | 30030 | 290.65 | 204 | +1286 | t3_conf_sell | center_dead | t5_type1_conf_buy |  |
| segment | 513008 | 533.99 | 513304 | 526.99 | 296 | +1311 | div_consolidation_sell | inside_center_osc | t5_type1_conf_buy |  |
| segment | 126645 | 373.64 | 126845 | 368.35 | 200 | +1425 | t1_conf_sell | above_zg_uptrend_leg | t5_type1_conf_buy |  |
| segment | 668322 | 607.09 | 668585 | 593.95 | 263 | +2163 | t1_conf_sell | below_zd_downtrend_leg | t5_type1_conf_buy |  |

### BRN

| ladder | 开bar | 开价 | 闭bar | 闭价 | 持有 | 盈亏 | 开腿触发 | 结构位置 | 闭腿原因 | 强平 |
|--------|-------|------|-------|------|------|------|----------|----------|----------|------|
| segment | 2040574 | 60.64 | 2040676 | 65.39 | 102 | -8026 | t1_conf_sell | above_zg_uptrend_leg | t6_pre_type3_buy |  |
| segment | 385840 | 33.68 | 385923 | 36.67 | 83 | -6558 | div_consolidation_sell | center_dead | t5_type1_conf_buy |  |
| segment | 605486 | 40.35 | 605769 | 42.8 | 283 | -6049 | t1_conf_sell | below_zd_downtrend_leg | t6_pre_type3_buy |  |
| segment | 202935 | 58.31 | 203050 | 60.78 | 115 | -4221 | t3_conf_sell | center_dead | t6_pre_type3_buy |  |
| segment | 426673 | 17.59 | 426705 | 18.3 | 32 | -4053 | div_consolidation_sell | above_zg_uptrend_leg | t5_div_cons_buy |  |
| segment | 2403412 | 104.15 | 2403447 | 107.25 | 35 | -2929 | t3_conf_sell | below_zd_downtrend_leg | t5_type1_conf_buy |  |
| segment | 600258 | 38.84 | 600394 | 39.92 | 136 | -2854 | t1_conf_sell | below_zd_downtrend_leg | t6_pre_type3_buy |  |
| segment | 426713 | 18.8 | 426726 | 19.24 | 13 | -2511 | t1_conf_sell | above_zg_uptrend_leg | t6_pre_type3_buy |  |
| segment | 447620 | 30.1 | 447702 | 30.82 | 82 | -2392 | div_consolidation_sell | center_dead | t5_div_cons_buy |  |
| segment | 431972 | 22.98 | 432032 | 23.5 | 60 | -2329 | t1_conf_sell | below_zd_downtrend_leg | t5_div_cons_buy |  |
| segment | 142673 | 60.74 | 142772 | 62.14 | 99 | -2289 | div_consolidation_sell | center_dead | t6_pre_type3_buy |  |
| segment | 424343 | 26.41 | 424415 | 26.99 | 72 | -2116 | t3_conf_sell | center_dead | t5_type1_conf_buy |  |
| segment | 2364616 | 98.86 | 2364718 | 95.3 | 102 | +3677 | t1_conf_sell | above_zg_uptrend_leg | t5_div_cons_buy |  |
| segment | 1200259 | 93.52 | 1200492 | 89.8 | 233 | +4054 | t1_conf_sell | above_zg_uptrend_leg | t6_pre_type3_buy |  |
| segment | 1893965 | 75.9 | 1894107 | 72.61 | 142 | +4331 | t3_conf_sell | center_dead | t5_type1_conf_buy |  |
| segment | 1331751 | 83.5 | 1331964 | 79.93 | 213 | +4353 | t1_conf_sell | inside_center_osc | t5_div_cons_buy |  |
| segment | 2061190 | 61.6 | 2061263 | 58.77 | 73 | +4618 | div_consolidation_sell | center_dead | t5_type1_conf_buy |  |
| segment | 2343848 | 103.56 | 2343957 | 98.65 | 109 | +4734 | t1_conf_sell | above_zg_uptrend_leg | t5_div_cons_buy |  |

## 8. 假设检验判决（任务 §5）

| 假设 | 判决 | 证据 |
|------|------|------|
| a) 趋势段反向操盘，41课门应阻止但没有 | **部分成立（机制精确化）** | REV 开腿 33-50% 发生在价格位于中枢上方（above_zg 上升中段）；41课门完全失效——O1≡O2 逐位相同、门拒=0，机制见 §6：segment 层衰竭证据集 99.95-99.99% 时间非空（卖侧证据事件 9.5K-54K 个 vs up-move settle 清空仅 96-503 次），门的时效设计在该分辨率下不构成约束。但『趋势段反向』的最大亏损通道在 master 层（见 e），voice 层的趋势段开腿亏损由 t6 槽承载 |
| b) 闭腿信号配对问题 | **成立（t6 槽）** | t6_pre_type3_buy（candidate type3 买预逃逸）三标的胜率一致 ≈29-32%、净额大负（O1v：OKLO -224.7K / QQQ -63.7K / BRN -302.0K）——REV 卖出后价格向上离开中枢被迫高位回补，与此前 sell_any/buy_any 配对问题同构（卖低买高）；t5_type1_conf_buy（38课设计剧本）反而是正槽（胜率 56-63%）。逃逸通道本身没错（不逃亏更多），错的是开腿后走到需要逃逸的频率太高 = 开腿判定（段终结）在 1min segment 层假阳性过半 |
| c) 操作规模太大 | **否证** | rev frac = 1/N_sub 与 osc 腿同口径；stock 模式 cost>0 阶段 REV 是虚拟价差收集器（不动 total_shares，不影响真实暴露）；O3（结构规模缩腿至 244/607/1663 对）Δ 仍 -654.9/-79.0/-439.1——规模轴改不了符号 |
| d) 标的选择偏差 | **否证（方向）/ 成立（幅度）** | 三标的机制完全同构（暴露比都 ≈0.50、t6 槽胜率都 ≈30%、门都恒开）→ 不是标的问题；但 Δ 幅度 ∝ 趋势强度（OKLO -626 > BRN -450 > QQQ -80），暴露缺口在强趋势标的上代价最大。不存在『中性标的上 REV 转正』的证据——QQQ（最弱趋势）rev 腿净额本身就是负的 |
| e) 其他：master 触发扩展（真正主因） | **成立** | rev_mode 捆绑把 master 出场扩展为段终结三触发：sell1 兼容出场仅占 1.4-1.8%，ext_type3_conf_sell 出场（高频割肉：平均持有 50-102 bar、平均 pnl -0.77/-0.10/-0.19%）+ ext_div_consolidation_sell（E10 引入的盘整背驰流量）把交易数放大 3.7-4.4×、暴露腰斩；空仓缺口错失复合 +308.9%（OKLO）/+44.9%（QQQ）/+209.0%（BRN）。O1v 隔离：master 恢复 sell1 后 Δ 从 -626/-80/-450 收敛到 -64/-41/-186 |

## 结果包（六要素）

**1. 结论**：O1 全负 = master 触发扩展砍暴露（OKLO -562.5pp[90%] / QQQ -39.0pp[49%] / BRN -263.6pp[59%]；暴露比 0.50/0.49/0.50，空仓错失复合 +309/+45/+209%）⊕ voice REV 腿自身负贡献（-63.7/-41.4/-186.2pp：osc 截断 + t6 预逃逸槽）。REV 腿配对现金（OKLO +55.6K/BRN +7.6K 为正）不是 Δ 的载体——亏的是没拿住的仓位，不是拿反的差价。亏损不集中（最差10笔仅占毛亏 7-27%），是结构性均匀失血，排除少数极端事件解释。

**2. 定义依据**：38课段终结三触发（confirmed type1 卖 ∨ div 卖侧 ∨ confirmed type3 卖，`LevelOperatingUnit.seg_end_trigger`）；其用于 master 出场时与 45课持股持币二元的级别相对性冲突——segment 层段终结事件密度（1min 数据上日均数十个）远高于 entry 级走势终结密度，把 entry 级持仓的出场判定下放到 segment 级 = 级别错配（与『走势方向代理陷阱』『PH settle 用途边界』同构：低级别信号越级驱动高级别操作）。41课门时效定义（up-move settle 清空）在该密度比下恒开。

**3. 边界条件**：(a) 本诊断在 1min/segment floor 上成立——更高 floor（move 级段终结）的事件密度低一个量级，master 扩展的暴露代价可能反转，未检验；(b) 强趋势标的（BH +87~+307%）上空仓代价被趋势放大——长期横盘/下行标的上同一机制的符号可能不同；(c) t6 槽若改为『预逃逸不回补、等 confirmed』（t7-only）REV 净额会变化，但逃逸延迟在真突破时亏更多——该消融未跑；(d) 结论对费率单调恶化（rev 腿 avg_diff ≈0，任何费率下 t6 槽更深）。

**4. 下游推论**：(i) 若要保留 REV 假设的任何残余，唯一可活的形态是 O1v 框架 + 关掉 t6 亏损源（开腿端收紧而非关腿端放宽——t5 剧本本身是正的）+ 不碰 osc（取消『先 CLOSE_OSC』动作或 REV 与 osc 槽位共存）；(ii) master 出场任何扩展必须先过暴露守恒检查（新触发集的出场频率 vs sell1 的倍数 = 暴露损失的先验上界）；(iii) 41课门若要有约束力，时效必须改为与被守卫层同分辨率的清空事件（如 u 层新中枢生成），否则证据集恒非空=门恒开=死代码；(iv) E10 盘整背驰进 div_events 的直接消费者（seg_end_trigger 的 div 分量）是 master 出场 churn 的两大来源之一——盘整背驰作为『段终结证据』在 segment 层假阳性过半。

**5. 谱系引用**：532号（段终结判定单轴性分离，生成态——本诊断是其数据基底：master 轴与 voice 轴的亏损机制已定量分离）；E6（bar级短差有害）/E10（盘整背驰可见）；『走势方向代理陷阱』『PH settle用途边界』（低级别信号越级驱动高级别操作的同构先例）；project_organic_fugue_implementation（REV 否证记录）。

**6. 影响声明**：新增 `analysis/rev_leg_diagnostic.py`（只读诊断，不触碰 organic_fugue.py / 回测缓存）+ 本报告 + `data_cache/rev_leg_diag_{OKLO,QQQ,BRN}.json`（逐笔分类数据）。不改变任何定义、引擎、回测结论；为 532号谱系候选提供 L2 证据。
