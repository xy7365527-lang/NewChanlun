use super::*;
use super::super::test_support::*;
use crate::theta_v0::types::{Direction, BspBits};
use crate::theta_v0::classifier::LevelState;
use super::super::super::interp::{ActiveLeg, Buckets, Candidate};

    /// ★(I-1) open 候选父注入测试（depth>0 准入的**真**生产场景，codex 行级裁决坐实，642/644）：
    ///
    /// L2 诊断坐实根因：父 carrier **不在 prev_active 持仓腿**（sd_parent_held=0），也几乎不与子同 bar
    /// 共现（容器 BSP 极稀疏），但**在 persistent registry LiveDetached 存活**（sd_parent_registry_alive
    /// ≈51525）。`cross_bar_held_container_*` 测的是父作 prev_active 持仓腿的路径——**不**覆盖此真实场景。
    /// 本测试覆盖：**prev_active 为空、父仅 registry-live** ⟹ open 候选父注入必须从 registry 恢复祖先链
    /// 才能让子腿 AncOK 准入。
    ///
    /// **RED（修复前）**：open 候选路径从不调 `restore_ancestor_chain_from_registry` ⟹ 父 carrier 不在
    /// raw（既非 held 又非 open 候选自身）⟹ 生产 AncOK（#183 归一后 =
    /// `exit::step_active_set_with_subtree_close`）判子声部祖先不齐 ⟹ AncOK 全剪 ⟹
    /// active 不含 L0 ReverseOpen 子腿。**GREEN（修复后）**：open 候选父 parent_id registry-live ⟹ 恢复父
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
            "bar2：L0 ReverseOpen 子腿借 registry-live 父 carrier（非持仓腿）经 open 父注入准入 depth>0；实得 {active2:?}"
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
        restore_ancestor_chain_from_registry(&mut work, &mut raw, &reg, carrier, &id_idx, &mut overlay_seen, 0, &[], &mut pending);

        assert_eq!(work.len(), 1, "restore 不得 push 重复 id 元素（应复用 work[0]，overlay 空）");
        assert_eq!(raw, vec![0], "raw 须复用现有 idx 0，非追加新 idx");
        let dup = raw.iter().filter(|&&r| work[r].id == carrier).count();
        assert_eq!(dup, 1, "carrier 在 raw 中须唯一表示（双计根因守卫）");
    }

    /// ★#247 缺口二**直测**：restore 恢复元素的**角色输入重建**（`parent` 索引 + `attached_dir`）。
    ///
    /// **RED（修复前）**：`restore_ancestor_chain_from_registry` 构造 `CoverageElement` 写死
    /// `parent:None, attached_dir:None`（只留 `parent_id`）⟹ `parent_sign(None)=0 ⟹ V=Ambient`、
    /// `ell_p=ell_g ⟹ G=SameLevel`——恢复元素**恒定**命中 `operation_role_*` 自称「防御性（不应
    /// 到达）」的 `父越界/None` 分支，角色经 `dir_weight`/`w_grade`/`element_depth` 改下单权重。
    /// **GREEN（修复后）**：walk 结束回填 `parent_id → work idx`，子元素得 `parent=Some(父idx)`、
    /// `attached_dir=Some(父 eps)` ⟹ V/G 由真实 (σ_p, ℓ_p) 判定（此形态：δ=Long vs σ_p=Short ⟹
    /// ReverseOpen；ℓ_g=0 < ℓ_p=1 ⟹ SubLevel）。真 ∂ 根（`parent_id=None`）保持 None——那是**正确**
    /// 语义（Ambient），非防御兜底。
    ///
    /// 定义依据：#247 缺口二（#244 承重核查）；spec §7.2 V(g)=分类(σ_{p(g)}, δ_g)；anc.pdf §15
    /// （LiveDetached 腿的 parent 仍是 op_parent，只是当前 snapshot 未展示）。
    #[test]
    fn restore_rebuilds_parent_link_and_attached_dir() {
        let child = eid(0, 700);
        let parent = eid(1, 701);
        // registry 源元素——work 树前缀**不含**它们 ⟹ 真 LiveDetached，须从持久身份恢复。
        let src = vec![
            CoverageElement {
                lambda: 10, rho: 20, eps: VoiceSide::Long, level: 0,
                parent: None, attached_dir: None, id: child, parent_id: Some(parent),
            },
            CoverageElement {
                lambda: 5, rho: 30, eps: VoiceSide::Short, level: 1,
                parent: None, attached_dir: None, id: parent, parent_id: None,
            },
        ];
        let reg = super::super::super::persistent::PersistentRegistry::new().merge(&src, &[]);
        let base: Vec<CoverageElement> = vec![];
        let mut work = ElementView::new(&base);
        let mut raw: Vec<usize> = Vec::new();
        let id_idx = build_tree_id_index(&base);
        let mut overlay_seen = std::collections::HashMap::new();
        let mut pending = Vec::new();
        restore_ancestor_chain_from_registry(
            &mut work, &mut raw, &reg, child, &id_idx, &mut overlay_seen, 0, &[], &mut pending,
        );
        // ★票#350：parent/attached_dir 不再在函数内立即解析，测试须补跑生产统一 fixup 才能观测。
        resolve_pending_parent_fixups(&mut work, &pending, &id_idx, &overlay_seen, &raw);
        assert_eq!(raw, vec![0, 1], "walk 自下而上恢复 child(0) → parent(1)");
        assert_eq!(work[0].parent, Some(1), "恢复元素 parent 索引须由 parent_id 重建（#247 缺口二）");
        assert_eq!(
            work[0].attached_dir, Some(VoiceSide::Short),
            "attached_dir 须取父元素绝对方向 σ_p（Short）"
        );
        assert_eq!(work[1].parent, None, "真 ∂ 根（parent_id=None）保持 None——正确语义非兜底");
        assert_eq!(work[1].attached_dir, None, "真 ∂ 根 σ_p=0 ⟹ Ambient（正确）");
        let flat = work.as_contiguous().into_owned();
        assert_eq!(
            vertical_relation(&flat, 0), Vertical::ReverseOpen,
            "恢复元素不再恒定落 V=Ambient（δ=Long vs σ_p=Short ⟹ ReverseOpen）"
        );
        assert_eq!(
            grade_relation(&flat, 0), GradeRel::SubLevel,
            "恢复元素不再恒定落 G=SameLevel（ℓ_g=0 < ℓ_p=1 ⟹ SubLevel）"
        );
    }

    /// ★#247 **p̃ / 腿集合对拍**（缺口二的下单影响，生产函数端到端）：两级 registry 恢复祖先链
    /// （child ← P1 ← P2，全 LiveDetached）经 held 路 restore 注入 ⟹ A_{t+1}={child,P1,P2}。
    ///
    /// **腿集合（身份）不变 —— 限定：仅在 gross cap 未激活时成立**（★#247 C3-3）。
    /// `element_as_leg` 只读 λ/ρ/ε/ℓ/id/parent_id，角色输入重建不碰这些 ⟹ AncOK 成员资格不变。
    /// 但**成员资格不等于 next_active 成员资格**：`risk=Some(..)` 且 `enforce_gross_cap=true` 时，
    /// [`apply_gross_cap`] 按 units 求缩放 `c_r`，units 变 ⟹ `gross_zeroed` 可能变 ⟹ 幽灵腿防护
    /// 剔除的开仓腿集合可能变 ⟹ **`next_active` 成员资格会变**。本测试 `risk=None`，故断言的
    /// 「身份不变」应读作 **「`enforce_gross_cap=false`/`risk=None` 下身份不变」**，不是无条件命题。
    ///
    /// ## p̃ 三口径对拍（★#247 C2 关票条件；default config，`base_units=1000`）
    ///
    /// | 元素 | 改前（全 Ambient） | 半修（仅祖先，bc7b8c26dd） | 全修（祖先+持仓腿，本 commit） |
    /// |---|---|---|---|
    /// | child L0 Long | d=0, w=0.60 → **+600** | d=0 → +600（未修） | **d=2, w=0.10 → +100** |
    /// | P1 L1 Short | d=0, w=0.60 → **−600** | **d=1, w=0.30 → −300** | d=1 → −300 |
    /// | P2 L2 Short（真 ∂ 根）| d=0 → **−600** | d=0 → −600（正确） | d=0 → −600 |
    /// | **p̃** | **−600** | **−300** | **−800**（本测试断言） |
    ///
    /// 半修（只修 restore 祖先、不修 [`held_stale_reregister_idx`]）在本场景把 p̃ 推到 −300，
    /// **离全重建口径 −800 比改前的 −600 更远** —— 这正是 C2 必须同步修的理由（该形态是 held 路
    /// 常态：LiveDetached 分支 restore 紧接重注册，触发腿必走伪根路径）。
    ///
    /// **「差异全部来自 depth」的限定 —— 仅在 Neutral preset + `w_grade=[1,1]` 下成立**（★#247 C3-2）。
    /// `w = depth_weight(d) × dir_weight(role,d) × w_grade(role)`。本对拍下 `dir_weight` 恒 1.0
    /// （`ThetaDirPreset::Neutral`，`config.rs` default）、`w_grade` 恒 1.0（default `[1.0,1.0]`），
    /// 故三口径差异确实只由 `depth_weight` 产生。但 `wverify_run.rs` 有环境变量驱动的
    /// `Follow{eta_adv}` / `Adversary{eta_same}` **生产入口**：那些运行下 V 从 `Ambient` 变
    /// `FollowParent`/`ReverseOpen` 会打开 `dir_weight` 的查表 CASE（Ambient 走恒 1.0 的 CASE），
    /// `w_grade` 同理随 G 从 `SameLevel` 变 `SubLevel` 切槽位 ⟹ **units 另有两条独立变化通道**。
    /// 「差异全部来自 depth」**不是本修复的普遍性质**，只是 Neutral 口径下的读数。
    #[test]
    fn restore_role_rebuild_changes_p_tilde_leg_set_unchanged() {
        let child = eid(0, 720);
        let p1 = eid(1, 721);
        let p2 = eid(2, 722);
        // registry 源：child ← P1 ← P2（P2 真 ∂ 根）。二次 merge（空 snapshot）把三者置
        // snapshot_present=false ⟹ LiveDetached（restore 路径的前提态）。
        let src = vec![
            CoverageElement { lambda: 10, rho: 20, eps: VoiceSide::Long, level: 0,
                parent: None, attached_dir: None, id: child, parent_id: Some(p1) },
            CoverageElement { lambda: 8, rho: 22, eps: VoiceSide::Short, level: 1,
                parent: None, attached_dir: None, id: p1, parent_id: Some(p2) },
            CoverageElement { lambda: 5, rho: 30, eps: VoiceSide::Short, level: 2,
                parent: None, attached_dir: None, id: p2, parent_id: None },
        ];
        let reg = super::super::super::persistent::PersistentRegistry::new().merge(&src, &[]).merge(&[], &[]);
        let leg_child = ActiveLeg {
            level: 0, dir: VoiceSide::Long, source_index: 20, lambda: 10,
            id: child, parent_id: Some(p1), is_boundary_root: false, op_parent: Some(p1),
        };
        let prev = [leg_child];
        let buckets = Buckets { close: vec![], open: vec![], record: vec![] };
        let tree: Vec<CoverageElement> = vec![];
        let (active, p) =
            coverage_step_from_buckets(view_split(&tree, 0), &prev, &buckets, 1000.0, &cfg(), None, &reg);
        let mut ids: Vec<_> = active.iter().map(|l| l.id).collect();
        ids.sort_by_key(|i| (i.level, i.ordinal));
        assert_eq!(
            ids, vec![child, p1, p2],
            "腿集合（身份）= {{child,P1,P2}}，角色重建不改成员资格（限定：risk=None/gross cap 未激活）"
        );
        assert!(
            (p - (-800.0)).abs() < 1e-9,
            "p̃ 三口径对拍：改前 −600 / 半修（仅祖先）−300 / 全修（祖先+持仓腿）−800；实得 {p}"
        );
        // 三口径中另两口径的**复现**（非口算）：三口径下 work 元素数组的唯一差异 = 各元素的
        // parent/attached_dir（id/parent_id/λ/ρ/ε/ℓ 逐位相同 ⟹ raw 序与 AncOK 成员资格相同）。
        // 按各自形态重建同一元素数组、喂同一 p̃ 管线读数。work 序 = [P1(0), P2(1), child(2)]
        //（restore 先于 held 重注册 push）。
        let mut pre_fix = vec![
            CoverageElement { lambda: 8, rho: 22, eps: VoiceSide::Short, level: 1,
                parent: None, attached_dir: None, id: p1, parent_id: Some(p2) },
            CoverageElement { lambda: 5, rho: 30, eps: VoiceSide::Short, level: 2,
                parent: None, attached_dir: None, id: p2, parent_id: None },
            CoverageElement { lambda: 10, rho: 20, eps: VoiceSide::Long, level: 0,
                parent: None, attached_dir: None, id: child, parent_id: Some(p1) },
        ];
        let pre_view = ElementView::new(&pre_fix);
        let pre_p = net_target_units(&strategy_target_legs(&pre_view, &[0, 1, 2], 1000.0, &cfg()));
        assert!((pre_p - (-600.0)).abs() < 1e-9, "口径①改前（全 Ambient）p̃ 复现 = −600；实得 {pre_p}");
        // 口径②半修（bc7b8c26dd）：只有 restore 祖先 P1 拿到真 parent，持仓腿 child 仍伪根。
        pre_fix[0].parent = Some(1);
        pre_fix[0].attached_dir = Some(VoiceSide::Short);
        let half_view = ElementView::new(&pre_fix);
        let half_p = net_target_units(&strategy_target_legs(&half_view, &[0, 1, 2], 1000.0, &cfg()));
        assert!(
            (half_p - (-300.0)).abs() < 1e-9,
            "口径②半修（仅祖先）p̃ 复现 = −300 —— 比改前 −600 离全修 −800 **更远**；实得 {half_p}"
        );
    }

    /// ★#247 C3-1（影子评审证据级）：**深链 ≥3 级 ⟹ `depth_weight` 越界归零**（0.60 → **0.0**）。
    ///
    /// `voice.rs` `depth_weight` = `depth_weights.get(depth).unwrap_or(0.0)`，default 表
    /// `[0.60, 0.30, 0.10]` 只覆盖 depth 0/1/2 ⟹ **depth ≥ 3 的元素 `w_depth = 0`，整条腿 units 归零**
    /// （spec:42「权重表外的深度不获得资金，剩余资金留现金、不按比例重分配」——归零是**定义行为**，
    /// 不是 bug）。改前这些元素恒 `depth=0` 拿满权 0.60，故 #247 在深链上的量级是 **0.60→0**，
    /// 远大于已对拍的 0.60→0.30 / 0.60→0.10。本测试补上这个量级最大的行为面。
    ///
    /// **本测试不改权重表**（`depth_weights` 的表长/语义另裁），只如实呈现「≥3 级链权重归零」。
    /// registry 恢复链长度**无上界**（walk 只受 registry 链长约束）⟹ 深链在数据层可达；真实窗口
    /// 里的 depth 分布仍需运行时 trace（未决，见 ⑥）。
    ///
    /// 四级链 child(L0) ← P1(L1) ← P2(L2) ← P3(L3，真 ∂ 根)，全 LiveDetached，held 路注入：
    ///
    /// | 元素 | 改前（全 Ambient） | 全修（本 commit） |
    /// |---|---|---|
    /// | child L0 Long | d=0, w=0.60 → +600 | **d=3, w=0.0 → 0**（越界归零） |
    /// | P1 L1 Short | d=0 → −600 | d=2, w=0.10 → −100 |
    /// | P2 L2 Short | d=0 → −600 | d=1, w=0.30 → −300 |
    /// | P3 L3 Short（∂ 根）| d=0 → −600 | d=0, w=0.60 → −600 |
    /// | **p̃** | **−1200** | **−1000** |
    ///
    /// 同 C3-2 限定：读数成立于 Neutral preset + `w_grade=[1,1]`。
    #[test]
    fn restore_deep_chain_depth_ge3_weight_zeroed() {
        let child = eid(0, 730);
        let p1 = eid(1, 731);
        let p2 = eid(2, 732);
        let p3 = eid(3, 733);
        let src = vec![
            CoverageElement { lambda: 12, rho: 20, eps: VoiceSide::Long, level: 0,
                parent: None, attached_dir: None, id: child, parent_id: Some(p1) },
            CoverageElement { lambda: 10, rho: 22, eps: VoiceSide::Short, level: 1,
                parent: None, attached_dir: None, id: p1, parent_id: Some(p2) },
            CoverageElement { lambda: 8, rho: 26, eps: VoiceSide::Short, level: 2,
                parent: None, attached_dir: None, id: p2, parent_id: Some(p3) },
            CoverageElement { lambda: 5, rho: 30, eps: VoiceSide::Short, level: 3,
                parent: None, attached_dir: None, id: p3, parent_id: None },
        ];
        let reg = super::super::super::persistent::PersistentRegistry::new().merge(&src, &[]).merge(&[], &[]);
        let leg_child = ActiveLeg {
            level: 0, dir: VoiceSide::Long, source_index: 20, lambda: 12,
            id: child, parent_id: Some(p1), is_boundary_root: false, op_parent: Some(p1),
        };
        let prev = [leg_child];
        let buckets = Buckets { close: vec![], open: vec![], record: vec![] };
        let tree: Vec<CoverageElement> = vec![];
        let (active, p) =
            coverage_step_from_buckets(view_split(&tree, 0), &prev, &buckets, 1000.0, &cfg(), None, &reg);
        assert_eq!(active.len(), 4, "四级链全部恢复入 A_{{t+1}}（身份不变，限定 risk=None）");
        assert!(
            (p - (-1000.0)).abs() < 1e-9,
            "深链 p̃：改前 −1200 → 全修 −1000（child d=3 越界归零 0.60→0.0）；实得 {p}"
        );
        // 单元级直读 depth ≥ 3 那条腿的 units = 0（不经净额抵消遮蔽）。
        let deep = vec![
            CoverageElement { lambda: 12, rho: 20, eps: VoiceSide::Long, level: 0,
                parent: Some(1), attached_dir: Some(VoiceSide::Short), id: child, parent_id: Some(p1) },
            CoverageElement { lambda: 10, rho: 22, eps: VoiceSide::Short, level: 1,
                parent: Some(2), attached_dir: Some(VoiceSide::Short), id: p1, parent_id: Some(p2) },
            CoverageElement { lambda: 8, rho: 26, eps: VoiceSide::Short, level: 2,
                parent: Some(3), attached_dir: Some(VoiceSide::Short), id: p2, parent_id: Some(p3) },
            CoverageElement { lambda: 5, rho: 30, eps: VoiceSide::Short, level: 3,
                parent: None, attached_dir: None, id: p3, parent_id: None },
        ];
        let dv = ElementView::new(&deep);
        assert_eq!(element_depth(&dv, 0), 3, "child 真嵌套 depth = 3（沿 parent 链）");
        let legs = strategy_target_legs(&dv, &[0, 1, 2, 3], 1000.0, &cfg());
        assert!(
            legs[0].units.abs() < 1e-12,
            "depth=3 ⟹ depth_weight 越界归零 ⟹ 该腿 units = 0（spec:42 未用部分留现金）；实得 {}",
            legs[0].units
        );
        // 改前口径（全 Ambient/d=0）复现：−1200。
        let pre: Vec<CoverageElement> = deep
            .iter()
            .map(|e| CoverageElement { parent: None, attached_dir: None, ..*e })
            .collect();
        let pre_p = net_target_units(&strategy_target_legs(
            &ElementView::new(&pre), &[0, 1, 2, 3], 1000.0, &cfg(),
        ));
        assert!((pre_p - (-1200.0)).abs() < 1e-9, "深链改前 p̃ 复现 = −1200；实得 {pre_p}");
    }

    /// ★#247 C1（影子评审阻断级）：**`element_depth` 环路 fuel 硬门 —— 造环即显式失败 + 探针计数**。
    ///
    /// 缺口二回填后 restore 元素的 `parent` 可指向 overlay ⟹ 链的无环性转而依赖 registry
    /// `structural_parent_id` 无环，而 `persistent.rs` 写入侧（三处直接赋值、其中一处跨 bar 可变
    /// 更新）**无任何无环校验** ⟹ 环在数据层不可排除。改前恢复元素 `parent` 恒 None ⟹ 恒 depth=0，
    /// 不具备成环条件；本 commit 起首次具备。
    ///
    /// 本测试人为造环（A.parent=B、B.parent=A），断言 [`element_depth`]：
    /// ① **显式 panic**（不静默钳制成某个合法深度）；② 计入探针 `element_depth_fuel_exhausted`。
    ///
    /// 「walk 遇重复即止」只证 walk 终止，**不证** parent 图无环——见 [`element_depth`] doc 的订正。
    #[test]
    fn element_depth_cycle_fails_loudly_with_probe() {
        let a = eid(0, 740);
        let b = eid(1, 741);
        // 人为环：A.parent=1(B)、B.parent=0(A)。裸 while let 会无限循环 ⟹ 生产挂死。
        let cyclic = vec![
            CoverageElement { lambda: 0, rho: 10, eps: VoiceSide::Long, level: 0,
                parent: Some(1), attached_dir: Some(VoiceSide::Short), id: a, parent_id: Some(b) },
            CoverageElement { lambda: 0, rho: 12, eps: VoiceSide::Short, level: 1,
                parent: Some(0), attached_dir: Some(VoiceSide::Long), id: b, parent_id: Some(a) },
        ];
        let view = ElementView::new(&cyclic);
        ancok_probe_reset();
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| element_depth(&view, 0)));
        assert!(r.is_err(), "环路必须**显式失败**（不静默钳制成合法深度）");
        assert_eq!(
            ancok_probe_snapshot().element_depth_fuel_exhausted, 1,
            "环路命中须计入探针 element_depth_fuel_exhausted（witness 面）"
        );
        // 无环链不受影响（fuel 门不误伤）：A→B(∂根)。
        let acyclic = vec![
            CoverageElement { parent: Some(1), ..cyclic[0] },
            CoverageElement { parent: None, attached_dir: None, ..cyclic[1] },
        ];
        ancok_probe_reset();
        assert_eq!(element_depth(&ElementView::new(&acyclic), 0), 1, "无环链正常返回 depth=1");
        assert_eq!(
            ancok_probe_snapshot().element_depth_fuel_exhausted, 0,
            "fuel 门不误伤合法链（简单路径最长 len−1 条边 < fuel=len）"
        );
    }

    /// ★#247 缺口二**语义裁定见证**：祖先链断裂（registry 丢失祖先）时恢复元素 `parent` 留 None，
    /// 但**与「防御分支」/「真 ∂ 根」分离**——`parent_id` 保持 `Some` ⟹ 统一 AncOK
    /// （[`super::super::exit::step_active_set_with_subtree_close`]）按 id 判祖先不在集 ⟹ **必被剪除**，
    /// 从不进 `next_idx` ⟹ [`strategy_target_legs`] 从不对它求角色。
    ///
    /// 这是「防御分支回归不应到达」的**不变量论证**（非注释宣称）：能被打分的恢复元素要么
    /// `parent_id=None`（真 ∂ 根，Ambient 正确），要么 `parent` 已重建（角色真实）——`parent_id=Some`
    /// 且 `parent=None` 的元素恒被 AncOK 剪除。
    #[test]
    fn restore_broken_chain_element_pruned_never_scored() {
        let child = eid(0, 710);
        let lost_parent = eid(1, 711);
        // registry 只有 child，其 structural_parent_id 指向**不在 registry** 的祖先 ⟹ 链断
        // （`restore_break_registry_lost`）。
        let src = vec![CoverageElement {
            lambda: 10, rho: 20, eps: VoiceSide::Long, level: 0,
            parent: None, attached_dir: None, id: child, parent_id: Some(lost_parent),
        }];
        let reg = super::super::super::persistent::PersistentRegistry::new().merge(&src, &[]);
        let base: Vec<CoverageElement> = vec![];
        let mut work = ElementView::new(&base);
        let mut raw: Vec<usize> = Vec::new();
        let id_idx = build_tree_id_index(&base);
        let mut overlay_seen = std::collections::HashMap::new();
        let mut pending = Vec::new();
        ancok_probe_reset();
        restore_ancestor_chain_from_registry(
            &mut work, &mut raw, &reg, child, &id_idx, &mut overlay_seen, 0, &[], &mut pending,
        );
        // ★票#350：unresolved 判定（含 probe 计数）移到统一 fixup 时点。
        resolve_pending_parent_fixups(&mut work, &pending, &id_idx, &overlay_seen, &raw);
        assert_eq!(work[0].parent, None, "链断 ⟹ parent 无法解析，留 None");
        assert_eq!(work[0].parent_id, Some(lost_parent), "parent_id **保持 Some**（不是 ∂ 根）");
        let probe = ancok_probe_snapshot();
        assert_eq!(probe.restore_parent_unresolved, 1, "链断元素计入 restore_parent_unresolved");
        assert_eq!(probe.restore_break_registry_lost, 1, "链断原因 = registry 丢失祖先");
        // 不变量：parent_id=Some 且父不在集 ⟹ 统一 AncOK 必剪 ⟹ 该元素从不被 strategy_target_legs 打分。
        let legs: Vec<ActiveLeg> = raw.iter().map(|&i| element_as_leg(&work[i])).collect();
        let next = super::super::super::exit::step_active_set_with_subtree_close(&legs, &[], &[]);
        assert!(
            next.is_empty(),
            "链断恢复元素（parent_id=Some 且父不在集）必被 AncOK 剪除 ⟹ 角色从不被消费；实得 {next:?}"
        );
    }

    /// ★#216 restore 缺口**根因直测**（m3_partition_btc_fullhistory/m6_btc_oos_r_decomposition 炸
    /// coverage.rs:2376 的最小复现）：prev_active 中 **restore 祖先腿排在其子腿之后**（生产序——子腿
    /// 先开仓、父 carrier 经 open 父注入 restore 恢复入 next_active ⟹ A_t 序 = [子, 父]），下一 bar
    /// 两腿同 Stale LiveDetached：子腿先处理，其 op_parent 链 restore 把父从 registry 恢复 push 入
    /// work/raw；轮到父腿自身 Stale 重注册时若**不复用** restore 已 push 的现有 idx 而再 push 新元素
    /// ⟹ raw/next_active 同 ElementId 占两槽 ⟹ strategy_target_legs 双计 p̃ 伪证。
    ///
    /// **RED（修复前）**：父腿 Stale 重注册不查 id_idx/overlay_seen ⟹ 重复 push ⟹ next_active 含两个
    /// 父 id（炸 I-1 双计守卫 debug_assert）。**GREEN（修复后）**：重注册查得 restore 已 push 的 idx
    /// ⟹ 复用其槽位，每 ElementId 在 next_active 唯一。
    ///
    /// 定义依据：#216 票面 + spec §13 元素集语义（同一 ElementId 在 A_t 唯一）+ anc.pdf §9 rule 2
    /// （I1 持久身份——重注册恢复的是同一持久元素，非新建）。
    #[test]
    fn held_leg_reregister_reuses_restore_pushed_idx_no_duplicate_id() {
        let child = eid(0, 900);
        let parent = eid(1, 901);
        // prev_active = [子, 父]（生产序）。两腿 id 不在当前树（空树 ⟹ 皆 Stale）；registry 由空
        // snapshot + held 引用 merge 建 LiveDetached 条目（snapshot_present=false；§I4 op_parent 持久）。
        let leg_child = ActiveLeg {
            level: 0, dir: VoiceSide::Long, source_index: 30, lambda: 20,
            id: child, parent_id: Some(parent), is_boundary_root: false, op_parent: Some(parent),
        };
        let leg_parent = ActiveLeg {
            level: 1, dir: VoiceSide::Long, source_index: 30, lambda: 20,
            id: parent, parent_id: None, is_boundary_root: true, op_parent: None,
        };
        let prev = [leg_child, leg_parent];
        // 空三桶（无 open/close）——隔离 held Stale 重注册路径（restore 缺口唯一作用点）。
        let buckets = Buckets { close: vec![], open: vec![], record: vec![] };
        let tree: Vec<CoverageElement> = vec![];
        let reg = super::super::super::persistent::PersistentRegistry::new().merge(&tree, &prev);
        let (active, _p) =
            coverage_step_from_buckets(view_split(&tree, 0), &prev, &buckets, 1000.0, &cfg(), None, &reg);
        // 子腿借 restore 父链通过 AncOK（§13 持仓准入）；父（边界根）保留。两者皆须在 A_{t+1}。
        assert!(active.iter().any(|l| l.id == child), "子腿借 restore 恢复的父链 AncOK 准入；实得 {active:?}");
        let n_parent = active.iter().filter(|l| l.id == parent).count();
        assert_eq!(
            n_parent, 1,
            "restore 祖先腿 Stale 重注册须复用现有 idx（next_active 每 ElementId 唯一，#216）；实得 {active:?}"
        );
    }

    /// ★#216 restore 缺口**真实根因直测**（m6 p3fold 诊断 dump 实证，2026-07-24）：同 bar 两个 open
    /// 候选（**反向**双信号，Long+Short）attach **同一 carrier ElementId**，候选元素数组各携一份拷贝
    /// （相邻 gamma_index、lambda==rho==source_index 点元素）。open 注册循环只按 idx 判重
    /// （`!raw.contains(&idx)`）⟹ 两份拷贝各占一槽入 raw ⟹ next_active 同 id 双腿 ⟹ 炸 (I-1) 双计
    /// 守卫（coverage.rs:2376 debug_assert，m3/m6 实证）。
    ///
    /// **RED（修复前）**：两候选 idx 不同 ⟹ 双双入 raw ⟹ next_active 含同 carrier id 两腿（炸断言）。
    /// **GREEN（修复后）**：反向同 id 候选**成对湮灭**——剔除先序拷贝、本候选亦不入（净敞口 0 =
    /// 幽灵腿防护裁定4 同款：净零目标从未建仓；与修复前净额路径 p̃ 贡献 ±q−q=0 一致，轨迹不翻）。
    ///
    /// 定义依据：#216 票面 + spec §13 元素集语义（同一 ElementId 在 A_t 唯一）+ 裁定4 幽灵腿防护。
    /// 父 carrier 经 registry LiveDetached 恢复（open 父注入），保证候选 AncOK 准入以暴露双写入。
    #[test]
    fn open_candidates_same_carrier_id_reverse_pair_annihilates() {
        let carrier = eid(0, 162);
        let parent = eid(1, 33);
        // 候选段两元素：同 carrier id、反向 eps（生产 dump：lambda==rho==source_index 点元素）。
        let mk = |eps: VoiceSide| CoverageElement {
            lambda: 42, rho: 42, eps, level: 0,
            parent: None, attached_dir: None, id: carrier, parent_id: Some(parent),
        };
        let elements = vec![mk(VoiceSide::Long), mk(VoiceSide::Short)];
        let buckets = Buckets {
            close: vec![],
            open: vec![cand(0, 42, VoiceSide::Long, 0), cand(0, 42, VoiceSide::Short, 1)],
            record: vec![],
        };
        // 父 carrier 仅 registry LiveDetached（非持仓腿、不在候选段）⟹ open 父注入 restore 恢复入 raw
        // ⟹ 候选 AncOK 准入条件齐备（§13），双写入暴露于 next_active。
        let leg_parent = ActiveLeg {
            level: 1, dir: VoiceSide::Long, source_index: 40, lambda: 30,
            id: parent, parent_id: None, is_boundary_root: true, op_parent: None,
        };
        let reg = super::super::super::persistent::PersistentRegistry::new().merge(&[], &[leg_parent]);
        let (active, p) =
            coverage_step_from_buckets(view_split(&elements, 0), &[], &buckets, 1000.0, &cfg(), None, &reg);
        // 反向对湮灭：carrier 不占活动集槽位（净零从未建仓，裁定4 同款）；p̃ 无该 carrier 贡献。
        let carrier_legs: Vec<_> = active.iter().filter(|l| l.id == carrier).collect();
        assert_eq!(
            carrier_legs.len(), 0,
            "同 carrier 反向 open 对成对湮灭（净零不开仓，#216）；实得 {active:?}"
        );
        // 父 carrier restore 恢复在场（AncOK/父注入行为不因子候选湮灭而回退）。
        assert!(active.iter().any(|l| l.id == parent), "父 carrier 经 open 父注入 restore 恢复；实得 {active:?}");
        let _ = p; // p̃ 数值由父腿权重决定，不在本测试锁定范围（唯一性/湮灭为锁）。
    }

    /// ★#216 同向对照组：同 bar 两个**同向** open 候选 attach 同一 carrier（双买点同 carrier）——
    /// 首现序优先恰留一腿（同向双计 = (I-1) 守卫防的 p̃ +2q 伪证；or_insert 首现序约定一致）。
    ///
    /// **RED（修复前）**：两拷贝双入 raw ⟹ 同 id 双腿炸断言。**GREEN（修复后）**：首现候选准入，
    /// 重复拷贝跳过（next_active 每 ElementId 唯一）。
    ///
    /// 定义依据：#216 票面 + spec §13 元素集语义（同一 ElementId 在 A_t 唯一）+ 首现序约定
    /// （build_tree_id_index/overlay_seen 的 or_insert 首现序）。
    #[test]
    fn open_candidates_same_carrier_id_same_dir_dedup_first_wins() {
        let carrier = eid(0, 162);
        let parent = eid(1, 33);
        let mk = |lambda: usize| CoverageElement {
            lambda, rho: 42, eps: VoiceSide::Long, level: 0,
            parent: None, attached_dir: None, id: carrier, parent_id: Some(parent),
        };
        let elements = vec![mk(40), mk(42)];
        let buckets = Buckets {
            close: vec![],
            open: vec![cand(0, 40, VoiceSide::Long, 0), cand(0, 42, VoiceSide::Long, 1)],
            record: vec![],
        };
        let leg_parent = ActiveLeg {
            level: 1, dir: VoiceSide::Long, source_index: 40, lambda: 30,
            id: parent, parent_id: None, is_boundary_root: true, op_parent: None,
        };
        let reg = super::super::super::persistent::PersistentRegistry::new().merge(&[], &[leg_parent]);
        let (active, _p) =
            coverage_step_from_buckets(view_split(&elements, 0), &[], &buckets, 1000.0, &cfg(), None, &reg);
        let carrier_legs: Vec<_> = active.iter().filter(|l| l.id == carrier).collect();
        assert_eq!(
            carrier_legs.len(), 1,
            "同 carrier 同向 open 候选首现序判重（next_active 每 ElementId 唯一，#216）；实得 {active:?}"
        );
        assert_eq!(carrier_legs[0].dir, VoiceSide::Long, "同向候选首现方向保留");
    }

    /// ★#220 炸点根因直测（勘察报告 assertion2-restore-balance-scope-20260724「炸点实证」定案）：
    /// 同 bar 同 carrier 二类 Long+Short 候选对（nest✓）× **restore 在场腿**（held 路 LiveDetached
    /// 祖先链恢复注入 raw）——两候选经 #216 规则①「id 已在 raw ⟹ 持仓身份优先」让位（活动集侧
    /// 已正确去重），但 opened 外化若按「候选 id ∈ next_active 即配对」会把**同一条 restore 腿**
    /// 配对给两个候选 ⟹ opened ×2 ⟹ runner 双重登记（open_trades.insert 覆盖 + 双份镜像开仓），
    /// 物理平仓只消费最新登记条目 ⟹ 先注册实例成永不消账的孤儿（m3/m6 炸断言②门 Core{{1}}=
    /// 1296.87 之源，wf8 窗 bar=160606，炸点实证 §2 逐笔生命周期对照）。
    ///
    /// **RED（id 配对）**：两候选 id 均命中 restore 腿 ⟹ opened=2、两条配对腿同 id（一腿双登记）。
    /// **GREEN（路④ idx 配对）**：配对键 = 候选自身元素 idx（candidate_start+gamma_index）∈
    /// next_active_idx——被规则①让位的候选自身元素从未入 raw ⟹ opened 空；restore 腿按设计意图
    /// 留在活动集（物理在场、账面无外化——「非信号入场，无 z，不入 ledger」，StepTrace.opened doc）。
    ///
    /// 定义依据：#220 勘察「炸点实证」§3（真根因）+ §6 路④（修复交接口径）；#216 规则①复合
    /// （活动集去重语义不回退——restore 腿恰一条在场、方向取 registry）。
    #[test]
    fn opened_restore_leg_not_externalized_for_same_carrier_candidate_pair() {
        let carrier = eid(1, 3);
        let child = eid(0, 900);
        // held 路 restore 源：lvl0 子腿 Stale LiveDetached（不在树 ⟹ Stale；registry 有条目 ⟹
        // LiveDetached），op_parent=carrier ⟹ restore 把 carrier 注入 raw（炸点 bar 2978 形态）。
        let leg_child = ActiveLeg {
            level: 0, dir: VoiceSide::Long, source_index: 30, lambda: 20,
            id: child, parent_id: Some(carrier), is_boundary_root: false, op_parent: Some(carrier),
        };
        // carrier 在 base 树前缀（生产形态：carrier 为历史走势元素在因果树内）——restore 经 id_idx
        // **复用树 idx**（id_idx 优先于 overlay_seen 候选段拷贝），腿属性取树/registry 坐标
        // （炸点实证 leg_dir=registry dir，非候选拷贝方向）；registry 条目由 snapshot upsert 建立。
        let tree = vec![CoverageElement {
            lambda: 15, rho: 28, eps: VoiceSide::Long, level: 1,
            parent: None, attached_dir: None, id: carrier, parent_id: None,
        }];
        let reg = super::super::super::persistent::PersistentRegistry::new().merge(&tree, &[leg_child]);
        // 候选段：同 carrier id 的二类 Long+Short 候选对（nest✓，炸点实证形态）。parent_id=None
        // （边界根，隔离 open 父注入路径——restore 仅经 held 路注入，形态最小）。
        let mk = |eps: VoiceSide| CoverageElement {
            lambda: 42, rho: 42, eps, level: 1,
            parent: None, attached_dir: None, id: carrier, parent_id: None,
        };
        let candidates = vec![mk(VoiceSide::Long), mk(VoiceSide::Short)];
        let c2 = |dir: VoiceSide, gamma_index: usize| Candidate {
            level: 1,
            source_index: 42,
            bits: BspBits::default(),
            dir,
            bsp_class: 2,
            role: role(Horizontal::First, Vertical::Ambient, Dir::Plus),
            nest_confirmed: true,
            gamma_index,
            force: None,
        };
        let gamma = vec![c2(VoiceSide::Long, 0), c2(VoiceSide::Short, 1)];
        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        let (na, _ps, _o, trace) = pi_theta_step_traced(
            ElementView::from_parts(&tree, candidates), &gamma, &[leg_child], 0.0, 5, 1000.0,
            &r, w, KThetaRiskGate::open(), &cfg(), &reg, None, &protocol_hold(),
        );
        // restore 腿物理在场：活动集正确去重后恰一条，dir 取树/registry=Long（#216 持仓身份优先不回退）。
        let carrier_legs: Vec<_> = na.iter().filter(|l| l.id == carrier).collect();
        assert_eq!(carrier_legs.len(), 1, "restore 复用树 idx 注入 carrier 恰一条在场；实得 {na:?}");
        assert_eq!(carrier_legs[0].dir, VoiceSide::Long, "restore 腿方向取树/registry，非候选方向");
        // 路④：让位候选不外化——opened 空（一条物理腿不得被登记两次，断言②门孤儿之源消除）。
        assert!(
            trace.opened.is_empty(),
            "#220：restore 在场腿不得被同 carrier 候选对借 id 配对双重外化（配对键 = 候选自身元素 idx）；实得 {:?}",
            trace.opened.iter().map(|(c, l)| (c.dir, l.id)).collect::<Vec<_>>()
        );
    }

    /// ★#216 评审锁定（code-review Spec 轴严重项）：held Stale 腿 id 命中**候选段拷贝**时，重注册
    /// 不得复用该拷贝——候选元素 eps 是信号方向（可与持仓反向）、lambda==rho 点元素（λ 失真）、
    /// parent_id 非本腿 op_parent（I4 失真）。复用 ⟹ element_as_leg 采纳候选属性 = 持仓方向静默
    /// 翻转（未经 close/risk 路径，伪证）。
    ///
    /// **RED（区分前）**：helper 复用候选 idx ⟹ 活动腿 dir 被翻成候选 Short、source_index 被候选
    /// 点元素覆盖。**GREEN（区分后）**：候选段拷贝（idx < 候选段终点）不复用——held 腿 push 自身
    /// 元素（持仓身份优先），候选随后被 open 循环 ① 规则（id 已在 raw ⟹ 跳过）让位。
    ///
    /// 定义依据：anc.pdf I1（持久身份=腿自身身份）+ I4（op_parent 持久，非候选 parent_id）+
    /// open 循环 ①「持仓身份优先」（名实相符）；code-review Spec 轴 (c)1（2026-07-24）。
    #[test]
    fn held_leg_id_hits_candidate_copy_keeps_held_identity() {
        let carrier = eid(0, 162);
        let parent = eid(1, 33);
        // 候选段：同 id 的 **Short** 信号拷贝（lambda==rho 点元素）；持仓腿是同 id **Long**。
        let cand_elem = CoverageElement {
            lambda: 42, rho: 42, eps: VoiceSide::Short, level: 0,
            parent: None, attached_dir: None, id: carrier, parent_id: Some(parent),
        };
        let elements = vec![cand_elem];
        let leg_held = ActiveLeg {
            level: 0, dir: VoiceSide::Long, source_index: 40, lambda: 30,
            id: carrier, parent_id: Some(parent), is_boundary_root: false, op_parent: Some(parent),
        };
        let buckets = Buckets {
            close: vec![],
            open: vec![cand(0, 42, VoiceSide::Short, 0)],
            record: vec![],
        };
        // snapshot 含候选拷贝 ⟹ held 腿 held_state=LivePresent（候选段命中场景）；op_parent 经 §I4
        // 注册（snapshot_present=false，供 open 父注入 restore）。
        let reg = super::super::super::persistent::PersistentRegistry::new().merge(&elements, &[leg_held]);
        let (active, _p) =
            coverage_step_from_buckets(view_split(&elements, 0), &[leg_held], &buckets, 1000.0, &cfg(), None, &reg);
        // carrier 恰一腿，且为**持仓身份**：dir=Long（未被候选 Short 翻转）、source_index=40（未被
        // 候选点元素覆盖）、op_parent 保持（I4）。
        let carrier_legs: Vec<_> = active.iter().filter(|l| l.id == carrier).collect();
        assert_eq!(carrier_legs.len(), 1, "held 腿与候选拷贝同 id ⟹ 恰一腿（#216 唯一性）；实得 {active:?}");
        assert_eq!(carrier_legs[0].dir, VoiceSide::Long, "持仓身份优先：方向不得被候选信号翻转");
        assert_eq!(carrier_legs[0].source_index, 40, "持仓身份优先：坐标取腿自身（非候选点元素）");
        assert_eq!(carrier_legs[0].op_parent, Some(parent), "op_parent 持久（I4），非候选 parent_id 改写");
    }

    /// ★#446 补移植（影子评审 HIGH-1，2026-07-29）：restore 侧祖先命中**候选段拷贝**时不得复用——
    /// 门禁语义须与 [`held_stale_reregister_idx`] 一致（`held_leg_id_hits_candidate_copy_keeps_held_identity`
    /// 覆盖 held 侧同一碰撞）。本测试直测 [`restore_ancestor_chain_from_registry`] 本身：`overlay_seen`
    /// 命中的 ancestor idx 落在候选段（`< overlay_cand_end`），registry 持有该 ancestor 的持久身份
    /// （方向/坐标/structural_parent_id 均与候选拷贝不同）。
    ///
    /// **RED（修复前）**：门禁缺失 ⟹ 直接复用候选段 idx 入 raw、`cur` 沿候选伪 `parent_id` 上溯——
    /// `raw` 命中候选拷贝（信号方向/点元素坐标），且祖先链在候选伪 parent 处走错，真持久祖先
    /// （grandparent）未物化。**GREEN（修复后）**：候选段命中被过滤，restore 从 registry 新 push
    /// 持久身份元素（真实 λ/ρ/eps/structural_parent_id），沿真链上溯至 grandparent。
    ///
    /// 定义依据：影子评审 `shadow-642-review-20260729.md` HIGH-1（`held.rs:320` 一带缺失同款门禁，
    /// 可构造 raw 同 ElementId 双槽）；kimi `f838540eff`（#446）restore 侧候选段过滤。
    #[test]
    fn restore_does_not_reuse_candidate_segment_copy_for_ancestor() {
        let ancestor = eid(1, 800);
        let grandparent = eid(2, 801);
        // 候选段拷贝：同 id 信号方向 Short、lambda==rho 点元素、parent_id 是候选自身 Compose 父
        // （非持久 structural_parent_id）——与 `held_leg_id_hits_candidate_copy_keeps_held_identity`
        // 同构造（候选段与 held 侧共用同一张 overlay_seen，风险同级）。
        let cand_copy = CoverageElement {
            lambda: 99, rho: 99, eps: VoiceSide::Short, level: 1,
            parent: None, attached_dir: None, id: ancestor, parent_id: Some(eid(9, 999)),
        };
        let base: Vec<CoverageElement> = vec![];
        let mut work = ElementView::new(&base);
        work.push(cand_copy);
        // 候选段终点：本 bar 候选段已 push 完毕（此刻 work.len()==1），此后 work 只被 restore/held push
        // 增长——与生产 `step.rs:140` 的 overlay_cand_end 语义一致。
        let overlay_cand_end = work.len();
        let mut overlay_seen: std::collections::HashMap<_, _> = std::collections::HashMap::new();
        // 候选段登记（同生产 step.rs:131-135：for i in candidate_start..work.len() { ... } ）。
        overlay_seen.insert(ancestor, 0);

        // registry 持有该 ancestor 的持久身份：真实 λ/ρ/eps/structural_parent_id，均与候选拷贝不同。
        let persistent_ancestor = CoverageElement {
            lambda: 10, rho: 20, eps: VoiceSide::Long, level: 1,
            parent: None, attached_dir: None, id: ancestor, parent_id: Some(grandparent),
        };
        let persistent_grandparent = CoverageElement {
            lambda: 5, rho: 30, eps: VoiceSide::Short, level: 2,
            parent: None, attached_dir: None, id: grandparent, parent_id: None,
        };
        let reg = super::super::super::persistent::PersistentRegistry::new()
            .merge(&[persistent_ancestor, persistent_grandparent], &[]);

        let id_idx = build_tree_id_index(&base); // base 空 ⟹ 候选段/持久身份均不在 id_idx。
        let mut raw: Vec<usize> = Vec::new();
        let mut pending = Vec::new();
        restore_ancestor_chain_from_registry(
            &mut work, &mut raw, &reg, ancestor, &id_idx, &mut overlay_seen, overlay_cand_end, &[],
            &mut pending,
        );

        assert_eq!(
            work.len(), 3,
            "候选段拷贝不可复用 ⟹ restore 须从 registry 新 push ancestor + grandparent 两个持久元素"
        );
        assert!(!raw.contains(&0), "raw 不得含候选段 idx 0（候选拷贝非持久身份）；实得 raw={raw:?}");
        assert_eq!(raw.len(), 2, "沿持久 structural_parent_id 链上溯 ancestor→grandparent 恰两级；实得 raw={raw:?}");
        let new_ancestor_idx = raw[0];
        assert_eq!(
            work[new_ancestor_idx].eps, VoiceSide::Long,
            "新 push 元素须取 registry 持久方向 Long，不得沿用候选拷贝的信号方向 Short"
        );
        assert_eq!(work[new_ancestor_idx].lambda, 10, "新 push 元素须取 registry 持久坐标，非候选 λ==ρ 点元素");
        assert_eq!(
            work[new_ancestor_idx].parent_id, Some(grandparent),
            "新 push 元素须取 registry structural_parent_id，不得沿用候选伪 parent_id"
        );
        assert_eq!(work[raw[1]].id, grandparent, "上溯第二环须抵达真持久 grandparent（候选伪 parent_id 不会指向它）");
    }

