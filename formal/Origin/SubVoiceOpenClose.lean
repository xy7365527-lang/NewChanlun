/-
  Origin/SubVoiceOpenClose.lean — §9 子声部开平状态机（X-E 关闭/开启谓词 + ã_{v,t+1} 状态递归）

  ★工位定位（L3 §9 X-E 开平状态机，真实 canonical 缺口）：
    canonical `pasted-text.txt §9 line 544-592`（=`strict_hybrid_state_machine_strategy.md` §8 声部树
    公理的开平动态版）定义了「子声部的开平状态机」——关闭谓词 X_{v,t}、开启谓词 E_{v,t}、状态
    递归 ã_{v,t+1}（关闭优先于开启）。Origin canonical 此前**只形式化了状态相位的数据载体**
    （`CompleteStateEvent.OrderPhase` 四态 + `VoiceForest.{ancestorClosed,exactUnitMatch}` 树层
    一致性条件）与**开启谓词的部分分量**（`VoiceThreeLevel.{ParentValid,Inside,G}` 父子许可），
    但 §9 的**核心转移**——关闭谓词 X、状态递归 ã、关闭优先于开启——**从未被形式化**。

    本文件补全该缺口：把 §9 line 552-592 的 X/E 谓词与 ã 状态递归逐条转写为 Lean，证「关闭优先于
    开启」（line 592「关闭优先于开启」的结构编码）+ 状态四态封闭 + 转移确定唯一。

  ── canonical 依据（pasted-text.txt §9 line 544-592 boxed，逐条转写）──────────────
    父方向：σ_p = σ_{p(v)}（line 548）。
    关闭谓词（line 552-562）：
        X_{v,t} = ¬ParentValid_{v,t} ∨ χ^{σ_p}_{v,t} ∨ Stop_{v,t} ∨ RiskClose_{v,t}.
    开启谓词（line 566-576）：
        E_{v,t} = ParentValid_{v,t} ∧ χ^{-σ_p}_{v,t} ∧ Fresh_{v,t} ∧ F^{eq}_{v,t}.
    状态递归（line 582-590）：
        ã_{v,t+1} = 0,              当 X_{v,t}；
                    1,              当 ¬X_{v,t} ∧ a_{v,t}=0 ∧ E_{v,t}；
                    a_{v,t},        其他。
    关闭优先于开启（line 592）。
    语义（line 596-601）：父级多头 ⟹ 次级卖出证书开空、次级买入证书平空（χ^{-σ_p}=次级反父向
    信号=开子仓；χ^{σ_p}=次级同父向信号=平子仓）。

  ── 与 committed 的关系（不重复定义，复用）──────────────────────────────────────
    - `ParentValid`：复用 `VoiceThreeLevel.ParentValid`（committed，§9 ¬ParentValid 是 X 的第一析取项、
      ParentValid 是 E 的第一合取项——二者共用同一父有效性谓词，无重复定义）。
    - 关闭/开启的**信号/止损/风险/Fresh/F^eq 分量**：canonical 把它们列为状态读出的 bool（χ^{±σ_p}、
      Stop、RiskClose、Fresh、F^eq 都是「当下从 D_t/ν_t 读出」的判定）——本文件承载它们为 `SubVoiceEnv`
      的 bool 字段（与 `CompleteStateEvent.VenueState`/`SignalMemory` 的状态读出语义对齐），不臆造其
      内部计算（χ 由 D_t 缠论结构判、Stop 由止损价判、Fresh 由 M_t 信号记录判、F^eq 由 ν_t 借券/手数/
      保证金判——这些是各自工位的 L0/L2 判据，本文件只组装开平状态机的转移代数）。
    - 持仓事实 a_{v,t} ∈ {0,1}：复用 `CompleteStateEvent.VoiceState.active : Bool`。

  ── 认识论等级（formalization-validity-domain 强制标注）─────────────────────────
    本文件 = **L0**（纯结构：X/E 谓词的布尔代数 + ã 状态递归的转移代数 + 关闭优先证明，
    decide/rfl machine-checked，不依赖数据）。`lake env lean Origin/SubVoiceOpenClose.lean` 通过 =
    「关闭优先于开启」「状态递归落 {0,1}」「转移确定唯一」在定义层成立。

    ★诚实边界（no声明膨胀）：本文件**不**证：
      · 子声部开平**盈利/最优/实盘有效**（L3 EmpiricalDomain）——开平状态机是缠论次级别短差的
        结构规则，其盈利性是 L2/L3，本文件不声称。
      · χ^{±σ_p}/Stop/RiskClose/Fresh/F^eq 各**分量的判定逻辑**——它们作 `SubVoiceEnv` 的 bool 输入
        （状态读出），本文件证「给定这些读出后，X/E/ã 的转移代数确定」，不证读出本身（那是缠论
        结构层 D_t / 风控层 / ν_t 的判据，各自工位 L0/L2）。
      · 「关闭优先于开启」**不是**本文件臆造的工程裁决——它是 canonical line 592 的明文公理，本文件
        把它编码为 ã 的分支顺序（X 先于 E 判）并证其后果（X ⟹ ã=0 无论 E）。

  ── 依赖方向（单向无环，全 committed 只读）─────────────────────────────────────
    SubVoiceOpenClose → {VoiceThreeLevel（ParentValid/PermitEnv）, CompleteStateEvent（OrderPhase/
    VoiceState）}。不改任何 committed。standalone（不 import legacy Strict）。
    验证：`cd formal && lake env lean Origin/SubVoiceOpenClose.lean`。禁 sorry/admit/axiom。
    命名空间 NewChanlun.Origin。待 Lead 登记 root：`NewChanlun.Origin.SubVoiceOpenClose`。

  谱系：CompleteStateEvent（§3/§20 状态/事件 schema，OrderPhase 四态相位）+ VoiceThreeLevel（§9 父子
    许可 ParentValid/G）→ 本文件（§9 X-E 开平状态机转移：关闭谓词 + 状态递归 + 关闭优先于开启）。
