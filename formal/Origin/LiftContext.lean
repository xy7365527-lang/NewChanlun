/-
Origin/LiftContext.lean — #257 Lift / Context / 父中枢内反例

本文件只完成 #257 收窄后的第 1–3 项：

1. `Induces : Cert ℓ → ParentCarrier → BspClass → Prop`；
2. `ContextOne` / `ContextTwo` / `ContextThree` 的完整定义；
3. 判定 `InsideParentCenter W c` 能否排除三个 Context。

结论先行：原三类全排除命题为假。中枢内几何能推出 `brokeCenter=false` 与
`leftCenter=false`，却不能抹掉历史量 `afterTypeOne`。因此一类、三类可排除；若
`afterTypeOne=true`，二类反而成立。本文件给出机器检查反例，且把补
`afterTypeOne=false` 后才成立的加强定理明确命名为 `_of_not_afterTypeOne`。

认识论等级：
- 下列结构、定义、等价和反例均为 **L0 machine-checked / 定义**；
- `Cert.isTypeOne` 只编码“输入 g 已是次级别一类证书”这一 PDF 定义域；
- `ParentCarrier.endpoint` 是上级分类器已给出的端点判据；本文件不冒充从行情识别
  `afterTypeOne`，也不证明现有前序折叠等于完整市场历史识别（该识别仍是上游责任）；
- `Cert.notUsedAtTime` 只消费共享 `M_t` 的外部时点观测；从账本计算该观测仍未证，
  且账本归属不在本票收窄范围；
- `CarrierRef (ℓ+1)` 只在类型层表达调用方声明的 Compose 父引用；其 canonical 签发、
  `CarrierId` 唯一性与区间输入良构仍未证，state→LevelView 正确性按拆票留给 #663；
- 不修改 `IntervalNestCertificate` 的通用结构核；文件后部提供 Bool 适配器，
  将本文件的具体 Lift 语义送入外部 `candidateOK` / `confirmOK` 缝。

无证明占位符。
-/

import Origin.BspConstruction
import Strict.Nest

namespace NewChanlun.Origin

open Strict.Nest (Interval)

/-! ═══════════════════════════════════════════════════════════════════════
    § 1. Lift 的载体：次级别一类证书、host 与父 carrier
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **同点递归身份锚（定义）**。

  只保存 canonical `(极值价, 合并组首根锚)`；不保存会随包含合并漂移的单根序号，
  也不把方向或分型类型混进点身份。
-/
structure RecursivePointAnchor where
  extremePrice : Tick
  mergeGroupAnchor : Index
deriving DecidableEq, Repr

/-- 两个身份锚是否表示同一个递归点（定义）。 -/
def SamePointRecursive (a b : RecursivePointAnchor) : Prop :=
  a.extremePrice = b.extremePrice ∧ a.mergeGroupAnchor = b.mergeGroupAnchor

instance (a b : RecursivePointAnchor) : Decidable (SamePointRecursive a b) := by
  unfold SamePointRecursive
  infer_instance

/-- `SamePointRecursive` 的可执行 Bool 视图。 -/
def samePointRecursiveB (a b : RecursivePointAnchor) : Bool :=
  decide (SamePointRecursive a b)

/-- carrier 在当前快照内的全局对象身份；不携带级别副本。 -/
structure CarrierId where
  value : Nat
deriving DecidableEq, Repr

/--
  **类型化 carrier 引用（定义）**。

  `CarrierRef k` 表示调用参照系中的 `C_k` 对象句柄；`k` 是零运行时字段的类型索引。
-/
structure CarrierRef (k : Nat) where
  id : CarrierId
deriving DecidableEq, Repr

/--
  **次级别走势 host 候选** —— PDF `Host(g)` 所量化的 `d ∈ K_ℓ`。

  `ℓ` 仅作类型索引/参照系视图，不在对象中重复存级别字段。

  - `rightPoint`：host 右端点 `ρ(d)` 的 canonical 同点递归锚；
  - `window`：host 的候选转折区间 `J_ℓ(d)`；
  - `parent`：调用方提供的 Compose 父引用声明；其类型索引为 `C_{ℓ+1}`。
-/
structure HostCarrier (ℓ : Nat) where
  id : Nat
  parent : CarrierRef (ℓ + 1)
  rightPoint : RecursivePointAnchor
  window : Interval
