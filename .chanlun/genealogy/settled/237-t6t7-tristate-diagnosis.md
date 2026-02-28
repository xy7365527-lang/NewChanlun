---
id: '237'
number: 237
title: T6/T7 三态诊断——229号递归深度问题的三态根因分类
type: 诊断分析
status: 已结算
date: 2026-02-28
source: v96-swarm 229-reopen 工位
depends_on:
  - '229'  # T6/T7 真实数据贡献率分析
  - '235'  # 离散化算子 D 的三态分类
  - '236'  # 纤维丛集成
---

# 237号：T6/T7 三态诊断——229号递归深度问题的三态根因分类

## 1. 结论

用 235号三态分类工具重新审视 229号发现的 T6/T7 低贡献率问题。核心诊断结果：

| 特征 | T6（嵌套背驰） | T7（买卖点） |
|------|----------------|-------------|
| 三态位置 | conditional-on-absorbed | preserved-but-starved |
| 根因 | D2 弱方向吸收的递归累积 → recursive_levels=1 | 方向信息 preserved，但上游结构稀缺 |
| 纤维丛影响 | 无 | 无 |
| 可操作性 | 仅多 TF 输入可绕过 | 同上（上游问题共享） |

### T6：conditional-on-absorbed

T6 的瓶颈是递归深度不足（recursive_levels=1）。递归深度不足的根因是 D2 层对弱方向信号的累积吸收——每次递归迭代，D2 都大规模压缩输入数据量。在三态框架中，这属于 ker(D) 在递归维度的表现：D2 的弱方向吸收使得递归叠加后数据量指数衰减。

此外，T6 的高级别背驰力度判断步骤（`_amplitude_force`）使用价格振幅（∈ ker(D)），双重削弱。

### T7：preserved-but-starved

T7 的核心判断步骤大多基于方向信息（∉ ker(D)）：
- Type1 方向判定、Type2 回调搜索、Type3 突破方向——纯方向信息，D 保留
- Type3 回试范围检查（low > zg / high < zd）——虽使用价格数值，但实质是拓扑判断（中枢内外），∉ ker(D)

T7 低产出不是因为信号被 D 吸收，而是因为上游结构稀缺——中枢/走势/背驰太少。上游不足的根因与 T6 共享：D2 压缩率问题。

### 纤维丛修正：无通道

代码审查确认：
- `a_nested_divergence.py`：不导入 Configuration/polarity_index
- `a_buysellpoint_v1.py`：不导入 Configuration/polarity_index
- T6/T7 在构造层（bi/segment/zhongshu/move）操作
- 纤维丛修正在策略层（Configuration → polarity → scan_configuration）操作
- 两者在 pipeline 中没有数据通路

236号发现的 45% polarity 差异影响的是策略层决策，不影响构造层信号产生。

## 2. 定义依据

### 三态分类（235号）

| 态 | 定义 | T6/T7 关联 |
|----|------|-----------|
| 吸收（ker(D)） | 纯幅度耦合 → 0 | T6 高级别背驰力度（振幅）∈ ker(D)；D2 弱方向吸收是递归深度瓶颈 |
| 保留 | 底空间独立耦合不变 | T7 方向判断 ∉ ker(D)，信息保留完整 |
| 放大 | 方向耦合被噪声滤除放大 | L1 MACD 背驰经 D2 滤噪后信噪比上升，但不解决递归深度问题 |

### 离散化算子 D 的递归结构

D = D3 . D2 . D1，递归应用 D：

```
L1: bar → bi → segment → zhongshu → move  (第一次 D)
L2: settled(L1 moves) → bi' → segment' → zhongshu' → move'  (第二次 D)
终止: len(moves) < 3
```

229号实证压缩率：
- AAPL 6621 bar → 574 bi → 7 segments → 1 zhongshu → 1 move → recursive_levels=1
- GOOGL 5416 bar → 476 bi → 9 segments → 2 zhongshus → 1 move → recursive_levels=1

### T6/T7 各步骤信息类型标注

**T6 嵌套背驰：**

| 步骤 | 信息类型 | 三态位置 |
|------|---------|---------|
| 递归深度检查 | direction | conditional（依赖 D2 累积吸收） |
| 高级别趋势背驰力度 | mixed（振幅力度） | absorbed（振幅 ∈ ker(D)） |
| 高级别盘整背驰力度 | mixed（振幅力度） | absorbed |
| 区间套收缩 | direction（位置索引） | preserved |
| L1 MACD 背驰 | mixed（MACD面积） | amplified（方向信噪比上升） |

