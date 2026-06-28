/-
  Origin/CoverFeasibleTarget.lean — 工作单元 W11：分账本全定义策略「定理2」
  C36（定理2）：覆盖可行域 X^cover_Θ 中，J_t 第一项二次偏离型 ≥ 0 等号唯一 ⟹ 最终仓位 q*=q̄
  （目标腿不被风险投影改变）。

  ════════════════════════════════════════════════════════════════════════
  ## 本文件做什么（C36 / 定理2，权威 PDF 23 页版 §八 页16–17）

  权威源（唯一）：`.chanlun/specs/2026-06-28-complete-classification-pdf-extract.md`
  §B 表 **C36** 行 + §D **W11** 行 + §八。互斥 spec M22=C36 共享，本工位成果互斥分类直接复用。

  分账本扩展（§一–§十四）的全定义策略链中，本文件承载「覆盖可行 ⟹ 仓位=目标」一环：

  ### C36 · 定理2（覆盖可行时最终仓位=目标仓位）（PDF §八 页16–17）
      覆盖可行域 `X^cover_Θ = {x_t : q̄_{t+1} ∈ K_Θ(x_t)}`
      （所有应开多/空腿在当前保证金/杠杆/借券/资本规则下都允许存在）。
      若 `x_t ∈ X^cover_Θ`（即 q̄ ∈ K_Θ，覆盖可行）且 J_t 第一项权重 `w_v > 0`，则 `q*_{t+1} = q̄_{t+1}`。

      证明（§八 页16–17，boxed）：
        - q̄_{t+1} ∈ K_Θ 是可行点（X^cover_Θ 的定义）。
        - 对任意 q ∈ K_Θ，二次偏离项
            D(q) = Σ_{v∈V} w_v[(q⁺_v − q̄⁺_v)² + (q⁻_v − q̄⁻_v)²] ≥ 0，
          等号成立 ⟺ q = q̄（w_v>0 + 平方和非负 + 各分量平方为零 ⟺ 分量相等）。
        - 故 q̄ 是**唯一**使偏离项为零的可行点（字典序第一目标）⟹ q*_{t+1} = q̄_{t+1}。
        - 更严格的字典序目标 `q* = LexArgmin_{q∈K}(‖q−q̄‖²_W, Cost(q), Risk(q))`，只要 q̄∈K
          第一坐标最小值为 0 ⟹ 必 q*=q̄。

  ════════════════════════════════════════════════════════════════════════
  ## ★为什么本文件必须显式建二次偏离型，不能停留在 W10 抽象 jKey（no-patch）

  W10 `SeparateStrategyWellDefined` 把 J_t **抽象为字典序键** `jKey : SepPosition → LexKey`，
  只要求 `jKey_inj`（单射 ⟹ q* 唯一）。`jKey_inj` 给出「q* 存在唯一」（C35），但**给不出**
  「q* = q̄」——因为抽象单射不知道 q̄ 在键上处于最小位置。

  C36 定理2 的严格内容**正是** J_t 第一坐标的**二次型结构**：
    `D(q) = Σ_v w_v[(q⁺−q̄⁺)²+(q⁻−q̄⁻)²] ≥ 0, =0 ⟺ q=q̄`。
  这是 q*=q̄ 的**根据**——q̄ 处第一坐标取全局最小 0，字典序第一目标定胜负 ⟹ q̄ 命中 LexArgmin。
  故本文件**显式构造** D(q)（§B 二次偏离型）+ 证 `D≥0`/`D=0⟺q=q̄`（§C），再把 D 作为
  jKey 第一坐标装配出一个**具体 StrategyObjective**（§D `coverObjective`），证 q̄ 命中
  `IsLexArgmin`（§E）⟹ 复用 W10 `qStar_unique` 得 **q*=q̄**（§F 定理2）。

  这**不是**回避 W10，而是 C36 的严格落地：W10 给「∃!q*（给定 jKey 单射）」，本文件给「在覆盖
  可行域上，把 J_t 第一坐标实例化为二次偏离型后，那个唯一 q* 恰是 q̄」。两者是 C35→C36 的递进
  （策略全定义 → 覆盖可行时输出=目标），不打架（no-workaround 核验，见 §G）。

  ════════════════════════════════════════════════════════════════════════
  ## ★坐标统一（集成铁律，复用 W10 桥接）

  q̄（目标头寸）经 W10 `strategyTargetSep` / `targetLegToSep` 桥接表达在 **W6 P^sep 坐标**
  （`SepPosition = List Leg`）。本文件的二次偏离型 D(q) 在同一 P^sep 坐标上对齐 q 与 q̄——
  逐声部腿 (q⁺_v, q⁻_v) 与 (q̄⁺_v, q̄⁻_v) 作差平方加权。最终 q*=q̄ 在 P^sep 坐标成立，**不留
  两份发散的腿表示**（W10 集成铁律延续）。下游 W14 顶点引本文件 `coverFeasible_qStar_eq_target`。

  ════════════════════════════════════════════════════════════════════════
  ## 认识论等级（formalization-validity-domain / 231号，强制标注）

  **全文件 L0**（纯结构/代数，零数据依赖）。`lake env lean` 通过 = 「覆盖可行域上，加权二次偏离
  型 ≥0 等号唯一 ⟹ q*=q̄」的代数命题正确。
  ★q*=q̄ 是**语法层覆盖可行结论**（确定性策略在覆盖域内输出=目标腿），**不是 L2 实盘盈利**、
    不是「净账户每笔盈利」（C40/C42 明确否定后者，有效域 = 分账本声部级 / 毛收益级 / 语法元素级）。
  ★权重 w_v（>0）是 Θ_risk 参数（非缠论可导）——本文件证「给定 w_v>0 后二次型等号唯一 ⟹ q*=q̄」，
    **不**证权重经验最优（那是 L2 EmpiricalDomain，本文件不声称）。
  ★有效域诚实声明：q*=q̄ 仅在**覆盖可行域 X^cover_Θ**（q̄∈K_Θ）内成立——若 q̄∉K_Θ（保证金/杠杆/
    资本规则不允许目标腿全部存在），则风险投影会改变目标腿（q*≠q̄），定理2 不适用。这是 PDF 自带
    的有效域限定（§八「覆盖可行域」前提），不可膨胀为「任意状态下 q*=q̄」。

  ════════════════════════════════════════════════════════════════════════
  ## owner 边界（铁律）

  本文件**只建** `formal/Origin/CoverFeasibleTarget.lean`。不碰 W6/W9/W10/LexArgmin
  （只 import 只读）。不编辑 lakefile.toml。root 名 `Origin.CoverFeasibleTarget`。
  禁 sorry/admit/axiom。简体中文。

  依赖方向（单向无环，全 committed 只读）：
    CoverFeasibleTarget → SeparateStrategyWellDefined（W10）
                        → SeparateStrategyTarget（W9）
                        → SeparateLedger（W6）
                        → LexArgmin → ConstraintSystem → SourceAxioms。standalone。

  谱系：C36（PDF §八 页16–17）→ W6（P^sep 根 C25/C26）+ W9（目标头寸 q̄_Θ C31/C32）+
        W10（策略全定义 ∃!O + qStar_unique + 桥接 C33/C34/C35）+ LexArgmin（字典序 ∃!u*）
        → 本文件 W11（覆盖可行 ⟹ q*=q̄，供 W14 顶点引）。
