//! Theta 分类引擎适配器
//!
//! 将 theta_v0::classifier 包装为 fugue_v3::ClassificationEngine。

use std::rc::Rc;

use crate::fugue_v3::interface::classification::{
    BspPointPlaceholder, CenterPlaceholder, ClassificationEngine, ClassificationResult,
    Direction as FugueDir, LevelState, MoveKind as FugueMoveKind, MovePlaceholder,
    MoveStatus as FugueMoveStatus,
};
use crate::theta_v0::classifier::decompose::MoveStatus;
use crate::theta_v0::classifier::{classify, classify_incremental, BspPoint, ClassifyOutput, TowerCache};
use crate::theta_v0::config::ThetaConfig;
use crate::theta_v0::parser::parse_layer;
use crate::theta_v0::types::{Bar, BspBits, Direction, MoveKind};

/// Theta 分类引擎适配器
pub struct ThetaClassificationAdapter {
    config: ThetaConfig,
}

impl ThetaClassificationAdapter {
    pub fn new(config: ThetaConfig) -> Self {
        Self { config }
    }

    /// 转换 BspBits 到 u8 bit vector
    fn convert_bsp_bits(bits: &BspBits) -> u8 {
        let mut result = 0u8;
        if bits.buy1 {
            result |= 0b000001;
        }
        if bits.sell1 {
            result |= 0b000010;
        }
        if bits.buy2 {
            result |= 0b000100;
        }
        if bits.sell2 {
            result |= 0b001000;
        }
        if bits.buy3 {
            result |= 0b010000;
        }
        if bits.sell3 {
            result |= 0b100000;
        }
        result
    }

    /// 转换 BspPoint 到 BspPointPlaceholder
    fn convert_bsp_point(bsp: &BspPoint) -> BspPointPlaceholder {
        BspPointPlaceholder {
            source_index: bsp.source_index,
            bits: Self::convert_bsp_bits(&bsp.bits),
            pivot_low: bsp.pivot_low as f64,
            pivot_high: bsp.pivot_high as f64,
            center_index: None, // TODO: 从 bsp.owner 提取中枢索引
        }
    }

    /// 转换 MoveKind
    fn convert_move_kind(kind: &MoveKind) -> FugueMoveKind {
        match kind {
            MoveKind::Trend => FugueMoveKind::Trend,
            MoveKind::Consolidation => FugueMoveKind::Consolidation,
        }
    }

    /// 转换 Direction
    fn convert_direction(dir: &Option<Direction>) -> Option<FugueDir> {
        dir.as_ref().map(|d| match d {
            Direction::Up => FugueDir::Up,
            Direction::Down => FugueDir::Down,
        })
    }

    /// 转换 MoveStatus
    fn convert_move_status(status: &MoveStatus) -> FugueMoveStatus {
        match status {
            MoveStatus::Active => FugueMoveStatus::Active,
            MoveStatus::Completed => FugueMoveStatus::Closed,
        }
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
                // 转换中枢
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

                // 转换买卖点
                let bsp_points = level
                    .bsp
                    .iter()
                    .map(|b| Self::convert_bsp_point(b))
                    .collect();

                // 转换走势段
                let moves = level
                    .moves
                    .iter()
                    .map(|m| MovePlaceholder {
                        start_center: m.start_center,
                        end_center: m.end_center,
                        kind: Self::convert_move_kind(&m.kind),
                        dir: Self::convert_direction(&m.dir),
                        level_lift: m.level_lift,
                        status: Self::convert_move_status(&m.status),
                    })
                    .collect();

                LevelState {
                    centers,
                    bsp_points,
                    moves,
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
