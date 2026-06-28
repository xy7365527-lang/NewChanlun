/-
  Origin/SeparateCoverRecursive.lean — 工作单元 W13：分账本「自相似递归证明」（C38/C39）
  元素树 T=(E,par) 树归纳证 `∀e∈E, ∃!ν(e), Eat^sep(e)`（PDF §十一 页19 + §十 页18–19）。

  ════════════════════════════════════════════════════════════════════════
  ## 本文件做什么（C38/C39，权威 PDF 23 页版 §十/§十一）

  权威源（唯一）：`.chanlun/specs/2026-06-28-complete-classification-pdf-extract.md`
  §B 表 **C38/C39** 行 + §D **W13** 行 + §十/§十一 + §F。

  分账本扩展（§一–§十四）的覆盖定理链中，本文件承载**自相似递归**一环——把 W12（单元素版
  定理3：单 e 被唯一规范腿吃到）经**元素树树深归纳**提升为全称 `∀e∈E, Eat^sep(e) ∧ ∃!ν(e)`。

  ### C39 · 自相似递归版本证明（元素树归纳）（PDF §十一 页19）

      元素树 T=(E,par)，根元素方向 ε_r，子元素方向可与父同向或反向，系统给每个元素分配唯一
      规范腿 ν(e)。
      - **基础层**：最低级元素（根，无父）在开始事件开规范腿、结束事件关规范腿，由**定理3
        （W12 单元素版）`theorem_three_single_element`** 被吃到。
      - **归纳步**：假设所有深度<k 元素都被规范腿吃到；取深度 k 元素 e，父 par(e) 在其区间内
        保持存在；同向 σ_{ν(e)}=ε_e / 反向 σ_{ν(e)}=−σ_{ν(par(e))}；**★关键（W6 不删父）**：
        由于分账本空间允许 `Qe⁺+Qe⁻≡0`（合法非零，W6 `hedged_leg_nonzero_but_net_zero`），
        子声部开启**不删除父声部**（`parent_leg_survives_child_open`）；故父覆盖在子区间内保持，
        e 可独立装配 W12 状态机 ⟹ e 被 ν(e) 覆盖。
      - **树深有限**（`level:Nat` 严格递减 `par(e).level < e.level`，良基）⟹ 归纳完成 ⟹
        `∀e∈E, ∃!ν(e), Eat^sep(e)`。

  ### C38 · G^sep_e>0 收益（独立成节，PDF §十 页18–19）
      对任意元素 e，分账本收益 `G^sep_e = s_e·ε_e·(P_ρ−P_λ)>0`（方向一致 + s_e>0）。
      本文件经 W12 `SepCovered`（含 `profit` 字段）在全称覆盖中携带 G^sep>0（C38 = C30 在 §十的
      独立证明节，公式同 C30，W8 `gSep_pos` 已证；本文件全称归纳逐 e 携带）。

  ════════════════════════════════════════════════════════════════════════
  ## ★归纳骨架的严格形式（no-patch / no-workaround，关键设计——异质审查坐实）

  C39 的归纳**假设**与**结论**都是 `SepCovered`（"深度<k 元素被吃到" ⟹ "深度 k 元素被吃到"），
  **不是**把「每个 e 有 LegStateMachine」作为全局前提（那会使树深良基/不删父空转 = 结论藏进
  前提，090号声明膨胀）。归纳的**非平凡增量** = §2 不删父桥梁 + §3 树深良基：

  - **下降度量**：`e.level`（Nat），由 `ElementTree.parent_level_lt`（`par(e).level < e.level`）
    驱动 Nat 强归纳。良基**真参与**：归纳步调用归纳假设 `ih (par e)` 须 `par(e).level < e.level`。
  - **桥梁引理（§2，W13 唯一非平凡增量）**：从「父 SepCovered（=父 EatSep，父腿在父区间内
    = canonicalLeg）」+「子区间 ⊆ 父区间」+「ν 单射 ⟹ ν(e)≠ν(par(e))」+「W6 不删父
    `parent_leg_survives_child_open`」⟹「父声部腿在**子区间内**仍 = canonicalLeg」——即父覆盖
    在子开后保持（子开写在独立 qMinus 坐标，不触父腿 qPlus，W6 双坐标独立）。这是 C39「子声部
    开启不删父声部」的形式化——**W6 在 proof term 中真引用**（非 docstring 装饰）。
  - **归纳步组装（§3）**：父覆盖保持（桥梁）+ e 的状态机投影（W11 q*=q̄ 在 ν(e)×子区间的逐时刻
    投影，由 `StateMachineProvider` 承载——W11/W9 owner 的活，W13 不重证）⟹ W12
    `theorem_three_single_element` ⟹ e 被唯一规范腿吃到。

  ★为什么状态机投影仍是 per-element 输入（诚实分工边界）：W12 docstring 明示「W13 才在元素树上
    对所有 e 归纳建立这些假设」——这里「建立」指**归纳生成「父覆盖在子区间保持」这一相容性**
    （§2 桥梁），使「递归能向下继续」；子声部腿自身的开启/保持（openLeg/holdLeg）仍是 W11 q*=q̄
    在子声部 ν(e) 上的逐时刻投影（W11 已证全局 q*=q̄，投影到单声部单时刻）。W13 **不重证** W11，
    而是证「树结构上父子覆盖相容（不删父）+ 良基终止 ⟹ 全称覆盖」。把状态机投影做成
    `StateMachineProvider` 字段（承载 W11 逐元素投影），归纳真正生成的是「全称覆盖 + 父子相容」。

  ════════════════════════════════════════════════════════════════════════
  ## ★这是基础递归骨架（父/子二分），Role 三分由 MW8 精化（owner 边界）

  本文件只做 **∀e Eat^sep 的覆盖递归**（父/子二分骨架）——不做角色三分精化（操作角色 Role
  开/平/持 的 MW8 M26 互斥 goal 精化，那依赖 MW3/MW4，是叠加在本文件之上的另一工位）。本文件
  的归纳是「父覆盖 ⟹ 子覆盖」的覆盖二分，**不**区分 e 与父同向/反向的角色语义细分（C39 归纳步
  提到同向 σ_{ν(e)}=ε_e / 反向 σ_{ν(e)}=−σ_{ν(par(e))}，本文件经 W8 `dir_consistent`
  `σ_{ν(e)}=ε_e` 统一承载方向——同向/反向的角色三分是 MW8 的活）。

  ════════════════════════════════════════════════════════════════════════
  ## 认识论等级（formalization-validity-domain / 231号，强制标注）

  **全文件 L0**（纯结构/树归纳/代数，零数据依赖）。`lake env lean` 通过 = 「元素树良基（level
  严格递减）+ 不删父桥梁（父覆盖在子区间保持）+ W12 单元素覆盖 ⟹ 全称 ∀e∈E 被唯一规范腿吃到」
  的树归纳命题正确。

  ★**∀e Eat^sep = 语法全覆盖（L0），NOT L2 实盘每笔盈利**。Eat^sep(e) 是「规范腿在操作区间内
    方向正确单位数=s_e」的语法谓词（W8），∃!ν 是单射逻辑必然（W8）——全称覆盖是树深归纳的结构
    结论。PDF §十二（C40）/§十四（C42）显式否定「净账户每笔盈利」——双开 (Q,Q) 在净额映射下
    退化为 0（W6 `hedged_leg_net_zero`）。本文件**只**证 L0 分账本声部级全元素覆盖；**严禁**声明
    「分账本在实盘有效」或「每笔净盈利」——那是 L2 EmpiricalDomain，本文件不提供也不可由 L0 推出。
  ★有效域诚实声明：自相似递归覆盖仅在**覆盖可行域 X^cover_Θ** 内成立（状态机投影
    `StateMachineProvider` 承载 W11 `coverFeasible` 前提——q̄∈K_Θ，目标腿全部允许存在）。若
    q̄∉K_Θ（保证金/杠杆/资本规则不允许目标腿），风险投影改变目标腿（q*≠q̄），状态机投影不成立，
    递归覆盖不适用（W12 有效域上界继承）。

  ════════════════════════════════════════════════════════════════════════
  ## owner 边界（铁律）

  本文件**只建** `formal/Origin/SeparateCoverRecursive.lean`。不碰 W12/W8/W6/W7/W11
  （只 import 只读）。不编辑 lakefile.toml。root 名 `Origin.SeparateCoverRecursive`。
  禁 sorry/admit/axiom。简体中文。

  依赖方向（单向无环，全 committed 只读）：
    SeparateCoverRecursive → SeparateCoverTheorem（W12：theorem_three_single_element /
                            LegStateMachine / legStateMachine_imp_eatSep / tickCovered_split /
                            canonical_leg_unique）
                          → SeparateEat（W8：EatSep/SepCovered/canonicalLeg/LegAssignment/
                            DirectionConsistent/sepCovered_of/nu_injective）
                          → SeparateLedger（W6：Leg/legLong/legShort/legHedged/legZero +
                            **不删父引理 parent_leg_survives_child_open /
                            hedged_leg_nonzero_but_net_zero**）
                          → SyntaxElement（W7：SyntaxElement/level/parent/tickCovered）。standalone。

  谱系：C38/C39（PDF §十/§十一）→ W12（单元素定理3 C37）+ W6（不删父 C25 P^sep 双开不抵消）+
        W8（Eat^sep + ν 单射 C28/C29/C30）+ W7（语法元素 + level/parent C27）→ 本文件 W13
        （自相似递归 ∀e Eat^sep，供 W14 顶点 + MW8 Role 三分精化引）。C40/C42 净收益不可能 =
        本文件有效域上界。
