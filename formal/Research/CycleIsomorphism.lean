/-
  Research/CycleIsomorphism.lean — issue #144「周期同构猜想」的条件化证明与反例

  结论（L0，模型论/操作语义层）：
  1. 已确认的子级完整周期本身不推出父级端点投影；若另给 `ContextProjectionLaw`，则
     两端投影存在当且仅当两端父级中枢语境均可见（离开/突破，而非中枢内）。
  2. 两端有父级投影仍不足以推出一次父级短差；只有再给 ShortDiffEntry/Exit 在这两个
     投影点的精确触发，才得到操作语义上的条件同构见证。
  3. `internalCycle n` 给出一族父级中枢内部完整周期。它们无父级投影；在触发器满足
     「触发必有投影」的 soundness 条件下，父级归类不可能是短差。若同时无其他父级通道，
     才能进一步归入 C0/Hold。

  复用边界：
  - 声部：直接复用 `Origin.VoiceTree.VoiceTree`，不重建声部概念；
  - 证书：端点直接使用 canonical 薄证书 `Origin.Bsp`，确认性由参数化谓词携带；
  - 短差角色：直接复用当前去根化权威 `Origin.OperationRole18.RoleV.shortDiff`；
  - 中枢：显式反例直接使用 `Origin.Center`。

  认识论边界：PDF/ADR 只给方向/角色与显式触发器语义，没有给跨级投影定理。
  因而 `ContextProjectionLaw` 与精确触发条件均作为显式前提，不冒充已由教义推出。
  本文件无 sorry/admit/axiom。
-/

import Origin.ChanlunElements
import Origin.VoiceTree
import Origin.OperationRole18

namespace NewChanlun.Research.CycleIsomorphism

open NewChanlun.Origin

abbrev CanonicalVoiceTree := NewChanlun.Origin.VoiceTree.VoiceTree

/-! ## 1. 完整周期：复用 VoiceTree 与 Bsp -/

/--
  `CompleteCycle T Confirmed`：某个 long 声部上的买点开仓→卖点平仓完整周期。
  `buy`/`sell` 直接是 canonical `Bsp`；`Confirmed` 是外置确认谓词，避免复制证书结构。
-/
structure CompleteCycle {V : Type} (T : CanonicalVoiceTree V)
    (Confirmed : V → Bsp → Prop) where
  voice : V
  buy : Bsp
  sell : Bsp
  voice_is_long : T.side voice = Side.long
  buy_is_buy : buy.side = Side.long
  sell_is_sell : sell.side = Side.short
  buy_before_sell : buy.index < sell.index
  buy_confirmed : Confirmed voice buy
  sell_confirmed : Confirmed voice sell

/-! ## 2. 父级投影与中枢语境 -/

/-- 子级薄证书与父级薄证书之间的投影关系。关系而非函数，避免擅自假定投影存在/唯一。 -/
abbrev Projection := Bsp → Bsp → Prop

/-- 一个子级证书存在至少一个父级投影。 -/
def HasParentProjection (Project : Projection) (child : Bsp) : Prop :=
  ∃ parent : Bsp, Project child parent

/-- issue #144 所需的最小父级中枢语境三分。 -/
inductive ParentCenterContext where
  | insideCenter
  | leavingCenter
  | breakout
deriving DecidableEq, Repr

/-- 离开/突破语境对父级可见；中枢内震荡不可见。这里只定义候选门，不声称它来自 PDF 定理。 -/
def ParentCenterContext.Projectable : ParentCenterContext → Prop
  | .insideCenter => False
  | .leavingCenter => True
  | .breakout => True

/--
  跨级投影规律的显式接口。该规律不是现有 PDF/Lean 库的结论，必须作为外加前提：
  子证书有父级投影 ↔ 它在父级语境中可见。
-/
def ContextProjectionLaw (Project : Projection)
    (context : Bsp → ParentCenterContext) : Prop :=
  ∀ child, HasParentProjection Project child ↔ (context child).Projectable

/-- 一个完整周期的两个端点都存在父级投影。 -/
def EndpointsProject {V : Type} {T : CanonicalVoiceTree V}
    {Confirmed : V → Bsp → Prop} (Project : Projection)
    (W : CompleteCycle T Confirmed) : Prop :=
  HasParentProjection Project W.buy ∧ HasParentProjection Project W.sell

