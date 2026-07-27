use super::*;

// ════════════════════════════════════════════════════════════════════════════
//  §9b 组合层 I_Θ（#134 裁定4「组合层」；#145/#146 typed exit + #199/#209/#237 断言①）
//  自 `sizing` 分出（main 侧增长后 sizing 达 892 行，超 800 行上限）。分缝判据 =
//  **决策点归属**，非「生产 vs 旁路」：`sizing` 承担 §9 π_Θ 定序/可行集/下单；本文件
//  承担组合层裁决——P1 `force_flat` 短路、P2/P3/P4 优先级级联、断言①终态门。这些是
//  **生产控制流**，`StepTrace`/`VoiceVerdict` 只是其外化载体，故名 compose 而非 trace。
// ════════════════════════════════════════════════════════════════════════════

// ── ★#199 断言①「T1 目标态」探针（thread_local，runner.rs #198/#199 同款模式；
//    恒在计数——「断言在生产路径真实触发」以探针 >0 为凭）──
thread_local! {
    static T1_TARGET_ZERO_PROBE: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static T1_TARGET_RESIDUAL_PROBE: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// 断言①评估探针计数。pub 的真实理由：lib-target dead_code lint 规避——唯一调用点在
/// `pub(crate)` 的 `pi_theta_step_traced` 体内（lib lint 视角该函数不可达，private
/// 探针函数会被连锁误报 never used）；跨模块消费者（runner BTC 见证）只经 reset/count
/// 读取，bump 本身仅本模块断言块调用。
pub fn t1_target_zero_probe_bump() {
    T1_TARGET_ZERO_PROBE.with(|c| c.set(c.get() + 1));
}

/// 归零断言①两枚探针（评估 zero + 违例 residual；见证测试 run 前调用）。
pub fn t1_target_zero_probe_reset() {
    T1_TARGET_ZERO_PROBE.with(|c| c.set(0));
    T1_TARGET_RESIDUAL_PROBE.with(|c| c.set(0));
}

/// 读取断言①探针快照（一类批 Core 目标评估笔数）。
pub fn t1_target_zero_probe_count() -> u64 {
    T1_TARGET_ZERO_PROBE.with(std::cell::Cell::get)
}

/// ★#209 终态违例探针：一类批后**级残余非零**笔数（release 构建可观测面；debug 构建
/// 由断言①挂载点 debug_assert 先行拦截）。★#237 起口径 = **批次方向侧**分量残余
/// （一类卖批查多侧/一类买批查空侧，按方向拆查）。历史对照：#199 测量态 BTC 实测违例=1。
/// pub 理由同上（lib-target dead_code lint 规避；跨模块消费只经 reset/count）。
pub fn t1_target_residual_probe_bump() {
    T1_TARGET_RESIDUAL_PROBE.with(|c| c.set(c.get() + 1));
}

/// 读取级残余违例探针快照（#209 终态见证材料：一类批后批次方向侧 target_qty(Core{L})≠0
/// 笔数——#237 拆查口径，验收口径 = 全窗 0）。
pub fn t1_target_residual_probe_count() -> u64 {
    T1_TARGET_RESIDUAL_PROBE.with(std::cell::Cell::get)
}

// ════════════════════════════════════════════════════════════════════════════
//  §1 语法元素 e（M16/M17 `SyntaxElement` 的 rust 镜像，从 classifier 塔提取）
// ════════════════════════════════════════════════════════════════════════════

/// 声部裁决记录（#201 阶段 B：per-voice 裁决序列的记录单元，**schema 冻结**）。
///
/// **冻结字段清单**（本票之后不得增删改；阶段 C 换内核在本契约上对齐，spec WP-3）：
/// - `leg`：裁决对象腿（声部身份 level/dir/ElementId 全在）。
/// - `exit`：typed 裁决（[`interp::ExitType`] 单源五枚举，**枚举零改**）——
///   反向关闭 = CloseRoot/ReduceCore/CloseReverseOpen（interpret 规则2 触发，
///   [`interp::reverse_exit_type`] 判定）；P1 强平 = RiskExit；TW P2 overlay 关闭 =
///   CloseReverseOpen（与规则2 短差关闭同 typed——源头区分在保留桶 `closed`/`overlay_closes`，
///   与 typed ledger 粒度一致）；**无出场 = Hold（显式持有——本票核心：Hold 由隐式转显式）**。
///
/// 覆盖域不变量：`prev_active = verdicts ⊎ silent_drops`（划分）——每 bar 每**解释器裁决域**
/// 持仓声部恰一枚；§13 结构剪除（AncOK 连带剪/Stale prune）是活动集与树的状态同步、**非
/// 解释器裁决**（组合层既有口径），其生命周期事件仍在 `silent_drops` → typed ledger 轨显式
/// （`via_structural_prune=true`），不进裁决序列。序列按 `prev_active` 次序确定序输出
/// （bit-exact 可复现）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VoiceVerdict {
    /// 裁决对象腿（声部身份全在：level/dir/ElementId）。
    pub leg: ActiveLeg,
    /// typed 裁决（单源五枚举；Hold = 显式持有）。
    pub exit: interp::ExitType,
}

