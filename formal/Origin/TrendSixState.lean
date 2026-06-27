/-
Origin/TrendSixState.lean — 走势六态 r + 信号位向量 b∈{0,1}⁶（L2-A，FULL §5 对照补全）

★工位定位（L2-A 补全工位，cov-classification B4/B5 缺）：`Origin.CenterStates` 只把位置
  形式化为**三态** `CenterPosition`（below/within/above），`Origin.BspClassification` 把买卖点
  形式化为**互斥 sum** `BspClass`（one/two/three）+ 非互斥裁定 `no_exclusive_trichotomy`。但
  FULL_USER_FORMULA_SOURCE.md §5（line 277-308）的 canonical 表示是两个**不同构**的对象：
    (1) 走势六态 r ∈ {⊥,I,U⁰,U¹,D⁰,D¹}（位置三态 × 第三类标记位的**真扩展**为六态）；
    (2) 信号位向量 b ∈ {0,1}⁶（六个独立 bool，三类买卖点**不强制互斥**——maimai 2B/3B 可重合）。
  rust 侧 `classifier/level_state.rs::RLevel` 已实装六态，`types.rs::BspBits` 已实装 {0,1}⁶，
  但 Lean 侧**无**六态 r 的 inductive + ∃! 互斥穷尽，**无** {0,1}⁶ 位向量的真 6 维非塌缩证明。
  本文件补这两个 canonical 形式化 + parity 锚点（rust six_state.rs include 同一见证）。

═══════════════════════════════════════════════════════════════════════════
权威来源（对照结果包原文 + 三级权威链）
═══════════════════════════════════════════════════════════════════════════
- FULL_USER_FORMULA_SOURCE.md §5（line 277-308，对照结果包，gpt canonical）：
  · r_{ℓ,t} ∈ {⊥,I,U⁰,U¹,D⁰,D¹}：
      ⊥ = 无确认中枢；I = 中枢内；
      U⁰ = 中枢上方·无三买；U¹ = 中枢上方·已有三买；
      D⁰ = 中枢下方·无三卖；D¹ = 中枢下方·已有三卖。
  · b_{ℓ,t} = (B₁,B₂,B₃,S₁,S₂,S₃) ∈ {0,1}⁶：「三类买卖点不能强行互斥，因此定义信号位向量」。
- 第49课（位置三态，博文最终权威）：「当下在该中枢之中(ZG-ZD)/之下(<ZD)/之上(>ZG)」。
  本文件六态 r 在三态基础上**扩展**：位置三态是 r 的几何骨架，⊥/三买标记/三卖标记是
  Θ_signal 参数化的精化维度（非把三态硬塞成六态，见 §1 扩展论证）。
- maimai.md:58/65/66/170-176（信号位非互斥律，三级权威链第3级第三方总结 + 第21课溯源）：
  · 「2B 与第 3B 可以重合（V型反转）」「1B 与 2B 不可能重合」「1B 与 3B 不可能重合」。
  · ⟹ 信号位向量 b 真 6 维（2B/3B 可同时为 1），不是互斥 sum（塌缩为单标签是错误）。

═══════════════════════════════════════════════════════════════════════════
认识论等级（formalization-validity-domain 231号，强制标注）
═══════════════════════════════════════════════════════════════════════════
全部 **L0**（纯定义 / 整数三歧 / 枚举穷尽 / 结构推导，不依赖数据，omega/decide machine-checked）。
`lake env lean Origin/TrendSixState.lean` 通过 = 六态 r 的互斥穷尽 ∃! 与信号位 b 的真 6 维
非塌缩**在定义层成立**，**不是**「六态/信号位在真实行情上有效」的实证断言（那是 L2/L3）。

★关键诚实点（两分类代数性质相反，对齐 level_state.rs 模块头）：
  - 六态 r 是 **partition**（互斥穷尽 Σ𝟙=1）⟹ inductive sum type + ∃! 唯一标签。
  - 信号位 b 是 **subset**（2/3 类可共存）⟹ {0,1}⁶ bit-vector，**不**是 sum type。
  把 b 做成互斥 sum 是 canonical 错误（no_exclusive_trichotomy 已证 sum 在 x_2b3b 失败）。