deriving DecidableEq, Repr

/--
  **第 ℓ 级终端证书** —— PDF Lift 的输入 `g ∈ P^δ_{1,ℓ}`。

  `isTypeOne` 把“g 必须是次级别第一类买卖点”放进类型载体，避免把 Lift 偷偷扩张到
  任意类型证书。`hostCandidates : List (HostCarrier ℓ)` 是 `K_ℓ` 的有限视图；
  `sourcePoint` 用 canonical 锚表达 `s(g)`，不以单根序号判同。

  `notUsedAtTime` 是调用方对共享 `M_t` 作出的 `g ∉ M_t` 时点观测。账本归属属于
  #257 已挂起的第 6 项，故这里不把 `M_t` 私自塞进某个父 carrier。
-/
structure Cert (ℓ : Nat) where
  id : Nat
  sourcePoint : RecursivePointAnchor
  endpoint : BspEndpoint
  isTypeOne : IsType1 endpoint
  hostCandidates : List (HostCarrier ℓ)
  notUsedAtTime : Bool
deriving Repr

/--
  **上级操作 carrier `c ∈ C_{ℓ+1}`**。

  `endpoint` 是上级分类器在 carrier 右端给出的完整判据载体。两条 `_spec` 把几何位
  `brokeCenter/leftCenter` 锚回 canonical `BspConstruction` 的末端价推导；历史位
  `afterTypeOne` 刻意不加几何等式，因为它属于父级一类之后的时序。

  carrier 自身不存级别；`Nest` 只接受 id 与命中 host 所携 `CarrierRef (ℓ+1)` 一致的对象。
-/
structure ParentCarrier where
  id : CarrierId
  window : Interval
  terminalMove : Move
  center : Center
  endpoint : BspEndpoint
  endpointCenter : endpoint.center = center
  brokeCenter_spec : endpoint.brokeCenter = brokeCenterOf terminalMove center
  leftCenter_spec : endpoint.leftCenter = leftCenterOf terminalMove center
deriving Repr

/-- `d` 是否命中 `g`：二者同属类型视图 `ℓ`，且 `ρ(d)=s(g)` 按同点递归锚判定。 -/
def hostMatchB {ℓ : Nat} (g : Cert ℓ) (d : HostCarrier ℓ) : Bool :=
  samePointRecursiveB d.rightPoint g.sourcePoint

/--
  **PDF `Host(g)` 的可执行版** —— `K_ℓ` 中按同点递归锚命中右端点的 host 恰有一个。

  `K_ℓ` 的集合语义用 `eraseDups` 保留：同一 host 的重复列表记录不制造第二个数学对象。
-/
def hostB {ℓ : Nat} (g : Cert ℓ) : Bool :=
  decide ((g.hostCandidates.filter (hostMatchB g)).eraseDups.length = 1)

/-- PDF `Host(g)`，Prop 视图。 -/
def Host {ℓ : Nat} (g : Cert ℓ) : Prop := hostB g = true

/--
  **PDF `Nest(c,g)` 的可执行版** —— 命中的 host：
  (1) `CarrierRef (ℓ+1)` 的对象 id 是 c；(2) `J_ℓ(d) ⊆ J_{ℓ+1}(c)`。

  `d ∈ K_ℓ` 与 `c ∈ C_{ℓ+1}` 分别由 `HostCarrier ℓ` / `d.parent : CarrierRef (ℓ+1)`
  的类型索引表达，carrier 对象内没有可漂移的 level 值。
  `Induces` 还同时要求 `Host`，故 `.any` 命中的 d 在合法输入上就是唯一 host。
-/
def nestB {ℓ : Nat} (g : Cert ℓ) (c : ParentCarrier) : Bool :=
  g.hostCandidates.any fun d =>
    hostMatchB g d &&
      decide (d.parent.id = c.id ∧ c.window.startTime ≤ d.window.startTime ∧
        d.window.endTime ≤ c.window.endTime)

/-- PDF `Nest(c,g)`，Prop 视图。 -/
def Nest {ℓ : Nat} (g : Cert ℓ) (c : ParentCarrier) : Prop := nestB g c = true

/--
  PDF `Fresh(g) ↔ g ∉ M_t` 的可执行视图。

  `M_t` 是同一时点共享记忆；本票不裁定其归属，只消费证书携带的外部时点观测。
