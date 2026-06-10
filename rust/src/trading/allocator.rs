//! SizeAllocator — 40课结构规模：中枢振幅占比归一 frac[k]（K6 逐字保留）。
//!
//! A_k = (ZG_k − ZD_k)/c（k 层当前存活中枢相对振幅）；无存活中枢 → A_k=0
//! （没有结构域就没有操作权，E2 同构）；frac[k] = A_k / Σ_j A_j（Σ=0 → 全 0）。
//! 重算时机 = 中枢生死事件（CenterBook 版本号变化，不逐 bar 避免 churn）。
//!
//! bit-exact 前提（T1 陷阱）：求和顺序 = ladder 升序（Python dict 插入序 =
//! active_levels 升序）——`active` 切片由调用方保证升序。
//!
//! v2 改动仅在消费方式（per-tranche 而非 per-leg，C4）与验证框架（V3′ 独立
//! 重验）；公式本身不动。

use super::center_book::CenterBook;
use super::types::MAX_LADDER;

#[derive(Debug)]
pub struct SizeAllocator {
    frac: [f64; MAX_LADDER],
    version: Option<u64>,
}

impl Default for SizeAllocator {
    fn default() -> Self {
        SizeAllocator { frac: [0.0; MAX_LADDER], version: None }
    }
}

impl SizeAllocator {
    pub fn new() -> Self {
        Self::default()
    }

    /// 与 Python `SizeAllocator.maybe_recompute` 逐字（None/dead 跳过、amp>0 过滤、
    /// 升序求和、total≤0 → 全 0）。
    pub fn maybe_recompute(&mut self, version: u64, active: &[usize], book: &CenterBook, c: f64) {
        if self.version == Some(version) {
            return;
        }
        self.version = Some(version);
        let mut amps = [0.0f64; MAX_LADDER];
        let mut total = 0.0f64;
        for &k in active {
            let Some(lc) = book.last(k) else { continue };
            if book.is_dead(k, lc.seg_start) {
                continue;
            }
            let amp = if c > 0.0 { (lc.zg - lc.zd) / c } else { 0.0 };
            if amp > 0.0 {
                amps[k] = amp;
                total += amp; // 升序累加（active 升序）= Python sum(dict.values())
            }
        }
        self.frac = [0.0; MAX_LADDER];
        if total > 0.0 {
            for k in 0..MAX_LADDER {
                if amps[k] > 0.0 {
                    self.frac[k] = amps[k] / total;
                }
            }
        }
    }

    /// frac[k]（无 → 0.0，Python `.get(k, 0.0)`）。
    pub fn frac(&self, k: usize) -> f64 {
        self.frac[k]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::{BspClass, BspEvent};

    fn formed(cs: i64, zd: f64, zg: f64) -> BspEvent {
        BspEvent {
            class: BspClass::Sell1,
            seg_idx: 0,
            confirmed: true,
            cs: Some(cs),
            zd: Some(zd),
            zg: Some(zg),
            price: 0.0,
        }
    }

    #[test]
    fn amplitude_normalization() {
        let mut book = CenterBook::new();
        book.ingest(2, &[formed(1, 9.0, 10.0)], true, None); // amp = 1/10
        book.ingest(3, &[formed(2, 7.0, 10.0)], true, None); // amp = 3/10
        let mut alloc = SizeAllocator::new();
        alloc.maybe_recompute(book.version, &[2, 3], &book, 10.0);
        assert!((alloc.frac(2) - 0.25).abs() < 1e-12);
        assert!((alloc.frac(3) - 0.75).abs() < 1e-12);
        assert_eq!(alloc.frac(4), 0.0);
    }

    #[test]
    fn version_gate_skips_recompute() {
        let mut book = CenterBook::new();
        book.ingest(2, &[formed(1, 9.0, 10.0)], true, None);
        let mut alloc = SizeAllocator::new();
        alloc.maybe_recompute(book.version, &[2], &book, 10.0);
        let f = alloc.frac(2);
        // 同版本不同价格 → 不重算（churn 避免）
        alloc.maybe_recompute(book.version, &[2], &book, 20.0);
        assert_eq!(alloc.frac(2), f);
    }
}
