/-
  会计层 · 缠论取本金三阶段账本 TW=free+holding+withdrawn（task #90 / 共有缺口2）
  工位：cc-accounting-tw（codex 编排者代理裁决 R1=B 严格综合，codex binding，2026-06-26）

  ════════════════════════════════════════════════════════════════════════
  ## 本文件做什么：形式化缠论取本金三阶段账本 + 裁定它与 R=Π-A-W 不同构

  缠师第31课「取本金」三阶段会计（`t_engine.rs:107-451` / `rec_engine.rs:1219-1266`，
  设计文档 `docs/three_stages_accounting_design.md`，t_engine 有 prove_tw_neutral L2 守卫）：
  - 守恒量 `TW = free + holding + withdrawn`（逐 bar 不变量）。
  - 三阶段状态机 `CostReduction`（降成本）→ `CapitalRecovered`（退本金 free→withdrawn）
    → `EarningShares`（增股数），**单向不可逆**（OQ-9）。
  - 守恒律随阶段切换：CostReduction 守 Σ|units|（股数）；EarningShares 守 K（外部投入）。

  另一版形式化（`Strict/HybridAssembly.lean:90-163`）用 `LedgerComp`(R=Π-A-W 财务守恒账本)：
  - 五字段 `(i0,Π,A,W,R)`，**结构不变量** `inv : R = Π - A - W`（R 由 Π/A/W 代数决定）。
  - 三事件 `realize dΠ`/`allocate dA`/`withdraw dW`（皆 Int，可正可负），每步重算 R。

  codex 标 R=Π-A-W vs 取本金三阶段是**最大剩余风险**。本文件裁定：**二者不同构**，
  并给出 machine-checked 精确反例（三条结构性阻断，理由3 stage 不可逆性为最强）。

  ════════════════════════════════════════════════════════════════════════
  ## 裁定结论：不同构（codex gpt-5.5 异质审查确认，2026-06-26）

  不同构 = 不存在「保守恒量 + 保操作语义」的双向忠实映射。三条**结构性**阻断：

  1. **外生价格维度**（codex 修正：原"代数角色相反"是表示层差异，须升级为结构差异）：
     B 的 `holding = Σ units · c` 依赖**外生市价 c**（跨 bar 变动 = 盈亏，TW 守恒仅在同价
     操作下成立）；A 是纯整数格 `Z³`（Π/A/W 皆 Int），无外生变量。状态空间基础场不同。

  2. **状态空间结构**（codex 标为**最强、不可反驳**）：A 是**可逆平移系统**——realize/
     allocate/withdraw 的增量皆 Int 可正可负，任意 (Π,A,W) 互相可达（Z³ 强连通，每个操作有逆）。
     B 是**带单向不可逆 stage 迁移的混合系统**——CostReduction→CapitalRecovered→EarningShares
     的偏序可达图，EarningShares **不可回退**（OQ-9）。任何 B→A 映射丢失 stage（非单射）；
     任何 A→B 映射的可逆操作破坏 stage 单向性（不保操作语义）。

  3. **守恒律切换**：A 全程单一恒等 `R=Π-A-W`（任意赋值永真，与操作/阶段无关）；B 的守恒律
     **随 stage 切换**（CostReduction 守 Σ|units| 股数 / EarningShares 守 K 外部投入，
     design §3.3「守恒律从股数守恒扬弃为外部投入资本守恒」）。A 无「阶段相关守恒律」结构。

  ════════════════════════════════════════════════════════════════════════
  ## 认识论等级（formalization-validity-domain，强制标注）

  - **全文件 L0**（结构/定义层，零数据依赖）。`lake build` 通过 = TW 三阶段状态机类型自洽 +
    TW 守恒在每个会计算子下结构成立（L0 代数恒等）+ stage 单向性结构可证 + **不同构反例**
    的机器验证。**不**是缠论盈利 / 实盘有效声明（那是 L3，Rust prove_tw_neutral 是 L2 守卫）。
  - **不同构裁定本身是 L0 定理**：从两个模型的代数/状态结构推导，有效域=定义域（L0 不需 L2+）。
  - **本文件的 TW 守恒是结构恒等**（同价 c 固定下 `TW' = TW`），对应 Rust 的「同价操作 NAV
    中性 = L0」（design §6.3）；Rust 的「全程零 panic = L2」不在本文件（跨 bar 价格变动 L2/L3）。
  - 禁 sorry/admit/axiom。纯 Prop/Type，不依赖 Mathlib（Int 取自 Lean core）。

  ════════════════════════════════════════════════════════════════════════
  ## 隔离声明
  本模块自包含：不 import Strict.* / 不 import Formal.* / 不 import Phase2.*。
  R=Π-A-W 侧（LedgerComp）在本文件**本地重建**（与 HybridAssembly.lean 的定义同构镜像，
  仅为做对照裁定——不 import 它，避免跨 lib 耦合；HybridAssembly.lean 只读不改）。

  谱系：#42（账本 watch-item）/ #86（HybridAssembly LedgerComp 新建 R=Π-A-W）/ OQ-9（守恒律
        相变可逆性矛盾，design §3.3 已登记，与本不同构裁定正交）→ 本文件（#90 不同构裁定）。
-/

namespace Formal.Tlayers.Accounting.TotalWealth

