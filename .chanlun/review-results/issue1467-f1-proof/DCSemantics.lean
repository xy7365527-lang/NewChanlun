import Std

/- #1467 R_W-DC-v0的独立关系规格。价格为任意整数，长度包括0，δ>0。
   本文件只有定义；证明文件不得通过修改它来使目标变弱。 -/
namespace DCSpec

inductive Dir where
  | up | down
  deriving DecidableEq, Repr

def flip : Dir → Dir
  | .up => .down
  | .down => .up

def score (d : Dir) (x : Int) : Int := match d with
  | .up => x
  | .down => -x

/-- 区间内最早的方向极值。上行取最早最大值，下行取最早最小值。 -/
def Extreme (p : Nat → Int) (d : Dir) (s r e : Nat) : Prop :=
  s ≤ e ∧ e ≤ r ∧
  (∀ j, s ≤ j → j ≤ r → score d (p j) ≤ score d (p e)) ∧
  ∀ j, s ≤ j → j < e → p j ≠ p e

def First (P : Nat → Prop) (t : Nat) : Prop := P t ∧ ∀ j, j < t → ¬ P j

/-- 存在任意一对较早/当前观察形成δ回撤；不调用运行极值选择函数。 -/
def Drop (p : Nat → Int) (delta : Int) (d : Dir) (s t : Nat) : Prop :=
  s < t ∧ ∃ k, s ≤ k ∧ k < t ∧ delta ≤ score d (p k) - score d (p t)

/-- 前缀振幅至少δ，不要求两极值的时间方向。 -/
def Wide (p : Nat → Int) (delta : Int) (t : Nat) : Prop :=
  ∃ a b, a ≤ t ∧ b ≤ t ∧ delta ≤ p a - p b

def Initial (p : Nat → Int) (delta : Int) (d : Dir) (s c : Nat) : Prop :=
  First (Wide p delta) c ∧ Extreme p (flip d) 0 c s ∧
  delta ≤ score d (p c) - score d (p s)

/-- 仅用于存在性证明的不变式，不作为Whole的附加准入前提。 -/
def Seeded (p : Nat → Int) (delta : Int) (d : Dir) (s n : Nat) : Prop :=
  ∃ c, s < c ∧ c ≤ n ∧ delta ≤ score d (p c) - score d (p s) ∧
    (∀ t, t ≤ c → ¬ Drop p delta d s t) ∧
    ∀ j, s ≤ j → j ≤ c → score d (p s) ≤ score d (p j)

structure Unit where
  direction : Dir
  start : Nat
  finish : Nat
  knownAt : Nat
  deriving DecidableEq, Repr

structure Tail where
  direction : Dir
  start : Nat
  extreme : Nat
  deriving DecidableEq, Repr

/-- 独立贪心分解关系。首次回撤与最早极值独立定义；尾部不能任意截断。 -/
inductive Run (p : Nat → Int) (delta : Int) (n : Nat) :
    Dir → Nat → (List Unit × Tail) → Prop where
  | stop {d s e} (hno : ∀ t, t ≤ n → ¬ Drop p delta d s t)
      (he : Extreme p d s n e) : Run p delta n d s ([], ⟨d, s, e⟩)
  | next {d s e t rest} (ht : t ≤ n) (hf : First (Drop p delta d s) t)
      (he : Extreme p d s (t-1) e) (hprogress : s < e)
      (hr : Run p delta n (flip d) e rest) :
      Run p delta n d s (⟨d, s, e, t⟩ :: rest.1, rest.2)

inductive Result where
  | empty
  | waiting (low high : Nat)
  | started (initialDirection : Dir) (initialStart initializedAt : Nat)
      (output : List Unit × Tail)
  deriving DecidableEq, Repr

/-- 参数len是观察数量；p在len外的值无意义。前导片段由initialStart标明。 -/
inductive Whole (p : Nat → Int) (delta : Int) : Nat → Result → Prop where
  | empty : Whole p delta 0 .empty
  | waiting {n lo hi} (hno : ∀ t, t ≤ n → ¬ Wide p delta t)
      (hl : Extreme p .down 0 n lo) (hh : Extreme p .up 0 n hi) :
      Whole p delta (n+1) (.waiting lo hi)
  | started {n d s c output} (hc : c ≤ n) (hi : Initial p delta d s c)
      (hr : Run p delta n d s output) :
      Whole p delta (n+1) (.started d s c output)

def units : Result → List Unit
  | .started _ _ _ output => output.1
  | _ => []

def DC_CANON_TARGET : Prop :=
  ∀ (p : Nat → Int) (delta : Int), 0 < delta → ∀ len,
    ∃ result, Whole p delta len result ∧
      ∀ other, Whole p delta len other → other = result

def DC_CAUSAL_TARGET : Prop :=
  ∀ (p q : Nat → Int) (delta : Int), 0 < delta →
    ∀ len₁ len₂ t r₁ r₂, t < len₁ → t < len₂ →
      (∀ i, i ≤ t → p i = q i) →
      Whole p delta len₁ r₁ → Whole q delta len₂ r₂ →
      (units r₁).filter (fun u => u.knownAt ≤ t) =
      (units r₂).filter (fun u => u.knownAt ≤ t)

def UnitGeometry (p : Nat → Int) (delta : Int) (len : Nat) (u : Unit) : Prop :=
  u.start < u.finish ∧ u.finish < u.knownAt ∧ u.knownAt < len ∧
  delta ≤ score u.direction (p u.finish) - score u.direction (p u.start) ∧
  ∀ j, u.start ≤ j → j ≤ u.finish →
    score u.direction (p u.start) ≤ score u.direction (p j) ∧
    score u.direction (p j) ≤ score u.direction (p u.finish)

def DC_GEOMETRY_TARGET : Prop :=
  ∀ (p : Nat → Int) (delta : Int), 0 < delta → ∀ len result,
    Whole p delta len result →
    (∀ u ∈ units result, UnitGeometry p delta len u) ∧
    (units result).Pairwise (fun a b => a.knownAt < b.knownAt) ∧
    ∀ pre a b post, units result = pre ++ a :: b :: post →
      a.finish = b.start ∧ flip a.direction = b.direction

end DCSpec