-/

import Origin.SeparateStrategyWellDefined
import Origin.SeparateStrategyTarget
import Origin.SeparateLedger
import Origin.LexArgmin

namespace NewChanlun.Origin.CoverFeasibleTarget

open NewChanlun.Origin.SeparateLedger (Leg legZero legLong legShort SepPosition)
open NewChanlun.Origin.SeparateStrategyWellDefined
  (RiskFeasibleSet StrategyObjective)
open NewChanlun.Origin.LexArgmin (LexKey lexLe RiskProjection)

/-! ════════════════════════════════════════════════════════════════════════
  ## §A 单腿二次偏离 `legDev`（C36 §八：(q⁺_v−q̄⁺_v)² + (q⁻_v−q̄⁻_v)²）

  C36 二次偏离项的单声部分量：声部 v 的当前腿 (q⁺_v, q⁻_v) 与目标腿 (q̄⁺_v, q̄⁻_v) 的
  加权平方偏离 `w_v · [(q⁺−q̄⁺)² + (q⁻−q̄⁻)²]`。用 `Int` 承载差（Nat 减法会截断，必须先转 Int）。

  ★平方在 Int 上恒 ≥0（`Int.mul_self_nonneg` / `sq_nonneg`），w_v>0 ⟹ 加权后仍 ≥0。
  ════════════════════════════════════════════════════════════════════════ -/

/-- 整数平方非负的本地引理（L0，core Int，不依赖 Mathlib）：`0 ≤ a * a`。 -/
theorem int_mul_self_nonneg (a : Int) : 0 ≤ a * a := by
  rcases Int.le_total 0 a with h | h
  · exact Int.mul_nonneg h h
  · -- a ≤ 0 ⟹ 0 ≤ -a ⟹ 0 ≤ (-a)*(-a) = a*a（Int.neg_mul_neg）
    have hna : (0 : Int) ≤ -a := by omega
    have hge : 0 ≤ (-a) * (-a) := Int.mul_nonneg hna hna
    rw [Int.neg_mul_neg] at hge
    exact hge

/-- 整数平方为零 ⟺ 该数为零（L0）：`a * a = 0 ↔ a = 0`。 -/
theorem int_mul_self_eq_zero (a : Int) : a * a = 0 ↔ a = 0 := by
  constructor
  · intro h
    rcases Int.mul_eq_zero.mp h with ha | ha <;> exact ha
  · intro h; rw [h]; decide

/--
  ★单腿坐标差 `legDiff`（L0，C36 §八）：声部 v 当前腿 `l` 与目标腿 `t` 在多/空两坐标的差
  `(q⁺−q̄⁺, q⁻−q̄⁻)`（Int 承载，可负）。这是二次偏离 `(q⁺−q̄⁺)²+(q⁻−q̄⁻)²` 的前置。
-/
def legDiffPlus (l t : Leg) : Int := (l.qPlus : Int) - (t.qPlus : Int)

def legDiffMinus (l t : Leg) : Int := (l.qMinus : Int) - (t.qMinus : Int)

