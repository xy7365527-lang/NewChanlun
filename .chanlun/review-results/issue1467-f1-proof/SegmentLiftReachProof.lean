import SegmentLiftReachSpec

set_option maxRecDepth 100000
set_option maxHeartbeats 15000000

namespace SegmentLiftReach
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

instance (i : Nat) : Decidable (SegEndComplete (datum i)) := by
  by_cases h : i%2=0
  · simp only [datum,seg,h,↓reduceIte]
    unfold SegEndComplete TopAboveBottom
    dsimp
    unfold IsTopFractal SegmentEndUp FractalInSeq
    infer_instance
  · simp only [datum,seg,h,↓reduceIte]
    unfold SegEndComplete TopAboveBottom
    dsimp
    unfold IsBottomFractal SegmentEndDown FractalInSeq
    infer_instance

theorem source_checked : SOURCE_TARGET := by
  unfold SOURCE_TARGET price qty
  decide +kernel

theorem strokes_checked : STROKE_TARGET := by
  unfold STROKE_TARGET
  decide +kernel

theorem segments_checked : SEGMENT_TARGET := by
  unfold SEGMENT_TARGET SourceRange initialCore
  decide +kernel

theorem frame_checked : FRAME_TARGET := by
  have r0 : CenterFrame.Returned .short f0 (unit 3) (unit 4) := by
    unfold CenterFrame.Returned CenterFrame.PairBase CenterAttempt.Joined CenterConfirmedComplete DirAlternates
    decide +kernel
  have r1 : CenterFrame.Returned .short f1 (unit 5) (unit 6) := by
    unfold CenterFrame.Returned CenterFrame.PairBase CenterAttempt.Joined CenterConfirmedComplete DirAlternates
    decide +kernel
  have r2 : CenterFrame.Returned .short f2 (unit 7) (unit 8) := by
    unfold CenterFrame.Returned CenterFrame.PairBase CenterAttempt.Joined CenterConfirmedComplete DirAlternates
    decide +kernel
  have h0 : CenterFrame.Absorb .short f0 (unit 3) (unit 4) (.active f1) :=
    ⟨r0,by decide,CenterFrame.recorded_extend _ _ _,by decide⟩
  have h1 : CenterFrame.Absorb .short f1 (unit 5) (unit 6) (.active f2) :=
    ⟨r1,by decide,CenterFrame.recorded_extend _ _ _,by decide⟩
  have h2 : CenterFrame.Absorb .short f2 (unit 7) (unit 8) (.retilePending f3) :=
    ⟨r2,by decide,CenterFrame.recorded_extend _ _ _,by decide⟩
  refine ⟨h0,h1,h2,rfl,by decide,by decide,by decide,by decide,by decide,rfl,by decide,by decide,?_⟩
  exact Retile.recover_retile f3.knownAt (CenterFrame.members f3)

theorem rejection_checked : REJECTION_TARGET := by
  intro a b c hal hah hbl hbh hcl hch h
  have hc := h.2
  simp [computeZD,computeZG,hal,hah,hbl,hbh,hcl,hch,tmin,tmax] at hc

theorem exact_root : SOURCE_TARGET ∧ STROKE_TARGET ∧ SEGMENT_TARGET ∧ FRAME_TARGET ∧ REJECTION_TARGET :=
  ⟨source_checked,strokes_checked,segments_checked,frame_checked,rejection_checked⟩

end SegmentLiftReach
