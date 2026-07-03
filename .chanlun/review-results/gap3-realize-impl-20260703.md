# GAP3 Realize 实装结果包：TwEvent::Realize 已实现利润入账（codex 裁定 A' 九条清单落地）

- 工位: ws-realize（Task #140）| 分支: gap3-rework-codex9-fix | 日期: 2026-07-03
- 规格源（唯一权威）: `.chanlun/review-results/codex-gap3-ledger-20260703.md` §3（codex 终局裁定 A'
  推导链 11 条 + 两条硬边界 + 九条可执行改动清单）
- commit 序列（分批，每批 `cargo test --lib` 全绿）:
  - `f3e3709252` 清单①②（ledger.rs：Realize 构造子 + 新不变量 + is_legal_from）
  - `49f0821584` 清单④⑧（transition.rs：成本基/利润分离 + 白名单落文档 + cash-sound gate 测试）
  - `3987926ba5` 清单③⑤⑥⑦ + 可达性生产见证（runner.rs/coverage.rs/state.rs/mutex.rs）
- 测试基线: 1447 → 终态 **1452 passed, 0 failed**（108 ignored 不变；净增 5 个测试，无删除——
  两个具名测试按裁定重写/改名而非删除）

## 1. 结论

codex 裁定 A' 九条清单全部落地。核心变化：

1. **`TwEvent::Realize(i64)`（第 8 构造子，可正可负）**：`tw_step` 做 `free += d_pi`，唯一 TW
   漂移构造子。新不变量两半：非 Realize 七构造子保 TW 守恒（`tw_step_preserves_tw` 更新）；
   Realize 漂移恰 = Σd_pi（`tw_step_realize_drift_equals_dpi` 新增，含推导链第 8 条
   「Realize 后不回退 stage」见证）。（清单①）
2. **`is_legal_from` 把 `Realize(_)` 归恒合法**（推导链第 7 条，同 ShortDiff 类）；约束落在
   producer/source-validity + cash-sound gate——`transition_negative_realize_overdraft_returns_err`
   见证两条注入路径（realized_pnl 字段 / 直接 tw_event 注入）都被 `CashUnsound` 拦截，负 free
   不静默落盘。（清单②）
3. **生产接线（π fill loop）**：`apply_fill`/`apply_order` 返回本次 fill 的费后已实现 PnL；
   `pi_theta_fill_loop` 新增 ②'' 步——对累计已实现 PnL 量化取差分派 `Realize(d_pi)`（shadow
   模式，截断误差有界不累积，TW 漂移恒 = ⌊Σ已实现PnL⌋）。位置 = ②' 成本基 ShortDiff 之后、
   ③ TwStepCtx（stage 判据）之前（硬边界2）。同 bar 多 fill 经累计差分自然聚合（推导链第 9 条）；
   `forced_pnl` 在循环外计算、②'' 只消费 apply_order 返回值 ⟹ 结构性隔离（推导链第 11 条）。（清单③）
4. **closed_loop 成本基/利润分离**：`OrderOut` 增 `realized_pnl` 字段（固定形状事件序列
   (成本基, 利润)）；schedule_adapter 减仓分支成本基改走 `Allocate(−basis)`（与建仓对称，
   **不再**把成本基释放记 `LedgerEvent::Realize` 污染 Π）；transition_adapter 按序应用
   `TwEvent::Realize`（TW free）与 `LedgerEvent::Realize`（R 账本 Π）——两账本构造子不混称。（清单④⑨）
5. **P3/P4 masking 订单流测试**：coverage P2/P3/P4 三测试补 tw=None 对照，断言 **Order 本身**
   不同（P3/P4 无订单事件但消耗当步裁决 ⟹ 同 bar 普通开仓 qty 从 >0 变 0；P2 直接产关腿订单）。（清单⑤）
6. **具名测试⑥重写**：`tw_ledger_producer_in_place_and_conserved` →
   `tw_ledger_producer_drift_equals_quantized_realized_pnl`，断言「TW 增量 = 已实现 PnL 量化和」
   （旧断言恒守恒 = 正 PnL 在账本里凭空消失，codex 裁定具名要求重写）。
