# 多级别 PH 分层赋格 FSM 回测 — 1分钟数据

## 架构

5状态 FSM + 三级别 PH 分层：

| PH级别 | 输入 | 更新频率 | 信号用途 |
|--------|------|---------|---------|
| L0 | 1min close | 每bar | 进场门控（rank-1 settle） |
| L1 | L1线段端点价格 | ~数千次 | 降成本触发/加仓比例（non-rank-1 settle） |
| L2 | L1走势端点价格 | ~数百次 | 方向裁决/止损（rank-1 settle） |

**vs 旧版 H 组**：旧版只有一棵 L0 PH 树做方向裁决，方向翻转~780次；
新版用 L2 PH 做方向裁决，翻转频率应降至~47次量级。

## QQQ

- 数据：**728,030** bars (1min)
- 价格：268.82 → 738.28
- Buy-and-hold: **+174.64%**
- 回测耗时：395.0s

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
| 交易数 | 55 (多27/空28) |
| 胜率 | 65.5% |
| 平均收益 | +0.880% |
| 复利累计 | +58.26% |
| 最大回撤 | -6.18% |
| 平均持仓 | 4,124 bars |
| 有加仓的交易 | 26/55 |
| 有降成本的交易 | 25/55 |
| 达到本金回收 | 0/55 |

### 交易明细

| # | 方向 | 入场 | 出场 | 持仓bars | PnL% | 加仓 | 短差 | 本金回收 | 退出原因 | 状态路径 |
|---|------|------|------|---------|------|------|------|---------|---------|---------|
| 1 | 多 | 313.67@45926 | 305.43@49505 | 3,579 | -1.13 | 3 | 3 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 2 | 多 | 315.00@50895 | 317.32@56108 | 5,213 | +0.99 | 1 | 5 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 3 | 多 | 322.18@67707 | 319.88@67882 | 175 | -0.71 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 4 | 多 | 319.43@67965 | 318.33@72362 | 4,397 | +0.08 | 2 | 4 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 5 | 多 | 323.59@72703 | 352.97@87747 | 15,044 | +10.06 | 4 | 11 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 6 | 多 | 357.55@92088 | 354.25@92162 | 74 | -0.92 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 7 | 多 | 357.62@93055 | 365.80@97876 | 4,821 | +3.15 | 1 | 5 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 8 | 多 | 377.16@109965 | 376.68@115656 | 5,691 | +1.75 | 1 | 5 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 9 | 多 | 357.95@131228 | 358.07@131250 | 22 | +0.03 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 10 | 空 | 366.49@160970 | 366.95@160977 | 7 | -0.13 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 11 | 空 | 351.34@168860 | 350.27@174984 | 6,124 | -0.53 | 3 | 6 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 12 | 多 | 389.75@186026 | 388.55@192317 | 6,291 | +1.37 | 1 | 6 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 13 | 多 | 395.32@196835 | 396.84@208841 | 12,006 | +2.63 | 1 | 14 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 14 | 多 | 413.20@216485 | 419.92@223983 | 7,498 | +3.34 | 1 | 8 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 15 | 多 | 435.16@229308 | 429.72@231925 | 2,617 | +0.17 | 1 | 1 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 16 | 多 | 439.07@236272 | 439.85@243498 | 7,226 | +1.76 | 2 | 6 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 17 | 多 | 448.45@252489 | 442.59@259485 | 6,996 | +0.25 | 1 | 9 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 18 | 空 | 420.71@268939 | 422.16@269250 | 311 | -0.34 | 1 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 19 | 空 | 416.61@269419 | 424.38@271192 | 1,773 | -1.59 | 2 | 1 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 20 | 空 | 427.85@271673 | 427.39@271812 | 139 | +0.11 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 21 | 空 | 435.70@278336 | 435.55@278359 | 23 | +0.03 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 22 | 空 | 440.53@279693 | 440.30@279704 | 11 | +0.05 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 23 | 空 | 453.13@285358 | 452.77@285490 | 132 | +0.08 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 24 | 空 | 451.79@285618 | 451.65@285663 | 45 | +0.03 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 25 | 多 | 456.21@288172 | 451.10@293623 | 5,451 | +0.02 | 1 | 7 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 26 | 多 | 463.92@296108 | 481.63@308780 | 12,672 | +3.33 | 5 | 13 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 27 | 多 | 488.76@310992 | 489.62@318067 | 7,075 | +1.44 | 2 | 8 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 28 | 多 | 487.67@318163 | 484.92@318262 | 99 | -0.56 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 29 | 空 | 451.11@333303 | 450.29@333363 | 60 | +0.18 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 30 | 空 | 463.08@336068 | 462.16@336255 | 187 | +0.20 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 31 | 空 | 477.46@338791 | 479.37@338919 | 128 | -0.40 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 32 | 空 | 506.78@386210 | 507.95@386411 | 201 | -0.23 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 33 | 空 | 515.67@387817 | 515.65@387833 | 16 | +0.00 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 34 | 空 | 516.41@387986 | 515.63@388043 | 57 | +0.15 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 35 | 空 | 511.38@388289 | 512.53@388365 | 76 | -0.22 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 36 | 多 | 519.99@400611 | 524.05@410720 | 10,109 | +2.69 | 1 | 9 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 37 | 多 | 540.63@440493 | 516.63@445662 | 5,169 | -2.08 | 1 | 6 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 38 | 空 | 402.75@470586 | 423.64@471017 | 431 | -4.19 | 1 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 39 | 空 | 473.00@483482 | 471.70@483797 | 315 | +0.27 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 40 | 空 | 517.69@494779 | 517.40@494947 | 168 | +0.06 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 41 | 空 | 530.60@508058 | 529.51@508123 | 65 | +0.21 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 42 | 空 | 532.42@508455 | 530.97@508509 | 54 | +0.27 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 43 | 空 | 541.66@519842 | 542.07@519949 | 107 | -0.08 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 44 | 多 | 544.30@520847 | 565.00@543673 | 22,826 | +5.16 | 1 | 22 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 45 | 多 | 580.37@549098 | 565.70@554420 | 5,322 | -1.02 | 1 | 7 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 46 | 多 | 583.50@566173 | 601.49@588515 | 22,342 | +4.86 | 2 | 20 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 47 | 多 | 616.84@594460 | 624.25@601887 | 7,427 | +1.97 | 1 | 5 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 48 | 多 | 599.57@603660 | 603.75@603769 | 109 | +0.70 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 49 | 空 | 621.54@617240 | 620.52@617435 | 195 | +0.16 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 50 | 空 | 580.27@680908 | 581.15@687843 | 6,935 | -1.38 | 5 | 6 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 51 | 空 | 612.91@693617 | 611.08@693898 | 281 | +0.30 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 52 | 空 | 631.94@696319 | 632.76@696456 | 137 | -0.13 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 53 | 空 | 642.65@697975 | 648.39@698194 | 219 | -0.89 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 54 | 空 | 647.16@698959 | 647.22@698993 | 34 | -0.01 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 55 | 多 | 649.98@699681 | 738.28@728029 | 28,348 | +17.08 | 0 | 32 | 否 | eod_close | COST_REDUCING→POSITION_OPEN |

