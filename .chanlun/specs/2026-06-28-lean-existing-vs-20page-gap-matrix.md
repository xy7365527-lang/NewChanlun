# 既有 Lean 模块 vs 20 页权威 spec 覆盖/缺口矩阵（scope reconciler 用）

- **产出性质**：只读审计（未改任何代码 / lakefile）。
- **权威 spec**：`.chanlun/specs/2026-06-28-recursive-complete-classification-bsp-pdf-extract.md`（20 页，1269 行）。
- **审计对象**：`formal/Origin/*.lean`（lakefile.toml `Origin` lib，root 全列表已 cat）。
- **认识论等级**：本矩阵结论为 **L0/结构层审计**——基于 `grep` 定理头 + 读 docstring + 读 lakefile roots 的机器证据（635 纪律）。逐定理语义对齐为 [需人工确认 L1：lake env lean 编译核验]，未做。
- **提取/审计日期**：2026-06-28。

---

## 一、七链逐环覆盖矩阵

约定：**覆盖度** = 既有模块对该环 20 页要求的结构覆盖（全/部分/无）；**性质** = 确认覆盖 / 需 reconcile（旧语义）/ 真缺口（无既有）。文件引用格式 `文件:定理`，行号见 grep。

| 链环节 | 20 页 spec 要求（§+公式） | 既有模块覆盖（文件:定理） | 覆盖度 | 缺口（vs 20 页：去根化/18类/互斥分层/协变资本/纯core） | 处置建议 |
|---|---|---|---|---|---|
| **环1** 买卖点谓词 b_ℓ∈{0,1}^6（64 类，**不互斥**）+ Conf^±（P4§5/P5§6） | b_ℓ=(B1,B2,B3,S1,S2,S3)∈{0,1}^6；Σ_u 1[b=u]=1（64 类）；Conf^+_e=⋁B_{i,e}，Conf^-_e=⋁S_{i,e} | `BuySellPredicate.lean`:`signalVector`/`coincidence_allowed`/`signalVector_indicator_sum_one`/`ConfBuy`/`ConfSell`/`ConfBuy_iff_exists` | **部分** | (1) **纯 core 违规**：`signalVector_card_64` 用 `Fintype.card`，64 类计数用 `Finset.univ.filter...card`（行 202/223-227）——spec 硬约束**严禁** Fintype/Finset，须改 List 显式枚举+decide。(2) **未注册 root**（lakefile `Origin` roots 无 `BuySellPredicate`）=孤儿草稿，不参与 build。语义对齐（不互斥 `coincidence_allowed`✓、Conf=⋁✓）。 | **需 reconcile**：保留语义（不互斥+Conf⋁），删 Fintype/Finset 证法改 List+decide，登记为 root。 |
| **环2** 区间套证书 N^δ_{ℓ↓e}（P5§6） | N^δ={Conf(ℓ=e); Cand∧[J_{ℓ-1}⊆J_ℓ]∧N_{ℓ-1↓e}(ℓ>e)}；∀ ∃! N∈{0,1}；S_k 平移不变 | `NestingCertificate.lean`:`N_base`/`N_step`/`N_exists_unique`/`N_mem_zero_one`/`N_translation_invariant`/`Conf`/`Sub`/`subB_iff_Sub`（纯 core，root）★ | **全** | 无结构缺口。`N_translation_invariant` 顺带兑现 §18 自相似第 3 步。`Cand^δ_ℓ` 独立定义式 spec 未给（疑点2），此处 `candOf` 已具体化。 | **确认覆盖**。§6 逐字对齐、纯 core、已 root。 |
| —（环2 重复实现） | 同上区间套 | `IntervalNestCertificate.lean`:`selectΘ`/`nestCertB`/`chiBool`/`selectedByKey_unique`/`selOrder_trichotomy/trans/antisymm`（List NestLevel + 总序best-selection） | **全（重复）** | 与 `NestingCertificate` 对环2 **功能重叠**：前者 N^δ 对 Nat 差递归（§6 忠实），后者 selectΘ 总序选**单个**最佳区间（§6 未显式要求的候选裁决）。 | **需 reconcile（去重）**：定 canonical=`NestingCertificate`（§6 忠实）；`IntervalNestCertificate.selectΘ/selOrder` 的总序原语保留供环5复用（见环5）。 |
| **环3** 候选集 Γ(x) 有限（P10§12） | Γ(x)=⋃(所有级别×买卖点×区间套确认)；\|Γ(x)\|<∞ | 无专用 Γ(x) 组装模块（候选块散见 `IntervalNestCertificate` List NestLevel） | **无** | 无"把每级 b+N^δ 确认聚成 Γ:List Cand"的模块。 | **真缺口 → 新建**（lean-gamma）：Γ:状态→List Cand（List 即有限，\|Γ\|<∞ 平凡）。依赖环1/环2 输出。 |
| **环4** 关系 H,V → 角色 R(g)=H×V×δ（**18 类互斥**，P6§7/P7§8）★ | H∈{First,SameFollow,SameReverse}，V∈{Ambient,FollowParent,ShortDiff}，R=(H,V,δ)；Σ_{r∈ℛ}1=1（3×3×2=18）；无根特例 | `OperationRole18.lean`:`RoleH`/`RoleV`/`Role18`/`allR18`/`card18`/`role18_count_one`/`classifyR18`/`classifyV_germ_ambient`/`classifyV_shortDiff_iff`（纯 core List+decide，root）★ | **全** | 无结构缺口。docstring 显式"去根化：无 RootRole/根特例，σ_p 用 `Option Side`（none=胚元）"，显式标 L0+疑点5（18 类经验可达性 L2 未验）。 | **确认覆盖**（首波，20 页对齐标杆）。互斥责任分层之"角色层互斥"在此严格成立。 |
| **环5** ℛ_Θ 解释器唯一化 (𝒟,ℬ,𝒦) via 总序 ≺_Θ（P10§12）★ | (𝒟,ℬ,𝒦)=ℛ_Θ(Γ(x))；≺_Θ 平移不变全序；∀x ∃!(𝒟,ℬ,𝒦)；按序确定性 fold | 无"Γ→(𝒟,ℬ,𝒦) 三桶"模块。原语：`LexArgmin.lean`:`lexLe`/`lexLe_total/trans/antisymm`（全序）；`IntervalNestCertificate`:`selOrder_*`（总序）；`SubVoiceOpenClose`:`nextActive_trichotomy`（close/open 二值，子声部） | **无** | selectΘ 是"选 1 个最佳区间"，**非** Γ→三桶 fold（机制不同，见特别核 (a)）。SubVoiceOpenClose 仅 close/open 二值，非 (𝒟,ℬ,𝒦) 三桶。 | **真缺口 → 新建**（lean-interp）：Cand 上平移不变全序 ≺_Θ + insertionSort + 确定性 fold→(𝒟,ℬ,𝒦)；∃! 由 fold 确定性。**复用** `LexArgmin.lexLe`/`selOrder` 全序原语，不 reconcile `IntervalNestCertificate`。 |
| **环6a** 活动集 A_{t+1}=AncOK(A^raw)（P11§13） | A^raw=(A_t∖𝒟)∪ℬ；A_{t+1}=AncOK(A^raw)，AncOK(A)={a∈A:Anc(a)⊆A}；a∈A⟹Anc(a)⊆A；S_k AncOK=AncOK S_k | `AncestorClosure.lean`:`ancOK`/`ancOK_subset`/`ancestorClose`/`ancestorClose_anc_closed`/`mem_ancestorClose_of_anc_closed`/`parent_in_raw_of_mem_ancestorClose`（root）；另 `ConstraintSystem.c2_ancestorClose` | **全（AncOK 部分）** | AncOK 闭合完备（`ancOK`=单次 filter，`ancestorClose`=迭代不动点，**解了疑点4 单次vs迭代**）。但 A^raw=(A_t∖𝒟)∪ℬ 的原始更新依赖环5 的 𝒟/ℬ（缺口）；平移不变 `S_k AncOK=AncOK S_k` [需人工确认是否已证]。 | **AncOK 确认覆盖** → lean-active **reconcile/复用** `AncestorClosure`，**勿新建闭包**；只新建 A^raw 更新（接环5 输出）。 |
| **环6b** 目标头寸 p̃_{t+1}=Σ_{g∈A}Leg(g)（P11§14） | p̃=Σ Leg(g)；多空分组求和；∀x ∃! p̃ | `SeparateLedger.lean`:`Leg`(qPlus/qMinus)/`legLong/legShort/legHedged`/`SepPosition`/`Net`/`net_append`/`parent_leg_survives_child_open` | **部分** | Leg 原语 + 求和（`net_append`）齐备，§10 𝒫^sep 分账本对齐。但"A_{t+1} 列表 fold→p̃"未组装（依赖环6a/环5）。 | **需 reconcile/补装**：在 `SeparateLedger.Leg` 上 fold A_{t+1}→p̃；∃! 由 fold 确定性。 |
| **环7a** 全定义策略 π_Θ：𝒦_Θ（8 约束）+ LexArgmin（P12§15）★ | 𝒦_Θ⊆𝒫^sep，𝒦_Θ≠∅，8 类约束；J_x=‖p-p̃‖²_W+λCost+νRisk；p*=LexArgmin_{𝒦_Θ}J；O=Schedule_Θ(p*-p_t) | `ConstraintSystem.lean`:`Feasible`/`feasible_iff_all17`/`feasible_nonempty`(=𝒦_Θ≠∅)/`c1..c17`；`LexArgmin.lean`:`lexArgmin`/`lexArgmin_exists_unique`/`RiskProjection.project`/`project_existsUnique` | **部分（约束+argmin 全；J/Schedule 缺）** | 𝒦_Θ：c1-c17（17 细约束）覆盖 8 类（父闭合 `c2_ancestorClose`/同单位短差 `c4_sameUnitHedge`/手数 `c1_qtyGrid`/杠杆保证金 `c6-c9`/三阶段资本 `c10-c14`/订单执行 `c15-c17`）✓。LexArgmin 唯一性✓（LexKey=List Int，**离散**）。**缺**：J_x 连续加权范数 ‖·‖²_W（LexKey 整数离散，疑点7 离散化未做）；`Schedule_Θ`（O=p*-p_t 排程，无模块）。 | **𝒦_Θ+LexArgmin 确认覆盖** → lean-strategy **reconcile**：补 J_x 离散网格化 + `Schedule_Θ`。 |
| **环7b** 中心定理 §16：12 假设 → ∀x ∃! O=π_Θ(x)（P13§16）★ | 沿七链逐环传递唯一性 b→N^δ→H/V→R→(𝒟,ℬ,𝒦)→A→p̃→p*→O | `DecisionSufficiency.lean`:`decision_pipeline_exists_unique`/`decision_policy_total_unique`/`policy_factors_unique`；`MainTheorem.lean`:`main_theorem`/`main_hybrid_step_exists_unique`/`main_pipeline_exists_unique_via_foundation` | **部分（需 reconcile）** | 既有 ∃! 是 **Foundation/hybrid 管线**（StrictState/Event）的唯一性，**非** 20 页七链 12 假设的 ℭ_Θ 逐环穿线（输入须为新 R_Θ+18 类）。唯一性骨架可复用，七链装配缺。 | **需 reconcile**：以 `DecisionSufficiency`/`MainTheorem` 唯一性手法为骨架，重穿 12 假设→新七链（环3/环5 缺口补齐后）。 |