**T7 买卖点：**

| 步骤 | 信息类型 | 三态位置 |
|------|---------|---------|
| Type1 趋势背驰触发 | mixed | absorbed（力度成分） |
| Type1 方向判定 | direction | preserved |
| Type2 回调搜索 | direction | preserved |
| Type3 突破方向 | direction | preserved |
| Type3 回试范围检查 | amplitude（但拓扑实质） | preserved |

## 3. 边界条件

### 3.1 多 TF 输入可能改变结论

如果引入多 TF 输入（如日线+周线+月线分别构建各级别），D2 的递归累积吸收被绕过。此时：
- T6 的递归深度可能 ≥ 2（不再受限于单 TF 的压缩率）
- T7 的上游结构可能更丰富（各 TF 独立产出中枢/走势）
- 高级别背驰力度问题（振幅 ∈ ker(D)）仍然存在

### 3.2 替代力度指标可能改善 T6 信号质量

如果高级别背驰力度使用方向性指标（∉ ker(D)）替代价格振幅（∈ ker(D)），信号质量可能提升。但这不解决递归深度问题——递归深度是先决条件。

### 3.3 D2 压缩率是公理约束

D2 的弱方向吸收来自缠论线段定义：
- 线段至少三笔
- 特征序列分型确认终结
- 古怪线段（笔破坏未发展为线段破坏）被吸收

修改 D2 压缩率 = 修改线段定义 = 修改缠论公理。这不是工程选择。

## 4. 下游推论

### 4.1 T6 在单 TF 递归架构下结构性不可达——229号结论在三态框架下得到强化

229号通过实证发现递归深度不足。237号在三态框架下给出了理论解释：D2 弱方向吸收的递归累积是根因。这是比"数据量不足"更精确的诊断——不是数据量问题，是 D2 公理性吸收的递归叠加问题。

### 4.2 T7 的方向信息保留意味着：如果上游结构足够，T7 本身不是瓶颈

T7 的判断逻辑大部分 ∉ ker(D)，这意味着一旦上游（中枢/走势）足够丰富，T7 的信号产出不受 D 的选择性过滤影响。问题纯粹在上游供给。

### 4.3 纤维丛修正不打开 T6/T7 的新通道

236号纤维丛修正与 T6/T7 在 pipeline 中无数据通路。45% polarity 差异是策略层信息，不反哺构造层。

### 4.4 多 TF 输入是唯一可操作路径

三态分析与 229号路径建议一致：多 TF 输入绕过 D2 递归累积。三态框架补充了理论依据——问题定位在 D2 层的公理性吸收，不在 D1 或 D3。

## 5. 谱系引用

- **229号**（T6/T7 真实数据贡献率分析）：直接前置。提供了 T6/T7 低贡献率的实证数据和"递归深度不足"的初步诊断。
- **235号**（离散化算子 D 的三态分类）：直接前置。提供三态分类工具（classify_coupling, analyze_amplification）和 D 各层的吸收机制描述。
- **236号**（纤维丛集成）：直接前置。236号 45% polarity 差异与 T6/T7 的关系需要明确——结论是无直接影响。
- **233号**（ker(D) 结构）：间接前置。ker(D) = 纯幅度耦合的理论基础。
- **195号**（缠论代码拓扑化）：间接前置。T1-T8 转换函数等价框架。

## 6. 影响声明

### 新增文件

1. `scripts/t6t7_tristate_diagnosis.py`：诊断脚本，对 T6/T7 每个判断步骤标注信息类型和三态位置
2. `tmp/t6t7-tristate-diagnosis.json`：诊断结果 JSON
3. `.chanlun/genealogy/settled/237-t6t7-tristate-diagnosis.md`：本谱系

### 不修改的结构

1. `src/newchan/a_nested_divergence.py`：不修改
2. `src/newchan/a_buysellpoint_v1.py`：不修改
3. `src/newchan/topology/discretization_kernel.py`：不修改
4. `src/newchan/topology/fiber_pipeline_adapter.py`：不修改

### 对 229号的修正

229号结论"递归深度不足"被三态框架精化为：

- 旧：数据量不足 / 更长时间跨度可能改善
- 新：D2 弱方向吸收的递归累积（ker(D) 在递归维度的表现），不随数据量增加改善（229号 v90-swarm 已证伪），只能通过多 TF 输入绕过

这不是否定 229号，而是在三态框架下给出更精确的根因分类。
