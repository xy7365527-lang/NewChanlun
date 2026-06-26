/-
  无前视全定义状态转移 δ（chanlun-strict-classification-standard 第3部分，task #66 / T-causal）

  上游标准（chanlun-strict-classification-standard.md 第一部分 B 第3部分）：
    δₗ : Sₗ × Eₗ → Sₗ 全函数且确定（∀(s,e) ∃!s'）；
    Cₗ(h ⌢ e) = δₗ(Cₗ(h), e)（分类 = 在状态上跑一步 δ）；
    因果无前视 ω₀:ₜ = ω'₀:ₜ ⟹ Cₜ(ω) = Cₜ(ω')（t 时刻分类只依赖 t 及之前的数据）。

  缠论依据（因果性的领域来源）：
  - 第29课 since<bar / L_confirm 因果字段（DivergenceNesting：区间套深度由 *已出现* 的嵌套链读出，
    不前向等待未来 K 线）；Signal 层 located 流状态机 `run`（since<bar：confirm 用已出现数据，
    不偷看未来）——本模块是这两处因果性的 **结构形式**（把"只用过去"从个案提升为通用 δ 定理）。

  范式（继承 Formal/ + Tlayers/Signal）：纯 inductive + List.foldl 在线语义，不依赖 Mathlib，
  禁 sorry/admit/axiom，认识论 **L0**（δ 全定义 / 无前视是结构性质，定义内蕴）。

  ★依赖最小化（本体修正）：只 import 自包含的 `Formal.TrendTrichotomy`。笔方向用
  `TrendKind.Direction`（up/down）而非 `BSPLabels.Side`（buy/sell）——笔的本体属性是
  **方向**（向上笔/向下笔，原则13），不是"买卖"；买卖是操作层叠加的语义。用 `Direction`
  比借用 `Side` 更忠实缠论本体，且解除对 BSPLabels 的偶然依赖。

  ★严格性核心（formalization-validity-domain：无前视必须带信息增量，不能平凡成立）：
  若把 t 时刻分类直接定义为 `fold δ s₀ (take t ω)`，则 `take t ω = take t ω' ⟹ 分类相等`
  退化为 `congrArg` 同义反复（L0 零增量）。为避免这种平凡，本模块 **对比两个分类器**：
  - `classifyAt`（因果，只 fold 前缀）：证 **满足** 无前视；
  - `peekAt`（非因果，把"未来事件总数"掺进 t 时刻读数）：证 **违反** 无前视（给出反例）。
  无前视定理因此 **否证** "分类可依赖未来"——它区分因果/非因果两个分类器类（否定性结果，
  缩小有效域边界），不是 take 的 congrArg 同义反复。δ 的签名 `State → Event → State`
  （一次只吃一个事件、状态只承载已处理事件）是因果性得以成立的结构根源。
-/

import Formal.TrendTrichotomy

namespace Tlayers.Dynamics

open Formal.TrendTrichotomy (TrendKind isTrend Direction)

/-! ## 1. 事件 Eₗ 与状态 Sₗ（全定义的载体） -/

/--
  ★事件 Eₗ（标准第3部分 Eₗ）：驱动状态前进的离散输入，**穷尽**所有可能的增量。

  缠论在线推进只有两类增量事件（在线分类系统的输入字母表）：
  - `newBar`：新 K 线到达（携带涨跌 = 该 bar 相对前 bar 的涨跌，T₅ 离散时刻推进的最小单元）。
  - `newStroke`：新笔确认（携带方向 up/down = 笔的方向，笔是走势的最小构成元，原则13）。

  ★全定义关键（no-workaround）：事件类型 inductive **穷尽** ⟹ δ 对每个构造子都有分支，
  不存在"理论没规定的事件"。新增事件种类 = 新增构造子 = δ 必须扩展分支（编译器强制穷尽）。
-/
inductive Event where
  | newBar (rising : Bool)        -- 新 K 线（rising=该 bar 收涨）
  | newStroke (dir : Direction)   -- 新笔确认（dir=笔方向 up/down）
deriving DecidableEq, Repr

open Event

