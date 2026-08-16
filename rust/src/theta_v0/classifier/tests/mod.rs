//! classifier 测试层（#648 T1：自 mod.rs 内联 `mod tests` ~4273 行纯移动抽离，零行为变更）。
//! 主题分件参照 #497 level_view/tests 先例；夹具集中在 fixtures.rs。

mod fixtures;
mod incremental_parity;
mod incremental_profile;
mod operation;
mod pipeline_geometry;
mod rebase_txn;
mod third_class_anchor;
