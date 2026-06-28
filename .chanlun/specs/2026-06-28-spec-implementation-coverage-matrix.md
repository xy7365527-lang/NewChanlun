# 20 页 spec 逐条 vs Lean/rust 实装 —— 完整覆盖矩阵（验收门）

- **审计日期**：2026-06-28
- **权威 spec**：`.chanlun/specs/2026-06-28-recursive-complete-classification-bsp-pdf-extract.md`（20 页 / 1269 行，§1–§20）
- **被审实装**：
  - Lean：`formal/Origin/*.lean`（81 个 .lean 文件）
  - Rust：`rust/src/theta_v0/`（types/config/classifier/strategy/ledger/complete/closed_loop/…）
- **方法（认识论等级）**：纯静态 grep + 读定理头/函数签名。**未跑 `lake build` / `cargo build`**，编译性依赖文件自证注释，非本审计验证。本矩阵结论等级 = **L0/L1**（结构存在性核对，非 L2 经验验证）。
- **纪律**：只读机器证据。不确定标 `[需确认]`。遗漏据实报，不粉饰。

---

## 0. 头条发现（Headline）

1. **Lean 全树零 `sorry` / 零 `admit` / 零 `axiom` 声明**（静态 decl-grep 坐实；~90 处 "sorry/admit/axiom" 全是中文注释 `禁…`，含 `SevenLinkStrategy.lean:70`、`ConcreteBehaviorQuotient.lean:427` 两处行首 `axiom` 也是注释 prose）。无补丁、无占位。
2. **Rust 生产代码零 `todo!`/`unimplemented!`/stub**（`interp.rs` 显式标注其 close 机制"非 stub"）。唯一 `#[cfg(test)]` 门控模块 = `backtest/`。
3. **§5–§15（七链核心 11 节）双层完整且 Lean 端真证**（多为 `decide` 支撑或确定性 fold）。
4. **核心薄弱点不在"没写"，在"以假设承载未兑现"**：完全性/唯一性/自相似定理（§16/§18/§19/§20/七链）是 **conditional 真证** —— 结论 `∀x ∃! O=π_Θ(x)` 成立 **modulo** 结构体捆绑的前提（Prom/b/J/Schedule 等变、资本协变）。前提是 `structure` 字段（premise），**不是 axiom，不是 sorry，不是 True**。
5. **唯一的"概念层"真遗漏 = 提升层 Prom_*/Qual_***：`Qual` 在 Lean 全树**完全不存在**；`Prom` 仅作抽象 `Bool` 参数 + `PromEquivariant` 谓词（作 `classify_equivariant` 的前提 `hProm` 携带），**无具体 Prom_ℓ 算子、无 𝒩Prom=Prom_* 定理**。连带 §20 条件 4（涌现⟺Qual）无法形式化。

---

## 1. 完整覆盖矩阵（逐 § 一行）

格式：`§N 条目 | spec 公式 | Lean(文件:名) | rust(文件:fn) | 状态`
状态：**完整**=双层有且 Lean 真证；**部分**=单层缺/conditional 前提未兑现/表示差异；**遗漏**=整条核心缺失。

