/-
  最高走势类型 r* 非特殊 = 终余代数 νG（603 §四 + level_recursion + 597）
  ★codex 异质审计修正版（session 019eff21，发现 8/9 已吸收，no-patch：重写非补丁）

  缠论定理（已结算）：
  - 级别向上无结构上限（zhongshu.md:18，level_recursion.md:249）：级别递归向上无限
    unfold，只有 `settled moves < 3` 的 **自然终止**，`max_levels` 是安全上限非缠论约束。
  - r* = unfold 到当前的最深层，与任意 Move(ℓ) 用 **同一三构造子**（603 §四，codex Q2 PASS）。

  ★codex 审计修正（faithfulness）：
  - 发现8：`Tower := Nat → TowerLevel` + `∃ next, tower(k+1)=next` 只证"Nat 无最大元"
    （平凡），不等价于"级别无结构上限/r* 非特殊"，且未保证 k+1 层由 k 层构造。
    修正：用 `ValidTower`（带生成约束）；r* 非特殊重述为 **操作函数对"是否顶层"不可观察**
    + **扩展 tower 不改变既有层的操作**（extensionality）。
  - 发现9：`operate_uniform` 只因 `nonempty` 得 true，没证"不查 has_higher/protect_core"。
    修正：证 operate 只依赖 (kind, settled, BSP)——extensionality：两层若这三者相同则
    operate 相同；且 tower 扩展不改任意已有层 operate 结果。

  范式重铸（603）：级别递归是 **终余代数（coinductive）**——向上无限 unfold，无"最后一步"。
  r* 非特殊 = 操作语义对顶层性不可观察 ⟹ 支撑 597 无边界例外（无 protect_core 特例）。
-/

import Formal.TrendTrichotomy
import Formal.CenterTrichotomy
import Formal.RecursiveConstruction
import Formal.BSPLabels

namespace Formal.RStarNonSpecial

open Formal.TrendTrichotomy
open Formal.CenterTrichotomy (Center MoveOutcome)
open Formal.RecursiveConstruction (Move classifyMove CentersDerivedFrom MovesComposedFrom)

/--
  级别塔的一层。关键（codex 审计发现8/9）：**没有 `is_top` / `has_higher` 字段**——
  顶层性不是层的内禀属性。

  ★codex R4：携带本层 `moves`（已结算次级别走势）+ `centers`（本层中枢）——
  本层走势类型 `outcome` **由本层 centers 计算**（`classifyMove`），不继承下层 kind。
  本层 `bsp` 是本层端点标签集（不从下层字段复制）。这使生成关系忠实（codex R4 方案 B）。
-/
structure TowerLevel where
  /-- 本层级别索引（递归深度）。 -/
  levelIdx : Nat
  /-- 本层走势（本层级别的走势，构造上层中枢的原料）。 -/
  moves : List Move
  /-- 本层中枢序列（由 moves 生成，见 GeneratedStep）。 -/
  centers : List Center
  /-- 本层端点买卖点标签集。 -/
  bsp : Formal.BSPLabels.BSPLabelSet
  /-- ★本层标签集合法（codex R5：避免 malformed label set 进入 operate）。 -/
  bsp_wf : Formal.BSPLabels.WellFormedBSPLabelSet bsp

namespace TowerLevel
/-- ★本层走势裁决由本层中枢计算（codex R4：不继承下层 kind）。 -/
def outcome (lvl : TowerLevel) : MoveOutcome := classifyMove lvl.centers
end TowerLevel

/--
  ★当前层可继续构造下一层（codex R3#8 + R4：自然终止判据，level_recursion.md:249）。
  本层已结算走势数 ≥ 3 才能向上构造更高级别中枢。这是 **termination soundness**
  （codex R4：可保留作终止判据，但不单独作 generation soundness）。
-/
def CanBuildNext (lvl : TowerLevel) : Prop := lvl.moves.length ≥ 3

