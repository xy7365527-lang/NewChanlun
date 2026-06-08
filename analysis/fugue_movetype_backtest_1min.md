# 走势类型过滤回测 — 1分钟数据

## 架构

操盘层用交易级别（L1）Move.kind 区分策略 + 三级别 PH 分层：

| 信号 | 来源 | 规则 |
|------|------|------|
| 操盘方向 | L1 Move.kind + direction | trend_up→只多, trend_down→只空, consolidation→双向 |
| 进场门控 | L0 PH rank-1 settle + BSP | 微观结构确认 |
| 降成本 | L1 PH non-rank-1 settle | 中间级别短差 |
| 加仓 | L1 PH persistence ratio + BSP | 与多级别PH版相同 |
| 止损 | L2 PH rank-1 settle / Move 翻转为反向趋势 | 结构止损 |

**核心区别**：不用选股层/最高级别方向，只用交易级别自身的走势类型做过滤。

**盘整（consolidation）处理**：接受两方向 BSP，中枢范围内高抛低吸。

**止损逻辑**：持多时 Move 翻为 TREND_DOWN → 平仓；持空时翻为 TREND_UP → 平仓。
切换到 CONSOLIDATION 不平仓（盘整兼容两方向）。

## QQQ

- 数据：**728,030** bars (1min)
- 价格：268.82 → 738.28
- Buy-and-hold: **+174.64%**
- 回测耗时：365.5s

### Move 类型统计

| 指标 | 值 | 占比 |
|------|-----|------|
| 模式切换次数 | 797 | — |
| TREND_UP bars | 316,803 | 43.5% |
| TREND_DOWN bars | 188,536 | 25.9% |
| CONSOLIDATION bars | 222,071 | 30.5% |
| UNKNOWN bars | 620 | 0.1% |

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
| 胜率 | 50.0% |
| 平均收益 | +0.380% |
| 复利累计 | +16.58% |
| 最大回撤 | -7.72% |
| 平均持仓 | 3,973 bars |
| 有加仓的交易 | 12/44 |
| 有降成本的交易 | 34/44 |
| 达到本金回收 | 0/44 |

### 按 Move 类型分组

| 类型 | 交易数 | 多/空 | 胜率 | 平均PnL | 复利% |
|------|--------|------|------|---------|------|
| CONSOLIDATION | 20 | 18/2 | 55% | +1.072% | +22.69% |
| TREND_DOWN | 6 | 0/6 | 0% | -2.086% | -11.94% |
| TREND_UP | 18 | 18/0 | 61% | +0.434% | +7.90% |

### 交易明细

| # | 方向 | 入场 | 出场 | bars | PnL% | Move类型 | 加仓 | 短差 | PW | 退出原因 |
|---|------|------|------|------|------|---------|------|------|----|---------| 
| 1 | 空 | 266.33@959 | 274.53@3770 | 2,811 | -3.39 | TREND_DOWN | 0 | 5 | 否 | move_flip_trend_up |
| 2 | 多 | 275.17@5184 | 282.84@10516 | 5,332 | +2.79 | TREND_UP | 0 | 4 | 否 | move_flip_trend_down |
| 3 | 多 | 292.45@13661 | 292.79@16013 | 2,352 | +0.12 | TREND_UP | 0 | 2 | 否 | move_flip_trend_down |
| 4 | 多 | 298.24@16909 | 305.89@20247 | 3,338 | +3.84 | CONSOLIDATION | 0 | 4 | 否 | move_flip_trend_down |
| 5 | 空 | 297.27@27437 | 294.14@28444 | 1,007 | +0.47 | CONSOLIDATION | 2 | 0 | 否 | bsp_invalidate |
| 6 | 空 | 287.37@39206 | 295.08@40529 | 1,323 | -2.68 | TREND_DOWN | 0 | 0 | 否 | move_flip_trend_up |
| 7 | 多 | 313.67@45926 | 311.37@48006 | 2,080 | -0.22 | TREND_UP | 1 | 2 | 否 | move_flip_trend_down |
| 8 | 多 | 315.00@50895 | 318.10@55783 | 4,888 | +1.42 | CONSOLIDATION | 0 | 5 | 否 | move_flip_trend_down |
| 9 | 多 | 319.43@67965 | 316.08@72003 | 4,038 | -0.71 | CONSOLIDATION | 1 | 4 | 否 | move_flip_trend_down |
| 10 | 多 | 327.57@77960 | 330.63@82996 | 5,036 | +1.04 | TREND_UP | 0 | 4 | 否 | move_flip_trend_down |
| 11 | 多 | 340.69@84038 | 347.42@86834 | 2,796 | +1.98 | TREND_UP | 0 | 0 | 否 | move_flip_trend_down |
| 12 | 多 | 357.62@93055 | 366.89@97689 | 4,634 | +2.59 | TREND_UP | 0 | 4 | 否 | move_flip_trend_down |
| 13 | 多 | 377.16@109965 | 378.73@114709 | 4,744 | +0.54 | TREND_UP | 0 | 3 | 否 | move_flip_trend_down |
| 14 | 空 | 351.34@168860 | 349.96@174956 | 6,096 | -0.75 | TREND_DOWN | 2 | 6 | 否 | move_flip_trend_up |
| 15 | 多 | 389.75@186026 | 386.20@191243 | 5,217 | -0.64 | TREND_UP | 0 | 4 | 否 | move_flip_trend_down |
| 16 | 多 | 395.32@196835 | 403.50@207269 | 10,434 | +2.38 | TREND_UP | 0 | 11 | 否 | move_flip_trend_down |
| 17 | 多 | 413.20@216485 | 425.79@221363 | 4,878 | +3.67 | CONSOLIDATION | 0 | 5 | 否 | move_flip_trend_down |
| 18 | 多 | 435.16@229308 | 428.01@231018 | 1,710 | -1.64 | CONSOLIDATION | 0 | 0 | 否 | move_flip_trend_down |
| 19 | 多 | 439.07@236272 | 434.09@239791 | 3,519 | -0.62 | CONSOLIDATION | 1 | 3 | 否 | move_flip_trend_down |
| 20 | 多 | 441.11@240883 | 439.85@243498 | 2,615 | +0.64 | CONSOLIDATION | 1 | 2 | 否 | bsp_invalidate |
| 21 | 多 | 448.45@252489 | 445.39@254428 | 1,939 | -0.68 | TREND_UP | 0 | 1 | 否 | move_flip_trend_down |
| 22 | 空 | 420.71@268939 | 422.16@269250 | 311 | -0.34 | TREND_DOWN | 1 | 0 | 否 | bsp_invalidate |
| 23 | 空 | 416.61@269419 | 420.63@271022 | 1,603 | -1.16 | TREND_DOWN | 1 | 0 | 否 | move_flip_trend_up |
| 24 | 多 | 456.21@288172 | 455.16@289755 | 1,583 | -0.23 | TREND_UP | 0 | 1 | 否 | move_flip_trend_down |
| 25 | 多 | 463.92@296108 | 478.61@305293 | 9,185 | +3.77 | CONSOLIDATION | 0 | 9 | 否 | move_flip_trend_down |
| 26 | 多 | 488.76@310992 | 489.64@317997 | 7,005 | +0.34 | TREND_UP | 1 | 7 | 否 | move_flip_trend_down |
| 27 | 多 | 519.99@400611 | 525.07@403960 | 3,349 | +1.49 | TREND_UP | 0 | 3 | 否 | move_flip_trend_down |
| 28 | 多 | 533.59@407121 | 516.47@409317 | 2,196 | -3.21 | TREND_UP | 0 | 1 | 否 | move_flip_trend_down |
| 29 | 多 | 540.63@440493 | 537.77@443146 | 2,653 | -0.23 | TREND_UP | 0 | 3 | 否 | move_flip_trend_down |
| 30 | 空 | 402.75@470586 | 423.64@471017 | 431 | -4.19 | TREND_DOWN | 1 | 0 | 否 | bsp_invalidate |
| 31 | 多 | 544.30@520847 | 550.37@524569 | 3,722 | +1.74 | CONSOLIDATION | 0 | 4 | 否 | move_flip_trend_down |
| 32 | 多 | 556.27@525233 | 556.21@525240 | 7 | -0.01 | CONSOLIDATION | 0 | 0 | 否 | bsp_invalidate |
| 33 | 多 | 560.78@532889 | 561.02@535494 | 2,605 | +0.09 | TREND_UP | 0 | 2 | 否 | move_flip_trend_down |
| 34 | 多 | 569.53@538459 | 563.52@543534 | 5,075 | -1.03 | TREND_UP | 0 | 4 | 否 | move_flip_trend_down |
| 35 | 多 | 580.37@549098 | 577.53@551637 | 2,539 | -0.38 | CONSOLIDATION | 0 | 3 | 否 | move_flip_trend_down |
| 36 | 多 | 583.50@566173 | 596.54@571262 | 5,089 | +2.67 | CONSOLIDATION | 0 | 4 | 否 | move_flip_trend_down |
| 37 | 多 | 598.29@572438 | 598.45@572468 | 30 | +0.03 | CONSOLIDATION | 0 | 0 | 否 | bsp_invalidate |
| 38 | 多 | 602.84@573742 | 595.32@575196 | 1,454 | -1.25 | CONSOLIDATION | 0 | 0 | 否 | move_flip_trend_down |
| 39 | 多 | 605.00@579856 | 601.49@588515 | 8,659 | -0.49 | CONSOLIDATION | 2 | 6 | 否 | bsp_invalidate |
| 40 | 多 | 616.84@594460 | 618.00@601392 | 6,932 | +0.36 | CONSOLIDATION | 0 | 4 | 否 | move_flip_trend_down |
| 41 | 空 | 580.27@680908 | 581.78@687759 | 6,851 | -1.89 | CONSOLIDATION | 4 | 5 | 否 | move_flip_trend_up |
| 42 | 多 | 649.98@699681 | 712.60@714315 | 14,634 | +11.87 | CONSOLIDATION | 0 | 16 | 否 | move_flip_trend_down |
| 43 | 多 | 719.08@715449 | 701.65@718141 | 2,692 | -2.03 | CONSOLIDATION | 0 | 3 | 否 | move_flip_trend_down |
| 44 | 多 | 733.61@722601 | 738.28@728029 | 5,428 | +0.70 | TREND_UP | 0 | 6 | 否 | eod_close |

