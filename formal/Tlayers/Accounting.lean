/-
  会计层 T₄₂（孤儿不可能定理）·森林良构结构侧（task #49 / gap-map 层C）
  工位：t-accounting（会计层 L-class 形式化，仅 T₄₂ 结构侧 = 本层唯一真 (L)）

  ── 认识论等级：L0（定义内蕴，formalization-validity-domain）──────────────────
  本模块对 **递归 voice 森林 datatype** 做结构归纳，machine-check「孤儿不可能」=
  「close_voice 后序关闭 ⟹ 父关闭时其所有后代已关闭」+「森林同时至多一个 active root」。
  这是 datatype 上的纯结构定理（继承 necessity_derivation.md §873 T₄₂ 已结算命题），
  **不依赖任何运行时数据**。`lake build` 通过 = 逻辑/管线正确（L0），不是实证有效域，
  不得膨胀。运行时数值侧（prove_n1_forest panic 守卫）= R 类，由 Rust 引擎覆盖，不进 Lean。

  ── T₄₂ 命题（necessity_derivation.md §873-883，已结算）────────────────────────
  陈述：voice 仅经 `close_voice`（后序：先关活跃子树再结算自身）关闭 ⟹ 父关闭时子必已
       关闭 ⟹ 永不产生孤儿（无需 reparent）。森林单根不变量（同时至多一个 root）。
  证明（缠论侧）：由 T₂₃ 自相似嵌套构成森林。`close_voice` 后序遍历：关闭节点前先递归
       关闭其所有活跃子节点。故父关闭时其所有子已关闭，不存在「父已关而子仍活」的孤儿。
       由 T₂₇（永远在场，单一持仓主体），同时至多一个 active root。□

  ── 范式（603 内涵式）：森林良构 = 归纳数据类型的结构性质 ───────────────────────
  「无孤儿」不是「外延式检查所有节点对」，是 **归纳 voice 森林 datatype** 的结构归纳
  性质——子节点物理嵌套在父的 `children` 字段里（无悬空引用是 datatype 内蕴），后序
  `closeVoice` 函数保证关闭传播。T₄₂ 结构侧 = 对此 datatype 的结构归纳定理。

  ── 隔离声明（任务卡硬约束：不碰 lakefile.toml / Formal.lean）─────────────────
  本模块 **自包含**：不 import Formal.* / 不 import Phase2.*。无 sorry/admit/axiom。
  与 RStarNonSpecial 的 ValidTower / RecursiveConstruction 的 Move 复用关系是 **范式层**
  （同为有限递归森林的结构归纳），非 import 层（避免命名空间耦合，integrator 不需 wire）。
-/

namespace Formal.Tlayers.Accounting

/-- voice 关闭状态（A4 close_voice 单向：Active → Closed，无 reopen）。 -/
inductive VoiceStatus where
  | active
  | closed
deriving DecidableEq, Repr

open VoiceStatus

/--
  ★voice 递归森林节点（T₂₃ 自相似嵌套的初代数 μF）。

  每个 voice 携带 `status` + 递归 `children : List Voice`。关键结构性质（孤儿不可能的
  datatype 内蕴）：子节点 **物理嵌套** 在父的 `children` 列表里——不存在「悬空父引用」
  （无 parent 指针、无外部 reparent），故「孤儿」唯一可能形式 = 「父 Closed 而某后代仍
  Active」。T₄₂ 证的就是 `closeVoice` 后序关闭排除这一形式。
-/
inductive Voice where
  | mk (status : VoiceStatus) (children : List Voice)
deriving Repr

namespace Voice

/-- 节点自身状态。 -/
def status : Voice → VoiceStatus
  | mk s _ => s

/-- 直接子节点列表。 -/
def children : Voice → List Voice
  | mk _ cs => cs

end Voice

open Voice

/-- 节点是否 active。 -/
def isActive (v : Voice) : Bool :=
  match v.status with
  | active => true
  | closed => false