/-- 一个完整周期的两个端点在父级中枢语境中都可见。 -/
def EndpointsContextVisible {V : Type} {T : CanonicalVoiceTree V}
    {Confirmed : V → Bsp → Prop} (context : Bsp → ParentCenterContext)
    (W : CompleteCycle T Confirmed) : Prop :=
  (context W.buy).Projectable ∧ (context W.sell).Projectable

/--
  命题 1（条件成立）：在 `ContextProjectionLaw` 下，周期两端有父级投影，当且仅当
  两端均处于父级可见语境。这个定理把真正缺失的跨级规律完整暴露为前提。
-/
theorem cycle_endpoints_project_iff_context_visible
    {V : Type} {T : CanonicalVoiceTree V} {Confirmed : V → Bsp → Prop}
    (Project : Projection) (context : Bsp → ParentCenterContext)
    (W : CompleteCycle T Confirmed) (law : ContextProjectionLaw Project context) :
    EndpointsProject Project W ↔ EndpointsContextVisible context W := by
  constructor
  · intro h
    exact ⟨(law W.buy).mp h.1, (law W.sell).mp h.2⟩
  · intro h
    exact ⟨(law W.buy).mpr h.1, (law W.sell).mpr h.2⟩

/-! ## 3. 条件同构：投影 + 精确触发，而不是投影单独蕴含 -/

/-- 两个已选择的父级端点，以及它们分别来自 W 两端的投影见证。 -/
structure ProjectedEndpoints {V : Type} {T : CanonicalVoiceTree V}
    {Confirmed : V → Bsp → Prop} (Project : Projection)
    (W : CompleteCycle T Confirmed) where
  parentBuy : Bsp
  parentSell : Bsp
  buy_projected : Project W.buy parentBuy
  sell_projected : Project W.sell parentSell

/-- 二元触发关系：`Trigger child parent` 表示子证书恰在该父级投影点触发。 -/
abbrev Trigger := Bsp → Bsp → Prop

/-- 对固定子端点，触发器恰好只在指定父级投影点触发。 -/
def FiresExactlyAt (TriggerAt : Trigger) (child parent : Bsp) : Prop :=
  TriggerAt child parent ∧ ∀ p, TriggerAt child p → p = parent

/--
  一次父级短差运行。`role` 直接锚到当前权威 `RoleV.shortDiff`；入/出场点由显式
  ShortDiffEntry/ShortDiffExit 触发关系给出。
-/
structure ParentShortDiffRun {V : Type} {T : CanonicalVoiceTree V}
    {Confirmed : V → Bsp → Prop} (ShortDiffEntry ShortDiffExit : Trigger)
    (W : CompleteCycle T Confirmed) where
  role : RoleV
  role_is_shortDiff : role = RoleV.shortDiff
  entryPoint : Bsp
  exitPoint : Bsp
  entry_fires : ShortDiffEntry W.buy entryPoint
  exit_fires : ShortDiffExit W.sell exitPoint

/-- 父级账本把 W 归类成一次短差，意即存在上述完整短差运行见证。 -/
def IsParentShortDiff {V : Type} {T : CanonicalVoiceTree V}
    {Confirmed : V → Bsp → Prop} (ShortDiffEntry ShortDiffExit : Trigger)
    (W : CompleteCycle T Confirmed) : Prop :=
  Nonempty (ParentShortDiffRun ShortDiffEntry ShortDiffExit W)

/--
  周期同构见证：子 long 声部确为 short 父声部的直接子声部；两端有选定父级投影；
  父级角色为 shortDiff；Entry/Exit 分别且唯一地在两个投影点触发。
-/
structure CycleIsomorphismWitness {V : Type} {T : CanonicalVoiceTree V}
    {Confirmed : V → Bsp → Prop} (Project : Projection)
    (ShortDiffEntry ShortDiffExit : Trigger) (W : CompleteCycle T Confirmed) where
  parentVoice : V
  child_of_parent : T.parent W.voice = some parentVoice
  parent_is_short : T.side parentVoice = Side.short
  projected : ProjectedEndpoints Project W
  role_is_shortDiff :
    classifyV (some (T.side parentVoice)) (T.side W.voice) = RoleV.shortDiff
  entry_exact : FiresExactlyAt ShortDiffEntry W.buy projected.parentBuy
  exit_exact : FiresExactlyAt ShortDiffExit W.sell projected.parentSell

