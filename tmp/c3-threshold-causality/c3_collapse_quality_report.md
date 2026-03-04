# C3 线二: 坍缩质量分析 — 安静压缩 vs 震荡压缩

## 认识论等级: L2 (真实数据验证)

## 核心问题

向心坍缩事件 (Mahalanobis 极端 + 四节点同步 eff.dim 压缩) 中, 压缩是"安静的"还是"震荡的"?

- 安静压缩: 边波动率与坍缩烈度成比例 (vol z-score 温和)
- 震荡压缩: 边波动率异常极端 (vol z-score 远超坍缩深度的合理比例)

## 坍缩事件定义 (双重条件)

- Mahalanobis > 99th percentile (K4 极端冲击天)
- 四节点同时 eff.dim < 5th percentile (同步压缩)
- 交集: **8 天** (极端天 64 天中的子集)

## 数据: 2000-08-30 to 2026-03-03, 6392 obs

### Cluster 1: 2020-04-22 to 2020-05-05 (6 days)

同步天: 2020-04-22, 2020-04-23, 2020-04-27, 2020-04-29, 2020-04-30, 2020-05-05

#### eff.dim 压缩深度

| Node | Cluster Mean | Full Mean | z-score |
|------|:--:|:--:|:--:|
| Au | 1.5939 | 2.1623 | -2.4143 |
| Dollar | 1.5527 | 2.1323 | -2.4313 |
| Equity | 1.7585 | 2.2129 | -1.8449 |
| Commodity | 1.1668 | 1.5358 | -1.6070 |

#### 边波动率 z-score

| Node | Edge | Vol | z-score |
|------|------|:--:|:--:|
| Au | Au/DXY | 0.012837 | +0.0702 |
| Au | Au/SPX | 0.014592 | +0.0456 |
| Au | Au/OIL | 0.205639 | +11.1909 |
| Dollar | DXY/Au | 0.012837 | +0.0702 |
| Dollar | DXY/SPX | 0.013142 | +0.2491 |
| Dollar | DXY/OIL | 0.210824 | +11.2219 |
| Equity | SPX/DXY | 0.013142 | +0.2491 |
| Equity | SPX/Au | 0.014592 | +0.0456 |
| Equity | SPX/OIL | 0.213042 | +11.6406 |
| Commodity | OIL/DXY | 0.210824 | +11.2219 |
| Commodity | OIL/Au | 0.205639 | +11.1909 |
| Commodity | OIL/SPX | 0.213042 | +11.6406 |

#### Mahalanobis

- 值: [11.6224, 6.9653, 11.2028, 7.6207, 9.1133, 7.1047]
- 均值: 8.9382, 最大: 11.6224, 最小: 6.9653

### Cluster 2: 2020-06-11 to 2020-06-11 (1 days)

同步天: 2020-06-11

#### eff.dim 压缩深度

| Node | Cluster Mean | Full Mean | z-score |
|------|:--:|:--:|:--:|
| Au | 1.5341 | 2.1623 | -2.6681 |
| Dollar | 1.4934 | 2.1323 | -2.6799 |
| Equity | 1.6836 | 2.2129 | -2.1493 |
| Commodity | 1.1458 | 1.5358 | -1.6986 |

#### 边绝对收益率 z-score

| Node | Edge | |Ret| | z-score |
|------|------|:--:|:--:|
| Au | Au/DXY | 0.002863 | -0.7681 |
| Au | Au/SPX | 0.071608 | +5.0919 |
| Au | Au/OIL | 0.096765 | +3.8754 |
| Dollar | DXY/Au | 0.002863 | -0.7681 |
| Dollar | DXY/SPX | 0.068745 | +5.9918 |
| Dollar | DXY/OIL | 0.093902 | +3.6205 |
| Equity | SPX/DXY | 0.068745 | +5.9918 |
| Equity | SPX/Au | 0.071608 | +5.0919 |
| Equity | SPX/OIL | 0.025157 | +0.3607 |
| Commodity | OIL/DXY | 0.093902 | +3.6205 |
| Commodity | OIL/Au | 0.096765 | +3.8754 |
| Commodity | OIL/SPX | 0.025157 | +0.3607 |

#### Mahalanobis

- 值: [5.8134]
- 均值: 5.8134, 最大: 5.8134, 最小: 5.8134

### Cluster 3: 2020-11-09 to 2020-11-09 (1 days)

同步天: 2020-11-09

#### eff.dim 压缩深度

