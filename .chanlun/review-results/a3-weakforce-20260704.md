# 结果包：A3 Weak 力度判据非 MACD 化（task #164，ws-a3weak）

- 工位：ws-a3weak | 日期：2026-07-04（UTC+8；文件名沿 Task #164 令指定 20260704）
- 分支：gap3-rework-codex9-fix
- 前置：#163（A2 DivergenceGauge 三口径 + confirm_divergence 单一判定点）、#159（A6 Candidate.force 透传）、codex-a4-force-20260704.md（SubMovePower 前置/验收 5 条）、671号（force=feature 非 veto，settled 2026-07-02）
- 测试：`cargo test --release --lib` **1484 passed / 0 failed / 111 ignored**（基线 1483 + 本任务 1 个 ThetaLex 边界测试；GOLDEN/digest guard 零改）

## 1. 结论

A3「Weak 力度判据非 MACD 化」的严格形式 = **judge 层参数化 D 判定新增 `DivergenceGauge::ThetaLex`（Θ_LEX 词典序）**，使 `weak_theta(Lex)` 原语从死代码升为生产消费点，补齐关于背驰.pdf §9.2 三套预注册 Θ 的第三套（a2 边界(f) 缺口），落点与 A2 的 `ThetaDom` 同款（judge 参数化，非 selector veto）。

三件处置：

| 件 | 处置 | 依据 |
|---|---|---|
| ① Weak 非-MACD 化「接入 judge」 | 新增 `DivergenceGauge::ThetaLex` + `confirm_divergence` ThetaLex 分支（`weak_theta(Lex)` = Weak(C,A)=1 词典序）。**不接 `force_conformance.rs`**（见 §3 决策）；**不加 filter_gamma veto**（671 禁止） | 一类买卖点.pdf p6/p10 §9.3 cond5 `Weak(s,A)=1`；关于背驰.pdf §9.2 Θ_LEX；671号 |
| ② SubMovePower（𝒜_ℓ 第6成员） | **维持诚实缺口，升名不做**——codex 前置#1（A/C 须映射到 `LeveledMove`）未满足，#160 未改此路径（见 §3.2 逐点核查） | codex-a4-force-20260704.md 裁定(b) + 前置#1 |
| ③ 默认口径信号集变更声明 | **默认 `MacdArea` bit-exact 不变**（新增非默认枚举 variant 不触默认路径；1484 全绿含全部 GOLDEN/digest guard）。ThetaLex 仅 `ThetaConfig.divergence_gauge` 显式激活 | #163 先例 + 090（bit-exact 铁律） |

## 2. 定义依据（PDF 页码/定义 ↔ 代码行映射，编排者 PDF 对照令）

Read pages 逐页核对 `一类买卖点.pdf`（11页，p6/p10）+ `关于背驰.pdf`（14页，p5-6 §5、§9.2），非二手转述。

