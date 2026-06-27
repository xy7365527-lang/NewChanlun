# Canonical 全定义策略 S_Θ × repo Lean 实装 —— 逐条覆盖矩阵

---

## 【5层真封后更新 — 2026-06-27 最终验收】

> 下方正文是**补全前**矩阵（5节完整/13节部分/4节缺口，G1-G8）。本节是 codex 5 层架构真封后 G1-G8 的更新（git 真相核 Lean root 定理签名）。

| 缺口 | 旧状态 | 真封 root:定理 | 新状态 |
|---|---|---|---|
| **G1** §9 根声部 RootSel/GlobalRiskClose/根方向递归 | ❌ 全缺（高）| `RootSelDisambig.lean` `rootSel`/`rootSel_mirror_antisymmetric`/`rootSel_double_trigger_flat`/`globalRiskClose` + `SubVoiceOpenClose.lean:120/131` `closePred`/`openPred`（关闭优先开启）| ✅ 已实装 |
| **G2** §11 杠杆 n_v/G_t/N_t/L^G/L^N | ❌ 全缺（高）| `LeverageCapital.lean` `signedNotional`/`grossNotional`/`netNotional`/`netLeGross`/`gross_cap_implies_net_cap` | ✅ 已实装 |
| **G3** §6 区间套 Sel_Θ + N^δ 嵌套链 | ❌ 缺（中）| `IntervalNestCertificate.lean` `selectΘ`/`selectedByKey_unique`/`NestLevel`（J⊆J 嵌套 + 三键字典序选择器）| ✅ 已实装 |
| **G4** §10 负成本 c^adj<0⟺W+R>I_0 + K^adj | ❌ 缺（中）| `LeverageCapital.lean` 资本基准 + `CompleteStateEvent.lean` 完整账本分量（c^adj/K^adj 公式接入完整状态）| ◐→✅（结构层）|
| **G5** §8 声部树有限有根 + 同单位对冲约束 | ◐ 弱（中）| `SubVoiceOpenClose.lean` 子声部开平 + `ConstraintSystem.lean` K17 含 `a'_v=1⟹q'_v=q'_{p(v)}`（同单位）/`Σa'_w≤1`（每父一活子）逐条 | ✅ 已实装 |
| **G6** §7 镜像等变全链（非假设式）| ◐（中）| `RMoveCompose.lean`/`RootSelDisambig.lean:185` `rootSel_mirror_antisymmetric` + `MainTheorem.lean` 镜像等变综合支（从 Θ 镜像构造）| ◐→✅（综合支）|
| **G7** §1/§17 hybrid_step 六段义务版 | ◐（低-中）| `MainTheorem.lean:main_theorem` 闭环∃! 综合（引六段 ExistsUnique 义务）+ `DecisionSufficiency.lean:decision_pipeline_exists_unique` | ✅ 已实装 |
| **G8** §14 决策充分性 + 动态同余 | ❌ 全缺（**最严重**）| `DecisionSufficiency.lean`（四支全综合）+ `DynamicCongruence.lean:dynamic_congruence_commutes`（算子级交换图 + 非空洞见证 + 对接 Foundation `dynamic_closure_step`）| ✅ 已实装 |

**真封后**：22 节中 **§6/§9/§11/§14 四个显著 ❌ 缺口节全部填补**，G1-G8 八个关键缺口全部 ✅。
剩余诚实 still-MISSING（§18 不证盈利/回本/负成本/增单 + 最小性⟸行为商 + Θ 参数 L2 校准）= goal 自陈不证的条件性不变量，**非结构缺口**。

---

**审计类型**：反膨胀完整性核查（逐条核、标缺口、不声明全覆盖）
**canonical 源（只读）**：
- `~/Downloads/newchanlun-formal/strict_hybrid_state_machine_strategy.md`（S_Θ 混合状态机，619 行 22 节）
- `~/Downloads/newchanlun-formal/NewChanlunHybridStateMachine.lean`（canonical 配套 Lean，已编译无 sorry）

**repo 实装根（只读）**：`/Users/silencehan/Projects/NewChanlun/formal/Origin/`

**审计日期**：2026-06-27
**审计员**：NewChanlun 蜂群对照审计工位

---

## 0. 关键方法论声明（反膨胀前置）

1. **定义域 vs 有效域**（formalization-validity-domain）：
   - S_Θ 的 **22 节定义域是完整的**（混合状态机九元组 + 账本恒等 + 10 级优先级 + hybrid_step + 行为等价商）。
   - repo 实装的 **有效域严格小于定义域**——本矩阵给出覆盖率 + 缺口清单。

2. **L 级标注**（231 号）：所有 repo 实装均为 **L0**（machine-checked 结构层，无 sorry/admit/axiom）。
   L0 = 「定义/代数/转移函数图正确」，**不是**「策略盈利/实盘有效」（那是 L2/L3，repo 诚实标 EmpiricalDomain）。

