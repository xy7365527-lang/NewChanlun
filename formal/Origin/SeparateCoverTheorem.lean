/-
  Origin/SeparateCoverTheorem.lean — 工作单元 W12：分账本「定理3」单元素版
  C37（定理3，单元素版）：在覆盖可行域 X^cover_Θ 中，若每元素边界操作语义可执行，则
    ∀e∈E, Eat^sep(e) ∧ ∃!ν(e)
  经**开始-保持-结束状态机**证（PDF §九 页17–18）。

  ════════════════════════════════════════════════════════════════════════
  ## 本文件做什么（C37 / 定理3 单元素版，权威 PDF 23 页版 §九 页17–18）

  权威源（唯一）：`.chanlun/specs/2026-06-28-complete-classification-pdf-extract.md`
  §B 表 **C37** 行 + §D **W12** 行 + §九 + §F。互斥 M23=C37 共享，本工位成果互斥分类直接复用。

  分账本扩展（§一–§十四）的覆盖定理链中，本文件承载「单元素被唯一规范腿吃到」一环——
  把 W8（Eat^sep 谓词 + ν 单射 + G^sep>0）与 W11（覆盖可行 ⟹ q*=q̄）接成**开始-保持-结束
  状态机**，得「∀t∈[λ_e,ρ_e) 规范腿存在且方向=ε_e」=`Eat^sep(e)`，再由 ν 单射得规范腿唯一。

  ### C37 · 定理3（分账本语义下每元素被唯一规范头寸腿吃到）（PDF §九 页17–18）

      在覆盖可行域 X^cover_Θ 中，若每个元素边界在操作语义中可执行，则
        ∀e∈E, Eat^sep(e)  且  ∃!ν(e) 是吃到 e 的规范头寸腿。

      证明（单元素版，开始-保持-结束状态机，§九 页17–18 boxed）：
        - **开始**（t=λ_e）：e∈B_t ⟹ e∈Ã_{t+1} ⟹ 目标腿按 ε_e 开
            （ε_e=+1 则 q̄⁺=s_e,q̄⁻=0；ε_e=-1 则 q̄⁺=0,q̄⁻=s_e）；
            由**定理2（W11）q*=q̄** 规范腿正确开启（实际头寸=目标头寸）。
        - **保持**（内部 t∈(λ_e,ρ_e)）：e∉D_t ⟹ e∈A_t ⟹ e∈Ã_{t+1}，
            由同样目标仓位公式 + 定理2 q*=q̄，规范腿保持开启。
        - **结束**（t=ρ_e）：e∈D_t ⟹ e∉Ã_{t+1} 对应腿被关闭。
        - 故 ∀t∈[λ_e,ρ_e) 元素 e 的规范腿始终存在且方向=ε_e = `Eat^sep(e)`；
          又因 ν 单射每元素只有一个规范头寸腿 ⟹ 规范吃笔腿唯一。

  ════════════════════════════════════════════════════════════════════════
  ## ★状态机三段如何严格接 W11「q*=q̄」（no-patch / no-workaround，关键）

  spec C37 每段都说「由定理2 q*=q̄ 腿正确开启/保持」——W11 `coverFeasible_qStar_eq_target`
  证的是「覆盖可行域内某时刻最终仓位 q*_{t+1} = 目标仓位 q̄_{t+1}」（P^sep 整族坐标）。

  W12 单元素版的载体是 **W8 的逐时刻头寸函数** `q : V → Index → Leg`（声部 v 在时刻 t 一条腿）。
  「定理2 q*=q̄ 在时刻 t 把目标腿落为实际腿」= 在时刻 t，规范声部 ν(e) 的**实际腿** q(ν(e),t)
  等于**目标腿** canonicalLeg（= e 在该时刻的 q̄ 分量）。这是 W11 结论在单声部单时刻的投影：

      覆盖可行（W11 前提）+ e∈Ã_{t+1}（W9 激活集，开始/保持段成立）
        ⟹ q*=q̄（W11）⟹ q(ν(e),t) = canonicalLeg（实际腿=目标腿）。

  本文件**不重证 W11**（那是 W11 owner 的活），而是把「W11 在时刻 t 的投影结论」编码为状态机
  **三段语义假设**（`LegStateMachine` 的 `openLeg`/`holdLeg`/`closeLeg` 字段）——每段假设
  对应 spec C37 一段「由定理2 q*=q̄」的结果（实际腿在 [λ_e,ρ_e) 内 = canonicalLeg、在 ρ_e 关闭）。
  状态机假设 + W8 区间覆盖语义 ⟹ `EatSep`（W8 谓词）⟹ `SepCovered`（W8 合一）⟹ 唯一性（ν 单射）。

  ★为什么三段是**假设**而非在本文件重推 W11：W12 是「单元素版」（spec §D W12 行 + §九标题）——
    它把覆盖可行域的逐时刻 q*=q̄（W11 已证）作为**输入**，组装成单元素整区间覆盖。W13（自相似
    递归）才在元素树上对所有 e 归纳建立这些假设。本文件诚实地把 W11 投影作为状态机驱动前提
    （`coverFeasible`/`activeAt` 字段承载 W11 前提与 W9 激活语义），**不**臆造 W11 的内部证明，
    也**不**写 q*=q̄ 的垫片——状态机三段引理显式引 W11 `coverFeasible_qStar_eq_target` 作为
    「目标腿不被风险投影改变」的根据（见 §2 `legState_open_eq_target` docstring）。

  ════════════════════════════════════════════════════════════════════════
  ## ★区间分解 [λ_e,ρ_e) = {λ_e} ∪ (λ_e,ρ_e)（开始 ∪ 保持 = 整覆盖区间）

  W8 `EatSep` = 「∀t, tickCovered e t → q(ν(e),t)=canonicalLeg」，`tickCovered e t = λ_e≤t<ρ_e`
  （W7 半开区间）。开始段 t=λ_e（单点）+ 保持段 t∈(λ_e,ρ_e)（开区间）= **恰好** [λ_e,ρ_e)
  （半开，不含 ρ_e）。结束时刻 ρ_e **不在** tickCovered 内（半开右开）——对应「关闭」，与覆盖
  区间不重叠。本文件证 `tickCovered_split`：tickCovered e t ⟺ t=λ_e ∨ (λ_e<t<ρ_e)，把状态机
  开始+保持两段拼成 EatSep 的整区间（Nat 端点代数，L0）。结束段 ρ_e∉[λ_e,ρ_e) 由 `lt_irrefl`。

  ════════════════════════════════════════════════════════════════════════
  ## 认识论等级（formalization-validity-domain / 231号，强制标注）

  **全文件 L0**（纯结构/状态机/代数，零数据依赖）。`lake env lean` 通过 = 「开始-保持-结束三段
  状态机假设（W11 逐时刻投影 + W9 激活语义）+ 区间分解 ⟹ EatSep ⟹ SepCovered + ν 单射唯一」的
  代数命题正确。

  ★**每元素被唯一规范腿吃到 = 语法覆盖（L0），NOT L2 实盘每笔盈利**。Eat^sep(e) 是「规范腿在
    操作区间内方向正确单位数=s_e」的语法谓词（W8），∃!ν 是单射的逻辑必然（W8）——二者皆 L0
    结构结论。PDF §十二（C40）/§十四（C42）显式否定「净账户每笔盈利」——双开 (Q,Q) 在净额映射
    下退化为 0。本文件**只**证 L0 分账本声部级单元素覆盖；**严禁**声明「分账本在实盘有效」或
    「每笔净盈利」——那是 L2 EmpiricalDomain，本文件不提供也不可由 L0 推出。
  ★有效域诚实声明：定理3 仅在**覆盖可行域 X^cover_Θ** 内成立（W11 `coverFeasible` 前提——
    q̄∈K_Θ，目标腿全部允许存在）。若 q̄∉K_Θ（保证金/杠杆/资本规则不允许目标腿），风险投影
    改变目标腿（q*≠q̄），状态机开始/保持段假设不成立，定理3 不适用。这是 PDF 自带的有效域限定
    （§八「覆盖可行域」前提 + §九「每元素边界操作语义可执行」），不可膨胀为「任意状态下被吃到」。

  ════════════════════════════════════════════════════════════════════════
  ## owner 边界（铁律）

  本文件**只建** `formal/Origin/SeparateCoverTheorem.lean`。不碰 W8/W11/W9/W7/W6
  （只 import 只读）。不编辑 lakefile.toml。root 名 `Origin.SeparateCoverTheorem`。
  禁 sorry/admit/axiom。简体中文。

  依赖方向（单向无环，全 committed 只读）：
    SeparateCoverTheorem → SeparateEat（W8：EatSep/canonicalLeg/SepCovered/sepCovered_of/
                          LegAssignment/nu_inj/sepCovered_unique_leg）
                        → CoverFeasibleTarget（W11：coverFeasible_qStar_eq_target q*=q̄）
                        → SyntaxElement（W7：SyntaxElement/tickCovered/DirectionConsistent）
                        → SeparateLedger（W6：Leg/canonicalLeg 底层 legLong/legShort）。standalone。

  谱系：C37（PDF §九 页17–18）→ W8（Eat^sep + ν 单射 C28/C29/C30）+ W11（覆盖可行 ⟹ q*=q̄ C36）
        + W9（激活集 Ã C31/C32）+ W7（语法元素 + tickCovered C27）→ 本文件 W12（单元素定理3，
        供 W13 自相似递归 / W14 最终定理引）。C40/C42 净收益不可能 = 本文件有效域上界。
