# 全定义互斥策略——严格数学证明整合文档（阶段一：已稳定层）

> 日期 2026-07-03。任务 #125（goal g-20260703T0300Z-full-spec-pi q3 验收项）。
> 源规格＝`docs/formal-chain/完整的策略.pdf`（编排者 7-02 22:50 钦定「严格实装版」，唯一权威）。
> 对照实现基线：`rust/src/theta_v0/`，1402 单元测试全绿。
>
> **编排者双令**：「实现不能是简化的，只能是严格的，并且要附带严格数学证明。」本文档是「严格数学证明」验收项。
>
> **阶段边界（诚实声明，非 TODO 遗留）**：本文档阶段一覆盖**已稳定层**（parser bit-exact / §14 互斥 / AncOK / 区间套 / LexArgmin / Schedule），其证明完整无缺。q2 实装在飞（G1 Cand 力度门 #115 / G3 z 扩维 #122+#123 / G5 解释器统一 #124 / 高级别一三类候选 #123）会新增锚点——这些项登记在 §D「证明义务登记表」，由阶段二任务闭合。

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

**机器证明看守（∃! 结论）**：`rust/src/theta_v0/strategy/coverage.rs:3571` `pi_theta_step_deterministic_unique_order`——同状态两次调用 `pi_theta_step` 产逐字段相等订单（决定性＝全函数的可观测面）。生产入口 `coverage.rs:2146` `pi_theta_step` / `coverage.rs:2024` `pi_theta_position` / `coverage.rs:2058` `schedule_order`。

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
- **(b) 实装锚点**：`rust/src/theta_v0/strategy/interp.rs:163` `assemble_gamma(&Classification) -> Vec<Candidate>`——返回 `Vec`，长度即 |Γ|（有限）。模块文档 `interp.rs:19` 显式声明「Γ(x) 有限（`assemble_gamma` 返回 Vec）」。
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
  - **有效域声明（部分 OPEN）**：类型*分类*函数在 L0 上全定义（穷尽 partition 已证）；但 **ℓ≥2 高级别一/三类买卖点的候选*生成*完备性**是在飞工位 #123（G3），登记于 §D-3。已稳定层＝L0/L1 塔基类型分类，其证明完整；高级别候选生成锚点由阶段二补。

---

**假设 5：𝔠_Θ 是函数**（成本函数）

- **(a) 数学陈述**：交易成本 `C_i` 是状态/成交的确定函数（单边费率 commission+slippage+tax）。
- **(b) 实装锚点**：`rust/src/theta_v0/strategy/exec.rs:73` `apply_fees(base, side, config) -> Tick`（买加卖减，确定）；消费侧 `backtest/metrics.rs` 方向感知 `trade_abs_pnl`（多头 `qty·(exit·(1−fee)−entry·(1+fee))`，成本解耦）。机器证明 `exec.rs:336` `fees_buy_adds_sell_subtracts`。
- **(c) 证明状态**：**L0**（纯函数，输入 base/side/config ⟹ 唯一输出）+ **机器证明**（费用施加对拍）。

---

**假设 6：解释器 I_Θ 使用固定优先级**

- **(a) 数学陈述**：`I_Θ(A_t, Γ_t) = (D_t, O_t, L_t)`——候选按固定优先级 `≺_Θ` 全序裁决为关/开/记录三桶，无二义。等价于原始可重叠谓词 `P_1..P_m` 的固定优先级互斥化（见 §B）。
- **(b) 实装锚点**：`rust/src/theta_v0/strategy/interp.rs:163+` `interpret`（候选按 `theta_key` `≺_Θ` 全序 fold）；固定优先级谓词互斥化 `strategy/mutex.rs:127` `mutex_class`；桶级等价对拍 `strategy/mutex.rs:431` `shadow_fold_bucket_equivalence`（9 场景 interp `≺_Θ` 序 == mutex P1..P8 优先级序，零分叉）。
- **(c) 证明状态**：**机器证明**（§B 互斥定理 + D1 桶级等价 9 场景）。
  - **有效域声明（部分 OPEN）**：本对拍范围＝**候选级 P2..P8**；**P1 风险强平**由 `KThetaRiskGate.force_flat` 上层独立兜（`coverage.rs:1924`），**TW/GAP3 事件**由 GAP3 桥独立——两者尚未统一进 P1..P10 单一固定优先级序（G5 #124）。已稳定层＝候选级 ≺_Θ 三桶固定优先级，证明完整；统一 P1..P10 锚点由阶段二补（§D-5）。

---

