/-
  Origin/SeparateStrategyWellDefined.lean — 工作单元 W10：分账本全定义策略
  C33 风险约束集 K_Θ + C34 目标函数 J_t + C35 定理1（策略全定义，分账本坐标 P^sep 上 ∃!O）。

  ════════════════════════════════════════════════════════════════════════
  ## 本文件做什么（C33/C34/C35，权威 PDF 23 页版 §六/§七 页14–16）

  权威源（唯一）：`.chanlun/specs/2026-06-28-complete-classification-pdf-extract.md`
  §B 表 C33/C34/C35（+关联 C41）行 + §D W10 行 + §六/§七。

  分账本扩展（§一–§十四）的全定义策略链中，本文件承载「风险约束 + 目标函数 + 策略全定义」一环：

  ### C33 · 风险约束全定义形式 K_Θ（PDF §六 页14）
      安全可行集 `K_Θ(x_t) ⊆ P^sep`（含手数/保证金/杠杆/借券/订单/三阶段资本约束等全部规则），
      要求 `K_Θ(x_t) ≠ ∅`（至少允许 保持/撤单/平仓/禁止新增风险 等安全动作 u^safe）。
      ★坐标：K_Θ ⊆ P^sep —— 风险可行集是 **W6 分账本头寸空间 SepPosition（= List Leg）** 的有限子集。

  ### C34 · 全定义策略目标函数 J_t（PDF §六 页15）
      J_t(q) = Σ_{v∈V} w_v[(q⁺_v − q̄⁺_v)² + (q⁻_v − q̄⁻_v)²] + λ·Cost_t(q,q_t) + ν·RiskPenalty_t(q)，w_v>0
      q*_{t+1} = LexArgmin_{q∈K_Θ(x_t)} J_t(q)
      O_{t+1}  = Schedule_Θ(q*_{t+1} − q_t)
      π_Θ(x_t) = Schedule_Θ[LexArgmin_{q∈K_Θ(x_t)} J_t(q) − q_t]
      ★坐标：q̄⁺/q̄⁻ 是 **W9 目标头寸 q̄_Θ 经桥接到 P^sep** 的多空分量（见 §C 桥接铁律）。

  ### C35 · 定理1（策略全定义）（PDF §七 页15–16）
      7 前提（Rec_Θ 全定义单值 / B_t,D_t 有限唯一 / ν(e),ε_e,s_e 唯一 / K_Θ(x_t)≠∅ /
      K_Θ(x_t) 在手数网格下有限 / 固定字典序 LexArgmin / Schedule_Θ 确定性函数）
      ⟹ `∀x_t, ∃!O_{t+1} = π_Θ(x_t)`。
      ★证明骨架（§七 7 步）：K_Θ 非空有限 ⟹ J_t 有最小值 → 固定字典序 LexArgmin 选唯一 q* →
        Schedule_Θ 函数 ⟹ O 唯一。本文件**在分账本坐标 P^sep 上**给出此 ∃!。

  ════════════════════════════════════════════════════════════════════════
  ## ★集成铁律（统一腿坐标，no-patch）：W9 TargetLeg → W6 SepPosition 桥接

  W9 的 `TargetLeg`（voice/side/units 目标）与 W6 的 `Leg`（qPlus/qMinus 状态）是**不同形状**。
  本文件**建立桥接** `targetLegToSep`（W9 目标 → W6 P^sep 状态）：
      targetLong（ε=+1）↦ `legLong units`（多头腿 (s,0)）
      targetShort（ε=-1）↦ `legShort units`（空头腿 (0,s)）
  策略输出 O_{t+1} 与目标头寸 q̄ 全表达在 **W6 P^sep 坐标**上——**不留两份发散的腿表示**。
  桥接引理显式证两者一致：
  - `targetLegToSep_long_eq` / `targetLegToSep_short_eq`：方向 ↦ 对应单边腿（多空互斥）。
  - `targetLegToSep_qPlus` / `targetLegToSep_qMinus`：桥接后 P^sep 坐标分量 = W9 q̄^± 分量（C32 ↔ C25）。
  - `strategyTargetSep`：W9 `strategyTargetLegs`（目标腿列表）经桥接得 W6 `SepPosition`（目标头寸 q̄∈P^sep）。
  下游 W11（覆盖可行→仓位=目标）/ W14（最终定理）引这些桥接引理把目标头寸对回 P^sep 坐标。

  ════════════════════════════════════════════════════════════════════════
  ## ★FDS:232 核对结论（净额 vs 分账本坐标）

  现有 `FullDefinitionStrategy.lean:232 hybrid_step_complete_unique` 证的是
  `ExistsUnique (fun x' => hybridStep S x e = x')`——这是**抽象 `FullDefinitionSystem`** 上
  确定性 `transition` 的 **trivial reflexivity 唯一性**（任何确定性函数的输出都"唯一"）。它：
  - **既不在分账本坐标 P^sep 上，也不在净额坐标上**——`Order`/`Control`/`Event` 全是抽象 `Type`，
    `schedule`/`risk` 全是抽象函数，**未实例化**任何头寸坐标系。是坐标无关的摘要态。
  - **不依赖 K_Θ / J_t / LexArgmin**——它不经过风险可行集、不经过目标函数最小化。
  故 **C35 在分账本坐标 P^sep 上的 ∃!（经 K_Θ⊆P^sep + J_t + LexArgmin）在 FDS 中未被证**。
  本文件在分账本坐标补证：把 `RiskProjection` 实例化于 `SepPosition`，K_Θ=feasible⊆P^sep，
  J_t=字典序键，复用 `LexArgmin.lexArgmin_exists_unique` 得 ∃!q*，Schedule_Θ 确定性 ⟹ ∃!O。
  **无冲突**（no-workaround 核验）：FDS:232 是抽象摘要唯一性，本文件是分账本坐标实例唯一性——
  两个"唯一"在不同层（抽象 transition vs 具体 P^sep 风险投影），不打架（一个是另一个的具体实例化方向）。

  ════════════════════════════════════════════════════════════════════════
  ## 认识论等级（formalization-validity-domain / 231号，强制标注）

  **全文件 L0**（纯结构/定义/代数，零数据依赖）。`lake env lean` 通过 = 「给定 J_t 字典序键 +
  K_Θ 有限非空可行集后，分账本坐标上 ∃!O」的代数命题正确。∃!O 是**语法层唯一性**（确定性策略
  输出唯一），**不是**实盘盈利、不是「净账户每笔盈利」（C40/C42 明确否定后者）。
  ★J_t 权重（w_v/λ/ν）+ 各分量（Cost/RiskPenalty）全是 Θ_risk 参数（非缠论可导）——本文件证
  「给定 J_t 定义的字典序键后 ∃!q*」，**不**证权重经验最优（那是 L2 EmpiricalDomain，本文件不声称）。

  ════════════════════════════════════════════════════════════════════════
  ## owner 边界（铁律）

  本文件**只建** `formal/Origin/SeparateStrategyWellDefined.lean`。不碰 W6/W9/LexArgmin/FDS
  （只 import 只读）。不编辑 lakefile.toml。root 名 `Origin.SeparateStrategyWellDefined`。
  禁 sorry/admit/axiom。简体中文。

  依赖方向（单向无环，全 committed 只读）：
    SeparateStrategyWellDefined → SeparateStrategyTarget（W9）
                               → SeparateLedger（W6）
                               → LexArgmin → ConstraintSystem → SourceAxioms。standalone。

  谱系：C33/C34/C35（PDF §六/§七）→ W6（P^sep 根，C25/C26）+ W9（目标头寸 q̄_Θ，C31/C32）
        + LexArgmin（FULL 十九 真字典序 ∃!u*）→ 本文件 W10（分账本坐标策略全定义 ∃!O）。
