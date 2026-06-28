/-
Origin/SeparateStrategyTarget.lean — 工作单元 W9：分账本全定义策略「目标激活集」+「目标头寸 q̄_Θ」
的 L0 结构形式化（条目 C31 / C32 + 关联 C41 LegTarget 显式定义）。

★工位定位（cov: 推导完全分类.pdf 23 页权威版 §五 页14 + §十三 页20）：

  分账本扩展（§一–§十四）的全定义策略链中，本文件承载「集合更新 + 目标头寸生成」一环：
  C31 给出**目标激活集** `Ã_{t+1} = (A_t \ D_t) ∪ B_t`（先关闭结束元素，再开启新开始元素）；
  C32 给出**目标分账本头寸** `q̄^±_{ν(e),t+1} = s_e · 1[e∈Ã_{t+1}] · 1[ε_e=±1]`（每语法元素
  一头寸腿，多空独立坐标，按方向 ε_e 决定开多腿还是空腿）；C41 给出统一的 `LegTarget_Θ(e)` =
  `s_e e^+_{ν(e)} if ε_e=+1; s_e e^-_{ν(e)} if ε_e=-1`，目标头寸 `q̄_Θ = LegTarget((A\D)∪B)`。

  这是分账本声部级全元素覆盖（C42 最终定理）的「目标生成」前置：把激活元素集映射为目标头寸腿。
  下游 W10（策略全定义 ∃!）/W11（覆盖可行→仓位=目标）引本文件的 `targetActiveSet` /
  `legTarget` / `targetLeg` 作为目标头寸的 canonical 表示。

★owner 边界（铁律）：本文件**新建**，不扩展 `FullDefinitionStrategy.lean`（共享文件 owner 冲突）。
  import `FullDefinitionStrategy` 仅为命名空间一致（实际复用通过 `SourceAxioms.Side`）。
  不碰 lakefile.toml；root 名 `Origin.SeparateStrategyTarget` 待 Lead 登记。

★诚实标注（231号 / formalization-validity-domain）：

  - 本文件 = **L0**（纯结构定义，不依赖任何数据）。所有定理是「从 §五/§十三 定义直接转写 +
    集合/指示函数的代数恒等」，信息增量为零（同义反复：PDF 文字 ↦ Lean 定义/引理）。
    **目标头寸是语法层的定义对象**——`q̄_Θ` 是「策略想要的仓位」的形式表达，**不是**实盘盈利保证、
    不是「净账户每笔盈利」（C40/C42 明确否定后者，有效域 = 分账本声部级 / 毛收益级 / 语法元素级）。

  - 上游 W6（分账本头寸空间 P^sep，C25）/ W7（语法元素四元组 e，C27）/ W8（单射 ν + s_e，C28）
    的 Lean 文件**尚未建立**（grep `Origin/Separate*.lean` 无命中）。C31/C32 的定义依赖语法元素 e、
    单射 ν、目标单位数 s_e、多空腿坐标 e^±_v——本文件按 §B 的 W9 定位（依赖 `CompleteStateEvent`、
    自包含），在文件内建立 C31/C32 **所需的最小语法元素 + 头寸腿表示**（`SyntaxElement` /
    `targetLeg`），严格转写 §二 C27（四元组）/ §三 C28（ν 单射 + s_e>0 + σ_{ν(e)}=ε_e）的定义子集，
    **不臆造** W6 的完整 P^sep 直积代数。待 W6/W7/W8 文件建立后，本文件的 `SyntaxElement` /
    `targetLeg` 可重锚到那些更完整的类型——届时是 schema 精化，非本文件缺陷（no-workaround：
    本文件不写垫片 fallback，只严格定义 C31/C32 自身需要的对象）。

  - 极性 ε_e ∈ {+1,−1} 与头寸腿方向 σ_{ν(e)} 复用 Origin canonical `Side`（long/short，
    `SourceAxioms.lean:43`）——PDF §三 C28 要求 `σ_{ν(e)} = ε_e`，二者共享同一两态极性域。
    本文件给出 `Side.toInt`（long↦+1 / short↦−1）对齐 §二 `ε_e ∈ {+1,−1}` 的数值约定。

依赖方向（单向无环，全 committed 只读）：SeparateStrategyTarget → FullDefinitionStrategy → …（纯 Origin）。

命名空间 NewChanlun.Origin.SeparateStrategyTarget。
-/

import Origin.FullDefinitionStrategy

namespace NewChanlun.Origin.SeparateStrategyTarget

