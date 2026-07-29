/-
  Origin/LeverageCapital.lean — 杠杆系统（§13 / FULL 十三，port 到 Origin canonical base）
  ★task #54 L4 资本风控层（杠杆五元素：n_v / G_t / N_t / L^G / L^N）

  ── 存在论位置（A′ port，非重证从零）─────────────────────────────────────────
  Origin 六层 standalone 闭包是**唯一 canonical base**（formal/Origin/）。审计：杠杆系统
  （strict §11「杠杆、保证金和风险模式」line 341-359 / FULL 十三 line 975-1006）的**名义头寸
  与杠杆度量** MISSING in Origin——只有 RiskProj 的网格投影和 risk.rs 的风险模式 μ_t，缺
  「有符号名义头寸 n_v → 毛/净敞口 G_t/N_t → 毛/净杠杆 L^G/L^N」这条度量链。本文件把它
  **重锚到 Origin 类型**：复用 Origin `SourceAxioms.Side`（long/short，σ_v 方向域）作声部方向，
  定理挂 Origin 命名空间 `NewChanlun.Origin.LeverageCapital`，**不** import legacy Strict。

  ── canonical 依据（FULL 十三 line 977-1004，逐式）────────────────────────────
    n_{v,t} = σ_v M_v P_{v,t} q_{v,t}              （有符号名义头寸：方向×乘数×价格×手数）
    G_t = Σ_v |n_{v,t}|                            （总/毛名义头寸：各声部名义绝对值之和）
    N_t = |Σ_v n_{v,t}|                            （净名义头寸：各声部名义代数和的绝对值）
    L^G_t = G_t / E_t,  L^N_t = N_t / E_t          （毛杠杆 / 净杠杆：名义除以权益）
  「精确同单位数双开时，净杠杆可能很低，但总杠杆仍然很高，因此必须同时约束二者」
  （FULL 十三 line 1006）——本文件证 N_t ≤ G_t（净 ≤ 毛，三角不等式），坐实「双开抬毛不抬净」。

  ── 认识论等级（formalization-validity-domain 强制标注）──────────────────────
  全部 **L0**（纯定义/代数，不依赖数据）。机器可检验命题：
  - `net_le_gross`：N_t ≤ G_t（净名义 ≤ 毛名义，三角不等式 |Σ| ≤ Σ|·|）。
  - `lev_net_le_gross`：E_t > 0 ⟹ L^N_t ≤ L^G_t（净杠杆 ≤ 毛杠杆，除以正权益保序）。
  - `grossOk_iff_levG_le`：E_t > 0 ⟹ G_t ≤ G̅ ↔ L^G_t ≤ G̅/E_t（比值层与整数名义层接通）。
  - `gross_cap_implies_net_cap`：G_t ≤ Ḡ ∧ Ḡ ≤ N̄ ⟹ N_t ≤ N̄（毛名义帽蕴含净名义帽，若净帽不紧）。
  Lean build 只证这些 L0 代数命题，**不**证杠杆上限经验校准最优（那是 L2 EmpiricalDomain）。

  ── 诚实标注（formalization-validity-domain + no-patch-mentality）────────────
  ★乘数 M_v / 价格 P_v / 权益 E_t / 杠杆上限 L̄^G,L̄^N 全部是 **Θ_leverage / 账户层参数
    （非缠论可导）**；本文件只证给定参数后的 G/N/L^G/L^N 代数关系，不证上限来自缠论。
  ★**多空双开结构保持**：名义聚合**不加** Q⁺Q⁻=0 禁令（净用代数和的绝对值，毛用绝对值之和——
    双开使 G 增、N 可不增，正是「净低毛高」需同时约束的来源，与 VoiceTree 多空双开一致）。
  ★公理足迹例外：`lev_net_le_gross`/`grossOk_iff_levG_le` 的 `#print axioms` 含 `Classical.choice`；
    它来自 Lean core `Rat.div → Rat.mul` 的既有约分实现，非本文件显式调用/用户 axiom；仅改证明无法消除。
  ★still-MISSING（L2）：杠杆上限的真实账户经验校准；本文件只证「给定上限后的约束代数」。

  ── 依赖方向（单向无环，不 import #113 分类血肉、不 import legacy Strict）──────
  LeverageCapital → Origin.SourceAxioms（仅用 Side）。standalone。
  验证：`cd formal && lake env lean Origin/LeverageCapital.lean`。禁 sorry/admit/用户 axiom 声明。

  谱系：FULL 十三 / strict §11 → #96/#97（A′ Origin canonical base）→ 本文件 #54（杠杆港入 Origin）。
