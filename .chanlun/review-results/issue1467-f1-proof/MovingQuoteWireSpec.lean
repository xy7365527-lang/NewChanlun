import MovingQuoteSpec

namespace MovingQuote

structure Output where
  located : Located
  geometry : Option NewChanlun.Origin.Segment

def construct (es : List Event) (quotes : List (Option Quote)) : Option (List Output) :=
  if quotes.length = es.length+1 then
    some ((locate 0 (group es)).map (fun l =>
      ⟨l,readout (positive l.block.first) l.start
        ((quotes.drop l.start).take ((events l.block).length+1))⟩))
  else none

def OUTPUT_TARGET : Prop := ∀ es quotes,
  (∃ out, construct es quotes = some out ∧
    out.map Output.located = locate 0 (group es) ∧
    decode (out.map (fun x => x.located.block)) = es) ↔ quotes.length = es.length+1

def quotes : List (Option Quote) := [some q0,some q1,some q2,some q3,some q4,none,some q6]
def WIRE_WITNESS_TARGET : Prop := ∃ out,
  construct es quotes = some out ∧ decode (out.map (fun x => x.located.block)) = es ∧
  out.map (fun x => x.geometry.isSome) = [true,true,true,false,false] ∧
  out.map (fun x => x.located.knownAt) = [some 2,some 3,some 4,some 6,none] ∧
  construct es [] = none

end MovingQuote
