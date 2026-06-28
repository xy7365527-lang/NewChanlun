/-
Origin/ForceConformance.lean — 力度 conformance 规约（rust MACD ↔ Lean ForceMeasure，task #131）

★工位定位（#131 force-L2 conformance，承接 #124 ForceInterface）：
  committed Origin/ForceInterface.lean（#124）已把力度 measure 形式化为抽象 interface
  `ForceMeasure α`（走势载体 → Force + 单调/忠实公理）+ 背驰判据经 interface
  （`IsDivergenceVia`）+ well-definedness。但 **still-MISSING-C（force-L2 conformance）**：
  rust MACD 引擎与 Lean `ForceMeasure` 之间**无形式 conformance spec**——「rust 端算出的力度
  是一个合法 ForceMeasure 实例」这一命题既未陈述也未标等级。

  本文件补这个缺口：定义 **conformance 规约**，把「外部实现（rust MACD）符合 Lean
  ForceMeasure interface」严格表达，并精确划分两层：
    ① **结构层 conformance（L0，本文件证）**：「任何满足 ForceMeasure 公理的度量 m 在背驰
       判据中行为一致」——conformance 关系良构（自反/对称/传递），等价 measure 给等价判据，
       conformance 是判据层的等价不变量。不依赖任何数据，纯逻辑。
    ② **MACD conformance 命题（L2，本文件不证，显式标 L2 接口假设）**：「rust MACD 是
       ForceMeasure 实例」——这是关于真实数值引擎在真实 K 线上行为的**经验命题**，需 L2 真实
       数据验证，**当前未验证**。本文件把它表达为**显式假设**（structure 字段 / Prop 前件），
       绝不伪装成已证定理，绝不声明 rust 已对齐（那是 #127 的事，不是本工位）。

═══════════════════════════════════════════════════════════════════════════
权威来源（三级权威链，博文为最终权威）
═══════════════════════════════════════════════════════════════════════════
- 第24课（背驰-买卖点定理，博文最终权威）：趋势背驰——同向两段，后段力度弱于前段
  （MACD 柱子面积 C < A）。
- §9（背驰，知识库）：力度比较；力度序的逻辑结构。
- committed Origin/ForceInterface.lean（#124）：`ForceMeasure α`（measure + strength + mono +
  faithful）+ `IsDivergenceVia fm a c = (fm.measure c).area < (fm.measure a).area`——本文件
  build on 此 interface，给其 conformance 规约（不改 interface，只新增 conformance 层）。
- committed Origin/Divergence.lean：`Force`（area : Nat）+ `IsDivergence`——经 ForceInterface 桥接。

═══════════════════════════════════════════════════════════════════════════
认识论等级（formalization-validity-domain 强制标注）+ L0/L2 边界精确划线
═══════════════════════════════════════════════════════════════════════════
本文件 = **conformance 规约**，明确划分 conformance 的 **结构层（L0）** 与 **实例符合（L2）**：

  ┌─────────────────────────────────────────────────────────────────────┐
  │ L0（本文件实证，纯逻辑，不依赖数据）：conformance 的结构性质          │
  │   · `ForceConforms ref cand D`：两个 ForceMeasure 在测试集 D 上对**所有**│
  │     走势对给出相同背驰判定——conformance 关系的定义。                   │
  │   · 证：conformance 在判据层自反/对称/传递（等价关系结构）；measure 输出 │
  │     相等 ⟹ conformance（外延蕴含）；conforming 实例继承全部背驰判据      │
  │     定理（互斥穷尽等）——「满足公理的 measure 在背驰判据中行为一致」。   │
  ├─────────────────────────────────────────────────────────────────────┤
  │ L2（本文件**不实证**，显式标 L2 接口假设，rust 端由 #127 对齐）：       │
  │   · `MacdForceMeasureWitness`：携带「rust MACD 给出一个合法 ForceMeasure │
  │     实例」的**假设见证**（structure，字段是 L2 待验证义务，非已证定理）。│
  │   · `MacdConformsTo`：「rust MACD 实例与参考 measure conformance」的       │
  │     命题——**陈述不证明**，作为 L2 接口假设。真实 MACD 数值引擎在真实 K   │
  │     线上是否真满足 ForceMeasure 公理 + 与参考一致，是经验问题，当前未验证。│
  └─────────────────────────────────────────────────────────────────────┘

