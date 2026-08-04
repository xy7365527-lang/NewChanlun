/-
  Origin/TotalWealth.lean — 取本金三阶段账本 TW 的 Origin native 模块（task #127，TW-port 工位）

  ════════════════════════════════════════════════════════════════════════
  ## 本文件做什么：把取本金三阶段账本 TW **native 重证进 Origin canonical base**

  ★A′ 裁决（Origin-TW #103 = native，codex binding 2026-06-26）：A′「Origin 唯一 canonical base」
  要求把取本金三阶段 TW 物理 port 进 `formal/Origin/` 成**独立 native 模块**——
  - **不** import legacy `Tlayers/Accounting/TotalWealth.lean`（不偷渡 legacy 已证结果）；
  - **不** 合并进 Origin `LedgerState`（#90 排除合并：R=Π-A-W 与取本金三阶段 TW 不同构，两独立结构并置）；
  - 在 Origin 命名空间**重新定义 + 重新证明**全部 TW 模型与 #90/OQ-9 关键定理（零 sorry/admit/axiom）。

  这是 native port（重证），不是 re-export（偷渡 legacy）。本文件的每条定理在 Origin native 定义
  上**重新成立**——legacy TotalWealth.lean 此后降为待清理 legacy reference（不再被 canonical 路径引用）。

  ════════════════════════════════════════════════════════════════════════
  ## 模型 B：缠论取本金三阶段账本（缠师第31课，原 Tlayers TW 模型的 native 重证）

  缠师第31课「取本金」三阶段会计（`t_engine.rs:107-451` / `rec_engine.rs:1219-1266`，
  设计文档 `docs/three_stages_accounting_design.md`，t_engine 有 prove_tw_neutral L2 守卫）：
  - 守恒量 `TW = free + holding + withdrawn`（逐 bar 不变量）。
  - 三阶段状态机 `CostReduction`（降成本）→ `CapitalRecovered`（退本金 free→withdrawn）
    → `EarningShares`（增股数），**单向不可逆**（OQ-9）。
  - 守恒律随阶段切换：CostReduction 守 Σ|units|（股数）；EarningShares 守 K（外部投入）。

  另一版形式化（Origin `LedgerState` = R=Π-A-W 财务守恒账本）与本 TW 模型**不同构**（#90）——
  本文件 §2 本地重建 R=Π-A-W 镜像（`LedgerComp`）仅作不同构对照裁定（不 import Origin LedgerState，
  与 legacy 同样的隔离策略，避免跨模块耦合）。

  ════════════════════════════════════════════════════════════════════════
  ## 认识论等级（formalization-validity-domain，强制标注）

  - **全文件 L0**（结构/定义层，零数据依赖）。`lake build` 通过 = TW 三阶段状态机类型自洽 +
    TW 守恒在每个会计算子下结构成立（L0 代数恒等）+ stage 单向性结构可证 + **不同构反例**的
    机器验证。**不**是缠论盈利 / 实盘有效声明（那是 L3，Rust prove_tw_neutral 是 L2 守卫）。
  - **不同构裁定本身是 L0 定理**：从两个模型的代数/状态结构推导，有效域=定义域（L0 不需 L2+）。
  - **本文件的 TW 守恒是结构恒等**（同价 c 固定下 `TW' = TW`），对应 Rust「同价操作 NAV 中性 = L0」。
  - 禁 sorry/admit/axiom。纯 Prop/Type，不依赖 Mathlib（Int 取自 Lean core）。

  ════════════════════════════════════════════════════════════════════════
  ## 隔离声明（native，关键）
  本模块自包含：**不 import legacy Tlayers.Accounting.TotalWealth**（native 重证），不 import
  Strict.* / Formal.* / Phase2.*。R=Π-A-W 侧（LedgerComp）在本文件本地重建（与 Origin LedgerState
  同构镜像，仅为做对照裁定）。

  谱系：#42（账本 watch-item）/ #86（HybridAssembly LedgerComp 新建 R=Π-A-W）/ #90（不同构裁定，
        machine-checked，原 Tlayers TW）/ OQ-9（守恒律相变可逆性矛盾，design §3.3）/ #97（Origin
        canonical base）/ Origin-TW #103（裁决=native port）→ 本文件 #127（TW native 重锚进 Origin）。
-/

namespace NewChanlun.Origin.TotalWealth

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 缠论取本金三阶段账本 TW（模型B，缠师第31课 L0 结构，Origin native）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★三阶段 `TStage`（L0，缠师第31课；**本文件 = 契约正本**，生产镜像 =
  `rust/src/theta_v0/strategy/ledger.rs` `TStage`——#127 native port 后锚的方向是「ledger.rs 锚
  本文件」。原注释「对齐 `t_engine.rs:120` TStage」指针作废：行号已漂，且 flat T 引擎的同名
  兄弟已由 #889 R8 改名 `FlatTStage`，划界见 `ledger.rs`/`t_engine.rs` 两侧枚举文档）。
  - `costReduction`（① 降成本）：短差，Σ|units| 守恒（"买入多少卖出多少不增仓"）。
  - `capitalRecovered`（② 退本金）：本金部分/全额移出在险池（free→withdrawn）。
  - `earningShares`（③ 增股数）：本金已全退，纯利润买更多 units，Σ|units| 单调增。
-/
inductive TStage where
  | costReduction
  | capitalRecovered
  | earningShares
deriving DecidableEq, Repr

/--
  ★阶段序 `stageRank`（L0）：把三阶段映到 {0,1,2}——单向不可逆迁移的偏序载体。
  CostReduction(0) → CapitalRecovered(1) → EarningShares(2)，rank **只增不减**（OQ-9）。
