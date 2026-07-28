use super::*;
use super::super::test_support::*;
use crate::theta_v0::types::Center;
use crate::theta_v0::classifier::bsp::BspPoint;
use crate::theta_v0::classifier::LevelState;
use crate::theta_v0::types::BspBits;

    /// ★环6 开启：空 A_t + ℬ_x 一个买候选 ⟹ A_{t+1} 含该腿，p̃ = base×w[0]（根 depth 0）。
    #[test]
    fn buckets_open_creates_active_leg_and_target() {
        let reg = super::super::super::persistent::PersistentRegistry::new();
        let buckets = Buckets {
            close: vec![],
            open: vec![cand(0, 5, VoiceSide::Long, 0)],
            record: vec![],
        };
        let (active, p) = coverage_step_from_buckets(view_split(&flat_elements(&buckets.open), 0), &[], &buckets, 1000.0, &cfg(), None, &reg);
        assert_eq!(active.len(), 1, "ℬ_x 开启 ⟹ A_{{t+1}} 一条新腿");
        assert_eq!(active[0], aleg(0, VoiceSide::Long, 5, 5 ));
        // p̃ = 1000×w_depth(0)=1000×0.60=600（根 depth 0，多腿正号）。
        assert!((p - 600.0).abs() < 1e-9, "p̃ = base×w[0] = 600（单多腿）");
    }


    /// ★环6 关闭：A_t 一条 Long 腿 + 𝒟_x={该腿} ⟹ A_{t+1}=∅，p̃=0（先关后开）。
    #[test]
    fn buckets_close_removes_active_leg() {
        let reg = super::super::super::persistent::PersistentRegistry::new();
        let leg = aleg(0, VoiceSide::Long, 3, 3 );
        let buckets = Buckets {
            close: vec![leg], // 𝒟_x ⊆ A_t
            open: vec![],
            record: vec![],
        };
        let (active, p) = coverage_step_from_buckets(view_split(&flat_elements(&buckets.open), 0), &[leg], &buckets, 1000.0, &cfg(), None, &reg);
        assert!(active.is_empty(), "𝒟_x 关闭活动腿 ⟹ A_{{t+1}} 空");
        assert_eq!(p, 0.0, "无活动腿 ⟹ p̃=0");
    }


    /// ★环6 净额聚合：A_{t+1} = {Long, Short} ⟹ p̃ = +600 −600 = 0（方向聚合，多空抵消）。
    #[test]
    fn buckets_target_nets_long_and_short() {
        let reg = super::super::super::persistent::PersistentRegistry::new();
        let buckets = Buckets {
            close: vec![],
            open: vec![
                cand(0, 1, VoiceSide::Long, 0),
                cand(1, 2, VoiceSide::Short, 1),
            ],
            record: vec![],
        };
        let (active, p) = coverage_step_from_buckets(view_split(&flat_elements(&buckets.open), 0), &[], &buckets, 1000.0, &cfg(), None, &reg);
        assert_eq!(active.len(), 2);
        // 两腿同根 depth 0（w[0]=0.60）：Long +600，Short −600 ⟹ 净 0。
        assert!(p.abs() < 1e-9, "p̃ = +600 −600 = 0（方向净额聚合）");
    }


    /// ★环6 先关后开 + 保留：A_t={legA, legB}，𝒟_x={legA}，ℬ_x={candC} ⟹ A_{t+1}={legB, legC}。
    #[test]
    fn buckets_close_then_open_keeps_survivor() {
        let reg = super::super::super::persistent::PersistentRegistry::new();
        let leg_a = aleg(0, VoiceSide::Long, 1, 1 );
        let leg_b = aleg(1, VoiceSide::Short, 2, 2 );
        let buckets = Buckets {
            close: vec![leg_a],                          // 关 legA
            open: vec![cand(0, 9, VoiceSide::Long, 0)],  // 开 candC（level 0 Long）
            record: vec![],
        };
        let (active, _p) = coverage_step_from_buckets(view_split(&flat_elements(&buckets.open), 0), &[leg_a, leg_b], &buckets, 1000.0, &cfg(), None, &reg);
        assert!(!active.contains(&leg_a), "legA 被 𝒟_x 关闭");
        assert!(active.contains(&leg_b), "legB（不在 𝒟_x）保留");
        assert!(
            active.contains(&aleg(0, VoiceSide::Long, 9, 9 )),
            "candC 被 ℬ_x 开启"
        );
        assert_eq!(active.len(), 2);
    }


    /// ★环6 AncOK 扁平恒等（诚实有效域）：桶路径活动腿全独立根（parent=∂）⟹ Anc=∅ ⟹ 不剔任何腿。
    /// （真嵌套塔剔孤儿在 §3 active_set_step 已验；本桶路径无塔故恒等——MEMORY tower-export-bridge 缺口）。
    #[test]
    fn buckets_ancok_identity_on_flat_roots() {
        let reg = super::super::super::persistent::PersistentRegistry::new();
        let legs = [
            aleg(0, VoiceSide::Long, 0, 0 ),
            aleg(1, VoiceSide::Long, 1, 1 ),
            aleg(2, VoiceSide::Short, 2, 2 ),
        ];
        let buckets = Buckets { close: vec![], open: vec![], record: vec![] };
        let (active, _p) = coverage_step_from_buckets(view_split(&flat_elements(&buckets.open), 0), &legs, &buckets, 1000.0, &cfg(), None, &reg);
        assert_eq!(active.len(), 3, "扁平根全保留（AncOK 恒等——无父子可剔孤儿）");
        let _reg = super::super::super::persistent::PersistentRegistry::new();
    }


    /// ★环6 immutable：coverage_step_from_buckets 不 mutate prev_active。
    #[test]
    fn buckets_step_immutable_prev_active() {
        let reg = super::super::super::persistent::PersistentRegistry::new();
        let prev = vec![aleg(0, VoiceSide::Long, 0, 0 )];
        let snapshot = prev.clone();
        let buckets = Buckets {
            close: vec![],
            open: vec![cand(1, 3, VoiceSide::Short, 0)],
            record: vec![],
        };
        let _ = coverage_step_from_buckets(view_split(&flat_elements(&buckets.open), 0), &prev, &buckets, 1000.0, &cfg(), None, &reg);
        assert_eq!(prev, snapshot, "桶驱动递归不 mutate prev_active（纯函数）");
    }


    /// 𝒦_x（record 桶）不进活动集（spec「记录但暂不执行」）。
    #[test]
    fn buckets_record_excluded_from_active_set() {
        let reg = super::super::super::persistent::PersistentRegistry::new();
        let buckets = Buckets {
            close: vec![],
            open: vec![],
            record: vec![cand(0, 7, VoiceSide::Long, 0)], // 𝒦_x：记录不执行
        };
        let (active, p) = coverage_step_from_buckets(view_split(&flat_elements(&buckets.open), 0), &[], &buckets, 1000.0, &cfg(), None, &reg);
        assert!(active.is_empty(), "𝒦_x 不入活动集");
        assert_eq!(p, 0.0);
    }


    /// 空三桶 + 空 A_t ⟹ A_{t+1}=∅，p̃=0（边界）。
    #[test]
    fn buckets_empty_yields_empty_and_zero() {
        let buckets = Buckets { close: vec![], open: vec![], record: vec![] };
        let reg = super::super::super::persistent::PersistentRegistry::new();
        let (active, p) = coverage_step_from_buckets(view_split(&flat_elements(&buckets.open), 0), &[], &buckets, 1000.0, &cfg(), None, &reg);
        assert!(active.is_empty());
        assert_eq!(p, 0.0);
    }


    /// ★环5+环6 端到端：Classification（一买点）→ Γ → 解释器 → A_{t+1} → p̃。
    #[test]
    fn classification_end_to_end_ring5_ring6() {
        let reg = super::super::super::persistent::PersistentRegistry::new();
        // L0 一个一类买点（source_index=4）。
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
        // 空 A_t：买候选开启 ⟹ A_{t+1} 一条 Long 腿，p̃ = 600。
        // 空塔（tower &[]）⟹ 候选父=∂ ⟹ Ambient（与扁平一致）；本测试只验开腿/p̃，角色不约束。
        let (active, p) = coverage_step_classification(&classification, &[], &[], 1000.0, &cfg(), None, &reg);
        assert_eq!(active.len(), 1, "买点 ℬ_x 开启 ⟹ 一条活动腿");
        assert_eq!(active[0].dir, VoiceSide::Long);
        assert_eq!(active[0].source_index, 4);
        assert!((p - 600.0).abs() < 1e-9, "p̃ = base×w[0] = 600");
    }


    /// ★环5↔环6 闭环：A_{t+1} 回喂 interpret——持仓 Long 遇反向卖点 ⟹ 关闭，A_{t+2}=∅。
    #[test]
    fn ring6_active_set_feeds_back_into_interpret() {
        let reg = super::super::super::persistent::PersistentRegistry::new();
        // bar t：买点开 Long。
        let buy = BspPoint { level_origin: 0,
            source_index: 0,
            bits: BspBits { buy1: true, ..Default::default() },
            pivot_low: 90, pivot_high: 0,
            center: Some(crate::theta_v0::classifier::bsp::OwnerRef::Center(Center { zd: 100, zg: 200, dd: 90, gg: 210, start_index: 0, end_index: 9 })),
            struct_break_dir: None,
            force: None,
        };
        let c_buy = Classification { levels: vec![LevelState { bsp: Rc::new(vec![buy]), ..Default::default() }] };
        let (active_t1, _) = coverage_step_classification(&c_buy, &[], &[], 1000.0, &cfg(), None, &reg);
        assert_eq!(active_t1.len(), 1, "买点开 Long 腿");
        // bar t+1：卖点（反向）→ A_{t+1} 回喂 interpret ⟹ 关闭 Long 腿 ⟹ A_{t+2}=∅。
        let sell = BspPoint { level_origin: 0,
            source_index: 10,
            bits: BspBits { sell1: true, ..Default::default() },
            pivot_low: 0, pivot_high: 210,
            center: Some(crate::theta_v0::classifier::bsp::OwnerRef::Center(Center { zd: 100, zg: 200, dd: 90, gg: 210, start_index: 0, end_index: 9 })),
            struct_break_dir: None,
            force: None,
        };
        let c_sell = Classification { levels: vec![LevelState { bsp: Rc::new(vec![sell]), ..Default::default() }] };
        let (active_t2, p2) = coverage_step_classification(&c_sell, &[], &active_t1, 1000.0, &cfg(), None, &reg);
        assert!(active_t2.is_empty(), "反向卖点关闭持仓 Long（𝒟_x）⟹ A_{{t+2}}=∅（闭环）");
        assert_eq!(p2, 0.0, "无活动腿 ⟹ p̃=0");
    }

    // ── §8b §13 AncOK 持仓准入（639(c)：ShortDiff 未持父则剔除，不开 naked 逆势仓）──────────
    //   真嵌套塔（L1 Long 父走势）+ 638 附着候选 ⟹ AncOK 在真 Compose 父链上对附着候选剪枝/准入。