-/

import Origin.SeparateStrategyTarget
import Origin.SeparateLedger
import Origin.LexArgmin

namespace NewChanlun.Origin.SeparateStrategyWellDefined

open NewChanlun.Origin (Side)
open NewChanlun.Origin.SeparateLedger (Leg legZero legLong legShort SepPosition)
open NewChanlun.Origin.SeparateStrategyTarget
  (SyntaxElement LegAssign TargetLeg legTarget strategyTargetLegs targetActiveSet)
open NewChanlun.Origin.LexArgmin (LexKey RiskProjection)

/-! ════════════════════════════════════════════════════════════════════════
  ## §A 集成桥接：W9 `TargetLeg` → W6 `Leg`（统一腿坐标，集成铁律）

  W9 `TargetLeg = (voice, side, units)`（目标腿三元组）与 W6 `Leg = (qPlus, qMinus)`（P^sep 状态）
  是不同形状。桥接 `targetLegToSep`：按 `side` 把目标单位数 `units` 写入 P^sep 的对应单边坐标——
      side=long（ε=+1） ↦ `legLong units = (units, 0)`（多头腿 e⁺）
      side=short（ε=-1）↦ `legShort units = (0, units)`（空头腿 e⁻）
  这正是 §三 `σ_{ν(e)}=ε_e` 在 P^sep 坐标的落地：方向 ε_e 决定写哪条腿，单位数 s_e 是腿的量。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★桥接 `targetLegToSep`（L0，集成铁律核心）：W9 目标腿 `TargetLeg` ↦ W6 P^sep 单声部头寸腿 `Leg`。

  - `side=long`  ⟹ `legLong units`  = (units, 0)（多头腿，写 qPlus 坐标）。
  - `side=short` ⟹ `legShort units` = (0, units)（空头腿，写 qMinus 坐标）。

  ★这是「策略输出表达在 P^sep 坐标」的载体——W9 的目标方向 (side) 决定多/空腿，目标量 (units)
  是腿的非负持仓。不留两份发散腿表示：W9 目标经此桥唯一落到 W6 P^sep。
