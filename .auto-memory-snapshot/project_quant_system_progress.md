---
name: quant-system-progress
description: 量化交易系统M1/M2进展：完整版赋格回测+K4选股+全市场端到端
metadata: 
  node_type: memory
  type: project
  originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---

## M1 操盘层回测进展（2026-06-05/06）

### 控制变量实验历程

| 版本 | QQQ | OKLO | HK700 | BTC | 关键变量 |
|------|-----|------|-------|-----|---------|
| 单层PH H组 | -19.8% | -99% | -84% | — | 基线 |
| 多级别PH双向 | +58.3% | +72.3% | +222.8% | — | PH分层 |
| 纯做多 | +71.7% | +228% | +292.9% | — | 去掉做空 |
| persistence过滤 | +58.7% | +194.4% | +313.0% | — | 高persistence才触发退出 |
| +区间套 | +60.7% | +192.4% | +325.3% | — | 进场精确化 |
| +去掉加仓 | **+138.9%** | **+652.3%** | **+1644.6%** | — | 加仓是负alpha根因 |
| +做空（个股） | +53.9% | +934.5% | +4086.3% | — | 指数不做空个股做空 |
| BTC完整版3年 | — | — | — | +486.6% (vs BH+345%) | 首次crypto验证 |

### 关键发现

1. **加仓是隐藏的负alpha因素**——扩大BSP ID集合引入额外退出触发器。去掉后所有标的暴增。
2. **做空效果取决于标的属性**——指数（QQQ）做空亏，个股（OKLO/HK700/BTC）做空赚。根因：指数有幸存者偏差向上漂移。
3. **persistence过滤有效**——约2/3走势settle是噪音，过滤后退出质量大幅提高。
4. **区间套通过过滤坏入场产生alpha**——不是找更好价位，而是放弃不好的入场。

### 最终基线

- 指数标的：D2无加仓版纯做多（QQQ +138.9%）
- 个股/crypto标的：D2无加仓版+做空（OKLO +934.5%, HK700 +4086.3%, BTC +486.6%）

## M2 选股层进展

### 已实现
- selection_pool.py / asset_classifier.py / omega_regime.py — 品种池+方向表
- K4六边比价递归（ES/GC/CL/SPY/GLD/USO 1min数据）
- ω L2验证：相关系数-0.22，单独预测力有限

### 全市场端到端回测（v1日线版，有bug）
- K4日线递归太粗，SPY 90%时间判为盘整
- OKLO/BTC组合回测与单标的不一致（-140%/-99% vs +934%/+486%），需诊断
- HK700 K4选股产生+274%增量（阻止灾难性做空）

### 待做
- 用1min递归做K4配置（已开任务）
- 诊断OKLO/BTC组合回测bug（已开任务）
- 用ES/GC/CL期货真实数据替代ETF代理（Databento已拉到）
- 比价关系递归驱动选股（六边各自递归→配置空间动态演化）
- 跨国K4拓扑（graph.py已有CROSS_NATIONAL边类型定义）

## 数据资产

| 标的 | 文件 | bars | 来源 |
|------|------|------|------|
| QQQ | qqq_1m_databento_full.json | 728K | Databento |
| OKLO | oklo_1m_databento_full.json | 333K | Databento |
| HK700 | hk700_1m_tws.json | 1.4M | TWS |
| BTC | btc_1m_3year.json | 1.8M | Binance归档 |
| SPY | spy_1m_full.json | 1.2M | Databento |
| GLD | gld_1m_full.json | 790K | Databento |
| USO | uso_1m_full.json | 726K | Databento |
| UUP | uup_1m_full.json | 158K | Databento |
| ES期货 | es_1m_databento.json | — | Databento |
| GC期货 | gc_1m_databento.json | — | Databento |
| CL期货 | cl_1m_databento.json | — | Databento |

## 文档资产

- docs/ROADMAP.md — 项目总纲领（358行）
- docs/architecture/complete_fugue_design.md — 完整版赋格设计（780行）
- docs/architecture/ibkr_quant_system.md — IBKR量化架构（1029行）
- docs/architecture/m2_selection_layer.md — M2选股层架构

**Why:** 这是整个量化系统的核心进展记录
**How to apply:** 新conversation接续时参考此记录了解当前状态和待做事项
