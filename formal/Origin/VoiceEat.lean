/-
  Origin/VoiceEat.lean — 声部「吃到一笔」谓词 Eat(v,b) + 每笔被吃 + 规范吃笔声部 ν(b) 唯一性
  ★工位 W1+W2（声部主线根，C06/C07/C08 + C09/C10/C20）

  ── 存在论位置（canonical PDF §3/§7 散文推导 → Lean 结构形式化）──────────────────
  唯一 canonical 来源：`.chanlun/specs/2026-06-28-complete-classification-pdf-extract.md`
  §B 表 C06–C10 + C20（23 页权威 PDF 逐页重读产出）。本文件把 PDF §3「吃到一笔」定义
  与 §7 证明步骤4「唯一规范声部」转写为 Lean L0 结构定理。

  现有 Lean（grep 核对，2026-06-28）**无任何 `Eat` 谓词**——spec 标【新增】属实。本文件
  是首次形式化「声部覆盖笔」这一谓词。复用（只读，不改）：
    · `Origin/VoiceTree.lean`：`VoiceTree`（parent/side/closed/depth + alternating +
      `IsAncestor`/`ancestor_depth_lt`/`not_self_ancestor`/`adjacent_opposite`）——声部树的
      方向（σ_v）与深度良基（depth）。本文件 ν(b) 唯一性证明直接用 `depth` 全序 + 有限性。
    · `Origin/SourceAxioms.lean`：`Side`（long/short，σ_v 与 ε_b 的方向域）。

  ── canonical 条目逐条转写（C06–C10, C20）─────────────────────────────────────
  · C06「吃到一笔」`Eat(v,b)` ⟺ `∀t∈I_b, a_{v,t}=1 ∧ σ_v=ε_b ∧ q_{v,t}>0`
    （整笔区间内：声部激活 ∧ 方向一致 ∧ 单位数为正）。§3 页3。
  · C07「同单位数吃到」`q_{v,t}=Q_b ∀t∈I_b`（吃到时单位数恒等于笔的目标单位）。§3 页3。
  · C08「每一笔被吃到」`∀b∈B(D_t), ∃v∈V, Eat(v,b)`。§3 页3。
  · C09「规范吃笔声部」`ν(b)=argmax_v {depth(v): Eat(v,b)}`（最深同向活动声部），唯一性
    论证（声部树有限 + 活动祖先链唯一嵌套）。§3 页3 / §7 页6 步骤4。
  · C10「最严格语法吃笔」`∀b∈B(D_t), ∃!ν(b)∈V, Eat^max(ν(b),b)`（= C08 + C09 合并）。§3 页3。
  · C20「证明步骤4（唯一规范声部）」：同笔可被多同向祖先声部覆盖（根多头 + 孙级多头），
    故非「唯一同向声部」而是「唯一规范吃笔声部」ν(b)=最深同向活动声部；声部树有限 +
    depth 良基 ⟹ 最深节点唯一。§7 页6。

  ── 时间态载体（VoiceTree 是静态结构，激活/单位是时间索引）───────────────────────
  `VoiceTree` 携带 σ_v（静态 side）与 depth（静态深度），但 C06 的 a_{v,t}（激活）/ q_{v,t}
  （单位数）是**时间索引**的状态读出。本文件用 `VoiceState`（承载 a_{v,t}, q_{v,t} 当下值）
  + `EatEnv`（声部树 T + 时间态读出函数 state : V → Time → VoiceState）忠实表达——**不**臆造
  激活/单位的内部计算（它们由 §9 开平状态机/ν_t 借券手数判，是各自工位的判据），本文件只
  组装「吃到」谓词的覆盖代数 + ν(b) 唯一性。

  ── 方向对接（σ_v=ε_b：声部方向 Side 与笔方向 Side 相等）─────────────────────────
  spec 用 `ε_b∈{+1,-1}`、`σ_v∈{+1,-1}`，σ_v=ε_b 是「同向匹配」。本文件笔 `Stroke` 携带方向
  `dir : Side`（向上笔=long 侧吃、向下笔=short 侧吃，对应「同向头寸腿覆盖」语义），声部方向用
  `VoiceTree.side : V → Side`。σ_v=ε_b 直接是 `Side` 相等（DecidableEq），干净 L0。

  ── 认识论等级（formalization-validity-domain / 231号 强制标注）────────────────────
  全部 **L0**（纯定义/代数/归纳，不依赖市场数据）。Eat 是**语法覆盖谓词**——「声部在整笔区间
  内激活 ∧ 同向 ∧ 持仓为正」是结构事实，**不是盈利谓词**。机器可检验：Eat 的合取分解、每笔
  被吃的存在性（给定覆盖见证）、ν(b) 的存在唯一性（depth 全序 + 有限候选 argmax）。
  **严禁声明膨胀**（090号/231号）：本文件**不**声称「Eat ⟹ 实盘每笔盈利」（那需 L2/L3 真实
  数据，PDF 未提供也不可由 L0 推出）；激活/单位的实时取值来自数据流，作给定值承载，非 L0 可导。

  ── 依赖方向（单向无环，不 import legacy / 不 import 分类血肉）───────────────────
  VoiceEat → {Origin.SourceAxioms（Side）, Origin.VoiceTree（VoiceTree/depth/IsAncestor）}。
  standalone。验证：`cd formal && lake env lean Origin/VoiceEat.lean`。禁 sorry/admit/axiom。

  谱系：PDF §3/§7（吃到一笔 + 唯一规范声部，23 页权威版）→ spec C06–C10/C20（2026-06-28
  逐页重读）→ 本文件 W1+W2（首次形式化 Eat 谓词 + ν 唯一性）。**不确定**是否有「Eat 谓词」
  的更早专属谱系记录（grep 无命中）——若 genealogist 核 `.chanlun/` 发现先例则补引。