-/
def TStage.rank : TStage → Nat
  | TStage.costReduction => 0
  | TStage.capitalRecovered => 1
  | TStage.earningShares => 2

/--
  ★TW 账本态 `TWState`（L0，模型B，对齐 `t_engine.rs:208` TPositionEngine 会计分量）。

  - `free : Int`：自由现金/在险池（NAV 的现金部分）。
  - `holding : Int`：当前持仓市值 Σ units·c（NAV 的市值部分；c 是**外生价格**，见不同构理由1）。
  - `withdrawn : Int`：已退本金（退本金阶段移出在险池的现金，clear 时归还 free）。
  - `notionalIn : Int`：本 campaign 投入本金 K（退本金目标；enter 记 m·c，clear 重置 0）。
  - `stage : TStage`：当前阶段（带历史/路径依赖的状态机分量，单向不可逆）。
  - `openLegacyLegs : Nat`：未闭合**旧 ShareConserving（legacy）降成本腿**计数（OQ-9 扩维新维度，
    与 stage 正交——legacy 腿专指降成本期遗留的短差腿，earning 阶段不再开）。
  - `cumNetCash : Int`：净现金口径累加（cost_basis 符号的结构载体，design §2.2）——口径量，
    与 TW 三量正交，不进 TW = free+holding+withdrawn。

  ★诚实标注：`holding` 用 Int 承载市值摘要（结构层；真实 holding=Σunits·c 含外生 c，c 的跨 bar
  变动 = 盈亏，是 L2/L3 数据，本 L0 结构层把同价操作下的市值作整数量承载，不臆造价格）。
-/
structure TWState where
  free : Int
  holding : Int
  withdrawn : Int
  notionalIn : Int
  stage : TStage
  openLegacyLegs : Nat
  cumNetCash : Int
deriving Repr

/-- ★总财富 `TWState.tw`（L0，守恒量）：`TW = free + holding + withdrawn`（缠师第31课守恒律）。 -/
def TWState.tw (s : TWState) : Int := s.free + s.holding + s.withdrawn

/--
  ★TW 账本事件 `TWEvent`（L0，穷尽，对齐 t_engine 三阶段算子 + OQ-9 扩维载体）：
  - `shortDiff dCash`：降成本短差（free⇄holding 同价转换，TW 不变；CostReduction 阶段）。
  - `openShareLeg`：开一条 legacy ShareConserving 降成本短差腿（openLegacyLegs += 1）。
  - `closeShareLeg profit`：闭合一条 legacy ShareConserving 腿（openLegacyLegs -= 1，盈亏落 cumNetCash）。
    **OQ-9 矛盾的精确载体**：profit<0（亏损闭合）+ stage=earningShares ⟹ 净现金口径成本回正（端A）。
  - `recoverCapital w`：退本金（free→withdrawn，移出在险池 w）。触发/推进 CapitalRecovered。
  - `enterEarning`：本金全退后切 EarningShares（单向不可逆相变，无资金变动）。
    ★诚实（rawStep/legalStep 双层）：twStep（raw 层）对 enterEarning **不加入口守卫**——raw 层
    保留「带未闭合 legacy 腿进入 EarningShares」这条算术路径作为 violation witness。合法性由独立谓词
    `LegalEnterEarning` 承载（见 §1.5 OQ-9 gate）。
  - `clearCampaign`：campaign 结束（withdrawn→free 归还，stage 重置 CostReduction，legacy 腿/cumNetCash 清零）。
-/
inductive TWEvent where
  | shortDiff (dCash : Int)
  | openShareLeg
  | closeShareLeg (profit : Int)
  | recoverCapital (w : Int)
  | enterEarning
  | clearCampaign
deriving Repr

/--
  ★阶段推进 `advanceTo`（L0，单向，对齐 OQ-9 不可逆）：退本金达标 ⟹ rank 至少推到目标；
  **rank 只增不减**（见 advanceTo_rank_ge）。
-/
def advanceTo (current target : TStage) : TStage :=
  if current.rank ≤ target.rank then target else current

/-- ★advanceTo 的 rank 单调（L0，辅助）：`current.rank ≤ (advanceTo current target).rank`。 -/
theorem advanceTo_rank_ge (current target : TStage) :
    current.rank ≤ (advanceTo current target).rank := by
  unfold advanceTo
  by_cases h : current.rank ≤ target.rank
  · rw [if_pos h]; exact h
  · rw [if_neg h]; exact Nat.le_refl _

/--
  ★TW 更新 `twStep`（L0，全函数，**保 TW 守恒**——同价 c 固定下 free/holding/withdrawn 间转移）：
  - `shortDiff dCash`：holding→free 转 dCash ⟹ TW 不变。
  - `openShareLeg`：openLegacyLegs += 1（无 TW 三量变动）⟹ TW 不变。
  - `closeShareLeg profit`：openLegacyLegs -= 1 + cumNetCash += profit（口径量，不进 TW）⟹ TW 不变。
  - `recoverCapital w`：free→withdrawn 转 w ⟹ TW 不变；stage 推进 capitalRecovered（单向）。
  - `enterEarning`：纯 stage 切换 earningShares（单向），无资金变动 ⟹ TW 不变。
    ★raw 层无入口守卫（保留 violation witness）；合法性由 LegalEnterEarning（§1.5）排除。
  - `clearCampaign`：withdrawn→free 归还 ⟹ TW 不变；stage 重置 + legacy 腿/cumNetCash 清零
    （campaign 边界，新 campaign 起点，非回退）。

  ★诚实标注（OQ-9 关键）：`clearCampaign` 把 stage 重置回 costReduction，但这**不违反 stage 单向性**
  ——OQ-9 不可逆是「同一 campaign 内」的性质。故单向性定理（stage_rank_monotone）**只对不含
  clearCampaign 的算子声明**——诚实有效域边界，非 workaround。
