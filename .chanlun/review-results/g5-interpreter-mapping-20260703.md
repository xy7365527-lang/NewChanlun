# G5 解释器统一 P1..P10——映射表 + 撞界分析 + 真统一测试场景清单

- **工位**：ws-g5interp（Task #124，goal g-20260703T0300Z-full-spec-pi q2 缺口 G5）
- **日期**：2026-07-03
- **规格源**：`docs/formal-chain/完整的策略.pdf` §7（P1..P10 固定优先级互斥化）+ §16（π^full 单解释器 I_Θ 发结构桶 + TWEvent）
- **对照基线**：main BTC struct baseline（memory: T引擎BTC权威基线；bit-exact 现状）
- **状态**：分析定案（步骤1-3）；实装 blockedBy #122（typed-exit / K_Θ gross / P1·TW-into-fold 四裁定一并出）
- **下游消费**：ws-q3proof（#125 q3 证明文档引用本映射表）；#124 实装工位（#122 收口后据此续做）

---

## 1. PDF §7 P1..P10 → 当前实装映射

| PDF §7 谓词 | 语义 | 当前实装位置 | 状态 |
|---|---|---|---|
| P1 = 强平/保证金风险 | force_flat | `runner::k_theta_risk_gate` → `coverage::KThetaRiskGate.force_flat`，作 K_Θ={0} downstream 收窄，**非 fold 内谓词** | 存在，解释器外（Q2「close_pred 折 𝒦_Θ，非第二决策出口」） |
| P2 = TW StageII ∧ H>0, CloseOverlay | 关重叠腿（发订单） | **无** | 缺——真统一=新增 close 订单 |
| P3 = TW StageII 可 Withdraw | 退本金（无订单） | `closed_loop/transition::stage_progression` → `RecoverCapital`（free→withdrawn） | 存在，disjoint closed_loop 路径 |
| P4 = TW EnterEarning | 相变增股（无订单） | `stage_progression` → `EnterEarning`（CostReduction→…→EarningShares 单向相变） | 存在，disjoint 路径（memory: EarningShares 结构不可达） |
| P5 = 正规 CloseRoot | 一类根清仓 | `interp::interpret` `buckets.close`（interp.rs:977-982，reverse_signal 反向关腿）**未 typed 区分** | 单一 close 桶，未拆 |
| P6 = 正规 ReduceCore | 三类核减仓 | 同上 close 桶（未拆 ReduceCore） | 同上 |
| P7 = Close ShortDiff | 关短差子声部 | 同上 close 桶（reverse_signal 关任意反向腿，未按 ShortDiff carrier 区分） | 同上 |
| P8 = Open Root | 开根 | `interp::interpret` `buckets.open`（interp.rs:992-994，role.v=Ambient/root，slot 空） | 存在 |
| P9 = Open ShortDiff | 开短差 | 同上 open 桶（role.v=`Vertical::ShortDiff`，父声部 active） | 存在 |
| P10 = Record StructBreak | 记录不交易 | `interp::interpret` `buckets.record`（interp.rs:964-966/995-996，Flat/无类候选） | 存在 |
| P0 = Hold | 无动作 | 无候选命中 → LexArgmin p* 不变 | 存在（隐式） |

**现 mutex.rs oracle 是 alpha2 §5 的 P1..P8（不同分解）**：alpha2 P2/P3=close_long/close_short（=interp close 桶，未按 typed-exit 拆成 PDF P5/P6/P7）；alpha2 P4/P5/P6=open_short_diff/open_long/open_short（=PDF P8/P9）；alpha2 P7=same_slot_record（=PDF P10）；**alpha2 无 TW 事件（PDF P2/P3/P4）**。故 P1..P8→P1..P10 不是「加两谓词」，是重分解 + 精细化。

---

## 2. 现调用顺序是否隐式等价 PDF 优先级？——部分等价，真统一=语义变更

