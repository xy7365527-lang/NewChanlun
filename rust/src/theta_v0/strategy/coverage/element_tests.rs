use super::*;
use super::super::test_support::*;
use crate::theta_v0::classifier::recursive_tower::LeveledMove;
use crate::theta_v0::types::Direction;
use crate::theta_v0::classifier::center::UnitRange;

    /// 元素提取：L1 走势 + 3 个真嵌套 L0 子声部 = 4 个元素（1 根 + 3 子，真父子）。
    #[test]
    fn extract_elements_nested_parent_child() {
        let l1 = nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up]);
        let tower = rc_tower(vec![Vec::new(), vec![l1]]); // 索引=级别：L0 空（子由 Compose 带出），L1 一个走势
        let elements = extract_elements(&tower);
        // 1 根（L1 走势）+ 3 子（L0 线段，真嵌套）= 4 元素。
        assert_eq!(elements.len(), 4);
        // 根元素 parent=None（边界胚元 ∂，去根化）。
        assert_eq!(elements[0].parent, None);
        assert_eq!(elements[0].level, 1);
        // 子元素 parent=Some(0)（真父子，非级别差伪造）。
        assert_eq!(elements[1].parent, Some(0));
        assert_eq!(elements[2].parent, Some(0));
        assert_eq!(elements[3].parent, Some(0));
        // 子元素 level=0（descend 级别严格递减）。
        assert_eq!(elements[1].level, 0);
    }

    /// 元素区间 [λ_e,ρ_e)：根覆盖全跨度，子覆盖各自 L0 段（操作区间真坐标）。
    #[test]
    fn element_intervals_from_tower_coords() {
        let l1 = nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up]);
        let tower = rc_tower(vec![Vec::new(), vec![l1]]);
        let elements = extract_elements(&tower);
        // 根：λ=0, ρ=12（窗口首起点..末终点）。
        assert_eq!((elements[0].lambda, elements[0].rho), (0, 12));
        // 子0：λ=0, ρ=4（首 L0 段）。
        assert_eq!((elements[1].lambda, elements[1].rho), (0, 4));
        // 子1：λ=4, ρ=8。
        assert_eq!((elements[2].lambda, elements[2].rho), (4, 8));
    }

    // ── §2b 638 附着判准（hostOf 本级右端点命中；三漏洞防护）─────────────────────

    /// 638 附着：候选继承 hostOf 的真 Compose 父 + 父方向 σ_{p(g)}（V≠Ambient 前提，真父附着）。
    #[test]
    fn attach_inherits_host_compose_parent_dir() {
        let tree = extract_elements(&two_parent_tower());
        // 级别 0、source_index=8（= a1 的 end_index，hostOf=a1，父=compose_a=Long）。
        let (parent, sigma) = attach_bsp_to_tree(&tree, 0, 8);
        assert_eq!(parent, Some(0), "hostOf(a1) 的真 Compose 父 = compose_a(idx0)");
        assert_eq!(sigma, Some(VoiceSide::Long), "sigma_p(g) = 父走势方向 Long");
    }

    /// 638 漏洞②：严格右端点命中——共享端点处归**产出段**（前一走势 ρ 命中），非后一走势 λ。
    #[test]
    fn attach_right_endpoint_归产出段_not_next_start() {
        let tree = extract_elements(&two_parent_tower());
        // source_index=12：a2.ρ=12（产出段，父 compose_a=Long）∧ b0.λ=12（后一走势起点）。
        // 右端点命中 ⟹ host=a2 → (compose_a, Long)，**非** b0（b0.ρ=16，父 compose_b=Short）。
        let (parent, sigma) = attach_bsp_to_tree(&tree, 0, 12);
        assert_eq!(parent, Some(0), "右端归产出段 a2 → 父 compose_a");
        assert_eq!(sigma, Some(VoiceSide::Long), "非后一走势 b0（那会是 Short）");
        // 反证：b0 的右端点是 16，不是 12 ⟹ 12 不附着到 b0。
        let (pb, sb) = attach_bsp_to_tree(&tree, 0, 16);
        assert_eq!((pb, sb), (Some(4), Some(VoiceSide::Short)), "16=b0.ρ ⟹ host=b0,父 compose_b=Short");
    }

    /// 638 漏洞①：级别上下文消歧——同 source_index 跨级共享端点，按候选级别命中本级 host。
    #[test]
    fn attach_level_context_disambiguates_shared_rho() {
        let tree = extract_elements(&two_parent_tower());
        // compose_a.ρ=12（L1）与 a2.ρ=12（L0）共享 ρ=12。
        // 级别 0 ⟹ host=a2（有真父 compose_a=Long）。
        assert_eq!(attach_bsp_to_tree(&tree, 0, 12), (Some(0), Some(VoiceSide::Long)));
        // 级别 1 ⟹ host=compose_a（顶层根，无父）⟹ Ambient（不误命中 L0 host）。
        assert_eq!(attach_bsp_to_tree(&tree, 1, 12), (None, None));
    }

    /// 638 漏洞③：ordinal 坐标身份——结构全等子走势(a2/b0=Seg{Up,5,15})按 (level,ρ) 区分附着。
    #[test]
    fn attach_ordinal_identity_not_rmove_struct_equal() {
        let tree = extract_elements(&two_parent_tower());
        // a2 与 b0 RMove 结构全等。结构相等查表(index_of_in)会把 16 误配 a2(首个结构匹配,父Long)。
        // (level,ρ) 身份 ⟹ ρ=16 唯一命中 b0（父 compose_b=Short），ρ=12 唯一命中 a2（父 Long）。
        assert_eq!(attach_bsp_to_tree(&tree, 0, 16).1, Some(VoiceSide::Short), "ρ=16 → b0 真父 Short");
        assert_eq!(attach_bsp_to_tree(&tree, 0, 12).1, Some(VoiceSide::Long), "ρ=12 → a2 真父 Long");
    }

    /// ★方案D K_i（裁决648）：extract_carrier_forest 含**所有级别**元素（不像 T_i 只最高级根），
    /// dedup 后每 ElementId 唯一、parent 索引有效（codex NO#2 守卫）。
    #[test]
    fn carrier_forest_all_levels_dedup_unique_id() {
        // two_parent_tower：L1 有 compose_a/compose_b（各 3 个 L0 sub），L0 级为空（sub 由 Compose 带出）。
        let tower = two_parent_tower();
        let t_i = extract_elements(&tower); // T_i：只展开最高非空级（L1）根 → 2 根 + 6 子 = 8
        let k_i = extract_carrier_forest(&tower); // K_i：所有级别（L1 同上；L0 空 ⟹ 无额外根）
        // 本塔 L0 级为空（sub_moves 携子），故 K_i 与 T_i 元素数相同（dedup 后），但 ElementId 必唯一。
        let mut ids: Vec<_> = k_i.iter().map(|e| (e.id.level, e.id.ordinal)).collect();
        let n_before = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), n_before, "K_i dedup 后无重复 ElementId（codex NO#2）");
        // parent 索引有效（指向 K_i 内更早元素，父在子前不变量）。
        for (i, e) in k_i.iter().enumerate() {
            if let Some(p) = e.parent {
                assert!(p < i, "parent 索引 {p} 必 < 子索引 {i}（父在子前）");
                assert!(p < k_i.len(), "parent 索引越界");
            }
        }
        // endpoint-complete 见证：每个 L0 子走势的 ρ 在 K_i 中可命中（host^op 不 miss）。
        // a1.ρ=8（L0），compose_a.ρ=12（L1）都应在 K_i。
        let kidx = build_tree_endpoint_index(&k_i);
        assert!(kidx.contains_key(&(0, 8)), "L0 子 ρ=8 在 K_i（endpoint-complete）");
        assert!(kidx.contains_key(&(1, 12)), "L1 根 ρ=12 在 K_i");
    }

    /// ★方案D K_i dedup **真触发**（codex NO#2 核心）：L0 级非空 ∧ 其元素同时是 L1 compose 的 sub
    /// ⟹ 同一 ElementId 被遍历两次（作 L0 根 parent=None + 作 L1 子 parent=Some）。dedup 须保留**带
    /// parent_id 的出现**（子声部父链所需），且最终无重复 ElementId、parent 索引有效。
    #[test]
    fn carrier_forest_dedup_triggers_when_l0_nonempty() {
        let s0 = LeveledMove::from_unit(&unit(0, 4, Direction::Up, 0, 10), eid(0, 0));
        let s1 = LeveledMove::from_unit(&unit(4, 8, Direction::Down, 3, 12), eid(0, 1));
        let s2 = LeveledMove::from_unit(&unit(8, 12, Direction::Up, 5, 15), eid(0, 2));
        let l1 = LeveledMove::compose(&[s0.clone(), s1.clone(), s2.clone()], ctr(0, 12), 1, eid(1, 0));
        // L0 级**非空**（含 s0/s1/s2）+ L1 级含 compose（其 sub_moves 也是 s0/s1/s2，同 ElementId）。
        let tower = rc_tower(vec![vec![s0, s1, s2], vec![l1]]);
        let k_i = extract_carrier_forest(&tower);
        // 4 个唯一元素：L1 根 (1,0) + 3 个 L0 (0,0)/(0,1)/(0,2)——dedup 合并了重复遍历。
        let mut ids: Vec<_> = k_i.iter().map(|e| (e.id.level, e.id.ordinal)).collect();
        let n = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), n, "dedup 后无重复 ElementId");
        assert_eq!(n, 4, "L1 根 + 3 L0 子 = 4 唯一元素（重复遍历被 dedup）");
        // L0 元素保留**带 parent 的出现**（作 L1 子时 parent_id=Some(1,0)），非作根的 None。
        let l0_child = k_i.iter().find(|e| e.id == eid(0, 0)).expect("s0 在 K_i");
        assert_eq!(l0_child.parent_id, Some(eid(1, 0)), "dedup 保留带真 parent_id 的出现（codex NO#2）");
        assert!(l0_child.parent.is_some(), "parent 索引指向 L1 根");
        // parent 索引有效。
        for (i, e) in k_i.iter().enumerate() {
            if let Some(p) = e.parent {
                assert!(p < i && p < k_i.len(), "parent 索引有效");
            }
        }
    }

    /// ★A12 双向映射一致性（648 裁决 D / P1-3）：T_i↪K_i 嵌入 + K_i id/(level,ρ) 唯一——
    /// 对链式塔（two_parent_tower，T_i==K_i）与 orphan 塔（K_i⊋T_i）都成立。
    #[test]
    fn a12_dual_view_consistency_holds() {
        for tower in [two_parent_tower(), orphan_subtree_tower()] {
            let t_i = extract_elements(&tower);
            let k_i = extract_carrier_forest(&tower);
            assert_eq!(dual_view_consistency(&t_i, &k_i), Ok(()), "双视图一致性（L0 代数）");
            assert!(k_i.len() >= t_i.len(), "T_i⊆K_i ⟹ |K_i|>=|T_i|");
        }
        // orphan 塔上 K_i 严格更大（c1 子树 4 元素只在 K_i）。
        let tower = orphan_subtree_tower();
        assert_eq!(
            extract_carrier_forest(&tower).len() - extract_elements(&tower).len(),
            4,
            "orphan c1 子树（c1+s0/s1/s2）仅在 K_i"
        );
    }

    /// ★A12 一致性断言的否定性分支：伪造 K_i（id 重复 / 缺 T_i 元素 / 字段分叉）必 Err。
    #[test]
    fn a12_dual_view_consistency_rejects_forgery() {
        let tower = orphan_subtree_tower();
        let t_i = extract_elements(&tower);
        let k_i = extract_carrier_forest(&tower);
        // id 重复。
        let mut dup = k_i.clone();
        dup.push(k_i[0]);
        assert!(dual_view_consistency(&t_i, &dup).is_err(), "K_i id 重复必拒");
        // T_i 元素缺失（K_i 少一个 T_i 元素）。
        let missing: Vec<CoverageElement> =
            k_i.iter().filter(|e| e.id != t_i[0].id).copied().collect();
        assert!(dual_view_consistency(&t_i, &missing).is_err(), "T_i⊆K_i 破裂必拒");
        // 同 id 字段分叉。
        let mut forked = k_i.clone();
        let pos = forked.iter().position(|e| e.id == t_i[0].id).unwrap();
        forked[pos].rho += 1;
        assert!(dual_view_consistency(&t_i, &forked).is_err(), "同 id 字段分叉必拒");
    }

    /// ★A12 host^struct（T_i，部分函数）vs host^op（K_i，endpoint-complete）分离见证：
    /// orphan 子树上的 bsp 在 T_i 宇宙 host-miss（∂ 根退化，676 子声部恒零根因），在 K_i 宇宙
    /// 严格右端点命中（P2a 保留）真 Compose 父 c1——子声部对冲腿的结构前提就位。
    #[test]
    fn a12_host_op_hits_orphan_frontier_host_struct_misses() {
        let tower = orphan_subtree_tower();
        let t_i = extract_elements(&tower);
        let k_i = extract_carrier_forest(&tower);
        let t_idx = build_tree_endpoint_index(&t_i);
        let k_idx = build_tree_endpoint_index(&k_i);
        // bsp g @ (level=0, source_index=8)=s1.ρ（orphan c1 的中间子；s1 方向 Down、父 c1 方向 Up）。
        // host^struct：T_i 无 s1 ⟹ ⊥（旧生产 = ∂ 根 Ambient，676 子声部结构性不可达）。
        assert_eq!(
            attach_bsp_carrier_indexed(&t_idx, &t_i, 0, 8),
            (None, None, None),
            "host^struct 部分函数：orphan frontier 上 ⊥"
        );
        // host^op：K_i 命中 s1（严格右端点，P2a），真 Compose 父 c1 + carrier id。
        let (parent, attached_dir, carrier_id) = attach_bsp_carrier_indexed(&k_idx, &k_i, 0, 8);
        assert_eq!(carrier_id, Some(eid(0, 1)), "host^op 命中 s1（P2b：宇宙 T_i→K_i）");
        assert_eq!(attached_dir, Some(VoiceSide::Long), "σ_p = c1 外缘 Up = Long");
        let parent_idx = parent.expect("s1 携真 Compose 父 c1");
        assert_eq!(k_i[parent_idx].id, eid(1, 0), "父 carrier=c1（真嵌套，非级别差伪造，547）");
    }

    /// 638 边界：host 未找到（无 ρ==source_index 的本级元素）⟹ (None,None) 去根化 Ambient。
    #[test]
    fn attach_host_not_found_is_ambient() {
        let tree = extract_elements(&two_parent_tower());
        assert_eq!(attach_bsp_to_tree(&tree, 0, 99), (None, None), "无本级 host ⟹ 无真父 ⟹ Ambient");
    }

    /// 638 边界 + tower-export-i guard：tower.len()<2（仅 L0 全根）⟹ host 是根 ⟹ 恒 Ambient。
    #[test]
    fn attach_guard_no_compose_level_is_ambient() {
        let s0 = LeveledMove::from_unit(&unit(0, 4, Direction::Up, 0, 10), eid(0, 0));
        let s1 = LeveledMove::from_unit(&unit(4, 8, Direction::Down, 3, 12), eid(0, 1));
        let tower = rc_tower(vec![vec![s0, s1]]); // 仅 L0，len()==1 < 2（无 Compose 级）
        let tree = extract_elements(&tower);
        // host=s0（ρ=4，level 0）但 s0 是根（parent=None）⟹ 缺塔诚实退化 Ambient。
        assert_eq!(attach_bsp_to_tree(&tree, 0, 4), (None, None), "缺塔（len<2）⟹ host 是根 ⟹ Ambient");
    }

    // ── §3 活动集递归 A_{t+1}=AncOK[(A_t∖D_t)∪B_t]（先关后开 + 祖先闭合）──────────

    /// B_t/D_t：bar t 的开始/结束元素索引（按 λ_e/ρ_e 过滤）。
    #[test]
    fn starting_ending_sets_by_endpoints() {
        let l1 = nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up]);
        let tower = rc_tower(vec![Vec::new(), vec![l1]]);
        let elements = extract_elements(&tower);
        // bar 0：根（λ=0）+ 子0（λ=0）开始。
        let b0 = starting_set(&elements, 0);
        assert!(b0.contains(&0) && b0.contains(&1));
        // bar 4：子1（λ=4）开始；子0（ρ=4）结束。
        assert!(starting_set(&elements, 4).contains(&2));
        assert!(ending_set(&elements, 4).contains(&1));
    }

    /// ★扁平入口 from_classification_levels：从 Classification 的中枢提取根级覆盖元素
    /// （生产入口验证，no-声明膨胀——坐实它真从 levels 产元素）。诚实边界：扁平近似无真嵌套
    /// ⟹ 父=胚元 σ=0，V 恒 Ambient（无 FollowParent/ReverseOpen，短差需真 Compose 塔，用 extract_elements）。
    #[test]
    fn from_classification_levels_flat_root_coverage() {
        use super::super::super::super::classifier::decompose::{MoveBlock, MoveStatus};
        use super::super::super::super::classifier::{Classification, LevelState};
        use super::super::super::super::types::MoveKind;
        let blk = |kind: MoveKind| MoveBlock {
            start_center: 0, end_center: 0, kind, dir: None, status: MoveStatus::Active,
        };
        // L0 一个中枢（盘整），L1 一个中枢（盘整）——两个根级覆盖元素（同级兄弟，无父子）。
        let classification = Classification {
            levels: vec![
                LevelState {
                    moves: vec![blk(MoveKind::Consolidation)],
                    centers: Rc::new(vec![ctr(0, 12)]),
                    ..Default::default()
                },
                LevelState {
                    moves: vec![blk(MoveKind::Trend)],
                    centers: Rc::new(vec![ctr(0, 30)]),
                    ..Default::default()
                },
            ],
        };
        let elements = from_classification_levels(&classification);
        assert_eq!(elements.len(), 2, "两级各一中枢 ⟹ 两个根级覆盖元素");
        // 去根化：扁平入口父=胚元 σ=0 ⟹ V=Ambient（无 RootRole）；不同级别(L0/L1)非同级兄弟 ⟹ 各自 First。
        assert!(elements.iter().all(|e| e.parent.is_none()), "扁平入口全根级（无真父子）");
        assert_eq!(vertical_relation(&elements, 0), Vertical::Ambient);
        assert_eq!(vertical_relation(&elements, 1), Vertical::Ambient);
        assert_eq!(horizontal_relation(&elements, 0), Horizontal::First);
        assert_eq!(horizontal_relation(&elements, 1), Horizontal::First);
        // 级别索引：L0 元素 level=0，L1 元素 level=1。
        assert_eq!(elements[0].level, 0);
        assert_eq!(elements[1].level, 1);
        // 区间来自中枢 [start_index, end_index)。
        assert_eq!((elements[0].lambda, elements[0].rho), (0, 12));
    }

    /// 空塔 ⟹ 空元素集（提取边界）。
    #[test]
    fn empty_tower_yields_empty_elements() {
        let elements = extract_elements(&[]);
        assert!(elements.is_empty());
    }

    // ── §8 环6：解释器三桶 → A_{t+1}=AncOK[(A_t∖𝒟_x)∪ℬ_x] → p̃（桶驱动，闭环 interpret）──


    /// ★codex Q4 确定性 ElementId 跨 bar 稳定测试：全量/增量产同 ID。
    #[test]
    fn element_id_deterministic_full_vs_incremental() {
        // ★task #142 延伸语义诚实重算：旧全重叠 [0,100] fixture 在 canonical 扫描（PDF §5）下被
        // 吸收为 1 个延伸中枢 ⟹ 只有 1 个 ID，测不出接续。改三组「核心分离」fixture：
        // 组1 核心 [max(0,40,40),min(50,200,50)]=[40,50]；组2 首段 lo=55>ZG₁=50 ⟹ non-extension，
        // 核心 [55,150]；组3 首段 lo=155>ZG₂=150 ⟹ non-extension，核心 [155,280]。
        // ⟹ 全量 3 个中枢/3 个上级走势，ID (1,0)(1,1)(1,2)。
        let ranges = [
            (0, 50), (40, 200), (40, 50),
            (55, 150), (52, 180), (55, 160),
            (155, 300), (152, 280), (155, 290),
        ];
        let units: Vec<UnitRange> = ranges
            .iter()
            .enumerate()
            .map(|(i, &(lo, hi))| {
                unit(i * 4, i * 4 + 4, if i % 2 == 0 { Direction::Up } else { Direction::Down }, lo, hi)
            })
            .collect();
        let moves: Vec<LeveledMove> = units
            .iter()
            .enumerate()
            .map(|(i, u)| LeveledMove::from_unit(u, ElementId { level: 0, ordinal: i as u64 }))
            .collect();
        let (_fc, full_upper, _) = compose_level(&units, &moves, true, 1);
        assert_eq!(full_upper.len(), 3, "三组核心分离 ⟹ 3 个中枢/3 个上级走势");
        // 增量 resume(prefix_count=0) == 全量。
        let (_tc, tail_upper, _, _, _) = compose_level_resume(&units, &moves, true, 1, 0, 0);
        assert_eq!(full_upper.len(), tail_upper.len());
        for (f, t) in full_upper.iter().zip(tail_upper.iter()) {
            assert_eq!(f.id, t.id, "全量/增量产同 ElementId（确定性）");
        }
        // 增量续扫：前 6 段（产 2 中枢，末位开放——其延伸终止于 units 用尽而非 non-extension）+
        // 追加 3 段。★task #142 唯一合法 resume 协议：pop 末位开放中枢的上级走势 + 从 resume_from
        // （其 seed 起点）重扫；tail ID 接续 prefix_count（pop 后 =1）⟹ 重算中枢仍得 ID (1,1)。
        let (_pc, mut prefix_upper, _, _m6, cursor6) = compose_level_resume(&units[..6], &moves[..6], true, 1, 0, 0);
        if cursor6.resume_from < cursor6.consumed {
            prefix_upper.pop();
        }
        let (_tc2, tail_upper2, _, _mt2, _) =
            compose_level_resume(&units, &moves, true, 1, cursor6.resume_from, prefix_upper.len());
        let mut comb = prefix_upper.clone();
        comb.extend(tail_upper2);
        assert_eq!(comb.len(), full_upper.len());
        for (f, c) in full_upper.iter().zip(comb.iter()) {
            assert_eq!(f.id, c.id, "增量续扫 ID 接续前缀（全量/增量产同 ID）");
        }
    }

    // ── §9 环7 π_Θ：J_x + 𝒦_Θ + LexArgmin + Schedule_Θ → 唯一订单 ──────────────

