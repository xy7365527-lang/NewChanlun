/-
  Eval 健全性 ⟦Dₙ(h)⟧=h + 递归核显式化（最严格完全分类标准 第2部分 + 第4部分，L0）
  ★cc-reccore 工位（标准清单 C6/C8/C9 的基座；603/615 谱系）

  编排者最严标准（tmp/chanlun-strict-classification-standard.md）的两个缺口：

  ── 第2部分「唯一递归分解」核心缺口 ──
  标准要求：Dₙ:Xₙ→Tₙ，证 **Eval(Dₙ(h)) = h**（健全：分解能重构原对象）。
  当前代码库 **完全没有** "分解能重构原对象" 的定理——RecursiveConstruction 只有
  witness soundness（CentersDerivedFrom）/ termination soundness（generatedStep_strictly_
  decreases），**没有** round-trip 重构健全。本文件补上。

  ── 第4部分「递归核」缺口 ──
  标准要求 𝒰ₙ₊₁ = Can_Φₙ(∪_{m≥3} 𝒰ₙᵐ) 三要素显式化：
  ① m≥3（高级别结构至少 3 个低级子走势，缠论"至少三笔/三段"）；
  ② Can 规范化唯一边界（同一 base 经规范化得唯一提升，与 Spiral.gaugeFix 呼应）；
  ③ f₁≠f₂（起始步 segment-base vs 逐级步 compose 是不同函数，避免中枢↔走势循环定义）。

  ★★严格性铁律（formalization-validity-domain + no-patch-mentality）：
  Eval 健全 **不能** 退化为重言。`Move` 本身就是分解树（inductive），若定义 Eval:Move→Move
  并证 Eval(D x)=x 的恒等，那是零信息同义反复（codex 审计要抓的）。本文件的非平凡性来自：
  - `leaves : Move → List Move` 把层级分解树 **剥离层级**，遍历回底层 segment 时间序
    （level_recursion.md：Move[0]=Segment，走势最终由线段序列构成）。
  - `Eval := leaves` **丢失** 层级结构；`decompose` **添加** 层级结构（把扁平 segment
    序列 + 分组方案打包成 Move 树）。round-trip `leaves(decompose xs scheme) = xs`
    要求添加的层级被遍历精确剥离还原——这是真定理，**不是** 恒等重言（输入 xs:List Move
    扁平，中间 decompose xs scheme:Move 带层级，输出 leaves(...):List Move 扁平且=xs）。

  认识论等级：所有定理 **L0**（定义内蕴，对 Move/List 结构归纳，零数据依赖）。
  Lean 通过 = 逻辑/管线正确（L0），**不是** 实证有效域，不得膨胀
  （formalization-validity-domain：Eval 健全是 L0 结构定理，断言的是"分解树无损保存
  底层序列"这一代数事实，不断言"任何真实走势的分解唯一"——后者是 L2/L3 经验问题）。

  ★codex 异质审计修正（gpt-5.5 high，2026-06-25）——两大目标 PASS，诚实化 5 处声明膨胀：
  - Eval 健全 round-trip：PASS（leaves 真剥离层级、groupBySpecs 真添加层级、hflat 做实事
    去掉则定理为假、leaves_strictly_flattens 见证非重言）。补 `wellformed_leaves_strictly_
    flattens`（良构域全称见证）+ `wellformed_leaves_ge_three`（一般良构叶数≥3，无 hflat）。
  - GroupSpec.count：删去 docstring 的"≥1"声明（结构无此约束，count=0 退化组不破坏保叶）。
  - kernel_leaves_ge_three：诚实标"全 segment 一层限定"，一般版另立 wellformed_leaves_ge_three。
  - canonicalize_unique：诚实标"输出单值骨架"，实质（下界+幂等）由其他两定理承载。
  - RecursiveKernelStep：docstring 从"完整 contract"降级"三要素骨架联合"，列出未编码语义。
  ★第二轮复审（同 session）3 处一致性同步：§2 分节说明"每组≥1段"改"任意 count"；
  canonicalize **定义级** docstring 标明"同 base/Φₙ/纤维"是元层意图非 Lean 所证；
  f1_ne_f2 / kernel_step_product_ne_start docstring 标明"避免循环定义"是元层解释，定理
  本身只证构造子级不相交。复审结论：5 处指定修正全 PASS，无退化重言，L0 标注恰当。
-/

import Formal.TrendTrichotomy
import Formal.CenterTrichotomy
import Formal.RecursiveConstruction

namespace Formal.EvalSoundness

open Formal.TrendTrichotomy (Direction)
open Formal.CenterTrichotomy (Center)
open Formal.RecursiveConstruction
  (Move classifyMove WellFormed CentersDerivedFrom)

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 Eval = 叶遍历（剥离层级，还原底层 segment 时间序）

  `leaves m` = 走势分解树 `m` 的全部 segment 基底，按时间（左→右）序。
  这是覆盖映射 p:E→B 的 **base 投影**：把级别提升（Move 树）打回底层走势序列（叶）。
  非平凡性：leaves 对 segment 是单点 [m]，对 compose 是子树叶的拼接——层级被抹平。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★叶遍历（Eval 的核心）：分解树 → 底层 segment 序列（时间序）。
  - `segment`：叶就是它自己（递归底，level_recursion.md：Move[0]=Segment）。
  - `compose subs _ _`：叶 = 各子走势叶的顺序拼接（flatMap）——层级结构被剥离，
    只留下底层 segment 的时间序。这是覆盖映射打回 base 的精确实现。