### FSM 状态分布

| 状态 | Bars | 占比 |
|------|------|------|
| SCANNING | 553,210 | 76.0% |
| POSITION_OPEN | 109,279 | 15.0% |
| COST_REDUCING | 65,541 | 9.0% |
| PRINCIPAL_WITHDRAWN | 0 | 0.0% |
| STOPPED_OUT | 0 | 0.0% |

<details><summary>事件流（421条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 959 | 266.33 | POSITION_OPEN | ENTRY_SHORT@266.33 mode=TREND_DOWN L2dir=0 |
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
| 3,770 | 274.53 | SCANNING | STOP:move_flip_trend_up@274.53 pnl=-3.39% |
| 5,184 | 275.17 | POSITION_OPEN | ENTRY_LONG@275.17 mode=TREND_UP L2dir=0 |
| 6,003 | 277.71 | COST_REDUCING | TRIM@277.71 shares=318.0 L1ratio=0.875 |
| 6,239 | 279.25 | POSITION_OPEN | CLOSE_DIFF@279.25 profit=-489.74 |
| 6,717 | 277.20 | COST_REDUCING | TRIM@277.20 shares=318.1 L1ratio=0.875 |
| 6,819 | 277.51 | POSITION_OPEN | CLOSE_DIFF@277.51 profit=-98.63 |
| 8,024 | 281.60 | COST_REDUCING | TRIM@281.60 shares=341.3 L1ratio=0.939 |
| 8,318 | 282.83 | POSITION_OPEN | CLOSE_DIFF@282.83 profit=-419.79 |
| 9,190 | 275.99 | COST_REDUCING | TRIM@275.99 shares=291.4 L1ratio=0.802 |
| 9,483 | 276.62 | POSITION_OPEN | CLOSE_DIFF@276.62 profit=-183.61 |
| 10,516 | 282.84 | SCANNING | STOP:move_flip_trend_down@282.84 pnl=+2.79% |
| 13,661 | 292.45 | POSITION_OPEN | ENTRY_LONG@292.45 mode=TREND_UP L2dir=0 |
| 14,704 | 292.56 | COST_REDUCING | TRIM@292.56 shares=287.8 L1ratio=0.842 |
| 14,811 | 292.74 | POSITION_OPEN | CLOSE_DIFF@292.74 profit=-51.81 |
| 15,620 | 288.77 | COST_REDUCING | TRIM@288.77 shares=275.6 L1ratio=0.806 |
| 15,797 | 291.27 | POSITION_OPEN | CLOSE_DIFF@291.27 profit=-688.93 |
| 16,013 | 292.79 | SCANNING | STOP:move_flip_trend_down@292.79 pnl=+0.12% |
| 16,909 | 298.24 | POSITION_OPEN | ENTRY_LONG@298.24 mode=CONSOLIDATION L2dir=0 |
| 17,975 | 309.05 | COST_REDUCING | TRIM@309.05 shares=270.9 L1ratio=0.808 |
| 18,466 | 311.10 | POSITION_OPEN | CLOSE_DIFF@311.10 profit=-555.28 |
| 18,728 | 306.41 | COST_REDUCING | TRIM@306.41 shares=324.0 L1ratio=0.966 |
| 19,261 | 303.36 | POSITION_OPEN | CLOSE_DIFF@303.36 profit=988.06 |
| 19,318 | 304.98 | COST_REDUCING | TRIM@304.98 shares=270.9 L1ratio=0.808 |
| 19,531 | 303.91 | POSITION_OPEN | CLOSE_DIFF@303.91 profit=288.47 |
| 19,593 | 303.51 | COST_REDUCING | TRIM@303.51 shares=281.1 L1ratio=0.838 |
| 19,821 | 304.12 | POSITION_OPEN | CLOSE_DIFF@304.12 profit=-171.44 |
| 20,082 | 303.78 | COST_REDUCING | TRIM@303.78 shares=281.3 L1ratio=0.839 |
| 20,247 | 305.89 | SCANNING | STOP:move_flip_trend_down@305.89 pnl=+3.84% |
| 27,437 | 297.27 | POSITION_OPEN | ENTRY_SHORT@297.27 mode=CONSOLIDATION L2dir=0 |
| 27,859 | 294.39 | POSITION_OPEN | ADD_POS@294.39 +239.1 L1ratio=0.711 |
| 28,261 | 294.76 | POSITION_OPEN | ADD_POS@294.76 +409.2 L1ratio=0.711 |
| 28,444 | 294.14 | SCANNING | STOP:bsp_invalidate@294.14 pnl=+0.47% |
| 39,206 | 287.37 | POSITION_OPEN | ENTRY_SHORT@287.37 mode=TREND_DOWN L2dir=0 |
| 39,909 | 291.90 | COST_REDUCING | TRIM@291.90 shares=188.7 L1ratio=0.542 |
| 40,529 | 295.08 | SCANNING | STOP:move_flip_trend_up@295.08 pnl=-2.68% |
| 45,926 | 313.67 | POSITION_OPEN | ENTRY_LONG@313.67 mode=TREND_UP L2dir=1 |
| 46,025 | 309.74 | COST_REDUCING | TRIM@309.74 shares=314.6 L1ratio=0.987 |
| 46,714 | 313.12 | POSITION_OPEN | CLOSE_DIFF@313.12 profit=-1063.21 |
| 47,157 | 310.23 | COST_REDUCING | TRIM@310.23 shares=364.9 L1ratio=0.995 |
| 47,712 | 308.69 | POSITION_OPEN | CLOSE_DIFF@308.69 profit=562.00 |
| 48,006 | 311.37 | SCANNING | STOP:move_flip_trend_down@311.37 pnl=-0.22% |
| 50,895 | 315.00 | POSITION_OPEN | ENTRY_LONG@315.00 mode=CONSOLIDATION L2dir=1 |
| 51,694 | 315.66 | COST_REDUCING | TRIM@315.66 shares=315.8 L1ratio=0.995 |
| 52,243 | 321.03 | POSITION_OPEN | CLOSE_DIFF@321.03 profit=-1695.63 |
| 52,737 | 319.50 | COST_REDUCING | TRIM@319.50 shares=292.2 L1ratio=0.920 |
| 52,767 | 319.20 | POSITION_OPEN | CLOSE_DIFF@319.20 profit=87.66 |
| 52,924 | 318.36 | COST_REDUCING | TRIM@318.36 shares=292.2 L1ratio=0.920 |
| 53,510 | 321.52 | POSITION_OPEN | CLOSE_DIFF@321.52 profit=-923.34 |
| 53,767 | 319.65 | COST_REDUCING | TRIM@319.65 shares=312.3 L1ratio=0.984 |
| 54,238 | 318.54 | POSITION_OPEN | CLOSE_DIFF@318.54 profit=346.62 |
| 55,453 | 314.73 | COST_REDUCING | TRIM@314.73 shares=277.6 L1ratio=0.874 |
| 55,597 | 317.48 | POSITION_OPEN | CLOSE_DIFF@317.48 profit=-763.31 |
| 55,783 | 318.10 | SCANNING | STOP:move_flip_trend_down@318.10 pnl=+1.42% |
| 67,965 | 319.43 | POSITION_OPEN | ENTRY_LONG@319.43 mode=CONSOLIDATION L2dir=1 |
| 68,196 | 320.04 | COST_REDUCING | TRIM@320.04 shares=302.8 L1ratio=0.967 |
| 68,316 | 319.31 | POSITION_OPEN | CLOSE_DIFF@319.31 profit=221.07 |
| 68,532 | 321.28 | POSITION_OPEN | ADD_POS@321.28 +54.2 L1ratio=0.173 |
| 68,893 | 322.36 | COST_REDUCING | TRIM@322.36 shares=362.9 L1ratio=0.988 |
| 69,320 | 322.04 | POSITION_OPEN | CLOSE_DIFF@322.04 profit=116.14 |
| 69,405 | 322.46 | COST_REDUCING | TRIM@322.46 shares=363.1 L1ratio=0.989 |
| 69,827 | 322.03 | POSITION_OPEN | CLOSE_DIFF@322.03 profit=156.11 |
| 71,205 | 316.80 | COST_REDUCING | TRIM@316.80 shares=358.8 L1ratio=0.977 |
| 71,802 | 317.60 | POSITION_OPEN | CLOSE_DIFF@317.60 profit=-287.06 |
| 72,003 | 316.08 | SCANNING | STOP:move_flip_trend_down@316.08 pnl=-0.71% |
| 77,960 | 327.57 | POSITION_OPEN | ENTRY_LONG@327.57 mode=TREND_UP L2dir=1 |
| 77,981 | 327.22 | COST_REDUCING | TRIM@327.22 shares=303.5 L1ratio=0.994 |
| 78,031 | 327.75 | POSITION_OPEN | CLOSE_DIFF@327.75 profit=-160.88 |
| 78,440 | 327.44 | COST_REDUCING | TRIM@327.44 shares=299.2 L1ratio=0.980 |
| 78,534 | 327.64 | POSITION_OPEN | CLOSE_DIFF@327.64 profit=-59.84 |
| 78,826 | 328.60 | COST_REDUCING | TRIM@328.60 shares=302.3 L1ratio=0.990 |
| 79,420 | 332.09 | POSITION_OPEN | CLOSE_DIFF@332.09 profit=-1055.20 |
| 80,484 | 336.69 | COST_REDUCING | TRIM@336.69 shares=299.5 L1ratio=0.981 |
| 81,188 | 336.32 | POSITION_OPEN | CLOSE_DIFF@336.32 profit=110.80 |
| 82,391 | 333.84 | COST_REDUCING | TRIM@333.84 shares=296.0 L1ratio=0.970 |
| 82,996 | 330.63 | SCANNING | STOP:move_flip_trend_down@330.63 pnl=+1.04% |
| 84,038 | 340.69 | POSITION_OPEN | ENTRY_LONG@340.69 mode=TREND_UP L2dir=1 |
| 85,792 | 351.01 | COST_REDUCING | TRIM@351.01 shares=277.9 L1ratio=0.947 |
| 86,834 | 347.42 | SCANNING | STOP:move_flip_trend_down@347.42 pnl=+1.98% |
| 93,055 | 357.62 | POSITION_OPEN | ENTRY_LONG@357.62 mode=TREND_UP L2dir=1 |
| 93,739 | 361.71 | COST_REDUCING | TRIM@361.71 shares=261.0 L1ratio=0.933 |
| 93,922 | 363.24 | POSITION_OPEN | CLOSE_DIFF@363.24 profit=-399.35 |
| 94,011 | 362.72 | COST_REDUCING | TRIM@362.72 shares=277.2 L1ratio=0.991 |
| 94,978 | 366.34 | POSITION_OPEN | CLOSE_DIFF@366.34 profit=-1003.51 |
| 95,575 | 368.10 | COST_REDUCING | TRIM@368.10 shares=277.9 L1ratio=0.994 |
| 95,754 | 370.23 | POSITION_OPEN | CLOSE_DIFF@370.23 profit=-591.86 |
| 95,904 | 369.83 | COST_REDUCING | TRIM@369.83 shares=268.3 L1ratio=0.960 |
| 96,071 | 370.83 | POSITION_OPEN | CLOSE_DIFF@370.83 profit=-268.34 |
| 96,272 | 371.02 | COST_REDUCING | TRIM@371.02 shares=267.2 L1ratio=0.956 |
| ... | ... | ... | （共421条，显示前100） |
</details>

