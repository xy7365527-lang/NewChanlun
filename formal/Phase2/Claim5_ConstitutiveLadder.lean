/-
  缠论元素构成性阶梯·真完全分类（Phase 2, claim5 / task #28）
  工位：swarm/claim5-ladder（与 claim6-opsem/claim7-divergence/claim8-conservation 并发）
  认识论等级：L0（定义内蕴——构造子穷尽由各级缠论已结算定义钉死，对构造子结构归纳，
              不依赖数据；Lean build 通过 = 逻辑/管线正确，不是实证有效域，不得膨胀，
              formalization-validity-domain 规则）。

  ── 对象 ────────────────────────────────────────────────────────────────────
  构成性递归阶梯 = K线 → 分型(fenxing) → 笔(bi) → 线段(xianduan) → 走势(zoushi)
                  → 中枢(zhongshu)。每一级元素由 **低一级元素的有序窗口** 递归构造。
  这不是"特征枚举"，是 **构成性递归数据类型塔**（603 §二/§三的内涵式完全性对
  "缠论元素构成性"的应用）：每级元素一个 datatype，构造子来自低一级；每级完全分类
  = 该级构造子穷尽，对构造子结构归纳证明（machine-checked）。

  ── 同名异物澄清（任务卡硬要求，no-workaround 诚实标注）─────────────────────
  `docs/architecture/recursive_decomposition_tree.md` 是 **K4 资本分解树**
  （M/P/C/R 四顶点 → 行业 → 个股，卢麒元资本病理学，27 配置守恒律，M2 选股层），
  与本文件的 **缠论元素构成性阶梯**（K线→分型→笔→线段→走势→中枢）是 **同名异物**：
  - 缠论阶梯：纯形态学递归构造塔，每级元素由低一级元素的有序窗口构造（本文件对象）。
  - K4 资本树：资本流向的经济学分解树（顶点→成分→个股），在 σ 符号层做圈闭合表决。
  两者都叫"递归分解/构造树"但论域、构造子、判据完全不同，不得混淆。本文件只形式化
  **缠论元素构成性阶梯**。

  ── 隔离声明（任务卡：不碰 lakefile.toml / Formal.lean，integrator 统一 wire）──
  本模块 **自包含**（不 import Formal.*），由 integrator wire 时与 Phase 1 脊柱对接。
  对接点（命题层，非 import 层）：本阶梯第4级 `Segment` = Phase 1 `Move[0]`
  （level_recursion.md:21 `Move[0]=Segment`），第5级 `Move` 递归塔与 Phase 1
  `RecursiveConstruction.Move` 同构（走势分解定理二），第6级 `Center` 三态与 Phase 1
  `CenterTrichotomy` 同构（中心定理二）。见 §对接命题。

  ── 缠论定义依据（逐级溯源）────────────────────────────────────────────────
  - 第1级 K线/合并K线：baohan v1.3（包含处理，`hi ≥ lo` 不变量）。**K线层不分类**
    （formal_axioms §2 完全分类起于分型；K线无分类定理）。
  - 第2级 分型：fenxing v1.0 第62课 **双条件**（顶分型 mid.high 且 mid.low 三者最高，
    fenxing.md:92-110）；二构造子 {顶, 底} 穷尽。
  - 第3级 笔：bi v1.4 第62/65课 异性分型 + 间距（新笔 raw≥3）+ 方向有效性（顶高于底，
    bi.md:124）；方向二分 {上, 下} 穷尽。
  - 第4级 线段：xianduan v1.3。**口径分离（谱系 003）**：v0=三笔重叠骨架（参考实现，
    xianduan.md:145 口径A），v1=特征序列法（第67/71/78课，唯一正式口径，
    xianduan.md:160 口径B）。本文件形式化 **v0 reference ladder**（诚实标注，不冒充 v1）；
    方向二分 {上, 下} 穷尽。
  - 第5级 走势：zoushi 第17课走势三分法（上涨/下跌/盘整穷尽，zoushi.md:26）+ 走势分解
    定理二（≥3 段次级别构成，zoushi.md:28）。三构造子穷尽。
  - 第6级 中枢：zhongshu 第17/20课（至少三段次级别重叠，ZD/ZG/DD/GG）+ 中心定理一/二
    三态（延伸/新生/扩展，zhongshu.md:328 已结算）。本形式化把"新生"按方向裂为
    上涨延续/下跌延续，得 **四个判定结果**（extension + 中心定理二的 up/down/扩展），
    判定结果穷尽（codex 修正5：延伸独立，不被 else 吞）。
-/

namespace Formal.Phase2.ConstitutiveLadder

/-! ## 公共：有序窗口起点严格递增（构成性的时间顺序，沿 Phase 1 标准） -/

/--
  起点列表严格递增。构成性阶梯的层间窗口必须时间有序——
  关死"重复消费同一低级元素 / 乱序 / 重排"漏洞。
-/
def StrictlyIncreasing : List Nat → Prop
  | [] => True
  | [_] => True
  | a :: b :: rest => a < b ∧ StrictlyIncreasing (b :: rest)

/-! ## 第1级：K线 / 合并K线（baohan）——无完全分类，只保区间良构 -/

/--
  合并K线（MergedBar，baohan v1.3）：包含处理后的 K 线，`hi ≥ lo` 不变量。

  K线层 **不分类**（formal_axioms §2：完全分类起于分型，baohan 无 K线完全分类定理）。
  这是阶梯的最底层原子——其上第2级分型才开始有构造子穷尽。
-/
structure MergedBar where
  hi : Int
  lo : Int
  valid : lo ≤ hi

/-! ## 第2级：分型（fenxing）——二构造子完全分类 + 双条件 -/

/-- 分型类型（fenxing 第62课二构造子穷尽：顶 / 底）。 -/
inductive FractalKind where
  | top
  | bottom
deriving DecidableEq, Repr

/--
  分型：由三根连续合并K线（左/中/右）构成，带类型（构成性：来自第1级窗口）。

  ★codex 修正4（session 019eff21）：分型携带极值价 `ext`（顶分型=mid.high，底分型=mid.low）——
  这是第3级笔"顶高于底"价格有效性（bi 条件3，bi.md:124）所需的极值价载体。
  良构 `wellFormed` 双条件会强制 `ext` 与 mid 一致。