/// ★#446：子腿先恢复父腿、父腿随后自行重注册时，必须复用 restore 已物化的同 ID 槽位。
#[test]
fn held_leg_reregister_reuses_restore_pushed_idx_no_duplicate_id() {
    let child = eid(0, 900);
    let parent = eid(1, 901);
    let leg_child = ActiveLeg {
        level: 0,
        dir: VoiceSide::Long,
        source_index: 30,
        lambda: 20,
        id: child,
        parent_id: Some(parent),
        is_boundary_root: false,
        op_parent: Some(parent),
    };
    let leg_parent = ActiveLeg {
        level: 1,
        dir: VoiceSide::Long,
        source_index: 30,
        lambda: 20,
        id: parent,
        parent_id: None,
        is_boundary_root: true,
        op_parent: None,
    };
    let prev = [leg_child, leg_parent];
    let buckets = Buckets {
        close: vec![],
        open: vec![],
        record: vec![],
    };
    let tree: Vec<CoverageElement> = vec![];
    let reg = super::super::super::persistent::PersistentRegistry::new().merge(&tree, &prev);

    let (active, _p) =
        coverage_step_from_buckets(view_split(&tree, 0), &prev, &buckets, 1000.0, &cfg(), None, &reg);

    assert!(active.iter().any(|leg| leg.id == child));
    assert_eq!(
        active.iter().filter(|leg| leg.id == parent).count(),
        1,
        "restore 祖先腿与 held 重注册必须共用一个 ElementId 槽；实得 {active:?}"
    );
}

