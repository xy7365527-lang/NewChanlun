//! 中枢震荡动作契约（组合 R：DB-B / DB-O3 / DB-S5）。
//!
//! 本模块只实现已经裁决的 DB 组语义：
//! - `[ZD,ZG]` 裸触碰不产生候选；候选必须携盘整背驰或次级别买卖点确认；
//! - `CenterOscillation` 是独立原因，不借用 B1/B2/B3 bits；
//! - 开仓创建反父方向 `ShortDiff/OscillationLot` 子腿（P9），平仓只关闭同一子腿（P7）；
//! - `target_units` 是同股数目标，成交量只经 `SizeTheta/KTheta` 上界投影；
//! - partial fill 的残量留在同一 `oscillation_id`，母腿身份、股数、方向永不改写。
//!
//! PanDiv 的 Nest/XZD 生产门属于组合 R 的 D-C，不在本模块重开。本模块接收的
//! [`ConsolidationDivergenceEvidence`] 表示上游已经确认并允许进入 DB 候选契约的证据引用。

use super::super::classifier::recursive_tower::ElementId;
use super::coverage::Vertical;
use super::interp::ExitType;
use super::mutex::MutexClass;
use super::voice::VoiceSide;

/// 默认关闭的执行开关。关闭时证据可进入协议轨，但配对子腿账本逐字段不变。
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

/// 已确认盘整背驰证据。无 `Default`/`Option` 路径，候选构造时必须实传。
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

/// 已完成次级别买卖点确认证据。构造器强制 `lower_level < operation_level`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LowerLevelBspEvidence {
    reference: OscillationEvidenceRef,
    lower_level: u32,
}

impl LowerLevelBspEvidence {
    pub fn new(
        operation_level: u32,
        lower_level: u32,
        reference: OscillationEvidenceRef,
    ) -> Result<Self, OscillationContractError> {
        if lower_level >= operation_level {
            return Err(OscillationContractError::EvidenceNotLowerLevel);
        }
        Ok(Self {
            reference,
            lower_level,
        })
    }

    pub const fn reference(self) -> OscillationEvidenceRef {
        self.reference
    }

    pub const fn lower_level(self) -> u32 {
        self.lower_level
    }
}

/// CenterOscillation 唯一合法证据和；不存在 `None`/BareTouch 构造子。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OscillationEvidence {
    ConsolidationDivergence(ConsolidationDivergenceEvidence),
    LowerLevelBsp(LowerLevelBspEvidence),
}

impl OscillationEvidence {
    pub const fn reference(self) -> OscillationEvidenceRef {
        match self {
            Self::ConsolidationDivergence(e) => e.reference(),
            Self::LowerLevelBsp(e) => e.reference(),
        }
    }
}

/// 独立候选原因；没有 B1/B2/B3 字段或转换接口。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OscillationCandidateReason {
    CenterOscillation(OscillationEvidence),
}

/// 价格相对中枢边界的位置只作语境，不单独构成候选。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BoundarySide {
    Above,
    Below,
}

/// 裸价格触碰的显式输入类型。唯一投影是空候选集。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BareBoundaryTouch {
    pub center: OscillationCenterRef,
    pub level: u32,
    pub boundary: BoundarySide,
}

/// L0 否定式契约：裸触碰永远产 0 个 CenterOscillation 候选。
pub const fn candidates_from_bare_boundary_touch(
    _touch: BareBoundaryTouch,
) -> [CenterOscillationCandidate; 0] {
    []
}

/// 活动母腿快照。母腿是只读前件，任何震荡动作都不得改写它。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OscillationParentLeg {
    id: ElementId,
    level: u32,
    side: VoiceSide,
    units: u64,
}

impl OscillationParentLeg {
    pub fn new(
        id: ElementId,
        level: u32,
        side: VoiceSide,
        units: u64,
    ) -> Result<Self, OscillationContractError> {
        if side == VoiceSide::Flat {
            return Err(OscillationContractError::FlatParent);
        }
        if units == 0 {
            return Err(OscillationContractError::ZeroParentUnits);
        }
        Ok(Self {
            id,
            level,
            side,
            units,
        })
    }

