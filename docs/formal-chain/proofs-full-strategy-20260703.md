# 全定义互斥策略——严格数学证明整合文档（阶段一：已稳定层）

> 日期 2026-07-03。任务 #125（goal g-20260703T0300Z-full-spec-pi q3 验收项）。
> 源规格＝`docs/formal-chain/完整的策略.pdf`（编排者 7-02 22:50 钦定「严格实装版」，唯一权威）。
> 对照实现基线：`rust/src/theta_v0/`，1402 单元测试全绿。
>
> **编排者双令**：「实现不能是简化的，只能是严格的，并且要附带严格数学证明。」本文档是「严格数学证明」验收项。
>
> **阶段边界（诚实声明，非 TODO 遗留）**：本文档阶段一覆盖**已稳定层**（parser bit-exact / §14 互斥 / AncOK / 区间套 / LexArgmin / Schedule），其证明完整无缺。q2 实装在飞（G1 Cand 力度门 #115 / G3 z 扩维 #122+#123 / G5 解释器统一 #124 / 高级别一三类候选 #123）会新增锚点——这些项登记在 §D「证明义务登记表」，由阶段二任务闭合。
>
> **阶段二（2026-07-03，Task #131）已执行**：q2 六工位（#115/#132/#138/#134/#124/#133）全部落地后，§D 六项 OPEN 义务已逐项闭合（实装锚点+机器证明锚点+认识论等级，见 §D 闭合明细），§A/§B/§C 受 q2 改动影响的锚点行号已全部重新 Read 验证并修正漂移（interp.rs / coverage.rs / mutex.rs / econ_positive.rs / transition.rs 五文件）。残余诚实缺口（D-1 A4 缺 TV/SubMovePower、D-3 TStage/ηBucket/CostBucket 装配与 Live 轴、D-5 P2/P3/P4 生产可达性依赖 #139 账本重裁）照实标注为 OPEN 子项，不并入 CLOSED 声明（090号）。阶段二结果包见 §F。
>
> **增量（2026-07-03，#140 gap3-realize 落地后，Task #131 送审前置）**：#139 已裁——codex 终局裁定 A'「已实现利润入 TW 账本」（`.chanlun/review-results/codex-gap3-ledger-20260703.md`），#140 三实装 commit + 结果包（`f3e3709252`→`49f0821584`→`3987926ba5`→`334487e309`，`gap3-realize-impl-20260703.md`）落地 `TwEvent::Realize` ⟹ D-5 可达性子项闭合：GAP3「∃t TStage=III」从结构不可达 FALSIFIED 翻转为 **L1 生产可达**（不可达定理限定为 L0 同价无盈亏版本）。§C 新增 C.5（TW 账本不变量新形式）；§D 新增 D-8（Lean 侧独立证明义务）。测试基线 1447→1452 全绿。本文档受影响的可达性陈述（假设 8 / §B / §D-5 / §F 交叉引用）已同步更新，锚点写前 Read 重验。

---

## 认识论等级约定（formalization-validity-domain 231号，强制）

| 等级 | 名称 | 含义 |
|------|------|------|
| **L0** | 纯代数/定义推导 | 从定义推出，零信息增量（同义反复）|
| **机器证明** | property test 看守 | 穷举/对拍验证实装无 bug（L1 管线正确性）——验证「实装＝定义」，不验证经验假设 |
| **L2** | 真实数据验证 | 真实标的/时段假设检验，可产生否定性结果 |

**关键区分**：本文档所证的「唯一性/互斥性/全定义」全部是 **L0 结构性质**（策略是全函数、订单唯一），**不是** L2 alpha 声明。alpha 有效性（μ(z,a)>0）属 §12 选择器层，四口径实证全 INCONCLUSIVE（见 `strategy-spec-for-external-review-20260703.md` 六节），**不在本文档证明范围**。有效域≠定义域处在各节显式声明。

---

## §A. 全定义唯一性定理（PDF §13）

### 定理陈述

给定 11 条假设，则

$$\forall x_t,\quad \exists! \, O_{t+1} = \pi_\Theta(x_t).$$

即：策略 π_Θ 是从状态空间到订单空间的**全函数**——每个状态恰好产出一个订单。

### 证明结构（PDF §13 逐链）

由假设 1–4 ⟹ 证书集合 Γ_t 唯一；由 5 ⟹ 状态 z_t 唯一；由 6 ⟹ 全互斥类别唯一；由 7 ⟹ 活动声部集合 A_{t+1} 唯一；由 8 ⟹ 三阶段事件唯一；由仓位函数 ⟹ p̃ 唯一；由 9–10 ⟹ 风险投影最优点 p\* 唯一；由 11 ⟹ 订单 O 唯一。链上每一步都是全函数且输出唯一 ⟹ 复合全函数 ⟹ ∃! O_{t+1}。

**机器证明看守（∃! 结论）**：`rust/src/theta_v0/strategy/coverage.rs:4457` `pi_theta_step_deterministic_unique_order`——同状态两次调用 `pi_theta_step` 产逐字段相等订单（决定性＝全函数的可观测面）。生产入口 `coverage.rs:2327` `pi_theta_step` / `coverage.rs:2205` `pi_theta_position` / `coverage.rs:2239` `schedule_order`。

### 11 条假设逐条：数学陈述 + 实装锚点 + 证明状态

所有锚点写前经 Read 验证行号真实存在。

---

**假设 1：结构塔 T_t 全定义且 bit-exact**（`T^{inc}_t = T^{full}_t \ \forall t`，PDF §2.1）

- **(a) 数学陈述**：增量塔与全量重算塔逐字段相等；未确认 frontier 不得当 sealed prefix 复用。
- **(b) 实装锚点**：
  - `rust/src/theta_v0/parser/fractal.rs:327` `bit_exact_incr_fractals_per_bar`——增量 `IncrFractals::append` 链 == 全量 `detect_fractals`，逐 merged 长度。
  - `rust/src/theta_v0/parser/stroke.rs:486` `bit_exact_incr_strokes_per_bar`——增量 `IncrStrokes::append` == 全量 `build_strokes`。
  - `rust/src/theta_v0/parser/profile.rs:240` `bit_exact_parse_layer_incr_per_bar_synthetic`（always-run 合成）+ `parser/profile.rs:275` `bit_exact_parse_layer_incr_per_bar_es`（`#[ignore]`，真实 ES 数据）。
  - `rust/src/theta_v0/backtest/incremental.rs:116` `bit_exact_per_bar`——逐 bar 断言增量 == 全量解析。
  - `rust/src/theta_v0/backtest/incremental.rs:924` `decisive_endpoint_tower_parity_longhistory`（`#[ignore]`，L2 真实 CL/BTC 长历史终点 level 分布对拍）。
- **(c) 证明状态**：**机器证明**（合成逐 bar always-run）+ **L2**（真实 CL/BTC 长历史，`#[ignore]` 需数据；#88/#93 修复史 commit `c546b5633c` 已跑通并复核零翻转）。
  - **有效域声明**：合成 always-run 是 L1 管线正确性；level≥2 高级别塔的 bit-exact 依赖真实长历史（`incremental.rs:900+` 记录 H2 否证——增量长历史无 frontier 发散 ⟹ level2-4 稀疏是市场几何非 bug 伪影，L2）。

---

**假设 2：候选集合 Γ_t 有限**

- **(a) 数学陈述**：每时刻候选证书集合 Γ(x_t) 是有限集，|Γ|=len<∞。
- **(b) 实装锚点**：`rust/src/theta_v0/strategy/interp.rs:213` `assemble_gamma(&Classification) -> Vec<Candidate>`——返回 `Vec`，长度即 |Γ|（有限）。模块文档 `interp.rs:19` 显式声明「Γ(x) 有限（`assemble_gamma` 返回 Vec）」。H2 优化后生产另有单次建树版 `interp.rs:383` `assemble_gamma_with_tower`（委托 parts 版，bit-exact == 旧版）。
- **(c) 证明状态**：**L0**——`Vec` 的类型保证有限性（来自有限结构塔的有限元素集，无生成无穷候选的路径）。

---

**假设 3：N^δ_{ℓ↓e} 全定义**（真递归区间套）

