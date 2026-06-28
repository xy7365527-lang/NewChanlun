/-
  Origin/SeparateLedger.lean — 分账本头寸空间 P^sep + 净额映射 Net（工作单元 W6）

  ════════════════════════════════════════════════════════════════════════
  ## 本文件做什么：建立分账本头寸空间 P^sep 这一新代数结构（C25/C26）

  权威源（唯一）：`.chanlun/specs/2026-06-28-complete-classification-pdf-extract.md`
  §B 表 C25/C26 行 + §一/§二（23 页权威 PDF 页11–12）。本文件是分账本主线（§一–§十四）
  **依赖的根**——整条分账本链（W7→W8→W12→W13→W14）建立在本文件的 P^sep 上。

  ### C25 · 分账本头寸空间 P^sep（PDF §一 页11–12）
      P^sep = ∏_{v∈V} (R≥0·e⁺_v ⊕ R≥0·e⁻_v)
  每个声部 v 有两个**独立的非负坐标**：多头腿 e⁺_v 与空头腿 e⁻_v。头寸
      p_t = (q⁺_{v,t}, q⁻_{v,t})_{v∈V}
  **关键（区别于净额账本）**：同标的同量多空双开 `(Q,Q) ≡ (0,0)` **不成立**——
  `Q·e⁺_v + Q·e⁻_v` 是两个独立头寸腿，是非零头寸（仅当 Q=0 时才退化为零）。

  ### C26 · 净额映射 Net（PDF §一 页12）
      Net(p_t) = Σ_{v∈V} (q⁺_{v,t} − q⁻_{v,t})
  净额映射把 `Q·e⁺ + Q·e⁻ ↦ 0`（净额下退化）。**分账本语义不做此映射**——
  P^sep 保留两条腿，不经 Net 折叠。Net 是从 P^sep 到 ℤ 的**有损投影**（非单射）。

  ════════════════════════════════════════════════════════════════════════
  ## ★关键产出（下游 W8/W13 自相似递归依赖）：不删父机制

  §十一自相似递归（W13）"子声部开启不删父声部"成立的**代数前提** =
  「`Q·e⁺ + Q·e⁻ ≡ 0`（同股数双开）在 P^sep 是**合法非零状态**，仅在净额映射下退化为 0」。

  本文件把这条性质做成可被下游引用的引理：
  - `hedged_leg_nonzero`：Q>0 ⟹ 双开腿 (Q,Q) ≠ 零腿（P^sep 中保留两腿，不抵消）。
  - `hedged_leg_net_zero`：双开腿 (Q,Q) 的净额 = 0（净额映射下退化）。
  - `hedged_leg_nonzero_but_net_zero`：上两条合一 —— P^sep 非零 ∧ 净额为零（核心反例形状）。
  - `parent_leg_survives_child_open`：父多头腿 (Q,0) 开子空头腿后变 (Q,Q)，父多头分量
    `qPlus` 不减（"子声部开启不删父声部"的代数坐实）。

  ════════════════════════════════════════════════════════════════════════
  ## 认识论等级（formalization-validity-domain / 231号，强制标注）

  **全文件 L0**（纯代数/定义，零数据依赖）。`lake env lean` 通过 = P^sep 的代数结构自洽 +
  净额映射 Net 的退化点（双开 (Q,Q)↦0）在结构上成立。

  ★**C26 净额映射退化点直接对应 230号直积退化谱系**：230号"直积在概率度量下退化为 <3 自由度"
  同构于本文件"双开 (Q,Q) 在净额映射下退化为 0 自由度"——两腿（多/空）在净额 ℤ 上塌缩为单点。
  本文件**只**证 L0 代数结构（P^sep 是直积、Net 是非单射投影、双开腿非零）；**严禁**声明
  分账本"在实盘有效"——那是 L2 EmpiricalDomain，本文件不提供也不可由 L0 推出。

  ════════════════════════════════════════════════════════════════════════
  ## 隔离声明（standalone，关键）

  本文件自包含：**不 import** `Origin.LedgerBridge`（净额账本 R=Π-A-W）、`Origin.TotalWealth`
  （取本金三阶段 TW）、`Origin.LeverageCapital`，也不 import 任何 legacy。P^sep 是与上述账本
  **正交的新代数结构**——分账本头寸空间承载"每声部多空两独立腿"，与净额/财务/杠杆账本不同层。
  纯 Prop/Type，不依赖 Mathlib（Nat/Int 取自 Lean core）。禁 sorry/admit/axiom。

  ## P^sep 与净额账本的关系（no-workaround：非冲突，是层分离）
  P^sep（本文件）与 `LedgerBridge` 净额账本**不冲突**——它们是不同有效域的不同结构：
  - 净额账本（LedgerBridge / LeverageCapital netNotional）：度量净敞口 |Σ n_v|，双开后净可低；
  - 分账本 P^sep（本文件）：保留每声部多/空两独立腿，双开后两腿都在、不抵消。
  净额映射 Net 正是从 P^sep 投影到净额视角的桥——本文件证 Net 有损（非单射），故
  "用净额坐标消解分账本语义"在结构上不可行（投影丢信息）。这与 230号一致：直积的有效域
  在净额度量下退化，但直积结构本身（P^sep）在定义域上代数成立。

  谱系：C25/C26（PDF §一）→ 230号（直积退化：双开 (Q,Q)↦0 净额退化）→ 本文件 W6
        （P^sep 根，建立不删父代数前提供 W13 自相似递归引用）。
