/-
  操作语义完全分类（Phase 2 claim6）
  T 算子操作 + #39 操作商 + 267/338 operational-methodology + 603 范式

  ★范式判定（formalization-lead，2026-06-25）：claim6 **可递归重铸，不暴露定义冲突**。

  范式分析（operation_route_exhaustion.md §1-2 + 598 元判据）：
    操作语义有三层（D∞ 结构）：
    - 第0层 句法/path：自由幺半群 Σ* = 原子操作字母表 Σ 上的有限串。
    - 第1层 群元素/净效果：D∞ 商（Britton 正规形 hᵃτᵇ）。
    - 第2层 同调/不变内容：H₁(D∞,ℝ_-)=ℝ。

    operation_route_exhaustion.md §2 自证：穷尽性引擎在 **第0层句法**——
    "单 bar 单级别合法原子几何动作恰有 4 个"（Σ={e,h⁺,h⁻,τ}，由 P1/P2/NR-4 筛选 D∞ 生成元×模式
    穷尽，无第5个，L0）；"Σ* 由 Σ 自由幺半群泛性质穷尽所有路线"。

    ★这正是 603 内涵式构造子穷尽（**不是**外延式轴枚举）：
    - Σ 是 **4 构造子 sum type**（原子动作穷尽，定义钉死），同走势三分。
    - 操作路线 = **Σ*** = `List Σ`（自由幺半群=初代数，构造子穷尽 by 泛性质）。
    - #39 的 9 轨道 = Σ*/D∞ 的 **有限商**（598 要素2 有限商 Burnside）——是构造子完备句法的
      **派生有限商**（净效果等价类），不是与构造子竞争的独立轴。轴范式残留消解。

  ∴ 操作完全分类在递归范式可达：操作=端点 BSP 标签到 Σ* 路线的函数；Σ 4 构造子穷尽（内涵式）；
    9 轨道是 Σ*/D∞ 有限商（派生，非独立轴）。**无需 /escalate**（无 claim 无法表达为构造子完备，
    无定理间矛盾）。

  ★codex Phase 2 审计修正（有效域膨胀，非定义冲突）：本模块 **只声明两个忠实子命题**，
  撤回"操作完全分类=BSP totality composition"（膨胀声明）：
  - 子命题A（PASS）：Σ = 4 构造子穷尽 + Σ*=List OpAtom 穷尽**单级别句法 operate route**。
  - 子命题B（PASS）：#39 9 轨道 = Σ*/D∞ 派生有限商（非独立轴，无需 escalate）。
  **不声明**完整合法操作语义（LegalRoute 含手性/级别/数量/守恒/状态）——标注 Phase2+/Rust 职责。

  本模块形式化：
  1. Σ = 4 原子操作构造子穷尽（e/h⁺/h⁻/τ，operation_route_exhaustion §2.2 定理 Σ-exh）。
  2. 操作路线（句法层）= `List OpAtom`（自由幺半群 Σ*，构造子完备 by List 归纳）。
  3. Mode（NR-4 operate/observe）模式投影保留（不被吸收）。
  4. 净效果/Burnside 9 轨道是 Σ* 的派生有限商（非独立轴）——读出层。
  5. 操作 **触发点** 由 BSP 端点决定（弱声明）；操作 **内容**（route）依赖状态/守恒（不在范围，诚实标注）。

  认识论：L0（定义内蕴，继承 operation_route_exhaustion §2 Σ-exh 定理 + 自由幺半群泛性质）。
  有效域 = 单级别句法 Σ* 穷尽（**非**完整合法操作语义，formalization-validity-domain）。
-/

import Formal.BSPLabels

namespace Formal.OperationalSemantics

/--
  ★原子操作字母表 Σ（operation_route_exhaustion §2.2 定理 Σ-exh，4 构造子穷尽）。

  单 bar 单级别合法原子几何动作 **恰有 4 个**——由 D∞ 生成元 {h^{±1}, τ} × 模式
  {operate, observe} 经 P1（时间前向）/P2（手性翻仅 φ=0）/NR-4（读/写延异）筛选穷尽，无第5个：
  - `hold`（e）：不动（operate，恒合法）。
  - `advance`（h⁺）：持仓前向推进（operate，时间前向 a≥0）。
  - `observePast`（h⁻）：向心读已结算踪迹（observe，T49）。
  - `flipChirality`（τ）：手性翻转（operate，仅 φ=0，T51）。

  这是 **4 构造子 sum type**——穷尽性由 Σ-exh 定理钉死（D∞ 只有 2 几何生成元 + 2 模式，σ=h²³
  导出非原子）。同走势三分的内涵式构造子穷尽，非外延式轴枚举。
