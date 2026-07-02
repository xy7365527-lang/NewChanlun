//! **07b frontier 门控 bit-exact 验收（task #23，A 泳道 resume 家族）**。
//!
//! `classifier::extract_second_resume`（07b frontier 门控）只重算 frontier tail、复用 confirmed 前缀
//! B2 缓存，消解 `extract_second_for_level` 的 O(U²) 全塔重扫。门控函数内置 `debug_assert`：**每次调用**
//! 逐字段对拍全量 `extract_second_for_level`（门控输出 == 全量重扫）。本集成测试在 **debug 构建**
//! （debug_assert 生效）下把真实 BTC bar 流逐 bar 喂 `classify_with_tower_incremental`（门控所在的增量
//! 热路径）——每 bar 触发门控 debug_assert，任一 bar 门控破裂即 panic。跑通 = 门控在真实数据上逐 bar
//! bit-exact。
//!
//! ★为何是集成测试（独立 crate，看不到 lib 的 `#[cfg(test)]` 模块）：并发域 task #40 把
//! `RMove::Compose.subs` 改为 `Rc<Vec<RMove>>` 但未同步 econ_positive.rs/cand_predicate.rs 的 test
//! 构造子 ⟹ lib `--lib` test 二进制暂不编译。集成测试只链 lib 公共 API、不编译那些 cfg(test) 模块，
//! 故本验收独立于 #40 的 WIP 破坏可跑。
//!
//! ★认识论 L1（formalization-validity-domain 231号）：debug_assert 逐 bar 对拍是**管线正确性**验证
//! （门控 == 全量重扫的确定性等价），非行情有效断言。
//!
//! 跑法：`cargo test --manifest-path rust/Cargo.toml --test theta_v0_07b_gating -- --ignored --nocapture`
//! （debug 构建 = debug_assert 生效；需 analysis/data_cache/btc_1m_full.json；bar 数 env GATING_BARS，
//! 缺省 30000）。

use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::types::{quantize, Bar};
use newchan_rust::theta_v0::{classifier, parser};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize, Default)]
struct RawData {
    #[serde(default)]
    opens: Vec<Option<f64>>,
    #[serde(default)]
    highs: Vec<Option<f64>>,
    #[serde(default)]
    lows: Vec<Option<f64>>,
    #[serde(default)]
    closes: Vec<Option<f64>>,
    #[serde(default)]
    volumes: Vec<Option<f64>>,
}

/// 加载 BTC 全集为量化 Bar 序列（与 tests/theta_v0_perf_profile.rs::load_btc_bars 同口径）。
fn load_btc_bars(tick_size: f64, limit: usize) -> Vec<Bar> {
    load_bars("analysis/data_cache/btc_1m_full.json", tick_size, limit)
}

/// 从项目根相对路径加载量化 Bar 序列（parallel-array schema，同 data.rs 口径）。
fn load_bars(rel: &str, tick_size: f64, limit: usize) -> Vec<Bar> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("rust/ 父目录 = 项目根")
        .join(rel);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {rel}: {e}"));
    let text = text
        .replace("-Infinity", "null")
        .replace("Infinity", "null")
        .replace("NaN", "null");
    let raw: RawData = serde_json::from_str(&text).expect("解析 BTC JSON");
    let n = raw.closes.len().min(limit);
    let mut bars = Vec::with_capacity(n);
    for i in 0..n {
        let (o, h, l, c) = (raw.opens[i], raw.highs[i], raw.lows[i], raw.closes[i]);
        let v = raw.volumes.get(i).and_then(|x| *x).unwrap_or(0.0);
        let (oq, hq, lq, cq, untradable) =
            if let (Some(o), Some(h), Some(l), Some(c)) = (o, h, l, c) {
                let bad_range = h < o.max(c).max(l) || l > o.min(c).min(h);
                let bad_price = o <= 0.0 || h <= 0.0 || l <= 0.0 || c <= 0.0;
                let untradable = bad_range || bad_price || v <= 0.0;
                (
                    quantize(o, tick_size),
                    quantize(h, tick_size),
                    quantize(l, tick_size),
                    quantize(c, tick_size),
                    untradable,
                )
            } else {
                let prev = bars.last().map(|b: &Bar| b.close).unwrap_or(0);
                (prev, prev, prev, prev, true)
            };
        bars.push(Bar {
            source_index: i,
            timestamp: i as i64,
            open: oq,
            high: hq,
            low: lq,
            close: cq,
            volume: v as i64,
            untradable,
        });
    }
    bars
}

