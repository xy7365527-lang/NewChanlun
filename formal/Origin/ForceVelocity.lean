/-
Origin/ForceVelocity.lean — 力度的速度净增量定义（#875）

本文件只使用 Lean core 的 `Rat`，不引入 Mathlib。`Force.area` 继续保留为 MACD
代理槽；这里给出真力度 `impulse`，并把 MACD 侧尚未实现的义务留在显式假设载体中。

边界约定：本文件所有关于速度数值的断言只在良构笔上成立；零时长笔的速度值 `0`
是 `Rat` 除零总化的产物，不是有意义的速度。
-/

import Origin.ForceInterface

namespace NewChanlun.Origin

/-- 良构笔至少跨过一根 K 线，即结束序号严格晚于开始序号。 -/
def Stroke.WellFormed (s : Stroke) : Prop :=
  s.startIndex < s.endIndex

/--
一笔的速度：价格增量除以 K 线序号增量。

序号先各自转成 `Int` 再相减；因此 `endIndex < startIndex` 时得到负分母，
不会落入 `Nat.sub` 的截断语义。
-/
def Stroke.velocity (s : Stroke) : Rat :=
  ((s.endPrice - s.startPrice : Int) : Rat) /
    (((s.endIndex : Int) - (s.startIndex : Int) : Int) : Rat)

/-- 良构笔的 K 线序号增量在 `Rat` 中非零，不会进入除零总化分支。 -/
theorem Stroke.velocity_denominator_ne_zero (s : Stroke) (h : s.WellFormed) :
    (((s.endIndex : Int) - (s.startIndex : Int) : Int) : Rat) ≠ 0 := by
  apply Rat.ne_of_gt
  exact Rat.intCast_pos.mpr (Int.sub_pos.mpr (Int.ofNat_lt.mpr h))

/-- 良构且价格严格上涨的笔，其速度严格为正。 -/
theorem Stroke.velocity_pos (s : Stroke) (hWellFormed : s.WellFormed)
    (hPrice : s.startPrice < s.endPrice) :
    s.velocity > 0 := by
  unfold Stroke.velocity
  rw [Rat.div_def]
  exact Rat.mul_pos
    (Rat.intCast_pos.mpr (Int.sub_pos.mpr hPrice))
    (Rat.inv_pos.mpr
      (Rat.intCast_pos.mpr
        (Int.sub_pos.mpr (Int.ofNat_lt.mpr hWellFormed))))

/-- 非空笔串在给定首笔后的末笔；只给 `impulse` 的端点定义使用。 -/
def lastStroke : Stroke → List Stroke → Stroke
  | current, [] => current
  | _, next :: rest => lastStroke next rest

/--
笔串的力度（单位质量冲量）：末笔速度减首笔速度。空笔串没有端点，约定力度为 `0`。
-/
def impulse : List Stroke → Rat
  | [] => 0
  | first :: rest => (lastStroke first rest).velocity - first.velocity

/--
望远镜可加性。`aHead :: aTail` 与 `bHead :: bTail` 在接缝处共享同一笔时，
两段力度之和等于去掉第二段重复首笔后的拼接力度。

前件 `hSeam` 真正用于消去中间速度；没有该前件时，两个任意笔串的结论不成立。
-/
theorem impulse_append_tail
    (aHead bHead : Stroke) (aTail bTail : List Stroke)
    (hSeam : lastStroke aHead aTail = bHead) :
    impulse (aHead :: aTail) + impulse (bHead :: bTail) =
      impulse ((aHead :: aTail) ++ bTail) := by
  subst bHead
  cases bTail with
  | nil =>
      simp [impulse, lastStroke, Rat.sub_self, Rat.add_zero]
  | cons b bs =>
      have hlast :
          lastStroke aHead (aTail ++ b :: bs) = lastStroke b bs := by
        induction aTail generalizing aHead with
        | nil => rfl
        | cons next rest ih => exact ih next
      change
        (lastStroke aHead aTail).velocity - aHead.velocity +
            ((lastStroke b bs).velocity - (lastStroke aHead aTail).velocity) =
          (lastStroke aHead (aTail ++ b :: bs)).velocity - aHead.velocity
      rw [hlast]
      simp only [Rat.sub_eq_add_neg]
      calc
        _ = ((lastStroke aHead aTail).velocity +
              -(lastStroke aHead aTail).velocity) +
            ((lastStroke b bs).velocity + -aHead.velocity) := by
              ac_rfl
        _ = (lastStroke b bs).velocity + -aHead.velocity := by
          rw [Rat.add_neg_cancel, Rat.zero_add]

/--
在 Lean 侧尚无独立 MACD 实现时，`List Stroke` 上的 `ForceMeasure` 只能条件性地
由假设载体给出。`measure` 是外部提供的独立 `Force` 代理；另外两个字段正是它必须
相对于真力度 `impulse` 满足的义务。本文件不构造该 structure 的值。
-/
structure ImpulseForceMeasureHypothesis where
  /-- 外部 MACD（或其他独立代理）给出的 `Force`；不得由 `impulse` 反向定义。 -/
  measure : List Stroke → Force
  /-- 真力度序不下降时，代理面积序也不得下降。 -/
  mono : ∀ a b : List Stroke,
    impulse a ≤ impulse b → (measure a).area ≤ (measure b).area
  /-- 代理面积严格上升时，真力度序不得反转。 -/
  faithful : ∀ a b : List Stroke,
    (measure a).area < (measure b).area → impulse a ≤ impulse b

/--
把显式假设载体装配成 `ForceMeasure (List Stroke)`；真力度字段按定义固定为 `impulse`。
这不是一个无条件的真实 MACD 实例：构造它必须先提供独立 `measure` 及两条序义务。
-/
def ImpulseForceMeasureHypothesis.toForceMeasure
    (h : ImpulseForceMeasureHypothesis) : ForceMeasure (List Stroke) where
  measure := h.measure
  strength := impulse
  mono := h.mono
  faithful := h.faithful

end NewChanlun.Origin
