# 完整版多层赋格 FSM 回测（H组）

## 架构

5状态 FSM：`SCANNING → POSITION_OPEN → COST_REDUCING → PRINCIPAL_WITHDRAWN → STOPPED_OUT`

| 维度 | 实现 | 参数来源 |
|------|------|---------|
| 方向 | rank-1 settle（elder rule 约束下的因果信号） | 零参数 |
| 进场 | chanlun candidate + PH settle 门控 | 零参数 |
| 加仓 | 同方向 type2/type3 BSP | ratio = alive_rank1 / dominant |
| 降成本 | non-rank1 settle → 减仓 | ratio = alive_rank1 / dominant |
| 回补 | 同方向 settle 或 BSP → 买回 | ≤ 已减量 |
| 本金回收 | cumulative_recovered ≥ own_capital | 自动阈值 |
| 清仓 | rank-1 settle（方向翻转）或 BSP invalidate | 零参数 |
| 双向 | 多/空���称 | — |
| 多层 | 所有 alive components 参与 | — |

## QQQ

- 数据：1500 bars, 2020-06-10 → 2026-05-29
- 价格：246.22 → 738.31
- Buy-and-hold: **+199.86%**

### 总体指标

| 指标 | 值 |
|------|-----|
| 交易数 | 18 (多9/空9) |
| 胜率 | 27.8% |
| 平均收益 | -0.478% |
| 复利累计 | -8.89% |
| 最大回撤 | -12.72% |
| 平均持仓 | 8 bars |
| 有加仓的交易 | 0/18 |
| 有降成本的交易 | 1/18 |
| 达到本金回收 | 0/18 |

### 交易明细

| # | 方向 | 入场 | 出场 | 持仓 | PnL% | 加仓 | 短差 | 本金回收 | 退出原因 | 状态路径 |
|---|------|------|------|------|------|------|------|---------|---------|---------|
| 1 | 多 | 275.32@47 | 287.41@60 | 13d | +4.39 | 0 | 0 | 否 | direction_flip_short | POSITION_OPEN |
| 2 | 多 | 323.77@154 | 319.43@159 | 5d | -1.34 | 0 | 0 | 否 | direction_flip_short | POSITION_OPEN |
| 3 | 空 | 314.56@161 | 330.24@165 | 4d | -4.98 | 0 | 0 | 否 | direction_flip_long | POSITION_OPEN→COST_REDUCING |
| 4 | 多 | 337.11@209 | 336.51@212 | 3d | -0.18 | 0 | 0 | 否 | direction_flip_short | POSITION_OPEN |
| 5 | 空 | 366.84@294 | 368.82@297 | 3d | -0.54 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 6 | 多 | 373.23@303 | 379.47@315 | 12d | +1.67 | 0 | 0 | 否 | direction_flip_short | POSITION_OPEN |
| 7 | 空 | 380.69@386 | 385.82@401 | 15d | -1.35 | 0 | 1 | 否 | bsp_invalidate | POSITION_OPEN→COST_REDUCING |
| 8 | 空 | 338.08@429 | 337.30@437 | 8d | +0.23 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN→COST_REDUCING |
| 9 | 多 | 429.01@918 | 427.32@929 | 11d | -0.39 | 0 | 0 | 否 | direction_flip_short | POSITION_OPEN |
| 10 | 空 | 435.27@935 | 439.00@936 | 1d | -0.86 | 0 | 0 | 否 | direction_flip_long | POSITION_OPEN |
| 11 | 空 | 433.92@947 | 437.48@948 | 1d | -0.82 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 12 | 多 | 505.58@1110 | 520.60@1133 | 23d | +2.97 | 0 | 0 | 否 | direction_flip_short | POSITION_OPEN |
| 13 | 多 | 539.37@1178 | 500.27@1185 | 7d | -7.25 | 0 | 0 | 否 | direction_flip_short | POSITION_OPEN→COST_REDUCING |
| 14 | 多 | 539.78@1265 | 561.25@1284 | 19d | +3.98 | 0 | 0 | 否 | direction_flip_short | POSITION_OPEN |
| 15 | 空 | 565.01@1291 | 569.24@1296 | 5d | -0.75 | 0 | 0 | 否 | direction_flip_long | POSITION_OPEN |
| 16 | 空 | 569.28@1304 | 571.97@1307 | 3d | -0.47 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 17 | 多 | 603.25@1334 | 589.50@1341 | 7d | -2.28 | 0 | 0 | 否 | direction_flip_short | POSITION_OPEN |
| 18 | 空 | 619.25@1358 | 623.23@1362 | 4d | -0.64 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |

### FSM 状态分布（按 bar 数）

