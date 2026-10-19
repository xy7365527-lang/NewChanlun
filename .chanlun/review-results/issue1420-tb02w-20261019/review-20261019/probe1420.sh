set -e
cat >> src/theta_v0/backtest/l3_delta_r_alpha.rs <<'RS'
#[cfg(test)]
mod probe1420 {
    use super::*;
    #[test]
    fn probe1420_segments() {
        let cfg = ThetaConfig::default();
        for &(n, seed) in &[(3000usize, 20260704u64), (20000, 20260704)] {
            let ds = synthetic_martingale(n, seed, 100);
            let layer = crate::theta_v0::parser::parse_layer(&ds.bars, &cfg);
            let ledger = super::super::runner::typed_ledger_from_bars(&ds.bars, &cfg);
            let ends: Vec<usize> = layer.segments.iter().map(|s| s.end_index).collect();
            eprintln!("[probe1420] n={n} merged={} strokes={} segments={} last_end={:?} ledger={} seg_ends_tail={:?}",
                layer.merged_bars.len(), layer.strokes.len(), layer.segments.len(), ends.last(), ledger.len(),
                &ends[ends.len().saturating_sub(6)..]);
        }
    }
}
RS
cargo test --release --lib probe1420 -- --nocapture 2>&1 | grep -E "probe1420|test result|error" | head -20
cargo test --release --lib m3_partition_synthetic_holds -- --nocapture 2>&1 | grep -E "m3-partition|test result|panicked" | head -5
