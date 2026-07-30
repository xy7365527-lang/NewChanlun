use super::super::super::interp::ActiveLeg;
use super::super::test_support::*;
use super::*;
use crate::theta_v0::classifier::recursive_tower::LeveledMove;
use crate::theta_v0::classifier::LevelState;
use crate::theta_v0::types::Direction;

/// ★祖先闭合裁掉祖先不齐者：子元素激活但父不在 raw ⟹ 被 AncOK 裁掉（覆盖不漂浮）。
#[test]
fn ancestor_close_prunes_orphan_child() {
    let l1 = nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up]);
    let tower = rc_tower(vec![Vec::new(), vec![l1]]);
    let elements = extract_elements(&tower);
    // 人为构造：raw 只含子1（idx=2），不含其父（根 idx=0）⟹ AncOK 裁掉子1（祖先不齐）。
    let active = active_set_step(&elements, &[2], 99); // t=99 无新开始/结束，raw=active∖∅
                                                       // 子1 的祖先（根 0）不在 raw{2} ⟹ 被裁 ⟹ 空活动集。
    assert!(active.is_empty(), "孤儿子元素（父不在场）被祖先闭合裁掉");
}

/// ★祖先齐全则保留：根 + 子同在 raw ⟹ AncOK 都保留（覆盖闭合）。
#[test]
fn ancestor_close_keeps_closed_family() {
    let l1 = nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up]);
    let tower = rc_tower(vec![Vec::new(), vec![l1]]);
    let elements = extract_elements(&tower);
    // raw 含根（0）+ 子0（1）：子0 祖先=根0 在 raw ⟹ 都保留。
    let active = active_set_step(&elements, &[0, 1], 99);
    assert!(active.contains(&0) && active.contains(&1));
}

/// 先关后开：D_t 中的元素被关闭（不在 A_{t+1}），B_t 中的被开启。
#[test]
fn raw_close_then_open() {
    let l1 = nested_l1(0, 12, [Direction::Up, Direction::Down, Direction::Up]);
    let tower = rc_tower(vec![Vec::new(), vec![l1]]);
    let elements = extract_elements(&tower);
    // t=4：子0（ρ=4）结束被关，子1（λ=4）开始被开。active 初始含根+子0。
    let active = active_set_step(&elements, &[0, 1], 4);
    // 子0（idx=1，ρ=4∈D_t）被关闭，不在 A_{t+1}。
    assert!(!active.contains(&1), "结束元素 D_t 被关闭");
    // 子1（idx=2，λ=4∈B_t）被开启（其祖先根0在 active）。
    assert!(active.contains(&2), "开始元素 B_t 被开启");
    // 根（idx=0，ρ=12≠4）保持。
    assert!(active.contains(&0), "未结束的根保持");
}

// ── §4 操作角色 R(g)=(H,V,δ) 24 类（H 兄弟轴 / V 父轴 / δ 方向，去根化）──────────

/// ★测试①（AncOK 准入）：有向父容器 + **已持父仓** + 逆向次级 ReverseOpen 候选 ⟹ ReverseOpen 子腿
/// **准入**（祖先齐全：父容器腿在 A_t）。
#[test]
fn ancok_admits_reverse_open_when_parent_held() {
    use super::super::super::interp::{
        assemble_gamma_with_tower, coverage_elements_with_tower, interpret,
    };
    // per-bar 因果塔：L1 Long 父走势（compose idx0）+ 3 L0 子（idx1/2/3，sub(4,8) ρ=8）。
    let tower = rc_tower(vec![
        Vec::new(),
        vec![nested_l1(
            0,
            12,
            [Direction::Up, Direction::Down, Direction::Up],
        )],
    ]);
    // L0 卖候选 src=8 ⟹ host=sub(4,8) ⟹ 真父 L1 Long ⟹ ReverseOpen（δ=Short=−σ_p）。
    let classification = Classification {
        levels: vec![LevelState {
            bsp: Rc::new(vec![sell_bsp(8)]),
            ..Default::default()
        }],
    };
    let (elements, cstart) = coverage_elements_with_tower(&classification, &tower);
    let gamma = assemble_gamma_with_tower(&classification, &tower);
    assert_eq!(
        gamma[0].role.v,
        Vertical::ReverseOpen,
        "前置：候选 V=ReverseOpen（639 σ_p=父容器方向）"
    );
    // 持父仓：prev_active 含父容器腿（L1 Long，ID=(1,0) 与塔 compose 元素同 ID，source_index=ρ=12）。
    // ★codex Q4：held_leg_tree_index 按 ElementId 匹配——leg.id 必须与塔元素 id 一致。
    let held_parent = ActiveLeg {
        level: 1,
        dir: VoiceSide::Long,
        source_index: 12,
        lambda: 0,
        id: eid(1, 0),
        parent_id: None,
        is_boundary_root: true,
        op_parent: None,
    };
    let buckets = interpret(&gamma, &[held_parent]);
    let reg = super::super::super::persistent::PersistentRegistry::new();
    let (active, _p) = coverage_step_from_buckets(
        view_split(&elements, cstart),
        &[held_parent],
        &buckets,
        1000.0,
        &cfg(),
        None,
        &reg,
    );
    assert!(
        active
            .iter()
            .any(|l| l.level == 0 && l.dir == VoiceSide::Short),
        "持父仓 ⟹ ReverseOpen 子腿准入（AncOK 祖先齐全）；实得 {active:?}"
    );
    assert!(
        active
            .iter()
            .any(|l| l.level == 1 && l.dir == VoiceSide::Long),
        "父容器腿保留"
    );
}