---

## 二、三大严格性升级覆盖矩阵

| 升级 | 20 页 spec 要求（§+公式） | 既有模块覆盖（文件:定理） | 覆盖度 | 缺口 | 处置建议 |
|---|---|---|---|---|---|
| **升级1 去根化**（P1-8/P17）：消除根级别/根头寸特例，根=父为胚元 ∂ 的普通子级；分类轴 {根/同级/次级}→H×V×δ | 无 RootRole；p(e)=∂_{ℓ+1}（status=Germ）；V=Ambient 吸收"根方向"；禁 RootRole 枚举 | **新侧**：`OperationRole18`（去根化角色，σ_p=Option Side none=Germ）✓；`RecursiveLevelSystem`:`chan_recursive_self_similar`（无根递归）；`CanonicalQuotientTower`:`atLevel_self_similar` | **部分（新侧覆盖，旧侧需清理）** | **旧根特例 paradigm 并存**：`OperationRole.lean`(`rootDir`/`root_role_is_rootDir`)、`MutexElement.lean`(`LevelRelation.root`/`levelRelation_root_iff`/`isRootRel`)、`MutexExhaustive`/`MutexRecursive`/`MutexFinalTheorem`(全建在 root/same/sub × 4-role 上)=**正是 15 页 {根/同级/次级} 含根特例 paradigm**，被 20 页去根化取代。`RootSelDisambig.rootSel`（根方向裁决）亦旧 paradigm（已自标 `not_true_classification`）。 | **需 reconcile（弃旧/重锚）**：`OperationRole18` 为 canonical 去根化角色层；`OperationRole`+`MutexElement`+`MutexExhaustive/Recursive/FinalTheorem` 是被取代的旧 paradigm——重锚到 18 类或显式弃用（删前 grep 复用点，遵 no-patch 保留契约锚原语谱系）。 |
| **升级2 互斥责任分层**：谓词层 64 不互斥 / 角色层 18 互斥 / 解释器层 R_Θ 总序唯一化 | 三层分工 | 谓词不互斥 `BuySellPredicate.coincidence_allowed`✓；角色互斥 `OperationRole18.role18_count_one`✓；解释器总序=环5 | **部分** | 谓词层（Fintype 须 reconcile，环1）+ 角色层（✓）已落；**解释器层 R_Θ 总序唯一化=真缺口**（环5）。分层概念本身在 `OperationRole18` docstring 已显式声明。 | **谓词/角色层确认**（谓词层随环1 reconcile）；**解释器层随环5 新建**。 |
| **升级3a 分类自相似 §18**：ℭ_Θ(S_k x)=S_k ℭ_Θ(x)（八步分量等变） | S_k:ℓ↦ℓ+k 下九元组 ℭ_Θ 等变 | 散片：`NestingCertificate.N_translation_invariant`（步3 N^δ✓）；`OperationRole18.classifyR18`（R 仅依赖相对方向→步4/5 R(S_k g)=R(g) 隐含）；`RecursiveLevelSystem`/`CanonicalQuotientTower` self-similar（步1 D） | **部分/真缺口** | 复合 ℭ_Θ 九元组在 S_k:ℓ↦ℓ+k 下等变**未组装/未证**；步2(b)、步6(ℛ_Θ)、步7(AncOK 平移)、步8(Leg 平移) 缺。`DynamicCongruence.dynamic_congruence_commutes` 是**另一种**同余（事件类/镜像 dynamic，**非** S_k 级别平移，见特别核 (b)），不可混入 §18。 | **真缺口 → 新建**（lean-selfsim）：S_k=Int 平移作用 + 八步等变引理合成 ℭ_Θ 等变。**复用** `NestingCertificate.N_translation_invariant`；用 `OperationRole18` 给 R(S_k g)=R(g)。**勿** reconcile `DynamicCongruence`（正交）。 |
| **升级3b 策略自相似 §19**：π_Θ(S_k x)=S_k π_Θ(x)（条件：𝒦_Θ/J/Schedule 等变） | 条件定理 | 无 | **无** | 无 π_Θ 平移等变模块。 | **真缺口 → 新建**（随 lean-selfsim/lean-strategy）。 |
| **升级3c 协变资本（方案 A）** vs 既有绝对资本 | 协变/无量纲资本（保 J 平移等变，免破策略自相似） | `LeverageCapital.lean`:`LeverageAccount`(grossCap/netCap)/`grossNotional`/`netNotional`/`net_le_gross`（**绝对** Nat notional + 绝对 Int caps） | **部分（语义错向）** | `LeverageCapital` 用**绝对**名义额上限——spec §19/疑点6 明示绝对资本约束**非平移等变**，破坏策略自相似（潜在概念矛盾，spec 已标 escalate 候选）。**方案 A 协变资本 spec 尚未落地**（`...absolute-capital-equivariance-resolution-pdf-extract.md` 文件**当前不存在**，已核）。 | **需 reconcile（blocked）**：`LeverageCapital` 绝对→无量纲/协变；**前置阻塞**=方案 A 决议 spec 未落盘——reconciler 须等该 spec 或随其一并改。见特别核 (e)。 |
| **（涌现）ℓ_max 涌现 §3/P18 条件4**：ℓ_max(t+1)>ℓ_max(t) ⟺ 前沿满足 Qual_*；Prom_*；∂ 胚元 | C_{ℓ+1}=Prom_*(C_ℓ,…)；Qual_*∈{0,1} | `RMoveCompose.composeMove`(≥3 subs→高级 RMove=Prom)/`SecondTypeStructure`；`CanonicalQuotientTower.legalDecomp_segments_ge_three` | **部分** | Prom（composeMove）✓；**缺** Qual_* 提升谓词显式定义、ℓ_max 双向涌现判据（疑点3 m≥3 下界）、∂ 边界胚元 Germ→实元素转化。 | **需 reconcile/补装**：在 `RMoveCompose` 上加 Qual_* + ℓ_max 双向 decide + ∂ Germ 转化。 |
| **（地基）统一递归单元 C_ℓ≅C_*（§1-2）+ 去根化级别系统 𝕃=ℤ** | C_ℓ=(S,Γ,A,P,E)；C_ℓ≅C_*；𝒩_{ℓ+1}∘F_ℓ=F_*∘𝒩_ℓ；𝕃=ℤ 有限支撑 | `RecursiveLevelSystem`:`chan_recursive_self_similar`/`chanLevelAt_self_similar`；`CanonicalQuotientTower`:`CanonicalTower`/`toRecursiveLevelSystem`/`toRLS_self_similar` | **部分** | 自相似递归塔✓；五元组 C_ℓ=(S,Γ,A,P,E) 显式同型 `C_ℓ≅C_*` + 归一化交换律 𝒩∘F=F∘𝒩 + 𝕃=ℤ 有限支撑 ℓ_max 动态——**未显式建模**。 | **需 reconcile/补装**：在递归塔上显式五元组同型 + 归一化共轭。 |

