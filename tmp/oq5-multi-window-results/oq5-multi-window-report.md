# OQ5-ext 多窗口扩展实验——Au/R 在不同危机窗口的行为验证

生成时间: 2026-03-01T10:37:33.789965
认识论等级: L2（真实数据验证——多窗口，同一标的集）
谱系引用: 287号（综合结论）、284号（OQ v2）、283号、279号、286号

## 1. 实验设计

验证 287号下游推论1：Au/R 背驰状态是否区分'逼近边界'的不同阶段。

每条边在每个窗口内**独立**跑 RecursiveOrchestrator。
不从全历史结果截取事件——确保背驰判断不跨越窗口边界。
六个窗口方法完全一致，可对等比较。

| 窗口 | 日期范围 | 背景 |
|------|---------|------|
| 前危机平静期 2005-2006 | 2005-01-01 ~ 2006-12-31 | |
| 流动性危机 2007-2009 | 2007-01-01 ~ 2009-12-31 | |
| 欧债危机 2010-2012 | 2010-01-01 ~ 2012-12-31 | |
| 中国股灾 2015-2016 | 2015-01-01 ~ 2016-12-31 | |
| 贸易战 2018-2019 | 2018-01-01 ~ 2019-12-31 | |
| 当前周期 2020-2026 | 2020-01-01 ~ 2026-03-01 | |

## 2. Au/R 多窗口对比（核心对比项）

| 窗口 | Au/R 激活 | 走势数 | 背驰(BSP) | 趋势 | 盘整 | 末走势 |
|------|---------|--------|----------|------|------|--------|
| 前危机平静期 2005-2006 | No | 0 | 0 | 0 | 0 |   |
| 流动性危机 2007-2009 | Yes | 1 | 0 | 0 | 1 | consolidation up |
| 欧债危机 2010-2012 | Yes | 1 | 0 | 0 | 1 | consolidation up |
| 中国股灾 2015-2016 | No | 0 | 0 | 0 | 0 |   |
| 贸易战 2018-2019 | No | 0 | 0 | 0 | 0 |   |
| 当前周期 2020-2026 | Yes | 3 | 2 | 0 | 3 | consolidation up |

## 3. 沉寂边位置对比

| 窗口 | 沉寂边 | E/$ | Au/$ | R/$ |
|------|--------|-----|------|-----|
| 前危机平静期 2005-2006 | E/$, Au/$, R/$, E/Au, Au/R, XAU/USD, R/Au_corner, DEXJPUS, DEXSZUS, DEXUSEU | silent | silent | silent |
| 流动性危机 2007-2009 | R/$ | active | active | silent |
| 欧债危机 2010-2012 | E/$, Au/$, E/Au, E/R, XAU/USD, DEXUSUK, DEXSZUS, DEXUSEU | silent | silent | active |
| 中国股灾 2015-2016 | E/$, Au/$, R/$, E/Au, Au/R, E/R, XAU/USD, R/Au_corner, DEXUSUK, DEXSZUS, DEXUSEU | silent | silent | silent |
| 贸易战 2018-2019 | E/$, Au/$, R/$, E/Au, Au/R, E/R, XAU/USD, R/Au_corner, DEXJPUS, DEXUSUK, DEXSZUS, DEXUSEU | silent | silent | silent |
| 当前周期 2020-2026 | E/$ | silent | active | active |

## 4. 全景总表

| 窗口 | 活跃边 | 背驰边 | 趋势边 | 全盘整 |
|------|--------|--------|--------|--------|
| 前危机平静期 2005-2006 | 2/12 | 1 | 0 | Yes |
| 流动性危机 2007-2009 | 11/12 | 7 | 0 | Yes |
| 欧债危机 2010-2012 | 4/12 | 2 | 0 | Yes |
| 中国股灾 2015-2016 | 1/12 | 0 | 0 | Yes |
| 贸易战 2018-2019 | 0/12 | 0 | 0 | Yes |
| 当前周期 2020-2026 | 11/12 | 9 | 0 | Yes |

