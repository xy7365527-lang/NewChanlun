---
id: "363"
type: 验证报告
title: "Cerf 分岔 L2 验证——加权 Morse 未改善均匀 Morse 的否定性预测能力"
status: 已结算
date: 2026-03-05
depends_on: ["360", "355"]
related: ["351", "068", "087", "089"]
negated_by: []
negates: []
---

# 363 -- Cerf 分岔 L2 验证（否定性结果）

**认识论等级**: L2（真实 relations.jsonl + 355号 ground truth 对比）

## 结论

加权 Morse Cerf 分岔分析**未能改善**均匀 Morse 对"关键否定"事件的预测能力。B-M 研究线的核心假设——"语义权重能让 Morse 理论更好地捕捉概念层否定"——在 L2 验证中被否证。

## 定义依据

- **Cerf 分岔**定义来自 `cerf_bifurcation.py`：三种标准（临界数量变化、Betti 数变化、权重偏移），阈值 0.3
- **加权 Morse**定义来自 `weighted_morse.py`：关系类型权重 negates(4) > supersedes(3) > modifies(2) > depends_on(1)
- **Ground truth** 来自 355号验证报告：22个关键否定事件（编排者标注 + 规则引用 + INTERRUPT）

## 验证数据

| 指标 | 加权 Cerf | 均匀 Morse (DM2) |
|------|-----------|------------------|
| 检测事件数 | 292 | 345 |
| 关键否定未检出数 | 5/22 | 1/22 |
| Top-5 Precision | 0.2000 | 0.0000 |
| Top-10 Precision | 0.1000 | 0.1000 |
| Top-20 Precision | 0.1000 | 0.1000 |
| Top-50 Precision | 0.1000 | 0.1200 |
| 全量 Recall | 0.7727 | 0.9545 |
| 平均排名（GT keys） | 127.3 | 123.1 |
| 中位排名（GT keys） | 152 | 154 |
| 排名更优计数 | 7 | 14 |

### 关键发现

1. **087号进入 Top-5**：加权 Cerf 将 087号排到第4位（均匀 Morse 排第6位）。这是因为 087号同时引入大量 Betti_2 变化（beta_2: 200->225，delta=25），加权策略没有改变这一效果
2. **5个关键否定完全未检出**：157, 161, 162, 275, 277 在加权 Cerf 中无分岔信号（均匀 Morse 仅 275 未检出）。加权策略的阈值效应**反而减少了召回率**
3. **14/22 关键否定在均匀 Morse 中排名更优**：加权策略整体上没有提升排名，反而因为改变配对优先级导致一些关键否定的临界集变化被配对消除
4. **分岔类型单一**：292个分岔中288个是 betti_change 类型，3个 compound，1个 critical_count。weight_shift 标准从未独立触发——加权策略没有产生与 Betti 变化不同的信号

### 087号的特殊性（与355号一致）

087号在两种方法中都排名靠前（Cerf #4, DM2 #6），且 086号紧随其后（Cerf #11, DM2 #9）。这确认了 355号的发现：否定对(086, 087)的高排名源于**广度**（同时影响多个连接），不是 Morse 理论能区分"否定"。

## 边界条件

| 条件 | 当前状态 | 翻转阈值 |
|------|---------|---------|
| 加权策略 | negates(4)/depends_on(1) 线性权重 | 非线性权重（如 negates >> all）或不同加权策略可能改变结果 |
| Cerf 阈值 | 0.3（30% 相对变化） | 不同阈值可能改变 precision/recall 平衡 |
| Ground truth | 355号的22个关键否定 | 更完整/不同的 ground truth 可能改变结论 |
| 单标的 | 仅本谱系 DAG | 其他概念体系的 DAG 可能有不同结果 |

## 下游推论

1. **B-M 研究线关闭** — resolved（370号结晶）：三步（optimal_morse + weighted_morse + cerf_bifurcation）全部完成 L2 验证。结论：加权 Morse 和 Cerf 分岔不是"关键否定"的预测器
2. **Morse 理论在本 DAG 上的有效域确认** — resolved（370号结晶）：Morse 理论能度量拓扑变化的**量**，但不能识别变化的**语义角色**（否定 vs 丰富化 vs 结构性添加）
3. **355号结论增强** — resolved（370号结晶）：355号发现"均匀 Morse 跳跃幅度不是关键否定的强预测器"，363号确认"加权 Morse + Cerf 分岔也不是"。这从两个方向封闭了 Morse 理论对"关键否定"预测的研究空间
4. **否定性结果的价值** — resolved（370号结晶）：确认了有效域的边界——Morse 理论的有效域是"拓扑变化量度量"，不包含"概念语义角色识别"

## 影响声明

- **新增脚本**: `scripts/cerf_l2_validation.py`
- **新增数据**: `experiments/discrete_morse/cerf_l2_validation_results.json`
- **不改动已有代码**：纯增量验证
- **关闭研究线**：B-M（optimal_morse -> weighted_morse -> cerf_bifurcation）-> L2 验证完成

## 谱系引用

| 关联谱系 | 关系 |
|---------|------|
| 360号 | 依赖——B-M 三步完成（L0+L1），本验证提升到 L2 |
| 355号 | 依赖——均匀 Morse L2 否定性结果 + ground truth 来源 |
| 351号 | 间接——Morse 理论研究线启动 |
| 068号 | 间接——连续到离散范式转移 |
| 087号 | 间接——Top-5 中唯一的关键否定命中 |
| 231号 | 适用——形式化有效域规则（本验证确认了 Morse 理论的有效域边界） |
