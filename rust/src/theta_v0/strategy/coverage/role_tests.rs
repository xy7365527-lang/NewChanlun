use super::*;
use super::super::test_support::*;
use crate::theta_v0::types::Direction;

    /// ★GPT 命名冲突裁决（2026-07-05）：V 轴三分类 living authority + G 轴独立细化。
    /// δ_g=−σ_p（反父方向）无论 ℓ_g vs ℓ_p 均归 V::ReverseOpen（商映射 AgainstParent）；
    /// 级别区分在 G 轴：ℓ_g=ℓ_p→SameLevel（阶段3 SameReverse_3），ℓ_g<ℓ_p→SubLevel（阶段3 ShortDiff_3）。
    #[test]
    fn vertical_three_class_plus_grade_axis_refinement() {
        // parent（idx0）：level=1, σ_p=+1（Long）。
        let elements = vec![
            CoverageElement { lambda: 0, rho: 12, eps: VoiceSide::Long, level: 1, parent: None, attached_dir: None, id: eid(1, 0), parent_id: None },
            // idx1 同级别反父：ℓ_g=1=ℓ_p=1, δ_g=−1=−σ_p ⟹ V=ReverseOpen ∧ G=SameLevel（阶段3 SameReverse_3）。
            CoverageElement { lambda: 0, rho: 4, eps: VoiceSide::Short, level: 1, parent: Some(0), attached_dir: Some(VoiceSide::Long), id: eid(1, 1), parent_id: Some(eid(1, 0)) },
            // idx2 次级别反父：ℓ_g=0<ℓ_p=1, δ_g=−1=−σ_p ⟹ V=ReverseOpen ∧ G=SubLevel（阶段3 ShortDiff_3）。
            CoverageElement { lambda: 4, rho: 8, eps: VoiceSide::Short, level: 0, parent: Some(0), attached_dir: Some(VoiceSide::Long), id: eid(0, 0), parent_id: Some(eid(1, 0)) },
        ];
        // V 三分类：δ=−σ_p 无论级别都归 ReverseOpen（GPT AgainstParent 商映射，§二）。
        assert_eq!(vertical_relation(&elements, 1), Vertical::ReverseOpen,
            "同级别反父 δ=−σ_p ⟹ V=ReverseOpen（GPT 商映射，不分级别）");
        assert_eq!(vertical_relation(&elements, 2), Vertical::ReverseOpen,
            "次级别反父 δ=−σ_p ⟹ V=ReverseOpen（GPT 商映射，不分级别）");
        // G 轴细化：同级别 vs 次级别在独立 G 轴区分（GPT §四）。
        assert_eq!(grade_relation(&elements, 1), GradeRel::SameLevel,
            "ℓ_g=ℓ_p ⟹ G=SameLevel（阶段3 SameReverse_3 派生）");
        assert_eq!(grade_relation(&elements, 2), GradeRel::SubLevel,
            "ℓ_g<ℓ_p ⟹ G=SubLevel（阶段3 ShortDiff_3 派生）");
        // 派生谓词：阶段3 角色经 (V,G) 恢复（非 canonical，GPT 定理2）。
        assert!(operation_role(&elements, 1).is_same_level_against_parent(),
            "V=ReverseOpen ∧ G=SameLevel = 阶段3 SameReverse_3 派生角色");
        assert!(operation_role(&elements, 2).is_sub_level_reverse_open(),
            "V=ReverseOpen ∧ G=SubLevel = 阶段3 ShortDiff_3 派生角色");
    }

    /// ★去根化：顶层元素（parent=None=边界胚元 ∂）= (First, Ambient, δ)——**无 RootRole**。
    #[test]
    fn role_toplevel_is_first_ambient_not_root() {
        let l1 = nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up]);
        let tower = rc_tower(vec![Vec::new(), vec![l1]]);
        let elements = extract_elements(&tower);
        let r = operation_role(&elements, 0);
        // 顶层根（唯一同级元素）：H=First（无前兄弟），V=Ambient（父胚元 σ=0），δ=Plus（外缘上移 Long）。
        assert_eq!(r.h, Horizontal::First);
        assert_eq!(r.v, Vertical::Ambient, "去根化：顶层父=胚元 σ=0 → Ambient（非 RootRole）");
        assert_eq!(r.delta, Dir::Plus);
    }

    /// ★垂直轴 V：次级别顺父子 ⟹ FollowParent（δ_g=σ_{p(g)}）；反父子 ⟹ ReverseOpen（δ_g=−σ_{p(g)}）。
    #[test]
    fn vertical_followparent_vs_reverse_open_by_parent_direction() {
        // 根 L1 走势外缘上移 ⟹ ε_root=Long（σ_{p(g)}=+1 for 子）。
        // 子0=Up(Long)=顺父 ⟹ FollowParent；子1=Down(Short)=反父 ⟹ ReverseOpen。
        let l1 = nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up]);
        let tower = rc_tower(vec![Vec::new(), vec![l1]]);
        let elements = extract_elements(&tower);
        // 根方向（外缘 hi：首子 hi=10, 末子 hi=15 ⟹ 上移 ⟹ Long）。
        assert_eq!(elements[0].eps, VoiceSide::Long);
        // 子0（Up=Long）顺父（+1）⟹ V=FollowParent。
        assert_eq!(vertical_relation(&elements, 1), Vertical::FollowParent);
        // 子1（Down=Short）反父（+1）⟹ V=ReverseOpen（短差，反向子声部）。
        assert_eq!(vertical_relation(&elements, 2), Vertical::ReverseOpen);
        // H 轴：子0 无前同级兄弟 ⟹ First；子1 前兄弟=子0(Long,+1)，δ=−1 ⟹ SameReverse。
        assert_eq!(horizontal_relation(&elements, 1), Horizontal::First);
        assert_eq!(horizontal_relation(&elements, 2), Horizontal::SameReverse);
        // 完整角色元组（子1）：(SameReverse, ReverseOpen, Minus, SubLevel)——子1 level 0 < 父 level 1 ⟹ G=SubLevel。
        assert_eq!(
            operation_role(&elements, 2),
            OperationRole { h: Horizontal::SameReverse, v: Vertical::ReverseOpen, delta: Dir::Minus, grade: GradeRel::SubLevel }
        );
    }

    /// ★水平轴 H（同级别兄弟）：同一父容器下前兄弟 ⟹ SameFollow（同向）/SameReverse（反向）。
    #[test]
    fn horizontal_sibling_follow_vs_reverse() {
        // 三个同级别兄弟（同父=边界胚元 ∂ parent=None，同 level=1）——去根化下顶层亦是兄弟。
        let elements = vec![
            CoverageElement { lambda: 0, rho: 4, eps: VoiceSide::Long, level: 1, parent: None, attached_dir: None, id: eid(1, 0), parent_id: None },
            CoverageElement { lambda: 4, rho: 8, eps: VoiceSide::Long, level: 1, parent: None, attached_dir: None, id: eid(1, 1), parent_id: None },
            CoverageElement { lambda: 8, rho: 12, eps: VoiceSide::Short, level: 1, parent: None, attached_dir: None, id: eid(1, 2), parent_id: None },
        ];
        // idx0：无前兄弟 ⟹ First。
        assert_eq!(horizontal_relation(&elements, 0), Horizontal::First);
        // idx1：前兄弟 idx0(Long,+1)，δ=+1 ⟹ SameFollow。
        assert_eq!(horizontal_relation(&elements, 1), Horizontal::SameFollow);
        // idx2：前兄弟 idx1(Long,+1)，δ=−1 ⟹ SameReverse。
        assert_eq!(horizontal_relation(&elements, 2), Horizontal::SameReverse);
        // 三者 V 均 Ambient（父胚元 σ=0，去根化无 RootRole）。
        assert!((0..elements.len()).all(|i| vertical_relation(&elements, i) == Vertical::Ambient));
    }

    // ── §5 LegTarget（方向 ε_e + role/depth 权重单位数）──────────────────────────

    /// ★工位 4c bit-exact 守卫：`operation_role_indexed_split`（双段 tree+candidate 兄弟索引）
    /// == `operation_role_indexed`（旧单合并 sibling_idx）。LCG 压力构造多 (parent,level,eps,cstart)
    /// 配置，逐 candidate 元素断言 role 三轴逐字段相等。覆盖前兄弟在 tree 段 / candidate 段 / 无前兄弟
    /// 三种分支（split 的 cand-overlay 优先 + tree-fallback last() 路径）。
    #[test]
    fn operation_role_split_matches_merged_lcg() {
        let mut seed = 0x4c_u64;
        let mut next = || { seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); (seed >> 33) as usize };
        for _ in 0..200 {
            let tree_n = next() % 8;       // tree 段元素数 0..7
            let cand_n = 1 + next() % 6;   // candidate 段 1..6（至少 1 个被测）
            let n = tree_n + cand_n;
            // 构造 elements：parent 指向更早的 tree idx（或 None），level 小集合、eps 双向。
            let elems: Vec<CoverageElement> = (0..n).map(|i| {
                let parent = if tree_n > 0 && next() % 2 == 0 { Some(next() % tree_n.max(1)) } else { None };
                CoverageElement {
                    lambda: i, rho: i,
                    eps: if next() % 2 == 0 { VoiceSide::Long } else { VoiceSide::Short },
                    level: (next() % 3) as u32,
                    parent: parent.filter(|&p| p < i), // parent 必在自身之前（树前序）
                    attached_dir: if next() % 2 == 0 { None } else { Some(VoiceSide::Long) },
                    id: ElementId { level: 0, ordinal: i as u64 },
                    parent_id: None,
                }
            }).collect();
            let cstart = tree_n;
            // 旧路径：合并 sibling_idx（全 elements）。
            let merged = build_prev_sibling_index(&elems);
            // 新路径：tree-only + candidate-only 双段。
            let tree_sib = build_prev_sibling_index(&elems[..cstart]);
            let mut cand_sib: std::collections::HashMap<(Option<usize>, u32), Vec<usize>> = std::collections::HashMap::new();
            for ci in cstart..n {
                cand_sib.entry((elems[ci].parent, elems[ci].level)).or_default().push(ci);
            }
            let view = ElementView::from_parts(&elems[..cstart], elems[cstart..].to_vec());
            for ci in cstart..n {
                let old = operation_role_indexed(&view, ci, &merged);
                let new = operation_role_indexed_split(&view, ci, &tree_sib, &cand_sib);
                assert_eq!(old, new, "split≠merged @ci={ci} cstart={cstart} n={n}");
            }
        }
    }

