# 自适应多级别 PH 分层赋格 FSM 回测 — 1分钟数据

## 架构

5状态 FSM + 自适应 PH 分层（方向级别从数据涌现）：

| PH级别 | 输入 | 信号用途 |
|--------|------|---------|
| L0 | 1min close | 仅作为 PH 基础数据，不用于降成本 |
| L1 | L1线段端点 | 门控/降成本（取决于 E） |
| L(N+1) | 结构层级N走势端点 | 方向/门控/降成本（取决于 E） |

**信号路由规则**：设有效级别 E = 最高有充足数据的 PH 级别
- 方向裁决 = PH L(E) rank-1 settle
- 进场门控 = PH L(E-1) rank-1 settle + BSP
- 降成本 = PH L(max(E-2, 1)) non-rank-1 settle（下界 L1，L0 太噪）
- 有效阈值：≥10 次更新 + ≥1 次 rank-1 settle

## QQQ

- 数据：**728,030** bars (1min)
- 价格：268.82 → 738.28
- Buy-and-hold: **+174.64%**
- 回测耗时：375.3s
- **最终有效级别：L2**

### PH 分层统计

| PH级别 | 更新次数 | settle总数 | rank-1 settle | 有效？ |
|--------|---------|-----------|---------------|--------|
| L0 | 728,030 | 350,072 | 3962 | **是** |
| L1 | 6,119 | 5,804 | 286 | **是** |
| L2 | 389 | 190 | 25 | **是** |
| L3 | 29 | 8 | 0 | 否 |
| L4 | 1 | 0 | 0 | 否 |

### 有效级别涌现历史

| Bar | 有效级别 |
|-----|---------|
| 2,026 | L1 |
| 40,373 | L2 |

### 信号路由（最终：方向=L2, 门控=L1, 降成本=L0）

### 总体指标

| 指标 | 值 |
|------|-----|
| 交易数 | 51 (多30/空21) |
| 胜率 | 43.1% |
| 平均收益 | +0.714% |
| 复利累计 | +40.00% |
| 最大回撤 | -15.52% |
| 平均持仓 | 4,514 bars |
| 有加仓的交易 | 26/51 |
| 有降成本的交易 | 30/51 |
| 达到本金回收 | 0/51 |

### 交易明细

| # | 方向 | EffLv | 入场 | 出场 | 持仓bars | PnL% | 加仓 | 短差 | PW | 退出原因 |
|---|------|-------|------|------|---------|------|------|------|-----|---------|
| 1 | 多 | L0 | 266.47@943 | 266.33@959 | 16 | -0.05 | 0 | 0 |  | dir_flip_short_L0 |
| 2 | 空 | L1 | 261.48@2,330 | 268.88@3,113 | 783 | -3.15 | 0 | 1 |  | dir_flip_long_L1 |
| 3 | 多 | L1 | 270.73@3,483 | 268.55@4,276 | 793 | -0.81 | 0 | 0 |  | dir_flip_short_L1 |
| 4 | 多 | L1 | 275.44@5,285 | 278.84@8,559 | 3,274 | +1.23 | 0 | 3 |  | dir_flip_short_L1 |
| 5 | 空 | L1 | 273.96@9,275 | 282.47@10,566 | 1,291 | -3.11 | 0 | 0 |  | dir_flip_long_L1 |
| 6 | 多 | L1 | 292.45@13,661 | 292.11@15,035 | 1,374 | -0.12 | 0 | 1 |  | dir_flip_short_L1 |
| 7 | 空 | L1 | 288.56@15,516 | 302.99@17,001 | 1,485 | -5.21 | 0 | 1 |  | dir_flip_long_L1 |
| 8 | 多 | L1 | 308.99@17,540 | 302.90@19,018 | 1,478 | -1.97 | 0 | 1 |  | dir_flip_short_L1 |
| 9 | 空 | L1 | 302.31@19,270 | 304.36@19,423 | 153 | -0.68 | 0 | 0 |  | dir_flip_long_L1 |
| 10 | 空 | L1 | 300.15@22,243 | 303.12@24,201 | 1,958 | -0.56 | 1 | 1 |  | bsp_invalidate |
| 11 | 空 | L1 | 297.27@27,437 | 294.14@28,444 | 1,007 | +0.47 | 2 | 0 |  | bsp_invalidate |
| 12 | 空 | L1 | 287.37@39,206 | 292.56@39,435 | 229 | -1.81 | 0 | 0 |  | dir_flip_long_L1 |
| 13 | 多 | L2 | 312.05@45,958 | 309.74@46,025 | 67 | -0.74 | 0 | 0 |  | bsp_invalidate |
| 14 | 多 | L2 | 315.35@51,094 | 317.32@56,108 | 5,014 | +0.89 | 1 | 5 |  | bsp_invalidate |
| 15 | 多 | L2 | 322.86@68,749 | 318.33@72,362 | 3,613 | -0.95 | 1 | 3 |  | bsp_invalidate |
| 16 | 多 | L2 | 323.18@72,729 | 352.97@87,747 | 15,018 | +10.17 | 4 | 11 |  | bsp_invalidate |
| 17 | 多 | L2 | 362.52@93,452 | 365.80@97,876 | 4,424 | +1.84 | 1 | 5 |  | bsp_invalidate |
| 18 | 多 | L2 | 366.99@108,069 | 366.88@108,299 | 230 | -0.03 | 0 | 0 |  | bsp_invalidate |
| 19 | 多 | L2 | 372.83@109,083 | 376.68@115,656 | 6,573 | +2.83 | 1 | 5 |  | bsp_invalidate |
| 20 | 多 | L2 | 377.45@116,422 | 378.92@116,631 | 209 | +0.39 | 0 | 0 |  | bsp_invalidate |
| 21 | 多 | L2 | 374.80@122,746 | 373.54@124,079 | 1,333 | -0.30 | 1 | 1 |  | bsp_invalidate |
| 22 | 空 | L2 | 377.06@140,289 | 377.63@140,350 | 61 | -0.15 | 0 | 0 |  | bsp_invalidate |
| 23 | 空 | L2 | 357.75@152,154 | 358.59@152,394 | 240 | -0.23 | 0 | 0 |  | bsp_invalidate |
| 24 | 空 | L2 | 352.14@154,112 | 355.17@154,193 | 81 | -0.86 | 0 | 0 |  | bsp_invalidate |
| 25 | 空 | L2 | 370.00@163,511 | 370.02@163,554 | 43 | -0.01 | 0 | 0 |  | bsp_invalidate |
| 26 | 空 | L2 | 346.96@171,494 | 350.27@174,984 | 3,490 | -0.80 | 2 | 4 |  | bsp_invalidate |
| 27 | 多 | L2 | 390.83@187,637 | 388.55@192,317 | 4,680 | +1.11 | 1 | 5 |  | bsp_invalidate |
| 28 | 多 | L2 | 395.30@196,808 | 396.84@208,841 | 12,033 | +2.64 | 1 | 14 |  | bsp_invalidate |
| 29 | 多 | L2 | 411.31@216,302 | 419.92@223,983 | 7,681 | +3.79 | 1 | 9 |  | bsp_invalidate |
| 30 | 多 | L2 | 429.94@225,726 | 429.72@231,925 | 6,199 | +1.45 | 1 | 4 |  | bsp_invalidate |
| 31 | 多 | L2 | 438.65@236,852 | 439.85@243,498 | 6,646 | +1.81 | 1 | 6 |  | bsp_invalidate |
| 32 | 多 | L2 | 448.15@252,523 | 442.59@259,485 | 6,962 | +0.31 | 1 | 9 |  | bsp_invalidate |
| 33 | 空 | L2 | 420.78@269,115 | 422.16@269,250 | 135 | -0.33 | 0 | 0 |  | bsp_invalidate |
| 34 | 空 | L2 | 417.32@269,528 | 424.38@271,192 | 1,664 | -1.53 | 2 | 1 |  | bsp_invalidate |
| 35 | 多 | L2 | 463.70@296,067 | 481.63@308,780 | 12,713 | +3.36 | 5 | 13 |  | bsp_invalidate |
| 36 | 多 | L2 | 490.70@311,143 | 489.62@318,067 | 6,924 | +1.08 | 2 | 8 |  | bsp_invalidate |
| 37 | 多 | L2 | 480.51@320,903 | 478.84@320,944 | 41 | -0.35 | 0 | 0 |  | bsp_invalidate |
| 38 | 空 | L2 | 512.53@388,365 | 511.36@389,769 | 1,404 | +0.23 | 0 | 0 |  | dir_flip_long_L2 |
| 39 | 多 | L2 | 519.64@400,566 | 524.05@410,720 | 10,154 | +2.75 | 1 | 9 |  | bsp_invalidate |
| 40 | 多 | L2 | 522.16@410,838 | 518.50@410,920 | 82 | -0.70 | 0 | 0 |  | bsp_invalidate |
| 41 | 多 | L2 | 540.24@440,638 | 516.63@445,662 | 5,024 | -2.01 | 1 | 6 |  | bsp_invalidate |
| 42 | 空 | L2 | 492.69@449,415 | 475.76@454,025 | 4,610 | +0.37 | 5 | 6 |  | bsp_invalidate |
| 43 | 空 | L2 | 422.94@470,427 | 423.64@471,017 | 590 | -0.90 | 1 | 0 |  | bsp_invalidate |
| 44 | 多 | L2 | 543.03@520,731 | 565.00@543,673 | 22,942 | +5.38 | 1 | 22 |  | bsp_invalidate |
| 45 | 多 | L2 | 582.62@566,236 | 601.49@588,515 | 22,279 | +5.00 | 2 | 20 |  | bsp_invalidate |
| 46 | 多 | L2 | 613.85@594,336 | 624.25@601,887 | 7,551 | +2.44 | 1 | 5 |  | bsp_invalidate |
| 47 | 空 | L2 | 603.54@613,834 | 605.11@613,921 | 87 | -0.26 | 0 | 0 |  | bsp_invalidate |
| 48 | 空 | L2 | 613.60@616,344 | 618.60@616,621 | 277 | -0.81 | 0 | 0 |  | bsp_invalidate |
| 49 | 空 | L2 | 582.64@680,950 | 581.15@687,843 | 6,893 | -1.36 | 5 | 6 |  | bsp_invalidate |
| 50 | 空 | L2 | 608.50@694,356 | 611.99@694,502 | 146 | -0.57 | 0 | 0 |  | bsp_invalidate |
| 51 | 多 | L2 | 649.21@699,768 | 738.28@728,029 | 28,261 | +17.22 | 0 | 32 |  | eod_close |

