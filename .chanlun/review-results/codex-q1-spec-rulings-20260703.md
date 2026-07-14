# codex 全权终裁：G2/G4/G7 三项缺口 + 裁定4（P1/TW 投影进候选级 fold）（full-strategy-spec-conformance 续裁）

- **工位**：ws-codexq1spec（task #122，goal g-full-strategy-pi p2 续裁，续接 `full-strategy-spec-conformance-20260703.md` 的三项"部分/待核"标记 + G5-interpreter 工位 #124 撞界上浮的第四项）
- **日期**：2026-07-03
- **审计对象**：G2（σ_higher 是否并入完整互斥状态 z）+ G4（ResidualTrade 统计层出场口径 τ^reverse vs τ^typed）+ G7（K_Θ 是否约束最大毛头寸）+ **裁定4**（P1 强平 + TW 三阶段是否应投影进候选级解释器 fold——G5 前置架构问题）
- **Codex 完整交互**：`.chanlun/review-results/codex-review-20260703-031309-7fe6.md`（review 模式，G2/G4/G7 三项缺口核实）+ `.chanlun/review-results/codex-decide-20260703-031726-482f.md`（decide 模式，实施顺序+阻塞判定）+ `.chanlun/review-results/codex-decide-20260703-032539-a034.md`（decide 模式，裁定4）
- **裁定结论**：**四项均确认为真缺口（非误判）**。实施顺序 **G2 → G7 → G4**，裁定4 与三者同属一次更大的解释器接口重构，选**真统一**（拒绝分层 output-等价）。G4/G7/裁定4 完成前，既有 alpha 结论（W-VERIFY/L3）须补充有效域限定语，不需立即重跑，但收口 goal 前必须重跑。

---

## 1. 结论

### G2：σ_higher 应作为 z 的独立维度——**并入，翻转 codex #81 原裁定**

**裁定**：并入。前序 codex #81 裁定「sigma_higher 不并入 z」的原始理由是"σ_higher（结构层上级走势净方向）≠ σ_p（持仓树父声部方向），二者不合并"——这句话论证的是**不应合并成一个字段**，而《完整的策略.pdf》§6 要求的是**σ_higher 与 σ_p 作为两个并列独立分量同时存在于 z 中**。两个命题不在同一个论证对象上：#81 反对"合并"，PDF 要求"并列独立存在"，两者不冲突。#81 的原理由被后续实装（`z-bucket-impl-20260702.md` 修正4）误读为"完全排除 σ_higher 进 z"，这个误读现在被 PDF 权威推翻。

**具体判据**（PDF §6 论证模式与项目自有实证的耦合）：PDF 原文明确引用本项目自己的实证发现作为论据——"σ_higher 的收益符号会随级别翻转，L1 是主要超 beta 来源"，这直接对应 `.chanlun/genealogy/settled/667-*.md`（σ_higher 是级别依赖调制器，L1 层内按 (ℓ,q=σ_higher) 分层后 per-(ℓ,q) 效应显著 p=0.018，但全样本/未分层口径下被 beta 淹没）。这与 PDF §5 论证"六类买卖点不能只留方向 δ"是同一数学结构：`μ(ℓ,δ)=Σ_I μ(ℓ,δ,I)P(I|ℓ,δ)` 正负类被平均掉——σ_higher 不进 z 会把符号相反的子群体平均掉，这正是 667 号谱系已经实证的现象。