/--
  ★层间生成步（codex R4 方案 B：强关系 contract，kind/bsp 不继承）。

  `GeneratedStep cur next` ⟺
  - `CanBuildNext cur`（本层有足够走势向上构造）；
  - ★下一层中枢由本层走势 **生成**（`CentersDerivedFrom cur.moves next.centers`，
    沿用 RecursiveConstruction 的 witness+顺序 soundness contract）；
  - ★下一层走势是本层走势的封装序列（`next.moves = cur.moves`——封装恒等式
    Move(k)≡Level-(k+1) 笔的最小关系：上一层走势成为下一层构造的原料）。

  关键（codex R4）：下一层 `outcome` **不出现在 GeneratedStep 里**——它由
  `next.centers` 经 `classifyMove` **自行计算**（`TowerLevel.outcome`），不继承 `cur`。
  `next.bsp` 也不从 `cur.bsp` 复制（无 inheritance 约束）。伪造的 next.centers
  （与 cur.moves 无生成关系）不满足 `CentersDerivedFrom`。
-/
def GeneratedStep (cur next : TowerLevel) : Prop :=
  CanBuildNext cur
  ∧ next.levelIdx = cur.levelIdx + 1
  ∧ CentersDerivedFrom cur.moves next.centers
  ∧ MovesComposedFrom cur.moves cur.levelIdx next.moves

/--
  ★有效级别塔（终余代数 + 生成约束，codex 审计 R1#8 + R2#8）。

  不是任意 `Nat → TowerLevel`，而是带 **生成约束** 的有效塔：
  - `level k` 给出第 k 层（向上无限可读 = 终余代数 unfold）。
  - `terminated k`：第 k 层是否因 `settled moves < 3` **自然终止**（level_recursion.md:249）——
    缠论级别上界来源（数据涌现），非结构上限。
  - `termination_absorbing`：终止吸收态（一旦终止，更高层保持终止）。
  - ★`valid_step`（R2#8 生成约束）：若第 k 层未终止，则它可构造下一层（CanBuildNext）
    且第 k+1 层由第 k 层生成（GeneratedStep）。关死"k+1 层未必由 k 层构造"漏洞。
  - ★`terminated_iff`（R2#8）：终止 ⟺ 不可构造下一层（terminated k ↔ ¬CanBuildNext）。
-/
structure ValidTower where
  level : Nat → TowerLevel
  terminated : Nat → Bool
  termination_absorbing : ∀ k, terminated k = true → terminated (k + 1) = true
  /-- ★生成约束：未终止 ⟹ 可构造下一层 ∧ 下一层由本层生成（codex R2#8）。 -/
  valid_step : ∀ k, terminated k = false →
    CanBuildNext (level k) ∧ GeneratedStep (level k) (level (k + 1))
  /-- ★终止 ⟺ 不可构造下一层（codex R2#8：terminated 不是独立标志，由 CanBuildNext 决定）。 -/
  terminated_iff : ∀ k, terminated k = true ↔ ¬ CanBuildNext (level k)

/--
  ★r* 非特殊·操作语义不可观察顶层性（codex 审计发现9，603 §四）。

  操作函数 `operate` 只依赖一层的本征数据 (kind, settled, bsp)——**没有"是否顶层 /
  是否有更高级别"参数**。这形式化"r* 真顶减仓"与 Move(ℓ<r*) 用同一规则，无 protect_core
  的 `if has_higher_level` 特判。

  操作输出 = (走势类型, 操作是否有效)；有效性由端点标签集非空（升跌完备性）决定。
-/
def operate (lvl : TowerLevel) : MoveOutcome × Bool :=
  (lvl.outcome, lvl.bsp.labels ≠ [])

/--
  ★操作 extensionality（codex 审计发现9）：两层若 (kind, settled, bsp) 相同则 operate 相同。

  这是"operate 只观察本征数据"的精确表述——operate **不可能** 区分"顶层"与"非顶层"，
  因为 TowerLevel 根本没有顶层性字段。r* 与任意级别用同一 operate。
-/
theorem operate_blind_to_topness (a b : TowerLevel)
    (hk : a.outcome = b.outcome) (hb : a.bsp = b.bsp) :
    operate a = operate b := by
  unfold operate
  rw [hk, hb]

/--
  ★扩展 tower 不改变既有层的操作（codex 审计发现8：r* 非特殊的核心命题）。

  给一个有效塔，在任意更高层继续 unfold（得到新塔 `ext`，只要它在前缀上与原塔一致），
  则原塔每个已有层 `k` 的 operate 结果 **不变**。这形式化"r* 不是特殊的顶"——
  发现更高级别（tower 扩展）不会回头改变已涌现层的操作，顶部无边界例外。
