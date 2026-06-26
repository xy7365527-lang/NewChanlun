/-
  Strict/OpenTail.lean — 标准第5部分实例化：未完成走势 → 当下状态 + 未来延伸分支集
  （task #65, RTAS 蜂群 T-opentail 工位）

  上游标准：`tmp/chanlun-strict-classification-standard.md` 第一部分 B 第 5 部分
            （ChatGPT 最强调的"关键修正"）+ `Strict/Classification.lean` 的
            `OpenTailSystem` 结构骨架（结构 7）。

  标准第5部分严格陈述（standard:37）：
    "未完成走势不能强行唯一分类为**最终结果**，只能唯一分类为**当下状态** s；
     未来延伸 Ext(h)=⊔ⱼ Bⱼ(h) 互斥穷尽分支。"

  关键修正（standard:40-41）："未完成走势只分类为状态，不分类为结果。这是我们当前
    `classifyMove` 直接给最终 outcome（trend/consolidation）的根本性问题：对未完成走势
    这是越界的事后分类。"

  本文件做两件事（两层定理）：
  1. **实例化** `Strict.OpenTailSystem`：把缠论未完成走势 h 的当下状态 + 背驰未来分支集
     填入标准内核结构，**证明三项分支义务**（branch_total / branch_disjoint / branch_sound）。
     这把"声明了未完成→状态分支集"变成"实例化了带证明的 OpenTailSystem"。
  2. **当下状态唯一**：未完成走势的当下状态 `current(h)` 存在且唯一（状态层 ∃!，
     不是 outcome 层）——标准第5部分允许的唯一分类。

  ★越界修正的位置（codex#2 #2）：CandidateMove 越界的根上修复在 `RecursiveConstruction`
    （删除 `CandidateMove.outcome` + `candidate_preserves_totality`，替换为 `MoveState` /
    `CandidateMove.state` 状态层语义）。本文件是标准内核侧的 OpenTailSystem 实例化，
    与该根上修复同源（同一标准第5部分），但聚焦"内核结构的证明义务实例化"。

  ★认识论等级（formalization-validity-domain 强制标注）：本文件全部 **L0**
    （纯定义/结构层，不依赖经验数据）。lake build 通过 = 这些证明义务在本实例上
    机器可检验地成立（分支集是良定义的互斥穷尽分割），**不**声称"未来被唯一预测"——
    哪个分支实际发生由后续真实走势决定（L2/L3，不由本 L0 声称）。

  范式：纯 Prop/Type，不依赖 Mathlib（继承 Strict/Classification 的自包含约束）。
  禁 sorry/admit/axiom。

  谱系：598（真完全分类元判据）→ 603（递归 vs 轴范式）→ 615（Layer1 ⊊ Layer2）。
-/

import Strict.Classification

namespace Strict.OpenTail

open Strict

/-! ## 缠论未完成走势的承载对象 -/

/--
  ★方向（未完成走势的背驰原方向；反转=翻转、延续=保持相对此方向定义）。

  本文件自包含一个最小 `Dir`（不 import `Formal.*`，保持 Strict 库对 Formal 的独立——
  Strict 是标准内核侧，各工位实例化时再桥接到自己的 Formal 对象）。
-/
inductive Dir where
  | up
  | down
deriving DecidableEq, Repr

/-- 方向翻转（反转语义：上涨↔下跌）。 -/
def Dir.flip : Dir → Dir
  | Dir.up => Dir.down
  | Dir.down => Dir.up

/-- 翻转两次还原（对合性，方向只有两态）。 -/
theorem Dir.flip_flip (d : Dir) : d.flip.flip = d := by
  cases d <;> rfl

/-- 翻转改变方向（反转 ≠ 原方向，两态对合无不动点）。 -/
theorem Dir.flip_ne (d : Dir) : d.flip ≠ d := by
  cases d <;> simp [Dir.flip]

/--
  ★临时尾部倾向 `TailLean`（未完成走势的当下倾向，标准第5部分"当下状态 s"的内容）。

  三态对应已出现中枢读出的临时倾向（与 `Formal.MoveOutcome` 的非升级分支同构，
  但**本质是"当下状态"，不是"最终结果"**——它由 `pendingTail` 包成状态，见下）：
  - `leaningUp`：当下倾向上涨延续；
  - `leaningDown`：当下倾向下跌延续；
  - `leaningFlat`：当下倾向盘整（单中枢/未定）。