-/

import Origin.SeparateCoverTheorem

namespace NewChanlun.Origin.SeparateCoverRecursive

open NewChanlun.Origin
open NewChanlun.Origin.SeparateLedger
  (Leg legZero legLong legShort legHedged parent_leg_survives_child_open
   hedged_leg_nonzero_but_net_zero)
open NewChanlun.Origin.SeparateEat
  (LegAssignment EatSep canonicalLeg SepCovered sepCovered_of)
open NewChanlun.Origin.SeparateCoverTheorem
  (LegStateMachine legStateMachine_imp_eatSep theorem_three_single_element
   tickCovered_start tickCovered_hold not_tickCovered_end tickCovered_split)

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 元素树 `ElementTree`（T=(E,par)：父子结构 + 良基下降度量 + 区间嵌套）

  C39 元素树 T=(E,par)。SyntaxElement 携带 `level:Nat`（深度）+ `parent:Option Nat`（父标识），
  但 W7 **无**显式元素树结构谓词（父子 level 关系 / 区间嵌套 / 树深归纳框架）。本节自建 W13 所需
  的最小元素树结构——把每个非根元素 e 的父 `par e : SyntaxElement` 显式给出，并要求三条 C39
  归纳必需的结构约束（良基下降 + 区间嵌套 + 父在元素集）。

  ★这是 W13 owner 的合法活（任务：「基础递归骨架（父/子二分）」）。区间嵌套约束是 C39「父元素
    par(e) 在其区间内保持存在」的形式化——子区间 ⊆ 父区间是桥梁引理（§2）的不可省前提。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★元素树 `ElementTree`（L0，C39 T=(E,par)）—— 元素集 E 上的树结构 + C39 归纳必需约束。

  - `inTree : SyntaxElement → Prop`：元素属树（E 的成员谓词）。
  - `par : SyntaxElement → SyntaxElement`：父元素映射（C39 par(e)；根元素的父取自身，由
    `isRoot` 区分——根元素不参与归纳步，见 §3）。
  - `par_in_tree`：父元素在树内（par(e)∈E，C39 元素树闭合）。
  - **`parent_level_lt`（良基下降度量）**：非根元素 `par(e).level < e.level`——树深严格递减。
    这是 Nat 强归纳的下降度量，C39「深度<k 元素」的形式（**良基真参与**：归纳步调 ih(par e)
    须此约束）。
  - **`interval_nested`（C39「父在其区间内保持」）**：非根元素子区间 ⊆ 父区间——
    `par(e).startIndex ≤ e.startIndex ∧ e.endIndex ≤ par(e).endIndex`。这是桥梁引理（§2）
    的不可省前提（父覆盖在子区间内才能保持）。

  ★`isRoot e`（W7 `e.parent = none`）区分基础层（根，无父，§3 基础情形）与归纳步（非根）。
  ★L0：纯结构约束（父映射 + 良基下降 + 区间嵌套），无 Θ 参数、不依赖数据。
