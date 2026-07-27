use super::*;
use super::super::test_support::*;
use crate::theta_v0::types::Direction;
use crate::theta_v0::classifier::LevelState;

    /// ★票#247 缺口二·∂ 语义保持（裁定 3）：`parent_id=None` 的恢复元素（真边界胚元 ∂）保持
    /// `parent=None, attached_dir=None`——σ_{p(∂)}=0 ⟹ V=Ambient 是去根化正解，非防御分支。
    #[test]
    fn restore_boundary_germ_keeps_parent_none() {
        let root = CoverageElement {
            lambda: 0, rho: 12, eps: VoiceSide::Long, level: 2,
            parent: None, attached_dir: None, id: eid(2, 0), parent_id: None,
        };
        let reg = super::super::super::persistent::PersistentRegistry::new().merge(&[root], &[]);
        let base: Vec<CoverageElement> = Vec::new();
        let mut work = ElementView::new(&base);
        let mut raw: Vec<usize> = Vec::new();
        let id_idx = build_tree_id_index(&base);
        let mut overlay_seen = std::collections::HashMap::new();
        let mut pending = Vec::new();

        restore_ancestor_chain_from_registry(&mut work, &mut raw, &reg, eid(2, 0), &id_idx, &mut overlay_seen, &mut pending);
        resolve_pending_parent_fixups(&mut work, &pending, &id_idx, &overlay_seen, &raw);

        assert_eq!(work.len(), 1, "单元素（∂）恢复");
        assert_eq!(work[0].parent, None, "parent_id=None（∂）保持 parent=None");
        assert_eq!(work[0].attached_dir, None, "parent_id=None（∂）保持 attached_dir=None");
        let next_idx = ancestor_close_by_id(&work, &raw);
        assert_eq!(next_idx, vec![0], "∂ 无祖先要求 ⟹ AncOK 恒等准入");
        let legs = strategy_target_legs(&work, &next_idx, 1000.0, &cfg());
        assert_eq!(legs[0].role.v, Vertical::Ambient,
            "σ_{{p(∂)}}=0 ⟹ V=Ambient（去根化正解）");
        assert!((legs[0].units - 600.0).abs() < 1e-9,
            "depth=0 ⟹ units=600；实得 {}", legs[0].units);
    }


    /// ★票#267（#247 同类位点）：held 腿 **LiveDetached 占位元素**角色输入重建。
    ///
    /// LiveDetached 分支 restore 祖先链后 push 的占位元素携 `parent_id=leg.op_parent`（已知真父），
    /// 修复前写死 `parent:None, attached_dir:None`。AncOK 按 **parent_id 结构映射**判（不看 parent
    /// 索引）⟹ 链完整时占位元素**存活进 next_idx**（不被剪）；`strategy_target_legs` 对其算角色：
    /// `element_depth` 沿 parent 索引链（None⟹depth=0）、V 由 attached_dir（None⟹σ_p=0⟹Ambient）、
    /// G 由 parent（None⟹SameLevel）⟹ **角色输入丢失进 depth_weight/dir_weight/w_grade/units/p̃**——
    /// 承重缺口，与 #247 缺口二同类。
    ///
    /// **RED（修复前）**：占位腿 V=Ambient + depth=0 ⟹ q_units=600，p̃=+300。
    /// **GREEN（修复后）**：parent=Some(恢复父 idx)、attached_dir=Some(父 eps) ⟹ V=ShortDiff
    /// （δ=−σ_p）+ G=SubLevel + depth=2 ⟹ q_units=100，p̃=+800。
    ///
    /// 定义依据：anc.pdf §10/§15（LiveDetached 的 parent 仍是 op_parent(L)）；spec §7.2（V 相对
    /// σ_{p(g)}）；`push_element_tree`（子的 attached_dir=Some(父 eps)）；票#247 缺口二同口径。
    #[test]
    fn held_leg_live_detached_placeholder_rebuilds_parent_attached_dir() {
        // 祖父（level 2 容器，Long，parent_id=None=∂）+ 父（level 1，Long，parent_id=祖父）。
        let grand = CoverageElement {
            lambda: 0, rho: 12, eps: VoiceSide::Long, level: 2,
            parent: None, attached_dir: None, id: eid(2, 0), parent_id: None,
        };
        let parent = CoverageElement {
            lambda: 0, rho: 8, eps: VoiceSide::Long, level: 1,
            parent: None, attached_dir: None, id: eid(1, 0), parent_id: Some(eid(2, 0)),
        };
        // held 子腿（level 0，Short）：id 不在当前树（Stale）+ registry LiveDetached
        // （or_insert snapshot_present=false）+ op_parent=父（已知真父，可解析）。
        let child_leg = ActiveLeg {
            level: 0, dir: VoiceSide::Short, source_index: 4, lambda: 0,
            id: eid(0, 0), parent_id: Some(eid(1, 0)), is_boundary_root: false, op_parent: Some(eid(1, 0)),
        };
        let reg = super::super::super::persistent::PersistentRegistry::new().merge(&[parent, grand], &[child_leg]);
        let base: Vec<CoverageElement> = Vec::new();
        let buckets = Buckets { close: vec![], open: vec![], record: vec![] };

        let (next_active, p_tilde, sep_legs) = coverage_step_from_buckets_sep(
            ElementView::new(&base), &[child_leg], &buckets, 1000.0, &cfg(), None, &reg,
        );

        // 承重坐实①：占位元素不被 AncOK 剪除（restore 完整 ⟹ parent_id 祖先全在 raw）⟹ 进 next_idx。
        assert!(
            next_active.iter().any(|l| l.id == eid(0, 0)),
            "LiveDetached 占位元素 restore 完整 ⟹ 存活进 next_idx（承重前提）；实得 {next_active:?}"
        );
        // 承重坐实②：占位腿角色被 strategy_target_legs 消费（sep_legs 只读暴露 role_v/q_units）。
        let sep_child = sep_legs.iter().find(|s| s.id == eid(0, 0))
            .expect("占位腿须在 sep_legs（legs 重打包）");
        assert_eq!(sep_child.role_v, Vertical::ShortDiff,
            "δ=−σ_p（Short vs 父 Long）⟹ V=ShortDiff（修复前写死 None ⟹ 恒定 Ambient）");
        assert!((sep_child.q_units - 100.0).abs() < 1e-9,
            "depth=2（子→父→祖父）⟹ q=1000×0.10=100（修复前 depth=0 ⟹ 600）；实得 {}", sep_child.q_units);
        // restore 链两条腿不受本修复影响（#247 已重建，bit-exact 锚）。
        let sep_parent = sep_legs.iter().find(|s| s.id == eid(1, 0)).expect("父腿须在");
        assert_eq!(sep_parent.role_v, Vertical::FollowParent, "父 Long vs 祖父 Long ⟹ FollowParent");
        assert!((sep_parent.q_units - 300.0).abs() < 1e-9, "父 depth=1 ⟹ 300；实得 {}", sep_parent.q_units);
        // p̃：GREEN = +300(父) + 600(祖父) − 100(子 ShortDiff) = +800；RED = 300+600−600 = +300。
        assert!((p_tilde - 800.0).abs() < 1e-9,
            "p̃=+800（修复前占位腿 depth0/Ambient 600 ⟹ p̃=+300）；实得 {p_tilde}");
    }


    /// ★票#267：held 腿 **LivePresent 占位元素**角色输入重建（同 LiveDetached 同口径）。
    ///
    /// LivePresent 分支（理论不可达防御臂：Exact 未命中但 registry snapshot_present=true = snapshot
    /// 不一致）**不做 restore**，占位元素存活条件是 op_parent 经其他路径在 raw（本测试：父作另一
    /// 持仓腿 Exact 对位入 raw）。占位元素同样携 `parent_id=leg.op_parent` 却写死 None/None ⟹
    /// 同类角色输入丢失。
    ///
    /// **RED（修复前）**：占位腿 V=Ambient + depth=0 ⟹ q_units=600，p̃=0。
    /// **GREEN（修复后）**：parent=Some(base 父 idx)、attached_dir=Some(父 eps) ⟹ V=ShortDiff +
    /// depth=1 ⟹ q_units=300，p̃=+300。
    #[test]
    fn held_leg_live_present_placeholder_rebuilds_parent_attached_dir() {
        // base 树含父 carrier（level 1，Long，∂ 根）。
        let parent = CoverageElement {
            lambda: 0, rho: 8, eps: VoiceSide::Long, level: 1,
            parent: None, attached_dir: None, id: eid(1, 0), parent_id: None,
        };
        let base = vec![parent];
        // 父作持仓腿 Exact 对位入 raw（占位的 op_parent 因此在 raw，占位存活 AncOK）。
        let parent_leg = ActiveLeg {
            level: 1, dir: VoiceSide::Long, source_index: 8, lambda: 0,
            id: eid(1, 0), parent_id: None, is_boundary_root: true, op_parent: None,
        };
        // 子腿：id 不在当前树（Stale）+ registry snapshot_present=true（LivePresent）。
        let child_leg = ActiveLeg {
            level: 0, dir: VoiceSide::Short, source_index: 4, lambda: 0,
            id: eid(99, 99), parent_id: Some(eid(1, 0)), is_boundary_root: false, op_parent: Some(eid(1, 0)),
        };
        let child_cov = CoverageElement {
            lambda: 0, rho: 4, eps: VoiceSide::Short, level: 0,
            parent: None, attached_dir: None, id: eid(99, 99), parent_id: Some(eid(1, 0)),
        };
        let reg = super::super::super::persistent::PersistentRegistry::new().merge(&[child_cov], &[]);
        let buckets = Buckets { close: vec![], open: vec![], record: vec![] };

        let (next_active, p_tilde, sep_legs) = coverage_step_from_buckets_sep(
            ElementView::new(&base), &[parent_leg, child_leg], &buckets, 1000.0, &cfg(), None, &reg,
        );

        // 承重坐实①：LivePresent 占位元素 op_parent 在 raw ⟹ 存活进 next_idx。
        assert!(
            next_active.iter().any(|l| l.id == eid(99, 99)),
            "LivePresent 占位元素 op_parent 在 raw ⟹ 存活进 next_idx（承重前提）；实得 {next_active:?}"
        );
        // 承重坐实②：角色被消费。
        let sep_child = sep_legs.iter().find(|s| s.id == eid(99, 99))
            .expect("占位腿须在 sep_legs");
        assert_eq!(sep_child.role_v, Vertical::ShortDiff,
            "δ=−σ_p（Short vs 父 Long）⟹ V=ShortDiff（修复前恒定 Ambient）");
        assert!((sep_child.q_units - 300.0).abs() < 1e-9,
            "depth=1 ⟹ q=300（修复前 depth=0 ⟹ 600）；实得 {}", sep_child.q_units);
        // p̃：GREEN = +600(父根) − 300(子) = +300；RED = 600−600 = 0。
        assert!((p_tilde - 300.0).abs() < 1e-9,
            "p̃=+300（修复前 0）；实得 {p_tilde}");
    }


    /// ★票#267·断链语义坐实（同 #247 裁定 (a) 口径）：LiveDetached 占位元素的 op_parent **无法解析**
    /// （restore 断链：祖父不在 registry）时，**不伪造** parent/attached_dir——占位元素的 parent_id
    /// 链顶端不在 raw ⟹ `ancestor_close_by_id`（AncOK：Anc(e)⊆raw 才保留）恒剪除 ⟹ 不进 next_idx，
    /// 到不了角色计算。这不是「防御分支兜底」，是 AncOK 结构判据的正规剪枝（修复前后行为一致）。
    #[test]
    fn held_leg_placeholder_broken_chain_pruned_by_ancok() {
        // 父在 registry 但其父（祖父 eid(2,0)）不在 ⟹ restore 上溯断链（restore_break_registry_lost）。
        let parent = CoverageElement {
            lambda: 0, rho: 8, eps: VoiceSide::Long, level: 1,
            parent: None, attached_dir: None, id: eid(1, 0), parent_id: Some(eid(2, 0)),
        };
        let child_leg = ActiveLeg {
            level: 0, dir: VoiceSide::Short, source_index: 4, lambda: 0,
            id: eid(0, 0), parent_id: Some(eid(1, 0)), is_boundary_root: false, op_parent: Some(eid(1, 0)),
        };
        let reg = super::super::super::persistent::PersistentRegistry::new().merge(&[parent], &[child_leg]);
        let base: Vec<CoverageElement> = Vec::new();
        let buckets = Buckets { close: vec![], open: vec![], record: vec![] };

        let (next_active, p_tilde, sep_legs) = coverage_step_from_buckets_sep(
            ElementView::new(&base), &[child_leg], &buckets, 1000.0, &cfg(), None, &reg,
        );

        assert!(
            !next_active.iter().any(|l| l.id == eid(0, 0)),
            "断链 ⟹ 占位元素 parent_id 链顶端（祖父）不在 raw ⟹ AncOK 剪除；实得 {next_active:?}"
        );
        assert!(sep_legs.is_empty(), "断链 ⟹ 全链剪除 ⟹ 无腿（不伪造角色输入，到不了角色计算）");
        assert_eq!(p_tilde, 0.0, "断链 ⟹ p̃=0");
        // 本用例注：op_parent=eid(1,0) 本身已被 registry 收录（`parent` 入参），restore 能把它
        // 物化进 raw ⟹ child 的占位在统一 fixup 能正常解析（非 unresolved）——真正断链在更深一层
        // （eid(1,0) 自己的 parent_id=eid(2,0) 不在 registry），由 `ancestor_close_by_id` 沿链直接
        // 剪除，不经过 `rebuild_placeholder_parent_attached` 的 unresolved 分支。`unresolved→pruned`
        // 交叉核对的坐实用例见 `placeholder_unresolved_and_pruned_cross_check_probe`
        // （op_parent 在 registry 中根本不存在的更浅层断链）。
    }


    /// ★#347 MED-1：`placeholder_parent_unresolved`/`placeholder_pruned_by_ancok` 交叉核对坐实。
    ///
    /// 走 `HeldLegState::LivePresent` 分支（非 `LiveDetached`）——LiveDetached 在 push 占位**之前**
    /// 先调 `restore_ancestor_chain_from_registry` 把 op_parent 祖先链物化进 raw，此后统一 fixup
    /// 几乎总能解析到（`held_leg_placeholder_broken_chain_pruned_by_ancok` 用例证实：即便祖父断链，
    /// op_parent 自身仍先被 restore 物化，占位不落 unresolved）。LivePresent 分支**没有**这一步
    /// restore（"理论不可达"注释所在分支，见 `coverage_step_from_buckets_sep` 上方代码）——若其
    /// op_parent 从未出现在 base/overlay/raw 任何一处，统一 fixup 三级解析必空 ⟹ 落
    /// `placeholder_parent_unresolved`；随后 `ancestor_close_by_id` 因 op_parent 不在 raw_ids
    /// 剪除该占位 ⟹ 落 `placeholder_pruned_by_ancok`。两计数在此同为 1，验证函数头文档声称的
    /// "可交叉核对"确实可执行（而非只声明不可验证，#347 MED-1）。
    #[test]
    fn placeholder_unresolved_and_pruned_cross_check_probe() {
        let phantom_parent = eid(9, 9); // 从未出现在 registry/base/overlay/raw 任何一处。
        let child_id = eid(0, 0);
        let child_leg = ActiveLeg {
            level: 0, dir: VoiceSide::Short, source_index: 4, lambda: 0,
            id: child_id, parent_id: Some(phantom_parent), is_boundary_root: false,
            op_parent: Some(phantom_parent),
        };
        // registry：child_id 本身 snapshot_present=true（LivePresent 判据，见 held_state）——
        // 用一个**不参与本次 coverage_step 调用**的 CoverageElement 快照建立这个持久状态
        // （"Exact 未命中但 registry LivePresent" 本就是代码注释标注的理论边界场景，通过直接
        // 摆放 registry 状态构造，不经真实的两 bar 演化）。刻意不把 child_leg 传进 `merge` 的
        // held_legs 参数——避免触发 I4 规则自动把 phantom_parent 也 upsert 进 registry。
        let child_snapshot = CoverageElement {
            lambda: 0, rho: 4, eps: VoiceSide::Short, level: 0,
            parent: None, attached_dir: None, id: child_id, parent_id: Some(phantom_parent),
        };
        let reg = super::super::super::persistent::PersistentRegistry::new().merge(&[child_snapshot], &[]);
        assert!(!reg.registry_live(&phantom_parent), "前提：phantom_parent 确实不在 registry 中");
        assert_eq!(
            reg.held_state(&child_leg),
            super::super::super::persistent::HeldLegState::LivePresent,
            "前提：child_leg 在 registry 中为 LivePresent（走无 restore 的分支）"
        );
        let base: Vec<CoverageElement> = Vec::new(); // 本 bar work 树为空 ⟹ Exact 对位必未命中（Stale）。
        let buckets = Buckets { close: vec![], open: vec![], record: vec![] };

        ancok_probe_reset();
        let (next_active, _p_tilde, sep_legs) = coverage_step_from_buckets_sep(
            ElementView::new(&base), &[child_leg], &buckets, 1000.0, &cfg(), None, &reg,
        );
        let probe = ancok_probe_snapshot();

        assert!(
            !next_active.iter().any(|l| l.id == child_id),
            "phantom_parent 不可解析 ⟹ AncOK 剪除；实得 {next_active:?}"
        );
        assert!(sep_legs.is_empty(), "剪除 ⟹ 无腿");
        assert_eq!(probe.state_live_present, 1, "前提复核：确实走了 LivePresent 分支");
        assert_eq!(probe.placeholder_parent_unresolved, 1, "唯一占位元素 op_parent 三级解析全空");
        assert_eq!(probe.placeholder_pruned_by_ancok, 1, "该未解析占位随后被 AncOK 剪除——交叉核对成立");
    }


    /// ★#347 LOW-1：环形 `parent_id`（`op_parent == leg.id` 自指）不得使 coverage 步骤挂起。
    ///
    /// 排查过程中实测发现挂起点**不是**先前文档假设的 `element_depth`，而是更早一步的
    /// `ancestor_close_by_id`→`ancestors_by_id_lookup`（用 bisect eprintln 定位：`rebuild_
    /// placeholder_parent_attached` 后打印可达，`ancestor_close_by_id` 后打印不可达）：
    /// `ancestors_by_id_lookup` 的 `lookup` 闭包从 `elements.overlay`（此刻已含刚 push 的占位
    /// 元素自己）现建 `overlay_id_idx`，`self_id→自己的 idx` 天然在表中——`.parent_id` 结构链
    /// 沿 `self_id→lookup(self_id)=自己→parent_id 仍是 self_id→...` 永不终止。该函数旧头部注释
    /// "环不可能"的前提只对 `push_element_tree` 真树成立，held-leg 占位的 `parent_id` 来自外部
    /// `ActiveLeg::op_parent`，不受这个约束——两处都须修（`ancestors_by_id_lookup` 加环检测 +
    /// 本函数 `r==idx` 守卫防 `.parent` 索引自环），任一处漏修本测试都会挂起。
    ///
    /// 本用例走 LivePresent 分支（无 restore 前置物化，raw 兜底/overlay_id_idx 才是唯一入口）；
    /// push 占位后 raw 里只有它自己，`id == parent_id`（自指）。
    #[test]
    fn held_leg_placeholder_self_parent_id_does_not_hang() {
        let self_id = eid(7, 7);
        let self_loop_leg = ActiveLeg {
            level: 0, dir: VoiceSide::Short, source_index: 4, lambda: 0,
            id: self_id, parent_id: Some(self_id), is_boundary_root: false,
            op_parent: Some(self_id), // ★环形：op_parent 指向自己。
        };
        let self_snapshot = CoverageElement {
            lambda: 0, rho: 4, eps: VoiceSide::Short, level: 0,
            parent: None, attached_dir: None, id: self_id, parent_id: Some(self_id),
        };
        let reg = super::super::super::persistent::PersistentRegistry::new().merge(&[self_snapshot], &[]);
        assert_eq!(
            reg.held_state(&self_loop_leg),
            super::super::super::persistent::HeldLegState::LivePresent,
            "前提：走 LivePresent 分支（无 restore 前置物化，raw 兜底才是唯一防线）"
        );
        let base: Vec<CoverageElement> = Vec::new();
        let buckets = Buckets { close: vec![], open: vec![], record: vec![] };

        // 有界超时坐实"不挂起"（非 panic 场景 debug_assert 无法直接断言死循环，用超时替代）：
        // 正常（守卫在场）应在毫秒级返回；若守卫被移除会在此挂起，`join_timeout` 之后仍未收到
        // 结果即判失败——比"跑很久才发现 CI 卡死"更早、更明确地暴露自环回归。
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = coverage_step_from_buckets_sep(
                ElementView::new(&base), &[self_loop_leg], &buckets, 1000.0, &cfg(), None, &reg,
            );
            let _ = tx.send(result);
        });
        let (next_active, _p_tilde, sep_legs) = rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("自环 parent_id 使 element_depth 挂起——r==idx 守卫缺失或失效（#347 LOW-1 回归）");

        // 不伪造语义：自指未被当作真父解析，`parent` 保持 None（诚实标注不可解析）。
        assert!(
            next_active.iter().any(|l| l.id == self_id),
            "结构 parent_id 链自指⊆raw 平凡满足 ⟹ AncOK 不剪除（唯一防线是 r==idx 守卫，非 AncOK）；实得 {next_active:?}"
        );
        let leg = sep_legs.iter().find(|s| s.id == self_id).expect("自环元素须在 sep_legs（AncOK 未剪）");
        assert_eq!(leg.parent_id, Some(self_id), "structural parent_id 字段本身保持自指（不改写输入）");
    }


    /// ★票#267·∂ 语义保持：`op_parent=None` 的 LiveDetached held 腿（真边界胚元 ∂ 根声部）保持
    /// `parent=None, attached_dir=None`——σ_{p(∂)}=0 ⟹ V=Ambient 是去根化正解，非防御分支
    /// （修复只重建已知 parent_id，parent_id=None 无可重建；修复前后行为一致）。
    #[test]
    fn held_leg_placeholder_boundary_germ_keeps_parent_none() {
        let root_leg = ActiveLeg {
            level: 1, dir: VoiceSide::Long, source_index: 8, lambda: 0,
            id: eid(1, 0), parent_id: None, is_boundary_root: true, op_parent: None,
        };
        let reg = super::super::super::persistent::PersistentRegistry::new().merge(&[], &[root_leg]);
        let base: Vec<CoverageElement> = Vec::new();
        let buckets = Buckets { close: vec![], open: vec![], record: vec![] };

        let (next_active, p_tilde, sep_legs) = coverage_step_from_buckets_sep(
            ElementView::new(&base), &[root_leg], &buckets, 1000.0, &cfg(), None, &reg,
        );

        assert!(
            next_active.iter().any(|l| l.id == eid(1, 0)),
            "∂ 根 LiveDetached 占位（parent_id=None）无祖先要求 ⟹ AncOK 恒等准入；实得 {next_active:?}"
        );
        let sep_root = sep_legs.iter().find(|s| s.id == eid(1, 0)).expect("∂ 根腿须在");
        assert_eq!(sep_root.role_v, Vertical::Ambient, "σ_{{p(∂)}}=0 ⟹ V=Ambient（去根化正解）");
        assert_eq!(sep_root.parent_id, None, "∂ 根保持 parent_id=None");
        assert!((sep_root.q_units - 600.0).abs() < 1e-9,
            "depth=0 ⟹ q=600（∂ 语义修复前后不变）；实得 {}", sep_root.q_units);
        assert!((p_tilde - 600.0).abs() < 1e-9, "p̃=+600；实得 {p_tilde}");
    }


    /// ★票#315（#284 评审 MED-1 坐实的时序孔，父晚物化场景）：`prev_active` 中**子在父之前**、
    /// 且父与子都是 Stale/LivePresent 占位（都在本轮 `prev_active` 循环内才被 push 进 raw，都不在
    /// base 树/overlay_seen）——立即式修补（占位 push 当轮即调 helper）在 push 子时父尚未 push，
    /// 三级解析（id_idx/overlay_seen/raw）全查不到，误留 `parent:None, attached_dir:None`；循环后
    /// 统一 fixup（本票修复）在两个物化循环结束、AncOK 判定前执行，此时父已在 raw，可正确解析。
    ///
    /// 子占位存活性不受影响（AncOK 用 `parent_id`——ElementId 结构映射，不读 `parent` 索引字段，
    /// 与 helper 的时序无关，见 [`ancestor_close_by_id`]）；受影响的是**角色输入**（V/depth/units），
    /// 这正是本票要堵的缺角。
    ///
    /// **RED（立即式，修复前）**：子占位 V=Ambient + depth=0 ⟹ q_units=600，p̃=0（同修复前恒定
    /// Ambient 的既有 held_leg 测试同构：600 父根 − 600 子 = 0）。
    /// **GREEN（循环后统一 fixup，修复后）**：子占位 parent=Some(父 idx)、attached_dir=Some(父 eps)
    /// ⟹ V=ShortDiff（δ=−σ_p）+ depth=1 ⟹ q_units=300，p̃=+300（600 父 − 300 子）。
    #[test]
    fn held_leg_placeholder_parent_materializes_in_later_iteration() {
        // 父：LivePresent 占位，∂ 根（parent_id=None，level 1，Long）——本身角色计算 trivial，
        // 但在 prev_active 中排在子**之后**，本轮循环内是**晚于子**才被 push 进 raw 的元素。
        let father_cov = CoverageElement {
            lambda: 0, rho: 8, eps: VoiceSide::Long, level: 1,
            parent: None, attached_dir: None, id: eid(1, 0), parent_id: None,
        };
        let father_leg = ActiveLeg {
            level: 1, dir: VoiceSide::Long, source_index: 8, lambda: 0,
            id: eid(1, 0), parent_id: None, is_boundary_root: true, op_parent: None,
        };
        // 子：LivePresent 占位，parent_id=父（eid(1,0)），在 prev_active 中排在父**之前**
        // （时序孔可达性：open 候选先入 raw、父链随后追加，令 next_active 中子先于父——见
        // shadow-review-267-20260726.md §4 排序可达性推导；本测试直接构造 prev_active 顺序复现
        // 同一时序孔，无需经过完整的 bar-to-bar 传播）。
        let child_cov = CoverageElement {
            lambda: 0, rho: 4, eps: VoiceSide::Short, level: 0,
            parent: None, attached_dir: None, id: eid(0, 0), parent_id: Some(eid(1, 0)),
        };
        let child_leg = ActiveLeg {
            level: 0, dir: VoiceSide::Short, source_index: 4, lambda: 0,
            id: eid(0, 0), parent_id: Some(eid(1, 0)), is_boundary_root: false, op_parent: Some(eid(1, 0)),
        };
        let reg = super::super::super::persistent::PersistentRegistry::new()
            .merge(&[child_cov, father_cov], &[]);
        let base: Vec<CoverageElement> = Vec::new();
        let buckets = Buckets { close: vec![], open: vec![], record: vec![] };

        // prev_active 顺序：子在前、父在后（timing hole 复现的必要条件）。
        let (next_active, p_tilde, sep_legs) = coverage_step_from_buckets_sep(
            ElementView::new(&base), &[child_leg, father_leg], &buckets, 1000.0, &cfg(), None, &reg,
        );

        // 承重坐实：子占位不受 helper 时序影响地存活（AncOK 判据是 parent_id，非 parent 索引）。
        assert!(
            next_active.iter().any(|l| l.id == eid(0, 0)),
            "子占位 parent_id 链（父 eid(1,0)）经本轮统一 fixup 后已在 raw ⟹ AncOK 存活；实得 {next_active:?}"
        );
        let sep_child = sep_legs.iter().find(|s| s.id == eid(0, 0)).expect("子腿须在 sep_legs");
        assert_eq!(sep_child.role_v, Vertical::ShortDiff,
            "父晚物化（同 bar 更晚迭代）：循环后统一 fixup 应解析到父 ⟹ V=ShortDiff（立即式修补会误留 Ambient）");
        assert!((sep_child.q_units - 300.0).abs() < 1e-9,
            "depth=1 ⟹ q=300（立即式修补残留 depth=0 ⟹ 600）；实得 {}", sep_child.q_units);
        let sep_father = sep_legs.iter().find(|s| s.id == eid(1, 0)).expect("父腿须在");
        assert_eq!(sep_father.role_v, Vertical::Ambient, "父自身 ∂ 根，V=Ambient 不受本修复影响");
        assert!((p_tilde - 300.0).abs() < 1e-9,
            "p̃=+300（600 父 − 300 子）；立即式修补下应为 0（600−600）；实得 {p_tilde}");
    }


    /// ★(I-1) open 父注入非膨胀守卫：父 carrier **不在 registry**（既非持仓又非 registry-live）⟹ 子腿
    /// 仍被剪枝。open 父注入只在父真实 live 时恢复祖先链，不无条件放行（no-patch：非 AncOK 加特例）。
    #[test]
    fn open_candidate_parent_not_in_registry_still_pruned() {
        let reg = super::super::super::persistent::PersistentRegistry::new();
        let tower = rc_tower(vec![
            Vec::new(),
            vec![nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up])],
        ]);
        // 空 registry（父 carrier 从未出现在任何 snapshot）+ 仅 L0 ShortDiff 子卖点 + 空 prev_active。
        let bar = Classification {
            levels: vec![LevelState { bsp: Rc::new(vec![sell_bsp(8)]), ..Default::default() }],
        };
        let (active, p) =
            coverage_step_classification(&bar, &tower, &[], 1000.0, &cfg(), None, &reg);
        assert!(
            active.is_empty(),
            "父 carrier 不在 registry ⟹ open 父注入不恢复 ⟹ 子腿仍剪枝（非膨胀）；实得 {active:?}"
        );
        assert_eq!(p, 0.0, "孤立 ShortDiff（父不可恢复）剪枝 ⟹ p̃=0");
    }


    /// ★codex Q4 发现 A 修复测试：Stale 非边界根被 prune（非伪造 parent:None root）。
    ///
    /// 持仓腿 ID 不在当前因果树（Stale）+ `is_boundary_root=false`（非真边界根 ∂）⟹ prune（不入 raw），
    /// AncOK 严格 §13 line 671。旧逻辑伪造 `parent:None` ⟹ AncOK 恒等放行（放宽 spec §13）。
    /// Q4 修复：保留原 `is_boundary_root`，非边界根 Stale = prune。
    #[test]
    fn stale_non_boundary_root_is_pruned_not_fabricated_root() {
        use super::super::super::interp::{assemble_gamma_with_tower, coverage_elements_with_tower, interpret};
        let tower = rc_tower(vec![
            Vec::new(),
            vec![nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up])],
        ]);
        let classification = Classification {
            levels: vec![LevelState { bsp: Rc::new(vec![sell_bsp(8)]), ..Default::default() }],
        };
        let (elements, cstart) = coverage_elements_with_tower(&classification, &tower);
        let gamma = assemble_gamma_with_tower(&classification, &tower);
        // 持仓腿：ID=(99,99) 不在当前因果树（Stale）+ is_boundary_root=false（非真边界根 ∂）。
        // parent_id=Some((1,0)) 表示它本应有父（非 ∂ 根），但父不在当前树。
        let stale_non_root = ActiveLeg {
            level: 1, dir: VoiceSide::Long, source_index: 12, lambda: 0,
            id: eid(99, 99), parent_id: Some(eid(1, 0)), is_boundary_root: false, op_parent: Some(eid(1, 0)),
        };
        let buckets = interpret(&gamma, &[stale_non_root]);
        let reg = super::super::super::persistent::PersistentRegistry::new();
        let (active, p) =
            coverage_step_from_buckets(view_split(&elements, cstart), &[stale_non_root], &buckets, 1000.0, &cfg(), None, &reg);
        // Stale 非边界根 ⟹ prune（不入 raw）⟹ 不在 A_{t+1}。
        assert!(
            !active.iter().any(|l| l.id == eid(99, 99)),
            "Stale 非边界根被 prune（非伪造 root，spec §13 严格）；实得 {active:?}"
        );
        // p̃ 不含该腿（pruned ⟹ 不贡献）。
        let _ = p; // p̃ 可非零（若 ShortDiff 候选准入），关键是 stale_non_root 不在 active。
    }


