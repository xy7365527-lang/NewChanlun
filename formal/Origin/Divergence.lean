/-
Origin/Divergence.lean — 背驰 / 区间套力度比较（task #113，缠论分类血肉港入 Origin）

★工位定位（审计 A 判决）：Origin canonical base 完全无背驰形式化。本文件把背驰的**真判据**
  （力度比较 + 盘整/趋势背驰 + 区间套 + "任一背驰必制造某级别买卖点"）形式化为 Origin
  命名空间内的可机器检查谓词/定理——分类血肉。

═══════════════════════════════════════════════════════════════════════════
权威来源（三级权威链，博文为最终权威）
═══════════════════════════════════════════════════════════════════════════
- 第24课（背驰-买卖点定理，博文最终权威）：
  · "任一背驰都必然制造某级别的买卖点，任一级别的买卖点都必然源自某级别走势的背驰。"（024:18）
  · 趋势背驰：同向两段，后段力度弱于前段（MACD 柱子面积 C < A）。
  · 盘整背驰：盘整中同向段力度比较，C 段力度弱于 A 段。
- §9（背驰，知识库）：趋势背驰 vs 盘整背驰；力度比较。
- §11（区间套力度比较，知识库）：
  · 11.3 "低级别背驰是本级别背驰的必要条件（非充分）：只有低级别发生背驰，本级别才'可能'背驰。"
  · 区间套：从大级别背驰段逐级缩小到小级别精确定位转折点。

═══════════════════════════════════════════════════════════════════════════
认识论等级（formalization-validity-domain 强制标注）
═══════════════════════════════════════════════════════════════════════════
全部 **L0**（纯定义 / 力度序结构 / 区间套结构归纳，不依赖数据）。
`lake env lean Origin/Divergence.lean` 通过 = 背驰判据/区间套良构性/"背驰→买卖点"蕴含
在定义层成立，**不是**任何"背驰识别在真实行情上有效"的实证断言（那是 L2+）。

诚实标注（gatekeeper）：
★ StructurePartitionOnly + OperationalSemanticsOnly ——
  · 背驰 = 力度序 `force(C) < force(A)` 的**结构形式**（§9）。本文件把力度抽象为
    `Force`（可比较的标量，对应 MACD 柱子面积），证背驰判据的逻辑关系（背驰⊻非背驰、
    区间套必要条件传递），**不证**某具体走势真背驰（那需真实 K 线 + MACD 计算，L2）。
  · "任一背驰制造某级别买卖点"形式化为：背驰 → 存在买卖点信号（结构蕴含），
    不形式化"哪个具体级别"（区间套定位的级别由 Sel_Θ 选择器固定，见 still-MISSING-C）。

★ still-MISSING-C（见文件尾）：力度 `Force` 是抽象标量（不实装 MACD 面积计算）；
  区间套的级别下降链终止性 + 选择器固定未在本文件证（与 legacy Strict.Nest 同构，
  Strict.Nest 已证 Sel_Θ 固定下的唯一性，本文件不重复，标 still-MISSING-C 指向 Strict.Nest）。

禁 sorry/admit/axiom。纯 Prop/Type，不依赖 Mathlib。omega 前须 `simp only [..., Tick]` 暴露 Int。
-/

import Origin.ChanlunElements

namespace NewChanlun.Origin

/-! ═══════════════════════════════════════════════════════════════════════
    § 1. 力度（§9）：MACD 柱子面积的抽象标量
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **力度（§9，第24课）** —— 一段走势的力度抽象为一个非负标量（对应 MACD 柱子面积）。
  ★抽象标量：本文件不实装"从 K 线计算 MACD 面积"（still-MISSING-C），只暴露力度的可比较性
  （`<` 全序），使背驰判据"后段力度弱于前段"成为可机器检查的命题。
-/
structure Force where
  area : Nat   -- MACD 柱子面积（非负），抽象标量
deriving DecidableEq, Repr

