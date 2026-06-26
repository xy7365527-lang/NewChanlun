/-
  Strict/BSP.lean — 买卖点完全分类 Layer2 canonical（task #60, C3, 615 概念分离 Layer2）
  ★T-bsp 工位（RTAS 严格完全分类蜂群）

  ════════════════════════════════════════════════════════════════════════
  范式背景（chanlun-strict-classification-standard.md 第一部分 A + codex#1 (c) 段）
  ════════════════════════════════════════════════════════════════════════

  Layer1 + 本地诚实裁定（Formal/BSPLabels.lean，已有，自包含 L0）：
  - (A) 互斥三分粒度失败的诚实裁定：`coarseClass_none_at_coincidence`（重合点无像）/
    `bsp_not_exclusive_trichotomy`（无全函数互斥三分）/ `coincidence_iff`——
    `twoB_threeB_can_coincide`（2B/3B 重合，maimai.md:170）⟹ 把买卖点当**互斥 sum type**
    （type1∣type2∣type3 恰好一格）时失败，不可冒充互斥三分。
  - (B) 诚实对偶：`situationLabels_separates_coincidence`——**非空标签集**粒度**可区分**
    重合点（[2B] ≠ [2B,3B]，无塌缩）。失败专属于互斥 sum type 粒度，非标签集粒度。
  - (C) 第三类本征子域穷尽互斥 + 双射三件套：`third_exhaustive` / `third_exclusive` /
    `third_subdomain_partition` + `thirdInvariant_invariant`/`_complete`/`_realizable`——
    `AfterBreakRetrace` 子域内 IsThirdBuy ∪ IsThirdSell 互斥穷尽且 I' 诱导双射。

  codex#1 硬规则（严格性命名约束）：μF/inductive **不自动**给语义 ∼ 下的双射。
  凡只在 `x ∼ y ↔ I x = I y` 下成立的分类，**必须显式命名为「按标签商分类」**，
  不得冒充 `X/∼ ≅ P`。

  共享内核：import `Strict.Classification`（task #57 T-kernel 工位），复用
  `Strict.Classifies` / `Strict.SemanticQuotient`——不内联、不重复定义
  （no-patch-mentality：消除赘余）。与 sibling `Strict/Trend.lean`（T-trend）同构。

  ★依赖方向（避免 Formal↔Strict 循环依赖，Lake 可构建性要求）：
  本文件（Strict lib）import `Formal.BSPLabels`（域对象）+ `Strict.Classification`（内核），
  依赖方向**单向 Strict→Formal**。Formal/BSPLabels.lean **不** import Strict（否则成环）。

  ════════════════════════════════════════════════════════════════════════
  本模块的 canonical 双层裁定（codex#1 (c) 段双层定理的内核兑现）
  ════════════════════════════════════════════════════════════════════════

  - **Layer2A canonical（全域互斥分类不存在，诚实降级）**：
    (a) `no_global_classifies`：买卖点全域**不存在** `Strict.Classifies` 实例——
        谓词族 {type1↦IsFirst, type2↦IsSecond, type3↦IsThird} 的 `disjoint` 必然失败
        （x_2b3b 同时是 2B 与 3B，type2 ≠ type3）。这是 (A) 在内核类型上的 canonical 形式
        （= 互斥 sum type 三分失败：重合点同占二/三格，没有恰好一格的像）。
    (b) `global_only_label_quotient`：全域分类只能停留在 `Strict.SemanticQuotient` 在
        **标签核** ∼_label（`fun x y => IGlobal x = IGlobal y`）下——即 codex#1 命名规则
        要求显式标注的「按标签商分类」，complete 是同义反复、无独立语义内容，
        **不是**真语义双射 X/∼ ≅ P。

  - **Layer2B canonical（第三类本征子域真双射）**：
    `third_subdomain_classifies`：第三类本征子域（离开中枢后回试/回抽不入中枢）
    实例化 `Strict.Classifies ThirdSubX Side ...`——五项证明义务全证，
    是 codex#1 推荐的诚实子域定理的 canonical 兑现（真完全分类 X'/∼ ≅ {三买,三卖}）。
    ★有效域声明（formalization-validity-domain）：本 `Classifies` 实例的有效域 =
    `ThirdSubX` 子域，**严格小于**全买卖点定义域（全域见 `no_global_classifies`：
    互斥分类不存在）。与 Layer2A 互补：全局互斥三分失败，子域双射。

  认识论等级：全部 L0（定义内蕴；互斥分类不存在是构造性反例，子域双射是结构推导，非经验否证）。
  formalization-validity-domain：Lean build 通过 = 逻辑正确（L0），不膨胀为实证有效域。

  谱系：598（真完全分类元判据）→ 603（递归 vs 轴范式）→ 615（Layer1 ⊊ Layer2 概念分离）。