---

## 三、五个特别核查结论（任务指定 a-e）

**(a) `IntervalNestCertificate.selectΘ` 总序 vs lean-interp（环5 R_Θ 总序唯一化）——是否同一机制？**
**否，不同机制。** `selectΘ:List Interval→Option Interval`（`selectΘ_isSelectedByKey`/`selectedByKey_unique`）是按 key 总序（selOrder：endTime 晚优先，tiebreak startTime/idx）选**单个最佳区间**——属环2 区间套候选裁决。环5 `ℛ_Θ(Γ)→(𝒟,ℬ,𝒦)` 是对 Γ 按平移不变全序 ≺_Θ 排序后**确定性 fold 出三桶**（close/open/record）。**结论**：lean-interp **新建**，不 reconcile `IntervalNestCertificate`；但**复用**其 `selOrder_trichotomy/trans/antisymm` 与 `LexArgmin.lexLe` 全序原语作 ≺_Θ 的可判定线序构件。

**(b) `DynamicCongruence` vs lean-selfsim（§18/§19）——ClassLabel 是否迁到 18 类 R？capital 涉及？**
`DynamicCongruence.dynamic_congruence_commutes` 是**事件类/dynamic 同余**（`classifyState:StrictState→ClassLabel` 的 T̄ 交换），**与 §18 级别平移 S_k:ℓ↦ℓ+k 等变正交**（自标 `dynamic_congruence_not_true_classification`、`StructuralSynthesisL0`）。**结论**：lean-selfsim **新建**，**勿** reconcile `DynamicCongruence`（不同同余，混入会偷换概念）。§18 需要的 `R(S_k g)=R(g)` 由 `OperationRole18.classifyR18`（仅依赖相对方向）直接给出——即自相似应**直接用 18 类 R**，不是把 `ClassLabel` 迁入 `DynamicCongruence`。**capital 不涉及** `DynamicCongruence`（它只管事件类，不碰资本）。