    pub const fn id(self) -> ElementId {
        self.id
    }

    pub const fn level(self) -> u32 {
        self.level
    }

    pub const fn side(self) -> VoiceSide {
        self.side
    }

    pub const fn units(self) -> u64 {
        self.units
    }
}

/// 配对子腿稳定身份：母腿 + 中枢 + 同一母腿/中枢内的显式序号。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OscillationId {
    parent_id: ElementId,
    center: OscillationCenterRef,
    sequence: u32,
}

impl OscillationId {
    pub const fn new(parent_id: ElementId, center: OscillationCenterRef, sequence: u32) -> Self {
        Self {
            parent_id,
            center,
            sequence,
        }
    }

    pub const fn parent_id(self) -> ElementId {
        self.parent_id
    }

    pub const fn center(self) -> OscillationCenterRef {
        self.center
    }

    pub const fn sequence(self) -> u32 {
        self.sequence
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OscillationIntent {
    Open,
    Close,
}

/// 独立 CenterOscillation 候选。字段私有，只有有证据的 open/close 构造器。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CenterOscillationCandidate {
    center: OscillationCenterRef,
    level: u32,
    parent_leg_id: ElementId,
    oscillation_id: OscillationId,
    boundary_side: BoundarySide,
    action_side: VoiceSide,
    intent: OscillationIntent,
    reason: OscillationCandidateReason,
    target_units: u64,
}

impl CenterOscillationCandidate {
    pub fn open(
        center: OscillationCenterRef,
        level: u32,
        parent: OscillationParentLeg,
        oscillation_id: OscillationId,
        boundary_side: BoundarySide,
        evidence: OscillationEvidence,
    ) -> Result<Self, OscillationContractError> {
        if parent.level() != level {
            return Err(OscillationContractError::LevelMismatch);
        }
        if oscillation_id.parent_id() != parent.id() || oscillation_id.center() != center {
            return Err(OscillationContractError::IdentityMismatch);
        }
        let expected_boundary = match parent.side() {
            VoiceSide::Long => BoundarySide::Above,
            VoiceSide::Short => BoundarySide::Below,
            VoiceSide::Flat => return Err(OscillationContractError::FlatParent),
        };
        if boundary_side != expected_boundary {
            return Err(OscillationContractError::BoundaryDirectionMismatch);
        }
        Ok(Self {
            center,
            level,
            parent_leg_id: parent.id(),
            oscillation_id,
            boundary_side,
            action_side: parent.side().flip(),
            intent: OscillationIntent::Open,
            reason: OscillationCandidateReason::CenterOscillation(evidence),
            target_units: parent.units(),
        })
    }

    pub fn close(
        parent: OscillationParentLeg,
        lot: OscillationLot,
        boundary_side: BoundarySide,
        evidence: OscillationEvidence,
    ) -> Result<Self, OscillationContractError> {
        if lot.parent_leg_id != parent.id() || lot.level != parent.level() {
            return Err(OscillationContractError::IdentityMismatch);
        }
        if lot.remaining_units() == 0 {
            return Err(OscillationContractError::LotAlreadyClosed);
        }
        let expected_boundary = match parent.side() {
            VoiceSide::Long => BoundarySide::Below,
            VoiceSide::Short => BoundarySide::Above,
            VoiceSide::Flat => return Err(OscillationContractError::FlatParent),
        };
        if boundary_side != expected_boundary {
            return Err(OscillationContractError::BoundaryDirectionMismatch);
        }
        Ok(Self {
            center: lot.oscillation_id.center(),
            level: lot.level,
            parent_leg_id: parent.id(),
            oscillation_id: lot.oscillation_id,
            boundary_side,
            action_side: parent.side(),
            intent: OscillationIntent::Close,
            reason: OscillationCandidateReason::CenterOscillation(evidence),
            target_units: lot.remaining_units(),
        })
    }