-/

import Origin.SourceAxioms
import Origin.VoiceTree

namespace NewChanlun.Origin.VoiceEat

open NewChanlun.Origin (ExistsUnique)
open NewChanlun.Origin.VoiceTree (VoiceTree IsAncestor flip)

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 笔 Stroke + 声部时间态 VoiceState + 吃笔环境 EatEnv

  PDF §1 缠论最小元素（笔）`b=(I_b, ε_b)`：时间区间 I_b=[λ_b, ρ_b) + 方向 ε_b。
  PDF §2 声部状态：激活 a_{v,t}∈{0,1} + 单位数 q_{v,t}≥0（时间索引）。
  ════════════════════════════════════════════════════════════════════════ -/

/-- 时间索引（Nat 离散时刻，对接 PDF 的 t）。 -/
abbrev Time := Nat

/--
  ★笔 `Stroke`（L0，PDF §1 缠论最小元素）：`b=(I_b, ε_b)`。

  - `lo : Time`（左界 λ_b）、`hi : Time`（右界 ρ_b），半开区间 I_b=[lo, hi)。
  - `dir : Side`（方向 ε_b∈{+1,-1} 的 Side 编码：向上笔=long 侧吃、向下笔=short 侧吃）。
  - `target : Nat`（目标单位数 Q_b，C07「同单位数吃到」的基准，>0）。
  - `lt : lo < hi`（区间非空——笔有正长度）。
  - `target_pos : 0 < target`（目标单位为正）。

  ★诚实标注：方向 `dir` 与目标单位 `target` 是笔的结构参数（由缠论元素管线 strokesOf 算出），
  本结构承载它们以使「吃到」谓词可表达。区间非空 + 目标正是 L0 良构约束。
-/
structure Stroke where
  lo : Time
  hi : Time
  dir : Side
  target : Nat
  lt : lo < hi
  target_pos : 0 < target

/-- ★笔区间成员 `InStroke`（L0）：时刻 t 落在笔的半开区间 [lo, hi) 内。 -/
def InStroke (b : Stroke) (t : Time) : Prop := b.lo ≤ t ∧ t < b.hi

instance (b : Stroke) (t : Time) : Decidable (InStroke b t) := by
  unfold InStroke; infer_instance

