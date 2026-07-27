use super::*;
use super::super::test_support::*;
use crate::theta_v0::types::{Center, BspBits};
use crate::theta_v0::classifier::LevelState;
use crate::theta_v0::classifier::bsp::BspPoint;
use super::super::super::interp::ActiveLeg;

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

    /// ★#80 DA-Q2/O-8：五个协议成熟度桶都与订单轨作积；切换协议证据不得改变同输入订单。
    #[test]
    fn protocol_event_does_not_reorder_p1_p10_order_track_bitexact() {
        use super::super::super::protocol::{
            CompletedCenterRef, EvidenceRef, PostTrendKey, PreTrendKey, ProtocolMaturity,
        };

        let (classification, tower) = buy_gamma();
        let previous = Center {
            zd: 100,
            zg: 110,
            dd: 90,
            gg: 120,
            start_index: 0,
            end_index: 10,
        };
        let successor = Center {
            zd: 130,
            zg: 140,
            dd: 125,
            gg: 145,
            start_index: 20,
            end_index: 30,
        };
        let pre = PreTrendKey::new(
            0,
            CompletedCenterRef::from_center(&previous),
            EvidenceRef::new(11, 0),
            EvidenceRef::new(12, 0),
        );
        let post = PostTrendKey::new(
            0,
            CompletedCenterRef::from_center(&previous),
            EvidenceRef::new(20, 0),
            EvidenceRef::new(21, 0),
        );
        let events = [
            ProtocolEvent::hold(0),
            ProtocolEvent::type3_candidate(pre),
            ProtocolEvent::completed_reentry(post),
            ProtocolEvent::completed_move_evidence(post),
            ProtocolEvent::settled_center_relation(0, &previous, &successor),
        ];
        let expected_maturity = [
            ProtocolMaturity::Hold,
            ProtocolMaturity::Type3Candidate,
            ProtocolMaturity::CompletedRetraceReentry,
            ProtocolMaturity::CompletedMoveEvidence,
            ProtocolMaturity::SettledCenterRelation,
        ];
        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        let mut baseline_order = None;
        let mut baseline_p1_order = None;
        for (event, maturity) in events.into_iter().zip(expected_maturity) {
            let protocol = ProtocolEventSet::hold(0).with(event);
            let registry = super::super::super::persistent::PersistentRegistry::new();
            let (_, _, (order, emitted)) = pi_theta_step(
                &classification,
                &tower,
                &[],
                0.0,
                5,
                1000.0,
                &cfg(),
                &r,
                w,
                KThetaRiskGate::open(),
                &protocol,
                &registry,
            );
            assert_eq!(emitted.maturity(), maturity, "每个协议桶均有显式事件像");
            assert_eq!(*baseline_order.get_or_insert(order), order, "协议轨不得改 P8 订单");

            let force_flat = KThetaRiskGate {
                force_flat: true,
                stop_long: false,
                stop_short: false,
                no_increase_cap: None,
            };
            let (_, _, (p1_order, p1_emitted)) = pi_theta_step(
                &classification,
                &tower,
                &[],
                600.0,
                5,
                1000.0,
                &cfg(),
                &r,
                w,
                force_flat,
                &protocol,
                &registry,
            );
            assert_eq!(p1_emitted.maturity(), maturity);
            assert_eq!(
                *baseline_p1_order.get_or_insert(p1_order),
                p1_order,
                "协议轨不得改 P1 强平订单",
            );
        }
        let order = baseline_order.expect("P8 基线订单");
        assert_eq!((order.action, order.qty, order.exec_index), (StrictAction::Buy, 600, 5));
        let p1_order = baseline_p1_order.expect("P1 基线订单");
        assert_eq!((p1_order.action, p1_order.qty, p1_order.exec_index), (StrictAction::Close, 600, 5));
    }

    /// ★#81 DB-B/D-7：默认不激活时，即使事件轨携 CenterOscillation，订单轨仍逐字段 bit-exact。
    /// （#282：事件轨载体自 candidate 改为 [`PanDivTrigger`] 触发事件；开关 frozen 纪律不变。）
    #[test]
    fn center_oscillation_default_inactive_order_track_bitexact() {
        use super::super::super::oscillation::{
            ConsolidationDivergenceEvidence, OscillationCenterRef, OscillationEvidenceRef,
            PanDivTrigger,
        };

        let (classification, tower) = buy_gamma();
        let risk = rcfg();
        let weights = PiThetaWeights::from_risk(&risk);
        let config = super::super::super::super::config::ThetaConfig::default();
        assert!(!config.center_oscillation.enabled);
        let registry = super::super::super::persistent::PersistentRegistry::new();
        let baseline = pi_theta_step(
            &classification,
            &tower,
            &[],
            0.0,
            5,
            1000.0,
            &cfg(),
            &risk,
            weights,
            KThetaRiskGate::open(),
            &ProtocolEventSet::hold(0),
            &registry,
        );

        let trigger = PanDivTrigger::from_gated_pan_div(
            0,
            VoiceSide::Long,
            OscillationCenterRef::new(0, 10),
            ConsolidationDivergenceEvidence::new(OscillationEvidenceRef::new(11, 0)),
        )
        .unwrap();
        let protocol = ProtocolEventSet::hold(0).with_center_oscillation(trigger);
        let observed = pi_theta_step(
            &classification,
            &tower,
            &[],
            0.0,
            5,
            1000.0,
            &cfg(),
            &risk,
            weights,
            KThetaRiskGate::open(),
            &protocol,
            &registry,
        );
        assert_eq!(observed.0, baseline.0);
        assert_eq!(observed.1, baseline.1);
        assert_eq!(observed.2.0, baseline.2.0);
        assert_eq!(protocol.center_oscillation_trigger(), Some(trigger));
    }

    /// ★#80 L0/O-8：无方向、无候选、无订单也仍返回 `(OrderDecision, ProtocolEvent)` 的两个显式值。
    #[test]
    fn order_and_protocol_product_total() {
        let classification = Classification::default();
        let risk = rcfg();
        let (_, p_star, (order, event)) = pi_theta_step(
            &classification,
            &[],
            &[],
            0.0,
            17,
            1000.0,
            &cfg(),
            &risk,
            PiThetaWeights::from_risk(&risk),
            KThetaRiskGate::open(),
            &ProtocolEventSet::hold(0),
            &super::super::super::persistent::PersistentRegistry::new(),
        );
        assert_eq!(p_star, 0.0, "dir=None/无候选必须有确定目标仓位");
        assert_eq!((order.action, order.qty, order.exec_index), (StrictAction::Wait, 0, 17));
        assert_eq!(event, ProtocolEvent::hold(0), "Hold 是显式协议像，不以 Option/缺省表达");
    }

    /// ★#80 L0/O-8：PendingRetrace 可在订单 qty=0 时独立可见，不能被订单 Hold 吞掉。
    #[test]
    fn zero_qty_protocol_event_remains_observable() {
        use super::super::super::protocol::{CompletedCenterRef, EvidenceRef, PostTrendKey, ProtocolMaturity};

        let previous = Center {
            zd: 100,
            zg: 110,
            dd: 90,
            gg: 120,
            start_index: 0,
            end_index: 10,
        };
        let post = PostTrendKey::new(
            0,
            CompletedCenterRef::from_center(&previous),
            EvidenceRef::new(20, 0),
            EvidenceRef::new(21, 0),
        );
        let protocol = ProtocolEventSet::hold(0).with(ProtocolEvent::completed_retrace(post));
        let risk = rcfg();
        let (_, _, (order, event)) = pi_theta_step(
            &Classification::default(),
            &[],
            &[],
            0.0,
            23,
            1000.0,
            &cfg(),
            &risk,
            PiThetaWeights::from_risk(&risk),
            KThetaRiskGate::open(),
            &protocol,
            &super::super::super::persistent::PersistentRegistry::new(),
        );
        assert_eq!((order.action, order.qty), (StrictAction::Wait, 0));
        assert_eq!(event.maturity(), ProtocolMaturity::CompletedRetraceReentry);
        assert!(event.reasons().contains(super::super::super::protocol::ReasonSet::COMPLETED_RETRACE));
    }

    /// ★Q2 close_pred 折 𝒦_Θ：force_flat ⟹ 𝒦_Θ={0} ⟹ p*=0（强平，单一决策出口产平仓 O）。
    #[test]
    fn pi_theta_position_force_flat_gate_clamps_to_zero() {
        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        // 无门：p̃=600 cap 内 ⟹ p*=600。force_flat 门：𝒦_Θ={0} ⟹ p*=0（不论 p̃）。
        let open = pi_theta_position(600.0, 600.0, 1000.0, &r, w, KThetaRiskGate::open());
        assert!((open - 600.0).abs() < 1e-9, "全开门 ⟹ p*=600");
        let flat = KThetaRiskGate { force_flat: true, stop_long: false, stop_short: false, no_increase_cap: None };
        let p_star = pi_theta_position(600.0, 600.0, 1000.0, &r, w, flat);
        assert_eq!(p_star, 0.0, "force_flat ⟹ 𝒦_Θ={{0}} ⟹ p*=0（持仓 600 → Close）");
    }

    /// ★M2/M3 no_increase_cap（margin-design §2.8 真改订单流）：持仓 300 + 信号要 800，但净幅上限
    /// 压到当前 |net|=300 ⟹ p*≤300（禁增仓）；无 cap 时 p*=800（对照）。
    #[test]
    fn pi_theta_position_no_increase_cap_forbids_growth() {
        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        // 无 cap：p̃=800 cap=1000 内 ⟹ p*=800。
        let grow = pi_theta_position(800.0, 300.0, 1000.0, &r, w, KThetaRiskGate::open());
        assert!((grow - 800.0).abs() < 1e-9, "无 cap ⟹ p*=800");
        // no_increase_cap=300（当前 |net|）：净幅不得超 300 ⟹ p*≤300（禁增仓，M2/M3 真改订单流）。
        let gate = KThetaRiskGate { force_flat: false, stop_long: false, stop_short: false, no_increase_cap: Some(300.0) };
        let capped = pi_theta_position(800.0, 300.0, 1000.0, &r, w, gate);
        assert!(capped <= 300.0 + 1e-9, "no_increase_cap=300 ⟹ p*≤300（禁增仓），实得 {capped}");
        assert!(capped < grow, "接 no_increase_cap 后订单流确改变（{capped} < {grow}）");
    }

    /// ★Q2 stop_long 门：禁净多 ⟹ 持多 p_t=600 + p̃=600（信号仍要多）⟹ p*=0（止损经 𝒦_Θ 平多，
    /// 非第二 exit 出口）。stop_short 不触发 ⟹ 净空仍可（对称见证）。
    #[test]
    fn pi_theta_position_stop_long_gate_forbids_net_long() {
        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        let gate = KThetaRiskGate { force_flat: false, stop_long: true, stop_short: false, no_increase_cap: None };
        // 持多 600 + 信号要多 600，但 stop_long 禁净多 ⟹ 𝒦_Θ⊆[−cap,0] ⟹ p*=0（平多，单出口）。
        let p_star = pi_theta_position(600.0, 600.0, 1000.0, &r, w, gate);
        assert!(p_star <= 1e-9, "stop_long ⟹ 禁净多 ⟹ p*≤0（止损平多走 𝒦_Θ，非第二出口），实得 {p_star}");
    }

    /// ★执行层 σ_p = 父容器方向（639，端到端 pi_theta_step；取代旧"活动父腿"错口径测试）：
    /// per-bar 因果塔含有向 L1 Long 父走势 + L0 逆向卖候选 ⟹ 候选 V=ReverseOpen（来自**父容器方向**，
    /// **非持仓**——空 `prev_active` 仍 ReverseOpen，坐实 σ 来源 ⊥ 持仓）。
    ///
    /// ★两机制正交端到端坐实（639，AncOK 已接入）：① **σ_p 来源**=ReverseOpen（`assemble_gamma_with_tower`，
    /// 不接受 active ⟹ 与持仓无关）；② **§13 AncOK 持仓准入**=空 `prev_active`（未持父）⟹ ReverseOpen 子腿
    /// 被剪枝 ⟹ pi_theta_step 产 **Wait/qty=0**（不开 naked 逆势仓，639(c)）。σ 仍分类 ReverseOpen 但准入
    /// 剔除——正是两正交机制（σ 用因果塔，准入用持仓台账）。
    #[test]
    fn pi_theta_step_reverse_open_from_parent_container_not_position() {
        use super::super::super::interp::assemble_gamma_with_tower;
        // per-bar 因果塔：L1 Long 父走势（结构对象；3 个 L0 子，sub(8,12) 右端点 ρ=12）。
        let tower = rc_tower(vec![
            Vec::new(),
            vec![nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up])],
        ]);
        // L0 卖候选 source_index=12 ⟹ host=sub(8,12)（ρ=12）⟹ 真父 L1 Long ⟹ σ_p=Long。
        let sell = BspPoint { level_origin: 0,
            source_index: 12,
            bits: BspBits { sell1: true, ..Default::default() },
            pivot_low: 0, pivot_high: 210,
            center: Some(crate::theta_v0::classifier::bsp::OwnerRef::Center(Center { zd: 100, zg: 200, dd: 90, gg: 210, start_index: 0, end_index: 12 })),
            struct_break_dir: None,
            force: None,
        };
        let classification = Classification {
            levels: vec![LevelState { bsp: Rc::new(vec![sell]), ..Default::default() }],
        };
        // ★639 核心坐实：空 prev_active（未持仓）+ 有向父容器 ⟹ ReverseOpen（σ_p=父容器方向，非持仓父腿）。
        let gamma = assemble_gamma_with_tower(&classification, &tower);
        assert_eq!(gamma[0].dir, VoiceSide::Short);
        assert_eq!(
            gamma[0].role.v,
            Vertical::ReverseOpen,
            "L0 卖 δ=Short = −σ_p（父容器 L1 Long）⟹ ReverseOpen（639：来自父容器方向，未持仓仍成立）"
        );
        // 端到端 pi_theta_step（GAP-5 入场 + 父容器 σ_p + §13 AncOK 持仓准入 + 风控门全开）。
        // 空 prev_active（未持父）⟹ ReverseOpen 子腿被 AncOK 剪枝 ⟹ Wait/qty=0（639(c)：不开 naked 逆势仓）。
        let r = rcfg();
        let (a, p_star, (order, _protocol_event)) = pi_theta_step(
            &classification, &tower, &[], 0.0, 9, 1000.0, &cfg(), &r,
            PiThetaWeights::from_risk(&r), KThetaRiskGate::open(), &protocol_hold(),
            &super::super::super::persistent::PersistentRegistry::new(),
        );
        assert!(a.is_empty(), "未持父 ⟹ ReverseOpen 子腿 AncOK 剪枝 ⟹ A_{{t+1}} 空");
        assert_eq!(p_star, 0.0, "无活动腿 ⟹ p*=0（不开仓）");
        assert_eq!(order.action, StrictAction::Wait, "未持父 ⟹ ReverseOpen 剔除 ⟹ Wait（639(c)）");
        assert_eq!(order.qty, 0, "Wait ⟹ qty=0（不建 naked 逆势仓）");
        assert_eq!(order.exec_index, 9, "订单携 exec_index（延迟成交 bar）");
    }

    /// **★工位 4d L1 bit-exact 守卫：注入缓存 base 索引 vs fallback 现建逐 bar 对拍**（真实 CL）。
    ///
    /// 热点①（`strategy_target_legs` 兄弟索引）+ ②（`held_leg_tree_index` ID 索引）改为 base 段复用
    /// 缓存 `Rc` 索引（runner 从 TreeCache 注入）。本守卫逐 bar 跑**两条并行账本**：
    /// - `with`：`ElementView::with_base_indices` 注入缓存 sibling/id 索引（生产路径）。
    /// - `without`：不注入 ⟹ `coverage_step_from_buckets` fallback 现建（旧路径）。
    /// 两路径各自独立演进 `prev_active`，断言每 bar `(next_active, p_tilde)` 逐字节相等。任何缓存路径
    /// 与现建路径的发散（split partition_point ≠ 合并 partition_point、id_idx 覆盖范围错位）立即捕获。
    ///
    /// 认识论 L1（formalization-validity-domain 231号）：管线正确性验证（增量缓存 == 全量现建），
    /// 非 L2 alpha。bit-exact == 旧 [`operation_role_indexed`]/[`build_tree_id_index`] 现建逻辑。
    #[test]
    #[ignore = "工位 4d L1 bit-exact：注入缓存 vs 现建逐 bar 对拍；需 CL；--release --ignored"]
    fn bit_exact_cached_indices_vs_fallback() {
        use super::super::super::super::backtest::data;
        use super::super::super::super::config::ThetaConfig;
        use super::super::super::super::{classifier, parser};
        use super::super::super::interp::{self, TreeCache};
        let config = ThetaConfig::default();
        let ds = data::load_by_symbol("CL", &config).expect("CL");
        let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
        let n = 4000.min(oos.bars.len());
        let voice = config.voice.clone();
        let mut cache = TreeCache::new();
        let mut reg_with = super::super::super::persistent::PersistentRegistry::new();
        let mut reg_without = super::super::super::persistent::PersistentRegistry::new();
        let mut prev_with: Vec<ActiveLeg> = Vec::new();
        let mut prev_without: Vec<ActiveLeg> = Vec::new();
        let mut hits = 0usize;
        for i in 0..n {
            let l0 = parser::parse_layer(&oos.bars[..=i], &config);
            let (cls, tower) = classifier::classify_with_tower(&l0, &config);
            let (tree, candidates, gamma) =
                interp::coverage_elements_and_gamma_with_tower_cached(&cls, &tower, &mut Some(&mut cache));

            // with：注入缓存 base 索引（生产路径）。
            let mut work_with = ElementView::from_parts(&tree, candidates.clone());
            if let Some((sib, id)) = cache.tree_sibling_and_id() {
                work_with = work_with.with_base_indices(sib, id);
                hits += 1;
            }
            let (next_with, p_with) =
                coverage_step_prebuilt(work_with, &gamma, &prev_with, 1000.0, &voice, None, &reg_with);

            // without：不注入 ⟹ fallback 现建（旧路径）。
            let work_without = ElementView::from_parts(&tree, candidates);
            let (next_without, p_without) =
                coverage_step_prebuilt(work_without, &gamma, &prev_without, 1000.0, &voice, None, &reg_without);

            assert_eq!(
                next_with, next_without,
                "bar {i}: 缓存索引路径 next_active ≠ 现建路径（split partition_point/id_idx 发散）"
            );
            assert_eq!(
                p_with.to_bits(), p_without.to_bits(),
                "bar {i}: 缓存索引路径 p_tilde ≠ 现建路径（bit-exact 破裂）"
            );

            reg_with.merge_in_place(
                ElementView::from_parts(&tree, Vec::new()).as_contiguous().as_ref(), &next_with);
            reg_without.merge_in_place(
                ElementView::from_parts(&tree, Vec::new()).as_contiguous().as_ref(), &next_without);
            prev_with = next_with;
            prev_without = next_without;
        }
        eprintln!("bit_exact_cached_indices_vs_fallback: {n} bars 全部 with==without，{hits} bars 缓存命中");
    }