★边界条件（结论翻转，见文件尾 §6）。
禁 sorry/admit/axiom。纯 Prop/Type，不依赖 Mathlib。omega 前须 `simp only [..., Tick]` 暴露 Int。
-/

import Origin.ChanlunElements
import Origin.CenterStates
import Origin.BspClassification

namespace NewChanlun.Origin

/-! ═══════════════════════════════════════════════════════════════════════
    § 1. 走势六态 r（FULL §5）：⊥/I/U⁰/U¹/D⁰/D¹ —— 位置三态的真扩展
    ═══════════════════════════════════════════════════════════════════════

  ★扩展论证（no-patch：六态是位置三态的真扩展，非硬塞）：
    位置三态 `CenterPosition`（below/within/above）是纯几何划分（点相对 [ZD,ZG]）。
    六态 r 在两个正交方向上扩展它：
    1. **新增 ⊥ 维度**：位置三态的定义域是「有中枢 + 一个点」，r 的定义域是「Option 中枢 + 点」。
       无中枢（`last_center = none`）映 ⊥——这是位置三态**无法表达**的状态（三态预设有中枢）。
    2. **above/below 各按第三类事件裂为两态**：
       - above（中枢上方）× 三买事件 b3 ⟹ {U⁰(无三买), U¹(有三买)}。
       - below（中枢下方）× 三卖事件 s3 ⟹ {D⁰(无三卖), D¹(有三卖)}。
       - within（中枢内）**不裂**（中枢内无「离开后回试」的第三类语境）⟹ 直接映 I。
    ⟹ 6 = 1(⊥) + 1(I=within) + 2(above×b3) + 2(below×s3)。这是 3 态在 Θ_signal 参数化下的
       严格精化（每个 above/below 态按可观测的第三类事件二分），不是把 3 个值改名成 6 个。
-/

/--
  **走势六态 r（FULL §5，line 281-291）** —— 走势相对最后确认中枢的位置态（六态 partition）。

  逐构造子对齐 FULL §5：
  - `bot`     ↔ ⊥：无确认中枢。
  - `insideZ` ↔ I：中枢内（[ZD,ZG]）。
  - `aboveNo3B` ↔ U⁰：中枢上方、无三买。
  - `aboveB3`   ↔ U¹：中枢上方、已有三买。
  - `belowNo3S` ↔ D⁰：中枢下方、无三卖。
  - `belowS3`   ↔ D¹：中枢下方、已有三卖。
  逐构造子 bit-exact 对齐 rust `classifier/level_state.rs::RLevel`
  （Bot/Inside/AboveNo3B/AboveB3/BelowNo3S/BelowS3）。
-/
inductive TrendSixState where
  | bot        -- ⊥：无确认中枢
  | insideZ    -- I：中枢内
  | aboveNo3B  -- U⁰：中枢上方、无三买
  | aboveB3    -- U¹：中枢上方、已有三买
  | belowNo3S  -- D⁰：中枢下方、无三卖
  | belowS3    -- D¹：中枢下方、已有三卖
deriving DecidableEq, Repr

/--
  **六态运行上下文（FULL §5；Θ-参数化运行输入，对齐 rust `RContext`）** ——
  - `lastCenter`：最后确认中枢 Z_ℓ（`none` = 无中枢 → ⊥）。哪个是「最后」由 Θ_parse 选取。
  - `price`：当下走势末端价 p（相对 Z_ℓ 核心区间 [ZD,ZG] 定位，§5 + 632号 Move.endPrice）。
  - `b3`：第三类买点事件是否成立（Θ_signal：离开中枢上破后回试不破 ZG，BspClassification.IsType3Buy）。
  - `s3`：第三类卖点事件是否成立（Θ_signal：IsType3Sell）。
-/
structure SixStateContext where
  lastCenter : Option Center
  price : Tick
  b3 : Bool
  s3 : Bool
deriving Repr