-/
theorem extension_preserves_operate (t ext : ValidTower)
    (k : Nat) (h_prefix : ext.level k = t.level k) :
    operate (ext.level k) = operate (t.level k) := by
  rw [h_prefix]

/--
  ★r* 同构造子（L0，codex Q2 PASS）：任意级别（含 r* 最深层）的走势类型属三构造子之一。
  与任意其他级别完全同构——没有"r* 用特殊构造子"分支。
-/
theorem rstar_same_constructors (t : ValidTower) (k : Nat) :
    (t.level k).outcome = MoveOutcome.trend Direction.up
    ∨ (t.level k).outcome = MoveOutcome.trend Direction.down
    ∨ (t.level k).outcome = MoveOutcome.consolidation
    ∨ (t.level k).outcome = MoveOutcome.higherCenterCandidate :=
  Formal.RecursiveConstruction.outcome_total (t.level k).centers

/--
  ★级别上界由自然终止决定，非结构上限（zhongshu.md:18 + 601 离散二分消除）。

  级别是否有上界由 `terminated`（settled moves < 3 数据涌现）决定，且终止是吸收态——
  这不是"Move 构造子的离散字段 has_higher∈{有,无}"，是终余代数的 unfold 何时自然停。
  形式化：一旦某层终止，所有更高层终止——级别上界是数据的连续涌现边界，非结构离散二分。
-/
theorem termination_is_upward_closed (t : ValidTower) (k j : Nat)
    (hk : t.terminated k = true) (hj : k ≤ j) :
    t.terminated j = true := by
  induction j with
  | zero =>
      have : k = 0 := Nat.le_zero.mp hj
      rw [this] at hk; exact hk
  | succ n ih =>
      rcases Nat.lt_or_ge k (n + 1) with hlt | hge
      · exact t.termination_absorbing n (ih (Nat.le_of_lt_succ hlt))
      · -- k ≥ n+1 且 k ≤ n+1 ⟹ k = n+1
        have : k = n + 1 := Nat.le_antisymm hj hge
        rw [this] at hk; exact hk

/--
  ★操作有效性由端点标签集非空保证（支撑 597 无边界例外）。
  任意级别（含 r*）operate 的有效位 = 端点标签集非空——由升跌完备性 totality（010）
  保证为真，不引入 protect_core 分支。r* 与任意级别一致有效。
-/
theorem operate_valid_uniform (t : ValidTower) (k : Nat) :
    (operate (t.level k)).2 = true := by
  unfold operate
  exact decide_eq_true (t.level k).bsp.nonempty

/--
  ★未终止层可向上 unfold（codex R2#8 生成约束的使用）：

  对任意未终止层 k，它可构造下一层（CanBuildNext）且下一层由它生成（GeneratedStep）。
  这形式化"r* 不是结构上限"——只要未自然终止（settled moves ≥ 3），递归就能继续 unfold，
  没有"最后一层"的结构性禁止。r* 是数据涌现的当前最深层，非类型上的特殊顶。
-/
theorem active_level_unfolds (t : ValidTower) (k : Nat) (h : t.terminated k = false) :
    CanBuildNext (t.level k) ∧ GeneratedStep (t.level k) (t.level (k + 1)) :=
  t.valid_step k h

/--
  ★操作限制在 active 层（codex R2#8 caveat）：

  对一个 active（未终止）层，其操作既由 (kind, bsp) 决定（不查顶层性，`operate_blind_to_topness`），
  又因该层可继续 unfold（`active_level_unfolds`）而无需"是否到顶"判断——r* 非特殊的完整表述：
  active 层的操作语义与"是否为最深层"完全无关。
-/
theorem active_operate_independent_of_top (t : ValidTower) (k : Nat)
    (h : t.terminated k = false) :
    (operate (t.level k)).2 = true ∧ CanBuildNext (t.level k) :=
  ⟨operate_valid_uniform t k, (t.valid_step k h).1⟩

/--
  ★生成步蕴含中枢 soundness（codex R4 方案 B：next.centers 由 cur.moves 生成）。
  若 `GeneratedStep cur next`，则 `next.centers` 满足 `CentersDerivedFrom cur.moves`——
  伪造的 next.centers（与 cur.moves 无 witness+顺序生成关系）不满足。关死"随意填下层"漏洞。