-/
structure Fractal where
  left : MergedBar
  mid : MergedBar
  right : MergedBar
  kind : FractalKind
  ext : Int   -- 极值价：顶分型=mid.hi，底分型=mid.lo（由 wellFormed 强制，bi 条件3 所需）

/--
  ★第2级完全分类（分型二构造子穷尽，L0）：任意分型类型必属 {顶, 底}。
  对 `FractalKind` 结构归纳——Lean 接受 = 构造子穷尽（无第三类分型）。
-/
theorem fractal_dichotomy (k : FractalKind) :
    k = FractalKind.top ∨ k = FractalKind.bottom := by
  cases k
  · exact Or.inl rfl
  · exact Or.inr rfl

/--
  分型构成性合法（fenxing **双条件**，第62课，fenxing.md:92-110）。

  - 顶分型：mid 的 high **且** low 都严格三者最高（不只 high）。
  - 底分型：mid 的 low **且** high 都严格三者最低（不只 low）。

  双条件是分型定义本身（fenxing.md:108"为什么是双条件"），非加强版——
  只满足 high 不满足 low（或反之）不构成分型。

  ★codex 修正4：良构强制极值价 `ext` 与 mid 一致（顶=mid.hi，底=mid.lo，fenxing.md:98/106），
  关死"自由携带与几何不符的极值价"漏洞。
-/
def Fractal.wellFormed (f : Fractal) : Prop :=
  match f.kind with
  | FractalKind.top =>
      f.mid.hi > f.left.hi ∧ f.mid.hi > f.right.hi
      ∧ f.mid.lo > f.left.lo ∧ f.mid.lo > f.right.lo
      ∧ f.ext = f.mid.hi
  | FractalKind.bottom =>
      f.mid.lo < f.left.lo ∧ f.mid.lo < f.right.lo
      ∧ f.mid.hi < f.left.hi ∧ f.mid.hi < f.right.hi
      ∧ f.ext = f.mid.lo

/-! ## 第3级：笔（bi）——方向二分完全分类 -/

/-- 笔方向（bi 第62课二构造子穷尽：上 / 下）。 -/
inductive StrokeDir where
  | up
  | down
deriving DecidableEq, Repr

/--
  笔：由两个相邻 **异性** 分型（起 / 止）构成，带方向。
  `confirmed` 时间性标志（最后一笔未确认，bi.md:151），**非第三态**——方向仍二分。
-/
structure Stroke where
  startFractal : Fractal
  endFractal : Fractal
  direction : StrokeDir
  confirmed : Bool

/--
  ★第3级完全分类（笔方向二构造子穷尽，L0）：任意笔方向必属 {上, 下}。
-/
theorem stroke_dichotomy (s : Stroke) :
    s.direction = StrokeDir.up ∨ s.direction = StrokeDir.down := by
  cases h : s.direction
  · exact Or.inl rfl
  · exact Or.inr rfl

/--
  笔构成性合法（bi 条件1 异性分型 + 条件3 价格有效性）。

  ★codex 修正4（session 019eff21）：纳入 bi **条件1（异性分型，bi.md:107）+ 条件3
  （方向有效性"顶高于底"，bi.md:124）**：
  - 上笔（底→顶）：终点顶分型极值价 > 起点底分型极值价（顶高于底）。
  - 下笔（顶→底）：终点底分型极值价 < 起点顶分型极值价（顶高于底）。
  条件3 用 `Fractal.ext` 极值价表达（由分型良构强制与 mid 一致）。

  ★codex 第二轮修正（递归闭包，session 019eff21）：笔良构 **递归要求两端分型良构**
  （`startFractal.wellFormed ∧ endFractal.wellFormed`）——关死"用 ill-formed 分型 +
  配合 kind/ext 伪造良构笔"漏洞。构成性阶梯是真正的递归闭包：上层良构 ⟹ 低层良构。

  ★诚实标注（no-workaround，formalization-validity-domain）：bi **条件2（间距：新笔
  raw≥3，bi.md:114）不在本结构层形式化范围**——间距需 raw K线位置（本结构按构成性阶梯
  只持相邻分型对象，不持 raw 索引），涉及新笔谱系 001。本 `wellFormed` 形式化的是
  **两端良构分型 + 异性 + 顶高于底 的笔骨架**（条件1+3 + 递归闭包），间距条件2 由实装
  引擎在 raw 序列上施加。明确声明覆盖条件1+3+递归闭包、不覆盖条件2。
-/
def Stroke.wellFormed (s : Stroke) : Prop :=
  s.startFractal.wellFormed ∧ s.endFractal.wellFormed
  ∧ (match s.direction with
    | StrokeDir.up =>
        s.startFractal.kind = FractalKind.bottom ∧ s.endFractal.kind = FractalKind.top
        ∧ s.endFractal.ext > s.startFractal.ext   -- 条件3：顶高于底
    | StrokeDir.down =>
        s.startFractal.kind = FractalKind.top ∧ s.endFractal.kind = FractalKind.bottom
        ∧ s.endFractal.ext < s.startFractal.ext)  -- 条件3：顶高于底

/-! ## 第4级：线段（xianduan = Move[0]）——★v0 reference ladder（非 v1 特征序列法） -/

/-- 线段方向（xianduan 二构造子穷尽：上 / 下；第77课方向与起始笔一致）。 -/
inductive SegmentDir where
  | up
  | down
deriving DecidableEq, Repr

/--
  线段（xianduan = Move[0]）。

  ★口径诚实标注（no-workaround + 谱系 003 线段概念分离）：
  本 `Segment` 的良构 `wellFormedV0 = strokes.length ≥ 3`（三笔重叠下界骨架，
  xianduan.md:145 口径A=v0 reference）。**不是** 正式 v1 口径（特征序列法第67课 +
  缺口两情形 + 结算锚 + 前三笔重叠 + 奇数性 + 顶高于底，xianduan.md:160 口径B）。
  v1 **不在本形式化范围**——本模块只声明"线段是 Move[0] 实体层级的 v0 骨架"，
  引用谱系 003。不冒充 v1。`confirmed` 时间性标志（非第三态）。