- **(a) 数学陈述**：方向化区间套证书 `N^δ_{ℓ↓e}(x) ∈ {0,1}` 对任意输入有确定值；基例 `N^δ_{e↓e}=Conf^δ_e`，递归步 `N^δ_{ℓ↓e}=Cand^δ_ℓ ∧ [J^δ_{ℓ-1}⊆J^δ_ℓ] ∧ N^δ_{ℓ-1↓e}`。**区间包含非端点相等**（PDF §3）。
- **(b) 实装锚点**：
  - `rust/src/theta_v0/classifier/nest.rs:200` `NestCertificate::n_delta(&self) -> bool`（全定义，返回 bool ⟹ ∃!∈{0,1}）；`classifier/nest.rs:205` `n_delta_rec`（级别结构递归，rungs 从高到低）。
  - `rust/src/theta_v0/classifier/nest.rs:62` `is_sub(inner, outer)`：`inner.start_time>=outer.start_time ∧ inner.end_time<=outer.end_time`——子⊆父闭口径区间包含。
  - 机器证明：`classifier/nest.rs:326` `is_sub_nesting_bit_exact`。
- **(c) 证明状态**：**L0**（结构递归全定义）+ **机器证明**（is_sub parity）。终止性：级别每步严格下降（rungs 缩短，对应 Lean `(ℓ-e):Nat` 结构递归）⟹ 有限终止。
  - **有效域声明**：`N^δ` 的**结构形式**（级别比较＋⊆＋Sel_Θ 选 chosen）是 L0；实测 BTC 全历史 95% 退化为深度=1（"小转大"92%），深度≥2 几乎为 0——这是 L2 市场几何事实，见假设 3 的区间套 J_child⊆J_parent L2 验证（§C.3）。

---

**假设 4：I_γ 类型全定义**（六类买卖点 + StructBreak）

- **(a) 数学陈述**：买卖点类别 `I_γ ⊆ {B1,B2,B3,S1,S2,S3,StructBreak}`，走势六态 `r∈{⊥,I,U⁰,U¹,D⁰,D¹}` 是穷尽 partition（∃! 落一态）。
- **(b) 实装锚点**：`rust/src/theta_v0/classifier/six_state.rs:155` `six_state_exhaustive_and_bot_dimension`——穷举上下文断言落六态之一且符号唯一（`six_state.rs:179` 断言 `seen.len()==6` 六态全可达非退化）；6-bit `BspBits` 掩码 64 类不压扁（`classifier/bsp.rs`，#83 六态不塌缩）。
- **(c) 证明状态**：**机器证明**（六态 partition 穷尽 + bot 真新增维度）。
  - **有效域声明（阶段二闭合，#123 hl13-impl，commit `0a35f0167c`）**：类型*分类*函数在 L0 上全定义（穷尽 partition 已证）。**level≥1 高级别一/三类候选生成**已实装（codex 裁定 A：级别-N 直接判定，非 L0 标签继承）：`rust/src/theta_v0/classifier/mod.rs:157` `extract_first_third_for_level`——本级 units 经 `unit_to_segment` 还原后复用 `signal.rs` 同一套一/三类判据（零第二套趋势/力度引擎），全量+增量两路 bit-exact parity。L2 普查（BTC 4.61M bar，`hl13-impl-20260703.md`）：level≥1 净增 2364 个三类信号（修前 else 禁闭恒 0）；一类全历史 0 于所有级别=市场事实（该级中枢链全局非单调 ⟹ trend_dir=None），牛市窗（2020-10..2021-04）L4 产 2 个 sell1（≈$60K/$61.5K 历史顶区）证伪「恒死」——触达是水平线依赖（L2），非架构死码。

---

**假设 5：𝔠_Θ 是函数**（成本函数）

- **(a) 数学陈述**：交易成本 `C_i` 是状态/成交的确定函数（单边费率 commission+slippage+tax）。
- **(b) 实装锚点**：`rust/src/theta_v0/strategy/exec.rs:73` `apply_fees(base, side, config) -> Tick`（买加卖减，确定）；消费侧 `backtest/metrics.rs` 方向感知 `trade_abs_pnl`（多头 `qty·(exit·(1−fee)−entry·(1+fee))`，成本解耦）。机器证明 `exec.rs:336` `fees_buy_adds_sell_subtracts`。
- **(c) 证明状态**：**L0**（纯函数，输入 base/side/config ⟹ 唯一输出）+ **机器证明**（费用施加对拍）。

---

**假设 6：解释器 I_Θ 使用固定优先级**

- **(a) 数学陈述**：`I_Θ(A_t, Γ_t) = (D_t, O_t, L_t)`——候选按固定优先级 `≺_Θ` 全序裁决为关/开/记录三桶，无二义。等价于原始可重叠谓词 `P_1..P_m` 的固定优先级互斥化（见 §B）。
- **(b) 实装锚点**：`rust/src/theta_v0/strategy/interp.rs:990` `interpret`（候选按 `theta_key` `≺_Θ` 全序 fold；G4 后委托 `interp.rs:1004` `interpret_with_close_triggers` 单源本体，签名/行为不变）；固定优先级谓词互斥化 `strategy/mutex.rs:143` `mutex_class`；桶级等价对拍 `strategy/mutex.rs:574` `shadow_fold_bucket_equivalence`（12 场景，interp `≺_Θ` 序 == mutex P1..P10 优先级序 + typed 精确类号，零分叉）。
- **(c) 证明状态**：**机器证明**（§B 互斥定理 + D1 桶级等价 12 场景）。
  - **有效域声明（阶段二闭合，#124 G5）**：P1 风险强平与 TW/GAP3 事件已统一进 PDF §7 单一 P1..P10 固定优先级序，生产以**两级结构**兑现（codex-ruling4-addendum 分歧A 裁决：`interpret` 签名与本 ∃! 证明锚不动，I_Θ=组合层）——bar 级 P1..P4 在 `coverage.rs:2435` `pi_theta_step_traced`（P1 上游短路 `coverage.rs:2449` / P2 CloseOverlay `coverage.rs:2472` / P3/P4 消耗当步裁决 `coverage.rs:2510`），候选级 P5..P10 在 interpret fold（结构原样）。逐谓词锚点表与可达性证明（L0 同价不可达定理 + #140 L1 生产可达见证）见 §D-5。

---

**假设 7：AncOK 是函数**（祖先生命期包含，PDF §2.2/§8）

- **(a) 数学陈述**：`AncOK(A)={a∈A : Anc(a)⊆A}`；活动集递归 `A_{t+1}=AncOK[(A_t∖D_t)∪B_t]`。生命期包含不变量：`∀a∈Anc(e), [λ_e,ρ_e]⊆[λ_a,ρ_a]`（父生命期覆盖子生命期，否则子声部漂浮在已结束的父上）。
- **(b) 实装锚点**：
  - `rust/src/theta_v0/strategy/coverage.rs:698` `ancestor_close_by_id`（§13 生产版，按 `parent_id` 结构映射链，跨 bar 稳定）；`coverage.rs:675` `ancestor_close`（M16 索引链版）。
  - `rust/src/theta_v0/strategy/coverage.rs:740` `active_set_step`——`A_{t+1}=AncOK[(A_t∖D_t)∪B_t]`（先关后开＋祖先闭合，immutable 返回新集合）。
  - 不变量 I1–I5：`strategy/mod.rs:57-58`（持久身份/方向不变/parent 是关系非身份/操作父持久/AncOK 作用 persistent set，anc.pdf §7）。
- **(c) 证明状态**：**L0**（`AncOK` 是确定集合运算：过滤祖先不全者，输入 A ⟹ 唯一输出）。祖先不齐的元素被裁掉 ⟹ 保证 `v∈A_{t+1} ⟹ Anc(v)⊆A_{t+1}`（生命期包含不变量的直接推论）。修复史：Q4「LiveDetached 误判 Stale ⟹ depth>0 腿被系统性剪掉」（`mod.rs:57`）。

---

**假设 8：TStage 和 TW 事件全定义**（三阶段资金，PDF §10）

- **(a) 数学陈述**：`TStage∈{CostReduction, CapitalRecovered, EarningShares}`，单向阶段迁移；TW 事件 `{ShortDiff, RecoverCapital, Withdraw, EnterEarning, BuyCore}` 经 OQ-9 门在当前 stage 下合法性判定确定。
- **(b) 实装锚点**：
  - `rust/src/theta_v0/closed_loop/transition.rs:319` `stage_progression`（单向 `CostReduction→CapitalRecovered→EarningShares`，PDF §10 步骤2/3；#124 后 `pub(crate)` 化为 G5 P3/P4 谓词的单源判据，生产 TW 状态由 π fill loop 真实交易流驱动 `runner.rs:680`——见 §D-5）。
  - `closed_loop/transition.rs:268` `TransitionError::Oq9Illegal{event, stage}`——OQ-9 门（如 EarningShares 阶段开 legacy 腿非法）。