-/
def leaves : Move → List Move
  | Move.segment d lo hi => [Move.segment d lo hi]
  | Move.compose subs _ _ => subs.flatMap leaves

/-- segment 的叶序列是单点（它自己）。 -/
theorem leaves_segment (d : Direction) (lo hi : Int) :
    leaves (Move.segment d lo hi) = [Move.segment d lo hi] := by
  simp [leaves]

/-- compose 的叶序列 = 子走势叶的顺序拼接（层级剥离）。 -/
theorem leaves_compose (subs : List Move) (centers : List Center) (lvl : Nat) :
    leaves (Move.compose subs centers lvl) = subs.flatMap leaves := by
  simp [leaves]

/--
  ★Move 的强归纳原理（nested inductive 的列表归纳）：证明 `motive m` 时，
  compose 情形可使用 **每个** 子走势的归纳假设（`∀ s ∈ subs, motive s`）。
  这弥补 Lean 标准 `induction` 不支持 nested inductive（subs : List Move）的限制——
  用 `Move.rec` 的双 motive（motive_2 := ∀ s ∈ ·, motive s）手工构造。
-/
theorem Move.strongRec {motive : Move → Prop}
    (hseg : ∀ d lo hi, motive (Move.segment d lo hi))
    (hcomp : ∀ subs centers level, (∀ s ∈ subs, motive s) →
      motive (Move.compose subs centers level))
    (m : Move) : motive m :=
  Move.rec
    (motive_1 := motive)
    (motive_2 := fun subs => ∀ s ∈ subs, motive s)
    hseg
    (fun subs centers level ih => hcomp subs centers level ih)
    (fun _ h => absurd h (List.not_mem_nil))
    (fun head tail ihh iht s hs => by
      rcases List.mem_cons.mp hs with h | h
      · exact h ▸ ihh
      · exact iht s h)
    m

/--
  ★叶全是 segment（L0，结构归纳）：leaves 的输出确实是底层 segment 序列，
  不含任何 compose——遍历彻底剥离了所有层级，到达递归底。
  这保证 leaves 是"打回 base 空间"的忠实投影（输出在 segment 子集内）。
-/
theorem leaves_all_segments (m : Move) :
    ∀ x ∈ leaves m, ∃ d lo hi, x = Move.segment d lo hi := by
  induction m using Move.strongRec with
  | hseg d lo hi =>
      intro x hx
      rw [leaves_segment] at hx
      rw [List.mem_singleton] at hx
      exact ⟨d, lo, hi, hx⟩
  | hcomp subs centers lvl ih =>
      intro x hx
      rw [leaves_compose] at hx
      rw [List.mem_flatMap] at hx
      obtain ⟨sub, hsub_mem, hx_in⟩ := hx
      exact ih sub hsub_mem x hx_in

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 decompose = 分组分解（添加层级），Eval 健全 round-trip ⟦D(h)⟧=h

  decompose 是 Eval 的逆方向（标准 Dₙ）：给一个扁平 segment 序列 + 一个分组方案
  （每组消费任意 `count` 个 segment，组的中枢/级别由方案给出；"≥3 段"的良构下界
  另由 `WellFormed` 给出，不是 GroupSpec 的字段约束），打包成一棵 Move 树。
  健全性 `leaves (decompose xs scheme) = xs`：分组添加的层级被叶遍历精确剥离，
  原扁平序列无损还原——分解可重构原对象（标准第2部分核心命题）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  一个分组方案的单组：把扁平序列的一段连续 segment 封装为一个 compose 子走势。
  - `count`：本组消费多少个连续底层 segment（`Nat`，**无下界约束**——count=0 给出退化组
    `compose [] ...`，其叶为空，不破坏 round-trip 保叶）。
  - `centers`：本组封装走势的中枢序列（compose 字段）。
  - `level`：本组封装走势的级别（compose 字段）。
  这是 Dₙ 的"如何分组"参数——不同分组方案对应同一 base 的不同提升（T₅₂ 多义性）。

  ★诚实声明（codex 审计修正）：`count` **不携带** ≥1 约束。Eval 健全（保叶 round-trip）
  对任意 count（含 0）成立——这是有意的：round-trip 健全性是 count 无关的代数事实，
  不依赖"每组 ≥1 段"。若需要"良构走势"的 ≥3 子走势约束，那是 `WellFormed`（见 §4
  kernel_arity_ge_three）的职责，不是 GroupSpec 的字段约束。
-/
structure GroupSpec where
  count : Nat
  centers : List Center
  level : Nat

/--
  按一个分组方案把扁平 segment 序列分解为子走势列表（Dₙ 的分组步）。
  每个 spec 从剩余序列头部取 `count` 个 segment 封装为一个 compose 子走势，
  余下的 segment（无对应 spec）按其本身（segment 或既有 Move）原样保留在尾部。
  这是覆盖映射的提升（lift）：底层序列 → 带层级的子走势列表。
