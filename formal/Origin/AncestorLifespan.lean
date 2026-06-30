/-
Origin/AncestorLifespan.lean — gap-B：祖先生命期包含不变量（MW7/MW8 全元素覆盖的硬前置）

★工位定位（task #24 mutex-prove，gap-B 关键前置；mutex-derive #23 + codex 逐字独立发现）：

  #23 推导链复核（`.chanlun/diagnostics/mutex-derive-result.md` §3-B）+ codex 异质独立确认：
  M23（定理3 全元素覆盖）/ M26（自相似递归）的 `∀e Eat(e)` 缺一个 **未论证的不变量**：

      ∀ a ∈ Anc(e),  [λ_e, ρ_e) ⊆ [λ_a, ρ_a)      （祖先生命期包含后代生命期）

  不证此不变量，覆盖证明的「保持开启」步（`SeparateCoverTheorem` §保持：t∈(λ_e,ρ_e) 时
  e∈A_t ⟹ e∈Ã_{t+1}）只能把「祖先在 e 生命期内不提前死」**藏为字段假设**（`activeAt`），
  而 AncOK（`AncestorClosure`）只保证「某 t 时刻祖先在集合中」，**不保证「祖先生命期时间上
  覆盖子生命期」**——这正是 #23 §3-B 的核心 gap：AncOK 会因某祖先提前出局而在 e 自己 ρ_e
  还没到时就把 e 关闭。

★gap-B 判定（缠论语义，非纯编码问题）+ codex 异质确认（VERDICT: TRUE，闭端点形式）：

  关键缠论语义问题：「高级别走势父结束时，内部低级别子是否可能延续到父之后？」

  - **走势分解定理二**（缠论知识库 §8）：任何级别走势类型 **至少由 3 段以上次级别走势构成**。
    ⟹ 父 = 构成它的子的首尾相接拼接（子 tile 父，无缝无重叠）。
  - **走势必完美 / 技术分析基本原理一**（缠论知识库 §8）：任何级别走势类型 **终要完成**；
    父在其 **最后一个构成子完成时** 完成。
  - ⟹ 直接父子满足 **闭端点包含** `λ_a ≤ λ_e ∧ ρ_e ≤ ρ_a`（第一子共享父左端 λ_e=λ_a，
    最后一子共享父结束 ρ_e=ρ_a；端点可重合）。
  - ⟹ **子不能延续超过父**（codex Q4 NO）：父在最后一子完成时完成，更早的子已完成，
    故 ρ_e 之后无构成子存活。
  - ⟹ 同时死亡（t=ρ_e=ρ_a）合法（codex Q5）：e 因 **自己的 ρ_e** 离开，非被祖先提前剔除。

  **判定：gap-B 成立**（TRUE，可证为引理），**不 escalate**（非反例）。
  **但 M08 的开区间真包含 `I_e ⊂ I_{α_e}` 不忠实**——正确形式是 **闭端点包含**
  `λ_a ≤ λ_e ∧ ρ_e ≤ ρ_a`（codex Q2 REVISE-DEFINITION）。本文件以闭端点包含为不变量。

  codex 异质独立确认（`/tmp/gapb_codex_prompt.txt`，read-only sandbox，无 spec/无本工位判断）：
  Q1 YES（直接父子闭包含）/ Q2 REVISE（闭非开）/ Q3 YES（传递到全祖先）/ Q4 NO（子不超父）/
  Q5 OWN-ρ_e（同时死合法）/ VERDICT TRUE 闭端点形式——与本工位判定逐点一致。

★no-workaround / 形式化策略（缠论公理忠实编码，非硬证、非补丁）：

  「直接父子闭端点包含」是 **走势分解定理二 + 走势必完美** 的结构推论，本身是缠论 **公理性
  结构性质**（不可从更原始 Lean 命题推出——它是走势级别递归的定义内蕴）。本文件把它编码为对
  父函数 `par` 的 **结构假设谓词** `DirectParentContains par`（对每个非根 e 与其直接父 p，
  `p.λ ≤ e.λ ∧ e.ρ ≤ p.ρ`），然后 **严格证明** 它递归传递到全祖先链 + 推出「祖先在子生命期内
  不提前死」。这不是 admit/sorry——是把缠论公理作为显式 hypothesis，在其上做真证明。下游
  覆盖证明（M23/M26）引本文件的 `ancestor_alive_in_child_life` 把「保持开启」步的字段假设
  `activeAt` 替换为结构推论（祖先生命期闭包含 ⟹ 祖先在子生命期内必活）。

