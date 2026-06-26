/-
  线段划分 v1 特征序列法·真完全分类（Phase 2, claim10 / task #37）
  工位：claim10-segment-v1（与 claim5/6/9 并发，integrator=formalization-lead）

  认识论等级：L0（定义内蕴——v1 划分情形的构造子穷尽由缠师第67课"只有两种可能"
              这条已结算定理钉死，对构造子结构归纳，不依赖数据；`lake build` 通过
              = 逻辑/管线正确，不是实证有效域，不得膨胀，formalization-validity-domain）。

  ── 对象（v1 = 特征序列线段，xianduan.md 口径B，谱系 003）────────────────────
  本模块形式化线段划分 **v1 特征序列法**（第67课）的真完全分类——这是 claim5
  `ConstitutiveLadder` 中 `Segment` 第4级的 **v1 升级**：claim5 显式把第4级降级为
  **v0 reference ladder**（`wellFormedV0 = 三笔重叠下界骨架`，Claim5_ConstitutiveLadder.lean:180-235），
  并声明"v1 特征序列法不在本形式化范围，引用谱系 003"（同文件 :194-196）。
  本 claim10 补全 v1：第67课特征序列分型 → 线段划分两种情况穷尽 → 第77课方向/奇数性
  → 第78课古怪线段 + "顶高于底"硬约束。

  ── v0/v1 口径分离（watch-item #1，谱系 003，bit-exact 引擎口径不动）────────────
  谱系 003 已结算：v0=三笔重叠骨架（`a_segment_v0.py`，参考实现），
  v1=特征序列法（`a_segment_v1.py`，唯一正式口径）。编排者决断（xianduan.md:226-237）：
  v1 为唯一正式口径，v0 降级为参考实现。本模块只在 Lean 层形式化 v1 的完全分类，
  **不碰 Rust 引擎**、不改 v0 bit-exact 行为。v0 与 v1 **不是定义冲突**——是同一概念
  （线段）的两个已分离口径（谱系 003），v0 是 v1 的下界骨架（v0 每三笔重叠即成段，
  v1 等特征序列分型才断段，v0 段数是 v1 的 2-3 倍，xianduan.md:199），二者并存合法。

  ── 真完全分类的对象（第67课"只有两种可能"）──────────────────────────────────
  缠师第67课原文（067-第67课.md:24-46）：
    "在标准特征序列里，构成分型的三个相邻元素，只有两种可能：第一种情况…第二种情况…
     上面两种情况，就给出所有线段划分的标准。"
  这是缠师 **自己声明的完全分类**——划分情形二分 {第一种(无缺口) / 第二种(有缺口)}。
  603 范式：完全性不是"特征空间轨道穷尽"（外延式→无穷回归），是 **划分情形 inductive
  构造子穷尽**（内涵式，由缠师"只有两种可能"定理钉死）。本模块的真完全性定理
  `termination_case_dichotomy` = 对该 inductive 结构归纳（machine-checked，无第三种情况）。

  ── 隔离声明（任务卡：不碰 lakefile.toml / Formal.lean，integrator 统一 wire）──
  本模块 **自包含**（不 import Formal.* / 不 import Phase2.Claim5）。与 claim5 第4级
  `Segment` 的对接点是命题层（非 import 层）：本模块第4级 v1 `Segment` 与 claim5 v0
  `Segment` 是 **同一线段概念的两个口径**（谱系 003），claim5 = v0 骨架，本模块 = v1 完整。

  ── 缠论定义依据（逐条溯源，watch-item #3：博文回溯第67/71/77/78课原文）────────
  - 第67课（067-第67课.md，特征序列法核心）：
    * 特征序列定义（:16-18）：向上线段取反向笔 X 序列、向下线段取反向笔 S 序列；
      相邻元素无重叠区间 = 缺口。
    * 标准特征序列（:20）：特征序列元素当 K 线做包含处理（非包含处理）后的序列。
    * 分型方向（:22）：向上线段只看顶分型，向下线段只看底分型。
    * 两种情况完全分类（:24-48）：第一种(分型第1、2元素无缺口→该高/低点即终点)、
      第二种(第1、2元素有缺口→须第二特征序列出反向分型才终结)。"只有两种可能。"
    * 第二种简化（:46）：第二特征序列的分型不再分第一二种情况，只要有分型即可。
  - 第71课（在 067 第四/五种情况引文 :122/126 + 077 提及）：包含关系作用域——
    特征序列元素做包含处理的前提是元素 **同属一个特征序列**；转折点两边（不同性质）
    不能包含；第三笔完全在第一笔范围内 → 待定（旧段延续 or 新段成立两结果）。
  - 第77课（077-第77课.md，方向与奇数性）：
    * 方向一致性（:62）：以向上笔开始的线段一定结束于向上笔，不可能从底到底/顶到顶。
    * 奇数性（:62）：线段中包含笔的数目都是单数。
    * 前三笔重叠必须性（:64）：线段开始三笔必须有重合。
    * 第二种情况其实就包含"线段A未被B破坏但C破坏B"的情形（:72-78）。
  - 第78课（078-第78课.md，古怪线段 + 硬约束）：
    * 古怪线段唯一原因（:30）："所有古怪的线段，都是因为线段出现第一种情况的笔破坏后
      最终没有在该方向由该笔发展形成线段破坏。"
    * 笔破坏≠线段破坏（:34）：笔破坏后不一定在该方向发展成线段破坏。
    * "顶高于底"硬约束（:20）："同一线段中，两端的一顶一底，顶肯定要高于底，
      如果你划出一个不符合这基本要求的线段，那肯定是划错了。"——定义的一部分，非后置过滤。
    * 第二种情况第二特征序列必须严格按包含处理（:54）：不存在第一种情况"分界点两边不
      包含"的要求（方向与原线段一致，包含=能量充足，须按包含来）。
    * 标准化处理（:58-60）：最高低点不在端点的线段可标准化为最高低点都在端点。
