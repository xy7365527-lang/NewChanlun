# Θ v0 L2 真实数据回测结果 v0（acceptance #5）

> 产物：`rust/src/theta_v0/backtest/runner.rs::l2_oos_eight_symbols`（`#[ignore]`，真实数据）
> 跑法：`cargo test --release --lib theta_v0::backtest::runner::tests::l2_oos_eight_symbols -- --ignored --nocapture`
> 数据：`analysis/data_cache/{btc,es,cl,gc,brn,dx,qqq,oklo}_*.json`（databento 1m）
> 切分：`rust/src/theta_v0/backtest/prereg_windows.rs::PREREG_WINDOWS`（OOS 窗，看结果前冻结）

## 0. 报告核心结论（先于数据，TL;DR）

本次 L2 跑的**核心产出不是回测收益，是一个被反事实对照坐实的根因诊断**：

**8 品种全部坐标系断裂——`slice_date_window` 切 OOS 窗时未重置 `Bar.source_index`，
保留了全数据集绝对偏移，导致 `fill_bar_index` 用全集偏移索引 OOS 局部数组而越界，
吞掉全部决策。这是 (b) L1 实现 bug，不是 (a) 引擎在真实数据上的有效域边界，也不是定义冲突。**

直接后果（**改写了原 v0 报告**）：
- 原结论「7/8 品种 L1 工程层断流（引擎产不出信号）」**不准确**——引擎**产了**信号（反事实
  对照下 BTC 产 9,323,059 个决策）。准确表述：引擎产信号，回测**数据装配层**的 source_index
  坐标系 bug 在 fill 层吞掉决策。
- **OKLO 的 L2 标注作废**——OKLO 同样坐标系断裂，其 strat=-0.12% 仅覆盖 OOS 中
  source_index<bars.len 的前段（~54%）信号，是污染结果，不是干净 L2 否定性结果。
- **bug 修复（`data.rs::slice_date_window` 重置 source_index）前，#5 无任何可信 L2 结果。**

## 1. 真实回测表格（每品种一行，OOS 窗）— 受 bug 污染，非可信 L2

下表为 `cargo --nocapture` 的**真实输出**，但 strat%/sharpe 列受 §3 坐标系 bug 污染，
**不可作为 Θ 策略经验有效性证据**。保留它用于诊断对照（展示 bug 的可观测后果）。

| symbol | pool     | oos_bars  | bh%     | strat% | sharpe | seg   | ctr  | bsp       | decis   | ord | trades | L 级（污染） | 断点层           |
|--------|----------|-----------|---------|--------|--------|-------|------|-----------|---------|-----|--------|-------------|------------------|
| BTC    | Core     | 1,313,200 | +547.66 |  0.00  |  0.000 | 11221 | 4674 | 9,323,059 | 0       | 0   | 0      | L1(bug)     | recognize:无决策 |
| ES     | Core     |   881,913 |  +60.73 |  0.00  |  0.000 |  6977 | 2909 | 3,608,856 | 0       | 0   | 0      | L1(bug)     | recognize:无决策 |
| CL     | Core     |   870,572 |  -19.28 |  0.00  |  0.000 |  6970 | 2913 | 3,507,460 | 0       | 0   | 0      | L1(bug)     | recognize:无决策 |
| GC     | Core     |   870,964 |  +81.26 |  0.00  |  0.000 |  6946 | 2893 | 3,568,205 | 0       | 0   | 0      | L1(bug)     | recognize:无决策 |
| BRN    | Extended |   816,448 |  -22.49 |  0.00  |  0.000 |  6153 | 2566 | 2,733,135 | 0       | 0   | 0      | L1(bug)     | recognize:无决策 |
| DX     | Extended |   678,728 |   -6.81 |  0.00  |  0.000 |  5308 | 2186 | 2,030,280 | 0       | 0   | 0      | L1(bug)     | recognize:无决策 |
| QQQ    | Extended |   523,112 | +105.16 |  0.00  |  0.000 |  3776 | 1460 |   995,372 | 0       | 0   | 0      | L1(bug)     | recognize:无决策 |
| OKLO   | Observ.  |   268,411 | +155.46 | -0.12  | -0.003 |  2021 |  785 |   282,839 | 154,218 | 1   | 0      | ~~L2~~污染  | L2有效(部分信号) |

**L 级标注（231 号，修正）**：原 v0 标 OKLO=L2、7 品种=L1。**修正后全 8 品种均不可标 L2**——
strat/sharpe 受 source_index 坐标系 bug 系统性污染。OKLO 的 n_orders=1 来自仅前 54% 信号的部分
回测，不是真实数据 + 完整订单流，不满足 L2（可证伪扣成本指标）的诚实条件。

## 2. 分层诊断断点归因汇总