-/

namespace NewChanlun.Origin.SeparateLedger

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 头寸腿 Leg（单声部的多空两独立坐标，R≥0·e⁺_v ⊕ R≥0·e⁻_v）

  C25 中每个声部 v 的头寸是直积因子 `(R≥0·e⁺_v ⊕ R≥0·e⁻_v)` 的一个元素——一对非负实数
  `(q⁺_v, q⁻_v)`。本文件用 `Nat` 承载 R≥0（非负实数的整数最小单位摘要，与参考文件
  `LeverageCapital.VoicePosition.notionalMag : Nat` 同口径）。

  ★诚实标注：q⁺/q⁻ 用 Nat 承载手数/股数（R≥0 的整数最小单位）。这是 L0 结构层——
  不臆造价格，不承载实盘盈亏（那是 L2/L3）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★头寸腿 `Leg`（L0，C25 单声部双坐标）：声部 v 的多头腿 + 空头腿。
  - `qPlus  : Nat`：多头坐标 q⁺_v（e⁺_v 方向的非负持仓量）。
  - `qMinus : Nat`：空头坐标 q⁻_v（e⁻_v 方向的非负持仓量）。

  ★**两坐标独立**——这正是"多空作独立头寸坐标"的形式载体。`(Q,Q)` 与 `(0,0)` 当且仅当
  Q=0 时相等（逐分量相等，DecidableEq），故 Q>0 的同股数双开是**非零**头寸腿。
-/
structure Leg where
  qPlus : Nat
  qMinus : Nat
deriving DecidableEq, Repr

/-- ★零腿 `legZero`（L0）：(q⁺,q⁻) = (0,0) —— 该声部无任何持仓（多空腿皆空）。 -/
def legZero : Leg := { qPlus := 0, qMinus := 0 }

/-- ★多头腿 `legLong q`（L0，q·e⁺_v）：纯多头持仓 (q, 0)。 -/
def legLong (q : Nat) : Leg := { qPlus := q, qMinus := 0 }

/-- ★空头腿 `legShort q`（L0，q·e⁻_v）：纯空头持仓 (0, q)。 -/
def legShort (q : Nat) : Leg := { qPlus := 0, qMinus := q }

/-- ★对冲腿 `legHedged q`（L0，q·e⁺_v + q·e⁻_v）：同股数多空双开 (q, q)。 -/
def legHedged (q : Nat) : Leg := { qPlus := q, qMinus := q }

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 分账本头寸空间 P^sep（声部族上的直积）

  C25：P^sep = ∏_{v∈V} (R≥0·e⁺_v ⊕ R≥0·e⁻_v)。声部族上的直积——每声部一条 `Leg`。
  用 `List Leg` 承载有限声部族（声部树有限，对齐参考文件 `LeverageCapital.grossNotional`
  用 `List VoicePosition`）。`SepPosition` 的相等是逐声部 `Leg` 相等（DecidableEq 继承）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★分账本头寸 `SepPosition`（L0，C25 直积元素）：声部族上每声部一条头寸腿。
  `p_t = (q⁺_{v,t}, q⁻_{v,t})_{v∈V}` —— 有限声部族的多空双坐标族。
-/
abbrev SepPosition := List Leg