-/

import Origin.VoiceThreeLevel
import Origin.CompleteStateEvent

namespace NewChanlun.Origin.SubVoiceOpenClose

open NewChanlun.Origin.VoiceThreeLevel (PermitEnv ParentValid)
open NewChanlun.Origin.CompleteStateEvent (OrderPhase VoiceState)

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 子声部开平环境 `SubVoiceEnv`（§9 line 552-576 X/E 谓词的当下读出）

  canonical X/E 谓词读出当下的若干判定。本结构逐字段承载这些读出（与 §3 状态分量的读出语义对齐）：
    - `permit : PermitEnv`：父子许可环境（含 ParentValid 所需的 Normal/父持仓/父平仓，复用
      VoiceThreeLevel committed）。
    - `chiParentDir : Bool`：χ^{σ_p}_{v,t}——**同父向**信号（父级多头时的次级买入/父级空头时的次级
      卖出）。canonical line 596-601：同父向信号 = 平子仓信号（出场触发）。
    - `chiAntiParentDir : Bool`：χ^{-σ_p}_{v,t}——**反父向**信号（父级多头时的次级卖出/父级空头时的
      次级买入）。canonical line 596-601：反父向信号 = 开子仓信号（进场触发）。
    - `stop : Bool`：Stop_{v,t}——止损触发（line 559）。
    - `riskClose : Bool`：RiskClose_{v,t}——风险关闭触发（line 561，§11 风险模式驱动的强平/去杠杆）。
    - `fresh : Bool`：Fresh_{v,t}——信号新鲜（同一信号不重复使用，line 573；载体 M_t consumedSignals）。
    - `eqFeasible : Bool`：F^{eq}_{v,t}——同单位双开在借券/手数/保证金/场所规则上可行（line 575-578；
      载体 ν_t）。

  ★诚实标注：本结构的 bool 字段是**状态读出**（χ/Stop/RiskClose/Fresh/F^eq 的判定结果），不是它们的
  内部计算。开平状态机的转移代数（X/E/ã）在给定这些读出后**确定**——这是 L0；读出本身由各自工位
  （缠论结构/止损/风控/ν）产出，本文件不 discharge（诚实暴露 §9 状态机读哪些输入）。
