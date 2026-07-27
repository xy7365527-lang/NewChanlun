use super::*;
use crate::theta_v0::classifier::recursive_tower::LeveledMove;
use crate::theta_v0::classifier::center::UnitRange;
use crate::theta_v0::types::{Center, Direction, Tick};
use crate::theta_v0::classifier::bsp::BspPoint;
use crate::theta_v0::classifier::LevelState;
use crate::theta_v0::types::BspBits;

    pub(crate) fn cfg() -> VoiceConfig {
        VoiceConfig::default() // max_depth=3, depth_weights=[0.60,0.30,0.10]
    }


    pub(crate) fn protocol_hold() -> ProtocolEventSet {
        ProtocolEventSet::hold(0)
    }


    /// ★O(n) 重构测试适配：把 `Vec<Vec<LeveledMove>>` 字面量塔逐级包 `Rc`（生产塔现为
    /// `Vec<Rc<Vec<LeveledMove>>>`）。bit-exact 无关——仅类型适配，Rc deref 后内容不变。
    pub(crate) fn rc_tower(levels: Vec<Vec<LeveledMove>>) -> Vec<Rc<Vec<LeveledMove>>> {
        levels.into_iter().map(Rc::new).collect()
    }


    /// 测试用 24 类角色构造器（三轴元组）。
    pub(crate) fn role(h: Horizontal, v: Vertical, d: Dir) -> OperationRole {
        OperationRole { h, v, delta: d, grade: GradeRel::SameLevel }
    }

    /// q_Θ v1 σ_higher 升级验收测试套（prereg-rev4 §九 g2 验收点 2/3）。


    pub(crate) fn unit(si: usize, ei: usize, dir: Direction, lo: Tick, hi: Tick) -> UnitRange {
        UnitRange { start_index: si, end_index: ei, direction: dir, lo, hi }
    }


    pub(crate) fn ctr(start: usize, end: usize) -> Center {
        Center { zd: 5, zg: 10, dd: 0, gg: 15, start_index: start, end_index: end }
    }


    /// 构造一个真嵌套 L1 走势（Compose 三段 L0 线段子声部，真父子）。
    /// ★codex Q4：注入确定性 ElementId（L0 子 ordinal=0/1/2，L1 父 ordinal=0）。
    pub(crate) fn nested_l1(si: usize, ei: usize, sub_dirs: [Direction; 3]) -> LeveledMove {
        let s0 = LeveledMove::from_unit(&unit(si, si + 4, sub_dirs[0], 0, 10), ElementId { level: 0, ordinal: 0 });
        let s1 = LeveledMove::from_unit(&unit(si + 4, si + 8, sub_dirs[1], 3, 12), ElementId { level: 0, ordinal: 1 });
        let s2 = LeveledMove::from_unit(&unit(si + 8, ei, sub_dirs[2], 5, 15), ElementId { level: 0, ordinal: 2 });
        LeveledMove::compose(&[s0, s1, s2], ctr(si, ei), 1, ElementId { level: 1, ordinal: 0 })
    }


    /// 测试用 ElementId（默认根级 (0,0)）。
    pub(crate) fn eid(level: u32, ordinal: u64) -> ElementId {
        ElementId { level, ordinal }
    }


    /// 测试用 ActiveLeg 构造器（默认 is_boundary_root=true 真边界根 ∂，parent_id=None）。
    /// 旧测试字面量用此辅助补齐 id/parent_id/is_boundary_root 字段（codex Q4）。
    pub(crate) fn aleg(level: u32, dir: VoiceSide, source_index: usize, lambda: usize) -> ActiveLeg {
        ActiveLeg {
            level,
            dir,
            source_index,
            lambda,
            id: ElementId { level, ordinal: source_index as u64 },
            parent_id: None,
            is_boundary_root: true,
            op_parent: None,
        }
    }

    // ── §1/§2 元素提取（真嵌套父子，铁律守护）─────────────────────────────────


    /// 两父塔：compose_a(Long) + compose_b(Short) 相邻；a2 与 b0 **结构全等**(Seg{Up,5,15})
    /// 但属不同父 + 不同 ρ（漏洞③可观测）。a2.ρ=12==b0.λ=12（共享端点，漏洞②）。
    /// compose_a.ρ=12==a2.ρ=12 跨级共享（漏洞①）。
    pub(crate) fn two_parent_tower() -> Vec<Rc<Vec<LeveledMove>>> {
        let a0 = LeveledMove::from_unit(&unit(0, 4, Direction::Up, 0, 10), eid(0, 0));
        let a1 = LeveledMove::from_unit(&unit(4, 8, Direction::Down, 3, 12), eid(0, 1));
        let a2 = LeveledMove::from_unit(&unit(8, 12, Direction::Up, 5, 15), eid(0, 2));
        let compose_a = LeveledMove::compose(&[a0, a1, a2], ctr(0, 12), 1, eid(1, 0)); // 外缘 10→15 ⟹ Long
        let b0 = LeveledMove::from_unit(&unit(12, 16, Direction::Up, 5, 15), eid(0, 3)); // 结构 == a2
        let b1 = LeveledMove::from_unit(&unit(16, 20, Direction::Down, 3, 12), eid(0, 4));
        let b2 = LeveledMove::from_unit(&unit(20, 24, Direction::Down, 0, 8), eid(0, 5));
        let compose_b = LeveledMove::compose(&[b0, b1, b2], ctr(12, 24), 1, eid(1, 1)); // 外缘 15→8 ⟹ Short
        rc_tower(vec![Vec::new(), vec![compose_a, compose_b]])
    }


    /// ★A12 双视图见证塔：L2 根只收 c2a/c2b，**c1（L1）整棵子树掉出 T_i**（↓r_i 只展开最高级根）
    /// ——c1 及其 L0 subs 是 orphan frontier（676/648 第四根因的最小合成形态）。K_i 全含。
    pub(crate) fn orphan_subtree_tower() -> Vec<Rc<Vec<LeveledMove>>> {
        let s0 = LeveledMove::from_unit(&unit(0, 4, Direction::Up, 0, 10), eid(0, 0));
        let s1 = LeveledMove::from_unit(&unit(4, 8, Direction::Down, 3, 12), eid(0, 1));
        let s2 = LeveledMove::from_unit(&unit(8, 12, Direction::Up, 5, 15), eid(0, 2));
        let c1 = LeveledMove::compose(&[s0, s1, s2], ctr(0, 12), 1, eid(1, 0)); // Up（外缘 15>=10）
        let t0 = LeveledMove::from_unit(&unit(12, 16, Direction::Down, 8, 20), eid(0, 3));
        let t1 = LeveledMove::from_unit(&unit(16, 20, Direction::Up, 10, 25), eid(0, 4));
        let t2 = LeveledMove::from_unit(&unit(20, 24, Direction::Down, 12, 30), eid(0, 5));
        let c2a = LeveledMove::compose(&[t0, t1, t2], ctr(12, 24), 1, eid(1, 1));
        let u0 = LeveledMove::from_unit(&unit(24, 28, Direction::Up, 15, 35), eid(0, 6));
        let u1 = LeveledMove::from_unit(&unit(28, 32, Direction::Down, 18, 40), eid(0, 7));
        let u2 = LeveledMove::from_unit(&unit(32, 36, Direction::Up, 20, 45), eid(0, 8));
        let c2b = LeveledMove::compose(&[u0, u1, u2], ctr(24, 36), 1, eid(1, 2));
        let l2 = LeveledMove::compose(&[c2a.clone(), c2b.clone()], ctr(12, 36), 2, eid(2, 0));
        rc_tower(vec![Vec::new(), vec![c1, c2a, c2b], vec![l2]])
    }


    /// G7 测试用根元素/子元素构造（parent_id 结构映射真值——分组判据与 AncOK 同源）。
    pub(crate) fn gce(level: u32, ordinal: u64, parent_id: Option<ElementId>, eps: VoiceSide) -> CoverageElement {
        CoverageElement {
            lambda: 0,
            rho: 10,
            eps,
            level,
            parent: None,
            attached_dir: None,
            id: ElementId { level, ordinal },
            parent_id,
        }
    }


    pub(crate) fn gleg(e_idx: usize, side: VoiceSide, units: f64) -> LegTarget {
        LegTarget { e_idx, side, units, role: role(Horizontal::First, Vertical::Ambient, Dir::Plus) }
    }


    /// 测试用候选构造器（ℬ_x 开启候选，方向/级别可控）。
    pub(crate) fn cand(level: u32, source_index: usize, dir: VoiceSide, gamma_index: usize) -> Candidate {
        Candidate {
            level,
            source_index,
            bits: BspBits::default(),
            dir,
            bsp_class: 1,
            role: role(Horizontal::First, Vertical::Ambient, Dir::Plus),
            nest_confirmed: true,
            gamma_index,
            force: None,
        }
    }


    /// 测试辅助：ℬ_x 开启候选 → 扁平根元素数组（缺塔/Ambient 路径，`candidate_start=0`）。
    /// `elements[gamma_index]` = 候选根（`parent=None` ⟹ AncOK 恒等准入，与缺塔生产路径一致）。
    /// 按 `gamma_index` 定位（测试 `cand` 用连续 0..n，无父子）。
    pub(crate) fn flat_elements(open: &[Candidate]) -> Vec<CoverageElement> {
        let n = open.iter().map(|c| c.gamma_index + 1).max().unwrap_or(0);
        let mut v: Vec<CoverageElement> = (0..n)
            .map(|i| CoverageElement {
                lambda: 0,
                rho: 0,
                eps: VoiceSide::Flat,
                level: 0,
                parent: None,
                attached_dir: None,
                id: ElementId { level: 0, ordinal: i as u64 },
                parent_id: None,
            })
            .collect();
        for c in open {
            v[c.gamma_index] = CoverageElement {
                lambda: c.source_index,
                rho: c.source_index,
                eps: c.dir,
                level: c.level,
                parent: None,
                attached_dir: None,
                id: ElementId { level: c.level, ordinal: c.source_index as u64 },
                parent_id: None,
            };
        }
        v
    }


    /// ★O(n²) 重构测试适配：把旧 `(elements, candidate_start)` 拆成 ElementView 双段
    /// （base=tree 前缀 elements[..cstart]，overlay=candidate 段 elements[cstart..]）。
    /// bit-exact == 旧合并 Vec：索引语义不变（base 在前 overlay 在后）。
    pub(crate) fn view_split(elements: &[CoverageElement], cstart: usize) -> ElementView<'_> {
        ElementView::from_parts(&elements[..cstart], elements[cstart..].to_vec())
    }


    /// L0 卖买卖点（src=si；host 右端点 ρ=si；pivot 远离 ⟹ 止损不触及）。
    pub(crate) fn sell_bsp(si: usize) -> BspPoint {
        BspPoint { level_origin: 0,
            source_index: si,
            bits: BspBits { sell1: true, ..Default::default() },
            pivot_low: 0,
            pivot_high: 210,
            center: Some(crate::theta_v0::classifier::bsp::OwnerRef::Center(ctr(0, si))),
            struct_break_dir: None,
            force: None,
        }
    }


    /// L0 买买卖点（src=si）。
    pub(crate) fn buy_bsp(si: usize) -> BspPoint {
        BspPoint { level_origin: 0,
            source_index: si,
            bits: BspBits { buy1: true, ..Default::default() },
            pivot_low: 90,
            pivot_high: 0,
            center: Some(crate::theta_v0::classifier::bsp::OwnerRef::Center(ctr(0, si))),
            struct_break_dir: None,
            force: None,
        }
    }


    pub(crate) fn rcfg() -> RiskConfig {
        RiskConfig::default() // rho=0.005, beta=0.5, gamma=1.0, kappa=2.0, default_lot=1
    }

    // ── ★M4 级别级风险帽：level_cap / clamp_levels_to_weighted_cap ──────────


    pub(crate) fn buy_gamma() -> (Classification, Vec<Rc<Vec<LeveledMove>>>) {
        let buy = BspPoint { level_origin: 0,
            source_index: 4,
            bits: BspBits { buy1: true, ..Default::default() },
            pivot_low: 90,
            pivot_high: 0,
            center: Some(crate::theta_v0::classifier::bsp::OwnerRef::Center(Center { zd: 100, zg: 200, dd: 90, gg: 210, start_index: 0, end_index: 9 })),
            struct_break_dir: None,
            force: None,
        };
        (
            Classification { levels: vec![LevelState { bsp: Rc::new(vec![buy]), ..Default::default() }] },
            rc_tower(vec![]),
        )
    }