| 状态 | Bars | 占比 |
|------|------|------|
| SCANNING | 1356 | 90.4% |
| POSITION_OPEN | 133 | 8.9% |
| COST_REDUCING | 11 | 0.7% |
| PRINCIPAL_WITHDRAWN | 0 | 0.0% |
| STOPPED_OUT | 0 | 0.0% |

<details><summary>事件流（41条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 47 | 275.32 | POSITION_OPEN | ENTRY_LONG@275.32 |
| 60 | 287.41 | SCANNING | STOP:direction_flip_short@287.41 pnl=+4.39% |
| 154 | 323.77 | POSITION_OPEN | ENTRY_LONG@323.77 |
| 159 | 319.43 | SCANNING | STOP:direction_flip_short@319.43 pnl=-1.34% |
| 161 | 314.56 | POSITION_OPEN | ENTRY_SHORT@314.56 |
| 162 | 322.42 | COST_REDUCING | TRIM@322.42 shares=47.2 ratio=0.148 |
| 165 | 330.24 | SCANNING | STOP:direction_flip_long@330.24 pnl=-4.98% |
| 209 | 337.11 | POSITION_OPEN | ENTRY_LONG@337.11 |
| 212 | 336.51 | SCANNING | STOP:direction_flip_short@336.51 pnl=-0.18% |
| 294 | 366.84 | POSITION_OPEN | ENTRY_SHORT@366.84 |
| 297 | 368.82 | SCANNING | STOP:bsp_invalidate@368.82 pnl=-0.54% |
| 303 | 373.23 | POSITION_OPEN | ENTRY_LONG@373.23 |
| 315 | 379.47 | SCANNING | STOP:direction_flip_short@379.47 pnl=+1.67% |
| 386 | 380.69 | POSITION_OPEN | ENTRY_SHORT@380.69 |
| 390 | 403.48 | COST_REDUCING | TRIM@403.48 shares=36.0 ratio=0.137 |
| 392 | 401.55 | POSITION_OPEN | CLOSE_DIFF@401.55 profit=-69.50 |
| 401 | 385.82 | SCANNING | STOP:bsp_invalidate@385.82 pnl=-1.35% |
| 429 | 338.08 | POSITION_OPEN | ENTRY_SHORT@338.08 |
| 435 | 347.22 | COST_REDUCING | TRIM@347.22 shares=129.8 ratio=0.439 |
| 437 | 337.30 | SCANNING | STOP:bsp_invalidate@337.30 pnl=+0.23% |
| 918 | 429.01 | POSITION_OPEN | ENTRY_LONG@429.01 |
| 929 | 427.32 | SCANNING | STOP:direction_flip_short@427.32 pnl=-0.39% |
| 935 | 435.27 | POSITION_OPEN | ENTRY_SHORT@435.27 |
| 936 | 439.00 | SCANNING | STOP:direction_flip_long@439.00 pnl=-0.86% |
| 947 | 433.92 | POSITION_OPEN | ENTRY_SHORT@433.92 |
| 948 | 437.48 | SCANNING | STOP:bsp_invalidate@437.48 pnl=-0.82% |
| 1110 | 505.58 | POSITION_OPEN | ENTRY_LONG@505.58 |
| 1133 | 520.60 | SCANNING | STOP:direction_flip_short@520.60 pnl=+2.97% |
| 1178 | 539.37 | POSITION_OPEN | ENTRY_LONG@539.37 |
| 1181 | 526.08 | COST_REDUCING | TRIM@526.08 shares=184.6 ratio=0.996 |
| 1185 | 500.27 | SCANNING | STOP:direction_flip_short@500.27 pnl=-7.25% |
| 1265 | 539.78 | POSITION_OPEN | ENTRY_LONG@539.78 |
| 1284 | 561.25 | SCANNING | STOP:direction_flip_short@561.25 pnl=+3.98% |
| 1291 | 565.01 | POSITION_OPEN | ENTRY_SHORT@565.01 |
| 1296 | 569.24 | SCANNING | STOP:direction_flip_long@569.24 pnl=-0.75% |
| 1304 | 569.28 | POSITION_OPEN | ENTRY_SHORT@569.28 |
| 1307 | 571.97 | SCANNING | STOP:bsp_invalidate@571.97 pnl=-0.47% |
| 1334 | 603.25 | POSITION_OPEN | ENTRY_LONG@603.25 |
| 1341 | 589.50 | SCANNING | STOP:direction_flip_short@589.50 pnl=-2.28% |
| 1358 | 619.25 | POSITION_OPEN | ENTRY_SHORT@619.25 |
| 1362 | 623.23 | SCANNING | STOP:bsp_invalidate@623.23 pnl=-0.64% |
</details>