| § | 条目 | spec 公式 | Lean 定理(文件:名) | rust 实装(文件:fn) | 状态 |
|---|------|----------|-------------------|-------------------|------|
| §1 | 去根化级别系统 | 𝕃=ℤ；supp(D_t)=[ℓmin,ℓmax]；ℓmax(t+1)=ℓmax(t)+1 | `RecursiveLevelSystem`:`RecursiveLevelSystem`/`chan_recursive_self_similar`（**Nat 非 ℤ**；**无 ℓmax 涌现定理**，ℓmax 仅作 `CandidateSet.State` 字段） | `config.rs:74 l_max`(默认6)；`classifier/mod.rs:178 classify_impl`(0..=l_max)；`level.rs:32 RLevel` | **部分** |
| §2 | 统一递归单元 C_ℓ + 自相似归一化 | C_ℓ=(S,Γ,A,P,E)；C_ℓ≅C_*；𝒩_{ℓ+1}∘F_ℓ=F_*∘𝒩_ℓ | `SelfSimilarity`:`shiftState`/`shiftCand`(=𝒩/S_k)、`N_shift_invariant`；`RecursiveLevelSystem.chan_lift_eq_composeStep`(F_*)（**无字面 5 元组 record**） | `descend.rs:53 RMove{Segment,Compose}`+`recursive_tower.rs:63 LeveledMove`（**3 字段非 5 元组；无 𝒩/F_***） | **部分** |
| §3 | 提升 Qual_*/Prom_ℓ + 涌现 | Qual_*∈{0,1}；Prom_ℓ(e₁…)=E；𝒩Prom_ℓ=Prom_*(𝒩e…)；ℓmax↑ | `SelfSimilarity`:`PromEquivariant`(Prop)、`promote:State→Nat→Bool`(抽象参)（**Qual 不存在；无具体 Prom_ℓ 算子；无 𝒩Prom=Prom_* 定理**） | `rmove_compose.rs:68 compose_move`；`recursive_tower.rs:180 compose_level`；门 `config.rs:76 min_parts_per_level=3`（机制有，**未命名 Prom/Qual**） | **部分（核心薄弱）** |
| §4 | 边界胚元 ∂_{ℓ+1} | ∂∈C_{ℓ+1}；status=Germ；p(e)=∂；∂⟶E | `OperationRole18.classifyV_germ_ambient`/`classifyH_first`（**无 Germ 归纳/无 ∂ 父容器对象，仅 parent=none 隐式编码**） | `coverage.rs:75 CoverageElement.parent:Option<usize>`(None=∂)、`:329 ancestors`（**无 Germ 标识符**） | **部分** |
| §5 | 买卖点向量 b_ℓ 64 类 | b_ℓ∈{0,1}^6；Σ_u 1[b=u]=1 | `BuySellPredicate`:`SignalVector`、`allBSP`、`allBSP_length_64`、`signalVector_indicator_sum_one`（**真证 decide**） | `types.rs:173 BspBits`、`:229 class_index`/`:242 from_class_index`；`bsp.rs:76 endpoint_to_bsp`（往返测试 `types.rs:390`） | **完整** |
| §6 | 区间套证书 N^δ | N^δ_{ℓ↓e}；Conf^±=⋁B/⋁S；∃!；平移不变 | `NestingCertificate`:`N`、`Conf`、`N_exists_unique`、`N_translation_invariant`、`N_mem_zero_one`（**真证**）；`IntervalNestCertificate`:`selectΘ`/`chiBool` | `nest.rs:163 NestCertificate`/`:106 confirm`/`:62 is_sub`/`:114 Chi`；`types.rs:202 conf_plus`/`:210 conf_minus` | **完整** |
| §7 | 水平 H / 垂直 V 关系 | H∈{First,SameFollow,SameReverse}；V∈{Ambient,FollowParent,ShortDiff}；各 Σ=1 | `OperationRole18`:`RoleH`/`RoleV`、`classifyH`/`classifyV`、`roleH_count_one`/`roleV_count_one`（真证） | `coverage.rs:407 Horizontal`/`:498 horizontal_relation`；`:424 Vertical`/`:525 vertical_relation` | **完整** |
| §8 | 完整角色 R(g) 18 类 | R=(H,V,δ)；ℛ=3×3×2；Σ_{r∈ℛ}1=1 | `OperationRole18`:`Role18`、`allR18`、`card18`(=18 decide)、`role18_count_one`（**真证**） | `coverage.rs:448 OperationRole`/`:563 operation_role`；`:393 Dir`(δ) | **完整** |
| §9 | 短差 ShortDiff | V=ShortDiff⟺σ_p≠0∧δ=-σ_p；父多则空/父空则多 | `OperationRole18`:`classifyV_shortDiff_iff`/`..long_parent_short..`/`..short_parent_long..`（真证） | `coverage.rs Vertical::ShortDiff`(`:525 vertical_relation` 计算) | **完整** |
| §10 | 分账本 Leg(g) | 𝒫^sep；Leg=s_g e^±_{ν(g)}；Δq^child=0；s_g=s_{p(g)} | `SeparateLedger`:`Leg`、`SepPosition`、`hedged_leg_nonzero_but_net_zero`、`parent_leg_survives_child_open`（真证）；`ActiveSet.legOf` | `ledger/separate.rs:69 Leg`/`:184 eat_sep`/`:239 open_child_leg`；`strategy/ledger.rs:324 ledger_step`；`coverage.rs:608 leg_target` | **完整** |
| §11 | 局部解释器 ℐ_* | ℐ_*→(𝒟,ℬ,𝒦)；ℐ_ℓ=𝒩⁻¹∘ℐ_*∘𝒩；平移自相似 | `RThetaInterp`:`RΘ`、`Triple`(D/B/K)、`rTheta_order_invariant`、`witness_shift_inv`；`SelfSimilarity.InterpEquivariant`（真证） | `interp.rs:416 interpret`→`:104 Buckets`(三桶)；`:128 assemble_gamma`（close 机制注明非 stub） | **完整** |
| §12 | 冲突互斥化 ℛ_Θ | Γ(x)；\|Γ\|<∞；≺_Θ 平移不变全序；∃!(𝒟,ℬ,𝒦) | `CandidateSet`:`gamma`/`gamma_finite`；`RThetaInterp`:`candLe`+`candLe_total/_trans/_antisymm`、`sortΓ`、`rTheta_exists_unique`（**真证全序**） | `interp.rs:128 assemble_gamma`/`:389 theta_lt`+`:342 theta_key`；`exec.rs:145 ConflictKey` | **完整** |
| §13 | 活动集 AncOK | A^raw=(A∖𝒟)∪ℬ；A_{t+1}=AncOK(A^raw)；a∈A⟹Anc(a)⊆A | `ActiveSet`:`rawUpdate`、`activeNext`、`ancOKG`、`activeNext_anc_closed`、`parent_mem_activeNext`；`AncestorClosure`:`ancestorClose_anc_closed`（真证） | `coverage.rs:378 active_set_step`/`:362 ancestor_close`/`:329 ancestors`；`complete/state.rs:107 ancestor_closed` | **完整** |
| §14 | 目标头寸 p̃ | p̃=Σ_{g∈A}Leg(g)；∃! | `ActiveSet`:`ptilde`、`ptilde_exists_unique`、`net_ptilde_append`（真证） | `coverage.rs:663 net_target_units`/`:677 gross_target_units`/`:636 strategy_target_legs` | **完整** |
| §15 | 全定义策略 π_Θ | 𝒦_Θ≠∅；J_x；p*=LexArgmin；O=Schedule(p*-p_t) | `FullDefinitionStrategy`:`policyTheta`、`hybrid_step_complete_unique`；`LexArgmin`:`lexArgmin_exists_unique`；`ConstraintSystem`:`Feasible`(**17 约束 c1–c17**)、`feasible_nonempty`；`CoverFeasibleTarget`:`target_isLexArgmin`（真证） | `coverage.rs:943 feasible_candidates`/`:991 pi_theta_position`/`:1022 schedule_order`/`:967 j_theta_key`；`intent.rs:270 lex_argmin` | **完整** |
| §16 | 策略全定义定理 | 12 假设 ⟹ ∀x ∃! O_{t+1}=π_Θ(x) | `MainTheorem`:`main_theorem`、`MainTheoremClosure`；`SeparateFinalTheorem`:`theorem_final`、`FinalTheoremHypotheses`（**conditional 真证；12 假设=structure 字段 premise**） | `complete/mod.rs:48 TransitionTheta::step`；`closed_loop/conformance.rs:130 step_spec_total_unique_per_state`（**断言 per-state 确定性，非 ∃! 证明 → 证明在 Lean**） | **部分（conditional；假设 2 Prom 未兑现）** |
| §17 | 全局分类函数 ℭ_Θ 九元组 | ℭ_Θ=(D,(b_ℓ),(χ^±),Γ,(R(g)),ℛ_Θ,A,p̃)；𝒳=⊔C_c；Σ_c 1=1 | `GlobalProductClassification`:`GlobalProductSpec`(=G1∧G2∧G3 父子一致 × Local)、`global_product_complete_unique`（**真证，但是 3-合取一致性结构，≠字面九元组**；九元组管线散落 `SelfSimilarity.classify_equivariant`+`SevenLinkStrategy`） | **NOT-FOUND**（无九元组 struct；`classifier/mod.rs:87 Classification` 是 per-x，`complete/state.rs:236 CompleteState` 未标注九元组） | **部分（对齐缺口）** |
| §18 | 自相似证明（八步等变） | ℭ_Θ(S_k x)=S_k ℭ_Θ(x)；八步 Prom/b/N/H/V/R/≺/AncOK/p̃ | `SelfSimilarity.classify_equivariant`（**conditional 真证**：步 3–8 由 `N_shift_invariant`/`H_V_R_shift_invariant`/`candLe_shift'`/`AncOK_shift_equivariant`/`ptilde_shift_equivariant` 真兑现；**步 1 Prom、步 2 b 由前提 `hProm:PromEquivariant`/`hBsp:BspEquivariant` 携带**） | **NOT-FOUND**（proof 层，rust 不证；仅 `backtest/metrics.rs:565` 多空镜像 ℭ_{MΘ}∘M_X=M_S∘ℭ_Θ 注释） | **部分（6/8 真证，2 步为假设）** |
| §19 | 策略自相似证明 | 𝒦_Θ/J/Schedule 等变 ⟹ π_Θ(S_k x)=S_k π_Θ(x) | `SelfSimilarity`:`piTheta_equivariant`/`piTheta_equivariant_strong`、`feasibleSet_equivariant_witness`、`argmin_equivariant_witness`（**conditional**：前提 `Jinv`/`schedEquiv`/`hstrict`；依赖 `CovariantCapital.feasibleSet_equivariant_strong`） | N/A（策略产订单，无等变证） | **部分（conditional）** |
| §20 | 五条件 + 终极结论 | 1 互斥穷尽 / 2 分类自相似 / 3 策略自相似 / 4 涌现⟺Qual / 5 全定义 | 条1 `global_product_complete_unique`；条2 `classify_equivariant`；条3 `piTheta_equivariant`；条5 `main_theorem`/`theorem_final`（**条4 涌现⟺Qual NOT-FOUND**；**五条件未捆绑为单一定理/结构**） | **NOT-FOUND**（无 五条件 construct；`risk.rs:692 risk_mode_five_states` 无关） | **部分（条4 遗漏；未捆绑）** |
| 七链 | ℭ_Θ 七链对照 | 8 节点 7 环，唯一性逐环传递 | `SevenLinkStrategy`:`ThetaStrategy`、`piTheta`、`link3/5/6/7`、`seven_link_exists_unique`、`central_theorem`（**conditional**，依赖 `ThetaStrategy S` 字段） | 端到端管线注释逐环标注：环1 `types.rs:188`、环5 `strategy/mod.rs:45`、环6 `coverage.rs:690`（**无单一 SevenChain struct**） | **部分（管线有，无单一对象）** |