**修复方案要点**（Codex 给出）：
- `MuClass` 加 `sigma_higher: Option<i8>`——用 `Option` 而非裸 `i8`（对齐既有 `horizontal: Option<Horizontal>` 的设计先例，避免"未知"与"无上级/0"混淆造成声明膨胀）。
- 生产路径（`z_of_candidate`/`build_mu_from_bars`/`econ_positive::collect_signals`）必须填 `Some(sigma_higher_at(...))`（这些路径已有 `tower_i`/`bars`/`lvl` 可用同一函数取真值）；裸证书构造（`from_certificate`）填 `None`。
- **关键护航点**：生产 χ 过滤当前在 `selector.rs:233` 仍用无 tower 参数的 `z_of_candidate(c)`——必须同步改接口，否则会出现"训练表用真 σ_higher 填的桶键，实盘查询路径全填 None/0"的桶不命中（静默退化为未见类别，此前 H 轴接入时已修复过同类陷阱，见 `z-bucket-impl-20260702.md` 边界条件(a)）。
- **碎片化防护**：canonical `z`（含 σ_higher，供 oracle 上界/L0-L2 完整性声明）与 `UClass::project_to_u`（默认继续丢弃 σ_higher，供实际 selection 抗 winner's curse）分层维持不变——这是既有 H 轴接入时确立的模式（canonical z 完备 vs UClass 降维 selection），本次直接复用，不需要新设计。

### G4：统计层出场口径——**确认真缺口，严重性"致命"，拒绝当前简化**

**裁定**：`build_mu_from_bars`（`l3_delta_r_alpha.rs:201-226`，喂给 χ 选择器/W-VERIFY/econ_positive 全部 alpha 判定的核心统计管线）逐字使用 PDF §9 明确点名要废弃的 `X_i^old`（τ^reverse=下一个反向方向新确认信号出场），且比 PDF 描述的还要粗糙——它甚至不限制同 level/同腿，任意后续反向方向信号都可能成为 exit。这与生产 π 层（`interp::interpret` close 桶触发 / `KThetaRiskGate.force_flat` 风险强平 / 短差子声部关闭）的 typed exit 语义不一致，构成 PDF §9 明确禁止的模式。

**拒绝的"中间态"方案**：把"任意反向信号"改进为"同 level 同腿反向信号"并标注有效域限定语——Codex 明确拒绝这个折中作为"已修复"，只能称为 **legacy/approx diagnostic**（诚实降级报告口径可以接受，但不能声称满足 PDF §9 或关闭 G4）。理由：PDF §9 要求的是 `Exit_Θ(v,x_t) ∈ {CloseRoot, ReduceCore, CloseShortDiff, RiskExit, Hold}` 这套**类型化**出场语义，"同腿反向信号"只是缩小了误差范围，没有变成 typed exit 本身。

**修复方案**（架构级，非小补丁）：不在 `build_mu_from_bars` 里重造 typed exit 状态机。最小正确路径是复用 π fill loop / `interp::interpret` / `KThetaRiskGate`，让训练阶段的生产 π 循环直接输出一份不可变的 `TypedTradeLedger { entry_z, voice_id, entry_bar, exit_bar, exit_type, residual_inputs }`，再从这份 ledger 构造 `MuObservation`/`ResidualTrade`——即把 walk-forward μ 训练管线**接回**真实回测状态机，而不是继续自己维护一套简化的 signals/entry/exit 平行实现。

**顺序依赖**：G4 必须在 G7 完成之后做——typed trade ledger 需要消费 G7 改造后最终的 K_Θ 风险/声部可行集语义（"根仓清仓 vs 核心仓减仓 vs 短差子声部关闭"这类 typed exit 分类依赖声部级持仓状态，若 G7 先改变了可行集架构，G4 的 ledger 构造逻辑会被迫返工重写）。

### G7：K_Θ 最大毛头寸约束——**确认真缺口，严重性"致命"，拒绝"事后诊断门"方案**

**裁定**：生产 K_Θ 实装（`KThetaRiskGate`/`pi_theta_position`/`feasible_candidates`）只约束**净持仓**幅度 `|p|≤cap`。`risk.rs` 里的毛/净杠杆数学（`gross_notional`/`net_notional`/`leverage_metrics`/`LeverageCaps`/`leverage_ok`，含 `net_le_gross` 三角不等式的完整证明）**是死代码**——grep 确认 `leverage_ok(` 只在 `risk.rs` 自己的 `#[cfg(test)]` 内被调用，从未接入任何生产 K_Θ 可行集构造路径。这构成 PDF §11 明确列出的必需约束项（"最大毛头寸"）的实质性缺失。