-/
structure SubVoiceEnv where
  /-- 父子许可环境（ParentValid 的载体，复用 VoiceThreeLevel committed）。 -/
  permit : PermitEnv
  /-- χ^{σ_p}：同父向信号（平子仓触发，line 596-601）。 -/
  chiParentDir : Bool
  /-- χ^{-σ_p}：反父向信号（开子仓触发，line 596-601）。 -/
  chiAntiParentDir : Bool
  /-- Stop_{v,t}：止损触发（line 559）。 -/
  stop : Bool
  /-- RiskClose_{v,t}：风险关闭触发（line 561）。 -/
  riskClose : Bool
  /-- Fresh_{v,t}：信号新鲜，同一信号不重复（line 573）。 -/
  fresh : Bool
  /-- F^{eq}_{v,t}：同单位双开可行（借券/手数/保证金/场所，line 575-578）。 -/
  eqFeasible : Bool

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 关闭谓词 X_{v,t} 与开启谓词 E_{v,t}（§9 line 552-576 boxed）

  X 与 E 都是 `Bool`（可判定）——`ParentValid` 经 `VoiceThreeLevel` 的 `Decidable` 实例转 bool。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★关闭谓词 `closePred`（X_{v,t}，L0，§9 line 552-562）：
      X_{v,t} = ¬ParentValid ∨ χ^{σ_p} ∨ Stop ∨ RiskClose.

  四析取项任一为真即触发关闭（子声部出场）：父失效（背景级否决，与 `VoiceThreeLevel.ParentValid`
  共用同一谓词）∨ 同父向信号（line 596-601 平子仓）∨ 止损 ∨ 风险关闭。
-/
def closePred (env : SubVoiceEnv) : Bool :=
  (!decide (ParentValid env.permit)) || env.chiParentDir || env.stop || env.riskClose

/--
  ★开启谓词 `openPred`（E_{v,t}，L0，§9 line 566-576）：
      E_{v,t} = ParentValid ∧ χ^{-σ_p} ∧ Fresh ∧ F^{eq}.

  四合取项全真才触发开启（子声部进场）：父有效（背景级许可）∧ 反父向信号（line 596-601 开子仓）∧
  信号新鲜（不重复用）∧ 同单位双开可行。第一合取项 `ParentValid` 与 X 的第一析取项 `¬ParentValid`
  共用同一谓词——这使「父失效时 E 必假」成为代数事实（见 `open_false_of_not_parentValid`）。
-/
def openPred (env : SubVoiceEnv) : Bool :=
  decide (ParentValid env.permit) && env.chiAntiParentDir && env.fresh && env.eqFeasible

/--
  ★父失效 ⟹ 关闭触发（L0，「能不能做」的关闭侧必要性）：
  `¬ParentValid → closePred = true`——父声部失效（背景级否决）⟹ X 触发（无论信号/止损/风险）。
  这编码「高级别决定能不能做」的关闭侧：背景级否决支配关闭（X 的第一析取项）。
-/
theorem close_of_not_parentValid (env : SubVoiceEnv)
    (h : ¬ ParentValid env.permit) : closePred env = true := by
  unfold closePred
  have : decide (ParentValid env.permit) = false := decide_eq_false h
  simp [this]

/--
  ★父失效 ⟹ 开启不触发（L0，「能不能做」的开启侧必要性）：
  `¬ParentValid → openPred = false`——父失效 ⟹ E 必假（开启的第一合取项 ParentValid 不成立）。
  与 `close_of_not_parentValid` 配对：父失效时 X 真且 E 假 ⟹ ã=0（关闭），无歧义。