### § 覆盖率统计（按 20 节 + 七链 = 21 行）

- **完整**：11 条 —— §5, §6, §7, §8, §9, §10, §11, §12, §13, §14, §15（七链核心环全部双层完整且 Lean 真证）
- **部分**：10 条 —— §1, §2, §3, §4, §16, §17, §18, §19, §20, 七链
- **整条遗漏**：0 条（无任何一节完全无实装）
- **但部分项内含核心子项遗漏 2 个**（见下 §2）：①Qual_* 算子（Lean 全缺）②§20 条件 4 涌现⟺Qual 定理（无法形式化，连带缺）

---

## 2. 遗漏清单（分层 + 严重度）

严重度：**核心**=触及中心定理/五条件验收项；**中**=支撑核心的结构层；**边缘**=表示差异/proof 层归属/L0 同义反复。

### 2A. Lean 遗漏

| # | 遗漏项 | spec 出处 | 严重度 | 说明 |
|---|--------|----------|--------|------|
| L1 | **Qual_* 提升谓词完全不存在** | §2-3, §20 条件4 | **核心** | 全树 grep 无 `Qual`。涌现判据 ℓmax↑⟺Qual_*=1 无载体 → §20 条件 4 无法形式化。 |
| L2 | **具体 Prom_ℓ 算子 + 𝒩Prom=Prom_* 定理缺** | §3, §18 步1, §16 假设2 | **核心** | Prom 仅抽象 `Bool` 参 + `PromEquivariant` 谓词（作 `hProm` 前提）。§18 步1 由 `hProm` 携带而非兑现；§16 假设 2「Prom 全定义且单值」是 12 假设中**唯一未被下游引理 discharge 的**（其余 11 条各环 ∃! 已证）。**注**：诊断曾担心的"Prom 以 True 假设承载"已修正 —— §18 现用 `PromEquivariant` Prop（非 `True`），但 **Prom 本体算子仍未构造**，等变性无从 discharge。 |
| L3 | **ℓ_max 动态涌现律未形式化** | §1, §18 条件4 | **中** | ℓmax 仅 `CandidateSet.State` 字段；`ℓmax(t+1)=ℓmax(t)+1` 与 `supp(D_t)=[ℓmin,ℓmax]` 无定理。级别索引用 **Nat 非 ℤ**（去根化 𝕃=ℤ 表示偏差）。 |
| L4 | **Germ 状态 / ∂_{ℓ+1} 父容器对象缺** | §4 | **中** | 无 `Germ` 归纳、无 ∂ 作 C_{ℓ+1} 普通值；germ-hood 编码为 `parent=none`。**张力**：spec §4 下游推论明确要求「Lean 父容器是全函数 p，无 Option 包裹的根例外」，而现编码恰用 `Option=none` —— 表示上**反向**引入了 spec 想消除的根特例（语义经 `classifyV_germ_ambient` 兜住，但与去根化范式存在表示矛盾）。[需确认] 是否上 escalate。 |
| L5 | **§20 五条件未捆绑为单一验收定理；条4 缺** | §20 | **核心** | 条 1/2/3/5 各有定理（散落 4 文件），条 4（涌现⟺Qual）缺（依赖 L1）。无 `five_conditions` 顶层合取定理作验收门。 |
| L6 | **§17 九元组 ℭ_Θ 未组装为单一对象 + 与七链对齐缺** | §17 | 边缘 | `GlobalProductClassification` 是 3-合取父子一致性结构，**≠字面九元组**；九元组管线散在 `classify_equivariant`+`SevenLinkStrategy`。三者未统一为一个 `ℭ_Θ` classifier 供七链引用。（spec 自标 §17 完全分类为 L0 同义反复，故边缘。） |
| L7 | **§18 步2（b 等变）未单独证，仍作 hBsp 假设** | §18 步2 | 边缘 | `signalVector` 已构造真证（64 类），其平移等变 `BspEquivariant` **理应可 discharge**，但 `classify_equivariant` 仍作前提 `hBsp` 携带，未见 `bsp_shift_invariant` 引理。[需确认] 是否存在该引理。 |