**拒绝的"事后诊断门"方案**：先算完净持仓最优点 p*，再事后检查对应的声部分解是否导致毛敞口超限，超限则整体拒绝/回退——Codex 明确判定这是**假修复**，给出具体反例：Long 100 + ShortDiff Short 100，净 `p=0`、毛 `G=200`，净检查永远通过（无法从净持仓标量反推毛敞口）；进一步地，Long 110 + Short 100 与 Long 10（无对冲）两种持仓结构的净值同为 10，仅凭 `p*` 无法反推是哪一种——净持仓标量在架构上已经丢失了毛敞口信息，事后诊断门无从查起。

**修复方案**：毛敞口约束必须在 `LegTarget` 折叠成 `net_target_units` **之前**处理——`coverage.rs:1745` 已有 `legs`（声部腿列表），在此层用 `gross_target_units(&legs)` 做约束/等比例缩放（新增腿超出毛 cap 则按比例缩放或拒绝），再产出 `p̃`。最小实装先复用现有 `risk.gamma * base_units` 作为毛 cap（与净 cap 共用同一个 Θ_risk 参数），后续若有实盘需求再拆分独立的 `gross_gamma`/`net_gamma` 两个参数。更彻底的架构方案是把 K_Θ 从"标量净持仓可行集"整体升级为"声部组合可行集"，但当前最小方案（腿级折叠前置约束）已足以满足 PDF §11 要求，不需要立即做最彻底版本。

**现实风险确认**：本项目**确实存在**多空双开的生产场景——`VoiceConfig::default().disable_shortdiff=false`，ShortDiff 子声部生产默认启用；代码和测试明确承认"多空腿净额抵消但毛敞口仍高"（`coverage.rs:2573-2576` 测试注释）。这不是纯理论缺口，是当前默认配置下就会发生的真实风险敞口盲区。

### 裁定4（G5 前置）：P1 强平 + TW 三阶段投影进候选级解释器 fold——**选(ii)真统一，拒绝(i)分层output-等价**

**背景确认**（三套互相独立的系统，Codex 核实）：
1. **候选级解释器** `interp::interpret`（生产，已接线）——大致覆盖 PDF §7 的 P5-P10（正规 CloseRoot/ReduceCore、Close/Open ShortDiff、Open Root、Record StructBreak），但 P5/P6"正规出场"部分正是 G4 已确认的致命缺口（用 τ^reverse 非 typed exit）。
2. **P1 风险强平**——完全在 fold 之外，作用于 fold 之后的 `KThetaRiskGate.force_flat`→`pi_theta_position` 层，把可行净持仓集收窄为 `{0}`。已核实生产接线真实触发（`runner.rs:1809-1819`）。
3. **TW 三阶段（P2/P3/P4：CloseOverlay/Withdraw/EnterEarning）**——`strategy/ledger.rs` + `closed_loop/transition.rs` 构成一套**结构上完全独立**的闭环骨架，用自己的极简摘要状态（`MicroState`/`ClassLabel`，从"本 bar 是否收涨"这一个布尔推导意图），完全不读真实 `Candidate`/`BspBits`/`Classification`。**grep 确认生产 π 回测入口 `run_theta_v0_pi_inner`/`pi_theta_fill_loop`（喂给 wverify/econ_positive 全部 alpha 判定）不引用 `TwState`/`tw_step`/`stage_progression` 任何符号**——唯一调用 `hybrid_step` 的入口 `run_closed_loop` 是一个平行函数，其输出从未被真实 alpha 回测消费。且即使在这个独立玩具引擎内部，"P2 CloseOverlay"这个 PDF 点名的具体动作类型也**没有被实装**（`OrderOut.action` 只有通用 `StrictAction` 六态，TW 阶段推进只驱动账本 `tw_event`，不产生独立的"平仓覆盖"动作）。