- **P1（强平）：output-等价，纳入 fold 由 #122 裁定4 重裁。** force_flat 作 K_Θ={0} downstream 收窄，对 force_flat 情形 output 等价于「P1 首位屏蔽 P5..P10」（p*=0）；`no_increase_cap`（Deleverage/CloseOnly）等价于「P1 屏蔽开仓 P8/P9」。但**候选级 fold 内表达 P1** 的桥接设计（codex-q2-d1 §3「需重新裁定」）已并入 #122 ws-codexq2 裁定4。
- **P5/P6/P7（typed close）：现单一 close 桶，未拆。** interp.rs:977-982 只按 reverse_signal 反向关腿，不区分 CloseRoot（一类根清仓）/ReduceCore（三类核减仓）/CloseShortDiff（短差子声部关）。此拆分**消费 #122 裁定2（typed-exit / ResidualTrade P_out 出场口径）**——判据边界未定前无法实装。
- **P2/P3/P4（TW）：与 pi 订单流 disjoint（memory: trades-vs-closedloop-disjoint-paths）。** `run_theta_v0_pi`（pi 生产路径）不消费 TW 也不发 TW 事件；TW 三阶段在 `run_closed_loop`/`ledger.rs`。PDF §16 要求 I_Θ 单解释器**同时**发结构桶 + TWEvent：
  - P3 Withdraw / P4 EnterEarning：纯资本记账/相变，无订单 → 纳入不改订单流。
  - **P2 CloseOverlay：新增 close 订单 → 改 pi 订单流 → 非 bit-exact → 动 BTC 基线**（语义变更）。

**三处语义变更点（q2 验收「留简化/占位即 fail」⟹ 真统一是唯一路线，编排者裁定 BTC GOLDEN 诚实重算 + diff 记录，q4 验收已要求）：**
1. P2 CloseOverlay 进 pi 解释器 = 新增 close 订单谓词（优先级在 P5 CloseRoot 之上）。
2. P3/P4 TW 事件由 pi 解释器发出（I_Θ 输出扩为 (D,O,L,TWEvent)），TW 状态 threaded 进 fold（现 disjoint）。
3. close 桶按 typed-exit 拆成 P5/P6/P7（消费 #122 裁定2）。

---

## 3. 撞界与裁定归属（team-lead 2026-07-03 已裁，全部解除）

| 撞界 | 归属 | 处置 |
|---|---|---|
| P1/TW 投影进候选级 fold（codex-q2-d1 §3「需重新裁定」） | #122 ws-codexq2 裁定4（codex 全权，无需编排者） | 等 #122 收口 |
| P5/P6/P7 typed 拆 + P1 K_Θ gross 投影 | #122 裁定2（typed-exit）+ 裁定3（K_Θ gross） | 等 #122 收口 |
| P2 CloseOverlay 破 bit-exact BTC 基线 = 语义变更 | 定理类（goal q2 验收「留简化即 fail」的必然推论，非新裁断） | 真统一唯一路线；BTC GOLDEN 诚实重算 + 与旧基线 diff（q4 验收要求） |

---

## 4. 真统一 fold 的 shadow_fold P1..P10 property-test 场景清单（预设计，非实装）

现 `mutex.rs::tests::shadow_fold_bucket_equivalence`（S1-S9）覆盖 P1..P8 三桶（close/open/record）。真统一 P1..P10 需扩展的场景（**判据待 #122 裁定2/3/4 定，此处只列覆盖意图**）：

**机器证明层（`mutex_class` 组合逻辑，无数据依赖，可先扩）：**
- [ ] 穷举 2^10 谓词组合断言 `Σ_{j=0}^{10} 1[C_j]=1`（现 2^8 → 2^10，全互斥+全定义机器证明保持）。
- [ ] 优先级屏蔽：P1 成立屏蔽 P2..P10（即使全 true → C_1）；最小成立索引 r → C_r。

