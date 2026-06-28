/-
  Origin/SeparateEat.lean — 唯一头寸腿单射 ν:E→V + 分账本吃到 Eat^sep + 毛收益 G^sep>0（工作单元 W8）

  ════════════════════════════════════════════════════════════════════════
  ## 本文件做什么：分账本主线核心覆盖谓词（C28 / C29 / C30）

  权威源（唯一）：`.chanlun/specs/2026-06-28-complete-classification-pdf-extract.md`
  §B 表 C28/C29/C30 行 + §三/§四（23 页权威 PDF 页12–13）+ §九/§十（页17–19）+ §C 完备性
  （互斥=disjoint）+ §D W8 行 + §F。本文件是分账本主线（§一–§十四）的**核心覆盖谓词层**——
  把 W6（P^sep 头寸空间）+ W7（语法元素集 E）接成「每元素一规范头寸腿 + 腿覆盖整操作区间 +
  方向一致毛收益为正」三件套，供 W12（定理3）/W13（自相似递归）/W14（最终定理）直接引用。

  ### C28 · 唯一头寸腿映射 ν:E→V（PDF §三 页12–13）
      ν: E → V 单射，e ≠ e′ ⟹ ν(e) ≠ ν(e′)（每语法元素有自己的规范头寸腿）
      目标方向 σ_{ν(e)} = ε_e（ε_e=+1 多头腿 / ε_e=-1 空头腿）
      目标单位数 s_e > 0；同股数双开 s_e = s_{par(e)}，一般 s_e = κ_e·s_{par(e)}, 0<κ_e≤1
  **这是互斥归属**（连 W7 `ElementUniquePartition.disjoint`）——单射 ⟹ 不同元素不共用同一规范腿。

  ### C29 · 分账本吃到 Eat^sep(e)（PDF §四 页13）
      Eat^sep(e) ⟺ ∀t∈[λ_e,ρ_e):
        (ε_e=+1 ⟹ q⁺_{ν(e),t}=s_e ∧ q⁻_{ν(e),t}=0)
        ∧ (ε_e=-1 ⟹ q⁺_{ν(e),t}=0 ∧ q⁻_{ν(e),t}=s_e)
  向上元素由多头腿覆盖（legLong s_e）、向下由空头腿覆盖（legShort s_e）、覆盖发生在该元素
  **整操作区间** [λ_e,ρ_e)、多空头寸**不净额抵消**（引 W6 `legLong`/`legShort`，及不删父引理
  `hedged_leg_nonzero_but_net_zero` 坐实双开腿在 P^sep 非零）。

  ### C30 · 分账本毛收益 G^sep_e > 0（PDF §四 页13 / §十 页18–19）
      G^sep_e = s_e · ε_e · (P_{ρ_e} − P_{λ_e})，由方向一致定义 ⟹ G^sep_e > 0
  用 W7 `DirectionConsistent`（ε_e·ΔP_e>0）+ s_e>0 ⟹ G^sep_e = s_e·(ε_e·ΔP_e) > 0。L0 代数恒等。

  ════════════════════════════════════════════════════════════════════════
  ## ★canonical 统一（no-patch / no-workaround，关键）

  **E 用 W7 的 `SyntaxElement`/`ElementSet`（唯一 canonical）**。本文件**不**复制、不桥接到 W9
  （`Origin.SeparateStrategyTarget`）内联的那份最小 SyntaxElement——只用 W7 的（W9 内联版须在
  W14 集成前 re-anchor 到 W7，那是 Lead 跟踪的 cleanup，非本文件的活）。腿坐标用 W6 的
  `Leg`/`SepPosition`（`legLong`/`legShort`/`legZero`）。声部方向域用 Origin canonical `Side`
  （`SourceAxioms.Side`，long/short）——与 W7 元素方向 `Direction`（up/down）经 `dirToSide` 对接。

  ### 方向对接桥 dirToSide（C28 `σ_{ν(e)}=ε_e` 的形式载体）
  W7 元素方向是 `Direction`（up/down），W6/声部方向是 `Side`（long/short）。C28 要求
  `σ_{ν(e)}=ε_e`——声部方向 = 元素方向。两域经 `dirToSide : Direction → Side`
  （up ↦ long、down ↦ short）对接。这不是新概念，是把 PDF「向上元素→多头腿、向下元素→空头腿」
  的方向语义在两个 canonical 方向域之间落地（`directionMatchesSide` 谓词承载 σ_{ν(e)}=ε_e）。

  ════════════════════════════════════════════════════════════════════════
  ## 认识论等级（formalization-validity-domain / 231号，强制标注）

  **全文件 L0**（纯结构/定义/代数恒等，零数据依赖）。`lake env lean` 通过 =
  - C28 单射 ν 是 `Function.Injective` 的结构定义（core 等价谓词，core-only 无 Mathlib）；
  - C29 Eat^sep 是「头寸函数在区间内逐 t 等于规范腿」的语法覆盖谓词；
  - C30 G^sep>0 是 `s_e>0 ∧ ε_e·ΔP_e>0 ⟹ s_e·ε_e·ΔP_e>0` 的代数恒等（`Int` 正乘正为正）。

  ★**Eat^sep = 语法覆盖谓词，G^sep>0 = 代数恒等，NOT L2 实盘每笔盈利**。PDF §十二（C40）/
  §十四（C42）显式否定「净账户每笔盈利」——双开 (Q,Q) 在净额映射下退化为 0（W6
  `hedged_leg_net_zero`）。本文件**只**证 L0 分账本声部级覆盖；**严禁**声明分账本「在实盘有效」
  或「每笔净盈利」——那是 L2 EmpiricalDomain，本文件不提供也不可由 L0 推出。有效域诚实标注：
  「严格要求当下可判定」（W7 结构性承载，非事后端点）是内部操作语义假设，非现实市场假设。

  ════════════════════════════════════════════════════════════════════════
  ## 依赖方向（单向无环）+ 验证 + 隔离声明

  SeparateEat → { Origin.SyntaxElement（W7：SyntaxElement/ElementSet/dirSign/priceDelta/
                  DirectionConsistent/Direction.sign + ElementUniquePartition.disjoint）,
                  Origin.SeparateLedger（W6：Leg/SepPosition/legLong/legShort/legZero +
                  不删父引理 hedged_leg_nonzero_but_net_zero）}。
  `Side` 经 SyntaxElement → ChanlunElements → SourceAxioms 间接可用（W6 也 standalone 用 Nat，
  本文件方向域 Side 取自 SourceAxioms，不新建方向枚举）。不碰其他文件（owner 互斥）。
  验证：`cd formal && lake env lean Origin/SeparateEat.lean`。禁 sorry/admit/axiom。简体中文。

  谱系：C28/C29/C30（PDF §三/§四/§九/§十）→ W6（P^sep 根）+ W7（语法元素 E）→ 本文件 W8
        （分账本核心覆盖谓词）→ 下游 W12（定理3）/W13（自相似递归）/W14（最终定理）。
        C40/C42 净收益不可能（双开净额退化）= 本文件有效域上界（声部级覆盖，非净资产盈利）。