-/

import Formal.BSPLabels
import Strict.Classification

namespace Strict.BSP

open Formal.BSPLabels
open Formal.BSPLabels.BSPType
open Formal.BSPLabels.EndpointSituation

/-! ════════════════════════════════════════════════════════════════════════
    § Layer2A canonical — 全域互斥分类不存在（诚实降级为按标签商分类）
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★全域谓词族（X=EndpointSituation, C=BSPType）：type1↦IsFirst / type2↦IsSecond / type3↦IsThird。
  这是 `Strict.Classifies` 的 `P : C → X → Prop` 参数——把三类判据组织为内核要求的谓词族。
-/
def globalBSPPred : BSPType → EndpointSituation → Prop
  | type1, x => x.IsFirst
  | type2, x => x.IsSecond
  | type3, x => x.IsThird

/--
  ★★Layer2A canonical：买卖点全域**不存在** `Strict.Classifies` 实例（L0）。

  内核 `Classifies` 要求 `disjoint`：∀ x c₁ c₂, P c₁ x → P c₂ x → c₁ = c₂（互斥/≤1 重叠）。
  但 `x_2b3b` 同时满足 `globalBSPPred type2`（IsSecond）与 `globalBSPPred type3`（IsThird），
  而 `type2 ≠ type3` ⟹ disjoint 必然失败 ⟹ 任何 I 都不能构成 `Classifies`。

  这是 Formal/BSPLabels (A) `bsp_not_exclusive_trichotomy` 在内核类型上的 canonical 形式——
  买卖点全域不是互斥 sum type 分类（codex#1：重合点无恰好一格的像，不得冒充互斥三分）。
-/
theorem no_global_classifies :
    ¬ ∃ I : EndpointSituation → BSPType,
        Strict.Classifies EndpointSituation BSPType I globalBSPPred := by
  rintro ⟨I, hcls⟩
  -- x_2b3b 同时是二买与三买，触发 disjoint 矛盾 type2 = type3
  have h2 : globalBSPPred type2 x_2b3b := (⟨rfl, rfl⟩ : x_2b3b.IsSecond)
  have h3 : globalBSPPred type3 x_2b3b := (⟨rfl, rfl⟩ : x_2b3b.IsThird)
  have heq : type2 = type3 := hcls.disjoint h2 h3
  exact BSPType.noConfusion heq

/--
  ★具体满射标签函数 `IGlobal`（用于 `global_only_label_quotient` 的 realized）。

  读出端点的"主导类别标签"（优先级 一类 > 二类 > 三类，仅作满射见证用）：
  - 在最后中枢下方 → type1；否则 afterFirstBuy → type2；否则 → type3。
  这只是一个**具体**的标签投影——它满射到 BSPType（三个标签都可达），
  使「按标签商分类」的 SemanticQuotient 可被真正实例化（realized 有真见证）。
-/
def IGlobal (x : EndpointSituation) : BSPType :=
  if x.belowLastCenter then type1
  else if x.afterFirstBuy then type2
  else type3

/-- type1 满射见证。 -/
def witness1 : EndpointSituation :=
  { afterFirstBuy := false, isPullbackEnd := false, leftCenter := false,
    retraceNotReenter := false, belowLastCenter := true, sd := Side.buy }

/-- type2 满射见证。 -/
def witness2 : EndpointSituation :=
  { afterFirstBuy := true, isPullbackEnd := false, leftCenter := false,
    retraceNotReenter := false, belowLastCenter := false, sd := Side.buy }

/-- type3 满射见证。 -/
def witness3 : EndpointSituation :=
  { afterFirstBuy := false, isPullbackEnd := false, leftCenter := false,
    retraceNotReenter := false, belowLastCenter := false, sd := Side.buy }