3. **任务清单覆盖范围警示**：原任务清单仅列 11 个文件，但 `formal/Origin/` 有 **45 个 Lean 文件**。
   S_Θ 的若干组件（§7 镜像等变、§4 自相似递归、§13/§15 完全分类/行为商）实装在**任务清单外的文件**中。
   本矩阵已逐条核查全部 45 文件的相关实装，避免「把已实装误判为缺失」（反向膨胀错误）。

---

## 1. 覆盖矩阵（逐条）

图例：✅ 完整 machine-checked | ◐ 部分（结构有、canonical 形式不全） | ❌ 缺口（repo 无对应） | Θ-参 = Θ-参数化接口（诚实不 discharge）

### §1 单一总式 / π_Θ / 状态转移 / hybrid_step

| canonical 条目（节/行） | repo 实装（文件:定理/定义名） | 状态 | L级 |
|---|---|---|---|
| §1 `S_Θ` 九元组（X/E/Rec/C/Intent/K/J/Schedule/T） | `FullDefinitionStrategy.lean:FullDefinitionSystem`（structure，Event/Intent/Control/Order + recStruct/classify/intent/risk/schedule/transition 六段） | ✅ | L0 |
| §1 `π_Θ(x_t) = Schedule(LexArgmin J(...))` | `FullDefinitionStrategy.lean:policyTheta`（schedule∘risk∘intent∘classify∘recStruct） | ✅ | L0 |
| §1 状态转移 `x_{t+1}=T(x_t,π,e_{t+1})` | `FullDefinitionStrategy.lean:hybridStep` + `transition_writes_full_state` | ✅ | L0 |
| §1/§17 `hybrid_step_complete_unique`（一步唯一） | `FullDefinitionStrategy.lean:hybrid_step_complete_unique`；通用版 `NewChanlunHybridStateMachine.lean:hybrid_step_complete_unique`（六段 ∃! 义务版，repo Origin **未移植通用六段义务版**——repo 是函数图平凡 ∃!，见缺口 G7） | ◐ | L0 |
| §1 策略经分类因子化 | `FullDefinitionStrategy.lean:policy_factors_through_classification` | ✅ | L0 |

### §2 参数固定 Θ（全定义/单值/可判定/因果）

| canonical 条目 | repo 实装 | 状态 | L级 |
|---|---|---|---|
| §2 Θ 12 元组（data/parse/scale/signal/mirror/voice/phase/ledger/leverage/risk/execution/prec） | `StrategyFamily.lean:Theta`（structure：prefixEq/recog/target/riskProj/proj/exec + recog_causal）——**抽象 Θ，非 12 字段逐一**；`ThetaInstantiation.lean:chanlunTheta`（真缠论实例） | ◐ | L0 |
| §2「全定义/单值/可判定」 | `StrategyFamily.lean:given_theta_total_unique`（给定 Θ ⟹ π_Θ 全定义 + 唯一） | ✅ | L0 |
| §2「因果（无前视）」 | `StrategyFamily.lean:piTheta_causal` + `piTheta_isCausal`（对接 Origin `Causal`，**非平凡前提 recog_causal**） | ✅ | L0 |
| §2「无 Θ 则只有策略族」 | `StrategyFamily.lean:classification_does_not_choose_unique_policy` + `classification_does_not_produce_strategy` + `StrategyFamily` 结构 | ✅ | L0 |

### §3 完整状态空间 / 账本恒等

| canonical 条目 | repo 实装 | 状态 | L级 |
|---|---|---|---|
| §3 完整状态 `x_t`（h/D/σ/声部三元组/Φ/I_0/Π/A/W/R/E/Cash/O/M/ν） | `FullDefinitionStrategy.lean:StrictState`（parsed/trend/actionClass/riskMode/phase/ledger）——**精简版，非 canonical 全分量**（无 σ_r/声部/E/Cash/O/M/ν 字段） | ◐ | L0 |
| §3 **账本恒等 `R=Π-A-W`** | `FullDefinitionStrategy.lean:LedgerState`（inv 字段强制 R=Pi-A-W）+ `ledger_invariant_preservation`（每步保持） | ✅ | L0 |
| §3 转移保账本恒等 | `LedgerBridge.lean:origin_ledger_inv_preserved` / `origin_ledgerStep_preserves_inv` | ✅ | L0 |

### §4 自相似递归核

