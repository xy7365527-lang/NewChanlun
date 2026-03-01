# OQ5 补充实验——当前窗口 vs 2007-2009 对等比较

生成时间: 2026-03-01T10:18:29.450565
认识论等级: L2（真实数据验证）
谱系引用: 284号（OQ v2）、283号、279号、286号

## 1. 实验设计

每条边在每个窗口内**独立**跑 RecursiveOrchestrator。
不是从全历史结果里截取事件——确保背驰判断不跨越窗口边界。
两个窗口方法完全一致，可对等比较。

## 2. 对比总表

| 指标 | 流动性危机 2007-2009 | 当前周期 2020-2026 |
|------|---------------------|-------------------|
| 活跃边数 | 11 / 12 | 11 / 12 |
| 背驰边数 | 7 | 9 |
| 纯资产边活跃 | E/Au, Au/R, E/R | E/Au, Au/R, E/R |
| 现金边活跃 | E/$, Au/$ | Au/$, R/$ |
| 叙事不一致 | True | True |
| 现金角子图有结构 | True | True |
| Au/R 活跃 | True | True |
| Au/R 背驰 | False | True |
| Au/R 趋势/盘整 | 0/1 | 0/3 |
| R/$ 活跃 | False | True |
| R/$ 背驰 | False | True |
| R/$ 趋势/盘整 | 0/0 | 0/2 |

## 3. liquidity_crisis_2007_2009 (2007-01-01 ~ 2009-12-31)

- 活跃边: E/$, Au/$, E/Au, Au/R, E/R, XAU/USD, R/Au_corner, DEXJPUS, DEXUSUK, DEXSZUS, DEXUSEU
- 背驰边: E/$, E/Au, E/R, XAU/USD, DEXJPUS, DEXUSUK, DEXSZUS
- 沉寂边: R/$

| 边 | bars | 走势 | settled | 趋势 | 盘整 | BSP | 事件 | 末走势 |
|---|---|---|---|---|---|---|---|---|
| E/$ | 756 | 1 | 0 | 0 | 1 | 1 | 17 | consolidation down |
| Au/$ | 756 | 1 | 0 | 0 | 1 | 0 | 14 | consolidation up |
| R/$ | 756 | 0 | 0 | 0 | 0 | 0 | 11 |   |
| E/Au | 756 | 2 | 1 | 0 | 2 | 1 | 20 | consolidation down |
| Au/R | 756 | 1 | 0 | 0 | 1 | 0 | 15 | consolidation up |
| E/R | 756 | 1 | 0 | 0 | 1 | 1 | 17 | consolidation down |
| XAU/USD | 757 | 1 | 0 | 0 | 1 | 1 | 10 | consolidation up |
| R/Au_corner | 756 | 1 | 0 | 0 | 1 | 0 | 15 | consolidation down |
| DEXJPUS | 758 | 1 | 0 | 0 | 1 | 1 | 16 | consolidation down |
| DEXUSUK | 758 | 1 | 0 | 0 | 1 | 1 | 20 | consolidation down |
| DEXSZUS | 758 | 2 | 1 | 0 | 2 | 1 | 16 | consolidation up |
| DEXUSEU | 758 | 1 | 0 | 0 | 1 | 0 | 16 | consolidation down |

## 3. current_2020_2026 (2020-01-01 ~ 2026-03-01)

- 活跃边: Au/$, R/$, E/Au, Au/R, E/R, XAU/USD, R/Au_corner, DEXJPUS, DEXUSUK, DEXSZUS, DEXUSEU
- 背驰边: Au/$, R/$, Au/R, E/R, R/Au_corner, DEXJPUS, DEXUSUK, DEXSZUS, DEXUSEU
- 沉寂边: E/$

| 边 | bars | 走势 | settled | 趋势 | 盘整 | BSP | 事件 | 末走势 |
|---|---|---|---|---|---|---|---|---|
| E/$ | 1547 | 0 | 0 | 0 | 0 | 0 | 12 |   |
| Au/$ | 1547 | 1 | 0 | 0 | 1 | 1 | 22 | consolidation up |
| R/$ | 1547 | 2 | 1 | 0 | 2 | 1 | 34 | consolidation down |
| E/Au | 1547 | 2 | 1 | 0 | 2 | 0 | 22 | consolidation down |
| Au/R | 1547 | 3 | 2 | 0 | 3 | 2 | 34 | consolidation up |
| E/R | 1547 | 2 | 1 | 0 | 2 | 2 | 27 | consolidation up |
| XAU/USD | 1549 | 1 | 0 | 0 | 1 | 0 | 17 | consolidation up |
| R/Au_corner | 1547 | 3 | 2 | 0 | 3 | 2 | 34 | consolidation down |
| DEXJPUS | 1533 | 4 | 3 | 0 | 4 | 4 | 47 | consolidation up |
| DEXUSUK | 1533 | 1 | 0 | 0 | 1 | 1 | 20 | consolidation down |
| DEXSZUS | 1533 | 3 | 2 | 0 | 3 | 1 | 41 | consolidation down |
| DEXUSEU | 1533 | 4 | 3 | 0 | 4 | 2 | 55 | consolidation up |

## 4. 结果包

1. **结论**: 见对比总表
2. **定义依据**: 284号 OQ v2——扭曲形式的边界条件阅读
3. **边界条件**:
   - 每条边在窗口内独立跑 D 算子，窗口外的走势历史不参与判断
   - 窗口长度不完全相等（2007-09 三年 vs 2020-26 六年）——更长窗口有更多结构产出机会
   - FRED 汇率 O=H=L=C 退化数据的影响不变（边界条件继承自 OQ5 主实验）
4. **下游推论**: Au/R 在当前窗口的状态——如果和 2007-09 不同——是该窗口的结构签名
5. **谱系引用**: 284号、283号、279号、286号
6. **影响声明**: 实验结果，不修改现有代码或定义