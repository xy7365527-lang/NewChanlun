//! 观察适配器（票 #621 S1 第一段）：原始观察 → 已验证观察，残废件**注册期拒收报错**。
//!
//! # 错误与终态分家（#574 裁定三）
//!
//! 本模块产出的 [`RetraceRejection`] 是**错误**，不是终态：残废输入根本没资格进账本，
//! 既不建仓、也不产任何修订、更不写 `NotConstituted`。判败（`NotConstituted{RetestReentered}`）
//! 是「合法候选竞选失败」，与本模块的拒收在语义上不可混。
//!
//! # 残废四桶
//!
//! | 桶 | 判据 | 错误码 |
//! |---|---|---|
//! | leave 未出中枢 | 向上离开须 `leave_end.price > zg`；向下离开须 `< zd` | [`RetraceRejection::LeaveNotOutsideCenter`] |
//! | 回抽同向 | `retest_direction == leave_direction` | [`RetraceRejection::RetestSameDirection`] |
//! | 不紧邻 | `retest_move_index != leave_move_index + 1`（补充十五） | [`RetraceRejection::NotAdjacent`] |
//! | missing | 框退化（`zd > zg` 或 `start_index >= end_index`）/ 判定结局却无回抽位置 | [`RetraceRejection::MalformedFrame`] / [`RetraceRejection::MissingRetestPosition`] |
//!
//! 第五码 [`RetraceRejection::ActiveCandidateNotSettled`] 属裁定二（单活跃候选纪律），
//! 判定需要账本态，故由 [`super::book::RetraceLedger::observe`] 在本模块四桶之后施加。

use super::super::super::types::Direction;
use super::super::first_retrace_replay::{RetraceOutcome, StrictCompletedPair};
use super::book::CenterDeathCertificate;
use super::{
    CenterAnchor, CenterFrame, RetraceEvidence, RetraceKey, RetraceObservation, RetracePoint,
    RetraceSide,
};

/// 观察适配器的原始输入（票面「观察进」的唯一入口）。
///
/// 契约字段对照：`pair`（严格相邻 pair）、`outcome`（结局）、`center`（当前中枢窗口）、
/// `as_of`（知情时）；其余字段是四桶判据与位置证据所必需的材料。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetraceInput {
    /// 当前中枢窗口（引擎当拍报的四条边）——注册时拍成身份的「临时右边」。
    pub center: CenterFrame,
    /// 严格相邻 CompletedMove pair（`leave` = 离开中枢那根，`retest` = 回抽那根）。
    pub pair: StrictCompletedPair,
    /// 离开方向（决定候选侧：Up ⟹ 三类买点）。
    pub leave_direction: Direction,
    /// 回抽方向（须与离开反向）。
    pub retest_direction: Direction,
    /// 离开笔终点（「离开的边」位置证据）。
    pub leave_end: RetracePoint,
    /// 回抽笔终点（三类点位置证据）；未决时诚实 `None`。
    pub retest_end: Option<RetracePoint>,
    /// 结局；`None` = 未决（裁定七：沉默 = 未决，永不超时处死）。
    pub outcome: Option<RetraceOutcome>,
    /// 知情时。
    pub as_of: usize,
}

/// 注册期拒收（残废四桶 + 单活跃候选纪律）：**错误，不是终态**（裁定三）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetraceRejection {
    /// missing 桶之一：中枢框退化（价沿倒挂或时间边非严格递增）。
    MalformedFrame { frame: CenterFrame },
    /// missing 桶之二：已给结局却无回抽位置——判案材料缺。
    MissingRetestPosition { as_of: usize },
    /// leave 未出中枢：离开笔终点仍未越过对应价沿。
    LeaveNotOutsideCenter {
        frame: CenterFrame,
        leave_end: RetracePoint,
        leave_direction: Direction,
    },
    /// 回抽同向：回抽必须与离开反向。
    RetestSameDirection { direction: Direction },
    /// 不紧邻：`retest != leave + 1`（补充十五「任一失败即 None，禁止后扫替代配对」）。
    NotAdjacent {
        leave_move_index: usize,
        retest_move_index: usize,
    },
    /// 同一中枢已有未判完的活跃候选，而新 departure（不同 departure_move_index）到达
    /// （裁定二：provider 有病 ⟹ fail-loud）。**不含**同 departure 改口——那走引擎改口处死路径
    /// （票 #622；[`super::NotConstitutedReason::CenterRebased`]），不是拒收。
    ActiveCandidateNotSettled {
        active: RetraceKey,
        incoming_departure_move_index: usize,
    },
    /// 死人挂号：该中枢锚已有死亡证明（Success 已落锤），仍收到新 departure（裁定四「给死人
    /// 挂号」）。由 [`super::book::RetraceLedger::observe`] 施加，自查
    /// [`super::book::RetraceLedger::death_certificate`]，不依赖外部（票 #622）。
    ///
    /// 与同身份迟到输入（幂等，裁定三静默吸收）严格区分：本码只在**新身份**试图挂号时触发，
    /// 迟到输入走 `advance_existing` 的终态吸收分支，从不经过本模块。
    DeadCenterReentry {
        anchor: CenterAnchor,
        attempted: RetraceKey,
        death_certificate: CenterDeathCertificate,
    },
    /// 两拍证据矛盾：同一身份注册拍与落锤拍的侧 / 离开边不一致（裁定二 fail-loud 同族；
    /// 影子评审 #621 MEDIUM-2 补，票 #622）。frame 不比对——落到本码时二者必然同 key 故同 frame。
    TerminalEvidenceContradictsRegistration {
        key: RetraceKey,
        registered_side: RetraceSide,
        registered_leave_end: RetracePoint,
        incoming_side: RetraceSide,
        incoming_leave_end: RetracePoint,
    },
    /// 改口处死知情时护栏：`kill_as_rebased` 落锤前 `as_of` 早于旧档门卫钟（裁定二 fail-loud
    /// 同族；影子评审 #622 S2 HIGH-1 修复，票 #622 S2 修复轮）。由
    /// [`super::book::RetraceLedger::observe`] 的 `observe` 碰撞分支与
    /// [`super::book::RetraceLedger::reconcile_window`] 两条通道共用，不经 `admit_input`——
    /// `kill_as_rebased` 是唯一不过 `LedgerBook::admit` 倒退门的终态落账路径，故须自带这道护栏，
    /// 否则可静默写出违反内核「出生钟 ≤ 终态钟」不变量的账本条目。
    RebaseAsOfBehindGate {
        key: RetraceKey,
        /// 旧档门卫钟（该身份已见最大知情时）。
        gate_as_of: usize,
        /// 被拒绝的处死知情时。
        as_of: usize,
    },
}

