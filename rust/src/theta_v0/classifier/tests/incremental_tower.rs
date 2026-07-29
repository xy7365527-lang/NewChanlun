use super::super::pipeline::segment_to_unit;
use super::super::*;
use super::super::super::parser::ParseLayer;
use super::super::super::types::Direction;
use super::{bars_from_closes, seg};

// ──────────────────────────────────────────────────────────────────────
//  增量塔 API bit-exact（task #93：incremental == 全量，逐 bar 断言）
// ──────────────────────────────────────────────────────────────────────

/// 构造逐段追加的合成段序列（方向交替 + 价格震荡，产足够中枢触发多级塔）。
fn synthetic_segments(count: usize) -> Vec<Segment> {
    (0..count)
        .map(|i| {
            let dir = if i % 2 == 0 { Direction::Up } else { Direction::Down };
            let base = 100i64 + (i as i64) * 3;
            let swing = if i % 2 == 0 { 50 } else { -50 };
            let sp = base;
            let ep = base + swing;
            seg(dir, i * 4, i * 4 + 4, sp, ep)
        })
        .collect()
}

/// ★增量塔单次 bit-exact：对完整段序列，`classify_with_tower_incremental(.., fresh cache)`
/// 输出 == `classify_with_tower`（Classification + tower 逐字段相等）。
///
/// fresh cache（空）从 consumed=0 续扫 == 全量扫描。验证增量入口的基础正确性。
#[test]
fn incremental_tower_fresh_cache_equals_full() {
    let cfg = ThetaConfig::default();
    for n in [3usize, 6, 9, 12, 18] {
        let segments = synthetic_segments(n);
        let closes: Vec<i64> = (0..(n * 4 + 8) as i64).map(|i| 100 + (i % 5) * 5).collect();
        let layer =
            ParseLayer { segments: Rc::new(segments), merged_bars: Rc::new(bars_from_closes(&closes)), ..Default::default() };

        let (full_cls, full_tower) = classify_with_tower(&layer, &cfg);
        let mut cache = TowerCache::new();
        let (inc_cls, inc_tower) = classify_with_tower_incremental(&layer, &cfg, &mut cache);

        assert_eq!(inc_cls, full_cls, "n={n}: 增量 Classification == 全量");
        assert_eq!(inc_tower.len(), full_tower.len(), "n={n}: 增量 tower 层数 == 全量");
        for (lvl, (il, fl)) in inc_tower.iter().zip(full_tower.iter()).enumerate() {
            assert_eq!(il, fl, "n={n} level {lvl}: 增量 tower 级 LeveledMove 序列 == 全量");
        }
    }
}

/// ★增量塔逐段追加 bit-exact（#93 核心铁律）：模拟 per-bar substrate 逐段追加，
/// 每步断言 `classify_with_tower_incremental(layer[..=i], cache)` ==
/// `classify_with_tower(layer[..=i])`（Classification + tower 逐字段相等）。
///
/// 这是增量塔的真实使用场景——段账本单调增长，cache 跨步复用前级 confirmed 前缀。
/// 任何 resume bit-exact 破裂、跨级传播错误、裁决漂移都会在此捕获。
#[test]
fn incremental_tower_per_segment_append_matches_full() {
    let cfg = ThetaConfig::default();
    let all_segments = synthetic_segments(21);
    let closes: Vec<i64> = (0..100).map(|i| 100 + (i % 7) * 4).collect();

    let mut cache = TowerCache::new();
    for n in 1..=all_segments.len() {
        let segments = all_segments[..n].to_vec();
        let layer = ParseLayer {
            segments: Rc::new(segments),
            merged_bars: Rc::new(bars_from_closes(&closes)),
            ..Default::default()
        };

        // 全量基准。
        let (full_cls, full_tower) = classify_with_tower(&layer, &cfg);
        // 增量（cache 跨步复用）。
        let (inc_cls, inc_tower) = classify_with_tower_incremental(&layer, &cfg, &mut cache);

        assert_eq!(inc_cls, full_cls, "n={n}: 增量 Classification != 全量（bit-exact 破裂）");
        assert_eq!(
            inc_tower.len(),
            full_tower.len(),
            "n={n}: 增量 tower 层数 != 全量"
        );
        for (lvl, (il, fl)) in inc_tower.iter().zip(full_tower.iter()).enumerate() {
            assert_eq!(
                il, fl,
                "n={n} level {lvl}: 增量 tower 级 LeveledMove != 全量（真 subs 嵌套破裂）"
            );
        }
    }
}



