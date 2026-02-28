---
id: '246'
number: 246
title: 多 TF pipeline 集成——238号架构接入交易管道 + 239号方向性力度替代
type: 工程架构
status: 已结算
date: 2026-02-28
source: v100-swarm multi-tf-integration 工位
depends_on:
  - '238'  # 多 TF 输入架构
  - '237'  # T6/T7 三态诊断
  - '239'  # 方向性力度
---

# 246号：多 TF pipeline 集成——238号架构接入交易管道

## 1. 结论

实现了 `MultiTFPipelineAdapter`，将 238号 `MultiTFOrchestrator` 的输出接入 `pipeline.py` 的交易管道，同时用 239号方向性力度替代振幅力度做跨级别背驰检测。

### 核心设计

| 组件 | 职责 |
|------|------|
| `buysellpoint_to_bsp` | `BuySellPoint`（多TF）→ `BSP`（nesting） |
| `cross_divergence_to_resonance_signal` | `CrossLevelDivergence` → `ResonanceSignal` |
| `detect_cross_level_divergence_directional` | 用方向性力度（∉ ker(D)）替代振幅力度（∈ ker(D)）做跨级别背驰检测 |
| `_directional_move_force` | 走势方向性力度计算（中枢密度倒数 + 线段方向持续性） |
| `MultiTFPipelineAdapter` | 集成编排器：MultiTFOrchestrator.run → 方向性力度背驰检测 → 类型转换 → pipeline 信号 |
| `MultiTFPipelineResult` | 聚合结果：原始 MultiTFResult + BSP + ResonanceSignal + T6 可达性 |

### adapter 模式

- 不修改 `pipeline.py`（trading context / state machine 不变）
- 不修改 `multi_tf_adapter.py`（238号原始代码不变）
- 不修改 `directional_force.py`（239号原始代码不变）
- 新增 `multi_tf_pipeline.py` 作为桥梁层

### 类型转换映射

**BuySellPoint → BSP：**

| MultiTF (kind, side) | BSP (BSPType) | 背驰映射 |
|----------------------|---------------|---------|
| ("type1", "buy") | B1 | cross_divergence.direction → DivergenceType |
| ("type1", "sell") | S1 | 同上 |
| ("type2", "buy") | B2 | 预留 |
| ("type3", "sell") | S3 | 预留 |

**CrossLevelDivergence → ResonanceSignal：**

| 字段 | 映射 |
|------|------|
| edge_id | `cross_tf_{high_tf}_{low_tf}` |
| level | high_tf.level_index |
| layer | INDEPENDENT_EDGE（跨 TF 背驰是独立信号源） |
| bsp | direction=top → S1, direction=bottom → B1 |

### 方向性力度替代

`_directional_move_force` 替代 `_move_amplitude`：
- 组件1：中枢密度倒数 = 1 / (zs_count / seg_span)。中枢密度低 = 走势趋势性强 = 力度高
- 组件2：线段方向持续性 = same_dir_count / total。方向一致 = 力度高
- 复合：0.5 * persistence + 0.5 * density_force

两者均 ∉ ker(D)（不依赖价格振幅），解决 237号诊断的"振幅力度 ∈ ker(D)"问题。

### T6 可达性判断

`recursive_levels_equivalent = 有走势输出的 TF 层数`。当 >= 2 时，`t6_reachable = True`。

这绕过了 237号诊断的 D2 递归压缩瓶颈——多 TF 架构下每个 TF 独立运行完整管线，跨 TF 比较替代了递归深度要求。

## 2. 定义依据

### 238号结论

> MultiTFOrchestrator 每个 TF 独立运行完整管线，跨 TF 通过时间戳对齐和力度比较关联。不修改现有 BiEngine / pipeline / RecursiveOrchestrator。

本集成保持 238号的 adapter 模式不变，在其输出端新增类型转换层。

### 237号诊断

> T6 的瓶颈是递归深度不足。高级别背驰力度使用振幅（∈ ker(D)），双重削弱信号。

本集成用方向性力度替代振幅力度，修复"双重削弱"的第二个因素。

### 239号方向性力度

> 三种方向性力度方法均 ∉ ker(D)。composite_directional_force 结合比例/密度/偏移。