-/
theorem open_false_of_not_parentValid (env : SubVoiceEnv)
    (h : ¬ ParentValid env.permit) : openPred env = false := by
  unfold openPred
  have : decide (ParentValid env.permit) = false := decide_eq_false h
  simp [this]

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 状态递归 ã_{v,t+1}（§9 line 582-590 boxed）+ 关闭优先于开启（line 592）

  持仓事实 a_{v,t} ∈ {0,1} 用 `Bool` 承载（复用 `CompleteStateEvent.VoiceState.active`）。状态递归
  三分支：关闭 ⟹ 0；（不关闭 ∧ 当前空仓 ∧ 开启）⟹ 1；其他 ⟹ 保持 a。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★子声部持仓状态递归 `nextActive`（ã_{v,t+1}，L0，§9 line 582-590，本文件核心）：
      ã_{v,t+1} = 0,        当 X_{v,t}（关闭，**优先**）；
                  1,        当 ¬X ∧ a=0 ∧ E（空仓且开启触发）；
                  a_{v,t},  其他（保持）。

  ★关闭优先于开启（line 592）的结构编码：`if closePred then false else …`——X 在第一分支判，
  E 只在 ¬X 分支才被考虑。故 X∧E 同真时结果是 `false`（关闭赢，见 `nextActive_close_wins`）。

  ★第二分支的 `active = false` 守卫（line 587 `a_{v,t}=0`）：只在**空仓**时才由 E 触发开仓——
  已持仓（a=1）时 E 不重复开（重复开会破坏 §9 精确同单位双开的 a∈{0,1} 二值，line 522-525）。
  已持仓且不关闭 ⟹ 落「其他」分支保持 a=1（持仓延续）。
-/
def nextActive (env : SubVoiceEnv) (active : Bool) : Bool :=
  if closePred env then
    false                              -- X：关闭（优先，line 592）
  else if active = false && openPred env then
    true                               -- ¬X ∧ a=0 ∧ E：开仓
  else
    active                             -- 其他：保持

/--
  ★关闭优先于开启（L0，line 592 的核心后果）：
  `closePred = true → nextActive = false`——只要关闭触发，ã=0，**无论开启谓词与当前持仓如何**。
  这是 line 592「关闭优先于开启」的可观测投影：X 真 ⟹ 出场，E 不能翻转它。
-/
theorem nextActive_close_wins (env : SubVoiceEnv) (active : Bool)
    (hX : closePred env = true) : nextActive env active = false := by
  unfold nextActive
  simp [hX]

/--
  ★X∧E 同真 ⟹ 关闭赢（L0，关闭优先的最强见证）：当 X 与 E **同时**触发时，结果是关闭（false）。
  这排除「同时收到反父向开仓信号 + 父失效/止损」时的歧义——canonical line 592 钉死：关闭赢。
-/
theorem nextActive_close_beats_open (env : SubVoiceEnv) (active : Bool)
    (hX : closePred env = true) (_hE : openPred env = true) :
    nextActive env active = false :=
  nextActive_close_wins env active hX

/--
  ★空仓 + 开启 + 不关闭 ⟹ 开仓（L0，开启侧充分性，line 587）：
  `closePred=false → openPred=true → nextActive false = true`——空仓状态下，无关闭触发且开启谓词
  成立 ⟹ ã=1（进场）。这是 §9 开仓的充分条件（第二分支构造）。
-/
theorem nextActive_open (env : SubVoiceEnv)
    (hX : closePred env = false) (hE : openPred env = true) :
    nextActive env false = true := by
  unfold nextActive
  simp [hX, hE]

/--
  ★持仓延续（L0，line 590「其他」分支，持仓侧）：
  `closePred=false → active=true → nextActive=true`——已持仓且无关闭触发 ⟹ 保持持仓（落「其他」
  分支，因第二分支的 `active=false` 守卫不满足）。子声部一旦开仓，只有 X 能让它出场（持仓黏性）。
-/
theorem nextActive_hold (env : SubVoiceEnv)
    (hX : closePred env = false) : nextActive env true = true := by
  unfold nextActive
  simp [hX]