★类型选择（与 AncOK 同型，gap-B 必须支撑的对象）：本文件在 **`SeparateStrategyTarget.SyntaxElement`**
  （W9 元素，含 `lambda : Int`/`rho : Int` 端点 + `parent` + AncestorClosure `ancestors` 父链上溯）
  上定义 gap-B——因 gap-B 的全部价值是「保证 `AncestorClosure` 的 AncOK 不提前剔除 e」，而 AncOK
  正作用于此类型。端点用 `Int`（W9 `lambda/rho : Int`），与 `SyntaxElement.start/finish` 一致。

★owner 边界（互斥铁律）：本文件 **新建**。import `Origin.AncestorClosure`（复用 `ancestors` 父链
  上溯 + `IsRoot`/`IsParent`，及其传递的 `SeparateStrategyTarget.SyntaxElement`），不重定义祖先链、
  不改 W9。不碰其他文件、不编辑 lakefile.toml 之外（Lead 登记 root `Origin.AncestorLifespan`）。

★认识论等级（231号强制）：全部 **L0**（纯结构 / Int 端点代数 / 树深归纳）。闭端点包含是走势
  分解定理二 + 走势必完美的结构转录（缠论公理），传递性与不提前死是 Int 不等式组合的同义反复
  ——**信息增量为零**（L0），在「构造的元素树」上成立 ≠ 真实市场有效。本文件 **不** 声称任何
  L1+ 经验有效性。

依赖方向（单向无环，全 committed 只读）：
  AncestorLifespan → AncestorClosure → SeparateStrategyTarget → … （纯 Origin）。
命名空间 NewChanlun.Origin.AncestorLifespan。禁 sorry/admit/axiom。简体中文。
-/

import Origin.AncestorClosure

namespace NewChanlun.Origin.AncestorLifespan

open NewChanlun.Origin.SeparateStrategyTarget (SyntaxElement)
open NewChanlun.Origin.AncestorClosure (ancestors ancestors_succ ancestors_zero IsRoot IsParent)

/-! ## §A 直接父子闭端点包含（gap-B 不变量的缠论公理基底，codex Q1/Q2）

走势分解定理二（父=子首尾相接拼接）+ 走势必完美（父在最后一子完成时完成）⟹ 直接父子满足
**闭端点包含**：第一子共享父左端、最后一子共享父结束，故 `λ_p ≤ λ_e ∧ ρ_e ≤ ρ_p`（端点可重合）。

★`DirectParentContains par` = 对每个非根元素 e 与其直接父 p（`par e = some p`），闭端点包含成立。
这是缠论 **公理性结构假设**（走势级别递归的定义内蕴，不从更原始命题推出，codex Q2 确认应为
闭非开）。下游一切传递/不提前死的证明以它为 hypothesis（no-workaround：显式 hypothesis 非 admit）。-/

/--
**闭端点包含谓词**（两元素，codex Q2 闭非开）：`a` 的生命期 `[λ_a, ρ_a)` 闭包含 `e` 的生命期
`[λ_e, ρ_e)`——`λ_a ≤ λ_e ∧ ρ_e ≤ ρ_a`（端点可重合，区别于 M08 开区间真包含 `⊂`）。
-/
def LifespanContains (a e : SyntaxElement) : Prop :=
  a.lambda ≤ e.lambda ∧ e.rho ≤ a.rho

/-- 闭端点包含自反（L0，≤ 自反）：任一元素生命期闭包含自身。 -/
theorem lifespanContains_refl (e : SyntaxElement) : LifespanContains e e :=
  ⟨Int.le_refl _, Int.le_refl _⟩

/-- ★闭端点包含传递（L0，≤ 传递）：a 闭包含 b，b 闭包含 c ⟹ a 闭包含 c。
    这是 §C 沿祖先链传递到全祖先的代数核心（codex Q3 inequalities compose）。 -/
theorem lifespanContains_trans {a b c : SyntaxElement}
    (hab : LifespanContains a b) (hbc : LifespanContains b c) :
    LifespanContains a c :=
  ⟨Int.le_trans hab.1 hbc.1, Int.le_trans hbc.2 hab.2⟩

/--
**直接父子闭端点包含公理**（gap-B 缠论基底，走势分解定理二 + 走势必完美的结构编码）：
对父函数 `par`，每个非根元素 e（`par e = some p`）其直接父 p 闭包含 e 的生命期。