/// G4 typed exit 组合层 trace（#134，裁定4 I_Θ 组合层雏形——G5 #124 升级 I_Θ 时在此层加
/// RiskState/TwState 输入与 tw_event/exit_kind 输出）。
///
/// 记录本步腿级生命周期事件（#145 T1：反向关闭的 ExitType 判定**前移到本组合层决策点**——
/// 入场角色 `entry_v` 经 [`TwStepCtx::entry_v`] 在飞映射取得，喂 [`interp::reverse_exit_type`]
/// 单源判据；消费端 runner 直接携带 trace 的裁决入 typed ledger，不再结算补算）：
/// - `closed`：被 interpret 规则2 的同级别已确认证书反向关闭的腿 + 触发证书（一一对应归因）+
///   typed 裁决；`nest_confirmed=false` 的信号由解释器归 record，不进入本通道。
/// - `silent_drops`：不在 close 桶但从 active 消失的腿（§13 AncOK 连带剪 / Stale prune）。
/// - `opened`：open 桶候选中**真正准入** `next_active` 的（AncOK 后），携对应新腿。
///   restore 恢复的祖先 carrier 腿不在此列（非信号入场，无 z，不入 ledger）。
///   ★#220 路④：准入判据 = 候选**自身元素 idx** ∈ next_active_idx（非 id 命中）——被 #216
///   规则①/③ 让位/湮灭的候选不外化；同 id restore 在场腿不得借候选配对入列（旧 id 配对
///   在同 bar 同 carrier 候选对 × restore 在场形态下一腿双登记，断言②门孤儿根因）。
///
/// **RiskExit 通道**（#124 P1 落地）：`force_flat`（PDF §7 全互斥 C_1 强平，屏蔽 P2..P10）⟹
/// [`pi_theta_step_traced`] 在解释器上游短路，prev_active 全部经 `risk_exits` 外化为
/// `ExitType::RiskExit`、next_active=∅（幽灵腿堵口，映射设计 §6.7）。RiskExit 无触发候选（非反向
/// 信号），故独立于 `closed`（后者携触发候选喂 [`interp::reverse_exit_type`]）。
#[derive(Debug, Clone, Default)]
pub(crate) struct StepTrace {
    /// 反向关闭三元组 `(被关腿, 同级别已确认触发证书, typed 裁决)`（#145 T1 / #146 T2）。裁决 =
    /// [`interp::reverse_exit_type`]
    /// (entry_v, trigger_class)，entry_v 从 [`TwStepCtx::entry_v`] 在飞映射取；`tw=None` 或腿不在
    /// 映射（非本窗信号入场/restore 祖先腿）⟹ **诚实回退** `Vertical::Ambient`（=非 ReverseOpen ⟹
    /// 按触发类派 P5/P6）——消费端 runner 只对在飞表登记腿入 ledger，登记腿必在映射 ⟹ 回退值
    /// 不进 typed ledger。逐笔一致性链（与原 runner 结算补算 bit-exact）：closed ⊆ prev_active ⊆
    /// 本 bar 快照 open_trades（同 bar 新开腿不可能当 bar 被规则2关闭）∧ entry_v 入场固定 ⟹
    /// 前移取值 = 旧补算值；runner 消费端有 debug_assert 守此不变量。
    pub closed: Vec<(ActiveLeg, Candidate, interp::ExitType)>,
    pub silent_drops: Vec<ActiveLeg>,
    pub opened: Vec<(Candidate, ActiveLeg)>,
    /// P1 强平清空的活动腿（force_flat ⟹ RiskExit）——无触发候选，独立通道。
    pub risk_exits: Vec<ActiveLeg>,
    /// P2 CloseOverlay 关闭的重叠腿（TW StageII ∧ H>0 ⟹ 关 legacy ReverseOpen 腿，PDF §7 C_2）
    /// ——无触发候选（TW 账本谓词驱动，非反向信号），独立于 `closed`；真产订单进同一
    /// schedule/fill/typed ledger（裁定4），消费端归 `ExitType::CloseReverseOpen`。
    pub overlay_closes: Vec<ActiveLeg>,
    /// P3/P4 TW 账本事件分量 `TWEvent_t`（PDF §16 四元组 `(D,O,L,TWEvent)`；裁定4：P3
    /// RecoverCapital / P4 EnterEarning 无订单，成立时**消耗当步裁决**——gamma 全部推迟
    /// record 桶，屏蔽 P5..P10）。账本推进（`tw_step`）由消费端 runner 单点做。
    pub tw_event: Option<TwEvent>,
    /// ★M5 逐声部目标头寸 `P^sep_{t+1}`（多空对冲.pdf p16 关卡10 `Σ_{v∈A_{t+1}}q_vσ_v e_v` 的分量）
    /// ——runner [`OverlayState`](super::super::overlay_state) hedge-mode 簿据此建持久逐声部账本 + ΔN 订单。
    /// 空 = 本 bar 无活动腿（force_flat/无候选/AncOK 全剪）⟹ P^sep_{t+1}=∅，Net=0。只读暴露，
    /// 不进决策路径（`Σσ_v·q_units == p̃` 恒等，见 [`coverage_step_from_buckets_sep`]）。
    pub sep_legs: Vec<SepLeg>,
    /// ★#201 阶段 B：显式 per-voice 裁决序列（含显式 Hold；**schema 冻结**，记录单元
    /// [`VoiceVerdict`]）。每 bar 每解释器裁决域持仓声部恰一枚（`prev_active = verdicts ⊎
    /// silent_drops` 划分：closed/risk/overlay 各携其 typed，延续持有 = Hold）。空 = 本 bar
    /// 无持仓声部。runner 消费段据此加裁决账本轨（加轨不减轨）；shadow 生产事实由本序列
    /// 单源推导（trace/裁决层统一）。
    pub verdicts: Vec<VoiceVerdict>,
    /// ★opsem-dump（R5-a，基因 073a/274号）：本步 LexArgmin 的 top-3 J_Θ 候选键（字典序升序，
    /// `(JThetaKey, control)`）。经 runner `OpsemEntrySnapshot.lex_top3` 透传至 dump 的
    /// `lex_argmin_top3` 字段。**不进 p_star/J_Θ/χ 门控**——纯只读诊断切片（R5-1 铁律：dump 数据
    /// 不进 μ 桶/J_Θ 排序/χ，生产 p_star 仍由 `lex_argmin` 单独决定）。空 vec = 本步无开仓路径
    /// （P1 强平/P2 关腿/P3-P4 无订单）⟹ opened 为空 ⟹ 不被 opsem_snap 消费。
    pub lex_top3: Vec<(JThetaKey, f64)>,
}