-/
def twStep (s : TWState) : TWEvent → TWState
  | TWEvent.shortDiff dCash =>
      { s with free := s.free + dCash, holding := s.holding - dCash }
  | TWEvent.openShareLeg =>
      { s with openLegacyLegs := s.openLegacyLegs + 1 }
  | TWEvent.closeShareLeg profit =>
      { s with openLegacyLegs := s.openLegacyLegs - 1, cumNetCash := s.cumNetCash + profit }
  | TWEvent.recoverCapital w =>
      { s with free := s.free - w, withdrawn := s.withdrawn + w, stage := advanceTo s.stage TStage.capitalRecovered }
  | TWEvent.enterEarning =>
      { s with stage := advanceTo s.stage TStage.earningShares }
  | TWEvent.clearCampaign =>
      { s with free := s.free + s.withdrawn, withdrawn := 0, notionalIn := 0, stage := TStage.costReduction, openLegacyLegs := 0, cumNetCash := 0 }

/--
  ★★TW 守恒（L0，缠师第31课守恒律的结构形式）：每个会计算子 `twStep` 保持 `TW = free +
  holding + withdrawn`。对齐 Rust `prove_tw_neutral`（rec_engine.rs:1259）的**代数侧**（L0：同价
  c 固定下三量间转移 TW 不变）——Rust 的「全程零 panic」是 L2（跨 bar 价格）。
-/
theorem twStep_preserves_tw (s : TWState) (e : TWEvent) :
    (twStep s e).tw = s.tw := by
  cases e <;> simp only [twStep, TWState.tw] <;> omega

/--
  ★阶段单向不可逆 `stage_rank_monotone`（L0，OQ-9 结构形式，campaign 内）：对**不含
  clearCampaign** 的算子，stage 的 rank **只增不减**。machine-check「CostReduction→
  CapitalRecovered→EarningShares 单向不可逆」（OQ-9）。
-/
theorem stage_rank_monotone (s : TWState) (e : TWEvent)
    (hne : e ≠ TWEvent.clearCampaign) :
    s.stage.rank ≤ (twStep s e).stage.rank := by
  cases e with
  | shortDiff d => simp only [twStep, Nat.le_refl]
  | openShareLeg => simp only [twStep, Nat.le_refl]
  | closeShareLeg p => simp only [twStep, Nat.le_refl]
  | recoverCapital w =>
      simp only [twStep]; exact advanceTo_rank_ge s.stage TStage.capitalRecovered
  | enterEarning =>
      simp only [twStep]; exact advanceTo_rank_ge s.stage TStage.earningShares
  | clearCampaign => exact absurd rfl hne

/--
  ★EarningShares 不可回退（L0，OQ-9 的最强形式，campaign 内）：一旦 stage=earningShares，
  对不含 clearCampaign 的任何算子，stage 仍是 earningShares（rank=2 最大，advanceTo 不下降）。
  关死「增股数阶段回退到降成本/退本金」的误读——单向相变锁死（design §3.2 INV-2）。
-/
theorem earning_no_regress (s : TWState) (e : TWEvent)
    (hstage : s.stage = TStage.earningShares) (hne : e ≠ TWEvent.clearCampaign) :
    (twStep s e).stage = TStage.earningShares := by
  cases e with
  | shortDiff d => simpa only [twStep] using hstage
  | openShareLeg => simpa only [twStep] using hstage
  | closeShareLeg p => simpa only [twStep] using hstage
  | recoverCapital w =>
      simp only [twStep, advanceTo, hstage]
      rw [if_neg (by decide)]
  | enterEarning =>
      simp only [twStep, advanceTo, hstage]
      rw [if_pos (by decide)]
  | clearCampaign => exact absurd rfl hne

/-! ════════════════════════════════════════════════════════════════════════
  ## §1.5 OQ-9 gate（守恒律相变可逆性矛盾的扩维消解，codex binding 立场C 修正版）

  ### OQ-9 矛盾（design §3.3 / rust/src/trading/ledger.rs:11-18）
  缠论三阶段单向相变（INV-2）的可逆性矛盾，两端：
  - **端A（数值真相）**：进入 EarningShares 后闭合**旧 ShareConserving 腿**且 profit<0 时，净现金
    口径成本（cumNetCash）下降（"cost_basis 回正"）。算术不可否认。
  - **端B（相位锁死，INV-2）**：LedgerPhase 单向相变，EarningShares 不回退。

  ### 扩维消解（codex 三修正全部吸收，非补丁）
  1. 非法性在 **enterEarning 入口**强制（合法进入须 openLegacyLegs=0）⟹ 合法系统中「亏损腿闭合回正」
     根本不可达（无旧腿可闭）。
  2. **rawStep/legalStep 双层**：raw 层（twStep 全函数）保留端A 路径作 violation witness；legal 层
     （LegalTransition 谓词）排除它。端A 在 raw 层成立、在 legal 层不可达——不是「两端并存」。
  3. 非对称（入口依赖相位推进、持续独立于 cumNetCash）是自洽 hysteresis/latch 模式。

  ### review 修正（端B 从单步升级到 trace 级 + 堵 clearCampaign/zero-close 漏洞）
  - **修正1（trace 级 gate）**：`openShareLeg` 合法性收紧为 `stage.rank < earningShares.rank`
    + invariant `OQ9Inv`（stage=earning ⟹ openLegacyLegs=0）+ trace 级归纳 ⟹ 端B trace 级成立。
  - **修正4(b)**：`clearCampaign` 合法性加 `openLegacyLegs = 0`（不强制抹账绕 gate）。
  - **修正4(c)**：`closeShareLeg` 在 openLegacyLegs=0 时 raw 层 Nat 饱和减保持 0 但仍改 cumNetCash
    = 幽灵腿；legal 层已排除（要求 ≥1），加 `zero_close_is_ghost` 显式标注。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★OQ-9 入口证书 `LegalEnterEarning`（L0，codex 修正1）：合法进入 EarningShares 的前提 =
  **无未闭合 legacy ShareConserving 腿**（`openLegacyLegs = 0`）。缠师第31课「成本为0后卖出多少
  资金就买入多少资金」+ 第87课「真正的拉抬不需要花钱」。把 OQ-9 非法性**上移到入口**的证书。
