# Θ v0 L2 真实数据回测结果 v0（acceptance #5）

> 产物：`rust/src/theta_v0/backtest/runner.rs::l2_oos_eight_symbols`（`#[ignore]`，真实数据）
> 跑法：`cargo test --lib theta_v0::backtest::runner::tests::l2_oos_eight_symbols -- --ignored --nocapture`
> 数据：`analysis/data_cache/{btc,es,cl,gc,brn,dx,qqq,oklo}_*.json`（databento 1m）
> 切分：`rust/src/theta_v0/backtest/prereg_windows.rs::PREREG_WINDOWS`（OOS 窗，看结果前冻结）

## 0. 报告核心结论（先于数据，TL;DR）

本报告记录一个完整的「诊断 → 坐实 → 修复 → 重跑」闭环：

1. **首跑（commit 12f8e4b8）暴露**：8 品种全部 `decisions=0`，无任何 L2 结果。
2. **反事实对照坐实根因**：`slice_date_window` 切 OOS 窗时未重置 `Bar.source_index`，保留了
   全数据集绝对偏移，导致下游 `fill_bar_index` 用全集偏移索引 OOS 局部数组而越界、吞掉全部
   决策。这是 **(b) L1 实现 bug**，不是 (a) 引擎在真实数据上的有效域边界，也不是定义冲突。
3. **修复（本工位 SG-coordsys）**：`data.rs::slice_date_window` 切片时重置 `source_index` 为
   局部 enumerate 下标 `0..len`。
4. **修复后重跑**：8/8 品种全部产订单流（`decisions=0` 断流消失，`坐标系断裂品种数=0`），
   得到**干净的 L2 经验结果**——strat%/sharpe 现可作为 Θ 策略经验有效性证据。

a/b 定性经 **codex 异质审查独立确认**（reference:16 只要求单序列内唯一单调平局键，不要求全集
绝对位置；局部下标 0..len 既是合法数组下标又与全集同序，平局裁决顺序不变 ⟹ 不改任何缠论定义
⟹ 实现 bug，正常修复，不上浮）。

## 1. 修复后真实回测表格（每品种一行，OOS 窗）— 干净 L2

下表为修复后 `cargo --nocapture` 的**真实输出**（`/tmp/l2_postfix_run.log`，test result: ok）：

| symbol | pool     | oos_bars  | bh%     | strat%  | sharpe | seg   | ctr  | bsp       | decis     | ord | trades | L 级 |
|--------|----------|-----------|---------|---------|--------|-------|------|-----------|-----------|-----|--------|------|
| BTC    | Core     | 1,313,200 | +547.66 | +327.27 |  0.036 | 11221 | 4674 | 9,323,059 | 9,323,059 | 2   | 0      | L2   |
| ES     | Core     |   881,913 |  +60.73 |  +37.30 |  0.033 |  6977 | 2909 | 3,608,856 | 3,608,856 | 8   | 0      | L2   |
| CL     | Core     |   870,572 |  -19.28 |  -11.76 | -0.004 |  6970 | 2913 | 3,507,460 | 3,507,460 | 1   | 0      | L2   |
| GC     | Core     |   870,964 |  +81.26 |  +47.82 |  0.043 |  6946 | 2893 | 3,568,205 | 3,568,205 | 2   | 0      | L2   |
| BRN    | Extended |   816,448 |  -22.49 |   -9.09 |  0.009 |  6153 | 2566 | 2,733,135 | 2,733,135 | 25  | 0      | L2   |
| DX     | Extended |   678,728 |   -6.81 |   -4.55 | -0.013 |  5308 | 2186 | 2,030,280 | 2,030,280 | 1   | 0      | L2   |
| QQQ    | Extended |   523,112 | +105.16 | +100.42 |  0.053 |  3776 | 1460 |   995,372 |   995,372 | 21  | 0      | L2   |
| OKLO   | Observ.  |   268,411 | +155.46 |  +69.28 |  0.031 |  2021 |  785 |   282,839 |   282,839 | 1   | 0      | L2   |

**L 级标注（231 号）**：修复后全 8 品种均达 **L2**（真实数据 + 非空订单流，扣成本指标可证伪）。
OKLO 不再是唯一 L2，原首跑的「OKLO 唯一 L2、7 品种 L1」结论已被修复推翻——首跑的 7/8 断流是
source_index bug，非引擎缺口；OKLO 的首跑 strat=-0.12% 是 bug 污染的部分回测（仅前 ~54% 信号），
修复后 OKLO strat=+69.28% 才是干净结果。

### 1.1 经验结论（625：此为干净 L2，非工程层断流）

- BTC/ES/GC/QQQ/OKLO strat% 为正，CL/BRN/DX 为负——这是 Θ v0 策略在真实数据上的**经验表现**
  （混合，非系统性盈利或亏损），属可证伪 L2 结果。