-/

import Origin.SourceAxioms

namespace NewChanlun.Origin.LeverageCapital

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 方向符号 σ_v（Side → ℤ 的 ±1 编码）

  有符号名义头寸 n_v = σ_v · M_v · P_v · q_v 中的 σ_v ∈ {-1, +1}。Origin canonical
  方向域是 `Side`（long/short）。本文件把 Side 映到 ℤ 的 ±1：long → +1, short → -1。
  这是「赋格声部方向 → 名义头寸符号」的形式编码（多头名义为正，空头名义为负）。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★方向符号 `sign`（L0）：long → +1, short → -1（σ_v ∈ {±1}）。 -/
def sign : Side → Int
  | Side.long => 1
  | Side.short => -1

/-- ★方向符号非零（L0）：σ_v ≠ 0（多空都有非零名义符号）。 -/
theorem sign_ne_zero (s : Side) : sign s ≠ 0 := by
  cases s <;> decide

/-- ★方向符号绝对值为 1（L0）：|σ_v| = 1（符号只贡献方向，不贡献大小）。 -/
theorem sign_abs_one (s : Side) : (sign s).natAbs = 1 := by
  cases s <;> decide

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 声部名义头寸 n_v + 毛/净敞口 G_t / N_t

  每个声部 v 有：方向 σ_v（Side）、乘数 M_v（合约乘数，正）、价格 P_v（正）、手数 q_v（ℕ）。
  名义大小 |n_v| = M_v · P_v · q_v（≥0），有符号名义 n_v = σ_v · |n_v|（ℤ）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★声部名义数据 `VoicePosition`（L0，FULL 十三 line 977-981）：单声部的名义头寸输入。

  - `side : Side`：声部方向 σ_v（long/short）。
  - `notionalMag : Nat`：名义大小 |n_v| = M_v · P_v · q_v（≥0，已折算为整数最小单位）。

  ★诚实标注：notionalMag = M_v·P_v·q_v 的乘积已在调用方算好（M_v/P_v 是 Θ_leverage/账户层
  参数，q_v 是手数）。本结构承载折算后的名义大小，证聚合关系，不重算乘积。
-/
structure VoicePosition where
  side : Side
  notionalMag : Nat
deriving Repr

/-- ★有符号名义头寸 `signedNotional`（L0，n_v = σ_v · |n_v|）：方向符号 × 名义大小。 -/
def VoicePosition.signedNotional (p : VoicePosition) : Int :=
  sign p.side * (p.notionalMag : Int)

/-- ★名义大小非负（L0）：|n_v| ≥ 0（notionalMag : Nat 自然非负）。 -/
theorem VoicePosition.notionalMag_nonneg (p : VoicePosition) :
    0 ≤ (p.notionalMag : Int) := Int.natCast_nonneg p.notionalMag

/-- ★有符号名义的绝对值 = 名义大小（L0）：|n_v| = M_v·P_v·q_v（符号不改大小）。 -/
theorem VoicePosition.signedNotional_natAbs (p : VoicePosition) :
    p.signedNotional.natAbs = p.notionalMag := by
  unfold VoicePosition.signedNotional
  cases p.side <;> simp [sign]

/--
  ★毛名义头寸 `grossNotional` G_t（L0，FULL 十三 line 985-987）：Σ_v |n_v|。
  各声部名义大小之和（绝对值之和——双开的两腿都计入，故毛敞口高）。
-/
def grossNotional (ps : List VoicePosition) : Nat :=
  (ps.map VoicePosition.notionalMag).sum