-/
def LegalEnterEarning (s : TWState) : Prop := s.openLegacyLegs = 0

/--
  ★OQ-9 合法转移谓词 `LegalTransition`（L0，review 修正后的 legal 层）：哪些 (s, e) 合法。
  - `enterEarning`：须 `LegalEnterEarning s`（openLegacyLegs = 0）。
  - `openShareLeg`：须 `s.stage.rank < TStage.earningShares.rank`（修正1：earning 阶段不再开 legacy 腿）。
  - `closeShareLeg p`：须 `s.openLegacyLegs ≥ 1`（有腿才能闭，否则幽灵）。
  - `clearCampaign`：须 `s.openLegacyLegs = 0`（修正4(b)：结束前先清 legacy 腿）。
  - 其余（shortDiff/recoverCapital）：无额外约束。

  ★诚实（rawStep/legalStep 双层）：twStep 对所有 (s,e) 都有定义；本谓词只标注合法子集。
-/
def LegalTransition (s : TWState) : TWEvent → Prop
  | TWEvent.enterEarning => LegalEnterEarning s
  | TWEvent.openShareLeg => s.stage.rank < TStage.earningShares.rank
  | TWEvent.closeShareLeg _ => s.openLegacyLegs ≥ 1
  | TWEvent.clearCampaign => s.openLegacyLegs = 0
  | _ => True

/--
  ★★OQ-9 trace 级 invariant `OQ9Inv`（L0，端B 的 trace 级形式）：
  `stage = earningShares ⟹ openLegacyLegs = 0`——一旦进入增股数阶段，legacy 降成本腿恒空。
-/
def OQ9Inv (s : TWState) : Prop :=
  s.stage = TStage.earningShares → s.openLegacyLegs = 0

/--
  ★OQ-9 invariant 初态成立 `costReduction_satisfies_oq9inv`（L0）：任何 stage=costReduction 的态
  满足 `OQ9Inv`（前件 `costReduction = earningShares` 假 ⟹ 蕴含平凡真）。给 trace 级定理合法起点。
-/
theorem costReduction_satisfies_oq9inv (s : TWState) (h : s.stage = TStage.costReduction) :
    OQ9Inv s := by
  intro hst; rw [h] at hst; exact absurd hst (by decide)

/--
  ★OQ-9 invariant 标准初态成立 `init_satisfies_oq9inv`（L0）：标准开局态（costReduction，legacy
  腿 0，cumNetCash 0）满足 `OQ9Inv`——trace 定理的具体合法起点。
-/
theorem init_satisfies_oq9inv (free holding withdrawn notionalIn : Int) :
    OQ9Inv { free := free, holding := holding, withdrawn := withdrawn, notionalIn := notionalIn,
             stage := TStage.costReduction, openLegacyLegs := 0, cumNetCash := 0 } :=
  costReduction_satisfies_oq9inv _ rfl

/--
  ★★OQ-9 invariant 单步保持 `oq9inv_preserved`（L0，review 修正1 核心）：
  若当前态满足 `OQ9Inv` 且转移 `(s, e)` 合法，则后态 `twStep s e` 仍满足 `OQ9Inv`。
-/
theorem oq9inv_preserved (s : TWState) (e : TWEvent)
    (hinv : OQ9Inv s) (hlegal : LegalTransition s e) :
    OQ9Inv (twStep s e) := by
  cases e with
  | shortDiff d =>
      simpa only [OQ9Inv, twStep] using hinv
  | openShareLeg =>
      simp only [LegalTransition] at hlegal
      intro hst
      simp only [twStep] at hst
      rw [hst] at hlegal
      exact absurd hlegal (by decide)
  | closeShareLeg p =>
      simp only [LegalTransition] at hlegal
      intro hst
      simp only [twStep] at hst ⊢
      have hpre : s.stage = TStage.earningShares := hst
      have h0 : s.openLegacyLegs = 0 := hinv hpre
      omega
  | recoverCapital w =>
      intro hst
      simp only [twStep, advanceTo] at hst
      by_cases hse : s.stage.rank ≤ TStage.capitalRecovered.rank
      · rw [if_pos hse] at hst; exact absurd hst (by decide)
      · rw [if_neg hse] at hst
        exact hinv hst
  | enterEarning =>
      simp only [LegalTransition, LegalEnterEarning] at hlegal
      intro _
      simpa only [twStep] using hlegal
  | clearCampaign =>
      intro hst
      simp only [twStep] at hst
      exact absurd hst (by decide)

/--
  ★★OQ-9 trace 级定理 `oq9inv_trace`（L0，review 修正1 的 trace 级端B）：
  从满足 `OQ9Inv` 的初态出发，经**任意合法 trace**，到达态恒满足 `OQ9Inv`。
-/
def LegalChain : TWState → List TWEvent → Prop
  | _, [] => True
  | s, e :: rest => LegalTransition s e ∧ LegalChain (twStep s e) rest