## OKLO

- 数据：**333,613** bars (1min)
- 价格：18.07 → 63.48
- Buy-and-hold: **+251.30%**
- 回测耗时：72.2s

### Move 类型统计

| 指标 | 值 | 占比 |
|------|-----|------|
| 模式切换次数 | 376 | — |
| TREND_UP bars | 113,891 | 34.1% |
| TREND_DOWN bars | 106,633 | 32.0% |
| CONSOLIDATION bars | 112,428 | 33.7% |
| UNKNOWN bars | 661 | 0.2% |

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
| 交易数 | 17 (多10/空7) |
| 胜率 | 29.4% |
| 平均收益 | -2.405% |
| 复利累计 | -61.49% |
| 最大回撤 | -71.68% |
| 平均持仓 | 3,002 bars |
| 有加仓的交易 | 7/17 |
| 有降成本的交易 | 14/17 |
| 达到本金回收 | 0/17 |

### 按 Move 类型分组

| 类型 | 交易数 | 多/空 | 胜率 | 平均PnL | 复利% |
|------|--------|------|------|---------|------|
| CONSOLIDATION | 8 | 7/1 | 38% | +2.334% | +1.63% |
| TREND_DOWN | 6 | 0/6 | 0% | -19.633% | -76.82% |
| TREND_UP | 3 | 3/0 | 67% | +19.413% | +63.48% |