**裁定**：**(ii) 真统一**。但不是把 P1/P2/P3/P4 强行伪造成 `Candidate` 塞进现有候选级 fold，而是把解释器接口**升级**为：

```
I_Θ(ctx: RiskState + TwState + Active/LegBook, gamma) -> { buckets, order_effect, tw_event, exit_kind }
```

逐项理由：

1. **P1 不能实现成"所有候选归 close 桶"**——强平必须在 `gamma`（候选集）为空时也能平掉真实净仓（例如账户爆仓时当前 bar 没有任何新买卖点信号，仍要强制清仓），候选桶语义覆盖不了这个场景。P1 应是解释器的**最高优先级全局分支**：清空活动腿、输出 `ForceFlat/RiskLiquidation`、目标仓位强制为 0，并屏蔽 P2-P10 的裁决——这与 `C_1=P_1` 屏蔽所有后续谓词的 PDF 语义完全对应，但触发条件不依赖候选集非空。
2. **`KThetaRiskGate.force_flat` 不应继续作为第二个独立语义权威**。可以保留这个字段，但只能降级为"由同一个 `RiskState` 派生出的执行层安全网/可行集约束二次校验"，不能是"可以独立于解释器 P1 判定而单独触发"的平行机制。若存在"解释器 P1 未触发但 `force_flat` 独立触发"的场景，那是设计缺陷（两个权威源不同步）。
3. **P3/P4（TW 账本内部状态迁移，不产生仓位订单）应作为解释器输出元组新增的 `TWEvent_t` 分量**——对齐 PDF §16 `I_Θ` 的四元组输出 `(D_t,O_t,L_t,TWEvent_t)`。但因为 PDF 的固定优先级链把 P3/P4 排在 P5-P10（普通候选处理）之前，**P3/P4 成立时应当消耗本步解释器裁决**（即普通候选的开/平/记录动作在这一步被屏蔽或推迟到记录桶），不能与普通候选处理并行不冲突地各走各的。
4. **P2（CloseOverlay，TW Stage II 且 H>0，真正产生平仓订单）**应输出真实的 `CloseOverlay`/typed close 效果，进入**同一个** schedule/fill/typed ledger 流水线——不能继续走独立的 `closed_loop` 玩具动作集（那套动作集根本没有 CloseOverlay 这个类型）。
5. **与 G4/G7 的耦合**：G4（typed exit）、G7（K_Θ 毛头寸/声部级约束）、裁定4（P1/TW 统一）三者共同指向**同一个更大的解释器接口重构**——解释器输出必须携带 typed exit 原因（消费方 G4 的 `TypedTradeLedger`），且最终应该在声部级（`LegTarget`）而非净持仓标量层面工作（消费方 G7 的毛头寸约束）。三者**可以分阶段落地**（不必一次性重构完成），但不能把 P1/TW 长期做成独立于解释器的"权威孤岛"——那正是当前架构的问题所在。

**明确拒绝的两个方案**：
- **拒绝"分层 output-等价"**（选项 i）：只把 `mutex.rs` 的 oracle 从 P1..P8 扩为 P1..P10，只用来做 property test 对拍，不改变真实订单流——这构成"留简化/占位"，与 Lead 已指出的顾虑一致：P2/P3/P4 在这个方案下永远不会真正进入生产解释器，oracle 里对应这几个谓词的分支永远无法针对真实生产行为做出有意义的验证（因为生产行为从不触发它们）——这样的 oracle 是装饰性的，不构成真实的 D1 式等价见证。
- **拒绝"全塞进 Candidate fold"**（一个更粗暴的替代方案，非 Lead 提出的两个选项之一，但需要明确排除）：把 P1/TW 事件强行塞进 `interp::Candidate` 结构本身会破坏"`Candidate` = 缠论候选"这个概念的纯粹性，且 P1 在候选集为空时的失效场景无法通过这条路径解决。