**假设 7：AncOK 是函数**（祖先生命期包含，PDF §2.2/§8）

- **(a) 数学陈述**：`AncOK(A)={a∈A : Anc(a)⊆A}`；活动集递归 `A_{t+1}=AncOK[(A_t∖D_t)∪B_t]`。生命期包含不变量：`∀a∈Anc(e), [λ_e,ρ_e]⊆[λ_a,ρ_a]`（父生命期覆盖子生命期，否则子声部漂浮在已结束的父上）。
- **(b) 实装锚点**：
  - `rust/src/theta_v0/strategy/coverage.rs:695` `ancestor_close_by_id`（§13 生产版，按 `parent_id` 结构映射链，跨 bar 稳定）；`coverage.rs:665` `ancestor_close`（M16 索引链版）。
  - `rust/src/theta_v0/strategy/coverage.rs:737` `active_set_step`——`A_{t+1}=AncOK[(A_t∖D_t)∪B_t]`（先关后开＋祖先闭合，immutable 返回新集合）。
  - 不变量 I1–I5：`strategy/mod.rs:57-58`（持久身份/方向不变/parent 是关系非身份/操作父持久/AncOK 作用 persistent set，anc.pdf §7）。
- **(c) 证明状态**：**L0**（`AncOK` 是确定集合运算：过滤祖先不全者，输入 A ⟹ 唯一输出）。祖先不齐的元素被裁掉 ⟹ 保证 `v∈A_{t+1} ⟹ Anc(v)⊆A_{t+1}`（生命期包含不变量的直接推论）。修复史：Q4「LiveDetached 误判 Stale ⟹ depth>0 腿被系统性剪掉」（`mod.rs:57`）。

---

**假设 8：TStage 和 TW 事件全定义**（三阶段资金，PDF §10）

- **(a) 数学陈述**：`TStage∈{CostReduction, CapitalRecovered, EarningShares}`，单向阶段迁移；TW 事件 `{ShortDiff, RecoverCapital, Withdraw, EnterEarning, BuyCore}` 经 OQ-9 门在当前 stage 下合法性判定确定。
- **(b) 实装锚点**：
  - `rust/src/theta_v0/closed_loop/transition.rs:259` `stage_progression`（单向 `CostReduction→CapitalRecovered→EarningShares`，PDF §10 步骤2/3）。
  - `closed_loop/transition.rs:242` `TransitionError::Oq9Illegal{event, stage}`——OQ-9 门（如 EarningShares 阶段开 legacy 腿非法）。
- **(c) 证明状态**：**L0**（阶段迁移＋OQ-9 门是确定状态机）。
  - **有效域声明（诚实边界）**：`closed_loop/state.rs:213`——注资**不**使 L0 同价闭环达 EarningShares；EarningShares 真达需 **L2 价格升值**让已实现利润进 TW（超本 L0 模型事件集）。故 TStage *转移语义*是 L0 全定义；EarningShares 阶段的*可达性*在纯 L0 同价模型下不触达，是数据依赖（L2）而非定义缺陷（memory：`project_gap3_l2_unreachable_architecture`）。

---

**假设 9：K_Θ(x_t)≠∅ 且在手数网格上有限**（风险可行集）

- **(a) 数学陈述**：可行净持仓集 `K_Θ = {lot 对齐的 p ∈ [−cap,+cap]}` 非空且有限；风控约束门收窄可行集（force_flat→{0}）。
- **(b) 实装锚点**：
  - `rust/src/theta_v0/strategy/coverage.rs:1974` `feasible_candidates`——lot 对齐净持仓有限网格，含安全锚 `0.0`（**K_Θ≠∅ 的构造性非空见证**）+ bracketing floor/ceil lot 点（凸跟踪目标在 lot 网格的精确有限表示，非近似）。
  - `coverage.rs:1904` `KThetaRiskGate` + `coverage.rs:1943` `caps`（force_flat→(0,0)；stop_long→禁净多；M2/M3→净幅上限）。
- **(c) 证明状态**：**L0**——非空性由构造性见证 `0` 保证（恒在集中）；有限性由 lot 网格离散 + cap 有界保证。凸性精确：主键 `w(p−p̃)²`（w>0）在 lot 离散区间全局最小必在 `clamp(p̃)` 相邻 lot 点 ⟹ 代表集上 LexArgmin ＝ 全 K_Θ 网格 LexArgmin（精确相等，`coverage.rs:1971`）。
  - **有效域声明（部分 OPEN）**：保证金/强平约束已接（#113，`coverage.rs:1933` no_increase_cap）；**最大毛头寸约束**待核（G7），登记 §D-7。