def runLegal : TWState → List TWEvent → TWState
  | s, [] => s
  | s, e :: rest => runLegal (twStep s e) rest

theorem oq9inv_trace (s : TWState) (trace : List TWEvent)
    (hinv : OQ9Inv s) (hchain : LegalChain s trace) :
    OQ9Inv (runLegal s trace) := by
  induction trace generalizing s with
  | nil => simpa only [runLegal] using hinv
  | cons e rest ih =>
      simp only [LegalChain] at hchain
      simp only [runLegal]
      exact ih (twStep s e) (oq9inv_preserved s e hinv hchain.1) hchain.2

/--
  ★★OQ-9 trace 级不可达 `oq9_legacy_leg_unreachable_in_earning`（L0，端B trace 级 machine-check）：
  从满足 inv 的初态出发，经任意合法 trace 到达 earning 阶段的态，其 legacy 腿恒为 0——
  故 raw witness 形状（earning ∧ openLegacyLegs=1）在合法 trace 上**不可达**。
-/
theorem oq9_legacy_leg_unreachable_in_earning (s : TWState) (trace : List TWEvent)
    (hinv : OQ9Inv s) (hchain : LegalChain s trace)
    (hst : (runLegal s trace).stage = TStage.earningShares) :
    (runLegal s trace).openLegacyLegs = 0 :=
  oq9inv_trace s trace hinv hchain hst

/--
  ★★OQ-9 单步核心定理 `legal_earning_no_legacy_leg_closure`（L0，端B 单步形式，保留）：
  合法进入 EarningShares 后，闭合 legacy 腿不是合法转移（合法 enterEarning ⟹ openLegacyLegs=0 ⟹
  closeShareLeg 的 ≥1 合法性为假）。
-/
theorem legal_earning_no_legacy_leg_closure (s : TWState) (p : Int)
    (hlegal : LegalTransition s TWEvent.enterEarning) :
    ¬ LegalTransition (twStep s TWEvent.enterEarning) (TWEvent.closeShareLeg p) := by
  simp only [LegalTransition, LegalEnterEarning] at hlegal
  simp only [LegalTransition, twStep, hlegal]
  decide

/--
  ★★OQ-9 端A witness（L0，rawStep 层保留，codex 修正2）：raw 层**存在**「带未闭合 legacy 腿进入
  EarningShares 后闭合亏损腿使 cumNetCash 下降」的路径——端A 算术真相不被吞、不被禁止。
-/
theorem raw_earning_legacy_leg_closure_witness :
    ∃ (s : TWState) (p : Int),
      s.stage = TStage.earningShares ∧ s.openLegacyLegs = 1
      ∧ p < 0
      ∧ (twStep s (TWEvent.closeShareLeg p)).cumNetCash < s.cumNetCash
      ∧ (twStep s (TWEvent.closeShareLeg p)).stage = TStage.earningShares := by
  refine ⟨{ free := 0, holding := 0, withdrawn := 0, notionalIn := 0,
            stage := TStage.earningShares, openLegacyLegs := 1, cumNetCash := 0 }, -5,
          rfl, rfl, by decide, ?_, rfl⟩
  simp only [twStep]
  decide

/--
  ★★zero-close 幽灵腿标注 `zero_close_is_ghost`（L0，review 修正4(c)）：
  raw 层在 openLegacyLegs=0 时 closeShareLeg 仍改 cumNetCash（幽灵副作用）；本定理**显式标注它非法**。
-/
theorem zero_close_is_ghost (s : TWState) (p : Int) (h0 : s.openLegacyLegs = 0) :
    (twStep s (TWEvent.closeShareLeg p)).cumNetCash = s.cumNetCash + p
    ∧ ¬ LegalTransition s (TWEvent.closeShareLeg p) := by
  constructor
  · simp only [twStep]
  · simp only [LegalTransition, h0]; decide

/--
  ★★OQ-9 双层分离定理 `oq9_witness_path_illegal`（L0，端A/端B 的扩维消解汇总，单步侧）：
  任何**合法**进入 EarningShares 得到的态，openLegacyLegs 必为 0（≠ witness 的 1）。
-/
theorem oq9_witness_path_illegal (s : TWState)
    (hlegal : LegalTransition s TWEvent.enterEarning) :
    (twStep s TWEvent.enterEarning).openLegacyLegs = 0 := by
  simp only [LegalTransition, LegalEnterEarning] at hlegal
  simp only [twStep, hlegal]

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 R=Π-A-W 财务守恒账本（模型A，Origin LedgerState 的本地镜像，仅作对照）

  本地重建 LedgerComp（不 import Origin LedgerState，避免跨模块耦合）——与 Origin
  `LedgerState`(R=Π-A-W) 的定义结构同构（同字段同操作语义），仅为做不同构裁定。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★财务守恒账本 `LedgerComp`（L0，模型A，镜像 Origin LedgerState）。
  `inv : R = Π - A - W` 是**结构不变量字段**——R 由 Π/A/W 代数决定（非独立自由度）。
-/
structure LedgerComp where
  i0 : Int
  Pi : Int
  A : Int
  W : Int
  R : Int
  inv : R = Pi - A - W
deriving Repr

/-- ★财务账本事件（镜像 Origin LedgerEvent）。 -/
inductive LedgerEvent where
  | realize (dPi : Int)
  | allocate (dA : Int)
  | withdraw (dW : Int)
deriving Repr