## 5-1. 前危机平静期 2005-2006 (2005-01-01 ~ 2006-12-31)

- 活跃边: E/R, DEXUSUK
- 背驰边: E/R
- 沉寂边: E/$, Au/$, R/$, E/Au, Au/R, XAU/USD, R/Au_corner, DEXJPUS, DEXSZUS, DEXUSEU

| 边 | bars | 走势 | settled | 趋势 | 盘整 | BSP | 事件 | 末走势 |
|---|---|---|---|---|---|---|---|---|
| E/$ | 503 | 0 | 0 | 0 | 0 | 0 | 2 |   |
| Au/$ | 503 | 0 | 0 | 0 | 0 | 0 | 1 |   |
| R/$ | 503 | 0 | 0 | 0 | 0 | 0 | 4 |   |
| E/Au | 503 | 0 | 0 | 0 | 0 | 0 | 6 |   |
| Au/R | 503 | 0 | 0 | 0 | 0 | 0 | 1 |   |
| E/R | 503 | 1 | 0 | 0 | 1 | 1 | 11 | consolidation up |
| XAU/USD | 498 | 0 | 0 | 0 | 0 | 0 | 2 |   |
| R/Au_corner | 503 | 0 | 0 | 0 | 0 | 0 | 1 |   |
| DEXJPUS | 502 | 0 | 0 | 0 | 0 | 0 | 5 |   |
| DEXUSUK | 502 | 1 | 0 | 0 | 1 | 0 | 9 | consolidation up |
| DEXSZUS | 502 | 0 | 0 | 0 | 0 | 0 | 9 |   |
| DEXUSEU | 502 | 0 | 0 | 0 | 0 | 0 | 8 |   |

## 5-2. 流动性危机 2007-2009 (2007-01-01 ~ 2009-12-31)

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

## 5-3. 欧债危机 2010-2012 (2010-01-01 ~ 2012-12-31)

- 活跃边: R/$, Au/R, R/Au_corner, DEXJPUS
- 背驰边: R/$, DEXJPUS
- 沉寂边: E/$, Au/$, E/Au, E/R, XAU/USD, DEXUSUK, DEXSZUS, DEXUSEU

| 边 | bars | 走势 | settled | 趋势 | 盘整 | BSP | 事件 | 末走势 |
|---|---|---|---|---|---|---|---|---|
| E/$ | 754 | 0 | 0 | 0 | 0 | 0 | 6 |   |
| Au/$ | 754 | 0 | 0 | 0 | 0 | 0 | 3 |   |
| R/$ | 754 | 1 | 0 | 0 | 1 | 1 | 15 | consolidation up |
| E/Au | 754 | 0 | 0 | 0 | 0 | 0 | 12 |   |
| Au/R | 754 | 1 | 0 | 0 | 1 | 0 | 9 | consolidation up |
| E/R | 754 | 0 | 0 | 0 | 0 | 0 | 9 |   |
| XAU/USD | 754 | 0 | 0 | 0 | 0 | 0 | 1 |   |
| R/Au_corner | 754 | 1 | 0 | 0 | 1 | 0 | 9 | consolidation down |
| DEXJPUS | 752 | 1 | 0 | 0 | 1 | 1 | 15 | consolidation down |
| DEXUSUK | 752 | 0 | 0 | 0 | 0 | 0 | 16 |   |
| DEXSZUS | 752 | 0 | 0 | 0 | 0 | 0 | 2 |   |
| DEXUSEU | 752 | 0 | 0 | 0 | 0 | 0 | 9 |   |

## 5-4. 中国股灾 2015-2016 (2015-01-01 ~ 2016-12-31)

- 活跃边: DEXJPUS
- 背驰边: 无
- 沉寂边: E/$, Au/$, R/$, E/Au, Au/R, E/R, XAU/USD, R/Au_corner, DEXUSUK, DEXSZUS, DEXUSEU