- **(c) 证明状态**：**L0**（阶段迁移＋OQ-9 门是确定状态机）。TW 事件集经 #139 裁定 A' + #140 落地扩至八构造子（`ledger.rs:364` `TwEvent::Realize(i64)`，唯一 TW 漂移构造子），不变量新形式见 §C.5。
  - **有效域声明（#140 更新）**：TStage *转移语义*是 L0 全定义。EarningShares *可达性*分两个有效域：(i) **L0 同价无盈亏模型下不可达**——定理保留，测试改名 `runner.rs:2550` `earning_shares_unreachable_l0_same_price_zero_pnl`（同价 ⟹ 平仓 PnL≡0 ⟹ 无 Realize 漂移 ⟹ 退本金前提代数矛盾不变）；(ii) **L1 生产可达**——已实现利润经 `Realize` 入 free 后，生产 π fill loop 见证 `runner.rs:2148` `pi_loop_realized_profit_reaches_earning_shares`（合成变价序列，P3 RecoverCapital → P4 EnterEarning，终态 TStage=EarningShares，全链生产路径无 fixture 直捅）。GAP3「∃t TStage=III」从 FALSIFIED 翻转 L1 可达；真实数据触发频率归 L2（#135）。旧「结构不可达」归因订正史见 memory `project_gap3_l2_unreachable_architecture` 与 §D-5。

---

**假设 9：K_Θ(x_t)≠∅ 且在手数网格上有限**（风险可行集）

- **(a) 数学陈述**：可行净持仓集 `K_Θ = {lot 对齐的 p ∈ [−cap,+cap]}` 非空且有限；风控约束门收窄可行集（force_flat→{0}）。
- **(b) 实装锚点**：
  - `rust/src/theta_v0/strategy/coverage.rs:2155` `feasible_candidates`——lot 对齐净持仓有限网格，含安全锚 `0.0`（**K_Θ≠∅ 的构造性非空见证**）+ bracketing floor/ceil lot 点（凸跟踪目标在 lot 网格的精确有限表示，非近似）。
  - `coverage.rs:2105` `KThetaRiskGate` + `coverage.rs:2124` `caps`（force_flat→(0,0)；stop_long→禁净多；M2/M3→净幅上限）。
- **(c) 证明状态**：**L0**——非空性由构造性见证 `0` 保证（恒在集中）；有限性由 lot 网格离散 + cap 有界保证。凸性精确：主键 `w(p−p̃)²`（w>0）在 lot 离散区间全局最小必在 `clamp(p̃)` 相邻 lot 点 ⟹ 代表集上 LexArgmin ＝ 全 K_Θ 网格 LexArgmin（精确相等，`coverage.rs:2151`）。
  - **有效域声明（阶段二闭合，#133 G7）**：保证金/强平约束已接（#113，`coverage.rs:2114` no_increase_cap）；**最大毛头寸约束**已实装于 legs 折叠前（`coverage.rs:1329` `apply_gross_cap`，逐根子树 KKT water-filling 投影）——锚点与激活口径的诚实声明见 §D-7（default `enforce_gross_cap=false` 保 frozen bit-exact，#135 π^full 收口跑批须显式激活）。

---

**假设 10：LexArgmin 有固定平局规则**

- **(a) 数学陈述**：`p\* = LexArgmin_{p∈K_Θ} J_x(p)`，J_x 是字典序键（主键 tracking_err ≻ 次键 trade_cost ≻ 三键 risk_penalty ≻ turnover ≻ tie-break grid_index）；固定平局规则 ⟹ 键单射 ⟹ p\* 唯一。
- **(b) 实装锚点**：
  - `rust/src/theta_v0/strategy/intent.rs:197` `JThetaKey`（五分量键，`grid_index` 末键 tie-break）；`intent.rs:214` `lex_le`（真字典序逐分量，非加权和标量 min）；`intent.rs:270` `lex_argmin`（foldl pick，平局保留先出现者）。
  - J_x 键构造 `coverage.rs:2181` `j_theta_key`；代表点 `coverage.rs:2205` `pi_theta_position`。
  - 机器证明：`intent.rs:419` `lex_argmin_selects_minimum`、`intent.rs:431` `lex_argmin_tiebreak_by_secondary`、`intent.rs:442` `lex_argmin_empty_none`。
- **(c) 证明状态**：**L0**（字典序全序 + grid_index 升序位次单射 ⟹ 无平局 ⟹ argmin 唯一）+ **机器证明**（选最小 + 次键平局裁决对拍）。对齐 Lean `lexArgmin_exists_unique`（有限非空 + 键单射 ⟹ u\* 存在唯一）。

---

**假设 11：Schedule_Θ 是函数**

- **(a) 数学陈述**：`O_{t+1}=Schedule_Θ(p\*−p_t)`——目标持仓差映射为唯一订单；无交易时返回 Hold/Wait（qty=0），保 ∀x ∃! O。
- **(b) 实装锚点**：`rust/src/theta_v0/strategy/coverage.rs:2239` `schedule_order(p_star, p_t, exec_index) -> Order`（差额 p\*−p_t ⟹ 唯一 Order；qty=0 ⟹ Hold）。函数文档 `coverage.rs:2228` 显式引 §16 假设12「Schedule_Θ 是函数」。
- **(c) 证明状态**：**L0**（纯函数：差额 ⟹ 唯一订单方向/数量）。

---

### §A 结论

11 条假设中：假设 1（bit-exact）、假设 4（六态 partition）、假设 6（互斥）、假设 10（LexArgmin）有**机器证明**看守；假设 1/3/8 附 **L2** 真实数据有效域声明；其余为 **L0** 全函数性质。链复合 ⟹ ∃! O_{t+1}=π_Θ(x)，机器证明看守 `coverage.rs:4457`。

**阶段二闭合（2026-07-03）**：阶段一登记的三处「部分 OPEN」——假设 4 高级别候选生成（#123，`0a35f0167c`）、假设 6 P1..P10 统一（#124，§D-5）、假设 9 毛头寸约束（#133 G7，§D-7）——均已落地并补锚点，∃! 唯一性的有效域从已稳定层扩展至 q2 新增层（完整 §6 z 13 维 + 统一 P1..P10 + gross cap）。残余诚实缺口不影响 ∃! 结论（均为分桶维装配/生产可达性问题，非全函数性破裂），逐项见 §D。

---

## §B. 策略互斥定理（PDF §14）

### 定理陈述

对任意状态 x，定义原始谓词 `P_1(x),…,P_m(x)`（**可重叠**；阶段一锚 alpha2 的 P1..P8 分解 m=8，#124 G5 真统一后按 PDF §7 重分解为 **m=10**——P1 强平 / P2 CloseOverlay / P3 Withdraw / P4 EnterEarning / P5..P7 typed close / P8..P9 open / P10 record，契约锚 `mutex.rs:6-17`；定理与证明对一般 m 成立，实例化随之更新）。固定优先级互斥化：

$$C_1 = P_1,\qquad C_j = P_j \wedge \bigwedge_{k<j}\neg P_k \ (j=2..m),\qquad C_0 = \bigwedge_{j=1}^m \neg P_j.$$

则

$$\sum_{j=0}^m \mathbf{1}[C_j(x)] = 1.$$

（全互斥 + 全定义：任意谓词向量恰好对应一个 C_j。）

### 完整证明（PDF §14 逐字）

设谓词向量 P∈{0,1}^m。分两情形：

1. **无 P_j 成立**（∀j, P_j=0）：则 C_0=⋀¬P_j=1，且每个 C_j（j≥1）含合取项 P_j=0 ⟹ C_j=0。故恰 C_0 成立，Σ=1。
2. **至少一个 P_j 成立**：令 `r=min{j : P_j=1}`（最小成立索引）。则：
   - **C_r=1**：P_r=1（定义）且 ∀k<r, P_k=0（r 最小）⟹ ⋀_{k<r}¬P_k=1 ⟹ C_r=P_r∧1=1。
   - **∀j<r, C_j=0**：j<r ⟹ P_j=0（r 最小）⟹ C_j=P_j∧…=0。
   - **∀j>r, C_j=0**：C_j 含合取项 ¬P_r，而 P_r=1 ⟹ ¬P_r=0 ⟹ ⋀_{k<j}¬P_k=0 ⟹ C_j=0。
   - **C_0=0**：含合取项 ¬P_r=0。

   故恰 C_r 成立，Σ=1。∎

### 机器证明锚点

