/-
  缠论元素构成性阶梯完全分类（Phase 2 claim5）
  ★codex + 异工位双复核修正版（Phase 2，有效域膨胀全部吸收，no-workaround：诚实标注口径分离）

  K线 → 分型 → 笔 → 线段 → 走势 → 中枢 = 递归数据类型链（构成性递归完全分类）。

  范式（沿 Phase 1 ordered window witness 标准）：构成性阶梯不是"特征枚举"，是 **构成性递归
  数据类型链**——每层由下层 **有序窗口** 构造（非凭空 membership），每层分类是该层 inductive
  构造子穷尽。这是 603 元级洞察对"缠论元素构成性"的应用。

  ★codex + 异工位双复核修正（有效域膨胀，非定义冲突）：
  - 复核1（fenxing 双条件）：分型是 **双条件**（顶分型 high 最高 **且** low 最高；fenxing.md:92）——
    原 wellFormed 只编码单条件，膨胀。已补双条件。
  - 复核2（ordered window）：层间关系不能是 membership——分型=相邻3根 K 窗口、笔=相邻异性分型、
    线段=连续覆盖笔（formal_axioms I3）。已升级为 ordered window witness（沿 Phase 1 标准）。
  - 复核3（wellFormed↔DerivedFrom 桥接）：原两谓词解耦。已加桥接命题。
  - 复核4（线段口径分离，谱系 003）：`Segment.wellFormed = length≥3` 是**被降级的 v0 口径**，
    **不是** 正式 v1（特征序列法第67课）。原注释称"特征序列法"但实现 v0 = 口径倒置。
    **诚实修正**（no-workaround，概念分离领域）：本模块明确标注为 **v0 reference ladder**
    （三笔重叠下界骨架），v1 特征序列法 **不在本形式化范围**，引用谱系 003。不冒充 v1。

  缠论定义依据：baohan/fenxing/bi/xianduan + formal_axioms §1-2 + level_recursion（Move[0]=Segment）。
  认识论：L0（定义内蕴）。有效域 = 各层构造子穷尽 + 有序窗口构成性 + **线段 v0 骨架**（非 v1）。
-/

import Formal.TrendTrichotomy

namespace Formal.ConstitutiveLadder

open Formal.TrendTrichotomy (Direction)

/-- 有序窗口起点严格递增（沿 Phase 1 标准，构成性的时间顺序）。 -/
def StrictlyIncreasing : List Nat → Prop
  | [] => True
  | [_] => True
  | a :: b :: rest => a < b ∧ StrictlyIncreasing (b :: rest)

/-! ## 第1层：K线包含处理（baohan，MergedBar）——无完全分类，只保区间良构 -/

/--
  合并 K 线（MergedBar，baohan v1.3）。`hi ≥ lo` 不变量。
  K 线层 **不分类**（formal_axioms §2 完全分类起于 Fractal，baohan 无 K 线完全分类）。
-/
structure MergedBar where
  hi : Int
  lo : Int
  valid : lo ≤ hi

/-! ## 第2层：分型（fenxing，Fractal）——二构造子完全分类 + 双条件 -/

/-- ★分型完全分类（fenxing，formal_axioms §2.1：`Fractal.kind ∈ {top, bottom}`，二构造子）。 -/
inductive FractalKind where
  | top
  | bottom
deriving DecidableEq, Repr

/-- 分型：由三根连续合并 K 线构成，带类型。 -/
structure Fractal where
  left : MergedBar
  mid : MergedBar
  right : MergedBar
  kind : FractalKind

/-- ★分型完全分类（二构造子穷尽，L0）。 -/
theorem fractal_dichotomy (k : FractalKind) : k = FractalKind.top ∨ k = FractalKind.bottom := by
  cases k
  · exact Or.inl rfl
  · exact Or.inr rfl

/--
  ★分型构成性合法（fenxing **双条件**，复核1 修正，fenxing.md:92）。

  - 顶分型：mid 的 high **且** low 都三者最高（不只 high）。
  - 底分型：mid 的 low **且** high 都三者最低（不只 low）。

  双条件是 fenxing 定义本身（fenxing.md:108"为什么是双条件"），非加强版。
  原单条件实现接受了缠论拒绝的对象（有效域膨胀），已修。
