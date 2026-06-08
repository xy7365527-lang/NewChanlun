# 完整版缠论+PH多重赋格回测 — 阶段一（纯做多+降成本）

## 架构

基于 docs/architecture/complete_fugue_design.md 实现。

核心改进（相比 longonly 版）：

1. **完整38课操作程式FSM**（6状态）：
   WAIT_ENTRY→ENTRY→HOLDING→EVAL→EXIT/WAIT_DIP→OBSERVE
2. **走势段比较**（S7-S9）：不创新高/盘整背驰判断
3. **区间套定位**：L0 PH settle精确入场
4. **降成本子状态机**：FULL_POS↔REDUCED→PRINCIPAL_RECOVERED

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
- 回测耗时：368.4s

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
| 交易数 | 41（纯多头） |
| 胜率 | 58.5% |
| 平均收益 | +0.534% |
| 复利累计 | **+23.08%** |
| 最大回撤 | -7.94% |
| 平均持仓 | 4,837 bars |
| 有加仓的交易 | 12/41 |
| 有降成本的交易 | 29/41 |
| 达到本金回收 | 0/41 |
| 有段间比较的交易 | 23/41 |

### 交易明细

| # | 入场 | 出场 | 持仓bars | PnL% | 加仓 | 短差 | 向上段数 | 退出原因 | 状态路径 |
|---|------|------|---------|------|------|------|---------|---------|---------|
| 1 | 313.83@45927 | 308.80@47393 | 1,466 | -1.57 | 1 | 0 | 2 | no_new_high | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 2 | 314.81@50896 | 314.50@54689 | 3,793 | +0.34 | 0 | 3 | 2 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 3 | 322.52@67708 | 319.88@67882 | 174 | -0.82 | 0 | 0 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 4 | 319.41@67966 | 318.33@72362 | 4,396 | +0.08 | 2 | 3 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 5 | 323.61@72704 | 331.49@82871 | 10,167 | +2.91 | 2 | 7 | 4 | no_new_high | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 6 | 338.83@83683 | 352.97@87747 | 4,064 | +4.19 | 1 | 2 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 7 | 357.54@92089 | 354.25@92162 | 73 | -0.92 | 0 | 0 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 8 | 357.61@93056 | 365.80@97876 | 4,820 | +2.12 | 1 | 4 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 9 | 377.02@109966 | 378.90@114270 | 4,304 | +0.62 | 0 | 3 | 2 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 10 | 358.17@131229 | 358.07@131250 | 21 | -0.03 | 0 | 0 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 11 | 389.89@186027 | 388.55@192317 | 6,290 | +0.12 | 1 | 5 | 2 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 12 | 395.37@196836 | 405.90@202741 | 5,905 | +2.85 | 0 | 6 | 2 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 13 | 410.31@204602 | 396.84@208841 | 4,239 | -2.73 | 1 | 4 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 14 | 413.10@216486 | 423.91@220988 | 4,502 | +2.99 | 0 | 3 | 2 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 15 | 417.42@224067 | 417.42@224154 | 87 | +0.00 | 0 | 0 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 16 | 435.14@229309 | 429.72@231925 | 2,616 | -1.19 | 1 | 0 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 17 | 439.04@236273 | 439.10@243197 | 6,924 | +1.18 | 1 | 4 | 2 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 18 | 448.45@252490 | 442.69@256018 | 3,528 | -1.26 | 0 | 3 | 2 | no_new_high | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 19 | 456.16@288173 | 454.68@292105 | 3,932 | -0.32 | 0 | 4 | 2 | no_new_high | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 20 | 463.86@296109 | 488.05@303525 | 7,416 | +5.43 | 0 | 7 | 3 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 21 | 488.67@310993 | 489.62@318067 | 7,074 | +0.29 | 1 | 5 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 22 | 487.55@318164 | 484.92@318262 | 98 | -0.54 | 0 | 0 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 23 | 519.92@400612 | 522.24@403599 | 2,987 | +0.45 | 0 | 2 | 2 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 24 | 527.47@404718 | 529.88@404963 | 245 | +0.46 | 0 | 0 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 25 | 530.41@405040 | 498.50@448805 | 43,765 | -6.02 | 0 | 1 | 2 | l2_direction_flip_short_in_observe | ENTRY→EVAL→HOLDING→OBSERVE→WAIT_DIP |
| 26 | 544.24@520848 | 547.80@523752 | 2,904 | +0.65 | 0 | 3 | 2 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 27 | 556.10@525234 | 556.21@525240 | 6 | +0.02 | 0 | 0 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 28 | 558.98@530703 | 562.89@535181 | 4,478 | +1.36 | 0 | 3 | 3 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 29 | 569.50@538460 | 552.73@542613 | 4,153 | -2.92 | 0 | 3 | 2 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 30 | 580.40@549099 | 565.70@554420 | 5,321 | -1.13 | 1 | 6 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 31 | 583.18@566174 | 602.07@573427 | 7,253 | +3.24 | 0 | 6 | 3 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 32 | 602.89@573743 | 598.85@578930 | 5,187 | -0.52 | 0 | 2 | 2 | no_new_high | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 33 | 605.01@579857 | 609.14@583852 | 3,995 | +0.68 | 0 | 2 | 2 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 34 | 609.88@583978 | 597.06@587745 | 3,767 | -1.97 | 1 | 2 | 3 | no_new_high | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 35 | 616.76@594461 | 633.27@599842 | 5,381 | +2.84 | 0 | 1 | 3 | no_new_high | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 36 | 617.82@601418 | 617.32@601511 | 93 | -0.08 | 0 | 0 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 37 | 599.51@603661 | 603.75@603769 | 108 | +0.71 | 0 | 0 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 38 | 650.00@699682 | 660.06@702498 | 2,816 | +2.36 | 0 | 3 | 2 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 39 | 664.10@702930 | 703.02@717431 | 14,501 | +7.30 | 0 | 14 | 4 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 40 | 702.00@718194 | 701.88@718219 | 25 | -0.02 | 0 | 0 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 41 | 733.50@722602 | 738.28@728029 | 5,427 | +0.72 | 0 | 6 | 1 | eod_close | ENTRY→EVAL→HOLDING→WAIT_DIP |

### FSM 状态分布（按 bar 数）

| 状态 | Bars | 占比 |
|------|------|------|
| WAIT_ENTRY | 529,688 | 72.8% |
| ENTRY | 41 | 0.0% |
| HOLDING | 144,266 | 19.8% |
| EVAL | 0 | 0.0% |
| WAIT_DIP | 15,731 | 2.2% |
| OBSERVE | 38,304 | 5.3% |
| EXIT | 0 | 0.0% |