### 2B. Rust 遗漏

| # | 遗漏项 | spec 出处 | 严重度 | 说明 |
|---|--------|----------|--------|------|
| R1 | **统一递归单元 5 元组 C_ℓ=(S,Γ,A,P,E) 无结构体；𝒩_ℓ/F_* 缺** | §2 | 边缘 | `RMove::Compose` 仅 3 字段(`subs,centers,level`)；无归一化算子 `𝒩`、无统一生成算子 `F_*`。机制经 RMove/LeveledMove 部分承载。 |
| R2 | **九元组 ℭ_Θ struct 缺** | §17 | 边缘 | `Classification` 是 per-x 输出，非九元组聚合。 |
| R3 | **§18 八步等变 / §20 五条件 无 construct** | §18, §20 | 边缘 | proof/验收层，rust 非证明器；属正确分层（证明归 Lean）。 |
| R4 | **§16 ∃! 仅断言确定性非证明** | §16 | 边缘 | `step_spec_total_unique_per_state` 测 per-state 确定性；∃! 定理在 Lean。正确分层。 |

### 2C. 双遗漏（Lean + rust 均无对应单一对象）

| # | 遗漏项 | spec 出处 | 严重度 | 说明 |
|---|--------|----------|--------|------|
| D1 | **九元组 ℭ_Θ 作为单一对象** | §17 | 边缘 | Lean=3-合取（≠九元组）；rust=per-x Classification。两层均未把 (D,(b),(χ),Γ,(R),ℛ_Θ,A,p̃) 组装为一个命名对象。 |
| D2 | **§20 五条件统一验收捆绑** | §20 | **中** | 两层均无「五条件合取」单一验收门；Lean 条 1-3,5 散落、条 4 缺，rust 全缺。 |
| D3 | **Qual_* 作为命名谓词** | §2-3 | **核心** | Lean 全缺；rust 仅 `min_parts_per_level=3` 机制（未抽象为 Qual 谓词）。提升判据未在任一层成为一等公民。 |