/// 适配器输出：已验证观察（内核建项入口）+ 注册拍证据（钉进留档的载荷）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdmittedObservation {
    pub observation: RetraceObservation,
    pub evidence: RetraceEvidence,
}

/// 原始观察 → 已验证观察：过四桶即拍身份，任一桶命中即 fail-loud。
pub fn admit_input(input: &RetraceInput) -> Result<AdmittedObservation, RetraceRejection> {
    check_frame(input)?;
    check_adjacency(input)?;
    check_leave_outside_center(input)?;
    check_retest_opposes_leave(input)?;
    check_material_completeness(input)?;
    let key = RetraceKey {
        frame: input.center,
        departure_move_index: input.pair.leave_move_index,
    };
    Ok(AdmittedObservation {
        observation: RetraceObservation {
            key,
            outcome: input.outcome,
            as_of: input.as_of,
        },
        evidence: RetraceEvidence {
            frame: input.center,
            side: RetraceSide::from_departure(input.leave_direction),
            leave_end: input.leave_end,
            retest_end: input.retest_end,
        },
    })
}

/// missing 桶之一：框必须是一个真实的判案锚。
fn check_frame(input: &RetraceInput) -> Result<(), RetraceRejection> {
    let frame = input.center;
    if frame.zd > frame.zg || frame.start_index >= frame.end_index {
        return Err(RetraceRejection::MalformedFrame { frame });
    }
    Ok(())
}

/// 不紧邻桶：`retest = leave + 1`（补充十五紧邻语义）。
fn check_adjacency(input: &RetraceInput) -> Result<(), RetraceRejection> {
    let StrictCompletedPair {
        leave_move_index,
        retest_move_index,
    } = input.pair;
    if retest_move_index != leave_move_index + 1 {
        return Err(RetraceRejection::NotAdjacent {
            leave_move_index,
            retest_move_index,
        });
    }
    Ok(())
}

/// leave 未出中枢桶：向上离开须越 ZG，向下离开须破 ZD（严格不含等值——中枢边界闭区间）。
fn check_leave_outside_center(input: &RetraceInput) -> Result<(), RetraceRejection> {
    let frame = input.center;
    let outside = match input.leave_direction {
        Direction::Up => input.leave_end.price > frame.zg,
        Direction::Down => input.leave_end.price < frame.zd,
    };
    if outside {
        return Ok(());
    }
    Err(RetraceRejection::LeaveNotOutsideCenter {
        frame,
        leave_end: input.leave_end,
        leave_direction: input.leave_direction,
    })
}

/// 回抽同向桶。
fn check_retest_opposes_leave(input: &RetraceInput) -> Result<(), RetraceRejection> {
    if input.retest_direction == input.leave_direction {
        return Err(RetraceRejection::RetestSameDirection {
            direction: input.leave_direction,
        });
    }
    Ok(())
}

/// missing 桶之二：给了结局就必须给判案位置（未决则允许缺）。
fn check_material_completeness(input: &RetraceInput) -> Result<(), RetraceRejection> {
    if input.outcome.is_some() && input.retest_end.is_none() {
        return Err(RetraceRejection::MissingRetestPosition {
            as_of: input.as_of,
        });
    }
    Ok(())
}
