/-
  信号层必然性 T₁/T₂/T₃/T₅/T₁₄/T₁₅ 的 Lean 形式化（task #47）

  上游：`docs/necessity_derivation.md` 第1部分（T₁-T₁₆ 命题 + necessity 证明）。
  范式：纯 inductive 构造子穷尽 + 条件化结构定理，不依赖 Mathlib，认识论 **L0**。
  禁 sorry/admit/axiom——所有外部公理（A₀/A₁/A₂/A₄）作为 **定理 hypothesis** 传入，
  不用 Lean `axiom` 关键字。Lean 证明的内容是"在公理成立时结论成立"，公理本身的真值
  由缠论原文（权威链一级）担保，不由 Lean 担保。

  ★异质审查（codex gpt-5.5 high，session 019effe4-28dc-75c3-9955-6bd8ea99b11d）裁决吸收：
  - T₁/T₂/T₅：L0 同义反复在本范式可接受，但定理须 **条件化**（A₀ 前提显式），
    不写无条件世界事实。T₅ "辨识需相邻可比较单元"作显式前提，a0 唯一仅在固定 origin+grain。
  - T₃：A₁ 作 hypothesis（非 axiom）忠实；T₃ ≈ unpack A₁，命名标注条件性。
  - T₁₄：用事件状态机（PositionState + Step + Run）证 sides 交替 + 时间首尾相接，
    **非**裸 `List Side`；raw 流不加约束（诚实标注 raw ~）。
  - T₁₅：**只**形式化结构配对（located 卖点 spawn + located 买点 close + 交替 ⟹ 配对良构）；
    价格 zigzag（买<前卖）= **L2 经验，非 Lean 可证**（located 流违反~50%，regime 依赖），
    标注为注释，**不**写伪定理，定理命名禁含 cost_reduction/price_lt 等价格语义。

  认识论：本模块所有定理 = **L0**（定义内蕴 + 条件结构推论）。lake build 通过 = 逻辑/管线正确，
  **不**是实证有效域，不得膨胀（formalization-validity-domain）。
-/

import Formal.TrendTrichotomy
import Formal.BSPLabels

namespace Tlayers.Signal

open Formal.TrendTrichotomy (Direction)
open Formal.BSPLabels (Side)

/-! ## 势的载体（D0.1 消耗律 + D0.2 完美 + A₀ 走势终完美） -/

/--
  ★势 Φ（necessity_derivation §0.1 primitive）：非负标量储备 = 运动的"燃料"。
  二构造子：`finite n`（有限势，n 单位储备）/ `infinite`（无限势）。
  这把"有限 vs 无限"建模为 inductive 区分——T₁ 的内容 = 在 A₀ 公理下排除 `infinite`。
-/
inductive Potential where
  | finite (n : Nat)
  | infinite
deriving DecidableEq, Repr

open Potential

/--
  ★D0.1 势的消耗律：运动消耗势，每推进一单位 Φ 严格单调下降（`dΦ < 0`）。
  - `finite (n+1)` 消耗一单位 ↦ `finite n`（严格下降）。
  - `finite 0` 已耗尽，保持 0（完美态吸收）。
  - `infinite` 消耗一单位仍 `infinite`（无限减一仍无限——T₁ 矛盾的根源）。
-/
def consume : Potential → Potential
  | finite (n+1) => finite n
  | finite 0     => finite 0
  | infinite     => infinite

/-- 消耗 k 次（沿运动方向推进 k 单位）。 -/
def consumeN : Nat → Potential → Potential
  | 0,      φ => φ
  | (k+1),  φ => consume (consumeN k φ)

/-- ★D0.2 完美：走势完美 ⟺ Φ = 0（势耗尽）。 -/
def isPerfect : Potential → Prop
  | finite 0 => True
  | _        => False

/-! ## 定理 T₁（势有限） -/

/--
  ★无限势消耗任意有限步仍无限（D0.1 在 `infinite` 上的不动点）。
  这是 T₁ 反证的核心引理：无限势永不被有限运动耗尽。