-/

import Origin.SyntaxElement
import Origin.SeparateLedger

namespace NewChanlun.Origin.SeparateEat

open NewChanlun.Origin
open NewChanlun.Origin.SeparateLedger

/-! ════════════════════════════════════════════════════════════════════════
  ## §0 方向对接桥 dirToSide（C28 `σ_{ν(e)}=ε_e` 的方向域对接）

  W7 元素方向 `Direction`（up/down）与声部/头寸方向 `Side`（long/short）的对接：
  向上元素（up）→ 多头腿（long），向下元素（down）→ 空头腿（short）。这是 PDF「向上元素由
  多头腿覆盖、向下由空头腿覆盖」（§四）在两个 canonical 方向域之间的桥——不新建方向枚举，
  复用 SourceAxioms 的 `Side` 与 W7 的 `Direction`。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★方向对接桥 `dirToSide`（L0，C28）：元素方向 ε_e（Direction）↦ 声部方向 σ（Side）。
    up ↦ long（向上元素→多头腿）、down ↦ short（向下元素→空头腿）。 -/
def dirToSide : Direction → Side
  | Direction.up => Side.long
  | Direction.down => Side.short

/-- ★方向对接保符号（L0）：dirToSide 与 Direction.sign 一致——
    多头侧 ⟺ ε=+1、空头侧 ⟺ ε=-1。坐实方向桥不漂移（与 W7 `Direction.sign` 同口径）。 -/