/--
  ★单腿二次偏离 `legDev`（L0，C36 §八单声部分量）：
  `legDev w l t = w · [(q⁺_l − q⁺_t)² + (q⁻_l − q⁻_t)²]`（w_v 加权，Int 承载）。

  - `w : Nat`：声部权重 w_v（C36 要求 `w_v > 0`，正性作可分离前提进入定理）。
  - `l : Leg`：当前腿 (q⁺_v, q⁻_v)。
  - `t : Leg`：目标腿 (q̄⁺_v, q̄⁻_v)。

  ★多/空两坐标各自作差平方再相加——分账本「多空独立坐标」的偏离度量（C25：两腿独立）。
-/
def legDev (w : Nat) (l t : Leg) : Int :=
  (w : Int) * (legDiffPlus l t * legDiffPlus l t + legDiffMinus l t * legDiffMinus l t)

/-- ★单腿偏离非负（L0，C36：平方和 ≥0，w≥0）：`0 ≤ legDev w l t`。 -/
theorem legDev_nonneg (w : Nat) (l t : Leg) : 0 ≤ legDev w l t := by
  unfold legDev
  apply Int.mul_nonneg
  · exact Int.natCast_nonneg w
  · exact Int.add_nonneg (int_mul_self_nonneg _) (int_mul_self_nonneg _)

/--
  ★单腿偏离为零 ⟺ 两坐标相等（L0，C36 等号条件单声部，w>0）：
  `w > 0 ⟹ (legDev w l t = 0 ↔ q⁺_l = q⁺_t ∧ q⁻_l = q⁻_t)`。

  w_v>0 时，加权平方和为零 ⟺ 两个平方各自为零 ⟺ 两坐标差各自为零 ⟺ 多/空坐标分别相等。
  这是 C36「等号 ⟺ q=q̄」在单声部腿上的精确落地（w>0 是关键前提——w=0 时偏离恒零，无法区分）。
-/
theorem legDev_eq_zero_iff (w : Nat) (hw : 0 < w) (l t : Leg) :
    legDev w l t = 0 ↔ l.qPlus = t.qPlus ∧ l.qMinus = t.qMinus := by
  unfold legDev
  constructor
  · intro h
    -- w>0 ⟹ (w:Int)≠0，故平方和=0
    have hwne : (w : Int) ≠ 0 := by
      simp only [ne_eq, Int.natCast_eq_zero]; omega
    have hsum : legDiffPlus l t * legDiffPlus l t
        + legDiffMinus l t * legDiffMinus l t = 0 := by
      rcases Int.mul_eq_zero.mp h with hw0 | hs0
      · exact absurd hw0 hwne
      · exact hs0
    -- 平方和=0 + 各项≥0 ⟹ 各项=0
    have hp2 : legDiffPlus l t * legDiffPlus l t = 0 := by
      have hpge := int_mul_self_nonneg (legDiffPlus l t)
      have hmge := int_mul_self_nonneg (legDiffMinus l t)
      omega
    have hm2 : legDiffMinus l t * legDiffMinus l t = 0 := by
      have hpge := int_mul_self_nonneg (legDiffPlus l t)
      have hmge := int_mul_self_nonneg (legDiffMinus l t)
      omega
    have hpd : legDiffPlus l t = 0 := (int_mul_self_eq_zero _).mp hp2
    have hmd : legDiffMinus l t = 0 := (int_mul_self_eq_zero _).mp hm2
    -- 坐标差=0 ⟹ 坐标相等（Int 转回 Nat）
    unfold legDiffPlus at hpd
    unfold legDiffMinus at hmd
    constructor
    · omega
    · omega
  · rintro ⟨hp, hm⟩
    -- 坐标相等 ⟹ 差=0 ⟹ 平方=0 ⟹ 偏离=0
    have hpd : legDiffPlus l t = 0 := by unfold legDiffPlus; rw [hp]; omega
    have hmd : legDiffMinus l t = 0 := by unfold legDiffMinus; rw [hm]; omega
    rw [hpd, hmd]; simp

/--
  ★单腿偏离为零 ⟺ 两腿相等（L0，C36 等号条件的 `Leg` 形式，w>0）：
  `w > 0 ⟹ (legDev w l t = 0 ↔ l = t)`。整合 `legDev_eq_zero_iff` 的两坐标相等为整腿相等
  （`Leg` 由 qPlus/qMinus 两字段决定，逐字段相等 ⟺ 整腿相等）。
-/
theorem legDev_eq_zero_iff_leg (w : Nat) (hw : 0 < w) (l t : Leg) :
    legDev w l t = 0 ↔ l = t := by
  rw [legDev_eq_zero_iff w hw]
  constructor
  · rintro ⟨hp, hm⟩
    cases l; cases t; simp_all
  · intro h; rw [h]; exact ⟨rfl, rfl⟩