### 交易明细

| # | 方向 | 入场 | 出场 | bars | PnL% | Move类型 | 加仓 | 短差 | PW | 退出原因 |
|---|------|------|------|------|------|---------|------|------|----|---------| 
| 1 | 空 | 9.11@940 | 10.80@2170 | 1,230 | -24.87 | CONSOLIDATION | 0 | 1 | 否 | move_flip_trend_up |
| 2 | 空 | 7.12@4025 | 9.06@7214 | 3,189 | -53.03 | TREND_DOWN | 1 | 2 | 否 | bsp_invalidate |
| 3 | 空 | 5.90@29391 | 6.59@32883 | 3,492 | -19.76 | TREND_DOWN | 0 | 3 | 否 | move_flip_trend_up |
| 4 | 多 | 13.08@41091 | 19.14@46744 | 5,653 | +51.67 | CONSOLIDATION | 0 | 4 | 否 | move_flip_trend_down |
| 5 | 多 | 24.60@47161 | 20.36@50507 | 3,346 | -15.03 | CONSOLIDATION | 0 | 2 | 否 | move_flip_trend_down |
| 6 | 多 | 28.02@53130 | 23.40@54705 | 1,575 | -10.54 | CONSOLIDATION | 1 | 2 | 否 | move_flip_trend_down |
| 7 | 多 | 31.98@82271 | 34.20@86342 | 4,071 | +13.61 | TREND_UP | 0 | 3 | 否 | move_flip_trend_down |
| 8 | 多 | 46.11@89133 | 54.69@94200 | 5,067 | +17.18 | CONSOLIDATION | 2 | 5 | 否 | bsp_invalidate |
| 9 | 多 | 66.80@151334 | 65.64@151540 | 206 | -1.74 | CONSOLIDATION | 0 | 0 | 否 | bsp_invalidate |
| 10 | 多 | 76.58@173692 | 75.26@176521 | 2,829 | -1.58 | TREND_UP | 1 | 4 | 否 | move_flip_trend_down |
| 11 | 多 | 78.49@177102 | 76.59@178793 | 1,691 | -2.42 | CONSOLIDATION | 0 | 0 | 否 | move_flip_trend_down |
| 12 | 多 | 88.82@199494 | 111.53@207278 | 7,784 | +46.21 | TREND_UP | 0 | 6 | 否 | move_flip_trend_down |
| 13 | 多 | 146.72@215837 | 154.48@219699 | 3,862 | +4.41 | CONSOLIDATION | 1 | 2 | 否 | move_flip_trend_down |
| 14 | 空 | 96.88@235756 | 104.29@236068 | 312 | -7.65 | TREND_DOWN | 0 | 0 | 否 | bsp_invalidate |
| 15 | 空 | 74.94@255587 | 87.80@258571 | 2,984 | -18.44 | TREND_DOWN | 2 | 1 | 否 | move_flip_trend_up |
| 16 | 空 | 61.02@275312 | 70.30@277398 | 2,086 | -15.21 | TREND_DOWN | 0 | 2 | 否 | move_flip_trend_up |
| 17 | 空 | 50.22@298313 | 51.23@299970 | 1,657 | -3.71 | TREND_DOWN | 2 | 1 | 否 | move_flip_trend_up |

### FSM 状态分布

| 状态 | Bars | 占比 |
|------|------|------|
| SCANNING | 282,579 | 84.7% |
| POSITION_OPEN | 26,545 | 8.0% |
| COST_REDUCING | 24,489 | 7.3% |
| PRINCIPAL_WITHDRAWN | 0 | 0.0% |
| STOPPED_OUT | 0 | 0.0% |