| canonical 条目 | repo 实装 | 状态 | L级 |
|---|---|---|---|
| §4 级别集 L + 最低级 `D_0=F_0(h)` + 高级递归 `D_{ℓ+1}=F_ℓ(D_ℓ)` | `RecursiveLevelSystem.lean:chanRecursiveLevelSystem` + `chanLevelAt`（窗口化递归塔）；接口 `TrendCompleteClassification.lean:RecursiveLevelSystem`（lift/at） | ✅ | L0 |
| §4 唯一性 `∀h,∀ℓ,∃! D_{ℓ,t}` | `RecursiveLevelSystem.lean:chan_recursive_self_similar`（递归塔确定）——**唯一性来自函数性，非独立 ∃! 定理** | ◐ | L0 |
| §4 **规则自相似 `N_{ℓ+1}∘F_ℓ = F_*∘N_ℓ`** | `TrendCompleteClassification.lean:recursive_level_self_similar` 特化 `RecursiveLevelSystem.lean:chan_recursive_self_similar`（`at base (n+1) = lift n (at base n)`） | ✅ | L0 |
| §4 `D_{ℓ,t}=(D^confirmed, D^active)` 二分量 | `BspClassification` / `CenterStates`（confirmed/active 中枢区分散在判据层）——**未见显式 (confirmed,active) 二元组结构** | ◐ | L0 |

### §5 每级走势分类 / 信号位

| canonical 条目 | repo 实装 | 状态 | L级 |
|---|---|---|---|
| §5 走势类型 `τ∈{bottom,P,U,D}` | `TrendCompleteClassification.lean:TrendClass`（四态分类 + complete_unique） | ✅ | L0 |
| §5 中枢相对位置 `r∈{bottom,I,U0,U1,D0,D1}` | `TrendCompleteClassification.lean:TrendOrder` + `CenterStates`（IsWithin/IsAbove/IsBelow 位置三态）——**位置态有，但非 canonical 6 值 r 枚举一一对应** | ◐ | L0 |
| §5 信号位 `b=(B1,B2,B3,S1,S2,S3)∈{0,1}^6` | `BspClassification`（IsType1/IsType2/IsType3Buy/IsType3Sell）+ `SellPointRecog`（IsType1Sell/IsType2Sell/IsType2Buy 卖买全六类判据） | ✅ | L0 |
| §5「2B/3B 可重合，不强制互斥」 | `SecondSellClosedLoop.lean` 注引 committed `no_exclusive_trichotomy`（2类3类可重合 V 型反转 x_2b3b 见证）；`SellPointRecog.lean:type3_buy_sell_exclusive`（仅同类买卖 side 互斥） | ✅ | L0 |

### §6 区间套证书

| canonical 条目 | repo 实装 | 状态 | L级 |
|---|---|---|---|
| §6 方向 `δ∈{-1,+1}` + 级别链 `o_v=ℓ_0>…>ℓ_k=e_v` | `SubLevelDescent.lean:descend` + `descend_level_decreases`（级别严格递减）；`VoiceThreeLevel.lean`（b_v⪰o_v⪰e_v 三级别序） | ✅ | L0 |
| §6 **递归证书 `N^δ`**（`Confirm` 基 + `Candidate ∧ J_{ℓ+1}⊆J_ℓ ∧ N^δ_{ℓ+1}` 嵌套递归） | `SubLevelDescent.lean:SubLevelType1` + `secondType_via_subLevel_type1`（次级别破中枢 ⟹ 第二类）——**仅次级别一类构成，非 canonical 完整 `N^δ` 嵌套递归证书 + `J⊆J` 区间套包含链** | ◐ | L0 |
| §6 **多候选 `Sel_Θ` 唯一选定**（无候选时 χ=0 非未定义） | **无对应实装**——repo 无固定选择器 `Sel_Θ` 把多区间套候选唯一选定 | ❌ | — |
| §6 最终证书 `χ^δ_v ∈{0,1}` | 部分隐含于 `SubLevelType1`（Bool 判据），**非 canonical χ^δ 区间套证书形式** | ◐ | L0 |

### §7 镜像等变

| canonical 条目 | repo 实装 | 状态 | L级 |
|---|---|---|---|
| §7 镜像对合 `M^2=id` | `VoiceTree.lean:flip_flip`（Side flip 对合）；`FullDefinitionStrategy.lean:action_mirror_involutive`；`TrendCompleteClassification.lean:trend_mirror_involutive`/`trend_order_mirror_involutive` | ✅ | L0 |
| §7 结构作用（U↔D, B_i↔S_i, U0↔D0, U1↔D1, +1↔-1） | `TrendCompleteClassification.lean:TrendClass.mirror`/`TrendOrder.mirror`；`SellPointRecog.lean`（买卖判据 side long↔short 全六类对偶） | ✅ | L0 |
| §7 **`classified_policy_mirror_equivariant`** | canonical 配套 `NewChanlunHybridStateMachine.lean:classified_policy_mirror_equivariant`；repo `FullDefinitionStrategy.lean:hybrid_step_mirror_equivariant`（**假设式**：以等变前提为 hypothesis，非从 Θ 镜像构造证）+ `TrendCompleteClassification.lean:trend_mirror_equivariant`/`chooseTrendByOrder_mirror`（**分类层真证**） | ◐ | L0 |
| §7 镜像等变四式（Rec/C/π/T 的 MΘ 版） | 仅分类层（trend）+ 动作标签层（ActionClass.mirror）真证；**Rec_{MΘ}/T_{MΘ} 全链镜像等变未端到端证**（hybrid_step 版是假设式） | ◐ | L0 |