### FSM 状态分布（按 bar 数）

| 状态 | Bars | 占比 |
|------|------|------|
| SCANNING | 501,200 | 68.8% |
| POSITION_OPEN | 142,217 | 19.5% |
| COST_REDUCING | 84,613 | 11.6% |
| PRINCIPAL_WITHDRAWN | 0 | 0.0% |
| STOPPED_OUT | 0 | 0.0% |

<details><summary>事件流（579条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 45,926 | 313.67 | POSITION_OPEN | ENTRY_LONG@313.67 L2dir=1 |
| 46,025 | 309.74 | COST_REDUCING | TRIM@309.74 shares=314.6 L1ratio=0.987 |
| 46,714 | 313.12 | POSITION_OPEN | CLOSE_DIFF@313.12 profit=-1063.21 |
| 47,157 | 310.23 | COST_REDUCING | TRIM@310.23 shares=364.9 L1ratio=0.995 |
| 47,712 | 308.69 | POSITION_OPEN | CLOSE_DIFF@308.69 profit=562.00 |
| 48,006 | 311.37 | POSITION_OPEN | ADD_POS@311.37 +55.0 L1ratio=0.150 |
| 48,740 | 309.02 | COST_REDUCING | TRIM@309.02 shares=419.7 L1ratio=0.995 |
| 49,360 | 306.38 | POSITION_OPEN | CLOSE_DIFF@306.38 profit=1107.97 |
| 49,505 | 305.43 | SCANNING | STOP:bsp_invalidate@305.43 pnl=-1.13% |
| 50,895 | 315.00 | POSITION_OPEN | ENTRY_LONG@315.00 L2dir=1 |
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
| 55,783 | 318.10 | POSITION_OPEN | ADD_POS@318.10 +47.3 L1ratio=0.149 |
| 56,108 | 317.32 | SCANNING | STOP:bsp_invalidate@317.32 pnl=+0.99% |
| 67,707 | 322.18 | POSITION_OPEN | ENTRY_LONG@322.18 L2dir=1 |
| 67,882 | 319.88 | SCANNING | STOP:bsp_invalidate@319.88 pnl=-0.71% |
| 67,965 | 319.43 | POSITION_OPEN | ENTRY_LONG@319.43 L2dir=1 |
| 68,196 | 320.04 | COST_REDUCING | TRIM@320.04 shares=302.8 L1ratio=0.967 |
| 68,316 | 319.31 | POSITION_OPEN | CLOSE_DIFF@319.31 profit=221.07 |
| 68,532 | 321.28 | POSITION_OPEN | ADD_POS@321.28 +54.2 L1ratio=0.173 |
| 68,893 | 322.36 | COST_REDUCING | TRIM@322.36 shares=362.9 L1ratio=0.988 |
| 69,320 | 322.04 | POSITION_OPEN | CLOSE_DIFF@322.04 profit=116.14 |
| 69,405 | 322.46 | COST_REDUCING | TRIM@322.46 shares=363.1 L1ratio=0.989 |
| 69,827 | 322.03 | POSITION_OPEN | CLOSE_DIFF@322.03 profit=156.11 |
| 71,205 | 316.80 | COST_REDUCING | TRIM@316.80 shares=358.8 L1ratio=0.977 |
| 71,802 | 317.60 | POSITION_OPEN | CLOSE_DIFF@317.60 profit=-287.06 |
| 72,003 | 316.08 | POSITION_OPEN | ADD_POS@316.08 +49.3 L1ratio=0.134 |
| 72,362 | 318.33 | SCANNING | STOP:bsp_invalidate@318.33 pnl=+0.08% |
| 72,703 | 323.59 | POSITION_OPEN | ENTRY_LONG@323.59 L2dir=1 |
| 73,460 | 323.47 | COST_REDUCING | TRIM@323.47 shares=308.4 L1ratio=0.998 |
| 74,028 | 322.43 | POSITION_OPEN | CLOSE_DIFF@322.43 profit=320.68 |
| 74,074 | 321.91 | COST_REDUCING | TRIM@321.91 shares=307.0 L1ratio=0.993 |
| 74,926 | 324.00 | POSITION_OPEN | CLOSE_DIFF@324.00 profit=-641.62 |
| 75,607 | 325.21 | COST_REDUCING | TRIM@325.21 shares=300.2 L1ratio=0.971 |
| 75,993 | 326.14 | POSITION_OPEN | CLOSE_DIFF@326.14 profit=-279.21 |
| 76,441 | 325.48 | COST_REDUCING | TRIM@325.48 shares=307.8 L1ratio=0.996 |
| 77,031 | 325.95 | POSITION_OPEN | CLOSE_DIFF@325.95 profit=-144.68 |
| 77,328 | 325.96 | POSITION_OPEN | ADD_POS@325.96 +19.3 L1ratio=0.059 |
| 77,981 | 327.22 | COST_REDUCING | TRIM@327.22 shares=344.5 L1ratio=0.994 |
| 78,031 | 327.75 | POSITION_OPEN | CLOSE_DIFF@327.75 profit=-182.61 |
| 78,440 | 327.44 | COST_REDUCING | TRIM@327.44 shares=339.6 L1ratio=0.980 |
| 78,534 | 327.64 | POSITION_OPEN | CLOSE_DIFF@327.64 profit=-67.93 |
| 78,826 | 328.60 | COST_REDUCING | TRIM@328.60 shares=343.2 L1ratio=0.990 |
| 79,420 | 332.09 | POSITION_OPEN | CLOSE_DIFF@332.09 profit=-1197.74 |
| 80,484 | 336.69 | COST_REDUCING | TRIM@336.69 shares=339.9 L1ratio=0.981 |
| 81,188 | 336.32 | POSITION_OPEN | CLOSE_DIFF@336.32 profit=125.76 |
| 82,391 | 333.84 | COST_REDUCING | TRIM@333.84 shares=336.0 L1ratio=0.970 |
| 82,996 | 330.63 | POSITION_OPEN | CLOSE_DIFF@330.63 profit=1078.43 |
| 83,632 | 337.79 | POSITION_OPEN | ADD_POS@337.79 +38.6 L1ratio=0.111 |
| 85,792 | 351.01 | COST_REDUCING | TRIM@351.01 shares=364.6 L1ratio=0.947 |
| 86,834 | 347.42 | POSITION_OPEN | CLOSE_DIFF@347.42 profit=1308.79 |
| 87,198 | 348.05 | COST_REDUCING | TRIM@348.05 shares=355.0 L1ratio=0.922 |
| 87,236 | 347.99 | POSITION_OPEN | CLOSE_DIFF@347.99 profit=21.30 |
| 87,663 | 352.53 | POSITION_OPEN | ADD_POS@352.53 +30.9 L1ratio=0.080 |
| 87,747 | 352.97 | SCANNING | STOP:bsp_invalidate@352.97 pnl=+10.06% |
| 92,088 | 357.55 | POSITION_OPEN | ENTRY_LONG@357.55 L2dir=1 |
| 92,162 | 354.25 | SCANNING | STOP:bsp_invalidate@354.25 pnl=-0.92% |
| 93,055 | 357.62 | POSITION_OPEN | ENTRY_LONG@357.62 L2dir=1 |
| 93,739 | 361.71 | COST_REDUCING | TRIM@361.71 shares=261.0 L1ratio=0.933 |
| 93,922 | 363.24 | POSITION_OPEN | CLOSE_DIFF@363.24 profit=-399.35 |
| 94,011 | 362.72 | COST_REDUCING | TRIM@362.72 shares=277.2 L1ratio=0.991 |
| 94,978 | 366.34 | POSITION_OPEN | CLOSE_DIFF@366.34 profit=-1003.51 |
| 95,575 | 368.10 | COST_REDUCING | TRIM@368.10 shares=277.9 L1ratio=0.994 |
| 95,754 | 370.23 | POSITION_OPEN | CLOSE_DIFF@370.23 profit=-591.86 |
| 95,904 | 369.83 | COST_REDUCING | TRIM@369.83 shares=268.3 L1ratio=0.960 |
| 96,071 | 370.83 | POSITION_OPEN | CLOSE_DIFF@370.83 profit=-268.34 |
| 96,272 | 371.02 | COST_REDUCING | TRIM@371.02 shares=267.2 L1ratio=0.956 |
| 97,689 | 366.89 | POSITION_OPEN | CLOSE_DIFF@366.89 profit=1103.67 |
| 97,876 | 365.80 | SCANNING | STOP:bsp_invalidate@365.80 pnl=+3.15% |
| 109,965 | 377.16 | POSITION_OPEN | ENTRY_LONG@377.16 L2dir=1 |
| 110,916 | 380.20 | COST_REDUCING | TRIM@380.20 shares=258.9 L1ratio=0.976 |
| 111,605 | 380.26 | POSITION_OPEN | CLOSE_DIFF@380.26 profit=-15.53 |
| 112,060 | 382.55 | COST_REDUCING | TRIM@382.55 shares=264.2 L1ratio=0.996 |
| 112,096 | 382.10 | POSITION_OPEN | CLOSE_DIFF@382.10 profit=118.89 |
| 112,795 | 385.24 | COST_REDUCING | TRIM@385.24 shares=264.6 L1ratio=0.998 |
| 112,977 | 386.31 | POSITION_OPEN | CLOSE_DIFF@386.31 profit=-283.10 |
| 113,388 | 385.11 | COST_REDUCING | TRIM@385.11 shares=255.2 L1ratio=0.962 |
| 114,885 | 377.83 | POSITION_OPEN | CLOSE_DIFF@377.83 profit=1857.59 |
| 114,940 | 377.28 | COST_REDUCING | TRIM@377.28 shares=249.0 L1ratio=0.939 |
| 115,159 | 376.99 | POSITION_OPEN | CLOSE_DIFF@376.99 profit=72.22 |
| 115,476 | 376.90 | POSITION_OPEN | ADD_POS@376.90 +24.8 L1ratio=0.094 |
| 115,656 | 376.68 | SCANNING | STOP:bsp_invalidate@376.68 pnl=+1.75% |
| 131,228 | 357.95 | POSITION_OPEN | ENTRY_LONG@357.95 L2dir=1 |
| 131,250 | 358.07 | SCANNING | STOP:bsp_invalidate@358.07 pnl=+0.03% |
| 160,970 | 366.49 | POSITION_OPEN | ENTRY_SHORT@366.49 L2dir=-1 |
| 160,977 | 366.95 | SCANNING | STOP:bsp_invalidate@366.95 pnl=-0.13% |
| 168,860 | 351.34 | POSITION_OPEN | ENTRY_SHORT@351.34 L2dir=-1 |
| 169,349 | 357.98 | COST_REDUCING | TRIM@357.98 shares=81.7 L1ratio=0.287 |
| 170,195 | 357.35 | POSITION_OPEN | CLOSE_DIFF@357.35 profit=-51.45 |
| 170,577 | 356.91 | COST_REDUCING | TRIM@356.91 shares=81.7 L1ratio=0.287 |
| 170,666 | 356.82 | POSITION_OPEN | CLOSE_DIFF@356.82 profit=-7.35 |
| 171,494 | 346.96 | POSITION_OPEN | ADD_POS@346.96 +193.4 L1ratio=0.679 |
| ... | ... | ... | （共579条，显示前100） |
</details>