<details><summary>事件流（128条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 940 | 9.11 | POSITION_OPEN | ENTRY_SHORT@9.11 mode=CONSOLIDATION L2dir=0 |
| 974 | 8.86 | COST_REDUCING | TRIM@8.86 shares=6718.0 L1ratio=0.612 |
| 1,451 | 9.80 | POSITION_OPEN | CLOSE_DIFF@9.80 profit=6314.89 |
| 1,510 | 10.19 | COST_REDUCING | TRIM@10.19 shares=9777.0 L1ratio=0.891 |
| 2,170 | 10.80 | SCANNING | STOP:move_flip_trend_up@10.80 pnl=-24.87% |
| 4,025 | 7.12 | POSITION_OPEN | ENTRY_SHORT@7.12 mode=TREND_DOWN L2dir=0 |
| 4,581 | 7.54 | COST_REDUCING | TRIM@7.54 shares=13072.7 L1ratio=0.931 |
| 6,177 | 11.16 | POSITION_OPEN | CLOSE_DIFF@11.16 profit=47323.14 |
| 6,343 | 11.15 | COST_REDUCING | TRIM@11.15 shares=13072.7 L1ratio=0.931 |
| 6,637 | 10.41 | POSITION_OPEN | CLOSE_DIFF@10.41 profit=-9673.79 |
| 7,192 | 9.18 | POSITION_OPEN | ADD_POS@9.18 +5529.0 L1ratio=0.394 |
| 7,214 | 9.06 | SCANNING | STOP:bsp_invalidate@9.06 pnl=-53.03% |
| 29,391 | 5.90 | POSITION_OPEN | ENTRY_SHORT@5.90 mode=TREND_DOWN L2dir=0 |
| 30,129 | 5.83 | COST_REDUCING | TRIM@5.83 shares=16441.1 L1ratio=0.970 |
| 30,579 | 5.60 | POSITION_OPEN | CLOSE_DIFF@5.60 profit=-3863.65 |
| 31,265 | 6.00 | COST_REDUCING | TRIM@6.00 shares=16441.1 L1ratio=0.970 |
| 31,505 | 6.09 | POSITION_OPEN | CLOSE_DIFF@6.09 profit=1397.49 |
| 31,963 | 6.21 | COST_REDUCING | TRIM@6.21 shares=16064.7 L1ratio=0.948 |
| 32,730 | 6.62 | POSITION_OPEN | CLOSE_DIFF@6.62 profit=6666.84 |
| 32,883 | 6.59 | SCANNING | STOP:move_flip_trend_up@6.59 pnl=-19.76% |
| 41,091 | 13.08 | POSITION_OPEN | ENTRY_LONG@13.08 mode=CONSOLIDATION L2dir=1 |
| 41,862 | 16.22 | COST_REDUCING | TRIM@16.22 shares=6887.0 L1ratio=0.901 |
| 42,565 | 15.84 | POSITION_OPEN | CLOSE_DIFF@15.84 profit=2617.04 |
| 43,016 | 18.46 | COST_REDUCING | TRIM@18.46 shares=6819.9 L1ratio=0.892 |
| 43,450 | 19.91 | POSITION_OPEN | CLOSE_DIFF@19.91 profit=-9888.86 |
| 44,516 | 20.63 | COST_REDUCING | TRIM@20.63 shares=7044.4 L1ratio=0.921 |
| 44,992 | 20.44 | POSITION_OPEN | CLOSE_DIFF@20.44 profit=1338.44 |
| 45,375 | 18.33 | COST_REDUCING | TRIM@18.33 shares=6593.8 L1ratio=0.862 |
| 45,777 | 18.12 | POSITION_OPEN | CLOSE_DIFF@18.12 profit=1384.70 |
| 45,878 | 17.97 | COST_REDUCING | TRIM@17.97 shares=6593.8 L1ratio=0.862 |
| 46,744 | 19.14 | SCANNING | STOP:move_flip_trend_down@19.14 pnl=+51.67% |
| 47,161 | 24.60 | POSITION_OPEN | ENTRY_LONG@24.60 mode=CONSOLIDATION L2dir=1 |
| 47,960 | 23.43 | COST_REDUCING | TRIM@23.43 shares=3919.3 L1ratio=0.964 |
| 48,481 | 26.75 | POSITION_OPEN | CLOSE_DIFF@26.75 profit=-13012.23 |
| 48,715 | 23.60 | COST_REDUCING | TRIM@23.60 shares=3612.8 L1ratio=0.889 |
| 49,561 | 22.99 | POSITION_OPEN | CLOSE_DIFF@22.99 profit=2203.78 |
| 49,609 | 22.92 | COST_REDUCING | TRIM@22.92 shares=3612.8 L1ratio=0.889 |
| 50,507 | 20.36 | SCANNING | STOP:move_flip_trend_down@20.36 pnl=-15.03% |
| 53,130 | 28.02 | POSITION_OPEN | ENTRY_LONG@28.02 mode=CONSOLIDATION L2dir=1 |
| 53,314 | 26.42 | POSITION_OPEN | ADD_POS@26.42 +1252.1 L1ratio=0.351 |
| 53,605 | 24.94 | COST_REDUCING | TRIM@24.94 shares=4790.7 L1ratio=0.994 |
| 53,907 | 24.36 | POSITION_OPEN | CLOSE_DIFF@24.36 profit=2778.59 |
| 53,993 | 24.40 | COST_REDUCING | TRIM@24.40 shares=4790.7 L1ratio=0.994 |
| 54,119 | 23.72 | POSITION_OPEN | CLOSE_DIFF@23.72 profit=3257.66 |
| 54,228 | 23.83 | COST_REDUCING | TRIM@23.83 shares=4790.7 L1ratio=0.994 |
| 54,705 | 23.40 | SCANNING | STOP:move_flip_trend_down@23.40 pnl=-10.54% |
| 82,271 | 31.98 | POSITION_OPEN | ENTRY_LONG@31.98 mode=TREND_UP L2dir=1 |
| 83,207 | 32.37 | COST_REDUCING | TRIM@32.37 shares=2760.5 L1ratio=0.883 |
| 83,593 | 38.78 | POSITION_OPEN | CLOSE_DIFF@38.78 profit=-17694.90 |
| 84,429 | 42.84 | COST_REDUCING | TRIM@42.84 shares=2706.0 L1ratio=0.865 |
| 84,463 | 42.52 | POSITION_OPEN | CLOSE_DIFF@42.52 profit=865.91 |
| 84,755 | 35.90 | COST_REDUCING | TRIM@35.90 shares=2449.6 L1ratio=0.783 |
| 86,173 | 33.53 | POSITION_OPEN | CLOSE_DIFF@33.53 profit=5805.65 |
| 86,342 | 34.20 | SCANNING | STOP:move_flip_trend_down@34.20 pnl=+13.61% |
| 89,133 | 46.11 | POSITION_OPEN | ENTRY_LONG@46.11 mode=CONSOLIDATION L2dir=1 |
| 89,466 | 46.55 | COST_REDUCING | TRIM@46.55 shares=2140.4 L1ratio=0.987 |
| 90,008 | 45.95 | POSITION_OPEN | CLOSE_DIFF@45.95 profit=1284.22 |
| 91,119 | 48.95 | COST_REDUCING | TRIM@48.95 shares=1940.2 L1ratio=0.895 |
| 91,475 | 48.63 | POSITION_OPEN | CLOSE_DIFF@48.63 profit=611.16 |
| 92,285 | 54.89 | COST_REDUCING | TRIM@54.89 shares=2104.1 L1ratio=0.970 |
| 92,312 | 55.20 | POSITION_OPEN | CLOSE_DIFF@55.20 profit=-652.26 |
| 92,466 | 54.30 | COST_REDUCING | TRIM@54.30 shares=2104.1 L1ratio=0.970 |
| 93,016 | 52.38 | POSITION_OPEN | CLOSE_DIFF@52.38 profit=4029.26 |
| 93,454 | 50.88 | COST_REDUCING | TRIM@50.88 shares=1967.2 L1ratio=0.907 |
| 93,772 | 52.73 | POSITION_OPEN | CLOSE_DIFF@52.73 profit=-3629.55 |
| 93,903 | 54.91 | POSITION_OPEN | ADD_POS@54.91 +426.2 L1ratio=0.197 |
| 94,086 | 54.37 | POSITION_OPEN | ADD_POS@54.37 +510.0 L1ratio=0.197 |
| 94,200 | 54.69 | SCANNING | STOP:bsp_invalidate@54.69 pnl=+17.18% |
| 151,334 | 66.80 | POSITION_OPEN | ENTRY_LONG@66.80 mode=CONSOLIDATION L2dir=1 |
| 151,540 | 65.64 | SCANNING | STOP:bsp_invalidate@65.64 pnl=-1.74% |
| 173,692 | 76.58 | POSITION_OPEN | ENTRY_LONG@76.58 mode=TREND_UP L2dir=1 |
| 173,991 | 72.22 | COST_REDUCING | TRIM@72.22 shares=1206.1 L1ratio=0.924 |
| 174,553 | 75.00 | POSITION_OPEN | CLOSE_DIFF@75.00 profit=-3352.83 |
| 175,060 | 74.46 | COST_REDUCING | TRIM@74.46 shares=1361.2 L1ratio=0.971 |
| 175,622 | 75.20 | POSITION_OPEN | CLOSE_DIFF@75.20 profit=-1007.32 |
| 176,091 | 71.38 | COST_REDUCING | TRIM@71.38 shares=1311.2 L1ratio=0.935 |
| 176,122 | 71.57 | POSITION_OPEN | CLOSE_DIFF@71.57 profit=-249.12 |
| 176,234 | 71.40 | COST_REDUCING | TRIM@71.40 shares=1311.2 L1ratio=0.935 |
| 176,398 | 75.80 | POSITION_OPEN | CLOSE_DIFF@75.80 profit=-5762.53 |
| 176,521 | 75.26 | SCANNING | STOP:move_flip_trend_down@75.26 pnl=-1.58% |
| 177,102 | 78.49 | POSITION_OPEN | ENTRY_LONG@78.49 mode=CONSOLIDATION L2dir=1 |
| 177,478 | 76.18 | COST_REDUCING | TRIM@76.18 shares=1242.1 L1ratio=0.975 |
| 178,793 | 76.59 | SCANNING | STOP:move_flip_trend_down@76.59 pnl=-2.42% |
| 199,494 | 88.82 | POSITION_OPEN | ENTRY_LONG@88.82 mode=TREND_UP L2dir=1 |
| 200,406 | 91.20 | COST_REDUCING | TRIM@91.20 shares=1063.8 L1ratio=0.945 |
| 200,579 | 94.58 | POSITION_OPEN | CLOSE_DIFF@94.58 profit=-3595.50 |
| 201,126 | 94.68 | COST_REDUCING | TRIM@94.68 shares=1125.1 L1ratio=0.999 |
| 201,832 | 100.44 | POSITION_OPEN | CLOSE_DIFF@100.44 profit=-6480.74 |
| 203,456 | 131.41 | COST_REDUCING | TRIM@131.41 shares=1089.3 L1ratio=0.968 |
| 203,886 | 138.20 | POSITION_OPEN | CLOSE_DIFF@138.20 profit=-7396.29 |
| 204,334 | 137.28 | COST_REDUCING | TRIM@137.28 shares=1079.9 L1ratio=0.959 |
| 205,165 | 140.50 | POSITION_OPEN | CLOSE_DIFF@140.50 profit=-3477.22 |
| 205,261 | 136.24 | COST_REDUCING | TRIM@136.24 shares=1114.5 L1ratio=0.990 |
| 205,442 | 138.40 | POSITION_OPEN | CLOSE_DIFF@138.40 profit=-2407.35 |
| 205,518 | 136.88 | COST_REDUCING | TRIM@136.88 shares=1114.5 L1ratio=0.990 |
| 207,032 | 118.36 | POSITION_OPEN | CLOSE_DIFF@118.36 profit=20640.75 |
| 207,189 | 109.70 | COST_REDUCING | TRIM@109.70 shares=948.2 L1ratio=0.842 |
| 207,278 | 111.53 | SCANNING | STOP:move_flip_trend_down@111.53 pnl=+46.21% |
| 215,837 | 146.72 | POSITION_OPEN | ENTRY_LONG@146.72 mode=CONSOLIDATION L2dir=1 |
| 215,927 | 147.00 | COST_REDUCING | TRIM@147.00 shares=609.9 L1ratio=0.895 |
| ... | ... | ... | （共128条，显示前100） |
</details>