### FSM 状态分布

| 状态 | Bars | 占比 |
|------|------|------|
| SCANNING | 497,815 | 68.4% |
| POSITION_OPEN | 146,225 | 20.1% |
| COST_REDUCING | 83,990 | 11.5% |
| PRINCIPAL_WITHDRAWN | 0 | 0.0% |
| STOPPED_OUT | 0 | 0.0% |

<details><summary>事件流（572条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 943 | 266.47 | POSITION_OPEN | ENTRY_LONG@266.47 eff=L0 gate=L0 |
| 959 | 266.33 | SCANNING | STOP:dir_flip_short_L0@266.33 pnl=-0.05% |
| 2,330 | 261.48 | POSITION_OPEN | ENTRY_SHORT@261.48 eff=L1 gate=L0 |
| 2,711 | 263.52 | COST_REDUCING | TRIM@263.52 shares=318.4 ratio=0.833 |
| 2,805 | 264.53 | POSITION_OPEN | CLOSE_DIFF@264.53 profit=321.59 |
| 2,899 | 266.69 | COST_REDUCING | TRIM@266.69 shares=307.3 ratio=0.803 |
| 3,113 | 268.88 | SCANNING | STOP:dir_flip_long_L1@268.88 pnl=-3.15% |
| 3,483 | 270.73 | POSITION_OPEN | ENTRY_LONG@270.73 eff=L1 gate=L0 |
| 4,098 | 270.50 | COST_REDUCING | TRIM@270.50 shares=258.3 ratio=0.699 |
| 4,276 | 268.55 | SCANNING | STOP:dir_flip_short_L1@268.55 pnl=-0.81% |
| 5,285 | 275.44 | POSITION_OPEN | ENTRY_LONG@275.44 eff=L1 gate=L0 |
| 6,003 | 277.71 | COST_REDUCING | TRIM@277.71 shares=317.7 ratio=0.875 |
| 6,239 | 279.25 | POSITION_OPEN | CLOSE_DIFF@279.25 profit=-489.26 |
| 6,717 | 277.20 | COST_REDUCING | TRIM@277.20 shares=317.8 ratio=0.875 |
| 6,819 | 277.51 | POSITION_OPEN | CLOSE_DIFF@277.51 profit=-98.53 |
| 8,024 | 281.60 | COST_REDUCING | TRIM@281.60 shares=341.0 ratio=0.939 |
| 8,318 | 282.83 | POSITION_OPEN | CLOSE_DIFF@282.83 profit=-419.38 |
| 8,559 | 278.84 | SCANNING | STOP:dir_flip_short_L1@278.84 pnl=+1.23% |
| 9,275 | 273.96 | POSITION_OPEN | ENTRY_SHORT@273.96 eff=L1 gate=L0 |
| 9,483 | 276.62 | COST_REDUCING | TRIM@276.62 shares=119.2 ratio=0.327 |
| 10,566 | 282.47 | SCANNING | STOP:dir_flip_long_L1@282.47 pnl=-3.11% |
| 13,661 | 292.45 | POSITION_OPEN | ENTRY_LONG@292.45 eff=L1 gate=L0 |
| 14,704 | 292.56 | COST_REDUCING | TRIM@292.56 shares=287.8 ratio=0.842 |
| 14,811 | 292.74 | POSITION_OPEN | CLOSE_DIFF@292.74 profit=-51.81 |
| 15,035 | 292.11 | SCANNING | STOP:dir_flip_short_L1@292.11 pnl=-0.12% |
| 15,516 | 288.56 | POSITION_OPEN | ENTRY_SHORT@288.56 eff=L1 gate=L0 |
| 15,797 | 291.27 | COST_REDUCING | TRIM@291.27 shares=87.6 ratio=0.253 |
| 16,769 | 293.70 | POSITION_OPEN | CLOSE_DIFF@293.70 profit=212.75 |
| 17,001 | 302.99 | SCANNING | STOP:dir_flip_long_L1@302.99 pnl=-5.21% |
| 17,540 | 308.99 | POSITION_OPEN | ENTRY_LONG@308.99 eff=L1 gate=L0 |
| 17,975 | 309.05 | COST_REDUCING | TRIM@309.05 shares=261.4 ratio=0.808 |
| 18,466 | 311.10 | POSITION_OPEN | CLOSE_DIFF@311.10 profit=-535.96 |
| 18,728 | 306.41 | COST_REDUCING | TRIM@306.41 shares=312.7 ratio=0.966 |
| 19,018 | 302.90 | SCANNING | STOP:dir_flip_short_L1@302.90 pnl=-1.97% |
| 19,270 | 302.31 | POSITION_OPEN | ENTRY_SHORT@302.31 eff=L1 gate=L0 |
| 19,423 | 304.36 | SCANNING | STOP:dir_flip_long_L1@304.36 pnl=-0.68% |
| 22,243 | 300.15 | POSITION_OPEN | ENTRY_SHORT@300.15 eff=L1 gate=L0 |
| 22,847 | 299.14 | COST_REDUCING | TRIM@299.14 shares=98.3 ratio=0.295 |
| 24,005 | 305.74 | POSITION_OPEN | CLOSE_DIFF@305.74 profit=648.46 |
| 24,201 | 303.12 | SCANNING | STOP:bsp_invalidate@303.12 pnl=-0.56% |
| 27,437 | 297.27 | POSITION_OPEN | ENTRY_SHORT@297.27 eff=L1 gate=L0 |
| 27,859 | 294.39 | POSITION_OPEN | ADD_POS@294.39 +239.1 ratio=0.711 |
| 28,261 | 294.76 | POSITION_OPEN | ADD_POS@294.76 +409.2 ratio=0.711 |
| 28,444 | 294.14 | SCANNING | STOP:bsp_invalidate@294.14 pnl=+0.47% |
| 39,206 | 287.37 | POSITION_OPEN | ENTRY_SHORT@287.37 eff=L1 gate=L0 |
| 39,435 | 292.56 | SCANNING | STOP:dir_flip_long_L1@292.56 pnl=-1.81% |
| 45,958 | 312.05 | POSITION_OPEN | ENTRY_LONG@312.05 eff=L2 gate=L1 |
| 46,025 | 309.74 | SCANNING | STOP:bsp_invalidate@309.74 pnl=-0.74% |
| 51,094 | 315.35 | POSITION_OPEN | ENTRY_LONG@315.35 eff=L2 gate=L1 |
| 51,694 | 315.66 | COST_REDUCING | TRIM@315.66 shares=315.4 ratio=0.995 |
| 52,243 | 321.03 | POSITION_OPEN | CLOSE_DIFF@321.03 profit=-1693.74 |
| 52,737 | 319.50 | COST_REDUCING | TRIM@319.50 shares=291.9 ratio=0.920 |
| 52,767 | 319.20 | POSITION_OPEN | CLOSE_DIFF@319.20 profit=87.56 |
| 52,924 | 318.36 | COST_REDUCING | TRIM@318.36 shares=291.9 ratio=0.920 |
| 53,510 | 321.52 | POSITION_OPEN | CLOSE_DIFF@321.52 profit=-922.32 |
| 53,767 | 319.65 | COST_REDUCING | TRIM@319.65 shares=311.9 ratio=0.984 |
| 54,238 | 318.54 | POSITION_OPEN | CLOSE_DIFF@318.54 profit=346.24 |
| 55,453 | 314.73 | COST_REDUCING | TRIM@314.73 shares=277.3 ratio=0.874 |
| 55,597 | 317.48 | POSITION_OPEN | CLOSE_DIFF@317.48 profit=-762.46 |
| 55,783 | 318.10 | POSITION_OPEN | ADD_POS@318.10 +47.2 ratio=0.149 |
| 56,108 | 317.32 | SCANNING | STOP:bsp_invalidate@317.32 pnl=+0.89% |
| 68,749 | 322.86 | POSITION_OPEN | ENTRY_LONG@322.86 eff=L2 gate=L1 |
| 68,893 | 322.36 | COST_REDUCING | TRIM@322.36 shares=306.1 ratio=0.988 |
| 69,320 | 322.04 | POSITION_OPEN | CLOSE_DIFF@322.04 profit=97.96 |
| 69,405 | 322.46 | COST_REDUCING | TRIM@322.46 shares=306.2 ratio=0.989 |
| 69,827 | 322.03 | POSITION_OPEN | CLOSE_DIFF@322.03 profit=131.67 |
| 71,205 | 316.80 | COST_REDUCING | TRIM@316.80 shares=302.6 ratio=0.977 |
| 71,802 | 317.60 | POSITION_OPEN | CLOSE_DIFF@317.60 profit=-242.11 |
| 72,003 | 316.08 | POSITION_OPEN | ADD_POS@316.08 +41.6 ratio=0.134 |
| 72,362 | 318.33 | SCANNING | STOP:bsp_invalidate@318.33 pnl=-0.95% |
| 72,729 | 323.18 | POSITION_OPEN | ENTRY_LONG@323.18 eff=L2 gate=L1 |
| 73,460 | 323.47 | COST_REDUCING | TRIM@323.47 shares=308.7 ratio=0.998 |
| 74,028 | 322.43 | POSITION_OPEN | CLOSE_DIFF@322.43 profit=321.09 |
| 74,074 | 321.91 | COST_REDUCING | TRIM@321.91 shares=307.4 ratio=0.993 |
| 74,926 | 324.00 | POSITION_OPEN | CLOSE_DIFF@324.00 profit=-642.43 |
| 75,607 | 325.21 | COST_REDUCING | TRIM@325.21 shares=300.6 ratio=0.971 |
| 75,993 | 326.14 | POSITION_OPEN | CLOSE_DIFF@326.14 profit=-279.56 |
| 76,441 | 325.48 | COST_REDUCING | TRIM@325.48 shares=308.2 ratio=0.996 |
| 77,031 | 325.95 | POSITION_OPEN | CLOSE_DIFF@325.95 profit=-144.86 |
| 77,328 | 325.96 | POSITION_OPEN | ADD_POS@325.96 +19.3 ratio=0.059 |
| 77,981 | 327.22 | COST_REDUCING | TRIM@327.22 shares=345.0 ratio=0.994 |
| 78,031 | 327.75 | POSITION_OPEN | CLOSE_DIFF@327.75 profit=-182.84 |
| 78,440 | 327.44 | COST_REDUCING | TRIM@327.44 shares=340.1 ratio=0.980 |
| 78,534 | 327.64 | POSITION_OPEN | CLOSE_DIFF@327.64 profit=-68.01 |
| 78,826 | 328.60 | COST_REDUCING | TRIM@328.60 shares=343.6 ratio=0.990 |
| 79,420 | 332.09 | POSITION_OPEN | CLOSE_DIFF@332.09 profit=-1199.26 |
| 80,484 | 336.69 | COST_REDUCING | TRIM@336.69 shares=340.3 ratio=0.981 |
| 81,188 | 336.32 | POSITION_OPEN | CLOSE_DIFF@336.32 profit=125.92 |
| 82,391 | 333.84 | COST_REDUCING | TRIM@333.84 shares=336.4 ratio=0.970 |
| 82,996 | 330.63 | POSITION_OPEN | CLOSE_DIFF@330.63 profit=1079.80 |
| 83,632 | 337.79 | POSITION_OPEN | ADD_POS@337.79 +38.7 ratio=0.111 |
| 85,792 | 351.01 | COST_REDUCING | TRIM@351.01 shares=365.0 ratio=0.947 |
| 86,834 | 347.42 | POSITION_OPEN | CLOSE_DIFF@347.42 profit=1310.45 |
| 87,198 | 348.05 | COST_REDUCING | TRIM@348.05 shares=355.4 ratio=0.922 |
| 87,236 | 347.99 | POSITION_OPEN | CLOSE_DIFF@347.99 profit=21.32 |
| 87,663 | 352.53 | POSITION_OPEN | ADD_POS@352.53 +30.9 ratio=0.080 |
| 87,747 | 352.97 | SCANNING | STOP:bsp_invalidate@352.97 pnl=+10.17% |
| 93,452 | 362.52 | POSITION_OPEN | ENTRY_LONG@362.52 eff=L2 gate=L1 |
| 93,739 | 361.71 | COST_REDUCING | TRIM@361.71 shares=257.5 ratio=0.933 |
| 93,922 | 363.24 | POSITION_OPEN | CLOSE_DIFF@363.24 profit=-393.95 |
| 94,011 | 362.72 | COST_REDUCING | TRIM@362.72 shares=273.5 ratio=0.991 |
| 94,978 | 366.34 | POSITION_OPEN | CLOSE_DIFF@366.34 profit=-989.95 |
| 95,575 | 368.10 | COST_REDUCING | TRIM@368.10 shares=274.1 ratio=0.994 |
| 95,754 | 370.23 | POSITION_OPEN | CLOSE_DIFF@370.23 profit=-583.86 |
| 95,904 | 369.83 | COST_REDUCING | TRIM@369.83 shares=264.7 ratio=0.960 |
| 96,071 | 370.83 | POSITION_OPEN | CLOSE_DIFF@370.83 profit=-264.71 |
| 96,272 | 371.02 | COST_REDUCING | TRIM@371.02 shares=263.6 ratio=0.956 |
| 97,689 | 366.89 | POSITION_OPEN | CLOSE_DIFF@366.89 profit=1088.76 |
| 97,876 | 365.80 | SCANNING | STOP:bsp_invalidate@365.80 pnl=+1.84% |
| 108,069 | 366.99 | POSITION_OPEN | ENTRY_LONG@366.99 eff=L2 gate=L1 |
| 108,299 | 366.88 | SCANNING | STOP:bsp_invalidate@366.88 pnl=-0.03% |
| 109,083 | 372.83 | POSITION_OPEN | ENTRY_LONG@372.83 eff=L2 gate=L1 |
| 110,916 | 380.20 | COST_REDUCING | TRIM@380.20 shares=261.9 ratio=0.976 |
| 111,605 | 380.26 | POSITION_OPEN | CLOSE_DIFF@380.26 profit=-15.71 |
| 112,060 | 382.55 | COST_REDUCING | TRIM@382.55 shares=267.3 ratio=0.996 |
| 112,096 | 382.10 | POSITION_OPEN | CLOSE_DIFF@382.10 profit=120.27 |
| 112,795 | 385.24 | COST_REDUCING | TRIM@385.24 shares=267.6 ratio=0.998 |
| 112,977 | 386.31 | POSITION_OPEN | CLOSE_DIFF@386.31 profit=-286.39 |
| 113,388 | 385.11 | COST_REDUCING | TRIM@385.11 shares=258.1 ratio=0.962 |
| 114,885 | 377.83 | POSITION_OPEN | CLOSE_DIFF@377.83 profit=1879.17 |
| ... | ... | ... | （共572条，显示前120） |
</details>

