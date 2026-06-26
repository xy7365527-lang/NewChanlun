/-
Origin/ChanlunElements.lean

This file gives the canonical interface for Chanlun elements.  It is deliberately
an interface: the repository implementations may instantiate it, but the
mathematical origin is a total deterministic element pipeline.
-/

import Origin.CompleteClassification

namespace NewChanlun.Origin

abbrev Tick := Int
abbrev Index := Nat

structure Bar where
  index : Index
  openPrice : Tick
  high : Tick
  low : Tick
  closePrice : Tick
deriving Repr

inductive FractalKind where
  | top
  | bottom
deriving DecidableEq, Repr

structure Fractal where
  kind : FractalKind
  index : Index
  price : Tick
deriving Repr

structure Stroke where
  direction : Direction
  startIndex : Index
  endIndex : Index
  startPrice : Tick
  endPrice : Tick
deriving Repr

structure Segment where
  direction : Direction
  startIndex : Index
  endIndex : Index
  startPrice : Tick
  endPrice : Tick
deriving Repr

structure Center where
  zd : Tick
  zg : Tick
  startIndex : Index
  endIndex : Index
  valid : zd <= zg
deriving Repr

inductive MoveKind where
  | consolidation
  | trendUp
  | trendDown
deriving DecidableEq, Repr

structure Move where
  kind : MoveKind
  startIndex : Index
  endIndex : Index
  centers : List Center
deriving Repr

inductive BspKind where
  | type1
  | type2
  | type3
deriving DecidableEq, Repr

structure Bsp where
  kind : BspKind
  side : Side
  index : Index
  price : Tick
deriving Repr

inductive OpenTail where
  | none
  | pendingFractal (bars : List Bar)
  | pendingStroke (fractals : List Fractal)
  | pendingSegment (strokes : List Stroke)
  | pendingMove (segments : List Segment)
deriving Repr

structure ParseStruct where
  mergedBars : List Bar
  fractals : List Fractal
  strokes : List Stroke
  segments : List Segment
  centers : List Center
  moves : List Move
  bsp : List Bsp
  tail : OpenTail
deriving Repr

structure ElementPipeline where
  mergeBars : List Bar -> List Bar
  fractalsOf : List Bar -> List Fractal
  strokesOf : List Fractal -> List Stroke
  segmentsOf : List Stroke -> List Segment
  centersOf : List Segment -> List Center
  movesOf : List Segment -> List Center -> List Move
  bspOf : List Move -> List Bsp
  tailOf : List Bar -> List Fractal -> List Stroke -> List Segment -> List Center -> List Move -> OpenTail

def ElementPipeline.parse (P : ElementPipeline) (bars : List Bar) : ParseStruct :=
  let mb := P.mergeBars bars
  let fs := P.fractalsOf mb
  let ss := P.strokesOf fs
  let segs := P.segmentsOf ss
  let centers := P.centersOf segs
  let moves := P.movesOf segs centers
  let bsp := P.bspOf moves
  let tail := P.tailOf mb fs ss segs centers moves
  { mergedBars := mb
    fractals := fs
    strokes := ss
    segments := segs
    centers := centers
    moves := moves
    bsp := bsp
    tail := tail }

theorem parse_total_unique (P : ElementPipeline) :
    TotalUnique (fun bars out => P.parse bars = out) :=
  total_unique_of_fun P.parse

theorem merge_total_unique (P : ElementPipeline) :
    TotalUnique (fun bars out => P.mergeBars bars = out) :=
  total_unique_of_fun P.mergeBars

theorem fractals_total_unique (P : ElementPipeline) :
    TotalUnique (fun bars out => P.fractalsOf bars = out) :=
  total_unique_of_fun P.fractalsOf

theorem strokes_total_unique (P : ElementPipeline) :
    TotalUnique (fun fs out => P.strokesOf fs = out) :=
  total_unique_of_fun P.strokesOf

theorem segments_total_unique (P : ElementPipeline) :
    TotalUnique (fun ss out => P.segmentsOf ss = out) :=
  total_unique_of_fun P.segmentsOf

theorem centers_total_unique (P : ElementPipeline) :
    TotalUnique (fun segs out => P.centersOf segs = out) :=
  total_unique_of_fun P.centersOf

theorem moves_total_unique (P : ElementPipeline) :
    forall segs centers,
      ExistsUnique (fun out => P.movesOf segs centers = out) := by
  intro segs centers
  exact total_unique_of_fun (fun pair : List Segment × List Center => P.movesOf pair.1 pair.2) (segs, centers)

theorem bsp_total_unique (P : ElementPipeline) :
    TotalUnique (fun moves out => P.bspOf moves = out) :=
  total_unique_of_fun P.bspOf

end NewChanlun.Origin