本集成在跨 TF 场景下使用简化版方向性力度（中枢密度 + 方向持续性），因为跨 TF 比较的输入是 Move 而非线段序列。

### pipeline.py 数据流

pipeline 消费 `BSP` + `ResonanceSignal`：
- `locate_bsp(ctx, bsp)` 推进横向区间套
- `compute_position(ctx, resonance_signals, tolerance_fn)` 计算共振仓位
- `execute_entry(ctx, layer_type, bsp_label, position)` 入场

类型转换后的信号直接兼容这三个入口。

## 3. 边界条件

### 3.1 合成数据可能不产生走势

简单合成数据（单调波动）可能只产生 1 笔，不足以构成线段→中枢→走势。因此 T6 可达性依赖于真实市场数据的充分波动。这不是代码问题，是数据量和波动性的先决条件。

### 3.2 方向性力度在无 snapshot 时回退到默认值

`_directional_move_force` 在 `level_result.snapshot is None` 时将 persistence 设为 0.5（中性值）。此时力度计算完全依赖中枢密度。如果需要更精确的力度，应确保 snapshot 可用。

### 3.3 类型转换精度

BuySellPoint.kind 当前只有 "type1" 在多 TF 架构中实际产生。"type2"/"type3" 的映射是预留接口，实际激活需要扩展 `_derive_buysellpoints` 的逻辑。

### 3.4 共振信号层级分配

跨 TF 背驰映射为 `INDEPENDENT_EDGE` 层。如果未来多 TF 产生的信号需要与其他层级的信号区分，层级分配可能需要调整。

## 4. 下游推论

### 4.1 交易管道可消费多 TF 信号

`MultiTFPipelineResult.bsps` 和 `.resonance_signals` 可直接传入 `pipeline.pipeline_step` 的 `bsp` 和 `resonance_signals` 参数。不需要修改 pipeline 内部逻辑。

### 4.2 T6 从"结构性不可达"变为"有条件可达"

238号+246号组合绕过了 237号诊断的 D2 递归压缩瓶颈。T6 可达的条件是：至少两个 TF 的 bar 数据能独立产出走势（Move）。

### 4.3 方向性力度可独立升级

`_directional_move_force` 是模块内部函数，可独立替换为更精确的方向性力度实现（如直接调用 239号的 `composite_directional_force`），不影响外部接口。

### 4.4 与 TFOrchestrator 的流式集成

当前 `MultiTFPipelineAdapter` 是批量模式（一次性传入所有 bar）。流式模式（逐 bar 步进）可通过 `TFOrchestrator`（`orchestrator/timeframes.py`）+ 本适配器的类型转换函数组合实现。两个编排器共享 `RecursiveOrchestrator` 底层，不冲突。

## 5. 谱系引用

- **238号**（多 TF 输入架构）：直接前置。提供了 `MultiTFOrchestrator` 及其输出类型。
- **237号**（T6/T7 三态诊断）：直接前置。诊断了 T6 不可达的根因（D2 递归压缩 + 振幅力度 ∈ ker(D)）。
- **239号**（方向性力度）：直接前置。提供了 ∉ ker(D) 的力度计算方法论。本集成在跨 TF 场景下实现了简化版。
- **195号**（缠论代码拓扑化）：间接前置。T1-T8 转换函数等价框架。

## 6. 影响声明

### 新增文件

1. `src/newchan/topology/multi_tf_pipeline.py`：多 TF pipeline 集成适配器（~310 行）
2. `tests/test_multi_tf_pipeline.py`：26 个测试用例，全部通过
3. `.chanlun/genealogy/settled/246-multi-tf-pipeline-integration.md`：本谱系

### 不修改的结构

1. `src/newchan/pipeline.py`：不修改
2. `src/newchan/topology/multi_tf_adapter.py`：不修改
3. `src/newchan/topology/directional_force.py`：不修改
4. `src/newchan/orchestrator/recursive.py`：不修改
5. `src/newchan/nesting/bsp.py`：不修改
6. `src/newchan/nesting/resonance.py`：不修改
7. `tests/test_multi_tf_adapter.py`：不修改（24 个测试仍全部通过）