-/
def freshB {ℓ : Nat} (g : Cert ℓ) : Bool := g.notUsedAtTime

/-- PDF `Fresh(g)`，Prop 视图。 -/
def Fresh {ℓ : Nat} (g : Cert ℓ) : Prop := freshB g = true

/-! ═══════════════════════════════════════════════════════════════════════
    § 2. 三个 Context^δ_j 的完整展开
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **`Context^δ_1(c,g)`（定义）** —— 证书方向与父端点方向一致，且父端点满足第一类：
  `brokeCenter=true ∧ IsDivergence`。
-/
def ContextOne {ℓ : Nat} (g : Cert ℓ) (c : ParentCarrier) : Prop :=
  g.endpoint.side = c.endpoint.side ∧ IsType1 c.endpoint

/--
  **`Context^δ_2(c,g)`（定义）** —— 证书方向与父端点方向一致，且父端点处于
  **父级一类之后的回抽序列**：`afterTypeOne=true ∧ brokeCenter=false`。

  这里 `afterTypeOne` 是历史条件，不是价格几何的别名。它由上级结构历史提供；
  本定义不把“中枢内”偷换成 `afterTypeOne=false`。
-/
def ContextTwo {ℓ : Nat} (g : Cert ℓ) (c : ParentCarrier) : Prop :=
  g.endpoint.side = c.endpoint.side ∧ IsType2 c.endpoint

/--
  **`Context^δ_3(c,g)`（定义）** —— 证书方向与父端点方向一致，且父端点满足方向化第三类：
  离开中枢、第一次回抽且买向不回到 ZG / 卖向不回到 ZD。
-/
def ContextThree {ℓ : Nat} (g : Cert ℓ) (c : ParentCarrier) : Prop :=
  g.endpoint.side = c.endpoint.side ∧ IsType3 c.endpoint

instance {ℓ : Nat} (g : Cert ℓ) (c : ParentCarrier) : Decidable (ContextOne g c) := by
  unfold ContextOne
  infer_instance

instance {ℓ : Nat} (g : Cert ℓ) (c : ParentCarrier) : Decidable (ContextTwo g c) := by
  unfold ContextTwo
  infer_instance

instance {ℓ : Nat} (g : Cert ℓ) (c : ParentCarrier) : Decidable (ContextThree g c) := by
  unfold ContextThree
  infer_instance

/-- 三个 Context 的类别索引分派。二/三类可重合，不强行做互斥 sum。 -/
def Context {ℓ : Nat} (g : Cert ℓ) (c : ParentCarrier) : BspClass → Prop
  | .one => ContextOne g c
  | .two => ContextTwo g c
  | .three => ContextThree g c

/-! ═══════════════════════════════════════════════════════════════════════
    § 3. PDF Lift：Host ∧ Nest ∧ Context ∧ Fresh
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **PDF `Lift^δ_{j,ℓ}(c,g)` 的 Lean 实现（定义）**。

  类型正是票体要求的 `Cert ℓ → ParentCarrier → BspClass → Prop`；四个合取逐项对应
  PDF §2.1–§2.4，不加入任何加强前提。
-/
def Induces {ℓ : Nat} (g : Cert ℓ) (c : ParentCarrier) (j : BspClass) : Prop :=
  Host g ∧ Nest g c ∧ Context g c j ∧ Fresh g

/-! ─── § 3.1 Bool 适配：不改通用 IntervalNestCertificate 核 ─── -/

/--
  三个 Context 的可执行 Bool 镜像。它只把**给定**的父端点厚判据转成 Bool；
  不构造 `LevelView`，也不冒充从行情识别完整历史。
-/
instance {ℓ : Nat} (g : Cert ℓ) (c : ParentCarrier) (j : BspClass) :
    Decidable (Context g c j) := by
  cases j <;> unfold Context <;> infer_instance

def contextB {ℓ : Nat} (g : Cert ℓ) (c : ParentCarrier) (j : BspClass) : Bool :=
  decide (Context g c j)

/-- `contextB` 与三个 Prop Context 逐类同真（L0 machine-checked）。 -/
theorem contextB_iff {ℓ : Nat} (g : Cert ℓ) (c : ParentCarrier) (j : BspClass) :
    contextB g c j = true ↔ Context g c j := by
  simp [contextB]

