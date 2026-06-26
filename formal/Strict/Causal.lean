/-
  Strict/Causal.lean — 无前视全定义状态转移（最严格完全分类标准 第3部分，L0）
  ★t-causal 工位（任务 #64 / #66；598/603/615 谱系）

  ── 标准第3部分（chanlun-strict-classification-standard.md:35）三项严格陈述 ──
    (a) δₗ : Sₗ × Eₗ → Sₗ **全函数且确定**（∀(s,e) ∃! s'）。
    (b) **状态转移一致** Cₗ(h ⌢ e) = δₗ(Cₗ(h), e)（分类与状态机 fold 一致）。
    (c) **因果无前视** ω₀:ₜ = ω'₀:ₜ ⟹ Cₜ(ω) = Cₜ(ω')（当下状态只依赖已出现数据）。

  ── codex#2 缺口 + 本文件的严格裁定（no-workaround / formalization-validity-domain） ──

  codex#2 判第3部分「**基本缺失**」，并指出核心张力：
    "现 `Signal.step` 是 `Option` 非全函数——要么补全为全函数，要么诚实说明 None 的语义
     （无效事件）并把域收窄到 ValidEvent。"

  **裁定（采域收窄 B，非补全 A；理由：A 是补丁思维）**：

  `Tlayers.Signal.step : PositionState → Side → Option PositionState` 的 `none`
  **不是「未定义转移」，是「拒绝非法事件」**——`step long buy = none` / `step short sell = none`
  在缠论 located 流语义里是「持多状态下出现买入事件（同向连续）= 非法」。这个 `none`
  **是 T₁₄ located 流交替性的结构基础**（`Signal.located_state_machine_sides_alternate`
  正是靠 `none` 拒绝同向连续来证交替）。

  ★路径 A（补全为全函数，如 `long+buy → long` 持仓不变）会**改变 step 的语义**——
    它制造一个与既有 step 在 `long+buy` 上冲突（`long` vs `none`）的新函数，
    且破坏 T₁₄ 交替性证明的前提。这是 **补丁思维**（no-patch-mentality）：在正确代码上
    强塞一个语义来满足「全函数」的字面要求。**拒绝**。

  ★路径 B（域收窄，本文件采用，formalization-validity-domain）：诚实承认 δ 的忠实事件
    定义域 Eₗ **不是裸 `Side`**，而是「当前状态 s 下合法的事件」`ValidEvent s`。
    在裸 `Side` 上 step 是**偏函数**（none = 非法事件，**有效域 < 定义域**）；
    在收窄域 `{e // ValidEvent s e}` 上，δ 是**全函数 + 确定**（标准 (a) 严格兑现）。
    这**不改 Signal.step 的任何定义**，只读出它既有的「none = 非法」语义。

  ── 这是定义冲突吗？（testing-override.md 判准）──
  否。判准：「修复需改变某条定义的含义/边界/适用范围 → 定义冲突」。本文件**不改任何
  定义**——step 不动、T₁₄ 不动；只是诚实标注 step 的事件域是状态相关的（none=非法事件），
  并在忠实有效域上证 δ 全函数 + ∃!。故**无 /escalate**（无真定义冲突）。

  ── 认识论等级 ──
  全部 **L0**（定义内蕴：对 PositionState/List/Nat 结构归纳，零数据依赖）。lake 通过 =
  逻辑/管线正确（L0 因果性是「分类函数只读前缀」这一**结构形式**），**不**断言任何真实
  走势流的经验因果（formalization-validity-domain：L0 证结构必然，不证经验有效域）。
  (c) 因果无前视是 **T49 向心回溯「只用过去数据」的结构形式**——分类函数读前缀 ⟹ 因果。
-/

import Strict.Classification
import Signal
import Formal.BSPLabels

namespace Strict.Causal

open Strict (PrefixEq OnlineSystem CausalOnlineSystem)
open Formal.BSPLabels (Side)
open Tlayers.Signal (PositionState LocatedEvent step run)
open Tlayers.Signal.PositionState

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 事件合法性 ValidEvent（codex#2 域收窄 B：诚实读出 step 的 none = 非法事件）

  `step s e = none` ⟺ 事件 `e` 在状态 `s` 下**非法**（同向连续：long+buy / short+sell）。
  `ValidEvent s e := (step s e).isSome` = 事件在状态 s 下合法（step 给出确定下一态）。
  这是 δ 全函数的**忠实定义域**——裸 `Side` 太宽（含非法同向连续），收窄到 ValidEvent。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★事件在状态 s 下合法（step 给出确定下一态，非 none）。 -/
