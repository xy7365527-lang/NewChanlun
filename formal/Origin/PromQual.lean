/-
Origin/PromQual.lean — 提升谓词 Qual_* + 提升算子 Prom_ℓ（spec §16 假设2：全定义且单值）

── 存在论位置 ───────────────────────────────────────────────────────────────
信源：spec `2026-06-28-recursive-complete-classification-bsp-pdf-extract.md`
  §3（P1–P3，line 123–177）：Qual_*(e_1,…,e_m)∈{0,1} 提升谓词 + Prom_ℓ(e_1,…,e_m)=E∈C_{ℓ+1}
    提升算子 + 自相似交换律 𝒩_{ℓ+1}Prom_ℓ = Prom_*(𝒩_ℓe_1,…,𝒩_ℓe_m) + ℓ_max(t+1)=ℓ_max(t)+1。
  §2（P2，line 95–113）：𝒩_ℓ:C_ℓ→C_* 级别归一化算子 + 统一算子 F_* + 交换律
    𝒩_{ℓ+1}∘F_ℓ = F_*∘𝒩_ℓ（line 103）。本文件 F_ℓ = Prom_ℓ（ℓ→ℓ+1 生成规则）。
  §4（P3–P4，line 209–225）：边界胚元 ∂_{ℓ+1}，前沿 Qual_*=1 时胚元→已确认元素由同一 Prom 完成。
  §16 假设2（P12–P13，line 770–781）：「更高级别提升规则 Prom_* 全定义且单值」。
    ★本文件目标：把假设2从「假设」变成「已证引理」。

── 复用既有对象（不另造平行定义，现场勘查依据）────────────────────────────────
  Prom_ℓ / Qual_* 建立在 `Formal.RecursiveConstruction` 的走势初代数 μF 之上：
  · `Move`（μF）：`segment d lo hi` | `compose subs centers level`（走势分解定理二，#89）。
  · `WellFormed`：良构 = subs.length≥3 ∧ level≥1 ∧ 子级别递减 ∧ centers≥1 ∧ CentersDerivedFrom ∧ 递归。
  · `windowCenters` / `deriveWindowCenter3` / `WindowOverlap` / `CenterDerivedAt`：三段窗口真派生中枢
    （核心+外缘 sound）。本文件 Qual_* 的「≥3 子走势 + 首窗成中枢」与 rust `compose_level`
    （`min_parts_per_level=3` + 三段窗口 `LeveledMove::compose`，recursive_tower.rs:180）**同口径**。
  · `classifyMove` / `outcome?`：走势裁决（盘整/趋势/升父级）。
  自相似交换律接 `SelfSimilarity.lean` 的级别平移精神（S_k），本文件给 Move 层的级别平移 𝒩=`shiftMove`。

── Prom_ℓ / Qual_* 的精确形式化 ──────────────────────────────────────────────
  `Qual lvl es`（提升谓词）⟺ es 至少三段（a::b::c::rest）∧ 首窗三段区间公共重叠 WindowOverlap a b c
    （成中枢，缠论「至少三个次级别走势形成更高级别结构」，spec line 133）∧ 所有 es 级别 = lvl
    （级别一致）∧ 所有 es 良构 WellFormed（递归）。Qual=1 即此 Prop 成立。
  `promote lvl es`（提升算子 Prom_ℓ）= `Move.compose es (windowCenters es) (lvl+1)`——把 m 段次级别
    走势 es 封装为级别 lvl+1 的本级别走势 E，中枢由首窗 `windowCenters` 真派生。**全函数**
    （Foundation 提升需要），有效域（产物良构）⊊ 定义域，由 total 定理前提 Qual 显式标注（231号）。

── 全定义（total）与单值（single-valued）的真内容（反声明膨胀，硬约束）────────────
  ★total 真内容（非「返回某个值」）：`promote_wellFormed` 证 Qual=1 ⟹ 产物是**良构 C_{ℓ+1} 元素**：
    级别 = lvl+1（`promote_level`）+ 方向/裁决确定（`promote_outcome`：首窗单中枢 ⟹ 盘整裁决）
    + 包含关系（`promote_center_contains`：派生中枢 dd≤zd<zg≤gg，外缘含核心）+ WellFormed 全六约束。
    覆盖所有 Qual=1 情形（`promote_wellFormed` 对**任意** es 全称，无遗漏分支——qual_cons 把
    Qual 的 length≥3 结构暴露，短列表分支由 Qual=False 关死）。
  ★single-valued 真内容（诚实分层，反声明膨胀，codex 异质审查结论）：
    (a) by-construction 函数性 `promote_function_unique`（`∃! E, E = promote lvl es`）：promote 是
        全函数 ⟹ 每个 (lvl,es) 有唯一像。**L0 零信息增量**（函数应用确定性），是「单值」在本构造下
        成立的**唯一**真实意义。★**结构唯一性是假的**：`derivation_not_unique` 证 sound 派生
        （CentersDerivedFrom）非唯一（[] 与 windowCenters es 都 sound 却不等）——曾误用 `IsLiftOf`
        钉死 `centers = windowCenters es` 把函数性包装为结构定理 `lift_single_valued`（同义反复），已删。
    (b) 自相似交换律 `promote_shift_commute`（★核心，spec line 103/151）：
        `𝒩_{ℓ+1}(Prom_ℓ es) = Prom_*(𝒩_ℓ es)`，即 `shiftMove k (promote lvl es) =
        promote (lvl+k) (es.map (shiftMove k))`——平移不变。非平凡：依赖 `windowCenters_shift`
        （𝒩 保 lo/hi ⟹ 派生中枢不变），证「高一级结构不是特殊生成，是同一 Prom_* 在更高尺度再应用」。