/--
  PDF Lift 的可执行 Bool 镜像，可作为 `IntervalNestCertificate.NestLevel` 外部
  `candidateOK/confirmOK` 的语义适配输入；通用区间套结构核本身保持解耦。
-/
def inducesB {ℓ : Nat} (g : Cert ℓ) (c : ParentCarrier) (j : BspClass) : Bool :=
  hostB g && (nestB g c && (contextB g c j && freshB g))

/-- `inducesB` 与 `Induces` 四合取同真（L0 machine-checked）。 -/
theorem inducesB_iff {ℓ : Nat} (g : Cert ℓ) (c : ParentCarrier) (j : BspClass) :
    inducesB g c j = true ↔ Induces g c j := by
  simp only [inducesB, Bool.and_eq_true, contextB_iff, Induces, Host, Nest, Fresh]

/-! ═══════════════════════════════════════════════════════════════════════
    § 4. `W ⊆ [ZD,ZG]` 的真实推论边界
    ═══════════════════════════════════════════════════════════════════════ -/

/-- **字面几何前提（定义）**：`W ⊆ [ZD,ZG]`，不暗加 W 与父终端的关系。 -/
def WalkInsideParentCenter (W : List Tick) (c : ParentCarrier) : Prop :=
  ∀ p, p ∈ W → IsWithin c.center p

/--
  **与待分类父终端连接的 Inside（定义）** —— 除字面子集外，父 carrier 的终端价属于 W。

  正向推出 `brokeCenter/leftCenter` 必须有这条对象连接；下文对票体字面弱前提的两个
  全称命题则单独使用 `WalkInsideParentCenter`，不会把加强前提冒充原题。
-/
def InsideParentCenter (W : List Tick) (c : ParentCarrier) : Prop :=
  c.terminalMove.endPrice ∈ W ∧ WalkInsideParentCenter W c

/-- Inside 蕴含父 carrier 终端价在 `[ZD,ZG]` 内（L0）。 -/
theorem insideParentCenter_terminal_within {W : List Tick} {c : ParentCarrier}
    (h : InsideParentCenter W c) :
    IsWithin c.center c.terminalMove.endPrice :=
  h.2 _ h.1

/-- Inside 的终端价不在中枢外（供 broke/left 两个几何位共享，L0）。 -/
theorem insideParentCenter_not_outside {W : List Tick} {c : ParentCarrier}
    (h : InsideParentCenter W c) :
    ¬ (c.terminalMove.endPrice < c.center.zd ∨
      c.center.zg < c.terminalMove.endPrice) := by
  have hw := insideParentCenter_terminal_within h
  unfold IsWithin at hw
  simp only [Tick] at hw ⊢
  omega

/-- Inside 排除几何位 `brokeCenter`（L0 machine-checked）。 -/
theorem insideParentCenter_brokeCenter_false {W : List Tick} {c : ParentCarrier}
    (h : InsideParentCenter W c) :
    c.endpoint.brokeCenter = false := by
  rw [c.brokeCenter_spec]
  simp [brokeCenterOf, insideParentCenter_not_outside h]

/-- Inside 排除几何位 `leftCenter`（L0 machine-checked）。 -/
theorem insideParentCenter_leftCenter_false {W : List Tick} {c : ParentCarrier}
    (h : InsideParentCenter W c) :
    c.endpoint.leftCenter = false := by
  rw [c.leftCenter_spec]
  simp [leftCenterOf, insideParentCenter_not_outside h]

/-- Inside 排除一类 Context：一类要求 `brokeCenter=true`（L0）。 -/
theorem insideParentCenter_not_contextOne {ℓ : Nat} {g : Cert ℓ}
    {c : ParentCarrier} {W : List Tick}
    (h : InsideParentCenter W c) :
    ¬ ContextOne g c := by
  intro hctx
  have hfalse := insideParentCenter_brokeCenter_false h
  have htrue : c.endpoint.brokeCenter = true := hctx.2.1
  rw [hfalse] at htrue
  exact Bool.noConfusion htrue

/-- Inside 排除三类 Context：三类两方向都要求 `leftCenter=true`（L0）。 -/
theorem insideParentCenter_not_contextThree {ℓ : Nat} {g : Cert ℓ}
    {c : ParentCarrier} {W : List Tick}
    (h : InsideParentCenter W c) :
    ¬ ContextThree g c := by
  intro hctx
  have hfalse := insideParentCenter_leftCenter_false h
  rcases hctx.2 with hbuy | hsell
  · have htrue : c.endpoint.leftCenter = true := hbuy.2.1
    rw [hfalse] at htrue
    exact Bool.noConfusion htrue
  · have htrue : c.endpoint.leftCenter = true := hsell.2.1
    rw [hfalse] at htrue
    exact Bool.noConfusion htrue