/--
  **六态判定函数（FULL §5，给定 Θ 后全函数）** —— 上下文 ↦ 六态标签。

  逐分支精化 `CenterStates.classifyPosition` 三态 × 第三类事件（bit-exact 对齐 rust `rlevel_of`）：
  - 无中枢 → bot；
  - 有中枢 c 按 `classifyPosition c p` 三态分流：
    · within → insideZ（不裂）；
    · above  → b3 ? aboveB3 : aboveNo3B；
    · below  → s3 ? belowS3 : belowNo3S。
-/
def classifySixState (ctx : SixStateContext) : TrendSixState :=
  match ctx.lastCenter with
  | none => TrendSixState.bot
  | some c =>
    match classifyPosition c ctx.price with
    | CenterPosition.within => TrendSixState.insideZ
    | CenterPosition.above =>
      if ctx.b3 then TrendSixState.aboveB3 else TrendSixState.aboveNo3B
    | CenterPosition.below =>
      if ctx.s3 then TrendSixState.belowS3 else TrendSixState.belowNo3S

/-! ═══════════════════════════════════════════════════════════════════════
    § 2. 六态 r 互斥穷尽 ∃!（partition 核心，FULL §5）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **★六态穷尽（L0，FULL §5）** —— 任一上下文必落六态之一（类型层穷尽，枚举无第七态）。
  `classifySixState` 是全函数 ⟹ 其值必是六构造子之一。
-/
theorem sixState_exhaustive (ctx : SixStateContext) :
    classifySixState ctx = TrendSixState.bot ∨
    classifySixState ctx = TrendSixState.insideZ ∨
    classifySixState ctx = TrendSixState.aboveNo3B ∨
    classifySixState ctx = TrendSixState.aboveB3 ∨
    classifySixState ctx = TrendSixState.belowNo3S ∨
    classifySixState ctx = TrendSixState.belowS3 := by
  rcases h : classifySixState ctx with _ | _ | _ | _ | _ | _
  · exact Or.inl rfl
  · exact Or.inr (Or.inl rfl)
  · exact Or.inr (Or.inr (Or.inl rfl))
  · exact Or.inr (Or.inr (Or.inr (Or.inl rfl)))
  · exact Or.inr (Or.inr (Or.inr (Or.inr (Or.inl rfl))))
  · exact Or.inr (Or.inr (Or.inr (Or.inr (Or.inr rfl))))

/--
  **★六态类型穷尽（L0，无第七态）** —— `TrendSixState` 恰有六构造子。与
  `TrendCompleteClassification.trend_no_fourth_class` / `BspClassification.bspType_no_fourth` 同构。
-/
theorem sixState_no_seventh (r : TrendSixState) :
    r = TrendSixState.bot ∨ r = TrendSixState.insideZ ∨
    r = TrendSixState.aboveNo3B ∨ r = TrendSixState.aboveB3 ∨
    r = TrendSixState.belowNo3S ∨ r = TrendSixState.belowS3 := by
  cases r
  · exact Or.inl rfl
  · exact Or.inr (Or.inl rfl)
  · exact Or.inr (Or.inr (Or.inl rfl))
  · exact Or.inr (Or.inr (Or.inr (Or.inl rfl)))
  · exact Or.inr (Or.inr (Or.inr (Or.inr (Or.inl rfl))))
  · exact Or.inr (Or.inr (Or.inr (Or.inr (Or.inr rfl))))

/--
  **★六态唯一标签 ∃!（L0，partition 核心，FULL §5）** —— 给定上下文，存在唯一的六态标签
  使 `classifySixState ctx = r`。这把六态 r 形式化为**确定性 partition**（每个上下文恰好一态）。
  互斥（两两不同标签不可同时是 classify 的输出）+ 穷尽（总有一个输出）= partition，∃! 是其凝聚。
-/
theorem sixState_unique (ctx : SixStateContext) :
    ExistsUnique (fun r => classifySixState ctx = r) := by
  refine ⟨classifySixState ctx, rfl, ?_⟩
  intro r hr
  exact hr.symm

/--
  **★六态互斥（L0，partition）** —— 六态两两不同（DecidableEq 枚举的构造子互不相等）。
  这保证 `classifySixState` 的输出落在某态时**不同时**是另一态（partition 的互斥半边）。
  ★这里证的是「标签互斥」（六个构造子两两 ≠），与穷尽合成 partition。
