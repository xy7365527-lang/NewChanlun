//! 中枢震荡触发链契约（组合 R：DB-B / DB-O3 / DB-S5 的触发侧残余）。
//!
//! #282（#280 裁定，ADR 0001 修正案一 修1）：S6 开空腿账面形态全套已删除——
//! `OscillationBook`（parents/lots 双表 + apply）、`OscillationLot`、`OscillationId`、
//! `CenterOscillationCandidate`（父数量 target / 父翻转 action_side 的全量反翻子腿规格）、
//! `OscillationAction`/`OscillationApplyResult`/`OscillationRecordReason`、
//! `OscillationRouteRecordReason`、`SizeKThetaProjection`、`OscillationParentLeg`、
//! `OscillationContractError`、`LowerLevelBspEvidence`/`OscillationEvidence`/
//! `OscillationCandidateReason`（仅喂账面构造的证据包装）全部移除，git 历史可回溯。
//!
//! **保留（#274 狭义短差实装原料，修2 语境）——触发链三件套**：
//! 1. 盘背 PanDiv 证据：[`ConsolidationDivergenceEvidence`]（证书生产在 classifier/signal.rs，
//!    生产门在 backtest/econ_positive.rs，均不动）；
//! 2. 本级中枢上下沿：[`OscillationCenterRef`] + [`BoundarySide`]（裸触碰 L0 否定式契约
//!    [`triggers_from_bare_boundary_touch`] 不动——裸触碰永不产触发）；
//! 3. 触发事件 [`PanDivTrigger`]：门后盘背信号 + 级别 + 中枢身份 + 边界侧的**纯触发事实**，
//!    不携任何账面/动作语义（无父数量、无父翻转、无开平仓意向）——账面动作待 #274
//!    按修2/修4（该级账内减仓回补）重新设计。
//!
//! `CenterOscillationConfig` 开关本体保留（默认关，#274 落点门控 seam）。

use super::voice::VoiceSide;

/// 默认关闭的执行开关。关闭时证据可进入协议轨，但配对子腿账本逐字段不变。
/// （#282：子腿账本已删；开关保留为 #274 狭义短差实装的既有落点门控。）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CenterOscillationConfig {
    pub enabled: bool,
}

impl Default for CenterOscillationConfig {
    fn default() -> Self {
        Self { enabled: false }
    }
}

/// 中枢稳定身份（canonical completed center 的坐标投影）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OscillationCenterRef {
    start_index: usize,
    end_index: usize,
}

impl OscillationCenterRef {
    pub const fn new(start_index: usize, end_index: usize) -> Self {
        Self {
            start_index,
            end_index,
        }
    }

    pub const fn start_index(self) -> usize {
        self.start_index
    }

    pub const fn end_index(self) -> usize {
        self.end_index
    }
}

/// 不可伪造为空值的证据引用。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OscillationEvidenceRef {
    source_index: usize,
    generation: u32,
}

impl OscillationEvidenceRef {
    pub const fn new(source_index: usize, generation: u32) -> Self {
        Self {
            source_index,
            generation,
        }
    }

    pub const fn source_index(self) -> usize {
        self.source_index
    }

    pub const fn generation(self) -> u32 {
        self.generation
    }
}

/// 已确认盘整背驰证据。无 `Default`/`Option` 路径，触发构造时必须实传。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConsolidationDivergenceEvidence {
    reference: OscillationEvidenceRef,
}

impl ConsolidationDivergenceEvidence {
    pub const fn new(reference: OscillationEvidenceRef) -> Self {
        Self { reference }
    }

    pub const fn reference(self) -> OscillationEvidenceRef {
        self.reference
    }
}

/// 价格相对中枢边界的位置只作语境，不单独构成触发。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BoundarySide {
    Above,
    Below,
}

/// 裸价格触碰的显式输入类型。唯一投影是空触发集。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BareBoundaryTouch {
    pub center: OscillationCenterRef,
    pub level: u32,
    pub boundary: BoundarySide,
}

/// L0 否定式契约：裸触碰永远产 0 个中枢震荡触发。
pub const fn triggers_from_bare_boundary_touch(
    _touch: BareBoundaryTouch,
) -> [PanDivTrigger; 0] {
    []
}

/// 触发构造的 typed 拒绝（无静默兜底）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanDivTriggerError {
    /// 信号方向为 Flat——触发必须有向。
    FlatSignal,
}

