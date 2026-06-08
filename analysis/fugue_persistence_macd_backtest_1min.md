# persistence过滤版 — 控制变量：MACD面积盘整背驰

## 架构

与 persistence 过滤版完全相同，唯一差异：

- **旧**（价格振幅）：`|C.high - C.low| < |A.high - A.low|` → 盘整背驰
- **新**（MACD面积）：`MACD_hist_area(C) < MACD_hist_area(A)` → 盘整背驰

MACD通过增量OnlineMacdState计算（O(1)/bar），面积查询O(段长度)。

控制变量：只有盘整背驰度量改变。其余所有逻辑不变。

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
- 回测耗时：432.1s

### PH 分层统计

| 级别 | 更新次数 | settle总数 | rank-1 settle |
|------|---------|-----------|---------------|
| L0 | 728,030 (每bar) | 350,072 | 3962 |
| L1 | 6,119 | 5,804 | 286 |
| L2 | 389 | 190 | 25 |
| **L2 方向翻转** | — | — | **14** |
| **EVAL** | 触发=19 | 过滤=55 | 过滤率=74% |

### 总体指标

| 指标 | 值 |
|------|-----|
| 交易数 | 30（纯多头） |
| 胜率 | 70.0% |
| 平均收益 | +1.595% |
| 复利累计 | **+58.88%** |
| 最大回撤 | -2.12% |
| 平均持仓 | 6,706 bars |
| 有加仓的交易 | 20/30 |
| 有降成本的交易 | 24/30 |
| 达到本金回收 | 0/30 |
| 有段间比较的交易 | 5/30 |

### 交易明细

| # | 入场 | 出场 | 持仓bars | PnL% | 加仓 | 短差 | 向上段数 | 退出原因 | 状态路径 |
|---|------|------|---------|------|------|------|---------|---------|---------|
| 1 | 313.83@45927 | 305.43@49505 | 3,578 | -1.16 | 3 | 3 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 2 | 314.81@50896 | 314.09@56300 | 5,404 | +0.05 | 1 | 4 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 3 | 322.52@67708 | 319.88@67882 | 174 | -0.82 | 0 | 0 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 4 | 319.41@67966 | 318.33@72362 | 4,396 | +0.08 | 2 | 3 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 5 | 323.61@72704 | 352.97@87747 | 15,043 | +10.18 | 4 | 11 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 6 | 357.54@92089 | 354.25@92162 | 73 | -0.92 | 0 | 0 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 7 | 357.61@93056 | 365.80@97876 | 4,820 | +3.15 | 1 | 5 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 8 | 377.02@109966 | 376.68@115656 | 5,690 | +1.79 | 1 | 5 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 9 | 358.17@131229 | 358.07@131250 | 21 | -0.03 | 0 | 0 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 10 | 389.89@186027 | 388.55@192317 | 6,290 | +1.34 | 1 | 6 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 11 | 395.37@196836 | 396.84@208841 | 12,005 | +2.62 | 1 | 14 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 12 | 413.10@216486 | 419.92@223983 | 7,497 | +3.37 | 1 | 8 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 13 | 435.14@229309 | 429.72@231925 | 2,616 | +0.17 | 1 | 1 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 14 | 439.04@236273 | 439.85@243498 | 7,225 | +1.27 | 2 | 4 | 2 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 15 | 448.45@252490 | 442.59@259485 | 6,995 | +0.25 | 1 | 9 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 16 | 456.16@288173 | 451.10@293623 | 5,450 | -0.97 | 1 | 5 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 17 | 463.86@296109 | 481.63@308780 | 12,671 | +3.34 | 5 | 13 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 18 | 488.67@310993 | 489.18@318090 | 7,097 | +0.25 | 2 | 7 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 19 | 519.92@400612 | 524.05@410720 | 10,108 | +2.70 | 1 | 9 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 20 | 540.87@440494 | 516.63@445662 | 5,168 | -2.12 | 1 | 6 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 21 | 544.24@520848 | 558.20@532487 | 11,639 | +3.99 | 0 | 12 | 2 | macd_consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 22 | 561.20@532890 | 565.00@543673 | 10,783 | +0.92 | 1 | 9 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 23 | 580.40@549099 | 565.70@554420 | 5,321 | -1.13 | 1 | 6 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 24 | 583.18@566174 | 601.49@588515 | 22,341 | +4.91 | 2 | 18 | 2 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 25 | 616.76@594461 | 633.27@599842 | 5,381 | +2.84 | 0 | 2 | 2 | no_new_high | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 26 | 617.82@601418 | 617.32@601511 | 93 | -0.08 | 0 | 0 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 27 | 599.51@603661 | 603.75@603769 | 108 | +0.71 | 0 | 0 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 28 | 650.00@699682 | 703.02@717431 | 17,749 | +10.44 | 0 | 19 | 3 | macd_consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 29 | 702.00@718194 | 701.88@718219 | 25 | -0.02 | 0 | 0 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 30 | 733.50@722602 | 738.28@728029 | 5,427 | +0.72 | 0 | 6 | 0 | eod_close | ENTRY→HOLDING |

### FSM 状态分布（按 bar 数）

| 状态 | Bars | 占比 |
|------|------|------|
| WAIT_ENTRY | 526,812 | 72.4% |
| ENTRY | 30 | 0.0% |
| HOLDING | 194,869 | 26.8% |
| EVAL | 0 | 0.0% |
| WAIT_DIP | 6,319 | 0.9% |
| OBSERVE | 0 | 0.0% |
| EXIT | 0 | 0.0% |

