//! #1581（xianduan §15 X-37）：Rust 下游读段端门 fail-closed 标记的正负控。
//!
//! 正控（改前/改后对拍）：`tb1581_digest_*` 在随机 K 线上逐 bar 跑生产增量解析与增量分类、批量
//! 解析与全量分类，打印 SipHash 摘要。该测试先于实现单独提交，在基线与实现两个提交上各跑一次，
//! 摘要逐位相同 ⟹ 无门失败时下游产出与改前逐位相同。摘要只覆盖改前已有的字段。
use super::super::classifier::{self, streaming::OwnedIncrementalClassifier};
use super::super::types::{Bar, PendingTail, Stroke};
use super::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// 确定性随机游走 K 线（LCG），收盘价步长有正负，高低价包住开收。
pub(super) fn tb1581_bars(seed: u64, n: usize) -> Vec<Bar> {
    let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (state >> 33) as i64
    };
    let mut close = 10_000i64;
    let mut drift = 0i64;
    (0..n)
        .map(|i| {
            if i % 9 == 0 {
                drift = next() % 41 - 20;
            }
            let open = close;
            close = open + drift + next() % 13 - 6;
            let high = open.max(close) + next() % 15;
            let low = open.min(close) - next() % 15;
            Bar {
                source_index: i,
                timestamp: 60 * i as i64,
                open,
                high,
                low,
                close,
                volume: 1.0,
                untradable: false,
            }
        })
        .collect()
}

fn layer_text(l: &ParseLayer) -> String {
    format!(
        "{:?}|{}|{}|{:?}|{:?}|{:?}|{:?}|{:?}",
        l.merged_bars,
        l.merged_confirmed_len,
        l.segments_confirmed_len,
        l.segments_earliest_unsealed,
        l.fractals,
        l.strokes,
        l.segments,
        l.tail
    )
}

#[test]
#[ignore = "改前/改后对拍：在基线与实现两个提交上各跑一次，比较输出摘要"]
fn tb1581_digest_production_paths() {
    let cfg = ThetaConfig::default();
    let mut all = DefaultHasher::new();
    for seed in 0..6u64 {
        let bars = tb1581_bars(0x1581_0000 + seed, 4_000);
        let mut h = DefaultHasher::new();
        let mut incr = ParseLayerIncr::new(&cfg);
        let mut owned = OwnedIncrementalClassifier::new(cfg.clone());
        let mut segs = 0usize;
        let mut levels = 0usize;
        for (i, bar) in bars.iter().enumerate() {
            let layer = incr.append(*bar);
            layer_text(&layer).hash(&mut h);
            let out = owned.append_bar(*bar);
            format!("{out:?}").hash(&mut h);
            levels = out.classification.levels.len();
            segs = layer.segments.len();
            if i % 1_000 == 999 {
                let full = parse_layer(&bars[..=i], &cfg);
                layer_text(&full).hash(&mut h);
                format!("{:?}", classifier::classify(&full, &cfg, &[])).hash(&mut h);
                let (facts_layer, _) = parse_layer_with_inclusion_facts(&bars[..=i], &cfg);
                layer_text(&facts_layer).hash(&mut h);
            }
        }
        let d = h.finish();
        d.hash(&mut all);
        eprintln!("tb1581 digest seed={seed} segments={segs} levels={levels} digest={d:016x}");
    }
    eprintln!("tb1581 digest ALL={:016x}", all.finish());
}

fn tb1581_chain(prices: &[i64]) -> Vec<Stroke> {
    prices
        .windows(2)
        .enumerate()
        .map(|(i, p)| Stroke {
            direction: if p[1] > p[0] {
                super::super::types::Direction::Up
            } else {
                super::super::types::Direction::Down
            },
            start_index: 4 * i + 1,
            end_index: 4 * i + 5,
            start_price: p[0],
            end_price: p[1],
        })
        .collect()
}

/// 笔端点处的退化 K 线（只供分型/尾部层取数；段判定只读笔）。
fn tb1581_merged(ss: &[Stroke]) -> Vec<Bar> {
    let mut pts = vec![(ss[0].start_index, ss[0].start_price)];
    pts.extend(ss.iter().map(|s| (s.end_index, s.end_price)));
    pts.into_iter()
        .map(|(i, p)| Bar {
            source_index: i,
            timestamp: i as i64,
            open: p,
            high: p,
            low: p,
            close: p,
            volume: 1.0,
            untradable: false,
        })
        .collect()
}

fn has_pending_segment(l: &ParseLayer) -> bool {
    l.tail
        .iter()
        .any(|pt| matches!(pt, PendingTail::PendingSegment { .. }))
}

