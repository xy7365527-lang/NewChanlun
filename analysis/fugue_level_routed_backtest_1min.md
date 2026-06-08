# 级别路由版缠论+PH赋格回测 — 阶段一（纯做多+降成本）

## 架构

基于完整版（v6），两个核心改动：

### 改动1：级别路由

每个 MoveSettleV1 事件按 persistence 与 P75 阈值比较，路由到不相交路径：

1. **操作级别路径**（persistence ≥ P75）：38课段间比较，可能退出
2. **短差级别路径**（persistence < P75）：降成本循环，不会退出
3. **两条路径不相交**：一个 settle 事件只走一条路径

### 改动2：MACD三维度背驰

S7b-i 盘整背驰判断从价格振幅改为 MACD 三维度 OR：
- T2: MACD面积(U3) < MACD面积(U1)
- T6: DIF峰值(U3) < DIF峰值(U1)
- T7: 柱子高度(U3) < 柱子高度(U1)
- 任一满足 → 盘整背驰（beichi.md #2 已结算）

### 与 v6/v7 的区别

| 版本 | 段间比较触发 | 背驰判据 | 降成本触发 | 两路关系 |
|------|-------------|---------|-----------|---------|
| v6 完整版 | 所有 up-move settle | 价格振幅 | L1 PH nr1 | 独立并行 |
| v7 过滤版 | median 以上 up-move | 价格振幅 | L1 PH nr1 | 段间比较被过滤 |
| v8 路由版 | P75 以上 up-move | **MACD三维度** | P75以下move | **完全不相交** |

### FSM 状态转移

```
WAIT_ENTRY ──(buy BSP + L2多 + L0确认)──→ ENTRY ──→ HOLDING
   ↑                                                  │
   │                          move settle (persistence) │
 OBSERVE ←── WAIT_DIP ←──── EVAL ←─── [≥P75 up] ─────┘
                              │           │
                        不创新高/背驰      └── [<P75] → 降成本
                              ↓
                        EXIT → WAIT_ENTRY
```

### PH三级别信号映射

| PH级别 | 输入 | 更新频率 | 信号用途 |
|--------|------|---------|---------|
| L0 | 1min close | 每bar | 区间套精确入场/回调结束确认 |
| L1 | L1线段端点 | ~数千次 | 加仓 persistence ratio |
| L2 | L1走势端点 | ~数百次 | 方向裁决/紧急清仓 |

## QQQ

- 数据：**728,030** bars (1min)
- 价格：268.82 → 738.28
- Buy-and-hold: **+174.64%**
- 回测耗时：425.8s

### 级别路由统计

| 路径 | 事件数 | 占比 |
|------|--------|------|
| 操作级别（≥P75） | 144 | 37.0% |
| 短差级别（<P75） | 245 | 63.0% |
| **合计** | **389** | 100% |

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
| 交易数 | 30（纯多头） |
| 胜率 | 60.0% |
| 平均收益 | +0.786% |
| 复利累计 | **+25.51%** |
| 最大回撤 | -3.93% |
| 平均持仓 | 6,646 bars |
| 有加仓的交易 | 20/30 |
| 有降成本的交易 | 19/30 |
| 达到本金回收 | 0/30 |
| 有段间比较的交易 | 6/30 |
| 操作级别路由总数 | 23 |
| 短差级别路由总数 | 94 |

### 交易明细

| # | 入场 | 出场 | 持仓bars | PnL% | 加仓 | 短差 | 向上段 | 操作/短差路由 | 退出原因 | 状态路径 |
|---|------|------|---------|------|------|------|--------|--------------|---------|---------|
| 1 | 313.83@45927 | 305.43@49505 | 3,578 | -2.26 | 3 | 1 | 1 | 1/2 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 2 | 314.81@50896 | 317.32@56108 | 5,212 | +0.66 | 1 | 1 | 1 | 1/2 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 3 | 322.52@67708 | 319.88@67882 | 174 | -0.82 | 0 | 0 | 0 | 0/0 | bsp_invalidate | ENTRY→HOLDING |
| 4 | 319.41@67966 | 318.33@72362 | 4,396 | -0.02 | 2 | 1 | 0 | 0/2 | bsp_invalidate | ENTRY→HOLDING |
| 5 | 323.61@72704 | 349.29@86208 | 13,504 | +7.51 | 3 | 3 | 2 | 2/6 | consolidation_divergence_macd | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 6 | 357.54@92089 | 354.25@92162 | 73 | -0.92 | 0 | 0 | 0 | 0/0 | bsp_invalidate | ENTRY→HOLDING |
| 7 | 357.61@93056 | 365.80@97876 | 4,820 | +2.12 | 1 | 0 | 1 | 1/0 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 8 | 377.02@109966 | 376.68@115656 | 5,690 | +0.16 | 1 | 1 | 1 | 1/2 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 9 | 358.17@131229 | 358.07@131250 | 21 | -0.03 | 0 | 0 | 0 | 0/0 | bsp_invalidate | ENTRY→HOLDING |
| 10 | 389.89@186027 | 388.55@192317 | 6,290 | +0.15 | 1 | 2 | 0 | 0/4 | bsp_invalidate | ENTRY→HOLDING |
| 11 | 395.37@196836 | 396.84@208841 | 12,005 | +1.01 | 1 | 2 | 1 | 1/4 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 12 | 413.10@216486 | 419.92@223983 | 7,497 | +1.61 | 1 | 2 | 1 | 1/4 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 13 | 435.14@229309 | 429.72@231925 | 2,616 | -1.19 | 1 | 1 | 0 | 0/2 | bsp_invalidate | ENTRY→HOLDING |
| 14 | 439.04@236273 | 439.10@243197 | 6,924 | +0.05 | 1 | 0 | 2 | 2/0 | consolidation_divergence_macd | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 15 | 448.45@252490 | 442.59@259485 | 6,995 | -1.24 | 1 | 2 | 1 | 1/4 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 16 | 456.16@288173 | 451.10@293623 | 5,450 | -0.51 | 1 | 2 | 0 | 0/4 | bsp_invalidate | ENTRY→HOLDING |
| 17 | 463.86@296109 | 481.63@308780 | 12,671 | +2.91 | 5 | 2 | 2 | 2/4 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 18 | 488.67@310993 | 489.62@318067 | 7,074 | +0.18 | 1 | 0 | 1 | 1/0 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 19 | 487.55@318164 | 484.92@318262 | 98 | -0.54 | 0 | 0 | 0 | 0/0 | bsp_invalidate | ENTRY→HOLDING |
| 20 | 519.92@400612 | 524.05@410720 | 10,108 | +3.22 | 1 | 3 | 1 | 1/6 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 21 | 540.87@440494 | 516.63@445662 | 5,168 | -3.93 | 1 | 1 | 0 | 0/2 | bsp_invalidate | ENTRY→HOLDING |
| 22 | 544.24@520848 | 565.00@543673 | 22,825 | +3.73 | 1 | 10 | 0 | 0/20 | bsp_invalidate | ENTRY→HOLDING |
| 23 | 580.40@549099 | 565.70@554420 | 5,321 | -2.22 | 1 | 1 | 0 | 0/2 | bsp_invalidate | ENTRY→HOLDING |
| 24 | 583.18@566174 | 598.97@586355 | 20,181 | +3.01 | 0 | 7 | 2 | 2/14 | consolidation_divergence_macd | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 25 | 616.76@594461 | 624.25@601887 | 7,426 | +1.61 | 1 | 3 | 1 | 1/6 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 26 | 599.51@603661 | 603.75@603769 | 108 | +0.71 | 0 | 0 | 0 | 0/0 | bsp_invalidate | ENTRY→HOLDING |
| 27 | 650.00@699682 | 677.90@708863 | 9,181 | +4.29 | 0 | 2 | 2 | 2/4 | consolidation_divergence_macd | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 28 | 678.16@708915 | 703.02@717431 | 8,516 | +3.67 | 0 | 0 | 2 | 2/0 | consolidation_divergence_macd | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 29 | 702.00@718194 | 701.88@718219 | 25 | -0.02 | 0 | 0 | 0 | 0/0 | bsp_invalidate | ENTRY→HOLDING |
| 30 | 733.50@722602 | 738.28@728029 | 5,427 | +0.65 | 0 | 0 | 1 | 1/0 | eod_close | ENTRY→EVAL→HOLDING→WAIT_DIP |

