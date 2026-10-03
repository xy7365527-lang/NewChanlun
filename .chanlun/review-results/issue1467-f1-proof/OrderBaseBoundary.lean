import Std

/-
研究票：订单基底候选构造与合法性证据（#1467）。
范围：证明复用的逻辑边界与自然供需偏序的一个精确障碍。
本文件没有定义或证明完整缠论F1，不改生产/Origin正本。
-/
namespace OrderBaseBoundary

/-- 一个与程序输出等式分开的语义关系具有唯一合法结果。 -/
def UniqueRel {X U : Type} (R : X → U → Prop) : Prop :=
  ∀ x, ∃ u, R x u ∧ ∀ v, R x v → v = u

/-- 只有先取得语义唯一性前提，观察映射的复合才继承该性质。 -/
theorem semantic_unique_composes {E X U : Type} (φ : E → X)
    (R : X → U → Prop) (h : UniqueRel R) :
    UniqueRel (fun e u => R (φ e) u) := by
  intro e
  exact h (φ e)

/-- 确定选择一个合法结果，仍可存在第二个合法结果。 -/
def ambiguous (_ : Unit) (u : Nat) : Prop := u = 0 ∨ u = 1

def chosen (_ : Unit) : Nat := 0

theorem selector_is_legal (x : Unit) : ambiguous x (chosen x) := Or.inl rfl

theorem selector_does_not_supply_semantic_uniqueness : ¬ UniqueRel ambiguous := by
  intro h
  obtain ⟨u, _, hu⟩ := h ()
  have hzero := hu 0 (Or.inl rfl)
  have hone := hu 1 (Or.inr rfl)
  omega

/-- 固定买卖报价时的两侧数量，Nat坐标仅用于本研究反例。 -/
structure Qty where
  bid : Nat
  ask : Nat
deriving DecidableEq, Repr

/-- 自然供需偏序候选：买量不减且卖量不增，称为不弱于原来的买方状态。 -/
def supplyLE (x y : Qty) : Prop := x.bid ≤ y.bid ∧ y.ask ≤ x.ask

def small : Qty := ⟨1, 1⟩
def thick : Qty := ⟨2, 2⟩

theorem positive_witness :
    0 < small.bid ∧ 0 < small.ask ∧ 0 < thick.bid ∧ 0 < thick.ask := by
  decide

theorem two_sided_thickening_incomparable :
    ¬ supplyLE small thick ∧ ¬ supplyLE thick small := by
  simp [supplyLE, small, thick]

/--
  有不可比对的自然偏序，不能完整反映为整数价格的全序。
  仅否定“全序比较必反映全部自然供需关系”的强桥接，
  不否定有损投影，也不否定其他订单F1的存在。
-/
theorem no_exact_scalar_order_reflection (f : Qty → Int)
    (reflects : ∀ x y,
      (0 < x.bid ∧ 0 < x.ask) → (0 < y.bid ∧ 0 < y.ask) →
      f x ≤ f y → supplyLE x y) : False := by
  have hs : 0 < small.bid ∧ 0 < small.ask := by decide
  have ht : 0 < thick.bid ∧ 0 < thick.ask := by decide
  rcases Int.le_total (f small) (f thick) with h | h
  · exact two_sided_thickening_incomparable.1 (reflects small thick hs ht h)
  · exact two_sided_thickening_incomparable.2 (reflects thick small ht hs h)

/-- 有序报价下，数量加权分子的上下界。分母为正时可转成区间内的有理数。 -/
theorem weighted_numerator_bounds (b a qb qa : Nat) (h : b ≤ a) :
    b * (qb + qa) ≤ a * qb + b * qa ∧
    a * qb + b * qa ≤ a * (qb + qa) := by
  constructor
  · simpa only [Nat.mul_add] using
      Nat.add_le_add_right (Nat.mul_le_mul_right qb h) (b * qa)
  · simpa only [Nat.mul_add] using
      Nat.add_le_add_left (Nat.mul_le_mul_right qa h) (a * qb)

/-- 一组固定报价数量脉冲的三个不同有理数观察，使用交叉乘积核验。 -/
theorem pulse_observation_changes :
    (102 * 100 + 100 * 100) * (1000 + 100)
      < (102 * 1000 + 100 * 100) * (100 + 100) := by
  decide

end OrderBaseBoundary

#print axioms OrderBaseBoundary.semantic_unique_composes
#print axioms OrderBaseBoundary.selector_does_not_supply_semantic_uniqueness
#print axioms OrderBaseBoundary.no_exact_scalar_order_reflection
#print axioms OrderBaseBoundary.weighted_numerator_bounds
#print axioms OrderBaseBoundary.pulse_observation_changes