/-! ════════════════════════════════════════════════════════════════════════
  ## §B 总二次偏离型 `D(q)`（C36 §八：Σ_{v∈V} w_v[(q⁺−q̄⁺)²+(q⁻−q̄⁻)²]）

  C36 的二次偏离项是声部族上的加权平方和 `D(q) = Σ_v w_v[...]`。本文件在 P^sep 坐标
  （`SepPosition = List Leg`）上逐声部对齐当前头寸 `q` 与目标头寸 `q̄`，每声部用 `legDev` 算
  加权偏离，求和得 `D(q)`。

  ★对齐前提：q 与 q̄ 在 P^sep 中**同声部同顺序**（等长，逐声部对应同一 v）。这是分账本头寸
  族对齐的自然要求（C25：P^sep = ∏_v，声部族固定）。本文件用「等长 + 逐声部权重列表 w」承载
  Σ_v——权重列表 `ws : List Nat` 与头寸列表同长，第 i 项是声部 i 的 w_v。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★总二次偏离型 `quadDev`（L0，C36 §八 `D(q) = Σ_v w_v[(q⁺−q̄⁺)²+(q⁻−q̄⁻)²]`）：
  权重列表 `ws`、当前头寸 `q`、目标头寸 `target` 在 P^sep 坐标上逐声部 `legDev` 求和。

  - `ws : List Nat`：声部权重族 (w_v)_{v∈V}（C36 要求每 w_v>0，正性作可分离谓词 §C）。
  - `q : SepPosition`：当前分账本头寸 (q⁺_v, q⁻_v)_{v∈V}。
  - `target : SepPosition`：目标分账本头寸 q̄ = (q̄⁺_v, q̄⁻_v)_{v∈V}（W10 strategyTargetSep）。

  逐声部三元 zip（w_v, q_v, q̄_v）取 `legDev`，求和。三列表等长时严格对齐 Σ_v；不等长时
  多出的尾部不计入（由 zip 截断）——故定理用「三列表等长」作前提保证完整 Σ_v（§C `quadDev_eq_zero`）。
-/
def quadDev (ws : List Nat) (q target : SepPosition) : Int :=
  ((ws.zip (q.zip target)).map (fun p => legDev p.1 p.2.1 p.2.2)).sum

/-- 本地引理（L0，core，替 Mathlib `List.sum_nonneg`）：各项 ≥0 ⟹ 列表和 ≥0。 -/
theorem list_int_sum_nonneg (l : List Int) (h : ∀ x ∈ l, 0 ≤ x) : 0 ≤ l.sum := by
  induction l with
  | nil => simp
  | cons a as ih =>
    rw [List.sum_cons]
    have ha : 0 ≤ a := h a List.mem_cons_self
    have has : 0 ≤ as.sum := ih (fun x hx => h x (List.mem_cons_of_mem a hx))
    omega

/-- 本地引理（L0，core，替 Mathlib `List.sum_eq_zero`）：各项 =0 ⟹ 列表和 =0。 -/
theorem list_int_sum_eq_zero (l : List Int) (h : ∀ x ∈ l, x = 0) : l.sum = 0 := by
  induction l with
  | nil => simp
  | cons a as ih =>
    rw [List.sum_cons]
    have ha : a = 0 := h a List.mem_cons_self
    have has : as.sum = 0 := ih (fun x hx => h x (List.mem_cons_of_mem a hx))
    omega

/-- ★总偏离非负（L0，C36：各声部偏离 ≥0 ⟹ 和 ≥0）：`0 ≤ quadDev ws q target`。 -/
theorem quadDev_nonneg (ws : List Nat) (q target : SepPosition) :
    0 ≤ quadDev ws q target := by
  unfold quadDev
  apply list_int_sum_nonneg
  intro x hx
  rw [List.mem_map] at hx
  obtain ⟨p, _, rfl⟩ := hx
  exact legDev_nonneg p.1 p.2.1 p.2.2

/-! ════════════════════════════════════════════════════════════════════════
  ## §C 偏离为零 ⟺ q = q̄（C36 §八等号唯一，核心代数）

  C36：`D(q) ≥ 0`，等号 ⟺ `q = q̄`。§B 已证 ≥0；本节证「等号 ⟺ q=q̄」——这是 q*=q̄ 的
  根据（q̄ 是唯一使 D=0 的头寸）。前提：三列表等长 + 每权重 w_v>0（C36「w_v>0」）。
  ════════════════════════════════════════════════════════════════════════ -/

/-- 权重族全正 `allPos`（L0，C36「∀v, w_v>0」）：权重列表每项 > 0。 -/
def allPos (ws : List Nat) : Prop := ∀ w ∈ ws, 0 < w

/--
  ★★总偏离为零 ⟹ q = q̄（L0，C36 §八等号唯一·正向，核心）：
  三列表等长（`ws`/`q`/`target` 同长）+ 每权重 w_v>0 + `quadDev ws q target = 0` ⟹ `q = target`。

  证明（对列表归纳）：和为零 + 各项 ≥0 ⟹ 各项 = 0（`legDev_nonneg`）⟹ 每声部 `legDev w q_v q̄_v=0`
  ⟹（w_v>0，`legDev_eq_zero_iff_leg`）每声部腿相等 q_v=q̄_v ⟹ 逐声部相等 ⟹ q=q̄。
  这是「q̄ 是唯一使偏离为零的头寸」的形式——下游 §E 用此证 q̄ 命中 LexArgmin。