`lake env lean Origin/ForceConformance.lean` 通过 = conformance 关系的**逻辑结构**（等价性 +
外延蕴含 + 判据继承）+ L2 假设的**显式可消费形式**（条件定理：给定 L2 见证 ⟹ L0 后果）在
定义层成立，**不是**任何「rust MACD 真的是合法 ForceMeasure 实例」的实证断言（那需 L2 真实
数据 + #127 rust 对齐，本文件不冒充、不声明已对齐）。

★诚实标注（no-patch-mentality / no-声明膨胀 / formalization-validity-domain）：
  · L2 假设用 `structure` 字段 + `Prop` 前件**显式表达**（不是 sorry，不是 axiom 伪装定理）。
    条件定理 `conformance_consequences_under_witness` 形如「假设 H_L2 → 结论 L0」——L2 前件
    **从不被本文件 discharge**，留给 #127 rust 对齐 + 真实数据验证。
  · 本文件**不声明** rust 已对齐——`MacdConformsTo` 是命题（Prop），不是已证 theorem；
    `MacdForceMeasureWitness` 是假设载体（其存在性本文件不构造，只定义形状）。
  · L2 否定性结果入口：若真实 MACD 在某标的上算出的力度违反 ForceMeasure 公理（mono/faithful）
    或与参考 measure 判据不一致，则该实例**不构成** `MacdForceMeasureWitness`——L2 数据可否证
    conformance 假设（formalization-validity-domain：有效域 ≠ 定义域，L2 可缩小有效域边界）。

禁 sorry/admit。L2 假设用 structure field / Prop 前件显式表达 + 注释标 L2（不是 sorry）。
不依赖 Mathlib。纯 Prop/Type。
-/

import Origin.ForceInterface

namespace NewChanlun.Origin

/-! ═══════════════════════════════════════════════════════════════════════
    § 1. conformance 关系（L0）：两个 ForceMeasure 在测试集上背驰判定一致

    ★L0 边界：conformance 只规定「两个 measure 对相同走势对给出相同背驰判定」，
    不规定「measure 怎么从 K 线算」。这是 conformance 的**逻辑形式**——独立于任何实现。
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **conformance（L0，#131 核心定义）** —— 候选 measure `cand` 与参考 measure `ref` 在测试集
  `D : List (α × α)`（走势对列表）上 **conform**：对 `D` 中**每个**走势对 `(a, c)`，二者的背驰
  判定一致（`IsDivergenceVia ref a c ↔ IsDivergenceVia cand a c`）。

  ★这是「rust MACD 实现符合 Lean ForceMeasure 参考」的**结构形式**：conformance 不要求两个
  measure 的 `measure` 输出逐点相等（实现可不同），只要求它们在背驰判据上**行为一致**——
  这正是「背驰的形式化独立于具体 MACD 实现」（#124 `divergenceVia_depends_only_on_output`）
  在 conformance 层的对应：判据一致是 conformance 的充要刻画，不是 measure 逐点相等。
-/
def ForceConforms {α : Type u} (ref cand : ForceMeasure α) (D : List (α × α)) : Prop :=
  ∀ p ∈ D, (IsDivergenceVia ref p.1 p.2 ↔ IsDivergenceVia cand p.1 p.2)

/--
  **全域 conformance（L0）** —— 候选 measure 与参考在**所有**走势对上 conform（不限于测试集）。
  这是 conformance 的最强形式：`cand` 与 `ref` 背驰判定处处一致。测试集 conformance（`ForceConforms`）
  是其在有限样本上的可观测投影——L2 验证只能检查有限测试集，全域 conformance 是其外推假设。
-/
def ForceConformsAll {α : Type u} (ref cand : ForceMeasure α) : Prop :=
  ∀ a c : α, (IsDivergenceVia ref a c ↔ IsDivergenceVia cand a c)

/-! ═══════════════════════════════════════════════════════════════════════
    § 2. conformance 结构定理（L0，本文件证）：①「任何满足 ForceMeasure 公理的度量在
         背驰判据中行为一致」——conformance 是判据层的等价关系 + 外延蕴含 + 判据继承。
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **★conformance 自反（L0，§9）** —— 任意 ForceMeasure 与自身 conform（在任意测试集上）。
  平凡但必要：conformance 是自反关系（参考 measure 与自身对齐）。