-/
def groupBySpecs : List GroupSpec → List Move → List Move
  | [], rest => rest
  | spec :: specs, ms =>
      let chunk := ms.take spec.count
      let remaining := ms.drop spec.count
      Move.compose chunk spec.centers spec.level :: groupBySpecs specs remaining

/--
  ★分组保叶（L0，核心引理）：把扁平序列按任意分组方案封装后，叶序列不变。
  `(groupBySpecs specs ms).flatMap leaves = ms.flatMap leaves`。
  对 specs 结构归纳：每组封装一个 chunk（take）+ 余下（drop），封装产生的 compose
  的叶 = chunk.flatMap leaves（leaves_compose），与 drop 部分拼接 = take++drop 的
  flatMap = 原 ms 的 flatMap（List.take_append_drop）。层级添加被叶遍历抵消。
-/
theorem groupBySpecs_preserves_leaves (specs : List GroupSpec) (ms : List Move) :
    (groupBySpecs specs ms).flatMap leaves = ms.flatMap leaves := by
  induction specs generalizing ms with
  | nil => rfl
  | cons spec specs ih =>
      simp only [groupBySpecs, List.flatMap_cons]
      rw [leaves_compose]
      rw [ih (ms.drop spec.count)]
      rw [← List.flatMap_append]
      rw [List.take_append_drop]

/--
  ★Eval 健全·分解 round-trip（L0，标准第2部分核心定理）：⟦D(xs)⟧ = ⟦xs⟧。

  对 **任意** 底层 segment 序列 xs（每个元素是 segment）和 **任意** 分组方案 specs，
  把 xs 按 specs 分解（添加层级）再叶遍历（剥离层级），还原 xs 的叶序列。
  因 xs 全是 segment（前提 hflat），其叶 = xs 自身（leaves_segment 逐元），故
  `leaves` 还原 xs 本身——分解 **能重构原对象**。

  ★非重言见证：输入 xs:List Move（扁平 segment）→ decompose 产生带层级的子走势列表
  → leaves 剥离层级 → = xs。中间对象 groupBySpecs specs xs 含 compose（层级），
  输出与输入相等说明层级被无损添加再无损剥离，不是恒等映射的平凡相等。
-/
theorem eval_sound_roundtrip (specs : List GroupSpec) (xs : List Move)
    (hflat : ∀ x ∈ xs, ∃ d lo hi, x = Move.segment d lo hi) :
    (groupBySpecs specs xs).flatMap leaves = xs := by
  rw [groupBySpecs_preserves_leaves]
  -- 扁平序列的 flatMap leaves = 自身（每个 segment 叶是单点自己）。
  induction xs with
  | nil => rfl
  | cons x rest ih =>
      obtain ⟨d, lo, hi, hx⟩ := hflat x (List.mem_cons_self)
      subst hx
      rw [List.flatMap_cons, leaves_segment]
      rw [ih (fun y hy => hflat y (List.mem_cons_of_mem _ hy))]
      rfl

/--
  ★扁平序列的叶 = 自身（L0，eval_sound_roundtrip 的退化基底 specs=[]）：
  空分组方案下，decompose 不添加任何层级，叶遍历直接还原扁平序列。
  这是 round-trip 在"无分组"边界的实例——leaves∘decompose 在底层是恒等。
-/
theorem leaves_flatten_flat (xs : List Move)
    (hflat : ∀ x ∈ xs, ∃ d lo hi, x = Move.segment d lo hi) :
    xs.flatMap leaves = xs := by
  have := eval_sound_roundtrip [] xs hflat
  simpa [groupBySpecs] using this

/--
  ★单走势 Eval 健全（L0）：一个 compose 走势的叶 = 其子走势叶的还原。
  把 round-trip 锚定到单棵 Move 树：`leaves (compose subs c l) = subs.flatMap leaves`，
  当 subs 全是 segment 时即 = subs——单走势分解（compose 一层）可被 leaves 重构。
-/
theorem single_move_eval_sound (subs : List Move) (centers : List Center) (lvl : Nat)
    (hflat : ∀ x ∈ subs, ∃ d lo hi, x = Move.segment d lo hi) :
    leaves (Move.compose subs centers lvl) = subs := by
  rw [leaves_compose]
  exact leaves_flatten_flat subs hflat

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 Eval 非平凡性见证：leaves 真的剥离层级（不是恒等重言）

  codex 审计的核心质疑：⟦D(h)⟧=h 是不是退化为重言？以下定理证明 leaves **真的**
  改变结构——存在 Move m 使 leaves m ≠ [m]（即 leaves 不是单点恒等），且 leaves 的
  长度 = 底层 segment 数（与 Move 树的节点数不同）。这关死"重言"指控。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★leaves 严格剥离层级·语法层见证（L0）：存在走势其叶序列 ≠ 单点自身。
  取一个 compose 三段 segment 的走势 m，leaves m 是 3 个 segment（长度 3），
  而 [m] 长度 1——leaves m ≠ [m]。这证明 leaves **不是** 恒等映射 m↦[m]，
  Eval 健全 round-trip 不是 `id` 的平凡相等（formalization-validity-domain：
  非重言，有真实的层级剥离信息增量）。

  ★诚实声明（codex 审计修正）：此见证用 `centers=[]` 的 **raw**（非 WellFormed）compose，
  只见证 **语法层** 非重言（leaves 在 Move 语法上非恒等）。良构域（WellFormed）上的
  非平凡见证见下方 `wellformed_leaves_strictly_flattens`（全称、更强）。
