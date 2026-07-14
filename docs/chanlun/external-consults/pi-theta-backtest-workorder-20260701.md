# 施工单：忠实 π_Θ 闭环回测（测 𝔖_Θ 本身，不测投影）

> **目的**：现有全部 alpha 结论（663→667：奇偶交替、beta/alpha、命题② inconclusive、时间轴 walk-forward）
> 绑定的都是 `econ_positive.rs` 产出的**方向投影台账**（12626 信号，三类混合 δ=±1，口径4 出场），
> **不是** π_Θ 的真实执行。按「测 𝔖_Θ 必须实装整个系统再测」的原则，本施工单列出把回测从
> 投影切换到忠实闭环执行所需的确切工作。
>
> **认证归属**：忠实性由 `cargo test` 的 parity fixture（bit-exact 对齐 Lean）+ `lake build` 零 sorry
> 认证。执行方需在有 rust/lean 工具链的环境里 build + 跑 parity——这是本施工单每块的验收关口。

---

## 0. 已就绪（READY，无需重做，仅需接线）

逐条给了契约锚 + 代码位置，执行方应先跑通这些以确认基线：

| 分量 | 代码位置 | Origin Lean 契约锚 | 状态 |
|---|---|---|---|
| 闭环 driver（逐 bar 喂 `hybrid_step`） | `rust/src/theta_v0/backtest/runner.rs:802` `run_closed_loop` | `FullDefinitionStrategy.hybridStep` | ✅ 真逐 bar（`:812` for bar → `:816` x=hybrid_step；`closed_loop_threads_every_bar` 证每分量步数=bar 数） |
| 全互斥解释器 I_Θ（P1..P8→C_0..C_9） | `closed_loop/mutex_interp.rs` | `FullDefinitionStrategy.chooseAction` / `action_priority_complete_unique` | ✅ 10 互斥类，唯一性已证 |
| 六段数据流 | `closed_loop/transition.rs` `hybrid_step` | `policyTheta = schedule∘risk∘intent∘classify∘recStruct`（:226-227） | ✅ |
| R=Π-A-W 账本 | `strategy/ledger.rs` `LedgerComp` | `Origin.LedgerState`（inv: R=Π-A-W） | ✅ `ledger_inv_always` |
| 卖侧闭环（顶背驰识别 + closeRoot/reduceCore） | `closed_loop/sell.rs` `sell_transition` | `SellPointRecog.lean`+`SellClosedLoop.lean` | ✅ bit-exact parity 已过（`lean_sell_ledger_delta_bit_exact`） |
| 买侧闭环转移 | `closed_loop/buy.rs` `buy_transition` / `recog_chanlun_buy` / `buy_decision_ledger_delta` | `ThetaInstantiation.chanlunTransition`（:297） | ✅ 实装在（openRoot→A+1 / accreteCore→A+2 / hold→noop） |
| 风险门（保证金强平） | `runner.rs` `run_theta_v0_pi` 的 `k_theta_risk_gate` | `FullDefinitionStrategy` risk 段 | ✅ |

---

## GAP 1 — 买侧独立 bit-exact parity（测试保真缺口，非实装缺口）

**性质**：买侧闭环**已实装**（见上表）。缺的是**独立断言**——parity 测试
`closed_loop/sell.rs:405 buy_sell_A_delta_mirror` 用**硬编码常量** `open_root_da: i64 = 1`
（:406）经卖侧对偶间接断言，买侧 rust 函数 `buy_decision_ledger_delta` 的输出**从未被独立**
对齐 Lean `decisionLedgerDelta`（`ThetaInstantiation.lean:256-259`）。`tests/theta_v0_lean_parity.rs:44,271`
已诚实标注此 vacuity。

**契约锚**：`formal/Origin/ThetaInstantiation.lean`
- `decisionLedgerDelta`（:256）：openRoot→(0,1,0)、accreteCore→(0,2,0)、hold→(0,0,0)
- `recog_type1_openRoot`（:168）、`recog_type3_accreteCore`（:181）、`recog_continuation_hold`（:199）

**要做**：在 `tests/theta_v0_lean_parity.rs` 加独立测试，从 Lean fixture（`load_fixture`，:110）取
买侧 delta 期望值，直接断言 rust `buy_decision_ledger_delta(OpenRoot/AccreteCore/Hold)` 与
`recog_chanlun_buy` 的三类识别 bit-exact 匹配——**不经卖侧对偶、不硬编码常量**。

