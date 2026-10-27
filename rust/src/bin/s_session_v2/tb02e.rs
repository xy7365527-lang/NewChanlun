//! #1419：中心 wire 合同；常量沿用 donor 634f62cd。段输入核见 content_structure::segment_units。
use crate::v3::structure_value::{Text, Vec};
use newchan_rust::theta_v0::classifier::center::UnitRange;
pub(super) const KINDS: &[&str] = &[
    "CC-016.center_seed_attempt",
    "CC-016.center",
    "CC-017.center_roles",
    "CC-022.unit_assignment",
    "CC-055.center_relation",
    "CC-055.center_change",
    "CC-056.center_version",
];
pub(super) const SCHEMA_REVISION: &str = "s-center/1";
const SEMANTIC_VERSION: &str = "center-seed/1";
const POLICY_VERSION: &str = "strict_seed_zn_outer_c_resume";
const DELIVERED_SCOPE: &str = "l0_strict_seed_zn_roles_c_resume_assignment";
/// 版本对象与 CC-016/017/022 目录轴的延后范围（根裁定 D-J 第 4 点：带实票号）。
pub(super) const DEFERRED_SCOPE: [&str; 5] = [
    "global_G009_upper_levels_1429",
    "ge9_upgrade_1429",
    "third_class_and_break_1434",
    "expansion_newborn_1424",
    "connector_upper_construction_1429",
];
pub(super) const DELIVERED_SUBOBLIGATIONS: [&str; 3] = [
    "l0_strict_seed_vector_and_core_order",
    "zn_outer_inner_roles_with_contributors",
    "c_resume_assignment_and_connectors",
];

#[derive(Debug)]
pub(super) struct SegUnit {
    pub unit_id: Text,
    pub object_id: Text,
    pub range: Option<UnitRange>,
    pub sources: Vec<usize>,
}
pub(super) struct UnitInputs {
    pub completed: Vec<SegUnit>,
    pub in_progress: Option<SegUnit>,
}
