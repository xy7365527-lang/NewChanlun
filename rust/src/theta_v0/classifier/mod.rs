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

// ★子模块共享 prelude（#648 T2 坐实）：约 30 个子文件以 `use super::*` 消费这些绑定——
// pub(crate) 保 re-export 计为使用（零 unused 警告），crate 外不可见，公共接口面不变。
#[cfg(test)]
pub(crate) use super::config::ThetaConfig;
#[cfg(test)]
pub(crate) use super::types::Segment;
pub(crate) use super::types::{Center, Tick};
pub(crate) use divergence::MacdState;
pub(crate) use std::collections::HashMap;
pub(crate) use std::rc::Rc;

pub mod bsp;
pub mod center;
/// #291（SPEC #274 T1）：中枢生命周期事件机（born/broken/reset，ADR 0001 修正案一·补充二
/// 「中枢=事件」）。只产事件不产动作；wf8 经 opsem 只读旁路外化，默认零行为变化。
pub mod center_lifecycle;
pub mod decompose;
pub mod descend;
pub mod divergence;
// force_conformance 已退役（#991 I-3，G2 #978 裁定二）：样板留档 .chanlun/review-results/force-conformance-retired-sample-20260816.md
pub mod level_state;
pub mod nest;
/// #92/#93 证书索引：确认事件 → typed 证书（身份主键；构建口径 B + CWindow）。
pub mod nest_index;
/// V3 活假设状态机：NestLifecycleBook sidecar 注册表（三态 + 五钟；#231 重建，spec #232）。
pub mod nest_lifecycle;
/// #881 S3（ADR 0011 裁定一/六/七）：操作分解层——横向旁路读法，拿第 k 层元素用不延伸
/// 规则重折产操作序列（含并列盘整）；纯函数，旁路结果不回流主干。
pub mod operation;
/// #543 D1a：重基构造证书 seam（`RebaseTransformTxnV1`，env `OPSEM_DUMP_DIR` 门控的只读观测）。
pub mod rebase_txn;
pub mod recursive_tower;
pub mod ref_v1;
pub mod rmove_compose;
/// p118 关④ 小转大显式分类分支：旁挂联合分类 `NestTurnClass`（四类 partition，纯只读派生）。
pub mod turn_class;
pub use turn_class::{
    classify_certificate_turn, classify_nest_turns, is_defer_orphan_event, CertKey, NestTurnClass,
    XzdEvidence,
};
/// #668（N4）：事件↔BSP 稳定身份桥接对象（身份=（N1 事件键，BSP 结构键 v2）对，双向产出、
/// 端死边死、E2E-O 修订协议）。纯产出零消费接线（p92/π runner 拼缝本票不动，#666 裁定⑥）。
pub mod bsp_bridge;
/// #550：塔内原生背驰段候选事件（对象、append-only 修订流与 C⊆C 谓词）。
pub mod cand_event;
pub mod cand_predicate;
/// #552（N2）：候选事件区间上的跨级 `C⊆C` 包含谓词与相邻级只读扫描探针。零消费接线。
pub mod cand_sub;
/// #641（N3）：级别链证书塔对象（节点=候选事件、边=C⊆C 覆盖关系、E2E-L 三态谱系 + skip edge）。
/// 纯产出零消费接线。
pub mod chain_cert;
/// #981（N7）：Consume_at 签名冻结——新类型族 + 冻结函数签名（逻辑 stub 归后续票）。纯新造，
/// 现役对象零改（`BspStructuralKey` 仅补 `Hash` derive 一行）。
pub mod consume_at;
/// 区间套必要条件——递归塔原生检查器（条款 9，任务 #106；只读，不回写判据 bit）。
pub mod interval_necessity;
/// C2 走势消费 seam：显式 exact-three 投影、D3 方向绑定与 D2 A/C provider。
/// #630 生产段拆分的 4 个子域（`projection`/`confirm`/`pan`/`pan_provider`）在
/// #630 修复轮改为 `level_view` 内部子模块（目录模块，非 classifier 兄弟文件）——
/// 43 项 `pub(super)` 的可见域随之从「classifier 24 个兄弟模块」收窄到「level_view 子树」
/// （影子评审 #630 MEDIUM-2 指名路径；先例 #633 批7 `incremental/`）。
pub mod level_view;
/// C2 CompletedFreeze 的正式 append-only event-store adapter。
pub mod level_view_store;
/// #110 投影层骨架 + 级别身份标签（SPEC #109 expand 第一票）。默认门关零开销。
pub mod projection;
pub mod signal;
pub mod six_state;
pub mod voice_eat;