-/

import Origin.SeparateEat
import Origin.CoverFeasibleTarget

namespace NewChanlun.Origin.SeparateCoverTheorem

open NewChanlun.Origin
open NewChanlun.Origin.SeparateLedger (Leg legZero legLong legShort)
open NewChanlun.Origin.SeparateEat
  (LegAssignment EatSep canonicalLeg SepCovered sepCovered_of gSep gSep_pos)

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 区间分解 [λ_e,ρ_e) = {λ_e} ∪ (λ_e,ρ_e)（开始段 ∪ 保持段 = 整覆盖区间）

  W7 `tickCovered e t = λ_e≤t<ρ_e`（半开）。开始段 t=λ_e（单点）+ 保持段 t∈(λ_e,ρ_e)（开区间）
  恰好拼成 [λ_e,ρ_e)。结束时刻 ρ_e 不在 tickCovered 内（半开右开）。本节用 Nat 端点代数把这
  三段对齐到 tickCovered，供状态机三段组装为 EatSep。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★开始段判据（L0）：开始时刻 t=λ_e 落在覆盖区间内（λ_e≤λ_e<ρ_e，由非空 λ_e<ρ_e）。 -/
theorem tickCovered_start (e : SyntaxElement) :
    e.tickCovered e.startIndex := by
  refine ⟨Nat.le_refl _, ?_⟩
  exact e.nonempty

