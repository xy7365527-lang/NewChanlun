# 控制变量：去掉加仓 — D2区间套版基线

## 架构

D2区间套版去掉所有加仓路径：

- 持仓期间不因同方向新BSP加仓
- S5a回调不跌破前低只继续持有，不加仓
- 保留降成本（30课原文）
- 其余全部不动

控制变量：有加仓 vs 无加仓。

### FSM 状态转移

```
WAIT_ENTRY ──(buy BSP + L2多 + L0确认)──→ ENTRY ──→ HOLDING
   ↑                                                  │
   │                                    sell BSP/L1r1↓ │
 OBSERVE ←── WAIT_DIP ←──────── EVAL ←────────────────┘
                                  │
                          不创新高/盘整背驰
                                  ↓
                          EXIT → WAIT_ENTRY
```

### PH三级别信号映射

| PH级别 | 输入 | 更新频率 | 信号用途 |
|--------|------|---------|---------|
| L0 | 1min close | 每bar | 区间套精确入场/回调结束确认 |
| L1 | L1线段端点 | ~数千次 | 降成本+走势完成信号+加仓 |
| L2 | L1走势端点 | ~数百次 | 方向裁决/紧急清仓 |

## QQQ

- 数据：**728,030** bars (1min)
- 价格：268.82 → 738.28
- Buy-and-hold: **+174.64%**
- 回测耗时：415.3s

### PH 分层统计

| 级别 | 更新次数 | settle总数 | rank-1 settle |
|------|---------|-----------|---------------|
| L0 | 728,030 (每bar) | 350,072 | 3962 |
| L1 | 6,119 | 5,804 | 286 |
| L2 | 389 | 190 | 25 |
| **L2 方向翻转** | — | — | **14** |
| **EVAL** | 触发=32 | 过滤=64 | 过滤率=67% |

### 总体指标

| 指标 | 值 |
|------|-----|
| 交易数 | 17（纯多头） |
| 胜率 | 47.1% |
| 平均收益 | +1.383% |
| 复利累计 | **+23.69%** |
| 最大回撤 | -12.91% |
| 平均持仓 | 17,875 bars |
| 有加仓的交易 | 0/17 |
| 有降成本的交易 | 15/17 |
| 达到本金回收 | 0/17 |
| 有段间比较的交易 | 13/17 |

### 交易明细

| # | 入场 | 出场 | 持仓bars | PnL% | 加仓 | 短差 | 向上段数 | 退出原因 | 状态路径 |
|---|------|------|---------|------|------|------|---------|---------|---------|
| 1 | 309.96@45988 | 320.85@74564 | 28,576 | -2.05 | 0 | 25 | 3 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY→WAIT_SUB_EXIT |
| 2 | 327.53@78022 | 383.88@119128 | 41,106 | +17.63 | 0 | 40 | 2 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY→WAIT_SUB_EXIT |
| 3 | 390.34@186088 | 406.96@215144 | 29,056 | +0.69 | 0 | 28 | 3 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY→WAIT_SUB_EXIT |
| 4 | 413.04@216547 | 439.25@243258 | 26,711 | +9.17 | 0 | 22 | 3 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY→WAIT_SUB_EXIT |
| 5 | 448.55@252551 | 444.06@264314 | 11,763 | -1.78 | 0 | 14 | 2 | no_new_high | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY→WAIT_SUB_EXIT |
| 6 | 456.21@288234 | 451.21@294537 | 6,303 | -2.47 | 0 | 6 | 2 | no_new_high | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY→WAIT_SUB_EXIT |
| 7 | 464.26@296170 | 443.17@330925 | 34,755 | -3.27 | 0 | 39 | 2 | l2_direction_flip_short | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY |
| 8 | 520.33@400673 | 513.81@419345 | 18,672 | -2.73 | 0 | 14 | 2 | no_new_high | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY→WAIT_SUB_EXIT |
| 9 | 540.25@440555 | 498.50@448805 | 8,250 | -3.38 | 0 | 11 | 1 | l2_direction_flip_short | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY |
| 10 | 543.85@520909 | 558.27@532548 | 11,639 | +2.34 | 0 | 12 | 2 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY→WAIT_SUB_EXIT |
| 11 | 561.70@532951 | 577.89@564430 | 31,479 | +2.36 | 0 | 27 | 2 | no_new_high | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY→WAIT_SUB_EXIT |
| 12 | 582.25@566235 | 605.02@593225 | 26,990 | -0.59 | 0 | 23 | 3 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY→WAIT_SUB_EXIT |
| 13 | 615.91@594522 | 632.76@599903 | 5,381 | +2.54 | 0 | 2 | 2 | no_new_high | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY→WAIT_SUB_EXIT |
| 14 | 617.83@601479 | 617.32@601511 | 32 | -0.08 | 0 | 0 | 0 | bsp_invalidate | ENTRY→HOLDING→WAIT_SUB_ENTRY |
| 15 | 602.60@603722 | 603.75@603769 | 47 | +0.19 | 0 | 0 | 0 | bsp_invalidate | ENTRY→HOLDING→WAIT_SUB_ENTRY |
| 16 | 650.02@699743 | 703.67@717492 | 17,749 | +5.51 | 0 | 19 | 3 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY→WAIT_SUB_EXIT |
| 17 | 734.98@722663 | 738.28@728029 | 5,366 | -0.58 | 0 | 6 | 0 | eod_close | ENTRY→HOLDING→WAIT_SUB_ENTRY |

### FSM 状态分布（按 bar 数）

| 状态 | Bars | 占比 |
|------|------|------|
| WAIT_ENTRY | 423,053 | 58.1% |
| ENTRY | 19 | 0.0% |
| WAIT_SUB_ENTRY | 1,083 | 0.1% |
| HOLDING | 294,918 | 40.5% |
| EVAL | 0 | 0.0% |
| WAIT_DIP | 8,225 | 1.1% |
| OBSERVE | 0 | 0.0% |
| WAIT_SUB_EXIT | 732 | 0.1% |
| EXIT | 0 | 0.0% |