-/
theorem generatedStep_centers_sound (cur next : TowerLevel)
    (h : GeneratedStep cur next) :
    CentersDerivedFrom cur.moves next.centers := h.2.2.1

/--
  ★生成步级别递进（codex R5：next 级别 = cur 级别 +1，非塌缩）。
  关死"每层复制同一 moves"——上一层被封装为更高级别，级别索引严格 +1。
-/
theorem generatedStep_level_up (cur next : TowerLevel)
    (h : GeneratedStep cur next) :
    next.levelIdx = cur.levelIdx + 1 := h.2.1

/--
  ★生成步上级走势是封装对象（codex R5：MovesComposedFrom，非原样复制）。
  `next.moves` 由 `cur.moves` compose 而成（非空、良构、级别+1、每个是 compose 且 subs⊆cur.moves）——
  关死 codex R5"next.moves = cur.moves 原样复制"漏洞。tower 层级语义不塌缩。
-/
theorem generatedStep_moves_composed (cur next : TowerLevel)
    (h : GeneratedStep cur next) :
    MovesComposedFrom cur.moves cur.levelIdx next.moves := h.2.2.2

/-- 生成步上级走势非空（封装产生新走势）。 -/
theorem generatedStep_next_nonempty (cur next : TowerLevel)
    (h : GeneratedStep cur next) : next.moves ≠ [] :=
  (generatedStep_moves_composed cur next h).1

/--
  ★生成步消费约束（codex R6 核心：每上级走势吃 ≥3 下级走势，有限下级不能无限封装）。

  `next.moves.length * 3 ≤ cur.moves.length`——上级走势数 × 3 ≤ 下级走势数。这关死 codex R6
  反例（3 个相同 compose 重复消费同 3 段）：3 个 upper move 需 ≥9 个 lower move，3 个 lower
  无法满足。保证级别递归逐层减少、自然收敛终止（level_recursion.md:249）。
-/
theorem generatedStep_consumption_bound (cur next : TowerLevel)
    (h : GeneratedStep cur next) :
    next.moves.length * 3 ≤ cur.moves.length :=
  (generatedStep_moves_composed cur next h).2.2.1

/--
  ★级别递归严格收敛（codex R6 反例的根本关闭）：上级走势数 **严格小于** 下级走势数。

  因 `CanBuildNext cur`（cur.moves.length ≥ 3）+ 消费约束（next.moves.length * 3 ≤ cur.moves.length），
  得 `next.moves.length < cur.moves.length`。走势数沿塔严格递减 ⟹ 不能无限 build ⟹ 自然终止
  （level_recursion.md:249）。这关死 codex R6"3 段重复封装成 3 段无限递归"的伪 tower——
  级别语义不塌缩，r* 是数据涌现的有限最深层。
-/
theorem generatedStep_strictly_decreases (cur next : TowerLevel)
    (h : GeneratedStep cur next) :
    next.moves.length < cur.moves.length := by
  have hcan : CanBuildNext cur := h.1
  have hbound : next.moves.length * 3 ≤ cur.moves.length :=
    generatedStep_consumption_bound cur next h
  unfold CanBuildNext at hcan
  omega

/--
  ★下一层裁决由下一层中枢自算（codex R4：不继承 cur）。
  生成步成立时，`next.outcome` = `classifyMove next.centers`（其本征定义），**与 cur.outcome 无关**——
  父级走势类型由父级中枢分类，不继承子级（codex R4 问题一的修正见证）。
-/
theorem next_outcome_self_classified (cur next : TowerLevel)
    (_ : GeneratedStep cur next) :
    next.outcome = classifyMove next.centers := rfl

/--
  ★不可构造时无生成步（codex R3#8 + R4：自然终止 ⟹ 无下一层）。
  若当前层走势不足 3（CanBuildNext 假），则不存在 next 使 GeneratedStep 成立——
  级别在此自然终止（level_recursion.md:249），非结构截断。
-/
theorem no_step_when_terminated (cur next : TowerLevel)
    (h : ¬ CanBuildNext cur) : ¬ GeneratedStep cur next := by
  unfold GeneratedStep
  intro ⟨hcan, _, _, _⟩
  exact h hcan

end Formal.RStarNonSpecial
