import Std

/-
#1467：独立局部语义关系。只证明量化和三点K线的局部性质，
不把局部性质升级为整条笔/线段/递归解析的唯一性。
-/
namespace OrderBasePrimitive

/-- 用上下界独立定义非负整数商，不把“等于某程序输出”作为关系。 -/
def QuotientOK (x d k : Nat) : Prop := d * k ≤ x ∧ x < d * (k + 1)

theorem quotient_exists (x d : Nat) (hd : 0 < d) : QuotientOK x d (x / d) := by
  constructor
  · simpa only [Nat.mul_comm] using Nat.div_mul_le_self x d
  · exact Nat.lt_mul_div_succ x hd

theorem quotient_unique {x d k l : Nat}
    (hk : QuotientOK x d k) (hl : QuotientOK x d l) : k = l := by
  apply Nat.le_antisymm
  · by_cases h : k ≤ l
    · exact h
    · have hstep : l + 1 ≤ k := by omega
      have hm := Nat.mul_le_mul_left d hstep
      unfold QuotientOK at hk hl
      omega
  · by_cases h : l ≤ k
    · exact h
    · have hstep : k + 1 ≤ l := by omega
      have hm := Nat.mul_le_mul_left d hstep
      unfold QuotientOK at hk hl
      omega

/-- 半值向上的非负有理数量化，用其取值区间刻画。 -/
def HalfUpOK (n d k : Nat) : Prop := QuotientOK (2 * n + d) (2 * d) k

theorem half_up_exists_unique (n d : Nat) (hd : 0 < d) :
    ∃ k, HalfUpOK n d k ∧ ∀ l, HalfUpOK n d l → l = k := by
  have hd2 : 0 < 2 * d := by omega
  refine ⟨(2 * n + d) / (2 * d), quotient_exists _ _ hd2, ?_⟩
  intro l hl
  exact quotient_unique hl (quotient_exists _ _ hd2)

def PointContains (x y : Int) : Prop :=
  (x ≥ y ∧ x ≤ y) ∨ (y ≥ x ∧ y ≤ x)

theorem point_contains_iff_equal (x y : Int) : PointContains x y ↔ x = y := by
  unfold PointContains
  omega

inductive Shape where
  | rise | fall | top | bottom
deriving DecidableEq, Repr

/-- 无相邻等价点的三根点K，按几何不等式定义四型。 -/
def ShapeOK (left mid right : Int) : Shape → Prop
  | .rise => left < mid ∧ mid < right
  | .fall => left > mid ∧ mid > right
  | .top => left < mid ∧ mid > right
  | .bottom => left > mid ∧ mid < right

theorem shape_at_most_one (left mid right : Int) (s t : Shape)
    (hs : ShapeOK left mid right s) (ht : ShapeOK left mid right t) : s = t := by
  cases s <;> cases t <;> simp_all [ShapeOK] <;> omega

theorem point_shape_exists_unique (left mid right : Int)
    (hlm : left ≠ mid) (hmr : mid ≠ right) :
    ∃ s, ShapeOK left mid right s ∧ ∀ t, ShapeOK left mid right t → t = s := by
  have hex : ∃ s, ShapeOK left mid right s := by
    by_cases h1 : left < mid
    · by_cases h2 : mid < right
      · exact ⟨.rise, h1, h2⟩
      · exact ⟨.top, h1, by omega⟩
    · by_cases h2 : mid < right
      · exact ⟨.bottom, by omega, h2⟩
      · exact ⟨.fall, by omega, by omega⟩
  obtain ⟨s, hs⟩ := hex
  exact ⟨s, hs, fun t ht => shape_at_most_one left mid right t s ht hs⟩

structure Node where
  top : Bool
  raw : Nat
  merged : Nat
  price : Int

/-- 新笔必要条件的独立表达：异型、不共合并K、原始间隔、顶高于底。 -/
def StrokeAdmissible (a b : Node) : Prop :=
  a.top ≠ b.top ∧ a.merged + 3 ≤ b.merged ∧ a.raw + 3 < b.raw ∧
  (if a.top then b.price < a.price else a.price < b.price)

/-- 本研究参考的延伸选择：同型、更晚、更极端；不声称这是原文唯一选择器。 -/
def EndExtension (b c : Node) : Prop :=
  b.top = c.top ∧ b.raw ≤ c.raw ∧ b.merged ≤ c.merged ∧
  (if b.top then b.price < c.price else c.price < b.price)

theorem extending_endpoint_preserves_necessary_conditions (a b c : Node)
    (hab : StrokeAdmissible a b) (hbc : EndExtension b c) :
    StrokeAdmissible a c := by
  cases ha : a.top <;> cases hb : b.top <;> cases hc : c.top <;>
    simp_all [StrokeAdmissible, EndExtension] <;> omega

end OrderBasePrimitive

#print axioms OrderBasePrimitive.half_up_exists_unique
#print axioms OrderBasePrimitive.point_contains_iff_equal
#print axioms OrderBasePrimitive.point_shape_exists_unique
#print axioms OrderBasePrimitive.extending_endpoint_preserves_necessary_conditions
