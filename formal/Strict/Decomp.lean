/-
  ≈ₙ 完整结构等价 + GaugeNormal 截面唯一 + 多义反例（最严格完全分类标准 第2部分，L0）
  ★t-decomp-uniq 工位（任务 #62；603/615 谱系）— canonical ∼ₙ（codex 全权代理裁决修订版）

  ── codex 全权代理裁决（物证 /tmp/codex_simn_ruling.md，exit0）：选 (b) 完整语义代表 ──
  上一版 `∼ₙ := (I = I)`，`I = leaves`（底层 segment 序列）被判为 **摘要核**——leaves 丢弃
  中间 compose 节点的 `centers / level / 递归结构`，会把「同底层 segment 序列但不同中枢/
  内部结构」的分解误判等价，与裁决1「走势 = 结构等价 B（保留分型/笔/线段/中枢序列）」冲突。
  上一版「`|𝒟ₙ(h)/∼ₙ| = 1` 无条件」之所以成立，正是因为 leaves 摘要过粗，把本不该等价的
  也合并了——这是用过粗等价掩盖真多义性，是隐蔽 workaround。本版按裁决修订为本质 (b)。

  ── 本版的严格形式（站在 (b) 完整语义代表） ──
  1. `StructEqN` / `≈ₙ`：**完整结构等价**——保留 `Direction / subs / centers / level / 递归
     结构`，**只允许结合律括号化归一**（(a⊕b)⊕c ≈ₙ a⊕(b⊕c)）。实现为「connect-扁平化
     规范形式相等」：`flattenConnect` 只解开 connect 二元包装（`compose [_,_] [] 1`），
     **保留** 所有真实走势子树（centers≠[] 或非二元）的完整身份。
  2. **不强证 `|𝒟ₙ(h)/≈ₙ| = 1`**——gauge 前商可能 **> 1**（这是真多义性 T₅₂）。补
     **多义反例定理** `decomp_ambiguity_witness`：∃ h t₁ t₂ 同 base 合法但 ¬ StructEqN
     （不同中枢/方向不可被 ≈ₙ 合并，只有括号化才可合并）。
  3. `GaugeNormalₙ` + `gaugeFixₙ`：**按 level 最小、平级最左** 的纤维内确定性选择（迁入
     cc-spiral 的 gauge/fiber/minLift 逻辑，spiral-extra-from-cc-spiral.md）。★诚实：这是
     确定性列表选择，**不** 形式化完整区间套语义（仅 level 最小+平级最左这一可计算判据）。
  4. 唯一定理改为 **截面唯一**（非商唯一）：`gauge_section_unique`（带 ValidDecomp 纤维前提，
     结论含截面在纤维内）+ `gaugeFix_section_valid`（截面不逃逸纤维）+
     `gauge_normal_subsingleton`（候选集层规范代表至多一个）。
     ★诚实：`gaugeFix` 是「按 level 最小、平级最左」的纤维内确定性选择，**不** 声称完整
     区间套语义；「所有合法分解 ≈ₙ 同一截面」即商唯一，已被 §5 多义反例否证，**不** 强证。

  ── 摘要核保留（裁决：旧 I=leaves 重命名为 SummarySimN，作 GaugeKey / 截面选择工具） ──
  `SummarySimN` = 旧 leaves 核，**不是** 语义 ∼ₙ——仅作规范摘要 / 截面选择的可计算工具。
  诚实标有效域：SummarySimN 只在「语义仅由底层 segment 序列决定」的退化范畴成立，
  比 MoveOutcome 标签细、比完整 Move 语义（StructEqN）粗。§5 多义反例独立见证「同
  SummarySimN 但 ¬≈ₙ」⟹ 摘要核确实合并了 ≈ₙ 区分的对象（摘要更粗的硬证据）。

  ── 严格性铁律（formalization-validity-domain + no-patch-mentality + codex 防坑） ──
  - `≈ₙ` 是 **结构等价**（保 Direction/centers/level），不是标签核（不是 MoveOutcome enum 的核）。
  - **不声称 `X/≈ₙ ≅ P` 双射**——`liftedNormalForm` 是单向良定义不变量（无逆/inj/surj）。
    完整双射（不变性+完备性+可实现性）是 T-trend/T-bsp 工位，本文件诚实标「不变量良定义」。
  - `Decomp h := {t // SummaryI t = h}` 子类型以「投影回 base h」为 **前提**，非由 gaugeFix
    端到端推导落入——注释不暗示已建 gaugeFix→Decomp 流水线。

  认识论等级：所有定理 **L0**（定义内蕴，对 Move/List/Quotient 结构归纳，零数据依赖）。
  Lean 通过 = 逻辑/管线正确（L0），断言「括号化在 ≈ₙ 下归一、中枢/方向差异不归一、gauge
  规范代表唯一」这些代数事实，**不** 断言「任何真实走势分解经验唯一」（后者 L2/L3）。
