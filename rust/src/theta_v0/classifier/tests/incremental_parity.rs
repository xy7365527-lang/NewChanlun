//! #648 T1 主题件：incremental_parity（自 classifier/mod.rs 内联 `mod tests` 纯移动抽离，零行为变更）。

use super::super::super::parser::ParseLayer;
use super::super::super::types::Direction;
use super::super::*;
use super::fixtures::*;

#[test]
fn tower_cache_clear_preserves_candidate_history_and_resets_derived_cache() {
    let mut cache = TowerCache::new();
    let removed = cache_candidate(30, 35);
    let retained = cache_candidate(40, 45);
    cache
        .candidate_book
        .advance(&[removed.clone(), retained.clone()], 50);
    cache.last_l0_segments_len = 9;
    cache.macd_hist.push(1.0);

    cache.clear();
    assert_eq!(cache.last_l0_segments_len, 0);
    assert!(cache.macd_hist.is_empty());

    let mut grown = retained.clone();
    grown.interval.1 = 46;
    let delta = cache.candidate_book.advance(&[grown], 60);
    let invalidated = delta.iter().find(|event| event.key == removed.key).unwrap();
    let revised = delta
        .iter()
        .find(|event| event.key == retained.key)
        .unwrap();
    assert_eq!(invalidated.state, cand_event::CandidateState::Invalidated);
    assert_eq!(invalidated.revision, 1);
    assert_eq!(revised.revision, 1);
    assert_eq!(revised.observed_at, 50);
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
        assert_eq!(
            cache.macd_dif(),
            full.dif.as_slice(),
            "bar {k}: dif 增量 ≠ 全量（bit-exact 破）"
        );
        assert_eq!(
            cache.macd_hist_for_test(),
            full.hist.as_slice(),
            "bar {k}: hist 增量 ≠ 全量"
        );
        assert_eq!(
            cache.macd_dif().len(),
            cache.macd_hist_for_test().len(),
            "dif/hist 锁步等长"
        );
    }

    // ── closes_tick 增量 == merged_bars.close（整数域，force 振幅/速度 proxy 输入）──
    let bars = bars_from_closes(&vals);
    let mut cache2 = TowerCache::new();
    for k in 1..=bars.len() {
        update_closes_cache(&bars[..k], k.saturating_sub(1), &mut cache2);
        let expect: Vec<Tick> = bars[..k].iter().map(|b| b.close).collect();
        assert_eq!(
            cache2.closes_tick(),
            expect.as_slice(),
            "bar {k}: closes_tick ≠ merged_bars.close"
        );
    }
}

/// ★#613（收 #609 F2，#712 收 #645 MED-1 随入）：段账本回缩 bar 上 `level_scan_units(1)`
/// （L0 units 只读契约）与 `tower[0]` 必须仍同长。
///
/// 回归的是这条真 bug：回缩检测的 `cache.clear()` 曾位于 `l0_units_cache` 构建**之后**，
/// 于是该 bar 上访问器返回空而 `tower[0]` 满载。消费方（`p123_fast_replay` 的 L2 活窗派生）
/// 拿空切片重扫，只在 `resume_from > 0` 时被越界守卫恰好接住；`resume_from == 0` 时会静默
/// 落 `no_window_formed`。
#[test]
fn l0_units_stays_in_sync_with_tower0_on_segment_ledger_shrink() {
    let cfg = ThetaConfig::default();
    let mut cache = TowerCache::new();
    let (long_layer, short_layer) = segment_ledger_shrink_fixture();

    let __co1 = classify_incremental(&long_layer, &cfg, &mut cache, &[]);
    let tower_long = __co1.tower;
    assert_eq!(
        cache.level_scan_units(1).map(|u| u.len()),
        Some(tower_long[0].len()),
        "非回缩 bar 本就同长"
    );

    let __co2 = classify_incremental(&short_layer, &cfg, &mut cache, &[]);
    let tower_short = __co2.tower;

    assert!(!tower_short.is_empty(), "4 段仍足以产出 L0 塔快照");
    assert_eq!(
        tower_short[0].len(),
        4,
        "回缩后 tower[0] = 新段账本全量重建"
    );
    assert_eq!(
        cache.level_scan_units(1).map(|u| u.len()),
        Some(tower_short[0].len()),
        "★F2 不变式：回缩 bar 上 level_scan_units(1) 不得为空/失步（#613 收 #609 F2）"
    );
    // 同长之外再钉同源：逐元素等于新段账本的 `segment_to_unit` 投影。
    let expected: Vec<UnitRange> = short_layer.segments.iter().map(segment_to_unit).collect();
    assert_eq!(
        cache.level_scan_units(1),
        Some(expected.as_slice()),
        "同序同源，非仅同长"
    );
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
    assert!(
        !cache_series_ok(&TowerCache::new(), 5),
        "空 cache 必不命中（守卫仍 over-invalidate）"
    );
    assert!(
        cache_series_ok(&TowerCache::new(), 0),
        "n=0：空 cache 与空序列自洽（与旧守卫同界）"
    );
}

/// ★classify 的 `tower` 字段（逐级走势塔快照）——tower 非空 + depth≥1 真嵌套存在。
///
/// 9 段 L0 → 3 个 L1 走势 → L2 中枢（几何路径）：tower[1] 含 sub_moves 非空的
/// LeveledMove（RMove::Compose，depth=1 真嵌套）。坐实：导出桥正确产出真嵌套塔。
#[test]
fn classify_tower_depth_ge1_true_nesting() {
    let cfg = ThetaConfig::default();
    // 9 段：三组 up-down-up（每组 → 一个 L1 走势），三个 L1 走势外缘重叠成 L2 中枢。
    // ★task #142 延伸语义诚实重算（同 end_to_end_second_buy_via_l1_l2_geometric 推导）：三组
    // 核心分离（组B 首段 hi=115<ZD_A=120、组C 首段 lo=115>ZG_B=114 ⟹ non-extension，PDF §5
    // Step3），外缘 O_A=[110,150]/O_B=[80,125]/O_C=[112,148] 共同相交 ⟹ L2 核心 [112,125] 非空。
    let segments = vec![
        seg(Direction::Up, 0, 4, 110, 150),
        seg(Direction::Down, 4, 8, 150, 120),
        seg(Direction::Up, 8, 12, 120, 148),
        seg(Direction::Down, 12, 16, 115, 80),
        seg(Direction::Up, 16, 20, 80, 125),
        seg(Direction::Down, 20, 24, 114, 85),
        seg(Direction::Up, 24, 28, 115, 148),
        seg(Direction::Down, 28, 32, 148, 112),
        seg(Direction::Up, 32, 36, 112, 147),
    ];
    let mut closes: Vec<i64> = Vec::new();
    for i in 0..12 {
        closes.push(100 + if i % 2 == 0 { 40 } else { -40 });
    }
    for i in 0..12 {
        closes.push(100 + if i % 2 == 0 { 5 } else { -5 });
    }
    for i in 0..16 {
        closes.push(100 + if i % 2 == 0 { 3 } else { -3 });
    }
    let layer = ParseLayer {
        segments: Rc::new(segments),
        merged_bars: Rc::new(bars_from_closes(&closes)),
        ..Default::default()
    };
    let __co3 = classify(&layer, &cfg, &[]);
    let tower = __co3.tower;
    assert!(!tower.is_empty(), "tower 非空（至少 L0 级被处理）");
    assert!(
        tower.len() >= 2,
        "9 段 L0 → 3 个 L1 走势 → L2 中枢 ⟹ tower 至少 2 层"
    );
    // depth≥1 真嵌套：tower[1] 含 sub_moves 非空的 LeveledMove（RMove::Compose，L1 输入塔）。
    let has_true_nesting = tower[1].iter().any(|m| !m.sub_moves.is_empty());
    assert!(
        has_true_nesting,
        "tower[1] 含真嵌套 LeveledMove（sub_moves 非空，depth≥1）"
    );
}