/// ★#512 MED-1：release 生产边界遇活动集重复 ID 必须 fail-loud，禁返回已双计的 p̃/P^sep。
#[test]
fn duplicate_active_id_panics_with_id_indices_and_sources_in_release() {
    let duplicate = aleg(0, VoiceSide::Long, 777, 777);
    let prev = [duplicate, duplicate];
    let buckets = Buckets {
        close: vec![],
        open: vec![],
        record: vec![],
    };
    let tree: Vec<CoverageElement> = vec![];
    let reg = super::super::super::persistent::PersistentRegistry::new();

    let payload = std::panic::catch_unwind(|| {
        coverage_step_from_buckets(
            view_split(&tree, 0),
            &prev,
            &buckets,
            1000.0,
            &cfg(),
            None,
            &reg,
        )
    })
    .expect_err("release 生产边界不得返回含重复活动 ID 的双计结果");
    let message = payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&str>().copied())
        .expect("panic payload 必须是可审计文本");

    assert!(
        message.contains("ElementId { level: 0, ordinal: 777 }"),
        "{message}"
    );
    assert!(
        message.contains("idx 0(boundary-root-retain)"),
        "{message}"
    );
    assert!(
        message.contains("idx 1(boundary-root-retain)"),
        "{message}"
    );
}