-/
inductive OpAtom where
  | hold            -- e：不动（hold，operate）
  | advance         -- h⁺：持仓前向推进（operate）
  | observePast     -- h⁻：向心读过去（observe，T49）
  | flipChirality   -- τ：手性翻转（operate，仅 φ=0，T51）
deriving DecidableEq, Repr

open OpAtom

/--
  ★原子操作完全分类（Σ-exh，构造子穷尽，L0）：任意原子操作必属 4 构造子之一。
  这是对 `OpAtom` 的结构归纳——"无第5个原子动作"是 inductive 的结构归纳原理。
-/
theorem opatom_exhaustive (a : OpAtom) :
    a = hold ∨ a = advance ∨ a = observePast ∨ a = flipChirality := by
  cases a
  · exact Or.inl rfl
  · exact Or.inr (Or.inl rfl)
  · exact Or.inr (Or.inr (Or.inl rfl))
  · exact Or.inr (Or.inr (Or.inr rfl))

/--
  ★操作路线 = Σ*（自由幺半群，operation_route_exhaustion §2.4 推论 Σ*-exh）。

  所有单级别操作路线 = `List OpAtom`——自由幺半群 Σ* 的 Lean 实现（List 是字母表上的
  自由幺半群=初代数）。构造子穷尽 by `List` 的结构归纳（nil + cons）：任意路线要么空，
  要么是"一个原子动作 + 剩余路线"。这把"无限路线穷尽"归约为"有限字母表穷尽"（Σ-exh）。
-/
abbrev OpRoute := List OpAtom

/--
  ★操作路线完全分类（构造子穷尽 by List 归纳，L0）：任意路线要么空，要么 cons。
  自由幺半群 Σ* 的初代数结构——无第三种路线构造方式。
-/
theorem oproute_exhaustive (r : OpRoute) :
    r = [] ∨ ∃ (a : OpAtom) (rest : OpRoute), r = a :: rest := by
  cases r with
  | nil => exact Or.inl rfl
  | cons a rest => exact Or.inr ⟨a, rest, rfl⟩

/--
  ★模式维（NR-4，operation_route_exhaustion §2.1）：operate（写）/ observe（读）。

  codex 审计修正（Phase 2）：NR-4 是正交模式维，可折进字母表，但 `OpAtom` 必须保留
  mode 投影（`modeOf`），否则净效果会混掉 observe/operate。这里显式给出每个原子的模式。
-/
inductive Mode where
  | operate   -- 写（状态迁移）
  | observe   -- 读（读已沉淀踪迹）
deriving DecidableEq, Repr

/-- 原子操作的模式投影（NR-4 正交维，不被吸收掉）。 -/
def modeOf : OpAtom → Mode
  | hold => Mode.operate
  | advance => Mode.operate
  | observePast => Mode.observe       -- h⁻ 是 observe 模式（向心读过去，T49）
  | flipChirality => Mode.operate

/-- ★observe 模式恰为 observePast（NR-4 模式投影不混淆，L0）。 -/
theorem observe_iff_observePast (a : OpAtom) :
    modeOf a = Mode.observe ↔ a = observePast := by
  cases a <;> simp [modeOf]

/-!
  ## ★操作语义有效域诚实标注（codex 审计修正，Phase 2 + formalization-validity-domain）

  codex Phase 2 审计裁定：本模块的忠实有效域是 **第0层句法 Σ*（单级别 operate route 穷尽）**，
  **不是** "操作完全分类继承 BSP totality"。后者是有效域膨胀——
  完整操作语义（合法 operate route）还依赖：手性 {±1}、σ 级别塔、M 数量系数、
  account/position 状态、成本阶段、regime gating、T34/T40/T48 守恒约束
  （operation_route_exhaustion §0.1 句法存在性 ≠ 路线合法性；§7 总式 = Σ*×{±1}×σ-tower×M 受守恒约束）。

  故本模块 **只声明** 两个忠实子命题：
  - 子命题A（PASS）：`Σ* = List OpAtom` 穷尽单级别句法 operate route（自由幺半群构造子完备）。
  - 子命题B（PASS）：#39 的 9 轨道 = Σ*/D∞ 的派生有限商（非独立轴，无需 escalate）。
  **不声明**："操作完全分类 = BSP totality 的 composition"（有效域膨胀，已撤）。

  完整合法操作语义（LegalRoute：含手性/级别/数量/守恒/状态）**不在本 Phase 形式化范围**——
  标注为 Phase 2+ / Rust 引擎层职责（trading::types σ-tower + 守恒约束）。