-/

namespace Formal.Phase2.SegmentV1

/-! ## 公共：方向 / 价格区间 -/

/-- 笔 / 线段方向（第77课二构造子穷尽：上 / 下）。 -/
inductive Dir where
  | up
  | down
deriving DecidableEq, Repr

/-- 方向取反（向上线段的特征序列由向下笔构成，反之亦然，第67课:16-18）。 -/
def Dir.flip : Dir → Dir
  | Dir.up => Dir.down
  | Dir.down => Dir.up

@[simp] theorem Dir.flip_flip (d : Dir) : d.flip.flip = d := by
  cases d <;> rfl

/--
  价格区间 [lo, hi]（笔 / 特征序列元素的几何投影，`lo ≤ hi` 不变量）。
  特征序列把每一元素当 K 线，其包含关系 / 缺口都基于此区间（第67课:20）。
-/
structure Interval where
  lo : Int
  hi : Int
  valid : lo ≤ hi
deriving Repr

/-- 两区间有公共重叠（交集非空）。缺口 = 无重叠（第67课:18）。 -/
def Interval.overlaps (a b : Interval) : Prop := a.lo ≤ b.hi ∧ b.lo ≤ a.hi

/-- 缺口：两相邻特征序列元素 **没有** 重合区间（第67课:18 缺口定义）。 -/
def Interval.gap (a b : Interval) : Prop := ¬ a.overlaps b

/-- 缺口判定（Decidable 镜像，用于完全性枚举）。 -/
def Interval.hasGap (a b : Interval) : Bool :=
  decide (a.hi < b.lo ∨ b.hi < a.lo)

/-! ## 笔（构成线段的原子）—— v1 输入

  本模块的 v1 划分作用在 **笔序列** 上。笔的方向二分 + 价格区间是特征序列构造的输入。
  （笔本身的良构 = bi 条件1/3，已在 claim5 第3级形式化；本模块只取其方向 + 区间。）
-/

/-- 笔：带方向与价格区间（区间由两端分型极值价确定，方向无关取 min/max）。 -/
structure Stroke where
  dir : Dir
  range : Interval
deriving Repr

/-! ## 特征序列（第67课:16-22）—— v1 的核心构造

  以向上笔开始的线段，笔序列 S1 X1 S2 X2 … ，特征序列取 **向下笔 X 序列**；
  以向下笔开始的线段，特征序列取 **向上笔 S 序列**。即：线段方向 `d`，特征序列取
  方向为 `d.flip` 的笔。特征序列元素当 K 线，做包含处理 → 标准特征序列（第67课:20）。
-/

/--
  从笔序列抽取特征序列元素区间：取与线段方向 **相反** 的笔的区间（第67课:16-18）。
  向上线段 (d=up) 取向下笔 (dir=down)；向下线段取向上笔。
-/
def featureElements (segDir : Dir) (strokes : List Stroke) : List Interval :=
  (strokes.filter (fun s => s.dir = segDir.flip)).map (·.range)