-/
def Fractal.wellFormed (f : Fractal) : Prop :=
  match f.kind with
  | FractalKind.top =>
      f.mid.hi > f.left.hi ∧ f.mid.hi > f.right.hi
      ∧ f.mid.lo > f.left.lo ∧ f.mid.lo > f.right.lo
  | FractalKind.bottom =>
      f.mid.lo < f.left.lo ∧ f.mid.lo < f.right.lo
      ∧ f.mid.hi < f.left.hi ∧ f.mid.hi < f.right.hi

/-! ## 第3层：笔（bi，Stroke）——方向二分完全分类 -/

/--
  ★笔完全分类（bi，formal_axioms §2.2：`Stroke.direction ∈ {up, down}`，方向二分）。
  `confirmed` 时间性标志（最后一笔未确认），非第三态（formal_axioms §2.2 注）。
-/
structure Stroke where
  startFractal : Fractal
  endFractal : Fractal
  direction : Direction
  confirmed : Bool

/--
  笔构成性合法（bi 条件1 异性分型）：上笔起底止顶，下笔起顶止底。

  ★诚实标注（复核2）：本 wellFormed 只编码 bi **条件1（异性分型方向）**。
  bi 条件2（间距：新笔 raw≥3/旧笔 merged gap≥4）+ 条件3（方向有效性"顶高于底"，bi.md:124）
  **不在本层形式化范围**（条件3 需 Fractal 携带极值价，本结构未持；涉及新笔谱系 001）。
-/
def Stroke.wellFormed (s : Stroke) : Prop :=
  match s.direction with
  | Direction.up => s.startFractal.kind = FractalKind.bottom ∧ s.endFractal.kind = FractalKind.top
  | Direction.down => s.startFractal.kind = FractalKind.top ∧ s.endFractal.kind = FractalKind.bottom

/-- ★笔方向完全分类（二构造子穷尽，L0）。 -/
theorem stroke_dichotomy (s : Stroke) : s.direction = Direction.up ∨ s.direction = Direction.down := by
  cases h : s.direction
  · exact Or.inl rfl
  · exact Or.inr rfl

/-! ## 第4层：线段（xianduan，Segment = Move[0]）——★v0 reference ladder（非 v1 特征序列法） -/

/--
  ★线段（xianduan，formal_axioms §2.3：`Segment.direction ∈ {up, down}`，方向二分）。

  ★口径诚实标注（复核4 + no-workaround，谱系 003 线段概念分离）：
  本 `Segment` 的 `wellFormed = strokes.length ≥ 3` 是 **v0 reference ladder 口径**
  （三笔重叠下界骨架，xianduan.md 口径A）。**不是** 正式 v1 口径（特征序列法第67课 +
  缺口两情形 + 结算锚 + 前三笔重叠 + 奇数性 + 顶高于底）。v1 **不在本形式化范围**。
  本模块只声明"线段是 Move[0] 实体层级的 v0 骨架"，引用谱系 003-segment-concept-separation。
  `confirmed` 时间性标志（非第三态）。
-/
structure Segment where
  strokes : List Stroke
  direction : Direction
  confirmed : Bool

/-- ★线段 v0 骨架良构（三笔重叠下界，非 v1 特征序列法，诚实标注口径）。 -/
def Segment.wellFormedV0 (s : Segment) : Prop := s.strokes.length ≥ 3

/-- ★线段方向完全分类（二构造子穷尽，L0）。 -/
theorem segment_dichotomy (s : Segment) : s.direction = Direction.up ∨ s.direction = Direction.down := by
  cases h : s.direction
  · exact Or.inl rfl
  · exact Or.inr rfl

/-! ## 阶梯层间构成性关系（★ordered window witness，非 membership，复核2 修正） -/

/--
  ★分型由 K 线相邻三根窗口构成（复核2：ordered window，非 membership，fenxing 定义）。

  分型由 `bars` 中 **相邻三根** `bars[i], bars[i+1], bars[i+2]` 构成（mid 是中心 bar）——
  起点 `i` 在界内，且 left/mid/right 恰为该窗口。这关死 membership 漏洞
  （三根同 bar / 非相邻 / 乱序都不满足），且合取 `wellFormed`（几何由窗口决定）。
-/
def FractalFromWindow (bars : List MergedBar) (f : Fractal) : Prop :=
  ∃ i, i + 2 < bars.length
    ∧ (∀ (h0 : i < bars.length) (h1 : i + 1 < bars.length) (h2 : i + 2 < bars.length),
        f.left = bars[i] ∧ f.mid = bars[i+1] ∧ f.right = bars[i+2])
    ∧ f.wellFormed   -- ★桥接（复核3）：构成性 ∧ 几何良构合取，不解耦