/// ★测试①b（持久身份 coord_drift，codex Q4 假阴性消除）：父走势**延伸 end_index 变但同一父**
/// （吸收更多次级别子走势 ⟹ ρ 漂移）+ 已持父仓 + 逆向次级 ReverseOpen 候选 ⟹ ReverseOpen 子腿
/// **仍准入**（持久身份 λ 稳定对位识别为 coord_drift，非误判 stale 剪枝）。
///
/// 旧逻辑（仅 ρ 精确匹配）：持仓父腿 ρ 漂移 ⟹ `held_leg_tree_index` 误判 None ⟹ 静默降 orphan ⟹
/// 子腿父不在 raw ⟹ AncOK 误剪 ReverseOpen（**假阴性**）。本测试断言修正后不再剪。
#[test]
fn ancok_admits_reverse_open_under_parent_coord_drift() {
    use super::super::super::interp::{
        assemble_gamma_with_tower, coverage_elements_with_tower, interpret,
    };
    // 当前因果塔：L1 Long 父走势**已延伸**到 ρ=12（持仓时旧 ρ 曾=8，现吸收更多子走势 ρ 漂移）。
    let tower = rc_tower(vec![
        Vec::new(),
        vec![nested_l1(
            0,
            12,
            [Direction::Up, Direction::Down, Direction::Up],
        )],
    ]);
    let classification = Classification {
        levels: vec![LevelState {
            bsp: Rc::new(vec![sell_bsp(8)]),
            ..Default::default()
        }],
    };
    let (elements, cstart) = coverage_elements_with_tower(&classification, &tower);
    let gamma = assemble_gamma_with_tower(&classification, &tower);
    assert_eq!(
        gamma[0].role.v,
        Vertical::ReverseOpen,
        "前置：候选 V=ReverseOpen（639 σ_p=父容器方向）"
    );
    // ★codex Q4 发现 B 归因修正：旧注释"ρ 漂移"归因错误——λ=start_index 不变，真因是值比较非
    // spec §13 结构映射。Q4 修复：按确定性 ElementId 匹配——leg.id=(1,0) 与塔 L1 父元素同 ID
    //（父延伸 ρ 8→12 不变 ID）⟹ Exact 命中 ⟹ 延伸父仍在 raw ⟹ ReverseOpen 子腿准入。
    let held_parent = ActiveLeg {
        level: 1,
        dir: VoiceSide::Long,
        source_index: 8,
        lambda: 0,
        id: eid(1, 0),
        parent_id: None,
        is_boundary_root: true,
        op_parent: None,
    };
    let buckets = interpret(&gamma, &[held_parent]);
    let reg = super::super::super::persistent::PersistentRegistry::new();
    let (active, _p) = coverage_step_from_buckets(
        view_split(&elements, cstart),
        &[held_parent],
        &buckets,
        1000.0,
        &cfg(),
        None,
        &reg,
    );
    assert!(
            active.iter().any(|l| l.level == 0 && l.dir == VoiceSide::Short),
            "ID 匹配（确定性 ElementId）⟹ 延伸父仍在 raw ⟹ ReverseOpen 子腿准入（Q4 假阴性消除）；实得 {active:?}"
        );
    let reg = super::super::super::persistent::PersistentRegistry::new();
    assert!(
        active
            .iter()
            .any(|l| l.level == 1 && l.dir == VoiceSide::Long),
        "延伸父容器腿按 ID 对位回当前树元素并保留（非降 orphan）"
    );
}

