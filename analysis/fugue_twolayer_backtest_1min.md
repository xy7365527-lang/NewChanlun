# 两层分离回测 — 1分钟数据

## 架构

两层分离 + 5状态 FSM + 三级别 PH 分层：

| 层 | 信号来源 | 决定什么 |
|---|---------|---------|
| 选股层 | 最高有效递归级别 Move 方向 | 品种方向（long/short/neutral） |
| 操盘层 | L1 Move 类型+方向 | 操作策略（只多/只空/双向半仓） |
| PH L2 | L1 走势端点 PH rank-1 settle | 止损触发 |
| PH L1 | L1 线段端点 PH | 降成本/加仓比例 |
| PH L0 | 1min close PH rank-1 settle | 进场门控 |

**两层组合规则**：入场需两层都同意，矛盾时不操作。

## QQQ

- 数据：**728,030** bars (1min)
- 价格：268.82 → 738.28
- Buy-and-hold: **+174.64%**
- 回测耗时：366.7s

### 两层状态统计

| 指标 | 值 |
|------|-----|
| 操盘层模式切换 | 797 |
| 选股层偏向切换 | 15 |
| 两层矛盾（不操作） | 243,146 bars |

### PH 分层统计

| 级别 | 更新次数 | settle总数 | rank-1 settle |
|------|---------|-----------|---------------|
| L0 | 728,030 (每bar) | 350,072 | 3962 |
| L1 | 6,119 | 5,804 | 286 |
| L2 | 389 | 190 | 25 |
| **L2 方向翻转** | — | — | **14** |

### 总体指标

| 指标 | 值 |
|------|-----|
| 交易数 | 44 (多36/空8) |
| 胜率 | 52.3% |
| 平均收益 | +0.410% |
| 复利累计 | +18.47% |
| 最大回撤 | -8.07% |
| 平均持仓 | 3,880 bars |
| 有加仓的交易 | 13/44 |
| 有降成本的交易 | 34/44 |
| 达到本金回收 | 0/44 |

### 按操盘层模式分组

| 模式 | 交易数 | 胜率 | 平均PnL | 复利% |
|------|--------|------|---------|------|
| BOTH_HALF | 20 | 50% | +0.789% | +16.27% |
| LONG_ONLY | 17 | 76% | +0.769% | +13.70% |
| SHORT_ONLY | 7 | 0% | -1.545% | -10.39% |

### 交易明细

| # | 方向 | 入场 | 出场 | bars | PnL% | 模式 | 偏向 | 加仓 | 短差 | PW | 退出原因 |
|---|------|------|------|------|------|------|------|------|------|----|---------| 
| 1 | 空 | 266.33@959 | 269.97@3332 | 2,373 | -1.68 | SHORT_ONLY | SHORT | 0 | 5 | 否 | stock_bias_flip_long |
| 2 | 多 | 270.73@3483 | 272.89@4833 | 1,350 | +0.56 | BOTH_HALF | LONG | 1 | 1 | 否 | bsp_invalidate |
| 3 | 多 | 275.17@5184 | 282.84@10516 | 5,332 | +2.79 | LONG_ONLY | LONG | 0 | 4 | 否 | trading_mode_flip_short |
| 4 | 多 | 292.45@13661 | 292.79@16013 | 2,352 | +0.12 | LONG_ONLY | LONG | 0 | 2 | 否 | trading_mode_flip_short |
| 5 | 多 | 298.24@16909 | 305.89@20247 | 3,338 | +3.84 | BOTH_HALF | LONG | 0 | 4 | 否 | trading_mode_flip_short |
| 6 | 空 | 287.37@39206 | 295.08@40529 | 1,323 | -2.68 | SHORT_ONLY | SHORT | 0 | 0 | 否 | trading_mode_flip_long |
| 7 | 多 | 315.00@50895 | 318.10@55783 | 4,888 | +1.42 | BOTH_HALF | LONG | 0 | 5 | 否 | trading_mode_flip_short |
| 8 | 多 | 319.43@67965 | 316.08@72003 | 4,038 | -0.71 | BOTH_HALF | LONG | 1 | 4 | 否 | trading_mode_flip_short |
| 9 | 多 | 327.57@77960 | 330.63@82996 | 5,036 | +1.04 | LONG_ONLY | LONG | 0 | 4 | 否 | trading_mode_flip_short |
| 10 | 多 | 340.69@84038 | 347.42@86834 | 2,796 | +1.98 | LONG_ONLY | LONG | 0 | 0 | 否 | trading_mode_flip_short |
| 11 | 多 | 357.62@93055 | 366.89@97689 | 4,634 | +2.59 | LONG_ONLY | LONG | 0 | 4 | 否 | trading_mode_flip_short |
| 12 | 多 | 377.16@109965 | 378.73@114709 | 4,744 | +0.54 | LONG_ONLY | LONG | 0 | 3 | 否 | trading_mode_flip_short |
| 13 | 空 | 357.63@131258 | 357.63@131990 | 732 | -0.01 | SHORT_ONLY | SHORT | 1 | 0 | 否 | bsp_invalidate |
| 14 | 空 | 351.34@168860 | 349.96@174956 | 6,096 | -0.75 | SHORT_ONLY | SHORT | 2 | 6 | 否 | trading_mode_flip_long |
| 15 | 多 | 395.32@196835 | 403.50@207269 | 10,434 | +2.38 | LONG_ONLY | LONG | 0 | 11 | 否 | trading_mode_flip_short |
| 16 | 多 | 413.20@216485 | 425.79@221363 | 4,878 | +3.67 | BOTH_HALF | LONG | 0 | 5 | 否 | trading_mode_flip_short |
| 17 | 多 | 435.16@229308 | 428.01@231018 | 1,710 | -1.64 | BOTH_HALF | LONG | 0 | 0 | 否 | trading_mode_flip_short |
| 18 | 多 | 439.07@236272 | 434.09@239791 | 3,519 | -0.62 | BOTH_HALF | LONG | 1 | 3 | 否 | trading_mode_flip_short |
| 19 | 多 | 441.11@240883 | 439.85@243498 | 2,615 | +0.64 | BOTH_HALF | LONG | 1 | 2 | 否 | bsp_invalidate |
| 20 | 多 | 448.45@252489 | 445.39@254428 | 1,939 | -0.68 | LONG_ONLY | LONG | 0 | 1 | 否 | trading_mode_flip_short |
| 21 | 空 | 420.71@268939 | 422.16@269250 | 311 | -0.34 | SHORT_ONLY | SHORT | 1 | 0 | 否 | bsp_invalidate |
| 22 | 空 | 416.61@269419 | 420.63@271022 | 1,603 | -1.16 | SHORT_ONLY | SHORT | 1 | 0 | 否 | trading_mode_flip_long |
| 23 | 多 | 463.92@296108 | 478.61@305293 | 9,185 | +3.77 | BOTH_HALF | LONG | 0 | 9 | 否 | trading_mode_flip_short |
| 24 | 多 | 488.76@310992 | 489.64@317997 | 7,005 | +0.34 | LONG_ONLY | LONG | 1 | 7 | 否 | trading_mode_flip_short |
| 25 | 多 | 505.07@385905 | 512.68@390373 | 4,468 | +1.74 | LONG_ONLY | LONG | 0 | 2 | 否 | trading_mode_flip_short |
| 26 | 多 | 519.99@400611 | 525.07@403960 | 3,349 | +1.49 | LONG_ONLY | LONG | 0 | 3 | 否 | trading_mode_flip_short |
| 27 | 多 | 533.59@407121 | 516.47@409317 | 2,196 | -3.21 | LONG_ONLY | LONG | 0 | 1 | 否 | trading_mode_flip_short |
| 28 | 多 | 540.63@440493 | 537.77@443146 | 2,653 | -0.23 | LONG_ONLY | LONG | 0 | 3 | 否 | trading_mode_flip_short |
| 29 | 空 | 402.75@470586 | 423.64@471017 | 431 | -4.19 | SHORT_ONLY | SHORT | 1 | 0 | 否 | bsp_invalidate |
| 30 | 多 | 541.30@519792 | 550.37@524569 | 4,777 | +2.45 | LONG_ONLY | LONG | 0 | 6 | 否 | trading_mode_flip_short |
| 31 | 多 | 556.27@525233 | 556.21@525240 | 7 | -0.01 | BOTH_HALF | LONG | 0 | 0 | 否 | bsp_invalidate |
| 32 | 多 | 560.78@532889 | 561.02@535494 | 2,605 | +0.09 | LONG_ONLY | LONG | 0 | 2 | 否 | trading_mode_flip_short |
| 33 | 多 | 569.53@538459 | 563.52@543534 | 5,075 | -1.03 | LONG_ONLY | LONG | 0 | 4 | 否 | trading_mode_flip_short |
| 34 | 多 | 580.37@549098 | 577.53@551637 | 2,539 | -0.38 | BOTH_HALF | LONG | 0 | 3 | 否 | trading_mode_flip_short |
| 35 | 多 | 583.50@566173 | 596.54@571262 | 5,089 | +2.67 | BOTH_HALF | LONG | 0 | 4 | 否 | trading_mode_flip_short |
| 36 | 多 | 598.29@572438 | 598.45@572468 | 30 | +0.03 | BOTH_HALF | LONG | 0 | 0 | 否 | bsp_invalidate |
| 37 | 多 | 602.84@573742 | 595.32@575196 | 1,454 | -1.25 | BOTH_HALF | LONG | 0 | 0 | 否 | trading_mode_flip_short |
| 38 | 多 | 605.00@579856 | 601.49@588515 | 8,659 | -0.49 | BOTH_HALF | LONG | 2 | 6 | 否 | bsp_invalidate |
| 39 | 多 | 616.84@594460 | 618.00@601392 | 6,932 | +0.36 | BOTH_HALF | LONG | 0 | 4 | 否 | trading_mode_flip_short |
| 40 | 多 | 637.00@648663 | 629.63@650319 | 1,656 | -1.07 | BOTH_HALF | LONG | 1 | 2 | 否 | trading_mode_flip_short |
| 41 | 空 | 580.27@680908 | 581.78@687759 | 6,851 | -1.89 | BOTH_HALF | SHORT | 4 | 5 | 否 | trading_mode_flip_long |
| 42 | 多 | 662.75@702722 | 712.60@714315 | 11,593 | +8.92 | BOTH_HALF | LONG | 0 | 12 | 否 | trading_mode_flip_short |
| 43 | 多 | 719.08@715449 | 701.65@718141 | 2,692 | -2.03 | BOTH_HALF | LONG | 0 | 3 | 否 | trading_mode_flip_short |
| 44 | 多 | 733.61@722601 | 738.28@728029 | 5,428 | +0.70 | LONG_ONLY | LONG | 0 | 6 | 否 | eod_close |