/--
  **Inside 的无加强结论（L0）** —— 只能同时排除一类与三类；刻意不声称排除二类。
-/
theorem insideParentCenter_excludes_geometricContexts {ℓ : Nat} {g : Cert ℓ}
    {c : ParentCarrier} {W : List Tick}
    (h : InsideParentCenter W c) :
    ¬ ContextOne g c ∧ ¬ ContextThree g c :=
  ⟨insideParentCenter_not_contextOne h, insideParentCenter_not_contextThree h⟩

/--
  **二类的历史条件精确暴露（L0）** —— 在方向匹配且 W 位于父中枢内时，
  `ContextTwo` 当且仅当历史位 `afterTypeOne=true`。

  这正是几何约束无法排掉二类的突破口。
-/
theorem insideParentCenter_contextTwo_iff {ℓ : Nat} {g : Cert ℓ}
    {c : ParentCarrier} {W : List Tick}
    (h : InsideParentCenter W c)
    (hsame : g.endpoint.side = c.endpoint.side) :
    ContextTwo g c ↔ c.endpoint.afterTypeOne = true := by
  unfold ContextTwo IsType2
  constructor
  · intro hctx
    exact hctx.2.1
  · intro hafter
    exact ⟨hsame, hafter, insideParentCenter_brokeCenter_false h⟩

/--
  **加强版三类全排除（L0）** —— 只有显式另加历史前提
  `afterTypeOne=false`，Inside 才能推出三个 Context 全假。

  ★这不是题设原命题；定理名和注释均保留 `_of_not_afterTypeOne`，禁止冒充。
-/
theorem insideParentCenter_excludes_allContexts_of_not_afterTypeOne
    {ℓ : Nat} {g : Cert ℓ} {c : ParentCarrier} {W : List Tick}
    (h : InsideParentCenter W c)
    (hafter : c.endpoint.afterTypeOne = false) :
    ¬ ContextOne g c ∧ ¬ ContextTwo g c ∧ ¬ ContextThree g c := by
  refine ⟨insideParentCenter_not_contextOne h, ?_,
    insideParentCenter_not_contextThree h⟩
  intro hctx
  have htrue : c.endpoint.afterTypeOne = true := hctx.2.1
  rw [hafter] at htrue
  exact Bool.noConfusion htrue

/-- 票体三元组 `(brokeCenter,leftCenter,afterTypeOne)`。 -/
def parentFlags (c : ParentCarrier) : Bool × Bool × Bool :=
  (c.endpoint.brokeCenter, c.endpoint.leftCenter, c.endpoint.afterTypeOne)

/--
  **Inside 对三元组的最强无加强定理（L0）**：
  前两位归零，第三位保留原历史值，而不是被几何强制归零。
-/
theorem insideParentCenter_flags_exact {W : List Tick} {c : ParentCarrier}
    (h : InsideParentCenter W c) :
    parentFlags c = (false, false, c.endpoint.afterTypeOne) := by
  unfold parentFlags
  rw [insideParentCenter_brokeCenter_false h, insideParentCenter_leftCenter_false h]

/-! ═══════════════════════════════════════════════════════════════════════
    § 5. 原命题的二类历史反例（完整 Lift，而非条件反例）
    ═══════════════════════════════════════════════════════════════════════ -/

/-- 反例父中枢 `[10,20]`。 -/
def insideTypeTwoCenter : Center :=
  { zd := 10
    zg := 20
    startIndex := 0
    endIndex := 30
    valid := by decide }

/-- 父 carrier 终端走势：末端价 15，完整落点在中枢内部。 -/
def insideTypeTwoParentMove : Move :=
  { kind := MoveKind.consolidation
    startIndex := 0
    endIndex := 30
    startPrice := 12
    endPrice := 15
    centers := [insideTypeTwoCenter] }

/--
  父级二类端点：几何位均假，但历史位 `afterTypeOne=true`。
  这镜像仓内既有 `SecondSellClosedLoop.sampleType2Buy` 的 `[10,20] / 15` 见证。