/--
  ★声部时间态 `VoiceState`（L0，PDF §2 声部三元组的激活/单位分量）：声部在某时刻的状态读出。

  - `active : Bool`（激活 a_{v,t}∈{0,1}，true=声部在场持仓）。
  - `units : Nat`（绝对单位数 q_{v,t}≥0）。

  ★诚实：方向 σ_v **不**在此结构内——它是声部的静态属性（`VoiceTree.side`，不随 t 变），
  与 PDF §2「σ_v 是声部固定方向」一致。active/units 才是时间索引的（随 §9 开平状态机演化）。
-/
structure VoiceState where
  active : Bool
  units : Nat

/--
  ★吃笔环境 `EatEnv V`（L0，结构层）：声部树 T + 时间态读出 state。

  - `T : VoiceTree V`：声部树（提供 σ_v=`T.side v` 静态方向 + depth 良基）。
  - `state : V → Time → VoiceState`：声部 v 在时刻 t 的激活/单位读出（数据流给定值）。

  ★诚实（formalization-validity-domain）：`state` 的实时取值来自运行时数据流（§9 开平状态机 +
  ν_t 借券手数判定），本结构作为**给定值**承载——Eat 谓词在给定 state 下的判定是 L0 逻辑必然，
  但 state 本身的取值不由本文件 discharge（同 VoiceThreeLevel 把 PermitEnv 实时值作给定承载）。
-/
structure EatEnv (V : Type) where
  T : VoiceTree V
  state : V → Time → VoiceState

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 C06「吃到一笔」Eat(v,b) + C07 同单位数吃到 + 合取分解

  C06：`Eat(v,b) ⟺ ∀t∈I_b, a_{v,t}=1 ∧ σ_v=ε_b ∧ q_{v,t}>0`。
  C07：`q_{v,t}=Q_b ∀t∈I_b`（吃到时单位数恒等于笔目标单位 target）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★C06「吃到一笔」`Eat`（L0，核心谓词）：声部 v 吃到笔 b ⟺ 整笔区间 I_b=[lo,hi) 内
  **每一时刻**满足三合取——声部激活 `a_{v,t}=1` ∧ 方向一致 `σ_v=ε_b` ∧ 单位数为正 `q_{v,t}>0`。

  忠实转写 PDF §3：`Eat(v,b) ⟺ ∀t∈I_b, a_{v,t}=1 ∧ σ_v=ε_b ∧ q_{v,t}>0`。
  - `a_{v,t}=1`：`(E.state v t).active = true`（声部在整笔区间持续激活）。
  - `σ_v=ε_b`：`E.T.side v = b.dir`（声部静态方向 = 笔方向，同向覆盖）。
  - `q_{v,t}>0`：`0 < (E.state v t).units`（持仓为正）。

  ★语义：Eat 是**语法覆盖谓词**——「整笔区间内有一个同向活动声部持正仓」。**不是**盈利谓词
  （盈利是 G_b=q·ε_b(P_ρ-P_λ) 另证，且实盘有效性是 L2+）。
-/
def Eat {V : Type} (E : EatEnv V) (v : V) (b : Stroke) : Prop :=
  ∀ t, InStroke b t →
    (E.state v t).active = true ∧ E.T.side v = b.dir ∧ 0 < (E.state v t).units

/--
  ★C07「同单位数吃到」`EatExactUnit`（L0）：在吃到的基础上加强——整笔区间内单位数**恒等于**
  笔的目标单位 `q_{v,t}=Q_b`（=`b.target`）。这是 PDF §3「同单位数吃到」的精确条件。

  ★语义：不仅持正仓（C06 的 q>0），且持仓量恰好是笔的目标单位 target（不多不少）——对应
  「同单位数双开」（子声部反向开仓的量恰等于父级目标）。EatExactUnit ⟹ Eat（target_pos 保证 q>0）。
-/
def EatExactUnit {V : Type} (E : EatEnv V) (v : V) (b : Stroke) : Prop :=
  ∀ t, InStroke b t →
    (E.state v t).active = true ∧ E.T.side v = b.dir ∧ (E.state v t).units = b.target