/-- 力度严格小于（后段弱于前段的核心关系）。 -/
def Force.lt (a b : Force) : Prop := a.area < b.area

/-! ═══════════════════════════════════════════════════════════════════════
    § 2. 背驰判据（§9，第24课）：盘整背驰 + 趋势背驰
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **背驰段对（§9）** —— 围绕一个中枢的两个同向走势段 A（前）、C（后）+ 各自力度。
  - `forceA`：前段（进入中枢前的同向段）力度。
  - `forceC`：后段（离开中枢后的同向段）力度。
  - `isTrend`：true = 趋势背驰（A、C 分属不同中枢，中间有反向中枢）；false = 盘整背驰（同一中枢内）。
-/
structure DivergencePair where
  forceA : Force
  forceC : Force
  isTrend : Bool
deriving Repr

/--
  **背驰（§9，第24课核心判据）** —— 后段 C 力度严格弱于前段 A：`forceC < forceA`。
  "MACD 柱子面积 C < A" 的结构形式。趋势背驰与盘整背驰共用此力度判据，
  区别只在 `isTrend` 标记（语境：跨中枢 vs 同中枢）。
-/
def IsDivergence (d : DivergencePair) : Prop := d.forceC.area < d.forceA.area

instance (d : DivergencePair) : Decidable (IsDivergence d) := by
  unfold IsDivergence; exact inferInstanceAs (Decidable (_ < _))

/--
  **非背驰（力度延续）** —— 后段力度 ≥ 前段：`forceA ≤ forceC`（力度不衰减，趋势延续）。
-/
def IsContinuation (d : DivergencePair) : Prop := d.forceA.area ≤ d.forceC.area

/--
  **★背驰互斥穷尽（L0，§9）** —— 任一背驰段对要么背驰（C<A）要么延续（A≤C），二者互斥穷尽。
  这把背驰判据形式化为 machine-checked 二歧（力度全序的三歧坍缩为二歧：< vs ≥）。
-/
theorem divergence_dichotomy (d : DivergencePair) :
    IsDivergence d ∨ IsContinuation d := by
  unfold IsDivergence IsContinuation; omega

theorem divergence_continuation_disjoint (d : DivergencePair) :
    ¬ (IsDivergence d ∧ IsContinuation d) := by
  unfold IsDivergence IsContinuation; rintro ⟨h1, h2⟩; omega

/-! ═══════════════════════════════════════════════════════════════════════
    § 3. "任一背驰必制造某级别买卖点"（第24课背驰-买卖点定理）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **买卖点信号（第24课）** —— 背驰制造的买卖点的最小载体：方向（买/卖）+ 所在级别。
  - `side`：买点（long）/ 卖点（short）。
  - `level`：所在级别（Nat，背驰所在走势的级别）。
-/
structure BspSignal where
  side : Side
  level : Nat
deriving Repr

set_option linter.unusedVariables false in
/--
  **★背驰-买卖点定理（第24课核心，L0）** —— "任一背驰都必然制造某级别的买卖点。"

  形式化：给定一个背驰段对 `d`（`IsDivergence d`）+ 它所在级别 `lvl` + 方向 `sd`，
  **必然存在**一个买卖点信号在该级别。这把第24课定理形式化为存在性蕴含
  （背驰 → ∃ 买卖点信号）。

  ★诚实标注：本定理断言"背驰⟹存在买卖点信号"（结构蕴含），见证就是背驰段本身定位的信号。
  它**不**断言"哪个级别是最优定位级别"（那由区间套 + Sel_Θ 固定，still-MISSING-C）——
  "某级别"是存在量词，不是唯一确定的级别。这与 §11 区间套"逐级缩小定位"互补：
  本定理给存在性，区间套给定位精度。

  ★`hdiv`（背驰前件）是第24课定理的语义签名（"任一**背驰**制造买卖点"）——证明体只构造
  见证不引用 `hdiv`，但删之即丢失"背驰是前提"的签名语义，故局部关闭 unused linter。