-/

import Formal.TrendTrichotomy
import Formal.CenterTrichotomy
import Formal.RecursiveConstruction
import Formal.EvalSoundness

namespace Strict.Decomp

open Formal.TrendTrichotomy (Direction)
open Formal.CenterTrichotomy (Center MoveOutcome)
open Formal.RecursiveConstruction (Move classifyMove)
open Formal.EvalSoundness (leaves leaves_segment leaves_compose leaves_flatten_flat)

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 连接 ⊕ 与 connect-扁平化（≈ₙ 的载体，保完整结构，只解括号）

  连接 `⊕` = 二元 `Move.compose [a, b] [] 1`（次级别走势序列拼接的二元包装，第36课）。
  `flattenConnect` **只解开 connect 二元包装**（`compose [_,_] [] 1`），保留所有真实走势
  子树（centers≠[] 或非二元 compose 或 segment）的 **完整身份**——这是 ≈ₙ 的规范形式：
  括号化被归一，但 Direction/centers/level/递归内部结构全部保留。
  ════════════════════════════════════════════════════════════════════════ -/

/-- 连接 ⊕：二元 compose 包装（level 1，无自身中枢——纯括号节点）。 -/
def connect (a b : Move) : Move := Move.compose [a, b] [] 1

/--
  ★connect-扁平化（≈ₙ 规范形式，保完整结构）：递归解开 connect 二元包装。
  - `segment`：完整保留（叶含 Direction/lo/hi）。
  - `compose [a, b] [] 1`（**恰** connect 签名：二元 + 空中枢 + level 1）：解括号，
    递归 flatten 两子并拼接。
  - 其它 `compose`（centers≠[] 或非二元或 level≠1）：**完整保留为单点**（真实走势节点，
    含 centers/level/递归结构，不拆）。

  这保证 flattenConnect 只归一 connect 括号化，**不丢** 任何真实走势的结构信息。
-/
def flattenConnect : Move → List Move
  | Move.segment d lo hi => [Move.segment d lo hi]
  | Move.compose subs centers lvl =>
      match subs, centers, lvl with
      | [a, b], [], 1 => flattenConnect a ++ flattenConnect b
      | _, _, _ => [Move.compose subs centers lvl]

/-- segment 的 connect-扁平 = 完整单点自身。 -/
theorem flattenConnect_segment (d : Direction) (lo hi : Int) :
    flattenConnect (Move.segment d lo hi) = [Move.segment d lo hi] := rfl

/-- connect 的 connect-扁平 = 两子扁平拼接（解括号）。 -/
theorem flattenConnect_connect (a b : Move) :
    flattenConnect (connect a b) = flattenConnect a ++ flattenConnect b := rfl

/--
  ★真实走势 compose（含中枢，centers≠[]）的 connect-扁平 = 完整单点自身（L0，保结构核心）：
  一个携带中枢序列的走势节点 **不被** flattenConnect 拆开——它作为完整结构对象保留
  （含 centers / level）。这是 ≈ₙ「保 Direction/subs/centers」的实现保证：真实走势的
  中枢/递归结构不会被括号归一抹掉（与摘要核 leaves 的本质区别）。
