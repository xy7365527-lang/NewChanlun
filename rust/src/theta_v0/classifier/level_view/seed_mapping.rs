//! D1 seed → CompletedMove 唯一归属证明（#640：#624 裁定 A 净减登记的 provider 侧重落地）。
//!
//! # 出处
//!
//! 本模块把 #624 裁定 A（5 删）删除的 `first_retrace_replay.rs` 中「D1 seed → 唯一 CompletedMove
//! 映射 + fail-closed 证明」按当前 `level_view` 的 [`ExactThreeProjection`] / [`CompletionStatus`]
//! 现状重落地。旧模块零生产调用点，故删除不改任何现有生产行为；但买卖点账本接真实 provider 时
//! （[`super::super::retrace_ledger`] 的入口契约「观察携带已配对的严格相邻 pair」）上游映射证明
//! 不能默认可信——本模块即该缺口的在案补齐（#640，见
//! `chanlun/review-results/issue624-fixture-replay-diff-20260729.md` §四 C）。
//!
//! # 与账本适配器残废桶的对齐（#640 范围「输出码对齐，拒收语义不重造」）
//!
//! 四类 fail-closed 码 [`StrictPairError`] 逐条对齐 [`super::super::retrace_ledger::admit_input`]
//! 的残废四桶，**不重造拒收语义**：
//!
//! | 本模块码 | 语义 | 账本适配器对齐桶 |
//! |---|---|---|
//! | [`StrictPairError::MissingSource`] | seed 不在投影里 | missing 桶的 provider 侧对应（账本入口见不到 pair，无对应码） |
//! | [`StrictPairError::NoCompletedMove`] | seed 存在但无 CompletedMove 覆盖 | 同上 |
//! | [`StrictPairError::AmbiguousCompletedMove`] | seed 被多根 CompletedMove 覆盖，归属不唯一 | 同上 |
//! | [`StrictPairError::NotAdjacent`] | leave/retest 非「不同且严格相邻」（`retest == leave + 1`） | [`super::super::retrace_ledger::RetraceRejection::NotAdjacent`]（同判据、同载荷） |
//!
//! `NotAdjacent` 一条与适配器不紧邻桶**同判据、同载荷**（`leave_move_index` / `retest_move_index`），
//! 且把旧模块的 `SameMove` 一并收进（`leave == retest` 时 `retest != leave + 1` 自然命中）——
//! 与 #624 对拍报告 B-4「`SameMove` 与 `NotAdjacent` 两码合一」逐字一致。适配器在注册期仍重验
//! 这条紧邻判据：两层防线跑的是**同一个谓词**，不是宽严两档（#799 收敛通则——不存在
//! 「严格版失败换宽松版兜底」的通道分派）。
//!
//! # 与 p83 近邻统计的关系（#624 修订补记指名核对）
//!
//! `rust/src/bin/p83_yield_remeasure.rs` 的 `unassigned_projected` 按「seed 是否落在任一 move 的
//! `center_indices`」计数（不区分 Completed/Pending，非 fail-closed、非唯一性证明），是诊断统计
//! 不是准入判据；本模块是「seed 唯一归属一根 **Completed** Move」的 fail-closed 证明。两者判的
//! 不是同一个判断（一个计未分配、一个证唯一 Completed 归属），p83 亦不参与任何准入——无双实现
//! 分叉风险。

use super::super::recursive_tower::ElementId;
use super::super::retrace_ledger::StrictCompletedPair;
use super::{CompletionStatus, ExactThreeProjection, LevelAsOfView};

/// D1 seed → CompletedMove 映射的四类 fail-closed 码（#640 重落地）。
///
/// `NotAdjacent` 与账本适配器不紧邻桶同判据同载荷（见模块头对齐表）；其余三码是 missing 桶的
/// provider 侧对应——它们命中时 pair 根本不产出，账本入口永远见不到缺料的 pair。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrictPairError {
    /// seed 不在投影里（provider 输入缺料）。
    MissingSource(ElementId),
    /// seed 存在，但没有任何一根 CompletedMove 覆盖它（0 个）。
    NoCompletedMove(ElementId),
    /// seed 同时被多根 CompletedMove 覆盖——归属不唯一，fail-closed（多个）。
    AmbiguousCompletedMove {
        source_id: ElementId,
        move_indices: Vec<usize>,
    },
    /// leave/retest 非「不同且严格相邻」（`retest != leave + 1`；`leave == retest` 亦落本码）。
    NotAdjacent {
        leave_move_index: usize,
        retest_move_index: usize,
    },
}

/// D1 seed → C2 CompletedMove 的严格映射（自 #624 删除的 `first_retrace_replay.rs:31-86`
/// 原算法形状重落地；判定面一个 bit 不改——唯一例外是旧 `SameMove` 与 `NotAdjacent` 两码按
/// #624 对拍报告 B-4 收成一码 [`StrictPairError::NotAdjacent`]，对齐账本适配器不紧邻桶）。
///
/// seed 必须在 view 中唯一属于一根 CompletedMove；leave/retest 必须是不同且正向相邻的
/// CompletedMove。Pending、缺 seed、边界处同时属于两根 CompletedMove 均 fail closed。
pub fn strict_completed_pair(
    projection: &ExactThreeProjection,
    view: &LevelAsOfView,
    leave_source: ElementId,
    retest_source: ElementId,
) -> Result<StrictCompletedPair, StrictPairError> {
    let leave = unique_completed_move(projection, view, leave_source)?;
    let retest = unique_completed_move(projection, view, retest_source)?;
    if retest != leave + 1 {
        return Err(StrictPairError::NotAdjacent {
            leave_move_index: leave,
            retest_move_index: retest,
        });
    }
    Ok(StrictCompletedPair {
        leave_move_index: leave,
        retest_move_index: retest,
    })
}

/// seed 必须唯一属于一根 CompletedMove：找不到报 [`StrictPairError::MissingSource`]、
/// 0 个报 [`StrictPairError::NoCompletedMove`]、多个报 [`StrictPairError::AmbiguousCompletedMove`]。
pub fn unique_completed_move(
    projection: &ExactThreeProjection,
    view: &LevelAsOfView,
    source_id: ElementId,
) -> Result<usize, StrictPairError> {
    let seed_index = projection
        .seeds
        .iter()
        .position(|seed| seed.source_id == source_id)
        .ok_or(StrictPairError::MissingSource(source_id))?;
    let move_indices: Vec<_> = view
        .moves
        .iter()
        .enumerate()
        .filter(|(_, value)| {
            value.center_indices.contains(&seed_index)
                && matches!(value.completion, CompletionStatus::Completed { .. })
        })
        .map(|(index, _)| index)
        .collect();
    match move_indices.as_slice() {
        [] => Err(StrictPairError::NoCompletedMove(source_id)),
        [index] => Ok(*index),
        _ => Err(StrictPairError::AmbiguousCompletedMove {
            source_id,
            move_indices,
        }),
    }
}