<details><summary>事件流（478条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 45,926 | 313.67 | ENTRY | WAIT→ENTRY BSP={1699618109} |
| 45,927 | 313.83 | HOLDING | ENTRY_LONG@313.83 L2dir=1 |
| 46,025 | 309.74 | HOLDING | TRIM@309.74 shares=314.4 |
| 46,384 | 309.40 | WAIT_DIP | →EVAL(move_settle) high=315.24 force=13.26 → WAIT_DIP(U1完成 high=315.24) |
| 46,714 | 313.12 | HOLDING | DIP_REBUY@313.12 不跌破前低(down_low=308.40 >= prev_low=301.98) |
| 47,157 | 310.23 | HOLDING | TRIM@310.23 shares=364.8 |
| 47,393 | 308.80 | WAIT_ENTRY | →EVAL(move_settle) high=314.15 force=8.57 → EXIT(不创新高 314.15<315.24) |
| 50,895 | 315.00 | ENTRY | WAIT→ENTRY BSP={987157400} |
| 50,896 | 314.81 | HOLDING | ENTRY_LONG@314.81 L2dir=1 |
| 51,694 | 315.66 | HOLDING | TRIM@315.66 shares=315.9 |
| 52,243 | 321.03 | HOLDING | CLOSE_DIFF@321.03 profit=-1696.65 |
| 52,737 | 319.50 | HOLDING | TRIM@319.50 shares=292.4 |
| 52,767 | 319.20 | HOLDING | CLOSE_DIFF@319.20 profit=87.71 |
| 52,924 | 318.36 | HOLDING | TRIM@318.36 shares=292.4 |
| 53,510 | 321.52 | WAIT_DIP | →EVAL(move_settle) high=321.44 force=14.07 → WAIT_DIP(U1完成 high=321.44) |
| 53,630 | 320.80 | HOLDING | DIP_REBUY@320.80 不跌破前低(down_low=320.53 >= prev_low=307.37) |
| 53,767 | 319.65 | HOLDING | TRIM@319.65 shares=312.5 |
| 54,238 | 318.54 | HOLDING | CLOSE_DIFF@318.54 profit=346.83 |
| 54,689 | 314.50 | WAIT_ENTRY | →EVAL(move_settle) high=322.17 force=4.65 → EXIT(盘整背驰 force 4.65<14.07) |
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
| 71,368 | 318.03 | WAIT_DIP | →EVAL(move_settle) high=323.62 force=10.29 → WAIT_DIP(U1完成 high=323.62) |
| 72,003 | 316.08 | HOLDING | DIP_REBUY@316.08 不跌破前低(down_low=315.28 >= prev_low=313.33) |
| 72,362 | 318.33 | WAIT_ENTRY | BSP_INVALIDATE@318.33 |
| 72,703 | 323.59 | ENTRY | WAIT→ENTRY BSP={285522146} |
| 72,704 | 323.61 | HOLDING | ENTRY_LONG@323.61 L2dir=1 |
| 73,460 | 323.47 | HOLDING | TRIM@323.47 shares=308.3 |
| 74,028 | 322.43 | HOLDING | CLOSE_DIFF@322.43 profit=320.67 |
| 74,074 | 321.91 | HOLDING | TRIM@321.91 shares=307.0 |
| 74,503 | 321.04 | WAIT_DIP | →EVAL(move_settle) high=324.04 force=6.56 → WAIT_DIP(U1完成 high=324.04) |
| 74,691 | 324.30 | HOLDING | DIP_REBUY@324.30 不跌破前低(down_low=320.73 >= prev_low=317.48) |
| 74,864 | 324.55 | HOLDING | TRIM@324.55 shares=307.0 |
| 74,926 | 324.00 | HOLDING | CLOSE_DIFF@324.00 profit=168.84 |
| 75,607 | 325.21 | HOLDING | TRIM@325.21 shares=300.2 |
| 75,993 | 326.14 | HOLDING | CLOSE_DIFF@326.14 profit=-279.19 |
| 76,441 | 325.48 | HOLDING | TRIM@325.48 shares=307.8 |
| 76,897 | 326.05 | WAIT_DIP | →EVAL(move_settle) high=327.33 force=7.11 → WAIT_DIP(创新高无背驰 high=327.33) |
| 77,031 | 325.95 | HOLDING | DIP_REBUY@325.95 不跌破前低(down_low=325.60 >= prev_low=320.22) |
| 77,328 | 325.96 | HOLDING | ADD_POS@325.96 +19.3 L1ratio=0.059 |
| 77,981 | 327.22 | HOLDING | TRIM@327.22 shares=344.5 |
| 78,031 | 327.75 | HOLDING | CLOSE_DIFF@327.75 profit=-182.60 |
| 78,440 | 327.44 | HOLDING | TRIM@327.44 shares=339.6 |
| 78,534 | 327.64 | HOLDING | CLOSE_DIFF@327.64 profit=-67.92 |
| 78,826 | 328.60 | HOLDING | TRIM@328.60 shares=343.2 |
| 79,420 | 332.09 | HOLDING | CLOSE_DIFF@332.09 profit=-1197.67 |
| 80,484 | 336.69 | HOLDING | TRIM@336.69 shares=339.9 |
| 81,188 | 336.32 | HOLDING | CLOSE_DIFF@336.32 profit=125.76 |
| 81,552 | 338.07 | WAIT_DIP | →EVAL(move_settle) high=338.88 force=15.94 → WAIT_DIP(创新高无背驰 high=338.88) |
| 82,822 | 331.37 | HOLDING | DIP_REBUY@331.37 不跌破前低(down_low=331.24 >= prev_low=322.94) |
| 82,871 | 331.49 | WAIT_ENTRY | →EVAL(move_settle) high=338.78 force=3.35 → EXIT(不创新高 338.78<338.88) |
| 83,682 | 338.83 | ENTRY | WAIT→ENTRY BSP={1758490280} |
| 83,683 | 338.83 | HOLDING | ENTRY_LONG@338.83 L2dir=1 |
| 85,792 | 351.01 | HOLDING | TRIM@351.01 shares=279.4 |
| 86,208 | 349.29 | WAIT_DIP | →EVAL(move_settle) high=354.18 force=23.78 → WAIT_DIP(U1完成 high=354.18) |
| 86,232 | 348.65 | HOLDING | DIP_REBUY@348.65 不跌破前低(down_low=348.45 >= prev_low=330.40) |
| 86,331 | 348.60 | HOLDING | TRIM@348.60 shares=272.0 |
| 86,834 | 347.42 | HOLDING | CLOSE_DIFF@347.42 profit=320.98 |
| 87,198 | 348.05 | HOLDING | TRIM@348.05 shares=272.0 |
| 87,236 | 347.99 | HOLDING | CLOSE_DIFF@347.99 profit=16.32 |
| 87,663 | 352.53 | HOLDING | ADD_POS@352.53 +23.7 L1ratio=0.080 |
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
| 97,237 | 364.91 | WAIT_DIP | →EVAL(move_settle) high=372.82 force=23.92 → WAIT_DIP(U1完成 high=372.82) |
| 97,689 | 366.89 | HOLDING | DIP_REBUY@366.89 不跌破前低(down_low=364.75 >= prev_low=348.90) |
| 97,876 | 365.80 | WAIT_ENTRY | BSP_INVALIDATE@365.80 |
| 109,965 | 377.16 | ENTRY | WAIT→ENTRY BSP={4111992} |
| 109,966 | 377.02 | HOLDING | ENTRY_LONG@377.02 L2dir=1 |
| 110,916 | 380.20 | HOLDING | TRIM@380.20 shares=259.0 |
| 111,605 | 380.26 | HOLDING | CLOSE_DIFF@380.26 profit=-15.54 |
| 111,754 | 381.36 | WAIT_DIP | →EVAL(move_settle) high=382.86 force=18.24 → WAIT_DIP(U1完成 high=382.86) |
| 111,843 | 382.39 | HOLDING | DIP_REBUY@382.39 不跌破前低(down_low=381.23 >= prev_low=364.62) |
| 112,060 | 382.55 | HOLDING | TRIM@382.55 shares=264.3 |
| 112,096 | 382.10 | HOLDING | CLOSE_DIFF@382.10 profit=118.94 |
| 112,795 | 385.24 | HOLDING | TRIM@385.24 shares=264.7 |
| 112,977 | 386.31 | HOLDING | CLOSE_DIFF@386.31 profit=-283.20 |
| 113,388 | 385.11 | HOLDING | TRIM@385.11 shares=255.3 |
| 114,270 | 378.90 | WAIT_ENTRY | →EVAL(move_settle) high=387.98 force=9.80 → EXIT(盘整背驰 force 9.80<18.24) |
| 131,228 | 357.95 | ENTRY | WAIT→ENTRY BSP={1073924458} |
| 131,229 | 358.17 | HOLDING | ENTRY_LONG@358.17 L2dir=1 |
| 131,250 | 358.07 | WAIT_ENTRY | BSP_INVALIDATE@358.07 |
| 186,026 | 389.75 | ENTRY | WAIT→ENTRY BSP={77258701} |
| 186,027 | 389.89 | HOLDING | ENTRY_LONG@389.89 L2dir=1 |
| 186,738 | 387.46 | HOLDING | TRIM@387.46 shares=250.5 |
| 187,452 | 390.52 | HOLDING | CLOSE_DIFF@390.52 profit=-766.63 |
| 187,547 | 390.47 | WAIT_DIP | →EVAL(move_settle) high=391.41 force=7.12 → WAIT_DIP(U1完成 high=391.41) |
| 187,637 | 390.83 | HOLDING | DIP_REBUY@390.83 不跌破前低(down_low=390.14 >= prev_low=384.29) |
| 188,058 | 389.84 | HOLDING | TRIM@389.84 shares=255.9 |
| 188,296 | 389.12 | HOLDING | CLOSE_DIFF@389.12 profit=184.26 |
| 188,427 | 389.31 | HOLDING | TRIM@389.31 shares=255.9 |
| 188,830 | 390.08 | HOLDING | CLOSE_DIFF@390.08 profit=-197.06 |
| 189,110 | 389.90 | HOLDING | TRIM@389.90 shares=255.9 |
| 189,589 | 389.56 | HOLDING | CLOSE_DIFF@389.56 profit=87.01 |
| 190,433 | 391.27 | HOLDING | TRIM@391.27 shares=252.7 |
| 190,749 | 390.22 | WAIT_DIP | →EVAL(move_settle) high=394.14 force=7.64 → WAIT_DIP(创新高无背驰 high=394.14) |
| 191,243 | 386.20 | HOLDING | DIP_REBUY_DIVERGE@386.20 跌破+盘整背驰 |
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
| 200,569 | 406.38 | WAIT_DIP | →EVAL(move_settle) high=406.30 force=19.33 → WAIT_DIP(U1完成 high=406.30) |
| 200,579 | 406.48 | HOLDING | DIP_REBUY@406.48 不跌破前低(down_low=406.26 >= prev_low=386.97) |
| 201,771 | 408.25 | HOLDING | TRIM@408.25 shares=251.8 |
| 202,150 | 409.79 | HOLDING | CLOSE_DIFF@409.79 profit=-387.81 |
| 202,741 | 405.90 | WAIT_ENTRY | →EVAL(move_settle) high=410.46 force=10.12 → EXIT(盘整背驰 force 10.12<19.33) |
| 204,601 | 410.42 | ENTRY | WAIT→ENTRY BSP={153863425} |
| 204,602 | 410.31 | HOLDING | ENTRY_LONG@410.31 L2dir=1 |
| 205,470 | 411.80 | HOLDING | TRIM@411.80 shares=242.3 |
| 205,494 | 411.57 | HOLDING | CLOSE_DIFF@411.57 profit=55.73 |
| 205,846 | 411.95 | HOLDING | TRIM@411.95 shares=241.1 |
| 206,018 | 411.99 | HOLDING | CLOSE_DIFF@411.99 profit=-9.64 |
| 206,147 | 411.26 | HOLDING | TRIM@411.26 shares=241.1 |
| 206,729 | 409.61 | WAIT_DIP | →EVAL(move_settle) high=413.10 force=10.22 → WAIT_DIP(U1完成 high=413.10) |
| 207,269 | 403.50 | HOLDING | DIP_REBUY_DIVERGE@403.50 跌破+盘整背驰 |
| ... | ... | ... | （共478条，显示前150） |
</details>

