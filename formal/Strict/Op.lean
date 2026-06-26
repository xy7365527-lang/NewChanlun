/-
  C5 操作类型 → 标准第6部分「完全应对 πₗ:Sₗ→A」Strategy 实例化（严格分类蜂群 T-op）
  ★task #67 续（T-op 升级，编排者严格完全分类纲领，codex#2 诊断 + Strategy 骨架）

  topo_address: swarm/严格完全分类/t-op ｜ parent: Lead

  codex#2 诊断（gapmatrix 标准6行）：当前 `OperationalSemantics.pi` / `Claim6.decideOp` 满足的是
  **收窄定义域** `ResolvedOpSignal × PositionState → OpType`，**不是** `Sₗ → A` 全策略；
  缺显式 `wait`（只有 `hold`，flat 状态误返回 hold）；数量/合法 route 被标域外。

  本文件升级（标准第6部分完全应对）：
  - **建 Strategy 实例**：`Strict.Strategy StrictState`——`π : StrictState → StrictAction` 全定义
    （StrictAction 含 `wait`，codex#2 修正 hold/wait 区分：hold=持仓不动，wait=空仓观望）+ total。
  - **Layer1 保留**：操作动作标签穷尽（StrictAction 7 构造子，含 wait）。
  - **Layer2 诚实失败**：op_complete 失败（买1手 vs 买10手 同标签异语义，codex#1）⟹ 操作类型 =
    「按标签商分类」（标签层）；**数量维度 = L3/运行时**，不进 Layer2——明确标注有效域，不强塞
    （formalization-validity-domain）。复用 `Formal.OperationalSemantics.i_not_complete` 反例。

  依赖方向：Strict → Formal（单向，无环）。import Strict.Classification（t-kernel #57 内核）+
  Formal.OperationalSemantics（C5 Layer2：Exec/OpEqSemantic/i_not_complete/OpTag）。

  认识论：L0（定义内蕴）。数量/价格/合法 route = L3 运行时有效域，诚实标注不入本 L0 Layer2。
  禁 sorry/admit/axiom。验证：`cd formal && lake env lean Strict/Op.lean`。
-/

import Strict.Classification
import Formal.OperationalSemantics

namespace Strict.Op

open Strict (StrictAction Strategy)
open Formal.OperationalSemantics (OpTag Exec OpEqSemantic I i_not_complete)

/-! ## 一、完整结构状态 Sₗ（codex#2：从完整结构状态到动作，非收窄域）

  标准第6部分 `πₗ : Sₗ → A`——Sₗ 是完整结构状态（持仓 + 信号 + 是否已建仓）。
  codex#2 核心修正：**flat（空仓）状态应返回 `wait`（观望），不是 `hold`（持仓不动）**。
  故 StrictState 须区分"空仓"与"持仓"——`hold` 仅对持仓状态合法，`wait` 仅对空仓状态。
-/

/-- ★持仓方向（多/空/空仓）。空仓 = flat（wait 的合法域）。 -/
inductive Pos where
  | long | short | flat
deriving DecidableEq, Repr

/-- ★信号方向（买侧/卖侧/无信号）。 -/
inductive Sig where
  | buySide | sellSide | none
deriving DecidableEq, Repr

/--
  ★完整结构状态 StrictState（codex#2：Sₗ = 持仓 × 信号，π 的完整定义域）。
  这是"完整结构状态 Cₗ(hₜ)"的 L0 摘要——3 持仓 × 3 信号 = 9 状态，π 对全 9 格定义。
-/
structure StrictState where
  pos : Pos
  sig : Sig
deriving DecidableEq, Repr

/-! ## 二、完全应对策略 π（standard §6，Strategy 实例，含 wait） -/

/--
  ★完全应对策略 π（standard §6：StrictState → StrictAction 全函数，含 wait）。

  codex#2 修正：**flat（空仓）+ 无买信号 → `wait`（空仓观望）**，区别于持仓状态的 `hold`：
  - flat + buySide → buy（建仓）
  - flat + sellSide → wait（空仓遇卖侧，裸空非缠论 §4.4，观望不动 = wait 非 hold）
  - flat + none → wait（空仓无信号 = 等待，codex#2 关键修正：非 hold）
  - long + buySide → add（降成本买回）
  - long + sellSide → reduce（降成本减仓）
  - long + none → hold（持多不动 = hold，有仓位）
  - short + buySide → close（平空头腿）
  - short + sellSide → sell（空头翻转/止损）
  - short + none → hold（持空不动 = hold，有仓位）

  全函数：match 穷尽 3×3=9 格。hold 仅 long/short+none（持仓不动）；wait 仅 flat（空仓观望）。
-/
def piStrict (s : StrictState) : StrictAction :=
  match s.pos, s.sig with
  | Pos.flat,  Sig.buySide  => StrictAction.buy
  | Pos.flat,  Sig.sellSide => StrictAction.wait   -- 空仓遇卖侧 → 观望（裸空非缠论）
  | Pos.flat,  Sig.none     => StrictAction.wait   -- ★codex#2：空仓无信号 = wait（非 hold）
  | Pos.long,  Sig.buySide  => StrictAction.add
  | Pos.long,  Sig.sellSide => StrictAction.reduce
  | Pos.long,  Sig.none     => StrictAction.hold   -- 持多不动 = hold（有仓位）
  | Pos.short, Sig.buySide  => StrictAction.close
  | Pos.short, Sig.sellSide => StrictAction.sell
  | Pos.short, Sig.none     => StrictAction.hold   -- 持空不动 = hold（有仓位）

