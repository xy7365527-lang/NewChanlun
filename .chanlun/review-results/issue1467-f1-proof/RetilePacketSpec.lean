import SourceHullProof

namespace RetilePacket

open CenterAttempt (ObservedUnit)
open SourceHull (Part Role)

structure Packet where
  part : Part
  source : List ObservedUnit
  outer : Option UpgradeResearch.Interval
  availableAt : Nat
  cell : Option Retile.Cell

def latest (xs : List ObservedUnit) : Nat :=
  xs.foldr (fun u t => max u.1.knownAt t) 0

def cellPacket (time start : Nat) (t : Retile.Triple) : Packet :=
  let c := Retile.classify time t
  let src := Retile.tripleMembers t
  { part := ⟨if Retile.accepted c then .candidate else .retained,start,3⟩
    source := src
    outer := SourceHull.hull (SourceHull.intervals src)
    availableAt := Retile.knownAt c
    cell := some c }

def tailPacket (time start : Nat) (xs : List ObservedUnit) : Packet :=
  { part := ⟨.active,start,xs.length⟩
    source := xs
    outer := SourceHull.hull (SourceHull.intervals xs)
    availableAt := max time (latest xs)
    cell := none }

def assemble (time start : Nat) : List ObservedUnit → List Packet
  | [] => []
  | [a] => [tailPacket time start [a]]
  | [a,b] => [tailPacket time start [a,b]]
  | a::b::c::rest => cellPacket time start ⟨a,b,c⟩ :: assemble time (start+3) rest

def recover (ps : List Packet) : List ObservedUnit := ps.flatMap Packet.source
def cells (ps : List Packet) : List Retile.Cell := ps.filterMap Packet.cell

def FIDELITY_TARGET : Prop := ∀ time start xs,
  recover (assemble time start xs) = xs ∧
  SourceHull.Cover start (start+xs.length) ((assemble time start xs).map Packet.part) ∧
  cells (assemble time start xs) = (Retile.retile time xs).1 ∧
  (∀ p ∈ assemble time start xs, p.part.len = p.source.length ∧ p.source ≠ [])

def PACKET_TARGET : Prop := ∀ time start xs p, p ∈ assemble time start xs →
  time ≤ p.availableAt ∧
  (∀ u ∈ p.source, u.1.knownAt ≤ p.availableAt) ∧
  (∃ h, p.outer = some h ∧ SourceHull.ExactHull (SourceHull.intervals p.source) h)

def ROLE_TARGET : Prop := ∀ time start t,
  ((cellPacket time start t).part.role = .candidate ↔ Retile.Local t) ∧
  ((cellPacket time start t).part.role = .retained ↔ ¬ Retile.Local t)

def weak := assemble 10 0 Retile.weakInput
def boundary := assemble 10 0 (CenterFrame.members LiftBoundary.f3)
def shortInput := Retile.weakInput.take 5
def short := assemble 10 0 shortInput

def WITNESS_TARGET : Prop :=
  Retile.WITNESS_TARGET ∧ LiftBoundary.WITNESS_TARGET ∧
  weak.map (fun p => p.part.role) = [.candidate,.candidate,.retained] ∧
  recover weak = Retile.weakInput ∧ weak.map Packet.availableAt = [10,10,10] ∧
  boundary.map (fun p => p.part.role) = [.candidate,.candidate,.candidate] ∧
  boundary.map Packet.outer = [some ⟨10040,10060⟩,some ⟨10030,10055⟩,some ⟨10030,10040⟩] ∧
  ¬ LiftBoundary.strictLift (cells boundary) ∧
  short.map (fun p => p.part.role) = [.candidate,.active] ∧
  short.map (fun p => p.part.len) = [3,2] ∧ recover short = shortInput

end RetilePacket