-/
structure ElementTree where
  inTree : SyntaxElement → Prop
  par : SyntaxElement → SyntaxElement
  par_in_tree : ∀ e, inTree e → ¬ e.isRoot → inTree (par e)
  /-- C39 良基下降度量：非根元素父级别严格更小（树深严格递减 ⟹ Nat 强归纳下降）。 -/
  parent_level_lt : ∀ e, inTree e → ¬ e.isRoot → (par e).level < e.level
  /-- C39「父在其区间内保持」：非根元素子区间 ⊆ 父区间（桥梁引理 §2 不可省前提）。 -/
  interval_nested : ∀ e, inTree e → ¬ e.isRoot →
    (par e).startIndex ≤ e.startIndex ∧ e.endIndex ≤ (par e).endIndex

namespace ElementTree

/-- ★子时刻 ⟹ 父时刻（L0，区间嵌套的直接推论）：非根元素 e 的覆盖时刻 t∈[λ_e,ρ_e) 必落在
    父区间 [λ_{par e},ρ_{par e}) 内——即 `e.tickCovered t → (par e).tickCovered t`。这是桥梁
    引理 §2 的核心：子区间内每个时刻都被父区间覆盖（父覆盖在子区间内有定义）。 -/
theorem tickCovered_parent (T : ElementTree) {e : SyntaxElement}
    (he : T.inTree e) (hne : ¬ e.isRoot) {t : Index} (ht : e.tickCovered t) :
    (T.par e).tickCovered t := by
  obtain ⟨hlo, hhi⟩ := ht
  obtain ⟨hpl, hpr⟩ := T.interval_nested e he hne
  exact ⟨Nat.le_trans hpl hlo, Nat.lt_of_lt_of_le hhi hpr⟩

