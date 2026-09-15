use newchan_rust::theta_v0::{classifier, classifier::streaming::OwnedIncrementalClassifier, config::ThetaConfig, parser, types::Bar};
use serde_json::{json, Value};
use std::{env, fs};
fn main() {
    let input = env::args().nth(1).unwrap();
    let values: Vec<Value> = serde_json::from_slice(&fs::read(input).unwrap()).unwrap();
    let bars: Vec<Bar> = values.iter().take(4096).map(|v| Bar {
        source_index: v["source_index"].as_u64().unwrap() as usize,
        timestamp: v["timestamp"].as_i64().unwrap(), open: v["open"].as_i64().unwrap(),
        high: v["high"].as_i64().unwrap(), low: v["low"].as_i64().unwrap(),
        close: v["close"].as_i64().unwrap(), volume: v["volume"].as_f64().unwrap(), untradable: false,
    }).collect();
    assert_eq!(bars.len(), 4096);
    let mut cfg = ThetaConfig::default(); cfg.tick.tick_size = 1.0; cfg.level.l_max = 6;
    let mut incremental = OwnedIncrementalClassifier::new(cfg.clone());
    let mut checked = Vec::new();
    for (index, bar) in bars.iter().enumerate() {
        let actual = incremental.append_bar(*bar);
        let count = index + 1;
        if [512, 1024, 2048, 4096].contains(&count) {
            let full = classifier::classify(&parser::parse_layer(&bars[..count], &cfg), &cfg, &[]);
            assert!(actual.classification == full.classification, "classification mismatch at {count}");
            assert!(actual.tower == full.tower, "tower mismatch at {count}");
            checked.push(json!({"bars": count, "classification_and_tower_equal": true, "levels": full.classification.levels.len()}));
        }
    }
    println!("{}", json!({"status": "PASS_FIXED_INPUT_CLASSIFICATION_AND_TOWER", "checked": checked}));
}