### FSM 状态分布（按 bar 数）

| 状态 | Bars | 占比 |
|------|------|------|
| WAIT_ENTRY | 528,626 | 72.6% |
| ENTRY | 30 | 0.0% |
| HOLDING | 192,918 | 26.5% |
| EVAL | 0 | 0.0% |
| WAIT_DIP | 6,456 | 0.9% |
| OBSERVE | 0 | 0.0% |
| EXIT | 0 | 0.0% |

<details><summary>事件流（228条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 45,926 | 313.67 | ENTRY | WAIT→ENTRY BSP={1699618109} |
| 45,927 | 313.83 | HOLDING | ENTRY_LONG@313.83 L2dir=1 |
| 46,384 | 309.40 | WAIT_DIP | →EVAL(op_route) p=13.26≥P75=13.26 high=315.24 area=41.48 → WAIT_DIP(U1完成 high=315.24) |
| 46,714 | 313.12 | HOLDING | DIP_REBUY@313.12 不跌破前低(down_low=308.40 >= prev_low=301.98) |
| 47,393 | 308.80 | HOLDING | TRIM(diff_route) p=8.57<P75=13.26 ratio=0.500 shares=183.2 |
| 48,006 | 311.37 | HOLDING | CLOSE_DIFF(diff_route)@311.37 profit=-470.88 |
| 49,360 | 306.38 | HOLDING | ADD_POS@306.38 +63.2 L1ratio=0.150 |
| 49,505 | 305.43 | WAIT_ENTRY | BSP_INVALIDATE@305.43 |
| 50,895 | 315.00 | ENTRY | WAIT→ENTRY BSP={987157400} |
| 50,896 | 314.81 | HOLDING | ENTRY_LONG@314.81 L2dir=1 |
| 53,510 | 321.52 | WAIT_DIP | →EVAL(op_route) p=14.07≥P75=13.26 high=321.44 area=35.47 → WAIT_DIP(U1完成 high=321.44) |
| 53,630 | 320.80 | HOLDING | DIP_REBUY@320.80 不跌破前低(down_low=320.53 >= prev_low=307.37) |
| 54,689 | 314.50 | HOLDING | TRIM(diff_route) p=4.65<P75=13.26 ratio=0.350 shares=111.3 |
| 55,783 | 318.10 | HOLDING | CLOSE_DIFF(diff_route)@318.10 profit=-400.59 |
| 56,108 | 317.32 | WAIT_ENTRY | BSP_INVALIDATE@317.32 |
| 67,707 | 322.18 | ENTRY | WAIT→ENTRY BSP={1666850169, 103427621} |
| 67,708 | 322.52 | HOLDING | ENTRY_LONG@322.52 L2dir=1 |
| 67,882 | 319.88 | WAIT_ENTRY | BSP_INVALIDATE@319.88 |
| 67,965 | 319.43 | ENTRY | WAIT→ENTRY BSP={414981298} |
| 67,966 | 319.41 | HOLDING | ENTRY_LONG@319.41 L2dir=1 |
| 68,532 | 321.28 | HOLDING | ADD_POS@321.28 +54.2 L1ratio=0.173 |
| 71,368 | 318.03 | HOLDING | TRIM(diff_route) p=10.29<P75=11.87 ratio=0.500 shares=183.6 |
| 72,003 | 316.08 | HOLDING | CLOSE_DIFF(diff_route)@316.08 profit=358.08 |
| 72,362 | 318.33 | WAIT_ENTRY | BSP_INVALIDATE@318.33 |
| 72,703 | 323.59 | ENTRY | WAIT→ENTRY BSP={285522146} |
| 72,704 | 323.61 | HOLDING | ENTRY_LONG@323.61 L2dir=1 |
| 74,503 | 321.04 | HOLDING | TRIM(diff_route) p=6.56<P75=10.80 ratio=0.500 shares=154.5 |
| 74,926 | 324.00 | HOLDING | CLOSE_DIFF(diff_route)@324.00 profit=-457.34 |
| 76,897 | 326.05 | HOLDING | TRIM(diff_route) p=7.11<P75=10.80 ratio=0.500 shares=154.5 |
| 77,031 | 325.95 | HOLDING | CLOSE_DIFF(diff_route)@325.95 profit=15.45 |
| 77,328 | 325.96 | HOLDING | ADD_POS@325.96 +19.3 L1ratio=0.059 |
| 81,552 | 338.07 | WAIT_DIP | →EVAL(op_route) p=15.94≥P75=10.80 high=338.88 area=44.20 → WAIT_DIP(U1完成 high=338.88) |
| 82,822 | 331.37 | HOLDING | DIP_REBUY@331.37 不跌破前低(down_low=331.24 >= prev_low=322.94) |
| 82,871 | 331.49 | HOLDING | TRIM(diff_route) p=3.35<P75=10.80 ratio=0.310 shares=107.5 |
| 82,996 | 330.63 | HOLDING | CLOSE_DIFF(diff_route)@330.63 profit=92.43 |
| 83,632 | 337.79 | HOLDING | ADD_POS@337.79 +38.6 L1ratio=0.111 |
| 86,208 | 349.29 | WAIT_ENTRY | →EVAL(op_route) p=23.78≥P75=11.87 high=354.18 area=32.84 → EXIT(盘整背驰MACD area 32.84<44.20) |
| 92,088 | 357.55 | ENTRY | WAIT→ENTRY BSP={665482103} |
| 92,089 | 357.54 | HOLDING | ENTRY_LONG@357.54 L2dir=1 |
| 92,162 | 354.25 | WAIT_ENTRY | BSP_INVALIDATE@354.25 |
| 93,055 | 357.62 | ENTRY | WAIT→ENTRY BSP={909163236} |
| 93,056 | 357.61 | HOLDING | ENTRY_LONG@357.61 L2dir=1 |
| 97,237 | 364.91 | WAIT_DIP | →EVAL(op_route) p=23.92≥P75=10.80 high=372.82 area=78.80 → WAIT_DIP(U1完成 high=372.82) |
| 97,689 | 366.89 | HOLDING | DIP_REBUY@366.89 不跌破前低(down_low=364.75 >= prev_low=348.90) |
| 97,876 | 365.80 | WAIT_ENTRY | BSP_INVALIDATE@365.80 |
| 109,965 | 377.16 | ENTRY | WAIT→ENTRY BSP={4111992} |
| 109,966 | 377.02 | HOLDING | ENTRY_LONG@377.02 L2dir=1 |
| 111,754 | 381.36 | WAIT_DIP | →EVAL(op_route) p=18.24≥P75=10.29 high=382.86 area=42.63 → WAIT_DIP(U1完成 high=382.86) |
| 111,843 | 382.39 | HOLDING | DIP_REBUY@382.39 不跌破前低(down_low=381.23 >= prev_low=364.62) |
| 114,270 | 378.90 | HOLDING | TRIM(diff_route) p=9.80<P75=10.29 ratio=0.500 shares=132.6 |
| 115,476 | 376.90 | HOLDING | CLOSE_DIFF(diff_route)@376.90 profit=265.24 |
| 115,656 | 376.68 | WAIT_ENTRY | BSP_INVALIDATE@376.68 |
| 131,228 | 357.95 | ENTRY | WAIT→ENTRY BSP={1073924458} |
| 131,229 | 358.17 | HOLDING | ENTRY_LONG@358.17 L2dir=1 |
| 131,250 | 358.07 | WAIT_ENTRY | BSP_INVALIDATE@358.07 |
| 186,026 | 389.75 | ENTRY | WAIT→ENTRY BSP={77258701} |
| 186,027 | 389.89 | HOLDING | ENTRY_LONG@389.89 L2dir=1 |
| 187,547 | 390.47 | HOLDING | TRIM(diff_route) p=7.12<P75=11.57 ratio=0.500 shares=128.2 |
| 187,637 | 390.83 | HOLDING | CLOSE_DIFF(diff_route)@390.83 profit=-46.17 |
| 190,749 | 390.22 | HOLDING | TRIM(diff_route) p=7.64<P75=11.29 ratio=0.500 shares=128.2 |
| 191,243 | 386.20 | HOLDING | CLOSE_DIFF(diff_route)@386.20 profit=515.52 |
| 192,053 | 389.14 | HOLDING | ADD_POS@389.14 +14.4 L1ratio=0.056 |
| 192,317 | 388.55 | WAIT_ENTRY | BSP_INVALIDATE@388.55 |
| 196,835 | 395.32 | ENTRY | WAIT→ENTRY BSP={773048308} |
| 196,836 | 395.37 | HOLDING | ENTRY_LONG@395.37 L2dir=1 |
| 200,569 | 406.38 | WAIT_DIP | →EVAL(op_route) p=19.33≥P75=11.29 high=406.30 area=48.84 → WAIT_DIP(U1完成 high=406.30) |
| 200,579 | 406.48 | HOLDING | DIP_REBUY@406.48 不跌破前低(down_low=406.26 >= prev_low=386.97) |
| 202,741 | 405.90 | HOLDING | TRIM(diff_route) p=10.12<P75=11.29 ratio=0.500 shares=126.5 |
| 206,729 | 409.61 | HOLDING | TRIM(diff_route) p=10.22<P75=11.29 ratio=0.500 shares=126.5 |
| 207,269 | 403.50 | HOLDING | CLOSE_DIFF(diff_route)@403.50 profit=772.69 |
| 208,561 | 398.48 | HOLDING | ADD_POS@398.48 +24.1 L1ratio=0.095 |
| 208,841 | 396.84 | WAIT_ENTRY | BSP_INVALIDATE@396.84 |
| 216,485 | 413.20 | ENTRY | WAIT→ENTRY BSP={61870484} |
| 216,486 | 413.10 | HOLDING | ENTRY_LONG@413.10 L2dir=1 |
| 219,376 | 426.75 | WAIT_DIP | →EVAL(op_route) p=21.80≥P75=11.57 high=424.72 area=43.07 → WAIT_DIP(U1完成 high=424.72) |
| 219,394 | 426.89 | HOLDING | DIP_REBUY@426.89 不跌破前低(down_low=426.66 >= prev_low=402.92) |
| 220,988 | 423.91 | HOLDING | TRIM(diff_route) p=9.28<P75=11.57 ratio=0.500 shares=121.0 |
| 221,363 | 425.79 | HOLDING | CLOSE_DIFF(diff_route)@425.79 profit=-227.55 |
| 223,574 | 420.82 | HOLDING | TRIM(diff_route) p=5.09<P75=11.29 ratio=0.451 shares=109.1 |
| 223,893 | 420.31 | HOLDING | CLOSE_DIFF(diff_route)@420.31 profit=55.66 |
| 223,952 | 419.68 | HOLDING | ADD_POS@419.68 +15.1 L1ratio=0.062 |
| 223,983 | 419.92 | WAIT_ENTRY | BSP_INVALIDATE@419.92 |
| 229,308 | 435.16 | ENTRY | WAIT→ENTRY BSP={645670336} |
| 229,309 | 435.14 | HOLDING | ENTRY_LONG@435.14 L2dir=1 |
| 230,932 | 428.00 | HOLDING | TRIM(diff_route) p=10.04<P75=11.25 ratio=0.500 shares=114.9 |
| 231,018 | 428.01 | HOLDING | CLOSE_DIFF(diff_route)@428.01 profit=-1.15 |
| 231,735 | 431.66 | HOLDING | ADD_POS@431.66 +16.2 L1ratio=0.070 |
| 231,925 | 429.72 | WAIT_ENTRY | BSP_INVALIDATE@429.72 |
| 236,272 | 439.07 | ENTRY | WAIT→ENTRY BSP={604968658} |
| 236,273 | 439.04 | HOLDING | ENTRY_LONG@439.04 L2dir=1 |
| 236,534 | 437.17 | HOLDING | ADD_POS@437.17 +22.0 L1ratio=0.096 |
| 239,110 | 435.88 | WAIT_DIP | →EVAL(op_route) p=18.92≥P75=11.57 high=440.56 area=31.10 → WAIT_DIP(U1完成 high=440.56) |
| 239,791 | 434.09 | HOLDING | DIP_REBUY@434.09 不跌破前低(down_low=433.58 >= prev_low=421.64) |
| 243,197 | 439.10 | WAIT_ENTRY | →EVAL(op_route) p=12.64≥P75=11.87 high=446.75 area=36.59 → EXIT(盘整背驰MACD area 36.59<31.10) |
| 252,489 | 448.45 | ENTRY | WAIT→ENTRY BSP={153832548} |
| 252,490 | 448.45 | HOLDING | ENTRY_LONG@448.45 L2dir=1 |
| 254,242 | 444.86 | WAIT_DIP | →EVAL(op_route) p=14.76≥P75=11.89 high=449.34 area=28.77 → WAIT_DIP(U1完成 high=449.34) |
| 254,428 | 445.39 | HOLDING | DIP_REBUY@445.39 不跌破前低(down_low=444.64 >= prev_low=434.58) |
| 256,018 | 442.69 | HOLDING | TRIM(diff_route) p=4.90<P75=11.87 ratio=0.413 shares=92.1 |
| 256,406 | 444.51 | HOLDING | CLOSE_DIFF(diff_route)@444.51 profit=-167.53 |
| 258,496 | 439.48 | HOLDING | TRIM(diff_route) p=5.26<P75=11.87 ratio=0.443 shares=98.8 |
| 259,393 | 442.82 | HOLDING | CLOSE_DIFF(diff_route)@442.82 profit=-330.04 |
| 259,485 | 442.59 | WAIT_ENTRY | BSP_INVALIDATE@442.59 |
| 288,172 | 456.21 | ENTRY | WAIT→ENTRY BSP={1407799728} |
| 288,173 | 456.16 | HOLDING | ENTRY_LONG@456.16 L2dir=1 |
| 289,596 | 454.11 | HOLDING | TRIM(diff_route) p=11.99<P75=12.15 ratio=0.500 shares=109.6 |
| 289,755 | 455.16 | HOLDING | CLOSE_DIFF(diff_route)@455.16 profit=-115.09 |
| 292,105 | 454.68 | HOLDING | TRIM(diff_route) p=7.53<P75=12.15 ratio=0.500 shares=109.6 |
| 293,018 | 450.15 | HOLDING | CLOSE_DIFF(diff_route)@450.15 profit=496.54 |
| 293,467 | 445.54 | HOLDING | ADD_POS@445.54 +13.7 L1ratio=0.062 |
| 293,623 | 451.10 | WAIT_ENTRY | BSP_INVALIDATE@451.10 |
| 296,108 | 463.92 | ENTRY | WAIT→ENTRY BSP={233153697} |
| 296,109 | 463.86 | HOLDING | ENTRY_LONG@463.86 L2dir=1 |
| 298,271 | 464.37 | WAIT_DIP | →EVAL(op_route) p=15.84≥P75=12.22 high=465.74 area=44.91 → WAIT_DIP(U1完成 high=465.74) |
| 298,338 | 464.22 | HOLDING | DIP_REBUY@464.22 不跌破前低(down_low=464.19 >= prev_low=449.90) |
| 301,518 | 478.61 | WAIT_DIP | →EVAL(op_route) p=17.45≥P75=12.42 high=478.95 area=54.71 → WAIT_DIP(创新高无背驰 high=478.95) |
| 301,544 | 478.81 | HOLDING | DIP_REBUY@478.81 不跌破前低(down_low=478.56 >= prev_low=461.50) |
| 303,525 | 488.05 | HOLDING | TRIM(diff_route) p=12.17<P75=12.22 ratio=0.500 shares=107.8 |
| 304,813 | 480.32 | HOLDING | TRIM(diff_route) p=5.22<P75=12.22 ratio=0.427 shares=92.1 |
| 305,293 | 478.61 | HOLDING | CLOSE_DIFF(diff_route)@478.61 profit=157.47 |
| 306,407 | 478.32 | HOLDING | ADD_POS@478.32 +14.1 L1ratio=0.065 |
| 307,062 | 478.52 | HOLDING | ADD_POS@478.52 +15.0 L1ratio=0.065 |
| 308,206 | 482.01 | HOLDING | ADD_POS@482.01 +16.0 L1ratio=0.065 |
| 308,351 | 483.64 | HOLDING | ADD_POS@483.64 +17.1 L1ratio=0.065 |
| 308,618 | 484.06 | HOLDING | ADD_POS@484.06 +18.2 L1ratio=0.065 |
| 308,780 | 481.63 | WAIT_ENTRY | BSP_INVALIDATE@481.63 |
| 310,992 | 488.76 | ENTRY | WAIT→ENTRY BSP={979779344} |
| 310,993 | 488.67 | HOLDING | ENTRY_LONG@488.67 L2dir=1 |
| 315,965 | 498.86 | WAIT_DIP | →EVAL(op_route) p=28.95≥P75=12.48 high=505.21 area=80.83 → WAIT_DIP(U1完成 high=505.21) |
| 317,997 | 489.64 | HOLDING | DIP_REBUY@489.64 不跌破前低(down_low=489.36 >= prev_low=476.26) |
| 318,067 | 489.62 | WAIT_ENTRY | BSP_INVALIDATE@489.62 |
| 318,163 | 487.67 | ENTRY | WAIT→ENTRY BSP={415698439} |
| 318,164 | 487.55 | HOLDING | ENTRY_LONG@487.55 L2dir=1 |
| 318,262 | 484.92 | WAIT_ENTRY | BSP_INVALIDATE@484.92 |
| 400,611 | 519.99 | ENTRY | WAIT→ENTRY BSP={1882772922} |
| 400,612 | 519.92 | HOLDING | ENTRY_LONG@519.92 L2dir=1 |
| 402,513 | 525.43 | WAIT_DIP | →EVAL(op_route) p=22.11≥P75=13.85 high=524.04 area=53.24 → WAIT_DIP(U1完成 high=524.04) |
| 402,576 | 526.01 | HOLDING | DIP_REBUY@526.01 不跌破前低(down_low=525.07 >= prev_low=501.93) |
| 403,599 | 522.24 | HOLDING | TRIM(diff_route) p=6.87<P75=13.85 ratio=0.496 shares=95.4 |
| 403,998 | 524.07 | HOLDING | CLOSE_DIFF(diff_route)@524.07 profit=-174.59 |
| 405,331 | 528.61 | HOLDING | TRIM(diff_route) p=9.81<P75=13.85 ratio=0.500 shares=96.2 |
| 406,060 | 529.58 | HOLDING | CLOSE_DIFF(diff_route)@529.58 profit=-93.28 |
| 408,469 | 536.16 | HOLDING | TRIM(diff_route) p=13.58<P75=13.85 ratio=0.500 shares=96.2 |
| 410,501 | 510.35 | HOLDING | CLOSE_DIFF(diff_route)@510.35 profit=2482.11 |
| 410,720 | 524.05 | WAIT_ENTRY | BSP_INVALIDATE@524.05 |
| 440,493 | 540.63 | ENTRY | WAIT→ENTRY BSP={900729299} |
| 440,494 | 540.87 | HOLDING | ENTRY_LONG@540.87 L2dir=1 |
| 441,920 | 539.09 | HOLDING | TRIM(diff_route) p=13.92<P75=14.07 ratio=0.500 shares=92.4 |
| 443,146 | 537.77 | HOLDING | CLOSE_DIFF(diff_route)@537.77 profit=122.03 |
| 445,577 | 517.56 | HOLDING | ADD_POS@517.56 +20.9 L1ratio=0.113 |
| ... | ... | ... | （共228条，显示前150） |
</details>