theorem dirToSide_long_iff_sign_pos (d : Direction) :
    dirToSide d = Side.long ↔ d.sign = 1 := by
  cases d <;> simp [dirToSide, Direction.sign]

theorem dirToSide_short_iff_sign_neg (d : Direction) :
    dirToSide d = Side.short ↔ d.sign = -1 := by
  cases d <;> simp [dirToSide, Direction.sign]

/-- ★方向对接单射（L0）：dirToSide 不同方向映到不同侧（up/down → long/short 双射）。 -/
theorem dirToSide_injective {d₁ d₂ : Direction} (h : dirToSide d₁ = dirToSide d₂) :
    d₁ = d₂ := by
  cases d₁ <;> cases d₂ <;> first | rfl | (simp [dirToSide] at h)

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 C28 · 唯一头寸腿映射 ν:E→V（单射 + σ_{ν(e)}=ε_e + s_e>0）

  PDF §三：ν:E→V 单射（每元素一规范头寸腿），σ_{ν(e)}=ε_e（声部方向=元素方向），s_e>0
  （目标单位数正）。本节把这三件封装为 `LegAssignment`——给定声部类型 V，承载 ν/方向读出 σ/
  单位数 s 三个函数，并要求单射 + 方向一致 + 单位数正。

  ★V 抽象：ν 值域 V 是抽象声部类型（与 W7 `VoiceTree (V : Type)` 同口径，声部类型参数化）。
  声部方向由 `voiceSide : V → Side` 读出（对接 `VoiceTree.side`，但不强行 import 整个 VoiceTree
  结构——只取「声部→方向」这一读出函数，保持本文件聚焦覆盖谓词）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★唯一头寸腿赋值 `LegAssignment V`（L0，C28 核心）—— ν:E→V 单射 + σ_{ν(e)}=ε_e + s_e>0。

  - `nu : SyntaxElement → V`：唯一头寸腿映射 ν（每语法元素的规范头寸腿所在声部）。
  - `voiceSide : V → Side`：声部方向读出 σ_v（对接 VoiceTree.side，long/short）。
  - `units : SyntaxElement → Nat`：目标单位数 s_e（手数/股数，R≥0 整数最小单位，对齐 W6 Nat 口径）。
  - `nu_injective`：**单射** e≠e′ ⟹ ν(e)≠ν(e′)（这是 C28 的「每元素一腿」互斥归属）。
  - `dir_consistent`：**方向一致** σ_{ν(e)}=ε_e，即 `voiceSide (nu e) = dirToSide e.direction`。
  - `units_pos`：**单位数正** s_e>0（C28 `s_e>0`，下游 G^sep>0 的非退化前提）。

  ★L0：纯结构约束（单射/方向一致/正），无 Θ 参数、不依赖数据。
-/
structure LegAssignment (V : Type) where
  nu : SyntaxElement → V
  voiceSide : V → Side
  units : SyntaxElement → Nat
  /-- C28 单射：不同元素映到不同声部腿（每元素一规范腿，互斥归属）。 -/
  nu_injective : ∀ e e', nu e = nu e' → e = e'
  /-- C28 方向一致 σ_{ν(e)}=ε_e：规范腿声部方向 = 元素方向（经 dirToSide 桥）。 -/
  dir_consistent : ∀ e, voiceSide (nu e) = dirToSide e.direction
  /-- C28 单位数正 s_e>0：每元素目标单位数严格为正。 -/
  units_pos : ∀ e, 0 < units e

namespace LegAssignment