### FSM 状态分布（按 bar 数）

| 状态 | Bars | 占比 |
|------|------|------|
| SCANNING | 557,315 | 76.6% |
| POSITION_OPEN | 108,717 | 14.9% |
| COST_REDUCING | 61,998 | 8.5% |
| PRINCIPAL_WITHDRAWN | 0 | 0.0% |
| STOPPED_OUT | 0 | 0.0% |

<details><summary>事件流（413条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 959 | 266.33 | POSITION_OPEN | ENTRY_SHORT@266.33 mode=SHORT_ONLY bias=SHORT scale=1.0 |
| 991 | 265.84 | COST_REDUCING | TRIM@265.84 shares=38.8 L1ratio=0.103 |
| 1,188 | 264.79 | POSITION_OPEN | CLOSE_DIFF@264.79 profit=-40.71 |
| 1,324 | 265.70 | COST_REDUCING | TRIM@265.70 shares=38.8 L1ratio=0.103 |
| 1,705 | 265.70 | POSITION_OPEN | CLOSE_DIFF@265.70 profit=0.00 |
| 1,813 | 266.44 | COST_REDUCING | TRIM@266.44 shares=246.4 L1ratio=0.656 |
| 2,231 | 262.90 | POSITION_OPEN | CLOSE_DIFF@262.90 profit=-872.27 |
| 2,303 | 262.17 | COST_REDUCING | TRIM@262.17 shares=345.9 L1ratio=0.921 |
| 2,357 | 261.78 | POSITION_OPEN | CLOSE_DIFF@261.78 profit=-134.89 |
| 2,711 | 263.52 | COST_REDUCING | TRIM@263.52 shares=312.6 L1ratio=0.833 |
| 2,805 | 264.53 | POSITION_OPEN | CLOSE_DIFF@264.53 profit=315.74 |
| 2,899 | 266.69 | COST_REDUCING | TRIM@266.69 shares=301.7 L1ratio=0.803 |
| 3,332 | 269.97 | SCANNING | STOP:stock_bias_flip_long@269.97 pnl=-1.68% |
| 3,483 | 270.73 | POSITION_OPEN | ENTRY_LONG@270.73 mode=BOTH_HALF bias=LONG scale=0.5 |
| 4,098 | 270.50 | COST_REDUCING | TRIM@270.50 shares=129.1 L1ratio=0.699 |
| 4,376 | 271.50 | POSITION_OPEN | CLOSE_DIFF@271.50 profit=-129.14 |
| 4,821 | 272.76 | POSITION_OPEN | ADD_POS@272.76 +83.3 L1ratio=0.451 |
| 4,833 | 272.89 | SCANNING | STOP:bsp_invalidate@272.89 pnl=+0.56% |
| 5,184 | 275.17 | POSITION_OPEN | ENTRY_LONG@275.17 mode=LONG_ONLY bias=LONG scale=1.0 |
| 6,003 | 277.71 | COST_REDUCING | TRIM@277.71 shares=318.0 L1ratio=0.875 |
| 6,239 | 279.25 | POSITION_OPEN | CLOSE_DIFF@279.25 profit=-489.74 |
| 6,717 | 277.20 | COST_REDUCING | TRIM@277.20 shares=318.1 L1ratio=0.875 |
| 6,819 | 277.51 | POSITION_OPEN | CLOSE_DIFF@277.51 profit=-98.63 |
| 8,024 | 281.60 | COST_REDUCING | TRIM@281.60 shares=341.3 L1ratio=0.939 |
| 8,318 | 282.83 | POSITION_OPEN | CLOSE_DIFF@282.83 profit=-419.79 |
| 9,190 | 275.99 | COST_REDUCING | TRIM@275.99 shares=291.4 L1ratio=0.802 |
| 9,483 | 276.62 | POSITION_OPEN | CLOSE_DIFF@276.62 profit=-183.61 |
| 10,516 | 282.84 | SCANNING | STOP:trading_mode_flip_short@282.84 pnl=+2.79% |
| 13,661 | 292.45 | POSITION_OPEN | ENTRY_LONG@292.45 mode=LONG_ONLY bias=LONG scale=1.0 |
| 14,704 | 292.56 | COST_REDUCING | TRIM@292.56 shares=287.8 L1ratio=0.842 |
| 14,811 | 292.74 | POSITION_OPEN | CLOSE_DIFF@292.74 profit=-51.81 |
| 15,620 | 288.77 | COST_REDUCING | TRIM@288.77 shares=275.6 L1ratio=0.806 |
| 15,797 | 291.27 | POSITION_OPEN | CLOSE_DIFF@291.27 profit=-688.93 |
| 16,013 | 292.79 | SCANNING | STOP:trading_mode_flip_short@292.79 pnl=+0.12% |
| 16,909 | 298.24 | POSITION_OPEN | ENTRY_LONG@298.24 mode=BOTH_HALF bias=LONG scale=0.5 |
| 17,975 | 309.05 | COST_REDUCING | TRIM@309.05 shares=135.4 L1ratio=0.808 |
| 18,466 | 311.10 | POSITION_OPEN | CLOSE_DIFF@311.10 profit=-277.64 |
| 18,728 | 306.41 | COST_REDUCING | TRIM@306.41 shares=162.0 L1ratio=0.966 |
| 19,261 | 303.36 | POSITION_OPEN | CLOSE_DIFF@303.36 profit=494.03 |
| 19,318 | 304.98 | COST_REDUCING | TRIM@304.98 shares=135.4 L1ratio=0.808 |
| 19,531 | 303.91 | POSITION_OPEN | CLOSE_DIFF@303.91 profit=144.24 |
| 19,593 | 303.51 | COST_REDUCING | TRIM@303.51 shares=140.5 L1ratio=0.838 |
| 19,821 | 304.12 | POSITION_OPEN | CLOSE_DIFF@304.12 profit=-85.72 |
| 20,082 | 303.78 | COST_REDUCING | TRIM@303.78 shares=140.7 L1ratio=0.839 |
| 20,247 | 305.89 | SCANNING | STOP:trading_mode_flip_short@305.89 pnl=+3.84% |
| 39,206 | 287.37 | POSITION_OPEN | ENTRY_SHORT@287.37 mode=SHORT_ONLY bias=SHORT scale=1.0 |
| 39,909 | 291.90 | COST_REDUCING | TRIM@291.90 shares=188.7 L1ratio=0.542 |
| 40,529 | 295.08 | SCANNING | STOP:trading_mode_flip_long@295.08 pnl=-2.68% |
| 50,895 | 315.00 | POSITION_OPEN | ENTRY_LONG@315.00 mode=BOTH_HALF bias=LONG scale=0.5 |
| 51,694 | 315.66 | COST_REDUCING | TRIM@315.66 shares=157.9 L1ratio=0.995 |
| 52,243 | 321.03 | POSITION_OPEN | CLOSE_DIFF@321.03 profit=-847.81 |
| 52,737 | 319.50 | COST_REDUCING | TRIM@319.50 shares=146.1 L1ratio=0.920 |
| 52,767 | 319.20 | POSITION_OPEN | CLOSE_DIFF@319.20 profit=43.83 |
| 52,924 | 318.36 | COST_REDUCING | TRIM@318.36 shares=146.1 L1ratio=0.920 |
| 53,510 | 321.52 | POSITION_OPEN | CLOSE_DIFF@321.52 profit=-461.67 |
| 53,767 | 319.65 | COST_REDUCING | TRIM@319.65 shares=156.1 L1ratio=0.984 |
| 54,238 | 318.54 | POSITION_OPEN | CLOSE_DIFF@318.54 profit=173.31 |
| 55,453 | 314.73 | COST_REDUCING | TRIM@314.73 shares=138.8 L1ratio=0.874 |
| 55,597 | 317.48 | POSITION_OPEN | CLOSE_DIFF@317.48 profit=-381.65 |
| 55,783 | 318.10 | SCANNING | STOP:trading_mode_flip_short@318.10 pnl=+1.42% |
| 67,965 | 319.43 | POSITION_OPEN | ENTRY_LONG@319.43 mode=BOTH_HALF bias=LONG scale=0.5 |
| 68,196 | 320.04 | COST_REDUCING | TRIM@320.04 shares=151.4 L1ratio=0.967 |
| 68,316 | 319.31 | POSITION_OPEN | CLOSE_DIFF@319.31 profit=110.53 |
| 68,532 | 321.28 | POSITION_OPEN | ADD_POS@321.28 +27.1 L1ratio=0.173 |
| 68,893 | 322.36 | COST_REDUCING | TRIM@322.36 shares=181.5 L1ratio=0.988 |
| 69,320 | 322.04 | POSITION_OPEN | CLOSE_DIFF@322.04 profit=58.07 |
| 69,405 | 322.46 | COST_REDUCING | TRIM@322.46 shares=181.5 L1ratio=0.989 |
| 69,827 | 322.03 | POSITION_OPEN | CLOSE_DIFF@322.03 profit=78.06 |
| 71,205 | 316.80 | COST_REDUCING | TRIM@316.80 shares=179.4 L1ratio=0.977 |
| 71,802 | 317.60 | POSITION_OPEN | CLOSE_DIFF@317.60 profit=-143.53 |
| 72,003 | 316.08 | SCANNING | STOP:trading_mode_flip_short@316.08 pnl=-0.71% |
| 77,960 | 327.57 | POSITION_OPEN | ENTRY_LONG@327.57 mode=LONG_ONLY bias=LONG scale=1.0 |
| 77,981 | 327.22 | COST_REDUCING | TRIM@327.22 shares=303.5 L1ratio=0.994 |
| 78,031 | 327.75 | POSITION_OPEN | CLOSE_DIFF@327.75 profit=-160.88 |
| 78,440 | 327.44 | COST_REDUCING | TRIM@327.44 shares=299.2 L1ratio=0.980 |
| 78,534 | 327.64 | POSITION_OPEN | CLOSE_DIFF@327.64 profit=-59.84 |
| 78,826 | 328.60 | COST_REDUCING | TRIM@328.60 shares=302.3 L1ratio=0.990 |
| 79,420 | 332.09 | POSITION_OPEN | CLOSE_DIFF@332.09 profit=-1055.20 |
| 80,484 | 336.69 | COST_REDUCING | TRIM@336.69 shares=299.5 L1ratio=0.981 |
| 81,188 | 336.32 | POSITION_OPEN | CLOSE_DIFF@336.32 profit=110.80 |
| 82,391 | 333.84 | COST_REDUCING | TRIM@333.84 shares=296.0 L1ratio=0.970 |
| 82,996 | 330.63 | SCANNING | STOP:trading_mode_flip_short@330.63 pnl=+1.04% |
| 84,038 | 340.69 | POSITION_OPEN | ENTRY_LONG@340.69 mode=LONG_ONLY bias=LONG scale=1.0 |
| 85,792 | 351.01 | COST_REDUCING | TRIM@351.01 shares=277.9 L1ratio=0.947 |
| 86,834 | 347.42 | SCANNING | STOP:trading_mode_flip_short@347.42 pnl=+1.98% |
| 93,055 | 357.62 | POSITION_OPEN | ENTRY_LONG@357.62 mode=LONG_ONLY bias=LONG scale=1.0 |
| 93,739 | 361.71 | COST_REDUCING | TRIM@361.71 shares=261.0 L1ratio=0.933 |
| 93,922 | 363.24 | POSITION_OPEN | CLOSE_DIFF@363.24 profit=-399.35 |
| 94,011 | 362.72 | COST_REDUCING | TRIM@362.72 shares=277.2 L1ratio=0.991 |
| 94,978 | 366.34 | POSITION_OPEN | CLOSE_DIFF@366.34 profit=-1003.51 |
| 95,575 | 368.10 | COST_REDUCING | TRIM@368.10 shares=277.9 L1ratio=0.994 |
| 95,754 | 370.23 | POSITION_OPEN | CLOSE_DIFF@370.23 profit=-591.86 |
| 95,904 | 369.83 | COST_REDUCING | TRIM@369.83 shares=268.3 L1ratio=0.960 |
| 96,071 | 370.83 | POSITION_OPEN | CLOSE_DIFF@370.83 profit=-268.34 |
| 96,272 | 371.02 | COST_REDUCING | TRIM@371.02 shares=267.2 L1ratio=0.956 |
| 97,689 | 366.89 | SCANNING | STOP:trading_mode_flip_short@366.89 pnl=+2.59% |
| 109,965 | 377.16 | POSITION_OPEN | ENTRY_LONG@377.16 mode=LONG_ONLY bias=LONG scale=1.0 |
| 110,916 | 380.20 | COST_REDUCING | TRIM@380.20 shares=258.9 L1ratio=0.976 |
| 111,605 | 380.26 | POSITION_OPEN | CLOSE_DIFF@380.26 profit=-15.53 |
| 112,060 | 382.55 | COST_REDUCING | TRIM@382.55 shares=264.2 L1ratio=0.996 |
| ... | ... | ... | （共413条，显示前100） |
</details>

