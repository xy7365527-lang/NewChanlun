# 完整版多层赋格 FSM 回测（H组）— 1分钟数据

## 架构

5状态 FSM：`SCANNING → POSITION_OPEN → COST_REDUCING → PRINCIPAL_WITHDRAWN → STOPPED_OUT`

单 pass：BiEngine (增量笔) + 滑窗 BSP (W=500) + PH settle-only + FSM。

| 维度 | 实现 | 参数来源 |
|------|------|---------|
| 方向 | rank-1 settle | 零参数 |
| 进场 | chanlun candidate + PH settle 门控 | 零参数 |
| 加仓 | 同方向新 BSP | ratio = alive_rank1 / dominant |
| 降成本 | non-rank1 settle → 减仓 | ratio = alive_rank1 / dominant |
| 回补 | 同方向 settle 或 BSP → 买回 | ≤ 已减量 |
| 本金回收 | cumulative_recovered ≥ own_capital | 自动阈值 |
| 清仓 | rank-1 settle 或 BSP invalidate | 零参数 |
| 双向 | 多/空对称 | — |
| PENDING_EXPIRY | 390 bars | — |

## QQQ

- 数据：**728,030** bars (1min), bar_0 → bar_728029
- 价格：268.82 → 738.28
- Buy-and-hold: **+174.64%**
- 回测耗时：**187.2s** (3889 bars/s)

### 总体指标

| 指标 | 值 |
|------|-----|
| 交易数 | 824 (多280/空544) |
| 胜率 | 39.2% |
| 平均收益 | -0.026% |
| 复利累计 | -19.29% |
| 最大回撤 | -21.84% |
| 平均持仓 | 89 bars |
| 有加仓 | 175/824 |
| 有降成本 | 304/824 |
| 本金回收 | 1/824 |

### 交易明细