/--
  ★状态 Sₗ（标准第3部分 Sₗ）：分类系统在某时刻的 **完整内部状态**（含已完成结构 + 未完成尾部）。

  ★全定义关键（标准"理论没有规定这种情况不可出现"）：状态字段必须让 δ 对 *任意* (s,e)
  都有确定的下一状态——本结构用 Nat 计数 + Option 尾部，对任意输入都封闭（无未定义洞）。

  - `barCount`：已处理 K 线数（T₅ 离散时刻 = 已消费事件的计数，单调不减）。
  - `strokeCount`：已确认笔数（笔级结构积累）。
  - `lastStrokeDir`：最近一笔方向（`none` = 尚无笔；未完成尾部的状态承载）。
  - `pendingRise`：自上一笔以来的净涨 bar 计数（未完成走势的尾部状态，下一笔确认时清零）。

  所有字段都从 **已出现** 的事件读出（因果性的状态侧根源：状态不含任何未来信息）。
-/
structure State where
  barCount : Nat
  strokeCount : Nat
  lastStrokeDir : Option Direction
  pendingRise : Nat
deriving DecidableEq, Repr

/-- 初始状态 s₀（空历史：无 bar、无笔、无尾部）。 -/
def s0 : State := { barCount := 0, strokeCount := 0, lastStrokeDir := none, pendingRise := 0 }

/-! ## 2. 状态转移 δ（全函数且确定） -/

/--
  ★状态转移 δ : Sₗ × Eₗ → Sₗ（标准第3部分核心）。

  Lean 全函数 ⟹ 自动 total（对每个输入有输出）+ 确定（同输入同输出，函数外延性）。
  ★对 `Event` 的两个构造子 **都有分支**（穷尽），对 `State` 的任意取值都返回确定的新 `State`
  （Nat/Option 运算全定义）——不存在"δ 在某事件下无定义"的洞（no-workaround：若某事件下
  下一状态无法严格确定，那是真矛盾须 escalate；此处 δ 对全定义域封闭，无矛盾）。

  - `newBar rising`：bar 计数 +1；涨 bar 累积 pendingRise（未完成尾部）。
  - `newStroke dir`：笔计数 +1；记录笔方向；清零 pendingRise（尾部被新笔吸收 = 结构完成一段）。
-/
def delta : State → Event → State
  | s, newBar rising =>
      { s with
        barCount := s.barCount + 1
        pendingRise := if rising then s.pendingRise + 1 else s.pendingRise }
  | s, newStroke dir =>
      { s with
        strokeCount := s.strokeCount + 1
        lastStrokeDir := some dir
        pendingRise := 0 }

/--
  ★δ 全函数且确定（标准第3部分 ∀(s,e) ∃!s'，L0）。

  Lean 函数 `delta` 已是全函数（每个 (s,e) 有值）+ 确定（函数即单值）。本定理把标准的
  `∃! s', δ s e = s'` 显式展开（本工程不依赖 Mathlib，无 `∃!` notation，故手写
  `∃ s', δ s e = s' ∧ ∀ y, δ s e = y → y = s'` —— 这正是 `ExistsUnique` 的定义展开）：
  存在唯一的 s' 使 δ s e = s'，即 s' = delta s e。唯一性由"另一个满足等式的 y 也等于
  delta s e"给出——这正是函数确定性的形式表达。

  非平凡内容（非纯 trivial）：它确认 δ 把"对每个 (s,e) 恰好一个后继状态"落实为定理——
  全定义域无遗漏（total）+ 无歧义（单值）。这是分类系统 **在该事件集下封闭** 的证明。
-/
theorem delta_total_deterministic (s : State) (e : Event) :
    ∃ s', delta s e = s' ∧ ∀ y, delta s e = y → y = s' := by
  refine ⟨delta s e, rfl, ?_⟩
  intro y h
  exact h.symm

/-- ★δ 对每个事件构造子都有定义（穷尽，L0）：newBar / newStroke 均落到确定的新状态。 -/
theorem delta_defined_on_all_events (s : State) :
    (∃ s', delta s (newBar true) = s') ∧
    (∃ s', delta s (newBar false) = s') ∧
    (∀ dir, ∃ s', delta s (newStroke dir) = s') :=
  ⟨⟨_, rfl⟩, ⟨_, rfl⟩, fun _dir => ⟨_, rfl⟩⟩

/-! ## 3. 分类 C 与一致性 C(h ⌢ e) = δ(C h, e) -/

/--
  ★分类函数 C（标准第3部分 Cₗ）：历史 h（事件序列）的当前状态 = 从 s₀ 顺序跑 δ。

  `classify h = h.foldl delta s0`——在线语义：每来一个事件在状态上跑一步 δ。
  这是缠论"逐 K 线推进、不回头重算"的形式：状态是历史的左折叠摘要。
