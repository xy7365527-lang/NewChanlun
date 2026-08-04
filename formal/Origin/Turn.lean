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
`T_q^d` 的最小抽象载体，只保留 `Turn_q` 需要的字段——级别，以及用来判定完成的背驰段对；
笔、线段、中枢等内部结构本文件不需要，故不引入。

★**名分：`level` 的编码形态已转正（2026-08-04，issue #815 M-3 裁定）。**

把级别写成 `Nat`、把「同级」写成数字相等（见 `EventualHandover` / `Turn_q`），**预设了
「级别是递归级别、可用一个自然数线性索引」**——issue #815 的 **M-3 已裁定：级别 ＝ 递归
定义**（本级由次级构成，`level_id = 下级 + 1`），persistence 聚类那套**不是定义**（其病在
`CalendarPeriodNamer` 的阈值表给级别装了**绝对锚点**，而缠论的级别是纯相对关系——「本级
由次级构成」就是全部内容，没有绝对参照系）。

**⟹ 本编码与 M-3 一致，无需重做。**（原标注 `[新缠论:候选]` 于 issue #877 关票后追认、
登记在 issue #815；M-3 落定后转正。）

分清楚：`Turn_q` 的**语义**（完成 ∧ 让位于同级）已由 issue #869 第 2 问裁定；本条转正的
是这个**编码形态**。
-/
structure TrendInstance where
  level : Nat
  divPair : DivergencePair
deriving Repr

/-! ## 2. 「完成」由背驰定义 -/

/--
走势类型的「完成」由背驰定义，不是纯形态学判据。

`.chanlun/definitions/beichi.md:228-232` 裁定原文：「『完成』由背驰定义，归动力学……
裁定把箭头定死：不是先由形状判出完成再喂给背驰，而是背驰判出来了就叫完成。」

这里的 `T.divPair` 应读作「`T` 自己的 `T.level` 那一级」的背驰证据。但这只是构造
`TrendInstance` 的调用方必须保证的约定，不是类型强制：`DivergencePair` 本身没有 `level`
字段，因此类型系统不会检查 `T.divPair` 是否确实由 `T.level` 那一级的走势数据算出。这是主链
既有的类型缺口，与 #857 勘察查出的 `MoveOutcome` 缺少 `level` 字段属于同一类问题，不是本文件
新引入的漏洞；本文件也不以恒真占位命题假装已经补上该检查。
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

后继与当前实例「同级」由 `T'.level = T.level` 直接钉死在定义里，不是留白。新走势类型在
更高级别上的具体归属才由 `Outcome³_q` 三分支给出，不并入 `Turn_q` 的定义（map #854 裁定）；
本文件不处理这一更高级别归属，也不冒充已处理。

★still-MISSING：`succ` 只是抽象后继函数参数。本文件不得声称某个具体 `succ` 忠实刻画了真实
缠论的走势后继关系，理由同 `ForceVelocity.lean` 中 `ImpulseForceMeasureHypothesis` 的纪律：
防止把未经验证的经验断言悄悄塞进「已证」结论。但为测试某个前件是否真的在出力而构造的
玩具 `succ`（如第 6 节的 `succNone` / `succSelf`）不受此限；它们不是对真实市场后继关系的
断言，而是删前件自查需要的反例材料。
-/
def EventualHandover (succ : TrendInstance → Option TrendInstance) : Prop :=
  ∀ T : TrendInstance, Completed T → ∃ T', succ T = some T' ∧ T'.level = T.level

/-! ## 4. `Turn_q` 的正面定义 -/

/--
`.chanlun/definitions/beichi.md:433`：`Turn_q(T_q^d) ⟺` 当前 q 级走势类型 `T_q^d` 完成，
并让位于一个新的 q 级走势类型。

这是肯定式（完成 ∧ 让位），不是 `¬Ext_q` 那种否定式；两者不是同义反复（issue #861
裁定③）。

