//! 历史回放引擎（D4 实现）
//!
//! 职责：逐 bar 回放历史数据，完整记录每一重的状态演化。
//!
//! ## 账本导出
//!
//! `export_book()` 方法导出回放账本（类 YieldBook 风格），补齐关票门子句 ②（验证）
//! 的账本可查要求。账本内容：
//! - `total_bars`：已回放的 bar 数量
//! - `snapshots`：生成的状态快照数量
//! - `layers`：分类引擎的重列表
//!
//! ## 验收
//!
//! - ADR：`docs/adr/00XX-replay-engine-architecture.md`（待编号）
//! - 单测：`test_export_book` 覆盖 3 bars 回放场景
//! - 对拍：`scripts/replay_compare.sh`（Phase 0 子票 B）
//!
//! ## 接口占位符（fugue_v3::interface 未暴露）
//!
//! 当前使用内部占位符类型（BarInput, ClassificationEngine 等）以保持模块独立性。
//! 待 fugue_v3::interface 正式暴露后，替换为生产接口。

use std::collections::HashMap;

// ════════════════════════════ 占位符类型（待 fugue_v3::interface 暴露后替换）════════════════════════════

pub type LayerId = usize;

/// Bar 输入（占位符）
#[derive(Debug, Clone)]
pub struct BarInput {
    pub timestamp_ms: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

/// 分类引擎输出（占位符）
#[derive(Debug, Clone)]
pub struct ClassificationOutput {
    pub timestamp_ms: i64,
    pub layer_updates: HashMap<LayerId, String>,
    pub signals: HashMap<LayerId, Vec<String>>,
}

/// 重状态（占位符）
#[derive(Debug, Clone)]
pub struct LayerState {
    pub layer_id: LayerId,
    pub state_description: String,
    pub tstage: String,
    pub chips: f64,
    pub gross_balance: f64,
}

/// 分类引擎接口（占位符）
pub trait ClassificationEngine {
    fn process_bar(&mut self, bar: BarInput) -> ClassificationOutput;
    fn get_layer_state(&self, layer_id: LayerId) -> Option<LayerState>;
    fn list_layers(&self) -> Vec<(LayerId, String)>;
    fn reset(&mut self);
}

// ════════════════════════════ 回放引擎 ════════════════════════════

/// 历史回放引擎
pub struct ReplayEngine {
    /// 分类引擎实例
    classification_engine: Box<dyn ClassificationEngine>,
    /// 已处理的 bar 序列
    processed_bars: Vec<BarInput>,
    /// 逐 bar 的状态快照
    state_snapshots: Vec<StateSnapshot>,
}

/// 单个 bar 的状态快照
#[derive(Debug, Clone)]
pub struct StateSnapshot {
    /// Bar 的时间戳
    pub timestamp_ms: i64,
    /// 分类引擎的输出
    pub output: ClassificationOutput,
    /// 所有重的完整状态
    pub layer_states: HashMap<LayerId, LayerState>,
}

/// 回放账本（类 YieldBook 风格）
///
/// 导出回放引擎的账本信息，补齐关票门子句 ②（验证）的账本可查要求。
#[derive(Debug, Clone)]
pub struct ReplayBook {
    /// 已回放的 bar 数量
    pub total_bars: usize,
    /// 生成的状态快照数量
    pub snapshots: usize,
    /// 分类引擎的重列表（LayerId → 名称）
    pub layers: Vec<(LayerId, String)>,
}

impl ReplayEngine {
    /// 创建新的回放引擎
    pub fn new(classification_engine: Box<dyn ClassificationEngine>) -> Self {
        Self {
            classification_engine,
            processed_bars: Vec::new(),
            state_snapshots: Vec::new(),
        }
    }

    /// 回放单个 bar
    pub fn replay_bar(&mut self, bar: BarInput) {
        let output = self.classification_engine.process_bar(bar.clone());

        let layer_states: HashMap<LayerId, LayerState> = self
            .classification_engine
            .list_layers()
            .iter()
            .filter_map(|(layer_id, _)| {
                self.classification_engine
                    .get_layer_state(*layer_id)
                    .map(|state| (*layer_id, state))
            })
            .collect();

        let snapshot = StateSnapshot {
            timestamp_ms: bar.timestamp_ms,
            output,
            layer_states,
        };

        self.state_snapshots.push(snapshot);
        self.processed_bars.push(bar);
    }