7. **具名测试⑦改名/改断言**：`earning_shares_structurally_unreachable_from_campaign_tw_conserved`
   → `earning_shares_unreachable_l0_same_price_zero_pnl`（L0 同价无盈亏定理）。有效域收窄：
   L2 realized-PnL 路径不再被该定理覆盖；事件枚举补 `Realize(0)`（同价平仓 PnL≡0 的守恒退化）。
8. **★可达性生产见证（GAP3 留白解除的 L1 证明）**：`pi_loop_realized_profit_reaches_earning_shares`
   ——合成价格序列（px 100→1000）上 buy@bar8 → sell@bar15（realized ≈ +5.4e6，全部来自实际平仓
   fill 费后 PnL）→ re-buy@bar18，生产 π 路径 P3 于 bar18 派 `RecoverCapital(1_000_000)`（足额
   退本金，stage II）→ P4 于 bar19 `EnterEarning` ⟹ **终态 TStage=EarningShares**，且 TW 漂移
   仍恰 = 已实现 PnL 量化和（资金源唯一性端到端审计）。GAP3「∃t TStage=III」从「结构不可达
   FALSIFIED」变为「生产路径 L1 可达」。

## 2. 定义依据

- PDF p8①②③（断裂句法/不变量存在依赖/禁语义回补）经 codex A' 推导链第 4 条兑现：
  `pnl = pos_sign·(px_exit_net − entry_cost)·close_qty`（`runner.rs::apply_fill` 段 1）是平仓
  结算事实，后续价格不能否定——与 hwm_gain（可被回撤否定的未实现峰值）本质区别，故不落入 p8③
  禁止范围。实装严格复用该已有计算（返回值透传，不重造第二套 PnL 公式）。
- 推导链第 5 条（可正可负）：Realize/realized_pnl 全链路无符号过滤；亏损见证在
  `tw_step_realize_drift_equals_dpi`（d_pi=−80/−1）、`transition_realize_profit_and_loss_enters_
  both_ledgers`（realized=−6）、②'' 注释（负 d_pi 不钳制）。
- 推导链第 6 条（stage 驱动白名单）：`stage_progression` 文档落白名单（free/holding/withdrawn/
  notional_in/open_legacy_legs/risk_mode）与黑名单（hwm_gain/MTM equity/forced_pnl/未平仓浮盈）；
  代码审计：stage_progression + enter_ready 只读白名单字段（tw() 三量均成本基口径）。

## 3. 边界条件（结论翻转条件）

- (a) 若 ②'' 的资金源改为逐 bar MTM 累计（非锚定实际 close fill 的 apply_order 返回值）⟹
  违反推导链第 10 条，本实装的合法性依据失效。
- (b) 若未来让 hwm_gain/MTM 权益进入 stage 判据（白名单外字段）⟹ 触发裁定边界条件 (b)，
  须重新提交裁决。
- (c) 若 forced_pnl 变成改变 units/cash 的真实订单 ⟹ 按真实 fill 重新评估纳入（当前结构性隔离）。
- (d) 量化口径：②'' 用「累计值截断量化再差分」（`as i64`，与 nav0/basis shadow 同款）。若改为
  逐笔量化求和，TW 漂移与 ⌊Σ⌋ 的差可累积——清单⑥测试的等式断言会翻转。
- (e) 可达性见证依赖 w₀=0.60 的 depth_weights 默认（再投资比例）与 κ=0 基线；若 sizing/barrier
  配置变更使「holding≥notional ∧ free≥target」在该序列上不再同时满足，见证测试需重构序列
  （机制不受影响，见证的价格参数受影响）。

## 4. 下游推论

- **#135 q4（prereg 冻结，被本工位 block）**：重跑清单必须标注**两类 bit-exact 风险**（裁定
  清单⑧，codex §4 修正已核验）——(i) P2 CloseOverlay 直接产订单；(ii) P3/P4 priority masking
  无订单事件但消耗当步裁决、间接改同 bar 普通开/平仓订单（测试锚 = coverage 三测试的 tw=None
  对照断言）。TW→订单流的唯一反馈通道是 P2/P3/P4 触发（TW 是 shadow，不进 base_units/sizing），
  故全历史重跑中若任一 bar 触发 P3（需已实现利润 ≥ ~1.5×NAV 且再投资足额），该 bar 起订单流
  分叉。若后续把 TStage/ηBucket 加进 z 并被 χ 消费，μ key 变化也改订单流（裁定原文第 8 条）。