```
断点 L0(线段<3)      : 0
断点 classify(无中枢): 0
断点 signal(无BSP)   : 0
断点 recognize(无决策): 7
断点 sizing(无订单)  : 0
L2 端到端(产订单流)  : 1     ← OKLO（污染，见 §3.3）
```

8 品种全部通过 parse→classify→signal 三层（segments 数千、centers 数千、bsp 数百万到千万），
**无任何品种在 classifier 层塌缩**。断点集中在 `recognize` 层。

### 2.1 recognize 断点子原因细分（逐 BspPoint 三道判据，单位=point）

```
reject empty_bits(非交易点)   : 0
reject fill_oob(落点越界)     : 25,766,367   ← 全部 7 品种 bsp 在此被拒
reject center_inv(3类无中枢)  : 0
accept(三道判据通过本应产决策): 0
坐标系不一致品种数(max_bsp_src≥bars.len): 7
```

**全部 25,766,367 个 BspPoint 都被第二道判据 `fill_bar_index` 拒**。`empty_bits=0`、
`center_inv=0`、`accept=0`，排除了「recognize 逻辑错误」「bits 退化」「中枢不变量违反」。
断点 100% 在 fill 层的 source_index 越界。

## 3. 根因坐实：(b) L1 实现 bug（反事实对照，非推理）

### 3.1 坐标证据（全 8 品种，真实输出）

| symbol | oos_first_src | max_bsp_src | bars.len  | 断裂  | decisions(原) | decisions(reindex 反事实) |
|--------|---------------|-------------|-----------|-------|---------------|----------------------------|
| BTC    | 2,817,999     | 4,131,007   | 1,313,200 | true  | 0             | 9,323,059                  |
| ES     | 2,449,435     | 3,331,081   |   881,913 | true  | 0             | 3,608,856                  |
| CL     | 2,457,529     | 3,328,002   |   870,572 | true  | 0             | 3,507,460                  |
| GC     | 2,437,488     | 3,308,288   |   870,964 | true  | 0             | 3,568,205                  |
| BRN    | 1,310,703     | 2,127,083   |   816,448 | true  | 0             | 2,733,135                  |
| DX     | 1,116,146     | 1,794,650   |   678,728 | true  | 0             | 2,030,280                  |
| QQQ    |   888,487     | 1,411,261   |   523,112 | true  | 0             |   995,372                  |
| OKLO   |    74,871     |   343,217   |   268,411 | true  | 154,218(部分) | （已部分成功，见 §3.3）    |

**`oos_first_src` 对所有品种 > 0**（BTC=281万）——证明 OOS 是全集中段切片，`slice_date_window`
保留了全集绝对 source_index。BTC 的 oos_first_src=2,817,999 **已超** bars.len=1,313,200 ⟹ OOS
**首 bar 的 source_index 就越界** ⟹ 几乎全部 point 的 fill 失败。

### 3.2 反事实对照坐实 a/b（核心证据）

测试内**只读重建**（不改 data.rs）：把 `oos.bars` 的 source_index 由全集绝对偏移改为局部下标
`0..len`，管线其余完全不变，重跑 parse→classify→recognize：

**7 品种 decisions 由 0 全部转非零（百万级，每 bsp 产决策）。唯一改变的变量 = source_index。**
断言 `n_fixed_by_reindex(7) == n_coordsys_mismatch(7)` 通过 ⟹ 根因唯一。

→ **坐实 (b)**：不是 (a) 真实拒绝（信号确实存在），不是定义冲突。

### 3.3 OKLO 为何「部分成功」（推翻原「坐标系巧合一致」归因）

OKLO `max_bsp_src=343217 ≥ bars.len=268411`，**同样断裂=true**。但 `oos_first_src=74871 <
bars.len` ⟹ OOS **前段** bar 的 source_index 落在局部数组内，这些 point 的 fill 成功
（154,218 decisions），只有 source_index≥bars.len 的**后段**信号被吞。故 OKLO 的 strat=-0.12%
是**坐标系断裂下仅覆盖 OOS 前 ~54% 信号的污染回测**，不是干净 L2。原 v0 报告「OKLO 坐标系巧合
一致所以成功」的归因**被证伪**——OKLO 也断裂，只是部分而非全部越界。

### 3.4 根因链（机器证据）

```
load_symbol(data.rs:206)  : Bar.source_index = 全数据集绝对下标 i
        ↓
slice_date_window(data.rs:106) : bars.push(*b) 保留全集 source_index，未重置为局部下标
        ↓ oos.bars[0].source_index = 全集偏移（BTC=2,817,999 > bars.len）
parse_layer 链(inclusion:56/fractal/stroke/segment) : 信任 bar.source_index 字段（非数组下标）
        ↓ BspPoint.source_index(=segment.end_index) 携全集偏移
recognize_point → fill_bar_index(source_index, oos.bars) : 全集偏移索引 OOS 局部数组
        ↓ source_index ≥ bars.len ⟹ 越界返 None ⟹ 全决策被吞 ⟹ 零订单
```