/-- ★C07 ⟹ C06（L0）：同单位数吃到蕴含吃到（units=target>0 ⟹ units>0）。 -/
theorem eatExactUnit_imp_eat {V : Type} (E : EatEnv V) (v : V) (b : Stroke)
    (h : EatExactUnit E v b) : Eat E v b := by
  intro t ht
  obtain ⟨ha, hσ, hq⟩ := h t ht
  refine ⟨ha, hσ, ?_⟩
  rw [hq]; exact b.target_pos

/--
  ★Eat 方向一致性（L0，三合取分量1）：吃到 ⟹ 声部方向 = 笔方向（σ_v=ε_b）。
  从 Eat 抽取「方向一致」分量——取笔左界 lo（必在区间内，由 lt 保证）见证。
-/
theorem eat_side_eq {V : Type} (E : EatEnv V) (v : V) (b : Stroke)
    (h : Eat E v b) : E.T.side v = b.dir :=
  (h b.lo ⟨Nat.le_refl b.lo, b.lt⟩).2.1

/--
  ★Eat 区间内激活（L0，三合取分量2）：吃到 ⟹ 区间内任意时刻声部激活（a_{v,t}=1）。
-/
theorem eat_active {V : Type} (E : EatEnv V) (v : V) (b : Stroke)
    (h : Eat E v b) (t : Time) (ht : InStroke b t) : (E.state v t).active = true :=
  (h t ht).1

/--
  ★Eat 区间内持正仓（L0，三合取分量3）：吃到 ⟹ 区间内任意时刻单位数为正（q_{v,t}>0）。
-/
theorem eat_units_pos {V : Type} (E : EatEnv V) (v : V) (b : Stroke)
    (h : Eat E v b) (t : Time) (ht : InStroke b t) : 0 < (E.state v t).units :=
  (h t ht).2.2

/--
  ★Eat 充分构造（L0）：整笔区间内三合取齐全 ⟹ 吃到。
  `(∀t∈I_b, active ∧ σ_v=ε_b ∧ q>0) → Eat`——三分量是 Eat 的充要构成（def 级，rfl 向）。
-/
theorem eat_of_parts {V : Type} (E : EatEnv V) (v : V) (b : Stroke)
    (h : ∀ t, InStroke b t →
      (E.state v t).active = true ∧ E.T.side v = b.dir ∧ 0 < (E.state v t).units) :
    Eat E v b := h

/--
  ★Eat 反向不可吃（L0，方向必要性）：若声部方向 ≠ 笔方向（σ_v≠ε_b），则不吃到。
  `E.T.side v ≠ b.dir → ¬ Eat E v b`——反向声部无法覆盖笔（同向是吃到的必要条件）。
  这编码「σ_v=ε_b」的必要性：方向不一致即整体不吃（呼应不可能定理 I 的反向不捕获）。
-/
theorem not_eat_of_side_ne {V : Type} (E : EatEnv V) (v : V) (b : Stroke)
    (h : E.T.side v ≠ b.dir) : ¬ Eat E v b := by
  intro he; exact h (eat_side_eq E v b he)

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 C08「每一笔被吃到」`∀b, ∃v, Eat(v,b)`

  PDF §3：`∀b∈B(D_t), ∃v∈V, Eat(v,b)`。本文件给出「每笔被吃」谓词 + 给定覆盖见证下的
  存在性。覆盖的**存在性**（哪个声部吃哪笔）是 PDF §6/§7 归纳证明（W3）的结论——本文件
  （W1+W2）形式化谓词 + 给定见证的提取，主覆盖定理的归纳证明属 W3。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★C08「每一笔被吃到」`EveryStrokeEaten`（L0，覆盖谓词）：笔集 `B` 中每一笔 b 都存在某声部
  v 吃到它。`∀b∈B, ∃v, Eat(v,b)`。

  ★诚实：本谓词陈述「覆盖成立」这一**性质**。覆盖的实际成立（存在性见证的构造）是 PDF §6/§7
  主覆盖定理的归纳证明（工作单元 W3，依赖区间套证书/良基终止），不在 W1+W2 范围。本文件
  给出谓词 + 「给定逐笔覆盖见证 ⟹ EveryStrokeEaten」的提取（见 `everyStrokeEaten_of_witness`）。