-/
theorem leaves_strictly_flattens :
    ∃ m : Move, leaves m ≠ [m] := by
  refine ⟨Move.compose
      [Move.segment Direction.up 0 1,
       Move.segment Direction.down 1 0,
       Move.segment Direction.up 0 2] [] 1, ?_⟩
  -- leaves m 长度为 3，[m] 长度为 1，长度不等故列表不等。
  intro h
  have hlen := congrArg List.length h
  simp [leaves_compose, List.flatMap, leaves_segment] at hlen

/--
  ★每个走势的叶序列非空（L0）：任意 Move 的 leaves 至少 1 个 segment。
  segment 叶是单点（长度 1）；compose 叶是子走势叶拼接——由 WellFormed 的 subs 非空
  （subs.length ≥ 3）传导非空。对 Move 强归纳。这是"覆盖映射打回 base 不丢底"的基础。
-/
theorem leaves_nonempty (m : Move) (h : WellFormed m) : leaves m ≠ [] := by
  induction m using Move.strongRec with
  | hseg d lo hi => rw [leaves_segment]; exact List.cons_ne_nil _ _
  | hcomp subs centers lvl ih =>
      rw [leaves_compose]
      unfold WellFormed at h
      obtain ⟨harity, _, _, _, _, hsubwf⟩ := h
      -- subs 非空（length ≥ 3），头部子走势叶非空 ⟹ flatMap 非空。
      cases subs with
      | nil => simp at harity
      | cons hd tl =>
          rw [List.flatMap_cons]
          have hhd : leaves hd ≠ [] :=
            ih hd (List.mem_cons_self) (hsubwf hd (List.mem_cons_self))
          exact List.append_ne_nil_of_left_ne_nil hhd _

/--
  ★leaves 长度 ≥ 子走势数（L0）：良构 compose 走势的叶数 ≥ subs.length。
  每个子走势的叶非空（leaves_nonempty）⟹ 至少贡献 1 个叶 ⟹ flatMap 长度 ≥ 子走势数。
  对 subs 归纳。这把"叶剥离"定量下界——叶数不少于子走势数（层级剥离不丢分支）。
-/
theorem leaves_length_ge_subs_length (subs : List Move) (centers : List Center) (lvl : Nat)
    (hsubwf : ∀ m ∈ subs, WellFormed m) :
    (leaves (Move.compose subs centers lvl)).length ≥ subs.length := by
  rw [leaves_compose]
  induction subs with
  | nil => simp
  | cons hd tl ih =>
      rw [List.flatMap_cons, List.length_append, List.length_cons]
      have hhd : leaves hd ≠ [] := leaves_nonempty hd (hsubwf hd (List.mem_cons_self))
      have hhd_pos : (leaves hd).length ≥ 1 := List.length_pos_iff.mpr hhd
      have htl : (tl.flatMap leaves).length ≥ tl.length :=
        ih (fun m hm => hsubwf m (List.mem_cons_of_mem hd hm))
      omega

/--
  ★leaves 严格剥离层级·良构域见证（L0，全称，比语法层见证更强）：
  **任意** 良构 compose 走势 m 都满足 leaves m ≠ [m]——不只是某个构造的反例。
  良构 ⟹ subs.length ≥ 3（WellFormed.1）⟹ leaves 长度 ≥ subs 数 ≥ 3 > 1 = [m].length。
  这关死 codex 审计 CONCERN（语法层见证不覆盖良构域）：在缠论真正关心的良构走势域上，
  leaves 也 **必然** 非恒等——Eval 健全的非平凡性在有效走势域上全称成立。
-/
theorem wellformed_leaves_strictly_flattens (subs : List Move) (centers : List Center) (lvl : Nat)
    (h : WellFormed (Move.compose subs centers lvl)) :
    leaves (Move.compose subs centers lvl) ≠ [Move.compose subs centers lvl] := by
  intro heq
  have hlen := congrArg List.length heq
  rw [List.length_cons, List.length_nil] at hlen
  have harity : subs.length ≥ 3 := by unfold WellFormed at h; exact h.1
  have hsubwf : ∀ m ∈ subs, WellFormed m := by
    unfold WellFormed at h; exact h.2.2.2.2.2
  have hge : (leaves (Move.compose subs centers lvl)).length ≥ subs.length :=
    leaves_length_ge_subs_length subs centers lvl hsubwf
  omega

/--
  ★叶数 = compose 走势的底层 segment 计数（L0）：leaves 长度记录底层 segment 数，
  与 Move 树深度/节点数无关——这是"剥离到 base 空间"的定量见证。对全 segment 子走势，
  叶数 = 子走势数。层级越深，leaves 长度仍只数最底层 segment。