- **D1 oracle 身份**：`rust/src/theta_v0/strategy/mutex.rs` 模块文档 `mutex.rs:62-68` 声明——`mutex_class` **零生产消费者**（生产裁决走 interp fold + I_Θ 组合层），保留唯一用途＝**P1..P10 等价 property test 的对拍参照**。裁定4 反装饰约束：#124 的 oracle 扩展与生产接线（`f9333e21b2` TW 进 fold + `c4c2a027ad` P1 全局分支 + `8150acb97f` typed close 归因）同批落，P1..P4 谓词均有真实生产分支对应。
- **互斥化实装**：`strategy/mutex.rs:143` `mutex_class(&Predicates) -> MutexClass`——取最小成立谓词索引 r（C_r 唯一）或 C_0（无成立）。构造即证明形式。
- **Σ=1 穷举机器证明**：`strategy/mutex.rs:296` `mutex_total_exhaustive_2pow10`——穷举全部 2^10=1024 谓词组合，**独立重算**每个 `1[C_j]`（不调 `mutex_class`，避免循环论证），逐组合断言 `Σ_j 1[C_j]==1`，并交叉验证命中类号 == `mutex_class` 返回（实装＝独立定义）。全定义兜底 `mutex.rs:333` `mutex_total_definedness`。
- **优先级屏蔽**：`mutex.rs:342` `priority_p1_masks_all`（P1 成立屏蔽 P2..P10 ⟹ C_1）；`mutex.rs:349` `priority_bar_level_chain`（bar 级 P1≻P2≻P3≻P4 链）；`mutex.rs:363` `priority_min_index`（取最小成立索引 ⟹ C_r）。
- **桶级等价（≺_Θ vs P1..P10，12 场景零分叉）**：`strategy/mutex.rs:233` `predicates_of`（生产候选 + fold 状态 → PDF 谓词向量，独立于 interp 规则序**重新推导**每个 P_j，使对拍非循环；typed close 经 `interp.rs:188` `reverse_exit_type` 同一单源）；`mutex.rs:212` `bridge_bucket`（MutexClass → interp 三桶 close/open/record）；`mutex.rs:574` `shadow_fold_bucket_equivalence`——12 场景（生成域覆盖：空/同级同向/同级反向/双向腿/跨级/fold 内重复 slot/ShortDiff/无类无向 + typed close 精确类号）断言 interp `≺_Θ` 桶归属 == mutex P1..P10 桶归属，分叉即红（真矛盾 ⟹ /escalate）；bar 级屏蔽对拍 `mutex.rs:426` `bar_level_ctx_masks_candidate_predicates`（C1..C4 成立 ⟹ bridge_bucket=None，候选无桶归属——与组合层「gamma 全部推迟 record」行为一致）。

### 认识论等级

**L0 结构定理**（非 L2 alpha）：`Σ_j 1[C_j]=1` 是固定优先级互斥化的组合逻辑恒等式（`mutex.rs:53-60` 强制标注：alpha2 Doc2§9 定理1/Doc3§6 构造证毕，零信息增量同义反复）。Rust 穷举 2^10 ＝ **L1 管线正确性**（验证互斥化无 bug，不验证谓词 P_j 经验有效——P_j 触发率/盈利性是 L2 未覆盖；P2/P3/P4 经 #139 裁定 A' + #140 落地后**生产可达 L1**——L0 同价无盈亏版不可达定理保留，见 §D-5 增量与假设 8 有效域声明）。

**边界更新（#124 后 P1 已进对拍范围）**：阶段一「P1 不在 D1 范围」的边界声明已被 §D-5 闭合取代——P1..P4 现为 bar 级谓词（`StepPredicateCtx`），成立时屏蔽全部候选的 P5..P10；对拍锚＝生产分支测试 `coverage.rs:4217/4278/4327/4377`（P1/P2/P3/P4，#140 清单⑤后含 tw=None 订单流对照断言）+ oracle 侧 `mutex.rs:426` bar 级屏蔽。

---

## §C. 各层不变量

### C.1 bit-exact 增量等价 `T^{inc}=T^{full}`

见 §A 假设 1（锚点 parser/fractal.rs:327、stroke.rs:486、profile.rs:240、incremental.rs:116/924）。**机器证明**（合成逐 bar always-run）+ **L2**（真实 CL/BTC，#88/#93 修复史 `c546b5633c`）。

### C.2 AncOK 祖先生命期包含

见 §A 假设 7（锚点 coverage.rs:698/675/740、mod.rs:57-58 不变量 I1–I5）。**L0**——`AncOK` 过滤祖先不齐者 ⟹ `v∈A_{t+1} ⟹ Anc(v)⊆A_{t+1}`。

### C.3 区间套 `J_child ⊆ J_parent`

- **数学陈述**：区间套是**区间包含**非端点相等（PDF §3）——`J_child ⊆ J_parent`，即 `child.lo≥parent.lo ∧ child.hi≤parent.hi`（闭口径）。端点相等仅用于「买卖点挂到本级走势段右端点」，不作跨级 rung 定义。
- **实装锚点**：
  - `rust/src/theta_v0/classifier/nest.rs:62` `is_sub(inner, outer)`：`inner.start_time>=outer.start_time && inner.end_time<=outer.end_time`——子⊆父。
  - `classifier/nest.rs:214` `n_delta_rec` 递归步用 `is_sub(child, top.interval)` 逐级校验 `J^δ_{ℓ-1}⊆J^δ_ℓ`。
  - **L2 bottom-up parity**：`rust/src/theta_v0/backtest/econ_positive.rs:4103` `acc_bottomup_nest_parity_probe`（`#[ignore]`，真实 BTC）——对拍生产 descend（点包含）vs PDF bottom-up（`J_{k-1}⊆I(c)` 子区间包含 + Sel_Θ）；`econ_positive.rs:4251-4252` 断言 `n_some_mismatch==0 && n_gamma_diff==0`（差异全 0 ⟹ 生产点包含 descend ≡ PDF bottom-up 等价）。
- **证明状态**：**L0**（is_sub 闭口径包含谓词，确定）+ **机器证明**（nest.rs:326 is_sub_nesting_bit_exact）+ **L2**（BTC bottom-up parity 差异 0，memory `project_bottomup_nest_equals_pointcontain`：三窗 bit-exact 差异 0，descend 早已非端点相等）。
- **有效域声明**：BTC 全历史实测 95% 退化深度=1（区间套单级），深度≥2 几乎为 0——L2 市场几何事实（"小转大"92%），非实现缺陷。

### C.4 运行时守恒断言（信号漏分类/双分类当场断言失败）

- **数学陈述**：每个候选恰落一桶（open ⊎ record over 候选；close over 活动腿），计数守恒 `|open|+|record|+|关闭触发|=|Γ|`；漏分类/双分类当场断言失败。
- **实装锚点**：
  - `rust/src/theta_v0/strategy/interp.rs:1372` `interpret_buckets_partition_candidates`——环5 互斥分流，`interp.rs:1385` 断言「候选守恒：open + record + 关闭触发 = |Γ|」。
  - `strategy/interp.rs:568` / `interp.rs:574` `debug_assert_eq!(c.tree.as_ref(), &coverage::extract_elements(tower))`——代次快路复用 tree 与全量 extract 逐字节相等（bit-exact 守卫）。
  - `classifier/six_state.rs:179` 六态 partition 非退化断言（六态全可达）。
- **证明状态**：**机器证明**（候选守恒计数 partition + bit-exact debug_assert 守卫）。**L0** 分类穷尽性（六态/三桶 partition）。

### C.5 TW 账本不变量（#140 新形式：非 Realize 守恒 + Realize 漂移 = Σd_pi）

- **数学陈述**：`TwEvent` 八构造子中，**非 Realize 七构造子（含诊断-only `Revalue`）逐事件保 `tw()=free+holding+withdrawn` 守恒**；`Realize(d_pi)` 是**唯一 TW 漂移构造子**，混入任意事件流时终态漂移恰 = Σd_pi（d_pi 可正可负——只入正数会重造利润棘轮，codex A' 推导链第 5 条）。旧不变量「全构造子守恒」经 #139 裁定 A' 分裂为这对新定理（守恒集收窄 + 漂移精确刻画）。
- **实装锚点**：
  - `rust/src/theta_v0/strategy/ledger.rs:364` `TwEvent::Realize(i64)`（第 8 构造子；`ledger.rs:383` `is_legal_from` 归恒合法——OQ-9 门不管它，真约束在 producer/source-validity 唯一资金源 + cash-sound gate 拦负 free，裁定清单②⑦）。
  - 机器证明：`ledger.rs:676` `tw_step_preserves_tw`（前半：七个非 Realize 构造子逐事件守恒）+ `ledger.rs:735` `tw_step_realize_drift_equals_dpi`（后半：漂移恰 = d_pi + 分量正交（stage/legs/hwm_gain 全不变）+ Realize 后 stage 不回退——推导链第 8 条）。
  - 生产端到端：`backtest/runner.rs:2102` `tw_ledger_producer_drift_equals_quantized_realized_pnl`（π fill loop 上 `tw() = ⌊nav0⌋ + ⌊Σ费后已实现PnL⌋`——利润不再凭空消失；旧具名测试断言「恒守恒」按裁定清单⑥重写非删除）。