/--
  ★空仓 + 无开启 + 不关闭 ⟹ 保持空仓（L0，line 590「其他」分支，空仓侧）：
  `closePred=false → openPred=false → nextActive false = false`——空仓且开启谓词不成立 ⟹ 保持空仓
  （第二分支因 openPred=false 不触发，落「其他」分支保持 a=0）。无信号不进场。
-/
theorem nextActive_stay_flat (env : SubVoiceEnv)
    (hX : closePred env = false) (hE : openPred env = false) :
    nextActive env false = false := by
  unfold nextActive
  simp [hX, hE]

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 状态机良构：二值封闭 · 确定唯一 · 关闭优先全覆盖
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★状态二值封闭（L0）：ã_{v,t+1} ∈ {false, true}——状态递归保持 a∈{0,1} 二值（line 587 守卫使
  持仓不超 1，对齐 §9 精确同单位双开的 a∈{0,1}，line 522-525）。`Bool` 类型层已二值，本定理把它
  显式陈述为「输出必是两值之一」（状态空间封闭，无第三态泄漏）。
-/
theorem nextActive_binary (env : SubVoiceEnv) (active : Bool) :
    nextActive env active = false ∨ nextActive env active = true := by
  cases nextActive env active <;> simp

/--
  ★转移确定唯一（L0，§20 line 1442 `∃! x'` 的开平状态机投影）：给定 (env, active)，下一持仓状态
  **存在且唯一**（函数求值确定性）。与 `CompleteStateEvent.transition_exists_unique`（完整态）/
  `FullDefinitionStrategy.hybrid_step_complete_unique`（摘要态）平行——此处是子声部持仓位的版本。
-/
theorem nextActive_exists_unique (env : SubVoiceEnv) (active : Bool) :
    ExistsUnique (fun a' => nextActive env active = a') := by
  refine ⟨nextActive env active, rfl, ?_⟩
  intro y hy
  exact hy.symm

/--
  ★关闭/开启/保持三分支全覆盖且互斥（L0，line 582-590 的完全分类）：对任意 (env, active)，状态
  递归恰落入三分支之一——
    (C) 关闭：closePred=true（⟹ false）；
    (O) 开仓：closePred=false ∧ active=false ∧ openPred=true（⟹ true）；
    (H) 保持：closePred=false ∧ ¬(active=false ∧ openPred=true)（⟹ active）。
  三分支谓词的析取恒真（全覆盖），互斥由 if-then-else 的结构顺序保证（C 先于 O 先于 H）。
  这坐实 ã 的定义是 §9 line 582-590 的**完全分类**（无遗漏 case、无重叠裁决）。
-/
theorem nextActive_trichotomy (env : SubVoiceEnv) (active : Bool) :
    (closePred env = true ∧ nextActive env active = false)
    ∨ (closePred env = false ∧ active = false ∧ openPred env = true
        ∧ nextActive env active = true)
    ∨ (closePred env = false ∧ ¬(active = false ∧ openPred env = true)
        ∧ nextActive env active = active) := by
  by_cases hX : closePred env = true
  · exact Or.inl ⟨hX, nextActive_close_wins env active hX⟩
  · have hXf : closePred env = false := by
      cases h : closePred env with
      | false => rfl
      | true => exact absurd h hX
    by_cases hOpenGuard : active = false ∧ openPred env = true
    · obtain ⟨ha, hE⟩ := hOpenGuard
      subst ha
      exact Or.inr (Or.inl ⟨hXf, rfl, hE, nextActive_open env hXf hE⟩)
    · refine Or.inr (Or.inr ⟨hXf, hOpenGuard, ?_⟩)
      -- 保持分支：active=true 时持仓延续；active=false 时（因 hOpenGuard 必有 openPred=false）保持空仓。
      cases ha : active with
      | true => exact nextActive_hold env hXf
      | false =>
        have hEf : openPred env = false := by
          cases hE : openPred env with
          | false => rfl
          | true => exact absurd ⟨ha, hE⟩ hOpenGuard
        exact nextActive_stay_flat env hXf hEf