| Node | Cluster Mean | Full Mean | z-score |
|------|:--:|:--:|:--:|
| Au | 1.5667 | 2.1623 | -2.5297 |
| Dollar | 1.5368 | 2.1323 | -2.4979 |
| Equity | 1.7366 | 2.2129 | -1.9338 |
| Commodity | 1.1588 | 1.5358 | -1.6422 |

#### 边绝对收益率 z-score

| Node | Edge | |Ret| | z-score |
|------|------|:--:|:--:|
| Au | Au/DXY | 0.056476 | +4.8960 |
| Au | Au/SPX | 0.062701 | +4.3368 |
| Au | Au/OIL | 0.132478 | +5.6343 |
| Dollar | DXY/Au | 0.056476 | +4.8960 |
| Dollar | DXY/SPX | 0.006225 | -0.3004 |
| Dollar | DXY/OIL | 0.076002 | +2.7569 |
| Equity | SPX/DXY | 0.006225 | -0.3004 |
| Equity | SPX/Au | 0.062701 | +4.3368 |
| Equity | SPX/OIL | 0.069777 | +2.5796 |
| Commodity | OIL/DXY | 0.076002 | +2.7569 |
| Commodity | OIL/Au | 0.132478 | +5.6343 |
| Commodity | OIL/SPX | 0.069777 | +2.5796 |

#### Mahalanobis

- 值: [5.8825]
- 均值: 5.8825, 最大: 5.8825, 最小: 5.8825

## Sync vs Non-sync 极端天波动率对比

同步压缩天: 8 天, 非同步极端天: 56 天

| Edge | Sync Vol | NonSync Vol | Ratio (Sync/NonSync) |
|------|:--:|:--:|:--:|
| Au/DXY | 0.023589 | 0.047453 | 0.4971 |
| Au/SPX | 0.038464 | 0.071219 | 0.5401 |
| Au/OIL | 0.192281 | 0.123806 | 1.5531 |
| DXY/Au | 0.023589 | 0.047453 | 0.4971 |
| DXY/SPX | 0.030360 | 0.065192 | 0.4657 |
| DXY/OIL | 0.196245 | 0.129876 | 1.5110 |
| SPX/DXY | 0.030360 | 0.065192 | 0.4657 |
| SPX/Au | 0.038464 | 0.071219 | 0.5401 |
| SPX/OIL | 0.188229 | 0.123508 | 1.5240 |
| OIL/DXY | 0.196245 | 0.129876 | 1.5110 |
| OIL/Au | 0.192281 | 0.123806 | 1.5531 |
| OIL/SPX | 0.188229 | 0.123508 | 1.5240 |

## eff.dim 深度跨 cluster 变异性

Verdict: **UNIFORM**

## Dollar 特异性

Verdict: **NO**

| Cluster | Dollar mean z | Others mean z | Diff |
|---------|:--:|:--:|:--:|
| 1 | 3.8471 | 6.3662 | -2.5191 |
| 2 | 2.9481 | 3.0556 | -0.1075 |
| 3 | 2.4508 | 3.6060 | -1.1552 |

## 综合判定

**DIFFERENTIATED**

Edge vol z-scores extreme (max |z|=11.64, mean |z|=4.15). OIL edges: mean |z|=5.88, non-OIL edges: mean |z|=2.42.

## 判决标准参照

- 所有 cluster 的边 vol z-score 在同一区间 (如 2-4 sigma) → **质量一致** (全部安静压缩)
- 某些 cluster 的 z-score 显著高于其他 (如一个 6 sigma 其他 3 sigma) → **质量分化** (存在震荡压缩)
- $ 节点在某 cluster 中 z-score 异常高而其他节点不异常 → **$ 特异性** ($ 度量功能被特异性质疑)
- 所有节点同步异常 → 只是冲击烈度大, 不特异于 $

## 边界条件

结论翻转的条件:
1. Mahalanobis 极端阈值从 99th pct 改变 — 不同极端天子集
2. all-or-nothing 同步阈值从 5th pct 改变 — 改变同步天集合
3. cluster gap 从 7 天改变 — 影响 cluster 划分粒度
4. 波动率 z-score 基准窗口长度 — 影响基准分布

## 谱系引用

- C3 v2: 阈值因果检验 (THRESHOLD_CAUSAL, 4/4 FLAT)
- C3 cross-node check: all-or-nothing 同步确认 (8/64 全同步, 56/64 全不同步)
- 142 号蜂群: 四节点实验框架