**(c) `ConstraintSystem`/`LexArgmin`/`DecisionSufficiency` vs lean-strategy（§15 π_Θ + §16 唯一性）——覆盖多少？**
覆盖**约束可行集 + argmin + 唯一性骨架**，缺 **J 连续目标 + Schedule + 七链穿线**。`ConstraintSystem.feasible_iff_all17`+`feasible_nonempty` 覆盖 §15 𝒦_Θ 八约束（17 细约束）且证 𝒦_Θ≠∅；`LexArgmin.lexArgmin_exists_unique`/`project_existsUnique` 覆盖 LexArgmin 唯一（离散 LexKey）；`DecisionSufficiency.decision_pipeline_exists_unique`/`MainTheorem.main_theorem` 给 ∃! 唯一性手法。**缺口**：J_x 加权范数 ‖p-p̃‖²_W 连续优化（LexKey 整数离散，疑点7 网格化未做）、`Schedule_Θ`、以及 §16 的 12 假设须按**新七链**（环3 Γ + 环5 R_Θ + 环4 18 类为输入）重穿——既有 ∃! 是 Foundation/hybrid 管线非 20 页七链。**结论**：lean-strategy **reconcile** 这三者（复用约束/argmin/唯一性骨架，补 J 网格化 + Schedule + 七链装配）。