| # | 方向 | 入场 | 出场 | 持仓 | PnL% | 加仓 | 短差 | PW | 退出 | 状态 |
|---|------|------|------|------|------|------|------|-----|------|------|
| 1 | 空 | 268.60@146 | 267.60@155 | 9 | +0.37 | 0 | 0 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 2 | 多 | 268.47@225 | 266.51@336 | 111 | -0.67 | 1 | 6 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 3 | 空 | 264.17@398 | 264.96@401 | 3 | -0.30 | 0 | 0 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 4 | 空 | 262.43@488 | 263.10@495 | 7 | -0.26 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 5 | 多 | 266.36@808 | 266.33@959 | 151 | +0.12 | 2 | 10 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 6 | 多 | 266.98@1236 | 267.31@1243 | 7 | +0.12 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 7 | 多 | 267.36@1256 | 265.05@1465 | 209 | +0.02 | 2 | 13 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 8 | 空 | 261.83@1960 | 262.47@2180 | 220 | -0.51 | 1 | 13 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 9 | 空 | 261.48@2330 | 261.81@2356 | 26 | -0.13 | 1 | 3 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 10 | 多 | 261.04@2503 | 260.98@2506 | 3 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 11 | 多 | 260.89@2604 | 260.99@2610 | 6 | +0.04 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 12 | 多 | 265.36@2810 | 269.29@3081 | 271 | +0.68 | 5 | 22 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 13 | 多 | 269.94@3258 | 270.09@3291 | 33 | +0.06 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 14 | 多 | 270.92@3535 | 272.89@3593 | 58 | +0.73 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 15 | 空 | 274.76@3652 | 274.89@3653 | 1 | -0.05 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 16 | 空 | 272.52@3852 | 271.56@3878 | 26 | +0.35 | 0 | 2 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 17 | 空 | 270.74@3904 | 270.58@4420 | 516 | -0.72 | 6 | 32 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 18 | 多 | 275.17@5184 | 275.80@5310 | 126 | +0.61 | 0 | 6 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 19 | 多 | 276.20@5342 | 275.64@5354 | 12 | -0.20 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 20 | 空 | 275.62@5404 | 275.70@5406 | 2 | -0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 21 | 多 | 276.59@5469 | 276.60@5488 | 19 | +0.00 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 22 | 多 | 277.60@5581 | 277.70@5684 | 103 | +0.06 | 0 | 8 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 23 | 多 | 278.55@5860 | 278.51@5863 | 3 | -0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 24 | 空 | 277.62@5989 | 279.50@6144 | 155 | -0.55 | 2 | 11 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 25 | 多 | 279.98@6982 | 279.79@7031 | 49 | -0.07 | 0 | 4 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 26 | 空 | 280.58@7088 | 280.74@7090 | 2 | -0.06 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 27 | 空 | 280.54@7104 | 280.50@7107 | 3 | +0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 28 | 空 | 279.70@7236 | 279.66@7246 | 10 | +0.01 | 0 | 0 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 29 | 空 | 280.30@7400 | 281.24@7536 | 136 | -0.35 | 0 | 8 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 30 | 空 | 280.44@7608 | 280.40@7611 | 3 | +0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 31 | 多 | 283.51@8241 | 282.18@8244 | 3 | -0.47 | 0 | 0 |  | direction_flip_short | POSITION_OPEN |
| 32 | 多 | 283.76@8322 | 283.14@8349 | 27 | -0.22 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 33 | 空 | 279.73@8413 | 279.57@8454 | 41 | +0.06 | 0 | 1 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 34 | 空 | 279.05@10183 | 279.36@10186 | 3 | -0.11 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 35 | 空 | 281.27@10260 | 281.45@10266 | 6 | -0.06 | 0 | 0 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 36 | 多 | 284.71@10788 | 285.77@10808 | 20 | +0.37 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 37 | 多 | 287.41@10849 | 288.79@10889 | 40 | +0.51 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 38 | 多 | 289.52@11000 | 289.79@11035 | 35 | +0.09 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 39 | 空 | 289.55@11041 | 289.17@11054 | 13 | +0.13 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 40 | 空 | 286.92@11576 | 290.53@11986 | 410 | -1.00 | 1 | 27 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 41 | 空 | 289.75@12090 | 286.60@12161 | 71 | +0.63 | 1 | 3 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 42 | 空 | 286.38@12174 | 286.35@12186 | 12 | +0.01 | 0 | 1 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 43 | 空 | 286.10@12205 | 285.69@12214 | 9 | +0.14 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 44 | 空 | 284.82@12317 | 284.90@12376 | 59 | -0.05 | 2 | 4 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 45 | 多 | 292.45@13661 | 292.35@13690 | 29 | -0.03 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 46 | 多 | 293.55@14145 | 294.50@14181 | 36 | +0.34 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 47 | 多 | 295.55@14262 | 295.54@14264 | 2 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 48 | 多 | 295.83@14294 | 295.99@14341 | 47 | +0.13 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 49 | 空 | 296.74@14408 | 296.87@14409 | 1 | -0.04 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 50 | 多 | 297.44@14448 | 297.38@14476 | 28 | -0.02 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 51 | 空 | 297.29@14505 | 296.14@14535 | 30 | +0.39 | 0 | 0 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 52 | 空 | 293.33@14626 | 293.21@14633 | 7 | +0.04 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 53 | 空 | 288.56@15516 | 291.23@15729 | 213 | -0.73 | 2 | 16 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 54 | 空 | 294.85@16486 | 294.80@16493 | 7 | +0.02 | 0 | 1 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 55 | 多 | 300.47@16929 | 299.84@16942 | 13 | -0.21 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 56 | 多 | 301.76@16956 | 300.90@16976 | 20 | -0.28 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 57 | 空 | 303.25@17040 | 303.18@17055 | 15 | +0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 58 | 空 | 303.56@17102 | 303.99@17114 | 12 | -0.14 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 59 | 空 | 303.85@17144 | 303.82@17155 | 11 | +0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 60 | 空 | 305.44@17172 | 305.37@17219 | 47 | -0.04 | 1 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 61 | 多 | 305.75@17388 | 306.03@17429 | 41 | +0.09 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 62 | 空 | 306.86@17466 | 306.95@17467 | 1 | -0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 63 | 多 | 308.99@17540 | 310.37@17570 | 30 | +0.45 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 64 | 空 | 310.25@17573 | 310.98@17579 | 6 | -0.24 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 65 | 空 | 310.17@17590 | 310.53@17600 | 10 | -0.12 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 66 | 空 | 312.57@17762 | 311.62@17770 | 8 | +0.30 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 67 | 空 | 308.00@17892 | 306.74@18617 | 725 | -0.20 | 4 | 45 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 68 | 空 | 302.31@22000 | 301.63@22059 | 59 | +0.04 | 1 | 4 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 69 | 空 | 298.92@22315 | 300.83@22580 | 265 | -0.52 | 2 | 20 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 70 | 空 | 297.79@22673 | 297.91@22680 | 7 | -0.04 | 0 | 1 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 71 | 空 | 297.27@27437 | 297.32@27475 | 38 | -0.07 | 0 | 2 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 72 | 空 | 288.15@38612 | 289.31@38700 | 88 | -0.45 | 1 | 7 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 73 | 空 | 287.37@39206 | 289.65@39291 | 85 | -1.73 | 1 | 5 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 74 | 空 | 306.73@42383 | 307.50@42651 | 268 | -0.24 | 1 | 22 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 75 | 多 | 313.95@45963 | 311.00@45982 | 19 | -0.94 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 76 | 空 | 305.32@49265 | 309.71@49825 | 560 | -0.73 | 3 | 39 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 77 | 空 | 314.50@50815 | 314.51@50822 | 7 | -0.00 | 0 | 0 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 78 | 多 | 315.00@50895 | 314.83@50920 | 25 | -0.05 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 79 | 多 | 315.55@50979 | 314.50@50992 | 13 | -0.33 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 80 | 空 | 315.47@51027 | 315.50@51031 | 4 | -0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 81 | 多 | 316.77@51833 | 317.80@51907 | 74 | +0.33 | 0 | 5 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 82 | 空 | 318.68@51984 | 318.78@51986 | 2 | -0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 83 | 多 | 319.36@52018 | 319.05@52037 | 19 | -0.10 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 84 | 多 | 319.68@52156 | 320.23@52194 | 38 | +0.17 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 85 | 多 | 321.15@52282 | 321.11@52316 | 34 | -0.01 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 86 | 空 | 321.21@52378 | 321.33@52388 | 10 | -0.04 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 87 | 多 | 321.44@52418 | 319.46@52420 | 2 | -0.62 | 0 | 0 |  | direction_flip_short | POSITION_OPEN |
| 88 | 空 | 318.60@52460 | 318.75@52469 | 9 | -0.03 | 1 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 89 | 空 | 318.60@52645 | 318.37@52663 | 18 | +0.07 | 0 | 0 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 90 | 多 | 321.58@53404 | 321.87@53424 | 20 | +0.09 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 91 | 空 | 321.70@53469 | 321.85@53471 | 2 | -0.05 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 92 | 空 | 316.68@54515 | 315.50@54849 | 334 | -0.09 | 4 | 25 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 93 | 空 | 313.77@55188 | 317.31@55556 | 368 | -0.71 | 3 | 22 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 94 | 空 | 313.06@58271 | 313.30@58289 | 18 | -0.08 | 0 | 1 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 95 | 空 | 312.24@65735 | 310.67@65934 | 199 | +0.23 | 3 | 17 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 96 | 空 | 314.80@67256 | 314.74@67265 | 9 | +0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 97 | 空 | 315.30@67345 | 318.04@67463 | 118 | -0.55 | 1 | 7 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 98 | 多 | 322.18@67707 | 319.40@68131 | 424 | -0.08 | 3 | 29 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 99 | 多 | 322.53@68718 | 322.40@68756 | 38 | -0.04 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 100 | 空 | 322.03@68924 | 322.12@68926 | 2 | -0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 101 | 多 | 322.02@68959 | 321.72@69067 | 108 | -0.04 | 0 | 11 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 102 | 多 | 323.12@69217 | 323.15@69219 | 2 | +0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 103 | 多 | 323.59@72703 | 323.15@72727 | 24 | -0.14 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 104 | 空 | 323.71@73495 | 323.57@73500 | 5 | +0.04 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 105 | 空 | 323.34@73585 | 323.35@73608 | 23 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 106 | 空 | 322.29@73704 | 322.48@73710 | 6 | -0.06 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 107 | 空 | 321.43@74105 | 321.65@74109 | 4 | -0.07 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 108 | 多 | 324.30@74691 | 324.22@74731 | 40 | -0.02 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 109 | 多 | 325.01@74740 | 323.85@74748 | 8 | -0.36 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 110 | 多 | 325.65@75091 | 325.60@75114 | 23 | -0.02 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 111 | 多 | 326.48@75339 | 326.07@75378 | 39 | -0.13 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 112 | 空 | 325.18@75563 | 325.21@75578 | 15 | -0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 113 | 多 | 326.73@76080 | 326.95@76118 | 38 | +0.07 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 114 | 空 | 326.59@76262 | 326.65@76270 | 8 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 115 | 空 | 326.71@77987 | 328.29@78033 | 46 | -0.48 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 116 | 多 | 328.36@78049 | 327.87@78053 | 4 | -0.15 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 117 | 多 | 328.52@78066 | 327.87@78073 | 7 | -0.20 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 118 | 多 | 331.21@78985 | 331.37@79008 | 23 | +0.05 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 119 | 空 | 331.14@79037 | 331.23@79039 | 2 | -0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 120 | 空 | 330.86@79047 | 330.95@79050 | 3 | -0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 121 | 多 | 331.73@79263 | 331.82@79288 | 25 | +0.03 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 122 | 空 | 331.69@79303 | 331.69@79305 | 2 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 123 | 多 | 332.18@79396 | 332.06@79438 | 42 | -0.03 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 124 | 多 | 335.42@79638 | 335.76@79671 | 33 | +0.10 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 125 | 空 | 335.13@79681 | 334.81@79688 | 7 | +0.10 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 126 | 空 | 335.03@79747 | 335.22@79749 | 2 | -0.06 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 127 | 空 | 337.67@79980 | 337.66@79989 | 9 | +0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 128 | 空 | 337.63@80011 | 337.56@80016 | 5 | +0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 129 | 空 | 337.71@80110 | 337.62@80119 | 9 | +0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 130 | 空 | 337.53@80221 | 337.04@80436 | 215 | +0.17 | 2 | 0 |  | direction_flip_long | POSITION_OPEN |
| 131 | 空 | 335.65@80581 | 337.61@81427 | 846 | -0.27 | 2 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 132 | 空 | 335.45@82207 | 331.37@82822 | 615 | +0.42 | 5 | 41 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 133 | 空 | 330.99@82874 | 334.68@83261 | 387 | -1.18 | 3 | 26 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 134 | 空 | 338.85@83705 | 339.27@83719 | 14 | -0.12 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 135 | 多 | 339.52@83743 | 339.10@83765 | 22 | -0.11 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 136 | 多 | 340.69@84038 | 339.83@84069 | 31 | -0.25 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 137 | 多 | 340.86@84419 | 341.14@84483 | 64 | +0.12 | 0 | 5 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 138 | 空 | 341.08@84487 | 341.07@84492 | 5 | +0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 139 | 空 | 340.53@84518 | 340.60@84519 | 1 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 140 | 空 | 342.98@84606 | 346.10@84693 | 87 | -0.44 | 1 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 141 | 空 | 347.09@84729 | 347.48@84731 | 2 | -0.11 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 142 | 多 | 347.61@84749 | 347.90@84781 | 32 | +0.08 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 143 | 空 | 348.10@84805 | 348.29@84808 | 3 | -0.05 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 144 | 多 | 348.52@84894 | 348.77@84932 | 38 | +0.07 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 145 | 多 | 349.16@84982 | 349.06@85028 | 46 | -0.02 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 146 | 多 | 349.25@85126 | 351.89@85202 | 76 | +0.76 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 147 | 多 | 352.10@85216 | 352.23@85237 | 21 | +0.04 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 148 | 空 | 352.31@85286 | 352.43@85293 | 7 | -0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 149 | 空 | 352.11@85320 | 352.38@85327 | 7 | -0.08 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 150 | 空 | 353.71@85400 | 353.70@85405 | 5 | +0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 151 | 多 | 353.01@85477 | 352.24@85491 | 14 | -0.22 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 152 | 空 | 350.09@85660 | 351.01@85792 | 132 | -0.40 | 2 | 3 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 153 | 空 | 348.53@86221 | 348.65@86232 | 11 | -0.03 | 1 | 1 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 154 | 空 | 348.30@86259 | 348.58@86325 | 66 | -0.08 | 1 | 4 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 155 | 空 | 348.06@86372 | 351.26@87335 | 963 | -0.59 | 5 | 64 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 156 | 空 | 353.08@87518 | 351.59@87549 | 31 | +0.42 | 0 | 1 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 157 | 多 | 353.76@87832 | 353.19@87948 | 116 | +0.06 | 0 | 7 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 158 | 多 | 354.67@88006 | 354.04@88024 | 18 | -0.18 | 0 | 0 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 159 | 多 | 355.76@88819 | 356.85@88935 | 116 | +0.61 | 0 | 7 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 160 | 空 | 357.19@89033 | 357.00@89039 | 6 | +0.05 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 161 | 空 | 354.73@89111 | 354.80@89461 | 350 | +0.01 | 2 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 162 | 空 | 356.81@92077 | 356.87@92080 | 3 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 163 | 多 | 357.55@92088 | 358.07@93105 | 1,017 | +1.78 | 0 | 70 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 164 | 空 | 359.72@93163 | 359.61@93168 | 5 | +0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 165 | 多 | 360.54@93203 | 361.00@93255 | 52 | +0.15 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 166 | 空 | 360.99@93274 | 360.99@93276 | 2 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 167 | 空 | 362.32@93394 | 362.64@93402 | 8 | -0.09 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 168 | 空 | 362.57@93448 | 361.87@93492 | 44 | +0.11 | 1 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 169 | 空 | 361.45@93562 | 362.96@93613 | 51 | -0.42 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 170 | 多 | 363.62@93641 | 363.64@93662 | 21 | +0.01 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 171 | 多 | 364.68@94534 | 364.33@94579 | 45 | -0.10 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 172 | 多 | 364.39@94731 | 362.30@94761 | 30 | -0.57 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 173 | 多 | 366.19@94882 | 366.65@94933 | 51 | +0.15 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 174 | 空 | 366.53@94949 | 366.55@94950 | 1 | -0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 175 | 空 | 366.20@95003 | 366.35@95005 | 2 | -0.04 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 176 | 空 | 366.30@95015 | 366.34@95024 | 9 | -0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 177 | 空 | 365.69@95051 | 364.57@95102 | 51 | +0.31 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 178 | 空 | 363.71@95134 | 363.38@95228 | 94 | +0.07 | 3 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 179 | 多 | 367.88@95442 | 368.02@95503 | 61 | +0.04 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 180 | 空 | 369.71@95879 | 369.87@95886 | 7 | -0.04 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 181 | 空 | 369.26@96328 | 366.71@97017 | 689 | +0.03 | 6 | 17 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 182 | 空 | 363.90@97902 | 361.60@97985 | 83 | +0.20 | 3 | 7 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 183 | 空 | 360.66@98511 | 360.70@98522 | 11 | -0.01 | 0 | 2 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 184 | 空 | 366.61@99159 | 366.56@99163 | 4 | +0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 185 | 空 | 360.10@100467 | 361.40@101299 | 832 | -0.45 | 4 | 61 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 186 | 空 | 373.52@109048 | 373.69@109052 | 4 | -0.05 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 187 | 空 | 373.31@109069 | 373.04@109089 | 20 | +0.07 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 188 | 空 | 372.57@109110 | 371.22@109126 | 16 | +0.36 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 189 | 多 | 375.44@109596 | 375.41@109607 | 11 | -0.01 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 190 | 空 | 375.08@109712 | 375.21@109721 | 9 | -0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 191 | 多 | 377.07@109858 | 376.51@109867 | 9 | -0.15 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 192 | 多 | 376.78@109874 | 376.00@109889 | 15 | -0.21 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 193 | 空 | 377.30@110004 | 377.75@110014 | 10 | -0.12 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 194 | 空 | 377.58@110031 | 377.66@110034 | 3 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 195 | 多 | 378.74@110165 | 379.25@110220 | 55 | +0.13 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 196 | 空 | 378.96@110288 | 379.03@110297 | 9 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 197 | 空 | 378.95@110307 | 378.96@110308 | 1 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 198 | 多 | 379.97@110396 | 378.86@110483 | 87 | -0.19 | 0 | 6 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 199 | 空 | 378.74@110525 | 378.85@110527 | 2 | -0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 200 | 空 | 378.57@111004 | 381.44@111649 | 645 | -0.30 | 7 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 201 | 多 | 382.97@111881 | 382.42@111899 | 18 | -0.14 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 202 | 多 | 384.61@112526 | 384.62@112542 | 16 | +0.00 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 203 | 空 | 386.14@112616 | 386.31@112618 | 2 | -0.04 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 204 | 多 | 386.54@112646 | 386.57@112661 | 15 | +0.01 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 205 | 空 | 386.32@112674 | 386.39@112679 | 5 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 206 | 空 | 386.00@112696 | 385.30@112714 | 18 | +0.18 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 207 | 空 | 385.18@112755 | 385.29@112772 | 17 | -0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 208 | 多 | 387.43@113273 | 386.75@113286 | 13 | -0.17 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 209 | 空 | 384.79@113365 | 385.15@113441 | 76 | -0.07 | 2 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 210 | 空 | 377.22@114321 | 376.29@114358 | 37 | +0.25 | 0 | 3 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 211 | 空 | 371.74@123702 | 373.93@123959 | 257 | -0.33 | 2 | 19 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 212 | 空 | 371.39@124893 | 370.72@125009 | 116 | +0.15 | 0 | 5 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 213 | 空 | 368.67@125851 | 367.99@125938 | 87 | +0.01 | 2 | 7 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 214 | 空 | 367.16@126863 | 369.40@127105 | 242 | -0.25 | 2 | 17 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 215 | 多 | 357.95@131228 | 358.02@131233 | 5 | +0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 216 | 空 | 357.35@131469 | 357.32@131496 | 27 | -0.02 | 1 | 2 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 217 | 空 | 360.23@132267 | 360.32@132287 | 20 | -0.03 | 0 | 1 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 218 | 空 | 365.45@133076 | 366.76@133270 | 194 | -0.20 | 2 | 14 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 219 | 空 | 354.85@153017 | 355.12@153113 | 96 | -0.04 | 2 | 8 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 220 | 空 | 353.13@153976 | 353.45@154015 | 39 | -0.14 | 2 | 3 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 221 | 空 | 364.07@160026 | 364.04@160037 | 11 | +0.01 | 0 | 1 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 222 | 空 | 357.35@169260 | 357.63@169271 | 11 | -0.08 | 0 | 1 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 223 | 空 | 358.99@170036 | 359.15@170042 | 6 | -0.04 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 224 | 空 | 347.94@171350 | 347.80@171374 | 24 | +0.04 | 0 | 1 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 225 | 空 | 348.17@171733 | 345.85@171826 | 93 | +0.47 | 1 | 6 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 226 | 空 | 345.36@171890 | 345.05@172180 | 290 | -0.51 | 4 | 19 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 227 | 空 | 347.07@172464 | 347.10@172466 | 2 | -0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 228 | 空 | 357.50@175739 | 357.59@175747 | 8 | -0.03 | 0 | 1 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 229 | 空 | 373.02@179483 | 372.20@179531 | 48 | +0.13 | 1 | 3 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 230 | 多 | 387.82@183336 | 388.04@183416 | 80 | +0.11 | 0 | 6 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 231 | 多 | 388.55@185848 | 388.71@185949 | 101 | +0.15 | 0 | 7 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 232 | 多 | 389.03@185970 | 388.91@185993 | 23 | -0.03 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 233 | 空 | 388.87@186008 | 388.92@186020 | 12 | -0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 234 | 多 | 390.40@186084 | 390.12@186095 | 11 | -0.07 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 235 | 空 | 390.99@186166 | 391.17@186169 | 3 | -0.05 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 236 | 空 | 390.12@186512 | 389.74@186552 | 40 | +0.10 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 237 | 空 | 389.43@186565 | 388.69@186657 | 92 | +0.05 | 3 | 0 |  | direction_flip_long | POSITION_OPEN |
| 238 | 空 | 389.18@188187 | 389.05@188252 | 65 | -0.01 | 1 | 0 |  | direction_flip_long | POSITION_OPEN |
| 239 | 多 | 392.88@190185 | 393.65@190284 | 99 | +0.25 | 0 | 8 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 240 | 空 | 387.62@191033 | 388.69@191618 | 585 | -0.10 | 6 | 32 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 241 | 空 | 383.69@192627 | 383.54@192635 | 8 | +0.04 | 0 | 1 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 242 | 空 | 388.19@193483 | 386.36@193495 | 12 | +0.47 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 243 | 空 | 390.82@195269 | 390.46@195282 | 13 | +0.09 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 244 | 空 | 394.31@196588 | 394.31@196591 | 3 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 245 | 多 | 395.28@196730 | 395.24@196746 | 16 | -0.01 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 246 | 多 | 395.33@196752 | 395.39@196786 | 34 | +0.02 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 247 | 多 | 396.17@197011 | 396.11@197039 | 28 | -0.02 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 248 | 空 | 396.57@197191 | 396.67@197193 | 2 | -0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 249 | 空 | 395.61@197262 | 397.55@197529 | 267 | -0.39 | 1 | 0 |  | direction_flip_long | POSITION_OPEN |
| 250 | 多 | 398.63@197655 | 399.18@197684 | 29 | +0.14 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 251 | 多 | 399.27@197693 | 399.22@197708 | 15 | -0.01 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 252 | 空 | 399.18@197718 | 399.18@197730 | 12 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 253 | 空 | 399.10@197745 | 399.15@197748 | 3 | -0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 254 | 空 | 399.10@197754 | 399.30@197763 | 9 | -0.05 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 255 | 空 | 399.24@197866 | 399.57@197886 | 20 | -0.08 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 256 | 多 | 399.87@198000 | 400.06@198044 | 44 | +0.05 | 0 | 4 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 257 | 空 | 399.48@198235 | 400.81@198332 | 97 | -0.38 | 1 | 0 |  | direction_flip_long | POSITION_OPEN |
| 258 | 多 | 402.38@198364 | 403.30@198400 | 36 | +0.23 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 259 | 空 | 405.15@198553 | 405.12@198561 | 8 | +0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 260 | 空 | 405.00@198571 | 405.13@198583 | 12 | -0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 261 | 空 | 405.09@198773 | 405.19@198785 | 12 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 262 | 空 | 404.88@198798 | 405.80@198832 | 34 | -0.23 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 263 | 空 | 403.57@198960 | 401.93@199104 | 144 | +0.24 | 2 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 264 | 空 | 401.59@199136 | 403.14@199318 | 182 | -0.20 | 1 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 265 | 多 | 406.14@199744 | 406.21@199790 | 46 | +0.05 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 266 | 多 | 406.48@200579 | 407.35@200694 | 115 | +0.23 | 0 | 8 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 267 | 空 | 406.69@200930 | 406.74@200937 | 7 | -0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 268 | 空 | 407.58@201084 | 408.07@201098 | 14 | -0.12 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 269 | 多 | 407.80@201131 | 407.60@201169 | 38 | -0.05 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 270 | 多 | 409.03@201612 | 408.35@201717 | 105 | -0.16 | 0 | 6 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 271 | 空 | 407.89@201743 | 408.14@201755 | 12 | -0.06 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 272 | 多 | 409.49@202005 | 409.83@202048 | 43 | +0.08 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 273 | 空 | 407.87@202279 | 405.21@202319 | 40 | +0.65 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 274 | 空 | 403.71@202329 | 403.44@202402 | 73 | +0.06 | 2 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 275 | 多 | 410.40@204375 | 410.42@204618 | 243 | +0.15 | 0 | 18 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 276 | 空 | 410.91@204761 | 411.09@204769 | 8 | -0.04 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 277 | 多 | 411.53@205010 | 411.22@205021 | 11 | -0.08 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 278 | 空 | 410.18@205091 | 410.61@205099 | 8 | -0.10 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 279 | 多 | 411.81@205453 | 411.55@205481 | 28 | -0.06 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 280 | 空 | 412.71@205535 | 412.67@205538 | 3 | +0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 281 | 空 | 412.83@205661 | 412.95@205682 | 21 | -0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 282 | 空 | 412.04@205826 | 411.76@206301 | 475 | -0.00 | 3 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 283 | 空 | 402.89@207249 | 403.24@207338 | 89 | -0.17 | 2 | 5 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 284 | 空 | 400.32@207574 | 401.12@207854 | 280 | -0.02 | 3 | 22 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 285 | 空 | 398.20@208320 | 399.30@208601 | 281 | -0.12 | 3 | 22 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 286 | 空 | 397.92@208687 | 398.22@208698 | 11 | -0.08 | 0 | 0 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 287 | 空 | 397.75@208743 | 398.04@208993 | 250 | -0.16 | 3 | 17 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 288 | 空 | 396.17@209214 | 396.40@209223 | 9 | -0.06 | 0 | 1 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 289 | 空 | 394.59@209475 | 399.10@209725 | 250 | -0.74 | 1 | 13 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 290 | 空 | 401.37@210547 | 401.32@210550 | 3 | +0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 291 | 多 | 413.20@216485 | 413.04@216514 | 29 | -0.04 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 292 | 空 | 415.49@216680 | 415.53@216719 | 39 | -0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 293 | 多 | 416.35@216777 | 416.15@216794 | 17 | -0.05 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 294 | 空 | 416.53@216835 | 416.64@216842 | 7 | -0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 295 | 空 | 416.14@216861 | 415.94@216876 | 15 | +0.05 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 296 | 空 | 415.19@216991 | 415.15@216995 | 4 | +0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 297 | 多 | 418.10@217194 | 420.40@217282 | 88 | +0.55 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 298 | 多 | 420.90@217289 | 420.34@217292 | 3 | -0.13 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 299 | 空 | 420.94@217341 | 421.33@217382 | 41 | -0.09 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 300 | 空 | 421.79@217439 | 421.66@217454 | 15 | +0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 301 | 空 | 421.65@217472 | 421.68@217475 | 3 | -0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 302 | 多 | 423.56@217571 | 423.79@217683 | 112 | +0.05 | 0 | 6 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 303 | 空 | 423.66@217688 | 424.02@217700 | 12 | -0.09 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 304 | 空 | 423.93@217711 | 424.02@217713 | 2 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 305 | 空 | 423.13@217817 | 423.41@217835 | 18 | -0.07 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 306 | 空 | 423.02@217855 | 421.73@218467 | 612 | +0.01 | 3 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 307 | 多 | 426.40@219185 | 426.38@219198 | 13 | -0.00 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 308 | 空 | 426.46@219257 | 426.53@219273 | 16 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 309 | 多 | 427.42@219480 | 427.07@219481 | 1 | -0.08 | 0 | 0 |  | direction_flip_short | POSITION_OPEN |
| 310 | 空 | 428.07@219577 | 427.94@219580 | 3 | +0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 311 | 多 | 429.22@219658 | 429.02@219692 | 34 | -0.05 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 312 | 空 | 427.39@219761 | 426.32@220170 | 409 | -0.09 | 4 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 313 | 空 | 428.04@220323 | 427.72@220336 | 13 | +0.07 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 314 | 空 | 424.65@220598 | 424.61@220602 | 4 | +0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 315 | 空 | 425.47@220671 | 425.47@220674 | 3 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 316 | 空 | 424.14@220836 | 424.94@221070 | 234 | -0.21 | 4 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 317 | 空 | 425.08@221082 | 425.16@221086 | 4 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 318 | 空 | 422.31@223251 | 421.48@223617 | 366 | -0.09 | 5 | 23 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 319 | 空 | 419.12@223737 | 420.39@223930 | 193 | -0.18 | 4 | 11 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 320 | 空 | 419.85@224331 | 419.71@224339 | 8 | +0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 321 | 多 | 430.05@225681 | 429.70@225700 | 19 | -0.08 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 322 | 多 | 430.20@227781 | 431.95@227876 | 95 | +0.52 | 0 | 6 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 323 | 空 | 430.88@228005 | 431.37@228010 | 5 | -0.11 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 324 | 空 | 432.85@228690 | 432.74@228695 | 5 | +0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 325 | 多 | 434.11@229133 | 434.02@229161 | 28 | -0.02 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 326 | 多 | 434.50@229238 | 434.30@229267 | 29 | -0.02 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 327 | 多 | 435.18@229413 | 435.66@229482 | 69 | +0.11 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 328 | 空 | 436.66@229593 | 437.00@229604 | 11 | -0.08 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 329 | 空 | 436.96@229617 | 437.01@229630 | 13 | -0.02 | 1 | 0 |  | direction_flip_long | POSITION_OPEN |
| 330 | 多 | 437.19@229658 | 437.57@229686 | 28 | +0.09 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 331 | 空 | 436.77@230128 | 437.15@230146 | 18 | -0.08 | 1 | 0 |  | direction_flip_long | POSITION_OPEN |
| 332 | 多 | 438.23@230256 | 438.79@230277 | 21 | +0.13 | 0 | 0 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 333 | 空 | 436.00@230428 | 435.39@230434 | 6 | +0.14 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 334 | 空 | 434.82@230471 | 434.47@230602 | 131 | -0.02 | 2 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 335 | 空 | 433.23@230701 | 428.88@231494 | 793 | -0.27 | 9 | 35 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 336 | 空 | 425.12@234308 | 424.61@234461 | 153 | +0.08 | 1 | 11 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 337 | 空 | 438.84@236278 | 438.66@236282 | 4 | +0.04 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 338 | 多 | 439.09@236745 | 439.21@236824 | 79 | +0.10 | 0 | 7 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 339 | 空 | 437.05@236890 | 436.50@236904 | 14 | -0.02 | 1 | 0 |  | direction_flip_long | POSITION_OPEN |
| 340 | 多 | 440.50@240563 | 440.69@240616 | 53 | +0.05 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 341 | 多 | 441.11@240883 | 441.76@240902 | 19 | +0.15 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 342 | 多 | 442.28@240938 | 442.78@240957 | 19 | +0.11 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 343 | 空 | 442.75@240978 | 443.10@240986 | 8 | -0.08 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 344 | 多 | 446.43@241151 | 445.87@241158 | 7 | -0.13 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 345 | 空 | 445.27@241290 | 445.36@241296 | 6 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 346 | 空 | 445.25@241313 | 445.31@241317 | 4 | -0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 347 | 空 | 445.05@241336 | 445.16@241337 | 1 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 348 | 空 | 444.55@241711 | 441.00@242307 | 596 | +0.07 | 8 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 349 | 空 | 439.93@242508 | 435.17@242768 | 260 | +0.25 | 3 | 13 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 350 | 空 | 445.02@245144 | 444.52@245151 | 7 | +0.11 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 351 | 多 | 448.45@252489 | 447.77@252569 | 80 | -0.06 | 0 | 5 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 352 | 多 | 449.31@252728 | 447.14@252816 | 88 | -0.28 | 0 | 5 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 353 | 空 | 420.71@268939 | 421.00@269040 | 101 | -0.15 | 3 | 7 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 354 | 空 | 416.61@269419 | 417.86@269984 | 565 | -0.39 | 4 | 41 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 355 | 空 | 436.76@278851 | 438.24@278887 | 36 | -0.23 | 1 | 3 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 356 | 空 | 442.96@281817 | 442.96@281818 | 1 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 357 | 多 | 449.31@284342 | 450.23@284393 | 51 | +0.20 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 358 | 空 | 450.16@284428 | 450.28@284430 | 2 | -0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 359 | 空 | 450.84@284470 | 451.00@284474 | 4 | -0.04 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 360 | 空 | 451.54@284512 | 451.71@284515 | 3 | -0.04 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 361 | 空 | 452.62@284565 | 452.69@284572 | 7 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 362 | 多 | 453.06@284629 | 452.29@284672 | 43 | -0.14 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 363 | 空 | 454.37@285229 | 453.74@285315 | 86 | +0.13 | 1 | 0 |  | direction_flip_long | POSITION_OPEN |
| 364 | 空 | 452.44@285411 | 451.90@285715 | 304 | -0.04 | 3 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 365 | 空 | 451.26@286136 | 451.47@286141 | 5 | -0.05 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 366 | 多 | 454.70@286715 | 455.38@286860 | 145 | +0.31 | 0 | 12 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 367 | 空 | 455.30@286864 | 454.39@286889 | 25 | +0.20 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 368 | 多 | 456.11@287923 | 455.90@287928 | 5 | -0.05 | 0 | 0 |  | direction_flip_short | POSITION_OPEN |
| 369 | 空 | 455.62@287963 | 455.87@288048 | 85 | -0.10 | 2 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 370 | 多 | 456.21@288172 | 456.13@288179 | 7 | -0.02 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 371 | 空 | 456.03@288224 | 456.60@288266 | 42 | -0.12 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 372 | 多 | 457.59@288582 | 456.00@288594 | 12 | -0.35 | 0 | 0 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 373 | 多 | 459.68@288784 | 459.46@288795 | 11 | -0.05 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 374 | 多 | 460.51@288903 | 461.19@288974 | 71 | +0.16 | 0 | 4 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 375 | 多 | 461.41@288979 | 461.14@288993 | 14 | -0.06 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 376 | 空 | 457.23@289106 | 454.76@289637 | 531 | -0.14 | 4 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 377 | 空 | 451.88@292636 | 451.79@292666 | 30 | +0.02 | 0 | 1 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 378 | 空 | 449.68@292975 | 446.27@293209 | 234 | +0.73 | 3 | 13 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 379 | 空 | 455.21@295118 | 453.18@295134 | 16 | +0.45 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 380 | 空 | 457.41@295589 | 457.61@295596 | 7 | -0.04 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 381 | 空 | 462.56@295944 | 462.47@295948 | 4 | +0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 382 | 多 | 463.68@296010 | 463.65@296041 | 31 | -0.01 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 383 | 空 | 463.50@296076 | 463.60@296088 | 12 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 384 | 多 | 464.29@296169 | 464.20@296184 | 15 | -0.02 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 385 | 空 | 462.22@297149 | 464.75@297227 | 78 | -0.55 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 386 | 多 | 465.58@297382 | 464.99@297387 | 5 | -0.13 | 0 | 0 |  | direction_flip_short | POSITION_OPEN |
| 387 | 多 | 466.61@298983 | 467.05@299019 | 36 | +0.09 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 388 | 多 | 467.11@299050 | 467.91@299187 | 137 | +0.21 | 0 | 7 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 389 | 多 | 468.62@299301 | 468.60@299340 | 39 | -0.00 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 390 | 多 | 468.90@299380 | 468.71@299388 | 8 | -0.04 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 391 | 多 | 473.08@299505 | 474.84@299564 | 59 | +0.37 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 392 | 多 | 474.41@299585 | 474.03@299766 | 181 | +0.08 | 0 | 17 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 393 | 空 | 476.28@300041 | 476.38@300043 | 2 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 394 | 多 | 476.76@300172 | 476.49@300403 | 231 | +0.30 | 0 | 18 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 395 | 空 | 475.17@300530 | 476.81@300827 | 297 | -0.17 | 1 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 396 | 空 | 479.12@301630 | 479.20@301633 | 3 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 397 | 空 | 479.07@302009 | 479.16@302012 | 3 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 398 | 空 | 478.66@302023 | 478.50@302031 | 8 | +0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 399 | 多 | 480.97@302142 | 480.54@302151 | 9 | -0.09 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 400 | 多 | 483.22@302220 | 485.62@302290 | 70 | +0.50 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 401 | 空 | 485.62@302340 | 485.90@302342 | 2 | -0.06 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 402 | 多 | 484.46@302409 | 484.58@302442 | 33 | +0.02 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 403 | 空 | 484.20@302903 | 485.87@303329 | 426 | -0.20 | 2 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 404 | 空 | 487.36@303583 | 487.19@303587 | 4 | +0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 405 | 空 | 486.65@303620 | 480.46@304427 | 807 | +0.10 | 5 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 406 | 空 | 477.56@305300 | 477.90@305406 | 106 | -0.06 | 1 | 7 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 407 | 空 | 474.55@305592 | 474.00@305683 | 91 | +0.10 | 0 | 7 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 408 | 空 | 478.90@306292 | 478.57@306306 | 14 | +0.07 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 409 | 空 | 484.09@308470 | 483.84@308555 | 85 | -0.18 | 2 | 4 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 410 | 多 | 488.76@310992 | 490.72@311077 | 85 | +0.41 | 0 | 5 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 411 | 多 | 491.37@311109 | 491.90@311319 | 210 | +0.18 | 0 | 12 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 412 | 空 | 491.55@311488 | 492.67@311504 | 16 | -0.23 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 413 | 多 | 494.38@311641 | 495.43@311717 | 76 | +0.21 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 414 | 多 | 495.72@311726 | 495.81@311744 | 18 | +0.02 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 415 | 多 | 496.47@311937 | 496.21@311952 | 15 | -0.05 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 416 | 多 | 496.59@312286 | 496.48@312324 | 38 | -0.01 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 417 | 多 | 496.87@312387 | 495.65@312433 | 46 | -0.25 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 418 | 空 | 496.79@312495 | 497.06@312497 | 2 | -0.05 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 419 | 多 | 497.90@312835 | 498.00@312851 | 16 | +0.02 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 420 | 多 | 499.00@312902 | 499.04@312920 | 18 | +0.01 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 421 | 空 | 498.97@312929 | 498.98@312936 | 7 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 422 | 空 | 499.27@312995 | 498.70@313107 | 112 | +0.09 | 1 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 423 | 空 | 497.91@313264 | 498.15@313286 | 22 | -0.05 | 1 | 0 |  | direction_flip_long | POSITION_OPEN |
| 424 | 空 | 497.68@313352 | 497.88@313610 | 258 | -0.04 | 2 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 425 | 空 | 499.75@313859 | 499.70@313863 | 4 | +0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 426 | 空 | 499.58@313929 | 500.10@313947 | 18 | -0.10 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 427 | 多 | 500.77@314100 | 502.31@314217 | 117 | +0.31 | 0 | 9 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 428 | 空 | 502.58@314658 | 504.59@314671 | 13 | -0.40 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 429 | 空 | 502.48@314733 | 502.97@314735 | 2 | -0.10 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 430 | 空 | 502.43@314741 | 502.99@314745 | 4 | -0.11 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 431 | 空 | 496.24@314819 | 491.98@315250 | 431 | +0.06 | 6 | 23 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 432 | 空 | 490.73@317819 | 487.67@318163 | 344 | +0.26 | 4 | 26 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 433 | 多 | 474.96@320166 | 474.85@320180 | 14 | -0.02 | 0 | 1 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 434 | 空 | 472.48@322582 | 470.24@322635 | 53 | +0.31 | 1 | 3 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 435 | 空 | 469.67@322644 | 464.70@322941 | 297 | +0.09 | 5 | 20 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 436 | 空 | 467.23@327032 | 467.08@327036 | 4 | +0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 437 | 多 | 424.14@329774 | 424.32@329779 | 5 | +0.04 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 438 | 空 | 451.11@333303 | 451.25@333310 | 7 | -0.03 | 0 | 0 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 439 | 空 | 473.11@337033 | 473.35@337049 | 16 | -0.05 | 0 | 1 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 440 | 空 | 481.31@338968 | 481.36@338972 | 4 | -0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 441 | 空 | 481.52@339080 | 481.56@339094 | 14 | -0.01 | 0 | 0 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 442 | 空 | 482.29@339147 | 481.70@339312 | 165 | +0.04 | 1 | 12 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 443 | 空 | 484.52@340947 | 484.45@340950 | 3 | +0.01 | 0 | 0 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 444 | 空 | 504.14@385852 | 504.67@385862 | 10 | -0.10 | 1 | 0 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 445 | 多 | 505.46@385953 | 505.43@385990 | 37 | -0.01 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 446 | 多 | 506.44@386066 | 506.95@386189 | 123 | +0.13 | 0 | 7 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 447 | 多 | 507.18@386304 | 507.37@386337 | 33 | +0.04 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 448 | 多 | 507.60@386359 | 507.77@386376 | 17 | +0.03 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 449 | 空 | 510.87@386506 | 510.75@386509 | 3 | +0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 450 | 空 | 511.69@386537 | 511.88@386541 | 4 | -0.04 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 451 | 空 | 512.36@386619 | 512.30@386626 | 7 | +0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 452 | 空 | 513.52@386812 | 513.98@386823 | 11 | -0.09 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 453 | 空 | 512.40@387056 | 511.84@387078 | 22 | +0.11 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 454 | 多 | 514.43@387517 | 514.20@387602 | 85 | -0.01 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 455 | 多 | 515.80@387834 | 515.22@387860 | 26 | -0.11 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 456 | 空 | 516.41@387986 | 516.73@387991 | 5 | -0.06 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 457 | 空 | 516.41@387997 | 516.44@387999 | 2 | -0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 458 | 空 | 514.82@388079 | 512.88@388115 | 36 | +0.38 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 459 | 空 | 510.76@389104 | 512.22@389411 | 307 | -0.12 | 1 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 460 | 空 | 509.38@390621 | 504.43@391218 | 597 | +0.22 | 4 | 0 |  | direction_flip_long | POSITION_OPEN |
| 461 | 空 | 505.84@394637 | 501.47@394646 | 9 | +0.86 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 462 | 空 | 508.80@396058 | 508.91@396076 | 18 | -0.02 | 0 | 1 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 463 | 空 | 512.80@399283 | 514.14@399323 | 40 | -0.13 | 1 | 3 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 464 | 空 | 517.88@400463 | 517.93@400467 | 4 | -0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 465 | 空 | 517.86@400482 | 517.87@400483 | 1 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 466 | 空 | 517.63@400503 | 517.71@400510 | 7 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 467 | 空 | 519.66@400558 | 519.64@400566 | 8 | +0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 468 | 空 | 520.25@400692 | 520.55@400739 | 47 | -0.04 | 1 | 0 |  | direction_flip_long | POSITION_OPEN |
| 469 | 空 | 520.95@400798 | 521.23@400866 | 68 | -0.05 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 470 | 多 | 522.05@400899 | 521.48@400922 | 23 | -0.11 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 471 | 多 | 522.14@401076 | 522.50@401121 | 45 | +0.07 | 0 | 4 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 472 | 多 | 522.70@401145 | 523.08@401199 | 54 | +0.07 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 473 | 空 | 522.66@401392 | 523.44@401600 | 208 | -0.15 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 474 | 多 | 523.77@401761 | 523.71@401773 | 12 | -0.01 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 475 | 空 | 521.87@401871 | 522.41@401880 | 9 | -0.10 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 476 | 空 | 521.73@401957 | 522.01@402130 | 173 | -0.12 | 4 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 477 | 多 | 524.25@402376 | 525.61@402416 | 40 | +0.26 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 478 | 多 | 526.01@402576 | 526.38@402601 | 25 | +0.07 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 479 | 空 | 526.08@402614 | 526.30@402627 | 13 | -0.04 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 480 | 空 | 526.07@402634 | 525.78@402648 | 14 | +0.06 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 481 | 空 | 526.09@402716 | 526.06@402718 | 2 | +0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 482 | 空 | 525.40@402974 | 523.19@403805 | 831 | -0.12 | 6 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 483 | 空 | 519.68@404278 | 521.35@404577 | 299 | -0.16 | 1 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 484 | 空 | 527.69@404747 | 528.08@404759 | 12 | -0.07 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 485 | 空 | 527.37@404778 | 528.19@404794 | 16 | -0.16 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 486 | 空 | 528.94@404826 | 529.03@404832 | 6 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 487 | 空 | 529.52@404868 | 529.62@404869 | 1 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 488 | 空 | 529.22@404877 | 529.52@404884 | 7 | -0.06 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 489 | 空 | 529.96@404960 | 529.88@404963 | 3 | +0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 490 | 空 | 528.90@405214 | 530.40@406140 | 926 | -0.29 | 8 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 491 | 多 | 530.55@406168 | 531.34@406282 | 114 | +0.15 | 0 | 7 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 492 | 空 | 530.48@406315 | 530.71@406334 | 19 | -0.04 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 493 | 多 | 533.59@407121 | 534.19@407171 | 50 | +0.11 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 494 | 多 | 535.64@407207 | 535.96@407245 | 38 | +0.06 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 495 | 多 | 538.41@407372 | 538.08@407391 | 19 | -0.06 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 496 | 空 | 538.65@407484 | 538.70@407494 | 10 | -0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 497 | 空 | 538.00@407571 | 537.88@407633 | 62 | +0.02 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 498 | 空 | 536.68@407856 | 534.83@408004 | 148 | +0.34 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 499 | 空 | 532.72@409027 | 522.84@410659 | 1,632 | -2.10 | 10 | 108 | Y | bsp_invalidate | COST_REDUCING→POSITION_OPEN→PRINCIPAL_WITHDRAWN |
| 500 | 空 | 504.91@420177 | 504.65@420185 | 8 | +0.05 | 0 | 1 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 501 | 空 | 521.50@424286 | 521.32@424290 | 4 | +0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 502 | 空 | 525.77@425515 | 528.90@425690 | 175 | -0.32 | 2 | 12 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 503 | 空 | 529.80@425940 | 529.54@425969 | 29 | +0.05 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 504 | 多 | 540.63@440493 | 540.40@440525 | 32 | -0.04 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 505 | 多 | 537.29@441080 | 535.92@442387 | 1,307 | +1.45 | 0 | 94 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 506 | 空 | 539.00@443061 | 538.99@443067 | 6 | +0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 507 | 空 | 532.48@443217 | 528.32@443426 | 209 | +0.44 | 3 | 0 |  | direction_flip_long | POSITION_OPEN |
| 508 | 空 | 499.58@447442 | 510.13@448111 | 669 | -0.93 | 3 | 48 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 509 | 空 | 494.46@448731 | 493.10@449196 | 465 | +0.22 | 4 | 35 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 510 | 多 | 487.13@451295 | 487.98@451316 | 21 | +0.17 | 0 | 0 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 511 | 多 | 453.46@469109 | 453.42@469112 | 3 | -0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 512 | 多 | 425.49@470276 | 427.29@470305 | 29 | +0.54 | 1 | 2 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 513 | 空 | 402.75@470586 | 422.84@471269 | 683 | -3.19 | 4 | 45 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 514 | 空 | 468.78@482612 | 468.68@482617 | 5 | +0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 515 | 空 | 506.45@492798 | 506.59@492812 | 14 | -0.03 | 0 | 0 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 516 | 空 | 515.88@494117 | 515.75@494122 | 5 | +0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 517 | 多 | 541.30@519792 | 541.66@519842 | 50 | +0.07 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 518 | 多 | 541.98@519858 | 541.90@519866 | 8 | -0.01 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 519 | 空 | 541.23@520004 | 540.47@520126 | 122 | +0.05 | 2 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 520 | 空 | 543.60@520495 | 543.45@520584 | 89 | +0.01 | 1 | 0 |  | direction_flip_long | POSITION_OPEN |
| 521 | 空 | 543.40@520727 | 542.92@520790 | 63 | +0.04 | 1 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 522 | 多 | 544.30@520847 | 543.55@520906 | 59 | -0.14 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 523 | 空 | 545.39@521045 | 545.48@521046 | 1 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 524 | 空 | 546.12@521093 | 546.10@521098 | 5 | +0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 525 | 空 | 545.89@521106 | 546.02@521114 | 8 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 526 | 多 | 546.66@521261 | 546.65@521319 | 58 | -0.00 | 0 | 6 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 527 | 多 | 548.24@521389 | 547.80@521485 | 96 | -0.05 | 0 | 6 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 528 | 空 | 548.73@521591 | 548.82@521593 | 2 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 529 | 空 | 547.70@521635 | 547.81@521644 | 9 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 530 | 空 | 547.27@521672 | 547.66@521677 | 5 | -0.07 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 531 | 多 | 549.85@521810 | 547.76@521929 | 119 | -0.21 | 0 | 7 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 532 | 空 | 545.50@521982 | 551.06@522221 | 239 | -1.03 | 1 | 0 |  | direction_flip_long | POSITION_OPEN |
| 533 | 空 | 551.18@522242 | 551.70@522296 | 54 | -0.09 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 534 | 空 | 551.95@522323 | 551.60@522333 | 10 | +0.06 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 535 | 空 | 551.94@522370 | 552.00@522379 | 9 | -0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 536 | 空 | 550.69@522548 | 550.39@522554 | 6 | +0.05 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 537 | 空 | 550.03@522597 | 551.31@522916 | 319 | -0.24 | 2 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 538 | 多 | 552.24@522928 | 551.46@522940 | 12 | -0.14 | 0 | 0 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 539 | 空 | 550.91@524759 | 550.92@524765 | 6 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 540 | 多 | 553.09@525134 | 555.89@525215 | 81 | +0.54 | 0 | 4 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 541 | 空 | 556.69@525306 | 556.82@525308 | 2 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 542 | 空 | 556.35@525337 | 556.06@525345 | 8 | +0.05 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 543 | 空 | 555.80@525361 | 555.95@525371 | 10 | -0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 544 | 空 | 553.47@525549 | 553.76@525873 | 324 | -0.01 | 3 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 545 | 空 | 552.59@525954 | 551.34@526211 | 257 | -0.01 | 4 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 546 | 空 | 559.00@530720 | 559.78@530799 | 79 | -0.14 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 547 | 多 | 559.80@530848 | 559.43@530870 | 22 | -0.06 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 548 | 多 | 560.47@531015 | 560.08@531027 | 12 | -0.07 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 549 | 空 | 557.63@531232 | 558.04@531360 | 128 | +0.05 | 2 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 550 | 空 | 557.58@531376 | 555.65@531529 | 153 | +0.20 | 2 | 0 |  | direction_flip_long | POSITION_OPEN |
| 551 | 多 | 555.43@531965 | 551.86@532051 | 86 | -0.53 | 0 | 6 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 552 | 多 | 560.78@532889 | 561.60@532962 | 73 | +0.15 | 0 | 5 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 553 | 空 | 561.49@533076 | 562.30@533336 | 260 | -0.15 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 554 | 多 | 562.57@533358 | 562.27@533367 | 9 | -0.05 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 555 | 多 | 562.65@533377 | 562.63@533439 | 62 | -0.00 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 556 | 空 | 562.19@533700 | 563.13@534178 | 478 | -0.35 | 2 | 0 |  | direction_flip_long | POSITION_OPEN |
| 557 | 多 | 565.70@534536 | 565.80@534553 | 17 | +0.02 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 558 | 多 | 566.03@534729 | 565.94@534731 | 2 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 559 | 空 | 564.48@534859 | 562.93@535167 | 308 | +0.01 | 4 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 560 | 空 | 562.27@535359 | 560.62@535768 | 409 | +0.05 | 4 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 561 | 多 | 566.26@538016 | 566.06@538064 | 48 | -0.02 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 562 | 多 | 569.53@538459 | 569.31@538469 | 10 | -0.04 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 563 | 空 | 569.22@538487 | 569.30@538488 | 1 | -0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 564 | 空 | 568.75@538602 | 568.50@538611 | 9 | +0.04 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 565 | 空 | 568.12@538690 | 568.08@538766 | 76 | +0.01 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 566 | 多 | 570.48@539304 | 570.55@539333 | 29 | +0.01 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 567 | 空 | 570.50@539343 | 570.46@539352 | 9 | +0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 568 | 多 | 570.70@539405 | 570.79@539426 | 21 | +0.02 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 569 | 空 | 570.82@539444 | 570.97@539455 | 11 | -0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 570 | 空 | 570.73@539486 | 570.85@539493 | 7 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 571 | 空 | 566.85@539980 | 568.25@540232 | 252 | -0.10 | 3 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 572 | 多 | 573.04@540952 | 573.20@540972 | 20 | +0.03 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 573 | 空 | 574.01@541069 | 576.41@541071 | 2 | -0.42 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 574 | 空 | 575.83@541115 | 575.92@541116 | 1 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 575 | 多 | 575.60@541176 | 575.40@541182 | 6 | -0.03 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 576 | 空 | 575.38@541201 | 575.50@541203 | 2 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 577 | 空 | 575.65@541229 | 575.70@541231 | 2 | -0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 578 | 空 | 575.14@541286 | 559.39@542227 | 941 | -0.04 | 12 | 14 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 579 | 空 | 551.81@542628 | 558.37@543139 | 511 | -0.52 | 3 | 37 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 580 | 空 | 563.51@543483 | 563.51@543502 | 19 | -0.00 | 0 | 2 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 581 | 空 | 567.34@545440 | 567.24@545446 | 6 | +0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 582 | 空 | 574.40@547460 | 574.02@547467 | 7 | +0.07 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 583 | 多 | 576.86@548602 | 574.50@548673 | 71 | -0.22 | 0 | 6 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 584 | 多 | 577.50@548755 | 577.82@548770 | 15 | +0.06 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 585 | 多 | 578.17@548800 | 579.07@548865 | 65 | +0.16 | 0 | 6 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 586 | 空 | 579.11@548891 | 579.16@548896 | 5 | -0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 587 | 空 | 579.42@548930 | 579.52@548932 | 2 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 588 | 空 | 579.44@548956 | 579.33@548971 | 15 | +0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 589 | 空 | 579.24@548996 | 579.33@548998 | 2 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 590 | 空 | 579.72@549027 | 579.70@549030 | 3 | +0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 591 | 多 | 580.37@549098 | 581.65@549260 | 162 | +0.29 | 0 | 9 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 592 | 空 | 581.47@549317 | 581.52@549323 | 6 | -0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 593 | 空 | 581.35@549417 | 582.06@549477 | 60 | -0.12 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 594 | 空 | 580.40@549594 | 580.65@549608 | 14 | -0.04 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 595 | 空 | 580.07@549628 | 580.48@550642 | 1,014 | -0.11 | 6 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 596 | 空 | 576.76@551288 | 577.19@551294 | 6 | -0.07 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 597 | 空 | 576.24@551399 | 576.37@551951 | 552 | -0.01 | 5 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 598 | 空 | 575.54@552213 | 567.58@553587 | 1,374 | +0.08 | 12 | 0 |  | direction_flip_long | POSITION_OPEN |
| 599 | 多 | 583.50@566173 | 583.62@566203 | 30 | +0.03 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 600 | 空 | 583.87@566231 | 582.62@566236 | 5 | +0.21 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 601 | 空 | 581.52@566405 | 582.25@566803 | 398 | -0.13 | 5 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 602 | 多 | 584.64@567216 | 584.37@567227 | 11 | -0.05 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 603 | 多 | 584.85@567478 | 583.62@567684 | 206 | -0.01 | 0 | 12 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 604 | 空 | 585.14@567970 | 585.18@567974 | 4 | -0.01 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 605 | 多 | 586.70@568064 | 586.32@568066 | 2 | -0.06 | 0 | 0 |  | direction_flip_short | POSITION_OPEN |
| 606 | 多 | 586.71@568157 | 586.38@568219 | 62 | -0.03 | 0 | 4 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 607 | 空 | 587.66@568735 | 588.00@568745 | 10 | -0.06 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 608 | 多 | 588.74@568809 | 589.90@568859 | 50 | +0.20 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 609 | 多 | 590.43@568885 | 590.86@568959 | 74 | +0.08 | 0 | 4 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 610 | 多 | 591.13@569085 | 590.12@569122 | 37 | -0.14 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 611 | 空 | 593.51@569362 | 593.63@569374 | 12 | -0.01 | 1 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 612 | 空 | 593.68@569398 | 593.80@569400 | 2 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 613 | 空 | 593.25@569516 | 593.24@569526 | 10 | +0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 614 | 空 | 592.73@569590 | 592.83@569613 | 23 | -0.01 | 1 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 615 | 多 | 590.98@569795 | 591.40@569897 | 102 | +0.09 | 0 | 9 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 616 | 多 | 590.69@570218 | 589.69@570510 | 292 | -0.08 | 0 | 19 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 617 | 空 | 588.39@570589 | 587.03@570822 | 233 | +0.21 | 4 | 0 |  | direction_flip_long | POSITION_OPEN |
| 618 | 空 | 595.95@571134 | 596.02@571135 | 1 | -0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 619 | 空 | 595.65@571215 | 595.53@571230 | 15 | +0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 620 | 空 | 595.42@571235 | 595.35@571246 | 11 | +0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 621 | 多 | 596.71@571334 | 595.88@571360 | 26 | -0.12 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 622 | 空 | 594.00@571409 | 596.92@571447 | 38 | -0.49 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 623 | 多 | 596.77@571490 | 596.19@571541 | 51 | -0.10 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 624 | 空 | 597.65@572312 | 597.36@572323 | 11 | +0.05 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 625 | 空 | 598.14@572454 | 598.53@572465 | 11 | -0.07 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 626 | 多 | 598.30@572512 | 599.39@572710 | 198 | +0.26 | 0 | 10 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 627 | 空 | 598.25@573179 | 600.03@573239 | 60 | -0.30 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 628 | 多 | 600.77@573329 | 601.68@573367 | 38 | +0.15 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 629 | 空 | 601.62@573395 | 601.97@573402 | 7 | -0.06 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 630 | 空 | 601.80@573413 | 602.10@573424 | 11 | -0.05 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 631 | 空 | 602.42@573490 | 602.41@573496 | 6 | +0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 632 | 空 | 602.53@573527 | 602.65@573531 | 4 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 633 | 空 | 601.99@573560 | 601.95@573574 | 14 | +0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 634 | 空 | 601.76@573681 | 601.76@573691 | 10 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 635 | 多 | 602.84@573742 | 602.72@573753 | 11 | -0.02 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 636 | 空 | 600.54@574198 | 596.53@575487 | 1,289 | -0.04 | 8 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 637 | 空 | 593.26@575670 | 593.48@575989 | 319 | -0.13 | 3 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 638 | 多 | 603.73@579635 | 603.41@579643 | 8 | -0.05 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 639 | 空 | 603.30@579814 | 605.00@579856 | 42 | -0.28 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 640 | 空 | 605.41@579943 | 605.29@579949 | 6 | +0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 641 | 空 | 606.23@580073 | 606.54@580088 | 15 | -0.05 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 642 | 空 | 606.78@580138 | 606.95@580153 | 15 | -0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 643 | 空 | 606.50@580185 | 604.55@580215 | 30 | +0.32 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 644 | 空 | 603.65@580269 | 603.98@580282 | 13 | -0.05 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 645 | 空 | 605.77@580551 | 607.73@580731 | 180 | -0.32 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 646 | 空 | 607.50@580809 | 607.58@580816 | 7 | -0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 647 | 空 | 607.78@580862 | 607.81@580866 | 4 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 648 | 空 | 607.04@580924 | 607.15@580926 | 2 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 649 | 空 | 602.92@581317 | 603.55@581365 | 48 | -0.13 | 1 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 650 | 空 | 608.87@582238 | 609.11@582244 | 6 | -0.04 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 651 | 多 | 609.38@582800 | 609.36@582817 | 17 | -0.00 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 652 | 多 | 609.15@582831 | 609.25@582846 | 15 | +0.02 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 653 | 空 | 606.16@582929 | 605.33@582932 | 3 | +0.14 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 654 | 空 | 609.96@584053 | 610.27@584070 | 17 | -0.05 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 655 | 多 | 611.65@584143 | 612.04@584169 | 26 | +0.07 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 656 | 空 | 611.93@584183 | 612.06@584184 | 1 | -0.02 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 657 | 空 | 611.75@584193 | 611.79@584196 | 3 | -0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 658 | 空 | 611.70@584304 | 611.72@584311 | 7 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 659 | 空 | 611.11@584338 | 608.23@584881 | 543 | +0.25 | 4 | 0 |  | direction_flip_long | POSITION_OPEN |
| 660 | 多 | 613.11@594218 | 613.35@594265 | 47 | +0.05 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 661 | 空 | 613.68@594363 | 613.69@594375 | 12 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 662 | 多 | 616.85@594525 | 616.85@594572 | 47 | +0.00 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 663 | 多 | 617.72@594634 | 617.51@594644 | 10 | -0.03 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 664 | 多 | 617.94@594690 | 617.71@594793 | 103 | -0.03 | 0 | 5 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 665 | 空 | 617.62@594835 | 617.94@594853 | 18 | -0.05 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 666 | 空 | 623.83@595085 | 624.16@595108 | 23 | -0.05 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 667 | 多 | 624.26@595126 | 624.07@595147 | 21 | -0.03 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 668 | 空 | 624.93@595192 | 624.96@595195 | 3 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 669 | 空 | 625.22@595236 | 625.20@595240 | 4 | +0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 670 | 空 | 624.93@595254 | 624.97@595256 | 2 | -0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 671 | 空 | 624.67@595286 | 624.72@595299 | 13 | -0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 672 | 空 | 625.36@595432 | 625.83@595452 | 20 | -0.08 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 673 | 多 | 626.40@595507 | 626.36@595536 | 29 | -0.01 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 674 | 空 | 627.26@595692 | 627.18@595698 | 6 | +0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 675 | 空 | 627.32@595728 | 627.53@595730 | 2 | -0.03 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 676 | 空 | 627.14@595737 | 627.21@595745 | 8 | -0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 677 | 多 | 628.65@595798 | 629.06@595838 | 40 | +0.07 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 678 | 空 | 628.93@595869 | 629.06@595870 | 1 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 679 | 空 | 628.90@595886 | 628.87@595891 | 5 | +0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 680 | 空 | 628.10@595972 | 627.88@595994 | 22 | +0.04 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 681 | 空 | 629.08@596206 | 628.98@596211 | 5 | +0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 682 | 多 | 629.40@596229 | 630.25@596289 | 60 | +0.14 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 683 | 空 | 630.08@596320 | 630.53@596321 | 1 | -0.07 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 684 | 多 | 632.41@596546 | 633.43@596611 | 65 | +0.16 | 0 | 5 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 685 | 多 | 634.17@596656 | 634.13@596670 | 14 | -0.01 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 686 | 多 | 636.15@596884 | 636.10@596909 | 25 | +0.02 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 687 | 空 | 635.25@596953 | 636.72@597240 | 287 | -0.23 | 1 | 0 |  | direction_flip_long | POSITION_OPEN |
| 688 | 空 | 634.27@597367 | 636.94@597604 | 237 | -0.46 | 1 | 0 |  | direction_flip_long | POSITION_OPEN |
| 689 | 空 | 630.57@598166 | 631.43@598333 | 167 | -0.25 | 2 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 690 | 空 | 626.25@598550 | 634.05@598857 | 307 | -0.49 | 2 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 691 | 空 | 624.12@600521 | 617.89@601417 | 896 | +0.17 | 11 | 9 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 692 | 空 | 615.47@601536 | 619.32@601722 | 186 | -0.30 | 2 | 12 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 693 | 空 | 614.78@602681 | 613.94@602700 | 19 | +0.14 | 0 | 0 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 694 | 空 | 612.80@602723 | 611.81@602783 | 60 | -0.02 | 2 | 7 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 695 | 空 | 610.36@603254 | 599.57@603660 | 406 | +0.31 | 10 | 31 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 696 | 空 | 597.42@609573 | 599.17@610202 | 629 | -0.45 | 5 | 42 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 697 | 空 | 587.72@612147 | 583.69@612342 | 195 | +0.45 | 2 | 13 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 698 | 空 | 623.56@617964 | 623.68@617972 | 8 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 699 | 空 | 623.56@617989 | 623.30@617994 | 5 | +0.04 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 700 | 空 | 636.60@648683 | 636.73@648685 | 2 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 701 | 多 | 596.02@654754 | 600.21@654802 | 48 | +0.67 | 1 | 0 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 702 | 空 | 580.27@680908 | 586.44@681052 | 144 | -0.64 | 1 | 10 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 703 | 空 | 574.51@684618 | 570.44@685064 | 446 | +0.53 | 4 | 32 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 704 | 空 | 558.81@686452 | 563.45@686766 | 314 | -1.17 | 3 | 21 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 705 | 空 | 586.66@688197 | 586.99@688203 | 6 | -0.06 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 706 | 空 | 611.21@693388 | 611.21@693390 | 2 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 707 | 空 | 627.73@695666 | 627.76@695676 | 10 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 708 | 空 | 631.94@696319 | 632.19@696321 | 2 | -0.04 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 709 | 空 | 632.80@696342 | 632.66@696346 | 4 | +0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 710 | 多 | 636.96@696627 | 637.87@696779 | 152 | +0.17 | 0 | 8 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 711 | 多 | 638.03@696781 | 637.86@696799 | 18 | -0.03 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 712 | 多 | 638.14@696809 | 638.96@696857 | 48 | +0.13 | 0 | 4 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 713 | 多 | 639.82@697249 | 640.15@697273 | 24 | +0.05 | 0 | 0 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 714 | 多 | 641.37@697297 | 641.29@697323 | 26 | -0.01 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 715 | 空 | 639.17@697375 | 638.81@697477 | 102 | +0.01 | 1 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 716 | 空 | 642.65@697975 | 643.21@697977 | 2 | -0.09 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 717 | 多 | 645.23@698001 | 645.44@698040 | 39 | +0.08 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 718 | 空 | 648.18@698128 | 649.39@698139 | 11 | -0.19 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 719 | 多 | 649.62@698164 | 648.44@698177 | 13 | -0.18 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 720 | 空 | 649.50@698248 | 649.51@698251 | 3 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 721 | 空 | 647.88@698274 | 648.15@698275 | 1 | -0.04 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 722 | 空 | 643.68@699051 | 643.55@699060 | 9 | +0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 723 | 空 | 646.08@699223 | 647.87@699454 | 231 | -0.12 | 3 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 724 | 多 | 650.33@699719 | 650.27@699731 | 12 | -0.01 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 725 | 多 | 648.52@699816 | 646.61@699925 | 109 | +0.00 | 0 | 7 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 726 | 多 | 652.92@700806 | 651.77@700828 | 22 | -0.18 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 727 | 多 | 653.17@700956 | 653.77@700992 | 36 | +0.10 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 728 | 空 | 653.67@701016 | 653.40@701044 | 28 | +0.04 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 729 | 空 | 654.08@701103 | 654.36@701107 | 4 | -0.04 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 730 | 空 | 655.36@701150 | 655.66@701159 | 9 | -0.05 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 731 | 空 | 651.69@701379 | 651.98@701437 | 58 | -0.10 | 1 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 732 | 空 | 655.54@701798 | 655.45@701801 | 3 | +0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 733 | 空 | 650.68@701891 | 654.31@702224 | 333 | -0.30 | 2 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 734 | 多 | 656.94@702398 | 657.50@702427 | 29 | +0.09 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 735 | 多 | 658.55@702444 | 660.00@702459 | 15 | +0.22 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 736 | 空 | 661.23@702475 | 660.80@702481 | 6 | +0.07 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 737 | 空 | 657.96@702588 | 662.75@702722 | 134 | -0.73 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 738 | 空 | 662.23@702753 | 662.82@702760 | 7 | -0.09 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 739 | 空 | 664.04@702939 | 663.87@703041 | 102 | +0.02 | 1 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 740 | 多 | 664.39@703221 | 664.32@703272 | 51 | +0.01 | 0 | 5 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 741 | 多 | 665.43@703343 | 665.10@703361 | 18 | -0.05 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 742 | 空 | 662.64@703471 | 665.67@704021 | 550 | -0.37 | 1 | 0 |  | direction_flip_long | POSITION_OPEN |
| 743 | 空 | 660.71@704129 | 658.21@704789 | 660 | -0.13 | 5 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 744 | 多 | 665.70@705873 | 662.32@705894 | 21 | -0.51 | 0 | 0 |  | direction_flip_short | POSITION_OPEN |
| 745 | 空 | 661.35@706229 | 660.34@706243 | 14 | +0.15 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 746 | 空 | 667.05@706516 | 667.00@706519 | 3 | +0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 747 | 空 | 666.93@706550 | 667.56@706561 | 11 | -0.09 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 748 | 空 | 668.49@706733 | 668.51@706736 | 3 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 749 | 空 | 668.17@706764 | 668.40@706766 | 2 | -0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 750 | 空 | 666.94@706974 | 669.91@707143 | 169 | -0.45 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 751 | 空 | 674.64@707191 | 675.04@707192 | 1 | -0.06 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 752 | 空 | 672.57@707209 | 672.99@707211 | 2 | -0.06 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 753 | 多 | 676.00@707750 | 676.09@707821 | 71 | +0.02 | 0 | 9 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 754 | 空 | 675.85@707831 | 671.57@707844 | 13 | +0.63 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 755 | 多 | 676.33@708115 | 676.00@708129 | 14 | -0.05 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 756 | 空 | 671.26@708231 | 676.71@708691 | 460 | -0.72 | 2 | 0 |  | direction_flip_long | POSITION_OPEN |
| 757 | 多 | 676.90@708707 | 676.70@708724 | 17 | -0.03 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 758 | 空 | 676.61@708727 | 676.78@708745 | 18 | -0.03 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 759 | 空 | 676.16@708788 | 676.30@708790 | 2 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 760 | 多 | 677.16@708852 | 677.66@708890 | 38 | +0.08 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 761 | 多 | 678.22@708914 | 679.80@708979 | 65 | +0.26 | 0 | 4 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 762 | 多 | 681.23@708991 | 680.82@709003 | 12 | -0.06 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 763 | 空 | 681.28@709083 | 681.70@709093 | 10 | -0.06 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 764 | 空 | 680.75@709113 | 680.93@709120 | 7 | -0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 765 | 空 | 682.40@709230 | 682.49@709244 | 14 | -0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 766 | 空 | 682.29@709257 | 682.26@709268 | 11 | +0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 767 | 空 | 681.86@709315 | 681.67@709332 | 17 | +0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 768 | 多 | 684.36@709445 | 686.30@709583 | 138 | +0.30 | 0 | 10 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 769 | 空 | 690.14@709668 | 690.77@709679 | 11 | -0.09 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 770 | 空 | 690.52@709691 | 690.89@709693 | 2 | -0.05 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 771 | 多 | 692.25@709723 | 692.88@709766 | 43 | +0.09 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 772 | 空 | 688.47@709869 | 688.19@709872 | 3 | +0.04 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 773 | 多 | 694.40@710233 | 694.61@710255 | 22 | +0.03 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 774 | 空 | 694.49@710272 | 694.99@710273 | 1 | -0.07 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 775 | 多 | 700.27@710889 | 699.36@710915 | 26 | -0.08 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 776 | 空 | 693.54@711048 | 694.27@711358 | 310 | -0.15 | 3 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 777 | 多 | 701.26@711694 | 701.55@711707 | 13 | +0.04 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 778 | 多 | 705.89@711802 | 704.79@711811 | 9 | -0.16 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 779 | 空 | 704.75@711817 | 705.19@711822 | 5 | -0.06 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 780 | 多 | 707.21@711862 | 708.02@711936 | 74 | +0.11 | 0 | 7 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 781 | 多 | 710.02@711982 | 709.97@711994 | 12 | -0.01 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 782 | 多 | 710.35@711996 | 709.57@712004 | 8 | -0.11 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 783 | 多 | 710.82@712135 | 710.95@712160 | 25 | +0.02 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 784 | 空 | 712.00@712280 | 712.14@712297 | 17 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 785 | 空 | 710.31@712730 | 713.44@712804 | 74 | -0.45 | 1 | 0 |  | direction_flip_long | POSITION_OPEN |
| 786 | 空 | 714.00@712858 | 712.98@712866 | 8 | +0.14 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 787 | 空 | 707.39@713423 | 707.20@713516 | 93 | +0.00 | 2 | 0 |  | direction_flip_long | POSITION_OPEN |
| 788 | 空 | 705.89@713602 | 706.09@713969 | 367 | -0.46 | 4 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 789 | 空 | 713.44@714199 | 714.55@714715 | 516 | -0.79 | 6 | 0 |  | direction_flip_long | POSITION_OPEN |
| 790 | 空 | 714.14@714770 | 713.96@714773 | 3 | +0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 791 | 空 | 714.27@714883 | 714.44@714887 | 4 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 792 | 多 | 718.44@715066 | 718.20@715075 | 9 | -0.03 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 793 | 空 | 717.24@715111 | 719.08@715449 | 338 | -0.50 | 3 | 0 |  | direction_flip_long | POSITION_OPEN |
| 794 | 多 | 719.32@715502 | 721.12@715558 | 56 | +0.25 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 795 | 空 | 720.81@715610 | 720.89@715627 | 17 | -0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 796 | 多 | 711.96@716032 | 705.55@717097 | 1,065 | +1.75 | 0 | 75 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 797 | 空 | 706.80@718924 | 706.90@718926 | 2 | -0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 798 | 多 | 723.31@721606 | 723.00@721611 | 5 | -0.04 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 799 | 多 | 724.12@721654 | 725.35@721738 | 84 | +0.17 | 0 | 6 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 800 | 空 | 724.57@721789 | 724.89@721801 | 12 | -0.04 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 801 | 空 | 725.08@721814 | 725.23@721819 | 5 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 802 | 空 | 726.20@721875 | 726.25@721879 | 4 | -0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 803 | 多 | 726.72@721919 | 730.10@721989 | 70 | +0.52 | 0 | 5 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 804 | 空 | 728.01@722025 | 727.71@722038 | 13 | +0.04 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 805 | 空 | 733.19@722546 | 733.20@722549 | 3 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 806 | 空 | 732.80@722578 | 733.03@722584 | 6 | -0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 807 | 空 | 735.15@722701 | 735.12@722705 | 4 | +0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 808 | 空 | 734.94@722719 | 734.75@722724 | 5 | +0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 809 | 空 | 725.75@722868 | 731.15@723256 | 388 | -0.32 | 2 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 810 | 空 | 725.51@723580 | 729.97@723730 | 150 | -0.32 | 2 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 811 | 多 | 737.55@724514 | 739.69@724687 | 173 | +0.44 | 0 | 8 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 812 | 多 | 742.33@725750 | 744.92@725823 | 73 | +0.40 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 813 | 多 | 744.36@725883 | 743.80@725908 | 25 | -0.08 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 814 | 空 | 741.80@725980 | 741.95@725982 | 2 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 815 | 多 | 745.62@726615 | 745.20@726640 | 25 | -0.06 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 816 | 多 | 745.16@726670 | 744.53@726705 | 35 | -0.08 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 817 | 空 | 746.76@726944 | 746.88@726946 | 2 | -0.02 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 818 | 空 | 746.62@726972 | 746.58@726983 | 11 | +0.01 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 819 | 空 | 746.48@727102 | 746.22@727107 | 5 | +0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 820 | 空 | 747.00@727163 | 747.13@727187 | 24 | +0.01 | 1 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 821 | 空 | 746.91@727196 | 746.70@727200 | 4 | +0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 822 | 空 | 746.43@727408 | 746.86@727413 | 5 | -0.06 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 823 | 空 | 741.33@727836 | 740.83@727858 | 22 | +0.07 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 824 | 空 | 739.38@727883 | 738.28@728029 | 146 | +0.08 | 1 | 0 |  | eod_close | POSITION_OPEN |