-/
theorem infinite_never_consumed (k : Nat) : consumeN k infinite = infinite := by
  induction k with
  | zero => rfl
  | succ k ih => simp [consumeN, ih, consume]

/--
  ★无限势永不完美（D0.2 + 上引理）：任意有限步后 `infinite` 都不满足 `isPerfect`。
-/
theorem infinite_never_perfect (k : Nat) : ¬ isPerfect (consumeN k infinite) := by
  rw [infinite_never_consumed]
  simp [isPerfect]

/--
  ★定理 T₁（势有限，necessity_derivation §1 T₁，条件化 L0）：

  陈述（原文）：∀ 走势 m，Φ(m) < ∞。
  证明（原文反证）：设 Φ=∞，由 D0.1 无限势消耗后仍 > 0，永不完美，与 A₀（走势终完美=
  ∃ 有限 τ 完美）矛盾。

  ★条件化（codex 裁决）：A₀ 作 hypothesis `a0_perfection`——"存在有限步 k 使势完美"。
  在该前提下，势不可能是 `infinite`（否则违反 `infinite_never_perfect`）⟹ 势有限。
  命名标注 A₀ 前提：`a0_eventual_perfection_implies_finite`。
-/
theorem a0_eventual_perfection_implies_finite (φ : Potential)
    (a0_perfection : ∃ k, isPerfect (consumeN k φ)) :
    ∃ n, φ = finite n := by
  cases φ with
  | finite n => exact ⟨n, rfl⟩
  | infinite =>
      obtain ⟨k, hk⟩ := a0_perfection
      exact absurd hk (infinite_never_perfect k)

/-! ## 定理 T₂（走势必然完美） -/

/--
  ★消耗的一般引理（D0.1 严格正消耗）：`finite (i + base)` 消耗 i 次 ↦ `finite base`。
  对消耗步数 i 归纳——每步把储备减一，i 步后剩 base。
-/
theorem consumeN_finite_general (i base : Nat) :
    consumeN i (finite (i + base)) = finite base := by
  induction i generalizing base with
  | zero => simp [consumeN]
  | succ i ih =>
      -- consumeN (i+1) (finite ((i+1)+base)) = consume (consumeN i (finite ((i+1)+base)))
      -- (i+1)+base = i+(base+1)，用 ih (base+1) 得 finite (base+1)，再 consume 到 finite base
      have heq : (i + 1) + base = i + (base + 1) := by omega
      calc consumeN (i+1) (finite ((i+1)+base))
          = consume (consumeN i (finite ((i+1)+base))) := rfl
        _ = consume (consumeN i (finite (i+(base+1)))) := by rw [heq]
        _ = consume (finite (base+1)) := by rw [ih (base+1)]
        _ = finite base := rfl

/--
  ★有限势经 n 步消耗归零（D0.1 严格正消耗 + Nat 良基性）。
  `finite n` 消耗 n 次到 `finite 0`——有限非负量在单位正消耗下有限时刻归零。
  由一般引理取 base=0（`finite (n+0) = finite n`）直接得。
-/
theorem finite_consumeN_reaches_zero (n : Nat) : consumeN n (finite n) = finite 0 :=
  consumeN_finite_general n 0

/--
  ★定理 T₂（走势必然完美，necessity_derivation §1 T₂，L0）：

  陈述（原文）：∀ 走势 m，m 必然在有限时刻完美。
  证明（原文）：由 T₁ Φ 有限非负，D0.1 持续正消耗，有限非负量必有限步归零（Φ=0 ⟺ 完美）。

  Lean：有限势 `finite n` 经 n 步消耗到 `finite 0`，满足 `isPerfect`。
  存在有限步使其完美——A₀ 的可操作化（势耗尽=完美的内在必然，非外部停止）。
-/
theorem finite_potential_reaches_perfection (n : Nat) :
    ∃ k, isPerfect (consumeN k (finite n)) :=
  ⟨n, by rw [finite_consumeN_reaches_zero]; trivial⟩

/-! ## 定理 T₁₁（背驰 = 势的自我度量·力量维衰减结构侧） -/