/// 负控：注入段端门失败（TestOnly 种子起点，#1493 `tb1493_gate_failure_is_fail_closed` 同一输入）
/// ⟹ ParseLayer 带标记、尾部无 PendingSegment、classifier 无产出；批量入口取标记的增量核与
/// facts 同值。正控：同一输入生产口径无失败 ⟹ 无标记、尾部照常、classifier 照常运行。
#[test]
fn tb1581_injected_gate_failure_blocks_downstream() {
    let cfg = ThetaConfig::default();
    let ss = tb1581_chain(&[0, 20, 8, 10, -20, -10, -16, -6, -8, -7, -12, -2, -9, 5, -5]);
    let merged = tb1581_merged(&ss);

    let (prod, prod_facts) = parse_layer_from_strokes(&merged, ss.clone(), &cfg.parse, &mut None);
    assert!(prod_facts.parse_invalid_from.is_none());
    assert_eq!(prod.segments_parse_invalid_from, None);
    let (segs, pending) = segment::divide_segments_with_tail(&ss, &cfg.parse);
    let fractals = fractal::detect_fractals(&merged);
    assert_eq!(
        *prod.tail,
        tail::build_tail(&merged, &fractals, &ss, &segs, pending, None)
    );

    segment::FIRST_START_AT_SEED.with(|c| c.set(true));
    let (bad, bad_facts) = parse_layer_from_strokes(&merged, ss.clone(), &cfg.parse, &mut None);
    let incr_marker = segment::IncrSegments::empty()
        .append(&ss, &cfg.parse)
        .parse_invalid_from();
    let (_, bad_pending) = segment::divide_segments_with_tail(&ss, &cfg.parse);
    segment::FIRST_START_AT_SEED.with(|c| c.set(false));

    let seg_start = bad_facts.parse_invalid_from.as_ref().map(|p| p.seg_start);
    assert_eq!(seg_start, Some(0), "注入须触发门失败");
    assert_eq!(bad.segments_parse_invalid_from, seg_start);
    assert_eq!(
        incr_marker, seg_start,
        "批量入口的增量核标记须与 facts 同一判定"
    );
    assert!(
        bad_pending.is_some(),
        "注入现场有未确认尾段（PendingSegment 的构造素材在）"
    );
    assert!(
        !has_pending_segment(&bad),
        "标记非空 ⟹ 不从 seg_start 起构造尾部"
    );
    assert_ne!(bad, prod, "标记计入 ParseLayer 结构等价");

    // 本输入注入后无已确认段，下句不证明拒读（无标记时同样无产出）；拒读的正控在热缓存测试。
    let out = classifier::classify(&bad, &cfg, &[1]);
    assert_eq!(out.classification, classifier::Classification::default());
    assert!(out.tower.is_empty());
    assert!(out.operations.is_empty());
}

/// 负控（classifier 入口，热缓存）：生产流中途某 bar 的 L0 带标记 ⟹ 该 bar 无中枢/走势/买卖点、
/// 无塔、无新候选事件；下一个无标记 bar 与全量 `classify` 逐位相同（缓存被清后全量重建）。
#[test]
fn tb1581_classifier_refuses_marked_layer_with_warm_cache() {
    let cfg = ThetaConfig::default();
    let bars = tb1581_bars(0x1581_0001, 2_000);
    let mut incr = ParseLayerIncr::new(&cfg);
    let mut cache = classifier::TowerCache::new();
    let inject_at = 1_700;
    let mut checked_after = false;
    for (i, bar) in bars.iter().enumerate() {
        let layer = incr.append(*bar);
        assert_eq!(layer.segments_parse_invalid_from, None, "生产口径无门失败");
        if i == inject_at {
            let warm = classifier::classify(&layer, &cfg, &[]);
            assert!(
                !warm.classification.levels.is_empty(),
                "正控：无标记时有产出"
            );
            let before = cache.candidate_streams();
            let mut marked = layer.clone();
            marked.segments_parse_invalid_from = Some(0);
            let out = classifier::classify_incremental(&marked, &cfg, &mut cache, &[1]);
            assert_eq!(out.classification, classifier::Classification::default());
            assert!(out.tower.is_empty());
            assert!(out.operations.is_empty());
            assert_eq!(out.candidate_streams, before, "拒读 bar 不产新候选事件");
            continue;
        }
        let out = classifier::classify_incremental(&layer, &cfg, &mut cache, &[]);
        if i == inject_at + 1 {
            let full = classifier::classify(&layer, &cfg, &[]);
            assert_eq!(out.classification, full.classification);
            assert_eq!(out.tower, full.tower);
            checked_after = true;
        }
    }
    assert!(checked_after);
}