/--
  特征序列两相邻元素的包含关系（第67课:20，把元素当 K 线）：
  一元素的区间完全含于另一元素（与 K 线包含处理同义）。
-/
def Interval.contains (a b : Interval) : Prop := a.lo ≤ b.lo ∧ b.hi ≤ a.hi

def Interval.hasInclusion (a b : Interval) : Bool :=
  decide ((a.lo ≤ b.lo ∧ b.hi ≤ a.hi) ∨ (b.lo ≤ a.lo ∧ a.hi ≤ b.hi))

/--
  特征序列分型（第67课:22，把每个标准特征序列元素当 K 线找分型）。
  向上线段只考察 **顶分型**，向下线段只考察 **底分型**（第67课:22）——
  即分型类型由线段方向钉死，不是自由的第三维。
-/
inductive FeatureFractalKind where
  | top      -- 顶分型（向上线段考察）
  | bottom   -- 底分型（向下线段考察）
deriving DecidableEq, Repr

/-- 线段方向决定其特征序列应考察的分型类型（第67课:22）。 -/
def expectedFractalKind : Dir → FeatureFractalKind
  | Dir.up => FeatureFractalKind.top
  | Dir.down => FeatureFractalKind.bottom

/-! ## ★线段划分两种情况——真完全分类（第67课:24-48 "只有两种可能"）

  这是本 claim 的核心：缠师第67课原文断言"在标准特征序列里，构成分型的三个相邻元素，
  **只有两种可能**"。把"两种可能"铸成二构造子 inductive，"无第三种"由结构归纳钉死。
-/

/--
  ★线段划分情形（第67课:24-48 真完全分类，**二构造子穷尽**）。

  ★前提澄清（复核吸收，关三层次混淆）：`TerminationCase` 分类的对象是 **已在标准特征序列里
  形成的分型**（第67课:24"构成分型的三个相邻元素"）。"笔破坏 / 特征序列分型形成 / 线段终结"
  是 **三个递进层次**，不可混淆（077:66 / 078:30-36）：
    (a) 笔破坏 = 破坏笔出现（触发信号，**非**终结）；
    (b) 特征序列分型形成 = 后续三笔重合、在标准特征序列上构成顶/底分型（077:66"后面这三笔
        没有重合，不可能构成一线段…根本不可能形成特征序列的分型"——分型可能 **形不成**）；
    (c) 线段终结 = 分型已形成后按本二分类判定。
  本 `TerminationCase` 仅刻画层次 (c)（**分型已形成** 的前提下的情形二分）；层次 (b)
  "笔破坏后分型是否形成" 由下文 `PostBreakOutcome` 刻画（古怪线段的来源）。

  在分型已形成的前提下，线段终结由"分型第一与第二元素是否有缺口"二分，**只有两种**：

  - `firstKind`（第一种情况，无缺口，第67课:28）：特征序列分型中第一、二元素间 **不存在**
    缺口。**该分型一旦形成**，该线段即在该分型的高/低点处结束，该点即终点（067:28；
    067 答疑:194"第一种情况…任何三笔其实都构成对前面线段的破坏"），**无需** 第二特征序列。
    （注意：此处"直接终结"的前提是分型 **已形成**；若笔破坏后分型形不成，则线段未终结=古怪
    线段，见 `PostBreakOutcome`——这正是 077:66/078:36 与 067:28 的层次区分。）

  - `secondKind`（第二种情况，有缺口，第67课:38）：特征序列分型中第一、二元素间 **存在**
    缺口。**即使该分型已形成也不直接终结**，须 **从该分型极值点开始的反向笔序列的特征序列
    出现反向分型**（第二特征序列分型）才确认终结（第67课:38；067 答疑:196"麻烦的是第二
    种情况，并不是任何三笔都能构成破坏"）。第二特征序列分型不再分第一二种情况（第67课:46）。

  ★口径依据声明（复核吸收 + no-workaround，CLAUDE.md 三级权威链：博文 > 编纂版）：
  本二分类的 **终结语义** 依据 **第67课博文原文**（067:28 第一种=分型形成即终结 /
  067:38 第二种=须第二特征序列）。`.chanlun/definitions/xianduan.md:169-170` 的表述
  （"第一种=需发展为线段破坏才终结 / 第二种=直接线段破坏"）**与博文原文相反**，疑似编纂层
  笔误——已 /escalate 上浮 genealogist 核对（见模块末 escalate 标注）。本模块站博文一侧。

  缠师"只有两种可能"= 这个 inductive 的构造子集穷尽。无第三种情况
  （067 答疑:176-184 缠师亲自把"两种情况"复制重申）。