-/
theorem sixState_labels_distinct :
    TrendSixState.bot ≠ TrendSixState.insideZ ∧
    TrendSixState.bot ≠ TrendSixState.aboveNo3B ∧
    TrendSixState.aboveNo3B ≠ TrendSixState.aboveB3 ∧
    TrendSixState.belowNo3S ≠ TrendSixState.belowS3 ∧
    TrendSixState.aboveB3 ≠ TrendSixState.belowS3 := by
  refine ⟨?_, ?_, ?_, ?_, ?_⟩ <;> decide

/-! ═══════════════════════════════════════════════════════════════════════
    § 3. 六态与位置三态的扩展关系（重锚 CenterStates，no-patch 真扩展见证）
    ═══════════════════════════════════════════════════════════════════════ -/

/--
  **★within → I（L0，扩展关系）** —— 价格在中枢内（within）⟹ 六态为 I（insideZ），
  且**不受** b3/s3 影响（中枢内不裂第三类，对齐 rust `rlevel_within_ignores_b3_s3`）。
-/
theorem within_maps_inside (c : Center) (p : Tick) (b3 s3 : Bool)
    (h : classifyPosition c p = CenterPosition.within) :
    classifySixState { lastCenter := some c, price := p, b3 := b3, s3 := s3 }
      = TrendSixState.insideZ := by
  simp only [classifySixState, h]

/--
  **★above 按 b3 裂两态（L0，真扩展）** —— 价格在中枢上方（above）⟹ 六态按第三类买点
  事件 b3 二分：b3 ⟹ U¹(aboveB3)，¬b3 ⟹ U⁰(aboveNo3B)。这证明 above 单一三态被**真扩展**
  为两个六态（非改名），扩展维度 = Θ_signal 的三买事件。
-/
theorem above_splits_by_b3 (c : Center) (p : Tick) (s3 : Bool)
    (h : classifyPosition c p = CenterPosition.above) :
    classifySixState { lastCenter := some c, price := p, b3 := true, s3 := s3 }
      = TrendSixState.aboveB3 ∧
    classifySixState { lastCenter := some c, price := p, b3 := false, s3 := s3 }
      = TrendSixState.aboveNo3B := by
  refine ⟨?_, ?_⟩ <;> simp only [classifySixState, h] <;> rfl

/--
  **★below 按 s3 裂两态（L0，真扩展，above 的对偶）** —— below ⟹ s3 ? D¹ : D⁰。
-/
theorem below_splits_by_s3 (c : Center) (p : Tick) (b3 : Bool)
    (h : classifyPosition c p = CenterPosition.below) :
    classifySixState { lastCenter := some c, price := p, b3 := b3, s3 := true }
      = TrendSixState.belowS3 ∧
    classifySixState { lastCenter := some c, price := p, b3 := b3, s3 := false }
      = TrendSixState.belowNo3S := by
  refine ⟨?_, ?_⟩ <;> simp only [classifySixState, h] <;> rfl

/--
  **★⊥ 是真新增维度（L0，no-patch）** —— 无中枢（`lastCenter = none`）⟹ bot，**不论** price/b3/s3。
  这证明 ⊥ 是位置三态**无法表达**的状态（三态预设有中枢）——六态比三态多一个独立维度。
-/
theorem no_center_maps_bot (p : Tick) (b3 s3 : Bool) :
    classifySixState { lastCenter := none, price := p, b3 := b3, s3 := s3 }
      = TrendSixState.bot := by
  simp only [classifySixState]

/-! ═══════════════════════════════════════════════════════════════════════
    § 4. 信号位向量 b ∈ {0,1}⁶（FULL §5，line 294-302）：非强制互斥
    ═══════════════════════════════════════════════════════════════════════

  ★no-patch（信号位真 6 维，不塌缩）：FULL §5「三类买卖点不能强行互斥」——把六类买卖点
    装进 {0,1}⁶（六个独立 bool），不是 `BspClass` 互斥 sum。`BspClassification` 已证
    `no_exclusive_trichotomy`（互斥 sum 在 x_2b3b 失败）。本节给 {0,1}⁶ 的**正面构造** +
    真 6 维见证（2B/3B 同时为 1 可达 ⟹ 状态空间 = 2⁶ = 64，非 sum 的 6）。