// ── #614 并线：kimi 线（`kimi-nest-mainline-20260717`）独有的三个新模块 ─────────────
// 说明：kimi 线把本文件的主体拆成 `pipeline`/`cand_delta`/`tower_cache`/`incremental`/
// `sublevel` 等私有子模块；本合并树取 main 线的内联主体（见 #614 合并报告），故那些拆分
// 模块**不**在此声明（同一份代码，声明即重复定义）。下列三个是 kimi 线**新增能力**，
// 无 main 侧对应物，且已有消费方在合并树中：
//   - `ledger_kernel` ← `nest_lifecycle.rs:101` 消费
//   - `streaming`     ← `nautilus/strategy.rs:36` 消费（#345 增量分类路径）
//   - `retrace_ledger`← #624 裁定 A 之下 `first_retrace_replay` 的迁入去处（旧模块已随 kimi
//                        线 `8b8905def2` 删除，其 D7 只读复核原语迁至本模块）
/// 账本内核（票 #573 T1）：per-key 注册/首建/append-only 修订/倒退拒绝/终态吸收/钟首写/
/// 增量返回/身份迁移/只读枚举/不变量骨架的对象无关泛型承载体（四组类型参数）。
pub mod ledger_kernel;
/// 买卖点身份账本 S1（票 #621，#465 裁定 A 之 T3 首环）：观察适配器 → 三态状态机 →
/// append-only 修订日志（JSONL 外化 + 重放折叠恢复）→ 成立档门户；全部经 [`ledger_kernel`] 表达。
pub mod retrace_ledger;
/// #345：自持缓冲区增量分类器变体（Nautilus 流式适配，无条件编译——见模块头）。
pub mod streaming;

/// 诊断驱动器与观测面（#748/C4 纯移动，行为不变）：cp 重放计数器、cand_delta 三驱动器、
/// cp 召回上界审计、阶段计时插桩、oracle 探针——五者均迁至 [`diag`] 子树，此处 `pub use`
/// 保原 `classifier::cp_replay_diagnostics` 等路径全仓零变化。
pub mod diag;
#[cfg(test)]
pub(crate) use diag::cand_delta::cache_series_ok;
pub mod tower_cache;
pub use diag::cp_replay_diagnostics;
#[cfg(test)]
pub use diag::oracle_probe;
pub use diag::stage_profile;
pub use diag::{
    cand_delta_entry_tower, cand_delta_tower, cand_delta_tower_cached, cp_recall_upper_bound_audit,
};
pub use tower_cache::TowerCache;
// 测试子树消费（classifier/tests 经 glob 取用；pub(super) 件不可 pub(crate) 再导出，留私有 use）。
#[cfg(test)]
use tower_cache::{compute_macd_hist_incremental, update_closes_cache};

#[cfg(test)]
pub(crate) use super::types::Side;
pub(crate) use bsp::BspPoint;
pub(crate) use center::UnitRange;
pub(crate) use decompose::decompose;
#[cfg(test)]
pub(crate) use recursive_tower::{compose_level, project_to_units, ElementId};
pub(crate) use recursive_tower::{CpScanOwnership, LeveledMove, WinMeta};

/// 分类管线实现（#648 T2 抽离，纯移动零行为）：`classify*` 入口族 + `LevelState`/`Classification`。
/// 此处 `pub use` 保 `classifier::Classification` 等公共路径全仓零变化。
mod pipeline;

pub use pipeline::*;

#[cfg(test)]
mod tests;