## SPY

- 数据：1500 bars, 2020-06-10T00:00:00 → 2026-05-29T00:00:00
- 价格：319.00 → 756.48
- Buy-and-hold: **+137.14%**

### 总体指标

| 指标 | 值 |
|------|-----|
| 交易数 | 21 (多10/空11) |
| 胜率 | 19.0% |
| 平均收益 | -0.818% |
| 复利累计 | -16.02% |
| 最大回撤 | -16.02% |
| 平均持仓 | 8 bars |
| 有加仓的交易 | 0/21 |
| 有降成本的交易 | 1/21 |
| 达到本金回收 | 0/21 |

### 交易明细

| # | 方向 | 入场 | 出场 | 持仓 | PnL% | 加仓 | 短差 | 本金回收 | 退出原因 | 状态路径 |
|---|------|------|------|------|------|------|------|---------|---------|---------|
| 1 | 空 | 362.06@120 | 366.02@121 | 1d | -1.09 | 0 | 0 | 否 | direction_flip_long | POSITION_OPEN |
| 2 | 空 | 366.85@127 | 370.17@132 | 5d | -0.91 | 0 | 0 | 否 | direction_flip_long | POSITION_OPEN |
| 3 | 多 | 383.89@154 | 374.41@159 | 5d | -2.47 | 0 | 0 | 否 | direction_flip_short | POSITION_OPEN |
| 4 | 多 | 393.53@189 | 391.48@194 | 5d | -0.52 | 0 | 0 | 否 | direction_flip_short | POSITION_OPEN |
| 5 | 多 | 422.60@248 | 421.65@251 | 3d | -0.22 | 0 | 0 | 否 | direction_flip_short | POSITION_OPEN |
| 6 | 多 | 453.59@345 | 467.57@367 | 22d | +3.08 | 0 | 0 | 否 | direction_flip_short | POSITION_OPEN |
| 7 | 多 | 470.74@380 | 459.87@385 | 5d | -2.31 | 0 | 0 | 否 | direction_flip_short | POSITION_OPEN→COST_REDUCING |
| 8 | 空 | 366.65@509 | 380.34@517 | 8d | -3.73 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN→COST_REDUCING |
| 9 | 空 | 482.88@916 | 494.35@918 | 2d | -2.38 | 0 | 0 | 否 | direction_flip_long | POSITION_OPEN |
| 10 | 多 | 507.50@931 | 509.83@947 | 16d | +0.46 | 0 | 0 | 否 | direction_flip_short | POSITION_OPEN |
| 11 | 空 | 513.07@960 | 519.32@963 | 3d | -1.22 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 12 | 空 | 568.62@1084 | 573.17@1089 | 5d | -0.80 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 13 | 空 | 577.99@1100 | 580.01@1105 | 5d | -0.35 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 14 | 多 | 591.04@1110 | 593.35@1116 | 6d | +0.39 | 0 | 0 | 否 | direction_flip_short | POSITION_OPEN |
| 15 | 空 | 604.68@1132 | 606.79@1137 | 5d | -0.35 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 16 | 多 | 609.75@1161 | 597.21@1182 | 21d | -2.06 | 0 | 1 | 否 | direction_flip_short | POSITION_OPEN→COST_REDUCING |
| 17 | 多 | 614.91@1268 | 621.72@1292 | 24d | +1.11 | 0 | 0 | 否 | direction_flip_short | POSITION_OPEN→COST_REDUCING |
| 18 | 多 | 677.25@1351 | 675.24@1358 | 7d | -0.30 | 0 | 0 | 否 | direction_flip_short | POSITION_OPEN |
| 19 | 空 | 665.67@1367 | 680.27@1376 | 9d | -2.19 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN→COST_REDUCING |
| 20 | 空 | 677.58@1409 | 685.40@1410 | 1d | -1.15 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 21 | 空 | 689.53@1419 | 690.62@1422 | 3d | -0.16 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |

### FSM 状态分布（按 bar 数）

| 状态 | Bars | 占比 |
|------|------|------|
| SCANNING | 1339 | 89.3% |
| POSITION_OPEN | 133 | 8.9% |
| COST_REDUCING | 28 | 1.9% |
| PRINCIPAL_WITHDRAWN | 0 | 0.0% |
| STOPPED_OUT | 0 | 0.0% |