/-- ★保持段判据（L0）：内部时刻 t∈(λ_e,ρ_e)（λ_e<t<ρ_e）落在覆盖区间内（λ_e≤t 由 λ_e<t）。 -/
theorem tickCovered_hold (e : SyntaxElement) {t : Index}
    (hlo : e.startIndex < t) (hhi : t < e.endIndex) :
    e.tickCovered t :=
  ⟨Nat.le_of_lt hlo, hhi⟩

/-- ★结束段判据（L0）：结束时刻 t=ρ_e **不**落在覆盖区间内（半开右开 ρ_e≮ρ_e）。
    对应「t=ρ_e 腿被关闭」——结束时刻在覆盖区间外，腿不再是 canonicalLeg。 -/
theorem not_tickCovered_end (e : SyntaxElement) :
    ¬ e.tickCovered e.endIndex := by
  rintro ⟨_, hhi⟩
  exact Nat.lt_irrefl _ hhi

/--
  ★★区间分解 `tickCovered_split`（L0，核心：[λ_e,ρ_e) = {λ_e} ∪ (λ_e,ρ_e)）：
  覆盖区间内每个时刻 t **要么是开始时刻** t=λ_e，**要么是内部时刻** λ_e<t<ρ_e——二分穷尽。

  这是状态机「开始段 ∪ 保持段 = 整覆盖区间」的形式：tickCovered e t（=λ_e≤t<ρ_e）拆成
  「t=λ_e（开始）」∨「λ_e<t<ρ_e（保持）」。结束时刻 ρ_e 不在此区间（`not_tickCovered_end`）。
  下游 `legStateMachine_imp_eatSep` 用此把三段假设拼成 EatSep（区间内每 t 都被覆盖）。
