// #1467研究探针：合成显示队列更新 → 精确数量加权报价 → 既有解析模块。
// 不做交易、不声称语义唯一性或完整生产入口验收。直接链接现有生产crate。
use newchan_rust::theta_v0::classifier::center::{center_from_segments, UnitRange};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser;
use newchan_rust::theta_v0::types::Bar;

#[derive(Clone, Copy, Debug)]
struct QueueState {
    bid_qty: i64,
    ask_qty: i64,
}

const BID: i64 = 10_000;
const ASK: i64 = 10_200;

fn weighted_tick(s: QueueState) -> i64 {
    assert!(s.bid_qty > 0 && s.ask_qty > 0);
    let num = ASK as i128 * s.bid_qty as i128 + BID as i128 * s.ask_qty as i128;
    let den = s.bid_qty as i128 + s.ask_qty as i128;
    // 此研究输入全正，精确有理数舍入，半值向上；不是Rust f64量化路径对拍。
    let p = ((2 * num + den) / (2 * den)) as i64;
    assert!((BID..=ASK).contains(&p));
    p
}

fn bars(prices: &[i64], repetitions: usize) -> Vec<Bar> {
    let mut out = Vec::new();
    for &p in prices {
        for _ in 0..repetitions {
            let i = out.len();
            out.push(Bar {
                source_index: i,
                timestamp: i as i64,
                open: p,
                high: p,
                low: p,
                close: p,
                volume: 0.0,
                untradable: true,
            });
        }
    }
    out
}

fn generated_states() -> Vec<QueueState> {
    // 固定报价；每次只更新一侧数量。无订单身份/排队模型的合成队列域。
    let pivots = [
        25, 155, 65, 185, 45, 135, 15, 175, 55, 145, 35, 165, 75, 195, 85, 125,
    ];
    let mut state = QueueState {
        bid_qty: pivots[0],
        ask_qty: 200 - pivots[0],
    };
    let mut states = vec![state];
    for cycle in 0..4 {
        for j in 0..pivots.len() {
            if cycle == 0 && j == 0 {
                continue;
            }
            let from = state.bid_qty;
            let target = pivots[j];
            for k in 1..=6 {
                let q = from + (target - from) * k / 6;
                if q != state.bid_qty {
                    state.bid_qty = q;
                    states.push(state);
                }
                if 200 - q != state.ask_qty {
                    state.ask_qty = 200 - q;
                    states.push(state);
                }
            }
        }
    }
    states
}

fn describe(name: &str, input: &[Bar], cfg: &ThetaConfig, check_prefixes: bool) -> (usize, usize) {
    let layer = parser::parse_layer(input, cfg);
    let mut checked = 0;
    let mut certified_len = 0;
    if check_prefixes {
        let mut incremental = parser::ParseLayerIncr::new(cfg);
        let mut last_segments = Vec::new();
        for (i, &bar) in input.iter().enumerate() {
            let got = incremental.append(bar);
            let batch = parser::parse_layer(&input[..=i], cfg);
            assert_eq!(got, batch, "{name}: prefix {} differs", i + 1);
            assert!(got.segments.len() >= certified_len);
            assert_eq!(
                &got.segments[..certified_len],
                &last_segments[..certified_len],
                "{name}: previously certified segment prefix changed at {}",
                i + 1
            );
            certified_len = got.segments_confirmed_len;
            assert!(certified_len <= got.segments.len());
            last_segments = got.segments.to_vec();
            checked += 1;
        }
    }
    let ranges: Vec<_> = layer
        .segments
        .iter()
        .take(certified_len)
        .map(|s| UnitRange {
            start_index: s.start_index,
            end_index: s.end_index,
            direction: s.direction,
            lo: s.start_price.min(s.end_price),
            hi: s.start_price.max(s.end_price),
        })
        .collect();
    let centers: Vec<_> = ranges
        .windows(3)
        .filter_map(|w| center_from_segments(&w[0], &w[1], &w[2]))
        .collect();
    println!("case={name} bars={} merged={} fractals={} strokes={} segments={} certified_segments={certified_len} certified_center_windows={} checked_prefixes={checked}",
        input.len(),layer.merged_bars.len(),layer.fractals.len(),layer.strokes.len(),layer.segments.len(),centers.len());
    if let Some(s) = layer.segments.first() {
        println!("first_segment={s:?}");
    }
    if let Some(c) = centers.first() {
        println!("first_center_window={c:?}");
    }
    (certified_len, centers.len())
}

fn main() {
    let cfg = ThetaConfig::default();
    println!("scope=synthetic_queue_projection_module_probe min_gap={} metadata_volume=0 untradable=true", cfg.parse.new_stroke_min_gap);
    let states = generated_states();
    let weighted: Vec<_> = states.iter().copied().map(weighted_tick).collect();
    let mids = vec![(BID + ASK) / 2; states.len()];
    let witness = describe("quantity_projection", &bars(&weighted, 1), &cfg, true);
    let control = describe("unchanged_quotes_midpoint", &bars(&mids, 1), &cfg, true);
    assert!(
        witness.0 > 0 && witness.1 > 0,
        "nonempty structural witness missing"
    );
    assert_eq!(control, (0, 0));
    // 单独检查观察时钟的影响，不将插入重复观察冒称等价的真实订单事件。
    let short = [
        25, 155, 65, 185, 45, 135, 15, 175, 55, 145, 35, 165, 75, 195, 85, 125,
    ];
    let point_prices: Vec<_> = short.iter().map(|q| BID + q).collect();
    describe("short_clock_once", &bars(&point_prices, 1), &cfg, true);
    describe("short_clock_repeat4", &bars(&point_prices, 4), &cfg, true);
}