- **stage 驱动字段白名单/黑名单（p8③ 语义回补禁令的字段级兑现）**：`closed_loop/transition.rs:309-316` `stage_progression` 文档硬边界——**白名单**（允许驱动 stage 判据）：`free`/`holding`/`withdrawn`/`notional_in`/`open_legacy_legs`/`risk_mode`（tw() 三量均成本基口径）；**黑名单**（不得驱动）：`hwm_gain`（未实现峰值，R3 已裁语义回补）/当前 MTM equity/`forced_pnl`（报告用假设强平）/任何未平仓路径依赖浮盈。已实现 PnL 与未实现浮盈的分界＝**平仓 fill 结算事实后续价格不能否定**（`codex-gap3-ledger-20260703.md` 裁定 A' 推导链第 4/6 条，清单⑧）。黑名单字段进判据即触发 A' 边界条件 (b)，须重新提交裁决。
- **证明状态**：**L0**（构造子代数：逐分支检查 tw() 增量）+ **机器证明**（守恒/漂移双测试 + 生产端到端，1452 全绿）。
- **诚实留白（独立证明义务，登记 §D D-8）**：`Origin.TotalWealth.TWEvent`（Lean）**尚无 Realize 对应构造子**——与 `Revalue` 同为 Rust 先行，`ledger.rs:405-407` tw_step 文档已诚实标注不冒充已锚；Lean `twStep_preserves_tw` 的陈述须扩展为「非 Realize 守恒 + Realize 漂移」两定理后契约锚才闭合。

---

## §D. 证明义务登记表（阶段二已闭合，2026-07-03，Task #131）

阶段一登记 7 项义务（6 项 OPEN + D-6 本文档）。q2 六工位（#115/#132/#138/#134/#124/#133）全部落地后，阶段二逐项补实装锚点 + 机器证明锚点 + 认识论等级（所有锚点写前 Read 验证）。**闭合口径**：CLOSED = 该义务的实装与证明锚点已真实存在；条目内的**残余诚实缺口**照实标注为 OPEN 子项，不并入 CLOSED 声明（090号）。阶段一各行的「待锚定状态」保留在 git 历史（`1ce80641ce`），本表为闭合后状态。

| 编号 | 定理陈述 | 闭合状态 | 落地任务 / commit 链 | 影响假设 |
|------|---------|---------|---------------------|---------|
| **D-1 (G1/P2)** | Cand^δ 力度门＝可替换力度签名（支配序，非 MACD 面积一票否决）+ 生产热路由激活 | **CLOSED**（残余 OPEN 子项：A4 缺 TV/SubMovePower） | **#115**：`ec16810bac`→`f575b8fe60`→`c3ed1ccb81`→`ca6f0cd794`→`594d2ea738` | 假设 4（Cand 生成）、假设 6（Weak_Θ 门） |
| **D-2 (G2)** | 完整互斥状态 z 必含 (ℓ,δ,σ_higher) | **CLOSED**（codex-q1 G2 终裁翻转 #81 误读，PDF 权威） | **#132**：`c3ed1ccb81` | 假设 5（状态 z 唯一）有效域 |
| **D-3 (G3)** | z 扩维至 §6 完整形态（13 维 + 全 20 条目逐条裁定）；UClass 并列防碎裂 | **CLOSED**（残余 OPEN 子项：TStage/ηBucket 装配、CostBucket、CandType Live 轴——共 3.5 项） | **#138**：`1eac54ca08`；高级别候选生成 **#123**：`0a35f0167c` | 假设 4（I_γ 高级别生成）、假设 5 |
| **D-4 (G4)** | 统计层出场用 τ^typed 非 τ^reverse | **CLOSED**（τ^reverse 状态机整体删除，不留 fallback） | **#134**：`8150acb97f`→`cd2f326a52`→`3bd26ac3ee`→`99bab5ad68`（+`1a4d931b3e` 剪枝标记） | 假设 5（成本/出场）有效域 |
| **D-5 (G5)** | 解释器统一 P1..P10 固定优先级（强平 + TW/GAP3 纳入单一序） | **CLOSED**（可达性子项亦闭合：#139 裁 A' + #140 落地，P2/P3/P4 生产可达 L1） | **#124**：`c4c2a027ad`→`f9333e21b2`→`972d5cfefa`；**#140**：`f3e3709252`→`49f0821584`→`3987926ba5` | 假设 6（解释器固定优先级）完整域 |
| **D-6 (G6)** | 全定义证明整合文档 | **已交付**（阶段一 #125 + 阶段二 #131 本次更新） | `1ce80641ce` + 本 commit | — |
| **D-7 (G7)** | K_Θ 含最大毛头寸约束（gross exposure cap） | **CLOSED**（default 不激活=frozen bit-exact；#135 须显式激活） | **#133**：`f1c9700332`→`6a282f5f46` | 假设 9（K_Θ 有限）完整域 |
| **D-8** | Lean 侧 TW 不变量扩展：`Origin.TotalWealth.TWEvent` 增 Realize 构造子，`twStep_preserves_tw` 分裂为「非 Realize 守恒 + Realize 漂移」两定理 | **OPEN**（Rust 先行，`ledger.rs:405-407` 已诚实标注不冒充已锚；#140 增量登记） | 待 Lean 工位（与 Revalue 同批） | 假设 8（TW 事件全定义）契约锚 |

**登记表条目数：8**（D-1 至 D-7 + #140 增量登记的 D-8）。**阶段二闭合：6 项全部 CLOSED**；残余 OPEN 项：D-1 一处、D-3 三处半（子项）+ D-8 整项（Lean 侧契约锚）——D-5 可达性子项已由 #139 裁定 A' + #140 落地闭合，不再计入。余项均为数据源不存在/Lean 侧未跟进的诚实缺口，照实登记，关闭条件逐条见下方明细。

### D-1 闭合明细（G1 Cand 力度门——#115 beta-route，结果包 `beta-route-impl-20260703.md`）

- **实装锚点**：
  - 支配序单源：`rust/src/theta_v0/classifier/divergence.rs:358` `ForceProxies::force_state() -> ForceStateA4`——𝒜₄={macd_area, dif_peak, price_amplitude, price_speed} 逐 proxy 支配比较（四态枚举 `divergence.rs:340`；背驰=Dominated=C 在全部 4 proxy 上 ≤A 且至少一个 <，**非 MACD 面积一票否决**；口径冲突=Incomparable 不作背驰确认）。力度签名 `divergence.rs:303` `ForceFeatures`，供 `divergence.rs:474` `weak_theta` 预注册 Θ 口径词典序比较（prereg 三套 Θ 冻结 `71f78103c7`，#112）。
  - 热路由（本义务的原 OPEN 主体）：`rust/src/theta_v0/classifier/bsp.rs:158` `BspPoint.force: Option<ForceProxies>` 旁挂 + `bsp.rs:162` 手写 `impl PartialEq for BspPoint` 排除 force（力度铁律：force 绝不进相等/去重/`class_index`/`BspBits`/分桶 key）；生产热路径单一来源 `classifier/signal.rs:586` `extract_signals_with_hist`（6 参；hist/dif 同一 `compute_macd` 单趟，无额外 O(n)）；级别-N 同源 `classifier/mod.rs:157` `extract_first_third_for_level`。
  - 消费端：`rust/src/theta_v0/backtest/econ_positive.rs:401` `z_of_candidate_with_force(c, p.force, &tower_i, bars, &ext)` ⟹ force_state 进 z 第 8 维（`backtest/mu_estimator.rs:103`）；dx 对拍同源 `econ_positive.rs:3686`。