## OKLO

- 数据：**333,613** bars (1min)
- 价格：18.07 → 63.48
- Buy-and-hold: **+251.30%**
- 回测耗时：72.9s

### 两层状态统计

| 指标 | 值 |
|------|-----|
| 操盘层模式切换 | 376 |
| 选股层偏向切换 | 15 |
| 两层矛盾（不操作） | 114,368 bars |

### PH 分层统计

| 级别 | 更新次数 | settle总数 | rank-1 settle |
|------|---------|-----------|---------------|
| L0 | 333,613 (每bar) | 154,247 | 730 |
| L1 | 2,806 | 2,657 | 102 |
| L2 | 185 | 101 | 12 |
| **L2 方向翻转** | — | — | **7** |

### 总体指标

| 指标 | 值 |
|------|-----|
| 交易数 | 16 (多7/空9) |
| 胜率 | 18.8% |
| 平均收益 | -9.983% |
| 复利累计 | -86.49% |
| 最大回撤 | -86.49% |
| 平均持仓 | 2,684 bars |
| 有加仓的交易 | 8/16 |
| 有降成本的交易 | 13/16 |
| 达到本金回收 | 0/16 |

### 按操盘层模式分组

| 模式 | 交易数 | 胜率 | 平均PnL | 复利% |
|------|--------|------|---------|------|
| BOTH_HALF | 7 | 29% | -4.436% | -33.34% |
| LONG_ONLY | 2 | 50% | +1.589% | +1.76% |
| SHORT_ONLY | 7 | 0% | -18.836% | -80.08% |