/// #550 主缝②：修订富集的逐段因果重放，全历史重建与跨步增量事件簿逐字段相等。
#[test]
fn candidate_event_stream_per_segment_full_replay_equals_incremental() {
    let cfg = ThetaConfig::default();
    let all_segments = candidate_rich_segments(120);
    let closes: Vec<i64> = (0..=(all_segments.last().unwrap().end_index + 16))
        .map(|i| 100 + if i % 8 < 4 { 35 } else { -35 } + (i as i64 / 16))
        .collect();
    let mut steps = Vec::new();
    for n in 1..=all_segments.len() {
        let prefix = all_segments[..n].to_vec();
        steps.push(prefix.clone());
        if n >= 12 {
            let mut extended = prefix;
            let tail = extended.last_mut().unwrap();
            tail.end_index += 1;
            match tail.direction {
                Direction::Up => tail.end_price += 7,
                Direction::Down => tail.end_price -= 7,
            }
            steps.push(extended);
        }
    }
    let mut incremental_cache = TowerCache::new();
    let mut terminal = std::rc::Rc::new(Vec::new());

    for (step_idx, segments) in steps.iter().enumerate() {
        let end = segments.last().unwrap().end_index.min(closes.len() - 1);
        let layer = ParseLayer {
            segments: Rc::new(segments.clone()),
            merged_bars: Rc::new(bars_from_closes(&closes[..=end])),
            ..Default::default()
        };
        terminal =
            classify_incremental(&layer, &cfg, &mut incremental_cache, &[]).candidate_streams;

        let mut replay_cache = TowerCache::new();
        let mut replay_terminal = std::rc::Rc::new(Vec::new());
        for replay_segments in &steps[..=step_idx] {
            let replay_end = replay_segments
                .last()
                .unwrap()
                .end_index
                .min(closes.len() - 1);
            let replay_layer = ParseLayer {
                segments: Rc::new(replay_segments.clone()),
                merged_bars: Rc::new(bars_from_closes(&closes[..=replay_end])),
                ..Default::default()
            };
            replay_terminal =
                classify_incremental(&replay_layer, &cfg, &mut replay_cache, &[]).candidate_streams;
        }
        assert_eq!(
            terminal, replay_terminal,
            "step={step_idx}: 全历史重建≡增量事件簿"
        );
    }
    let identities: std::collections::BTreeSet<_> = terminal
        .iter()
        .flat_map(|stream| stream.iter())
        .map(|event| event.key)
        .collect();
    assert!(!identities.is_empty(), "事件身份流必须非空");

    let sample = terminal
        .iter()
        .flat_map(|stream| stream.iter())
        .next()
        .expect("非空锁");
    let mut book = cand_event::CandidateEventBook::default();
    let observation = cand_event::CandidateObservation {
        key: sample.key,
        kind: sample.kind,
        center_ids: sample.center_ids,
        candidate_group_id: sample.candidate_group_id,
        pair_id: sample.pair_id,
        structural_predicates: sample.structural_predicates,
        extreme_proof: sample.extreme_proof,
        third_class_proof: sample.third_class_proof,
        interval: sample.interval,
        state: cand_event::ObservedState::Provisional,
        first_provable_at: sample.first_provable_at,
        confirmed_at: None,
    };
    book.advance(std::slice::from_ref(&observation), sample.revision_at);
    let mut changed = observation;
    changed.pair_id ^= 1;
    assert_eq!(
        book.advance(&[changed], sample.revision_at + 1)[0].revision,
        1,
        "修订富集锁：右端不动而投影变化必须追加 revision"
    );
}

/// #550 FNV 锁真实 classify 产出，不锁手搓 book。
#[test]
fn candidate_event_stream_classify_fnv1a_golden() {
    let cfg = ThetaConfig::default();
    let segments = candidate_rich_segments(120);
    let closes: Vec<i64> = (0..=segments.last().unwrap().end_index)
        .map(|i| 100 + if i % 8 < 4 { 35 } else { -35 } + (i as i64 / 16))
        .collect();
    let layer = ParseLayer {
        segments: Rc::new(segments),
        merged_bars: Rc::new(bars_from_closes(&closes)),
        ..Default::default()
    };
    let streams = classify(&layer, &cfg, &[]).candidate_streams;
    assert!(streams.iter().any(|stream| !stream.is_empty()));
    let digest = format!("{streams:?}")
        .bytes()
        .fold(cand_event::FNV_OFFSET_BASIS, |hash, byte| {
            (hash ^ byte as u64).wrapping_mul(cand_event::FNV_PRIME)
        });
    // #551 诚实更新（旧值 3608191067574153658）：`CandidateKey` 增 `rule_version` 分量、
    // `first_provable_at` 由 `usize` 改 `Option<usize>`（未决期不落钟）、Trend 域四态映射上线
    // （Unresolved 生产可达）、同 episode 多腿归约为每 key 一条观察。
    // #898 诚实更新（旧值 3542680779063880892）：扩展支写全 + 本级盘背 lift==0 过滤——
    // 扩展折出的高一级盘整块（level_lift=1）不再触发本级盘整背驰路由，Pan 候选减少，
    // 事件流字节随之漂移（教义性收紧，非回归）。
    assert_eq!(
        digest, 3475352420846762134,
        "真实事件流漂移须诚实更新 golden"
    );
}

/// ★#754 链簿真平价（#670 复审 §3b R2-HIGH-4：重言式换真平价）：
/// 增量 `advance` 逐段喂 ≡ 全量自 ∅ 重放——两条**不同计算路径**互拍，禁同输入跑两遍。
///
/// **seam（平价锁判定公共接口）**：`ChainCertificateBook::advance`（增量簿推进）+
/// `classify_with_tower_events_incremental`（事件流生成），经 `chain_fixture(120)` 前缀序列
/// 两路驱动对拍 bit-exact。
///
/// 旧锁（#641 原版）把同一个预生成 `inputs` 列表（**事件流**）喂给驻留簿与重放簿两遍，
/// 等于 `f(x)≡f(x)`，结构上不可能失败（#670 §3b 点名）。本锁按 N1 先例
/// `candidate_event_stream_per_segment_full_replay_equals_incremental` 重写为两个**不同
/// 驱动**：
/// - **增量侧**：一个共享 `TowerCache` 跨前缀累积（逐段增量分类）+ 一个驻留簿逐段
///   `advance`；
/// - **全量侧**：每个前缀从 ∅ 起建 fresh `TowerCache` + fresh 簿，重放 `1..=n` 全部前缀
///   （O(n²) 从零重建）。
///
/// 两侧共享的只有**原始前缀几何**（`ParseLayer` 只构造一次）；事件流由各自路径**独立生成**
/// （共享 cache 单程 vs fresh cache 全程重放），簿由各自路径**独立推进**（驻留 vs 自 ∅ 重建）。
/// 若增量分类缓存泄漏状态、或簿跨步状态分叉，两侧产物不再 bit-exact，锁变红（红绿证据见
/// 实装报告摄动夹具节）。
///
/// **非真空锁（旧锁三条补偿锁原样保留）**：最终簿 certificates 非空、跨多个 `as_of` 落簿、
/// 至少一条证书带边。
#[test]
fn chain_certificate_book_incremental_equals_full_replay() {
    let cfg = ThetaConfig::default();
    let (segments, closes) = chain_fixture(120);

    // 原始前缀几何只构造一次（两侧共享的是输入，不是计算）。
    let mut prefixes = Vec::with_capacity(segments.len());
    for n in 1..=segments.len() {
        let end = segments[n - 1].end_index.min(closes.len() - 1);
        prefixes.push((
            ParseLayer {
                segments: Rc::new(segments[..n].to_vec()),
                merged_bars: Rc::new(bars_from_closes(&closes[..=end])),
                ..Default::default()
            },
            end,
        ));
    }

    // 增量路径：一个共享 cache（逐段增量分类）+ 一个驻留簿逐段推进。
    let mut incremental_cache = TowerCache::new();
    let mut incremental = chain_cert::ChainCertificateBook::default();
    let mut snapshots = Vec::with_capacity(prefixes.len());
    for (layer, end) in &prefixes {
        let streams =
            classify_incremental(layer, &cfg, &mut incremental_cache, &[]).candidate_streams;
        incremental.advance(&streams, *end);
        snapshots.push(incremental.clone());
    }

    // 全量路径：每个前缀从 ∅ 起，fresh cache 全程重放 + fresh 簿从第一步重建。
    for (step_index, snapshot_a) in snapshots.iter().enumerate() {
        let prefix_len = step_index + 1;
        let mut replay_cache = TowerCache::new();
        let mut replay_book = chain_cert::ChainCertificateBook::default();
        for (layer, end) in prefixes.iter().take(prefix_len) {
            let streams =
                classify_incremental(layer, &cfg, &mut replay_cache, &[]).candidate_streams;
            replay_book.advance(&streams, *end);
        }
        assert_eq!(
                snapshot_a, &replay_book,
                "prefix={prefix_len}: 增量（共享 cache + 驻留簿）≡ 全量自 ∅ 重放（fresh cache + fresh 簿）"
            );
    }

    let final_book = snapshots.last().expect("fixture 必须至少产生一个输入步骤");
    let certificates = final_book.certificates();
    let certificate_count = certificates.len();
    assert!(
        certificate_count > 0,
        "非真空锁：最终 book 的 certificates 必须非空；certificates={certificate_count}"
    );
    let distinct_as_of: std::collections::BTreeSet<usize> = certificates
        .iter()
        .map(|certificate| certificate.revision_at)
        .collect();
    assert!(
        distinct_as_of.len() > 1,
        "生命史锁：链簿必须跨多个 as_of 落簿；distinct_as_of={distinct_as_of:?} \
             certificates={certificate_count}"
    );
    let with_edges_count = certificates
        .iter()
        .filter(|certificate| !certificate.edges.is_empty())
        .count();
    assert!(
        with_edges_count > 0,
        "边非真空锁：至少一条 certificate 的 edges 非空；\
             with_edges={with_edges_count} certificates={certificate_count}"
    );
}

