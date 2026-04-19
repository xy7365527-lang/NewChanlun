---
id: '484'
number: 484
title: "multi_tf架构审计——完整实现 + 跨TF区间套链路缺口（align_bars_by_timestamp未接入nested_divergence）"
type: 概念发现
status: 已结算
date: 2026-04-18
source: "编排者与Claude对话——SI1!多级别联立验证 → multi_tf完成度评估 → 架构定位澄清"
depends_on:
  - '237'   # T6三态诊断——D2弱方向吸收根因
  - '238'   # 多TF输入架构（MultiTFOrchestrator）
  - '239'   # 方向性力度（∉ ker(D)）
related:
  - '482'   # 资本旋转动力学——金油比实验用到多TF数据
  - '230'   # K4 独立边独立性
epistemological_level: L1（代码层验证——两文件935行全量阅读，无运行时数据）
negation_form: internal
negation_source: ""
topo_effect: null
tensions_with: []
---

# 484号：multi_tf架构审计——完整实现 + 跨TF区间套链路缺口

**认识论等级**: L1（代码层验证：两文件全量阅读，未运行，无真实数据验证）

## 发现过程

用户在 SI1!（COMEX白银）做了周线→日线→4小时→30分钟的多级别联立判断，要求评估
`multi_tf_pipeline.py` 和 `multi_tf_adapter.py` 的完成度，以及能否打通跨时间周期自动联立。

全量阅读两文件（935行），形成以下评估。

## 核心结论：两文件完全实现，架构定位与用户需求不同构

### 完成度评估

| 文件 | 行数 | 实现状态 | stub/TODO |
|------|------|---------|-----------|
| `topology/multi_tf_adapter.py` | 470行 | **完全实现** | 无 |
| `topology/multi_tf_pipeline.py` | 465行 | **完全实现** | 无 |

`multi_tf_adapter.py` 完整实现：
- `TimeframeLevel`、`LevelResult`、`CrossLevelDivergence`、`BuySellPoint`、`MultiTFResult` 数据类
- `MultiTFOrchestrator.run()`：每个TF独立运行RecursiveOrchestrator，相邻TF两两比较跨级别背驰
- `align_bars_by_timestamp()`：按时间戳范围过滤bar数据（已实现，但只是过滤工具）

`multi_tf_pipeline.py` 完整实现：
- `_directional_move_force()`：方向性力度（239号）——中枢密度倒数 × 线段方向一致性
- `detect_cross_level_divergence_directional()`：用方向性力度替代振幅力度
- `buysellpoint_to_bsp()`、`cross_divergence_to_resonance_signal()`：类型转换层
- `MultiTFPipelineAdapter.run()`：完整四步管线，计算等效递归深度和T6可达性

### 架构定位澄清（核心）

**multi_tf解决的问题（237/238号）**：
- 单TF递归架构下，D2（线段引擎）弱方向吸收在递归叠加中累积
- `recursive_levels` 无法突破1——原因：每一层的素材是低一层的"已确认走势"，D2压缩率不足
- **解决方案**：不同TF的bar数据分别跑独立RecursiveOrchestrator，绕过单TF的D2压缩瓶颈

**multi_tf不解决的问题**：
- 第27课区间套：高级别背驰C段的**时间范围**约束低级别bar搜索范围
- 用户需求："日线确定方向+30分钟找入场"的自动联立
- 这两者都是**跨时间周期的范围传递**——高级别的时间窗口决定低级别的搜索域

### 真正的缺口

```
现有：align_bars_by_timestamp(bars, start, end) → 过滤bar
缺少：高级别背驰C段 → 自动提取时间范围 → 传入低级别bar过滤 → 低级别 nested_divergence_search
```

完整链路（第27课跨TF版本）：

```
日线 RecursiveOrchestrator
  → 检测到日线级别背驰（C段：bar_i 到 bar_j）
  → 提取时间范围 [ts_i, ts_j]
  → 调用 align_bars_by_timestamp(30min_bars, ts_i, ts_j)
  → 过滤后的30min_bars 跑 RecursiveOrchestrator
  → 在30min结构内搜索 NestedDivergence（a_nested_divergence.py）
  → 输出精确入场信号
```

`align_bars_by_timestamp()` 已实现，但**调用方**——"从高级别背驰C段自动提取时间范围并传递给低级别"的逻辑——**尚未实装**。

## 被否定的

| 被否定 | 否定者 | 原因 |
|--------|--------|------|
| multi_tf = 跨时间周期联立 | 代码审计 | multi_tf解决D2压缩瓶颈，不解决高级别→低级别范围传递 |
| align_bars_by_timestamp已接入联立链路 | 代码审计 | 函数存在但无调用方自动传递C段时间范围 |

## 边界条件

| 条件 | 当前 | 翻转阈值 |
|------|------|----------|
| 两文件完全实现 | L1代码层验证 | 运行时发现未测试分支有逻辑错误 |
| align_bars_by_timestamp未接入联立 | 代码层确认 | 若有外部调用者传入C段时间范围，则缺口已弥补 |
| multi_tf定位 = D2压缩绕过 | 237/238号谱系及注释确认 | 若237号诊断被否定，则multi_tf定位需重评 |

## 下游推论

### 推论1：实现跨TF联立的最小可行路径

最小可行路径：
1. 在 `MultiTFOrchestrator.run()` 或新建函数中，遍历高级别TF的背驰列表
2. 对每个背驰，提取C段的起止时间戳
3. 调用已有的 `align_bars_by_timestamp()` 过滤低级别bars
4. 对过滤后的bars跑低级别RecursiveOrchestrator
5. 在低级别结果上调用 `a_nested_divergence.py` 搜索区间套

**认识论等级**：L0（设计推论，未实现）。

### 推论2：现有multi_tf的T6可达性计算偏乐观

`t6_reachable = active_levels >= 2`

这判断的是"有2个TF都有走势"，不是"高级别背驰已对低级别进行范围约束"。
T6（跨级别背驰买卖点）的真实条件是：低级别走势在高级别背驰范围**内**且力度衰减——
现有计算不检查"范围内"这个条件。

**认识论等级**：L0（逻辑推论）。

## 影响声明

- 本谱系不修改任何代码
- `topology/multi_tf_adapter.py` 和 `topology/multi_tf_pipeline.py` 状态：完整实现，架构定位为D2压缩绕过（238号）
- 跨TF区间套联立链路（第27课完整实现）为待实装状态
- 若用户要实现"日线方向+30分钟入场"自动化，需在 `multi_tf_adapter.py` 中新增C段时间范围传递逻辑

## 谱系关联

related_records:
  parent: null
  related:
    - '237'   # T6三态诊断——D2弱方向吸收
    - '238'   # MultiTFOrchestrator架构
    - '239'   # 方向性力度
    - '482'   # 金油比实验（跨TF数据使用场景）
  children: []