### 交易明细

| # | 方向 | 入场 | 出场 | bars | PnL% | 模式 | 偏向 | 加仓 | 短差 | PW | 退出原因 |
|---|------|------|------|------|------|------|------|------|------|----|---------| 
| 1 | 空 | 9.11@940 | 11.60@1960 | 1,020 | -33.65 | BOTH_HALF | SHORT | 0 | 1 | 否 | stock_bias_flip_long |
| 2 | 空 | 7.12@4025 | 9.06@7214 | 3,189 | -53.03 | SHORT_ONLY | SHORT | 1 | 2 | 否 | bsp_invalidate |
| 3 | 空 | 5.90@29391 | 6.59@32883 | 3,492 | -19.76 | SHORT_ONLY | SHORT | 0 | 3 | 否 | trading_mode_flip_long |
| 4 | 多 | 28.02@53130 | 23.40@54705 | 1,575 | -10.54 | BOTH_HALF | LONG | 1 | 2 | 否 | trading_mode_flip_short |
| 5 | 多 | 28.75@76268 | 25.75@77960 | 1,692 | -10.43 | LONG_ONLY | LONG | 0 | 0 | 否 | trading_mode_flip_short |
| 6 | 多 | 31.98@82271 | 34.20@86342 | 4,071 | +13.61 | LONG_ONLY | LONG | 0 | 3 | 否 | trading_mode_flip_short |
| 7 | 多 | 46.11@89133 | 54.69@94200 | 5,067 | +17.18 | BOTH_HALF | LONG | 2 | 5 | 否 | bsp_invalidate |
| 8 | 空 | 21.15@114742 | 23.04@120908 | 6,166 | -14.05 | SHORT_ONLY | SHORT | 4 | 6 | 否 | bsp_invalidate |
| 9 | 多 | 68.35@150439 | 66.45@152586 | 2,147 | -2.78 | BOTH_HALF | LONG | 0 | 1 | 否 | trading_mode_flip_short |
| 10 | 多 | 78.49@177102 | 76.59@178793 | 1,691 | -2.42 | BOTH_HALF | LONG | 0 | 0 | 否 | trading_mode_flip_short |
| 11 | 空 | 120.22@206197 | 119.51@208129 | 1,932 | -3.26 | BOTH_HALF | SHORT | 1 | 2 | 否 | trading_mode_flip_long |
| 12 | 多 | 146.72@215837 | 154.48@219699 | 3,862 | +4.41 | BOTH_HALF | LONG | 1 | 2 | 否 | trading_mode_flip_short |
| 13 | 空 | 96.88@235756 | 104.29@236068 | 312 | -7.65 | SHORT_ONLY | SHORT | 0 | 0 | 否 | bsp_invalidate |
| 14 | 空 | 74.94@255587 | 87.80@258571 | 2,984 | -18.44 | SHORT_ONLY | SHORT | 2 | 1 | 否 | trading_mode_flip_long |
| 15 | 空 | 61.02@275312 | 70.30@277398 | 2,086 | -15.21 | SHORT_ONLY | SHORT | 0 | 2 | 否 | trading_mode_flip_long |
| 16 | 空 | 50.22@298313 | 51.23@299970 | 1,657 | -3.71 | SHORT_ONLY | SHORT | 2 | 1 | 否 | trading_mode_flip_long |

### FSM 状态分布（按 bar 数）

| 状态 | Bars | 占比 |
|------|------|------|
| SCANNING | 290,670 | 87.1% |
| POSITION_OPEN | 23,861 | 7.2% |
| COST_REDUCING | 19,082 | 5.7% |
| PRINCIPAL_WITHDRAWN | 0 | 0.0% |
| STOPPED_OUT | 0 | 0.0% |