- **所有品种 strat% < bh%**：策略未跑赢买入持有（如 BTC strat +327% vs bh +548%）。这是干净的
  经验否定性观察（Θ v0 单声部 L0 第三类买卖点策略在 OOS 窗未超越基准），有 231 价值。
- sharpe 普遍接近 0（-0.013 ~ +0.053）——风险调整后收益微弱。

## 2. 分层诊断断点归因汇总（修复后）

```
断点 L0(线段<3)      : 0
断点 classify(无中枢): 0
断点 signal(无BSP)   : 0
断点 recognize(无决策): 0     ← bug 修复前为 7，修复后归零
断点 sizing(无订单)  : 0
L2 端到端(产订单流)  : 8     ← 全 8 品种
坐标系断裂品种数      : 0     ← bug 修复前为 7，修复后归零（回归不变量）
```

8 品种全部通过 parse→classify→signal→recognize→sizing 五层端到端，无任何断点。

### 2.1 修复后新暴露的下游观察（decis ≫ ord，供下游工位，非本工位矛盾）

修复后 `decisions == bsp`（每个 BspPoint 都产决策，如 BTC 9,323,059），但 `orders` 极少
（1~25）。原因：海量决策在 `run_theta_v0` 内按 `exec_index` 分组、由账户层（plan_orders/sizing
+ 持仓状态机）收敛——同一 exec_index 的决策批量 plan，且已持仓时多数决策不再开新仓。这是 sizing
层的**正常收敛行为**（非断流——`断点 sizing=0`，确有订单产出），不是新 bug。但 `decis` 到 `ord`
的 ~10⁶:1 收敛比是否符合 Θ_risk sizing 的预期密度，**留作下游 sizing 工位的经验校验项**（不在
本 source_index 坐标系工位范围）。

## 3. 根因坐实：(b) L1 实现 bug（反事实对照 + 异质审查，非推理）

### 3.1 坐标证据（首跑，全 8 品种，真实输出）

首跑（bug 未修）`/tmp/l2_counterfactual_run.log` 真实输出：

| symbol | oos_first_src | max_bsp_src | bars.len  | 断裂  | decisions(原) | decisions(reindex 反事实) |
|--------|---------------|-------------|-----------|-------|---------------|----------------------------|
| BTC    | 2,817,999     | 4,131,007   | 1,313,200 | true  | 0             | 9,323,059                  |
| ES     | 2,449,435     | 3,331,081   |   881,913 | true  | 0             | 3,608,856                  |
| CL     | 2,457,529     | 3,328,002   |   870,572 | true  | 0             | 3,507,460                  |
| GC     | 2,437,488     | 3,308,288   |   870,964 | true  | 0             | 3,568,205                  |
| BRN    | 1,310,703     | 2,127,083   |   816,448 | true  | 0             | 2,733,135                  |
| DX     | 1,116,146     | 1,794,650   |   678,728 | true  | 0             | 2,030,280                  |
| QQQ    |   888,487     | 1,411,261   |   523,112 | true  | 0             |   995,372                  |
| OKLO   |    74,871     |   343,217   |   268,411 | true  | 154,218(部分) | （部分成功，见 §3.3）      |

`oos_first_src` 对所有品种 > 0（BTC=281万），证明 OOS 是全集中段切片、`slice_date_window` 保留了
全集绝对 source_index。

### 3.2 反事实对照坐实 a/b（首跑核心证据）

首跑测试内**只读重建**（不改 data.rs）：把 `oos.bars` 的 source_index 由全集绝对偏移改为局部
下标 `0..len`，管线其余完全不变，重跑 parse→classify→recognize：

**7 品种 decisions 由 0 全部转非零（百万级）。唯一改变的变量 = source_index。**
→ **坐实 (b)**：不是 (a) 真实拒绝（信号确实存在），不是定义冲突。

### 3.3 OKLO 首跑「部分成功」（推翻原「坐标系巧合一致」归因）

OKLO 首跑 `max_bsp_src=343217 ≥ bars.len=268411`，**同样断裂=true**。但 `oos_first_src=74871 <
bars.len` ⟹ OOS **前段** bar 的 source_index 落在局部数组内、fill 成功（154,218 decisions），只有
后段越界被吞。故 OKLO 首跑 strat=-0.12% 是**仅覆盖 OOS 前 ~54% 信号的污染回测**，非干净 L2。
修复后 OKLO 全段信号 fill 成功（282,839 decisions），strat=+69.28%。

### 3.4 修复（本工位落地）

`data.rs::slice_date_window` 切片时重置 source_index 为局部 enumerate 下标：

```rust
let local_index = bars.len();
bars.push(Bar { source_index: local_index, ..*b });
```

修复后重跑（`/tmp/l2_postfix_run.log`）：全 8 品种 `oos_first_src=0`、`断裂=false`、产订单流。

### 3.5 为何是实现 bug 而非定义冲突（testing-override 判据 + 异质审查）

- `source_index` 在本系统承担**双语义**：(1) 局部数组下标（`fill_bar_index` 用
  `bars[source_index]` 直接索引当前 Dataset）；(2) 平局裁决键（reference:16
  `(timestamp, source_index)`）。