## OKLO

- 数据：**333,613** bars (1min)
- 价格：18.07 → 63.48
- Buy-and-hold: **+251.30%**
- 回测耗时：72.3s

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
| 交易数 | 18 (多9/空9) |
| 胜率 | 44.4% |
| 平均收益 | +4.587% |
| 复利累计 | +72.25% |
| 最大回撤 | -45.46% |
| 平均持仓 | 2,977 bars |
| 有加仓的交易 | 11/18 |
| 有降成本的交易 | 11/18 |
| 达到本金回收 | 0/18 |

### 交易明细

| # | 方向 | 入场 | 出场 | 持仓bars | PnL% | 加仓 | 短差 | 本金回收 | 退出原因 | 状态路径 |
|---|------|------|------|---------|------|------|------|---------|---------|---------|
| 1 | 多 | 13.08@41091 | 19.11@46795 | 5,704 | +39.20 | 1 | 5 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 2 | 多 | 24.60@47161 | 20.33@50552 | 3,391 | -4.80 | 1 | 3 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 3 | 多 | 28.02@53130 | 19.57@57372 | 4,242 | +2.32 | 2 | 7 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 4 | 空 | 28.20@77486 | 28.02@77496 | 10 | +0.64 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 5 | 多 | 31.98@82271 | 54.69@94200 | 11,929 | +49.76 | 5 | 9 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 6 | 空 | 39.00@136306 | 38.13@136346 | 40 | +2.23 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 7 | 空 | 62.98@151001 | 67.02@151205 | 204 | -6.41 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 8 | 多 | 66.80@151334 | 65.64@151540 | 206 | -1.74 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 9 | 多 | 76.58@173692 | 76.68@178964 | 5,272 | +1.07 | 4 | 5 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 10 | 多 | 64.98@187214 | 64.39@187431 | 217 | -0.91 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 11 | 多 | 88.82@199494 | 109.95@207459 | 7,965 | +35.57 | 1 | 7 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 12 | 多 | 146.72@215837 | 168.62@220471 | 4,634 | +21.06 | 2 | 4 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 13 | 空 | 96.88@235756 | 104.29@236068 | 312 | -7.65 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 14 | 空 | 84.22@239719 | 85.61@241588 | 1,869 | -3.37 | 1 | 2 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 15 | 空 | 74.94@255587 | 90.79@259097 | 3,510 | -25.84 | 3 | 2 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 16 | 空 | 61.02@275312 | 69.81@277496 | 2,184 | -10.29 | 1 | 2 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 17 | 空 | 50.22@298313 | 49.78@300135 | 1,822 | -2.70 | 3 | 2 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 18 | 空 | 61.50@306112 | 64.92@306190 | 78 | -5.56 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |

### FSM 状态分布（按 bar 数）

| 状态 | Bars | 占比 |
|------|------|------|
| SCANNING | 280,024 | 83.9% |
| POSITION_OPEN | 28,912 | 8.7% |
| COST_REDUCING | 24,677 | 7.4% |
| PRINCIPAL_WITHDRAWN | 0 | 0.0% |
| STOPPED_OUT | 0 | 0.0% |

<details><summary>事件流（147条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 41,091 | 13.08 | POSITION_OPEN | ENTRY_LONG@13.08 L2dir=1 |
| 41,862 | 16.22 | COST_REDUCING | TRIM@16.22 shares=6887.0 L1ratio=0.901 |
| 42,565 | 15.84 | POSITION_OPEN | CLOSE_DIFF@15.84 profit=2617.04 |
| 43,016 | 18.46 | COST_REDUCING | TRIM@18.46 shares=6819.9 L1ratio=0.892 |
| 43,450 | 19.91 | POSITION_OPEN | CLOSE_DIFF@19.91 profit=-9888.86 |
| 44,516 | 20.63 | COST_REDUCING | TRIM@20.63 shares=7044.4 L1ratio=0.921 |
| 44,992 | 20.44 | POSITION_OPEN | CLOSE_DIFF@20.44 profit=1338.44 |
| 45,375 | 18.33 | COST_REDUCING | TRIM@18.33 shares=6593.8 L1ratio=0.862 |
| 45,777 | 18.12 | POSITION_OPEN | CLOSE_DIFF@18.12 profit=1384.70 |
| 45,878 | 17.97 | COST_REDUCING | TRIM@17.97 shares=6593.8 L1ratio=0.862 |
| 46,744 | 19.14 | POSITION_OPEN | CLOSE_DIFF@19.14 profit=-7714.77 |
| 46,795 | 19.11 | SCANNING | STOP:bsp_invalidate@19.11 pnl=+39.20% |
| 47,161 | 24.60 | POSITION_OPEN | ENTRY_LONG@24.60 L2dir=1 |
| 47,960 | 23.43 | COST_REDUCING | TRIM@23.43 shares=3919.3 L1ratio=0.964 |
| 48,481 | 26.75 | POSITION_OPEN | CLOSE_DIFF@26.75 profit=-13012.23 |
| 48,715 | 23.60 | COST_REDUCING | TRIM@23.60 shares=3612.8 L1ratio=0.889 |
| 49,561 | 22.99 | POSITION_OPEN | CLOSE_DIFF@22.99 profit=2203.78 |
| 49,609 | 22.92 | COST_REDUCING | TRIM@22.92 shares=3612.8 L1ratio=0.889 |
| 50,507 | 20.36 | POSITION_OPEN | CLOSE_DIFF@20.36 profit=9248.67 |
| 50,552 | 20.33 | SCANNING | STOP:bsp_invalidate@20.33 pnl=-4.80% |
| 53,130 | 28.02 | POSITION_OPEN | ENTRY_LONG@28.02 L2dir=1 |
| 53,314 | 26.42 | POSITION_OPEN | ADD_POS@26.42 +1252.1 L1ratio=0.351 |
| 53,605 | 24.94 | COST_REDUCING | TRIM@24.94 shares=4790.7 L1ratio=0.994 |
| 53,907 | 24.36 | POSITION_OPEN | CLOSE_DIFF@24.36 profit=2778.59 |
| 53,993 | 24.40 | COST_REDUCING | TRIM@24.40 shares=4790.7 L1ratio=0.994 |
| 54,119 | 23.72 | POSITION_OPEN | CLOSE_DIFF@23.72 profit=3257.66 |
| 54,228 | 23.83 | COST_REDUCING | TRIM@23.83 shares=4790.7 L1ratio=0.994 |
| 54,965 | 22.34 | POSITION_OPEN | CLOSE_DIFF@22.34 profit=7138.10 |
| 55,122 | 22.74 | COST_REDUCING | TRIM@22.74 shares=4790.7 L1ratio=0.994 |
| 55,491 | 23.99 | POSITION_OPEN | CLOSE_DIFF@23.99 profit=-5988.34 |
| 56,100 | 23.94 | COST_REDUCING | TRIM@23.94 shares=4790.7 L1ratio=0.994 |
| 56,142 | 23.71 | POSITION_OPEN | CLOSE_DIFF@23.71 profit=1101.85 |
| 56,427 | 24.00 | COST_REDUCING | TRIM@24.00 shares=4790.7 L1ratio=0.994 |
| 56,700 | 21.99 | POSITION_OPEN | CLOSE_DIFF@21.99 profit=9629.25 |
| 56,759 | 22.40 | COST_REDUCING | TRIM@22.40 shares=4790.7 L1ratio=0.994 |
| 57,138 | 18.72 | POSITION_OPEN | CLOSE_DIFF@18.72 profit=17629.66 |
| 57,372 | 19.57 | SCANNING | STOP:bsp_invalidate@19.57 pnl=+2.32% |
| 77,486 | 28.20 | POSITION_OPEN | ENTRY_SHORT@28.20 L2dir=-1 |
| 77,496 | 28.02 | SCANNING | STOP:bsp_invalidate@28.02 pnl=+0.64% |
| 82,271 | 31.98 | POSITION_OPEN | ENTRY_LONG@31.98 L2dir=1 |
| 83,207 | 32.37 | COST_REDUCING | TRIM@32.37 shares=2760.5 L1ratio=0.883 |
| 83,593 | 38.78 | POSITION_OPEN | CLOSE_DIFF@38.78 profit=-17694.90 |
| 84,429 | 42.84 | COST_REDUCING | TRIM@42.84 shares=2706.0 L1ratio=0.865 |
| 84,463 | 42.52 | POSITION_OPEN | CLOSE_DIFF@42.52 profit=865.91 |
| 84,755 | 35.90 | COST_REDUCING | TRIM@35.90 shares=2449.6 L1ratio=0.783 |
| 86,173 | 33.53 | POSITION_OPEN | CLOSE_DIFF@33.53 profit=5805.65 |
| 86,342 | 34.20 | POSITION_OPEN | ADD_POS@34.20 +1079.2 L1ratio=0.345 |
| 87,232 | 36.11 | POSITION_OPEN | ADD_POS@36.11 +1451.7 L1ratio=0.345 |
| 87,675 | 41.59 | POSITION_OPEN | ADD_POS@41.59 +1952.7 L1ratio=0.345 |
| 88,040 | 41.95 | COST_REDUCING | TRIM@41.95 shares=7537.6 L1ratio=0.990 |
| 88,940 | 43.40 | POSITION_OPEN | CLOSE_DIFF@43.40 profit=-10967.24 |
| 89,466 | 46.55 | COST_REDUCING | TRIM@46.55 shares=7511.0 L1ratio=0.987 |
| 90,008 | 45.95 | POSITION_OPEN | CLOSE_DIFF@45.95 profit=4506.61 |
| 91,119 | 48.95 | COST_REDUCING | TRIM@48.95 shares=6808.6 L1ratio=0.895 |
| 91,475 | 48.63 | POSITION_OPEN | CLOSE_DIFF@48.63 profit=2144.70 |
| 92,285 | 54.89 | COST_REDUCING | TRIM@54.89 shares=7383.6 L1ratio=0.970 |
| 92,312 | 55.20 | POSITION_OPEN | CLOSE_DIFF@55.20 profit=-2288.90 |
| 92,466 | 54.30 | COST_REDUCING | TRIM@54.30 shares=7383.6 L1ratio=0.970 |
| 93,016 | 52.38 | POSITION_OPEN | CLOSE_DIFF@52.38 profit=14139.52 |
| 93,454 | 50.88 | COST_REDUCING | TRIM@50.88 shares=6903.5 L1ratio=0.907 |
| 93,772 | 52.73 | POSITION_OPEN | CLOSE_DIFF@52.73 profit=-12736.87 |
| 93,903 | 54.91 | POSITION_OPEN | ADD_POS@54.91 +1495.6 L1ratio=0.197 |
| 94,086 | 54.37 | POSITION_OPEN | ADD_POS@54.37 +1789.5 L1ratio=0.197 |
| 94,200 | 54.69 | SCANNING | STOP:bsp_invalidate@54.69 pnl=+49.76% |
| 136,306 | 39.00 | POSITION_OPEN | ENTRY_SHORT@39.00 L2dir=-1 |
| 136,346 | 38.13 | SCANNING | STOP:bsp_invalidate@38.13 pnl=+2.23% |
| 151,001 | 62.98 | POSITION_OPEN | ENTRY_SHORT@62.98 L2dir=-1 |
| 151,205 | 67.02 | SCANNING | STOP:bsp_invalidate@67.02 pnl=-6.41% |
| 151,334 | 66.80 | POSITION_OPEN | ENTRY_LONG@66.80 L2dir=1 |
| 151,540 | 65.64 | SCANNING | STOP:bsp_invalidate@65.64 pnl=-1.74% |
| 173,692 | 76.58 | POSITION_OPEN | ENTRY_LONG@76.58 L2dir=1 |
| 173,991 | 72.22 | COST_REDUCING | TRIM@72.22 shares=1206.1 L1ratio=0.924 |
| 174,553 | 75.00 | POSITION_OPEN | CLOSE_DIFF@75.00 profit=-3352.83 |
| 175,060 | 74.46 | COST_REDUCING | TRIM@74.46 shares=1361.2 L1ratio=0.971 |
| 175,622 | 75.20 | POSITION_OPEN | CLOSE_DIFF@75.20 profit=-1007.32 |
| 176,091 | 71.38 | COST_REDUCING | TRIM@71.38 shares=1311.2 L1ratio=0.935 |
| 176,122 | 71.57 | POSITION_OPEN | CLOSE_DIFF@71.57 profit=-249.12 |
| 176,234 | 71.40 | COST_REDUCING | TRIM@71.40 shares=1311.2 L1ratio=0.935 |
| 176,398 | 75.80 | POSITION_OPEN | CLOSE_DIFF@75.80 profit=-5762.53 |
| 176,627 | 75.22 | POSITION_OPEN | ADD_POS@75.22 +152.2 L1ratio=0.109 |
| 176,907 | 77.59 | POSITION_OPEN | ADD_POS@77.59 +168.7 L1ratio=0.109 |
| 177,024 | 77.30 | COST_REDUCING | TRIM@77.30 shares=1717.3 L1ratio=0.997 |
| 178,793 | 76.59 | POSITION_OPEN | CLOSE_DIFF@76.59 profit=1219.26 |
| 178,964 | 76.68 | SCANNING | STOP:bsp_invalidate@76.68 pnl=+1.07% |
| 187,214 | 64.98 | POSITION_OPEN | ENTRY_LONG@64.98 L2dir=1 |
| 187,431 | 64.39 | SCANNING | STOP:bsp_invalidate@64.39 pnl=-0.91% |
| 199,494 | 88.82 | POSITION_OPEN | ENTRY_LONG@88.82 L2dir=1 |
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
| ... | ... | ... | （共147条，显示前100） |
</details>