-/
structure Segment where
  strokes : List Stroke
  direction : SegmentDir
  confirmed : Bool

/-- ★第4级完全分类（线段方向二构造子穷尽，L0）：任意线段方向必属 {上, 下}。 -/
theorem segment_dichotomy (s : Segment) :
    s.direction = SegmentDir.up ∨ s.direction = SegmentDir.down := by
  cases h : s.direction
  · exact Or.inl rfl
  · exact Or.inr rfl

/-- 笔的价格区间 [lo, hi]（由两端分型极值价确定，方向无关取 min/max）。 -/
def Stroke.lo (s : Stroke) : Int := min s.startFractal.ext s.endFractal.ext
/-- 笔的价格区间上沿。 -/
def Stroke.hi (s : Stroke) : Int := max s.startFractal.ext s.endFractal.ext

/--
  ★线段 v0 骨架良构（codex 修正3：三笔重叠 + 方向由首笔决定，非自由携带）。

  xianduan.md:145 口径A（v0 reference）+ 第77课方向一致性 + 第62/65课"前三笔重叠"：
  - (a) `strokes.length ≥ 3`（至少三笔，xianduan.md:128）；
  - (b) `direction` 由 **首笔方向决定**（第77课"以向上笔开始的线段一定结束于向上笔"，
        方向不自由携带，codex 修正3）；
  - (c) **前三笔价格区间有公共重叠**（第65/77课"这三笔必须有重叠的部分"）：
        `max(三笔 lo) < min(三笔 hi)`（公共交集非空）；
  - (d) ★codex 第二轮修正（递归闭包）：**所有笔良构**（`∀ st ∈ strokes, st.wellFormed`）——
        关死"由坏笔组成的良构线段"漏洞。

  ★诚实标注（谱系 003）：这是 **v0 三笔重叠骨架**，**不是** v1 特征序列法
  （第67/71/78课，缺口两情形 + 结算锚 + 奇数性，xianduan.md:160 口径B，唯一正式口径）。
  v1 不在本形式化范围。本良构已比"仅 length≥3"忠实——方向锚定 + 前三笔重叠几何 + 递归闭包。
-/
def Segment.wellFormedV0 (s : Segment) : Prop :=
  s.strokes.length ≥ 3
  ∧ (∀ (h : 0 < s.strokes.length),
      (s.strokes[0].direction = StrokeDir.up ↔ s.direction = SegmentDir.up))
  ∧ (∀ (h0 : 0 < s.strokes.length) (h1 : 1 < s.strokes.length) (h2 : 2 < s.strokes.length),
      max s.strokes[0].lo (max s.strokes[1].lo s.strokes[2].lo)
        < min s.strokes[0].hi (min s.strokes[1].hi s.strokes[2].hi))
  ∧ (∀ st ∈ s.strokes, st.wellFormed)

/-! ## 第6级·辅助：中枢区间端点（zhongshu，第20课 ZD/ZG/DD/GG）

  中枢在第5级走势之前定义其端点结构，因为第5级走势的三态分类依赖中枢关系
  （zhongshu.md:328 三态穷尽是走势三分归纳步骤的关键）。
-/

/--
  中枢区间端点（zhongshu 第20课，zhongshu.md:53-60）。
  - `[zd, zg]` 核心区间：前两段定，`zd < zg` 成立条件（中枢核固定）。
  - `[dd, gg]` 外缘区间：全部段定（含全部波动）。约束 `dd ≤ zd ∧ zg ≤ gg`。
-/
structure Center where
  dd : Int
  zd : Int
  zg : Int
  gg : Int
  core_valid : zd < zg
  outer_lo : dd ≤ zd
  outer_hi : zg ≤ gg

/--
  两中枢关系四态（★codex 修正5：延伸独立，不用 else 吞，session 019eff21）。

  zhongshu.md:328 已结算"延伸 ∪ 新生 ∪ 扩展 = 全集"——其中"新生"按方向再分上涨/下跌。
  四态对应中心定理一（延伸）+ 中心定理二（新生 up/down + 扩展）：
  - `extension`（延伸，中心定理一 zhongshu.md:65）：后中枢核心 [zd,zg] 与前中枢核心重叠
    （围绕同一中枢震荡，[ZD,ZG] 不变）。
  - `upContinuation`（新生·上涨延续，中心定理二）：后 DD > 前 GG（外缘完全分离向上）。
  - `downContinuation`（新生·下跌延续，中心定理二）：后 GG < 前 DD（外缘完全分离向下）。
  - `levelExpansion`（扩展，中心定理二原公式）：核心分离但外缘重叠 → 升父级中枢。
-/
inductive CenterRelation where
  | extension          -- 延伸（中心定理一：核心重叠，围绕同一中枢震荡）
  | upContinuation     -- 新生·上涨延续（后 DD > 前 GG）
  | downContinuation   -- 新生·下跌延续（后 GG < 前 DD）
  | levelExpansion     -- 扩展（核心分离 + 外缘重叠 → 升父级中枢）
deriving DecidableEq, Repr

/--
  核心区间重叠谓词（中心定理一延伸的判据，zhongshu.md:65）：
  后中枢核心 [zd,zg] 与前中枢核心 [zd,zg] 有公共交集（弱不等式，zhongshu.md:298 延伸用 ≤/≥）。
-/
def CoresOverlap (prev next : Center) : Prop :=
  next.zd ≤ prev.zg ∧ prev.zd ≤ next.zg

/--
  从两中枢端点判定关系（★codex 修正5：四态忠实判定，延伸不被 else 吞）。

  判定顺序（中心定理一优先，再中心定理二，zhongshu.md:65/68）：
  - 核心重叠（`next.zd ≤ prev.zg ∧ prev.zd ≤ next.zg`）→ 延伸（中心定理一）；
  - 否则核心已分离，按外缘判中心定理二：
    - 后 DD > 前 GG → 上涨延续；后 GG < 前 DD → 下跌延续；
    - 否则（核心分离 + 外缘重叠）→ 级别扩张。
-/
def classifyCenter (prev next : Center) : CenterRelation :=
  if next.zd ≤ prev.zg ∧ prev.zd ≤ next.zg then CenterRelation.extension
  else if next.dd > prev.gg then CenterRelation.upContinuation
  else if next.gg < prev.dd then CenterRelation.downContinuation
  else CenterRelation.levelExpansion