/// ★#446：同一 carrier 的反向 open 对成对湮灭，净零目标不得留下活动腿。
#[test]
fn open_candidates_same_carrier_id_reverse_pair_annihilates() {
    let carrier = eid(0, 162);
    let parent = eid(1, 33);
    let mk = |eps: VoiceSide| CoverageElement {
        lambda: 42,
        rho: 42,
        eps,
        level: 0,
        parent: None,
        attached_dir: None,
        id: carrier,
        parent_id: Some(parent),
    };
    let elements = vec![mk(VoiceSide::Long), mk(VoiceSide::Short)];
    let buckets = Buckets {
        close: vec![],
        open: vec![
            cand(0, 42, VoiceSide::Long, 0),
            cand(0, 42, VoiceSide::Short, 1),
        ],
        record: vec![],
    };
    let leg_parent = ActiveLeg {
        level: 1,
        dir: VoiceSide::Long,
        source_index: 40,
        lambda: 30,
        id: parent,
        parent_id: None,
        is_boundary_root: true,
        op_parent: None,
    };
    let reg =
        super::super::super::persistent::PersistentRegistry::new().merge(&[], &[leg_parent]);

    let (active, _p) =
        coverage_step_from_buckets(view_split(&elements, 0), &[], &buckets, 1000.0, &cfg(), None, &reg);

    assert_eq!(
        active.iter().filter(|leg| leg.id == carrier).count(),
        0,
        "同 carrier 反向 open 对应成对湮灭；实得 {active:?}"
    );
    assert!(active.iter().any(|leg| leg.id == parent));
}