## OKLO

- 数据：**333,613** bars (1min)
- 价格：18.07 → 63.48
- Buy-and-hold: **+251.30%**
- 回测耗时：77.8s

### 级别路由统计

| 路径 | 事件数 | 占比 |
|------|--------|------|
| 操作级别（≥P75） | 78 | 42.2% |
| 短差级别（<P75） | 107 | 57.8% |
| **合计** | **185** | 100% |

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
| 交易数 | 11（纯多头） |
| 胜率 | 54.5% |
| 平均收益 | +7.507% |
| 复利累计 | **+83.70%** |
| 最大回撤 | -37.82% |
| 平均持仓 | 3,696 bars |
| 有加仓的交易 | 7/11 |
| 有降成本的交易 | 1/11 |
| 达到本金回收 | 0/11 |
| 有段间比较的交易 | 4/11 |
| 操作级别路由总数 | 13 |
| 短差级别路由总数 | 4 |

### 交易明细

| # | 入场 | 出场 | 持仓bars | PnL% | 加仓 | 短差 | 向上段 | 操作/短差路由 | 退出原因 | 状态路径 |
|---|------|------|---------|------|------|------|--------|--------------|---------|---------|
| 1 | 13.00@41092 | 20.04@43481 | 2,389 | +54.15 | 0 | 0 | 2 | 2/0 | consolidation_divergence_macd | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 2 | 21.67@43694 | 19.11@46795 | 3,101 | -9.05 | 1 | 0 | 1 | 1/0 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 3 | 24.50@47162 | 20.33@50552 | 3,390 | -13.79 | 1 | 0 | 1 | 1/0 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 4 | 28.23@53131 | 19.57@57372 | 4,241 | -20.69 | 2 | 0 | 1 | 1/0 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 5 | 32.84@82272 | 44.60@88990 | 6,718 | +26.49 | 3 | 0 | 2 | 2/0 | consolidation_divergence_macd | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 6 | 46.01@89134 | 54.69@94200 | 5,066 | +13.23 | 2 | 0 | 1 | 1/0 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 7 | 66.89@151335 | 65.64@151540 | 205 | -1.87 | 0 | 0 | 0 | 0/0 | bsp_invalidate | ENTRY→HOLDING |
| 8 | 76.66@173693 | 76.68@178964 | 5,271 | +0.06 | 4 | 2 | 1 | 1/4 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 9 | 65.04@187215 | 64.39@187431 | 216 | -1.00 | 0 | 0 | 0 | 0/0 | bsp_invalidate | ENTRY→HOLDING |
| 10 | 89.23@199495 | 110.98@206090 | 6,595 | +24.38 | 0 | 0 | 2 | 2/0 | consolidation_divergence_macd | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 11 | 146.69@215838 | 163.65@219307 | 3,469 | +10.68 | 1 | 0 | 2 | 2/0 | consolidation_divergence_macd | ENTRY→EVAL→HOLDING→WAIT_DIP |