variable {V : Type}

/-- ★ν 单射（L0，core 等价 `Function.Injective`）：core-only 环境无 Mathlib，用结构性单射谓词
    `∀ e e', nu e = nu e' → e = e'`。这正是 `Function.Injective nu` 的展开形式——同一命题。 -/
theorem nu_inj (L : LegAssignment V) : Function.Injective L.nu :=
  fun e e' h => L.nu_injective e e' h

/-- ★不同元素 ⟹ 不同规范腿（L0，单射的逆否，C28 互斥归属的直接陈述）：
    e ≠ e′ ⟹ ν(e) ≠ ν(e′)。这是 PDF §三「每语法元素有自己的规范头寸腿」的形式。
    连 W7 `ElementUniquePartition.disjoint`：元素侧无重叠 + 腿侧单射 = 完备性「互斥」两面。 -/
theorem distinct_elements_distinct_legs (L : LegAssignment V)
    {e e' : SyntaxElement} (h : e ≠ e') : L.nu e ≠ L.nu e' :=
  fun heq => h (L.nu_injective e e' heq)

/-- ★规范腿方向 = 元素方向（L0，C28 σ_{ν(e)}=ε_e 的展开）：向上元素 ⟹ 规范腿是多头侧，
    向下元素 ⟹ 规范腿是空头侧。坐实「向上元素由多头腿覆盖、向下由空头腿覆盖」（PDF §四）。 -/
theorem voiceSide_long_of_up (L : LegAssignment V) {e : SyntaxElement}
    (h : e.direction = Direction.up) : L.voiceSide (L.nu e) = Side.long := by
  rw [L.dir_consistent e, h]; rfl

theorem voiceSide_short_of_down (L : LegAssignment V) {e : SyntaxElement}
    (h : e.direction = Direction.down) : L.voiceSide (L.nu e) = Side.short := by
  rw [L.dir_consistent e, h]; rfl

end LegAssignment

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 C29 · 分账本吃到 Eat^sep(e)（规范腿覆盖整操作区间）

  PDF §四：Eat^sep(e) ⟺ ∀t∈[λ_e,ρ_e): 向上元素规范腿 = legLong s_e（q⁺=s_e,q⁻=0）/
  向下元素规范腿 = legShort s_e（q⁺=0,q⁻=s_e）。给定头寸函数 `q : V → Index → Leg`（每声部
  每时刻一条腿），Eat^sep 谓词刻画「规范腿 ν(e) 在 e 整操作区间内方向正确、单位数=s_e」。

  ★规范腿目标形状 `canonicalLeg`：把方向 ε_e + 单位数 s_e 合成应有的腿——up→legLong s_e、
  down→legShort s_e。Eat^sep = 「∀t∈[λ_e,ρ_e), q(ν(e),t) = canonicalLeg」。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★规范腿目标形状 `canonicalLeg`（L0，C29）：元素 e 的应有头寸腿——
    up（ε=+1）↦ legLong s_e = (s_e, 0)（多头腿）、down（ε=-1）↦ legShort s_e = (0, s_e)（空头腿）。
    这是 PDF §四「向上元素 q⁺=s_e∧q⁻=0、向下元素 q⁺=0∧q⁻=s_e」的腿形式载体。 -/
def canonicalLeg (e : SyntaxElement) (s : Nat) : Leg :=
  match e.direction with
  | Direction.up => legLong s
  | Direction.down => legShort s

/-- ★规范腿多头形状（L0）：向上元素的规范腿是纯多头 (s,0)（q⁺=s ∧ q⁻=0）。 -/
theorem canonicalLeg_up (e : SyntaxElement) (s : Nat) (h : e.direction = Direction.up) :
    canonicalLeg e s = legLong s := by
  simp [canonicalLeg, h]

/-- ★规范腿空头形状（L0）：向下元素的规范腿是纯空头 (0,s)（q⁺=0 ∧ q⁻=s）。 -/
theorem canonicalLeg_down (e : SyntaxElement) (s : Nat) (h : e.direction = Direction.down) :
    canonicalLeg e s = legShort s := by
  simp [canonicalLeg, h]

/-- ★规范腿 q⁺/q⁻ 分量（L0，C29 逐分量）：向上元素 q⁺=s∧q⁻=0、向下元素 q⁺=0∧q⁻=s。
    直接坐实 PDF §四的逐坐标条件（多头腿/空头腿在各自坐标记 s、另一坐标记 0）。 -/
theorem canonicalLeg_components (e : SyntaxElement) (s : Nat) :
    (e.direction = Direction.up → (canonicalLeg e s).qPlus = s ∧ (canonicalLeg e s).qMinus = 0)
    ∧ (e.direction = Direction.down → (canonicalLeg e s).qPlus = 0 ∧ (canonicalLeg e s).qMinus = s) := by
  constructor
  · intro h; rw [canonicalLeg_up e s h]; exact ⟨rfl, rfl⟩
  · intro h; rw [canonicalLeg_down e s h]; exact ⟨rfl, rfl⟩

/--
  ★★分账本吃到 `EatSep`（L0，C29 核心谓词）—— `Eat^sep(e)` ⟺ 规范腿覆盖整操作区间。

  给定腿赋值 `L : LegAssignment V` 与头寸函数 `q : V → Index → Leg`（声部 v 在时刻 t 的头寸腿），
      EatSep L q e ⟺ ∀t∈[λ_e,ρ_e): q(ν(e), t) = canonicalLeg e s_e
  即元素 e 的规范声部腿 ν(e) 在 e 的**整操作区间** [λ_e,ρ_e)（半开，W7 `tickCovered` 口径）内，
  头寸恒等于 e 的规范腿形状（up→legLong s_e、down→legShort s_e）。

  ★这是「向上元素由多头腿覆盖、向下由空头腿覆盖、覆盖发生在整操作区间、多空不净额抵消」
  （PDF §四）的形式：腿是 P^sep 的 `Leg`（W6，两坐标独立），故双开不抵消（净额退化是 W6
  `hedged_leg_net_zero` 的事，Eat^sep 谓词在 P^sep 坐标上判定，不经净额映射）。
-/
def EatSep {V : Type} (L : LegAssignment V) (q : V → Index → Leg) (e : SyntaxElement) : Prop :=
  ∀ t : Index, e.tickCovered t → q (L.nu e) t = canonicalLeg e (L.units e)

/--
  ★Eat^sep ⟹ 区间内 q⁺/q⁻ 逐分量（L0，C29 展开）：若 `EatSep L q e` 且向上元素，则区间内
  每个 t 都有 q⁺_{ν(e),t}=s_e ∧ q⁻_{ν(e),t}=0（多头腿覆盖）；向下元素对称（q⁺=0∧q⁻=s_e）。
  这把 EatSep 的「腿相等」拆成 PDF §四原文的逐坐标条件，供下游 W12/W13 直接引用分量。
-/
theorem EatSep.up_components {V : Type} {L : LegAssignment V} {q : V → Index → Leg}
    {e : SyntaxElement} (heat : EatSep L q e) (hdir : e.direction = Direction.up)
    {t : Index} (ht : e.tickCovered t) :
    (q (L.nu e) t).qPlus = L.units e ∧ (q (L.nu e) t).qMinus = 0 := by
  rw [heat t ht, canonicalLeg_up e (L.units e) hdir]
  exact ⟨rfl, rfl⟩

theorem EatSep.down_components {V : Type} {L : LegAssignment V} {q : V → Index → Leg}
    {e : SyntaxElement} (heat : EatSep L q e) (hdir : e.direction = Direction.down)
    {t : Index} (ht : e.tickCovered t) :
    (q (L.nu e) t).qPlus = 0 ∧ (q (L.nu e) t).qMinus = L.units e := by
  rw [heat t ht, canonicalLeg_down e (L.units e) hdir]
  exact ⟨rfl, rfl⟩

/--
  ★Eat^sep 规范腿非零（L0，C29 ∧ 双开不抵消的桥）：因 s_e>0（`units_pos`），区间内每个 t 的
  规范腿都**非零腿**（≠ legZero）——向上元素多头分量 q⁺=s_e>0、向下元素空头分量 q⁻=s_e>0。

  这连 W6 不删父：规范腿在 P^sep 是非零状态（即便父子双开，两腿在各自坐标都非零，
  净额退化不抹掉 P^sep 的非零性——`hedged_leg_nonzero_but_net_zero`）。
-/
theorem EatSep.canonicalLeg_nonzero {V : Type} (L : LegAssignment V) (e : SyntaxElement) :
    canonicalLeg e (L.units e) ≠ legZero := by
  have hs : 0 < L.units e := L.units_pos e
  cases hdir : e.direction with
  | up =>
    rw [canonicalLeg_up e (L.units e) hdir]
    intro h
    have : (legLong (L.units e)).qPlus = (legZero).qPlus := by rw [h]
    simp only [legLong, legZero] at this
    omega
  | down =>
    rw [canonicalLeg_down e (L.units e) hdir]
    intro h
    have : (legShort (L.units e)).qMinus = (legZero).qMinus := by rw [h]
    simp only [legShort, legZero] at this
    omega

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 C30 · 分账本毛收益 G^sep_e > 0（方向一致 + 单位数正 ⟹ 正收益）

  PDF §四 页13 / §十 页18–19：G^sep_e = s_e·ε_e·(P_{ρ_e}−P_{λ_e})，由元素方向一致定义
  ε_e·ΔP_e>0 + s_e>0 ⟹ G^sep_e>0。用 W7 `DirectionConsistent`（ε_e·ΔP_e>0）+ `units_pos`。
  L0 代数恒等（Int 正乘正为正）。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★分账本毛收益 `gSep`（L0，C30）：G^sep_e = s_e·ε_e·(P_ρ−P_λ) = s_e · dirSign(e) · priceDelta(e)。
    用 W7 `dirSign`（ε_e∈{+1,-1}）+ `priceDelta`（ΔP_e=P_ρ−P_λ）。s_e 取自腿赋值 units。 -/
def gSep {V : Type} (L : LegAssignment V) (e : SyntaxElement) (P : Index → Tick) : Tick :=
  (L.units e : Int) * (e.dirSign * e.priceDelta P)

/--
  ★★分账本毛收益为正 `gSep_pos`（L0，C30 核心定理）—— 方向一致 ⟹ G^sep_e > 0。

  前提：元素方向一致 `DirectionConsistent e P`（ε_e·ΔP_e>0，W7 §2 方向语义）。
  结论：G^sep_e = s_e·(ε_e·ΔP_e) > 0（s_e>0 由 `units_pos`，正乘正为正）。

  ★L0 代数恒等：这是「空头算一个、多头算一个」严格吃笔定义下的正向收益（PDF §十结论）。
  **NOT L2 实盘盈利**——ε_e·ΔP_e>0 是元素方向语义（W7 §二「方向由端点确定」），非市场预测；
  双开父子腿的净账户收益相加=0（W6 `hedged_leg_net_zero`，C40 不可能定理），声部级毛收益 G^sep>0
  与净资产级盈利是两个有效域（PDF §十二/§十四显式区分）。
-/
theorem gSep_pos {V : Type} (L : LegAssignment V) (e : SyntaxElement) (P : Index → Tick)
    (hcons : DirectionConsistent e P) :
    0 < gSep L e P := by
  -- DirectionConsistent e P 即 e.dirSign * e.priceDelta P > 0（W7 定义展开）
  have hfactor : 0 < e.dirSign * e.priceDelta P := hcons
  -- s_e > 0（units_pos），整数提升后仍正
  have hs : (0 : Int) < (L.units e : Int) := by
    have := L.units_pos e; exact_mod_cast this
  -- G^sep = s_e * (ε_e·ΔP_e)，正乘正为正
  unfold gSep
  exact Int.mul_pos hs hfactor

/--
  ★毛收益方向分解 `gSep_sign_split`（L0，C30 二分穷尽，对照 PDF §十二/§二十"空头吃跌/多头吃涨"）：
  在方向一致下，G^sep_e>0 来自两种情形——向上元素（ε=+1）端点上涨（ΔP>0）的多头腿收益、
  向下元素（ε=-1）端点下跌（ΔP<0）的空头腿收益。两者都使 G^sep=s_e·(ε_e·ΔP_e)>0。
  这坐实 PDF「空头腿吃下跌元素、多头腿吃上涨元素，各自独立坐标计数」的双向覆盖（非净额）。

  ★方向分解只取决于元素 e 与价格 P（ε_e 与 ΔP_e 的符号关系），不涉及腿赋值 L——故不列 L 参数
  （no-patch：代码能做什么就声明什么，无用参数删除）。下游用 `gSep_pos` 拿正收益，用本引理拿方向。
-/
theorem gSep_sign_split (e : SyntaxElement) (P : Index → Tick)
    (hcons : DirectionConsistent e P) :
    (e.direction = Direction.up ∧ 0 < e.priceDelta P)
    ∨ (e.direction = Direction.down ∧ e.priceDelta P < 0) :=
  (directionConsistent_iff e P).1 hcons

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 C28+C29+C30 合一：单元素分账本覆盖 + 正收益（下游 W12 直接引用接口）

  把 C28（单射规范腿）+ C29（覆盖整区间）+ C30（正收益）合成单元素的「被唯一规范腿吃到且
  正收益」结论——这是 W12（定理3）单元素版的核心引理形状：给定 Eat^sep + 方向一致，得
  规范腿唯一（单射）+ 覆盖区间 + G^sep>0 三件套。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★单元素分账本覆盖见证 `SepCovered`（L0，C28+C29+C30 合一）—— 元素 e 在分账本语义下
  「被唯一规范头寸腿吃到且产生正向毛收益」：

  - **覆盖**（C29）：`EatSep L q e`——规范腿 ν(e) 覆盖 e 整操作区间。
  - **正收益**（C30）：`0 < gSep L e P`——分账本毛收益为正。
  规范腿的**唯一性**（C28）由 `L.nu_injective` 保证（不同元素不共腿）——这是 SepCovered 的
  结构前提（封装在 `L : LegAssignment V` 内），故 SepCovered 持有即「∃!ν(e) 吃到 e」。
-/
structure SepCovered {V : Type} (L : LegAssignment V) (q : V → Index → Leg)
    (e : SyntaxElement) (P : Index → Tick) : Prop where
  /-- C29 覆盖：规范腿覆盖整操作区间（Eat^sep(e)）。 -/
  eaten : EatSep L q e
  /-- C30 正收益：分账本毛收益 G^sep_e > 0。 -/
  profit : 0 < gSep L e P

/--
  ★单元素覆盖构造 `sepCovered_of`（L0，W12 入口）：给定 Eat^sep(e) + 方向一致，构造单元素
  分账本覆盖见证（覆盖 + G^sep>0）。规范腿唯一性由 `L` 的单射字段自动持有。
  这是 W12 定理3（每元素被唯一规范腿吃到 + 正收益）单元素版的合成接口。
-/
theorem sepCovered_of {V : Type} (L : LegAssignment V) (q : V → Index → Leg)
    (e : SyntaxElement) (P : Index → Tick)
    (heat : EatSep L q e) (hcons : DirectionConsistent e P) :
    SepCovered L q e P :=
  { eaten := heat, profit := gSep_pos L e P hcons }

/--
  ★分账本覆盖 ⟹ 规范腿唯一（L0，C28 ∃! 的腿侧陈述）：若 e₁、e₂ 共用同一规范腿
  ν(e₁)=ν(e₂)，则 e₁=e₂（单射）。这是「∀e ∃!ν(e)」中唯一性的形式——每条规范腿至多对应
  一个元素，故元素↦规范腿是单射归属（连 W7 元素侧 disjoint，两面合成完备性「互斥」）。
-/
theorem sepCovered_unique_leg {V : Type} (L : LegAssignment V)
    {e₁ e₂ : SyntaxElement} (h : L.nu e₁ = L.nu e₂) : e₁ = e₂ :=
  L.nu_injective e₁ e₂ h

/-! ════════════════════════════════════════════════════════════════════════
  ## 交付总结（W8 工位，C28/C29/C30）

  本文件**证**（L0，machine-checked，无 sorry/admit/axiom，import 仅 W6/W7）：

  1. **C28 唯一头寸腿单射 ν:E→V**：`LegAssignment V`（nu/voiceSide/units + 单射/方向一致/正）+
     - `nu_inj`：ν 是 `Function.Injective`（core 等价谓词）。
     - `distinct_elements_distinct_legs`：e≠e′ ⟹ ν(e)≠ν(e′)（互斥归属，连 W7 disjoint）。
     - `voiceSide_long_of_up`/`voiceSide_short_of_down`：σ_{ν(e)}=ε_e（向上→多头/向下→空头）。
     - 方向桥 `dirToSide`（up↦long/down↦short）+ `dirToSide_injective`（方向域双射）。

  2. **C29 分账本吃到 Eat^sep(e)**：`canonicalLeg`（up→legLong s/down→legShort s）+ `EatSep`
     （规范腿覆盖整操作区间 [λ_e,ρ_e)）+
     - `EatSep.up_components`/`down_components`：区间内逐 t 的 q⁺/q⁻ 分量（PDF §四逐坐标条件）。
     - `EatSep.canonicalLeg_nonzero`：规范腿非零（s_e>0，连 W6 双开不抵消）。

  3. **C30 分账本毛收益 G^sep_e>0**：`gSep`（s_e·ε_e·ΔP_e）+
     - `gSep_pos`：**核心定理**——方向一致（ε_e·ΔP_e>0，W7）+ s_e>0 ⟹ G^sep_e>0（L0 代数恒等）。
     - `gSep_sign_split`：方向二分（向上吃涨/向下吃跌，各自独立坐标，非净额）。

  4. **C28+C29+C30 合一**：`SepCovered`（覆盖+正收益见证）+ `sepCovered_of`（W12 入口构造）+
     `sepCovered_unique_leg`（规范腿唯一性，C28 ∃! 腿侧）。

  本文件**不证**（formalization-validity-domain 诚实边界）：
  - ✗ 分账本在实盘有效 / 每笔净盈利（L0 结构；实盘有效是 L2 EmpiricalDomain，不声称）。
  - ✗ 净账户每笔盈利（C40/C42 显式否定——双开净额退化 G_net=0，W6 hedged_leg_net_zero）。
  - ✗ 自相似递归覆盖定理本身（W13 标的）/ 全定义策略唯一性（W10 标的）——本文件只建单元素
    覆盖谓词三件套（ν 单射 + Eat^sep + G^sep>0），递归归纳与策略全定义在下游工位。

  ★下游引用（W12 定理3 / W13 递归 / W14 最终定理直接引）：
  - C28 单射：`LegAssignment.nu_injective` / `nu_inj` / `distinct_elements_distinct_legs` /
    `sepCovered_unique_leg`（∃!ν(e) 唯一性）。
  - C29 Eat^sep：`EatSep` 谓词 / `EatSep.up_components` / `EatSep.down_components` /
    `EatSep.canonicalLeg_nonzero`（区间覆盖 + 逐分量）。
  - C30 G^sep>0：`gSep` / `gSep_pos`（正收益核心）/ `gSep_sign_split`（双向覆盖）。
  - 合一接口：`SepCovered` / `sepCovered_of`（W12 单元素覆盖见证构造入口）。

  谱系：C28/C29/C30（PDF §三/§四/§九/§十）→ W6（P^sep）+ W7（语法元素 E）→ 本文件 W8 →
        下游 W12/W13/W14。有效域上界 = 声部级覆盖（C40/C42 净资产盈利被否定）。
  ════════════════════════════════════════════════════════════════════════ -/

end NewChanlun.Origin.SeparateEat