<details><summary>事件流（570条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 45,926 | 313.67 | ENTRY | WAIT→ENTRY BSP={1699618109} |
| 45,927 | 313.83 | HOLDING | ENTRY_LONG@313.83 L2dir=1 |
| 46,025 | 309.74 | HOLDING | TRIM@309.74 shares=314.4 |
| 46,384 | 309.40 | HOLDING | MOVE_SETTLE_SKIP(p=13.26<med=54.06) |
| 46,714 | 313.12 | HOLDING | ADD_POS@313.12 +47.8 L1ratio=0.150 |
| 47,157 | 310.23 | HOLDING | TRIM@310.23 shares=364.8 |
| 47,393 | 308.80 | HOLDING | MOVE_SETTLE_SKIP(p=8.57<med=31.08) |
| 47,712 | 308.69 | HOLDING | CLOSE_DIFF@308.69 profit=561.72 |
| 48,006 | 311.37 | HOLDING | ADD_POS@311.37 +55.0 L1ratio=0.150 |
| 48,740 | 309.02 | HOLDING | TRIM@309.02 shares=419.5 |
| 49,360 | 306.38 | HOLDING | ADD_POS@306.38 +63.2 L1ratio=0.150 |
| 49,505 | 305.43 | WAIT_ENTRY | BSP_INVALIDATE@305.43 |
| 50,895 | 315.00 | ENTRY | WAIT→ENTRY BSP={987157400} |
| 50,896 | 314.81 | HOLDING | ENTRY_LONG@314.81 L2dir=1 |
| 51,694 | 315.66 | HOLDING | TRIM@315.66 shares=315.9 |
| 52,243 | 321.03 | HOLDING | CLOSE_DIFF@321.03 profit=-1696.65 |
| 52,737 | 319.50 | HOLDING | TRIM@319.50 shares=292.4 |
| 52,767 | 319.20 | HOLDING | CLOSE_DIFF@319.20 profit=87.71 |
| 52,924 | 318.36 | HOLDING | TRIM@318.36 shares=292.4 |
| 53,510 | 321.52 | HOLDING | MOVE_SETTLE_SKIP(p=14.07<med=61.83) |
| 53,767 | 319.65 | HOLDING | TRIM@319.65 shares=312.5 |
| 54,238 | 318.54 | HOLDING | CLOSE_DIFF@318.54 profit=346.83 |
| 54,689 | 314.50 | WAIT_DIP | →EVAL(move_settle) high=322.17 force=4.65 p=4.65/med=4.34 → WAIT_DIP(U1完成 high=322.17) |
| 55,783 | 318.10 | HOLDING | DIP_REBUY_DIVERGE@318.10 跌破+盘整背驰 |
| 56,108 | 317.32 | HOLDING | ADD_POS@317.32 +47.3 L1ratio=0.149 |
| 56,300 | 314.09 | WAIT_ENTRY | BSP_INVALIDATE@314.09 |
| 67,707 | 322.18 | ENTRY | WAIT→ENTRY BSP={1666850169, 103427621} |
| 67,708 | 322.52 | HOLDING | ENTRY_LONG@322.52 L2dir=1 |
| 67,882 | 319.88 | WAIT_ENTRY | BSP_INVALIDATE@319.88 |
| 67,965 | 319.43 | ENTRY | WAIT→ENTRY BSP={414981298} |
| 67,966 | 319.41 | HOLDING | ENTRY_LONG@319.41 L2dir=1 |
| 68,196 | 320.04 | HOLDING | TRIM@320.04 shares=302.8 |
| 68,316 | 319.31 | HOLDING | CLOSE_DIFF@319.31 profit=221.08 |
| 68,532 | 321.28 | HOLDING | ADD_POS@321.28 +54.2 L1ratio=0.173 |
| 68,893 | 322.36 | HOLDING | TRIM@322.36 shares=363.0 |
| 69,320 | 322.04 | HOLDING | CLOSE_DIFF@322.04 profit=116.15 |
| 69,405 | 322.46 | HOLDING | TRIM@322.46 shares=363.1 |
| 69,827 | 322.03 | HOLDING | CLOSE_DIFF@322.03 profit=156.12 |
| 71,205 | 316.80 | HOLDING | TRIM@316.80 shares=358.8 |
| 71,368 | 318.03 | WAIT_DIP | →EVAL(move_settle) high=323.62 force=10.29 p=10.29/med=1.90 → WAIT_DIP(U1完成 high=323.62) |
| 72,003 | 316.08 | HOLDING | DIP_REBUY@316.08 不跌破前低(down_low=315.28 >= prev_low=313.33) |
| 72,362 | 318.33 | WAIT_ENTRY | BSP_INVALIDATE@318.33 |
| 72,703 | 323.59 | ENTRY | WAIT→ENTRY BSP={285522146} |
| 72,704 | 323.61 | HOLDING | ENTRY_LONG@323.61 L2dir=1 |
| 73,460 | 323.47 | HOLDING | TRIM@323.47 shares=308.3 |
| 74,028 | 322.43 | HOLDING | CLOSE_DIFF@322.43 profit=320.67 |
| 74,074 | 321.91 | HOLDING | TRIM@321.91 shares=307.0 |
| 74,503 | 321.04 | WAIT_DIP | →EVAL(move_settle) high=324.04 force=6.56 p=6.56/med=2.31 → WAIT_DIP(U1完成 high=324.04) |
| 74,691 | 324.30 | HOLDING | DIP_REBUY@324.30 不跌破前低(down_low=320.73 >= prev_low=317.48) |
| 74,864 | 324.55 | HOLDING | TRIM@324.55 shares=307.0 |
| 74,926 | 324.00 | HOLDING | CLOSE_DIFF@324.00 profit=168.84 |
| 75,607 | 325.21 | HOLDING | TRIM@325.21 shares=300.2 |
| 75,993 | 326.14 | HOLDING | CLOSE_DIFF@326.14 profit=-279.19 |
| 76,441 | 325.48 | HOLDING | TRIM@325.48 shares=307.8 |
| 76,897 | 326.05 | HOLDING | MOVE_SETTLE_SKIP(p=7.11<med=66.54) |
| 77,031 | 325.95 | HOLDING | ADD_POS@325.95 +18.2 L1ratio=0.059 |
| 77,328 | 325.96 | HOLDING | ADD_POS@325.96 +19.3 L1ratio=0.059 |
| 77,981 | 327.22 | HOLDING | TRIM@327.22 shares=344.5 |
| 78,031 | 327.75 | HOLDING | CLOSE_DIFF@327.75 profit=-182.60 |
| 78,440 | 327.44 | HOLDING | TRIM@327.44 shares=339.6 |
| 78,534 | 327.64 | HOLDING | CLOSE_DIFF@327.64 profit=-67.92 |
| 78,826 | 328.60 | HOLDING | TRIM@328.60 shares=343.2 |
| 79,420 | 332.09 | HOLDING | CLOSE_DIFF@332.09 profit=-1197.67 |
| 80,484 | 336.69 | HOLDING | TRIM@336.69 shares=339.9 |
| 81,188 | 336.32 | HOLDING | CLOSE_DIFF@336.32 profit=125.76 |
| 81,552 | 338.07 | HOLDING | MOVE_SETTLE_SKIP(p=15.94<med=77.77) |
| 82,391 | 333.84 | HOLDING | TRIM@333.84 shares=335.9 |
| 82,871 | 331.49 | HOLDING | MOVE_SETTLE_SKIP(p=3.35<med=78.44) |
| 82,996 | 330.63 | HOLDING | CLOSE_DIFF@330.63 profit=1078.36 |
| 83,632 | 337.79 | HOLDING | ADD_POS@337.79 +38.6 L1ratio=0.111 |
| 85,792 | 351.01 | HOLDING | TRIM@351.01 shares=364.5 |
| 86,208 | 349.29 | HOLDING | MOVE_SETTLE_SKIP(p=23.78<med=93.84) |
| 86,834 | 347.42 | HOLDING | CLOSE_DIFF@347.42 profit=1308.71 |
| 87,198 | 348.05 | HOLDING | TRIM@348.05 shares=354.9 |
| 87,236 | 347.99 | HOLDING | CLOSE_DIFF@347.99 profit=21.30 |
| 87,663 | 352.53 | HOLDING | ADD_POS@352.53 +30.9 L1ratio=0.080 |
| 87,747 | 352.97 | WAIT_ENTRY | BSP_INVALIDATE@352.97 |
| 92,088 | 357.55 | ENTRY | WAIT→ENTRY BSP={665482103} |
| 92,089 | 357.54 | HOLDING | ENTRY_LONG@357.54 L2dir=1 |
| 92,162 | 354.25 | WAIT_ENTRY | BSP_INVALIDATE@354.25 |
| 93,055 | 357.62 | ENTRY | WAIT→ENTRY BSP={909163236} |
| 93,056 | 357.61 | HOLDING | ENTRY_LONG@357.61 L2dir=1 |
| 93,739 | 361.71 | HOLDING | TRIM@361.71 shares=261.0 |
| 93,922 | 363.24 | HOLDING | CLOSE_DIFF@363.24 profit=-399.36 |
| 94,011 | 362.72 | HOLDING | TRIM@362.72 shares=277.2 |
| 94,978 | 366.34 | HOLDING | CLOSE_DIFF@366.34 profit=-1003.54 |
| 95,575 | 368.10 | HOLDING | TRIM@368.10 shares=277.9 |
| 95,754 | 370.23 | HOLDING | CLOSE_DIFF@370.23 profit=-591.87 |
| 95,904 | 369.83 | HOLDING | TRIM@369.83 shares=268.3 |
| 96,071 | 370.83 | HOLDING | CLOSE_DIFF@370.83 profit=-268.35 |
| 96,272 | 371.02 | HOLDING | TRIM@371.02 shares=267.2 |
| 97,237 | 364.91 | HOLDING | MOVE_SETTLE_SKIP(p=23.92<med=111.46) |
| 97,689 | 366.89 | HOLDING | ADD_POS@366.89 +19.8 L1ratio=0.071 |
| 97,876 | 365.80 | WAIT_ENTRY | BSP_INVALIDATE@365.80 |
| 109,965 | 377.16 | ENTRY | WAIT→ENTRY BSP={4111992} |
| 109,966 | 377.02 | HOLDING | ENTRY_LONG@377.02 L2dir=1 |
| 110,916 | 380.20 | HOLDING | TRIM@380.20 shares=259.0 |
| 111,605 | 380.26 | HOLDING | CLOSE_DIFF@380.26 profit=-15.54 |
| 111,754 | 381.36 | WAIT_DIP | →EVAL(move_settle) high=382.86 force=18.24 p=18.24/med=4.68 → WAIT_DIP(U1完成 high=382.86) |
| 111,843 | 382.39 | HOLDING | DIP_REBUY@382.39 不跌破前低(down_low=381.23 >= prev_low=364.62) |
| 112,060 | 382.55 | HOLDING | TRIM@382.55 shares=264.3 |
| 112,096 | 382.10 | HOLDING | CLOSE_DIFF@382.10 profit=118.94 |
| 112,795 | 385.24 | HOLDING | TRIM@385.24 shares=264.7 |
| 112,977 | 386.31 | HOLDING | CLOSE_DIFF@386.31 profit=-283.20 |
| 113,388 | 385.11 | HOLDING | TRIM@385.11 shares=255.3 |
| 114,270 | 378.90 | HOLDING | MOVE_SETTLE_SKIP(p=9.80<med=127.31) |
| 114,885 | 377.83 | HOLDING | CLOSE_DIFF@377.83 profit=1858.28 |
| 114,940 | 377.28 | HOLDING | TRIM@377.28 shares=249.1 |
| 115,159 | 376.99 | HOLDING | CLOSE_DIFF@376.99 profit=72.25 |
| 115,476 | 376.90 | HOLDING | ADD_POS@376.90 +24.8 L1ratio=0.094 |
| 115,656 | 376.68 | WAIT_ENTRY | BSP_INVALIDATE@376.68 |
| 131,228 | 357.95 | ENTRY | WAIT→ENTRY BSP={1073924458} |
| 131,229 | 358.17 | HOLDING | ENTRY_LONG@358.17 L2dir=1 |
| 131,250 | 358.07 | WAIT_ENTRY | BSP_INVALIDATE@358.07 |
| 186,026 | 389.75 | ENTRY | WAIT→ENTRY BSP={77258701} |
| 186,027 | 389.89 | HOLDING | ENTRY_LONG@389.89 L2dir=1 |
| 186,738 | 387.46 | HOLDING | TRIM@387.46 shares=250.5 |
| 187,452 | 390.52 | HOLDING | CLOSE_DIFF@390.52 profit=-766.63 |
| 187,547 | 390.47 | HOLDING | MOVE_SETTLE_SKIP(p=7.12<med=67.53) |
| 188,058 | 389.84 | HOLDING | TRIM@389.84 shares=255.9 |
| 188,296 | 389.12 | HOLDING | CLOSE_DIFF@389.12 profit=184.26 |
| 188,427 | 389.31 | HOLDING | TRIM@389.31 shares=255.9 |
| 188,830 | 390.08 | HOLDING | CLOSE_DIFF@390.08 profit=-197.06 |
| 189,110 | 389.90 | HOLDING | TRIM@389.90 shares=255.9 |
| 189,589 | 389.56 | HOLDING | CLOSE_DIFF@389.56 profit=87.01 |
| 190,433 | 391.27 | HOLDING | TRIM@391.27 shares=252.7 |
| 190,749 | 390.22 | HOLDING | MOVE_SETTLE_SKIP(p=7.64<med=132.97) |
| 191,243 | 386.20 | HOLDING | CLOSE_DIFF@386.20 profit=1282.30 |
| 191,523 | 388.44 | HOLDING | TRIM@388.44 shares=246.3 |
| 191,773 | 387.58 | HOLDING | CLOSE_DIFF@387.58 profit=211.80 |
| 192,053 | 389.14 | HOLDING | ADD_POS@389.14 +14.4 L1ratio=0.056 |
| 192,317 | 388.55 | WAIT_ENTRY | BSP_INVALIDATE@388.55 |
| 196,835 | 395.32 | ENTRY | WAIT→ENTRY BSP={773048308} |
| 196,836 | 395.37 | HOLDING | ENTRY_LONG@395.37 L2dir=1 |
| 197,341 | 396.43 | HOLDING | TRIM@396.43 shares=244.4 |
| 198,028 | 400.10 | HOLDING | CLOSE_DIFF@400.10 profit=-896.82 |
| 198,319 | 399.24 | HOLDING | TRIM@399.24 shares=248.9 |
| 198,439 | 403.06 | HOLDING | CLOSE_DIFF@403.06 profit=-950.71 |
| 198,796 | 405.29 | HOLDING | TRIM@405.29 shares=250.4 |
| 198,906 | 404.56 | HOLDING | CLOSE_DIFF@404.56 profit=182.81 |
| 198,981 | 403.65 | HOLDING | TRIM@403.65 shares=249.6 |
| 199,487 | 404.68 | HOLDING | CLOSE_DIFF@404.68 profit=-257.05 |
| 199,941 | 404.27 | HOLDING | TRIM@404.27 shares=251.6 |
| 200,214 | 404.86 | HOLDING | CLOSE_DIFF@404.86 profit=-148.42 |
| 200,360 | 404.70 | HOLDING | TRIM@404.70 shares=251.6 |
| 200,569 | 406.38 | HOLDING | MOVE_SETTLE_SKIP(p=19.33<med=146.00) |
| 200,761 | 407.86 | HOLDING | CLOSE_DIFF@407.86 profit=-795.14 |
| 201,771 | 408.25 | HOLDING | TRIM@408.25 shares=251.8 |
| 202,150 | 409.79 | HOLDING | CLOSE_DIFF@409.79 profit=-387.81 |
| 202,741 | 405.90 | HOLDING | MOVE_SETTLE_SKIP(p=10.12<med=78.64) |
| ... | ... | ... | （共570条，显示前150） |
</details>