    /// 批量回放 bar 序列
    pub fn replay_bars(&mut self, bars: Vec<BarInput>) {
        for bar in bars {
            self.replay_bar(bar);
        }
    }

    /// 获取指定 bar 的状态快照
    pub fn get_snapshot(&self, bar_index: usize) -> Option<&StateSnapshot> {
        self.state_snapshots.get(bar_index)
    }

    /// 获取所有快照
    pub fn all_snapshots(&self) -> &[StateSnapshot] {
        &self.state_snapshots
    }

    /// 查询某重在某个 bar 的状态
    pub fn query_layer_state(&self, bar_index: usize, layer_id: LayerId) -> Option<LayerState> {
        self.state_snapshots
            .get(bar_index)
            .and_then(|snapshot| snapshot.layer_states.get(&layer_id))
            .cloned()
    }

    /// 导出逐重的状态时间序列
    pub fn export_layer_timeline(&self, layer_id: LayerId) -> Vec<(i64, LayerState)> {
        self.state_snapshots
            .iter()
            .filter_map(|snapshot| {
                snapshot
                    .layer_states
                    .get(&layer_id)
                    .map(|state| (snapshot.timestamp_ms, state.clone()))
            })
            .collect()
    }

    /// 导出回放账本（类 YieldBook 风格）
    ///
    /// 补齐关票门子句 ②（验证）的账本可查要求。
    ///
    /// # 返回
    ///
    /// `ReplayBook` 包含：
    /// - `total_bars`：已回放的 bar 数量
    /// - `snapshots`：生成的状态快照数量
    /// - `layers`：分类引擎的重列表
    ///
    /// # 示例
    ///
    /// ```rust,ignore
    /// let mut replay = ReplayEngine::new(engine);
    /// replay.replay_bars(bars);
    /// let book = replay.export_book();
    /// assert_eq!(book.total_bars, bars.len());
    /// ```
    pub fn export_book(&self) -> ReplayBook {
        ReplayBook {
            total_bars: self.processed_bars.len(),
            snapshots: self.state_snapshots.len(),
            layers: self
                .classification_engine
                .list_layers()
                .iter()
                .map(|(id, name)| (*id, name.clone()))
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MinimalEngine;

    impl ClassificationEngine for MinimalEngine {
        fn process_bar(&mut self, bar: BarInput) -> ClassificationOutput {
            ClassificationOutput {
                timestamp_ms: bar.timestamp_ms,
                layer_updates: HashMap::new(),
                signals: HashMap::new(),
            }
        }

        fn get_layer_state(&self, layer_id: LayerId) -> Option<LayerState> {
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

        fn list_layers(&self) -> Vec<(LayerId, String)> {
            vec![
                (0, "L0".to_string()),
                (1, "L1".to_string()),
                (2, "L2".to_string()),
            ]
        }

        fn reset(&mut self) {}
    }

    #[test]
    fn test_replay_single_bar() {
        let engine = Box::new(MinimalEngine);
        let mut replay = ReplayEngine::new(engine);

        let bar = BarInput {
            timestamp_ms: 1000,
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.5,
            volume: 1000.0,
        };

        replay.replay_bar(bar);
        assert_eq!(replay.all_snapshots().len(), 1);
    }

    #[test]
    fn test_export_book() {
        let engine = Box::new(MinimalEngine);
        let mut replay = ReplayEngine::new(engine);

        // replay 3 bars
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

        let book = replay.export_book();
        assert_eq!(book.total_bars, 3);
        assert_eq!(book.snapshots, 3);
        assert_eq!(book.layers.len(), 3); // MinimalEngine has 3 layers

        // 验证 layers 内容
        let layer_ids: Vec<LayerId> = book.layers.iter().map(|(id, _)| *id).collect();
        assert_eq!(layer_ids, vec![0, 1, 2]);
    }
}