/// 因果簿驱动与终态窗口投影驱动都是 `chain_cert` 声明支持的输入语义
/// （见 `ChainNodeStatus::Absent` 文档）。#676-5（编排 2026-07-29 重裁）：`chain_fixture(40)`
/// 上曾实测到「terminal_projection ⊆ causal」，把这条巧合升格成了断言
/// （`causal_book_drive_is_a_superset_of_terminal_projection_drive`）；升到 `chain_fixture(120)`
/// 后子集关系当场破裂——`terminal_only` 非空（因果簿反而**漏**了 11 个终态投影侧独有的
/// key），说明超集关系不是规律，是 40 段夹具的规模偏差。
///
/// 归因追查（为何终态投影会出现因果簿没有的 key）另开 #681，不在本测试分辨力内。
///
/// **硬锁口径二次收窄（本轮实测新发现）**：先按「共有 key 的证书逐字段相同」起草硬锁，
/// 在 120 段夹具上实测**当场击穿**——20 个共有 key 里 4 个证书分叉，逐字段比对后分叉
/// 精确定位在 `edges[].skipped_levels[].alive_at_level` / `.inside_parent`（候选全集里
/// 该级别当场存活/落入父端点区间的候选计数），其余全部字段（含 `nodes`/`status`/
/// `revision_at`/边的 `parent`/`child`/`kind`/`predicate_holds`/`breach`）逐一相同。
/// 这两个数字统计的是候选全集，而候选全集本就由驱动决定（因果簿 append-only 累积、
/// 终态投影每步 fresh），两驱动在此项上天然不同——不是链簿重建被破坏，是统计口径
/// 引用了驱动相关的外部量。硬锁因此收窄为
/// [`certificates_agree_ignoring_alive_candidate_universe_counts`]：链身份/生命史/边拓扑
/// 逐字段相同，唯独候选全集计数不参与比对。
///
/// 双向差集（`causal_only` / `terminal_only`）不是不变量，只照实登记为本夹具上的
/// **实测形状**：`causal_only` 方向的语义差由 #551 裁定甲管；`terminal_only` 方向
/// （即本次发现的反常子集破裂）根因在查，见 #681 与报告锚
/// `chanlun/review-results/issue641-chain-dualpath-divergence-20260729.md`。
#[test]
fn causal_and_terminal_projection_drives_agree_on_common_chain_keys() {
    let cfg = ThetaConfig::default();
    let (segments, closes) = chain_fixture(120);
    let causal_book = chain_book_over_prefixes(
        &segments,
        &closes,
        &cfg,
        PrefixCacheReuse::SharedAcrossPrefixes,
    );
    let terminal_projection_book =
        chain_book_over_prefixes(&segments, &closes, &cfg, PrefixCacheReuse::FreshPerPrefix);

    let causal_heads = causal_book.heads();
    let terminal_projection_heads = terminal_projection_book.heads();

    // 硬锁（唯一不变量）：两驱动共有 key 的证书逐字段相同，唯独候选全集计数
    // （`alive_at_level` / `inside_parent`）豁免——理由见上方函数文档。
    let common_differences: Vec<_> = terminal_projection_heads
        .iter()
        .filter_map(|terminal| {
            let causal = causal_heads
                .iter()
                .find(|causal| causal.key == terminal.key)?;
            (!certificates_agree_ignoring_alive_candidate_universe_counts(causal, terminal))
                .then_some((*causal, *terminal))
        })
        .collect();
    assert!(
        common_differences.is_empty(),
        "共有 key 的最新 revision 证书在候选全集计数以外的字段分叉；\
             differences={common_differences:#?}"
    );

    // 照实登记（golden 锚，非规律断言）：`chain_fixture(120)` 上的实测差集形状。
    // 40 段时 terminal_only=0（超集关系「成立」）纯属规模巧合；120 段上子集关系不成立，
    // causal_only/terminal_only 双向计数按此固定，漂移即改证据、不悄悄放宽。
    let common_count = terminal_projection_heads
        .iter()
        .filter(|terminal| causal_heads.iter().any(|causal| causal.key == terminal.key))
        .count();
    let causal_only_count = causal_heads
        .iter()
        .filter(|causal| {
            !terminal_projection_heads
                .iter()
                .any(|terminal| terminal.key == causal.key)
        })
        .count();
    let terminal_only_count = terminal_projection_heads
        .iter()
        .filter(|terminal| !causal_heads.iter().any(|causal| causal.key == terminal.key))
        .count();
    // #898 诚实更新（旧值 (38, 31, 20) / causal_only 18）：扩展支写全 + 本级盘背 lift==0
    // 过滤 ⟹ Pan 候选身份总数收紧（教义性收紧，非回归）。
    assert_eq!(
        (
            causal_heads.len(),
            terminal_projection_heads.len(),
            common_count
        ),
        (33, 27, 16),
        "驱动身份总数 golden 漂移（causal, terminal_projection, common）"
    );
    assert_eq!(
        causal_only_count, 17,
        "causal_only 计数 golden 漂移（#551 甲管方向）"
    );
    assert_eq!(
        terminal_only_count, 11,
        "terminal_only 计数 golden 漂移（子集关系破裂方向，根因在查 #681）"
    );
}

/// ★#641 FNV golden + 非真空锁：链簿产出漂移当场变红，且覆盖探针证明真走过链路径。
#[test]
fn chain_certificate_book_classify_fnv1a_golden() {
    chain_cert::chain_probe::reset();
    let cfg = ThetaConfig::default();
    let (segments, closes) = chain_fixture(120);
    let book = chain_book_over_prefixes(
        &segments,
        &closes,
        &cfg,
        PrefixCacheReuse::SharedAcrossPrefixes,
    );

    let heads = book.heads();
    assert!(!heads.is_empty(), "非真空锁：链身份非空");
    assert!(
        heads
            .iter()
            .any(|certificate| !certificate.edges.is_empty()),
        "非真空锁：至少一条链带边（否则边侧全部判据未被触发）"
    );
    let probe = chain_cert::chain_probe::snapshot();
    assert!(probe.birth_open + probe.birth_closed > 0, "{probe:?}");
    assert!(
        probe.skip_edges > 0,
        "非真空锁：合成 classify 面上 skip 边真被走过（{probe:?}）"
    );
    assert_eq!(
        probe.birth_invalidated, 0,
        "构造口径锁：首次观察的链不可能一出生即 Invalidated（{probe:?}）"
    );
    // ★#641 `extends` 查簿分支在主缝上真被走到（非真空）：本合成夹具的三节点链一次成型，
    // 其结构真前缀从未作为极大路径落过簿 ⟹ 全部落「未物化」格、一条 `extends` 都不写。
    // 旧实装在这一格上照写不误（幽灵前缀）——本断言即那条修复的机器载体。
    assert!(
        probe.extends_not_materialized > 0,
        "非真空锁：extends 的查簿分支必须真被走过（{probe:?}）"
    );
    assert!(
        heads
            .iter()
            .all(|certificate| certificate.extends.is_none()),
        "本夹具上无一前缀物化 ⟹ extends 全空（{probe:?}）"
    );
    // 090 照实：本合成夹具**未覆盖**到的分支（`to_invalidated` / `fact_edges` /
    // `falsified_nodes` / `payload_revision`）由 `chain_cert::tests` 的语义锁逐条覆盖；
    // 此处不为凑覆盖率而断言它们非零（合成数据规整不是缺陷）。
    let pan_rooted = heads
        .iter()
        .filter(|certificate| certificate.key.root().kind == cand_event::CandidateKind::Pan)
        .count();

    // 摘要走库内口径 `ChainCertificateBook::digest()`（#641 修复轮从 B 侧移植的库内读数口）
    // ——golden 与诊断 bin 的读数因此不可能各算各的。
    let digest = book.digest();
    // 诚实更新（旧值 9771189513849272089）：#641 修复轮给 `TowerChainCertificate` 加了
    // `invalidation_cause`、给 `ChainEdge` 加了 `breach`，两者都进 `#[derive(Debug)]`；
    // 且 `extends` 由结构派生改为查簿命中（本夹具上原值即全是未物化前缀 ⟹ 现全部为 None）。
    // 三处都改写 Debug 字节流，摘要必然翻转。裁定锚：#641 comment-5121793896。
    // #898 诚实更新（旧值 9935805022530767834）：本级盘背 lift==0 过滤 ⟹ Pan 链身份减少
    // （birth_closed 38→33），链簿字节流随之漂移（教义性收紧，非回归）。
    assert_eq!(
            digest, 1019647215966295227,
            "链簿产出漂移须诚实更新 golden（probe={probe:?} certs={} heads={} pan_rooted={pan_rooted}）",
            book.certificates().len(),
            heads.len()
        );

    // ★地板条款监视格（#641 comment-5121572134）：合成 classify 面上 `Closed` 且零链段恒 0。
    let summary = book.summarize();
    assert_eq!(
        summary.closed_with_zero_segments, 0,
        "地板条款被绕过（summary={summary:?}）"
    );
    assert_eq!(
        summary.chains,
        heads.len(),
        "库内读数口与簿的 head 集合同源"
    );
}