### §8 多重赋格声部树

| canonical 条目 | repo 实装 | 状态 | L级 |
|---|---|---|---|
| §8 声部树 `T=(V,p)` 有限有根有序 | `VoiceTree.lean:VoiceTree`（parent/side/closed/depth + alternating/depth_decreasing/cascade_close）——**诚实标：强制无环+深度良基，不强制「有限/有根」**（canonical 要求有限有根，repo 弱于 canonical） | ◐ | L0 |
| §8 子声部级别低于父 `ℓ_v<ℓ_{p(v)}` | `VoiceTree.lean:depth_decreasing` + `VoiceThreeLevel.lean`（三级别序）；`ancestor_depth_lt` | ✅ | L0 |
| §8 非根声部方向 `σ_v=-σ_{p(v)}=σ_r(-1)^{depth}` | `VoiceTree.lean:alternating` + `adjacent_opposite`（赋格交替）——**σ_r(-1)^depth 闭式未显式证**（仅相邻反向） | ◐ | L0 |
| §8 当前严格公理（ancestor close / same-unit hedge / parent preserved / one active child） | `VoiceTree.lean:cascade_close` + `ancestor_closed`（祖先关闭级联）；`VoiceThreeLevel.lean:G`（父子许可合取）——**「exact same-unit hedge / Δq_parent=0 / Σa_child≤1」未逐条形式化** | ◐ | L0 |

### §9 根声部状态机

| canonical 条目 | repo 实装 | 状态 | L级 |
|---|---|---|---|
| §9 根方向 `σ_r∈{-1,0,+1}` | `SourceAxioms.Side`（long/short，**缺 0 中性态**）；`VoiceTree.side`（Side 二态） | ◐ | L0 |
| §9 **根选择函数 `RootSel_Θ`**（基础规则 RootSel(1,0)=+1 等 + 镜像 RootSel(M_D)=-RootSel） | **无对应实装**——repo 无 `RootSel` 根选择函数 | ❌ | — |
| §9 **根方向递归 `σ̃_{r,t+1}`**（GlobalRiskClose / 反向平根仓 / 先平后建） | **无对应实装**——repo 无 `GlobalRiskClose`、无「先平根仓下周期才反向」根方向递归状态机 | ❌ | — |

### §10 三阶段资本账本

| canonical 条目 | repo 实装 | 状态 | L级 |
|---|---|---|---|
| §10 方向中性账本 `R=Π-A-W` | `FullDefinitionStrategy.lean:LedgerState`（见 §3） | ✅ | L0 |
| §10 调整资本基准 `K^adj=I_0+A-Π` + `c^adj=K^adj/Q` | `TotalWealth.lean`（取本金三阶段 TW，free/holding/withdrawn）——**TW 模型与 R=Π-A-W 不同构（#90）**；canonical `K^adj/c^adj/负成本 c^adj<0 ⟺ W+R>I_0` **公式未逐条形式化** | ◐ | L0 |
| §10 负成本条件 `c^adj<0 ⟺ W_t+R_t>I_0` | **无对应实装**——repo 无负成本不等式定理 | ❌ | — |
| §10 资本阶段五分类（Phase I/II/III_repair/protected/accretive） | `FullDefinitionStrategy.lean:CapitalPhase`（phaseI/phaseII/repair/protectedPhase/accretive）+ `capital_phase_complete_unique`（互斥完备） | ✅ | L0 |
| §10 `phase_complete_unique` | `FullDefinitionStrategy.lean:capital_phase_complete_unique`；通用版 `NewChanlunHybridStateMachine.lean:phase_complete_unique` | ✅ | L0 |
| §10（衍生）取本金三阶段状态机 + OQ-9 单向不可逆 | `TotalWealth.lean:TStage` + `twStep_preserves_tw` + `stage_rank_monotone` + `earning_no_regress` + OQ-9 全套（`oq9inv_trace`/`oq9_legacy_leg_unreachable_in_earning` 等） | ✅（**超 canonical：OQ-9 扩维消解**） | L0 |

### §11 杠杆、保证金和风险模式

| canonical 条目 | repo 实装 | 状态 | L级 |
|---|---|---|---|
| §11 有符号名义 `n_v=σ_v M_v P_v q_v` + 总/净名义 `G_t/N_t` | **无对应实装**——repo 无名义头寸/总净名义计算 | ❌ | — |
| §11 总/净杠杆 `L^G_t=G_t/E_t`, `L^N_t=N_t/E_t` | **无对应实装**——repo 无杠杆比计算（`StrategyFamily.lean` 仅注释提及 leverage 为 Θ-参，无定理） | ❌ | — |
| §11 风险模式五分类 `μ∈{Insolvent,Liquidation,Deleverage,CloseOnly,Normal}` | `FullDefinitionStrategy.lean:RiskMode`（insolvent/liquidation/deleverage/closeOnly/normal）+ `risk_mode_complete_unique`（M0-M4 互斥完备） | ✅ | L0 |
| §11 风险模式优先级 M0-M4 定义（含 MM/B1/B2 保证金阈值） | `FullDefinitionStrategy.lean:chooseRiskMode`（m0-m3 Bool 优先级）——**MM_t(q)+B1/B2 保证金阈值公式抽象为 Bool 输入，未 discharge**（Θ-参） | ◐ | L0 |
| §11 `risk_mode_complete_unique` | `FullDefinitionStrategy.lean:risk_mode_complete_unique`；通用版同名 | ✅ | L0 |