-/
theorem leaves_length_eq_segment_count (subs : List Move) (centers : List Center) (lvl : Nat)
    (hflat : ∀ x ∈ subs, ∃ d lo hi, x = Move.segment d lo hi) :
    (leaves (Move.compose subs centers lvl)).length = subs.length := by
  rw [single_move_eval_sound subs centers lvl hflat]

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 递归核显式化（标准第4部分）：𝒰ₙ₊₁ = Can_Φₙ(∪_{m≥3} 𝒰ₙᵐ)

  三要素：① m≥3；② Can 规范化唯一边界；③ f₁≠f₂（segment-base vs compose）。
  ════════════════════════════════════════════════════════════════════════ -/

/-! ── 要素① m≥3：高级别结构至少 3 个低级子走势（缠论"至少三笔/三段"） ── -/

/--
  ★递归核 m≥3（L0，标准第4部分要素①）：良构 compose 走势的子走势数 ≥ 3。
  这是 𝒰ₙ₊₁ = Can(∪_{m≥3} 𝒰ₙᵐ) 中 m≥3 的形式化——高级别走势由 **至少 3 个**
  低级子走势构成（走势分解定理二，zoushi.md 第17课）。从 WellFormed 提取。
-/
theorem kernel_arity_ge_three (subs : List Move) (centers : List Center) (lvl : Nat)
    (h : WellFormed (Move.compose subs centers lvl)) :
    subs.length ≥ 3 := by
  unfold WellFormed at h
  exact h.1

/--
  ★递归核 m≥3 在底层 segment 上的传导（L0）：良构走势的叶数 ≥ 3。

  ★诚实声明（codex 审计修正）：本定理 **限定** `subs` 全是 segment（前提 hflat）——
  即"一层封装、子走势是底层线段"的情形，此时叶数恰 = subs 数 ≥ 3。它 **不是** 一般
  良构走势（子走势可为更深 compose）的完整叶下界。一般良构走势的叶数 ≥3 由更弱但全称的
  `leaves_length_ge_subs_length`（叶数 ≥ subs 数）配合 `kernel_arity_ge_three`（subs ≥3）
  给出（见 wellformed_leaves_strictly_flattens 的证明），无需 hflat。本定理是其全 segment
  退化版的精确等式形态，保留作"递归核生成基"层（𝒰₀ segment 直接组装 𝒰₁）的定量见证。
-/
theorem kernel_leaves_ge_three (subs : List Move) (centers : List Center) (lvl : Nat)
    (h : WellFormed (Move.compose subs centers lvl))
    (hflat : ∀ x ∈ subs, ∃ d lo hi, x = Move.segment d lo hi) :
    (leaves (Move.compose subs centers lvl)).length ≥ 3 := by
  rw [leaves_length_eq_segment_count subs centers lvl hflat]
  exact kernel_arity_ge_three subs centers lvl h

/--
  ★递归核 m≥3 的底层叶数下界·一般良构走势（L0，无 hflat，全称完整版）：
  **任意** 良构 compose 走势（子走势可为任意深度的 compose，非限定 segment）的叶数 ≥ 3。
  由 leaves_length_ge_subs_length（叶数 ≥ 子走势数）+ kernel_arity_ge_three（子走势数 ≥3）
  组合——叶数 ≥ subs.length ≥ 3。这补全 kernel_leaves_ge_three 的"全 segment 限定"，
  给出 m≥3 在底层 segment 上的 **一般** 传导：每个良构走势至少由 3 个底层 segment 支撑。
-/
theorem wellformed_leaves_ge_three (subs : List Move) (centers : List Center) (lvl : Nat)
    (h : WellFormed (Move.compose subs centers lvl)) :
    (leaves (Move.compose subs centers lvl)).length ≥ 3 := by
  have harity : subs.length ≥ 3 := kernel_arity_ge_three subs centers lvl h
  have hsubwf : ∀ m ∈ subs, WellFormed m := by
    unfold WellFormed at h; exact h.2.2.2.2.2
  have hge : (leaves (Move.compose subs centers lvl)).length ≥ subs.length :=
    leaves_length_ge_subs_length subs centers lvl hsubwf
  omega

/-! ── 要素② Can 规范化唯一边界：同一 base 经规范化得唯一提升（呼应 Spiral.gaugeFix） ── -/

/--
  ★规范化（Can）：从一组候选分组方案中按确定规则选唯一边界。
  规范化按"级别（`specLevel`）最小优先、同级别按方案在列表中的位置最早优先"选唯一。

  ★诚实声明（codex 审计修正）：本定义 **只** 实现"按 level 最小确定性选择"这一计算。
  它 **不编码** "同一 base"、Φₙ、候选合法性、纤维等价类——把 candidates 解释为"同一
  base 的多提升"、把结果解释为"纤维归约的单一截面"是 **元层意图**（与 Spiral.gaugeFix
  的 foldl minLift 同构的设计目标），不是本定义在 Lean 中证明的内容。其可证的实质性质
  是 `canonicalize_result_is_min`（结果是候选 level 下界）+ `canonicalize_idempotent_on_
  result`（幂等不漂移）。
-/
def specLevel (s : GroupSpec) : Nat := s.level

def minSpec (a b : GroupSpec) : GroupSpec :=
  if specLevel a ≤ specLevel b then a else b

def canonicalize : List GroupSpec → Option GroupSpec
  | [] => none
  | s :: ss => some (ss.foldl minSpec s)