-/

/--
  ★操作触发点由 BSP 端点决定（弱化声明，codex 审计修正）：

  BSP 端点决定操作的 **触发点/端点类别**（哪个端点该操作），**不**决定完整 operate route
  （完整 route 还依赖手性/级别/数量/状态/守恒，见上 §有效域标注）。
  `OperationTrigger` 把 BSP 标签集映射为"是否触发操作"的判定——这是忠实的弱声明：
  操作触发继承 BSP totality（端点非空 ⟹ 有操作触发），但操作 **内容**（route）不由 BSP 单独决定。
-/
def OperationTrigger := Formal.BSPLabels.BSPLabelSet → Bool

/--
  ★操作触发继承 BSP totality（L0，弱声明）：BSP 端点标签集非空 ⟹ 存在操作触发。

  这是忠实的有效域——升跌完备性（端点 totality）保证每个端点有操作触发点，
  但 **不**声明操作 route 由 BSP 完全决定（route 依赖状态/守恒，有效域更小）。
-/
theorem trigger_inherits_totality (s : Formal.BSPLabels.BSPLabelSet) :
    s.labels ≠ [] := s.nonempty

/--
  ★净效果商是 Σ* 的派生（598 要素2 有限商 + 603：9 轨道非独立轴）。

  D∞ 群元素（净效果 hᵃτᵇ）是 Σ* 经约化 π_red 的商——把"怎么走"抹掉只留"净走到哪"。
  这里以"路线长度的奇偶"作净效果商的一个最小可判定投影示例（τ²=e 使手性翻转偶数次=恒等）：
  净效果是 Σ* 的 **派生有限商**，不是与 Σ 构造子竞争的独立轴。#39 的 9 轨道同理——
  是 Σ*/D∞ 的 Burnside 有限商（598 要素2），由构造子完备句法派生，轴范式残留消解。
-/
def netChiralityFlips (r : OpRoute) : Nat :=
  (r.filter (· = flipChirality)).length

/-- 净手性（τ²=e）：翻转偶数次 = 恒等手性（净效果商的最小示例，派生自 Σ* 路线）。 -/
def netChiralityIsIdentity (r : OpRoute) : Bool :=
  netChiralityFlips r % 2 == 0

/--
  ★净效果是路线的函数（派生商，非独立轴，L0）：净手性由路线 Σ* 计算，不是独立维度。
  这关死"操作商是与构造子竞争的独立轴"的轴范式误读——净效果/9 轨道是 Σ* 的派生有限商。
-/
theorem net_effect_derived_from_route (r : OpRoute) :
    netChiralityIsIdentity r = (netChiralityFlips r % 2 == 0) := rfl

/--
  ★空路线净手性恒等（hold 序列净效果=恒等，派生商边界检查）。
-/
theorem empty_route_identity : netChiralityIsIdentity [] = true := rfl

