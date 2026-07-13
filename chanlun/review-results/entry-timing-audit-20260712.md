# 任务 #59：回测入场时刻审计（只读，未来函数检查）

日期：2026-07-12  
审计范围：`rust/src/theta_v0/backtest/runner.rs`、`econ_positive.rs`，以及 classifier 发布、strategy 消费、订单排程、成交、PnL 的必要上溯。  
审计约束：只报告，不修改 Rust 代码。

## 结论

**选择 (b)：存在违规点。**

存在三条已经由源码闭合的违规路径：

1. `run_theta_v0` 先做全窗分类，再把事件端点 `BspPoint.source_index` 当作信号时刻回放，可能在信号首次可用之前成交；仓库自带夹具明确给出 `source_index=3`、直到 bar 7 才确认的情形。
2. `econ_positive::decompose_capturable_spread` 虽按前缀首次确认 bar `i` 收集信号，却把该同一 bar 的 close 当作“实际入场价”，即 `entry_bar = i`，不满足 `entry_bar >= judge_at + 1`。
3. `l3_delta_r_alpha::build_walk_forward_mu` 采用同样的确认 bar close 入场口径，污染训练期 `mu(z)` 的收益标签；它随后被 `run_theta_v0_pi_chi` 的 test 窗准入使用。

因此不能声明“全链路满足入场 bar ≥ judge_at+1”。默认 `entry_delay_bars=1` 只保护了以正确可用时刻 `i` 调度的 `run_theta_v0_pi*` 成交链；它不能修复静态全窗回填链，也没有被类型或校验强制为至少 1。

## 1. 时刻定义与 classifier 端点

本文区分三个时刻：

- `s`：结构事件端点，即 `BspPoint.source_index`；按任务口径，三类点的 `judge_at` 取回试/判定走势的 `end_index`，故下文用 `j = s`。
- `a`：信号第一次出现在只含 `bars[..=i]` 的前缀分类中的 bar，即真正可用/发布时刻。
- `e`：订单实际改变持仓的成交 bar。

源码对 `s` 的构造如下：

- 线段端点投影把 `source_index` 直接设为 `Segment.end_index`：`rust/src/theta_v0/classifier/signal.rs:90-105`。
- 第一类使用破中枢 C 段终点 `end.source_index` 构造点：`signal.rs:250-300`。
- 第三类对 `(leave, retest)` 取 `retest = seg_end(retest_seg)`，再用 `retest.source_index` 构造点：`signal.rs:319-347`。
- 第二类取回拉走势 `m2` 的结束点，并经 `index_of(m2)` 写入 `source_index`：`signal.rs:427-456`；生产闭包最终返回该走势的 `end_index`：`classifier/mod.rs:1153-1161`、`classifier/recursive_tower.rs:441-457`。
- `BspPoint` 自身只保存“候选点在 L0 原始 K 序的位置”，没有“首次可用时刻”字段：`classifier/bsp.rs:100-114`。

关键事实是 `a` 不保证等于 `s`。`runner.rs` 明确说明买卖点会回溯确认，`source_index` 常早于首次进入前缀塔的 bar：`rust/src/theta_v0/backtest/runner.rs:424-458`。现有测试夹具进一步给出可执行反例：点的 `source_index=3`，但 `i<7` 时分类为空，`i>=7` 才出现该点：`runner.rs:1484-1513`；同一夹具被命名为 `buy1_at3_confirmed_at7`：`runner.rs:1524-1535`。`l3_pi_probe` 也直接统计 `lag = i - source_index` 并称其为“确认点落后信号点”：`rust/src/theta_v0/backtest/l3_pi_probe.rs:410-441`。

所以安全调度必须从 `a` 出发，而不能只从 `s` 出发。必要条件应写为 `e >= a + 1`；因 `a >= j`，这也蕴含任务要求的 `e >= j + 1`。

## 2. 完整链路

### 2.1 静态 `run_theta_v0`：违规的全窗回填链

链路如下：