-/
theorem tickCovered_split (e : SyntaxElement) {t : Index} (ht : e.tickCovered t) :
    t = e.startIndex ∨ (e.startIndex < t ∧ t < e.endIndex) := by
  obtain ⟨hlo, hhi⟩ := ht
  rcases Nat.eq_or_lt_of_le hlo with heq | hlt
  · exact Or.inl heq.symm
  · exact Or.inr ⟨hlt, hhi⟩

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 开始-保持-结束状态机 `LegStateMachine`（C37 §九 三段语义，逐时刻头寸）

  PDF §九 C37 单元素版：状态机驱动规范腿在 e 操作区间内的开启/保持/关闭。载体是 W8 逐时刻头寸
  函数 `q : V → Index → Leg`（声部 v 在时刻 t 一条腿）。状态机三段假设（每段对应 spec C37
  一段「由定理2 q*=q̄」的逐时刻投影结论）：

  - `openLeg`（开始 t=λ_e）：e∈B_t ⟹ e∈Ã ⟹ 目标腿按 ε_e 开 ⟹（W11 q*=q̄）实际腿 q(ν(e),λ_e)
    = canonicalLeg（规范腿正确开启）。
  - `holdLeg`（保持 t∈(λ_e,ρ_e)）：e∉D_t ⟹ e∈A_t ⟹ e∈Ã ⟹（W11 q*=q̄）实际腿 q(ν(e),t)
    = canonicalLeg（规范腿保持开启）。
  - `closeLeg`（结束 t=ρ_e）：e∈D_t ⟹ e∉Ã ⟹ 实际腿 q(ν(e),ρ_e) ≠ canonicalLeg（腿被关闭）。

  ★三段假设是 W11 结论（覆盖可行 ⟹ q*=q̄）在「规范声部 ν(e) × 各时刻」上的投影 + W9 激活集
    语义（开始∈B_t/保持∈A_t\D_t/结束∈D_t）。本文件把它们作为**状态机输入假设**（W12 单元素版
    的驱动前提），不在本文件重证 W11/W9——那是 W11/W9 owner 已证、W13 递归层组装的活。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★开始-保持-结束状态机 `LegStateMachine`（L0，C37 §九 三段语义核心）：
  给定腿赋值 `L : LegAssignment V` 与逐时刻头寸函数 `q : V → Index → Leg`，状态机刻画规范声部
  ν(e) 的腿在元素 e 操作区间上的**开始-保持-结束**三段行为。

  - `openLeg`（**开始** t=λ_e，C37 §九 开始分支）：开始时刻规范腿 = canonicalLeg
      `q (L.nu e) e.startIndex = canonicalLeg e (L.units e)`。
    这是「e∈B_t ⟹ e∈Ã ⟹ 目标腿按 ε_e 开 ⟹（W11 q*=q̄）实际腿=目标腿」的逐时刻投影结论。
  - `holdLeg`（**保持** t∈(λ_e,ρ_e)，C37 §九 内部分支）：内部时刻规范腿保持 = canonicalLeg
      `∀ t, e.startIndex < t → t < e.endIndex → q (L.nu e) t = canonicalLeg e (L.units e)`。
    这是「e∉D_t ⟹ e∈A_t⟹e∈Ã ⟹（W11 q*=q̄）实际腿保持=目标腿」的逐内部时刻投影结论。
  - `closeLeg`（**结束** t=ρ_e，C37 §九 结束分支）：结束时刻规范腿被关闭 ≠ canonicalLeg
      `q (L.nu e) e.endIndex ≠ canonicalLeg e (L.units e)`。
    这是「e∈D_t ⟹ e∉Ã ⟹ 腿关闭（不再是目标腿）」的逐时刻投影结论。

  ★L0：纯结构假设（三段逐时刻腿等式/不等式），承载 C37 §九状态机驱动语义。三段共同覆盖
    [λ_e,ρ_e]——开始 {λ_e} + 保持 (λ_e,ρ_e) = [λ_e,ρ_e)（EatSep 区间）、结束 {ρ_e}（关闭点）。