-/
def classify (h : List Event) : State := h.foldl delta s0

/--
  ★分类从任意起始状态（辅助：one-step 一致性的一般形式）。
  `classifyFrom s h = h.foldl delta s`——从状态 s 出发跑历史 h。
-/
def classifyFrom (s : State) (h : List Event) : State := h.foldl delta s

/--
  ★分类一致性（标准第3部分 Cₗ(h ⌢ e) = δₗ(Cₗ(h), e)，L0）。

  历史 h 追加单个事件 e 后的分类 = 在 h 的分类状态上跑一步 δ。
  这是 `List.foldl_concat`（foldl 对 ++[e] 的展开）的直接实例——把标准的"追加一个事件 =
  在状态上跑一步 δ"落实为定理。在线性（增量计算 = 全量重算）由此保证：
  不必对 h⌢e 从头 fold，只需在 C(h) 上跑 δ 一步。
-/
theorem classify_step (h : List Event) (e : Event) :
    classify (h ++ [e]) = delta (classify h) e := by
  unfold classify
  rw [List.foldl_append]
  rfl

/--
  ★分类一致性（cons 形式，L0）：从状态 s 出发，先吃 e 再吃 rest = 在 δ s e 上吃 rest。
  这是 foldl 的 cons 展开，配合 `classify_step` 给出 δ 的在线推进双向刻画。
-/
theorem classifyFrom_cons (s : State) (e : Event) (rest : List Event) :
    classifyFrom s (e :: rest) = classifyFrom (delta s e) rest := by
  unfold classifyFrom
  rfl

/-! ## 4. 因果无前视（标准第3部分核心，最关键） -/

/--
  ★t 时刻前缀（标准 ω₀:ₜ）：取走势流 ω 的前 t 个事件。
  `prefixAt t ω = ω.take t`——"t 及之前的数据"，因果分类的输入。
-/
def prefixAt (t : Nat) (ω : List Event) : List Event := ω.take t

/--
  ★因果分类器 Cₜ（标准 Cₜ(ω)）：t 时刻的分类 = 对前缀 ω₀:ₜ 跑 classify。
  `classifyAt t ω = classify (prefixAt t ω)`——只读 t 及之前，**结构上**不接触未来事件。
-/
def classifyAt (t : Nat) (ω : List Event) : State := classify (prefixAt t ω)

/--
  ★因果无前视（标准第3部分核心定理，L0）：
    ω₀:ₜ = ω'₀:ₜ ⟹ Cₜ(ω) = Cₜ(ω')。

  若两条走势流在 t 时刻之前（含 t）完全一致，则它们在 t 时刻的分类相同——
  t 时刻分类 **不依赖** t 之后的任何数据。这是 L_confirm/located since<bar 因果性的结构形式：
  confirm 只用已出现的数据，不前向等待未来 K 线（第29课）。

  ★诚实标注（codex 异质审计 session 019f0018 吸收）：对具体的 `classifyAt`（已被定义成
  "先 take 前缀再分类"），本定理由 `rw [hpre]` 直接成立——信息增量来自 `classifyAt`
  的 **定义约束**（未来数据在定义层已不可访问），不是这行证明本身。本定理的真正严格内容
  在下方 `causal_iff_factors_through_prefix`：把"无前视"对 **任意抽象分类器** 刻画为
  "可经前缀因子分解"的充要条件——这才把无前视从单点平凡提升为特征定理（codex 给出的严格形式）。