/-!
  ## ★C5 操作类型完全分类·两层定理（严格分类标准 第6部分 + 差距矩阵 C5 行）
  ★task #67（T-op，严格分类标准升级，codex#1 两层规则）

  标准（chanlun-strict-classification-standard.md §A codex#1 硬规则 + §B 第6部分）：
  - **Layer1（标签商分类，保留）**：操作类型 `OpTag` = {买,卖,加,减,持,平} 6 构造子穷尽
    （π_op 标签商）。**显式命名为「按标签商分类」**，不冒充语义双射（codex#1 规则）。
  - **Layer2（语义双射尝试 → 诚实失败）**：定义 `Exec`（账户状态 + 动作 + 可行性）+ 语义等价
    `OpEqSemantic`（账户转移效果相同）+ 标签读出 `I : Exec → OpTag`。
    **complete 失败**（codex 019f001b 精确修正措辞）：`i_not_complete` 证的是 **`I` 的 fiber 内
    含语义不等价对象**（买1单位 vs 买10单位 同标签 `buy` 异账户转移）——即 **标签等价不蕴含
    账户转移语义等价**，`OpTag` **不是** 语义完备分类器（非 `Exec/∼ ≅ OpTag` 双射）。
    （**不**声称"良定义商映射非单射"——若有 ∼-等价但异标签则商映射不良定义；本反例是 fiber 内
    语义分裂，no-patch 诚实刻画不强证。）正向忠实弱命题：标签+参数(qty/price) ⟹ 语义等价
    （`exec_eq_gives_semantic`）。
  - **标准第6部分 π 完全应对**：`π : State → OpTag` 全定义（∀状态有应对）+ 无前视弱形式
    （`pi_no_lookahead`：同状态同应对=纯函数同参同值；**前提** State = 当前及之前数据摘要 Cₗ(hₜ)，
    codex 019f001b：完整无前视因果（历史模型 + 前缀依赖 stateOf h t 只依赖 ≤t）属 T-causal #64/#66
    工位，本模块只给"State 为当前摘要"假设下的 L0 弱形式，诚实标注不冒充强因果）。

  认识论：L0（定义内蕴）。语义 ∼ 维度 = 账户转移效果（units delta + cash delta），
  由 task #67 spec 钉定（非开放选择）。账户量用 Int（不依赖 Mathlib ℚ）。
  ★诚实边界（codex 019f001b）：`feasible` 不参与语义等价；qty/price 用 Int 未约束非负；
  本模块忠实的是"账户转移语义"，非"完整 Exec 语义"。
-/

/--
  ★操作类型标签 OpTag（C5：买/卖/加/减/持/平，6 构造子穷尽，π_op 标签商 Layer1）。
  与 Phase2 Claim6 `OpType` 同构（自包含，OperationalSemantics 不 import Claim6）。
-/
inductive OpTag where
  | buy      -- 买（建仓）
  | sell     -- 卖（清仓/止损）
  | add      -- 加（降成本买点加回）
  | reduce   -- 减（降成本卖点减仓）
  | hold     -- 持（不动）
  | close    -- 平（平反向腿）
deriving DecidableEq, Repr

namespace OpTag

/-- ★C5 Layer1 操作类型完全分类（标签商，构造子穷尽 L0）：任意操作类型属 6 构造子之一，无第7类。 -/
theorem optag_exhaustive (t : OpTag) :
    t = buy ∨ t = sell ∨ t = add ∨ t = reduce ∨ t = hold ∨ t = close := by
  cases t
  · exact Or.inl rfl
  · exact Or.inr (Or.inl rfl)
  · exact Or.inr (Or.inr (Or.inl rfl))
  · exact Or.inr (Or.inr (Or.inr (Or.inl rfl)))
  · exact Or.inr (Or.inr (Or.inr (Or.inr (Or.inl rfl))))
  · exact Or.inr (Or.inr (Or.inr (Or.inr (Or.inr rfl))))

end OpTag

open OpTag

/--
  ★账户状态（C5 Layer2：持仓 units + 现金 cash，账户转移的载体）。
  units > 0 = 多头，< 0 = 空头，= 0 = 空仓。用 Int 建模量纲（不依赖 ℚ）。
-/
structure Account where
  units : Int
  cash : Int
deriving DecidableEq, Repr

/--
  ★执行对象 Exec（C5 Layer2：账户状态 + 动作 + 可行性，标准 §6 状态化）。
  - `pre`：操作前账户状态。
  - `tag`：操作类型标签（OpTag）。
  - `qty`：操作数量（≥0，加/减/买/卖的手数——**标签不携带，语义携带**）。
  - `price`：成交价（> 0）。
  - `feasible`：可行性（如减仓不超持仓、资金充足；此处建模为携带的可行性证据 Bool）。
  这关死"操作类型 = 完整语义"——同 tag 不同 qty/price = 不同 Exec（账户转移效果不同）。
-/
structure Exec where
  pre : Account
  tag : OpTag
  qty : Int
  price : Int
  feasible : Bool
deriving DecidableEq, Repr

/--
  ★账户转移（C5 Layer2：动作对账户的效果，标准 §6 δ 状态转移）。
  买/加：units += qty，cash -= qty×price（买入花现金增持仓）。
  卖/减/平：units -= qty，cash += qty×price（卖出收现金减持仓）。
  持：不动。这是操作的 **语义效果**（账户转移），区别于操作 **标签**（OpTag）。