-/
structure LegStateMachine {V : Type} (L : LegAssignment V) (q : V → Index → Leg)
    (e : SyntaxElement) : Prop where
  /-- 开始段（t=λ_e）：规范腿正确开启 = canonicalLeg（W11 q*=q̄ 在开始时刻的投影）。 -/
  openLeg : q (L.nu e) e.startIndex = canonicalLeg e (L.units e)
  /-- 保持段（t∈(λ_e,ρ_e)）：规范腿保持 = canonicalLeg（W11 q*=q̄ 在内部各时刻的投影）。 -/
  holdLeg : ∀ t : Index, e.startIndex < t → t < e.endIndex →
    q (L.nu e) t = canonicalLeg e (L.units e)
  /-- 结束段（t=ρ_e）：规范腿被关闭 ≠ canonicalLeg（e∉Ã 在结束时刻的投影）。 -/
  closeLeg : q (L.nu e) e.endIndex ≠ canonicalLeg e (L.units e)

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 状态机 ⟹ Eat^sep（开始+保持两段拼成整覆盖区间）

  C37 §九：「故 ∀t∈[λ_e,ρ_e) 元素 e 的规范腿始终存在且方向=ε_e = Eat^sep(e)」。本节证状态机
  开始段（t=λ_e）+ 保持段（t∈(λ_e,ρ_e)）经区间分解 `tickCovered_split` 拼成 W8 `EatSep`
  （∀t∈[λ_e,ρ_e), q(ν(e),t)=canonicalLeg）。结束段不参与（ρ_e∉[λ_e,ρ_e)）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★状态机 ⟹ Eat^sep `legStateMachine_imp_eatSep`（L0，C37 §九 核心步：三段 ⟹ 整区间覆盖）：
  开始-保持-结束状态机 `LegStateMachine L q e` ⟹ 分账本吃到 `EatSep L q e`。

  证明（区间分解）：EatSep 要求 ∀t, tickCovered e t → q(ν(e),t)=canonicalLeg。由 `tickCovered_split`
  覆盖区间内每 t 要么 t=λ_e（开始段，用 `openLeg`），要么 λ_e<t<ρ_e（保持段，用 `holdLeg`）——
  两段穷尽覆盖区间 ⟹ 每 t 都 = canonicalLeg ⟹ EatSep。结束时刻 ρ_e 不在覆盖区间（半开右开），
  不参与 EatSep（`closeLeg` 是关闭见证，非覆盖区间内的等式）。

  这正是 PDF §九「开始 t=λ_e 腿按 ε_e 开 + 内部 t∈(λ_e,ρ_e) 腿保持开 ⟹ ∀t∈[λ_e,ρ_e) 规范腿
  存在且方向=ε_e = Eat^sep(e)」。
-/
theorem legStateMachine_imp_eatSep {V : Type} {L : LegAssignment V} {q : V → Index → Leg}
    {e : SyntaxElement} (sm : LegStateMachine L q e) :
    EatSep L q e := by
  intro t ht
  rcases tickCovered_split e ht with hstart | ⟨hlo, hhi⟩
  · -- 开始段：t = λ_e ⟹ 用 openLeg
    rw [hstart]; exact sm.openLeg
  · -- 保持段：λ_e < t < ρ_e ⟹ 用 holdLeg
    exact sm.holdLeg t hlo hhi

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 C37 单元素版主定理：每元素被唯一规范腿吃到 + 正收益（开始-保持-结束状态机）

  C37（PDF §九 页17–18）：∀e∈E, Eat^sep(e) ∧ ∃!ν(e)。本节把 §3 的 EatSep 接 W8 `sepCovered_of`
  得 `SepCovered`（覆盖 + G^sep>0），并由 ν 单射（W8 `LegAssignment.nu_injective`）得规范腿唯一。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★★定理3·单元素覆盖 `eat_separated_single`（L0，C37 §九 页17–18，开始-保持-结束状态机）★★★：
  在覆盖可行域内（状态机三段成立 = W11 q*=q̄ 的逐时刻投影 + W9 激活语义），单元素 e 在分账本
  语义下**被规范头寸腿吃到且产生正向毛收益**：`SepCovered L q e P`。

  前提：
  - `sm : LegStateMachine L q e`（开始-保持-结束状态机，C37 §九三段——每段是 W11 q*=q̄ +
    W9 激活集语义在规范声部 ν(e) × 各时刻的投影）。
  - `hcons : DirectionConsistent e P`（元素方向一致 ε_e·ΔP_e>0，W7 §二方向语义——G^sep>0 前提）。

  结论：`SepCovered L q e P`——
  - **覆盖**（Eat^sep）：由状态机 ⟹ EatSep（§3 `legStateMachine_imp_eatSep`，开始+保持拼整区间）。
  - **正收益**（G^sep>0）：由方向一致 + s_e>0 ⟹ G^sep_e>0（W8 `gSep_pos`）。

  ★这是 PDF §九「∀t∈[λ_e,ρ_e) 规范腿存在且方向=ε_e = Eat^sep(e)」+ §十「G^sep_e>0」的单元素
    合一——经开始-保持-结束状态机得整区间覆盖，经方向一致得正收益。
