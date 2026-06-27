# Canonical「完全分类数学」对照覆盖矩阵

> 反膨胀完整性核查（编排者质询「完全分类的数学有没有对照结果包做」）
> 逐条核 · 标缺口 · 不声明全覆盖
> 审计日期：2026-06-27 · 只读审计，未改代码

## 0. 范围与对照源

**canonical 源（权威文本，定义域）**：
- `~/Downloads/newchanlun-claude-code-result-package/FULL_USER_FORMULA_SOURCE.md`（1619 行，二十二节完全分类数学）
- `~/Downloads/newchanlun-from-origin-result-package-20260626/repo_artifacts/current-strictness-audit.md`（canonical Required Next Gates）
- `~/Downloads/newchanlun-from-origin-result-package-20260626/repo_artifacts/origin-proof-map.md`（canonical proof path）

**repo 实装（有效域）**：`/Users/silencehan/Projects/NewChanlun/formal/`（Origin/ + Foundation/）

**「完全分类」的 canonical 定义**（FULL_USER_FORMULA_SOURCE.md §1/§15/§17/§22）：
> 把系统定义成「参数固定、因果、可判定、自相似、镜像等变的确定性混合状态机」，
> 并把**完全分类**定义为该状态机的**行为等价商集** `X_Θ / ≡^beh_Θ`。
> 忠实编码 = 分类器纤维**恰好等于**行为等价类（`C_Θ(x)=C_Θ(y) ⟺ x≡^beh_Θ y`）。

因此「完全分类数学」= ①顶层行为商定义（§1/§15/§16/§17/§22）+ ②各子分类的互斥穷尽证（§5走势 / §10买卖点 / §11-13资本/风险/杠杆 / §18动作 / §8声部赋格）+ ③结构等变（§4自相似 / §7镜像 / §6区间套 / §9同单位双开）。

---

## 1. 覆盖矩阵（逐条）

L 级标注遵循 `formalization-validity-domain.md`：**全部实装为 L0**（结构 machine-checked，不依赖真实市场数据）。状态 ∈ {已实装 machine-checked / 部分 / 缺 / 诚实开口 still-MISSING}。

### A. 顶层行为商（完全分类的核心定义，§1/§15/§16/§17/§22）

| # | canonical 条目（出处行号）| repo 实装（文件:定理）| 状态 | L级 |
|---|---|---|---|---|
| A1 | 行为等价关系 `x≡^beh_Θ y ⟺ ∀e, Tr_Θ(x,e)=Tr_Θ(y,e)`（§17 L1256-1265）| `Origin/CompleteClassification.lean:14` `BehEquiv` + `beh_refl/symm/trans`（L18-34）| 已实装 | L0 |
| A2 | 完全分类 = 行为商纤维（忠实编码 `C(x)=C(y) ⟺ x≡^beh y`）（§17 L1274-1282 / §22 L1588）| `CompleteClassification.lean:36` `CompleteClassifier.complete` schema 双向核 | 已实装（schema） | L0 |
| A3 | 互斥穷尽 `X_Θ = ⊔_s C^{-1}(s)`（§15 L1175-1190）| `CompleteClassification.lean:64` `complete_classification_unique_class` + `:72` `_disjoint` + `:99` `ClassifierPartition`（FinitePartition）| 已实装 | L0 |
| A4 | 具体行为商（Gate 2：具体 engine X/Ω/TraceOut，证 concrete iff）| `Origin/ConcreteBehaviorQuotient.lean:175` `concrete_behavior_quotient`（ChanlunAccount/chanlunTransition/engineTrace）| 已实装 | L0 |
| A5 | 决策充分性 `C(x)=C(y) ⟹ K=K, Intent=, J=, π=`（§16 L1198-1213）| `Origin/FullDefinitionStrategy.lean:238` `policy_factors_through_classification`（π 经 classify 分解）| **部分** | L0 |
| A6 | 动态同余 `C∘T = T̄∘(C,ē)`（§17 L1221-1233 / §22 L1552-1562）| `Origin/ThetaInstantiation.lean` `chanlun_transition_distinguishes_classes`（转移区分类别）；`Foundation/HybridStateMachine.lean` | **部分** | L0 |
| A7 | 有限语料行为商（runtime audit，弱于全未来）| `Origin/FiniteTraceQuotient.lean:29` `finite_corpus_fiber_iff` | 已实装 | L0 |
| A8 | 分类器族 `C_Θ` Θ-参数化 + fiber partition（§15 全局分类函数）| `Origin/ClassifierFamily.lean:85` `classifier_total_unique` + `:139` `theta_fiber_partition` + `:158` `thetaFinitePartition` | 已实装 | L0 |
| A9 | guardrail：禁 label 伪装完全分类（缠论标签 coarser 于行为）| `Origin/ClassificationGuardrail.lean:173` `label_only_not_complete_classifier` + `:242` `origin_both_directions_fail` | 已实装（否定性）| L0 |