**(d) AncOK（活动集父容器闭合）在哪个模块 vs lean-active？**
**`AncestorClosure.lean` 是专用模块**（`ancOK`=单次 filter、`ancestorClose`=迭代不动点闭包、`ancestorClose_anc_closed` 证 a∈A⟹Anc(a)⊆A、`mem_ancestorClose_of_anc_closed`），**顺带解了 spec 疑点4（单次 vs 迭代：两者都有）**；另 `ConstraintSystem.c2_ancestorClose` 是其约束化身。**结论**：lean-active **reconcile/复用 `AncestorClosure`，勿新建闭包**；只需新建 A^raw=(A_t∖𝒟)∪ℬ 原始更新（接环5 𝒟/ℬ 输出）+ [需人工确认 `S_k AncOK=AncOK S_k` 平移不变是否已证]。

**(e) `LeverageCapital` 绝对资本 vs 方案 A 协变资本的差距？**
`LeverageCapital` 用**绝对**名义额（`grossNotional`/`netNotional` Nat + `grossCap`/`netCap` 绝对 Int），与方案 A**协变/无量纲资本**方向相反。spec §19/疑点6 明示：绝对资本约束**非平移等变**，破坏策略自相似 π_Θ(S_k x)=S_k π_Θ(x)——这是 spec 自标的**潜在概念矛盾/escalate 候选**。**且方案 A 决议 spec（`2026-06-28-absolute-capital-equivariance-resolution-pdf-extract.md`）当前在 `.chanlun/specs/` 不存在（已核）**。**结论**：资本层 reconcile **被阻塞**于方案 A spec 未落盘；reconciler 须等该 spec 或与其同步改 `LeverageCapital`（绝对→无量纲化以保 J 等变）。**勿**在绝对资本上加 workaround（no-workaround：这是概念矛盾，须走方案 A）。

