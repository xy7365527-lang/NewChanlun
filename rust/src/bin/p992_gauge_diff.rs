//! # `p992_gauge_diff` —— I-4（#992）：ForceL vs MacdArea 两档 classify 的买卖点差集，只读探针
//!
//! **测什么**：#990 默认判据切换的信号差集（SPEC #987 I-2 验收欠账 + T2 #977 转正义务）。
//! 同数据同塔同结构门，仅 gauge 档不同 ⟹ bsp 点集差集 = 两判据分歧面（真口径，经 #989 反查）。
//! 只读。

use newchan_rust::theta_v0::backtest::data::load_by_symbol;
use newchan_rust::theta_v0::classifier::classify;
use newchan_rust::theta_v0::classifier::divergence::DivergenceGauge;
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::parse_layer;
use std::collections::BTreeSet;

fn main() -> std::process::ExitCode {
    let symbol = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "OKLO".to_string());
    let mut cfg_f = ThetaConfig::default();
    cfg_f.divergence_gauge = DivergenceGauge::ForceL;
    let mut cfg_m = ThetaConfig::default();
    cfg_m.divergence_gauge = DivergenceGauge::MacdArea;

    let ds = match load_by_symbol(&symbol, &cfg_f) {
        Ok(ds) => ds,
        Err(e) => {
            eprintln!("数据加载失败: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let l0 = parse_layer(&ds.bars, &cfg_f);
    println!("P992_INPUT symbol={symbol} bars={}", ds.bars.len());

    let cls_f = classify(&l0, &cfg_f);
    let cls_m = classify(&l0, &cfg_m);

    // 键 = (level, start_index, is_buy1, is_sell1)——点位 + 一类 bit 面。
    let keyset = |cls: &newchan_rust::theta_v0::classifier::Classification| -> BTreeSet<(u32, usize, bool, bool)> {
        let mut s = BTreeSet::new();
        for (lvl, lv) in cls.levels.iter().enumerate() {
            for p in lv.bsp.iter() {
                s.insert((lvl as u32, p.source_index, p.bits.buy1, p.bits.sell1));
            }
        }
        s
    };
    let kf = keyset(&cls_f);
    let km = keyset(&cls_m);
    let both = kf.intersection(&km).count();
    let f_only = kf.difference(&km).count();
    let m_only = km.difference(&kf).count();
    println!(
        "P992_SUMMARY points_forcel={kf} points_macd={km} both={both} forcel_only={f_only} macd_only={m_only}",
        kf = kf.len(),
        km = km.len(),
    );
    // 差集样例（前 10）
    for (k, tag) in kf.difference(&km).take(5).map(|k| (k, "FORCEL_ONLY")) {
        println!("P992_DIFF {tag} {k:?}");
    }
    for (k, tag) in km.difference(&kf).take(5).map(|k| (k, "MACD_ONLY")) {
        println!("P992_DIFF {tag} {k:?}");
    }
    std::process::ExitCode::SUCCESS
}
