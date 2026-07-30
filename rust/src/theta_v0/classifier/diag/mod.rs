//! 诊断驱动器与观测面（#748/C4：从 `classifier/mod.rs` 收窄出的诊断子树，纯移动零行为）。
//!
//! 与生产分类主路径（`classify`/`classify_with_tower`/`classify_with_tower_incremental` 等五
//! 入口，仍在 `classifier/mod.rs`）分层：本树只产诊断读数（计数器/耗时/召回上界/候选事件对拍），
//! 不参与任何分类、交易、订单或风控判定分支。

pub mod cand_delta;
pub mod cp_recall_audit;
pub mod cp_replay_diagnostics;
#[cfg(test)]
pub mod oracle_probe;
pub mod stage_profile;

pub use cand_delta::{cand_delta_entry_tower, cand_delta_tower, cand_delta_tower_cached};
pub use cp_recall_audit::cp_recall_upper_bound_audit;