### FSM 状态分布（按 bar 数）

| 状态 | Bars | 占比 |
|------|------|------|
| WAIT_ENTRY | 292,941 | 87.8% |
| ENTRY | 11 | 0.0% |
| HOLDING | 37,398 | 11.2% |
| EVAL | 0 | 0.0% |
| WAIT_DIP | 3,263 | 1.0% |
| OBSERVE | 0 | 0.0% |
| EXIT | 0 | 0.0% |

<details><summary>事件流（64条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 41,091 | 13.08 | ENTRY | WAIT→ENTRY BSP={1146069223} |
| 41,092 | 13.00 | HOLDING | ENTRY_LONG@13.00 L2dir=1 |
| 42,786 | 19.43 | WAIT_DIP | →EVAL(op_route) p=8.58≥P75=4.35 high=17.71 area=17.22 → WAIT_DIP(U1完成 high=17.71) |
| 42,906 | 19.52 | HOLDING | DIP_REBUY@19.52 不跌破前低(down_low=19.16 >= prev_low=9.13) |
| 43,481 | 20.04 | WAIT_ENTRY | →EVAL(op_route) p=6.51≥P75=4.50 high=20.64 area=13.54 → EXIT(盘整背驰MACD area 13.54<17.22) |
| 43,693 | 21.68 | ENTRY | WAIT→ENTRY BSP={787574734} |
| 43,694 | 21.67 | HOLDING | ENTRY_LONG@21.67 L2dir=1 |
| 45,323 | 18.34 | WAIT_DIP | →EVAL(op_route) p=5.85≥P75=4.50 high=23.05 area=14.28 → WAIT_DIP(U1完成 high=23.05) |
| 46,744 | 19.14 | HOLDING | DIP_REBUY@19.14 不跌破前低(down_low=17.62 >= prev_low=17.20) |
| 46,795 | 19.11 | WAIT_ENTRY | BSP_INVALIDATE@19.11 |
| 47,161 | 24.60 | ENTRY | WAIT→ENTRY BSP={308273581} |
| 47,162 | 24.50 | HOLDING | ENTRY_LONG@24.50 L2dir=1 |
| 50,300 | 21.43 | WAIT_DIP | →EVAL(op_route) p=10.00≥P75=5.41 high=28.12 area=27.42 → WAIT_DIP(U1完成 high=28.12) |
| 50,307 | 21.52 | HOLDING | DIP_REBUY@21.52 不跌破前低(down_low=21.36 >= prev_low=18.12) |
| 50,507 | 20.36 | HOLDING | ADD_POS@20.36 +963.2 L1ratio=0.236 |
| 50,552 | 20.33 | WAIT_ENTRY | BSP_INVALIDATE@20.33 |
| 53,130 | 28.02 | ENTRY | WAIT→ENTRY BSP={465952852} |
| 53,131 | 28.23 | HOLDING | ENTRY_LONG@28.23 L2dir=1 |
| 53,314 | 26.42 | HOLDING | ADD_POS@26.42 +1242.8 L1ratio=0.351 |
| 54,589 | 23.20 | WAIT_DIP | →EVAL(op_route) p=8.68≥P75=5.85 high=28.68 area=19.13 → WAIT_DIP(U1完成 high=28.68) |
| 54,897 | 21.96 | HOLDING | DIP_REBUY@21.96 不跌破前低(down_low=21.83 >= prev_low=20.00) |
| 57,138 | 18.72 | HOLDING | ADD_POS@18.72 +1678.8 L1ratio=0.351 |
| 57,372 | 19.57 | WAIT_ENTRY | BSP_INVALIDATE@19.57 |
| 82,271 | 31.98 | ENTRY | WAIT→ENTRY BSP={1567619886} |
| 82,272 | 32.84 | HOLDING | ENTRY_LONG@32.84 L2dir=1 |
| 85,820 | 31.29 | WAIT_DIP | →EVAL(op_route) p=19.47≥P75=6.51 high=43.70 area=54.02 → WAIT_DIP(U1完成 high=43.70) |
| 86,342 | 34.20 | HOLDING | DIP_REBUY@34.20 不跌破前低(down_low=30.17 >= prev_low=24.23) |
| 87,232 | 36.11 | HOLDING | ADD_POS@36.11 +1413.7 L1ratio=0.345 |
| 87,675 | 41.59 | HOLDING | ADD_POS@41.59 +1901.6 L1ratio=0.345 |
| 88,990 | 44.60 | WAIT_ENTRY | →EVAL(op_route) p=12.21≥P75=8.34 high=45.21 area=34.96 → EXIT(盘整背驰MACD area 34.96<54.02) |
| 89,133 | 46.11 | ENTRY | WAIT→ENTRY BSP={1157762651} |
| 89,134 | 46.01 | HOLDING | ENTRY_LONG@46.01 L2dir=1 |
| 93,829 | 54.11 | WAIT_DIP | →EVAL(op_route) p=21.28≥P75=8.58 high=59.11 area=97.13 → WAIT_DIP(U1完成 high=59.11) |
| 93,903 | 54.91 | HOLDING | DIP_REBUY@54.91 不跌破前低(down_low=53.89 >= prev_low=37.83) |
| 94,086 | 54.37 | HOLDING | ADD_POS@54.37 +511.1 L1ratio=0.197 |
| 94,200 | 54.69 | WAIT_ENTRY | BSP_INVALIDATE@54.69 |
| 151,334 | 66.80 | ENTRY | WAIT→ENTRY BSP={528094879} |
| 151,335 | 66.89 | HOLDING | ENTRY_LONG@66.89 L2dir=1 |
| 151,540 | 65.64 | WAIT_ENTRY | BSP_INVALIDATE@65.64 |
| 173,692 | 76.58 | ENTRY | WAIT→ENTRY BSP={1926062708} |
| 173,693 | 76.66 | HOLDING | ENTRY_LONG@76.66 L2dir=1 |
| 174,553 | 75.00 | WAIT_DIP | →EVAL(op_route) p=15.42≥P75=10.39 high=77.16 area=33.59 → WAIT_DIP(U1完成 high=77.16) |
| 174,681 | 77.17 | HOLDING | DIP_REBUY@77.17 不跌破前低(down_low=74.81 >= prev_low=61.74) |
| 174,693 | 76.95 | HOLDING | ADD_POS@76.95 +95.8 L1ratio=0.073 |
| 176,091 | 71.38 | HOLDING | TRIM(diff_route) p=7.32<P75=10.39 ratio=0.500 shares=700.1 |
| 176,627 | 75.22 | HOLDING | CLOSE_DIFF(diff_route)@75.22 profit=-2688.51 |
| 176,907 | 77.59 | HOLDING | ADD_POS@77.59 +168.5 L1ratio=0.109 |
| 178,300 | 71.49 | HOLDING | TRIM(diff_route) p=9.23<P75=10.39 ratio=0.500 shares=860.4 |
| 178,793 | 76.59 | HOLDING | CLOSE_DIFF(diff_route)@76.59 profit=-4387.99 |
| 178,964 | 76.68 | WAIT_ENTRY | BSP_INVALIDATE@76.68 |
| 187,214 | 64.98 | ENTRY | WAIT→ENTRY BSP={1974301457} |
| 187,215 | 65.04 | HOLDING | ENTRY_LONG@65.04 L2dir=1 |
| 187,431 | 64.39 | WAIT_ENTRY | BSP_INVALIDATE@64.39 |
| 199,494 | 88.82 | ENTRY | WAIT→ENTRY BSP={545782922} |
| 199,495 | 89.23 | HOLDING | ENTRY_LONG@89.23 L2dir=1 |
| 203,886 | 138.20 | WAIT_DIP | →EVAL(op_route) p=65.27≥P75=10.41 high=138.00 area=140.37 → WAIT_DIP(U1完成 high=138.00) |
| 204,355 | 134.64 | HOLDING | DIP_REBUY@134.64 不跌破前低(down_low=132.89 >= prev_low=72.73) |
| 206,090 | 110.98 | WAIT_ENTRY | →EVAL(op_route) p=21.25≥P75=10.42 high=145.00 area=81.98 → EXIT(盘整背驰MACD area 81.98<140.37) |
| 215,837 | 146.72 | ENTRY | WAIT→ENTRY BSP={244261304} |
| 215,838 | 146.69 | HOLDING | ENTRY_LONG@146.69 L2dir=1 |
| 216,282 | 157.66 | HOLDING | ADD_POS@157.66 +90.8 L1ratio=0.133 |
| 217,570 | 181.14 | WAIT_DIP | →EVAL(op_route) p=44.68≥P75=10.92 high=175.88 area=131.74 → WAIT_DIP(U1完成 high=175.88) |
| 217,784 | 183.50 | HOLDING | DIP_REBUY@183.50 不跌破前低(down_low=172.41 >= prev_low=131.20) |
| 219,307 | 163.65 | WAIT_ENTRY | →EVAL(op_route) p=33.57≥P75=10.92 high=193.82 area=68.59 → EXIT(盘整背驰MACD area 68.59<131.74) |
</details>