/-! ════════════════════════════════════════════════════════════════════════
  ## §5 语义见证：父级多头时的开空/平空（§9 line 596-601）

  canonical line 596-601：「父级多头：次级卖出证书开空，次级买入证书平空」。在本文件代数里：
    - 次级卖出证书 = 反父向信号 χ^{-σ_p}（父多 σ_p=+1，反向=卖=空）⟹ `chiAntiParentDir` ⟹ 开子仓（E 侧）。
    - 次级买入证书 = 同父向信号 χ^{σ_p}（父多 σ_p=+1，同向=买）⟹ `chiParentDir` ⟹ 平子仓（X 侧）。
  本节用具体见证 env 坐实这条语义映射（开空走 openPred、平空走 closePred）。
  ════════════════════════════════════════════════════════════════════════ -/

/-- 父有效的许可环境见证（Normal ∧ 父持仓 ∧ 父未平）——使 ParentValid 成立。 -/
private def sampleParentValidEnv : PermitEnv :=
  { normal := true, parentPos := 1, parentClosed := false,
    parentCenter := { zd := 0, zg := 0, startIndex := 0, endIndex := 0, valid := by decide },
    nestTick := 0 }

private theorem sampleParentValidEnv_valid : ParentValid sampleParentValidEnv := by
  unfold ParentValid sampleParentValidEnv
  refine ⟨rfl, ?_, rfl⟩
  decide

/--
  ★父级多头·次级反父向信号 ⟹ 开子仓（L0 语义见证，line 596-601 开空侧）：
  父有效 + 反父向信号（χ^{-σ_p}，父多即次级卖出=开空）+ 新鲜 + 可行，且无同父向/止损/风险 ⟹
  空仓状态下 nextActive=true（开仓）。坐实「次级卖出证书开空」走 openPred → nextActive 开仓路径。
-/
theorem parent_long_anti_signal_opens :
    nextActive
      { permit := sampleParentValidEnv,
        chiParentDir := false, chiAntiParentDir := true,
        stop := false, riskClose := false, fresh := true, eqFeasible := true }
      false = true := by
  have hX : closePred
      { permit := sampleParentValidEnv,
        chiParentDir := false, chiAntiParentDir := true,
        stop := false, riskClose := false, fresh := true, eqFeasible := true } = false := by
    unfold closePred
    have : decide (ParentValid sampleParentValidEnv) = true :=
      decide_eq_true sampleParentValidEnv_valid
    simp [this]
  have hE : openPred
      { permit := sampleParentValidEnv,
        chiParentDir := false, chiAntiParentDir := true,
        stop := false, riskClose := false, fresh := true, eqFeasible := true } = true := by
    unfold openPred
    have : decide (ParentValid sampleParentValidEnv) = true :=
      decide_eq_true sampleParentValidEnv_valid
    simp [this]
  exact nextActive_open _ hX hE

/--
  ★父级多头·次级同父向信号 ⟹ 平子仓（L0 语义见证，line 596-601 平空侧）：
  同父向信号（χ^{σ_p}，父多即次级买入=平空）⟹ closePred=true ⟹ nextActive=false（出场），
  **无论当前是否持仓**。坐实「次级买入证书平空」走 closePred → 关闭路径。
-/
theorem parent_long_same_signal_closes (active : Bool) :
    nextActive
      { permit := sampleParentValidEnv,
        chiParentDir := true, chiAntiParentDir := false,
        stop := false, riskClose := false, fresh := true, eqFeasible := true }
      active = false := by
  have hX : closePred
      { permit := sampleParentValidEnv,
        chiParentDir := true, chiAntiParentDir := false,
        stop := false, riskClose := false, fresh := true, eqFeasible := true } = true := by
    unfold closePred; simp
  exact nextActive_close_wins _ active hX

end NewChanlun.Origin.SubVoiceOpenClose