-/

/--
  **信号位向量 b（FULL §5）** —— 六个独立 bool 标记位（B₁B₂B₃S₁S₂S₃），非互斥 subset。
  逐字段 bit-exact 对齐 rust `types.rs::BspBits`（buy1/buy2/buy3/sell1/sell2/sell3）。
  ★`structure`（积类型）而非 `inductive`（和类型）——subset 语义（一点可占多格），区别于
  互斥 `BspClass`（恰好一格）。
-/
structure SignalBits where
  b1 : Bool  -- 第一类买点（B₁）
  b2 : Bool  -- 第二类买点（B₂）
  b3 : Bool  -- 第三类买点（B₃）
  s1 : Bool  -- 第一类卖点（S₁）
  s2 : Bool  -- 第二类卖点（S₂）
  s3 : Bool  -- 第三类卖点（S₃）
deriving DecidableEq, Repr

/-- 全 0 信号位（无任何买卖点；§5 默认态，对齐 rust `BspBits::default`）。 -/
def SignalBits.empty : SignalBits :=
  { b1 := false, b2 := false, b3 := false, s1 := false, s2 := false, s3 := false }

/--
  **三类买卖点判据的 Decidable 实例（重锚 BspClassification）** —— `IsType1/2/3Buy/3Sell` 是
  `def`（不透明），Lean 不自动综合 Decidable。这里 `unfold` 暴露其合取/比较结构（`IsDivergence`
  已有实例 Divergence.lean:85，其余是 Bool 等式 + Int 比较）⟹ 判据可 `decide`，使 `signalBitsOf`
  忠实从判据投影（非另造 Bool 输入），bit-exact 对齐 rust `EndpointSituation::is_first/second/third`。
-/
instance (e : BspEndpoint) : Decidable (IsType1 e) := by
  unfold IsType1; exact inferInstanceAs (Decidable (_ ∧ _))
instance (e : BspEndpoint) : Decidable (IsType2 e) := by
  unfold IsType2; exact inferInstanceAs (Decidable (_ ∧ _))
instance (e : BspEndpoint) : Decidable (IsType3Buy e) := by
  unfold IsType3Buy; exact inferInstanceAs (Decidable (_ ∧ _ ∧ _ ∧ _))
instance (e : BspEndpoint) : Decidable (IsType3Sell e) := by
  unfold IsType3Sell; exact inferInstanceAs (Decidable (_ ∧ _ ∧ _ ∧ _))

/--
  **从 BspEndpoint 投影信号位（重锚 BspClassification 判据）** —— 把端点的三类买/卖判据
  投影为信号位向量。每个 bit 由对应判据的 `decide` 求值（判据是 Decidable Prop，见上实例）。
  ★非互斥：若端点同占二、三类（x_2b3b），则 b2=b3=true 同时成立——投影忠实保留 subset 语义。
-/
def signalBitsOf (e : BspEndpoint) : SignalBits :=
  { b1 := decide (e.side = Side.long) && decide (IsType1 e)
    b2 := decide (e.side = Side.long) && decide (IsType2 e)
    b3 := decide (IsType3Buy e)
    s1 := decide (e.side = Side.short) && decide (IsType1 e)
    s2 := decide (e.side = Side.short) && decide (IsType2 e)
    s3 := decide (IsType3Sell e) }

/--
  **★信号位真 6 维·2B/3B 可同时为 1（L0，FULL §5 + maimai:170 V型反转）** —— 存在一个
  信号位向量 b₂=b₃=1 同时成立。用 `BspClassification.x_2b3b`（已证同占二、三类）投影。
  ★这是「不强制互斥」的正面见证：信号位状态空间含 (b2,b3)=(1,1)，互斥 sum 无法表达此态。