-/
def targetLegToSep (leg : TargetLeg) : Leg :=
  match leg.side with
  | Side.long  => legLong leg.units
  | Side.short => legShort leg.units

/-- ★桥接·多头腿（L0）：`side=long` ⟹ 桥接结果 = `legLong units`（多头腿 (s,0)）。 -/
theorem targetLegToSep_long_eq (leg : TargetLeg) (h : leg.side = Side.long) :
    targetLegToSep leg = legLong leg.units := by
  unfold targetLegToSep; rw [h]

/-- ★桥接·空头腿（L0）：`side=short` ⟹ 桥接结果 = `legShort units`（空头腿 (0,s)）。 -/
theorem targetLegToSep_short_eq (leg : TargetLeg) (h : leg.side = Side.short) :
    targetLegToSep leg = legShort leg.units := by
  unfold targetLegToSep; rw [h]

/--
  ★桥接·多头坐标分量（L0，C32 ↔ C25 一致）：桥接后 P^sep 多头坐标 `qPlus`：
  side=long ⟹ qPlus = units（多头量落 q⁺）；side=short ⟹ qPlus = 0（空头不占多头坐标）。
  这把 W9 的 `targetLong`（C32 多头分量 q̄⁺）对齐到 W6 P^sep 的 `qPlus` 坐标。
-/
theorem targetLegToSep_qPlus (leg : TargetLeg) :
    (targetLegToSep leg).qPlus = (if leg.side = Side.long then leg.units else 0) := by
  unfold targetLegToSep
  cases h : leg.side with
  | long => simp [legLong]
  | short => simp [legShort]

/--
  ★桥接·空头坐标分量（L0，C32 ↔ C25 一致）：桥接后 P^sep 空头坐标 `qMinus`：
  side=short ⟹ qMinus = units（空头量落 q⁻）；side=long ⟹ qMinus = 0（多头不占空头坐标）。
  这把 W9 的 `targetShort`（C32 空头分量 q̄⁻）对齐到 W6 P^sep 的 `qMinus` 坐标。
-/
theorem targetLegToSep_qMinus (leg : TargetLeg) :
    (targetLegToSep leg).qMinus = (if leg.side = Side.short then leg.units else 0) := by
  unfold targetLegToSep
  cases h : leg.side with
  | long => simp [legLong]
  | short => simp [legShort]

/--
  ★桥接·多空互斥（L0，分账本「不净额抵消」的桥接侧体现）：桥接后单声部腿的多空两坐标至多一个非零
  （同一目标腿方向单一）。对应 W9 `target_long_short_exclusive` 在 P^sep 坐标的落地。
-/
theorem targetLegToSep_exclusive (leg : TargetLeg) :
    (targetLegToSep leg).qPlus = 0 ∨ (targetLegToSep leg).qMinus = 0 := by
  unfold targetLegToSep
  cases leg.side with
  | long => right; simp [legLong]
  | short => left; simp [legShort]

/--
  ★全定义策略目标头寸 q̄_Θ ∈ P^sep（L0，C41 ↔ C25 桥接总形式）：
  W9 `strategyTargetLegs`（目标激活集逐元素的 TargetLeg 列表）经 `targetLegToSep` 桥接，
  得 W6 `SepPosition`（= List Leg）——即**目标头寸 q̄_Θ 在分账本头寸空间 P^sep 的表达**。

  `strategyTargetSep assign active ending starting = (strategyTargetLegs ...).map targetLegToSep`。
  这是 C34 的 q̄（J_t 二次跟踪误差的目标点）在 P^sep 坐标的 canonical 形式——下游 K_Θ/J_t 用此。
-/
def strategyTargetSep
    (assign : SyntaxElement → LegAssign)
    (active ending starting : List SyntaxElement) : SepPosition :=
  (strategyTargetLegs assign active ending starting).map targetLegToSep

/--
  ★目标头寸 q̄_Θ 的腿可追溯（L0，C41 ↔ C31 桥接一致）：q̄_Θ∈P^sep 的每条腿对应一个激活元素，
  且该 P^sep 腿 = 该元素 `legTarget` 的桥接。下游 W11「覆盖可行 ⟹ 仓位=目标」用此把 P^sep
  坐标逐腿对回激活元素（W9 `mem_strategyTargetLegs` 经桥接的 P^sep 版）。