## HK700

- 数据：**1,408,882** bars (1min)
- 价格：11.36 → 458.20
- Buy-and-hold: **+3933.45%**
- 回测耗时：630.4s

### 级别路由统计

| 路径 | 事件数 | 占比 |
|------|--------|------|
| 操作级别（≥P75） | 157 | 52.7% |
| 短差级别（<P75） | 141 | 47.3% |
| **合计** | **298** | 100% |

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
| 交易数 | 18（纯多头） |
| 胜率 | 66.7% |
| 平均收益 | +4.274% |
| 复利累计 | **+97.35%** |
| 最大回撤 | -31.11% |
| 平均持仓 | 14,502 bars |
| 有加仓的交易 | 13/18 |
| 有降成本的交易 | 3/18 |
| 达到本金回收 | 0/18 |
| 有段间比较的交易 | 9/18 |
| 操作级别路由总数 | 26 |
| 短差级别路由总数 | 8 |

### 交易明细

| # | 入场 | 出场 | 持仓bars | PnL% | 加仓 | 短差 | 向上段 | 操作/短差路由 | 退出原因 | 状态路径 |
|---|------|------|---------|------|------|------|--------|--------------|---------|---------|
| 1 | 50.50@289429 | 57.10@341796 | 52,367 | +14.39 | 3 | 1 | 2 | 2/2 | consolidation_divergence_macd | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 2 | 64.90@348936 | 82.80@377587 | 28,651 | +20.74 | 2 | 0 | 2 | 2/0 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 3 | 87.50@378892 | 98.20@391198 | 12,306 | +12.23 | 0 | 0 | 2 | 2/0 | consolidation_divergence_macd | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 4 | 115.80@396226 | 90.50@409439 | 13,213 | -18.10 | 1 | 0 | 1 | 1/0 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 5 | 119.10@428315 | 111.25@438789 | 10,474 | -6.27 | 1 | 0 | 1 | 1/0 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 6 | 109.60@441287 | 105.25@443385 | 2,098 | -3.66 | 1 | 0 | 0 | 0/0 | bsp_invalidate | ENTRY→HOLDING |
| 7 | 119.55@453755 | 109.95@457850 | 4,095 | -6.84 | 1 | 0 | 1 | 1/0 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 8 | 124.65@468634 | 136.15@483010 | 14,376 | +9.19 | 2 | 1 | 2 | 2/2 | consolidation_divergence_macd | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 9 | 158.80@576774 | 167.75@624082 | 47,308 | +7.19 | 1 | 2 | 3 | 3/4 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 10 | 222.70@652086 | 251.60@667566 | 15,480 | +12.98 | 0 | 0 | 2 | 2/0 | consolidation_divergence_macd | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 11 | 265.70@670484 | 263.40@670541 | 57 | -0.87 | 0 | 0 | 0 | 0/0 | bsp_invalidate | ENTRY→HOLDING |
| 12 | 276.90@672228 | 299.90@682404 | 10,176 | +7.68 | 1 | 0 | 1 | 1/0 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 13 | 312.90@685236 | 351.40@704079 | 18,843 | +10.74 | 1 | 0 | 3 | 3/0 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 14 | 403.40@710944 | 422.00@717550 | 6,606 | +4.61 | 0 | 0 | 2 | 2/0 | consolidation_divergence_macd | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 15 | 487.60@917775 | 501.20@924223 | 6,448 | +2.53 | 1 | 0 | 1 | 1/0 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 16 | 552.80@944408 | 540.80@954533 | 10,125 | -1.96 | 1 | 0 | 1 | 1/0 | bsp_invalidate | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 17 | 583.20@962508 | 659.60@970887 | 8,379 | +11.68 | 1 | 0 | 2 | 2/0 | consolidation_divergence_macd | ENTRY→EVAL→HOLDING→WAIT_DIP |
| 18 | 629.60@972120 | 633.80@972151 | 31 | +0.67 | 0 | 0 | 0 | 0/0 | bsp_invalidate | ENTRY→HOLDING |