<details><summary>事件流（778条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 45,926 | 313.67 | ENTRY | WAIT→ENTRY BSP={1699618109} |
| 45,927 | 313.83 | WAIT_SUB_ENTRY | ENTRY→WAIT_SUB@313.83 L2dir=1 |
| 45,988 | 309.96 | HOLDING | SUB_ENTRY_TIMEOUT@309.96 |
| 46,025 | 309.74 | HOLDING | TRIM@309.74 shares=318.3 |
| 46,384 | 309.40 | HOLDING | MOVE_SETTLE_SKIP(p=13.26<med=54.06) |
| 46,714 | 313.12 | HOLDING | CLOSE_DIFF@313.12 profit=-1075.94 |
| 47,157 | 310.23 | HOLDING | TRIM@310.23 shares=321.1 |
| 47,393 | 308.80 | HOLDING | MOVE_SETTLE_SKIP(p=8.57<med=31.08) |
| 47,712 | 308.69 | HOLDING | CLOSE_DIFF@308.69 profit=494.54 |
| 48,740 | 309.02 | HOLDING | TRIM@309.02 shares=321.1 |
| 49,360 | 306.38 | HOLDING | CLOSE_DIFF@306.38 profit=847.78 |
| 49,754 | 307.47 | HOLDING | TRIM@307.47 shares=318.3 |
| 49,911 | 309.79 | HOLDING | CLOSE_DIFF@309.79 profit=-738.51 |
| 51,694 | 315.66 | HOLDING | TRIM@315.66 shares=320.9 |
| 52,243 | 321.03 | HOLDING | CLOSE_DIFF@321.03 profit=-1723.20 |
| 52,737 | 319.50 | HOLDING | TRIM@319.50 shares=296.9 |
| 52,767 | 319.20 | HOLDING | CLOSE_DIFF@319.20 profit=89.08 |
| 52,924 | 318.36 | HOLDING | TRIM@318.36 shares=296.9 |
| 53,510 | 321.52 | HOLDING | MOVE_SETTLE_SKIP(p=14.07<med=61.83) |
| 53,767 | 319.65 | HOLDING | TRIM@319.65 shares=317.4 |
| 54,238 | 318.54 | HOLDING | CLOSE_DIFF@318.54 profit=352.26 |
| 54,689 | 314.50 | WAIT_DIP | →EVAL(move_settle) high=322.17 force=4.65 p=4.65/med=4.34 → WAIT_DIP(U1完成 high=322.17) |
| 55,783 | 318.10 | HOLDING | DIP_REBUY_DIVERGE@318.10 跌破+盘整背驰 |
| 56,300 | 314.09 | HOLDING | TRIM@314.09 shares=300.8 |
| 56,776 | 317.96 | HOLDING | CLOSE_DIFF@317.96 profit=-1163.94 |
| 57,050 | 317.44 | HOLDING | TRIM@317.44 shares=301.1 |
| 57,965 | 316.33 | HOLDING | CLOSE_DIFF@316.33 profit=334.19 |
| 59,326 | 319.25 | HOLDING | TRIM@319.25 shares=283.2 |
| 59,355 | 318.60 | HOLDING | CLOSE_DIFF@318.60 profit=184.06 |
| 59,641 | 318.98 | HOLDING | TRIM@318.98 shares=307.5 |
| 59,691 | 317.88 | HOLDING | CLOSE_DIFF@317.88 profit=339.83 |
| 60,688 | 316.73 | HOLDING | TRIM@316.73 shares=313.1 |
| 61,111 | 320.86 | HOLDING | CLOSE_DIFF@320.86 profit=-1293.21 |
| 61,452 | 318.47 | HOLDING | TRIM@318.47 shares=318.1 |
| 61,737 | 318.81 | HOLDING | CLOSE_DIFF@318.81 profit=-108.17 |
| 61,990 | 316.18 | HOLDING | TRIM@316.18 shares=318.1 |
| 62,056 | 316.54 | HOLDING | CLOSE_DIFF@316.54 profit=-114.53 |
| 62,990 | 315.98 | HOLDING | TRIM@315.98 shares=318.1 |
| 63,201 | 317.77 | HOLDING | CLOSE_DIFF@317.77 profit=-569.46 |
| 63,419 | 315.96 | HOLDING | TRIM@315.96 shares=318.1 |
| 64,041 | 316.37 | HOLDING | CLOSE_DIFF@316.37 profit=-130.44 |
| 64,532 | 316.18 | HOLDING | TRIM@316.18 shares=318.1 |
| 64,625 | 316.38 | HOLDING | CLOSE_DIFF@316.38 profit=-63.63 |
| 65,238 | 315.87 | HOLDING | TRIM@315.87 shares=318.1 |
| 66,242 | 313.83 | HOLDING | CLOSE_DIFF@313.83 profit=649.00 |
| 66,818 | 311.68 | HOLDING | TRIM@311.68 shares=282.1 |
| 67,168 | 314.62 | HOLDING | CLOSE_DIFF@314.62 profit=-829.31 |
| 68,196 | 320.04 | HOLDING | TRIM@320.04 shares=312.1 |
| 68,316 | 319.31 | HOLDING | CLOSE_DIFF@319.31 profit=227.82 |
| 68,893 | 322.36 | HOLDING | TRIM@322.36 shares=318.9 |
| 69,320 | 322.04 | HOLDING | CLOSE_DIFF@322.04 profit=102.03 |
| 69,405 | 322.46 | HOLDING | TRIM@322.46 shares=319.0 |
| 69,827 | 322.03 | HOLDING | CLOSE_DIFF@322.03 profit=137.15 |
| 71,205 | 316.80 | HOLDING | TRIM@316.80 shares=315.2 |
| 71,368 | 318.03 | WAIT_DIP | →EVAL(move_settle) high=323.62 force=10.29 p=10.29/med=1.90 → WAIT_DIP(创新高无背驰 high=323.62) |
| 72,003 | 316.08 | HOLDING | DIP_REBUY@316.08 不跌破前低(down_low=315.28 >= prev_low=313.33) |
| 72,362 | 318.33 | HOLDING | TRIM@318.33 shares=315.2 |
| 72,488 | 321.22 | HOLDING | CLOSE_DIFF@321.22 profit=-911.01 |
| 73,460 | 323.47 | HOLDING | TRIM@323.47 shares=321.9 |
| 74,028 | 322.43 | HOLDING | CLOSE_DIFF@322.43 profit=334.79 |
| 74,074 | 321.91 | HOLDING | TRIM@321.91 shares=320.5 |
| 74,503 | 321.04 | WAIT_SUB_EXIT | →EVAL(move_settle) high=324.04 force=6.56 p=6.56/med=2.31 → WAIT_SUB_EXIT(盘整背驰 force 6.56<10.29) |
| 74,564 | 320.85 | WAIT_ENTRY | SUB_EXIT_TIMEOUT@320.85 |
| 77,960 | 327.57 | ENTRY | WAIT→ENTRY BSP={1693328398} |
| 77,961 | 327.57 | WAIT_SUB_ENTRY | ENTRY→WAIT_SUB@327.57 L2dir=1 |
| 78,022 | 327.53 | HOLDING | SUB_ENTRY_TIMEOUT@327.53 |
| 78,440 | 327.44 | HOLDING | TRIM@327.44 shares=299.3 |
| 78,534 | 327.64 | HOLDING | CLOSE_DIFF@327.64 profit=-59.85 |
| 78,826 | 328.60 | HOLDING | TRIM@328.60 shares=302.4 |
| 79,420 | 332.09 | HOLDING | CLOSE_DIFF@332.09 profit=-1055.33 |
| 80,484 | 336.69 | HOLDING | TRIM@336.69 shares=299.5 |
| 81,188 | 336.32 | HOLDING | CLOSE_DIFF@336.32 profit=110.81 |
| 81,552 | 338.07 | HOLDING | MOVE_SETTLE_SKIP(p=15.94<med=77.77) |
| 82,391 | 333.84 | HOLDING | TRIM@333.84 shares=296.0 |
| 82,871 | 331.49 | HOLDING | MOVE_SETTLE_SKIP(p=3.35<med=78.44) |
| 82,996 | 330.63 | HOLDING | CLOSE_DIFF@330.63 profit=950.20 |
| 85,792 | 351.01 | HOLDING | TRIM@351.01 shares=289.0 |
| 86,208 | 349.29 | HOLDING | MOVE_SETTLE_SKIP(p=23.78<med=93.84) |
| 86,834 | 347.42 | HOLDING | CLOSE_DIFF@347.42 profit=1037.56 |
| 87,198 | 348.05 | HOLDING | TRIM@348.05 shares=281.4 |
| 87,236 | 347.99 | HOLDING | CLOSE_DIFF@347.99 profit=16.88 |
| 88,680 | 354.29 | HOLDING | TRIM@354.29 shares=300.1 |
| 89,039 | 357.00 | HOLDING | MOVE_SETTLE_SKIP(p=8.63<med=97.16) |
| 89,140 | 355.06 | HOLDING | TRIM@355.06 shares=300.1 |
| 89,682 | 355.18 | HOLDING | CLOSE_DIFF@355.18 profit=-36.01 |
| 89,725 | 355.45 | HOLDING | TRIM@355.45 shares=300.1 |
| 90,796 | 348.62 | HOLDING | MOVE_SETTLE_SKIP(p=4.00<med=50.58) |
| 91,245 | 349.80 | HOLDING | CLOSE_DIFF@349.80 profit=1695.29 |
| 92,884 | 356.25 | HOLDING | TRIM@356.25 shares=304.8 |
| 92,986 | 356.98 | HOLDING | CLOSE_DIFF@356.98 profit=-222.50 |
| 93,739 | 361.71 | HOLDING | TRIM@361.71 shares=285.0 |
| 93,922 | 363.24 | HOLDING | CLOSE_DIFF@363.24 profit=-436.04 |
| 94,011 | 362.72 | HOLDING | TRIM@362.72 shares=302.7 |
| 94,978 | 366.34 | HOLDING | CLOSE_DIFF@366.34 profit=-1095.70 |
| 95,575 | 368.10 | HOLDING | TRIM@368.10 shares=303.4 |
| 95,754 | 370.23 | HOLDING | CLOSE_DIFF@370.23 profit=-646.23 |
| 95,904 | 369.83 | HOLDING | TRIM@369.83 shares=293.0 |
| 96,071 | 370.83 | HOLDING | CLOSE_DIFF@370.83 profit=-292.99 |
| 96,272 | 371.02 | HOLDING | TRIM@371.02 shares=291.8 |
| 97,237 | 364.91 | HOLDING | MOVE_SETTLE_SKIP(p=23.92<med=111.46) |
| 97,689 | 366.89 | HOLDING | CLOSE_DIFF@366.89 profit=1205.07 |
| 98,307 | 361.91 | HOLDING | TRIM@361.91 shares=289.9 |
| 98,564 | 360.88 | HOLDING | CLOSE_DIFF@360.88 profit=298.65 |
| 99,564 | 362.44 | HOLDING | TRIM@362.44 shares=288.6 |
| 100,608 | 358.67 | HOLDING | CLOSE_DIFF@358.67 profit=1089.38 |
| 101,209 | 359.25 | HOLDING | TRIM@359.25 shares=272.1 |
| 101,466 | 363.83 | HOLDING | CLOSE_DIFF@363.83 profit=-1246.29 |
| 101,841 | 362.66 | HOLDING | TRIM@362.66 shares=284.7 |
| 101,908 | 362.42 | HOLDING | CLOSE_DIFF@362.42 profit=68.32 |
| 102,015 | 362.83 | HOLDING | TRIM@362.83 shares=284.7 |
| 102,161 | 366.43 | HOLDING | CLOSE_DIFF@366.43 profit=-1024.78 |
| 102,365 | 363.92 | HOLDING | TRIM@363.92 shares=290.0 |
| 102,477 | 365.15 | HOLDING | CLOSE_DIFF@365.15 profit=-356.74 |
| 102,594 | 365.16 | HOLDING | TRIM@365.16 shares=290.0 |
| 103,003 | 363.99 | HOLDING | CLOSE_DIFF@363.99 profit=339.34 |
| 103,027 | 364.28 | HOLDING | TRIM@364.28 shares=290.1 |
| 103,454 | 365.68 | HOLDING | CLOSE_DIFF@365.68 profit=-406.16 |
| 104,566 | 369.13 | HOLDING | TRIM@369.13 shares=302.9 |
| 105,314 | 370.52 | HOLDING | CLOSE_DIFF@370.52 profit=-421.08 |
| 105,394 | 370.71 | HOLDING | TRIM@370.71 shares=305.2 |
| 105,941 | 365.05 | HOLDING | CLOSE_DIFF@365.05 profit=1727.62 |
| 106,009 | 366.34 | HOLDING | TRIM@366.34 shares=305.2 |
| 106,252 | 367.58 | HOLDING | CLOSE_DIFF@367.58 profit=-378.49 |
| 106,497 | 367.04 | HOLDING | TRIM@367.04 shares=305.2 |
| 106,548 | 366.99 | HOLDING | CLOSE_DIFF@366.99 profit=15.26 |
| 106,629 | 367.16 | HOLDING | TRIM@367.16 shares=305.2 |
| 106,682 | 366.68 | HOLDING | CLOSE_DIFF@366.68 profit=146.51 |
| 107,099 | 366.50 | HOLDING | TRIM@366.50 shares=305.2 |
| 107,237 | 364.82 | HOLDING | CLOSE_DIFF@364.82 profit=512.79 |
| 107,503 | 365.73 | HOLDING | TRIM@365.73 shares=305.2 |
| 108,069 | 366.99 | HOLDING | CLOSE_DIFF@366.99 profit=-384.59 |
| 108,299 | 366.88 | HOLDING | TRIM@366.88 shares=305.2 |
| 108,383 | 366.56 | HOLDING | CLOSE_DIFF@366.56 profit=97.67 |
| 108,554 | 366.96 | HOLDING | TRIM@366.96 shares=305.2 |
| 108,820 | 368.98 | HOLDING | CLOSE_DIFF@368.98 profit=-616.57 |
| 110,916 | 380.20 | HOLDING | TRIM@380.20 shares=298.1 |
| 111,605 | 380.26 | HOLDING | CLOSE_DIFF@380.26 profit=-17.89 |
| 111,754 | 381.36 | WAIT_DIP | →EVAL(move_settle) high=382.86 force=18.24 p=18.24/med=4.68 → WAIT_DIP(U1完成 high=382.86) |
| 111,843 | 382.39 | HOLDING | DIP_REBUY@382.39 不跌破前低(down_low=381.23 >= prev_low=364.62) |
| 112,060 | 382.55 | HOLDING | TRIM@382.55 shares=304.2 |
| 112,096 | 382.10 | HOLDING | CLOSE_DIFF@382.10 profit=136.91 |
| 112,795 | 385.24 | HOLDING | TRIM@385.24 shares=304.7 |
| 112,977 | 386.31 | HOLDING | CLOSE_DIFF@386.31 profit=-325.99 |
| 113,388 | 385.11 | HOLDING | TRIM@385.11 shares=293.8 |
| 114,270 | 378.90 | HOLDING | MOVE_SETTLE_SKIP(p=9.80<med=127.31) |
| 114,885 | 377.83 | HOLDING | CLOSE_DIFF@377.83 profit=2139.07 |
| 114,940 | 377.28 | HOLDING | TRIM@377.28 shares=286.8 |
| 115,159 | 376.99 | HOLDING | CLOSE_DIFF@376.99 profit=83.17 |
| 115,834 | 376.49 | HOLDING | TRIM@376.49 shares=276.4 |
| 116,422 | 377.45 | HOLDING | CLOSE_DIFF@377.45 profit=-265.36 |
| ... | ... | ... | （共778条，显示前150） |
</details>