-/
theorem signalBits_2b3b_coexist :
    (signalBitsOf x_2b3b).b2 = true ∧ (signalBitsOf x_2b3b).b3 = true := by
  -- x_2b3b 是闭项，signalBitsOf 经判据 Decidable 实例全可计算 ⟹ b2/b3 真求值为 true。
  -- 求值即证明（机器产，对齐 rust signalBitsOf(x_2b3b) 的 is_second/is_third 同时置位）。
  decide

/--
  **★信号位非塌缩（L0，no-patch 真 6 维）** —— 信号位状态空间含一个 (b2,b3)=(1,1) 的值，
  故信号位**不能**塌缩为「每点恰好一类」的互斥 sum（互斥 sum 中 b2/b3 至多一个为 1）。
  这把 FULL §5「不能强行互斥」形式化为：存在两位同时置 1 的合法向量。
-/
theorem signalBits_not_collapsible_to_sum :
    ∃ b : SignalBits, b.b2 = true ∧ b.b3 = true :=
  ⟨signalBitsOf x_2b3b, signalBits_2b3b_coexist⟩

/--
  **★信号位 1B/2B 不可重合（L0，maimai:172 时间互斥）** —— `empty` 投影外，第一类与第二类
  在同一端点不可同时为 1（maimai:65「第一类与第二类不可能重合」「时间上前后出现」）。
  形式化为本级别可观测判据：`IsType1` 要求 `brokeCenter=true`，`IsType2` 要求 `brokeCenter=false`
  ⟹ 两判据互斥（同一端点 brokeCenter 唯一）⟹ 投影后 b1/b2 不同时为 1。
  ★这与 2B/3B 可重合不矛盾：非互斥 ≠ 任意组合可达，是**判据决定哪些组合可达**。
-/
theorem signalBits_1b2b_exclusive (e : BspEndpoint) :
    ¬ (IsType1 e ∧ IsType2 e) := by
  rintro ⟨h1, h2⟩
  unfold IsType1 at h1; unfold IsType2 at h2
  -- h1.1 : brokeCenter = true；h2.2 : brokeCenter = false ⟹ 矛盾。
  rw [h1.1] at h2
  exact absurd h2.2 (by decide)

/-! ═══════════════════════════════════════════════════════════════════════
    § 5. 反退化见证（具体缠论数值例子，非平凡桩 + parity 锚点）
    ═══════════════════════════════════════════════════════════════════════ -/

/-- 见证用中枢：核心 [100,200]（重用 signal.rs/level_state.rs parity fixture 同口径）。 -/
def sampleSixCenter : Center :=
  { zd := 100, zg := 200, startIndex := 0, endIndex := 12, valid := by decide }

/-- ★见证：无中枢 ⟹ ⊥（bot）。 -/
theorem witness_six_bot :
    classifySixState { lastCenter := none, price := 150, b3 := true, s3 := true }
      = TrendSixState.bot := by decide

/-- ★见证：p=150 在中枢内（100≤150≤200）⟹ I（insideZ），不受 b3/s3 影响。 -/
theorem witness_six_inside :
    classifySixState { lastCenter := some sampleSixCenter, price := 150, b3 := true, s3 := true }
      = TrendSixState.insideZ := by decide

/-- ★见证：p=250 在中枢上方且无三买（b3=false）⟹ U⁰（aboveNo3B）。 -/
theorem witness_six_aboveNo3B :
    classifySixState { lastCenter := some sampleSixCenter, price := 250, b3 := false, s3 := false }
      = TrendSixState.aboveNo3B := by decide

/-- ★见证：p=250 在中枢上方且有三买（b3=true）⟹ U¹（aboveB3）。 -/
theorem witness_six_aboveB3 :
    classifySixState { lastCenter := some sampleSixCenter, price := 250, b3 := true, s3 := false }
      = TrendSixState.aboveB3 := by decide

/-- ★见证：p=50 在中枢下方且无三卖（s3=false）⟹ D⁰（belowNo3S）。 -/
theorem witness_six_belowNo3S :
    classifySixState { lastCenter := some sampleSixCenter, price := 50, b3 := false, s3 := false }
      = TrendSixState.belowNo3S := by decide