── 去根化（spec §4，边界胚元）──────────────────────────────────────────────────
  `promote` 是**级别多态全函数**——无「根级别」特例分支（不 `if lvl=ℓ_max then 特殊`）。
  自相似交换律本身即去根化见证：lvl+k 处提升 = lvl 处提升再平移 k，所有级别同一规则。
  前沿胚元 ∂→已确认由同一 `promote` 完成（spec line 213）：无独立分支，`promote` 对前沿/非前沿无区别。

── 认识论等级（formalization-validity-domain 231号，强制）──────────────────────
  全部 **L0**（纯定义/代数/逻辑推导，不依赖市场数据）。信息增量 = 管线正确性（Prom/Qual 结构构造
  自洽），**非**经验有效性——不声明任何 alpha / 第二类识别在真实行情有效（那是 L2+）。
  total/single-valued 的「有效」指**结构良构 + 平移不变**，不指「提升规则在真实多级别行情上经验成立」。

── 纯 core 硬约束 ──────────────────────────────────────────────────────────────
  无 Mathlib/Batteries/Std；禁 Fintype/Finset/Fintype.card；禁 sorry/admit/axiom/native_decide。
  纯 List + decide（具体见证）。不编辑 lakefile（root 名 `Origin.PromQual`，Lead 注册）。
  不碰其他工位文件——只 import `Formal.RecursiveConstruction`（只读）。

── 结果包六要素（完整版，涉及概念定义）──────────────────────────────────────────
  见文件尾 §7。
-/

import Formal.RecursiveConstruction

namespace NewChanlun.Origin.PromQual

open Formal.RecursiveConstruction
open Formal.CenterTrichotomy (Center MoveOutcome)
open Formal.TrendTrichotomy (Direction)

/-! ════════════════════════════════════════════════════════════════════════
    ## §1 提升谓词 Qual_* 与提升算子 Prom_ℓ（spec §3）
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★提升谓词 `Qual_*`（spec §3，line 131/133，L0）—— m 个低一级元素能否生成一个高一级元素。

  `Qual lvl es` ⟺ es 形如 `a :: b :: c :: rest`（至少三段，走势分解定理二 m≥3，与 rust
  `min_parts_per_level=3` 同口径）∧ 首窗三段区间公共重叠 `WindowOverlap a b c`（成中枢——
  缠论「至少三个次级别走势形成更高级别结构」spec line 133）∧ 所有 es 级别 = lvl（级别一致，
  级别递减前提）∧ 所有 es 良构（递归 WellFormed）。其余形状（长度 <3）⟹ False（不可提升，Qual=0）。

  Qual_*∈{0,1}：这里 Prop 取值即 {0,1}（成立=1 / 不成立=0）。
-/
def Qual (lvl : Nat) (es : List Move) : Prop :=
  match es with
  | a :: b :: c :: rest =>
      WindowOverlap a b c
      ∧ (∀ m ∈ (a :: b :: c :: rest), Move.level m = lvl)
      ∧ (∀ m ∈ (a :: b :: c :: rest), WellFormed m)
  | _ => False

/--
  ★提升算子 `Prom_ℓ`（spec §3，line 147，L0）—— Qual_*=1 时把 ℓ 级完成元素串 es 提升为 ℓ+1 级元素 E。

  `promote lvl es = Move.compose es (windowCenters es) (lvl+1)`：m 段次级别走势 es 整体封装为级别
  lvl+1 的本级别走势 E，中枢由首窗 `windowCenters`（三段公共重叠 ⟹ 单中枢，否则空）真派生。

  **全函数**（定义域 = 全部 `List Move`，Foundation `Fstep` / spec 提升需要）。有效域（产物良构
  C_{ℓ+1} 元素）⊊ 定义域，由 `promote_wellFormed` 的前提 `Qual` 显式标注（231号有效域 ⊊ 定义域）。
  E 的分解 = es 本身（输入逐段保留，无重排/复制），级别 = lvl+1（spec line 161 ℓ_max+1）。
-/
def promote (lvl : Nat) (es : List Move) : Move :=
  Move.compose es (windowCenters es) (lvl + 1)

/-! ════════════════════════════════════════════════════════════════════════
    ## §2 辅助引理（窗口中枢派生 + Qual 结构暴露）
    ════════════════════════════════════════════════════════════════════════ -/

/-- ★首窗成中枢 ⟹ windowCenters 取回单中枢（L0）：`windowCenters (a::b::c::rest) =
    [deriveWindowCenter3 a b c h]`（dif_pos h，重锚 windowCenters 定义的 then 分支）。 -/
theorem windowCenters_of_firstOverlap {a b c : Move} {rest : List Move}
    (h : WindowOverlap a b c) :
    windowCenters (a :: b :: c :: rest) = [deriveWindowCenter3 a b c h] := by
  show (if h' : WindowOverlap a b c then [deriveWindowCenter3 a b c h'] else []) = _
  rw [dif_pos h]