**A5/A6 部分原因（诚实标缺口）**：
- A5：`policy_factors_through_classification` 证 π 经 classify 因子分解（结构事实），**未**证 canonical §16 的完整四元义务 `C(x)=C(y) ⟹ K(x)=K(y) ∧ Intent(x)=Intent(y) ∧ J_x=J_y ∧ π(x)=π(y)`。只兑现「π 通过 classify」一支，K/Intent/J 三支的「同类⟹同应对」未独立证。
- A6：repo 证「转移区分类别」（非退化），**未**证 canonical §17 的算子级动态同余 `C_Θ∘T_Θ = T̄_Θ∘(C_Θ,ē)`（存在类别空间转移 T̄ 使图表交换）。这是 still-MISSING：类别空间转移 T̄ 及其交换图未形式化。

### B. 走势完全分类（§5 L252-308）

| # | canonical 条目（出处行号）| repo 实装（文件:定理）| 状态 | L级 |
|---|---|---|---|---|
| B1 | 走势类型 `τ∈{⊥,P,U,D}` 四类无第五（§5 L267-275）| `Origin/TrendCompleteClassification.lean:13` `TrendClass` + `:107` `trend_no_fourth_class` | 已实装 | L0 |
| B2 | 走势互斥穷尽 ∃!（§5 / §15）| `TrendCompleteClassification.lean:88` `trend_complete_unique` + `:98` `trend_classes_disjoint` | 已实装 | L0 |
| B3 | 走势分类器 total-unique | `TrendCompleteClassification.lean:120` `TrendClassifier.classify` + `:123` `trend_classifier_total_unique` | 已实装 | L0 |
| B4 | 相对中枢位置 `r∈{⊥,I,U⁰,U¹,D⁰,D¹}` 六态（§5 L277-292）| **缺** — repo 无 `r` 六态枚举（仅 CenterStates 有位置三态 above/in/below，非 canonical 六态含三买三卖标记）| **缺** | — |
| B5 | 信号位向量 `b∈{0,1}⁶`（B1/B2/B3/S1/S2/S3 不强制互斥）（§5 L294-302）| **缺** — repo 无六位信号向量结构；买卖点用独立判据（见 D）而非 `{0,1}⁶` 位向量 | **缺** | — |

### C. 缠论元素解析管线（§3 状态 / §4 递归）

