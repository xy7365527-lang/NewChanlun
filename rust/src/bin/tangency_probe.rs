//! 票 #248 相切影响量化探针 binary（#246 裁定，裁定书 §6「必须带影响量化」）。
//!
//! 在 OKLO 真实数据（#84 xcheck 同一数据入口与窗口：`oklo_1m_databento.json` →
//! 加载量化 → `parse_layer` → 前 2000 笔）上跑 theta_v0 线段划分**生产管线**，输出：
//!   (a) 两谓词相切命中计数（探针埋在生产谓词本体内，计数路径 = 生产调用路径）；
//!   (b) 段端点列表 JSONL（供改前/改后 diff：段数、逐段端点位移）；
//!   (c) `TANGENT_WINDOW` 窗口桥：前 2000 笔覆盖的 bar 前缀长度（§5 下游对拍的
//!       P116_MAX_BARS 截断口径，编排者 2026-07-25 收窄裁定）。
//!
//! 用法：`cargo run --release --bin tangency_probe -- <oklo_json_path> <segments_out.jsonl>`
//! 改前/改后各跑一遍（改前 = 旧口径副本 /tmp/rust-tangency-before），diff 两份 JSONL。
//!
//! ── 数据加载口径（自含，逐字抄 p123_fast_replay.rs:1543-1619 `load_bars`/`BarsJson`/
//! `date_to_timestamp`）──
//! 不依赖 `theta_v0::backtest::data::load_symbol` 的原因：本 worktree 的 backtest 模块
//! 在 `backtest_bin` feature 下编译失败（fill.rs/runner.rs 引用不存在项，属别 session
//! 在制品痕迹，本票不触碰）。等价性论证（OKLO 数据上本加载 == load_symbol 逐 bar 相同）：
//!   (i)   oklo_1m_databento.json OHLC 无 NaN/None（已核验 343282 bar）⟹ load_symbol 的
//!         NaN/Infinity→null 替换与 Option 缺值分支均不触发；
//!   (ii)  untradable 判据（bad_range || bad_price || volume<=0）与 load_symbol 逐字相同；
//!   (iii) quantize 同为 types::quantize；date_to_timestamp 与 data.rs:180-189 逻辑等价
//!         （去 '+' 后缀、取数字前 14 位）；volumes 列存在且等长（已核验）。
//!   (iv)  timestamp 不参与笔/段划分（parse_layer 只用价格与 source_index 序列）。

use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::feature_seq::{
    gap_tangent_probe_reset, gap_tangent_probe_snapshot,
};
use newchan_rust::theta_v0::parser::parse_layer;
use newchan_rust::theta_v0::parser::segment::{
    divide_segments, overlap_tangent_probe_reset, overlap_tangent_probe_snapshot,
};
use newchan_rust::theta_v0::types::{quantize, Bar, Direction, Timestamp};
use serde::Deserialize;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct BarsJson {
    opens: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    closes: Vec<f64>,
    volumes: Vec<f64>,
    dates: Vec<String>,
}

fn date_to_timestamp(date: &str) -> Timestamp {
    let digits: String = date
        .chars()
        .take_while(|c| *c != '+')
        .filter(|c| c.is_ascii_digit())
        .take(14)
        .collect();
    digits.parse().unwrap_or(0)
}