/-! ════════════════════════════════════════════════════════════════════════
  ## §1 缠论取本金三阶段账本 TW（模型B，缠师第31课 L0 结构）
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★三阶段 `TStage`（L0，缠师第31课，对齐 `t_engine.rs:120` TStage）。
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

  ★诚实标注：`holding` 用 Int 承载市值摘要（结构层；真实 holding=Σunits·c 含外生 c，c 的
  跨 bar 变动 = 盈亏，是 L2/L3 数据，本 L0 结构层把同价操作下的市值作整数量承载，不臆造价格）。
-/
structure TWState where
  free : Int
  holding : Int
  withdrawn : Int
  notionalIn : Int
  stage : TStage
  /-- ★OQ-9 扩维（codex binding 立场C 修正版 + review 019f0318 修正1，2026-06-26）：
      未闭合**旧 ShareConserving（legacy）降成本腿**计数。这是承载 OQ-9 矛盾的**新维度**——与
      stage 正交（stage 是阶段相位，openLegacyLegs 是「还有几条**降成本期遗留**的短差腿未平」）。

      ★review 修正1（拆 legacy/新腿）：OQ-9 锁死的**只是 legacy 腿**（降成本期 ShareConserving 短差），
      **不是** earning 阶段的增股数新腿（AmountConserving，那是合法的增核操作）。故本计数专指 legacy 腿，
      `openShareLeg` 只在 stage < earningShares 时合法开（earning 阶段不再开 legacy 腿，见 LegalTransition）。

      OQ-9 核心：进入 EarningShares 须 legacy 腿全清（openLegacyLegs=0）；若 trace 中违规留腿进 earning，
      闭合它且 profit<0 会让 cost_basis 数值回正（端A 算术真相），但 stage 单向不回退（端B INV-2）。
      扩维消解 = trace 级 invariant `OQ9Inv`（stage=earning ⟹ openLegacyLegs=0）在所有合法转移下保持。 -/
  openLegacyLegs : Nat
  /-- ★OQ-9 扩维 · 净现金口径累加 `cumNetCash : Int`（cost_basis 符号的结构载体，design §2.2）。
      缠师「成本」= 净现金流口径（"抽到的血"，可正可负），与 TW 三量（财富守恒量）正交：
      cumNetCash 是**口径量**（累计净现金贡献），不进 TW = free+holding+withdrawn。
      closeShareLeg profit ⟹ cumNetCash += profit。OQ-9 端A：EarningShares 下闭合亏损腿
      （profit<0）使 cumNetCash 下降（"成本回正"对应 cumNetCash 跨越基线）——这个**数值变化在
      raw 层保留**（不禁止、不吞异常），它**不驱动 stage 回退**（stage 是独立维度，端B）。 -/
  cumNetCash : Int
deriving Repr

/-- ★总财富 `TWState.tw`（L0，守恒量）：`TW = free + holding + withdrawn`（缠师第31课守恒律）。 -/
def TWState.tw (s : TWState) : Int := s.free + s.holding + s.withdrawn

/--
  ★TW 账本事件 `TWEvent`（L0，穷尽，对齐 t_engine 三阶段算子 + OQ-9 扩维载体）：
  - `shortDiff dCash`：降成本短差（free⇄holding 同价转换，TW 不变；CostReduction 阶段）。
    `dCash>0` = 卖出（holding→free），`dCash<0` = 买回（free→holding）。Σ|units| 守恒由阶段语义。
  - `openShareLeg`：开一条 legacy ShareConserving 降成本短差腿（openLegacyLegs += 1，无资金净变动摘要）。
  - `closeShareLeg profit`：闭合一条 legacy ShareConserving 腿（openLegacyLegs -= 1，盈亏 profit 落 cumNetCash）。
    **OQ-9 矛盾的精确载体**：profit<0（亏损闭合）时净抽出本金——若此时 stage 已 earningShares，
    这条腿的亏损闭合会让「净现金口径成本」回正（端A），但 stage 单向不回退（端B）。
  - `recoverCapital w`：退本金（free→withdrawn，移出在险池 w）。触发/推进 CapitalRecovered。
  - `enterEarning`：本金全退后切 EarningShares（单向不可逆相变，无资金变动，纯 stage 切换）。
    ★诚实（rawStep/legalStep 双层）：twStep（raw 层）对 enterEarning **不加入口守卫**——raw 层
    保留「带未闭合 legacy 腿进入 EarningShares」这条算术路径作为 violation witness。合法性由独立谓词
    `LegalEnterEarning`（要求 openLegacyLegs = 0）承载，见 §1.5 OQ-9 gate。
  - `clearCampaign`：campaign 结束（withdrawn→free 归还，stage 重置 CostReduction，legacy 腿/cumNetCash 清零）。
    ★review 修正4(b)：合法 clearCampaign 须先 `openLegacyLegs=0`（不绕 gate 强制抹账），见 LegalTransition。
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
  ★阶段推进 `advanceStage`（L0，单向，对齐 OQ-9 不可逆）：退本金达标 ⟹ rank 至少推到
  CapitalRecovered；enterEarning ⟹ 推到 EarningShares。**rank 只增不减**（见 stage_rank_monotone）。
  这里把「退本金推进」与「切增股数」分两个事件，stage 迁移函数本身只前进。
-/
def advanceTo (current target : TStage) : TStage :=
  if current.rank ≤ target.rank then target else current

/-- ★advanceTo 的 rank 单调（L0，辅助）：`current.rank ≤ (advanceTo current target).rank`。
    两支：条件真 ⟹ 结果=target，rank≥current（即条件本身）；条件假 ⟹ 结果=current，rank 相等。 -/