/-- ★空分账本头寸 `sepZero`（L0）：所有声部腿皆零（无任何持仓的 P^sep 原点）。 -/
def sepZero (n : Nat) : SepPosition := List.replicate n legZero

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 净额映射 Net（C26，从 P^sep 到 ℤ 的有损投影）

  C26：Net(p_t) = Σ_{v∈V} (q⁺_{v,t} − q⁻_{v,t})。把分账本头寸投影到净额（单一 ℤ 值）。
  关键性质：Net 把 `Q·e⁺ + Q·e⁻ ↦ 0`（双开退化）。**分账本语义不做此映射**——P^sep 的
  相等不经过 Net；本文件证 Net **非单射**（不同 P^sep 头寸可同净额），坐实"净额映射有损"。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★单腿净额 `legNet`（L0，C26 单声部）：q⁺_v − q⁻_v（多头记正、空头记负）。
  用 ℤ 承载（差可负）。`legNet (legHedged Q) = 0` —— 双开腿净额退化为 0。
-/
def legNet (l : Leg) : Int := (l.qPlus : Int) - (l.qMinus : Int)

/--
  ★分账本头寸净额 `Net`（L0，C26）：Σ_v (q⁺_v − q⁻_v)。
  各声部单腿净额的代数和——多空在净额上抵消（双开两腿符号相反，故净额可塌缩）。
-/
def Net (p : SepPosition) : Int := (p.map legNet).sum

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 ★★不删父机制（W13 自相似递归的代数前提，核心产出）

  §十一自相似递归（W13）"子声部开启不删父声部"成立的代数前提：
  `Q·e⁺ + Q·e⁻ ≡ 0` 在 P^sep 是**合法非零状态**，仅在净额映射下退化为 0。

  本节把这条性质做成可被下游引用的引理（引理名见 docstring 标注）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★不删父引理①·双开腿非零 `hedged_leg_nonzero`（L0，W13 引用）：
  Q > 0 ⟹ 同股数双开腿 `(Q,Q)` 在 P^sep 中**不等于**零腿 `(0,0)`。

  这是"子声部开启不删父声部"的代数核心：同股数多空双开**保留两条腿**（qPlus=Q≠0），
  P^sep 中是非零状态——不像净额账本那样把 (Q,Q) 抹成 0。
-/
theorem hedged_leg_nonzero (Q : Nat) (hQ : 0 < Q) :
    legHedged Q ≠ legZero := by
  intro h
  -- 若 (Q,Q) = (0,0)，则 qPlus 分量 Q = 0，与 0 < Q 矛盾
  have hp : (legHedged Q).qPlus = (legZero).qPlus := by rw [h]
  simp only [legHedged, legZero] at hp
  omega

/--
  ★★不删父引理②·双开腿净额退化 `hedged_leg_net_zero`（L0，C26 退化点，W13 引用）：
  同股数双开腿 `(Q,Q)` 的净额 `legNet = 0`（任意 Q，含 Q>0）。

  这正是 C26 净额映射 `Q·e⁺ + Q·e⁻ ↦ 0` 的单声部形式——**净额视角下双开退化**。
  ★直接对应 230号直积退化：两腿（多/空）在净额 ℤ 上塌缩为单点 0。
-/
theorem hedged_leg_net_zero (Q : Nat) :
    legNet (legHedged Q) = 0 := by
  simp only [legNet, legHedged]
  omega

/--
  ★★不删父引理③·P^sep 非零但净额为零 `hedged_leg_nonzero_but_net_zero`（L0，**W13 主引用**）：
  Q > 0 ⟹ 双开腿 `(Q,Q)` 在 P^sep 中非零（≠ 零腿）**但**净额为 0。

  这是分账本语义与净额语义**分离点**的精确陈述：
  - **分账本视角（P^sep）**：(Q,Q) 是合法非零状态（两腿都在）⟹ 子开不删父。
  - **净额视角（Net）**：(Q,Q) 退化为 0 ⟹ 价格方向暴露已抵消（C40 不可能定理的根）。
  W13 自相似递归归纳步引此引理：因 P^sep 允许 (Q,Q)≢0，子声部开启时父声部腿不被删除。
-/
theorem hedged_leg_nonzero_but_net_zero (Q : Nat) (hQ : 0 < Q) :
    legHedged Q ≠ legZero ∧ legNet (legHedged Q) = 0 :=
  ⟨hedged_leg_nonzero Q hQ, hedged_leg_net_zero Q⟩

/--
  ★★不删父引理④·父腿在子开后存活 `parent_leg_survives_child_open`（L0，W13 直接引用）：
  父声部持多头腿 `(Q,0)`（父方向 long、Q 股）。在该声部坐标上开同股数子空头腿后，
  腿变为 `(Q,Q)`（多头分量 qPlus 不变，新增空头分量 qMinus=Q）。父多头分量 `qPlus = Q`
  **不减**（子空头不吞父多头）——演化 `(σQ,0)→(σQ,-σQ)` 在分账本坐标的形式（PDF §五 页14）。

  这是 "(σQ,0)→(σQ,-σQ)：父仓不减子仓独立建立" 的代数坐实：子腿写在**独立的 qMinus 坐标**，
  父腿 qPlus 不被触碰。下游 W13 用此证"子声部开启不删父声部"。