/// ★段账本回缩退化 bit-exact：模拟 parser 回撤最后一段（非单调追加），
/// `cache` 自动检测回缩 ⟹ 清空 + 全量重扫 ⟹ 仍 bit-exact（退化不破坏正确性）。
#[test]
fn incremental_tower_shrink_falls_back_to_full() {
    let cfg = ThetaConfig::default();
    let all_segments = synthetic_segments(12);
    let closes: Vec<i64> = (0..80).map(|i| 100 + (i % 6) * 4).collect();

    let mut cache = TowerCache::new();
    // 先追加到 12 段。
    let layer_full =
        ParseLayer { segments: Rc::new(all_segments.clone()), merged_bars: Rc::new(bars_from_closes(&closes)), ..Default::default() };
    let _ = classify_with_tower_incremental(&layer_full, &cfg, &mut cache);
    // 回缩到 8 段（parser 回撤）。
    let layer_shrink = ParseLayer {
        segments: Rc::new(all_segments[..8].to_vec()),
        merged_bars: Rc::new(bars_from_closes(&closes)),
        ..Default::default()
    };
    let (full_cls, full_tower) = classify_with_tower(&layer_shrink, &cfg);
    let (inc_cls, inc_tower) = classify_with_tower_incremental(&layer_shrink, &cfg, &mut cache);
    assert_eq!(inc_cls, full_cls, "回缩退化：增量 Classification == 全量");
    assert_eq!(inc_tower, full_tower, "回缩退化：增量 tower == 全量");
}

/// ★B2 真产出 bit-exact：增量塔在产 B2 的真实结构（9 段三组 up-down-up）下，
/// `classify_with_tower_incremental` 产出的 B2 与全量 `classify_with_tower` bit-identical——
/// 验证增量 compose 的真 Fugue 547（subs 真 Compose，B2 真可产，禁级别差伪造）。
#[test]
fn incremental_tower_preserves_b2_second_buy() {
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

    let (full_cls, _) = classify_with_tower(&layer, &cfg);
    let mut cache = TowerCache::new();
    let (inc_cls, _) = classify_with_tower_incremental(&layer, &cfg, &mut cache);

    // 全量产 1 个 B2（见 end_to_end_second_buy_via_l1_l2_geometric），增量须 bit-identical。
    let full_b2: Vec<_> = full_cls.levels[1].bsp.iter().filter(|p| p.bits.buy2).collect();
    let inc_b2: Vec<_> = inc_cls.levels[1].bsp.iter().filter(|p| p.bits.buy2).collect();
    assert_eq!(inc_b2.len(), full_b2.len(), "增量塔 B2 数量 == 全量（真 Fugue 547 保留）");
    assert_eq!(inc_b2.len(), 1, "增量塔仍真产 B2（subs 真 Compose，非级别差伪造）");
    assert_eq!(inc_b2[0].source_index, full_b2[0].source_index, "B2 source_index bit-exact");
    assert_eq!(inc_cls, full_cls, "增量塔完整 Classification == 全量（含 B2 BSP）");
}

