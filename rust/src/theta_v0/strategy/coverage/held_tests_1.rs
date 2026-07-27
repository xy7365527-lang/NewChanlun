use super::*;
use super::super::test_support::*;
use crate::theta_v0::types::Direction;
use crate::theta_v0::classifier::LevelState;

    /// ★(I-1) open 候选父注入测试（depth>0 准入的**真**生产场景，codex 行级裁决坐实，642/644）：
    ///
    /// L2 诊断坐实根因：父 carrier **不在 prev_active 持仓腿**（sd_parent_held=0），也几乎不与子同 bar
    /// 共现（容器 BSP 极稀疏），但**在 persistent registry LiveDetached 存活**（sd_parent_registry_alive
    /// ≈51525）。`cross_bar_held_container_*` 测的是父作 prev_active 持仓腿的路径——**不**覆盖此真实场景。
    /// 本测试覆盖：**prev_active 为空、父仅 registry-live** ⟹ open 候选父注入必须从 registry 恢复祖先链
    /// 才能让子腿 AncOK 准入。
    ///
    /// **RED（修复前）**：open 候选路径从不调 `restore_ancestor_chain_from_registry` ⟹ 父 carrier 不在
    /// raw（既非 held 又非 open 候选自身）⟹ ancestor_close_by_id 判子声部祖先不齐 ⟹ AncOK 全剪 ⟹
    /// active 不含 L0 ShortDiff 子腿。**GREEN（修复后）**：open 候选父 parent_id registry-live ⟹ 恢复父
    /// carrier 祖先链入 raw ⟹ 子腿准入。
    ///
    /// 定义依据：级别容器.pdf §13（AncOK 持仓准入，父在 A_t 放行）+ anc.pdf §11（操作父 live ⟹ depth<d
    /// 祖先全在 raw）；§8 σ_r 父声部 carrier 跨 bar 持有（registry LiveDetached = 操作上仍持有）。
    #[test]
    fn open_candidate_parent_injected_from_registry_admits_depth_child() {
        let reg = super::super::super::persistent::PersistentRegistry::new();
        let tower = rc_tower(vec![
            Vec::new(),
            vec![nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up])],
        ]);
        // bar1：L1 容器卖点（src=12）⟹ 容器腿开 ⟹ merge 把容器 carrier 写入 registry（LiveDetached 源）。
        let bar1 = Classification {
            levels: vec![
                LevelState::default(),
                LevelState { bsp: Rc::new(vec![sell_bsp(12)]), ..Default::default() },
            ],
        };
        let (active1, _p1) =
            coverage_step_classification(&bar1, &tower, &[], 1000.0, &cfg(), None, &reg);
        let (elements1, _c1) =
            super::super::super::interp::coverage_elements_with_tower(&bar1, &tower);
        let reg2 = reg.merge(&elements1, &active1);

        // bar2：仅 L0 子卖点（src=8，**无** L1 BSP）+ **prev_active 为空**（父 carrier 非持仓腿）。
        // 父 carrier 仅在 reg2 中 LiveDetached 存活 ⟹ 唯有 open 候选父注入恢复祖先链才能准入子腿。
        let bar2 = Classification {
            levels: vec![LevelState { bsp: Rc::new(vec![sell_bsp(8)]), ..Default::default() }],
        };
        let (active2, _p2) =
            coverage_step_classification(&bar2, &tower, &[], 1000.0, &cfg(), None, &reg2);
        assert!(
            active2.iter().any(|l| l.level == 0 && l.dir == VoiceSide::Short),
            "bar2：L0 ShortDiff 子腿借 registry-live 父 carrier（非持仓腿）经 open 父注入准入 depth>0；实得 {active2:?}"
        );
        // ★(I-1) 双计守卫（codex 异质审查）：父 carrier 在 bar2 树前缀中也存在 ⟹ restore 须复用现有 idx，
        // 不得 push 重复 id ⟹ next_active 每 ElementId 唯一（否则 p̃ 双计）。
        let mut ids: Vec<_> = active2.iter().map(|l| l.id).collect();
        let n = ids.len();
        ids.sort_by_key(|id| (id.level, id.ordinal));
        ids.dedup();
        assert_eq!(ids.len(), n, "next_active 含重复 ElementId（restore 未复用现有 idx）⟹ p̃ 双计；实得 {active2:?}");
    }


    /// ★(I-1) **双计根因直测**（codex HIGH 行级坐实，644）：`restore_ancestor_chain_from_registry` 在
    /// 祖先**已存在于 work 树前缀**（如 carrier 走势）但未入 raw 时，必须**复用其现有 idx**，不得 push
    /// 重复 id 新元素。
    ///
    /// **RED（修复前 push 新元素）**：carrier id 已在 `work[0]`，restore 从 registry 取同 id 又 push 到
    /// `work[1]` ⟹ `work.len()==2`、`raw==[1]` 指向重复 id ⟹ 同一 carrier 在 next_idx 产两条 leg ⟹
    /// strategy_target_legs/net_target_units 双计 p̃、next_active 含两条同 id 腿（伪证仓位规模）。
    /// **GREEN（修复后复用现有 idx）**：restore 查得 `work[0].id==pid` ⟹ `raw==[0]`、`work.len()==1`
    /// 不增 ⟹ 每 id 在 raw 中唯一表示（spec §13 元素集语义）。
    ///
    /// 定义依据：codex 异质审查 HIGH（restore 只查 raw 不查 work 树前缀 ⟹ 重复 id）；spec §13 元素集
    /// （同一 ElementId 在 A_t 唯一）。
    #[test]
    fn restore_reuses_existing_work_idx_no_duplicate_id() {
        let carrier = eid(1, 0);
        // work 树前缀已含 carrier（如跨 bar 持仓的父声部走势对位回当前树）。
        let base = vec![CoverageElement {
            lambda: 0,
            rho: 12,
            eps: VoiceSide::Short,
            level: 1,
            parent: None,
            attached_dir: None,
            id: carrier,
            parent_id: None,
        }];
        let mut work = ElementView::new(&base);
        let mut raw: Vec<usize> = Vec::new();
        // registry 持有同一 carrier id（LiveDetached：snapshot_present 经下一 bar 增量重置为 false）。
        let reg = super::super::super::persistent::PersistentRegistry::new().merge(&base, &[]);

        let id_idx = build_tree_id_index(&base);
        let mut overlay_seen = std::collections::HashMap::new();
        let mut pending = Vec::new();
        restore_ancestor_chain_from_registry(&mut work, &mut raw, &reg, carrier, &id_idx, &mut overlay_seen, &mut pending);

        assert_eq!(work.len(), 1, "restore 不得 push 重复 id 元素（应复用 work[0]，overlay 空）");
        assert_eq!(raw, vec![0], "raw 须复用现有 idx 0，非追加新 idx");
        let dup = raw.iter().filter(|&&r| work[r].id == carrier).count();
        assert_eq!(dup, 1, "carrier 在 raw 中须唯一表示（双计根因守卫）");
    }


    /// ★票#350 核实起点（code-review 后订正措辞，原表述曾误判方向——见下方"局限"段）：
    /// `restore_ancestor_chain_from_registry` 本 bar 可能被多次调用（LiveDetached 持仓腿逐条 +
    /// open 候选逐条，见生产调用点 [`coverage_step_from_buckets_sep`]）。本测试坐实**跨调用共享
    /// 祖先的复用与去重**：两条腿（P1/P2）的恢复链共享同一祖父 GP，先调用（处理 P1）把 GP 物化
    /// （GP 是 P1 的直接父，同一次调用内紧接着解析，链内部无时序孔——父恰是同一 `while` 循环下一轮
    /// 要处理的 `cur`），后调用（处理 P2）复用（`already_in_raw` 提前收敛）该已在场的 GP、不重复
    /// push（双计根因守卫，同 #644）；两次调用各自贡献的 idx 一并进入调用方 `pending`，统一 fixup
    /// （票#350 修复后，见函数头文档）后 P1/P2 都正确接到同一个 GP idx 上。
    ///
    /// **局限（本测试不覆盖的方向）**：本构造中 GP 由**先调用**物化，不构成 issue 原文"父由**更晚**
    /// 一次 restore 调用物化"的反方向场景——该反方向的真实触发路径见下一测试
    /// `restore_chain_ancestor_unresolved_when_shared_ancestor_only_materializes_via_later_boundary_root_push`
    /// （#350 真正坐实的时序孔：父经**非 restore 的更晚直接 push** 物化，而非"更晚一次 restore 调用"
    /// 字面所指——两者同属 #247 缺口类，触发路径不同）。
    #[test]
    fn restore_chain_shared_ancestor_across_two_calls_resolves_and_dedupes() {
        let gp = eid(2, 0);
        let p1 = eid(1, 0);
        let p2 = eid(1, 1);
        let gp_elem = CoverageElement {
            lambda: 0, rho: 20, eps: VoiceSide::Long, level: 2,
            parent: None, attached_dir: None, id: gp, parent_id: None,
        };
        let p1_elem = CoverageElement {
            lambda: 0, rho: 12, eps: VoiceSide::Short, level: 1,
            parent: None, attached_dir: None, id: p1, parent_id: Some(gp),
        };
        let p2_elem = CoverageElement {
            lambda: 0, rho: 13, eps: VoiceSide::Short, level: 1,
            parent: None, attached_dir: None, id: p2, parent_id: Some(gp),
        };
        let reg = super::super::super::persistent::PersistentRegistry::new()
            .merge(&[gp_elem, p1_elem, p2_elem], &[]);

        let base: Vec<CoverageElement> = Vec::new();
        let mut work = ElementView::new(&base);
        let mut raw: Vec<usize> = Vec::new();
        let id_idx = build_tree_id_index(&base);
        let mut overlay_seen = std::collections::HashMap::new();
        let mut pending = Vec::new();

        // 先调：P1 链——GP 是 P1 的直接父，同一调用内紧接着物化（不依赖后续调用）。
        restore_ancestor_chain_from_registry(&mut work, &mut raw, &reg, p1, &id_idx, &mut overlay_seen, &mut pending);
        // 后调：P2 链——GP 已在场（already_in_raw 提前收敛），复用而非重复 push。
        restore_ancestor_chain_from_registry(&mut work, &mut raw, &reg, p2, &id_idx, &mut overlay_seen, &mut pending);
        // 票#350：两次调用各自的 idx 已汇入 pending，统一延后 fixup（模拟生产路径的调用方统一修补）。
        resolve_pending_parent_fixups(&mut work, &pending, &id_idx, &overlay_seen, &raw);

        assert_eq!(work.len(), 3, "GP 只应物化一次（P1 链物化，P2 链复用），work 应恰好 3 元素；实得 {}", work.len());
        let gp_count = (0..work.len()).filter(|&i| work[i].id == gp).count();
        assert_eq!(gp_count, 1, "GP 在 work 中须唯一（跨调用双计根因守卫）；实得 {gp_count}");

        let gp_idx = (0..work.len()).find(|&i| work[i].id == gp).expect("GP 须已物化");
        let p1_idx = (0..work.len()).find(|&i| work[i].id == p1).expect("P1 须已物化");
        let p2_idx = (0..work.len()).find(|&i| work[i].id == p2).expect("P2 须已物化");
        assert_eq!(work[p1_idx].parent, Some(gp_idx), "先调用链：P1.parent 须解析到 GP idx");
        assert_eq!(work[p1_idx].attached_dir, Some(VoiceSide::Long), "P1.attached_dir 须=GP.eps");
        assert_eq!(
            work[p2_idx].parent, Some(gp_idx),
            "后调用链：P2.parent 须解析到（先调用已物化的）同一 GP idx"
        );
        assert_eq!(work[p2_idx].attached_dir, Some(VoiceSide::Long), "P2.attached_dir 须=GP.eps");
    }


    /// ★票#350 对抗性核实（code-review Spec 轴浮出，#247/#315 同类第三位点，坐实并已修复）：上一
    /// 测试只坐实了"共享祖先由**先调用**物化、后调用复用"这一方向；本测试构造 issue 原文描述的
    /// **反方向**场景的真实可达触发路径——某恢复元素的祖先只在**同 bar 更晚**才通过**非 restore
    /// 的直接 push**物化（`Closed|Invalidated` + `is_boundary_root` 分支，held-leg 循环内、非经
    /// `overlay_seen`），而该祖先此前已被**更早一次** restore 调用尝试解析、因 registry 该祖先
    /// `invalidated` 而 `registry_lost` 断链——探测这两条独立判据（registry 断链 vs 该祖先自身 leg
    /// 状态）是否会不同步。
    ///
    /// 构造：D（腿，LiveDetached，op_parent=Some(Q)）在 prev_active[0] 先处理，触发 restore 从 Q 起链：
    /// Q 在 registry 中未失效 ⟹ 物化；Q 的 `structural_parent_id=Some(P)` ⟹ 链继续查 P——**此时** P
    /// 在 registry 中已被独立标记 `invalidated`（模拟结构性作废信号，与 P 自身是否仍是 held 腿无关，
    /// I3：parent 是关系非身份，作废是显式信号非隐含于父子关系）⟹ `registry.get(P)` 失败 ⟹ 断链，
    /// Q 的 idx 汇入调用方 `pending_parent_fixup`（票#350 修复后，不在本次 restore 调用内立即修补）。
    /// P 在 prev_active[1]（**更晚**）作为 held 腿处理：`held_state(P)` 独立查同一 `invalidated`
    /// 标记 ⟹ 也判 `Closed|Invalidated`；因 `is_boundary_root` ⟹ 保留为根，**直接 push 入 raw**
    /// （不经 restore、不经 overlay_seen）。
    ///
    /// **RED（修复前，本函数各次调用内立即 fixup）**：Q 的 fixup 在 prev_active[0] 处理时点（P 尚未
    /// push）立即执行 ⟹ 把 Q.parent 固化为 None，此后不会重跑——即便 P 随后（prev_active[1]）确实
    /// 入 raw、AncOK（`parent_id` 结构链，不读 `parent` idx 字段）判 Q 的祖先链全齐、Q 存活进
    /// next_idx，Q 的**角色计算**（`strategy_target_legs` 读 `parent`/`attached_dir` idx 字段）仍读到
    /// 早已固化的 None/None ⟹ V=Ambient/depth=0/q_units=600（#247 原始缺口的精确重现，触发路径不同：
    /// 非"更晚一次 restore 调用"而是"更晚一次非 restore 直接 push"）。
    /// **GREEN（修复后，票#350：本函数不再自行 fixup，idx 汇入 `pending_parent_fixup` 由调用方在
    /// 两个物化循环全部结束后统一修补）**：统一 fixup 时点 P 已在 raw ⟹ 三级解析（id_idx/overlay_seen/
    /// raw 扫）经 raw 扫命中 P ⟹ Q.parent=Some(P idx)、attached_dir=Some(P.eps) ⟹ V=ShortDiff、
    /// depth=1、q_units=300。
    ///
    /// 结果包边界条件：仅当共享祖先 P **同时**满足 (a) 是仍在 `prev_active` 的 held 腿、(b) `is_boundary_root`、
    /// (c) registry 状态被独立标记 invalidated、且 (d) 有另一条腿的祖先链经 P 时才触发；P 若非
    /// boundary_root（非根，非法或 closed 且非根 ⟹ 直接 prune 不入 raw）则不触发。
    #[test]
    fn restore_chain_ancestor_unresolved_when_shared_ancestor_only_materializes_via_later_boundary_root_push() {
        let p = eid(2, 0); // 共享祖先：仍是 held 腿（boundary_root）+ registry 侧已 invalidated。
        let q = eid(1, 0); // 中间祖先：仅 registry 存在（非 held 腿），链上 D→Q→P。
        let d = eid(0, 0); // 触发腿：LiveDetached，op_parent=Q。

        let p_cov = CoverageElement {
            lambda: 0, rho: 20, eps: VoiceSide::Long, level: 2,
            parent: None, attached_dir: None, id: p, parent_id: None,
        };
        let q_cov = CoverageElement {
            lambda: 0, rho: 12, eps: VoiceSide::Short, level: 1,
            parent: None, attached_dir: None, id: q, parent_id: Some(p),
        };
        // bar1：P/Q 均在 snapshot 中登记（真实存在过的走势元素）。
        let reg1 = super::super::super::persistent::PersistentRegistry::new().merge(&[p_cov, q_cov], &[]);

        let d_leg = ActiveLeg {
            level: 0, dir: VoiceSide::Short, source_index: 4, lambda: 0,
            id: d, parent_id: None, is_boundary_root: false, op_parent: Some(q),
        };
        // bar2：tree 空（P/Q 均不在新快照）⟹ 增量重置两者 snapshot_present=false；
        // held_legs=[d_leg] 令 D 自身登记为 LiveDetached 占位（op_parent=Q 的 registry 影子条目
        // 因 Q 已存在于 registry 而被 or_insert 跳过，不覆盖 Q 真实的 structural_parent_id=Some(P)）。
        let mut reg2 = reg1.merge(&[], &[d_leg]);
        // 独立结构性作废信号（与 P 是否仍是 held 腿无关，I3：parent 是关系非身份）。
        reg2.invalidate(&p);

        let p_leg = ActiveLeg {
            level: 2, dir: VoiceSide::Long, source_index: 20, lambda: 0,
            id: p, parent_id: None, is_boundary_root: true, op_parent: None,
        };
        let base: Vec<CoverageElement> = Vec::new();
        let buckets = Buckets { close: vec![], open: vec![], record: vec![] };

        // prev_active 顺序：D 先（触发对 Q 的 restore，链上溯到 P 时 registry_lost 断链）、
        // P 后（更晚一次处理，Closed|Invalidated + is_boundary_root ⟹ 直接 push 入 raw）。
        ancok_probe_reset();
        let (next_active, _p_tilde, sep_legs) = coverage_step_from_buckets_sep(
            ElementView::new(&base), &[d_leg, p_leg], &buckets, 1000.0, &cfg(), None, &reg2,
        );
        let probe = ancok_probe_snapshot();
        // ★Standards J2（code-review 复评浮出）：probe 交叉核对——Q 的链确实命中 registry_lost
        // （P 在 registry 侧 invalidated），但 Q 最终经统一 fixup 正确接线（非"unresolved 且未被
        // AncOK 剪除"的漏网），坐实 697 ceiling 真实判据（unresolved−pruned）在此场景下仍为 0，
        // 与 `restore_break_registry_lost>0` 不再蕴含"非严格"的订正一致（见 AncokProbe 文档）。
        assert!(probe.restore_break_registry_lost >= 1,
            "Q 的链须命中 registry_lost（P invalidated）；实得 {}", probe.restore_break_registry_lost);
        assert_eq!(probe.placeholder_parent_unresolved, probe.placeholder_pruned_by_ancok,
            "Q 虽经历 registry_lost 但最终统一 fixup 正确接线、AncOK 未剪除 ⟹ unresolved−pruned=0\
             （697 ceiling 真实判据无暴露）；unresolved={} pruned={}",
            probe.placeholder_parent_unresolved, probe.placeholder_pruned_by_ancok);

        // 承重坐实：Q（restore 恢复的中间祖先）经 AncOK（parent_id 结构判据）在 P 于本 bar 更晚
        // 入 raw 后仍应结构性存活——若这条都不满足，说明场景构造本身有误，非本票要坐实的缺口。
        assert!(
            next_active.iter().any(|l| l.id == q),
            "Q 的祖先链（parent_id: Q→P）在 raw_ids 中已全齐（P 更晚入 raw）⟹ AncOK 应判 Q 存活；实得 {next_active:?}"
        );
        let sep_q = sep_legs.iter().find(|s| s.id == q).expect("Q 须在 sep_legs（AncOK 存活）");
        // 核心断言（GREEN，#350 修复后）：Q 的角色计算须正确反映其真父 P——δ=−σ_p（Q Short vs P Long）
        // ⟹ V=ShortDiff、depth=1 ⟹ q_units=1000×0.30=300。修复前（restore 各次调用内立即 fixup）
        // 此处恒为 V=Ambient/q_units=600（P 更晚经非 restore 直接 push 物化，Q 的 fixup 已在更早一次
        // restore 调用内固化为 None/None，不会重跑）。
        assert_eq!(
            sep_q.role_v, Vertical::ShortDiff,
            "Q 存活进 next_idx 且 P 已在 raw（AncOK 判定祖先齐全）⟹ 角色计算须体现 V=ShortDiff；\
             若为 Ambient 则坐实 #350 时序孔（P 由更晚一次非 restore push 物化，Q 的 fixup 未跟上）"
        );
        assert!(
            (sep_q.q_units - 300.0).abs() < 1e-9,
            "depth=1 ⟹ q_units=300（时序孔存在时会是 depth=0 ⟹ 600）；实得 {}",
            sep_q.q_units
        );
    }


    /// ★票#247 缺口二（A 类实装缺口，mutex-domain-loadbearing-20260725 §二裁定）：完整祖先链恢复后，
    /// 恢复元素的 `parent`/`attached_dir` 必须重建为**真父 idx 与父容器方向**（σ_{p(g)}=父 eps，
    /// 与 `push_element_tree` 压子元素时传 `Some(父eps)` 同口径）——角色输入不再丢失。
    ///
    /// **RED（修复前）**：restore push 写死 `parent:None, attached_dir:None` ⟹ 恒定 `V=Ambient`
    /// （parent_sign(None)=0）+ `G=SameLevel`（ℓ_p 防御归 ℓ_g）+ `depth=0`（element_depth 沿 parent
    /// 链）⟹ 恢复父腿 units=600（depth0 全权重）。**GREEN（修复后）**：parent=Some(祖父 idx)、
    /// attached_dir=Some(祖父 eps) ⟹ V=ShortDiff（δ=−σ_p）+ G=SubLevel（ℓ_g<ℓ_p）+ depth=1
    /// ⟹ units=300（w_depth[1]=0.30），p̃ 由 0 变 +300。
    ///
    /// 定义依据：anc.pdf §11（恢复链 = 理想域持久祖先的物化）；spec §7.2（V 相对 σ_{p(g)}）；
    /// `push_element_tree`（子的 attached_dir=Some(父 eps)）。
    #[test]
    fn restore_rebuilds_parent_attached_dir_full_chain() {
        // 祖父（level 2 容器，Long，parent_id=None=∂）+ 父（level 1，Short，parent_id=祖父）。
        let grand = CoverageElement {
            lambda: 0, rho: 12, eps: VoiceSide::Long, level: 2,
            parent: None, attached_dir: None, id: eid(2, 0), parent_id: None,
        };
        let parent = CoverageElement {
            lambda: 0, rho: 8, eps: VoiceSide::Short, level: 1,
            parent: None, attached_dir: None, id: eid(1, 0), parent_id: Some(eid(2, 0)),
        };
        let reg = super::super::super::persistent::PersistentRegistry::new().merge(&[grand, parent], &[]);
        let base: Vec<CoverageElement> = Vec::new();
        let mut work = ElementView::new(&base);
        let mut raw: Vec<usize> = Vec::new();
        let id_idx = build_tree_id_index(&base);
        let mut overlay_seen = std::collections::HashMap::new();
        let mut pending = Vec::new();

        restore_ancestor_chain_from_registry(&mut work, &mut raw, &reg, eid(1, 0), &id_idx, &mut overlay_seen, &mut pending);
        resolve_pending_parent_fixups(&mut work, &pending, &id_idx, &overlay_seen, &raw);

        assert_eq!(work.len(), 2, "完整链恢复：父 + 祖父均 push 入 work");
        assert_eq!(raw, vec![0, 1], "恢复序 = 子先父后上溯");
        // 缺口二核心断言：恢复父的角色输入被重建（真父 idx + 父容器方向）。
        assert_eq!(work[0].parent, Some(1), "恢复父的 parent 须解析为祖父 idx（经 overlay_seen）");
        assert_eq!(work[0].attached_dir, Some(VoiceSide::Long),
            "恢复父的 attached_dir 须 = 祖父方向（σ_{{p(g)}}=父 eps，push_element_tree 同口径）");
        // 祖父 parent_id=None（真边界胚元 ∂）⟹ 保持 None/None（正确语义，非防御分支）。
        assert_eq!(work[1].parent, None, "祖父 parent_id=None（∂）保持 parent=None");
        assert_eq!(work[1].attached_dir, None, "祖父保持 attached_dir=None（σ_{{p(∂)}}=0）");

        // 角色 + 权重断言：恢复元素不再恒定 Ambient/SameLevel/depth0。
        let next_idx = ancestor_close_by_id(&work, &raw);
        assert_eq!(next_idx, vec![0, 1], "完整链 ⟹ AncOK 全保留");
        let legs = strategy_target_legs(&work, &next_idx, 1000.0, &cfg());
        let leg_parent = legs.iter().find(|l| l.e_idx == 0).expect("父腿须在");
        assert_eq!(leg_parent.role.v, Vertical::ShortDiff,
            "δ=−σ_p（Short vs Long）⟹ V=ShortDiff（修复前恒定 Ambient）");
        assert_eq!(leg_parent.role.grade, GradeRel::SubLevel,
            "ℓ_g=1<ℓ_p=2 ⟹ G=SubLevel（修复前恒定 SameLevel）");
        assert!((leg_parent.units - 300.0).abs() < 1e-9,
            "depth=1 ⟹ units=1000×0.30=300（修复前 depth=0 ⟹ 600）；实得 {}", leg_parent.units);
        let p = net_target_units(&legs);
        assert!((p - 300.0).abs() < 1e-9,
            "p̃=+600(Long 祖父)−300(Short 父)=+300（修复前 600−600=0）；实得 {p}");
    }


    /// ★票#247 缺口二·解析路径补全：恢复链上溯终止于 **base 段树前缀 carrier**（复用现有 idx
    /// 分支）时，本轮 push 元素的 `parent` 经 `id_idx` 解析到 base idx；base 段 carrier 自身
    /// （树前缀，`push_element_tree` 填字段处）**不被修补逻辑触碰**。
    ///
    /// **RED（修复前）**：work[1].parent==None（写死）⟹ 断言失败。
    #[test]
    fn restore_rebuilds_parent_attached_dir_to_base_carrier() {
        // base 段已含祖父 carrier（如跨 bar 持仓的父声部走势对位回当前树，parent_id=None）。
        let grand = CoverageElement {
            lambda: 0, rho: 12, eps: VoiceSide::Long, level: 2,
            parent: None, attached_dir: None, id: eid(2, 0), parent_id: None,
        };
        let parent = CoverageElement {
            lambda: 0, rho: 8, eps: VoiceSide::Short, level: 1,
            parent: None, attached_dir: None, id: eid(1, 0), parent_id: Some(eid(2, 0)),
        };
        let base = vec![grand];
        let reg = super::super::super::persistent::PersistentRegistry::new().merge(&[grand, parent], &[]);
        let mut work = ElementView::new(&base);
        let mut raw: Vec<usize> = Vec::new();
        let id_idx = build_tree_id_index(&base);
        let mut overlay_seen = std::collections::HashMap::new();
        let mut pending = Vec::new();

        restore_ancestor_chain_from_registry(&mut work, &mut raw, &reg, eid(1, 0), &id_idx, &mut overlay_seen, &mut pending);
        resolve_pending_parent_fixups(&mut work, &pending, &id_idx, &overlay_seen, &raw);

        assert_eq!(work.len(), 2, "祖父已在 base ⟹ 仅 push 父（overlay idx=1）");
        assert_eq!(raw, vec![1, 0], "父 push 入 raw 后上溯复用祖父现有 idx 0");
        // 缺口二核心断言：恢复父经 id_idx 解析到 base 祖父 idx。
        assert_eq!(work[1].parent, Some(0), "恢复父的 parent 须解析为 base 祖父 idx 0（经 id_idx）");
        assert_eq!(work[1].attached_dir, Some(VoiceSide::Long),
            "恢复父的 attached_dir = base 祖父 eps（σ_{{p(g)}}）");
        // 复用分支（base 树前缀 carrier）不被修补逻辑触碰——保持其既有字段。
        assert_eq!(work[0].parent, None, "base carrier 非本轮 push ⟹ 不被修补（保持原字段）");
        assert_eq!(work[0].attached_dir, None, "base carrier 保持原 attached_dir");

        // 角色断言：恢复父 V=ShortDiff / G=SubLevel / depth=1 / units=300。
        let next_idx = ancestor_close_by_id(&work, &raw);
        let legs = strategy_target_legs(&work, &next_idx, 1000.0, &cfg());
        let leg_parent = legs.iter().find(|l| l.e_idx == 1).expect("父腿须在");
        assert_eq!(leg_parent.role.v, Vertical::ShortDiff,
            "δ=−σ_p ⟹ V=ShortDiff（修复前 Ambient）");
        assert_eq!(leg_parent.role.grade, GradeRel::SubLevel,
            "ℓ_g<ℓ_p ⟹ G=SubLevel（修复前 SameLevel）");
        assert!((leg_parent.units - 300.0).abs() < 1e-9,
            "depth=1 ⟹ units=300（修复前 depth=0 ⟹ 600）；实得 {}", leg_parent.units);
    }


    /// ★票#247 缺口二·断链语义坐实（裁定 (a)）：registry 丢失父（restore_break_registry_lost）时，
    /// 本轮已 push 的恢复元素其 `parent_id` 不在 raw ⟹ `ancestor_close_by_id`（AncOK：Anc(e)⊆raw
    /// 才保留）将其全部剪除 ⟹ **不进 next_idx，到不了角色计算**，且不 panic。
    ///
    /// 断链元素**不伪造** parent/attached_dir（保持 None/None）——registry 已丢失父，父
    /// 在本轮（及此后任何轮）都不会物化进 raw ⟹ 被 AncOK 剪除（090 订正措辞，#284 MED-1：
    /// 「恒」须限定在「父确实从未物化进 raw」这一条件下，非无条件全称——本例满足该条件），
    /// 角色/p̃ 无影响；这不是「防御分支兜底」，是 AncOK 结构判据的正规剪枝。
    #[test]
    fn restore_broken_chain_pruned_by_ancok_no_panic() {
        // 子在 registry（parent_id=父），父不在 registry（已丢失/作废）。
        let child = CoverageElement {
            lambda: 0, rho: 4, eps: VoiceSide::Short, level: 0,
            parent: None, attached_dir: None, id: eid(0, 0), parent_id: Some(eid(1, 0)),
        };
        let reg = super::super::super::persistent::PersistentRegistry::new().merge(&[child], &[]);
        let base: Vec<CoverageElement> = Vec::new();
        let mut work = ElementView::new(&base);
        let mut raw: Vec<usize> = Vec::new();
        let id_idx = build_tree_id_index(&base);
        let mut overlay_seen = std::collections::HashMap::new();
        let mut pending = Vec::new();

        restore_ancestor_chain_from_registry(&mut work, &mut raw, &reg, eid(0, 0), &id_idx, &mut overlay_seen, &mut pending);
        resolve_pending_parent_fixups(&mut work, &pending, &id_idx, &overlay_seen, &raw);

        assert_eq!(work.len(), 1, "子已 push；父 registry 丢失 ⟹ 断链停止");
        assert_eq!(raw, vec![0], "断链子元素留在 raw（剪除归 AncOK，不在 restore 内）");
        // 断链元素不伪造 parent/attached_dir（父无法解析 ⟹ 保持 None/None）。
        assert_eq!(work[0].parent, None, "断链 ⟹ 不伪造 parent");
        assert_eq!(work[0].attached_dir, None, "断链 ⟹ 不伪造 attached_dir");
        // 裁定 (a)：断链 ⟹ 恢复元素不进 next_idx（AncOK 剪除），不 panic。
        let next_idx = ancestor_close_by_id(&work, &raw);
        assert!(next_idx.is_empty(),
            "子的 parent_id=父不在 raw ⟹ AncOK 剪除 ⟹ 不进 next_idx；实得 {next_idx:?}");
        let legs = strategy_target_legs(&work, &next_idx, 1000.0, &cfg());
        assert!(legs.is_empty(), "next_idx 空 ⟹ 无腿（断链元素到不了角色计算）");
    }


