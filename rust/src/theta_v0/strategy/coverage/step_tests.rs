use super::*;
use super::super::test_support::*;
use crate::theta_v0::types::{Center, BspBits};
use crate::theta_v0::classifier::LevelState;
use crate::theta_v0::classifier::bsp::BspPoint;
use super::super::super::interp::Buckets;

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
        let reg = super::super::super::persistent::PersistentRegistry::new();
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

    /// ★#714 MED-1（语义重放自 kimi 8d8895c652，#512）：coverage 生产边界遇活动集重复 ElementId
    /// 必须 fail-loud（不静默返回双计 p̃/P^sep），补 kimi 侧同名测试在 main 缺失的回归覆盖
    /// （影子评审 `shadow-642-review-20260729.md` MED-1：`#512` 新落防线此前零测试见证）。
    ///
    /// 构造：`prev_active` 含两条字面量相等的持仓腿（同 id、均真边界根、registry 未登记 ⟹
    /// `HeldLegState::Invalidated`）——held 循环对二者各自命中 `Closed|Invalidated` 分支的
    /// `is_boundary_root` 保留子路径，各自新 push 一个 `parent_id:None` 元素入 `raw`，产生同一
    /// `ElementId` 的两个 work idx（"boundary-root-retain" 来源）。
    ///
    /// **架构差异取舍（与 kimi 语义重放的必要偏离，非疏漏）**：main 在 `raw` 层另有更早的
    /// `#183` 硬门（`step.rs:464-471`，`debug_assert!` 检查「raw 内 ElementId 唯一」，kimi
    /// 侧无此对应机制）——本构造下 raw 必产生该重复，**debug 构建**（`cargo test --lib`）先
    /// 命中 `#183`（message 不含重复 ID 细节，只报「唯一性破裂」）；`#183` 是 `debug_assert!`，
    /// **release 构建**编译期消除后才继续到 `#512` 本体（`step.rs:511` 的 `panic!`，与 build
    /// profile 无关恒定触发）——此时 `next_idx` 的两条重复引用已被 `raw_id_idx`（`step.rs:478`
    /// 的 `HashMap`，同 id 只留最后一次 push 的 idx）折叠为**同一个** idx，故 message 呈现
    /// 「idx N(...) 与 idx N(...)」（两侧 idx 值相同），不是 kimi 侧「idx 0 与 idx 1」（两个不同
    /// idx）——kimi 无 `raw_id_idx` 折叠层，其两个 idx 独立保留。两条防线合起来才是 kimi
    /// commit message「release/debug 均 panic」的 main 侧等价形态：debug 由 `#183` 兜底、
    /// release 由 `#512` 本体兜底，本测试对两者均判定为「fail-loud 验证通过」（否则判定为
    /// 静默双计的真回归）。
    ///
    /// 定义依据：影子评审 `shadow-642-review-20260729.md` MED-1；kimi
    /// `step_tests.rs::duplicate_active_id_panics_with_id_indices_and_sources_in_release`；
    /// 留痕 `chanlun/review-results/issue714-642-fixbatch-20260729.md` MED-1 取舍段。
    #[test]
    fn duplicate_active_id_panics_with_id_indices_and_sources_in_release() {
        let duplicate = aleg(0, VoiceSide::Long, 777, 777);
        let prev = [duplicate, duplicate];
        let buckets = Buckets { close: vec![], open: vec![], record: vec![] };
        let tree: Vec<CoverageElement> = vec![];
        let reg = super::super::super::persistent::PersistentRegistry::new();

        let payload = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            coverage_step_from_buckets(view_split(&tree, 0), &prev, &buckets, 1000.0, &cfg(), None, &reg)
        }))
        .expect_err("coverage 生产边界不得对重复活动 ID 静默返回双计结果——必须 panic（#183 debug 早防线或 #512 release 本体）");
        let message = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&str>().copied())
            .expect("panic payload 必须是可审计文本");

        let hit_183_debug_guard = message.contains("raw 内 ElementId 唯一性破裂");
        let hit_512_release_guard = message.contains("重复 ElementId ElementId { level: 0, ordinal: 777 }")
            && message.contains("boundary-root-retain");
        assert!(
            hit_183_debug_guard || hit_512_release_guard,
            "既未命中 #183 debug 早防线也未命中 #512 release 本体，可能已静默返回双计结果；实得 panic message={message}"
        );
    }

    /// ★环5+环6 端到端：Classification（一买点）→ Γ → 解释器 → A_{t+1} → p̃。
    #[test]
    fn classification_end_to_end_ring5_ring6() {
        let reg = super::super::super::persistent::PersistentRegistry::new();
        // L0 一个一类买点（source_index=4）。
        let bsp = BspPoint { source_index: 4,
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
        let buy = BspPoint { source_index: 0,
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
        let sell = BspPoint { source_index: 10,
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

    /// #572 / D1：父 campaign 的入场右端点随 carrier 延伸后不再严格相等时，
    /// 只允许唯一的左开右闭 span 重建结构 seed。
    #[test]
    fn risk_seed_carrier_rebuilds_unique_rho_drift() {
        let carrier = CoverageElement {
            lambda: 0,
            rho: 20,
            eps: VoiceSide::Long,
            level: 1,
            parent: None,
            attached_dir: None,
            id: eid(1, 0),
            parent_id: None,
        };
        let mut seed = aleg(1, VoiceSide::Long, 12, 12);
        seed.id = eid(1, 6);

        let rebuilt = risk_seed_carrier(&[carrier], &seed).expect("唯一 span 应重建 carrier");
        assert_eq!(rebuilt.id, carrier.id);
    }

    /// #572 红线：两个同层 carrier 都包含父 campaign 入场点时身份有歧义，
    /// 必须失败关闭；不能按方向或最窄区间猜一条，避免误杀无关同向腿。
    #[test]
    fn risk_seed_carrier_ambiguous_span_fails_closed() {
        let carriers = [
            CoverageElement {
                lambda: 0,
                rho: 20,
                eps: VoiceSide::Long,
                level: 1,
                parent: None,
                attached_dir: None,
                id: eid(1, 0),
                parent_id: None,
            },
            CoverageElement {
                lambda: 8,
                rho: 16,
                eps: VoiceSide::Long,
                level: 1,
                parent: None,
                attached_dir: None,
                id: eid(1, 1),
                parent_id: None,
            },
        ];
        let mut seed = aleg(1, VoiceSide::Long, 12, 12);
        seed.id = eid(1, 6);

        ancok_probe_reset();
        assert!(
            risk_seed_carrier(&carriers, &seed).is_none(),
            "身份歧义必须失败关闭，不能猜测任一 carrier"
        );
        assert_eq!(
            ancok_probe_snapshot().risk_seed_carrier_ambiguous, 1,
            "#594：歧义 fail-closed 必须计入探针 risk_seed_carrier_ambiguous（此前零可观测）"
        );
    }

    // ── §8b §13 AncOK 持仓准入（639(c)：ReverseOpen 未持父则剔除，不开 naked 逆势仓）──────────
    //   真嵌套塔（L1 Long 父走势）+ 638 附着候选 ⟹ AncOK 在真 Compose 父链上对附着候选剪枝/准入。
