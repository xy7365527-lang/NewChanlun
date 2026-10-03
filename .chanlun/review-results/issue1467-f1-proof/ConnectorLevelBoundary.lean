import Formal.RecursiveConstruction

/- #1467：反驳把一条低级连接单元直接装箱/改层号当作完整上级走势的适配。 -/
namespace ConnectorLevelBoundary

open Formal.RecursiveConstruction

theorem singleton_is_not_wellformed_upper (u : Move)
    (centers : List Formal.CenterTrichotomy.Center) (level : Nat) :
    ¬ WellFormed (Move.compose [u] centers level) := by
  intro h
  simp only [WellFormed] at h
  have count := h.1
  simp at count

end ConnectorLevelBoundary

#print axioms ConnectorLevelBoundary.singleton_is_not_wellformed_upper