/--
  ★森林良构·无孤儿不变量（T₄₂ 结构侧核心谓词）。

  `NoOrphan v` ⟺ 「v 若 Closed，则其所有直接子均 Closed」∧「每个子递归满足 NoOrphan」。
  即沿森林任意路径，一旦某节点 Closed，其整棵后代子树皆 Closed——不存在「父已关而子仍活」
  的孤儿。这是 datatype 上的结构归纳谓词（List.Forall 递归到子树）。

  采用嵌套递归（对 children 用 List.all + 递归），DecidablePred 可由 decide 判定（L0）。
-/
def NoOrphan : Voice → Prop
  | mk s cs =>
      (s = closed → ∀ c ∈ cs, c.status = closed)
      ∧ (∀ c ∈ cs, NoOrphan c)

/--
  ★close_voice 后序关闭函数（necessity_derivation.md:251-258 cascade 关子树）。

  `closeVoice v` = 先递归关闭 v 的所有子树（后序），再把 v 自身置 Closed。这就是引擎
  `close_voice` 的后序遍历语义。结构上：返回的节点 status=closed，children 是逐个
  closeVoice 后的子（全 Closed）。
-/
def closeVoice : Voice → Voice
  | mk _ cs => mk closed (cs.map closeVoice)

/-- 辅助：节点经 closeVoice 后自身必 Closed。 -/
theorem closeVoice_status (v : Voice) : (closeVoice v).status = closed := by
  cases v with
  | mk s cs => simp [closeVoice, Voice.status]

/--
  ★closeVoice 后整棵子树全 Closed（后序关闭传播，T₄₂ 关键引理）。

  对 voice 森林结构归纳：closeVoice 把自身和递归所有后代都置 Closed。这形式化
  「关闭节点前先递归关闭其所有活跃子节点」（后序遍历）的结构后果——子树无残留 Active。