**shadow-fold 对拍层（production 状态 → P1..P10 谓词 → 桶，需 #122 定判据后实装）：**
- [ ] P1 force_flat=true + 任意候选 → 全 flat（p*=0）；no_increase_cap（Deleverage/CloseOnly）→ 屏蔽 P8/P9 开仓。
- [ ] P2 CloseOverlay：TW StageII ∧ H>0（同级兄弟重叠）→ 关重叠腿，优先于 P5 CloseRoot。
- [ ] P3 Withdraw：TW StageII withdraw-ready → TWEvent（无订单），优先于 P4。
- [ ] P4 EnterEarning：TW CapitalRecovered ∧ EnterReady → TWEvent（无订单）。
- [ ] P5 CloseRoot：depth-0 根腿遇反向信号 → close（区别于 P6/P7）。
- [ ] P6 ReduceCore：三类反向/减核信号 → reduce（判据待 #122 typed-exit）。
- [ ] P7 CloseShortDiff：ShortDiff carrier 子声部遇反向 → close（判据待 #122）。
- [ ] P8 OpenRoot：Ambient/root 候选，slot 空 → open。
- [ ] P9 OpenShortDiff：ShortDiff 角色候选，父声部 active → open。
- [ ] P10 RecordStructBreak：struct-break（Flat/无类）候选 → record。
- [ ] P0 Hold：无谓词命中 → p* 不变。
- [ ] 跨层屏蔽：P1≻P2..P10；P2（TW close）≻P5..P7（正规 close）；typed-close P5/P6/P7 互不混（依赖 #122 typed-exit 判据）。

---

## 5. 结果包六要素

1. **结论**：G5 统一 as-specified（PDF §7/§16 单解释器 P1..P10）不是 bit-exact 显式化重构，是语义变更（三处，见 §2）。P1 现 output-等价（K_Θ={0} 收窄）、P5..P10 结构在 interp 三桶、P2/P3/P4 TW 与 pi disjoint。真统一唯一路线（q2 验收裁定），实装 blockedBy #122。
2. **定义依据**：PDF §7（P1..P10 固定优先级 `C_j=P_j∧⋀_{k<j}¬P_k`，`Σ1[C_j]=1`）+ §16（I_Θ(A_t,{γ:N=1},TW_t,Risk_t)→(D,O,L,TWEvent)）。输入特征：现 interp.rs:940-999 fold 只产 close/open/record 三桶（未 typed），k_theta_risk_gate 产 force_flat/stop/cap（K_Θ 约束），stage_progression 产 TW 事件（disjoint）。
3. **边界条件（结论翻转条件）**：若 #122 裁定2 判 typed-exit 不拆（close 保单桶）→ P5/P6/P7 退化为单谓词，G5 精细化缩小；若 #122 裁定4 判 P1/TW 不进 fold（保 disjoint）→ 与 §16 单解释器冲突，须再上浮。现 team-lead 裁定真统一，故按「拆 + 进 fold」推进。
4. **下游推论**：ws-q3proof（#125）的 §13 唯一性证明须覆盖扩展后的 P1..P10 `Σ1[C_j]=1`（2^10）+ shadow-fold 桶级等价；BTC GOLDEN 语义变更后重算 + diff（q4 验收）。
5. **谱系引用**：codex-q2-d1-ruling-20260702（D1 oracle 收窄 + §3 P1 留后裁）；codex-q2-conflict-note（裁定A=留 mutex.rs）；full-strategy-spec-conformance-20260703（G5 条 + #92 显式留界）；231号（形式化有效域：L0 结构 vs L2 alpha）；090号（声明膨胀禁止：花名标签冒充精细化 = fail）；trades-vs-closedloop-disjoint-paths（memory）。
6. **影响声明**：本文件纯分析产出（映射表 + 场景清单 + §6 实装设计），未改任何生产代码。#124 保持 in_progress + blockedBy #122。实装阶段（#122 后）将改：`mutex.rs`（P1..P8→P1..P10 predicates_of + 2^10 证明 + shadow-fold 扩展）、`interp.rs`（close 桶 typed 拆 + I_Θ 发 TWEvent）、`coverage.rs`/`runner.rs`（P1/TW 投影进 fold）、受影响 BTC GOLDEN 重算。

---