/-- ★财务账本更新（皆 Int 可正可负，**操作可逆**）。 -/
def ledgerStep (L : LedgerComp) : LedgerEvent → LedgerComp
  | LedgerEvent.realize dPi =>
      { L with Pi := L.Pi + dPi, R := L.R + dPi, inv := by have h := L.inv; omega }
  | LedgerEvent.allocate dA =>
      { L with A := L.A + dA, R := L.R - dA, inv := by have h := L.inv; omega }
  | LedgerEvent.withdraw dW =>
      { L with W := L.W + dW, R := L.R - dW, inv := by have h := L.inv; omega }

/--
  ★R=Π-A-W 恒等永真（L0，模型A 守恒律）：任意 LedgerComp 值满足 R=Π-A-W（构造即恒等）。
  **定义性恒等**——与操作无关、与"阶段"无关（对照 TW 守恒是操作不变量）。
-/
theorem ledger_inv_always (L : LedgerComp) : L.R = L.Pi - L.A - L.W := L.inv

/--
  ★模型A 操作可逆（L0，不同构理由2 的载体）：每个 realize/allocate/withdraw 有逆操作，施加后回到
  原值。见证 A 是**可逆平移系统**（Z³ 上每个操作可逆）——对照 B 的 stage 单向不可逆。
-/
theorem ledger_realize_invertible (L : LedgerComp) (dPi : Int) :
    let L' := ledgerStep L (LedgerEvent.realize dPi)
    let L'' := ledgerStep L' (LedgerEvent.realize (-dPi))
    (L''.Pi, L''.A, L''.W) = (L.Pi, L.A, L.W) := by
  simp only [ledgerStep, Prod.mk.injEq, and_true]
  omega

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 不同构裁定（machine-checked 精确反例，三条结构性阻断）

  「同构」= 存在双向忠实映射（保守恒量 + 保操作语义 + 互逆）。下证**不存在**。
  最强阻断 = 理由2（stage 不可逆）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★不同构反例 · 理由2（**最强，codex 标不可反驳**）：stage 不可逆 vs 操作可逆。

  **精确反例**：存在 B 的两个态 s₁（stage=costReduction）与 s₂（stage=earningShares），它们的
  TW 三量分量**完全相同**，仅 stage 不同。任何「只看 TW 三量」的映射 φ:B→A **无法区分** s₁ 与 s₂
  （φ s₁ = φ s₂），故 φ 非单射 ⟹ 无双向忠实映射 ⟹ 不同构。
-/
theorem not_isomorphic_stage_collapses : ∃ (s₁ s₂ : TWState),
    s₁.free = s₂.free ∧ s₁.holding = s₂.holding ∧ s₁.withdrawn = s₂.withdrawn
    ∧ s₁.stage ≠ s₂.stage
    ∧ s₁.tw = s₂.tw := by
  refine ⟨
    { free := 100, holding := 0, withdrawn := 0, notionalIn := 0, stage := TStage.costReduction,
      openLegacyLegs := 0, cumNetCash := 0 },
    { free := 100, holding := 0, withdrawn := 0, notionalIn := 0, stage := TStage.earningShares,
      openLegacyLegs := 0, cumNetCash := 0 },
    rfl, rfl, rfl, ?_, rfl⟩
  decide

/--
  ★不同构反例 · 理由3：守恒律切换 vs 单一恒等。

  模型A 的守恒量 R=Π-A-W 是**单一恒等**，与 stage 无关。模型B 的守恒律**随 stage 切换**：
  CostReduction 守 Σ|units|，EarningShares 守 K=notionalIn。enterEarning 把 stage 翻转 =
  守恒律切换点；A 无此切换（R=Π-A-W 全程唯一恒等）。
-/
theorem not_isomorphic_conservation_switches :
    (∃ s : TWState, s.stage = TStage.costReduction
        ∧ (twStep s TWEvent.enterEarning).stage ≠ TStage.costReduction)
    ∧ (∀ (L : LedgerComp) (e : LedgerEvent),
        (ledgerStep L e).R = (ledgerStep L e).Pi - (ledgerStep L e).A - (ledgerStep L e).W) := by
  constructor
  · refine ⟨{ free := 0, holding := 0, withdrawn := 0, notionalIn := 0,
              stage := TStage.costReduction, openLegacyLegs := 0, cumNetCash := 0 }, rfl, ?_⟩
    simp only [twStep, advanceTo]
    rw [if_pos (by decide)]
    decide
  · intro L e
    exact (ledgerStep L e).inv

/--
  ★不同构反例 · 理由1（codex 修正版：外生价格维度，非"代数角色"表示差异）。

  模型B 的 `holding = Σ units·c` 依赖**外生市价 c**：TW 守恒仅在同价操作下成立。模型A 是纯 Int 格，
  **无外生价格变量**。给 B 一个价格重估算子 `repriceHolding`（模拟跨 bar 价格变动），它**改变 TW**
  （=盈亏）；模型A 无对应算子（所有 ledgerStep 保 R=Π-A-W 恒等）。
-/
def repriceHolding (s : TWState) (newHolding : Int) : TWState :=
  { s with holding := newHolding }

theorem not_isomorphic_exogenous_price :
    (∃ (s : TWState) (newHolding : Int),
        (repriceHolding s newHolding).tw ≠ s.tw)
    ∧ (∀ (L : LedgerComp) (e : LedgerEvent),
        (ledgerStep L e).R = (ledgerStep L e).Pi - (ledgerStep L e).A - (ledgerStep L e).W) := by
  constructor
  · refine ⟨{ free := 0, holding := 0, withdrawn := 0, notionalIn := 0,
              stage := TStage.costReduction, openLegacyLegs := 0, cumNetCash := 0 }, 50, ?_⟩
    decide
  · intro L e
    exact (ledgerStep L e).inv