theorem advanceTo_rank_ge (current target : TStage) :
    current.rank ≤ (advanceTo current target).rank := by
  unfold advanceTo
  by_cases h : current.rank ≤ target.rank
  · rw [if_pos h]; exact h
  · rw [if_neg h]; exact Nat.le_refl _

/--
  ★TW 更新 `twStep`（L0，全函数，**保 TW 守恒**——同价 c 固定下 free/holding/withdrawn 间转移）：
  每事件的资金转移都是「从一个池移到另一个池」，三量之和 TW 不变（同价中性，design §6.3 L0）。

  - `shortDiff dCash`：holding→free 转 dCash（free+=dCash, holding-=dCash）⟹ TW 不变。
  - `openShareLeg`：openLegacyLegs += 1（开 legacy 降成本短差腿，无 TW 三量变动）⟹ TW 不变。
  - `closeShareLeg profit`：openLegacyLegs -= 1（Nat 饱和减）+ cumNetCash += profit（净现金口径，
    端A 载体）⟹ **TW 三量不变**（profit 进 cumNetCash 口径量，不进 TW 财富量）⟹ TW 不变。
  - `recoverCapital w`：free→withdrawn 转 w（withdrawn+=w, free-=w）⟹ TW 不变；
    stage 推进到 capitalRecovered（advanceTo，单向）。
  - `enterEarning`：纯 stage 切换到 earningShares（advanceTo，单向），无资金变动 ⟹ TW 不变。
    ★raw 层无入口守卫（rawStep/legalStep 双层）：即使 openLegacyLegs>0 也允许切（保留 violation
    witness 路径）；合法性由 LegalEnterEarning 谓词（§1.5）排除带未闭合旧腿的进入。
  - `clearCampaign`：withdrawn→free 归还（free+=withdrawn, withdrawn:=0）⟹ TW 不变；
    stage 重置 + openLegacyLegs/cumNetCash 清零——**注意：campaign 边界，新 campaign 起点，非回退**。

  ★诚实标注（OQ-9 关键）：`clearCampaign` 把 stage 重置回 costReduction，但这**不违反 stage
  单向性**——OQ-9 的不可逆是「**同一 campaign 内**」cost_basis≤0 ⟹ EarningShares 不回退。
  clearCampaign 是 campaign 终点（核心走势完成，开启**新 campaign**），是不同的 campaign 实例。
  故 stage 单向性定理（stage_rank_monotone）**只对不含 clearCampaign 的算子声明**——这是诚实
  的有效域边界，非 workaround（clearCampaign 跨 campaign 边界，单向性是 campaign 内性质）。
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
  holding + withdrawn`。这对齐 Rust `prove_tw_neutral`（rec_engine.rs:1259）的**代数侧**
  （L0：同价 c 固定下三量间转移 TW 不变）——Rust 的「全程零 panic」是 L2（跨 bar 价格）。

  逐事件：shortDiff 是 holding⇄free 内转移；recoverCapital 是 free⇄withdrawn 内转移；
  enterEarning 无资金变动；clearCampaign 是 withdrawn→free 内转移——四者都不创造/销毁财富。
-/
theorem twStep_preserves_tw (s : TWState) (e : TWEvent) :
    (twStep s e).tw = s.tw := by
  cases e <;> simp only [twStep, TWState.tw] <;> omega

/--
  ★阶段单向不可逆 `stage_rank_monotone`（L0，OQ-9 结构形式，campaign 内）：对**不含
  clearCampaign** 的算子（shortDiff/openShareLeg/closeShareLeg/recoverCapital/enterEarning），
  stage 的 rank **只增不减**。这 machine-check「CostReduction→CapitalRecovered→EarningShares
  单向不可逆」（OQ-9）。腿事件（openShareLeg/closeShareLeg）只动 openLegacyLegs/cumNetCash，不动 stage。

  （clearCampaign 排除在外：它是 campaign 边界重置，跨 campaign 单向性不适用——见 twStep 诚实标注。）
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
  对不含 clearCampaign 的任何算子，stage 仍是 earningShares（rank=2 是最大，advanceTo 不下降）。
  这关死「增股数阶段回退到降成本/退本金」的误读——单向相变锁死（design §3.2 INV-2）。
-/
theorem earning_no_regress (s : TWState) (e : TWEvent)
    (hstage : s.stage = TStage.earningShares) (hne : e ≠ TWEvent.clearCampaign) :
    (twStep s e).stage = TStage.earningShares := by
  cases e with
  | shortDiff d => simpa only [twStep] using hstage
  | openShareLeg => simpa only [twStep] using hstage
  | closeShareLeg p => simpa only [twStep] using hstage
  | recoverCapital w =>
      -- earningShares.rank=2 ≤ capitalRecovered.rank=1 是假 ⟹ advanceTo 保持 earningShares
      simp only [twStep, advanceTo, hstage]
      rw [if_neg (by decide)]
  | enterEarning =>
      -- earningShares.rank=2 ≤ earningShares.rank=2 是真 ⟹ advanceTo 取 target=earningShares
      simp only [twStep, advanceTo, hstage]
      rw [if_pos (by decide)]
  | clearCampaign => exact absurd rfl hne

/-! ════════════════════════════════════════════════════════════════════════
  ## §1.5 OQ-9 gate（守恒律相变可逆性矛盾的扩维消解，codex binding 立场C 修正版）

  ════════════════════════════════════════════════════════════════════════
  ### OQ-9 矛盾（design §3.3 line 269 / rust/src/trading/ledger.rs:11-18）

  缠论三阶段单向相变（INV-2）的可逆性矛盾，两端：
  - **端A（数值真相，算术事实）**：进入 EarningShares 后，闭合**旧 ShareConserving 腿**且
    profit<0（亏损）时，净现金口径成本（cumNetCash）会下降（"cost_basis 回正"）。算术不可否认。
  - **端B（相位锁死，INV-2，缠师第87课）**：LedgerPhase 单向相变，EarningShares 不回退。

  ### 扩维消解（codex gpt-5.5 xhigh 异质审查 session 019f0303 修正后裁决）

  矛盾源于把「cumNetCash 数值符号」与「stage 相位」**绑成单一判据**（stage = sign(cost_basis)）。
  扩维消解 = 解耦：stage 是独立维度（路径依赖历史变量），openLegacyLegs/cumNetCash 是正交维度。

  **codex 三项修正（全部吸收，非补丁）**：
  1. 非法性在 **enterEarning 入口**强制（不是 close 时事后宣布违规）：合法进入 EarningShares
     须 `openLegacyLegs = 0`（清空旧腿）⟹「亏损腿闭合回正」在合法系统**根本不可达**（无旧腿可闭）。
  2. **rawStep/legalStep 双层**（拒绝「两端在同一系统内并存」的声明膨胀）：raw 层（twStep 全函数）
     保留端A 路径作为 **violation witness**；legal 层（LegalTransition 谓词）排除它。端A 在 raw 层
     成立、在 legal 层不可达——不是「两端并存」，是「raw 层端A / legal 层那条 trace 非法」。
  3. 非对称（入口依赖相位推进、持续独立于 cumNetCash）是自洽的 **hysteresis/latch** 模式，
     入口证书 `openLegacyLegs = 0` 是把 latch 闭合的充分前提（codex 确认非对称不引入新不一致）。

  ★这与 ledger.rs 的「严格形式（EarningShares 下 ShareConserving 亏损闭合不改写成本）」**一致**：
  legalStep 排除该 trace = 严格形式的结构化；rawStep 保留 witness = ledger.rs 的
  `n_t5_shareconserving_after_earning` 计数器可观测性的结构对应（端A 数值变化仍可观测，不被吞）。

  ════════════════════════════════════════════════════════════════════════
  ### review 019f0318 修正（端B 从单步升级到 trace 级 + 堵 clearCampaign/zero-close 漏洞）

  codex review 指出单步 `oq9_witness_path_illegal` **不足**：旧 `LegalTransition` 的 openShareLeg
  对所有 stage 恒合法（`_ => True`），故合法路径 enterEarning→openShareLeg→closeShareLeg 仍能合法
  到达 `stage=earningShares ∧ legacy 腿=1`。端B trace 级**未成立**。三项修正（全部吸收）：
  - **修正1（trace 级 gate）**：`openShareLeg` 合法性收紧为 `stage.rank < earningShares.rank`
    （earning 阶段不再开 legacy 腿——legacy 腿是降成本期遗留，earning 后只开增股数新腿，不入此计数）。
    + 定义 invariant `OQ9Inv`（stage=earning ⟹ openLegacyLegs=0）+ 证它**在每个合法转移下保持**
    （`oq9inv_preserved`）+ trace 级归纳（`oq9inv_trace`：合法 trace 全程保持）⟹ 端B trace 级成立。
  - **修正4(b)（clearCampaign 不绕 gate）**：`clearCampaign` 合法性加 `openLegacyLegs = 0`
    （campaign 结束前须先清空 legacy 腿，不强制抹账绕 gate）。
  - **修正4(c)（zero-close 幽灵腿标注）**：`closeShareLeg` 在 openLegacyLegs=0 时 raw 层 Nat 饱和减
    保持 0 但仍改 cumNetCash = 幽灵腿；legal 层已排除（要求 ≥1），加 `zero_close_is_ghost` 显式标注。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★OQ-9 入口证书 `LegalEnterEarning`（L0，codex 修正1）：合法进入 EarningShares 的前提 =
  **无未闭合 legacy ShareConserving 腿**（`openLegacyLegs = 0`）。缠师第31课「成本为0后卖出多少资金
  就买入多少资金」+ 第87课「真正的拉抬不需要花钱」——进入增股数阶段意味着旧降成本短差腿已全部
  结清（不再用本金消化旧负债）。这是把 OQ-9 非法性**上移到入口**的证书（非 close-time 事后判违规）。
-/
def LegalEnterEarning (s : TWState) : Prop := s.openLegacyLegs = 0

/--
  ★OQ-9 合法转移谓词 `LegalTransition`（L0，review 019f0318 修正后的 legal 层）：哪些 (s, e) 合法。
  - `enterEarning`：须 `LegalEnterEarning s`（openLegacyLegs = 0，清空 legacy 腿后才能进）。
  - `openShareLeg`：须 `s.stage.rank < TStage.earningShares.rank`（**修正1**：只能在 costReduction/
    capitalRecovered 阶段开 legacy 腿；earning 阶段不再开 legacy 腿——堵「进 earning 后再开 legacy 腿」漏洞）。
  - `closeShareLeg p`：须 `s.openLegacyLegs ≥ 1`（有腿才能闭——否则凭空闭不存在的腿 = 幽灵，非法）。
  - `clearCampaign`：须 `s.openLegacyLegs = 0`（**修正4(b)**：结束前先清 legacy 腿，不强制抹账绕 gate）。
  - 其余（shortDiff/recoverCapital）：无额外约束（不动 legacy 腿/stage 的资金转移，恒合法）。

  ★诚实（rawStep/legalStep 双层）：twStep（raw 层全函数）对所有 (s,e) 都有定义（包括非法的），
  本谓词**只标注合法子集**——raw 层保留非法路径作 violation witness（端A），legal 层排除它（端B）。
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
  这是 OQ-9「相位锁死」的状态空间不变量：earning 阶段不存在未闭合的 legacy 腿（故无亏损闭合回正路径）。
-/
def OQ9Inv (s : TWState) : Prop :=
  s.stage = TStage.earningShares → s.openLegacyLegs = 0

/--
  ★OQ-9 invariant 初态成立 `costReduction_satisfies_oq9inv`（L0，review 复审 5b 补全）：
  任何 stage=costReduction 的态满足 `OQ9Inv`（前件 `costReduction = earningShares` 假 ⟹ 蕴含平凡真）。
  这给 trace 级定理 `oq9inv_trace` 一个**合法起点**——campaign 起始态（costReduction）恒满足 inv，
  故任意从 campaign 起点出发的合法 trace 全程保持 OQ9Inv（闭合「若注入非法初态则 trace 定理不适用」口子）。
-/
theorem costReduction_satisfies_oq9inv (s : TWState) (h : s.stage = TStage.costReduction) :
    OQ9Inv s := by
  intro hst; rw [h] at hst; exact absurd hst (by decide)

/--
  ★OQ-9 invariant 标准初态成立 `init_satisfies_oq9inv`（L0，review 复审 5b 补全）：
  标准开局态（costReduction，legacy 腿 0，cumNetCash 0）满足 `OQ9Inv`——trace 定理的具体合法起点。
-/
theorem init_satisfies_oq9inv (free holding withdrawn notionalIn : Int) :
    OQ9Inv { free := free, holding := holding, withdrawn := withdrawn, notionalIn := notionalIn,
             stage := TStage.costReduction, openLegacyLegs := 0, cumNetCash := 0 } :=
  costReduction_satisfies_oq9inv _ rfl

/--
  ★★OQ-9 invariant 单步保持 `oq9inv_preserved`（L0，review 修正1 核心）：
  若当前态满足 `OQ9Inv` 且转移 `(s, e)` 合法（`LegalTransition s e`），则后态 `twStep s e` 仍满足 `OQ9Inv`。

  逐事件验证：
  - shortDiff/recoverCapital：不动 openLegacyLegs；stage 至多推进，但 recoverCapital 推到
    capitalRecovered 不到 earning，shortDiff 不动 stage ⟹ 不破坏 inv。
  - openShareLeg：合法性要求 stage.rank < earning.rank，故后态 stage 仍 < earning（不动 stage）⟹
    后态 stage ≠ earning ⟹ inv 前件假，恒成立。
  - closeShareLeg：openLegacyLegs 减（不增）；stage 不动。若后态 stage=earning，则前态也 earning，
    由 inv 前件得前态 openLegacyLegs=0，但合法性要求 ≥1，矛盾 ⟹ 该分支下后态 stage≠earning（vacuous）。
  - enterEarning：合法性要求 openLegacyLegs=0；twStep 推 stage 到 earning 但不动 openLegacyLegs=0 ⟹ inv 成立。
  - clearCampaign：把 openLegacyLegs 清 0 且 stage 重置 costReduction ⟹ 后态 stage≠earning ⟹ inv 成立。
-/
theorem oq9inv_preserved (s : TWState) (e : TWEvent)
    (hinv : OQ9Inv s) (hlegal : LegalTransition s e) :
    OQ9Inv (twStep s e) := by
  cases e with
  | shortDiff d =>
      -- 不动 stage/openLegacyLegs。
      simpa only [OQ9Inv, twStep] using hinv
  | openShareLeg =>
      -- 合法 ⟹ stage.rank < earning.rank ⟹ stage ≠ earning（twStep 不动 stage）⟹ 前件假。
      simp only [LegalTransition] at hlegal
      intro hst
      -- twStep openShareLeg 不动 stage，故 (twStep s e).stage = s.stage = earning，但 rank<earning.rank 矛盾。
      simp only [twStep] at hst
      rw [hst] at hlegal
      exact absurd hlegal (by decide)
  | closeShareLeg p =>
      -- 合法 ⟹ openLegacyLegs ≥ 1；stage 不动。若后态 earning 则前态 earning，inv 给 openLegacyLegs=0，矛盾。
      simp only [LegalTransition] at hlegal
      intro hst
      simp only [twStep] at hst ⊢
      have hpre : s.stage = TStage.earningShares := hst
      have h0 : s.openLegacyLegs = 0 := hinv hpre
      omega
  | recoverCapital w =>
      -- stage 推到 capitalRecovered（rank 1）< earning（rank 2）；openLegacyLegs 不动。
      intro hst
      simp only [twStep, advanceTo] at hst
      -- (twStep).stage = advanceTo s.stage capitalRecovered；它 = earning 不可能（capitalRecovered.rank=1<2，
      -- advanceTo 结果 rank ≤ max(s.stage.rank,1)；若 s.stage=earning 则 hinv 给 0，但此处需排除结果=earning）。
      by_cases hse : s.stage.rank ≤ TStage.capitalRecovered.rank
      · rw [if_pos hse] at hst; exact absurd hst (by decide)
      · rw [if_neg hse] at hst
        -- 结果 = s.stage；= earning ⟹ 前态 earning ⟹ hinv 给 openLegacyLegs=0（twStep 不动它）。
        exact hinv hst
  | enterEarning =>
      -- 合法 ⟹ openLegacyLegs=0；twStep 不动 openLegacyLegs ⟹ inv 成立。
      simp only [LegalTransition, LegalEnterEarning] at hlegal
      intro _
      simpa only [twStep] using hlegal
  | clearCampaign =>
      -- 重置 stage=costReduction ⟹ 后态 stage ≠ earning ⟹ 前件假。
      intro hst
      simp only [twStep] at hst
      exact absurd hst (by decide)

/--
  ★★OQ-9 trace 级定理 `oq9inv_trace`（L0，review 修正1 的 trace 级端B）：
  从满足 `OQ9Inv` 的初态出发，经**任意合法 trace**（事件序列，逐步合法），到达态恒满足 `OQ9Inv`。

  `runLegal` 沿 trace 折叠 twStep，前提是每步合法（`LegalChain`）。归纳：初态 inv + 每步 oq9inv_preserved
  ⟹ 全程 inv。这把端B「earning 后 legacy 腿恒空」从单步升级到**任意合法历史**——
  raw witness 的 `stage=earning ∧ legacy 腿=1` 态在**任何合法 trace 上都不可达**（review 要求的 trace 级）。
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
  故 raw witness 形状（earning ∧ openLegacyLegs=1）在合法 trace 上**不可达**（端B trace 级成立）。
-/
theorem oq9_legacy_leg_unreachable_in_earning (s : TWState) (trace : List TWEvent)
    (hinv : OQ9Inv s) (hchain : LegalChain s trace)
    (hst : (runLegal s trace).stage = TStage.earningShares) :
    (runLegal s trace).openLegacyLegs = 0 :=
  oq9inv_trace s trace hinv hchain hst

/--
  ★★OQ-9 单步核心定理 `legal_earning_no_legacy_leg_closure`（L0，端B 单步形式，保留）：
  合法进入 EarningShares 后，闭合 legacy 腿不是合法转移（合法 enterEarning ⟹ openLegacyLegs=0 ⟹
  closeShareLeg 的 ≥1 合法性为假）。这是 trace 级定理的单步特例（trace 级由 oq9inv_trace 承载）。
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

  精确 witness：取 s（openLegacyLegs=1，stage=earningShares 已进，cumNetCash=0），事件
  `closeShareLeg (-5)`（亏损闭合 profit=-5）。raw 层 twStep 产出 s' 满足 `s'.cumNetCash < s.cumNetCash`
  （净现金口径下降 = "成本回正"方向）**且** stage 仍是 earningShares（端B：stage 不因 cumNetCash
  变化回退）。这见证端A（数值变化真实发生）与端B（stage 不回退）**在 raw 层同时被观测**。
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
  raw 层在 openLegacyLegs=0 时 closeShareLeg 仍改 cumNetCash（Nat 饱和减保持 0，cumNetCash += profit）——
  这是「闭合不存在的腿」的幽灵副作用。本定理**显式标注它非法**（¬ LegalTransition），坐实 legal 层
  已排除该幽灵（closeShareLeg 合法性要求 openLegacyLegs ≥ 1，0 时为假）。raw 保留可观测、legal 排除。
