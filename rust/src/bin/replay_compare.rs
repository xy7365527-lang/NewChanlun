//! task #1518：双跑对拍 bin
//!
//! 对拍 D4 回放引擎 vs p123 生产基准，验证 D4 实现正确性。
//!
//! ## 使用
//!
//! ```bash
//! cargo run --release --bin replay_compare -- \
//!     <DATA_PATH> <P123_DUMP_PATH> <MAX_BARS>
//! ```
//!
//! ## 输出
//!
//! - 通过：exit 0，打印 "✅ 对拍通过：X bars, diff=0"
//! - 失败：exit 1，打印 "❌ 对拍失败：X diffs"

use newchan_rust::trading::replay::{
    BarInput, ClassificationEngine, ClassificationOutput, LayerState, ReplayEngine,
};
use newchan_rust::trading::replay_comparator::ReplayComparator;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

/// Bar 数据（从 JSON 反序列化）
#[derive(Debug, Deserialize)]
struct BarData {
    timestamp: i64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
}

/// 最小化分类引擎（占位符，待接入真实引擎）
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
        if layer_id < 3 {
            Some(LayerState {
                layer_id,
                state_description: format!("Layer {}", layer_id),
                tstage: "Test".to_string(),
                chips: 0.0,
                gross_balance: 0.0,
            })
        } else {
            None
        }
    }

    fn list_layers(&self) -> Vec<(usize, String)> {
        vec![
            (0, "L0".to_string()),
            (1, "L1".to_string()),
            (2, "L2".to_string()),
        ]
    }

    fn reset(&mut self) {}
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        eprintln!("用法: {} <DATA_PATH> <P123_DUMP_PATH> <MAX_BARS>", args[0]);
        return Err("参数错误".to_string());
    }

    let data_path = &args[1];
    let p123_dump_path = &args[2];
    let max_bars: usize = args[3]
        .parse()
        .map_err(|e| format!("MAX_BARS 非法: {}", e))?;

    // 1. 加载 bar 数据
    eprintln!("▶️  加载数据: {}", data_path);
    let bars = load_bars(data_path, max_bars)?;
    eprintln!("✅ 加载 {} bars", bars.len());

    // 2. D4 replay
    eprintln!("▶️  D4 回放中...");
    let engine = Box::new(MinimalEngine);
    let mut replay = ReplayEngine::new(engine);
    replay.replay_bars(bars);
    eprintln!("✅ D4 回放完成");

    // 3. 对拍
    eprintln!("▶️  对拍中...");
    let comparator = ReplayComparator::new(replay.all_snapshots().to_vec(), p123_dump_path)?;
    let result = comparator.compare();

    // 4. 报告
    if result.diffs.is_empty() {
        println!("✅ 对拍通过：{} bars, diff=0", result.total_bars);
        Ok(())
    } else {
        eprintln!(
            "❌ 对拍失败：{} bars, {} diffs",
            result.total_bars,
            result.diffs.len()
        );
        eprintln!("\n前 10 个差异：");
        for (i, diff) in result.diffs.iter().take(10).enumerate() {
            eprintln!(
                "  [{}/{}] bar={} field={} d4={} p123={}",
                i + 1,
                result.diffs.len(),
                diff.bar_index,
                diff.field,
                diff.d4_value,
                diff.p123_value
            );
        }
        Err(format!("{} diffs", result.diffs.len()))
    }
}

/// 从 JSON 文件加载 bar 数据
fn load_bars(path: &str, max_bars: usize) -> Result<Vec<BarInput>, String> {
    let file = File::open(path).map_err(|e| format!("无法打开数据文件 {}: {}", path, e))?;
    let reader = BufReader::new(file);

    let all_bars: Vec<BarData> =
        serde_json::from_reader(reader).map_err(|e| format!("解析 JSON 失败: {}", e))?;

    let bars: Vec<BarInput> = all_bars
        .into_iter()
        .take(max_bars)
        .map(|b| BarInput {
            timestamp_ms: b.timestamp,
            open: b.open,
            high: b.high,
            low: b.low,
            close: b.close,
            volume: b.volume,
        })
        .collect();

    if bars.is_empty() {
        return Err("数据文件为空".to_string());
    }

    Ok(bars)
}