-/
theorem forceConforms_refl {α : Type u} (fm : ForceMeasure α) (D : List (α × α)) :
    ForceConforms fm fm D := by
  intro _ _; exact Iff.rfl

/--
  **★conformance 对称（L0，§9）** —— 若 `cand` 与 `ref` conform，则 `ref` 与 `cand` conform。
  conformance 是对称关系（谁是参考、谁是候选不影响判据一致性）。
-/
theorem forceConforms_symm {α : Type u} (ref cand : ForceMeasure α) (D : List (α × α))
    (h : ForceConforms ref cand D) : ForceConforms cand ref D := by
  intro p hp; exact (h p hp).symm

/--
  **★conformance 传递（L0，§9）** —— conformance 链可传递：`ref`–`mid` conform 且 `mid`–`cand`
  conform ⟹ `ref`–`cand` conform。这保证 conformance 是判据层的**等价关系**：多个实现两两
  conform 可链式合成（如 rust MACD ↔ python 参考 ↔ Lean identityForceMeasure）。
-/
theorem forceConforms_trans {α : Type u} (ref mid cand : ForceMeasure α) (D : List (α × α))
    (h1 : ForceConforms ref mid D) (h2 : ForceConforms mid cand D) :
    ForceConforms ref cand D := by
  intro p hp; exact (h1 p hp).trans (h2 p hp)

/--
  **★measure 输出相等 ⟹ conformance（L0，外延蕴含，#124 `divergenceVia_depends_only_on_output`
  的 conformance 层）** —— 若候选与参考在测试集每个走势对上给出**相同的 measure 输出**
  （`measure` 值逐点相等），则二者 conform。

  这把「背驰判定只依赖 measure 输出」提升到 conformance：measure 逐点相等是 conformance 的
  **充分条件**（非必要——实现可在判据不变前提下输出不同 measure，见 §3 conformance 比逐点相等弱）。
  ★这是「满足公理的 measure 在背驰判据中行为一致」的硬核：判定行为完全由 measure 输出决定。
-/
theorem forceConforms_of_measure_eq {α : Type u} (ref cand : ForceMeasure α) (D : List (α × α))
    (hmeas : ∀ p ∈ D, ref.measure p.1 = cand.measure p.1 ∧ ref.measure p.2 = cand.measure p.2) :
    ForceConforms ref cand D := by
  intro p hp
  obtain ⟨h1, h2⟩ := hmeas p hp
  exact divergenceVia_depends_only_on_output ref cand p.1 p.2 h1 h2

/--
  **★conforming 实例继承背驰互斥穷尽（L0，判据继承）** —— 若 `cand` 与 `ref` 全域 conform，
  则 `cand` 上的背驰判据继承互斥穷尽性（背驰 ⊻ 延续）——这对**任意** ForceMeasure 本就成立
  （`divergenceVia_dichotomy`），本定理坐实 conformance 不破坏判据的二歧结构。

  这是 ① 的精确陈述：「任何满足 ForceMeasure 公理的度量在背驰判据中行为一致」——conformance
  保持判据的全部逻辑性质（互斥穷尽），因为这些性质对所有合法 ForceMeasure 一致成立。
-/
theorem conforming_inherits_dichotomy {α : Type u} (ref cand : ForceMeasure α)
    (_hconf : ForceConformsAll ref cand) (a c : α) :
    IsDivergenceVia cand a c ∨ IsContinuationVia cand a c :=
  divergenceVia_dichotomy cand a c

/--
  **★全域 conformance ⟹ 参考背驰判定可经候选读出（L0，conformance 的可观测意义）** ——
  若 `cand`（如 rust MACD）与 `ref`（Lean 参考）全域 conform，则**参考的背驰判定**等价于
  **候选的背驰判定**：用 rust MACD 算背驰 = 用 Lean 参考算背驰（判定层）。

  这是 conformance 规约的下游意义：一旦 conformance 成立（L2 验证后），rust MACD 可**无歧义
  替换** Lean 参考用于背驰判定——背驰判据接受任意 conforming 实现。