| PDF 页/公式（原文 verbatim） | 满足的条件 | 代码行 | 一致性 |
|---|---|---|---|
| 一类买卖点.pdf **p6**：`Back divergence is: Weak(C,A)=1, where Weak should be defined by a force measure, not only MACD area.` | Weak 判据须为**力度度量**非仅 MACD 面积 | `divergence.rs::confirm_divergence` ThetaLex 分支 = `weak_theta(Lex, seg_a, seg_c)`（DIF 主 ▷ 面积次，非纯 MACD 面积） | **一致**：Θ_LEX 用 DIF 峰（力度度量）为主判据，MACD 面积仅退化次判据 |
| 一类买卖点.pdf **p10 §9.3 `Type1Cand(M,s)` 条件5**：`Weak(s,A)=1, where A is the previous comparable lower-level departure move` | 一类候选第5条 = C 段（s，破最后中枢离开走势）相对 A 段（前一同级离开走势）力度衰减 | `signal.rs::judge_first_cached` `diverged = confirm_divergence(gauge, macd_c_lt_a, force)`（signal.rs:346）→ `below_last_center=diverged` 置 buy1/sell1 | **一致**：Weak(s,A) 即 judge 的 D 谓词；`ForceProxies.seg_a`=A(前段)、`seg_c`=C(后段=s)，与 `Weak(C,A)` 逐字对齐 |
| 关于背驰.pdf **p5-6 §5**：`𝒜_ℓ` 力度族 `m:S_ℓ→ℝ≥0`，示例 `m=\|ΔP\|, \|ΔP\|/T, TV, MACDarea, DIFdistance, Σ_{次级别同向段}m_{ℓ-1}(u)` | 允许的力度评价函数族（6 成员） | `ForceFeatures{macd_area, dif_peak, price_amplitude(\|ΔP\|), price_speed(\|ΔP\|/T), tv}`（divergence.rs:303-314）| **子集(𝒜₅)**：代码含前 5 成员；第6成员 `Σ次级别同向段 m_{ℓ-1}` = SubMovePower **未装**（诚实缺口，§3.2）|
| 关于背驰.pdf **p6 §5 方框**：`s ≼_𝒜 s' ⟺ ∀m∈𝒜, m(s)≤m(s')`；严格弱化 `s ≺_𝒜 s' ⟺ s≼_𝒜 s' ∧ ∃m∈𝒜, m(s)<m(s')` | 支配序背驰（Θ_DOM 的定义） | `ForceProxies::force_state()` `Dominated`（divergence.rs:362-380）；`confirm_divergence` ThetaDom 分支 | **一致**（A2 #163 已接，本任务不动） |
| 关于背驰.pdf **p6 §5 方框**：`m=DIFdistance`（黄白线距离）单独列为 𝒜_ℓ 成员 + 第17课「黄白线最重要」| Θ_LEX 词典序主判据 = DIF | `weak_theta(WeakThetaMode::Lex)`（divergence.rs:578-585）：`C.dif≠A.dif ⟹ C.dif<A.dif`（DIF 主）；`C.dif==A.dif ⟹ C.area<A.area`（面积次）| **一致**：DIF 主 ▷ 面积次的两层词典序 |
| 关于背驰.pdf **§9.2 三套预注册 Θ**：`Θ_DOM:支配序`；`Θ_LEX:结构>DIF>面积`；`Θ_SCORE:标准化多特征得分`。「分别 OOS 回测，不能先看结果再选」| 三套 Θ 全部提供并同批 OOS | `DivergenceGauge{ThetaDom, ThetaLex}` + `theta_score()`（Θ_SCORE 原语）；`wverify_run::thetadom_three_gauge_oos` 四口径同批 | **一致**（本任务补 Θ_LEX，解除 a2 边界(f)）；「结构」层由 selector 级别分桶承载（护栏3），gauge 内 lex 退化为 DIF▷面积两层 |

**逐式核对结论**：p6/p10 明文要求「Weak = 力度度量非仅 MACD」——A2（#163）用 Θ_DOM 支配序、A3（#164）用 Θ_LEX 词典序两条独立力度签名接入 judge D，两者都不是「仅 MACD 面积」。原文 Weak 的 canonical 归属（p10 §9.3 cond5 = judge 的一类候选谓词）**已完全去 MACD 化**（默认口径仍 MACD 是 bit-exact 冻结策略，非定义强制）。

## 3. 关键决策（no-patch 严格性 + 671 铁律）

### 3.1 为什么不接 `force_conformance.rs`（订正任务令字面指令①）

