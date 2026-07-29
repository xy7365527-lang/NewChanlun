use super::super::cand_delta::cache_series_ok;
use super::super::pipeline::segment_to_unit;
use super::super::tower_cache::{compute_macd_hist_incremental, update_closes_cache};
use super::super::*;
use super::super::super::parser::ParseLayer;
use super::super::super::types::Direction;
use super::{bars_from_closes, seg};

#[test]
fn p52_frontier_diagnostics_are_level_scoped_and_resettable() {
    cp_replay_diagnostics::enable();
    cp_replay_diagnostics::record_dirty_invalidation(2, 3, 5);
    cp_replay_diagnostics::record_tail_reinherit(2);
    cp_replay_diagnostics::record_tail_reinherit(1);

    let counters = cp_replay_diagnostics::snapshot();
    assert_eq!(counters[1].tail_reinherits, 1);
    assert_eq!(counters[2].pending_fallbacks, 3);
    assert_eq!(counters[2].tail_reinherits, 1);
    assert_eq!(counters[2].certificate_clear_recomputes, 5);

    cp_replay_diagnostics::disable();
    assert!(cp_replay_diagnostics::snapshot().is_empty());
}

/// ★#613（收 #609 F2）：段账本回缩 bar 上 `l0_units()` 与 `tower[0]` 必须仍同长。
///
/// 回归的是这条真 bug：回缩检测的 `cache.clear()` 曾位于 `l0_units_cache` 构建**之后**，
/// 于是该 bar 上访问器返回空而 `tower[0]` 满载（BTC 100k @ as_of=71040：0 vs 547）。
/// 消费方（`p123_fast_replay` 的 L2 活窗派生）拿空切片重扫，只在 `resume_from > 0` 时被
/// 越界守卫恰好接住；`resume_from == 0` 时会静默落 `no_window_formed`。
#[test]
fn l0_units_stays_in_sync_with_tower0_on_segment_ledger_shrink() {
    let cfg = ThetaConfig::default();
    let mut cache = TowerCache::new();

    // 第一 bar：6 段账本（confirmed 前缀 5，末段未确认——古怪线段可重划）。
    let long_segments = vec![
        seg(Direction::Up, 0, 4, 100, 150),
        seg(Direction::Down, 4, 8, 150, 120),
        seg(Direction::Up, 8, 12, 120, 148),
        seg(Direction::Down, 12, 16, 148, 110),
        seg(Direction::Up, 16, 20, 110, 145),
        seg(Direction::Down, 20, 24, 145, 115),
    ];
    let closes: Vec<i64> = (0..28).map(|i| 100 + if i % 2 == 0 { 20 } else { -20 }).collect();
    let long_layer = ParseLayer {
        segments: Rc::new(long_segments.clone()),
        segments_confirmed_len: 5,
        merged_bars: Rc::new(bars_from_closes(&closes)),
        ..Default::default()
    };
    let (_, tower_long) = classify_with_tower_incremental(&long_layer, &cfg, &mut cache);
    assert_eq!(
        cache.l0_units().len(),
        tower_long[0].len(),
        "非回缩 bar 本就同长"
    );

    // 第二 bar：段账本**回缩**到 4 段（末两段被重划吞并）⟹ 走 `cache.clear()` 分支。
    let short_layer = ParseLayer {
        segments: Rc::new(long_segments[..4].to_vec()),
        segments_confirmed_len: 3,
        merged_bars: Rc::new(bars_from_closes(&closes)),
        ..Default::default()
    };
    let (_, tower_short) = classify_with_tower_incremental(&short_layer, &cfg, &mut cache);

    assert!(!tower_short.is_empty(), "4 段仍足以产出 L0 塔快照");
    assert_eq!(
        tower_short[0].len(),
        4,
        "回缩后 tower[0] = 新段账本全量重建"
    );
    assert_eq!(
        cache.l0_units().len(),
        tower_short[0].len(),
        "★F2 不变式：回缩 bar 上 l0_units() 不得为空/失步（#613 收 #609 F2）"
    );
    // 同长之外再钉同源：逐元素等于新段账本的 `segment_to_unit` 投影。
    let expected: Vec<UnitRange> = short_layer.segments.iter().map(segment_to_unit).collect();
    assert_eq!(cache.l0_units(), expected.as_slice(), "同序同源，非仅同长");
}

