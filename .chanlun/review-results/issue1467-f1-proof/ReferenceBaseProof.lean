import ReferenceBaseSpec

set_option maxRecDepth 100000
set_option maxHeartbeats 10000000

namespace ReferenceBase
open NewChanlun.Origin

instance (s : Stroke) : Decidable (StrokeSource s) := by
  unfold StrokeSource
  cases s.direction <;> unfold top bottom <;> infer_instance

instance (d : SegEndData) (ss : List Stroke) : Decidable (SelectedBy d ss) := by
  unfold SelectedBy
  cases extractSegEndData d.dir d.startPrice d.endPrice ss <;> infer_instance

instance (d : Direction) (ss : List Stroke) : Decidable (NoAdjacentContainment d ss) := by
  unfold NoAdjacentContainment
  dsimp
  infer_instance

theorem source_checked : SOURCE_TARGET := by
  unfold SOURCE_TARGET price qty
  decide

theorem strokes_checked : STROKE_TARGET := by
  unfold STROKE_TARGET
  decide

theorem features_checked : FEATURE_TARGET := by
  have h0 : SegEndComplete d0 := by
    unfold SegEndComplete TopAboveBottom d0
    dsimp
    unfold IsTopFractal SegmentEndUp FractalInSeq
    decide
  have h1 : SegEndComplete d1 := by
    unfold SegEndComplete TopAboveBottom d1
    dsimp
    unfold IsBottomFractal SegmentEndDown FractalInSeq
    decide
  have h2 : SegEndComplete d2 := by
    unfold SegEndComplete TopAboveBottom d2
    dsimp
    unfold IsTopFractal SegmentEndUp FractalInSeq
    decide
  unfold FEATURE_TARGET
  simp only [h0, h1, h2, true_and]
  decide +kernel

theorem geometry_checked : GEOMETRY_TARGET := by
  unfold GEOMETRY_TARGET SpanSource initialCore CenterConfirmedComplete DirAlternates
  decide

theorem exact_root : SOURCE_TARGET ∧ STROKE_TARGET ∧ FEATURE_TARGET ∧ GEOMETRY_TARGET :=
  ⟨source_checked,strokes_checked,features_checked,geometry_checked⟩

end ReferenceBase