<details><summary>事件流（49条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 120 | 362.06 | POSITION_OPEN | ENTRY_SHORT@362.06 |
| 121 | 366.02 | SCANNING | STOP:direction_flip_long@366.02 pnl=-1.09% |
| 127 | 366.85 | POSITION_OPEN | ENTRY_SHORT@366.85 |
| 132 | 370.17 | SCANNING | STOP:direction_flip_long@370.17 pnl=-0.91% |
| 154 | 383.89 | POSITION_OPEN | ENTRY_LONG@383.89 |
| 159 | 374.41 | SCANNING | STOP:direction_flip_short@374.41 pnl=-2.47% |
| 189 | 393.53 | POSITION_OPEN | ENTRY_LONG@393.53 |
| 194 | 391.48 | SCANNING | STOP:direction_flip_short@391.48 pnl=-0.52% |
| 248 | 422.60 | POSITION_OPEN | ENTRY_LONG@422.60 |
| 251 | 421.65 | SCANNING | STOP:direction_flip_short@421.65 pnl=-0.22% |
| 345 | 453.59 | POSITION_OPEN | ENTRY_LONG@453.59 |
| 367 | 467.57 | SCANNING | STOP:direction_flip_short@467.57 pnl=+3.08% |
| 380 | 470.74 | POSITION_OPEN | ENTRY_LONG@470.74 |
| 382 | 463.36 | COST_REDUCING | TRIM@463.36 shares=211.2 ratio=0.994 |
| 385 | 459.87 | SCANNING | STOP:direction_flip_short@459.87 pnl=-2.31% |
| 509 | 366.65 | POSITION_OPEN | ENTRY_SHORT@366.65 |
| 513 | 378.06 | COST_REDUCING | TRIM@378.06 shares=171.7 ratio=0.630 |
| 517 | 380.34 | SCANNING | STOP:bsp_invalidate@380.34 pnl=-3.73% |
| 916 | 482.88 | POSITION_OPEN | ENTRY_SHORT@482.88 |
| 918 | 494.35 | SCANNING | STOP:direction_flip_long@494.35 pnl=-2.38% |
| 931 | 507.50 | POSITION_OPEN | ENTRY_LONG@507.50 |
| 947 | 509.83 | SCANNING | STOP:direction_flip_short@509.83 pnl=+0.46% |
| 960 | 513.07 | POSITION_OPEN | ENTRY_SHORT@513.07 |
| 963 | 519.32 | SCANNING | STOP:bsp_invalidate@519.32 pnl=-1.22% |
| 1084 | 568.62 | POSITION_OPEN | ENTRY_SHORT@568.62 |
| 1089 | 573.17 | SCANNING | STOP:bsp_invalidate@573.17 pnl=-0.80% |
| 1100 | 577.99 | POSITION_OPEN | ENTRY_SHORT@577.99 |
| 1105 | 580.01 | SCANNING | STOP:bsp_invalidate@580.01 pnl=-0.35% |
| 1110 | 591.04 | POSITION_OPEN | ENTRY_LONG@591.04 |
| 1116 | 593.35 | SCANNING | STOP:direction_flip_short@593.35 pnl=+0.39% |
| 1132 | 604.68 | POSITION_OPEN | ENTRY_SHORT@604.68 |
| 1137 | 606.79 | SCANNING | STOP:bsp_invalidate@606.79 pnl=-0.35% |
| 1161 | 609.75 | POSITION_OPEN | ENTRY_LONG@609.75 |
| 1168 | 597.77 | COST_REDUCING | TRIM@597.77 shares=163.0 ratio=0.994 |
| 1171 | 606.32 | POSITION_OPEN | CLOSE_DIFF@606.32 profit=-1393.43 |
| 1181 | 599.94 | COST_REDUCING | TRIM@599.94 shares=162.3 ratio=0.990 |
| 1182 | 597.21 | SCANNING | STOP:direction_flip_short@597.21 pnl=-2.06% |
| 1268 | 614.91 | POSITION_OPEN | ENTRY_LONG@614.91 |
| 1279 | 622.14 | COST_REDUCING | TRIM@622.14 shares=162.4 ratio=0.999 |
| 1292 | 621.72 | SCANNING | STOP:direction_flip_short@621.72 pnl=+1.11% |
| 1351 | 677.25 | POSITION_OPEN | ENTRY_LONG@677.25 |
| 1358 | 675.24 | SCANNING | STOP:direction_flip_short@675.24 pnl=-0.30% |
| 1367 | 665.67 | POSITION_OPEN | ENTRY_SHORT@665.67 |
| 1372 | 668.73 | COST_REDUCING | TRIM@668.73 shares=13.5 ratio=0.090 |
| 1376 | 680.27 | SCANNING | STOP:bsp_invalidate@680.27 pnl=-2.19% |
| 1409 | 677.58 | POSITION_OPEN | ENTRY_SHORT@677.58 |
| 1410 | 685.40 | SCANNING | STOP:bsp_invalidate@685.40 pnl=-1.15% |
| 1419 | 689.53 | POSITION_OPEN | ENTRY_SHORT@689.53 |
| 1422 | 690.62 | SCANNING | STOP:bsp_invalidate@690.62 pnl=-0.16% |
</details>