## HK700

- 数据：**1,408,882** bars (1min)
- 价格：11.36 → 458.20
- Buy-and-hold: **+3933.45%**
- 回测耗时：519.9s

### Move 类型统计

| 指标 | 值 | 占比 |
|------|-----|------|
| 模式切换次数 | 616 | — |
| TREND_UP bars | 608,302 | 43.2% |
| TREND_DOWN bars | 376,785 | 26.7% |
| CONSOLIDATION bars | 418,836 | 29.7% |
| UNKNOWN bars | 4,959 | 0.4% |

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
| 交易数 | 24 (多20/空4) |
| 胜率 | 58.3% |
| 平均收益 | +4.585% |
| 复利累计 | +133.76% |
| 最大回撤 | -28.45% |
| 平均持仓 | 14,199 bars |
| 有加仓的交易 | 12/24 |
| 有降成本的交易 | 22/24 |
| 达到本金回收 | 0/24 |

### 按 Move 类型分组

| 类型 | 交易数 | 多/空 | 胜率 | 平均PnL | 复利% |
|------|--------|------|------|---------|------|
| CONSOLIDATION | 14 | 11/3 | 57% | +4.296% | +57.49% |
| TREND_DOWN | 1 | 0/1 | 0% | -20.652% | -20.65% |
| TREND_UP | 9 | 9/0 | 67% | +7.838% | +87.07% |

### 交易明细

| # | 方向 | 入场 | 出场 | bars | PnL% | Move类型 | 加仓 | 短差 | PW | 退出原因 |
|---|------|------|------|------|------|---------|------|------|----|---------| 
| 1 | 多 | 12.50@5094 | 10.86@12823 | 7,729 | -9.83 | CONSOLIDATION | 1 | 1 | 否 | bsp_invalidate |
| 2 | 空 | 7.70@18094 | 8.62@29451 | 11,357 | -20.65 | TREND_DOWN | 1 | 7 | 否 | move_flip_trend_up |
| 3 | 多 | 13.82@53073 | 16.67@63471 | 10,398 | +22.52 | TREND_UP | 1 | 4 | 否 | bsp_invalidate |
| 4 | 多 | 17.66@64573 | 25.95@96465 | 31,892 | +48.64 | CONSOLIDATION | 0 | 10 | 否 | move_flip_trend_down |
| 5 | 多 | 34.30@138285 | 32.40@147449 | 9,164 | +0.31 | TREND_UP | 0 | 2 | 否 | move_flip_trend_down |
| 6 | 多 | 40.80@162024 | 35.55@165080 | 3,056 | -5.70 | CONSOLIDATION | 0 | 1 | 否 | move_flip_trend_down |
| 7 | 多 | 50.50@289428 | 46.70@326792 | 37,364 | +3.60 | CONSOLIDATION | 1 | 4 | 否 | move_flip_trend_down |
| 8 | 多 | 64.90@348935 | 82.80@377587 | 28,652 | +26.16 | TREND_UP | 2 | 4 | 否 | bsp_invalidate |
| 9 | 多 | 87.50@378891 | 91.65@409422 | 30,531 | +18.19 | TREND_UP | 1 | 9 | 否 | move_flip_trend_down |
| 10 | 多 | 119.10@428314 | 112.75@438678 | 10,364 | -3.90 | CONSOLIDATION | 0 | 3 | 否 | move_flip_trend_down |
| 11 | 多 | 119.30@453754 | 109.15@457634 | 3,880 | -8.51 | TREND_UP | 0 | 0 | 否 | move_flip_trend_down |
| 12 | 多 | 124.45@468633 | 149.35@501851 | 33,218 | +24.58 | CONSOLIDATION | 4 | 15 | 否 | move_flip_trend_down |
| 13 | 多 | 158.80@576773 | 173.50@592345 | 15,572 | +11.01 | CONSOLIDATION | 0 | 5 | 否 | move_flip_trend_down |
| 14 | 多 | 187.85@594983 | 171.00@622111 | 27,128 | -8.97 | CONSOLIDATION | 0 | 7 | 否 | move_flip_trend_down |
| 15 | 多 | 222.90@652085 | 241.50@667774 | 15,689 | +10.05 | TREND_UP | 0 | 5 | 否 | move_flip_trend_down |
| 16 | 多 | 277.10@672227 | 304.40@681887 | 9,660 | +9.85 | CONSOLIDATION | 0 | 3 | 否 | move_flip_trend_down |
| 17 | 多 | 346.40@696868 | 357.10@703430 | 6,562 | +3.53 | CONSOLIDATION | 0 | 2 | 否 | move_flip_trend_down |
| 18 | 多 | 404.70@710943 | 385.50@718144 | 7,201 | -4.74 | TREND_UP | 0 | 3 | 否 | move_flip_trend_down |
| 19 | 空 | 278.70@773351 | 279.30@785525 | 12,174 | -11.60 | CONSOLIDATION | 3 | 4 | 否 | move_flip_trend_up |
| 20 | 多 | 487.60@917774 | 490.00@922761 | 4,987 | +0.29 | CONSOLIDATION | 1 | 5 | 否 | bsp_invalidate |
| 21 | 多 | 552.20@944407 | 541.20@954142 | 9,735 | -1.57 | TREND_UP | 0 | 3 | 否 | move_flip_trend_down |
| 22 | 多 | 583.60@962507 | 634.80@971387 | 8,880 | +8.14 | TREND_UP | 1 | 4 | 否 | move_flip_trend_down |
| 23 | 空 | 380.50@1056649 | 366.20@1057460 | 811 | +1.55 | CONSOLIDATION | 2 | 0 | 否 | bsp_invalidate |
| 24 | 空 | 218.80@1109328 | 219.80@1114101 | 4,773 | -2.91 | CONSOLIDATION | 1 | 3 | 否 | move_flip_trend_up |