### FSM 状态分布

| 状态 | Bars | 占比 |
|------|------|------|
| SCANNING | 654,814 | 89.9% |
| POSITION_OPEN | 55,755 | 7.7% |
| COST_REDUCING | 17,414 | 2.4% |
| PRINCIPAL_WITHDRAWN | 47 | 0.0% |
| STOPPED_OUT | 0 | 0.0% |

<details><summary>事件流（7283条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 146 | 268.60 | POSITION_OPEN | ENTRY_SHORT@268.60 |
| 154 | 267.56 | COST_REDUCING | TRIM@267.56 s=349.3 r=0.938 |
| 155 | 267.60 | SCANNING | STOP:direction_flip_long@267.60 pnl=+0.37% |
| 225 | 268.47 | POSITION_OPEN | ENTRY_LONG@268.47 |
| 229 | 268.41 | COST_REDUCING | TRIM@268.41 s=201.8 r=0.542 |
| 241 | 267.95 | POSITION_OPEN | CLOSE_DIFF@267.95 p=92.81 |
| 255 | 267.93 | COST_REDUCING | TRIM@267.93 s=201.8 r=0.542 |
| 256 | 268.10 | POSITION_OPEN | CLOSE_DIFF@268.10 p=-34.30 |
| 267 | 268.31 | COST_REDUCING | TRIM@268.31 s=201.8 r=0.542 |
| 268 | 268.42 | POSITION_OPEN | CLOSE_DIFF@268.42 p=-22.19 |
| 283 | 268.59 | COST_REDUCING | TRIM@268.59 s=232.8 r=0.625 |
| 292 | 268.67 | POSITION_OPEN | CLOSE_DIFF@268.67 p=-18.62 |
| 305 | 268.92 | COST_REDUCING | TRIM@268.92 s=270.9 r=0.727 |
| 312 | 268.78 | POSITION_OPEN | CLOSE_DIFF@268.78 p=37.93 |
| 313 | 268.67 | COST_REDUCING | TRIM@268.67 s=378.6 r=0.727 |
| 317 | 269.07 | POSITION_OPEN | CLOSE_DIFF@269.07 p=-153.35 |
| 329 | 268.55 | COST_REDUCING | TRIM@268.55 s=494.4 r=0.950 |
| 336 | 266.51 | SCANNING | STOP:direction_flip_short@266.51 pnl=-0.67% |
| 398 | 264.17 | POSITION_OPEN | ENTRY_SHORT@264.17 |
| 400 | 264.78 | COST_REDUCING | TRIM@264.78 s=356.3 r=0.941 |
| 401 | 264.96 | SCANNING | STOP:direction_flip_long@264.96 pnl=-0.30% |
| 488 | 262.43 | POSITION_OPEN | ENTRY_SHORT@262.43 |
| 495 | 263.10 | SCANNING | STOP:direction_flip_long@263.10 pnl=-0.26% |
| 808 | 266.36 | POSITION_OPEN | ENTRY_LONG@266.36 |
| 817 | 266.00 | COST_REDUCING | TRIM@266.00 s=200.0 r=0.533 |
| 821 | 266.17 | POSITION_OPEN | CLOSE_DIFF@266.17 p=-34.00 |
| 831 | 266.15 | COST_REDUCING | TRIM@266.15 s=200.0 r=0.533 |
| 835 | 266.00 | POSITION_OPEN | CLOSE_DIFF@266.00 p=30.00 |
| 836 | 265.77 | COST_REDUCING | TRIM@265.77 s=302.9 r=0.533 |
| 840 | 266.02 | POSITION_OPEN | CLOSE_DIFF@266.02 p=-75.72 |
| 846 | 265.95 | COST_REDUCING | TRIM@265.95 s=302.9 r=0.533 |
| 850 | 266.11 | POSITION_OPEN | CLOSE_DIFF@266.11 p=-48.46 |
| 855 | 266.12 | COST_REDUCING | TRIM@266.12 s=302.9 r=0.533 |
| 865 | 266.25 | POSITION_OPEN | CLOSE_DIFF@266.25 p=-39.37 |
| 876 | 266.19 | COST_REDUCING | TRIM@266.19 s=310.3 r=0.546 |
| 882 | 266.33 | POSITION_OPEN | CLOSE_DIFF@266.33 p=-43.45 |
| 888 | 266.24 | COST_REDUCING | TRIM@266.24 s=310.3 r=0.546 |
| 901 | 265.60 | POSITION_OPEN | CLOSE_DIFF@265.60 p=198.61 |
| 932 | 266.21 | COST_REDUCING | TRIM@266.21 s=310.3 r=0.546 |
| 939 | 266.30 | POSITION_OPEN | CLOSE_DIFF@266.30 p=-27.93 |
| 941 | 266.10 | COST_REDUCING | TRIM@266.10 s=310.3 r=0.546 |
| 945 | 266.48 | POSITION_OPEN | CLOSE_DIFF@266.48 p=-117.92 |
| 948 | 266.40 | COST_REDUCING | TRIM@266.40 s=311.8 r=0.548 |
| 953 | 266.40 | POSITION_OPEN | CLOSE_DIFF@266.40 p=0.00 |
| 959 | 266.33 | SCANNING | STOP:direction_flip_short@266.33 pnl=+0.12% |
| 1,236 | 266.98 | POSITION_OPEN | ENTRY_LONG@266.98 |
| 1,243 | 267.31 | SCANNING | STOP:bsp_invalidate@267.31 pnl=+0.12% |
| 1,256 | 267.36 | POSITION_OPEN | ENTRY_LONG@267.36 |
| 1,259 | 267.11 | COST_REDUCING | TRIM@267.11 s=248.2 r=0.664 |
| 1,265 | 266.87 | POSITION_OPEN | CLOSE_DIFF@266.87 p=60.81 |
| 1,266 | 266.81 | COST_REDUCING | TRIM@266.81 s=338.5 r=0.664 |
| 1,273 | 266.84 | POSITION_OPEN | CLOSE_DIFF@266.84 p=-10.16 |
| 1,286 | 267.11 | COST_REDUCING | TRIM@267.11 s=338.5 r=0.664 |
| 1,291 | 266.92 | POSITION_OPEN | CLOSE_DIFF@266.92 p=64.32 |
| 1,308 | 267.12 | COST_REDUCING | TRIM@267.12 s=338.5 r=0.664 |
| 1,328 | 265.80 | POSITION_OPEN | CLOSE_DIFF@265.80 p=446.85 |
| 1,343 | 266.42 | COST_REDUCING | TRIM@266.42 s=338.5 r=0.664 |
| 1,344 | 266.57 | POSITION_OPEN | CLOSE_DIFF@266.57 p=-52.47 |
| 1,348 | 266.50 | COST_REDUCING | TRIM@266.50 s=338.5 r=0.664 |
| 1,353 | 266.57 | POSITION_OPEN | CLOSE_DIFF@266.57 p=-23.70 |
| 1,362 | 266.58 | COST_REDUCING | TRIM@266.58 s=338.5 r=0.664 |
| 1,365 | 266.76 | POSITION_OPEN | CLOSE_DIFF@266.76 p=-60.93 |
| 1,366 | 266.73 | COST_REDUCING | TRIM@266.73 s=338.5 r=0.664 |
| 1,381 | 266.92 | POSITION_OPEN | CLOSE_DIFF@266.92 p=-64.32 |
| 1,382 | 266.15 | COST_REDUCING | TRIM@266.15 s=338.5 r=0.664 |
| 1,394 | 265.37 | POSITION_OPEN | CLOSE_DIFF@265.37 p=264.05 |
| 1,402 | 265.62 | COST_REDUCING | TRIM@265.62 s=338.5 r=0.664 |
| 1,415 | 264.15 | POSITION_OPEN | CLOSE_DIFF@264.15 p=497.63 |
| 1,420 | 264.20 | COST_REDUCING | TRIM@264.20 s=338.5 r=0.664 |
| 1,422 | 264.76 | POSITION_OPEN | CLOSE_DIFF@264.76 p=-189.57 |
| 1,431 | 264.36 | COST_REDUCING | TRIM@264.36 s=338.5 r=0.664 |
| 1,434 | 264.88 | POSITION_OPEN | CLOSE_DIFF@264.88 p=-176.03 |
| 1,446 | 265.89 | COST_REDUCING | TRIM@265.89 s=338.5 r=0.664 |
| 1,458 | 265.81 | POSITION_OPEN | CLOSE_DIFF@265.81 p=27.08 |
| 1,460 | 265.55 | COST_REDUCING | ADD@265.55 +416.6 r=0.817 |
| 1,465 | 265.05 | SCANNING | STOP:bsp_invalidate@265.05 pnl=+0.02% |
| 1,960 | 261.83 | POSITION_OPEN | ENTRY_SHORT@261.83 |
| 1,964 | 262.21 | COST_REDUCING | TRIM@262.21 s=357.2 r=0.935 |
| 1,965 | 261.74 | POSITION_OPEN | CLOSE_DIFF@261.74 p=-167.86 |
| 1,971 | 262.42 | COST_REDUCING | TRIM@262.42 s=352.0 r=0.922 |
| ... | ... | ... | （共7283条） |
</details>

