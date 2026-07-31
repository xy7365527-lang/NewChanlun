//! P52 只读召回上界：不经过 `level_cand_delta` / `cand_delta` 事件入口，直接在每级稳定
//! `B_p/c_p` 对象全集上重判 leave/retest 几何。
//!
//! #748（C4）纯移动自 `classifier/mod.rs`（原 `cp_recall_upper_bound_audit`）。

use std::rc::Rc;

use crate::theta_v0::classifier::center::UnitRange;
use crate::theta_v0::classifier::recursive_tower::{
    project_to_units, CpRecallAuditCase, LeveledMove,
};
use crate::theta_v0::classifier::{decompose, recursive_tower, segment_to_unit, Classification};
use crate::theta_v0::parser::ParseLayer;
use crate::theta_v0::types::Direction;

pub fn cp_recall_upper_bound_audit(
    l0: &ParseLayer,
    classification: &Classification,
    tower_snapshots: &[Rc<Vec<LeveledMove>>],
) -> Vec<CpRecallAuditCase> {
    let mut out = Vec::new();
    for (level, state) in classification.levels.iter().enumerate() {
        let Some(unit_moves) = tower_snapshots.get(level) else {
            continue;
        };
        if level == 0 {
            let units: Vec<UnitRange> = l0.segments.iter().map(segment_to_unit).collect();
            out.extend(recursive_tower::audit_cp_recall_upper_bound(
                0,
                &state.centers,
                &state.cp_ownership,
                &units,
                unit_moves,
                None,
            ));
        } else {
            let parent_blocks = &classification.levels[level - 1].moves;
            let units = project_to_units(unit_moves, parent_blocks);
            let anchors: Vec<Option<Direction>> = (0..units.len())
                .map(|i| decompose::center_own_dir_at(parent_blocks, i))
                .collect();
            out.extend(recursive_tower::audit_cp_recall_upper_bound(
                level as u32,
                &state.centers,
                &state.cp_ownership,
                &units,
                unit_moves,
                Some(&anchors),
            ));
        }
    }
    out
}