### FSM 状态分布

| 状态 | Bars | 占比 |
|------|------|------|
| SCANNING | 1,068,105 | 75.8% |
| POSITION_OPEN | 196,628 | 14.0% |
| COST_REDUCING | 144,149 | 10.2% |
| PRINCIPAL_WITHDRAWN | 0 | 0.0% |
| STOPPED_OUT | 0 | 0.0% |

<details><summary>事件流（286条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 5,094 | 12.50 | POSITION_OPEN | ENTRY_LONG@12.50 mode=CONSOLIDATION L2dir=0 |
| 7,628 | 12.72 | COST_REDUCING | TRIM@12.72 shares=6616.5 L1ratio=0.827 |
| 10,757 | 12.00 | POSITION_OPEN | CLOSE_DIFF@12.00 profit=4763.91 |
| 11,999 | 12.36 | POSITION_OPEN | ADD_POS@12.36 +5413.5 L1ratio=0.677 |
| 12,823 | 10.86 | SCANNING | STOP:bsp_invalidate@10.86 pnl=-9.83% |
| 18,094 | 7.70 | POSITION_OPEN | ENTRY_SHORT@7.70 mode=TREND_DOWN L2dir=0 |
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
| 29,451 | 8.62 | SCANNING | STOP:move_flip_trend_up@8.62 pnl=-20.65% |
| 53,073 | 13.82 | POSITION_OPEN | ENTRY_LONG@13.82 mode=TREND_UP L2dir=0 |
| 53,473 | 13.04 | COST_REDUCING | TRIM@13.04 shares=6682.3 L1ratio=0.923 |
| 54,235 | 14.56 | POSITION_OPEN | CLOSE_DIFF@14.56 profit=-10157.13 |
| 55,587 | 14.56 | COST_REDUCING | TRIM@14.56 shares=6892.1 L1ratio=0.952 |
| 56,505 | 15.84 | POSITION_OPEN | CLOSE_DIFF@15.84 profit=-8821.89 |
| 58,754 | 16.48 | COST_REDUCING | TRIM@16.48 shares=6725.0 L1ratio=0.929 |
| 60,480 | 15.72 | POSITION_OPEN | CLOSE_DIFF@15.72 profit=5110.97 |
| 62,111 | 16.36 | COST_REDUCING | TRIM@16.36 shares=7018.0 L1ratio=0.970 |
| 62,678 | 16.41 | POSITION_OPEN | CLOSE_DIFF@16.41 profit=-350.90 |
| 63,288 | 16.60 | POSITION_OPEN | ADD_POS@16.60 +1056.3 L1ratio=0.146 |
| 63,471 | 16.67 | SCANNING | STOP:bsp_invalidate@16.67 pnl=+22.52% |
| 64,573 | 17.66 | POSITION_OPEN | ENTRY_LONG@17.66 mode=CONSOLIDATION L2dir=0 |
| 65,538 | 18.71 | COST_REDUCING | TRIM@18.71 shares=5375.9 L1ratio=0.949 |
| 65,703 | 18.95 | POSITION_OPEN | CLOSE_DIFF@18.95 profit=-1290.22 |
| 68,114 | 19.91 | COST_REDUCING | TRIM@19.91 shares=5615.0 L1ratio=0.992 |
| 68,448 | 20.26 | POSITION_OPEN | CLOSE_DIFF@20.26 profit=-1965.25 |
| 69,417 | 21.66 | COST_REDUCING | TRIM@21.66 shares=5462.3 L1ratio=0.965 |
| 69,585 | 21.35 | POSITION_OPEN | CLOSE_DIFF@21.35 profit=1693.31 |
| 71,182 | 21.90 | COST_REDUCING | TRIM@21.90 shares=5023.2 L1ratio=0.887 |
| 75,196 | 23.80 | POSITION_OPEN | CLOSE_DIFF@23.80 profit=-9544.08 |
| 76,985 | 23.15 | COST_REDUCING | TRIM@23.15 shares=5128.1 L1ratio=0.906 |
| 79,001 | 23.80 | POSITION_OPEN | CLOSE_DIFF@23.80 profit=-3333.25 |
| 82,153 | 25.65 | COST_REDUCING | TRIM@25.65 shares=4688.0 L1ratio=0.828 |
| 83,494 | 26.15 | POSITION_OPEN | CLOSE_DIFF@26.15 profit=-2344.02 |
| 85,580 | 26.85 | COST_REDUCING | TRIM@26.85 shares=5610.3 L1ratio=0.991 |
| 86,225 | 27.10 | POSITION_OPEN | CLOSE_DIFF@27.10 profit=-1402.58 |
| 87,012 | 26.60 | COST_REDUCING | TRIM@26.60 shares=5610.3 L1ratio=0.991 |
| 90,585 | 27.95 | POSITION_OPEN | CLOSE_DIFF@27.95 profit=-7573.94 |
| 91,098 | 28.35 | COST_REDUCING | TRIM@28.35 shares=5206.6 L1ratio=0.919 |
| 91,480 | 29.75 | POSITION_OPEN | CLOSE_DIFF@29.75 profit=-7289.29 |
| 93,216 | 31.05 | COST_REDUCING | TRIM@31.05 shares=5283.5 L1ratio=0.933 |
| 94,600 | 31.85 | POSITION_OPEN | CLOSE_DIFF@31.85 profit=-4226.82 |
| 95,432 | 29.10 | COST_REDUCING | TRIM@29.10 shares=5130.0 L1ratio=0.906 |
| 96,465 | 25.95 | SCANNING | STOP:move_flip_trend_down@25.95 pnl=+48.64% |
| 138,285 | 34.30 | POSITION_OPEN | ENTRY_LONG@34.30 mode=TREND_UP L2dir=0 |
| 140,700 | 33.65 | COST_REDUCING | TRIM@33.65 shares=2915.5 L1ratio=1.000 |
| 142,624 | 33.25 | POSITION_OPEN | CLOSE_DIFF@33.25 profit=1166.18 |
| 143,011 | 33.35 | COST_REDUCING | TRIM@33.35 shares=2753.2 L1ratio=0.944 |
| 144,562 | 31.65 | POSITION_OPEN | CLOSE_DIFF@31.65 profit=4680.43 |
| 145,776 | 30.80 | COST_REDUCING | TRIM@30.80 shares=2727.0 L1ratio=0.935 |
| 147,449 | 32.40 | SCANNING | STOP:move_flip_trend_down@32.40 pnl=+0.31% |
| 162,024 | 40.80 | POSITION_OPEN | ENTRY_LONG@40.80 mode=CONSOLIDATION L2dir=0 |
| 163,408 | 39.95 | COST_REDUCING | TRIM@39.95 shares=2273.9 L1ratio=0.928 |
| 164,411 | 36.80 | POSITION_OPEN | CLOSE_DIFF@36.80 profit=7162.87 |
| 164,907 | 35.45 | COST_REDUCING | TRIM@35.45 shares=2260.0 L1ratio=0.922 |
| 165,080 | 35.55 | SCANNING | STOP:move_flip_trend_down@35.55 pnl=-5.70% |
| 289,428 | 50.50 | POSITION_OPEN | ENTRY_LONG@50.50 mode=CONSOLIDATION L2dir=1 |
| 291,912 | 49.70 | COST_REDUCING | TRIM@49.70 shares=1958.0 L1ratio=0.989 |
| 302,478 | 45.90 | POSITION_OPEN | CLOSE_DIFF@45.90 profit=7440.39 |
| 307,353 | 49.60 | POSITION_OPEN | ADD_POS@49.60 +284.2 L1ratio=0.143 |
| 309,259 | 49.20 | COST_REDUCING | TRIM@49.20 shares=2218.7 L1ratio=0.980 |
| 309,924 | 50.00 | POSITION_OPEN | CLOSE_DIFF@50.00 profit=-1774.93 |
| 315,344 | 48.60 | COST_REDUCING | TRIM@48.60 shares=2249.2 L1ratio=0.993 |
| 316,732 | 51.30 | POSITION_OPEN | CLOSE_DIFF@51.30 profit=-6072.90 |
| 320,109 | 48.50 | COST_REDUCING | TRIM@48.50 shares=2185.9 L1ratio=0.965 |
| 324,400 | 46.20 | POSITION_OPEN | CLOSE_DIFF@46.20 profit=5027.65 |
| 326,792 | 46.70 | SCANNING | STOP:move_flip_trend_down@46.70 pnl=+3.60% |
| 348,935 | 64.90 | POSITION_OPEN | ENTRY_LONG@64.90 mode=TREND_UP L2dir=1 |
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
| 378,891 | 87.50 | POSITION_OPEN | ENTRY_LONG@87.50 mode=TREND_UP L2dir=1 |
| 379,838 | 87.00 | COST_REDUCING | TRIM@87.00 shares=1133.1 L1ratio=0.991 |
| 380,024 | 87.20 | POSITION_OPEN | CLOSE_DIFF@87.20 profit=-226.62 |
| 381,251 | 85.80 | COST_REDUCING | TRIM@85.80 shares=1078.7 L1ratio=0.944 |
| 383,149 | 92.00 | POSITION_OPEN | CLOSE_DIFF@92.00 profit=-6688.22 |
| 383,351 | 92.70 | COST_REDUCING | TRIM@92.70 shares=1082.2 L1ratio=0.947 |
| 385,176 | 90.20 | POSITION_OPEN | CLOSE_DIFF@90.20 profit=2705.38 |
| 385,518 | 92.10 | COST_REDUCING | TRIM@92.10 shares=1082.2 L1ratio=0.947 |
| 386,478 | 97.20 | POSITION_OPEN | CLOSE_DIFF@97.20 profit=-5518.97 |
| ... | ... | ... | （共286条，显示前100） |
</details>

