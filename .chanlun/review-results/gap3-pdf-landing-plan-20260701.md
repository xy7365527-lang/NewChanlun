# GAP3 三阶段提现端 PDF 落地规划（trust-but-verify）

> 来源：gap3-pdf-plan (planner) 只读规划，2026-07-01。规格=`/Users/silencehan/Downloads/gap.pdf`（ChatGPT 单方推导）。
> 认识论 L0/L1（代码事实核查，验证 PDF 声明 vs 代码一致性；不验证三阶段缠论/实盘有效性）。
> 异质裁决≠实施授权（memory 首条）——本产出是待编排者授权的规划。要点版，完整逐符号对照表可向 gap3-pdf-plan resume 索取。

## 三个重大偏差（PDF 假设 vs 实际代码）

1. **PDF"最重要修复"针对不存在的靶子**：`cost_basis≤0⇒EarningShares` 这条 Lean 不变量 grep formal/ 全目录零命中，**从不存在**。实际在 Rust `t_engine.rs:360`，且已是正确的 `cost_basis≤0⇒CapitalRecovered`（穿零→退本金，非增股数）。Rust 已是正确三段式。PDF §3/§12 反复强调删这条=修不存在的东西。

2. **PDF 把"实装接线缺口"误判为"spec 缺口"**：Lean `CovariantCapital.lean:366-390` 已有 ReadyReturn/StageThree/phaseOf/尺度不变定理；FullDefinitionStrategy+Foundation 各有 5 态 CapitalPhase。PDF 称"Lean 只有 raw step+gate 没触发"不符实。

3. **136 阻塞编号不存在**：goal events/interrupt/roadmap 无"GAP3"无"136"立项。GAP3 是临时编号。真实阻塞=**576号 pending**。

## 符号对应表（12 PDF 符号）
- 双侧已有(4)：TStage / openLegacyLegs / EnterEarning 事件 / RecoverCapital 事件
- Lean 有 Rust 缺(3)：ReadyReturn / Phase / η⋆ 参数
- 双侧缺(3)：RawPhase 独立类型 / D_dist / η⋆ barrier 公式
- PDF 靶子不存在(1)：cost_basis≤0⇒EarningShares
- 命名微差(1)：CapitalRecovery ↔ CapitalRecovered

## 四分法分布（PDF 5 裁定）
- 定理类(1)：裁定4（W≥I0 不蕴含 OQ-9）——Lean 已 machine-checked 结算
- 行动类(1)：裁定2（schedule_adapter 占位 → 接触发），transition.rs:135 字面坐实
- 选择类(2)：裁定1（RawPhase，基于错误前提）/ 裁定3（η⋆ canonical 形式）
- 混合(1)：裁定5（priority 结构=定理 + 参数=选择）

## 真实阻塞：576号账本不同构
- `R=Π−A−W`（closed_loop 主账本，sell.rs:33 "W 恒 0"）vs TW 三阶段（提现端 W 所属）**不同构**。
- 提现端 W 无法接线的根因=双账本对接方向未定。
- 三选一（编排者裁）：A 单账本 R=Π−A−W / B 单 TW 三阶段 / C 双层并置（planner 建议 C）。
- 引擎侧三阶段已完整实装（t_engine.rs 全套+deploy_earning）；闭环侧未接（schedule_adapter 占位）="都实现了却不实装"。

## 落地顺序（3 escalate 前置，不能先接线后补概念，避免重蹈 RecoverCapital(1) 占位覆辙）
1. [escalate] 576号账本不同构三选一
2. [escalate] η⋆ canonical 形式（PDF+codex 两异质源未收敛，需编排者定案非再加源）
3. [escalate/可消解] 裁定1 RawPhase 前提——基于不存在的靶子，倾向不采纳
4. → W1 ReadyReturn 完整触发谓词 Rust 镜像（补 μ/O/后代全平）
5. → W2 schedule_adapter 接触发，消除 RecoverCapital(1) 占位
6. → L2 验证 EarningShares 可达

## 结果包（六要素）
- **结论**：GAP3 真实缺口=闭环接线（非 spec 缺口）；核心阻塞=576号账本不同构，需编排者三选一
- **定义依据**：PDF vs Rust(t_engine.rs:360 / transition.rs:135)/Lean(CovariantCapital.lean:366) 逐符号对照
- **边界条件**：若编排者选账本方案 A/B/C 之一 → 接线方向定；若 576 维持不同构 → 提现端无法忠实接线
- **下游推论**：EarningShares"负成本挣股数"可达性依赖 576 裁决；PDF 裁定1 消解（靶子不存在）
- **谱系引用**：576号(账本 R vs TW 三阶段) + codex-decide-20260701-1646(C2 同问题独立裁决，更贴代码)
- **影响声明**：未改任何代码（未碰 ledger.rs / transition.rs / Lean 契约锚）