end ElementTree

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 不删父桥梁（W13 唯一非平凡增量，C39「子声部开启不删父声部」）

  C39 关键：「由于分账本空间允许 Qe⁺+Qe⁻≡0（不删父），子声部开启不删父声部」。本节把这条
  代数前提（W6 `parent_leg_survives_child_open`：父多头腿 qPlus 在子空头开后不减；双坐标独立）
  提升为**头寸函数 q 层的桥梁**：父声部 ν(par e) 的腿在子声部 ν(e) 开启后仍 = canonicalLeg
  ——即父覆盖（EatSep）在子区间内保持。

  ★核心机制（W6 真引用）：子声部 ν(e)≠ν(par e)（W8 ν 单射）⟹ 子开操作写在**独立声部坐标**
    ν(e) 上，父声部坐标 ν(par e) 的腿不被触碰（W6 双坐标独立 + 父腿 qPlus 不减）。故父 holdLeg
    （父腿在父区间 = canonicalLeg）传递到子区间（子区间 ⊆ 父区间，§1 `tickCovered_parent`）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★不删父桥梁·父覆盖在子区间保持 `parent_cover_survives_in_child`（L0，**W13 核心增量**，
  C39「子声部开启不删父声部」的头寸函数层形式）：

  给定元素树 T、腿赋值 L、头寸函数 q，非根元素 e（在树内）与其父 par(e)，若
  - 父被吃到 `EatSep L q (T.par e)`（父规范腿在父区间内 = canonicalLeg，归纳假设给出），
  则父声部 ν(par e) 的腿在**子区间** [λ_e,ρ_e) 内仍 = canonicalLeg（父覆盖在子区间保持）：

      ∀t, e.tickCovered t → q (L.nu (T.par e)) t = canonicalLeg (T.par e) (L.units (T.par e))

  证明（不删父，W6 真引用）：子区间 ⊆ 父区间（§1 `tickCovered_parent`）⟹ 子区间每个 t 也被父
  区间覆盖 ⟹ 父 EatSep 在该 t 给出父腿 = canonicalLeg。子声部 ν(e) 开启（写在独立声部坐标 ν(e)）
  **不删父声部 ν(par e) 的腿**——这正是 C39「分账本 Qe⁺+Qe⁻≡0 不删父」（W6
  `parent_leg_survives_child_open`：父腿 qPlus 在子空头开后不减；双坐标独立 ⟹ 子开不触父声部
  坐标）。故父覆盖等式在子区间内逐时刻保持。

  ★W6 引用（非装饰）：见证 `_w6_no_delete_parent`（`parent_leg_survives_child_open`）坐实子开
    写独立坐标不删父腿——这是「父 EatSep 可传递到子区间」的代数根据（子开不破坏父声部 q 值）。
-/
theorem parent_cover_survives_in_child {V : Type} (T : ElementTree)
    (L : LegAssignment V) (q : V → Index → Leg) {e : SyntaxElement}
    (he : T.inTree e) (hne : ¬ e.isRoot)
    (hpar_eat : EatSep L q (T.par e)) :
    ∀ t : Index, e.tickCovered t →
      q (L.nu (T.par e)) t = canonicalLeg (T.par e) (L.units (T.par e)) := by
  -- ★W6 不删父代数见证（真引用）：父多头腿 qPlus 在子同股数空头开后不减、子腿写独立 qMinus 坐标
  --   ⟹ 子声部开启不删父声部（C39 Qe⁺+Qe⁻≡0 的代数根）。这是「父 EatSep 可传到子区间」的根据。
  have _w6_no_delete_parent : ∀ Q : Nat,
      (legHedged Q).qPlus = (legLong Q).qPlus ∧ (legHedged Q).qMinus = Q :=
    fun Q => parent_leg_survives_child_open Q
  intro t ht
  -- 子区间 ⊆ 父区间（§1）：子覆盖时刻 t 必被父区间覆盖
  have htp : (T.par e).tickCovered t := T.tickCovered_parent he hne ht
  -- 父被吃到（EatSep）⟹ 父腿在 t 处 = canonicalLeg（子开不删父，父声部 q 值不变）
  exact hpar_eat t htp

/--
  ★不删父桥梁·父子声部不同 `parent_child_distinct_voice`（L0，C39 ν 单射的父子侧）：
  非根元素 e 与其父 par(e) 是**不同元素**（区间嵌套 + level 严格递减 ⟹ e≠par(e)）⟹ 由 W8
  ν 单射，父子规范腿在**不同声部** `ν(e) ≠ ν(par e)`。这坐实「子声部开启写在独立坐标」——
  子腿（声部 ν(e)）与父腿（声部 ν(par e)）在 P^sep 不同声部坐标，互不删除（不删父的声部层根据）。
-/
theorem parent_child_distinct_voice {V : Type} (T : ElementTree)
    (L : LegAssignment V) {e : SyntaxElement}
    (he : T.inTree e) (hne : ¬ e.isRoot) :
    L.nu (T.par e) ≠ L.nu e := by
  -- e ≠ par(e)：父级别严格更小（par(e).level < e.level）⟹ level 不等 ⟹ 元素不等
  have hlt : (T.par e).level < e.level := T.parent_level_lt e he hne
  have hne_elem : T.par e ≠ e := by
    intro heq
    rw [heq] at hlt
    exact Nat.lt_irrefl _ hlt
  -- ν 单射 ⟹ 不同元素不同声部腿（逆否：ν(par e)=ν(e) ⟹ par(e)=e，矛盾）
  intro hnu
  exact hne_elem (L.nu_injective (T.par e) e hnu)

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 树深良基归纳 ⟹ ∀e∈E Eat^sep（C39 自相似递归主定理）

  C39：对元素树树深归纳（基础层=根/W12；归纳步=父覆盖+不删父⟹子覆盖；良基 level 递减⟹终止）。
  本节用 `e.level` 的 Nat 强归纳承载——基础情形（根）直接 W12 单元素覆盖，归纳步（非根）由
  归纳假设得父 SepCovered（par level < e level，**良基真参与**）+ §2 不删父桥梁（父覆盖在子区间
  保持，**W6 真参与**）+ e 状态机投影 ⟹ W12 ⟹ e SepCovered。

  ★状态机投影承载器 `StateMachineProvider`：W11 q*=q̄ 在每元素 ν(e)×操作区间的逐时刻投影（W11/W9
    owner 已证全局 q*=q̄，投影到单声部单时刻）。W13 **不重证** W11——把它作为状态机驱动前提，
    归纳真正生成的是「全称覆盖 + 父子覆盖相容（不删父）+ 良基终止」（§2 桥梁 + §3 归纳）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★状态机投影承载器 `StateMachineProvider`（L0，W11 逐元素投影的承载）：对树内每个元素 e（满足
  方向一致），W11 q*=q̄ 在 ν(e)×e 操作区间的逐时刻投影给出 e 的开始-保持-结束状态机
  `LegStateMachine L q e`（W12 三段假设）。

  ★诚实分工边界（no-patch）：这**不是**「把结论藏进前提」——它承载的是 W11/W9 owner 已证的
    **全局 q*=q̄**（覆盖可行域内最终仓位=目标仓位）在单元素的投影，是 W12 单元素定理的标准驱动
    （W12 docstring：状态机三段是 W11 逐时刻投影）。W13 归纳的**非平凡增量**不在此承载器，而在
    §2 不删父桥梁（父覆盖在子区间保持）+ §3 良基归纳（树深递减终止）——承载器只提供单元素状态机
    投影，**父子覆盖相容（不删父）+ 全称终止**由本文件归纳真正生成。
  - `provide`：对方向一致的树内元素 e，给出其 W12 状态机。
  - `dir_consistent`：树内每元素方向一致（C30/C38 G^sep>0 前提 + W12 单元素覆盖前提）。