## OKLO

- 数据：**333,613** bars (1min)
- 价格：18.07 → 63.48
- Buy-and-hold: **+251.30%**
- 回测耗时：72.4s

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
| 交易数 | 12（纯多头） |
| 胜率 | 41.7% |
| 平均收益 | +8.758% |
| 复利累计 | **+133.23%** |
| 最大回撤 | -17.97% |
| 平均持仓 | 3,304 bars |
| 有加仓的交易 | 8/12 |
| 有降成本的交易 | 9/12 |
| 达到本金回收 | 0/12 |
| 有段间比较的交易 | 5/12 |

### 交易明细

| # | 入场 | 出场 | 持仓bars | PnL% | 加仓 | 短差 | 向上段数 | 退出原因 | 状态路径 |
|---|------|------|---------|------|------|------|---------|---------|---------|
| 1 | 13.00@41092 | 20.04@43481 | 2,389 | +56.79 | 0 | 2 | 2 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 2 | 21.67@43694 | 19.11@46795 | 3,101 | -8.43 | 1 | 1 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 3 | 24.50@47162 | 20.33@50552 | 3,390 | -8.61 | 1 | 3 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 4 | 28.23@53131 | 19.57@57372 | 4,241 | -1.98 | 2 | 6 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 5 | 32.84@82272 | 44.60@88990 | 6,718 | +26.83 | 3 | 3 | 2 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 6 | 46.01@89134 | 54.69@94200 | 5,066 | +17.37 | 2 | 5 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 7 | 66.89@151335 | 65.64@151540 | 205 | -1.87 | 0 | 0 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 8 | 76.66@173693 | 71.38@176091 | 2,398 | -6.91 | 1 | 1 | 2 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 9 | 78.61@177103 | 76.68@178964 | 1,861 | -2.15 | 1 | 0 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 10 | 65.04@187215 | 64.39@187431 | 216 | -1.00 | 0 | 0 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 11 | 89.23@199495 | 110.98@206090 | 6,595 | +24.38 | 0 | 3 | 2 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 12 | 146.69@215838 | 163.65@219307 | 3,469 | +10.68 | 1 | 2 | 2 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP |

### FSM 状态分布（按 bar 数）

| 状态 | Bars | 占比 |
|------|------|------|
| WAIT_ENTRY | 293,952 | 88.1% |
| ENTRY | 12 | 0.0% |
| HOLDING | 35,893 | 10.8% |
| EVAL | 0 | 0.0% |
| WAIT_DIP | 3,756 | 1.1% |
| OBSERVE | 0 | 0.0% |
| EXIT | 0 | 0.0% |

<details><summary>事件流（122条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 41,091 | 13.08 | ENTRY | WAIT→ENTRY BSP={1146069223} |
| 41,092 | 13.00 | HOLDING | ENTRY_LONG@13.00 L2dir=1 |
| 41,862 | 16.22 | HOLDING | TRIM@16.22 shares=6929.3 |
| 42,565 | 15.84 | HOLDING | CLOSE_DIFF@15.84 profit=2633.15 |
| 42,786 | 19.43 | WAIT_DIP | →EVAL(move_settle) high=17.71 force=8.58 → WAIT_DIP(U1完成 high=17.71) |
| 42,906 | 19.52 | HOLDING | DIP_REBUY@19.52 不跌破前低(down_low=19.16 >= prev_low=9.13) |
| 43,016 | 18.46 | HOLDING | TRIM@18.46 shares=6861.9 |
| 43,450 | 19.91 | HOLDING | CLOSE_DIFF@19.91 profit=-9949.72 |
| 43,481 | 20.04 | WAIT_ENTRY | →EVAL(move_settle) high=20.64 force=6.51 → EXIT(盘整背驰 force 6.51<8.58) |
| 43,693 | 21.68 | ENTRY | WAIT→ENTRY BSP={787574734} |
| 43,694 | 21.67 | HOLDING | ENTRY_LONG@21.67 L2dir=1 |
| 44,516 | 20.63 | HOLDING | TRIM@20.63 shares=4252.0 |
| 44,992 | 20.44 | HOLDING | CLOSE_DIFF@20.44 profit=807.88 |
| 45,323 | 18.34 | WAIT_DIP | →EVAL(move_settle) high=23.05 force=5.85 → WAIT_DIP(U1完成 high=23.05) |
| 46,744 | 19.14 | HOLDING | DIP_REBUY@19.14 不跌破前低(down_low=17.62 >= prev_low=17.20) |
| 46,795 | 19.11 | WAIT_ENTRY | BSP_INVALIDATE@19.11 |
| 47,161 | 24.60 | ENTRY | WAIT→ENTRY BSP={308273581} |
| 47,162 | 24.50 | HOLDING | ENTRY_LONG@24.50 L2dir=1 |
| 47,960 | 23.43 | HOLDING | TRIM@23.43 shares=3935.3 |
| 48,481 | 26.75 | HOLDING | CLOSE_DIFF@26.75 profit=-13065.35 |
| 48,715 | 23.60 | HOLDING | TRIM@23.60 shares=3627.5 |
| 49,561 | 22.99 | HOLDING | CLOSE_DIFF@22.99 profit=2212.78 |
| 49,609 | 22.92 | HOLDING | TRIM@22.92 shares=3627.5 |
| 50,300 | 21.43 | WAIT_DIP | →EVAL(move_settle) high=28.12 force=10.00 → WAIT_DIP(U1完成 high=28.12) |
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
| 54,589 | 23.20 | WAIT_DIP | →EVAL(move_settle) high=28.68 force=8.68 → WAIT_DIP(U1完成 high=28.68) |
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
| 85,820 | 31.29 | WAIT_DIP | →EVAL(move_settle) high=43.70 force=19.47 → WAIT_DIP(U1完成 high=43.70) |
| 86,342 | 34.20 | HOLDING | DIP_REBUY@34.20 不跌破前低(down_low=30.17 >= prev_low=24.23) |
| 87,232 | 36.11 | HOLDING | ADD_POS@36.11 +1413.7 L1ratio=0.345 |
| 87,675 | 41.59 | HOLDING | ADD_POS@41.59 +1901.6 L1ratio=0.345 |
| 88,040 | 41.95 | HOLDING | TRIM@41.95 shares=7340.2 |
| 88,940 | 43.40 | HOLDING | CLOSE_DIFF@43.40 profit=-10680.03 |
| 88,990 | 44.60 | WAIT_ENTRY | →EVAL(move_settle) high=45.21 force=12.21 → EXIT(盘整背驰 force 12.21<19.47) |
| 89,133 | 46.11 | ENTRY | WAIT→ENTRY BSP={1157762651} |
| 89,134 | 46.01 | HOLDING | ENTRY_LONG@46.01 L2dir=1 |
| 89,466 | 46.55 | HOLDING | TRIM@46.55 shares=2145.0 |
| 90,008 | 45.95 | HOLDING | CLOSE_DIFF@45.95 profit=1287.01 |
| 91,119 | 48.95 | HOLDING | TRIM@48.95 shares=1944.4 |
| 91,475 | 48.63 | HOLDING | CLOSE_DIFF@48.63 profit=612.49 |
| 92,285 | 54.89 | HOLDING | TRIM@54.89 shares=2108.6 |
| 92,312 | 55.20 | HOLDING | CLOSE_DIFF@55.20 profit=-653.67 |
| 92,466 | 54.30 | HOLDING | TRIM@54.30 shares=2108.6 |
| 93,016 | 52.38 | HOLDING | CLOSE_DIFF@52.38 profit=4038.01 |
| 93,454 | 50.88 | HOLDING | TRIM@50.88 shares=1971.5 |
| 93,772 | 52.73 | HOLDING | CLOSE_DIFF@52.73 profit=-3637.44 |
| 93,829 | 54.11 | WAIT_DIP | →EVAL(move_settle) high=59.11 force=21.28 → WAIT_DIP(U1完成 high=59.11) |
| 93,903 | 54.91 | HOLDING | DIP_REBUY@54.91 不跌破前低(down_low=53.89 >= prev_low=37.83) |
| 94,086 | 54.37 | HOLDING | ADD_POS@54.37 +511.1 L1ratio=0.197 |
| 94,200 | 54.69 | WAIT_ENTRY | BSP_INVALIDATE@54.69 |
| 151,334 | 66.80 | ENTRY | WAIT→ENTRY BSP={528094879} |
| 151,335 | 66.89 | HOLDING | ENTRY_LONG@66.89 L2dir=1 |
| 151,540 | 65.64 | WAIT_ENTRY | BSP_INVALIDATE@65.64 |
| 173,692 | 76.58 | ENTRY | WAIT→ENTRY BSP={1926062708} |
| 173,693 | 76.66 | HOLDING | ENTRY_LONG@76.66 L2dir=1 |
| 173,991 | 72.22 | HOLDING | TRIM@72.22 shares=1204.8 |
| 174,553 | 75.00 | WAIT_DIP | →EVAL(move_settle) high=77.16 force=15.42 → WAIT_DIP(U1完成 high=77.16) |
| 174,681 | 77.17 | HOLDING | DIP_REBUY@77.17 不跌破前低(down_low=74.81 >= prev_low=61.74) |
| 174,693 | 76.95 | HOLDING | ADD_POS@76.95 +95.8 L1ratio=0.073 |
| 175,060 | 74.46 | HOLDING | TRIM@74.46 shares=1359.8 |
| 175,622 | 75.20 | HOLDING | CLOSE_DIFF@75.20 profit=-1006.27 |
| 176,091 | 71.38 | WAIT_ENTRY | →EVAL(move_settle) high=78.54 force=7.32 → EXIT(盘整背驰 force 7.32<15.42) |
| 177,102 | 78.49 | ENTRY | WAIT→ENTRY BSP={352607149} |
| 177,103 | 78.61 | HOLDING | ENTRY_LONG@78.61 L2dir=1 |
| 177,478 | 76.18 | HOLDING | TRIM@76.18 shares=1240.2 |
| 178,300 | 71.49 | WAIT_DIP | →EVAL(move_settle) high=80.42 force=9.23 → WAIT_DIP(U1完成 high=80.42) |
| 178,793 | 76.59 | HOLDING | DIP_REBUY@76.59 不跌破前低(down_low=71.50 >= prev_low=71.19) |
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
| 203,886 | 138.20 | WAIT_DIP | →EVAL(move_settle) high=138.00 force=65.27 → WAIT_DIP(U1完成 high=138.00) |
| 204,355 | 134.64 | HOLDING | DIP_REBUY@134.64 不跌破前低(down_low=132.89 >= prev_low=72.73) |
| 205,261 | 136.24 | HOLDING | TRIM@136.24 shares=1109.4 |
| 205,442 | 138.40 | HOLDING | CLOSE_DIFF@138.40 profit=-2396.28 |
| 205,518 | 136.88 | HOLDING | TRIM@136.88 shares=1109.4 |
| 206,090 | 110.98 | WAIT_ENTRY | →EVAL(move_settle) high=145.00 force=21.25 → EXIT(盘整背驰 force 21.25<65.27) |
| 215,837 | 146.72 | ENTRY | WAIT→ENTRY BSP={244261304} |
| 215,838 | 146.69 | HOLDING | ENTRY_LONG@146.69 L2dir=1 |
| 215,927 | 147.00 | HOLDING | TRIM@147.00 shares=610.0 |
| 216,132 | 158.28 | HOLDING | CLOSE_DIFF@158.28 profit=-6881.15 |
| 216,282 | 157.66 | HOLDING | ADD_POS@157.66 +90.8 L1ratio=0.133 |
| 217,096 | 167.76 | HOLDING | TRIM@167.76 shares=706.8 |
| 217,321 | 170.10 | HOLDING | CLOSE_DIFF@170.10 profit=-1653.81 |
| 217,570 | 181.14 | WAIT_DIP | →EVAL(move_settle) high=175.88 force=44.68 → WAIT_DIP(U1完成 high=175.88) |
| 217,784 | 183.50 | HOLDING | DIP_REBUY@183.50 不跌破前低(down_low=172.41 >= prev_low=131.20) |
| 218,565 | 172.82 | HOLDING | TRIM@172.82 shares=699.0 |
| 219,307 | 163.65 | WAIT_ENTRY | →EVAL(move_settle) high=193.82 force=33.57 → EXIT(盘整背驰 force 33.57<44.68) |
</details>