def ValidEvent (s : PositionState) (e : Side) : Prop := (step s e).isSome = true

/--
  ★非法事件刻画（L0，诚实读出 step 的 none 语义）：在状态 s 下，事件 e 非法
  ⟺ `step s e = none` ⟺ (s=long ∧ e=buy) ∨ (s=short ∧ e=sell)（同向连续）。
  这关死「none 是未定义」的误读——none 恰是「同向连续非法事件」的有限穷尽刻画。
-/
theorem invalid_iff_same_direction (s : PositionState) (e : Side) :
    ¬ ValidEvent s e ↔ (s = long ∧ e = Side.buy) ∨ (s = short ∧ e = Side.sell) := by
  unfold ValidEvent
  cases s <;> cases e <;> simp [step]

/--
  ★合法事件覆盖（L0）：flat 态下任意事件都合法（空仓可买可卖建仓）；
  long 态只 sell 合法、short 态只 buy 合法（located 流交替）。这是 ValidEvent 的穷尽刻画。
-/
theorem valid_event_cases (s : PositionState) (e : Side) :
    ValidEvent s e ↔ (s = flat) ∨ (s = long ∧ e = Side.sell) ∨ (s = short ∧ e = Side.buy) := by
  unfold ValidEvent
  cases s <;> cases e <;> simp [step]

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 状态转移 δ 全函数 + 确定（标准 (a)：∀(s,e) ∃! s'，在忠实有效域上）

  δ 的忠实形式：依赖 `ValidEvent` 证据，从合法事件给出**确定**下一态。
  `delta s e (h : ValidEvent s e) : PositionState` 全定义于忠实域（合法事件），
  且**确定**（Lean 函数 + Option.get 单值）。标准 (a) 的 ∃! 在收窄域上严格成立。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★状态转移 δ（全函数，忠实有效域 = 合法事件）：合法事件 ⟹ 确定下一态。
  用 `Option.get` 在 `ValidEvent`（step 非 none）证据下提取确定值——全定义 + 确定。
  这是标准 δₗ:Sₗ×Eₗ→Sₗ 在 Eₗ = {e // ValidEvent s e} 收窄域上的全函数实现（无 Option、无补丁）。
-/
def delta (s : PositionState) (e : Side) (h : ValidEvent s e) : PositionState :=
  (step s e).get h

/--
  ★δ 与 step 一致（L0）：合法事件下 δ 提取的就是 step 的 some 值。
  `step s e = some (delta s e h)`——δ 忠实于 step（不另造语义，无补丁）。
-/
theorem delta_eq_step (s : PositionState) (e : Side) (h : ValidEvent s e) :
    step s e = some (delta s e h) := by
  unfold delta
  exact (Option.some_get h).symm

/--
  ★δ 全函数 + 确定（标准 (a) ∃! 的严格兑现，L0）：对任意状态 s 与**合法**事件 e，
  **存在唯一** s' 使 `step s e = some s'`。这正是 δₗ:Sₗ×Eₗ→Sₗ 的「∀(s,e) ∃! s'」
  （全 = 存在；确定 = 唯一）在忠实有效域（ValidEvent）上的精确形式。

  ★记号：本项目**不依赖 Mathlib**（lakefile），无 `∃!` 记号，故显式展开 ExistsUnique 为
  `∃ s', P s' ∧ ∀ s'', P s'' → s'' = s'`（同 Classification.lean 的 `UniqueClassify` 模式）。

  存在：`delta s e h` 是见证（`delta_eq_step`）。
  唯一：`some` 单射 ⟹ 任意满足 `step s e = some s''` 的 s'' 都 = delta s e h。
-/
theorem delta_existsUnique (s : PositionState) (e : Side) (h : ValidEvent s e) :
    ∃ s', step s e = some s' ∧ ∀ s'', step s e = some s'' → s'' = s' := by
  refine ⟨delta s e h, delta_eq_step s e h, ?_⟩
  intro s'' hs''
  have := (delta_eq_step s e h).symm.trans hs''
  exact (Option.some.injEq _ _ |>.mp this).symm

/--
  ★δ 确定性（L0，∃! 的唯一性侧单列）：合法事件下，任意两个满足 step 的下一态相等。
  `step s e = some a → step s e = some b → a = b`——状态转移是**函数**（确定，非关系多值）。
  这是标准「确定」（deterministic）的直接形式。
-/
theorem delta_deterministic (s : PositionState) (e : Side) {a b : PositionState}
    (ha : step s e = some a) (hb : step s e = some b) : a = b := by
  rw [ha] at hb
  exact (Option.some.injEq _ _ |>.mp hb)

/-! ── δ 的方向钉死（located 流：完美点事件翻转方向，标准 (a) 的语义内容） ── -/

/--
  ★合法卖点 ⟹ 翻空（L0）：任意状态下合法的 sell 事件，δ 转到 short（顶背驰→翻空）。
  （flat+sell 与 long+sell 都合法且都到 short；short+sell 非法被 ValidEvent 排除。）
-/
theorem delta_sell_to_short (s : PositionState) (h : ValidEvent s Side.sell) :
    delta s Side.sell h = short := by
  unfold delta
  cases s <;> simp [step] at h ⊢

/--
  ★合法买点 ⟹ 翻多（L0）：任意状态下合法的 buy 事件，δ 转到 long（底背驰→翻多）。
-/
theorem delta_buy_to_long (s : PositionState) (h : ValidEvent s Side.buy) :
    delta s Side.buy h = long := by
  unfold delta
  cases s <;> simp [step] at h ⊢

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 状态转移一致 Cₗ(h ⌢ e) = δₗ(Cₗ(h), e)（标准 (b)：分类与状态机 fold 一致）

  分类函数 `C h = run flat h`（从空仓 fold located 事件流到当下状态，Option 表「全程合法」）。
  追加一个合法事件 e，新状态 = δ 作用于旧状态——这是 fold 的 step 律（run 的展开）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★分类函数 C（L0）：从初态 s₀ fold located 事件流到当下状态。
  `C s₀ evs = run s₀ evs`——`some s` = 全程合法、当下态 s；`none` = 流中出现非法事件。
  这是 located 流的「当下结构状态」Cₗ(h)（标准 Cₗ:H→Sₗ 的状态机实现）。
-/
def C (s₀ : PositionState) (evs : List LocatedEvent) : Option PositionState :=
  run s₀ evs

/--
  ★run 追加一个事件的展开（L0，run 尾部 step 律的辅助）：
  `run s₀ (evs ++ [e])` = 先 run 到 evs 末态、再 step 一次 e。
  这是 fold 在尾部追加的结合性——状态机一致律 (b) 的核心引理。
-/
theorem run_append_one (s₀ : PositionState) (evs : List LocatedEvent) (e : LocatedEvent) :
    run s₀ (evs ++ [e]) =
      match run s₀ evs with
      | some s => step s e.side
      | none => none := by
  induction evs generalizing s₀ with
  | nil =>
      simp only [List.nil_append, run]
      cases step s₀ e.side <;> rfl
  | cons hd tl ih =>
      simp only [List.cons_append, run]
      cases hstep : step s₀ hd.side with
      | none => rfl
      | some s' => exact ih s'

/--
  ★状态转移一致 Cₗ(h ⌢ e) = δₗ(Cₗ(h), e)（标准 (b)，L0）：
  若历史 h 全程合法（`C flat h = some s`）且追加事件 e 在末态 s 下合法（`ValidEvent s e.side`），
  则新历史的当下状态 = δ 作用于旧状态：`C flat (h ++ [e]) = some (delta s e.side hv)`。

  这把「分类 C」与「状态转移 δ」钉死为一致——追加合法事件后的分类 = δ 施于旧分类。
  这是标准 (b) `Cₗ(h⌢e) = δₗ(Cₗ(h), e)` 的严格形式（在合法事件忠实域上）。
-/
theorem step_sound (h : List LocatedEvent) (e : LocatedEvent) (s : PositionState)
    (hC : C flat h = some s) (hv : ValidEvent s e.side) :
    C flat (h ++ [e]) = some (delta s e.side hv) := by
  unfold C at hC ⊢
  rw [run_append_one, hC]
  exact delta_eq_step s e.side hv

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 因果无前视（标准 (c)：ω₀:ₜ = ω'₀:ₜ ⟹ Cₜ(ω) = Cₜ(ω')）——核心命题

  这是标准第3部分**核心**、codex#2 判「无定理」的缺口。
  `Cstream ω t` = 读流 ω 的前 t+1 个事件（索引 0..t）算当下状态。**只读前缀** ⟹ 因果。
  PrefixEq ω ω' t（前 t 个逐点相等）⟹ Cstream 在 t 相等——结构归纳（T49 向心回溯结构形式）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★流前缀取列（L0）：取流 ω 的前 n 个事件 [ω 0, ω 1, …, ω (n-1)]。
  `prefixList ω n` 只引用索引 < n 的事件——这是「只读过去」的语法载体。