/--
  ★第6级完全分类（中枢四态穷尽，L0）：任意两中枢关系必属
  {延伸, 上涨延续, 下跌延续, 扩展}。classifyCenter 全函数判定完备
  （延伸独立态 + 中心定理二三态，zhongshu.md:328 已结算，codex 修正5）。
-/
theorem center_trichotomy (prev next : Center) :
    classifyCenter prev next = CenterRelation.extension
    ∨ classifyCenter prev next = CenterRelation.upContinuation
    ∨ classifyCenter prev next = CenterRelation.downContinuation
    ∨ classifyCenter prev next = CenterRelation.levelExpansion := by
  unfold classifyCenter
  split
  · exact Or.inl rfl
  · split
    · exact Or.inr (Or.inl rfl)
    · split
      · exact Or.inr (Or.inr (Or.inl rfl))
      · exact Or.inr (Or.inr (Or.inr rfl))

/-- ★延伸判定忠实于核心重叠谓词（中心定理一，codex 修正5）。 -/
theorem classify_eq_extension (prev next : Center) :
    classifyCenter prev next = CenterRelation.extension ↔ CoresOverlap prev next := by
  unfold classifyCenter CoresOverlap
  by_cases h : next.zd ≤ prev.zg ∧ prev.zd ≤ next.zg
  · constructor
    · intro _; exact h
    · intro _; simp [h]
  · constructor
    · intro hc
      rw [if_neg h] at hc
      split at hc
      · exact absurd hc (by simp)
      · split at hc
        · exact absurd hc (by simp)
        · exact absurd hc (by simp)
    · intro hc; exact absurd hc h

/-! ## 第5级：走势（zoushi）——三构造子完全分类（递归塔顶，由中枢计算） -/

/-- 走势类型（zoushi 第17课走势三分法三构造子穷尽：上涨/下跌/盘整）。 -/
inductive TrendKind where
  | upTrend         -- 上涨趋势：≥2 个依次向上中枢
  | downTrend       -- 下跌趋势：≥2 个依次向下中枢
  | consolidation   -- 盘整：恰好 1 个中枢
deriving DecidableEq, Repr

/--
  ★第5级完全分类（走势三构造子穷尽，L0）：任意走势类型必属 {上涨, 下跌, 盘整}。
  对 `TrendKind` 结构归纳——Lean 接受 = 构造子穷尽（无第四走势构造子，
  走势分解定理一 zoushi.md:26）。
-/
theorem trend_trichotomy (t : TrendKind) :
    t = TrendKind.upTrend ∨ t = TrendKind.downTrend ∨ t = TrendKind.consolidation := by
  cases t
  · exact Or.inl rfl
  · exact Or.inr (Or.inl rfl)
  · exact Or.inr (Or.inr rfl)

/-! ## ★构成性阶梯的递归塔（每级由低一级的有序窗口构造）

  这是 claim5 的核心：构成性递归数据类型塔的 **层间构造性关系**——
  每级元素由低一级元素的 **有序窗口**（非凭空 membership）构造，关死
  "重复消费 / 乱序 / 非相邻" 漏洞。沿 Phase 1 ordered window witness 标准。
-/

/--
  ★第1→2级：分型由K线相邻三根窗口构成（fenxing：mid 是中心 bar）。

  分型由 `bars` 中 **相邻三根** `bars[i], bars[i+1], bars[i+2]` 构成——起点 `i` 在界内，
  left/mid/right 恰为该窗口，且合取 `wellFormed`（几何由窗口决定，构成性 ∧ 良构不解耦）。
  关死 membership 漏洞（三根同 bar / 非相邻 / 乱序都不满足）。
-/
def FractalFromBars (bars : List MergedBar) (f : Fractal) : Prop :=
  ∃ i, i + 2 < bars.length
    ∧ (∀ (h0 : i < bars.length) (h1 : i + 1 < bars.length) (h2 : i + 2 < bars.length),
        f.left = bars[i] ∧ f.mid = bars[i+1] ∧ f.right = bars[i+2])
    ∧ f.wellFormed

/--
  ★第2→3级：笔由相邻异性分型构成（bi 条件1）。

  笔两端分型是 `fractals` 中 **相邻** 的 `fractals[i], fractals[i+1]`（start 在前），
  且合取 `wellFormed`（异性方向）。关死 membership 漏洞（非相邻 / 乱序 / 同分型）。
-/
def StrokeFromFractals (fractals : List Fractal) (s : Stroke) : Prop :=
  ∃ i, i + 1 < fractals.length
    ∧ (∀ (h0 : i < fractals.length) (h1 : i + 1 < fractals.length),
        s.startFractal = fractals[i] ∧ s.endFractal = fractals[i+1])
    ∧ s.wellFormed

/--
  ★第3→4级：线段由连续覆盖笔窗口构成（xianduan v0 + 连续覆盖无空洞）。

  线段的笔是 `strokes` 中 **连续窗口** `[start, start+len)`（len ≥ 3，v0 骨架），
  且 `seg.strokes` 恰为该窗口切片（连续、无空洞、无重复消费）。

  ★codex 第二轮修正（递归闭包）：合取 `seg.wellFormedV0`——线段不仅来自连续窗口，
  还必须本身良构（方向锚定 + 前三笔重叠 + 所有笔良构）。关死"连续窗口但坏线段"漏洞。
-/
def SegmentFromStrokes (strokes : List Stroke) (seg : Segment) : Prop :=
  ∃ start len, len ≥ 3
    ∧ start + len ≤ strokes.length
    ∧ seg.strokes = (strokes.drop start).take len
    ∧ Segment.wellFormedV0 seg

/-! ## ★第4→5→6级：走势递归塔（Move[0]=Segment，向上无限 unfold）

  第5级走势是递归数据类型（初代数 μF）：`Move[0]=Segment`（归纳基底），
  `Move[k]` 由 `Move[k-1]` 的有序窗口（≥3 段）构造（走势分解定理二）。
  这是构成性阶梯的递归核心——级别 k 是递归层数（非时间周期，level_recursion.md）。
-/