/// ★codex 反例（cascade reset 完备性，L1 构造）：L0 frontier 末段**内点改写**——
/// 上级投影 `UnitRange(lo,hi)` bit-identical 但底层 `sub_moves` 变。
///
/// 场景：9 段三组（task #142 核心分离 fixture），末段 seg[8] `Up[32,36]` end_price 从 147 改写
/// 为 140。140 是组 C 外缘内点（组 C max-hi=148(seg[6]/seg[7]) / min-lo=112(seg[7]/seg[8].sp)
/// 不变）⟹ L0 该窗口中枢 gg/dd 不变 ⟹ L1 输入投影 `project_to_units` bit-identical。但 seg[8]
/// 的 lo/hi 从 [112,147] 变 [112,140] ⟹ L0 upper_moves[2].sub_moves[2] 深嵌套坐标变。
///
/// 旧守卫（仅比对本级 `project_to_units` 投影）：L0 reset 正确，但 L1 frontier_mutated=false
/// 漏 reset ⟹ `cache.levels[1].upper_moves` 深嵌套 sub_moves 陈旧（仍 [112,147]）+ BSP memo
/// （key 仅三长度）复用陈旧 BSP ⟹ 与全量发散。
/// cascade reset 修复：L0 变异 → 强制 reset L1+（无条件跟随下级），深嵌套 sub_moves 重建为 [112,140]。
///
/// **L1**（合成构造，验证管线完备性，非真实数据假设——formalization-validity-domain 231号）。
#[test]
fn cascade_reset_on_frontier_interior_rewrite() {
    let cfg = ThetaConfig::default();
    // ★task #142 延伸语义诚实重算：三组核心分离 fixture（同 end_to_end_second_buy_via_l1_l2_geometric
    // 推导——组B 首段 hi=115<ZD_A=120、组C 首段 lo=115>ZG_B=114 ⟹ non-extension，PDF §5 Step3）。
    let base = vec![
        seg(Direction::Up,   0,  4, 110, 150),
        seg(Direction::Down, 4,  8, 150, 120),
        seg(Direction::Up,   8, 12, 120, 148),
        seg(Direction::Down,12, 16, 115,  80),
        seg(Direction::Up,  16, 20,  80, 125),
        seg(Direction::Down,20, 24, 114,  85),
        seg(Direction::Up,  24, 28, 115, 148),
        seg(Direction::Down,28, 32, 148, 112),
        seg(Direction::Up,  32, 36, 112, 147), // v1 末段
    ];
    let mut closes: Vec<i64> = Vec::new();
    for i in 0..12 { closes.push(100 + if i % 2 == 0 { 40 } else { -40 }); }
    for i in 0..12 { closes.push(100 + if i % 2 == 0 {  5 } else {  -5 }); }
    for i in 0..16 { closes.push(100 + if i % 2 == 0 {  3 } else {  -3 }); }
    let merged = Rc::new(bars_from_closes(&closes));

    // v1: 末段 end_price=147。
    let layer_v1 = ParseLayer { segments: Rc::new(base.clone()), merged_bars: merged.clone(), ..Default::default() };
    // v2: 仅末段 end_price 改写 147→140（组 C 外缘内点，L1 投影不变，L0 sub_moves 变）。
    let mut v2_segs = base.clone();
    v2_segs[8].end_price = 140;
    let layer_v2 = ParseLayer { segments: Rc::new(v2_segs), merged_bars: merged.clone(), ..Default::default() };

    // 前提自检（codex 反例成立的必要条件）：L1 投影输入 v1==v2 bit-identical（守卫看不到变异），
    // 但 L0 末段 sub_moves 已变（147→140）。若此前提不成立，本测试不构成反例。
    let mk_l1_units = |segs: &Rc<Vec<Segment>>| {
        let l0_units: Vec<UnitRange> = segs.iter().map(segment_to_unit).collect();
        let moves_l0: Vec<LeveledMove> = l0_units.iter().enumerate()
            .map(|(i,u)| LeveledMove::from_unit(u, recursive_tower::ElementId{level:0,ordinal:i as u64})).collect();
        let (c, upper, _) = recursive_tower::compose_level(&l0_units, &moves_l0, true, 1);
        recursive_tower::project_to_units(&upper, &decompose::decompose(&c))
    };
    assert_eq!(mk_l1_units(&layer_v1.segments), mk_l1_units(&layer_v2.segments),
        "前提：L1 投影输入 v1==v2（守卫的本级投影比对看不到此变异）");

    // 共享 cache：先喂 v1（缓存 L0/L1），再喂 v2（frontier 内点改写）——模拟 per-bar 末段重划。
    let mut cache = TowerCache::new();
    let _ = classify_with_tower_incremental(&layer_v1, &cfg, &mut cache);
    let (inc_v2, inc_tower_v2) = classify_with_tower_incremental(&layer_v2, &cfg, &mut cache);
    let (full_v2, full_tower_v2) = classify_with_tower(&layer_v2, &cfg);

    // ★核心断言（latent 陈旧检测，非仅返回值）：cache 内 L1 深嵌套 sub_moves 末段坐标必须 == v2
    // 的 [112,140]。返回的 Classification/tower 不消费 cache 内 L1 upper_moves 的深 subs（tower 用
    // 新鲜 moves_tower 快照），故陈旧在返回值里 latent——但它喂 BSP（extract_second_for_level）+
    // 下一 bar 的 L2 投影。直接断言 cache 深 subs，捕获 latent 陈旧（640：不靠返回值碰巧相等）。
    // 推导（task #142 fixture）：v2 seg[8] = Up 112→140 ⟹ 区间 [112,140]（v1 为 [112,147]）。
    let l0_seg8_full = classify_with_tower(&layer_v2, &cfg).1[0].last().unwrap().rmove.clone();
    assert_eq!(l0_seg8_full, descend::RMove::Segment { direction: Direction::Up, lo: 112, hi: 140 },
        "前提：v2 全量 L0 末段 == [112,140]");
    // cache.L1.upper_moves[0].sub_moves[2](groupC).sub_moves[2](seg[8]) 应 == [112,140]。
    let l1_deep = &cache.levels[1].upper_moves[0].sub_moves[2].sub_moves[2].rmove;
    assert_eq!(*l1_deep, descend::RMove::Segment { direction: Direction::Up, lo: 112, hi: 140 },
        "cascade: cache L1 深嵌套 seg[8] == v2 [112,140]（陈旧则 [112,147]——L1 漏 cascade reset）");

    // 返回值也须 bit-exact（cascade 后 L1 重建，tower/Classification 全对齐）。
    assert_eq!(inc_v2, full_v2, "cascade: v2 增量 Classification == 全量");
    assert_eq!(inc_tower_v2.len(), full_tower_v2.len(), "cascade: tower 层数 == 全量");
    for (lvl, (il, fl)) in inc_tower_v2.iter().zip(full_tower_v2.iter()).enumerate() {
        assert_eq!(il, fl, "cascade: level {lvl} LeveledMove == 全量");
    }
}