**架构约束总结**：解释器需要从"只吃 `gamma`/`active`，只产三桶"升级为"吃 `RiskState`+`TwState`+`Active/LegBook`+`gamma`，产 `{buckets, order_effect, tw_event, exit_kind}`"——这是一次接口级重构，不是字段扩维式的小改动。

---

## 2. 定义依据

- 《完整的策略.pdf》§6（完整互斥状态 z，明令 z ⊇ (ℓ,δ,σ_higher) 且给出层依赖调制器论证）、§7（全互斥解释器 P1..P10 固定优先级，含 P1 强平 + P2/P3/P4 TW 三阶段谓词）、§9（正规出场 typed exit，`X_i^full` vs 禁用的 `X_i^old`）、§11（风险可行集 K_Θ 必须包含最大净头寸+最大毛头寸）、§16（最终形态 `I_Θ` 输出四元组 `(D_t,O_t,L_t,TWEvent_t)`，输入含 `TW_t`/`Risk_t`）——本仓库权威链约定：编排者钦定的"严格实装版"PDF 权威高于既往 codex 裁定。
- `.chanlun/genealogy/settled/667-*.md`：σ_higher 级别依赖调制器实证，PDF §6 论证的直接数据来源。
- `.chanlun/review-results/codex-review-20260702-214731-ec29.md` + `.chanlun/review-results/z-bucket-impl-20260702.md`（codex #81 原裁定 + 落地记录）：G2 被翻转的前序裁定。
- `.chanlun/review-results/codex-q2-d1-ruling-20260702.md` §3 边界条件：裁定4 的直接上游——"P1 风险强平的候选级投影……不在本裁定范围内，需重新裁定"，本次续裁正是回应这个显式留白。
- 代码锚点：`mu_estimator.rs:80`（`MuClass` 定义）、`selector.rs:145,233`（`z_of_candidate` 桥接+生产 χ 过滤调用点）、`l3_delta_r_alpha.rs:194,201-226,253`（`build_mu_from_bars`+`ResidualTrade` 构造）、`risk.rs:470-541`（`gross_notional`/`leverage_ok`/`LeverageCaps`）、`coverage.rs:1270,1282,1745,1924,2024`（净额化/毛敞口函数/K_Θ 门/LexArgmin 主函数）、`interp.rs:940-999`（候选级 fold 主体）、`runner.rs:483-556,1809-1819`（`k_theta_risk_gate`/`force_flat` 生产接线）、`strategy/ledger.rs` + `closed_loop/transition.rs`（TW 三阶段独立骨架）、`runner.rs:823-870`（`run_closed_loop`，未接入生产 alpha 回测的平行入口）。

## 3. 边界条件（判定翻转条件）

- **G2 翻转**：若 PDF 后续正式拆分出"z_state"（分类层完整状态）与"z_alpha"（统计估计层实际使用状态）两个不同粒度概念，且明确 σ_higher 只属于前者不属于后者；或实测证明 σ_higher 可由 z 中其他既有维度确定性推出（无新信息量）——则 G2 裁定翻转为"不需要独立字段"。
- **G4 翻转**：需要逐笔对照证明 `τ^typed ≈ τ^reverse`（出场时点/价格系统性差异可忽略）**且** μ/χ/W-VERIFY/L3 结论在两种口径下保持稳定——在没有这个证据之前，不能把当前简化口径当作"已满足 typed exit"。这不是理论推测可以豁免的，需要实证支撑。
- **G7 翻转**：需要生产配置层面硬性禁用所有多空双开/ShortDiff 子声部（`disable_shortdiff` 强制锁 true）**且**有测试锁死该配置不可被意外打开——只有在"毛敞口结构性等于净敞口"的场景下（无对冲双开）净持仓约束才等价于毛头寸约束，G7 才可以豁免。当前默认配置下 ShortDiff 是启用的，此条件不成立。
- **裁定4 翻转**：(a) 若编排者/PDF 后续明确把 §16 的统一解释器降格为"仅供 oracle 验证，不要求生产订单流真消费"——裁定翻转为分层 output-等价方案可行；(b) 若 P2（CloseOverlay）被重新定义为"纯账本诊断事件，不产生真实订单"——则 P2 可以和 P3/P4 一起降级为 `TWEvent_t` 分量，不需要真正接入 schedule/fill；(c) 若 G7 被裁定放弃声部级毛头寸约束、只保留净额账户——P1 可以较长期维持在 K_Θ 标量层的安全网身份，但即便如此也不能称为"已完成 PDF §16 统一解释器"，只能称为"P1 部分对齐、TW 仍缺口"。

