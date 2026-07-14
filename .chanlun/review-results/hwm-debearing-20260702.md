# GAP3 桥 hwm_gain 棘轮承重移除（codex R3 C' 终局裁定落地）

- 工位: ws-hwmfix（task #34）| 分支: gap3-rework-codex9-fix | 日期: 2026-07-02
- 裁决依据: `.chanlun/review-results/codex-r3-ruling-20260702.md`（codex 终局裁定 C'）
- 认识论等级: L0/L1 代码改动（结构镜像/管线正确性）+ L2 真实 BTC 实证（FALSIFIED 确认）

## 方向选择推导链（三候选中取 (a)，非最省改动 workaround）

codex 给出三个允许方向：(a) hwm_gain 保留为纯诊断、从承重链路移除；(b) 改「已实现利润」入账；
(c) 可正可负 MTM 账本 + 权益/可退现金分离。**选 (a)**，推导链：

1. codex 对**强制部分**的首选即 (a)——「可保留为诊断字段，但不得驱动 RecoverCapital/EnterEarning」；
   (b)/(c) 是「若继续做价格桥」的**未来改造方向**，codex 明示「若未来改造……可重新提交裁决」。
2. (b) 需要「已实现利润事件流」，当前架构**缺失**：schedule_adapter 的卖出（filled_delta<0）在 TW 侧
   走 `ShortDiff(-cost_flow)` 且 `cost_flow=Δ·avg_cost`（成本基），现价浮盈**只**由 Revalue 入账——移除
   Revalue 承重后，无任何路径让已实现利润（现价−成本）进 TW.free。构建 (b) 需重做 schedule 卖出语义，
   属新架构（见 memory「GAP3 L2也不可达=架构非数据」）+ 须重新提交裁决。
3. (c) 需可正可负 MTM 账本重构（权益/可退现金分离），同属新架构 + 须重新提交裁决。
4. 现在抢建 (b)/(c) 以重开可达性 = 声明膨胀（宣称架构无法诚实支撑的 L2 可达性），违反
   formalization-validity-domain 与 codex「GAP3 可达性应保持 FALSIFIED 直到有非回补资金源语义」。
5. 故 **(a)+诚实声明 FALSIFIED 是唯一不膨胀选项**——且 codex §5.4 明示保留 Revalue 构造子本体 +
   价格幅度打通管线（不全盘推倒），(a) 恰匹配。

## 结果包六要素

### 1. 结论

`hwm_gain` 高水位棘轮对 `free`/`cum_net_cash`/`stage_progression` 的承重已移除，降为**纯诊断字段**：

- `ledger.rs` `TwEvent::Revalue(g)` arm：从 `{free += g, cum_net_cash += g, hwm_gain += g}` 改为
  **只 `hwm_gain += g`**——Revalue 现为诊断-only，**保 TW 守恒**（TW 三量不变）。承重在**共享函数
  层**斩断（no-patch/ponytail 根因修复：任何调用方都无法经 Revalue 入账 free）。
- `transition.rs` 重估步：仍计算 `unrealized = positions·price − holding` 并以 `Revalue(hwm_delta)`
  推进诊断高水位（价格管线保留，§5.4），但因 Revalue 不再入账 free，`stage_progression` 只依赖真实
  卖出现金回流的 sound free ⟹ L0 同价与 L2 变价下浮盈均不入 free ⟹ EarningShares 结构不可达。
- GAP3 acceptance 判据 `∃t, TStage(t)=EarningShares` **恢复 FALSIFIED**——L2 真实 BTC（500K bar 截尾）
  实测：`final_stage=CostReduction, count=0, TW=notional_in（守恒）, free=0, hwm_gain=2.17e12（诊断，零承重）`。

### 2. 定义依据

- PDF《New-Chan 不变量映射》p8③「Ledger 在条件被否定时强制冻结解释权，不允许用『级别、调整、延续』
  进行语义回补」——旧 hwm 棘轮（回撤不撤销、把未实现浮盈峰值当可分配权益驱动退本金）= 语义回补。
- 缠师第31课三阶段（降成本/退本金/增股数）单向不可逆：`RecoverCapital` 的 sound 资金源须真实存在
  （L0 证明 `TW守恒 ∧ holding≥notional_in ⟹ free≤0`，退本金前提与 cash-tight 互斥）——诊断浮盈非资金源。