1. **产生**：整段 `bars` 一次性解析和分类，得到包含全窗信息的 `Classification`：`rust/src/theta_v0/backtest/runner.rs:130-141`。
2. **时间分组**：`strategy::recognize` 按候选的 `source_index` 分 moment，而不是按首次可用 bar：`rust/src/theta_v0/strategy/mod.rs:445-488`。
3. **时间戳透传**：`build_decision` 从 `fill_bar_index(point.source_index, ...)` 取目标 bar，并把 `signal_index` 写成 `point.source_index`：`strategy/mod.rs:512-562`。
4. **订单调度**：开仓订单再次用 `fill_bar_index(d.signal_index, ...)` 得到 `exec_index`：`strategy/mod.rs:320-380`。
5. **回测消费**：`plan_and_fill_mtm` 预先把全部 decisions 按上述 `exec_index` 分到历史 bar：`runner.rs:882-908`。
6. **成交**：循环到该历史 bar 时用该 bar 的 close 调 `apply_order`，并把该 bar 记为 `TradeRecord.entry_bar`：`runner.rs:931-1023`、`runner.rs:1128-1164`。
7. **PnL**：`apply_fill` 用成交 close 和合并费用率建立 `entry_cost`；平仓时从该成本基开始计已实现 PnL：`runner.rs:1335-1402`。

违规机制：步骤 1 在时间 0 已经知道全窗信号，步骤 2-5 却把它们回填到事件端点 `s`。在仓库已有 `s=3, a=7` 夹具下，默认 delay=1 会产生 `e=4`，而因果链应为 `e>=8`。这不是单纯的标签差异，而是持仓、权益、交易轨迹和全部下游指标都从过早 bar 开始。

代码自身也承认全窗分类不适合执行：`run_theta_v0_pi_inner` 的注释明确写“全窗 classify（非因果，bsp/父走势可能用 >i 数据确认）执行层禁用”：`runner.rs:337-340`。

影响面：所有 `run_theta_v0` 调用结果。除 runner 内多个 L2/L3/真实数据测试外，`rust/src/theta_v0/backtest/l3_fullwindow.rs:100-134` 直接用它生成全窗 PnL、bootstrap 和随机对照，因此这些结果都继承该回填偏差。主要 runner 调用点为 `runner.rs:2304-2308`、`2400-2407`、`2805-2818`、`3055-3066`。

### 2.2 增量 `run_theta_v0_pi*`：默认配置下时序合格

链路如下：

1. **产生/可用**：`IncrementalClassifier::classify_at(i)` 只用 `<=i` 数据：`runner.rs:325-365`。
2. **发布**：`newly_confirmed_step` 用 append-only seen-set 只保留 bar `i` 首次出现的点：`runner.rs:424-458`。
3. **调度**：成交索引用当前可用 bar `i`，即 `fill_bar_index(i, ...)`，而不是点的 `source_index`：`runner.rs:663-675`。
4. **挂单**：订单进入 `pending[exec_index]`：`runner.rs:701-720`。
5. **成交**：到达该 bar 时，以该 bar close 调 `apply_order` 并记录 entry bar：`runner.rs:640-652`。

默认 `entry_delay_bars=1` 见 `rust/src/theta_v0/config.rs:169-190`；`fill_bar_index` 计算 `signal_index + entry_delay_bars` 并向后跳过不可交易 bar：`rust/src/theta_v0/strategy/exec.rs:32-52`。故默认路径满足 `e >= a+1 >= j+1`。

边界：`ExecConfig.entry_delay_bars` 是公开的 `u32`，没有 `>=1` 校验。对静态 `run_theta_v0` 设为 0 会直接允许 `e=s=j`，违反任务不变量。对 `pi_theta_fill_loop` 设为 0 时，挂单发生在本 bar 的成交阶段之后（先处理 `pending[i]`，后计算/写入 `pending[i]`），因此订单被留在已错过的槽位而不成交；这不会制造同 bar fill，但会丢单。两者共同说明“不早于下一 bar”只是默认值，不是全链硬不变量。

### 2.3 `econ_positive`：确认 bar close 回填为“实际成交”

链路如下：