-/
theorem quadDev_eq_zero_imp (ws : List Nat) (q target : SepPosition)
    (hlen1 : ws.length = q.length) (hlen2 : q.length = target.length)
    (hpos : allPos ws) (hzero : quadDev ws q target = 0) :
    q = target := by
  induction ws generalizing q target with
  | nil =>
    -- ws=[] ⟹ q=[]（hlen1）⟹ target=[]（hlen2）
    have hq : q = [] := List.length_eq_zero_iff.mp (by simp at hlen1; omega)
    subst hq
    have ht : target = [] := List.length_eq_zero_iff.mp (by simp at hlen2; omega)
    subst ht; rfl
  | cons w ws' ih =>
    -- q 非空、target 非空（长度匹配）
    cases q with
    | nil => simp at hlen1
    | cons ql qs =>
      cases target with
      | nil => simp at hlen2
      | cons tl ts =>
        -- quadDev (w::ws') (ql::qs) (tl::ts) = legDev w ql tl + quadDev ws' qs ts
        have hhead : legDev w ql tl
            + quadDev ws' qs ts = 0 := by
          unfold quadDev at hzero ⊢
          simp only [List.zip_cons_cons, List.map_cons, List.sum_cons] at hzero
          exact hzero
        -- 头项 ≥0、尾和 ≥0 ⟹ 各自=0
        have hheadge : 0 ≤ legDev w ql tl := legDev_nonneg w ql tl
        have htailge : 0 ≤ quadDev ws' qs ts := quadDev_nonneg ws' qs ts
        have hhead0 : legDev w ql tl = 0 := by omega
        have htail0 : quadDev ws' qs ts = 0 := by omega
        -- w>0（hpos 首项）⟹ ql=tl
        have hw : 0 < w := hpos w List.mem_cons_self
        have hleq : ql = tl := (legDev_eq_zero_iff_leg w hw ql tl).mp hhead0
        -- 尾部递归
        have hpos' : allPos ws' := fun w' hw' => hpos w' (List.mem_cons_of_mem w hw')
        have hlen1' : ws'.length = qs.length := by simp at hlen1; omega
        have hlen2' : qs.length = ts.length := by simp at hlen2; omega
        have hqs : qs = ts := ih qs ts hlen1' hlen2' hpos' htail0
        rw [hleq, hqs]

/--
  ★总偏离在 q̄ 处为零（L0，C36 §八：D(q̄)=0，等号唯一·反向）：`quadDev ws target target = 0`。
  当前头寸 = 目标头寸时二次偏离恒为零（每声部腿与自身差为零）——q̄ 是 D 的零点。
  这是 §E 证「q̄ 处第一坐标键最小」的根据（D(q̄)=0 是 D≥0 的全局最小）。
-/
theorem quadDev_self_zero (ws : List Nat) (target : SepPosition) :
    quadDev ws target target = 0 := by
  -- 单腿自身偏离恒零（不依赖 w>0）：legDev w l l = 0。
  have hself : ∀ (w : Nat) (l : Leg), legDev w l l = 0 := by
    intro w l
    unfold legDev legDiffPlus legDiffMinus
    have h1 : (l.qPlus : Int) - (l.qPlus : Int) = 0 := by omega
    have h2 : (l.qMinus : Int) - (l.qMinus : Int) = 0 := by omega
    rw [h1, h2]; simp
  -- 对 ws 与 target 同步归纳：每步头项 legDev w tl tl = 0，尾部递归。
  induction ws generalizing target with
  | nil => unfold quadDev; simp
  | cons w ws' ih =>
    cases target with
    | nil => unfold quadDev; simp
    | cons tl ts =>
      unfold quadDev
      simp only [List.zip_cons_cons, List.map_cons, List.sum_cons]
      rw [hself w tl]
      have htail : quadDev ws' ts ts = 0 := ih ts
      unfold quadDev at htail
      omega

/-! ════════════════════════════════════════════════════════════════════════
  ## §D 覆盖目标函数 `coverObjective`（把 D 装进 J_t 第一坐标，对接 W10）

  C36：J_t = (D(q), Cost(q), Risk(q)) 的字典序。本节构造一个**具体** W10 `StrategyObjective`，
  其字典序键 `jKey q = [D(q), tieBreak(q)]`——第一坐标是二次偏离 D（C36 第一目标），第二坐标
  是格点索引 tie-break（消平局保单射）。q̄∈K_Θ（覆盖可行）时 D(q̄)=0 是全局最小 ⟹ q̄ 命中
  LexArgmin（§E）⟹ q*=q̄（§F）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★覆盖目标函数键 `coverKey`（L0，C36 J_t 字典序键在 P^sep 的实例）：
  `coverKey ws target idx q = [D(q), idx q]`——第一分量 = 二次偏离 D(q)（C36 第一目标，§B），
  第二分量 = 格点索引 `idx q`（tie-break，消平局；W10/LexArgmin「固定字典序平局规则」）。

  - `ws/target`：权重族 + 目标头寸 q̄（D 的参数）。
  - `idx : SepPosition → Int`：格点索引（在可行集上单射，保证键单射；执行层提供）。

  ★第一分量 D 在 q̄ 处取 0（`quadDev_self_zero`），是 D≥0 的全局最小——这是定理2 的关键。
-/
def coverKey (ws : List Nat) (target : SepPosition) (idx : SepPosition → Int)
    (q : SepPosition) : LexKey :=
  [quadDev ws q target, idx q]

