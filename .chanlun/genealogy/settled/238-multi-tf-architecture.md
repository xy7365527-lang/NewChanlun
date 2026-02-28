---
id: '238'
number: 238
title: 多 TF 输入架构——绕过 D2 递归压缩瓶颈
type: 工程架构
status: 已结算
date: 2026-02-28
source: v96-swarm multi-tf 工位
depends_on:
  - '229'  # T6/T7 真实数据贡献率分析
  - '237'  # T6/T7 三态诊断
---

# 238号：多 TF 输入架构——绕过 D2 递归压缩瓶颈

## 1. 结论

实现了 `MultiTFOrchestrator` 架构，用不同 TF 的 bar 数据分别构建各级别，绕过单 TF 递归中 D2（线段引擎）的压缩率瓶颈。

### 核心设计

| 组件 | 职责 |
|------|------|
| `TimeframeLevel` | TF 配置（tf_name, bar_source, level_index） |
| `MultiTFOrchestrator` | 接收多 TF bar 数据，每个 TF 独立运行 RecursiveOrchestrator |
| `CrossLevelDivergence` | 跨级别背驰检测——T6 的多 TF 替代实现 |
| `BuySellPoint` | 从跨级别背驰推导买卖点 |
| `MultiTFResult` | 聚合结果（各 TF 独立结果 + 跨级别背驰 + 买卖点） |

### adapter 模式

- 不修改现有 BiEngine / pipeline / RecursiveOrchestrator
- MultiTFOrchestrator 调用现有 RecursiveOrchestrator 多次，每次用不同 TF 数据
- 跨 TF 对齐：用时间戳对齐不同 TF 的结构

### 跨级别背驰检测逻辑

比较相邻 TF 层级的走势方向与力度：
1. 两个级别都有走势（last_move 非 None）
2. 两个级别走势方向一致
3. 低级别力度 < 高级别力度（力量衰减 → 背驰）
4. 背驰-买卖点定理：顶背驰→卖点，底背驰→买点

## 2. 定义依据

### 229号结论

> 瓶颈不在 T6/T7 逻辑本身，而在递归深度不足——递归仅到 1 级

### 237号诊断

> T6 的瓶颈是递归深度不足（recursive_levels=1）。递归深度不足的根因是 D2 层对弱方向信号的累积吸收

### 缠论区间套定理（第27课）

> 精确大转折点寻找程序定理：从高级别到低级别逐级检测背驰并收缩范围

多 TF 架构是区间套定理的直接工程实现——高级别走势来自高 TF 数据（而非低 TF 递归得到的），低级别走势来自低 TF 数据。

## 3. 边界条件

### 3.1 跨 TF 时间对齐的精度

当前实现用简单的时间戳范围过滤做跨 TF 对齐。如果两个 TF 的 bar 时间粒度差异很大（如 1min vs monthly），对齐精度可能不足。

### 3.2 力度度量的局限

当前跨级别背驰力度使用价格振幅（|high - low|），这是 237号诊断中标注的 ker(D) 成分。替代力度指标（如方向性指标）可能提升信号质量。

### 3.3 数据源一致性

多 TF 数据必须来自同一标的同一交易所。如果数据源不一致（如日线和周线来自不同提供商），价格口径可能不统一。

## 4. 下游推论

### 4.1 T6 在多 TF 架构下可达

229号结论"T6 在单 TF 递归架构下结构性不可达"被绕过。多 TF 架构下，跨级别背驰不依赖 RecursiveStack 的递归深度。

### 4.2 力度指标可独立升级

CrossLevelDivergence 的力度计算（_move_amplitude）可以独立替换为方向性指标，不影响其他组件。

### 4.3 更多 TF 层级可自然扩展

当前实现支持任意数量的 TF 层级。增加 TF（如 1min/5min/30min/daily/weekly）只需扩展 TimeframeLevel 列表。

## 5. 谱系引用

- **229号**（T6/T7 真实数据贡献率分析）：直接前置。提供了递归深度不足的实证数据和多 TF 路径建议。
- **237号**（T6/T7 三态诊断）：直接前置。提供了 D2 弱方向吸收的理论诊断和多 TF 绕过路径的理论支撑。
- **195号**（缠论代码拓扑化）：间接前置。T1-T8 转换函数等价框架，CrossLevelDivergence 是 T6 的多 TF 替代实现。

## 6. 影响声明

### 新增文件

1. `src/newchan/topology/multi_tf_adapter.py`：多 TF 输入架构核心模块
2. `tests/test_multi_tf_adapter.py`：24 个测试用例，全部通过
3. `scripts/multi_tf_prototype.py`：SPY 日线/周线验证脚本
4. `.chanlun/genealogy/settled/238-multi-tf-architecture.md`：本谱系

### 不修改的结构

1. `src/newchan/bi_engine.py`：不修改
2. `src/newchan/orchestrator/recursive.py`：不修改
3. `src/newchan/pipeline.py`：不修改
4. `src/newchan/a_nested_divergence.py`：不修改
5. `src/newchan/a_divergence.py`：不修改