## HK700

- 数据：**1,408,882** bars (1min)
- 价格：11.36 → 458.20
- Buy-and-hold: **+3933.45%**
- 回测耗时：521.5s

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
| 交易数 | 21 (多14/空7) |
| 胜率 | 57.1% |
| 平均收益 | +6.651% |
| 复利累计 | +222.81% |
| 最大回撤 | -13.83% |
| 平均持仓 | 15,566 bars |
| 有加仓的交易 | 17/21 |
| 有降成本的交易 | 17/21 |
| 达到本金回收 | 1/21 |

### 交易明细

| # | 方向 | 入场 | 出场 | 持仓bars | PnL% | 加仓 | 短差 | 本金回收 | 退出原因 | 状态路径 |
|---|------|------|------|---------|------|------|------|---------|---------|---------|
| 1 | 空 | 43.30@245433 | 43.50@245646 | 213 | -0.46 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 2 | 空 | 40.60@251169 | 39.60@253797 | 2,628 | +2.46 | 0 | 1 | 否 | l2_direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 3 | 多 | 50.50@289428 | 82.80@377587 | 88,159 | +62.38 | 5 | 12 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 4 | 多 | 87.50@378891 | 90.50@409439 | 30,548 | +17.70 | 2 | 10 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 5 | 多 | 119.10@428314 | 111.25@438789 | 10,475 | -4.92 | 1 | 3 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 6 | 多 | 109.40@441286 | 105.25@443385 | 2,099 | -0.02 | 1 | 1 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 7 | 多 | 119.30@453754 | 109.95@457850 | 4,096 | -2.32 | 1 | 1 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 8 | 多 | 124.45@468633 | 140.55@502832 | 34,199 | +14.00 | 6 | 15 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 9 | 多 | 158.80@576773 | 167.75@624082 | 47,309 | +8.08 | 1 | 15 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 10 | 多 | 222.90@652085 | 263.40@670541 | 18,456 | +25.31 | 1 | 8 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 11 | 多 | 277.10@672227 | 299.90@682404 | 10,177 | +7.60 | 1 | 4 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 12 | 多 | 312.90@685235 | 351.40@704079 | 18,844 | +13.43 | 1 | 6 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 13 | 多 | 404.70@710943 | 384.60@719629 | 8,686 | +7.62 | 1 | 5 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 14 | 空 | 292.20@766519 | 290.40@785965 | 19,446 | -11.21 | 7 | 9 | 是 | bsp_invalidate | COST_REDUCING→POSITION_OPEN→PRINCIPAL_WITHDRAWN |
| 15 | 空 | 293.10@787259 | 296.80@787307 | 48 | -1.26 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 16 | 空 | 457.80@915572 | 460.20@915755 | 183 | -0.52 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 17 | 多 | 487.60@917774 | 490.00@922761 | 4,987 | +0.29 | 1 | 5 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 18 | 多 | 552.20@944407 | 540.80@954533 | 10,126 | -1.48 | 1 | 3 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 19 | 多 | 583.60@962507 | 641.60@972802 | 10,295 | +10.49 | 2 | 6 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 20 | 空 | 380.50@1056649 | 366.20@1057460 | 811 | +1.55 | 2 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 21 | 空 | 218.80@1109328 | 234.80@1114422 | 5,094 | -9.04 | 2 | 3 | 否 | bsp_invalidate | COST_REDUCING→POSITION_OPEN |