/// 诊断：生产增量路径上出现标记的 bar 区间（不与批量入口交错，排除线程态串扰）。
#[test]
#[ignore = "诊断探针"]
fn tb1581_probe_marker_in_production() {
    let cfg = ThetaConfig::default();
    for seed in 0..6u64 {
        let bars = tb1581_bars(0x1581_0000 + seed, 4_000);
        let mut incr = ParseLayerIncr::new(&cfg);
        let mut runs: Vec<(usize, usize, Option<usize>)> = Vec::new();
        for (i, bar) in bars.iter().enumerate() {
            let layer = incr.append(*bar);
            if let Some(p) = layer.segments_parse_invalid_from {
                match runs.last_mut() {
                    Some(r) if r.1 + 1 == i && r.2 == Some(p) => r.1 = i,
                    _ => runs.push((i, i, Some(p))),
                }
            }
        }
        eprintln!("tb1581 probe seed={seed} marker_runs(bar_from,bar_to,seg_start)={runs:?}");
    }
}

/// 对拍补充：seed 4 逐 bar 摘要（生产增量解析 + 增量分类），在基线与实现上各跑一次逐行比对，
/// 定位差异是否只落在标记区间内。
#[test]
#[ignore = "改前/改后逐 bar 对拍"]
fn tb1581_perbar_digest_seed4() {
    let cfg = ThetaConfig::default();
    let bars = tb1581_bars(0x1581_0004, 4_000);
    let mut incr = ParseLayerIncr::new(&cfg);
    let mut owned = OwnedIncrementalClassifier::new(cfg.clone());
    for (i, bar) in bars.iter().enumerate() {
        let layer = incr.append(*bar);
        let out = owned.append_bar(*bar);
        let mut hl = DefaultHasher::new();
        layer_text(&layer).hash(&mut hl);
        let mut ho = DefaultHasher::new();
        format!("{out:?}").hash(&mut ho);
        eprintln!(
            "tb1581 perbar {i} layer={:016x} out={:016x} levels={}",
            hl.finish(),
            ho.finish(),
            out.classification.levels.len()
        );
    }
}

/// 负控（生产增量与批量装配路径，审查 must_fix）：TestOnly 种子起点在随机 K 线上逐 bar 驱动
/// `ParseLayerIncr`，取首个带标记的 bar 断言：增量/批量/facts 三路标记同值且非空、三层逐位相等、
/// 尾部无 PendingSegment 而同一层去掉标记时尾部有之（正控）、classifier 无产出。
/// （TestOnly 只改流首段起点，门失败时尚无已确认段；classifier 有产出的正控见热缓存测试。）
#[test]
fn tb1581_marker_reaches_incremental_and_batch_layers() {
    let cfg = ThetaConfig::default();
    let mut hit = None;
    segment::FIRST_START_AT_SEED.with(|c| c.set(true));
    'seeds: for seed in 0..40u64 {
        let bars = tb1581_bars(0x1581_1000 + seed, 1_500);
        let mut incr = ParseLayerIncr::new(&cfg);
        for (i, bar) in bars.iter().enumerate() {
            let layer = incr.append(*bar);
            let Some(p) = layer.segments_parse_invalid_from else {
                continue;
            };
            let batch = parse_layer(&bars[..=i], &cfg);
            let (facts_layer, facts) = parse_layer_with_inclusion_facts(&bars[..=i], &cfg);
            let open_tail = {
                let strokes = &layer.strokes;
                let (segs, pending) = segment::divide_segments_with_tail(strokes, &cfg.parse);
                tail::build_tail(
                    &layer.merged_bars,
                    &layer.fractals,
                    strokes,
                    &segs,
                    pending,
                    None,
                )
            };
            hit = Some((seed, i, p, layer, batch, facts_layer, facts, open_tail));
            break 'seeds;
        }
    }
    segment::FIRST_START_AT_SEED.with(|c| c.set(false));
    let (seed, i, p, layer, batch, facts_layer, facts, open_tail) =
        hit.expect("TestOnly 种子起点须在随机 K 线上触发门失败");
    eprintln!(
        "tb1581 incr marker seed={seed} bar={i} seg_start={p} strokes={} segments={}",
        layer.strokes.len(),
        layer.segments.len()
    );
    let facts_marker = facts
        .segment_facts
        .as_ref()
        .and_then(|f| f.parse_invalid_from.as_ref())
        .map(|x| x.seg_start);
    assert_eq!(facts_marker, Some(p));
    assert_eq!(batch.segments_parse_invalid_from, Some(p));
    assert_eq!(facts_layer.segments_parse_invalid_from, Some(p));
    assert_eq!(batch, layer, "批量 ≡ 增量（含标记）");
    assert_eq!(facts_layer, layer, "facts ≡ 增量（含标记）");
    assert!(
        open_tail
            .iter()
            .any(|pt| matches!(pt, PendingTail::PendingSegment { .. })),
        "正控：同一现场无标记时有 PendingSegment"
    );
    assert!(!has_pending_segment(&layer), "标记非空 ⟹ 无 PendingSegment");
    // 命中现场无已确认段，下句只锁「不出错、无产出」；拒读的正控在热缓存测试。
    let refused = classifier::classify(&layer, &cfg, &[1]);
    assert_eq!(
        refused.classification,
        classifier::Classification::default()
    );
    assert!(refused.tower.is_empty() && refused.operations.is_empty());
}
