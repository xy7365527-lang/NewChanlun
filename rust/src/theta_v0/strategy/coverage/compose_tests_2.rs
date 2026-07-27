use super::*;
use super::super::test_support::*;
use crate::theta_v0::types::{Center, BspBits};
use crate::theta_v0::classifier::LevelState;
use crate::theta_v0::classifier::bsp::BspPoint;

    /// ★#124 P1 强平（PDF §7 全互斥 C_1 屏蔽 P2..P10）：force_flat ⟹ 活动腿全部 RiskExit 清空、
    /// next_active=∅、无 open/close/silent（幽灵腿堵口 §6.7）。持仓 Long + 同 bar 买候选（正常 P8 会 open）
    /// ⟹ 均被 P1 屏蔽，open 不入 next_active（否则跨 bar 幽灵腿）。
    #[test]
    fn pi_theta_step_traced_p1_force_flat_risk_exits_all() {
        let buy = BspPoint { source_index: 4,
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

    /// P2 CloseOverlay 分支：重叠腿裁 CloseReverseOpen、保留腿裁 Hold，按 prev_active 次序
    /// 确定序输出（sd_leg 先、root_leg 后）。
    #[test]
    fn pi_theta_step_traced_verdicts_p2_overlay_typed_and_hold() {
        use super::super::super::ledger::{RiskPolicy, TStage, TwState};
        let (classification, tower) = buy_gamma();
        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        let reg = super::super::super::persistent::PersistentRegistry::new();
        let (tree, candidates, gamma) =
            interp::coverage_elements_and_gamma_with_tower(&classification, &tower);
        let work = ElementView::from_parts(&tree, candidates);
        let sd_leg = aleg(0, VoiceSide::Long, 7, 7);
        let root_leg = aleg(1, VoiceSide::Long, 3, 3);
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
            eta_correction: 0,
        };
        let (_na, _p, _d, trace) = pi_theta_step_traced(
            work, &gamma, &[sd_leg, root_leg], 0.0, 11, 1000.0, &r, w,
            KThetaRiskGate::open(), &cfg(), &reg, Some(&twc),
            &protocol_hold(),
        );
        assert_eq!(trace.overlay_closes.len(), 1, "前置：P2 关重叠腿恰一条");
        assert_eq!(
            trace.verdicts,
            vec![
                VoiceVerdict { leg: sd_leg, exit: interp::ExitType::CloseReverseOpen },
                VoiceVerdict { leg: root_leg, exit: interp::ExitType::Hold },
            ],
            "P2：overlay 腿 = CloseReverseOpen，保留腿 = Hold（prev_active 次序确定序）"
        );
    }

    /// P3/P4 TW 事件分支：消耗当步裁决（屏蔽 P5..P10）时持仓声部仍逐枚裁 Hold
    /// （活动腿保持，无开/平）。
    #[test]
    fn pi_theta_step_traced_verdicts_p3_consumes_step_holds() {
        use super::super::super::ledger::{RiskPolicy, TwEvent, TwState};
        let (classification, tower) = buy_gamma();
        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        let reg = super::super::super::persistent::PersistentRegistry::new();
        let (tree, candidates, gamma) =
            interp::coverage_elements_and_gamma_with_tower(&classification, &tower);
        let work = ElementView::from_parts(&tree, candidates);
        let held = aleg(1, VoiceSide::Long, 3, 3); // buy_gamma 场景下可存活（P2 测试实证）
        let tw_state = TwState {
            free: 100,
            holding: 100,
            notional_in: 100,
            ..TwState::initial()
        }; // CostReduction + holding≥notional_in + free 足额 ⟹ P3 成立
        let policy = RiskPolicy::baseline();
        let empty_ids: std::collections::HashMap<ElementId, Vertical> = Default::default();
        let twc = TwStepCtx {
            state: &tw_state,
            policy: &policy,
            risk_mode: RiskMode::Normal,
            entry_v: &empty_ids,
            eta_correction: 0,
        };
        let (next_active, _p, _d, trace) = pi_theta_step_traced(
            work, &gamma, &[held], 0.0, 11, 1000.0, &r, w,
            KThetaRiskGate::open(), &cfg(), &reg, Some(&twc),
            &protocol_hold(),
        );
        assert!(
            matches!(trace.tw_event, Some(TwEvent::RecoverCapital(_))),
            "前置：P3 成立（消耗当步裁决）"
        );
        assert_eq!(next_active, vec![held], "前置：活动腿保持");
        assert_eq!(
            trace.verdicts,
            vec![VoiceVerdict { leg: held, exit: interp::ExitType::Hold }],
            "P3 消耗当步裁决：持仓声部仍逐枚裁 Hold"
        );
    }

    /// ★优先级 C_1≻C_2（PDF §7）：P1 force_flat 与 P2 条件同时成立 ⟹ P1 赢（risk_exits，
    /// 无 overlay_closes/tw_event）。TW 谓词恒被 P1 屏蔽。
    #[test]
    fn pi_theta_step_traced_p1_masks_tw_predicates() {
        use super::super::super::ledger::{RiskPolicy, TStage, TwState};
        let (classification, tower) = buy_gamma();
        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        let reg = super::super::super::persistent::PersistentRegistry::new();
        let (tree, candidates, gamma) =
            interp::coverage_elements_and_gamma_with_tower(&classification, &tower);
        let work = ElementView::from_parts(&tree, candidates);
        let sd_leg = aleg(0, VoiceSide::Short, 7, 7);
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
        let flat = KThetaRiskGate { force_flat: true, stop_long: false, stop_short: false, no_increase_cap: None };
        let (next_active, _ps, _o, trace) = pi_theta_step_traced(
            work, &gamma, &[sd_leg], 0.0, 11, 1000.0, &r, w, flat, &cfg(), &reg, Some(&twc),
            &protocol_hold(),
        );
        assert!(next_active.is_empty());
        assert_eq!(trace.risk_exits.len(), 1, "P1 赢：RiskExit 通道");
        assert!(trace.overlay_closes.is_empty() && trace.tw_event.is_none(), "P2/P3/P4 被 P1 屏蔽");
    }

    /// ★TW ctx 存在但无 TW 谓词成立 ⟹ 与 tw=None 逐分量一致（P5..P10 原路径 bit-exact）。
    #[test]
    fn pi_theta_step_traced_tw_ctx_inert_bitexact() {
        use super::super::super::ledger::{RiskPolicy, TwState};
        let (classification, tower) = buy_gamma();
        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        let reg = super::super::super::persistent::PersistentRegistry::new();
        let (tree, candidates, gamma) =
            interp::coverage_elements_and_gamma_with_tower(&classification, &tower);
        let tw_state = TwState::initial(); // notional_in=0 ⟹ stage_progression inert；StageI ⟹ P2 不评估
        let policy = RiskPolicy::baseline();
        let empty_ids: std::collections::HashMap<ElementId, Vertical> = Default::default();
        let twc = TwStepCtx {
            state: &tw_state,
            policy: &policy,
            risk_mode: RiskMode::Normal,
            entry_v: &empty_ids,
            eta_correction: 0, // A10 C5：零修正 = 历史判据 bit-exact（测试基准口径）
        };
        let work_a = ElementView::from_parts(&tree, candidates.clone());
        let (na_a, ps_a, o_a, tr_a) = pi_theta_step_traced(
            work_a, &gamma, &[], 0.0, 5, 1000.0, &r, w, KThetaRiskGate::open(), &cfg(), &reg,
            Some(&twc),
            &protocol_hold(),
        );
        let work_b = ElementView::from_parts(&tree, candidates);
        let (na_b, ps_b, o_b, tr_b) = pi_theta_step_traced(
            work_b, &gamma, &[], 0.0, 5, 1000.0, &r, w, KThetaRiskGate::open(), &cfg(), &reg, None,
            &protocol_hold(),
        );
        assert_eq!(na_a, na_b);
        assert_eq!(ps_a, ps_b);
        assert_eq!(o_a, o_b);
        assert_eq!(tr_a.opened.len(), tr_b.opened.len());
        assert!(tr_a.tw_event.is_none() && tr_a.overlay_closes.is_empty());
    }