-/
inductive TerminationCase where
  | firstKind    -- 第一种情况：无缺口，分型形成即终结（前提：分型已形成）
  | secondKind   -- 第二种情况：有缺口，须第二特征序列分型才终结
deriving DecidableEq, Repr

/--
  ★线段划分情形真完全分类（**二构造子穷尽，L0**，第67课"只有两种可能"）。

  对 `TerminationCase` 结构归纳——Lean 接受证明 = 构造子集穷尽（无第三种线段划分情形）。
  这正是 603 内涵式完全性对线段 v1 的应用：完全性由缠师第67课"只有两种可能"定理钉死，
  不是经验断言。
-/
theorem termination_case_dichotomy (c : TerminationCase) :
    c = TerminationCase.firstKind ∨ c = TerminationCase.secondKind := by
  cases c
  · exact Or.inl rfl
  · exact Or.inr rfl

/--
  从特征序列分型中心的相邻两元素判定 **进入哪个划分分支**（第67课:28/38 缺口判据，全函数）。

  `e1, e2` = 特征序列分型的第一、第二元素（标准特征序列中分型中心前/中两元素）。
  - 第一、二元素 **无缺口**（重叠）→ 第一种分支；
  - 第一、二元素 **有缺口**（不重叠）→ 第二种分支。

  ★承载范围澄清（codex 异质审计吸收，关有效域膨胀）：本函数判定的是"分型已形成后
  **进入哪个分支**"（纯缺口判据），**不是**"线段终结已确认"。具体而言：
  - 返回 `firstKind`（无缺口）：在分型已形成的前提下，按第67课:28 该分支 **即终结**
    （此分支的终结与缺口判据同步，故 firstKind 可直接读作终结）；
  - 返回 `secondKind`（有缺口）：仅表示 **进入第二种分支**，终结 **尚未确认**——
    第67课:38 要求"从分型极值点开始的反向笔序列的特征序列出现反向分型"（第二特征序列
    分型）才确认终结。该"第二特征序列分型是否实际出现"是 **动态过程**（`a_segment_v1.py`
    状态机职责），**不**由本函数判定。故 `secondKind` 不可读作"终结已确认"，只可读作
    "进入有缺口分支、待第二特征序列分型确认"。
  这是 `TerminationCase` **分支** 的忠实判定（缺口 ⟺ 进入 secondKind 分支）。
-/
def classifyTermination (e1 e2 : Interval) : TerminationCase :=
  if e1.hasGap e2 then TerminationCase.secondKind else TerminationCase.firstKind

/-- 判定忠实：第一种情况 ⟺ 第一、二元素无缺口（第67课:28）。 -/
theorem firstKind_iff_no_gap (e1 e2 : Interval) :
    classifyTermination e1 e2 = TerminationCase.firstKind ↔ ¬ (e1.hasGap e2 = true) := by
  unfold classifyTermination
  cases h : e1.hasGap e2 <;> simp_all

/-- 判定忠实：第二种情况 ⟺ 第一、二元素有缺口（第67课:38）。 -/
theorem secondKind_iff_gap (e1 e2 : Interval) :
    classifyTermination e1 e2 = TerminationCase.secondKind ↔ e1.hasGap e2 = true := by
  unfold classifyTermination
  cases h : e1.hasGap e2 <;> simp_all

/-! ## 第71课：包含关系作用域 + 第78课：第二种情况第二特征序列严格性

  第71课（067:122/126 引文）：特征序列元素做包含处理的前提是元素 **同属一个特征序列**；
  转折点两边（性质不同）不能包含。
  第78课:54：但 **第二种情况的第二特征序列** 必须严格按包含处理，不存在"分界点两边
  不包含"的要求（方向与原线段一致，包含=能量充足）。
  这两条是 **对偶约束**：第一种情况分界点两边不包含 / 第二种情况第二特征序列必须包含。
-/