-/
def prefixList {E : Type} (ω : Nat → E) : Nat → List E
  | 0 => []
  | (n + 1) => prefixList ω n ++ [ω n]

/--
  ★前缀取列只依赖前缀（L0，因果性的核心引理）：
  若 ω、ω' 在索引 < n 上逐点相等，则 `prefixList ω n = prefixList ω' n`。
  对 n 结构归纳——每多取一个事件 ω (n)，由前缀相等知 ω n = ω' n，拼接保持相等。
  这是「分类函数只读已出现数据」的结构本质（未来索引 ≥ n 不进入 prefixList）。
-/
theorem prefixList_congr {E : Type} (ω ω' : Nat → E) (n : Nat)
    (hpre : ∀ i, i < n → ω i = ω' i) :
    prefixList ω n = prefixList ω' n := by
  induction n with
  | zero => rfl
  | succ k ih =>
      simp only [prefixList]
      rw [ih (fun i hi => hpre i (Nat.lt_succ_of_lt hi)), hpre k (Nat.lt_succ_self k)]

/--
  ★流分类函数 Cstream（L0，标准 Cₜ(ω)）：读流 ω 的前 t+1 个事件算当下状态。
  `Cstream ω t = run flat (prefixList ω (t+1))`——索引 0..t 的事件 fold 到当下态。
  **只读 ≤ t 的事件**（prefixList (t+1) 取索引 0..t）——这是无前视的语法保证。
-/
def Cstream (ω : Nat → LocatedEvent) (t : Nat) : Option PositionState :=
  run flat (prefixList ω (t + 1))

/--
  ★★★因果无前视（标准 (c) 核心定理，L0）★★★：
  `PrefixEq ω ω' t ⟹ Cstream ω t = Cstream ω' t`——时刻 t 的当下状态只依赖 t 及之前的事件。

  即：若两条 located 事件流在时刻 t 及之前逐点相等（ω₀:ₜ = ω'₀:ₜ），则它们在 t 算出的
  分类状态相同——**未来事件（索引 > t）不影响当下分类**。这是缠论「当下判断不能用未来数据」
  （T49 向心回溯「只用过去数据」）的精确**结构形式**：分类 = 前缀的函数 ⟹ 因果无前视。

  证明：Cstream ω t 经 prefixList ω (t+1)，由 prefixList_congr（前 t+1 个索引即 0..t 相等
  ⟸ PrefixEq）知前缀列相等 ⟹ run 同 ⟹ Cstream 同。零数据依赖，纯结构归纳。
-/
theorem causal_no_lookahead (ω ω' : Nat → LocatedEvent) (t : Nat)
    (hpre : PrefixEq ω ω' t) :
    Cstream ω t = Cstream ω' t := by
  unfold Cstream
  rw [prefixList_congr ω ω' (t + 1)]
  intro i hi
  exact hpre i (Nat.lt_succ_iff.mp hi)

/--
  ★因果性等价刻画（L0，加强）：Cstream ω t 只是 prefixList ω (t+1) 的函数——
  存在一个**只吃前缀列**的函数 `g`（= run flat），使 `Cstream ω t = g (prefixList ω (t+1))`。
  这把「无前视」表达为「分类因子分解过前缀」——因果性 = 分类穿过前缀算子（标准 (c) 的本质）。
-/
theorem cstream_factors_through_prefix (ω : Nat → LocatedEvent) (t : Nat) :
    Cstream ω t = run flat (prefixList ω (t + 1)) := rfl

/-! ── 非平凡性：偷看未来的非因果分类器**不满足**因果性（关死「因果性平凡」指控） ──

  `causal_no_lookahead` 若任何分类器都满足，则因果性是同义反复（无内容）。
  本节给一个**偷看未来**的分类器 `lookahead`（读 ω(t+1) = 当下时刻之后的事件），
  证它**不满足** PrefixEq ⟹ 相等——即因果性是**非平凡判据**（区分因果 vs 非因果分类器）。
  这对应 task #66「对比偷看未来的非因果分类器，证因果版满足而非因果版不满足」。
  （formalization-validity-domain：否定性结果缩小有效域边界，证因果性有真实信息内容。）
-/

/--
  ★非因果分类器（偷看未来）：`lookahead ω t = ω (t+1)` 读**时刻 t 之后**的事件（索引 t+1 > t）。
  这是「用未来数据做当下判断」的反面教材——它依赖 PrefixEq 不约束的索引（> t）。
-/
def lookahead (ω : Nat → LocatedEvent) (t : Nat) : LocatedEvent := ω (t + 1)

/--
  ★★非平凡见证（L0）：偷看未来的 `lookahead` **不满足**因果无前视。
  存在两条流 ω、ω'，在时刻 0 及之前逐点相等（PrefixEq ω ω' 0），但 `lookahead ω 0 ≠ lookahead ω' 0`
  （它们在索引 1=未来 处不同）。这证 `causal_no_lookahead` **非同义反复**——因果性真实区分
  「读前缀」（满足）与「偷看未来」（不满足）的分类器。