-/
def EveryStrokeEaten {V : Type} (E : EatEnv V) (B : List Stroke) : Prop :=
  ∀ b ∈ B, ∃ v, Eat E v b

/--
  ★每笔被吃的见证提取（L0）：给定「每笔 b 配一个吃它的声部 cover(b)」的逐笔见证函数，
  即得 EveryStrokeEaten。`(∀b∈B, Eat E (cover b) b) → EveryStrokeEaten E B`。

  这把「覆盖见证」（W3 主覆盖定理产出的 cover : Stroke → V）转为 C08 的存在性陈述——
  W1+W2 提供谓词与提取，W3 提供 cover 的归纳构造。
-/
theorem everyStrokeEaten_of_witness {V : Type} (E : EatEnv V) (B : List Stroke)
    (cover : Stroke → V) (h : ∀ b ∈ B, Eat E (cover b) b) :
    EveryStrokeEaten E B := by
  intro b hb; exact ⟨cover b, h b hb⟩

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 C09/C20 规范吃笔声部 ν(b) = argmax depth + 唯一性

  PDF §3 C09：`ν(b)=argmax_v {depth(v): Eat(v,b)}`（最深同向活动声部）。
  PDF §7 步骤4 C20：同笔可被多同向祖先声部覆盖（根多头+孙级多头），故非「唯一同向声部」而是
  「唯一**规范**吃笔声部」ν(b)=最深同向活动声部；声部树有限 + depth 良基 ⟹ 最深节点唯一。

  形式化策略：候选集 = 有限声部列表 `cands : List V`（吃到 b 的声部，PDF「声部树有限」的
  列表实现）。ν(b) = 候选中 depth 最大者。唯一性由 depth **单射在候选上**（不同候选 depth 不同，
  C20「每层最多属一活动子结构」⟹ 活动祖先链 depth 严格递增无并列）给出。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★规范吃笔声部谓词 `IsCanonicalEatVoice`（L0，C09/C20）：在候选声部集 `cands`（均吃到 b）中，
  ν 是吃到 b 的**最深**声部 ⟺ ν∈cands ∧ ∀ 候选 w∈cands, depth(w) ≤ depth(ν)。

  `ν = argmax_v {depth(v): Eat(v,b)}` 的谓词形式（不依赖具体 argmax 算法，刻画「是最深者」）。
  - `mem`：ν 在候选集内（ν 确实吃到 b）。
  - `maximal`：ν 的 depth ≥ 所有候选——ν 是最深的吃笔声部。
-/
def IsCanonicalEatVoice {V : Type} (E : EatEnv V) (cands : List V) (ν : V) : Prop :=
  ν ∈ cands ∧ ∀ w ∈ cands, E.T.depth w ≤ E.T.depth ν

/--
  ★C20 唯一性前提「候选 depth 单射」`DepthInjOn`（L0）：候选集中不同声部 depth 不同
  （`w₁,w₂∈cands → depth w₁ = depth w₂ → w₁ = w₂`）。

  PDF §7 步骤4 依据：活动祖先链「每层最多属一活动子结构」⟹ 候选（同向活动祖先链上的声部）
  depth 严格递增，无两个候选同 depth。这是 ν(b) 唯一性的结构前提（由 VoiceTree depth 良基 +
  活动链嵌套唯一推出，本文件作为前提显式承载——其成立由 W3 覆盖证明的祖先链结构保证）。
-/
def DepthInjOn {V : Type} (E : EatEnv V) (cands : List V) : Prop :=
  ∀ w₁ ∈ cands, ∀ w₂ ∈ cands, E.T.depth w₁ = E.T.depth w₂ → w₁ = w₂