<details><summary>事件流（117条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 940 | 9.11 | POSITION_OPEN | ENTRY_SHORT@9.11 mode=BOTH_HALF bias=SHORT scale=0.5 |
| 974 | 8.86 | COST_REDUCING | TRIM@8.86 shares=3359.0 L1ratio=0.612 |
| 1,451 | 9.80 | POSITION_OPEN | CLOSE_DIFF@9.80 profit=3157.45 |
| 1,510 | 10.19 | COST_REDUCING | TRIM@10.19 shares=4888.5 L1ratio=0.891 |
| 1,960 | 11.60 | SCANNING | STOP:stock_bias_flip_long@11.60 pnl=-33.65% |
| 4,025 | 7.12 | POSITION_OPEN | ENTRY_SHORT@7.12 mode=SHORT_ONLY bias=SHORT scale=1.0 |
| 4,581 | 7.54 | COST_REDUCING | TRIM@7.54 shares=13072.7 L1ratio=0.931 |
| 6,177 | 11.16 | POSITION_OPEN | CLOSE_DIFF@11.16 profit=47323.14 |
| 6,343 | 11.15 | COST_REDUCING | TRIM@11.15 shares=13072.7 L1ratio=0.931 |
| 6,637 | 10.41 | POSITION_OPEN | CLOSE_DIFF@10.41 profit=-9673.79 |
| 7,192 | 9.18 | POSITION_OPEN | ADD_POS@9.18 +5529.0 L1ratio=0.394 |
| 7,214 | 9.06 | SCANNING | STOP:bsp_invalidate@9.06 pnl=-53.03% |
| 29,391 | 5.90 | POSITION_OPEN | ENTRY_SHORT@5.90 mode=SHORT_ONLY bias=SHORT scale=1.0 |
| 30,129 | 5.83 | COST_REDUCING | TRIM@5.83 shares=16441.1 L1ratio=0.970 |
| 30,579 | 5.60 | POSITION_OPEN | CLOSE_DIFF@5.60 profit=-3863.65 |
| 31,265 | 6.00 | COST_REDUCING | TRIM@6.00 shares=16441.1 L1ratio=0.970 |
| 31,505 | 6.09 | POSITION_OPEN | CLOSE_DIFF@6.09 profit=1397.49 |
| 31,963 | 6.21 | COST_REDUCING | TRIM@6.21 shares=16064.7 L1ratio=0.948 |
| 32,730 | 6.62 | POSITION_OPEN | CLOSE_DIFF@6.62 profit=6666.84 |
| 32,883 | 6.59 | SCANNING | STOP:trading_mode_flip_long@6.59 pnl=-19.76% |
| 53,130 | 28.02 | POSITION_OPEN | ENTRY_LONG@28.02 mode=BOTH_HALF bias=LONG scale=0.5 |
| 53,314 | 26.42 | POSITION_OPEN | ADD_POS@26.42 +626.0 L1ratio=0.351 |
| 53,605 | 24.94 | COST_REDUCING | TRIM@24.94 shares=2395.3 L1ratio=0.994 |
| 53,907 | 24.36 | POSITION_OPEN | CLOSE_DIFF@24.36 profit=1389.29 |
| 53,993 | 24.40 | COST_REDUCING | TRIM@24.40 shares=2395.3 L1ratio=0.994 |
| 54,119 | 23.72 | POSITION_OPEN | CLOSE_DIFF@23.72 profit=1628.83 |
| 54,228 | 23.83 | COST_REDUCING | TRIM@23.83 shares=2395.3 L1ratio=0.994 |
| 54,705 | 23.40 | SCANNING | STOP:trading_mode_flip_short@23.40 pnl=-10.54% |
| 76,268 | 28.75 | POSITION_OPEN | ENTRY_LONG@28.75 mode=LONG_ONLY bias=LONG scale=1.0 |
| 77,315 | 28.92 | COST_REDUCING | TRIM@28.92 shares=3181.8 L1ratio=0.915 |
| 77,960 | 25.75 | SCANNING | STOP:trading_mode_flip_short@25.75 pnl=-10.43% |
| 82,271 | 31.98 | POSITION_OPEN | ENTRY_LONG@31.98 mode=LONG_ONLY bias=LONG scale=1.0 |
| 83,207 | 32.37 | COST_REDUCING | TRIM@32.37 shares=2760.5 L1ratio=0.883 |
| 83,593 | 38.78 | POSITION_OPEN | CLOSE_DIFF@38.78 profit=-17694.90 |
| 84,429 | 42.84 | COST_REDUCING | TRIM@42.84 shares=2706.0 L1ratio=0.865 |
| 84,463 | 42.52 | POSITION_OPEN | CLOSE_DIFF@42.52 profit=865.91 |
| 84,755 | 35.90 | COST_REDUCING | TRIM@35.90 shares=2449.6 L1ratio=0.783 |
| 86,173 | 33.53 | POSITION_OPEN | CLOSE_DIFF@33.53 profit=5805.65 |
| 86,342 | 34.20 | SCANNING | STOP:trading_mode_flip_short@34.20 pnl=+13.61% |
| 89,133 | 46.11 | POSITION_OPEN | ENTRY_LONG@46.11 mode=BOTH_HALF bias=LONG scale=0.5 |
| 89,466 | 46.55 | COST_REDUCING | TRIM@46.55 shares=1070.2 L1ratio=0.987 |
| 90,008 | 45.95 | POSITION_OPEN | CLOSE_DIFF@45.95 profit=642.11 |
| 91,119 | 48.95 | COST_REDUCING | TRIM@48.95 shares=970.1 L1ratio=0.895 |
| 91,475 | 48.63 | POSITION_OPEN | CLOSE_DIFF@48.63 profit=305.58 |
| 92,285 | 54.89 | COST_REDUCING | TRIM@54.89 shares=1052.0 L1ratio=0.970 |
| 92,312 | 55.20 | POSITION_OPEN | CLOSE_DIFF@55.20 profit=-326.13 |
| 92,466 | 54.30 | COST_REDUCING | TRIM@54.30 shares=1052.0 L1ratio=0.970 |
| 93,016 | 52.38 | POSITION_OPEN | CLOSE_DIFF@52.38 profit=2014.63 |
| 93,454 | 50.88 | COST_REDUCING | TRIM@50.88 shares=983.6 L1ratio=0.907 |
| 93,772 | 52.73 | POSITION_OPEN | CLOSE_DIFF@52.73 profit=-1814.77 |
| 93,903 | 54.91 | POSITION_OPEN | ADD_POS@54.91 +213.1 L1ratio=0.197 |
| 94,086 | 54.37 | POSITION_OPEN | ADD_POS@54.37 +255.0 L1ratio=0.197 |
| 94,200 | 54.69 | SCANNING | STOP:bsp_invalidate@54.69 pnl=+17.18% |
| 114,742 | 21.15 | POSITION_OPEN | ENTRY_SHORT@21.15 mode=SHORT_ONLY bias=SHORT scale=1.0 |
| 115,146 | 22.23 | POSITION_OPEN | ADD_POS@22.23 +2014.7 L1ratio=0.426 |
| 115,561 | 22.14 | COST_REDUCING | TRIM@22.14 shares=4756.5 L1ratio=0.705 |
| 116,151 | 22.58 | POSITION_OPEN | CLOSE_DIFF@22.58 profit=2092.85 |
| 116,297 | 22.81 | COST_REDUCING | TRIM@22.81 shares=4756.5 L1ratio=0.705 |
| 116,526 | 22.64 | POSITION_OPEN | CLOSE_DIFF@22.64 profit=-808.60 |
| 116,925 | 21.86 | POSITION_OPEN | ADD_POS@21.86 +2873.2 L1ratio=0.426 |
| 117,251 | 22.36 | COST_REDUCING | TRIM@22.36 shares=6832.9 L1ratio=0.711 |
| 117,357 | 22.11 | POSITION_OPEN | CLOSE_DIFF@22.11 profit=-1708.23 |
| 117,668 | 19.31 | POSITION_OPEN | ADD_POS@19.31 +4097.5 L1ratio=0.426 |
| 118,096 | 18.49 | COST_REDUCING | TRIM@18.49 shares=10486.4 L1ratio=0.765 |
| 118,188 | 18.64 | POSITION_OPEN | CLOSE_DIFF@18.64 profit=1572.96 |
| 118,329 | 21.76 | COST_REDUCING | TRIM@21.76 shares=10486.4 L1ratio=0.765 |
| 119,055 | 22.07 | POSITION_OPEN | CLOSE_DIFF@22.07 profit=3303.22 |
| 119,663 | 21.25 | COST_REDUCING | TRIM@21.25 shares=10486.4 L1ratio=0.765 |
| 120,238 | 23.29 | POSITION_OPEN | CLOSE_DIFF@23.29 profit=21392.30 |
| 120,725 | 22.95 | POSITION_OPEN | ADD_POS@22.95 +5843.6 L1ratio=0.426 |
| 120,908 | 23.04 | SCANNING | STOP:bsp_invalidate@23.04 pnl=-14.05% |
| 150,439 | 68.35 | POSITION_OPEN | ENTRY_LONG@68.35 mode=BOTH_HALF bias=LONG scale=0.5 |
| 150,663 | 65.13 | COST_REDUCING | TRIM@65.13 shares=613.7 L1ratio=0.839 |
| 151,324 | 66.94 | POSITION_OPEN | CLOSE_DIFF@66.94 profit=-1113.94 |
| 151,663 | 61.77 | COST_REDUCING | TRIM@61.77 shares=594.1 L1ratio=0.812 |
| 152,586 | 66.45 | SCANNING | STOP:trading_mode_flip_short@66.45 pnl=-2.78% |
| 177,102 | 78.49 | POSITION_OPEN | ENTRY_LONG@78.49 mode=BOTH_HALF bias=LONG scale=0.5 |
| 177,478 | 76.18 | COST_REDUCING | TRIM@76.18 shares=621.0 L1ratio=0.975 |
| 178,793 | 76.59 | SCANNING | STOP:trading_mode_flip_short@76.59 pnl=-2.42% |
| 206,197 | 120.22 | POSITION_OPEN | ENTRY_SHORT@120.22 mode=BOTH_HALF bias=SHORT scale=0.5 |
| 207,032 | 118.36 | COST_REDUCING | TRIM@118.36 shares=107.0 L1ratio=0.257 |
| 207,189 | 109.70 | POSITION_OPEN | CLOSE_DIFF@109.70 profit=-926.26 |
| 207,287 | 111.00 | POSITION_OPEN | ADD_POS@111.00 +350.3 L1ratio=0.842 |
| 207,376 | 109.15 | COST_REDUCING | TRIM@109.15 shares=197.0 L1ratio=0.257 |
| 207,413 | 110.77 | POSITION_OPEN | CLOSE_DIFF@110.77 profit=318.21 |
| 207,678 | 112.93 | COST_REDUCING | TRIM@112.93 shares=197.0 L1ratio=0.257 |
| 208,129 | 119.51 | SCANNING | STOP:trading_mode_flip_long@119.51 pnl=-3.26% |
| 215,837 | 146.72 | POSITION_OPEN | ENTRY_LONG@146.72 mode=BOTH_HALF bias=LONG scale=0.5 |
| 215,927 | 147.00 | COST_REDUCING | TRIM@147.00 shares=305.0 L1ratio=0.895 |
| 216,132 | 158.28 | POSITION_OPEN | CLOSE_DIFF@158.28 profit=-3439.87 |
| 216,282 | 157.66 | POSITION_OPEN | ADD_POS@157.66 +45.4 L1ratio=0.133 |
| 217,096 | 167.76 | COST_REDUCING | TRIM@167.76 shares=353.3 L1ratio=0.915 |
| 217,321 | 170.10 | POSITION_OPEN | CLOSE_DIFF@170.10 profit=-826.73 |
| 218,565 | 172.82 | COST_REDUCING | TRIM@172.82 shares=349.4 L1ratio=0.905 |
| 219,699 | 154.48 | SCANNING | STOP:trading_mode_flip_short@154.48 pnl=+4.41% |
| 235,756 | 96.88 | POSITION_OPEN | ENTRY_SHORT@96.88 mode=SHORT_ONLY bias=SHORT scale=1.0 |
| 236,068 | 104.29 | SCANNING | STOP:bsp_invalidate@104.29 pnl=-7.65% |
| 255,587 | 74.94 | POSITION_OPEN | ENTRY_SHORT@74.94 mode=SHORT_ONLY bias=SHORT scale=1.0 |
| 255,813 | 74.54 | COST_REDUCING | TRIM@74.54 shares=843.9 L1ratio=0.632 |
| 255,860 | 74.56 | POSITION_OPEN | CLOSE_DIFF@74.56 profit=16.88 |
| ... | ... | ... | （共117条，显示前100） |
</details>