/--
  ★笔由相邻异性分型构成（复核2：ordered window + 桥接 wellFormed）。

  笔的两端分型是 `fractals` 中 **相邻** 的 `fractals[i], fractals[i+1]`（顺序：start 在前），
  且合取 `wellFormed`（异性方向）。关死 membership 漏洞（非相邻/乱序/同分型）。
-/
def StrokeFromAdjacentFractals (fractals : List Fractal) (s : Stroke) : Prop :=
  ∃ i, i + 1 < fractals.length
    ∧ (∀ (h0 : i < fractals.length) (h1 : i + 1 < fractals.length),
        s.startFractal = fractals[i] ∧ s.endFractal = fractals[i+1])
    ∧ s.wellFormed   -- ★桥接：构成性 ∧ 方向良构合取

/--
  ★线段由连续覆盖笔窗口构成（复核2 + formal_axioms I3：连续覆盖无空洞）。

  线段的笔是 `strokes` 中 **连续窗口** `[start, start+len)`（len ≥ 3，v0 骨架），
  且 `s.strokes` 恰为该窗口切片（连续、无空洞、无重复消费，formal_axioms I3 覆盖）。
-/
def SegmentFromStrokeWindow (strokes : List Stroke) (seg : Segment) : Prop :=
  ∃ start len, len ≥ 3
    ∧ start + len ≤ strokes.length
    ∧ seg.strokes = (strokes.drop start).take len

/-! ## 构成性阶梯完全分类 + 桥接定理 -/

/--
  ★构成性阶梯真完全分类（L0）：阶梯每层分类是该层 inductive 构造子穷尽。

  分型二分 / 笔二分 / 线段二分——各层构造子穷尽由各层缠论定义钉死，结构归纳证明。
  （走势/中枢层已在 RecursiveConstruction/CenterTrichotomy 形式化。）
-/
theorem ladder_each_layer_complete (f : FractalKind) (s : Stroke) (seg : Segment) :
    (f = FractalKind.top ∨ f = FractalKind.bottom)
    ∧ (s.direction = Direction.up ∨ s.direction = Direction.down)
    ∧ (seg.direction = Direction.up ∨ seg.direction = Direction.down) :=
  ⟨fractal_dichotomy f, stroke_dichotomy s, segment_dichotomy seg⟩

/--
  ★桥接定理（复核3）：由 K 线窗口构成的分型必几何良构。
  `FractalFromWindow` 合取了 `wellFormed`——构成性（来源窗口）⟹ 几何良构，两谓词不再解耦。
-/
theorem fractal_window_wellformed (bars : List MergedBar) (f : Fractal)
    (h : FractalFromWindow bars f) : f.wellFormed := by
  obtain ⟨_, _, _, hwf⟩ := h
  exact hwf

/-- ★桥接定理：由相邻分型构成的笔必方向良构。 -/
theorem stroke_adjacent_wellformed (fractals : List Fractal) (s : Stroke)
    (h : StrokeFromAdjacentFractals fractals s) : s.wellFormed := by
  obtain ⟨_, _, _, hwf⟩ := h
  exact hwf

/--
  ★线段窗口连续无重复消费（复核2 + I3）：线段笔由连续窗口切片，非任意 membership。
  这关死"重复消费同一笔/乱序/空洞"——线段笔是 strokes 的连续片段。
-/
theorem segment_window_contiguous (strokes : List Stroke) (seg : Segment)
    (h : SegmentFromStrokeWindow strokes seg) :
    ∃ start len, len ≥ 3 ∧ seg.strokes = (strokes.drop start).take len := by
  obtain ⟨start, len, hlen, _, hslice⟩ := h
  exact ⟨start, len, hlen, hslice⟩

/--
  ★阶梯衔接 Phase 1：线段（v0 骨架）= Move[0] 实体层级。
  线段是走势递归（RecursiveConstruction.Move）的归纳基底层级——构成性阶梯在线段层与
  Move[0] 衔接，之上是 Move（走势三分）→ Center（中枢三态）。**诚实标注**：本衔接是
  "实体层级对应"（Segment 是 Move[0] 所在层级），v0 骨架口径，非 v1 特征序列法构造。
-/
theorem segment_is_move_base_level (s : Segment) (h : Segment.wellFormedV0 s) :
    s.strokes.length ≥ 3 := h

end Formal.ConstitutiveLadder