-/
theorem closeVoice_all_closed (v : Voice) :
    (closeVoice v).status = closed
    ∧ ∀ c ∈ (closeVoice v).children, c.status = closed := by
  cases v with
  | mk s cs =>
      refine ⟨closeVoice_status (mk s cs), ?_⟩
      intro c hc
      -- c ∈ (cs.map closeVoice) ⟹ c = closeVoice c' for some c' ⟹ status = closed
      simp only [closeVoice, Voice.children] at hc
      obtain ⟨c', _, hc'⟩ := List.mem_map.mp hc
      rw [← hc']
      exact closeVoice_status c'

/--
  ★T₄₂ 孤儿不可能定理（结构侧，L0）：close_voice 后序关闭 ⟹ 无孤儿。

  对 voice 森林结构归纳，证 `NoOrphan (closeVoice v)`：
  - 自身 Closed 时，所有直接子 closeVoice 后皆 Closed（closeVoice_all_closed）；
  - 每个子递归满足 NoOrphan（归纳假设）。
  这 machine-check「父关闭时子必已关闭，永不产生孤儿」——T₄₂ 命题的结构归纳实现。
-/
theorem closeVoice_no_orphan (v : Voice) : NoOrphan (closeVoice v) := by
  -- ★nested inductive（children : List Voice）不支持 induction tactic，改用
  --   well-founded 结构递归（termination_by + decreasing_by sizeOf 下降）。
  match v with
  | mk s cs =>
      unfold NoOrphan closeVoice
      refine ⟨?_, ?_⟩
      · -- 自身 closed → 所有 (cs.map closeVoice) 的子 Closed
        intro _ c hc
        obtain ⟨c', _, hc'⟩ := List.mem_map.mp hc
        rw [← hc']
        exact closeVoice_status c'
      · -- 每个 closeVoice 后的子满足 NoOrphan（结构递归 c' < mk s cs）
        intro c hc
        obtain ⟨c', hc'mem, hc'eq⟩ := List.mem_map.mp hc
        rw [← hc'eq]
        exact closeVoice_no_orphan c'
termination_by v
decreasing_by
  have h1 : sizeOf c' < sizeOf cs := List.sizeOf_lt_of_mem hc'mem
  simp only [Voice.mk.sizeOf_spec]
  omega

/--
  ★森林单根不变量（necessity_derivation.md §883 ①：≤1 active root，T₂₇ 单一持仓主体）。

  `AtMostOneActiveRoot forest` ⟺ 森林根节点中 active 的数量 ≤ 1。这形式化「同时至多一个
  active root」（T₂₇ 永远在场，单一持仓主体）。用根列表的 active 计数 ≤ 1 表达。
-/
def activeRootCount (forest : List Voice) : Nat :=
  (forest.filter isActive).length

def AtMostOneActiveRoot (forest : List Voice) : Prop :=
  activeRootCount forest ≤ 1

/-
  ★诚实声明（codex 审计 thread 019effeb Q5 FAIL 修正）：单根不变量的来源是 **外部前提**
  T₂₇（永远在场，单一持仓主体），**不是森林结构内蕴**——一个任意 active voice 森林结构上
  可以有任意多个 active root（datatype 不禁止）。故「≤1 active root」必须以 T₂₇ 为前提
  声明（`SingleSubjectForest`），不能从 datatype 结构凭空证出。本模块对单根不变量给两条
  诚实定理：(a) 过程不变量 `closeVoice_root_count_noninc`（close 操作不增 active root，真
  过程性质）；(b) 终态推论 `closedForest_zeroActiveRoot`（全关闭后 = 0，平凡终态，不冒充
  活森林一般不变量）。formalization-validity-domain：不声明 datatype 不具备的能力。
-/

/-- 单一持仓主体前提（T₂₇ 编码为森林结构前提，非 datatype 内蕴）。 -/
def SingleSubjectForest (forest : List Voice) : Prop :=
  AtMostOneActiveRoot forest

/--
  ★单根不变量·以 T₂₇ 为前提（§883①忠实形式）：满足单一持仓主体前提的森林 ≤1 active root。
  这如实表达「单根来自 T₂₇ 外部前提」——前提即结论是诚实的恒等（不冒充结构内蕴推导）。
-/
theorem singleSubject_atMostOneRoot (forest : List Voice)
    (h : SingleSubjectForest forest) : AtMostOneActiveRoot forest := h

/--
  ★close 操作的真过程不变量（codex Q5 修正核心）：closeVoice 后 active root 计数 **不增**。

  对森林每个根 closeVoice 后该根必 Closed（closeVoice_status）⟹ filter isActive 后为空 ⟹
  count = 0 ≤ 原 count。这是 **真过程性质**（不是终态平凡）：关闭操作永不凭空产生新的 active
  root，保持单根不变量（若关闭前 ≤1，关闭后仍 ≤1，实际降为 0）。
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
  这是 close_voice_root_count_noninc 的终态特例（计数降为 0），≤1 平凡成立。**仅终态**——
  注释不再声称这是「单根不变量一般成立」（codex Q5 FAIL 已纠正为终态特例）。
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

/--
  ★孤儿不可能 ⟺ 无「父 Closed 而子 Active」（T₄₂ 命题的等价展开，可证伪见证）。

  若 v 满足 NoOrphan 且 v.status = closed，则不存在 active 直接子——这把抽象谓词
  NoOrphan 兑现为命题陈述「父关闭时子必已关闭」。关死「NoOrphan 是恒真占位」疑虑：
  NoOrphan **真的** 排除「父 Closed 而子 Active」的孤儿态。
-/
theorem noOrphan_closed_implies_children_closed (v : Voice)
    (h : NoOrphan v) (hclosed : v.status = closed) :
    ∀ c ∈ v.children, c.status = closed := by
  cases v with
  | mk s cs =>
      unfold NoOrphan at h
      exact h.1 hclosed

/--
  ★NoOrphan 真会拒绝孤儿（可证伪性见证，非恒真）：构造一个父 Closed 而子 Active 的森林，
  它 **不** 满足 NoOrphan。这证明 NoOrphan 是有判别力的真约束（formalization-validity-domain
  L0：定理本身可被反例否证，不是 vacuous）。
-/
theorem orphan_violates_noOrphan :
    ¬ NoOrphan (Voice.mk closed [Voice.mk active []]) := by
  unfold NoOrphan
  intro h
  have hsub := h.1 rfl (Voice.mk active []) (by simp)
  -- hsub : (Voice.mk active []).status = closed，与 active ≠ closed 矛盾
  simp only [Voice.status] at hsub
  exact VoiceStatus.noConfusion hsub

end Formal.Tlayers.Accounting
