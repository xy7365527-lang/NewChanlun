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

/--
  **走势（canonical，632号路1：加价格端点）** —— 一段走势的规范类型。

  ★632号路1 canonical 修复（codex 异质裁决，acceptance #2）：原 `Move` 只携带
  `kind/startIndex/endIndex/centers`，**不携带价格端点**——这把「同 kind/index/centers 但
  末端价不同的走势」折叠成同一类型值 = **行为等价类错误合并 = canonical 类型缺陷**
  （呼应完全分类 = 行为等价商 X_Θ/≡^beh：丢失末端价 ⟹ 商映射坍缩不同行为到同一格）。

  - `startPrice`：走势起点价（透传自构成该走势的 `Segment` 起点端点 `Segment.startPrice`）。
  - `endPrice`：走势末端价（透传自 `Segment.endPrice`）。走势末端是买卖点候选位置，其价格
    与中枢 [zd,zg] 的几何关系（之上/之下，608号 CenterStates）是 brokeCenter/leftCenter
    判据的 **L0 结构输入**——价格在 `Segment` 层（管线 strokesOf→segmentsOf）已算出，
    `movesOf` 透传而非丢弃（消除原 632号 canonical 契约缺陷）。

  ★认识论等级 L0：价格端点是 `Segment` 端点的结构透传（非经验数据），加字段不引入 Θ 参数。
-/
structure Move where
  kind : MoveKind
  startIndex : Index
  endIndex : Index
  startPrice : Tick
  endPrice : Tick
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

/--
  **元素管线接口（canonical）** —— 各字段是元素流水线的确定性阶段（接口，非实现）。

  ★632号路1 价格透传契约（codex 裁决）：`movesOf : List Segment → List Center → List Move`
  的任一合法实例**必须**从构成每个 `Move` 的 `Segment` 端点透传 `startPrice`/`endPrice`
  （`Move.startPrice := Segment.startPrice`，`Move.endPrice := Segment.endPrice`）——
  价格在 `segmentsOf` 层已算出，`movesOf` 透传而非丢弃。这是 `Move` 携带价格端点后的
  接口义务（消除原 canonical 契约缺陷）。签名不变（价格在 `Segment` 内），契约在 `Move` 字段层。
-/
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
