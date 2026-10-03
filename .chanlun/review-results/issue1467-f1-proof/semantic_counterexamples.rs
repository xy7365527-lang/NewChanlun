// #1467：以有效单侧数量更新检验R_W复用入口的具名语义条件。
// 本程序记录反例，不修生产、不把反例计数解释为收益或全市场发生率。
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser;
use newchan_rust::theta_v0::types::Bar;

const BID: i128 = 10_000;
const ASK: i128 = 10_200;

fn observed(qb: i128, qa: i128) -> i64 {
    assert!(qb > 0 && qa > 0);
    let n = ASK * qb + BID * qa;
    let d = qb + qa;
    ((2 * n + d) / (2 * d)) as i64
}

fn make_bars(qb: &[i128], qa: i128) -> Vec<Bar> {
    assert!(qb.windows(2).all(|pair| pair[0] != pair[1]));
    qb.iter()
        .enumerate()
        .map(|(i, &q)| {
            let p = observed(q, qa);
            Bar {
                source_index: i,
                timestamp: i as i64,
                open: p,
                high: p,
                low: p,
                close: p,
                volume: 0.0,
                untradable: true,
            }
        })
        .collect()
}

fn evaluate(name: &str, bars: &[Bar]) -> (usize, usize) {
    let cfg = ThetaConfig::default();
    let p = parser::parse_layer(bars, &cfg);
    let mut incremental = parser::ParseLayerIncr::new(&cfg);
    for (i, &bar) in bars.iter().enumerate() {
        assert_eq!(
            incremental.append(bar),
            parser::parse_layer(&bars[..=i], &cfg)
        );
    }
    // 独立按两端分型在合并序列上的三根窗口求交，不复制gap_ok。
    let mut shared = Vec::new();
    for s in p.strokes.iter() {
        let left = p
            .merged_bars
            .iter()
            .position(|b| b.source_index == s.start_index)
            .unwrap();
        let right = p
            .merged_bars
            .iter()
            .position(|b| b.source_index == s.end_index)
            .unwrap();
        assert!(left > 0 && right + 1 < p.merged_bars.len());
        if left + 1 >= right - 1 {
            shared.push((s.start_index, s.end_index, left, right));
        }
    }
    let disconnected: Vec<_> = p
        .strokes
        .windows(2)
        .filter(|s| s[0].end_index != s[1].start_index || s[0].end_price != s[1].start_price)
        .collect();
    println!("case={name} bars={} merged={} fractals={} strokes={} segments={} shared_fractal_k_strokes={} disconnected_stroke_pairs={} prefix_equal={}",
        bars.len(),p.merged_bars.len(),p.fractals.len(),p.strokes.len(),p.segments.len(),shared.len(),disconnected.len(),bars.len());
    if let Some(s) = shared.first() {
        println!("first_shared=(raw_start,raw_end,merged_start,merged_end)={s:?}");
    }
    if let Some(s) = disconnected.first() {
        println!("first_disconnected={s:?}");
    }
    (shared.len(), disconnected.len())
}

fn main() {
    // 真正L1变化而非网络重复：q_b每次变化，q_a恒定；四个不同状态可量化成同一点。
    let qa = 50_400;
    let pivots: [i128; 16] = [
        20, 160, 60, 180, 40, 140, 20, 180, 60, 140, 40, 160, 80, 180, 100, 120,
    ];
    let mut quantities = Vec::new();
    for _ in 0..4 {
        for k in pivots {
            assert_eq!((k * qa) % (200 - k), 0);
            let base = k * qa / (200 - k);
            for d in 0..4 {
                assert_eq!(observed(base + d, qa), BID as i64 + k as i64);
                quantities.push(base + d);
            }
        }
    }
    let shared = evaluate(
        "distinct_queue_updates_same_quantized_points",
        &make_bars(&quantities, qa),
    );
    assert!(
        shared.0 > 0,
        "specified shared-K counterexample was not reproduced"
    );

    // 含合法长间隔和短反折；所有相邻状态的买量不同，报价不动。
    let q = [30, 20, 40, 50, 60, 70, 80, 40, 70, 60, 50, 30, 20, 10, 20];
    let disconnected = evaluate("short_reversal_after_valid_stroke", &make_bars(&q, 100));
    assert!(
        disconnected.1 > 0,
        "specified endpoint-connection counterexample was not reproduced"
    );
    println!("scope=two_synthetic_counterexamples_not_semantic_qualification_or_market_incidence");
}