---

## 3. 任务重点问题逐条回答

**Q1 §1–§5 Prom_*/ℓmax 涌现 有 Lean+rust 吗？**
- **Prom**：Lean = 抽象 Bool 参 + `PromEquivariant` 谓词（前提 `hProm`），**无具体算子/无 𝒩Prom=Prom_* 定理**；rust = `compose_move`/`compose_level`（机制有，未命名）。**Qual**：Lean **全缺**；rust = `min_parts_per_level=3`（机制）。
- **ℓmax 涌现**：Lean = 字段，**无涌现定理**；rust = `l_max` 配置 + 递归（机制）。
- **结论**：诊断担忧坐实一半 —— §18 已不用 `True`（改 `PromEquivariant` Prop ✓），但 **Prom 本体算子 + Qual 谓词仍未形式化**，等变性只能作假设携带。**这是全 spec 唯一的概念层真遗漏（核心）。**

**Q2 §16 中心定理 12 假设逐条对应？**
- 12 假设捆绑为 `FinalTheoremHypotheses`/`MainTheoremClosure` **structure 字段（premise，非 axiom）**。其中 **11 条被各环 ∃! 引理 discharge**（`gamma_finite`、`rTheta_exists_unique`、`role18_count_one`、`lexArgmin_exists_unique`、`feasible_nonempty` 等）；**唯一未兑现 = 假设 2「Prom_* 全定义且单值」**（因 Prom 未构造，见 L2）。定理本身 conditional 真证。