/--
  ★净名义头寸 `netNotional` N_t（L0，FULL 十三 line 991-996）：|Σ_v n_v|。
  各声部有符号名义的代数和的绝对值（多空抵消——双开的两腿符号相反，故净敞口可低）。
-/
def netNotional (ps : List VoicePosition) : Nat :=
  ((ps.map VoicePosition.signedNotional).sum).natAbs

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 核心代数：净名义 ≤ 毛名义（双开抬毛不抬净的形式坐实）

  「精确同单位数双开时，净杠杆可能很低，但总杠杆仍然很高」（FULL 十三 line 1006）。
  核心代数命题：N_t = |Σ n_v| ≤ Σ |n_v| = G_t（三角不等式）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★净名义 ≤ 毛名义（L0，核心三角不等式）：N_t ≤ G_t。
  `|Σ_v n_v| ≤ Σ_v |n_v|`——代数和的绝对值不超过绝对值之和。这坐实「双开使毛敞口高于净敞口」
  （多空两腿在 Σ n_v 中抵消，在 Σ|n_v| 中累加），故毛/净必须**同时约束**（单约束净不够）。
-/
theorem net_le_gross (ps : List VoicePosition) :
    netNotional ps ≤ grossNotional ps := by
  unfold netNotional grossNotional
  induction ps with
  | nil => simp
  | cons p ps' ih =>
      simp only [List.map_cons, List.sum_cons]
      calc
        (p.signedNotional + (ps'.map VoicePosition.signedNotional).sum).natAbs
            ≤ p.signedNotional.natAbs + ((ps'.map VoicePosition.signedNotional).sum).natAbs :=
              Int.natAbs_add_le _ _
        _ = p.notionalMag + ((ps'.map VoicePosition.signedNotional).sum).natAbs := by
              rw [p.signedNotional_natAbs]
        _ ≤ p.notionalMag + (ps'.map VoicePosition.notionalMag).sum :=
              Nat.add_le_add_left ih _

/--
  ★双开抬毛不抬净见证（L0）：存在多空两声部，净名义 = 0 但毛名义 = 2k（k>0）。
  精确同单位数双开（long k 手 + short k 手，同名义大小）⟹ N = |k - k| = 0，G = k + k = 2k。
  这具体化 line 1006「净杠杆可能很低（这里 0），但总杠杆仍然很高（这里满额）」。
-/
theorem hedged_gross_high_net_zero (k : Nat) (_hk : 0 < k) :
    ∃ ps : List VoicePosition,
      netNotional ps = 0 ∧ grossNotional ps = 2 * k := by
  refine ⟨[{ side := Side.long, notionalMag := k },
           { side := Side.short, notionalMag := k }], ?_, ?_⟩
  · -- N = |(+k) + (-k)| = |0| = 0
    show (((1 : Int) * (k : Int)) + (((-1 : Int) * (k : Int)) + 0)).natAbs = 0
    have h : ((1 : Int) * (k : Int)) + (((-1 : Int) * (k : Int)) + 0) = 0 := by omega
    rw [h]; rfl
  · -- G = k + k = 2k
    unfold grossNotional
    simp [Nat.two_mul]

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 杠杆度量 L^G / L^N + 杠杆约束（毛/净杠杆上限）

  杠杆 = 名义 / 权益。用有理数（名义 : ℚ / 权益 : ℚ），权益正时除法保序。杠杆约束：
  L^G ≤ L̄^G（毛上限），L^N ≤ L̄^N（净上限）。两上限是 Θ_leverage 参数。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★杠杆账户输入 `LeverageAccount`（L0，FULL 十三 line 1000-1004）。

  - `positions : List VoicePosition`：当前所有声部名义头寸。
  - `equity : Int`：账户权益 E_t（要求 > 0，杠杆分母）。
  - `grossCap : Int`、`netCap : Int`：毛/净杠杆上限对应的**名义上限** G̅ = L̄^G·E、N̅ = L̄^N·E
    （运行时把 L^G ≤ L̄^G ⟺ G ≤ L̄^G·E 化为整数名义比较；`grossOk_iff_levG_le`
    在 E>0 下机器证明 G≤G̅ ↔ L^G≤G̅/E，故当输入 G̅=L̄^G·E 时接通两种约束）。

  ★诚实标注：equity / grossCap / netCap 全是账户/Θ_leverage 层运行时输入（非缠论可导）。
  桥接定理只证给定 grossCap 的等价，不证明 grossCap 的经验值或账户标定本身正确。
-/
structure LeverageAccount where
  positions : List VoicePosition
  equity : Int
  grossCap : Int
  netCap : Int

/-- ★毛敞口（整数名义，L0）：G_t（毛名义头寸 ≥ 0）。 -/
def LeverageAccount.gross (a : LeverageAccount) : Int := (grossNotional a.positions : Int)

/-- ★净敞口（整数名义，L0）：N_t。 -/
def LeverageAccount.net (a : LeverageAccount) : Int := (netNotional a.positions : Int)

/-- ★毛杠杆（精确有理数比值，L0）：L^G_t = G_t / E_t。 -/
def LeverageAccount.levG (a : LeverageAccount) : Rat :=
  (a.gross : Rat) / (a.equity : Rat)

/-- ★净杠杆（精确有理数比值，L0）：L^N_t = N_t / E_t。 -/
def LeverageAccount.levN (a : LeverageAccount) : Rat :=
  (a.net : Rat) / (a.equity : Rat)

/-- ★毛杠杆约束满足（L0）：G_t ≤ G̅（= L^G ≤ L̄^G 的整数名义形式）。 -/
def LeverageAccount.grossOk (a : LeverageAccount) : Prop := a.gross ≤ a.grossCap

/-- ★净杠杆约束满足（L0）：N_t ≤ N̅（= L^N ≤ L̄^N 的整数名义形式）。 -/
def LeverageAccount.netOk (a : LeverageAccount) : Prop := a.net ≤ a.netCap

/-- ★净敞口 ≤ 毛敞口（L0，整数版 net_le_gross 提升）。 -/
theorem LeverageAccount.netLeGross (a : LeverageAccount) : a.net ≤ a.gross := by
  unfold LeverageAccount.net LeverageAccount.gross
  exact_mod_cast net_le_gross a.positions

/--
  ★★净杠杆 ≤ 毛杠杆（L0）：正权益 `E_t > 0` 时，有理数除法保序，
  `L^N_t = N_t / E_t ≤ G_t / E_t = L^G_t`。
-/
theorem lev_net_le_gross (a : LeverageAccount) (hequity : 0 < a.equity) :
    a.levN ≤ a.levG := by
  unfold LeverageAccount.levN LeverageAccount.levG
  rw [Rat.div_def, Rat.div_def]
  exact Rat.mul_le_mul_of_nonneg_right
    (Rat.intCast_le_intCast.mpr a.netLeGross)
    (Rat.le_of_lt (Rat.inv_pos.mpr (Rat.intCast_pos.mpr hequity)))

/--
  ★★毛帽蕴含净帽（条件性，L0）：**若净上限不紧于毛上限**（G̅ ≤ N̅）则 G ≤ G̅ ⟹ N ≤ N̅。
  N ≤ G ≤ G̅ ≤ N̅。这给出「同时约束毛净」的一个充分条件下的层级——但**反向不成立**
  （N ≤ N̅ 不蕴含 G ≤ G̅），故 line 1006「必须同时约束」：净约束**不能**替代毛约束。
-/
theorem gross_cap_implies_net_cap (a : LeverageAccount)
    (hcap : a.grossCap ≤ a.netCap) (hgross : a.grossOk) : a.netOk := by
  unfold LeverageAccount.netOk
  unfold LeverageAccount.grossOk at hgross
  exact Int.le_trans (Int.le_trans a.netLeGross hgross) hcap

/--
  ★★整数毛名义帽 ↔ 精确毛杠杆帽（L0）：正权益时，
  `G_t ≤ G̅ ↔ G_t/E_t ≤ G̅/E_t`。右端的 `G̅/E_t` 在运行时输入满足
  `G̅ = L̄^G·E_t` 时即为毛杠杆上限 `L̄^G`。
-/
theorem grossOk_iff_levG_le (a : LeverageAccount) (hequity : 0 < a.equity) :
    a.grossOk ↔ a.levG ≤ (a.grossCap : Rat) / (a.equity : Rat) := by
  unfold LeverageAccount.grossOk LeverageAccount.levG
  rw [Rat.div_def, Rat.div_def]
  have hinv : 0 < (a.equity : Rat)⁻¹ :=
    Rat.inv_pos.mpr (Rat.intCast_pos.mpr hequity)
  constructor
  · intro h
    exact Rat.mul_le_mul_of_nonneg_right
      (Rat.intCast_le_intCast.mpr h) (Rat.le_of_lt hinv)
  · intro h
    exact Rat.intCast_le_intCast.mp
      (Rat.le_of_mul_le_mul_right h hinv)

/--
  ★★净约束不能替代毛约束（L0，line 1006「必须同时约束二者」的形式坐实）：
  存在账户 a，满足净约束（N ≤ N̅）但**违反**毛约束（G > G̅）——即只约束净不足以控毛。
  构造：双开 long k + short k（k>0），N = 0，G = 2k。取 N̅ = 0（净约束 0 ≤ 0 满足），
  G̅ = 0（毛约束 2k ≤ 0 **违反**，k>0）。故净 OK 但毛非 OK。
-/
theorem net_ok_not_imply_gross_ok :
    ∃ a : LeverageAccount, a.netOk ∧ ¬ a.grossOk := by
  refine ⟨{
    positions := [{ side := Side.long, notionalMag := 1 },
                  { side := Side.short, notionalMag := 1 }]
    equity := 1
    grossCap := 0
    netCap := 0
  }, ?_, ?_⟩
  · -- netOk: N = |(+1)+(-1)| = 0 ≤ 0
    unfold LeverageAccount.netOk LeverageAccount.net netNotional VoicePosition.signedNotional sign
    simp
  · -- ¬ grossOk: G = 1+1 = 2 > 0 = G̅
    unfold LeverageAccount.grossOk LeverageAccount.gross grossNotional
    decide

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 标签声明（formalization-validity-domain gatekeeper，诚实分层）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★杠杆标签 `LeverageTag`（gatekeeper，诚实分层）。
  OperationalSemanticsOnly（名义聚合 + 杠杆度量是操作语义，非走势分类）+
  ThetaLeverageParametric（M_v/上限 是 Θ_leverage 参数，非缠论可导）+
  EmpiricalDomain（杠杆上限经验校准 L2）。
  ★**没有** `TrueCompleteClassification` 构造子——类型层拒绝把杠杆度量标为分类定理。
-/
inductive LeverageTag where
  | OperationalSemanticsOnly
  | ThetaLeverageParametric
  | EmpiricalDomain
deriving DecidableEq, Repr

/-- ★杠杆子类（gatekeeper）：GrossNetExposureGivenThetaLeverage（唯一子类）。 -/
inductive LeverageSubkind where
  | GrossNetExposureGivenThetaLeverage
deriving DecidableEq, Repr

/-- ★杠杆诚实标签包（L0 声明）。 -/
def leverageLabels : List LeverageTag × LeverageSubkind :=
  ([LeverageTag.OperationalSemanticsOnly, LeverageTag.ThetaLeverageParametric,
    LeverageTag.EmpiricalDomain],
   LeverageSubkind.GrossNetExposureGivenThetaLeverage)

/-- ★禁标真完全分类（L0，gatekeeper 见证）：杠杆子类必是 GrossNetExposureGivenThetaLeverage。 -/
theorem leverage_not_true_classification (k : LeverageSubkind) :
    k = LeverageSubkind.GrossNetExposureGivenThetaLeverage := by
  cases k; rfl

end NewChanlun.Origin.LeverageCapital