/// ★测试②（AncOK 剪枝，639(c) 关键测试）：有向父容器 + **未持父仓** + 逆向次级 ReverseOpen 候选 ⟹
/// ReverseOpen 子腿**被剔除**（不开 naked 逆势仓 garbage trade）。这是 639(c) 承诺的兑现。
#[test]
fn ancok_prunes_reverse_open_when_parent_unheld() {
    use super::super::super::interp::{
        assemble_gamma_with_tower, coverage_elements_with_tower, interpret,
    };
    let tower = rc_tower(vec![
        Vec::new(),
        vec![nested_l1(
            0,
            12,
            [Direction::Up, Direction::Down, Direction::Up],
        )],
    ]);
    let classification = Classification {
        levels: vec![LevelState {
            bsp: Rc::new(vec![sell_bsp(8)]),
            ..Default::default()
        }],
    };
    let (elements, cstart) = coverage_elements_with_tower(&classification, &tower);
    let gamma = assemble_gamma_with_tower(&classification, &tower);
    assert_eq!(
        gamma[0].role.v,
        Vertical::ReverseOpen,
        "前置：候选 V=ReverseOpen（σ 来源 ⊥ 持仓）"
    );
    // 未持父仓：空 prev_active。
    let buckets = interpret(&gamma, &[]);
    // ★Q3 加固（codex）：坐实剪枝**前**候选确实进 open 桶 + 携非空父指针——否则
    // active.is_empty() 在「候选从未入场」时也假性通过（interpret 行为变后防假阳性）。
    assert_eq!(
        buckets.open.len(),
        1,
        "interpret 确实把 ReverseOpen 候选放入 open 桶（非从未入场）"
    );
    assert!(
        matches!(elements[cstart].parent, Some(_)),
        "候选携非空真 Compose 父指针（AncOK 剪枝的前提是父存在但未持，非父=None）"
    );
    let reg = super::super::super::persistent::PersistentRegistry::new();
    let (active, p) = coverage_step_from_buckets(
        view_split(&elements, cstart),
        &[],
        &buckets,
        1000.0,
        &cfg(),
        None,
        &reg,
    );
    assert!(
        active.is_empty(),
        "未持父 ⟹ ReverseOpen 子腿被 AncOK 剪枝（639(c)：不开 naked 逆势仓）；实得 {active:?}"
    );
    assert_eq!(p, 0.0, "ReverseOpen 剔除 ⟹ 无活动腿 ⟹ p̃=0（不开仓）");
}

/// ★测试③（根级无父要求）：Ambient 根候选（缺塔/host 是根，σ_p=0）⟹ 空持仓也正常准入。
#[test]
fn ancok_admits_ambient_root_without_held_parent() {
    let reg = super::super::super::persistent::PersistentRegistry::new();
    use super::super::super::interp::{
        assemble_gamma_with_tower, coverage_elements_with_tower, interpret,
    };
    let tower: Vec<Rc<Vec<LeveledMove>>> = Vec::new(); // 缺塔 ⟹ 候选父=∂ ⟹ Ambient 根
    let classification = Classification {
        levels: vec![LevelState {
            bsp: Rc::new(vec![buy_bsp(4)]),
            ..Default::default()
        }],
    };
    let (elements, cstart) = coverage_elements_with_tower(&classification, &tower);
    let gamma = assemble_gamma_with_tower(&classification, &tower);
    assert_eq!(
        gamma[0].role.v,
        Vertical::Ambient,
        "缺塔 ⟹ 候选父=∂ ⟹ Ambient 根"
    );
    let buckets = interpret(&gamma, &[]); // 空持仓
    let (active, p) = coverage_step_from_buckets(
        view_split(&elements, cstart),
        &[],
        &buckets,
        1000.0,
        &cfg(),
        None,
        &reg,
    );
    assert_eq!(active.len(), 1, "Ambient 根腿无父要求 ⟹ 空持仓也准入");
    assert_eq!(active[0].dir, VoiceSide::Long);
    assert!((p - 600.0).abs() < 1e-9, "根 depth 0 ⟹ p̃=base×w[0]=600");
}