-/
inductive TailLean where
  | leaningUp
  | leaningDown
  | leaningFlat
deriving DecidableEq, Repr

/--
  ★未完成走势历史 `OpenHist`（标准第5部分的 h）。

  缠论未完成走势 = 已出现的中枢读数（临时尾部倾向）+ 背驰原方向。它是"历史 h"的
  最小承载：
  - `tail`：从已出现中枢读出的临时尾部倾向（盘整/上涨/下跌的当下倾向）——这是
    **当下状态的内容**，不是最终结果（标准第5部分：未完成只给状态）。
  - `origDir`：背驰发生时的原趋势方向（002 方向不抹——反转/延续相对此方向定义）。

  关键：`OpenHist` 不携带"最终 outcome"字段——类型层就拒绝"未完成走势有最终结果"。
-/
structure OpenHist where
  /-- 临时尾部倾向（当下状态内容，非最终结果）。 -/
  tail : TailLean
  /-- 背驰原趋势方向（002 方向保留）。 -/
  origDir : Dir

/-! ## 当下状态（State）：未完成走势只给状态，不给最终结果 -/

/--
  ★当下状态 `OpenState`（标准第5部分：未完成走势唯一分类为当下状态 s）。

  `OpenState` 只有一个构造子 `pendingTail tail`——**所有** `OpenHist` 都是未完成走势，
  其当下状态恒为 `pendingTail`（携带临时尾部倾向）。**没有** `completed` 构造子——
  类型层就保证 `OpenHist`（未完成走势）**不可能**被赋予最终结果地位（越界根上不可表达）。

  这与 `RecursiveConstruction.MoveState` 互补：那里 `MoveState` 含 `completed`/`pending`
  二态（因为 CandidateMove 含 settled 可完成可未完成）；这里 `OpenHist` 专指**未完成**
  走势（standard:37 "Ext(h) 只对未完成 h 有意义"），故状态恒为 pending 侧。
-/
inductive OpenState where
  | pendingTail (tail : TailLean)
deriving DecidableEq, Repr

/--
  ★当下状态读出 `current : OpenHist → OpenState`（标准第5部分 OpenTailSystem.current）。

  未完成走势 h 的当下状态 = `pendingTail h.tail`——把已出现中枢的临时尾部倾向包成
  **状态**（不是最终结果）。这是全函数，对每个未完成走势唯一给出当下状态。
-/
def current (h : OpenHist) : OpenState :=
  OpenState.pendingTail h.tail

/-! ## 未来延伸分支集：Ext(h) = ⊔ⱼ Bⱼ(h) 互斥穷尽 -/

/--
  ★未来延伸分支标签 `Branch`（标准第5部分 B₁/B₂，背驰 candidate 的两个未来）。

  - `reversal`（B₁，背驰成立）：趋势反转——结果方向 = 原方向**翻转**（产生第一类 BSP，
    002 背驰-买卖点"背驰⟹买卖点⟹转折"侧）。
  - `continuation`（B₂，背驰破坏）：趋势延续——结果方向 = 原方向**保持**（背驰被新力度
    否定，beichi 第24课延续侧）。

  无第三分支：未完成背驰走势的未来在缠论里恰二分（成立→反转 ⊔ 破坏→延续）。
-/
inductive Branch where
  | reversal       -- B₁：背驰成立 → 反转（方向翻转）
  | continuation   -- B₂：背驰破坏 → 延续（方向保持）
deriving DecidableEq, Repr

/--
  ★未来延伸 `Ext h h'`（标准第5部分 OpenTailSystem.Ext）：`h'` 是 `h` 的一个真未来延伸。

  缠论里 h' 是 h 的未来延伸 ⟺ h' 继承 h 的背驰原方向（同一段未完成走势的延续/反转，
  不是另起一段）。`Ext h h' := h'.origDir = h.origDir`——延伸保持原方向身份
  （反转/延续是相对原方向的，故未来延伸必继承原方向作为参照系）。

  ★这是 Ext 的**h 依赖**形式（codex 修正：Ext 是历史依赖的未来集合，不是全域二分）：
  只有继承 h.origDir 的 h' 才属于 Ext(h)。
