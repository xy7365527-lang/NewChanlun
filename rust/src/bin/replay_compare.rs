//! #1528 / #1320-B：生产 π 回放的有限公开结果双跑对拍。
//!
//! `replay_compare ES [MAX_BARS|YYYY-MM-DD]` 使用 `run_theta_v0_pi`。
//! 旧 D4/p123 协议仅作为显式诊断入口 `--engine minimal` 保留。
//! 逐 bar 分类、信号、订单、持仓与账本事件尚无生产 tape 出口；本工具不宣称验证这些事件。

use newchan_rust::theta_v0::backtest::data::{bars_per_year, load_by_symbol};
use newchan_rust::theta_v0::backtest::metrics::TradeRecord;
use newchan_rust::theta_v0::backtest::replay_dump::initial_nav_for;
use newchan_rust::theta_v0::backtest::runner::run_theta_v0_pi;
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::trading::replay::{
    BarInput, ClassificationEngine, ClassificationOutput, LayerState, ReplayEngine,
};
use newchan_rust::trading::replay_comparator::ReplayComparator;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

#[derive(Debug, Deserialize)]
struct BarData {
    timestamp: i64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
}

/// #1518 旧 D4/p123 诊断臂：不得作为生产验收结果。
struct MinimalEngine;

impl ClassificationEngine for MinimalEngine {
    fn process_bar(&mut self, bar: BarInput) -> ClassificationOutput {
        ClassificationOutput {
            timestamp_ms: bar.timestamp_ms,
            layer_updates: HashMap::new(),
            signals: HashMap::new(),
        }
    }

    fn get_layer_state(&self, layer_id: usize) -> Option<LayerState> {
        (layer_id < 3).then(|| LayerState {
            layer_id,
            state_description: format!("Layer {layer_id}"),
            tstage: "Test".into(),
            chips: 0.0,
            gross_balance: 0.0,
        })
    }

    fn list_layers(&self) -> Vec<(usize, String)> {
        vec![(0, "L0".into()), (1, "L1".into()), (2, "L2".into())]
    }

    fn reset(&mut self) {}
}

fn usage(bin: &str) -> String {
    format!(
        "用法: {bin} [--engine theta_v0_pi] SYMBOL [MAX_BARS|YYYY-MM-DD]\n\
         旧诊断: {bin} --engine minimal DATA_PATH P123_DUMP_PATH MAX_BARS"
    )
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let tail = &args[1..];
    match tail {
        [flag, engine, rest @ ..] if flag == "--engine" && engine == "minimal" => {
            if let [data, dump, max] = rest {
                minimal_compare(data, dump, max)
            } else {
                Err(usage(&args[0]))
            }
        }
        [flag, engine, rest @ ..] if flag == "--engine" && engine == "theta_v0_pi" => {
            production(rest)
        }
        [flag, engine, ..] if flag == "--engine" => {
            Err(format!("未知引擎 {engine}\n{}", usage(&args[0])))
        }
        rest => production(rest),
    }
}

fn production(args: &[String]) -> Result<(), String> {
    let ([symbol] | [symbol, _]) = args else {
        return Err(usage("replay_compare"));
    };
    let config = ThetaConfig::default();
    let mut dataset = load_by_symbol(symbol, &config)?;
    if let Some(selector) = args.get(1) {
        if selector.len() == 10
            && selector.as_bytes().get(4) == Some(&b'-')
            && selector.as_bytes().get(7) == Some(&b'-')
        {
            dataset = dataset.slice_date_window(selector, selector);
        } else {
            let n: usize = selector
                .parse()
                .map_err(|e| format!("MAX_BARS 非法: {e}"))?;
            if n == 0 {
                return Err("MAX_BARS 必须大于 0".into());
            }
            dataset = dataset.slice_bar_range(0, n);
        }
    }
    if dataset.bars.is_empty() {
        return Err(format!("{symbol} 所选窗口无 bar"));
    }
    let years = (dataset.bars.len() as f64 / bars_per_year(dataset.bar_seconds)).max(1e-9);
    let initial_nav = initial_nav_for(&dataset, &config);
    let first = run_theta_v0_pi(&dataset, &config, years, initial_nav);
    let second = run_theta_v0_pi(&dataset, &config, years, initial_nav);

    // 直接比较逐位编码；哈希只用于展示，绝不以哈希相等代替相等判定。
    compare_f64("equity_curve", &first.equity_curve, &second.equity_curve)?;
    compare_f64("daily_returns", &first.daily_returns, &second.daily_returns)?;
    compare_f64("trade_pnls", &first.trade_pnls, &second.trade_pnls)?;
    compare_f64(
        "trade_pnls_with_forced",
        &first.trade_pnls_with_forced,
        &second.trade_pnls_with_forced,
    )?;
    compare_text(
        "trades",
        &encode_trades(&first.trades),
        &encode_trades(&second.trades),
    )?;
    compare_text(
        "n_orders",
        &first.n_orders.to_string(),
        &second.n_orders.to_string(),
    )?;
    compare_text(
        "r_decomp",
        &format!("{:?}", first.r_decomp),
        &format!("{:?}", second.r_decomp),
    )?;
    compare_text(
        "closed_loop_final",
        &format!("{:?}", first.closed_loop_final),
        &format!("{:?}", second.closed_loop_final),
    )?;
    compare_text(
        "turn_class_rows_sparse",
        &format!("{:?}", first.turn_class_rows),
        &format!("{:?}", second.turn_class_rows),
    )?;
    println!(
        "✅ theta_v0_pi 公开结果双跑通过：symbol={} bars={} orders={} closed_trades={}; 逐 bar 事件 tape 未验",
        symbol,
        first.n_bars,
        first.n_orders,
        first.trades.len()
    );
    Ok(())
}