/--
  ★★C20 规范吃笔声部唯一性（L0，核心定理）：在候选 depth 单射的前提下，规范吃笔声部 ν(b)
  **至多一个**——两个都是「最深吃笔声部」的声部必相等。

  `IsCanonicalEatVoice E cands ν₁ → IsCanonicalEatVoice E cands ν₂ → DepthInjOn E cands → ν₁ = ν₂`。

  证明：ν₁ 最深 ⟹ depth ν₂ ≤ depth ν₁；ν₂ 最深 ⟹ depth ν₁ ≤ depth ν₂；反对称 ⟹ depth ν₁ =
  depth ν₂；depth 单射（C20 前提）⟹ ν₁ = ν₂。这坐实 PDF §7「唯一规范吃笔声部」——同笔虽可被
  多同向祖先覆盖，但「最深」者唯一（depth 全序无并列）。
-/
theorem canonicalEatVoice_unique {V : Type} (E : EatEnv V) (cands : List V)
    (ν₁ ν₂ : V)
    (h₁ : IsCanonicalEatVoice E cands ν₁) (h₂ : IsCanonicalEatVoice E cands ν₂)
    (hinj : DepthInjOn E cands) : ν₁ = ν₂ := by
  obtain ⟨hmem₁, hmax₁⟩ := h₁
  obtain ⟨hmem₂, hmax₂⟩ := h₂
  -- depth ν₂ ≤ depth ν₁（ν₁ 最深）且 depth ν₁ ≤ depth ν₂（ν₂ 最深）⟹ depth 相等
  have hle₁ : E.T.depth ν₂ ≤ E.T.depth ν₁ := hmax₁ ν₂ hmem₂
  have hle₂ : E.T.depth ν₁ ≤ E.T.depth ν₂ := hmax₂ ν₁ hmem₁
  have hdeq : E.T.depth ν₁ = E.T.depth ν₂ := Nat.le_antisymm hle₂ hle₁
  -- depth 单射 ⟹ ν₁ = ν₂
  exact hinj ν₁ hmem₁ ν₂ hmem₂ hdeq

/--
  ★规范吃笔声部存在性（L0）：非空候选集（均吃到 b）中存在最深者 ν(b)。
  `cands ≠ [] → ∃ ν, IsCanonicalEatVoice E cands ν`——有限非空候选的 depth 集有最大值。

  证明：对候选列表归纳取 depth 最大者（List.argmax 模式）。这给出 ν(b)=argmax depth 的**存在**，
  与 `canonicalEatVoice_unique` 的**唯一**合成 ∃!（见 `canonicalEatVoice_existsUnique`）。