/// ★#551 状态机全谱在 classify 全链上的生产可达锁：∅→Unresolved→Provisional，
/// 身份不变、右端生长、首证钟在转 Provisional 那一刻一次写入。
#[test]
fn classify_chain_walks_unresolved_to_provisional_with_growth_and_clock() {
    let cfg = ThetaConfig::default();
    let layer = lifecycle_rich_layer();
    let book = causal_book_over_prefixes(&layer, &cfg)
        .candidate_book
        .streams();
    let history: Vec<_> = book
        .iter()
        .flat_map(|stream| stream.iter())
        .filter(|event| event.kind == cand_event::CandidateKind::Trend)
        .collect();
    assert_eq!(history.len(), 2, "Trend 候选须走满两段生命史，禁真空绿");
    assert_eq!(history[0].key, history[1].key, "右端不入键 ⟹ 同一身份");

    assert_eq!(history[0].state, cand_event::CandidateState::Unresolved);
    assert!(
        !history[0].structural_predicates.extreme,
        "破核心未破包络极值"
    );
    assert_eq!(history[0].first_provable_at, None, "未决期不落首证钟");
    assert_eq!(history[0].revision, 0);

    assert_eq!(history[1].state, cand_event::CandidateState::Provisional);
    assert!(history[1].structural_predicates.extreme);
    assert_eq!(history[1].revision, 1);
    assert_eq!(history[1].supersedes_revision, Some(0));
    assert!(
        history[1].interval.1 > history[0].interval.1,
        "C 段右端生长（生产触发的生长修订）"
    );
    assert_eq!(
        history[1].first_provable_at,
        Some(history[1].interval.1),
        "首证钟 = 首次全谓词成立的结构位"
    );
    assert_eq!(
        history[1].observed_at, history[0].observed_at,
        "入簿钟不后移"
    );
}

/// ★#551 裁定(i) 投影等价锁 —— **甲口径定稿**（编排者 2026-07-28 裁决，非上浮态）。
///
/// 裁定(i) 原文「每 key 最新 revision 逐字段相等」已按出路甲收窄，本锁是该修订的机器载体：
/// - **无条件域 = 非终态**：因果簿中 `Provisional`/`Unresolved` 的每个 key，fresh-full 侧必
///   存在同 key 且 `payload_eq` 逐字段成立。零豁免。
/// - **计量域 = 终态**：`Confirmed`/`Invalidated` 的 key 上只数三类事实
///   （`terminal_equal` / `terminal_differ` / `terminal_absent_in_fresh`），不作相等断言。
///   理由：终态载荷在终态时刻**冻结**，而其上游证书仍随 bar 变，fresh-full 无状态、按当前
///   结构重判 ⟹ 终态域分叉是 E2E-O 终态语义历史相关性的**定义后果**，不是缺陷。
/// - **仍是硬断言**：「fresh 有而因果簿无该 key」。它不在终态域豁免范围内 —— 豁免的是
///   「因果簿判了终态、fresh 侧不再产出/载荷已变」这一个方向；反过来 fresh 侧凭空多出因果簿
///   从未记过的 key，说明增量宿主漏记，是实装缺陷而非语义后果。
///
/// 非真空三重前提：因果簿真含多 revision、fresh 非空、且 `live_checked > 0`（非终态域断言
/// 不得真空 —— 夹具若一个非终态 key 都没有，本锁等于没锁，必须红）。
///
/// **计量域的量度归属（诚实声明）**：合成夹具规整，终态计数可能全为 0，故本锁**不断言**它们
/// 非零（断言非零会把「合成数据规整」误报成缺陷）。真实数据的非真空量度由电池给出：
/// BTC 100k `payload_differ=25` 且 `payload_differ_live=0`（`ISSUE551_FORK` 行）。
#[test]
fn causal_book_terminal_projection_equals_fresh_full_stream() {
    let cfg = ThetaConfig::default();
    let layer = lifecycle_rich_layer();
    let fresh = latest_by_key(&classify(&layer, &cfg, &[]).candidate_streams);
    let book = causal_book_over_prefixes(&layer, &cfg)
        .candidate_book
        .streams();

    let revisions: usize = book.iter().map(|stream| stream.len()).sum();
    let terminal = latest_by_key(&book);
    assert!(
        revisions > terminal.len(),
        "非真空前提：因果簿须真含多 revision（revisions={revisions} identities={}）",
        terminal.len()
    );
    assert!(!fresh.is_empty(), "fresh 全量流非空，禁真空绿");

    // 无条件域（非终态）逐字段相等 + 计量域（终态）只数不断。遍历因果簿（正本）。
    let mut live_checked = 0usize;
    let mut terminal_equal = 0usize;
    let mut terminal_differ = 0usize;
    let mut terminal_absent_in_fresh = 0usize;
    for (key, booked) in &terminal {
        if booked.state.is_terminal() {
            match fresh.get(key) {
                None => terminal_absent_in_fresh += 1,
                Some(event) if payload_eq(booked, event) => terminal_equal += 1,
                Some(_) => terminal_differ += 1,
            }
            continue;
        }
        let event = fresh.get(key).unwrap_or_else(|| {
            panic!("非终态域无条件相等：因果簿有活候选而 fresh 缺席 {key:?}\n  因果簿 {booked:?}")
        });
        assert!(
            payload_eq(booked, event),
            "非终态域无条件相等（裁定(i) 甲口径）\n  因果簿 {booked:?}\n  fresh {event:?}"
        );
        live_checked += 1;
    }
    assert!(
        live_checked > 0,
        "非真空前提：非终态域断言不得真空（terminal_identities={} fresh_identities={}）",
        terminal.len(),
        fresh.len()
    );

    // 反方向仍是硬断言：fresh 侧不得出现因果簿从未记过的身份（漏记缺陷，非终态语义后果）。
    for (key, event) in &fresh {
        assert!(
                terminal.contains_key(key),
                "fresh 有而因果簿无该 key（因果簿是正本，此方向不在终态域豁免内）：{key:?}\n  fresh {event:?}"
            );
    }

    // 计量域事实（口径：只报数，不断言非零；见函数头量度归属）。
    assert_eq!(
        terminal_equal + terminal_differ + terminal_absent_in_fresh + live_checked,
        terminal.len(),
        "计量口径分解须覆盖因果簿全部身份：live={live_checked} \
             terminal_equal={terminal_equal} terminal_differ={terminal_differ} \
             terminal_absent_in_fresh={terminal_absent_in_fresh} \
             terminal_identities={} fresh_identities={}",
        terminal.len(),
        fresh.len()
    );
}

