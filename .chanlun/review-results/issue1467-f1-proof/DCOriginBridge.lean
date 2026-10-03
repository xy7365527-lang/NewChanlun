import DCGeometryProof
import Origin.CenterComplete

/- #1467：实际Origin局部中枢谓词的桥接与数值反例，不声称完整F₂闭合。 -/
namespace DCOriginBridge

open NewChanlun.Origin

def direction : DCSpec.Dir → Direction
  | .up => .up
  | .down => .down

/-- Segment在此只作现有谓词的几何输入载体，不给DC单元教义线段身份。 -/
def embed (p : Nat → Int) (u : DCSpec.Unit) : Segment :=
  ⟨direction u.direction, u.start, u.finish, p u.start, p u.finish⟩

theorem flip_ne (d : DCSpec.Dir) : direction d ≠ direction (DCSpec.flip d) := by
  cases d <;> simp [direction, DCSpec.flip]

/-- 已证DC关系的连续三单元，只需再检查严格核心非空，即满足现有局部谓词。 -/
theorem dc_three_center {p : Nat → Int} {delta : Int} {len : Nat} {result : DCSpec.Result}
    (hd : 0 < delta) (hwhole : DCSpec.Whole p delta len result)
    (pre : List DCSpec.Unit) (a b c : DCSpec.Unit) (rest : List DCSpec.Unit)
    (hseq : DCSpec.units result = pre ++ a :: b :: c :: rest)
    (hcore : computeZD (embed p a) (embed p b) (embed p c) <
      computeZG (embed p a) (embed p b) (embed p c)) :
    CenterConfirmedComplete (embed p a) (embed p b) (embed p c) := by
  have geometry := DCSpec.dc_geometry p delta hd len result hwhole
  have hab := (geometry.2.2 pre a b (c::rest) hseq).2
  have hbc := (geometry.2.2 (pre ++ [a]) b c rest (by simpa [List.append_assoc] using hseq)).2
  refine ⟨⟨?_, ?_⟩, hcore⟩
  · change direction a.direction ≠ direction b.direction
    rw [← hab]
    exact flip_ne _
  · change direction b.direction ≠ direction c.direction
    rw [← hbc]
    exact flip_ne _

def childA : Segment := ⟨.down,3,4,10033,10029⟩
def childB : Segment := ⟨.up,4,5,10029,10037⟩
def childC : Segment := ⟨.down,5,6,10037,10031⟩

theorem own_child_core : computeZD childA childB childC = 10031 ∧
    computeZG childA childB childC = 10033 := by decide

theorem own_child_confirmed : CenterConfirmedComplete childA childB childC := by
  unfold CenterConfirmedComplete DirAlternates
  decide

/-- 真实探针返回的DD=10029、ZD=10027，不可能同时具有CenterFull的证明字段。 -/
theorem inherited_core_not_CenterFull :
    ¬ ∃ c : CenterFull, c.dd = 10029 ∧ c.core.zd = 10027 := by
  intro ⟨c, hdd, hzd⟩
  have h := c.outer_lo
  rw [hdd,hzd] at h
  simp only [Tick] at h
  omega

def leftEnvelope : Segment := ⟨.up,0,3,10025,10039⟩
def connector : Segment := ⟨.down,3,4,10039,10037⟩
def rightEnvelope : Segment := ⟨.up,4,7,10037,10047⟩

theorem connector_geometric_witness :
    CenterConfirmedComplete leftEnvelope connector rightEnvelope ∧
    computeZD leftEnvelope connector rightEnvelope = 10037 ∧
    computeZG leftEnvelope connector rightEnvelope = 10039 := by
  unfold CenterConfirmedComplete DirAlternates
  decide

end DCOriginBridge

#print axioms DCOriginBridge.dc_three_center
#print axioms DCOriginBridge.own_child_core
#print axioms DCOriginBridge.inherited_core_not_CenterFull
#print axioms DCOriginBridge.connector_geometric_witness