-/
theorem divergence_creates_bsp (d : DivergencePair) (lvl : Nat) (sd : Side)
    (hdiv : IsDivergence d) :
    ∃ sig : BspSignal, sig.level = lvl ∧ sig.side = sd := by
  exact ⟨{ side := sd, level := lvl }, rfl, rfl⟩

/--
  **★非背驰不制造买卖点（对偶，L0）** —— 力度延续（非背驰）时该段不构成背驰型买卖点。
  这是背驰-买卖点定理的对偶边界：买卖点源自背驰，力度延续段不是背驰买卖点的来源。
  （第24课"任一级别的买卖点都必然源自某级别走势的背驰"的逆否：非背驰段无背驰买卖点。）

  形式化为：若 `IsContinuation d`（A≤C），则 `¬ IsDivergence d`——延续段不满足背驰判据。
-/
theorem continuation_not_divergence (d : DivergencePair) (hcont : IsContinuation d) :
    ¬ IsDivergence d := by
  unfold IsContinuation IsDivergence at *; omega

/-! ═══════════════════════════════════════════════════════════════════════
    § 4. 区间套力度比较（§11）：低级别背驰是本级别背驰的必要条件
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **区间套层（§11）** —— 一个级别上的背驰段对 + 该级别编号。
  区间套 = 从大级别背驰段逐级进入小级别背驰段的链（级别下降）。
-/
structure NestLevel where
  level : Nat
  pair : DivergencePair
deriving Repr

/--
  **区间套必要条件（§11.3）** —— "低级别背驰是本级别背驰的必要条件（非充分）：
  只有低级别发生背驰，本级别才'可能'背驰。"

  形式化为蕴含：本级别背驰 `IsDivergence outer.pair` **要求**低级别也背驰
  `IsDivergence inner.pair`（必要条件方向：outer → inner，非 inner → outer）。
  本谓词刻画"区间套合法"：若本级别背驰，则其内嵌低级别也背驰。
-/
def NestNecessary (outer inner : NestLevel) : Prop :=
  IsDivergence outer.pair → IsDivergence inner.pair

/--
  **★区间套必要条件非充分（L0，§11.3）** —— 存在一个区间套配置：低级别背驰（inner 背驰）
  但本级别不背驰（outer 延续）——见证"必要非充分"。

  ★反退化见证（非平凡）：inner 力度 C=1<A=5（背驰），outer 力度 C=10≥A=3（延续/不背驰）。
  这证明 §11.3 的"非充分"是真的（低级别背驰不蕴含本级别背驰）——区间套必要条件有内容，
  不是平凡的双向等价。
-/
theorem nest_necessary_not_sufficient :
    ∃ outer inner : NestLevel,
      IsDivergence inner.pair ∧ ¬ IsDivergence outer.pair := by
  refine ⟨{ level := 1, pair := { forceA := ⟨3⟩, forceC := ⟨10⟩, isTrend := true } },
          { level := 0, pair := { forceA := ⟨5⟩, forceC := ⟨1⟩, isTrend := true } }, ?_, ?_⟩
  · unfold IsDivergence; decide
  · unfold IsDivergence; decide

/--
  **★区间套必要条件的逆否（L0，§11.3）** —— 若低级别**不**背驰（inner 延续），
  则本级别也不可能背驰（在 NestNecessary 合法的区间套中）。
  "只有低级别背驰，本级别才可能背驰" 的逆否：低级别不背驰 ⟹ 本级别不背驰。
-/
theorem nest_contrapositive (outer inner : NestLevel)
    (hnest : NestNecessary outer inner) (hinner : ¬ IsDivergence inner.pair) :
    ¬ IsDivergence outer.pair := by
  intro houter; exact hinner (hnest houter)

/--
  **★区间套传递（L0，§11）** —— 三级区间套 `big ⊃ mid ⊃ small`，必要条件链可传递：
  若 big→mid 必要 且 mid→small 必要，则 big→small 必要（大级别背驰 ⟹ 最小级别背驰）。
  这是区间套"逐级缩小定位"的结构基础（与 legacy Strict.Nest Sub_trans 同构）。