/-- ★三段窗口派生中枢满足 `CenterDerivedAt`（cons 版，L0，反退化忠实性）：`deriveWindowCenter3 a b c h`
    在序列 `a::b::c::rest` 起点 0 处核心+外缘全 sound（前三段 a/b/c 的真实 lo/hi 边界算出，rest 不干扰）。
    泛化既有 `deriveWindowCenter3_derivedAt`（仅 [a,b,c]）到带尾列表——total 的 CentersDerivedFrom 需要。 -/
theorem deriveWindowCenter3_derivedAt_cons (a b c : Move) (rest : List Move)
    (h : WindowOverlap a b c) :
    CenterDerivedAt (a :: b :: c :: rest) (deriveWindowCenter3 a b c h) 0 := by
  unfold CenterDerivedAt deriveWindowCenter3
  refine ⟨by simp only [List.length_cons]; omega, ?_⟩
  intro _ _ _
  refine ⟨?_, ?_, ?_, ?_⟩ <;> simp [Move.lo, Move.hi]

/-- ★Qual 结构暴露（L0）：`Qual lvl es` ⟹ es 形如 a::b::c::rest 且首窗重叠 + 级别一致 + 全良构。
    短列表分支由 `Qual = False` 关死（Lean 空类型消去自动 discharge）——这把 Qual 的 length≥3
    结构对下游证明暴露（total 无遗漏分支的依据）。 -/
theorem qual_cons {lvl : Nat} {es : List Move} (hq : Qual lvl es) :
    ∃ a b c rest, es = a :: b :: c :: rest ∧ WindowOverlap a b c
      ∧ (∀ m ∈ (a :: b :: c :: rest), Move.level m = lvl)
      ∧ (∀ m ∈ (a :: b :: c :: rest), WellFormed m) := by
  cases es with
  | nil => nomatch hq
  | cons a t1 => cases t1 with
    | nil => nomatch hq
    | cons b t2 => cases t2 with
      | nil => nomatch hq
      | cons c rest =>
        obtain ⟨hov, hlv, hwf⟩ := hq
        exact ⟨a, b, c, rest, rfl, hov, hlv, hwf⟩

/-! ════════════════════════════════════════════════════════════════════════
    ## §3 全定义（total）：Qual_*=1 ⟹ Prom 产出良构 C_{ℓ+1} 元素（spec §16 假设2 前半）

    total 真内容：不是「返回某个值」，而是返回满足**级别 + 方向/裁决 + 包含关系**的良构元素，
    且覆盖所有 Qual=1 情形（无遗漏分支）。
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★全定义主定理（L0，spec §16 假设2 前半「Prom_* 全定义」）：Qual_*=1 ⟹ `promote lvl es`
  是**良构** C_{ℓ+1} 元素（`WellFormed`）。

  非平凡（反 total_unique_of_fun 退化）：不是「promote 返回某 Move」（恒真），而是证产物满足
  WellFormed 全六约束——subs.length≥3（Qual 给 es=a::b::c::rest）、级别 lvl+1≥1、子级别递减
  （∀m∈es,level=lvl=(lvl+1)-1，Qual 给级别一致）、中枢非空（首窗派生单中枢）、CentersDerivedFrom
  （首窗中枢 sound witness）、子走势递归良构（Qual 给）。覆盖**任意** es 的 Qual=1 情形（全称 es）。
-/
theorem promote_wellFormed (lvl : Nat) (es : List Move) (hq : Qual lvl es) :
    WellFormed (promote lvl es) := by
  obtain ⟨a, b, c, rest, rfl, hov, hlevel, hwf⟩ := qual_cons hq
  unfold promote
  rw [windowCenters_of_firstOverlap hov]
  unfold WellFormed
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_⟩
  · -- subs.length ≥ 3
    simp only [List.length_cons]; omega
  · -- lvl + 1 ≥ 1
    omega
  · -- ∀ m ∈ subs, m.level = (lvl+1) - 1 = lvl（级别递减）
    intro m hm; rw [Nat.add_sub_cancel]; exact hlevel m hm
  · -- centers.length ≥ 1（首窗单中枢）
    simp
  · -- CentersDerivedFrom subs [c0]（首窗中枢 sound witness，starts=[0]）
    exact ⟨List.cons_ne_nil _ _, [0], rfl, True.intro,
      ⟨deriveWindowCenter3_derivedAt_cons a b c rest hov, True.intro⟩⟩
  · -- ∀ m ∈ subs, WellFormed m（递归良构）
    exact hwf

/-- ★产物级别 = lvl+1（L0，spec line 161 `E∈C_{ℓ_max+1}` / line 165 `ℓ_max(t+1)=ℓ_max(t)+1`）。
    rfl——提升使级别恰增 1（前沿提升时 ℓ_max 增 1 的结构基础）。 -/
theorem promote_level (lvl : Nat) (es : List Move) : (promote lvl es).level = lvl + 1 := rfl

/-- ★前沿提升 ℓ_max 增 1（L0，spec line 157–165 方框）：前沿 Qual_*=1 时，产物在 ℓmax+1。
    `_hq` 是 spec 的前沿前件（line 157「Qual_*=1 在最高级别处成立」），**有意未用**——因
    `promote_level` 无条件给级别 lvl+1（提升总产高一级），前件只标注前沿语境，不参与证明。 -/
theorem lmax_increment (ℓmax : Nat) (es : List Move) (_hq : Qual ℓmax es) :
    (promote ℓmax es).level = ℓmax + 1 := promote_level ℓmax es