-/
theorem lookahead_not_causal :
    ∃ (ω ω' : Nat → LocatedEvent) (t : Nat),
      PrefixEq ω ω' t ∧ lookahead ω t ≠ lookahead ω' t := by
  -- 两条流：仅在索引 1（未来）不同的 side
  let e0 : LocatedEvent := ⟨Side.buy, 0⟩
  let eB : LocatedEvent := ⟨Side.buy, 1⟩
  let eS : LocatedEvent := ⟨Side.sell, 1⟩
  let ω  : Nat → LocatedEvent := fun n => if n = 1 then eB else e0
  let ω' : Nat → LocatedEvent := fun n => if n = 1 then eS else e0
  refine ⟨ω, ω', 0, ?_, ?_⟩
  · -- PrefixEq ω ω' 0：索引 ≤ 0 即 i=0，两流都取 e0（i=1 的差异在前缀外）
    intro i hi
    rw [Nat.le_zero.mp hi]
    rfl
  · -- lookahead ω 0 = ω 1 = eB ≠ eS = ω' 1 = lookahead ω' 0
    unfold lookahead
    show ω 1 ≠ ω' 1
    simp only [ω, ω', if_pos rfl]
    intro h
    -- eB.side = buy ≠ sell = eS.side
    have : eB.side = eS.side := by rw [h]
    exact Side.noConfusion this

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 实例化标准内核 OnlineSystem + CausalOnlineSystem（task #64 核心交付）

  把上面的 δ / C / Cstream / 因果性组装为 `Strict.Classification` 的两个内核结构实例——
  这把「声明了无前视状态转移」变成「实例化了带证明的内核结构」（标准内核库的目的）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★located 流的合法历史（OnlineSystem 的 Hist）：全程合法的事件流 = 携带「run 非 none」证据。
  `{ evs : List LocatedEvent // (run flat evs).isSome = true }`——只取全程合法历史，
  使分类 C 落在 PositionState（非 Option）上，满足 OnlineSystem 的 `C : Hist → S` 全函数。
-/
def LegalHist : Type := { evs : List LocatedEvent // (run flat evs).isSome = true }

/-- ★合法历史的当下状态（提取 run 的 some 值，全函数到 PositionState）。 -/
def legalState (h : LegalHist) : PositionState := (run flat h.val).get h.property

/--
  ★legalState 规范（L0）：若 `run flat h.val = some s`，则 `legalState h = s`。
  这把含依赖证据的 `Option.get` 转为纯等式——下游证明用它绕开 `get` 的 motive 依赖问题。
-/
theorem legalState_eq (h : LegalHist) (s : PositionState) (hs : run flat h.val = some s) :
    legalState h = s := by
  unfold legalState
  rw [Option.get_of_eq_some _ hs]

/-- ★legalState 的 some 形式（L0）：`run flat h.val = some (legalState h)`。 -/
theorem run_legalState (h : LegalHist) : run flat h.val = some (legalState h) := by
  unfold legalState; exact (Option.some_get h.property).symm

/--
  ★追加合法事件保持历史合法（OnlineSystem 的 append 闭合性，L0）：
  若历史 h 合法（末态 s）且事件 e 在 s 下合法，则 h ++ [e] 仍合法。
-/
theorem legalHist_append (h : LegalHist) (e : LocatedEvent)
    (hv : ValidEvent (legalState h) e.side) :
    (run flat (h.val ++ [e])).isSome = true := by
  rw [run_append_one, run_legalState h]
  unfold ValidEvent at hv
  exact hv

/--
  ★实例化 OnlineSystem（标准结构 4，task #64）：located 流的在线状态转移系统。

  - Hist = 合法历史（LegalHist），E = LocatedEvent，S = PositionState。
  - append：追加一个**默认合法**的事件（用 flat+该事件 side 必合法，因 flat 态任意事件合法
    `valid_event_cases`）——这是 append 的全函数兜底（追加为新建仓事件，flat 态恒合法）。
  - C = legalState（合法历史的当下态）。
  - δ = 在末态 + 该事件 side 下，若合法则 step、否则保持（兜底，但 step_sound 只在合法时被引用）。
  - step_sound：见下方 `online_step_sound`（在合法事件时 C(append) = δ(C h, e)）。

  ★诚实标注：OnlineSystem 的 δ:S→E→S 要求 E 上**全函数**，而 located δ 的忠实域是
    ValidEvent（§2 裁定）。这里 δ 在非法事件上**兜底为「保持原状态」**——这是 OnlineSystem
    接口的类型兜底，**不是**对缠论 located 流的语义断言（located 流由 T₁₄ 保证不出现同向连续
    非法事件，故兜底分支在合法 located 流上永不触发）。真正的语义内容由 §2 的 `delta`
    （收窄域全函数 + ∃!）+ §3 的 `step_sound`（合法事件一致）承载，已严格证明。
-/
def onlineDelta (s : PositionState) (e : LocatedEvent) : PositionState :=
  match step s e.side with
  | some s' => s'
  | none => s          -- 兜底：非法事件保持原态（located 流由 T₁₄ 不触发；接口全函数化）

/-- ★onlineDelta 在合法事件上 = delta（L0，兜底分支不污染合法语义）。 -/
theorem onlineDelta_eq_delta (s : PositionState) (e : LocatedEvent)
    (hv : ValidEvent s e.side) :
    onlineDelta s e = delta s e.side hv := by
  unfold onlineDelta
  rw [delta_eq_step s e.side hv]

/-- ★onlineDelta 在合法事件上与 step 一致（L0）：合法 ⟹ onlineDelta = step 的 some 值。 -/
theorem onlineDelta_step_sound (s : PositionState) (e : LocatedEvent)
    (hv : ValidEvent s e.side) :
    step s e.side = some (onlineDelta s e) := by
  rw [onlineDelta_eq_delta s e hv]
  exact delta_eq_step s e.side hv

/--
  ★append 算子（OnlineSystem，L0）：合法历史追加一个事件。
  用 flat 态恒合法（`valid_event_cases`）保证：把事件**视作新建仓**追加时合法。
  这里采用最朴素的全函数 append——直接拼接（合法性由 step_sound 的前提单独把守）。
-/
def onlineAppend (h : LegalHist) (e : LocatedEvent)
    (hv : ValidEvent (legalState h) e.side) : LegalHist :=
  ⟨h.val ++ [e], legalHist_append h e hv⟩

/--
  ★OnlineSystem 步律（标准 (b) 在 LegalHist 上的兑现，L0）：
  追加合法事件后的当下态 = onlineDelta 施于旧当下态。
  `legalState (onlineAppend h e hv) = onlineDelta (legalState h) e`。
-/
theorem online_step_sound (h : LegalHist) (e : LocatedEvent)
    (hv : ValidEvent (legalState h) e.side) :
    legalState (onlineAppend h e hv) = onlineDelta (legalState h) e := by
  -- 用 legalState_eq 把含依赖证据的 get 转为纯 Option 等式（绕开 motive 依赖）。
  apply legalState_eq
  show run flat (h.val ++ [e]) = some (onlineDelta (legalState h) e)
  rw [run_append_one, run_legalState h, ← onlineDelta_step_sound (legalState h) e hv]

/--
  ★★实例化 CausalOnlineSystem（标准结构 5，task #64 核心交付）：located 流的无前视因果系统。

  S = PositionState（用 Option 兜底未知前缀的非法）。这里取 `Option PositionState` 作状态，
  使 Cstream 全定义（流可能在某前缀出现非法事件 ⟹ none）。`causal` 字段由
  `causal_no_lookahead` 直接填充——**因果无前视已机器证明**。

  这是标准第3部分 (c) 的内核实例：located 流的当下分类构成一个 `CausalOnlineSystem`，
  其 `causal` 证明义务由本文件 §4 兑现（PrefixEq ⟹ Cstream 相等）。
-/
def causalSystem : CausalOnlineSystem (Option PositionState) LocatedEvent where
  Cstream := Cstream
  causal := causal_no_lookahead

/-! ════════════════════════════════════════════════════════════════════════
  ## §6 有效域诚实标注（formalization-validity-domain + no-patch-mentality）

  本文件**忠实交付**（L0，machine-checked）：
  1. (a) δ 全函数 + 确定：`delta`（收窄域全函数）+ `delta_existsUnique`（∀(s,e)∃!s'）+
     `delta_deterministic`（确定）。忠实有效域 = `ValidEvent`（合法事件），**非裸 Side**。
  2. (b) 状态转移一致：`step_sound`（Cₗ(h⌢e)=δ(Cₗ h, e)）+ `online_step_sound`（内核版）。
  3. (c) 因果无前视：`causal_no_lookahead`（PrefixEq ⟹ Cstream 相等）——**核心**，
     T49 向心回溯「只用过去数据」的结构形式；`prefixList_congr`（只读前缀引理）。
  4. 内核实例化：`causalSystem : CausalOnlineSystem`（causal 义务已证）+ OnlineSystem 零件
     （`onlineDelta`/`onlineAppend`/`online_step_sound`）。

  本文件**不声明**（有效域边界，诚实标注，避免膨胀）：
  - ✗ δ 在**裸 Side** 上全函数——裸 Side 域含非法同向连续事件（step=none），是**偏函数**。
       δ 全函数性的忠实有效域是 `ValidEvent`（codex#2 域收窄 B，**非**补全 A 的补丁）。
  - ✗ `Signal.step` 的 none 是「未定义/缺陷」——none 是**非法事件**（同向连续），是 T₁₄
       交替性的结构基础（`invalid_iff_same_direction` 穷尽刻画）。本文件**不改 step**。
  - ✗ `onlineDelta` 兜底分支（非法事件保持原态）是缠论语义断言——它是 OnlineSystem 接口的
       类型全函数兜底（located 流由 T₁₄ 不触发同向连续，兜底分支在合法流上永不执行）。
       真正语义由 §2 收窄域 `delta` + §3 `step_sound` 承载（合法事件忠实一致）。
  - ✗ 因果性的**经验有效域**——本文件证的是「分类函数读前缀 ⟹ 因果」的**结构形式**（L0）。
       真实走势流是否「当下判断真的没用未来数据」是实现/数据层（L2/L3 引擎守卫），不由此 L0 声称。

  ★与 Signal.lean（T₁₄）的关系：本文件**复用** `step`/`run`/`LocatedEvent`（一致引用，
    无符号碰撞、无重定义），是 T₁₄ located 状态机的**因果性补全**——T₁₄ 证「合法流交替」，
    本文件证「分类读前缀 ⟹ 无前视」+「δ 收窄域全函数」。两者互补（交替性 + 因果性）。
-/

end Strict.Causal