/-
  ★T₁₁ 范围（gap-map §8.2 层A E + codex 裁决 session 019efff0-4776-7263-8632-3aa1e50f3224）：
  分类侧（背驰二分 + 趋势背驰↔type1 + 嵌套收缩）已 D（`Formal.DivergenceNesting`，不重做）。
  本模块只补 **力量维衰减结构骨架**（force-decay skeleton）：力度 = 势的离散读数，
  consume 单调不增 ⟹ 后力度 ≤ 前力度（可比较且有序）；T₂ 势归零 ⟹ 正力度必严格衰减。
  ★**不**声明 MACD 面积 / DIF peak / 价格强弱等 L2 命题（数据/管线层，formalization-validity-domain）；
  势的趋向维（dφ/ds）由操作层 located 斜率承载，不在本 L0 结构骨架内。
-/

/--
  ★力度读数（T₁₁：力度 = 势的离散 Nat 读数，仅在 finite 势上可比较）。
  `forceOf? infinite = none`——无限势不可作有限力度比较（codex 裁决：避免 infinite 占位污染）。
-/
def forceOf? : Potential → Option Nat
  | finite n => some n
  | infinite => none

/--
  ★consume 后力度单调不增（T₁₁：每步推进力度不增，可比较有序）。
  仅对 finite 势：`forceOf? (consume (finite n))` 的 Nat 读数 ≤ `forceOf? (finite n)`。
-/
theorem force_monotone_under_consume (n : Nat) :
    (forceOf? (consume (finite n))).getD 0 ≤ (forceOf? (finite n)).getD 0 := by
  cases n with
  | zero => simp [consume, forceOf?]
  | succ m => simp [consume, forceOf?]

/--
  ★consumeN 后力度单调不增（T₁₁：后段力度 ≤ 前段力度，可比较）。
  中枢后势 = consumeN k（中枢前势）⟹ 后力度 ≤ 前力度（"后不强于前"的有序性）。
-/
theorem force_monotone_under_consumeN (k n : Nat) :
    (forceOf? (consumeN k (finite n))).getD 0 ≤ (forceOf? (finite n)).getD 0 := by
  -- consumeN k (finite n) 恒为 finite（消耗保持 finite），力度由 consumeN_finite 单调
  have hfin : ∀ j, ∃ m, consumeN j (finite n) = finite m := by
    intro j
    induction j with
    | zero => exact ⟨n, rfl⟩
    | succ i ihi =>
        obtain ⟨m, hm⟩ := ihi
        cases m with
        | zero => exact ⟨0, by simp [consumeN, hm, consume]⟩
        | succ p => exact ⟨p, by simp [consumeN, hm, consume]⟩
  induction k with
  | zero => simp [consumeN]
  | succ k ih =>
      obtain ⟨m, hm⟩ := hfin k
      calc (forceOf? (consumeN (k+1) (finite n))).getD 0
          = (forceOf? (consume (consumeN k (finite n)))).getD 0 := rfl
        _ = (forceOf? (consume (finite m))).getD 0 := by rw [hm]
        _ ≤ (forceOf? (finite m)).getD 0 := force_monotone_under_consume m
        _ = (forceOf? (consumeN k (finite n))).getD 0 := by rw [hm]
        _ ≤ (forceOf? (finite n)).getD 0 := ih

/--
  ★正力度经一步 consume 严格衰减（T₁₁ "后弱于前"：力度 > 0 ⟹ 下一步严格下降）。
  这是背驰的结构核心——只要势未耗尽（力度 > 0），力度必然在推进中严格衰减。
-/
theorem positive_force_strictly_decreases_after_one_consume (n : Nat) (hpos : 0 < n) :
    (forceOf? (consume (finite n))).getD 0 < (forceOf? (finite n)).getD 0 := by
  cases n with
  | zero => omega
  | succ m => simp [consume, forceOf?]

/--
  ★完美 ⟺ 力度归零（T₂ + T₁₁：势耗尽=力度衰减到底=背驰完成，仅 finite 势）。
  `isPerfect (finite n) ⟺ forceOf? (finite n) = some 0`——完美的力度显现。