## OKLO

- 数据：**333,613** bars (1min)
- 价格：18.07 → 63.48
- Buy-and-hold: **+251.30%**
- 回测耗时：79.3s

### PH 分层统计

| 级别 | 更新次数 | settle总数 | rank-1 settle |
|------|---------|-----------|---------------|
| L0 | 333,613 (每bar) | 154,247 | 730 |
| L1 | 2,806 | 2,657 | 102 |
| L2 | 185 | 101 | 12 |
| **L2 方向翻转** | — | — | **7** |
| **EVAL** | 触发=6 | 过滤=10 | 过滤率=62% |

### 总体指标

| 指标 | 值 |
|------|-----|
| 交易数 | 9（纯多头） |
| 胜率 | 55.6% |
| 平均收益 | +14.454% |
| 复利累计 | **+194.37%** |
| 最大回撤 | -10.42% |
| 平均持仓 | 4,839 bars |
| 有加仓的交易 | 7/9 |
| 有降成本的交易 | 7/9 |
| 达到本金回收 | 0/9 |
| 有段间比较的交易 | 1/9 |

### 交易明细

| # | 入场 | 出场 | 持仓bars | PnL% | 加仓 | 短差 | 向上段数 | 退出原因 | 状态路径 |
|---|------|------|---------|------|------|------|---------|---------|---------|
| 1 | 13.00@41092 | 19.11@46795 | 5,703 | +38.85 | 1 | 3 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 2 | 24.50@47162 | 20.33@50552 | 3,390 | -8.61 | 1 | 3 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 3 | 28.23@53131 | 19.57@57372 | 4,241 | -1.98 | 2 | 6 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 4 | 32.84@82272 | 54.69@94200 | 11,928 | +47.70 | 5 | 9 | 2 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 5 | 66.89@151335 | 65.64@151540 | 205 | -1.87 | 0 | 0 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 6 | 76.66@173693 | 76.68@178964 | 5,271 | +0.87 | 4 | 4 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 7 | 65.04@187215 | 64.39@187431 | 216 | -1.00 | 0 | 0 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 8 | 89.23@199495 | 109.95@207459 | 7,964 | +35.04 | 1 | 7 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 9 | 146.69@215838 | 168.62@220471 | 4,633 | +21.08 | 2 | 4 | 0 | bsp_invalidate | ENTRY→HOLDING |