/// I_Θ 组合层 TW/风控上下文（#124 裁定4：解释器接口升级 `I_Θ(ctx: RiskState+TwState+LegBook,
/// gamma) -> {buckets, order_effect, tw_event, exit_kind}` 的 ctx 分量；分歧A 裁决——
/// [`interp::interpret`] 签名不变，本 ctx 只进组合层 [`pi_theta_step_traced`]）。
///
/// `None`（旧调用方）⟹ TW 谓词 P2/P3/P4 不评估（bit-exact 原路径）；生产 π fill loop 传
/// `Some`（TW 真值源 = fill loop 内逐 bar 推进的 [`TwState`]，裁定4：`run_closed_loop`
/// 降级纯结构验证工具）。
pub(crate) struct TwStepCtx<'a> {
    /// TW 账本态（生产真值源，fill loop 维护）。
    pub state: &'a TwState,
    /// barrier 政策（P3/P4 判据经 [`stage_progression`] 单源）。
    pub policy: &'a RiskPolicy,
    /// 当前风控模式（EnterReady 的 RiskNormal 门）。
    pub risk_mode: RiskMode,
    /// 在飞腿入场角色映射 `voice_id → entry_v`（runner 从 typed ledger 在飞表取——腿声部身份
    /// 入场固定，与 TW `open_legacy_legs` 计数同源同步）。双消费（#145 T1 升级原 shortdiff_leg_ids
    /// 半镜像）：① P2 CloseOverlay 过滤 `entry_v == ReverseOpen` 的重叠腿（语义 bit-exact——原
    /// HashSet 即按同判据在 runner 预滤）；② 反向关闭 typed 裁决的 entry_v 原料
    /// （[`interp::reverse_exit_type`] 单源，`StepTrace::closed` 第三分量）。
    pub entry_v: &'a std::collections::HashMap<ElementId, Vertical>,
    /// η 修正量（A10 C5 裁定 (b)，TW 桥 G1）：生产 π loop 的 `cum_holding_cost` i64 shadow
    /// （funding+borrow+liq 累计量化），经 [`stage_progression_eta_corrected`] 进 P4 EnterReady
    /// 的 η 左操作数（η_corrected = tw() − eta_correction）。**0 ⟹ 与历史判据同值 bit-exact**
    /// （回归锁）。★F4 同源约束：与 ZExt 第 15 维 η_bucket 的修正量同一变量（runner 单点喂两处）。
    pub eta_correction: i64,
}