/// ★#446：同一 carrier 的同向 open 候选只保留首现身份。
#[test]
fn open_candidates_same_carrier_id_same_dir_dedup_first_wins() {
    let carrier = eid(0, 162);
    let parent = eid(1, 33);
    let mk = |lambda: usize| CoverageElement {
        lambda,
        rho: 42,
        eps: VoiceSide::Long,
        level: 0,
        parent: None,
        attached_dir: None,
        id: carrier,
        parent_id: Some(parent),
    };
    let elements = vec![mk(40), mk(42)];
    let buckets = Buckets {
        close: vec![],
        open: vec![
            cand(0, 40, VoiceSide::Long, 0),
            cand(0, 42, VoiceSide::Long, 1),
        ],
        record: vec![],
    };
    let leg_parent = ActiveLeg {
        level: 1,
        dir: VoiceSide::Long,
        source_index: 40,
        lambda: 30,
        id: parent,
        parent_id: None,
        is_boundary_root: true,
        op_parent: None,
    };
    let reg =
        super::super::super::persistent::PersistentRegistry::new().merge(&[], &[leg_parent]);

    let (active, _p) =
        coverage_step_from_buckets(view_split(&elements, 0), &[], &buckets, 1000.0, &cfg(), None, &reg);

    let carrier_legs: Vec<_> = active.iter().filter(|leg| leg.id == carrier).collect();
    assert_eq!(carrier_legs.len(), 1, "同 carrier 同向候选只留首现；实得 {active:?}");
    assert_eq!(carrier_legs[0].lambda, 40, "同向去重须保持首现候选");
}

/// ★#446：held 身份与候选拷贝同 ID 时，候选不能覆盖持仓方向、坐标或操作父。
#[test]
fn held_leg_id_hits_candidate_copy_keeps_held_identity() {
    let carrier = eid(0, 162);
    let parent = eid(1, 33);
    let candidate = CoverageElement {
        lambda: 42,
        rho: 42,
        eps: VoiceSide::Short,
        level: 0,
        parent: None,
        attached_dir: None,
        id: carrier,
        parent_id: Some(parent),
    };
    let elements = vec![candidate];
    let held = ActiveLeg {
        level: 0,
        dir: VoiceSide::Long,
        source_index: 40,
        lambda: 30,
        id: carrier,
        parent_id: Some(parent),
        is_boundary_root: false,
        op_parent: Some(parent),
    };
    let buckets = Buckets {
        close: vec![],
        open: vec![cand(0, 42, VoiceSide::Short, 0)],
        record: vec![],
    };
    let reg =
        super::super::super::persistent::PersistentRegistry::new().merge(&elements, &[held]);

    let (active, _p) =
        coverage_step_from_buckets(view_split(&elements, 0), &[held], &buckets, 1000.0, &cfg(), None, &reg);

    let carrier_legs: Vec<_> = active.iter().filter(|leg| leg.id == carrier).collect();
    assert_eq!(carrier_legs.len(), 1, "held 与候选同 ID 时恰留 held 身份；实得 {active:?}");
    assert_eq!(carrier_legs[0].dir, VoiceSide::Long);
    assert_eq!(carrier_legs[0].source_index, 40);
    assert_eq!(carrier_legs[0].op_parent, Some(parent));
}