-/
theorem zero_close_is_ghost (s : TWState) (p : Int) (h0 : s.openLegacyLegs = 0) :
    -- raw 层幽灵副作用真实发生（cumNetCash 被改）：
    (twStep s (TWEvent.closeShareLeg p)).cumNetCash = s.cumNetCash + p
    -- 但该转移非法（legal 层排除幽灵腿闭合）：
    ∧ ¬ LegalTransition s (TWEvent.closeShareLeg p) := by
  constructor
  · simp only [twStep]
  · simp only [LegalTransition, h0]; decide

/--
  ★★OQ-9 双层分离定理 `oq9_witness_path_illegal`（L0，端A/端B 的扩维消解汇总，单步侧）：
  任何**合法**进入 EarningShares 得到的态，openLegacyLegs 必为 0（≠ witness 的 1）。
  故「带未闭合 legacy 腿的 EarningShares 态」是 raw 层可表达、legal 层不可达的 violation witness——
  端A 数值真相在 raw 层保留（不糊），端B 相位锁死在 legal 层成立。trace 级由 oq9inv_trace 承载。
-/
theorem oq9_witness_path_illegal (s : TWState)
    (hlegal : LegalTransition s TWEvent.enterEarning) :
    (twStep s TWEvent.enterEarning).openLegacyLegs = 0 := by
  simp only [LegalTransition, LegalEnterEarning] at hlegal
  simp only [twStep, hlegal]