/// 逐 bar 增量分类：每 bar 触发 07b 门控 debug_assert（门控 == 全量重扫）。跑通 = 门控逐 bar bit-exact。
/// 附带断言：过程中真产出过第二类 B2/S2（否则门控路径未被走过，验收空转）。
#[test]
#[ignore = "07b 门控 bit-exact 验收；需 BTC 数据；debug 构建（debug_assert 生效）；GATING_BARS env"]
fn extract_second_resume_bit_exact_vs_full_per_bar() {
    let config = ThetaConfig::default();
    let n: usize = std::env::var("GATING_BARS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(30_000);
    let bars = load_btc_bars(config.tick.tick_size, n);
    assert!(!bars.is_empty(), "BTC 数据非空");

    let mut parser_incr = parser::ParseLayerIncr::new(&config);
    let mut tower_cache = classifier::TowerCache::new();
    let mut second_seen = 0usize;
    for (i, &bar) in bars.iter().enumerate() {
        let l0 = parser_incr.append(bar);
        // 门控 debug_assert 在此每 memo-miss 触发；破裂 ⟹ panic（bar 定位）。
        let (cls, _) = classifier::classify_with_tower_incremental(&l0, &config, &mut tower_cache);
        for lvl in &cls.levels {
            second_seen += lvl
                .bsp
                .iter()
                .filter(|p| p.bits.buy2 || p.bits.sell2)
                .count();
        }
        let _ = i; // bar 索引在 panic 回溯里由 debug_assert 覆盖
    }
    eprintln!(
        "[07b 门控验收] {n} bar 逐 bar 门控 debug_assert 全通过；过程中累计第二类端点={second_seen}"
    );
    assert!(
        second_seen > 0,
        "验收须走过 07b 门控路径（真产出过 B2/S2）——否则门控未被 exercise，验收空转"
    );
}

/// **07b 门控计时对照（release，CL）**：与 lib 内 profile_stage_a3_cl 同口径，但因 #40 破坏 lib test
/// 二进制，改在集成测试跑。`stage_profile::time("07b_extract_second", ...)` 累积门控/全量的墙钟。
/// 跑法：`THETA_PROFILE_STAGES=1 GATING_BARS=1000000 cargo test --release --test theta_v0_07b_gating \
///   profile_07b_cl -- --ignored --nocapture`。
#[test]
#[ignore = "07b 门控 CL 计时；需 cl_1m_databento_10y.json；THETA_PROFILE_STAGES=1；--release"]
fn profile_07b_cl() {
    let config = ThetaConfig::default();
    let n: usize = std::env::var("GATING_BARS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1_000_000);
    let bars = load_bars("analysis/data_cache/cl_1m_databento_10y.json", config.tick.tick_size, n);
    let n = bars.len();
    if std::env::var("THETA_PROFILE_STAGES").is_err() {
        eprintln!("★未设 THETA_PROFILE_STAGES=1 ⟹ stage dump 为空。");
    }
    let mut parser_incr = parser::ParseLayerIncr::new(&config);
    let mut tower_cache = classifier::TowerCache::new();
    let t0 = std::time::Instant::now();
    for &bar in &bars {
        let l0 = parser_incr.append(bar);
        let _ = classifier::classify_with_tower_incremental(&l0, &config, &mut tower_cache);
    }
    eprintln!("[07b-CL] {n} bar 墙钟={:.2}s", t0.elapsed().as_secs_f64());
    classifier::stage_profile::dump();
}