## OKLO

- 数据：**333,613** bars (1min)
- 价格：18.07 → 63.48
- Buy-and-hold: **+251.30%**
- 回测耗时：78.0s

### PH 分层统计

| 级别 | 更新次数 | settle总数 | rank-1 settle |
|------|---------|-----------|---------------|
| L0 | 333,613 (每bar) | 154,247 | 730 |
| L1 | 2,806 | 2,657 | 102 |
| L2 | 185 | 101 | 12 |
| **L2 方向翻转** | — | — | **7** |
| **EVAL** | 触发=8 | 过滤=16 | 过滤率=67% |

### 总体指标

| 指标 | 值 |
|------|-----|
| 交易数 | 6（纯多头） |
| 胜率 | 83.3% |
| 平均收益 | +15.139% |
| 复利累计 | **+114.08%** |
| 最大回撤 | -11.03% |
| 平均持仓 | 11,430 bars |
| 有加仓的交易 | 0/6 |
| 有降成本的交易 | 4/6 |
| 达到本金回收 | 0/6 |
| 有段间比较的交易 | 3/6 |

### 交易明细

| # | 入场 | 出场 | 持仓bars | PnL% | 加仓 | 短差 | 向上段数 | 退出原因 | 状态路径 |
|---|------|------|---------|------|------|------|---------|---------|---------|
| 1 | 14.77@41153 | 23.35@54650 | 13,497 | +33.72 | 0 | 11 | 3 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY→WAIT_SUB_EXIT |
| 2 | 32.57@82333 | 49.49@96226 | 13,893 | +42.48 | 0 | 11 | 3 | no_new_high | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY→WAIT_SUB_EXIT |
| 3 | 64.15@151396 | 65.64@151540 | 144 | +2.32 | 0 | 0 | 0 | bsp_invalidate | ENTRY→HOLDING→WAIT_SUB_ENTRY |
| 4 | 75.00@173754 | 80.19@181396 | 7,642 | -11.03 | 0 | 9 | 2 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY→WAIT_SUB_EXIT |
| 5 | 64.14@187276 | 64.39@187431 | 155 | +0.39 | 0 | 0 | 0 | bsp_invalidate | ENTRY→HOLDING→WAIT_SUB_ENTRY |
| 6 | 90.36@199556 | 117.34@232808 | 33,252 | +22.94 | 0 | 36 | 0 | l2_direction_flip_short | ENTRY→HOLDING→WAIT_SUB_ENTRY |

### FSM 状态分布（按 bar 数）

| 状态 | Bars | 占比 |
|------|------|------|
| WAIT_ENTRY | 264,658 | 79.3% |
| ENTRY | 6 | 0.0% |
| WAIT_SUB_ENTRY | 366 | 0.1% |
| HOLDING | 66,755 | 20.0% |
| EVAL | 0 | 0.0% |
| WAIT_DIP | 1,645 | 0.5% |
| OBSERVE | 0 | 0.0% |
| WAIT_SUB_EXIT | 183 | 0.1% |
| EXIT | 0 | 0.0% |

