import Stage60Fixture
set_option maxRecDepth 100000
set_option maxHeartbeats 10000000
namespace Stage60
open NewChanlun.Origin

instance (ps : List Int) (s : Stroke) : Decidable (StrokeSource ps s) := by
 unfold StrokeSource Stroke.WellFormed
 cases s.direction <;> dsimp <;> infer_instance
instance (d : SegEndData) (ss : List Stroke) : Decidable (SelectedBy d ss) := by
 unfold SelectedBy
 cases extractSegEndData d.dir d.startPrice d.endPrice ss <;> infer_instance
instance (d : Direction) (ss : List Stroke) : Decidable (NoContainment d ss) := by
 unfold NoContainment
 dsimp
 infer_instance

theorem atoms_checked : ATOMS := by
 unfold ATOMS
 decide +kernel

theorem features_checked : FEATURES := by
 have h_positive_0 : SegEndComplete positiveD0 := by
  unfold SegEndComplete TopAboveBottom positiveD0
  dsimp
  unfold IsTopFractal SegmentEndUp FractalInSeq
  decide +kernel
 have h_positive_1 : SegEndComplete positiveD1 := by
  unfold SegEndComplete TopAboveBottom positiveD1
  dsimp
  unfold IsBottomFractal SegmentEndDown FractalInSeq
  decide +kernel
 have h_positive_2 : SegEndComplete positiveD2 := by
  unfold SegEndComplete TopAboveBottom positiveD2
  dsimp
  unfold IsTopFractal SegmentEndUp FractalInSeq
  decide +kernel
 have h_positive_3 : SegEndComplete positiveD3 := by
  unfold SegEndComplete TopAboveBottom positiveD3
  dsimp
  unfold IsBottomFractal SegmentEndDown FractalInSeq
  decide +kernel
 have h_positive_4 : SegEndComplete positiveD4 := by
  unfold SegEndComplete TopAboveBottom positiveD4
  dsimp
  unfold IsTopFractal SegmentEndUp FractalInSeq
  decide +kernel
 have h_negative_0 : SegEndComplete negativeD0 := by
  unfold SegEndComplete TopAboveBottom negativeD0
  dsimp
  unfold IsTopFractal SegmentEndUp FractalInSeq
  decide +kernel
 have h_negative_1 : SegEndComplete negativeD1 := by
  unfold SegEndComplete TopAboveBottom negativeD1
  dsimp
  unfold IsBottomFractal SegmentEndDown FractalInSeq
  decide +kernel
 have h_negative_2 : SegEndComplete negativeD2 := by
  unfold SegEndComplete TopAboveBottom negativeD2
  dsimp
  unfold IsTopFractal SegmentEndUp FractalInSeq
  decide +kernel
 have h_negative_3 : SegEndComplete negativeD3 := by
  unfold SegEndComplete TopAboveBottom negativeD3
  dsimp
  unfold IsBottomFractal SegmentEndDown FractalInSeq
  decide +kernel
 have h_negative_4 : SegEndComplete negativeD4 := by
  unfold SegEndComplete TopAboveBottom negativeD4
  dsimp
  unfold IsTopFractal SegmentEndUp FractalInSeq
  decide +kernel
 unfold FEATURES
 simp only [h_positive_0,h_positive_1,h_positive_2,h_positive_3,h_positive_4,h_negative_0,h_negative_1,h_negative_2,h_negative_3,h_negative_4, true_and]
 decide +kernel
theorem core_checked : CORE := by
 unfold CORE CenterConfirmedComplete DirAlternates
 decide +kernel
theorem force_checked : FORCE := by
 unfold FORCE
 decide +kernel
theorem exact_root : ATOMS ∧ FEATURES ∧ CORE ∧ FORCE := ⟨atoms_checked, features_checked, core_checked, force_checked⟩
end Stage60