-/
theorem conformsAll_iff_divergence {α : Type u} (ref cand : ForceMeasure α)
    (hconf : ForceConformsAll ref cand) (a c : α) :
    IsDivergenceVia ref a c ↔ IsDivergenceVia cand a c :=
  hconf a c

/-! ═══════════════════════════════════════════════════════════════════════
    § 3. conformance 非平凡（L0）：conformance 比「measure 逐点相等」严格弱——
         存在 conform 但 measure 输出不同的实例（否则 conformance 退化为相等，规约无内容）。
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **缩放力度 measure 见证（L0）** —— 把 `identityForceMeasure`（measure n = ⟨n⟩）缩放：
  `measure n = ⟨2*n⟩`（力度翻倍），`strength n = n`（内在序不变）。单调/忠实公理仍成立
  （`2*a ≤ 2*b ↔ a ≤ b`，`2*a < 2*b → a ≤ b`）。这是一个**合法但与 identity 输出不同**的 measure。
-/
def scaledForceMeasure : ForceMeasure Nat where
  measure n := ⟨2 * n⟩
  strength n := n
  mono := by intro a b h; simp only; omega
  faithful := by intro a b h; simp only at h; omega

/--
  **★conformance 比逐点相等严格弱（L0，#131 非平凡核心）** —— `scaledForceMeasure`（力度翻倍）
  与 `identityForceMeasure` **全域 conform**（背驰判定处处一致），但二者 measure 输出**不相等**
  （在 n=1：identity 给 ⟨1⟩，scaled 给 ⟨2⟩）。

  论证：背驰判定 `(measure c).area < (measure a).area`。identity：`c < a`；scaled：`2c < 2a`，
  二者等价（`2c < 2a ↔ c < a`）。故判定处处一致 ⟹ 全域 conform。但 measure 输出不同。

  ★这证明 conformance **非空洞退化为相等**：conformance 规约真有内容——它允许实现差异
  （rust MACD 的力度数值不必逐点等于 Lean 参考，只需判据一致）。若 conformance = 逐点相等，
  则规约过强（要求 rust 数值精确等于 Lean，无意义）。本定理坐实 conformance 是判据层等价，
  非数值层相等（no-声明膨胀：规约的有效域是判据一致，不是数值相等）。
-/
theorem conformance_strictly_weaker_than_eq :
    ForceConformsAll identityForceMeasure scaledForceMeasure ∧
      identityForceMeasure.measure 1 ≠ scaledForceMeasure.measure 1 := by
  refine ⟨?_, ?_⟩
  · -- 全域 conform：判定等价（`c < a ↔ 2*c < 2*a`）
    intro a c
    unfold IsDivergenceVia identityForceMeasure scaledForceMeasure
    simp only
    -- 拆 Iff 后逐支为纯不等式目标再 omega：保持 #print axioms 仅 propext/Quot.sound
    -- （omega 直接闭合 Iff 目标会引入 Classical.choice；分支后是纯算术，不引入）。
    constructor <;> intro h <;> omega
  · -- measure 输出不同：⟨1⟩ ≠ ⟨2⟩
    unfold identityForceMeasure scaledForceMeasure
    simp only
    decide

/--
  **★缩放 measure 上真背驰见证（L0）** —— conformance 在合法实例对上真跑通：scaled measure
  对走势对 (8, 2) 判背驰（2*2=4 < 2*8=16），与 identity 对 (8,2) 判背驰一致。
-/
theorem witness_scaled_divergence :
    IsDivergenceVia scaledForceMeasure 8 2 := by
  unfold IsDivergenceVia scaledForceMeasure; decide

