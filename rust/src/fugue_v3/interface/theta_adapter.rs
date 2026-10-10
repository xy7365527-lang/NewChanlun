//! Theta 分类引擎适配器
//!
//! 将 theta_v0::classifier 包装为 fugue_v3::ClassificationEngine。

use std::rc::Rc;

use crate::fugue_v3::interface::classification::{
    CenterPlaceholder, ClassificationEngine, ClassificationResult, LevelState,
};
use crate::theta_v0::classifier::{classify, classify_incremental, ClassifyOutput, TowerCache};
use crate::theta_v0::config::ThetaConfig;
use crate::theta_v0::parser::parse_layer;
use crate::theta_v0::types::Bar;

/// Theta 分类引擎适配器
pub struct ThetaClassificationAdapter {
    config: ThetaConfig,
}

impl ThetaClassificationAdapter {
    pub fn new(config: ThetaConfig) -> Self {
        Self { config }
    }
}

impl ClassificationEngine for ThetaClassificationAdapter {
    fn classify(
        &self,
        bars: &[Bar],
        _segments: &[usize],
        operating_levels: &[u32],
    ) -> ClassificationResult {
        // 构造 L0 ParseLayer
        let l0 = parse_layer(bars, &self.config);

        // 调用 theta_v0::classifier::classify
        let output: ClassifyOutput = classify(&l0, &self.config, operating_levels);

        // 从 classification 提取各级状态
        let levels = output
            .classification
            .levels
            .iter()
            .map(|level| {
                let centers = Rc::new(
                    level
                        .centers
                        .iter()
                        .map(|c| CenterPlaceholder {
                            zd: c.zd as f64,
                            zg: c.zg as f64,
                            dd: c.dd as f64,
                            gg: c.gg as f64,
                            start_index: c.start_index,
                            end_index: c.end_index,
                        })
                        .collect(),
                );

                LevelState {
                    centers,
                    bsp_points: vec![],
                    moves: vec![],
                }
            })
            .collect();

        ClassificationResult { levels }
    }

    fn classify_incremental(
        &self,
        _prev: &ClassificationResult,
        _new_bar: &Bar,
        _operating_levels: &[u32],
    ) -> ClassificationResult {
        // TODO: 实现真正的增量逻辑
        // 暂时 stub 返回空结果
        ClassificationResult { levels: vec![] }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_instantiation() {
        let config = ThetaConfig::default();
        let _adapter = ThetaClassificationAdapter::new(config);
    }

    #[test]
    fn test_classify_empty() {
        let config = ThetaConfig::default();
        let adapter = ThetaClassificationAdapter::new(config);
        let result = adapter.classify(&[], &[], &[]);
        // 空输入可能返回空 levels 或单个空 level
        assert!(result.levels.is_empty() || result.levels[0].centers.is_empty());
    }

    #[test]
    fn test_classify_basic() {
        let config = ThetaConfig::default();
        let adapter = ThetaClassificationAdapter::new(config);
        let bars = vec![
            Bar {
                source_index: 0,
                timestamp: 0,
                open: 10000,
                high: 11000,
                low: 9000,
                close: 10500,
                volume: 1000.0,
                untradable: false,
            },
            Bar {
                source_index: 1,
                timestamp: 1,
                open: 10500,
                high: 11500,
                low: 9500,
                close: 10000,
                volume: 1100.0,
                untradable: false,
            },
        ];
        let segments = vec![0, 1];
        let result = adapter.classify(&bars, &segments, &[]);
        // 两个 bar 可能不足以构成完整结构，只验证调用成功
    }
}