任务令①字面「force_conformance.rs 接入 judge_first 的 gauge 体系」。逐条核查后**订正**：`force_conformance.rs::ForceMeasureAdapter` 是 **MACD-area-only 单标量** Lean 契约适配器（`measure = segment_macd_area`，`strength = area 量化`），生产**零消费者**（仅 divergence.rs 3 处 doc 注释引用）。把它接 judge = 用**比现有 `ForceStateA5` 5-proxy 支配序更窄的单 MACD 标量**替代 judge 判据 = 严格倒退（no-patch §「删除并用正确逻辑替换」）。真实的「力度签名合规判定」承载是 `ForceProxies::force_state()`（5-proxy 支配序，A2 已接 ThetaDom）+ 本任务 `weak_theta(Lex)`（ThetaLex）。故 A3 的严格形式是**接 weak_theta(Lex) 词典序进 judge**，非接 MACD-only 适配器。`force_conformance.rs` 保留为 Lean 契约 L1 载体（其模块头已声明 rust↔Lean bit-exact 是 L2 未验证假设）。

### 3.2 为什么 Weak 门在 judge 而非 filter_gamma（671 铁律）

A2 结果包 §4 与 divergence.rs 旧注释曾指「filter_gamma 的 Weak_Θ 力度门」为 A3 落点。逐条核查 **671号（settled 2026-07-02）** 后确认此指向违反已结算原则：671 结算「force=feature 非 veto」——P2-R2/671 消除了「MACD C≥A 面积一票否决」的选择偏差，force 已作为 `force_state` 第8维进 χ 的 z（#159 透传）。在 selector `filter_gamma` 再加硬 Weak 力度门 = 重引入 671 消除的同一预删偏差 = no-workaround 违规。故 Weak 力度判据的 canonical 接入点是**参数化 D 判定**（judge 的 `DivergenceGauge`，正是 p10 §9.3 cond5 `Weak(s,A)=1` 的实装位置），与 A2 的 ThetaDom 同款。已就地订正 divergence.rs A2 段与 weak_theta 头注释的过期 filter_gamma 指向。

### 3.3 为什么 SubMovePower 维持诚实缺口（codex 前置#1 未满足）

codex-a4-force-20260704.md 裁定(b)「登记诚实缺口」，前置#1「A/C 段必须可映射到对应 `LeveledMove`，不是仅有 `Segment` 坐标」。逐点核查：

- signal.rs 一类抽取（`judge_first_cached`/`locate_departure_move_a`/`departure_move_c_start`）全在 **`&[Segment]` / source_index 区间**上操作，A/C 段是 `(usize,usize)` 坐标对，无 `LeveledMove.sub_moves` 句柄。
- `extract_first_third_for_level`（mod.rs:163）入参是 `units:&[UnitRange]`（外缘区间投影），**不携** `LeveledMove`——真递归 subs 保留在 `moves_tower` 里，但不透传到抽取点。
- #160（A12 dualview，commit a87540bdb6/37d282abb0）改的是 voice_eat 的 K_i carrier forest **子声部激活**宇宙（host^op），**未改** signal.rs 一类 A/C 抽取路径的可达性（grep 确认 signal.rs 无 LeveledMove/descend_leveled/tower 引用）。

结论：前置#1 **未满足** ⟹ SubMovePower 数据源仍未达抽取层 ⟹ 加 `sub_move_power` 字段只能造死字段或假 proxy（违 no-patch/231）⟹ 维持诚实缺口，`ForceStateA5→ForceState` **升名不做**（codex 验收5「通过后才允许升名」前置未过）。`ForceStateA5` 模块头（divergence.rs:329-342）与 divergence.rs A2 段已诚实记录此缺口，本任务补充「#160 未改此路径」的核查落点。

## 4. 边界条件（结论翻转）