---

## 四、汇总：确认覆盖 / 需 reconcile / 真缺口

- **确认覆盖（无需工位，仅核验）**：环2 `NestingCertificate`（N^δ §6 忠实）；环4 `OperationRole18`（18 类去根化角色，标杆）；环6a AncOK `AncestorClosure`；环7a 的 𝒦_Θ `ConstraintSystem` + LexArgmin `LexArgmin`。
- **需 reconcile（既有模块在，语义/证法/装配待对齐）**：环1 `BuySellPredicate`（去 Fintype/Finset 改 List+decide + 注册 root）；环2 去重（`IntervalNestCertificate` vs `NestingCertificate`）；环6b p̃ fold 补装（`SeparateLedger` 上）；环7b §16 七链穿线（`DecisionSufficiency`/`MainTheorem` 骨架重穿）；升级1 去根化旧 paradigm 清理（`OperationRole`+`MutexElement`+`MutexExhaustive/Recursive/FinalTheorem` 弃旧/重锚到 18 类）；升级3c 协变资本（`LeverageCapital` 绝对→协变，**blocked 于方案 A spec**）；涌现 Qual_*/ℓ_max（`RMoveCompose` 补装）；地基 C_ℓ≅C_* 同型（递归塔补装）。
- **真缺口（无既有，需新建）**：环3 Γ(x) 候选集组装（lean-gamma）；环5 ℛ_Θ 解释器三桶唯一化（lean-interp，复用全序原语）；升级3a/3b 自相似 ℭ_Θ/π_Θ 平移等变（lean-selfsim，复用 N^δ 平移不变 + 18 类 R 相对性）；环7a 的 J_x 离散网格 + Schedule_Θ。