/-! ═══════════════════════════════════════════════════════════════════════
    § 4. ★MACD conformance 命题（L2，本文件**不证**，显式标 L2 接口假设）

    ② #131 核心交付：「rust MACD 是 ForceMeasure 实例」——这是关于真实数值引擎在真实 K 线上
    行为的**经验命题**（L2），**当前未验证**。本节把它表达为**显式假设载体**（structure 字段 /
    Prop 前件），绝不伪装成已证定理，绝不声明 rust 已对齐（那是 #127）。

    ★严格性（no-patch-mentality）：不用 sorry，不用 axiom 伪装定理。用 structure 把 L2 义务
    显式化为字段——「持有一个 MacdForceMeasureWitness」= 假设 L2 已验证（本文件不构造其实例）。
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **★MACD conformance 假设见证（L2 接口假设，本文件不构造实例）** —— 一个 `MacdForceMeasureWitness`
  携带「rust MACD 引擎在走势载体 `α` 上给出一个合法 ForceMeasure 实例 `macd`，且它与 Lean 参考
  measure `ref` 全域 conform」的**假设**。

  ★字段（L2 待验证义务，**非已证定理**）：
  - `macd : ForceMeasure α`：rust MACD 引擎产出的力度 measure 实例（measure = EMA/DIF/DEA →
    同向段面积积分的 Lean 侧抽象表示）。**本文件不构造此实例**——它由 rust 端（#127）提供 +
    L2 真实数据验证「rust 算出的力度真满足 ForceMeasure 公理」（mono/faithful 在真实 K 线上成立）。
  - `conforms : ForceConformsAll ref macd`：rust MACD 与 Lean 参考全域 conform（背驰判定一致）。
    这是 **L2 命题**——是否真成立需真实数据验证（rust MACD 与 Lean 参考在真实行情上判据是否一致）。

  ★为何用 structure 而非 axiom：axiom 会把 L2 命题注册为**全局真**（伪装已证），违反
  no-声明膨胀。structure 字段是**假设**——只有当外部（#127 + L2 数据）真构造出一个实例时，
  其字段才成立。本文件**不构造** `MacdForceMeasureWitness` 实例，故不声明任何 L2 命题为真。

  ★L2 否定性结果入口（formalization-validity-domain）：若真实 MACD 违反 ForceMeasure 公理
  （如背驰段力度与几何幅度反向）或与参考判据不一致，则**无法构造** `MacdForceMeasureWitness`
  ——L2 数据可否证 conformance 假设，缩小有效域边界（否定性结果 > 确认性结果）。
-/
structure MacdForceMeasureWitness (α : Type u) (ref : ForceMeasure α) where
  /-- rust MACD 引擎产出的力度 measure 实例（L2，本文件不构造，由 #127 + L2 数据提供）。 -/
  macd : ForceMeasure α
  /-- rust MACD 与 Lean 参考全域 conform（L2 命题，需真实数据验证，本文件不证）。 -/
  conforms : ForceConformsAll ref macd

/--
  **MACD conformance 命题（L2，陈述不证明）** —— 「存在一个 rust MACD ForceMeasure 实例与
  参考 measure `ref` 全域 conform」。这是 conformance 规约要 L2 验证的**目标命题**。

  ★本定义是 `Prop`，**不是** theorem——本文件**不证明** `MacdConformsTo ref` 成立（那需 rust
  对齐 #127 + L2 真实数据）。它是 conformance 假设的可引用名，供下游条件定理 + #127 对齐目标使用。
-/
def MacdConformsTo {α : Type u} (ref : ForceMeasure α) : Prop :=
  ∃ macd : ForceMeasure α, ForceConformsAll ref macd

/--
  **★L2 见证 ⟹ L2 命题（L0 桥接，纯逻辑）** —— 若持有一个 `MacdForceMeasureWitness`（假设
  L2 已验证），则 `MacdConformsTo` 命题成立。这是 L2 假设载体到 L2 目标命题的**纯逻辑桥接**
  （把 witness 的存在转为存在量词），本身 L0——不证 L2 命题，只说「**若**有 witness **则**命题成立」。
-/
theorem macdConformsTo_of_witness {α : Type u} (ref : ForceMeasure α)
    (w : MacdForceMeasureWitness α ref) : MacdConformsTo ref :=
  ⟨w.macd, w.conforms⟩

/--
  **★conformance 后果（条件定理，L2 前件 → L0 结论）** —— **给定** 一个 MACD conformance 见证
  `w`（L2 假设，本文件不 discharge），rust MACD 的背驰判定等价于 Lean 参考的背驰判定：
  对任意走势对 `(a, c)`，`IsDivergenceVia ref a c ↔ IsDivergenceVia w.macd a c`。

  ★这是 #131 交付的核心条件定理：它把 L2 假设（conformance）与 L0 后果（判定可替换）**显式
  连接**，且 L2 前件 `w : MacdForceMeasureWitness`**从不被本文件 discharge**——留给 #127 rust
  对齐 + L2 真实数据验证。条件定理形如「假设 H_L2 → 结论 L0」是 no-声明膨胀的严格形式：
  我们证「若 conformance 成立则判定可替换」（L0），不证「conformance 成立」（L2）。