/// [`pi_theta_step_prebuilt`] 的 trace 版（G4 #134 组合层）：同一决策路径（interpret →
/// coverage_step_from_buckets → pi_theta_position → schedule_order，单源无平行状态机）+
/// 腿级生命周期差分 [`StepTrace`]。
///
/// bit-exact 见证：`interpret_with_close_triggers(..).0 == interpret(..)`（interp.rs 委托单源），
/// 其余三环逐字调用同函数 ⟹ `(next_active, p*, order)` 与 prebuilt 完全一致。
#[allow(clippy::too_many_arguments)]
pub(crate) fn pi_theta_step_traced(
    work: ElementView,
    gamma: &[Candidate],
    prev_active: &[ActiveLeg],
    p_t: f64,
    exec_index: usize,
    base_units: f64,
    risk: &RiskConfig,
    weights: PiThetaWeights,
    gate: KThetaRiskGate,
    config: &VoiceConfig,
    registry: &super::super::persistent::PersistentRegistry,
    tw: Option<&TwStepCtx>,
    protocol: &ProtocolEventSet,
) -> (Vec<ActiveLeg>, f64, PiThetaDecision, StepTrace) {
    // 协议轨只读折叠，与下方 P1..P10 订单轨正交；所有 return 分支携同一显式事件。
    let protocol_event = protocol.selected();
    // P1 强平（PDF §7 全互斥 C_1=P_1 屏蔽 P2..P10）：force_flat ⟹ 活动腿全部 RiskExit 清空、无开仓、
    // 目标 flat。**在 interpret/coverage_step 上游短路**——open 桶不进 next_active（幽灵腿堵口 §6.7）、
    // held 逐条 RiskExit 经 StepTrace 外化（G4 预留通道兑现）。触发不依赖候选集非空（gamma 空也清仓，
    // 裁定4 P1 语义）。p_star bit-exact 原路径：force_flat ⟹ 𝒦_Θ={0} ⟹ pi_theta_position=0（与 p̃ 无关，
    // 见 `pi_theta_position_force_flat_gate_clamps_to_zero`）——仅 next_active 从含幽灵腿变 ∅（跨 bar 语义
    // 修正，非本 bar order 变）。P1 成立时 TW 谓词 P2/P3/P4 一并被屏蔽（本分支先于 tw 检查）。
    if gate.force_flat {
        let p_star = pi_theta_position(0.0, p_t, base_units, risk, weights, gate);
        let order = schedule_order(p_star, p_t, exec_index);
        return (
            Vec::new(),
            p_star,
            (order, protocol_event),
            StepTrace {
                risk_exits: prev_active.to_vec(),
                // #201：P1 分支每持仓声部恰一枚 RiskExit 裁决（prev_active 次序）。
                verdicts: prev_active
                    .iter()
                    .map(|&leg| VoiceVerdict { leg, exit: interp::ExitType::RiskExit })
                    .collect(),
                ..Default::default()
            },
        );
    }
    // ── TW 谓词 P2/P3/P4（#124 裁定4「真统一」：TW 三阶段进 fold，PDF §7 C_2/C_3/C_4）──
    // 优先级 P2 ≻ P3 ≻ P4 ≻ P5..P10；成立时**消耗当步裁决**（gamma 全部推迟 record 桶，屏蔽
    // P5..P10 的开/平；§13 结构剪枝 AncOK/Stale 照常——那是活动集与树的状态同步，非解释器裁决）。
    // P2/P3 判据天然互斥（P2 要求 StageII，P3 的 RecoverCapital 只在 StageI 派）；P4 的
    // enter_ready 要求 open_legacy_legs==0 ⟹ P2 成立（有重叠腿）时 P4 自动不成立——优先级链
    // 与账本合法性谓词一致。tw=None（旧调用方/无 TW 源）⟹ 本段跳过，bit-exact 原路径。
    if let Some(twc) = tw {
        // P2 CloseOverlay：TW StageII ∧ H>0（legacy ReverseOpen 重叠腿仍开）⟹ 关重叠腿。
        // 真产订单：合成 close 桶复用生产关腿路径（coverage_step_from_buckets 的 𝒟_x 通道 +
        // p̃ 重算 + LexArgmin + Schedule 全部原样单源），非平行订单机制。
        if twc.state.stage == TStage::CapitalRecovered {
            let overlay: Vec<ActiveLeg> = prev_active
                .iter()
                .filter(|l| twc.entry_v.get(&l.id) == Some(&Vertical::ReverseOpen))
                .copied()
                .collect();
            if !overlay.is_empty() {
                let buckets = Buckets {
                    close: overlay.clone(),
                    open: Vec::new(),
                    record: gamma.to_vec(), // 当步普通候选推迟记录（消耗当步裁决）
                };
                let (next_active, p_tilde, sep_legs, _idx) = coverage_step_from_buckets_sep(
                    work, prev_active, &buckets, base_units, config, Some(risk), registry,
                );
                let p_star = pi_theta_position(p_tilde, p_t, base_units, risk, weights, gate);
                let order = schedule_order(p_star, p_t, exec_index);
                // §13 结构剪枝（AncOK/Stale）照常 ⟹ 被剪腿仍须外化（消费端在飞表不泄漏）。
                let next_ids: std::collections::HashSet<ElementId> =
                    next_active.iter().map(|l| l.id).collect();
                let overlay_ids: std::collections::HashSet<ElementId> =
                    overlay.iter().map(|l| l.id).collect();
                let silent_drops = prev_active
                    .iter()
                    .filter(|l| !overlay_ids.contains(&l.id) && !next_ids.contains(&l.id))
                    .copied()
                    .collect();
                // #201：P2 分支每持仓声部恰一枚裁决——overlay 腿 = CloseReverseOpen（与规则2
                // 短差关闭同 typed，源头区分在 overlay_closes 桶）、保留腿 = Hold；
                // prev_active 次序；§13 剪除腿非裁决（silent_drops 轨）。
                let verdicts = prev_active
                    .iter()
                    .filter_map(|l| {
                        if overlay_ids.contains(&l.id) {
                            Some(VoiceVerdict { leg: *l, exit: interp::ExitType::CloseReverseOpen })
                        } else if next_ids.contains(&l.id) {
                            Some(VoiceVerdict { leg: *l, exit: interp::ExitType::Hold })
                        } else {
                            None
                        }
                    })
                    .collect();
                return (
                    next_active,
                    p_star,
                    (order, protocol_event),
                    StepTrace { overlay_closes: overlay, silent_drops, sep_legs, verdicts, ..Default::default() },
                );
            }
        }
        // P3 RecoverCapital / P4 EnterEarning：无订单账本事件（stage_progression 单源判据，
        // 与 closed_loop 结构验证共用同一函数——不镜像）。输出 TWEvent_t 分量；账本推进
        // （tw_step）由消费端 runner 单点做（组合层只读 ctx，不 mutate 账本）。
        // ★A10 C5：η 修正经 stage_progression_eta_corrected（twc.eta_correction =
        // cum_holding_cost shadow；0 ⟹ bit-exact）。
        if let Some(ev) = stage_progression_eta_corrected(twc.policy, twc.state, twc.risk_mode, twc.eta_correction) {
            let buckets = Buckets {
                close: Vec::new(),
                open: Vec::new(),
                record: gamma.to_vec(), // 当步普通候选推迟记录（消耗当步裁决，屏蔽 P5..P10）
            };
            let (next_active, p_tilde, sep_legs, _idx) = coverage_step_from_buckets_sep(
                work, prev_active, &buckets, base_units, config, Some(risk), registry,
            );
            let p_star = pi_theta_position(p_tilde, p_t, base_units, risk, weights, gate);
            let order = schedule_order(p_star, p_t, exec_index);
            // §13 结构剪枝（AncOK/Stale）照常 ⟹ 被剪腿仍须外化（消费端在飞表不泄漏）。
            let next_ids: std::collections::HashSet<ElementId> =
                next_active.iter().map(|l| l.id).collect();
            let silent_drops = prev_active
                .iter()
                .filter(|l| !next_ids.contains(&l.id))
                .copied()
                .collect();
            // #201：P3/P4 分支消耗当步裁决（屏蔽 P5..P10）——持仓声部仍逐枚裁 Hold
            // （活动腿保持）；§13 剪除腿非裁决（silent_drops 轨）。
            let verdicts = prev_active
                .iter()
                .filter(|l| next_ids.contains(&l.id))
                .map(|&leg| VoiceVerdict { leg, exit: interp::ExitType::Hold })
                .collect();
            return (
                next_active,
                p_star,
                (order, protocol_event),
                StepTrace { tw_event: Some(ev), silent_drops, sep_legs, verdicts, ..Default::default() },
            );
        }
    }
    // 环5：解释器三桶 + close 触发归因（fold 单源，interp.rs）。
    // ★#202 阶段 C（spec WP-3「仅替换 P2/P3」）：本级证书平仓域（channel 口径 P2/P3 =
    // entry_v≠ReverseOpen 腿的 CloseRoot/ReduceCore）改经 channel 判据
    // [`super::super::channel::cert_close_trigger`] **逐腿**裁决——「每声部每步一枚」替代散装
    // fold 规则2 的候选消费粒度（find_reverse+reverse_exit_type 与规则2 同单源判据；
    // 多腿/多候选场景两链裁决结构不同，票面明知非 bit-exact）。channel 只出裁决不建腿：
    // 开仓/记录/其余通道维持散装——规则2 外部化 fold 变体
    // [`interp::interpret_with_external_closes`] 承接（S 组 ReverseOpen 腿 = P4 域候选驱动
    // 原样、规则3/4 开仓原样、#200 二类 dual-effect 原样）。生产侧建腿（环6/7）零改。
    let entry_v_of = |l: &ActiveLeg| {
        tw.and_then(|t| t.entry_v.get(&l.id).copied())
            .unwrap_or(Vertical::Ambient)
    };
    let external_closes: Vec<(usize, Candidate)> = prev_active
        .iter()
        .enumerate()
        .filter_map(|(i, l)| {
            let entry_v = entry_v_of(l);
            if entry_v == Vertical::ReverseOpen {
                return None; // S 组（P4 域）：维持散装 fold 规则2 候选消费现状。
            }
            super::super::channel::cert_close_trigger(l.level, l.dir, entry_v, gamma)
                .map(|(trigger, _exit)| (i, *trigger))
        })
        .collect();
    let (buckets, close_triggers) =
        interp::interpret_with_external_closes(gamma, prev_active, &external_closes);
    // candidate_start（候选段基址）先抓——work 即将 move 进 coverage_step_from_buckets_sep；
    // opened 外化的配对键（候选自身元素 idx，#220 路④）以它为基准。
    let candidate_start = work.base_len();
    // 环6：活动集递归 + AncOK + G7 毛约束（原样单源）。★M5：sep 出口暴露逐声部 P^sep_{t+1}。
    // ★#220 路④：第 4 分量 next_active_idx（与 next_active 逐位对位的 work idx）= opened 配对键。
    let (next_active, p_tilde, sep_legs, next_active_idx) =
        coverage_step_from_buckets_sep(work, prev_active, &buckets, base_units, config, Some(risk), registry);
    // 环7：LexArgmin + Schedule（原样单源）。
    let p_star = pi_theta_position(p_tilde, p_t, base_units, risk, weights, gate);
    let order = schedule_order(p_star, p_t, exec_index);

    // ── trace 差分（决策已定，纯只读观测）──
    let next_ids: std::collections::HashSet<ElementId> =
        next_active.iter().map(|l| l.id).collect();
    // #145 T1 typed 裁决（组合层单点）：entry_v 从在飞映射取；tw=None/腿不在映射 ⟹ 回退
    // Ambient（诚实语义见 `StepTrace::closed` doc——回退值不被 ledger 消费）。#202：C 组腿
    // 的 typed 与 channel 裁决同单源（reverse_exit_type(entry_v, trigger.class) 逐字同判据）。
    let closed: Vec<(ActiveLeg, Candidate, interp::ExitType)> = buckets
        .close
        .iter()
        .copied()
        .zip(close_triggers)
        .map(|(l, c)| {
            let exit_type = interp::reverse_exit_type(entry_v_of(&l), c.bsp_class);
            (l, c, exit_type)
        })
        .collect();
    // ★#199 断言①（#185「建议断言」1，T1 目标态）：一类批 ⟹ target_qty(Core{L} 批次方向侧)==0——
    // 一类核心关闭后 sep_legs 无该级该方向侧核心目标。账户身份经 #197 `account::identity_of`
    // 单源判定（Ambient×Long/FollowParent ⟹ Core{level}；反手开空=Ambient×Short ⟹ Short
    // 账不触发本断言，ID-3「允许当场反手」相容）。探针恒在计数（一类核心关闭评估笔数）。
    //
    // ★#237 按方向拆查（#234 蓝图依据：出场方向锁定「多头声部由卖点证书平仓；空头声部由
    // 买点证书平仓」买卖点2.pdf p.4/14 §5.2 ＋ 分侧账禁净额 P_sep，拆查为唯一一致查法；
    // 用户裁 2026-07-24）：一类卖批 ⇒ 查该级**多侧**分量=0；一类买批 ⇒ 查该级**空侧**
    // 分量=0——S2「该级该方向」的良构形式。批次方向由被关腿方向单源表达（Exit_v
    // δ(γ)=−σ_v：卖批关 Long 腿、买批关 Short 腿 ⟺ l.dir 即批次方向）。顺父级联 Short 腿
    // （FollowParent×Short）归空侧分量：一类卖批下合法存活（同向信号持有/加仓/记录，
    // 蓝图动作表），不计入卖批残余——m3 win9 bar=232810 (1,470) 误报面收口（#233 §5 另案，
    // 本票）；其合法出场路径 = 一类买点/元素终结/父关连清/风险强平（#234 Q4.3）。
    //
    // ★#209 终态硬门（用户裁 A 2026-07-23：S7 级别内全平是必须非应当）：#199 测量态
    // 期满转正——fold 规则2 已修为一类候选关闭该级别**全部**反向命中腿（interp.rs
    // #209 段），批次方向侧级残余结构性归零。硬门形态 = debug 构建逐笔 panic + 违例探针恒在
    // 计数（release 下仍可观测），与断言③同款。历史对照（勿删）：#199 测量态 BTC
    // train 窗实测违例=1（L=1 残留 FollowParent 延续腿 277.9 单位，一类只关首条所致）。
    for (l, c, _) in &closed {
        if c.bsp_class != 1 {
            continue;
        }
        let entry_v = entry_v_of(l);
        if account::identity_of(entry_v, l.dir, l.level)
            != Some(account::AccountIdentity::Core { level: l.level })
        {
            continue;
        }
        t1_target_zero_probe_bump();
        let residual: f64 = sep_legs
            .iter()
            .filter(|s| s.id.level == l.level)
            // ★#237 按方向拆查：只计批次方向侧分量（l.dir ⟺ 批次方向，Exit_v 方向锁定的
            // 生产同义式）；异侧腿不在本批出场域（卖批不关空侧、买批不关多侧）。
            .filter(|s| s.side == l.dir)
            .filter(|s| {
                account::identity_of(s.role_v, s.side, s.id.level)
                    == Some(account::AccountIdentity::Core { level: s.id.level })
            })
            .map(|s| s.q_units)
            .sum();
        if residual != 0.0 {
            t1_target_residual_probe_bump();
        }
        debug_assert!(
            residual == 0.0,
            "#237 断言①终态违例：一类批后 target_qty(Core{{{}}} {:?}侧) 残余 {residual} ≠ 0——S7 级别内全平是必须（按方向拆查）",
            l.level, l.dir
        );
    }
    let closed_ids: std::collections::HashSet<ElementId> =
        closed.iter().map(|(l, _, _)| l.id).collect();
    // #201：正常路径每持仓声部恰一枚显式裁决——close 桶腿携组合层单源 typed（与 closed
    // 第三分量一致），延续腿 = Hold（由隐式转显式）；prev_active 次序确定序；§13 剪除腿
    // 非裁决（silent_drops 轨，见下）。
    let closed_typed: std::collections::HashMap<ElementId, interp::ExitType> =
        closed.iter().map(|(l, _, e)| (l.id, *e)).collect();
    let verdicts: Vec<VoiceVerdict> = prev_active
        .iter()
        .filter_map(|l| {
            if let Some(&exit) = closed_typed.get(&l.id) {
                Some(VoiceVerdict { leg: *l, exit })
            } else if next_ids.contains(&l.id) {
                Some(VoiceVerdict { leg: *l, exit: interp::ExitType::Hold })
            } else {
                None
            }
        })
        .collect();
    // 静默离场：prev_active 中既未被 close 桶认领、也不在 next_active（AncOK 剪/Stale prune）。
    let silent_drops: Vec<ActiveLeg> = prev_active
        .iter()
        .filter(|l| !closed_ids.contains(&l.id) && !next_ids.contains(&l.id))
        .copied()
        .collect();
    // ★#220 路④：真正准入的 open 候选 = 其**自身元素 idx**（candidate_start+gamma_index）出现在
    // next_active_idx（AncOK 未剪且未被 gross 零化）。旧「id ∈ next_active 即配对」在同 bar 同
    // carrier 候选对 × restore 在场形态下，把同一条 restore 腿配给两个被 #216 规则①让位的候选
    // ⟹ opened ×2 ⟹ runner 双重登记（open_trades.insert 覆盖 + 双份镜像开仓）⟹ 物理平仓只消费
    // 最新条目，先注册实例成永不消账孤儿（m3/m6 炸断言②门 Core{1}=1296.87 根因，勘察「炸点
    // 实证」定案）。idx 配对下被 #216 ①/③ 跳过/湮灭的候选自身元素从未入 raw，自然不外化——
    // 恢复「opened ⟺ 信号真实准入」与「restore 腿不入账」的设计意图自洽（runner 零改）。
    let opened: Vec<(Candidate, ActiveLeg)> = buckets
        .open
        .iter()
        .filter_map(|c| {
            let idx = candidate_start + c.gamma_index;
            next_active_idx
                .iter()
                .position(|&i| i == idx)
                .map(|j| (*c, next_active[j]))
        })
        .collect();
    // ★#220 机器锁定（A9 不变量断言化）：同 bar opened 不得含重复 ElementId——一条物理腿恰一条
    // 账面实例；重复 = 双重外化回归（runner open_trades.insert 覆盖 + 镜像孤儿之源）。
    debug_assert!(
        {
            let mut ids: Vec<_> = opened.iter().map(|(_, l)| l.id).collect();
            ids.sort_by_key(|id| (id.level, id.ordinal));
            ids.windows(2).all(|w| w[0] != w[1])
        },
        "#220：同 bar opened 含重复 ElementId ⟹ 一条物理腿被双重登记（opened 配对键须为候选自身元素 idx）"
    );

    // ★R5-a opsem-dump：LexArgmin top-3 J_Θ 切片。仅本步有开仓时算（无开仓 bar 零开销；6 候选排序
    // O(1)）。复用 [`feasible_lex_candidates`]（与 pi_theta_position 同源候选集）⟹ top-3 首名 ≡ p_star
    // 选址。不进 p_star/J_Θ/χ——纯只读诊断（R5-1 铁律），生产决策由上方 pi_theta_position 单独定。
    let lex_top3 = if opened.is_empty() {
        Vec::new()
    } else {
        lex_argmin_top_k(&feasible_lex_candidates(p_tilde, p_t, base_units, risk, weights, gate), 3)
    };

    (
        next_active,
        p_star,
        (order, protocol_event),
        StepTrace { closed, silent_drops, opened, sep_legs, verdicts, lex_top3, ..Default::default() },
    )
}


#[cfg(test)]
#[path = "compose_tests_1.rs"]
mod tests_1;

#[cfg(test)]
#[path = "compose_tests_2.rs"]
mod tests_2;