/--
  ★走势递归数据类型（初代数 μF，走势分解定理二 zoushi.md:28 + level_recursion.md:21）。
  - `base`：归纳基底 `Move[0] = Segment`（线段层，level_recursion.md:21）。
  - `compose`：递归分支——`Move[k]` 由 `Move[k-1] list` + 中枢序列构造。

  ★codex 修正1（session 019eff21）：`compose` **不携带** 自由 `kind`——
  走势类型由 `centers` **计算**（`classifyMove`），关死"标成 upTrend 但只 1 中枢"的有效域膨胀。
-/
inductive Move where
  | base (seg : Segment)
  | compose (subs : List Move) (centers : List Center) (level : Nat)

namespace Move

/-- 递归层级（base=0，compose 显式携带）。 -/
def level : Move → Nat
  | base _ => 0
  | compose _ _ lvl => lvl

/--
  非空列表最小/最大（★codex 第四轮：用 head 初始化，不用 0 sentinel——
  0 不是 min/max 单位元，会污染全正/全负价格区间，伪造 dd/gg=0 的中枢）。
  空列表回退到 `dflt`（well-formed 路径下列表非空，sentinel 不触发，见下良构约束）。
-/
def minByHead (dflt : Int) : List Int → Int
  | [] => dflt
  | x :: xs => xs.foldl min x
def maxByHead (dflt : Int) : List Int → Int
  | [] => dflt
  | x :: xs => xs.foldl max x

/--
  走势价格区间下沿（segment=笔列表最小 lo；compose=中枢外缘 dd 最小）。
  ★codex 第三/四轮：中枢由子走势区间窗口重叠生成所需的 lo/hi 载体（对齐 Phase 1 Move.interval），
  用 head 初始化避免 0 sentinel 伪造区间。
-/
def lo : Move → Int
  | base seg => minByHead 0 (seg.strokes.map Stroke.lo)
  | compose _ centers _ => minByHead 0 (centers.map Center.dd)

/-- 走势价格区间上沿（segment=笔列表最大 hi；compose=中枢外缘 gg 最大）。 -/
def hi : Move → Int
  | base seg => maxByHead 0 (seg.strokes.map Stroke.hi)
  | compose _ centers _ => maxByHead 0 (centers.map Center.gg)

end Move

/--
  ★相邻中枢全链同向一致（趋势 = "依次同向"中枢链，对齐 Phase 1 allAdjacent）。
  全链都 `upContinuation`（或都 `downContinuation`）才是趋势，否则非全链一致。
-/
def allAdjacent (rel : CenterRelation) : List Center → Bool
  | [] => true
  | [_] => true
  | c0 :: c1 :: rest =>
      (decide (classifyCenter c0 c1 = rel)) && allAdjacent rel (c1 :: rest)

/--
  ★走势类型裁决（codex 修正1：由中枢序列计算，非自由携带；对齐 Phase 1 classifyMove）。
  - 恰 1 中枢 → 盘整（盘整定义，zoushi.md:41）。
  - ≥2 中枢全链上涨延续 → 上涨趋势；全链下跌延续 → 下跌趋势。
  - 0 中枢或 ≥2 非全链一致（含扩展/方向混合）→ none（升父级候选/退化，本级不裁为三类之一）。

  返回 `Option TrendKind`：`some` 时是三构造子之一（走势三分穷尽 by 第17课）；
  `none` 是"本级未裁决/升父级"（与 Phase 1 higherCenterCandidate 同构，非第四走势类型）。
-/
def classifyMove (centers : List Center) : Option TrendKind :=
  match centers with
  | [] => none
  | [_] => some TrendKind.consolidation
  | c0 :: c1 :: rest =>
      if allAdjacent CenterRelation.upContinuation (c0 :: c1 :: rest) then
        some TrendKind.upTrend
      else if allAdjacent CenterRelation.downContinuation (c0 :: c1 :: rest) then
        some TrendKind.downTrend
      else none

/-- 走势类型读出（base 线段无走势裁决，none；compose 由中枢计算）。 -/
def trendKind? : Move → Option TrendKind
  | Move.base _ => none
  | Move.compose _ centers _ => classifyMove centers

/-! ## ★第5→6级 witness：中枢由子走势重叠窗口生成（codex 第三轮修正，对齐 Phase 1） -/

/-- 逐对谓词（无 Mathlib Forall₂ 替代）：两 list 等长且逐位满足 R（多态）。 -/
def PairwiseRel {α β : Type} (R : α → β → Prop) : List α → List β → Prop
  | [], [] => True
  | a :: as, b :: bs => R a b ∧ PairwiseRel R as bs
  | _, _ => False

/--
  ★中枢在某起点 `i` 由子走势窗口生成（codex 第三轮：核心 ZD/ZG + 外缘 DD/GG 都 sound，
  对齐 Phase 1 `RecursiveConstruction.CenterDerivedAt`，zhongshu.md:53-60 第20课）。

  中枢 `c` 在起点 `i` 由 `subs` 生成 ⟺ 连续三段 `subs[i..i+2]` 的区间重叠产生 `c`：
  - 核心：`c.zd = max(三段 lo)`、`c.zg = min(三段 hi)`（zhongshu.md:60）；
  - 外缘：`c.dd = min(三段 lo)`、`c.gg = max(三段 hi)`（zhongshu.md:53-56）。
  外缘 dd/gg 驱动 up/down/扩展判定——不约束外缘则可伪造外缘翻转分类。
-/
def CenterDerivedAt (subs : List Move) (c : Center) (i : Nat) : Prop :=
  i + 2 < subs.length
    ∧ (∀ (h0 : i < subs.length) (h1 : i + 1 < subs.length) (h2 : i + 2 < subs.length),
        c.zd = max (subs[i].lo) (max (subs[i+1].lo) (subs[i+2].lo))
        ∧ c.zg = min (subs[i].hi) (min (subs[i+1].hi) (subs[i+2].hi))
        ∧ c.dd = min (subs[i].lo) (min (subs[i+1].lo) (subs[i+2].lo))
        ∧ c.gg = max (subs[i].hi) (max (subs[i+1].hi) (subs[i+2].hi)))