## OKLO

- 数据：**333,613** bars (1min)
- 价格：18.07 → 63.48
- Buy-and-hold: **+251.30%**
- 回测耗时：75.0s
- **最终有效级别：L2**

### PH 分层统计

| PH级别 | 更新次数 | settle总数 | rank-1 settle | 有效？ |
|--------|---------|-----------|---------------|--------|
| L0 | 333,613 | 154,247 | 730 | **是** |
| L1 | 2,806 | 2,657 | 102 | **是** |
| L2 | 185 | 101 | 12 | **是** |
| L3 | 15 | 3 | 0 | 否 |
| L4 | 1 | 0 | 0 | 否 |

### 有效级别涌现历史

| Bar | 有效级别 |
|-----|---------|
| 1,889 | L1 |
| 39,674 | L2 |

### 信号路由（最终：方向=L2, 门控=L1, 降成本=L0）

### 总体指标

| 指标 | 值 |
|------|-----|
| 交易数 | 17 (多11/空6) |
| 胜率 | 29.4% |
| 平均收益 | +1.616% |
| 复利累计 | -13.96% |
| 最大回撤 | -57.48% |
| 平均持仓 | 3,250 bars |
| 有加仓的交易 | 15/17 |
| 有降成本的交易 | 13/17 |
| 达到本金回收 | 0/17 |