/--
  ★Layer2A canonical（诚实降级目标）：买卖点全域分类只能停留在 `Strict.SemanticQuotient`
  在**标签核** ∼_label（`fun x y => IGlobal x = IGlobal y`）下——即「按标签商分类」（L0）。

  内核 `SemanticQuotient` 在 `Rel := fun x y => IGlobal x = IGlobal y` 下四项义务成立：
  - equiv：标签相等是等价关系。
  - invariant/complete：在标签核下二者均是 `IGlobal x = IGlobal y`（同义反复——complete
    无独立语义内容）。
  - realized：IGlobal 满射（witness1/2/3 见证三个标签）。

  内核 docstring 已警示：此时 `Rel` 正是"分类结果相同"，**无独立语义内容**——这恰是
  codex#1 命名规则要求显式标注的「按标签商分类」，**不是**真语义双射 X/∼ ≅ P
  （真语义双射在全域被 `no_global_classifies` 否定）。
-/
theorem global_only_label_quotient :
    Strict.SemanticQuotient EndpointSituation BSPType IGlobal
      (fun x y => IGlobal x = IGlobal y) where
  equiv := ⟨fun _ => rfl, fun h => h.symm, fun h1 h2 => h1.trans h2⟩
  invariant := fun h => h
  complete := fun h => h
  realized := by
    intro p
    cases p
    · exact ⟨witness1, rfl⟩
    · exact ⟨witness2, rfl⟩
    · exact ⟨witness3, rfl⟩

/-! ════════════════════════════════════════════════════════════════════════
    § Layer2B canonical — 第三类本征子域 = 真 `Strict.Classifies` 双射

    目标分类集 P' = `Side`（三买=buy / 三卖=sell）——与 Formal/BSPLabels.lean (C) 段的
    `thirdInvariant : EndpointSituation → Side` 一致（第三类只按方向区分，不另立 enum）。
    ════════════════════════════════════════════════════════════════════════ -/