**Q3 §17 九元组 ℭ_Θ 与七链对齐？**
- **未对齐**。`GlobalProductClassification` 是 3-合取父子一致性结构（提供完全性/唯一性），**不是**九元组 ℭ_Θ。九元组管线实体散在 `SelfSimilarity.classify_equivariant`（八步）+`SevenLinkStrategy`（链）。**无单一 ℭ_Θ 对象统一三者供七链引用**（L6/D1）。

**Q4 §18 八步等变逐步齐否？Prom/b 步底层等变是真证还是假设？**
- **6/8 真证**：步 3-8（N/H/V/R/≺/AncOK/p̃）由 `N_shift_invariant`、`H_V_R_shift_invariant`、`candLe_shift'`、`AncOK_shift_equivariant`、`ptilde_shift_equivariant` **真兑现**。
- **2/8 假设**：步 1（Prom）= `hProm:PromEquivariant`，**底层无法 discharge**（Prom 未构造，L2）；步 2（b）= `hBsp:BspEquivariant`，**理应可 discharge 但未**（signalVector 已证存在，缺 `bsp_shift_invariant` 引理，L7，[需确认]）。
- `classify_equivariant` 整体 = conditional 真证（前提 Prop，非 axiom/非 True）。

**Q5 §19/§20 π_Θ 等变 + 五条件齐否？**
- π_Θ 等变：`piTheta_equivariant`/`_strong` conditional 真证 ✓（依赖 `CovariantCapital`）。
- 五条件：条 1/2/3/5 有（散落）；**条 4（涌现⟺Qual）缺**（L1 连带）；**五条件未捆绑为单一验收定理**（D2）。

**Q6 疑点清单 10 条现状**（见下 §4）。

---

## 4. 疑点清单 10 条现状