/--
  ★Can 规范化单值性（L0，标准第4部分要素② 的输出单值骨架）：canonicalize 返回 Option，
  至多一个——多候选提升经规范化输出唯一边界（none 或 some 单值，绝不多值）。

  ★诚实声明（codex 审计修正）：本定理只断言 **输出单值性**（Option 函数的低信息事实），
  **不** 编码"同一 base / Φₙ / 候选合法性 / 规范结果属某有效纤维"——那些是更强的语义约束，
  本文件未形式化。Can 唯一边界的 **实质** 内容（规范结果是候选 level 下界 + 幂等不漂移）
  由 `canonicalize_result_is_min` 与 `canonicalize_idempotent_on_result` 承载（非本定理）。
  与 Spiral T52_gauge_fixes_unique 同构（覆盖空间纤维 → 唯一规范截面的 Option 单值骨架）。
-/
theorem canonicalize_unique (cands : List GroupSpec) :
    canonicalize cands = none ∨ ∃ s : GroupSpec, canonicalize cands = some s := by
  cases h : canonicalize cands with
  | none => exact Or.inl rfl
  | some s => exact Or.inr ⟨s, rfl⟩

/--
  ★Can 空候选 ⟹ 无规范（L0，偏函数诚实）：无候选提升 ⟹ canonicalize = none。
  无任何分组方案则规范无定义——诚实表达 Can 偏函数性（不伪造全函数边界）。
-/
theorem canonicalize_empty_none : canonicalize [] = none := rfl

/--
  ★Can 单候选 ⟹ 即是规范（L0）：唯一候选时规范化输出它自己（边界平凡确定）。
  这是规范唯一性的退化实例——单提升 base 的规范 = 该提升本身。
-/
theorem canonicalize_singleton (s : GroupSpec) : canonicalize [s] = some s := rfl

/--
  ★foldl minSpec 结果 level ≤ 累加器 level（L0，min 单调引理）：
  从任意累加器 acc 出发，foldl minSpec 只会让 level 不增——结果是 acc 与列表元素中
  level 最小者。这是 Can 规范化"选 level 最小提升"的核心不变量（对 List 结构归纳）。
-/
theorem foldl_minSpec_le_acc (ss : List GroupSpec) (acc : GroupSpec) :
    specLevel (ss.foldl minSpec acc) ≤ specLevel acc := by
  induction ss generalizing acc with
  | nil => simp
  | cons x xs ih =>
      simp only [List.foldl_cons]
      have hstep : specLevel (minSpec acc x) ≤ specLevel acc := by
        unfold minSpec
        by_cases hc : specLevel acc ≤ specLevel x
        · simp [hc]
        · simp [hc]; omega
      exact Nat.le_trans (ih (minSpec acc x)) hstep

/--
  ★Can 规范结果是候选 level 下界（L0）：规范化结果 s 的 level ≤ 候选集中任意候选 level。
  这是"规范化唯一边界 = 选 level 最小提升"的精确刻画——s 是整个候选集（含头部）的
  level 最小者。由 foldl_minSpec_le_acc 应用导出。
-/
theorem canonicalize_result_is_min (cands : List GroupSpec) (s : GroupSpec)
    (h : canonicalize cands = some s) :
    ∀ c ∈ cands, specLevel s ≤ specLevel c := by
  -- 通用引理：从任意累加器出发的 foldl 结果 level ≤ 累加器及列表中每个元素 level。
  have key : ∀ (l : List GroupSpec) (acc : GroupSpec) (c : GroupSpec),
      c ∈ l → specLevel (l.foldl minSpec acc) ≤ specLevel c := by
    intro l
    induction l with
    | nil => intro acc c hcl; exact absurd hcl (List.not_mem_nil)
    | cons y ys ihl =>
        intro acc c hcl
        rcases List.mem_cons.mp hcl with hcy | hcys
        · subst hcy
          simp only [List.foldl_cons]
          have hle : specLevel (minSpec acc c) ≤ specLevel c := by
            unfold minSpec
            by_cases hcc : specLevel acc ≤ specLevel c
            · simp only [hcc, if_true]
            · simp [hcc]
          exact Nat.le_trans (foldl_minSpec_le_acc ys (minSpec acc c)) hle
        · simp only [List.foldl_cons]
          exact ihl (minSpec acc y) c hcys
  cases cands with
  | nil => simp [canonicalize] at h
  | cons hd tl =>
      simp only [canonicalize, Option.some.injEq] at h
      intro c hc
      rcases List.mem_cons.mp hc with hc | hc
      · -- c = hd（头部）：结果 = foldl tl hd ≤ hd（min 单调）。
        rw [hc, ← h]
        exact foldl_minSpec_le_acc tl hd
      · -- c ∈ tl（尾部）：用通用引理。
        rw [← h]
        exact key tl hd c hc

/--
  ★Can 规范化幂等于规范结果（L0，标准第4部分要素②不动点）：
  把已规范化得到的结果 s 重新放回候选集头部，规范化仍得 s——规范边界是 **不动点**。
  非平凡性：这 **真的用到** `h`（s 是 cands 的 level 最小者），不是单点退化重言。
  混入已规范的提升不产生新规范 ⟹ 规范化幂等 Can(s::cands)=Can(cands)=some s，
  关死"重复规范化漂移"，是"唯一边界"的稳定性见证（与 Spiral gaugeFix 唯一截面同构）。