## HK700

- 数据：**1,408,882** bars (1min)
- 价格：11.36 → 458.20
- Buy-and-hold: **+3933.45%**
- 回测耗时：523.0s

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
| 交易数 | 19（纯多头） |
| 胜率 | 63.2% |
| 平均收益 | +7.673% |
| 复利累计 | **+224.07%** |
| 最大回撤 | -28.62% |
| 平均持仓 | 14,312 bars |
| 有加仓的交易 | 11/19 |
| 有降成本的交易 | 14/19 |
| 达到本金回收 | 0/19 |
| 有段间比较的交易 | 9/19 |

### 交易明细

| # | 入场 | 出场 | 持仓bars | PnL% | 加仓 | 短差 | 向上段数 | 退出原因 | 状态路径 |
|---|------|------|---------|------|------|------|---------|---------|---------|
| 1 | 50.50@289429 | 80.70@375182 | 85,753 | +76.77 | 2 | 9 | 5 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 2 | 87.50@378892 | 98.20@391198 | 12,306 | +14.93 | 0 | 5 | 2 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 3 | 115.80@396226 | 90.50@409439 | 13,213 | -18.10 | 1 | 0 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 4 | 119.10@428315 | 111.25@438789 | 10,474 | -6.27 | 1 | 2 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 5 | 109.60@441287 | 105.25@443385 | 2,098 | -0.18 | 1 | 1 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 6 | 119.55@453755 | 109.95@457850 | 4,095 | -6.84 | 1 | 0 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 7 | 124.65@468634 | 149.35@492712 | 24,078 | +20.92 | 3 | 8 | 5 | no_new_high | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 8 | 158.80@576774 | 164.90@584519 | 7,745 | +5.41 | 0 | 2 | 2 | no_new_high | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 9 | 163.50@584883 | 175.95@620707 | 35,824 | +7.80 | 0 | 7 | 3 | no_new_high | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 10 | 167.85@623892 | 167.75@624082 | 190 | -0.06 | 0 | 0 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 11 | 222.70@652086 | 251.60@667566 | 15,480 | +14.68 | 0 | 4 | 2 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 12 | 265.70@670484 | 263.40@670541 | 57 | -0.87 | 0 | 0 | 0 | bsp_invalidate | ENTRY→HOLDING |
| 13 | 276.90@672228 | 299.90@682404 | 10,176 | +7.68 | 1 | 3 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 14 | 312.90@685236 | 351.40@704079 | 18,843 | +11.23 | 1 | 6 | 3 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 15 | 403.40@710944 | 422.00@717550 | 6,606 | +4.61 | 0 | 2 | 2 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 16 | 487.60@917775 | 501.20@924223 | 6,448 | +2.53 | 1 | 4 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 17 | 552.80@944408 | 540.80@954533 | 10,125 | -1.58 | 1 | 2 | 1 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 18 | 583.20@962508 | 659.60@970887 | 8,379 | +12.46 | 1 | 4 | 2 | consolidation_divergence | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 19 | 629.60@972120 | 633.80@972151 | 31 | +0.67 | 0 | 0 | 0 | bsp_invalidate | ENTRY→HOLDING |