- codex R3 终局裁定 C'（`codex-r3-ruling-20260702.md` §3-5）：非法的是「只记正向峰值+不撤销+驱动
  stage_progression」这个组合，非 Revalue 构造子本身（§5.4 保留构造子 + 价格管线）。

### 3. 边界条件（结论翻转条件）

- 若未来重装 (b)「已实现利润」入账（卖出现价−成本进 free）或 (c) 可正可负 MTM 账本（权益/可退现金
  分离），且**不驱动棘轮式承重**，可重新提交 codex 裁决——届时 EarningShares 可能重新可达（非回补
  资金源语义成立）。当前架构缺此事件流，故 FALSIFIED。
- 若 schedule_adapter 卖出语义改为按现价移出 holding（realized gain 进 free），则 stage_progression
  可能因真实卖出现金推进——但那是 (b) 路径，须先裁决。
- 诊断口径边界：`hwm_gain` 仍如实追踪未实现浮盈峰值（L2 BTC 达 2.17e12），但**零承重**——它是
  observability 量，不是盈利/可达性声明。

### 4. 下游推论

- `gap3-bridge-20260702.md` 声称的「L2 字面 PASS / count=1」不再成立（task #35 已订正措辞）。
- task #10 GAP3 补桥「部分作废」：Revalue 构造子 + 价格幅度打通管线保留，stage_progression 承重链路
  已重做为诊断-only（裁决 §5.4）。
- 依赖 Revalue 驱动阶段推进的 5+1 个 closed_loop 测试（runner.rs）断言已按 FALSIFIED/诊断口径改写，
  测试名与语义对齐（`price_magnitude_drives_diagnostic_hwm_not_tw_closed_loop`、
  `l2_btc_earning_shares_unreachable_hwm_debearing`），**未删测试消红**。
- `pdf-conformance-audit-20260702.md` R3 段应从「偏离（潜在）」转「偏离（确认）」——已由 task #35/#36 覆盖。

### 5. 谱系引用

- 231号（形式化有效域）：hwm_gain 声明膨胀（把 L0/L1 诊断口径当 L2 可达性承重）已纠正。
- 090号（严格性语法）：候选 A「仅降级措辞」被 codex 裁定不足以豁免代码违规——本工位做代码层移除。
- 576号（R vs TW 账本边界）：本改动**未破坏正交性**——`LedgerComp`（R=Π-A-W）与 `TwState`（TW）双层
  并置不变，`forget_stage_to_ledger_view` 单向有损投影不动（`projection_forgets_stage_non_injective`
  测试仍绿）。Revalue 只改 TW 账本内 hwm_gain 诊断字段，不触 R 账本。
- 161号/no-workaround/no-patch-mentality：(a)+诚实 FALSIFIED 非补丁——是承重的诚实移除，恢复真实
  认识论状态（合法账本语义下无非回补资金源）。
- 谱系记录条目（task #36 覆盖）：「hwm_gain 高水位棘轮被裁定为语义回补，不能作为 GAP3 acceptance bridge」。

### 6. 影响声明

- **改动文件（3 个，均在授权范围，不碰 WIP 的 econ_positive/classifier/incremental/recursive_tower）**：
  - `rust/src/theta_v0/strategy/ledger.rs`：Revalue arm（承重移除）+ hwm_gain 字段/tw()/枚举文档 +
    测试 `tw_step_revalue_adds_gain`→`tw_step_revalue_diagnostic_only`（重写+改名）+ `tw_step_preserves_tw`
    事件表加 Revalue（全七守恒）。
  - `rust/src/theta_v0/closed_loop/transition.rs`：重估步（诊断-only）+ stage_progression 注释 + T 段文档
    + AssemblyEvent.price 文档。
  - `rust/src/theta_v0/backtest/runner.rs`：6 个测试断言改写为 FALSIFIED/诊断口径 + 2 个改名。
- **验收**：`cargo test --lib` — ledger 19/19、closed_loop 60/60、runner 30/30（+6 ignored）全绿；
  l2_btc（#[ignore]）500K bar 实测 count=0 FALSIFIED 通过。576 正交性不破坏。
- **零 git 操作**（按约束）；econ_positive.rs/conformance.rs/l3_delta_r_alpha.rs 无 hwm_gain 引用，未触碰。