### FSM 状态分布（按 bar 数）

| 状态 | Bars | 占比 |
|------|------|------|
| WAIT_ENTRY | 1,147,831 | 81.5% |
| ENTRY | 18 | 0.0% |
| HOLDING | 222,724 | 15.8% |
| EVAL | 0 | 0.0% |
| WAIT_DIP | 38,309 | 2.7% |
| OBSERVE | 0 | 0.0% |
| EXIT | 0 | 0.0% |

<details><summary>事件流（106条）</summary>

| Bar | 价格 | 状态 | 事件 |
|-----|------|------|------|
| 289,428 | 50.50 | ENTRY | WAIT→ENTRY BSP={2113800284} |
| 289,429 | 50.50 | HOLDING | ENTRY_LONG@50.50 L2dir=1 |
| 307,353 | 49.60 | HOLDING | TRIM(diff_route) p=6.90<P75=7.70 ratio=0.500 shares=990.1 |
| 324,400 | 46.20 | WAIT_DIP | →EVAL(op_route) p=8.60≥P75=7.90 high=52.70 area=121.25 → WAIT_DIP(U1完成 high=52.70) |
| 326,792 | 46.70 | HOLDING | DIP_REBUY@46.70 不跌破前低(down_low=44.90 >= prev_low=44.10) |
| 329,386 | 50.00 | HOLDING | ADD_POS@50.00 +506.7 L1ratio=0.188 |
| 341,796 | 57.10 | WAIT_ENTRY | →EVAL(op_route) p=13.30≥P75=8.00 high=58.10 area=78.49 → EXIT(盘整背驰MACD area 78.49<121.25) |
| 348,935 | 64.90 | ENTRY | WAIT→ENTRY BSP={1648568564} |
| 348,936 | 64.90 | HOLDING | ENTRY_LONG@64.90 L2dir=1 |
| 356,878 | 70.40 | WAIT_DIP | →EVAL(op_route) p=20.40≥P75=8.00 high=70.10 area=104.45 → WAIT_DIP(U1完成 high=70.10) |
| 356,973 | 71.10 | HOLDING | DIP_REBUY@71.10 不跌破前低(down_low=69.80 >= prev_low=49.70) |
| 375,182 | 80.70 | WAIT_DIP | →EVAL(op_route) p=19.20≥P75=8.00 high=83.90 area=120.01 → WAIT_DIP(创新高无背驰 high=83.90) |
| 376,128 | 81.90 | HOLDING | DIP_REBUY@81.90 不跌破前低(down_low=80.20 >= prev_low=64.70) |
| 377,223 | 82.40 | HOLDING | ADD_POS@82.40 +286.4 L1ratio=0.160 |
| 377,587 | 82.80 | WAIT_ENTRY | BSP_INVALIDATE@82.80 |
| 378,891 | 87.50 | ENTRY | WAIT→ENTRY BSP={1299500946} |
| 378,892 | 87.50 | HOLDING | ENTRY_LONG@87.50 L2dir=1 |
| 389,456 | 96.80 | WAIT_DIP | →EVAL(op_route) p=27.30≥P75=8.00 high=98.80 area=147.48 → WAIT_DIP(U1完成 high=98.80) |
| 389,613 | 96.40 | HOLDING | DIP_REBUY@96.40 不跌破前低(down_low=94.00 >= prev_low=71.50) |
| 391,198 | 98.20 | WAIT_ENTRY | →EVAL(op_route) p=11.80≥P75=8.35 high=100.50 area=24.82 → EXIT(盘整背驰MACD area 24.82<147.48) |
| 396,225 | 116.00 | ENTRY | WAIT→ENTRY BSP={1723949316} |
| 396,226 | 115.80 | HOLDING | ENTRY_LONG@115.80 L2dir=1 |
| 400,708 | 108.40 | WAIT_DIP | →EVAL(op_route) p=26.80≥P75=8.35 high=119.00 area=118.39 → WAIT_DIP(U1完成 high=119.00) |
| 409,083 | 90.95 | HOLDING | DIP_REBUY_DIVERGE@90.95 跌破+盘整背驰 |
| 409,422 | 91.65 | HOLDING | ADD_POS@91.65 +189.3 L1ratio=0.219 |
| 409,439 | 90.50 | WAIT_ENTRY | BSP_INVALIDATE@90.50 |
| 428,314 | 119.10 | ENTRY | WAIT→ENTRY BSP={1685477336} |
| 428,315 | 119.10 | HOLDING | ENTRY_LONG@119.10 L2dir=1 |
| 436,751 | 117.35 | WAIT_DIP | →EVAL(op_route) p=11.70≥P75=10.20 high=123.50 area=59.72 → WAIT_DIP(U1完成 high=123.50) |
| 438,678 | 112.75 | HOLDING | DIP_REBUY@112.75 不跌破前低(down_low=112.00 >= prev_low=111.80) |
| 438,789 | 111.25 | WAIT_ENTRY | BSP_INVALIDATE@111.25 |
| 441,286 | 109.40 | ENTRY | WAIT→ENTRY BSP={1444010681} |
| 441,287 | 109.60 | HOLDING | ENTRY_LONG@109.60 L2dir=1 |
| 443,150 | 106.00 | HOLDING | ADD_POS@106.00 +94.9 L1ratio=0.104 |
| 443,385 | 105.25 | WAIT_ENTRY | BSP_INVALIDATE@105.25 |
| 453,754 | 119.30 | ENTRY | WAIT→ENTRY BSP={1867672144} |
| 453,755 | 119.55 | HOLDING | ENTRY_LONG@119.55 L2dir=1 |
| 457,173 | 109.70 | WAIT_DIP | →EVAL(op_route) p=16.80≥P75=11.80 high=124.35 area=65.41 → WAIT_DIP(U1完成 high=124.35) |
| 457,634 | 109.15 | HOLDING | DIP_REBUY@109.15 不跌破前低(down_low=108.40 >= prev_low=107.55) |
| 457,850 | 109.95 | WAIT_ENTRY | BSP_INVALIDATE@109.95 |
| 468,633 | 124.45 | ENTRY | WAIT→ENTRY BSP={439129090} |
| 468,634 | 124.65 | HOLDING | ENTRY_LONG@124.65 L2dir=1 |
| 474,856 | 124.05 | WAIT_DIP | →EVAL(op_route) p=15.10≥P75=12.35 high=127.20 area=69.32 → WAIT_DIP(U1完成 high=127.20) |
| 475,334 | 124.70 | HOLDING | DIP_REBUY@124.70 不跌破前低(down_low=123.25 >= prev_low=112.10) |
| 479,825 | 125.25 | HOLDING | TRIM(diff_route) p=9.20<P75=12.35 ratio=0.500 shares=423.9 |
| 480,282 | 125.35 | HOLDING | CLOSE_DIFF(diff_route)@125.35 profit=-42.39 |
| 483,010 | 136.15 | WAIT_ENTRY | →EVAL(op_route) p=17.15≥P75=12.35 high=137.15 area=63.22 → EXIT(盘整背驰MACD area 63.22<69.32) |
| 576,773 | 158.80 | ENTRY | WAIT→ENTRY BSP={1264064818} |
| 576,774 | 158.80 | HOLDING | ENTRY_LONG@158.80 L2dir=1 |
| 582,192 | 162.80 | WAIT_DIP | →EVAL(op_route) p=17.85≥P75=15.50 high=164.25 area=49.31 → WAIT_DIP(U1完成 high=164.25) |
| 582,681 | 157.55 | HOLDING | DIP_REBUY@157.55 不跌破前低(down_low=156.70 >= prev_low=146.40) |
| 584,519 | 164.90 | HOLDING | TRIM(diff_route) p=9.80<P75=15.50 ratio=0.500 shares=314.9 |
| 584,618 | 163.90 | HOLDING | CLOSE_DIFF(diff_route)@163.90 profit=314.86 |
| 591,175 | 171.10 | WAIT_DIP | →EVAL(op_route) p=20.65≥P75=16.75 high=174.85 area=81.66 → WAIT_DIP(创新高无背驰 high=174.85) |
| 592,669 | 173.40 | HOLDING | DIP_REBUY@173.40 不跌破前低(down_low=167.30 >= prev_low=154.20) |
| 604,357 | 198.90 | WAIT_DIP | →EVAL(op_route) p=32.30≥P75=16.75 high=199.80 area=127.78 → WAIT_DIP(创新高无背驰 high=199.80) |
| 619,998 | 176.05 | HOLDING | DIP_REBUY@176.05 不跌破前低(down_low=174.75 >= prev_low=167.50) |
| 620,707 | 175.95 | HOLDING | TRIM(diff_route) p=16.05<P75=16.75 ratio=0.500 shares=314.9 |
| 623,740 | 168.30 | HOLDING | CLOSE_DIFF(diff_route)@168.30 profit=2408.69 |
| 624,082 | 167.75 | WAIT_ENTRY | BSP_INVALIDATE@167.75 |
| 652,085 | 222.90 | ENTRY | WAIT→ENTRY BSP={1407391510} |
| 652,086 | 222.70 | HOLDING | ENTRY_LONG@222.70 L2dir=1 |
| 664,568 | 258.70 | WAIT_DIP | →EVAL(op_route) p=95.55≥P75=16.75 high=261.20 area=571.74 → WAIT_DIP(U1完成 high=261.20) |
| 664,706 | 261.40 | HOLDING | DIP_REBUY@261.40 不跌破前低(down_low=255.90 >= prev_low=165.65) |
| 667,566 | 251.60 | WAIT_ENTRY | →EVAL(op_route) p=17.50≥P75=16.80 high=265.80 area=78.29 → EXIT(盘整背驰MACD area 78.29<571.74) |
| 670,483 | 266.00 | ENTRY | WAIT→ENTRY BSP={1941873731, 300395621} |
| 670,484 | 265.70 | HOLDING | ENTRY_LONG@265.70 L2dir=1 |
| 670,541 | 263.40 | WAIT_ENTRY | BSP_INVALIDATE@263.40 |
| 672,227 | 277.10 | ENTRY | WAIT→ENTRY BSP={27868888} |
| 672,228 | 276.90 | HOLDING | ENTRY_LONG@276.90 L2dir=1 |
| 681,005 | 295.00 | WAIT_DIP | →EVAL(op_route) p=66.30≥P75=17.15 high=314.30 area=240.64 → WAIT_DIP(U1完成 high=314.30) |
| 681,887 | 304.40 | HOLDING | DIP_REBUY@304.40 不跌破前低(down_low=294.80 >= prev_low=248.00) |
| 682,404 | 299.90 | WAIT_ENTRY | BSP_INVALIDATE@299.90 |
| 685,235 | 312.90 | ENTRY | WAIT→ENTRY BSP={289921900} |
| 685,236 | 312.90 | HOLDING | ENTRY_LONG@312.90 L2dir=1 |
| 688,021 | 313.40 | WAIT_DIP | →EVAL(op_route) p=23.10≥P75=17.50 high=322.30 area=67.17 → WAIT_DIP(U1完成 high=322.30) |
| 689,181 | 323.40 | HOLDING | DIP_REBUY@323.40 不跌破前低(down_low=307.00 >= prev_low=299.20) |
| 696,860 | 345.10 | WAIT_DIP | →EVAL(op_route) p=25.60≥P75=17.50 high=328.50 area=111.68 → WAIT_DIP(创新高无背驰 high=328.50) |
| 696,868 | 346.40 | HOLDING | DIP_REBUY@346.40 不跌破前低(down_low=345.50 >= prev_low=302.90) |
| 702,851 | 371.70 | WAIT_DIP | →EVAL(op_route) p=86.80≥P75=17.50 high=405.20 area=315.32 → WAIT_DIP(创新高无背驰 high=405.20) |
| 703,004 | 369.10 | HOLDING | DIP_REBUY@369.10 不跌破前低(down_low=367.80 >= prev_low=318.40) |
| 703,893 | 352.10 | HOLDING | ADD_POS@352.10 +45.5 L1ratio=0.142 |
| 704,079 | 351.40 | WAIT_ENTRY | BSP_INVALIDATE@351.40 |
| 710,943 | 404.70 | ENTRY | WAIT→ENTRY BSP={1672154718} |
| 710,944 | 403.40 | HOLDING | ENTRY_LONG@403.40 L2dir=1 |
| 712,822 | 409.60 | WAIT_DIP | →EVAL(op_route) p=52.90≥P75=18.35 high=411.50 area=158.26 → WAIT_DIP(U1完成 high=411.50) |
| 712,979 | 402.10 | HOLDING | DIP_REBUY@402.10 不跌破前低(down_low=399.30 >= prev_low=358.60) |
| 717,550 | 422.00 | WAIT_ENTRY | →EVAL(op_route) p=43.50≥P75=19.20 high=439.30 area=157.90 → EXIT(盘整背驰MACD area 157.90<158.26) |
| 917,774 | 487.60 | ENTRY | WAIT→ENTRY BSP={2147221760} |
| 917,775 | 487.60 | HOLDING | ENTRY_LONG@487.60 L2dir=1 |
| 922,555 | 501.20 | WAIT_DIP | →EVAL(op_route) p=76.60≥P75=28.60 high=520.00 area=243.35 → WAIT_DIP(U1完成 high=520.00) |
| 924,073 | 504.20 | HOLDING | DIP_REBUY@504.20 不跌破前低(down_low=475.40 >= prev_low=443.40) |
| 924,223 | 501.20 | WAIT_ENTRY | BSP_INVALIDATE@501.20 |
| 944,407 | 552.20 | ENTRY | WAIT→ENTRY BSP={1342882846} |
| 944,408 | 552.80 | HOLDING | ENTRY_LONG@552.80 L2dir=1 |
| 952,547 | 533.40 | WAIT_DIP | →EVAL(op_route) p=102.40≥P75=32.35 high=583.60 area=496.50 → WAIT_DIP(U1完成 high=583.60) |
| 954,142 | 541.20 | HOLDING | DIP_REBUY@541.20 不跌破前低(down_low=525.20 >= prev_low=481.20) |
| 954,533 | 540.80 | WAIT_ENTRY | BSP_INVALIDATE@540.80 |
| 962,507 | 583.60 | ENTRY | WAIT→ENTRY BSP={751750714} |
| 962,508 | 583.20 | HOLDING | ENTRY_LONG@583.20 L2dir=1 |
| 966,904 | 674.40 | WAIT_DIP | →EVAL(op_route) p=219.00≥P75=33.80 high=707.60 area=743.70 → WAIT_DIP(U1完成 high=707.60) |
| 967,147 | 677.60 | HOLDING | DIP_REBUY@677.60 不跌破前低(down_low=669.80 >= prev_low=488.60) |
| 970,887 | 659.60 | WAIT_ENTRY | →EVAL(op_route) p=86.80≥P75=34.60 high=715.00 area=406.53 → EXIT(盘整背驰MACD area 406.53<743.70) |
| 972,119 | 633.80 | ENTRY | WAIT→ENTRY BSP={176270262} |
| 972,120 | 629.60 | HOLDING | ENTRY_LONG@629.60 L2dir=1 |
| 972,151 | 633.80 | WAIT_ENTRY | BSP_INVALIDATE@633.80 |
</details>