### FSM 状态分布（按 bar 数）

| 状态 | Bars | 占比 |
|------|------|------|
| WAIT_ENTRY | 1,136,942 | 80.7% |
| ENTRY | 19 | 0.0% |
| HOLDING | 224,227 | 15.9% |
| EVAL | 0 | 0.0% |
| WAIT_DIP | 47,694 | 3.4% |
| OBSERVE | 0 | 0.0% |
| EXIT | 0 | 0.0% |

<details><summary>事件流（244条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 289,428 | 50.50 | ENTRY | WAIT→ENTRY BSP={2113800284} |
| 289,429 | 50.50 | HOLDING | ENTRY_LONG@50.50 L2dir=1 |
| 291,912 | 49.70 | HOLDING | TRIM@49.70 shares=1958.0 |
| 302,478 | 45.90 | HOLDING | CLOSE_DIFF@45.90 profit=7440.39 |
| 307,353 | 49.60 | WAIT_DIP | →EVAL(move_settle) high=51.70 force=6.90 → WAIT_DIP(U1完成 high=51.70) |
| 316,533 | 51.50 | HOLDING | DIP_REBUY@51.50 不跌破前低(down_low=48.40 >= prev_low=44.80) |
| 320,109 | 48.50 | HOLDING | TRIM@48.50 shares=1911.6 |
| 324,400 | 46.20 | WAIT_DIP | →EVAL(move_settle) high=52.70 force=8.60 → WAIT_DIP(创新高无背驰 high=52.70) |
| 326,792 | 46.70 | HOLDING | DIP_REBUY@46.70 不跌破前低(down_low=44.90 >= prev_low=44.10) |
| 329,386 | 50.00 | HOLDING | ADD_POS@50.00 +443.1 L1ratio=0.188 |
| 331,116 | 50.30 | HOLDING | TRIM@50.30 shares=2699.4 |
| 334,063 | 55.10 | HOLDING | CLOSE_DIFF@55.10 profit=-12956.97 |
| 336,793 | 55.40 | HOLDING | TRIM@55.40 shares=2790.7 |
| 338,777 | 55.50 | HOLDING | CLOSE_DIFF@55.50 profit=-279.07 |
| 340,518 | 51.70 | HOLDING | TRIM@51.70 shares=2563.2 |
| 341,695 | 55.50 | HOLDING | CLOSE_DIFF@55.50 profit=-9740.12 |
| 341,796 | 57.10 | WAIT_DIP | →EVAL(move_settle) high=58.10 force=13.30 → WAIT_DIP(创新高无背驰 high=58.10) |
| 341,917 | 56.40 | HOLDING | DIP_REBUY@56.40 不跌破前低(down_low=56.00 >= prev_low=44.80) |
| 343,058 | 56.70 | HOLDING | TRIM@56.70 shares=2747.2 |
| 343,399 | 56.10 | HOLDING | CLOSE_DIFF@56.10 profit=1648.29 |
| 352,110 | 68.50 | HOLDING | TRIM@68.50 shares=2623.1 |
| 352,332 | 67.70 | HOLDING | CLOSE_DIFF@67.70 profit=2098.49 |
| 352,795 | 68.70 | HOLDING | TRIM@68.70 shares=2783.0 |
| 355,033 | 66.20 | HOLDING | CLOSE_DIFF@66.20 profit=6957.39 |
| 356,878 | 70.40 | WAIT_DIP | →EVAL(move_settle) high=70.10 force=20.40 → WAIT_DIP(创新高无背驰 high=70.10) |
| 356,973 | 71.10 | HOLDING | DIP_REBUY@71.10 不跌破前低(down_low=69.80 >= prev_low=49.70) |
| 364,770 | 76.90 | HOLDING | TRIM@76.90 shares=2761.4 |
| 367,580 | 78.70 | HOLDING | CLOSE_DIFF@78.70 profit=-4970.52 |
| 368,642 | 77.30 | HOLDING | TRIM@77.30 shares=2612.0 |
| 372,638 | 75.70 | HOLDING | CLOSE_DIFF@75.70 profit=4179.14 |
| 375,182 | 80.70 | WAIT_ENTRY | →EVAL(move_settle) high=83.90 force=19.20 → EXIT(盘整背驰 force 19.20<20.40) |
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
| 389,456 | 96.80 | WAIT_DIP | →EVAL(move_settle) high=98.80 force=27.30 → WAIT_DIP(U1完成 high=98.80) |
| 389,613 | 96.40 | HOLDING | DIP_REBUY@96.40 不跌破前低(down_low=94.00 >= prev_low=71.50) |
| 389,910 | 93.60 | HOLDING | TRIM@93.60 shares=1122.2 |
| 390,418 | 95.90 | HOLDING | CLOSE_DIFF@95.90 profit=-2581.03 |
| 391,198 | 98.20 | WAIT_ENTRY | →EVAL(move_settle) high=100.50 force=11.80 → EXIT(盘整背驰 force 11.80<27.30) |
| 396,225 | 116.00 | ENTRY | WAIT→ENTRY BSP={1723949316} |
| 396,226 | 115.80 | HOLDING | ENTRY_LONG@115.80 L2dir=1 |
| 398,194 | 110.60 | HOLDING | TRIM@110.60 shares=804.4 |
| 400,708 | 108.40 | WAIT_DIP | →EVAL(move_settle) high=119.00 force=26.80 → WAIT_DIP(U1完成 high=119.00) |
| 409,083 | 90.95 | HOLDING | DIP_REBUY_DIVERGE@90.95 跌破+盘整背驰 |
| 409,422 | 91.65 | HOLDING | ADD_POS@91.65 +189.3 L1ratio=0.219 |
| 409,439 | 90.50 | WAIT_ENTRY | BSP_INVALIDATE@90.50 |
| 428,314 | 119.10 | ENTRY | WAIT→ENTRY BSP={1685477336} |
| 428,315 | 119.10 | HOLDING | ENTRY_LONG@119.10 L2dir=1 |
| 429,469 | 118.80 | HOLDING | TRIM@118.80 shares=807.6 |
| 429,983 | 119.35 | HOLDING | CLOSE_DIFF@119.35 profit=-444.15 |
| 431,636 | 121.50 | HOLDING | TRIM@121.50 shares=819.3 |
| 432,017 | 123.05 | HOLDING | CLOSE_DIFF@123.05 profit=-1269.97 |
| 435,026 | 120.20 | HOLDING | TRIM@120.20 shares=820.0 |
| 436,751 | 117.35 | WAIT_DIP | →EVAL(move_settle) high=123.50 force=11.70 → WAIT_DIP(U1完成 high=123.50) |
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
| 457,173 | 109.70 | WAIT_DIP | →EVAL(move_settle) high=124.35 force=16.80 → WAIT_DIP(U1完成 high=124.35) |
| 457,634 | 109.15 | HOLDING | DIP_REBUY@109.15 不跌破前低(down_low=108.40 >= prev_low=107.55) |
| 457,850 | 109.95 | WAIT_ENTRY | BSP_INVALIDATE@109.95 |
| 468,633 | 124.45 | ENTRY | WAIT→ENTRY BSP={439129090} |
| 468,634 | 124.65 | HOLDING | ENTRY_LONG@124.65 L2dir=1 |
| 470,753 | 124.35 | HOLDING | TRIM@124.35 shares=786.2 |
| 471,012 | 124.05 | HOLDING | CLOSE_DIFF@124.05 profit=235.85 |
| 471,415 | 124.70 | HOLDING | TRIM@124.70 shares=798.1 |
| 471,599 | 124.55 | HOLDING | CLOSE_DIFF@124.55 profit=119.71 |
| 472,091 | 122.40 | HOLDING | TRIM@122.40 shares=798.1 |
| 474,856 | 124.05 | WAIT_DIP | →EVAL(move_settle) high=127.20 force=15.10 → WAIT_DIP(U1完成 high=127.20) |
| 475,334 | 124.70 | HOLDING | DIP_REBUY@124.70 不跌破前低(down_low=123.25 >= prev_low=112.10) |
| 477,069 | 123.35 | HOLDING | TRIM@123.35 shares=841.8 |
| 479,346 | 122.15 | HOLDING | CLOSE_DIFF@122.15 profit=1010.16 |
| 479,825 | 125.25 | WAIT_DIP | →EVAL(move_settle) high=127.20 force=9.20 → WAIT_DIP(创新高无背驰 high=127.20) |
| 480,282 | 125.35 | HOLDING | DIP_REBUY@125.35 不跌破前低(down_low=122.95 >= prev_low=118.00) |
| 481,043 | 133.40 | HOLDING | TRIM@133.40 shares=893.2 |
| 481,240 | 134.50 | HOLDING | CLOSE_DIFF@134.50 profit=-982.53 |
| 482,025 | 131.35 | HOLDING | TRIM@131.35 shares=843.7 |
| 482,914 | 134.60 | HOLDING | CLOSE_DIFF@134.60 profit=-2742.17 |
| 483,010 | 136.15 | WAIT_DIP | →EVAL(move_settle) high=137.15 force=17.15 → WAIT_DIP(创新高无背驰 high=137.15) |
| 483,438 | 135.85 | HOLDING | DIP_REBUY@135.85 不跌破前低(down_low=134.75 >= prev_low=120.00) |
| 485,735 | 145.20 | HOLDING | TRIM@145.20 shares=813.4 |
| 486,139 | 146.45 | HOLDING | CLOSE_DIFF@146.45 profit=-1016.79 |
| 486,777 | 145.75 | HOLDING | TRIM@145.75 shares=857.3 |
| 487,544 | 150.35 | WAIT_DIP | →EVAL(move_settle) high=157.65 force=28.30 → WAIT_DIP(创新高无背驰 high=157.65) |
| 487,689 | 150.90 | HOLDING | DIP_REBUY@150.90 不跌破前低(down_low=148.95 >= prev_low=129.35) |
| 488,643 | 149.60 | HOLDING | TRIM@149.60 shares=949.2 |
| 488,865 | 150.70 | HOLDING | CLOSE_DIFF@150.70 profit=-1044.09 |
| 489,342 | 149.50 | HOLDING | TRIM@149.50 shares=949.2 |
| 491,627 | 147.65 | HOLDING | CLOSE_DIFF@147.65 profit=1755.96 |
| 492,712 | 149.35 | WAIT_ENTRY | →EVAL(move_settle) high=152.65 force=12.10 → EXIT(不创新高 152.65<157.65) |
| 576,773 | 158.80 | ENTRY | WAIT→ENTRY BSP={1264064818} |
| 576,774 | 158.80 | HOLDING | ENTRY_LONG@158.80 L2dir=1 |
| 578,384 | 158.10 | HOLDING | TRIM@158.10 shares=626.7 |
| 578,760 | 161.05 | HOLDING | CLOSE_DIFF@161.05 profit=-1848.88 |
| 582,192 | 162.80 | WAIT_DIP | →EVAL(move_settle) high=164.25 force=17.85 → WAIT_DIP(U1完成 high=164.25) |
| 582,681 | 157.55 | HOLDING | DIP_REBUY@157.55 不跌破前低(down_low=156.70 >= prev_low=146.40) |
| 582,807 | 157.90 | HOLDING | TRIM@157.90 shares=628.4 |
| 583,315 | 155.40 | HOLDING | CLOSE_DIFF@155.40 profit=1570.91 |
| 584,519 | 164.90 | WAIT_ENTRY | →EVAL(move_settle) high=164.00 force=9.80 → EXIT(不创新高 164.00<164.25) |
| 584,882 | 163.45 | ENTRY | WAIT→ENTRY BSP={1361013528} |
| 584,883 | 163.50 | HOLDING | ENTRY_LONG@163.50 L2dir=1 |
| 585,065 | 160.30 | HOLDING | TRIM@160.30 shares=606.0 |
| 586,264 | 166.45 | HOLDING | CLOSE_DIFF@166.45 profit=-3727.01 |
| 587,565 | 170.45 | HOLDING | TRIM@170.45 shares=603.1 |
| 587,878 | 170.15 | HOLDING | CLOSE_DIFF@170.15 profit=180.94 |
| 589,223 | 170.90 | HOLDING | TRIM@170.90 shares=608.4 |
| 589,341 | 171.80 | HOLDING | CLOSE_DIFF@171.80 profit=-547.55 |
| 590,137 | 172.65 | HOLDING | TRIM@172.65 shares=602.6 |
| 591,175 | 171.10 | WAIT_DIP | →EVAL(move_settle) high=174.85 force=20.65 → WAIT_DIP(U1完成 high=174.85) |
| 592,669 | 173.40 | HOLDING | DIP_REBUY@173.40 不跌破前低(down_low=167.30 >= prev_low=154.20) |
| 593,284 | 172.45 | HOLDING | TRIM@172.45 shares=611.1 |
| 594,708 | 181.60 | HOLDING | CLOSE_DIFF@181.60 profit=-5591.49 |
| 595,001 | 185.45 | HOLDING | TRIM@185.45 shares=586.1 |
| 595,530 | 186.95 | HOLDING | CLOSE_DIFF@186.95 profit=-879.15 |
| 596,545 | 186.55 | HOLDING | TRIM@186.55 shares=588.5 |
| 598,844 | 187.10 | HOLDING | CLOSE_DIFF@187.10 profit=-323.69 |
| 600,982 | 194.10 | HOLDING | TRIM@194.10 shares=581.3 |
| 602,947 | 194.70 | HOLDING | CLOSE_DIFF@194.70 profit=-348.80 |
| 604,357 | 198.90 | WAIT_DIP | →EVAL(move_settle) high=199.80 force=32.30 → WAIT_DIP(创新高无背驰 high=199.80) |
| 619,998 | 176.05 | HOLDING | DIP_REBUY@176.05 不跌破前低(down_low=174.75 >= prev_low=167.50) |
| 620,092 | 175.95 | HOLDING | TRIM@175.95 shares=553.9 |
| 620,707 | 175.95 | WAIT_ENTRY | →EVAL(move_settle) high=193.20 force=16.05 → EXIT(不创新高 193.20<199.80) |
| 623,891 | 167.85 | ENTRY | WAIT→ENTRY BSP={1545513244} |
| 623,892 | 167.85 | HOLDING | ENTRY_LONG@167.85 L2dir=1 |
| 624,082 | 167.75 | WAIT_ENTRY | BSP_INVALIDATE@167.75 |
| 652,085 | 222.90 | ENTRY | WAIT→ENTRY BSP={1407391510} |
| 652,086 | 222.70 | HOLDING | ENTRY_LONG@222.70 L2dir=1 |
| 653,562 | 224.00 | HOLDING | TRIM@224.00 shares=444.3 |
| 654,774 | 232.70 | HOLDING | CLOSE_DIFF@232.70 profit=-3865.20 |
| 657,219 | 249.80 | HOLDING | TRIM@249.80 shares=439.6 |
| 658,091 | 250.50 | HOLDING | CLOSE_DIFF@250.50 profit=-307.75 |
| 660,628 | 251.10 | HOLDING | TRIM@251.10 shares=439.7 |
| 661,334 | 251.10 | HOLDING | CLOSE_DIFF@251.10 profit=0.00 |
| 662,667 | 252.60 | HOLDING | TRIM@252.60 shares=438.8 |
| 664,568 | 258.70 | WAIT_DIP | →EVAL(move_settle) high=261.20 force=95.55 → WAIT_DIP(U1完成 high=261.20) |
| 664,706 | 261.40 | HOLDING | DIP_REBUY@261.40 不跌破前低(down_low=255.90 >= prev_low=165.65) |
| ... | ... | ... | （共244条，显示前150） |
</details>

