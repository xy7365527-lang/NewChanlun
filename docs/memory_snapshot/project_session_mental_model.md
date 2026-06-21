---
name: session-mental-model
description: 本session全局心智模型：引擎O(N)完成、谱系生产7命题、P1否证、P3待跑、Version I架构误解已发现
metadata:
  type: project
  originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---

## 引擎层（已完成）

- **全链路O(N)达成**：bi_zhongshu路径75×（四层增量器）+ 主链segment层98×（SegCheckpoint resume）
- **Rust 8层bit-exact**：K线→分型→笔→线段→中枢→走势→背驰→买卖点，Python↔Rust逐位等价
- **L3跨标的验证**：OKLO/ES/GC/CL/DX 全部零mismatch
- **数据**：7期货10年1min（Databento）+ BTC 4.6M bars（Binance归档2017-2026）+ 中国IF/AU/SC（akshare 1023根/次）

## 谱系生产（核心概念层产出）

7个新命题从已结算否定中扬弃推导：

| 命题 | 状态 | 结论 |
|------|------|------|
| **P0** 谱系-实证断层 | L0成立 | 500条settled全偏概念层，实证否定未结晶 |
| **P1** ⋆=D（Hodge star=缠论D算子）| **L2否证** | 47.3%一致率≈抛硬币，退化bar上persistence≡amplitude |
| **P2** ω=配置驻留尺度参数 | L3待验 | 依赖K4转换矩阵+ω列 |
| **P3** alpha=暴露守恒 | **L2-L3待判决** | 脚手架完成，核心函数待确认后跑 |
| **P4** K4内禀信息下界 | L3待验 | 依赖固定全L3重算双熵 |
| **P5** ⋆=D推广 | **随P1否证降级** | 降为启发式 |
| **P6** 门控门regime切换 | L3待验 | 依赖P3+P4 |

## Version I架构误解（关键bug）

**正确理解**（用户纠正）：
- 一次满仓建仓100%暴露
- 区间套确认的最高级别买卖点决定出场
- 最高级别以下所有级别都是"多重赋格降成本"空间
- 降成本是在同一个仓位上叠加操作，不是独立slice
- 操作后仓位恢复100%

**当前实现的错误**：
- 每个级别独立FSM各管一个slice（机械化）
- sub-level卖点可能触发清仓（应该只触发降成本短差）
- 入场可能是ladder式逐级建仓（应该一次满仓）
- 审计任务在跑，精确到行号的报告即将完成

## 关键否定性结果汇总

| 否定 | 精确边界 | 保留 |
|------|---------|------|
| COT 7/7不领先 | 所有口径、所有标的 | 残差方向仍有效（内生反转点） |
| ω→美股方向 | p=0.069反向 | ω→金油比自身均值回归L2确认 |
| 门控毁alpha | K4门控932→128% | 排序保留96%（暴露守恒） |
| 2.074 bits不可约 | Γ→交叉边 | 交叉边必须直接读，六边各自跑 |
| ⋆=D | L1退化bar残差 | 高级别/非退化bar未关闭 |
| 流速空信号 | 2020饱和L3 | 流量（振幅）在内生反转点有效 |

## 正在跑的任务

- BTC V-I回测：I信号32%→1.5M减速
- Version I架构审计：在写报告
- P3随机门控：脚手架完成待确认
- V-I 7期货：后台nohup
- K4矩阵：数据完成写报告

## 度规问题（开放）

P1否证后度规问题回到开放状态。剩余候选：
- 方向4：K4转换矩阵平稳分布作为内禀度规（P4假说，不依赖⋆=D）
- 非退化bar残差上⋆=D重检（persistence≠amplitude时）
- 高级别走势方向的方向性力度（需可靠settle_ts锚定）

**Why:** 防止context compact后丢失关键结论和状态
**How to apply:** 新session开始时先读此文件获取全局心智模型

Related: [[quant-system-progress]], [[project_todo_master]], [[project_m3_capital_flow_milestone]]