/// ★on2w2 G7（epoch 递增覆盖性）：L0 尾段重划场景（= A12 bar3020 假命中场景）**必须** bump
/// forest_epoch。复用 `cascade_reset_on_frontier_interior_rewrite` 的 v1→v2 fixture——v2 仅末段
/// end_price 147→140（组 C 外缘内点，L1 投影 bit-identical，L0 sub_moves 变）。这是 gen 快路当年
/// 假命中返陈旧森林的精确形态（TowerCache::generation 靠 l0_is_root blunt 兜底才没漏）。
///
/// 断言：喂 v1 后喂 v2，forest_epoch **严格递增**（若 epoch 漏 bump ⟹ 下游 TreeCache 假命中返
/// v1 陈旧森林 ⟹ bit-exact 破裂）。E1 逐值判据在此场景 [reuse..] 尾段值变（147→140）⟹ dirty。
#[test]
fn forest_epoch_bumps_on_l0_tail_redivision() {
    let cfg = ThetaConfig::default();
    let base = vec![
        seg(Direction::Up,   0,  4, 110, 150),
        seg(Direction::Down, 4,  8, 150, 120),
        seg(Direction::Up,   8, 12, 120, 148),
        seg(Direction::Down,12, 16, 115,  80),
        seg(Direction::Up,  16, 20,  80, 125),
        seg(Direction::Down,20, 24, 114,  85),
        seg(Direction::Up,  24, 28, 115, 148),
        seg(Direction::Down,28, 32, 148, 112),
        seg(Direction::Up,  32, 36, 112, 147), // v1 末段
    ];
    let mut closes: Vec<i64> = Vec::new();
    for i in 0..12 { closes.push(100 + if i % 2 == 0 { 40 } else { -40 }); }
    for i in 0..12 { closes.push(100 + if i % 2 == 0 {  5 } else {  -5 }); }
    for i in 0..16 { closes.push(100 + if i % 2 == 0 {  3 } else {  -3 }); }
    let merged = Rc::new(bars_from_closes(&closes));
    let layer_v1 = ParseLayer { segments: Rc::new(base.clone()), merged_bars: merged.clone(), ..Default::default() };
    let mut v2_segs = base.clone();
    v2_segs[8].end_price = 140; // L0 尾段重划（内点改写）——A12 bar3020 假命中场景。
    let layer_v2 = ParseLayer { segments: Rc::new(v2_segs), merged_bars: merged.clone(), ..Default::default() };

    let mut cache = TowerCache::new();
    let _ = classify_with_tower_incremental(&layer_v1, &cfg, &mut cache);
    let epoch_after_v1 = cache.forest_epoch();
    let _ = classify_with_tower_incremental(&layer_v2, &cfg, &mut cache);
    let epoch_after_v2 = cache.forest_epoch();
    assert!(
        epoch_after_v2 > epoch_after_v1,
        "L0 尾段重划（147→140）必须 bump forest_epoch（漏 bump ⟹ TreeCache 假命中返陈旧森林）：\
         v1_epoch={epoch_after_v1} v2_epoch={epoch_after_v2}"
    );
}