<details><summary>事件流（186条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 41,091 | 13.08 | ENTRY | WAIT→ENTRY BSP={1146069223} |
| 41,092 | 13.00 | WAIT_SUB_ENTRY | ENTRY→WAIT_SUB@13.00 L2dir=1 |
| 41,153 | 14.77 | HOLDING | SUB_ENTRY_TIMEOUT@14.77 |
| 41,862 | 16.22 | HOLDING | TRIM@16.22 shares=6098.9 |
| 42,565 | 15.84 | HOLDING | CLOSE_DIFF@15.84 profit=2317.60 |
| 42,786 | 19.43 | HOLDING | MOVE_SETTLE_SKIP(p=8.58<med=15.15) |
| 43,016 | 18.46 | HOLDING | TRIM@18.46 shares=6039.6 |
| 43,450 | 19.91 | HOLDING | CLOSE_DIFF@19.91 profit=-8757.37 |
| 43,481 | 20.04 | HOLDING | MOVE_SETTLE_SKIP(p=6.51<med=16.18) |
| 44,516 | 20.63 | HOLDING | TRIM@20.63 shares=6238.4 |
| 44,992 | 20.44 | HOLDING | CLOSE_DIFF@20.44 profit=1185.30 |
| 45,323 | 18.34 | WAIT_DIP | →EVAL(move_settle) high=23.05 force=5.85 p=5.85/med=3.16 → WAIT_DIP(U1完成 high=23.05) |
| 46,744 | 19.14 | HOLDING | DIP_REBUY@19.14 不跌破前低(down_low=17.62 >= prev_low=17.20) |
| 47,960 | 23.43 | HOLDING | TRIM@23.43 shares=6527.8 |
| 48,481 | 26.75 | HOLDING | CLOSE_DIFF@26.75 profit=-21672.37 |
| 48,715 | 23.60 | HOLDING | TRIM@23.60 shares=6017.2 |
| 49,561 | 22.99 | HOLDING | CLOSE_DIFF@22.99 profit=3670.49 |
| 49,609 | 22.92 | HOLDING | TRIM@22.92 shares=6017.2 |
| 50,300 | 21.43 | WAIT_DIP | →EVAL(move_settle) high=28.12 force=10.00 p=10.00/med=5.26 → WAIT_DIP(创新高无背驰 high=28.12) |
| 50,307 | 21.52 | HOLDING | DIP_REBUY@21.52 不跌破前低(down_low=21.36 >= prev_low=18.12) |
| 50,377 | 21.56 | HOLDING | TRIM@21.56 shares=5801.5 |
| 50,507 | 20.36 | HOLDING | CLOSE_DIFF@20.36 profit=6961.84 |
| 50,922 | 20.13 | HOLDING | TRIM@20.13 shares=5333.8 |
| 51,314 | 21.06 | HOLDING | CLOSE_DIFF@21.06 profit=-4960.40 |
| 51,654 | 20.43 | HOLDING | TRIM@20.43 shares=5333.8 |
| 52,297 | 22.03 | HOLDING | CLOSE_DIFF@22.03 profit=-8534.03 |
| 52,845 | 25.04 | HOLDING | TRIM@25.04 shares=5358.1 |
| 53,016 | 25.61 | HOLDING | CLOSE_DIFF@25.61 profit=-3054.10 |
| 53,605 | 24.94 | HOLDING | TRIM@24.94 shares=6728.0 |
| 53,907 | 24.36 | HOLDING | CLOSE_DIFF@24.36 profit=3902.21 |
| 53,993 | 24.40 | HOLDING | TRIM@24.40 shares=6728.0 |
| 54,119 | 23.72 | HOLDING | CLOSE_DIFF@23.72 profit=4575.01 |
| 54,228 | 23.83 | HOLDING | TRIM@23.83 shares=6728.0 |
| 54,589 | 23.20 | WAIT_SUB_EXIT | →EVAL(move_settle) high=28.68 force=8.68 p=8.68/med=5.93 → WAIT_SUB_EXIT(盘整背驰 force 8.68<10.00) |
| 54,650 | 23.35 | WAIT_ENTRY | SUB_EXIT_TIMEOUT@23.35 |
| 82,271 | 31.98 | ENTRY | WAIT→ENTRY BSP={1567619886} |
| 82,272 | 32.84 | WAIT_SUB_ENTRY | ENTRY→WAIT_SUB@32.84 L2dir=1 |
| 82,333 | 32.57 | HOLDING | SUB_ENTRY_TIMEOUT@32.57 |
| 83,207 | 32.37 | HOLDING | TRIM@32.37 shares=2710.5 |
| 83,593 | 38.78 | HOLDING | CLOSE_DIFF@38.78 profit=-17374.36 |
| 84,429 | 42.84 | HOLDING | TRIM@42.84 shares=2657.0 |
| 84,463 | 42.52 | HOLDING | CLOSE_DIFF@42.52 profit=850.23 |
| 84,755 | 35.90 | HOLDING | TRIM@35.90 shares=2405.3 |
| 85,820 | 31.29 | HOLDING | MOVE_SETTLE_SKIP(p=19.47<med=37.58) |
| 86,173 | 33.53 | HOLDING | CLOSE_DIFF@33.53 profit=5700.48 |
| 88,040 | 41.95 | HOLDING | TRIM@41.95 shares=3040.9 |
| 88,940 | 43.40 | HOLDING | CLOSE_DIFF@43.40 profit=-4424.51 |
| 88,990 | 44.60 | WAIT_DIP | →EVAL(move_settle) high=45.21 force=12.21 p=12.21/med=7.38 → WAIT_DIP(U1完成 high=45.21) |
| 89,005 | 45.42 | HOLDING | DIP_REBUY@45.42 不跌破前低(down_low=44.35 >= prev_low=33.00) |
| 89,466 | 46.55 | HOLDING | TRIM@46.55 shares=3030.2 |
| 90,008 | 45.95 | HOLDING | CLOSE_DIFF@45.95 profit=1818.10 |
| 91,119 | 48.95 | HOLDING | TRIM@48.95 shares=2746.8 |
| 91,475 | 48.63 | HOLDING | CLOSE_DIFF@48.63 profit=865.24 |
| 92,285 | 54.89 | HOLDING | TRIM@54.89 shares=2978.7 |
| 92,312 | 55.20 | HOLDING | CLOSE_DIFF@55.20 profit=-923.41 |
| 92,466 | 54.30 | HOLDING | TRIM@54.30 shares=2978.7 |
| 93,016 | 52.38 | HOLDING | CLOSE_DIFF@52.38 profit=5704.30 |
| 93,454 | 50.88 | HOLDING | TRIM@50.88 shares=2785.1 |
| 93,772 | 52.73 | HOLDING | CLOSE_DIFF@52.73 profit=-5138.43 |
| 93,829 | 54.11 | WAIT_DIP | →EVAL(move_settle) high=59.11 force=21.28 p=21.28/med=4.49 → WAIT_DIP(创新高无背驰 high=59.11) |
| 93,903 | 54.91 | HOLDING | DIP_REBUY@54.91 不跌破前低(down_low=53.89 >= prev_low=37.83) |
| 94,200 | 54.69 | HOLDING | TRIM@54.69 shares=2923.9 |
| 94,334 | 53.96 | HOLDING | CLOSE_DIFF@53.96 profit=2134.42 |
| 95,174 | 52.14 | HOLDING | TRIM@52.14 shares=3002.2 |
| 95,733 | 51.70 | HOLDING | CLOSE_DIFF@51.70 profit=1320.98 |
| 95,806 | 50.74 | HOLDING | TRIM@50.74 shares=3002.2 |
| 96,165 | 49.62 | WAIT_SUB_EXIT | →EVAL(move_settle) high=57.49 force=10.39 p=10.39/med=10.28 → WAIT_SUB_EXIT(不创新高 57.49<59.11) |
| 96,226 | 49.49 | WAIT_ENTRY | SUB_EXIT_TIMEOUT@49.49 |
| 151,334 | 66.80 | ENTRY | WAIT→ENTRY BSP={528094879} |
| 151,335 | 66.89 | WAIT_SUB_ENTRY | ENTRY→WAIT_SUB@66.89 L2dir=1 |
| 151,396 | 64.15 | HOLDING | SUB_ENTRY_TIMEOUT@64.15 |
| 151,540 | 65.64 | WAIT_ENTRY | BSP_INVALIDATE@65.64 |
| 173,692 | 76.58 | ENTRY | WAIT→ENTRY BSP={1926062708} |
| 173,693 | 76.66 | WAIT_SUB_ENTRY | ENTRY→WAIT_SUB@76.66 L2dir=1 |
| 173,754 | 75.00 | HOLDING | SUB_ENTRY_TIMEOUT@75.00 |
| 173,991 | 72.22 | HOLDING | TRIM@72.22 shares=1231.5 |
| 174,553 | 75.00 | WAIT_DIP | →EVAL(move_settle) high=77.16 force=15.42 p=15.42/med=5.21 → WAIT_DIP(U1完成 high=77.16) |
| 174,681 | 77.17 | HOLDING | DIP_REBUY@77.17 不跌破前低(down_low=74.81 >= prev_low=61.74) |
| 175,060 | 74.46 | HOLDING | TRIM@74.46 shares=1294.8 |
| 175,622 | 75.20 | HOLDING | CLOSE_DIFF@75.20 profit=-958.17 |
| 176,091 | 71.38 | HOLDING | MOVE_SETTLE_SKIP(p=7.32<med=7.93) |
| 176,122 | 71.57 | HOLDING | MOVE_SETTLE_SKIP(p=7.32<med=7.93) |
| 176,234 | 71.40 | HOLDING | TRIM@71.40 shares=1247.2 |
| 176,398 | 75.80 | HOLDING | CLOSE_DIFF@75.80 profit=-5481.37 |
| 177,024 | 77.30 | HOLDING | TRIM@77.30 shares=1329.2 |
| 178,300 | 71.49 | HOLDING | MOVE_SETTLE_SKIP(p=9.23<med=74.93) |
| 178,793 | 76.59 | HOLDING | CLOSE_DIFF@76.59 profit=943.75 |
| 179,021 | 76.50 | HOLDING | TRIM@76.50 shares=1296.5 |
| 179,779 | 83.38 | HOLDING | CLOSE_DIFF@83.38 profit=-8919.91 |
| 180,016 | 83.37 | HOLDING | TRIM@83.37 shares=1315.6 |
| 180,042 | 83.42 | HOLDING | CLOSE_DIFF@83.42 profit=-65.78 |
| 180,153 | 83.62 | HOLDING | TRIM@83.62 shares=1315.6 |
| 180,450 | 84.16 | HOLDING | CLOSE_DIFF@84.16 profit=-703.84 |
| 180,559 | 83.27 | HOLDING | TRIM@83.27 shares=1317.1 |
| 180,836 | 84.40 | HOLDING | CLOSE_DIFF@84.40 profit=-1488.36 |
| 181,177 | 78.82 | HOLDING | TRIM@78.82 shares=1251.3 |
| 181,250 | 79.65 | HOLDING | CLOSE_DIFF@79.65 profit=-1038.61 |
| 181,335 | 79.85 | WAIT_SUB_EXIT | →EVAL(move_settle) high=85.33 force=13.73 p=13.73/med=6.62 → WAIT_SUB_EXIT(盘整背驰 force 13.73<15.42) |
| 181,396 | 80.19 | WAIT_ENTRY | SUB_EXIT_TIMEOUT@80.19 |
| 187,214 | 64.98 | ENTRY | WAIT→ENTRY BSP={1974301457} |
| 187,215 | 65.04 | WAIT_SUB_ENTRY | ENTRY→WAIT_SUB@65.04 L2dir=1 |
| 187,276 | 64.14 | HOLDING | SUB_ENTRY_TIMEOUT@64.14 |
| 187,431 | 64.39 | WAIT_ENTRY | BSP_INVALIDATE@64.39 |
| 199,494 | 88.82 | ENTRY | WAIT→ENTRY BSP={545782922} |
| 199,495 | 89.23 | WAIT_SUB_ENTRY | ENTRY→WAIT_SUB@89.23 L2dir=1 |
| 199,556 | 90.36 | HOLDING | SUB_ENTRY_TIMEOUT@90.36 |
| 200,406 | 91.20 | HOLDING | TRIM@91.20 shares=1045.6 |
| 200,579 | 94.58 | HOLDING | CLOSE_DIFF@94.58 profit=-3534.22 |
| 201,126 | 94.68 | HOLDING | TRIM@94.68 shares=1106.0 |
| 201,832 | 100.44 | HOLDING | CLOSE_DIFF@100.44 profit=-6370.29 |
| 203,456 | 131.41 | HOLDING | TRIM@131.41 shares=1070.7 |
| 203,886 | 138.20 | HOLDING | MOVE_SETTLE_SKIP(p=65.27<med=137.34) |
| 204,334 | 137.28 | HOLDING | TRIM@137.28 shares=1061.5 |
| 205,165 | 140.50 | HOLDING | CLOSE_DIFF@140.50 profit=-3417.96 |
| 205,261 | 136.24 | HOLDING | TRIM@136.24 shares=1095.5 |
| 205,442 | 138.40 | HOLDING | CLOSE_DIFF@138.40 profit=-2366.32 |
| 205,518 | 136.88 | HOLDING | TRIM@136.88 shares=1095.5 |
| 206,090 | 110.98 | HOLDING | MOVE_SETTLE_SKIP(p=21.25<med=74.78) |
| 207,032 | 118.36 | HOLDING | CLOSE_DIFF@118.36 profit=20288.98 |
| 207,189 | 109.70 | HOLDING | TRIM@109.70 shares=932.0 |
| 207,376 | 109.15 | HOLDING | CLOSE_DIFF@109.15 profit=512.60 |
| 207,413 | 110.77 | HOLDING | TRIM@110.77 shares=932.0 |
| 207,459 | 109.95 | HOLDING | CLOSE_DIFF@109.95 profit=759.58 |
| 208,304 | 116.80 | HOLDING | TRIM@116.80 shares=932.0 |
| 208,714 | 115.40 | HOLDING | CLOSE_DIFF@115.40 profit=1304.79 |
| 208,775 | 112.10 | HOLDING | TRIM@112.10 shares=938.1 |
| 209,446 | 112.28 | HOLDING | CLOSE_DIFF@112.28 profit=-168.86 |
| 209,977 | 116.04 | HOLDING | TRIM@116.04 shares=938.1 |
| 210,086 | 118.12 | HOLDING | MOVE_SETTLE_SKIP(p=12.22<med=32.93) |
| 211,329 | 127.36 | HOLDING | TRIM@127.36 shares=989.2 |
| 212,217 | 144.43 | HOLDING | MOVE_SETTLE_SKIP(p=21.71<med=139.49) |
| 212,526 | 143.05 | HOLDING | TRIM@143.05 shares=1100.7 |
| 213,019 | 135.42 | HOLDING | MOVE_SETTLE_SKIP(p=24.03<med=139.49) |
| 213,422 | 136.43 | HOLDING | CLOSE_DIFF@136.43 profit=7286.86 |
| 213,823 | 134.18 | HOLDING | TRIM@134.18 shares=1100.7 |
| 213,849 | 133.56 | HOLDING | CLOSE_DIFF@133.56 profit=682.46 |
| 213,994 | 131.60 | HOLDING | TRIM@131.60 shares=1100.7 |
| 214,099 | 134.69 | HOLDING | CLOSE_DIFF@134.69 profit=-3401.27 |
| 214,301 | 138.59 | HOLDING | TRIM@138.59 shares=1100.7 |
| 214,373 | 138.21 | HOLDING | CLOSE_DIFF@138.21 profit=418.28 |
| 214,635 | 138.20 | HOLDING | TRIM@138.20 shares=1100.7 |
| 215,346 | 137.55 | HOLDING | CLOSE_DIFF@137.55 profit=715.48 |
| 215,408 | 140.59 | HOLDING | TRIM@140.59 shares=1100.7 |
| 215,758 | 150.82 | HOLDING | CLOSE_DIFF@150.82 profit=-11260.51 |
| 215,927 | 147.00 | HOLDING | TRIM@147.00 shares=990.3 |
| 216,132 | 158.28 | HOLDING | CLOSE_DIFF@158.28 profit=-11170.83 |
| 217,096 | 167.76 | HOLDING | TRIM@167.76 shares=1012.4 |
| 217,321 | 170.10 | HOLDING | CLOSE_DIFF@170.10 profit=-2369.11 |
| 217,570 | 181.14 | HOLDING | MOVE_SETTLE_SKIP(p=44.68<med=170.41) |
| 218,565 | 172.82 | HOLDING | TRIM@172.82 shares=1001.3 |
| ... | ... | ... | （共186条，显示前150） |
</details>

## HK700

- 数据：**1,408,882** bars (1min)
- 价格：11.36 → 458.20
- Buy-and-hold: **+3933.45%**
- 回测耗时：615.4s

### PH 分层统计

| 级别 | 更新次数 | settle总数 | rank-1 settle |
|------|---------|-----------|---------------|
| L0 | 1,408,882 (每bar) | 463,934 | 2312 |
| L1 | 5,014 | 4,798 | 184 |
| L2 | 298 | 141 | 11 |
| **L2 方向翻转** | — | — | **5** |
| **EVAL** | 触发=16 | 过滤=35 | 过滤率=69% |

### 总体指标

| 指标 | 值 |
|------|-----|
| 交易数 | 6（纯多头） |
| 胜率 | 66.7% |
| 平均收益 | +24.056% |
| 复利累计 | **+206.73%** |
| 最大回撤 | -16.88% |
| 平均持仓 | 79,955 bars |
| 有加仓的交易 | 0/6 |
| 有降成本的交易 | 6/6 |
| 达到本金回收 | 0/6 |
| 有段间比较的交易 | 4/6 |