open NewChanlun.Origin (Side)

/-! ## §A 极性 ε / σ 的数值投影（对齐 §二 `ε_e ∈ {+1,−1}`） -/

/--
极性到 `Int` 的语义投影（`long ↦ +1` / `short ↦ −1`，对齐 PDF §二 line 1456 `ε_e ∈ {+1,−1}`）。

★复用 Origin canonical `Side`（`SourceAxioms.lean:43`）作为元素方向 ε_e 与头寸腿方向 σ_{ν(e)}
的共享极性域——§三 C28 要求 `σ_{ν(e)} = ε_e`，二者同域。`toInt` 给出 §二的 ±1 数值约定。
-/
def Side.toInt : Side -> Int
  | .long  =>  1
  | .short => -1

/-- 极性数值投影非零（`+1` 或 `−1`，永不为 0）——两态完全分类，无空仓极性。 -/
theorem Side.toInt_ne_zero (ε : Side) : Side.toInt ε ≠ 0 := by
  cases ε <;> decide

/-! ## §B 缠论语法元素 e（§二 C27 四元组，W9 自包含最小转写）

§二 line 1452-1462：`e = (I_e, ε_e, ℓ_e, par(e))`，区间 `I_e=[λ_e,ρ_e)`、方向 `ε_e∈{+1,-1}`、
级别 `ℓ_e`、父 `par(e)`；方向 `ε_e(P_{ρ_e}-P_{λ_e})>0`、`Start(e)=λ_e`、`End(e)=ρ_e`。

★本文件只取 C31/C32 真正读到的子集：时间端点 `λ_e/ρ_e`（C31 的 `B_t={e:λ_e=t}` /
`D_t={e:ρ_e=t}` 读端点）+ 方向 `ε_e`（C32 的 `1[ε_e=±1]` 读方向）。级别 ℓ_e / 父 par(e) 是
C27 完整四元组的分量，C31/C32 不直接读——为 schema 完整保留为字段，不臆造其递归语义（那是 W7）。-/

/--
缠论语法元素 `e`（§二 C27 四元组，`SourceAxioms.lean` 风格的纯结构）。

逐分量对照（§二 line 1452-1462）：
- `lambda : Int`（`λ_e`，区间左端点 `Start(e)`，C31 `B_t={e:λ_e=t}` 读此）。
- `rho : Int`（`ρ_e`，区间右端点 `End(e)`，C31 `D_t={e:ρ_e=t}` 读此）。
- `eps : Side`（`ε_e∈{+1,-1}`，元素方向；C32 `1[ε_e=±1]` 读此；复用 Origin `Side`）。
- `level : Nat`（`ℓ_e`，级别；C27 分量，C31/C32 不读，schema 完整保留）。
- `parent : Option Nat`（`par(e)`，父元素索引；C27 分量，根元素 `none`；C31/C32 不读）。

★诚实标注：方向语义约束 `ε_e(P_{ρ_e}-P_{λ_e})>0`（§二 line 1458）是元素**构造**的一致性条件
（向上元素 ε=+1 / 向下 ε=-1 由端点价格决定），属 W7 元素识别层；本结构承载元素**数据**，
该约束作为可分离前提（下游 W12/W13 用），不耦进构造——与 `CompleteStateEvent.VoiceForest`
把一致性条件外置为 `Prop` 谓词同构。
-/
structure SyntaxElement where
  lambda : Int
  rho : Int
  eps : Side
  level : Nat
  parent : Option Nat
deriving Repr, DecidableEq

/-- `Start(e) = λ_e`（§二 line 1460）。 -/
def SyntaxElement.start (e : SyntaxElement) : Int := e.lambda

/-- `End(e) = ρ_e`（§二 line 1460）。 -/
def SyntaxElement.finish (e : SyntaxElement) : Int := e.rho

/-! ## §C 唯一头寸腿映射 ν + 目标单位数 s_e（§三 C28，W9 自包含最小转写）

§三 line 1466-1476：`ν: E→V` 单射、目标方向 `σ_{ν(e)}=ε_e`、目标单位数 `s_e>0`、
同股数 `s_e=s_{par(e)}`（一般 `s_e=κ_e s_{par(e)}, 0<κ_e≤1`）。

★C32 读到的子集：`ν(e)`（头寸腿归属的声部索引）+ `s_e`（目标单位数）+ `ε_e`（方向，已在元素中）。
本文件把 `ν` + `s_e` 表示为「元素配属」`LegAssign`——每个元素带它的规范腿声部索引与单位数。
单射性 `e≠e′⟹ν(e)≠ν(e′)`（§三 line 1468）作为可分离谓词 `legAssignInjective` 给出（C28 要求），
不耦进构造（避免数据与约束耦死）。-/