/--
  ★完全应对 Strategy 实例（standard §6 结构 8）：opStrategy : Strategy StrictState。
  π = piStrict 全定义（∀状态有应对）+ total 证明义务。这把"完全应对"语义钉为可下游引用的
  Strategy 对象——每个完整结构状态都有严格动作应对（含 wait/hold 区分）。
-/
def opStrategy : Strategy StrictState where
  π := piStrict
  total := fun s => ⟨piStrict s, rfl⟩

/-- ★standard §6 完全应对（L0）：opStrategy.π 全定义——∀状态有应对（无未定义状态）。 -/
theorem opStrategy_total (s : StrictState) : ∃ a : StrictAction, opStrategy.π s = a :=
  opStrategy.total s

/--
  ★Layer1 动作标签穷尽（L0，含 wait）：任意严格动作必属 7 构造子之一（buy/sell/add/reduce/
  hold/close/wait），无第 8 类。这是 `StrictAction` 的结构归纳——补全 codex#2 指出的 wait 缺口。
-/
theorem strictaction_exhaustive (a : StrictAction) :
    a = StrictAction.buy ∨ a = StrictAction.sell ∨ a = StrictAction.add
    ∨ a = StrictAction.reduce ∨ a = StrictAction.hold ∨ a = StrictAction.close
    ∨ a = StrictAction.wait := by
  cases a
  · exact Or.inl rfl
  · exact Or.inr (Or.inl rfl)
  · exact Or.inr (Or.inr (Or.inl rfl))
  · exact Or.inr (Or.inr (Or.inr (Or.inl rfl)))
  · exact Or.inr (Or.inr (Or.inr (Or.inr (Or.inl rfl))))
  · exact Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inl rfl)))))
  · exact Or.inr (Or.inr (Or.inr (Or.inr (Or.inr (Or.inr rfl)))))

/-- ★π 应对穷尽于 StrictAction（L0）：π 值域落在 7 严格动作内（完全应对类型穷尽）。 -/
theorem piStrict_in_action (s : StrictState) :
    piStrict s = StrictAction.buy ∨ piStrict s = StrictAction.sell
    ∨ piStrict s = StrictAction.add ∨ piStrict s = StrictAction.reduce
    ∨ piStrict s = StrictAction.hold ∨ piStrict s = StrictAction.close
    ∨ piStrict s = StrictAction.wait :=
  strictaction_exhaustive (piStrict s)

/-! ## 三、wait/hold 区分（codex#2 核心修正，L0） -/

/--
  ★wait 仅对空仓（L0，codex#2 修正）：π 返回 `wait` ⟹ 持仓为 flat（空仓）。
  这关死"持仓状态返回 wait"的误读——wait = 空仓观望，hold = 持仓不动，二者状态域互斥。
-/
theorem wait_only_when_flat (s : StrictState) (h : piStrict s = StrictAction.wait) :
    s.pos = Pos.flat := by
  unfold piStrict at h
  cases hp : s.pos <;> cases hs : s.sig <;> simp [hp, hs] at h ⊢

/--
  ★hold 仅对持仓（L0，codex#2 修正）：π 返回 `hold` ⟹ 持仓为 long 或 short（有仓位）。
  这坐实 hold ≠ wait：hold 仅持仓不动（long/short + 无信号），wait 仅空仓观望。
-/
theorem hold_only_when_holding (s : StrictState) (h : piStrict s = StrictAction.hold) :
    s.pos = Pos.long ∨ s.pos = Pos.short := by
  unfold piStrict at h
  cases hp : s.pos <;> cases hs : s.sig <;> simp [hp, hs] at h ⊢

/--
  ★wait 实例存在（L0，构造性见证）：空仓无信号状态 → wait（补全 codex#2 wait 缺口非空）。
-/
theorem wait_realizable : piStrict ⟨Pos.flat, Sig.none⟩ = StrictAction.wait := rfl

/-! ## 四、两层定理收口 + 数量有效域诚实裁定

  - **Layer1**（标签穷尽，本文件 `strictaction_exhaustive`）：动作类型 7 构造子穷尽（含 wait）。
  - **Layer2**（语义双射失败，诚实）：操作类型标签 **不决定** 完整操作语义（数量 M / 价格 /
    账户转移）——复用 `Formal.OperationalSemantics.i_not_complete`（买1手 vs 买10手 同 OpTag 异
    账户转移）。**数量维度 = L3/运行时，不进 Layer2**（formalization-validity-domain）。
-/

/--
  ★Layer2 数量有效域诚实裁定（L0，codex#1 + formalization-validity-domain）：操作类型（标签层）
  **不是** 完整操作语义双射——同标签 fiber 含账户转移不等价对象（数量不同）。复用 C5 反例
  `i_not_complete`。**裁定**：操作类型 = 标签商分类；**数量 M = L3 payoff/运行时维度，不进本
  L0 Layer2**（OpType/StrictAction 只给类型不给数量，603 §诚实分层）。不强塞数量进 L0（no-patch）。
-/
theorem op_quantity_out_of_layer2 :
    ∃ x y : Exec, I x = I y ∧ ¬ OpEqSemantic x y := i_not_complete

end Strict.Op