/--
  ★覆盖目标函数 `CoverObjective`（L0，C36 §八）：覆盖可行域上的 J_t 实例。
  - `K : RiskFeasibleSet`：C33 风险约束集 K_Θ ⊆ P^sep（有限非空）。
  - `ws : List Nat` + `wsPos : allPos ws`：权重族 (w_v)_{v∈V}，每 w_v>0（C36「w_v>0」）。
  - `target : SepPosition`：目标头寸 q̄（W10 strategyTargetSep 桥接到 P^sep）。
  - `targetMem : target ∈ K.feasible`：**覆盖可行**——q̄∈K_Θ（X^cover_Θ 的定义，C36 前提）。
  - `idx : SepPosition → Int` + `idx_inj`：格点索引，在 K_Θ 上单射（tie-break 保 jKey 单射）。
  - `lenWs / lenQT`：声部族对齐——可行头寸与权重/目标等长（Σ_v 完整，§C 用）。

  ★`targetMem` 是 X^cover_Θ 的形式编码：`x_t ∈ X^cover_Θ ⟺ q̄ ∈ K_Θ`。本结构内含此前提，
    故 `CoverObjective` 的实例 = 覆盖可行状态。q̄∉K_Θ 时无法构造本结构（定理2 不适用，有效域限定）。
-/
structure CoverObjective where
  K : RiskFeasibleSet
  ws : List Nat
  wsPos : allPos ws
  target : SepPosition
  targetMem : target ∈ K.feasible
  idx : SepPosition → Int
  idx_inj : ∀ p q, p ∈ K.feasible → q ∈ K.feasible → idx p = idx q → p = q
  /-- 可行头寸与权重族等长（声部族对齐，Σ_v 完整覆盖所有声部）。 -/
  lenWs : ∀ q, q ∈ K.feasible → ws.length = q.length
  /-- 可行头寸与目标头寸等长（同声部族）。 -/
  lenQT : ∀ q, q ∈ K.feasible → q.length = target.length

/--
  ★覆盖目标函数键单射 `coverKey_inj`（L0，§D：jKey 单射，对接 W10 StrategyObjective.jKey_inj）：
  在 K_Θ 上 `coverKey ws target idx` 单射——由格点索引 `idx_inj` 保证（键含 idx 作第二分量，
  两可行头寸键相等 ⟹ idx 相等 ⟹ 头寸相等）。这是 C35「固定字典序平局规则」在本实例的兑现。
-/
theorem coverKey_inj (C : CoverObjective) :
    ∀ p q, p ∈ C.K.feasible → q ∈ C.K.feasible →
      coverKey C.ws C.target C.idx p = coverKey C.ws C.target C.idx q → p = q := by
  intro p q hp hq hkey
  unfold coverKey at hkey
  -- [D(p), idx p] = [D(q), idx q] ⟹ idx p = idx q
  have hidx : C.idx p = C.idx q := by
    simp only [List.cons.injEq] at hkey
    exact hkey.2.1
  exact C.idx_inj p q hp hq hidx

/--
  ★装配为 W10 `StrategyObjective`（L0，§D → W10 接口）：把 `CoverObjective` 装为 W10 的
  `StrategyObjective`——K_Θ=K、jKey=coverKey（D 作第一坐标）、jKey_inj=coverKey_inj。
  这是「C36 J_t 第一坐标实例化为二次偏离型」对接 W10「∃!q*」机制的接口。下游复用 W10
  `qStar` / `qStar_unique` / `qStar_isLexArgmin`。
-/
def CoverObjective.toStrategyObjective (C : CoverObjective) : StrategyObjective :=
  { K := C.K
    jKey := coverKey C.ws C.target C.idx
    jKey_inj := coverKey_inj C }

/-! ════════════════════════════════════════════════════════════════════════
  ## §E q̄ 命中 LexArgmin（C36 §八：q̄ 是唯一使第一坐标为零的可行点 ⟹ 字典序最小）

  C36 证明核心：q̄∈K_Θ 是可行点，且 D(q̄)=0 是 D≥0 的全局最小 ⟹ 对任意可行 q，
  `coverKey q̄ = [0, idx q̄] ≤ [D(q), idx q] = coverKey q`（字典序：首分量 0 ≤ D(q)，若 D(q)>0
  则严格小，若 D(q)=0 则 q=q̄ 键相等）。故 q̄ 命中 `IsLexArgmin`。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★q̄ 处第一坐标键最小（L0，C36 §八核心步）：对任意可行头寸 q，
  `lexLe (coverKey ... target) (coverKey ... q) = true`——q̄ 的字典序键 ≤ q 的字典序键。

  证明（字典序首分量）：q̄ 的首分量 = D(q̄) = 0（`quadDev_self_zero`），q 的首分量 = D(q) ≥ 0
  （`quadDev_nonneg`）。
  - 若 `0 < D(q)`：首分量严格小 ⟹ `lexLe` true（`lexLe_cons_lt`）。
  - 若 `D(q) = 0`：首分量相等，比第二分量——但此时（§C 等号唯一 + q̄∈K_Θ）`q = target`，
    故整键相等 ⟹ `lexLe` refl。
  这正是 C36「q̄ 唯一使偏离为零 ⟹ q̄ 字典序第一目标最小」。
