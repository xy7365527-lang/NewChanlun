//! Θ_level + Θ_signal 子模块（reference-theta-v0.md:27-37）。
//!
//! ## 契约重锚（legacy Strict/* → Origin canonical，task #127 A′ Phase2）
//!
//! 递归级别构造 + R6 态 + 买卖点 bit-vector + 背驰度量 + 区间套。给定 Θ_level/Θ_signal ⟹ R6 态 +
//! BSP 证书唯一。契约锚点从 legacy `Strict/{LevelState,Center,BSP,Trend,Nest,Recursive}.lean`
//! **重锚到 Origin canonical**：`Origin.RecursiveLevelSystem` / `Origin.CenterStates` /
//! `Origin.BspClassification` / `Origin.TrendCompleteClassification` / `Origin.SubLevelDescent`。
//!
//! ## 子模块拓扑（各子模块契约锚 Origin def）
//!
//! - [`center`]：完整中枢判据（方向交替+第三段贯穿）+ 关系/位置三态。对齐
//!   `Origin.CenterComplete.CenterConfirmedComplete` / `Origin.CenterConstruction.centersOf` /
//!   `Origin.CenterStates.{classifyDevelopment,classifyPosition}`。
//! - [`level`]：递归级别走势裁决（`classifyMove` 全链同向）。对齐
//!   `Origin.TrendCompleteClassification.{TrendClass,chooseTrend}` + `Origin.RecursiveLevelSystem`。
//! - [`level_state`]：R6 位置态 + LevelState 三元组。对齐 `Origin.CenterStates.CenterPosition` +
//!   `Origin.RecursiveLevelSystem`。
//! - [`bsp`]：买卖点 bit-vector 判据（三类结构谓词，非互斥）。对齐
//!   `Origin.BspClassification.{BspEndpoint,IsType1,IsType2,IsType3Buy,IsType3Sell}`。
//! - [`divergence`]：背驰 MACD 度量（浮点域隔离 + 同向段面积严格变小）。对齐
//!   `Origin.Divergence.{Force,IsDivergence}` + `Origin.ForceInterface.ForceMeasure`（reference:37）。
//! - [`nest`]：区间套有限递归证书 χ（`Sel_Θ` 选择器 + 终端确认）。对齐
//!   `Origin.SubLevelDescent.{descend,subLevelHasBrokenCenter}`。
//! - [`turn_class`]：小转大显式分类分支（旁挂联合分类 `NestTurnClass` 四类 partition，
//!   纯只读派生）。p118 施工图形态 D；Lean 侧 `Origin.NestTurnClass` 列 formal-chain 遗留
//!   （rust 领先 Origin，T3 L1 先例登记漂移）。
//!
//! ## 递归级别（reference-theta-v0.md:29-30；契约锚 `Origin.RecursiveLevelSystem`）
//!
//! `L0=1分钟线段账本`（parser segments）；`L(k+1)` 只由 `Lk` 已完成走势/Move 构造（对齐
//! `Origin.RecursiveLevelSystem.lift` + `chanRecursiveLevelSystem` 的 `composeStep`：Lk 走势单元 →
//! 连续三段窗口中枢 → 中枢序列裁决走势 → L(k+1) 输入单元）。禁跳级混级。
//! 某层无 ≥`config.level.min_parts_per_level` 完成部件则自然终止；上界 `config.level.l_max`。
//!
//! ## 认识论（formalization-validity-domain）
//!
//! 实装本身 = L1（bit-exact 一致性：Rust 与 Lean spec 输出对齐 = 验证管线正确，不验证
//! Θ 在市场上有效）。各子模块的判定函数是 Lean 纯函数的镜像（L0 给定 Θ 后）；golden/
//! property 测试是 L1（管线正确性，零信息增量）。L2/L3 有效域检验是 Phase 3-4，本模块不声称。
//!
//! ## 铁律（编排者硬指令）
//!
//! 只实装已冻结 Θ v0；遇 spec 漏洞/与 Lean 冲突 → change request，不静默改语义。
//! 不可变：构造新对象，不原地修改。不可交易/退化情况显式处理不静默吞。

