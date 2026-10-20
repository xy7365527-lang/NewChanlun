//! #1581（xianduan §15 X-37）：Rust 下游读段端门 fail-closed 标记的正负控。
//!
//! 正控（改前/改后对拍）：`tb1581_digest_*` 在随机 K 线上逐 bar 跑生产增量解析与增量分类、批量
//! 解析与全量分类，打印 SipHash 摘要。该测试先于实现单独提交，在基线与实现两个提交上各跑一次，
//! 摘要逐位相同 ⟹ 无门失败时下游产出与改前逐位相同。摘要只覆盖改前已有的字段。
use super::super::classifier::{self, streaming::OwnedIncrementalClassifier};
use super::super::types::Bar;
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