/// #282 保留的触发链产出物（#274 狭义短差实装原料）：门后盘背 PanDiv + 本级中枢上下沿
/// 的**纯触发事实**——级别、信号方向、中枢身份、边界侧、盘背证据。不携账面语义：
/// 无父数量、无父翻转、无开/平仓意向（修1 废止的 S6 形态三要素全部不在类型面）。
///
/// 全序派生供协议轨确定性合并（同 bar 多触发按派生序取最大，交换/结合/幂等由全序保证）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PanDivTrigger {
    level: u32,
    signal_side: VoiceSide,
    center: OscillationCenterRef,
    boundary_side: BoundarySide,
    evidence: ConsolidationDivergenceEvidence,
}

impl PanDivTrigger {
    /// 触发链唯一构造点：门后盘背信号 → 本级中枢上下沿映射（`Long` 信号 = 下沿回试
    /// `Below`、`Short` 信号 = 上沿回试 `Above`；与 #280 前 `route_gated_pan_div` 的
    /// 边界映射同口径）。`Flat` 信号显式 typed 拒绝——触发必须有向。
    pub fn from_gated_pan_div(
        level: u32,
        signal_side: VoiceSide,
        center: OscillationCenterRef,
        evidence: ConsolidationDivergenceEvidence,
    ) -> Result<Self, PanDivTriggerError> {
        let boundary_side = match signal_side {
            VoiceSide::Long => BoundarySide::Below,
            VoiceSide::Short => BoundarySide::Above,
            VoiceSide::Flat => return Err(PanDivTriggerError::FlatSignal),
        };
        Ok(Self {
            level,
            signal_side,
            center,
            boundary_side,
            evidence,
        })
    }

    pub const fn level(self) -> u32 {
        self.level
    }

    pub const fn signal_side(self) -> VoiceSide {
        self.signal_side
    }

    pub const fn center(self) -> OscillationCenterRef {
        self.center
    }

    pub const fn boundary_side(self) -> BoundarySide {
        self.boundary_side
    }

    pub const fn evidence(self) -> ConsolidationDivergenceEvidence {
        self.evidence
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn center() -> OscillationCenterRef {
        OscillationCenterRef::new(100, 130)
    }

    fn pan(seed: usize) -> ConsolidationDivergenceEvidence {
        ConsolidationDivergenceEvidence::new(OscillationEvidenceRef::new(seed, 0))
    }

    #[test]
    fn bare_boundary_touch_emits_zero_triggers() {
        let touch = BareBoundaryTouch {
            center: center(),
            level: 2,
            boundary: BoundarySide::Above,
        };
        assert!(triggers_from_bare_boundary_touch(touch).is_empty());
    }

    /// 触发链边界映射：Long 信号 = 中枢下沿、Short 信号 = 中枢上沿；级别/中枢/证据原样入账。
    #[test]
    fn trigger_maps_signal_side_to_opposite_boundary() {
        let long = PanDivTrigger::from_gated_pan_div(2, VoiceSide::Long, center(), pan(200)).unwrap();
        assert_eq!(long.boundary_side(), BoundarySide::Below);
        assert_eq!(long.level(), 2);
        assert_eq!(long.signal_side(), VoiceSide::Long);
        assert_eq!(long.center(), center());
        assert_eq!(long.evidence(), pan(200));
        let short = PanDivTrigger::from_gated_pan_div(1, VoiceSide::Short, center(), pan(201)).unwrap();
        assert_eq!(short.boundary_side(), BoundarySide::Above);
        assert!(long > short, "派生全序确定性（协议轨合并键：字段序 level 先行）");
    }

    /// Flat 信号不构成触发（typed 拒绝，无静默兜底）。
    #[test]
    fn flat_signal_is_not_a_trigger() {
        assert_eq!(
            PanDivTrigger::from_gated_pan_div(2, VoiceSide::Flat, center(), pan(202)),
            Err(PanDivTriggerError::FlatSignal)
        );
    }

    /// 触发事件类型面不携账面语义：无父数量、无父翻转、无开/平仓意向字段
    /// （编译期契约——字段集即文档；修1 废止形态三要素不可构造）。
    #[test]
    fn trigger_carries_no_bookkeeping_semantics() {
        let t = PanDivTrigger::from_gated_pan_div(2, VoiceSide::Long, center(), pan(203)).unwrap();
        // 触发事实五要素齐备（级别/方向/中枢/边界/证据），除此之外无其他字段。
        let _ = (t.level(), t.signal_side(), t.center(), t.boundary_side(), t.evidence());
    }
}