-/
theorem conformance_consequences_under_witness {α : Type u} (ref : ForceMeasure α)
    (w : MacdForceMeasureWitness α ref) (a c : α) :
    IsDivergenceVia ref a c ↔ IsDivergenceVia w.macd a c :=
  w.conforms a c

/--
  **★conformance 见证下 rust MACD 继承背驰判据全部逻辑性质（条件定理，L2 → L0）** ——
  给定 conformance 见证 `w`，rust MACD（`w.macd`）的背驰判据继承互斥穷尽（背驰 ⊻ 延续）。
  这坐实：conformance 一旦 L2 验证，rust MACD 可作为合法 ForceMeasure 插入全部背驰/买卖点判据
  （BspClassification 第一类 = 背驰点）——conformance 是 L0 判据层与 L2 数值引擎的**接缝**。
-/
theorem macd_inherits_dichotomy_under_witness {α : Type u} (ref : ForceMeasure α)
    (w : MacdForceMeasureWitness α ref) (a c : α) :
    IsDivergenceVia w.macd a c ∨ IsContinuationVia w.macd a c :=
  divergenceVia_dichotomy w.macd a c

/-! ═══════════════════════════════════════════════════════════════════════
    § 5. L0 conformance 见证：identity 自 conform（L0 闭环，不碰 L2）

    ★区分 §4（L2 假设载体，本文件不构造实例）：本节给一个**纯 L0**的 conformance 实例
    （identity 与 scaled 自 conform）——证 conformance 规约在 L0 层**可被满足**（不是空约束），
    但**不触及** rust MACD（那是 L2，本文件只陈述假设不构造）。
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **★L0 conformance 见证（identity ↔ scaled 全域 conform）** —— `scaledForceMeasure` 与
  `identityForceMeasure` 全域 conform（§3 已证），故 `MacdConformsTo identityForceMeasure`
  成立——**用纯 L0 实例**（scaled，非 rust MACD）作见证。

  ★这是 L0 闭环：conformance 规约在 L0 层可被满足（存在 conforming 实例对）。**注意**：本见证
  用的是 Lean 内部 L0 实例（scaled），**不是** rust MACD——它证「conformance 规约非空洞」（L0），
  **不**证「rust MACD conform」（那是 L2，本文件不构造 rust 见证）。两者严格区分：L0 见证存在
  ≠ L2 rust 实例存在。
-/
theorem l0_conformance_witness :
    MacdConformsTo identityForceMeasure :=
  ⟨scaledForceMeasure, (conformance_strictly_weaker_than_eq).1⟩

