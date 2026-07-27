use super::*;
use super::super::test_support::*;
use crate::theta_v0::types::{Center, BspBits};
use crate::theta_v0::classifier::LevelState;
use crate::theta_v0::classifier::bsp::BspPoint;

    /// ★#202 对照锁（散装域维持现状）：S 组腿（entry_v==ReverseOpen）的关闭**不**经
    /// channel 判据——P4 域维持散装 fold 规则2（typed=CloseReverseOpen 由 #145 T1 既有
    /// 测试锁）；本锁钉死「仅替换 P2/P3」边界：一类候选场景 channel 判据与 #209 fold
    /// 全平同效（每腿独立裁 CloseRoot ⟺ fold 一类关全部），该域 bit-exact 不翻。
    #[test]
    fn p23_channel_type1_multi_leg_bit_exact_with_fold() {
        use super::super::super::ledger::{RiskPolicy, TwState};
        let tower = rc_tower(vec![]);
        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        let reg = super::super::super::persistent::PersistentRegistry::new();
        let held_root = aleg(0, VoiceSide::Long, 0, 0);
        let held_cascade = aleg(0, VoiceSide::Long, 2, 2);
        let classification = sell_classification(1); // 一类：两链全平同效域
        let (tree, candidates, gamma) =
            interp::coverage_elements_and_gamma_with_tower(&classification, &tower);
        let work = ElementView::from_parts(&tree, candidates);
        let tw_state = TwState::initial();
        let policy = RiskPolicy::baseline();
        let entry_v: std::collections::HashMap<ElementId, Vertical> = [
            (held_root.id, Vertical::Ambient),
            (held_cascade.id, Vertical::FollowParent),
        ]
        .into_iter()
        .collect();
        let twc = TwStepCtx {
            state: &tw_state,
            policy: &policy,
            risk_mode: RiskMode::Normal,
            entry_v: &entry_v,
            eta_correction: 0,
        };
        let (na, _ps, _o, trace) = pi_theta_step_traced(
            work, &gamma, &[held_root, held_cascade], 600.0, 11, 1000.0, &r, w,
            KThetaRiskGate::open(), &cfg(), &reg, Some(&twc), &protocol_hold(),
        );
        assert_eq!(trace.closed.len(), 2, "一类多腿：channel 判据与 #209 全平同效（bit-exact 域）");
        assert!(
            trace.closed.iter().all(|(_, trig, exit)| {
                trig.bsp_class == 1 && *exit == interp::ExitType::CloseRoot
            }),
            "两腿归因同一一类候选、typed 均 CloseRoot（P2 域）"
        );
        assert!(!na.iter().any(|l| l.id == held_root.id || l.id == held_cascade.id));
        // 归因序保持 active 次序（同候选内 #209 push 序同构）。
        assert_eq!(trace.closed[0].0.id, held_root.id);
        assert_eq!(trace.closed[1].0.id, held_cascade.id);
    }

    /// ★#146 T2 验收1（原始烤料，端到端）：L1 声部 A 持仓 + L0 反向已确认证书属于另一未持有
    /// L1 声部 B。解释器只按递归结构 `level` 匹配关闭，故 L0 不得关闭 L1；B 的父 carrier 未持有，
    /// §13 AncOK 又会拒绝其开腿，最终显式 Hold、目标仓位与订单均零变动。
    #[test]
    fn t2_cross_level_confirmed_certificate_holds_l1_position_end_to_end() {
        use super::super::super::channel::{self, ChannelDecision, VoiceState, VoiceStepInput};

        let tower = two_parent_tower();
        // source=20 命中 compose_b 的 L0 子 b1；卖向与 L1 Long 持仓 A 反向，但 level=0≠1。
        let classification = sell_classification_at(0, 20, 1);
        let (tree, candidates, gamma) =
            interp::coverage_elements_and_gamma_with_tower(&classification, &tower);
        assert_eq!(gamma.len(), 1);
        assert_eq!(gamma[0].level, 0);
        assert!(gamma[0].nest_confirmed, "L0 卖点携 Conf^-，是已确认证书");

        let held = held_l1_compose_a();
        let held_idx = tree.iter().position(|e| e.id == held.id).expect("L1 compose_a 在塔中");
        let held_target = leg_target(&tree, held_idx, 1000.0, &cfg());
        assert_eq!(held_target.side, VoiceSide::Long);
        let p_t = held_target.units;

        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        let reg = super::super::super::persistent::PersistentRegistry::new();
        let work = ElementView::from_parts(&tree, candidates);
        let (next_active, p_star, (order, _protocol), trace) = pi_theta_step_traced(
            work, &gamma, &[held], p_t, 21, 1000.0, &r, w, KThetaRiskGate::open(),
            &cfg(), &reg, None, &protocol_hold(),
        );

        assert!(trace.closed.is_empty(), "L0 证书不得进入 L1 的 close/typed-exit 通道");
        assert!(trace.opened.is_empty(), "未持有 compose_b 父声部，L0 子候选被 §13 AncOK 拒绝");
        assert_eq!(next_active, vec![held], "L1 持仓腿原样延续");
        assert_eq!(p_star, p_t, "目标净仓不变");
        assert_eq!((order.action, order.qty), (StrictAction::Hold, 0), "显式 Hold 且仓位零变动");

        // #147 T3 只读交叉：同一声部输入仍落 C0/Hold，不改变 P1-P8 语义。
        let voice = VoiceState {
            level: held.level,
            leg: Some(held),
            entry_v: Vertical::Ambient,
            step: 0,
            sub_cycle: channel::SubCycleTracker::default(),
            reverse_open: None,
        };
        let input = VoiceStepInput {
            force_flat: false,
            candidates: gamma,
            parent_kappa: channel::ParentKappa::Unknown,
            parent_projections: Vec::new(),
        };
        assert_eq!(
            channel::step_voice(&voice, &input).1,
            ChannelDecision::Exit(interp::ExitType::Hold),
            "跨级证书不进入 L1 声部出场谓词"
        );
    }

    /// ★#146 T2 验收2：同级别已确认反向证书正常进入 `StepTrace.closed`，typed 裁决严格复用
    /// [`interp::reverse_exit_type`]：一类→CloseRoot、三类→ReduceCore。
    #[test]
    fn t2_same_level_confirmed_certificate_matches_t1_exit_split() {
        let held = held_l1_compose_a();
        for class in [1u8, 3u8] {
            let tower = two_parent_tower();
            // L1 source=12 命中 compose_a；与 held 同一递归结构级别。
            let classification = sell_classification_at(1, 12, class);
            let (tree, candidates, gamma) =
                interp::coverage_elements_and_gamma_with_tower(&classification, &tower);
            assert!(gamma[0].nest_confirmed, "同级别触发者必须是已确认证书");
            let held_idx = tree.iter().position(|e| e.id == held.id).expect("L1 compose_a 在塔中");
            let p_t = leg_target(&tree, held_idx, 1000.0, &cfg()).units;
            let r = rcfg();
            let w = PiThetaWeights::from_risk(&r);
            let reg = super::super::super::persistent::PersistentRegistry::new();
            let work = ElementView::from_parts(&tree, candidates);
            let (_next, _p_star, (_order, _protocol), trace) = pi_theta_step_traced(
                work, &gamma, &[held], p_t, 13, 1000.0, &r, w, KThetaRiskGate::open(),
                &cfg(), &reg, None, &protocol_hold(),
            );

            assert_eq!(trace.closed.len(), 1, "同级别已确认反向证书必须关闭持仓腿（class={class}）");
            let (leg, trigger, decision) = trace.closed[0];
            assert_eq!(leg.id, held.id);
            assert_eq!(trigger.level, held.level, "被关腿与证书必须同级别");
            assert_eq!(
                decision,
                interp::reverse_exit_type(Vertical::Ambient, class),
                "#146 T2 不改 #145 T1 的 typed 二分（class={class}）"
            );
        }
    }

    /// ★#146 T2 验收3：方向信号若 `nest_confirmed=false`，不是 §9 closePred 可消费的证书；必须
    /// 留在 𝒦_x record，不进 close 桶，也不得在组合层产生任何 typed 出场裁决或仓位变化。
    #[test]
    fn t2_unconfirmed_same_level_signal_records_without_exit_decision() {
        let tower = two_parent_tower();
        let classification = sell_classification_at(1, 12, 1);
        let (tree, candidates, mut gamma) =
            interp::coverage_elements_and_gamma_with_tower(&classification, &tower);
        gamma[0].nest_confirmed = false; // 仅方向信号：证书成立层明确否定 N^δ。
        let held = held_l1_compose_a();

        let (buckets, close_triggers) = interp::interpret_with_close_triggers(&gamma, &[held]);
        assert!(buckets.close.is_empty(), "未确认信号不得进入 close 桶");
        assert!(close_triggers.is_empty(), "未确认信号不得成为 typed exit 原料");
        assert!(buckets.open.is_empty(), "未成立证书不得借规则3反向开同 carrier");
        assert_eq!(buckets.record.len(), 1, "未确认信号保留在 𝒦_x，记录但不执行");

        let held_idx = tree.iter().position(|e| e.id == held.id).expect("L1 compose_a 在塔中");
        let p_t = leg_target(&tree, held_idx, 1000.0, &cfg()).units;
        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        let reg = super::super::super::persistent::PersistentRegistry::new();
        let work = ElementView::from_parts(&tree, candidates);
        let (next_active, p_star, (order, _protocol), trace) = pi_theta_step_traced(
            work, &gamma, &[held], p_t, 13, 1000.0, &r, w, KThetaRiskGate::open(),
            &cfg(), &reg, None, &protocol_hold(),
        );
        assert!(trace.closed.is_empty(), "StepTrace 不得生成 CloseRoot/ReduceCore/CloseReverseOpen");
        assert!(trace.opened.is_empty(), "未确认信号只记录，不开腿");
        assert_eq!(next_active, vec![held]);
        assert_eq!(p_star, p_t);
        assert_eq!((order.action, order.qty), (StrictAction::Hold, 0));
    }

    /// ★#145 T1：entry_v=ReverseOpen 压过触发类——TwStepCtx 在飞 entry_v 映射携 ReverseOpen ⟹
    /// 一类触发仍派 CloseReverseOpen（P7 子声部关闭语义压过触发信号语义，reverse_exit_type 单源）。
    #[test]
    fn pi_theta_step_traced_typed_close_reverse_open_overrides_trigger() {
        use super::super::super::ledger::{RiskPolicy, TwState};
        let classification = sell_classification(1);
        let tower = rc_tower(vec![]);
        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        let reg = super::super::super::persistent::PersistentRegistry::new();
        let (tree, candidates, gamma) =
            interp::coverage_elements_and_gamma_with_tower(&classification, &tower);
        let work = ElementView::from_parts(&tree, candidates);
        let held = aleg(0, VoiceSide::Long, 0, 0);
        let tw_state = TwState::initial(); // inert：P2/P3/P4 不成立，落普通 fold 路径
        let policy = RiskPolicy::baseline();
        let entry_v: std::collections::HashMap<ElementId, Vertical> =
            [(held.id, Vertical::ReverseOpen)].into_iter().collect();
        let twc = TwStepCtx {
            state: &tw_state,
            policy: &policy,
            risk_mode: RiskMode::Normal,
            entry_v: &entry_v,
            eta_correction: 0,
        };
        let (_na, _ps, _o, trace) = pi_theta_step_traced(
            work, &gamma, &[held], 600.0, 11, 1000.0, &r, w, KThetaRiskGate::open(),
            &cfg(), &reg, Some(&twc), &protocol_hold(),
        );
        assert_eq!(trace.closed.len(), 1, "一类卖候选关闭持仓 Long 腿");
        let (leg, trig, exit_type) = &trace.closed[0];
        assert_eq!(leg.id, held.id);
        assert_eq!(trig.bsp_class, 1, "触发类=1（若非 ReverseOpen 会派 CloseRoot）");
        assert_eq!(
            *exit_type,
            interp::ExitType::CloseReverseOpen,
            "entry_v=ReverseOpen ⟹ P7 CloseReverseOpen（压过触发类）"
        );
    }

    /// ★#124 P1 强平（PDF §7 全互斥 C_1 屏蔽 P2..P10）：force_flat ⟹ 活动腿全部 RiskExit 清空、
    /// next_active=∅、无 open/close/silent（幽灵腿堵口 §6.7）。持仓 Long + 同 bar 买候选（正常 P8 会 open）
    /// ⟹ 均被 P1 屏蔽，open 不入 next_active（否则跨 bar 幽灵腿）。
    #[test]
    fn pi_theta_step_traced_p1_force_flat_risk_exits_all() {
        let buy = BspPoint { level_origin: 0,
            source_index: 4,
            bits: BspBits { buy1: true, ..Default::default() },
            pivot_low: 90,
            pivot_high: 0,
            center: Some(crate::theta_v0::classifier::bsp::OwnerRef::Center(Center { zd: 100, zg: 200, dd: 90, gg: 210, start_index: 0, end_index: 9 })),
            struct_break_dir: None,
            force: None,
        };
        let classification = Classification {
            levels: vec![LevelState { bsp: Rc::new(vec![buy]), ..Default::default() }],
        };
        let tower = rc_tower(vec![]);
        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        let reg = super::super::super::persistent::PersistentRegistry::new();
        let (tree, candidates, gamma) =
            interp::coverage_elements_and_gamma_with_tower(&classification, &tower);
        let work = ElementView::from_parts(&tree, candidates);
        let held = aleg(0, VoiceSide::Long, 0, 0);
        let flat = KThetaRiskGate { force_flat: true, stop_long: false, stop_short: false, no_increase_cap: None };
        let (next_active, p_star, _decision, trace) = pi_theta_step_traced(
            work, &gamma, &[held], 600.0, 11, 1000.0, &r, w, flat, &cfg(), &reg, None,
            &protocol_hold(),
        );
        assert!(next_active.is_empty(), "P1 强平 ⟹ next_active=∅（open 不入账 = 幽灵腿堵口）");
        assert_eq!(trace.risk_exits.len(), 1, "prev_active 全部 RiskExit（无触发候选）");
        assert_eq!(trace.risk_exits[0].id, held.id);
        assert!(
            trace.closed.is_empty() && trace.opened.is_empty() && trace.silent_drops.is_empty(),
            "P1 屏蔽 P2..P10：无 close/open/silent 分量"
        );
        assert_eq!(p_star, 0.0, "force_flat ⟹ 𝒦_Θ={{0}} ⟹ p*=0");
    }

    // ── #124 裁定4 TW 谓词 P2/P3/P4 组合层分支（L0/L1：分支逻辑正确性；生产触发可达性在
    //    codex GAP3 裁定 A' 后现实可达——已实现利润经 Realize 入 free，见 runner
    //    `pi_loop_realized_profit_reaches_earning_shares`）。★A' 清单⑤：P2/P3/P4 成立时
    //    **消耗当步裁决**（gamma 推迟 record）不仅改 trace 桶归属，还**真改同 bar 订单流**
    //    ——各测试以 tw=None 对照断言 Order 本身不同（masking 的 bit-exact 风险载体，
    //    #135 重跑清单第二类风险：P3/P4 无订单事件但间接改普通开平仓订单）。──

    /// ★P2 CloseOverlay（PDF §7 C_2）：TW StageII ∧ 持 legacy ReverseOpen 腿 ⟹ 关重叠腿
    /// （overlay_closes），普通买候选被消耗当步裁决（不开仓）；非 ReverseOpen 腿保留。
    #[test]
    fn pi_theta_step_traced_p2_close_overlay() {
        use super::super::super::ledger::{RiskPolicy, TStage, TwState};
        let (classification, tower) = buy_gamma();
        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        let reg = super::super::super::persistent::PersistentRegistry::new();
        let (tree, candidates, gamma) =
            interp::coverage_elements_and_gamma_with_tower(&classification, &tower);
        let work = ElementView::from_parts(&tree, candidates.clone());
        // legacy ReverseOpen 重叠腿。dir 取 Long：使 tw=None 对照的 p̃ 不落在「候选开仓 +600 与
        // Short 腿 −600 恰好抵消」的巧合点上（P2 关腿 vs 对照保腿的订单差异可观测）。
        let sd_leg = aleg(0, VoiceSide::Long, 7, 7);
        let root_leg = aleg(1, VoiceSide::Long, 3, 3); // 非重叠根腿（保留）
        let sd_ids: std::collections::HashMap<ElementId, Vertical> =
            [(sd_leg.id, Vertical::ReverseOpen)].into_iter().collect();
        let tw_state = TwState {
            stage: TStage::CapitalRecovered,
            open_legacy_legs: 1,
            ..TwState::initial()
        };
        let policy = RiskPolicy::baseline();
        let twc = TwStepCtx {
            state: &tw_state,
            policy: &policy,
            risk_mode: RiskMode::Normal,
            entry_v: &sd_ids,
            eta_correction: 0, // A10 C5：零修正 = 历史判据 bit-exact（测试基准口径）
        };
        let (next_active, _ps, (order_p2, _), trace) = pi_theta_step_traced(
            work, &gamma, &[sd_leg, root_leg], 0.0, 11, 1000.0, &r, w,
            KThetaRiskGate::open(), &cfg(), &reg, Some(&twc),
            &protocol_hold(),
        );
        assert_eq!(trace.overlay_closes.len(), 1, "P2 关重叠腿恰一条");
        assert_eq!(trace.overlay_closes[0].id, sd_leg.id);
        assert!(!next_active.iter().any(|l| l.id == sd_leg.id), "重叠腿不入 next_active");
        assert!(next_active.iter().any(|l| l.id == root_leg.id), "非重叠根腿保留");
        assert!(trace.opened.is_empty(), "P2 消耗当步裁决 ⟹ 买候选不开仓（屏蔽 P8）");
        assert!(trace.tw_event.is_none(), "P2 屏蔽 P3/P4：无 TW 事件");
        // ★A' 清单⑤（masking 真改订单流）：同输入 tw=None 走普通路径（买候选开仓 + 无 overlay
        // 关腿）⟹ Order 与 P2 路径**不同**——P2 直接产订单是 #135 重跑清单的第一类 bit-exact 风险。
        let work_none = ElementView::from_parts(&tree, candidates);
        let (_na, _ps2, (order_none, _), _tr) = pi_theta_step_traced(
            work_none, &gamma, &[sd_leg, root_leg], 0.0, 11, 1000.0, &r, w,
            KThetaRiskGate::open(), &cfg(), &reg, None,
            &protocol_hold(),
        );
        assert_ne!(order_p2, order_none, "P2 CloseOverlay 真改同 bar 订单（非仅 trace 差异）");
    }

    /// ★P3 RecoverCapital（PDF §7 C_3）：TW CostReduction ∧ holding≥notional_in ∧ free 足额
    /// ⟹ tw_event=RecoverCapital（无订单），普通候选消耗当步裁决（不开仓），活动腿保持。
    #[test]
    fn pi_theta_step_traced_p3_withdraw_consumes_step() {
        use super::super::super::ledger::{RiskPolicy, TwEvent, TwState};
        let (classification, tower) = buy_gamma();
        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        let reg = super::super::super::persistent::PersistentRegistry::new();
        let (tree, candidates, gamma) =
            interp::coverage_elements_and_gamma_with_tower(&classification, &tower);
        let work = ElementView::from_parts(&tree, candidates.clone());
        let tw_state = TwState {
            free: 100,
            holding: 100,
            notional_in: 100,
            ..TwState::initial()
        }; // CostReduction + holding≥notional_in + free≥recover_target=100 ⟹ P3 成立
        let policy = RiskPolicy::baseline();
        let empty_ids: std::collections::HashMap<ElementId, Vertical> = Default::default();
        let twc = TwStepCtx {
            state: &tw_state,
            policy: &policy,
            risk_mode: RiskMode::Normal,
            entry_v: &empty_ids,
            eta_correction: 0, // A10 C5：零修正 = 历史判据 bit-exact（测试基准口径）
        };
        let (next_active, _ps, (order_p3, _), trace) = pi_theta_step_traced(
            work, &gamma, &[], 0.0, 11, 1000.0, &r, w,
            KThetaRiskGate::open(), &cfg(), &reg, Some(&twc),
            &protocol_hold(),
        );
        assert_eq!(
            trace.tw_event,
            Some(TwEvent::RecoverCapital(100)),
            "P3 成立 ⟹ TWEvent_t=RecoverCapital(足额退本金目标)"
        );
        assert!(trace.opened.is_empty(), "P3 消耗当步裁决 ⟹ 买候选不开仓（屏蔽 P5..P10）");
        assert!(next_active.is_empty(), "无持仓腿 ⟹ next_active 空（无开仓）");
        // ★A' 清单⑤（P3 masking 真改订单流）：P3 无订单账本事件，但消耗当步裁决使同 bar 普通
        // 开仓被推迟 ⟹ 订单 qty=0；tw=None 对照下买候选正常开仓 qty>0——「P3/P4 无直接订单但
        // 间接改订单流」正是 codex §4 修正认定的第二类 bit-exact 风险（#135 重跑清单）。
        assert_eq!(order_p3.qty, 0, "P3 屏蔽开仓 ⟹ 本 bar 无订单量");
        let work_none = ElementView::from_parts(&tree, candidates);
        let (_na, _ps2, (order_none, _), _tr) = pi_theta_step_traced(
            work_none, &gamma, &[], 0.0, 11, 1000.0, &r, w,
            KThetaRiskGate::open(), &cfg(), &reg, None,
            &protocol_hold(),
        );
        assert!(order_none.qty > 0, "tw=None 对照：买候选正常开仓（qty>0）");
        assert_ne!(order_p3, order_none, "P3 masking 真改同 bar 订单流");
    }

    /// ★P4 EnterEarning（PDF §7 C_4）：TW CapitalRecovered ∧ EnterReady 五合取成立 ⟹
    /// tw_event=EnterEarning（无订单相变）。无 ReverseOpen 腿 ⟹ P2 不触发（H=0），落到 P4。
    #[test]
    fn pi_theta_step_traced_p4_enter_earning() {
        use super::super::super::ledger::{RiskPolicy, TStage, TwEvent, TwState};
        let (classification, tower) = buy_gamma();
        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        let reg = super::super::super::persistent::PersistentRegistry::new();
        let (tree, candidates, gamma) =
            interp::coverage_elements_and_gamma_with_tower(&classification, &tower);
        let work = ElementView::from_parts(&tree, candidates.clone());
        // EnterReady 五合取：S=II ∧ withdrawn≥notional_in ∧ legs=0 ∧ RiskNormal ∧ tw()≥η⋆
        // （κ=0 ⟹ η⋆=L^wc=(notional_in−withdrawn)⁺=0）。
        let tw_state = TwState {
            withdrawn: 100,
            notional_in: 100,
            stage: TStage::CapitalRecovered,
            ..TwState::initial()
        };
        let policy = RiskPolicy::baseline();
        let empty_ids: std::collections::HashMap<ElementId, Vertical> = Default::default();
        let twc = TwStepCtx {
            state: &tw_state,
            policy: &policy,
            risk_mode: RiskMode::Normal,
            entry_v: &empty_ids,
            eta_correction: 0, // A10 C5：零修正 = 历史判据 bit-exact（测试基准口径）
        };
        let (_na, _ps, (order_p4, _), trace) = pi_theta_step_traced(
            work, &gamma, &[], 0.0, 11, 1000.0, &r, w,
            KThetaRiskGate::open(), &cfg(), &reg, Some(&twc),
            &protocol_hold(),
        );
        assert_eq!(trace.tw_event, Some(TwEvent::EnterEarning), "P4 ⟹ EnterEarning 相变事件");
        assert!(trace.opened.is_empty(), "P4 消耗当步裁决 ⟹ 不开仓");
        // ★A' 清单⑤（P4 masking 真改订单流）：同 P3——相变事件无订单，但同 bar 普通开仓被推迟。
        assert_eq!(order_p4.qty, 0, "P4 屏蔽开仓 ⟹ 本 bar 无订单量");
        let work_none = ElementView::from_parts(&tree, candidates);
        let (_na2, _ps2, (order_none, _), _tr) = pi_theta_step_traced(
            work_none, &gamma, &[], 0.0, 11, 1000.0, &r, w,
            KThetaRiskGate::open(), &cfg(), &reg, None,
            &protocol_hold(),
        );
        assert!(order_none.qty > 0, "tw=None 对照：买候选正常开仓（qty>0）");
        assert_ne!(order_p4, order_none, "P4 masking 真改同 bar 订单流");
    }

    /// ★A10 C5（裁定 (b) TW 桥 G1 闭合见证，T-N4 接线层）：同一 EnterReady 态，
    /// `eta_correction`（= π loop `cum_holding_cost` shadow，funding+borrow+liq 累计量化）使
    /// η_corrected = tw() − correction 跌破 η⋆ ⟹ P4 **不派** EnterEarning——修正判据变严
    /// （η 高估消除 ⟹ 激进侧封死 = 安全侧）；correction=0 ⟹ 派（回归锁：与历史判据同值
    /// bit-exact）；边界 correction = tw()（η_corrected=0=η⋆）仍派（≥ 含等号）。
    #[test]
    fn a10_c5_eta_correction_gates_p4_enter_earning() {
        use super::super::super::ledger::{RiskPolicy, TStage, TwEvent, TwState};
        let (classification, tower) = buy_gamma();
        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        let reg = super::super::super::persistent::PersistentRegistry::new();
        let (tree, candidates, gamma) =
            interp::coverage_elements_and_gamma_with_tower(&classification, &tower);
        // EnterReady 态：tw()=100（withdrawn=100），η⋆=0（κ=0 ∧ L^wc=0）⟹ 未修正五合取全过。
        let tw_state = TwState {
            withdrawn: 100,
            notional_in: 100,
            stage: TStage::CapitalRecovered,
            ..TwState::initial()
        };
        let policy = RiskPolicy::baseline();
        let empty_ids: std::collections::HashMap<ElementId, Vertical> = Default::default();
        let run = |eta_correction: i64| {
            let work = ElementView::from_parts(&tree, candidates.clone());
            let twc = TwStepCtx {
                state: &tw_state,
                policy: &policy,
                risk_mode: RiskMode::Normal,
                entry_v: &empty_ids,
                eta_correction,
            };
            pi_theta_step_traced(
                work, &gamma, &[], 0.0, 11, 1000.0, &r, w,
                KThetaRiskGate::open(), &cfg(), &reg, Some(&twc),
                &protocol_hold(),
            )
            .3 // StepTrace
            .tw_event
        };
        // 回归锁：correction=0 ⟹ 与历史判据同值（派 EnterEarning）。
        assert_eq!(run(0), Some(TwEvent::EnterEarning), "correction=0 ⟹ 派（bit-exact 回归锁）");
        // 边界：correction=tw()=100 ⟹ η_corrected=0=η⋆ ⟹ 仍派（≥ 含等号）。
        assert_eq!(run(100), Some(TwEvent::EnterEarning), "η_corrected=0=η⋆ ⟹ 边界仍派");
        // G1 闭合见证：correction=101 ⟹ η_corrected=−1<η⋆=0 ⟹ 不派（η 高估被持盾成本修正
        // 消除 ⟹ EnterReady 不再易过——修正前 (correction=0) 同一态必派，对照在上方两条）。
        assert_eq!(run(101), None, "η_corrected=−1<η⋆ ⟹ 不派（G1：修正判据变严=安全侧）");
    }

    // ── #201 阶段 B：StepTrace.verdicts 显式 per-voice 裁决序列（schema 冻结，见 VoiceVerdict
    //    doc）——四个 return 分支逐一枚举：正常路径（Hold/typed）、P1（RiskExit）、
    //    P2（CloseReverseOpen+Hold）、P3/P4（Hold）。──

    /// 正常路径：跨级反向证书不触 L1 出场（t2 跨级同构场景）⟹ 持仓声部裁 **Hold**
    /// （由隐式转显式），每声部恰一枚、prev_active 次序。
    #[test]
    fn pi_theta_step_traced_verdicts_normal_path_explicit_hold() {
        let tower = two_parent_tower();
        let classification = sell_classification_at(0, 20, 1); // L0 卖证书（与 L1 持仓跨级）
        let (tree, candidates, gamma) =
            interp::coverage_elements_and_gamma_with_tower(&classification, &tower);
        let held = held_l1_compose_a();
        let held_idx = tree.iter().position(|e| e.id == held.id).expect("L1 compose_a 在塔中");
        let p_t = leg_target(&tree, held_idx, 1000.0, &cfg()).units;
        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        let reg = super::super::super::persistent::PersistentRegistry::new();
        let work = ElementView::from_parts(&tree, candidates);
        let (next_active, _p_star, _d, trace) = pi_theta_step_traced(
            work, &gamma, &[held], p_t, 21, 1000.0, &r, w, KThetaRiskGate::open(),
            &cfg(), &reg, None, &protocol_hold(),
        );
        assert_eq!(next_active, vec![held], "前置：跨级证书不触 L1 出场（腿延续）");
        assert_eq!(
            trace.verdicts,
            vec![VoiceVerdict { leg: held, exit: interp::ExitType::Hold }],
            "每持仓声部恰一枚显式裁决：延续 = Hold（由隐式转显式）"
        );
    }

    /// 正常路径：同级别已确认反向证书 ⟹ 该声部裁决 = [`interp::reverse_exit_type`] 单源
    /// typed（一类 CloseRoot / 三类 ReduceCore），与 `closed` 第三分量一致，无 Hold 记录。
    #[test]
    fn pi_theta_step_traced_verdicts_normal_path_typed_close() {
        let held = held_l1_compose_a();
        for class in [1u8, 3u8] {
            let tower = two_parent_tower();
            let classification = sell_classification_at(1, 12, class);
            let (tree, candidates, gamma) =
                interp::coverage_elements_and_gamma_with_tower(&classification, &tower);
            let held_idx = tree.iter().position(|e| e.id == held.id).expect("L1 compose_a 在塔中");
            let p_t = leg_target(&tree, held_idx, 1000.0, &cfg()).units;
            let r = rcfg();
            let w = PiThetaWeights::from_risk(&r);
            let reg = super::super::super::persistent::PersistentRegistry::new();
            let work = ElementView::from_parts(&tree, candidates);
            let (_na, _p, _d, trace) = pi_theta_step_traced(
                work, &gamma, &[held], p_t, 13, 1000.0, &r, w, KThetaRiskGate::open(),
                &cfg(), &reg, None, &protocol_hold(),
            );
            assert_eq!(trace.closed.len(), 1, "前置：同级别已确认证书关闭持仓腿");
            assert_eq!(
                trace.verdicts,
                vec![VoiceVerdict {
                    leg: held,
                    exit: interp::reverse_exit_type(Vertical::Ambient, class),
                }],
                "关闭声部裁决 = 单源 typed（class={class}，与 closed 第三分量一致）"
            );
        }
    }

    /// P1 force_flat 分支：prev_active 全部裁 RiskExit（每声部恰一枚，无 Hold/typed 混合）。
    #[test]
    fn pi_theta_step_traced_verdicts_p1_force_flat_all_risk_exit() {
        let (classification, tower) = buy_gamma();
        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        let reg = super::super::super::persistent::PersistentRegistry::new();
        let (tree, candidates, gamma) =
            interp::coverage_elements_and_gamma_with_tower(&classification, &tower);
        let work = ElementView::from_parts(&tree, candidates);
        let held = aleg(0, VoiceSide::Long, 0, 0);
        let flat = KThetaRiskGate { force_flat: true, stop_long: false, stop_short: false, no_increase_cap: None };
        let (_na, _p, _d, trace) = pi_theta_step_traced(
            work, &gamma, &[held], 600.0, 11, 1000.0, &r, w, flat, &cfg(), &reg, None,
            &protocol_hold(),
        );
        assert_eq!(trace.risk_exits.len(), 1, "前置：prev_active 全部 RiskExit");
        assert_eq!(
            trace.verdicts,
            vec![VoiceVerdict { leg: held, exit: interp::ExitType::RiskExit }],
            "P1 强平声部裁决 = RiskExit（每声部恰一枚）"
        );
    }