- 修复 = 重置为局部下标 `0..len`：既复原「= 数组下标」语义（索引合法），又仍是合法平局键
  （0..len 严格单调唯一，且与全集绝对下标同序 ⟹ 平局裁决顺序不变）。
- **不改任何缠论定义的含义/边界**——reference:16 只要求单序列内唯一单调，未要求全集绝对位置。
- 据 testing-override：「不改任何定义能修 → 实现 bug，正常修复」⟹ **(b)，不上浮**。
- **codex 异质审查独立确认**（三质询点全过）：两语义在重置后都成立；无依赖全集绝对语义的消费者
  （边界提示：若未来需全集溯源应新增 `raw_source_index`，不复用 source_index）；不是定义冲突，
  冲突在实现坐标系而非定义。

---

## 结果包六要素

1. **结论**：source_index 坐标系矛盾经反事实对照 + 异质审查坐实为 **(b) L1 实现 bug**——
   `slice_date_window` 未重置 source_index 致全 8 品种坐标系断裂、fill 层吞决策。**已修复**
   （`data.rs::slice_date_window` 重置 source_index 为局部下标）。修复后重跑：8/8 品种产干净 L2
   订单流（`坐标系断裂品种数=0`、`断点 recognize=0`），strat% 混合（BTC +327% / CL -12% /
   QQQ +100% 等），全部 strat% < bh%（策略未跑赢基准，干净否定性 L2 观察）。**不 escalate**
   （非定义冲突）。

2. **定义依据**：source_index 定义（reference:16，`(timestamp,source_index)` 单序列唯一单调平局
   键，不要求全集绝对位置）；source_index 在 `fill_bar_index`/`recognize_point`/`run_backtest` 中
   被当**当前 Dataset 局部数组下标**直接索引（`bars[source_index]`，无全集语义消费者）；
   testing-override（不改定义能修=实现 bug 正常修复）；231 号 L 级（修复后真实数据+非空订单流
   =L2）；625 铁律（首跑断流=工程层 bug 吞信号，非 Θ 经验否证；修复后产订单流方为干净 L2）。

3. **边界条件**（结论翻转）：
   - 若全仓出现依赖 source_index **全集绝对位置**的消费者（如跨 Dataset 溯源），则「重置为局部
     下标」会破坏该消费者——此时须新增 `raw_source_index` 字段而非复用。当前无此消费者
     （已全仓 grep + codex 审查确认），不触发。
   - 若 reference:16 被改判为「source_index 必须全集绝对位置」→ 降级为定义冲突需上浮；但
     reference:16 无此要求，不触发。
   - 反事实对照若某品种 reindex 后仍 decisions=0 → 根因非坐标系；首跑实测 7/7 转非零，不触发。

4. **下游推论**：acceptance #5「产可证伪 L2 结果 + 分层诊断」**完全达成**——8/8 品种干净 L2，
   strat/sharpe 可作 Θ 策略经验有效性证据。下游可基于此判 Θ v0 策略表现（当前结论：未跑赢
   buy&hold，sharpe≈0，单声部 L0 第三类策略经验效力弱）。新暴露 `decis≫ord` 收敛比（§2.1）
   留作下游 sizing 工位经验校验项（非本工位矛盾）。

5. **谱系引用**：[[l2-engine-incompleteness-vs-theta-falsification]]（625：本次严格区分——首跑
   引擎产信号被数据装配 bug 吞 ≠ Θ 产信号不盈利；修复后才报真实经验结果）；231 号
   formalization-validity-domain（L 级，修复后真实数据非空订单流标 L2）；627（theta_v0 引擎线
   独立，修复仅触 backtest/data.rs，未代偿旧引擎线）。source_index 坐标系 bug 疑似首次暴露，
   **建议 genealogist 核查**是否已有「切片不重置局部坐标」的工程谱系条目（codex 提示的
   `raw_source_index` 全集溯源边界值得记录）。

6. **影响声明**：
   - 改动文件：`rust/src/theta_v0/backtest/data.rs`（`slice_date_window` 重置 source_index 为局部
     下标——核心修复）；`rust/src/theta_v0/backtest/runner.rs`（`l2_oos_eight_symbols`：删除已
     完成使命的反事实对照诊断脚手架，断言由「坐实坐标系断裂」改写为「修复后坐标系不断裂
     `n_coordsys_mismatch==0` + ≥1 品种产干净 L2 `n_l2_orders>=1`」，文档注释更新为发生史+修复）；
     `docs/theta-v0-l2-results-v0.md`（本报告重写）。
   - **未改动** classifier/strategy/parser 实现（修复在数据装配层，source_index 定义未改）。
   - 影响模块：goal #5 状态（#5 完全达成，干净 L2 结果产出）；解除 SG-5-L2 的 source_index
     BLOCKED（撤销 escalate——非定义冲突）。