-/
theorem parent_leg_survives_child_open (Q : Nat) :
    (legHedged Q).qPlus = (legLong Q).qPlus
    ∧ (legHedged Q).qMinus = Q :=
  ⟨rfl, rfl⟩

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 净额映射有损（Net 非单射，"分账本不做此映射"的结构坐实）

  C26："分账本语义不做此映射"。本节证 Net 是**有损投影**（非单射）——故"用净额坐标
  消解分账本语义"在结构上不可行（投影丢失多/空两腿的区分信息）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★净额映射非单射 `net_not_injective`（L0，C26 有损性，**反退化防火墙**）：
  存在两个**不同**的分账本头寸 `p₁ ≠ p₂`，净额相同 `Net p₁ = Net p₂`。

  构造：单声部头寸 `[legHedged 1] = [(1,1)]`（双开）vs `[legZero] = [(0,0)]`（空仓）——
  二者在 P^sep 中不同（(1,1) ≠ (0,0)），净额都为 0。故 Net 把不同 P^sep 态塌缩为同一净额，
  **非单射**。这坐实"净额映射有损"：下游若用净额坐标表示分账本头寸，丢失双开/空仓的区分
  ⟹ "用净额消解分账本语义"不可行（=230号直积退化在净额度量下的具体机制）。
-/
theorem net_not_injective :
    ∃ (p₁ p₂ : SepPosition), p₁ ≠ p₂ ∧ Net p₁ = Net p₂ := by
  refine ⟨[legHedged 1], [legZero], ?_, ?_⟩
  · -- [(1,1)] ≠ [(0,0)]：列表头元素 (1,1) ≠ (0,0)
    intro h
    have hhead : legHedged 1 = legZero := List.head_eq_of_cons_eq h
    exact hedged_leg_nonzero 1 (by omega) hhead
  · -- Net [(1,1)] = 1-1 = 0 = Net [(0,0)]
    simp only [Net, legNet, legHedged, legZero, List.map_cons, List.map_nil, List.sum_cons,
      List.sum_nil]
    omega

/--
  ★净额映射在多头腿上忠实 `net_long_faithful`（L0，对照）：纯多头腿 `(q,0)` 的净额 = q。
  净额映射只在**无对冲腿**时无损——一旦双开，多/空信息在净额上抵消（net_not_injective）。
  这界定 Net 的有效域：净额视角只对单向持仓忠实，对双开持仓退化（C26 的精确边界）。
-/
theorem net_long_faithful (q : Nat) :
    Net [legLong q] = (q : Int) := by
  simp only [Net, legNet, legLong, List.map_cons, List.map_nil, List.sum_cons, List.sum_nil]
  omega

/-! ════════════════════════════════════════════════════════════════════════
  ## §6 P^sep 直积结构（C25 直积语义 + 双坐标独立性）

  C25：P^sep 是声部族上的直积，每因子是 `R≥0·e⁺_v ⊕ R≥0·e⁻_v`。本节坐实直积/独立坐标的
  基本结构性质（聚合可加、双坐标正交）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★净额可加 `net_append`（L0，直积聚合）：两段声部族拼接的净额 = 各段净额之和。
  `Net (p₁ ++ p₂) = Net p₁ + Net p₂`——直积因子上的净额按声部线性叠加（C26 求和的结合性）。
-/
theorem net_append (p₁ p₂ : SepPosition) :
    Net (p₁ ++ p₂) = Net p₁ + Net p₂ := by
  simp only [Net, List.map_append, List.sum_append]

/--
  ★空仓净额为零 `net_zero`（L0）：所有声部腿皆零 ⟹ 净额为 0（P^sep 原点的净额）。
-/
theorem net_sepZero (n : Nat) : Net (sepZero n) = 0 := by
  simp only [Net, sepZero, List.map_replicate]
  have hlz : legNet legZero = 0 := by simp only [legNet, legZero]; omega
  rw [hlz]
  simp

/--
  ★双坐标独立 `leg_coords_independent`（L0，C25 ⊕ 的直和独立性）：多头坐标与空头坐标
  互不影响——构造 `(a,b)` 时，qPlus 只由 a 决定、qMinus 只由 b 决定。
  这是 `R≥0·e⁺_v ⊕ R≥0·e⁻_v` 直和的形式：两坐标正交，双开（a=b=Q）不使任一坐标归零。