### FSM 状态分布（按 bar 数）

| 状态 | Bars | 占比 |
|------|------|------|
| SCANNING | 1,082,003 | 76.8% |
| POSITION_OPEN | 194,154 | 13.8% |
| COST_REDUCING | 132,285 | 9.4% |
| PRINCIPAL_WITHDRAWN | 440 | 0.0% |
| STOPPED_OUT | 0 | 0.0% |

<details><summary>事件流（283条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 245,433 | 43.30 | POSITION_OPEN | ENTRY_SHORT@43.30 L2dir=-1 |
| 245,646 | 43.50 | SCANNING | STOP:bsp_invalidate@43.50 pnl=-0.46% |
| 251,169 | 40.60 | POSITION_OPEN | ENTRY_SHORT@40.60 L2dir=-1 |
| 251,784 | 41.50 | COST_REDUCING | TRIM@41.50 shares=320.4 L1ratio=0.130 |
| 252,848 | 38.60 | POSITION_OPEN | CLOSE_DIFF@38.60 profit=-929.30 |
| 253,797 | 39.60 | SCANNING | STOP:l2_direction_flip_long@39.60 pnl=+2.46% |
| 289,428 | 50.50 | POSITION_OPEN | ENTRY_LONG@50.50 L2dir=1 |
| 291,912 | 49.70 | COST_REDUCING | TRIM@49.70 shares=1958.0 L1ratio=0.989 |
| 302,478 | 45.90 | POSITION_OPEN | CLOSE_DIFF@45.90 profit=7440.39 |
| 307,353 | 49.60 | POSITION_OPEN | ADD_POS@49.60 +284.2 L1ratio=0.143 |
| 309,259 | 49.20 | COST_REDUCING | TRIM@49.20 shares=2218.7 L1ratio=0.980 |
| 309,924 | 50.00 | POSITION_OPEN | CLOSE_DIFF@50.00 profit=-1774.93 |
| 315,344 | 48.60 | COST_REDUCING | TRIM@48.60 shares=2249.2 L1ratio=0.993 |
| 316,732 | 51.30 | POSITION_OPEN | CLOSE_DIFF@51.30 profit=-6072.90 |
| 320,109 | 48.50 | COST_REDUCING | TRIM@48.50 shares=2185.9 L1ratio=0.965 |
| 324,400 | 46.20 | POSITION_OPEN | CLOSE_DIFF@46.20 profit=5027.65 |
| 326,792 | 46.70 | POSITION_OPEN | ADD_POS@46.70 +426.4 L1ratio=0.188 |
| 329,386 | 50.00 | POSITION_OPEN | ADD_POS@50.00 +506.7 L1ratio=0.188 |
| 331,116 | 50.30 | COST_REDUCING | TRIM@50.30 shares=3086.7 L1ratio=0.965 |
| 334,063 | 55.10 | POSITION_OPEN | CLOSE_DIFF@55.10 profit=-14816.27 |
| 336,793 | 55.40 | COST_REDUCING | TRIM@55.40 shares=3191.1 L1ratio=0.998 |
| 338,777 | 55.50 | POSITION_OPEN | CLOSE_DIFF@55.50 profit=-319.11 |
| 340,518 | 51.70 | COST_REDUCING | TRIM@51.70 shares=2931.0 L1ratio=0.917 |
| 341,695 | 55.50 | POSITION_OPEN | CLOSE_DIFF@55.50 profit=-11137.81 |
| 343,058 | 56.70 | COST_REDUCING | TRIM@56.70 shares=3141.4 L1ratio=0.982 |
| 343,399 | 56.10 | POSITION_OPEN | CLOSE_DIFF@56.10 profit=1884.82 |
| 352,110 | 68.50 | COST_REDUCING | TRIM@68.50 shares=2999.5 L1ratio=0.938 |
| 352,332 | 67.70 | POSITION_OPEN | CLOSE_DIFF@67.70 profit=2399.62 |
| 352,795 | 68.70 | COST_REDUCING | TRIM@68.70 shares=3182.3 L1ratio=0.995 |
| 355,033 | 66.20 | POSITION_OPEN | CLOSE_DIFF@66.20 profit=7955.76 |
| 364,770 | 76.90 | COST_REDUCING | TRIM@76.90 shares=3157.7 L1ratio=0.988 |
| 367,580 | 78.70 | POSITION_OPEN | CLOSE_DIFF@78.70 profit=-5683.78 |
| 368,642 | 77.30 | COST_REDUCING | TRIM@77.30 shares=2986.8 L1ratio=0.934 |
| 372,638 | 75.70 | POSITION_OPEN | CLOSE_DIFF@75.70 profit=4778.83 |
| 376,128 | 81.90 | POSITION_OPEN | ADD_POS@81.90 +512.3 L1ratio=0.160 |
| 377,223 | 82.40 | POSITION_OPEN | ADD_POS@82.40 +594.3 L1ratio=0.160 |
| 377,587 | 82.80 | SCANNING | STOP:bsp_invalidate@82.80 pnl=+62.38% |
| 378,891 | 87.50 | POSITION_OPEN | ENTRY_LONG@87.50 L2dir=1 |
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
| 409,422 | 91.65 | POSITION_OPEN | CLOSE_DIFF@91.65 profit=5427.08 |
| 409,439 | 90.50 | SCANNING | STOP:bsp_invalidate@90.50 pnl=+17.70% |
| 428,314 | 119.10 | POSITION_OPEN | ENTRY_LONG@119.10 L2dir=1 |
| 429,469 | 118.80 | COST_REDUCING | TRIM@118.80 shares=807.6 L1ratio=0.962 |
| 429,983 | 119.35 | POSITION_OPEN | CLOSE_DIFF@119.35 profit=-444.15 |
| 431,636 | 121.50 | COST_REDUCING | TRIM@121.50 shares=819.3 L1ratio=0.976 |
| 432,017 | 123.05 | POSITION_OPEN | CLOSE_DIFF@123.05 profit=-1269.97 |
| 435,026 | 120.20 | COST_REDUCING | TRIM@120.20 shares=820.0 L1ratio=0.977 |
| 437,828 | 118.45 | POSITION_OPEN | CLOSE_DIFF@118.45 profit=1435.04 |
| 438,678 | 112.75 | COST_REDUCING | ADD_POS@112.75 +53.0 L1ratio=0.063 |
| 438,789 | 111.25 | SCANNING | STOP:bsp_invalidate@111.25 pnl=-4.92% |
| 441,286 | 109.40 | POSITION_OPEN | ENTRY_LONG@109.40 L2dir=1 |
| 441,365 | 110.45 | COST_REDUCING | TRIM@110.45 shares=864.6 L1ratio=0.946 |
| 443,150 | 106.00 | POSITION_OPEN | CLOSE_DIFF@106.00 profit=3847.68 |
| 443,385 | 105.25 | SCANNING | STOP:bsp_invalidate@105.25 pnl=-0.02% |
| 453,754 | 119.30 | POSITION_OPEN | ENTRY_LONG@119.30 L2dir=1 |
| 456,029 | 115.30 | COST_REDUCING | TRIM@115.30 shares=819.3 L1ratio=0.977 |
| 457,634 | 109.15 | POSITION_OPEN | CLOSE_DIFF@109.15 profit=5038.92 |
| 457,850 | 109.95 | SCANNING | STOP:bsp_invalidate@109.95 pnl=-2.32% |
| 468,633 | 124.45 | POSITION_OPEN | ENTRY_LONG@124.45 L2dir=1 |
| 470,753 | 124.35 | COST_REDUCING | TRIM@124.35 shares=787.4 L1ratio=0.980 |
| 471,012 | 124.05 | POSITION_OPEN | CLOSE_DIFF@124.05 profit=236.23 |
| 471,415 | 124.70 | COST_REDUCING | TRIM@124.70 shares=799.4 L1ratio=0.995 |
| 471,599 | 124.55 | POSITION_OPEN | CLOSE_DIFF@124.55 profit=119.90 |
| 472,091 | 122.40 | COST_REDUCING | TRIM@122.40 shares=799.4 L1ratio=0.995 |
| 475,334 | 124.70 | POSITION_OPEN | CLOSE_DIFF@124.70 profit=-1838.51 |
| 477,069 | 123.35 | COST_REDUCING | TRIM@123.35 shares=843.2 L1ratio=0.993 |
| 479,346 | 122.15 | POSITION_OPEN | CLOSE_DIFF@122.15 profit=1011.78 |
| 480,282 | 125.35 | POSITION_OPEN | ADD_POS@125.35 +48.6 L1ratio=0.057 |
| 481,043 | 133.40 | COST_REDUCING | TRIM@133.40 shares=894.6 L1ratio=0.996 |
| 481,240 | 134.50 | POSITION_OPEN | CLOSE_DIFF@134.50 profit=-984.11 |
| 482,025 | 131.35 | COST_REDUCING | TRIM@131.35 shares=845.1 L1ratio=0.941 |
| 482,914 | 134.60 | POSITION_OPEN | CLOSE_DIFF@134.60 profit=-2746.57 |
| 485,735 | 145.20 | COST_REDUCING | TRIM@145.20 shares=814.7 L1ratio=0.907 |
| 486,139 | 146.45 | POSITION_OPEN | CLOSE_DIFF@146.45 profit=-1018.43 |
| 486,777 | 145.75 | COST_REDUCING | TRIM@145.75 shares=858.6 L1ratio=0.956 |
| 487,544 | 150.35 | POSITION_OPEN | CLOSE_DIFF@150.35 profit=-3949.74 |
| 487,689 | 150.90 | POSITION_OPEN | ADD_POS@150.90 +67.3 L1ratio=0.075 |
| 488,643 | 149.60 | COST_REDUCING | TRIM@149.60 shares=950.7 L1ratio=0.985 |
| 488,865 | 150.70 | POSITION_OPEN | CLOSE_DIFF@150.70 profit=-1045.76 |
| 489,342 | 149.50 | COST_REDUCING | TRIM@149.50 shares=950.7 L1ratio=0.985 |
| 491,627 | 147.65 | POSITION_OPEN | CLOSE_DIFF@147.65 profit=1758.78 |
| ... | ... | ... | （共283条，显示前100） |
</details>

