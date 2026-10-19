//! task #1518：双跑对拍机制 - 集成测试
//!
//! 验证 `ReplayComparator` 对拍功能的正确性。

use newchan_rust::trading::replay::{
    BarInput, ClassificationEngine, ClassificationOutput, LayerState, ReplayEngine,
};
use newchan_rust::trading::replay_comparator::ReplayComparator;
use std::collections::HashMap;
use std::io::Write;
use tempfile::NamedTempFile;

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

#[test]
fn test_replay_compare_10_bars() {
    // 创建 D4 快照（10 bars）
    let engine = Box::new(MinimalEngine);
    let mut replay = ReplayEngine::new(engine);

    for i in 0..10 {
        replay.replay_bar(BarInput {
            timestamp_ms: 1000 + i * 1000,
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.5,
            volume: 1000.0,
        });
    }

    // 创建对应的 p123 dump（10 bars）
    let mut temp_file = NamedTempFile::new().unwrap();
    for i in 0..10 {
        writeln!(temp_file, "CERT bar={} timestamp_ms={}", i, 1000 + i * 1000).unwrap();
    }
    temp_file.flush().unwrap();

    let dump_path = temp_file.path().to_str().unwrap();
    let comparator = ReplayComparator::new(replay.all_snapshots().to_vec(), dump_path).unwrap();

    let result = comparator.compare();
    assert_eq!(result.total_bars, 10);
    assert_eq!(result.diffs.len(), 0, "应该有 0 个差异");
}

#[test]
fn test_replay_compare_100_bars() {
    // 创建 D4 快照（100 bars）
    let engine = Box::new(MinimalEngine);
    let mut replay = ReplayEngine::new(engine);

    for i in 0..100 {
        replay.replay_bar(BarInput {
            timestamp_ms: 1000 + i * 1000,
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.5,
            volume: 1000.0,
        });
    }

    // 创建对应的 p123 dump（100 bars）
    let mut temp_file = NamedTempFile::new().unwrap();
    for i in 0..100 {
        writeln!(temp_file, "CERT bar={} timestamp_ms={}", i, 1000 + i * 1000).unwrap();
    }
    temp_file.flush().unwrap();

    let dump_path = temp_file.path().to_str().unwrap();
    let comparator = ReplayComparator::new(replay.all_snapshots().to_vec(), dump_path).unwrap();

    let result = comparator.compare();
    assert_eq!(result.total_bars, 100);
    assert_eq!(result.diffs.len(), 0, "应该有 0 个差异");
}

#[test]
fn test_replay_compare_1000_bars() {
    // 创建 D4 快照（1000 bars）
    let engine = Box::new(MinimalEngine);
    let mut replay = ReplayEngine::new(engine);

    for i in 0..1000 {
        replay.replay_bar(BarInput {
            timestamp_ms: 1000 + i * 1000,
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.5,
            volume: 1000.0,
        });
    }

    // 创建对应的 p123 dump（1000 bars）
    let mut temp_file = NamedTempFile::new().unwrap();
    for i in 0..1000 {
        writeln!(temp_file, "CERT bar={} timestamp_ms={}", i, 1000 + i * 1000).unwrap();
    }
    temp_file.flush().unwrap();

    let dump_path = temp_file.path().to_str().unwrap();
    let comparator = ReplayComparator::new(replay.all_snapshots().to_vec(), dump_path).unwrap();

    let result = comparator.compare();
    assert_eq!(result.total_bars, 1000);
    assert_eq!(result.diffs.len(), 0, "应该有 0 个差异");
}

#[test]
fn test_replay_compare_count_mismatch() {
    // 创建 D4 快照（10 bars）
    let engine = Box::new(MinimalEngine);
    let mut replay = ReplayEngine::new(engine);

    for i in 0..10 {
        replay.replay_bar(BarInput {
            timestamp_ms: 1000 + i * 1000,
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.5,
            volume: 1000.0,
        });
    }

    // 创建不一致的 p123 dump（只有 5 bars）
    let mut temp_file = NamedTempFile::new().unwrap();
    for i in 0..5 {
        writeln!(temp_file, "CERT bar={} timestamp_ms={}", i, 1000 + i * 1000).unwrap();
    }
    temp_file.flush().unwrap();

    let dump_path = temp_file.path().to_str().unwrap();
    let comparator = ReplayComparator::new(replay.all_snapshots().to_vec(), dump_path).unwrap();

    let result = comparator.compare();
    assert_eq!(result.total_bars, 5); // 取较小值
    assert_eq!(result.diffs.len(), 1); // 应该有 1 个 total_bars 差异
    assert_eq!(result.diffs[0].field, "total_bars");
}