/// ★#446 双轴审查锁定：子腿先 restore 父 ID 时也不得复用候选段父拷贝；随后父腿重注册不能双写。
#[test]
fn child_restore_and_held_parent_do_not_reuse_candidate_copy_or_duplicate_id() {
    let child = eid(0, 900);
    let parent = eid(1, 901);
    let candidate_parent = CoverageElement {
        lambda: 42,
        rho: 42,
        eps: VoiceSide::Short,
        level: 1,
        parent: None,
        attached_dir: None,
        id: parent,
        parent_id: None,
    };
    let held_child = ActiveLeg {
        level: 0,
        dir: VoiceSide::Long,
        source_index: 30,
        lambda: 20,
        id: child,
        parent_id: Some(parent),
        is_boundary_root: false,
        op_parent: Some(parent),
    };
    let held_parent = ActiveLeg {
        level: 1,
        dir: VoiceSide::Long,
        source_index: 30,
        lambda: 20,
        id: parent,
        parent_id: None,
        is_boundary_root: true,
        op_parent: None,
    };
    let elements = vec![candidate_parent];
    let prev = [held_child, held_parent];
    let buckets = Buckets {
        close: vec![],
        open: vec![],
        record: vec![],
    };
    let reg =
        super::super::super::persistent::PersistentRegistry::new().merge(&elements, &prev);

    let (active, _p) =
        coverage_step_from_buckets(view_split(&elements, 0), &prev, &buckets, 1000.0, &cfg(), None, &reg);

    let parent_legs: Vec<_> = active.iter().filter(|leg| leg.id == parent).collect();
    assert_eq!(
        parent_legs.len(),
        1,
        "child restore 与 held 父重注册必须共享持久槽，不得复用候选后再双写；实得 {active:?}"
    );
    assert_eq!(parent_legs[0].dir, VoiceSide::Long);
    assert_eq!(parent_legs[0].lambda, 20);
    assert_eq!(parent_legs[0].source_index, 30);
}

/// ★#446：同一合成输入在 debug/release 打出可逐字对拍的数值与活动集唯一性见证。
#[test]
fn coverage_unique_active_set_profile_parity_witness() {
    let carrier = eid(0, 162);
    let parent = eid(1, 33);
    let elements = vec![
        CoverageElement {
            lambda: 40,
            rho: 42,
            eps: VoiceSide::Long,
            level: 0,
            parent: None,
            attached_dir: None,
            id: carrier,
            parent_id: Some(parent),
        },
        CoverageElement {
            lambda: 42,
            rho: 42,
            eps: VoiceSide::Long,
            level: 0,
            parent: None,
            attached_dir: None,
            id: carrier,
            parent_id: Some(parent),
        },
    ];
    let buckets = Buckets {
        close: vec![],
        open: vec![
            cand(0, 40, VoiceSide::Long, 0),
            cand(0, 42, VoiceSide::Long, 1),
        ],
        record: vec![],
    };
    let parent_leg = ActiveLeg {
        level: 1,
        dir: VoiceSide::Long,
        source_index: 40,
        lambda: 30,
        id: parent,
        parent_id: None,
        is_boundary_root: true,
        op_parent: None,
    };
    let reg =
        super::super::super::persistent::PersistentRegistry::new().merge(&[], &[parent_leg]);
    super::super::ancok_probe_reset();

    let (active, p_tilde, sep) = coverage_step_from_buckets_sep(
        view_split(&elements, 0),
        &[],
        &buckets,
        1000.0,
        &cfg(),
        None,
        &reg,
    );

    let mut ids = std::collections::HashSet::new();
    assert!(active.iter().all(|leg| ids.insert(leg.id)));
    let duplicate_id_violations =
        super::super::ancok_probe_snapshot().duplicate_active_id_violations;
    assert_eq!(duplicate_id_violations, 0);
    let active_cells: Vec<_> = active
        .iter()
        .map(|leg| {
            (
                leg.id.level,
                leg.id.ordinal,
                leg.dir,
                leg.lambda,
                leg.source_index,
            )
        })
        .collect();
    let sep_cells: Vec<_> = sep
        .iter()
        .map(|leg| {
            (
                leg.id.level,
                leg.id.ordinal,
                leg.side,
                leg.q_units.to_bits(),
                leg.parent_id,
            )
        })
        .collect();
    eprintln!(
        "COVERAGE_PROFILE_PARITY active={active_cells:?} p_tilde_bits={} sep={sep_cells:?} duplicate_id_violations={duplicate_id_violations}",
        p_tilde.to_bits()
    );
}