## OKLO

- 数据：**333,613** bars (1min), bar_0 → bar_333612
- 价格：18.07 → 63.48
- Buy-and-hold: **+251.30%**
- 回测耗时：**46.7s** (7142 bars/s)

### 总体指标

| 指标 | 值 |
|------|-----|
| 交易数 | 174 (多70/空104) |
| 胜率 | 43.1% |
| 平均收益 | -1.285% |
| 复利累计 | -97.78% |
| 最大回撤 | -97.90% |
| 平均持仓 | 344 bars |
| 有加仓 | 59/174 |
| 有降成本 | 100/174 |
| 本金回收 | 4/174 |

### 交易明细

| # | 方向 | 入场 | 出场 | 持仓 | PnL% | 加仓 | 短差 | PW | 退出 | 状态 |
|---|------|------|------|------|------|------|------|-----|------|------|
| 1 | 空 | 14.60@201 | 13.28@272 | 71 | +5.52 | 1 | 4 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 2 | 空 | 11.14@403 | 11.53@434 | 31 | -4.08 | 1 | 3 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 3 | 空 | 8.91@512 | 9.38@523 | 11 | -5.33 | 0 | 1 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 4 | 空 | 8.51@540 | 8.56@553 | 13 | -0.59 | 0 | 0 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 5 | 多 | 8.88@624 | 8.94@640 | 16 | +0.68 | 0 | 0 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 6 | 多 | 9.49@755 | 9.25@777 | 22 | -2.16 | 1 | 2 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 7 | 多 | 9.99@1089 | 10.02@1535 | 446 | +0.11 | 4 | 29 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 8 | 多 | 10.62@1612 | 10.31@2202 | 590 | +0.15 | 3 | 39 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 9 | 多 | 9.77@2731 | 9.76@2737 | 6 | -0.10 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 10 | 空 | 7.86@3485 | 7.76@3555 | 70 | +1.15 | 0 | 2 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 11 | 空 | 7.80@3608 | 7.48@3663 | 55 | +4.10 | 0 | 1 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 12 | 空 | 7.30@3699 | 7.48@3717 | 18 | -2.47 | 0 | 0 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 13 | 空 | 7.22@3833 | 7.20@3849 | 16 | +0.28 | 0 | 0 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 14 | 多 | 7.35@3912 | 7.33@3913 | 1 | -0.27 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 15 | 多 | 7.33@4282 | 7.27@4296 | 14 | -0.82 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 16 | 多 | 8.05@4408 | 8.22@4698 | 290 | +0.94 | 3 | 18 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 17 | 多 | 6.92@23115 | 7.06@23144 | 29 | +2.02 | 0 | 0 |  | direction_flip_short | POSITION_OPEN |
| 18 | 多 | 6.39@28709 | 6.17@29164 | 455 | -4.18 | 1 | 0 |  | direction_flip_short | POSITION_OPEN |
| 19 | 空 | 6.08@29184 | 5.99@29211 | 27 | +1.56 | 0 | 1 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 20 | 空 | 5.96@29225 | 6.00@29368 | 143 | -1.93 | 0 | 8 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 21 | 空 | 5.76@29424 | 5.79@29433 | 9 | -0.52 | 0 | 0 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 22 | 空 | 5.63@29583 | 5.59@29653 | 70 | +0.71 | 0 | 3 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 23 | 空 | 5.49@29790 | 5.68@29808 | 18 | -3.46 | 0 | 0 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 24 | 多 | 5.92@29927 | 5.49@30521 | 594 | -5.93 | 3 | 0 |  | direction_flip_short | POSITION_OPEN |
| 25 | 多 | 5.45@30691 | 5.41@30696 | 5 | -0.73 | 0 | 0 |  | direction_flip_short | POSITION_OPEN |
| 26 | 空 | 5.53@30769 | 6.05@31075 | 306 | -13.16 | 0 | 21 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 27 | 多 | 5.87@31213 | 6.30@32067 | 854 | +0.36 | 8 | 46 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 28 | 空 | 6.70@32788 | 6.75@32795 | 7 | -0.82 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 29 | 多 | 8.08@33019 | 7.99@33234 | 215 | +0.13 | 3 | 16 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 30 | 空 | 8.39@33454 | 8.58@33463 | 9 | -2.26 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 31 | 多 | 12.22@40636 | 11.87@40677 | 41 | -0.62 | 2 | 3 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 32 | 空 | 13.05@41102 | 14.31@41135 | 33 | -9.66 | 0 | 0 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 33 | 多 | 16.86@41499 | 14.87@41983 | 484 | +8.40 | 5 | 28 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 34 | 多 | 21.98@43725 | 22.32@43779 | 54 | +1.52 | 0 | 4 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 35 | 多 | 19.93@44230 | 19.38@44232 | 2 | -2.76 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 36 | 空 | 18.60@45157 | 17.99@45204 | 47 | +3.23 | 1 | 2 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 37 | 多 | 23.08@47029 | 22.65@47051 | 22 | -1.86 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 38 | 多 | 24.60@47161 | 24.38@47178 | 17 | -0.89 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 39 | 空 | 24.29@47188 | 24.99@47196 | 8 | -2.88 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 40 | 空 | 24.16@47272 | 24.08@47290 | 18 | +0.33 | 0 | 0 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 41 | 空 | 23.50@47730 | 23.43@47860 | 130 | -2.06 | 3 | 9 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 42 | 空 | 24.44@48049 | 24.59@48162 | 113 | +0.02 | 1 | 5 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 43 | 多 | 25.46@48267 | 25.50@48299 | 32 | +0.16 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 44 | 多 | 25.78@48311 | 26.03@48331 | 20 | +0.97 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 45 | 多 | 27.00@48445 | 27.00@48461 | 16 | +0.00 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 46 | 空 | 24.86@48630 | 23.12@48866 | 236 | +3.57 | 3 | 17 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 47 | 空 | 21.54@50294 | 21.52@50307 | 13 | +0.09 | 0 | 1 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 48 | 空 | 23.07@52642 | 24.88@52718 | 76 | -5.22 | 1 | 6 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 49 | 空 | 27.91@53149 | 28.30@53154 | 5 | -1.40 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 50 | 多 | 24.57@53636 | 24.61@53645 | 9 | +0.16 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 51 | 多 | 21.96@54897 | 21.94@54901 | 4 | -0.09 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 52 | 空 | 18.57@56952 | 18.60@57128 | 176 | -1.04 | 2 | 17 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 53 | 空 | 24.38@59752 | 24.57@59760 | 8 | -0.80 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 54 | 空 | 17.60@68749 | 18.00@68783 | 34 | -2.31 | 2 | 5 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 55 | 空 | 21.12@69834 | 21.52@69843 | 9 | -1.89 | 0 | 0 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 56 | 空 | 25.97@75692 | 26.04@75696 | 4 | -0.27 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 57 | 空 | 25.94@75707 | 25.94@75710 | 3 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 58 | 空 | 26.65@75765 | 26.94@75778 | 13 | -1.09 | 0 | 0 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 59 | 多 | 28.75@76268 | 30.86@76366 | 98 | +7.34 | 0 | 7 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 60 | 多 | 31.54@76373 | 30.80@76394 | 21 | -1.57 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 61 | 空 | 29.94@76686 | 29.92@76701 | 15 | +0.07 | 0 | 0 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 62 | 空 | 29.36@76982 | 25.98@77837 | 855 | -1.61 | 9 | 61 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 63 | 多 | 32.89@82349 | 32.79@82384 | 35 | -0.30 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 64 | 多 | 34.24@82724 | 33.99@82730 | 6 | -0.73 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 65 | 空 | 33.90@82738 | 34.77@82756 | 18 | -2.57 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 66 | 多 | 34.92@82967 | 32.80@83127 | 160 | -3.46 | 1 | 10 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 67 | 空 | 35.50@83400 | 36.58@83433 | 33 | -3.04 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 68 | 多 | 39.85@83501 | 38.46@83512 | 11 | -3.49 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 69 | 空 | 37.70@84092 | 40.28@84145 | 53 | -6.84 | 1 | 5 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 70 | 多 | 42.63@84254 | 42.72@84289 | 35 | +0.20 | 0 | 4 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 71 | 空 | 36.25@84661 | 35.10@84791 | 130 | +2.61 | 0 | 7 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 72 | 多 | 33.64@84948 | 33.86@84952 | 4 | +0.65 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 73 | 空 | 37.11@86563 | 34.64@86612 | 49 | +4.82 | 1 | 3 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 74 | 空 | 39.10@87303 | 39.56@87307 | 4 | -1.18 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 75 | 多 | 43.70@88068 | 44.38@88126 | 58 | +1.56 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 76 | 空 | 43.00@88222 | 38.73@88484 | 262 | +3.67 | 4 | 16 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 77 | 多 | 45.42@89005 | 46.28@89025 | 20 | +1.89 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 78 | 空 | 46.54@89089 | 46.10@89103 | 14 | +0.95 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 79 | 空 | 45.79@89130 | 46.11@89133 | 3 | -0.70 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 80 | 空 | 46.40@89258 | 46.49@89268 | 10 | -0.19 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 81 | 空 | 46.07@89471 | 45.97@90098 | 627 | -0.17 | 4 | 35 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 82 | 多 | 48.34@90228 | 50.75@90253 | 25 | +4.99 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 83 | 多 | 52.70@90834 | 50.85@90837 | 3 | -3.51 | 0 | 0 |  | direction_flip_short | POSITION_OPEN |
| 84 | 空 | 47.51@90882 | 47.11@91174 | 292 | +1.70 | 2 | 18 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 85 | 多 | 52.92@91528 | 57.45@91552 | 24 | +8.56 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 86 | 空 | 54.40@91715 | 54.19@91730 | 15 | +0.39 | 0 | 0 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 87 | 多 | 53.98@91797 | 54.89@92285 | 488 | +6.44 | 4 | 31 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 88 | 空 | 53.21@92908 | 52.64@92980 | 72 | +0.31 | 2 | 4 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 89 | 空 | 51.28@93062 | 54.78@93874 | 812 | -8.05 | 5 | 56 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 90 | 空 | 46.68@96369 | 45.87@96407 | 38 | +1.74 | 0 | 2 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 91 | 空 | 43.88@96816 | 44.75@96857 | 41 | -2.00 | 1 | 4 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 92 | 空 | 43.52@97071 | 44.09@97073 | 2 | -1.31 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 93 | 空 | 37.54@98392 | 25.53@104696 | 6,304 | -31.35 | 8 | 433 | Y | direction_flip_long | COST_REDUCING→POSITION_OPEN→PRINCIPAL_WITHDRAWN |
| 94 | 多 | 25.13@104776 | 25.47@104786 | 10 | +1.35 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 95 | 空 | 21.15@114742 | 21.74@115200 | 458 | -6.92 | 3 | 28 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 96 | 空 | 39.67@137643 | 39.35@137652 | 9 | +0.81 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 97 | 空 | 40.08@137936 | 39.17@137943 | 7 | +2.27 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 98 | 多 | 59.17@150098 | 65.92@150167 | 69 | +11.41 | 0 | 4 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 99 | 空 | 65.43@150223 | 65.96@150226 | 3 | -0.81 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 100 | 多 | 67.82@150318 | 68.11@150458 | 140 | +1.16 | 0 | 9 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 101 | 空 | 64.93@150463 | 63.90@150470 | 7 | +1.59 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 102 | 空 | 62.98@151001 | 67.02@151205 | 204 | -3.44 | 1 | 16 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 103 | 空 | 62.44@151594 | 60.61@151782 | 188 | -0.12 | 5 | 9 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 104 | 多 | 72.26@152740 | 66.99@153083 | 343 | -0.69 | 2 | 27 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 105 | 多 | 61.89@154740 | 62.00@154748 | 8 | +0.18 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 106 | 空 | 58.73@155666 | 58.45@155729 | 63 | -0.22 | 2 | 4 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 107 | 空 | 52.22@161180 | 51.70@161819 | 639 | +0.13 | 4 | 49 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 108 | 多 | 76.13@173454 | 74.40@173490 | 36 | -2.15 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 109 | 多 | 76.58@173692 | 72.60@173883 | 191 | -2.81 | 0 | 13 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 110 | 多 | 73.00@174175 | 74.14@174216 | 41 | +1.56 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 111 | 多 | 77.17@174681 | 76.95@174824 | 143 | -0.25 | 0 | 12 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 112 | 多 | 77.97@174914 | 77.50@174933 | 19 | -0.60 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 113 | 空 | 72.16@175686 | 73.41@175868 | 182 | -1.78 | 3 | 10 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 114 | 空 | 77.16@176363 | 78.11@176581 | 218 | -1.73 | 1 | 14 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 115 | 空 | 77.46@176596 | 75.91@176604 | 8 | +2.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 116 | 多 | 78.49@177102 | 78.99@177136 | 34 | +0.64 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 117 | 空 | 76.42@177435 | 75.20@177566 | 131 | +0.64 | 2 | 9 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 118 | 空 | 70.63@177892 | 72.82@178032 | 140 | -2.75 | 1 | 8 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 119 | 空 | 83.25@179408 | 83.95@179412 | 4 | -0.84 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 120 | 多 | 85.09@180403 | 80.44@180912 | 509 | -1.02 | 0 | 34 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 121 | 空 | 79.79@180947 | 79.29@181518 | 571 | -0.20 | 3 | 43 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 122 | 空 | 67.32@186756 | 64.98@187214 | 458 | +0.57 | 5 | 28 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 123 | 多 | 86.20@199409 | 83.79@199422 | 13 | -2.80 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 124 | 多 | 88.82@199494 | 90.27@199522 | 28 | +1.63 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 125 | 多 | 91.31@199586 | 91.84@199607 | 21 | +0.58 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 126 | 空 | 91.01@199629 | 91.13@199630 | 1 | -0.13 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 127 | 空 | 92.03@199661 | 91.83@199667 | 6 | +0.22 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 128 | 多 | 94.72@199758 | 94.83@199781 | 23 | +0.11 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 129 | 空 | 96.60@200028 | 95.60@200047 | 19 | +1.04 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 130 | 空 | 90.92@200341 | 91.78@200416 | 75 | -0.81 | 1 | 5 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 131 | 空 | 90.23@201234 | 96.88@201476 | 242 | -7.57 | 0 | 16 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 132 | 多 | 97.47@201512 | 97.50@201529 | 17 | +0.03 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 133 | 多 | 98.03@201550 | 98.36@201599 | 49 | +0.34 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 134 | 空 | 98.32@201707 | 98.95@201759 | 52 | -0.50 | 1 | 0 |  | direction_flip_long | POSITION_OPEN |
| 135 | 空 | 100.80@201813 | 101.61@201856 | 43 | -1.21 | 1 | 0 |  | direction_flip_long | POSITION_OPEN |
| 136 | 空 | 104.46@201897 | 104.07@201904 | 7 | +0.37 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 137 | 多 | 105.73@201912 | 109.53@201986 | 74 | +3.59 | 0 | 4 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 138 | 多 | 108.68@202009 | 106.89@202078 | 69 | -1.51 | 0 | 4 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 139 | 空 | 105.97@202102 | 105.97@202112 | 10 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 140 | 多 | 111.47@202559 | 121.75@202655 | 96 | +12.23 | 0 | 5 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 141 | 多 | 126.55@202700 | 128.61@202732 | 32 | +1.63 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 142 | 空 | 129.40@202762 | 130.34@202764 | 2 | -0.73 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 143 | 空 | 131.88@202863 | 132.88@202895 | 32 | -0.76 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 144 | 空 | 134.90@202947 | 135.29@202950 | 3 | -0.29 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 145 | 空 | 136.84@202978 | 137.00@202980 | 2 | -0.12 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 146 | 空 | 132.20@203172 | 131.10@203258 | 86 | -0.05 | 3 | 5 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 147 | 空 | 129.66@203470 | 130.62@203487 | 17 | -0.74 | 0 | 2 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 148 | 多 | 138.11@203685 | 140.34@203841 | 156 | +1.66 | 0 | 11 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 149 | 空 | 137.99@204141 | 136.00@204146 | 5 | +1.44 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 150 | 空 | 134.73@204344 | 134.53@204348 | 4 | +0.15 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 151 | 多 | 143.05@204719 | 142.08@204752 | 33 | -0.68 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 152 | 多 | 143.60@204805 | 143.80@204827 | 22 | +0.14 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 153 | 空 | 143.60@204853 | 143.66@204855 | 2 | -0.04 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 154 | 空 | 142.11@205077 | 145.25@211969 | 6,892 | -39.06 | 11 | 462 | Y | direction_flip_long | COST_REDUCING→POSITION_OPEN→PRINCIPAL_WITHDRAWN |
| 155 | 空 | 133.21@212888 | 135.38@214199 | 1,311 | -0.90 | 5 | 84 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 156 | 多 | 147.22@215431 | 157.09@215488 | 57 | +7.40 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 157 | 空 | 147.20@215808 | 146.72@215837 | 29 | +0.03 | 1 | 2 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 158 | 空 | 171.18@216387 | 167.90@216414 | 27 | +1.64 | 1 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 159 | 多 | 174.22@216683 | 174.40@216698 | 15 | +0.11 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 160 | 空 | 173.06@216724 | 173.50@216762 | 38 | -0.62 | 1 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 161 | 多 | 175.66@217291 | 176.27@217368 | 77 | +1.72 | 0 | 6 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 162 | 多 | 178.65@217406 | 179.44@217427 | 21 | +0.44 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 163 | 多 | 182.77@217471 | 182.00@217528 | 57 | -0.24 | 0 | 6 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 164 | 空 | 178.88@217581 | 183.50@217784 | 203 | -3.69 | 1 | 15 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 165 | 多 | 188.22@218104 | 184.99@218151 | 47 | -1.00 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 166 | 空 | 169.55@218286 | 175.17@218672 | 386 | -1.72 | 4 | 24 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 167 | 空 | 161.12@219108 | 113.16@230023 | 10,915 | -15.32 | 8 | 761 | Y | direction_flip_long | COST_REDUCING→POSITION_OPEN→PRINCIPAL_WITHDRAWN |
| 168 | 空 | 108.00@231303 | 103.15@231835 | 532 | +0.66 | 7 | 41 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 169 | 空 | 101.82@231849 | 102.74@231876 | 27 | -0.90 | 0 | 1 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 170 | 空 | 100.51@231978 | 109.51@232361 | 383 | -7.88 | 3 | 28 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 171 | 空 | 96.88@235756 | 79.02@251428 | 15,672 | -88.85 | 11 | 1084 | Y | direction_flip_long | COST_REDUCING→POSITION_OPEN→PRINCIPAL_WITHDRAWN |
| 172 | 空 | 74.94@255587 | 75.09@255597 | 10 | -0.20 | 0 | 1 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 173 | 空 | 70.80@274177 | 69.54@274568 | 391 | -5.46 | 4 | 23 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 174 | 空 | 61.50@306112 | 63.53@306119 | 7 | -3.30 | 0 | 0 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |

### FSM 状态分布

| 状态 | Bars | 占比 |
|------|------|------|
| SCANNING | 273,767 | 82.1% |
| POSITION_OPEN | 13,559 | 4.1% |
| COST_REDUCING | 10,014 | 3.0% |
| PRINCIPAL_WITHDRAWN | 36,273 | 10.9% |
| STOPPED_OUT | 0 | 0.0% |

<details><summary>事件流（8349条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 201 | 14.60 | POSITION_OPEN | ENTRY_SHORT@14.60 |
| 219 | 12.83 | COST_REDUCING | TRIM@12.83 s=4156.8 r=0.607 |
| 221 | 11.89 | POSITION_OPEN | CLOSE_DIFF@11.89 p=-3907.42 |
| 232 | 12.88 | COST_REDUCING | TRIM@12.88 s=3839.1 r=0.561 |
| 239 | 12.48 | POSITION_OPEN | CLOSE_DIFF@12.48 p=-1535.64 |
| 245 | 12.32 | COST_REDUCING | TRIM@12.32 s=7781.1 r=0.904 |
| 247 | 12.41 | POSITION_OPEN | CLOSE_DIFF@12.41 p=700.30 |
| 254 | 12.53 | COST_REDUCING | TRIM@12.53 s=7781.1 r=0.904 |
| 265 | 12.42 | POSITION_OPEN | CLOSE_DIFF@12.42 p=-855.93 |
| 268 | 12.65 | COST_REDUCING | TRIM@12.65 s=7781.1 r=0.904 |
| 272 | 13.28 | SCANNING | STOP:direction_flip_long@13.28 pnl=+5.52% |
| 403 | 11.14 | POSITION_OPEN | ENTRY_SHORT@11.14 |
| 407 | 11.47 | COST_REDUCING | TRIM@11.47 s=8841.5 r=0.985 |
| 412 | 11.19 | POSITION_OPEN | CLOSE_DIFF@11.19 p=-2475.62 |
| 415 | 11.24 | COST_REDUCING | TRIM@11.24 s=8841.5 r=0.985 |
| 416 | 11.15 | POSITION_OPEN | CLOSE_DIFF@11.15 p=-795.74 |
| 421 | 11.28 | COST_REDUCING | TRIM@11.28 s=8965.4 r=0.999 |
| 428 | 11.36 | POSITION_OPEN | CLOSE_DIFF@11.36 p=717.23 |
| 431 | 11.41 | COST_REDUCING | TRIM@11.41 s=9460.4 r=0.999 |
| 434 | 11.53 | SCANNING | STOP:direction_flip_long@11.53 pnl=-4.08% |
| 512 | 8.91 | POSITION_OPEN | ENTRY_SHORT@8.91 |
| 515 | 9.28 | COST_REDUCING | TRIM@9.28 s=10642.3 r=0.948 |
| 521 | 9.10 | POSITION_OPEN | CLOSE_DIFF@9.10 p=-1862.40 |
| 522 | 9.36 | COST_REDUCING | TRIM@9.36 s=10642.3 r=0.948 |
| 523 | 9.38 | SCANNING | STOP:direction_flip_long@9.38 pnl=-5.33% |
| 540 | 8.51 | POSITION_OPEN | ENTRY_SHORT@8.51 |
| 552 | 8.44 | COST_REDUCING | TRIM@8.44 s=11664.1 r=0.993 |
| 553 | 8.56 | SCANNING | STOP:direction_flip_long@8.56 pnl=-0.59% |
| 624 | 8.88 | POSITION_OPEN | ENTRY_LONG@8.88 |
| 637 | 8.80 | COST_REDUCING | TRIM@8.80 s=1253.5 r=0.111 |
| 640 | 8.94 | SCANNING | STOP:bsp_invalidate@8.94 pnl=+0.68% |
| 755 | 9.49 | POSITION_OPEN | ENTRY_LONG@9.49 |
| 759 | 9.41 | COST_REDUCING | TRIM@9.41 s=1541.8 r=0.146 |
| 764 | 9.44 | POSITION_OPEN | CLOSE_DIFF@9.44 p=-46.25 |
| 765 | 9.45 | POSITION_OPEN | ADD@9.45 +9071.3 r=0.861 |
| 767 | 9.40 | COST_REDUCING | TRIM@9.40 s=2869.1 r=0.146 |
| 775 | 9.29 | POSITION_OPEN | CLOSE_DIFF@9.29 p=315.60 |
| 777 | 9.25 | SCANNING | STOP:bsp_invalidate@9.25 pnl=-2.16% |
| 1,089 | 9.99 | POSITION_OPEN | ENTRY_LONG@9.99 |
| 1,092 | 9.82 | COST_REDUCING | TRIM@9.82 s=1913.9 r=0.191 |
| 1,102 | 9.80 | POSITION_OPEN | CLOSE_DIFF@9.80 p=47.85 |
| 1,115 | 10.19 | COST_REDUCING | TRIM@10.19 s=4522.9 r=0.247 |
| 1,119 | 10.26 | POSITION_OPEN | CLOSE_DIFF@10.26 p=-316.60 |
| 1,124 | 10.17 | COST_REDUCING | TRIM@10.17 s=4522.9 r=0.247 |
| 1,131 | 10.45 | POSITION_OPEN | CLOSE_DIFF@10.45 p=-1266.40 |
| 1,134 | 10.39 | COST_REDUCING | TRIM@10.39 s=4522.9 r=0.247 |
| 1,138 | 10.46 | POSITION_OPEN | CLOSE_DIFF@10.46 p=-316.60 |
| 1,142 | 10.41 | COST_REDUCING | TRIM@10.41 s=4522.9 r=0.247 |
| 1,144 | 10.40 | POSITION_OPEN | CLOSE_DIFF@10.40 p=45.23 |
| 1,149 | 10.42 | COST_REDUCING | TRIM@10.42 s=8116.0 r=0.247 |
| 1,150 | 10.27 | POSITION_OPEN | CLOSE_DIFF@10.27 p=1217.40 |
| 1,154 | 10.08 | COST_REDUCING | TRIM@10.08 s=8116.0 r=0.247 |
| 1,165 | 10.05 | POSITION_OPEN | CLOSE_DIFF@10.05 p=243.48 |
| 1,169 | 10.12 | COST_REDUCING | TRIM@10.12 s=8116.0 r=0.247 |
| 1,177 | 10.15 | POSITION_OPEN | CLOSE_DIFF@10.15 p=-202.90 |
| 1,192 | 9.80 | COST_REDUCING | TRIM@9.80 s=8116.0 r=0.247 |
| 1,195 | 10.01 | POSITION_OPEN | CLOSE_DIFF@10.01 p=-1704.36 |
| 1,219 | 10.29 | COST_REDUCING | TRIM@10.29 s=8116.0 r=0.247 |
| 1,230 | 10.21 | POSITION_OPEN | CLOSE_DIFF@10.21 p=649.28 |
| 1,237 | 10.11 | COST_REDUCING | TRIM@10.11 s=8116.0 r=0.247 |
| 1,239 | 10.15 | POSITION_OPEN | CLOSE_DIFF@10.15 p=-324.64 |
| 1,243 | 10.05 | COST_REDUCING | TRIM@10.05 s=8116.0 r=0.247 |
| 1,249 | 9.91 | POSITION_OPEN | CLOSE_DIFF@9.91 p=1136.24 |
| 1,250 | 9.80 | COST_REDUCING | TRIM@9.80 s=8116.0 r=0.247 |
| 1,252 | 9.97 | POSITION_OPEN | CLOSE_DIFF@9.97 p=-1379.72 |
| 1,259 | 9.70 | COST_REDUCING | TRIM@9.70 s=8116.0 r=0.247 |
| 1,262 | 9.78 | POSITION_OPEN | CLOSE_DIFF@9.78 p=-649.28 |
| 1,265 | 9.71 | COST_REDUCING | TRIM@9.71 s=8116.0 r=0.247 |
| 1,272 | 9.78 | POSITION_OPEN | CLOSE_DIFF@9.78 p=-568.12 |
| 1,282 | 9.97 | COST_REDUCING | TRIM@9.97 s=8116.0 r=0.247 |
| 1,306 | 9.80 | POSITION_OPEN | CLOSE_DIFF@9.80 p=1379.72 |
| 1,331 | 9.82 | COST_REDUCING | TRIM@9.82 s=8116.0 r=0.247 |
| 1,334 | 10.00 | POSITION_OPEN | CLOSE_DIFF@10.00 p=-1460.88 |
| 1,349 | 10.48 | COST_REDUCING | TRIM@10.48 s=8116.0 r=0.247 |
| 1,361 | 10.35 | POSITION_OPEN | CLOSE_DIFF@10.35 p=1055.08 |
| 1,362 | 10.30 | COST_REDUCING | TRIM@10.30 s=8116.0 r=0.247 |
| 1,365 | 10.31 | POSITION_OPEN | CLOSE_DIFF@10.31 p=-81.16 |
| 1,369 | 10.37 | POSITION_OPEN | ADD@10.37 +27771.5 r=0.845 |
| 1,371 | 10.31 | COST_REDUCING | TRIM@10.31 s=14971.6 r=0.247 |
| 1,375 | 10.31 | POSITION_OPEN | CLOSE_DIFF@10.31 p=0.00 |
| ... | ... | ... | （共8349条） |
</details>