/// ★#551 裁定(i) 失效分叉锁 —— **甲口径定稿**（编排者 2026-07-28 裁决，非上浮态）。
///
/// 与 `causal_book_terminal_projection_equals_fresh_full_stream` 同一口径的失效侧：
/// - **无条件域 = 非终态禁向**：不存在这样的 key —— 因果簿中为 `Provisional`/`Unresolved`
///   而 fresh 侧缺席或载荷不等。**本夹具上该分支真空**（诚实声明）：L0 塌空后因果簿里的活
///   候选被当场判 `Invalidated`，故非终态集合为空，该断言在此缝上走不到。非真空由锁一的
///   `live_checked > 0` 承担。真空不是删掉它的理由 —— 口径分支必须在两枚锁上同构存在，
///   否则夹具一变（塌空前提被替换）就没有任何东西守住这个方向。
/// - **计量域 = 终态**：原「禁止方向（fresh 在产而因果簿已判终态 = 复活）必须为空」的**禁令
///   已按裁决取消**，改为计量 `revived_terminal`。复活型分叉是 E2E-O 终态语义历史相关性的
///   定义后果，不再是违规。本夹具（塌空后**未回长**）上它恒 0；回长场景的非零量度由
///   `escalated_upstream_regrowth_after_collapse_revival_fork_metered` 单独计量。
///   同域的 `fork_causal_only_terminal`（因果簿终态在案而 fresh 无该流）一并报数。
///
/// 非真空前提保留：塌空须真产生 `Invalidated`、塌空后 fresh 须为空 —— 二者保证本锁走的是
/// 真实的中途消失，而非空跑。
#[test]
fn invalidation_fork_only_points_from_causal_book_to_absent_fresh_stream() {
    let cfg = ThetaConfig::default();
    let layer = lifecycle_rich_layer();
    let mut cache = causal_book_over_prefixes(&layer, &cfg);
    let collapsed = ParseLayer {
        segments: Rc::new(Vec::new()),
        merged_bars: Rc::clone(&layer.merged_bars),
        ..Default::default()
    };
    classify_incremental(&collapsed, &cfg, &mut cache, &[]);
    let terminal = latest_by_key(&cache.candidate_book.streams());
    let fresh = latest_by_key(&classify(&collapsed, &cfg, &[]).candidate_streams);

    let dead: Vec<_> = terminal
        .values()
        .filter(|event| event.state == cand_event::CandidateState::Invalidated)
        .collect();
    assert!(!dead.is_empty(), "非真空前提：塌空须真产生 Invalidated");
    assert!(fresh.is_empty(), "塌空后 fresh-full 无任何候选流");

    // 无条件域：非终态禁向。本夹具上 live_scanned 恒 0（见函数头真空声明）。
    let mut live_scanned = 0usize;
    for (key, booked) in &terminal {
        if booked.state.is_terminal() {
            continue;
        }
        live_scanned += 1;
        let event = fresh.get(key).unwrap_or_else(|| {
            panic!("非终态禁向：因果簿有活候选而 fresh 缺席 {key:?}\n  因果簿 {booked:?}")
        });
        assert!(
            payload_eq(booked, event),
            "非终态禁向：活候选载荷须逐字段相等\n  因果簿 {booked:?}\n  fresh {event:?}"
        );
    }

    // 计量域（**非禁令**）：终态 key 在 fresh 侧的两种去向。
    let revived_terminal = fresh
        .keys()
        .filter(|key| {
            terminal
                .get(*key)
                .is_some_and(|event| event.state.is_terminal())
        })
        .count();
    let fork_causal_only_terminal = terminal
        .iter()
        .filter(|(key, event)| event.state.is_terminal() && !fresh.contains_key(*key))
        .count();
    // 计量口径分解（覆盖因果簿全部身份，报出实值）；本夹具未回长 ⟹ revived_terminal=0，
    // 但该 0 是**观测结果**，不是禁令 —— 回长场景下它为 1 且被上面点名的那枚锁计量。
    assert_eq!(
        live_scanned + revived_terminal + fork_causal_only_terminal,
        terminal.len(),
        "计量口径分解：live_scanned={live_scanned} revived_terminal={revived_terminal} \
             fork_causal_only_terminal={fork_causal_only_terminal} \
             terminal_identities={} fresh_identities={}",
        terminal.len(),
        fresh.len()
    );
}

/// ★#551：终态候选的载荷在确认时刻冻结 ⟹ 与 fresh-full 的当前重判必然分叉。
///
/// 本锁**不掩盖**该分叉，而是把它的边界机器化：差异只允许落在终态候选上（`Confirmed`/
/// `Invalidated`），非终态候选必须逐字段相等。Pan 域一入簿即 `Confirmed`（#550 决策），其
/// C 段区间此后仍随 bar 推进而变——这曾是 E2E-O「终态不改写」与裁定(i)「全 key 等价」在
/// 「终态候选上游载荷仍会变」下的不可弥合张力，**已裁决：甲**（编排者 2026-07-28，见
/// `chanlun/review-results/issue551-t2-impl-20260728.md` §五）。裁决把裁定(i) 收窄到非终态
/// 域，本锁的断言形状恰是甲口径本身 ⟹ 断言逐字未动，只是身份从「矛盾边界锁」变为
/// **甲口径的机器载体**。
///
/// **口径边界（诚实声明）**：本 lib 锁只固定不变式的**方向**（`differ_live == 0`）。合成
/// 夹具规整、终态候选的上游证书不再变动，`differ_terminal` 恒为 0，故该分支在本缝上真空。
/// 非真空量度由真实数据电池给出：BTC 100k 实测 `payload_differ=25` 且 `payload_differ_live=0`
/// （`ISSUE551_FORK` 行），即 25 个差异 100% 落在终态候选上。
#[test]
fn projection_divergence_is_confined_to_terminal_candidates() {
    let cfg = ThetaConfig::default();
    let layer = candidate_rich_layer();
    let fresh = latest_by_key(&classify(&layer, &cfg, &[]).candidate_streams);
    let terminal = latest_by_key(
        &causal_book_over_prefixes(&layer, &cfg)
            .candidate_book
            .streams(),
    );

    let mut differ_terminal = 0usize;
    let mut differ_live = 0usize;
    for (key, event) in &fresh {
        let Some(booked) = terminal.get(key) else {
            continue;
        };
        if payload_eq(booked, event) {
            continue;
        }
        if booked.state.is_terminal() {
            differ_terminal += 1;
        } else {
            differ_live += 1;
        }
    }
    assert_eq!(
        differ_live, 0,
        "非终态候选的投影必须逐字段相等（裁定(i) 在活假设域上无条件成立）"
    );
    // `differ_terminal` 在合成夹具上恒 0（见函数头口径边界），此处只报数不断言非零——
    // 断言它 > 0 会把「合成数据规整」误报成缺陷。真实数据的非零量度在电池 bin。
    assert!(
        differ_terminal < fresh.len(),
        "终态分叉不应吞掉全部身份（合成夹具期望 0，真实数据期望少数）"
    );
}

/// ★#551 复活型分叉的**计量锁**（甲口径定稿）。
///
/// **命名已按甲口径订正**（编排者 2026-07-28 裁决）：原上浮期名
/// `escalated_upstream_regrowth_after_collapse_forks_in_forbidden_direction` →
/// 现名 `…_revival_fork_metered`。该方向已不再是「禁止方向」，旧名与代码实际锁住的事实
/// 名实不符 = 声明膨胀（090 严格性），故改名，而非靠 doc 反向纠正。
/// 名册差集对账不靠「不改名」偿付，改由**显式声明 rename 非删增**偿付：改名 commit 的正文
/// 记录「删旧名 1 / 增新名 1、二者为同一测试的改名两端、测试体与断言消息逐字未动」，
/// 对账时按此把这一删一增抵消，不计入测试增删。
///
/// **改名与断言消息的不对称（诚实声明）**：函数名改了，下方断言消息中的「禁止方向」
/// 「矛盾已上浮」**逐字未改**。二者性质不同 —— 函数名是本测试对外的身份标识，不进入任何
/// 断言比较，改它只改称谓；断言消息是断言失败时打印的文本，属于本锁固定下来的现场记录，
/// 改动它会改变本锁锁住的事实边界（裁决要求断言逐字不动）。故：称谓按裁决后的甲口径订正，
/// 断言文本保持上浮期原貌，其定性一律以本 doc 为准 —— 消息里的「禁止方向」读作
/// 「原判为禁止、现判为计量的那个方向」。
///
/// 上游塌空后再回长到**逐字段相同**的结构时：因果簿按 E2E-O 判该 key 终态（`Invalidated`
/// 不复活），而 fresh-full 无状态、只看当下，会重新产出同一个 key —— 复活型分叉在机制上可达。
///
/// 根因不是实装缺陷，是两条规则的语义差：E2E-O 的终态语义**历史相关**（一旦终态永远终态），
/// fresh-full 的语义**历史无关**（只反映当前结构）。**已裁决：甲**（编排者 2026-07-28，报告
/// §五）——裁定(i) 收窄为非终态域逐字段相等；终态域（含本处的复活型分叉）是该语义差的
/// **定义后果**，接受并计量，不再作禁止断言。乙（改 Pan 状态映射）与丙（key 补上游代次）
/// 未被采纳，代价见报告 §五 对照表。
///
/// 于是本锁从「矛盾边界锁」转为**计量锁**：锁住的是该分叉恰落在终态域、数量恰为 1、且两侧
/// 状态如实可解释（因果簿 `Invalidated` / fresh `Provisional`）。断言逐字未动 —— 它锁的事实
/// 没变，变的只是这些事实的定性。
///
/// 生产可达性（保留）：BTC 100k 实测 `invalidations=0`，塌空—回长从未发生 ⟹ 该分叉当前
/// **生产不可达**。
#[test]
fn escalated_upstream_regrowth_after_collapse_revival_fork_metered() {
    let cfg = ThetaConfig::default();
    let layer = lifecycle_rich_layer();
    let mut cache = causal_book_over_prefixes(&layer, &cfg);
    let collapsed = ParseLayer {
        segments: Rc::new(Vec::new()),
        merged_bars: Rc::clone(&layer.merged_bars),
        ..Default::default()
    };
    classify_incremental(&collapsed, &cfg, &mut cache, &[]);
    classify_incremental(&layer, &cfg, &mut cache, &[]);

    let terminal = latest_by_key(&cache.candidate_book.streams());
    let fresh = latest_by_key(&classify(&layer, &cfg, &[]).candidate_streams);
    let revived: Vec<_> = fresh
        .keys()
        .filter(|key| {
            terminal.get(key).map(|event| event.state)
                == Some(cand_event::CandidateState::Invalidated)
        })
        .collect();
    assert_eq!(
        revived.len(),
        1,
        "锁定当前语义：塌空—回长恰产生 1 个禁止方向的分叉（矛盾已上浮，见函数头）"
    );
    assert_eq!(
        terminal[revived[0]].state,
        cand_event::CandidateState::Invalidated,
        "因果簿侧：终态不复活（E2E-O 成立）"
    );
    assert_eq!(
        fresh[revived[0]].state,
        cand_event::CandidateState::Provisional,
        "fresh 侧：历史无关，重新产出同一 key（裁定(i) 的禁止方向）"
    );
}