### §12 全局安全可行集

| canonical 条目 | repo 实装 | 状态 | L级 |
|---|---|---|---|
| §12 控制量 `u=(q',w',cancel,orders)` | `StrategyFamily.lean:Theta.exec`（D→Z→Pos→Order）；`ThetaInstantiation.lean:ChanlunOrder`——**抽象 Order，非 canonical 4 元控制** | ◐ | L0 |
| §12 可行集 `K_Θ(x_t)`（17 项约束：网格/许可/杠杆上界/IM/MM/StressLoss/提现/Phase II/ΔQ_core/μ≠Normal） | `RiskProj.lean:RiskGrid`（有限网格 grid + cost + cost_inj）+ `VoiceThreeLevel.lean:G`（父子许可合取）——**仅网格 + 父子许可；17 项约束中杠杆上界/IM/MM/StressLoss/Phase II 约束未逐条形式化**（Θ-参 + 缺口） | ◐ | L0 |
| §12 `K_Θ(x_t)≠∅`（安全控制总存在） | `RiskProj.lean:grid_ne_nil`（0∈𝒦 ⟹ grid 非空） | ✅ | L0 |
| §12 风险投影 `u*=LexArgmin J` 唯一（有限网格 + 字典序） | `RiskProj.lean:riskproj_exists_unique`（有限非空网格 ⟹ 字典序最小唯一）+ `gridProject_isLexArgmin` + `lexargmin_eq_project` | ✅ | L0 |

### §13 全局分类函数 C_Θ

| canonical 条目 | repo 实装 | 状态 | L级 |
|---|---|---|---|
| §13 离散模式 `m_Θ`（δ/μ/Φ/σ_r/S^Chan/χ/声部态/ExecMode） | 分散：`TrendCompleteClassification`（趋势）+ `RiskMode`/`CapitalPhase`（μ/Φ）+ `BspClassification`（χ 信号）——**未组装为单一 `m_Θ` 元组** | ◐ | L0 |
| §13 连续充分统计 `r_Θ`（P/E/G/N/I_0/Π/A/W/R/IM/MM/边界...） | `LedgerState`（Π/A/W/R）+ `Center`（边界）——**E/G/N/IM/MM 等大部分连续统计未形式化**（多与 §11 缺口耦合） | ◐ | L0 |
| §13 最终分类 `C_Θ=(m_Θ,r_Θ)` + 静态完全分类 `X_Θ=⊔C_s` + `∀x,∃!s` | `CompleteClassification.lean:CompleteClassifier` + `complete_classification_unique_class`（∀x ∃! class）+ `complete_classification_disjoint` + `ClassifierPartition`（有限划分） | ✅（**抽象完全分类器**） | L0 |

### §14 为什么这不是平凡分类（决策充分性 + 动态同余）

| canonical 条目 | repo 实装 | 状态 | L级 |
|---|---|---|---|
| §14 **决策充分性** `C_Θ(x)=C_Θ(y) ⟹ K_Θ/Intent/J/π_Θ 相等` | **无对应实装**——repo 无「同类 ⟹ 同可行集/同意图/同策略」定理 | ❌ | — |
| §14 **动态同余 `C_Θ(T(x,π,e))=T̄_Θ(C_Θ(x),ē)`** | **无对应实装**——repo 无 `DynamicallyClosed`/`Tbar`（canonical 配套有 `dynamic_closure_step`，repo Origin **未移植**） | ❌ | — |
| §14 `dynamic_closure_step` | 仅 canonical 配套 `NewChanlunHybridStateMachine.lean:dynamic_closure_step`；**repo Origin 零命中** | ❌ | — |

### §15 最小完全分类：行为等价商

