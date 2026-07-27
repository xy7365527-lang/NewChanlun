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

    /// ★G4 组合层 [`StepTrace`]：opened=准入信号腿；traced 决策三分量 == prebuilt（委托 bit-exact 见证）。
    #[test]
    fn pi_theta_step_traced_opened_and_bitexact() {
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
        let tower = rc_tower(vec![]);
        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        let reg = super::super::super::persistent::PersistentRegistry::new();
        let (tree, candidates, gamma) =
            interp::coverage_elements_and_gamma_with_tower(&classification, &tower);
        let work_t = ElementView::from_parts(&tree, candidates.clone());
        let (na_t, ps_t, o_t, trace) = pi_theta_step_traced(
            work_t, &gamma, &[], 0.0, 5, 1000.0, &r, w, KThetaRiskGate::open(), &cfg(), &reg, None,
            &protocol_hold(),
        );
        // opened：唯一买点候选准入为信号腿（空 A_t ⟹ closed/silent 必空）。
        assert_eq!(trace.opened.len(), 1, "买点候选准入 ⟹ opened 恰一条");
        assert_eq!(trace.opened[0].0.dir, VoiceSide::Long);
        assert_eq!(trace.opened[0].1.id, na_t[0].id, "opened 腿 = next_active 中的新腿");
        assert!(trace.closed.is_empty() && trace.silent_drops.is_empty());
        // 委托 bit-exact：prebuilt 决策三分量 == traced。
        let work_p = ElementView::from_parts(&tree, candidates);
        let (na_p, ps_p, o_p) = pi_theta_step_prebuilt(
            work_p, &gamma, &[], 0.0, 5, 1000.0, &r, w, KThetaRiskGate::open(), &cfg(),
            &protocol_hold(), &reg,
        );
        assert_eq!(na_t, na_p);
        assert_eq!(ps_t, ps_p);
        assert_eq!(o_t, o_p);
    }

    /// ★G4 组合层 [`StepTrace`] 关闭归因：持仓 Long 腿遇反向卖候选 ⟹ closed=[(腿,触发卖候选)]，
    /// 被关腿不入 next_active、不入 silent_drops（close 认领互斥于静默离场）。
    #[test]
    fn pi_theta_step_traced_reverse_close_attribution() {
        let sell = BspPoint { level_origin: 0,
            source_index: 10,
            bits: BspBits { sell1: true, ..Default::default() },
            pivot_low: 0,
            pivot_high: 210,
            center: Some(crate::theta_v0::classifier::bsp::OwnerRef::Center(Center { zd: 100, zg: 200, dd: 90, gg: 210, start_index: 0, end_index: 9 })),
            struct_break_dir: None,
            force: None,
        };
        let classification = Classification {
            levels: vec![LevelState { bsp: Rc::new(vec![sell]), ..Default::default() }],
        };
        let tower = rc_tower(vec![]);
        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        let reg = super::super::super::persistent::PersistentRegistry::new();
        let (tree, candidates, gamma) =
            interp::coverage_elements_and_gamma_with_tower(&classification, &tower);
        let work = ElementView::from_parts(&tree, candidates);
        let held = aleg(0, VoiceSide::Long, 0, 0);
        let (next_active, _ps, _o, trace) = pi_theta_step_traced(
            work, &gamma, &[held], 600.0, 11, 1000.0, &r, w, KThetaRiskGate::open(), &cfg(), &reg, None,
            &protocol_hold(),
        );
        assert_eq!(trace.closed.len(), 1, "反向卖候选关闭持仓 Long 腿");
        let (leg, trig, _exit) = &trace.closed[0];
        assert_eq!(leg.id, held.id, "被关腿 = 持仓腿");
        assert_eq!(trig.bsp_class, 1, "触发归因 = 一类卖候选（reverse_exit_type ⟹ CloseRoot）");
        assert!(!next_active.iter().any(|l| l.id == held.id), "被关腿不入 next_active");
        assert!(trace.silent_drops.is_empty(), "close 认领互斥于静默离场");
    }

    /// ★#145 T1：typed 裁决前移到组合层——`StepTrace.closed` 第三分量携 [`interp::ExitType`]，
    /// 由 [`interp::reverse_exit_type`] 单源产出（一类反向 ⟹ CloseRoot；三类反向 ⟹ ReduceCore）。
    /// 双分支各验一遍：主路径（在飞 entry_v 映射携 Ambient）与回退分支（tw=None ⟹ 回退
    /// Ambient=非 ReverseOpen，诚实回退语义）裁决必须一致。
    #[test]
    fn pi_theta_step_traced_typed_close_root_and_reduce_core() {
        use super::super::super::ledger::{RiskPolicy, TwState};
        let tower = rc_tower(vec![]);
        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        let reg = super::super::super::persistent::PersistentRegistry::new();
        let held = aleg(0, VoiceSide::Long, 0, 0);
        let run = |class: u8, mapped: bool| {
            let classification = sell_classification(class);
            let (tree, candidates, gamma) =
                interp::coverage_elements_and_gamma_with_tower(&classification, &tower);
            let work = ElementView::from_parts(&tree, candidates);
            let tw_state = TwState::initial(); // inert：P2/P3/P4 不成立，落普通 fold 路径
            let policy = RiskPolicy::baseline();
            let entry_v: std::collections::HashMap<ElementId, Vertical> =
                [(held.id, Vertical::Ambient)].into_iter().collect();
            let twc = TwStepCtx {
                state: &tw_state,
                policy: &policy,
                risk_mode: RiskMode::Normal,
                entry_v: &entry_v,
                eta_correction: 0,
            };
            let tw = if mapped { Some(&twc) } else { None };
            let (_na, _ps, _o, trace) = pi_theta_step_traced(
                work, &gamma, &[held], 600.0, 11, 1000.0, &r, w, KThetaRiskGate::open(),
                &cfg(), &reg, tw, &protocol_hold(),
            );
            assert_eq!(trace.closed.len(), 1, "反向卖候选关闭持仓 Long 腿");
            trace.closed[0].2
        };
        for mapped in [true, false] {
            assert_eq!(
                run(1, mapped),
                interp::ExitType::CloseRoot,
                "一类反向 ⟹ P5 CloseRoot（mapped={mapped}）"
            );
            assert_eq!(
                run(3, mapped),
                interp::ExitType::ReduceCore,
                "三类反向 ⟹ P6 ReduceCore（mapped={mapped}）"
            );
        }
    }

    /// ★#199 断言①（#185「建议断言」1，T1 目标态）生产路径触发见证（合成单腿场景）：
    /// 一类卖 Core 评估探针真实触发，且单腿场景 `target_qty(Core{L})==0`（违例探针=0——
    /// 单腿即「一类=全平」成立的平凡域）。**多核心腿场景经 #209 终态化**：fold 一类全平
    /// + 断言①硬门（见断言①挂载点注释）+ 多腿合成见证
    /// `t1_target_zero_assertion_holds_with_multiple_core_legs` + runner 侧 BTC 见证
    /// `btc_type2_residual_correction_witness` 全窗违例=0。
    #[test]
    fn t1_target_zero_assertion_fires_in_step() {
        use super::super::super::ledger::{RiskPolicy, TwState};
        t1_target_zero_probe_reset();
        let tower = rc_tower(vec![]);
        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        let reg = super::super::super::persistent::PersistentRegistry::new();
        let held = aleg(0, VoiceSide::Long, 0, 0);
        let classification = sell_classification(1);
        let (tree, candidates, gamma) =
            interp::coverage_elements_and_gamma_with_tower(&classification, &tower);
        let work = ElementView::from_parts(&tree, candidates);
        let tw_state = TwState::initial();
        let policy = RiskPolicy::baseline();
        let entry_v: std::collections::HashMap<ElementId, Vertical> =
            [(held.id, Vertical::Ambient)].into_iter().collect();
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
        assert_eq!(trace.closed.len(), 1, "一类卖关闭 Long 根腿（断言①评估对象）");
        assert_eq!(
            t1_target_zero_probe_count(),
            1,
            "一类核心关闭一笔 ⟹ 断言①评估在生产路径真实触发（探针为凭）"
        );
        assert_eq!(
            t1_target_residual_probe_count(),
            0,
            "单腿场景：一类卖后 target_qty(Core{{0}})==0（级残余为零的平凡域）"
        );
    }

    /// ★#209 断言①终态（多核心腿场景，S7 级别内全平）：一类卖候选 ⟹ 同级**全部**
    /// Core 腿全关、级目标残余=0。prev_active 直喂「Ambient 根 + FollowParent 级联」
    /// 两条同级核心腿（账户同归 Core{0}，经 `identity_of` 单源）——#199 BTC 实测
    /// 缺口（一类只关首条、级联腿残余 277.9）的修复见证。
    #[test]
    fn t1_target_zero_assertion_holds_with_multiple_core_legs() {
        use super::super::super::ledger::{RiskPolicy, TwState};
        t1_target_zero_probe_reset();
        let tower = rc_tower(vec![]);
        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        let reg = super::super::super::persistent::PersistentRegistry::new();
        let held_root = aleg(0, VoiceSide::Long, 0, 0); // Ambient 根
        let held_cascade = aleg(0, VoiceSide::Long, 2, 2); // FollowParent 级联（同级同向）
        let classification = sell_classification(1);
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
        assert_eq!(trace.closed.len(), 2, "一类卖关闭同级全部核心腿（S7 全平）");
        assert!(
            trace.closed.iter().all(|(_, trig, _)| trig.bsp_class == 1),
            "两腿触发归因同一一类候选"
        );
        assert!(
            !na.iter().any(|l| l.id == held_root.id || l.id == held_cascade.id),
            "被关两腿均不入 next_active"
        );
        assert_eq!(
            t1_target_zero_probe_count(),
            2,
            "一类核心关闭两笔 ⟹ 断言①评估逐腿触发（探针为凭）"
        );
        assert_eq!(
            t1_target_residual_probe_count(),
            0,
            "多腿场景：一类卖后 target_qty(Core{{0}})==0（S7 级别内全平）"
        );
    }

    /// ★#237 断言①**按方向拆查**（#234 蓝图依据：出场方向锁定「多头声部由卖点证书平仓；
    /// 空头声部由买点证书平仓」+ 分侧账禁净额，拆查为唯一一致查法；用户裁 2026-07-24）：
    /// **一类卖批 ⇒ 查该级多侧分量=0，空侧不查**。同级顺父级联 Short 腿（当 bar
    /// role_v=FollowParent ⟹ 归 Core{0}，#185 映射不动）在一类卖批下**合法存活**（蓝图
    /// 动作表：空头声部×卖点 = 同向信号持有/加仓/记录，买卖点2.pdf p.8/14 §11），不计入
    /// 卖批残余——其合法出场路径为一类买点/元素终结/父关连清/风险强平（#234 Q4.3）。
    ///
    /// 场景（two_parent_tower：compose_a Long 父 + compose_b Short 父）：prev_active =
    /// L0 Long 子腿（a2，父 compose_a）+ L0 Short 子腿（b1，父 compose_b，顺父级联）
    /// + L1 Short 父腿（compose_b，Ambient×Short⟹Short 账，不入 Core 残余域）。
    /// 一类卖候选（L0）反向关闭 Long 子腿；Short 子腿同向不关闭（reverse_signal 方向锁定）。
    ///
    /// **RED（旧口径）**：残余求和不分方向 ⟹ 存活 Short 腿 q_units 计入 ⟹ 级残余≠0
    /// panic——m3 win9 bar=232810 (1,470) FollowParent×Short 误报的最小复现（#233 §5
    /// 另案面，本票收口）。**GREEN（新口径）**：多侧分量=0（卖批查零过）；空侧分量>0
    /// 但不查（违例探针=0）。
    #[test]
    fn t1_target_zero_assertion_sell_batch_checks_long_side_only() {
        use super::super::super::ledger::{RiskPolicy, TwState};
        t1_target_zero_probe_reset();
        let tower = two_parent_tower();
        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        let reg = super::super::super::persistent::PersistentRegistry::new();
        // L0 Long 子腿（a2 λ=8 ρ=12，父 compose_a=Long）——一类卖批的关闭对象
        // （entry_v=Ambient ⟹ Core{0}，断言①评估对象）。
        let held_long = ActiveLeg {
            level: 0, dir: VoiceSide::Long, source_index: 12, lambda: 8,
            id: eid(0, 2), parent_id: Some(eid(1, 0)), is_boundary_root: false, op_parent: None,
        };
        // L0 Short 子腿（b1 λ=16 ρ=20，父 compose_b=Short）——顺父级联空侧腿（当 bar
        // role_v=FollowParent ⟹ Core{0}），一类卖批下同向持有（m3 (1,470) 同族形态）。
        let held_short = ActiveLeg {
            level: 0, dir: VoiceSide::Short, source_index: 20, lambda: 16,
            id: eid(0, 4), parent_id: Some(eid(1, 1)), is_boundary_root: false, op_parent: None,
        };
        // L1 Short 父腿（compose_b λ=12 ρ=24）——held_short 的 AncOK 父容器。
        let held_parent_b = ActiveLeg {
            level: 1, dir: VoiceSide::Short, source_index: 24, lambda: 12,
            id: eid(1, 1), parent_id: None, is_boundary_root: true, op_parent: None,
        };
        let classification = sell_classification(1);
        let (tree, candidates, gamma) =
            interp::coverage_elements_and_gamma_with_tower(&classification, &tower);
        let work = ElementView::from_parts(&tree, candidates);
        let tw_state = TwState::initial();
        let policy = RiskPolicy::baseline();
        let entry_v: std::collections::HashMap<ElementId, Vertical> = [
            (held_long.id, Vertical::Ambient),
            (held_short.id, Vertical::FollowParent),
            (held_parent_b.id, Vertical::Ambient),
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
            work, &gamma, &[held_long, held_short, held_parent_b], 600.0, 11, 1000.0, &r, w,
            KThetaRiskGate::open(), &cfg(), &reg, Some(&twc), &protocol_hold(),
        );
        assert_eq!(trace.closed.len(), 1, "一类卖只关反向 Long 腿（出场方向锁定）");
        assert_eq!(trace.closed[0].0.id, held_long.id, "被关 = L0 Long 子腿");
        assert_eq!(trace.closed[0].1.bsp_class, 1, "触发归因 = 一类卖候选");
        assert!(
            na.iter().any(|l| l.id == held_short.id),
            "顺父级联 Short 腿一类卖批下合法存活（空头×卖点=同向持有）"
        );
        // 分侧账面事实重算（与断言①残余同源的只读投影）：空侧>0（存活在册）、多侧=0。
        let side_sum = |side: VoiceSide| -> f64 {
            trace
                .sep_legs
                .iter()
                .filter(|s| s.id.level == 0 && s.side == side)
                .filter(|s| {
                    account::identity_of(s.role_v, s.side, s.id.level)
                        == Some(account::AccountIdentity::Core { level: 0 })
                })
                .map(|s| s.q_units)
                .sum()
        };
        assert!(
            side_sum(VoiceSide::Short) > 0.0,
            "空侧分量非零（合法存活腿在册）；实得 {}",
            side_sum(VoiceSide::Short)
        );
        assert_eq!(
            side_sum(VoiceSide::Long),
            0.0,
            "多侧分量=0（一类卖批全平该级该方向，S2「该级该方向」良构形式）"
        );
        assert_eq!(
            t1_target_zero_probe_count(),
            1,
            "一类核心关闭一笔 ⟹ 断言①评估在生产路径真实触发（探针为凭）"
        );
        assert_eq!(
            t1_target_residual_probe_count(),
            0,
            "一类卖批只查多侧分量：空侧存活不计入（#237 拆查口径，违例=0）"
        );
    }

    /// ★#237 断言①按方向拆查**反向对称**（票面验收 1 镜像）：**一类买批 ⇒ 查该级空侧
    /// 分量=0，多侧不查**。同级多侧腿（FollowParent×Long ⟹ Core{0}）在一类买批下合法
    /// 存活（多头声部×买点 = 同向信号持有/加仓/记录），不计入买批残余。
    ///
    /// 场景（two_parent_tower 镜像）：prev_active = L0 Short 子腿（b1，父 compose_b，
    /// FollowParent⟹Core{0}，买批关闭对象）+ L0 Long 子腿（a2，父 compose_a，
    /// FollowParent⟹Core{0}，存活）+ L1 Long 父腿（compose_a，AncOK 父）。一类买候选
    /// （L0）反向关闭 Short 子腿；Long 子腿同向不关闭。
    ///
    /// 旧口径下本场景残余 = 存活 Long 腿 q_units ≠ 0 ⟹ 必违例（与卖批枚 red=残余 300
    /// panic 同构）；新口径空侧分量=0（买批查零过）、多侧>0 不查（违例探针=0）。
    #[test]
    fn t1_target_zero_assertion_buy_batch_checks_short_side_only() {
        use super::super::super::ledger::{RiskPolicy, TwState};
        t1_target_zero_probe_reset();
        let tower = two_parent_tower();
        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        let reg = super::super::super::persistent::PersistentRegistry::new();
        // L0 Short 子腿（b1 λ=16 ρ=20，父 compose_b=Short）——一类买批的关闭对象
        // （entry_v=FollowParent ⟹ Core{0}，断言①评估对象）。
        let held_short = ActiveLeg {
            level: 0, dir: VoiceSide::Short, source_index: 20, lambda: 16,
            id: eid(0, 4), parent_id: Some(eid(1, 1)), is_boundary_root: false, op_parent: None,
        };
        // L0 Long 子腿（a2 λ=8 ρ=12，父 compose_a=Long）——顺父级联多侧腿（当 bar
        // role_v=FollowParent ⟹ Core{0}），一类买批下同向持有（合法存活）。
        let held_long = ActiveLeg {
            level: 0, dir: VoiceSide::Long, source_index: 12, lambda: 8,
            id: eid(0, 2), parent_id: Some(eid(1, 0)), is_boundary_root: false, op_parent: None,
        };
        // L1 Long 父腿（compose_a λ=0 ρ=12）——held_long 的 AncOK 父容器。
        let held_parent_a = ActiveLeg {
            level: 1, dir: VoiceSide::Long, source_index: 12, lambda: 0,
            id: eid(1, 0), parent_id: None, is_boundary_root: true, op_parent: None,
        };
        let classification = buy_classification_at(0, 10, 1);
        let (tree, candidates, gamma) =
            interp::coverage_elements_and_gamma_with_tower(&classification, &tower);
        let work = ElementView::from_parts(&tree, candidates);
        let tw_state = TwState::initial();
        let policy = RiskPolicy::baseline();
        let entry_v: std::collections::HashMap<ElementId, Vertical> = [
            (held_short.id, Vertical::FollowParent),
            (held_long.id, Vertical::FollowParent),
            (held_parent_a.id, Vertical::Ambient),
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
            work, &gamma, &[held_short, held_long, held_parent_a], 600.0, 11, 1000.0, &r, w,
            KThetaRiskGate::open(), &cfg(), &reg, Some(&twc), &protocol_hold(),
        );
        assert_eq!(trace.closed.len(), 1, "一类买只关反向 Short 腿（出场方向锁定）");
        assert_eq!(trace.closed[0].0.id, held_short.id, "被关 = L0 Short 子腿");
        assert_eq!(trace.closed[0].1.bsp_class, 1, "触发归因 = 一类买候选");
        assert!(
            na.iter().any(|l| l.id == held_long.id),
            "顺父级联 Long 腿一类买批下合法存活（多头×买点=同向持有）"
        );
        // 分侧账面事实重算：多侧>0（存活在册）、空侧=0。
        let side_sum = |side: VoiceSide| -> f64 {
            trace
                .sep_legs
                .iter()
                .filter(|s| s.id.level == 0 && s.side == side)
                .filter(|s| {
                    account::identity_of(s.role_v, s.side, s.id.level)
                        == Some(account::AccountIdentity::Core { level: 0 })
                })
                .map(|s| s.q_units)
                .sum()
        };
        assert!(
            side_sum(VoiceSide::Long) > 0.0,
            "多侧分量非零（合法存活腿在册）；实得 {}",
            side_sum(VoiceSide::Long)
        );
        assert_eq!(
            side_sum(VoiceSide::Short),
            0.0,
            "空侧分量=0（一类买批全平该级该方向，与卖批镜像）"
        );
        assert_eq!(
            t1_target_zero_probe_count(),
            1,
            "一类核心关闭一笔 ⟹ 断言①评估在生产路径真实触发（探针为凭）"
        );
        assert_eq!(
            t1_target_residual_probe_count(),
            0,
            "一类买批只查空侧分量：多侧存活不计入（#237 拆查口径，违例=0）"
        );
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