/--
单元素的头寸腿配属 `(ν(e), s_e)`（§三 C28：规范头寸腿声部 + 目标单位数）。

- `nu : Nat`（`ν(e)`，元素 e 的规范头寸腿在声部空间 V 中的索引；§三 line 1466 单射映射的值）。
- `s : Nat`（`s_e`，目标单位数；§三 line 1472 `s_e>0`，用 `Nat` 承载非负，正性作可分离谓词）。

★方向 `σ_{ν(e)}=ε_e`（§三 line 1470）不存在本结构里——它是「腿方向 = 元素方向」的恒等，
本文件在 `legTarget` / `targetLeg` 生成腿时**直接用元素的 `eps`** 作腿方向（即令 σ_{ν(e)}:=ε_e），
故该恒等是定义性满足（`targetLeg` 的 `side` 字段恒等于 `e.eps`，见 `targetLeg_side_eq_eps`）。
-/
structure LegAssign where
  nu : Nat
  s : Nat
deriving Repr, DecidableEq

/--
单射性谓词 `ν: E→V` 单射（§三 line 1468：`e≠e′ ⟹ ν(e)≠ν(e′)`）。

在元素列表 `es` 与配属函数 `assign : SyntaxElement → LegAssign` 上：不同元素的腿声部索引不同。
可分离谓词（C28 要求），不耦进构造——下游 W12「ν 单射 ⟹ 规范腿唯一」引此。
-/
def legAssignInjective
    (es : List SyntaxElement) (assign : SyntaxElement -> LegAssign) : Prop :=
  ∀ e e', e ∈ es -> e' ∈ es -> (assign e).nu = (assign e').nu -> e = e'

/-- 目标单位数正性 `s_e > 0`（§三 line 1472），可分离谓词（C28 要求）。 -/
def targetUnitsPositive
    (es : List SyntaxElement) (assign : SyntaxElement -> LegAssign) : Prop :=
  ∀ e, e ∈ es -> (assign e).s > 0

/-! ## §D 目标头寸腿 `LegTarget(e)`（C41 §十三 line 1480 显式定义）

§十三 boxed：`LegTarget_Θ(e) = s_e e^+_{ν(e)}  if ε_e=+1;  s_e e^-_{ν(e)}  if ε_e=-1`。

★头寸腿 `targetLeg` = `(声部索引 ν(e), 多/空侧 = ε_e, 单位数 s_e)`——这是 §一 P^sep 头寸坐标
`q^±_{v}` 的「目标值」表达（C25 多空独立腿）。本文件**不**展开 P^sep 完整直积（W6 的活），
只承载 `LegTarget(e)` 这一个元素的目标腿三元组。多腿（`ε=+1`）写 `side=long`、空腿（`ε=-1`）
写 `side=short`——直接由 `e.eps` 决定，定义性满足 §三 `σ_{ν(e)}=ε_e`。-/

/--
单元素的目标头寸腿 `LegTarget(e)`（C41 §十三 line 1480）。

- `voice : Nat`（`ν(e)`，腿归属的声部坐标，来自配属 `assign`）。
- `side : Side`（多/空侧 = `ε_e`：`ε=+1↦long=多头腿 e^+`、`ε=-1↦short=空头腿 e^-`）。
- `units : Nat`（`s_e`，目标单位数，来自配属 `assign`）。

★这是「q̄ 在元素 e 这条腿上的目标值」。`side := e.eps` 使 §三 `σ_{ν(e)}=ε_e` 定义性成立。
-/
structure TargetLeg where
  voice : Nat
  side : Side
  units : Nat
deriving Repr, DecidableEq

/--
`LegTarget_Θ(e)`（C41 §十三 line 1480）：元素 e 在配属 `assign` 下的目标头寸腿。

ε_e=+1（long）⟹ 多头腿 `s_e e^+_{ν(e)}`；ε_e=-1（short）⟹ 空头腿 `s_e e^-_{ν(e)}`。
两分支统一为「腿方向 := e.eps」——故无需 if 分叉，`side` 直接取 `e.eps`（C41 两分支的统一形式）。
-/
def legTarget (assign : SyntaxElement -> LegAssign) (e : SyntaxElement) : TargetLeg :=
  { voice := (assign e).nu, side := e.eps, units := (assign e).s }

/-- 腿方向恒等元素方向（§三 C28 `σ_{ν(e)}=ε_e` 的定义性满足）。 -/
theorem legTarget_side_eq_eps (assign : SyntaxElement -> LegAssign) (e : SyntaxElement) :
    (legTarget assign e).side = e.eps :=
  rfl

/-- 腿声部 = ν(e)（C41：腿归属 `ν(e)`）。 -/
theorem legTarget_voice_eq_nu (assign : SyntaxElement -> LegAssign) (e : SyntaxElement) :
    (legTarget assign e).voice = (assign e).nu :=
  rfl

/-- 腿单位数 = s_e（C41：目标单位数 `s_e`）。 -/
theorem legTarget_units_eq_s (assign : SyntaxElement -> LegAssign) (e : SyntaxElement) :
    (legTarget assign e).units = (assign e).s :=
  rfl

/-! ## §E 目标头寸 q̄^± 的多空分量指示（C32 §五 line 1478）

§五 boxed：`q̄^+_{ν(e),t+1} = s_e · 1[e∈Ã_{t+1}] · 1[ε_e=+1]`、
`q̄^-_{ν(e),t+1} = s_e · 1[e∈Ã_{t+1}] · 1[ε_e=-1]`。

★给定「e 在激活集中」（`inActive : Bool`，由 §F 的 `targetActiveSet` 提供），C32 给出多空两分量：
多头分量 `q̄^+` 仅当 `e∈Ã ∧ ε_e=+1` 取 `s_e`，否则 0；空头分量 `q̄^-` 仅当 `e∈Ã ∧ ε_e=-1` 取 `s_e`，
否则 0。多空互斥（同一 e 至多一个分量非零）——这是分账本「不净额抵消」语义的元素级体现。-/

/-- 多头目标分量 `q̄^+_{ν(e)}`（C32 §五）：`s_e · 1[e∈Ã] · 1[ε_e=+1]`。 -/
def targetLong (assign : SyntaxElement -> LegAssign) (inActive : Bool)
    (e : SyntaxElement) : Nat :=
  if inActive ∧ e.eps = Side.long then (assign e).s else 0

/-- 空头目标分量 `q̄^-_{ν(e)}`（C32 §五）：`s_e · 1[e∈Ã] · 1[ε_e=-1]`。 -/
def targetShort (assign : SyntaxElement -> LegAssign) (inActive : Bool)
    (e : SyntaxElement) : Nat :=
  if inActive ∧ e.eps = Side.short then (assign e).s else 0

/--
多空互斥（C32 + §四 line 1474「多空不净额抵消」的元素级形式）：
对任意元素 e，`q̄^+` 与 `q̄^-` 至多一个非零（同一元素一条腿，方向单一）。
-/
theorem target_long_short_exclusive
    (assign : SyntaxElement -> LegAssign) (inActive : Bool) (e : SyntaxElement) :
    targetLong assign inActive e = 0 ∨ targetShort assign inActive e = 0 := by
  unfold targetLong targetShort
  cases h : e.eps with
  | long => right; simp
  | short => left; simp

/--
未激活元素目标头寸为零（C32：`1[e∈Ã]=0 ⟹ q̄^±=0`）——`inActive=false` 时多空分量皆 0。
这是「先关闭结束元素」的目标侧体现：不在激活集的元素目标仓位归零。
-/
theorem target_zero_when_inactive
    (assign : SyntaxElement -> LegAssign) (e : SyntaxElement) :
    targetLong assign false e = 0 ∧ targetShort assign false e = 0 := by
  constructor <;> simp [targetLong, targetShort]

/--
激活元素目标头寸 = legTarget 的对应分量（C32 ↔ C41 一致性）：
当 `inActive=true` 时，非零的那个目标分量等于 `legTarget e` 的 `units`（=`s_e`），
且非零分量的「侧」与 `legTarget e` 的 `side`（=`ε_e`）一致。多头：ε=long ⟹ `q̄^+=s_e`、`q̄^-=0`。
-/
theorem target_active_long
    (assign : SyntaxElement -> LegAssign) (e : SyntaxElement) (hlong : e.eps = Side.long) :
    targetLong assign true e = (legTarget assign e).units
      ∧ targetShort assign true e = 0 := by
  refine ⟨?_, ?_⟩
  · simp [targetLong, legTarget, hlong]
  · simp [targetShort, hlong]

/-- 激活空头元素：ε=short ⟹ `q̄^-=s_e`、`q̄^+=0`（C32 ↔ C41 一致性，空头侧）。 -/
theorem target_active_short
    (assign : SyntaxElement -> LegAssign) (e : SyntaxElement) (hshort : e.eps = Side.short) :
    targetShort assign true e = (legTarget assign e).units
      ∧ targetLong assign true e = 0 := by
  refine ⟨?_, ?_⟩
  · simp [targetShort, legTarget, hshort]
  · simp [targetLong, hshort]

/-! ## §F 目标激活集 `Ã_{t+1} = (A_t \ D_t) ∪ B_t`（C31 §五 line 1478）

§五 boxed：`B_t={e:λ_e=t}`（新开始元素）、`D_t={e:ρ_e=t}`（结束元素）、
`Ã_{t+1}=(A_t\D_t)∪B_t`（先关闭结束元素，再开启新开始元素）。

★用 `List SyntaxElement`（元素有 `DecidableEq`）表示元素集合 A_t；`B_t`/`D_t` 由当前时刻 t 对
全元素流 `allElems` 按端点过滤得到。「先关后开」= 先从 A_t 滤除 D_t（`A.filter (∉ D)`），
再 append B_t。`∪` 用 `++`（List 的并；元素去重不影响指示函数 1[e∈Ã] 的语义——`e∈(l++m) ↔ e∈l∨e∈m`）。-/

/-- `B_t = {e : λ_e = t}` 新开始元素集（C31 §五）：全元素流中左端点 = t 的元素。 -/
def startingSet (allElems : List SyntaxElement) (t : Int) : List SyntaxElement :=
  allElems.filter (fun e => decide (e.lambda = t))

/-- `D_t = {e : ρ_e = t}` 结束元素集（C31 §五）：全元素流中右端点 = t 的元素。 -/
def endingSet (allElems : List SyntaxElement) (t : Int) : List SyntaxElement :=
  allElems.filter (fun e => decide (e.rho = t))

/--
目标激活集 `Ã_{t+1} = (A_t \ D_t) ∪ B_t`（C31 §五 line 1478，先关后开）。

`active`（A_t，当前活动元素）先滤除结束元素 `ending`（D_t），再 append 新开始元素 `starting`（B_t）。
**关闭在前**（`filter (∉ ending)`）**开启在后**（`++ starting`）——严格对应 §五「先关后开」语序。
-/
def targetActiveSet
    (active ending starting : List SyntaxElement) : List SyntaxElement :=
  active.filter (fun e => decide (e ∉ ending)) ++ starting

/--
激活集成员判据（C31 的指示函数 `1[e∈Ã_{t+1}]` 语义）：
`e ∈ Ã_{t+1} ↔ (e ∈ A_t ∧ e ∉ D_t) ∨ e ∈ B_t`——先关（在 A 且不在 D）后开（在 B）。
这是 C32 中 `1[e∈Ã]` 的可判定展开，下游 W11/W12 引此判定元素目标头寸是否非零。
-/
theorem mem_targetActiveSet
    (active ending starting : List SyntaxElement) (e : SyntaxElement) :
    e ∈ targetActiveSet active ending starting ↔
      ((e ∈ active ∧ e ∉ ending) ∨ e ∈ starting) := by
  unfold targetActiveSet
  rw [List.mem_append, List.mem_filter]
  simp

/--
「先关闭结束元素」的代数体现（C31 §五）：若 e 是结束元素（`e ∈ ending = D_t`）且不是新开始元素
（`e ∉ starting = B_t`），则 e 不在目标激活集 `Ã_{t+1}`——结束元素被关闭（目标头寸归零，见
`target_zero_when_inactive`）。这是 §九定理3「t=ρ_e∈D_t ⟹ e∉Ã ⟹ 腿关闭」的目标集侧前置。
-/
theorem ending_closed
    (active ending starting : List SyntaxElement) (e : SyntaxElement)
    (hEnd : e ∈ ending) (hNotStart : e ∉ starting) :
    e ∉ targetActiveSet active ending starting := by
  rw [mem_targetActiveSet]
  rintro (⟨_, hNotEnd⟩ | hStart)
  · exact hNotEnd hEnd
  · exact hNotStart hStart

/--
「新开始元素被开启」的代数体现（C31 §五）：若 e 是新开始元素（`e ∈ starting = B_t`），则 e 必在
目标激活集 `Ã_{t+1}`——无条件开启（即使它同时在 D_t 中，B_t 的 append 也覆盖）。这是 §九定理3
「t=λ_e∈B_t ⟹ e∈Ã ⟹ 腿按 ε_e 开」的目标集侧前置。
-/
theorem starting_opened
    (active ending starting : List SyntaxElement) (e : SyntaxElement)
    (hStart : e ∈ starting) :
    e ∈ targetActiveSet active ending starting := by
  rw [mem_targetActiveSet]
  exact Or.inr hStart

/--
「保持」的代数体现（C31 §五，§九定理3 内部时刻分支）：若 e 当前活动（`e∈A_t`）、不是结束元素
（`e∉D_t`），则 e 仍在 `Ã_{t+1}`——既未关闭也无需重开，目标头寸保持。对应 §九「内部时刻
t∈(λ_e,ρ_e) e∉D_t ⟹ e∈A_t⟹e∈Ã 腿保持开」。
-/
theorem active_held
    (active ending starting : List SyntaxElement) (e : SyntaxElement)
    (hActive : e ∈ active) (hNotEnd : e ∉ ending) :
    e ∈ targetActiveSet active ending starting := by
  rw [mem_targetActiveSet]
  exact Or.inl ⟨hActive, hNotEnd⟩

/-! ## §G 全定义策略目标头寸 `q̄_Θ = LegTarget((A\D)∪B)`（C41 §十三 line 1480 总形式）

§十三 boxed：`q̄_Θ(x_t) = LegTarget_Θ((A_t\D_t)∪B_t)`——目标头寸 = 对目标激活集中每个元素取
`LegTarget`。本文件给出「逐元素目标头寸映射」`strategyTargetLegs`：把 `Ã_{t+1}` 中每个元素映为
其 `legTarget`，得到目标头寸腿的列表（=分账本目标头寸 q̄_Θ 的腿分解）。-/

/--
全定义策略目标头寸 `q̄_Θ`（C41 §十三 line 1480 总形式）：目标激活集中每个元素的 `legTarget` 列表。

`strategyTargetLegs assign active ending starting = (Ã_{t+1}).map (legTarget assign)`——把
`q̄_Θ = LegTarget((A\D)∪B)` 实现为「对激活集逐元素生成目标腿」。每个目标腿带 `(ν(e), ε_e, s_e)`，
是分账本目标头寸的腿级分解（多空独立坐标，C25）。
-/
def strategyTargetLegs
    (assign : SyntaxElement -> LegAssign)
    (active ending starting : List SyntaxElement) : List TargetLeg :=
  (targetActiveSet active ending starting).map (legTarget assign)

/--
目标头寸腿来源于激活集（C41 ↔ C31 一致性）：每条目标腿对应一个激活集中的元素，且该腿 = 该元素的
`legTarget`。这是 §十三 `q̄_Θ=LegTarget((A\D)∪B)` 的「腿可追溯到激活元素」性质，下游 W11
「覆盖可行 ⟹ 最终仓位=目标仓位」用此把仓位逐腿对应回元素。
-/
theorem mem_strategyTargetLegs
    (assign : SyntaxElement -> LegAssign)
    (active ending starting : List SyntaxElement) (leg : TargetLeg) :
    leg ∈ strategyTargetLegs assign active ending starting ↔
      ∃ e, e ∈ targetActiveSet active ending starting ∧ leg = legTarget assign e := by
  unfold strategyTargetLegs
  rw [List.mem_map]
  constructor
  · rintro ⟨e, he, rfl⟩; exact ⟨e, he, rfl⟩
  · rintro ⟨e, he, rfl⟩; exact ⟨e, he, rfl⟩

/--
目标头寸腿的方向忠实（C41 ↔ C28 ↔ C32 三方一致）：每条目标腿的方向 = 其源元素的 ε_e。
合并 `legTarget_side_eq_eps`（腿方向=ε_e）与 `mem_strategyTargetLegs`（腿源于激活元素）——
保证目标头寸的每条腿方向严格由元素方向决定（§三 `σ_{ν(e)}=ε_e`），无方向歧义。
-/
theorem strategyTargetLegs_side_faithful
    (assign : SyntaxElement -> LegAssign)
    (active ending starting : List SyntaxElement) (leg : TargetLeg)
    (hmem : leg ∈ strategyTargetLegs assign active ending starting) :
    ∃ e, e ∈ targetActiveSet active ending starting ∧ leg.side = e.eps := by
  rw [mem_strategyTargetLegs] at hmem
  obtain ⟨e, he, rfl⟩ := hmem
  exact ⟨e, he, legTarget_side_eq_eps assign e⟩

end NewChanlun.Origin.SeparateStrategyTarget