## 六版横向对比

| 版本 | QQQ | OKLO | HK700 |
|------|-----|------|-------|
| 1. 单层PH H组 | -19.8% | -99% | -84% |
| 2. 多级别PH（双向） | +58.3% | +72.3% | +222.8% |
| 3. Regime过滤器 | +58.3% | +23.7% | +222.8% |
| 4. 走势类型过滤 | +18% | -86% | +6% |
| 5. 纯做多 | +71.7% | +228% | +292.9% |
| 6. **完整版（本版）** | **+23.1%** | **+133.2%** | **+224.1%** |

## 汇总

| 标的 | Bars | BH% | 复利% | 胜率 | 交易数 | L2翻转 | 加仓 | 降成本 | PW | 段比较 | 耗时 |
|------|------|-----|------|------|--------|--------|------|--------|-----|--------|------|
| QQQ | 728,030 | +174.6 | +23.08 | 59% | 41 | 14 | 12/41 | 29/41 | 0/41 | 23/41 | 368s |
| OKLO | 333,613 | +251.3 | +133.23 | 42% | 12 | 7 | 8/12 | 9/12 | 0/12 | 5/12 | 72s |
| HK700 | 1,408,882 | +3933.5 | +224.07 | 63% | 19 | 5 | 11/19 | 14/19 | 0/19 | 9/19 | 523s |

## 结果包六要素

**结论**：完整版38课操作程式FSM在1分钟级别3标的上的表现。阶段一实现（纯做多+降成本+走势段比较）。

**定义依据**：
- 38课第36行：同级别分解操作程式逐句形式化
- S7: 不创新高判断 + 盘整背驰力度比较
- S5: 回调段不跌破前低/跌破+盘整背驰 判断
- 37课第22行 + 29课第52行：区间套精确定位
- 38课第30行：分区卷钱降成本

**边界条件**：
- PENDING_EXPIRY = 390 bars
- 力度比较使用价格振幅（无MACD），可能低估背驰
- 阶段一：纯做多，向下段空仓等待
- 无滑点/手续费建模

**下游推论**：
- 走势段比较 vs PH flip 退出的效果对比
- 若完整版 > 纯做多 → 段间比较改善了退出时机
- 若完整版 < 纯做多 → 段间比较引入了额外摩擦

**谱系引用**：
- 525号：组件局部完成性
- 526号：a0递归存在论区分
- 521号：PH纯拓扑无动量

**影响声明**：新建独立回测脚本和strategy模块，不修改引擎代码或旧版回测。

**认识论等级**：L2（真实数据，3标的 1min；可产生否定性结果）。