/-- 从同构见证忘掉投影/唯一性，可得父级短差运行。 -/
def CycleIsomorphismWitness.toParentShortDiffRun
    {V : Type} {T : CanonicalVoiceTree V} {Confirmed : V → Bsp → Prop}
    {Project : Projection} {ShortDiffEntry ShortDiffExit : Trigger}
    {W : CompleteCycle T Confirmed}
    (h : CycleIsomorphismWitness Project ShortDiffEntry ShortDiffExit W) :
    ParentShortDiffRun ShortDiffEntry ShortDiffExit W :=
  { role := classifyV (some (T.side h.parentVoice)) (T.side W.voice)
    role_is_shortDiff := h.role_is_shortDiff
    entryPoint := h.projected.parentBuy
    exitPoint := h.projected.parentSell
    entry_fires := h.entry_exact.1
    exit_fires := h.exit_exact.1 }

/-- VoiceTree 交替律的实例：long 子声部的直接父声部必为 short。 -/
theorem parent_short_of_long_child
    {V : Type} (T : CanonicalVoiceTree V) (child parent : V)
    (hparent : T.parent child = some parent) (hchild : T.side child = Side.long) :
    T.side parent = Side.short := by
  have halt := T.alternating child parent hparent
  rw [hchild] at halt
  cases hp : T.side parent with
  | long => simp [hp, NewChanlun.Origin.VoiceTree.flip] at halt
  | short => rfl

/--
  命题 2（条件成立）：若两端已有选定父级投影，且 ShortDiffEntry/Exit 各自恰在对应投影点
  唯一触发，则 W 与一次父级短差在操作语义上同构。
-/
theorem conditional_cycle_isomorphism
    {V : Type} {T : CanonicalVoiceTree V} {Confirmed : V → Bsp → Prop}
    {Project : Projection} {ShortDiffEntry ShortDiffExit : Trigger}
    (W : CompleteCycle T Confirmed) (parentVoice : V)
    (hparent : T.parent W.voice = some parentVoice)
    (P : ProjectedEndpoints Project W)
    (hentry : FiresExactlyAt ShortDiffEntry W.buy P.parentBuy)
    (hexit : FiresExactlyAt ShortDiffExit W.sell P.parentSell) :
    Nonempty (CycleIsomorphismWitness Project ShortDiffEntry ShortDiffExit W) := by
  have hparentSide :=
    parent_short_of_long_child T W.voice parentVoice hparent W.voice_is_long
  exact ⟨
    { parentVoice := parentVoice
      child_of_parent := hparent
      parent_is_short := hparentSide
      projected := P
      role_is_shortDiff := by
        rw [hparentSide, W.voice_is_long]
        rfl
      entry_exact := hentry
      exit_exact := hexit }
  ⟩

/-- 条件同构见证直接给出“一次父级短差”的存在性结论。 -/
theorem conditional_cycle_isomorphism_yields_parent_shortDiff
    {V : Type} {T : CanonicalVoiceTree V} {Confirmed : V → Bsp → Prop}
    {Project : Projection} {ShortDiffEntry ShortDiffExit : Trigger}
    (W : CompleteCycle T Confirmed) (parentVoice : V)
    (hparent : T.parent W.voice = some parentVoice)
    (P : ProjectedEndpoints Project W)
    (hentry : FiresExactlyAt ShortDiffEntry W.buy P.parentBuy)
    (hexit : FiresExactlyAt ShortDiffExit W.sell P.parentSell) :
    IsParentShortDiff ShortDiffEntry ShortDiffExit W := by
  rcases conditional_cycle_isomorphism W parentVoice hparent P hentry hexit with ⟨hiso⟩
  exact ⟨hiso.toParentShortDiffRun⟩

/-! ## 4. 显式父级中枢内部反例族 -/

/-- 两节点 canonical 声部树：long 子声部隶属 short 父声部，满足交替与良基。 -/
def twoVoiceTree : CanonicalVoiceTree Side where
  parent
    | Side.long => some Side.short
    | Side.short => none
  side := fun v => v
  closed := fun _ => false
  depth
    | Side.long => 1
    | Side.short => 0
  alternating := by
    intro v p h
    cases v <;> cases p <;>
      simp [NewChanlun.Origin.VoiceTree.flip] at h ⊢
  depth_decreasing := by
    intro v p h
    cases v <;> cases p <;> simp at h ⊢
  cascade_close := by
    intro _v _p _hparent hclosed
    simp at hclosed