---

## 结果包（六要素 · 完整版）

1. **结论**：见上四节矩阵。七链中环2/环4 确认覆盖且对齐 20 页；环6a-AncOK、环7a-𝒦_Θ/LexArgmin 确认覆盖；环1/环6b/环7b 需 reconcile；环3/环5 真缺口。三升级：去根化新侧（`OperationRole18`）覆盖但旧 paradigm（Mutex* 家族）需清理；互斥责任分层谓词+角色层覆盖、解释器层缺；自相似 ℭ_Θ/π_Θ 等变真缺口（散片可复用）；协变资本 reconcile 但 blocked 于方案 A spec 缺失。

2. **定义依据**：20 页 spec 七链对照表（行 1161-1177）+ §5/§6/§8/§12/§13/§15/§16/§18/§19 方框公式 + 形式化指引 Lean 表（行 1208-1225）；既有模块的 `grep` 定理头 + docstring（`OperationRole18` 行 16-59 显式声明去根化/18 类/L0+L2；`BuySellPredicate` 行 200-227 Fintype.card 实证；`AncestorClosure` ancOK/ancestorClose；`ConstraintSystem` c1-c17+feasible_nonempty；`LexArgmin` lexArgmin_exists_unique；`LeverageCapital` grossCap/netCap 绝对）+ lakefile.toml `Origin` roots 列表（`BuySellPredicate` 不在其中）。

3. **边界条件（结论翻转条件）**：本矩阵为 L0 结构层审计——(i) 若 `lake env lean` 编译核验（L1）发现某"确认覆盖"模块有 sorry/语义偏差，则该行降级；(ii) 若环5/环3 实为某未识别模块覆盖（grep 漏网），"真缺口"翻转——已对 Γ/interpret/Schedule grep 未中，置信但非穷尽；(iii) 若方案 A 协变资本 spec 落盘后判定绝对资本可保留（如按级别归一化即等变），则升级3c 从"reconcile"降为"补装"；(iv) 18 类经验可达性是 L2（spec 疑点5），本审计不评经验有效性。

4. **下游推论**：scope reconciler 工位可据此精确分派——**勿重复造** `NestingCertificate`(环2)/`OperationRole18`(环4)/`AncestorClosure`(AncOK)/`ConstraintSystem`+`LexArgmin`(𝒦_Θ/argmin)；**新建**仅 lean-gamma(环3)/lean-interp(环5)/lean-selfsim(§18/19)；**reconcile** 工位接既有模块（环1 去 Fintype、环6b/环7b 装配、Mutex* 清理）。这直接回应任务提到的"lean-nest 差点重复 IntervalNestCertificate"——环2 已**双重**覆盖，应去重而非再建。

5. **谱系引用**：去根化对应 MEMORY `newchanlun-no-patch-keep-primitive-when-contract-anchored`（删旧 root paradigm 前须 grep 复用点+查契约锚）；18 类角色/嵌套对冲对应 `coverage-engine-needs-tower-export-bridge`（互斥全定义策略=买卖点入场+多级角色/嵌套对冲；classify 不导出塔=环4→塔前置）；环5 R_Θ 的 𝒟_x=exit 来源对应 `theta_v0-recognize 硬编码 exit:false`；协变资本矛盾对应 `formalization-validity-domain`（绝对资本非平移等变=有效域问题）+ no-workaround（概念矛盾走方案 A 不绕过）；§16 七链穿线对应 `t-lean-all-not-just-L`。[需人工确认]：本 spec §16/§18/§20 与既有 T₁-T₅₉ 编号精确映射（spec 疑点8 未结）。

6. **影响声明**：本产出**仅新增** `.chanlun/specs/2026-06-28-lean-existing-vs-20page-gap-matrix.md`（审计文档），**未改任何 `.lean`/lakefile/代码**。影响范围=为 Lead scope reconciler 工位提供分派依据；不改变任何既有定义或模块状态。
