import CenterExitProof
import RetilePacketProof
import NineRegroupSpec

namespace ExitLift

open CenterAttempt (ObservedUnit obs visible)
open UpgradeResearch (Interval Valid core3)

def path (i : Nat) : Int := if i ≤ 10 then CenterExit.path false i else 10054
def extra : DCSpec.Unit := ⟨.down,9,10,11⟩
def allUnits := CenterExit.units ++ [extra]
def tail : DCSpec.Tail := ⟨.up,10,11⟩
def result : DCSpec.Result := .started .down 0 1 (allUnits,tail)
def seen (t : Nat) := visible path result t

def PREFIX_TARGET : Prop :=
  DCSpec.Whole path 2 12 result ∧
  (∀ i : Fin 11, path i.val = CenterExit.path false i.val) ∧
  (∀ i : Fin 12, 0 < CenterAttempt.quantity path i.val ∧
    CenterAttempt.projected (CenterAttempt.quantity path i.val) = path i.val) ∧
  seen 9 = visible (CenterExit.path false) CenterExit.result 9 ∧
  seen 10 = visible (CenterExit.path false) CenterExit.result 10 ∧
  (seen 9).length = 7 ∧ (seen 10).length = 8 ∧ (seen 11).length = 9 ∧
  seen 9 = CenterFrame.members (CenterExit.frame false) ++
    [CenterExit.leave false] ++ Retile.tripleMembers (CenterExit.nextTriple false) ∧
  Retile.Local (CenterExit.initialTriple false) ∧ Retile.Local (CenterExit.nextTriple false) ∧
  Formal.CenterTrichotomy.IsLevelExpansion (CenterExit.initialCenter false) (CenterExit.nextCenter false)

def SOURCE_TARGET : Prop :=
  (∀ (α : Type) (xs a b c : List α), NineRegroup.SourceTriplet xs a b c → 9 ≤ xs.length) ∧
  (¬ ∃ a b c, NineRegroup.SourceTriplet (seen 9) a b c) ∧
  (¬ ∃ a b c, NineRegroup.SourceTriplet (seen 10) a b c)

def oldPart : SourceHull.Part := ⟨.candidate,0,3⟩
def nextPart : SourceHull.Part := ⟨.candidate,4,3⟩
def gapPart : SourceHull.Part := ⟨.retained,3,1⟩
def frozenPlan (middle : SourceHull.Part) (rest : List SourceHull.Part) :=
  oldPart :: middle :: nextPart :: rest
def FROZEN_TARGET : Prop :=
  SourceHull.Cover 0 7 (frozenPlan gapPart []) ∧
  (∀ finish middle rest, SourceHull.Cover 0 finish (frozenPlan middle rest) →
    middle.start = 3 ∧ middle.len = 1 ∧ ¬ 3 ≤ middle.len)

def packets := RetilePacket.assemble 11 0 (seen 11)
def parentCore : Option Interval :=
  match packets.map (·.outer) with
  | [some a,some b,some c] => some (core3 a b c)
  | _ => none

def LATE_TARGET : Prop :=
  RetilePacket.recover packets = seen 11 ∧
  SourceHull.Cover 0 9 (packets.map (·.part)) ∧
  packets.map (fun p => p.part.start) = [0,3,6] ∧
  packets.map (fun p => p.part.len) = [3,3,3] ∧
  packets.map (fun p => p.part.role) = [.candidate,.candidate,.candidate] ∧
  packets.map (·.outer) = [some ⟨10020,10040⟩,some ⟨10024,10060⟩,some ⟨10036,10060⟩] ∧
  packets.map (·.availableAt) = [11,11,11] ∧
  parentCore = some ⟨10036,10040⟩ ∧ Valid ⟨10036,10040⟩

end ExitLift