## HK700

- 数据：**1,408,882** bars (1min)
- 价格：11.36 → 458.20
- Buy-and-hold: **+3933.45%**
- 回测耗时：523.8s

### 两层状态统计

| 指标 | 值 |
|------|-----|
| 操盘层模式切换 | 616 |
| 选股层偏向切换 | 17 |
| 两层矛盾（不操作） | 404,324 bars |

### PH 分层统计

| 级别 | 更新次数 | settle总数 | rank-1 settle |
|------|---------|-----------|---------------|
| L0 | 1,408,882 (每bar) | 463,934 | 2312 |
| L1 | 5,014 | 4,798 | 184 |
| L2 | 298 | 141 | 11 |
| **L2 方向翻转** | — | — | **5** |

### 总体指标

| 指标 | 值 |
|------|-----|
| 交易数 | 22 (多17/空5) |
| 胜率 | 50.0% |
| 平均收益 | +0.767% |
| 复利累计 | +5.61% |
| 最大回撤 | -32.32% |
| 平均持仓 | 11,969 bars |
| 有加仓的交易 | 11/22 |
| 有降成本的交易 | 20/22 |
| 达到本金回收 | 0/22 |

### 按操盘层模式分组

| 模式 | 交易数 | 胜率 | 平均PnL | 复利% |
|------|--------|------|---------|------|
| BOTH_HALF | 11 | 45% | -1.640% | -18.77% |
| LONG_ONLY | 9 | 67% | +6.433% | +67.76% |
| SHORT_ONLY | 2 | 0% | -11.487% | -22.49% |

### 交易明细

| # | 方向 | 入场 | 出场 | bars | PnL% | 模式 | 偏向 | 加仓 | 短差 | PW | 退出原因 |
|---|------|------|------|------|------|------|------|------|------|----|---------| 
| 1 | 多 | 12.50@5094 | 10.86@12823 | 7,729 | -9.83 | BOTH_HALF | LONG | 1 | 1 | 否 | bsp_invalidate |
| 2 | 空 | 7.70@18094 | 8.62@29451 | 11,357 | -20.65 | SHORT_ONLY | SHORT | 1 | 7 | 否 | trading_mode_flip_long |
| 3 | 多 | 34.30@138285 | 32.40@147449 | 9,164 | +0.31 | LONG_ONLY | LONG | 0 | 2 | 否 | trading_mode_flip_short |
| 4 | 多 | 40.80@162024 | 35.55@165080 | 3,056 | -5.70 | BOTH_HALF | LONG | 0 | 1 | 否 | trading_mode_flip_short |
| 5 | 多 | 50.50@289428 | 46.70@326792 | 37,364 | +3.60 | BOTH_HALF | LONG | 1 | 4 | 否 | trading_mode_flip_short |
| 6 | 多 | 64.90@348935 | 82.80@377587 | 28,652 | +26.16 | LONG_ONLY | LONG | 2 | 4 | 否 | bsp_invalidate |
| 7 | 多 | 87.50@378891 | 91.65@409422 | 30,531 | +18.19 | LONG_ONLY | LONG | 1 | 9 | 否 | trading_mode_flip_short |
| 8 | 多 | 119.10@428314 | 112.75@438678 | 10,364 | -3.90 | BOTH_HALF | LONG | 0 | 3 | 否 | trading_mode_flip_short |
| 9 | 多 | 119.30@453754 | 109.15@457634 | 3,880 | -8.51 | LONG_ONLY | LONG | 0 | 0 | 否 | trading_mode_flip_short |
| 10 | 多 | 163.45@584882 | 173.50@592345 | 7,463 | +6.33 | BOTH_HALF | LONG | 0 | 3 | 否 | trading_mode_flip_short |
| 11 | 多 | 187.85@594983 | 171.00@622111 | 27,128 | -8.97 | BOTH_HALF | LONG | 0 | 7 | 否 | trading_mode_flip_short |
| 12 | 多 | 222.90@652085 | 241.50@667774 | 15,689 | +10.05 | LONG_ONLY | LONG | 0 | 5 | 否 | trading_mode_flip_short |
| 13 | 多 | 277.10@672227 | 304.40@681887 | 9,660 | +9.85 | BOTH_HALF | LONG | 0 | 3 | 否 | trading_mode_flip_short |
| 14 | 多 | 346.40@696868 | 357.10@703430 | 6,562 | +3.53 | BOTH_HALF | LONG | 0 | 2 | 否 | trading_mode_flip_short |
| 15 | 多 | 404.70@710943 | 385.50@718144 | 7,201 | -4.74 | LONG_ONLY | LONG | 0 | 3 | 否 | trading_mode_flip_short |
| 16 | 空 | 323.50@758489 | 318.90@762257 | 3,768 | -2.32 | SHORT_ONLY | SHORT | 2 | 2 | 否 | bsp_invalidate |
| 17 | 空 | 278.70@773351 | 279.30@785525 | 12,174 | -11.60 | BOTH_HALF | SHORT | 3 | 4 | 否 | trading_mode_flip_long |
| 18 | 多 | 442.10@915380 | 490.00@922761 | 7,381 | +9.88 | LONG_ONLY | LONG | 1 | 6 | 否 | bsp_invalidate |
| 19 | 多 | 552.20@944407 | 541.20@954142 | 9,735 | -1.57 | LONG_ONLY | LONG | 0 | 3 | 否 | trading_mode_flip_short |
| 20 | 多 | 583.60@962507 | 634.80@971387 | 8,880 | +8.14 | LONG_ONLY | LONG | 1 | 4 | 否 | trading_mode_flip_short |
| 21 | 空 | 380.50@1056649 | 366.20@1057460 | 811 | +1.55 | BOTH_HALF | SHORT | 2 | 0 | 否 | bsp_invalidate |
| 22 | 空 | 218.80@1109328 | 219.80@1114101 | 4,773 | -2.91 | BOTH_HALF | SHORT | 1 | 3 | 否 | trading_mode_flip_long |

### FSM 状态分布（按 bar 数）