### 交易明细

| # | 方向 | EffLv | 入场 | 出场 | 持仓bars | PnL% | 加仓 | 短差 | PW | 退出原因 |
|---|------|-------|------|------|---------|------|------|------|-----|---------|
| 1 | 空 | L1 | 7.12@4,025 | 9.06@7,214 | 3,189 | -53.03 | 1 | 2 |  | bsp_invalidate |
| 2 | 空 | L1 | 5.90@29,391 | 6.11@31,396 | 2,005 | -3.47 | 0 | 1 |  | dir_flip_long_L1 |
| 3 | 多 | L1 | 7.00@32,925 | 8.20@34,390 | 1,465 | -3.32 | 3 | 0 |  | bsp_invalidate |
| 4 | 多 | L2 | 11.94@40,669 | 11.61@40,921 | 252 | -2.99 | 1 | 0 |  | bsp_invalidate |
| 5 | 多 | L2 | 14.64@41,222 | 19.11@46,795 | 5,573 | +26.86 | 1 | 5 |  | bsp_invalidate |
| 6 | 多 | L2 | 23.84@47,125 | 20.33@50,552 | 3,427 | -2.37 | 1 | 3 |  | bsp_invalidate |
| 7 | 多 | L2 | 21.53@69,911 | 18.81@70,448 | 537 | -12.63 | 0 | 0 |  | dir_flip_short_L2 |
| 8 | 多 | L2 | 30.89@82,091 | 54.69@94,200 | 12,109 | +52.53 | 5 | 9 |  | bsp_invalidate |
| 9 | 多 | L2 | 69.12@152,818 | 58.43@155,767 | 2,949 | -0.94 | 1 | 3 |  | bsp_invalidate |
| 10 | 多 | L2 | 77.39@174,756 | 76.68@178,964 | 4,208 | +0.21 | 3 | 4 |  | bsp_invalidate |
| 11 | 多 | L2 | 89.75@199,564 | 109.95@207,459 | 7,895 | +34.38 | 1 | 7 |  | bsp_invalidate |
| 12 | 多 | L2 | 144.43@212,217 | 131.60@213,994 | 1,777 | -3.71 | 1 | 2 |  | bsp_invalidate |
| 13 | 多 | L2 | 167.77@216,517 | 168.62@220,471 | 3,954 | +8.94 | 1 | 3 |  | bsp_invalidate |
| 14 | 空 | L2 | 100.00@235,613 | 104.29@236,068 | 455 | -4.54 | 1 | 0 |  | bsp_invalidate |
| 15 | 空 | L2 | 86.75@239,467 | 85.61@241,588 | 2,121 | -1.28 | 1 | 2 |  | bsp_invalidate |
| 16 | 空 | L2 | 78.10@252,103 | 81.25@253,761 | 1,658 | -4.48 | 1 | 1 |  | bsp_invalidate |
| 17 | 空 | L2 | 50.27@298,464 | 49.78@300,135 | 1,671 | -2.65 | 2 | 2 |  | bsp_invalidate |

### FSM 状态分布

| 状态 | Bars | 占比 |
|------|------|------|
| SCANNING | 278,368 | 83.4% |
| POSITION_OPEN | 29,237 | 8.8% |
| COST_REDUCING | 26,008 | 7.8% |
| PRINCIPAL_WITHDRAWN | 0 | 0.0% |
| STOPPED_OUT | 0 | 0.0% |