## HK700

- 数据：**1,408,882** bars (1min), 2008-06-23 10:00:00+08:00 → 2026-06-04 13:12:00+08:00
- 价格：11.36 → 458.20
- Buy-and-hold: **+3933.45%**
- 回测耗时：**341.7s** (4124 bars/s)

### 总体指标

| 指标 | 值 |
|------|-----|
| 交易数 | 461 (多182/空279) |
| 胜率 | 40.8% |
| 平均收益 | -4.712% |
| 复利累计 | -1002.76% |
| 最大回撤 | -930.01% |
| 平均持仓 | 917 bars |
| 有加仓 | 142/461 |
| 有降成本 | 269/461 |
| 本金回收 | 3/461 |

### 交易明细

| # | 方向 | 入场 | 出场 | 持仓 | PnL% | 加仓 | 短差 | PW | 退出 | 状态 |
|---|------|------|------|------|------|------|------|-----|------|------|
| 1 | 空 | 11.54@476 | 11.06@1036 | 560 | +2.47 | 2 | 22 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 2 | 空 | 10.84@1567 | 10.76@1612 | 45 | +0.68 | 1 | 2 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 3 | 多 | 11.32@2115 | 11.58@2505 | 390 | +3.49 | 2 | 18 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 4 | 多 | 12.08@2890 | 11.76@3127 | 237 | -2.15 | 0 | 7 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 5 | 多 | 12.50@5272 | 12.64@5364 | 92 | +1.28 | 0 | 5 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 6 | 空 | 12.78@5438 | 12.82@5449 | 11 | -0.31 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 7 | 空 | 12.44@5728 | 12.24@6211 | 483 | -1.56 | 3 | 24 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 8 | 空 | 12.82@6439 | 12.82@6453 | 14 | -0.00 | 0 | 0 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 9 | 空 | 12.78@7263 | 12.32@8055 | 792 | +1.46 | 3 | 27 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 10 | 空 | 11.52@9277 | 12.06@10697 | 1,420 | -5.09 | 3 | 64 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 11 | 空 | 11.52@12476 | 10.96@12617 | 141 | +4.18 | 1 | 5 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 12 | 空 | 10.74@12722 | 11.62@13280 | 558 | -9.13 | 2 | 24 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 13 | 空 | 10.52@13803 | 10.80@14344 | 541 | -8.88 | 1 | 22 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 14 | 空 | 10.32@14494 | 10.40@14511 | 17 | -0.78 | 0 | 0 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 15 | 空 | 10.24@14564 | 9.54@14625 | 61 | +6.84 | 0 | 2 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 16 | 多 | 8.34@17608 | 8.40@17652 | 44 | +0.72 | 0 | 0 |  | direction_flip_short | POSITION_OPEN |
| 17 | 空 | 7.70@18094 | 8.06@18265 | 171 | -5.00 | 1 | 13 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 18 | 多 | 8.90@18448 | 9.40@19166 | 718 | +0.01 | 6 | 29 | Y | bsp_invalidate | COST_REDUCING→POSITION_OPEN→PRINCIPAL_WITHDRAWN |
| 19 | 空 | 7.56@20879 | 7.38@20928 | 49 | +2.38 | 0 | 2 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 20 | 空 | 7.24@20965 | 7.28@20981 | 16 | -0.55 | 0 | 1 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 21 | 空 | 7.16@20994 | 7.18@21016 | 22 | -0.28 | 0 | 2 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 22 | 多 | 6.82@21080 | 7.38@21096 | 16 | +8.21 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 23 | 多 | 7.66@21244 | 9.72@21773 | 529 | +5.33 | 6 | 25 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 24 | 多 | 8.66@24155 | 8.62@24172 | 17 | -0.46 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 25 | 多 | 10.68@45732 | 10.44@46586 | 854 | +1.64 | 2 | 36 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 26 | 空 | 11.16@47463 | 11.18@47475 | 12 | -0.18 | 0 | 1 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 27 | 多 | 11.82@49533 | 11.76@50641 | 1,108 | +1.74 | 6 | 67 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 28 | 多 | 13.80@52070 | 13.72@52074 | 4 | -0.58 | 0 | 0 |  | direction_flip_short | POSITION_OPEN |
| 29 | 空 | 13.58@52084 | 13.62@52096 | 12 | -0.29 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 30 | 空 | 13.10@52443 | 13.36@52461 | 18 | -1.98 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 31 | 多 | 13.82@53073 | 13.08@53163 | 90 | -3.33 | 0 | 5 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 32 | 空 | 15.08@54149 | 15.00@54153 | 4 | +0.53 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 33 | 空 | 14.28@54364 | 14.52@55051 | 687 | -0.39 | 2 | 45 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 34 | 空 | 14.22@55803 | 14.30@56061 | 258 | -1.34 | 2 | 14 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 35 | 多 | 15.30@56286 | 15.90@56374 | 88 | +4.05 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 36 | 空 | 15.98@56463 | 15.58@56606 | 143 | +0.98 | 1 | 3 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 37 | 多 | 15.76@57003 | 16.40@57304 | 301 | +4.57 | 0 | 11 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 38 | 多 | 16.86@57725 | 16.26@57741 | 16 | -3.56 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 39 | 空 | 16.26@57989 | 16.50@58439 | 450 | -2.56 | 3 | 20 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 40 | 空 | 16.88@60207 | 16.89@60218 | 11 | -0.06 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 41 | 多 | 17.14@61068 | 17.03@61251 | 183 | +0.05 | 0 | 19 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 42 | 多 | 17.33@61487 | 17.23@61565 | 78 | -0.58 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 43 | 多 | 17.74@64598 | 19.14@64778 | 180 | +8.27 | 0 | 6 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 44 | 多 | 19.49@64839 | 19.17@64923 | 84 | -1.08 | 0 | 6 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 45 | 空 | 18.84@64981 | 18.77@65015 | 34 | +0.37 | 0 | 1 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 46 | 空 | 18.68@65081 | 18.80@65403 | 322 | -1.40 | 2 | 29 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 47 | 多 | 21.09@68539 | 20.98@68567 | 28 | -0.43 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 48 | 多 | 21.50@68613 | 21.57@68681 | 68 | +0.33 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 49 | 空 | 21.63@68702 | 21.53@68710 | 8 | +0.46 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 50 | 空 | 20.80@69729 | 20.65@69733 | 4 | +0.72 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 51 | 多 | 23.40@73130 | 23.45@73211 | 81 | +0.21 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 52 | 空 | 23.30@73236 | 23.35@73238 | 2 | -0.21 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 53 | 多 | 23.95@73516 | 23.45@73542 | 26 | -2.09 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 54 | 空 | 23.05@74200 | 23.30@74290 | 90 | -1.20 | 1 | 3 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 55 | 空 | 24.15@76091 | 22.55@76701 | 610 | +2.90 | 3 | 18 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 56 | 空 | 22.35@76748 | 23.20@77224 | 476 | -0.92 | 2 | 25 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 57 | 多 | 24.40@79173 | 25.05@79317 | 144 | +2.66 | 0 | 8 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 58 | 多 | 25.15@79349 | 25.15@79378 | 29 | +0.00 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 59 | 空 | 25.70@79567 | 25.50@79589 | 22 | +0.78 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 60 | 空 | 24.95@79775 | 25.00@79807 | 32 | -0.20 | 0 | 1 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 61 | 多 | 26.20@79976 | 26.35@80006 | 30 | +0.57 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 62 | 多 | 27.15@80133 | 27.15@80161 | 28 | +0.00 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 63 | 空 | 27.15@80255 | 27.25@80258 | 3 | -0.37 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 64 | 空 | 26.50@80357 | 27.40@80844 | 487 | -4.24 | 2 | 30 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 65 | 空 | 27.10@81094 | 26.90@81101 | 7 | +0.74 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 66 | 空 | 24.90@81579 | 25.75@82332 | 753 | -2.66 | 4 | 37 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 67 | 多 | 28.00@89107 | 28.00@89165 | 58 | +0.52 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 68 | 多 | 28.50@89236 | 29.25@89485 | 249 | +2.63 | 0 | 12 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 69 | 空 | 29.50@89661 | 29.60@89681 | 20 | -0.17 | 1 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 70 | 空 | 29.10@90128 | 28.50@91126 | 998 | -0.90 | 4 | 62 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 71 | 多 | 30.40@91959 | 30.90@92168 | 209 | +1.64 | 0 | 8 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 72 | 空 | 30.80@92245 | 30.90@92260 | 15 | -0.32 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 73 | 空 | 31.40@92565 | 31.55@92575 | 10 | -0.48 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 74 | 空 | 32.00@92822 | 32.10@92825 | 3 | -0.31 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 75 | 空 | 31.80@92880 | 31.90@92913 | 33 | -0.31 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 76 | 空 | 31.15@92988 | 31.25@92989 | 1 | -0.32 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 77 | 空 | 30.80@93018 | 31.45@93622 | 604 | -1.28 | 3 | 36 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 78 | 多 | 32.25@94421 | 32.25@94564 | 143 | +0.00 | 0 | 4 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 79 | 空 | 30.25@94881 | 28.85@95485 | 604 | -0.62 | 6 | 33 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 80 | 空 | 23.90@117873 | 22.35@119158 | 1,285 | +1.64 | 7 | 77 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 81 | 空 | 29.40@135170 | 29.35@135190 | 20 | +0.17 | 0 | 1 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 82 | 多 | 32.55@137848 | 32.50@137943 | 95 | -0.15 | 0 | 6 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 83 | 多 | 34.10@138120 | 33.90@138152 | 32 | -0.44 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 84 | 多 | 34.35@138409 | 34.90@138474 | 65 | +1.60 | 0 | 4 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 85 | 多 | 35.25@138516 | 35.25@138534 | 18 | +0.00 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 86 | 空 | 33.20@138753 | 34.65@138827 | 74 | -3.90 | 1 | 2 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 87 | 空 | 32.80@140914 | 34.45@141960 | 1,046 | -2.98 | 1 | 50 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 88 | 空 | 32.15@143315 | 32.85@143399 | 84 | -1.61 | 1 | 4 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 89 | 空 | 32.10@144189 | 32.00@144271 | 82 | +0.31 | 1 | 2 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 90 | 空 | 36.75@153977 | 36.75@153993 | 16 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 91 | 空 | 36.85@154084 | 36.90@154092 | 8 | -0.14 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 92 | 空 | 36.90@154123 | 37.15@154132 | 9 | -0.68 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 93 | 多 | 37.50@154207 | 37.50@154236 | 29 | +0.00 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 94 | 空 | 37.50@154284 | 37.55@154288 | 4 | -0.13 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 95 | 多 | 37.40@154588 | 36.85@154630 | 42 | -1.20 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 96 | 空 | 36.65@154893 | 36.35@154946 | 53 | +0.82 | 0 | 3 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 97 | 空 | 35.75@155133 | 36.80@156326 | 1,193 | +0.05 | 4 | 70 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 98 | 空 | 37.75@156865 | 38.40@157182 | 317 | -3.31 | 2 | 13 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 99 | 空 | 38.25@157358 | 37.50@157447 | 89 | +1.05 | 1 | 1 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 100 | 空 | 36.80@157474 | 35.20@157922 | 448 | +1.60 | 3 | 20 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 101 | 多 | 39.10@161213 | 40.35@161881 | 668 | +5.75 | 0 | 46 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 102 | 多 | 40.80@162024 | 40.00@162095 | 71 | -1.35 | 0 | 5 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 103 | 空 | 39.80@162278 | 40.85@162573 | 295 | -2.82 | 1 | 0 |  | direction_flip_long | POSITION_OPEN |
| 104 | 空 | 40.65@162590 | 42.20@162628 | 38 | -3.81 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 105 | 空 | 41.60@162821 | 41.45@162866 | 45 | +0.24 | 1 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 106 | 空 | 39.40@163475 | 39.10@163862 | 387 | +0.56 | 0 | 20 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 107 | 空 | 36.70@164403 | 37.05@164418 | 15 | -0.95 | 0 | 0 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 108 | 空 | 34.50@165995 | 35.55@166590 | 595 | -0.85 | 2 | 38 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 109 | 空 | 34.50@167474 | 34.40@167528 | 54 | +0.29 | 0 | 4 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 110 | 多 | 42.50@172209 | 42.00@172253 | 44 | -1.18 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 111 | 空 | 32.80@193803 | 33.15@194077 | 274 | +0.05 | 3 | 12 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 112 | 空 | 31.90@207303 | 30.80@207634 | 331 | +1.42 | 2 | 10 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 113 | 空 | 25.80@216388 | 28.10@217341 | 953 | -5.89 | 2 | 56 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 114 | 空 | 43.40@245081 | 43.50@245089 | 8 | -0.23 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 115 | 空 | 43.30@245433 | 44.30@246133 | 700 | -2.31 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 116 | 空 | 41.80@246678 | 44.40@247578 | 900 | -3.28 | 2 | 48 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 117 | 空 | 44.40@247671 | 44.50@247674 | 3 | -0.23 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 118 | 多 | 45.10@247743 | 45.20@247776 | 33 | +0.22 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 119 | 多 | 45.70@248090 | 45.20@248423 | 333 | -1.09 | 0 | 14 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 120 | 空 | 44.70@248491 | 42.40@250749 | 2,258 | -1.68 | 9 | 136 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 121 | 空 | 40.60@251169 | 41.50@251817 | 648 | -2.53 | 3 | 24 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 122 | 空 | 46.10@272811 | 46.00@272878 | 67 | +0.22 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 123 | 空 | 44.80@274983 | 43.70@276256 | 1,273 | +1.26 | 3 | 70 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 124 | 空 | 45.90@278664 | 46.00@278700 | 36 | -0.22 | 0 | 1 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 125 | 多 | 47.10@279755 | 46.90@279841 | 86 | -0.42 | 0 | 5 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 126 | 空 | 46.70@279959 | 46.90@280037 | 78 | -0.43 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 127 | 空 | 46.70@280096 | 46.70@280928 | 832 | -0.00 | 1 | 0 |  | direction_flip_long | POSITION_OPEN |
| 128 | 多 | 47.90@282420 | 48.00@282493 | 73 | +0.21 | 0 | 5 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 129 | 多 | 48.70@282644 | 48.50@282657 | 13 | -0.41 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 130 | 空 | 48.10@283893 | 48.40@286581 | 2,688 | -0.21 | 5 | 122 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 131 | 多 | 50.30@287907 | 50.00@287963 | 56 | -0.60 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 132 | 空 | 50.00@288003 | 50.10@288014 | 11 | -0.20 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 133 | 空 | 49.90@288115 | 50.30@288185 | 70 | -0.80 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 134 | 空 | 49.90@288301 | 49.50@288413 | 112 | +0.41 | 1 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 135 | 多 | 50.50@289428 | 50.50@289785 | 357 | +0.00 | 0 | 15 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 136 | 多 | 51.50@289833 | 51.30@289884 | 51 | -0.39 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 137 | 空 | 47.10@292803 | 47.40@294463 | 1,660 | -1.59 | 4 | 97 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 138 | 多 | 51.50@316533 | 51.60@316642 | 109 | +0.19 | 0 | 5 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 139 | 空 | 51.50@316683 | 51.60@316692 | 9 | -0.19 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 140 | 多 | 52.50@317523 | 52.30@317566 | 43 | -0.38 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 141 | 空 | 51.00@318465 | 48.50@319593 | 1,128 | +2.42 | 3 | 37 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 142 | 多 | 44.30@323133 | 44.50@323428 | 295 | +0.74 | 1 | 19 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 143 | 多 | 53.10@332143 | 54.00@332355 | 212 | +1.69 | 0 | 9 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 144 | 多 | 55.60@332394 | 56.00@332712 | 318 | +0.72 | 0 | 11 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 145 | 空 | 54.40@333414 | 54.60@333544 | 130 | -0.86 | 2 | 10 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 146 | 空 | 56.60@334718 | 56.60@334747 | 29 | -0.00 | 0 | 2 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 147 | 多 | 57.10@335197 | 57.20@335552 | 355 | +0.18 | 0 | 21 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 148 | 多 | 56.90@337352 | 54.80@338039 | 687 | +0.53 | 0 | 38 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 149 | 空 | 53.50@339607 | 52.60@339849 | 242 | +1.68 | 2 | 18 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 150 | 空 | 60.70@345178 | 60.80@345241 | 63 | -0.16 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 151 | 空 | 60.70@345504 | 60.60@345513 | 9 | +0.16 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 152 | 多 | 61.50@345753 | 60.00@346104 | 351 | -0.98 | 0 | 19 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 153 | 空 | 60.00@346540 | 61.90@347666 | 1,126 | -1.13 | 4 | 0 |  | direction_flip_long | POSITION_OPEN |
| 154 | 空 | 64.10@348030 | 64.00@348043 | 13 | +0.16 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 155 | 多 | 65.40@348422 | 65.90@348624 | 202 | +0.76 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 156 | 空 | 66.40@348670 | 66.50@348678 | 8 | -0.15 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 157 | 空 | 65.00@348894 | 64.90@348935 | 41 | +0.15 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 158 | 空 | 64.30@349027 | 67.30@349713 | 686 | -3.43 | 1 | 0 |  | direction_flip_long | POSITION_OPEN |
| 159 | 多 | 67.40@349864 | 68.40@350041 | 177 | +1.48 | 0 | 13 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 160 | 多 | 70.00@350391 | 68.80@350575 | 184 | -1.29 | 0 | 12 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 161 | 空 | 66.80@350624 | 66.60@350655 | 31 | +0.30 | 0 | 1 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 162 | 空 | 66.40@350753 | 69.50@352130 | 1,377 | -4.49 | 2 | 73 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 163 | 多 | 70.00@356337 | 70.60@356379 | 42 | +1.00 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 164 | 空 | 69.40@356643 | 71.00@356744 | 101 | -2.31 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 165 | 多 | 71.10@356973 | 71.20@357047 | 74 | +0.28 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 166 | 空 | 71.20@357235 | 71.20@357249 | 14 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 167 | 多 | 71.90@357633 | 71.90@357675 | 42 | +0.00 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 168 | 空 | 71.90@357810 | 73.20@357963 | 153 | -1.81 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 169 | 多 | 73.40@358291 | 74.00@358336 | 45 | +0.82 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 170 | 多 | 74.10@358621 | 75.40@358703 | 82 | +1.75 | 0 | 4 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 171 | 空 | 75.30@358765 | 75.50@358768 | 3 | -0.27 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 172 | 空 | 75.30@358865 | 75.40@358895 | 30 | -0.13 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 173 | 空 | 77.20@359617 | 77.30@359620 | 3 | -0.13 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 174 | 空 | 74.80@360093 | 74.40@360120 | 27 | +0.53 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 175 | 空 | 74.00@361036 | 76.00@362126 | 1,090 | -2.28 | 1 | 64 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 176 | 空 | 76.00@364083 | 76.50@364089 | 6 | -0.66 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 177 | 多 | 78.70@366214 | 79.30@366334 | 120 | +0.76 | 0 | 4 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 178 | 空 | 81.70@366776 | 81.90@366801 | 25 | -0.24 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 179 | 空 | 81.70@366992 | 81.60@367000 | 8 | +0.12 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 180 | 空 | 81.50@367007 | 82.00@367053 | 46 | -0.61 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 181 | 空 | 80.90@367264 | 78.00@368703 | 1,439 | -0.77 | 7 | 88 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 182 | 空 | 75.60@370023 | 73.30@372118 | 2,095 | +0.91 | 8 | 119 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 183 | 多 | 83.80@376622 | 84.20@377766 | 1,144 | +3.70 | 0 | 68 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 184 | 多 | 84.50@377804 | 84.80@377904 | 100 | +0.47 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 185 | 多 | 85.60@378236 | 85.20@378261 | 25 | -0.47 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 186 | 多 | 87.50@378635 | 87.10@378946 | 311 | -0.46 | 0 | 18 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 187 | 多 | 87.60@379914 | 87.20@379939 | 25 | -0.34 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 188 | 空 | 85.20@380271 | 86.70@380592 | 321 | -1.64 | 1 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 189 | 多 | 87.80@381786 | 87.70@381817 | 31 | -0.11 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 190 | 空 | 88.20@381890 | 88.40@381927 | 37 | -0.23 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 191 | 空 | 88.40@382004 | 88.60@382016 | 12 | -0.23 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 192 | 多 | 91.10@382134 | 91.00@382199 | 65 | -0.11 | 0 | 6 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 193 | 空 | 91.10@382290 | 91.10@382310 | 20 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 194 | 空 | 91.50@382435 | 91.30@382442 | 7 | +0.22 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 195 | 多 | 92.50@382736 | 91.90@383147 | 411 | -0.00 | 0 | 37 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 196 | 空 | 90.40@384523 | 91.00@384527 | 4 | -0.66 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 197 | 多 | 93.30@385805 | 96.30@385849 | 44 | +3.22 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 198 | 空 | 94.40@386159 | 95.10@386177 | 18 | -0.74 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 199 | 多 | 97.50@386371 | 92.90@387813 | 1,442 | -1.03 | 0 | 125 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 200 | 空 | 92.70@388114 | 93.40@388743 | 629 | -1.33 | 3 | 29 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 201 | 多 | 97.90@389098 | 97.70@389283 | 185 | +0.41 | 0 | 9 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 202 | 空 | 96.40@389469 | 96.00@389481 | 12 | +0.41 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 203 | 多 | 100.40@391688 | 100.70@391707 | 19 | +0.30 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 204 | 多 | 101.70@392172 | 101.60@392207 | 35 | -0.10 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 205 | 空 | 104.60@392402 | 104.80@392404 | 2 | -0.19 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 206 | 空 | 105.10@392457 | 105.20@392459 | 2 | -0.10 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 207 | 多 | 107.80@392720 | 108.30@392964 | 244 | +1.67 | 0 | 17 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 208 | 空 | 103.80@393568 | 105.60@393805 | 237 | -0.77 | 1 | 18 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 209 | 多 | 111.00@395008 | 112.60@395084 | 76 | +1.44 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 210 | 空 | 113.00@395147 | 113.40@395182 | 35 | -0.35 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 211 | 空 | 114.60@395333 | 114.60@395352 | 19 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 212 | 空 | 112.00@395824 | 112.60@395883 | 59 | -0.54 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 213 | 多 | 116.00@396364 | 116.20@396873 | 509 | +0.86 | 0 | 44 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 214 | 空 | 111.00@397873 | 102.60@400114 | 2,241 | +0.64 | 8 | 129 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 215 | 空 | 92.80@403803 | 103.20@405109 | 1,306 | -4.72 | 4 | 88 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 216 | 空 | 91.55@408854 | 90.95@409083 | 229 | -0.09 | 1 | 14 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 217 | 空 | 87.25@409936 | 87.55@410623 | 687 | -1.06 | 2 | 37 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 218 | 空 | 92.20@410884 | 92.30@410887 | 3 | -0.11 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 219 | 多 | 119.10@428552 | 119.45@428584 | 32 | +0.29 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 220 | 空 | 118.90@428610 | 118.90@428621 | 11 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 221 | 多 | 120.30@428725 | 119.90@428760 | 35 | -0.33 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 222 | 空 | 118.75@429037 | 120.55@430331 | 1,294 | -1.43 | 2 | 0 |  | direction_flip_long | POSITION_OPEN |
| 223 | 空 | 120.50@430387 | 120.50@430391 | 4 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 224 | 多 | 121.10@430475 | 122.70@430530 | 55 | +1.32 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 225 | 空 | 120.30@431003 | 118.80@431193 | 190 | +1.06 | 1 | 0 |  | direction_flip_long | POSITION_OPEN |
| 226 | 多 | 123.45@431909 | 118.00@433282 | 1,373 | +1.13 | 0 | 109 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 227 | 空 | 117.15@436069 | 115.95@436286 | 217 | +0.55 | 4 | 18 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 228 | 多 | 108.70@441653 | 110.45@442007 | 354 | +1.57 | 1 | 36 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 229 | 多 | 124.00@453483 | 115.40@454295 | 812 | +0.61 | 3 | 49 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 230 | 多 | 104.35@459144 | 104.45@459156 | 12 | +0.10 | 0 | 0 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 231 | 多 | 99.90@460671 | 100.00@460712 | 41 | +0.40 | 1 | 1 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 232 | 多 | 125.75@468782 | 125.45@468789 | 7 | -0.24 | 0 | 0 |  | direction_flip_short | POSITION_OPEN |
| 233 | 多 | 124.05@469035 | 123.60@469081 | 46 | -0.28 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 234 | 空 | 122.60@469963 | 123.45@470463 | 500 | -0.83 | 2 | 30 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 235 | 空 | 126.85@476529 | 126.55@476555 | 26 | +0.24 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 236 | 多 | 127.20@480386 | 128.50@480520 | 134 | +1.02 | 0 | 11 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 237 | 多 | 128.95@480551 | 129.05@480562 | 11 | +0.08 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 238 | 空 | 135.80@480688 | 135.60@480696 | 8 | +0.15 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 239 | 空 | 131.80@480919 | 133.55@482952 | 2,033 | -1.09 | 7 | 77 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 240 | 空 | 137.45@483910 | 138.00@483961 | 51 | -0.40 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 241 | 空 | 139.75@483982 | 139.55@483990 | 8 | +0.14 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 242 | 空 | 145.85@484703 | 156.05@485234 | 531 | -3.79 | 1 | 32 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 243 | 多 | 157.00@485279 | 147.10@485283 | 4 | -6.31 | 0 | 0 |  | direction_flip_short | POSITION_OPEN |
| 244 | 空 | 145.85@485653 | 144.05@486953 | 1,300 | -1.06 | 7 | 83 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 245 | 空 | 140.40@490854 | 144.70@491886 | 1,032 | +0.31 | 4 | 70 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 246 | 空 | 139.75@502611 | 142.80@503078 | 467 | -0.90 | 3 | 38 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 247 | 空 | 129.35@503952 | 133.85@504022 | 70 | -3.16 | 1 | 4 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 248 | 空 | 127.30@504423 | 135.95@504967 | 544 | -6.28 | 3 | 34 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 249 | 空 | 120.30@514983 | 117.25@515851 | 868 | -0.28 | 4 | 72 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 250 | 空 | 157.15@576651 | 157.65@576659 | 8 | -0.32 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 251 | 多 | 159.00@577090 | 159.40@577132 | 42 | +0.25 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 252 | 多 | 159.75@577245 | 158.75@577305 | 60 | -0.53 | 0 | 5 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 253 | 空 | 156.05@577653 | 156.80@578247 | 594 | -0.23 | 2 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 254 | 多 | 159.85@578624 | 161.20@578649 | 25 | +0.84 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 255 | 空 | 160.95@578733 | 161.20@578735 | 2 | -0.16 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 256 | 多 | 161.85@578886 | 162.40@578973 | 87 | +0.34 | 0 | 8 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 257 | 空 | 161.20@579015 | 161.15@579016 | 1 | +0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 258 | 空 | 161.50@579631 | 156.15@580718 | 1,087 | -0.31 | 6 | 57 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 259 | 多 | 165.10@584332 | 164.65@584429 | 97 | -0.03 | 0 | 7 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 260 | 空 | 165.55@585931 | 166.65@586233 | 302 | -0.66 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 261 | 空 | 166.10@586277 | 166.10@586280 | 3 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 262 | 多 | 167.05@586570 | 167.05@586583 | 13 | +0.00 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 263 | 多 | 169.60@587222 | 169.40@587234 | 12 | -0.12 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 264 | 空 | 170.60@587389 | 169.70@587445 | 56 | +0.32 | 1 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 265 | 空 | 170.35@587506 | 170.45@587509 | 3 | -0.06 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 266 | 空 | 169.40@587895 | 171.65@588745 | 850 | -1.37 | 1 | 0 |  | direction_flip_long | POSITION_OPEN |
| 267 | 多 | 171.80@589272 | 171.65@589339 | 67 | -0.09 | 0 | 9 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 268 | 多 | 172.20@589443 | 172.45@589531 | 88 | +0.15 | 0 | 6 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 269 | 多 | 174.75@589631 | 173.40@589821 | 190 | -0.66 | 0 | 15 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 270 | 空 | 172.10@589853 | 172.30@589864 | 11 | -0.12 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 271 | 空 | 171.75@590995 | 170.70@592186 | 1,191 | -0.45 | 3 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 272 | 多 | 175.95@593623 | 174.95@593717 | 94 | -0.40 | 0 | 11 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 273 | 多 | 176.25@594108 | 176.15@594155 | 47 | -0.06 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 274 | 空 | 181.30@594483 | 180.85@594492 | 9 | +0.25 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 275 | 空 | 181.15@594616 | 181.20@594621 | 5 | -0.03 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 276 | 空 | 180.10@594869 | 179.10@594911 | 42 | +0.56 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 277 | 空 | 177.65@594956 | 187.85@594983 | 27 | -5.74 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 278 | 多 | 187.85@595080 | 188.25@595129 | 49 | +0.21 | 0 | 4 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 279 | 多 | 187.70@596341 | 185.45@596582 | 241 | -0.88 | 0 | 31 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 280 | 空 | 183.90@597022 | 185.45@597024 | 2 | -0.84 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 281 | 多 | 191.40@599003 | 192.70@599099 | 96 | +0.68 | 0 | 11 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 282 | 空 | 192.30@599136 | 192.80@599140 | 4 | -0.26 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 283 | 空 | 192.70@599156 | 192.80@599179 | 23 | -0.05 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 284 | 多 | 194.50@599296 | 196.90@599317 | 21 | +1.23 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 285 | 多 | 199.50@599600 | 196.20@599646 | 46 | -1.25 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 286 | 空 | 195.00@599939 | 196.20@599948 | 9 | -0.62 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 287 | 多 | 201.50@603494 | 201.90@603546 | 52 | +0.20 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 288 | 空 | 198.90@603790 | 198.00@603888 | 98 | +0.45 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 289 | 空 | 189.70@611837 | 186.55@613940 | 2,103 | +0.60 | 7 | 218 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 290 | 空 | 178.85@614899 | 179.40@616098 | 1,199 | -0.19 | 5 | 81 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 291 | 空 | 175.40@619661 | 176.05@619998 | 337 | -0.30 | 2 | 26 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 292 | 空 | 174.50@621381 | 171.10@622729 | 1,348 | +0.52 | 6 | 114 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 293 | 空 | 167.10@623950 | 167.75@624546 | 596 | -0.28 | 3 | 50 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 294 | 多 | 205.20@643619 | 208.90@643689 | 70 | +1.80 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 295 | 多 | 209.60@643788 | 208.70@643806 | 18 | -0.43 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 296 | 空 | 210.70@644256 | 209.80@644283 | 27 | +0.43 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 297 | 空 | 206.30@644473 | 206.50@644475 | 2 | -0.10 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 298 | 空 | 206.30@644625 | 207.40@645624 | 999 | -0.40 | 7 | 79 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 299 | 多 | 212.00@649383 | 213.50@649533 | 150 | +0.80 | 0 | 14 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 300 | 多 | 214.00@649583 | 214.40@649639 | 56 | +0.23 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 301 | 空 | 212.90@649682 | 211.60@650079 | 397 | +0.66 | 1 | 0 |  | direction_flip_long | POSITION_OPEN |
| 302 | 多 | 216.30@650432 | 215.50@650445 | 13 | -0.37 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 303 | 多 | 216.30@650584 | 219.80@650795 | 211 | +1.76 | 0 | 20 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 304 | 空 | 217.70@650900 | 217.20@650916 | 16 | +0.23 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 305 | 空 | 217.00@651014 | 216.30@651068 | 54 | +0.32 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 306 | 多 | 220.70@651477 | 222.00@651566 | 89 | +0.59 | 0 | 4 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 307 | 多 | 222.30@651695 | 224.40@651814 | 119 | +0.94 | 0 | 9 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 308 | 多 | 225.30@651857 | 224.90@651862 | 5 | -0.18 | 0 | 0 |  | direction_flip_short | POSITION_OPEN |
| 309 | 多 | 225.30@651945 | 223.80@652015 | 70 | -0.49 | 0 | 5 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 310 | 空 | 225.10@652357 | 225.70@652446 | 89 | -0.27 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 311 | 多 | 228.00@652887 | 227.90@652970 | 83 | -0.04 | 0 | 11 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 312 | 多 | 228.40@653096 | 226.40@653139 | 43 | -0.88 | 0 | 5 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 313 | 空 | 223.40@653481 | 229.20@654430 | 949 | -2.26 | 1 | 0 |  | direction_flip_long | POSITION_OPEN |
| 314 | 多 | 235.20@654622 | 233.60@654762 | 140 | -0.34 | 0 | 14 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 315 | 多 | 238.60@655363 | 237.80@655497 | 134 | -0.17 | 0 | 12 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 316 | 空 | 239.70@655874 | 240.40@656199 | 325 | -0.50 | 1 | 0 |  | direction_flip_long | POSITION_OPEN |
| 317 | 多 | 239.50@656537 | 244.80@656573 | 36 | +2.21 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 318 | 多 | 247.00@656588 | 247.60@656774 | 186 | +0.40 | 0 | 19 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 319 | 空 | 242.60@656865 | 243.20@656867 | 2 | -0.25 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 320 | 多 | 250.70@657227 | 252.80@657474 | 247 | +1.24 | 0 | 24 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 321 | 多 | 255.00@657793 | 251.80@657816 | 23 | -1.25 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 322 | 空 | 250.70@657908 | 250.00@657915 | 7 | +0.28 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 323 | 空 | 247.40@659412 | 249.40@660205 | 793 | -0.10 | 4 | 46 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 324 | 多 | 260.30@661693 | 259.90@661712 | 19 | -0.15 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 325 | 空 | 260.30@661761 | 258.10@661895 | 134 | +0.27 | 2 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 326 | 空 | 255.70@661977 | 252.80@663135 | 1,158 | +0.23 | 2 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 327 | 多 | 261.00@664023 | 260.30@664046 | 23 | -0.27 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 328 | 多 | 258.30@664332 | 261.80@665718 | 1,386 | +3.48 | 0 | 125 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 329 | 多 | 264.60@665919 | 263.40@666059 | 140 | -0.45 | 0 | 14 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 330 | 空 | 242.40@667771 | 254.20@669247 | 1,476 | -2.11 | 2 | 144 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 331 | 多 | 270.60@671272 | 274.30@671522 | 250 | +1.51 | 0 | 20 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 332 | 空 | 273.00@671842 | 274.50@671868 | 26 | -0.55 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 333 | 多 | 279.90@672547 | 279.70@672630 | 83 | +0.07 | 0 | 7 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 334 | 多 | 278.70@672838 | 278.00@672895 | 57 | -0.25 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 335 | 空 | 275.60@672937 | 277.10@672962 | 25 | -0.54 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 336 | 多 | 281.50@673368 | 281.50@673393 | 25 | +0.00 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 337 | 空 | 280.80@673539 | 280.20@673545 | 6 | +0.21 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 338 | 多 | 286.30@673939 | 286.70@673971 | 32 | +0.14 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 339 | 多 | 287.20@674081 | 288.30@674154 | 73 | +0.38 | 0 | 5 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 340 | 空 | 288.00@674187 | 288.90@674218 | 31 | -0.31 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 341 | 空 | 290.50@674285 | 291.50@674304 | 19 | -0.34 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 342 | 多 | 292.40@674330 | 290.50@674474 | 144 | -0.48 | 0 | 15 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 343 | 空 | 286.10@674899 | 286.70@674900 | 1 | -0.21 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 344 | 多 | 294.00@675613 | 294.60@675647 | 34 | +0.20 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 345 | 多 | 295.50@675753 | 295.00@675813 | 60 | -0.17 | 0 | 4 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 346 | 多 | 302.90@676094 | 302.50@676099 | 5 | -0.13 | 0 | 0 |  | direction_flip_short | POSITION_OPEN |
| 347 | 空 | 302.50@676347 | 303.10@676373 | 26 | -0.20 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 348 | 空 | 300.90@676409 | 301.20@676424 | 15 | -0.10 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 349 | 空 | 300.70@676655 | 297.60@677566 | 911 | -0.87 | 4 | 41 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 350 | 多 | 311.20@678299 | 310.50@678307 | 8 | -0.22 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 351 | 空 | 311.20@684802 | 310.80@684809 | 7 | +0.13 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 352 | 多 | 315.60@685439 | 319.70@685818 | 379 | +1.90 | 0 | 32 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 353 | 空 | 316.00@687242 | 309.50@688551 | 1,309 | -0.02 | 5 | 82 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 354 | 多 | 324.70@689519 | 324.10@689539 | 20 | -0.18 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 355 | 多 | 324.30@689580 | 324.50@689643 | 63 | +0.06 | 0 | 8 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 356 | 空 | 325.20@690894 | 323.70@691178 | 284 | +0.29 | 1 | 0 |  | direction_flip_long | POSITION_OPEN |
| 357 | 空 | 320.60@691219 | 326.10@691764 | 545 | -1.62 | 2 | 0 |  | direction_flip_long | POSITION_OPEN |
| 358 | 空 | 326.50@691819 | 326.50@691837 | 18 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 359 | 空 | 326.10@691880 | 326.90@691886 | 6 | -0.25 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 360 | 空 | 324.80@692275 | 325.20@692316 | 41 | -0.06 | 1 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 361 | 空 | 324.10@692829 | 321.70@692893 | 64 | +0.74 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 362 | 空 | 322.10@693274 | 328.00@695833 | 2,559 | -2.27 | 3 | 0 |  | direction_flip_long | POSITION_OPEN |
| 363 | 多 | 328.30@695884 | 329.10@695934 | 50 | +0.24 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 364 | 多 | 332.60@696013 | 334.60@696072 | 59 | +0.60 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 365 | 空 | 333.70@696078 | 333.90@696128 | 50 | -0.06 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 366 | 多 | 336.30@696319 | 336.60@696330 | 11 | +0.09 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 367 | 多 | 339.20@696657 | 336.60@696661 | 4 | -0.77 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 368 | 空 | 348.80@696975 | 349.20@696982 | 7 | -0.11 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 369 | 多 | 358.00@697101 | 358.60@697119 | 18 | +0.17 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 370 | 多 | 358.90@697326 | 357.80@697339 | 13 | -0.31 | 0 | 0 |  | direction_flip_short | POSITION_OPEN |
| 371 | 空 | 363.70@697515 | 361.70@697555 | 40 | +0.55 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 372 | 空 | 358.00@697633 | 357.80@697634 | 1 | +0.06 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 373 | 空 | 355.30@698357 | 359.50@698404 | 47 | -1.18 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 374 | 空 | 354.90@699177 | 359.10@699379 | 202 | -1.18 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 375 | 空 | 370.00@699887 | 370.70@699895 | 8 | -0.19 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 376 | 多 | 376.80@700225 | 395.60@700425 | 200 | +4.99 | 0 | 6 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 377 | 空 | 388.30@701376 | 384.80@701527 | 151 | +0.28 | 2 | 3 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 378 | 空 | 380.70@701976 | 383.80@702455 | 479 | -0.61 | 5 | 29 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 379 | 空 | 369.60@702779 | 369.10@703004 | 225 | +0.63 | 3 | 14 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 380 | 空 | 352.70@703459 | 361.70@703704 | 245 | -1.62 | 1 | 10 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 381 | 空 | 347.90@704122 | 345.30@704479 | 357 | -0.63 | 3 | 20 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 382 | 多 | 404.70@710943 | 408.70@711444 | 501 | +1.92 | 0 | 40 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 383 | 空 | 406.50@711656 | 408.20@711683 | 27 | -0.42 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 384 | 空 | 410.00@711753 | 409.80@711758 | 5 | +0.05 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 385 | 多 | 413.00@713590 | 412.60@713600 | 10 | -0.10 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 386 | 空 | 410.60@713826 | 410.90@713834 | 8 | -0.07 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 387 | 多 | 418.30@714357 | 420.00@714408 | 51 | +0.41 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 388 | 空 | 419.20@714435 | 419.60@714438 | 3 | -0.10 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 389 | 多 | 429.70@714694 | 429.90@714724 | 30 | +0.05 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 390 | 多 | 431.90@714779 | 432.70@714860 | 81 | +0.28 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 391 | 多 | 432.90@714978 | 433.40@715024 | 46 | +0.12 | 0 | 2 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 392 | 空 | 425.90@715405 | 428.30@715699 | 294 | -0.61 | 2 | 0 |  | direction_flip_long | POSITION_OPEN |
| 393 | 多 | 438.80@716097 | 435.30@716136 | 39 | -0.73 | 0 | 1 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 394 | 空 | 421.80@716769 | 428.80@716971 | 202 | -0.46 | 2 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 395 | 多 | 391.60@718093 | 397.70@718464 | 371 | +4.78 | 2 | 16 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 396 | 空 | 366.50@730817 | 378.30@731488 | 671 | -1.42 | 2 | 47 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 397 | 多 | 348.10@737904 | 348.40@737911 | 7 | +0.09 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 398 | 空 | 328.50@757646 | 332.20@757679 | 33 | -1.13 | 0 | 1 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 399 | 空 | 318.60@758072 | 314.50@758206 | 134 | +0.82 | 2 | 10 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 400 | 空 | 323.50@758489 | 324.10@758570 | 81 | -0.03 | 1 | 6 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 401 | 空 | 330.20@762939 | 330.20@762944 | 5 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 402 | 空 | 292.20@766519 | 290.90@767300 | 781 | -1.15 | 7 | 45 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 403 | 空 | 285.60@767552 | 286.10@768093 | 541 | -1.13 | 2 | 41 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 404 | 空 | 273.60@773878 | 262.50@774915 | 1,037 | -4.18 | 8 | 70 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 405 | 空 | 264.90@775019 | 261.60@775024 | 5 | +1.25 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 406 | 空 | 270.10@776570 | 268.40@776586 | 16 | +0.63 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 407 | 空 | 243.90@777808 | 238.60@778152 | 344 | +1.06 | 2 | 27 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 408 | 空 | 237.30@778282 | 238.60@778386 | 104 | -1.03 | 1 | 7 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 409 | 空 | 318.40@798811 | 318.20@798818 | 7 | +0.06 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 410 | 多 | 442.90@915410 | 457.80@915572 | 162 | +3.41 | 0 | 9 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 411 | 空 | 460.00@915751 | 460.20@915755 | 4 | -0.04 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 412 | 空 | 448.60@916191 | 467.40@917079 | 888 | -3.06 | 3 | 0 |  | direction_flip_long | POSITION_OPEN |
| 413 | 多 | 474.80@917320 | 476.20@917428 | 108 | +0.29 | 0 | 10 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 414 | 多 | 480.30@917611 | 485.50@917695 | 84 | +1.08 | 0 | 7 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 415 | 空 | 479.00@917994 | 487.60@918101 | 107 | -1.80 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 416 | 多 | 489.60@918516 | 498.80@918620 | 104 | +1.88 | 0 | 9 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 417 | 空 | 498.40@918657 | 502.40@918693 | 36 | -0.80 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 418 | 多 | 510.80@918896 | 512.60@918980 | 84 | +0.35 | 0 | 4 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 419 | 空 | 498.80@919459 | 506.20@919469 | 10 | -1.48 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 420 | 空 | 483.00@919799 | 520.00@921837 | 2,038 | -4.05 | 7 | 138 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 421 | 空 | 467.00@926048 | 472.00@927049 | 1,001 | -0.04 | 6 | 61 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 422 | 多 | 524.60@941129 | 524.60@941187 | 58 | +0.19 | 0 | 7 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 423 | 空 | 526.40@941315 | 527.80@941338 | 23 | -0.27 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 424 | 空 | 516.80@941443 | 515.40@942316 | 873 | -0.05 | 3 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 425 | 多 | 530.00@943771 | 535.60@943851 | 80 | +1.06 | 0 | 5 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 426 | 多 | 544.40@944099 | 553.60@944349 | 250 | +2.13 | 0 | 22 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 427 | 空 | 551.80@944367 | 552.80@944377 | 10 | -0.18 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 428 | 多 | 557.80@944621 | 561.40@944701 | 80 | +0.65 | 0 | 4 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 429 | 空 | 560.20@944912 | 543.00@945000 | 88 | +2.11 | 1 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 430 | 空 | 542.60@945019 | 555.00@945423 | 404 | -1.03 | 2 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 431 | 多 | 566.00@946140 | 570.20@946285 | 145 | +0.99 | 0 | 5 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 432 | 空 | 573.40@946335 | 571.60@946374 | 39 | +0.31 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 433 | 空 | 574.40@946441 | 574.40@946449 | 8 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 434 | 空 | 571.20@946487 | 570.60@946492 | 5 | +0.10 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 435 | 多 | 582.20@946819 | 582.60@946821 | 2 | +0.07 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 436 | 多 | 582.60@946963 | 575.20@947119 | 156 | -0.41 | 0 | 7 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 437 | 空 | 558.60@947168 | 555.60@947173 | 5 | +0.54 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 438 | 空 | 529.80@947513 | 533.40@947839 | 326 | -3.35 | 4 | 11 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 439 | 空 | 551.80@948526 | 548.20@948601 | 75 | -0.25 | 2 | 8 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 440 | 空 | 497.80@958199 | 517.20@959154 | 955 | -3.41 | 5 | 72 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 441 | 多 | 593.40@962551 | 590.00@962564 | 13 | -0.57 | 0 | 0 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 442 | 空 | 605.60@962916 | 616.80@963119 | 203 | -1.85 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 443 | 空 | 600.20@963370 | 620.40@963536 | 166 | -3.37 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 444 | 多 | 623.20@963615 | 625.00@963777 | 162 | +0.83 | 0 | 12 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 445 | 多 | 660.60@964479 | 677.20@964555 | 76 | +3.09 | 0 | 5 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 446 | 空 | 692.00@964651 | 694.80@964702 | 51 | -0.52 | 1 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 447 | 多 | 699.80@964773 | 687.80@964819 | 46 | -1.71 | 0 | 3 |  | direction_flip_short | COST_REDUCING→POSITION_OPEN |
| 448 | 空 | 671.20@964904 | 671.20@964907 | 3 | -0.00 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 449 | 空 | 657.80@965176 | 661.60@965178 | 2 | -0.58 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 450 | 空 | 640.40@965499 | 657.00@965530 | 31 | -1.82 | 1 | 2 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 451 | 空 | 680.40@966534 | 675.40@966715 | 181 | +0.60 | 1 | 13 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 452 | 空 | 708.00@968700 | 706.20@968718 | 18 | +0.25 | 0 | 0 |  | direction_flip_long | POSITION_OPEN |
| 453 | 空 | 673.80@970083 | 663.40@970648 | 565 | +1.22 | 2 | 40 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 454 | 多 | 630.60@971328 | 643.40@971444 | 116 | +1.99 | 2 | 6 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 455 | 空 | 620.40@971779 | 633.80@972119 | 340 | -2.23 | 1 | 22 |  | direction_flip_long | COST_REDUCING→POSITION_OPEN |
| 456 | 空 | 600.60@980282 | 598.20@980287 | 5 | +0.40 | 0 | 0 |  | bsp_invalidate | POSITION_OPEN |
| 457 | 空 | 545.40@988403 | 538.00@989333 | 930 | +0.17 | 3 | 64 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 458 | 空 | 467.00@1005599 | 402.10@1006497 | 898 | -0.12 | 12 | 52 | Y | bsp_invalidate | COST_REDUCING→POSITION_OPEN→PRINCIPAL_WITHDRAWN |
| 459 | 多 | 247.50@1105022 | 246.80@1105038 | 16 | -0.21 | 0 | 1 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 460 | 空 | 231.20@1107962 | 241.40@1108447 | 485 | -5.55 | 2 | 25 |  | bsp_invalidate | COST_REDUCING→POSITION_OPEN |
| 461 | 空 | 222.60@1108949 | 458.20@1408881 | 299,932 | -2099.99 | 14 | 22255 | Y | eod_close | COST_REDUCING→POSITION_OPEN→PRINCIPAL_WITHDRAWN |

