use super::*;
use super::super::test_support::*;
use crate::theta_v0::types::{Center, BspBits};
use crate::theta_v0::classifier::LevelState;
use crate::theta_v0::classifier::bsp::BspPoint;
use super::super::super::interp::ActiveLeg;

    /// PiThetaWeights::from_risk 复用 κ/ρ（不引新参数）。
    #[test]
    fn pi_theta_weights_reuse_kappa_rho() {
        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        assert_eq!(w.w, 1.0);
        assert_eq!(w.lambda, r.kappa);
        assert_eq!(w.nu, r.rho);
    }

    /// ★𝒦_Θ≠∅：可行集恒含安全锚 0（非空性构造见证，spec line 725 硬前提）。
    #[test]
    fn feasible_set_nonempty_contains_zero() {
        let cands = feasible_candidates(600.0, 0.0, 1000.0, 1000.0, 1.0);
        assert!(!cands.is_empty(), "𝒦_Θ 非空");
        assert!(cands.iter().any(|&(p, _)| p.abs() < 1e-9), "含安全锚 0");
        // cap<lot 退化：hi=0 ⟹ 𝒦_Θ={0} 仍非空。
        let degen = feasible_candidates(5.0, 0.0, 0.4, 0.4, 1.0);
        assert!(!degen.is_empty());
        assert!(degen.iter().all(|&(p, _)| p.abs() < 1e-9), "cap<lot ⟹ 𝒦_Θ={{0}}");
    }

    /// ★候选全 lot 对齐 ∈[−hi,hi] + grid_index 升序单射（手数约束 + 固定平局规则）。
    #[test]
    fn feasible_candidates_lot_aligned_within_cap() {
        let (lot, cap) = (10.0, 100.0);
        let cands = feasible_candidates(23.0, 0.0, cap, cap, lot);
        for &(p, _) in &cands {
            assert!((p / lot).fract().abs() < 1e-9, "lot 对齐");
            assert!(p.abs() <= cap + 1e-9, "∈[−cap,cap]");
        }
        for i in 1..cands.len() {
            assert!(cands[i].1 > cands[i - 1].1 && cands[i].0 > cands[i - 1].0, "升序单射");
        }
    }

    /// ★p* 跟踪 p̃（cap 内）：p̃=600 在 cap=1000 内 ⟹ p*=600（主键跟踪误差 0）。
    #[test]
    fn pi_theta_position_tracks_ptilde_within_cap() {
        let r = rcfg();
        let p_star =
            pi_theta_position(600.0, 0.0, 1000.0, &r, PiThetaWeights::from_risk(&r), KThetaRiskGate::open());
        assert!((p_star - 600.0).abs() < 1e-9, "p* = p̃ = 600（cap 内跟踪）");
    }

    /// ★杠杆/资本 cap binding：p̃=1500 超 cap=1000 ⟹ p*=1000（±hi 截断，结论翻转）。
    #[test]
    fn pi_theta_position_clamps_over_cap() {
        let r = rcfg(); // base_units=1000（U_ℓ）, gamma=1.0（γ̄）⟹ cap=U_ℓ·γ̄=1000
        let p_star =
            pi_theta_position(1500.0, 0.0, 1000.0, &r, PiThetaWeights::from_risk(&r), KThetaRiskGate::open());
        assert!((p_star - 1000.0).abs() < 1e-9, "p* = +hi = cap = 1000（杠杆/资本 cap binding）");
    }

    /// ★lot 取整：lot=10、p̃=23 ⟹ p*=20（最近 lot，|23−20|<|23−30|）。
    #[test]
    fn pi_theta_position_lot_rounds_to_nearest() {
        let mut r = rcfg();
        r.default_lot = 10;
        let p_star =
            pi_theta_position(23.0, 0.0, 1000.0, &r, PiThetaWeights::from_risk(&r), KThetaRiskGate::open());
        assert!((p_star - 20.0).abs() < 1e-9, "p* = 20（最近 lot 点）");
    }

    /// ★Schedule_Θ 开仓：空仓 → p*=±600 ⟹ Buy/Sell 600。
    #[test]
    fn schedule_open_from_flat() {
        let o = schedule_order(600.0, 0.0, 7);
        assert_eq!((o.action, o.qty, o.exec_index), (StrictAction::Buy, 600, 7));
        assert_eq!(schedule_order(-600.0, 0.0, 7).action, StrictAction::Sell);
    }

    /// ★Schedule_Θ 平仓：持多 p_t=600 → p*=0 ⟹ Close 600。
    #[test]
    fn schedule_close_to_flat() {
        let o = schedule_order(0.0, 600.0, 3);
        assert_eq!((o.action, o.qty), (StrictAction::Close, 600));
    }

    /// ★Schedule_Θ 增/减持：同号幅度增=Add、减=Reduce（多空对称）。
    #[test]
    fn schedule_add_reduce() {
        assert_eq!(schedule_order(900.0, 600.0, 0).action, StrictAction::Add);
        assert_eq!(schedule_order(900.0, 600.0, 0).qty, 300);
        assert_eq!(schedule_order(300.0, 600.0, 0).action, StrictAction::Reduce);
        assert_eq!(schedule_order(-900.0, -600.0, 0).action, StrictAction::Add); // 更空=Add
    }

    /// ★Schedule_Θ 无交易（全函数）：Δ=0 ⟹ 持仓 Hold / 空仓 Wait（qty=0）。
    #[test]
    fn schedule_hold_wait_no_trade() {
        assert_eq!(schedule_order(600.0, 600.0, 0).action, StrictAction::Hold);
        assert_eq!(schedule_order(600.0, 600.0, 0).qty, 0);
        assert_eq!(schedule_order(0.0, 0.0, 0).action, StrictAction::Wait);
    }

    /// ★Schedule_Θ 反号穿零（净反转）：持多 p_t=600 → p*=−200 ⟹ Sell 800。
    #[test]
    fn schedule_sign_flip_reverse() {
        let o = schedule_order(-200.0, 600.0, 0);
        assert_eq!((o.action, o.qty), (StrictAction::Sell, 800));
    }

    /// ★全链 π_Θ（GAP-5）：买点 Γ 入场 → π_Θ → Buy；入场源=买卖点 source_index（非走势边界）。
    #[test]
    fn pi_theta_step_buy_point_entry_gap5() {
        let bsp = BspPoint { level_origin: 0,
            source_index: 4,
            bits: BspBits { buy1: true, ..Default::default() },
            pivot_low: 90,
            pivot_high: 0,
            center: Some(crate::theta_v0::classifier::bsp::OwnerRef::Center(Center { zd: 100, zg: 200, dd: 90, gg: 210, start_index: 0, end_index: 9 })),
            struct_break_dir: None,
            force: None,
        };
        let classification = Classification {
            levels: vec![LevelState { bsp: Rc::new(vec![bsp]), ..Default::default() }],
        };
        let r = rcfg();
        let (active, p_star, (order, protocol_event)) = pi_theta_step(
            &classification, &[], &[], 0.0, 5, 1000.0, &cfg(), &r, PiThetaWeights::from_risk(&r),
            KThetaRiskGate::open(), &protocol_hold(), &super::super::super::persistent::PersistentRegistry::new(),
        );
        // GAP-5：活动腿 source_index = BspPoint.source_index = 4（买卖点，非 LeveledMove.start_index）。
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].source_index, 4, "GAP-5：入场源=买卖点 source_index");
        assert_eq!(active[0].dir, VoiceSide::Long);
        assert!((p_star - 600.0).abs() < 1e-9, "p̃=600 cap 内 ⟹ p*=600");
        assert_eq!((order.action, order.qty, order.exec_index), (StrictAction::Buy, 600, 5));
        assert_eq!(protocol_event, ProtocolEvent::hold(0), "无协议证据也显式输出 Hold");
    }

    /// ★#202 阶段 C（spec WP-3：仅替换 P2/P3——本级证书平仓由 channel 解释器承担）：
    /// 多腿场景「每声部每步一枚」的 channel 裁决替代散装 fold 规则2 的候选消费粒度。
    /// 两条同级 C 组核心腿（entry_v≠ReverseOpen）+ 单个**三类**反向候选：channel 判据下
    /// **每条**命中腿独立裁 `Exit(ReduceCore)`（散装 fold 二/三类只关首个——票面明知
    /// 非 bit-exact 域，本测试 = 该结构差的标志性行为见证）。
    /// 交叉断言 `channel::step_voice` 同输入逐腿同裁（「走 channel 解释器裁决」之凭）。
    #[test]
    fn p23_channel_closes_every_reverse_hit_leg_multi_leg() {
        use super::super::super::channel::{self, ChannelDecision, VoiceState, VoiceStepInput};
        use super::super::super::ledger::{RiskPolicy, TwState};
        let tower = rc_tower(vec![]);
        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        let reg = super::super::super::persistent::PersistentRegistry::new();
        let held_root = aleg(0, VoiceSide::Long, 0, 0); // Ambient 根
        let held_cascade = aleg(0, VoiceSide::Long, 2, 2); // FollowParent 级联（同级同向）
        let classification = sell_classification(3); // 三类反向：散装 fold 只关首个
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
        assert_eq!(
            trace.closed.len(),
            2,
            "channel 判据：每条命中腿独立裁 Exit（散装 fold 三类只关首个）"
        );
        assert!(
            trace.closed.iter().all(|(_, trig, exit)| {
                trig.bsp_class == 3 && *exit == interp::ExitType::ReduceCore
            }),
            "两腿触发归因同一三类候选、typed 均 ReduceCore（reverse_exit_type 单源）"
        );
        assert!(
            !na.iter().any(|l| l.id == held_root.id || l.id == held_cascade.id),
            "被关两腿均不入 next_active"
        );
        // verdicts 落在 #201 冻结 schema：两腿各携 ReduceCore 裁决（无 Hold 残留）。
        assert_eq!(trace.verdicts.len(), 2);
        assert!(
            trace.verdicts.iter().all(|v| v.exit == interp::ExitType::ReduceCore),
            "verdicts：每持仓声部恰一枚 typed 裁决（P3 域）"
        );
        // 交叉断言：channel::step_voice 同输入逐腿同裁 Exit(ReduceCore)——生产裁决
        // 与 channel 解释器同单源（阶段 C「走 channel 解释器裁决」之凭）。
        for (held, ev) in [(held_root, Vertical::Ambient), (held_cascade, Vertical::FollowParent)] {
            let voice = VoiceState {
                level: held.level,
                leg: Some(held),
                entry_v: ev,
                step: 0,
                sub_cycle: channel::SubCycleTracker::default(),
                reverse_open: None,
            };
            let input = VoiceStepInput {
                force_flat: false,
                candidates: gamma.clone(),
                parent_kappa: channel::ParentKappa::Unknown,
                parent_projections: Vec::new(),
            };
            assert_eq!(
                channel::step_voice(&voice, &input).1,
                ChannelDecision::Exit(interp::ExitType::ReduceCore),
                "channel 逐腿裁决 == 生产 P3 域裁决（entry_v={ev:?}）"
            );
        }
    }

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