/--
  ★中枢序列由子走势生成（codex 第三轮：witness 起点列表 + 时间顺序，对齐 Phase 1
  `CentersDerivedFrom`）。关死"凭空填入 centers 伪造走势裁决"漏洞（codex 第三轮反例）。

  `CentersDerivedFrom subs centers` ⟺ 存在 witness 起点列表 `starts`：
  - 长度等于 centers；
  - 严格递增（中枢时间顺序——趋势"依次同向中枢"依赖此序，否则可重排 centers 翻转分类）；
  - 每个 center 在对应起点处核心+外缘 sound。
-/
def CentersDerivedFrom (subs : List Move) (centers : List Center) : Prop :=
  ∃ starts : List Nat,
    starts.length = centers.length
    ∧ StrictlyIncreasing starts
    ∧ PairwiseRel (fun c i => CenterDerivedAt subs c i) centers starts

/--
  ★走势递归良构（构成性 + 走势分解定理二 + 级别递减 + 中枢 witness，codex 修正1+2+第三轮）。
  - `base seg` 良构 ⟺ `seg.wellFormedV0`（★codex 修正2：线段基底须 v0 良构，
    关死"空线段当 Move[0]"漏洞，level_recursion.md:31 线段自足但仍须 ≥3 笔+重叠）。
  - `compose subs centers lvl` 良构 ⟺
      (a) `subs.length ≥ 3`（走势分解定理二，zoushi.md:28）；
      (b) `lvl ≥ 1`；
      (c) `∀ m ∈ subs, m.level = lvl - 1`（级别递减，构成性来自低一级）；
      (d) `centers.length ≥ 1`（完成走势必含中枢，原理二）；
      (e) ★`CentersDerivedFrom subs centers`（中枢由子走势重叠窗口生成，非凭空，
          codex 第三轮：关死伪造中枢漏洞——这是构成性阶梯"中枢由下级走势构造"的精确编码）；
      (f) `∀ m ∈ subs, WellFormed m`（递归良构）。
-/
def MoveWellFormed : Move → Prop
  | Move.base seg => Segment.wellFormedV0 seg
  | Move.compose subs centers lvl =>
      subs.length ≥ 3 ∧ lvl ≥ 1
      ∧ (∀ m ∈ subs, m.level = lvl - 1)
      ∧ centers.length ≥ 1
      ∧ CentersDerivedFrom subs centers
      ∧ (∀ m ∈ subs, MoveWellFormed m)

/--
  ★完成走势（SettledMove，codex 第二轮修正5：区分"结构良构"与"完成走势"）。

  一个 compose 走势是 **完成走势** ⟺ 良构 ∧ 裁出三类之一（`classifyMove centers = some _`）。

  **关键概念分离（no-workaround，非 workaround）**：`MoveWellFormed` 是 **结构良构**
  （递归构造正确），但良构 ≠ 完成走势。当 `centers` 含扩展（levelExpansion）或方向混合时，
  `classifyMove = none`——这 **不是** 走势三分的第四类，而是缠论本体的"本级走势终结、
  升父级中枢重分类"（与 Phase 1 `RecursiveConstruction.higherCenterCandidate` 同构、
  603 §三）。**完成走势必裁出三类之一**正是缠论"完成走势三分"的精确边界：扩展态本就
  不是本级完成走势类型，是升父级信号。把扩展态算作"良构完成走势的第四类"才是膨胀；
  本定义诚实地把它排除在完成走势之外（none 出口）。
-/
def SettledMove (m : Move) : Prop :=
  MoveWellFormed m ∧ ∃ t, trendKind? m = some t

/-! ## ★阶梯每层完全分类（构造子穷尽，对构造子结构归纳，L0） -/

/--
  ★构成性阶梯·每层真完全分类（L0）：阶梯每一可分类层的构造子穷尽。

  - 第2级 分型：二构造子 {顶, 底} 穷尽。
  - 第3级 笔：方向二分 {上, 下} 穷尽。
  - 第4级 线段：方向二分 {上, 下} 穷尽。
  - 第5级 走势：三构造子 {上涨, 下跌, 盘整} 穷尽。
  - 第6级 中枢：四态 {延伸, 上涨延续, 下跌延续, 扩展} 穷尽（两中枢关系，codex 修正5）。
  （第1级 K线无分类——完全分类起于分型，formal_axioms §2。）

  各层穷尽由各层缠论已结算定义钉死，对构造子结构归纳证明（machine-checked）。
-/
theorem ladder_each_layer_complete
    (k : FractalKind) (s : Stroke) (seg : Segment) (t : TrendKind)
    (cprev cnext : Center) :
    (k = FractalKind.top ∨ k = FractalKind.bottom)
    ∧ (s.direction = StrokeDir.up ∨ s.direction = StrokeDir.down)
    ∧ (seg.direction = SegmentDir.up ∨ seg.direction = SegmentDir.down)
    ∧ (t = TrendKind.upTrend ∨ t = TrendKind.downTrend ∨ t = TrendKind.consolidation)
    ∧ (classifyCenter cprev cnext = CenterRelation.extension
        ∨ classifyCenter cprev cnext = CenterRelation.upContinuation
        ∨ classifyCenter cprev cnext = CenterRelation.downContinuation
        ∨ classifyCenter cprev cnext = CenterRelation.levelExpansion) :=
  ⟨fractal_dichotomy k, stroke_dichotomy s, segment_dichotomy seg,
   trend_trichotomy t, center_trichotomy cprev cnext⟩

/-! ## ★层间桥接定理（构成性 ⟹ 良构，两谓词不解耦） -/

/-- ★桥接：由K线窗口构成的分型必几何良构（双条件）。 -/
theorem fractal_from_bars_wellformed (bars : List MergedBar) (f : Fractal)
    (h : FractalFromBars bars f) : f.wellFormed := by
  obtain ⟨_, _, _, hwf⟩ := h
  exact hwf

/-- ★桥接：由相邻分型构成的笔必方向良构（异性分型）。 -/
theorem stroke_from_fractals_wellformed (fractals : List Fractal) (s : Stroke)
    (h : StrokeFromFractals fractals s) : s.wellFormed := by
  obtain ⟨_, _, _, hwf⟩ := h
  exact hwf

/--
  ★桥接：线段笔由连续窗口切片（非任意 membership），关死重复消费/乱序/空洞。
  线段的笔是 strokes 的连续片段 `[start, start+len)`，len ≥ 3，且线段良构（递归闭包）。