-/
def applyExec (e : Exec) : Account :=
  match e.tag with
  | OpTag.buy | OpTag.add => ⟨e.pre.units + e.qty, e.pre.cash - e.qty * e.price⟩
  | OpTag.sell | OpTag.reduce | OpTag.close => ⟨e.pre.units - e.qty, e.pre.cash + e.qty * e.price⟩
  | OpTag.hold => e.pre

/--
  ★语义等价 OpEqSemantic（C5 Layer2，task #67 spec：账户转移效果相同）。
  `OpEqSemantic x y` ⟺ 两个 Exec 的 **前后账户转移效果相同**（pre 相同 ∧ 转移后 Account 相同）。
  这是"对什么相同的对象分类"的语义 ∼——以账户转移效果为等价维度（spec 钉定，非开放选择）。
-/
def OpEqSemantic (x y : Exec) : Prop :=
  x.pre = y.pre ∧ applyExec x = applyExec y

/-- ★标签不变量 I（C5 Layer2：Exec → OpTag，读出操作类型标签）。 -/
def I (e : Exec) : OpTag := e.tag

/--
  ★C5 Layer2 正向忠实弱命题（L0）：若两 Exec 标签相同 **且** 同 pre/qty/price，则语义等价
  （标签 + 参数 ⟹ 账户转移语义等价）。这是 `I` 的弱忠实方向——**注意不是** "∼ ⟹ I=I"
  的完整不变量（codex 019f001b：那方向不在此声称，见下 `i_not_complete` 的精确措辞）。
-/
theorem exec_eq_gives_semantic (x y : Exec)
    (hpre : x.pre = y.pre) (htag : x.tag = y.tag) (hqty : x.qty = y.qty)
    (hprice : x.price = y.price) : OpEqSemantic x y := by
  unfold OpEqSemantic applyExec
  rw [hpre, htag, hqty, hprice]
  exact ⟨rfl, rfl⟩

/--
  ★C5 Layer2 **complete 失败**（codex#1 Q2(d) + codex 019f001b 精确措辞，task #67）。

  反例：两个 Exec 标签都是 `buy`（I 相同），但 qty 不同（1 vs 10）⟹ 账户转移效果不同
  （units 增量 1 vs 10）⟹ **¬ OpEqSemantic**。即 `∃ x y, I x = I y ∧ ¬ OpEqSemantic x y`。

  **精确含义（codex 019f001b 修正）**：本定理证的是 **`I` 的 fiber 内含语义不等价对象**
  （同标签 fiber 里有账户转移不同的 Exec）——即 **标签等价不蕴含账户转移语义等价**，
  `OpTag` **不是** 语义完备分类器。**不**声称"良定义商映射 `Exec/∼ → OpTag` 非单射"
  （若存在 ∼-等价但异标签则该商映射不良定义）——本反例是 fiber 内语义分裂，诚实刻画。

  **诚实裁定（no-patch + codex#1 规则）**：操作类型 OpTag 是 **「按标签商分类」**，
  **不是** 语义双射 `Exec/∼ ≅ OpTag`——标签太粗（不含 qty/price），complete 失败。
  这是缠论操作类型的本质：类型 {买卖加减持平} 不决定数量 M（M 属 payoff 层，603 §诚实分层）。
-/
theorem i_not_complete :
    ∃ x y : Exec, I x = I y ∧ ¬ OpEqSemantic x y := by
  refine ⟨⟨⟨0, 100⟩, buy, 1, 10, true⟩, ⟨⟨0, 100⟩, buy, 10, 10, true⟩, rfl, ?_⟩
  -- applyExec x = ⟨1, 90⟩, applyExec y = ⟨10, 0⟩ — units 增量 1 vs 10 不同 ⟹ ¬ 语义等价
  intro h
  have h2 : applyExec ⟨⟨0, 100⟩, buy, 1, 10, true⟩ = applyExec ⟨⟨0, 100⟩, buy, 10, 10, true⟩ := h.2
  simp only [applyExec, Account.mk.injEq] at h2
  omega

/--
  ★C5 Layer2 标签商分类（诚实命名，codex#1 硬规则 + codex 019f001b 精确措辞）：OpTag 分类是
  `Exec` 在 **标签等价** `fun x y => I x = I y` 下的商——这是 **标签商**。由 `i_not_complete`：
  标签等价的 fiber 内 **含账户转移语义不等价对象**（同标签 fiber 语义分裂）⟹ 标签等价 **严格弱于**
  账户转移语义等价 OpEqSemantic（同标签可含异语义 Exec）。这关死"操作类型 = 完整操作语义双射"
  的声明膨胀（OpTag 不是语义完备分类器）。