<details><summary>事件流（140条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 4,025 | 7.12 | POSITION_OPEN | ENTRY_SHORT@7.12 eff=L1 gate=L0 |
| 4,581 | 7.54 | COST_REDUCING | TRIM@7.54 shares=13072.7 ratio=0.931 |
| 6,177 | 11.16 | POSITION_OPEN | CLOSE_DIFF@11.16 profit=47323.14 |
| 6,343 | 11.15 | COST_REDUCING | TRIM@11.15 shares=13072.7 ratio=0.931 |
| 6,637 | 10.41 | POSITION_OPEN | CLOSE_DIFF@10.41 profit=-9673.79 |
| 7,192 | 9.18 | POSITION_OPEN | ADD_POS@9.18 +5529.0 ratio=0.394 |
| 7,214 | 9.06 | SCANNING | STOP:bsp_invalidate@9.06 pnl=-53.03% |
| 29,391 | 5.90 | POSITION_OPEN | ENTRY_SHORT@5.90 eff=L1 gate=L0 |
| 30,129 | 5.83 | COST_REDUCING | TRIM@5.83 shares=16441.1 ratio=0.970 |
| 30,579 | 5.60 | POSITION_OPEN | CLOSE_DIFF@5.60 profit=-3863.65 |
| 31,265 | 6.00 | COST_REDUCING | TRIM@6.00 shares=16441.1 ratio=0.970 |
| 31,396 | 6.11 | SCANNING | STOP:dir_flip_long_L1@6.11 pnl=-3.47% |
| 32,925 | 7.00 | POSITION_OPEN | ENTRY_LONG@7.00 eff=L1 gate=L0 |
| 33,331 | 8.14 | POSITION_OPEN | ADD_POS@8.14 +12678.4 ratio=0.887 |
| 34,067 | 8.86 | POSITION_OPEN | ADD_POS@8.86 +23930.4 ratio=0.887 |
| 34,279 | 8.74 | POSITION_OPEN | ADD_POS@8.74 +45168.5 ratio=0.887 |
| 34,390 | 8.20 | SCANNING | STOP:bsp_invalidate@8.20 pnl=-3.32% |
| 40,669 | 11.94 | POSITION_OPEN | ENTRY_LONG@11.94 eff=L2 gate=L1 |
| 40,798 | 12.00 | POSITION_OPEN | ADD_POS@12.00 +6918.4 ratio=0.826 |
| 40,921 | 11.61 | SCANNING | STOP:bsp_invalidate@11.61 pnl=-2.99% |
| 41,222 | 14.64 | POSITION_OPEN | ENTRY_LONG@14.64 eff=L2 gate=L1 |
| 41,862 | 16.22 | COST_REDUCING | TRIM@16.22 shares=6151.0 ratio=0.901 |
| 42,565 | 15.84 | POSITION_OPEN | CLOSE_DIFF@15.84 profit=2337.38 |
| 43,016 | 18.46 | COST_REDUCING | TRIM@18.46 shares=6091.1 ratio=0.892 |
| 43,450 | 19.91 | POSITION_OPEN | CLOSE_DIFF@19.91 profit=-8832.11 |
| 44,516 | 20.63 | COST_REDUCING | TRIM@20.63 shares=6291.7 ratio=0.921 |
| 44,992 | 20.44 | POSITION_OPEN | CLOSE_DIFF@20.44 profit=1195.41 |
| 45,375 | 18.33 | COST_REDUCING | TRIM@18.33 shares=5889.2 ratio=0.862 |
| 45,777 | 18.12 | POSITION_OPEN | CLOSE_DIFF@18.12 profit=1236.73 |
| 45,878 | 17.97 | COST_REDUCING | TRIM@17.97 shares=5889.2 ratio=0.862 |
| 46,744 | 19.14 | POSITION_OPEN | CLOSE_DIFF@19.14 profit=-6890.35 |
| 46,795 | 19.11 | SCANNING | STOP:bsp_invalidate@19.11 pnl=+26.86% |
| 47,125 | 23.84 | POSITION_OPEN | ENTRY_LONG@23.84 eff=L2 gate=L1 |
| 47,960 | 23.43 | COST_REDUCING | TRIM@23.43 shares=4044.3 ratio=0.964 |
| 48,481 | 26.75 | POSITION_OPEN | CLOSE_DIFF@26.75 profit=-13427.05 |
| 48,715 | 23.60 | COST_REDUCING | TRIM@23.60 shares=3727.9 ratio=0.889 |
| 49,561 | 22.99 | POSITION_OPEN | CLOSE_DIFF@22.99 profit=2274.04 |
| 49,609 | 22.92 | COST_REDUCING | TRIM@22.92 shares=3727.9 ratio=0.889 |
| 50,507 | 20.36 | POSITION_OPEN | CLOSE_DIFF@20.36 profit=9543.51 |
| 50,552 | 20.33 | SCANNING | STOP:bsp_invalidate@20.33 pnl=-2.37% |
| 69,911 | 21.53 | POSITION_OPEN | ENTRY_LONG@21.53 eff=L2 gate=L1 |
| 70,284 | 19.12 | COST_REDUCING | TRIM@19.12 shares=3636.1 ratio=0.783 |
| 70,448 | 18.81 | SCANNING | STOP:dir_flip_short_L2@18.81 pnl=-12.63% |
| 82,091 | 30.89 | POSITION_OPEN | ENTRY_LONG@30.89 eff=L2 gate=L1 |
| 83,207 | 32.37 | COST_REDUCING | TRIM@32.37 shares=2857.9 ratio=0.883 |
| 83,593 | 38.78 | POSITION_OPEN | CLOSE_DIFF@38.78 profit=-18319.29 |
| 84,429 | 42.84 | COST_REDUCING | TRIM@42.84 shares=2801.5 ratio=0.865 |
| 84,463 | 42.52 | POSITION_OPEN | CLOSE_DIFF@42.52 profit=896.47 |
| 84,755 | 35.90 | COST_REDUCING | TRIM@35.90 shares=2536.1 ratio=0.783 |
| 86,173 | 33.53 | POSITION_OPEN | CLOSE_DIFF@33.53 profit=6010.51 |
| 86,342 | 34.20 | POSITION_OPEN | ADD_POS@34.20 +1117.3 ratio=0.345 |
| 87,232 | 36.11 | POSITION_OPEN | ADD_POS@36.11 +1502.9 ratio=0.345 |
| 87,675 | 41.59 | POSITION_OPEN | ADD_POS@41.59 +2021.6 ratio=0.345 |
| 88,040 | 41.95 | COST_REDUCING | TRIM@41.95 shares=7803.6 ratio=0.990 |
| 88,940 | 43.40 | POSITION_OPEN | CLOSE_DIFF@43.40 profit=-11354.23 |
| 89,466 | 46.55 | COST_REDUCING | TRIM@46.55 shares=7776.1 ratio=0.987 |
| 90,008 | 45.95 | POSITION_OPEN | CLOSE_DIFF@45.95 profit=4665.64 |
| 91,119 | 48.95 | COST_REDUCING | TRIM@48.95 shares=7048.8 ratio=0.895 |
| 91,475 | 48.63 | POSITION_OPEN | CLOSE_DIFF@48.63 profit=2220.38 |
| 92,285 | 54.89 | COST_REDUCING | TRIM@54.89 shares=7644.1 ratio=0.970 |
| 92,312 | 55.20 | POSITION_OPEN | CLOSE_DIFF@55.20 profit=-2369.67 |
| 92,466 | 54.30 | COST_REDUCING | TRIM@54.30 shares=7644.1 ratio=0.970 |
| 93,016 | 52.38 | POSITION_OPEN | CLOSE_DIFF@52.38 profit=14638.46 |
| 93,454 | 50.88 | COST_REDUCING | TRIM@50.88 shares=7147.0 ratio=0.907 |
| 93,772 | 52.73 | POSITION_OPEN | CLOSE_DIFF@52.73 profit=-13186.30 |
| 93,903 | 54.91 | POSITION_OPEN | ADD_POS@54.91 +1548.4 ratio=0.197 |
| 94,086 | 54.37 | POSITION_OPEN | ADD_POS@54.37 +1852.7 ratio=0.197 |
| 94,200 | 54.69 | SCANNING | STOP:bsp_invalidate@54.69 pnl=+52.53% |
| 152,818 | 69.12 | POSITION_OPEN | ENTRY_LONG@69.12 eff=L2 gate=L1 |
| 153,037 | 68.04 | COST_REDUCING | TRIM@68.04 shares=1446.5 ratio=1.000 |
| 153,404 | 66.72 | POSITION_OPEN | CLOSE_DIFF@66.72 profit=1909.43 |
| 153,692 | 67.99 | COST_REDUCING | TRIM@67.99 shares=1446.5 ratio=1.000 |
| 155,024 | 62.20 | POSITION_OPEN | CLOSE_DIFF@62.20 profit=8375.46 |
| 155,447 | 62.12 | COST_REDUCING | TRIM@62.12 shares=1174.7 ratio=0.812 |
| 155,721 | 58.60 | POSITION_OPEN | CLOSE_DIFF@58.60 profit=4135.01 |
| 155,767 | 58.43 | SCANNING | STOP:bsp_invalidate@58.43 pnl=-0.94% |
| 174,756 | 77.39 | POSITION_OPEN | ENTRY_LONG@77.39 eff=L2 gate=L1 |
| 175,060 | 74.46 | COST_REDUCING | TRIM@74.46 shares=1254.8 ratio=0.971 |
| 175,622 | 75.20 | POSITION_OPEN | CLOSE_DIFF@75.20 profit=-928.58 |
| 176,091 | 71.38 | COST_REDUCING | TRIM@71.38 shares=1208.7 ratio=0.935 |
| 176,122 | 71.57 | POSITION_OPEN | CLOSE_DIFF@71.57 profit=-229.65 |
| 176,234 | 71.40 | COST_REDUCING | TRIM@71.40 shares=1208.7 ratio=0.935 |
| 176,398 | 75.80 | POSITION_OPEN | CLOSE_DIFF@75.80 profit=-5312.09 |
| 176,627 | 75.22 | POSITION_OPEN | ADD_POS@75.22 +140.3 ratio=0.109 |
| 176,907 | 77.59 | POSITION_OPEN | ADD_POS@77.59 +155.5 ratio=0.109 |
| 177,024 | 77.30 | COST_REDUCING | TRIM@77.30 shares=1583.0 ratio=0.997 |
| 178,793 | 76.59 | POSITION_OPEN | CLOSE_DIFF@76.59 profit=1123.95 |
| 178,964 | 76.68 | SCANNING | STOP:bsp_invalidate@76.68 pnl=+0.21% |
| 199,564 | 89.75 | POSITION_OPEN | ENTRY_LONG@89.75 eff=L2 gate=L1 |
| 200,406 | 91.20 | COST_REDUCING | TRIM@91.20 shares=1052.7 ratio=0.945 |
| 200,579 | 94.58 | POSITION_OPEN | CLOSE_DIFF@94.58 profit=-3558.25 |
| 201,126 | 94.68 | COST_REDUCING | TRIM@94.68 shares=1113.5 ratio=0.999 |
| 201,832 | 100.44 | POSITION_OPEN | CLOSE_DIFF@100.44 profit=-6413.59 |
| 203,456 | 131.41 | COST_REDUCING | TRIM@131.41 shares=1078.0 ratio=0.968 |
| 203,886 | 138.20 | POSITION_OPEN | CLOSE_DIFF@138.20 profit=-7319.65 |
| 204,334 | 137.28 | COST_REDUCING | TRIM@137.28 shares=1068.7 ratio=0.959 |
| 205,165 | 140.50 | POSITION_OPEN | CLOSE_DIFF@140.50 profit=-3441.19 |
| 205,261 | 136.24 | COST_REDUCING | TRIM@136.24 shares=1103.0 ratio=0.990 |
| 205,442 | 138.40 | POSITION_OPEN | CLOSE_DIFF@138.40 profit=-2382.40 |
| 205,518 | 136.88 | COST_REDUCING | TRIM@136.88 shares=1103.0 ratio=0.990 |
| 207,032 | 118.36 | POSITION_OPEN | CLOSE_DIFF@118.36 profit=20426.87 |
| 207,189 | 109.70 | COST_REDUCING | TRIM@109.70 shares=938.3 ratio=0.842 |
| 207,376 | 109.15 | POSITION_OPEN | CLOSE_DIFF@109.15 profit=516.08 |
| 207,413 | 110.77 | COST_REDUCING | ADD_POS@110.77 +286.5 ratio=0.257 |
| 207,459 | 109.95 | SCANNING | STOP:bsp_invalidate@109.95 pnl=+34.38% |
| 212,217 | 144.43 | POSITION_OPEN | ENTRY_LONG@144.43 eff=L2 gate=L1 |
| 212,526 | 143.05 | COST_REDUCING | TRIM@143.05 shares=688.7 ratio=0.995 |
| 213,422 | 136.43 | POSITION_OPEN | CLOSE_DIFF@136.43 profit=4559.05 |
| 213,823 | 134.18 | COST_REDUCING | TRIM@134.18 shares=688.7 ratio=0.995 |
| 213,849 | 133.56 | POSITION_OPEN | CLOSE_DIFF@133.56 profit=426.98 |
| 213,994 | 131.60 | SCANNING | STOP:bsp_invalidate@131.60 pnl=-3.71% |
| 216,517 | 167.77 | POSITION_OPEN | ENTRY_LONG@167.77 eff=L2 gate=L1 |
| 217,096 | 167.76 | COST_REDUCING | TRIM@167.76 shares=545.3 ratio=0.915 |
| 217,321 | 170.10 | POSITION_OPEN | CLOSE_DIFF@170.10 profit=-1275.99 |
| 218,565 | 172.82 | COST_REDUCING | TRIM@172.82 shares=539.3 ratio=0.905 |
| 219,699 | 154.48 | POSITION_OPEN | CLOSE_DIFF@154.48 profit=9890.30 |
| 220,103 | 161.23 | COST_REDUCING | TRIM@161.23 shares=525.8 ratio=0.882 |
| 220,302 | 163.65 | POSITION_OPEN | CLOSE_DIFF@163.65 profit=-1272.34 |
| 220,471 | 168.62 | SCANNING | STOP:bsp_invalidate@168.62 pnl=+8.94% |
| 235,613 | 100.00 | POSITION_OPEN | ENTRY_SHORT@100.00 eff=L2 gate=L1 |
| ... | ... | ... | （共140条，显示前120） |
</details>