-/
theorem segment_from_strokes_contiguous (strokes : List Stroke) (seg : Segment)
    (h : SegmentFromStrokes strokes seg) :
    ∃ start len, len ≥ 3 ∧ seg.strokes = (strokes.drop start).take len
      ∧ Segment.wellFormedV0 seg := by
  obtain ⟨start, len, hlen, _, hslice, hwf⟩ := h
  exact ⟨start, len, hlen, hslice, hwf⟩

/-! ## ★与 Phase 1 脊柱的对接命题（integrator wire 点，命题层非 import 层） -/

/--
  ★对接1：线段 = Move[0]（归纳基底，level_recursion.md:21）。
  本阶梯第5级走势递归塔的 `base seg` 层级为 0——即 `Move[0]=Segment`。
  这是构成性阶梯（第4级线段）与走势递归（Phase 1 `RecursiveConstruction.Move`
  的 `segment` 基底）的衔接：线段是走势递归的归纳基底层级。
-/
theorem segment_is_move_base (seg : Segment) :
    (Move.base seg).level = 0 := rfl

/--
  ★对接2：base 走势良构 ⟺ 线段 v0 良构（★codex 修正2：线段基底须 v0 良构）。
  归纳基底 `Move[0]=Segment` 的良构 **就是** 线段的 v0 良构（≥3 笔 + 方向锚定 + 前三笔重叠）——
  不依赖中枢（level_recursion.md:31 线段自足，打破"中枢需要走势、走势需要中枢"循环），
  但 **必须** 是良构线段，不是任意 `Segment`（关死空线段当 Move[0]）。
-/
theorem base_move_wellformed (seg : Segment) (h : Segment.wellFormedV0 seg) :
    MoveWellFormed (Move.base seg) := by
  unfold MoveWellFormed
  exact h

/--
  ★对接3：良构 compose 走势 ≥3 段次级别构成（走势分解定理二，与 Phase 1 同构）。
  本阶梯第5级走势递归塔的 compose 良构强制 `subs.length ≥ 3`——
  与 Phase 1 `RecursiveConstruction.WellFormed` 的 `subs.length ≥ 3` 同构。
-/
theorem compose_move_at_least_three (subs : List Move) (centers : List Center)
    (lvl : Nat)
    (h : MoveWellFormed (Move.compose subs centers lvl)) :
    subs.length ≥ 3 := by
  unfold MoveWellFormed at h
  exact h.1

/--
  ★对接4：良构 compose 走势级别递减（构成性来自低一级，与 Phase 1 同构）。
  良构 compose 的每个子走势级别 = 父级 -1——构成性阶梯"每级由低一级构造"的精确编码。
-/
theorem compose_move_subs_one_lower (subs : List Move) (centers : List Center)
    (lvl : Nat)
    (h : MoveWellFormed (Move.compose subs centers lvl)) :
    ∀ m ∈ subs, m.level = lvl - 1 := by
  unfold MoveWellFormed at h
  exact h.2.2.1

/--
  ★对接4b：良构 compose 的中枢由子走势生成（★codex 第三轮：CentersDerivedFrom，非凭空）。
  良构 compose 满足 `CentersDerivedFrom subs centers`——中枢序列由 subs 重叠窗口生成。
-/
theorem compose_centers_derived (subs : List Move) (centers : List Center) (lvl : Nat)
    (h : MoveWellFormed (Move.compose subs centers lvl)) :
    CentersDerivedFrom subs centers := by
  unfold MoveWellFormed at h
  exact h.2.2.2.2.1

/--
  ★伪造中枢无 witness 窗口（★codex 第三轮：CenterDerivedAt 真约束，非恒真，对齐 Phase 1
  `no_window_when_too_short`）。

  子走势不足 3 段时（subs.length < 3），任何中枢都 **无** 生成窗口——
  `CenterDerivedAt` 的 `i + 2 < subs.length` 无解。这证明 witness 关系 **真的会拒绝**
  伪造中枢，不是恒真占位（可证伪性见证：关死 codex 第三轮"凭空 centers"漏洞）。
-/
theorem no_center_witness_when_too_short (subs : List Move) (c : Center) (i : Nat)
    (h : subs.length < 3) : ¬ CenterDerivedAt subs c i := by
  unfold CenterDerivedAt
  intro ⟨hi, _⟩
  omega

/--
  ★区间 sentinel 不触发（★codex 第四轮：well-formed Move 的 lo/hi 由真实数据计算，非 0 sentinel）。

  `minByHead/maxByHead` 的 `0` sentinel **只在空列表触发**；良构 `base seg` 强制
  `seg.strokes.length ≥ 3`（非空），良构 `compose` 强制 `centers.length ≥ 1`（非空）——
  故良构 Move 的 `lo/hi` 走 `x :: xs` 分支（head 初始化），sentinel `0` 永不污染区间。
  本定理见证 base 路径：良构 base 的笔列表非空（codex 第四轮 0-sentinel 漏洞关死）。
-/
theorem wellformed_base_strokes_nonempty (seg : Segment)
    (h : MoveWellFormed (Move.base seg)) : seg.strokes ≠ [] := by
  unfold MoveWellFormed Segment.wellFormedV0 at h
  intro hempty
  rw [hempty] at h
  simp at h

/-- ★良构 compose 的中枢列表非空（区间 sentinel 不触发的 compose 路径见证，codex 第四轮）。 -/
theorem wellformed_compose_centers_nonempty (subs : List Move) (centers : List Center)
    (lvl : Nat) (h : MoveWellFormed (Move.compose subs centers lvl)) : centers ≠ [] := by
  unfold MoveWellFormed at h
  intro hempty
  rw [hempty] at h
  simp at h

/--
  ★对接5：走势裁决由中枢计算（★codex 修正1：kind 不自由携带）。
  compose 走势的 `trendKind?` 完全由 `centers` 决定——不存在"自由携带与中枢不一致的裁决"。
  这是 codex 修正1 的形式化见证：`Move.compose` 删除了自由 `kind` 字段。
-/
theorem trend_kind_determined_by_centers (subs : List Move) (centers : List Center)
    (lvl : Nat) :
    trendKind? (Move.compose subs centers lvl) = classifyMove centers := rfl