- **机器证明锚点**：GOLDEN digest 诚实重算 `classifier/signal.rs:1743` `extract_signals_bit_exact_digest_guard`（`signal.rs:1767` GOLDEN=`0x90c7_9ee6_17e1_1392`，翻转原因=Debug 串 force 常量字段；六 bit/pivot/center/struct_break_dir 逐字段不变由 per-case guard 锁定）；反静默断裂断言 `econ_positive.rs:2398`（`n_type1==0 || n_force_some>0`，落 `#[ignore]` L2 批测——force 真实流经处）。
- **认识论等级**：**L0/L1**（支配序是确定性算术，`divergence.rs` 文档明注 L1 管线正确性口径）+ **机器证明**（bit-exact guard + 1436 全绿）。「哪个力度态有 alpha」= L2 未覆盖，归 #135 重跑。
- **残余诚实缺口（OPEN 子项）**：`ForceStateA4` 的 A4 后缀诚实标注只用现有 4 proxy——原文完整 𝒜_ℓ 还含 TV（全变差）与递归次级别力度（SubMovePower）。单调性保证：补维只会把 Dominated/Dominates/Tie 变 Incomparable ⟹ A4 的 Dominated 是完整支配序 Dominated 的**超集（宽判背驰）**（`divergence.rs:327-336` 文档明注）。关闭条件：TV/次级别力度生产者实装后升名 `ForceState`。

### D-2 闭合明细（G2 σ_higher——#132，结果包 `g2-impl-20260703.md`）

- **实装锚点**：`rust/src/theta_v0/backtest/mu_estimator.rs:110` `MuClass.sigma_higher: Option<i8>`（第 9 维；`Some(v)`=塔真值 +1/−1/0，**0 是计算结果非未知**；`None`=裸证书口径诚实 None，231号）；单一来源 `backtest/selector.rs:180` `sigma_higher_at`（z 构造与 666 号信号分解共用同一函数）。
- **类型层护航点（裁定明文，接口强制携塔）**：`selector.rs:212` `z_of_candidate` / `selector.rs:252` `z_of_candidate_with_force` / `selector.rs:288` `filter_gamma` / `selector.rs:311` `filter_gamma_with_admission` 签名强制 `(+tower, +bars)`——「训练表填真值/实盘查询填 None」的静默未见类别退化在**类型层不可构造**（不存在无塔重载；090号拒绝可选参数默认 None 的兼容垫片）。
- **机器证明/防泄漏锚点**：perm 置换桶键不动——`backtest/perm_test.rs:230` fullz 重构闭包显式 `sigma_higher: None`（防经 `..*c` 泄漏 stratum 代表值）；`backtest/wverify_run.rs:388` 判定键同投影（修复本变更激活的全表 miss 陷阱——records 带 Some 而 perm 表键 None）。1436 全绿（`from_certificate` 路径 None 常量分量 ⟹ 分桶基线零翻转，无 μ GOLDEN 重算）。
- **认识论等级**：**L0**（状态键维度定义；PDF §6 权威 > codex #81 误读，codex-q1 G2 终裁）+ **机器证明**（基线不变性）。667 号 L2 实证（per-(ℓ,q) p=0.018 级别依赖调制器）是引入依据，非本条证明内容。

### D-3 闭合明细（G3 z 扩维——#138，结果包 `g3-impl-20260703.md`；高级别候选生成 #123 见 §A 假设 4）

- **实装锚点（MuClass 9→13 维，每维数据源真接，无常量占位）**：
  - `backtest/mu_estimator.rs:120` `cand_channel`（第 10 维，`NestTrigger` 门通道三值，P0-1 单源派生）；
  - `mu_estimator.rs:126` `nest_depth`（第 11 维，=`rungs.len()`；前缀定理 `econ_positive.rs:997` `effective_nest_depth`——n_delta 逐级短路 ⟹ pass 证书深度==rungs.len()；Some(0)=经门基例真值，无门=None 非 0）；
  - `mu_estimator.rs:134` `origin_level`（第 12 维，链顶起始级 ℓ；恒等式 `ℓ=e+Ndepth` debug_assert 于 z_of_candidate）；
  - `mu_estimator.rs:143` `risk_mode`（第 13 维，risk.rs M0-M4——§6 RiskMode 三值细化与 MarginState 离散化的**单字段双覆盖**）。
  - 装配护航点：`backtest/selector.rs:63` `ZExt` + `selector.rs:81` `ZExt::NONE`（无源路径显式声明，可 grep 审计；静默遗漏类型层不可构造）；训练/查询同口径由 runner fill loop 共享 `ext_i` 变量保证（G2 全表 miss 教训的构造性排除）。
- **机器证明锚点**：`selector.rs:516` `g3_ext_dims_assembled_into_z` + `selector.rs:556` `g3_project_to_u_ignores_new_dims`（canonical 全维 vs UClass 降维分层——i_class×δ 共线教训，新维不进 selection 投影）；置换/判定键四新维显式 None（`perm_test.rs:235`、`wverify_run.rs:389-392`）。1438 全绿。
- **§6 全 20 条目对照表**＝`g3-impl-20260703.md` §3（本文档引用不复制）：已接（含前置）11 条；G3 新接 4 字段覆盖 4 条；代数派生/分解承载 2 条（Jchain≅(ℓ,e) 级别签名无独立自由度 / role 分解承载）；诚实缺口 3.5 条（见下）。
- **认识论等级**：**L0**（维度定义+装配恒等式）+ **机器证明**（新测试+基线）；BTC 95.36% 基例退化在 μ̂ 分桶层就此可观测=L2 市场几何事实。
- **残余诚实缺口（OPEN 子项，状态与理由本次更新）**：
  - **TStage（§6 #15）/ ηBucket（§6 #16）**：G3 时点的前提「closed_loop 未接 fill loop」**已被 #124 消除**——生产者已就位（TStage=`runner.rs:680` `TwState.stage` 逐 bar 真值；η=`tw.tw()`，η⋆=`strategy/ledger.rs:264` `eta_star`）。仍 OPEN 的理由（#140 后更新）：装配进 z=动 MuClass 桶键=统计层决策，归 #135 prereg 冻结，不静默扩维。原理由 (b)「stage 恒 CostReduction ⟹ 死维度」已被 #139 裁 A' + #140 落地部分消解——stage 不再结构恒值（L1 可达见证 §D-5），但真实数据上 P3 触发门（已实现利润 ≥ ~1.5×NAV 且再投资足额）的满足频率未知，维度信息量仍待 #135 全历史重跑证实。
  - **CostBucket（§6 #18）**：仍无 bar 级时变成本状态生产者（费率是 run 级常量配置非「状态」，填之即变相常量占位）。关闭条件：动态滑点/冲击成本模型实装。
  - **CandType Live 轴（§6 #6 半条）**：确认-bar 部署架构下候选恒 Settled 口径，Live（未确认活动候选）无生产者——架构口径缺口。翻转条件：异质审计若裁定 PDF 字面三分必须为独立维 ⟹ `cand_channel` 重命名 + Live 生产者升级为实装缺口上浮。

### D-4 闭合明细（G4 typed exit——#134，结果包 `g4-impl-20260703.md`）

- **实装锚点**：
  - 判据单源：`rust/src/theta_v0/strategy/interp.rs:162` `ExitType`（PDF §9 五枚举 {CloseRoot, ReduceCore, CloseShortDiff, RiskExit, Hold}，`e96bfbff32`）+ `interp.rs:188` `reverse_exit_type`（G4/G5 共用：ShortDiff 腿→P7 / 触发类 3→P6 / 一二类→P5；二类归 CloseRoot 经 ws-g5interp 确认，唯一硬约束=单源已满足）。
  - fold 归因本体：`interp.rs:1004` `interpret_with_close_triggers`（close 桶第 k 腿↔第 k 触发候选 fold 内同步 push，非外部重放配对；`interp.rs:990` `interpret` 委托丢归因 ⟹ **签名与 §A ∃! 证明锚不动**，分歧A 裁决）。
  - I_Θ 组合层：`strategy/coverage.rs:2392` `StepTrace`（closed/silent_drops/opened/risk_exits/overlay_closes/tw_event）+ `coverage.rs:2435` `pi_theta_step_traced`（`coverage.rs:2357` `pi_theta_step_prebuilt` 委托 traced——单源组装）。
  - 账本：`backtest/runner.rs:1165` `TypedTrade`（entry_z 经 z_of_candidate 塔真值=与生产 χ 查询同口径；`runner.rs:1184` `via_structural_prune` 结构剪枝可分离标记，`1a4d931b3e`）+ `runner.rs:1044` `typed_ledger_from_bars`（=生产 `runner.rs:616` `pi_theta_fill_loop` 全链 χ≡1，非平行状态机）。
  - 统计管线重接：`backtest/l3_delta_r_alpha.rs:150` `build_mu_from_bars`（签名不变，消费 ledger；**τ^reverse 平行简化状态机整体删除，不留 fallback**，no-patch）。