    pub const fn center(self) -> OscillationCenterRef {
        self.center
    }
    pub const fn level(self) -> u32 {
        self.level
    }
    pub const fn parent_leg_id(self) -> ElementId {
        self.parent_leg_id
    }
    pub const fn oscillation_id(self) -> OscillationId {
        self.oscillation_id
    }
    pub const fn boundary_side(self) -> BoundarySide {
        self.boundary_side
    }
    pub const fn action_side(self) -> VoiceSide {
        self.action_side
    }
    pub const fn intent(self) -> OscillationIntent {
        self.intent
    }
    pub const fn reason(self) -> OscillationCandidateReason {
        self.reason
    }
    pub const fn target_units(self) -> u64 {
        self.target_units
    }

    pub const fn evidence(self) -> OscillationEvidence {
        match self.reason {
            OscillationCandidateReason::CenterOscillation(e) => e,
        }
    }

    /// 协议事件轨在同成熟度内的稳定终局键。
    pub(crate) const fn stable_key(self) -> (u32, usize, usize, u32, u64, u32, usize, u32) {
        let parent = self.parent_leg_id;
        let evidence = self.evidence().reference();
        (
            self.level,
            self.center.start_index(),
            self.center.end_index(),
            parent.level,
            parent.ordinal,
            self.oscillation_id.sequence(),
            evidence.source_index(),
            evidence.generation(),
        )
    }
}

/// `SizeTheta/KTheta` 对目标量的可行投影。只允许上界裁剪，不提供固定比例/力度比例接口。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SizeKThetaProjection {
    pub size_theta_units: u64,
    pub k_theta_units: u64,
}

impl SizeKThetaProjection {
    pub const fn project(self, requested_units: u64) -> u64 {
        let size_limited = if requested_units < self.size_theta_units {
            requested_units
        } else {
            self.size_theta_units
        };
        if size_limited < self.k_theta_units {
            size_limited
        } else {
            self.k_theta_units
        }
    }
}

/// 有身份的反向 ShortDiff/OscillationLot 子腿。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OscillationLot {
    oscillation_id: OscillationId,
    parent_leg_id: ElementId,
    level: u32,
    side: VoiceSide,
    trigger_evidence: OscillationEvidenceRef,
    target_units: u64,
    opened_units: u64,
    closed_units: u64,
}