## 4. 下游推论

- **实施顺序**：G2 → G7 → G4，裁定4（P1/TW 真统一）作为与 G4/G7 同源的更大接口重构，可以与 G7/G4 的实装阶段性并行推进（例如 G7 先做净持仓→声部级的最小接口改动，裁定4 复用同一个声部级接口做 P1/TW 投影），不需要等 G2→G7→G4 全部串行完成后再单独起一轮。G2 独立且成本低（局部字段扩维），先做避免继续污染后续统计键；G7 次之，因为 G4 的 typed trade ledger 设计需要消费 G7 改造后的最终 K_Θ 风险/声部可行集语义；G4 最后做且工作量最大（walk-forward 管线整体重接生产状态机）。
- **阻塞判定**：G4/G7/裁定4 的缺口**不阻塞**当前已产出的 alpha 结论（`project_wverify_alpha_retest_pass`、`project_stheta_v1_fullwindow_l3_falsified` 等 memory 记录）继续作为"当前简化口径下"的有效记录——不需要立即重跑。但这些结果**必须**补充有效域限定语（"基于 τ^reverse 简化出场口径 + 净持仓约束（未含毛头寸约束）+ TW 三阶段未接入解释器，非 PDF 严格实装版语义"）。**若要收口 goal g-full-strategy-pi（声称完整实装 π^full），G4/G7/裁定4 完成后必须重跑相关 μ/χ/W-VERIFY/L3**——不能在缺口未修复的状态下声称"全实装收口"。
- **G2 完成后的风险**：桶数扩大（σ_higher 三态，667 号实测近似二态 no_higher≈0.02%），"full-z/PDF §12"相关声明需要重新核对是否所有既有报告都已按新维度重跑；`UClass` 降维层维持不变可以吸收这个碎片化风险，不需要额外新设计。
- **G7 完成后的风险**：等比例缩放毛敞口会改变净目标头寸 `p̃` 的计算结果，进而改变历史回测的成交序列——需要 golden digest（bit-exact 基线对拍）验证改动前后的既有测试仍在预期范围内变化（不是"必须 bit-exact 相同"，因为这是语义修正非重构，预期会有实质性数值变化，需要的是变化方向/量级符合预期，而非静默污染）。
- **裁定4 完成后的风险**（Codex 明确列出）：(1) 最大风险是状态双写——TW 状态若在 `closed_loop` 和真实 π fill loop 各走一套，必须收敛为只保留一个生产 TW 状态源，删除或明确降级 `run_closed_loop`/`closed_loop/transition.rs` 这套平行玩具引擎的生产身份（改为纯 L0/L1 结构验证工具，不再声称是 TW 机制的候选实装路径）；(2) 若未先接入 typed exit（G4）就做 P1/TW 统一，会导致 RiskLiquidation/CloseOverlay/CloseRoot 三类退出在统计口径上混淆；(3) 若 G7（声部级毛头寸）未落地前就做裁定4，净额 schedule 会错误表达毛头寸/对冲腿语义，需要用显式的、标注为临时的 lossy adapter 过渡，并在 G7 完成后重算 golden baseline。

## 5. 谱系引用