## 四版对比

| 标的 | 版本 | 复利% | 胜率 | 交易数 | 最大回撤 |
|------|------|------|------|--------|---------|
| QQQ | 1. 单层PH（旧H组） | 全亏 | — | — | — |
| QQQ | 2. 多级别PH | +58% | — | — | — |
| QQQ | 3. regime filter（无效） | +18% | — | — | — |
| QQQ | **4. 走势类型过滤** | **+16.58%** | 50% | 44 | -7.7% |
| OKLO | 1. 单层PH（旧H组） | 全亏 | — | — | — |
| OKLO | 2. 多级别PH | +72% | — | — | — |
| OKLO | 3. regime filter（无效） | -86% | — | — | — |
| OKLO | **4. 走势类型过滤** | **-61.49%** | 29% | 17 | -71.7% |
| HK700 | 1. 单层PH（旧H组） | 全亏 | — | — | — |
| HK700 | 2. 多级别PH | +223% | — | — | — |
| HK700 | 3. regime filter（无效） | +6% | — | — | — |
| HK700 | **4. 走势类型过滤** | **+133.76%** | 58% | 24 | -28.4% |

## 汇总

| 标的 | Bars | BH% | 复利% | 胜率 | 交易数 | 模式切换 | 趋势占比 | 盘整占比 | 耗时 |
|------|------|-----|------|------|--------|---------|---------|---------|------|
| QQQ | 728,030 | +174.6 | +16.58 | 50% | 44 | 797 | 69% | 31% | 365s |
| OKLO | 333,613 | +251.3 | -61.49 | 29% | 17 | 376 | 66% | 34% | 72s |
| HK700 | 1,408,882 | +3933.5 | +133.76 | 58% | 24 | 616 | 70% | 30% | 520s |

## 结果包六要素

**结论**：走势类型过滤回测 — 用交易级别（L1）Move.kind 区分操盘策略。

**定义依据**：
- Move.kind = trend（2+ 中枢递升/递降）/ consolidation（1 中枢）— `a_move_v1.py`
- Move.direction = up/down — 趋势方向由中枢固定区间递升/递降决定
- 规则：trend_up→只多, trend_down→只空, consolidation→双向
- 止损：持仓方向与 Move 趋势方向矛盾时平仓；盘整不触发方向止损

**边界条件**：
- PENDING_EXPIRY = 390 bars
- Move 类型在线段/中枢更新时变化 → 切换频率取决于 L1 结构的稳定性
- consolidation 的中枢范围（zg_max, zd_min）已提取但本版未用于仓位管理
- 无滑点/手续费建模

**下游推论**：
- 若走势类型过滤优于多级别PH → Move.kind 是有效的操盘过滤器
- 若 TREND_UP 交易胜率高于 CONSOLIDATION → 趋势跟随优于区间交易
- 若模式切换频率仍过高 → L1 走势类型在 1min 上不够稳定，需用更高级别

**谱系引用**：
- 526号：递归存在论区分（a0 级别分层）
- 267号：满仓满融降成本体系
- §7.5：在线因果 merge tree

**影响声明**：新建独立回测脚本 `fugue_movetype_backtest_1min.py`，不修改引擎代码或旧版回测。

**认识论等级**：L2（真实数据，3标的 1min；可产生否定性结果）。