---

**假设 10：LexArgmin 有固定平局规则**

- **(a) 数学陈述**：`p\* = LexArgmin_{p∈K_Θ} J_x(p)`，J_x 是字典序键（主键 tracking_err ≻ 次键 trade_cost ≻ 三键 risk_penalty ≻ turnover ≻ tie-break grid_index）；固定平局规则 ⟹ 键单射 ⟹ p\* 唯一。
- **(b) 实装锚点**：
  - `rust/src/theta_v0/strategy/intent.rs:197` `JThetaKey`（五分量键，`grid_index` 末键 tie-break）；`intent.rs:214` `lex_le`（真字典序逐分量，非加权和标量 min）；`intent.rs:270` `lex_argmin`（foldl pick，平局保留先出现者）。
  - J_x 键构造 `coverage.rs:2000` `j_theta_key`；代表点 `coverage.rs:2024` `pi_theta_position`。
  - 机器证明：`intent.rs:419` `lex_argmin_selects_minimum`、`intent.rs:431` `lex_argmin_tiebreak_by_secondary`、`intent.rs:442` `lex_argmin_empty_none`。
- **(c) 证明状态**：**L0**（字典序全序 + grid_index 升序位次单射 ⟹ 无平局 ⟹ argmin 唯一）+ **机器证明**（选最小 + 次键平局裁决对拍）。对齐 Lean `lexArgmin_exists_unique`（有限非空 + 键单射 ⟹ u\* 存在唯一）。

---

**假设 11：Schedule_Θ 是函数**

- **(a) 数学陈述**：`O_{t+1}=Schedule_Θ(p\*−p_t)`——目标持仓差映射为唯一订单；无交易时返回 Hold/Wait（qty=0），保 ∀x ∃! O。
- **(b) 实装锚点**：`rust/src/theta_v0/strategy/coverage.rs:2058` `schedule_order(p_star, p_t, exec_index) -> Order`（差额 p\*−p_t ⟹ 唯一 Order；qty=0 ⟹ Hold）。模块文档 `coverage.rs:2050` 显式引 §16 假设12「Schedule_Θ 是函数」。
- **(c) 证明状态**：**L0**（纯函数：差额 ⟹ 唯一订单方向/数量）。

---

### §A 结论

11 条假设中：假设 1（bit-exact）、假设 4（六态 partition）、假设 6（互斥）、假设 10（LexArgmin）有**机器证明**看守；假设 1/3/8 附 **L2** 真实数据有效域声明；其余为 **L0** 全函数性质。链复合 ⟹ ∃! O_{t+1}=π_Θ(x)，机器证明看守 `coverage.rs:3571`。

**部分 OPEN**（阶段二闭合）：假设 4 高级别候选生成（#123）、假设 6 P1..P10 统一（#124）、假设 9 毛头寸约束（G7）——见 §D。**已稳定层的 11 条假设证明本身完整无缺**；OPEN 项是有效域从已稳定层向 q2 新增层的扩展锚点，非已证内容的缺失。

---

## §B. 策略互斥定理（PDF §14）

### 定理陈述

对任意状态 x，定义原始谓词 `P_1(x),…,P_m(x)`（m=8，**可重叠**）。固定优先级互斥化：

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

- **D1 oracle 身份**：`rust/src/theta_v0/strategy/mutex.rs` 模块文档 `mutex.rs:48-57` 声明——`mutex_class` **零生产消费者**（生产 π 走 `interp::interpret` 的 ≺_Θ 三桶），保留唯一用途＝**D1 逐候选级等价 property test 的对拍参照**。
- **互斥化实装**：`strategy/mutex.rs:127` `mutex_class(&Predicates) -> MutexClass`——取最小成立谓词索引 r（C_r 唯一）或 C_0（无成立）。构造即证明形式。
- **Σ=1 穷举机器证明**：`strategy/mutex.rs:226` `mutex_total_exhaustive_2pow8`——穷举全部 2^8=256 谓词组合，**独立重算**每个 `1[C_j]`（不调 `mutex_class`，避免循环论证），逐组合断言 `Σ_j 1[C_j]==1`，并交叉验证命中类号 == `mutex_class` 返回（实装＝独立定义）。
- **优先级屏蔽**：`mutex.rs:277` `priority_p1_masks_all`（P1 成立屏蔽 P2..P8 ⟹ C_1）；`mutex.rs:293` `priority_min_index`（取最小成立索引 r=4 ⟹ C_4）。
- **桶级等价（≺_Θ vs P1..P8，9 场景零分叉）**：`strategy/mutex.rs:174` `predicates_of`（生产候选 + fold 状态 → PDF 谓词向量，独立于 interp 规则序**重新推导**每个 P_j，使对拍非循环）；`mutex.rs:153` `bridge_bucket`（MutexClass → interp 三桶 close/open/record）；`mutex.rs:431` `shadow_fold_bucket_equivalence`——9 场景（S1 空/S2 fold 内重复 slot/S3 反向平仓/S4 同向占用/S5 无类无向/S6 ShortDiff/S7 跨级/S8 双向腿/S9 三桶同刻）断言 interp `≺_Θ` 桶归属 == mutex P1..P8 桶归属，分叉即红（真矛盾 ⟹ /escalate）。

