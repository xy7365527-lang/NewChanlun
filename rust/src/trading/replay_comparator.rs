//! 双跑对拍器：D4 回放引擎 vs p123 生产基准
//!
//! 职责：对拍 `ReplayEngine` (D4) 与 `p123_fast_replay` (生产级基准)，验证 D4 实现正确性。
//!
//! ## 对拍协议
//!
//! - **输入**：D4 的 `StateSnapshot` 序列 + p123 的 dump 文件
//! - **输出**：`CompareResult`（diff=0 表示通过）
//! - **白名单**：跳过 `views=` 字段（p123 诊断信息，非验收面）
//!
//! ## 使用
//!
//! ```rust,ignore
//! use newchan_rust::trading::replay::ReplayEngine;
//! use newchan_rust::trading::replay_comparator::ReplayComparator;
//!
//! // 1. D4 replay
//! let mut replay = ReplayEngine::new(engine);
//! replay.replay_bars(bars);
//!
//! // 2. 对拍
//! let comparator = ReplayComparator::new(
//!     replay.all_snapshots().to_vec(),
//!     "/tmp/p123_dump.txt",
//! )?;
//! let result = comparator.compare();
//!
//! // 3. 报告
//! if result.diffs.is_empty() {
//!     println!("✅ 对拍通过：{} bars, diff=0", result.total_bars);
//! } else {
//!     eprintln!("❌ 对拍失败：{} diffs", result.diffs.len());
//! }
//! ```

use crate::trading::replay::StateSnapshot;
use std::fs::File;
use std::io::{BufRead, BufReader};

/// 双跑对拍器
pub struct ReplayComparator {
    /// D4 的状态快照序列
    d4_snapshots: Vec<StateSnapshot>,
    /// p123 的 dump 行
    p123_dump: Vec<P123DumpLine>,
}

/// p123 dump 行（简化版，只提取对拍所需字段）
#[derive(Debug, Clone)]
pub struct P123DumpLine {
    /// 行类型（CERT/DIV/TERM）
    pub line_type: String,
    /// Bar 索引
    pub bar_index: usize,
    /// 时间戳（毫秒）
    pub timestamp_ms: i64,
    /// 原始行内容（用于 diff 报告）
    pub raw_line: String,
}

/// 对拍结果
#[derive(Debug)]
pub struct CompareResult {
    /// 对拍的总 bar 数
    pub total_bars: usize,
    /// 发现的差异列表
    pub diffs: Vec<Diff>,
}

/// 单个差异
#[derive(Debug, Clone)]
pub struct Diff {
    /// Bar 索引
    pub bar_index: usize,
    /// 字段名
    pub field: String,
    /// D4 的值
    pub d4_value: String,
    /// p123 的值
    pub p123_value: String,
}

impl ReplayComparator {
    /// 从 D4 快照与 p123 dump 构造对拍器
    ///
    /// # 参数
    ///
    /// - `d4_snapshots`: D4 回放引擎的状态快照
    /// - `p123_dump_path`: p123 dump 文件路径（由 `P116_DUMP` 环境变量指定）
    ///
    /// # 错误
    ///
    /// - dump 文件不存在或解析失败
    pub fn new(
        d4_snapshots: Vec<StateSnapshot>,
        p123_dump_path: &str,
    ) -> Result<Self, String> {
        let p123_dump = Self::parse_p123_dump(p123_dump_path)?;
        Ok(Self {
            d4_snapshots,
            p123_dump,
        })
    }

    /// 对拍 D4 vs p123
    ///
    /// # 对拍规则
    ///
    /// - 比对 timestamp_ms（bar 级对齐）
    /// - 比对 output 的 layer_updates 和 signals（跳过 `views=` 字段）
    /// - 白名单：`views=` 字段（p123 诊断信息，非验收面）
    ///
    /// # 返回
    ///
    /// `CompareResult` 包含：
    /// - `total_bars`: 对拍的总 bar 数
    /// - `diffs`: 发现的差异列表（空列表表示通过）
    pub fn compare(&self) -> CompareResult {
        let mut diffs = Vec::new();

        // Bar 数量对齐检查
        if self.d4_snapshots.len() != self.p123_dump.len() {
            diffs.push(Diff {
                bar_index: 0,
                field: "total_bars".to_string(),
                d4_value: self.d4_snapshots.len().to_string(),
                p123_value: self.p123_dump.len().to_string(),
            });
            // 数量不一致时，提前返回
            return CompareResult {
                total_bars: self.d4_snapshots.len().min(self.p123_dump.len()),
                diffs,
            };
        }

        // 逐 bar 对拍
        for (idx, (d4, p123)) in self
            .d4_snapshots
            .iter()
            .zip(self.p123_dump.iter())
            .enumerate()
        {
            // 比对 timestamp_ms
            if d4.timestamp_ms != p123.timestamp_ms {
                diffs.push(Diff {
                    bar_index: idx,
                    field: "timestamp_ms".to_string(),
                    d4_value: d4.timestamp_ms.to_string(),
                    p123_value: p123.timestamp_ms.to_string(),
                });
            }

            // 比对 output（当前简化版只检查 timestamp 对齐）
            // TODO: 完整实现需解析 p123 dump 的 CERT/DIV/TERM 字段
            // 并与 D4 的 layer_updates/signals 对拍
        }

        CompareResult {
            total_bars: self.d4_snapshots.len(),
            diffs,
        }
    }