- **机器证明锚点**：`coverage.rs:4140` `pi_theta_step_traced_opened_and_bitexact`（traced 决策三分量与 prebuilt bit-exact）+ `coverage.rs:4181` `pi_theta_step_traced_reverse_close_attribution`；L2 冒烟 `l3_delta_r_alpha.rs:1888` `typed_ledger_btc_smoke`（`#[ignore]`，BTC 16K bar：18 腿五枚举计数守恒——CloseRoot 6/ReduceCore 11/CloseShortDiff 0/RiskExit 0/Hold 1）。
- **认识论等级**：**L0**（§9 五枚举全定义语义）+ **机器证明**（bit-exact 委托，1435 全绿含全部既有 golden）+ **L2 冒烟**（单标的单窗管线可用性 + 语义量级证据，非 alpha 声明）。
- **★μ 样本坍缩语义（诚实必读，非缺口）**：入场口径随出场同步收敛——μ 观测对象=生产 π **真正开腿**的信号（treatment-on-the-treated；record 桶/slot 冲突/AncOK 被剪候选不再入 μ），样本量下降约两个数量级（16K 半窗：旧≈数百信号 → 新 18 腿）。这是生产 π 语义的忠实镜像非简化；后果：旧口径全部 μ̂/W-VERIFY/L3 数字失效为历史证据，#135 prereg 须按新样本量设计功效声明（n_eff 门槛预期大量桶 ¬powered）。

### D-5 闭合明细（G5 P1..P10 统一——#124，结果包 `g5-impl-20260703.md`）

**逐谓词实装锚点表**（生产两级结构：bar 级 P1..P4 组合层 + 候选级 P5..P10 fold；行号本次 Read 重验）：

| PDF §7 谓词 | 生产实装锚点 | oracle 锚点（mutex.rs Predicates 字段） | 测试见证 |
|---|---|---|---|
| P1 强平 | `coverage.rs:2449`（force_flat 上游短路：RiskExit 全清 + next_active=∅ 幽灵腿堵口） | `mutex.rs:92` `risk_liquidate` | `coverage.rs:4217` `_p1_force_flat_risk_exits_all` |
| P2 CloseOverlay | `coverage.rs:2472`（StageII∧H>0 ⟹ 合成 close 桶**真产订单**进同一 schedule/fill/ledger）；`runner.rs:937`（typed=CloseShortDiff 入 ledger） | `mutex.rs:94` `close_overlay` | `coverage.rs:4278` `_p2_close_overlay`（#140 清单⑤：+tw=None 对照断言 Order 本身不同） |
| P3 Withdraw | `coverage.rs:2510`（`transition.rs:319` `stage_progression` 单源 ⟹ TWEvent_t，**消耗当步裁决**——gamma 全 record，分歧B 裁决） | `mutex.rs:96` `tw_withdraw` | `coverage.rs:4327` `_p3_withdraw_consumes_step`（#140：+tw=None 对照——同 bar 普通开仓 qty >0 变 0） |
| P4 EnterEarning | 同 P3 分支（stage_progression→EnterEarning） | `mutex.rs:98` `tw_enter_earning` | `coverage.rs:4377` `_p4_enter_earning`（#140：+tw=None 对照） |
| P5 CloseRoot | interp fold 规则2 + `interp.rs:188` `reverse_exit_type`（一/二类反向） | `mutex.rs:100` `close_root` | shadow-fold typed 精确类号 |
| P6 ReduceCore | 同上（三类反向） | `mutex.rs:102` `reduce_core` | shadow-fold |
| P7 CloseShortDiff | 同上（ShortDiff 入场角色压过触发类） | `mutex.rs:104` `close_short_diff` | shadow-fold |
| P8 Open Root | interp fold 规则3（非 ShortDiff 角色） | `mutex.rs:106` `open_root` | shadow-fold |
| P9 Open ShortDiff | 同上（ShortDiff 角色） | `mutex.rs:108` `open_short_diff` | shadow-fold |
| P10 Record | interp fold 规则1/4（Flat/无类/slot 冲突） | `mutex.rs:111` `record_struct_break` | shadow-fold |
| P0 Hold | 无谓词命中 ⟹ p\* 不变 | `MutexClass::C0`（`mutex.rs:81`） | `mutex.rs:333` `mutex_total_definedness` |

- **TW 状态单一生产源**：`runner.rs:680` `TwState`（π fill loop 内，由真实交易流驱动——成本基方向差分→ShortDiff 划转 `runner.rs:721`；#140 ②'' 已实现 PnL 量化差分→`Realize(d_pi)` `runner.rs:752`（唯一资金源=apply_order 返回的实际平仓 fill 费后 PnL，forced_pnl 结构性隔离——推导链第 10/11 条）；腿平/开计数 `runner.rs:930/956` `tw_step`，OQ-9 守卫）；`run_closed_loop` 降级纯结构验证工具（裁定4 明文）。
- **机器证明锚点**：2^10 穷举 `mutex.rs:296` + typed shadow-fold `mutex.rs:574`（12 场景）+ bar 级屏蔽 `mutex.rs:426` + TW 生产端到端 `runner.rs:2102` `tw_ledger_producer_drift_equals_quantized_realized_pnl`（#124 原具名测试 `tw_ledger_producer_in_place_and_conserved` 断言「恒守恒」= 正 PnL 凭空消失，按 #139 裁定清单⑥**重写非删除**：新断言 TW 漂移 = ⌊Σ费后已实现PnL⌋）。#124 交付基线 1447 全绿；#140 后 1452 全绿。
- **可达性（#140 增量：FALSIFIED → L1 可达）**：
  - **L0 同价无盈亏不可达定理（限定版保留）**：`runner.rs:2550` `earning_shares_unreachable_l0_same_price_zero_pnl`（原名 `earning_shares_structurally_unreachable_from_campaign_tw_conserved`，按裁定改名+收窄有效域）——同价 ⟹ 平仓 PnL≡0 ⟹ `Realize(0)` 守恒退化 ⟹ 退本金前提 `holding≥notional_in ∧ free≥recover_target` 联立仍要求 `TW ≥ 2·notional_in−withdrawn > TW`（代数矛盾不变）⟹ L0 模型下 stage 恒 CostReduction。原「结构不可达」证明的 TW 守恒代数**在此收窄域内继续成立**；L2 realized-PnL 路径不再被该定理覆盖。
  - **L1 生产可达见证**：`runner.rs:2148` `pi_loop_realized_profit_reaches_earning_shares`——合成变价序列（px 100→1000）上生产 π fill loop 全链（buy1 开仓→sell1 平仓 realized≈+5.4e6 经 `Realize` 入 free→再建仓）⟹ P3 派 `RecoverCapital`（stage II）⟹ P4 `EnterEarning` ⟹ **终态 TStage=EarningShares**，且 TW 漂移恒 = 已实现 PnL 量化和（资金源唯一性端到端审计，无 fixture 直捅 stage）。**GAP3「∃t TStage=III」从 FALSIFIED 翻转 L1 可达**。
  - **bit-exact 边界（#135 消费）**：现有非忽略测试集零 GOLDEN diff（P3 触发门在既有场景不满足）；全历史 L2 重跑中任一 bar 触发 P2/P3/P4 即订单流分叉——两类风险（P2 直接产订单 / P3/P4 masking 间接改同 bar 订单）已在 `gap3-realize-impl-20260703.md` §4 标注，GOLDEN 真重算归 #135。
- **认识论等级**：互斥定理 **L0**；2^10 穷举/shadow-fold = **L1 管线正确性**；L0 同价不可达定理 = **L0 代数结论（收窄域）**；EarningShares 可达性 = **L1**（合成序列生产路径见证）；TW 谓词的真实数据触发频率/盈利性 = L2 未覆盖（#135）。

### D-7 闭合明细（G7 毛头寸约束——#133，结果包 `g7-impl-20260703.md`）

- **实装锚点**：
  - 激活开关：`rust/src/theta_v0/config.rs:162` `RiskConfig.enforce_gross_cap: bool`（default false `config.rs:175`；frozen 冻结测试 `config.rs:295`）。毛 cap 复用 `gamma×base_units`（#122 裁定暂不拆 gross_gamma/net_gamma）。
  - 判定谓词：`strategy/risk.rs:563` `gross_units_cap` + `risk.rs:572` `gross_units_ok`——strict §11 毛杠杆 `L^G≤L̄^G` 在单标的 units 空间的精确等价（`base_units=E/px` ⟹ px 两侧对消）；coverage 不私写同义比较。
  - 缩放语义（codex decide 5b46 方案B）：`strategy/coverage.rs:1329` `apply_gross_cap`——**legs 折叠成净标量之前**逐根子树 KKT water-filling 投影（`min Σ_r A_r(c_r−1)² s.t. Σ_r G_r·c_r ≤ Ḡ, c_r∈[0,1]`，闭式 `c_r=max(0,1−μ/t_r)`；子树内同 c_r 保 κ/depth_weights 比率约束，单根退化等比例）；接线点 `coverage.rs:1716` `coverage_step_from_buckets`（`risk: Option<&RiskConfig>` 参数，生产路径传 Some）。
  - 幽灵腿防护（`6a282f5f46`）：`coverage.rs:1916`——被整体零化（c_r=0/Ḡ=0）的开仓腿不入 next_active（force_flat 幽灵腿同款模式禁止复制）。