/// ★测试④（持父→撤父→连带剪枝，覆盖不漂浮）：先持父仓准入 ReverseOpen 子腿，下一 bar 父腿被反向
/// 关闭（𝒟_x）⟹ 子腿同 bar 失祖先 ⟹ AncOK 连带剪枝（spec §13 覆盖不漂浮）。
#[test]
fn ancok_prunes_child_when_parent_closed_same_step() {
    use super::super::super::interp::{
        assemble_gamma_with_tower, coverage_elements_with_tower, interpret,
    };
    let tower = rc_tower(vec![
        Vec::new(),
        vec![nested_l1(
            0,
            12,
            [Direction::Up, Direction::Down, Direction::Up],
        )],
    ]);
    // 同 bar 两候选：L1 卖（反向关闭 L1 Long 父腿）+ L0 卖（ReverseOpen 子腿）。
    let classification = Classification {
        levels: vec![
            LevelState {
                bsp: Rc::new(vec![sell_bsp(8)]),
                ..Default::default()
            }, // L0：ReverseOpen 子
            LevelState {
                bsp: Rc::new(vec![sell_bsp(12)]),
                ..Default::default()
            }, // L1：反向关父
        ],
    };
    let (elements, cstart) = coverage_elements_with_tower(&classification, &tower);
    let gamma = assemble_gamma_with_tower(&classification, &tower);
    // 持父仓（L1 Long，ID=(1,0) 与塔 compose 元素同 ID）。
    let held_parent = ActiveLeg {
        level: 1,
        dir: VoiceSide::Long,
        source_index: 12,
        lambda: 0,
        id: eid(1, 0),
        parent_id: None,
        is_boundary_root: true,
        op_parent: None,
    };
    let buckets = interpret(&gamma, &[held_parent]);
    // L1 卖反向关闭 L1 Long 父腿（𝒟_x），故 (A_t∖𝒟_x) 不含父 ⟹ 子腿失祖先 ⟹ AncOK 连带剪枝。
    assert!(
        buckets.close.iter().any(|l| l.level == 1),
        "L1 卖反向关闭 L1 Long 父腿（𝒟_x）"
    );
    let reg = super::super::super::persistent::PersistentRegistry::new();
    let (active, _p) = coverage_step_from_buckets(
        view_split(&elements, cstart),
        &[held_parent],
        &buckets,
        1000.0,
        &cfg(),
        None,
        &reg,
    );
    assert!(
        !active
            .iter()
            .any(|l| l.level == 0 && l.dir == VoiceSide::Short),
        "父腿同 bar 关闭 ⟹ ReverseOpen 子腿连带剪枝（覆盖不漂浮）；实得 {active:?}"
    );
}

