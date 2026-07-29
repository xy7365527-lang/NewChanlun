use super::super::*;
use super::super::super::parser::ParseLayer;
use super::super::super::types::{Direction, MoveKind};
use super::{bars_from_closes, seg};

#[test]
fn classify_empty_layer_yields_empty() {
    let cfg = ThetaConfig::default();
    let out = classify(&ParseLayer::default(), &cfg);
    assert_eq!(out, Classification::default());
}

#[test]
fn fewer_than_min_parts_natural_termination() {
    // L0 线段数 < min_parts_per_level(3) ⟹ 无 L0 走势，levels 空（自然终止）。
    let cfg = ThetaConfig::default();
    let layer = ParseLayer {
        segments: Rc::new(vec![seg(Direction::Up, 0, 4, 0, 10), seg(Direction::Down, 4, 8, 10, 5)]),
        ..Default::default()
    };
    let out = classify(&layer, &cfg);
    assert!(out.levels.is_empty(), "2 段 < min_parts 3 ⟹ 自然终止");
}

#[test]
fn three_overlapping_segments_form_center() {
    // 完整判据（Origin.CenterComplete，口径 B 637号）：方向交替 上-下-上 + 全三段核心非空。
    // 段区间 [0,10]up,[3,12]down,[5,15]up：核心取**全三段** zd=max(0,3,5)=5, zg=min(10,12,15)=10。
    // 第三段 [5,15] 收窄核心下沿（A 口径 zd=3 → B zd=5）⟹ 真中枢成立。
    let cfg = ThetaConfig::default();
    let layer = ParseLayer {
        segments: Rc::new(vec![
            seg(Direction::Up, 0, 4, 0, 10),
            seg(Direction::Down, 4, 8, 12, 3),
            seg(Direction::Up, 8, 12, 5, 15),
        ]),
        ..Default::default()
    };
    let out = classify(&layer, &cfg);
    assert!(!out.levels.is_empty());
    let l0 = &out.levels[0];
    assert_eq!(l0.centers.len(), 1, "三段方向交替+贯穿 ⟹ 一个真中枢");
    // 核心取全三段（口径 B，637号 computeZD/computeZG s1 s2 s3）——第三段收窄核心下沿至 5。
    assert_eq!((l0.centers[0].zd, l0.centers[0].zg), (5, 10));
    // 一个中枢 ⟹ 分解 = 单盘整块（PDF §6 情形1）。
    assert_eq!(l0.moves.len(), 1);
    assert_eq!((l0.moves[0].kind, l0.moves[0].start_center, l0.moves[0].end_center),
               (MoveKind::Consolidation, 0, 0));
}

#[test]
fn same_direction_three_segments_rejected_by_complete() {
    // G4 完整判据反退化：三段同向（全 up）+ 前两段核心非空，但无方向交替 ⟹ L0 不识别中枢
    // （旧几何窗口会误判，完整判据正确拒绝，对齐 Origin.CenterComplete.sameDir_not_centerConfirmed）。
    let cfg = ThetaConfig::default();
    let layer = ParseLayer {
        segments: Rc::new(vec![
            seg(Direction::Up, 0, 4, 10, 20),
            seg(Direction::Up, 4, 8, 18, 25),
            seg(Direction::Up, 8, 12, 22, 30),
        ]),
        ..Default::default()
    };
    let out = classify(&layer, &cfg);
    // 无方向交替 ⟹ L0 无中枢（完整判据拒绝单边三段）。
    assert_eq!(out.levels.len(), 1);
    assert!(out.levels[0].centers.is_empty(), "同向三段无方向交替 ⟹ 完整判据拒绝（非中枢）");
}

#[test]
fn lmax_bound_respected() {
    // 构造大量重叠段，验证递归不超过 l_max+1 级（每级至少消耗中枢，最终自然终止）。
    let cfg = ThetaConfig::default();
    let mut segments = Vec::new();
    // 27 段全重叠区间 [0,100]（每三段成一中枢，逐级递归）。
    for i in 0..27 {
        let dir = if i % 2 == 0 { Direction::Up } else { Direction::Down };
        segments.push(seg(dir, i * 4, i * 4 + 4, 0, 100));
    }
    let layer = ParseLayer { segments: Rc::new(segments), ..Default::default() };
    let out = classify(&layer, &cfg);
    // 级别数不超过 l_max+1（reference:30 上界，default l_max=6）。
    assert!(out.levels.len() <= cfg.level.l_max as usize + 1);
}