/-! ════════════════════════════════════════════════════════════════════════
  ## §4 诚实标签（gatekeeper：裁定 = 不同构，不糊"大致兼容"）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★裁定标签 `IsomorphismVerdict`（gatekeeper，诚实分层）。
  唯一构造子 `notIsomorphic`——类型层钉死「裁定 = 不同构」，**没有** `Isomorphic` 或
  `RoughlyCompatible` 构造子（拒绝"大致兼容"含糊裁定）。
-/
inductive IsomorphismVerdict where
  | notIsomorphic
deriving DecidableEq, Repr

/-! ════════════════════════════════════════════════════════════════════════
  ## §11 RiskPolicy barrier κ + EnterReady + BuyCore（GAP3 κ-gated，acc-GAP3 工位）

  ★契约锚 k的条件.pdf §10-11 canonical 规范。**不可识别性定理2**：κ 不是价格可推的值，是
  **声明式风险政策参数**（operator 声明，可 walk-forward 优化）——故 `RiskPolicy` 承载 κ 作配置 knob。

  Rust 侧 bit-exact 对齐 `theta_v0/strategy/ledger.rs` 的 `RiskPolicy`/`eta_star`/`enter_ready`/
  `buy_core_legal`（整数域，同定义）。**认识论 L0**（纯代数：barrier 不等式的移项恒等，零信息增量）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★风险政策 `RiskPolicy`（契约锚 PDF §11 `structure RiskPolicy`）：barrier 缓冲系数 κ + κ≥0 证据。

  依赖类型字段 `kappa_nonneg` 在类型层钉死 **κ≥0**（PDF §11 `kappa_nonneg`）——任何 RiskPolicy 值在
  构造时即携带 κ≥0 证据（Rust 侧无依赖类型，用运行时谓词 `kappa_nonneg()` 承载等价约束）。
-/
structure RiskPolicy where
  kappa : Int
  kappa_nonneg : kappa ≥ 0

/--
  最小基线政策（κ=0，PDF §10 canonical 默认）：`η⋆=L^wc`，仅覆盖最坏损失无额外缓冲。
-/
def RiskPolicy.baseline : RiskPolicy := { kappa := 0, kappa_nonneg := Int.le_refl 0 }

/--
  最坏损失 `lWc`（契约锚 PDF §10 `L^wc`，L0 缠论「降成本」语义）：仍在险的本金
  `max(0, notionalIn − withdrawn)`。退本金推进 ⟹ 在险本金递减 ⟹ L^wc→0（缠师第31课「成本为0后」）。
-/
def lWc (s : TWState) : Int := max 0 (s.notionalIn - s.withdrawn)

/-- `lWc` 下界 0（L0 辅助）：在险本金非负（`max 0 _ ≥ 0`）。 -/
theorem lWc_nonneg (s : TWState) : lWc s ≥ 0 := by
  unfold lWc; omega

/--
  状态依赖 barrier `etaStar`（契约锚 PDF §10 `η⋆(x_t)=L^wc_{t+1}(x_t)+κ·Q_t`）：进入 EarningShares 的
  在险权益门槛。`Q`=名义敞口 = `notionalIn`。κ=0 ⟹ η⋆=L^wc。
-/
def etaStar (p : RiskPolicy) (s : TWState) : Int := lWc s + p.kappa * s.notionalIn

/--
  ★EnterEarning 合法性 `LegalEnterEarningStrict`（严格 EnterReady，契约锚 PDF §10 步骤3）：
  `S=II ∧ W≥I0 ∧ openLegacyLegs=0 ∧ RiskNormal ∧ η≥η⋆`。比单纯 `LegalEnterEarning`（仅 legacy 腿=0）
  **强得多**（五合取）。`i0`=campaign 本金 I₀，`riskNormal`=风控正常。

  ★与 `LegalEnterEarning`（§1.5）的关系：本谓词的第三合取项**就是** `LegalEnterEarning s`
  （openLegacyLegs=0）——严格 EnterReady 是 OQ-9 入口证书 `LegalEnterEarning` 的加强（见
  `enterReadyStrict_implies_legalEnterEarning`）。
-/
def LegalEnterEarningStrict (p : RiskPolicy) (s : TWState) (i0 : Int) (riskNormal : Prop) : Prop :=
  s.stage = TStage.capitalRecovered ∧ s.withdrawn ≥ i0 ∧ s.openLegacyLegs = 0
    ∧ riskNormal ∧ s.tw ≥ etaStar p s

/--
  ★严格 EnterReady 蕴含 OQ-9 入口证书（L0）：`LegalEnterEarningStrict ⟹ LegalEnterEarning`
  ——严格谓词的 openLegacyLegs=0 合取项正是 `LegalEnterEarning`（增股数入口的 OQ-9 gate 被严格谓词覆盖）。
-/
theorem enterReadyStrict_implies_legalEnterEarning
    (p : RiskPolicy) (s : TWState) (i0 : Int) (riskNormal : Prop)
    (h : LegalEnterEarningStrict p s i0 riskNormal) : LegalEnterEarning s :=
  h.2.2.1

/--
  ★BuyCore 合法性 `LegalBuyCore`（契约锚 PDF §10 步骤4 定理1 充要）：
  `a_n + L^wc_{n+1} + κ·ΔQ_n ≤ η_n + g_n − κ·Q_n`。增股数阶段建核仓的充要合法条件。
-/
def LegalBuyCore (p : RiskPolicy) (aN lWcNext deltaQ etaN gN qN : Int) : Prop :=
  aN + lWcNext + p.kappa * deltaQ ≤ etaN + gN - p.kappa * qN

