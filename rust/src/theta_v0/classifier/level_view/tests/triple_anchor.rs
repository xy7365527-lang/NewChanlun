use super::fixtures::{extended_windows, mbar, trend_block};
use super::*;

// ───────────── T1 (#170) 三元锚供给/携带测试（先红后绿） ─────────────

/// 锚解析口径锁：极值价 = 拐点处 L0 分型极值（`Fractal.price`，整数 tick 精确等值，
/// 不设 kind 守卫——高级别走势可以次级别反向段收束，x 在 L0 的分型方向可与事件 side
/// 不同，但 x 处实际打印价跨级不变）；组锚 = 该级包含层组内首根序号。
#[test]
fn triple_anchor_resolution_caliber() {
    let fractals = vec![
        Fractal {
            kind: FractalKind::Bottom,
            source_index: 10,
            timestamp: 10,
            price: 100,
        },
        // W 底第二脚：同向同价（底, 100），与第一脚分属不同合并组。
        Fractal {
            kind: FractalKind::Bottom,
            source_index: 30,
            timestamp: 30,
            price: 100,
        },
        Fractal {
            kind: FractalKind::Top,
            source_index: 50,
            timestamp: 50,
            price: 190,
        },
    ];
    // 合并组：g0 = raw [0,20)（锚 0）；g1 = raw [20,50)（锚 20）；g2 = raw [50,∞)（锚 50）。
    let merged = vec![mbar(0), mbar(20), mbar(50)];
    // W 底双脚：同向同价由合并组区分——锚不同 ⟹ 键不同（教义裁定 4）。
    let (p1, a1) = resolve_triple_anchor(10, &fractals, &merged);
    let (p2, a2) = resolve_triple_anchor(30, &fractals, &merged);
    assert_eq!((p1, a1), (Some(100), Some(0)));
    assert_eq!((p2, a2), (Some(100), Some(20)));
    assert_ne!(a1, a2, "同向同价碰撞由合并组区分（教义裁定 4）");
    // 跨级不变量构造锁：解析只读（x, 供给），不读级别几何——同一 x 在任意级别查到
    // 同一分型 ⟹ 极值价逐值相同（教义裁定 3）。
    let again = resolve_triple_anchor(10, &fractals, &merged);
    assert_eq!(
        again,
        (p1, a1),
        "同一 x 重复解析逐值相同（与级别无关 ⟹ 跨级不变）"
    );
    assert_eq!(
        p1,
        Some(100),
        "极值价口径 = 分型极值价（整数 tick，非腿包络/非原始 K 极值）"
    );
    // x 在 L0 是反向分型（高级别走势以次级别反向段收束的情形）：极值价照取 x 处实际
    // 打印价（190 = 顶分型 high）——键内方向由 event.side 携带，不经本供给。
    let (pt, at) = resolve_triple_anchor(50, &fractals, &merged);
    assert_eq!(
        (pt, at),
        (Some(190), Some(50)),
        "极值价 = x 处实际分型价（不设 kind 守卫）"
    );
    // 分型供给未命中 ⟹ 极值价 None；组锚仍可解（x=51 ∈ g2）。
    let (pm, am) = resolve_triple_anchor(51, &fractals, &merged);
    assert_eq!(pm, None);
    assert_eq!(am, Some(50), "组内后续根映射回组锚");
    // 空供给 ⟹ 双 None（事件视图包装路径——锚载体不进事件本体）。
    assert_eq!(resolve_triple_anchor(10, &[], &[]), (None, None));
}

/// provider ext 携带两元锚（T1 #170；T5a #207 方向退役后锚 = 极值价 + 组锚）：confirmed 事件的极值价/组锚来自分型管与包含层
/// 单一来源；锚是 sidecar——事件集/排序与无锚供给路径逐字节同。
#[test]
fn provider_ext_carries_triple_anchor_sidecar() {
    // 夹具同 auto_pairing_completes_only_after_real_macd_divergence：confirmed 事件
    // 离开段 (120,129)，x=129，side=Short（Direction::Up 对）。
    let (windows, lower) = extended_windows();
    let projection = project_extended_windows_carried_only(&windows).unwrap();
    let legs = lower_legs_from(&lower).unwrap();
    // provider 的 interval_a 需要 leave→retest 块对（structural_pair_span：两块均
    // Completed）；retest 块取 Consolidation（dir None ⟹ 不再产第二个 pair，夹具保单 pair）。
    let blocks = [
        trend_block(Some(Direction::Up)),
        MoveBlock {
            start_center: 1,
            end_center: 2,
            kind: MoveKind::Consolidation,
            dir: None,
            status: MoveStatus::Completed,
        },
    ];
    let mut hist = vec![0.0; 140];
    hist[80..110].fill(2.0);
    hist[120..140].fill(0.1);
    let mut dif = vec![0.0; 140];
    dif[80..=100].fill(-5.0);
    dif[101..140].fill(1.0);
    let close_src: Vec<_> = (0..140).collect();
    let query = LevelViewQuery {
        level: 1,
        coordinate_window: CoordinateWindow { start: 0, end: 139 },
        as_of: 139,
        version: C2VersionTuple::auto_pairing(),
    };
    let view = assemble_level_view(
        C2LevelViewConfig { enabled: true },
        query,
        LevelViewMaterial {
            projection: ProjectionMaterial::ExactThree(&projection),
            move_blocks: &blocks,
            lower_legs: &legs,
            hist: &hist,
            dif: &dif,
            close_src: &close_src,
        },
    )
    .unwrap();
    // 供给：x=129 处顶分型（极值 190 = 腿12 hi）；包含层逐根成组（锚 = 序号自身）。
    let fractals = vec![Fractal {
        kind: FractalKind::Top,
        source_index: 129,
        timestamp: 129,
        price: 190,
    }];
    let merged: Vec<Bar> = (0..140).map(mbar).collect();
    let exts = provide_nest_candidate_events_ext(
        1,
        &projection,
        &blocks,
        &legs,
        &view,
        &hist,
        &dif,
        &close_src,
        &fractals,
        &merged,
    );
    let ext = exts
        .iter()
        .find(|e| e.event.divergence_confirmed && e.event.kind == NestDivergenceKind::Trend)
        .expect("夹具应产 1 个 confirmed Trend 事件");
    assert_eq!(
        ext.extreme_price,
        Some(190),
        "极值价 = x=129 处分型极值（跨级不变量口径）"
    );
    assert_eq!(ext.group_anchor, Some(129), "组锚 = x 所在合并组首根序号");
    // 锚 sidecar 不进事件本体：事件集/排序与事件视图（无锚供给）逐字节同。
    let events_only = provide_nest_candidate_events(
        1,
        &projection,
        &blocks,
        &legs,
        &view,
        &hist,
        &dif,
        &close_src,
    );
    assert_eq!(
        exts.iter().map(|e| e.event).collect::<Vec<_>>(),
        events_only,
        "锚 sidecar 不改变事件集/排序（逐字节同）"
    );
}