#[test]
fn no_center_terminates_recursion() {
    // 三段无公共重叠 ⟹ L0 无中枢 ⟹ moves 为空（HigherCenterCandidate→None）+ 递归终止。
    let cfg = ThetaConfig::default();
    let layer = ParseLayer {
        segments: Rc::new(vec![
            seg(Direction::Up, 0, 4, 0, 4),
            seg(Direction::Down, 4, 8, 14, 10),
            seg(Direction::Up, 8, 12, 20, 24),
        ]),
        ..Default::default()
    };
    let out = classify(&layer, &cfg);
    // L0 无中枢 ⟹ 该级 moves 空（裁决退化），递归在该级后终止（无上级单元）。
    assert_eq!(out.levels.len(), 1);
    assert!(out.levels[0].centers.is_empty());
    assert!(out.levels[0].moves.is_empty());
}

/// ★端到端 fixture（Lead 验证门）：≥3 重叠线段 → 非空中枢 + 至少一个买卖点。
/// 解阻塞点 A：classify 在真实结构输入上返回非空 Classification（n_orders>0 的前提）。
#[test]
fn end_to_end_third_buy_signal() {
    let cfg = ThetaConfig::default();
    // 段0-2：三段在 [100,200] 重叠 ⟹ seed 中枢，核心 [ZD,ZG]=[100,200] 冻结，end_index=12。
    // 段3：向上离开——延伸语义下（PDF §5 Step3，task #142）离开段必须与冻结核心不相交：
    //   lo=205 > ZG=200 ⟹ non-extension（旧 fixture lo=150 ≤ 200 会被 Step2 吸收进中枢 ⟹ 无离开段）。
    // 段4：向下回试低点 210 > zg=200（严格不触闭区间）⟹ 3 买 @ source_index=20。
    let layer = ParseLayer {
        segments: Rc::new(vec![
            seg(Direction::Up, 0, 4, 100, 200),
            seg(Direction::Down, 4, 8, 200, 100),
            seg(Direction::Up, 8, 12, 100, 200),
            seg(Direction::Up, 12, 16, 205, 250),   // 离开中枢上方（lo=205>ZG ⟹ non-extension）
            seg(Direction::Down, 16, 20, 250, 210), // 回试低点 > zg → 3 买
        ]),
        ..Default::default()
    };
    let out = classify(&layer, &cfg);

    // 1) 非空 Classification + L0 有中枢。
    assert!(!out.levels.is_empty(), "解阻塞 A：classify 返回非空");
    let l0 = &out.levels[0];
    assert_eq!(l0.centers.len(), 1, "三段重叠 ⟹ 一个中枢");
    assert_eq!((l0.centers[0].zd, l0.centers[0].zg), (100, 200));

    // 2) 至少一个买卖点（第三类买点），且携带结构止损价 single source。
    assert!(!l0.bsp.is_empty(), "解阻塞 A：L0 至少一个买卖点");
    let third_buys: Vec<_> = l0.bsp.iter().filter(|p| p.bits.buy3).collect();
    assert_eq!(third_buys.len(), 1, "一个第三类买点");
    let p = third_buys[0];
    assert_eq!(p.source_index, 20, "买卖点定位回试端点");
    assert_eq!(p.pivot_low, 210, "结构止损价 pivot_low single source");
    assert_eq!(
        p.center.and_then(|o| match o { signal::OwnerRef::Center(c) => Some(c.zg), _ => None }),
        Some(200),
        "3 买止损 = ZG single source（#218 面 A 载体形态：Center 变体读出）"
    );
}

/// 端到端边界：有中枢但无离开/回试 ⟹ 中枢非空、bsp 空（无买卖点是诚实产出，非错误）。
#[test]
fn end_to_end_center_without_signal() {
    let cfg = ThetaConfig::default();
    // 三段重叠成中枢，但无后续离开线段 ⟹ 无第三类买卖点。
    let layer = ParseLayer {
        segments: Rc::new(vec![
            seg(Direction::Up, 0, 4, 100, 200),
            seg(Direction::Down, 4, 8, 200, 100),
            seg(Direction::Up, 8, 12, 100, 200),
        ]),
        ..Default::default()
    };
    let out = classify(&layer, &cfg);
    assert_eq!(out.levels[0].centers.len(), 1);
    assert!(out.levels[0].bsp.is_empty(), "无离开/回试 ⟹ 无买卖点（诚实空）");
}