-/
theorem target_key_le (C : CoverObjective) (q : SepPosition) (hq : q ∈ C.K.feasible) :
    lexLe (coverKey C.ws C.target C.idx C.target) (coverKey C.ws C.target C.idx q) = true := by
  unfold coverKey
  -- 首分量：D(target)=0, D(q)≥0
  have hself : quadDev C.ws C.target C.target = 0 := quadDev_self_zero C.ws C.target
  have hqge : 0 ≤ quadDev C.ws q C.target := quadDev_nonneg C.ws q C.target
  rw [hself]
  rcases Int.lt_or_le 0 (quadDev C.ws q C.target) with hlt | hle0
  · -- 0 < D(q)：首分量严格小 ⟹ lexLe true
    exact NewChanlun.Origin.LexArgmin.lexLe_cons_lt [C.idx C.target] [C.idx q] hlt
  · -- D(q) ≤ 0 与 D(q) ≥ 0 ⟹ D(q) = 0：等号唯一 ⟹ q = target ⟹ 整键相等 ⟹ refl
    have hqzero : quadDev C.ws q C.target = 0 := by omega
    have hqeq : q = C.target :=
      quadDev_eq_zero_imp C.ws q C.target
        (C.lenWs q hq) (C.lenQT q hq) C.wsPos hqzero
    subst hqeq
    -- coverKey target = coverKey target ⟹ lexLe refl（首分量 0=0，整键相等）
    rw [hself]
    exact NewChanlun.Origin.LexArgmin.lexLe_refl [(0 : Int), C.idx C.target]

/--
  ★★q̄ 命中 LexArgmin（L0，C36 §八：q̄ 是 K_Θ 上 J_t 字典序最小点）：
  `(C.toStrategyObjective).toRiskProjection.IsLexArgmin C.target`。

  q̄∈K_Θ（覆盖可行 `targetMem`）+ q̄ 处键最小（`target_key_le`）⟹ q̄ 满足 `IsLexArgmin`
  （可行 ∧ 键 ≤ 所有可行键）。这是定理2 的关键中间步——q̄ 命中 LexArgmin ⟹（W10 唯一性）q*=q̄。
-/
theorem target_isLexArgmin (C : CoverObjective) :
    C.toStrategyObjective.toRiskProjection.IsLexArgmin C.target := by
  constructor
  · -- q̄ ∈ feasible（覆盖可行）
    exact C.targetMem
  · -- q̄ 键 ≤ 所有可行键
    intro y hy
    exact target_key_le C y hy

/-! ════════════════════════════════════════════════════════════════════════
  ## §F C36 定理2：覆盖可行 ⟹ q* = q̄（核心产出，分账本坐标 P^sep）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★★★定理2（C36 全形式）·覆盖可行 ⟹ 最终仓位=目标仓位 `coverFeasible_qStar_eq_target`★★★★
  （L0，C36 §八 页16–17，**分账本坐标 P^sep 上 q*_{t+1}=q̄_{t+1}**）：

  给定覆盖目标函数 `C : CoverObjective`（含覆盖可行前提 q̄∈K_Θ + 权重 w_v>0），则风险投影后的
  最终仓位 `q*_{t+1}`（= W10 `qStar`，K_Θ 上 J_t 字典序最小点）**等于目标仓位 q̄_{t+1}**：

      `(C.toStrategyObjective).qStar = C.target`

  证明（§八 页16–17）：
  - q̄∈K_Θ 可行（`targetMem`）+ 二次偏离 D≥0 等号唯一（§B/§C，w_v>0）⟹ D(q̄)=0 全局最小
    ⟹ q̄ 命中 `IsLexArgmin`（§E `target_isLexArgmin`）。
  - 复用 W10 `qStar_unique`（任意 LexArgmin 点 = qStar）⟹ q̄ = qStar ⟹ qStar = q̄。

  ★这就是 PDF §八「q̄ 是唯一使偏离项为零的可行点（字典序第一目标）⟹ q*=q̄」的形式化——
    **目标腿不被风险投影改变**（覆盖可行域内）。
  ★坐标核验：q*、q̄ 均在 **P^sep 坐标**（SepPosition）；q̄ 经 W10 strategyTargetSep 桥接而来。
  -/
theorem coverFeasible_qStar_eq_target (C : CoverObjective) :
    C.toStrategyObjective.qStar = C.target :=
  (C.toStrategyObjective.qStar_unique C.target (target_isLexArgmin C)).symm

/--
  ★定理2·对称形式 `target_eq_qStar`（L0，C36）：目标仓位 = 最终仓位（q̄ = q*）。
  `coverFeasible_qStar_eq_target` 的对称——下游若需「q̄ 即 LexArgmin 输出」用此向。
-/
theorem target_eq_qStar (C : CoverObjective) :
    C.target = C.toStrategyObjective.qStar :=
  (coverFeasible_qStar_eq_target C).symm

/--
  ★定理2·q* 二次偏离为零 `qStar_quadDev_zero`（L0，C36 推论）：覆盖可行时最终仓位 q* 的二次
  偏离 `D(q*) = 0`——q* 完美命中目标（无残差偏离）。由 q*=q̄（定理2）+ D(q̄)=0（§C）直接得。
  这是「目标腿不被改变」的偏离侧度量：覆盖可行域内风险投影残差为零。
-/
theorem qStar_quadDev_zero (C : CoverObjective) :
    quadDev C.ws C.toStrategyObjective.qStar C.target = 0 := by
  rw [coverFeasible_qStar_eq_target C]
  exact quadDev_self_zero C.ws C.target