-/
def insideTypeTwoParentEndpoint : BspEndpoint :=
  { side := Side.long
    center := insideTypeTwoCenter
    divPair := { forceA := ⟨3⟩, forceC := ⟨3⟩, isTrend := false }
    brokeCenter := false
    afterTypeOne := true
    leftCenter := false
    retracePrice := 15
    firstRetrace := false }

/-- 父 carrier：由 host 的 `CarrierRef 1` 指向，区间为 `[0,30]`。 -/
def insideTypeTwoParent : ParentCarrier :=
  { id := { value := 200 }
    window := { startTime := 0, endTime := 30, idx := 0 }
    terminalMove := insideTypeTwoParentMove
    center := insideTypeTwoCenter
    endpoint := insideTypeTwoParentEndpoint
    endpointCenter := rfl
    brokeCenter_spec := by native_decide
    leftCenter_spec := by native_decide }

/-- 次级别唯一 host：类型视图为 level 0，嵌套于父区间并真指向父 id 200。 -/
def insideTypeTwoHost : HostCarrier 0 :=
  { id := 100
    parent := { id := { value := 200 } }
    rightPoint := { extremePrice := 10, mergeGroupAnchor := 18 }
    window := { startTime := 10, endTime := 20, idx := 1 } }

/-- 次级别一类证书端点（方向 long、破中枢且背驰）。 -/
def insideTypeTwoChildEndpoint : BspEndpoint :=
  { side := Side.long
    center := insideTypeTwoCenter
    divPair := { forceA := ⟨3⟩, forceC := ⟨1⟩, isTrend := true }
    brokeCenter := true
    afterTypeOne := false
    leftCenter := false
    retracePrice := 10
    firstRetrace := false }

/-- PDF 定义域内的 `g ∈ P^+_{1,0}`，唯一 host 且在共享 `M_t` 观测中尚未使用。 -/
def insideTypeTwoCert : Cert 0 :=
  { id := 7
    sourcePoint := { extremePrice := 10, mergeGroupAnchor := 18 }
    endpoint := insideTypeTwoChildEndpoint
    isTypeOne := by native_decide
    hostCandidates := [insideTypeTwoHost]
    notUsedAtTime := true }

/-- 反例价格窗 `W={15}`。 -/
def insideTypeTwoWalk : List Tick := [15]

/-- W 完整在 `[10,20]` 内（L0 machine-checked）。 -/
theorem witness_insideTypeTwo_inside :
    InsideParentCenter insideTypeTwoWalk insideTypeTwoParent := by
  simp [InsideParentCenter, WalkInsideParentCenter, insideTypeTwoWalk,
    insideTypeTwoParent, insideTypeTwoParentMove, insideTypeTwoCenter, IsWithin]

/-- 反例三元组精确为 `(0,0,1)`（L0 machine-checked）。 -/
theorem witness_insideTypeTwo_flags :
    parentFlags insideTypeTwoParent = (false, false, true) := by
  native_decide

/-- Inside 场景的一类 Context 为假（L0）。 -/
theorem witness_insideTypeTwo_not_contextOne :
    ¬ ContextOne insideTypeTwoCert insideTypeTwoParent :=
  insideParentCenter_not_contextOne witness_insideTypeTwo_inside

/-- Inside 场景的三类 Context 为假（L0）。 -/
theorem witness_insideTypeTwo_not_contextThree :
    ¬ ContextThree insideTypeTwoCert insideTypeTwoParent :=
  insideParentCenter_not_contextThree witness_insideTypeTwo_inside

/--
  **历史突破几何约束**：同一 Inside 场景的二类 Context 为真（L0 machine-checked）。
-/
theorem witness_insideTypeTwo_contextTwo :
    ContextTwo insideTypeTwoCert insideTypeTwoParent := by
  native_decide

/--
  **完整 Lift 反例**：Host、Nest、Context₂、Fresh 四门全过，故中枢内 W 确实可诱导
  父级二类，不只是“若其余门恰好为真”的条件反例。
-/
theorem witness_insideTypeTwo_induces :
    Induces insideTypeTwoCert insideTypeTwoParent BspClass.two := by
  apply (inducesB_iff insideTypeTwoCert insideTypeTwoParent BspClass.two).mp
  native_decide