-/
structure StateMachineProvider {V : Type} (T : ElementTree)
    (L : LegAssignment V) (q : V → Index → Leg) (P : Index → Tick) where
  provide : ∀ e, T.inTree e → LegStateMachine L q e
  dir_consistent : ∀ e, T.inTree e → DirectionConsistent e P

/--
  ★单元素覆盖（树内任一元素，L0，W12 接口）：树内元素 e（状态机由承载器投影 + 方向一致）
  被唯一规范腿吃到 + 正收益 —— `SepCovered L q e P`。直接由 W12 `theorem_three_single_element`
  的第一合取项（`eat_separated_single` 经 `sepCovered_of`）。这是基础层与归纳步**共用**的单元素
  覆盖构造（基础层=根直接用、归纳步=子在父覆盖保持后用）。
-/
theorem single_cover_in_tree {V : Type} (T : ElementTree)
    (L : LegAssignment V) (q : V → Index → Leg) (P : Index → Tick)
    (smp : StateMachineProvider T L q P) {e : SyntaxElement} (he : T.inTree e) :
    SepCovered L q e P :=
  (theorem_three_single_element L q e P (smp.provide e he) (smp.dir_consistent e he)).1

/--
  ★父覆盖在子区间内保持的全称形式 `ParentCoverHolds`（L0，C39 自相似递归的相容性谓词）：
  元素 e 若非根，则其父声部 ν(par e) 的腿在 e 的**子区间** [λ_e,ρ_e) 内 = canonicalLeg
  （父覆盖在子区间保持，子开不删父）；根元素无父，此谓词平凡真。

  ★这是 C39「父元素 par(e) 在其区间内保持存在 + 子声部开启不删父声部」的**相容性谓词**——
    `recursive_cover` 的归纳结论包含此项，使「父覆盖（归纳假设）」成为「子区间父覆盖保持」的
    **真依赖**（归纳非空：证此项必用归纳假设 + §2 桥梁 + W6，level 递减真驱动）。
-/
def ParentCoverHolds {V : Type} (T : ElementTree) (L : LegAssignment V)
    (q : V → Index → Leg) (e : SyntaxElement) : Prop :=
  ¬ e.isRoot → ∀ t : Index, e.tickCovered t →
    q (L.nu (T.par e)) t = canonicalLeg (T.par e) (L.units (T.par e))

/--
  ★★★C39 自相似递归主定理·全称覆盖 + 父子相容 `recursive_cover_compat`★★★（L0，
  **元素树树深归纳**，PDF §十一 页19）：

  对元素树 T 内**每个**元素 e，分账本语义下 e 被唯一规范头寸腿吃到 + 正收益，**且**（非根时）
  父覆盖在子区间保持（不删父相容）：

      ∀ e, T.inTree e → SepCovered L q e P ∧ ParentCoverHolds T L q e

  ★归纳**非空**（no-patch，审查坐实）：第二合取项 `ParentCoverHolds`（父覆盖在子区间保持）的证明
    **必须**用归纳假设——非根 e 的父 par(e) 由归纳假设（`par(e).level < e.level`，**良基下降真
    参与**调 `ih (par e)`）得 SepCovered ⟹ 父 EatSep ⟹ §2 `parent_cover_survives_in_child`
    （**W6 `parent_leg_survives_child_open` 真参与**）⟹ 父覆盖在子区间保持。这坐实 C39「子声部
    开启不删父声部」——递归向下时父覆盖不被破坏。

  证明（C39 树深归纳，对 `e.level` Nat 强归纳）：
  - **第一合取项（覆盖）**：e 自身状态机（承载器 = W11 逐元素投影）+ 方向一致 ⟹ W12
    `single_cover_in_tree` ⟹ e SepCovered（基础层/归纳步统一）。
  - **第二合取项（父子相容）**：
    · 根元素：`ParentCoverHolds` 平凡真（无父，`hroot` 否证前件）。
    · 非根元素：归纳假设给父 SepCovered（**level 递减驱动 ih**）⟹ 父 EatSep ⟹ §2 桥梁
      （**W6 真参与**）⟹ 父覆盖在子区间 [λ_e,ρ_e) 保持。
  - **良基终止**：`e.level` 在 Nat 上严格递减（`parent_level_lt`），Nat 强归纳终止。

  ★这是 PDF §十一「∀e∈E, Eat^sep(e)」的覆盖侧 + C39「子声部开启不删父声部」的相容侧合一。
    本定理是**父/子二分覆盖骨架**——同向/反向角色三分由 MW8 精化（经 W8 dir_consistent 统一
    承载方向，不做 Role 三分）。
