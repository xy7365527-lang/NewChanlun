import Init

/-!
# raw 包含初始化 W（#1393；#1342 comment6149622771）

这是 raw 初始化的具名定义，不借用特征序列 processUp。语义来自 baohan v1.6：
首个相邻 raw 非包含严格对出现前保持原件，出现后从 raw0 左折叠。
只形式化初始化/几何投影，不声称覆盖 OHLC、RD-02 来源集合、S 持久身份或 Rust 等价。
本轮不执行 Lean 构建；以下证明无占位，其可编译性待授权环境定向核验。
-/
namespace Origin.RawInclusionInitialization

inductive Direction where
  | up | down
  deriving DecidableEq, Repr

structure Raw where
  low : Int
  high : Int
  deriving DecidableEq, Repr

def Valid (a : Raw) : Prop := a.low ≤ a.high

def Contains (a b : Raw) : Prop :=
  (a.low ≤ b.low ∧ b.high ≤ a.high) ∨ (b.low ≤ a.low ∧ a.high ≤ b.high)

instance (a b : Raw) : Decidable (Contains a b) := inferInstanceAs (Decidable
  ((a.low ≤ b.low ∧ b.high ≤ a.high) ∨ (b.low ≤ a.low ∧ a.high ≤ b.high)))

def strictDirection (a b : Raw) : Option Direction :=
  if a.high < b.high ∧ a.low < b.low then some .up
  else if b.high < a.high ∧ b.low < a.low then some .down
  else none

structure Evidence where
  offset : Nat
  left : Raw
  right : Raw
  direction : Direction
  deriving DecidableEq, Repr

def shiftEvidence (e : Evidence) : Evidence := { e with offset := e.offset + 1 }

/-- 按到达序搜索；包含时只移向下一对，不比较 raw0 与任意后根。 -/
def firstPair : List Raw → Option Evidence
  | [] => none
  | [_] => none
  | a :: b :: rest =>
    if Contains a b then (firstPair (b :: rest)).map shiftEvidence
    else match strictDirection a b with
      | some d => some ⟨0, a, b, d⟩
      | none => (firstPair (b :: rest)).map shiftEvidence

/-- 已裁方向合并；不改 contains/merge/strictDirection 的几何。 -/
def merge (d : Direction) (a b : Raw) : Raw :=
  match d with
  | .up => ⟨max a.low b.low, max a.high b.high⟩
  | .down => ⟨min a.low b.low, min a.high b.high⟩

/-- 每个非包含步按真实 acc 与 incoming 更新方向。 -/
def foldFrom (d : Direction) (acc : Raw) : List Raw → List Raw
  | [] => [acc]
  | b :: rest =>
    if Contains acc b then foldFrom d (merge d acc b) rest
    else acc :: foldFrom ((strictDirection acc b).getD d) b rest

def foldRaw (d : Direction) : List Raw → List Raw
  | [] => []
  | a :: rest => foldFrom d a rest

inductive Initialization where
  | undetermined (raw : List Raw)
  | established (evidence : Evidence) (groups : List Raw)
  deriving DecidableEq, Repr

def initialize (raw : List Raw) : Initialization :=
  match firstPair raw with
  | none => .undetermined raw
  | some e => .established e (foldRaw e.direction raw)

def AllAdjacentContained : List Raw → Prop
  | [] => True
  | [_] => True
  | a :: b :: rest => Contains a b ∧ AllAdjacentContained (b :: rest)

theorem empty_undetermined : initialize [] = .undetermined [] := rfl

theorem singleton_undetermined (a : Raw) : initialize [a] = .undetermined [a] := rfl

theorem first_strict_pair (a b : Raw) (rest : List Raw) (d : Direction)
    (hn : ¬ Contains a b) (hd : strictDirection a b = some d) :
    initialize (a :: b :: rest) =
      .established ⟨0, a, b, d⟩ (foldRaw d (a :: b :: rest)) := by
  simp [initialize, firstPair, hn, hd]

theorem all_contained_no_pair (raw : List Raw) :
    AllAdjacentContained raw → firstPair raw = none := by
  induction raw with
  | nil => intro _; rfl
  | cons a rest ih =>
    cases rest with
    | nil => intro _; rfl
    | cons b tail =>
      intro h
      have parts : Contains a b ∧ AllAdjacentContained (b :: tail) := h
      simp [firstPair, parts.1, ih parts.2]

theorem all_contained_undetermined (raw : List Raw) (h : AllAdjacentContained raw) :
    initialize raw = .undetermined raw := by
  simp [initialize, all_contained_no_pair raw h]

/-- 任意延迟首对一经出现，重折仍使用整个 raw，不能从证据对才开始。 -/
theorem established_replays_from_zero (raw : List Raw) (e : Evidence)
    (h : firstPair raw = some e) :
    initialize raw = .established e (foldRaw e.direction raw) := by
  simp [initialize, h]

/-- U04：相邻包含而非链式，不能偷用首尾方向。 -/
example : initialize [⟨0,10⟩, ⟨2,8⟩, ⟨1,11⟩] =
    .undetermined [⟨0,10⟩, ⟨2,8⟩, ⟨1,11⟩] := by decide

/-- U10：DOWN 初始化可重折成单组，后续仍按 DOWN。 -/
example : initialize [⟨0,10⟩, ⟨2,8⟩, ⟨1,7⟩, ⟨2,6⟩] =
    .established ⟨1, ⟨2,8⟩, ⟨1,7⟩, .down⟩ [⟨0,6⟩] := by decide

end Origin.RawInclusionInitialization