/-- ★产物裁决确定（方向，L0）：Qual=1 ⟹ `outcome? (promote lvl es) = some consolidation`。
    首窗单中枢 ⟹ `classifyMove [c0] = consolidation`（盘整裁决，走势分解定理二最小高级别结构）。
    这是 total 的「方向」分量——产物有**确定的走势裁决**，不是无裁决的退化值。 -/
theorem promote_outcome (lvl : Nat) (es : List Move) (hq : Qual lvl es) :
    outcome? (promote lvl es) = some MoveOutcome.consolidation := by
  obtain ⟨a, b, c, rest, rfl, hov, _, _⟩ := qual_cons hq
  unfold promote
  rw [windowCenters_of_firstOverlap hov]
  rfl

/-- ★产物包含关系（L0）：Qual=1 ⟹ 产物中枢满足 `dd ≤ zd < zg ≤ gg`（外缘含核心 + 核心非退化）。
    这是 total 的「包含关系」分量——高级别元素的中枢结构满足区间套包含不变量（reference §中枢）。 -/
theorem promote_center_contains (lvl : Nat) (es : List Move) (hq : Qual lvl es) :
    ∃ ctr : Center, windowCenters es = [ctr]
      ∧ ctr.dd ≤ ctr.zd ∧ ctr.zd < ctr.zg ∧ ctr.zg ≤ ctr.gg := by
  obtain ⟨a, b, c, rest, rfl, hov, _, _⟩ := qual_cons hq
  exact ⟨deriveWindowCenter3 a b c hov, windowCenters_of_firstOverlap hov,
    (deriveWindowCenter3 a b c hov).outer_lo,
    (deriveWindowCenter3 a b c hov).core_valid,
    (deriveWindowCenter3 a b c hov).outer_hi⟩

/-! ════════════════════════════════════════════════════════════════════════
    ## §4 单值（single-valued）之一：promote 函数性（by-construction）+ sound 派生非唯一（结构唯一性证伪）

    ★诚实分层（反声明膨胀，codex 异质审查结论）：promote 的单值性是**纯 by-construction 函数性**
    （`promote_function_unique : ∃!E,E=promote`，L0 零增量），**非结构唯一性**。曾误用 `IsLiftOf`
    （钉死 centers=windowCenters）把函数性包装为结构定理 `lift_single_valued`（同义反复），已删。
    **结构唯一性是假的**：`derivation_not_unique`（§7）证 sound 派生（CentersDerivedFrom）允许多个
    不同 centers（如 [] 与 windowCenters es 都 sound）——单值性仅由 canonical 首窗选择给出。
    ════════════════════════════════════════════════════════════════════════ -/

/-- ★提升忠实保输入（L0，反退化，**定义层展开 rfl**）：`promote lvl es = compose es
    (windowCenters es) (lvl+1)`——产物的**直接分解恰是输入 es**（逐段、原序、无重排、无复制）。
    这排除 codex R6 退化 `compose [m,m,m]`（同一走势复制三份充数）——本提升的 subs 是真实 m 段输入。
    诚实标注：这是读 `promote` 定义的展开（rfl 级），陈述 subs=输入，**不**主张任何 uniqueness。 -/
theorem promote_decomposes_to_input (lvl : Nat) (es : List Move) :
    promote lvl es = Move.compose es (windowCenters es) (lvl + 1) := rfl

/--
  ★单值（**by-construction 函数性**，L0，spec §16 假设2「Prom_* 单值」的诚实形式）：
  `promote` 是全函数 ⟹ 每个 (lvl, es) 有**唯一**像 E（`∃! E, E = promote lvl es`）。

  ★诚实分层（formalization-validity-domain，反声明膨胀）：这是**纯 by-construction 函数性**
  （L0 零信息增量 = 函数应用确定性），**不是**结构唯一性。即「单值」在本构造下成立的唯一真实
  意义就是「promote 是函数」。曾误用 `IsLiftOf`（钉死 `centers = windowCenters es`）把此函数性
  包装成命名「结构唯一性定理」`lift_single_valued`——codex 异质审查判定为声明膨胀（同义反复钉死
  存在量词），已删除。**结构唯一性是假的**：见 `derivation_not_unique`（§7，sound 派生非唯一）。
-/
theorem promote_function_unique (lvl : Nat) (es : List Move) :
    ∃ E : Move, E = promote lvl es ∧ ∀ E' : Move, E' = promote lvl es → E' = E :=
  ⟨promote lvl es, rfl, fun _ hy => hy⟩

/-! ════════════════════════════════════════════════════════════════════════
    ## §5 单值（single-valued）之二：自相似交换律 𝒩_{ℓ+1}Prom_ℓ = Prom_*(𝒩_ℓ…)（★核心）

    spec §2 line 103 `𝒩_{ℓ+1}∘F_ℓ = F_*∘𝒩_ℓ` + §3 line 151。F_ℓ = Prom_ℓ。
    𝒩 = 级别平移 `shiftMove`（Move 层 S_k，接 SelfSimilarity.lean 平移精神）：compose 级别 +k，
    segment（μF 递归底，无级别）平移不变。交换律 = 提升与级别平移可交换 = 平移不变 = 自相似。
    ════════════════════════════════════════════════════════════════════════ -/

