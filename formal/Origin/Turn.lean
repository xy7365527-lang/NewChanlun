/-
Origin/Turn.lean — q 级走势转折 `Turn_q` 的正面定义（#877）

权威来源：
- `.chanlun/definitions/beichi.md:431-445`：`Turn_q` 定义为当前 q 级走势类型完成并让位于
  一个新的 q 级走势类型。
- `.chanlun/definitions/beichi.md:228-232`：「完成」由背驰定义；不是先由形状判出完成再喂给
  背驰，而是背驰判出来了就叫完成。
- `docs/chanlun/text/blog/017-第17课.md:22`：「缠中说禅技术分析基本原理一」——任何级别的
  任何走势类型终要完成。
- `docs/chanlun/text/blog/017-第17课.md:36`：「走势终完美」有两个不可分割的方面：任何走势
  在图形上最终都要完成；一种走势类型完成后，就会转化为其他类型的走势。

认识论等级：全部 L0。这里只把裁定后的定义与外部结构假设编码成命题，并证明其定义层推论；
不声称真实行情中的后继关系已经被构造或验证。禁 `sorry` / `admit` / `axiom`，不依赖 Mathlib。
-/

import Origin.Divergence

namespace NewChanlun.Origin

/-! ## 1. q 级走势类型实例的最小抽象载体 -/

/--
`T_q^d` 的最小抽象载体，只保留 `Turn_q` 需要的字段——用来判定完成的背驰段对；
笔、线段、中枢等内部结构本文件不需要，故不引入。
-/
structure TrendInstance where
  divPair : DivergencePair
deriving Repr

/-! ## 2. 「完成」由背驰定义 -/

/--
走势类型的「完成」由背驰定义，不是纯形态学判据。

`.chanlun/definitions/beichi.md:228-232` 裁定原文：「『完成』由背驰定义，归动力学……
裁定把箭头定死：不是先由形状判出完成再喂给背驰，而是背驰判出来了就叫完成。」
-/
def Completed (T : TrendInstance) : Prop := IsDivergence T.divPair

instance (T : TrendInstance) : Decidable (Completed T) := by
  unfold Completed
  exact inferInstanceAs (Decidable (IsDivergence T.divPair))

/-! ## 3. 基本原理一的「让位」半 -/

/--
`EventualHandover succ` 编码缠中说禅技术分析基本原理一的「让位」半：第 17 课第 22 行断言
「任何级别的任何走势类型终要完成」，第 36 行进一步断言走势完成后「就会转化为其他类型的
走势」。这是缠师的断言，不是能从更原始 Lean 命题推出的东西；对后继函数 `succ` 而言，
它是一条外部结构假设。编码形态对齐 `AncestorLifespan.lean` 的 `DirectParentContains`：采用
具名 `Prop`，下游定理以显式 binder `(hEH : EventualHandover succ)` 消费，不用 `axiom`
关键字。

★still-MISSING：`succ` 只是抽象后继函数参数。本文件不断言 `succ` 给出的确实是一个「不同
走势类型」的实例这一进一步忠实性条件。新走势类型在更高级别上的具体归属由 `Outcome³_q`
三分支给出，不并入 `Turn_q` 的定义（map #854 裁定）；本文件不处理，也不冒充已处理。

★本文件不得为 `EventualHandover` 构造任何「某个具体 `succ` 确实满足它」的正面实例；它必须
永远只作为假设被消费，理由同 `ForceVelocity.lean` 中 `ImpulseForceMeasureHypothesis` 从不被
构造实例的做法。第 6 节只反证具体 `succ` 不满足它，不构造满足它的正面实例。
-/
def EventualHandover (succ : TrendInstance → Option TrendInstance) : Prop :=
  ∀ T : TrendInstance, Completed T → ∃ T', succ T = some T'

/-! ## 4. `Turn_q` 的正面定义 -/

/--
`.chanlun/definitions/beichi.md:433`：`Turn_q(T_q^d) ⟺` 当前 q 级走势类型 `T_q^d` 完成，
并让位于一个新的 q 级走势类型。

这是肯定式（完成 ∧ 让位），不是 `¬Ext_q` 那种否定式；两者不是同义反复（issue #861
裁定③）。
-/
def Turn_q (succ : TrendInstance → Option TrendInstance) (T : TrendInstance) : Prop :=
  Completed T ∧ ∃ T', succ T = some T'

/-! ## 5. 显式消费基本原理一假设的推论 -/

/--
背驰完成证据与让位假设合取推出 `Turn_q`。

★这不是「背驰蕴含转折」的无条件版本（`Div_q ⟹ Turn_q`）：`hEH` 是本文件从不证明成立的
假设，缺了它就推不出让位那一半。本定理只说背驰完成证据 `hdiv` 加上基本原理一假设 `hEH`，
两者合起来才足够得到 `Turn_q`。

★这也不是拿基本原理一去证明「`Div_q ⟹ 完成`」：`hdiv` 本身就是 `Completed T` 的证据；
`Completed` 是背驰的别名，定义层面直接相等。基本原理一在本定理中只贡献「让位」那一半，
没有被用来推出「完成」。
-/
theorem turn_of_divergence_and_handover
    (succ : TrendInstance → Option TrendInstance)
    (hEH : EventualHandover succ)
    (T : TrendInstance) (hdiv : Completed T) :
    Turn_q succ T :=
  ⟨hdiv, hEH T hdiv⟩

/--
在显式的基本原理一假设下，若某实例没有后继，则它不能已经完成。`hEH` 在证明中提供完成后
必须存在后继的义务；没有该前件，结论不成立。
-/
theorem no_handover_implies_not_completed
    (succ : TrendInstance → Option TrendInstance)
    (hEH : EventualHandover succ)
    (T : TrendInstance) (hno : succ T = none) :
    ¬ Completed T := by
  intro hc
  obtain ⟨T', hT'⟩ := hEH T hc
  simp [hno] at hT'

/-! ## 6. 删前件自查 -/

/-- 反例 A 的不背驰实例：C=8 不小于 A=3。 -/
def T0 : TrendInstance :=
  { divPair := { forceA := ⟨3⟩, forceC := ⟨8⟩, isTrend := false } }

#eval decide (Completed T0)

example : ¬ Completed T0 := by decide

/-- 反例 B 的背驰实例：C=2 小于 A=8。 -/
def T1 : TrendInstance :=
  { divPair := { forceA := ⟨8⟩, forceC := ⟨2⟩, isTrend := false } }

/-- 一个永远不交出后继走势的具体后继函数。 -/
def succNone : TrendInstance → Option TrendInstance := fun _ => none

#eval decide (Completed T1)

example : Completed T1 := by decide

example : ¬ EventualHandover succNone := by
  intro hEH
  obtain ⟨T', hT'⟩ := hEH T1 (by decide)
  simp [succNone] at hT'

example : ¬ Turn_q succNone T1 := by
  rintro ⟨_, T', hT'⟩
  simp [succNone] at hT'

end NewChanlun.Origin