/--
  ★对接6：单中枢 compose ⟹ 盘整（codex 修正1：恰 1 中枢裁为盘整，非自由标 upTrend）。
  这关死 codex 反例 `compose [...] [c] upTrend 1`——单中枢只能裁为 consolidation。
-/
theorem one_center_is_consolidation (subs : List Move) (c : Center) (lvl : Nat) :
    trendKind? (Move.compose subs [c] lvl) = some TrendKind.consolidation := rfl

/--
  ★对接7·完成走势三分完全性（★codex 第二轮修正5：well-formed 走势对象穷尽，非仅标签穷尽）。

  **任意完成走势（SettledMove）的走势类型必属三构造子之一 {上涨, 下跌, 盘整}。**
  这不再是"TrendKind 标签穷尽"（trend_trichotomy），而是"well-formed 完成走势对象穷尽"——
  完成走势 by 定义裁出 `some t`，而 t 必属三类（走势分解定理一 zoushi.md:26）。
  含扩展/未完成的良构 compose 裁决为 none（升父级候选，非第四走势类型），被 SettledMove
  诚实排除——这正是缠论"完成走势必属三类"的精确边界（codex 修正5 关死 none 出口膨胀）。
-/
theorem settled_move_trichotomy (m : Move) (h : SettledMove m) :
    ∃ t, trendKind? m = some t
      ∧ (t = TrendKind.upTrend ∨ t = TrendKind.downTrend ∨ t = TrendKind.consolidation) := by
  obtain ⟨_, t, ht⟩ := h
  exact ⟨t, ht, trend_trichotomy t⟩

/--
  ★完成走势诚实边界（codex 修正5）：非完成走势（trendKind? = none）的良构 compose 存在，
  它 **不是** 走势三分的反例，而是"本级终结、升父级"信号。本定理见证：良构 + none ⟹ ¬SettledMove
  （none 出口被诚实排除在完成走势之外，非膨胀为第四类）。
-/
theorem none_outcome_not_settled (m : Move) (hnone : trendKind? m = none) :
    ¬ SettledMove m := by
  intro ⟨_, t, ht⟩
  rw [hnone] at ht
  exact absurd ht (by simp)

/-! ## ★实时未完成实例不破坏完全分类（603 §三 / codex Q1，诚实边界） -/

/--
  ★实时未完成走势（Candidate，603 §三）：实时数据中最高级别走势总不完整。
  `settled : Bool` 包装——未结算走势仍是同一 `Move` 的"未结算实例"，**非新构造子**，
  不破坏走势三分完全分类（formalization-validity-domain：构造层完全性 vs 实时未结算实例）。

  ★codex 修正1：candidate 的走势类型也 **不自由携带**——由 `inner` 计算 `trendKind?`。
-/
structure CandidateMove where
  inner : Move
  settled : Bool

/-- candidate 的走势裁决（由 inner 计算，非自由携带；`outcome?` 避免与 Move.trendKind? 同名）。 -/
def CandidateMove.outcome? (c : CandidateMove) : Option TrendKind :=
  trendKind? c.inner

/--
  ★未结算实例的裁决若产出走势类型，必属三构造子之一（完全分类不被实时未完成破坏，L0）。

  candidate 的裁决是 `Option TrendKind`：`none` = 未裁决/升父级（实时未完成的诚实表达，
  非第四走势类型）；`some t` 时 `t` 必属 {上涨, 下跌, 盘整}（走势三分穷尽）。
  前提 `_h` 见证"裁出 t"，结论由走势三分穷尽给出（与 t 是否来自 candidate 无关——
  这正是完全分类的强度：任何 TrendKind 实例都属三类）。
-/
theorem candidate_preserves_trichotomy (c : CandidateMove) (t : TrendKind)
    (_h : c.outcome? = some t) :
    t = TrendKind.upTrend ∨ t = TrendKind.downTrend ∨ t = TrendKind.consolidation :=
  trend_trichotomy t

/-! ## ★阶梯递归无穷回归关死（603 核心：构造子穷尽 ⟹ 无第N+1级待发现轴）

  构成性阶梯是 **有限级数据类型塔 + 走势级别递归**：阶梯的6个构成性级别
  （K线/分型/笔/线段/走势/中枢）是固定的元素层级（缠论定义钉死），不是"待发现的轴序列"；
  走势级别 k 向上递归（Move[k] 由 Move[k-1] 构造）由数据自然终止（settled moves<3，
  level_recursion.md:249）。两种递归都不产生"待发现的第N+1级独立维度"——
  这是 603 内涵式完全性（构造子穷尽）对构成性阶梯的应用：阶梯关死无穷回归。
-/

/--
  ★阶梯级别有限性见证：构成性阶梯恰6级，第i级（i≥2）的完全分类由第i级构造子穷尽给出，
  不依赖"发现第i+1级新轴"。本定理把"每层都有完全分类"打包——见证阶梯的完全性是
  **逐级构造子穷尽的合取**，非"轴集合是否齐全"的经验命题。
-/
theorem ladder_complete_no_axis_discovery
    (k : FractalKind) (s : Stroke) (seg : Segment) (t : TrendKind)
    (cprev cnext : Center) :
    -- 第2级分型完全 ∧ 第3级笔完全 ∧ 第4级线段完全 ∧ 第5级走势完全 ∧ 第6级中枢完全
    (k = FractalKind.top ∨ k = FractalKind.bottom)
    ∧ (s.direction = StrokeDir.up ∨ s.direction = StrokeDir.down)
    ∧ (seg.direction = SegmentDir.up ∨ seg.direction = SegmentDir.down)
    ∧ (t = TrendKind.upTrend ∨ t = TrendKind.downTrend ∨ t = TrendKind.consolidation)
    ∧ (classifyCenter cprev cnext = CenterRelation.extension
        ∨ classifyCenter cprev cnext = CenterRelation.upContinuation
        ∨ classifyCenter cprev cnext = CenterRelation.downContinuation
        ∨ classifyCenter cprev cnext = CenterRelation.levelExpansion) :=
  ladder_each_layer_complete k s seg t cprev cnext

end Formal.Phase2.ConstitutiveLadder