impl OscillationLot {
    pub const fn oscillation_id(self) -> OscillationId {
        self.oscillation_id
    }
    pub const fn parent_leg_id(self) -> ElementId {
        self.parent_leg_id
    }
    pub const fn level(self) -> u32 {
        self.level
    }
    pub const fn side(self) -> VoiceSide {
        self.side
    }
    pub const fn trigger_evidence(self) -> OscillationEvidenceRef {
        self.trigger_evidence
    }
    pub const fn target_units(self) -> u64 {
        self.target_units
    }
    pub const fn opened_units(self) -> u64 {
        self.opened_units
    }
    pub const fn closed_units(self) -> u64 {
        self.closed_units
    }
    pub const fn open_residual_units(self) -> u64 {
        self.target_units - self.opened_units
    }
    pub const fn remaining_units(self) -> u64 {
        self.opened_units - self.closed_units
    }
    pub const fn is_closed(self) -> bool {
        self.open_residual_units() == 0 && self.remaining_units() == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OscillationAction {
    OpenShortDiff,
    CloseShortDiff,
    Record,
}

impl OscillationAction {
    /// 复用既有 P7/P9/P10 通道，不扩 P1..P10。
    pub const fn mutex_class(self) -> MutexClass {
        match self {
            Self::OpenShortDiff => MutexClass::Cj(9),
            Self::CloseShortDiff => MutexClass::Cj(7),
            Self::Record => MutexClass::Cj(10),
        }
    }

    pub const fn exit_type(self) -> Option<ExitType> {
        match self {
            Self::CloseShortDiff => Some(ExitType::CloseShortDiff),
            Self::OpenShortDiff | Self::Record => None,
        }
    }

    pub const fn entry_role(self) -> Option<Vertical> {
        match self {
            Self::OpenShortDiff => Some(Vertical::ShortDiff),
            Self::CloseShortDiff | Self::Record => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OscillationRecordReason {
    MissingLiveParent,
    DuplicateOrClosedIdentity,
    MissingMatchedLot,
    RiskProjectionZero,
    CandidateStateMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OscillationApplyResult {
    Disabled,
    Applied {
        action: OscillationAction,
        oscillation_id: OscillationId,
        units: u64,
    },
    Recorded {
        action: OscillationAction,
        oscillation_id: OscillationId,
        reason: OscillationRecordReason,
    },
}

impl OscillationApplyResult {
    pub const fn mutex_class(self) -> Option<MutexClass> {
        match self {
            Self::Disabled => None,
            Self::Applied { action, .. } | Self::Recorded { action, .. } => {
                Some(action.mutex_class())
            }
        }
    }
}

/// 配对子腿 reducer。母腿与子腿分表，震荡 apply 只写 `lots`，从结构上禁止改母腿。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OscillationBook {
    parents: Vec<OscillationParentLeg>,
    lots: Vec<OscillationLot>,
}

impl OscillationBook {
    pub fn with_parents(parents: Vec<OscillationParentLeg>) -> Self {
        Self {
            parents,
            lots: Vec::new(),
        }
    }

    /// 用当前生产活动父腿刷新只读父表；ShortDiff 子腿账本原位保留，不另建平行账本。
    pub fn sync_live_parents(&mut self, parents: Vec<OscillationParentLeg>) {
        self.parents = parents;
    }

    pub fn parent(&self, id: ElementId) -> Option<OscillationParentLeg> {
        self.parents.iter().copied().find(|p| p.id() == id)
    }

    pub fn lot(&self, id: OscillationId) -> Option<OscillationLot> {
        self.lots
            .iter()
            .copied()
            .find(|lot| lot.oscillation_id == id)
    }

    pub fn lots(&self) -> &[OscillationLot] {
        &self.lots
    }

    /// 当前 ShortDiff 子腿对净持仓的有符号投影。
    pub fn signed_live_child_units(&self) -> i64 {
        self.lots
            .iter()
            .map(|lot| match lot.side {
                VoiceSide::Long => lot.remaining_units() as i64,
                VoiceSide::Short => -(lot.remaining_units() as i64),
                VoiceSide::Flat => 0,
            })
            .sum()
    }

    /// 门后盘整背驰的角色感知候选：恢复父方向优先平匹配腿（P7），反父方向且无匹配腿才开
    /// 新 ShortDiff（P9）。父腿缺失/不唯一或身份不明均显式 P10 Record。
    pub fn route_gated_pan_div(
        &self,
        level: u32,
        signal_side: VoiceSide,
        center: OscillationCenterRef,
        evidence: ConsolidationDivergenceEvidence,
    ) -> Result<CenterOscillationCandidate, OscillationRouteRecordReason> {
        if signal_side == VoiceSide::Flat {
            return Err(OscillationRouteRecordReason::UnknownDirection);
        }
        let evidence = OscillationEvidence::ConsolidationDivergence(evidence);
        let boundary = match signal_side {
            VoiceSide::Long => BoundarySide::Below,
            VoiceSide::Short => BoundarySide::Above,
            VoiceSide::Flat => unreachable!(),
        };

        // P7 在 P9 前：信号恢复父方向且存在同 parent+center 的实际 live 单位，只能关该腿；
        // 即使 P9 原目标尚有未成交残量，也不阻塞已成交部分的回补。
        let mut closes = self.lots.iter().copied().filter_map(|lot| {
            if lot.oscillation_id.center() != center
                || lot.level != level
                || lot.remaining_units() == 0
            {
                return None;
            }
            let parent = self.parent(lot.parent_leg_id)?;
            (parent.side() == signal_side).then_some((parent, lot))
        });
        if let Some((parent, lot)) = closes.next() {
            if closes.next().is_some() {
                return Err(OscillationRouteRecordReason::AmbiguousMatchedLot);
            }
            return CenterOscillationCandidate::close(parent, lot, boundary, evidence)
                .map_err(|_| OscillationRouteRecordReason::CandidateContractRejected);
        }

        let mut parents = self
            .parents
            .iter()
            .copied()
            .filter(|parent| parent.level() == level && parent.side().flip() == signal_side);
        let Some(parent) = parents.next() else {
            return Err(OscillationRouteRecordReason::MissingLiveParent);
        };
        if parents.next().is_some() {
            return Err(OscillationRouteRecordReason::AmbiguousLiveParent);
        }
        if self.lots.iter().any(|lot| {
            lot.parent_leg_id == parent.id()
                && lot.oscillation_id.center() == center
                && lot.remaining_units() > 0
        }) {
            return Err(OscillationRouteRecordReason::MatchingLotAlreadyLive);
        }
        let sequence = self
            .lots
            .iter()
            .filter(|lot| {
                lot.parent_leg_id == parent.id() && lot.oscillation_id.center() == center
            })
            .map(|lot| lot.oscillation_id.sequence())
            .max()
            .map_or(0, |s| s.saturating_add(1));
        CenterOscillationCandidate::open(
            center,
            level,
            parent,
            OscillationId::new(parent.id(), center, sequence),
            boundary,
            evidence,
        )
        .map_err(|_| OscillationRouteRecordReason::CandidateContractRejected)
    }

    pub fn live_child_units(&self) -> u64 {
        self.lots.iter().map(|lot| lot.remaining_units()).sum()
    }

    pub fn total_opened_units(&self) -> u64 {
        self.lots.iter().map(|lot| lot.opened_units()).sum()
    }

    pub fn total_closed_units(&self) -> u64 {
        self.lots.iter().map(|lot| lot.closed_units()).sum()
    }

    pub fn units_conserved(&self) -> bool {
        self.total_opened_units() == self.total_closed_units() + self.live_child_units()
            && self.lots.iter().all(|lot| {
                lot.closed_units <= lot.opened_units && lot.opened_units <= lot.target_units
            })
    }

    pub fn apply(
        &mut self,
        candidate: CenterOscillationCandidate,
        config: CenterOscillationConfig,
        projection: SizeKThetaProjection,
    ) -> OscillationApplyResult {
        if !config.enabled {
            return OscillationApplyResult::Disabled;
        }
        let id = candidate.oscillation_id();
        let Some(parent) = self.parent(candidate.parent_leg_id()) else {
            return Self::record(id, OscillationRecordReason::MissingLiveParent);
        };
        match candidate.intent() {
            OscillationIntent::Open => {
                if candidate.action_side() != parent.side().flip()
                    || candidate.target_units() != parent.units()
                {
                    return Self::record(id, OscillationRecordReason::CandidateStateMismatch);
                }
                if let Some(index) = self.lots.iter().position(|lot| lot.oscillation_id == id) {
                    let lot = &mut self.lots[index];
                    if lot.is_closed() || lot.open_residual_units() == 0 {
                        return Self::record(
                            id,
                            OscillationRecordReason::DuplicateOrClosedIdentity,
                        );
                    }
                    let units = projection.project(lot.open_residual_units());
                    if units == 0 {
                        return Self::record(id, OscillationRecordReason::RiskProjectionZero);
                    }
                    lot.opened_units += units;
                    return OscillationApplyResult::Applied {
                        action: OscillationAction::OpenShortDiff,
                        oscillation_id: id,
                        units,
                    };
                }
                let units = projection.project(candidate.target_units());
                if units == 0 {
                    return Self::record(id, OscillationRecordReason::RiskProjectionZero);
                }
                self.lots.push(OscillationLot {
                    oscillation_id: id,
                    parent_leg_id: parent.id(),
                    level: candidate.level(),
                    side: candidate.action_side(),
                    trigger_evidence: candidate.evidence().reference(),
                    target_units: candidate.target_units(),
                    opened_units: units,
                    closed_units: 0,
                });
                OscillationApplyResult::Applied {
                    action: OscillationAction::OpenShortDiff,
                    oscillation_id: id,
                    units,
                }
            }
            OscillationIntent::Close => {
                let Some(index) = self.lots.iter().position(|lot| lot.oscillation_id == id) else {
                    return Self::record(id, OscillationRecordReason::MissingMatchedLot);
                };
                let lot = &mut self.lots[index];
                if lot.parent_leg_id != parent.id()
                    || lot.side != parent.side().flip()
                    || candidate.target_units() < lot.remaining_units()
                {
                    return Self::record(id, OscillationRecordReason::CandidateStateMismatch);
                }
                let units = projection.project(lot.remaining_units());
                if units == 0 {
                    return Self::record(id, OscillationRecordReason::RiskProjectionZero);
                }
                lot.closed_units += units;
                OscillationApplyResult::Applied {
                    action: OscillationAction::CloseShortDiff,
                    oscillation_id: id,
                    units,
                }
            }
        }
    }

    fn record(id: OscillationId, reason: OscillationRecordReason) -> OscillationApplyResult {
        OscillationApplyResult::Recorded {
            action: OscillationAction::Record,
            oscillation_id: id,
            reason,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OscillationRouteRecordReason {
    MissingLiveParent,
    AmbiguousLiveParent,
    AmbiguousMatchedLot,
    MatchingLotAlreadyLive,
    UnknownDirection,
    CandidateContractRejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OscillationContractError {
    FlatParent,
    ZeroParentUnits,
    EvidenceNotLowerLevel,
    LevelMismatch,
    IdentityMismatch,
    BoundaryDirectionMismatch,
    OpenResidualPending,
    LotAlreadyClosed,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn element(level: u32, ordinal: u64) -> ElementId {
        ElementId { level, ordinal }
    }

    fn parent(side: VoiceSide, units: u64) -> OscillationParentLeg {
        OscillationParentLeg::new(element(2, 7), 2, side, units).unwrap()
    }

    fn center() -> OscillationCenterRef {
        OscillationCenterRef::new(100, 130)
    }

    fn pan(seed: usize) -> OscillationEvidence {
        OscillationEvidence::ConsolidationDivergence(ConsolidationDivergenceEvidence::new(
            OscillationEvidenceRef::new(seed, 0),
        ))
    }

    fn lower(seed: usize) -> OscillationEvidence {
        OscillationEvidence::LowerLevelBsp(
            LowerLevelBspEvidence::new(2, 1, OscillationEvidenceRef::new(seed, 0)).unwrap(),
        )
    }

    fn projection(units: u64) -> SizeKThetaProjection {
        SizeKThetaProjection {
            size_theta_units: units,
            k_theta_units: units,
        }
    }

    fn open_candidate(parent: OscillationParentLeg, sequence: u32) -> CenterOscillationCandidate {
        let boundary = match parent.side() {
            VoiceSide::Long => BoundarySide::Above,
            VoiceSide::Short => BoundarySide::Below,
            VoiceSide::Flat => unreachable!(),
        };
        CenterOscillationCandidate::open(
            center(),
            2,
            parent,
            OscillationId::new(parent.id(), center(), sequence),
            boundary,
            pan(200 + sequence as usize),
        )
        .unwrap()
    }

    #[test]
    fn center_oscillation_candidate_requires_evidence() {
        let p = parent(VoiceSide::Long, 100);
        let candidate = open_candidate(p, 0);
        assert!(matches!(
            candidate.reason(),
            OscillationCandidateReason::CenterOscillation(
                OscillationEvidence::ConsolidationDivergence(_)
            )
        ));
        assert_eq!(candidate.target_units(), 100);
        // 编译期契约：open/close 构造器的 evidence 参数不是 Option，且 Evidence 无 BareTouch 构造子。
    }

    #[test]
    fn bare_boundary_touch_emits_zero_candidates() {
        let touch = BareBoundaryTouch {
            center: center(),
            level: 2,
            boundary: BoundarySide::Above,
        };
        assert!(candidates_from_bare_boundary_touch(touch).is_empty());
    }

    #[test]
    fn lower_level_evidence_is_strictly_lower() {
        assert!(LowerLevelBspEvidence::new(2, 2, OscillationEvidenceRef::new(1, 0)).is_err());
        assert!(matches!(lower(1), OscillationEvidence::LowerLevelBsp(_)));
    }

    #[test]
    fn center_candidate_maps_only_p7_or_p9_or_p10() {
        assert_eq!(
            OscillationAction::OpenShortDiff.mutex_class(),
            MutexClass::Cj(9)
        );
        assert_eq!(
            OscillationAction::CloseShortDiff.mutex_class(),
            MutexClass::Cj(7)
        );
        assert_eq!(OscillationAction::Record.mutex_class(), MutexClass::Cj(10));
        assert_eq!(
            OscillationAction::CloseShortDiff.exit_type(),
            Some(ExitType::CloseShortDiff)
        );
        assert_eq!(
            OscillationAction::OpenShortDiff.entry_role(),
            Some(Vertical::ShortDiff)
        );
    }

    #[test]
    fn open_creates_child_without_mutating_parent() {
        let p = parent(VoiceSide::Long, 100);
        let before = p;
        let mut book = OscillationBook::with_parents(vec![p]);
        let candidate = open_candidate(p, 0);
        let result = book.apply(
            candidate,
            CenterOscillationConfig { enabled: true },
            projection(100),
        );
        assert!(matches!(
            result,
            OscillationApplyResult::Applied {
                action: OscillationAction::OpenShortDiff,
                units: 100,
                ..
            }
        ));
        assert_eq!(book.parent(p.id()), Some(before));
        let lot = book.lot(candidate.oscillation_id()).unwrap();
        assert_eq!(lot.parent_leg_id(), p.id());
        assert_eq!(lot.side(), VoiceSide::Short);
        assert_eq!(lot.opened_units(), p.units());
    }

    #[test]
    fn no_naked_shortdiff_missing_parent_records_p10() {
        let p = parent(VoiceSide::Long, 100);
        let candidate = open_candidate(p, 0);
        let mut book = OscillationBook::default();
        let result = book.apply(
            candidate,
            CenterOscillationConfig { enabled: true },
            projection(100),
        );
        assert!(matches!(
            result,
            OscillationApplyResult::Recorded {
                action: OscillationAction::Record,
                reason: OscillationRecordReason::MissingLiveParent,
                ..
            }
        ));
        assert_eq!(result.mutex_class(), Some(MutexClass::Cj(10)));
        assert!(book.lots().is_empty());
    }

    #[test]
    fn close_matches_exact_oscillation_child_and_preserves_parent() {
        let p = parent(VoiceSide::Long, 80);
        let mut book = OscillationBook::with_parents(vec![p]);
        let open = open_candidate(p, 3);
        book.apply(
            open,
            CenterOscillationConfig { enabled: true },
            projection(80),
        );
        let lot = book.lot(open.oscillation_id()).unwrap();
        let close =
            CenterOscillationCandidate::close(p, lot, BoundarySide::Below, lower(300)).unwrap();
        let result = book.apply(
            close,
            CenterOscillationConfig { enabled: true },
            projection(80),
        );
        assert_eq!(result.mutex_class(), Some(MutexClass::Cj(7)));
        assert_eq!(book.parent(p.id()), Some(p));
        assert!(book.lot(open.oscillation_id()).unwrap().is_closed());
    }

    #[test]
    fn oscillation_target_equals_matched_units_and_risk_projection_never_exceeds_target() {
        let p = parent(VoiceSide::Short, 101);
        let open = open_candidate(p, 1);
        assert_eq!(open.target_units(), p.units());
        let cap = SizeKThetaProjection {
            size_theta_units: 70,
            k_theta_units: 40,
        };
        assert_eq!(cap.project(open.target_units()), 40);
        assert!(cap.project(open.target_units()) <= open.target_units());
    }

    #[test]
    fn partial_fill_preserves_residual_identity() {
        let p = parent(VoiceSide::Long, 100);
        let mut book = OscillationBook::with_parents(vec![p]);
        let open = open_candidate(p, 9);
        book.apply(
            open,
            CenterOscillationConfig { enabled: true },
            projection(30),
        );
        let first = book.lot(open.oscillation_id()).unwrap();
        assert_eq!(first.open_residual_units(), 70);
        book.apply(
            open,
            CenterOscillationConfig { enabled: true },
            projection(70),
        );
        let second = book.lot(open.oscillation_id()).unwrap();
        assert_eq!(first.oscillation_id(), second.oscillation_id());
        assert_eq!(second.open_residual_units(), 0);
    }

    #[test]
    fn oscillation_round_trip_units_conserved() {
        let p = parent(VoiceSide::Long, 100);
        let mut book = OscillationBook::with_parents(vec![p]);
        let open = open_candidate(p, 2);
        book.apply(
            open,
            CenterOscillationConfig { enabled: true },
            projection(40),
        );
        book.apply(
            open,
            CenterOscillationConfig { enabled: true },
            projection(60),
        );
        let close = CenterOscillationCandidate::close(
            p,
            book.lot(open.oscillation_id()).unwrap(),
            BoundarySide::Below,
            lower(400),
        )
        .unwrap();
        book.apply(
            close,
            CenterOscillationConfig { enabled: true },
            projection(25),
        );
        let close_rest = CenterOscillationCandidate::close(
            p,
            book.lot(open.oscillation_id()).unwrap(),
            BoundarySide::Below,
            lower(401),
        )
        .unwrap();
        book.apply(
            close_rest,
            CenterOscillationConfig { enabled: true },
            projection(75),
        );
        assert_eq!(book.total_opened_units(), 100);
        assert_eq!(book.total_closed_units(), 100);
        assert_eq!(book.live_child_units(), 0);
        assert!(book.units_conserved());
    }

    #[test]
    fn disabled_path_is_frozen_book_bit_exact() {
        let p = parent(VoiceSide::Long, 100);
        let mut book = OscillationBook::with_parents(vec![p]);
        let before = book.clone();
        let result = book.apply(
            open_candidate(p, 0),
            CenterOscillationConfig::default(),
            projection(100),
        );
        assert_eq!(result, OscillationApplyResult::Disabled);
        assert_eq!(book, before);
    }

    #[test]
    fn shortdiff_parent_invariant_arbitrary_interleavings_property() {
        let p = parent(VoiceSide::Long, 37);
        let mut book = OscillationBook::with_parents(vec![p]);
        let frozen_parent = p;
        let enabled = CenterOscillationConfig { enabled: true };
        let mut seed = 0x5eed_u64;
        for step in 0..2_000u32 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let sequence = ((seed >> 32) % 17) as u32;
            let id = OscillationId::new(p.id(), center(), sequence);
            match seed % 3 {
                0 | 1 => {
                    let candidate = open_candidate(p, sequence);
                    let cap = 1 + ((seed >> 8) % p.units());
                    let _ = book.apply(candidate, enabled, projection(cap));
                }
                _ => {
                    if let Some(lot) = book.lot(id) {
                        if lot.open_residual_units() == 0 && lot.remaining_units() > 0 {
                            let candidate = CenterOscillationCandidate::close(
                                p,
                                lot,
                                BoundarySide::Below,
                                lower(1_000 + step as usize),
                            )
                            .unwrap();
                            let cap = 1 + ((seed >> 16) % lot.remaining_units());
                            let _ = book.apply(candidate, enabled, projection(cap));
                        }
                    }
                }
            }
            assert_eq!(book.parent(p.id()), Some(frozen_parent));
            assert!(book.units_conserved());
            for lot in book.lots() {
                assert_eq!(lot.parent_leg_id(), p.id());
                assert_eq!(lot.side(), p.side().flip());
                assert!(lot.closed_units() <= lot.opened_units());
                assert!(lot.opened_units() <= lot.target_units());
            }
        }
    }
}