/-- 票体目标“三位必为 0”的无加强、全称版本。 -/
def InsideForcesZero : Prop :=
  ∀ (W : List Tick) (c : ParentCarrier),
    WalkInsideParentCenter W c → parentFlags c = (false, false, false)

/-- 票体第 3 项“三个 Context 全假”的无加强、全称版本。 -/
def InsideExcludesAllContexts : Prop :=
  ∀ (ℓ : Nat) (g : Cert ℓ) (c : ParentCarrier) (W : List Tick),
    WalkInsideParentCenter W c →
      ¬ ContextOne g c ∧ ¬ ContextTwo g c ∧ ¬ ContextThree g c

/--
  **目标三元组命题证伪（L0 machine-checked）**：
  `W⊆[ZD,ZG]` 不推出 `(brokeCenter,leftCenter,afterTypeOne)=(0,0,0)`。
-/
theorem insideParentCenter_not_forces_zero : ¬ InsideForcesZero := by
  intro h
  have hz := h insideTypeTwoWalk insideTypeTwoParent witness_insideTypeTwo_inside.2
  rw [witness_insideTypeTwo_flags] at hz
  have hthird := congrArg (fun x : Bool × Bool × Bool => x.2.2) hz
  exact Bool.noConfusion hthird

/--
  **原三类全排除命题证伪（L0 machine-checked）**：
  反例中的 `ContextTwo` 为真。
-/
theorem insideParentCenter_not_excludes_allContexts :
    ¬ InsideExcludesAllContexts := by
  intro h
  have hall := h 0 insideTypeTwoCert insideTypeTwoParent insideTypeTwoWalk
    witness_insideTypeTwo_inside.2
  exact hall.2.1 witness_insideTypeTwo_contextTwo

-- TDD seam：票体要求的精确公共签名。
example {ℓ : Nat} : Cert ℓ → ParentCarrier → BspClass → Prop := Induces

-- TDD seam：同价不同合并组不是同一递归点，防止 W 底双脚被误并。
example :
    samePointRecursiveB
      { extremePrice := 10, mergeGroupAnchor := 18 }
      { extremePrice := 10, mergeGroupAnchor := 19 } = false := by
  native_decide

-- TDD seam：Compose 父引用的 `ℓ+1` 由 elaborator 检查，不存在运行时 level 副本。
example : CarrierRef (0 + 1) := insideTypeTwoHost.parent

-- TDD seam：区间相同但全局对象身份不同，不能冒充类型化父引用指向的 carrier。
example :
    nestB insideTypeTwoCert
      { insideTypeTwoParent with id := { value := 201 } } = false := by
  native_decide

-- TDD seam：通用区间套 Bool 参数消费本模块语义时，必须与 Prop 定义同真。
example {ℓ : Nat} (g : Cert ℓ) (c : ParentCarrier) (j : BspClass) :
    contextB g c j = true ↔ Context g c j := contextB_iff g c j

example {ℓ : Nat} (g : Cert ℓ) (c : ParentCarrier) (j : BspClass) :
    inducesB g c j = true ↔ Induces g c j := inducesB_iff g c j

-- TDD seam：Inside 只能消去两个几何位；二类精确退化为独立历史位。
example {ℓ : Nat} {g : Cert ℓ} {c : ParentCarrier} {W : List Tick}
    (h : InsideParentCenter W c) :
    ¬ ContextOne g c ∧ ¬ ContextThree g c :=
  insideParentCenter_excludes_geometricContexts h

example {ℓ : Nat} {g : Cert ℓ} {c : ParentCarrier} {W : List Tick}
    (h : InsideParentCenter W c)
    (hsame : g.endpoint.side = c.endpoint.side) :
    ContextTwo g c ↔ c.endpoint.afterTypeOne = true :=
  insideParentCenter_contextTwo_iff h hsame

-- TDD seam：原命题必须由具体二类历史反例证伪，而不是只写一段解释。
example : InsideParentCenter insideTypeTwoWalk insideTypeTwoParent :=
  witness_insideTypeTwo_inside

example : Induces insideTypeTwoCert insideTypeTwoParent BspClass.two :=
  witness_insideTypeTwo_induces

example : ¬ InsideForcesZero := insideParentCenter_not_forces_zero

example : ¬ InsideExcludesAllContexts := insideParentCenter_not_excludes_allContexts

end NewChanlun.Origin