/-
  ★级别平移算子 𝒩 = `shiftMove k`（Move 层 S_k，L0）—— compose 节点级别整体 +k，segment（μF
  递归底，无级别维度）平移不变。镜像 SelfSimilarity.lean 的 S_k（Cand 层 lvl+k）于走势 μF 层。
  互递归（nested List Move：`shiftMoveList` 处理 subs 列表）——core Lean 4 结构互递归，无 Mathlib。
  （doc 注释下移到各 def 上，因 `mutual` 不可直接挂 doc-comment。）
-/
mutual
  /-- ★级别平移 𝒩=S_k 施于单走势：compose 级别 +k（子走势逐个平移），segment（递归底）不变。 -/
  def shiftMove (k : Nat) : Move → Move
    | Move.segment d lo hi => Move.segment d lo hi
    | Move.compose subs centers level => Move.compose (shiftMoveList k subs) centers (level + k)
  def shiftMoveList (k : Nat) : List Move → List Move
    | [] => []
    | m :: ms => shiftMove k m :: shiftMoveList k ms
end

/-- ★`shiftMoveList = List.map (shiftMove k)`（L0，桥接互递归与 map）。 -/
theorem shiftMoveList_eq_map (k : Nat) (xs : List Move) :
    shiftMoveList k xs = xs.map (shiftMove k) := by
  induction xs with
  | nil => rfl
  | cons x xs ih => simp only [shiftMoveList, List.map_cons, ih]

/-- ★shiftMove 对 compose 的作用（L0）：`shiftMove k (compose subs centers level) =
    compose (subs.map (shiftMove k)) centers (level+k)`——级别 +k，子走势逐个平移，中枢不变。 -/
theorem shiftMove_compose (k : Nat) (subs : List Move) (centers : List Center) (level : Nat) :
    shiftMove k (Move.compose subs centers level)
      = Move.compose (subs.map (shiftMove k)) centers (level + k) := by
  simp only [shiftMove, shiftMoveList_eq_map]

/-- ★平移保价格区间（L0）：`(shiftMove k m).interval = m.interval`——shiftMove 只改级别（compose）
    或不改（segment），interval 由 segment 直接字段 / compose 中枢外缘算（中枢不变）⟹ 区间不变。 -/
theorem shiftMove_interval (k : Nat) (m : Move) : (shiftMove k m).interval = m.interval := by
  cases m with
  | segment d lo hi => simp only [shiftMove]
  | compose subs centers level => simp only [shiftMove, Move.interval]

/-- ★平移保下沿（L0，windowCenters_shift 用）。 -/
theorem shiftMove_lo (k : Nat) (m : Move) : (shiftMove k m).lo = m.lo := by
  unfold Move.lo; rw [shiftMove_interval]

/-- ★平移保上沿（L0，windowCenters_shift 用）。 -/
theorem shiftMove_hi (k : Nat) (m : Move) : (shiftMove k m).hi = m.hi := by
  unfold Move.hi; rw [shiftMove_interval]

/-- ★windowCenters 仅依赖首三段 lo/hi 的同余（L0）：首三段 lo/hi 对应相等 ⟹ windowCenters 相等
    （WindowOverlap 与 deriveWindowCenter3 全由 lo/hi 决定，rest 不影响）。 -/
theorem center_eq {c1 c2 : Center} (hdd : c1.dd = c2.dd) (hzd : c1.zd = c2.zd)
    (hzg : c1.zg = c2.zg) (hgg : c1.gg = c2.gg) : c1 = c2 := by
  obtain ⟨dd1, zd1, zg1, gg1, _, _, _⟩ := c1
  obtain ⟨dd2, zd2, zg2, gg2, _, _, _⟩ := c2
  subst hdd; subst hzd; subst hzg; subst hgg
  rfl

theorem windowCenters_congr_lohi {a b c a' b' c' : Move} {rest rest' : List Move}
    (ha : a.lo = a'.lo) (ha2 : a.hi = a'.hi) (hb : b.lo = b'.lo) (hb2 : b.hi = b'.hi)
    (hc : c.lo = c'.lo) (hc2 : c.hi = c'.hi) :
    windowCenters (a :: b :: c :: rest) = windowCenters (a' :: b' :: c' :: rest') := by
  show (if h : WindowOverlap a b c then [deriveWindowCenter3 a b c h] else [])
     = (if h : WindowOverlap a' b' c' then [deriveWindowCenter3 a' b' c' h] else [])
  have hiff : WindowOverlap a b c ↔ WindowOverlap a' b' c' := by
    unfold WindowOverlap; rw [ha, ha2, hb, hb2, hc, hc2]
  by_cases h : WindowOverlap a b c
  · rw [dif_pos h, dif_pos (hiff.mp h)]
    congr 1
    apply center_eq
    · show min a.lo (min b.lo c.lo) = min a'.lo (min b'.lo c'.lo); rw [ha, hb, hc]
    · show max a.lo (max b.lo c.lo) = max a'.lo (max b'.lo c'.lo); rw [ha, hb, hc]
    · show min a.hi (min b.hi c.hi) = min a'.hi (min b'.hi c'.hi); rw [ha2, hb2, hc2]
    · show max a.hi (max b.hi c.hi) = max a'.hi (max b'.hi c'.hi); rw [ha2, hb2, hc2]
  · rw [dif_neg h, dif_neg (fun hp => h (hiff.mpr hp))]