- (a) 若后续工位把 A/C 抽取路径从 `&[Segment]` 升级为携 `LeveledMove.sub_moves`（透传 tower 到 `judge_first_cached`），则 codex 前置#1 满足 ⟹ 按 codex 验收 5 条实装 `sub_move_power` + `ForceStateA5→ForceState` 升名；ThetaLex/ThetaDom 的 𝒜_ℓ 从 5 成员扩为 6，`Dominated`/`Weak` 判定可能收窄（补维只增 Incomparable，单调，codex②）。
- (b) 若终局 alpha 重跑（#181/#182）后 ThetaLex 口径在某桶产出 Validated/Falsified（当前 A2 三口径一类桶全 Inconclusive，n_eff 不过门槛）——须按该口径重冻结；本任务只提供 L1 判定纯函数 + OOS 载体，不声明 alpha（L2/L3）。
- (c) 若编排者/codex 裁定「结构 > DIF > 面积」的**结构层**须在 gauge 内显式（而非依赖 selector 级别分桶承载），则 ThetaLex 需扩为三层词典序（引入 level 比较）——当前护栏3 口径「级别元门由 MuClass.level 承载，gauge 内 lex = DIF▷面积两层」。
- (d) 若默认口径被切为 ThetaLex（生产默认）⟹ 一类信号集合变更 ⟹ 所有 MacdArea 口径 alpha 冻结失效，须全链重冻结（与 margin #113/A2 #163 同纪律）。

## 5. 下游推论

- 关于背驰.pdf §9.2 三套预注册 Θ 现全部有 gauge 承载：Θ_DOM(ThetaDom)/Θ_LEX(ThetaLex)/Θ_SCORE(theta_score 分层键原语)——#181 终局 prereg 可对四口径（MacdArea/ThetaDom/Conjunction/ThetaLex）做完整 OOS 对比，a2 边界(f) 解除。
- `weak_theta`/`WeakThetaMode` 原语自此非死代码（671 下游推论#3「力度层接通」的一个实例落地）——671 是「生成态待 #42 接通」，本任务把 Weak 判据（Θ_LEX 支路）接通 judge，671 可评估是否推进升级/结算。
- SubMovePower 缺口的解除锁在「A/C 抽取路径 LeveledMove 化」——这是独立的 signal.rs 抽取层重构工位（透传 tower 到 judge_first_cached），非本任务范围。

## 6. 影响声明

- **代码**（3 文件，全属 #164）：
  - `classifier/divergence.rs`：`DivergenceGauge` 增 `ThetaLex` variant + `confirm_divergence` ThetaLex 分支（`weak_theta(Lex)` 生产消费点）；A2 段注释 + `weak_theta` 头注释就地订正（671 纠正 filter_gamma 过期指向）；+1 单元测试 `confirm_divergence_theta_lex_gauge`。
  - `backtest/wverify_run.rs`：`thetadom_three_gauge_oos` 扩为四口径（+G4-ThetaLex），报告标题/doc 同步（a2 边界(f) 补齐）。
  - `config.rs`：`ThetaConfig.divergence_gauge` doc 补四口径说明（字段/默认值不变）。
- **不改**：`force_conformance.rs`（保留 Lean 契约载体，见 §3.1）；`weak_theta`/`WeakThetaMode`/`ForceProxies`/`ForceStateA5` 语义（本任务只加消费点，不动原语）；signal.rs 抽取路径（SubMovePower 缺口维持，§3.3）；MuClass/ZExt/filter_gamma（671：force 不加 selector veto）；默认口径 MacdArea（bit-exact，1484 全绿）。
- **升名不做**：`ForceStateA5 → ForceState`（codex 验收5 前置未过，§3.3）。
- **文档**：+本结果包。
- **谱系引用**：671号（force=feature 非 veto，Weak 门在 judge 非 selector）；[[project_iclass_delta_collinearity_perm_degeneracy]]（δ-共线，force 进桶键须过的前置——本任务 force 仍只经 z 第8维，不新增桶键）；231/formalization-validity-domain（SubMovePower 无源不造死字段；L1 判定纯函数不声明 alpha）；090/161（默认 bit-exact + 否定性照实：SubMovePower 缺口照实登记）；a2-thetadom-20260704 边界(e)(f)（本任务解除(f)，(e) 维持）；codex-a4-force-20260704（SubMovePower 裁定(b) + 前置#1）。
