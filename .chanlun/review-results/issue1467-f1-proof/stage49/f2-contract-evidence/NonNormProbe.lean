import Origin.CenterComplete

/- 只检验局部 range/direction 消费接口；没有定义 Completed，也不宣称这些对象是完整 a₁。 -/
namespace Stage49.F2InputContract
open NewChanlun.Origin

def pathA : List Int := [10, 10, 10, 10, 30, 20, 20, 20]
def pathB : List Int := [20, 25, 5]
def pathC : List Int := [5, 28, 10]

/-- 空路径不产生范围；非空路径从自身首值开始，不用 0 哨兵。 -/
def fullRange : List Int → Option (Int × Int)
  | [] => none
  | h :: t => some (t.foldl min h, t.foldl max h)

def actualA : Segment := ⟨.up, 0, 7, 10, 20⟩
def actualB : Segment := ⟨.down, 7, 9, 20, 5⟩
def actualC : Segment := ⟨.up, 9, 11, 5, 10⟩

/-- 只为调用既有数值谓词而重编码 full range；这些 start/endPrice 不是实际边界价。 -/
def rangeA : Segment := ⟨.up, 0, 7, 10, 30⟩
def rangeB : Segment := ⟨.down, 7, 9, 25, 5⟩
def rangeC : Segment := ⟨.up, 9, 11, 5, 28⟩

def Norm (s : Segment) (p : List Int) : Prop :=
  match s.direction with
  | .up => fullRange p = some (s.startPrice, s.endPrice) ∧ s.startPrice < s.endPrice
  | .down => fullRange p = some (s.endPrice, s.startPrice) ∧ s.endPrice < s.startPrice

instance (s : Segment) (p : List Int) : Decidable (Norm s p) := by
  unfold Norm
  cases s.direction <;> infer_instance

theorem real_ranges :
    fullRange pathA = some (10, 30) ∧ fullRange pathB = some (5, 25) ∧
    fullRange pathC = some (5, 28) := by decide

theorem actual_all_non_norm :
    ¬ Norm actualA pathA ∧ ¬ Norm actualB pathB ∧ ¬ Norm actualC pathC := by decide

theorem actual_boundaries_connect :
    actualA.endIndex = actualB.startIndex ∧ actualB.endIndex = actualC.startIndex ∧
    actualA.endPrice = actualB.startPrice ∧ actualB.endPrice = actualC.startPrice := by decide

theorem actual_directions_alternate : DirAlternates actualA actualB actualC := by decide

theorem endpoint_projection_loses_range :
    (segLow actualA, segHigh actualA) ≠ (10, 30) ∧
    (segLow actualB, segHigh actualB) ≠ (5, 25) ∧
    (segLow actualC, segHigh actualC) ≠ (5, 28) := by decide

theorem endpoints_reject_at_singleton :
    computeZD actualA actualB actualC = 10 ∧ computeZG actualA actualB actualC = 10 ∧
    ¬ CenterConfirmedComplete actualA actualB actualC := by
  unfold CenterConfirmedComplete
  decide

theorem range_encoding_is_exact :
    fullRange pathA = some (segLow rangeA, segHigh rangeA) ∧
    fullRange pathB = some (segLow rangeB, segHigh rangeB) ∧
    fullRange pathC = some (segLow rangeC, segHigh rangeC) := by decide

theorem ranges_pass_local_criterion :
    computeZD rangeA rangeB rangeC = 10 ∧ computeZG rangeA rangeB rangeC = 25 ∧
    computeDD rangeA rangeB rangeC = 5 ∧ computeGG rangeA rangeB rangeC = 30 ∧
    CenterConfirmedComplete rangeA rangeB rangeC := by
  unfold CenterConfirmedComplete
  decide

theorem range_encoding_changes_boundary_and_breaks_connection :
    rangeA.endPrice ≠ actualA.endPrice ∧ rangeB.startPrice ≠ actualB.startPrice ∧
    rangeC.endPrice ≠ actualC.endPrice ∧ rangeA.endPrice ≠ rangeB.startPrice := by decide

/-- 两点载体保真需要的恰是范围相等；这是适配层义务，不能从可构造 Segment 自动得到。 -/
theorem upward_endpoint_projection_iff (s e lo hi : Int) (h : s < e) :
    ((if s ≤ e then s else e) = lo ∧ (if s ≥ e then s else e) = hi) ↔
    (s = lo ∧ e = hi) := by
  have hle : s ≤ e := by omega
  have hnot : ¬s ≥ e := by omega
  simp only [if_pos hle, if_neg hnot]

#check @real_ranges
#check @actual_all_non_norm
#check @actual_boundaries_connect
#check @actual_directions_alternate
#check @endpoint_projection_loses_range
#check @endpoints_reject_at_singleton
#check @range_encoding_is_exact
#check @ranges_pass_local_criterion
#check @range_encoding_changes_boundary_and_breaks_connection
#check @upward_endpoint_projection_iff
#print axioms real_ranges
#print axioms actual_all_non_norm
#print axioms actual_boundaries_connect
#print axioms actual_directions_alternate
#print axioms endpoint_projection_loses_range
#print axioms endpoints_reject_at_singleton
#print axioms range_encoding_is_exact
#print axioms ranges_pass_local_criterion
#print axioms range_encoding_changes_boundary_and_breaks_connection
#print axioms upward_endpoint_projection_iff
#eval fullRange pathA
#eval fullRange pathB
#eval fullRange pathC
#eval (centerHolds actualA actualB actualC, centerHolds rangeA rangeB rangeC)
#eval (computeZD rangeA rangeB rangeC, computeZG rangeA rangeB rangeC,
       computeDD rangeA rangeB rangeC, computeGG rangeA rangeB rangeC)

end Stage49.F2InputContract