## 6. P5/P6/P7/P1 实装设计（#122 G4/G7 终裁后，等待期 prep）

依据 `codex-q1-spec-rulings-20260703.md`（G4/G7 已终裁；裁定4 TW-into-fold 催收中）。

### 6.1 P5/P6/P7/P1/P0 ≡ PDF §9 typed exit 枚举（G4 裁定直接给判据）

G4 裁定：PDF §9 出场语义 = `Exit_Θ(v,x_t) ∈ {CloseRoot, ReduceCore, CloseShortDiff, RiskExit, Hold}`。**这五个 typed exit 恰是 G5 的 P5/P6/P7/P1/P0**：

| G5 谓词 | PDF §9 typed exit | 判据（依赖声部级持仓状态） |
|---|---|---|
| P5 CloseRoot | CloseRoot | 关 depth-0 根腿（一类反向点=根清仓） |
| P6 ReduceCore | ReduceCore | 减核心仓（三类反向点=核心仓减仓） |
| P7 CloseShortDiff | CloseShortDiff | 关 ShortDiff carrier 子声部（短差反向确认） |
| P1 强平 | RiskExit | `KThetaRiskGate.force_flat`/stop（保证金/强平） |
| P0 Hold | Hold | 无出场 |

### 6.2 ★关键协调：G4 与 G5 共用同一 typed-exit 枚举（no-patch，090号）

G4 裁定的修复=让生产 π loop 输出 `TypedTradeLedger { entry_z, voice_id, entry_bar, exit_bar, **exit_type**, residual_inputs }`，`exit_type` 就是上表五枚举。**G5 的 close 桶 typed 拆 = 生产 interp::interpret 发出同一 `exit_type`**。⟹ **G4 实装工位与本 G5 工位必须共用一个 typed-exit 枚举**，不能各造一个镜像（否则两权威镜像 = codex-q2-d1 删 mutex_interp 同款矛盾）。建议：interp.rs 定义 `ExitType` 枚举（单源），interp close 桶发它、KThetaRiskGate force_flat 发 `RiskExit`、G4 TypedTradeLedger 消费它、mutex.rs predicates_of 据它投 P1/P5/P6/P7。**须与 G4 impl 工位对齐枚举定义**（谁先落谁定义，后者复用）。

### 6.3 P1 投影口径（G7 裁定）

G7：毛敞口约束在 `coverage.rs:1745` legs 折叠成 net 前用 `gross_target_units(&legs)` 约束（K_Θ 从"标量净可行集"升级为"声部组合可行集"）。P1（RiskExit）在 fold 内的投影 = force_flat 命中时该层直接产 flat 目标。**G7 impl 工位即将动 coverage.rs:1745，本工位续做时 rebase 到其后**（team-lead 已告知）。P1 判据口径以 G7 改后的 K_Θ 可行集语义为准。

### 6.4 裁定4（已落定，codex-q1-spec-rulings-20260703.md）：选(ii)真统一

裁定4 选**(ii) 真统一**，拒(i) 分层 output-等价——**明确判定「只扩 mutex.rs oracle 到 P1..P10、不改真实订单流」为「装饰性 oracle / 留简化占位」**（生产从不触发 P2/P3/P4 分支 ⟹ oracle 对应分支无法对真实生产行为做有意义验证）。⟹ **mutex.rs P1..P10 扩展不可先于生产 typed 接线单独做**（否则正是被拒的装饰路径 + 破坏现working 的 P1..P8 shadow_fold）。P2 CloseOverlay/P3 Withdraw/P4 EnterEarning 必须进**同一** schedule/fill/typed ledger（不走 closed_loop 玩具动作集）。

### 6.5 实装排序（据 G4/G7 顺序 G2→G7→G4 + 裁定4；接口级重构非字段扩维）