### 3.5 为何是实现 bug 而非定义冲突（testing-override 判据）

- 修复 = `slice_date_window` 重置 source_index 为局部下标 `0..len`。
- 局部下标**满足** source_index 的定义（reference:16「`(timestamp, source_index)` 单序列内
  唯一单调平局裁决键」——**不要求**它是全集绝对位置）⟹ **不改任何定义的含义/边界**。
- 据 testing-override：「不改任何定义能修 → 实现 bug，正常修复」⟹ **(b)，不上浮，正常修复**。

### 3.6 修复归属与本工位边界

- 修复位置 = `data.rs::slice_date_window`（引擎层数据装配），**非本回测工位（runner.rs）owner**。
- 本工位**不跨文件改 data.rs、不在测试内打补丁伪造 L2**（工位边界 + 不伪造结果 + no-workaround）。
- **移交下游引擎层工位**：在 `slice_date_window` 内构造新 Dataset 时，重置每个保留 bar 的
  `source_index` 为切片后局部 enumerate 下标（dates 同步）。修复后重跑本测试，全 8 品种才可能产
  **干净** L2，届时方可判 Θ 策略经验有效性（盈利/不盈利）。

---

## 结果包六要素

1. **结论**：本次 L2 跑坐实了一个 (b) L1 实现 bug——`slice_date_window` 未重置 source_index 致
   全 8 品种坐标系断裂、fill 层吞决策。反事实对照（仅改 source_index→局部下标）使 7 品种
   decisions 由 0 转百万级，断言 `n_fixed_by_reindex==n_coordsys_mismatch==7` 坐实根因唯一。
   **OKLO 原 L2 标注作废**（污染）。bug 修复前 #5 无可信 L2 结果。

2. **定义依据**：source_index 定义（reference:16，`(timestamp,source_index)` 单序列唯一单调平局
   键，不要求全集绝对位置）；testing-override（不改定义能修=实现 bug 正常修复）；231 号 L 级
   （L2=真实数据+非空订单流，污染信号不满足）；625 铁律（引擎产不出信号=工程层 vs Θ 产信号不
   盈利=经验否证——本次是工程层 bug，引擎**产**信号被 bug 吞，非经验否证）。

3. **边界条件**（结论翻转）：
   - 若 `slice_date_window` 修复（重置 source_index）→ fill 不再越界 → 全品种产决策 → 此时
     strat/sharpe 才是干净 L2，**才**能判 Θ 策略盈利/不盈利。当前结论（bug 归因）不翻转，
     翻转的是「能否判策略有效性」（修复后才能）。
   - 反事实对照若某品种 reindex 后仍 decisions=0 → 根因非坐标系，需另查；实测 7/7 转非零，
     不触发。
   - 若 source_index 定义被改判为「必须全集绝对位置」→ 则降级为定义冲突需上浮；但 reference:16
     无此要求，不触发。

4. **下游推论**：acceptance #5「产可证伪结果 + 分层诊断」**部分达成**——分层诊断坐实了根因
   （高价值否定性诊断结果），但**未产可信 L2 回测结果**（全品种受 bug 污染）。下游 Θ 策略经验
   有效性评估（L2/L3）**阻塞**于 `slice_date_window` 修复。修复是明确的工程任务（非概念矛盾），
   移交引擎层工位后重跑即可解阻塞。

5. **谱系引用**：[[l2-engine-incompleteness-vs-theta-falsification]]（625：本次严格区分——引擎
   产信号被数据装配 bug 吞 ≠ Θ 产信号不盈利，故**不**报「策略不盈利」）；231 号
   formalization-validity-domain（L 级，污染信号不标 L2）；627（theta_v0 引擎线独立，未代偿）。
   source_index 坐标系 bug 是否有既存谱系**不确定**——建议 genealogist 核查 slice/source_index
   坐标系语义是否已有条目（疑似首次暴露，可能值得记一条「切片不重置局部坐标」的工程谱系）。

6. **影响声明**：
   - 改动文件：`rust/src/theta_v0/backtest/runner.rs`（增强 `l2_oos_eight_symbols`：+全品种坐标
     证据 +反事实对照坐实 a/b +根因坐实断言；删除原「定义层坐标系冲突/矛盾上浮候选」错误归因
     和「OKLO=干净 L2」结论——两者被反事实证据改写为「(b) 实现 bug + OKLO 污染」）。
   - 新建/重写文件：`docs/theta-v0-l2-results-v0.md`（本报告，核心结论改写）。
   - **未改动** classifier/strategy/parser/**data.rs** 实现（修复 data.rs::slice_date_window 移交
     引擎层工位，本工位不越界）。
   - 影响模块：goal #5 状态（诊断达成，L2 结果阻塞于 slice bug 修复）；**移交项**：
     `data.rs::slice_date_window` 重置 source_index。