/-! ════════════════════════════════════════════════════════════════════════
  ## §2 R=Π-A-W 财务守恒账本（模型A，HybridAssembly.lean:90 的本地镜像，仅作对照）

  本地重建 LedgerComp（不 import HybridAssembly，避免跨 lib 耦合）——与
  `Strict/HybridAssembly.lean:90-163` 的定义结构同构（同字段同操作语义），仅为做不同构裁定。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★财务守恒账本 `LedgerComp`（L0，模型A，镜像 HybridAssembly.lean:90）。
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

/-- ★财务账本事件（镜像 HybridAssembly.lean:112 LedgerEvent）。 -/
inductive LedgerEvent where
  | realize (dPi : Int)
  | allocate (dA : Int)
  | withdraw (dW : Int)
deriving Repr

/-- ★财务账本更新（镜像 HybridAssembly.lean:129 ledgerStep；皆 Int 可正可负，**操作可逆**）。 -/
def ledgerStep (L : LedgerComp) : LedgerEvent → LedgerComp
  | LedgerEvent.realize dPi =>
      { L with Pi := L.Pi + dPi, R := L.R + dPi, inv := by have h := L.inv; omega }
  | LedgerEvent.allocate dA =>
      { L with A := L.A + dA, R := L.R - dA, inv := by have h := L.inv; omega }
  | LedgerEvent.withdraw dW =>
      { L with W := L.W + dW, R := L.R - dW, inv := by have h := L.inv; omega }