/--
  包含处理是否允许（第71课作用域 + 第78课第二种情况严格性，对偶约束）。

  ★范围澄清（复核吸收）：本谓词刻画的是 **分界点处（转折点两边）** 的局部包含作用域，
  **不是** 整个特征序列的包含规则——同一特征序列内部的元素永远按包含处理（第67课:20
  标准特征序列的常规）。本对偶仅针对"转折点两边/第二特征序列"这一 **分界处** 的特殊作用域：

  - 第一种情况（`firstKind`）：分界点两边 **不允许** 包含（第71课:122：转折点后第一笔
    属"中间地带"，似有包含也不算；第78课:54"假设分界点两边不能进行包含关系处理"）。
    这 **不** 意味 firstKind 全程禁包含——仅分界处禁。
  - 第二种情况第二特征序列（`secondKind`）：**必须** 按包含处理（第78课:54"必须严格
    按照包含关系的处理来，不存在第一种情况中的假设分界点两边不能包含的要求"）。
-/
def inclusionAllowedAtBoundary : TerminationCase → Bool
  | TerminationCase.firstKind => false   -- 第71课：转折点两边不包含
  | TerminationCase.secondKind => true   -- 第78课:54：第二特征序列必须包含

/--
  ★包含作用域对偶约束完全分类（第71课 + 第78课:54，L0）：
  包含是否允许 **完全由划分情形二分决定**——第一种禁、第二种许，无第三种作用域。
  对 `TerminationCase` 穷举两构造子，判定完备。
-/
theorem inclusion_scope_total (c : TerminationCase) :
    inclusionAllowedAtBoundary c = false ∨ inclusionAllowedAtBoundary c = true := by
  cases c
  · exact Or.inl rfl
  · exact Or.inr rfl

/-- 包含作用域与划分情形一一对应（第一种禁包含、第二种许包含，无歧义）。 -/
theorem inclusion_iff_secondKind (c : TerminationCase) :
    inclusionAllowedAtBoundary c = true ↔ c = TerminationCase.secondKind := by
  cases c <;> simp [inclusionAllowedAtBoundary]

/-! ## 第77课：方向一致性 + 奇数性 ; 第78课："顶高于底"硬约束 + 前三笔重叠 -/

/--
  v1 线段：笔序列 + 方向。
  本结构的良构 `wellFormedV1` 编码 v1 的 **硬约束**（第77/78课），区别于 v0 骨架
  （claim5 `wellFormedV0` 只有 length≥3 + 首笔方向 + 前三笔重叠）。
-/
structure Segment where
  strokes : List Stroke
  direction : Dir
deriving Repr

/-- 线段方向二分完全分类（第77课，二构造子穷尽，L0）。 -/
theorem segment_dir_dichotomy (s : Segment) :
    s.direction = Dir.up ∨ s.direction = Dir.down := by
  cases h : s.direction
  · exact Or.inl rfl
  · exact Or.inr rfl

/-- 线段两端价（起点 = 首笔起价侧，终点 = 末笔止价侧；这里用笔区间端点抽象）。 -/
def Segment.startPrice (s : Segment) : Option Int :=
  match s.strokes with
  | [] => none
  | f :: _ => some (match s.direction with | Dir.up => f.range.lo | Dir.down => f.range.hi)

def Segment.endPrice (s : Segment) : Option Int :=
  match s.strokes.getLast? with
  | none => none
  | some l => some (match s.direction with | Dir.up => l.range.hi | Dir.down => l.range.lo)

/-- 笔数（奇数性约束的对象，第77课:62）。 -/
def Segment.strokeCount (s : Segment) : Nat := s.strokes.length

/--
  ★v1 线段良构（硬约束，第77/78课）——区别于 claim5 v0 骨架。

  五条 v1 硬约束（全部第77/78课原文）：
  - (H1) 至少三笔（第77课:62/64，xianduan.md:128）：`strokes.length ≥ 3`。
  - (H2) 奇数性（第77课:62"线段中包含笔的数目都是单数"）：`Odd strokes.length`。
  - (H3) 方向一致性（第77课:62"以向上笔开始的线段一定结束于向上笔"）：首笔、末笔方向
    都 = 线段方向。
  - (H4) 前三笔重叠（第77课:64"线段开始的那三笔必须有重合"）：前三笔区间公共交集非空。
  - (H5) ★"顶高于底"硬约束（第78课:20，**定义的一部分非后置过滤**）：终点价与起点价
    满足"顶高于底"——向上线段终点(顶) > 起点(底)，向下线段终点(底) < 起点(顶)。

  ★诚实标注（复核吸收，no-workaround / formalization-validity-domain，关有效域膨胀）：
  本良构编码 v1 的 **静态硬约束**（每个已划分线段必须满足的不变量）。本模块形式化的
  L0 对象是 **两件静态事物**：
    (1) 划分情形的 **静态完全分类**——给定特征序列分型相邻两元素 `e1,e2` 后的情形二分
        （`TerminationCase` + `classifyTermination`，完全性 = `termination_case_dichotomy`）；
    (2) 线段的 **静态不变量**（本 `wellFormedV1` 五硬约束）。
  **不在本模块形式化范围**（诚实声明，不冒充）：v1 的 **动态划分算法**——从笔序列增量
  抽取特征序列、对元素做包含处理生成标准特征序列、扫描定位分型中心、触发判定终结——
  这一动态过程是 `src/newchan/a_segment_v1.py` 的 `_FeatureSeqState` 状态机的职责
  （xianduan.md:202-216 记录其 4/4 审计 + 37/37 测试通过）。本模块 **不** 形式化该扫描
  循环：`featureElements`（上文）只做一次性 filter+map 抽取、**未** 做包含处理，
  `classifyTermination` 只接受 **已给定** 的分型两元素。本模块交付的是 v1 的 **L0 形态学
  骨架**（划分情形静态穷尽 + 线段静态不变量），不是 v1 算法的可执行形式化。