1. `IncrementalClassifier::classify_at(i)` 逐 bar 因果分类，seen-set 找本 bar 新确认点：`rust/src/theta_v0/backtest/econ_positive.rs:214-273`。
2. 信号元组直接写入 `entry_bar=i`：`econ_positive.rs:219-223`、`273`。
3. 下一反向新确认信号给出 `exit_bar`：`econ_positive.rs:285-325`。
4. 入场“实际价”取 `bars[entry_bar].close`，出场同理取确认 bar close：`econ_positive.rs:328-346`。
5. `actual_spread`、双边成本和 `actual_pnl` 都从这两个同-bar close 计算：`econ_positive.rs:338-354`。

违规机制：该路径知道 bar `i` 收盘后才完成 `classify_at(i)`，却用同一个 `close[i]` 成交。按任务硬约束必须从 `i+1` 或其后可交易 bar 成交。这里没有调用 `fill_bar_index`，也没有使用 `entry_delay_bars`，所以默认执行配置无法保护它。

影响面：`decompose_capturable_spread` 的所有 `SignalDecomp.actual_spread`、`actual_pnl`、`captured`、累计曲线、分层 `mu`、train/holdout 与 OOS 报告。调用点集中在 `econ_positive.rs:985`、`1298-1314`、`1452-1495`、`1837-1861`、`2237`。

### 2.4 `l3_delta_r_alpha` 的训练收益标签：同类违规

`build_walk_forward_mu` 先按前缀 `classify_at(i)` 和 seen-set 收集首次确认点，保存 `(i,z,dir)`：`rust/src/theta_v0/backtest/l3_delta_r_alpha.rs:109-175`；随后以 `bars[entry_bar].close` 作为 entry，以下一反向确认 bar close 为 exit，并构造 `marginal_return`：`l3_delta_r_alpha.rs:177-213`。

这同样是 `entry_bar=a` 而非 `a+1`。它不会偷看 test 窗，但 train 内每个 `mu(z)` 标签都包含不可实现的确认-bar close 入场。该估计表在 `l3_delta_r_alpha.rs:373-395` 被 frozen 后用于 test 窗的 `run_theta_v0_pi_chi` 准入，因此即使 test 的执行链本身使用安全的 `pi` 调度，选择器输入仍被同-bar成交口径污染。

## 3. 逐条违规清单

| ID | 文件:行 | 机制 | 影响面 |
|---|---|---|---|
| V1 | `runner.rs:130-151`; `strategy/mod.rs:445-562`; `runner.rs:882-1023` | 全窗分类后按 `BspPoint.source_index` 回放；`s<a` 时在首次可用前成交 | `run_theta_v0` 的订单、持仓、权益、PnL、TradeRecord、metrics、L2/L3/fullwindow 结果 |
| V2 | `econ_positive.rs:214-273,328-354` | 本 bar 首次确认后，用同 bar close 作“实际入场” | 全部 `SignalDecomp`、capturable-spread、分层 mu、累计与 OOS 结论 |
| V3 | `l3_delta_r_alpha.rs:109-213` | 本 bar 首次确认后，用同 bar close 构造 train 收益标签 | frozen `MuEstimator` 及后续 chi 准入/Delta-R 结论 |
| V4 | `config.rs:169-190`; `strategy/exec.rs:32-52` | `entry_delay_bars` 可设 0 且无下限校验；静态链可 `e=j` | 所有接受自定义 `ThetaConfig` 的静态回测调用 |
| V5 | `recursive_tower.rs:441-457` | 第二类坐标按 `RMove` 值相等取首个 sidecar；源码承认重复结构可有不同坐标，可能把后一个 `m2` 映到更早 `end_index` | 第二类静态回放时刻、pivot 取价及依赖 source_index 的分析；这是未被不变量排除的额外回填风险 |

V1-V4 是直接可达的代码路径；V5 是源码明确承认的坐标歧义，本次只报告机制，未声称已在真实数据中统计到发生次数。

## 4. 成交价、滑点与 PnL 起点

### 4.1 实际 runner 成交价