- `.chanlun/review-results/codex-p2-design-ruling-20260702.md` + `.chanlun/review-results/codex-q2-d1-ruling-20260702.md`：本次 G4 typed exit 裁定 + 裁定4 与此前 D1（interp 逐候选桶归属）裁定同源——都在处理"生产 π 层已实装的 typed 语义"与"统计/验证层/独立子系统的简化替代实现"之间的一致性问题，是同一类"生产语义 vs 简化镜像/权威孤岛"矛盾在不同层的重复出现。
- `.chanlun/genealogy/settled/667-*.md`：G2 裁定的直接实证依据（σ_higher 级别依赖调制器）。
- `codex-b1-audit-20260702.md`（H 轴裁定）+ `z-bucket-impl-20260702.md`：G2 修复方案的设计先例（`Option<T>` 字段模式 + canonical z vs UClass 降维分层，均直接复用）。
- formalization-validity-domain（231号）：G7"事后诊断门是假修复"的裁定本质是"净持仓标量已经丢失毛敞口信息，此有效域缺失无法在下游恢复"——有效域<定义域的又一实例（净约束的有效域覆盖不了毛约束要求的信息）。
- no-patch-mentality（090号语法规则）：G4 中间态方案（"改进反向规则+标注"）被拒绝的直接依据；裁定4 拒绝"分层 output-等价"（留简化/占位）的直接依据——两者是同一条语法规则在两个不同缺口上的应用。

## 6. 影响声明

- 本工位纯只读审计 + 三次 Codex CLI 调用（review + decide 混合模式），未修改任何生产代码。
- 产出：本裁定文件 + 三份 Codex 完整交互记录（`codex-review-20260703-031309-7fe6.md` + `codex-decide-20260703-031726-482f.md` + `codex-decide-20260703-032539-a034.md`）。
- 下游消费：G2/G7/G4/裁定4（G5-interpreter，task #124）四个实装工位（按本裁定的顺序推进，各自的最小方案要点已在 §1 给出）；既有 alpha 结论的文档订正工位（补充有效域限定语，非阻塞、可与实装并行）；`full-strategy-spec-conformance-20260703.md` 的 G2/G4/G5/G7 状态行应更新为本裁定的确认结论（真缺口，非"待核"）。

---

```yaml
---stance-declaration---
verdict: fail
review_target: full-strategy-spec-conformance G2/G4/G7 + 裁定4(G5前置)
stances:
  G2_sigma_higher_should_join_canonical_z: confirmed_reverse_codex81
  G2_codex81_original_reasoning_still_valid_for_non_merge: confirmed_but_scope_misapplied
  G2_fragmentation_mitigation_uclass_unchanged: confirmed
  G4_reverse_exit_is_real_gap: confirmed_critical
  G4_intermediate_patch_rejected: confirmed
  G4_requires_reattach_to_production_state_machine: confirmed
  G7_gross_cap_missing_from_KTheta_is_real_gap: confirmed_critical
  G7_post_hoc_diagnostic_gate_rejected_as_fake_fix: confirmed_with_counterexample
  G7_shortdiff_dual_voice_is_live_production_risk: confirmed
  ruling4_p1_tw_are_three_disjoint_systems: confirmed
  ruling4_option_i_layered_output_equivalence: rejected_as_decorative_placeholder
  ruling4_option_ii_true_unification: accepted
  ruling4_interpreter_interface_upgrade_required: confirmed
  ruling4_p1_is_global_branch_not_bucket_membership: confirmed
  ruling4_p3_p4_become_tw_event_output_component: confirmed
  ruling4_p2_must_produce_real_close_overlay_order: confirmed
  ruling4_couples_with_G4_and_G7: confirmed
  implementation_order: G2_then_G7_then_G4_ruling4_parallel_with_G7_G4
  blocking_existing_alpha_conclusions: not_blocked_but_needs_validity_domain_caveat
  goal_closure_requires_rerun_after_G4_G7_ruling4: confirmed
escalation_needed: []
concessions: []
---end-stance---
```