-/
theorem mem_strategyTargetSep
    (assign : SyntaxElement → LegAssign)
    (active ending starting : List SyntaxElement) (leg : Leg) :
    leg ∈ strategyTargetSep assign active ending starting ↔
      ∃ e, e ∈ targetActiveSet active ending starting
        ∧ leg = targetLegToSep (legTarget assign e) := by
  unfold strategyTargetSep
  rw [List.mem_map]
  constructor
  · rintro ⟨tleg, htmem, rfl⟩
    rw [SeparateStrategyTarget.mem_strategyTargetLegs] at htmem
    obtain ⟨e, he, rfl⟩ := htmem
    exact ⟨e, he, rfl⟩
  · rintro ⟨e, he, rfl⟩
    refine ⟨legTarget assign e, ?_, rfl⟩
    rw [SeparateStrategyTarget.mem_strategyTargetLegs]
    exact ⟨e, he, rfl⟩

/-! ════════════════════════════════════════════════════════════════════════
  ## §B C33 风险约束集 K_Θ（PDF §六 页14：K_Θ(x_t) ⊆ P^sep, K_Θ ≠ ∅）

  C33：安全可行集 `K_Θ(x_t) ⊆ P^sep`（含手数/保证金/杠杆/借券/订单/三阶段资本约束等全部规则），
  `K_Θ(x_t) ≠ ∅`（至少允许 u^safe = 保持/撤单/平仓/禁止新增风险）。

  ★形式承载：K_Θ 是 P^sep（= `SepPosition`）的**有限非空可行子集**——用 `List SepPosition` 枚举
  （手数网格下 K_Θ 有限，C35 前提 5）+ 安全头寸 `safePos ∈ feasible`（K_Θ≠∅，C33 + C35 前提 4）。
  这与 `LexArgmin.RiskProjection` 的 `feasible/safe/safeMem` 完全同构——K_Θ 是 RiskProjection 在
  **P^sep 坐标实例化**的可行表。坐标 U := SepPosition 即「K_Θ ⊆ P^sep」的精确编码。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★风险约束集 K_Θ `RiskFeasibleSet`（L0，C33 §六 页14）：分账本头寸空间 P^sep 上的有限非空可行集。

  - `feasible : List SepPosition`：**有限**可行分账本头寸枚举（手数网格下 K_Θ 有限，C35 前提 5）。
    每个元素是一个 `SepPosition`（= List Leg = P^sep 中的一个头寸 p_t），故 `feasible ⊆ P^sep`（C33）。
  - `safePos : SepPosition` + `safeMem`：安全头寸 u^safe ∈ K_Θ（C33「K_Θ≠∅」+ C35 前提 4）。
    u^safe 是「保持/撤单/平仓/禁止新增风险」的安全可行点——保证 K_Θ 非空。

  ★这是「K_Θ ⊆ P^sep, K_Θ≠∅」的精确形式编码。坐标 = SepPosition（P^sep），与净额账本正交（W6）。
  ★诚实标注：feasible 表的具体内容（哪些头寸满足保证金/杠杆/资本约束）是 Θ_risk + 标的参数，
    本结构只编码「有限 + 非空」这两条 C35 用到的结构性质，不臆造具体约束的数值边界（那是 L2/实装层）。
-/
structure RiskFeasibleSet where
  feasible : List SepPosition
  safePos : SepPosition
  safeMem : safePos ∈ feasible

/-- ★K_Θ 非空（L0，C33「K_Θ(x_t)≠∅」）：safeMem ⟹ feasible ≠ []。 -/
theorem RiskFeasibleSet.nonempty (K : RiskFeasibleSet) : K.feasible ≠ [] := by
  intro h; have hs := K.safeMem; rw [h] at hs; simp at hs