- `strategy::build_decision` 读取目标 fill bar 的 **open** 写入 `VoiceDecision.entry`：`strategy/mod.rs:523-525`。该值进入 sizing 的 `SizingInput.entry`：`strategy/mod.rs:352-366`。
- 但 `plan_and_fill_mtm` 和 `pi_theta_fill_loop` 真正改变现金/持仓时都使用目标 fill bar 的 **close**：`runner.rs:931-954,975-1005` 与 `640-650`。
- 因此“下一根 open 成交”的模块文档与真实 runner 账本不一致。open 只影响仓位 sizing，PnL 成本基从 fill bar close 开始。
- `TradeRecord.entry_bar` 在持仓从 0 变为非 0 的实际 fill bar 写入：`runner.rs:1128-1164`。已实现 PnL 从 `apply_fill` 保存的含费 `entry_cost` 起算：`runner.rs:1351-1402`。终点未平仓按最后 bar close 强平：`runner.rs:1069-1098`（`pi` 对应 `749-769`）。

### 4.2 滑点/费用

- runner 把 `commission_bps + slippage_bps + tax_bps` 合并为一个 `fee_rate`：`runner.rs:892-895`、`609-610`。
- `apply_fill` 对 close 价按方向乘 `1±fee_rate` 建成本基/退出净价：`runner.rs:1355-1399`。
- `strategy::exec::apply_fees` 的整数 tick 舍入实现只在其自身测试被调用；生产 runner 没有调用它。全仓库调用点只有定义与测试：`strategy/exec.rs:73-82,337-340`。
- 所以当前 runner 的“滑点”不是从 open 到实际成交的独立价格冲击模型，而是附着在 fill bar close 上的对称 bp 成本项。

### 4.3 止损成交

`stop_fill_price` 能返回跳空 open 或 stop 价：`strategy/exec.rs:85-123`，但 runner 只通过 `stop_hit(...)=stop_fill_price(...).is_some()` 检测触发：`runner.rs:497-525`。随后实际 Close 仍被延迟并按目标 bar close 进入 `apply_order`：`runner.rs:1027-1059`、`935-954`。因此 stop/open 价没有进入 runner PnL。

### 4.4 是否用 retest 低点回填成交价

没有发现 runner 把 retest 低点/高点直接当成交价：

- 第三类的 `retest.price` 只进入 `pivot_low/pivot_high`，中枢进入结构止损：`classifier/signal.rs:369-380`。
- 第一/二类 pivot 也只作为 `StopInput`/结构止损与 sizing 风险距离的输入：`strategy/mod.rs:527-558,320-366`。
- runner 真成交仍是 fill bar close。
- `econ_positive` 会用 `source_index` 对应 close 作为理想 pivot 价 `P[lambda_rev]`/`P[rho_rev]`，但其“实际价”字段用确认 bar close：`econ_positive.rs:328-348`。所以没有“用 retest 低点回填 actual fill”的字面路径；存在的是“理想 pivot 对照 + 同确认 bar close 伪实际成交”。

## 5. 验证记录与限制

- 代码发现原计划使用 codebase-memory-mcp；项目枚举和索引调用均被当前环境取消，故按仓库指令降级为 `rg` + 精确行号读取。
- 按 `codex-review` 流程尝试了独立只读复核，但 `codex exec -s read-only` 因沙箱无法初始化 app-server（`Operation not permitted`），未产生可用复核意见。
- 尝试运行现有 `fill_delay_one_bar` 单测；crate 在编译阶段因基线缺少 `rust/src/theta_v0/classifier/cand_predicate.rs` 报 `E0583`，未进入测试。该失败与本次报告写入无关；本次结论依据静态源码、现有测试夹具和调用链闭合。
- 未修改任何 `.rs` 文件。

## 最终一行

**结论 (b)：静态 `run_theta_v0` 存在全窗信号时间戳回填，`econ_positive` 与 `build_walk_forward_mu` 存在确认 bar 同 close 入场；仅默认配置下的增量 `run_theta_v0_pi*` 满足入场 bar ≥ 可用时刻+1。**