| 边 | bars | 走势 | settled | 趋势 | 盘整 | BSP | 事件 | 末走势 |
|---|---|---|---|---|---|---|---|---|
| E/$ | 504 | 0 | 0 | 0 | 0 | 0 | 6 |   |
| Au/$ | 504 | 0 | 0 | 0 | 0 | 0 | 1 |   |
| R/$ | 504 | 0 | 0 | 0 | 0 | 0 | 4 |   |
| E/Au | 504 | 0 | 0 | 0 | 0 | 0 | 5 |   |
| Au/R | 504 | 0 | 0 | 0 | 0 | 0 | 9 |   |
| E/R | 504 | 0 | 0 | 0 | 0 | 0 | 9 |   |
| XAU/USD | 502 | 0 | 0 | 0 | 0 | 0 | 1 |   |
| R/Au_corner | 504 | 0 | 0 | 0 | 0 | 0 | 9 |   |
| DEXJPUS | 502 | 1 | 0 | 0 | 1 | 0 | 12 | consolidation down |
| DEXUSUK | 502 | 0 | 0 | 0 | 0 | 0 | 5 |   |
| DEXSZUS | 502 | 0 | 0 | 0 | 0 | 0 | 8 |   |
| DEXUSEU | 502 | 0 | 0 | 0 | 0 | 0 | 11 |   |

## 5-5. 贸易战 2018-2019 (2018-01-01 ~ 2019-12-31)

- 活跃边: 无
- 背驰边: 无
- 沉寂边: E/$, Au/$, R/$, E/Au, Au/R, E/R, XAU/USD, R/Au_corner, DEXJPUS, DEXUSUK, DEXSZUS, DEXUSEU

| 边 | bars | 走势 | settled | 趋势 | 盘整 | BSP | 事件 | 末走势 |
|---|---|---|---|---|---|---|---|---|
| E/$ | 503 | 0 | 0 | 0 | 0 | 0 | 2 |   |
| Au/$ | 503 | 0 | 0 | 0 | 0 | 0 | 2 |   |
| R/$ | 503 | 0 | 0 | 0 | 0 | 0 | 6 |   |
| E/Au | 503 | 0 | 0 | 0 | 0 | 0 | 2 |   |
| Au/R | 503 | 0 | 0 | 0 | 0 | 0 | 8 |   |
| E/R | 503 | 0 | 0 | 0 | 0 | 0 | 8 |   |
| XAU/USD | 502 | 0 | 0 | 0 | 0 | 0 | 4 |   |
| R/Au_corner | 503 | 0 | 0 | 0 | 0 | 0 | 8 |   |
| DEXJPUS | 498 | 0 | 0 | 0 | 0 | 0 | 12 |   |
| DEXUSUK | 498 | 0 | 0 | 0 | 0 | 0 | 7 |   |
| DEXSZUS | 498 | 0 | 0 | 0 | 0 | 0 | 9 |   |
| DEXUSEU | 498 | 0 | 0 | 0 | 0 | 0 | 9 |   |

## 5-6. 当前周期 2020-2026 (2020-01-01 ~ 2026-03-01)

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

## 6. 结果包

1. **结论**: 见 Au/R 多窗口对比表（第2节）和全景总表（第4节）
2. **定义依据**: 284号 OQ v2——扭曲形式的边界条件阅读；287号综合结论中 Au/R 假说
3. **边界条件**:
   - 每条边在窗口内独立跑 D 算子，窗口外的走势历史不参与判断
   - 窗口长度不完全相等（2年/3年/6年混合）——更长窗口有更多结构产出机会
   - GLD 上市时间 2004-11，前两个窗口的 Au 相关边（Au/$, E/Au, Au/R, XAU/USD, R/Au_corner）数据有效
   - FRED 汇率 O=H=L=C 退化数据的影响不变
4. **下游推论**: Au/R 的激活/背驰模式在六个窗口中的分布——如果只在危机窗口激活且仅在当前窗口有背驰，则 287号结论加固
5. **谱系引用**: 287号、284号、283号、279号、286号
6. **影响声明**: 实验结果，不修改现有代码或定义