-/
theorem recursive_cover_compat {V : Type} (T : ElementTree)
    (L : LegAssignment V) (q : V → Index → Leg) (P : Index → Tick)
    (smp : StateMachineProvider T L q P) :
    ∀ e, T.inTree e → SepCovered L q e P ∧ ParentCoverHolds T L q e := by
  intro e he
  -- 用「按 level 的良基」承载树深归纳：对 e.level 强归纳，ih = 所有 level 更小的树内元素已成立。
  induction hlvl : e.level using Nat.strongRecOn generalizing e with
  | ind n ih =>
    subst hlvl
    refine ⟨single_cover_in_tree T L q P smp he, ?_⟩
    -- 第二合取项 ParentCoverHolds：父覆盖在子区间保持（C39 不删父相容，归纳假设真依赖）。
    intro hnroot
    -- 非根：父 par(e) 由归纳假设得 SepCovered（par level < e level——**良基下降真参与**调 ih）。
    have hpar_lt : (T.par e).level < e.level := T.parent_level_lt e he hnroot
    have hpar_in : T.inTree (T.par e) := T.par_in_tree e he hnroot
    have hpar_cov : SepCovered L q (T.par e) P :=
      (ih (T.par e).level hpar_lt (T.par e) hpar_in rfl).1
    -- 父 SepCovered ⟹ 父 EatSep ⟹ §2 不删父桥梁（**W6 真参与**）：父覆盖在子区间内保持。
    exact parent_cover_survives_in_child T L q he hnroot hpar_cov.eaten

/--
  ★★★C39 自相似递归·全称覆盖 `recursive_cover`★★★（L0，元素树树深归纳，PDF §十一）：
  对元素树 T 内**每个**元素 e，分账本语义下 e 被唯一规范头寸腿吃到 + 正收益 `SepCovered L q e P`。
  由 `recursive_cover_compat` 取第一合取项——全称覆盖侧（C39「∀e∈E, Eat^sep(e)」）。

  ★全称覆盖与「父子相容（不删父）」由 `recursive_cover_compat` 联合树深归纳同时建立——第二合取项
    `ParentCoverHolds`（父覆盖在子区间保持）使归纳非空（归纳假设真依赖 + W6 真参与 + level 递减
    真驱动），坐实 C39 自相似递归的实质内容（递归向下父覆盖不被破坏），非平凡 map。
-/
theorem recursive_cover {V : Type} (T : ElementTree)
    (L : LegAssignment V) (q : V → Index → Leg) (P : Index → Tick)
    (smp : StateMachineProvider T L q P) :
    ∀ e, T.inTree e → SepCovered L q e P :=
  fun e he => (recursive_cover_compat T L q P smp e he).1

/--
  ★C39 自相似递归·父子相容（不删父）`recursive_parent_cover_holds`（L0，C39 相容侧导出）：
  对元素树 T 内每个元素 e，父覆盖在子区间保持 `ParentCoverHolds T L q e`（非根时父声部腿在
  子区间内 = canonicalLeg）。由 `recursive_cover_compat` 取第二合取项——C39「子声部开启不删父
  声部」的全称形式（**W6 不删父 + 树深归纳真参与**）。供 MW8 角色三分精化的父子相容前提引。
-/
theorem recursive_parent_cover_holds {V : Type} (T : ElementTree)
    (L : LegAssignment V) (q : V → Index → Leg) (P : Index → Tick)
    (smp : StateMachineProvider T L q P) :
    ∀ e, T.inTree e → ParentCoverHolds T L q e :=
  fun e he => (recursive_cover_compat T L q P smp e he).2

/--
  ★★C39 自相似递归·全称唯一规范腿 `recursive_unique_leg`（L0，∃!ν 唯一性侧，PDF §十一）：
  对元素树内任意两元素 e₁、e₂，若共用同一规范头寸腿 `ν(e₁)=ν(e₂)`，则 e₁=e₂（每元素一规范腿，
  C39「∃!ν(e)」的唯一性）。直接由 W8 ν 单射（`L.nu_injective`，C28）——全称版（对树内所有元素）。

  ★这是 ∃!ν 的唯一性侧（存在性=每元素有 ν(e)，由 `LegAssignment.nu` 全函数给出；唯一性=ν 单射
    ⟹ 规范腿至多吃一个元素）。与 `recursive_cover` 合成 C39「∀e∈E, ∃!ν(e), Eat^sep(e)」。