/// 逐字对齐 p123_fast_replay.rs `load_bars` 的 bar 构造（含 untradable 判据）。
fn load_bars(path: &Path, tick_size: f64) -> Result<Vec<Bar>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("读取 {} 失败: {error}", path.display()))?;
    let raw: BarsJson = serde_json::from_str(&text)
        .map_err(|error| format!("{} JSON 解析失败: {error}", path.display()))?;
    let n = raw.closes.len();
    for (name, len) in [
        ("opens", raw.opens.len()),
        ("highs", raw.highs.len()),
        ("lows", raw.lows.len()),
        ("volumes", raw.volumes.len()),
        ("dates", raw.dates.len()),
    ] {
        if len != n {
            return Err(format!("列长度不一致: {name}={len}, closes={n}"));
        }
    }
    Ok((0..n)
        .map(|index| {
            let open = raw.opens[index];
            let high = raw.highs[index];
            let low = raw.lows[index];
            let close = raw.closes[index];
            let volume = raw.volumes[index];
            Bar {
                source_index: index,
                timestamp: date_to_timestamp(&raw.dates[index]),
                open: quantize(open, tick_size),
                high: quantize(high, tick_size),
                low: quantize(low, tick_size),
                close: quantize(close, tick_size),
                volume: volume as i64,
                untradable: high < open.max(close).max(low)
                    || low > open.min(close).min(high)
                    || open <= 0.0
                    || high <= 0.0
                    || low <= 0.0
                    || close <= 0.0
                    || volume <= 0.0,
            }
        })
        .collect())
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let data_path = args
        .next()
        .ok_or("用法: tangency_probe <oklo_json_path> <segments_out.jsonl>")?;
    let out_path = args
        .next()
        .ok_or("用法: tangency_probe <oklo_json_path> <segments_out.jsonl>")?;

    let cfg = ThetaConfig::default();
    let bars = load_bars(Path::new(&data_path), cfg.tick.tick_size)?;
    let layer = parse_layer(&bars, &cfg);
    // #84 xcheck 同一窗口：前 2000 笔（覆盖 FirstKind+SecondKind 混合）。
    let n = layer.strokes.len().min(2000);
    // §5 窗口桥：前 n 笔所覆盖的 bar 前缀长度 = 第 n 笔端点 bar + 1（笔 start/end_index 即
    // 原始 bar 序号，见 types.rs:89-95 与 canonical.rs:103-108）。p123 下游对拍用同一
    // P116_MAX_BARS 截断（裁定：§5 收窄至 §3/§4 同窗口）。笔构造只经包含/分型，不触本票
    // 两谓词 ⟹ 改前/改后窗口桥相同（调用点核验：feature_seq.rs:428 / segment.rs:151）。
    let bar_cutoff = layer.strokes[..n]
        .iter()
        .map(|s| s.end_index)
        .max()
        .map_or(0, |e| e + 1);
    println!(
        "TANGENT_WINDOW strokes={} bar_cutoff={} bars_total={}",
        n,
        bar_cutoff,
        bars.len()
    );

    // 探针清零 → 生产管线划分 → 读数（同一线程，thread_local 语义安全）。
    gap_tangent_probe_reset();
    overlap_tangent_probe_reset();
    let segs = divide_segments(&layer.strokes[..n], &cfg.parse);
    let gap = gap_tangent_probe_snapshot();
    let ovl = overlap_tangent_probe_snapshot();

    println!(
        "TANGENT_PROBE gap_evals={} gap_tangent_up={} gap_tangent_down={} \
         overlap_calls={} overlap_tangent={}",
        gap.gap_evals, gap.tangent_up, gap.tangent_down, ovl.calls, ovl.tangent
    );
    println!("TANGENT_SEGMENTS strokes={} segments={}", n, segs.len());

    // 段端点 JSONL：方向 + 端点 index + 端点价格（diff 的最小完备面）。
    let mut f =
        std::fs::File::create(&out_path).map_err(|e| format!("创建 {out_path} 失败: {e}"))?;
    for s in &segs {
        let dir = match s.direction {
            Direction::Up => "up",
            Direction::Down => "down",
        };
        writeln!(
            f,
            "{{\"dir\":\"{}\",\"i0\":{},\"i1\":{},\"p0\":{},\"p1\":{}}}",
            dir, s.start_index, s.end_index, s.start_price, s.end_price
        )
        .map_err(|e| format!("写 {out_path} 失败: {e}"))?;
    }
    eprintln!("段端点 → {out_path}（{} 段）", segs.len());
    Ok(())
}