## HK700

- 数据：**1,408,882** bars (1min)
- 价格：11.36 → 458.20
- Buy-and-hold: **+3933.45%**
- 回测耗时：544.8s
- **最终有效级别：L3**

### PH 分层统计

| PH级别 | 更新次数 | settle总数 | rank-1 settle | 有效？ |
|--------|---------|-----------|---------------|--------|
| L0 | 1,408,882 | 463,934 | 2312 | **是** |
| L1 | 5,014 | 4,798 | 184 | **是** |
| L2 | 298 | 141 | 11 | **是** |
| L3 | 26 | 11 | 1 | **是** |
| L4 | 2 | 0 | 0 | 否 |

### 有效级别涌现历史

| Bar | 有效级别 |
|-----|---------|
| 9,349 | L1 |
| 206,599 | L2 |
| 1,163,139 | L3 |

### 信号路由（最终：方向=L3, 门控=L2, 降成本=L1）

### 总体指标

| 指标 | 值 |
|------|-----|
| 交易数 | 28 (多17/空11) |
| 胜率 | 42.9% |
| 平均收益 | +5.198% |
| 复利累计 | +198.60% |
| 最大回撤 | -23.99% |
| 平均持仓 | 11,767 bars |
| 有加仓的交易 | 15/28 |
| 有降成本的交易 | 15/28 |
| 达到本金回收 | 1/28 |

### 交易明细

| # | 方向 | EffLv | 入场 | 出场 | 持仓bars | PnL% | 加仓 | 短差 | PW | 退出原因 |
|---|------|-------|------|------|---------|------|------|------|-----|---------|
| 1 | 空 | L1 | 10.50@14,097 | 9.86@22,598 | 8,501 | +6.10 | 0 | 3 |  | dir_flip_long_L1 |
| 2 | 多 | L1 | 13.82@53,073 | 16.67@63,471 | 10,398 | +22.52 | 1 | 4 |  | bsp_invalidate |
| 3 | 多 | L1 | 17.66@64,573 | 20.65@69,802 | 5,229 | +18.62 | 0 | 3 |  | dir_flip_short_L1 |
| 4 | 多 | L1 | 31.30@92,437 | 29.35@95,155 | 2,718 | -6.23 | 0 | 1 |  | dir_flip_short_L1 |
| 5 | 空 | L1 | 27.35@95,553 | 26.95@96,725 | 1,172 | -2.09 | 2 | 0 |  | bsp_invalidate |
| 6 | 多 | L1 | 34.30@138,285 | 33.30@142,225 | 3,940 | -2.92 | 0 | 0 |  | dir_flip_short_L1 |
| 7 | 空 | L1 | 34.90@158,103 | 36.05@158,322 | 219 | -3.30 | 0 | 0 |  | bsp_invalidate |
| 8 | 多 | L1 | 40.80@162,024 | 36.65@164,555 | 2,531 | -3.01 | 0 | 1 |  | dir_flip_short_L1 |
| 9 | 空 | L2 | 28.40@214,601 | 28.55@214,609 | 8 | -0.53 | 0 | 0 |  | bsp_invalidate |
| 10 | 多 | L2 | 41.90@257,942 | 82.80@377,587 | 119,645 | +85.09 | 6 | 19 |  | bsp_invalidate |
| 11 | 多 | L2 | 96.80@389,456 | 94.80@389,683 | 227 | -2.07 | 0 | 0 |  | bsp_invalidate |
| 12 | 多 | L2 | 119.20@428,960 | 111.25@438,789 | 9,829 | -5.00 | 1 | 3 |  | bsp_invalidate |
| 13 | 多 | L2 | 121.10@468,385 | 140.55@502,832 | 34,447 | +16.16 | 6 | 15 |  | bsp_invalidate |
| 14 | 多 | L2 | 158.20@576,743 | 167.75@624,082 | 47,339 | +8.44 | 1 | 15 |  | bsp_invalidate |
| 15 | 多 | L2 | 174.95@626,232 | 175.25@626,469 | 237 | +0.17 | 0 | 0 |  | bsp_invalidate |
| 16 | 多 | L2 | 204.30@644,871 | 263.40@670,541 | 25,670 | +35.92 | 1 | 10 |  | bsp_invalidate |
| 17 | 多 | L2 | 304.40@681,887 | 299.90@682,404 | 517 | -1.48 | 0 | 0 |  | bsp_invalidate |
| 18 | 多 | L2 | 345.10@696,860 | 351.40@704,079 | 7,219 | +4.01 | 1 | 3 |  | bsp_invalidate |
| 19 | 多 | L2 | 412.40@720,668 | 418.10@721,699 | 1,031 | +1.23 | 1 | 0 |  | bsp_invalidate |
| 20 | 空 | L2 | 299.90@761,634 | 318.90@762,257 | 623 | -4.85 | 1 | 0 |  | bsp_invalidate |
| 21 | 空 | L2 | 305.10@765,690 | 290.40@785,965 | 20,275 | -10.60 | 7 | 9 | 是 | bsp_invalidate |
| 22 | 空 | L2 | 308.10@802,444 | 313.30@803,037 | 593 | -1.69 | 0 | 0 |  | bsp_invalidate |
| 23 | 多 | L2 | 514.00@941,819 | 540.80@954,533 | 12,714 | +5.09 | 1 | 5 |  | bsp_invalidate |
| 24 | 多 | L2 | 533.40@959,571 | 531.00@959,754 | 183 | -0.45 | 0 | 0 |  | bsp_invalidate |
| 25 | 空 | L2 | 434.60@1,006,039 | 424.80@1,014,250 | 8,211 | -5.54 | 5 | 5 |  | bsp_invalidate |
| 26 | 空 | L2 | 368.50@1,057,130 | 366.20@1,057,460 | 330 | +0.35 | 1 | 0 |  | bsp_invalidate |
| 27 | 空 | L2 | 223.20@1,109,236 | 234.80@1,114,422 | 5,186 | -7.71 | 2 | 3 |  | bsp_invalidate |
| 28 | 空 | L2 | 301.80@1,123,018 | 303.90@1,123,503 | 485 | -0.70 | 0 | 0 |  | bsp_invalidate |

### FSM 状态分布