-/
def Ext (h h' : OpenHist) : Prop :=
  h'.origDir = h.origDir

/--
  ★延伸结果方向 `extResultDir`（标准第5部分 + 002/beichi：连接方向语义）。

  把未来延伸 h' 相对 h 的结果方向读出（由 h' 的临时尾部倾向相对 h.origDir 判定）：
  - h' 尾部顺 h.origDir（创新极值倾向）⟹ 延续（方向 = h.origDir）；
  - h' 尾部逆 h.origDir / 转平 ⟹ 反转（方向 = flip h.origDir）。

  这把分支从标签落到**实际方向**——`branchPred` 的方向语义来源。
-/
def extLeansContinuation (h h' : OpenHist) : Bool :=
  match h.origDir, h'.tail with
  | Dir.up,   TailLean.leaningUp   => true   -- 顺上涨延续
  | Dir.down, TailLean.leaningDown => true   -- 顺下跌延续
  | _, _ => false                            -- 逆向 / 转平 ⟹ 反转倾向

/--
  ★分支判定 `branchOf`（标准第5部分：把延伸 h' 分到 Bⱼ(h)）。

  - h' 顺 h.origDir 延续（`extLeansContinuation = true`）⟹ `continuation`（B₂）。
  - h' 逆向/转平（`extLeansContinuation = false`）⟹ `reversal`（B₁）。

  确定性分支函数——不预测哪个发生，而是把每个**已发生的**未来延伸归入唯一分支
  （因果无前视：分支由 h' 本身的可观测尾部倾向决定）。
-/
def branchOf (h h' : OpenHist) : Branch :=
  if extLeansContinuation h h' then Branch.continuation else Branch.reversal

/--
  ★分支谓词 `branchPred`（标准第5部分 OpenTailSystem.BranchPred）：`h' ∈ Bⱼ(h)`。

  `branchPred h b h' ⟺ (h' 是 h 的未来延伸) ∧ (h' 落入分支 b)`。
  绑定 `Ext h h'`（h 依赖）——只有真未来延伸才谈分支归属（关死全域二分冒充）。
-/
def branchPred (h : OpenHist) (b : Branch) (h' : OpenHist) : Prop :=
  Ext h h' ∧ branchOf h h' = b

/-! ## 三项分支证明义务（branch_total / branch_disjoint / branch_sound） -/

/--
  ★分支穷尽 `branch_total`（标准第5部分，L0）：每个未来延伸至少落入一个分支。

  对任意真未来延伸 h'（`Ext h h'`），存在分支 b 使 `branchPred h b h'`——
  即 `Ext(h) = ⊔ⱼ Bⱼ(h)` 的 ⊔ 覆盖全集侧（`branchOf` 全函数 ⟹ 至少一个分支）。
-/
theorem branch_total (h h' : OpenHist) (hext : Ext h h') :
    ∃ b, branchPred h b h' :=
  ⟨branchOf h h', hext, rfl⟩

/--
  ★分支互斥 `branch_disjoint`（标准第5部分，L0）：一个延伸不落入两个不同分支。

  若 h' 同时落入 b₁ 与 b₂，则 b₁ = b₂——即 `Ext(h) = ⊔ⱼ Bⱼ(h)` 的 ⊔ 不重叠侧
  （`branchOf` 是函数 ⟹ 至多一个分支值）。
-/
theorem branch_disjoint (h h' : OpenHist) (b₁ b₂ : Branch)
    (h1 : branchPred h b₁ h') (h2 : branchPred h b₂ h') : b₁ = b₂ := by
  have e1 : branchOf h h' = b₁ := h1.2
  have e2 : branchOf h h' = b₂ := h2.2
  rw [← e1, ← e2]

/--
  ★分支健全 `branch_sound`（标准第5部分，L0）：落入某分支的必是真未来延伸。

  `branchPred h b h' ⟹ Ext h h'`——分支归属蕴含真延伸关系（`branchPred` 内含 `Ext`）。
  这保证分支集不会把"非 h 的未来"误纳入（与 total/disjoint 一起构成互斥穷尽分割）。
-/
theorem branch_sound (h : OpenHist) (b : Branch) (h' : OpenHist)
    (hb : branchPred h b h') : Ext h h' :=
  hb.1

/-! ## 第一层定理：实例化标准内核 OpenTailSystem -/

/--
  ★缠论未完成走势的 OpenTailSystem 实例（标准第5部分内核实例化，L0）。

  把缠论未完成走势 h 的当下状态（`current`）+ 未来延伸（`Ext`）+ 背驰分支集
  （`branchPred`）填入 `Strict.OpenTailSystem`，并提供三项分支证明义务的证明。

  这是**第一层定理**：声明了"未完成→状态分支集"被升级为"实例化了带证明义务的
  OpenTailSystem 结构"——证明义务（branch_total/disjoint/sound）全部机器可检验地满足。
-/
def chanlunOpenTail : OpenTailSystem OpenHist OpenState Branch where
  current := current
  Ext := Ext
  BranchPred := branchPred
  branch_total := branch_total
  branch_disjoint := branch_disjoint
  branch_sound := branch_sound

/-! ## 第二层定理：当下状态唯一 + 方向语义落地 -/

/--
  ★当下状态唯一（标准第5部分，L0，第二层定理）：未完成走势唯一分类为当下状态。

  `current h` 是 h 的唯一当下状态——`current` 是全函数，对每个未完成走势给出唯一状态。
  **这是标准第5部分允许的唯一分类（状态层 ∃!，不是 outcome 层）**：唯一的是"当下状态"，
  未完成走势**没有**唯一最终结果（最终结果取决于落入哪个未来分支，由后续走势决定）。
-/
theorem current_unique (h : OpenHist) :
    ∃ s, current h = s ∧ ∀ s', current h = s' → s' = s :=
  ⟨current h, rfl, fun _ heq => heq.symm⟩

/--
  ★当下状态恒为 pending（标准第5部分，L0）：未完成走势的状态在类型上无法是最终结果。

  `current h` 必是 `pendingTail _`——`OpenState` 没有 `completed` 构造子，故未完成走势
  在类型层**不可能**取得"最终结果"地位。这是越界（把未完成压成最终 outcome）的
  类型层不可表达性见证。
-/
theorem current_always_pending (h : OpenHist) :
    ∃ t, current h = OpenState.pendingTail t :=
  ⟨h.tail, rfl⟩

/--
  ★分支结果方向（标准第5部分 + 002/beichi，L0）：反转翻方向 / 延续保方向。

  把分支 b 映射为相对 h.origDir 的结果方向：
  - `reversal` ⟹ `h.origDir.flip`（方向翻转，002 转折）；
  - `continuation` ⟹ `h.origDir`（方向保持，beichi 延续）。
-/
def branchResultDir (h : OpenHist) (b : Branch) : Dir :=
  match b with
  | Branch.reversal => h.origDir.flip
  | Branch.continuation => h.origDir

/--
  ★反转分支翻转方向（标准第5部分 + 002，L0）：落 B₁ 的延伸结果方向 = 原方向翻转。
-/
theorem reversal_flips_dir (h h' : OpenHist) (hb : branchPred h Branch.reversal h') :
    branchResultDir h (branchOf h h') = h.origDir.flip := by
  rw [hb.2]; rfl

/--
  ★延续分支保持方向（标准第5部分 + beichi，L0）：落 B₂ 的延伸结果方向 = 原方向保持。
-/
theorem continuation_keeps_dir (h h' : OpenHist) (hb : branchPred h Branch.continuation h') :
    branchResultDir h (branchOf h h') = h.origDir := by
  rw [hb.2]; rfl

/--
  ★两分支结果方向必不同（标准第5部分 + 002，L0）：反转方向 ≠ 延续方向。

  `branchResultDir h reversal = h.origDir.flip ≠ h.origDir = branchResultDir h continuation`。
  从方向层面坐实 B₁/B₂ 是真正不同的两个未来（不是同方向换标签）——分支分割连接真实
  缠论方向语义，关死"标签分割不连接语义"。
-/
theorem branch_result_dirs_differ (h : OpenHist) :
    branchResultDir h Branch.reversal ≠ branchResultDir h Branch.continuation := by
  simp only [branchResultDir]
  exact h.origDir.flip_ne

/--
  ★分支二分穷尽（标准第5部分，L0）：分支标签恰两个，无第三分支。

  任意分支 b 必是 `reversal` 或 `continuation`——这是 `Branch` 的构造子穷尽
  （结构归纳）。背驰未完成走势的未来恰二分，无第三种。
-/
theorem branch_dichotomy (b : Branch) :
    b = Branch.reversal ∨ b = Branch.continuation := by
  cases b
  · exact Or.inl rfl
  · exact Or.inr rfl

end Strict.OpenTail