-/
theorem canonicalize_idempotent_on_result (cands : List GroupSpec) (s : GroupSpec)
    (h : canonicalize cands = some s) :
    canonicalize (s :: cands) = some s := by
  -- Can(s :: cands) = some (foldl minSpec s cands)；需证 foldl minSpec s cands = s。
  simp only [canonicalize, Option.some.injEq]
  -- s 是 cands 全体的 level 下界（canonicalize_result_is_min），故每步 minSpec 都保留 s。
  have hmin := canonicalize_result_is_min cands s h
  -- foldl minSpec s cands = s：对 cands 归纳，每步 minSpec s c = s（因 specLevel s ≤ specLevel c）。
  clear h
  induction cands with
  | nil => rfl
  | cons c cs ih =>
      simp only [List.foldl_cons]
      have hsc : minSpec s c = s := by
        unfold minSpec
        have : specLevel s ≤ specLevel c := hmin c (List.mem_cons_self)
        simp [this]
      rw [hsc]
      exact ih (fun c' hc' => hmin c' (List.mem_cons_of_mem c hc'))

/-! ── 要素③ f₁≠f₂：起始步 segment-base vs 逐级步 compose 是不同函数 ── -/

/--
  起始步 f₁（𝒰₀ 的生成）：原始构件 = segment（线段基底，level_recursion.md）。
  f₁ 是"造一个底层 segment 走势"——不消费任何子走势，从原始端点构造。
-/
def f1_startStep (d : Direction) (lo hi : Int) : Move := Move.segment d lo hi

/--
  逐级步 f₂（𝒰ₙ₊₁ 的生成）：compose = 把 ≥3 个低级子走势封装为高级走势。
  f₂ 是"用已有子走势造上级走势"——消费子走势列表，添加级别与中枢。
-/
def f2_levelStep (subs : List Move) (centers : List Center) (lvl : Nat) : Move :=
  Move.compose subs centers lvl

/--
  ★f₁≠f₂·值构造子不相交（L0，标准第4部分要素③）：起始步产物永远不等于逐级步产物。
  f₁ 产生 segment 构造子，f₂ 产生 compose 构造子——两个构造子是 Move 初代数的
  不同分支，**永不相等**（inductive 构造子单射 + 不交）。这形式化"f₁≠f₂"：起始步
  与逐级步是不同函数。

  ★诚实声明（codex 审计修正）：本定理证的是 **构造子级** 分离（segment ≠ compose），
  **不是** 完整的依赖图无环性证明。"避免中枢↔走势循环定义"是这个分离的 **元层** 解释
  （segment 不依赖中枢、compose 依赖子走势的中枢，二者类型级不可混淆），Lean 定理本身
  只断言两构造子值不相等。
-/
theorem f1_ne_f2 (d : Direction) (lo hi : Int)
    (subs : List Move) (centers : List Center) (lvl : Nat) :
    f1_startStep d lo hi ≠ f2_levelStep subs centers lvl := by
  unfold f1_startStep f2_levelStep
  intro h
  exact Move.noConfusion h

/--
  ★f₁ 不消费子走势·叶为自身（L0）：起始步产物的叶 = 单点（它自己）。
  f₁（segment）是递归底，不含任何低级子走势——其叶序列长度恒为 1。这与 f₂ 的叶
  （子走势叶拼接，可 ≥3）形成定量区分：起始步在叶计数上与逐级步不同。
-/
theorem f1_leaves_singleton (d : Direction) (lo hi : Int) :
    leaves (f1_startStep d lo hi) = [Move.segment d lo hi] := by
  unfold f1_startStep
  rw [leaves_segment]

/--
  ★f₂ 良构产物叶 ≥3（L0）：逐级步良构产物的叶数 ≥ 3（继承 m≥3）。
  f₂ 在良构 + 全 segment 子走势下叶数 ≥3，而 f₁ 叶数恒 = 1——叶计数 1 vs ≥3 给出
  f₁≠f₂ 的 **定量** 见证（不只是构造子不交的定性见证）。起始步与逐级步在生成的
  底层结构规模上本质不同。
-/
theorem f2_leaves_ge_three (subs : List Move) (centers : List Center) (lvl : Nat)
    (h : WellFormed (f2_levelStep subs centers lvl))
    (hflat : ∀ x ∈ subs, ∃ d lo hi, x = Move.segment d lo hi) :
    (leaves (f2_levelStep subs centers lvl)).length ≥ 3 :=
  kernel_leaves_ge_three subs centers lvl h hflat

/--
  ★f₁/f₂ 叶计数定量分离（L0，f₁≠f₂ 的最强见证，**一般良构走势**，无 hflat）：
  起始步叶数 = 1，逐级步 **任意** 良构产物叶数 ≥ 3（wellformed_leaves_ge_three，
  子走势可为任意深度 compose），故 1 < 叶数 ⟹ 同一对象不可能既是 f₁ 产物又是良构
  f₂ 产物。这是"避免中枢↔走势循环定义"的 **结构** 实质：若 f₁=f₂ 则 segment 走势
  同时是良构 compose 走势（叶数 1=≥3 矛盾）。

  ★诚实声明（codex 审计修正）：本定理证的是 **构造子级 + 叶计数级** 的分离（f₁ 产
  segment、f₂ 产良构 compose，叶数 1 vs ≥3），**不是** 完整的"依赖图无环性"证明——
  "避免循环定义"是这个分离的元层解释，Lean 定理本身证明的是 f₁ 与良构 f₂ 的产物在
  叶计数上不可重合（故两步不可混为同一函数）。
