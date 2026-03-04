# FRED WTI + Yahoo K4 eff.dim 同步压缩 Cluster 分析报告

运行时间: 2026-03-04T12:11:03.673005
认识论等级: **L2**（真实数据，可否证）

## 数据源说明

FRED GOLDAMGBD228NLBM（LBMA London Fix gold AM price）于 2022-01-31 被 FRED 删除，
原因是 ICE Benchmark Administration (IBA) 全部数据从 FRED 移除。
因此本分析无法使用 1986-2000 的黄金现货日线数据。

替代方案：
- Au: Yahoo Finance GC=F（gold futures），数据从 2000-08-30 起
- WTI: FRED DCOILWTICO（WTI spot），数据从 1986-01-02 起，通过 Playwright 下载
- SPX: Yahoo Finance ^GSPC，数据从 1986+ 起
- DXY: Yahoo Finance DX-Y.NYB，数据从 1986+ 起

公共窗口受 GC=F 限制，从 ~2000-09 起。1986-2000 窗口无法进行四节点分析。

## 1. 结论

- 数据窗口: 2000-08-30 to 2026-02-23
- 公共观测数: 6373 交易日
- eff.dim 可计算观测数: ~6121
- 同步压缩日总数: 263
- 同步压缩 cluster 总数: **1**

- Mahalanobis 全样本统计: mean=1.39, p95=3.08, p99=5.34

### Cluster 列表

| # | 起始 | 结束 | 日历天 | 同步天 | Au min | $ min | Equity min | Commodity min | Maha mean | Maha max | 历史背景 |
|---|------|------|--------|--------|--------|-------|------------|---------------|-----------|----------|----------|
| 1 | 2020-04-03 | 2021-04-23 | 386 | 263 | 1.5079 | 1.5082 | 1.5406 | 1.1284 | 1.69 | 15.49 | COVID crash + massive QE/fiscal; V-shaped recovery |

### 全样本 5th Percentile 阈值

- Au: 1.709588
- $: 1.781095
- Equity: 1.835656
- Commodity: 1.261593

### eff.dim 全样本统计

| 节点 | count | mean | std | min | p5 | median | p95 | max |
|------|-------|------|-----|-----|----|--------|-----|-----|
| Au | 6118 | 2.1627 | 0.2331 | 1.5079 | 1.7096 | 2.1949 | 2.4800 | 2.5902 |
| $ | 6118 | 2.1200 | 0.2302 | 1.5082 | 1.7811 | 2.1283 | 2.5606 | 2.7264 |
| Equity | 6118 | 2.2002 | 0.2533 | 1.5406 | 1.8357 | 2.1745 | 2.6493 | 2.8260 |
| Commodity | 6118 | 1.5249 | 0.2305 | 1.1284 | 1.2616 | 1.4793 | 2.0161 | 2.2254 |

## 2. 定义依据

- **eff.dim 计算**: 完全复制 `scripts/effdim_monitor.py` 逻辑。每节点3条边 = log(price_node / price_peer) 的 diff。Rolling 252d 协方差矩阵 -> 特征值分解 -> Shannon entropy -> eff.dim = exp(entropy)。
- **同步压缩**: 329号结算定义——四节点 eff.dim 同时 < 各自全样本 5th percentile。
- **Cluster 聚合**: 同步压缩日间隔 <= 5 个日历天归为同一 cluster。
- **Mahalanobis 距离**: 3条线性独立边（Au/$, Au/Equity, Au/Commodity）的联合 return 相对全样本均值和协方差的 Mahalanobis 距离。K4 的 6 条边仅有 N-1=3 条独立，6x6 协方差矩阵 rank=3，直接 inv 会失败。选择一个节点的 3 条边作为独立基。
- **数据源差异 vs effdim_monitor.py**: WTI 用 FRED 现货替代 CL=F 期货（无展期价差），Au 仍用 GC=F 期货（LBMA 数据不可用）。

## 3. 边界条件

以下条件改变会导致 cluster 识别结果变化：

1. **阈值选择**: 5th percentile 是全样本统计量。改为滚动 percentile 或不同百分位（如 10th）会改变 cluster 数量和位置。
2. **窗口长度**: 252d rolling window 假设年度时间尺度。缩短（126d）增加噪声，延长（504d）平滑短期事件。
3. **数据源**: GC=F 期货价格包含展期价差和期限结构效应，与现货价格微观结构不同。如果 LBMA 数据可通过其他渠道获取（如 Bloomberg），结果可能不同。
4. **Gap 天数**: 间隔 <= 5 天是任意选择。改为 0 或 10 会改变 cluster 边界。
5. **窗口限制**: 因 LBMA 数据删除，1986-2000 窗口无法覆盖。如果该窗口存在同步压缩 cluster，本分析将遗漏。

## 4. 下游推论

- 本分析的公共窗口（~2000-2025）与 effdim_monitor.py 相同，因为后者也使用 GC=F 和 CL=F。
- WTI 从 CL=F 改为 FRED 现货是唯一的数据源差异。如果 cluster 结果与 effdim_monitor.py 一致，说明 WTI 现货 vs 期货的差异不影响同步压缩识别。
- 如果 cluster 结果显著不同，说明期货展期价差对 eff.dim 有实质影响（需进一步调查）。
- 1986-2000 窗口的分析需要替代黄金数据源（如 Bloomberg LBMA、Refinitiv）。
- Mahalanobis 交叉验证：如果 cluster 的 Maha > p95，说明同步压缩与联合 return 极端性一致。

## 5. 谱系引用

- 329号：K4 折叠内在性检验最终结算（同步压缩定义 + 阈值因果性 + QE 制度依赖）
- 330号：K4 拓扑与资本循环映射（可监控信号定义）
- 231号：形式化有效域规则（L2 标注要求）
- FRED 公告：https://news.research.stlouisfed.org/2022/01/ice-benchmark-administration-ltd-iba-data-to-be-removed-from-fred/

## 6. 影响声明

- 本分析不修改任何代码或定义。
- 产出三个新文件写入 `tmp/fred-k4-cluster/`（分析脚本 + JSON 结果 + 本报告）。
- FRED LBMA gold 数据不可用是一个外部约束（数据源删除），不是分析缺陷。
- 1986-2000 四节点分析需要替代黄金数据源，本分析无法覆盖。
- WTI 现货 vs 期货是本分析对 effdim_monitor.py 的唯一数据源改进。
