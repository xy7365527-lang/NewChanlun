//! #1393：解析装配不从未定原件偷产分型/笔；激活清理等待缓存。
use super::*;

#[test]
fn tb02u_parser_pending_activation_sparse_coords_and_resume() {
    let config = ThetaConfig::default();
    let mut incr = ParseLayerIncr::new(&config);
    let raw: Vec<Bar> = [
        (0, 10),
        (2, 8),
        (3, 11),
        (1, 7),
        (4, 12),
        (2, 9),
        (5, 13),
        (3, 10),
    ]
    .iter()
    .enumerate()
    .map(|(i, &(low, high))| Bar {
        source_index: 11 + i * 7,
        timestamp: i as i64,
        open: low,
        high,
        low,
        close: high,
        volume: 1.0,
        untradable: false,
    })
    .collect();
    for n in 1..=raw.len() {
        let actual = incr.append(raw[n - 1]);
        let (full, facts) = parse_layer_with_inclusion_facts(&raw[..n], &config);
        let compact = parse_layer(&raw[..n], &config);
        assert_eq!(actual.merged_bars, full.merged_bars);
        assert_eq!(actual.fractals, full.fractals);
        assert_eq!(actual.strokes, full.strokes);
        assert_eq!(actual.segments, full.segments);
        assert_eq!(actual.tail, full.tail);
        assert_eq!(compact, full);
        let stroke_facts = facts.stroke_facts.as_ref().unwrap();
        if n < 3 {
            assert!(
                actual.merged_bars.is_empty()
                    && actual.fractals.is_empty()
                    && actual.strokes.is_empty()
                    && actual.segments.is_empty()
            );
            assert_eq!(actual.merged_confirmed_len, 0);
            assert!(stroke_facts
                .waiting_reasons
                .contains(&"initial_direction_unsettled"));
        } else {
            if n == 3 {
                assert_eq!(actual.merged_confirmed_len, 0);
            }
            assert!(!stroke_facts
                .waiting_reasons
                .contains(&"initial_direction_unsettled"));
        }
        // 克隆状态后继续，覆盖首次重折之后的缓存证书。
        incr = incr.clone();
    }
}