| # | 疑点（spec line 1254-1266） | 现状 | 证据/说明 |
|---|------|------|----------|
| 1 | (χ_ℓ^+,χ_ℓ^-) 记号 | **已解** | =区间套确认 χ：Lean `IntervalNestCertificate.Chi`/`chiBool`；rust `nest.rs:114 Chi`、`strategy/nest.rs:122 chi_bool`。即 N^δ 确认的上下界对。 |
| 2 | Cand^δ_ℓ 定义 | **部分** | Lean `CandidateSet.gamma`/`mem_gamma_iff` + `IntervalNestCertificate` 用 Cand 作候选成员；rust `interp.rs:67 Candidate`/`:128 assemble_gamma`。作候选集成员谓词存在，但**独立闭式定义** [需确认]。 |
| 3 | 提升 m≥3 下界 | **已解（rust 硬编码）** | rust `config.rs:76 min_parts_per_level=3`，`classifier/mod.rs:208 units.len()<min_parts` 强制；Lean 端 [需确认 `allCands ℓmax` 是否含 m≥3 约束]。 |
| 4 | AncOK 单次 vs 迭代闭合 | **已解（迭代）** | Lean `AncestorClosure.ancestorClose` + `ancestorClose_anc_closed`（迭代至祖先闭合）；`ActiveSet.activeNext_anc_closed` 证闭合成立。rust `coverage.rs:362 ancestor_close`。 |
| 5 | 18 类经验可达性（L2） | **未解（L0 only）** | Lean `decide` 证 L0 完全分类（Σ=1）；**无 L2 真实数据可达性证据**。按 formalization-validity-domain 规则：L0 完全 ≠ 有效域=定义域。哪些 c∈im ℭ_Θ 实际出现，PDF/实装均未给。**诚实标：L0 已证，L2 未验证。** |
| 6 | 资本约束 vs 策略自相似张力 | **已解（构造性消解）** | `CovariantCapital.lean`:`feasibleSet_equivariant_strong` —— 通过令资本**协变**（按级别归一化）使 J 平移等变，`piTheta_equivariant` 依赖之。即 spec §16 边界条件标注的"绝对资本破坏自相似"被"资本协变"前提消解。**无需 escalate（已有构造）**，但前提 `Jinv` 仍须实装侧保证（资本不可用绝对值）。 |
| 7 | 纯 core 连续 LexArgmin 离散化 | **已解（List 离散）** | Lean `LexArgmin.lexArgmin_exists_unique` 在 **List 上**字典序最小元（离散）；rust `intent.rs:270 lex_argmin`。连续优化已离散为有限候选列表。 |
| 8 | 七链谱系 T₁-T₅₉ 编号映射 | **未解** | 本审计未对照 `.rtas`/谱系记录；§16/§18/§20 各定理与 T 编号精确对应 [需确认 调取谱系]。 |
| 9 | vs 15 页版逐条坐实 | **未解** | 需 15 页版原文比对；本审计仅核 20 页版 vs 实装，未取 15 页版。 |
| 10 | ν(g) 声部索引函数 | **部分** | rust `ledger/separate.rs` Leg 用 ν 索引、`coverage.rs:608 leg_target`；Lean `ActiveSet.legOf`。映射存在，**独立 ν 定义式** [需确认]。 |

**疑点小结**：已解 5（#1,3,4,6,7）、部分 3（#2,10 + #5 之 L0 部分）、未解 3（#5 之 L2,#8,#9）。其中 **#5（18 类 L2 可达性）是认识论硬缺口**（L0≠L2，不得声称已验证）；#8/#9 是谱系/对照工作（非实装缺口）。

---

## 5. 总判定

**spec 是否完整实装？—— 结构层近乎完整，概念层有 1 个核心遗漏。**