## GLD

- 数据：1500 bars, 2020-06-10T00:00:00 → 2026-05-29T00:00:00
- 价格：163.57 → 417.12
- Buy-and-hold: **+155.01%**

### 总体指标

| 指标 | 值 |
|------|-----|
| 交易数 | 13 (多8/空5) |
| 胜率 | 53.8% |
| 平均收益 | -1.072% |
| 复利累计 | -13.43% |
| 最大回撤 | -13.43% |
| 平均持仓 | 10 bars |
| 有加仓的交易 | 0/13 |
| 有降成本的交易 | 4/13 |
| 达到本金回收 | 0/13 |

### 交易明细

| # | 方向 | 入场 | 出场 | 持仓 | PnL% | 加仓 | 短差 | 本金回收 | 退出原因 | 状态路径 |
|---|------|------|------|------|------|------|------|---------|---------|---------|
| 1 | 空 | 154.98@571 | 158.43@583 | 12d | -2.23 | 0 | 0 | 否 | direction_flip_long | POSITION_OPEN |
| 2 | 多 | 163.48@611 | 163.92@616 | 5d | +0.27 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 3 | 多 | 196.01@938 | 199.71@947 | 9d | +1.89 | 0 | 0 | 否 | direction_flip_short | POSITION_OPEN |
| 4 | 多 | 223.66@991 | 211.60@1005 | 14d | -5.39 | 0 | 3 | 否 | direction_flip_short | POSITION_OPEN→COST_REDUCING |
| 5 | 多 | 228.29@1030 | 229.37@1057 | 27d | +0.47 | 0 | 1 | 否 | direction_flip_short | POSITION_OPEN→COST_REDUCING |
| 6 | 空 | 245.02@1082 | 245.00@1087 | 5d | +0.01 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 7 | 多 | 247.15@1095 | 250.87@1100 | 5d | +1.51 | 0 | 0 | 否 | direction_flip_short | POSITION_OPEN |
| 8 | 多 | 281.97@1205 | 286.42@1210 | 5d | +1.58 | 0 | 0 | 否 | direction_flip_short | POSITION_OPEN |
| 9 | 空 | 303.65@1223 | 297.98@1230 | 7d | +1.87 | 0 | 1 | 否 | bsp_invalidate | POSITION_OPEN→COST_REDUCING |
| 10 | 多 | 316.29@1259 | 300.96@1290 | 31d | -3.99 | 0 | 2 | 否 | direction_flip_short | POSITION_OPEN→COST_REDUCING |
| 11 | 空 | 362.32@1358 | 375.96@1366 | 8d | -3.76 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN→COST_REDUCING |
| 12 | 多 | 408.23@1391 | 398.60@1395 | 4d | -2.36 | 0 | 0 | 否 | direction_flip_short | POSITION_OPEN |
| 13 | 空 | 421.29@1408 | 437.23@1409 | 1d | -3.78 | 0 | 0 | 否 | direction_flip_long | POSITION_OPEN |

### FSM 状态分布（按 bar 数）

| 状态 | Bars | 占比 |
|------|------|------|
| SCANNING | 1367 | 91.1% |
| POSITION_OPEN | 99 | 6.6% |
| COST_REDUCING | 34 | 2.3% |
| PRINCIPAL_WITHDRAWN | 0 | 0.0% |
| STOPPED_OUT | 0 | 0.0% |