-/
theorem finite_perfect_iff_force_zero (n : Nat) :
    isPerfect (finite n) ↔ forceOf? (finite n) = some 0 := by
  cases n with
  | zero => simp [isPerfect, forceOf?]
  | succ m => simp [isPerfect, forceOf?]

/--
  ★恒定正力度阻断完美（T₁₁ 反证结构，necessity §312：力量不衰减 ⟹ 势不耗尽 ⟹ 违 T₂）。

  若某势在所有有限步力度恒不衰减（永不归零），则它永不完美——与 T₂（走势必然完美）矛盾。
  这正是 necessity 证明的反证骨架："否则力量不衰减，势不耗尽，与 T₂ 矛盾"。
  以 `infinite` 为恒定不衰减力度的载体（consumeN 下力度恒 = none，永不归零，永不完美）。
-/
theorem constant_positive_force_blocks_perfection (k : Nat) :
    forceOf? (consumeN k infinite) = none ∧ ¬ isPerfect (consumeN k infinite) := by
  refine ⟨?_, infinite_never_perfect k⟩
  rw [infinite_never_consumed]
  simp [forceOf?]

/-! ## 定理 T₃（新势必然涌现） -/

/--
  ★走势的运动状态（T₃ necessity 建模）：携带势 + 起始/完美时刻（离散时刻 = Nat，T₅ 离散化）。
  - `potential`：该走势的势（完美态 Φ=0）。
  - `perfectTime`：完美时刻 τ（势耗尽的离散时刻）。
-/
structure MotionState where
  potential : Potential
  perfectTime : Nat

/--
  ★A₁（流动性/价格不静止，necessity_derivation §0.2 A₁）形式化为 **谓词 hypothesis**：
  "零势状态（完美点）之后必然涌现一个新走势，其势 > 0 且开端时刻 = 旧走势完美时刻"。

  这是把 A₁（价格不能永远静止 + 无无运动间隙）作为对"完美点 ↦ 后继走势"的约束。
  ★codex 裁决：A₁ 作 hypothesis（非 axiom 关键字）——Lean 不担保 A₁ 真值（缠论原文担保），
  只证"在 A₁ 成立时 T₃ 成立"。`emergence` 返回后继走势的 witness。
-/
def A1_NoStillness (emergence : MotionState → MotionState) : Prop :=
  ∀ m : MotionState, isPerfect m.potential →
    (∃ n, (emergence m).potential = finite (n+1))            -- 新势 Φ > 0
    ∧ (emergence m).perfectTime = m.perfectTime               -- ★完美时刻 = 新势开端（同一时刻）

/--
  ★定理 T₃（新势必然涌现，necessity_derivation §1 T₃，A₁ 条件定理）：

  陈述（原文）：走势 m 完美后，必然有新走势 m′ 涌现，且 m 的完美时刻 = m′ 的开端时刻。
  证明（原文）：T₂ 完美 Φ=0；A₁ 价格不静止 ⟹ 必有运动；无势运动=静止违 A₁ ⟹ 新运动有势=m′；
  A₁ 无间隙 ⟹ 完美时刻 = 开端时刻。

  ★codex 裁决命名 `t3_from_no_stillness_and_no_gap`（A₁ 条件性显式）：
  在 A₁（`A1_NoStillness emergence`）成立时，任意完美走势 m 涌现的后继 m′ 有正势且时刻衔接。
-/
theorem t3_from_no_stillness_and_no_gap
    (emergence : MotionState → MotionState)
    (a1 : A1_NoStillness emergence)
    (m : MotionState) (hperfect : isPerfect m.potential) :
    (∃ n, (emergence m).potential = finite (n+1))
    ∧ (emergence m).perfectTime = m.perfectTime :=
  a1 m hperfect

/-! ## 定理 T₅（离散化必然 + a0 唯一经验参数） -/