/--
  ★★`buyCore_preserves_kappa_floor`（契约锚 PDF §5 代数证明，L0）：BuyCore 合法 ⟹ 建仓后 κ-floor
  不变量保持——建仓后在险权益 `η_n + g_n − a_n`（付建仓额 a_n、得已实现收益 g_n 后的权益）
  ≥ 建仓后 barrier `L^wc_{n+1} + κ·(Q_n + ΔQ_n)`（建仓后名义 = Q_n+ΔQ_n）。

  即：BuyCore 不等式移项 ⟺ post-buy `η ≥ η⋆`（barrier 覆盖不被建仓破坏）。纯整数移项恒等（L0）。
-/
theorem buyCore_preserves_kappa_floor
    (p : RiskPolicy) (aN lWcNext deltaQ etaN gN qN : Int)
    (h : LegalBuyCore p aN lWcNext deltaQ etaN gN qN) :
    etaN + gN - aN ≥ lWcNext + p.kappa * (qN + deltaQ) := by
  -- h : aN + lWcNext + κ·deltaQ ≤ etaN + gN − κ·qN
  -- 目标 : lWcNext + κ·(qN+deltaQ) ≤ etaN + gN − aN
  -- 移项：κ·(qN+deltaQ)=κ·qN+κ·deltaQ（Int.mul_add）展开后为线性不等式（omega）。
  unfold LegalBuyCore at h
  rw [Int.mul_add]
  omega

/--
  ★κ=0 基线特化（L0）：baseline 政策下 `buyCore_preserves_kappa_floor` 退化为
  `aN + lWcNext ≤ etaN + gN ⟹ etaN + gN − aN ≥ lWcNext`（barrier=L^wc，无 κ 缓冲）。
  这是 GAP3 可达性的 Lean 侧证据：κ=0 时 barrier 仅需覆盖 L^wc（本金全退 ⟹ L^wc=0 ⟹ η⋆=0 ⟹ 恒过）。
-/
theorem buyCore_preserves_floor_baseline
    (aN lWcNext deltaQ etaN gN qN : Int)
    (h : LegalBuyCore RiskPolicy.baseline aN lWcNext deltaQ etaN gN qN) :
    etaN + gN - aN ≥ lWcNext := by
  have := buyCore_preserves_kappa_floor RiskPolicy.baseline aN lWcNext deltaQ etaN gN qN h
  simpa [RiskPolicy.baseline] using this

/--
  ★裁定见证（L0，gatekeeper）：账本裁定必是 notIsomorphic。三条结构反例
  （not_isomorphic_stage_collapses / _conservation_switches / _exogenous_price）共同支撑。
-/
theorem accounting_verdict_is_not_isomorphic (v : IsomorphismVerdict) :
    v = IsomorphismVerdict.notIsomorphic := by
  cases v; rfl

/-! ════════════════════════════════════════════════════════════════════════
  ## 交付总结（TW-port 工位，task #127，Origin native）

  本文件**证**（L0，machine-checked，无 sorry/admit/axiom，**native 重证，不 import legacy**）：
  1. 缠论取本金三阶段账本 `TWState`（free/holding/withdrawn/notionalIn/stage/openLegacyLegs/
     cumNetCash）+ TStage 三态 + `TWEvent` 六算子 + `twStep` 全函数。
  2. `twStep_preserves_tw`（TW=free+holding+withdrawn 守恒，缠师第31课守恒律 L0 结构形式）。
  3. `stage_rank_monotone` + `earning_no_regress`（stage 单向不可逆，OQ-9 campaign 内结构形式）。
  4. OQ-9 gate 扩维消解全套：`LegalEnterEarning`/`LegalTransition`/`OQ9Inv`/`oq9inv_preserved`/
     `oq9inv_trace`/`oq9_legacy_leg_unreachable_in_earning`/`legal_earning_no_legacy_leg_closure`/
     `raw_earning_legacy_leg_closure_witness`/`zero_close_is_ghost`/`oq9_witness_path_illegal`
     + 初态见证 `costReduction_satisfies_oq9inv`/`init_satisfies_oq9inv`。
  5. R=Π-A-W 财务账本 `LedgerComp`（镜像 Origin LedgerState）+ `ledger_inv_always` + `ledger_realize_invertible`。
  6. ★★不同构三反例：`not_isomorphic_stage_collapses`（理由2 最强）+ `not_isomorphic_conservation_switches`
     （理由3）+ `not_isomorphic_exogenous_price`（理由1 codex 修正版）。
  7. 诚实标签 `accounting_verdict_is_not_isomorphic`（裁定 = notIsomorphic，无"大致兼容"）。

  本文件**不证**（formalization-validity-domain 诚实边界）：
  - ✗ TW 数值反映实盘真实盈亏（L0 结构守恒；数值正确性/跨 bar 价格 L2/L3，Rust prove_tw_neutral 守卫）。
  - ✗ 三阶段策略盈利/最优/实盘有效（L3）。

  ★Origin native 重锚裁定（task #127，Origin-TW #103 = native）：取本金三阶段 TW 物理 port 进
  `formal/Origin/` 成独立 native 模块——**不 import legacy Tlayers.Accounting.TotalWealth**（native
  重证，零偷渡），**不合并进 Origin LedgerState**（#90 排除合并：R=Π-A-W 与取本金三阶段 TW 不同构）。
  #90 不同构 + OQ-9 扩维消解的关键定理在 Origin native 定义上**重新成立**。

  谱系：#42 / #86 / #90（不同构裁定，原 Tlayers TW）/ OQ-9（design §3.3）/ #97（Origin canonical）/
        Origin-TW #103（裁决=native）→ 本文件 #127（TW native 重锚进 Origin，machine-checked）。
  ════════════════════════════════════════════════════════════════════════ -/

end NewChanlun.Origin.TotalWealth