-/
theorem flattenConnect_real_move (subs : List Move) (c : Center) (cs : List Center) (lvl : Nat) :
    flattenConnect (Move.compose subs (c :: cs) lvl) = [Move.compose subs (c :: cs) lvl] := by
  -- centers = c :: cs 非空，无论 subs/lvl 如何都落入 match 的 `_, _, _` 通配分支
  cases subs with
  | nil => rfl
  | cons a rest =>
      cases rest with
      | nil => rfl
      | cons b rest2 =>
          cases rest2 with
          | nil => rfl
          | cons _ _ => rfl

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 StructEqN / ≈ₙ 完整结构等价：connect-扁平规范形式相等

  `t₁ ≈ₙ t₂ := flattenConnect t₁ = flattenConnect t₂`（连接括号化归一后完整结构相等）。
  保留 Direction/centers/level/递归结构——只把 connect 括号位置归一。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★≈ₙ / StructEqN：两分解树 connect-扁平规范形式相等（保完整结构，归一括号）。 -/
def StructEqN (t₁ t₂ : Move) : Prop := flattenConnect t₁ = flattenConnect t₂

@[inherit_doc] infix:50 " ≈ₙ " => StructEqN

/--
  ★≈ₙ 是等价关系（L0，标准第2部分要求 Equivalence）：自反/对称/传递。
  ≈ₙ 是「flattenConnect 相等的拉回」，继承 Eq 等价性——这是严格等价关系，不变量是
  **完整结构规范形式**（List Move，保 Direction/centers/level），不是摘要标签。
-/
theorem structEqN_equivalence : Equivalence StructEqN where
  refl _ := rfl
  symm h := h.symm
  trans h₁ h₂ := h₁.trans h₂

/-- ≈ₙ 作为 Setoid（供 Quotient 使用）。 -/
instance structEqN_setoid : Setoid Move where
  r := StructEqN
  iseqv := structEqN_equivalence

/--
  ★T₅₃ 结合律入 ≈ₙ（L0，核心组装）：(A ⊕ B) ⊕ C ≈ₙ A ⊕ (B ⊕ C)。
  不同括号化的连接树字面 **不等**（connect 嵌套结构不同），但 connect-扁平规范形式都
  = flattenConnect A ++ flattenConnect B ++ flattenConnect C（append 结合律），故 ≈ₙ-等价。
  这是 `Spiral.T53_connection_assoc`（List append 结合律）在 **完整结构等价** 上的落地——
  **多义性 by 不同括号化** 的形式化，且保留 A/B/C 各自的完整结构（不丢方向/中枢）。
-/
theorem structEqN_connect_assoc (A B C : Move) :
    connect (connect A B) C ≈ₙ connect A (connect B C) := by
  unfold StructEqN
  rw [flattenConnect_connect, flattenConnect_connect,
      flattenConnect_connect, flattenConnect_connect]
  rw [List.append_assoc]

/--
  ★≈ₙ 保完整结构·中枢差异不归一（L0，对照结合律：≈ₙ 不是全关系）：
  两个携带不同中枢的真实走势 compose（即使同底层）**不** ≈ₙ-等价。这与
  `structEqN_connect_assoc`（括号化可归一）对照：≈ₙ 只归一 connect 括号，**保留** 中枢/
  方向/递归结构差异——见证 (b) 完整语义代表（非 (a) 摘要核）。
-/
theorem structEqN_keeps_center (subs : List Move) (c₁ c₂ : Center) (lvl : Nat)
    (h : c₁ ≠ c₂) :
    ¬ StructEqN (Move.compose subs [c₁] lvl) (Move.compose subs [c₂] lvl) := by
  unfold StructEqN
  rw [flattenConnect_real_move subs c₁ [] lvl, flattenConnect_real_move subs c₂ [] lvl]
  intro heq
  injection heq with hmove _
  injection hmove with _ hcenters _
  injection hcenters with hc _
  exact h hc

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 摘要核 SummarySimN（旧 I=leaves，裁决降级为 GaugeKey / base 投影工具）

  `SummaryI = leaves`（底层 segment 序列）。`SummarySimN := SummaryI 相等`。
  ★诚实标（codex 防坑）：这 **不是** 语义 ∼ₙ——leaves 丢 centers/level/递归结构。
  仅作可计算的规范摘要 / base 投影工具。有效域 = 「语义仅由底层 segment 序列决定」退化范畴。
  ════════════════════════════════════════════════════════════════════════ -/

/-- 摘要不变量（旧 I=leaves）：底层 segment 时间序（剥离全部层级 + 中枢）。 -/
def SummaryI (m : Move) : List Move := leaves m