### 认识论等级

**L0 结构定理**（非 L2 alpha）：`Σ_j 1[C_j]=1` 是固定优先级互斥化的组合逻辑恒等式（`mutex.rs:44-46` 强制标注：alpha2 Doc2§5/Doc3§6 证毕，零信息增量同义反复）。Rust 穷举 2^8 ＝ **L1 管线正确性**（验证互斥化无 bug，不验证谓词 P_j 经验有效——P_j 触发率/盈利性是 L2 未覆盖）。

**边界（P1 不在 D1 范围）**：`mutex.rs:56-57`——P1 风险强平由 `KThetaRiskGate.force_flat` 上层独立兜，D1 桶级等价覆盖候选级 P2..P8。P1/GAP3 统一进单一 P1..P10 序是 G5（§D-5）。

---

## §C. 各层不变量

### C.1 bit-exact 增量等价 `T^{inc}=T^{full}`

见 §A 假设 1（锚点 parser/fractal.rs:327、stroke.rs:486、profile.rs:240、incremental.rs:116/924）。**机器证明**（合成逐 bar always-run）+ **L2**（真实 CL/BTC，#88/#93 修复史 `c546b5633c`）。

### C.2 AncOK 祖先生命期包含

见 §A 假设 7（锚点 coverage.rs:695/665/737、mod.rs:57-58 不变量 I1–I5）。**L0**——`AncOK` 过滤祖先不齐者 ⟹ `v∈A_{t+1} ⟹ Anc(v)⊆A_{t+1}`。

### C.3 区间套 `J_child ⊆ J_parent`

- **数学陈述**：区间套是**区间包含**非端点相等（PDF §3）——`J_child ⊆ J_parent`，即 `child.lo≥parent.lo ∧ child.hi≤parent.hi`（闭口径）。端点相等仅用于「买卖点挂到本级走势段右端点」，不作跨级 rung 定义。
- **实装锚点**：
  - `rust/src/theta_v0/classifier/nest.rs:62` `is_sub(inner, outer)`：`inner.start_time>=outer.start_time && inner.end_time<=outer.end_time`——子⊆父。
  - `classifier/nest.rs:214` `n_delta_rec` 递归步用 `is_sub(child, top.interval)` 逐级校验 `J^δ_{ℓ-1}⊆J^δ_ℓ`。
  - **L2 bottom-up parity**：`rust/src/theta_v0/backtest/econ_positive.rs:4018` `acc_bottomup_nest_parity_probe`（`#[ignore]`，真实 BTC）——对拍生产 descend（点包含）vs PDF bottom-up（`J_{k-1}⊆I(c)` 子区间包含 + Sel_Θ）；`econ_positive.rs:4166-4167` 断言 `n_some_mismatch==0 && n_gamma_diff==0`（差异全 0 ⟹ 生产点包含 descend ≡ PDF bottom-up 等价）。
- **证明状态**：**L0**（is_sub 闭口径包含谓词，确定）+ **机器证明**（nest.rs:326 is_sub_nesting_bit_exact）+ **L2**（BTC bottom-up parity 差异 0，memory `project_bottomup_nest_equals_pointcontain`：三窗 bit-exact 差异 0，descend 早已非端点相等）。
- **有效域声明**：BTC 全历史实测 95% 退化深度=1（区间套单级），深度≥2 几乎为 0——L2 市场几何事实（"小转大"92%），非实现缺陷。

### C.4 运行时守恒断言（信号漏分类/双分类当场断言失败）

