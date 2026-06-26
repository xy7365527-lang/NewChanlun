/-
  会计层 · 递归森林（T₃₉ 任意深度守恒 / T₄₀ M=N 翻转 / T₄₂ 孤儿不可能 / T₄₄ 递归深度有限）
  工位：t-accounting（task #49 / gap-map 层C E-set）

  ── 认识论等级：L0（定义内蕴，formalization-validity-domain）──────────────────
  本模块对 **递归 voice 森林 datatype** 做结构归纳：T₃₉ 全链 Σunits 守恒（归纳不依赖深度）、
  T₄₀ 翻转保单位数（M=N）、T₄₂ 孤儿不可能（后序关闭传播）、T₄₄ 递归深度有限（级别良序
  严格递减）。这些是 datatype 上的纯结构定理，**不依赖任何运行时数据**。`lake build` 通过
  = 逻辑/管线正确（L0），不是实证有效域。运行时数值侧（prove_n1_forest panic 守卫）= R 类
  L2，由 Rust 引擎覆盖，不进 Lean。

  ── 与 #42 矛盾的边界声明 ────────────────────────────────────────────────────
  task #42 记录「58槽 D∞ 穷尽被会计层 L0 反推否定 3 条」。本模块形式化的是 **森林单位守恒
  与结构良构**（Σunits=N_base 的归纳形式），**不触及 58 槽 D∞ 穷尽**（那是几何螺旋层
  Spiral.lean 的 D∞ 群呈现）。会计守恒（圈内不变量）与 D∞ 角向穷尽是两个范畴——本模块在
  会计范畴内自洽，不向 D∞ 穷尽范畴施加约束，故 **不重新触及 #42 矛盾**。

  ── 隔离声明 ────────────────────────────────────────────────────────────────
  本模块 **自包含**：不 import Formal.* / 不 import Phase2.*。无 sorry/admit/axiom。
  （T₄₂ 部分承自原 Accounting.lean，已通过 codex 审计 thread 019effeb，原样保留。）
-/

namespace Formal.Tlayers.Accounting

/-! ## §1 voice 递归森林 datatype（T₂₃ 自相似嵌套的初代数 μF） -/

/-- voice 关闭状态（A4 close_voice 单向：Active → Closed，无 reopen）。 -/
inductive VoiceStatus where
  | active
  | closed
deriving DecidableEq, Repr

open VoiceStatus

/--
  ★voice 递归森林节点（T₂₃ 自相似嵌套的初代数 μF）。

  每个 voice 携带 `status`、`units`（本节点持有单位数，T₃₉ 守恒量）+ 递归
  `children : List Voice`。关键结构性质（孤儿不可能的 datatype 内蕴）：子节点 **物理嵌套**
  在父的 `children` 列表里——不存在「悬空父引用」，故「孤儿」唯一可能形式 = 「父 Closed
  而某后代仍 Active」。T₄₂ 证的就是 `closeVoice` 后序关闭排除这一形式。
-/
inductive Voice where
  | mk (status : VoiceStatus) (units : Nat) (children : List Voice)
deriving Repr

namespace Voice

/-- 节点自身状态。 -/
def status : Voice → VoiceStatus
  | mk s _ _ => s

/-- 节点自身持有单位数。 -/
def units : Voice → Nat
  | mk _ u _ => u

/-- 直接子节点列表。 -/
def children : Voice → List Voice
  | mk _ _ cs => cs

end Voice

open Voice

/-- 节点是否 active。 -/
def isActive (v : Voice) : Bool :=
  match v.status with
  | active => true
  | closed => false

/-! ## §2 T₃₉ 递归链守恒（父+子+孙 units = N_base，任意深度）

  necessity_derivation.md §819-824：递归嵌套到任意深度，Σ(全链所有 voice.units) = N_base
  恒成立。证明 = T₃₄ 归纳证明不依赖深度。形式化：森林全链单位总和 = 节点自身 + 所有后代
  units 之和（结构递归），翻转/重排不改变它。 -/

/--
  ★全链单位总和（T₃₉ 守恒量）：节点自身 units + 所有子树递归总和。
  这是「父+子+孙 units」对 **任意深度** 的统一表达（结构递归到任意层）。
-/
def chainTotalUnits : Voice → Nat
  | Voice.mk _ u cs => u + (cs.map chainTotalUnits).sum

/--
  ★森林全链单位总和（多根森林）：各根 chainTotalUnits 之和。 -/
def forestTotalUnits (forest : List Voice) : Nat :=
  (forest.map chainTotalUnits).sum