| 状态 | Bars | 占比 |
|------|------|------|
| SCANNING | 1,079,405 | 76.6% |
| POSITION_OPEN | 204,872 | 14.5% |
| COST_REDUCING | 124,165 | 8.8% |
| PRINCIPAL_WITHDRAWN | 440 | 0.0% |
| STOPPED_OUT | 0 | 0.0% |

<details><summary>事件流（290条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 14,097 | 10.50 | POSITION_OPEN | ENTRY_SHORT@10.50 eff=L1 gate=L0 |
| 15,070 | 11.24 | COST_REDUCING | TRIM@11.24 shares=8388.5 ratio=0.881 |
| 15,812 | 10.88 | POSITION_OPEN | CLOSE_DIFF@10.88 profit=-3019.87 |
| 16,881 | 9.96 | COST_REDUCING | TRIM@9.96 shares=7142.9 ratio=0.750 |
| 17,582 | 8.90 | POSITION_OPEN | CLOSE_DIFF@8.90 profit=-7571.43 |
| 19,212 | 9.38 | COST_REDUCING | TRIM@9.38 shares=5872.4 ratio=0.617 |
| 20,423 | 8.94 | POSITION_OPEN | CLOSE_DIFF@8.94 profit=-2583.85 |
| 21,798 | 9.70 | COST_REDUCING | TRIM@9.70 shares=7128.8 ratio=0.749 |
| 22,598 | 9.86 | SCANNING | STOP:dir_flip_long_L1@9.86 pnl=+6.10% |
| 53,073 | 13.82 | POSITION_OPEN | ENTRY_LONG@13.82 eff=L1 gate=L0 |
| 53,473 | 13.04 | COST_REDUCING | TRIM@13.04 shares=6682.3 ratio=0.923 |
| 54,235 | 14.56 | POSITION_OPEN | CLOSE_DIFF@14.56 profit=-10157.13 |
| 55,587 | 14.56 | COST_REDUCING | TRIM@14.56 shares=6892.1 ratio=0.952 |
| 56,505 | 15.84 | POSITION_OPEN | CLOSE_DIFF@15.84 profit=-8821.89 |
| 58,754 | 16.48 | COST_REDUCING | TRIM@16.48 shares=6725.0 ratio=0.929 |
| 60,480 | 15.72 | POSITION_OPEN | CLOSE_DIFF@15.72 profit=5110.97 |
| 62,111 | 16.36 | COST_REDUCING | TRIM@16.36 shares=7018.0 ratio=0.970 |
| 62,678 | 16.41 | POSITION_OPEN | CLOSE_DIFF@16.41 profit=-350.90 |
| 63,288 | 16.60 | POSITION_OPEN | ADD_POS@16.60 +1056.3 ratio=0.146 |
| 63,471 | 16.67 | SCANNING | STOP:bsp_invalidate@16.67 pnl=+22.52% |
| 64,573 | 17.66 | POSITION_OPEN | ENTRY_LONG@17.66 eff=L1 gate=L0 |
| 65,538 | 18.71 | COST_REDUCING | TRIM@18.71 shares=5375.9 ratio=0.949 |
| 65,703 | 18.95 | POSITION_OPEN | CLOSE_DIFF@18.95 profit=-1290.22 |
| 68,114 | 19.91 | COST_REDUCING | TRIM@19.91 shares=5615.0 ratio=0.992 |
| 68,448 | 20.26 | POSITION_OPEN | CLOSE_DIFF@20.26 profit=-1965.25 |
| 69,417 | 21.66 | COST_REDUCING | TRIM@21.66 shares=5462.3 ratio=0.965 |
| 69,585 | 21.35 | POSITION_OPEN | CLOSE_DIFF@21.35 profit=1693.31 |
| 69,802 | 20.65 | SCANNING | STOP:dir_flip_short_L1@20.65 pnl=+18.62% |
| 92,437 | 31.30 | POSITION_OPEN | ENTRY_LONG@31.30 eff=L1 gate=L0 |
| 93,216 | 31.05 | COST_REDUCING | TRIM@31.05 shares=2981.1 ratio=0.933 |
| 94,600 | 31.85 | POSITION_OPEN | CLOSE_DIFF@31.85 profit=-2384.85 |
| 95,155 | 29.35 | SCANNING | STOP:dir_flip_short_L1@29.35 pnl=-6.23% |
| 95,553 | 27.35 | POSITION_OPEN | ENTRY_SHORT@27.35 eff=L1 gate=L0 |
| 96,373 | 26.00 | POSITION_OPEN | ADD_POS@26.00 +3045.8 ratio=0.833 |
| 96,465 | 25.95 | POSITION_OPEN | ADD_POS@25.95 +5582.9 ratio=0.833 |
| 96,725 | 26.95 | SCANNING | STOP:bsp_invalidate@26.95 pnl=-2.09% |
| 138,285 | 34.30 | POSITION_OPEN | ENTRY_LONG@34.30 eff=L1 gate=L0 |
| 140,700 | 33.65 | COST_REDUCING | TRIM@33.65 shares=2915.5 ratio=1.000 |
| 142,225 | 33.30 | SCANNING | STOP:dir_flip_short_L1@33.30 pnl=-2.92% |
| 158,103 | 34.90 | POSITION_OPEN | ENTRY_SHORT@34.90 eff=L1 gate=L0 |
| 158,322 | 36.05 | SCANNING | STOP:bsp_invalidate@36.05 pnl=-3.30% |
| 162,024 | 40.80 | POSITION_OPEN | ENTRY_LONG@40.80 eff=L1 gate=L0 |
| 163,408 | 39.95 | COST_REDUCING | TRIM@39.95 shares=2273.9 ratio=0.928 |
| 164,411 | 36.80 | POSITION_OPEN | CLOSE_DIFF@36.80 profit=7162.87 |
| 164,555 | 36.65 | SCANNING | STOP:dir_flip_short_L1@36.65 pnl=-3.01% |
| 214,601 | 28.40 | POSITION_OPEN | ENTRY_SHORT@28.40 eff=L2 gate=L1 |
| 214,609 | 28.55 | SCANNING | STOP:bsp_invalidate@28.55 pnl=-0.53% |
| 257,942 | 41.90 | POSITION_OPEN | ENTRY_LONG@41.90 eff=L2 gate=L1 |
| 260,392 | 41.50 | COST_REDUCING | TRIM@41.50 shares=2271.0 ratio=0.952 |
| 261,452 | 40.90 | POSITION_OPEN | CLOSE_DIFF@40.90 profit=1362.57 |
| 261,751 | 41.20 | COST_REDUCING | TRIM@41.20 shares=2271.0 ratio=0.952 |
| 262,635 | 43.10 | POSITION_OPEN | CLOSE_DIFF@43.10 profit=-4314.82 |
| 264,719 | 41.10 | COST_REDUCING | TRIM@41.10 shares=2283.1 ratio=0.957 |
| 266,579 | 41.70 | POSITION_OPEN | CLOSE_DIFF@41.70 profit=-1369.88 |
| 269,683 | 42.20 | COST_REDUCING | TRIM@42.20 shares=2283.1 ratio=0.957 |
| 270,353 | 43.30 | POSITION_OPEN | CLOSE_DIFF@43.30 profit=-2511.45 |
| 270,516 | 43.40 | POSITION_OPEN | ADD_POS@43.40 +487.1 ratio=0.204 |
| 273,094 | 45.40 | COST_REDUCING | TRIM@45.40 shares=2793.1 ratio=0.972 |
| 273,491 | 45.50 | POSITION_OPEN | CLOSE_DIFF@45.50 profit=-279.31 |
| 274,789 | 45.40 | COST_REDUCING | TRIM@45.40 shares=2830.4 ratio=0.985 |
| 278,124 | 45.20 | POSITION_OPEN | CLOSE_DIFF@45.20 profit=566.08 |
| 284,475 | 48.20 | COST_REDUCING | TRIM@48.20 shares=2727.0 ratio=0.949 |
| 288,409 | 49.60 | POSITION_OPEN | CLOSE_DIFF@49.60 profit=-3817.82 |
| 291,912 | 49.70 | COST_REDUCING | TRIM@49.70 shares=2841.5 ratio=0.989 |
| 302,478 | 45.90 | POSITION_OPEN | CLOSE_DIFF@45.90 profit=10797.65 |
| 307,353 | 49.60 | POSITION_OPEN | ADD_POS@49.60 +412.4 ratio=0.143 |
| 309,259 | 49.20 | COST_REDUCING | TRIM@49.20 shares=3219.8 ratio=0.980 |
| 309,924 | 50.00 | POSITION_OPEN | CLOSE_DIFF@50.00 profit=-2575.81 |
| 315,344 | 48.60 | COST_REDUCING | TRIM@48.60 shares=3264.1 ratio=0.993 |
| 316,732 | 51.30 | POSITION_OPEN | CLOSE_DIFF@51.30 profit=-8813.12 |
| 320,109 | 48.50 | COST_REDUCING | TRIM@48.50 shares=3172.3 ratio=0.965 |
| 324,400 | 46.20 | POSITION_OPEN | CLOSE_DIFF@46.20 profit=7296.22 |
| 326,792 | 46.70 | POSITION_OPEN | ADD_POS@46.70 +618.8 ratio=0.188 |
| 329,386 | 50.00 | POSITION_OPEN | ADD_POS@50.00 +735.3 ratio=0.188 |
| 331,116 | 50.30 | COST_REDUCING | TRIM@50.30 shares=4479.5 ratio=0.965 |
| 334,063 | 55.10 | POSITION_OPEN | CLOSE_DIFF@55.10 profit=-21501.66 |
| 336,793 | 55.40 | COST_REDUCING | TRIM@55.40 shares=4631.0 ratio=0.998 |
| 338,777 | 55.50 | POSITION_OPEN | CLOSE_DIFF@55.50 profit=-463.10 |
| 340,518 | 51.70 | COST_REDUCING | TRIM@51.70 shares=4253.5 ratio=0.917 |
| 341,695 | 55.50 | POSITION_OPEN | CLOSE_DIFF@55.50 profit=-16163.41 |
| 343,058 | 56.70 | COST_REDUCING | TRIM@56.70 shares=4558.8 ratio=0.982 |
| 343,399 | 56.10 | POSITION_OPEN | CLOSE_DIFF@56.10 profit=2735.28 |
| 352,110 | 68.50 | COST_REDUCING | TRIM@68.50 shares=4353.0 ratio=0.938 |
| 352,332 | 67.70 | POSITION_OPEN | CLOSE_DIFF@67.70 profit=3482.37 |
| 352,795 | 68.70 | COST_REDUCING | TRIM@68.70 shares=4618.2 ratio=0.995 |
| 355,033 | 66.20 | POSITION_OPEN | CLOSE_DIFF@66.20 profit=11545.55 |
| 364,770 | 76.90 | COST_REDUCING | TRIM@76.90 shares=4582.5 ratio=0.988 |
| 367,580 | 78.70 | POSITION_OPEN | CLOSE_DIFF@78.70 profit=-8248.41 |
| 368,642 | 77.30 | COST_REDUCING | TRIM@77.30 shares=4334.5 ratio=0.934 |
| 372,638 | 75.70 | POSITION_OPEN | CLOSE_DIFF@75.70 profit=6935.14 |
| 376,128 | 81.90 | POSITION_OPEN | ADD_POS@81.90 +743.4 ratio=0.160 |
| 377,223 | 82.40 | POSITION_OPEN | ADD_POS@82.40 +862.5 ratio=0.160 |
| 377,587 | 82.80 | SCANNING | STOP:bsp_invalidate@82.80 pnl=+85.09% |
| 389,456 | 96.80 | POSITION_OPEN | ENTRY_LONG@96.80 eff=L2 gate=L1 |
| 389,683 | 94.80 | SCANNING | STOP:bsp_invalidate@94.80 pnl=-2.07% |
| 428,960 | 119.20 | POSITION_OPEN | ENTRY_LONG@119.20 eff=L2 gate=L1 |
| 429,469 | 118.80 | COST_REDUCING | TRIM@118.80 shares=806.9 ratio=0.962 |
| 429,983 | 119.35 | POSITION_OPEN | CLOSE_DIFF@119.35 profit=-443.78 |
| 431,636 | 121.50 | COST_REDUCING | TRIM@121.50 shares=818.6 ratio=0.976 |
| 432,017 | 123.05 | POSITION_OPEN | CLOSE_DIFF@123.05 profit=-1268.91 |
| 435,026 | 120.20 | COST_REDUCING | TRIM@120.20 shares=819.3 ratio=0.977 |
| 437,828 | 118.45 | POSITION_OPEN | CLOSE_DIFF@118.45 profit=1433.84 |
| 438,678 | 112.75 | COST_REDUCING | ADD_POS@112.75 +52.9 ratio=0.063 |
| 438,789 | 111.25 | SCANNING | STOP:bsp_invalidate@111.25 pnl=-5.00% |
| 468,385 | 121.10 | POSITION_OPEN | ENTRY_LONG@121.10 eff=L2 gate=L1 |
| 470,753 | 124.35 | COST_REDUCING | TRIM@124.35 shares=809.2 ratio=0.980 |
| 471,012 | 124.05 | POSITION_OPEN | CLOSE_DIFF@124.05 profit=242.77 |
| 471,415 | 124.70 | COST_REDUCING | TRIM@124.70 shares=821.5 ratio=0.995 |
| 471,599 | 124.55 | POSITION_OPEN | CLOSE_DIFF@124.55 profit=123.22 |
| 472,091 | 122.40 | COST_REDUCING | TRIM@122.40 shares=821.5 ratio=0.995 |
| 475,334 | 124.70 | POSITION_OPEN | CLOSE_DIFF@124.70 profit=-1889.37 |
| 477,069 | 123.35 | COST_REDUCING | TRIM@123.35 shares=866.5 ratio=0.993 |
| 479,346 | 122.15 | POSITION_OPEN | CLOSE_DIFF@122.15 profit=1039.77 |
| 480,282 | 125.35 | POSITION_OPEN | ADD_POS@125.35 +50.0 ratio=0.057 |
| 481,043 | 133.40 | COST_REDUCING | TRIM@133.40 shares=919.4 ratio=0.996 |
| 481,240 | 134.50 | POSITION_OPEN | CLOSE_DIFF@134.50 profit=-1011.33 |
| 482,025 | 131.35 | COST_REDUCING | TRIM@131.35 shares=868.5 ratio=0.941 |
| 482,914 | 134.60 | POSITION_OPEN | CLOSE_DIFF@134.60 profit=-2822.55 |
| 485,735 | 145.20 | COST_REDUCING | TRIM@145.20 shares=837.3 ratio=0.907 |
| 486,139 | 146.45 | POSITION_OPEN | CLOSE_DIFF@146.45 profit=-1046.60 |
| ... | ... | ... | （共290条，显示前120） |
</details>

