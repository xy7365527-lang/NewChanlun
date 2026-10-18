//! #1424 TB-02-F：相邻中枢五叶关系（CC-020）与延伸测试单元位置（CC-018）→ S 事实对象
//! （schema `s-center-pair/1`，`.chanlun/review-results/issue1424-tb02f-20261014/tb1424-schema.md`）。
//!
//! 本模块不重新扫描、不另写判据：由 `content_structure::build_ef_units` 消费同一次扫描（#804 L2），对与位置全走
//! 库内 `center_scan::{pair_facts, tested_positions}` → `center::{center_pair_facts, unit_core_position}`（塔的
//! `classify_relation` 与延伸循环读同一判定）。本模块只做身份、修订、获知时点与 wire 投影。
//!
//! 根裁定（#1424 issuecomment-6160553615）：一端为 ≥9 窗口的对照发对象、`status = PENDING_UPGRADE_TB02H`、叶/方向/
//! 矩阵为 null（第 2 问）；扩展/新生只是关系，`realization` 固定写「未构造/未断言」（票面验收二）；离开不是破坏，
//! `destruction` 固定写待 #1434；两段式结算 R5 归 #1496（第 6 问）。
pub(super) const KINDS: &[&str] = &[
    "CC-018.unit_core_position",
    "CC-020.center_pair",
    "CC-055.pair_relation",
    "CC-055.pair_change",
    "CC-056.pair_version",
];
pub(super) const SCHEMA_REVISION: &str = "s-center-pair/1";
const SEMANTIC_VERSION: &str = "center-pair/1";
const POLICY_VERSION: &str = "zn_outer_five_leaf_pending_ge9";
const DELIVERED_SCOPE: &str = "l0_adjacent_pair_five_leaf_zn_outer_and_unit_core_position";
/// 版本对象与 CC-018/CC-020 目录轴的延后范围（带实票号）。
pub(super) const DEFERRED_SCOPE: [&str; 6] = [
    "upper_level_pairs_1429",
    "ge9_pairs_pending_1429",
    "upper_center_realization_1429",
    "trend_realization_1426",
    "first_retest_destruction_1434",
    "two_step_settlement_r5_1496",
];
pub(super) const DELIVERED_SUBOBLIGATIONS: [&str; 3] = [
    "l0_unit_core_position_five_leaves",
    "l0_adjacent_pair_five_leaves_zn_outer_with_full_matrix",
    "expansion_relation_separate_from_realization",
];
/// 本规则下扩展/新生关系的实现状态（固定字面量：本片不构造上级对象、不断言趋势已实现）。
pub(super) const REALIZATION_UPPER: &str = "NOT_CONSTRUCTED_DEFERRED_1429";
pub(super) const REALIZATION_TREND: &str = "NOT_ASSERTED_DEFERRED_1426";
/// 离开不是破坏：首回试与破坏归 #1434。
pub(super) const DESTRUCTION: &str = "NOT_EVALUATED_DEFERRED_1434";
pub(super) const PENDING: &str = "PENDING_UPGRADE_TB02H";