/--
  ★稠密序（连续价格流的模型，necessity_derivation §1 T₅）：
  任两点之间仍有第三点——无 immediate successor（无最小可比较单元）。
  以有理数的稠密性为代表（用 Int 对的"中点不可达"刻画太弱，这里用抽象谓词）。
-/
def DenseOrder (lt : α → α → Prop) : Prop :=
  ∀ a b, lt a b → ∃ c, lt a c ∧ lt c b

/--
  ★稠密序无相邻点（T₅ 前半：连续流无最小可比较单元，L0 标准结构事实）：
  稠密序中不存在"紧邻对"——即不存在 a < b 使得其间无点。这正是"连续价格流中无可比较的点"。
-/
theorem dense_has_no_adjacent (lt : α → α → Prop) (hd : DenseOrder lt)
    (a b : α) (hab : lt a b) : ∃ c, lt a c ∧ lt c b :=
  hd a b hab

/--
  ★离散观测序列（T₅ 后半，codex 裁决：a0 唯一仅在固定 origin + grain generator 内）：
  由单一粒度 a0 与起点 origin 生成的可数点序列 `obs k = origin + k * a0`。
  a0 是 **唯一** 经验参数——序列完全由 (origin, a0) 决定（A₂ 后半"a0 唯一"）。
-/
def discreteObs (origin a0 : Int) (k : Nat) : Int := origin + (k : Int) * a0

/--
  ★离散序列有 successor（T₅：可数离散点有相邻可比较单元，与稠密序对立）。
  相邻观测点 obs(k) 与 obs(k+1) 相差恰一个 a0——存在最小可比较单元（辨识势耗尽的基础）。
-/
theorem discrete_has_successor (origin a0 : Int) (k : Nat) :
    discreteObs origin a0 (k+1) - discreteObs origin a0 k = a0 := by
  unfold discreteObs
  have hc : ((k + 1 : Nat) : Int) = (k : Int) + 1 := by push_cast; rfl
  rw [hc, Int.add_mul, Int.one_mul]
  omega

/--
  ★a0 唯一性（codex 裁决：固定 origin + grain generator 内可证）：
  离散观测序列由单一参数 a0 完全决定——若两序列同 origin 且首个相邻间距相同，则 a0 相同。
  这把"a0 是整个系统唯一经验参数"形式化为"序列的生成参数唯一"（A₂ 概念链环23闭合声明）。
