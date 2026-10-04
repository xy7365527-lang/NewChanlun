import PressureDCKernelSpec
import PressureDCProof

namespace PressureDCKernel
open PressureDC MovingQuote

theorem update_equiv (s : State) (i : Nat) (x : Int) :
    dcUpdate i x s = localUpdate i x s ∧
    (dcUpdate i x s).state.extremePrice = x ∧ (dcUpdate i x s).state.extremeIndex = i := by
  rcases s with ⟨d,p,t⟩
  cases d with
  | false =>
    by_cases h : x ≤ p
    · have hn : ¬ p < x := by omega
      simp [dcUpdate,localUpdate,recover,h,hn]
    · have hl : p < x := by omega
      have hd : 1 ≤ x-p := by omega
      simp [dcUpdate,localUpdate,recover,h,hl,hd]
  | true =>
    by_cases h : p ≤ x
    · have hn : ¬ x < p := by omega
      simp [dcUpdate,localUpdate,recover,h,hn]
    · have hl : x < p := by omega
      have hn : ¬ p < x := by omega
      have hd : 1 ≤ p-x := by omega
      simp [dcUpdate,localUpdate,recover,h,hl,hn,hd]

theorem initialize_source {e a b q r} (hs : Step e a b) (hq : QuoteOf a q) (hr : QuoteOf b r) :
    initDC (select (!(positive e)) q) (select (positive e) r) =
      some ⟨positive e,select (positive e) r,1⟩ := by
  have hd := (PressureDC.recover_initial false (positive e) q r hq.2.2 (quote_step hs hq hr)).2
  cases hp : positive e
  · simp only [hp,Bool.not_false,select,Bool.false_eq_true,↓reduceIte] at hd ⊢
    have hl : r.bid < q.ask := by omega
    have hn : ¬ q.ask < r.bid := by omega
    simp [initDC,hl,hn]
  · simp only [hp,Bool.not_true,select,Bool.false_eq_true,↓reduceIte] at hd ⊢
    have hl : q.bid < r.ask := by omega
    simp [initDC,hl]

theorem kernel_checked : KERNEL_TARGET :=
  ⟨update_equiv,fun _ _ _ _ _ => initialize_source⟩

theorem exact_root : PressureDC.PROJECTION_TARGET ∧ PressureDC.TRACE_TARGET ∧
    PressureDC.GEOMETRY_TARGET ∧ PressureDC.TIE_TARGET ∧ KERNEL_TARGET :=
  ⟨PressureDC.projection_checked,PressureDC.trace_checked,PressureDC.geometry_checked,
   PressureDC.tie_checked,kernel_checked⟩

end PressureDCKernel