★这是走势级别递归的定义内蕴（父=子拼接 + 父在最后一子完成时完成 ⟹ 第一子共享父左端、
最后一子共享父结束）。作为对 `par` 的结构假设（codex Q1 YES / Q2 闭形式确认），不从更原始
Lean 命题推出——下游证明以它为显式 hypothesis（no-workaround：非 admit/sorry）。
-/
def DirectParentContains (par : SyntaxElement -> Option SyntaxElement) : Prop :=
  ∀ e p, par e = some p -> LifespanContains p e

/-! ## §C 闭端点包含递归传递到全祖先链（codex Q3，沿 `ancestors` 树深归纳）

直接父闭包含（§A）沿父链 `ancestors par fuel e` 上溯，由 `lifespanContains_trans` 组合，
传递到 e 的 **全部祖先**——`∀ a ∈ Anc(e), LifespanContains a e`。证明对 fuel 归纳（树深有限，
§十九）：fuel=0 空链平凡；fuel=n+1 时链头是直接父 p（§A 给 p 闭包含 e），链尾是 p 的祖先
（归纳假设给闭包含 p，再 `trans` 得闭包含 e）。-/

/--
**祖先生命期闭包含全祖先**（gap-B 不变量主引理，codex Q3 YES）：在直接父闭包含公理
`DirectParentContains par` 下，e 的 **每个祖先** a（`a ∈ ancestors par fuel e`）都闭包含 e 的
生命期——`LifespanContains a e`，即 `λ_a ≤ λ_e ∧ ρ_e ≤ ρ_a`。

★对 fuel 归纳（树深有限 §十九）：
  - fuel=0：祖先链空（`ancestors_zero`），成员关系矛盾（无祖先）。
  - fuel=n+1：`par e = some p` 时 `ancestors par (n+1) e = p :: ancestors par n p`。
    祖先 a 或是直接父 p（§A 公理给 `LifespanContains p e`），
    或是 p 的祖先（归纳假设给 `LifespanContains a p`，再与 §A 的 `LifespanContains p e`
    经 `lifespanContains_trans` 得 `LifespanContains a e`）。
-/
theorem ancestor_lifespan_contains
    (par : SyntaxElement -> Option SyntaxElement)
    (hDPC : DirectParentContains par) :
    ∀ (fuel : Nat) (e a : SyntaxElement),
      a ∈ ancestors par fuel e -> LifespanContains a e := by
  intro fuel
  induction fuel with
  | zero =>
    intro e a ha
    rw [ancestors_zero] at ha
    exact absurd ha (List.not_mem_nil)
  | succ n ih =>
    intro e a ha
    cases hpar : par e with
    | none =>
      -- 根元素（par e = none）祖先链为空（ancestors 的 none 分支）⟹ a ∈ [] 矛盾
      have : ancestors par (n + 1) e = [] := by simp only [ancestors, hpar]
      rw [this] at ha
      exact absurd ha (List.not_mem_nil)
    | some p =>
      rw [ancestors_succ par n e p hpar] at ha
      rcases List.mem_cons.mp ha with hap | hap
      · -- a 是直接父 p：§A 公理直接给闭包含（a = p，rw 改写目标避免 subst 方向歧义）
        rw [hap]
        exact hDPC e p hpar
      · -- a 是 p 的祖先：归纳假设给 a 闭包含 p，§A 给 p 闭包含 e，trans
        have hcp : LifespanContains p e := hDPC e p hpar
        have hap' : LifespanContains a p := ih p a hap
        exact lifespanContains_trans hap' hcp

/-! ## §D 祖先在子生命期内不提前死（gap-B 顶点，codex Q4/Q5；覆盖证明「保持步」根据）

gap-B 的全部价值：在 e 的生命期 `t ∈ [λ_e, ρ_e)` 内，任何祖先 a 满足
`λ_a ≤ λ_e ≤ t < ρ_e ≤ ρ_a` ⟹ `t ∈ [λ_a, ρ_a)`——祖先此刻 **仍活着**（未进 D_t）。

这把 `SeparateCoverTheorem` 「保持开启」步的字段假设 `activeAt`（藏起来的「祖先不提前死」）
替换为 **结构推论**：祖先生命期闭包含 ⟹ 祖先在子的整个生命期内必活，故 AncOK 不在 e 自己 ρ_e
之前剔除 e（codex Q4 子不超父 / Q5 同时死才合法）。-/

/-- 时刻 t 落在元素 a 的生命期内（半开区间 `[λ_a, ρ_a)`，与 `SyntaxElement.start/finish` 同语义）：
    `λ_a ≤ t < ρ_a`（t 用 Int，与 W9 `lambda/rho : Int` 一致）。 -/
