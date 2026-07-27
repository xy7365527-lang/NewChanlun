use super::*;
use super::super::test_support::*;
use crate::theta_v0::types::Center;
use crate::theta_v0::classifier::bsp::BspPoint;
use crate::theta_v0::classifier::LevelState;
use crate::theta_v0::types::BspBits;

    /// ★协变分解守恒（票体验收「𝒦_Θ 协变 cap 按级别分解后仍满足」）：`Σ_ℓ cap_ℓ ≤ γ̄·U_ℓ`
    /// （账户层总 cap），由 `Σw_ℓ≤1` 代数保证——不依赖跑批数据，L0。
    #[test]
    fn level_cap_decomposition_never_exceeds_account_cap() {
        let risk = RiskConfig { level_weights: vec![0.5, 0.3, 0.2], ..rcfg() };
        let base_units: f64 = 1000.0;
        let account_cap = feasible_net_cap(&risk) * base_units.abs();
        let sum_level_caps: f64 =
            (0u32..3).map(|lvl| level_cap(lvl, base_units, &risk)).sum();
        assert!(
            sum_level_caps <= account_cap + 1e-6,
            "Σcap_ℓ={sum_level_caps} 必须 ≤ 账户层总 cap={account_cap}"
        );
        // Σw_ℓ=1.0（边界）⟹ 分解**恰好**覆盖账户层总 cap，非严格小于（1.0 是上确界而非余量）。
        assert!((sum_level_caps - account_cap).abs() < 1e-6, "Σw_ℓ=1 ⟹ 分解恰好覆盖总 cap");
    }


    /// ★level_cap 协变缩放：`base_units`（`U_ℓ`）翻倍 ⟹ `cap_ℓ` 同比翻倍（方案A协变律，与
    /// `feasible_net_cap` 文档同构，非另立一套缩放规则）。
    #[test]
    fn level_cap_is_covariant_with_base_units() {
        let risk = RiskConfig { level_weights: vec![0.4], ..rcfg() };
        let cap_1x = level_cap(0, 1000.0, &risk);
        let cap_2x = level_cap(0, 2000.0, &risk);
        assert!((cap_2x - 2.0 * cap_1x).abs() < 1e-9, "cap_ℓ 随 U_ℓ 协变缩放");
    }


    /// ★level_cap 未配置该级别权重 ⟹ 0（表外级别帽=0，同 `level_weight` 纪律，非错误）。
    #[test]
    fn level_cap_zero_for_unweighted_level() {
        let risk = RiskConfig { level_weights: vec![0.5], ..rcfg() };
        assert_eq!(level_cap(1, 1000.0, &risk), 0.0, "level 1 未配权重 ⟹ cap=0");
    }


    /// ★#351 MED-1 补课：`level_weights_sum_le_one` 是 doc 声明的「唯一权威判据」，但接线前
    /// 从未被生产路径（`clamp_levels_to_weighted_cap`）实际调用——误配 Σw_ℓ=1.2>1 时裁剪照常
    /// 施加，Σcap_ℓ 静默突破账户层总 cap（#349 影子评审 MED-1）。本测先坐实红：接线前误配
    /// 不拒绝、裁剪正常返回。
    #[test]
    fn clamp_levels_to_weighted_cap_rejects_misconfigured_weights_over_one() {
        let risk = RiskConfig { level_weights: vec![0.6, 0.6], ..rcfg() }; // Σ=1.2>1，违规配置
        let gated = vec![(0u32, 10i64), (1, 10)];
        let result = std::panic::catch_unwind(|| clamp_levels_to_weighted_cap(&gated, 1000.0, &risk));
        assert!(
            result.is_err(),
            "MED-1 修复后：Σw_ℓ=1.2>1 的违规配置必须在裁剪前被拒绝（panic），不得静默放行"
        );
    }


    /// ★clamp_levels_to_weighted_cap：超帽级别被裁到 ±cap_ℓ，未超帽级别原样透传，零项保留。
    #[test]
    fn clamp_levels_to_weighted_cap_clips_only_binding_levels() {
        let risk = RiskConfig { level_weights: vec![0.5, 0.1], ..rcfg() };
        let base_units = 100.0; // cap_0=0.5*1.0*100=50, cap_1=0.1*1.0*100=10
        let gated = vec![(0u32, 80i64), (1, 3), (2, 0)];
        let clamped = clamp_levels_to_weighted_cap(&gated, base_units, &risk);
        assert_eq!(clamped, vec![(0, 50), (1, 3), (2, 0)], "level0 超帽裁到 50，level1 未超帽原样，level2 零项保留");
        // 负向对称裁剪。
        let gated_neg = vec![(0u32, -80i64)];
        assert_eq!(clamp_levels_to_weighted_cap(&gated_neg, base_units, &risk), vec![(0, -50)]);
    }


    /// ★#351 MED-2 补课：`attribute_total` 的比例缩放可能把已裁剪到 `cap_ℓ` 的 `gated` 基准
    /// 放大后重新推出 `cap_ℓ`——`fill.rs` 注释「级别帽…不留一条未裁剪的旁路」在此场景下为假
    /// （#349 影子评审 MED-2）。构造：level0 裁剪后恰在 cap（50），账户层投影把 `T_lee` 从
    /// `Σgated=60` 放大到 100（如 pan_div 加项/lex 重估），`attribute_total` 按比例把 level0
    /// 放大到 83，突破 cap_0=50。修复：`fill.rs` 在归因后对 `plan.targets` 用同一
    /// `clamp_levels_to_weighted_cap` 二次裁剪（本函数残差桶安全，见上方 doc）——本测直接验证
    /// 「缩放会突破 cap」+「二次裁剪能收回」两段。
    #[test]
    fn attribute_total_scaling_can_exceed_cap_and_reclamp_restores_it() {
        use super::super::super::level_attrib::attribute_total;
        let risk = RiskConfig { level_weights: vec![0.5, 0.1], ..rcfg() };
        let base_units = 100.0; // cap_0=50, cap_1=10
        let gated = vec![(0u32, 50i64), (1, 10)]; // 裁剪后恰好压在 cap 上（Σ=60）
        // 账户层投影把目标从 60 放大到 100（模拟 pan_div 加项或 lex 重估使 T_lee ≠ Σgated）。
        let (scaled, _, rescaled) = attribute_total(&gated, 100);
        assert!(rescaled, "Σbasis(60)≠target(100) 必须触发比例缩放");
        let level0_scaled = scaled.iter().find(|&&(l, _)| l == 0).unwrap().1;
        assert!(
            level0_scaled > 50,
            "红：缩放后 level0={level0_scaled} 必须突破 cap_0=50，坐实『重越』可达（{scaled:?}）"
        );
        // 修复验证：对缩放结果二次施加同一 clamp 函数，突破被收回。
        let reclamped = clamp_levels_to_weighted_cap(&scaled, base_units, &risk);
        for &(lvl, q) in &reclamped {
            let cap = level_cap(lvl, base_units, &risk).floor() as i64;
            assert!(
                q.abs() <= cap,
                "绿：二次裁剪后 level{lvl}={q} 必须回到 ±cap_ℓ={cap} 以内（{reclamped:?}）"
            );
        }
    }


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
        let (leg, trig) = &trace.closed[0];
        assert_eq!(leg.id, held.id, "被关腿 = 持仓腿");
        assert_eq!(trig.bsp_class, 1, "触发归因 = 一类卖候选（reverse_exit_type ⟹ CloseRoot）");
        assert!(!next_active.iter().any(|l| l.id == held.id), "被关腿不入 next_active");
        assert!(trace.silent_drops.is_empty(), "close 认领互斥于静默离场");
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


    /// ★P2 CloseOverlay（PDF §7 C_2）：TW StageII ∧ 持 legacy ShortDiff 腿 ⟹ 关重叠腿
    /// （overlay_closes），普通买候选被消耗当步裁决（不开仓）；非 ShortDiff 腿保留。
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
        // legacy ShortDiff 重叠腿。dir 取 Long：使 tw=None 对照的 p̃ 不落在「候选开仓 +600 与
        // Short 腿 −600 恰好抵消」的巧合点上（P2 关腿 vs 对照保腿的订单差异可观测）。
        let sd_leg = aleg(0, VoiceSide::Long, 7, 7);
        let root_leg = aleg(1, VoiceSide::Long, 3, 3); // 非重叠根腿（保留）
        let sd_ids: std::collections::HashSet<ElementId> = [sd_leg.id].into_iter().collect();
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
            shortdiff_leg_ids: &sd_ids,
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
        let empty_ids: std::collections::HashSet<ElementId> = Default::default();
        let twc = TwStepCtx {
            state: &tw_state,
            policy: &policy,
            risk_mode: RiskMode::Normal,
            shortdiff_leg_ids: &empty_ids,
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
    /// tw_event=EnterEarning（无订单相变）。无 ShortDiff 腿 ⟹ P2 不触发（H=0），落到 P4。
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
        let empty_ids: std::collections::HashSet<ElementId> = Default::default();
        let twc = TwStepCtx {
            state: &tw_state,
            policy: &policy,
            risk_mode: RiskMode::Normal,
            shortdiff_leg_ids: &empty_ids,
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
        let empty_ids: std::collections::HashSet<ElementId> = Default::default();
        let run = |eta_correction: i64| {
            let work = ElementView::from_parts(&tree, candidates.clone());
            let twc = TwStepCtx {
                state: &tw_state,
                policy: &policy,
                risk_mode: RiskMode::Normal,
                shortdiff_leg_ids: &empty_ids,
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
        let sd_ids: std::collections::HashSet<ElementId> = [sd_leg.id].into_iter().collect();
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
            shortdiff_leg_ids: &sd_ids,
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
        let empty_ids: std::collections::HashSet<ElementId> = Default::default();
        let twc = TwStepCtx {
            state: &tw_state,
            policy: &policy,
            risk_mode: RiskMode::Normal,
            shortdiff_leg_ids: &empty_ids,
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


    /// ★∀x ∃! O_{t+1}（spec §16）：同输入 ⟹ 同订单 + 同 p*（确定唯一）。
    #[test]
    fn pi_theta_step_deterministic_unique_order() {
        let bsp = BspPoint { level_origin: 0,
            source_index: 0,
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
        let w = PiThetaWeights::from_risk(&r);
        let a = pi_theta_step(&classification, &[], &[], 0.0, 1, 1000.0, &cfg(), &r, w, KThetaRiskGate::open(), &protocol_hold(), &super::super::super::persistent::PersistentRegistry::new());
        let b = pi_theta_step(&classification, &[], &[], 0.0, 1, 1000.0, &cfg(), &r, w, KThetaRiskGate::open(), &protocol_hold(), &super::super::super::persistent::PersistentRegistry::new());
        assert_eq!(a.2.0, b.2.0, "∀x ∃! O_{{t+1}}：确定唯一订单");
        assert_eq!(a.2.1, b.2.1, "协议轨同样确定且显式");
        assert_eq!(a.1, b.1);
    }