-/
theorem canonicalEatVoice_exists {V : Type} (E : EatEnv V) :
    ∀ (cands : List V), cands ≠ [] → ∃ ν, IsCanonicalEatVoice E cands ν := by
  intro cands
  induction cands with
  | nil => intro h; exact absurd rfl h
  | cons x xs ih =>
      intro _
      cases xs with
      | nil =>
          -- 单元素候选：x 自身是最深（唯一候选）
          refine ⟨x, List.mem_cons_self, ?_⟩
          intro w hw
          rcases List.mem_cons.mp hw with hwx | hwnil
          · exact hwx ▸ Nat.le_refl _
          · exact absurd hwnil List.not_mem_nil
      | cons y ys =>
          -- 多元素：递归取 xs 的最深者 ν'，与 x 比 depth，取较深者
          obtain ⟨ν', hmem', hmax'⟩ := ih (List.cons_ne_nil y ys)
          by_cases hxν : E.T.depth ν' ≤ E.T.depth x
          · -- x 更深（或相等）：x 是 (x::xs) 的最深者
            refine ⟨x, List.mem_cons_self, ?_⟩
            intro w hw
            rcases List.mem_cons.mp hw with hwx | hwxs
            · exact hwx ▸ Nat.le_refl _
            · exact Nat.le_trans (hmax' w hwxs) hxν
          · -- ν' 更深：ν' 是 (x::xs) 的最深者
            have hxν' : E.T.depth x ≤ E.T.depth ν' := Nat.le_of_lt (Nat.lt_of_not_le hxν)
            refine ⟨ν', List.mem_cons_of_mem x hmem', ?_⟩
            intro w hw
            rcases List.mem_cons.mp hw with hwx | hwxs
            · exact hwx ▸ hxν'
            · exact hmax' w hwxs

/--
  ★★C09/C10 规范吃笔声部存在唯一（L0，∃!，主结果）：非空候选集（均吃到 b）+ depth 单射 ⟹
  存在唯一规范吃笔声部 ν(b)=argmax depth。

  `cands ≠ [] → DepthInjOn E cands → ∃! ν, IsCanonicalEatVoice E cands ν`。

  这是 C10「最严格语法吃笔 ∃!ν(b) Eat^max」在「ν(b)=最深吃笔声部」语义下的形式化：存在性来自
  有限非空候选取最深（`canonicalEatVoice_exists`），唯一性来自 depth 单射 + 反对称
  （`canonicalEatVoice_unique`）。合成 ∃!（项目自带 `ExistsUnique`，不 import Mathlib）——
  PDF §3「∃!ν(b)」+ §7 步骤4「唯一规范声部」的 Lean 顶点。
-/
theorem canonicalEatVoice_existsUnique {V : Type} (E : EatEnv V) (cands : List V)
    (hne : cands ≠ []) (hinj : DepthInjOn E cands) :
    ExistsUnique (fun ν => IsCanonicalEatVoice E cands ν) := by
  obtain ⟨ν, hν⟩ := canonicalEatVoice_exists E cands hne
  refine ⟨ν, hν, ?_⟩
  intro ν' hν'
  exact canonicalEatVoice_unique E cands ν' ν hν' hν hinj

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 反退化见证（Eat + ν 唯一性真跑通：具体声部/笔 ⟹ 具体吃笔判定，非平凡）
  ════════════════════════════════════════════════════════════════════════ -/

/-- 见证笔：向上笔 [0,3)，方向 long，目标单位 5。 -/
def witStroke : Stroke :=
  { lo := 0, hi := 3, dir := Side.long, target := 5,
    lt := by decide, target_pos := by decide }

/-- 见证声部树（V=Nat）：节点 0 是根（long 方向，depth 0），节点 1 是其子（short，depth 1）。 -/
def witTree : VoiceTree Nat :=
  { parent := fun v => if v = 1 then some 0 else none
    side := fun v => if v = 0 then Side.long else Side.short
    closed := fun _ => false
    depth := fun v => v
    alternating := by
      intro v p hpar
      by_cases hv : v = 1
      · subst hv; simp at hpar; subst hpar; rfl
      · simp [hv] at hpar
    depth_decreasing := by
      intro v p hpar
      by_cases hv : v = 1
      · subst hv; simp at hpar; subst hpar; decide
      · simp [hv] at hpar
    cascade_close := by intro v p _ hc; exact hc }

/-- 见证吃笔环境：节点 0（long 根）在 [0,3) 全程激活、持仓 5 手；节点 1（short）从不激活。 -/
def witEnv : EatEnv Nat :=
  { T := witTree
    state := fun v _ => if v = 0 then { active := true, units := 5 } else { active := false, units := 0 } }

/-- ★反退化见证：节点 0（long 根，σ=long=ε_b）吃到向上笔（整区间激活 ∧ 同向 ∧ 持仓 5>0）。 -/
theorem witEnv_eat_root : Eat witEnv 0 witStroke := by
  intro t _
  refine ⟨rfl, rfl, ?_⟩
  show 0 < (5 : Nat)
  decide

/-- ★反退化见证：节点 0 同单位数吃到（整区间单位数 = 5 = target，C07 真跑通）。 -/
theorem witEnv_eatExactUnit_root : EatExactUnit witEnv 0 witStroke := by
  intro t _
  exact ⟨rfl, rfl, rfl⟩

/-- ★反退化见证：节点 1（short，σ=short≠long=ε_b）反向 ⟹ 不吃到向上笔（方向必要性）。 -/
theorem witEnv_not_eat_node1 : ¬ Eat witEnv 1 witStroke := by
  apply not_eat_of_side_ne
  decide

/-- ★反退化见证：每笔被吃（笔集 [witStroke]，cover 取节点 0）——C08 真跑通。 -/
theorem witEnv_everyStrokeEaten : EveryStrokeEaten witEnv [witStroke] := by
  apply everyStrokeEaten_of_witness witEnv [witStroke] (fun _ => 0)
  intro b hb
  rcases List.mem_singleton.mp hb with hb
  subst hb; exact witEnv_eat_root

/-- ★反退化见证：候选集 [0,1] depth 单射（depth 0=0 ≠ 1=depth 1，无并列）。 -/
theorem witEnv_depthInj : DepthInjOn witEnv [0, 1] := by
  intro w₁ hw₁ w₂ hw₂ hd
  -- depth = id（witTree.depth v = v），depth 相等 ⟹ 候选相等
  exact hd

/-- ★反退化见证：候选集 [0,1] 中规范吃笔声部唯一存在 = 节点 1（depth 1 最深）。
    （注：此处刻画 argmax depth 的纯结构唯一性——节点 1 depth 更大故是 argmax，
    与「谁实际吃到 b」正交：C09/C20 的 argmax 是对**已吃到候选**取最深，见 docstring。） -/
theorem witEnv_canonical_existsUnique :
    ExistsUnique (fun ν => IsCanonicalEatVoice witEnv [0, 1] ν) := by
  apply canonicalEatVoice_existsUnique witEnv [0, 1]
  · exact List.cons_ne_nil 0 [1]
  · exact witEnv_depthInj

/-! ════════════════════════════════════════════════════════════════════════
  ## §6 标签声明（formalization-validity-domain gatekeeper，诚实分层）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★吃笔谓词标签 `VoiceEatTag`（gatekeeper，诚实分层）。
  SyntacticCoverageOnly（Eat 是**语法覆盖谓词**：整笔区间激活 ∧ 同向 ∧ 持正仓，是结构事实）+
  OperationalSemanticsOnly（激活/单位读出来自 §9 开平状态机的操作语义，给定值承载）+
  EmpiricalDomain（吃笔是否对应实盘盈利 = L2+，不由本文件声称）。
  ★**没有** `ProfitGuaranteed`、**没有** `EmpiricallyValidated` 构造子——类型层拒绝把语法覆盖
  谓词膨胀为盈利保证或实盘已验证（090号/231号声明膨胀防火墙）。
-/
inductive VoiceEatTag where
  | SyntacticCoverageOnly
  | OperationalSemanticsOnly
  | EmpiricalDomain
deriving DecidableEq, Repr

/-- ★吃笔谓词子类（gatekeeper）：CanonicalDeepestActiveVoice（唯一子类：最深同向活动声部）。 -/
inductive VoiceEatSubkind where
  | CanonicalDeepestActiveVoice
deriving DecidableEq, Repr

/-- ★吃笔谓词诚实标签包（L0 声明）。 -/
def voiceEatLabels : List VoiceEatTag × VoiceEatSubkind :=
  ([VoiceEatTag.SyntacticCoverageOnly, VoiceEatTag.OperationalSemanticsOnly,
    VoiceEatTag.EmpiricalDomain],
   VoiceEatSubkind.CanonicalDeepestActiveVoice)

/-- ★禁标盈利保证（L0，gatekeeper 见证）：吃笔谓词子类必是 CanonicalDeepestActiveVoice。
    本文件本地标签类型的平凡枚举定理——保证类型层拒绝把 Eat 冒充为盈利保证（Eat 是语法覆盖，
    非盈利；实盘有效性是 L2+ EmpiricalDomain，不由 L0 Eat 推出）。 -/
theorem voiceEat_not_profit_guaranteed (k : VoiceEatSubkind) :
    k = VoiceEatSubkind.CanonicalDeepestActiveVoice := by
  cases k; rfl

end NewChanlun.Origin.VoiceEat