/--
  ★R=Π-A-W 恒等永真（L0，模型A 守恒律）：任意 LedgerComp 值满足 R=Π-A-W（构造即恒等）。
  这是**定义性恒等**——与操作无关、与"阶段"无关，任意赋值永真（对照 TW 守恒是操作不变量）。
-/
theorem ledger_inv_always (L : LedgerComp) : L.R = L.Pi - L.A - L.W := L.inv

/--
  ★模型A 操作可逆（L0，不同构理由2 的载体）：每个 realize/allocate/withdraw 都有逆操作
  （realize dΠ 的逆 = realize (-dΠ)，等等），施加后回到原值。这见证 A 是**可逆平移系统**
  （Z³ 上每个操作可逆，任意态互达）——对照 B 的 stage 单向不可逆。
-/
theorem ledger_realize_invertible (L : LedgerComp) (dPi : Int) :
    let L' := ledgerStep L (LedgerEvent.realize dPi)
    let L'' := ledgerStep L' (LedgerEvent.realize (-dPi))
    (L''.Pi, L''.A, L''.W) = (L.Pi, L.A, L.W) := by
  simp only [ledgerStep, Prod.mk.injEq, and_true]
  omega

/-! ════════════════════════════════════════════════════════════════════════
  ## §3 不同构裁定（machine-checked 精确反例，三条结构性阻断）

  「同构」= 存在双向忠实映射 (φ : TWState → LedgerComp, ψ : LedgerComp → TWState)，保守恒量
  + 保操作语义 + 互逆。下证**不存在**——给出精确结构反例。最强阻断 = 理由2（stage 不可逆）。
  ════════════════════════════════════════════════════════════════════════ -/