### FSM 状态分布（按 bar 数）

| 状态 | Bars | 占比 |
|------|------|------|
| WAIT_ENTRY | 290,053 | 86.9% |
| ENTRY | 9 | 0.0% |
| HOLDING | 41,598 | 12.5% |
| EVAL | 0 | 0.0% |
| WAIT_DIP | 1,953 | 0.6% |
| OBSERVE | 0 | 0.0% |
| EXIT | 0 | 0.0% |

<details><summary>事件流（131条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 41,091 | 13.08 | ENTRY | WAIT→ENTRY BSP={1146069223} |
| 41,092 | 13.00 | HOLDING | ENTRY_LONG@13.00 L2dir=1 |
| 41,862 | 16.22 | HOLDING | TRIM@16.22 shares=6929.3 |
| 42,565 | 15.84 | HOLDING | CLOSE_DIFF@15.84 profit=2633.15 |
| 42,786 | 19.43 | HOLDING | MOVE_SETTLE_SKIP(p=8.58<med=15.15) |
| 43,016 | 18.46 | HOLDING | TRIM@18.46 shares=6861.9 |
| 43,450 | 19.91 | HOLDING | CLOSE_DIFF@19.91 profit=-9949.72 |
| 43,481 | 20.04 | HOLDING | MOVE_SETTLE_SKIP(p=6.51<med=16.18) |
| 44,516 | 20.63 | HOLDING | TRIM@20.63 shares=7087.8 |
| 44,992 | 20.44 | HOLDING | CLOSE_DIFF@20.44 profit=1346.68 |
| 45,323 | 18.34 | WAIT_DIP | →EVAL(move_settle) high=23.05 force=5.85 p=5.85/med=3.16 → WAIT_DIP(U1完成 high=23.05) |
| 46,744 | 19.14 | HOLDING | DIP_REBUY@19.14 不跌破前低(down_low=17.62 >= prev_low=17.20) |
| 46,795 | 19.11 | WAIT_ENTRY | BSP_INVALIDATE@19.11 |
| 47,161 | 24.60 | ENTRY | WAIT→ENTRY BSP={308273581} |
| 47,162 | 24.50 | HOLDING | ENTRY_LONG@24.50 L2dir=1 |
| 47,960 | 23.43 | HOLDING | TRIM@23.43 shares=3935.3 |
| 48,481 | 26.75 | HOLDING | CLOSE_DIFF@26.75 profit=-13065.35 |
| 48,715 | 23.60 | HOLDING | TRIM@23.60 shares=3627.5 |
| 49,561 | 22.99 | HOLDING | CLOSE_DIFF@22.99 profit=2212.78 |
| 49,609 | 22.92 | HOLDING | TRIM@22.92 shares=3627.5 |
| 50,300 | 21.43 | WAIT_DIP | →EVAL(move_settle) high=28.12 force=10.00 p=10.00/med=5.26 → WAIT_DIP(U1完成 high=28.12) |
| 50,307 | 21.52 | HOLDING | DIP_REBUY@21.52 不跌破前低(down_low=21.36 >= prev_low=18.12) |
| 50,377 | 21.56 | HOLDING | TRIM@21.56 shares=3497.5 |
| 50,507 | 20.36 | HOLDING | ADD_POS@20.36 +963.2 L1ratio=0.236 |
| 50,552 | 20.33 | WAIT_ENTRY | BSP_INVALIDATE@20.33 |
| 53,130 | 28.02 | ENTRY | WAIT→ENTRY BSP={465952852} |
| 53,131 | 28.23 | HOLDING | ENTRY_LONG@28.23 L2dir=1 |
| 53,314 | 26.42 | HOLDING | ADD_POS@26.42 +1242.8 L1ratio=0.351 |
| 53,605 | 24.94 | HOLDING | TRIM@24.94 shares=4755.0 |
| 53,907 | 24.36 | HOLDING | CLOSE_DIFF@24.36 profit=2757.92 |
| 53,993 | 24.40 | HOLDING | TRIM@24.40 shares=4755.0 |
| 54,119 | 23.72 | HOLDING | CLOSE_DIFF@23.72 profit=3233.42 |
| 54,228 | 23.83 | HOLDING | TRIM@23.83 shares=4755.0 |
| 54,589 | 23.20 | WAIT_DIP | →EVAL(move_settle) high=28.68 force=8.68 p=8.68/med=5.93 → WAIT_DIP(U1完成 high=28.68) |
| 54,897 | 21.96 | HOLDING | DIP_REBUY@21.96 不跌破前低(down_low=21.83 >= prev_low=20.00) |
| 55,122 | 22.74 | HOLDING | TRIM@22.74 shares=4755.0 |
| 55,491 | 23.99 | HOLDING | CLOSE_DIFF@23.99 profit=-5943.79 |
| 56,100 | 23.94 | HOLDING | TRIM@23.94 shares=4755.0 |
| 56,142 | 23.71 | HOLDING | CLOSE_DIFF@23.71 profit=1093.66 |
| 56,427 | 24.00 | HOLDING | TRIM@24.00 shares=4755.0 |
| 56,700 | 21.99 | HOLDING | CLOSE_DIFF@21.99 profit=9557.61 |
| 56,759 | 22.40 | HOLDING | TRIM@22.40 shares=4755.0 |
| 57,138 | 18.72 | HOLDING | ADD_POS@18.72 +1678.8 L1ratio=0.351 |
| 57,372 | 19.57 | WAIT_ENTRY | BSP_INVALIDATE@19.57 |
| 82,271 | 31.98 | ENTRY | WAIT→ENTRY BSP={1567619886} |
| 82,272 | 32.84 | HOLDING | ENTRY_LONG@32.84 L2dir=1 |
| 83,207 | 32.37 | HOLDING | TRIM@32.37 shares=2688.2 |
| 83,593 | 38.78 | HOLDING | CLOSE_DIFF@38.78 profit=-17231.51 |
| 84,429 | 42.84 | HOLDING | TRIM@42.84 shares=2635.1 |
| 84,463 | 42.52 | HOLDING | CLOSE_DIFF@42.52 profit=843.24 |
| 84,755 | 35.90 | HOLDING | TRIM@35.90 shares=2385.5 |
| 85,820 | 31.29 | HOLDING | MOVE_SETTLE_SKIP(p=19.47<med=37.58) |
| 86,173 | 33.53 | HOLDING | CLOSE_DIFF@33.53 profit=5653.61 |
| 86,342 | 34.20 | HOLDING | ADD_POS@34.20 +1050.9 L1ratio=0.345 |
| 87,232 | 36.11 | HOLDING | ADD_POS@36.11 +1413.7 L1ratio=0.345 |
| 87,675 | 41.59 | HOLDING | ADD_POS@41.59 +1901.6 L1ratio=0.345 |
| 88,040 | 41.95 | HOLDING | TRIM@41.95 shares=7340.2 |
| 88,940 | 43.40 | HOLDING | CLOSE_DIFF@43.40 profit=-10680.03 |
| 88,990 | 44.60 | WAIT_DIP | →EVAL(move_settle) high=45.21 force=12.21 p=12.21/med=7.38 → WAIT_DIP(U1完成 high=45.21) |
| 89,005 | 45.42 | HOLDING | DIP_REBUY@45.42 不跌破前低(down_low=44.35 >= prev_low=33.00) |
| 89,466 | 46.55 | HOLDING | TRIM@46.55 shares=7314.3 |
| 90,008 | 45.95 | HOLDING | CLOSE_DIFF@45.95 profit=4388.60 |
| 91,119 | 48.95 | HOLDING | TRIM@48.95 shares=6630.3 |
| 91,475 | 48.63 | HOLDING | CLOSE_DIFF@48.63 profit=2088.54 |
| 92,285 | 54.89 | HOLDING | TRIM@54.89 shares=7190.2 |
| 92,312 | 55.20 | HOLDING | CLOSE_DIFF@55.20 profit=-2228.96 |
| 92,466 | 54.30 | HOLDING | TRIM@54.30 shares=7190.2 |
| 93,016 | 52.38 | HOLDING | CLOSE_DIFF@52.38 profit=13769.24 |
| 93,454 | 50.88 | HOLDING | TRIM@50.88 shares=6722.7 |
| 93,772 | 52.73 | HOLDING | CLOSE_DIFF@52.73 profit=-12403.32 |
| 93,829 | 54.11 | WAIT_DIP | →EVAL(move_settle) high=59.11 force=21.28 p=21.28/med=4.49 → WAIT_DIP(创新高无MACD背驰 A=34.9578,C=97.1345) |
| 93,903 | 54.91 | HOLDING | DIP_REBUY@54.91 不跌破前低(down_low=53.89 >= prev_low=37.83) |
| 94,086 | 54.37 | HOLDING | ADD_POS@54.37 +1742.7 L1ratio=0.197 |
| 94,200 | 54.69 | WAIT_ENTRY | BSP_INVALIDATE@54.69 |
| 151,334 | 66.80 | ENTRY | WAIT→ENTRY BSP={528094879} |
| 151,335 | 66.89 | HOLDING | ENTRY_LONG@66.89 L2dir=1 |
| 151,540 | 65.64 | WAIT_ENTRY | BSP_INVALIDATE@65.64 |
| 173,692 | 76.58 | ENTRY | WAIT→ENTRY BSP={1926062708} |
| 173,693 | 76.66 | HOLDING | ENTRY_LONG@76.66 L2dir=1 |
| 173,991 | 72.22 | HOLDING | TRIM@72.22 shares=1204.8 |
| 174,553 | 75.00 | WAIT_DIP | →EVAL(move_settle) high=77.16 force=15.42 p=15.42/med=5.21 → WAIT_DIP(U1完成 high=77.16) |
| 174,681 | 77.17 | HOLDING | DIP_REBUY@77.17 不跌破前低(down_low=74.81 >= prev_low=61.74) |
| 174,693 | 76.95 | HOLDING | ADD_POS@76.95 +95.8 L1ratio=0.073 |
| 175,060 | 74.46 | HOLDING | TRIM@74.46 shares=1359.8 |
| 175,622 | 75.20 | HOLDING | CLOSE_DIFF@75.20 profit=-1006.27 |
| 176,091 | 71.38 | HOLDING | MOVE_SETTLE_SKIP(p=7.32<med=7.93) |
| 176,122 | 71.57 | HOLDING | MOVE_SETTLE_SKIP(p=7.32<med=7.93) |
| 176,234 | 71.40 | HOLDING | TRIM@71.40 shares=1309.8 |
| 176,398 | 75.80 | HOLDING | CLOSE_DIFF@75.80 profit=-5756.52 |
| 176,627 | 75.22 | HOLDING | ADD_POS@75.22 +152.0 L1ratio=0.109 |
| 176,907 | 77.59 | HOLDING | ADD_POS@77.59 +168.5 L1ratio=0.109 |
| 177,024 | 77.30 | HOLDING | TRIM@77.30 shares=1715.5 |
| 178,300 | 71.49 | HOLDING | MOVE_SETTLE_SKIP(p=9.23<med=74.93) |
| 178,793 | 76.59 | HOLDING | ADD_POS@76.59 +234.0 L1ratio=0.136 |
| 178,964 | 76.68 | WAIT_ENTRY | BSP_INVALIDATE@76.68 |
| 187,214 | 64.98 | ENTRY | WAIT→ENTRY BSP={1974301457} |
| 187,215 | 65.04 | HOLDING | ENTRY_LONG@65.04 L2dir=1 |
| 187,431 | 64.39 | WAIT_ENTRY | BSP_INVALIDATE@64.39 |
| 199,494 | 88.82 | ENTRY | WAIT→ENTRY BSP={545782922} |
| 199,495 | 89.23 | HOLDING | ENTRY_LONG@89.23 L2dir=1 |
| 200,406 | 91.20 | HOLDING | TRIM@91.20 shares=1058.9 |
| 200,579 | 94.58 | HOLDING | CLOSE_DIFF@94.58 profit=-3578.98 |
| 201,126 | 94.68 | HOLDING | TRIM@94.68 shares=1120.0 |
| 201,832 | 100.44 | HOLDING | CLOSE_DIFF@100.44 profit=-6450.96 |
| 203,456 | 131.41 | HOLDING | TRIM@131.41 shares=1084.3 |
| 203,886 | 138.20 | HOLDING | MOVE_SETTLE_SKIP(p=65.27<med=137.34) |
| 204,334 | 137.28 | HOLDING | TRIM@137.28 shares=1074.9 |
| 205,165 | 140.50 | HOLDING | CLOSE_DIFF@140.50 profit=-3461.25 |
| 205,261 | 136.24 | HOLDING | TRIM@136.24 shares=1109.4 |
| 205,442 | 138.40 | HOLDING | CLOSE_DIFF@138.40 profit=-2396.28 |
| 205,518 | 136.88 | HOLDING | TRIM@136.88 shares=1109.4 |
| 206,090 | 110.98 | HOLDING | MOVE_SETTLE_SKIP(p=21.25<med=74.78) |
| 207,032 | 118.36 | HOLDING | CLOSE_DIFF@118.36 profit=20545.91 |
| 207,189 | 109.70 | HOLDING | TRIM@109.70 shares=943.8 |
| 207,376 | 109.15 | HOLDING | CLOSE_DIFF@109.15 profit=519.09 |
| 207,413 | 110.77 | HOLDING | ADD_POS@110.77 +288.2 L1ratio=0.257 |
| 207,459 | 109.95 | WAIT_ENTRY | BSP_INVALIDATE@109.95 |
| 215,837 | 146.72 | ENTRY | WAIT→ENTRY BSP={244261304} |
| 215,838 | 146.69 | HOLDING | ENTRY_LONG@146.69 L2dir=1 |
| 215,927 | 147.00 | HOLDING | TRIM@147.00 shares=610.0 |
| 216,132 | 158.28 | HOLDING | CLOSE_DIFF@158.28 profit=-6881.15 |
| 216,282 | 157.66 | HOLDING | ADD_POS@157.66 +90.8 L1ratio=0.133 |
| 217,096 | 167.76 | HOLDING | TRIM@167.76 shares=706.8 |
| 217,321 | 170.10 | HOLDING | CLOSE_DIFF@170.10 profit=-1653.81 |
| 217,570 | 181.14 | HOLDING | MOVE_SETTLE_SKIP(p=44.68<med=170.41) |
| 218,565 | 172.82 | HOLDING | TRIM@172.82 shares=699.0 |
| 219,307 | 163.65 | HOLDING | MOVE_SETTLE_SKIP(p=33.57<med=188.33) |
| 219,699 | 154.48 | HOLDING | CLOSE_DIFF@154.48 profit=12818.78 |
| 220,103 | 161.23 | HOLDING | TRIM@161.23 shares=681.4 |
| 220,302 | 163.65 | HOLDING | ADD_POS@163.65 +187.9 L1ratio=0.243 |
| 220,471 | 168.62 | WAIT_ENTRY | BSP_INVALIDATE@168.62 |
</details>