- **机器证明锚点**：两空间等价 iff `risk.rs:1141` `gross_units_ok_iff_leverage_ok`（激活/违反/边界三侧）；幽灵腿 `coverage.rs:3155` `gross_cap_zeroed_open_legs_do_not_enter_active_set`（Ḡ=0 ⟹ next_active 空 + p̃=0）；default false 路径不触碰 legs=**构造性 bit-exact**（1428 全绿含全部既有 golden digest 为物证，零 GOLDEN 重算）。
- **认识论等级**：**L0**（KKT 闭式投影推导——K_Θ 成员约束下每根子树只剩整体尺度一个自由度 ⟹ 等权 W 下投影=逐根 water-filling，推导链在 `apply_gross_cap` doc + decide 5b46）+ **机器证明**。
- **诚实声明（激活口径，非缺口）**：default `enforce_gross_cap=false` ⟹ 当前 frozen 口径生产仍无毛 cap——这是 #122「约束未配置=不激活」+ frozen bit-exact 的钦定组合，非遗漏；**#135 π^full 收口跑批必须显式 `enforce_gross_cap=true`，否则 PDF §11 一致性声明不成立**（新 prereg 冻结项）。

---

## §E. 结果包（result-package 六要素）

1. **结论**：本文档整合了全定义互斥策略已稳定层的严格数学证明——§A 全定义唯一性定理（11 假设逐条锚点 + ∃! 机器证明看守）、§B 策略互斥定理（Σ_j 1[C_j]=1 完整证明 + 2^8 穷举 + 9 场景桶级等价机器证明）、§C 四层不变量（bit-exact/AncOK/区间套/守恒断言）。所有 37+ 处代码锚点写前经 Read 验证行号真实存在。

2. **定义依据**：PDF `完整的策略.pdf` §13（11 假设）、§14（互斥）、§2.1（bit-exact）、§2.2/§8（AncOK）、§3（区间套 J_child⊆J_parent）；互斥化数学出处 `买卖点alpha2.pdf` Doc2§5/Doc3§6 定理1。每条假设的输入数据特征（有限结构塔 ⟹ 有限 Γ；lot 网格离散 ⟹ K_Θ 有限；字典序键单射 ⟹ argmin 唯一）满足定义条件已逐条说明。

3. **边界条件**（结论翻转条件）：
   - §A ∃! 翻转：若任一假设的全函数性破裂（如 K_Θ=∅、LexArgmin 键非单射、Schedule 非确定）⟹ ∃! 失效。机器证明 `coverage.rs:3571` 决定性断言是看守。
   - §B Σ=1 翻转：若存在谓词组合 Σ≠1 ⟹ `mutex_total_exhaustive_2pow8` 红；若 interp ≺_Θ 与 P1..P8 分叉 ⟹ `shadow_fold_bucket_equivalence` 红。
   - §C.3 区间套翻转：若生产 descend 与 bottom-up 定位不一致 ⟹ `acc_bottomup_nest_parity_probe` 断言红（现差异 0）。

4. **下游推论**：本文档证明的是**策略 L0 结构性质**（全函数 + 互斥 + 唯一订单），**不蕴含 L2 alpha**——μ(z,a)>0 属 §12 选择器层，实证四口径全 INCONCLUSIVE。系统的经验价值定位＝风险控制（μ̂ 门减损减回撤），非产生正 alpha（`strategy-spec-for-external-review-20260703.md` 六/七节）。阶段二 6 项 OPEN 义务闭合后，∃! 唯一性的有效域从已稳定层扩展至完整 §6 z + 统一 P1..P10。

5. **谱系引用**：formalization-validity-domain 231号（L0/机器证明/L2 分级 + 有效域≠定义域）；no-patch-mentality 090号（声明膨胀禁止——OPEN 项不写「已证」）；memory `project_bottomup_nest_equals_pointcontain`（区间套 bit-exact 差异 0）、`project_gap3_l2_unreachable_architecture`（EarningShares 需 L2 价格升值，假设 8 有效域）、`project_iclass_delta_collinearity_perm_degeneracy`（i_class×δ 共线 ⟹ D-3 UClass 并列必要性）。

6. **影响声明**：新增 `docs/formal-chain/proofs-full-strategy-20260703.md`（本文档，唯一整合文档）；不改任何代码/定义/测试；引用 `rust/src/theta_v0/` 现有 37+ 锚点（全部 Read 验证）。登记 7 条证明义务（6 项 OPEN）供 Lead 调度阶段二（#115/#122/#123/#124）。后续须过 codex + gemini 双异质审（阶段三任务）。

---

## §F. 阶段二结果包（Task #131，简化三要素）

1. **结论**：§D 六项 OPEN 义务全部 CLOSED（D-1/D-2/D-3/D-4/D-5/D-7），每项补实装锚点 + 机器证明锚点 + 认识论等级（闭合明细见 §D）；§A/§B/§C 受 q2 六工位改动影响的既有锚点全部重新 Read 验证并修正行号漂移（interp.rs、coverage.rs、mutex.rs、econ_positive.rs、transition.rs 五文件；mutex.rs 为 #124 全文件重分解 m=8→10，§B 实例化随之更新）；§A 假设 4/6/9 的三处「部分 OPEN」有效域声明更新为「阶段二闭合」。残余 OPEN 子项 5 处照实登记（D-1 A4 缺 TV/SubMovePower；D-3 TStage/ηBucket 装配归 #135+#139、CostBucket 无生产者、CandType Live 轴；D-5 P2/P3/P4 可达性依赖 #139），不并入 CLOSED 声明。

2. **边界条件**（结论翻转条件）：
   - 本次只改锚点/状态/有效域声明，不改任何证明结论的数学内容——§B 定理与证明对一般 m 成立，m=8→10 是 #124 重分解的事实记录；若该重分解被异质审计推翻 ⟹ §B 实例化与 D-5 锚点表须回退。
   - 锚点行号相对本 commit 时点的工作树（三个曾在飞文件 econ_positive.rs/bsp.rs/classifier mod.rs 已确认与 HEAD 一致后才引用）；后续 commit 再漂移属正常演化，翻转条件=任一锚点符号 Read 不存在（非行号偏移）。
   - 各 CLOSED 项的翻转条件继承其结果包边界条件节（如 D-4 codex 原文：若逐笔对照证明 τ^typed≈τ^reverse 则旧口径可复活——BTC 冒烟已初步反证；D-5 (a)：#139 裁通过 ⟹ 订单流非 bit-exact ⟹ GOLDEN 真重算）。

3. **影响声明**：只改本文档（`docs/formal-chain/proofs-full-strategy-20260703.md`），零代码/测试改动。下游消费：**阶段三 codex+gemini 双异质审**审全文档——审前必须告知：假设 1（高级别 bit-exact）的 L2 锚点是 `#[ignore]` 真实数据测试的历史手工跑（commit `c546b5633c`），CI 不自动跑，文档已诚实披露，此为已知验证态边界非声明膨胀（quality-guard 验收注记 2026-07-03）；**#135** 消费 D-4 功效警告、D-7 激活口径（`enforce_gross_cap=true` 冻结项）、D-3 装配归属；**#139** 是 D-5 可达性子项的关闭闸门。

> **增量注记（#140 后，本节正文保留为阶段二时点记录）**：上述「D-5 可达性依赖 #139」已闭合——#139 裁 A' + #140 落地，见文档头增量说明与 §D-5/§C.5。阶段二边界条件「#139 裁通过 ⟹ 订单流非 bit-exact ⟹ GOLDEN 真重算」的实际结果：非忽略测试集**零 GOLDEN diff**（既有场景 P3 触发门不满足），GOLDEN 真重算移交 #135 全历史跑批（两类 bit-exact 风险已在 `gap3-realize-impl-20260703.md` §4 标注）。新增独立义务 D-8（Lean 侧）进登记表。