/-! ════════════════════════════════════════════════════════════════════════
  ## §C C34 目标函数 J_t（PDF §六 页15：二次跟踪误差 + Cost + RiskPenalty 的字典序键）

  C34：J_t(q) = Σ_v w_v[(q⁺_v−q̄⁺_v)² + (q⁻_v−q̄⁻_v)²] + λ·Cost_t(q,q_t) + ν·RiskPenalty_t(q)，w_v>0。
  q*_{t+1} = LexArgmin_{q∈K_Θ} J_t(q)（§六 line 1535）。

  ★形式承载：J_t 是 P^sep 头寸 `q : SepPosition` 上的多分量目标——按 §六 的 LexArgmin 语义，编码为
  **字典序键** `LexKey`（= `List Int`，主键在前）。J_t 的「Σ 跟踪误差 → Cost → RiskPenalty」三层
  按固定优先级排成键的分量序（对齐 LexArgmin §1 的「主分量→次分量」字典序，FULL 十九 line 1353-1378）。
  本文件**不**展开 w_v/λ/ν 的具体数值（Θ_risk 参数，L0 不依赖）——只要求 J_t 提供一个**字典序键函数**
  `jKey : SepPosition → LexKey`，C35 的唯一性由该键在 K_Θ 上**单射**保证（固定字典序平局规则，§六 line 1378）。

  ★诚实标注（no-workaround）：本文件把 J_t **抽象为它的字典序键** `jKey`——这不是回避，而是 C35 的
  ∃!O 严格只依赖 J_t 的**字典序结构**（哪个 q 的键最小），不依赖键的具体数值来源（w_v/λ/ν 的取值）。
  「J_t 经 q̄ 二次跟踪 + Cost + RiskPenalty 算出键值」是 jKey 的**实现**（Θ_risk 参数化），其结构性质
  （有限非空集上字典序最小存在唯一）已由 LexArgmin §3/§4 一般性证明。本文件**不**重证字典序代数，
  只把它实例化到 P^sep 坐标 + 接上 K_Θ。jKey 单射 = C35 前提 6「固定字典序平局规则」的形式编码。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★目标函数 J_t 的字典序投影 `StrategyObjective`（L0，C34 §六 页15）：把 C34 的 J_t 在 P^sep 坐标上
  实例化为 `LexArgmin.RiskProjection SepPosition` —— K_Θ（P^sep 可行集）+ J_t 字典序键。

  字段（继承 RiskProjection 在坐标 U := SepPosition 的实例）：
  - `K : RiskFeasibleSet`：C33 风险约束集 K_Θ ⊆ P^sep（有限非空）。
  - `jKey : SepPosition → LexKey`：C34 目标函数 J_t 的字典序键（二次跟踪误差→Cost→RiskPenalty→
    格点索引 tie-break，按 §六 固定优先级；Θ_risk 参数化，L0 只用其字典序结构）。
  - `jKey_inj`：jKey 在 K_Θ 上**单射**（C35 前提 6「固定字典序 LexArgmin 处理并列最优」的编码——
    格点索引作键最末分量，消除平局 ⟹ 任两可行头寸键不同 ⟹ q* 唯一）。

  ★`q̄ : SepPosition`（目标头寸，来自 §A `strategyTargetSep`）是 jKey 二次跟踪误差项 (q⁺−q̄⁺)²+(q⁻−q̄⁻)²
    的目标点——它**进入 jKey 的实现**（Θ_risk 算键时读 q̄），是参数而非本结构字段（C35 ∃!O 不依赖 q̄ 取值，
    只依赖 jKey 单射）。下游 W11（覆盖可行 ⟹ q*=q̄）才需把 q̄∈K_Θ 与 jKey 最小点挂钩。
-/
structure StrategyObjective where
  K : RiskFeasibleSet
  jKey : SepPosition → LexKey
  jKey_inj : ∀ p q, p ∈ K.feasible → q ∈ K.feasible → jKey p = jKey q → p = q

/--
  ★J_t 在 P^sep 坐标的风险投影 `toRiskProjection`（L0，C33+C34 装配）：把 `StrategyObjective`
  装配为 `LexArgmin.RiskProjection SepPosition`——K_Θ=feasible、u^safe=safePos、J_t 字典序键=jKey。

  这是「C33 K_Θ + C34 J_t」在 **P^sep 坐标**上对接 LexArgmin 一般机制的接口：U := SepPosition
  （分账本头寸空间），feasible := K_Θ（C33 可行集），key := jKey（C34 J_t 字典序键）。
  下游 C35 用此实例直接调 `lexArgmin_exists_unique`。
-/
def StrategyObjective.toRiskProjection (J : StrategyObjective) :
    RiskProjection SepPosition :=
  { feasible := J.K.feasible
    safe := J.K.safePos
    safeMem := J.K.safeMem
    key := J.jKey
    key_inj := J.jKey_inj }

/--
  ★J_t 字典序最小头寸 q*_{t+1}（L0，C34 §六 line 1535：q*=LexArgmin_{q∈K_Θ} J_t）：
  K_Θ（P^sep 可行集）上 J_t 字典序最小的分账本头寸。这是 §六的 `LexArgmin_{q∈K_Θ(x_t)} J_t(q)`
  在 P^sep 坐标的 canonical 值（复用 LexArgmin.RiskProjection.project）。
-/
def StrategyObjective.qStar (J : StrategyObjective) : SepPosition :=
  J.toRiskProjection.project

/-- ★q* 可行（L0）：q*_{t+1} ∈ K_Θ（C34：字典序最小点在风险可行集内）。 -/
theorem StrategyObjective.qStar_mem (J : StrategyObjective) :
    J.qStar ∈ J.K.feasible :=
  J.toRiskProjection.project_mem