/// ★on2w2：无字节变更 bar（inclusion-only，L0 尾段值不变）**不** bump forest_epoch——O(n²) 消除
/// 的机制（bump 率贴近 forest 真变率而非每 bar）。喂完全相同的 layer 两次，第二次 epoch 不应变。
#[test]
fn forest_epoch_stable_on_no_change() {
    let cfg = ThetaConfig::default();
    let base = vec![
        seg(Direction::Up,   0,  4, 110, 150),
        seg(Direction::Down, 4,  8, 150, 120),
        seg(Direction::Up,   8, 12, 120, 148),
        seg(Direction::Down,12, 16, 115,  80),
        seg(Direction::Up,  16, 20,  80, 125),
        seg(Direction::Down,20, 24, 114,  85),
        seg(Direction::Up,  24, 28, 115, 148),
    ];
    let mut closes: Vec<i64> = Vec::new();
    for i in 0..28 { closes.push(100 + if i % 2 == 0 { 30 } else { -30 }); }
    let merged = Rc::new(bars_from_closes(&closes));
    let layer = ParseLayer { segments: Rc::new(base.clone()), merged_bars: merged.clone(), ..Default::default() };

    let mut cache = TowerCache::new();
    let _ = classify_with_tower_incremental(&layer, &cfg, &mut cache);
    let e1 = cache.forest_epoch();
    // 完全相同输入再喂一次——无任何塔字节变更 ⟹ epoch 不应 bump。
    let _ = classify_with_tower_incremental(&layer, &cfg, &mut cache);
    let e2 = cache.forest_epoch();
    assert_eq!(e1, e2, "无变更 bar 不应 bump forest_epoch（否则退化每 bar bump = O(n²) 未消除）：e1={e1} e2={e2}");
}

// ──────────────────────────────────────────────────────────────────────
//  标度验证（task #93：per-bar 累积成本，增量 vs 全量）
// ──────────────────────────────────────────────────────────────────────