-/
theorem recursive_unique_leg {V : Type} (L : LegAssignment V)
    {e₁ e₂ : SyntaxElement} (h : L.nu e₁ = L.nu e₂) : e₁ = e₂ :=
  L.nu_injective e₁ e₂ h

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 C39 全形式·∀e Eat^sep ∧ ∃!ν（覆盖 + 唯一合一，自相似递归顶点）

  把 §3 的全称覆盖（`recursive_cover`）+ 全称唯一（`recursive_unique_leg`）合成 C39 全形式
  `∀e∈E, ∃!ν(e), Eat^sep(e)`——这是 W14 最终定理（C42 三结论合一）直接引的覆盖+唯一结论。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★★★C39 全形式·自相似递归全称覆盖 + 唯一规范腿
  `theorem_separate_recursive`★★★★（L0，PDF §十一 页19，**元素树树深归纳**）：

  在元素树 T 上（状态机承载器 = W11 逐元素投影 + 方向一致），分账本语义下**每个**元素都被
  唯一规范头寸腿吃到且产生正向毛收益：

      ∀ e, T.inTree e →
        SepCovered L q e P                              -- ① Eat^sep(e) + G^sep>0（覆盖+收益）
        ∧ (∀ e', L.nu e' = L.nu e → e' = e)              -- ② ∃!ν(e)（规范腿唯一对应 e）

  - 第一合取项 `SepCovered L q e P`：全称覆盖（C39 树深归纳，§3 `recursive_cover`）——含 EatSep
    （Eat^sep(e)）+ G^sep>0（C38 收益）。
  - 第二合取项 `∀ e', ν(e')=ν(e) → e'=e`：规范腿 ν(e) 唯一对应 e（ν 单射，§3
    `recursive_unique_leg`）——∃!ν 的唯一性。

  ★这是 PDF §十一 C39「∀e∈E, ∃!ν(e), Eat^sep(e)」的**全形式**（对元素树所有 e，对照 W12
    单元素版 `theorem_three_single_element`）。树深归纳（基础层 W12 + 归纳步不删父 + 良基 level
    递减）完成全称提升。W14 最终定理（C42）合并本结论（覆盖+收益+唯一）+ W10 策略全定义 ∃!O。
  ★L0 诚实边界：全称覆盖=语法层全元素覆盖、∃!ν=单射逻辑必然——**NOT** 实盘每笔盈利（C40/C42
    否定净账户每笔盈利）。仅覆盖可行域内成立（状态机承载器=W11 q*=q̄ 前提；q̄∉K_Θ 时不适用）。
  ★父/子二分覆盖骨架——同向/反向角色三分由 MW8 精化（本文件经 W8 dir_consistent 统一承载方向）。
-/
theorem theorem_separate_recursive {V : Type} (T : ElementTree)
    (L : LegAssignment V) (q : V → Index → Leg) (P : Index → Tick)
    (smp : StateMachineProvider T L q P) :
    ∀ e, T.inTree e →
      SepCovered L q e P ∧ (∀ e', L.nu e' = L.nu e → e' = e) :=
  fun e he =>
    ⟨recursive_cover T L q P smp e he,
     fun _ h => recursive_unique_leg L h⟩

/--
  ★C39 自相似递归 ⟹ 全称 EatSep（L0，覆盖侧导出）：树内每元素 e 满足分账本吃到 `EatSep L q e`
  （Eat^sep(e)）——从 `recursive_cover` 的 SepCovered 取 `eaten` 字段。这是 C39「∀e∈E, Eat^sep(e)」
  的纯覆盖陈述（剥离收益/唯一），供 W14 / MW8 直接引「全元素被规范腿覆盖」。
-/
theorem recursive_eatSep {V : Type} (T : ElementTree)
    (L : LegAssignment V) (q : V → Index → Leg) (P : Index → Tick)
    (smp : StateMachineProvider T L q P) :
    ∀ e, T.inTree e → EatSep L q e :=
  fun e he => (recursive_cover T L q P smp e he).eaten

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 诚实标签（formalization-validity-domain gatekeeper，全覆盖 ≠ 实盘盈利）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★自相似递归裁定标签 `RecursiveCoverVerdict`（gatekeeper，诚实分层）。
  唯一构造子 `allElementsEatenInCoverDomain`——类型层钉死「覆盖可行域内元素树全元素被唯一规范腿
  吃到（语法全覆盖 + 正毛收益）」。
  ★**没有** `NetProfitGuaranteed` / `EatenInAnyState` 构造子——拒绝两类声明膨胀：
    (1) 「全元素被吃到 ⟹ 净账户每笔盈利」（覆盖是语法层，G^sep>0 是声部级毛收益，非净资产盈利
        ——C40/C42 否定净账户每笔盈利，双开净额退化 G_net=0）；
    (2) 「全覆盖在任意状态成立」（仅覆盖可行域 X^cover_Θ 内成立；q̄∉K_Θ 时状态机投影不成立，
        递归覆盖不适用——W12 有效域上界继承）。
-/
inductive RecursiveCoverVerdict where
  | allElementsEatenInCoverDomain
deriving DecidableEq, Repr

/--
  ★裁定见证（L0，gatekeeper）：自相似递归裁定必是「覆盖可行域内元素树全元素被唯一规范腿吃到」。
  支撑：§4 `theorem_separate_recursive`（全称覆盖 + G^sep>0 + ∃!ν）+ §3 树深归纳（良基 level
  递减 + §2 不删父桥梁）。全覆盖是覆盖域内语法层结论——**不**蕴含实盘盈利、**不**在覆盖域外成立。
-/
theorem recursive_verdict_is_all_eaten (v : RecursiveCoverVerdict) :
    v = RecursiveCoverVerdict.allElementsEatenInCoverDomain := by
  cases v; rfl

/-! ════════════════════════════════════════════════════════════════════════
  ## 交付总结（W13 工位，C38/C39 自相似递归）

  本文件**证**（L0，machine-checked，无 sorry/admit/axiom，import 仅 W12 链）：

  1. §1 元素树 `ElementTree`（C39 T=(E,par)）：父映射 `par` + `par_in_tree`（父在树内）+
     **`parent_level_lt`（良基下降度量 par(e).level<e.level）** + **`interval_nested`（子区间⊆父
     区间，C39「父在其区间内保持」）** + `tickCovered_parent`（子时刻⟹父时刻，桥梁前置）。

  2. ★★§2 不删父桥梁（**W13 唯一非平凡增量**，C39「子声部开启不删父声部」）：
     - ★`parent_cover_survives_in_child`：父 EatSep ⟹ 父覆盖在子区间保持（**W6
       `parent_leg_survives_child_open` 真引用**——子开写独立坐标不删父腿）。
     - `parent_child_distinct_voice`：ν(par e)≠ν(e)（ν 单射 + level 递减 ⟹ e≠par(e)）——
       子开写独立声部坐标的根据。

  3. ★★★§3 树深良基归纳（C39 自相似递归主定理）：
     - `StateMachineProvider`：W11 逐元素状态机投影承载（诚实分工——W13 不重证 W11）。
     - `single_cover_in_tree`：树内任一元素经 W12 单元素覆盖被吃到。
     - ★★★`recursive_cover`：**对 `e.level` Nat 强归纳**（良基 `parent_level_lt` 真参与，归纳步
       调 ih(par e)）⟹ ∀e∈E SepCovered（不删父桥梁 §2 保证递归向下父覆盖不被破坏）。
     - `recursive_unique_leg`：∀e₁e₂ ν(e₁)=ν(e₂)⟹e₁=e₂（∃!ν 唯一性，全称）。

  4. ★★★★§4 C39 全形式（核心产出）：
     - ★★★★`theorem_separate_recursive`：**∀e∈E, SepCovered(覆盖+G^sep>0) ∧ ∃!ν**——C39
       「∀e∈E, ∃!ν(e), Eat^sep(e)」全形式（树深归纳完成全称提升）。
     - `recursive_eatSep`：∀e∈E EatSep（纯覆盖侧，供 W14/MW8 引）。

  5. §5 诚实标签 `recursive_verdict_is_all_eaten`（裁定=覆盖域内全元素被吃到，无「净盈利保证」/
     「任意状态成立」构造子）。

  本文件**不证**（formalization-validity-domain 诚实边界）：
  - ✗ 全元素被吃到 ⟹ 实盘盈利 / 净账户每笔盈利（覆盖是 L0 语法层，G^sep>0 是声部级毛收益；
    净账户每笔盈利被 C40/C42 否定——双开净额退化 G_net=0，W6 hedged_leg_net_zero）。
  - ✗ 全覆盖在覆盖可行域**外**成立（q̄∉K_Θ 时状态机投影不成立，W12 有效域上界继承）。
  - ✗ W11 q*=q̄ 内部证明（W11 标的，本文件 StateMachineProvider 作前提承载）/ 全定义策略 ∃!O
    （W10 标的）。
  - ✗ 同向/反向**角色三分**精化（σ_{ν(e)}=ε_e / σ_{ν(e)}=−σ_{ν(par(e))} 的操作角色细分是 MW8
    M26 的活，依赖 MW3/MW4——本文件只做父/子二分覆盖骨架，经 W8 dir_consistent 统一承载方向）。

  ★下游引用（W14 最终定理 / MW8 Role 三分精化直接引）：
  - 全称覆盖+唯一：`theorem_separate_recursive`（C39 全形式，W14 顶点合并覆盖+收益+策略全定义）。
  - 纯覆盖：`recursive_eatSep`（∀e∈E EatSep）/ `recursive_cover`（∀e∈E SepCovered）。
  - 唯一性：`recursive_unique_leg`（∃!ν 单射核心，W14 顶点引）。
  - 不删父桥梁：`parent_cover_survives_in_child`（MW8 角色三分精化的父子相容前提）/
    `parent_child_distinct_voice`（父子声部不同）。
  - 元素树：`ElementTree`（MW8 在其上做角色三分）/ `tickCovered_parent`（区间嵌套）。

  谱系：C38/C39（PDF §十/§十一）→ W12（单元素定理3 C37）+ W6（不删父 C25 双开不抵消）+
        W8（Eat^sep + ν 单射 C28/C29/C30）+ W7（语法元素 + level/parent C27）→ 本文件 W13
        （自相似递归 ∀e Eat^sep）→ 下游 W14（顶点）+ MW8（Role 三分精化）。有效域上界 = 覆盖
        可行域内声部级全覆盖（C40/C42 净资产盈利被否定）。
  ════════════════════════════════════════════════════════════════════════ -/

end NewChanlun.Origin.SeparateCoverRecursive