### 交易明细

| # | 入场 | 出场 | 持仓bars | PnL% | 加仓 | 短差 | 向上段数 | 退出原因 | 状态路径 |
|---|------|------|---------|------|------|------|---------|---------|---------|
| 1 | 50.40@289490 | 80.70@375243 | 85,753 | +58.30 | 0 | 9 | 5 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY→WAIT_SUB_EXIT |
| 2 | 86.80@378953 | 117.45@436812 | 57,859 | +17.81 | 0 | 16 | 3 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY→WAIT_SUB_EXIT |
| 3 | 110.60@441348 | 132.20@510220 | 68,872 | +18.49 | 0 | 31 | 2 | no_new_high | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY→WAIT_SUB_EXIT |
| 4 | 157.05@576835 | 367.80@707716 | 130,881 | +67.00 | 0 | 41 | 4 | no_new_high | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY→WAIT_SUB_EXIT |
| 5 | 401.30@711005 | 312.30@761142 | 50,137 | -14.67 | 0 | 28 | 1 | l2_direction_flip_short | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY |
| 6 | 482.60@917836 | 507.00@1004063 | 86,227 | -2.59 | 0 | 40 | 1 | l2_direction_flip_short | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY |

### FSM 状态分布（按 bar 数）

| 状态 | Bars | 占比 |
|------|------|------|
| WAIT_ENTRY | 928,781 | 65.9% |
| ENTRY | 6 | 0.0% |
| WAIT_SUB_ENTRY | 366 | 0.0% |
| HOLDING | 454,004 | 32.2% |
| EVAL | 0 | 0.0% |
| WAIT_DIP | 25,481 | 1.8% |
| OBSERVE | 0 | 0.0% |
| WAIT_SUB_EXIT | 244 | 0.0% |
| EXIT | 0 | 0.0% |

<details><summary>事件流（418条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 289,428 | 50.50 | ENTRY | WAIT→ENTRY BSP={2113800284} |
| 289,429 | 50.50 | WAIT_SUB_ENTRY | ENTRY→WAIT_SUB@50.50 L2dir=1 |
| 289,490 | 50.40 | HOLDING | SUB_ENTRY_TIMEOUT@50.40 |
| 291,912 | 49.70 | HOLDING | TRIM@49.70 shares=1961.9 |
| 302,478 | 45.90 | HOLDING | CLOSE_DIFF@45.90 profit=7455.16 |
| 307,353 | 49.60 | WAIT_DIP | →EVAL(move_settle) high=51.70 force=6.90 p=6.90/med=6.40 → WAIT_DIP(U1完成 high=51.70) |
| 316,533 | 51.50 | HOLDING | DIP_REBUY@51.50 不跌破前低(down_low=48.40 >= prev_low=44.80) |
| 320,109 | 48.50 | HOLDING | TRIM@48.50 shares=1915.4 |
| 324,400 | 46.20 | WAIT_DIP | →EVAL(move_settle) high=52.70 force=8.60 p=8.60/med=7.10 → WAIT_DIP(创新高无背驰 high=52.70) |
| 326,792 | 46.70 | HOLDING | DIP_REBUY@46.70 不跌破前低(down_low=44.90 >= prev_low=44.10) |
| 331,116 | 50.30 | HOLDING | TRIM@50.30 shares=1915.4 |
| 334,063 | 55.10 | HOLDING | CLOSE_DIFF@55.10 profit=-9193.98 |
| 336,793 | 55.40 | HOLDING | TRIM@55.40 shares=1980.2 |
| 338,777 | 55.50 | HOLDING | CLOSE_DIFF@55.50 profit=-198.02 |
| 340,518 | 51.70 | HOLDING | TRIM@51.70 shares=1818.8 |
| 341,695 | 55.50 | HOLDING | CLOSE_DIFF@55.50 profit=-6911.38 |
| 341,796 | 57.10 | WAIT_DIP | →EVAL(move_settle) high=58.10 force=13.30 p=13.30/med=5.70 → WAIT_DIP(创新高无背驰 high=58.10) |
| 341,917 | 56.40 | HOLDING | DIP_REBUY@56.40 不跌破前低(down_low=56.00 >= prev_low=44.80) |
| 343,058 | 56.70 | HOLDING | TRIM@56.70 shares=1949.3 |
| 343,399 | 56.10 | HOLDING | CLOSE_DIFF@56.10 profit=1169.59 |
| 352,110 | 68.50 | HOLDING | TRIM@68.50 shares=1861.3 |
| 352,332 | 67.70 | HOLDING | CLOSE_DIFF@67.70 profit=1489.04 |
| 352,795 | 68.70 | HOLDING | TRIM@68.70 shares=1974.7 |
| 355,033 | 66.20 | HOLDING | CLOSE_DIFF@66.20 profit=4936.81 |
| 356,878 | 70.40 | WAIT_DIP | →EVAL(move_settle) high=70.10 force=20.40 p=20.40/med=3.80 → WAIT_DIP(创新高无背驰 high=70.10) |
| 356,973 | 71.10 | HOLDING | DIP_REBUY@71.10 不跌破前低(down_low=69.80 >= prev_low=49.70) |
| 364,770 | 76.90 | HOLDING | TRIM@76.90 shares=1959.4 |
| 367,580 | 78.70 | HOLDING | CLOSE_DIFF@78.70 profit=-3526.97 |
| 368,642 | 77.30 | HOLDING | TRIM@77.30 shares=1853.4 |
| 372,638 | 75.70 | HOLDING | CLOSE_DIFF@75.70 profit=2965.42 |
| 375,182 | 80.70 | WAIT_SUB_EXIT | →EVAL(move_settle) high=83.90 force=19.20 p=19.20/med=10.30 → WAIT_SUB_EXIT(盘整背驰 force 19.20<20.40) |
| 375,243 | 80.70 | WAIT_ENTRY | SUB_EXIT_TIMEOUT@80.70 |
| 378,891 | 87.50 | ENTRY | WAIT→ENTRY BSP={1299500946} |
| 378,892 | 87.50 | WAIT_SUB_ENTRY | ENTRY→WAIT_SUB@87.50 L2dir=1 |
| 378,953 | 86.80 | HOLDING | SUB_ENTRY_TIMEOUT@86.80 |
| 379,838 | 87.00 | HOLDING | TRIM@87.00 shares=1142.2 |
| 380,024 | 87.20 | HOLDING | CLOSE_DIFF@87.20 profit=-228.45 |
| 381,251 | 85.80 | HOLDING | TRIM@85.80 shares=1087.4 |
| 383,149 | 92.00 | HOLDING | CLOSE_DIFF@92.00 profit=-6742.16 |
| 383,351 | 92.70 | HOLDING | TRIM@92.70 shares=1090.9 |
| 385,176 | 90.20 | HOLDING | CLOSE_DIFF@90.20 profit=2727.20 |
| 385,518 | 92.10 | HOLDING | TRIM@92.10 shares=1090.9 |
| 386,478 | 97.20 | HOLDING | CLOSE_DIFF@97.20 profit=-5563.48 |
| 387,966 | 94.20 | HOLDING | TRIM@94.20 shares=1080.9 |
| 389,456 | 96.80 | HOLDING | MOVE_SETTLE_SKIP(p=27.30<med=94.00) |
| 389,910 | 93.60 | HOLDING | TRIM@93.60 shares=1131.2 |
| 390,418 | 95.90 | HOLDING | CLOSE_DIFF@95.90 profit=-2601.85 |
| 391,198 | 98.20 | HOLDING | MOVE_SETTLE_SKIP(p=11.80<med=51.05) |
| 393,846 | 106.40 | HOLDING | TRIM@106.40 shares=1077.5 |
| 395,385 | 114.00 | HOLDING | CLOSE_DIFF@114.00 profit=-8189.21 |
| 398,194 | 110.60 | HOLDING | TRIM@110.60 shares=1073.2 |
| 400,708 | 108.40 | HOLDING | MOVE_SETTLE_SKIP(p=26.80<med=57.35) |
| 405,133 | 99.40 | HOLDING | CLOSE_DIFF@99.40 profit=12019.44 |
| 405,802 | 97.40 | HOLDING | TRIM@97.40 shares=1027.9 |
| 407,247 | 98.75 | HOLDING | CLOSE_DIFF@98.75 profit=-1387.70 |
| 408,073 | 96.55 | HOLDING | TRIM@96.55 shares=1027.9 |
| 408,783 | 93.75 | HOLDING | MOVE_SETTLE_SKIP(p=10.20<med=16.80) |
| 409,422 | 91.65 | HOLDING | CLOSE_DIFF@91.65 profit=5036.82 |
| 410,479 | 87.70 | HOLDING | TRIM@87.70 shares=862.7 |
| 411,058 | 93.05 | HOLDING | CLOSE_DIFF@93.05 profit=-4615.66 |
| 413,389 | 103.70 | WAIT_DIP | →EVAL(move_settle) high=103.25 force=17.50 p=17.50/med=13.40 → WAIT_DIP(U1完成 high=103.25) |
| 414,150 | 102.30 | HOLDING | DIP_REBUY@102.30 不跌破前低(down_low=101.85 >= prev_low=85.75) |
| 414,425 | 102.80 | HOLDING | TRIM@102.80 shares=1040.4 |
| 414,933 | 104.05 | HOLDING | CLOSE_DIFF@104.05 profit=-1300.55 |
| 418,539 | 107.20 | WAIT_DIP | →EVAL(move_settle) high=105.55 force=104.10 p=104.10/med=13.40 → WAIT_DIP(创新高无背驰 high=105.55) |
| 423,023 | 114.75 | HOLDING | DIP_REBUY@114.75 不跌破前低(down_low=105.10 >= prev_low=1.45) |
| 424,306 | 113.10 | HOLDING | TRIM@113.10 shares=1150.1 |
| 425,332 | 113.65 | HOLDING | CLOSE_DIFF@113.65 profit=-632.53 |
| 426,336 | 113.20 | HOLDING | TRIM@113.20 shares=1150.1 |
| 427,100 | 113.95 | HOLDING | CLOSE_DIFF@113.95 profit=-862.55 |
| 427,556 | 114.50 | HOLDING | MOVE_SETTLE_SKIP(p=16.75<med=109.50) |
| 429,469 | 118.80 | HOLDING | TRIM@118.80 shares=1108.1 |
| 429,983 | 119.35 | HOLDING | CLOSE_DIFF@119.35 profit=-609.43 |
| 431,636 | 121.50 | HOLDING | TRIM@121.50 shares=1124.2 |
| 432,017 | 123.05 | HOLDING | CLOSE_DIFF@123.05 profit=-1742.55 |
| 435,026 | 120.20 | HOLDING | TRIM@120.20 shares=1125.2 |
| 436,751 | 117.35 | WAIT_SUB_EXIT | →EVAL(move_settle) high=123.50 force=11.70 p=11.70/med=6.05 → WAIT_SUB_EXIT(盘整背驰 force 11.70<104.10) |
| 436,812 | 117.45 | WAIT_ENTRY | SUB_EXIT_TIMEOUT@117.45 |
| 441,286 | 109.40 | ENTRY | WAIT→ENTRY BSP={1444010681} |
| 441,287 | 109.60 | WAIT_SUB_ENTRY | ENTRY→WAIT_SUB@109.60 L2dir=1 |
| 441,348 | 110.60 | HOLDING | SUB_ENTRY_TIMEOUT@110.60 |
| 441,365 | 110.45 | HOLDING | TRIM@110.45 shares=855.3 |
| 443,150 | 106.00 | HOLDING | CLOSE_DIFF@106.00 profit=3805.93 |
| 443,385 | 105.25 | HOLDING | TRIM@105.25 shares=848.6 |
| 443,924 | 109.90 | HOLDING | CLOSE_DIFF@109.90 profit=-3945.98 |
| 444,887 | 108.60 | HOLDING | TRIM@108.60 shares=848.6 |
| 447,813 | 109.15 | HOLDING | CLOSE_DIFF@109.15 profit=-466.73 |
| 448,533 | 108.95 | HOLDING | TRIM@108.95 shares=848.6 |
| 449,933 | 113.20 | HOLDING | CLOSE_DIFF@113.20 profit=-3606.54 |
| 451,870 | 115.80 | HOLDING | TRIM@115.80 shares=848.6 |
| 452,346 | 118.00 | HOLDING | CLOSE_DIFF@118.00 profit=-1866.92 |
| 453,486 | 124.00 | HOLDING | TRIM@124.00 shares=877.1 |
| 453,689 | 119.55 | HOLDING | CLOSE_DIFF@119.55 profit=3903.18 |
| 456,029 | 115.30 | HOLDING | TRIM@115.30 shares=883.8 |
| 457,173 | 109.70 | HOLDING | MOVE_SETTLE_SKIP(p=16.80<med=19.35) |
| 457,634 | 109.15 | HOLDING | CLOSE_DIFF@109.15 profit=5435.29 |
| 458,266 | 110.05 | HOLDING | TRIM@110.05 shares=883.8 |
| 458,402 | 109.95 | HOLDING | CLOSE_DIFF@109.95 profit=88.38 |
| 459,724 | 105.00 | HOLDING | TRIM@105.00 shares=848.6 |
| 461,297 | 102.50 | HOLDING | CLOSE_DIFF@102.50 profit=2121.50 |
| 461,971 | 104.00 | HOLDING | TRIM@104.00 shares=848.6 |
| 462,609 | 104.15 | HOLDING | CLOSE_DIFF@104.15 profit=-127.29 |
| 463,489 | 103.25 | HOLDING | TRIM@103.25 shares=848.6 |
| 464,007 | 105.00 | HOLDING | CLOSE_DIFF@105.00 profit=-1485.05 |
| 465,151 | 116.90 | HOLDING | MOVE_SETTLE_SKIP(p=11.60<med=19.35) |
| 465,732 | 116.40 | HOLDING | TRIM@116.40 shares=870.1 |
| 466,046 | 116.90 | HOLDING | CLOSE_DIFF@116.90 profit=-435.04 |
| 466,429 | 114.85 | HOLDING | TRIM@114.85 shares=870.1 |
| 468,385 | 121.10 | HOLDING | MOVE_SETTLE_SKIP(p=16.75<med=122.35) |
| 470,753 | 124.35 | HOLDING | TRIM@124.35 shares=886.1 |
| 471,012 | 124.05 | HOLDING | CLOSE_DIFF@124.05 profit=265.82 |
| 471,415 | 124.70 | HOLDING | TRIM@124.70 shares=899.5 |
| 471,599 | 124.55 | HOLDING | CLOSE_DIFF@124.55 profit=134.92 |
| 472,091 | 122.40 | HOLDING | TRIM@122.40 shares=899.5 |
| 474,856 | 124.05 | HOLDING | MOVE_SETTLE_SKIP(p=15.10<med=65.97) |
| 475,334 | 124.70 | HOLDING | CLOSE_DIFF@124.70 profit=-2068.74 |
| 477,069 | 123.35 | HOLDING | TRIM@123.35 shares=897.7 |
| 479,346 | 122.15 | HOLDING | CLOSE_DIFF@122.15 profit=1077.23 |
| 479,825 | 125.25 | HOLDING | MOVE_SETTLE_SKIP(p=9.20<med=66.47) |
| 481,043 | 133.40 | HOLDING | TRIM@133.40 shares=900.9 |
| 481,240 | 134.50 | HOLDING | CLOSE_DIFF@134.50 profit=-991.02 |
| 482,025 | 131.35 | HOLDING | TRIM@131.35 shares=851.0 |
| 482,914 | 134.60 | HOLDING | CLOSE_DIFF@134.60 profit=-2765.86 |
| 483,010 | 136.15 | HOLDING | MOVE_SETTLE_SKIP(p=17.15<med=133.85) |
| 485,735 | 145.20 | HOLDING | TRIM@145.20 shares=820.5 |
| 486,139 | 146.45 | HOLDING | CLOSE_DIFF@146.45 profit=-1025.58 |
| 486,777 | 145.75 | HOLDING | TRIM@145.75 shares=864.7 |
| 487,544 | 150.35 | HOLDING | MOVE_SETTLE_SKIP(p=28.30<med=82.45) |
| 488,643 | 149.60 | HOLDING | TRIM@149.60 shares=890.6 |
| 488,865 | 150.70 | HOLDING | CLOSE_DIFF@150.70 profit=-979.66 |
| 489,342 | 149.50 | HOLDING | TRIM@149.50 shares=890.6 |
| 491,627 | 147.65 | HOLDING | CLOSE_DIFF@147.65 profit=1647.61 |
| 492,712 | 149.35 | WAIT_DIP | →EVAL(move_settle) high=152.65 force=12.10 p=12.10/med=11.50 → WAIT_DIP(U1完成 high=152.65) |
| 492,853 | 149.90 | HOLDING | DIP_REBUY@149.90 不跌破前低(down_low=148.80 >= prev_low=140.55) |
| 492,911 | 150.15 | HOLDING | TRIM@150.15 shares=880.3 |
| 495,992 | 144.90 | HOLDING | CLOSE_DIFF@144.90 profit=4621.51 |
| 497,215 | 144.55 | HOLDING | TRIM@144.55 shares=880.3 |
| 497,373 | 144.15 | HOLDING | CLOSE_DIFF@144.15 profit=352.12 |
| 497,822 | 143.15 | HOLDING | MOVE_SETTLE_SKIP(p=11.15<med=11.20) |
| 498,076 | 142.15 | HOLDING | TRIM@142.15 shares=838.4 |
| 498,442 | 141.15 | HOLDING | CLOSE_DIFF@141.15 profit=838.44 |
| 499,585 | 142.25 | HOLDING | TRIM@142.25 shares=852.0 |
| 500,267 | 142.30 | HOLDING | CLOSE_DIFF@142.30 profit=-42.60 |
| 500,870 | 142.90 | HOLDING | TRIM@142.90 shares=852.0 |
| 501,158 | 145.10 | HOLDING | CLOSE_DIFF@145.10 profit=-1874.39 |
| 503,682 | 142.90 | HOLDING | TRIM@142.90 shares=849.3 |
| 504,627 | 125.90 | HOLDING | CLOSE_DIFF@125.90 profit=14438.84 |
| 504,755 | 127.95 | HOLDING | TRIM@127.95 shares=741.2 |
| 505,038 | 134.95 | HOLDING | CLOSE_DIFF@134.95 profit=-5188.31 |
| 506,431 | 137.70 | HOLDING | TRIM@137.70 shares=821.1 |
| ... | ... | ... | （共418条，显示前150） |
</details>