/--
  ★摘要核 SummarySimN（**非语义 ∼ₙ**，裁决降级）：底层 segment 序列相等。
  保留作 GaugeKey / base 投影工具。**不得** 作 canonical 语义等价（丢中枢/方向/递归结构）。
-/
def SummarySimN (t₁ t₂ : Move) : Prop := SummaryI t₁ = SummaryI t₂

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 𝒟ₙ(h)：固定 base h 的全部合法分解（依赖 h，base 投影 = SummaryI）

  base 投影用 `SummaryI = leaves`（底层 segment 序列 = base 走势数据）。
  𝒟ₙ(h) 含多个 **≈ₙ-不等** 的分解（不同中枢/方向）——这是 gauge 前真多义性（>1）。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★𝒟ₙ(h)：底层 base 投影为 h 的全部合法分解（依赖 h 的子类型）。 -/
def Decomp (h : List Move) : Type := { t : Move // SummaryI t = h }

/-- 𝒟ₙ(h) 中分解的 base 投影恒为 h（子类型定义性质）。 -/
theorem decomp_proj (h : List Move) (t : Decomp h) : SummaryI t.val = h := t.property

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 多义反例：𝒟ₙ(h)/≈ₙ gauge 前 > 1（T₅₂ 真多义性，裁决要求，不强证 =1）

  存在同 base h 的两个合法分解，≈ₙ-**不等价**（不同中枢序列不可被括号归一合并）。
  这关死「用过粗等价强证商唯一」——多义性是真的，唯一性只能在 gaugeFix（level 最小+平级
  最左）选出的纤维内规范截面层成立（§6），不是商唯一。
  同时见证「摘要核 SummarySimN 比 ≈ₙ 粗」：t₁/t₂ 同 SummaryI 但 ¬≈ₙ。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★多义反例（L0，裁决核心要求）：∃ base h，其 𝒟ₙ(h) 含两个 ≈ₙ-不等的合法分解。

  取同一底层三段 base `[s1, s2, s3]`，两个 **真实走势 compose**（携带不同中枢序列）：
  - `t₁ = compose [s1,s2,s3] [c₁] 1`（中枢 c₁）；
  - `t₂ = compose [s1,s2,s3] [c₂] 1`（中枢 c₂≠c₁，外缘 gg 不同 3 vs 4）。
  二者 SummaryI 都 = [s1,s2,s3]（同 base，叶相同），但 flattenConnect 各为完整单点自身
  （真实走势不被拆，§1 flattenConnect_real_move），因 c₁≠c₂ 故 ¬ StructEqN。

  ⟹ 同 base 但不同中枢的分解 **≈ₙ-不等** ⟹ `|𝒟ₙ([s1,s2,s3])/≈ₙ| ≥ 2`（gauge 前真多义）。
  括号化可被 ≈ₙ 合并，但中枢/方向差异不可——这正是 (b) 完整语义代表的内容。
  附带：t₁/t₂ 同 SummaryI 却 ¬≈ₙ ⟹ 摘要核 SummarySimN 真比 ≈ₙ 粗（合并了 ≈ₙ 区分对象）。
-/
theorem decomp_ambiguity_witness :
    ∃ (h : List Move) (t₁ t₂ : Move),
      SummaryI t₁ = h ∧ SummaryI t₂ = h ∧ SummarySimN t₁ t₂ ∧ ¬ StructEqN t₁ t₂ := by
  let s1 := Move.segment Direction.up 0 1
  let s2 := Move.segment Direction.down 1 0
  let s3 := Move.segment Direction.up 0 2
  -- 两个不同中枢（core_valid: zd<zg；外缘包含核心 dd≤zd, zg≤gg）
  let c₁ : Center := ⟨0, 1, 2, 3, by decide, by decide, by decide⟩
  let c₂ : Center := ⟨0, 1, 2, 4, by decide, by decide, by decide⟩
  let t₁ := Move.compose [s1, s2, s3] [c₁] 1
  let t₂ := Move.compose [s1, s2, s3] [c₂] 1
  have hflat : ∀ x ∈ [s1, s2, s3], ∃ d lo hi, x = Move.segment d lo hi := by
    intro x hx
    simp only [List.mem_cons, List.not_mem_nil, or_false] at hx
    rcases hx with h | h | h
    · exact ⟨Direction.up, 0, 1, h⟩
    · exact ⟨Direction.down, 1, 0, h⟩
    · exact ⟨Direction.up, 0, 2, h⟩
  have hbase : SummaryI t₁ = [s1, s2, s3] := by
    show leaves (Move.compose [s1, s2, s3] [c₁] 1) = [s1, s2, s3]
    rw [leaves_compose]; exact leaves_flatten_flat [s1, s2, s3] hflat
  have hbase' : SummaryI t₂ = [s1, s2, s3] := by
    show leaves (Move.compose [s1, s2, s3] [c₂] 1) = [s1, s2, s3]
    rw [leaves_compose]; exact leaves_flatten_flat [s1, s2, s3] hflat
  have hc_ne : c₁ ≠ c₂ := by
    intro h
    have : c₁.gg = c₂.gg := congrArg Center.gg h
    simp [c₁, c₂] at this
  refine ⟨[s1, s2, s3], t₁, t₂, hbase, hbase', ?_, ?_⟩
  · -- SummarySimN t₁ t₂：同 SummaryI（都 = [s1,s2,s3]）
    show SummarySimN t₁ t₂
    unfold SummarySimN; rw [hbase, hbase']
  · -- ¬ StructEqN：不同中枢不可合并
    exact structEqN_keeps_center [s1, s2, s3] c₁ c₂ 1 hc_ne

/-! ════════════════════════════════════════════════════════════════════════
  ## §6 ValidDecomp 纤维 + gaugeFix 截面：截面在纤维内 + 唯一（裁决：非商唯一）

  ★codex 异质审计修正（首轮 FAIL：§6 把「列表选择函数单值」冒充「同 base 合法分解纤维上
  区间套截面唯一」= 声明膨胀）。本版按 codex 批评修正：
  - 建 `ValidDecomp h t`（t 是 base h 的合法分解纤维成员）——之前缺。
  - 证 `gaugeFix` 选出的截面 **是 cands 的成员**（不逃逸候选集）⟹ 若 cands 是 base h 纤维，
    截面 **ValidDecomp h**（截面在纤维内，真非平凡内容）。
  - `gauge_section_unique` 诚实标为 **gaugeFix 列表选择函数单值**（不冒充纤维截面唯一）。
  - **删除** 膨胀注释（"区间套规范固定""所有合法分解归约"超出 Lean 内容）。

  诚实有效域：gaugeFix 是「按级别最小、平级最左优先」的确定性列表选择（cc-spiral minLift/
  foldl 同构）。它给出 **纤维内的确定性规范截面选择**——但「该截面是区间套意义下的规范代表」
  「所有合法分解 ≈ₙ 该截面」**未** 在本文件证明（前者需区间套语义，后者即商唯一已被 §5 否证）。
  ════════════════════════════════════════════════════════════════════════ -/

/-- ★合法分解谓词：t 是 base h 的合法分解 ⟺ 投影回 h（纤维 𝒟ₙ(h) 成员）。 -/
def ValidDecomp (h : List Move) (t : Move) : Prop := SummaryI t = h

/-- 分解的 gauge 级别（= 分解树根级别，gaugeFix 的排序键，cc-spiral liftLevel 同构）。 -/
def gaugeLevel (m : Move) : Nat := m.level

/-- 两分解取级别较小者（平级保左，cc-spiral minLift 同构）。 -/
def minGauge (a b : Move) : Move :=
  if gaugeLevel a ≤ gaugeLevel b then a else b

/-- minGauge 结果总是两输入之一（选择性，不造新分解）。 -/
theorem minGauge_mem (a b : Move) : minGauge a b = a ∨ minGauge a b = b := by
  unfold minGauge
  by_cases h : gaugeLevel a ≤ gaugeLevel b
  · exact Or.inl (if_pos h)
  · exact Or.inr (if_neg h)

/-- gaugeFix：从候选分解列表按级别最小、平级最左折叠选截面（foldl，cc-spiral gaugeFix 同构）。 -/
def gaugeFix : List Move → Option Move
  | [] => none
  | x :: xs => some (xs.foldl minGauge x)

/-- GaugeNormal：c 是候选集 cands 的 gaugeFix 选出截面。 -/
def GaugeNormal (cands : List Move) (c : Move) : Prop := gaugeFix cands = some c

/--
  ★gaugeFix 偏函数单值（L0）：候选集要么无截面（none）要么唯一（some 单值）。
  ★诚实：这是 **列表选择函数单值**（Option 的至多一值），**不是** 纤维截面唯一
  （codex#2/codex 复审区分点）——真正的非平凡内容是 §下 gaugeFix_section_valid（截面在纤维内）。
-/
theorem gaugeFix_unique (cands : List Move) :
    gaugeFix cands = none ∨ ∃ c, gaugeFix cands = some c := by
  cases h : gaugeFix cands with
  | none => exact Or.inl rfl
  | some c => exact Or.inr ⟨c, rfl⟩

/-- ★空候选 ⟹ 无截面（L0，偏函数诚实，不伪造全函数）。 -/
theorem gaugeFix_empty_none : gaugeFix [] = none := rfl

/--
  ★foldl minGauge 结果 ∈ 候选（L0，截面不逃逸的核心引理）：
  `xs.foldl minGauge x ∈ x :: xs`——折叠每步取两输入之一（minGauge_mem），故结果总是
  某个候选元素，不造新分解。对 xs 结构归纳 + minGauge 选择性。
-/
theorem foldl_minGauge_mem (x : Move) (xs : List Move) :
    xs.foldl minGauge x ∈ x :: xs := by
  induction xs generalizing x with
  | nil => exact List.mem_cons_self
  | cons y ys ih =>
      -- foldl minGauge x (y::ys) = foldl minGauge (minGauge x y) ys
      simp only [List.foldl_cons]
      have hstep := ih (minGauge x y)
      -- hstep : foldl .. (minGauge x y) ys ∈ minGauge x y :: ys
      rcases List.mem_cons.mp hstep with heq | hmem
      · -- 结果 = minGauge x y ∈ {x, y} ⊆ x :: y :: ys
        rcases minGauge_mem x y with hx | hy
        · rw [heq, hx]; exact List.mem_cons_self
        · rw [heq, hy]; exact List.mem_cons_of_mem _ List.mem_cons_self
      · -- 结果 ∈ ys ⊆ x :: y :: ys
        exact List.mem_cons_of_mem _ (List.mem_cons_of_mem _ hmem)

/--
  ★gaugeFix 截面 ∈ 候选集（L0）：gaugeFix cands = some c ⟹ c ∈ cands。
  截面是候选中的某个分解（不造新对象）——gaugeFix（按 level 最小、平级最左）**选** 截面，
  不 **生成** 截面。
-/
theorem gaugeFix_section_mem (cands : List Move) (c : Move)
    (h : GaugeNormal cands c) : c ∈ cands := by
  unfold GaugeNormal gaugeFix at h
  cases cands with
  | nil => exact absurd h (by simp)
  | cons x xs =>
      have : c = xs.foldl minGauge x := (Option.some.inj h).symm
      rw [this]
      exact foldl_minGauge_mem x xs

/--
  ★gaugeFix 截面在纤维内（L0，裁决要求的非平凡内容，关死「选择函数单值冒充截面唯一」）：
  若候选集全是 base h 的合法分解（纤维 𝒟ₙ(h)），则 gaugeFix 选出的截面 **也是** base h 的
  合法分解（ValidDecomp h c）。截面不逃逸纤维——gaugeFix（按 level 最小、平级最左）从纤维内
  选规范代表，选出的仍在纤维内。这是 codex 首轮 FAIL 指出的缺失内容（之前只有列表选择单值）。
-/
theorem gaugeFix_section_valid (h : List Move) (cands : List Move) (c : Move)
    (hfiber : ∀ t ∈ cands, ValidDecomp h t)
    (hsec : GaugeNormal cands c) : ValidDecomp h c :=
  hfiber c (gaugeFix_section_mem cands c hsec)

/--
  ★纤维内 gaugeFix 截面唯一（L0，裁决「截面唯一」诚实形式）：
  固定 base h 的合法分解纤维 cands，其 gaugeFix 截面 **至多一个且在纤维内**：
  两截面 c₁ c₂ 相等（gaugeFix 单值）+ 都 ValidDecomp h（在纤维内）。
  这是「gaugeFix（level 最小+平级最左）从纤维内选确定性规范截面」的诚实陈述——多义性真存在
  （§5），但纤维的 gaugeFix 截面唯一确定且不逃逸纤维。**不冒充** 商唯一（§5 已否证 |·/≈ₙ|=1），
  **也不** 声称完整区间套语义（只是 level 最小+平级最左的确定性选择）。
-/
theorem gauge_section_unique (h : List Move) (cands : List Move) (c₁ c₂ : Move)
    (hfiber : ∀ t ∈ cands, ValidDecomp h t)
    (h₁ : GaugeNormal cands c₁) (h₂ : GaugeNormal cands c₂) :
    c₁ = c₂ ∧ ValidDecomp h c₁ ∧ ValidDecomp h c₂ := by
  refine ⟨?_, gaugeFix_section_valid h cands c₁ hfiber h₁,
              gaugeFix_section_valid h cands c₂ hfiber h₂⟩
  unfold GaugeNormal at h₁ h₂
  rw [h₁] at h₂
  exact Option.some.inj h₂

/--
  ★GaugeNormal 截面 Subsingleton（L0，|纤维规范截面| ≤ 1）：
  固定候选集，`{c // GaugeNormal cands c}` 至多一个元素（gaugeFix 单值）。
  与 §5 多义反例（gauge 前 ≥2）对照：gauge 前多义、gauge 后规范代表唯一。
  ★诚实：这是候选集层的选择单值（不依赖纤维前提）；纤维内截面唯一 + 在纤维内见
  `gauge_section_unique`（带 ValidDecomp）。
-/
theorem gauge_normal_subsingleton (cands : List Move) :
    ∀ c₁ c₂ : { c : Move // GaugeNormal cands c }, c₁ = c₂ := by
  intro c₁ c₂
  apply Subtype.ext
  have h₁ : gaugeFix cands = some c₁.val := c₁.property
  have h₂ : gaugeFix cands = some c₂.val := c₂.property
  exact Option.some.inj (h₁.symm.trans h₂)

/-! ════════════════════════════════════════════════════════════════════════
  ## §7 Quot.lift：完整结构规范形式在商集 Move/≈ₙ 上良定义（单向，非双射）

  标准要求：分类不变量经 Quotient.lift 提升到商集。分类不变量取 **完整结构规范形式**
  `flattenConnect`（保 Direction/centers/level，不是摘要）——它在 ≈ₙ 下不变（≈ₙ 定义即
  flattenConnect 相等），故良定义条件 zero-effort。
  ★诚实标（codex 防坑）：这是 **单向良定义不变量**，不是 X/≈ₙ≅P 双射。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★完整结构规范形式不变量：分解树 → connect-扁平规范形式（保 Direction/centers/level）。
  这是 ≈ₙ 的 canonical 代表——两分解 ≈ₙ-等价 ⟺ flattenConnect 相同（定义）。
-/
def normalForm (m : Move) : List Move := flattenConnect m

/--
  ★normalForm 在 ≈ₙ 下不变（L0，Quot.lift 良定义条件，定义性成立）：
  t₁ ≈ₙ t₂ ⟹ 同 normalForm。因 ≈ₙ 的定义就是 flattenConnect（=normalForm）相等，
  不变性是定义性的（zero-effort，非额外假设）。这是把完整结构规范形式经 Quotient.lift
  提升到商集 Move/≈ₙ 的充要条件。
-/
theorem normalForm_invariant (t₁ t₂ : Move) (h : t₁ ≈ₙ t₂) :
    normalForm t₁ = normalForm t₂ := h

/--
  ★Quot.lift·normalForm 提升到商集（L0，标准要求「不变量在 Quotient 上良定义」）：
  把完整结构规范形式提升为 `Quotient structEqN_setoid → List Move` 全函数。
  ★诚实：单向良定义不变量（无逆/inj/surj），不是 X/≈ₙ≅P 双射（双射是 T-trend/T-bsp）。
-/
def liftedNormalForm : Quotient structEqN_setoid → List Move :=
  Quotient.lift normalForm normalForm_invariant

/--
  ★Quot.lift 计算律（L0）：liftedNormalForm (mk t) = normalForm t（Quotient.lift β-规约）。
  确认提升忠实——商集上的规范形式 = 任一代表元的规范形式（≈ₙ-不变保证无歧义）。
-/
theorem liftedNormalForm_mk (t : Move) :
    liftedNormalForm (Quotient.mk structEqN_setoid t) = normalForm t := rfl

/--
  ★结合律在商集上归一（L0，Quot.lift + 结合律联合）：(A⊕B)⊕C 与 A⊕(B⊕C) 在商集 Move/≈ₙ
  上是 **同一点**。这把 §2 结合律 + §7 Quot.lift 合一：商集吸收了 connect 括号化自由度
  （不同括号化映到同一商类），但保留中枢/方向差异（§5 反例：不同中枢映到不同商类）。
-/
theorem connect_assoc_in_quotient (A B C : Move) :
    Quotient.mk structEqN_setoid (connect (connect A B) C)
      = Quotient.mk structEqN_setoid (connect A (connect B C)) :=
  Quotient.sound (structEqN_connect_assoc A B C)

/-! ════════════════════════════════════════════════════════════════════════
  ## §8 诚实边界 + 与裁决的对照（formalization-validity-domain）

  - **不强证 `|𝒟ₙ(h)/≈ₙ|=1`**：§5 `decomp_ambiguity_witness` 证 gauge 前商 ≥2（真多义）。
  - **唯一性 = 纤维内 gauge 截面唯一 + 截面在纤维内**（§6 `gauge_section_unique`：带
    ValidDecomp h 前提 + 输出截面 ValidDecomp h；`gaugeFix_section_valid`：截面不逃逸纤维），
    非商唯一——gaugeFix（level 最小+平级最左）从 base h 合法分解纤维内选确定性规范截面。
  - **≈ₙ 是完整结构等价**（保 Direction/centers/level，§2 `structEqN_keeps_center`），
    只归一括号化（§2 `structEqN_connect_assoc`）；摘要核 SummarySimN（旧 leaves）降级为
    GaugeKey/base 投影工具，§5 反例证其退化（同 SummaryI 但 ¬≈ₙ）。
  - **不声称双射**：§7 liftedNormalForm 是单向良定义不变量。

  ★codex 首轮 FAIL 修正（声明膨胀）：§6 之前把「列表选择函数单值」冒充「纤维区间套截面
  唯一」。本版补 `ValidDecomp h` 纤维谓词 + `gaugeFix_section_mem`（截面 ∈ 候选）+
  `gaugeFix_section_valid`（截面在纤维内）——把单值性升级为「纤维内截面唯一且不逃逸」。
  **删除** 膨胀注释（不再声称「区间套规范固定」「所有合法分解 ≈ₙ 归约」——后者即商唯一已被
  §5 否证；前者需完整区间套语义，本文件只给「按 level 最小+平级最左」的确定性纤维内选择）。

  与裁决（/tmp/codex_simn_ruling.md）逐条对照：
  | 裁决要求 | 本文件 |
  |---|---|
  | (b) 完整语义代表，保 Direction/subs/centers | `≈ₙ=flattenConnect 相等`（§2），`structEqN_keeps_center` |
  | 允许结合律括号化归一 | `structEqN_connect_assoc`（§2）+ `connect_assoc_in_quotient`（§7） |
  | gauge 前商 >1，不强证 =1 | `decomp_ambiguity_witness`（§5，同 base 两分解 ¬≈ₙ） |
  | GaugeNormal/gaugeFix 按 level 最小+平级最左 | `gaugeFix`/`minGauge`/`GaugeNormal`（§6） |
  | 截面唯一（非商唯一） | `gauge_section_unique`（纤维内+唯一）/`gaugeFix_section_valid`/`gauge_normal_subsingleton`（§6） |
  | 多义反例 | `decomp_ambiguity_witness`（§5） |
  | 旧 I=leaves 降级 SummarySimN | `SummaryI`/`SummarySimN`（§3） |

  认识论等级：全文 L0（Move/List/Quotient 结构推导，零数据依赖）。
  ════════════════════════════════════════════════════════════════════════ -/

end Strict.Decomp
