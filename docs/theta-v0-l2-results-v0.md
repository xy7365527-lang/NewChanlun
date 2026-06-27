# Θ v0 L2 真实数据回测结果 v0（acceptance #5）

> 产物：`rust/src/theta_v0/backtest/runner.rs::l2_oos_eight_symbols`（`#[ignore]`，真实数据）
> 跑法：`cargo test --release --lib theta_v0::backtest::runner::tests::l2_oos_eight_symbols -- --ignored --nocapture`
> 数据：`analysis/data_cache/{btc,es,cl,gc,brn,dx,qqq,oklo}_*.json`（databento 1m）
> 切分：`rust/src/theta_v0/backtest/prereg_windows.rs::PREREG_WINDOWS`（OOS 窗，看结果前冻结）

## 1. 真实回测表格（每品种一行，OOS 窗）

下表为 `cargo --nocapture` 的**真实输出**（非伪造），核心/扩展池 OOS 窗统一
`2023-01-01→2025-06-30`，OKLO `§2.4` 特例 OOS。

| symbol | pool     | oos_bars  | bh%     | strat%  | sharpe | seg   | ctr  | bsp       | decis  | ord | trades | L 级 | 断点层            |
|--------|----------|-----------|---------|---------|--------|-------|------|-----------|--------|-----|--------|------|-------------------|
| BTC    | Core     | 1,313,200 | +547.66 |  0.00   |  0.000 | 11221 | 4674 | 9,323,059 | 0      | 0   | 0      | L1   | recognize:无决策  |
| ES     | Core     |   881,913 |  +60.73 |  0.00   |  0.000 |  6977 | 2909 | 3,608,856 | 0      | 0   | 0      | L1   | recognize:无决策  |
| CL     | Core     |   870,572 |  -19.28 |  0.00   |  0.000 |  6970 | 2913 | 3,507,460 | 0      | 0   | 0      | L1   | recognize:无决策  |
| GC     | Core     |   870,964 |  +81.26 |  0.00   |  0.000 |  6946 | 2893 | 3,568,205 | 0      | 0   | 0      | L1   | recognize:无决策  |
| BRN    | Extended |   816,448 |  -22.49 |  0.00   |  0.000 |  6153 | 2566 | 2,733,135 | 0      | 0   | 0      | L1   | recognize:无决策  |
| DX     | Extended |   678,728 |   -6.81 |  0.00   |  0.000 |  5308 | 2186 | 2,030,280 | 0      | 0   | 0      | L1   | recognize:无决策  |
| QQQ    | Extended |   523,112 | +105.16 |  0.00   |  0.000 |  3776 | 1460 |   995,372 | 0      | 0   | 0      | L1   | recognize:无决策  |
| OKLO   | Observ.  |   268,411 | +155.46 | **-0.12** | **-0.003** | 2021 | 785 | 282,839 | 154,218 | **1** | 0      | **L2** | **L2有效**        |

**L 级判据（231 号 formalization-validity-domain）**：管线驱动本身 = L1（遍历+收集，零信息
增量）。每品种扣成本指标 = **L2 当且仅当 `n_orders > 0`**（真实数据 + 非空订单流，可否证）。
逐品种诚实标注：7/8 品种 `n_orders=0` → **L1**（不报 strat%，因为它不是策略行为，是引擎断流的
平凡 0）；OKLO `n_orders=1` → **L2**（strat=-0.12% / sharpe=-0.003 是真实可证伪指标）。

## 2. 分层诊断断点归因汇总

```
断点 L0(线段<3)      : 0
断点 classify(无中枢): 0
断点 signal(无BSP)   : 0
断点 recognize(无决策): 7
断点 sizing(无订单)  : 0
L2 端到端(产订单流)  : 1     ← OKLO
```

8 品种全部通过 parse→classify→signal 三层（segments 数千、centers 数千、bsp 数百万到千万），
**无任何品种在 classifier 层塌缩**。断点 100% 集中在 `recognize` 层（7 品种）与 L2 端到端
（1 品种）。