## BTC

- 数据：**1,795,600** bars (1min)
- 价格：16543.67 → 73674.39
- Buy-and-hold: **+345.33%**
- 回测耗时：2095.1s

### PH 分层统计

| 级别 | 更新次数 | settle总数 | rank-1 settle |
|------|---------|-----------|---------------|
| L0 | 1,795,600 (每bar) | 852,939 | 1710 |
| L1 | 14,339 | 13,866 | 230 |
| L2 | 753 | 416 | 44 |
| **L2 方向翻转** | — | — | **27** |
| **EVAL** | 触发=24 | 过滤=40 | 过滤率=62% |

### 总体指标

| 指标 | 值 |
|------|-----|
| 交易数 | 14（纯多头） |
| 胜率 | 35.7% |
| 平均收益 | +2.380% |
| 复利累计 | **+30.42%** |
| 最大回撤 | -16.29% |
| 平均持仓 | 18,583 bars |
| 有加仓的交易 | 0/14 |
| 有降成本的交易 | 12/14 |
| 达到本金回收 | 0/14 |
| 有段间比较的交易 | 10/14 |

### 交易明细

| # | 入场 | 出场 | 持仓bars | PnL% | 加仓 | 短差 | 向上段数 | 退出原因 | 状态路径 |
|---|------|------|---------|------|------|------|---------|---------|---------|
| 1 | 26714.47@108666 | 27800.07@119032 | 10,366 | -2.41 | 0 | 9 | 2 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY→WAIT_SUB_EXIT |
| 2 | 29806.59@143996 | 30391.84@149816 | 5,820 | -0.00 | 0 | 5 | 2 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY→WAIT_SUB_EXIT |
| 3 | 32888.97@426114 | 38278.00@481341 | 55,227 | +18.64 | 0 | 51 | 4 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY→WAIT_SUB_EXIT |
| 4 | 38731.97@481935 | 42964.50@500892 | 18,957 | +5.22 | 0 | 18 | 2 | no_new_high | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY→WAIT_SUB_EXIT |
| 5 | 45402.38@527199 | 43849.93@532858 | 5,659 | -8.31 | 0 | 9 | 2 | no_new_high | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY→WAIT_SUB_EXIT |
| 6 | 47102.18@536898 | 43660.14@542632 | 5,734 | -8.70 | 0 | 5 | 2 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY→WAIT_SUB_EXIT |
| 7 | 49930.88@587335 | 65478.21@676799 | 89,464 | +27.91 | 0 | 91 | 3 | l2_direction_flip_short | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY |
| 8 | 76522.00@976248 | 94390.56@1001052 | 24,804 | +8.80 | 0 | 24 | 2 | no_new_high | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY→WAIT_SUB_EXIT |
| 9 | 102657.01@1013894 | 99028.09@1016142 | 2,248 | -0.18 | 0 | 1 | 0 | l2_direction_flip_short | ENTRY→HOLDING→WAIT_SUB_ENTRY |
| 10 | 106596.07@1031905 | 98111.12@1056893 | 24,988 | -10.23 | 0 | 30 | 2 | no_new_high | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY→WAIT_SUB_EXIT |
| 11 | 107115.01@1255216 | 106871.14@1255278 | 62 | -0.23 | 0 | 0 | 0 | bsp_invalidate | ENTRY→HOLDING→WAIT_SUB_ENTRY |
| 12 | 110529.60@1255773 | 110494.49@1263611 | 7,838 | -0.50 | 0 | 5 | 2 | no_new_high | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY→WAIT_SUB_EXIT |
| 13 | 113240.01@1384956 | 113166.00@1384983 | 27 | -0.07 | 0 | 0 | 0 | bsp_invalidate | ENTRY→HOLDING→WAIT_SUB_ENTRY |
| 14 | 125181.78@1453985 | 114998.01@1462950 | 8,965 | +3.39 | 0 | 8 | 1 | l2_direction_flip_short | ENTRY→EVAL→HOLDING→WAIT_DIP→WAIT_SUB_ENTRY |