- **七链核心（§5–§15，11 节）+ 中心定理/自相似/策略自相似（§16/§18/§19/§20 条1-3,5/七链）：完整或 conditional 真证，Lean 零 sorry/admit/axiom。** 这是验收门的主体，**通过**。
- **唯一概念层真遗漏 = 提升层 Prom_*/Qual_***（L1/L2/D3，核心）：Qual 全缺、Prom 本体算子未构造，连带 §20 条件 4（涌现⟺Qual）无法形式化、§16 假设 2 未 discharge、§18 步 1 只能作假设。**这是验收门的唯一红线项。**
- **次要缺口**：ℓmax 涌现律（L3）、Germ/∂ 表示与去根化范式的张力（L4）、五条件未捆绑（D2/L5）、九元组未组装（L6/D1）—— 中/边缘，不阻塞主体。
- **认识论诚实声明**：本矩阵 L0/L1（结构存在性，未跑 build）。Lean "零 axiom/sorry" 由静态 decl-grep 坐实，**编译性未由本审计验证**。18 类完全分类是 L0，**L2 经验可达性未验证**（疑点 #5）。

**验收结论**：**有条件通过。** 主体（七链 + 唯一性 + 自相似）完整且真证；**放行前须补 1 项核心**：Prom_ℓ 算子 + Qual_* 谓词的 Lean 形式化（连带 §20 条件 4、§16 假设 2 的 discharge、§18 步 1 的兑现）。其余为中/边缘改进项。

---

## 结果包（六要素）

- **结论**：20 页 spec 逐条核对 = 完整 11 / 部分 10 / 整条遗漏 0；唯一核心概念遗漏 = 提升层 Prom_*/Qual_*（Lean 全缺算子、Qual 不存在），连带 §20 条件 4 不可形式化、§16 假设 2 未兑现、§18 步 1 作假设。结构主体（§5-§15 七链 + §16/§18/§19/§20 大部）conditional 真证，Lean 全树零 sorry/admit/axiom。验收**有条件通过**。
- **定义依据**：依 spec §3（Prom_ℓ/Qual_* 方框 line 129-151）、§18（八步等变 line 900-942）、§20 条件 4（涌现⟺提升谓词 line 1114-1116）；输入特征 = Lean `SelfSimilarity.classify_equivariant` 步 1 由 `hProm:PromEquivariant` 前提携带 + 全树无 `Qual` decl + 无具体 `Prom_ℓ` 算子，满足"提升判据未形式化"的遗漏条件。
- **边界条件**：本判定在"未跑 `lake build`/`cargo build`"（L0/L1）下成立。若 build 失败则 Lean 真证性翻转（须降级）；若后续补出 `Prom_ℓ` 算子 + `Qual` 谓词 + `bsp_shift_invariant` 引理，则核心遗漏消解、§18 八步全兑现、验收升为"无条件通过"。若 §4 Germ 的 `parent=none` 编码被判违反去根化（spec §4 下游推论"无 Option 根例外"），则 L4 升级为概念矛盾须 escalate。
- **下游推论**：放行 Prom/Qual 工位为最高优先级 Lean 缺口（其余环已闭合，仅此环以假设承载）；补出后 §16 12 假设可全 discharge、§20 五条件可捆绑为单一验收定理。rust 侧 Prom 机制（`min_parts_per_level=3`）已就位，Lean 补形式化后两层对齐。
- **谱系引用**：直接命中 MEMORY `b2s2-still-missing-tower`（RMove::Compose 塔/提升缺口）与 `theta-v0-tower-3seg-window`（≥3 段提升）——Prom/Qual 正是提升塔的形式化对象；`escalate-requires-l2-evidence`（本审计 L0/L1，未越级声称 L2，疑点 #5 守住 L0≠L2）；`v1-fullwindow-l3-falsified`（结构完整 ≠ 经验有效，18 类 L2 可达性仍开放）。[需确认] §16/§18/§20 与 T-Lean T₁-T₅₉ 编号映射（疑点 #8）。
- **影响声明**：本产出为只读审计，未改任何代码/定理。新增 1 个矩阵文档 `.chanlun/specs/2026-06-28-spec-implementation-coverage-matrix.md`。标定 Lean 唯一核心缺口（Prom/Qual）+ 3 个中度缺口（ℓmax 涌现、Germ 表示张力、五条件未捆绑）+ 边缘缺口，供 Lead 决定是否放行 Prom/Qual 形式化工位。