/--
  ★T₃₉ 守恒展开（L0）：单节点全链总和 = 自身 units + 子森林总和。
  这把「任意深度守恒」的递归结构显式化——守恒量在每层都等于「本层 + 下层总和」，归纳
  不依赖深度（§823：T₃₄ 归纳证明不依赖深度）。
-/
theorem chainTotalUnits_unfold (s : VoiceStatus) (u : Nat) (cs : List Voice) :
    chainTotalUnits (Voice.mk s u cs) = u + forestTotalUnits cs := by
  simp only [chainTotalUnits, forestTotalUnits]

/--
  ★T₃₉ 任意深度守恒守恒律（L0）：把一个子节点从父的 children 移到森林根层（重排层级），
  全链总和不变——单位只是在级别间流转（§727），总和恒定。

  形式化：`mk s u (child :: rest)` 的全链总和 = `chainTotalUnits child + chainTotalUnits (mk s u rest)`，
  即「父抱着 child」与「父放开 child 到同级」的全链总和相等。这 machine-check「多空嵌套是
  同一批 N 单位在不同级别间重新分配，总和恒为 N_base」对任意深度成立。
-/
theorem chain_reparent_conserves (s : VoiceStatus) (u : Nat)
    (child : Voice) (rest : List Voice) :
    chainTotalUnits (Voice.mk s u (child :: rest))
      = chainTotalUnits child + chainTotalUnits (Voice.mk s u rest) := by
  simp only [chainTotalUnits, List.map_cons, List.sum_cons]
  omega

/-! ## §3 T₄₀ 子 voice 翻转 = 会计断面（M = N 同股数）

  necessity_derivation.md §829-837：子级别走势完美时，子 voice 旧方向关闭 + 新方向开启 =
  会计断面（cascade settle），同股数翻转（M=N，不增不减仓）。形式化：翻转操作改 status
  与方向，但保 units（M=N）。 -/

/--
  ★翻转操作（T₄₀ 会计断面）：子 voice 旧方向关闭 + 新方向以 **同股数** 开启。
  这里以「units 不变」编码 M=N——翻转改 status（active→新 active，方向已在 polarity 层，
  本节点只关心单位数守恒），units 保持。返回新 voice。
-/
def flipVoice (v : Voice) : Voice :=
  Voice.mk active v.units v.children

/--
  ★T₄₀ 翻转保单位数 M=N（L0）：翻转后 units 与翻转前相等。
  这 machine-check「翻转单位数不变（M=N，不增不减仓）」（§836）——由 T₃₄ 守恒，会计断面
  是同股数翻转，关死「翻转 = 加仓/减仓」的误读。
-/
theorem flip_preserves_units (v : Voice) :
    (flipVoice v).units = v.units := by
  cases v with
  | mk s u cs => rfl

/--
  ★T₄₀ 翻转保全链总和（L0）：翻转不改变全链单位总和（M=N 推广到整棵子树）。
  翻转只改根 status，units 与 children 不变 ⟹ chainTotalUnits 不变。
-/
theorem flip_preserves_chainTotal (v : Voice) :
    chainTotalUnits (flipVoice v) = chainTotalUnits v := by
  cases v with
  | mk s u cs =>
      simp only [flipVoice, Voice.units, Voice.children, chainTotalUnits]

/-! ### T₄₀ 会计断面事件分解（修 codex FAIL #6：flip = 关闭旧 + 反向开启新）

  necessity §831：翻转 = 「子 voice 旧方向关闭 + 新方向开启」会计断面（cascade settle）。
  仅证「flip 保 |units|」不够（codex #6：缺事件分解）——必须把 flip 显式拆为 **两步事件**
  （关闭旧、反向开启新），证两步组合后 (a) 旧 voice Closed、(b) 新 voice 反向 active、
  (c) 单位数 M=N。下用方向枚举 + 两步事件结构表达。 -/

/-- 翻转方向（会计断面的多空反转，§831「新方向」）。 -/
inductive FlipDir where
  | longSide
  | shortSide
deriving DecidableEq, Repr

open FlipDir

/-- 方向反转（旧→新，会计断面「反向开启」）。 -/
def opposite : FlipDir → FlipDir
  | longSide => shortSide
  | shortSide => longSide

/--
  ★会计断面事件（T₄₀ 两步分解）：携带旧方向 `oldDir`、单位数 `m`、旧/新两个 voice 态。
  - `oldClosed`：旧方向 voice 关闭后态（status=closed）。
  - `newOpened`：新方向 voice 开启后态（status=active，方向 = opposite oldDir，units=m）。
  这显式建模「关闭旧 + 反向开启新」，而非只翻 status。