-/
def Segment.wellFormedV1 (s : Segment) : Prop :=
  -- H1 至少三笔
  s.strokes.length ≥ 3
  -- H2 奇数性（不依赖 Mathlib，用 `% 2 = 1` 表达"单数"，第77课:62）
  ∧ s.strokes.length % 2 = 1
  -- H3 方向一致性（首笔、末笔方向 = 线段方向）
  ∧ (∀ (h : 0 < s.strokes.length), s.strokes[0].dir = s.direction)
  ∧ (∀ l, s.strokes.getLast? = some l → l.dir = s.direction)
  -- H4 前三笔重叠（公共交集非空）
  ∧ (∀ (h0 : 0 < s.strokes.length) (h1 : 1 < s.strokes.length) (h2 : 2 < s.strokes.length),
      max s.strokes[0].range.lo (max s.strokes[1].range.lo s.strokes[2].range.lo)
        ≤ min s.strokes[0].range.hi (min s.strokes[1].range.hi s.strokes[2].range.hi))
  -- H5 顶高于底硬约束（第78课:20）
  ∧ (∀ p q, s.startPrice = some p → s.endPrice = some q →
      match s.direction with
      | Dir.up => p < q      -- 向上：起点(底) < 终点(顶) ⟺ 顶高于底
      | Dir.down => q < p)   -- 向下：终点(底) < 起点(顶) ⟺ 顶高于底

/--
  ★方向一致性蕴含端点性质（第78课:18"不可能从底到底或从顶到顶"）。
  良构线段首末笔方向都 = 线段方向 ⟹ 首末笔同向 ⟹ 端点是一顶一底（非同性质）。
-/
theorem direction_consistency_endpoints (s : Segment) (h : s.wellFormedV1)
    (hne : 0 < s.strokes.length) (l : Stroke) (hl : s.strokes.getLast? = some l) :
    s.strokes[0].dir = s.direction ∧ l.dir = s.direction := by
  obtain ⟨_, _, hfirst, hlast, _, _⟩ := h
  exact ⟨hfirst hne, hlast l hl⟩

/-! ## 第78课：古怪线段的唯一原因（:30）—— 笔破坏 ≠ 线段破坏

  第78课:30 原文："所有古怪的线段，都是因为线段出现第一种情况的笔破坏后最终没有在该
  方向由该笔发展形成线段破坏所造成的，这是线段古怪的唯一原因。"
  第78课:34："线段最终肯定都会被线段破坏，但线段出现笔破坏后最终并不一定在该方向由
  该笔发展形成线段破坏。"
  ⟹ 笔破坏是线段破坏的 **必要非充分** 条件。古怪 ⟺ 第一种情况笔破坏未发展成线段破坏。
-/

/--
  笔破坏后的发展结局（第78课:40-48，**二构造子穷尽**）。

  第一种情况笔破坏（向上破坏笔完成）后接反向笔，该反向笔形成反向线段，**只有两种**结局：
  - `developsIntoSegmentBreak`（第78课:48）：反向线段 **没破** 该破坏笔的底/顶
    → 破坏笔可延伸出线段 → 前线段确认被破坏（正常，不古怪）。
  - `staysWithinPriorSegment`（第78课:42）：反向线段 **破了** 该破坏笔的底/顶
    → 原线段未结束、继续延续 → 笔破坏未发展成线段破坏（**古怪线段的唯一来源**）。

  缠师"这是线段古怪的唯一原因"= 这个 inductive 二构造子穷尽（无第三种结局）。