| canonical 条目 | repo 实装 | 状态 | L级 |
|---|---|---|---|
| §15 输出轨迹 `Tr_Θ(x,e)` | `CompleteClassification.lean:CompleteClassifier.trace`（X→Ω→TraceOut）；`ConcreteBehaviorQuotient.lean:engineTrace`（具体引擎轨迹） | ✅ | L0 |
| §15 行为等价 `x≈_beh y ⟺ ∀e Tr(x,e)=Tr(y,e)` | `CompleteClassification.lean:BehEquiv` + `beh_refl`/`beh_symm`/`beh_trans`（等价关系） | ✅ | L0 |
| §15 最小完全分类 `X_Θ/≈_beh` + `C_Θ(x)=C_Θ(y) ⟺ x≈_beh y` | `CompleteClassification.lean:CompleteClassifier.complete`（双向）+ `complete_classification_behavior_quotient`；`BehaviorQuotient.lean`（商集 fibre = 行为类）；`ConcreteBehaviorQuotient.lean:thetaClass`（具体引擎行为商 ∃ 非退化见证 A=1 vs A=2 行为可分） | ✅ | L0 |
| §15 `BehEquiv`/`beh_refl`/`beh_symm`/`beh_trans`/`CompleteMinimalClassification`/`distinct_classes_behavior_separated` | `CompleteClassification.lean` 全部对应（`distinct_classes_behavior_separated` ✅）；通用版同名 canonical 配套；**`CompleteMinimalClassification` repo 命名为 `CompleteClassifier.complete` 字段** | ✅ | L0 |

### §16 动作意图优先级（10 级）

| canonical 条目 | repo 实装 | 状态 | L级 |
|---|---|---|---|
| §16 原始 10 级优先级（破产强平 > 去杠杆 > ... > 增核 > 保持） | `FullDefinitionStrategy.lean:ActionClass`（10 构造子，顺序与 canonical 一致：insolventOrLiquidation/deleverage/executionRepair/phaseTwoReturnCapital/rootOrAncestorInvalid/closeReverseChild/openRoot/openReverseChild/phaseThreeAccreteCore/hold） | ✅ | L0 |
| §16 互斥动作类 `A1=P1, Ai=Pi∧¬P1..¬P_{i-1}, A10=¬all` | `FullDefinitionStrategy.lean:ActionSpec` + `chooseAction`（优先级 if-then-else 级联） | ✅ | L0 |
| §16 `Σ1[A_i]=1`（恰一类成立） | `FullDefinitionStrategy.lean:action_priority_complete_unique`（∃!）；通用版 `NewChanlunHybridStateMachine.lean:action_partition_complete_unique` | ✅ | L0 |
| §16 `action_partition_complete_unique` + `action_classes_disjoint` | repo `action_priority_complete_unique`（=partition_complete_unique）；`action_classes_disjoint` **仅 canonical 配套有显式定理，repo Origin 命名为 `action_spec_iff_chosen` 蕴含互斥**（功能等价，命名不同） | ✅ | L0 |

### §17 最终总定理

| canonical 条目 | repo 实装 | 状态 | L级 |
|---|---|---|---|
| §17 13 前提 ⟹ `∀x,∀e,∃!(D,c,ũ,u*,O,x_{t+1})` 全链唯一 | `FullDefinitionStrategy.lean:hybrid_step_complete_unique`（**函数图平凡 ∃!**——因 policyTheta/hybridStep 是全函数）；通用六段义务版 `NewChanlunHybridStateMachine.lean:hybrid_step_complete_unique`（**13 前提显式**：六段 ExistsUnique 义务 ⟹ 整体 ∃!）**未移植 repo Origin** | ◐ | L0 |
| §17 全链 `D=Rec, c=C, ũ=Intent, u*=LexArgmin, O=Schedule, x'=T` | `FullDefinitionStrategy.lean:policyTheta`（链组合）+ `ThetaInstantiation.lean:chanlunTheta`（真缠论实例化全链） | ✅ | L0 |

### §18 能证明与不能证明（诚实边界）

| canonical 条目 | repo 实装 | 状态 | L级 |
|---|---|---|---|
| §18 能证：每合法态有且只有一个类/意图/控制/订单/下一态 | 见 §1/§13/§16/§17（结构唯一性全覆盖范围内 ✅） | ✅ | L0 |
| §18 **不能证：盈利/回本/负成本/增单**（条件性不变量） | repo **全文件诚实标 EmpiricalDomain（L3）**——所有 gatekeeper 标签禁标 `ProfitableStrategy`/`TrueCompleteClassification`（`StrategyFamily`/`ThetaInstantiation`/`SellClosedLoop`/`RiskProj`/`VoiceTree`/`VoiceThreeLevel` 全有 gatekeeper 见证定理） | ✅（**诚实边界对齐**） | L0/L3 标注 |

---

## 2. 覆盖率汇总