/// ★#551：L0 塌空的 bar 必须**当场**把在案活候选判 `Invalidated`，不推迟到下一非空 bar。
#[test]
fn empty_l0_bar_invalidates_live_candidates_without_delay() {
    let cfg = ThetaConfig::default();
    let layer = lifecycle_rich_layer();
    let mut cache = TowerCache::new();
    let live = classify_incremental(&layer, &cfg, &mut cache, &[]).candidate_streams;
    // 只有**非终态**候选会因观察缺席而失效；`Confirmed`/`Invalidated` 是终态，按 E2E-O
    // 不复活也不再改写。
    let live_keys: Vec<_> = latest_by_key(&live)
        .into_iter()
        .filter(|(_, event)| !event.state.is_terminal())
        .map(|(key, _)| key)
        .collect();
    assert!(!live_keys.is_empty(), "非真空前提：塌空前须有非终态活候选");

    cand_event::event_probe::reset();
    let collapsed = ParseLayer {
        segments: Rc::new(Vec::new()),
        merged_bars: Rc::clone(&layer.merged_bars),
        ..Default::default()
    };
    let after =
        latest_by_key(&classify_incremental(&collapsed, &cfg, &mut cache, &[]).candidate_streams);
    for key in &live_keys {
        assert_eq!(
            after.get(key).map(|event| event.state),
            Some(cand_event::CandidateState::Invalidated),
            "L0 塌空 ⟹ 候选身份消失 ⟹ 当场判终态（禁延迟到下一非空 bar）{key:?}"
        );
    }
    assert_eq!(
        cand_event::event_probe::snapshot().invalidated_absent as usize,
        live_keys.len(),
        "走的是既有「观察缺席 ⟹ Invalidated」单一路径"
    );
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
        let __co5 = classify(&layer, &cfg, &[]);
        let full_cls = __co5.classification;
        let full_tower = __co5.tower;
        // 增量（cache 跨步复用）。
        let __co6 = classify_incremental(&layer, &cfg, &mut cache, &[]);
        let inc_cls = __co6.classification;
        let inc_tower = __co6.tower;

        assert_eq!(
            inc_cls, full_cls,
            "n={n}: 增量 Classification != 全量（bit-exact 破裂）"
        );
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
    let layer_full = ParseLayer {
        segments: Rc::new(all_segments.clone()),
        merged_bars: Rc::new(bars_from_closes(&closes)),
        ..Default::default()
    };
    let _ = classify_incremental(&layer_full, &cfg, &mut cache, &[]);
    // 回缩到 8 段（parser 回撤）。
    let layer_shrink = ParseLayer {
        segments: Rc::new(all_segments[..8].to_vec()),
        merged_bars: Rc::new(bars_from_closes(&closes)),
        ..Default::default()
    };
    let __co7 = classify(&layer_shrink, &cfg, &[]);
    let full_cls = __co7.classification;
    let full_tower = __co7.tower;
    let __co8 = classify_incremental(&layer_shrink, &cfg, &mut cache, &[]);
    let inc_cls = __co8.classification;
    let inc_tower = __co8.tower;
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
        seg(Direction::Up, 0, 4, 110, 150),
        seg(Direction::Down, 4, 8, 150, 120),
        seg(Direction::Up, 8, 12, 120, 148),
        seg(Direction::Down, 12, 16, 115, 80),
        seg(Direction::Up, 16, 20, 80, 125),
        seg(Direction::Down, 20, 24, 114, 85),
        seg(Direction::Up, 24, 28, 115, 148),
        seg(Direction::Down, 28, 32, 148, 112),
        seg(Direction::Up, 32, 36, 112, 147),
    ];
    let mut closes: Vec<i64> = Vec::new();
    for i in 0..12 {
        closes.push(100 + if i % 2 == 0 { 40 } else { -40 });
    }
    for i in 0..12 {
        closes.push(100 + if i % 2 == 0 { 5 } else { -5 });
    }
    for i in 0..16 {
        closes.push(100 + if i % 2 == 0 { 3 } else { -3 });
    }
    let layer = ParseLayer {
        segments: Rc::new(segments),
        merged_bars: Rc::new(bars_from_closes(&closes)),
        ..Default::default()
    };

    let __co9 = classify(&layer, &cfg, &[]);
    let full_cls = __co9.classification;
    let mut cache = TowerCache::new();
    let __co10 = classify_incremental(&layer, &cfg, &mut cache, &[]);
    let inc_cls = __co10.classification;

    // 全量产 1 个 B2（见 end_to_end_second_buy_via_l1_l2_geometric），增量须 bit-identical。
    let full_b2: Vec<_> = full_cls.levels[1]
        .bsp
        .iter()
        .filter(|p| p.bits.buy2)
        .collect();
    let inc_b2: Vec<_> = inc_cls.levels[1]
        .bsp
        .iter()
        .filter(|p| p.bits.buy2)
        .collect();
    assert_eq!(
        inc_b2.len(),
        full_b2.len(),
        "增量塔 B2 数量 == 全量（真 Fugue 547 保留）"
    );
    assert_eq!(
        inc_b2.len(),
        1,
        "增量塔仍真产 B2（subs 真 Compose，非级别差伪造）"
    );
    assert_eq!(
        inc_b2[0].source_index, full_b2[0].source_index,
        "B2 source_index bit-exact"
    );
    assert_eq!(
        inc_cls, full_cls,
        "增量塔完整 Classification == 全量（含 B2 BSP）"
    );
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
        seg(Direction::Up, 0, 4, 110, 150),
        seg(Direction::Down, 4, 8, 150, 120),
        seg(Direction::Up, 8, 12, 120, 148),
        seg(Direction::Down, 12, 16, 115, 80),
        seg(Direction::Up, 16, 20, 80, 125),
        seg(Direction::Down, 20, 24, 114, 85),
        seg(Direction::Up, 24, 28, 115, 148),
        seg(Direction::Down, 28, 32, 148, 112),
        seg(Direction::Up, 32, 36, 112, 147), // v1 末段
    ];
    let mut closes: Vec<i64> = Vec::new();
    for i in 0..12 {
        closes.push(100 + if i % 2 == 0 { 40 } else { -40 });
    }
    for i in 0..12 {
        closes.push(100 + if i % 2 == 0 { 5 } else { -5 });
    }
    for i in 0..16 {
        closes.push(100 + if i % 2 == 0 { 3 } else { -3 });
    }
    let merged = Rc::new(bars_from_closes(&closes));

    // v1: 末段 end_price=147。
    let layer_v1 = ParseLayer {
        segments: Rc::new(base.clone()),
        merged_bars: merged.clone(),
        ..Default::default()
    };
    // v2: 仅末段 end_price 改写 147→140（组 C 外缘内点，L1 投影不变，L0 sub_moves 变）。
    let mut v2_segs = base.clone();
    v2_segs[8].end_price = 140;
    let layer_v2 = ParseLayer {
        segments: Rc::new(v2_segs),
        merged_bars: merged.clone(),
        ..Default::default()
    };

    // 前提自检（codex 反例成立的必要条件）：L1 投影输入 v1==v2 bit-identical（守卫看不到变异），
    // 但 L0 末段 sub_moves 已变（147→140）。若此前提不成立，本测试不构成反例。
    let mk_l1_units = |segs: &Rc<Vec<Segment>>| {
        let l0_units: Vec<UnitRange> = segs.iter().map(segment_to_unit).collect();
        let moves_l0: Vec<LeveledMove> = l0_units
            .iter()
            .enumerate()
            .map(|(i, u)| {
                LeveledMove::from_unit(
                    u,
                    recursive_tower::ElementId {
                        level: 0,
                        ordinal: i as u64,
                    },
                )
            })
            .collect();
        let (c, upper, _) = recursive_tower::compose_level(&l0_units, &moves_l0, true, 1);
        recursive_tower::project_to_units(&upper, &decompose::decompose(&c))
    };
    assert_eq!(
        mk_l1_units(&layer_v1.segments),
        mk_l1_units(&layer_v2.segments),
        "前提：L1 投影输入 v1==v2（守卫的本级投影比对看不到此变异）"
    );

    // 共享 cache：先喂 v1（缓存 L0/L1），再喂 v2（frontier 内点改写）——模拟 per-bar 末段重划。
    let mut cache = TowerCache::new();
    let _ = classify_incremental(&layer_v1, &cfg, &mut cache, &[]);
    let __co11 = classify_incremental(&layer_v2, &cfg, &mut cache, &[]);
    let inc_v2 = __co11.classification;
    let inc_tower_v2 = __co11.tower;
    let __co12 = classify(&layer_v2, &cfg, &[]);
    let full_v2 = __co12.classification;
    let full_tower_v2 = __co12.tower;

    // ★核心断言（latent 陈旧检测，非仅返回值）：cache 内 L1 深嵌套 sub_moves 末段坐标必须 == v2
    // 的 [112,140]。返回的 Classification/tower 不消费 cache 内 L1 upper_moves 的深 subs（tower 用
    // 新鲜 moves_tower 快照），故陈旧在返回值里 latent——但它喂 BSP（extract_second_for_level）+
    // 下一 bar 的 L2 投影。直接断言 cache 深 subs，捕获 latent 陈旧（640：不靠返回值碰巧相等）。
    // 推导（task #142 fixture）：v2 seg[8] = Up 112→140 ⟹ 区间 [112,140]（v1 为 [112,147]）。
    let l0_seg8_full = classify(&layer_v2, &cfg, &[]).tower[0]
        .last()
        .unwrap()
        .rmove
        .clone();
    assert_eq!(
        l0_seg8_full,
        descend::RMove::Segment {
            direction: Direction::Up,
            lo: 112,
            hi: 140
        },
        "前提：v2 全量 L0 末段 == [112,140]"
    );
    // cache.L1.upper_moves[0].sub_moves[2](groupC).sub_moves[2](seg[8]) 应 == [112,140]。
    let l1_deep = &cache.levels[1].upper_moves[0].sub_moves[2].sub_moves[2].rmove;
    assert_eq!(
        *l1_deep,
        descend::RMove::Segment {
            direction: Direction::Up,
            lo: 112,
            hi: 140
        },
        "cascade: cache L1 深嵌套 seg[8] == v2 [112,140]（陈旧则 [112,147]——L1 漏 cascade reset）"
    );

    // 返回值也须 bit-exact（cascade 后 L1 重建，tower/Classification 全对齐）。
    assert_eq!(inc_v2, full_v2, "cascade: v2 增量 Classification == 全量");
    assert_eq!(
        inc_tower_v2.len(),
        full_tower_v2.len(),
        "cascade: tower 层数 == 全量"
    );
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
        seg(Direction::Up, 0, 4, 110, 150),
        seg(Direction::Down, 4, 8, 150, 120),
        seg(Direction::Up, 8, 12, 120, 148),
        seg(Direction::Down, 12, 16, 115, 80),
        seg(Direction::Up, 16, 20, 80, 125),
        seg(Direction::Down, 20, 24, 114, 85),
        seg(Direction::Up, 24, 28, 115, 148),
        seg(Direction::Down, 28, 32, 148, 112),
        seg(Direction::Up, 32, 36, 112, 147), // v1 末段
    ];
    let mut closes: Vec<i64> = Vec::new();
    for i in 0..12 {
        closes.push(100 + if i % 2 == 0 { 40 } else { -40 });
    }
    for i in 0..12 {
        closes.push(100 + if i % 2 == 0 { 5 } else { -5 });
    }
    for i in 0..16 {
        closes.push(100 + if i % 2 == 0 { 3 } else { -3 });
    }
    let merged = Rc::new(bars_from_closes(&closes));
    let layer_v1 = ParseLayer {
        segments: Rc::new(base.clone()),
        merged_bars: merged.clone(),
        ..Default::default()
    };
    let mut v2_segs = base.clone();
    v2_segs[8].end_price = 140; // L0 尾段重划（内点改写）——A12 bar3020 假命中场景。
    let layer_v2 = ParseLayer {
        segments: Rc::new(v2_segs),
        merged_bars: merged.clone(),
        ..Default::default()
    };

    let mut cache = TowerCache::new();
    let _ = classify_incremental(&layer_v1, &cfg, &mut cache, &[]);
    let epoch_after_v1 = cache.forest_epoch();
    let _ = classify_incremental(&layer_v2, &cfg, &mut cache, &[]);
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
        seg(Direction::Up, 0, 4, 110, 150),
        seg(Direction::Down, 4, 8, 150, 120),
        seg(Direction::Up, 8, 12, 120, 148),
        seg(Direction::Down, 12, 16, 115, 80),
        seg(Direction::Up, 16, 20, 80, 125),
        seg(Direction::Down, 20, 24, 114, 85),
        seg(Direction::Up, 24, 28, 115, 148),
    ];
    let mut closes: Vec<i64> = Vec::new();
    for i in 0..28 {
        closes.push(100 + if i % 2 == 0 { 30 } else { -30 });
    }
    let merged = Rc::new(bars_from_closes(&closes));
    let layer = ParseLayer {
        segments: Rc::new(base.clone()),
        merged_bars: merged.clone(),
        ..Default::default()
    };

    let mut cache = TowerCache::new();
    let _ = classify_incremental(&layer, &cfg, &mut cache, &[]);
    let e1 = cache.forest_epoch();
    // 完全相同输入再喂一次——无任何塔字节变更 ⟹ epoch 不应 bump。
    let _ = classify_incremental(&layer, &cfg, &mut cache, &[]);
    let e2 = cache.forest_epoch();
    assert_eq!(
        e1, e2,
        "无变更 bar 不应 bump forest_epoch（否则退化每 bar bump = O(n²) 未消除）：e1={e1} e2={e2}"
    );
}