<details><summary>事件流（42条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 571 | 154.98 | POSITION_OPEN | ENTRY_SHORT@154.98 |
| 583 | 158.43 | SCANNING | STOP:direction_flip_long@158.43 pnl=-2.23% |
| 611 | 163.48 | POSITION_OPEN | ENTRY_LONG@163.48 |
| 616 | 163.92 | SCANNING | STOP:bsp_invalidate@163.92 pnl=+0.27% |
| 938 | 196.01 | POSITION_OPEN | ENTRY_LONG@196.01 |
| 947 | 199.71 | SCANNING | STOP:direction_flip_short@199.71 pnl=+1.89% |
| 991 | 223.66 | POSITION_OPEN | ENTRY_LONG@223.66 |
| 995 | 215.72 | COST_REDUCING | TRIM@215.72 shares=426.7 ratio=0.954 |
| 999 | 216.57 | POSITION_OPEN | CLOSE_DIFF@216.57 profit=-362.73 |
| 1000 | 215.30 | COST_REDUCING | TRIM@215.30 shares=426.7 ratio=0.954 |
| 1001 | 217.22 | POSITION_OPEN | CLOSE_DIFF@217.22 profit=-819.35 |
| 1002 | 215.27 | COST_REDUCING | TRIM@215.27 shares=426.7 ratio=0.954 |
| 1003 | 217.82 | POSITION_OPEN | CLOSE_DIFF@217.82 profit=-1088.19 |
| 1005 | 211.60 | SCANNING | STOP:direction_flip_short@211.60 pnl=-5.39% |
| 1030 | 228.29 | POSITION_OPEN | ENTRY_LONG@228.29 |
| 1033 | 221.73 | COST_REDUCING | TRIM@221.73 shares=416.8 ratio=0.952 |
| 1040 | 222.52 | POSITION_OPEN | CLOSE_DIFF@222.52 profit=-329.30 |
| 1057 | 229.37 | SCANNING | STOP:direction_flip_short@229.37 pnl=+0.47% |
| 1082 | 245.02 | POSITION_OPEN | ENTRY_SHORT@245.02 |
| 1087 | 245.00 | SCANNING | STOP:bsp_invalidate@245.00 pnl=+0.01% |
| 1095 | 247.15 | POSITION_OPEN | ENTRY_LONG@247.15 |
| 1100 | 250.87 | SCANNING | STOP:direction_flip_short@250.87 pnl=+1.51% |
| 1205 | 281.97 | POSITION_OPEN | ENTRY_LONG@281.97 |
| 1210 | 286.42 | SCANNING | STOP:direction_flip_short@286.42 pnl=+1.58% |
| 1223 | 303.65 | POSITION_OPEN | ENTRY_SHORT@303.65 |
| 1226 | 309.07 | COST_REDUCING | TRIM@309.07 shares=23.9 ratio=0.073 |
| 1228 | 303.77 | POSITION_OPEN | CLOSE_DIFF@303.77 profit=-126.80 |
| 1230 | 297.98 | SCANNING | STOP:bsp_invalidate@297.98 pnl=+1.87% |
| 1259 | 316.29 | POSITION_OPEN | ENTRY_LONG@316.29 |
| 1262 | 310.26 | COST_REDUCING | TRIM@310.26 shares=314.8 ratio=0.996 |
| 1270 | 307.55 | POSITION_OPEN | CLOSE_DIFF@307.55 profit=853.18 |
| 1274 | 304.16 | COST_REDUCING | TRIM@304.16 shares=314.8 ratio=0.996 |
| 1277 | 309.14 | POSITION_OPEN | CLOSE_DIFF@309.14 profit=-1567.83 |
| 1287 | 307.40 | COST_REDUCING | TRIM@307.40 shares=315.8 ratio=0.999 |
| 1290 | 300.96 | SCANNING | STOP:direction_flip_short@300.96 pnl=-3.99% |
| 1358 | 362.32 | POSITION_OPEN | ENTRY_SHORT@362.32 |
| 1361 | 368.31 | COST_REDUCING | TRIM@368.31 shares=44.7 ratio=0.162 |
| 1366 | 375.96 | SCANNING | STOP:bsp_invalidate@375.96 pnl=-3.76% |
| 1391 | 408.23 | POSITION_OPEN | ENTRY_LONG@408.23 |
| 1395 | 398.60 | SCANNING | STOP:direction_flip_short@398.60 pnl=-2.36% |
| 1408 | 421.29 | POSITION_OPEN | ENTRY_SHORT@421.29 |
| 1409 | 437.23 | SCANNING | STOP:direction_flip_long@437.23 pnl=-3.78% |
</details>

## BRENT

- 数据：1500 bars, 2020-06-19 → 2026-05-26
- 价格：42.33 → 102.75
- Buy-and-hold: **+142.74%**

### 总体指标

| 指标 | 值 |
|------|-----|
| 交易数 | 5 (多1/空4) |
| 胜率 | 20.0% |
| 平均收益 | -31.891% |
| 复利累计 | -150.15% |
| 最大回撤 | -150.15% |
| 平均持仓 | 89 bars |
| 有加仓的交易 | 2/5 |
| 有降成本的交易 | 1/5 |
| 达到本金回收 | 0/5 |

### 交易明细