-/
theorem leg_coords_independent (a b : Nat) :
    ({ qPlus := a, qMinus := b } : Leg).qPlus = a
    ∧ ({ qPlus := a, qMinus := b } : Leg).qMinus = b :=
  ⟨rfl, rfl⟩

/-! ════════════════════════════════════════════════════════════════════════
  ## §7 诚实标签（formalization-validity-domain gatekeeper，分账本≠净额）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★分账本裁定标签 `SeparateLedgerVerdict`（gatekeeper，诚实分层）。
  唯一构造子 `separateLegsNotNetted`——类型层钉死「分账本保留多空两独立腿，**不**做净额映射」。
  ★**没有** `NettedToScalar` / `MergedToNet` 构造子——拒绝"把 P^sep 折成净额标量"的声明膨胀
  （净额映射 Net 有损，net_not_injective 否证之；这正是 230号直积退化的防火墙）。
-/
inductive SeparateLedgerVerdict where
  | separateLegsNotNetted
deriving DecidableEq, Repr

/--
  ★裁定见证（L0，gatekeeper）：分账本裁定必是「保留独立腿，不净额折叠」。
  三组性质共同支撑：§4 不删父（双开腿非零）+ §5 净额有损（Net 非单射）+ §6 直积独立坐标。
-/
theorem separate_ledger_verdict_is_not_netted (v : SeparateLedgerVerdict) :
    v = SeparateLedgerVerdict.separateLegsNotNetted := by
  cases v; rfl

/-! ════════════════════════════════════════════════════════════════════════
  ## 交付总结（W6 工位，C25/C26）

  本文件**证**（L0，machine-checked，无 sorry/admit/axiom，standalone 不 import legacy）：
  1. C25 分账本头寸空间 P^sep：`Leg`（单声部多空双坐标）+ `SepPosition`（声部族直积）+
     `legZero`/`legLong`/`legShort`/`legHedged` 基本头寸 + `leg_coords_independent`（双坐标正交）。
  2. C26 净额映射 Net：`legNet`/`Net`（Σ(q⁺−q⁻)）+ `net_append`（直积聚合可加）+
     `net_sepZero`（原点净额 0）+ `net_long_faithful`（净额只对单向持仓忠实）。
  3. ★★**不删父机制（W13 自相似递归代数前提，核心产出）**：
     - `hedged_leg_nonzero`：Q>0 ⟹ 双开腿 (Q,Q) ≠ 零腿（P^sep 非零）。
     - `hedged_leg_net_zero`：双开腿净额 = 0（净额退化）。
     - `hedged_leg_nonzero_but_net_zero`：**W13 主引用** —— P^sep 非零 ∧ 净额为零（分离点）。
     - `parent_leg_survives_child_open`：父多头腿在子空头开启后 qPlus 不减（子不删父）。
  4. ★净额映射有损 `net_not_injective`：存在不同 P^sep 头寸同净额（双开 (1,1) vs 空仓 (0,0)）——
     坐实"分账本语义不做净额映射"（投影丢信息，反 230号直积退化的防火墙）。
  5. 诚实标签 `separate_ledger_verdict_is_not_netted`（裁定 = 保留独立腿，无"折成净额标量"构造子）。

  本文件**不证**（formalization-validity-domain 诚实边界）：
  - ✗ 分账本在实盘有效 / 双开盈利（L0 结构；实盘有效是 L2 EmpiricalDomain，本文件不声称）。
  - ✗ P^sep 头寸数值反映真实盈亏（不臆造价格；跨 bar 盈亏 L2/L3）。
  - ✗ 自相似递归覆盖定理本身（W13 标的）——本文件只建立其代数前提（不删父引理），不证递归归纳。

  ★W6 裁定：P^sep 是与净额账本（LedgerBridge R=Π-A-W / LeverageCapital netNotional）**正交的
  新代数结构**——保留每声部多/空两独立腿。净额映射 Net 是从 P^sep 到 ℤ 的**有损投影**
  （net_not_injective），故"用净额坐标消解分账本语义"在结构上不可行。这与 230号一致：直积
  （P^sep）在定义域代数成立，有效域在净额度量下退化（双开 (Q,Q)↦0）——本文件证前者，标注后者。

  谱系：C25/C26（PDF §一 页11–12）→ 230号（直积退化：双开净额塌缩）→ 本文件 W6
        （P^sep 根 + 不删父代数前提，供 W8/W13 引用）。
  ════════════════════════════════════════════════════════════════════════ -/

end NewChanlun.Origin.SeparateLedger