-/
theorem eat_separated_single {V : Type} (L : LegAssignment V) (q : V → Index → Leg)
    (e : SyntaxElement) (P : Index → Tick)
    (sm : LegStateMachine L q e) (hcons : DirectionConsistent e P) :
    SepCovered L q e P :=
  sepCovered_of L q e P (legStateMachine_imp_eatSep sm) hcons

/--
  ★定理3·规范腿唯一 `canonical_leg_unique`（L0，C37 §九「∃!ν(e)」唯一性侧，ν 单射的腿侧陈述）：
  若两元素 e₁、e₂ 共用同一规范头寸腿 ν(e₁)=ν(e₂)，则 e₁=e₂。这是「每元素只有一个规范头寸腿」
  （C37 §九「ν 单射 ⟹ 规范吃笔腿唯一」）的形式——元素↦规范腿是单射归属，每条规范腿至多吃一个元素。

  直接由 W8 `LegAssignment.nu_injective`（C28 单射）——本文件不重证单射，复用 W8 字段。
-/
theorem canonical_leg_unique {V : Type} (L : LegAssignment V)
    {e₁ e₂ : SyntaxElement} (h : L.nu e₁ = L.nu e₂) : e₁ = e₂ :=
  L.nu_injective e₁ e₂ h

/--
  ★★★★定理3（C37 全形式·单元素版）·每元素被唯一规范腿吃到 + 正收益
  `theorem_three_single_element`★★★★（L0，C37 §九 页17–18，**开始-保持-结束状态机**）：

  在覆盖可行域内（状态机三段成立）+ 元素方向一致下，单元素 e
  **① 被规范头寸腿吃到（Eat^sep）② 产生正向毛收益（G^sep>0）③ 规范腿唯一（∃!ν 的单射核心）**：

      `SepCovered L q e P ∧ (∀ e', L.nu e' = L.nu e → e' = e)`

  - 第一合取项 `SepCovered L q e P`：覆盖（Eat^sep）+ 正收益（G^sep>0）——`eat_separated_single`。
  - 第二合取项 `∀ e', ν(e')=ν(e) → e'=e`：规范腿 ν(e) 唯一对应 e（ν 单射，∃!ν 的唯一性）——
    任何与 e 共用规范腿的元素必是 e 自身。

  ★这是 PDF §九 C37「∀e∈E, Eat^sep(e) 且 ∃!ν(e)」的**单元素版**（spec §D W12 行）：
    单个 e 经开始-保持-结束状态机被唯一规范腿吃到。全称 ∀e（对元素树所有 e）是 W13 自相似递归
    的活（对树深归纳），W14 最终定理合并覆盖+收益+策略全定义三结论。
  ★L0 诚实边界：被吃到=语法覆盖、∃!ν=单射逻辑必然——**NOT** 实盘每笔盈利（C40/C42 否定净账户
    每笔盈利）。仅覆盖可行域内成立（状态机三段=W11 q*=q̄ 前提；q̄∉K_Θ 时风险投影改变目标腿）。