-/
theorem f1_f2_leaf_count_separation (d : Direction) (lo hi : Int)
    (subs : List Move) (centers : List Center) (lvl : Nat)
    (h : WellFormed (f2_levelStep subs centers lvl)) :
    (leaves (f1_startStep d lo hi)).length < (leaves (f2_levelStep subs centers lvl)).length := by
  rw [f1_leaves_singleton]
  have h3 : (leaves (f2_levelStep subs centers lvl)).length ≥ 3 :=
    wellformed_leaves_ge_three subs centers lvl h
  simp only [List.length_cons, List.length_nil]
  omega

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 递归核三要素的骨架联合：𝒰ₙ₊₁ = Can_Φₙ(∪_{m≥3} 𝒰ₙᵐ) 的三要素打包

  把 ①m≥3 + ②Can唯一 + ③f₁≠f₂ 打包为递归核生成步的 **三要素骨架**：
  一个递归核生成步 = 取 m≥3 个子走势（要素①）+ 用 f₂ 封装（要素③，区别于 f₁）
  + 经 Can 规范化得唯一规范（要素②）。

  ★诚实声明（codex 审计修正）：这是三要素的 **骨架联合**，**不是** 完整忠实 contract。
  本结构 **未编码**：product 的 WellFormed（仅 arity≥3，不含 level 递减/centers 派生/
  子走势递归良构）、cands 与 product/base 的"同 base"绑定、候选合法性、canonSpec 与
  product 的关联。这些更强语义是 WellFormed（RecursiveConstruction）+ Spiral 覆盖空间
  纤维 + SemanticQuotient（其他工位）的职责。本结构只保证三要素 **各自** 成立且
  各自的关键性质（f₁≠f₂ 分离、Can 幂等）可从字段导出。
-/
structure RecursiveKernelStep where
  subs : List Move
  centers : List Center
  lvl : Nat
  cands : List GroupSpec
  canonSpec : GroupSpec
  /-- 要素①：子走势数 ≥3（m≥3，走势分解定理二）。 -/
  arity_ge_three : subs.length ≥ 3
  /-- 要素②：候选集经 Can 规范化得唯一 canonSpec。 -/
  canon_unique : canonicalize cands = some canonSpec

/--
  ★递归核生成步产物用逐级步 f₂（L0）：生成步封装出的走势是 compose（f₂），非 segment（f₁）。
  这把三要素锚定：生成步的产物形态固定为 f₂_levelStep，与 f₁ 起始步类型级分离
  （f1_ne_f2），且满足 m≥3（arity_ge_three）、Can 唯一（canon_unique）。
-/
def RecursiveKernelStep.product (k : RecursiveKernelStep) : Move :=
  f2_levelStep k.subs k.centers k.lvl

/--
  ★递归核生成步产物 ≠ 任意起始步（L0，三要素联合定理）：
  任何递归核逐级生成步的产物都不等于任何起始步 segment——递归核的"逐级"分支与
  "起始"分支在每一步都保持构造子级分离（f₁≠f₂ 在生成步上落地）。这是 𝒰ₙ₊₁（逐级）
  与 𝒰₀（起始）不混淆的形式保证（构造子级，非完整依赖图无环性——同 f1_ne_f2 的诚实
  caveat：循环定义无环是元层解释，定理证的是 product 的 compose 构造子 ≠ segment 构造子）。
-/
theorem kernel_step_product_ne_start (k : RecursiveKernelStep)
    (d : Direction) (lo hi : Int) :
    k.product ≠ f1_startStep d lo hi := by
  unfold RecursiveKernelStep.product
  intro h
  exact f1_ne_f2 d lo hi k.subs k.centers k.lvl h.symm

/--
  ★递归核生成步满足 m≥3（L0，三要素联合）：生成步产物的子走势数 ≥ 3。
  从结构字段 arity_ge_three 直接读出——每个递归核步都遵守"至少三段"。
-/
theorem kernel_step_arity (k : RecursiveKernelStep) : k.subs.length ≥ 3 :=
  k.arity_ge_three

/--
  ★递归核生成步的 Can 规范是不动点（L0，三要素联合）：把生成步的规范结果重新混入
  其候选集头部，再规范化仍得同一 canonSpec——规范边界稳定，不因重复施加而漂移。
  从结构字段 canon_unique 经 canonicalize_idempotent_on_result 导出（真正用到"规范结果
  是候选 level 下界"，非单点退化）。这是每个递归核步的"唯一边界"稳定性见证。
-/
theorem kernel_step_canon_idempotent (k : RecursiveKernelStep) :
    canonicalize (k.canonSpec :: k.cands) = some k.canonSpec :=
  canonicalize_idempotent_on_result k.cands k.canonSpec k.canon_unique

end Formal.EvalSoundness
