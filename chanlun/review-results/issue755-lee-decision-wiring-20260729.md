# #755 LEE 决策层接线（level_order/level_risk 资金权+帽入 sizing）留痕报告

裁定链：#693(②接口形状 kimi 为准)/#642(语义重放边界)/#310(M4 原语)/#351(四 MED 补课)。工位
`/private/tmp/wt-755`（分支 `ticket-755`）。参照面 `/private/tmp/kimi-nest-mainline`（只读）。

## 1. kimi 位点 → main 重放位点逐段论证

### 1.1 起点核对（先读现状，再动手）

- kimi 侧 M4 落地于 `7d8b45be70`（#310：`level_risk.rs` 新建 `level_weight`/
  `level_weights_sum_le_one` + `RiskConfig.{level_weights, enforce_level_cap}` + `coverage.rs::
  {level_cap, clamp_levels_to_weighted_cap}` + `fill.rs` 门控接线），`19aea33a26`（#351：四 MED
  补课——Σw 校验接线/帽后二次裁剪/稀疏性归属/前置断言强化）。**kimi commit message 自称
  「default 关，M0-M3 bit-exact 不变」**——票面预设的「kimi 侧无条件内建」与实际不符，已订正
  （见下方冲突清单条目 1）。
- main 侧起点：`ff9db6fbca`（#310 语义重放，#642 部分）已落地核心数学原语
  `coverage/sizing.rs::{level_cap, clamp_levels_to_weighted_cap}`（逐字等价 kimi），`level_risk.rs`
  已有 `level_weight`/`level_weights_sum`/`level_weights_sum_le_one`（三态测试齐全）；
  `config.rs::RiskConfig.{level_weights, enforce_level_cap}` 已在场（default 空表/false）。
  **唯一缺口**（`sizing.rs:263-286` 旧 doc 原话）：`clamp_levels_to_weighted_cap` 全仓只被自己的
  单元测试调用（`sizing_tests_2.rs:623/632`），生产路径（`fill.rs`/`level_order.rs`）**零次**调用
  ——这正是 #755 票面要补的「生产者接线」。

### 1.2 生产者接线位点（本票新增）

- **fill.rs**（`pi_theta_fill_loop_overlay`，`standard_p_star`/`order`/`step_trace` 刚绑定之后、
  `order` 首次被下游消费之前——已核实该区间内 `order` 无其他写入点，插入点安全）：
  - 门禁：`config.risk.enforce_level_cap`（复用既有字段，未新增配置面）。
  - 门开时序：① `debug_assert!(level_weights_sum_le_one(&config.risk))`（#351 MED「Σw 校验接线」，
    机器断言此前只是纯函数、未被任何调用点消费）；② `level_nets(&step_trace.sep_legs, lot)` 取
    结构基准（与下方既有 M2 只读诊断读数**同一函数同一 basis**，未引入第二套归因口径）；
    ③ `attribute_total(&basis, pre_cap_total)` 把账户层单一目标 `standard_p_star`
    （§16 唯一决策出口）按结构基准归因到各级；④ `clamp_levels_to_weighted_cap(&targets,
    base_units, &config.risk)`（**本票唯一新增生产调用点**）逐级二次裁剪；⑤ 裁剪后重新求和
    `capped_total`；⑥ `capped_total != pre_cap_total`（帽真实 binding）⟹ `order =
    schedule_order(capped_total as f64, p_t, exec_index)` 覆盖，否则 `order` 原样（含默认关闭时
    整段不构造）。
  - **coverage/mod.rs**：新增 `pub(crate) use sizing::{clamp_levels_to_weighted_cap, level_cap};`
    （此前二者只是模块内 `pub(crate) fn`，未跨模块导出到 `backtest::fill`），并移除
    `clamp_levels_to_weighted_cap` 上过时的 `#[allow(dead_code)]`，doc 同步订正「唯一填入者」
    落点（指向本票实际生产调用点，而非 `LevelOrderLedger::plan_gated`——见冲突清单条目 2）。

## 2. 冲突清单（含门控形状差 + 范围收窄，逐条论证保留原因）

1. **kimi commit message 自称「default 关」，与票面预设「kimi 侧无条件内建」不符**：核对
   `7d8b45be70` 全文（`RiskConfig.{level_weights: 空表, enforce_level_cap: false}` 均为 default），
   kimi 侧本身即带门控（非 main 补的）。**保留原因**：票面裁定②「门控形状本票硬约束」的**结果**
   （main 决策层默认关、golden cmp=0）与 kimi 一致，故本条不改变任何实施决策，只订正归因表述——
   门控不是本票"新引入的形状差"，是 kimi 侧既有形状的语义重放（字段/默认值原样保留）。