| # | 方向 | 入场 | 出场 | 持仓 | PnL% | 加仓 | 短差 | 本金回收 | 退出原因 | 状态路径 |
|---|------|------|------|------|------|------|------|---------|---------|---------|
| 1 | 空 | 37.86@92 | 42.25@101 | 9d | -11.60 | 0 | 0 | 否 | direction_flip_long | POSITION_OPEN→COST_REDUCING |
| 2 | 多 | 56.42@158 | 62.11@191 | 33d | +8.51 | 1 | 0 | 否 | direction_flip_short | POSITION_OPEN |
| 3 | 空 | 68.61@296 | 69.07@299 | 3d | -0.67 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 4 | 空 | 84.29@574 | 85.97@576 | 2d | -1.99 | 0 | 0 | 否 | bsp_invalidate | POSITION_OPEN |
| 5 | 空 | 70.34@1069 | 138.21@1466 | 397d | -153.70 | 2 | 31 | 否 | direction_flip_long | POSITION_OPEN→COST_REDUCING |

### FSM 状态分布（按 bar 数）

| 状态 | Bars | 占比 |
|------|------|------|
| SCANNING | 1056 | 70.4% |
| POSITION_OPEN | 238 | 15.9% |
| COST_REDUCING | 206 | 13.7% |
| PRINCIPAL_WITHDRAWN | 0 | 0.0% |
| STOPPED_OUT | 0 | 0.0% |

<details><summary>事件流（75条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 92 | 37.86 | POSITION_OPEN | ENTRY_SHORT@37.86 |
| 100 | 40.93 | COST_REDUCING | TRIM@40.93 shares=2185.6 ratio=0.827 |
| 101 | 42.25 | SCANNING | STOP:direction_flip_long@42.25 pnl=-11.60% |
| 158 | 56.42 | POSITION_OPEN | ENTRY_LONG@56.42 |
| 180 | 64.70 | POSITION_OPEN | ADD_POS:type3@64.70 +213.7shares ratio=0.121 |
| 191 | 62.11 | SCANNING | STOP:direction_flip_short@62.11 pnl=+8.51% |
| 296 | 68.61 | POSITION_OPEN | ENTRY_SHORT@68.61 |
| 299 | 69.07 | SCANNING | STOP:bsp_invalidate@69.07 pnl=-0.67% |
| 574 | 84.29 | POSITION_OPEN | ENTRY_SHORT@84.29 |
| 576 | 85.97 | SCANNING | STOP:bsp_invalidate@85.97 pnl=-1.99% |
| 1069 | 70.34 | POSITION_OPEN | ENTRY_SHORT@70.34 |
| 1073 | 73.96 | COST_REDUCING | TRIM@73.96 shares=922.9 ratio=0.649 |
| 1081 | 73.06 | POSITION_OPEN | CLOSE_DIFF@73.06 profit=-830.58 |
| 1086 | 77.57 | COST_REDUCING | TRIM@77.57 shares=922.9 ratio=0.649 |
| 1094 | 73.68 | POSITION_OPEN | CLOSE_DIFF@73.68 profit=-3589.97 |
| 1099 | 75.59 | COST_REDUCING | TRIM@75.59 shares=922.9 ratio=0.649 |
| 1103 | 71.87 | POSITION_OPEN | CLOSE_DIFF@71.87 profit=-3433.08 |
| 1109 | 76.98 | COST_REDUCING | TRIM@76.98 shares=922.9 ratio=0.649 |
| 1123 | 74.27 | POSITION_OPEN | CLOSE_DIFF@74.27 profit=-2500.98 |
| 1129 | 74.80 | COST_REDUCING | TRIM@74.80 shares=922.9 ratio=0.649 |
| 1136 | 73.52 | POSITION_OPEN | CLOSE_DIFF@73.52 profit=-1181.28 |
| 1137 | 74.89 | COST_REDUCING | TRIM@74.89 shares=922.9 ratio=0.649 |
| 1139 | 73.16 | POSITION_OPEN | CLOSE_DIFF@73.16 profit=-1596.57 |
| 1147 | 74.58 | COST_REDUCING | TRIM@74.58 shares=922.9 ratio=0.649 |
| 1158 | 82.39 | POSITION_OPEN | CLOSE_DIFF@82.39 profit=7207.63 |
| 1176 | 77.65 | COST_REDUCING | TRIM@77.65 shares=922.9 ratio=0.649 |
| 1179 | 75.19 | POSITION_OPEN | CLOSE_DIFF@75.19 profit=-2270.27 |
| 1180 | 75.81 | COST_REDUCING | TRIM@75.81 shares=922.9 ratio=0.649 |
| 1184 | 74.88 | POSITION_OPEN | CLOSE_DIFF@74.88 profit=-858.27 |
| 1188 | 75.01 | COST_REDUCING | TRIM@75.01 shares=922.9 ratio=0.649 |
| 1190 | 72.85 | POSITION_OPEN | CLOSE_DIFF@72.85 profit=-1993.40 |
| 1202 | 71.94 | COST_REDUCING | TRIM@71.94 shares=922.9 ratio=0.649 |
| 1213 | 72.54 | POSITION_OPEN | CLOSE_DIFF@72.54 profit=553.72 |
| 1222 | 67.94 | COST_REDUCING | TRIM@67.94 shares=1002.9 ratio=0.705 |
| 1228 | 66.13 | POSITION_OPEN | CLOSE_DIFF@66.13 profit=-1815.20 |
| 1236 | 64.26 | COST_REDUCING | TRIM@64.26 shares=1069.7 ratio=0.752 |
| 1239 | 65.91 | POSITION_OPEN | CLOSE_DIFF@65.91 profit=1764.94 |
| 1243 | 66.93 | COST_REDUCING | TRIM@66.93 shares=1069.7 ratio=0.752 |
| 1247 | 64.32 | POSITION_OPEN | CLOSE_DIFF@64.32 profit=-2791.82 |
| 1248 | 65.72 | COST_REDUCING | TRIM@65.72 shares=1441.0 ratio=0.752 |
| 1250 | 64.32 | POSITION_OPEN | CLOSE_DIFF@64.32 profit=-2017.37 |
| 1251 | 66.55 | COST_REDUCING | TRIM@66.55 shares=1441.0 ratio=0.752 |
| 1266 | 74.34 | POSITION_OPEN | CLOSE_DIFF@74.34 profit=11225.23 |
| 1273 | 70.80 | COST_REDUCING | TRIM@70.80 shares=1441.0 ratio=0.752 |
| 1279 | 70.38 | POSITION_OPEN | CLOSE_DIFF@70.38 profit=-605.21 |
| 1286 | 71.92 | COST_REDUCING | TRIM@71.92 shares=1441.0 ratio=0.752 |
| 1287 | 69.69 | POSITION_OPEN | CLOSE_DIFF@69.69 profit=-3213.38 |
| 1291 | 70.87 | COST_REDUCING | TRIM@70.87 shares=1441.0 ratio=0.752 |
| 1297 | 69.14 | POSITION_OPEN | CLOSE_DIFF@69.14 profit=-2492.89 |
| 1304 | 68.12 | COST_REDUCING | TRIM@68.12 shares=1441.0 ratio=0.752 |
| ... | ... | ... | （共75条，显示前50） |
</details>