### FSM 状态分布（按 bar 数）

| 状态 | Bars | 占比 |
|------|------|------|
| WAIT_ENTRY | 1,534,573 | 85.5% |
| ENTRY | 14 | 0.0% |
| WAIT_SUB_ENTRY | 854 | 0.0% |
| HOLDING | 245,589 | 13.7% |
| EVAL | 0 | 0.0% |
| WAIT_DIP | 14,021 | 0.8% |
| OBSERVE | 0 | 0.0% |
| WAIT_SUB_EXIT | 549 | 0.0% |
| EXIT | 0 | 0.0% |

<details><summary>事件流（649条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 108,604 | 26380.42 | ENTRY | WAIT→ENTRY BSP={156615031} |
| 108,605 | 26333.56 | WAIT_SUB_ENTRY | ENTRY→WAIT_SUB@26333.56 L2dir=1 |
| 108,666 | 26714.47 | HOLDING | SUB_ENTRY_TIMEOUT@26714.47 |
| 108,919 | 26488.17 | HOLDING | TRIM@26488.17 shares=3.5 |
| 109,001 | 26382.62 | HOLDING | CLOSE_DIFF@26382.62 profit=370.53 |
| 109,229 | 26752.67 | HOLDING | TRIM@26752.67 shares=3.7 |
| 109,610 | 27290.32 | HOLDING | CLOSE_DIFF@27290.32 profit=-1987.23 |
| 110,092 | 27390.27 | HOLDING | TRIM@27390.27 shares=3.7 |
| 110,347 | 27455.79 | HOLDING | CLOSE_DIFF@27455.79 profit=-242.11 |
| 110,850 | 26912.01 | HOLDING | TRIM@26912.01 shares=3.7 |
| 111,714 | 27171.53 | HOLDING | CLOSE_DIFF@27171.53 profit=-948.43 |
| 112,838 | 28302.11 | HOLDING | TRIM@28302.11 shares=3.6 |
| 112,902 | 28087.38 | HOLDING | CLOSE_DIFF@28087.38 profit=766.85 |
| 113,330 | 27612.06 | HOLDING | TRIM@27612.06 shares=3.7 |
| 113,480 | 27825.57 | HOLDING | CLOSE_DIFF@27825.57 profit=-787.70 |
| 113,709 | 27883.62 | WAIT_DIP | →EVAL(move_settle) high=28472.00 force=4575.05 p=4575.05/med=1010.65 → WAIT_DIP(U1完成 high=28472.00) |
| 115,363 | 28078.84 | HOLDING | DIP_REBUY@28078.84 不跌破前低(down_low=27336.46 >= prev_low=23896.95) |
| 115,693 | 28161.65 | HOLDING | TRIM@28161.65 shares=3.7 |
| 116,041 | 28295.37 | HOLDING | CLOSE_DIFF@28295.37 profit=-499.15 |
| 117,285 | 27642.75 | HOLDING | MOVE_SETTLE_SKIP(p=2266.25<med=7089.00) |
| 117,467 | 27346.11 | HOLDING | TRIM@27346.11 shares=3.4 |
| 117,735 | 28342.17 | HOLDING | CLOSE_DIFF@28342.17 profit=-3393.58 |
| 117,951 | 28151.89 | HOLDING | TRIM@28151.89 shares=3.7 |
| 118,877 | 28086.55 | HOLDING | CLOSE_DIFF@28086.55 profit=242.25 |
| 118,942 | 27957.87 | HOLDING | TRIM@27957.87 shares=3.7 |
| 118,971 | 27969.00 | WAIT_SUB_EXIT | →EVAL(move_settle) high=28868.05 force=2266.25 p=2266.25/med=900.12 → WAIT_SUB_EXIT(盘整背驰 force 2266.25<4575.05) |
| 119,032 | 27800.07 | WAIT_ENTRY | SUB_EXIT_TIMEOUT@27800.07 |
| 143,934 | 29665.98 | ENTRY | WAIT→ENTRY BSP={1374506628} |
| 143,935 | 29689.51 | WAIT_SUB_ENTRY | ENTRY→WAIT_SUB@29689.51 L2dir=1 |
| 143,996 | 29806.59 | HOLDING | SUB_ENTRY_TIMEOUT@29806.59 |
| 144,512 | 30041.59 | HOLDING | TRIM@30041.59 shares=3.2 |
| 144,622 | 30091.26 | HOLDING | CLOSE_DIFF@30091.26 profit=-158.75 |
| 145,173 | 30158.61 | HOLDING | TRIM@30158.61 shares=3.2 |
| 145,347 | 30233.12 | HOLDING | CLOSE_DIFF@30233.12 profit=-238.14 |
| 145,571 | 29974.69 | HOLDING | TRIM@29974.69 shares=3.2 |
| 146,017 | 29996.27 | HOLDING | CLOSE_DIFF@29996.27 profit=-68.97 |
| 146,640 | 29930.19 | HOLDING | TRIM@29930.19 shares=3.2 |
| 147,116 | 30058.23 | WAIT_DIP | →EVAL(move_settle) high=30550.00 force=2455.50 p=2455.50/med=669.87 → WAIT_DIP(U1完成 high=30550.00) |
| 147,692 | 30494.70 | HOLDING | DIP_REBUY@30494.70 不跌破前低(down_low=30004.44 >= prev_low=28094.50) |
| 148,110 | 30312.49 | HOLDING | TRIM@30312.49 shares=3.3 |
| 148,523 | 30705.61 | HOLDING | CLOSE_DIFF@30705.61 profit=-1303.46 |
| 148,810 | 30730.01 | HOLDING | TRIM@30730.01 shares=3.3 |
| 148,882 | 30789.02 | HOLDING | CLOSE_DIFF@30789.02 profit=-194.83 |
| 149,143 | 30550.36 | HOLDING | TRIM@30550.36 shares=3.3 |
| 149,755 | 30367.30 | WAIT_SUB_EXIT | →EVAL(move_settle) high=31000.00 force=1362.60 p=1362.60/med=464.06 → WAIT_SUB_EXIT(盘整背驰 force 1362.60<2455.50) |
| 149,816 | 30391.84 | WAIT_ENTRY | SUB_EXIT_TIMEOUT@30391.84 |
| 426,052 | 31838.55 | ENTRY | WAIT→ENTRY BSP={1327508208} |
| 426,053 | 31801.95 | WAIT_SUB_ENTRY | ENTRY→WAIT_SUB@31801.95 L2dir=1 |
| 426,114 | 32888.97 | HOLDING | SUB_ENTRY_TIMEOUT@32888.97 |
| 426,906 | 34588.09 | HOLDING | TRIM@34588.09 shares=3.0 |
| 427,568 | 33911.19 | HOLDING | CLOSE_DIFF@33911.19 profit=1999.17 |
| 428,002 | 34109.80 | HOLDING | TRIM@34109.80 shares=3.0 |
| 428,270 | 34260.00 | HOLDING | CLOSE_DIFF@34260.00 profit=-443.60 |
| 429,181 | 34696.97 | HOLDING | TRIM@34696.97 shares=3.0 |
| 429,535 | 34561.76 | HOLDING | CLOSE_DIFF@34561.76 profit=404.70 |
| 429,705 | 34312.73 | HOLDING | TRIM@34312.73 shares=3.0 |
| 430,494 | 34110.50 | HOLDING | CLOSE_DIFF@34110.50 profit=605.30 |
| 430,652 | 34086.72 | HOLDING | TRIM@34086.72 shares=3.0 |
| 430,945 | 34046.13 | HOLDING | CLOSE_DIFF@34046.13 profit=121.49 |
| 431,010 | 34079.99 | HOLDING | TRIM@34079.99 shares=3.0 |
| 431,278 | 34112.70 | HOLDING | CLOSE_DIFF@34112.70 profit=-97.91 |
| 431,419 | 34019.33 | HOLDING | TRIM@34019.33 shares=3.0 |
| 432,208 | 34071.99 | HOLDING | CLOSE_DIFF@34071.99 profit=-157.62 |
| 432,824 | 34094.65 | HOLDING | TRIM@34094.65 shares=3.0 |
| 432,923 | 34119.49 | HOLDING | CLOSE_DIFF@34119.49 profit=-74.35 |
| 433,549 | 34011.30 | HOLDING | TRIM@34011.30 shares=3.0 |
| 434,087 | 34341.43 | HOLDING | CLOSE_DIFF@34341.43 profit=-988.13 |
| 434,271 | 34530.00 | HOLDING | TRIM@34530.00 shares=3.0 |
| 434,491 | 34668.51 | HOLDING | CLOSE_DIFF@34668.51 profit=-414.58 |
| 434,900 | 34469.39 | HOLDING | TRIM@34469.39 shares=3.0 |
| 435,416 | 34485.72 | HOLDING | CLOSE_DIFF@34485.72 profit=-48.88 |
| 436,056 | 34431.22 | HOLDING | TRIM@34431.22 shares=3.0 |
| 436,984 | 34443.00 | HOLDING | CLOSE_DIFF@34443.00 profit=-35.26 |
| 438,119 | 34442.83 | HOLDING | TRIM@34442.83 shares=3.0 |
| 438,510 | 34493.30 | HOLDING | CLOSE_DIFF@34493.30 profit=-151.06 |
| 438,733 | 34393.53 | HOLDING | TRIM@34393.53 shares=3.0 |
| 438,836 | 34542.85 | HOLDING | CLOSE_DIFF@34542.85 profit=-452.08 |
| 439,646 | 35276.82 | HOLDING | TRIM@35276.82 shares=3.0 |
| 440,164 | 34604.69 | HOLDING | MOVE_SETTLE_SKIP(p=6270.71<med=19117.00) |
| 441,013 | 34462.42 | HOLDING | CLOSE_DIFF@34462.42 profit=2432.69 |
| 441,378 | 34396.58 | HOLDING | TRIM@34396.58 shares=3.0 |
| 441,529 | 34732.01 | HOLDING | CLOSE_DIFF@34732.01 profit=-1001.96 |
| 442,346 | 34688.01 | HOLDING | TRIM@34688.01 shares=3.0 |
| 442,483 | 34738.94 | HOLDING | CLOSE_DIFF@34738.94 profit=-152.13 |
| 442,876 | 34768.48 | HOLDING | TRIM@34768.48 shares=3.0 |
| 442,899 | 34733.14 | HOLDING | CLOSE_DIFF@34733.14 profit=105.56 |
| 442,971 | 34727.48 | HOLDING | TRIM@34727.48 shares=3.0 |
| 443,445 | 35038.93 | HOLDING | CLOSE_DIFF@35038.93 profit=-930.33 |
| 443,777 | 35196.67 | HOLDING | TRIM@35196.67 shares=3.0 |
| 443,953 | 35138.15 | HOLDING | CLOSE_DIFF@35138.15 profit=174.80 |
| 444,387 | 34844.14 | HOLDING | TRIM@34844.14 shares=3.0 |
| 444,615 | 35042.82 | WAIT_DIP | →EVAL(move_settle) high=35305.75 force=742.21 p=742.21/med=646.49 → WAIT_DIP(U1完成 high=35305.75) |
| 445,785 | 35151.10 | HOLDING | DIP_REBUY_DIVERGE@35151.10 跌破+盘整背驰 |
| 445,932 | 34977.58 | HOLDING | TRIM@34977.58 shares=3.0 |
| 447,216 | 34822.89 | HOLDING | CLOSE_DIFF@34822.89 profit=464.53 |
| 447,576 | 35864.02 | HOLDING | TRIM@35864.02 shares=3.0 |
| 447,638 | 35500.00 | HOLDING | MOVE_SETTLE_SKIP(p=651.52<med=19388.99) |
| 448,035 | 35311.11 | HOLDING | TRIM@35311.11 shares=3.0 |
| 448,364 | 35307.55 | HOLDING | CLOSE_DIFF@35307.55 profit=10.67 |
| 448,852 | 35445.27 | HOLDING | TRIM@35445.27 shares=3.0 |
| 449,006 | 35550.01 | HOLDING | CLOSE_DIFF@35550.01 profit=-314.00 |
| 449,241 | 35797.12 | HOLDING | MOVE_SETTLE_SKIP(p=1364.94<med=10045.90) |
| 450,225 | 36382.93 | HOLDING | TRIM@36382.93 shares=2.9 |
| 450,605 | 36525.79 | HOLDING | CLOSE_DIFF@36525.79 profit=-410.57 |
| 450,976 | 36669.38 | HOLDING | MOVE_SETTLE_SKIP(p=2872.24<med=10833.21) |
| 451,542 | 37084.63 | HOLDING | TRIM@37084.63 shares=3.0 |
| 451,585 | 36948.07 | HOLDING | CLOSE_DIFF@36948.07 profit=413.34 |
| 452,379 | 37058.39 | HOLDING | TRIM@37058.39 shares=3.0 |
| 452,891 | 37105.39 | HOLDING | CLOSE_DIFF@37105.39 profit=-139.41 |
| 453,603 | 36953.49 | HOLDING | TRIM@36953.49 shares=3.0 |
| 453,987 | 37150.40 | HOLDING | CLOSE_DIFF@37150.40 profit=-584.07 |
| 455,055 | 37203.34 | HOLDING | TRIM@37203.34 shares=3.0 |
| 455,319 | 36916.94 | HOLDING | CLOSE_DIFF@36916.94 profit=858.23 |
| 456,093 | 36726.38 | HOLDING | TRIM@36726.38 shares=3.0 |
| 456,196 | 36785.48 | WAIT_DIP | →EVAL(move_settle) high=37526.00 force=1386.43 p=1386.43/med=794.90 → WAIT_DIP(创新高无背驰 high=37526.00) |
| 456,340 | 36484.00 | HOLDING | DIP_REBUY@36484.00 不跌破前低(down_low=36374.15 >= prev_low=36139.57) |
| 456,496 | 36396.00 | HOLDING | TRIM@36396.00 shares=3.0 |
| 456,991 | 36511.10 | HOLDING | CLOSE_DIFF@36511.10 profit=-341.41 |
| 458,108 | 35490.29 | HOLDING | TRIM@35490.29 shares=2.8 |
| 458,324 | 35651.15 | HOLDING | CLOSE_DIFF@35651.15 profit=-451.00 |
| 459,601 | 37331.32 | HOLDING | TRIM@37331.32 shares=3.0 |
| 460,420 | 36253.50 | HOLDING | MOVE_SETTLE_SKIP(p=2231.26<med=21392.44) |
| 461,320 | 36322.00 | HOLDING | CLOSE_DIFF@36322.00 profit=3016.44 |
| 461,749 | 36547.99 | HOLDING | TRIM@36547.99 shares=3.0 |
| 461,823 | 36387.71 | HOLDING | CLOSE_DIFF@36387.71 profit=479.01 |
| 462,049 | 36449.98 | HOLDING | TRIM@36449.98 shares=3.0 |
| 462,154 | 36521.10 | HOLDING | CLOSE_DIFF@36521.10 profit=-212.55 |
| 462,563 | 36322.00 | HOLDING | TRIM@36322.00 shares=3.0 |
| 462,880 | 36409.84 | HOLDING | CLOSE_DIFF@36409.84 profit=-262.52 |
| 463,552 | 36519.76 | HOLDING | TRIM@36519.76 shares=3.0 |
| 463,572 | 36543.82 | HOLDING | CLOSE_DIFF@36543.82 profit=-71.91 |
| 463,747 | 36434.00 | HOLDING | TRIM@36434.00 shares=3.0 |
| 464,009 | 36635.10 | HOLDING | CLOSE_DIFF@36635.10 profit=-601.01 |
| 464,216 | 36588.41 | HOLDING | MOVE_SETTLE_SKIP(p=984.39<med=1291.35) |
| 464,453 | 36429.63 | HOLDING | TRIM@36429.63 shares=3.0 |
| 464,642 | 36567.75 | HOLDING | CLOSE_DIFF@36567.75 profit=-412.78 |
| 464,940 | 36977.99 | HOLDING | MOVE_SETTLE_SKIP(p=331.98<med=851.45) |
| 465,612 | 37183.53 | HOLDING | TRIM@37183.53 shares=3.0 |
| 465,662 | 37186.33 | HOLDING | CLOSE_DIFF@37186.33 profit=-8.37 |
| 465,912 | 37154.00 | HOLDING | TRIM@37154.00 shares=3.0 |
| 465,983 | 37330.00 | HOLDING | CLOSE_DIFF@37330.00 profit=-525.99 |
| 467,231 | 37215.01 | HOLDING | TRIM@37215.01 shares=3.0 |
| 467,689 | 36894.83 | HOLDING | MOVE_SETTLE_SKIP(p=1365.98<med=2062.85) |
| 468,015 | 36094.04 | HOLDING | CLOSE_DIFF@36094.04 profit=3385.17 |
| 468,073 | 36145.99 | HOLDING | TRIM@36145.99 shares=3.0 |
| 468,495 | 36562.21 | HOLDING | CLOSE_DIFF@36562.21 profit=-1256.93 |
| 468,887 | 36532.20 | HOLDING | TRIM@36532.20 shares=3.0 |
| 469,135 | 37274.79 | HOLDING | CLOSE_DIFF@37274.79 profit=-2242.52 |
| 470,096 | 37406.92 | HOLDING | TRIM@37406.92 shares=3.0 |
| 470,169 | 37317.01 | HOLDING | CLOSE_DIFF@37317.01 profit=271.52 |
| ... | ... | ... | （共649条，显示前150） |
</details>