**验收**：`cargo test lean_buy_side_delta_bit_exact`（新）绿；且删除/替换 `buy_sell_A_delta_mirror`
里的硬编码 `open_root_da=1`，改为从 rust 函数取值。`lake build` `ThetaInstantiation.lean` 零 sorry。

**对 𝔖_Θ 的影响**：低。买侧执行本已忠实，此缺口只影响「买侧忠实性是否被独立证据背书」。可先做（快、解锁审查信心）。

---

## GAP 2 — 二类买卖点闭环（自动中枢分配未实装）

**性质**：`second_type_via_sublevel_type1`（`classifier/descend.rs:171）已实装二类⟸次级别一类的判据，
但 `center_of`（:141,174）是**外部传入闭包**——「次级别每走势自动配其相关中枢」未实装
（descend.rs:40 still-MISSING）。故二类无法在闭环里自动触发。`closed_loop/buy.rs:56` 标 `#113 still-MISSING-D`。

**契约锚**：`ThetaInstantiation` / `descend.lean` `secondType_via_subLevel_type1`；中枢构造见
`formal/Origin/CenterConstruct.lean` / `CenterAutoAssign.lean`（`center_of` 的自动版契约）。

**要做**：实装自动 `center_of(&RMove) -> Center`——对次级别每个走势，从已构造的中枢序列里
取「最后一个相关中枢」（缠论二类定义：一类买点后的回抽，回抽落在次级别一类构成的走势里）。
接进 `recog_chanlun_buy` / `sell` 的二类分支，使闭环能自动识别二类。

**验收**：`cargo test second_type_via_sublevel_type1_witness`（descend.rs:303，已存在）在
**自动 center_of** 下仍绿；新增端到端测试：闭环在含 ≥3 同向次级别走势的 fixture 上自动产二类信号。
`lake build` `CenterAutoAssign.lean` 零 sorry。

**对 𝔖_Θ 的影响**：中。二类是三类买卖点之一，缺它则闭环覆盖 {一类, 三类, 延续}，二类信号被漏。

---

## GAP 3 — TW 提现端 W（三阶段负成本挣股数，真正的硬缺口）★

**性质**：Rust 闭环里 **W 恒为 0**（`buy.rs` 明确「W 恒 0，TW 端 still-MISSING，不硬塞进单账本买侧」；
`sell.rs:34,134,170` 同）。三阶段「负成本挣股数」(S_t: CostReduction→CapitalRecovered→EarningShares,
递归 η_{n+1}=η_n+g_n−a_n) 依赖的正是提现分量 W——**没接 = 测不到 𝔖_Θ 最独特的论断**。

**契约锚（已是完整 native Lean，零 sorry，可直接 port）**：`formal/Origin/TotalWealth.lean`
- `TStage`（:63）三阶段 + `TStage.rank`（:73）+ `advanceTo`（:133）单向推进
- `TWState`（:94）：free/holding/withdrawn/openLegacyLegs/notionalIn；`TWState.tw`（:105）= free+holding+withdrawn
- `twStep`（:159）+ `twStep_preserves_tw`（:178）守恒 + `stage_rank_monotone`（:187）+ `earning_no_regress`（:205）
- `OQ9Inv`（:272）+ `oq9inv_preserved`（:296）+ `LegalEnterEarning`（:249，openLegacyLegs=0 才可进 EarningShares）
- ⚠ 与 R=Π-A-W **不同构**（`not_isomorphic_*`，:480/499/523）——**两独立账本并置，勿合并**（#90）

**要做**：
1. 把 `TWState` + `twStep` + 三阶段机 native port 进 rust（`closed_loop/` 新模块或 `strategy/tw.rs`），
   bit-exact 对齐 Lean。保持与 `LedgerComp`（R=Π-A-W）**并置不合并**。
2. 在 `AssemblyState`（`closed_loop/state.rs`，已有 `tw_state` 分量锚 `Origin.TotalWealth`）里
   真驱动 tw_state：`hybrid_step` 每步按闭环决策产 `TWEvent`，跑 `twStep`。