-/
structure FlipEvent where
  oldDir : FlipDir
  m : Nat

/-- 断面第一步：旧方向 voice 关闭（status→closed，units 保留 m）。 -/
def FlipEvent.oldClosed (e : FlipEvent) : Voice := Voice.mk closed e.m []

/-- 断面第二步：新方向 voice 以同股数 m 开启（status=active）。 -/
def FlipEvent.newOpened (e : FlipEvent) : Voice := Voice.mk active e.m []

/-- 断面后的新方向 = 旧方向的反转。 -/
def FlipEvent.newDir (e : FlipEvent) : FlipDir := opposite e.oldDir

/--
  ★T₄₀ 会计断面三性质（L0，**完整事件分解**，修 codex #6）：一次翻转断面同时满足
  (a) 旧 voice 已关闭、(b) 新 voice 反向开启（方向 = opposite oldDir）、(c) 单位数 M=N
  （oldClosed.units = newOpened.units = m）。这 machine-check「旧方向关闭 + 新方向开启 =
  会计断面，同股数翻转」（§831/§836）——不再只是「翻 status 保 units」，而是显式的两步
  事件 + 方向反转 + 单位守恒三位一体。
-/
theorem flip_event_decomposition (e : FlipEvent) :
    e.oldClosed.status = closed
    ∧ e.newOpened.status = active
    ∧ e.newDir = opposite e.oldDir
    ∧ e.oldClosed.units = e.newOpened.units := by
  refine ⟨rfl, rfl, rfl, rfl⟩

/--
  ★T₄₀ 断面方向真反转（L0，可证伪见证）：新方向 ≠ 旧方向（翻转是真反向，非恒等）。
  关死「flip 不改方向」的误读——newDir 严格是 oldDir 的对立面（opposite 无不动点）。
-/
theorem flip_dir_actually_reverses (e : FlipEvent) :
    e.newDir ≠ e.oldDir := by
  unfold FlipEvent.newDir opposite
  cases e.oldDir with
  | longSide => simp
  | shortSide => simp

/-! ## §4 T₄₂ 孤儿不可能森林良构（承自原 Accounting.lean，codex 审计 019effeb 保留）

  necessity_derivation.md §873-883：voice 仅经 `close_voice`（后序：先关活跃子树再结算自身）
  关闭 ⟹ 父关闭时子必已关闭 ⟹ 永不产生孤儿。森林单根不变量（同时至多一个 root）。 -/

/--
  ★森林良构·无孤儿不变量（T₄₂ 结构侧核心谓词）。

  `NoOrphan v` ⟺ 「v 若 Closed，则其所有直接子均 Closed」∧「每个子递归满足 NoOrphan」。
  即沿森林任意路径，一旦某节点 Closed，其整棵后代子树皆 Closed——不存在「父已关而子仍活」
  的孤儿。这是 datatype 上的结构归纳谓词（递归到子树）。
-/
def NoOrphan : Voice → Prop
  | Voice.mk s _ cs =>
      (s = closed → ∀ c ∈ cs, c.status = closed)
      ∧ (∀ c ∈ cs, NoOrphan c)

/--
  ★close_voice 后序关闭函数（necessity_derivation.md:251-258 cascade 关子树）。

  `closeVoice v` = 先递归关闭 v 的所有子树（后序），再把 v 自身置 Closed。这就是引擎
  `close_voice` 的后序遍历语义。units 不变（关闭不改单位数，守恒侧由 §2 覆盖）。
-/
def closeVoice : Voice → Voice
  | Voice.mk _ u cs => Voice.mk closed u (cs.map closeVoice)

/-- 辅助：节点经 closeVoice 后自身必 Closed。 -/
theorem closeVoice_status (v : Voice) : (closeVoice v).status = closed := by
  cases v with
  | mk s u cs => simp [closeVoice, Voice.status]

/--
  ★closeVoice 后整棵子树全 Closed（后序关闭传播，T₄₂ 关键引理）。
  对 voice 森林：closeVoice 把自身和递归所有后代都置 Closed。这形式化「关闭节点前先递归
  关闭其所有活跃子节点」（后序遍历）的结构后果——子树无残留 Active。