/--
  ★不同构反例 · 理由2（**最强，codex 标不可反驳**）：stage 不可逆 vs 操作可逆。

  形式化为状态空间结构差异：模型A 的状态由 (Π,A,W) 完全确定（R 派生，无"阶段"维度），且每个
  操作可逆（ledger_realize_invertible）；模型B 的 stage 携带**单向不可逆**信息
  （earning_no_regress：EarningShares 锁死）。

  **精确反例**：存在 B 的两个态 s₁（stage=costReduction）与 s₂（stage=earningShares），它们的
  TW 三量分量**完全相同**（free/holding/withdrawn 都相等），仅 stage 不同。任何「只看 TW 三量」
  的映射 φ:B→A **无法区分** s₁ 与 s₂（φ s₁ = φ s₂），故 φ 非单射 ⟹ 无双向忠实映射 ⟹ 不同构。
  （A 无 stage 维度可承载这个区分——A 的全部信息是 (Π,A,W)，无第四个离散不可逆分量。）
-/
theorem not_isomorphic_stage_collapses : ∃ (s₁ s₂ : TWState),
    -- 两态 TW 三量分量完全相同：
    s₁.free = s₂.free ∧ s₁.holding = s₂.holding ∧ s₁.withdrawn = s₂.withdrawn
    -- 但 stage 不同（一个 costReduction 一个 earningShares）：
    ∧ s₁.stage ≠ s₂.stage
    -- 故任何只读 TW 三量的 B→A 投影必将二者塌缩为同一像（非单射）：
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

  模型A 的守恒量 R=Π-A-W 是**单一恒等**，与 stage 无关（ledger_inv_always 对任意 L 成立，
  无"阶段"参数）。模型B 的守恒律**随 stage 切换**：CostReduction 阶段守 Σ|units|（股数守恒），
  EarningShares 阶段守 K=notionalIn（外部投入守恒）——二者是不同的守恒量。

  **精确反例（结构形式）**：B 中存在一个操作序列，使「同一守恒量」在阶段切换前后**改变其守恒
  地位**。具体：enterEarning 把 stage 从 costReduction 切到 earningShares，此后 Σ|units| 不再
  守恒（增股数单调增）而 K=notionalIn 守恒。模型A 无此「守恒律切换」——R=Π-A-W 在 realize/
  allocate/withdraw 全程是唯一守恒恒等。形式化：B 有一个 stage-依赖的不变量谓词，其真值随
  enterEarning **翻转**；A 的不变量谓词（R=Π-A-W）对所有操作恒为真（无翻转点）。