3. 卖侧顶背驰实现利润 → Π↑ 后，按三阶段把 free→withdrawn（CapitalRecovered），进 EarningShares 后
   用 g_n 挣股数（a_n）。接 `sell.rs` 已标的 W↑ 决策层。

**验收**：
- rust `twStep` 与 Lean bit-exact（新 parity 测试，对齐 `twStep_preserves_tw` / `oq9inv_preserved`）。
- `AssemblyState.tw_state` 逐 bar 真推进（仿 `closed_loop_threads_every_bar` 加 tw 分量断言）。
- `lake build` `TotalWealth.lean` 零 sorry（已满足，勿破坏）。
- L2 守卫：`prove_tw_neutral`（同价 NAV 中性）在闭环回测路径上成立。

**对 𝔖_Θ 的影响**：高。这是 𝔖_Θ 区别于「普通多空策略」的核心机制。**没有 GAP 3，回测跑得起来
但测不到三阶段论断——「测了整个系统」不成立。GAP 3 是本施工单的关键路径。**

---

## 依赖与顺序

1. **GAP 1**（快，低风险）先做——解锁买侧忠实性的独立证据，让后续审查有干净基线。
2. **GAP 3**（关键路径）——最大工作量，决定回测能否测到 𝔖_Θ 本质。与 GAP 2 独立,可并行。
3. **GAP 2**（中）——补齐二类覆盖。可与 GAP 3 并行(不同模块)。
4. 三块各自过 parity + lake build 后 → 回测 harness（下节）。

---

## 回测 harness 设计（三块闭合后执行）

**入口**：`run_closed_loop(bars, initial_nav)`（已存在）→ 逐 bar `hybrid_step` → 终态 `AssemblyState`。
需补：

1. **账本结算成收益曲线**：每 bar 从 `AssemblyState.ledger_state`（R=Π-A-W）+ `tw_state`（free+holding+withdrawn）
   取组合净值 NAV_t，产 NAV 曲线。区分 R 账本 PnL 与 TW 三阶段的 withdrawn（已提现本金）。
2. **因果性守卫**：确认 `NewBar(rising)` 事件无前视（runner 已用前缀分类，复核 `run_theta_v0_pi_prefix_classify_is_causal_no_lookahead`）。
3. **投影 vs 忠实对照**：同一条 BTC 全历史，跑 ① 忠实闭环 NAV 曲线 ② 现有 12626 投影台账 baseline。
   量化差异——尤其空头腿：投影里是裸空（`backtest_t_fugue.py` `pnl=sh*(ep-xp)`），闭环里是对冲子声部。
   **预期**：PDF 恒等式 ΔV⁺+Π⁻=−C 意味着裸空腿必亏（投影已证实），而闭环对冲腿的价值应体现在
   TW 三阶段的 withdrawn/挣股数上，不在空头腿的独立 PnL 上。这是「投影为何误导」的定量证据。
4. **alpha 判据**（承 663）：μ>0 判据用在闭环 NAV 的增量上，分级别（低频高级别不强求显著性）。
   walk-forward OOS（承本 session 的时间轴 R_t 方法）验 confirm/inconclusive。

---

## 禁止事项（否则又产投影）

- **勿**在没接 GAP 3（W=0）的情况下声称「测了 𝔖_Θ」——那仍是缺三阶段的投影。
- **勿**合并 TW 账本与 R=Π-A-W（#90 不同构，两独立结构并置）。
- **勿**用硬编码常量替代 Lean 见证（GAP 1 的教训）——每块的 delta 必须从 rust 函数经 parity 断言。
- **勿**在 parity/lake build 未过前就跑回测下结论——忠实性未认证的回测结果无效。
- 台账须透传 `min_class`（一/二/三类）+ 买卖侧（`interp.rs:214` 已算，下游没透传）——否则又无法按类型分层。

---

## 一句话结算（可入谱系）

> 忠实 π_Θ 回测 = `run_closed_loop` driver（现成）+ 三块闭合：GAP1 买侧独立 parity（测试保真，轻）、
> GAP2 二类自动 center_of（中）、**GAP3 TW 提现端 W（关键路径，无它则测不到三阶段负成本挣股数论断）**。
> 三块契约均已是 Origin native Lean（零 sorry），port 边界清晰。在三块过 parity+lake build 前,
> 任何「缠论(有/无)alpha」主张只对方向投影成立,不可外推到 𝔖_Θ。