1. 等 G7 impl（#133，coverage.rs:1745 毛约束/声部级可行集）落地 → rebase。
2. 与 G4 impl（#134）对齐 `ExitType` 单源（已落 e96bfbff32）→ interp close 桶 typed 拆（P5/P6/P7）+ KThetaRiskGate 发 RiskExit（P1）+ TypedTradeLedger 消费。
3. I_Θ 接口升级（§6.6）：+RiskState/TwState/LegBook 输入、+order_effect/tw_event/exit_kind 输出；P1 最高全局分支（gamma 空也触发）、P2 CloseOverlay 进 schedule/fill、P3/P4 发 TWEvent 并消耗本步裁决。
4. mutex.rs P1..P10 predicates_of + 2^10 证明 + shadow-fold 扩展——**与步骤2/3 原子同落**（predicates_of 投影生产 typed 输出，非先于生产单独扩，避免装饰性 oracle）。
5. 受影响 BTC GOLDEN 重算 + 与旧基线 diff（q4 验收；G7/裁定4 已声明非 bit-exact，语义修正预期数值变化）。

### 6.6 I_Θ 接口 delta（裁定4 给定签名，接口级重构非字段扩维）

裁定4 目标签名：
```
I_Θ(ctx: RiskState + TwState + Active/LegBook, gamma) -> { buckets, order_effect, tw_event, exit_kind }
```
对齐 PDF §16 四元组 `(D_t, O_t, L_t, TWEvent_t)`。

**★分歧1 裁决（team-lead 2026-07-03，补记 ws-codexq2-2）：`interpret(gamma, active) -> Buckets` 签名不变。** I_Θ 是**新组合层函数**（包在 `interpret()` 与 `coverage_step_from_buckets` 之间），不改 `interpret` 本体。理由：现唯一性证明（q3 §A，`coverage.rs:3571`）绑定的对象是 ℛ_Θ(Γ,A)——改 `interpret` 签名 = 破坏已交付证明 + 扩大重证面。故 I_Θ 的 RiskState/TwState/LegBook 输入 + order_effect/tw_event/exit_kind 输出全在**组合层**承载，`interpret` 仍只吃 `gamma`/`active` 产三桶：

| 层 | 签名 | 职责 |
|---|---|---|
| `interpret`（不变） | `(gamma, active) -> Buckets` | ℛ_Θ(Γ,A) 三桶（P5-P10 结构裁决），唯一性证明锚点不动 |
| I_Θ 组合层（新） | `(RiskState, TwState, Active/LegBook, gamma) -> {buckets, order_effect, tw_event, exit_kind}` | P1 全局分支 + P2/P3/P4 TW 投影 + ExitType 产出；内部**调** `interpret` 取三桶 |

关键语义（裁定4 逐项）：
- **P1（强平）= 最高优先级全局分支**，`gamma` 空也触发（账户爆仓无新信号仍须清仓）⟹ 不能实现成"所有候选归 close 桶"。清活动腿 + 目标仓强制 0 + 屏蔽 P2..P10（对应 `C_1=P_1` 屏蔽后续谓词，但触发不依赖候选集非空）。
- **`KThetaRiskGate.force_flat` 降级**为由同一 `RiskState` 派生的执行层安全网/可行集二次校验，**不再是独立第二语义权威**（"解释器 P1 未触发但 force_flat 独立触发" = 设计缺陷，两权威源不同步）。
- **P3/P4** 作 `tw_event` 分量，成立时**消耗本步裁决**（普通候选开/平/记录被屏蔽或推迟 record 桶），非并行各走各。
- **P2 CloseOverlay** 输出真实 typed close 进**同一** schedule/fill/typed ledger（closed_loop 玩具动作集无 CloseOverlay 类型）。
- **`run_closed_loop`** 降级为纯结构验证工具（TW 真值源移到 I_Θ ctx.TwState）。

### 6.7 幽灵腿 bug（P1 实装必堵，已行级核实；补记 ws-codexq2-2 + team-lead 裁决）