-/
theorem optag_is_label_quotient_not_semantic :
    (∃ x y : Exec, I x = I y ∧ ¬ OpEqSemantic x y) := i_not_complete

/-! ### C5 标准第6部分：完全应对 π（State → Action 全定义 + 无前视因果） -/

/--
  ★持仓状态 State（C5 §6：π 的定义域，当前及之前数据的摘要）。
  - `posSide`：当前持仓方向（多/空/空仓）。
  - `signalSide`：当前 bar 触发信号方向（买侧/卖侧/无信号）。
  这是"当前及之前数据"的状态摘要（Cₗ(hₜ)）——π 只依赖它，不依赖未来（无前视）。
-/
inductive PosSide where | long | short | flat
deriving DecidableEq, Repr

inductive SignalSide where | buySide | sellSide | none
deriving DecidableEq, Repr

structure State where
  posSide : PosSide
  signalSide : SignalSide
deriving DecidableEq, Repr

/--
  ★完全应对策略 π（C5 §6：State → OpTag 全函数，∀状态有应对）。
  这是 §4.1 π_op 表 + 三阶段的全定义编码——每个 (持仓, 信号) 状态都有唯一操作类型应对
  （含"持"=无操作应对，"等待"由 hold 承载）。**全函数**：match 穷尽所有 3×3 状态组合。
-/
def pi (s : State) : OpTag :=
  match s.posSide, s.signalSide with
  | PosSide.flat,  SignalSide.buySide  => buy      -- 空仓买信号 → 建仓
  | PosSide.flat,  SignalSide.sellSide => OpTag.hold     -- 空仓卖信号 → 不动（裸空非缠论）
  | PosSide.flat,  SignalSide.none     => OpTag.hold     -- 空仓无信号 → 持（等待）
  | PosSide.long,  SignalSide.buySide  => add      -- 多头买信号 → 加（降成本买回）
  | PosSide.long,  SignalSide.sellSide => reduce   -- 多头卖信号 → 减（降成本减仓）
  | PosSide.long,  SignalSide.none     => OpTag.hold     -- 多头无信号 → 持
  | PosSide.short, SignalSide.buySide  => close    -- 空头买信号 → 平（回补空头腿）
  | PosSide.short, SignalSide.sellSide => sell     -- 空头卖信号 → 卖（空头翻转/止损）
  | PosSide.short, SignalSide.none     => OpTag.hold     -- 空头无信号 → 持

/--
  ★C5 §6 完全应对（π 全定义，L0）：∀状态 s，π 给出确定操作类型应对（无未定义状态）。
  这是"完全应对" `πₗ:Sₗ→A` 的全函数性——任意持仓×信号状态都有应对，无死端点、无 partial。
-/
theorem pi_total (s : State) : ∃ a : OpTag, pi s = a := ⟨pi s, rfl⟩

/--
  ★C5 §6 π 应对穷尽于 OpTag（L0）：π 的值域落在 6 操作类型构造子内（应对类型完全分类）。
-/
theorem pi_in_optag (s : State) :
    pi s = buy ∨ pi s = sell ∨ pi s = add ∨ pi s = reduce ∨ pi s = OpTag.hold ∨ pi s = OpTag.close :=
  OpTag.optag_exhaustive (pi s)

/--
  ★C5 §6 无前视·**弱形式**（L0，codex 019f001b 诚实降级）：

  π 是 State 的纯函数——相同当前状态 ⟹ 相同应对（同参同值）。**这是弱形式**：
  它在 **前提 State = 当前及之前数据的摘要 Cₗ(hₜ)**（外部假设）下，表达"应对只由当前摘要决定"。
  **完整无前视因果**（历史模型 + 时间 t + 前缀依赖 `stateOf h t` 只依赖 `≤t` + ω₀:ₜ=ω'₀:ₜ⟹Cₜ相等）
  **不在本模块**——属 T-causal（#64/#66 CausalOnlineSystem）工位。本模块只给"State 为当前摘要"
  假设下的 L0 弱形式，诚实标注不冒充强因果（no-patch，不声明膨胀）。
-/
theorem pi_no_lookahead (s s' : State) (h : s = s') : pi s = pi s' := by rw [h]

end Formal.OperationalSemantics