-/
theorem a0_uniqueness (origin a0 a0' : Int)
    (h : discreteObs origin a0 1 - discreteObs origin a0 0
       = discreteObs origin a0' 1 - discreteObs origin a0' 0) :
    a0 = a0' := by
  rw [discrete_has_successor, discrete_has_successor] at h
  exact h

/-! ## 定理 T₁₄（BSP 首尾相连，located 流交替）+ T₁₅（结构配对，价格 zigzag = L2 经验） -/

/--
  ★根操作状态机的仓位态（T₁₄ necessity，codex 裁决：用事件状态机非裸 List Side）。
  located 势源流驱动根 F/C 操作：long → flip short → flip long（状态机内蕴交替约束）。
-/
inductive PositionState where
  | flat    -- 初始空仓
  | long    -- 持多
  | short   -- 持空
deriving DecidableEq, Repr

open PositionState

/--
  ★located 事件（带 side + 离散时刻 time，T₅ 离散化）：走势完美点驱动的操作事件。
  sell = 走势完美（顶背驰）→ 翻空；buy = 反向走势完美（底背驰）→ 翻多。
-/
structure LocatedEvent where
  side : Side
  time : Nat
deriving Repr

/--
  ★状态机单步（T₁₄ located 流根操作状态机，codex 裁决）：
  - 在 long/flat 态遇 sell 事件 ↦ 翻 short（卖出走势完美点）。
  - 在 short/flat 态遇 buy 事件 ↦ 翻 long（买入走势完美点）。
  内蕴交替约束：long 后只接 sell（不能再 buy），short 后只接 buy——状态机拒绝同向连续。
-/
def step : PositionState → Side → Option PositionState
  | flat,  Side.sell => some short
  | flat,  Side.buy  => some long
  | long,  Side.sell => some short      -- 持多遇卖点 → 翻空
  | long,  Side.buy  => none            -- ★持多不能再买（拒绝同向连续，located 交替内蕴）
  | short, Side.buy  => some long       -- 持空遇买点 → 翻多
  | short, Side.sell => none            -- ★持空不能再卖（拒绝同向连续）

/--
  ★状态机运行（T₁₄ located 流：从初态消费事件序列，全程合法 = 输出末态）。
  返回 `none` 表示流非法（出现同向连续）；`some s` 表示合法运行到末态 s。
-/
def run : PositionState → List LocatedEvent → Option PositionState
  | s, [] => some s
  | s, e :: rest =>
      match step s e.side with
      | some s' => run s' rest
      | none => none

/--
  ★sides 严格交替谓词（T₁₄ located 流首尾相连 = 无同向连续间隙）。
-/
def Alternating : List Side → Prop
  | [] => True
  | [_] => True
  | a :: b :: rest => a ≠ b ∧ Alternating (b :: rest)

/--
  ★状态机一步后状态由 side 唯一决定方向（辅助：合法步必翻转到对侧）。
-/
theorem step_flips (s : PositionState) (sd : Side) (s' : PositionState)
    (h : step s sd = some s') :
    (sd = Side.sell ∧ s' = short) ∨ (sd = Side.buy ∧ s' = long) := by
  cases s <;> cases sd <;> simp [step] at h <;>
    first
      | (subst h; first | exact Or.inl ⟨rfl, rfl⟩ | exact Or.inr ⟨rfl, rfl⟩)

/--
  ★定理 T₁₄（located 流 sides 交替，necessity_derivation §1 T₁₄，L0 状态机结构定理）：

  陈述（原文，located 流侧）：每个走势完美点（卖点）同时是反向走势起点（买点候选）⟹
  located 势源流在时间轴上首尾相连，严格交替 buy/sell。
  编排者裁决：raw 流 ~（不交替，非 bug）/ located 流 ✓（严格交替，根操作状态机内蕴）。

  ★codex 裁决命名 `located_state_machine_sides_alternate`（非完整 no_signal_gap）：
  任意被状态机合法接受（`run flat evs = some _`）的 located 事件流，其 sides 必严格交替。
  这是 located 流交替性的结构证明；raw 流 **不**加此约束（诚实标注 raw ~，见模块头）。
-/
theorem located_state_machine_sides_alternate :
    ∀ (s : PositionState) (evs : List LocatedEvent),
      (run s evs).isSome = true → Alternating (evs.map LocatedEvent.side)
  | _, [], _ => trivial
  | _, [_], _ => by simp [Alternating]
  | s, e1 :: e2 :: rest, h => by
      -- 解析首步：step s e1.side = some s1，且 run s1 (e2::rest) 合法
      simp only [run] at h
      cases hstep1 : step s e1.side with
      | none => rw [hstep1] at h; simp at h
      | some s1 =>
          -- 第二步必须合法（否则整体 none）；hstep1 化简首步后内层 run 由 match 方程自动展开
          simp only [hstep1] at h
          cases hstep2 : step s1 e2.side with
          | none => rw [hstep2] at h; simp at h
          | some s2 =>
              rw [hstep2] at h
              -- s1 由 e1.side 决定（sell→short / buy→long）
              -- s1 接受 e2.side 合法 ⟹ e2.side 与 e1.side 必相异
              have h1 := step_flips s e1.side s1 hstep1
              have hne : e1.side ≠ e2.side := by
                rcases h1 with ⟨hs, hsh⟩ | ⟨hb, hl⟩
                · -- e1.side = sell, s1 = short；short 只接 buy（sell→none）
                  subst hsh
                  intro heq
                  rw [hs] at heq
                  -- e2.side = sell ⟹ step short sell = none，矛盾 hstep2
                  rw [← heq, ← hs] at hstep2
                  rw [hs] at hstep2
                  simp [step] at hstep2
                · -- e1.side = buy, s1 = long；long 只接 sell（buy→none）
                  subst hl
                  intro heq
                  rw [hb] at heq
                  rw [← heq, ← hb] at hstep2
                  rw [hb] at hstep2
                  simp [step] at hstep2
              -- 递归：run s1 (e2::rest) 合法
              have hrec : (run s1 (e2 :: rest)).isSome = true := by
                simp only [run, hstep2]
                exact h
              have := located_state_machine_sides_alternate s1 (e2 :: rest) hrec
              simp only [List.map, Alternating]
              exact ⟨hne, this⟩

/-! ### T₁₅ 结构配对（L0）+ 价格 zigzag（L2 经验，非 Lean 可证） -/

/--
  ★located 配对（T₁₅ ① 降成本配对结构，codex 裁决：仅结构，禁价格语义命名）：
  一个 located 卖点 spawn（E：开空）+ 后继 located 买点 close（D：回补）= 一对方向相反的事件。
  结构良构 = 第一个事件 side=sell（卖点 spawn）、第二个 side=buy（买点 close）。
-/
def WellFormedPairing (e_open e_close : LocatedEvent) : Prop :=
  e_open.side = Side.sell ∧ e_close.side = Side.buy ∧ e_open.time < e_close.time

/--
  ★定理 T₁₅ ①（located 配对结构良构，necessity_derivation §1 T₁₅，L0 结构定理）：

  陈述（原文 ① 部分）：降成本盈利的 **结构保证** = child 空头在 located 卖点 spawn（E）+
  在 located 买点回补（D），方向由 located 流交替（T₁₄/S11）保证 ⟹ 同股数进出配对结构正确
  （卖在卖点 / 买在买点）。

  ★codex 裁决命名 `alternating_located_flow_has_sell_buy_pairing`（禁 cost_reduction）：
  给定两个相邻交替 located 事件（首 sell 次 buy）且时间递增，配对结构良构。
  这是从 T₁₄ 交替性派生的 **结构** 命题——**不**涉及价格、数量、账户、regime。
-/
theorem alternating_located_flow_has_sell_buy_pairing
    (e_open e_close : LocatedEvent)
    (hsell : e_open.side = Side.sell)
    (hbuy : e_close.side = Side.buy)
    (htime : e_open.time < e_close.time) :
    WellFormedPairing e_open e_close :=
  ⟨hsell, hbuy, htime⟩

/-
  ★★★ T₁₅ ② 价格 zigzag（买点价 < 前卖点价）= **L2 经验，非 Lean 可证** ★★★

  necessity_derivation §1 T₁₅ ② 编排者两阶段裁决 + 谱系 project_signal_layer_duality：
  价格 zigzag（买<前卖 / 卖>前买）在 located 势源流 **违反 ~50%**（CL 5/6、BRN 6/11）——
  根翻空发生在走势完美点（type1 顶背驰），与"翻空价高于入场"**无必然关系**（regime 依赖）。
  S9 设观测计数（非 panic）——若 panic 会使 BRN 崩溃（formalization-validity-domain）。

  ★no-workaround / no-patch-mentality / formalization-validity-domain 联合裁定（codex PASS）：
  价格序由 regime 决定（**螺旋 ⊥ 价格**：价格是 ambient 嵌入坐标），**不是结构必然**。
  因此本模块 **刻意不** 写 `buy_price_lt_previous_sell_price` 之类的 Lean 定理——
  写出来必然是伪证（在数据上违反~50%，结构 Lean 化 = 声明膨胀 = 把 L2 经验冒充 L0 必然）。
  价格 zigzag 的诚实归宿：**L2 经验命题**，由 Rust `prove_s11_s9_located` 观测计数承载
  （L2，regime 依赖，谱系 project_nrf_v4_strict_accounting），不进入 Lean theorem 结论。

  Lean 能严格证的只有 T₁₅ ① 的 **配对结构**（上方 `alternating_located_flow_has_sell_buy_pairing`）；
  T₁₅ ② 的价格不等式超出 L0 结构有效域，诚实标注，不伪造。
-/

end Tlayers.Signal