## 八版横向对比

| 版本 | QQQ | OKLO | HK700 |
|------|-----|------|-------|
| 1. 单层PH H组 | -19.8% | -99% | -84% |
| 2. 多级别PH（双向） | +58.3% | +72.3% | +222.8% |
| 3. Regime过滤器 | +58.3% | +23.7% | +222.8% |
| 4. 走势类型过滤 | +18% | -86% | +6% |
| 5. 纯做多 | +71.7% | +228% | +292.9% |
| 6. 完整版（无过滤） | +23.1% | +133.2% | +224.1% |
| 7. persistence过滤版 | — | — | — |
| 8. **级别路由版（本版）** | **+25.5%** | **+83.7%** | **+97.3%** |

## 汇总

| 标的 | Bars | BH% | 复利% | 胜率 | 交易数 | L2翻转 | 加仓 | 降成本 | PW | 段比较 | 操作路由 | 短差路由 | 耗时 |
|------|------|-----|------|------|--------|--------|------|--------|-----|--------|---------|---------|------|
| QQQ | 728,030 | +174.6 | +25.51 | 60% | 30 | 14 | 20/30 | 19/30 | 0/30 | 6/30 | 144 | 245 | 426s |
| OKLO | 333,613 | +251.3 | +83.70 | 55% | 11 | 7 | 7/11 | 1/11 | 0/11 | 4/11 | 78 | 107 | 78s |
| HK700 | 1,408,882 | +3933.5 | +97.35 | 67% | 18 | 5 | 13/18 | 3/18 | 0/18 | 9/18 | 157 | 141 | 630s |