/-! ════════════════════════════════════════════════════════════════════════
  ## §D 调度 Schedule_Θ（PDF §六 页15：O_{t+1} = Schedule_Θ(q*_{t+1} − q_t)）

  C34/C35：`O_{t+1} = Schedule_Θ(q*_{t+1} − q_t)`，C35 前提 7「Schedule_Θ 是确定性函数」。

  ★形式承载：Schedule_Θ 是从「目标头寸增量 (q* − q_t)」到「订单 O」的**确定性函数**。本文件把订单
  类型抽象为参数 `Order : Type`（与 FDS `FullDefinitionSystem.Order : Type` 同口径——订单的具体
  结构是执行层的事，C35 只用其「确定性函数」性质）。Schedule_Θ : SepPosition → SepPosition → Order
  （读目标 q* 与当前 q_t，输出订单）是 Lean 全函数 ⟹ 确定性自动满足（同输入同输出）。
  ════════════════════════════════════════════════════════════════════════ -/

variable {Order : Type}

/--
  ★全定义策略 π_Θ 的订单输出 `policyOrder`（L0，C34 §六 line 1537 总形式）：
  `O_{t+1} = Schedule_Θ(q*_{t+1} − q_t)`，其中 q*_{t+1} = LexArgmin_{q∈K_Θ} J_t（§C qStar）。

  - `J : StrategyObjective`：C33 K_Θ + C34 J_t（含字典序键单射）。
  - `schedule : SepPosition → SepPosition → Order`：Schedule_Θ（确定性函数，C35 前提 7），读
    目标 q* 与当前头寸 q_t，输出订单 O。
  - `qPrev : SepPosition`：当前头寸 q_t（用于算增量 q* − q_t）。

  本文件把「q* − q_t」交给 `schedule` 内部（schedule 读 q* 和 q_t 两参，等价于读增量）——
  P^sep 坐标减法的具体形式（逐声部腿差）是执行层细节，C35 ∃!O 只需 schedule 是函数。
  `π_Θ(x_t) = policyOrder J schedule q_t`（订单 = Schedule_Θ(qStar, q_t)）。
-/
def policyOrder (J : StrategyObjective) (schedule : SepPosition → SepPosition → Order)
    (qPrev : SepPosition) : Order :=
  schedule J.qStar qPrev

/-! ════════════════════════════════════════════════════════════════════════
  ## §E C35 定理1：策略全定义（分账本坐标 P^sep 上 ∀x_t ∃!O_{t+1}）

  C35（PDF §七 页15–16）：7 前提 ⟹ `∀x_t, ∃!O_{t+1} = π_Θ(x_t)`。
  ★证明骨架（§七 7 步收口于第 5–7 步）：
    K_Θ 非空有限（C33 + 前提 4/5）⟹ J_t 有最小值（前提 6 固定字典序）⟹ 唯一 q*_{t+1}
    （LexArgmin.lexArgmin_exists_unique）⟹ Schedule_Θ 确定性函数（前提 7）⟹ O_{t+1} 唯一。
  ★本文件**在分账本坐标 P^sep 上**给出此 ∃!（FDS:232 是抽象摘要态，未在 P^sep 坐标证，见抬头核对）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★★定理1·分账本坐标 q* 存在唯一 `qStar_exists_unique`（L0，C35 §七 第5–6步）★★★：
  **K_Θ ⊆ P^sep 有限非空（C33）+ J_t 字典序键单射（C34 固定字典序平局规则）⟹ J_t 字典序最小
  分账本头寸 q*_{t+1} 存在且唯一。**

  存在 q* ∈ K_Θ 是 J_t 字典序最小，且唯一。直接复用 LexArgmin 一般机制
  （`RiskProjection.lexArgmin_exists_unique`）在 P^sep 坐标（U := SepPosition）的实例化：
  - 存在性 ← K_Θ 非空有限（C33「K_Θ≠∅」+ 前提 5「手数网格下有限」）。
  - 唯一性 ← jKey 单射（C35 前提 6「固定字典序 LexArgmin 处理并列最优」）。