## 汇总对比

| 标的 | BH% | H组复���% | 胜率 | 加仓率 | 降成本率 | 本金回收率 |
|------|-----|---------|------|--------|---------|-----------|
| QQQ | +199.9 | -8.89 | 28% | 0/18 | 1/18 | 0/18 |
| SPY | +137.1 | -16.02 | 19% | 0/21 | 1/21 | 0/21 |
| GLD | +155.0 | -13.43 | 54% | 0/13 | 4/13 | 0/13 |
| BRENT | +142.7 | -150.15 | 20% | 2/5 | 1/5 | 0/5 |

## 结果包六要素

**结论**：完整版 5 状态 FSM 在日线级别多标的 2020-2026 上的表现。加仓/降成本/本金回收机制全部实装。

**定义依据**：
- FSM 5状态严格按 cost_reduction_fsm.py 定义实现
- 加仓比例 = alive_rank1_persistence / dominant_persistence（零人工参数）
- 降成本比例 = 同上（结构内蕴）
- 本金回收阈值 = cumulative_recovered ≥ own_capital（267号定义）

**边界条件**：
- 日线级别持仓期短 → 降成本循环难以完成 → 5min 可能更有效
- 2020-2026 全球牛市 → 空方向系统性亏损 → 单向验证不足
- Elder rule 约束 → 真正的 dominant settle 不发生 → 用 rank-1 settle 代替

**下游推论**：
- 若 n_with_cr > 0 且降成本贡献正 → 多层操作有结构性价值
- 若 n_pw > 0 → 本金回收在此时间尺度可达（267号操作方法论有效）
- 若全部 n_pw = 0 → 日线级别降成本速度不足以覆盖本金

**谱系引用**：
- 267号：满仓满融降成本体系（FSM 5状态定义来源）
- 268a号：own_capital 独立核算 + min_operable_level 外部参数
- §7.5：在线因果 merge tree
- Elder rule：dominant 永不被 merge 杀死（rank-1 settle 的动机）

**影响声明**：本回测不修改 cost_reduction_fsm.py 代码，仅在回测层实现其语义的完整版。若验证有效，后续可将加仓逻辑合入 FSM。

**认识论等级**：L2（真实数据，4标的日线；可产生否定性结果）。