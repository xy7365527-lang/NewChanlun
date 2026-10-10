//! 分类引擎接口层
//!
//! 定义 Classification（多级走势分类）的统一接口。

use std::rc::Rc;
use crate::theta_v0::types::Bar;

/// 多级别分类结果
#[derive(Debug, Clone, Default)]
pub struct ClassificationResult {
    /// 各级别状态（levels[0] = L0, levels[1] = L1, ...）
    pub levels: Vec<LevelState>,
}

/// 单级别分类状态
#[derive(Debug, Clone, Default)]
pub struct LevelState {
    /// 该级别的走势类型分解（趋势块/盘整块）
    pub moves: Vec<MovePlaceholder>,
    /// 该级别的中枢序列
    pub centers: Rc<Vec<CenterPlaceholder>>,
    /// 各买卖点条目
    pub bsp_points: Vec<BspPointPlaceholder>,
}

/// 中枢占位类型 - 对应 theta_v0::types::Center
#[derive(Debug, Clone)]
pub struct CenterPlaceholder {
    /// 核心区间下沿 ZD（闭区间）
    pub zd: f64,
    /// 核心区间上沿 ZG（闭区间）
    pub zg: f64,
    /// 外包络下沿 DD
    pub dd: f64,
    /// 外包络上沿 GG
    pub gg: f64,
    /// 起始 K 线索引
    pub start_index: usize,
    /// 结束 K 线索引
    pub end_index: usize,
}

/// 买卖点占位类型 - 对应 theta_v0::classifier::bsp::BspPoint
#[derive(Debug, Clone)]
pub struct BspPointPlaceholder {
    /// 候选点在 L0 原始 K 序的位置
    pub source_index: usize,
    /// 买卖点 bit-vector（6 bits：B1/S1/B2/S2/B3/S3）
    pub bits: u8,
    /// 该买卖点 pivot low（底分型/线段端点极值）
    pub pivot_low: f64,
    /// 该买卖点 pivot high（顶分型/线段端点极值）
    pub pivot_high: f64,
    /// 归属中枢索引（3类点必有；1/2类点可选）
    pub center_index: Option<usize>,
}

/// 走势块占位类型 - 对应 theta_v0::classifier::decompose::MoveBlock
#[derive(Debug, Clone)]
pub struct MovePlaceholder {
    /// span 首中枢下标（含；与前块共享）
    pub start_center: usize,
    /// span 尾中枢下标（含）
    pub end_center: usize,
    /// 走势类型：Trend 或 Consolidation
    pub kind: MoveKind,
    /// Trend 携方向；Consolidation = None
    pub dir: Option<Direction>,
    /// 级别升档：0 = 本级别块；1 = 高一级盘整块
    pub level_lift: u8,
    /// 状态：Active（当前走势）或 Closed（已结束）
    pub status: MoveStatus,
}

/// 走势类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveKind {
    Trend,
    Consolidation,
}

/// 方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
}

/// 走势状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveStatus {
    Active,
    Closed,
}

/// 分类引擎 trait
pub trait ClassificationEngine {
    /// 全量分类：从 bars 和 segments 到多级 Classification。
    ///
    /// 对应 `theta_v0::classifier::pipeline::classify()`。
    ///
    /// # 参数
    ///
    /// - `bars` - 原始 K 线序列
    /// - `segments` - 线段端点索引序列
    /// - `operating_levels` - 操作级别配置（哪些级别需要分类）
    ///
    /// # 返回
    ///
    /// 多级别分类结果（levels[0] = L0, levels[1] = L1, ...）
    fn classify(
        &self,
        bars: &[Bar],
        segments: &[usize],
        operating_levels: &[u32],
    ) -> ClassificationResult;

    /// 增量分类：逐bar更新已有 Classification。
    ///
    /// 对应 `theta_v0::classifier::pipeline::classify_incremental()`。
    ///
    /// # 参数
    ///
    /// - `prev` - 上一bar的分类结果
    /// - `new_bar` - 新增的bar数据
    /// - `operating_levels` - 操作级别配置
    ///
    /// # 返回
    ///
    /// 更新后的分类结果
    fn classify_incremental(
        &self,
        prev: &ClassificationResult,
        new_bar: &Bar,
        operating_levels: &[u32],
    ) -> ClassificationResult;
}