| 节 | 主题 | 状态 | 备注 |
|---|---|---|---|
| §1 | 单一总式/π_Θ/hybrid_step | ✅◐ | 六段义务版未移植（G7） |
| §2 | 参数固定 Θ | ✅◐ | 抽象 Θ，非 12 字段 |
| §3 | 状态空间/账本恒等 | ✅◐ | R=Π-A-W ✅；全状态分量精简 |
| §4 | 自相似递归核 | ✅◐ | 自相似律 ✅ |
| §5 | 每级走势分类/信号位 | ✅◐ | 六类买卖判据 ✅ |
| §6 | 区间套证书 | ◐❌ | **Sel_Θ 唯一选择器缺（G3）** |
| §7 | 镜像等变 | ◐ | 分类层真证，全链假设式（G6） |
| §8 | 赋格声部树 | ◐ | 无环良基 ✅；有限有根/单位对冲约束弱（G5） |
| §9 | **根声部状态机** | ❌ | **RootSel/GlobalRiskClose 全缺（G1）** |
| §10 | 三阶段资本账本 | ✅◐❌ | 五阶段分类 ✅ + OQ-9 超 canonical；负成本不等式缺（G4） |
| §11 | 杠杆/保证金/风险模式 | ✅◐❌ | 风险模式五分类 ✅；**杠杆 G/N/L 全缺（G2）** |
| §12 | 全局安全可行集 | ✅◐ | 风险投影唯一 ✅；17 项约束部分 |
| §13 | 全局分类 C_Θ | ✅◐ | 抽象完全分类器 ✅；m_Θ/r_Θ 未组装 |
| §14 | **决策充分性/动态同余** | ❌ | **全缺（G8）** |
| §15 | 行为等价商 | ✅ | **完整（含具体引擎非退化见证）** |
| §16 | 10 级动作优先级 | ✅ | **完整** |
| §17 | 最终总定理 | ◐ | 函数图 ∃! ✅；13 前提义务版未移植（G7） |
| §18 | 诚实边界 | ✅ | gatekeeper 全覆盖 |

**定性覆盖率**：22 节中 —— 完整 ✅ **5 节**（§15/§16 + §3/§10/§11 的核心账本/阶段/风险模式子项），
主体覆盖 ◐ **13 节**，**有显著 ❌ 缺口 4 节**（§6 部分 / §9 全 / §11 部分 / §14 全）。

---

## 3. 关键缺口清单（❌，反膨胀核心交付）

| 编号 | 缺口 | canonical 出处 | 严重度 | 性质 |
|---|---|---|---|---|
| **G1** | **根声部状态机 `RootSel_Θ` / `GlobalRiskClose` / 根方向递归 `σ̃_{r,t+1}`（先平根仓下周期才反向）** | §9 全节 | 高 | **结构层 L0 可证但未实装**——根方向状态机是 S_Θ 的全局风控核心，repo 零命中 |
| **G2** | **杠杆/保证金量化：名义头寸 `n_v`/`G_t`/`N_t`/总净杠杆 `L^G`/`L^N`** | §11 前半 + §12/§13 耦合 | 高 | 部分 Θ-参（保证金阈值），但 G_t/N_t/杠杆比的**计算结构**本身缺 L0 实装 |
| **G3** | **区间套唯一选择器 `Sel_Θ`（多候选唯一选定）+ 完整 `N^δ` 嵌套递归证书（J⊆J 包含链）** | §6 | 中 | repo 有级别递减但无 canonical 区间套证书递归形式 + 选择器 |
| **G4** | **负成本条件 `c^adj<0 ⟺ W_t+R_t>I_0` + `K^adj`/`c^adj` 公式** | §10 中段 | 中 | TW 三阶段已实装但 canonical 调整资本基准不等式未逐条证 |
| **G5** | **声部树「有限/有根」+「exact same-unit hedge / Δq_parent=0 / Σa_child≤1」严格公理** | §8 | 中 | repo `VoiceTree` **诚实标弱于 canonical**（只无环良基，不强制有限有根），单位对冲约束未形式化 |
| **G6** | **镜像等变全链 `Rec_{MΘ}/C_{MΘ}/π_{MΘ}/T_{MΘ}` 端到端（非假设式）** | §7 | 中 | 分类层 `trend_mirror_equivariant` 真证，但 `hybrid_step_mirror_equivariant` 是假设式（以等变为前提），Θ→镜像构造未端到端 |
| **G7** | **`hybrid_step_complete_unique` 六段 ExistsUnique 义务版（13 前提显式）未移植 repo Origin** | §1/§17 | 低-中 | canonical 配套 Lean 有义务版（每段 ∃! ⟹ 整体 ∃!），repo Origin 仅函数图平凡 ∃!——**信息量低于 canonical** |
| **G8** | **决策充分性 `C_Θ(x)=C_Θ(y)⟹K/Intent/J/π 相等` + 动态同余 `dynamic_closure_step`/`Tbar`** | §14 | **高** | **§14「为什么不是平凡分类」的核心**——repo Origin 零命中，canonical 配套有 `dynamic_closure_step` 但**未移植**。这是 S_Θ 最有信息量的非平凡性证明，缺失最严重 |

---

## 4. 反向核查（避免反向膨胀：已实装勿误判缺失）

以下 S_Θ 组件实装在**原任务清单 11 文件之外**，本审计已核查并计入 ✅/◐（非缺口）：

- §15 行为等价商 → `CompleteClassification.lean` / `BehaviorQuotient.lean` / `ConcreteBehaviorQuotient.lean` / `FiniteTraceQuotient.lean`（任务清单未列，但**完整实装**）
- §7 镜像等变（分类层）→ `TrendCompleteClassification.lean`（任务清单未列）
- §4 自相似递归 → `RecursiveLevelSystem.lean` / `WellFoundedRank.lean`（任务清单未列）
- §6 级别递减 → `SubLevelDescent.lean` / `BspConstruction.lean`（任务清单未列）
- §13 完全分类器 → `CompleteClassification.lean`（任务清单未列）

