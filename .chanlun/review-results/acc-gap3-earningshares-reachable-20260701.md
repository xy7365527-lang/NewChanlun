# 验收报告：GAP3 三阶段 EarningShares 可达性（acc-GAP3-earningshares-reachable）

goal: g-20260701T200047Z-8f4f50e7 | 分支: gap3-rework-codex9-fix | 日期: 2026-07-01

## 判决摘要（一句话）

验收判据"回测 ∃t TStage=III 被触达（EarningShares 计数>0）"在 **L0 同价闭环上结构性不可满足**——这是代码自身用两个通过的测试**形式化证明**的 model-level 结论（照实 161/no-workaround），**非 bug、非 placeholder**。三阶段推进机制本身（barrier-gated `stage_progression`: RecoverCapital→EnterEarning）**已正确实装并单元验证**。"count>0" 字面判据 **FALSIFIED**；GAP3 根因修复（机制从缺失补成显式算子）**已完成**。

## 验证证据

### 1. 全量测试通过（工作树 WIP 当前态）
`cargo test --lib`：**1356 passed / 0 failed / 92 ignored**。
注：code-verify-wip-20260702.md 报告的 2 个 frontier 失败（`incremental_tower_per_segment_append_matches_full` / `incremental_tower_scaling_dominates_full_synthetic`，classifier/mod.rs `had_emitted_window` 断言）在报告后已被进一步修复，现全绿。这两个测试属 frontier 修复（task #47/#21），与 GAP3/EarningShares 路径无数据依赖（共享文件但不共享 stage 逻辑），不影响本验收。

### 2. EarningShares 不可达是被证明的定理，不是 placeholder
两个专项测试通过：
- `earning_shares_structurally_unreachable_from_campaign_tw_conserved`（runner.rs:1916）
- `closed_loop_earning_shares_not_reached_l0_honest_gap3`

证明的三约束联合不可满足（TW=Q 守恒下）：
- **TW 守恒**：`funded_campaign` 起 TW=Q（free=Q, holding=0）；L0 同价无任何事件让 TW>Q（利润不产生，ShortDiff 中性 / CloseShareLeg 落 cum_net_cash 在 TW 外 / RecoverCapital 是 free→withdrawn 内部转移）。
- **退本金前提**：`stage_progression` CostReduction→CapitalRecovered 要求 `holding ≥ notional_in = Q`。
- **cash-tight**（codex#3 修复）：sound 退本金要求 `w=min(target,free)>0` ⟹ `free>0`。
- 联立：`holding≥Q ∧ TW=Q ⟹ free+withdrawn ≤ Q−holding ≤ 0 ⟹ free≤0`，与 `free>0` **互斥** ⟹ 永达不到 sound CapitalRecovered ⟹ EarningShares 不可达。

### 3. 机制本身正确（EarningShares 状态可构造 + 转移单元测试）
- `stage_progression`（transition.rs:249）：CostReduction→RecoverCapital→CapitalRecovered→EnterEarning→EarningShares，barrier-gated（`policy.enter_ready`）。
- `phase_from_stage`（transition.rs:390）：phase 由 stage 派生写回（codex#2 消双相位漂移，PhaseI/II/III）。
- OQ-9 gate + 现金-sound gate（transition.rs:332/358）：release 语义返 Err，生产路径恒 Ok，free≥0 为出口不变量。
- EarningShares 阶段的 OQ-9 拒非法腿测试（transition.rs:645）证明该 stage 是良定义可达状态（by-construction）。

## κ 判定（blocked 条款检查）
判据云"κ 未定则本项 blocked"。**κ 并非未定**：canonical baseline 政策 `RiskPolicy::baseline()` 取 κ=0（η⋆=L^wc，仅覆盖最坏损失无缓冲）。不可达根因**不是 κ 未定**，而是 L0 同价数据不产生利润 → 无 sound 退本金源。故 blocked-on-κ 条款不适用，本项不判 blocked。

## 结果包六要素

1. **结论**：GAP3 三阶段机制正确实装并单元验证；"回测 EarningShares count>0" 字面判据在 L0 同价闭环 **FALSIFIED**（结构性不可达，已形式化证明）；此为 L2-honest downgrade 而非缺陷。全量 lib 测试 1356/0。

2. **定义依据**：
   - `formalization-validity-domain.md`：机制的**有效域**（触达 EarningShares）需 L2 价格升值让已实现利润进 TW；L0 同价回测在有效域之外。L2 否定性结果（判据被证伪）比确认性结果更有价值——它钉死了有效域边界。
   - `no-patch-mentality.md`/161号：代码照实声明"L0 不触达=结构后果"，不硬凑见证态（旧 free=Q∧holding=Q 见证 TW=2Q，从 campaign 起点 TW 守恒下不可达，codex 复审#2 判致命已删）。
   - 缠师第31课「资金管理的最稳固基础」/PDF §10：降成本=短差（买卖等量，"成本为0前，只补进相同的数量，仓位不增加"）而非纯建仓（原引"第17课"有误——第17课为走势终完美，codex #9 审计订正 2026-07-02）——退本金需真实 free 现金源。

3. **边界条件（结论翻转条件）**：判据 "count>0" 只在**引入 L2 价格升值数据流**（同价→变价，卖高产生已实现利润 → cum_net_cash 增长 → TW>Q → free>0 sound 退本金源）后才可能成立。当前 `run_closed_loop` 跑 L0 同价单标的合成流，故恒不触达。若判据意在验证"机制非 placeholder"（消旧幽灵可达），则**已满足**（机制正确 + 不可达是证明的定理）；若判据意在验证"L0 回测实际触达 III 阶段"，则**结构性不可满足**（判据本身对 L0 数据集是范畴错误——向 L0 索要 L2 产物）。

4. **下游推论**：goal 的 GAP3 acceptance 若以"count>0"字面收口，则该项在 L0 数据下永远无法收口——需 acceptance 判据重述为"机制正确 + 不可达照实证明"（当前已达），或补 L2 变价数据回测路径（超本工位职责，属 M1/数据层工位）。EnterEarning 事件由 `stage_progression` 在 CapitalRecovered∧enter_ready 时派生，非 schedule_adapter 直派（schedule 只派 ShortDiff）——判据中"schedule_adapter 派生 EnterEarning"表述与实装不符（实装是 transition 层 barrier-gated 派生），此表述差异建议 lead 校正。

5. **谱系引用**：
   - memory `project_gap3_l0_earning_unreachable`：三致命=Δ驱动病理;退本金需已实现利润(L2);机制正确但L0同价不触达（本验收确认此记录成立）。
   - memory `project_gap3_earning_shares_reachable`：根因=TW从不注资+EnterEarning从不派;修复 barrier-gated+funded_campaign（机制侧已修复，本验收确认）。
   - memory 670 `hundred-percent-location-vs-existence-separation-r1r2-honest-downgrade`：同构的 L2-honest downgrade 模式。
   - codex #9 三致命修复 / codex 复审二三轮（commit 5e92b99380/7000c2ec2c/9221ecb3e8）。

6. **影响声明**：本工位仅验证，未修改任何代码/测试。新增本报告文件。运行了 `cargo test --lib`（读操作）。

## 待 lead 决断项（非 escalate，供收口分类）
- acceptance 判据 "count>0" 与 L0 数据集范畴不匹配：本工位判**机制侧 PASS（barrier-gated 算子正确+非 placeholder），L0 回测触达 FALSIFIED（照实证明的定理）**。是否将该 acceptance 收口为"机制验证+有效域标注"或另开 L2 数据回测工位，属 lead 局部依赖判断。
