use super::*;
use super::super::test_support::*;
use crate::theta_v0::types::Direction;
use crate::theta_v0::classifier::LevelState;
use super::super::super::interp::{ActiveLeg, Buckets};

    /// ★#183 T4 归一**生产路径见证**（#179 裁决：结构对应是硬要求，子树清仓接线进生产 π loop）：
    /// 父腿被裁决终结（∈𝒟_x）⟹ **子树全清**——后代腿（含短差腿，无豁免）同刻清除，
    /// restore 不得复活被关父腿。
    ///
    /// **RED（归一前）**：环6 按字面 𝒟_x skip；LiveDetached 子腿的 restore 把被关父腿从 registry
    /// 复活注入 raw ⟹ 父复活、子借 restore 父链 AncOK 准入存活（「父关则子关」靠 AncOK 被动
    /// 兑现，被 restore 扩集击穿）。**GREEN（归一后）**：活动集一步更新归一
    /// [`super::super::exit::step_active_set_with_subtree_close`]——subtree_close 把种子扩为子树闭包
    /// （restore 复活的被关父命中种子 ⟹ 父子同清），短差腿无豁免（ADR 0001 条目4）。
    ///
    /// 定义依据：#148 T4（AncOK 子树清仓）+ #179 裁决（接线进生产、restore 扩集保留）+
    /// spec §13 line 671（子级短差腿存在 ⟹ 父容器存在）。
    #[test]
    fn t4_subtree_close_unifies_production_active_set_step() {
        let child = eid(0, 900);
        let parent = eid(1, 901);
        // 短差子腿（Short，反父 Long 方向 ⟹ ReverseOpen 角色）+ 父腿（Long，边界根）。
        let leg_child = ActiveLeg {
            level: 0, dir: VoiceSide::Short, source_index: 30, lambda: 20,
            id: child, parent_id: Some(parent), is_boundary_root: false, op_parent: Some(parent),
        };
        let leg_parent = ActiveLeg {
            level: 1, dir: VoiceSide::Long, source_index: 30, lambda: 20,
            id: parent, parent_id: None, is_boundary_root: true, op_parent: None,
        };
        let prev = [leg_child, leg_parent];
        // 父腿被裁决终结（interpret 规则2 的 𝒟_x）；空树 ⟹ 两腿皆 Stale（LiveDetached 场景：
        // restore 复活面暴露——归一前父腿经子腿 restore 复活注入 raw）。
        let buckets = Buckets { close: vec![leg_parent], open: vec![], record: vec![] };
        let tree: Vec<CoverageElement> = vec![];
        let reg = super::super::super::persistent::PersistentRegistry::new().merge(&tree, &prev);
        let (active, _p) =
            coverage_step_from_buckets(view_split(&tree, 0), &prev, &buckets, 1000.0, &cfg(), None, &reg);
        assert!(
            !active.iter().any(|l| l.id == parent),
            "父腿被裁决终结 ⟹ 不得入 A_{{t+1}}（restore 不得复活被关父腿）；实得 {active:?}"
        );
        assert!(
            !active.iter().any(|l| l.id == child),
            "短差子腿无豁免：父终结 ⟹ 子树全清（#148 T4 生产兑现）；实得 {active:?}"
        );
    }

    /// ★#183 restore 扩集语义**保留守护**（#179 裁决：restore 注入的扩集语义保留并在新路径
    /// 显式兑现）：同构场景但父腿**未被**裁决终结——LiveDetached 子腿的 restore 照常复活父链
    /// 入 raw，父（registry-live 祖先）入 A_{t+1}、子借父链 AncOK 准入。归一前后行为不变。
    #[test]
    fn t4_unify_preserves_restore_expansion_for_surviving_legs() {
        let child = eid(0, 900);
        let parent = eid(1, 901);
        let leg_child = ActiveLeg {
            level: 0, dir: VoiceSide::Short, source_index: 30, lambda: 20,
            id: child, parent_id: Some(parent), is_boundary_root: false, op_parent: Some(parent),
        };
        let leg_parent = ActiveLeg {
            level: 1, dir: VoiceSide::Long, source_index: 30, lambda: 20,
            id: parent, parent_id: None, is_boundary_root: true, op_parent: None,
        };
        let prev = [leg_child, leg_parent];
        // 父腿**不关**（close 空）⟹ restore 扩集语义面（对照组）。
        let buckets = Buckets { close: vec![], open: vec![], record: vec![] };
        let tree: Vec<CoverageElement> = vec![];
        let reg = super::super::super::persistent::PersistentRegistry::new().merge(&tree, &prev);
        let (active, _p) =
            coverage_step_from_buckets(view_split(&tree, 0), &prev, &buckets, 1000.0, &cfg(), None, &reg);
        assert!(
            active.iter().any(|l| l.id == parent),
            "restore 扩集保留：父链经子腿 restore 复活入 A_{{t+1}}；实得 {active:?}"
        );
        assert!(
            active.iter().any(|l| l.id == child),
            "子腿借 restore 恢复的父链 AncOK 准入（restore 语义不变）；实得 {active:?}"
        );
    }

    /// ★#226 断言①违例**根因直测**（m3 win9 bar=17032 炸点最小复现，勘察报告
    /// `.chanlun/review-results/assertion1-win9-open-restore-resurrect-20260724.md`）：同 bar
    /// 一类卖终结父 carrier（close 种子）× 次级顺父 open 候选挂该父——(I-1) open 父注入 restore
    /// **不得复活当 bar 已被裁决终结的祖先**（S3：父终结 ⟹ 子树清仓；与 A_t 段「restore 不得复活
    /// 被关父」同一教义，ℬ_x 段此前无种子兜底 = #179 保留面的复活泄漏）。种子不复活 ⟹ 候选父链
    /// 断裂 ⟹ 统一 AncOK 正常剪除（非 AncOK 加特例）。
    ///
    /// **RED（修复前）**：restore id_idx 命中树前缀复用 ⟹ 种子父复活入 ℬ_x、候选借尸准入——炸点
    /// 实证残余 = 触发腿自身 q=518.0948228547287（Core{1}）。**GREEN（修复后）**：restore 遇种子
    /// 中断，父子均不入 A_{t+1}，一类批末 Core{1} sep 物理残余=0（断言①口径）。
    #[test]
    fn open_parent_restore_skips_closed_seed_no_resurrect() {
        let child = eid(0, 900);
        let parent = eid(1, 901);
        // 树前缀含父元素（炸点 origin=tree-prefix：restore id_idx 复用路径）。
        let tree_elem = CoverageElement {
            lambda: 20, rho: 30, eps: VoiceSide::Long, level: 1,
            parent: None, attached_dir: None, id: parent, parent_id: None,
        };
        // 候选段：lvl 0 顺父开多候选（炸点 OPEN-CAND：class=3 buy3 FollowParent Long），parent_id=父。
        let cand_elem = CoverageElement {
            lambda: 42, rho: 42, eps: VoiceSide::Long, level: 0,
            parent: None, attached_dir: None, id: child, parent_id: Some(parent),
        };
        // 父持仓腿（炸点 PREV：Long/Ambient/边界根）——一类卖本 bar 终结（close 种子）。
        let leg_parent = ActiveLeg {
            level: 1, dir: VoiceSide::Long, source_index: 30, lambda: 20,
            id: parent, parent_id: None, is_boundary_root: true, op_parent: None,
        };
        let buckets = Buckets {
            close: vec![leg_parent],
            open: vec![cand(0, 42, VoiceSide::Long, 0)],
            record: vec![],
        };
        let snapshot = vec![tree_elem, cand_elem];
        let reg = super::super::super::persistent::PersistentRegistry::new().merge(&snapshot, &[leg_parent]);
        ancok_probe_reset();
        let (active, _p, sep, _idx) = coverage_step_from_buckets_sep(
            view_split(&[tree_elem, cand_elem], 1), &[leg_parent], &buckets, 1000.0, &cfg(), None, &reg,
        );
        assert_eq!(
            ancok_probe_snapshot().restore_break_closed_seed,
            1,
            "restore 遇当 bar 关闭种子中断一次（探针为凭）"
        );
        assert!(
            !active.iter().any(|l| l.id == parent),
            "当 bar 已被裁决终结的父不得经 open 父注入 restore 复活（S3/#226）；实得 {active:?}"
        );
        assert!(
            !active.iter().any(|l| l.id == child),
            "父终结 ⟹ 挂尸候选父链断裂，统一 AncOK 剪除（S3 子树清仓）；实得 {active:?}"
        );
        let core1: f64 = sep
            .iter()
            .filter(|s| s.id.level == 1)
            .filter(|s| {
                super::super::super::account::identity_of(s.role_v, s.side, s.id.level)
                    == Some(super::super::super::account::AccountIdentity::Core { level: s.id.level })
            })
            .map(|s| s.q_units)
            .sum();
        assert_eq!(core1, 0.0, "一类批末 Core{{1}} sep 物理残余=0（断言①口径）；实得 {core1}");
    }

    /// ★#226 **反手保留守护**（ID-3「允许当场反手」+ #179 保留面收窄声明）：同 bar 一类终结父
    /// carrier × **反向候选同 id 重开**（反手开空——候选**自身元素**入 ℬ_x，非 restore 注入）。
    /// 种子过滤只作用 restore 祖先注入，候选推送零改 ⟹ 反手照常准入（方向/坐标取候选）。
    #[test]
    fn open_reverse_candidate_on_closed_carrier_unaffected() {
        let parent = eid(1, 901);
        let tree_elem = CoverageElement {
            lambda: 20, rho: 30, eps: VoiceSide::Long, level: 1,
            parent: None, attached_dir: None, id: parent, parent_id: None,
        };
        // 反手候选元素：同 carrier id、eps=Short、点元素（lambda==rho）、无父（反向根）。
        let rev_elem = CoverageElement {
            lambda: 55, rho: 55, eps: VoiceSide::Short, level: 1,
            parent: None, attached_dir: None, id: parent, parent_id: None,
        };
        let leg_parent = ActiveLeg {
            level: 1, dir: VoiceSide::Long, source_index: 30, lambda: 20,
            id: parent, parent_id: None, is_boundary_root: true, op_parent: None,
        };
        let buckets = Buckets {
            close: vec![leg_parent],
            open: vec![cand(1, 55, VoiceSide::Short, 0)],
            record: vec![],
        };
        let snapshot = vec![tree_elem, rev_elem];
        let reg = super::super::super::persistent::PersistentRegistry::new().merge(&snapshot, &[leg_parent]);
        let (active, _p, _sep, _idx) = coverage_step_from_buckets_sep(
            view_split(&[tree_elem, rev_elem], 1), &[leg_parent], &buckets, 1000.0, &cfg(), None, &reg,
        );
        let rev: Vec<_> = active.iter().filter(|l| l.id == parent).collect();
        assert_eq!(
            rev.len(),
            1,
            "反手候选同 id 重开不经 restore ⟹ 种子过滤不误伤（ID-3 反手保留）；实得 {active:?}"
        );
        assert_eq!(rev[0].dir, VoiceSide::Short, "反手腿方向取候选（反向），非被关父的 Long");
        assert_eq!(rev[0].source_index, 55, "反手腿坐标取候选点元素，非被关父的 30");
    }

    /// ★#226 种子在祖先链**中段**：restore 逐级走查遇种子即中断——已恢复的下级祖先链断于种子
    /// （其 parent_id 解析不到），与候选一并未通过统一 AncOK（链断则后代链同断，剪除单调）。
    #[test]
    fn open_parent_restore_mid_chain_seed_aborts() {
        let child = eid(0, 900);
        let p1 = eid(1, 901);
        let p2 = eid(2, 902);
        // P1/P2 仅 registry LiveDetached（空树）；P1.structural_parent=P2，P2=根。
        let p1_elem = CoverageElement {
            lambda: 10, rho: 20, eps: VoiceSide::Long, level: 1,
            parent: None, attached_dir: None, id: p1, parent_id: Some(p2),
        };
        let p2_elem = CoverageElement {
            lambda: 10, rho: 30, eps: VoiceSide::Long, level: 2,
            parent: None, attached_dir: None, id: p2, parent_id: None,
        };
        let cand_elem = CoverageElement {
            lambda: 42, rho: 42, eps: VoiceSide::Long, level: 0,
            parent: None, attached_dir: None, id: child, parent_id: Some(p1),
        };
        let leg_p2 = ActiveLeg {
            level: 2, dir: VoiceSide::Long, source_index: 30, lambda: 10,
            id: p2, parent_id: None, is_boundary_root: true, op_parent: None,
        };
        let buckets = Buckets {
            close: vec![leg_p2],
            open: vec![cand(0, 42, VoiceSide::Long, 0)],
            record: vec![],
        };
        let reg = super::super::super::persistent::PersistentRegistry::new()
            .merge(&[p1_elem, p2_elem, cand_elem], &[leg_p2]);
        let (active, _p, _sep, _idx) = coverage_step_from_buckets_sep(
            view_split(&[cand_elem], 0), &[leg_p2], &buckets, 1000.0, &cfg(), None, &reg,
        );
        assert!(
            !active.iter().any(|l| l.id == p2 || l.id == p1 || l.id == child),
            "种子在链中段 ⟹ 中断后下级祖先随候选一并 AncOK 剪除；实得 {active:?}"
        );
    }

    /// ★#233 **父翻向 = 父终结**（#227 裁决，蓝图两步形①）：载体方向被结构树改判（#269 事件
    /// 口径：registry 首见方向 ≠ 当前树元素方向）⟹
    /// ① 父声部关闭（旧世代退出活动集，close 事件经 silent_drops 入轨）+ 子树连带清仓
    /// （𝒟_x^† 现成机制——父不在 A 的后代数济判据，exit.rs `subtree_close`）+ **frontier
    /// 翻向同按终结**（焊缝规则：守卫不区分 frontier/confirmed，凡翻向**事件**即终结——旧
    /// #233 表述「凡树元素方向 ≠ 持仓腿方向即终结」是已被替换的状态轴判据，见 #269）；
    /// **方向盲 ID 对位复活路径废除**——翻向事件在场处旧世代不得复活。
    ///
    /// **RED（方向盲对位）**：父持仓腿 Long × 树元素同 id Short ⟹ `held_leg_tree_index_indexed`
    /// 按 id 命中即 Exact（不比方向）+ `element_as_leg` 静默采用树新方向——父不死、方向突变，
    /// 子树照常存活（leg-2-98 勘察 §3.2 实证形态：父 (3,22) 两次翻向零关闭事件，子腿 (2,98)
    /// 存活 10,944 bar）。**GREEN（翻向终结）**：父腿旧世代终结（不当 Exact 对位、不得复活），
    /// Exact 存活子腿沿 parent_id 链命中翻向种子连清——next_active 无父无子，探针为凭。
    ///
    /// 定义依据：anc.pdf §7 I2（同一持久元素方向不变）；ADR 0001 S1（σ 是身份成分）/S3
    /// （父终结连清子树）；voice-direction-flip-doctrine-20260724 §Q6（机制链同构裁定）。
    #[test]
    fn parent_direction_flip_terminates_voice_and_liquidates_subtree() {
        let parent = eid(1, 901);
        let child = eid(0, 900);
        // 树（翻向后）：父元素同 id eps=Short（frontier 重组翻向）；子元素 eps=Short（未翻，
        // 与持仓子腿方向一致 ⟹ 子腿 Exact 存活面——专验「父翻向连清」而非「子自身翻向」）。
        let tree_parent = CoverageElement {
            lambda: 20, rho: 30, eps: VoiceSide::Short, level: 1,
            parent: None, attached_dir: None, id: parent, parent_id: None,
        };
        let tree_child = CoverageElement {
            lambda: 25, rho: 28, eps: VoiceSide::Short, level: 0,
            parent: None, attached_dir: None, id: child, parent_id: Some(parent),
        };
        // 持仓：父腿 Long（旧世代方向）+ 子腿 Short（自身方向未翻 ⟹ Exact 对位存活，
        // 其死法只能是父翻向连清——隔离出本票机制）。
        let leg_parent = ActiveLeg {
            level: 1, dir: VoiceSide::Long, source_index: 30, lambda: 20,
            id: parent, parent_id: None, is_boundary_root: true, op_parent: None,
        };
        let leg_child = ActiveLeg {
            level: 0, dir: VoiceSide::Short, source_index: 28, lambda: 25,
            id: child, parent_id: Some(parent), is_boundary_root: false, op_parent: Some(parent),
        };
        let prev = [leg_child, leg_parent];
        let buckets = Buckets { close: vec![], open: vec![], record: vec![] };
        // registry = 翻向前状态（父 Long 首见方向，I2 守卫下永固）。
        let pre_flip = [CoverageElement {
            lambda: 20, rho: 29, eps: VoiceSide::Long, level: 1,
            parent: None, attached_dir: None, id: parent, parent_id: None,
        }];
        let reg = super::super::super::persistent::PersistentRegistry::new().merge(&pre_flip, &prev);
        ancok_probe_reset();
        let (active, _p) = coverage_step_from_buckets(
            view_split(&[tree_parent, tree_child], 2), &prev, &buckets, 1000.0, &cfg(), None, &reg,
        );
        assert_eq!(
            ancok_probe_snapshot().held_flip_terminated,
            1,
            "父翻向检测命中一次（方向守卫探针为凭）"
        );
        assert!(
            !active.iter().any(|l| l.id == parent),
            "父翻向 = 父终结：旧世代父腿不得存活/复活（方向盲对位废除）；实得 {active:?}"
        );
        assert!(
            !active.iter().any(|l| l.id == child),
            "父翻向 ⟹ 子树连带清仓（𝒟_x^† 数济判据：父不在 A 的后代连坐）；实得 {active:?}"
        );
    }

    /// ★#233 翻向终结的 **close 事件入轨**（蓝图两步形①「父声部关闭（close 事件入轨）」）：
    /// 翻向父腿与连清子腿经 `StepTrace.silent_drops` 外化——runner 既有消费链（typed ledger
    /// `via_structural_prune` + 账户镜像 `StructuralPrune` + TW 腿计数）自动兑现入轨，
    /// **不新造外化轨**；closed/opened 桶不染（翻向是结构事件，非信号裁决，无触发候选）。
    ///
    /// **RED**：方向盲对位下父子全存活 ⟹ silent_drops 空。**GREEN**：父子双双入轨。
    #[test]
    fn flip_termination_externalizes_close_track_via_silent_drops() {
        let parent = eid(1, 901);
        let child = eid(0, 900);
        let tree = vec![
            CoverageElement {
                lambda: 20, rho: 30, eps: VoiceSide::Short, level: 1,
                parent: None, attached_dir: None, id: parent, parent_id: None,
            },
            CoverageElement {
                lambda: 25, rho: 28, eps: VoiceSide::Short, level: 0,
                parent: None, attached_dir: None, id: child, parent_id: Some(parent),
            },
        ];
        let leg_parent = ActiveLeg {
            level: 1, dir: VoiceSide::Long, source_index: 30, lambda: 20,
            id: parent, parent_id: None, is_boundary_root: true, op_parent: None,
        };
        let leg_child = ActiveLeg {
            level: 0, dir: VoiceSide::Short, source_index: 28, lambda: 25,
            id: child, parent_id: Some(parent), is_boundary_root: false, op_parent: Some(parent),
        };
        let prev = [leg_child, leg_parent];
        let pre_flip = [CoverageElement {
            lambda: 20, rho: 29, eps: VoiceSide::Long, level: 1,
            parent: None, attached_dir: None, id: parent, parent_id: None,
        }];
        let reg = super::super::super::persistent::PersistentRegistry::new().merge(&pre_flip, &prev);
        let r = rcfg();
        let w = PiThetaWeights::from_risk(&r);
        ancok_probe_reset();
        let (na, _ps, _o, trace) = pi_theta_step_traced(
            ElementView::from_parts(&tree, vec![]), &[], &prev, 0.0, 5, 1000.0,
            &r, w, KThetaRiskGate::open(), &cfg(), &reg, None, &protocol_hold(),
        );
        assert_eq!(ancok_probe_snapshot().held_flip_terminated, 1, "翻向检测命中");
        assert!(na.is_empty(), "父终结 + 子树连清 ⟹ A_{{t+1}} 空；实得 {na:?}");
        let drops: Vec<ElementId> = trace.silent_drops.iter().map(|l| l.id).collect();
        assert!(
            drops.contains(&parent) && drops.contains(&child),
            "翻向父 + 连清子经 silent_drops 入轨（runner 既有消费链兑现 close 事件）；实得 {drops:?}"
        );
        assert!(
            trace.closed.is_empty(),
            "翻向非信号裁决（无触发候选）——closed 桶不染；实得 {:?}",
            trace.closed.iter().map(|(l, _, _)| l.id).collect::<Vec<_>>()
        );
        assert!(trace.opened.is_empty(), "无开仓——opened 桶不染");
    }

    /// ★#233 蓝图两步形②：**新世代声部重登记**（新 posId/generation）——父翻向同 bar 的
    /// ℬ_x 开仓候选（挂该父）**不被翻向种子连坐**（ℬ_x 段不经 𝒟_x^†，#183 分段/ID-3 反手
    /// 保护同构）；父的**新世代元素**（树元素新方向）经 open 父注入 restore 的 id_idx 复用
    /// 在场 ⟹ 候选 AncOK 准入，后续经 #220 idx 配对 opened 外化 ⟹ runner A9 generation+1
    /// 登记（零改动路径见证）。
    ///
    /// 形态 = leg-2-98 勘察 gen-10 出生 bar（父 (3,22) 翻 Short 同 bar buy2 开 Long）最小
    /// 复现。**复合检查**：翻向终结不回退反手/新世代准入（#216/#220/#226 语义兼容）。
    /// 回归锁：除翻向探针（旧版=0）外，准入/在场/方向断言在方向盲旧版同样成立——本测试
    /// 锁的是「终结化修复不误伤新世代登记路径」。
    #[test]
    fn flip_same_bar_new_generation_reregisters_via_open_candidate() {
        let parent = eid(1, 22);
        let child = eid(0, 98);
        // 树（翻向后）：父元素同 id eps=Short。
        let tree_parent = CoverageElement {
            lambda: 20, rho: 30, eps: VoiceSide::Short, level: 1,
            parent: None, attached_dir: None, id: parent, parent_id: None,
        };
        // 候选段：lvl0 开多候选（反父 Short ⟹ ReverseOpen 形态，gen-10 同构），parent_id=翻向父。
        let cand_elem = CoverageElement {
            lambda: 42, rho: 42, eps: VoiceSide::Long, level: 0,
            parent: None, attached_dir: None, id: child, parent_id: Some(parent),
        };
        let leg_parent = ActiveLeg {
            level: 1, dir: VoiceSide::Long, source_index: 30, lambda: 20,
            id: parent, parent_id: None, is_boundary_root: true, op_parent: None,
        };
        let buckets = Buckets {
            close: vec![],
            open: vec![cand(0, 42, VoiceSide::Long, 0)],
            record: vec![],
        };
        // registry = 翻向前状态（父 Long，I2 守卫下永固）——open 父注入的 registry_live 检查
        // 与 restore id_idx 树复用（新世代元素）均依赖此形态。
        let pre_flip = [CoverageElement {
            lambda: 20, rho: 29, eps: VoiceSide::Long, level: 1,
            parent: None, attached_dir: None, id: parent, parent_id: None,
        }];
        let reg = super::super::super::persistent::PersistentRegistry::new().merge(&pre_flip, &[leg_parent]);
        ancok_probe_reset();
        let (active, _p) = coverage_step_from_buckets(
            view_split(&[tree_parent, cand_elem], 1), &[leg_parent], &buckets, 1000.0, &cfg(), None, &reg,
        );
        assert_eq!(
            ancok_probe_snapshot().held_flip_terminated,
            1,
            "父翻向检测命中一次（终结化生效）——旧版方向盲对位下探针=0（RED 锚点）"
        );
        let child_legs: Vec<_> = active.iter().filter(|l| l.id == child).collect();
        assert_eq!(
            child_legs.len(),
            1,
            "ℬ_x 新世代候选不被翻向种子连坐（ℬ_x 段不经 𝒟_x^†）⟹ 准入；实得 {active:?}"
        );
        assert_eq!(child_legs[0].dir, VoiceSide::Long, "新世代候选方向取候选自身（Long）");
        let parent_legs: Vec<_> = active.iter().filter(|l| l.id == parent).collect();
        assert_eq!(
            parent_legs.len(),
            1,
            "父的新世代元素（树复用，dir=Short）在场 ⟹ 候选父链 AncOK 通过；实得 {active:?}"
        );
        assert_eq!(
            parent_legs[0].dir,
            VoiceSide::Short,
            "在场父元素 = 新世代（树元素新方向），非旧世代尸体复活"
        );
    }

    /// ★#233 蓝图两步形②补：**同 carrier 翻向 + 同 bar 反手候选 = 新世代立即重登记**
    /// （新 posId/generation 的最直接形态）——父持仓腿翻向（Long→Short）同 bar，同 id 反手
    /// 候选（Short）开仓：旧世代腿终结（翻向种子，经 silent_drops 入轨），反手候选**不被
    /// 翻向种子连坐**（ℬ_x 段不经 𝒟_x^†，与 #226 `open_reverse_candidate_on_closed_carrier_
    /// unaffected` 的 ID-3 反手保护同构——种子只作用 restore/持仓域，候选推送零改），
    /// 经 #220 idx 配对 opened 外化 ⟹ runner A9 generation+1 新世代登记。
    ///
    /// 与 `flip_same_bar_new_generation_reregisters_via_open_candidate` 的分工：彼验「新世代
    /// 父元素在场供养子候选」，本验「同 id 反手候选自身即新世代声部」。
    #[test]
    fn flip_same_bar_reverse_candidate_reregisters_new_generation_same_carrier() {
        let parent = eid(1, 22);
        // 树（翻向后）：父元素同 id eps=Short。
        let tree_parent = CoverageElement {
            lambda: 20, rho: 30, eps: VoiceSide::Short, level: 1,
            parent: None, attached_dir: None, id: parent, parent_id: None,
        };
        // 反手候选元素：同 carrier id、eps=Short、点元素（lambda==rho）、无父（反向根）。
        let rev_elem = CoverageElement {
            lambda: 55, rho: 55, eps: VoiceSide::Short, level: 1,
            parent: None, attached_dir: None, id: parent, parent_id: None,
        };
        let leg_parent = ActiveLeg {
            level: 1, dir: VoiceSide::Long, source_index: 30, lambda: 20,
            id: parent, parent_id: None, is_boundary_root: true, op_parent: None,
        };
        let buckets = Buckets {
            close: vec![],
            open: vec![cand(1, 55, VoiceSide::Short, 0)],
            record: vec![],
        };
        let pre_flip = [CoverageElement {
            lambda: 20, rho: 29, eps: VoiceSide::Long, level: 1,
            parent: None, attached_dir: None, id: parent, parent_id: None,
        }];
        let reg = super::super::super::persistent::PersistentRegistry::new().merge(&pre_flip, &[leg_parent]);
        ancok_probe_reset();
        let (active, _p, _sep, _idx) = coverage_step_from_buckets_sep(
            view_split(&[tree_parent, rev_elem], 1), &[leg_parent], &buckets, 1000.0, &cfg(), None, &reg,
        );
        assert_eq!(
            ancok_probe_snapshot().held_flip_terminated,
            1,
            "翻向检测命中一次（旧世代终结）"
        );
        let rev: Vec<_> = active.iter().filter(|l| l.id == parent).collect();
        assert_eq!(
            rev.len(),
            1,
            "同 id 反手候选不被翻向种子连坐（ℬ_x 段不经 𝒟_x^†）⟹ 恰一条新世代腿；实得 {active:?}"
        );
        assert_eq!(rev[0].dir, VoiceSide::Short, "新世代腿方向取反手候选（Short），非旧世代 Long");
        assert_eq!(rev[0].source_index, 55, "新世代腿坐标取候选点元素（55），非旧世代 30");
    }

    /// ★#269 翻向守卫**事件化**两态①：**出生对立无事件 ⟹ 不杀**。
    /// #264 归因：790 笔 1-bar prune 全死于「σ/ε 出生对立被误判为父翻向」——BSP 构造使买点
    /// 恒附下降段末端、卖点恒附上升段末端，σ=−ε 在入场时刻即恒真（出生即存在的两轴对立，
    /// 非事件）。事件口径：载体（registry 首见方向 Short，I2 永固）在腿存活期间**未发生**
    /// 树段方向冲突（当前树元素 eps 恒 == 首见方向）⟹ 无翻向事件 ⟹ 腿 Exact 存活并随载体
    /// 结构方向重登记（#233 前基线语义恢复）。
    ///
    /// **RED（#233 状态轴守卫）**：tree.eps(Short) ≠ leg.dir(Long) ⟹ Flipped 误杀（t+1 必剪，
    /// 探针=1、腿不入 next_active）。**GREEN（事件化）**：无事件 ⟹ 存活，探针=0。
    #[test]
    fn birth_opposition_without_flip_event_does_not_terminate() {
        let carrier = eid(0, 900);
        // 树：载体元素 eps=Short（下降段 = 买点载体），与出生 bar 逐位一致（无重组/无翻向）。
        let tree_carrier = CoverageElement {
            lambda: 20, rho: 30, eps: VoiceSide::Short, level: 0,
            parent: None, attached_dir: None, id: carrier, parent_id: None,
        };
        // 持仓腿：买点候选出生 σ=Long（出生对立 σ=−ε 恒真，BSP 构造不变量，#264 §2.1）。
        let leg = ActiveLeg {
            level: 0, dir: VoiceSide::Long, source_index: 30, lambda: 30,
            id: carrier, parent_id: None, is_boundary_root: true, op_parent: None,
        };
        let prev = [leg];
        let buckets = Buckets { close: vec![], open: vec![], record: vec![] };
        // registry：载体首见方向 Short（树段 upsert，I2 永固）——腿存活期间零方向冲突事件。
        let reg = super::super::super::persistent::PersistentRegistry::new().merge(&[tree_carrier], &prev);
        ancok_probe_reset();
        let (active, _p, _sep, _idx) = coverage_step_from_buckets_sep(
            view_split(&[tree_carrier], 1), &prev, &buckets, 1000.0, &cfg(), None, &reg,
        );
        assert_eq!(
            ancok_probe_snapshot().held_flip_terminated,
            0,
            "出生对立非翻向事件——守卫不得开火（#233 状态轴守卫下=1，RED 锚点）"
        );
        let legs: Vec<_> = active.iter().filter(|l| l.id == carrier).collect();
        assert_eq!(
            legs.len(),
            1,
            "无翻向事件 ⟹ 腿 Exact 存活（对位回载体元素）；实得 {active:?}"
        );
        assert_eq!(
            legs[0].dir,
            VoiceSide::Short,
            "存活腿随载体结构方向重登记（ε=Short，基线语义），非出生信号 σ=Long"
        );
    }

    /// ★#269 翻向守卫**事件化**两态②：**存活期真翻向事件 ⟹ 杀**——即使当前树元素方向与
    /// 腿 σ **一致**（无 σ/ε 状态对立）。形态 = #264 未能判定②幸存笔 (b) 态「id 重指同向
    /// 元素」/leg-2-98 (3,22) frontier 重组翻向（PREG Short→Long 单行，无候选参与）：载体
    /// 首见方向 Long（registry I2 永固），存活期树把同 id 元素改判 Short ⟹ 树段 upsert 方向
    /// 冲突 = 真实结构翻向**事件** ⟹ 父翻向=父终结（#227 裁决、#233 已结算条款不倒退）。
    ///
    /// **RED（#233 状态轴守卫）**：tree.eps(Short) == leg.dir(Short) ⟹ Exact 放行——状态轴
    /// 读不出「载体从 Long 翻成 Short」（事件盲区，幸存笔 (b) 态豁免机制）。**GREEN**：杀。
    #[test]
    fn carrier_flip_event_terminates_even_without_direction_opposition() {
        let carrier = eid(1, 22);
        // 树（当前 bar）：载体同 id 元素已被 frontier 重组改判为 Short。
        let tree_carrier = CoverageElement {
            lambda: 20, rho: 30, eps: VoiceSide::Short, level: 1,
            parent: None, attached_dir: None, id: carrier, parent_id: None,
        };
        // 持仓腿：卖点候选出生 σ=Short（出生时载体为 Long——出生对立 σ=−ε 同构）。
        let leg = ActiveLeg {
            level: 1, dir: VoiceSide::Short, source_index: 30, lambda: 20,
            id: carrier, parent_id: None, is_boundary_root: true, op_parent: None,
        };
        let prev = [leg];
        let buckets = Buckets { close: vec![], open: vec![], record: vec![] };
        // registry = 翻向前状态（载体首见方向 Long，I2 守卫下永固）——当前树 eps=Short ≠ 首见
        // Long ⟺ 树段 upsert 方向冲突（翻向事件）在腿存活期间发生。
        let pre_flip = [CoverageElement {
            lambda: 20, rho: 29, eps: VoiceSide::Long, level: 1,
            parent: None, attached_dir: None, id: carrier, parent_id: None,
        }];
        let reg = super::super::super::persistent::PersistentRegistry::new().merge(&pre_flip, &prev);
        ancok_probe_reset();
        let (active, _p, _sep, _idx) = coverage_step_from_buckets_sep(
            view_split(&[tree_carrier], 1), &prev, &buckets, 1000.0, &cfg(), None, &reg,
        );
        assert_eq!(
            ancok_probe_snapshot().held_flip_terminated,
            1,
            "载体真翻向事件 ⟹ 守卫开火一次（#233 状态轴守卫下 tree.eps==leg.dir 放行=0，RED 锚点）"
        );
        assert!(
            !active.iter().any(|l| l.id == carrier),
            "翻向事件 ⟹ 旧世代腿终结（父翻向=父终结不倒退）；实得 {active:?}"
        );
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
        // 空 registry（父 carrier 从未出现在任何 snapshot）+ 仅 L0 ReverseOpen 子卖点 + 空 prev_active。
        let bar = Classification {
            levels: vec![LevelState { bsp: Rc::new(vec![sell_bsp(8)]), ..Default::default() }],
        };
        let (active, p) =
            coverage_step_classification(&bar, &tower, &[], 1000.0, &cfg(), None, &reg);
        assert!(
            active.is_empty(),
            "父 carrier 不在 registry ⟹ open 父注入不恢复 ⟹ 子腿仍剪枝（非膨胀）；实得 {active:?}"
        );
        assert_eq!(p, 0.0, "孤立 ReverseOpen（父不可恢复）剪枝 ⟹ p̃=0");
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
        let _ = p; // p̃ 可非零（若 ReverseOpen 候选准入），关键是 stale_non_root 不在 active。
    }