fn compare_f64(name: &str, first: &[f64], second: &[f64]) -> Result<(), String> {
    let first_hash = hash_f64(first);
    let second_hash = hash_f64(second);
    let mismatch = first
        .iter()
        .zip(second)
        .position(|(a, b)| a.to_bits() != b.to_bits());
    let equal = first.len() == second.len() && mismatch.is_none();
    println!("{name}_hash run1={first_hash} run2={second_hash} equal={equal}");
    if equal {
        Ok(())
    } else {
        Err(format!(
            "双跑逐位不一致: {name}, 首个差异 index={mismatch:?}, len1={}, len2={}",
            first.len(),
            second.len()
        ))
    }
}

fn compare_text(name: &str, first: &str, second: &str) -> Result<(), String> {
    let equal = first == second;
    println!(
        "{name}_hash run1={} run2={} equal={equal}",
        hash_bytes(first.as_bytes()),
        hash_bytes(second.as_bytes())
    );
    if equal {
        Ok(())
    } else {
        let offset = first
            .bytes()
            .zip(second.bytes())
            .position(|(a, b)| a != b)
            .unwrap_or(first.len().min(second.len()));
        Err(format!("双跑逐位不一致: {name}, 首个差异 byte={offset}"))
    }
}

fn hash_f64(values: &[f64]) -> String {
    let mut hasher = Sha256::new();
    hasher.update((values.len() as u64).to_le_bytes());
    for value in values {
        hasher.update(value.to_bits().to_le_bytes());
    }
    format!("{:x}", hasher.finalize())
}

fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn encode_trades(trades: &[TradeRecord]) -> String {
    trades
        .iter()
        .map(|trade| {
            format!(
                "{},{},{},{},{},{}",
                trade.entry_bar,
                trade.exit_bar,
                trade.hold_bars,
                trade.qty.to_bits(),
                trade.long,
                trade.forced_close
            )
        })
        .collect::<Vec<_>>()
        .join(";")
}

fn minimal_compare(data: &str, dump: &str, max: &str) -> Result<(), String> {
    let max: usize = max.parse().map_err(|e| format!("MAX_BARS 非法: {e}"))?;
    let bars = load_bars(data, max)?;
    let mut replay = ReplayEngine::new(Box::new(MinimalEngine));
    replay.replay_bars(bars);
    let result = ReplayComparator::new(replay.all_snapshots().to_vec(), dump)?.compare();
    if result.diffs.is_empty() {
        println!("旧 D4 诊断对拍通过：{} bars, diff=0", result.total_bars);
        Ok(())
    } else {
        Err(format!("旧 D4 诊断对拍有 {} diffs", result.diffs.len()))
    }
}

fn load_bars(path: &str, max: usize) -> Result<Vec<BarInput>, String> {
    let reader =
        BufReader::new(File::open(path).map_err(|e| format!("无法打开数据文件 {path}: {e}"))?);
    let all: Vec<BarData> =
        serde_json::from_reader(reader).map_err(|e| format!("解析 JSON 失败: {e}"))?;
    let bars = all
        .into_iter()
        .take(max)
        .map(|bar| BarInput {
            timestamp_ms: bar.timestamp,
            open: bar.open,
            high: bar.high,
            low: bar.low,
            close: bar.close,
            volume: bar.volume,
        })
        .collect::<Vec<_>>();
    if bars.is_empty() {
        return Err("数据文件为空".into());
    }
    Ok(bars)
}