/-! ════════════════════════════════════════════════════════════════════════
  ## §G 诚实标签（formalization-validity-domain gatekeeper，q*=q̄ ≠ 实盘盈利）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★定理2 裁定标签 `CoverTargetVerdict`（gatekeeper，诚实分层）。
  唯一构造子 `qStarEqTargetInCoverDomain`——类型层钉死「覆盖可行域内 q*=q̄（目标腿不被风险投影改变）」。
  ★**没有** `ProfitGuaranteed` / `QStarEqTargetEverywhere` 构造子——拒绝两类声明膨胀：
    (1) 「q*=q̄ ⟹ 实盘盈利」（q*=q̄ 是语法层覆盖结论，非盈利，C40/C42 否定净账户每笔盈利）；
    (2) 「q*=q̄ 在任意状态成立」（仅覆盖可行域 X^cover_Θ 内成立；q̄∉K_Θ 时风险投影改变目标腿）。
-/
inductive CoverTargetVerdict where
  | qStarEqTargetInCoverDomain
deriving DecidableEq, Repr

/--
  ★裁定见证（L0，gatekeeper）：定理2 裁定必是「覆盖可行域内 q*=q̄」。
  支撑：§F `coverFeasible_qStar_eq_target`（q*=q̄）+ §E `target_isLexArgmin`（q̄ 命中 LexArgmin）
  + §B/§C 二次型 ≥0 等号唯一。q*=q̄ 是覆盖域内语法层结论——**不**蕴含实盘盈利、**不**在覆盖域外成立。
-/
theorem cover_verdict_is_qstar_eq_target (v : CoverTargetVerdict) :
    v = CoverTargetVerdict.qStarEqTargetInCoverDomain := by
  cases v; rfl

/-! ════════════════════════════════════════════════════════════════════════
  ## 交付总结（W11 工位，C36 定理2）

  本文件**证**（L0，machine-checked，无 sorry/admit/axiom，standalone 不 import legacy）：

  1. §A 单腿二次偏离 `legDev`：`legDev_nonneg`（≥0）+ `legDev_eq_zero_iff`/`legDev_eq_zero_iff_leg`
     （w>0 ⟹ 偏离=0 ⟺ 两腿相等）。整数平方 `int_mul_self_nonneg`/`int_mul_self_eq_zero` 本地证。

  2. §B 总二次偏离型 `quadDev`（C36 §八 `D(q)=Σ_v w_v[(q⁺−q̄⁺)²+(q⁻−q̄⁻)²]`）：
     `quadDev_nonneg`（D≥0）。P^sep 坐标逐声部 zip 加权求和。

  3. §C 等号唯一（C36 §八核心代数）：`quadDev_eq_zero_imp`（等长+w_v>0+D=0 ⟹ q=q̄）+
     `quadDev_self_zero`（D(q̄)=0）。这是 q*=q̄ 的根据（q̄ 唯一使 D=0）。

  4. §D 装配 W10：`coverKey`（jKey=[D(q),idx]，D 作第一坐标）+ `coverKey_inj`（idx 单射保键单射）
     + `CoverObjective`（含覆盖可行前提 targetMem=q̄∈K_Θ）+ `toStrategyObjective`（对接 W10 ∃!q*）。

  5. §E q̄ 命中 LexArgmin：`target_key_le`（q̄ 键 ≤ 所有可行键，字典序首分量 0=D(q̄)≤D(q)）+
     `target_isLexArgmin`（q̄ 满足 IsLexArgmin）。

  6. ★★§F 定理2（**分账本坐标 P^sep 上 q*=q̄**，核心产出）：
     - **`coverFeasible_qStar_eq_target`：覆盖可行 ⟹ qStar = q̄（目标腿不被风险投影改变）。**
     - `target_eq_qStar`（对称）+ `qStar_quadDev_zero`（q* 偏离残差为零）。

  7. §G 诚实标签 `cover_verdict_is_qstar_eq_target`（裁定=覆盖域内 q*=q̄，无「盈利保证」/「处处成立」构造子）。

  本文件**不证**（formalization-validity-domain 诚实边界）：
  - ✗ q*=q̄ ⟹ 实盘盈利 / 净账户每笔盈利（q*=q̄ 是语法层覆盖结论；盈利是 L2，C40/C42 否定净账户每笔盈利）。
  - ✗ q*=q̄ 在覆盖可行域**外**成立（q̄∉K_Θ 时风险投影改变目标腿；仅 X^cover_Θ 内成立——PDF §八前提）。
  - ✗ 权重 w_v 经验最优（Θ_risk 参数，校准是 L2 EmpiricalDomain，本文件只用「w_v>0」结构）。

  ★W10 关系核对（no-workaround）：W10 `qStar_exists_unique` 证「给定 jKey 单射 ⟹ ∃!q*」（C35
    策略全定义）；本文件把 J_t 第一坐标**实例化为二次偏离型** D，证「覆盖可行域内那个唯一 q* = q̄」
    （C36 定理2）。两者是 C35→C36 的递进（策略全定义 → 覆盖可行时输出=目标），**无冲突**——
    本文件复用 W10 `qStar_unique`（任意 LexArgmin 点=qStar），把 q̄ 证成 LexArgmin 点即得 q*=q̄。

  谱系：C36（PDF §八 页16–17）→ W6（P^sep 根 C25/C26）+ W9（目标头寸 q̄_Θ C31/C32）+
        W10（∃!q* + qStar_unique + 桥接 C33/C34/C35）+ LexArgmin（字典序）→ 本文件 W11
        （覆盖可行 ⟹ q*=q̄，供 W14 顶点引）。
  ════════════════════════════════════════════════════════════════════════ -/

end NewChanlun.Origin.CoverFeasibleTarget