| # | canonical 条目（出处行号）| repo 实装（文件:定理）| 状态 | L级 |
|---|---|---|---|---|
| C1 | 元素接口 Bar/Fractal/Stroke/Segment/Center/Move/Bsp/ParseStruct（§4 L184-207）| `Origin/ChanlunElements.lean` `parse_total_unique` 等 8 个 total_unique（proof-map §3）| 已实装（接口）| L0 |
| C2 | 自相似递归 `N_{ℓ+1}∘F_ℓ = F_*∘N_ℓ`（§4 L216-234）| `TrendCompleteClassification.lean:137` `RecursiveLevelSystem` + `:146` `recursive_level_self_similar`；`Foundation/CompleteClassification.lean` LevelRecursion | 已实装 | L0 |
| C3 | 良基性递归终止 `ℓ_child<ℓ_parent`（§4 L238-248）| `Origin/WellFoundedRank.lean`；`Origin/SubLevelDescent.lean:131` `descend_level_decreases` | 已实装 | L0 |
| C4 | 最低/高级别递归 `D_{ℓ+1}=F_ℓ(D_ℓ)`，每级 confirmed+active（§4 L184-207）| `Origin/Pipeline.lean` / `SubLevelDescent.lean:104` `descend`；中枢生成 `chanCenters_empty` 仍空桩（见缺口）| **部分** | L0 |

### D. 买卖点完全分类（§10，canonical §5 信号位的判据层）

| # | canonical 条目 | repo 实装（文件:定理）| 状态 | L级 |
|---|---|---|---|---|
| D1 | 第一类买卖点判据（破中枢+背驰）| `Origin/BspClassification.lean:94` `IsType1` + `:296` `witness_type1` | 已实装 | L0 |
| D2 | 第二类买卖点判据（一类后回抽，由次级别一类构成）| `BspClassification.lean:103` `IsType2`；`Origin/BspConstruction.lean:163` `secondType_via_subLevel`（次级别递归判据）| 已实装 | L0 |
| D3 | 第三类买卖点判据（离开中枢回抽不破 ZG/ZD）| `BspClassification.lean:111` `IsType3Buy`/`:119` `IsType3Sell` + `:135` `type3Buy_retrace_above` | 已实装 | L0 |
| D4 | 完备性「只有一二三类」（§10.2 公理）| `BspClassification.lean:214` `bspType_exhaustive` + `:234` `bsp_kinds_exhaustive`（6种=Side×3类）| 已实装 | L0 |
| D5 | 买卖点**非**互斥 sum type（2/3 类可重合）| `BspClassification.lean:197` `no_exclusive_trichotomy`（x_2b3b 见证）| 已实装（否定性）| L0 |
| D6 | 第三类本征子域真双射 | `BspClassification.lean:256` `third_subdomain_exhaustive`/`:263` `_exclusive`/`:271` `thirdInvariant` | 已实装 | L0 |
| D7 | bspOf 全自动构造（List Move → List Bsp 遍历识别）| `BspConstruction.lean:127` `bspOf` + `:367` `bspOfMoves` + `:200` `bspOf_total_unique` | 已实装 | L0 |

### E. 资本/风险/动作完全分类（§11/§12/§13/§18）

| # | canonical 条目（出处行号）| repo 实装（文件:定理）| 状态 | L级 |
|---|---|---|---|---|
| E1 | 资本三阶段 `Φ∈{I,II,III_repair,III_protected,III_accretive}` ∑1[·]=1（§12 L929-971）| `Origin/FullDefinitionStrategy.lean:145` `CapitalPhase` + `:175` `capital_phase_complete_unique` | 已实装 | L0 |
| E2 | 风险模式 `μ∈{Insolvent,Liquidation,Deleverage,CloseOnly,Normal}` M0-M4 ∑1[Mᵢ]=1（§13 L1024-1063）| `FullDefinitionStrategy.lean:108` `RiskMode` + `:136` `risk_mode_complete_unique` | 已实装（**仅模式枚举**）| L0 |
| E3 | 十路动作优先级 `A_i=P_i∧⋀_{j<i}¬P_j` ∑1[Aᵢ]=1（§18 L1293-1336）| `FullDefinitionStrategy.lean:13` `ActionClass`(10) + `:98` `action_priority_complete_unique` | 已实装 | L0 |
| E4 | 记账恒等式 `R_t=Π_t-A_t-W_t` 守恒（§3 L168 / §11 L668）| `FullDefinitionStrategy.lean:184` `LedgerState.inv` + `:198` `ledger_invariant_preservation`；`Origin/LedgerBridge.lean`/`TotalWealth.lean` | 已实装 | L0 |
| E5 | **杠杆完全分类**：总名义 `G_t`、净名义 `N_t`、总杠杆 `L^G=G/E`、净杠杆 `L^N=N/E`（§13 L977-1006）| **缺** — `Origin/SourceAxioms.lean:77` 仅 `leverage : Type` 抽象占位，**无** G_t/N_t/L^G/L^N 算式实装。RiskMode 的 `deleverage` 是标签，非杠杆量分类 | **缺（still-MISSING）** | — |