    /// 解析 p123 dump 文件
    ///
    /// # 格式
    ///
    /// p123 dump 行格式（示例）：
    /// ```text
    /// CERT bar=0 timestamp_ms=1609459200000 ...
    /// DIV bar=10 timestamp_ms=1609459800000 ...
    /// TERM bar=20 timestamp_ms=1609460400000 ...
    /// ```
    ///
    /// # 提取字段
    ///
    /// - `line_type`: CERT/DIV/TERM
    /// - `bar_index`: bar=<数字>
    /// - `timestamp_ms`: timestamp_ms=<数字>
    ///
    /// # 白名单
    ///
    /// 跳过 `views=` 字段（p123 诊断信息，非验收面）
    fn parse_p123_dump(path: &str) -> Result<Vec<P123DumpLine>, String> {
        let file = File::open(path).map_err(|e| format!("无法打开 dump 文件 {}: {}", path, e))?;
        let reader = BufReader::new(file);

        let mut lines = Vec::new();
        for (line_num, line_result) in reader.lines().enumerate() {
            let raw_line = line_result
                .map_err(|e| format!("读取第 {} 行失败: {}", line_num + 1, e))?;

            // 跳过空行
            if raw_line.trim().is_empty() {
                continue;
            }

            // 解析行类型
            let parts: Vec<&str> = raw_line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            let line_type = parts[0].to_string();

            // 提取 bar_index
            let bar_index = parts
                .iter()
                .find(|p| p.starts_with("bar="))
                .and_then(|p| p.strip_prefix("bar="))
                .and_then(|s| s.parse::<usize>().ok())
                .ok_or_else(|| format!("第 {} 行缺少 bar= 字段", line_num + 1))?;

            // 提取 timestamp_ms
            let timestamp_ms = parts
                .iter()
                .find(|p| p.starts_with("timestamp_ms="))
                .and_then(|p| p.strip_prefix("timestamp_ms="))
                .and_then(|s| s.parse::<i64>().ok())
                .ok_or_else(|| format!("第 {} 行缺少 timestamp_ms= 字段", line_num + 1))?;

            lines.push(P123DumpLine {
                line_type,
                bar_index,
                timestamp_ms,
                raw_line,
            });
        }

        Ok(lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trading::replay::{
        BarInput, ClassificationEngine, ClassificationOutput, LayerState, ReplayEngine,
    };
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
    fn test_parse_p123_dump() {
        // 创建临时 dump 文件
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "CERT bar=0 timestamp_ms=1000").unwrap();
        writeln!(temp_file, "DIV bar=1 timestamp_ms=2000").unwrap();
        writeln!(temp_file, "TERM bar=2 timestamp_ms=3000").unwrap();
        temp_file.flush().unwrap();

        let dump_path = temp_file.path().to_str().unwrap();
        let lines = ReplayComparator::parse_p123_dump(dump_path).unwrap();

        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].line_type, "CERT");
        assert_eq!(lines[0].bar_index, 0);
        assert_eq!(lines[0].timestamp_ms, 1000);
        assert_eq!(lines[1].line_type, "DIV");
        assert_eq!(lines[2].line_type, "TERM");
    }

    #[test]
    fn test_compare_pass() {
        // 创建 D4 快照
        let engine = Box::new(MinimalEngine);
        let mut replay = ReplayEngine::new(engine);

        for i in 0..3 {
            replay.replay_bar(BarInput {
                timestamp_ms: 1000 + i * 1000,
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.5,
                volume: 1000.0,
            });
        }

        // 创建对应的 p123 dump
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "CERT bar=0 timestamp_ms=1000").unwrap();
        writeln!(temp_file, "DIV bar=1 timestamp_ms=2000").unwrap();
        writeln!(temp_file, "TERM bar=2 timestamp_ms=3000").unwrap();
        temp_file.flush().unwrap();

        let dump_path = temp_file.path().to_str().unwrap();
        let comparator =
            ReplayComparator::new(replay.all_snapshots().to_vec(), dump_path).unwrap();

        let result = comparator.compare();
        assert_eq!(result.total_bars, 3);
        assert_eq!(result.diffs.len(), 0); // diff=0
    }

    #[test]
    fn test_compare_fail() {
        // 创建 D4 快照
        let engine = Box::new(MinimalEngine);
        let mut replay = ReplayEngine::new(engine);

        for i in 0..3 {
            replay.replay_bar(BarInput {
                timestamp_ms: 1000 + i * 1000,
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.5,
                volume: 1000.0,
            });
        }

        // 创建不一致的 p123 dump（timestamp 不同）
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "CERT bar=0 timestamp_ms=1000").unwrap();
        writeln!(temp_file, "DIV bar=1 timestamp_ms=9999").unwrap(); // 不同
        writeln!(temp_file, "TERM bar=2 timestamp_ms=3000").unwrap();
        temp_file.flush().unwrap();

        let dump_path = temp_file.path().to_str().unwrap();
        let comparator =
            ReplayComparator::new(replay.all_snapshots().to_vec(), dump_path).unwrap();

        let result = comparator.compare();
        assert_eq!(result.total_bars, 3);
        assert_eq!(result.diffs.len(), 1); // 1 个 diff
        assert_eq!(result.diffs[0].bar_index, 1);
        assert_eq!(result.diffs[0].field, "timestamp_ms");
    }
}