| 状态 | Bars | 占比 |
|------|------|------|
| SCANNING | 1,145,560 | 81.3% |
| POSITION_OPEN | 152,233 | 10.8% |
| COST_REDUCING | 111,089 | 7.9% |
| PRINCIPAL_WITHDRAWN | 0 | 0.0% |
| STOPPED_OUT | 0 | 0.0% |

<details><summary>事件流（223条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 5,094 | 12.50 | POSITION_OPEN | ENTRY_LONG@12.50 mode=BOTH_HALF bias=LONG scale=0.5 |
| 7,628 | 12.72 | COST_REDUCING | TRIM@12.72 shares=3308.3 L1ratio=0.827 |
| 10,757 | 12.00 | POSITION_OPEN | CLOSE_DIFF@12.00 profit=2381.95 |
| 11,999 | 12.36 | POSITION_OPEN | ADD_POS@12.36 +2706.8 L1ratio=0.677 |
| 12,823 | 10.86 | SCANNING | STOP:bsp_invalidate@10.86 pnl=-9.83% |
| 18,094 | 7.70 | POSITION_OPEN | ENTRY_SHORT@7.70 mode=SHORT_ONLY bias=SHORT scale=1.0 |
| 19,212 | 9.38 | COST_REDUCING | TRIM@9.38 shares=8007.8 L1ratio=0.617 |
| 20,423 | 8.94 | POSITION_OPEN | CLOSE_DIFF@8.94 profit=-3523.43 |
| 21,798 | 9.70 | COST_REDUCING | TRIM@9.70 shares=9721.0 L1ratio=0.749 |
| 23,146 | 9.34 | POSITION_OPEN | CLOSE_DIFF@9.34 profit=-3499.58 |
| 23,776 | 9.74 | COST_REDUCING | TRIM@9.74 shares=8837.3 L1ratio=0.680 |
| 24,003 | 9.36 | POSITION_OPEN | CLOSE_DIFF@9.36 profit=-3358.18 |
| 25,691 | 7.30 | POSITION_OPEN | ADD_POS@7.30 +8107.3 L1ratio=0.624 |
| 26,348 | 7.56 | COST_REDUCING | TRIM@7.56 shares=18785.2 L1ratio=0.891 |
| 26,482 | 7.38 | POSITION_OPEN | CLOSE_DIFF@7.38 profit=-3381.33 |
| 27,347 | 7.70 | COST_REDUCING | TRIM@7.70 shares=18785.2 L1ratio=0.891 |
| 27,446 | 8.14 | POSITION_OPEN | CLOSE_DIFF@8.14 profit=8265.47 |
| 27,506 | 7.96 | COST_REDUCING | TRIM@7.96 shares=18785.2 L1ratio=0.891 |
| 28,055 | 8.08 | POSITION_OPEN | CLOSE_DIFF@8.08 profit=2254.22 |
| 28,517 | 8.86 | COST_REDUCING | TRIM@8.86 shares=18785.2 L1ratio=0.891 |
| 29,343 | 8.88 | POSITION_OPEN | CLOSE_DIFF@8.88 profit=375.70 |
| 29,451 | 8.62 | SCANNING | STOP:trading_mode_flip_long@8.62 pnl=-20.65% |
| 138,285 | 34.30 | POSITION_OPEN | ENTRY_LONG@34.30 mode=LONG_ONLY bias=LONG scale=1.0 |
| 140,700 | 33.65 | COST_REDUCING | TRIM@33.65 shares=2915.5 L1ratio=1.000 |
| 142,624 | 33.25 | POSITION_OPEN | CLOSE_DIFF@33.25 profit=1166.18 |
| 143,011 | 33.35 | COST_REDUCING | TRIM@33.35 shares=2753.2 L1ratio=0.944 |
| 144,562 | 31.65 | POSITION_OPEN | CLOSE_DIFF@31.65 profit=4680.43 |
| 145,776 | 30.80 | COST_REDUCING | TRIM@30.80 shares=2727.0 L1ratio=0.935 |
| 147,449 | 32.40 | SCANNING | STOP:trading_mode_flip_short@32.40 pnl=+0.31% |
| 162,024 | 40.80 | POSITION_OPEN | ENTRY_LONG@40.80 mode=BOTH_HALF bias=LONG scale=0.5 |
| 163,408 | 39.95 | COST_REDUCING | TRIM@39.95 shares=1137.0 L1ratio=0.928 |
| 164,411 | 36.80 | POSITION_OPEN | CLOSE_DIFF@36.80 profit=3581.43 |
| 164,907 | 35.45 | COST_REDUCING | TRIM@35.45 shares=1130.0 L1ratio=0.922 |
| 165,080 | 35.55 | SCANNING | STOP:trading_mode_flip_short@35.55 pnl=-5.70% |
| 289,428 | 50.50 | POSITION_OPEN | ENTRY_LONG@50.50 mode=BOTH_HALF bias=LONG scale=0.5 |
| 291,912 | 49.70 | COST_REDUCING | TRIM@49.70 shares=979.0 L1ratio=0.989 |
| 302,478 | 45.90 | POSITION_OPEN | CLOSE_DIFF@45.90 profit=3720.20 |
| 307,353 | 49.60 | POSITION_OPEN | ADD_POS@49.60 +142.1 L1ratio=0.143 |
| 309,259 | 49.20 | COST_REDUCING | TRIM@49.20 shares=1109.3 L1ratio=0.980 |
| 309,924 | 50.00 | POSITION_OPEN | CLOSE_DIFF@50.00 profit=-887.46 |
| 315,344 | 48.60 | COST_REDUCING | TRIM@48.60 shares=1124.6 L1ratio=0.993 |
| 316,732 | 51.30 | POSITION_OPEN | CLOSE_DIFF@51.30 profit=-3036.45 |
| 320,109 | 48.50 | COST_REDUCING | TRIM@48.50 shares=1093.0 L1ratio=0.965 |
| 324,400 | 46.20 | POSITION_OPEN | CLOSE_DIFF@46.20 profit=2513.82 |
| 326,792 | 46.70 | SCANNING | STOP:trading_mode_flip_short@46.70 pnl=+3.60% |
| 348,935 | 64.90 | POSITION_OPEN | ENTRY_LONG@64.90 mode=LONG_ONLY bias=LONG scale=1.0 |
| 352,110 | 68.50 | COST_REDUCING | TRIM@68.50 shares=1445.4 L1ratio=0.938 |
| 352,332 | 67.70 | POSITION_OPEN | CLOSE_DIFF@67.70 profit=1156.36 |
| 352,795 | 68.70 | COST_REDUCING | TRIM@68.70 shares=1533.5 L1ratio=0.995 |
| 355,033 | 66.20 | POSITION_OPEN | CLOSE_DIFF@66.20 profit=3833.82 |
| 364,770 | 76.90 | COST_REDUCING | TRIM@76.90 shares=1521.7 L1ratio=0.988 |
| 367,580 | 78.70 | POSITION_OPEN | CLOSE_DIFF@78.70 profit=-2738.97 |
| 368,642 | 77.30 | COST_REDUCING | TRIM@77.30 shares=1439.3 L1ratio=0.934 |
| 372,638 | 75.70 | POSITION_OPEN | CLOSE_DIFF@75.70 profit=2302.89 |
| 376,128 | 81.90 | POSITION_OPEN | ADD_POS@81.90 +246.9 L1ratio=0.160 |
| 377,223 | 82.40 | POSITION_OPEN | ADD_POS@82.40 +286.4 L1ratio=0.160 |
| 377,587 | 82.80 | SCANNING | STOP:bsp_invalidate@82.80 pnl=+26.16% |
| 378,891 | 87.50 | POSITION_OPEN | ENTRY_LONG@87.50 mode=LONG_ONLY bias=LONG scale=1.0 |
| 379,838 | 87.00 | COST_REDUCING | TRIM@87.00 shares=1133.1 L1ratio=0.991 |
| 380,024 | 87.20 | POSITION_OPEN | CLOSE_DIFF@87.20 profit=-226.62 |
| 381,251 | 85.80 | COST_REDUCING | TRIM@85.80 shares=1078.7 L1ratio=0.944 |
| 383,149 | 92.00 | POSITION_OPEN | CLOSE_DIFF@92.00 profit=-6688.22 |
| 383,351 | 92.70 | COST_REDUCING | TRIM@92.70 shares=1082.2 L1ratio=0.947 |
| 385,176 | 90.20 | POSITION_OPEN | CLOSE_DIFF@90.20 profit=2705.38 |
| 385,518 | 92.10 | COST_REDUCING | TRIM@92.10 shares=1082.2 L1ratio=0.947 |
| 386,478 | 97.20 | POSITION_OPEN | CLOSE_DIFF@97.20 profit=-5518.97 |
| 387,966 | 94.20 | COST_REDUCING | TRIM@94.20 shares=1072.3 L1ratio=0.938 |
| 389,456 | 96.80 | POSITION_OPEN | CLOSE_DIFF@96.80 profit=-2787.93 |
| 389,910 | 93.60 | COST_REDUCING | TRIM@93.60 shares=1122.2 L1ratio=0.982 |
| 390,418 | 95.90 | POSITION_OPEN | CLOSE_DIFF@95.90 profit=-2581.03 |
| 391,198 | 98.20 | POSITION_OPEN | ADD_POS@98.20 +98.5 L1ratio=0.086 |
| 393,846 | 106.40 | COST_REDUCING | TRIM@106.40 shares=1161.0 L1ratio=0.935 |
| 395,385 | 114.00 | POSITION_OPEN | CLOSE_DIFF@114.00 profit=-8823.72 |
| 398,194 | 110.60 | COST_REDUCING | TRIM@110.60 shares=1156.3 L1ratio=0.932 |
| 405,133 | 99.40 | POSITION_OPEN | CLOSE_DIFF@99.40 profit=12950.72 |
| 405,802 | 97.40 | COST_REDUCING | TRIM@97.40 shares=1107.6 L1ratio=0.892 |
| 407,247 | 98.75 | POSITION_OPEN | CLOSE_DIFF@98.75 profit=-1495.22 |
| 408,073 | 96.55 | COST_REDUCING | TRIM@96.55 shares=1107.6 L1ratio=0.892 |
| 409,422 | 91.65 | SCANNING | STOP:trading_mode_flip_short@91.65 pnl=+18.19% |
| 428,314 | 119.10 | POSITION_OPEN | ENTRY_LONG@119.10 mode=BOTH_HALF bias=LONG scale=0.5 |
| 429,469 | 118.80 | COST_REDUCING | TRIM@118.80 shares=403.8 L1ratio=0.962 |
| 429,983 | 119.35 | POSITION_OPEN | CLOSE_DIFF@119.35 profit=-222.08 |
| 431,636 | 121.50 | COST_REDUCING | TRIM@121.50 shares=409.7 L1ratio=0.976 |
| 432,017 | 123.05 | POSITION_OPEN | CLOSE_DIFF@123.05 profit=-634.99 |
| 435,026 | 120.20 | COST_REDUCING | TRIM@120.20 shares=410.0 L1ratio=0.977 |
| 437,828 | 118.45 | POSITION_OPEN | CLOSE_DIFF@118.45 profit=717.52 |
| 438,678 | 112.75 | SCANNING | STOP:trading_mode_flip_short@112.75 pnl=-3.90% |
| 453,754 | 119.30 | POSITION_OPEN | ENTRY_LONG@119.30 mode=LONG_ONLY bias=LONG scale=1.0 |
| 456,029 | 115.30 | COST_REDUCING | TRIM@115.30 shares=819.3 L1ratio=0.977 |
| 457,634 | 109.15 | SCANNING | STOP:trading_mode_flip_short@109.15 pnl=-8.51% |
| 584,882 | 163.45 | POSITION_OPEN | ENTRY_LONG@163.45 mode=BOTH_HALF bias=LONG scale=0.5 |
| 585,065 | 160.30 | COST_REDUCING | TRIM@160.30 shares=303.1 L1ratio=0.991 |
| 586,264 | 166.45 | POSITION_OPEN | CLOSE_DIFF@166.45 profit=-1864.08 |
| 587,565 | 170.45 | COST_REDUCING | TRIM@170.45 shares=301.7 L1ratio=0.986 |
| 587,878 | 170.15 | POSITION_OPEN | CLOSE_DIFF@170.15 profit=90.50 |
| 589,223 | 170.90 | COST_REDUCING | TRIM@170.90 shares=304.3 L1ratio=0.995 |
| 589,341 | 171.80 | POSITION_OPEN | CLOSE_DIFF@171.80 profit=-273.86 |
| 590,137 | 172.65 | COST_REDUCING | TRIM@172.65 shares=301.4 L1ratio=0.985 |
| 592,345 | 173.50 | SCANNING | STOP:trading_mode_flip_short@173.50 pnl=+6.33% |
| 594,983 | 187.85 | POSITION_OPEN | ENTRY_LONG@187.85 mode=BOTH_HALF bias=LONG scale=0.5 |
| ... | ... | ... | （共223条，显示前100） |
</details>