/-- ★第三类本征子域对象（codex#1 `AfterBreakRetraceX`）：带子域成员证明的端点。 -/
def ThirdSubX : Type := { x : EndpointSituation // AfterBreakRetrace x }

/-- 子域不变量 I' : X' → Side（由方向读出，= Formal/BSPLabels `thirdInvariant`）。 -/
def thirdSubI (x : ThirdSubX) : Side := x.val.sd

/-- 子域谓词族 P' : Side → X' → Prop（buy↦三类买点 / sell↦三类卖点）。 -/
def thirdSubPred : Side → ThirdSubX → Prop
  | Side.buy, x => IsThirdBuy x.val
  | Side.sell, x => IsThirdSell x.val

/--
  ★★Layer2B canonical：第三类本征子域实例化 `Strict.Classifies`（真语义双射，L0）。

  这是 codex#1 推荐的诚实子域定理的 canonical 兑现：第三类买卖点在其本征子域
  （离开中枢后回试/回抽不入中枢）上**是**真完全分类 X'/∼ ≅ {三买(buy),三卖(sell)}。
  五项证明义务全部由 Formal/BSPLabels (C) 段的 third_exhaustive / third_exclusive +
  Side 二歧导出。这是 Formal/BSPLabels `thirdInvariant_*` 三件套在内核类型上的 canonical 形式。

  ★有效域声明（formalization-validity-domain）：本 `Classifies` 实例的有效域 =
  `ThirdSubX` 子域，**严格小于**全买卖点定义域（全域见 `no_global_classifies`：
  互斥分类不存在）。与 Layer2A 互补：全局互斥三分失败，子域双射。
-/
theorem third_subdomain_classifies :
    Strict.Classifies ThirdSubX Side thirdSubI thirdSubPred where
  total := by
    intro x
    rcases third_exhaustive x.val x.property with hb | hs
    · exact ⟨Side.buy, hb⟩
    · exact ⟨Side.sell, hs⟩
  sound := by
    intro x
    have ht : x.val.IsThird := (afterBreakRetrace_iff_isThird x.val).mp x.property
    show thirdSubPred (thirdSubI x) x
    unfold thirdSubI
    cases hsd : x.val.sd
    · show IsThirdBuy x.val; exact ⟨ht, hsd⟩
    · show IsThirdSell x.val; exact ⟨ht, hsd⟩
  complete := by
    intro x c hP
    cases c with
    | buy =>
      have hbuy : x.val.sd = Side.buy := hP.2
      show thirdSubI x = Side.buy
      unfold thirdSubI; exact hbuy
    | sell =>
      have hsell : x.val.sd = Side.sell := hP.2
      show thirdSubI x = Side.sell
      unfold thirdSubI; exact hsell
  disjoint := by
    intro x c₁ c₂ hP1 hP2
    -- 同一对象的方向唯一 ⟹ 两标签相等（互斥的内核形式）
    cases c₁ <;> cases c₂ <;>
      first
        | rfl
        | (exfalso
           first
             | exact third_exclusive x.val ⟨hP1, hP2⟩
             | exact third_exclusive x.val ⟨hP2, hP1⟩)
  realized := by
    intro c
    cases c with
    | buy =>
      refine ⟨⟨{ afterFirstBuy := false, isPullbackEnd := false, leftCenter := true,
                 retraceNotReenter := true, belowLastCenter := false, sd := Side.buy }, ?_⟩, ?_⟩
      · exact ⟨rfl, rfl⟩
      · show IsThirdBuy _; exact ⟨⟨rfl, rfl⟩, rfl⟩
    | sell =>
      refine ⟨⟨{ afterFirstBuy := false, isPullbackEnd := false, leftCenter := true,
                 retraceNotReenter := true, belowLastCenter := false, sd := Side.sell }, ?_⟩, ?_⟩
      · exact ⟨rfl, rfl⟩
      · show IsThirdSell _; exact ⟨⟨rfl, rfl⟩, rfl⟩

/-! ════════════════════════════════════════════════════════════════════════
    § Layer2A′ canonical — 精化 P/∼ 的**全域真双射**（codex 编排者裁决2.2：精化优先）

    codex 裁决（/tmp/codex_planruling_answer.md）：
    - 裁决1.3：买卖点语义 ∼ = **B 结构等价**（走势结构中的**触发形态**；账户效果属策略层，
      不用来定义买卖点本体等价）。
    - 裁决2.2：**选 b 精化**（不是仅降级）——既然已证 2B/3B 可重合=旧 6-标签非单射，
      正式目标是**精化 P/∼ 使成真双射**，把 2B/3B 重合例作为旧定义失败反例保留。

    精化思路（codex#1「路径 R∼ 细化 ∼ / P′ 含位置-离开标记的更细标签」）：
    旧 P = {一∣二∣三}（互斥 sum type，单标签）→ 非单射（V 重合无恰好一格的像）。
    精化 P′ = **触发形态签名** `RefinedBSP`（端点结构上触发了**哪些**类，作为完整签名，
    **不**压成单标签）——2B/3B 重合端点映到独立的 `secondThird` 标签（不塌缩）。
    ∼′ = **B 结构等价** = 触发形态相同（每类成员资格 ↔ 一致 ∧ 方向一致）——
    独立于标签的结构等价（由 IsFirst/IsSecond/IsThird 结构判据定义，非 I′ 输出的语法相等）。
    在 P′/∼′ 下分类**真单射** ⟹ `Strict.Classifies` 全证 = `TrueCompleteClassification`。
    旧 6-标签的单标签投影是 P′ 的 quotient（codex#1：粗标签是精化分类的商）。
    ════════════════════════════════════════════════════════════════════════ -/

/--
  ★合法买卖点端点（精化双射的论域 X，升跌完备性 010：端点必是某类买卖点）。

  限定为「至少触发一类买卖点」的端点——排除 `coarseClass=none` 中的"零格"（非买卖点端点）。
  "多格"（2B/3B 重合）**保留在论域内**（不是排除，是精化标签到 `secondThird`）。
-/
def ValidBSPEndpoint : Type :=
  { x : EndpointSituation // x.IsFirst ∨ x.IsSecond ∨ x.IsThird }

/--
  ★精化触发形态签名 `RefinedBSP`（codex 裁决2.2 的精化 P′）。

  端点结构上触发的**类组合**——穷尽枚举结构可实现的组合（1B 与 2B/3B 互斥，
  2B/3B 可重合，见 first_second_disjoint / first_third_disjoint / coincidence_iff）：
  - `first`        ：仅 1B（1B 与 2B/3B 不重合）。
  - `secondOnly`   ：仅 2B（强势二买，未离开中枢）。
  - `thirdOnly`    ：仅 3B（离开中枢回抽不入，非二买）。
  - `secondThird`  ：2B ∧ 3B 重合（V 型反转——**独立标签，不塌缩**）。
  这 4 个构造子穷尽合法端点的触发形态（不含"零格"=非买卖点，不含含 1B 的组合=互斥）。
-/
inductive RefinedBSP where
  | first
  | secondOnly
  | thirdOnly
  | secondThird
deriving DecidableEq, Repr

open RefinedBSP

/-- 精化分类目标 P′ = 触发形态 × 方向。 -/
abbrev RefinedLabel := RefinedBSP × Side

/--
  ★精化不变量 I′ : X → P′（端点 ↦ 触发形态签名 × 方向，L0）。

  读出端点触发的类组合（合法端点必落入 4 形态之一）+ 方向。这是「触发形态」的忠实读出
  （codex 裁决1.3 的 B 结构等价对象）。注意：含 1B 时优先 `first`（1B 与 2B/3B 互斥，
  合法端点含 1B ⟹ 不含 2B/3B，见 first_second_disjoint）。
-/
def refinedI (x : ValidBSPEndpoint) : RefinedLabel :=
  let e := x.val
  let form :=
    if decide e.IsFirst then RefinedBSP.first
    else
      match decide e.IsSecond, decide e.IsThird with
      | true,  true  => RefinedBSP.secondThird
      | true,  false => RefinedBSP.secondOnly
      | false, true  => RefinedBSP.thirdOnly
      | false, false => RefinedBSP.first  -- 不可达（合法端点至少一类，且非 1B 时必 2B/3B）
  (form, e.sd)

/--
  ★精化谓词族 P′ → X → Prop（触发形态签名匹配）。

  `refinedPred (form, sd) x` = 「x 的触发形态恰是 form ∧ 方向是 sd」。
  各 form 用结构判据 IsFirst/IsSecond/IsThird 的合取/否定刻画（B 结构等价的谓词形式）。
-/
def refinedPred : RefinedLabel → ValidBSPEndpoint → Prop
  | (RefinedBSP.first, sd), x =>
      x.val.IsFirst ∧ x.val.sd = sd
  | (RefinedBSP.secondOnly, sd), x =>
      x.val.IsSecond ∧ ¬ x.val.IsThird ∧ ¬ x.val.IsFirst ∧ x.val.sd = sd
  | (RefinedBSP.thirdOnly, sd), x =>
      x.val.IsThird ∧ ¬ x.val.IsSecond ∧ ¬ x.val.IsFirst ∧ x.val.sd = sd
  | (RefinedBSP.secondThird, sd), x =>
      x.val.IsSecond ∧ x.val.IsThird ∧ x.val.sd = sd

/--
  ★B 结构等价 ∼′（codex 裁决1.3，L0）：触发形态相同 ∧ 方向相同。

  独立于标签的**结构等价**——由每类成员资格的双向蕴含（IsFirst↔/IsSecond↔/IsThird↔）
  + 方向相等定义。这不是 I′ 输出的语法相等（那是标签核 QuotientByLabel），
  而是结构触发形态的等价（codex 裁决1.3 选 B）。
-/
def StructEquiv (x y : ValidBSPEndpoint) : Prop :=
  (x.val.IsFirst ↔ y.val.IsFirst) ∧ (x.val.IsSecond ↔ y.val.IsSecond)
    ∧ (x.val.IsThird ↔ y.val.IsThird) ∧ x.val.sd = y.val.sd

/-- 合法端点含 1B ⟹ 不含 2B/3B（first_second_disjoint + first_third_disjoint 的合并，L0）。 -/
theorem valid_first_excludes (x : ValidBSPEndpoint) (hf : x.val.IsFirst) :
    ¬ x.val.IsSecond ∧ ¬ x.val.IsThird :=
  ⟨fun h2 => first_second_disjoint x.val ⟨hf, h2⟩,
   fun h3 => first_third_disjoint x.val ⟨hf, h3⟩⟩

/-- refinedI 给出的形态恒满足 refinedPred（sound 的核心，L0）。 -/
theorem refinedI_sound (x : ValidBSPEndpoint) : refinedPred (refinedI x) x := by
  obtain ⟨e, hvalid⟩ := x
  show refinedPred (refinedI ⟨e, hvalid⟩) ⟨e, hvalid⟩
  unfold refinedI
  by_cases hf : e.IsFirst
  · obtain ⟨hns, hnt⟩ := valid_first_excludes ⟨e, hvalid⟩ hf
    simp only [hf, decide_true, if_true]
    exact ⟨hf, rfl⟩
  · simp only [hf, decide_false]
    by_cases hs : e.IsSecond <;> by_cases ht : e.IsThird <;>
      simp only [hs, ht, decide_true, decide_false]
    · exact ⟨hs, ht, rfl⟩
    · exact ⟨hs, ht, hf, rfl⟩
    · exact ⟨ht, hs, hf, rfl⟩
    · -- false,false：合法端点至少一类，且非 1B ⟹ 必 2B/3B，与 hs∧ht 矛盾
      exact absurd (hvalid.resolve_left hf |>.resolve_left hs) ht

/--
  ★refinedPred 唯一确定签名（complete/disjoint 的共享引理，L0）：
  若 `refinedPred c x` 成立，则 `c = refinedI x`（触发形态签名是 x 的函数）。
  含 1B 的端点（合法 ⟹ 不含 2B/3B）与各组合逐一对齐；不可能组合用
  first_second_disjoint / first_third_disjoint 判否。
-/
theorem refinedPred_unique (x : ValidBSPEndpoint) (c : RefinedLabel)
    (hP : refinedPred c x) : c = refinedI x := by
  obtain ⟨form, sd⟩ := c
  obtain ⟨e, hvalid⟩ := x
  -- 先消解不可能的类成员组合（1B 与 2B/3B 互斥）
  have hns_of_f : e.IsFirst → ¬ e.IsSecond := fun hf hs => first_second_disjoint e ⟨hf, hs⟩
  have hnt_of_f : e.IsFirst → ¬ e.IsThird := fun hf ht => first_third_disjoint e ⟨hf, ht⟩
  cases form <;>
    simp only [refinedPred] at hP <;>
    (unfold refinedI) <;>
    (by_cases hf : e.IsFirst <;> by_cases hs : e.IsSecond <;> by_cases ht : e.IsThird <;>
      simp_all)

/--
  ★★Layer2A′ canonical：精化 P′/∼′ 的**全域真双射**（codex 裁决2.2，L0）。
  **命名诚实性标签（codex 裁决4）：`TrueCompleteClassification`**。

  在精化触发形态签名 `RefinedBSP × Side` 与 B 结构等价 ∼′ 下，买卖点分类是**真完全分类**
  X/∼′ ≅ P′（五项证明义务全证）。V 型重合端点映到独立 `secondThird` 标签——**不塌缩**，
  非单射被精化消除。旧 6-标签单标签映射是此精化分类的 quotient（见 Layer2A 失败反例）。
-/
theorem refined_classifies :
    Strict.Classifies ValidBSPEndpoint RefinedLabel refinedI refinedPred where
  total := fun x => ⟨refinedI x, refinedI_sound x⟩
  sound := refinedI_sound
  complete := fun {x c} hP => (refinedPred_unique x c hP).symm
  disjoint := fun {x c₁ c₂} hP1 hP2 =>
    (refinedPred_unique x c₁ hP1).trans (refinedPred_unique x c₂ hP2).symm
  realized := by
    intro c
    obtain ⟨form, sd⟩ := c
    cases form with
    | first =>
      refine ⟨⟨{ afterFirstBuy := false, isPullbackEnd := false, leftCenter := false,
                 retraceNotReenter := false, belowLastCenter := true, sd := sd }, ?_⟩, ?_⟩
      · exact Or.inl ⟨rfl, rfl, rfl⟩
      · exact ⟨⟨rfl, rfl, rfl⟩, rfl⟩
    | secondOnly =>
      refine ⟨⟨{ afterFirstBuy := true, isPullbackEnd := true, leftCenter := false,
                 retraceNotReenter := false, belowLastCenter := false, sd := sd }, ?_⟩, ?_⟩
      · exact Or.inr (Or.inl ⟨rfl, rfl⟩)
      · refine ⟨⟨rfl, rfl⟩, ?_, ?_, rfl⟩
        · intro h; exact Bool.noConfusion h.1   -- ¬IsThird：leftCenter=false
        · intro h; exact Bool.noConfusion h.1   -- ¬IsFirst：belowLastCenter=false
    | thirdOnly =>
      refine ⟨⟨{ afterFirstBuy := false, isPullbackEnd := false, leftCenter := true,
                 retraceNotReenter := true, belowLastCenter := false, sd := sd }, ?_⟩, ?_⟩
      · exact Or.inr (Or.inr ⟨rfl, rfl⟩)
      · refine ⟨⟨rfl, rfl⟩, ?_, ?_, rfl⟩
        · intro h; exact Bool.noConfusion h.1   -- ¬IsSecond：afterFirstBuy=false
        · intro h; exact Bool.noConfusion h.1   -- ¬IsFirst：belowLastCenter=false
    | secondThird =>
      refine ⟨⟨{ afterFirstBuy := true, isPullbackEnd := true, leftCenter := true,
                 retraceNotReenter := true, belowLastCenter := false, sd := sd }, ?_⟩, ?_⟩
      · exact Or.inr (Or.inl ⟨rfl, rfl⟩)
      · exact ⟨⟨rfl, rfl⟩, ⟨rfl, rfl⟩, rfl⟩

/--
  ★精化分类对 ∼′ 不变 + 完备（B 结构等价下真双射的语义形式，L0）。

  把 `refined_classifies` 的 complete/sound 转写为 `Strict.SemanticQuotient` 风格的
  ∼′-不变性与 ∼′-完备性——∼′ = `StructEquiv`（B 结构等价，独立结构定义，非标签核）。
  这区别于 Layer2A `global_only_label_quotient`（QuotientByLabel，标签核同义反复）：
  此处 ∼′ 是独立的结构触发形态等价，complete 有真内容。
-/
theorem refined_struct_invariant {x y : ValidBSPEndpoint} (h : StructEquiv x y) :
    refinedI x = refinedI y := by
  obtain ⟨hf, hs, ht, hsd⟩ := h
  obtain ⟨ex, hvx⟩ := x; obtain ⟨ey, hvy⟩ := y
  unfold refinedI
  by_cases hfx : ex.IsFirst <;> by_cases hsx : ex.IsSecond <;> by_cases htx : ex.IsThird <;>
    simp_all

/-! ════════════════════════════════════════════════════════════════════════
    § 命名诚实性 gatekeeper 标签（codex 编排者裁决4）

    每个分类 theorem 标成六类之一（TrueCompleteClassification / QuotientByLabel /
    StructurePartitionOnly / OperationalSemanticsOnly / RuntimeGuard / EmpiricalDomain）：

    | theorem                          | 标签                        | 理由 |
    |----------------------------------|-----------------------------|------|
    | `refined_classifies`             | TrueCompleteClassification  | 精化 P′/∼′(B结构等价) 真单射双射 |
    | `refined_struct_invariant`       | TrueCompleteClassification  | ∼′ 独立结构等价下不变性 |
    | `third_subdomain_classifies`     | TrueCompleteClassification  | 第三类本征子域真双射 |
    | `no_global_classifies`           | QuotientByLabel(失败反例)   | 旧 6-标签互斥分类不存在(非单射) |
    | `global_only_label_quotient`     | QuotientByLabel             | 旧标签核 ∼_label 同义反复商 |
    | Formal `bsp_not_exclusive_trichotomy` | QuotientByLabel(失败反例) | 旧互斥三分非全函数 |
    | Formal `bsp_type_completeness`   | StructurePartitionOnly      | Layer1 标签构造子穷尽(非语义双射) |

    裁决2.2 兑现：精化(refined_classifies)为正式目标(TrueCompleteClassification)，
    2B/3B 重合(no_global_classifies/bsp_not_exclusive_trichotomy)作为旧定义失败反例保留。
    ════════════════════════════════════════════════════════════════════════ -/

end Strict.BSP