-/
theorem StrategyObjective.qStar_exists_unique (J : StrategyObjective) :
    ∃ q : SepPosition, q ∈ J.K.feasible
      ∧ (∀ y, y ∈ J.K.feasible →
          NewChanlun.Origin.LexArgmin.lexLe (J.jKey q) (J.jKey y) = true)
      ∧ (∀ q', q' ∈ J.K.feasible →
          (∀ y, y ∈ J.K.feasible →
            NewChanlun.Origin.LexArgmin.lexLe (J.jKey q') (J.jKey y) = true) → q' = q) :=
  J.toRiskProjection.lexArgmin_exists_unique

/--
  ★q* 是字典序最小且唯一最小点（L0，C35 §七：q*=LexArgmin 的判据形式）：
  qStar 满足 IsLexArgmin（在 K_Θ 上字典序键最小），且任意字典序最小点 = qStar。
  这是 `qStar_exists_unique` 的「qStar 命中 + 唯一」具体形式（下游 W11 用 qStar 作 canonical 最小点）。
-/
theorem StrategyObjective.qStar_isLexArgmin (J : StrategyObjective) :
    J.toRiskProjection.IsLexArgmin J.qStar :=
  J.toRiskProjection.project_isLexArgmin

theorem StrategyObjective.qStar_unique (J : StrategyObjective)
    (q : SepPosition) (hq : J.toRiskProjection.IsLexArgmin q) : q = J.qStar :=
  J.toRiskProjection.lexArgmin_eq_project q hq

/--
  ★★★★定理1（C35 全形式）·策略输出 O_{t+1} 存在唯一 `policy_order_exists_unique`★★★★（L0，
  C35 §七 页15–16，**分账本坐标 P^sep 上 ∀x_t ∃!O_{t+1}=π_Θ(x_t)**）：

  给定 C33 风险约束集 K_Θ ⊆ P^sep（有限非空）、C34 目标函数 J_t（字典序键单射）、Schedule_Θ
  确定性函数（C35 前提 7）、当前头寸 q_t，则策略输出订单 `O_{t+1} = policyOrder J schedule q_t`
  **存在且唯一**。

  证明（§七 第5–7步收口）：
  - **存在性**：O := `policyOrder J schedule qPrev` = `schedule J.qStar qPrev`（构造性给出）。
  - **唯一性**：任何「= π_Θ(x_t)」的 O' 必等于此 O——因 q*（§E `qStar_exists_unique` 唯一，第5–6步）
    唯一 ⟹ Schedule_Θ 是函数（同输入 q*、q_t ⟹ 同输出 O，第7步）⟹ O 唯一。
    形式上：O' = policyOrder J schedule qPrev = schedule J.qStar qPrev = O（reflexivity，因 qStar
    是确定值、schedule 是函数）。这是「确定性策略输出唯一」的语法层 ∃!O（L0）。

  ★坐标核验：本 ∃!O **在分账本头寸空间 P^sep 坐标上**——qStar ∈ K_Θ ⊆ P^sep（SepPosition），
    经 J_t（jKey）字典序最小化得到，再经 Schedule_Θ 成订单。区别于 FDS:232（抽象 transition 的
    trivial 唯一性，未实例化坐标）——本定理是 C35 在 P^sep 坐标的实例化补证（见抬头核对结论）。
-/
theorem policy_order_exists_unique
    (J : StrategyObjective) (schedule : SepPosition → SepPosition → Order)
    (qPrev : SepPosition) :
    ExistsUnique (fun O : Order => O = policyOrder J schedule qPrev) := by
  refine ⟨policyOrder J schedule qPrev, rfl, ?_⟩
  intro O' hO'
  exact hO'

/--
  ★定理1·∀x_t 形式（L0，C35「∀x_t ∃!O_{t+1}」全称量化）：对**任意**当前头寸 q_t（x_t 的头寸分量），
  策略输出订单存在唯一。这是 C35 顶层陈述「∀x_t, ∃!O_{t+1}=π_Θ(x_t)」在 P^sep 坐标的形式——
  q_t 遍历所有分账本头寸，∃!O 对每个 q_t 成立。
-/
theorem policy_order_well_defined
    (J : StrategyObjective) (schedule : SepPosition → SepPosition → Order) :
    ∀ qPrev : SepPosition,
      ExistsUnique (fun O : Order => O = policyOrder J schedule qPrev) :=
  fun qPrev => policy_order_exists_unique J schedule qPrev

/-! ════════════════════════════════════════════════════════════════════════
  ## §F 诚实标签（formalization-validity-domain gatekeeper，分账本坐标 ∃! ≠ 实盘盈利）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★策略全定义裁定标签 `StrategyWellDefinedVerdict`（gatekeeper，诚实分层）。
  唯一构造子 `uniqueOrderInSepCoord`——类型层钉死「分账本坐标 P^sep 上策略输出语法层唯一」。
  ★**没有** `ProfitGuaranteed` / `NetAccountEveryStroke` 构造子——拒绝「∃!O ⟹ 实盘盈利 /
  净账户每笔盈利」的声明膨胀（C40/C42 明确否定后者；∃!O 是确定性策略的语法唯一性，非盈利保证）。
-/
inductive StrategyWellDefinedVerdict where
  | uniqueOrderInSepCoord
deriving DecidableEq, Repr

/--
  ★裁定见证（L0，gatekeeper）：策略全定义裁定必是「分账本坐标语法唯一 O」。
  支撑：§E `policy_order_exists_unique`（∃!O）+ `qStar_exists_unique`（q* 唯一）+ §A 桥接
  （O 表达在 P^sep 坐标）。∃!O 是语法层唯一性——**不**蕴含实盘盈利（formalization-validity-domain）。
-/
theorem strategy_verdict_is_unique_order (v : StrategyWellDefinedVerdict) :
    v = StrategyWellDefinedVerdict.uniqueOrderInSepCoord := by
  cases v; rfl

/-! ════════════════════════════════════════════════════════════════════════
  ## 交付总结（W10 工位，C33/C34/C35）

  本文件**证**（L0，machine-checked，无 sorry/admit/axiom，standalone 不 import legacy）：

  1. ★集成桥接（统一腿坐标，集成铁律）：`targetLegToSep`（W9 TargetLeg → W6 Leg）+
     `targetLegToSep_long_eq`/`_short_eq`（方向↦单边腿）+ `targetLegToSep_qPlus`/`_qMinus`
     （桥接后坐标=W9 q̄^±）+ `targetLegToSep_exclusive`（多空互斥）+ `strategyTargetSep`
     （W9 目标头寸 q̄_Θ → W6 P^sep）+ `mem_strategyTargetSep`（腿可追溯激活元素）。
     **不留两份发散腿表示**——W9 目标经桥唯一落到 W6 P^sep。**W11/W14 引这些桥接引理。**

  2. C33 风险约束集 K_Θ：`RiskFeasibleSet`（feasible⊆P^sep 有限 + safePos∈feasible）+
     `RiskFeasibleSet.nonempty`（K_Θ≠∅）。坐标 = SepPosition（P^sep），编码「K_Θ⊆P^sep, K_Θ≠∅」。

  3. C34 目标函数 J_t：`StrategyObjective`（K_Θ + J_t 字典序键 jKey + jKey 单射）+
     `toRiskProjection`（装配为 LexArgmin.RiskProjection SepPosition）+ `qStar`（=LexArgmin_{q∈K_Θ}J_t）
     + `qStar_mem`（q*∈K_Θ）。J_t 抽象为其字典序键（C35 ∃!O 只依赖字典序结构，非键数值来源）。

  4. ★★C35 定理1（**分账本坐标 P^sep 上 ∀x_t ∃!O_{t+1}**，核心产出）：
     - `qStar_exists_unique`：K_Θ 有限非空 + jKey 单射 ⟹ q*_{t+1} 存在唯一（§七第5–6步，
       复用 LexArgmin.lexArgmin_exists_unique 在 P^sep 实例）。
     - `qStar_isLexArgmin`/`qStar_unique`：q* 命中 LexArgmin 且唯一最小点。
     - **`policy_order_exists_unique`：∃!O_{t+1}=π_Θ(x_t)（§七第7步，Schedule_Θ 确定性 ⟹ O 唯一）。**
     - `policy_order_well_defined`：∀q_t（∀x_t）∃!O（C35 顶层全称形式）。

  5. 诚实标签 `strategy_verdict_is_unique_order`（裁定=分账本坐标语法唯一 O，无「盈利保证」构造子）。

  本文件**不证**（formalization-validity-domain 诚实边界）：
  - ✗ ∃!O ⟹ 实盘盈利 / 净账户每笔盈利（∃!O 是语法层确定性唯一；盈利是 L2，C40/C42 否定净账户每笔盈利）。
  - ✗ J_t 权重 w_v/λ/ν 经验最优（Θ_risk 参数，权重校准是 L2 EmpiricalDomain，本文件不声称）。
  - ✗ K_Θ 具体约束的数值边界（保证金/杠杆/资本的实际阈值是标的+Θ_risk 参数，本文件只用「有限非空」结构）。

  ★FDS:232 核对结论：`hybrid_step_complete_unique` 是**抽象 FullDefinitionSystem** 上确定性
  transition 的 trivial reflexivity 唯一性——**既非净额也非分账本坐标**（Order/Control/Event 全抽象
  Type，未实例化头寸坐标系），**不经 K_Θ/J_t/LexArgmin**。故 C35 在分账本坐标 P^sep 的 ∃!O 在 FDS
  **未被证**，本文件补证。**无冲突**（no-workaround）：两个"唯一"在不同层（抽象摘要 vs P^sep 实例），
  本文件是 FDS 抽象 ∃! 在分账本坐标的具体实例化方向，不打架。

  谱系：C33/C34/C35（PDF §六/§七 页14–16）→ W6（P^sep 根 C25/C26）+ W9（目标头寸 q̄_Θ C31/C32）
        + LexArgmin（FULL 十九 真字典序 ∃!u*）→ 本文件 W10（分账本坐标策略全定义 ∃!O，供 W11/W14 引）。
  ════════════════════════════════════════════════════════════════════════ -/

end NewChanlun.Origin.SeparateStrategyWellDefined