- **G5/#124 的「P2/P3/P4 结构不可达」声明**：已全部更新为「生产可达（A' 落地）」——涉及
  runner TW 初始化注释、coverage 段注释、mutex.rs 模块头、state.rs funded_campaign、
  closed_loop 三个测试的结论段。`g5-impl-20260703.md` §3.4/§4 的可达性限定语按裁定 §6.4 失效，
  以本结果包为准（不改历史归档文件本身）。
- **closed_loop 的不触达语义收窄**：A' 后 closed_loop 不触达 EarningShares 的根因从「账本无
  资金源」变为「PhaseI intent 恒 Buy ⟹ 无平仓 fill ⟹ 无已实现 PnL」（策略性不触达）。
  `l2_btc_earning_shares_unreachable_hwm_debearing`（#[ignore]）预期 count=0 不变，理由已改写。
- **GOLDEN 影响（诚实声明）**：非忽略测试集**零 GOLDEN diff**（1452 全绿，无一冻结数值改动）
  ——因为现有非忽略场景中 P3 触发门不满足（小额 PnL ≪ notional）且 equity/cash/χ 路径与 TW
  shadow 完全解耦。**这不等于全历史 L2 重跑 bit-exact**：#[ignore] 全历史跑批属 #135 范围
  （见上两类风险标注）。
- **Lean 侧缺口（非本工位范围）**：`Origin.TotalWealth.TWEvent` 尚无 Realize 对应构造子（与
  Revalue 同为 Rust 先行），`twStep_preserves_tw` 的 Lean 陈述需扩展为「非 Realize 守恒 +
  Realize 漂移」两定理——已在 tw_step 文档诚实标注不冒充已锚。

## 5. 谱系引用

- `.chanlun/review-results/codex-gap3-ledger-20260703.md`（裁定 A' 原文，本实装唯一规格源）
- `.chanlun/review-results/codex-r3-ruling-20260702.md`（R3 C'：hwm_gain 去承重，A' 的直接前提）
- `.chanlun/review-results/g5-impl-20260703.md`（§3.4 边界条件 (a) 留白——本实装解除该留白）
- 090号（严格性：两个具名测试重写而非删除；声明与实际一致的过期注释全量清理）
- 231号（形式化有效域：本实装 L1——机制可达性见证用合成价格序列；真实数据触发频率/盈利性归
  L2/L3 即 #135）
- 576/674号（R vs TW 双账本不同构：LedgerEvent::Realize 与 TwEvent::Realize 不混称的谱系根据）

## 6. 影响声明

- 改动文件（全部 rust/src/theta_v0 下，3 commit）：
  - `strategy/ledger.rs`：TwEvent::Realize + tw_step 分支 + is_legal_from + 新旧不变量测试 +
    hwm_gain/tw()/tw_step 文档更新
  - `closed_loop/transition.rs`：OrderOut.realized_pnl + schedule_adapter 减仓分解 +
    transition_adapter 双账本按序入账 + stage 白名单文档 + 3 个新测试
  - `closed_loop/state.rs`：funded_campaign 文档（有效域收窄）
  - `backtest/runner.rs`：apply_fill/apply_order 返回 PnL + ②'' Realize 派发 + 清单⑥⑦测试
    重写/改名 + 可达性见证测试 + 过期声明清理
  - `strategy/coverage.rs`：P2/P3/P4 masking 订单流断言 + 段注释更新
  - `strategy/mutex.rs`：模块头可达性声明更新
- 行为变更面：**生产 π 路径的 TW 账本语义**（TW 从守恒量变为「非 Realize 守恒 + 已实现 PnL
  漂移」）与 **closed_loop 减仓的双账本事件**（Π 不再被成本基污染）。订单流仅在 P2/P3/P4 触发
  时改变（见 §4 两类风险）；equity_curve/trades/metrics/χ/μ 全部不变。
- 零改动：判据优先级（P1..P10 序不变）、interp fold、Lean 文件、数据文件。