**E2 部分边界**：RiskMode 五态枚举 + ∑1=1 已证，但 canonical §13 的 Mᵢ 判据**依赖**杠杆量 `E_t<MM_t(q)+B^{(i)}`（保证金缓冲阈值）——这些阈值算式（E5）缺失，故 RiskMode 实装是「五态互斥穷尽的**抽象骨架**」，未绑定 canonical 的具体保证金/杠杆触发条件。

### F. 结构等变与递归证书（§4/§6/§7/§9/§8）

| # | canonical 条目（出处行号）| repo 实装（文件:定理）| 状态 | L级 |
|---|---|---|---|---|
| F1 | 多空镜像 `M²=id` 等变 `C_{MΘ}∘M_X=M_S∘C_Θ`（§7 L372-431 / §22 L1564-1574）| `TrendCompleteClassification.lean:20` `TrendClass.mirror` + `:26` `trend_mirror_involutive` + `:129` `trend_mirror_equivariant`；`FullDefinitionStrategy.lean:44` `action_mirror_involutive` + `:259` `hybrid_step_mirror_equivariant` | 已实装 | L0 |
| F2 | 区间套递归证书 `χ^δ_{v,t}=N_{o↓e}(D)∈{0,1}`（§6 L312-368）| **部分** — `Origin/SubLevelDescent.lean:235` `SubLevelType1` + `:263` `secondType_via_subLevel_type1`（次级别破中枢递归判据），**但** canonical 的多层嵌套区间套证书 `N_{ℓ_j↓e}`（`J_{ℓ+1}⊆J_ℓ` 嵌套链 + Sel_Θ 唯一选择器）未完整形式化 | **部分** | L0 |
| F3 | 声部赋格树 `σ_v=-σ_{p(v)}`、祖先闭合 `a_v≤a_{p(v)}`、同单位 `a_v=1⟹q_v=q_{p(v)}`（§8/§9 L441-542）| `Origin/GlobalProductClassification.lean:94-124` `GAlternating`/`GActivationClosed`/`GSameUnit`/`Gconsistency`；`Origin/VoiceTree.lean`/`VoiceThreeLevel.lean:156` `ParentValid` | 已实装 | L0 |
| F4 | 全局乘积分类 `C^global=G_α∧⋂_v C_{v,α_v}` ∃!（§8 / §15）| `GlobalProductClassification.lean:307` `global_product_complete_unique`（结构语义前提下 ∃!）；`Foundation/CompleteClassification.lean:184` `global_class_complete_unique` | 已实装（条件）| L0 |
| F5 | 子声部开平状态机 `X_{v,t}`/`E_{v,t}`（关闭优先开启）（§9 L544-601）| `Origin/SellClosedLoop.lean`/`SecondSellClosedLoop.lean`/`VoiceThreeLevel.lean:220` `G_false_of_not_parentValid`（ParentValid 失效⟹关闭）| **部分** | L0 |
| F6 | 根声部双向状态机 `RootSel_Θ`、镜像反对称消歧（§10 L605-657）| **缺** — repo 无 `RootSel` (1,1) 同时信号的镜像反对称消歧实装 | **缺** | — |

### G. 风险投影/订单/总定理（§14/§19/§20/§21）