## 结果包六要素

**结论**：级别路由版+MACD背驰在1分钟级别3标的上的表现。两个核心改动：(1) persistence P75 级别路由，(2) MACD三维度背驰替代价格振幅。

**定义依据**：
- 38课第36行：同级别分解操作程式——段间比较是操作级别事件
- 38课第30行：分区卷钱降成本——降成本是短差级别事件
- 第24/25课：MACD辅助力度判断（面积+DIF峰值+柱子高度）
- beichi.md #2（已结算）：三维度 OR 判定
- Move.persistence（ph_layer.py）：走势的结构性影响度

**边界条件**：
- PENDING_EXPIRY = 390 bars
- P75 阈值在前 3 个 settle 事件期间为 0（全部路由到操作级别）
- 盘整背驰：MACD三维度OR（面积/DIF/柱子高度任一衰减即触发）
- 不创新高（S7a）仍为价格比较（定义本身是价格级别的）
- 降成本 trim ratio = persistence / P75，裁剪到 [0.05, 0.5]
- 阶段一：纯做多，向下段空仓等待
- 无滑点/手续费建模

**下游推论**：
- MACD背驰 vs 价格振幅背驰的退出时机差异
- 若路由版 > 完整版 → 分离路径+MACD共同改善了退出
- 若路由版 < 完整版 → 需区分路由效应和MACD效应

**谱系引用**：
- 525号：组件局部完成性
- 526号：a0递归存在论区分
- 521号：PH纯拓扑无动量

**影响声明**：新建独立回测脚本，不修改引擎代码或旧版回测。

**认识论等级**：L2（真实数据，3标的 1min；可产生否定性结果）。