def AliveAt (a : SyntaxElement) (t : Int) : Prop :=
  a.lambda ≤ t ∧ t < a.rho

/--
**祖先在子生命期内必活（gap-B 顶点引理，codex Q4/Q5）**：在直接父闭包含公理下，若 t 在 e 的
生命期内（`λ_e ≤ t < ρ_e`）且 a 是 e 的祖先（`a ∈ ancestors par fuel e`），则 t 也在 a 的
生命期内（`λ_a ≤ t < ρ_a`）——**祖先在子的整个生命期内不死**。

这是覆盖证明「保持开启」步（M23/M26）的结构根据：AncOK 不会因祖先提前出局而在 e 自己 ρ_e
之前关闭 e（祖先此刻必活 ⟹ e∈A_t ⟹ e∈A_{t+1} 真成立，非字段假设）。

证明：`ancestor_lifespan_contains` 给 `λ_a ≤ λ_e ∧ ρ_e ≤ ρ_a`；与 `λ_e ≤ t < ρ_e` 组合得
`λ_a ≤ λ_e ≤ t` 且 `t < ρ_e ≤ ρ_a`，即 `AliveAt a t`（≤/< 传递）。
-/
theorem ancestor_alive_in_child_life
    (par : SyntaxElement -> Option SyntaxElement)
    (hDPC : DirectParentContains par)
    (fuel : Nat) (e a : SyntaxElement) (t : Int)
    (hAnc : a ∈ ancestors par fuel e)
    (hAlive : AliveAt e t) :
    AliveAt a t := by
  have hcontain : LifespanContains a e :=
    ancestor_lifespan_contains par hDPC fuel e a hAnc
  obtain ⟨hlam, hrho⟩ := hcontain
  obtain ⟨ht0, ht1⟩ := hAlive
  exact ⟨Int.le_trans hlam ht0, Int.lt_of_lt_of_le ht1 hrho⟩

/--
**祖先不提前死的等价形式（codex Q5：e 因自己 ρ_e 离开，非被祖先提前剔除）**：在直接父闭包含
公理下，e 在自己生命期内（t < ρ_e）的任一时刻，其任一祖先 a 尚未到自己的结束（t < ρ_a）。

即：祖先的 ρ_a 不早于 e 的 ρ_e（因 ρ_e ≤ ρ_a），故 e 在 [λ_e,ρ_e) 内时祖先恒未结束——
AncOK 的 D_t 不含任何 e 的祖先（在 e 生命期内）⟹ 「保持开启」步不被祖先提前剔除破坏。
-/
theorem ancestor_not_ended_before_child
    (par : SyntaxElement -> Option SyntaxElement)
    (hDPC : DirectParentContains par)
    (fuel : Nat) (e a : SyntaxElement) (t : Int)
    (hAnc : a ∈ ancestors par fuel e)
    (hInLife : t < e.rho) :
    t < a.rho := by
  have hcontain : LifespanContains a e :=
    ancestor_lifespan_contains par hDPC fuel e a hAnc
  exact Int.lt_of_lt_of_le hInLife hcontain.2

/-! ## §E codex Q4 反例不存在的形式化（子不能延续超过父）

codex Q4 NO：子不能延续超过父——任一祖先 a，e 的结束 ρ_e ≤ a 的结束 ρ_a。
形式化为 `ancestor_lifespan_contains` 的右分量（`e.rho ≤ a.rho`），即 **不存在**
ρ_e > ρ_a 的祖先（子延续超父）的配置——gap-B 是 TRUE 而非 FALSE（无反例，不需 escalate）。-/

/--
**子不延续超父（codex Q4 NO，反例不存在）**：在直接父闭包含公理下，e 的任一祖先 a 满足
`ρ_e ≤ ρ_a`——子的结束不晚于祖先的结束，**不存在子延续超过父的配置**。

这坐实 gap-B VERDICT TRUE（无反例）：M23「吃到每个元素」无需修正定义（非 escalate 情形），
只需以闭端点包含（非 M08 开区间真包含）为不变量。
-/
theorem child_not_outlive_ancestor
    (par : SyntaxElement -> Option SyntaxElement)
    (hDPC : DirectParentContains par)
    (fuel : Nat) (e a : SyntaxElement)
    (hAnc : a ∈ ancestors par fuel e) :
    e.rho ≤ a.rho :=
  (ancestor_lifespan_contains par hDPC fuel e a hAnc).2

end NewChanlun.Origin.AncestorLifespan