## 控制变量对比

| 版本 | QQQ | OKLO | HK700 |
|------|-----|------|-------|
| D2 区间套版（有加仓） | +60.7% | +192.4% | +325.3% |
| D2 区间套版（**无加仓，本版**） | **+23.7%** | **+114.1%** | **+206.7%** | **+30.4%** |

唯一差异：是否在持仓期间加仓。

## 汇总

| 标的 | Bars | BH% | 复利% | 胜率 | 交易数 | L2翻转 | 加仓 | 降成本 | PW | 段比较 | 耗时 |
|------|------|-----|------|------|--------|--------|------|--------|-----|--------|------|
| QQQ | 728,030 | +174.6 | +23.69 | 47% | 17 | 14 | 0/17 | 15/17 | 0/17 | 13/17 | 415s |
| OKLO | 333,613 | +251.3 | +114.08 | 83% | 6 | 7 | 0/6 | 4/6 | 0/6 | 3/6 | 78s |
| HK700 | 1,408,882 | +3933.5 | +206.73 | 67% | 6 | 5 | 0/6 | 6/6 | 0/6 | 4/6 | 615s |
| BTC | 1,795,600 | +345.3 | +30.42 | 36% | 14 | 27 | 0/14 | 12/14 | 0/14 | 10/14 | 2095s |

## 结果包六要素

**结论**：控制变量——去掉加仓后的表现对比。

**定义依据**：
- 38课操作程式原文是全进全出，不涉及分批加仓
- 30课分区卷钱是不同级别各做各的，不是同级别加仓

**边界条件**：
- PENDING_EXPIRY = 390 bars
- 降成本保留（原文有）
- 加仓去掉（原文无明确依据）

**下游推论**：
- 若无加仓 > 有加仓 → 加仓逻辑是负alpha，应删除
- 若无加仓 < 有加仓 → 加仓产生了正alpha，应保留
- 若无加仓 ≈ 有加仓 → 加仓不是关键变量

**谱系引用**：
- 525号：组件局部完成性
- 526号：a0递归存在论区分
- 521号：PH纯拓扑无动量

**影响声明**：新建独立回测脚本，不修改引擎代码或旧版回测。

**认识论等级**：L2（真实数据，4标的 1min QQQ/OKLO/HK700/BTC；可产生否定性结果）。

**审计修复声明**（2026-06-06）：降成本利润计算已移除 `if profit > 0` 截断（审计 A1）——亏损 trim 现照实扣减 cost_basis / cumulative_recovered，不再是只赚不赔的提款机。本表数字为去截断后的真实基线。