-/
theorem no_lookahead (t : Nat) (ω ω' : List Event)
    (hpre : prefixAt t ω = prefixAt t ω') :
    classifyAt t ω = classifyAt t ω' := by
  unfold classifyAt
  rw [hpre]

/-! ### 4b. 无前视的特征定理（codex 严格形式：充要刻画，非平凡合取） -/

/--
  ★分类器的无前视谓词 `IsCausal`（对 **任意** 分类器 `F : Nat → List Event → α`）：
    ∀ t ω ω', ω₀:ₜ = ω'₀:ₜ → F t ω = F t ω'。

  这是把标准第3部分的"因果无前视"提取为对抽象分类器的 **性质**（不绑定 classify 的实现）——
  好让"满足无前视"成为可被任意分类器检验的判别标准，而非某个具体函数的平凡推论。
-/
def IsCausal {α : Type} (F : Nat → List Event → α) : Prop :=
  ∀ (t : Nat) (ω ω' : List Event), prefixAt t ω = prefixAt t ω' → F t ω = F t ω'

/--
  ★无前视特征定理（codex 异质审计给出的严格形式，L0）：
    IsCausal F ↔ ∃ G, ∀ t ω, F t ω = G t (prefixAt t ω)。

  一个分类器 **无前视** 当且仅当它 **可经 t 时刻前缀因子分解**（存在 G 使 F t ω 只通过
  ω₀:ₜ 计算）。这是无前视的 **充要刻画**，把它从"对某个具体 classifyAt 的平凡定理"提升为
  对所有分类器成立的特征定理（codex session 019f0018 指出的严格形式）：

  - (→) 无前视 ⟹ 可因子分解：取 `G t p := F t p`（前缀本身作 G 输入），需证
    `F t ω = F t (prefixAt t ω)`。由 `take_take`/`min_self`：`prefixAt t (prefixAt t ω) = prefixAt t ω`，
    故二者前缀相等，无前视给出相等。**这一步非平凡**——它真正用到了 F 不偷看未来。
  - (←) 可因子分解 ⟹ 无前视：若 F 经 G∘prefix 计算，则前缀相等 ⟹ G 输入相等 ⟹ 输出相等。

  这关死 codex 指出的"`no_lookahead_is_characteristic` 只是合取、命名偏强"问题：
  特征性现在是真正的 `↔` 定理，不是两个孤立命题的合取。
-/
theorem causal_iff_factors_through_prefix {α : Type} (F : Nat → List Event → α) :
    IsCausal F ↔ ∃ G : Nat → List Event → α, ∀ t ω, F t ω = G t (prefixAt t ω) := by
  constructor
  · intro hcausal
    refine ⟨fun t p => F t p, ?_⟩
    intro t ω
    apply hcausal
    -- prefixAt t ω = prefixAt t (prefixAt t ω)：take t (take t ω) = take t ω
    show ω.take t = (ω.take t).take t
    rw [List.take_take, Nat.min_self]
  · rintro ⟨G, hG⟩ t ω ω' hpre
    rw [hG t ω, hG t ω', hpre]

/--
  ★因果分类器满足无前视（特征定理的正向实例，L0）：`classifyAt` 是 `IsCausal`。
  由它经前缀因子分解（`classifyAt t ω = classify (prefixAt t ω)`，取 G = classify∘snd）
  + 特征定理的 (←) 方向得到。`classifyAt` 落在 `IsCausal` 这一侧。
-/
theorem classifyAt_isCausal : IsCausal classifyAt := by
  rw [causal_iff_factors_through_prefix]
  exact ⟨fun _ p => classify p, fun _ _ => rfl⟩

/-! ### 4c. 非因果分类器的反例（无前视的信息增量 = 否证"可偷看未来") -/

/--
  ★非因果分类器 peekAt（**故意** 偷看未来：把整条流的总长度掺进 t 时刻读数）。

  `peekAt t ω = (classify (prefixAt t ω)).barCount + ω.length`——
  它在因果分类的基础上 **加上整条流 ω 的总事件数**（含 t 之后的未来事件）。
  这是"前向等待 / 偷看未来"的最小形式：t 时刻的输出依赖 t 之后还会来多少事件。
  缠论禁止这种分类（第29课 since<bar：confirm 不能等未来），本定义是被否证的对象。
-/
def peekAt (t : Nat) (ω : List Event) : Nat :=
  (classify (prefixAt t ω)).barCount + ω.length

/--
  ★非因果分类器违反无前视（否定性结果，L0）：`peekAt` **不是** `IsCausal`。

  反例：t = 0，ω = [] （空流），ω' = [newBar true]（多一个未来事件）。
  - 前缀相等：prefixAt 0 ω = take 0 [] = [] = take 0 [newBar true] = prefixAt 0 ω'（✓ t 时刻前一致）。
  - peek 不等：peekAt 0 ω = 0 + 0 = 0；peekAt 0 ω' = 0 + 1 = 1（✗ 因偷看了未来长度）。

  由特征定理 `causal_iff_factors_through_prefix`，`¬ IsCausal peekAt` 等价于"peekAt **不可**
  经前缀因子分解"——即它真正依赖了 t 之后的数据。这 **否证** "t 时刻分类可以偷看未来"：
  存在分类器满足前缀相等却给出不同输出，正因它读了未来。
  （formalization-validity-domain：否定性结果缩小有效域——只有不偷看未来的分类器满足无前视。）
-/
theorem peekAt_not_isCausal : ¬ IsCausal peekAt := by
  intro hcausal
  -- 实例化无前视谓词到反例，导出 0 = 1 矛盾
  have h : peekAt 0 [] = peekAt 0 [newBar true] := by
    apply hcausal 0 [] [newBar true]
    rfl
  simp only [peekAt, prefixAt, classify] at h
  -- h : ... + [].length = ... + [newBar true].length，即 0 = 1
  exact absurd h (by decide)

/--
  ★无前视是分类器的判别标准（综合，L0）：因果分类器 ∈ IsCausal，非因果分类器 ∉ IsCausal。

  把 `classifyAt_isCausal`（classifyAt 满足）与 `peekAt_not_isCausal`（peekAt 违反）合为一条，
  使"`IsCausal` 真的把分类器划成两类"显式化。**与旧版（被 codex 否定的合取命名）的区别**：
  此处的 `IsCausal` 已由 `causal_iff_factors_through_prefix` 给出 **充要刻画**——
  满足 IsCausal ⟺ 可经前缀因子分解 ⟺ 不依赖未来。本命题是该特征定理在两个具体分类器上的
  判别结果，不再是"命名偏强的孤立合取"。
-/
theorem isCausal_separates_classifiers :
    IsCausal classifyAt ∧ ¬ IsCausal peekAt :=
  ⟨classifyAt_isCausal, peekAt_not_isCausal⟩

/-! ### 4c. 因果性的状态侧根源（状态只承载过去，单调推进） -/

/--
  ★δ 单调推进 bar 计数（因果性状态侧：状态只增不减，是过去事件的累积摘要）。
  任意一步 δ 后 barCount ≥ 原 barCount——状态不会"丢失"已处理的过去，也不预支未来。
-/
theorem delta_barCount_monotone (s : State) (e : Event) :
    s.barCount ≤ (delta s e).barCount := by
  cases e with
  | newBar rising => simp [delta]
  | newStroke dir => simp [delta]

/--
  ★分类的 bar 计数 = 历史的 bar 事件数（状态忠实摘要过去，L0）。

  `(classify h).barCount = (h.filter isBar).length`——分类状态的 bar 计数恰好等于
  历史 h 中 newBar 事件的个数。这坐实状态是 **过去事件的确定性摘要**（因果性根源）：
  状态完全由已出现的事件决定，不含未来信息（无前视在状态侧的体现）。
-/
def isBar : Event → Bool
  | newBar _ => true
  | newStroke _ => false

theorem classify_barCount_eq_filter (h : List Event) :
    (classify h).barCount = (h.filter isBar).length := by
  unfold classify
  suffices H : ∀ (s : State) (l : List Event),
      (l.foldl delta s).barCount = s.barCount + (l.filter isBar).length by
    have := H s0 h
    simpa [s0] using this
  intro s l
  induction l generalizing s with
  | nil => simp
  | cons e rest ih =>
      cases e with
      | newBar rising =>
          simp only [List.foldl_cons, isBar, List.filter_cons]
          rw [ih (delta s (newBar rising))]
          simp [delta]
          omega
      | newStroke dir =>
          simp only [List.foldl_cons, isBar, List.filter_cons]
          rw [ih (delta s (newStroke dir))]
          simp [delta]

end Tlayers.Dynamics

/-
  ════════════════════════════════════════════════════════════════════════════
  结果包（result-package.md 六要素，task #66/#64 T-causal）
  ════════════════════════════════════════════════════════════════════════════

  【1. 结论】
  formal/Tlayers/Dynamics.lean 形式化标准第3部分"无前视全定义状态转移 δ"：
  - `delta : State → Event → State` 全函数且确定（`delta_total_deterministic`：∃!s'
    的无-Mathlib 手写展开 ∃s'.δse=s'∧∀y.δse=y→y=s'；`delta_defined_on_all_events`：
    对每个 Event 构造子有定义，无"理论没规定的事件"洞）。
  - 分类一致性 C(h⌢e)=δ(C h,e)（`classify_step`，C=foldl δ s0）。
  - 因果无前视 ω₀:ₜ=ω'₀:ₜ⟹Cₜ(ω)=Cₜ(ω')（`no_lookahead`），并提升为充要特征定理
    `causal_iff_factors_through_prefix`：IsCausal F ↔ ∃G,∀tω,F t ω=G t(prefixAt t ω)。
    `peekAt_not_isCausal` 反例否证"可偷看未来"。
  全文件无 sorry/admit/axiom；11 定理仅依赖 Lean 标准 propext/Quot.sound（多数零公理）。
  codex gpt-5.5 high 三轮异质审计 PASS（session 019f0018→019f001b→019f001f）。

  【2. 定义依据】
  - 标准第3部分（chanlun-strict-classification-standard.md:35）：δₗ:Sₗ×Eₗ→Sₗ 全函数且确定
    ∀(s,e)∃!s'；Cₗ(h⌢e)=δₗ(Cₗ(h),e)；因果无前视 ω₀:ₜ=ω'₀:ₜ⟹Cₜ(ω)=Cₜ(ω')。
  - 缠论第29课 since<bar（confirm 用已出现数据，不前向等待）：本模块状态字段全从已出现事件
    读出（`classify_barCount_eq_filter`：状态是过去事件的确定性摘要），无未来信息——这是因果性
    的状态侧根源。Event 二构造子（newBar/newStroke）= 在线推进的输入字母表（原则13 笔为最小构成元）。
  - 笔方向用 `TrendKind.Direction`(up/down)（DivergenceNesting/Signal 用 Direction 建模方向），
    非 BSPLabels.Side(buy/sell)——笔的本体属性是方向，买卖是操作层叠加（codex 确认本体修正）。

  【3. 边界条件】（结论翻转条件）
  - 若 `Event` 不穷尽真实缠论增量（如遗漏"新线段确认/新中枢确认"事件），则 δ 全定义只在
    当前字母表内成立，不覆盖真实缠论——这是建模充分性问题，Lean 不自动证（codex 明确标注）。
    新增事件种类 ⟹ 新增构造子 ⟹ δ 必须扩展分支（编译器强制穷尽）。
  - 若把 `classifyAt` 改为依赖 `ω` 整体（如 peekAt 读 ω.length），无前视立即失败
    （`peekAt_not_isCausal` 即此反例）——无前视仅对"经前缀因子分解"的分类器成立。
  - 认识论 L0：δ 全定义/无前视是结构性质（定义内蕴）。lake 通过=逻辑正确，非实证有效域，
    不得膨胀（formalization-validity-domain）。无前视的信息增量来自 `peekAt` 否定性结果
    （区分因果/非因果分类器类），非 take 的 congrArg 同义反复。

  【4. 下游推论】
  - δ 在线一致性 ⟹ 增量计算=全量重算（不必对 h⌢e 从头 fold，只在 C(h) 上跑 δ 一步），
    为 Rust recursive_t 的在线推进（逐 K 线不回头）提供形式契约。
  - 因果无前视特征定理 ⟹ 任何 confirm/located 分类器若声称无前视，必须可经前缀因子分解；
    L_confirm（DivergenceNesting）/located 流（Signal.run）的 since<bar 因果性获得通用结构形式。
  - State/Event/δ 可作为标准第6部分 π:Sₗ→A 完全应对的状态侧载体（π 依赖 Cₗ=δ 的输出）。

  【5. 谱系引用】
  - 标准第三部分 C14/新建项"动态状态转移 δ·无前视"（chanlun-strict-classification-standard.md:163）。
  - 继承 Signal 层 located 因果性（T₁₄ run 状态机）+ DivergenceNesting L_confirm 因果字段。
  - formalization-validity-domain 222/223/230/231：本模块严守 L0 标注，无前视用否定性结果
    （peekAt 反例）避免"定义域=有效域"膨胀（230 直积退化的同型禁忌）。
  - no-patch-mentality 087/089/090：codex 首轮指出 `no_lookahead_is_characteristic` 命名偏强
    （平凡合取冒充特征定理）= 声明膨胀，已删除并重写为真正充要特征定理
    `causal_iff_factors_through_prefix`（"追问严格的形式是什么"→重写，非降级命名）。

  【6. 影响声明】
  - 新增文件 formal/Tlayers/Dynamics.lean（横切新建项，代码库此前完全缺失 δ 形式化）。
  - 依赖 formal/Formal/TrendTrichotomy.lean（自包含，未改动）。
  - 未改 lakefile.toml / Formal.lean（按 Lead 约束，单文件 lake env lean 验证）。
  - 解除了对 Formal.BSPLabels 的依赖（该文件被并行工位 #60 改为 import 生成态 Strict 库）——
    本模块改用 Direction，与 BSPLabels/Strict 的生成态完全解耦，不受其影响。
-/