/// ★classify_with_tower (i) 段导出桥——tower 非空 + depth≥1 真嵌套存在。
///
/// 9 段 L0 → 3 个 L1 走势 → L2 中枢（几何路径）：tower[1] 含 sub_moves 非空的
/// LeveledMove（RMove::Compose，depth=1 真嵌套）。坐实：导出桥正确产出真嵌套塔。
#[test]
fn classify_with_tower_depth_ge1_true_nesting() {
    let cfg = ThetaConfig::default();
    // 9 段：三组 up-down-up（每组 → 一个 L1 走势），三个 L1 走势外缘重叠成 L2 中枢。
    // ★task #142 延伸语义诚实重算（同 end_to_end_second_buy_via_l1_l2_geometric 推导）：三组
    // 核心分离（组B 首段 hi=115<ZD_A=120、组C 首段 lo=115>ZG_B=114 ⟹ non-extension，PDF §5
    // Step3），外缘 O_A=[110,150]/O_B=[80,125]/O_C=[112,148] 共同相交 ⟹ L2 核心 [112,125] 非空。
    let segments = vec![
        seg(Direction::Up,   0,  4, 110, 150),
        seg(Direction::Down, 4,  8, 150, 120),
        seg(Direction::Up,   8, 12, 120, 148),
        seg(Direction::Down,12, 16, 115,  80),
        seg(Direction::Up,  16, 20,  80, 125),
        seg(Direction::Down,20, 24, 114,  85),
        seg(Direction::Up,  24, 28, 115, 148),
        seg(Direction::Down,28, 32, 148, 112),
        seg(Direction::Up,  32, 36, 112, 147),
    ];
    let mut closes: Vec<i64> = Vec::new();
    for i in 0..12 { closes.push(100 + if i % 2 == 0 { 40 } else { -40 }); }
    for i in 0..12 { closes.push(100 + if i % 2 == 0 {  5 } else {  -5 }); }
    for i in 0..16 { closes.push(100 + if i % 2 == 0 {  3 } else {  -3 }); }
    let layer = ParseLayer { segments: Rc::new(segments), merged_bars: Rc::new(bars_from_closes(&closes)), ..Default::default() };
    let (_, tower) = classify_with_tower(&layer, &cfg);
    assert!(!tower.is_empty(), "tower 非空（至少 L0 级被处理）");
    assert!(tower.len() >= 2, "9 段 L0 → 3 个 L1 走势 → L2 中枢 ⟹ tower 至少 2 层");
    // depth≥1 真嵌套：tower[1] 含 sub_moves 非空的 LeveledMove（RMove::Compose，L1 输入塔）。
    let has_true_nesting = tower[1].iter().any(|m| !m.sub_moves.is_empty());
    assert!(has_true_nesting, "tower[1] 含真嵌套 LeveledMove（sub_moves 非空，depth≥1）");
}

/// ★classify_with_tower Classification 与 classify 同输入 bit-identical（导出不改原分类）。
#[test]
fn classify_with_tower_classification_equals_classify() {
    let cfg = ThetaConfig::default();
    // ★task #142 延伸语义诚实重算（同 end_to_end_second_buy_via_l1_l2_geometric 推导）：三组
    // 核心分离（组B 首段 hi=115<ZD_A=120、组C 首段 lo=115>ZG_B=114 ⟹ non-extension，PDF §5
    // Step3），外缘 O_A=[110,150]/O_B=[80,125]/O_C=[112,148] 共同相交 ⟹ L2 核心 [112,125] 非空。
    let segments = vec![
        seg(Direction::Up,   0,  4, 110, 150),
        seg(Direction::Down, 4,  8, 150, 120),
        seg(Direction::Up,   8, 12, 120, 148),
        seg(Direction::Down,12, 16, 115,  80),
        seg(Direction::Up,  16, 20,  80, 125),
        seg(Direction::Down,20, 24, 114,  85),
        seg(Direction::Up,  24, 28, 115, 148),
        seg(Direction::Down,28, 32, 148, 112),
        seg(Direction::Up,  32, 36, 112, 147),
    ];
    let mut closes: Vec<i64> = Vec::new();
    for i in 0..12 { closes.push(100 + if i % 2 == 0 { 40 } else { -40 }); }
    for i in 0..12 { closes.push(100 + if i % 2 == 0 {  5 } else {  -5 }); }
    for i in 0..16 { closes.push(100 + if i % 2 == 0 {  3 } else {  -3 }); }
    let layer = ParseLayer { segments: Rc::new(segments), merged_bars: Rc::new(bars_from_closes(&closes)), ..Default::default() };
    let expected = classify(&layer, &cfg);
    let (actual, _) = classify_with_tower(&layer, &cfg);
    assert_eq!(actual, expected, "classify_with_tower Classification 与 classify bit-identical（原分类不变）");
}