**核实**（读 `coverage.rs:1710-1714`）：`coverage_step_from_buckets` 对每个 `c in &buckets.open` 无条件 `raw.push(idx)`（无 `force_flat` 检查，该函数根本不收 gate），随后 `raw` → `ancestor_close_by_id` → `next_idx`（= 下一 bar `prev_active`）。⟹ 强平 bar 上若同 bar 有新候选命中 P8 Open Root，`interpret`（不知 force_flat）正常放行进 open 桶 → 该腿进 `next_active`，即使当 bar 实际订单被 clamp 成 Close/Hold ⟹ **从未真实建仓的"幽灵腿"跨 bar 传播**（下一 bar 参与关闭匹配 / 仓位计算 / AncOK 判定）。

**约束**：P1 全局分支实装时，`force_flat` 成立 ⟹ **open 桶不得进 `next_active`**，且须对 `prev_active` 逐条产 `ExitType::RiskExit` 清空活动腿（不能只跳过 open——旧腿也须清）。这落在 I_Θ 组合层（P1 分支短路 `coverage_step_from_buckets` 的 raw 构造），非改 `interpret`。

**现状影响**：force_flat 现仅 Insolvent/Liquidation（equity≤0）触发，当前 BTC 回测近乎不触发（M2/M3 不可达）⟹ 幽灵腿潜伏未污染现基线 ⟹ 修复随 P1 impl（post-G7）落 + GOLDEN 重算，不单独提前改（避免计划外 bit-exact 破坏）。

**★P1 复用 G7 已建的幽灵腿防护机制**（读 coverage.rs 6a282f5f46 核实）：G7 补充落地已建正解——`apply_gross_cap` 返回被零化腿 `e_idx: Vec<usize>`（`gross_zeroed`），`coverage_step_from_buckets` 建 `next_active` 时 `.filter(|&i| !(open_idx.contains(&i) && gross_zeroed.contains(&i)))` 排除「被零化 ∩ open 候选段」。**代码注释已显式标「幽灵腿防护（裁定4，#124 force_flat 同款模式禁止复制）」**——即 P1 force_flat 必须**复用**此 next_active 后置过滤（非 raw 层另起短路、非重造）：force_flat ⟹ 全 open 候选段 e_idx 进 rejected 集 → 同一 filter 排除出 next_active。
- **open 腿**：与 gross 零化**同款**——rejected open 不入 next_active（reuse）。
- **held 腿**：与 gross **不同**（G7 注释：gross 零化 held 保账本身份，减仓经净订单兑现）——P1 force_flat 的 held 须**关闭**（逐条产 `ExitType::RiskExit`，走 𝒟_x/P1 typed close 路径退账本，非保留）。这是 P1 与 gross 唯一语义差，其余复用。

### 6.8 P1 × G7 gross_cap 耦合核对（G7 #133 落地 f1c9700332 后，team-lead 请求）

**核实**（读 coverage.rs:1850-1898）：G7 `apply_gross_cap`（1893-1897）作用于 `legs`、在 `net_target_units` 折叠**前**、raw/AncOK/`strategy_target_legs`**后**。新签名 `coverage_step_from_buckets(..., risk: Option<&RiskConfig>, ...)`（1709），gross_cap 门控 `RiskConfig.enforce_gross_cap`（default false）。

**结论：无语义冲突，P1 与 gross_cap 按优先级天然复合（对齐 PDF §7 C_1 屏蔽）**：
- P1（强平）在 raw 层上游短路 ⟹ legs=∅ ⟹ gross_cap 作用于空 legs = 恒等 no-op ⟹ p̃=0。P1 触发时 gross_cap 无事可做，非"先后打架"。
- P1 **不**触发 ⟹ 正常 raw→legs→gross_cap→net。gross_cap 仅在 P1 静默时生效。先后 = P1 上游优先，gross_cap 次之——正是 `C_1=P_1` 屏蔽 P2..P10 的语义。
- **单一 risk 源一致性**：force_flat（P1 判据）与 enforce_gross_cap（G7 门）都 key off `RiskState`/`RiskConfig` ⟹ 与裁定4「force_flat 降级为同一 RiskState 派生」一致——I_Θ 组合层用同一 risk 源喂 P1 分支 + 传 `risk` 给 coverage_step_from_buckets。无双权威源。