- **数学陈述**：每个候选恰落一桶（open ⊎ record over 候选；close over 活动腿），计数守恒 `|open|+|record|+|关闭触发|=|Γ|`；漏分类/双分类当场断言失败。
- **实装锚点**：
  - `rust/src/theta_v0/strategy/interp.rs:1246` `interpret_buckets_partition_candidates`——环5 互斥分流，`interp.rs:1259` 断言「候选守恒：open + record + 关闭触发 = |Γ|」。
  - `strategy/interp.rs:518` / `interp.rs:524` `debug_assert_eq!(c.tree.as_ref(), &coverage::extract_elements(tower))`——代次快路复用 tree 与全量 extract 逐字节相等（bit-exact 守卫）。
  - `classifier/six_state.rs:179` 六态 partition 非退化断言（六态全可达）。
- **证明状态**：**机器证明**（候选守恒计数 partition + bit-exact debug_assert 守卫）。**L0** 分类穷尽性（六态/三桶 partition）。

---

## §D. 证明义务登记表（阶段二闭合）

以下项由 q2 实装在飞工位落地后，由**阶段二任务**补锚点闭合。每项列定理陈述 + 待锚定状态 + 负责任务号。**本文档阶段一不声明这些为「已证」——状态一律 OPEN。**

| 编号 | 定理陈述（待证） | 待锚定状态 | 负责任务 | 影响假设 |
|------|-----------------|-----------|---------|---------|
| **D-1 (G1/P2)** | Cand^δ_ℓ 力度门＝可替换力度签名 𝔉_ℓ(s)（ΔP,T,slope,TV,DIF,MACDarea,SubMovePower）三种预注册 Θ 口径，非 MACD 面积一票否决 | ForceStateA4 机器 + prereg 三套 Θ 已冻结（`71f78103c7`）；**热路由未激活**（force_state 生产门仍 MACD 面积 proxy 硬门）；A4 缺 TV/SubMovePower | **#115** beta-route | §A 假设 4（Cand 生成）、假设 6（Weak_Θ 门） |
| **D-2 (G2)** | 完整互斥状态 z 必含 (ℓ,δ,σ_higher)（上级方向与级别联立进状态键） | PDF §6 明令 z 含 σ_higher；codex #81 修正4 裁「σ_higher 不并入 z」——**冲突待重裁**（PDF 权威>codex） | **#122** codex-q1-spec | §A 假设 5（状态 z 唯一）有效域 |
| **D-3 (G3)** | z 扩维至 §6 完整 ~20 维（Ndepth/CandType/ForceState/Jchain/σ_higher/role/posState/shortDiff/H/TStage/ηBucket/RiskMode/CostBucket/MarginState/ExitType/e）；统计层 UClass 并列防碎裂（i_class×δ 共线教训） | 现 MuClass 8 维；~10 维数据源大多在（Ndepth=NestCert 深度/TStage=TW/RiskMode+MarginState=#113/ExitType=typed exit），消费侧接入工程未完 | **#123** hl13-impl | §A 假设 4（I_γ 高级别生成）、假设 5 |
| **D-4 (G4)** | 正规出场用 τ^typed 非 τ^reverse：`X^full_i=δ_i(P_{τ^typed}−P_i)−C_i`；统计层 ResidualTrade 出场时点须 typed exit | π 层 typed 动作在（interp close 桶/K_Θ force_flat）；**统计层 X_i 出场时点是否 typed exit 待核**——若 walk-forward 窗口/反向信号出场则 §15 P7 缺口成立 | **#122** codex-q1-spec | §A 假设 5（成本/出场）有效域 |
| **D-5 (G5)** | 解释器统一 P1..P10 固定优先级：P1 强平（KThetaRiskGate）+ TW/GAP3 事件纳入单一 ≺_Θ 序 | interp ≺_Θ 三桶 + 桶级等价（#94）已证（§B）；P1 强平/TW 事件仍独立兜（#92 显式留界），未统一进 P1..P10 | **#124** G5-interpreter | §A 假设 6（解释器固定优先级）完整域 |
| **D-6 (G6)** | 全定义证明整合文档 | **本文档 = G6 阶段一交付**（已稳定层完整） | **#125（本任务）** | — |
| **D-7 (G7)** | K_Θ 含最大毛头寸约束（gross exposure cap） | LexArgmin+Schedule+K_Θ✅；保证金/强平 #113✅；OQ-9✅；**最大毛头寸约束待核** | **#122** codex-q1-spec | §A 假设 9（K_Θ 有限）完整域 |

**登记表条目数：7**（D-1 至 D-7，其中 D-6＝本文档已交付）。**待阶段二闭合的 OPEN 义务：6 项**（D-1/D-2/D-3/D-4/D-5/D-7）。

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