| # | canonical 条目（出处行号）| repo 实装（文件:定理）| 状态 | L级 |
|---|---|---|---|---|
| G1 | 全局安全可行集 `K_Θ(x)≠∅`（§14 L1067-1113）| `Origin/RiskProj.lean`；`Origin/StrategyFamily.lean`（π_Θ 全定义/唯一/因果）| **部分** | L0 |
| G2 | 唯一风险投影 `u*=LexArgmin_{u∈K} J`（§19 L1340-1385）| `Origin/RiskProj.lean` `risk_quantity_projection_unique`（proof-map §7）| **部分** | L0 |
| G3 | 订单调度 + 下一状态 `T_Θ` 全定义唯一（§20 L1389-1447）| `FullDefinitionStrategy.lean:229` `hybridStep` + `:232` `hybrid_step_complete_unique` + `:244` `transition_writes_full_state` | 已实装 | L0 |
| G4 | 最终总定理：每状态 ∃! (D,c,ũ,u*,O,x')（§21 L1451-1525）| `Origin/EngineBridge.lean:54` `strict_assembly_*`（RustEngineContract total-unique 闭环）| **部分** | L0 |
| G5 | 因果性 `hybrid_step_causal`（决策只依赖截至当前信息，§21 条件13）| `FullDefinitionStrategy.lean:249` `hybrid_step_causal`（假设式）| 已实装（假设式）| L0 |

---

## 2. 覆盖率统计与缺口清单

**总条目数：35**（A9 + B5 + C4 + D7 + E5 + F6 + G5；按子分类细目计）

| 状态 | 数量 | 占比 |
|---|---|---|
| 已实装 machine-checked | 24 | 69% |
| 部分 | 7 | 20% |
| 缺 / still-MISSING | 4 | 11% |

**关键缺口清单（canonical 有、repo 缺/部分——核查核心价值，否定性结果 231）**：

1. **E5 杠杆完全分类（缺，最大缺口）**：canonical §13 的总名义 `G_t=∑|n_{v,t}|`、净名义 `N_t=|∑n_{v,t}|`、总杠杆 `L^G_t=G_t/E_t`、净杠杆 `L^N_t=N_t/E_t` **四个量及其约束**在 repo 仅 `leverage : Type` 抽象占位，无算式。**连带**：E2 RiskMode 的 M0-M4 判据依赖保证金缓冲阈值 `MM_t(q)+B^{(i)}`，因 E5 缺失而未绑定具体触发条件。

2. **B4/B5 相对中枢位置六态 + 信号位向量（缺）**：canonical §5 的 `r∈{⊥,I,U⁰,U¹,D⁰,D¹}` 六态（含三买/三卖标记位）与 `b∈{0,1}⁶` 信号位向量（三类买卖点不强制互斥的位编码）未实装。repo 用 CenterStates 位置三态 + 独立买卖点判据（D），与 canonical 的六态/位向量编码不同构。

3. **A6 动态同余算子（部分→still-MISSING 核心）**：canonical §17/§22 的 `C_Θ∘T_Θ=T̄_Θ∘(C_Θ,ē)`（类别空间转移 T̄ + 交换图）未形式化。repo 仅证「转移区分类别」（非退化），未证图表交换。

4. **A5 决策充分性四支（部分）**：canonical §16 的 `C(x)=C(y)⟹K=∧Intent=∧J=∧π=` 四元义务，repo 仅证 π 经 classify 因子分解一支，K/Intent/J 三支「同类⟹同应对」未独立证。

5. **F2 区间套多层嵌套证书（部分）**：canonical §6 的 `J_{ℓ+1}⊆J_ℓ` 嵌套链 + `Sel_Θ` 唯一选择器的完整递归证书未形式化（仅有单层次级别破中枢判据）。

6. **F6 根声部 RootSel 镜像反对称消歧（缺）**：canonical §10 的 (1,1) 同时多空信号的镜像反对称消歧 `RootSel(M_D D)=-RootSel(D)` 未实装。

7. **C4/G1/G2/G4 部分**：中枢生成仍空桩（`chanCenters_empty`，canonical current-strictness-audit Gate 1 自陈未兑现）；K_Θ/J/总定理在 schema/抽象接口层证 total-unique，未落到中枢生成完整 WellFormed 之上。

---

## 3. 结果包六要素

**1. 结论**：canonical「完全分类数学」**已对照结果包逐条核**。35 个细目中 24 已实装 machine-checked（69%）、7 部分、4 缺。**顶层行为商完全分类定义（A1-A4,A7-A9）+ 走势四类（B1-B3）+ 买卖点三类（D1-D7）+ 资本/风险/动作三态机（E1-E4）+ 镜像等变（F1）+ 声部赋格（F3-F4）= 完全分类的主干已 L0 兑现**。但**杠杆完全分类（E5）整体缺失**、中枢位置六态/信号位向量（B4/B5）缺、动态同余算子（A6）与决策充分性四支（A5）仅部分——**不构成 canonical 完全分类全定义域的覆盖**。

**2. 定义依据**：canonical「完全分类」定义为行为商纤维忠实编码（FULL_USER_FORMULA_SOURCE.md §17 L1274-1282 / §22 L1588）。repo `CompleteClassifier.complete`（CompleteClassification.lean:36）的双向核 `classify x=y ↔ BehEquiv trace x y` 是该定义的 schema 兑现；`concrete_behavior_quotient`（ConcreteBehaviorQuotient.lean:175）是具体引擎实例化。各子分类的 ∑1[·]=1 互斥穷尽（§5/§10.2/§12/§13/§18）对应 repo 各 `*_complete_unique` 定理。

**3. 边界条件（结论翻转）**：
- 若把「完全分类」严格读为 canonical §1 的**整个混合状态机的行为商**（含杠杆/保证金/区间套全部 9 节子结构），则 repo **未达全覆盖**（E5/B4/B5/F6 缺 ⟹ 有效域 ⊊ 定义域），结论从「主干覆盖」翻转为「不完整」。
- 若读为「**走势+买卖点+资本/风险/动作的离散态分类**」（§5/§10/§11-12/§18），则 repo 主干**已 machine-checked 覆盖**。
- 若 E5 杠杆算式补全 + A5/A6 四支/动态同余补全，则覆盖率从 69% 升至 ~94%（仅余 B4/B5/F6 编码差异）。

**4. 下游推论**：
- 任何「repo 已完整实装 canonical 完全分类」的声明是**声明膨胀**（090号）——E5 杠杆整体缺失证伪该声明。
- 杠杆/保证金风控（M4 支柱，IBKR 执行）的形式化保证**尚不存在**——RiskMode 五态是抽象骨架，未绑定具体杠杆触发条件。下游执行层不能依赖 repo 提供的杠杆完全分类。
- 完全分类的**有效域 = 缠论离散态结构层（L0）**，**不含**杠杆量化风控层与经验市场层（L2/L3）。

**5. 谱系引用**：
- 本核查直接对应 `formalization-validity-domain.md`（231号）「有效域 ≠ 定义域」——repo 实装（有效域）⊊ canonical 完全分类（定义域），缺口清单是缩小有效域边界的否定性结果。
- 缠论标签 coarser 于行为商：`ClassificationGuardrail.lean` `origin_both_directions_fail` + `Foundation/CompleteClassificationLimits.iglobal_not_complete_minimal`（谱系 598→603→615 概念分离）——A9 是该谱系在核查中的体现，确认「label 商 ≠ 行为商」。
- 买卖点互斥三分失败（D5）：legacy Strict/BSP.lean `no_global_classifies`（谱系 615）重锚到 Origin。

**6. 影响声明**：本产出新建 `docs/canonical-coverage-classification.md`，**只读审计未改任何代码/Lean 文件**。影响范围：为编排者「完全分类的数学有没有对照结果包做」质询提供逐条对照证据 + 缺口清单。不改动任何定义、不改动 formal/ 树、不改动 lakefile。下游可据本矩阵的缺口清单决定是否新开 E5 杠杆完全分类 / A6 动态同余 / B4-B5 信号位向量工位。