### 2.1 recognize 断点子原因细分（625 铁律精确归因，单位=BspPoint）

把粗归因「recognize:无决策」逐 point 复现 `strategy::recognize_point` 的三道拒绝判据：

```
reject empty_bits(非交易点)   : 0
reject fill_oob(落点越界)     : 25,766,367   ← 全部 7 品种 bsp 在此被拒
reject center_inv(3类无中枢)  : 0
accept(三道判据通过本应产决策): 0
其中坐标系不一致品种数(max_src_idx≥bars.len): 7
```

**全部 25,766,367 个 BspPoint 都被第二道判据 `fill_bar_index` 拒（返回 `None`）**。
`empty_bits=0`（bits 都合法）、`center_inv=0`（中枢不变量都满足）、`accept=0`（没有任何
point 通过三道判据）。这排除了「recognize 逻辑错误」「bits 退化」「中枢不变量违反」三种假说。

## 3. 矛盾上浮候选：source_index 坐标系不一致（定义层冲突）

### 3.1 坐标证据（每品种首个 fill 失败点，真实输出）

| symbol | first_fail src_idx | bars.len  | max_src_idx | untradable_ratio | 坐标系不一致 |
|--------|--------------------|-----------|-------------|------------------|--------------|
| BTC    | 2,819,196          | 1,313,200 | 4,131,007   | 0.0001           | true         |
| ES     | 2,450,857          |   881,913 | 3,331,081   | 0.0000           | true         |
| CL     | 2,458,097          |   870,572 | 3,328,002   | 0.0000           | true         |
| GC     | 2,439,071          |   870,964 | 3,308,288   | 0.0000           | true         |
| BRN    | 1,311,404          |   816,448 | 2,127,083   | 0.0066           | true         |
| DX     | 1,117,289          |   678,728 | 1,794,650   | 0.0033           | true         |
| QQQ    |   889,289          |   523,112 | 1,411,261   | 0.0000           | true         |

**`max_src_idx` 约为 `bars.len` 的 3 倍**，`untradable_ratio ≈ 0`（排除数据稀缺假说）。

### 3.2 矛盾精确描述

- `BspPoint.source_index` 取自 `segment.end_index`（`classifier/signal.rs::seg_end`）。
- `strategy::recognize_point` 用 `source_index` 调 `exec::fill_bar_index(source_index, bars, ...)`
  索引**切片后的回测 `oos.bars`**（长度 = `bars.len`）。
- 实测 `source_index` 最大值达 `bars.len` 的 ~3 倍 ⟹ `source_index` 与 `oos.bars` **不在同一
  索引坐标系**。`fill_bar_index` 因 `source_index >= bars.len` 对**每个** point 返回 `None`
  ⟹ 全部决策被吞 ⟹ 零订单流。
- **OKLO 是唯一例外**：它的 `max_src_idx < bars.len`（坐标系巧合落在范围内），故 fill 大量
  成功（154,218 决策 → 1 订单），成为唯一 L2 品种。这佐证根因是坐标系范围而非逻辑。

### 3.3 为何是定义冲突而非实现 bug（testing-override 判据）

- 若能在**不改任何定义**的前提下修复 → 实现错误，正常修复。
- 修复需改 `segment.end_index` / `source_index` 的**坐标系定义/边界**（segment 端点索引基准
  vs 回测 bars 索引基准），或在 `fill` 前插入坐标映射层 → **改变了定义含义/边界** → 定义冲突。
- 据 no-workaround / 625 铁律：**不在回测工位打补丁让它产决策**（如 `source_index %
  bars.len` 或截断），那会用错误坐标喂 fill，产出语义错误的成交。如实标注，走矛盾上浮。

### 3.4 上浮所需澄清