-/
inductive PostBreakOutcome where
  | developsIntoSegmentBreak   -- 反向线段未破回破坏笔 → 线段破坏确认（正常）
  | staysWithinPriorSegment    -- 反向线段破回破坏笔 → 原线段延续（古怪来源）
deriving DecidableEq, Repr

/-- 笔破坏发展结局二分完全分类（第78课:40-48，二构造子穷尽，L0）。 -/
theorem post_break_outcome_dichotomy (o : PostBreakOutcome) :
    o = PostBreakOutcome.developsIntoSegmentBreak ∨ o = PostBreakOutcome.staysWithinPriorSegment := by
  cases o
  · exact Or.inl rfl
  · exact Or.inr rfl

/--
  古怪线段判据（第78课:30 唯一原因）：线段古怪 ⟺ 第一种情况笔破坏后
  **未** 在该方向发展成线段破坏（即结局 = `staysWithinPriorSegment`）。
-/
def isOddSegmentCause (c : TerminationCase) (o : PostBreakOutcome) : Bool :=
  decide (c = TerminationCase.firstKind ∧ o = PostBreakOutcome.staysWithinPriorSegment)

/--
  ★古怪线段唯一原因定理（第78课:30，L0）：古怪 **当且仅当** 第一种情况(firstKind) +
  笔破坏未发展成线段破坏(staysWithinPriorSegment)。这是缠师"唯一原因"的形式化——
  其余三种 (firstKind/develops, secondKind/*) 组合都不产生古怪线段。
-/
theorem odd_segment_unique_cause (c : TerminationCase) (o : PostBreakOutcome) :
    isOddSegmentCause c o = true ↔
      (c = TerminationCase.firstKind ∧ o = PostBreakOutcome.staysWithinPriorSegment) := by
  unfold isOddSegmentCause
  simp

/--
  ★笔破坏≠线段破坏（第78课:34，L0）：存在第一种情况笔破坏 + 结局停留在原线段
  （`staysWithinPriorSegment`）的情形——即笔破坏不蕴含线段破坏。这关死"把笔破坏当
  线段破坏"的有效域膨胀（v0 正是只看三笔重叠=笔层破坏，故 v0 段数是 v1 的 2-3 倍）。
-/
theorem stroke_break_not_imply_segment_break :
    ∃ (c : TerminationCase) (o : PostBreakOutcome),
      c = TerminationCase.firstKind ∧ o = PostBreakOutcome.staysWithinPriorSegment ∧
      isOddSegmentCause c o = true :=
  ⟨TerminationCase.firstKind, PostBreakOutcome.staysWithinPriorSegment, rfl, rfl, rfl⟩

/-! ## v1 整体完全分类 = 两种情况穷尽 + 五硬约束 + 古怪唯一原因（顶层定理） -/

/--
  ★v1 线段划分真完全分类（顶层，L0，第67/77/78课联合）。

  v1 的完全性是 **三个 inductive 的构造子穷尽** 的合取：
  1. 划分情形二分穷尽（第67课"只有两种可能"）：`TerminationCase` 无第三种。
  2. 笔破坏发展结局二分穷尽（第78课"唯一原因"）：`PostBreakOutcome` 无第三种。
  3. 线段方向二分穷尽（第77课）：`Dir` 无第三种。

  三者各自由对应缠师定理钉死，结构归纳证明（machine-checked）。这就是 v1 特征序列法
  的真完全分类——不是"枚举了所有特征向量"，是"划分过程的每个分支点都构造子穷尽"。
-/
theorem segment_v1_complete_classification
    (c : TerminationCase) (o : PostBreakOutcome) (s : Segment) :
    (c = TerminationCase.firstKind ∨ c = TerminationCase.secondKind)
    ∧ (o = PostBreakOutcome.developsIntoSegmentBreak ∨ o = PostBreakOutcome.staysWithinPriorSegment)
    ∧ (s.direction = Dir.up ∨ s.direction = Dir.down) :=
  ⟨termination_case_dichotomy c, post_break_outcome_dichotomy o, segment_dir_dichotomy s⟩

/-! ## v0 / v1 口径关系（谱系 003，watch-item #1）—— v0 是 v1 的下界骨架，非冲突 -/

/--
  v0 **最弱下界核** 谓词（仅 `length ≥ 3`）。

  ★承载范围严格限定（codex 异质审计吸收，关有效域膨胀）：本谓词是 v0 口径的
  **最弱下界核**（"至少三笔"，xianduan.md:147 口径A 的核心必要条件），**不是** claim5
  `Phase2/Claim5_ConstitutiveLadder.lean` 中的完整 `Segment.wellFormedV0`——后者是
  `length≥3 ∧ 首笔方向锚定 ∧ 前三笔严格重叠(用 `<`) ∧ …` 的合取（Claim5:229-235）。
  本模块 **自包含**（不 import Claim5），无法引用其谓词，故只在 v0 的 **最弱核**（length≥3）
  上陈述 v1/v0 关系。**不声称** v1 精化 claim5 的完整 v0（那需 integrator 在 wire 时跨模块
  桥接，且须先弥合一个口径差异——见下文 H4 标注）。本谓词复述只为表达 v0/v1 包含关系，
  **不改 v0 引擎行为**（watch-item #1）。

  ★H4 口径差异诚实标注（codex 发现）：本模块 H4 前三笔重叠用 `≤`（闭区间，允许边界相切），
  claim5 v0 用 `<`（开区间，严格重叠）。原文（067/077"必须有重合的部分"）未明确边界相切
  是否算重合——这是一个 **未结算的口径细节**（边界相切=重合？）。本模块用 `≤` 是较宽口径；
  integrator 跨模块桥接 v1⟹claim5-v0 前须统一此 `≤`/`<` 差异。本模块不擅自裁定（保留张力）。
-/
def Segment.wellFormedV0Skeleton (s : Segment) : Prop := s.strokes.length ≥ 3

/--
  ★v1 ⟹ v0 最弱下界核（谱系 003，L0）：满足 v1 完整硬约束的线段 **必然** 满足 v0 的
  最弱下界核（H1 至少三笔 ⟹ length≥3）。即 v1 在 v0 的"至少三笔"下界核之上叠加奇数性 +
  方向一致 + 前三笔重叠 + 顶高于底，v1 比 v0 最弱核 **更强**。这形式化了 xianduan.md:199
  "v0 段数是 v1 的 2-3 倍"的方向——v0 接受更多（更弱），v1 拒绝更多（更强）。

  ★严格边界（codex 吸收）：本定理只证 "v1 ⟹ length≥3"，**未** 证 "v1 ⟹ claim5 的完整
  v0 谓词"（本模块自包含，不引用 claim5；且 H4 `≤` 与 claim5 `<` 有未结算口径差异）。
  二口径 **不冲突**（无定义矛盾，无须 escalate）：同一线段概念的两个已分离口径（谱系 003），
  v0/v1 并存合法，v1 是 v0 的加细方向。完整 v1⟹claim5-v0 桥接留给 integrator（须先统一口径）。
-/
theorem v1_refines_v0 (s : Segment) (h : s.wellFormedV1) : s.wellFormedV0Skeleton := by
  obtain ⟨h1, _, _, _, _, _⟩ := h
  exact h1

/--
  ★v0 不蕴含 v1（谱系 003，L0）：存在满足 v0 骨架但 **不** 满足 v1 的线段
  （v0 不要求奇数性 / 顶高于底，故 v0 可接受 v1 拒绝的对象）。
  构造见证：四笔（偶数，违反 H2 奇数性）线段满足 v0(length≥3) 但违反 v1。
  这关死"v0=v1"的有效域膨胀——二口径严格不等（v1 ⊊ v0），印证谱系 003 的概念分离。
-/
theorem v0_not_imply_v1 :
    ∃ s : Segment, s.wellFormedV0Skeleton ∧ ¬ s.wellFormedV1 := by
  -- 四笔线段：length=4 满足 v0(≥3)，但 4 不是奇数 ⟹ 违反 v1 的 H2
  refine ⟨{ strokes := List.replicate 4 { dir := Dir.up, range := ⟨0, 1, by decide⟩ },
            direction := Dir.up }, ?_, ?_⟩
  · show (List.replicate 4 _).length ≥ 3
    simp
  · intro h
    obtain ⟨_, hodd, _, _, _, _⟩ := h
    -- length = 4，4 % 2 = 1 假（4 是偶数 ⟹ 违反 H2 奇数性）
    have hlen : (List.replicate 4 ({ dir := Dir.up, range := ⟨0, 1, by decide⟩ } : Stroke)).length = 4 := by simp
    rw [hlen] at hodd
    exact absurd hodd (by decide)

end Formal.Phase2.SegmentV1