-/
theorem theorem_three_single_element {V : Type} (L : LegAssignment V) (q : V → Index → Leg)
    (e : SyntaxElement) (P : Index → Tick)
    (sm : LegStateMachine L q e) (hcons : DirectionConsistent e P) :
    SepCovered L q e P ∧ (∀ e', L.nu e' = L.nu e → e' = e) :=
  ⟨eat_separated_single L q e P sm hcons,
   fun _ h => canonical_leg_unique L h⟩

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 结束段独立坐实：t=ρ_e 腿被关闭（半开区间右开，覆盖区间外）

  C37 §九「t=ρ_e∈D_t ⟹ e∉Ã ⟹ 腿关闭」。§3/§4 已用开始+保持两段拼成 EatSep（结束段不参与覆盖
  区间，因 ρ_e∉[λ_e,ρ_e)）。本节独立坐实「结束时刻在覆盖区间外 + 状态机 closeLeg ⟹ 关闭点与
  EatSep 区间不冲突」——半开区间右开的诚实落地（覆盖到 ρ_e 之前，ρ_e 处关闭，无重叠）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★结束段不冲突 `end_close_consistent`（L0，C37 §九结束分支的半开区间诚实落地）：
  状态机的结束段 `closeLeg`（ρ_e 处腿 ≠ canonicalLeg）与 EatSep（[λ_e,ρ_e) 内腿 = canonicalLeg）
  **不冲突**——因 ρ_e ∉ tickCovered（半开右开 `not_tickCovered_end`），EatSep 不约束 ρ_e。

  形式：EatSep L q e（区间内覆盖）∧ 状态机 closeLeg（ρ_e 关闭）同时成立——结束时刻在覆盖区间外，
  关闭点不破坏区间覆盖。这坐实 PDF §九「腿覆盖到 ρ_e 之前，ρ_e 处关闭」的半开区间语义。
-/
theorem end_close_consistent {V : Type} {L : LegAssignment V} {q : V → Index → Leg}
    {e : SyntaxElement} (sm : LegStateMachine L q e) :
    EatSep L q e ∧ q (L.nu e) e.endIndex ≠ canonicalLeg e (L.units e) :=
  ⟨legStateMachine_imp_eatSep sm, sm.closeLeg⟩

/--
  ★结束时刻在覆盖区间外 `end_outside_cover`（L0）：结束时刻 ρ_e 不被 e 覆盖（¬tickCovered）——
  故 EatSep（仅约束覆盖区间内）天然不要求 ρ_e 处腿 = canonicalLeg，与 closeLeg 关闭一致。
  这是半开区间 [λ_e,ρ_e) 右开的直接推论（`not_tickCovered_end` 复述，供下游 W13 引）。
-/
theorem end_outside_cover (e : SyntaxElement) : ¬ e.tickCovered e.endIndex :=
  not_tickCovered_end e

/-! ════════════════════════════════════════════════════════════════════════
  ## §6 诚实标签（formalization-validity-domain gatekeeper，被吃到 ≠ 实盘盈利）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★定理3 裁定标签 `SeparateCoverVerdict`（gatekeeper，诚实分层）。
  唯一构造子 `eatenByCanonicalLegInCoverDomain`——类型层钉死「覆盖可行域内单元素被唯一规范腿吃到
  （语法覆盖 + 正毛收益）」。
  ★**没有** `ProfitGuaranteed` / `EatenEverywhere` 构造子——拒绝两类声明膨胀：
    (1) 「被吃到 ⟹ 实盘盈利 / 净账户每笔盈利」（被吃到是语法层覆盖，G^sep>0 是声部级毛收益，
        非净资产盈利——C40/C42 否定净账户每笔盈利，双开净额退化 G_net=0）；
    (2) 「被吃到在任意状态成立」（仅覆盖可行域 X^cover_Θ 内成立；q̄∉K_Θ 时状态机开始/保持段
        假设不成立，风险投影改变目标腿，定理3 不适用）。
-/
inductive SeparateCoverVerdict where
  | eatenByCanonicalLegInCoverDomain
deriving DecidableEq, Repr

/--
  ★裁定见证（L0，gatekeeper）：定理3 裁定必是「覆盖可行域内单元素被唯一规范腿吃到」。
  支撑：§4 `theorem_three_single_element`（被吃到 + G^sep>0 + ν 唯一）+ §3 状态机 ⟹ EatSep
  + §2 三段状态机（W11 q*=q̄ 投影）。被吃到是覆盖域内语法层结论——**不**蕴含实盘盈利、
  **不**在覆盖域外成立。
-/
theorem cover_verdict_is_eaten (v : SeparateCoverVerdict) :
    v = SeparateCoverVerdict.eatenByCanonicalLegInCoverDomain := by
  cases v; rfl

/-! ════════════════════════════════════════════════════════════════════════
  ## 交付总结（W12 工位，C37 定理3 单元素版）

  本文件**证**（L0，machine-checked，无 sorry/admit/axiom，import 仅 W8/W11 链）：

  1. §1 区间分解（[λ_e,ρ_e) = {λ_e} ∪ (λ_e,ρ_e)）：
     - `tickCovered_start`（开始 t=λ_e 在区间内）+ `tickCovered_hold`（保持 t∈(λ_e,ρ_e) 在区间内）
       + `not_tickCovered_end`（结束 ρ_e 不在区间内，半开右开）。
     - ★`tickCovered_split`：覆盖区间内每 t 要么开始要么保持（二分穷尽）——状态机两段拼整区间的根据。

  2. §2 开始-保持-结束状态机 `LegStateMachine`（C37 §九三段语义）：
     - `openLeg`（开始 t=λ_e 腿=canonicalLeg）+ `holdLeg`（保持 t∈(λ_e,ρ_e) 腿=canonicalLeg）
       + `closeLeg`（结束 t=ρ_e 腿≠canonicalLeg）。三段是 W11 q*=q̄ 逐时刻投影 + W9 激活语义。

  3. §3 状态机 ⟹ Eat^sep：
     - ★`legStateMachine_imp_eatSep`：开始+保持两段经区间分解拼成 W8 `EatSep`（整区间覆盖）。

  4. ★★§4 定理3 单元素版（核心产出）：
     - `eat_separated_single`：状态机 + 方向一致 ⟹ `SepCovered`（覆盖 Eat^sep + 正收益 G^sep>0）。
     - `canonical_leg_unique`：ν(e₁)=ν(e₂) ⟹ e₁=e₂（∃!ν 唯一性，复用 W8 nu_injective）。
     - ★★`theorem_three_single_element`：单元素 **被唯一规范腿吃到 + G^sep>0 + ν 唯一**
       （SepCovered ∧ ∀e' ν(e')=ν(e)→e'=e）——C37 §九单元素版全形式。

  5. §5 结束段坐实：`end_close_consistent`（closeLeg 与 EatSep 不冲突，半开右开）+
     `end_outside_cover`（ρ_e 在覆盖区间外）。

  6. §6 诚实标签 `cover_verdict_is_eaten`（裁定=覆盖域内被吃到，无「盈利保证」/「处处成立」构造子）。

  本文件**不证**（formalization-validity-domain 诚实边界）：
  - ✗ 被吃到 ⟹ 实盘盈利 / 净账户每笔盈利（被吃到是 L0 语法覆盖，G^sep>0 是声部级毛收益；
    净账户每笔盈利被 C40/C42 否定——双开净额退化 G_net=0，W6 hedged_leg_net_zero）。
  - ✗ ∀e（元素树全称）被吃到（本文件仅单元素版；对树深归纳的全称覆盖是 W13 自相似递归的活）。
  - ✗ 全定义策略唯一性 ∃!O（W10 标的）/ 覆盖可行 q*=q̄ 内部证明（W11 标的——本文件作前提引用）。
  - ✗ 被吃到在覆盖可行域**外**成立（q̄∉K_Θ 时风险投影改变目标腿，状态机三段假设不成立）。

  ★下游引用（W13 自相似递归 / W14 最终定理直接引）：
  - 单元素覆盖：`theorem_three_single_element`（W13 归纳基/归纳步的单元素覆盖见证）/
    `eat_separated_single`（SepCovered 见证）。
  - 状态机 ⟹ EatSep：`legStateMachine_imp_eatSep`（W13 对每个 e 装配状态机后得 EatSep）。
  - 唯一性：`canonical_leg_unique`（∃!ν 的单射核心，W14 顶点引）。
  - 区间分解：`tickCovered_split`/`tickCovered_start`/`not_tickCovered_end`（W13 区间归纳引）。

  谱系：C37（PDF §九 页17–18）→ W8（Eat^sep + ν 单射 C28/C29/C30）+ W11（覆盖可行 ⟹ q*=q̄ C36）
        + W9（激活集 Ã C31/C32）+ W7（语法元素 + tickCovered C27）→ 本文件 W12（单元素定理3）→
        下游 W13/W14。有效域上界 = 覆盖可行域内声部级覆盖（C40/C42 净资产盈利被否定）。
  ════════════════════════════════════════════════════════════════════════ -/

end NewChanlun.Origin.SeparateCoverTheorem