/-- 反例族共同的父级中枢 `[0,100]`。 -/
def counterexampleParentCenter : Center :=
  { zd := 0, zg := 100, startIndex := 0, endIndex := 100000, valid := by decide }

/-- 第 n 个反例的已确认子级买点。价格 40 严格位于父级中枢内部。 -/
def internalBuy (n : Nat) : Bsp :=
  { kind := BspKind.type1, side := Side.long, index := 2 * n, price := 40 }

/-- 第 n 个反例的已确认子级卖点。价格 60 严格位于父级中枢内部。 -/
def internalSell (n : Nat) : Bsp :=
  { kind := BspKind.type1, side := Side.short, index := 2 * n + 1, price := 60 }

/-- 薄证书的价格位于给定 canonical 中枢内部。 -/
def InsideCenter (c : Center) (b : Bsp) : Prop :=
  c.zd < b.price ∧ b.price < c.zg

theorem internalBuy_inside_parent_center (n : Nat) :
    InsideCenter counterexampleParentCenter (internalBuy n) := by
  simp [InsideCenter, counterexampleParentCenter, internalBuy]

theorem internalSell_inside_parent_center (n : Nat) :
    InsideCenter counterexampleParentCenter (internalSell n) := by
  simp [InsideCenter, counterexampleParentCenter, internalSell]

/-- 反例族的确认谓词：只确认指定 long 声部的该对 canonical Bsp。 -/
def internalConfirmed (n : Nat) (v : Side) (b : Bsp) : Prop :=
  v = Side.long ∧ (b = internalBuy n ∨ b = internalSell n)

/-- 第 n 个父级中枢内部完整周期。 -/
def internalCycle (n : Nat) : CompleteCycle twoVoiceTree (internalConfirmed n) where
  voice := Side.long
  buy := internalBuy n
  sell := internalSell n
  voice_is_long := rfl
  buy_is_buy := rfl
  sell_is_sell := rfl
  buy_before_sell := by
    unfold internalBuy internalSell
    exact Nat.lt_succ_self (2 * n)
  buy_confirmed := ⟨rfl, Or.inl rfl⟩
  sell_confirmed := ⟨rfl, Or.inr rfl⟩

/-- 父级中枢内反例模型没有任何跨级证书投影。 -/
def noProjection : Projection := fun _child _parent => False

/-- 反例模型的所有端点语境均为父级中枢内部。 -/
def insideContext : Bsp → ParentCenterContext := fun _ => .insideCenter

/-- 空投影模型确实满足候选的语境投影规律：两侧都为假。 -/
theorem noProjection_satisfies_context_law :
    ContextProjectionLaw noProjection insideContext := by
  intro child
  constructor
  · rintro ⟨parent, h⟩
    exact h
  · intro h
    exact False.elim h

/-- 命题 1（证伪部分）：任意 n 的已确认完整周期都不具有父级端点投影。 -/
theorem confirmed_cycle_does_not_force_parent_projection (n : Nat) :
    ¬ EndpointsProject noProjection (internalCycle n) := by
  intro h
  rcases h.1 with ⟨parent, hp⟩
  exact hp

/--
  反例族打包：候选语境投影律成立、两个端点价格都严格位于父级中枢内部，但两端投影不存在。
-/
theorem internal_center_counterexample_family (n : Nat) :
    ContextProjectionLaw noProjection insideContext ∧
      InsideCenter counterexampleParentCenter (internalBuy n) ∧
      InsideCenter counterexampleParentCenter (internalSell n) ∧
      ¬ EndpointsProject noProjection (internalCycle n) := by
  exact ⟨noProjection_satisfies_context_law,
    internalBuy_inside_parent_center n,
    internalSell_inside_parent_center n,
    confirmed_cycle_does_not_force_parent_projection n⟩

/-- 投影全关系：用于隔离证明「即使端点有投影，也仍不自动触发短差」。 -/
def totalProjection : Projection := fun _child _parent => True

/-- 永不触发的 Entry/Exit 关系。 -/
def neverTrigger : Trigger := fun _child _parent => False

/-- 端点投影存在只是关系层事实；这里给出明确的两个投影见证。 -/
def totalProjectedEndpoints (n : Nat) :
    ProjectedEndpoints totalProjection (internalCycle n) where
  parentBuy := internalBuy n
  parentSell := internalSell n
  buy_projected := True.intro
  sell_projected := True.intro