### FSM 状态分布

| 状态 | Bars | 占比 |
|------|------|------|
| SCANNING | 986,015 | 70.0% |
| POSITION_OPEN | 78,511 | 5.6% |
| COST_REDUCING | 46,118 | 3.3% |
| PRINCIPAL_WITHDRAWN | 298,238 | 21.2% |
| STOPPED_OUT | 0 | 0.0% |

<details><summary>事件流（58224条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 476 | 11.54 | POSITION_OPEN | ENTRY_SHORT@11.54 |
| 543 | 11.60 | COST_REDUCING | TRIM@11.60 s=5311.1 r=0.613 |
| 545 | 11.58 | POSITION_OPEN | CLOSE_DIFF@11.58 p=-106.22 |
| 546 | 11.60 | COST_REDUCING | TRIM@11.60 s=5311.1 r=0.613 |
| 548 | 11.58 | POSITION_OPEN | CLOSE_DIFF@11.58 p=-106.22 |
| 551 | 11.60 | COST_REDUCING | TRIM@11.60 s=5311.1 r=0.613 |
| 552 | 11.58 | POSITION_OPEN | CLOSE_DIFF@11.58 p=-106.22 |
| 553 | 11.60 | COST_REDUCING | TRIM@11.60 s=5311.1 r=0.613 |
| 556 | 11.58 | POSITION_OPEN | CLOSE_DIFF@11.58 p=-106.22 |
| 557 | 11.60 | COST_REDUCING | TRIM@11.60 s=5311.1 r=0.613 |
| 561 | 11.58 | POSITION_OPEN | CLOSE_DIFF@11.58 p=-106.22 |
| 563 | 11.60 | COST_REDUCING | TRIM@11.60 s=5311.1 r=0.613 |
| 564 | 11.58 | POSITION_OPEN | CLOSE_DIFF@11.58 p=-106.22 |
| 566 | 11.60 | COST_REDUCING | TRIM@11.60 s=5311.1 r=0.613 |
| 567 | 11.58 | POSITION_OPEN | CLOSE_DIFF@11.58 p=-106.22 |
| 568 | 11.60 | COST_REDUCING | TRIM@11.60 s=5311.1 r=0.613 |
| 569 | 11.58 | POSITION_OPEN | CLOSE_DIFF@11.58 p=-106.22 |
| 571 | 11.60 | COST_REDUCING | TRIM@11.60 s=5311.1 r=0.613 |
| 572 | 11.58 | POSITION_OPEN | CLOSE_DIFF@11.58 p=-106.22 |
| 575 | 11.60 | COST_REDUCING | TRIM@11.60 s=5311.1 r=0.613 |
| 577 | 11.58 | POSITION_OPEN | CLOSE_DIFF@11.58 p=-106.22 |
| 578 | 11.60 | COST_REDUCING | TRIM@11.60 s=5311.1 r=0.613 |
| 579 | 11.58 | POSITION_OPEN | CLOSE_DIFF@11.58 p=-106.22 |
| 583 | 11.60 | COST_REDUCING | TRIM@11.60 s=5311.1 r=0.613 |
| 584 | 11.58 | POSITION_OPEN | CLOSE_DIFF@11.58 p=-106.22 |
| 615 | 11.56 | COST_REDUCING | TRIM@11.56 s=5311.1 r=0.613 |
| 656 | 11.60 | POSITION_OPEN | CLOSE_DIFF@11.60 p=212.44 |
| 690 | 11.62 | COST_REDUCING | TRIM@11.62 s=5311.1 r=0.613 |
| 722 | 11.58 | POSITION_OPEN | CLOSE_DIFF@11.58 p=-212.44 |
| 739 | 11.58 | COST_REDUCING | TRIM@11.58 s=5311.1 r=0.613 |
| 746 | 11.60 | POSITION_OPEN | CLOSE_DIFF@11.60 p=106.22 |
| 778 | 11.48 | COST_REDUCING | TRIM@11.48 s=5311.1 r=0.613 |
| 782 | 11.44 | POSITION_OPEN | CLOSE_DIFF@11.44 p=-212.44 |
| 788 | 11.44 | COST_REDUCING | TRIM@11.44 s=5311.1 r=0.613 |
| 796 | 11.40 | POSITION_OPEN | CLOSE_DIFF@11.40 p=-212.44 |
| 838 | 11.40 | COST_REDUCING | TRIM@11.40 s=7267.8 r=0.839 |
| 912 | 10.96 | POSITION_OPEN | CLOSE_DIFF@10.96 p=-3197.85 |
| 913 | 10.98 | COST_REDUCING | TRIM@10.98 s=5596.5 r=0.646 |
| 943 | 11.02 | POSITION_OPEN | CLOSE_DIFF@11.02 p=223.86 |
| 976 | 11.06 | COST_REDUCING | TRIM@11.06 s=6412.6 r=0.646 |
| 1,001 | 11.02 | POSITION_OPEN | CLOSE_DIFF@11.02 p=-256.51 |
| 1,023 | 11.04 | COST_REDUCING | TRIM@11.04 s=10170.6 r=0.833 |
| 1,027 | 11.02 | POSITION_OPEN | CLOSE_DIFF@11.02 p=-203.41 |
| 1,028 | 11.04 | COST_REDUCING | TRIM@11.04 s=10170.6 r=0.833 |
| 1,030 | 11.02 | POSITION_OPEN | CLOSE_DIFF@11.02 p=-203.41 |
| 1,031 | 11.04 | COST_REDUCING | TRIM@11.04 s=10170.6 r=0.833 |
| 1,036 | 11.06 | SCANNING | STOP:direction_flip_long@11.06 pnl=+2.47% |
| 1,567 | 10.84 | POSITION_OPEN | ENTRY_SHORT@10.84 |
| 1,584 | 10.76 | COST_REDUCING | TRIM@10.76 s=8218.7 r=0.891 |
| 1,587 | 10.72 | POSITION_OPEN | CLOSE_DIFF@10.72 p=-328.75 |
| 1,588 | 10.76 | COST_REDUCING | TRIM@10.76 s=8667.0 r=0.891 |
| 1,602 | 10.54 | POSITION_OPEN | CLOSE_DIFF@10.54 p=-1906.74 |
| 1,604 | 10.60 | COST_REDUCING | TRIM@10.60 s=8492.9 r=0.873 |
| 1,612 | 10.76 | SCANNING | STOP:direction_flip_long@10.76 pnl=+0.68% |
| 2,115 | 11.32 | POSITION_OPEN | ENTRY_LONG@11.32 |
| 2,127 | 11.34 | COST_REDUCING | TRIM@11.34 s=6309.9 r=0.714 |
| 2,161 | 11.34 | POSITION_OPEN | CLOSE_DIFF@11.34 p=0.00 |
| 2,162 | 11.32 | COST_REDUCING | TRIM@11.32 s=6309.9 r=0.714 |
| 2,164 | 11.34 | POSITION_OPEN | CLOSE_DIFF@11.34 p=-126.20 |
| 2,188 | 11.34 | COST_REDUCING | TRIM@11.34 s=6309.9 r=0.714 |
| 2,192 | 11.34 | POSITION_OPEN | CLOSE_DIFF@11.34 p=0.00 |
| 2,198 | 11.28 | COST_REDUCING | TRIM@11.28 s=9414.8 r=0.714 |
| 2,199 | 11.34 | POSITION_OPEN | CLOSE_DIFF@11.34 p=-564.89 |
| 2,217 | 11.34 | COST_REDUCING | TRIM@11.34 s=9414.8 r=0.714 |
| 2,221 | 11.36 | POSITION_OPEN | CLOSE_DIFF@11.36 p=-188.30 |
| 2,223 | 11.34 | COST_REDUCING | TRIM@11.34 s=9414.8 r=0.714 |
| 2,224 | 11.34 | POSITION_OPEN | CLOSE_DIFF@11.34 p=0.00 |
| 2,237 | 11.32 | COST_REDUCING | TRIM@11.32 s=9414.8 r=0.714 |
| 2,253 | 11.34 | POSITION_OPEN | CLOSE_DIFF@11.34 p=-188.30 |
| 2,261 | 11.32 | COST_REDUCING | TRIM@11.32 s=9414.8 r=0.714 |
| 2,262 | 11.34 | POSITION_OPEN | CLOSE_DIFF@11.34 p=-188.30 |
| 2,270 | 11.36 | COST_REDUCING | TRIM@11.36 s=9414.8 r=0.714 |
| 2,280 | 11.38 | POSITION_OPEN | CLOSE_DIFF@11.38 p=-188.30 |
| 2,287 | 11.36 | COST_REDUCING | TRIM@11.36 s=9414.8 r=0.714 |
| 2,292 | 11.38 | POSITION_OPEN | CLOSE_DIFF@11.38 p=-188.30 |
| 2,298 | 11.36 | COST_REDUCING | TRIM@11.36 s=9414.8 r=0.714 |
| 2,300 | 11.38 | POSITION_OPEN | CLOSE_DIFF@11.38 p=-188.30 |
| 2,301 | 11.36 | COST_REDUCING | TRIM@11.36 s=9414.8 r=0.714 |
| 2,302 | 11.38 | POSITION_OPEN | CLOSE_DIFF@11.38 p=-188.30 |
| 2,304 | 11.36 | COST_REDUCING | TRIM@11.36 s=9414.8 r=0.714 |
| ... | ... | ... | （共58224条） |
</details>

