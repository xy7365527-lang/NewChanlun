//! S2 段口径复核 + λ_gap 校准（task #10，R3 §4 遗留）。
//!
//! R3（gn-feasibility-btc-20260707.md）用 zigzag 代理证必要条件；本模块把口径换成
//! **引擎内缠论段**（parser::segment，67 课特征序列法，与生产回测同一构造），重跑
//! R3 表格同款统计，并给出 λ_gap 止损穿越分布分位——**只产报告不定参**（定参在
//! S3 训练窗内做，OOS 纪律见 treasury-prereg-v1-draft-20260707.md）。
//!
//! 认识论等级：**L2 描述性**（真实数据 + 生产段口径；无 OOS、无功效声明，不进裁决基）。
//!
//! 口径备注（诚实声明）：
//! - `parse_layer` 批量全样本构造——段端点确认使用了段内未来 bar（构造性后视）。
//!   对**描述性分布**这与 R3 的 zigzag 全样本口径对等；对**可交易性**不构成证据。
//! - 幅比用段端点价（confirmed 段）；段内路径用**原始 bars** 高低——段的
//!   `start_index/end_index` 是原始 K 的 `source_index`（stroke.rs:15「按原始 K 计数」），
//!   非 merged 序号。

#[cfg(test)]
mod tests {
    use super::super::super::config::ThetaConfig;
    use super::super::super::parser::parse_layer;
    use super::super::super::types::Direction;
    use super::super::data;

    const FEE: f64 = 3e-4; // 与 R3 §1 / config.rs:228-230 同口径，单边 3 bps

    fn quantile(sorted: &[f64], q: f64) -> f64 {
        assert!(!sorted.is_empty());
        let idx = ((sorted.len() - 1) as f64 * q).round() as usize;
        sorted[idx]
    }

    /// S2 主统计：`cargo test --release --lib -- --ignored --nocapture s2_segment_gn_stats_real_btc`
    /// 可选 `S2_BARS=<n>` 截断窗口冒烟。
    #[test]
    #[ignore = "S2 段口径统计；需 BTC 全历史（329MB）；手动 --ignored 跑"]
    fn s2_segment_gn_stats_real_btc() {
        let config = ThetaConfig::default();
        let mut ds = data::load_by_symbol("BTC", &config)
            .expect("需 BTC 数据（analysis/data_cache/btc_1m_full.json）");
        if let Some(k) = std::env::var("S2_BARS").ok().and_then(|s| s.parse::<usize>().ok()) {
            ds.bars.truncate(k);
        }
        let layer = parse_layer(&ds.bars, &config);
        let segs = &layer.segments;
        assert!(!segs.is_empty(), "全史上段不可能为空");

        // ── R3 同款：幅比 (H−L)/(H+L) 分布 ──
        let mut ratios: Vec<f64> = Vec::with_capacity(segs.len());
        // ── λ_gap：段内逆向穿越深度（相对入场价分数）──
        let mut gaps: Vec<f64> = Vec::with_capacity(segs.len());
        for s in segs.iter() {
            let (h, l) = (
                s.start_price.max(s.end_price) as f64,
                s.start_price.min(s.end_price) as f64,
            );
            ratios.push((h - l) / (h + l));
            let entry = s.start_price as f64;
            let path = &ds.bars[s.start_index..=s.end_index];
            let gap = match s.direction {
                Direction::Up => {
                    let min_low = path.iter().map(|b| b.low).min().unwrap() as f64;
                    (entry - min_low) / entry
                }
                Direction::Down => {
                    let max_high = path.iter().map(|b| b.high).max().unwrap() as f64;
                    (max_high - entry) / entry
                }
            };
            gaps.push(gap.max(0.0));
        }
        let n = ratios.len();
        let pass = ratios.iter().filter(|&&r| r > FEE).count();
        let mut rs = ratios.clone();
        rs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mut gs = gaps.clone();
        gs.sort_by(|a, b| a.partial_cmp(b).unwrap());

        println!("== S2 段口径统计（引擎内缠论段，bars={}）==", ds.bars.len());
        println!(
            "segments={} 满足率={:.4}% 中位幅比={:.4}% p10幅比={:.4}% 净边际中位={:.4}%",
            n,
            pass as f64 / n as f64 * 100.0,
            quantile(&rs, 0.5) * 100.0,
            quantile(&rs, 0.1) * 100.0,
            (quantile(&rs, 0.5) - FEE) * 100.0,
        );
        println!(
            "λ_gap 穿越分布: p50={:.4}% p90={:.4}% p95={:.4}% p99={:.4}% max={:.4}%",
            quantile(&gs, 0.5) * 100.0,
            quantile(&gs, 0.9) * 100.0,
            quantile(&gs, 0.95) * 100.0,
            quantile(&gs, 0.99) * 100.0,
            gs.last().unwrap() * 100.0,
        );
        println!("λ 敏感性（止损=λ 时的段穿越率，穿越 ⟹ 被停出）:");
        for lam in [0.0005, 0.001, 0.002, 0.005, 0.01, 0.02, 0.05] {
            let crossed = gs.iter().filter(|&&g| g > lam).count();
            println!(
                "  λ={:.2}%  穿越率={:.4}%",
                lam * 100.0,
                crossed as f64 / n as f64 * 100.0
            );
        }
    }
}