/// ★工位 G 引擎自举测试（depth>0 准入 bootstrap，H2 裁决）：level-(ℓ+1) BSP 确认 ⟹ 其 **host
/// 容器** 同 bar 开腿入 raw（容器作 §8 σ_r 根声部持仓）⟹ 同 bar level-ℓ ReverseOpen 子候选的真
/// Compose 父（= 该容器）在 raw ⟹ AncOK 准入 depth>0 子腿。**空 prev_active 即可产 depth>0**，
/// 不再依赖外部预注入持仓父腿（死循环根因消除）。
///
/// 死循环根因（H2 settled）：旧引擎 open 集 100% 来自 BSP 叶子点（lambda==rho==source_index，
/// 挂容器**之下**作叶子），容器自身从不入 next_active ⟹ 永不成持仓腿 ⟹ depth>0 子腿父永不在
/// raw ⟹ AncOK 永剪。本测试断言：容器自身的 BSP 确认时，容器作根声部开腿（§8 σ_r=最高级别=持仓）。
///
/// 定义依据：pasted-text §8（根声部 σ_r=持仓）+ §9 祖先闭合（子声部激活 ⟹ 父声部激活持仓=AncOK）。
#[test]
fn engine_bootstrap_container_bsp_admits_depth_child_from_empty() {
    let reg = super::super::super::persistent::PersistentRegistry::new();
    // per-bar 因果塔：L1 Long 父走势（compose ρ=12，idx0）+ 3 L0 子（idx1/2/3）。
    let tower = rc_tower(vec![
        Vec::new(),
        vec![nested_l1(
            0,
            12,
            [Direction::Up, Direction::Down, Direction::Up],
        )],
    ]);
    // 两个 BSP 同 bar：
    //  - L1 容器自身的卖点（src=12=容器 ρ，host=L1 容器，是根 ⟹ Ambient 根腿，AncOK 无父要求准入）。
    //  - L0 卖点（src=8=sub(4,8) ρ，host=sub，真父=L1 容器 ⟹ ReverseOpen depth=1 子腿）。
    let classification = Classification {
        levels: vec![
            LevelState {
                bsp: Rc::new(vec![sell_bsp(8)]),
                ..Default::default()
            }, // L0：ReverseOpen 子
            LevelState {
                bsp: Rc::new(vec![sell_bsp(12)]),
                ..Default::default()
            }, // L1：容器自身买卖点
        ],
    };
    // ★空 prev_active：无外部预注入持仓父腿（死循环场景）。
    let (active, _p) =
        coverage_step_classification(&classification, &tower, &[], 1000.0, &cfg(), None, &reg);
    // 自举后：L1 容器腿开（其 BSP 确认）+ L0 ReverseOpen 子腿准入（父=L1 容器在 raw）。
    assert!(
        active.iter().any(|l| l.level == 1),
        "L1 容器自身 BSP 确认 ⟹ 容器开腿（§8 σ_r 根声部持仓）；实得 {active:?}"
    );
    assert!(
            active.iter().any(|l| l.level == 0 && l.dir == VoiceSide::Short),
            "容器在 raw ⟹ L0 ReverseOpen depth>0 子腿 AncOK 准入（空 prev_active 即产 depth>0）；实得 {active:?}"
        );
}

/// ★工位 G 自举非膨胀守卫（639(c) 保护）：**没有**容器自身 BSP 时，孤立 L0 ReverseOpen 候选仍被剪枝
/// （不开 naked 逆势仓）。自举只在容器自身买卖点确认时开容器腿，不无条件放行所有 ReverseOpen。
#[test]
fn engine_bootstrap_does_not_admit_orphan_reverse_open_without_container_bsp() {
    let reg = super::super::super::persistent::PersistentRegistry::new();
    let tower = rc_tower(vec![
        Vec::new(),
        vec![nested_l1(
            0,
            12,
            [Direction::Up, Direction::Down, Direction::Up],
        )],
    ]);
    // 仅 L0 ReverseOpen 候选，**无** L1 容器 BSP ⟹ 容器不开腿 ⟹ 子腿父不在 raw ⟹ 剪枝（639(c)）。
    let classification = Classification {
        levels: vec![LevelState {
            bsp: Rc::new(vec![sell_bsp(8)]),
            ..Default::default()
        }],
    };
    let (active, p) =
        coverage_step_classification(&classification, &tower, &[], 1000.0, &cfg(), None, &reg);
    assert!(
        active.is_empty(),
        "无容器 BSP ⟹ ReverseOpen 子腿仍剪枝（自举不膨胀，639(c) 保护）；实得 {active:?}"
    );
    assert_eq!(p, 0.0, "孤立 ReverseOpen 剪枝 ⟹ p̃=0");
}