## HK700

- 数据：**1,408,882** bars (1min)
- 价格：11.36 → 458.20
- Buy-and-hold: **+3933.45%**
- 回测耗时：540.6s

### PH 分层统计

| 级别 | 更新次数 | settle总数 | rank-1 settle |
|------|---------|-----------|---------------|
| L0 | 1,408,882 (每bar) | 463,934 | 2312 |
| L1 | 5,014 | 4,798 | 184 |
| L2 | 298 | 141 | 11 |
| **L2 方向翻转** | — | — | **5** |
| **EVAL** | 触发=11 | 过滤=24 | 过滤率=69% |

### 总体指标

| 指标 | 值 |
|------|-----|
| 交易数 | 15（纯多头） |
| 胜率 | 73.3% |
| 平均收益 | +9.188% |
| 复利累计 | **+252.26%** |
| 最大回撤 | -8.78% |
| 平均持仓 | 19,420 bars |
| 有加仓的交易 | 15/15 |
| 有降成本的交易 | 15/15 |
| 达到本金回收 | 0/15 |
| 有段间比较的交易 | 2/15 |

### 交易明细

| # | 入场 | 出场 | 持仓bars | PnL% | 加仓 | 短差 | 向上段数 | 退出原因 | 状态路径 |
|---|------|------|---------|------|------|------|---------|---------|---------|
| 1 | 50.50@289429 | 57.10@341796 | 52,367 | +19.50 | 2 | 4 | 3 | macd_consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 2 | 64.90@348936 | 82.80@377587 | 28,651 | +26.16 | 2 | 4 | 2 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 3 | 87.50@378892 | 90.50@409439 | 30,547 | +17.70 | 2 | 10 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 4 | 119.10@428315 | 111.25@438789 | 10,474 | -6.27 | 1 | 2 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 5 | 109.60@441287 | 105.25@443385 | 2,098 | -0.18 | 1 | 1 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 6 | 119.55@453755 | 109.95@457850 | 4,095 | -2.50 | 1 | 1 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 7 | 124.65@468634 | 140.55@502832 | 34,198 | +13.88 | 6 | 15 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 8 | 158.80@576774 | 167.75@624082 | 47,308 | +8.08 | 1 | 13 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 9 | 222.70@652086 | 263.40@670541 | 18,455 | +25.41 | 1 | 8 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 10 | 276.90@672228 | 299.90@682404 | 10,176 | +7.68 | 1 | 3 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 11 | 312.90@685236 | 351.40@704079 | 18,843 | +11.17 | 1 | 6 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 12 | 403.40@710944 | 384.60@719629 | 8,685 | +7.93 | 1 | 5 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 13 | 487.60@917775 | 490.00@922761 | 4,986 | +0.29 | 1 | 5 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 14 | 552.80@944408 | 540.80@954533 | 10,125 | -1.58 | 1 | 2 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 15 | 583.20@962508 | 641.60@972802 | 10,294 | +10.55 | 2 | 6 | 0 | bsp_invalidate | ENTRY→HOLDING |