## 汇总

| 标的 | Bars | BH% | FSM% | 胜率 | 交易 | 加仓 | 降成本 | PW | 耗时 |
|------|------|-----|------|------|------|------|--------|-----|------|
| QQQ | 728,030 | +174.6 | -19.29 | 39% | 824 | 175/824 | 304/824 | 1/824 | 187s |
| OKLO | 333,613 | +251.3 | -97.78 | 43% | 174 | 59/174 | 100/174 | 4/174 | 47s |
| HK700 | 1,408,882 | +3933.5 | -1002.76 | 41% | 461 | 142/461 | 269/461 | 3/461 | 342s |

## 结果包

**结论**：完整版 5 状态 FSM 在 1min 级别 3 标的。加仓/降成本/本金回收全实装。

**定义依据**：FSM 5状态(267号)，加仓/降成本 ratio 零参数(alive persistence 结构)。

**边界条件**：
- PENDING_EXPIRY=390 bars，BSP_WINDOW=500 笔
- 无滑点/手续费，1min 噪声交易多

**下游推论**：降成本周期更多(1min 级别 settle 频繁)，但本金回收率仍低。

**谱系**：267号(FSM)，268a号(own_capital)，§7.5(merge tree)，Elder rule。

**影响**：独立脚本，不修改 FSM 主代码。

**认识论等级**：L2。