## 三版对比

| 标的 | 版本 | 复利% | 胜率 | 交易数 | 最大回撤 |
|------|------|------|------|--------|---------|
| QQQ | 单层PH（旧H组） | 全亏 | — | — | — |
| QQQ | 多级别PH（上版） | +58% | — | — | — |
| QQQ | **两层分离（本版）** | **+18.47%** | 52% | 44 | -8.1% |
| OKLO | 单层PH（旧H组） | 全亏 | — | — | — |
| OKLO | 多级别PH（上版） | +72% | — | — | — |
| OKLO | **两层分离（本版）** | **-86.49%** | 19% | 16 | -86.5% |
| HK700 | 单层PH（旧H组） | 全亏 | — | — | — |
| HK700 | 多级别PH（上版） | +223% | — | — | — |
| HK700 | **两层分离（本版）** | **+5.61%** | 50% | 22 | -32.3% |

## 汇总

| 标的 | Bars | BH% | 复利% | 胜率 | 交易数 | 模式切换 | 偏向切换 | 矛盾bars | 耗时 |
|------|------|-----|------|------|--------|---------|---------|---------|------|
| QQQ | 728,030 | +174.6 | +18.47 | 52% | 44 | 797 | 15 | 243,146 | 367s |
| OKLO | 333,613 | +251.3 | -86.49 | 19% | 16 | 376 | 15 | 114,368 | 73s |
| HK700 | 1,408,882 | +3933.5 | +5.61 | 50% | 22 | 616 | 17 | 404,324 | 524s |

## 结果包六要素

**结论**：两层分离回测 — 操盘层用 L1 走势类型区分策略，选股层用最高递归级别方向做代理。

**定义依据**：
- 走势类型 Move.kind: trend（2+中枢递升/递降）/ consolidation（1中枢）— `a_move_v1.py`
- Move.direction: up/down — 趋势=中枢递升方向，盘整=break_direction
- 操盘层规则：trend_up→只多, trend_down→只空, consolidation→双向半仓
- 选股层规则：最高有效递归级别最后一个 Move 的方向
- 两层组合：入场需两层同意（选股层不反对 + 操盘层允许该方向）

**边界条件**：
- PENDING_EXPIRY = 390 bars
- 递归深度 max_levels=2 → 最高有效级别取决于数据量是否足以涌现
- 选股层在递归未涌现时退回 L1 → 两层可能实质退化为单层
- 盘整半仓是固定 0.5 比例 — 可考虑动态调整

**下游推论**：
- 若两层分离优于单纯多级别PH → 走势类型区分是有效的过滤器
- 若矛盾bars占比高 → 两层频繁矛盾说明级别错配
- 若盘整交易亏损多 → consolidation 策略需要更严格的进出条件

**谱系引用**：
- 走势方向代理陷阱（memory: project_trend_direction_proxy）
- 526号：递归存在论区分
- 267号：满仓满融降成本体系

**影响声明**：新建独立回测脚本 `fugue_twolayer_backtest_1min.py`，不修改引擎代码或旧版回测。

**认识论等级**：L2（真实数据，3标的 1min；可产生否定性结果）。