### FSM 状态分布（按 bar 数）

| 状态 | Bars | 占比 |
|------|------|------|
| WAIT_ENTRY | 1,117,565 | 79.3% |
| ENTRY | 15 | 0.0% |
| HOLDING | 268,535 | 19.1% |
| EVAL | 0 | 0.0% |
| WAIT_DIP | 22,767 | 1.6% |
| OBSERVE | 0 | 0.0% |
| EXIT | 0 | 0.0% |

<details><summary>事件流（269条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 289,428 | 50.50 | ENTRY | WAIT→ENTRY BSP={2113800284} |
| 289,429 | 50.50 | HOLDING | ENTRY_LONG@50.50 L2dir=1 |
| 291,912 | 49.70 | HOLDING | TRIM@49.70 shares=1958.0 |
| 302,478 | 45.90 | HOLDING | CLOSE_DIFF@45.90 profit=7440.39 |
| 307,353 | 49.60 | WAIT_DIP | →EVAL(move_settle) high=51.70 force=6.90 p=6.90/med=6.40 → WAIT_DIP(U1完成 high=51.70) |
| 316,533 | 51.50 | HOLDING | DIP_REBUY@51.50 不跌破前低(down_low=48.40 >= prev_low=44.80) |
| 320,109 | 48.50 | HOLDING | TRIM@48.50 shares=1911.6 |
| 324,400 | 46.20 | WAIT_DIP | →EVAL(move_settle) high=52.70 force=8.60 p=8.60/med=7.10 → WAIT_DIP(创新高无MACD背驰 A=28.1440,C=121.2452) |
| 326,792 | 46.70 | HOLDING | DIP_REBUY@46.70 不跌破前低(down_low=44.90 >= prev_low=44.10) |
| 329,386 | 50.00 | HOLDING | ADD_POS@50.00 +443.1 L1ratio=0.188 |
| 331,116 | 50.30 | HOLDING | TRIM@50.30 shares=2699.4 |
| 334,063 | 55.10 | HOLDING | CLOSE_DIFF@55.10 profit=-12956.97 |
| 336,793 | 55.40 | HOLDING | TRIM@55.40 shares=2790.7 |
| 338,777 | 55.50 | HOLDING | CLOSE_DIFF@55.50 profit=-279.07 |
| 340,518 | 51.70 | HOLDING | TRIM@51.70 shares=2563.2 |
| 341,695 | 55.50 | HOLDING | CLOSE_DIFF@55.50 profit=-9740.12 |
| 341,796 | 57.10 | WAIT_ENTRY | →EVAL(move_settle) high=58.10 force=13.30 p=13.30/med=5.70 → EXIT(MACD盘整背驰 area 78.4853<121.2452) |
| 348,935 | 64.90 | ENTRY | WAIT→ENTRY BSP={1648568564} |
| 348,936 | 64.90 | HOLDING | ENTRY_LONG@64.90 L2dir=1 |
| 352,110 | 68.50 | HOLDING | TRIM@68.50 shares=1445.4 |
| 352,332 | 67.70 | HOLDING | CLOSE_DIFF@67.70 profit=1156.36 |
| 352,795 | 68.70 | HOLDING | TRIM@68.70 shares=1533.5 |
| 355,033 | 66.20 | HOLDING | CLOSE_DIFF@66.20 profit=3833.82 |
| 356,878 | 70.40 | WAIT_DIP | →EVAL(move_settle) high=70.10 force=20.40 p=20.40/med=3.80 → WAIT_DIP(U1完成 high=70.10) |
| 356,973 | 71.10 | HOLDING | DIP_REBUY@71.10 不跌破前低(down_low=69.80 >= prev_low=49.70) |
| 364,770 | 76.90 | HOLDING | TRIM@76.90 shares=1521.7 |
| 367,580 | 78.70 | HOLDING | CLOSE_DIFF@78.70 profit=-2738.97 |
| 368,642 | 77.30 | HOLDING | TRIM@77.30 shares=1439.3 |
| 372,638 | 75.70 | HOLDING | CLOSE_DIFF@75.70 profit=2302.89 |
| 375,182 | 80.70 | WAIT_DIP | →EVAL(move_settle) high=83.90 force=19.20 p=19.20/med=10.30 → WAIT_DIP(创新高无MACD背驰 A=104.4476,C=120.0063) |
| 376,128 | 81.90 | HOLDING | DIP_REBUY@81.90 不跌破前低(down_low=80.20 >= prev_low=64.70) |
| 377,223 | 82.40 | HOLDING | ADD_POS@82.40 +286.4 L1ratio=0.160 |
| 377,587 | 82.80 | WAIT_ENTRY | BSP_INVALIDATE@82.80 |
| 378,891 | 87.50 | ENTRY | WAIT→ENTRY BSP={1299500946} |
| 378,892 | 87.50 | HOLDING | ENTRY_LONG@87.50 L2dir=1 |
| 379,838 | 87.00 | HOLDING | TRIM@87.00 shares=1133.1 |
| 380,024 | 87.20 | HOLDING | CLOSE_DIFF@87.20 profit=-226.62 |
| 381,251 | 85.80 | HOLDING | TRIM@85.80 shares=1078.7 |
| 383,149 | 92.00 | HOLDING | CLOSE_DIFF@92.00 profit=-6688.22 |
| 383,351 | 92.70 | HOLDING | TRIM@92.70 shares=1082.2 |
| 385,176 | 90.20 | HOLDING | CLOSE_DIFF@90.20 profit=2705.38 |
| 385,518 | 92.10 | HOLDING | TRIM@92.10 shares=1082.2 |
| 386,478 | 97.20 | HOLDING | CLOSE_DIFF@97.20 profit=-5518.97 |
| 387,966 | 94.20 | HOLDING | TRIM@94.20 shares=1072.3 |
| 389,456 | 96.80 | HOLDING | MOVE_SETTLE_SKIP(p=27.30<med=94.00) |
| 389,910 | 93.60 | HOLDING | TRIM@93.60 shares=1122.2 |
| 390,418 | 95.90 | HOLDING | CLOSE_DIFF@95.90 profit=-2581.03 |
| 391,198 | 98.20 | HOLDING | MOVE_SETTLE_SKIP(p=11.80<med=51.05) |
| 393,846 | 106.40 | HOLDING | TRIM@106.40 shares=1161.0 |
| 395,385 | 114.00 | HOLDING | CLOSE_DIFF@114.00 profit=-8823.72 |
| 398,194 | 110.60 | HOLDING | TRIM@110.60 shares=1156.3 |
| 400,708 | 108.40 | HOLDING | MOVE_SETTLE_SKIP(p=26.80<med=57.35) |
| 405,133 | 99.40 | HOLDING | CLOSE_DIFF@99.40 profit=12950.72 |
| 405,802 | 97.40 | HOLDING | TRIM@97.40 shares=1107.6 |
| 407,247 | 98.75 | HOLDING | CLOSE_DIFF@98.75 profit=-1495.22 |
| 408,073 | 96.55 | HOLDING | TRIM@96.55 shares=1107.6 |
| 408,783 | 93.75 | HOLDING | MOVE_SETTLE_SKIP(p=10.20<med=16.80) |
| 409,422 | 91.65 | HOLDING | ADD_POS@91.65 +272.1 L1ratio=0.219 |
| 409,439 | 90.50 | WAIT_ENTRY | BSP_INVALIDATE@90.50 |
| 428,314 | 119.10 | ENTRY | WAIT→ENTRY BSP={1685477336} |
| 428,315 | 119.10 | HOLDING | ENTRY_LONG@119.10 L2dir=1 |
| 429,469 | 118.80 | HOLDING | TRIM@118.80 shares=807.6 |
| 429,983 | 119.35 | HOLDING | CLOSE_DIFF@119.35 profit=-444.15 |
| 431,636 | 121.50 | HOLDING | TRIM@121.50 shares=819.3 |
| 432,017 | 123.05 | HOLDING | CLOSE_DIFF@123.05 profit=-1269.97 |
| 435,026 | 120.20 | HOLDING | TRIM@120.20 shares=820.0 |
| 436,751 | 117.35 | WAIT_DIP | →EVAL(move_settle) high=123.50 force=11.70 p=11.70/med=6.05 → WAIT_DIP(U1完成 high=123.50) |
| 438,678 | 112.75 | HOLDING | DIP_REBUY@112.75 不跌破前低(down_low=112.00 >= prev_low=111.80) |
| 438,789 | 111.25 | WAIT_ENTRY | BSP_INVALIDATE@111.25 |
| 441,286 | 109.40 | ENTRY | WAIT→ENTRY BSP={1444010681} |
| 441,287 | 109.60 | HOLDING | ENTRY_LONG@109.60 L2dir=1 |
| 441,365 | 110.45 | HOLDING | TRIM@110.45 shares=863.1 |
| 443,150 | 106.00 | HOLDING | ADD_POS@106.00 +94.9 L1ratio=0.104 |
| 443,385 | 105.25 | WAIT_ENTRY | BSP_INVALIDATE@105.25 |
| 453,754 | 119.30 | ENTRY | WAIT→ENTRY BSP={1867672144} |
| 453,755 | 119.55 | HOLDING | ENTRY_LONG@119.55 L2dir=1 |
| 456,029 | 115.30 | HOLDING | TRIM@115.30 shares=817.6 |
| 457,173 | 109.70 | HOLDING | MOVE_SETTLE_SKIP(p=16.80<med=19.35) |
| 457,634 | 109.15 | HOLDING | ADD_POS@109.15 +132.6 L1ratio=0.159 |
| 457,850 | 109.95 | WAIT_ENTRY | BSP_INVALIDATE@109.95 |
| 468,633 | 124.45 | ENTRY | WAIT→ENTRY BSP={439129090} |
| 468,634 | 124.65 | HOLDING | ENTRY_LONG@124.65 L2dir=1 |
| 470,753 | 124.35 | HOLDING | TRIM@124.35 shares=786.2 |
| 471,012 | 124.05 | HOLDING | CLOSE_DIFF@124.05 profit=235.85 |
| 471,415 | 124.70 | HOLDING | TRIM@124.70 shares=798.1 |
| 471,599 | 124.55 | HOLDING | CLOSE_DIFF@124.55 profit=119.71 |
| 472,091 | 122.40 | HOLDING | TRIM@122.40 shares=798.1 |
| 474,856 | 124.05 | HOLDING | MOVE_SETTLE_SKIP(p=15.10<med=65.97) |
| 475,334 | 124.70 | HOLDING | ADD_POS@124.70 +45.6 L1ratio=0.057 |
| 477,069 | 123.35 | HOLDING | TRIM@123.35 shares=841.8 |
| 479,346 | 122.15 | HOLDING | CLOSE_DIFF@122.15 profit=1010.16 |
| 479,825 | 125.25 | HOLDING | MOVE_SETTLE_SKIP(p=9.20<med=66.47) |
| 480,282 | 125.35 | HOLDING | ADD_POS@125.35 +48.5 L1ratio=0.057 |
| 481,043 | 133.40 | HOLDING | TRIM@133.40 shares=893.2 |
| 481,240 | 134.50 | HOLDING | CLOSE_DIFF@134.50 profit=-982.53 |
| 482,025 | 131.35 | HOLDING | TRIM@131.35 shares=843.7 |
| 482,914 | 134.60 | HOLDING | CLOSE_DIFF@134.60 profit=-2742.17 |
| 483,010 | 136.15 | HOLDING | MOVE_SETTLE_SKIP(p=17.15<med=133.85) |
| 485,735 | 145.20 | HOLDING | TRIM@145.20 shares=813.4 |
| 486,139 | 146.45 | HOLDING | CLOSE_DIFF@146.45 profit=-1016.79 |
| 486,777 | 145.75 | HOLDING | TRIM@145.75 shares=857.3 |
| 487,544 | 150.35 | HOLDING | MOVE_SETTLE_SKIP(p=28.30<med=82.45) |
| 487,689 | 150.90 | HOLDING | ADD_POS@150.90 +67.2 L1ratio=0.075 |
| 488,643 | 149.60 | HOLDING | TRIM@149.60 shares=949.2 |
| 488,865 | 150.70 | HOLDING | CLOSE_DIFF@150.70 profit=-1044.09 |
| 489,342 | 149.50 | HOLDING | TRIM@149.50 shares=949.2 |
| 491,627 | 147.65 | HOLDING | CLOSE_DIFF@147.65 profit=1755.96 |
| 492,712 | 149.35 | WAIT_DIP | →EVAL(move_settle) high=152.65 force=12.10 p=12.10/med=11.50 → WAIT_DIP(U1完成 high=152.65) |
| 492,853 | 149.90 | HOLDING | DIP_REBUY@149.90 不跌破前低(down_low=148.80 >= prev_low=140.55) |
| 492,911 | 150.15 | HOLDING | TRIM@150.15 shares=1017.1 |
| 495,992 | 144.90 | HOLDING | CLOSE_DIFF@144.90 profit=5339.62 |
| 497,215 | 144.55 | HOLDING | TRIM@144.55 shares=1017.1 |
| 497,373 | 144.15 | HOLDING | CLOSE_DIFF@144.15 profit=406.83 |
| 497,822 | 143.15 | HOLDING | MOVE_SETTLE_SKIP(p=11.15<med=11.20) |
| 498,076 | 142.15 | HOLDING | TRIM@142.15 shares=968.7 |
| 498,442 | 141.15 | HOLDING | CLOSE_DIFF@141.15 profit=968.72 |
| 499,585 | 142.25 | HOLDING | TRIM@142.25 shares=984.4 |
| 500,267 | 142.30 | HOLDING | CLOSE_DIFF@142.30 profit=-49.22 |
| 500,870 | 142.90 | HOLDING | TRIM@142.90 shares=984.4 |
| 501,158 | 145.10 | HOLDING | CLOSE_DIFF@145.10 profit=-2165.64 |
| 502,022 | 149.70 | HOLDING | ADD_POS@149.70 +99.8 L1ratio=0.096 |
| 502,721 | 140.55 | HOLDING | ADD_POS@140.55 +109.3 L1ratio=0.096 |
| 502,832 | 140.55 | WAIT_ENTRY | BSP_INVALIDATE@140.55 |
| 576,773 | 158.80 | ENTRY | WAIT→ENTRY BSP={1264064818} |
| 576,774 | 158.80 | HOLDING | ENTRY_LONG@158.80 L2dir=1 |
| 578,384 | 158.10 | HOLDING | TRIM@158.10 shares=626.7 |
| 578,760 | 161.05 | HOLDING | CLOSE_DIFF@161.05 profit=-1848.88 |
| 582,192 | 162.80 | HOLDING | MOVE_SETTLE_SKIP(p=17.85<med=161.90) |
| 582,807 | 157.90 | HOLDING | TRIM@157.90 shares=628.4 |
| 583,315 | 155.40 | HOLDING | CLOSE_DIFF@155.40 profit=1570.91 |
| 584,519 | 164.90 | HOLDING | MOVE_SETTLE_SKIP(p=9.80<med=85.50) |
| 585,065 | 160.30 | HOLDING | TRIM@160.30 shares=624.0 |
| 586,264 | 166.45 | HOLDING | CLOSE_DIFF@166.45 profit=-3837.32 |
| 587,565 | 170.45 | HOLDING | TRIM@170.45 shares=621.0 |
| 587,878 | 170.15 | HOLDING | CLOSE_DIFF@170.15 profit=186.30 |
| 589,223 | 170.90 | HOLDING | TRIM@170.90 shares=626.4 |
| 589,341 | 171.80 | HOLDING | CLOSE_DIFF@171.80 profit=-563.76 |
| 590,137 | 172.65 | HOLDING | TRIM@172.65 shares=620.5 |
| 591,175 | 171.10 | HOLDING | MOVE_SETTLE_SKIP(p=20.65<med=173.40) |
| 592,669 | 173.40 | HOLDING | CLOSE_DIFF@173.40 profit=-465.35 |
| 593,284 | 172.45 | HOLDING | TRIM@172.45 shares=629.2 |
| 594,708 | 181.60 | HOLDING | CLOSE_DIFF@181.60 profit=-5756.98 |
| 595,001 | 185.45 | HOLDING | TRIM@185.45 shares=603.4 |
| 595,530 | 186.95 | HOLDING | CLOSE_DIFF@186.95 profit=-905.17 |
| 596,545 | 186.55 | HOLDING | TRIM@186.55 shares=605.9 |
| 598,844 | 187.10 | HOLDING | CLOSE_DIFF@187.10 profit=-333.27 |
| 600,982 | 194.10 | HOLDING | TRIM@194.10 shares=598.5 |
| 602,947 | 194.70 | HOLDING | CLOSE_DIFF@194.70 profit=-359.12 |
| 604,357 | 198.90 | HOLDING | MOVE_SETTLE_SKIP(p=32.30<med=198.95) |
| 606,012 | 199.80 | HOLDING | TRIM@199.80 shares=628.5 |
| ... | ... | ... | （共269条，显示前150） |
</details>