★合取项 `T'.level = T.level` 的名分见 `TrendInstance` 的文档：**编码形态已于 2026-08-04
随 issue #815 的 M-3 裁定（级别 ＝ 递归定义）转正。**
-/
def Turn_q (succ : TrendInstance → Option TrendInstance) (T : TrendInstance) : Prop :=
  Completed T ∧ ∃ T', succ T = some T' ∧ T'.level = T.level

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
  obtain ⟨T', hT', _⟩ := hEH T hc
  simp [hno] at hT'

/-! ## 6. 删前件自查 -/

/-- 反例 A 的不背驰实例：C=8 不小于 A=3。 -/
def T0 : TrendInstance :=
  { level := 1
    divPair := { forceA := ⟨3⟩, forceC := ⟨8⟩, isTrend := false } }

#eval decide (Completed T0)

example : ¬ Completed T0 := by decide

/-- 反例 B 的背驰实例：C=2 小于 A=8。 -/
def T1 : TrendInstance :=
  { level := 1
    divPair := { forceA := ⟨8⟩, forceC := ⟨2⟩, isTrend := false } }

/-- 一个永远不交出后继走势的具体后继函数。 -/
def succNone : TrendInstance → Option TrendInstance := fun _ => none

#eval decide (Completed T1)

example : Completed T1 := by decide

example : ¬ EventualHandover succNone := by
  intro hEH
  obtain ⟨T', hT', _⟩ := hEH T1 (by decide)
  simp [succNone] at hT'

example : ¬ Turn_q succNone T1 := by
  rintro ⟨_, T', hT', _⟩
  simp [succNone] at hT'

/-- 删去 `hEH`：没有后继不妨碍 `T1` 已经完成。 -/
example : succNone T1 = none ∧ Completed T1 := ⟨rfl, by decide⟩

/-- 一个只用于删前件自查、把输入实例原样交回的玩具后继函数。 -/
def succSelf : TrendInstance → Option TrendInstance := fun T => some T

example : EventualHandover succSelf := fun T _ => ⟨T, rfl, rfl⟩

/-- 删去 `hno`：`succSelf` 满足让位假设，而 `T1` 仍然已经完成。 -/
example : succSelf T1 ≠ none ∧ Completed T1 :=
  ⟨by simp [succSelf], by decide⟩

/-- 总能给出后继、但故意把后继级别加一的玩具函数。 -/
def succWrongLevel : TrendInstance → Option TrendInstance :=
  fun T => some { level := T.level + 1, divPair := T.divPair }

example : ¬ EventualHandover succWrongLevel := by
  intro hEH
  obtain ⟨T', hT', hlvl⟩ := hEH T1 (by decide)
  have hT'eq : T' = { level := T1.level + 1, divPair := T1.divPair } := by
    simpa [succWrongLevel] using hT'.symm
  rw [hT'eq] at hlvl
  simp at hlvl

/--
若 `Turn_q` 的定义去掉 `T'.level = T.level` 这个合取项，下面这条会从「不满足」变成
「满足」（`T1` 已完成、`succWrongLevel T1` 总能给出某个后继）——这就是 `level` 在
`Turn_q` 里真的在承重的证据，不是装饰。
-/
example : ¬ Turn_q succWrongLevel T1 := by
  rintro ⟨_, T', hT', hlvl⟩
  have hT'eq : T' = { level := T1.level + 1, divPair := T1.divPair } := by
    simpa [succWrongLevel] using hT'.symm
  rw [hT'eq] at hlvl
  simp at hlvl

/-! ## 7. still-MISSING 诚实声明

- `succ` 只是一项抽象函数参数，本文件既不保证它给出的后继是「真的不同」的实例，也不断言
  它反映真实的缠论后继关系；结构上允许自继承，第 6 节的 `succSelf` 正是利用这一空隙构造的
  玩具见证。
- `T.divPair` 是否真是 `T.level` 那一级算出的背驰段对，本文件在类型上不检查，只把二者匹配
  作为构造 `TrendInstance` 时必须遵守的调用约定；根因是 `DivergencePair` 本身不带 `level`
  字段。
-/

end NewChanlun.Origin