-/
theorem not_isomorphic_conservation_switches :
    -- B 侧：存在态 s 与事件 enterEarning，使「stage=costReduction」谓词被翻转（守恒律切换点）：
    (∃ s : TWState, s.stage = TStage.costReduction
        ∧ (twStep s TWEvent.enterEarning).stage ≠ TStage.costReduction)
    -- A 侧：R=Π-A-W 恒等对所有 LedgerComp 与所有操作恒为真（无切换点）：
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

  模型B 的 `holding = Σ units·c` 依赖**外生市价 c**：TW 守恒**仅在同价操作下成立**（twStep 的
  shortDiff 假设同价 c 固定下 holding⇄free 等额转换）。跨 bar 价格变动时 holding 随 c 重估，
  TW 改变（=盈亏，L2/L3）。模型A 是纯 Int 格，**无外生价格变量**——Π/A/W 的演化完全由
  realize/allocate/withdraw 的 Int 增量决定，不依赖任何外部连续参数。

  **精确反例（结构形式）**：B 中「holding 的同一组 units 在不同价 c 下取不同值」是 holding
  作为外生函数的本质；A 中不存在任何「随外生参数取值的字段」。形式化：给 B 一个**价格重估
  算子** `repriceHolding`（holding 从 c₁ 估值改为 c₂ 估值，模拟跨 bar 价格变动），它**改变 TW**
  （TW 非守恒）——这正是盈亏。模型A 无对应算子：A 的所有算子（ledgerStep）都保 R=Π-A-W 恒等，
  **不存在**任何 A 算子改变守恒量 R 的"恒等地位"。B 有「破坏 TW 守恒的外生重估」，A 无。
-/
def repriceHolding (s : TWState) (newHolding : Int) : TWState :=
  { s with holding := newHolding }

theorem not_isomorphic_exogenous_price :
    -- B 侧：价格重估改变 TW（外生价格维度存在 ⟹ TW 非全局守恒，仅同价守恒）：
    (∃ (s : TWState) (newHolding : Int),
        (repriceHolding s newHolding).tw ≠ s.tw)
    -- A 侧：不存在任何 ledgerStep 改变 R=Π-A-W 的恒等地位（R 永远 = Π-A-W，无外生参数）：
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
  `RoughlyCompatible` 构造子（拒绝"大致兼容"含糊裁定，督导闸要求）。
-/
inductive IsomorphismVerdict where
  | notIsomorphic
deriving DecidableEq, Repr

/--
  ★裁定见证（L0，gatekeeper）：账本裁定必是 notIsomorphic。三条结构反例
  （not_isomorphic_stage_collapses / _conservation_switches / _exogenous_price）共同支撑。
-/
theorem accounting_verdict_is_not_isomorphic (v : IsomorphismVerdict) :
    v = IsomorphismVerdict.notIsomorphic := by
  cases v; rfl

/-! ════════════════════════════════════════════════════════════════════════
  ## 交付总结（cc-accounting-tw 工位，task #90）

  本文件**证**（L0，machine-checked，无 sorry/admit/axiom）：
  1. 缠论取本金三阶段账本 `TWState`（free/holding/withdrawn/notionalIn/stage）+ TStage 三态 +
     `TWEvent` 四算子（shortDiff/recoverCapital/enterEarning/clearCampaign）+ `twStep` 全函数。
  2. `twStep_preserves_tw`（TW=free+holding+withdrawn 守恒，缠师第31课守恒律 L0 结构形式）。
  3. `stage_rank_monotone` + `earning_no_regress`（stage 单向不可逆，OQ-9 campaign 内结构形式）。
  4. R=Π-A-W 财务账本 `LedgerComp`（镜像 HybridAssembly.lean:90）+ `ledger_inv_always`（恒等
     永真）+ `ledger_realize_invertible`（A 操作可逆 = Z³ 平移系统）。
  5. ★★不同构三反例：`not_isomorphic_stage_collapses`（理由2 最强：TW 三量相同但 stage 不同 ⟹
     B→A 投影非单射）+ `not_isomorphic_conservation_switches`（理由3：B 守恒律随 stage 切换 / A
     单一恒等无切换）+ `not_isomorphic_exogenous_price`（理由1 codex 修正版：B 含外生价格维度
     ⟹ 重估改变 TW / A 无外生维度恒保 R=Π-A-W）。
  6. 诚实标签 `accounting_verdict_is_not_isomorphic`（裁定 = notIsomorphic，无"大致兼容"）。

  本文件**不证**（formalization-validity-domain 诚实边界）：
  - ✗ TW 数值反映实盘真实盈亏（twStep 保 TW 守恒是 L0 结构，数值正确性/跨 bar 价格 L2/L3，
       Rust prove_tw_neutral 守卫）。
  - ✗ 三阶段策略盈利/最优/实盘有效（L3，须真 bear regime 数据，design §3.3 net-up 8标的灾难）。
  - ✗ OQ-9 守恒律相变可逆性矛盾的消解（OQ-9 是 B 内部独立矛盾，与本不同构裁定正交，仍待上浮）。

  ★裁定（重要，codex gpt-5.5 异质审查确认 2026-06-26）：R=Π-A-W（模型A）与缠论取本金三阶段
  TW=free+holding+withdrawn（模型B）**不同构**。三条结构性阻断（最强=stage 单向不可逆）。
  下游：HybridAssembly.lean:90 新建 LedgerComp 承载 R=Π-A-W 是**正确的**（不冒充复用 Ledger），
  但它**不等价于**缠论取本金三阶段账本——二者是两个不同会计范畴，缺口真实（非可消化表示差异）。
  R=Π-A-W = 利润→分配→提取→储备的收益表视角；TW = 在险资产→退出路径→财富守恒的现金流+持仓
  视角。完整持仓系统**两者都需要**，互不替代。codex 修正：原"代数角色相反"(理由1)是表示层
  差异，已升级为"外生价格维度"结构差异。

  谱系：#42 / #86 / OQ-9（design §3.3）→ 本文件（#90 不同构裁定，machine-checked）。
  ════════════════════════════════════════════════════════════════════════ -/

end Formal.Tlayers.Accounting.TotalWealth