// ──────────────────────────────────────────────────────────────────────
//  标度验证（task #93：per-bar 累积成本，增量 vs 全量）
// ──────────────────────────────────────────────────────────────────────

/// ★合成标度：per-bar 段追加累积成本，增量（持久 TowerCache）显著 < 全量（空 TowerCache）。
///
/// #1053 后「全量」入口 = 唯一循环 + **空 TowerCache**（每步从 0 全扫 ⟹ 累积 O(Σ i) ≈ O(N²)）；
/// 「增量」入口 = 同一循环 + **持久 TowerCache**（confirmed_len 证书复用前缀 ⟹ 累积 O(N)）。
/// 合成段序列单调追加（增量有效域）；confirmed_len 按真实 ParseLayerIncr 语义填 k-1 /
/// closes.len()-1（全量入口持空 cache，证书不惠及它 ⟹ 仍每步全扫）。always-run（无需真实数据）。
#[test]
fn incremental_tower_scaling_dominates_full_synthetic() {
    let cfg = ThetaConfig::default();
    let sizes = [100usize, 200, 400];
    let mut full_times = Vec::new();
    let mut inc_times = Vec::new();

    for &n in &sizes {
        let all_segments = synthetic_segments(n);
        let closes: Vec<i64> = (0..(n * 4 + 8) as i64).map(|i| 100 + (i % 7) * 4).collect();

        // 全量 per-bar 累积（空 cache ⟹ 每步全扫）。
        let t0 = std::time::Instant::now();
        for k in 1..=n {
            let layer = ParseLayer {
                segments: Rc::new(all_segments[..k].to_vec()),
                merged_bars: Rc::new(bars_from_closes(&closes)),
                segments_confirmed_len: k.saturating_sub(1),
                merged_confirmed_len: closes.len().saturating_sub(1),
                ..Default::default()
            };
            let _ = classify(&layer, &cfg, &[]);
        }
        full_times.push(t0.elapsed().as_secs_f64());

        // 增量 per-bar 累积（持久 cache 跨步复用 confirmed_len 前缀）。
        let t0 = std::time::Instant::now();
        let mut cache = TowerCache::new();
        for k in 1..=n {
            let layer = ParseLayer {
                segments: Rc::new(all_segments[..k].to_vec()),
                merged_bars: Rc::new(bars_from_closes(&closes)),
                segments_confirmed_len: k.saturating_sub(1),
                merged_confirmed_len: closes.len().saturating_sub(1),
                ..Default::default()
            };
            let _ = classify_incremental(&layer, &cfg, &mut cache, &[]);
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

    // 增量（持久 cache + confirmed_len 证书）须显著快于全量（空 cache 每步全扫）——
    // 这是 #1053 收口后唯一循环两种驱动（空 cache vs 持久 cache）的性能分离判据：
    // 若增量退化到与全量同速（ratio→1.0）则持久 cache 的证书复用失效，捕获之。
    // ★判据：最大规模下增量/全量时间比 < 0.7（即增量至少 ~1.43x 加速）为稳健下界
    // （小规模 debug 噪声大；asymptotic 分离的完整验证见 #[ignore] 真实大规模 profile）。
    let ratio_at_max = inc_times[2] / full_times[2].max(1e-12);
    assert!(
        ratio_at_max < 0.7,
        "增量/全量比 @n={} = {ratio_at_max:.3} 须 < 0.7（持久 cache 证书复用至少 ~1.43x 加速）\n\
             full_exp≈{full_exp:.2}, inc_exp≈{inc_exp:.2}",
        sizes[2]
    );
}

/// ★#902 e2e 对拍锁：增量塔挂载操作级别（`classify_with_tower_incremental_operations`）
/// 的口径 S 操作序列 == 全量 `classify_with_operations` 逐字段（frontier 逐段生长含
/// pop/回卷的真实路径——经逐 bar 截断喂入模拟 per-bar resume）。
#[test]
fn incremental_operation_bypass_matches_full() {
    let cfg = ThetaConfig::default();
    for n in [6usize, 9, 12, 18] {
        let segments = synthetic_segments(n);
        let closes: Vec<i64> = (0..(n * 4 + 8) as i64).map(|i| 100 + (i % 5) * 5).collect();
        let bars = bars_from_closes(&closes);
        // 全量对照：整段一次算。
        let layer_full = ParseLayer {
            segments: Rc::new(segments.clone()),
            merged_bars: Rc::new(bars.clone()),
            ..Default::default()
        };
        let __co13 = classify(&layer_full, &cfg, &[1]);
        let _fc = __co13.classification;
        let full_ops = __co13.operations;
        // 增量：逐段截断喂入（每步 = 一个 resume bar）。
        let mut cache = TowerCache::new();
        let mut last_ops = Vec::new();
        for cut in 3..=n {
            let layer = ParseLayer {
                segments: Rc::new(segments[..cut].to_vec()),
                merged_bars: Rc::new(bars.clone()),
                ..Default::default()
            };
            let __co1 = classify_incremental(&layer, &cfg, &mut cache, &[1]);
            let _ic = __co1.classification;
            let _it = __co1.tower;
            let ops = __co1.operations;
            last_ops = ops;
        }
        assert_eq!(
            format!("{last_ops:?}"),
            format!("{full_ops:?}"),
            "n={n}: 增量逐段 resume 的 L1 操作序列 == 全量逐字段"
        );
    }
}
/// ★#1019 MED-1 锁（cascade reset 两支 + L0 挂载覆盖）：frontier 改写触发 cascade 后，
/// 挂载级别的增量操作序列仍 == 全量逐字段——真实执行 `pipeline.rs` 的
/// `operation_state = Default::default()` 同批复位路径（评审指证：旧三锁对该路径零执行
/// 覆盖；本锁探针实测 P=0/P>0 两支均命中）。
///
/// 照实登记（#1019 复核）：该复位行是**保守防御**而非承重——`operation_decompose_resume`
/// 自身的 `dirty_from` 失效分支已保守覆盖（cascade bar 上 dirty_from 回退 ⟹ 读域越界窗口
/// 全弹 + k=0 清块链 + 重扫），删除复位行本锁仍绿。本锁锁的是「cascade 后输出逐字段 ==
/// 全量」这一行为不变式，复位行作为同 key 守卫纪律保留（与 #885 同型）。
///
/// 覆盖矩阵：
/// - v1→v2：末段内点改写（147→140，同 `cascade_reset_on_frontier_interior_rewrite` 夹具）
///   ⟹ L0 frontier_mutated（j_min=8，e=32）走 **P>0 前缀失效支**；cascade 传播 L1 复位；
/// - v2→v3：首段内点改写（150→145）⟹ j_min=0、e=0 ⟹ L0/L1 均走 **P=0 全清支**；
/// - v3→v4：复位后追加一段 ⟹ 复位后 resume 续扫仍逐字段一致；
/// - `operating_levels=[0,1]`：L0 挂载（segments_confirmed_len=0 ⟹ 每 bar dirty_from=0，
///   即 HIGH-1(a) 的 k=0 全清路径的 e2e 常态覆盖）+ L1 挂载。
#[test]
fn incremental_operation_bypass_matches_full_after_cascade_reset() {
    let cfg = ThetaConfig::default();
    // 夹具同 cascade_reset_on_frontier_interior_rewrite（三组核心分离，task #142）。
    let base = vec![
        seg(Direction::Up, 0, 4, 110, 150),
        seg(Direction::Down, 4, 8, 150, 120),
        seg(Direction::Up, 8, 12, 120, 148),
        seg(Direction::Down, 12, 16, 115, 80),
        seg(Direction::Up, 16, 20, 80, 125),
        seg(Direction::Down, 20, 24, 114, 85),
        seg(Direction::Up, 24, 28, 115, 148),
        seg(Direction::Down, 28, 32, 148, 112),
        seg(Direction::Up, 32, 36, 112, 147),
    ];
    let mut closes: Vec<i64> = Vec::new();
    for i in 0..12 {
        closes.push(100 + if i % 2 == 0 { 40 } else { -40 });
    }
    for i in 0..12 {
        closes.push(100 + if i % 2 == 0 { 5 } else { -5 });
    }
    for i in 0..20 {
        closes.push(100 + if i % 2 == 0 { 3 } else { -3 });
    }
    let merged = Rc::new(bars_from_closes(&closes));
    let mk_layer = |segs: Vec<Segment>| ParseLayer {
        segments: Rc::new(segs),
        merged_bars: Rc::clone(&merged),
        ..Default::default()
    };

    // v2：末段内点改写（组 C 外缘内点，L1 投影不变、L0 sub_moves 变——frontier_mutated）。
    let mut v2_segs = base.clone();
    v2_segs[8].end_price = 140;
    // v3：首段内点改写（e=0 ⟹ P=0 全清支，两级同）。
    let mut v3_segs = base.clone();
    v3_segs[0].end_price = 145;
    // v4：v3 末追加一段（复位后 resume 续扫）。
    let mut v4_segs = v3_segs.clone();
    v4_segs.push(seg(Direction::Down, 36, 40, 147, 118));

    let versions = [base, v2_segs, v3_segs, v4_segs];
    let mut cache = TowerCache::new();
    for (step, segs) in versions.into_iter().enumerate() {
        let layer = mk_layer(segs);
        let __co2 = classify_incremental(&layer, &cfg, &mut cache, &[0, 1]);
        let _ic = __co2.classification;
        let _it = __co2.tower;
        let inc_ops = __co2.operations;
        let __co14 = classify(&layer, &cfg, &[0, 1]);
        let _fc = __co14.classification;
        let full_ops = __co14.operations;
        assert_eq!(
            format!("{inc_ops:?}"),
            format!("{full_ops:?}"),
            "step {step}（v1 基线 / v2 P>0 cascade / v3 P=0 cascade / v4 复位后续扫）：\
         增量 L0+L1 操作序列 == 全量逐字段"
        );
    }
}