## 控制变量对比

| 版本 | QQQ | OKLO | HK700 |
|------|-----|------|-------|
| 5. 纯做多 | +71.7% | +228% | +292.9% |
| 7a. persistence过滤（价格振幅） | +58.7% | +194.4% | +313.0% |
| 7b. **persistence过滤（MACD面积）** | **+58.9%** | **+194.4%** | **+252.3%** |

唯一差异：盘整背驰度量（价格振幅 vs MACD面积）。

## 汇总

| 标的 | Bars | BH% | 复利% | 胜率 | 交易数 | L2翻转 | 加仓 | 降成本 | PW | 段比较 | 耗时 |
|------|------|-----|------|------|--------|--------|------|--------|-----|--------|------|
| QQQ | 728,030 | +174.6 | +58.88 | 70% | 30 | 14 | 20/30 | 24/30 | 0/30 | 5/30 | 432s |
| OKLO | 333,613 | +251.3 | +194.37 | 56% | 9 | 7 | 7/9 | 7/9 | 0/9 | 1/9 | 79s |
| HK700 | 1,408,882 | +3933.5 | +252.26 | 73% | 15 | 5 | 15/15 | 15/15 | 0/15 | 2/15 | 541s |

## 结果包六要素

**结论**：控制变量实验——盘整背驰度量从价格振幅改为MACD面积。

**定义依据**：
- 38课盘整背驰 = A段力度 vs C段力度
- 价格振幅：|high-low|（几何度量）
- MACD面积：sum(hist>0)（动量度量）
- 两者测量不同的东西——振幅测波动范围，MACD测动量累积

**边界条件**：
- PENDING_EXPIRY = 390 bars
- MACD参数(12,26,9)标准值
- 只影响「创新高+背驰」分支，不影响「不创新高」分支

**下游推论**：
- 若MACD面积版 > 价格振幅版 → MACD动量度量更精确识别力度衰竭
- 若MACD面积版 < 价格振幅版 → MACD面积在走势级别上更宽松（更多假背驰）
- 若MACD面积版 ≈ 价格振幅版 → 度量选择不是关键变量

**谱系引用**：
- 525号：组件局部完成性
- 526号：a0递归存在论区分
- 521号：PH纯拓扑无动量

**影响声明**：新建独立回测脚本，不修改引擎代码或旧版回测。

**认识论等级**：L2（真实数据，3标的 1min；可产生否定性结果）。