/// ★工位 H 跨 bar 持仓父链测试（depth>0 准入的**真**生产场景，非同 bar 共现）：
///
/// 真实数据（L2 诊断坐实）：L1 容器 BSP 极稀疏（全窗仅 ~5 个），几乎从不与 L0 子 BSP 同 bar 共现
/// ⟹ G 的同 bar 自举（`engine_bootstrap_*`）结构上几乎不触发 ⟹ active_depth1=0。真实场景是：
/// **bar t 容器 BSP 开容器腿 → 容器腿跨 bar 持有 → bar t+1（无 L1 BSP）L0 子 ReverseOpen 借持仓容器
/// 准入**（§8 σ_r 持仓跨 bar，§9 祖先=持仓父在 A_t）。
///
/// 本测试用**两 bar 序列**复现真实路径：
/// - bar1：仅 L1 容器卖点（src=12）⟹ 容器腿开（根，§8 σ_r）。
/// - bar2：仅 L0 子卖点（src=8，**无** L1 BSP）+ prev_active=bar1 的容器腿 ⟹ L0 ReverseOpen 子腿
///   的真 Compose 父（= 持仓容器腿）在 A_t ⟹ AncOK 准入（639(c) 兑现：父**已持仓**才放行）。
///
/// **RED（修复前）**：bar2 的 host 注入用 `(c.level=0, c.source_index=8)` = L0 子 host 自身（叶子），
/// 容器（L1，parent_id 指向）从不在 raw；prev_active 容器腿对位回树后入 raw，但 G 的注入逻辑不补
/// 容器到 raw——实际上**持仓容器腿**（prev_active）才是父在场源。若持仓对位生效，bar2 应准入子腿。
///
/// 定义依据：639(c)（持仓准入 §13 AncOK，父在 A_t 才放行——与本测试断言一致，非膨胀）；
/// §8 σ_r 根声部持仓跨 bar；§9 祖先闭合（子激活 ⟹ 持仓父在场）。
#[test]
fn cross_bar_held_container_admits_depth_child_next_bar() {
    let reg = super::super::super::persistent::PersistentRegistry::new();
    let tower = rc_tower(vec![
        Vec::new(),
        vec![nested_l1(
            0,
            12,
            [Direction::Up, Direction::Down, Direction::Up],
        )],
    ]);
    // bar1：仅 L1 容器卖点（src=12=容器 ρ）⟹ 容器腿开（根，§8 σ_r 持仓）。
    let bar1 = Classification {
        levels: vec![
            LevelState::default(), // L0：无 BSP
            LevelState {
                bsp: Rc::new(vec![sell_bsp(12)]),
                ..Default::default()
            }, // L1：容器自身卖点
        ],
    };
    let (active1, _p1) =
        coverage_step_classification(&bar1, &tower, &[], 1000.0, &cfg(), None, &reg);
    assert!(
        active1.iter().any(|l| l.level == 1),
        "bar1：L1 容器 BSP ⟹ 容器腿开（§8 σ_r 持仓根）；实得 {active1:?}"
    );
    // registry merge（生产 instrument_loop 同序）：跨 bar 持久身份。
    let (elements1, _c1) = super::super::super::interp::coverage_elements_with_tower(&bar1, &tower);
    let reg2 = reg.merge(&elements1, &active1);

    // bar2：仅 L0 子卖点（src=8，**无** L1 BSP）+ prev_active=bar1 容器腿。
    let bar2 = Classification {
        levels: vec![LevelState {
            bsp: Rc::new(vec![sell_bsp(8)]),
            ..Default::default()
        }],
    };
    let (active2, _p2) =
        coverage_step_classification(&bar2, &tower, &active1, 1000.0, &cfg(), None, &reg2);
    // 持仓容器腿（prev_active）= L0 子腿真 Compose 父在 A_t ⟹ AncOK 准入 depth>0 子腿。
    assert!(
        active2
            .iter()
            .any(|l| l.level == 0 && l.dir == VoiceSide::Short),
        "bar2：L0 ReverseOpen 子腿借跨 bar 持仓容器准入（639(c) 父已持仓放行）；实得 {active2:?}"
    );
}

/// ★#694（语义重放自 kimi #346/#347 LOW-1）：环形 `parent_id`（自指，`e.id == e.parent_id`）
/// 不得使 `ancestors_by_id_lookup` 挂起——本仓 fuel 硬门风格（同 `element_depth` #247 C1）下
/// 应改为**显式 panic**（非静默截断、非挂起）。构造单元素、`parent_id` 指向自身、`lookup`
/// 闭包能解析回自己（模拟 held-leg 占位 `op_parent` 自环，函数头 #694 订正记述的数据来源）。
#[test]
#[should_panic(expected = "环路硬门")]
fn ancestors_by_id_lookup_self_parent_id_panics_not_hangs() {
    let self_id = eid(7, 7);
    let base = vec![CoverageElement {
        lambda: 0,
        rho: 4,
        eps: VoiceSide::Short,
        level: 0,
        parent: None,
        attached_dir: None,
        id: self_id,
        parent_id: Some(self_id), // ★环形：自己的父就是自己。
    }];
    let view = ElementView::new(&base);
    let lookup = |pid: &ElementId| -> Option<usize> {
        if *pid == self_id {
            Some(0)
        } else {
            None
        }
    };
    ancok_probe_reset();
    let _ = ancestors_by_id_lookup(&view, 0, &lookup);
}