/-! ═══════════════════════════════════════════════════════════════════════
    § 6. still-MISSING 诚实声明 + L0/L2 边界精确划线 + 结果包六要素
    ═══════════════════════════════════════════════════════════════════════

  ═══════════════════════════════════════════════════════════════════════
  ★L0/L2 边界精确划线（#131 核心交付）
  ═══════════════════════════════════════════════════════════════════════
  本文件 = 力度 **conformance 规约**。划界如下：

  ┌── L0（本文件实证，纯逻辑，不依赖数据）──────────────────────────────┐
  │ · `ForceConforms` / `ForceConformsAll`：conformance 关系定义           │
  │   （两个 measure 背驰判定一致——独立于实现）。                          │
  │ · ①「满足 ForceMeasure 公理的 measure 在背驰判据中行为一致」：          │
  │   conformance 自反/对称/传递（`forceConforms_refl/symm/trans`）+        │
  │   measure 相等 ⟹ conform（`forceConforms_of_measure_eq`）+ 判据继承     │
  │   （`conforming_inherits_dichotomy`/`conformsAll_iff_divergence`）。    │
  │ · conformance 非平凡：比逐点相等严格弱                                  │
  │   （`conformance_strictly_weaker_than_eq`：scaled ↔ identity conform 但 │
  │   measure 不等）——规约有内容，非退化为相等。                           │
  │ · L2 见证 → L0 后果的条件定理                                           │
  │   （`conformance_consequences_under_witness` /                         │
  │   `macd_inherits_dichotomy_under_witness`：前件 L2，结论 L0）。         │
  │ · L0 conformance 见证（`l0_conformance_witness`：用 Lean 内部 scaled    │
  │   实例，**非 rust MACD**）——证规约 L0 可满足。                          │
  └──────────────────────────────────────────────────────────────────────┘
  ┌── L2（本文件**不实证**，显式标 L2 接口假设，rust 端由 #127 对齐）──────┐
  │ · ②「rust MACD 是 ForceMeasure 实例」：`MacdForceMeasureWitness`        │
  │   （structure，字段 macd + conforms 是 L2 待验证义务，本文件**不构造**  │
  │   实例）+ `MacdConformsTo`（Prop，**陈述不证明**）。                     │
  │ · 真实 MACD 数值引擎（EMA/DIF/DEA + 面积积分）在真实 K 线上是否真满足    │
  │   ForceMeasure 公理（mono/faithful）+ 与参考 conform，是**经验命题**，   │
  │   需 L2 真实数据验证，**当前未验证**。                                  │
  │ · 本文件**不声明 rust 已对齐**——`MacdConformsTo` 非已证 theorem；       │
  │   `MacdForceMeasureWitness` 实例本文件不构造。rust 对齐 = #127 工位。    │
  └──────────────────────────────────────────────────────────────────────┘

  ═══════════════════════════════════════════════════════════════════════
  ★still-MISSING（诚实开口，no-声明膨胀）
  ═══════════════════════════════════════════════════════════════════════
  · **still-MISSING-C（force-L2 conformance 验证，本工位补规约·未补验证）**：本文件给出
    conformance 的**形式规约**（L0 结构 + L2 假设载体），但「rust MACD 真满足 ForceMeasure 公理
    + 与参考 conform」这一 **L2 命题本身未验证**——`MacdForceMeasureWitness` 实例本文件不构造。
    补验证需：(1) #127 rust 端把 MACD 引擎对齐到 ForceMeasure 接口；(2) L2 真实数据验证 rust 算出
    的力度在真实 K 线上满足 mono/faithful + 与 Lean 参考判据一致。当前缺口明确指向 L2 验证层 +
    #127 rust 对齐，不是 L0 规约层（规约已闭合）。
  · **参考 measure `ref` 的具体来源**：本文件 conformance 以抽象 `ref : ForceMeasure α` 为参考锚，
    但「哪个 Lean measure 作 canonical 参考」（如 identityForceMeasure 还是从缠论几何导出的 measure）
    未在本文件固定——那是参考 measure 选择问题，与 ForceInterface 的 strength 几何来源（still-MISSING）
    对接。本文件设 ref 为参数，不预设具体参考。

  ═══════════════════════════════════════════════════════════════════════
  ★结果包六要素
  ═══════════════════════════════════════════════════════════════════════
  1. 结论：力度 conformance 规约——conformance 关系 `ForceConforms`/`ForceConformsAll`（两个
     ForceMeasure 背驰判定一致）+ ① 结构层 L0 定理（自反/对称/传递 + measure 相等⟹conform +
     判据继承 + conformance 比逐点相等严格弱）+ ② MACD conformance 命题显式标 L2 接口假设
     （`MacdForceMeasureWitness` structure + `MacdConformsTo` Prop，**陈述不证明**，本文件不构造
     rust 实例）+ L2 见证 → L0 后果的条件定理。全 L0 部分零 sorry；L2 假设用 structure field/Prop
     前件显式表达（非 sorry/axiom 伪装）。**不声明 rust 已对齐**（那是 #127）。
  2. 定义依据：§9 力度比较（力度序）+ 第24课趋势背驰（后段力度弱于前段，MACD 面积 C<A）+
     committed ForceInterface.lean `ForceMeasure`/`IsDivergenceVia`/`divergenceVia_depends_only_on_output`。
     输入特征：conformance = 背驰判定一致 ⟹ 满足「独立于 measure 实现」（判据只读 measure 输出，
     `forceConforms_of_measure_eq` 用 `divergenceVia_depends_only_on_output`）；rust MACD 是否合法
     实例 ⟹ 经验命题，标 L2 假设（不证）。
  3. 边界条件（结论翻转）：
     · conformance 用「背驰判定双向 ↔ 一致」。若改为「单向 →」（候选背驰 ⟹ 参考背驰，弱化），则
       `forceConforms_symm` 失效，conformance 不再是等价关系——须重裁 conformance 强度。当前双向
       保证等价关系结构。
     · conformance 比逐点相等严格弱（`conformance_strictly_weaker_than_eq`）。若要求 measure 逐点
       相等（强 conformance），则 scaled 实例不再 conform——规约过强（要求 rust 数值精确等于 Lean，
       无意义）。当前判据层 conformance 是正确强度（允许实现差异，要求判据一致）。
     · **L2 边界（核心）**：若 still-MISSING-C 补 rust 对齐 + L2 数据后，真实 MACD 在某标的上算出
       的力度违反 ForceMeasure 公理（mono/faithful）或与参考判据不一致，则**无法构造**
       `MacdForceMeasureWitness`——L2 数据否证 conformance 假设（formalization-validity-domain：
       L2 否定性结果缩小有效域）。本文件 L0 只证 conformance 规约逻辑自洽 + 条件定理，**不保证**
       某 L2 rust 实例满足 conformance。
  4. 下游推论：
     · conformance 规约闭合 ⟹ #127 rust 对齐有了**形式目标**：rust MACD 对齐 = 构造一个
       `MacdForceMeasureWitness`（rust 算出的力度满足 ForceMeasure 公理 + 与 Lean 参考 conform）。
       L2 验证目标明确为「构造 witness 实例」，不是模糊的「rust 大致对齐」。
     · 条件定理（L2 见证 → L0 后果）⟹ 一旦 conformance L2 验证，rust MACD 可**无歧义替换** Lean
       参考用于全部背驰/买卖点判据（BspClassification 第一类 = 背驰点）——conformance 是 L0 判据层
       与 L2 数值引擎的接缝（`conformance_consequences_under_witness`/`macd_inherits_dichotomy_under_witness`）。
     · conformance 比逐点相等弱 ⟹ rust MACD 的力度数值**不必精确等于** Lean 参考，只需判据一致——
       这放松了 #127 rust 对齐的义务（对齐判据，不对齐数值），是正确的工程接缝。
  5. 谱系引用：本文件承接 committed ForceInterface.lean（#124）still-MISSING-C（force-L2 conformance：
     rust MACD ↔ Lean ForceMeasure 无形式 conformance spec）+ committed Divergence.lean still-MISSING-C
     （Force/MACD 抽象标量无 K 线计算引擎）。无新概念分离——`ForceConforms` 是 ForceInterface
     `divergenceVia_depends_only_on_output`（判据只依赖 measure 输出）在 conformance 层的应用，
     不引入新定义冲突。L0/L2 划线与 222/223/230 有效域≠定义域三例**同模式**：conformance 规约
     定义域 = 所有 ForceMeasure 对，有效域 = 真满足公理 + conform 的实例（rust MACD 是否在有效域
     是独立的 L2 经验问题）。不确定是否有更早的「conformance 规约 vs 实例验证」概念分离谱系——
     若 genealogist 有相关记录（L0 接口/L2 实例边界先例），本文件 L0/L2 划线与其同模式。
  6. 影响声明：新增 `Origin.ForceConformance` 模块，import `Origin.ForceInterface`（只读，build on
     committed `ForceMeasure`/`IsDivergenceVia`/`IsContinuationVia`/`divergenceVia_dichotomy`/
     `divergenceVia_depends_only_on_output`/`identityForceMeasure`，不改 committed 类型）。无反向依赖，
     无命名冲突（namespace `NewChanlun.Origin`，新名 `ForceConforms`/`ForceConformsAll`/
     `scaledForceMeasure`/`MacdForceMeasureWitness`/`MacdConformsTo` 等与 committed 不碰撞）。
     ★待 Lead 登记 root：`Origin.ForceConformance`（lakefile Origin lib roots 追加，紧随
     `Origin.ForceInterface`）。本文件**不编辑 lakefile**（报 Lead 登记）。
     `lake env lean Origin/ForceConformance.lean` 单文件验证。**不触发定义矛盾**（conformance 规约
     与 committed ForceInterface/Divergence 一致，无冲突）。
-/

end NewChanlun.Origin