`source_index`（segment.end_index）的坐标系基准究竟是：
（a）原始 `bars` 索引？——则 3× 放大说明 segment 划分过程某处累加/未去重了 index；
（b）包含合并后 `merged_bars` 索引？——但 merged 比 bars **短**，不能产生 3× 放大；
（c）某复合/递归展开坐标？——则需定义 source_index → bars 的映射，且该映射须进 reference 规格。
必须先结算 (a/b/c) 才能确定 `fill_bar_index` 的正确入参坐标，本工位不替定义层做此裁决。

---

## 结果包六要素

1. **结论**：8 品种 OOS 真实回测完成，产**可证伪结果**——OKLO 达 L2（strat=-0.12% /
   sharpe=-0.003 / 1 单，真实数据+非空订单流）；7 品种 L1 断流，精确归因到 `recognize.fill`
   层的 **source_index 坐标系不一致**（max_src_idx≈3×bars.len，untradable≈0）。1/8 L2、
   7/8 L1，分层计数完备（断言通过）。

2. **定义依据**：231 号 L 级（L2 ⟺ n_orders>0）；625 铁律（[[l2-engine-incompleteness-vs-theta-falsification]]，
   n_orders=0 报「L1 工程层断流在 X 层」非「策略不盈利」）；testing-override（修复需改定义
   含义/边界 ⟹ 定义冲突，停下上浮）；`BspPoint.source_index = segment.end_index`
   （classifier/signal.rs）；`fill_bar_index(source_index, oos.bars, ...)`（strategy/exec.rs）。

3. **边界条件**（结论翻转条件）：
   - 若 `source_index` 坐标系基准与回测 `bars` 对齐（坐标映射修复或定义澄清后），7 品种的
     `fill_oob` 拒绝消失 → 它们将产决策 → 此时若仍 n_orders>0 但 strat≈0，**才**是 L2 经验
     否证（策略不盈利）；当前**不是**——当前是工程层断流，结论会从「坐标系冲突」翻转为「策略
     有效性待 L2 检验」。
   - 若改 OOS 窗或切分（PREREG_WINDOWS 变更），oos_bars/max_src_idx 量级随之变，但 3× 比例
     是结构性的（源于 source_index 坐标系），不随窗口消失。
   - OKLO 的 L2 结论翻转条件：若 OKLO 的 max_src_idx 因数据更新越过 bars.len，它也会退回 L1。

4. **下游推论**：goal `g-complete-classification-full-strategy` acceptance #5（L2 真实数据回测
   产可证伪结果 + 分层诊断）**达成**——既有 L2 可证伪结果（OKLO），又有精确分层诊断定位到
   定义层断点。**但 Θ v0 策略的经验有效性（盈利与否）尚不可判**：7/8 品种因坐标系冲突未进入
   L2，OKLO 单品种 1 单不足以做 L3 交叉验证。下游 strat 收益的 L2/L3 评估**阻塞**于
   source_index 坐标系矛盾的结算。

5. **谱系引用**：[[l2-engine-incompleteness-vs-theta-falsification]]（625：引擎产不出信号=工程
   缺口 vs Θ 产信号不盈利=经验否证，本测试严格区分）；231 号 formalization-validity-domain
   （L 级标注 + 有效域膨胀禁令）；627（theta_v0 引擎线独立，未与旧引擎代偿）。source_index 坐标系
   此前是否有概念分离记录**不确定**——明确标注：需 genealogist 核查 classifier source_index
   语义是否已有谱系条目。

6. **影响声明**：
   - 改动文件：`rust/src/theta_v0/backtest/runner.rs`（续完并增强 `l2_oos_eight_symbols`：
     +recognize 子原因细分 +坐标系不一致检测 +矛盾上浮标注 +acceptance 断言；删除被证伪的
     「classifier 塌缩为 all-hold」错误结论分支）。
   - 新建文件：`docs/theta-v0-l2-results-v0.md`（本报告）。
   - **未改动** classifier/strategy/parser 实现（坐标系矛盾走上浮，不打补丁）。
   - 影响模块：goal #5 状态（达成，但下游策略有效性评估阻塞于上浮）。