**结论**：repo 在**分类/行为商/递归/镜像分类层**的覆盖比任务清单暗示的更完整；缺口集中在**策略执行侧的全局风控**（§9 根声部、§11 杠杆、§14 动态同余/决策充分性）。

---

## 5. 结果包六要素

1. **结论**：S_Θ（22 节定义域完整）对照 repo Origin 实装，**5 节完整 ✅ / 13 节主体覆盖 ◐ / 4 节显著缺口 ❌**（§6 部分、§9 全、§11 部分、§14 全）。8 个关键缺口 G1-G8，最严重为 **G8（§14 决策充分性 + 动态同余，repo 零命中且 canonical 配套有但未移植）** 和 **G1（§9 根声部状态机全缺）**。

2. **定义依据**：逐条对照 `strict_hybrid_state_machine_strategy.md` §1-§18 的每个核心结构（九元组/账本恒等/10 级优先级/hybrid_step/行为等价商/决策充分性/动态同余）与 `formal/Origin/` 45 文件的定理/定义名。状态判定依据：repo 定理签名是否结构覆盖 canonical 条目（✅）、是否结构有但 canonical 形式不全/弱于 canonical（◐）、是否 grep 零命中（❌）。

3. **边界条件**（结论翻转条件）：
   - 若 repo 后续移植 `dynamic_closure_step`/`Tbar` + 决策充分性定理 → G8 翻转为 ✅，§14 从 ❌ 变 ✅。
   - 若 repo 实装 `RootSel`/`GlobalRiskClose` 根方向递归 → G1 翻转，§9 从 ❌ 变 ✅。
   - 若我对「函数图平凡 ∃!」的判定错误（即 repo `hybrid_step_complete_unique` 实际已含六段义务）→ G7 翻转；但已核 repo 版是 `⟨hybridStep S x e, rfl, fun _ heq => heq.symm⟩`（纯函数图），判定成立。
   - 若 ◐ 项中某些「弱于 canonical」实为 Θ-参数化的合法诚实留白（非缺口）→ 该项可上调，但不改 ❌ 缺口清单。

4. **下游推论**：
   - **G8 缺失意味 §14「为什么这不是平凡分类」在 repo 未兑现**——repo 的完全分类器（§13/§15）证了「分类 = 行为商」，但**未证「分类对决策充分 + 动态封闭」**，即 repo 尚未排除「分类是平凡 over-refinement」的可能。这是 S_Θ 最有信息量的定理，下游若声称「repo 实现了完整 S_Θ」是**声明膨胀**。
   - **G1/G2 缺失意味全局风控（根方向 + 杠杆）未形式化**——回测/实盘风控层（M4 里程碑）无 Lean 守卫支撑。
   - repo 在**分类/行为商层超额覆盖**（含具体引擎非退化见证 + OQ-9 扩维消解，§10 超 canonical），但**策略执行风控层有结构缺口**。

5. **谱系引用**：本审计涉及曾发生概念分离的领域——
   - **#90 不同构**（R=Π-A-W vs 取本金三阶段 TW）：§10 的 TW 实装诚实保持两账本不同构（`TotalWealth.lean:not_isomorphic_stage_collapses`），非缺口而是正确层分离。
   - **OQ-9 扩维消解**（守恒律相变可逆性矛盾）：`TotalWealth.lean` §1.5 rawStep/legalStep 双层，超出 canonical §10——记忆 [[oq9-extend-dimension-resolution-pattern]]。
   - **231 号有效域 ≠ 定义域**：本矩阵核心 = S_Θ 定义域（22 节完整）vs repo 有效域（实装覆盖），全部 repo 实装标 L0，诚实边界（§18）对齐。
   - **still-MISSING 链**（#117(c) 卖点对偶 → #120 → #121 卖侧闭环 → #129 第二类）：买卖闭环已对偶完整，但 bspOf 全自动识别 / TW 端提现对接 / 第二类次级别递归构成（still-MISSING-D）仍诚实留白——这些是**判据自动化缺口**，与本矩阵的 S_Θ 结构缺口（G1-G8）正交。

6. **影响声明**：本产出为**只读审计**，新建 `docs/canonical-coverage-strategy.md`（覆盖矩阵），**未改动任何 Lean 文件 / 定义 / 模块**。影响：为编排者「完整的全定义策略有没有对照结果包做」的质询提供逐条答案——**答：未全覆盖，4 节显著缺口（G1-G8），最严重 G8（§14 动态同余/决策充分性零命中）+ G1（§9 根声部全缺）**。下游若需补全 S_Θ，优先级序：G8 > G1 > G2 > G3/G4/G5/G6 > G7。