use super::config::ThetaConfig;
use super::parser::ParseLayer;
use super::types::{Center, Direction, Segment, Tick};
use divergence::MacdState;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub mod center;
pub mod ref_v1;
pub mod decompose;
pub mod level_state;
pub mod bsp;
pub mod divergence;
pub mod force_conformance;
pub mod descend;
pub mod rmove_compose;
pub mod recursive_tower;
pub mod nest;
/// 账本内核（票 #573 T1）：per-key 注册/首建/append-only 修订/倒退拒绝/终态吸收/钟首写/
/// 增量返回/身份迁移/只读枚举/不变量骨架的对象无关泛型承载体（四组类型参数）。
pub mod ledger_kernel;
/// V3 活假设状态机：NestLifecycleBook sidecar 注册表（三态 + 五钟；#231 重建，spec #232）。
pub mod nest_lifecycle;
/// #92/#93 证书索引：确认事件 → typed 证书（身份主键；构建口径 B + CWindow）。
pub mod nest_index;
/// p118 关④ 小转大显式分类分支：旁挂联合分类 `NestTurnClass`（四类 partition，纯只读派生）。
pub mod turn_class;
pub use turn_class::{
    classify_certificate_turn, classify_nest_turns, is_defer_orphan_event, CertKey, NestTurnClass,
    XzdEvidence,
};
pub mod signal;
/// #110 投影层骨架 + 级别身份标签（SPEC #109 expand 第一票）。默认门关零开销。
pub mod projection;
pub mod six_state;
pub mod voice_eat;
pub mod cand_predicate;
/// C2 走势消费 seam：显式 exact-three 投影、D3 方向绑定与 D2 A/C provider。
/// #630 生产段拆分的 4 个子域（`projection`/`confirm`/`pan`/`pan_provider`）在
/// #630 修复轮改为 `level_view` 内部子模块（目录模块，非 classifier 兄弟文件）——
/// 43 项 `pub(super)` 的可见域随之从「classifier 24 个兄弟模块」收窄到「level_view 子树」
/// （影子评审 #630 MEDIUM-2 指名路径；先例 #633 批7 `incremental/`）。
pub mod level_view;
/// 买卖点身份账本 S1（票 #621，#465 裁定 A 之 T3 首环）：观察适配器 → 三态状态机 →
/// append-only 修订日志（JSONL 外化 + 重放折叠恢复）→ 成立档门户；全部经 [`ledger_kernel`] 表达。
pub mod retrace_ledger;
/// C2 CompletedFreeze 的正式 append-only event-store adapter。
pub mod level_view_store;

/// #345：自持缓冲区增量分类器变体（Nautilus 流式适配，无条件编译——见模块头）。
pub mod streaming;

/// P52 全量增量重放专用的 frontier 只读计数器。
///
/// 默认关闭；只有诊断 bin 显式 [`enable`] 后，分类器在既有 pop/recompose 与 dirty 依赖门处
/// 累加旁路计数。计数不参与任何分类、交易、订单或风控分支。
pub mod cp_replay_diagnostics;
/// 阶段计时插桩（profile-only，#106 真热点定位）：thread_local 累加器，
/// env `THETA_PROFILE_STAGES=1` 时启用。只测时间不改逻辑（bit-exact 安全）。
pub mod stage_profile;
/// ★A3 证书 oracle 探针（仅 test 构建）：记录证书热路径分支命中，供 always-run oracle 断言
/// 「fixture 确实触发了 had_emitted_window pop（T==1/T>1）与两处早停缓存血缘失效」——防止
/// 「always-run 但覆盖为零」的陷阱（codex 审计第6条根修）。release/非 test 构建完全不编译。
#[cfg(test)]
pub mod oracle_probe;

// ── Θ 主体实现的按域分文件（#576）──────────────────────────────────────────
// 下列私有子模块一律 `use super::*` 取本模块的共享导入面（上方 `use` 块 + 下方补充），
// 故这些 `use` 的消费者在子模块内——不是死导入。

/// Θ_level + Θ_signal 分类主管线（级别态 / 分类输出 / 单元⇄线段规约 / 中枢扫描 / 递归构造）。
mod pipeline;
pub use pipeline::{classify, classify_with_tower, Classification, LevelState};

/// P1/P52/P53 Cand^δ 只读驱动器（纯增量只读层，不改 `classify` 行为）。
mod cand_delta;
pub use cand_delta::{
    cand_delta_entry_tower, cand_delta_tower, cand_delta_tower_cached, cp_recall_upper_bound_audit,
};

/// 增量塔缓存与 MACD 增量递推（task #93：per-bar substrate 塔构造 O(n²) → O(n)）——
/// [`TowerCache`] / `LevelCache` / area-memo / closes·MACD 增量序列。见其模块头有效域声明。
mod tower_cache;
pub use tower_cache::TowerCache;

/// 增量塔入口（task #93）：塔构造中枢扫描走增量 resume，bit-exact 等价 [`classify_with_tower`]。
mod incremental;
pub use incremental::classify_with_tower_incremental;

/// 递归组装层第二类（B2/S2）提取与次级别背驰力度（§10.2 买卖点定律一）。
mod sublevel;

use bsp::BspPoint;
use center::UnitRange;
use decompose::{decompose, decompose_resume, MoveBlock};
use pipeline::unit_to_segment;
use recursive_tower::{
    compose_level, compose_level_resume, descend_leveled, index_of_in, map_src_to_close_idx,
    project_to_units, CpScanOwnership, ElementId, LeveledMove, WinMeta, WindowScanCursor,
};
use super::types::Side;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod incremental_profile;