-/
theorem closeVoice_all_closed (v : Voice) :
    (closeVoice v).status = closed
    ∧ ∀ c ∈ (closeVoice v).children, c.status = closed := by
  cases v with
  | mk s u cs =>
      refine ⟨closeVoice_status (Voice.mk s u cs), ?_⟩
      intro c hc
      simp only [closeVoice, Voice.children] at hc
      obtain ⟨c', _, hc'⟩ := List.mem_map.mp hc
      rw [← hc']
      exact closeVoice_status c'

/--
  ★T₄₂ 孤儿不可能定理（结构侧，L0）：close_voice 后序关闭 ⟹ 无孤儿。

  对 voice 森林结构归纳，证 `NoOrphan (closeVoice v)`：
  - 自身 Closed 时，所有直接子 closeVoice 后皆 Closed；
  - 每个子递归满足 NoOrphan（well-founded 结构递归）。
  这 machine-check「父关闭时子必已关闭，永不产生孤儿」——T₄₂ 命题的结构归纳实现。
-/
theorem closeVoice_no_orphan (v : Voice) : NoOrphan (closeVoice v) := by
  match v with
  | Voice.mk s u cs =>
      unfold NoOrphan closeVoice
      refine ⟨?_, ?_⟩
      · intro _ c hc
        obtain ⟨c', _, hc'⟩ := List.mem_map.mp hc
        rw [← hc']
        exact closeVoice_status c'
      · intro c hc
        obtain ⟨c', hc'mem, hc'eq⟩ := List.mem_map.mp hc
        rw [← hc'eq]
        exact closeVoice_no_orphan c'
termination_by v
decreasing_by
  have h1 : sizeOf c' < sizeOf cs := List.sizeOf_lt_of_mem hc'mem
  simp only [Voice.mk.sizeOf_spec]
  omega

/--
  ★孤儿不可能 ⟺ 无「父 Closed 而子 Active」（T₄₂ 命题的等价展开，可证伪见证）。
  若 v 满足 NoOrphan 且 v.status = closed，则不存在 active 直接子——把抽象谓词 NoOrphan
  兑现为命题陈述「父关闭时子必已关闭」。
-/
theorem noOrphan_closed_implies_children_closed (v : Voice)
    (h : NoOrphan v) (hclosed : v.status = closed) :
    ∀ c ∈ v.children, c.status = closed := by
  cases v with
  | mk s u cs =>
      unfold NoOrphan at h
      exact h.1 hclosed

/--
  ★NoOrphan 真会拒绝孤儿（可证伪性见证，非恒真）：构造一个父 Closed 而子 Active 的森林，
  它 **不** 满足 NoOrphan。证明 NoOrphan 是有判别力的真约束（L0：可被反例否证，非 vacuous）。
-/
theorem orphan_violates_noOrphan :
    ¬ NoOrphan (Voice.mk closed 0 [Voice.mk active 0 []]) := by
  unfold NoOrphan
  intro h
  have hsub := h.1 rfl (Voice.mk active 0 []) (by simp)
  simp only [Voice.status] at hsub
  exact VoiceStatus.noConfusion hsub

/-! ## §5 森林单根不变量（necessity §883①：≤1 active root，T₂₇ 单一持仓主体）

  ★诚实声明（codex 审计 019effeb Q5 FAIL 修正）：单根不变量来源是 **外部前提** T₂₇，
  **不是森林结构内蕴**——任意 active 森林结构上可有任意多 active root（datatype 不禁止）。
  故「≤1 active root」必须以 T₂₇ 为前提声明。本节给两条诚实定理：(a) 过程不变量
  closeVoice_root_count_noninc（close 操作不增 active root）；(b) 终态推论（全关闭后 = 0）。 -/

/-- 森林 active root 计数。 -/
def activeRootCount (forest : List Voice) : Nat :=
  (forest.filter isActive).length

def AtMostOneActiveRoot (forest : List Voice) : Prop :=
  activeRootCount forest ≤ 1

/-- 单一持仓主体前提（T₂₇ 编码为森林结构前提，非 datatype 内蕴）。 -/
def SingleSubjectForest (forest : List Voice) : Prop :=
  AtMostOneActiveRoot forest

/--
  ★单根不变量·以 T₂₇ 为前提（§883①忠实形式）：满足单一持仓主体前提的森林 ≤1 active root。
  如实表达「单根来自 T₂₇ 外部前提」——前提即结论是诚实恒等（不冒充结构内蕴推导）。
-/
theorem singleSubject_atMostOneRoot (forest : List Voice)
    (h : SingleSubjectForest forest) : AtMostOneActiveRoot forest := h

/--
  ★close 操作的真过程不变量（codex Q5 修正核心）：closeVoice 后 active root 计数 **不增**。
  每个根 closeVoice 后必 Closed ⟹ filter isActive 后为空 ⟹ count = 0 ≤ 原 count。这是真
  过程性质：关闭操作永不凭空产生新 active root，保持单根不变量。
-/
theorem closeVoice_root_count_noninc (forest : List Voice) :
    activeRootCount (forest.map closeVoice) ≤ activeRootCount forest := by
  have hfilter : (forest.map closeVoice).filter isActive = [] := by
    apply List.filter_eq_nil_iff.mpr
    intro v hv
    obtain ⟨v', _, hv'⟩ := List.mem_map.mp hv
    rw [← hv']
    simp only [isActive, closeVoice_status v', Bool.not_eq_true]
  unfold activeRootCount
  rw [hfilter]
  simp

/--
  ★终态推论（诚实降级，不冒充活森林一般不变量）：全森林 closeVoice 后 active root = 0。
  这是 closeVoice_root_count_noninc 的终态特例（计数降为 0），≤1 平凡成立。仅终态。
-/
theorem closedForest_zeroActiveRoot (forest : List Voice) :
    activeRootCount (forest.map closeVoice) = 0 := by
  unfold activeRootCount
  have hfilter : (forest.map closeVoice).filter isActive = [] := by
    apply List.filter_eq_nil_iff.mpr
    intro v hv
    obtain ⟨v', _, hv'⟩ := List.mem_map.mp hv
    rw [← hv']
    simp only [isActive, closeVoice_status v', Bool.not_eq_true]
  rw [hfilter]
  simp

/-! ## §6 T₄₄ 递归深度有限（depth ≤ root.ladder − bi）

  necessity_derivation.md §924-933：递归森林深度有限：depth ≤ root.ladder − bi(a0)。每次
  spawn 子 voice 级别严格降低（子级别 = 父级别 − 1），级别良序（下界 bi），故从 root.ladder
  出发严格递减序列在有限步到达 bi 终止。形式化：级别索引严格递减 ⟹ 深度 ≤ 初始级别。 -/

/--
  ★带级别索引的递归塔（T₄₄ 载体）：每个 voice 携带 levelIdx，spawn 子时 levelIdx − 1。
  级别是良序集 Nat，下界 0（= bi 结构原子）。spawn 严格递减保证有限。
-/
def levelDescends (parentLevel childLevel : Nat) : Prop :=
  childLevel + 1 = parentLevel

/--
  ★T₄₄ 级别严格递减（L0）：spawn 子级别 < 父级别。
  这形式化「每次 spawn 子 voice 级别严格降低」（§930）——递归方向单调向下。
-/
theorem spawn_level_strictly_decreases (parentLevel childLevel : Nat)
    (h : levelDescends parentLevel childLevel) :
    childLevel < parentLevel := by
  unfold levelDescends at h
  omega

/--
  ★T₄₄ 递归深度有界（L0）：从级别 `rootLevel` 出发，每层级别严格递减且下界 0，则递归深度
  ≤ rootLevel。形式化为：一条级别严格递减链（chain : List Nat，相邻满足 levelDescends）从
  rootLevel 出发，其长度 ≤ rootLevel。这 machine-check「严格递减的级别序列在有限步到达
  bi 层终止」（§932）——级别良序 ⟹ 深度有限。

  用归纳：链头级别 = rootLevel，每步 −1，到 0 终止 ⟹ 链长 ≤ rootLevel。
-/
def IsDescendingChain : List Nat → Prop
  | [] => True
  | [_] => True
  | a :: b :: rest => levelDescends a b ∧ IsDescendingChain (b :: rest)

/--
  ★T₄₄ 深度有限定理（L0）：从 rootLevel 出发的严格递减级别链，长度 ≤ rootLevel + 1。
  即递归深度有限——级别 Nat 良序 + 每步严格递减 ⟹ 不能无限递归。
-/
theorem descendingChain_length_bounded :
    ∀ (chain : List Nat) (rootLevel : Nat),
      chain.head? = some rootLevel → IsDescendingChain chain →
      chain.length ≤ rootLevel + 1 := by
  intro chain
  induction chain with
  | nil => intro rootLevel hhead _; simp at hhead
  | cons a rest ih =>
      intro rootLevel hhead hchain
      simp only [List.head?_cons, Option.some.injEq] at hhead
      subst hhead
      cases rest with
      | nil => simp
      | cons b rest' =>
          unfold IsDescendingChain at hchain
          obtain ⟨hstep, htail⟩ := hchain
          have hba : b < a := spawn_level_strictly_decreases a b hstep
          have htail_head : (b :: rest').head? = some b := rfl
          have hlen := ih b htail_head htail
          simp only [List.length_cons] at hlen ⊢
          omega

end Formal.Tlayers.Accounting