## 汇总对比

| 标的 | Bars | BH% | 复利% | 胜率 | 交易数 | L2翻转 | 加仓率 | 降成本率 | PW率 | 耗时 |
|------|------|-----|------|------|--------|--------|--------|---------|------|------|
| QQQ | 728,030 | +174.6 | +58.26 | 65% | 55 | 14 | 26/55 | 25/55 | 0/55 | 395s |
| OKLO | 333,613 | +251.3 | +72.25 | 44% | 18 | 7 | 11/18 | 11/18 | 0/18 | 72s |
| HK700 | 1,408,882 | +3933.5 | +222.81 | 57% | 21 | 5 | 17/21 | 17/21 | 1/21 | 521s |

## 结果包六要素

**结论**：多级别 PH 分层 FSM 在 1 分钟级别 3 标的上的表现。方向裁决从 L0 提升到 L2，降成本从 L0 提升到 L1。

**定义依据**：
- L0 PH：1min close 序列的在线因果 merge tree（§7.5）
- L1 PH：L1 线段端点 ep1_price 序列的 merge tree
- L2 PH：L1 走势端点（up→high, down→low）序列的 merge tree
- 方向裁决 = L2 rank-1 settle（结构级别匹配交易级别）
- 降成本 = L1 non-rank-1 settle（中间级别的结构信号）
- 进场门控 = L0 rank-1 settle + BSP candidate（微观确认）

**边界条件**：
- PENDING_EXPIRY = 390 bars — BSP 候选需在 L0 rank-1 settle 前出现
- L2 PH 更新频率取决于 L1 走势 settle 速度 — 早期数据不足时 L2 方向可能长时间未决
- 若 L2 方向长期未翻转 → 交易数少 → 统计不显著
- 无滑点/手续费建模

**下游推论**：
- 若 L2 翻转~47次、胜率≥57% → 分层有效，方向级别匹配假说成立
- 若翻转频率仍高 → L2 PH 输入不够稀疏，需更高级别输入
- 若交易数为0 → L2 方向长期未决 或 BSP+L0 settle 未在窗口内共现 → 调整 PENDING_EXPIRY

**谱系引用**：
- 267号：满仓满融降成本体系
- §7.5：在线因果 merge tree
- 526号：递归存在论区分（a0 级别分层的谱系依据）

**影响声明**：新建独立回测脚本，不修改引擎代码或旧版回测。

**认识论等级**：L2（真实数据，3标的 1min；可产生否定性结果）。