/-- 是否存在完整的周期同构见证。 -/
def HasCycleIsomorphism {V : Type} {T : CanonicalVoiceTree V}
    {Confirmed : V → Bsp → Prop} (Project : Projection)
    (ShortDiffEntry ShortDiffExit : Trigger) (W : CompleteCycle T Confirmed) : Prop :=
  Nonempty (CycleIsomorphismWitness Project ShortDiffEntry ShortDiffExit W)

/--
  命题 2（证伪部分）：仅“两端均有父级投影”不能推出周期同构。
  反模型让投影处处成立、Entry/Exit 处处不触发。
-/
theorem projection_alone_does_not_force_cycle_isomorphism (n : Nat) :
    EndpointsProject totalProjection (internalCycle n) ∧
      ¬ HasCycleIsomorphism totalProjection neverTrigger neverTrigger (internalCycle n) := by
  constructor
  · exact ⟨⟨internalBuy n, True.intro⟩, ⟨internalSell n, True.intro⟩⟩
  · rintro ⟨hiso⟩
    exact hiso.entry_exact.1

/-! ## 5. 父级归类：无投影排除短差；C0 还需排除其他通道 -/

/-- ShortDiffEntry/Exit 的最小可靠性：任何触发都必须落在真实父级投影关系上。 -/
def TriggerProjectionSound (Project : Projection)
    (ShortDiffEntry ShortDiffExit : Trigger) : Prop :=
  (∀ child parent, ShortDiffEntry child parent → Project child parent) ∧
  (∀ child parent, ShortDiffExit child parent → Project child parent)

/-- 若 W 的买端点根本无父级投影，sound 的触发器不可能把 W 归成一次父级短差。 -/
theorem not_parent_shortDiff_of_no_buy_projection
    {V : Type} {T : CanonicalVoiceTree V} {Confirmed : V → Bsp → Prop}
    {Project : Projection} {ShortDiffEntry ShortDiffExit : Trigger}
    (W : CompleteCycle T Confirmed)
    (hno : ¬ HasParentProjection Project W.buy)
    (hsound : TriggerProjectionSound Project ShortDiffEntry ShortDiffExit) :
    ¬ IsParentShortDiff ShortDiffEntry ShortDiffExit W := by
  rintro ⟨run⟩
  apply hno
  exact ⟨run.entryPoint, hsound.1 W.buy run.entryPoint run.entry_fires⟩

/--
  命题 3：父级中枢内部反例族在任何 sound 触发器下都不可归为父级短差。
-/
theorem internal_cycle_not_parent_shortDiff
    (n : Nat) (ShortDiffEntry ShortDiffExit : Trigger)
    (hsound : TriggerProjectionSound noProjection ShortDiffEntry ShortDiffExit) :
    ¬ IsParentShortDiff ShortDiffEntry ShortDiffExit (internalCycle n) := by
  apply not_parent_shortDiff_of_no_buy_projection
    (Project := noProjection) (ShortDiffEntry := ShortDiffEntry)
    (ShortDiffExit := ShortDiffExit) (internalCycle n)
  · rintro ⟨parent, hp⟩
    exact hp
  · exact hsound

/--
  `ParentHoldC0` 明确编码裁定边界：C0/Hold 不仅要求“不是短差”，还要求不存在其他父级通道。
  无投影单独只能排除短差，不能排除独立的风险/本级证书/记录等通道。
-/
def ParentHoldC0 {V : Type} {T : CanonicalVoiceTree V}
    {Confirmed : V → Bsp → Prop} (ShortDiffEntry ShortDiffExit : Trigger)
    (W : CompleteCycle T Confirmed) (OtherParentChannel : Prop) : Prop :=
  ¬ IsParentShortDiff ShortDiffEntry ShortDiffExit W ∧ ¬ OtherParentChannel

/-- 若再给“无其他父级通道”，父级中枢内部周期才可严格归入 C0/Hold。 -/
theorem internal_cycle_parent_holdC0_if_no_other_channel
    (n : Nat) (ShortDiffEntry ShortDiffExit : Trigger)
    (OtherParentChannel : Prop)
    (hsound : TriggerProjectionSound noProjection ShortDiffEntry ShortDiffExit)
    (hother : ¬ OtherParentChannel) :
    ParentHoldC0 ShortDiffEntry ShortDiffExit (internalCycle n) OtherParentChannel := by
  exact ⟨internal_cycle_not_parent_shortDiff n ShortDiffEntry ShortDiffExit hsound, hother⟩

end NewChanlun.Research.CycleIsomorphism