/-- ★见证：p=50 在中枢下方且有三卖（s3=true）⟹ D¹（belowS3）。 -/
theorem witness_six_belowS3 :
    classifySixState { lastCenter := some sampleSixCenter, price := 50, b3 := false, s3 := true }
      = TrendSixState.belowS3 := by decide

/-- ★见证：x_2b3b 投影信号位 = (b2,b3)=(1,1)（V型反转，2B/3B 重合）。 -/
theorem witness_signal_2b3b :
    (signalBitsOf x_2b3b).b2 = true ∧ (signalBitsOf x_2b3b).b3 = true :=
  signalBits_2b3b_coexist

/-! ═══════════════════════════════════════════════════════════════════════
    § 6. 边界条件 + 下游推论 + 谱系引用（result-package 六要素的 Lean 侧）
    ═══════════════════════════════════════════════════════════════════════

  ★边界条件（结论翻转）：
    - 六态 r 的 partition 性依赖 `classifyPosition` 的三态 partition（CenterStates）。若位置三态
      改用开区间或 ℝ 测度（临界 p=zd/p=zg 归属翻转），within/above/below 边界翻转 ⟹ 六态边界跟着翻转。
      本文件用闭区间（p=zg 算 within → I，破 zg 才 above），bit-exact 对齐 rust `rlevel_boundary_zg_is_inside`。
    - within **不裂**第三类是定义选择（中枢内无「离开后回试」语境）。若某口径要求中枢内也标第三类
      （如中枢震荡中的次级别三买），则 within 须裂 ⟹ 六态不足，须扩为八态。当前 §5 原文 within 单态
      ⟹ 六态忠实。
    - 信号位非互斥的**可达组合**由判据决定：2B/3B 可重合（x_2b3b 见证），但 1B/2B 不可重合
      （signalBits_1b2b_exclusive，brokeCenter 互斥）。若改 IsType1/IsType2 判据使 brokeCenter
      不再互斥，则 1B/2B 可重合性翻转。

  ★下游推论：
    - rust `classifier/level_state.rs::RLevel` + `rlevel_of` 是本文件 `TrendSixState` + `classifySixState`
      的 bit-exact 镜像（六构造子逐一对应）⟹ rust 侧六态实装已有 Lean canonical 锚点（此前缺）。
    - rust `types.rs::BspBits` 是 `SignalBits` 的 bit-exact 镜像（六字段逐一对应）⟹ 信号位 {0,1}⁶
      实装已有 Lean canonical 锚点 + 真 6 维非塌缩证明（此前 BspClassification 只证 sum 失败，
      未给 {0,1}⁶ 正面构造）。
    - 策略层（StrategyFamily / FullDefinitionStrategy）消费 (r, b)：r 定位「在哪」（六态），
      b 标记「有什么信号」（位向量）。本文件确认这是 5 元组 §5 的两个**独立**分量（正交）。

  ★谱系引用：
    - 这是**新形式化**（六态 r + 信号位 b 的 canonical Lean），非概念分离。但承接两条已结算谱系：
      · `no_exclusive_trichotomy`（BspClassification）：买卖点互斥 sum 失败 → 本文件给 {0,1}⁶ 正面替代。
      · CenterStates 位置三态（谱系 608/049课）：本文件六态 r 是其在 Θ_signal 参数化下的精化扩展。
    - 与 `TrendCompleteClassification.TrendClass`（τ 四态：走势类型）**正交**：τ 是「走势是什么类型」
      （⊥/盘整/上涨/下跌），r 是「相对中枢在哪个位置态」。FULL §5 的 5 元组 (τ, r, b, u, ...) 明确
      二者是不同分量——本文件**不**混淆 τ 与 r（no-patch：六态 r 不是把四态 τ 扩两个）。

  ★影响声明：
    - 新增 `Origin.TrendSixState`（root），不改任何既有 Lean 文件（只 import CenterStates/BspClassification 只读）。
    - rust 侧新增 `classifier/six_state.rs`（parity 层，引用既有 RLevel/BspBits），改 `classifier/mod.rs`
      仅加模块注册。不碰 signal.rs/bsp.rs/level_state.rs/strategy。
-/

end NewChanlun.Origin