2. **未复用 `LevelOrderLedger`/`LevelOrderPlan::cap_narrowed_levels`（M2/M3 per-level 订单路由）**：
   `sizing.rs:263` 旧 doc 称「唯一填入者」应是 `plan_gated`（`level_order.rs:545-553`）的调用方
   ——但 `plan_gated` 依赖 `LevelOrderLedger.planned`（跨 bar 状态）与 `clock_ℓ` 事件门控
   （M3，`level_clock.rs`，目前仍**只读诊断**、未接 `regate`），若要经此路径落地帽，须**同时**
   把 M3 event-clock 从只读诊断升级为真实门控——这是比 M4 帽本身大得多的独立变更（改的是
   「何时重估」而非「重估多少」），且 `fill.rs:5200`/`runner.rs:412` 两处既有票面边界注释均
   将其单列注明「归 #755」但未细分 M3/M4 子范围。本票选择**账户层标量二次裁剪**（不引入
   per-level Δq_ℓ 订单路由，不接 M3 门控）：改动面从「新增一条完整决策分支+两处状态机」收窄到
   「四个纯函数调用+一次 debug_assert+一次 order 覆盖」，与票面裁定②「默认零变化+可验证」的
   验收要求相容，风险显著更低。**`LevelOrderPlan::cap_narrowed_levels` 字段消费链仍未被生产
   路径点亮**——如实登记，留作独立跟进（若后续要接 M3，建议单开票，不与本次账户层裁剪混改）。
3. **`RunResult`/`OverlayRunResult` 未新增字段承载「本次裁剪触发次数」等生产读数**：本票的
   BTC 20k 靶向对照（第 3 节）用现有 `n_orders`/`trade_pnls`/`equity_curve` 三项已足以证明生效，
   未额外扩 `RunResult` API 面——避免无请求授权下扩大生产结构体改动范围。

## 3. 三件验证读数

### ① `cargo test --lib`
基线：2605 passed / 0 failed / 139 ignored。
终态：2605 passed / 0 failed / **140** ignored（+1 = 本票新增 `issue755_level_cap_on_off_btc20k_diff`，
`#[ignore]`，需真实 BTC 数据）。**零新增红**。

### ② golden 护栏（门 off，默认配置）
门禁 `config.risk.enforce_level_cap` 默认 `false`（`RiskConfig::default()` 未改）；本票新增代码
段整体挂在 `if config.risk.enforce_level_cap { .. }` 内，门 off 时该分支**不构造不求值**——
`order`/`cash`/`units` 逐字节不变，等价于「未接入本票改动前」的既有生产路径。`cargo test --lib`
在默认配置下全量既有测试（含既有 golden/对拍用例）读数与基线**逐项相同**（2605/0/139→2605/0/140，
唯一差值是新增的 `#[ignore]` 测试项，不参与默认跑批），**cmp=0 成立**。

### ③ 门 on/off 靶向前后对照（BTC 前 20000 bar，`issue755_level_cap_on_off_btc20k_diff`）
`level_weights=[0.01,0.01,0.01,0.01,0.01,0.01]`（Σ=0.06≤1，刻意取紧配置，高概率 binding，
不依赖精确标定）：

| 口径 | 门 off（default） | 门 on | Δ |
|---|---|---|---|
| n_orders | 7082 | 274 | −6808 |
| trade_pnls 笔数 | 6814 | 218 | −6596 |
| trade_pnls 已实现总和 | −1,033,073.706880 | −13,676.224808 | +1,019,397.482072 |
| equity_curve 终值（归一化） | 0.757374 | 0.996781 | +0.239407 |

**口径自报**：本对照口径标签同 A10 附则B——费率/仓位参数未按票面标定，数值**禁作 alpha 论据**，
仅作「决策层是否真实改变生产读数」的接线证据。差异巨大且非零（n_orders 降 96%，equity 显著
改善）——证明帽在此紧配置下**大量 binding**、`order` 覆盖分支被真实走到，接线非空转 no-op。
测试内置断言 `r_on.n_orders != r_off.n_orders || Δpnl > 1e-9` 已固化为回归锁（未来若接线被
意外短路会立即变红）。

### ④ `cargo check --all-targets`
基线：0 error。终态：0 error（仅既有警告，无新增 error）。

## 4. 停手项
无——未触发"禁删"扫描命中（本票不涉及任何删除操作）；未发现需要上报的阻断。