/// ★合成标度：per-bar 段追加累积成本，增量 exp 显著 < 全量 exp。
///
/// 全量 `classify_with_tower` 每步从 0 重扫塔 ⟹ 累积 O(Σ i) ≈ O(N²)，exp≈2。
/// 增量 `classify_with_tower_incremental` 每步续扫 tail ⟹ 累积 O(Σ tail) ≈ O(N)，exp≈1。
/// 合成段序列单调追加（增量有效域）；此测试 always-run（无需真实数据）。
///
/// ## ⚠ 时间敏感（票 #619 L9 登记）
///
/// 本测试的判据是**墙钟时间比**（`ratio_at_max < 0.7`），因而对机器负载敏感：并行跑
/// 整个 `--lib` 套件、或机器同时在跑别的重活时，会偶发红（观测集群 ~0.55，余量 ~0.14）。
/// **隔离单跑恒绿**——复现红时的正确处置是
/// `cargo test --release --lib incremental_tower_scaling_dominates_full_synthetic`
/// 单独重跑确认，而不是改阈值（阈值的因果标定见下方长注释，`no-patch-mentality` 合规）。
///
/// 这是**该测试的固有属性**，不是回归信号，**与 #491**（`extract_signals_bit_exact_digest_guard`
/// 的确定性红）**无关**：#491 是字节摘要不符、恒红且与负载无关；本测试是负载相关的偶发红。
/// 此前该性质只散落在 4 份评审报告的自然语言里（`frontier-had-emitted-window-20260702.md:52`
/// 首次定性、`shadow-review-389-20260727.md:111`、`treasury-reverify-20260727.md:504`、
/// `shadow-603-review-20260728.md` §6），测试本体无注记 ⟹ 登记口径与 #491 不对称。本注记
/// 补齐该不对称的测试本体侧。
#[test]
fn incremental_tower_scaling_dominates_full_synthetic() {
    let cfg = ThetaConfig::default();
    let sizes = [100usize, 200, 400];
    let mut full_times = Vec::new();
    let mut inc_times = Vec::new();

    for &n in &sizes {
        let all_segments = synthetic_segments(n);
        let closes: Vec<i64> = (0..(n * 4 + 8) as i64).map(|i| 100 + (i % 7) * 4).collect();

        // 全量 per-bar 累积。
        let t0 = std::time::Instant::now();
        for k in 1..=n {
            let layer = ParseLayer {
                segments: Rc::new(all_segments[..k].to_vec()),
                merged_bars: Rc::new(bars_from_closes(&closes)),
                ..Default::default()
            };
            let _ = classify_with_tower(&layer, &cfg);
        }
        full_times.push(t0.elapsed().as_secs_f64());

        // 增量 per-bar 累积（cache 跨步复用）。
        let t0 = std::time::Instant::now();
        let mut cache = TowerCache::new();
        for k in 1..=n {
            let layer = ParseLayer {
                segments: Rc::new(all_segments[..k].to_vec()),
                merged_bars: Rc::new(bars_from_closes(&closes)),
                ..Default::default()
            };
            let _ = classify_with_tower_incremental(&layer, &cfg, &mut cache);
        }
        inc_times.push(t0.elapsed().as_secs_f64());
    }

    // exp 估计（log-log 斜率，sizes 翻倍）。
    let full_exp = (full_times[2] / full_times[0]).ln() / (sizes[2] as f64 / sizes[0] as f64).ln();
    let inc_exp = (inc_times[2] / inc_times[0]).ln() / (sizes[2] as f64 / sizes[0] as f64).ln();

    eprintln!(
        "\n===== 增量塔标度（合成 per-bar 累积）=====\n  \
         sizes={sizes:?}\n  full_times={full_times:?} (exp≈{full_exp:.2})\n  \
         inc_times={inc_times:?} (exp≈{inc_exp:.2})\n  \
         增量/全量比 @n={}: {:.2}x（越小增量越优）",
        sizes[2],
        inc_times[2] / full_times[2].max(1e-12)
    );

    // 增量须显著快于全量（MACD 增量 + 塔构造增量 + 走势分解增量 综合加速）。
    // ★判据：最大规模下增量/全量时间比 < 0.7（即增量至少 ~1.43x 加速）为稳健下界。
    // exp 差距在小规模 debug 噪声大（两者均 O(n²) 受限于 LevelState/tower_snapshots clone
    // 的 API 所需 O(k)/iter，故此合成尺度只能验证常数因子优势，asymptotic 分离须看
    // profile_incremental_tower_real_scaling 的真实大规模 #[ignore]）。此处验证常数因子：
    // 增量消除 MACD 全量重算 + 塔构造全量扫描。
    // 标度重标定（B4 / task#2，commit 254 改调 extract_signals_with_hist）：MACD 消重后
    // 全量只做 1×MACD（原 2×），增量相对优势从 >2x 收窄到 ~1.8x（ratio 实测集群
    // 0.543/0.548/0.559/0.55 across runs）。原阈值 0.5 按 full=2×MACD 标定，1×MACD 后需
    // 重标；取 0.7 为稳健下界（观测集群 ~0.55，留 ~0.14 机器噪声余量，仍断言真常数因子优势——
    // 若增量退化到无优势 ratio→1.0 则捕获）。这是因果重标定非「为绿改阈值」（no-patch 合规）。
    let ratio_at_max = inc_times[2] / full_times[2].max(1e-12);
    assert!(
        ratio_at_max < 0.7,
        "增量/全量比 @n={} = {ratio_at_max:.3} 须 < 0.7（增量至少 ~1.43x 加速；MACD+塔+分解增量）\n\
         full_exp≈{full_exp:.2}, inc_exp≈{inc_exp:.2}",
        sizes[2]
    );
}