/-- ★windowCenters 平移不变（L0，自相似交换律核心引理）：`windowCenters (es.map (shiftMove k)) =
    windowCenters es`——𝒩 保 lo/hi（shiftMove_lo/hi）⟹ 派生中枢不变。这坐实「同一中枢规则在
    平移后的次级别序列上产同一中枢」。 -/
theorem windowCenters_shift (k : Nat) (es : List Move) :
    windowCenters (es.map (shiftMove k)) = windowCenters es := by
  match es with
  | a :: b :: c :: rest =>
      simp only [List.map_cons]
      exact windowCenters_congr_lohi
        (shiftMove_lo k a) (shiftMove_hi k a) (shiftMove_lo k b) (shiftMove_hi k b)
        (shiftMove_lo k c) (shiftMove_hi k c)
  | [] => rfl
  | [_] => rfl
  | [_, _] => rfl

/--
  ★★自相似交换律（L0，spec §2 line 103 `𝒩_{ℓ+1}∘F_ℓ = F_*∘𝒩_ℓ` + §3 line 151，★核心单值内容）：

    `shiftMove k (promote lvl es) = promote (lvl+k) (es.map (shiftMove k))`

  即 `𝒩_{ℓ+1}(Prom_ℓ es) = Prom_*(𝒩_ℓ e_1,…,𝒩_ℓ e_m)`——提升与级别平移**可交换**（平移不变）。
  这是「高一级结构不是特殊生成，而是同一个 Prom_* 在更高尺度上的再应用」（spec line 153）的机器证明：
  Prom_* = 同一 `promote`（级别多态），lvl+k 处提升 = lvl 处提升再平移 k。

  非平凡：依赖 `windowCenters_shift`（𝒩 保 lo/hi ⟹ 派生中枢不变）+ shiftMove_compose（级别 +k 分配
  到 subs）+ 级别算术 (lvl+1)+k = (lvl+k)+1。**无条件成立**（对任意 k/lvl/es），即去根化——所有
  级别同一规则，无特殊根（spec §4）。
-/
theorem promote_shift_commute (k lvl : Nat) (es : List Move) :
    shiftMove k (promote lvl es) = promote (lvl + k) (es.map (shiftMove k)) := by
  unfold promote
  rw [shiftMove_compose, windowCenters_shift]
  have hlv : lvl + 1 + k = lvl + k + 1 := by omega
  rw [hlv]

/-! ════════════════════════════════════════════════════════════════════════
    ## §6 capstone：discharge spec §16 假设2「Prom_* 全定义且单值」
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★★★capstone（L0，spec §16 假设2 完整 discharge）：「更高级别提升规则 Prom_* 全定义且单值」
  从「假设」变为「已证引理」。三件齐备：

  1. **全定义（total）**：∀ lvl es, Qual=1 ⟹ promote 产**良构 C_{ℓ+1} 元素**（WellFormed ∧ 级别 lvl+1
     ∧ 裁决确定为盘整 ∧ 中枢含包含关系）——覆盖所有 Qual=1 情形。
  2. **单值（single-valued, by-construction）**：∀ lvl es, promote 有唯一像（∃!E,E=promote）——
     L0 函数性（函数应用确定性）。**非**结构唯一性（结构唯一性已由 `derivation_not_unique` 证伪）。
  3. **自相似（平移不变）**：∀ k lvl es, 𝒩_{ℓ+1}Prom_ℓ = Prom_*(𝒩_ℓ…)（spec line 103/151）。

  这三件即 spec §16 唯一性定理对假设2 的全部要求——Prom_* 全定义（1）且单值（2），自相似（3）
  保证「递归结构及可能涌现的更高级别唯一」（spec line 799）。