/// ★批1（force_state 生产热路由 step4）：TowerCache 的 dif 增量通路 bit-exact 对拍全量。
///
/// 严格路线（Lead 裁定，拒绝「增量恒 None」降级）：`compute_macd_hist_incremental` 逐 bar 产出的
/// `macd_dif` 必与全量 `compute_macd(&closes[..k]).dif` **逐位相等**（非 tolerance——bit-exact 是
/// 断言，不是近似；tolerance 会把非 bit-exact 藏进容差 = 声明膨胀）。同证 `macd_hist`（dif/hist
/// 锁步的锚），并证 `closes_tick == merged_bars.close`（force 价格振幅 proxy 输入的整数往返）。
///
/// 覆盖：resume 增量路径（append-only 前缀稳定，confirmed_len=k-1）逐 bar 生长——每步驱动
/// truncate+append+tail 三段（含单 bar 分支 k=1）。dif 是 hist 子表达式（hist=dif-dea），hist
/// 增量已证 bit-exact（tower 测试锁 BspPoint）⟹ dif 同证；本测试直接坐实 dif 数组本身。
#[test]
fn incremental_macd_dif_and_closes_tick_bit_exact() {
    let cfg = super::super::super::config::MacdConfig::default();
    // 合成 closes：上升 + 震荡 + 下降（EMA 充分递推，覆盖 dif 正负峰）。整值 ⟹ closes_tick 往返精确。
    let vals: Vec<i64> = (0..90)
        .map(|i| 1000 + (30.0 * ((i as f64) * 0.3).sin()) as i64 + i as i64)
        .collect();
    let closes: Vec<f64> = vals.iter().map(|&v| v as f64).collect();

    // ── dif/hist 增量 vs 全量（逐 bar 生长，resume 增量路径）──
    let mut cache = TowerCache::new();
    for k in 1..=closes.len() {
        let prefix = &closes[..k];
        let confirmed = k.saturating_sub(1); // append-only：前 k-1 稳定，尾 bar 不稳定。
        compute_macd_hist_incremental(prefix, confirmed, &cfg, &mut cache);
        let full = divergence::compute_macd(prefix, &cfg);
        assert_eq!(cache.macd_dif(), full.dif.as_slice(), "bar {k}: dif 增量 ≠ 全量（bit-exact 破）");
        assert_eq!(cache.macd_hist_for_test(), full.hist.as_slice(), "bar {k}: hist 增量 ≠ 全量");
        assert_eq!(cache.macd_dif().len(), cache.macd_hist_for_test().len(), "dif/hist 锁步等长");
    }

    // ── closes_tick 增量 == merged_bars.close（整数域，force 振幅/速度 proxy 输入）──
    let bars = bars_from_closes(&vals);
    let mut cache2 = TowerCache::new();
    for k in 1..=bars.len() {
        update_closes_cache(&bars[..k], k.saturating_sub(1), &mut cache2);
        let expect: Vec<Tick> = bars[..k].iter().map(|b| b.close).collect();
        assert_eq!(cache2.closes_tick(), expect.as_slice(), "bar {k}: closes_tick ≠ merged_bars.close");
    }
}

/// parser BUG-04 回归：缓存守卫按真实覆盖契约校验——`compute_macd_hist_incremental`
/// 在 n≥2 时 `macd_state_len = n-1`（state 只覆盖稳定前缀，hist/dif 才含不稳定尾 bar），
/// 旧守卫 `== n` 恒假 ⟹ 增量缓存死代码、每 bar 退化全量 O(n²)。修复后逐 bar 驱动
/// 生产同源更新（update_closes_cache + compute_macd_hist_incremental），守卫必须命中；
/// over-invalidate 方向保持（空/不齐 cache 必不命中）。
#[test]
fn cand_cache_guard_accepts_incremental_contract() {
    let cfg = super::super::super::config::MacdConfig::default();
    let vals: Vec<i64> = (0..40).map(|i| 1000 + (i as i64 * 3) % 17).collect();
    let closes: Vec<f64> = vals.iter().map(|&v| v as f64).collect();
    let bars = bars_from_closes(&vals);
    let mut cache = TowerCache::new();
    for k in 1..=bars.len() {
        update_closes_cache(&bars[..k], k.saturating_sub(1), &mut cache);
        compute_macd_hist_incremental(&closes[..k], k.saturating_sub(1), &cfg, &mut cache);
        assert!(
            cache_series_ok(&cache, k),
            "bar {k}: 生产同源增量更新后守卫必须命中（BUG-04：旧 ==n 守卫在 k≥2 恒假）"
        );
    }
    // over-invalidate 方向保持：空 cache 对非空序列必不命中（退化全量，bit-exact）。
    assert!(!cache_series_ok(&TowerCache::new(), 5), "空 cache 必不命中（守卫仍 over-invalidate）");
    assert!(cache_series_ok(&TowerCache::new(), 0), "n=0：空 cache 与空序列自洽（与旧守卫同界）");
}