## 汇总对比

| 标的 | Bars | EffLv | BH% | 复利% | 胜率 | 交易数 | 加仓率 | 降成本率 | PW率 | 耗时 |
|------|------|-------|-----|------|------|--------|--------|---------|------|------|
| QQQ | 728,030 | L2 | +174.6 | +40.00 | 43% | 51 | 26/51 | 30/51 | 0/51 | 375s |
| OKLO | 333,613 | L2 | +251.3 | -13.96 | 29% | 17 | 15/17 | 13/17 | 0/17 | 75s |
| HK700 | 1,408,882 | L3 | +3933.5 | +198.60 | 43% | 28 | 15/28 | 15/28 | 1/28 | 545s |

## 结果包六要素

**结论**：自适应 PH 分层 FSM — 方向裁决级别从数据涌现。

**定义依据**：
- PH L0 = 1min close 的因果 merge tree
- PH L1 = L1 线段端点序列的 merge tree
- PH L(N+1) = 结构层级 N 走势端点序列的 merge tree
- 有效级别 E = 最高有 ≥10 更新 + ≥1 rank-1 settle 的 PH 级别
- 信号路由：方向=E, 门控=E-1, 降成本=E-2（下界 0）

**边界条件**：
- MIN_PH_UPDATES = 10，更小 → 更早激活高级别但可能不稳定
- PENDING_EXPIRY = 390 bars
- 有效级别只会上升不会下降（结构一旦涌现不会消失）
- 数据前期只有低级别有效 → 行为与旧版类似

**下游推论**：
- 若不同标的涌现不同有效级别 → 证明自适应有意义
- 若有效级别越高、胜率越高 → 级别匹配假说进一步确认
- 若高级别标的（如 QQQ L4）比低级别标的（如 OKLO L3）表现更好 → 级别深度有价值

**谱系引用**：267号, §7.5, 526号

**影响声明**：新建独立回测脚本。RecursiveOrchestrator 使用 max_levels=6 充分递归。

**认识论等级**：L2（真实数据，3标的 1min；可产生否定性结果）。