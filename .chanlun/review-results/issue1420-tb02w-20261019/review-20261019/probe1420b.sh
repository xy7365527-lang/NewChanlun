set -e
cat >> src/theta_v0/backtest/l3_delta_r_alpha.rs <<'RS'
#[cfg(test)]
mod probe1420b {
    use super::*;
    #[test]
    fn probe1420b_facts() {
        let cfg = ThetaConfig::default();
        let ds = synthetic_martingale(3000, 20260704, 100);
        let layer = crate::theta_v0::parser::parse_layer(&ds.bars, &cfg);
        let (segs, tail, facts) = crate::theta_v0::parser::segment::divide_segments_with_facts(&layer.strokes, &cfg.parse);
        let mut hist = std::collections::BTreeMap::new();
        for c in &facts.candidates { *hist.entry(c.result).or_insert(0) += 1; }
        eprintln!("[probe1420b] strokes={} segs={} tail={:?} results={:?}", layer.strokes.len(), segs.len(), tail, hist);
        for c in facts.candidates.iter().rev().take(4) {
            eprintln!("[probe1420b] start={} result={} waiting={:?} end={:?} observed_through={}", c.start_stroke, c.result, c.waiting_reasons, c.end_stroke, c.observed_through);
        }
    }
}
RS
cargo test --release --lib probe1420b -- --nocapture 2>&1 | grep -E "probe1420b\]|test result|error" | head -20