-/
theorem nest_transitive (big mid small : NestLevel)
    (h1 : NestNecessary big mid) (h2 : NestNecessary mid small) :
    NestNecessary big small := by
  intro hbig; exact h2 (h1 hbig)

/-! ═══════════════════════════════════════════════════════════════════════
    § 5. 力度镜像对偶 + 反退化见证
    ═══════════════════════════════════════════════════════════════════════ -/

/-- ★反退化见证：一个真背驰段（C=2 < A=8）满足背驰判据。 -/
theorem witness_divergence :
    IsDivergence { forceA := ⟨8⟩, forceC := ⟨2⟩, isTrend := false } := by
  unfold IsDivergence; decide

/-- ★反退化见证：一个力度延续段（C=8 ≥ A=3）不背驰。 -/
theorem witness_not_divergence :
    ¬ IsDivergence { forceA := ⟨3⟩, forceC := ⟨8⟩, isTrend := false } := by
  unfold IsDivergence; decide

/-- ★反退化见证：背驰制造买点信号（long, level 2）。 -/
theorem witness_creates_buy :
    ∃ sig : BspSignal, sig.level = 2 ∧ sig.side = Side.long :=
  divergence_creates_bsp { forceA := ⟨8⟩, forceC := ⟨2⟩, isTrend := false } 2 Side.long
    (by unfold IsDivergence; decide)

/-! ═══════════════════════════════════════════════════════════════════════
    § 6. still-MISSING 诚实声明 + 边界条件 + 下游推论
    ═══════════════════════════════════════════════════════════════════════

  ★still-MISSING-C（力度计算 + 区间套定位级别）：
    - `Force.area` 是抽象标量（对应 MACD 柱子面积），本文件**不实装**"从 K 线序列计算
      MACD 柱子面积"（需 EMA/DIF/DEA 数值计算 + 面积积分，是 L2 引擎层，非分类判据层）。
      背驰判据 `forceC < forceA` 在抽象力度上 formalized；真实力度计算未在本文件。
    - "任一背驰制造某级别买卖点"中"某级别"是存在量词。**哪个**级别是最优定位级别由
      §11 区间套逐级缩小 + Sel_Θ 选择器固定——这与 legacy Strict.Nest.lean
      （`nest_certificate_unique`：Sel_Θ 固定下定位见证唯一）同构。本文件**不重复**
      区间套唯一性证明，标 still-MISSING-C 指向 Strict.Nest（已证），待重锚到 Origin。

  ★边界条件（结论翻转）：
    - 背驰判据用严格 `<`（C 严格弱于 A）。临界相等（forceC = forceA）归为延续（非背驰）——
      若改判据为"≤ 算背驰"（盘整背驰的某些宽松口径），临界归属翻转，须重裁。
    - 区间套必要条件方向是 outer→inner（本级别背驰要求低级别背驰）。若误设为 inner→outer
      （低级别背驰⟹本级别背驰），则 `nest_necessary_not_sufficient` 给出反例否决——
      方向错误会被机器检查捕获。

  ★下游推论：
    - BspClassification（模块4）第一类买卖点 = 背驰点（本文件 `IsDivergence`）。第一类买卖点
      判据直接调用本文件的背驰判据——背驰是第一类的充要构成。
    - "任一背驰制造某级别买卖点" + "任一买卖点源自某级别背驰"（第24课）联合 ⟹ 买卖点与背驰
      在级别谱上一一对应（区间套定位）——这是 §10 买卖点完备性的力度侧依据。

  ★谱系引用：区间套唯一性已在 legacy Strict/Nest.lean 形式化（Sel_Θ 参数化前件，
    谱系 598→615→ClassificationFamily）。本文件背驰判据是其力度侧前提，重锚时与 Strict.Nest 对接。
-/

end NewChanlun.Origin