-/
theorem prom_total_and_single_valued :
    -- 1. 全定义（total）：Qual=1 ⟹ 良构 C_{ℓ+1} 元素（级别 + 方向 + 包含 + WellFormed）
    (∀ (lvl : Nat) (es : List Move), Qual lvl es →
        WellFormed (promote lvl es)
        ∧ (promote lvl es).level = lvl + 1
        ∧ outcome? (promote lvl es) = some MoveOutcome.consolidation
        ∧ (∃ ctr : Center, windowCenters es = [ctr]
            ∧ ctr.dd ≤ ctr.zd ∧ ctr.zd < ctr.zg ∧ ctr.zg ≤ ctr.gg))
    -- 2. 单值（single-valued, by-construction 函数性，非结构唯一性）：promote 有唯一像
    ∧ (∀ (lvl : Nat) (es : List Move),
        ∃ E : Move, E = promote lvl es ∧ ∀ E' : Move, E' = promote lvl es → E' = E)
    -- 3. 自相似交换律（平移不变）：𝒩_{ℓ+1}Prom_ℓ = Prom_*(𝒩_ℓ…)
    ∧ (∀ (k lvl : Nat) (es : List Move),
        shiftMove k (promote lvl es) = promote (lvl + k) (es.map (shiftMove k))) := by
  refine ⟨?_, promote_function_unique, promote_shift_commute⟩
  intro lvl es hq
  exact ⟨promote_wellFormed lvl es hq, promote_level lvl es,
    promote_outcome lvl es hq, promote_center_contains lvl es hq⟩

/-! ════════════════════════════════════════════════════════════════════════
    ## §7 反退化见证（具体 Qual=1 提升真跑通 + 负向：Qual 是真约束，非恒真）
    ════════════════════════════════════════════════════════════════════════ -/

/-- 见证次级别走势 1（线段 [0,10]，level 0）。 -/
def segWit1 : Move := Move.segment Direction.up 0 10
/-- 见证次级别走势 2（线段 [2,12]，level 0）。 -/
def segWit2 : Move := Move.segment Direction.down 2 12
/-- 见证次级别走势 3（线段 [3,11]，level 0）。 -/
def segWit3 : Move := Move.segment Direction.up 3 11
/-- 见证次级别走势序列（三段重叠：max lo=3 < min hi=10 ⟹ 成中枢）。 -/
def esWit : List Move := [segWit1, segWit2, segWit3]

/-- ★反退化见证：具体 es 满足 Qual=1（L0→L1 提升前件成立：三段重叠 + 级别 0 + 良构）。 -/
theorem witness_qual : Qual 0 esWit := by
  refine ⟨?_, ?_, ?_⟩
  · -- WindowOverlap segWit1 segWit2 segWit3：max 0 (max 2 3) < min 10 (min 12 11)
    show WindowOverlap segWit1 segWit2 segWit3
    unfold WindowOverlap segWit1 segWit2 segWit3 Move.lo Move.hi Move.interval
    decide
  · -- ∀ m ∈ esWit, level = 0（线段级别 0）
    intro m hm
    simp only [List.mem_cons, List.not_mem_nil, or_false] at hm
    rcases hm with rfl | rfl | rfl <;> rfl
  · -- ∀ m ∈ esWit, WellFormed m（线段 WellFormed = True）
    intro m hm
    simp only [List.mem_cons, List.not_mem_nil, or_false] at hm
    rcases hm with rfl | rfl | rfl
    · simp only [segWit1, WellFormed]
    · simp only [segWit2, WellFormed]
    · simp only [segWit3, WellFormed]

/-- ★反退化见证：Qual=1 ⟹ 提升产良构 L1 元素（total 真跑通）。 -/
theorem witness_promote_wellFormed : WellFormed (promote 0 esWit) :=
  promote_wellFormed 0 esWit witness_qual

/--
  ★★诚实负向结果（L0，反结构唯一性，formalization-validity-domain：否定性结果缩小有效域边界）：
  sound 派生 `CentersDerivedFrom` **非唯一**——同一 `esWit` 的两个不同 centers 列表
  （`windowCenters esWit`=首窗单中枢 与 `[]`=空）**都**满足派生 soundness，但不相等。

  这直接证伪「结构唯一性」读法（codex 质询：原 `lift_single_valued` 对「非 canonical centers
  是否唯一」沉默）：promote 的单值性**仅**由 canonical 首窗选择 `windowCenters` 给出，**非**结构性
  ——若把 `IsLiftOf` 的 `centers` 放开为「任意 sound 派生 `CentersDerivedFrom es centers`」，提升
  关系是**多值**的（[] 与 windowCenters es 都是合法见证）。故原 `lift_single_valued`（钉死
  canonical）的「结构唯一性」外观是声明膨胀；真相是 by-construction 函数性（`promote_function_unique`）。

  ★有效域收窄（231号有效域⊊定义域，codex 异质审查裁断）：本定理证伪的是**裸 `CentersDerivedFrom`
  派生唯一性**——`cs2=[]` 对 `StrictlyIncreasing`/`PairwiseRel` 是 vacuous 满足（空 starts 平凡成立），
  故 `[]` 与 `windowCenters esWit` 都满足裸 `CentersDerivedFrom` 却不等。这**没有**证伪「WellFormed
  compose 的 centers 唯一性」：`WellFormed`（RecursiveConstruction.lean:160）有 `centers.length≥1`
  约束，`cs2=[]` 不满足 WellFormed，被排除在 WellFormed compose 的有效域之外。即 `CentersDerivedFrom`
  的定义域（全部 centers 列表）上的派生非唯一性，在 WellFormed 限定的有效域（`centers.length≥1`）内
  **未被本见证否证**——WellFormed compose centers 唯一性仍为开放问题，本定理不对其作断言。

  - `windowCenters esWit` sound：复用 total 已证 `WellFormed (promote 0 esWit)` ⟹ CentersDerivedFrom。
  - `[]` sound：空 starts witness 平凡满足（`StrictlyIncreasing [] = PairwiseRel _ [] [] = True`），subs 非空。
-/
theorem derivation_not_unique :
    ∃ (es : List Move) (cs1 cs2 : List Center),
      CentersDerivedFrom es cs1 ∧ CentersDerivedFrom es cs2 ∧ cs1 ≠ cs2 := by
  have hov : WindowOverlap segWit1 segWit2 segWit3 := by
    unfold WindowOverlap segWit1 segWit2 segWit3 Move.lo Move.hi Move.interval; decide
  have hwc : windowCenters esWit = [deriveWindowCenter3 segWit1 segWit2 segWit3 hov] :=
    windowCenters_of_firstOverlap hov
  refine ⟨esWit, windowCenters esWit, [], ?_, ?_, ?_⟩
  · -- windowCenters esWit 由首窗 sound 派生（从 total 的 WellFormed 提取 CentersDerivedFrom）
    exact wellformed_centers_derived esWit (windowCenters esWit) 1 witness_promote_wellFormed
  · -- [] 也 sound 派生：空 starts 平凡满足，esWit 非空
    refine ⟨?_, [], rfl, True.intro, True.intro⟩
    unfold esWit; exact List.cons_ne_nil _ _
  · -- 两 sound 派生不等（单中枢 ≠ 空）⟹ 派生非唯一
    rw [hwc]; exact List.cons_ne_nil _ _

/-- ★反退化见证：提升产物级别 = 1（L0→L1，级别恰增 1）。 -/
theorem witness_promote_level : (promote 0 esWit).level = 1 := rfl

/-- ★反退化见证：提升产物裁决 = 盘整（首窗单中枢 ⟹ consolidation）。 -/
theorem witness_promote_outcome : outcome? (promote 0 esWit) = some MoveOutcome.consolidation :=
  promote_outcome 0 esWit witness_qual

/-- ★反退化见证（自相似）：𝒩_1(Prom_0 esWit) = Prom_*(𝒩_0 esWit)（平移交换律具体实例真跑通）。 -/
theorem witness_shift_commute :
    shiftMove 5 (promote 0 esWit) = promote 5 (esWit.map (shiftMove 5)) :=
  promote_shift_commute 5 0 esWit

/-- ★反退化见证（自相似非平凡）：平移真改级别（产物 level 1 → 平移后 6）——交换律非平凡（平移有作用）。 -/
theorem witness_shift_changes_level : (shiftMove 5 (promote 0 esWit)).level = 6 := by
  rw [promote_shift_commute]; rfl

/-- ★反退化见证（负向）：两段不可提升（Qual 要求 ≥3 段，length 2 ⟹ Qual=False，非恒真）。 -/
theorem witness_qual_needs_three : ¬ Qual 0 [segWit1, segWit2] := fun h => h

/-- 不重叠见证段 1（线段 [0,1]）。 -/
def segNoOv1 : Move := Move.segment Direction.up 0 1
/-- 不重叠见证段 2（线段 [5,6]）。 -/
def segNoOv2 : Move := Move.segment Direction.up 5 6
/-- 不重叠见证段 3（线段 [10,11]）。 -/
def segNoOv3 : Move := Move.segment Direction.up 10 11

/-- ★反退化见证（负向，关键）：三段不重叠 ⟹ Qual=False（max lo=10 > min hi=1，首窗不成中枢）。
    这证 Qual 的「首窗重叠」是**真约束**（破之即不可提升）——非恒真占位（反声明膨胀）。 -/
theorem witness_qual_needs_overlap : ¬ Qual 0 [segNoOv1, segNoOv2, segNoOv3] := by
  intro hq
  obtain ⟨hov, _, _⟩ := hq
  revert hov
  unfold WindowOverlap segNoOv1 segNoOv2 segNoOv3 Move.lo Move.hi Move.interval
  decide

/-! ════════════════════════════════════════════════════════════════════════
    ## §8 诚实标签（formalization-validity-domain gatekeeper，L0 声明）
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★Prom/Qual 标签 `PromQualTag`（gatekeeper，诚实分层）：
  - `TotalL0`：全定义是 L0 结构定理（Qual=1 ⟹ 良构 C_{ℓ+1}，不依赖市场数据）。
  - `SingleValuedL0`：单值是 L0 **by-construction 函数性**（promote 全函数 ⟹ 唯一像；
    **非**结构唯一性——结构唯一性已由 `derivation_not_unique` 证伪，单值仅由 canonical 首窗给出）。
  - `SelfSimilarConjugacyL0`：自相似交换律是 L0 平移不变（spec line 103/151）。
  - `CanonicalFirstWindowCenter`：提升中枢取首窗规范派生（边界条件——多中枢趋势型提升见 composeStep）。
  - `EmpiricalValidityOutOfScope`：提升规则的实证有效性是 L2+，不在本层。
  ★**没有** `EmpiricallyValidated` 构造子——类型层拒绝把 L0 结构定理标为经验有效。
-/
inductive PromQualTag where
  | TotalL0
  | SingleValuedL0
  | SelfSimilarConjugacyL0
  | CanonicalFirstWindowCenter
  | EmpiricalValidityOutOfScope
deriving DecidableEq, Repr

/-- ★Prom/Qual 认识论等级 = L0（结构构造，不冒充经验有效性）。 -/
def promQualLevel : PromQualTag := PromQualTag.TotalL0

/-- ★gatekeeper：Prom/Qual 任何标签都不是经验有效（L0 结构性质，非 L2 实证）。 -/
theorem promQual_not_empirical (t : PromQualTag) :
    t = PromQualTag.TotalL0 ∨
    t = PromQualTag.SingleValuedL0 ∨
    t = PromQualTag.SelfSimilarConjugacyL0 ∨
    t = PromQualTag.CanonicalFirstWindowCenter ∨
    t = PromQualTag.EmpiricalValidityOutOfScope := by
  cases t <;> simp

/-! ════════════════════════════════════════════════════════════════════════
    ## §9 公理审计（确认仅 propext/Quot.sound，无 sorryAx/native_decide）
    ════════════════════════════════════════════════════════════════════════ -/

#print axioms promote_wellFormed
#print axioms promote_function_unique
#print axioms derivation_not_unique
#print axioms promote_shift_commute
#print axioms prom_total_and_single_valued
#print axioms witness_qual
#print axioms witness_qual_needs_overlap

end NewChanlun.Origin.PromQual
