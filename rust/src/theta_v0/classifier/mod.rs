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
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
/// on2w2-cascade 放行条件3 探针启用开关（env THETA_CASCADE_EPROBE，读一次缓存——热路径零 syscall）。
/// 仅 test 构建（探针本身 #[cfg(test)]），release 完全不编译。
#[cfg(test)]
static CASCADE_EPROBE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

/// on2w2-cascade 全清对照开关（env THETA_CASCADE_FULLCLEAR）：强制 P=0（退回 #65 整塔前缀清空），
/// 作 A/B 计时基线 + bit-exact 全量失效对照面（铁律 H1 神谕先例）。仅 test 构建，release 不编译
/// （默认 off = 增量失效路径）。开启后增量与全清应逐字段相等（bit_exact_per_bar 仍绿即证 P>0 sound）。
#[cfg(test)]
static CASCADE_FULLCLEAR: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

pub mod bsp;
pub mod center;
/// #291（SPEC #274 T1）：中枢生命周期事件机（born/broken/reset，ADR 0001 修正案一·补充二
/// 「中枢=事件」）。只产事件不产动作；wf8 经 opsem 只读旁路外化，默认零行为变化。
pub mod center_lifecycle;
pub mod decompose;
pub mod descend;
pub mod divergence;
pub mod force_conformance;
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
pub mod tower_cache;
use diag::cand_delta::cache_series_ok;
pub use diag::cp_replay_diagnostics;
#[cfg(test)]
pub use diag::oracle_probe;
pub use diag::stage_profile;
pub use diag::{
    cand_delta_entry_tower, cand_delta_tower, cand_delta_tower_cached, cp_recall_upper_bound_audit,
};
pub use tower_cache::TowerCache;
use tower_cache::{compute_macd_hist_incremental, update_closes_cache, AreaCache, LevelCache};

use super::types::Side;
use bsp::BspPoint;
use center::UnitRange;
use decompose::{decompose, decompose_resume, MoveBlock};
use recursive_tower::{
    compose_level, descend_leveled, index_of_in, map_src_to_close_idx, project_to_units,
    CpScanOwnership, ElementId, LeveledMove, WinMeta,
};
use recursive_tower::{compose_level_resume, WindowScanCursor};

/// 单级别分类状态（R6 态 + 走势类型 + 中枢 + 买卖点）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LevelState {
    /// 该级别的走势类型分解 C_ℓ = B₁⊕…⊕B_k（PDF §6，task #143）：maximal 同向趋势块/盘整块
    /// 真序列（替代旧 AllTrend 全链裁决的 ≤1 元素投影）。链尾块 Active = Q8 CurrentMove；
    /// 块携方向（econ_positive 旧注释抱怨的 MoveKind 丢方向由此消解）。
    pub moves: Vec<MoveBlock>,
    /// ★A1（07c/08）：`Rc` 共享——增量塔 `lc.centers` 经 `Rc::clone`（O(1)）投影到 LevelState，
    /// 消除 per-bar per-level 全量 `centers.clone()`（A0 profile 坐实 08 段 1M=1.58s）。前缀不可变，
    /// 尾部经 `Rc::make_mut` 追加（caller 逐 bar drop 上轮 Classification ⟹ strong_count==1 ⟹ 原地
    /// O(tail)；对拍 harness 跨 bar 持有 ⟹ 写时复制退化全拷，仍 bit-exact，生产路径不受影响）。
    pub centers: Rc<Vec<Center>>,
    /// 与 `centers` 1:1 的 `B_p/c_p` 生命周期对象；保存首个 non-extension 离开单元，并随每个
    /// 新同级递归单元从 Pending 单调推进到 Closed。事件确认快照不在这里回填。
    pub cp_ownership: Rc<Vec<CpScanOwnership>>,
    /// 各买卖点条目（非互斥 bit-vector + 结构止损价 single source，BSP.lean + reference:46）。
    ///
    /// 路 B（Lead 接口契约裁定）：每个 `BspPoint` 携带 pivot_low/pivot_high/center——
    /// strategy 直接构造 `StopInput`，**不**从 bars 重算结构 pivot。classifier 是结构
    /// 止损价的唯一来源（识别买卖点时已定位 pivot，避免两处结构逻辑漂移）。
    /// ★A1（07c）：`Rc` 共享——memo 命中经 `Rc::clone`（O(1)）投影到 LevelState，消除 per-bar
    /// per-level 全量 `cached_bsp.clone()`（A0 profile 坐实 07c 段 1M=1.80s）+ miss 路径 `b.clone()`。
    pub bsp: Rc<Vec<BspPoint>>,
    /// ★Q4（task #145）：该级盘整背驰证书（`signal::PanDivCert`，与 bsp 同一 extract 调用产出、
    /// 同 memo 键缓存）。**不是买卖点**（零 six-bit，不冒充 B1/S1）——承接路由在 econ 统计层
    /// （collect_signals 走 Nest/XZD 二通道，两门皆闭诚实丢弃）。
    pub pan_div: Rc<Vec<signal::PanDivCert>>,
    /// ★#885 S4-d：该级一类点 T3-in-c 固定首对分级记录（`signal::FirstClassGradeRecord`，与
    /// bsp 同一 extract 调用产出、同 memo 键缓存、同 frontier 冻结边界锚）。按 `diverged` 捕获
    /// （T3-in-c 二次门控**之前**，Present 与 Missing 两域均记录）——否则域
    /// （`T3InCGrade::Missing`）记录此前只落 thread_local 诊断 sidecar `GRADE_SIDECAR`，不进
    /// `Classification`、按坐标查不到，任何涉及类一类点的命中率因此只是下界；本字段是否则域
    /// （027 课「类第一类」归化域，ADR 0001 补充十六）的**生产可查载体**。
    ///
    /// **只建载体/可查性，不新定判据**——「算不算类一类点、几段起算」的教义面归 #817（未裁），
    /// 本字段不参与任何买卖点 bit、门控、生命周期或订单流（纯观测记录，同 `pan_div` 的诚实
    /// 缺省纪律）。`level` 字段由装配方按真实级别盖章（全量入口先落占位 0，见
    /// [`signal::FirstClassGradeRecord`] 文档）。
    pub first_class_grades: Rc<Vec<signal::FirstClassGradeRecord>>,
    /// #110 投影层（SPEC #109 expand 第一票）。门关（默认）= `None`（零开销，bit-exact 不变）；
    /// 门开 = stamping 路径构造 [`projection::LevelProjectionLayer`]（`bsp` 同 `Rc` O(1) 共享 +
    /// 单趟跨度扫描 + T2 (#171) 单趟三元锚索引构建，T1 供给线同源）。只描述不判定
    /// （禁第二查法）——零判定消费。
    pub level_projection: Option<projection::LevelProjectionLayer>,
}

impl LevelState {
    /// ★#885 S4-d：按 source_index 查一类点 T3-in-c 分级记录（Present 与 Missing 两域）。
    /// 同一级内一类候选 source_index 唯一 ⟹ 至多一条命中。
    pub fn first_class_grade_at(
        &self,
        source_index: usize,
    ) -> Option<&signal::FirstClassGradeRecord> {
        self.first_class_grades
            .iter()
            .find(|r| r.source_index == source_index)
    }

    /// ★#885 S4-d：否则域（`T3InCGrade::Missing`）记录迭代器——027 课「类第一类」归化域的
    /// 可查面（准入判据归 #817，本层只建可查性）。
    pub fn otherwise_domain_records(
        &self,
    ) -> impl Iterator<Item = &signal::FirstClassGradeRecord> + '_ {
        self.first_class_grades
            .iter()
            .filter(|r| matches!(r.grade, signal::T3InCGrade::Missing(_)))
    }
}

/// 多级别递归分类输出（L0..Lmax；某层自然终止则该层及以上为空）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Classification {
    /// 索引 = 级别 ℓ（0=L0=1分钟线段账本）。
    pub levels: Vec<LevelState>,
}

impl Classification {
    /// ★#885 S4-d：按 (level, source_index) 坐标查否则域（`T3InCGrade::Missing`）分级记录——
    /// 验收测试锁的查询入口。越界 level / 无该坐标 / 该坐标是 Present ⟹ None。
    pub fn otherwise_domain_at(
        &self,
        level: usize,
        source_index: usize,
    ) -> Option<&signal::FirstClassGradeRecord> {
        self.levels
            .get(level)?
            .otherwise_domain_records()
            .find(|r| r.source_index == source_index)
    }
}

/// 把 L0 线段规约为携带方向的走势单元（契约锚 `Origin.ChanlunElements.Segment` + `CenterConstruction.segHigh/segLow`）。
///
/// L0 单元 = parser 线段（**含方向**，完整判据 `DirAlternates` 的输入）；`[lo,hi]` 对齐
/// `Origin.CenterConstruction.segHigh/segLow`（向上段 hi=端价/向下段 lo=端价已规约为区间）。
pub(crate) fn segment_to_unit(seg: &Segment) -> UnitRange {
    let (lo, hi) = if seg.start_price <= seg.end_price {
        (seg.start_price, seg.end_price)
    } else {
        (seg.end_price, seg.start_price)
    };
    UnitRange {
        start_index: seg.start_index,
        end_index: seg.end_index,
        direction: seg.direction,
        lo,
        hi,
    }
}

/// 级别-N 输入单元 → `Segment`（`segment_to_unit` 的逆，端点价按 `fold_direction` 取 hi/lo）。
///
/// codex-decide-20260703 裁定 A 的最大实现风险点（end_price 忠实性，须单测）：级别-N 的「线段」
/// 角色由输入单元 [`UnitRange`] 承担——向上单元起点=lo/终点=hi（`seg_end` 取 end_price=hi）；
/// 向下单元起点=hi/终点=lo。与 [`segment_to_unit`] 互逆：L0 段经 `segment_to_unit` → `unit_to_segment`
/// round-trip bit-exact（向上段 start<end ⟹ lo=start/hi=end ⟹ 还原 (start,end)；向下段镜像）。
/// `start_index/end_index` 是 source_index（原始 K 序）——A/C 段 MACD 面积经 close_src 映射用同坐标系。
pub(crate) fn unit_to_segment(u: &UnitRange) -> Segment {
    let (start_price, end_price) = match u.direction {
        Direction::Up => (u.lo, u.hi),
        Direction::Down => (u.hi, u.lo),
    };
    Segment {
        direction: u.direction,
        start_index: u.start_index,
        end_index: u.end_index,
        start_price,
        end_price,
    }
}

/// #552（N2）：候选事件扫描的输入投影——L0 借用 parser 线段，L≥1 由 units 还原 `Segment`；
/// 方向锚一律取单元结构方向（与 [`extract_first_third_for_level`] 的 `structural_anchors` 同口径）。
fn candidate_scan_inputs<'a>(
    is_l0: bool,
    l0_segments: &'a [Segment],
    units: &[UnitRange],
) -> (Cow<'a, [Segment]>, Vec<Option<Direction>>) {
    let segments = if is_l0 {
        Cow::Borrowed(l0_segments)
    } else {
        Cow::Owned(units.iter().map(unit_to_segment).collect())
    };
    let anchors = segments
        .iter()
        .map(|segment| Some(segment.direction))
        .collect();
    (segments, anchors)
}

/// ★#487：为接线层给出的**精确挂起中枢**复核“旧框右边 → 紧邻 leave/retest”三类证书。
///
/// 本函数只消费结构事实，不读取 strategy/账户状态；挂起候选集合由接线层提供。配对规则：
///
/// 1. `center.end_index` 是旧框冻结的右边；从本级输入塔中第一条
///    `start_index >= end_index` 的走势起。这里的等值是仓内共享枢轴坐标约定：前段终点就是
///    后段起点，故 `start_index == center.end_index` 正是“右边之后第一条”，不是框内走势；
///    同口径见 `signal.rs::nearest_confirmed_center` 的 `end_index <= seg_start`、
///    `signal.rs::trend_third_class_in_c` 的 `partition_point(start_index < boundary)`，以及
///    `divergence.rs` A 区间声明的 `start_index >= prev_center.end_index`；
/// 2. 上述第一条就是 leave，紧随的下一条就是 retest；两者必须方向互反。下一条缺失或
///    方向不互反即返回 `None`，不得越过同向续行再向后寻找替代配对；
/// 3. 两条走势按本级生产投影还原为 `Segment`，价格只交给
///    [`signal::judge_third_cert`] 相对该旧框的 `ZG/ZD` 判定；价格失败同样立即返回 `None`。
///
/// 因此同向续行后的配对、同一价格带的晚框穿越、或首个 leave/retest 已失败后出现的远期
/// 相邻对，均不能回填旧框。
/// 九段升级同核心不设分支：接线层给出首次绑定时冻结的完整框，天然按该框四边判定。
pub(crate) fn historical_bound_third_cert(
    classification: &Classification,
    tower: &[Rc<Vec<LeveledMove>>],
    level: usize,
    center: &Center,
) -> Option<signal::ThirdClassCert> {
    let level_moves = tower.get(level)?;
    let leave_idx = level_moves.partition_point(|m| m.start_index < center.end_index);
    let leave = historical_bound_segment(classification, level_moves, level, leave_idx)?;
    let retest = historical_bound_segment(classification, level_moves, level, leave_idx + 1)?;
    debug_assert!(
        leave.start_index >= center.end_index,
        "historical-bound leave 必须位于旧框右边之后"
    );
    if retest.direction != leave.direction.flip() {
        return None;
    }
    signal::judge_third_cert(center, &leave, Some(leave.direction), &retest)
}

/// 把 `tower[level][idx]` 按该级生产投影的同一方向/外缘口径还原成 `Segment`。
fn historical_bound_segment(
    classification: &Classification,
    level_moves: &[LeveledMove],
    level: usize,
    idx: usize,
) -> Option<Segment> {
    let m = level_moves.get(idx)?;
    let prev = idx.checked_sub(1).and_then(|i| level_moves.get(i));
    let direction = if level == 0 {
        match &m.rmove {
            descend::RMove::Segment { direction, .. } => *direction,
            // L0 正常恒为 Segment；保守回退只维持结构方向，不另造判据。
            descend::RMove::Compose { .. } => m.fold_direction(prev),
        }
    } else {
        let lower_blocks = &classification.levels.get(level - 1)?.moves;
        decompose::center_own_dir_at(lower_blocks, idx).unwrap_or_else(|| m.fold_direction(prev))
    };
    let (lo, hi) = m.envelope();
    Some(unit_to_segment(&UnitRange {
        start_index: m.start_index,
        end_index: m.end_index,
        direction,
        lo,
        hi,
    }))
}

/// 级别-N 一/三类买卖点提取（codex-decide-20260703 裁定 A：级别-N 直接判定，非 L0 relabel）。
///
/// 把级别-N 输入单元 `units`（承担「线段」角色）还原为 `Segment` 后**复用 L0 的
/// [`signal::extract_signals_with_hist`]**——同一套逻辑，只换输入算子（is_l0 分支消失于领域层）：
/// - **趋势门控**：内部 `decompose(centers)` 局部趋势门（Q1/Q8，task #143）与本级 [`classify_level`]
///   同一 `decompose` 单一来源 ⟹ 「一类只在该级当前趋势块内产」忠实。
/// - **A/C 段力度**：内部 `AbcDivergence`/`locate_trend_seg_a` 复用 divergence.rs 面积原语（与
///   `sublevel_diverges` 同族的 `segment_macd_area`/`is_divergence`）——**禁第二套力度引擎**满足。
/// - **三类**：`judge_third` 在级别-N units（外缘区间）+ centers（几何中枢）的离开/回试关系上判定。
///
/// ★force_state 生产热路由（beta-route #115）：传真 `dif/closes_tick`（与 L0 层同源，L0 唯一可达
/// close 序列，级别-N A/C 段经 source_index 坐标映射同坐标系）⟹ 级别-N 一类趋势背驰候选的
/// `point.force` 亦算得 `Some`（5 proxy），进 selector force_state 第 8 维。二/三类 force=None。
fn extract_first_third_for_level(
    centers: &[Center],
    units: &[UnitRange],
    provenance_anchors: &[Option<Direction>],
    hist: &[f64],
    dif: &[f64],
    closes_tick: &[Tick],
    close_src: &[usize],
    gauge: divergence::DivergenceGauge,
    // ★#885 S4-d：分级记录生产 sink（原样透传 `_anchored`；记录 `level` 为占位 0，由
    // `classify_impl` 按 level_idx 盖章）。
    grade_sink: &mut Vec<signal::FirstClassGradeRecord>,
) -> (Vec<BspPoint>, Vec<signal::PanDivCert>) {
    let segs: Vec<Segment> = units.iter().map(unit_to_segment).collect();
    // ★#486（#485 范围 1）：L≥1 一/三类均以单元结构方向为方向锚；provenance 只保留平行数组
    // 长度契约，不再决定三类 leave 资格。`judge_third_cert` 的 Some(Direction)、方向匹配、
    // 回试方向、严格 >ZG/<ZD 与 OwnerRef 契约不变。
    let structural_anchors: Vec<Option<Direction>> =
        units.iter().map(|u| Some(u.direction)).collect();
    debug_assert_eq!(
        provenance_anchors.len(),
        structural_anchors.len(),
        "provenance anchors 与 L≥1 units 必等长"
    );
    signal::extract_signals_with_hist_anchored(
        centers,
        &segs,
        Some(&structural_anchors),
        hist,
        dif,
        closes_tick,
        close_src,
        gauge,
        grade_sink,
    )
}

/// 从 L0 线段单元序列识别 canonical 中枢序列（**完整判据** seed + 延伸吸收，契约锚
/// `Origin.CenterComplete.CenterConfirmedComplete` + 第20课中心定理一）。
///
/// L0 线段有内在方向 ⟹ seed 用 `center::center_from_segments`（完整判据：方向交替 ∧ 全三段核心
/// 非空，口径 B——第三段贯穿已被全三段核心非空吸收，637号 codex L0 等价）。seed 成立后进入延伸
/// 吸收（中心定理一：后续段区间触及 [ZD,ZG] ⟹ 同一中枢延伸，task #142），仅 non-extension
/// （`d_j>ZG ∨ g_j<ZD`）终止；seed 不成立则前进一段继续找。算法单一来源 = `recursive_tower::
/// detect_centers_windowed_resume`（全量/增量同一扫描，bit-exact 定义性）。
///
/// ⚠边界声明作废：本函数契约锚 `Origin.CenterComplete.CenterConfirmedComplete` 在 Lean 侧核心
/// 非空支仍是 `ZD≤ZG`（弱，单点核心成立）；`center_from_segments` 本身已于 #321（2026-07-26
/// 用户裁决）从严为 `ZD<ZG`（单点不成立）——**该落点（成立谓词端点分支）的 Lean↔Rust 对齐声明
/// 在本边界上作废，此边界不作机械锁用**（不得据本实装断言 Lean 侧行为，亦不得据 Lean 契约锚反推
/// 本实装期望值）；其余落点对齐声明不受影响。Lean 侧跟进留对方线。详见 `center.rs`
/// `center_from_segments` doc 同一登记。
fn detect_centers_complete(units: &[UnitRange]) -> Vec<Center> {
    detect_centers_with(units, center::center_from_segments)
}

/// 从上级走势单元序列识别 canonical 中枢序列（**几何路径** seed + 延伸吸收，契约锚
/// `Origin.centerHolds` + 三段共同重叠 + 第20课中心定理一）。
///
/// 上级单元是中枢外缘区间（**无内在缠论方向**，方向由 Move 趋势裁决携带）⟹ seed 用
/// `center::center_from_window`（几何判据：全三段核心非空，口径 B——第三段贯穿已吸收，637号；无方向交替）。
/// 延伸吸收与 L0 同一几何判据（区间触及 [ZD,ZG]，task #142）。上级发展裁决用
/// `Origin.CenterStates.classifyDevelopment`（外缘判据，无方向交替要求）——见 `center.rs`
/// 诚实有效域声明。
///
/// ⚠边界声明作废：本函数契约锚 `Origin.centerHolds`（`Origin.CenterConstruction.centerHolds`）
/// 在 Lean 侧是 `ZD≤ZG`（弱，单点核心成立）；`center_from_window` 本身已于 #321（2026-07-26
/// 用户裁决）从严为 `ZD<ZG`（单点不成立）——**该落点（成立谓词端点分支）的 Lean↔Rust 对齐声明
/// 在本边界上作废，此边界不作机械锁用**（不得据本实装断言 Lean 侧行为，亦不得据 Lean 契约锚反推
/// 本实装期望值）；其余落点对齐声明不受影响。Lean 侧跟进留对方线。详见 `center.rs`
/// `center_from_window` doc 同一登记。
fn detect_centers_geometric(units: &[UnitRange]) -> Vec<Center> {
    detect_centers_with(units, center::center_from_window)
}

/// canonical 中枢扫描（seed + 延伸吸收 + non-extension 终止）——**单一来源委托**
/// `recursive_tower::detect_centers_windowed_resume`（全量 = `start_i=0`；增量塔走同一函数的
/// resume 路径 ⟹ T^inc == T^full 定义性成立，非对拍性成立）。
fn detect_centers_with(
    units: &[UnitRange],
    build: fn(&UnitRange, &UnitRange, &UnitRange) -> Option<Center>,
) -> Vec<Center> {
    recursive_tower::detect_centers_windowed_resume(units, build, 0)
        .0
        .into_iter()
        .map(|(c, _)| c)
        .collect()
}

/// 把一级走势单元序列规约为该级走势类型分解 + 中枢（PDF §6，task #143，替代旧 AllTrend
/// `classifyMove` 全链裁决——吸收锁死谓词，见 decompose.rs 模块头）。
///
/// `is_l0`：L0 用完整判据（方向交替），上级用几何路径（外缘）。返回 `(中枢序列, 分解块序列)`。
fn classify_level(units: &[UnitRange], is_l0: bool) -> (Vec<Center>, Vec<MoveBlock>) {
    let centers = if is_l0 {
        detect_centers_complete(units)
    } else {
        detect_centers_geometric(units)
    };
    let blocks = decompose(&centers);
    (centers, blocks)
}

/// Θ_level + Θ_signal 分类内部实现（`classify` / `classify_with_tower` 共享单一来源）。
///
/// `tower_snapshots[i]` = 处理第 i 级时 `compose_level` 前的 `moves_tower` 快照，下标与
/// `Classification.levels` 同构（`tower_snapshots.len() == levels.len()`）：
/// - 索引 0（L0 级）：全 `RMove::Segment`（递归底，`sub_moves` 空）。
/// - 索引 ≥1（L(k) 级）：前一级产出的 `RMove::Compose` 序列（携次级别 subs，depth≥1 真嵌套）。
fn classify_impl(
    l0: &ParseLayer,
    config: &ThetaConfig,
) -> (
    Classification,
    Vec<Rc<Vec<LeveledMove>>>,
    cand_event::CandidateStreams,
) {
    let min_parts = config.level.min_parts_per_level as usize;
    let l_max = config.level.l_max as usize;

    // L0 输入单元 = parser 线段账本（reference:29 L0=1分钟线段账本）。
    let mut units: Vec<UnitRange> = l0.segments.iter().map(segment_to_unit).collect();
    // Q7-#1 裁定C：units 的方向锚资格（与 units 同步循环携带；L0 分支不消费，级别-N 在投影点派生）。
    let mut units_anchors: Vec<Option<Direction>> = Vec::new();

    // 空 L0：无可构造级别（自然终止于 L0 之前）。
    if units.is_empty() {
        return (
            Classification::default(),
            Vec::new(),
            std::rc::Rc::new(Vec::new()),
        );
    }

    // ★递归塔对象（#53 升级，still-MISSING-塔解除）：L0 走势单元 = 携坐标的 `RMove::Segment`
    // （`LeveledMove`，递归底 level 0）。旧塔把每级走势单元折叠为无 subs 的 `UnitRange`，
    // `extract_second_signals`（消费 `RMove::Compose` 的 descend 取回次级别走势）永产不出 B2/S2。
    // 新塔每级走势单元携次级别走势 subs（`RMove::Compose`）+ source_index 坐标 ⟹ B2/S2 真可产。
    // ★O(n) 重构：moves_tower/snapshots 用 Rc（与增量版同返回类型 `Vec<Rc<Vec<LeveledMove>>>`，
    // bit-exact 测试逐字段比较 *rc）。全量版非 per-bar 热点（O(n) 单趟），Rc 仅为类型对齐。
    let mut moves_tower: Rc<Vec<LeveledMove>> = Rc::new(
        units
            .iter()
            .enumerate()
            .map(|(i, u)| {
                LeveledMove::from_unit(
                    u,
                    ElementId {
                        level: 0,
                        ordinal: i as u64,
                    },
                )
            })
            .collect(),
    );

    // 第一类背驰 MACD：closes/close_src 在全递归层共享（L0 唯一可达 close 序列；上级走势的
    // 次级别 close 区间由 source_index 坐标定位，见 macd 接入点）。
    let closes: Vec<f64> = l0.merged_bars.iter().map(|b| b.close as f64).collect();
    let close_src: Vec<usize> = l0.merged_bars.iter().map(|b| b.source_index).collect();
    // ★force_state 生产热路由（beta-route #115）：dif（黄白线）+ closes_tick（整数 close）供一类候选
    // A/C 段 5 proxy（DIF 峰/振幅/速度）。hist/dif 同一 compute_macd 单趟产出（无额外 O(n) 扫描）。
    let series = divergence::compute_macd(&closes, &config.macd);
    let hist = series.hist;
    let dif = series.dif;
    let closes_tick: Vec<Tick> = l0.merged_bars.iter().map(|b| b.close).collect();

    let mut levels: Vec<LevelState> = Vec::new();
    let mut tower_snapshots: Vec<Rc<Vec<LeveledMove>>> = Vec::new();
    let mut candidate_observations = Vec::new();

    // 递归级别构造：每级由下级走势单元构造（L0 直接是线段单元，从 L0 开始裁决）。
    for level_idx in 0..=l_max {
        // 自然终止（reference:30）：某层无 ≥min_parts 完成部件 ⟹ 无法产生完整走势，停止。
        if units.len() < min_parts {
            break;
        }

        // 本级输入塔快照（compose_level 前，与 levels[level_idx] 对应——同步 index 不变量）。
        tower_snapshots.push(Rc::clone(&moves_tower));

        // L0（level_idx==0）用完整判据（方向交替，线段有方向）；上级用几何路径（外缘，单元无方向）。
        let is_l0 = level_idx == 0;
        let (centers, moves) = classify_level(&units, is_l0);

        // L(k+1) 走势塔 = 本级窗口化 compose（每中枢的构成三段次级别走势 → 一个上级 `RMove::Compose`，
        // 契约锚 `Origin.RecursiveLevelSystem.composeStep` 窗口封装）。`upper_moves` 是携坐标的上级走势
        // 序列（descend 取回构成它的次级别走势 ⟹ B2/S2 可产），与 `centers` 一一对应。
        let (centers_w, upper_moves, mut cp_ownership) =
            compose_level(&units, &moves_tower[..], is_l0, level_idx as u32 + 1);
        debug_assert_eq!(
            centers, centers_w,
            "compose_level 与 classify_level 中枢序列一致"
        );
        recursive_tower::advance_cp_lifecycles(
            &mut cp_ownership,
            &centers_w,
            &units,
            &moves_tower,
            (!is_l0).then_some(&units_anchors[..]),
            1,
        );

        // BSP 信号提取（reference:34-36）。三层覆盖：
        // - **L0 线段层**（`extract_signals`）：第一类（破中枢几何 L0 ∧ MACD 背驰 L1 真算）+ 第三类
        //   （confirmed 结构几何）。L0 走势单元 = 线段（有方向），第一/三类在线段端点上 bit-exact 判定。
        // - **递归组装层**（`extract_second_signals`，#53 接入）：第二类（B2/S2）由次级别第一类构成
        //   （买卖点定律一 §10.2）。对本级**每个上级走势** `RMove::Compose`，从 descend 取回的次级别
        //   走势序列内识别第二类走势结构（第一类离开 + 回拉不创新低/新高），产 B2/S2。背驰力度由
        //   `divergence_of` 闭包用 `divergence.rs` MACD 真算（次级别走势 close 区间 → 面积比较）。
        // ★#885 S4-d：一类点 T3-in-c 分级记录 sink——与 bsp/pan_div 同一 extract 调用产出；
        // 记录 `level` 先为占位 0（全量入口 level=None），下方按 level_idx 盖章（真实级别）。
        let mut first_class_grades: Vec<signal::FirstClassGradeRecord> = Vec::new();
        let (mut bsp, pan_div): (Vec<BspPoint>, Vec<signal::PanDivCert>) = if is_l0 {
            // ★force_state 生产热路由（beta-route #115）：传真 dif/closes_tick ⟹ 一类候选 point.force
            // = Some（5 proxy），进 selector force_state 第 8 维。结构六 bit 不变（force 不进 class_index/
            // 分桶 key，PartialEq 排除），GOLDEN 因 Debug 含 force 诚实翻转（signal.rs digest guard）。
            signal::extract_signals_with_hist(
                &centers,
                &l0.segments,
                &hist,
                &dif,
                &closes_tick,
                &close_src,
                config.divergence_gauge,
                &mut first_class_grades,
            )
        } else {
            // 级别-N 一/三类（codex-decide-20260703 裁定 A）：units 承担线段角色，复用 L0 判据（含 force）。
            extract_first_third_for_level(
                &centers,
                &units,
                &units_anchors,
                &hist,
                &dif,
                &closes_tick,
                &close_src,
                config.divergence_gauge,
                &mut first_class_grades,
            )
        };
        for g in &mut first_class_grades {
            g.level = level_idx as u32; // #885：占位 0 → 真实级别盖章（LevelState 下标即级别）。
        }
        // 递归组装层 B2/S2（#53 接入）：对每个上级走势的次级别走势序列识别第二类结构。
        bsp.extend(extract_second_for_level(&upper_moves, &hist, &close_src));
        bsp.sort_by_key(|p| p.source_index);

        let (candidate_segments, candidate_anchors) =
            candidate_scan_inputs(is_l0, &l0.segments, &units);
        candidate_observations.extend(cand_event::observations_for_level(
            level_idx as u32,
            &centers_w,
            &moves,
            candidate_segments.as_ref(),
            &candidate_anchors,
            &close_src,
            &pan_div,
        ));

        let bsp = Rc::new(bsp);
        // #110 投影层 stamping（机制位关 = None 零开销）。T3 (#172) 并门：本机制位转派生——
        // 层载由链路径是否启用单一驱动（π 入口 `admission::chain_driven_level_projection`
        // 唯一生产写入点，#168 裁定 3）；链活 ⟹ 层必载（含三元锚索引），链死不载。
        // T2 (#171)：三元锚供给（`l0.fractals`/`l0.merged_bars`，ParseLayer `Rc` 共享只读
        // 借用，零拷贝）随门开分支引入——门关分支零新增读。
        let level_projection = if config.level_projection.enabled {
            Some(projection::LevelProjectionLayer::from_level(
                levels.len() as u32,
                &bsp,
                &l0.fractals,
                &l0.merged_bars,
            ))
        } else {
            None
        };
        levels.push(LevelState {
            moves,
            centers: Rc::new(centers.clone()),
            cp_ownership: Rc::new(cp_ownership),
            bsp,
            pan_div: Rc::new(pan_div),
            first_class_grades: Rc::new(first_class_grades), // #885 S4-d
            level_projection,
        });

        // L(k+1) 输入单元 = 上级走势塔的 `UnitRange` 投影（外缘区间 + 坐标 + 外缘趋势方向）。
        // 上级走势携 subs（`RMove::Compose`），投影只为下一级几何中枢检测提供 [lo,hi] 区间——
        // 真递归 subs 在 `moves_tower` 里保留（不丢弃，旧塔丢弃 subs 是 B2 不可产的根因）。
        // Q7（task #145）：方向源 = 本级中枢 ownership 块方向（刚 push 的 LevelState.moves 单一来源）。
        {
            let pb = &levels.last().expect("本级 LevelState 已 push").moves;
            units = project_to_units(&upper_moves, pb);
            // Q7-#1 裁定C：锚资格与投影方向同一 provenance 来源（center_own_dir_at，None=fallback）。
            units_anchors = (0..units.len())
                .map(|i| decompose::center_own_dir_at(pb, i))
                .collect();
        }
        moves_tower = Rc::new(upper_moves);

        // 本级无中枢 ⟹ 无上级输入单元，停止递归（自然终止）。
        if units.is_empty() {
            break;
        }
    }

    let mut candidate_book = cand_event::CandidateEventBook::default();
    let as_of = l0.merged_bars.last().map_or(0, |bar| bar.source_index);
    candidate_book.advance(&candidate_observations, as_of);
    (
        Classification { levels },
        tower_snapshots,
        candidate_book.streams(),
    )
}

/// Θ_level + Θ_signal 顶层入口（reference-theta-v0.md:27-37）。
///
/// 递归构造 L0..Lmax：L0=parser 线段账本；每级由下级已完成走势单元构造中枢 + 裁决走势，
/// 走势成为上级输入单元。自然终止：某级单元数 < `min_parts_per_level`（无法产生完整走势），
/// 或达 `l_max` 上界。
///
/// ★边界条件：
/// - L0 线段数 < `min_parts_per_level` ⟹ `levels` 仅含 L0（或为空，见下）—— 自然终止。
/// - 任一级走势分解（PDF §6）产出完整块序列进 moves（混合链不再是级别整体退化裁决，
///   task #143——旧 AllTrend 的 HigherCenterCandidate 由多块序列吸收）。
/// - 空 ParseLayer（无线段）⟹ `Classification::default()`（空 levels，无可构造级别）。
pub fn classify(l0: &ParseLayer, config: &ThetaConfig) -> Classification {
    classify_impl(l0, config).0
}

/// 分类 + 逐级塔导出入口（(i) 段导出桥，MEMORY coverage-engine-needs-tower-export-bridge）。
///
/// 返回 `(Classification, Vec<Vec<LeveledMove>>)`：
/// - `Classification`：与 `classify` bit-identical（共用 `classify_impl` 单一来源，原行为不变）。
/// - `Vec<Vec<LeveledMove>>`：逐级走势塔快照（`tower[i]` 对应第 i 级处理的输入塔）：
///   - `tower[0]`：L0 线段层（全 `RMove::Segment`，递归底，`sub_moves` 空）。
///   - `tower[k]`（k≥1）：第 k 级输入塔，含 `RMove::Compose` 携次级别 subs（depth≥1 真嵌套），
///     下游 `descend_leveled` 可遍历次级别走势。
///
/// ★不碰附着映射：本函数只导出塔，不消费 coverage/interp 的附着规则（(ii) 段职责）。
/// ★L0/L1 认识论等级：纯结构导出操作，不依赖经验数据（formalization-validity-domain 231号）。
pub fn classify_with_tower(
    l0: &ParseLayer,
    config: &ThetaConfig,
) -> (Classification, Vec<Rc<Vec<LeveledMove>>>) {
    let (classification, tower, _) = classify_impl(l0, config);
    (classification, tower)
}

/// 分类 + 逐级塔 + #550 原生候选事件流。
///
/// 旧 [`classify_with_tower`] 保留二元返回以维持既有消费者逐字节不动；需要候选事件的调用方
/// 使用本纯增量通道。事件只产出，不参与 BSP、门、admission 或订单流。
///
/// 本入口每次以 fresh book 计算，因此事件流是“每个 key 一条终态”的**终态窗口投影**；
/// 跨 bar 因果簿的唯一正本由 [`classify_with_tower_events_incremental`] 的 `TowerCache` 持有。
///
/// **#551 裁定(i) 等价锁口径（编排者 2026-07-28 裁决：出路甲）**——原文「fresh-full 与因果簿
/// 终态投影每 key 最新 revision 逐字段相等」经裁决**收窄为非终态域**：
/// 1. **非终态域逐字段相等，无条件**：因果簿中状态为 `Provisional`/`Unresolved` 的每个 key，
///    fresh-full 侧必存在同 key 且业务载荷逐字段相等。无任何豁免。
/// 2. **终态域分叉只计量、不禁止**：`Confirmed`/`Invalidated` 的 key 上，两侧的载荷差异与 key
///    集合差异（含塌空—回长的复活型分叉）是 E2E-O 终态语义的**定义后果**，不是违规。成因：
///    终态候选的载荷在终态时刻**冻结**，而其上游证书（如 `PanDivCert.seg_c`）仍随 bar 推进而变，
///    fresh-full 无状态、只按当前结构重判 —— 历史相关语义与历史无关语义在终态域必然分叉。
///
/// 该口径修订的完整论证（命题、两个可观测面、三条出路对照与裁决）见
/// `chanlun/review-results/issue551-t2-impl-20260728.md` §五。机器载体 = 本模块 test 段的
/// `causal_book_terminal_projection_equals_fresh_full_stream` 与
/// `invalidation_fork_only_points_from_causal_book_to_absent_fresh_stream` 两枚定稿锁。
pub fn classify_with_tower_events(
    l0: &ParseLayer,
    config: &ThetaConfig,
) -> (
    Classification,
    Vec<Rc<Vec<LeveledMove>>>,
    cand_event::CandidateStreams,
) {
    classify_impl(l0, config)
}

/// ★增量塔入口：返回 `(Classification, tower_snapshots)` bit-exact 等价于
/// `classify_with_tower(l0, config)`，但塔构造的中枢扫描走增量 resume（前级 confirmed 前缀缓存，
/// 仅尾部续扫），解 per-bar substrate 的塔构造 O(n²) 根因。
///
/// ## bit-exact 保证（#93 铁律）
///
/// 输出 `(Classification, Vec<Vec<LeveledMove>>)` 与 `classify_with_tower(l0, config)` 逐字段
/// bit-identical：
/// - `Classification.levels[k].centers`：增量累积的中枢序列 == 全量 `detect_centers`（resume bit-exact，
///   见 recursive_tower.rs 证明）。
/// - `Classification.levels[k].moves`：从累积 centers 经 `decompose_resume` 续折 == 全量分解（resume 单一来源）。
/// - `Classification.levels[k].bsp`：从累积 centers + segments/hist 经同口径提取 == 全量提取。
/// - `tower_snapshots[k]`：本级 compose 前的 `moves_tower`，前缀来自缓存 + 尾部续扫 == 全量 compose。
///
/// ## 硬契约（#106 证书路径，codex 双轮审计锚定）
///
/// `cache: &mut TowerCache` 必须与 `l0` **同源逐 bar 推进**——即同一 `ParseLayerIncr` 血缘、同一
/// `ThetaConfig`、每 bar 调用一次（无跳 bar、无跨数据流复用、config 不变）。`merged_confirmed_len`/
/// `segments_confirmed_len` 证书只保证「本 parser 自己的前缀稳定」，不验证 cache 内旧前缀与本轮 l0 同源。
/// 违反（同 cache 跑两条流不 clear / 中途换 config）⟹ MACD/L0 tower/closes/frontier 复用陈旧前缀
/// （bit-exact 破裂）。生产路径满足：`IncrementalClassifier::new` 每实例新建 TowerCache + 同源逐 bar。
/// 换流/换 config 的调用方须先 `cache.clear()`。
/// ponytail: 不加运行时 lineage epoch——生产 IncrementalClassifier 结构保证同源，epoch 是为不存在的
/// 滥用场景加防御（YAGNI）；契约由本文档 + clear() 入口声明。
/// ## 增量有效性（exp≈1 前提）
///
/// 段账本单调追加（前缀稳定）时，每级扫描从 `consumed` 续扫 O(tail) 而非 O(units) ⟹ 塔构造总扫描
/// O(Σ tail) = O(n)（amortized）。段账本前缀回缩时自动退化为全量（`clear` + 重扫），仍 bit-exact。
///
/// ## 边界
///
/// - 空 ParseLayer（无线段）⟹ `Classification::default()` + 空 tower，cache 清空。
/// - 段账本前缩（`segments.len() < last_l0_segments_len`）⟹ cache 清空 + 全量重扫（bit-exact 退化）。
/// - merged_bars 前缀改写（inclusion 合并回退）⟹ MACD cache 局部重建（bit-exact 退化）。
pub fn classify_with_tower_incremental(
    l0: &ParseLayer,
    config: &ThetaConfig,
    cache: &mut TowerCache,
) -> (Classification, Vec<Rc<Vec<LeveledMove>>>) {
    let min_parts = config.level.min_parts_per_level as usize;
    let l_max = config.level.l_max as usize;

    // ★#543 D1a 构造证书 seam（`rebase_txn`，纯观测）：env `OPSEM_DUMP_DIR` 未设 ⟹ `txn_on=false`，
    // 下面全部快照/装配/落盘点一律不进（生产路径逐字节不变，同 opsem-dump 先例）。
    // `txn_bar` = 本 bar 末 merged bar 的 source_index——与 trades/tower_events/rebase_observability
    // 的 `bar` 同一坐标系（== 下方 `close_src` 末位，`update_closes_cache` 逐 bar push 同一字段）。
    // 声明前移到回缩检测（下方）之前——回缩检测的 txn 快照块需要 `txn_on`/`txn_cleared` 已在作用域内
    // （#712 收 #645 HIGH-1 随动：回缩检测本身前移到 `l0_units_cache` 构建之前，见下）。
    let txn_on = rebase_txn::enabled();
    let txn_bar = if txn_on {
        l0.merged_bars.last().map_or(0, |b| b.source_index)
    } else {
        0
    };
    // 整塔缓存全清（段账本回缩退化路径）丢弃的旧输出，按级别下标暂存；由下面的级别循环在同 bar
    // 全量重扫后配成 old→new 事务（未走到的级别在循环后补一条 removed-only 证书）。
    let mut txn_cleared: Vec<Vec<rebase_txn::TxnNode>> = Vec::new();

    // 前缀不变量校验：段账本回缩 ⟹ 清空重扫（bit-exact 退化，非增量）。
    //
    // ★#613（#609 F2 根因收口，#712 收 #645 HIGH-1）：本检测**必须**在 `l0_units_cache` 构建之前。
    // 它曾位于构建之后（旧序：00_l0_units_build → Rc::clone 出借 → 回缩 clear），于是回缩 bar 上
    // `TowerCache::clear` 把刚建好的 `l0_units_cache` 清空（该方法逐字段清塔缓存），而本 bar 的塔
    // 构造消费的是 clear **之前** `Rc::clone` 出的局部 `l0_units`（写时复制 ⟹ 值完整）⟹ 函数返回后
    // `l0_units_cache` 为空而 `tower[0]` 满载，`level_scan_units(1)` 的只读契约「与 tower[0] 同序
    // 同长同源」被破（影子评审 #645 HIGH-1 实测：段账本回缩 bar 上 `0` vs `4`）。提前后
    // `l0_units_cache` 已清 ⟹ 下方 `00_l0_units_build` 的 reuse=0 ⟹ 本 bar 全量重建，与同样在 clear
    // 之后全量重建的 `moves_tower_l0` 口径归一，不变式成立（回归见
    // `tests::l0_units_stays_in_sync_with_tower0_on_segment_ledger_shrink`）。
    // 值不变（bit-exact）：复用前缀受 `segments_confirmed_len` 证书约束（parser 保证
    // `segments[..confirmed_len]` 跨 bar 逐字节稳定），回缩只发生在未确认尾部 ⟹ 全量重建 ==
    // 复用重建。
    if l0.segments.len() < cache.last_l0_segments_len {
        // ★#543 D1a 产出点⑥b：整塔全清在**级别循环之外**发生——它丢弃的旧输出既不经 frontier
        // pop、也不经 cascade 后缀失效，若不在 clear 前快照，这批旧身份就是三流里的"凭空消失"
        // （wf8 实测 seq=49/64 两次重基正是此路径）。
        if txn_on {
            txn_cleared = cache
                .levels
                .iter()
                .map(|lc| {
                    rebase_txn::snapshot_nodes("old", &lc.upper_moves, &lc.centers, &lc.win_meta)
                })
                .collect();
        }
        cache.clear();
    }

    // L0 输入单元 = parser 线段账本。#106 证书增量（同 moves_tower_l0，消除每 bar 全量 collect）。
    // 返回 `reuse` = L0 units 复用前缀长度 = dirty_from[0]（§2.3：L0 不可变前缀，units[..reuse]
    // 逐字段等上 bar，units[reuse..] 本 bar 新 extend）。
    let l0_dirty_from = stage_profile::time("00_l0_units_build", || {
        let reuse = l0.segments_confirmed_len.min(cache.l0_units_cache.len());
        // make_mut：上 bar 的 l0_units Rc 已在循环内被投影重赋值 drop ⟹ strong_count==1 ⟹ 原地
        // truncate+extend O(tail)；>1（caller 跨 bar 持有）⟹ 写时复制（bit-exact，同 moves_tower_l0）。
        let c = Rc::make_mut(&mut cache.l0_units_cache);
        c.truncate(reuse);
        c.extend(l0.segments[reuse..].iter().map(segment_to_unit));
        reuse
    });
    // ★[H4] Rc::clone（引用计数 O(1)）替代全量 `.clone()`（O(segments)/bar × n = O(n²)，profile 坐实
    // 400K=380ms）。下游 `units` 只读消费（compose/extract 借 &[UnitRange]），bit-exact：值不变仅所有权。
    let l0_units: Rc<Vec<UnitRange>> =
        stage_profile::time("00b_l0_units_clone", || Rc::clone(&cache.l0_units_cache));

    // DIAG(frontier-bit-exact): 对拍复用版 l0_units_cache vs 全量 segment_to_unit（隔离 L0 units 前缀复用是否陈旧）。
    if std::env::var(super::env_registry::DIAG_L0UNITS).is_ok() {
        let full: Vec<UnitRange> = l0.segments.iter().map(segment_to_unit).collect();
        if *l0_units != full {
            let m = l0_units.len().min(full.len());
            let first = (0..m).find(|&i| l0_units[i] != full[i]);
            eprintln!(
                "[DIAG-L0UNITS] segs={} confirmed_len={} cache.len(reuse前)={} ★l0_units陈旧 first_diff={:?} \
                 lens=({},{})",
                l0.segments.len(), l0.segments_confirmed_len, cache.l0_units_cache.len(),
                first, l0_units.len(), full.len()
            );
        }
    }

    // 空 L0：无可构造级别 + 清空缓存（下次从头扫）。
    if l0_units.is_empty() {
        // ★#551：L0 无单元 ⟹ 本 bar 零候选观察 ⟹ 在案活候选的身份**当场**消失，必须当场判
        // `Invalidated`。旧路径在此早退且不推进事件簿，失效被推迟到下一个非空 bar——延迟非无痕：
        // 失效钟会落到错误的 as_of 上，且此间事件簿把已消失的候选继续挂为 active（#535 红线③
        // 「丢 Invalidated 路径」的时序变体）。`clear()` 按 #550 裁定保留因果簿，故此处 advance
        // 作用于保留下来的生命史，走的是既有「观察缺席 ⟹ Invalidated」单一路径，不新增判据。
        let as_of = l0.merged_bars.last().map_or(0, |bar| bar.source_index);
        // ★#543 D1a 产出点⑥a：空 L0 全清——本 bar 没有任何重扫产出，旧对象全部 removed，
        // 逐级各记一条 removed-only 证书（否则这批旧身份在三流里凭空消失）。
        if txn_on {
            for (level_idx, lc) in cache.levels.iter().enumerate() {
                if lc.upper_moves.is_empty() {
                    continue;
                }
                let old =
                    rebase_txn::snapshot_nodes("old", &lc.upper_moves, &lc.centers, &lc.win_meta);
                let _ = rebase_txn::emit(
                    rebase_txn::TxnContext {
                        bar: txn_bar,
                        level: level_idx,
                        cause: "cache_clear",
                        cascade_reason: "empty_l0",
                        dirty_e: 0,
                        resume_start: 0,
                        prefix_count: 0,
                    },
                    old,
                    Vec::new(),
                    None,
                );
            }
        }
        cache.clear();
        cache.candidate_book.advance(&[], as_of);
        return (Classification::default(), Vec::new());
    }

    cache.last_l0_segments_len = l0.segments.len();
    // ★#93：落账本 bar L0 确认前缀（tower[0] 水线，见字段文档）。min 防御 parser 古怪末段计数。
    cache.l0_confirmed_len = l0.segments_confirmed_len.min(l0.segments.len());

    // L0 走势塔 = 携坐标的 RMove::Segment（递归底）。
    // ★#106 证书增量（替代每 bar 全量 from_unit 重建 = O(segs)×n，profile 坐实 400K=5.8s）：
    // parser `segments_confirmed_len` 保证 segments[..confirmed_len] 跨 bar bit-stable（仅末段可古怪
    // 线段重划，codex 确认）⟹ moves_tower_l0[..reuse] 复用（from_unit 只依赖单 seg，无相邻依赖）。
    // ordinal=reuse+i 全局索引（前缀 reuse<=confirmed_len 时 ordinal 不变 = 全量 enumerate，bit-exact）。
    // make_mut：caller 逐 bar drop 上轮 tower_snapshots[0] ⟹ strong_count==1 ⟹ 原地 O(tail)；
    // >1（理论 caller 跨 bar 持有）⟹ 写时复制（仍 bit-exact）。clear() 已同步清空（退化全量）。
    // ★on2w2 E1：L0 塔（tower[0]）字节变更判据。`moves_tower_l0` 是 `l0.segments` 的纯函数
    // （from_unit∘segment_to_unit，ordinal=index）⟹ 内容变更 ⟺ segments 变更。parser 证书保
    // segments[..confirmed_len] 跨 bar bit-stable（同 line 1069）⟹ 只需比 tail [reuse..]。
    // ★实证订正（on2w2 实装，CL 8K）：初版设计用「reuse<len ∨ segments[reuse..]非空」长度判据，
    // 但 frontier resume 使 truncate+repush 每 bar 发生（segments 有未确认尾段 ⟹ reuse<len 恒真），
    // 而 repush 的尾段字节 99.2% 与旧值相同（inclusion-only 不改段）⟹ 长度判据 bump 率 100%，
    // O(n²) 未消除。故改为**逐值比对 tail**（O(未确认尾段)=O(1) 摊还，非全塔）——精确匹配
    // of_forest 真变率 0.78%。over-invalidate 方向保留：长度不等或任一尾段值不等即 dirty。
    let forest_dirty_l0 = {
        let old = &cache.moves_tower_l0;
        let old_len = old.len();
        if old_len != l0.segments.len() {
            true
        } else {
            let reuse = l0.segments_confirmed_len.min(old_len);
            (reuse..old_len).any(|i| {
                let u = segment_to_unit(&l0.segments[i]);
                old[i]
                    != LeveledMove::from_unit(
                        &u,
                        ElementId {
                            level: 0,
                            ordinal: i as u64,
                        },
                    )
            })
        }
    };
    stage_profile::time("01_l0_tower_rebuild", || {
        let reuse = l0.segments_confirmed_len.min(cache.moves_tower_l0.len());
        let m = Rc::make_mut(&mut cache.moves_tower_l0);
        m.truncate(reuse);
        for (off, seg) in l0.segments[reuse..].iter().enumerate() {
            let i = reuse + off;
            let u = segment_to_unit(seg);
            m.push(LeveledMove::from_unit(
                &u,
                ElementId {
                    level: 0,
                    ordinal: i as u64,
                },
            ));
        }
    });
    let moves_tower_l0: Rc<Vec<LeveledMove>> = Rc::clone(&cache.moves_tower_l0);

    // MACD hist（背驰真算，增量递推——231号纯性能，解 aed4d5f5 实证的 c_exp≈2.31 主导根因）。
    //
    // 增量策略（bit-exact 铁律，ac75d4b3 已证 append bit-exact）：
    // - merged_bars.close 前缀与 `macd_closes_prefix` 逐值一致 + 长度 >= 前缀 ⟹ 从
    //   `macd_state` 增量 append 新 close，`current_point().hist` 追加到 `macd_hist`。
    // - merged_bars 回缩 / 前缀改写 / cache 空 ⟹ `macd_state_from_closes` 全量重建
    //   state，并逐 bar append 重建 hist 数组（bit-exact 退化，同塔 cache clear 逻辑）。
    //
    // ★不调 `compute_macd`（全量）——增量路径用 `MacdState::append` O(1)/bar；退化路径
    //   用 `macd_state_from_closes` + 逐 bar `current_point`（与全量同 EMA 约简，bit-exact）。
    // closes/close_src 增量缓存（前缀稳定，仅尾 bar 可能 inclusion 改写——同 MACD stable_prefix 语义）。
    // 不每 bar 全量重建（O(n)/bar → O(n²) 根因之一，工位 E profile 坐实 t=0.085s@16K）。
    // mem::take 取出缓存 Vec（避免 &cache.closes 与下游 &mut cache 别名），用毕放回（缓冲复用，零额外分配）。
    stage_profile::time("00_update_closes_cache", || {
        update_closes_cache(&l0.merged_bars, l0.merged_confirmed_len, cache)
    });
    let closes: Vec<f64> = std::mem::take(&mut cache.closes);
    let close_src: Vec<usize> = std::mem::take(&mut cache.close_src);
    // ★force_state 生产热路由（beta-route #115）：closes_tick（整数 close）mem::take 出借（同 closes
    // 模式，避免与下游 &mut cache 别名），供一类候选 A/C 段振幅/速度 proxy。用毕放回（下同 closes）。
    let closes_tick: Vec<Tick> = std::mem::take(&mut cache.closes_tick);
    // 增量 MACD：更新 cache.macd_hist + cache.macd_dif（锁步；不返回克隆，直接借用缓存避免 O(n) 拷贝）。
    stage_profile::time("02_macd_incremental", || {
        compute_macd_hist_incremental(&closes, l0.merged_confirmed_len, &config.macd, cache)
    });
    let hist: &[f64] = &cache.macd_hist;
    // dif 借用（与 hist 同——disjoint field 借用；force DIF 峰 proxy 输入，bit-exact 等价全量）。
    let dif: &[f64] = &cache.macd_dif;
    // B3 #4 area-memo：`stable_len` = 本 bar hist 的确认边界（[`AreaCache`] 文档）——`area_cache`
    // 跨 bar 持久（mem::take 出借，用毕放回，同 closes/close_src 模式）。`RefCell` 包裹：
    // `divergence_of` 闭包接口是 `impl Fn(&RMove) -> bool`（`signal::extract_second_signals`），
    // `Fn` 只给闭包体 `&self` 访问——捕获的可变缓存须走内部可变性（共享引用 + `borrow_mut`），
    // 不能捕获 `&mut AreaCache`（Long/Short 两侧各建一个闭包，同一 `&mut` 不能捕获两次）。
    let stable_len = cache.macd_state_len;
    let area_cache: RefCell<AreaCache> = RefCell::new(std::mem::take(&mut cache.area_cache));

    let mut levels: Vec<LevelState> = Vec::new();
    // ★O(n) 重构：snapshots 存 Rc——L≥1 级 push Rc::clone(&lc.upper_moves)（O(1)）；L0 级 push
    // moves_tower_l0（Rc）。下游（runner/interp/l3）只读借 &[Rc<Vec<LeveledMove>>]。
    let mut tower_snapshots: Vec<Rc<Vec<LeveledMove>>> = Vec::new();

    // 逐级增量构造。`units`/`moves_tower` 是本级输入（前缀来自缓存，尾部新增）。
    // ★H7 Rc 化：loop 内 units 只读（读 len/切片/传 &[]，从不原地改），下一级由 stage 10
    // `Rc::clone(&lc.projected_units)` O(1) 重赋，替代全量 clone。
    // ★H4：L0 首级 units = l0_units（本身已是 Rc<Vec<UnitRange>>，00b 阶段 Rc::clone 出借缓存），
    // 直接 move 入 units（无 Rc::new 双重包裹）。
    let mut units: Rc<Vec<UnitRange>> = l0_units;
    // Q7-#1 裁定C：units 方向锚资格（循环携带，级别-N 在投影点与 units 同步派生；L0 分支不消费）。
    let mut units_anchors: Vec<Option<Direction>> = Vec::new();
    let mut moves_tower: Rc<Vec<LeveledMove>> = moves_tower_l0;

    // ★cascade reset（codex 异质审查裁决：option 2）：任一级检出 frontier 变异 ⟹ 该级**及所有更高级**
    // 无条件 reset。根因：本级守卫只比对 `project_to_units` 投影（有损——丢弃 `sub_moves`/`rmove.subs`）。
    // 当 L0 末段内点改写使上级投影 bit-identical 但底层 sub_moves 变时，上级 frontier_mutated=false 会
    // 漏 reset ⟹ `upper_moves` 深嵌套 sub_moves 陈旧 + BSP memo 复用陈旧（codex 反例
    // `cascade_reset_on_frontier_interior_rewrite` 坐实）。cascade 消除整类"投影是否捕获深字段变化"
    // 的易错判断（no-patch）：下级变异无条件向上传播，上级不依赖投影完备性。代价：变异 bar（16K 中
    // ~120 次稀疏）该级+所有上级全量重扫，amortized 仍 O(n)。
    let mut cascade_reset = false;
    // ★on2w2-cascade 失效边界定理（设计 §1）：本 bar 累积脏源下界 `e`（最小改变源坐标，逐级恒定
    // 传播）。init usize::MAX（=+∞，无改变 ⟹ 无 cascade）。**生产逻辑**：cascade 命中时按
    // `read_end_src < e` 取保留前缀 P，前缀 bit-identical 保留、后缀失效重扫（撤 #65「整塔前缀清空」）。
    let mut dirty_e: usize = usize::MAX;
    // 放行条件3 falsification 探针启用开关（仅 test 构建；测「保留前缀比例 P/len」判设计前提）。
    #[cfg(test)]
    let eprobe_on = *CASCADE_EPROBE
        .get_or_init(|| std::env::var(super::env_registry::THETA_CASCADE_EPROBE).is_ok());
    // ★工位 4g：本 bar 是否有任一级 extend 非空 tail（含新级涌现首产 + 最高级 append）——驱动
    // generation +1（与 cascade_reset 一起完整覆盖 extract 可观察树变更，codex Q3）。
    let mut did_extend = false;
    // ★on2w2 forest_epoch dirty 累积器（E1-E3 折叠，循环后统一 bump——避开与循环内 `lc` 可变别名，
    // 同 did_extend/cascade_reset 模式）。E1（L0 塔重建）在循环前置位；E2（upper extend）/E2a
    // （frontier pop）/E3（cascade clear）在循环内 `|=`。E4（clear()）不经此，方法内直接 bump。
    let mut forest_dirty = forest_dirty_l0;
    // ★A3 证书（per-level dirty_from，§2.4）：本级 units 的不可变前缀长度。L0 = l0_dirty_from
    // （§2.3）；L≥1 = 父级 prefix_count（loop 尾 `dirty_from = prefix_count`）。驱动 03（L1+ stable
    // 从 0 抬起）+ 04（cached_units truncate+extend O(tail)）。
    let mut dirty_from: usize = l0_dirty_from;
    let mut candidate_observations = Vec::new();

    // ★#543 D1a：同 bar 下一级事务（level_idx-1）的连续边——供本级把「源 X 是同 lineage 的修订」与
    // 「ordinal 位置号复用」分开（调研 §5.3 递归链接组）。仅当上一条恰是本级的下一级时使用。
    let mut txn_lower: Option<(usize, u64, rebase_txn::LowerMap)> = None;

    for level_idx in 0..=l_max {
        // 自然终止：单元数 < min_parts ⟹ 停止（与 classify_impl 同口径）。
        if units.len() < min_parts {
            // ★§2.6：level_idx 及所有更高级本 bar 全跳过（循环顶部 break，lc 未触碰）——truncate 掉
            // 这段不可达尾巴的陈旧 LevelCache，令其恢复时走 LevelCache::default() 全扫（血缘自洽，
            // 防 stable=dirty_from.min(scanned) 坍缩假阴）。
            #[cfg(test)]
            oracle_probe::on_minparts_break(level_idx < cache.levels.len());
            cache.levels.truncate(level_idx);
            break;
        }

        let is_l0 = level_idx == 0;

        // 缓存槽按需扩展（首次到达该级 ⟹ 新建空 LevelCache，start_i=0 全量扫）。
        if cache.levels.len() <= level_idx {
            cache.levels.push(LevelCache::default());
        }
        let lc = &mut cache.levels[level_idx];

        // 前缀不变量校验（anc.pdf §16：confirmed prefix immutable / frontier mutable）。
        // 两类违反 resume 充要条件 #2（`units[..consumed]` 不可变）⟹ 该级缓存全量重置（退化为
        // 全量扫，bit-exact）：
        //   (1) 长度回缩：units.len() < last_input_len（段账本前缩）。
        //   (2) frontier 末段原地改写：长度不变/增长但**扫描区**（`units[..consumed+2]`，已 build 读过
        //       的单元）内某单元值变了——parser 古怪线段重划改写末段（定义层正确，非 bug）。仅长度
        //       守卫漏此例（bar 1464 seg[9] end 1384→1170，段数不变）。扫描区外（未读尾部）的变化
        //       无害（resume 从 consumed 续扫会读到新值），不触发重置。
        let scanned = (lc.scan_cursor.consumed + 2)
            .min(units.len())
            .min(lc.cached_units.len());
        // #106 证书跳前缀（仅 L0 安全，codex 裁决）：L0 units = segments 投影，segments[..confirmed_len]
        // 跨 bar bit-stable ⟹ units[..stable] 必等，只比 [stable..scanned]。L1+ units 来自上级投影
        // （无 segments 证书）⟹ stable=0 全量比较（保守，不假定相等）。证书不证明 [stable..scanned]
        // 没变（bar-1464 末段改写在 scanned frontier 内）⟹ 该区间仍逐值比，触发 cascade。
        // ★A3 §3.3：L0 保持 confirmed_len 证书（不变）；L1+ 从保守 0 抬升到父级 prefix_count
        // （dirty_from），前缀跳过不比较（可证不可变）——收益全在 L1+。frontier 窗 [stable..scanned]
        // 仍逐值比（bar-1464 末段改写在 scanned 内），不漏 cascade。
        let stable = if is_l0 {
            l0.segments_confirmed_len.min(scanned)
        } else {
            dirty_from.min(scanned)
        };
        let frontier_mutated = stage_profile::time("03_frontier_compare", || {
            units[stable..scanned] != lc.cached_units[stable..scanned]
        });
        // 本级触发 reset ⟹ 置 cascade，所有更高级无条件跟随（投影有损，上级不能仅靠本级投影判断）。
        if units.len() < lc.last_input_len || frontier_mutated {
            cascade_reset = true;
        }
        // ★on2w2-cascade 失效边界定理（设计 §1.3）：累积脏源下界 `e`（最小改变源坐标，逐级传播）。
        // 现为**生产逻辑**（驱动增量失效），非探针。区间 [stable..scanned] 首个改变下标 j_min ⟹
        // units[j_min].start_index（取 min 保证 units[..j_min] 未变，§3.2）。len-shrink ⟹ e=0 全清
        // （设计 §5：回缩罕见，不做增量）。跨级 min（与布尔 OR 同位置、同无条件语义，§2.3 逐级恒定）。
        if units.len() < lc.last_input_len || frontier_mutated {
            let local_e = if units.len() < lc.last_input_len {
                0 // (A) len-shrink：退化全清（设计 §5）。
            } else {
                // (B) frontier_mutated：区间内首个改变下标 j_min ⟹ units[j_min].start_index。
                match (stable..scanned).find(|&j| units[j] != lc.cached_units[j]) {
                    Some(j) => units[j].start_index,
                    None => usize::MAX, // 理论不达（frontier_mutated 蕴含存在改变），保守不降 e。
                }
            };
            dirty_e = dirty_e.min(local_e);
        }
        // ★#543 D1a：本级事务的旧侧节点累积器（cascade 丢弃的后缀 + frontier pop 的整窗），
        // 与事务头的 cause/cascade_reason。`txn_on=false` 时三者恒为初值且无人读（零开销）。
        let mut txn_old_nodes: Vec<rebase_txn::TxnNode> = Vec::new();
        let mut txn_cause: &'static str = "append";
        let mut txn_cascade_reason: &'static str = "-";
        let mut txn_cascade_dropped = false;
        // ★#543 D1a 产出点⑥b 续：整塔全清丢弃的本级旧输出接进本级事务，与同 bar 全量重扫的
        // 新产出配成 old→new 边（`mem::take` 后循环尾的补记不会重复登记）。
        if txn_on {
            if let Some(dropped) = txn_cleared.get_mut(level_idx) {
                if !dropped.is_empty() {
                    txn_old_nodes = std::mem::take(dropped);
                    txn_cause = "cache_clear";
                    txn_cascade_reason = "l0_segment_ledger_shrink";
                    txn_cascade_dropped = true;
                }
            }
        }
        if cascade_reset {
            // ★on2w2-cascade E3：本级 upper_moves 尾段失效（tower[level+1] 字节变更）⟹ forest 变。
            forest_dirty = true;
            // ★增量失效边界（设计 §1.2/§3.4）：保留 `read_end_src < e` 的前缀 P（读域上界含停止哨兵，
            // 覆盖 codex §1.2.1 反例）。win_meta 与 centers 1:1 对齐、read_end_src 升序（源单调 S1）⟹
            // partition_point 定位 P。升级子中枢共享父窗口 read_end_src ⟹ 整窗保留或整窗失效（§3.5）。
            let mut p = lc.win_meta.partition_point(|w| w.read_end_src < dirty_e);
            // ★放行条件3 keep_frac 探针（test+eprobe，release 剥离）：真实 P/len（不再用 end_index 代理）。
            #[cfg(test)]
            if eprobe_on && !lc.centers.is_empty() {
                oracle_probe::on_cascade_event(p as f64 / lc.centers.len() as f64);
            }
            // ★全清对照（test-only，铁律 H1 神谕先例）：强制 P=0 退回整塔前缀清空，A/B 计时 + bit-exact
            // 对照。默认 off ⟹ 增量路径。开启后 bit_exact_per_bar 仍须绿（证 P>0 与全清逐字段相等）。
            #[cfg(test)]
            if *CASCADE_FULLCLEAR
                .get_or_init(|| std::env::var(super::env_registry::THETA_CASCADE_FULLCLEAR).is_ok())
            {
                p = 0;
            }
            // ★#543 D1a 产出点④（cascade 后缀失效，P=0/P>0 两支共用）：被丢弃的旧后缀 `[p..]` 在
            // truncate/clear **之前**形成只读快照。这些旧对象不经下方 frontier pop，若不在此捕获，
            // 三流里就只剩「旧三元组凭空消失」（调研 §5.2 第 4 点点名必须覆盖）。
            if txn_on {
                txn_cause = if p == 0 {
                    "cascade_p0"
                } else {
                    "cascade_prefix"
                };
                txn_cascade_reason = if units.len() < lc.last_input_len {
                    "len_shrink"
                } else if frontier_mutated {
                    "frontier_mutated"
                } else {
                    // 本级自身未变异，cascade 由更低级别无条件传播而来。
                    "inherited"
                };
                txn_old_nodes = rebase_txn::snapshot_nodes(
                    "old",
                    &lc.upper_moves[p..],
                    &lc.centers[p..],
                    &lc.win_meta[p..],
                );
                txn_cascade_dropped = !txn_old_nodes.is_empty();
            }
            if p == 0 {
                // P=0（无可保留前缀，含 e=0 全清 / e 坍缩到起点）：退化为原全清（bit-exact，与现码同）。
                lc.scan_cursor = WindowScanCursor::default();
                // make_mut：若上 bar snapshot 仍持引用则写时复制再 clear（退化 bit-exact）；否则原地清。
                Rc::make_mut(&mut lc.upper_moves).clear();
                Rc::make_mut(&mut lc.centers).clear();
                Rc::make_mut(&mut lc.cp_ownership).clear();
                lc.win_meta.clear();
                lc.decompose_state.reset();
                Rc::make_mut(&mut lc.cached_bsp).clear();
                Rc::make_mut(&mut lc.cached_pan_div).clear(); // Q4：与 cached_bsp 同批失效（同 key 守卫）。
                Rc::make_mut(&mut lc.cached_first_class_grades).clear(); // #885：同批失效（同 key 守卫）。
                lc.cached_bsp_key = None;
                lc.cached_candidate_key = None;
                lc.cached_second.clear(); // 07b 门控：前缀重排 ⟹ 前缀 B2 缓存失效，重扫。
                lc.cached_second_count = 0;
                lc.confirmed_watermark = 0; // ★#93 全清 ⟹ 水线归零（消费方全量重比）。
                Rc::make_mut(&mut lc.projected_units).clear(); // #106：投影缓存失效，重投影。
            } else {
                // P>0（增量失效，设计 §3）：保留 [..P] 前缀（读域<e，L1 bit-identical），失效 [P..] 后缀。
                // cursor 重建为「把 P-1 窗口当 frontier 待 pop」——回归常态 frontier pop 语义（§3.4）：
                // resume_from=win_meta[P-1].win_start，从该起点重扫复现被 pop 窗口（哨兵翻转则重扫吸收，
                // 自动覆盖 §1.2.1）；从 win_start 重扫至 len 覆盖 None-支失败 seed 尾部（§2.2.1）。
                // 下方 had_emitted_window 块（现有 bit-exact pop 机制）据此 cursor pop P-1 窗口整窗。
                let wm = lc.win_meta[p - 1];
                lc.scan_cursor = WindowScanCursor {
                    consumed: wm.win_exit,
                    resume_from: wm.win_start,
                    last_window_emitted: wm.emitted,
                };
                debug_assert!(
                    wm.read_end_src < dirty_e,
                    "O2 哨兵读域断言：保留末窗 P-1 读域上界 {} 必 < e={} （否则读了 dirty 单元，须左退）",
                    wm.read_end_src, dirty_e
                );
                // 07b 门控（设计 §3.1 + 放行条件4 契约）：cascade 后 cursor 把 P-1 窗口当 frontier 待
                // pop（§3.4）⟹ 下方 had_emitted_window 块把 prefix_count pop 到 `p - emitted`。
                // ★关键（bar-10299 修复）：cached_second **只含由前 old_count 个 parent 产出的 B2**——不能
                // 声明超过 old_count 的覆盖（否则 extract_second_resume 跳过 [old_count..) 的重算 ⟹ 漏 B2）；
                // 也不能保留被失效前缀 [p..old_count) 的 B2。故新覆盖锚 = min(old_count, pop_prefix)：
                // 既 <= pop 后 confirmed 前缀（放行条件4 单调），又 <= 已实际缓存的 parent 数。
                //
                // ★分离锚（bar-27947 修复）：cached_second 按 **parent 追加序**（非全局 source 排序——sort
                // 在 mod.rs 合并端，非缓存内）。B2 source_index = parent 内某 sub_move 的 end_index，落
                // parent 源区间；parent 源区间不重叠但**共享边界坐标**（unit.end == 下一 unit.start）。故
                // 分离锚须用「前一保留 parent 的 end_index，含界」：parent[b2_count-1].end_index。parent
                // [b2_count] 的首个 B2 source_index > 其 start_index >= 该 end_index ⟹ `<=` 精确分离，不误
                // 丢边界 B2（用 parent[b2_count].start_index 的 `<` 会丢掉恰落共享边界的前 parent B2）。
                let pop_prefix = p - wm.emitted;
                let b2_count = lc.cached_second_count.min(pop_prefix);
                // 含界上锚：前 b2_count 个 parent 中最后一个的 end_index（b2_count==0 ⟹ 无保留 ⟹ cut 前于全部）。
                let b2_cut_incl = if b2_count == 0 {
                    None
                } else {
                    Some(lc.upper_moves[b2_count - 1].end_index)
                };
                Rc::make_mut(&mut lc.centers).truncate(p);
                Rc::make_mut(&mut lc.upper_moves).truncate(p);
                Rc::make_mut(&mut lc.cp_ownership).truncate(p);
                lc.win_meta.truncate(p);
                // ★#93 水线收缩到保留前缀 P（[P..] 本 bar 重扫可能改写）；bar 末再 min(w_nat)。
                lc.confirmed_watermark = lc.confirmed_watermark.min(p);
                // decompose：保留 reset()（O(centers)=百级，非 05/09/07b 的 O(n²) 靶，设计 §3.2 scope）。
                // 输出恒等全折叠（decompose 模块头），reset+重折 bit-exact，仅不省非瓶颈的重折量。
                lc.decompose_state.reset();
                // cached_bsp memo：key=(centers.len,..) 变 ⟹ 必 miss ⟹ 部分保留零收益（设计 §3.2），维持全失效。
                Rc::make_mut(&mut lc.cached_bsp).clear();
                Rc::make_mut(&mut lc.cached_pan_div).clear();
                Rc::make_mut(&mut lc.cached_first_class_grades).clear(); // #885：同批失效（同 key 守卫）。
                lc.cached_bsp_key = None;
                lc.cached_candidate_key = None;
                let b2_keep = match b2_cut_incl {
                    None => 0,
                    Some(cut) => lc.cached_second.partition_point(|b| b.source_index <= cut),
                };
                lc.cached_second.truncate(b2_keep);
                lc.cached_second_count = b2_count;
                // 投影缓存：截到 P（前缀投影稳定，L2 §2.3）——下方 stage 09 truncate(prefix_count) 会进一步
                // 截到 pop 后的 prefix_count（pop_prefix），从此续投影（O(tail)）。此处截 p 是保守上界。
                Rc::make_mut(&mut lc.projected_units).truncate(p);
            }
        }
        lc.last_input_len = units.len();
        // 快照本级输入（下 bar 比对 frontier 变异）。★A3 §3.2 证书化：cached_units[..dirty_from]
        // == units[..dirty_from]（前缀可证不可变，上 bar 04 已写同值）⟹ 前缀无需重拷，truncate
        // + extend tail = O(tail)。三重 min 保证终长 == units.len()（含 units 回缩）。
        // debug_assert = debug/test 护栏（release profile 按 Rust 语义剥离——省下的即此全前缀比较）。
        stage_profile::time("04_cached_units_copy", || {
            let keep = dirty_from.min(lc.cached_units.len()).min(units.len());
            debug_assert!(
                units[..keep] == lc.cached_units[..keep],
                "dirty_from 证书违反：cached_units[..{}] 应等于 units 前缀（bit-exact 护栏）",
                keep
            );
            lc.cached_units.truncate(keep);
            lc.cached_units.extend_from_slice(&units[keep..]);
        });

        // ★frontier 修复（task #47/#21，区间套.pdf 六~十节裁决②）：resume 起点用 `resume_from`
        // （上次扫描最后一个成立窗口的起点）**而非** `consumed`——`consumed` 越过了最后一个成立
        // 窗口，把它当 sealed prefix，但该窗口第三段可能是 frontier（新 bar 后其后续段落使全量非
        // 重叠扫描产出不同中枢）。回退：从 `resume_from` 重扫 ⟹ 最后一个（frontier）中枢每 bar 重算，
        // 真正 sealed（后面又出现成立窗口）后自然稳定。对应地 **pop 最后一个 center/upper_move**
        // （它会被重扫重新产出），`prefix_count` 用 pop 后的长度（ordinal 接续，全量/增量同 ID）。
        //
        // 保守正确性（PDF §九增量等价定理）：`resume_from <= consumed` 恒成立（窗口起点 ≤ 退出点），
        // 从更早处重扫产出的 tail ⊇ 从 consumed 重扫的 tail（多含重算的末窗口）。cascade_reset 后
        // `scan_cursor = default`（resume_from=0=consumed）⟹ 无回退，从 0 全扫（bit-exact 退化）。
        // 无成立窗口时 `resume_from == 上次 start_i`（无中枢可 pop），guard `resume_from < consumed`
        // 为假 ⟹ 不 pop、从 resume_from(=上次start_i) 续扫（续进语义，仅不成立支推进过的区间）。
        let resume_start = lc.scan_cursor.resume_from;
        let had_emitted_window = lc.scan_cursor.resume_from < lc.scan_cursor.consumed;
        // ★on2w2：本级 upper_moves（=tower[level+1]，forest 输入）字节变更判据。upper_moves 前缀
        // [..prefix_count] 不可变（§16 confirmed），本 bar 只改尾部：pop 掉 `popped_upper`（末窗产出）
        // 后 extend `tail_upper`（重扫产出）⟹ **变更 ⟺ popped_upper != tail_upper**（逐值）。无 pop
        // 时 popped_upper 空 ⟹ 变更 ⟺ tail_upper 非空（纯追加，=E2）。捕获 pop 掉的 upper 尾段以供比对。
        let mut popped_upper: Vec<LeveledMove> = Vec::new();
        let mut popped_cp: Vec<CpScanOwnership> = Vec::new();
        if had_emitted_window {
            // pop 最后成立窗口的**全部**产出（frontier 域 = 整窗，重扫从窗口起点重产）——
            // ★#148 升级重切后一窗可产 k 个子中枢（`last_window_emitted`），只 pop 1 会残留旧
            // 子中枢与重扫产出重复。前缀不变量不破（pop 的是尾部整窗）。
            let pop_n = lc.scan_cursor.last_window_emitted;
            debug_assert!(
                pop_n >= 1 && lc.centers.len() >= pop_n && lc.upper_moves.len() >= pop_n,
                "had_emitted_window ⟹ 末窗口产出（pop_n={pop_n}）可回退"
            );
            // ★#543 D1a 产出点①（旧开放整窗 pop，truncate 前只读快照）：现有代码只捕获
            // `popped_upper`，会丢掉对应的 old `WinMeta`/`Center`（调研 §5.2 第 1 点明列）。
            // 旧节点按 output_ordinal 升序排列——pop 掉的整窗在 cascade 丢弃的后缀之前。
            if txn_on {
                let keep = lc.upper_moves.len().saturating_sub(pop_n);
                let mut popped_nodes = rebase_txn::snapshot_nodes(
                    "old",
                    &lc.upper_moves[keep..],
                    &lc.centers[keep..],
                    &lc.win_meta[keep..],
                );
                popped_nodes.append(&mut txn_old_nodes);
                txn_old_nodes = popped_nodes;
                if txn_cause == "append" {
                    txn_cause = "frontier_pop";
                }
            }
            let cs = Rc::make_mut(&mut lc.centers);
            cs.truncate(cs.len().saturating_sub(pop_n));
            let cp = Rc::make_mut(&mut lc.cp_ownership);
            let cp_keep = cp.len().saturating_sub(pop_n);
            popped_cp = cp[cp_keep..].to_vec();
            cp.truncate(cp_keep);
            let um = Rc::make_mut(&mut lc.upper_moves);
            let keep = um.len().saturating_sub(pop_n);
            popped_upper = um[keep..].to_vec(); // on2w2：pop 前捕获（与重扫 tail_upper 逐值比）。
            um.truncate(keep);
            // ★on2w2-cascade：win_meta 与 centers/upper_moves 1:1 同步 pop（末窗整窗，重扫重产）。
            lc.win_meta
                .truncate(lc.win_meta.len().saturating_sub(pop_n));
        }
        // ★codex Q4：prefix_count = lc.upper_moves.len()（pop 后的已产出前缀数），tail ordinal 接续
        // 前缀 ⟹ 全量/增量产同 ElementId（跨 bar 稳定身份）。★A3 §2.2：prefix_count 同时是本级
        // projected_units 的不可变前缀（§2.5 truncate 锚）+ 下一级 units 的 dirty_from（§2.4 loop 尾）。
        let prefix_count = lc.upper_moves.len();
        // ★on2w2-cascade §3.1 契约：cached_second_count（保留 parent 前缀数）须 <= pop 后 prefix_count
        // （否则缓存越过 confirmed 边界，extract_second_resume 单调守卫会重置）。cascade truncate 设
        // cached_second_count=P=win_meta 保留数 <= upper_moves 保留数 == prefix_count（同一 truncate(p)）。
        debug_assert!(
            lc.cached_second_count <= prefix_count,
            "§3.1 契约违反：cached_second_count={} > prefix_count={}",
            lc.cached_second_count,
            prefix_count
        );
        let (tail_centers, tail_upper, mut tail_cp, tail_metas, new_cursor) =
            stage_profile::time("05_compose_resume", || {
                compose_level_resume(
                    &units,
                    &moves_tower[..],
                    is_l0,
                    level_idx as u32 + 1,
                    resume_start,
                    prefix_count,
                )
            });
        // frontier pop/recompose 若产出同一个 B_p/c_p 身份，继承已扫描对象态，只从 dirty_from 推进。
        // Closed 证书若落入 dirty 后缀则不可继承，必须从 departure 重判；证书完全位于稳定前缀才保留。
        let dirty_invalidation = recursive_tower::invalidate_cp_lifecycle_dirty_dependencies(
            Rc::make_mut(&mut lc.cp_ownership).as_mut_slice(),
            dirty_from,
        );
        cp_replay_diagnostics::record_dirty_invalidation(
            level_idx,
            dirty_invalidation.pending_fallbacks,
            dirty_invalidation.certificate_clear_recomputes,
        );
        let mut lifecycle_scan_from = dirty_invalidation.scan_from;
        for object in &mut tail_cp {
            let prior = popped_cp.iter().find(|prior| {
                prior.b_center_id == object.b_center_id
                    && prior.b_center == object.b_center
                    && prior.departure_move_id == object.departure_move_id
                    && prior.departure_interval == object.departure_interval
            });
            let prior_is_stable = prior.is_some_and(|prior| {
                recursive_tower::cp_lifecycle_dependencies_stable_before(prior, dirty_from)
            });
            if prior_is_stable {
                let prior = prior.expect("prior_is_stable 蕴含 prior Some");
                cp_replay_diagnostics::record_tail_reinherit(level_idx);
                object.lifecycle = prior.lifecycle;
                object.cp_certificate_confirm_src = prior.cp_certificate_confirm_src;
                object.c_structure = prior.c_structure;
                object.third_class_in_c = prior.third_class_in_c;
                object.full_trend_evidence = prior.full_trend_evidence.clone();
                object.full_trend_c_qualified = prior.full_trend_c_qualified.clone();
            } else if let Some(departure) = object.departure_move_id {
                lifecycle_scan_from = lifecycle_scan_from.min(departure.ordinal as usize + 1);
            }
        }

        // ★A3 oracle 探针：had_emitted_window pop 后 T = tail_upper.len()（本 bar 本级重扫产出窗口数）。
        // T==1 = did_extend 证伪正向锁（重扫仅复现被 pop 窗口，tail_upper 恰 1）；T>1 = frontier 值改写。
        #[cfg(test)]
        if had_emitted_window {
            oracle_probe::on_pop_rescan(tail_upper.len());
        }

        // ★task #143：走势分解增量无需 frontier 失效钩子——decompose_resume 只冻结 sealed
        // 关系（i < m-2，两端中枢均有后继），触 frontier 中枢的临时尾关系每 bar 重折。frontier
        // pop 后重扫改值 / 一次重扫多产两种破口均被冻结不变量覆盖（decompose.rs 模块头 + 随机
        // 事件流 parity 测试）。
        // 追加到已缓存前缀（前缀不可变，仅尾部追加）⟹ 累积 centers/upper == 全量扫描结果。
        did_extend |= !tail_upper.is_empty();
        // ★on2w2 E2+E2a（合并逐值判据）：本级 upper_moves 尾部从 popped_upper 换成 tail_upper。
        // 变更 ⟺ 两者不逐值相等（含长度）。frontier resume 每 bar pop+重扫复现相同窗口（tail_upper
        // == popped_upper）时**不置 dirty**——这是把 bump 率从 did_extend 的 ~96% 压回 forest 真变率
        // 0.78% 的机制（实证订正：初版 `|=!tail_upper.is_empty()` 对每 bar 重扫复现的相同窗口误 bump）。
        // over-invalidate 保留：长度或任一值不等即 dirty。O(tail) 比对，非全塔。
        forest_dirty |= tail_upper != popped_upper;
        // ★#543 D1a 产出点②③⑤（新 tail 已由 `compose_level_resume` 返回，old/new 完整、写回前的
        // 自然 emit 点——调研 §5.2 第 2/3 点；一窗产 k 个子中枢的关系由 `WinMeta.emitted` 带进事务，
        // 第 5 点）。**纯观测**：只读 tail 与上面的旧快照，不改任何状态、不回馈决策。
        // 平凡事务不产出——判据与上一行 `forest_dirty` 同源（pop 后重扫复现同一整窗 ⟹ 无重基），
        // 否则每 bar 每级都会刷一条无信息行。
        if txn_on
            && (txn_cascade_dropped || tail_upper != popped_upper)
            && !(txn_old_nodes.is_empty() && tail_upper.is_empty())
        {
            let new_nodes =
                rebase_txn::snapshot_nodes("new", &tail_upper, &tail_centers, &tail_metas);
            let ctx = rebase_txn::TxnContext {
                bar: txn_bar,
                level: level_idx,
                cause: txn_cause,
                cascade_reason: txn_cascade_reason,
                dirty_e,
                resume_start,
                prefix_count,
            };
            // 递归链接只认「同 bar 的紧邻下一级」——中间级别无事务时不得跨级冒充。
            let lower = txn_lower
                .as_ref()
                .filter(|(lv, _, _)| level_idx > 0 && *lv + 1 == level_idx)
                .map(|(_, id, map)| (*id, map));
            let (txn_id, lower_map) =
                rebase_txn::emit(ctx, std::mem::take(&mut txn_old_nodes), new_nodes, lower);
            txn_lower = Some((level_idx, txn_id, lower_map));
        }
        stage_profile::time("06_extend_centers_upper", || {
            // make_mut：strong_count==1 ⟹ 原地 extend O(tail)；>1 ⟹ 写时复制（bit-exact）。
            Rc::make_mut(&mut lc.centers).extend(tail_centers);
            Rc::make_mut(&mut lc.upper_moves).extend(tail_upper);
            Rc::make_mut(&mut lc.cp_ownership).extend(tail_cp);
            // ★on2w2-cascade：win_meta 与 centers/upper_moves 同步 extend（1:1 对齐不变量维持）。
            lc.win_meta.extend(tail_metas);
        });
        let cp_objects = Rc::make_mut(&mut lc.cp_ownership);
        recursive_tower::advance_cp_lifecycles(
            cp_objects.as_mut_slice(),
            &lc.centers,
            &units,
            &moves_tower,
            (!is_l0).then_some(&units_anchors[..]),
            lifecycle_scan_from,
        );
        lc.scan_cursor = new_cursor;
        // ★#93 水线推进（字段文档见 LevelCache::confirmed_watermark）：
        // w_nat = len - last_window_emitted——本 bar frontier 末窗口（下 bar pop 重产）排除。
        // cascade bar 只允许收缩（前缀 min(P) 已在 cascade 分支落账），非 cascade bar 可增长。
        {
            let w_nat = lc
                .upper_moves
                .len()
                .saturating_sub(lc.scan_cursor.last_window_emitted);
            lc.confirmed_watermark = if cascade_reset {
                lc.confirmed_watermark.min(w_nat)
            } else {
                w_nat
            };
        }
        debug_assert!(
            lc.centers.len() == lc.upper_moves.len()
                && lc.centers.len() == lc.cp_ownership.len()
                && lc.centers.len() == lc.win_meta.len(),
            "增量塔：centers/upper_moves/cp_ownership/win_meta 一一对应"
        );

        // 本级输入塔快照（compose 前）。
        // ★O(1) 优化：moves_tower 是 Rc——move 入 snapshots（所有权转移，零拷贝）。下一级用
        // Rc::clone(&lc.upper_moves) 重置 moves_tower（line 912），故此处 move 后 moves_tower 失效合法。
        // bit-exact：snapshots 内容 == 全量版（Rc 指向的 Vec 值不变，仅所有权/引用计数变）。
        tower_snapshots.push(std::mem::take(&mut moves_tower));

        // 走势分解（增量续折：resume 单一来源 ⟹ 与全量 decompose 定义性 bit-exact）。
        // ★#148：链尾可变中枢数 = 本轮末窗口产出数（升级重切窗口的全部子中枢在窗口 sealed 前
        // 均可变——外缘随延伸改写、数量随段数增长改变），冻结边界随之后移（decompose.rs 文档）。
        let moves = decompose_resume(
            &lc.centers,
            &mut lc.decompose_state,
            lc.scan_cursor.last_window_emitted.max(1),
        );
        // #69 5b（#614 并线自 kimi 线 `incremental.rs:597` 同位点重放）：无条件登记本级一/三类
        // 所用 source 水位；不得挂在 BSP memo miss 分支，否则 hit bar 会暴露陈旧 e_src。
        // 公式与下方 `extract_first_third_resume` 的 (prefix_count, dirty_e) 单一同源；本行是
        // **只写缓存**，唯一读点是 `TowerCache::freeze_boundary`（诊断 bin），零生产读点。
        lc.last_freeze_boundary = signal::freeze_boundary_src(&lc.centers, prefix_count, dirty_e);

        // BSP 提取（同 classify_impl：L0 线段层 + 递归组装层）。
        // ★增量接入：传预计算 hist（从 cache 增量产出），避免 extract_signals 内部全量 compute_macd。
        //
        // ★memo 缓存（工位 E，O(n²) 主导根因——profile 坐实 bsp t=0.279s@16K，53% of tower）：
        // BSP 是 (centers, segments, upper_moves, hist, close_src) 的纯函数。其中 centers/segments/
        // upper_moves 跨 bar 单调追加（前缀不可变）；hist/close_src 仅尾 bar（不稳定，inclusion 可改写）
        // 变化。但 **confirmed 线段/中枢/上级走势的 source_index 区间全部落在稳定前缀**——尾 bar 在
        // 所有 confirmed 元素区间之后，故不影响其 BSP 判定。⟹ 当 (centers.len, segments.len,
        // upper_moves.len) 三者与上次一致时，BSP 逐字段 bit-identical（纯函数同输入同输出 + 尾 bar 不
        // 触及 confirmed 区间）。实测 16K bar 中 L0 segments 仅变 120 次（maxseg=134），其余 ~99.2%
        // bar 全量重算是冗余——memo 把 16K 次重算降为 O(段变化次数) 次，每次 O(S)，BSP 总成本坍缩近常数。
        //
        // bit-exact 铁律：guard 命中 ⟹ 复用上次输出（纯函数同输入）；guard miss ⟹ 全量重算并刷新
        // 缓存。与无 memo 版逐字段相等（同 extract_signals_with_hist/extract_second_for_level 代码路径）。
        // ★裁定 A memo soundness：level≥1 的一/三类依赖本级输入 `units`（不止 upper_moves）——units
        // 尾部 append（新级别-N 单元，可能破最后中枢=新一类 C 段 / 与前段构成新三类回试对）会改变
        // 一/三类输出却**不**改 centers.len/upper_moves.len（新单元未凑齐三段窗口 ⟹ 无新中枢/上级走势）。
        // 故 level≥1 的结构长度键用 `units.len()`（L0 用 segments.len()）——append 变长即触 miss 重算。
        // frontier **同长改写**由 cascade_reset（frontier_mutated 比对，scan 窗覆盖全 units 因
        // consumed+2≥units.len()）清 cached_bsp_key 兜底；回缩由 last_input_len 守卫触 cascade。三情形全覆盖。
        let struct_len = if is_l0 {
            l0.segments.len()
        } else {
            units.len()
        };
        let bsp_key = (lc.centers.len(), lc.upper_moves.len(), struct_len);
        let (bsp, pan_div, first_class_grades): (
            Rc<Vec<BspPoint>>,
            Rc<Vec<signal::PanDivCert>>,
            Rc<Vec<signal::FirstClassGradeRecord>>,
        ) = if lc.cached_bsp_key == Some(bsp_key) {
            // 07c：memo 命中 ⟹ `Rc::clone`（引用计数 O(1)），替代全量 `cached_bsp.clone()`。
            // Q4：pan_div 同批命中（同 key 守卫 ⟹ 同一 extract 产出的两半锁步复用）。
            // #885：first_class_grades 同批命中（同一 extract 产出的第三半，同 key 同批锁步）。
            stage_profile::time("07c_bsp_memo_clone", || {
                (
                    Rc::clone(&lc.cached_bsp),
                    Rc::clone(&lc.cached_pan_div),
                    Rc::clone(&lc.cached_first_class_grades),
                )
            })
        } else {
            // ★on2w3-07a frontier-resume：confirmed 前缀段的一/三类点缓存复用，只重判 frontier tail
            // （消 07a O(n²) 主导项）。冻结边界锚 = min(centers[prefix_count-2].end_index, dirty_e)——
            // `moves`（= decompose_resume 输出，本级增量续折）作 blocks 单一来源（不重 decompose）。
            // segments 来源：L0=l0.segments（有序）；L≥1=units→unit_to_segment 投影（几何衰减，
            // resume 内 debug_assert 守 end_index 严格递增）。cascade 清 cached_first_third 见 §失效块。
            let (mut b, pan, grades): (
                Vec<BspPoint>,
                Vec<signal::PanDivCert>,
                Vec<signal::FirstClassGradeRecord>,
            ) = if is_l0 {
                stage_profile::time("07a_extract_signals_l0", || {
                    signal::extract_first_third_resume(
                        &mut lc.cached_first_third,
                        &mut lc.cached_first_third_pan,
                        &mut lc.cached_first_third_grades,
                        &mut lc.cached_first_third_count,
                        level_idx as u32,
                        &lc.centers,
                        &l0.segments,
                        None,
                        &moves,
                        prefix_count,
                        dirty_e,
                        hist,
                        dif,
                        &closes_tick,
                        &close_src,
                        config.divergence_gauge,
                    )
                })
            } else {
                stage_profile::time("07a_extract_first_third_ln", || {
                    // 级别-N 一/三类（裁定 A）：units 承担线段角色，复用 L0 判据。units→Segment 投影
                    // （几何衰减 O(units_L)/miss）。#486：三类 leave 与一类同取单元结构方向锚；
                    // provenance `units_anchors` 仍供塔 ownership 链消费，不再传入 BSP 判据。
                    let segs: Vec<Segment> = units.iter().map(unit_to_segment).collect();
                    let structural_anchors: Vec<Option<Direction>> =
                        units.iter().map(|u| Some(u.direction)).collect();
                    signal::extract_first_third_resume(
                        &mut lc.cached_first_third,
                        &mut lc.cached_first_third_pan,
                        &mut lc.cached_first_third_grades,
                        &mut lc.cached_first_third_count,
                        level_idx as u32,
                        &lc.centers,
                        &segs,
                        Some(&structural_anchors),
                        &moves,
                        prefix_count,
                        dirty_e,
                        hist,
                        dif,
                        &closes_tick,
                        &close_src,
                        config.divergence_gauge,
                    )
                })
            };
            let second = stage_profile::time("07b_extract_second", || {
                // ★07b frontier 门控：confirmed 前缀 parent 的 B2 缓存复用（跳过其重复背驰扫描），
                // 只对 frontier tail 每 bar 重算。消 confirmed-parent 全塔重扫 O(U²)。
                extract_second_resume(
                    &mut lc.cached_second,
                    &mut lc.cached_second_count,
                    &lc.upper_moves[..],
                    prefix_count,
                    hist,
                    &close_src,
                    &area_cache,
                    stable_len,
                )
            });
            b.extend(second);
            b.sort_by_key(|p| p.source_index);
            // miss 路径：`Rc::new` 一次，cache 与 LevelState 共享同一 buffer（消除旧 `b.clone()`）。
            let rc = Rc::new(b);
            let rc_pan = Rc::new(pan);
            let rc_grades = Rc::new(grades); // #885：与 bsp/pan_div 同批缓存（同 key）。
            lc.cached_bsp = Rc::clone(&rc);
            lc.cached_pan_div = Rc::clone(&rc_pan); // Q4：与 bsp 同批缓存（同 key）。
            lc.cached_first_class_grades = Rc::clone(&rc_grades);
            lc.cached_bsp_key = Some(bsp_key);
            (rc, rc_pan, rc_grades)
        };

        if lc.cached_candidate_key != Some(bsp_key) {
            let (candidate_segments, candidate_anchors) =
                candidate_scan_inputs(is_l0, &l0.segments, &units);
            lc.cached_candidate_observations = cand_event::observations_for_level(
                level_idx as u32,
                &lc.centers,
                &moves,
                candidate_segments.as_ref(),
                &candidate_anchors,
                &close_src,
                &pan_div,
            );
            lc.cached_candidate_key = Some(bsp_key);
        }
        candidate_observations.extend(lc.cached_candidate_observations.iter().cloned());

        // 08：`Rc::clone`（O(1)）投影增量塔 centers 到 LevelState，替代全量 `centers.clone()`。
        let level_centers =
            stage_profile::time("08_levels_centers_clone", || Rc::clone(&lc.centers));
        // #110 投影层 stamping（增量塔 memo-miss/命中终装点同口径；机制位关 = None 零开销）。
        // T3 (#172) 并门：机制位转派生（π 入口 `admission::chain_driven_level_projection`
        // 唯一生产写入点，#168 裁定 3）——链活 ⟹ 层必载，链死不载。
        // T2 (#171)：三元锚供给（`l0.fractals`/`l0.merged_bars`，ParseLayer `Rc` 共享只读
        // 借用，零拷贝）随门开分支引入——门关分支零新增读。
        let level_projection = if config.level_projection.enabled {
            Some(projection::LevelProjectionLayer::from_level(
                levels.len() as u32,
                &bsp,
                &l0.fractals,
                &l0.merged_bars,
            ))
        } else {
            None
        };
        levels.push(LevelState {
            moves,
            centers: level_centers,
            cp_ownership: Rc::clone(&lc.cp_ownership),
            bsp,
            pan_div,
            first_class_grades, // #885 S4-d（同 memo 批次，与 bsp/pan_div 锁步）
            level_projection,
        });

        // 下一级输入 = 上级走势塔投影（前缀来自缓存 upper_moves 前缀，尾部来自续扫）。
        // ★O(n²) 真修（#106）：增量投影——只对 upper_moves 新 tail 投影（`rmove.lo()/hi()` 递归整棵
        // 子树 O(nodes) 仅算新元素），前缀复用 lc.projected_units。clone 给 units 是 O(level) memcpy
        // （UnitRange: Copy，无递归）。cascade_reset 已清空 projected_units（line 855 旁）⟹ 退化全量。
        // ★A3 §2.5 证书化加固：pop 非 cascade 时 append-only 投影不感知 pop（保留上 bar frontier
        // 投影，隐式依赖 cascade 兜底）。显式 truncate 到 prefix_count 丢弃 pop 掉的 frontier 投影，
        // 从 prefix_count 起重投影（O(tail)）——消除 did_extend 类漏判。cascade 时 prefix_count=0，
        // 与 §上 projected_units.clear() 一致（truncate(0)==clear，冗余无害）。
        stage_profile::time("09_project_to_units_resume", || {
            // make_mut：下一级 units 由 stage 10 `Rc::clone` 共享同一 buffer；本 bar 头部 truncate 前
            // 上轮 units 已被 loop 尾/下 bar 重赋 drop ⟹ strong_count==1 ⟹ 原地 O(tail)；>1 ⟹ 写时
            // 复制（bit-exact 退化）。resume 契约要求 cache.len()<=moves.len()，truncate(prefix_count) 保证。
            let proj = Rc::make_mut(&mut lc.projected_units);
            proj.truncate(prefix_count);
            // Q7（task #145）：方向源 = 本级中枢 ownership 块方向（levels 尾 = 本级刚 push 的
            // LevelState.moves，与 batch 同一 decompose 单一来源）。confirmed 前缀方向冻结
            // （R(i-1,i) sealed 后标签不变），frontier 由 truncate(prefix_count) 每 bar 重投影。
            recursive_tower::project_to_units_resume(
                &lc.upper_moves[..],
                &levels.last().expect("本级 LevelState 已 push").moves,
                proj,
            );
        });
        // ★A3 投影证书护栏（debug/test）：truncate(prefix_count)+resume 必逐字段等全量 project_to_units。
        debug_assert!(
            *lc.projected_units
                == recursive_tower::project_to_units(
                    &lc.upper_moves,
                    &levels.last().expect("本级 LevelState 已 push").moves
                ),
            "投影证书违反：projected_units != 全量 project_to_units（truncate(prefix_count)/resume 破裂）"
        );
        // 10：`Rc::clone`（O(1)）替代全量 `projected_units.clone()`（O(units_L)/bar）。下一级借
        // &units[..] 只读；stage 09 头部 make_mut 时本 Rc 已 drop（loop 尾重赋）⟹ 原地写不退化。
        units = stage_profile::time("10_projected_units_clone", || {
            Rc::clone(&lc.projected_units)
        });
        // Q7-#1 裁定C：锚资格与投影同源同步派生（deterministic 于 (moves, len) ⟹ resume bit-exact：
        // 全量与增量在同一 (pb, units.len()) 上得同一锚数组，无缓存陈旧面）。
        units_anchors = {
            let pb = &levels.last().expect("本级 LevelState 已 push").moves;
            (0..units.len())
                .map(|i| decompose::center_own_dir_at(pb, i))
                .collect()
        };

        // ★A3 §2.4：本级 prefix_count 是下一级 units 的不可变前缀（父 confirmed 前缀投影稳定）。
        dirty_from = prefix_count;

        if units.is_empty() {
            // ★§2.6：level_idx 已处理完（尾部 break），level_idx+1 及更高级本 bar 因空 units 全跳过——
            // truncate 掉这段不可达尾巴的陈旧 LevelCache（血缘自洽，恢复时全扫重建）。
            #[cfg(test)]
            oracle_probe::on_empty_break(level_idx + 1 < cache.levels.len());
            cache.levels.truncate(level_idx + 1);
            break;
        }
        // ★O(n) 重构：moves_tower = Rc::clone(&lc.upper_moves) —— O(1) 引用计数，消除 per-bar 全塔
        // 深拷贝（旧 `lc.upper_moves.clone()` 是 O(n²) 热点①根因）。下一级 line 832 借 &moves_tower[..]
        // 只读，line 854 move 入 snapshots。lc.upper_moves 跨 bar 持久于 cache；extend（line 843）经
        // make_mut，caller 逐 bar drop snapshot ⟹ strong_count==1 ⟹ 原地 O(tail)。
        // bit-exact：Rc 指向同一 Vec，逐字段与旧 clone 等价。
        moves_tower = Rc::clone(&lc.upper_moves);
    }

    // ★#543 D1a 产出点⑥b 收尾：整塔全清后本 bar 未被重扫触达的级别（`min_parts`/空 units 提前
    // break，或 l_max 收缩）——其旧输出没有任何新对象与之配对，逐级补一条 removed-only 证书。
    // 不补就会留下「旧身份消失但无变换边」的黑洞（正是本 seam 要消灭的东西）。
    if txn_on {
        for (level_idx, dropped) in txn_cleared.iter_mut().enumerate() {
            if dropped.is_empty() {
                continue;
            }
            let _ = rebase_txn::emit(
                rebase_txn::TxnContext {
                    bar: txn_bar,
                    level: level_idx,
                    cause: "cache_clear",
                    cascade_reason: "level_not_rescanned",
                    dirty_e: 0,
                    resume_start: 0,
                    prefix_count: 0,
                },
                std::mem::take(dropped),
                Vec::new(),
                None,
            );
        }
    }

    // closes/close_src 缓冲放回 cache（mem::take 取出的所有权归还，下 bar 复用，零额外分配）。
    cache.closes = closes;
    cache.close_src = close_src;
    cache.closes_tick = closes_tick; // force 价格振幅/速度 proxy 缓冲放回，下 bar 复用（同 closes 模式）。
    cache.area_cache = area_cache.into_inner(); // B3 #4 area-memo：缓冲放回，下 bar 复用（同上模式）。
    let as_of = l0.merged_bars.last().map_or(0, |bar| bar.source_index);
    cache.candidate_book.advance(&candidate_observations, as_of);

    // ★工位 4g：本 bar 若有 cascade 重扫或任一级 extend 非空 tail ⟹ extract_elements 可观察树变更 ⟹
    // +generation（下游 TreeCache 据此 O(1) 跳过 TreeKey::of）。无变化 bar（~99.8%）generation 不变 ⟹
    // 命中。soundness：保守过度计数（低级 extend 不动最高级输出时多算一次 TreeKey）安全，绝不假命中。
    //
    // ★L0-root 边界（codex Q3 漏洞 + gen_fastpath_bit_exact_debug bar 345 坐实）：当最高非空级是 **L0**
    // （tower 无 compose 级，extract 直接读 `moves_tower_l0`），L0 走势塔每 bar 从 `l0.segments` 重建
    // （非缓存 Rc），其增长/同 len 古怪线段重划**不经** cascade/did_extend（那两者只覆盖各级 upper_moves）。
    // 故 L0-root 阶段**强制每 bar +generation**（走 TreeKey fallback）——此阶段 tree 极小（早期 bar），
    // TreeKey O(small) 不影响大 n 标度。一旦 L1+ 出现（extract 读 L1），L0 任何变化必经 cascade 传播至
    // L1（codex Q1：L0 frontier 改写→L1 units 投影变→L1 frontier_mutated→cascade），被完整捕获。
    let highest_nonempty = tower_snapshots.iter().rposition(|s| !s.is_empty());
    let l0_is_root = highest_nonempty == Some(0);
    if cascade_reset || did_extend || l0_is_root {
        cache.generation += 1;
    }

    // ★on2w2 forest_epoch：E1-E3 折叠（forest_dirty，含 E1 L0 塔重建 / E2 upper extend / E2a frontier
    // pop / E3 cascade clear）循环后统一 bump。E4（clear()）不经此（方法内已 bump）。与 generation 的
    // 关键区别：generation 靠 l0_is_root 每-bar blunt 兜底（bump 率 98.5%）；forest_epoch 只在塔实际字节
    // 变更时 bump（E1 直接捕获 L0 变更，无需 blunt 兜底）⟹ bump 率贴近 forest 真变率 ≈0.8%（on2w2 §2）。
    // over-invalidate：写入站点无条件 dirty，宁可多失效不可假命中（假命中不可能性证明 on2w2 §4）。
    if forest_dirty {
        cache.forest_epoch += 1;
    }
    #[cfg(test)]
    oracle_probe::on_forest_dirty(
        forest_dirty_l0,
        forest_dirty && !forest_dirty_l0 && !cascade_reset,
        cascade_reset,
    );

    debug_assert_l0_units_in_sync(cache, &tower_snapshots);
    (Classification { levels }, tower_snapshots)
}

/// ★#613（#609 F2，#712 收 #645 MED-1 随入）：[`TowerCache::level_scan_units`]（level=1，即
/// L0 units）只读契约的不变式——**塔已产出 L0 级快照时**，`l0_units_cache` 与 `tower[0]` 同长
/// （同源同序由 `moves_tower_l0` 的构造保证：两者都是 `l0.segments` 的逐元素纯函数投影，且
/// `l0_units_cache` 只有一处写入站点，见 `classify_with_tower_incremental` 内 `00_l0_units_build`）。
///
/// 限定「非空」是结构事实而非放宽：段数不足以构造任何级别时 `tower_snapshots` 为空而
/// `l0_units_cache` 已有内容，此时消费方（`p123_fast_replay`）走 `tower_level_absent` 显式
/// 原因码，根本不读 `l0_units` ⟹ 无契约面。
fn debug_assert_l0_units_in_sync(cache: &TowerCache, tower_snapshots: &[Rc<Vec<LeveledMove>>]) {
    debug_assert!(
        tower_snapshots
            .first()
            .is_none_or(|l0_tower| l0_tower.len() == cache.l0_units_cache.len()),
        "level_scan_units(1) 契约违反：l0_units_cache.len()={} 与 tower[0].len()={:?} 失步（#613/#609 F2）",
        cache.l0_units_cache.len(),
        tower_snapshots.first().map(|t| t.len())
    );
}

/// [`classify_with_tower_incremental`] 的 #550 三元事件通道。
pub fn classify_with_tower_events_incremental(
    l0: &ParseLayer,
    config: &ThetaConfig,
    cache: &mut TowerCache,
) -> (
    Classification,
    Vec<Rc<Vec<LeveledMove>>>,
    cand_event::CandidateStreams,
) {
    let (classification, tower) = classify_with_tower_incremental(l0, config, cache);
    (classification, tower, cache.candidate_book.streams())
}

/// 递归组装层第二类提取（对一级的每个上级走势 `RMove::Compose` 产 B2/S2）。
///
/// ★#53 接入点（still-MISSING-塔解除）：对本级每个上级走势 `LeveledMove`（`RMove::Compose`），
/// 双侧（Long/Short）调 `signal::extract_second_signals`——从 `descend parent` 取回的次级别走势
/// 序列内识别第二类走势结构（第一类离开 + 回拉不创新低/新高，§10.2 买卖点定律一）。产出的 B2/S2
/// 端点零改动接入生产路径。
///
/// 三个闭包参数的真实接入（非占位）：
/// - `c1`（次级别中枢）：`RMove::Compose.centers` 的首个中枢（窗口三段区间重叠真派生，B 口径核心
///   区间）——`find_second_type_structure` 用它判次级别第一类离开是否破中枢。
/// - `divergence_of`（MACD 背驰）：次级别走势的 source_index 区间 → `hist` 面积，相对**前一同向次
///   级别走势**面积严格变小（reference:34 背驰，`divergence.rs` 真算 L1）。
/// - `index_of`（坐标）：从坐标侧车 `subs`（携 source_index 的 `LeveledMove`）按结构身份查回原始
///   K 序（`index_of_in`）——B2/S2 的 `source_index` 真坐标（still-MISSING-坐标解除）。
///
/// ★诚实 still-MISSING（背驰力度引擎配对，no-声明膨胀）：`divergence_of` 对次级别走势的「前一同向
/// 走势」配对用**序列序最近同向前驱**（与 signal.rs `extract_first_for_center` 同口径）——次级别
/// 走势的 close 区间由 source_index 坐标定位到 `hist`。L1 管线正确性（MACD 面积比较确定），**不**是
/// 「背驰预测在真实行情有效」（L2/L3，不在本层）。
fn extract_second_for_level(
    upper_moves: &[LeveledMove],
    hist: &[f64],
    close_src: &[usize],
) -> Vec<BspPoint> {
    let mut points = Vec::new();
    // 单次全量调用：无跨 bar 复用需求，本地缓存仅消同一调用内的重复 (start,end)（若有），
    // `stable_len=hist.len()` 视全 hist 为稳定（一次性快照，调用期间不会被改写）。
    let area_cache = RefCell::new(AreaCache::new());
    let stable_len = hist.len();
    for parent in upper_moves {
        second_for_parent(
            parent,
            hist,
            close_src,
            &area_cache,
            stable_len,
            &mut points,
        );
    }
    points
}

/// 单个上级走势 `parent` 的第二类 B2/S2 端点（[`extract_second_for_level`] 的 per-parent 主体）。
///
/// ★纯函数于 `parent`：只依赖 `parent`（`c1`=Compose 首中枢、subs=坐标侧车）+ 全局 hist/close_src，
/// **不依赖其它 parent**。这是 07b frontier 门控（[`extract_second_resume`]）的正确性基础——confirmed
/// 前缀 parent 的 B2 跨 bar 不变（源区间落稳定前缀，hist 前缀 append-only 稳定）⟹ 可缓存。
fn second_for_parent(
    parent: &LeveledMove,
    hist: &[f64],
    close_src: &[usize],
    area_cache: &RefCell<AreaCache>,
    stable_len: usize,
    out: &mut Vec<BspPoint>,
) {
    // 次级别中枢（RMove::Compose.centers 首个，窗口真派生 B 口径核心区间）。
    let c1 = match &parent.rmove {
        descend::RMove::Compose { centers, .. } => match centers.first() {
            Some(c) => *c,
            None => return, // 无中枢载荷 ⟹ 跳过（compose_level 必带中枢，防御性）。
        },
        // L0 线段（递归底）不会出现在 upper_moves（compose_level 只产 Compose），防御性跳过。
        descend::RMove::Segment { .. } => return,
    };
    // 坐标侧车：构成 parent 的次级别 LeveledMove 序列（与 descend parent 同序同长）。
    let subs = descend_leveled(parent);
    // 双侧识别第二类结构（B2=Long / S2=Short），各产至多一个端点。
    for side in [Side::Long, Side::Short] {
        let pts = signal::extract_second_signals(
            &parent.rmove,
            side,
            &c1,
            // 背驰：次级别走势 source_index 区间 → hist 面积，相对前一同向次级别走势严格变小。
            |m| sublevel_diverges(m, &subs[..], hist, close_src, area_cache, stable_len),
            // 坐标：从侧车按结构身份查回次级别走势的原始 K 序（end_index）。
            |m| index_of_in(&subs[..], m),
        );
        out.extend(pts);
    }
}

/// 07b frontier 门控（A 泳道 resume 家族，与 A3/07c 同族）：增量热路径的 [`extract_second_for_level`]
/// 每 memo-miss 全塔重扫 O(U)、跨 N bar 累积 O(U²)（profile 坐实 CL 1M 修前 3990ms、修后 1026ms，74%↓）。
/// 每个 parent 的 B2 只依赖该 parent（[`second_for_parent`] 纯函数于 parent）——confirmed 前缀 parent
/// （`upper_moves[..prefix_count]`，源区间落稳定前缀 + hist 前缀 append-only 稳定）的 B2 跨 bar 不变，
/// 可缓存。故**跳过 confirmed 前缀 parent 的重复背驰扫描**（`sublevel_diverges` 的 O(range) MACD 面积
/// 累加），只对 frontier tail `[prefix_count..]` 每 bar 重算，前缀 B2 一生一算。
///
/// ★area-memo 已接入（`d516aa42ea`，[`cached_segment_area`]）：MACD 面积经 `(start,end)→f64` 冻结
/// 缓存，`sublevel_diverges` 内 `07b_area` 实测 ~46ns/call（10.2M call@BTC-1M）= cache-hit 主导，
/// 面积累加已非瓶颈。门控（本函数）+ area-memo 两处均已落地。
///
/// ★残余 O(n²) 根因订正（on2-sweep 直测坐实，BTC-1M）：**不是** frontier parent 的 area 累加，而是
/// **cascade 触发的前缀重扫**。07b miss 的 frontier tail 分裂两支——纯 append miss（`07b_miss_append_tail`
/// sum=28570，avg=2.87，门控生效尾极短）vs cascade miss（`07b_miss_cascade_tail` sum=2015978=98.6%，
/// avg=97.5，max=2095）。cascade_reset 定义性清空 `cached_second`（前缀 B2 缓存失效，见 line ~1192），
/// 整个前缀被重扫 ⟹ cascade 频率 × 前缀长度 = O(n²)。cascade 频率由 `recursive_tower` 域的 frontier
/// 重排决定（H5/H9 已 NO-SHIP：cascade 定义性清前缀，不可在本文件域降阶），非 `second_for_parent`/
/// `sublevel_diverges` 的可优化项。per-call 常数因子（`position` 递归 eq ~14%、
/// [`cand_predicate::rmove_dir`] 的方向派生 ~15%）均 subs.len()≤8 有界，改写只削常数不改指数
/// （收益极低，不 ship）。
///
/// ★bit-exact 铁律：返回值逐字段 == [`extract_second_for_level`]（parent 序拼接：前缀 B2 + tail B2
/// = 全 parent 序，与全量重扫同序同集）。debug/test 护栏逐 bar 对拍全量重算锁定。
///
/// 前提（`prefix_count` = confirmed 前缀数，由主循环 pop 后 `lc.upper_moves.len()` 给出）：
/// `upper_moves[..prefix_count]` 跨 bar immutable（anc.pdf §16），故其 B2 一旦算出即永久稳定。
fn extract_second_resume(
    cached: &mut Vec<BspPoint>,
    cached_count: &mut usize,
    upper_moves: &[LeveledMove],
    prefix_count: usize,
    hist: &[f64],
    close_src: &[usize],
    area_cache: &RefCell<AreaCache>,
    stable_len: usize,
) -> Vec<BspPoint> {
    // 单调性守卫（§16：confirmed 前缀单调非降 ⟹ 正常永不触发；cascade 已在别处 clear 缓存）。若违反
    // ⟹ 缓存越过 confirmed 边界（曾判 confirmed 的 parent 又变 frontier 可变）⟹ 保守全量重算重置缓存。
    if *cached_count > prefix_count {
        cached.clear();
        *cached_count = 0;
    }
    // 推进：新晋 confirmed 的 parent [cached_count..prefix_count] 的 B2 一次性算入缓存（一生一算）。
    for parent in &upper_moves[*cached_count..prefix_count] {
        second_for_parent(parent, hist, close_src, area_cache, stable_len, cached);
    }
    *cached_count = prefix_count;
    // 结果 = confirmed 前缀 B2（缓存 clone）+ frontier tail B2（每 bar 重算，tail 小）。
    let mut out = cached.clone();
    for parent in &upper_moves[prefix_count..] {
        second_for_parent(parent, hist, close_src, area_cache, stable_len, &mut out);
    }
    debug_assert!(
        out == extract_second_for_level(upper_moves, hist, close_src),
        "07b frontier 门控破裂：门控输出 != 全量重扫（前缀 immutable/hist 前缀稳定不变式被违反）"
    );
    out
}

/// 次级别走势的 MACD 背驰判定（reference:34，`divergence.rs` 真算 L1）。
///
/// 给定次级别走势 `m`（descend 取回的 `RMove`）+ 坐标侧车 `subs`：定位 `m` 在 `subs` 中的位置，
/// 取其 source_index 区间 → `hist` 面积，相对**序列序最近同向次级别前驱走势**面积严格变小 ⟹ 背驰。
/// 同向 = [`cand_predicate::rmove_dir`] 判得且方向相同；方向不可判 ⟹ 无合法背驰对照。
///
/// ★诚实 still-MISSING（背驰力度引擎）：无前同向走势（`m` 是序列首个该向走势）⟹ 无背驰对照
/// ⟹ false（与 signal.rs `extract_first_for_center` 同口径——第一类是趋势末段必有前同向段）。
/// 无法定位 source_index 区间到 `hist`（坐标越界）⟹ false（不冒充背驰）。
fn sublevel_diverges(
    m: &descend::RMove,
    subs: &[LeveledMove],
    hist: &[f64],
    close_src: &[usize],
    area_cache: &RefCell<AreaCache>,
    stable_len: usize,
) -> bool {
    // 定位 m 在 subs 中的位置（结构身份匹配）。
    let Some(idx) = subs.iter().position(|x| &x.rmove == m) else {
        return false; // m 不在 subs（防御性）⟹ 无坐标 ⟹ 非背驰。
    };
    let curr = &subs[idx];
    let Some(curr_dir) = cand_predicate::rmove_dir(&curr.rmove) else {
        return false;
    };
    // 序列序最近同向前驱走势（reference:34「末段相对前同向段」的确定配对）。
    let Some(prev) = subs[..idx]
        .iter()
        .rev()
        .find(|x| cand_predicate::rmove_dir(&x.rmove) == Some(curr_dir))
    else {
        return false; // 无前同向走势 ⟹ 无背驰对照 ⟹ 非第一类（趋势末段必有前同向段）。
    };
    // 两走势 source_index 区间 → hist 面积比较（curr < prev ⟹ 背驰，divergence.rs 真算）。
    let (Some(curr_seg), Some(prev_seg)) = (
        map_src_to_close_idx(close_src, curr.start_index, curr.end_index),
        map_src_to_close_idx(close_src, prev.start_index, prev.end_index),
    ) else {
        return false; // 区间越界/空 ⟹ 无面积 ⟹ 不冒充背驰。
    };
    // B3 #4 area-memo：面积经 (start,end)→f64 冻结缓存（[`cached_segment_area`]），逐字段
    // == `divergence::segments_diverge`（同一 `segment_macd_area` 结果，仅省重复求和）。
    let prev_area = cached_segment_area(area_cache, hist, stable_len, prev_seg.0, prev_seg.1);
    let curr_area = cached_segment_area(area_cache, hist, stable_len, curr_seg.0, curr_seg.1);
    divergence::is_divergence(prev_area, curr_area)
}

/// B3 #4 area-memo：`(start,end)`→`segment_macd_area` 冻结缓存查询/写入（[`AreaCache`] 文档）。
///
/// 只有 `end < stable_len`（区间落在本 bar 已确认的 hist 前缀内）才读写缓存——`end >= stable_len`
/// 触及仍可能被下一 bar 改写的 unstable tail（`compute_macd_hist_incremental` 每 bar 末元素语义），
/// 缓存这类值会在下 bar 值变化后返回过期错值，故每次现算、绝不写入缓存（bit-exact 铁律）。
///
/// bit-exact：返回值逐位 == `divergence::segment_macd_area(hist, start, end)`——本函数只做**结果**
/// 记忆化（首次算出后原样存取），不做前缀和差分（B3 报告明确禁止：浮点求和顺序改变可能破 bit-exact）。
fn cached_segment_area(
    cache: &RefCell<AreaCache>,
    hist: &[f64],
    stable_len: usize,
    start: usize,
    end: usize,
) -> f64 {
    if end < stable_len {
        if let Some(&area) = cache.borrow().get(&(start, end)) {
            return area;
        }
        let area = divergence::segment_macd_area(hist, start, end);
        cache.borrow_mut().insert((start, end), area);
        area
    } else {
        divergence::segment_macd_area(hist, start, end)
    }
}

#[cfg(test)]
mod tests {
    use super::super::parser::ParseLayer;
    use super::super::types::Direction;
    use super::super::types::MoveKind;
    use super::*;

    #[test]
    fn p52_frontier_diagnostics_are_level_scoped_and_resettable() {
        cp_replay_diagnostics::enable();
        cp_replay_diagnostics::record_dirty_invalidation(2, 3, 5);
        cp_replay_diagnostics::record_tail_reinherit(2);
        cp_replay_diagnostics::record_tail_reinherit(1);

        let counters = cp_replay_diagnostics::snapshot();
        assert_eq!(counters[1].tail_reinherits, 1);
        assert_eq!(counters[2].pending_fallbacks, 3);
        assert_eq!(counters[2].tail_reinherits, 1);
        assert_eq!(counters[2].certificate_clear_recomputes, 5);

        cp_replay_diagnostics::disable();
        assert!(cp_replay_diagnostics::snapshot().is_empty());
    }

    fn seg(dir: Direction, si: usize, ei: usize, sp: i64, ep: i64) -> Segment {
        Segment {
            direction: dir,
            start_index: si,
            end_index: ei,
            start_price: sp,
            end_price: ep,
        }
    }

    /// 构造 merged_bars：source_index 连续 0..n，close = vals（MACD 背驰真算用）。
    fn bars_from_closes(vals: &[i64]) -> Vec<super::super::types::Bar> {
        vals.iter()
            .enumerate()
            .map(|(i, &v)| super::super::types::Bar {
                source_index: i,
                timestamp: i as i64,
                open: v,
                high: v,
                low: v,
                close: v,
                volume: 1,
                untradable: false,
            })
            .collect()
    }

    fn cache_candidate(c_start: usize, end: usize) -> cand_event::CandidateObservation {
        let key = cand_event::CandidateKey {
            rule_version: cand_event::CANDIDATE_RULE_VERSION,
            level: 0,
            kind: cand_event::CandidateKind::Trend,
            side: Side::Long,
            previous_center_start: Some(10),
            parent: cand_event::ParentFingerprint {
                center_start: 20,
                zd: 100,
                zg: 110,
            },
            seg_a: (11, 19),
            c_start,
        };
        cand_event::CandidateObservation {
            key,
            kind: cand_event::CandidateKind::Trend,
            center_ids: Some((10, 20)),
            candidate_group_id: c_start as u64,
            pair_id: (c_start as u64) + 1,
            structural_predicates: cand_event::StructuralPredicates {
                direction: true,
                comparable: true,
                extreme: true,
            },
            extreme_proof: key.seg_a,
            third_class_proof: None,
            interval: (c_start, end),
            state: cand_event::ObservedState::Provisional,
            first_provable_at: Some(end),
            confirmed_at: None,
        }
    }

    #[test]
    fn tower_cache_clear_preserves_candidate_history_and_resets_derived_cache() {
        let mut cache = TowerCache::new();
        let removed = cache_candidate(30, 35);
        let retained = cache_candidate(40, 45);
        cache
            .candidate_book
            .advance(&[removed.clone(), retained.clone()], 50);
        cache.last_l0_segments_len = 9;
        cache.macd_hist.push(1.0);

        cache.clear();
        assert_eq!(cache.last_l0_segments_len, 0);
        assert!(cache.macd_hist.is_empty());

        let mut grown = retained.clone();
        grown.interval.1 = 46;
        let delta = cache.candidate_book.advance(&[grown], 60);
        let invalidated = delta.iter().find(|event| event.key == removed.key).unwrap();
        let revised = delta
            .iter()
            .find(|event| event.key == retained.key)
            .unwrap();
        assert_eq!(invalidated.state, cand_event::CandidateState::Invalidated);
        assert_eq!(invalidated.revision, 1);
        assert_eq!(revised.revision, 1);
        assert_eq!(revised.observed_at, 50);
    }

    /// ★批1（force_state 生产热路由 step4）：TowerCache 的 dif 增量通路 bit-exact 对拍全量。
    ///
    /// 严格路线（Lead 裁定，拒绝「增量恒 None」降级）：`compute_macd_hist_incremental` 逐 bar 产出的
    /// `macd_dif` 必与全量 `compute_macd(&closes[..k]).dif` **逐位相等**（非 tolerance——bit-exact 是
    /// 断言，不是近似；tolerance 会把非 bit-exact 藏进容差 = 声明膨胀）。同证 `macd_hist`（dif/hist
    /// 锁步的锚），并证 `closes_tick == merged_bars.close`（force 价格振幅 proxy 输入的整数往返）。
    ///
    /// 覆盖：resume 增量路径（append-only 前缀稳定，confirmed_len=k-1）逐 bar 生长——每步驱动
    /// truncate+append+tail 三段（含单 bar 分支 k=1）。dif 是 hist 子表达式（hist=dif-dea），hist
    /// 增量已证 bit-exact（tower 测试锁 BspPoint）⟹ dif 同证；本测试直接坐实 dif 数组本身。
    #[test]
    fn incremental_macd_dif_and_closes_tick_bit_exact() {
        let cfg = super::super::config::MacdConfig::default();
        // 合成 closes：上升 + 震荡 + 下降（EMA 充分递推，覆盖 dif 正负峰）。整值 ⟹ closes_tick 往返精确。
        let vals: Vec<i64> = (0..90)
            .map(|i| 1000 + (30.0 * ((i as f64) * 0.3).sin()) as i64 + i as i64)
            .collect();
        let closes: Vec<f64> = vals.iter().map(|&v| v as f64).collect();

        // ── dif/hist 增量 vs 全量（逐 bar 生长，resume 增量路径）──
        let mut cache = TowerCache::new();
        for k in 1..=closes.len() {
            let prefix = &closes[..k];
            let confirmed = k.saturating_sub(1); // append-only：前 k-1 稳定，尾 bar 不稳定。
            compute_macd_hist_incremental(prefix, confirmed, &cfg, &mut cache);
            let full = divergence::compute_macd(prefix, &cfg);
            assert_eq!(
                cache.macd_dif(),
                full.dif.as_slice(),
                "bar {k}: dif 增量 ≠ 全量（bit-exact 破）"
            );
            assert_eq!(
                cache.macd_hist_for_test(),
                full.hist.as_slice(),
                "bar {k}: hist 增量 ≠ 全量"
            );
            assert_eq!(
                cache.macd_dif().len(),
                cache.macd_hist_for_test().len(),
                "dif/hist 锁步等长"
            );
        }

        // ── closes_tick 增量 == merged_bars.close（整数域，force 振幅/速度 proxy 输入）──
        let bars = bars_from_closes(&vals);
        let mut cache2 = TowerCache::new();
        for k in 1..=bars.len() {
            update_closes_cache(&bars[..k], k.saturating_sub(1), &mut cache2);
            let expect: Vec<Tick> = bars[..k].iter().map(|b| b.close).collect();
            assert_eq!(
                cache2.closes_tick(),
                expect.as_slice(),
                "bar {k}: closes_tick ≠ merged_bars.close"
            );
        }
    }

    /// ★#613（收 #609 F2，#712 收 #645 MED-1 随入）：段账本回缩 bar 上 `level_scan_units(1)`
    /// （L0 units 只读契约）与 `tower[0]` 必须仍同长。
    ///
    /// 回归的是这条真 bug：回缩检测的 `cache.clear()` 曾位于 `l0_units_cache` 构建**之后**，
    /// 于是该 bar 上访问器返回空而 `tower[0]` 满载。消费方（`p123_fast_replay` 的 L2 活窗派生）
    /// 拿空切片重扫，只在 `resume_from > 0` 时被越界守卫恰好接住；`resume_from == 0` 时会静默
    /// 落 `no_window_formed`。
    #[test]
    fn l0_units_stays_in_sync_with_tower0_on_segment_ledger_shrink() {
        let cfg = ThetaConfig::default();
        let mut cache = TowerCache::new();
        let (long_layer, short_layer) = segment_ledger_shrink_fixture();

        let (_, tower_long) = classify_with_tower_incremental(&long_layer, &cfg, &mut cache);
        assert_eq!(
            cache.level_scan_units(1).map(|u| u.len()),
            Some(tower_long[0].len()),
            "非回缩 bar 本就同长"
        );

        let (_, tower_short) = classify_with_tower_incremental(&short_layer, &cfg, &mut cache);

        assert!(!tower_short.is_empty(), "4 段仍足以产出 L0 塔快照");
        assert_eq!(
            tower_short[0].len(),
            4,
            "回缩后 tower[0] = 新段账本全量重建"
        );
        assert_eq!(
            cache.level_scan_units(1).map(|u| u.len()),
            Some(tower_short[0].len()),
            "★F2 不变式：回缩 bar 上 level_scan_units(1) 不得为空/失步（#613 收 #609 F2）"
        );
        // 同长之外再钉同源：逐元素等于新段账本的 `segment_to_unit` 投影。
        let expected: Vec<UnitRange> = short_layer.segments.iter().map(segment_to_unit).collect();
        assert_eq!(
            cache.level_scan_units(1),
            Some(expected.as_slice()),
            "同序同源，非仅同长"
        );
    }

    /// [`l0_units_stays_in_sync_with_tower0_on_segment_ledger_shrink`] 的两 bar 夹具。
    ///
    /// - 第一 bar：6 段账本（confirmed 前缀 5，末段未确认——古怪线段可重划）；
    /// - 第二 bar：段账本**回缩**到 4 段（末两段被重划吞并）⟹ 走 `cache.clear()` 分支。
    fn segment_ledger_shrink_fixture() -> (ParseLayer, ParseLayer) {
        let long_segments = vec![
            seg(Direction::Up, 0, 4, 100, 150),
            seg(Direction::Down, 4, 8, 150, 120),
            seg(Direction::Up, 8, 12, 120, 148),
            seg(Direction::Down, 12, 16, 148, 110),
            seg(Direction::Up, 16, 20, 110, 145),
            seg(Direction::Down, 20, 24, 145, 115),
        ];
        let closes: Vec<i64> = (0..28)
            .map(|i| 100 + if i % 2 == 0 { 20 } else { -20 })
            .collect();
        let long_layer = ParseLayer {
            segments: Rc::new(long_segments.clone()),
            segments_confirmed_len: 5,
            merged_bars: Rc::new(bars_from_closes(&closes)),
            ..Default::default()
        };
        let short_layer = ParseLayer {
            segments: Rc::new(long_segments[..4].to_vec()),
            segments_confirmed_len: 3,
            merged_bars: Rc::new(bars_from_closes(&closes)),
            ..Default::default()
        };
        (long_layer, short_layer)
    }

    /// parser BUG-04 回归：缓存守卫按真实覆盖契约校验——`compute_macd_hist_incremental`
    /// 在 n≥2 时 `macd_state_len = n-1`（state 只覆盖稳定前缀，hist/dif 才含不稳定尾 bar），
    /// 旧守卫 `== n` 恒假 ⟹ 增量缓存死代码、每 bar 退化全量 O(n²)。修复后逐 bar 驱动
    /// 生产同源更新（update_closes_cache + compute_macd_hist_incremental），守卫必须命中；
    /// over-invalidate 方向保持（空/不齐 cache 必不命中）。
    #[test]
    fn cand_cache_guard_accepts_incremental_contract() {
        let cfg = super::super::config::MacdConfig::default();
        let vals: Vec<i64> = (0..40).map(|i| 1000 + (i as i64 * 3) % 17).collect();
        let closes: Vec<f64> = vals.iter().map(|&v| v as f64).collect();
        let bars = bars_from_closes(&vals);
        let mut cache = TowerCache::new();
        for k in 1..=bars.len() {
            update_closes_cache(&bars[..k], k.saturating_sub(1), &mut cache);
            compute_macd_hist_incremental(&closes[..k], k.saturating_sub(1), &cfg, &mut cache);
            assert!(
                cache_series_ok(&cache, k),
                "bar {k}: 生产同源增量更新后守卫必须命中（BUG-04：旧 ==n 守卫在 k≥2 恒假）"
            );
        }
        // over-invalidate 方向保持：空 cache 对非空序列必不命中（退化全量，bit-exact）。
        assert!(
            !cache_series_ok(&TowerCache::new(), 5),
            "空 cache 必不命中（守卫仍 over-invalidate）"
        );
        assert!(
            cache_series_ok(&TowerCache::new(), 0),
            "n=0：空 cache 与空序列自洽（与旧守卫同界）"
        );
    }

    /// ★#543 D1a：构造证书 seam 的**生产放置点**端到端见证（不是纯函数单测——真走
    /// `classify_with_tower_incremental` 逐 bar 增量路径）。
    ///
    /// 覆盖三个放置点：
    /// 1. 常态 frontier pop（`had_emitted_window` 分支，产出点①）+ 新 tail 返回/自然 emit（②③）；
    /// 2. 一窗产 k 个子对象（`WinMeta.emitted`，产出点⑤——若本输入触发九段升级则 emitted>1）；
    /// 3. cascade 后缀失效（产出点④）——用 **input len shrink**（同 cache 喂更短前缀）强制
    ///    `units.len() < lc.last_input_len` ⟹ `e=0` ⟹ `P=0` 全清分支，旧后缀必须在 clear 前被捕获。
    ///
    /// 断言只落在「证书结构自洽 + 放置点确实被触达」上；不断言具体 relation 分布（那取决于合成
    /// 输入的几何，属现场事实，由 wf8 实测报告登记）。
    #[test]
    fn rebase_txn_seam_emits_certificates_at_production_placement_points() {
        let cfg = super::super::config::ThetaConfig::default();
        // 合成 closes：多频叠加锯齿（保证足够多分型/笔/线段 ⟹ L1 中枢 + frontier 每 bar 重算）。
        let vals: Vec<i64> = (0..600)
            .map(|i| {
                let f = i as f64;
                1000 + (40.0 * (f * 0.35).sin() + 15.0 * (f * 0.11).cos() + 6.0 * (f * 1.7).sin())
                    as i64
            })
            .collect();
        let bars = bars_from_closes(&vals);

        rebase_txn::test_capture_start();
        let mut cache = TowerCache::new();
        for i in 5..=bars.len() {
            let l0 = super::super::parser::parse_layer(&bars[..i], &cfg);
            let _ = classify_with_tower_incremental(&l0, &cfg, &mut cache);
        }
        // len shrink：同一 cache 喂更短前缀 ⟹ cascade P=0 全清（产出点④）。
        let l0_short = super::super::parser::parse_layer(&bars[..300], &cfg);
        let _ = classify_with_tower_incremental(&l0_short, &cfg, &mut cache);
        let lines = rebase_txn::test_capture_take();

        assert!(
            !lines.is_empty(),
            "生产放置点一条构造证书都没产出（seam 未接通）"
        );
        let mut n_pop = 0usize;
        let mut n_cascade = 0usize;
        let mut n_continued = 0usize;
        let mut n_multi_emit = 0usize;
        for line in &lines {
            for k in [
                "\"schema\":\"rebase_transform_txn_v1\"",
                "\"txn_id\"",
                "\"bar\"",
                "\"level\"",
                "\"cause\"",
                "\"dirty_e\"",
                "\"resume_start\"",
                "\"prefix_count\"",
                "\"old_nodes\"",
                "\"new_nodes\"",
                "\"transform_edges\"",
                "\"lower_txn_id\"",
                "\"lower_edge_refs\"",
                "\"algorithm_version\"",
                "\"source_order_digest\"",
                "\"txn_digest\"",
            ] {
                assert!(line.contains(k), "证书缺字段组 {k}");
            }
            assert!(
                !line.contains("18446744073709551615"),
                "usize::MAX 哨兵须记 null"
            );
            if line.contains("\"cause\":\"frontier_pop\"") {
                n_pop += 1;
            }
            if line.contains("\"cause\":\"cascade_p0\"")
                || line.contains("\"cause\":\"cascade_prefix\"")
            {
                n_cascade += 1;
                assert!(
                    line.contains("\"side\":\"old\""),
                    "cascade 事务必须带被丢弃的旧后缀快照（产出点④的全部意义）"
                );
            }
            if line.contains("\"relation\":\"continued_1to1\"") {
                n_continued += 1;
            }
            if line.contains("\"emitted\":2") || line.contains("\"emitted\":3") {
                n_multi_emit += 1;
            }
        }
        eprintln!(
            "[#543 seam] txn={} frontier_pop={n_pop} cascade={n_cascade} \
             含 continued_1to1={n_continued} 一窗多产={n_multi_emit}",
            lines.len()
        );
        assert!(n_pop > 0, "常态 frontier pop 放置点未触达");
        assert!(
            n_cascade > 0,
            "cascade 后缀失效放置点未触达（len shrink 未走到 P=0 分支）"
        );
        assert!(
            n_continued > 0,
            "无一条连续边——同 seed 重扫本应产 continued_1to1"
        );
    }

    /// ★#543 D1a 负控：seam 未启用（无 env、无捕获）⟹ 逐 bar 增量塔的输出与启用时**逐字段相同**。
    /// 这是「行为零变化」红线的单测化（wf8 关臂 cmp=0 是同一命题的现场版）。
    #[test]
    fn rebase_txn_seam_does_not_change_classification_output() {
        let cfg = super::super::config::ThetaConfig::default();
        let vals: Vec<i64> = (0..600)
            .map(|i| {
                let f = i as f64;
                1000 + (40.0 * (f * 0.35).sin() + 15.0 * (f * 0.11).cos() + 6.0 * (f * 1.7).sin())
                    as i64
            })
            .collect();
        let bars = bars_from_closes(&vals);

        let run = |capture: bool| {
            // ★#679 D1b：seam 在生产判径默认常开，故关臂须显式按下反证开关，否则本负控
            // 两臂都是「开」，失去区分力。
            crate::theta_v0::lineage_book::test_set_consumer(Some(capture));
            if capture {
                rebase_txn::test_capture_start();
            }
            let mut cache = TowerCache::new();
            let mut out = Vec::new();
            for i in 5..=bars.len() {
                let l0 = super::super::parser::parse_layer(&bars[..i], &cfg);
                let (c, tower) = classify_with_tower_incremental(&l0, &cfg, &mut cache);
                out.push((c, tower));
            }
            let n = if capture {
                rebase_txn::test_capture_take().len()
            } else {
                0
            };
            crate::theta_v0::lineage_book::test_set_consumer(None);
            (out, n)
        };
        let (off, _) = run(false);
        let (on, emitted) = run(true);
        assert!(emitted > 0, "开臂须真产出证书，否则本负控无区分力");
        assert_eq!(off.len(), on.len());
        for (i, (a, b)) in off.iter().zip(on.iter()).enumerate() {
            assert_eq!(
                a.0, b.0,
                "bar {i}: Classification 被观测旁路改变（行为零变化红线破）"
            );
            assert_eq!(
                a.1, b.1,
                "bar {i}: tower 快照被观测旁路改变（行为零变化红线破）"
            );
        }
    }

    /// ★端到端 B2 真产出（#53 验证门，L1 管线正确性）：升级后的递归塔（`RMove::Compose` 携 subs）
    /// 让 `extract_second_signals` **真接入生产路径**——classify 在真实结构输入上产出 B2 买点。
    ///
    /// 旧塔（`UnitRange` 无 subs）在**任何**输入上产 0 个 B2（descend 得空，结构上不可产）；新塔
    /// 在此输入上产 1 个 B2，坐实升级解除了 still-MISSING-塔。
    ///
    /// 路径（codex 异质裁决确认）：B2 在 **L1→L2 几何路径**产出——3 个同向 L1 走势经几何窗口
    /// （不强制方向交替）compose 成 L2 走势，其 descend 取回的 3 个 L1 走势内识别第二类结构：
    /// L1[1]（i1=1）破 L2 中枢 + MACD 背驰（相对前同向 L1[0]）= 第一类离开；L1[2]（i2=2）回拉不
    /// 创新低 = 第二类回拉走势。B2 端点 = L1[2] 的回拉结束点（坐标由 source_index 侧车真映射）。
    ///
    /// ★诚实边界（still-MISSING-窗口，codex 裁决坐实）：B2/S2 **不在 L0→L1 三段交替窗口产**——
    /// 三段方向交替窗口里可背驰的同向段只在位置 2（末段，无后继回拉），位置 0 无前同向对照，
    /// 位置 1 是唯一异向（无前同向）。这是固定三段封装的结构上界，非接入缺陷（接入逻辑双侧完整）。
    #[test]
    fn end_to_end_second_buy_via_l1_l2_geometric() {
        let cfg = ThetaConfig::default();
        // 9 段 L0：三组（每组 → 一个 L1 走势）。★中枢延伸语义下的诚实重算（PDF §5，task #142）：
        // 组间首段必须与前组**冻结核心 [ZD,ZG]** 不相交（Step3 non-extension），否则整串被 Step2
        // 吸收为 1 个延伸中枢 ⟹ 塔不生长（旧全触及 fixture 的坍缩后果）。推导：
        // - 组A up-down-up：核心 K_A=[max(110,120,120),min(150,150,148)]=[120,148]，外缘 O_A=[110,150]。
        // - 组B down-up-down：首段 [80,115] hi=115 < ZD_A=120 ⟹ non-extension（组间分离）；
        //   核心 K_B=[max(80,80,85),min(115,125,114)]=[85,114]，外缘 O_B=[80,125]。
        // - 组C up-down-up：首段 [115,148] lo=115 > ZG_B=114 ⟹ non-extension；
        //   核心 K_C=[max(115,112,112),min(148,148,147)]=[115,147]，外缘 O_C=[112,148]。
        // L2 核心（几何路径，三 L1 外缘交）= [max(110,80,112), min(150,125,148)] = [112,125] 非空。
        // B2 结构：L1[1].lo=80 < ZD2=112 深破 L2 核心下沿（第一类离开候选，Side::Long）；
        // L1[2] 回拉不创新低（lo=112 >= L1[1].lo=80）；L1[0]/L1[1] 外缘占位方向同 Down
        // （末子 hi < 首子 hi：148<150 / 114<115）⟹ 背驰可配对（closes 前大后小）。
        let segments = vec![
            // 组A（L1[0]）：up-down-up，核心 [120,148]，外缘 [110,150]
            seg(Direction::Up, 0, 4, 110, 150),
            seg(Direction::Down, 4, 8, 150, 120),
            seg(Direction::Up, 8, 12, 120, 148),
            // 组B（L1[1]）：down-up-down，首段 hi=115<ZD_A=120 non-ext，lo=80 深破 L2 核心下沿 112
            seg(Direction::Down, 12, 16, 115, 80),
            seg(Direction::Up, 16, 20, 80, 125),
            seg(Direction::Down, 20, 24, 114, 85),
            // 组C（L1[2]）：up-down-up，首段 lo=115>ZG_B=114 non-ext，回拉不创新低（lo=112 >= 80）
            seg(Direction::Up, 24, 28, 115, 148),
            seg(Direction::Down, 28, 32, 148, 112),
            seg(Direction::Up, 32, 36, 112, 147),
        ];
        // closes 让 L1[1] 区间（source_index [12,24]）MACD 面积 < L1[0] 区间（[0,12]）= 背驰（真算）。
        // 前段大幅波动（面积大），后段小幅（面积小）。
        let mut closes: Vec<i64> = Vec::new();
        for i in 0..12 {
            closes.push(100 + if i % 2 == 0 { 40 } else { -40 });
        } // L1[0] 大幅
        for i in 0..12 {
            closes.push(100 + if i % 2 == 0 { 5 } else { -5 });
        } // L1[1] 小幅（背驰）
        for i in 0..16 {
            closes.push(100 + if i % 2 == 0 { 3 } else { -3 });
        } // L1[2] 更小
        let layer = ParseLayer {
            segments: Rc::new(segments),
            merged_bars: Rc::new(bars_from_closes(&closes)),
            ..Default::default()
        };
        let out = classify(&layer, &cfg);

        // L1 级别（索引 1）含 L2 中枢 + B2（递归组装层产出）。
        assert!(
            out.levels.len() >= 2,
            "三组 L0 → L1 走势塔 → L2 中枢，至少 2 级"
        );
        let l1 = &out.levels[1];
        assert_eq!(
            l1.centers.len(),
            1,
            "3 个 L1 走势 → 1 个 L2 中枢（几何路径）"
        );
        let second_buys: Vec<_> = l1.bsp.iter().filter(|p| p.bits.buy2).collect();
        assert_eq!(
            second_buys.len(),
            1,
            "★升级后塔真产 B2（旧 UnitRange 塔产 0）"
        );
        let b2 = second_buys[0];
        // B2 端点坐标由 source_index 侧车真映射（回拉走势 L1[2] 的 end_index=36）。
        assert_eq!(
            b2.source_index, 36,
            "B2 source_index = 回拉走势 L1[2] 的原始 K 序（坐标侧车真映射）"
        );
        // 第二类止损 = 回拉低点（second_point = 回拉走势 m2.lo）——止损仍 pivot 非 center.zg/zd。
        assert!(
            b2.pivot_low != 0,
            "B2 携结构止损价 pivot_low（回拉低点 single source）"
        );
        // ★#218 面 A（spec owner-attribution-fix-20260724 ID-1，机械改写归因：载体形态变化）：
        // 二类点归属载体从判定中枢 c1（次级别中枢，确认层对象）改载该走势一类点身份锚——
        // 第一类离开走势 m1（L1[1]，背驰次级别走势）的终点坐标（区间套：该走势终点极值点 =
        // 一类点）；止损仍 pivot（上条已锁，止损语义不变）。
        assert_eq!(
            b2.center,
            Some(signal::OwnerRef::Type1Anchor(24)),
            "二类点归属载体 = 该走势一类点锚（m1=L1[1] 终点坐标 24）"
        );
        // 互斥语义：B2 端点不置 1/3 类 bit。
        assert!(!b2.bits.buy1 && !b2.bits.buy3, "第二类端点不置 1/3 类 bit");
    }

    /// ★still-MISSING-窗口边界（codex 异质裁决坐实，编码为可执行断言，formalization-validity-domain）：
    /// L0→L1 的三段方向交替窗口**结构上不产 B2/S2**——三段交替里可背驰的同向段只在位置 2（末段，
    /// 无后继回拉），位置 0 无前同向对照，位置 1 是唯一异向（无前同向）。故单个 L1 走势的 3 段 L0
    /// subs 内识别不出「第一类离开（破中枢∧背驰）+ 后继回拉」。
    ///
    /// 此断言锁定边界：L0 级别（索引 0）的 bsp **不含 B2/S2**（B2/S2 由 L1→L2 几何路径产，见
    /// `end_to_end_second_buy_via_l1_l2_geometric`）。这是固定三段封装的结构上界，非补丁——放松
    /// 背驰约束或窗口大小来强产 L0 层 B2 = 声明膨胀（no-patch 禁止）。
    #[test]
    fn l0_level_emits_no_second_class_window_bound() {
        let cfg = ThetaConfig::default();
        // 简单 up-down-up 三段（一个 L0 中枢，一个 L1 走势）——L1 走势 3 段 subs 内无法产 B2/S2。
        let segments = vec![
            seg(Direction::Up, 0, 4, 100, 200),
            seg(Direction::Down, 4, 8, 200, 100),
            seg(Direction::Up, 8, 12, 100, 200),
        ];
        let closes: Vec<i64> = (0..16).map(|i| 100 + (i % 4) * 10).collect();
        let layer = ParseLayer {
            segments: Rc::new(segments),
            merged_bars: Rc::new(bars_from_closes(&closes)),
            ..Default::default()
        };
        let out = classify(&layer, &cfg);
        // L0 级别 bsp 不含第二类（三段交替窗口结构上界）。
        for p in out.levels[0].bsp.iter() {
            assert!(
                !p.bits.buy2,
                "L0→L1 三段交替窗口不产 B2（still-MISSING-窗口，codex 裁决）"
            );
            assert!(
                !p.bits.sell2,
                "L0→L1 三段交替窗口不产 S2（still-MISSING-窗口，codex 裁决）"
            );
        }
    }

    /// ★codex-decide-20260703 裁定 A 最大实现风险点（end_price 忠实性，单测强制）：级别-N 输入单元
    /// → Segment 的端点价按 fold_direction 取 hi/lo，且与 `segment_to_unit` 互逆（L0 段 round-trip
    /// bit-exact）。此测试失败 ⟹ 级别-N「线段」端点价错位 ⟹ A/C 破中枢几何 + judge_third 判据全错。
    #[test]
    fn unit_to_segment_endpoint_faithful_and_roundtrips() {
        use super::center::UnitRange;
        // 向上单元：起点=lo、终点=hi（seg_end 取 end_price=hi=高点）。
        let up = UnitRange {
            start_index: 4,
            end_index: 8,
            direction: Direction::Up,
            lo: 90,
            hi: 150,
        };
        let s_up = unit_to_segment(&up);
        assert_eq!(
            (s_up.start_price, s_up.end_price),
            (90, 150),
            "向上单元 end_price=hi（终点=高点）"
        );
        assert_eq!(s_up.direction, Direction::Up);
        assert_eq!(
            (s_up.start_index, s_up.end_index),
            (4, 8),
            "source_index 坐标保留（A/C 面积映射用）"
        );
        // 向下单元：起点=hi、终点=lo（终点=低点）。
        let down = UnitRange {
            start_index: 8,
            end_index: 12,
            direction: Direction::Down,
            lo: 90,
            hi: 150,
        };
        let s_down = unit_to_segment(&down);
        assert_eq!(
            (s_down.start_price, s_down.end_price),
            (150, 90),
            "向下单元 end_price=lo（终点=低点）"
        );
        // round-trip：L0 段 → segment_to_unit → unit_to_segment == 原段（互逆 bit-exact）。
        for orig in [
            seg(Direction::Up, 0, 4, 100, 200),
            seg(Direction::Down, 4, 8, 200, 50),
        ] {
            let back = unit_to_segment(&segment_to_unit(&orig));
            assert_eq!(
                (
                    back.direction,
                    back.start_index,
                    back.end_index,
                    back.start_price,
                    back.end_price
                ),
                (
                    orig.direction,
                    orig.start_index,
                    orig.end_index,
                    orig.start_price,
                    orig.end_price
                ),
                "segment_to_unit ∘ unit_to_segment = id（round-trip bit-exact）"
            );
        }
    }

    /// ★裁定 A gap-fill 非 no-op（结构合法性）：`extract_first_third_for_level` 在级别-N 下跌趋势
    /// units（承担线段角色）+ ≥2 依次向下中枢（Trend(Down)）+ C 段破最后中枢 + C<A 背驰上产 1 买。
    /// 复用 signal.rs `first_buy_extracted_with_trend_divergence` 的 A/B/C 几何，但以 UnitRange 表达
    /// ——证明级别-N 一/三类判定真接线（旧 `else { Vec::new() }` 恒产 0，此测试产 1 = 缺口已填）。
    #[test]
    fn level_ge1_extract_first_third_produces_type1_via_units() {
        use super::center::UnitRange;
        // 两依次向下中枢（c1.gg=210 < c0.dd=290 ⟹ DownContinuation ⟹ trend_class=Trend(Down)）。
        let c0 = Center {
            zd: 300,
            zg: 400,
            dd: 290,
            gg: 410,
            start_index: 0,
            end_index: 2,
        };
        let c1 = Center {
            zd: 100,
            zg: 200,
            dd: 90,
            gg: 210,
            start_index: 0,
            end_index: 8,
        };
        // Down 单元：lo=终点价、hi=起点价（unit_to_segment 还原 start=hi/end=lo）。
        let units = vec![
            UnitRange {
                start_index: 3,
                end_index: 5,
                direction: Direction::Down,
                lo: 250,
                hi: 350,
            }, // A 段（C0 离开）
            UnitRange {
                start_index: 5,
                end_index: 7,
                direction: Direction::Up,
                lo: 250,
                hi: 280,
            }, // B 段连接
            UnitRange {
                start_index: 9,
                end_index: 11,
                direction: Direction::Down,
                lo: 80,
                hi: 150,
            }, // C 段破 C1（<100）
            UnitRange {
                start_index: 11,
                end_index: 13,
                direction: Direction::Up,
                lo: 80,
                hi: 90,
            }, // #607 D2：T3-in-c 固定首对 retest（仍 < zd=100）
        ];
        // A 段 bar[3,5] 急跌（hist 面积大）、C 段 bar[9,11] 缓动（面积小=背驰）——同 signal.rs fixture。
        let prices: Vec<i64> = vec![300, 300, 300, 300, 100, 250, 250, 250, 250, 248, 246, 244];
        let closes: Vec<f64> = prices.iter().map(|&v| v as f64).collect();
        let close_src: Vec<usize> = (0..prices.len()).collect();
        let hist = divergence::compute_macd(&closes, &ThetaConfig::default().macd).hist;
        // 本测试只验结构六 bit（force 旁挂不改），传空 dif/closes_tick ⟹ force=None（不影响 buy1 判据）。
        // Q7-#1 裁定C：显式全锚（本测试验证的是 Trend ownership 单元的 gap-fill 路径）。
        let anchors = [
            Some(Direction::Down),
            Some(Direction::Up),
            Some(Direction::Down),
            Some(Direction::Up),
        ];
        let (bsp, _pan) = extract_first_third_for_level(
            &[c0, c1],
            &units,
            &anchors,
            &hist,
            &[],
            &[],
            &close_src,
            divergence::DivergenceGauge::default(),
            &mut Vec::new(),
        );
        let buy1: Vec<_> = bsp.iter().filter(|p| p.bits.buy1).collect();
        assert_eq!(
            buy1.len(),
            1,
            "级别-N 下跌趋势 C 段破最后中枢 ∧ C<A 背驰 ⟹ 一个 1 买（缺口已填，非 no-op）"
        );
        assert_eq!(
            buy1[0].source_index, 11,
            "1 买端点 = C 段（破最后中枢单元）终止 source_index"
        );
        assert_eq!(
            buy1[0].pivot_low, 80,
            "1 买止损源 = pivot_low（C 段破中枢端点极值）"
        );
        // ★owner 载体补齐（关③ 补记② 路径 (a)）：一类点构造时填入判定中枢 last_center=c1
        //（被破的最后中枢）——名实一致根据同 signal.rs `first_buy_extracted_with_trend_divergence`
        //（本测试复用其 A/B/C 几何的 UnitRange 表达）；center 是 owner 载体，止损仍 pivot
        //（pivot_low=80 上条已锁，center 不进 1/2 类止损判据）。
        // （#218 面 A 载体形态机械适配：一/三类载 OwnerRef::Center，语义不动。）
        assert_eq!(
            buy1[0].center,
            Some(signal::OwnerRef::Center(c1)),
            "一类点 center = 判定中枢（owner 载体）；止损仍 pivot 非 center"
        );
    }

    /// ★裁定 A 三类（高级别「中枢外缘区间」边界语义，codex 风险点单独 snapshot）：级别-N 离开中枢
    /// + 回试不重入 ⟹ 3 买。`judge_third` 在级别-N units（外缘区间端点 hi/lo）vs 几何中枢 [zd,zg]
    /// 上判定——离开单元终点 > c.zg ∧ 回试单元终点 > c.zg（严格不触闭区间）。
    #[test]
    fn level_ge1_extract_first_third_produces_type3_via_units() {
        use super::center::UnitRange;
        // 单中枢 [100,200]（盘整 τ ⟹ 无一类）——三类是纯几何位置判据，不依赖趋势门控。
        let c = Center {
            zd: 100,
            zg: 200,
            dd: 90,
            gg: 210,
            start_index: 0,
            end_index: 12,
        };
        let units = vec![
            // 离开单元：向上，终点=hi=250 > zg=200（离开中枢上方）。
            UnitRange {
                start_index: 12,
                end_index: 16,
                direction: Direction::Up,
                lo: 150,
                hi: 250,
            },
            // 回试单元：向下，终点=lo=210 > zg=200（不重入闭区间中枢）⟹ 3 买。
            UnitRange {
                start_index: 16,
                end_index: 20,
                direction: Direction::Down,
                lo: 210,
                hi: 250,
            },
        ];
        // 三类无 MACD 依赖（纯整数几何），hist 空亦可——传空 hist/dif/closes_tick（第一类自然不产，force=None）。
        // Q7-#1 裁定C：显式全锚（leave 单元有 Trend ownership 资格的三类路径）。
        let anchors = [Some(Direction::Up), Some(Direction::Down)];
        let (bsp, _pan) = extract_first_third_for_level(
            &[c],
            &units,
            &anchors,
            &[],
            &[],
            &[],
            &(0..24).collect::<Vec<_>>(),
            divergence::DivergenceGauge::default(),
            &mut Vec::new(),
        );
        let buy3: Vec<_> = bsp.iter().filter(|p| p.bits.buy3).collect();
        assert_eq!(
            buy3.len(),
            1,
            "级别-N 离开中枢 + 回试不重入 ⟹ 一个 3 买（外缘区间端点判据）"
        );
        assert_eq!(
            buy3[0].source_index, 20,
            "3 买端点 = 回试单元终止 source_index"
        );
        assert_eq!(buy3[0].pivot_low, 210, "3 买止损源 = pivot_low（回试低点）");
        assert_eq!(
            buy3[0].center.and_then(|o| match o {
                signal::OwnerRef::Center(c) => Some(c.zg),
                _ => None,
            }),
            Some(200),
            "3 买 center=Some（止损=zg；#218 面 A 载体形态：Center 变体读出）"
        );
    }

    /// ★ADR 补充十三 / #486 / Spec #485：L≥1 一/三类的方向锚均取单元结构方向。
    /// provenance endpoint fallback（anchor=None）仍是序列成员，不再阻断几何合法的一/三类。
    /// 一类 p117 直调契约由 signal.rs 的
    /// `judge_first_cached_provenance_gate_preserved_for_direct_callers` 独立锁定；三类的方向匹配、
    /// 回试方向、严格 `>ZG/<ZD` 与 OwnerRef 契约均不变。
    #[test]
    fn q7_ruling_c_first_and_third_class_structural_direction_authorized() {
        use super::center::UnitRange;
        // fixture 同 level_ge1_extract_first_third_fills_type1_gap（两下行中枢 + A/B/C 三单元）。
        let c0 = Center {
            zd: 300,
            zg: 400,
            dd: 290,
            gg: 410,
            start_index: 0,
            end_index: 2,
        };
        let c1 = Center {
            zd: 100,
            zg: 200,
            dd: 90,
            gg: 210,
            start_index: 0,
            end_index: 8,
        };
        let units = vec![
            UnitRange {
                start_index: 3,
                end_index: 5,
                direction: Direction::Down,
                lo: 250,
                hi: 350,
            },
            UnitRange {
                start_index: 5,
                end_index: 7,
                direction: Direction::Up,
                lo: 250,
                hi: 280,
            },
            UnitRange {
                start_index: 9,
                end_index: 11,
                direction: Direction::Down,
                lo: 80,
                hi: 150,
            },
            UnitRange {
                start_index: 11,
                end_index: 13,
                direction: Direction::Up,
                lo: 80,
                hi: 90,
            }, // #607 D2：T3-in-c 固定首对 retest（仍 < zd=100）
        ];
        let prices: Vec<i64> = vec![300, 300, 300, 300, 100, 250, 250, 250, 250, 248, 246, 244];
        let closes: Vec<f64> = prices.iter().map(|&v| v as f64).collect();
        let close_src: Vec<usize> = (0..prices.len()).collect();
        let hist = divergence::compute_macd(&closes, &ThetaConfig::default().macd).hist;
        // 一类只锁 L≥1 调用层的结构锚授权：helper 会重建结构锚，故不在这里伪造 provenance
        // 变体。p117 的 provenance 直调契约由 signal.rs 上述独立测试锁定。
        let (first_bsp, _) = extract_first_third_for_level(
            &[c0, c1],
            &units,
            &[None, None, None, None],
            &hist,
            &[],
            &[],
            &close_src,
            divergence::DivergenceGauge::default(),
            &mut Vec::new(),
        );
        assert_eq!(
            first_bsp.iter().filter(|p| p.bits.buy1).count(),
            1,
            "#486：L≥1 一类按结构方向锚产一买"
        );
        // 三类：ADR 补充十三 / #486 / Spec #485 授权 L≥1 leave 使用结构方向锚；
        // provenance fallback=None 不再否决三买，retest 仍是几何角色、不另设锚门。
        let c = Center {
            zd: 100,
            zg: 200,
            dd: 90,
            gg: 210,
            start_index: 0,
            end_index: 12,
        };
        let u3 = vec![
            UnitRange {
                start_index: 12,
                end_index: 16,
                direction: Direction::Up,
                lo: 150,
                hi: 250,
            },
            UnitRange {
                start_index: 16,
                end_index: 20,
                direction: Direction::Down,
                lo: 210,
                hi: 250,
            },
        ];
        let src24: Vec<usize> = (0..24).collect();
        let (bsp, _) = extract_first_third_for_level(
            &[c],
            &u3,
            &[None, Some(Direction::Down)],
            &[],
            &[],
            &[],
            &src24,
            divergence::DivergenceGauge::default(),
            &mut Vec::new(),
        );
        let buy3: Vec<_> = bsp.iter().filter(|p| p.bits.buy3).collect();
        assert_eq!(
            buy3.len(),
            1,
            "#486：provenance fallback=None 时，L≥1 仍按结构方向产三买"
        );
        assert_eq!(
            buy3[0].center,
            Some(signal::OwnerRef::Center(c)),
            "三类 OwnerRef 仍精确指向所破中枢"
        );
        // provenance 是否有值不再改变同一结构几何的三类输出。
        let (bsp2, _) = extract_first_third_for_level(
            &[c],
            &u3,
            &[Some(Direction::Up), Some(Direction::Down)],
            &[],
            &[],
            &[],
            &src24,
            divergence::DivergenceGauge::default(),
            &mut Vec::new(),
        );
        assert_eq!(bsp2, bsp);
    }

    #[test]
    fn issue486_level_ge1_third_rejects_structural_direction_geometry_mismatch() {
        use super::center::UnitRange;
        let c = Center {
            zd: 100,
            zg: 200,
            dd: 90,
            gg: 210,
            start_index: 0,
            end_index: 12,
        };
        let units = vec![
            // 价格位于上方，但结构方向是 Down；旧 provenance=Up 不得越权把它认作向上离开。
            UnitRange {
                start_index: 12,
                end_index: 16,
                direction: Direction::Down,
                lo: 250,
                hi: 260,
            },
            UnitRange {
                start_index: 16,
                end_index: 20,
                direction: Direction::Down,
                lo: 210,
                hi: 250,
            },
        ];
        let src24: Vec<usize> = (0..24).collect();
        let (bsp, _) = extract_first_third_for_level(
            &[c],
            &units,
            &[Some(Direction::Up), Some(Direction::Down)],
            &[],
            &[],
            &[],
            &src24,
            divergence::DivergenceGauge::default(),
            &mut Vec::new(),
        );
        assert!(
            bsp.iter().all(|p| !p.bits.buy3 && !p.bits.sell3),
            "结构方向与三买几何方向不符时必须拒绝，provenance 不得覆盖结构事实"
        );
    }

    #[test]
    fn issue486_level_ge1_third_rejects_retest_equal_center_edge() {
        use super::center::UnitRange;
        let c = Center {
            zd: 100,
            zg: 200,
            dd: 90,
            gg: 210,
            start_index: 0,
            end_index: 12,
        };
        let src24: Vec<usize> = (0..24).collect();
        let buy_equal = vec![
            UnitRange {
                start_index: 12,
                end_index: 16,
                direction: Direction::Up,
                lo: 150,
                hi: 250,
            },
            UnitRange {
                start_index: 16,
                end_index: 20,
                direction: Direction::Down,
                lo: 200,
                hi: 250,
            },
        ];
        let sell_equal = vec![
            UnitRange {
                start_index: 12,
                end_index: 16,
                direction: Direction::Down,
                lo: 50,
                hi: 150,
            },
            UnitRange {
                start_index: 16,
                end_index: 20,
                direction: Direction::Up,
                lo: 50,
                hi: 100,
            },
        ];
        for units in [&buy_equal, &sell_equal] {
            let (bsp, _) = extract_first_third_for_level(
                &[c],
                units,
                &[None, None],
                &[],
                &[],
                &[],
                &src24,
                divergence::DivergenceGauge::default(),
                &mut Vec::new(),
            );
            assert!(
                bsp.iter().all(|p| !p.bits.buy3 && !p.bits.sell3),
                "retest==ZG/ZD 仍触及闭区间中枢，必须严格拒绝"
            );
        }
    }

    #[test]
    fn issue486_l0_third_output_fields_unchanged() {
        let c = Center {
            zd: 100,
            zg: 200,
            dd: 90,
            gg: 210,
            start_index: 0,
            end_index: 12,
        };
        let segs = vec![
            Segment {
                direction: Direction::Up,
                start_index: 12,
                end_index: 16,
                start_price: 150,
                end_price: 250,
            },
            Segment {
                direction: Direction::Down,
                start_index: 16,
                end_index: 20,
                start_price: 250,
                end_price: 210,
            },
        ];
        let src24: Vec<usize> = (0..24).collect();
        let (bsp, pan) = signal::extract_signals_with_hist(
            &[c],
            &segs,
            &[],
            &[],
            &[],
            &src24,
            divergence::DivergenceGauge::default(),
            &mut Vec::new(),
        );
        assert!(pan.is_empty());
        assert_eq!(bsp.len(), 1);
        let p = &bsp[0];
        assert_eq!(p.source_index, 20);
        assert!(!p.bits.buy1);
        assert!(!p.bits.buy2);
        assert!(p.bits.buy3);
        assert!(!p.bits.sell1);
        assert!(!p.bits.sell2);
        assert!(!p.bits.sell3);
        assert_eq!(p.pivot_low, 210);
        assert_eq!(p.pivot_high, 0);
        assert_eq!(p.center, Some(signal::OwnerRef::Center(c)));
        assert_eq!(p.struct_break_dir, None);
        assert!(p.force.is_none());
    }

    /// ★L2 信号普查诊断（裁定 A gap-fill 真实数据核验，`--ignored` 手动跑，依赖 analysis/data_cache）：
    /// 逐级别统计 BTC 全量分类的 buy1/sell1/buy2/sell2/buy3/sell3 计数 + 抽样 level≥1 一类端点。
    /// 修前 level≥1 一/三类恒 0（audit #119 `else{Vec::new()}`），故 level≥1 的 type1/type3 计数 =
    /// 本次实装引入的净增信号。运行：`cargo test --lib -- --ignored --nocapture level_signal_census_btc`。
    #[test]
    #[ignore = "L2 真实数据普查：cargo test --lib -- --ignored --nocapture level_signal_census_btc"]
    fn level_signal_census_btc() {
        use super::super::backtest::data::load_by_symbol;
        use super::super::parser::parse_layer;
        let cfg = ThetaConfig::default();
        let full = load_by_symbol("BTC", &cfg)
            .expect("BTC 数据加载（analysis/data_cache/btc_1m_full.json）");
        // 可选日期窗（CENSUS_WINDOW="2020-10-01,2021-04-01"）——检验 type1 的水平线依赖性：
        // 全历史中枢链全局非单调 ⟹ trend_class=Degenerate ⟹ type1=0；单向牛/熊窗内某级链可单调 ⟹ type1>0。
        let ds = match std::env::var(crate::theta_v0::env_registry::CENSUS_WINDOW) {
            Ok(w) => {
                let (s, e) = w.split_once(',').expect("CENSUS_WINDOW 格式 start,end");
                eprintln!("[census] window={s}..{e}");
                full.slice_date_window(s, e)
            }
            Err(_) => full,
        };
        eprintln!("[census] BTC bars={}", ds.bars.len());
        let layer = parse_layer(&ds.bars, &cfg);
        eprintln!(
            "[census] L0 segments={} merged_bars={}",
            layer.segments.len(),
            layer.merged_bars.len()
        );
        let out = classify(&layer, &cfg);
        eprintln!("[census] levels={}", out.levels.len());
        for (li, lv) in out.levels.iter().enumerate() {
            let trend = lv
                .moves
                .iter()
                .filter(|m| m.kind == MoveKind::Trend)
                .count();
            let (mut b1, mut s1, mut b2, mut s2, mut b3, mut s3) = (0, 0, 0, 0, 0, 0);
            for p in lv.bsp.iter() {
                b1 += p.bits.buy1 as usize;
                s1 += p.bits.sell1 as usize;
                b2 += p.bits.buy2 as usize;
                s2 += p.bits.sell2 as usize;
                b3 += p.bits.buy3 as usize;
                s3 += p.bits.sell3 as usize;
            }
            eprintln!(
                "[census] L{li}: centers={} moves={}(trend={}) bsp={} pan_div={} | buy1={b1} sell1={s1} buy2={b2} sell2={s2} buy3={b3} sell3={s3}",
                lv.centers.len(), lv.moves.len(), trend, lv.bsp.len(), lv.pan_div.len()
            );
            // 抽样：level≥1 的前 3 个一类端点（若有）+ 前 3 个三类端点（人工核结构合法性——
            // source_index + center[zd,zg] + pivot（回试端点极值）；三类不重入判据由 judge_third 保证）。
            if li >= 1 {
                let t1: Vec<_> = lv
                    .bsp
                    .iter()
                    .filter(|p| p.bits.buy1 || p.bits.sell1)
                    .take(3)
                    .collect();
                for (k, p) in t1.iter().enumerate() {
                    eprintln!(
                        "[census]   L{li} type1#{k}: src_idx={} buy1={} sell1={} break_dir={:?} pivot_low={} pivot_high={}",
                        p.source_index, p.bits.buy1, p.bits.sell1, p.struct_break_dir, p.pivot_low, p.pivot_high
                    );
                }
                let t3: Vec<_> = lv
                    .bsp
                    .iter()
                    .filter(|p| p.bits.buy3 || p.bits.sell3)
                    .take(3)
                    .collect();
                for (k, p) in t3.iter().enumerate() {
                    eprintln!(
                        "[census]   L{li} type3#{k}: src_idx={} buy3={} sell3={} center_zd={:?} center_zg={:?} pivot_low={} pivot_high={}",
                        p.source_index, p.bits.buy3, p.bits.sell3,
                        p.center.and_then(|o| match o { signal::OwnerRef::Center(c) => Some(c.zd), _ => None }),
                        p.center.and_then(|o| match o { signal::OwnerRef::Center(c) => Some(c.zg), _ => None }),
                        p.pivot_low, p.pivot_high
                    );
                }
            }
        }
    }

    /// ★#141 一类判据链漏斗普查（外审问题包证据，`--ignored` 手动跑，依赖 analysis/data_cache）：
    /// 逐级别逐环真实计数——候选评估→趋势门→≥2中枢→broke→A/C配对→背驰。级别循环与 `classify_impl`
    /// 同一批私有函数（classify_level/compose_level/project_to_units），每级 centers 与生产 `classify`
    /// 输出 assert 对拍（675号：探针走生产路径）。另产每级中枢链关系直方图 + 前缀 τ 时间线（因果
    /// 重放中趋势门何时永久锁死 Degenerate）+ 反事实局部同向 run 计数（若按走势分解的局部趋势数）。
    /// 运行：`cargo test --release --lib -- --ignored --nocapture type1_funnel_census_btc`
    /// 窗口对照：`CENSUS_WINDOW="2020-10-01,2021-04-01" cargo test --release --lib -- --ignored --nocapture type1_funnel_census_btc`
    #[test]
    #[ignore = "L2 真实数据漏斗普查：cargo test --release --lib -- --ignored --nocapture type1_funnel_census_btc"]
    fn type1_funnel_census_btc() {
        use super::super::backtest::data::load_by_symbol;
        use super::super::parser::parse_layer;
        use super::center::{classify_relation, CenterRelation};
        let cfg = ThetaConfig::default();
        let full = load_by_symbol("BTC", &cfg)
            .expect("BTC 数据加载（analysis/data_cache/btc_1m_full.json）");
        let ds = match std::env::var(crate::theta_v0::env_registry::CENSUS_WINDOW) {
            Ok(w) => {
                let (s, e) = w.split_once(',').expect("CENSUS_WINDOW 格式 start,end");
                eprintln!("[funnel] window={s}..{e}");
                full.slice_date_window(s, e)
            }
            Err(_) => full,
        };
        eprintln!("[funnel] BTC bars={}", ds.bars.len());
        let layer = parse_layer(&ds.bars, &cfg);
        eprintln!(
            "[funnel] L0 segments={} merged_bars={}",
            layer.segments.len(),
            layer.merged_bars.len()
        );

        // 生产对拍源（675号守卫：级别循环不分叉）。
        let out = classify(&layer, &cfg);

        // classify_impl 同源输入（同一批私有函数，非重写）。
        let min_parts = cfg.level.min_parts_per_level as usize;
        let l_max = cfg.level.l_max as usize;
        let mut units: Vec<UnitRange> = layer.segments.iter().map(segment_to_unit).collect();
        assert!(!units.is_empty(), "空 L0 无漏斗对象");
        let closes: Vec<f64> = layer.merged_bars.iter().map(|b| b.close as f64).collect();
        let close_src: Vec<usize> = layer.merged_bars.iter().map(|b| b.source_index).collect();
        let series = divergence::compute_macd(&closes, &cfg.macd);
        let closes_tick: Vec<Tick> = layer.merged_bars.iter().map(|b| b.close).collect();
        let mut moves_tower: Rc<Vec<LeveledMove>> = Rc::new(
            units
                .iter()
                .enumerate()
                .map(|(i, u)| {
                    LeveledMove::from_unit(
                        u,
                        ElementId {
                            level: 0,
                            ordinal: i as u64,
                        },
                    )
                })
                .collect(),
        );

        for level_idx in 0..=l_max {
            if units.len() < min_parts {
                break;
            }
            let is_l0 = level_idx == 0;
            let (centers, _outcome) = classify_level(&units, is_l0);
            assert_eq!(
                centers, *out.levels[level_idx].centers,
                "L{level_idx} 中枢对拍（探针级别循环须与生产 classify 逐字段一致）"
            );

            // ★task #142 量化验收：延伸段数分布（窗口段数 = seed 3 + 延伸段；同一 build 直调
            // detect_centers_windowed_resume 取窗口，中枢序列与生产 classify_level 对拍）。
            {
                let build: fn(&UnitRange, &UnitRange, &UnitRange) -> Option<Center> = if is_l0 {
                    center::center_from_segments
                } else {
                    center::center_from_window
                };
                let windowed = recursive_tower::detect_centers_windowed_resume(&units, build, 0).0;
                assert_eq!(
                    windowed.iter().map(|(c, _)| *c).collect::<Vec<_>>(),
                    centers,
                    "L{level_idx} 窗口探针中枢序列 == 生产中枢序列"
                );
                let mut hist = [0usize; 4]; // 桶：=3（无延伸）/4-5/6-9/≥10 段
                let mut max_w = 0usize;
                for (_, (s, e)) in &windowed {
                    let w = e - s + 1;
                    max_w = max_w.max(w);
                    hist[if w <= 3 {
                        0
                    } else if w <= 5 {
                        1
                    } else if w <= 9 {
                        2
                    } else {
                        3
                    }] += 1;
                }
                eprintln!(
                    "[funnel] L{level_idx}: 窗口段数分布 =3段:{} 4-5:{} 6-9:{} ≥10:{} max={}",
                    hist[0], hist[1], hist[2], hist[3], max_w
                );
            }

            // 中枢链相邻关系直方图 + 前缀 τ 时间线 + 反事实局部同向 run。
            let rels: Vec<CenterRelation> = centers
                .windows(2)
                .map(|w| classify_relation(&w[0], &w[1]))
                .collect();
            let n_up = rels
                .iter()
                .filter(|r| **r == CenterRelation::UpContinuation)
                .count();
            let n_down = rels
                .iter()
                .filter(|r| **r == CenterRelation::DownContinuation)
                .count();
            let n_exp = rels
                .iter()
                .filter(|r| **r == CenterRelation::LevelExpansion)
                .count();
            // 前缀 τ：τ(前k中枢)=Trend ⟺ k≥2 ∧ rels[0..k-1] 全等且非 Expansion。锁死点=首个异关系下标。
            let trend_open = !rels.is_empty() && rels[0] != CenterRelation::LevelExpansion;
            let lock_at = if rels.is_empty() {
                None
            } else if !trend_open {
                Some(0) // 首关系即 Expansion ⟹ 第3个中枢确认时 τ 已锁死 Degenerate
            } else {
                rels.iter().position(|r| *r != rels[0])
            };
            // 反事实（若走势分解为局部走势类型）：同向关系（Up/Down）的极大 run，每个 run 长 L = 局部
            // 趋势含 L+1 个中枢。计 run 数与最长 run。
            let (mut runs_ge1, mut longest_run, mut cur_run) = (0usize, 0usize, 0usize);
            for (k, r) in rels.iter().enumerate() {
                let same_dir = *r != CenterRelation::LevelExpansion;
                let cont = same_dir && (k == 0 || rels[k - 1] == *r);
                if same_dir {
                    cur_run = if cont { cur_run + 1 } else { 1 };
                    if cur_run == 1 {
                        runs_ge1 += 1;
                    }
                    longest_run = longest_run.max(cur_run);
                } else {
                    cur_run = 0;
                }
            }
            let lock_desc = match lock_at {
                None if trend_open => format!("全链同向（不锁死）"),
                None => format!("链长<2 无关系"),
                Some(i) => {
                    let c_end = centers[i + 1].end_index;
                    let date = ds
                        .dates
                        .get(c_end)
                        .map(|d| d.get(..10).unwrap_or("?"))
                        .unwrap_or("?");
                    format!("中枢#{}（end_src={} {date}）", i + 1, c_end)
                }
            };

            let (segs, funnel_anchors): (Vec<Segment>, Option<Vec<Option<Direction>>>) = if is_l0 {
                (layer.segments.to_vec(), None)
            } else {
                // Q7-#1 裁定C + 675号：漏斗探针锚与生产 units_anchors 同源（producer blocks 派生）。
                let pb = &out.levels[level_idx - 1].moves;
                (
                    units.iter().map(unit_to_segment).collect(),
                    Some(
                        (0..units.len())
                            .map(|i| decompose::center_own_dir_at(pb, i))
                            .collect(),
                    ),
                )
            };
            let f = signal::type1_funnel_dx(
                &centers,
                &segs,
                funnel_anchors.as_deref(),
                &series.hist,
                &series.dif,
                &closes_tick,
                &close_src,
            );
            eprintln!(
                "[funnel] L{level_idx}: centers={} segs={} rel(up/down/exp)={}/{}/{} blocks(trend/consol)={}/{} 最长趋势块={}中枢 | 旧AllTrend锁死点={} 局部同向run≥2中枢数={} 最长run={}(={}中枢)",
                f.n_centers, f.n_segments, n_up, n_down, n_exp, f.n_trend_blocks, f.n_consol_blocks,
                f.longest_trend_run, lock_desc, runs_ge1, longest_run, longest_run + 1
            );
            eprintln!(
                "[funnel] L{level_idx}: 环0候选(有最近中枢)={} → 环1有前驱中枢={} → 环2过局部趋势门={} → 环3破最后中枢={} → 环4 A/C配对={} → 环4b 037:20破b极值={} → 环5坐标映射={} → 环6背驰C<A={}",
                f.s_with_center, f.s_pos_ge1, f.s_gate_open, f.s_broke, f.s_a_paired, f.s_extreme, f.s_mapped, f.s_diverge
            );
            // ★task #144 验收证据：2021 顶区 sell1 在全历史因果重放中出现（先例窗反差闭合的正验证，
            // 生产 classify 输出直读——非探针另算）。窗口 = #141 外审切窗 2020-10-01..2021-04-15。
            {
                let top_sell1: Vec<&str> = out.levels[level_idx]
                    .bsp
                    .iter()
                    .filter(|p| p.bits.sell1)
                    .filter_map(|p| {
                        ds.dates
                            .get(p.source_index)
                            .map(|d| d.get(..10).unwrap_or("?"))
                    })
                    .filter(|d| ("2020-10-01".."2021-04-15").contains(d))
                    .collect();
                let n_sell1 = out.levels[level_idx]
                    .bsp
                    .iter()
                    .filter(|p| p.bits.sell1)
                    .count();
                let n_buy1 = out.levels[level_idx]
                    .bsp
                    .iter()
                    .filter(|p| p.bits.buy1)
                    .count();
                eprintln!(
                    "[funnel] L{level_idx}: 全历史 buy1={} sell1={} | 2021顶区(2020-10-01..2021-04-15) sell1×{}: {:?}",
                    n_buy1, n_sell1, top_sell1.len(), top_sell1
                );
            }

            let (_cw, upper_moves, _) =
                compose_level(&units, &moves_tower[..], is_l0, level_idx as u32 + 1);
            units = project_to_units(&upper_moves, &out.levels[level_idx].moves); // Q7：生产同源块
            moves_tower = Rc::new(upper_moves);
            if units.is_empty() {
                break;
            }
        }
    }

    /// #821 探针（wayfinder map #787 子票，乙类 task——只测量，不改生产行为/生产代码）：
    /// `recursive_tower.rs:766-780` 的 ≥9 段升级重切分支，子中枢核心「继承母中枢」
    /// （生产现状，口径 I）vs「按自身三段重算 `ZD=max(3 lo)`/`ZG=min(3 hi)` 且严格 `ZD<ZG`」
    /// （候选口径 R，`ZD>=ZG` 不产出该子中枢）——两种做法在真实行情上差多少。
    ///
    /// 与 `type1_funnel_census_btc` 同一套私有函数（`classify_level`/`compose_level`/
    /// `project_to_units`/`detect_centers_windowed_resume`）驱动级别循环，逐级别与生产
    /// `classify` 输出 assert 对拍（675号：探针走生产路径，非另起一套）。触发窗口的定位不靠
    /// 重新扫描——直接读 `detect_centers_windowed_resume` 返回的 `WinMeta.emitted`（升级窗口
    /// 该字段 = k > 1，`out`/`metas` 逐位对齐），子窗边界 `(s,e)` 按生产同一公式
    /// `s=win_start+t*3, e=(t+1==k ? win_exit-1 : s+2)` 重建，再从**同一份生产 `units`**切片
    /// 读三段真实 lo/hi——不重写 `detect_centers_windowed_resume` 本身的扫描逻辑。
    ///
    /// 数据：`analysis/data_cache/btc_1m_full.json`（`load_by_symbol("BTC", ..)`，1 分钟 K，
    /// 全历史）——bar 数以运行时 `[821] BTC bars=` 行为准（未做窗口切片，全量）。
    ///
    /// 下游翻转（点4）仅做**同级本地**翻译：把触发级别的中枢序列替换为口径 R 产出（丢弃项跳过、
    /// 保留项区间替换），直接喂 `decompose::decompose` 读 Trend/Consolidation 块计数差——
    /// **不做跨级级联**（口径 R 改变本级中枢数会经 `project_to_units` 改变下一级输入单元，
    /// 逐级复算等价于另起一条平行管线，超出本票探针预算，如实标注不做，非"无差异"）。
    /// BSP（买卖点）计数同理不做——`extract_signals_with_hist`/`extract_first_third_for_level`/
    /// `extract_second_for_level` 参数面广且依赖 MACD 背驰真算，接不通，如实标注。
    ///
    /// 运行：`cargo test --release --lib -- --ignored --nocapture issue821_upgrade_recut_probe`
    #[test]
    #[ignore = "issue #821 探针：cargo test --release --lib -- --ignored --nocapture issue821_upgrade_recut_probe"]
    fn issue821_upgrade_recut_probe() {
        use super::super::backtest::data::load_by_symbol;
        use super::super::parser::parse_layer;

        let cfg = ThetaConfig::default();
        let full = load_by_symbol("BTC", &cfg)
            .expect("BTC 数据加载（analysis/data_cache/btc_1m_full.json）");
        eprintln!("[821] BTC bars={}", full.bars.len());
        let layer = parse_layer(&full.bars, &cfg);
        eprintln!(
            "[821] L0 segments={} merged_bars={}",
            layer.segments.len(),
            layer.merged_bars.len()
        );

        // 生产对拍源（675号守卫：级别循环不分叉）。
        let out = classify(&layer, &cfg);

        let min_parts = cfg.level.min_parts_per_level as usize;
        let l_max = cfg.level.l_max as usize;
        let mut units: Vec<UnitRange> = layer.segments.iter().map(segment_to_unit).collect();
        let mut moves_tower: Rc<Vec<LeveledMove>> = Rc::new(
            units
                .iter()
                .enumerate()
                .map(|(i, u)| {
                    LeveledMove::from_unit(
                        u,
                        ElementId {
                            level: 0,
                            ordinal: i as u64,
                        },
                    )
                })
                .collect(),
        );

        let (mut total_centers_i, mut total_centers_r) = (0usize, 0usize);
        let (mut total_triggers, mut total_windows) = (0usize, 0usize);
        let mut total_discards = 0usize;
        let mut discard_examples: Vec<String> = Vec::new();
        let mut width_diffs: Vec<i64> = Vec::new(); // R宽 - 母核宽（ticks，可负=更窄）
        let mut outside_mother: Vec<String> = Vec::new();
        let mut trend_delta_examples: Vec<String> = Vec::new();

        for level_idx in 0..=l_max {
            if units.len() < min_parts {
                break;
            }
            let is_l0 = level_idx == 0;
            let (centers, _blocks) = classify_level(&units, is_l0);
            assert_eq!(
                centers, *out.levels[level_idx].centers,
                "L{level_idx} 中枢对拍（探针须与生产 classify 逐字段一致）"
            );

            let build: fn(&UnitRange, &UnitRange, &UnitRange) -> Option<Center> = if is_l0 {
                center::center_from_segments
            } else {
                center::center_from_window
            };
            let (out_win, metas, _cursor) =
                recursive_tower::detect_centers_windowed_resume(&units, build, 0);
            assert_eq!(
                out_win.len(),
                metas.len(),
                "L{level_idx} out/metas 1:1 对齐"
            );
            assert_eq!(
                out_win.iter().map(|(c, _)| *c).collect::<Vec<_>>(),
                centers,
                "L{level_idx} 窗口探针中枢序列 == 生产中枢序列"
            );

            let level_centers_i = out_win.len();
            total_centers_i += level_centers_i;

            let mut centers_r_level: Vec<Center> = Vec::new();
            let (mut level_triggers, mut level_centers_r, mut level_discards) =
                (0usize, 0usize, 0usize);
            let mut level_windows = 0usize; // 扫描出的成立窗口数（触发窗口计1，非触发窗口每条 meta 计1）
            let mut idx = 0usize;
            while idx < metas.len() {
                let m = metas[idx];
                level_windows += 1;
                if m.emitted > 1 {
                    level_triggers += 1;
                    total_triggers += 1;
                    let k = m.emitted;
                    let i = m.win_start;
                    let j = m.win_exit;
                    for t in 0..k {
                        let s = i + t * 3;
                        let e = if t + 1 == k { j - 1 } else { s + 2 };
                        let sub_units = &units[s..=e];
                        let zd_r = sub_units.iter().map(|u| u.lo).max().expect("子窗非空");
                        let zg_r = sub_units.iter().map(|u| u.hi).min().expect("子窗非空");
                        let mother = out_win[idx + t].0; // 口径I：zd/zg=继承母核心
                        if zd_r < zg_r {
                            level_centers_r += 1;
                            total_centers_r += 1;
                            let width_r = zg_r - zd_r;
                            let width_mother = mother.zg - mother.zd;
                            width_diffs.push(width_r - width_mother);
                            if zd_r < mother.zd || zg_r > mother.zg {
                                outside_mother.push(format!(
                                    "L{level_idx} win=(i={i},j={j}) 子#{t} segs[{s}..={e}] \
                                     R核心=[{zd_r},{zg_r}] 母核心=[{},{}]",
                                    mother.zd, mother.zg
                                ));
                            }
                            centers_r_level.push(Center {
                                zd: zd_r,
                                zg: zg_r,
                                dd: mother.dd,
                                gg: mother.gg,
                                start_index: mother.start_index,
                                end_index: mother.end_index,
                            });
                        } else {
                            level_discards += 1;
                            total_discards += 1;
                            if discard_examples.len() < 5 {
                                let detail: Vec<String> = sub_units
                                    .iter()
                                    .enumerate()
                                    .map(|(si, u)| {
                                        format!(
                                            "段{}(unit_idx={}) lo={} hi={}",
                                            si,
                                            s + si,
                                            u.lo,
                                            u.hi
                                        )
                                    })
                                    .collect();
                                discard_examples.push(format!(
                                    "L{level_idx} win=(i={i},j={j}) 子#{t} segs[{s}..={e}]: {} \
                                     => ZD=max(lo)={zd_r} ZG=min(hi)={zg_r}（ZD>=ZG，丢弃）",
                                    detail.join("; ")
                                ));
                            }
                        }
                    }
                    idx += k;
                } else {
                    level_centers_r += 1;
                    total_centers_r += 1;
                    centers_r_level.push(out_win[idx].0);
                    idx += 1;
                }
            }
            total_windows += level_windows;
            eprintln!(
                "[821] L{level_idx}: windows={level_windows} centers_I={level_centers_i} \
                 centers_R={level_centers_r} triggers={level_triggers} discards={level_discards}"
            );

            // 点4（部分，同级本地翻译，不跨级级联——见函数头注释）：口径R替换后 decompose 块计数差。
            if level_triggers > 0 {
                let blocks_i = decompose::decompose(&centers);
                let blocks_r = decompose::decompose(&centers_r_level);
                let trend_i = blocks_i
                    .iter()
                    .filter(|b| b.kind == MoveKind::Trend)
                    .count();
                let trend_r = blocks_r
                    .iter()
                    .filter(|b| b.kind == MoveKind::Trend)
                    .count();
                trend_delta_examples.push(format!(
                    "L{level_idx}: blocks_I={} (trend={trend_i}) blocks_R={} (trend={trend_r})",
                    blocks_i.len(),
                    blocks_r.len()
                ));
            }

            let (_cw, upper_moves, _) =
                compose_level(&units, &moves_tower[..], is_l0, level_idx as u32 + 1);
            units = project_to_units(&upper_moves, &out.levels[level_idx].moves);
            moves_tower = Rc::new(upper_moves);
            if units.is_empty() {
                break;
            }
        }

        eprintln!(
            "[821] TOTAL centers_I={total_centers_i} centers_R={total_centers_r} \
             triggers={total_triggers} discards={total_discards} (windows_seen={total_windows})"
        );
        eprintln!("[821] discard 示例（ZD>=ZG，最多5条）:");
        for ex in &discard_examples {
            eprintln!("  {ex}");
        }
        eprintln!(
            "[821] outside-mother 次数（R核心落在母核心之外）={}",
            outside_mother.len()
        );
        for ex in outside_mother.iter().take(10) {
            eprintln!("  {ex}");
        }
        if !width_diffs.is_empty() {
            let sum: i64 = width_diffs.iter().sum();
            let avg = sum as f64 / width_diffs.len() as f64;
            let min_d = *width_diffs.iter().min().unwrap();
            let max_d = *width_diffs.iter().max().unwrap();
            eprintln!(
                "[821] 区间宽度差（R宽-母核宽，ticks）: avg={avg:.3} min={min_d} max={max_d} n={}",
                width_diffs.len()
            );
        } else {
            eprintln!("[821] 区间宽度差：无保留的 R 子中枢样本（0 触发或全部丢弃）");
        }
        eprintln!("[821] 点4（trend block 计数，同级本地，非级联）:");
        for ex in &trend_delta_examples {
            eprintln!("  {ex}");
        }
    }

    /// #826 探针（wayfinder map #787 子票，乙类 task——只测量，不改生产行为/生产代码）：
    /// 本仓的塔（`compose_level` 逐级上升）产出的走势分解，与缠师第 38 课「同级别分解」
    /// （`docs/chanlun/text/blog/038-第38课.md:18`：把所有走势按一固定级别的走势类型分解成
    /// 一段段连接）是不是同一个东西。
    ///
    /// 测四件（票面）：
    /// 1. **切点对照**：塔每级的上级走势序列在 L0 原始 K 序上的切点，是否构成对整条行情的
    ///    **无缝无重叠覆盖**（同级别分解的定义性要求：「分解成一段段走势类型的**连接**」，
    ///    连接 ⟹ 前一段终点即后一段起点，无遗漏）。量：覆盖率、缺口条数、缺口跨的 bar 数。
    /// 2. **唯一性**：同输入同输出（纯函数）+ 前缀稳定性（把 units 截断到前 n 个再分解，
    ///    结果是否是全量分解的前缀）——第 38 课要求「分解的唯一性」。
    /// 3. **第 38 课细则**（`038-第38课.md:24`+`:26`）：同级别分解在**操作级别上不定义中枢
    ///    延伸、允许盘整+盘整**，而**该级别以下允许延伸**。塔在**每一级**都无条件跑
    ///    `detect_centers_windowed_resume` 的 Step2 延伸吸收 ⟹ 量「每级有多少窗口发生了延伸
    ///    （窗口段数>3）」，若每级都>0，则塔不存在一个「关掉延伸」的操作级别。
    /// 4. **最小反例**：打印首个缺口（塔丢弃的连接段）的坐标 + 该缺口两侧上级走势。
    ///
    /// 数据：`analysis/data_cache/btc_1m_full.json`（`load_by_symbol("BTC", ..)`，1 分钟 K，
    /// 全历史）——bar 数以运行时 `[826] BTC bars=` 行为准。级别循环与生产 `classify` 逐级
    /// assert 对拍（675号：探针走生产路径）。
    ///
    /// 运行：`cargo test --release --lib -- --ignored --nocapture issue826_same_level_decomp_probe`
    #[test]
    #[ignore = "issue #826 探针：cargo test --release --lib -- --ignored --nocapture issue826_same_level_decomp_probe"]
    fn issue826_same_level_decomp_probe() {
        use super::super::backtest::data::load_by_symbol;
        use super::super::parser::parse_layer;

        let cfg = ThetaConfig::default();
        let full = load_by_symbol("BTC", &cfg)
            .expect("BTC 数据加载（analysis/data_cache/btc_1m_full.json）");
        eprintln!("[826] BTC bars={}", full.bars.len());
        let layer = parse_layer(&full.bars, &cfg);
        eprintln!(
            "[826] L0 segments={} merged_bars={}",
            layer.segments.len(),
            layer.merged_bars.len()
        );
        let l0_span_lo = layer.segments.first().map(|s| s.start_index).unwrap_or(0);
        let l0_span_hi = layer.segments.last().map(|s| s.end_index).unwrap_or(0);
        eprintln!("[826] L0 线段覆盖的原始 K 序区间 = [{l0_span_lo}, {l0_span_hi}]");

        let out = classify(&layer, &cfg);

        let min_parts = cfg.level.min_parts_per_level as usize;
        let l_max = cfg.level.l_max as usize;
        let mut units: Vec<UnitRange> = layer.segments.iter().map(segment_to_unit).collect();
        let mut moves_tower: Rc<Vec<LeveledMove>> = Rc::new(
            units
                .iter()
                .enumerate()
                .map(|(i, u)| {
                    LeveledMove::from_unit(
                        u,
                        ElementId {
                            level: 0,
                            ordinal: i as u64,
                        },
                    )
                })
                .collect(),
        );

        let mut first_gap_report: Option<String> = None;

        for level_idx in 0..=l_max {
            if units.len() < min_parts {
                eprintln!(
                    "[826] L{level_idx}: units={} < min_parts ⟹ 塔自然终止",
                    units.len()
                );
                break;
            }
            let is_l0 = level_idx == 0;
            let (centers, _blocks) = classify_level(&units, is_l0);
            assert_eq!(
                centers, *out.levels[level_idx].centers,
                "L{level_idx} 中枢对拍（探针须与生产 classify 逐字段一致）"
            );

            let build: fn(&UnitRange, &UnitRange, &UnitRange) -> Option<Center> = if is_l0 {
                center::center_from_segments
            } else {
                center::center_from_window
            };
            let (out_win, _metas, _cursor) =
                recursive_tower::detect_centers_windowed_resume(&units, build, 0);
            assert_eq!(
                out_win.iter().map(|(c, _)| *c).collect::<Vec<_>>(),
                centers,
                "L{level_idx} 窗口探针中枢序列 == 生产中枢序列"
            );

            let (_cw, upper_moves, _) =
                compose_level(&units, &moves_tower[..], is_l0, level_idx as u32 + 1);

            // ── 点 1：单元层覆盖（本级 units 有多少被上级走势吃掉，多少被丢弃）
            let n_units = units.len();
            let mut consumed_units = 0usize;
            for (_, (a, b)) in &out_win {
                consumed_units += b - a + 1;
            }
            // 缺口 = 相邻窗口之间未被任何窗口覆盖的 unit 段（含头尾）
            let mut gap_runs: Vec<(usize, usize)> = Vec::new(); // unit 索引闭区间
            let mut cursor_u = 0usize;
            for (_, (a, b)) in &out_win {
                if *a > cursor_u {
                    gap_runs.push((cursor_u, a - 1));
                }
                cursor_u = b + 1;
            }
            if cursor_u < n_units {
                gap_runs.push((cursor_u, n_units - 1));
            }
            let gap_units: usize = gap_runs.iter().map(|(a, b)| b - a + 1).sum();
            // ── 基线校准：本级 units 序列自身在 bar 坐标上是否首尾相接（若本身就不相接，
            // 则塔的 bar 层断点不能全部归因于「丢弃连接段」）。
            let unit_adj_ok = units
                .windows(2)
                .filter(|w| w[0].end_index == w[1].start_index)
                .count();
            let unit_adj_gapbars: i64 = units
                .windows(2)
                .map(|w| (w[1].start_index as i64 - w[0].end_index as i64).max(0))
                .sum();
            eprintln!(
                "[826] L{level_idx} 基线: units 相邻首尾相接={unit_adj_ok}/{} units 自身缺口总 bar={unit_adj_gapbars}",
                units.len().saturating_sub(1)
            );
            // 点4 最小反例：L0 头 3 条缺口的逐单元明细（塔丢弃的连接段）
            if is_l0 {
                for (gi, (a, b)) in gap_runs.iter().take(3).enumerate() {
                    let detail: Vec<String> = (*a..=*b)
                        .map(|k| {
                            format!(
                                "unit#{k}[src {}..{} lo={} hi={}]",
                                units[k].start_index, units[k].end_index, units[k].lo, units[k].hi
                            )
                        })
                        .collect();
                    eprintln!(
                        "[826] 点4 L0 缺口#{gi}: 丢弃 units[{a}..={b}]（{} 段） {}",
                        b - a + 1,
                        detail.join(" ")
                    );
                }
            }

            // ── 点 1b：bar 层覆盖（上级走势在 L0 原始 K 序上的切点是否首尾相接）
            let mut covered_bars: i64 = 0;
            let mut joint_ok = 0usize;
            let mut joint_gap = 0usize;
            let mut joint_overlap = 0usize;
            let mut gap_bars: i64 = 0;
            for m in &upper_moves {
                covered_bars += m.end_index as i64 - m.start_index as i64;
            }
            for w in upper_moves.windows(2) {
                let (prev, next) = (&w[0], &w[1]);
                match next.start_index.cmp(&prev.end_index) {
                    std::cmp::Ordering::Equal => joint_ok += 1,
                    std::cmp::Ordering::Greater => {
                        joint_gap += 1;
                        gap_bars += next.start_index as i64 - prev.end_index as i64;
                        if first_gap_report.is_none() {
                            first_gap_report = Some(format!(
                                "L{level_idx}→L{}: 上级走势#{}[src {}..{}] 与 #{}[src {}..{}] 之间空出 {} 根 K（塔丢弃的连接段）",
                                level_idx + 1,
                                upper_moves.iter().position(|x| std::ptr::eq(x, prev)).unwrap_or(0),
                                prev.start_index, prev.end_index,
                                upper_moves.iter().position(|x| std::ptr::eq(x, next)).unwrap_or(0),
                                next.start_index, next.end_index,
                                next.start_index - prev.end_index
                            ));
                        }
                    }
                    std::cmp::Ordering::Less => joint_overlap += 1,
                }
            }
            let span_lo = upper_moves.first().map(|m| m.start_index).unwrap_or(0);
            let span_hi = upper_moves.last().map(|m| m.end_index).unwrap_or(0);
            let head_bars = span_lo as i64 - l0_span_lo as i64;
            let tail_bars = l0_span_hi as i64 - span_hi as i64;

            // ── 点 3：延伸窗口占比（窗口段数 > 3 ⟹ 该窗口发生了中枢延伸吸收）
            let ext_windows = out_win.iter().filter(|(_, (a, b))| b - a + 1 > 3).count();
            let max_win_len = out_win
                .iter()
                .map(|(_, (a, b))| b - a + 1)
                .max()
                .unwrap_or(0);

            // ── 点 1c：上级走势的跨度分布（同级别分解要求同级别 ⟹ 尺度可比）
            let mut spans: Vec<i64> = upper_moves
                .iter()
                .map(|m| m.end_index as i64 - m.start_index as i64)
                .collect();
            spans.sort_unstable();
            let (s_min, s_med, s_max) = if spans.is_empty() {
                (0, 0, 0)
            } else {
                (spans[0], spans[spans.len() / 2], spans[spans.len() - 1])
            };

            eprintln!(
                "[826] L{level_idx}: units={n_units} upper_moves={} | 单元覆盖: 被吃={consumed_units} 丢弃={gap_units}（{:.2}%）缺口段数={} \
                 | bar 切点: 首尾相接={joint_ok} 断开={joint_gap} 重叠={joint_overlap} 断开总 bar={gap_bars} 头={head_bars} 尾={tail_bars} \
                 | 上级走势跨度(bar) min={s_min} med={s_med} max={s_max} | 延伸窗口={ext_windows}/{} 最长窗口段数={max_win_len}",
                upper_moves.len(),
                100.0 * gap_units as f64 / n_units.max(1) as f64,
                gap_runs.len(),
                out_win.len(),
            );
            let _ = covered_bars;

            // ── 点 1d：与本仓**已有的另一套同级分解**对照——`decompose::decompose(centers)`
            // 产出的 MoveBlock 链（走势类型 Trend/Consolidation 的连接，中枢下标空间上
            // `b[j+1].start == b[j].end` 严格连续）。这是形状上唯一与「同级别分解」对得上的
            // 对象。塔**不走它**（塔走的是一中枢一上级走势的 upper_moves）。逐点比两者切点。
            let blocks = &out.levels[level_idx].moves;
            let blk_cuts: Vec<(usize, usize)> = blocks
                .iter()
                .map(|b| {
                    (
                        centers[b.start_center].start_index,
                        centers[b.end_center].end_index,
                    )
                })
                .collect();
            let mut blk_joint_ok = 0usize;
            let mut blk_joint_gap = 0usize;
            let mut blk_joint_overlap = 0usize;
            let mut blk_gap_bars: i64 = 0;
            for w in blk_cuts.windows(2) {
                match w[1].0.cmp(&w[0].1) {
                    std::cmp::Ordering::Equal => blk_joint_ok += 1,
                    std::cmp::Ordering::Greater => {
                        blk_joint_gap += 1;
                        blk_gap_bars += w[1].0 as i64 - w[0].1 as i64;
                    }
                    std::cmp::Ordering::Less => blk_joint_overlap += 1,
                }
            }
            // 切点集合逐点比：上级走势起点集合 vs 块起点集合
            let up_starts: std::collections::BTreeSet<usize> =
                upper_moves.iter().map(|m| m.start_index).collect();
            let blk_starts: std::collections::BTreeSet<usize> =
                blk_cuts.iter().map(|c| c.0).collect();
            let shared = up_starts.intersection(&blk_starts).count();
            eprintln!(
                "[826] L{level_idx} 两套口径切点对照: 走势类型块={} (切点接合: 相接={blk_joint_ok} 断开={blk_joint_gap} 重叠={blk_joint_overlap} 断开总bar={blk_gap_bars}) \
                 vs 塔上级走势={} | 起点切点交集={shared}（占块起点 {:.2}%，占塔起点 {:.2}%）",
                blocks.len(),
                upper_moves.len(),
                100.0 * shared as f64 / blk_starts.len().max(1) as f64,
                100.0 * shared as f64 / up_starts.len().max(1) as f64,
            );

            // ── 点 1e ★核心对照：口径 S = 第 38 课同级别分解规则的直译
            // （`038-第38课.md:20`「在这种同级别的分解中，是**不需要中枢延伸或扩展的概念**的，
            //   对30分钟来说，只要5分钟级别的**三段**上下上或下上下类型有价格区间的重合就构成
            //   中枢。如果这5分钟次级别延伸出6段，那么就当成**两个**30分钟盘整类型的连接」）：
            // 同一个 seed 判据（`build`，与生产逐字段同源），但窗口**恒为 3 段**、成立即 i += 3，
            // 不做任何延伸吸收。口径 T = 生产塔（seed + 延伸吸收 + ≥9 段重切）。
            let mut s_windows: Vec<(usize, usize)> = Vec::new();
            {
                let mut i = 0usize;
                while i + 2 < units.len() {
                    if build(&units[i], &units[i + 1], &units[i + 2]).is_some() {
                        s_windows.push((i, i + 2));
                        i += 3;
                    } else {
                        i += 1;
                    }
                }
            }
            let t_windows: Vec<(usize, usize)> = out_win.iter().map(|(_, w)| *w).collect();
            let s_starts: std::collections::BTreeSet<usize> =
                s_windows.iter().map(|w| w.0).collect();
            let t_starts: std::collections::BTreeSet<usize> =
                t_windows.iter().map(|w| w.0).collect();
            let st_shared = s_starts.intersection(&t_starts).count();
            let identical_windows = s_windows
                .iter()
                .filter(|w| t_windows.binary_search(w).is_ok())
                .count();
            let s_consumed: usize = s_windows.len() * 3;
            eprintln!(
                "[826] L{level_idx} ★口径对照 S(38课·禁延伸·恒3段) vs T(生产塔·延伸吸收): \
                 中枢数 S={} T={} (T/S={:.3}) | 窗口起点交集={st_shared}（占S {:.2}%，占T {:.2}%）\
                 | 完全相同的窗口(起止都同)={identical_windows} | 单元覆盖 S={s_consumed}/{n_units} T={consumed_units}/{n_units}",
                s_windows.len(),
                t_windows.len(),
                t_windows.len() as f64 / s_windows.len().max(1) as f64,
                100.0 * st_shared as f64 / s_starts.len().max(1) as f64,
                100.0 * st_shared as f64 / t_starts.len().max(1) as f64,
            );
            if is_l0 {
                // 最小反例：首个「T 延伸吸收 ≥6 段、S 拆成 ≥2 个中枢」的窗口
                if let Some((a, b)) = t_windows.iter().find(|(a, b)| b - a + 1 >= 6) {
                    let s_inside: Vec<_> = s_windows
                        .iter()
                        .filter(|w| w.0 >= *a && w.1 <= *b)
                        .collect();
                    eprintln!(
                        "[826] 点4 最小反例（L0 首个 ≥6 段延伸窗口）: T 把 units[{a}..={b}]（{} 段，src {}..{}）\
                         吃成 **1 个**中枢/1 段上级走势；S（38课）在同一区间产 **{} 个**中枢 {:?} \
                         ⟹ 上级走势数差 {}，38课口径下这里是「盘整+盘整」的连接，塔口径下是单个延伸中枢",
                        b - a + 1,
                        units[*a].start_index,
                        units[*b].end_index,
                        s_inside.len(),
                        s_inside,
                        s_inside.len() as i64 - 1,
                    );
                }
            }

            // ── 点 2：唯一性 / 前缀稳定性（同一份 units 截断到 90% 再分解，比对前缀）
            if n_units >= 20 {
                let cut = n_units * 9 / 10;
                let (pre_win, _, _) =
                    recursive_tower::detect_centers_windowed_resume(&units[..cut], build, 0);
                // 全量里完全落在 [0,cut) 内的窗口
                let full_inside: Vec<_> = out_win
                    .iter()
                    .filter(|(_, (_, b))| *b < cut)
                    .cloned()
                    .collect();
                let common = full_inside.len().min(pre_win.len());
                let mismatch = (0..common)
                    .filter(|&k| full_inside[k] != pre_win[k])
                    .count();
                for k in 0..common {
                    if full_inside[k] != pre_win[k] {
                        eprintln!(
                            "[826]   L{level_idx} 前缀不一致 #{k}: 全量 win={:?} center=[{},{}] vs 截断 win={:?} center=[{},{}]（cut={cut}）",
                            full_inside[k].1, full_inside[k].0.zd, full_inside[k].0.zg,
                            pre_win[k].1, pre_win[k].0.zd, pre_win[k].0.zg,
                        );
                        break;
                    }
                }
                eprintln!(
                    "[826] L{level_idx} 前缀稳定性: 截断到 units[..{cut}]，截断产出={} 全量内含={} 共同前缀比对不一致={}",
                    pre_win.len(),
                    full_inside.len(),
                    mismatch
                );
                // 确定性：同输入跑两遍
                let (again, _, _) =
                    recursive_tower::detect_centers_windowed_resume(&units, build, 0);
                assert_eq!(again, out_win, "L{level_idx} 同输入两次分解必须逐位相同");
            }

            units = project_to_units(&upper_moves, &out.levels[level_idx].moves);
            moves_tower = Rc::new(upper_moves);
            if units.is_empty() {
                break;
            }
        }

        eprintln!("[826] 点4 首个 bar 层缺口（最小反例锚）：");
        match &first_gap_report {
            Some(r) => eprintln!("  {r}"),
            None => eprintln!("  未发现 bar 层缺口"),
        }
    }

    #[test]
    fn classify_empty_layer_yields_empty() {
        let cfg = ThetaConfig::default();
        let out = classify(&ParseLayer::default(), &cfg);
        assert_eq!(out, Classification::default());
    }

    #[test]
    fn fewer_than_min_parts_natural_termination() {
        // L0 线段数 < min_parts_per_level(3) ⟹ 无 L0 走势，levels 空（自然终止）。
        let cfg = ThetaConfig::default();
        let layer = ParseLayer {
            segments: Rc::new(vec![
                seg(Direction::Up, 0, 4, 0, 10),
                seg(Direction::Down, 4, 8, 10, 5),
            ]),
            ..Default::default()
        };
        let out = classify(&layer, &cfg);
        assert!(out.levels.is_empty(), "2 段 < min_parts 3 ⟹ 自然终止");
    }

    #[test]
    fn three_overlapping_segments_form_center() {
        // 完整判据（Origin.CenterComplete，口径 B 637号）：方向交替 上-下-上 + 全三段核心非空。
        // 段区间 [0,10]up,[3,12]down,[5,15]up：核心取**全三段** zd=max(0,3,5)=5, zg=min(10,12,15)=10。
        // 第三段 [5,15] 收窄核心下沿（A 口径 zd=3 → B zd=5）⟹ 真中枢成立。
        let cfg = ThetaConfig::default();
        let layer = ParseLayer {
            segments: Rc::new(vec![
                seg(Direction::Up, 0, 4, 0, 10),
                seg(Direction::Down, 4, 8, 12, 3),
                seg(Direction::Up, 8, 12, 5, 15),
            ]),
            ..Default::default()
        };
        let out = classify(&layer, &cfg);
        assert!(!out.levels.is_empty());
        let l0 = &out.levels[0];
        assert_eq!(l0.centers.len(), 1, "三段方向交替+贯穿 ⟹ 一个真中枢");
        // 核心取全三段（口径 B，637号 computeZD/computeZG s1 s2 s3）——第三段收窄核心下沿至 5。
        assert_eq!((l0.centers[0].zd, l0.centers[0].zg), (5, 10));
        // 一个中枢 ⟹ 分解 = 单盘整块（PDF §6 情形1）。
        assert_eq!(l0.moves.len(), 1);
        assert_eq!(
            (
                l0.moves[0].kind,
                l0.moves[0].start_center,
                l0.moves[0].end_center
            ),
            (MoveKind::Consolidation, 0, 0)
        );
    }

    #[test]
    fn same_direction_three_segments_rejected_by_complete() {
        // G4 完整判据反退化：三段同向（全 up）+ 前两段核心非空，但无方向交替 ⟹ L0 不识别中枢
        // （旧几何窗口会误判，完整判据正确拒绝，对齐 Origin.CenterComplete.sameDir_not_centerConfirmed）。
        let cfg = ThetaConfig::default();
        let layer = ParseLayer {
            segments: Rc::new(vec![
                seg(Direction::Up, 0, 4, 10, 20),
                seg(Direction::Up, 4, 8, 18, 25),
                seg(Direction::Up, 8, 12, 22, 30),
            ]),
            ..Default::default()
        };
        let out = classify(&layer, &cfg);
        // 无方向交替 ⟹ L0 无中枢（完整判据拒绝单边三段）。
        assert_eq!(out.levels.len(), 1);
        assert!(
            out.levels[0].centers.is_empty(),
            "同向三段无方向交替 ⟹ 完整判据拒绝（非中枢）"
        );
    }

    #[test]
    fn lmax_bound_respected() {
        // 构造大量重叠段，验证递归不超过 l_max+1 级（每级至少消耗中枢，最终自然终止）。
        let cfg = ThetaConfig::default();
        let mut segments = Vec::new();
        // 27 段全重叠区间 [0,100]（每三段成一中枢，逐级递归）。
        for i in 0..27 {
            let dir = if i % 2 == 0 {
                Direction::Up
            } else {
                Direction::Down
            };
            segments.push(seg(dir, i * 4, i * 4 + 4, 0, 100));
        }
        let layer = ParseLayer {
            segments: Rc::new(segments),
            ..Default::default()
        };
        let out = classify(&layer, &cfg);
        // 级别数不超过 l_max+1（reference:30 上界，default l_max=6）。
        assert!(out.levels.len() <= cfg.level.l_max as usize + 1);
    }

    #[test]
    fn no_center_terminates_recursion() {
        // 三段无公共重叠 ⟹ L0 无中枢 ⟹ moves 为空（HigherCenterCandidate→None）+ 递归终止。
        let cfg = ThetaConfig::default();
        let layer = ParseLayer {
            segments: Rc::new(vec![
                seg(Direction::Up, 0, 4, 0, 4),
                seg(Direction::Down, 4, 8, 14, 10),
                seg(Direction::Up, 8, 12, 20, 24),
            ]),
            ..Default::default()
        };
        let out = classify(&layer, &cfg);
        // L0 无中枢 ⟹ 该级 moves 空（裁决退化），递归在该级后终止（无上级单元）。
        assert_eq!(out.levels.len(), 1);
        assert!(out.levels[0].centers.is_empty());
        assert!(out.levels[0].moves.is_empty());
    }

    /// ★端到端 fixture（Lead 验证门）：≥3 重叠线段 → 非空中枢 + 至少一个买卖点。
    /// 解阻塞点 A：classify 在真实结构输入上返回非空 Classification（n_orders>0 的前提）。
    #[test]
    fn end_to_end_third_buy_signal() {
        let cfg = ThetaConfig::default();
        // 段0-2：三段在 [100,200] 重叠 ⟹ seed 中枢，核心 [ZD,ZG]=[100,200] 冻结，end_index=12。
        // 段3：向上离开——延伸语义下（PDF §5 Step3，task #142）离开段必须与冻结核心不相交：
        //   lo=205 > ZG=200 ⟹ non-extension（旧 fixture lo=150 ≤ 200 会被 Step2 吸收进中枢 ⟹ 无离开段）。
        // 段4：向下回试低点 210 > zg=200（严格不触闭区间）⟹ 3 买 @ source_index=20。
        let layer = ParseLayer {
            segments: Rc::new(vec![
                seg(Direction::Up, 0, 4, 100, 200),
                seg(Direction::Down, 4, 8, 200, 100),
                seg(Direction::Up, 8, 12, 100, 200),
                seg(Direction::Up, 12, 16, 205, 250), // 离开中枢上方（lo=205>ZG ⟹ non-extension）
                seg(Direction::Down, 16, 20, 250, 210), // 回试低点 > zg → 3 买
            ]),
            ..Default::default()
        };
        let out = classify(&layer, &cfg);

        // 1) 非空 Classification + L0 有中枢。
        assert!(!out.levels.is_empty(), "解阻塞 A：classify 返回非空");
        let l0 = &out.levels[0];
        assert_eq!(l0.centers.len(), 1, "三段重叠 ⟹ 一个中枢");
        assert_eq!((l0.centers[0].zd, l0.centers[0].zg), (100, 200));

        // 2) 至少一个买卖点（第三类买点），且携带结构止损价 single source。
        assert!(!l0.bsp.is_empty(), "解阻塞 A：L0 至少一个买卖点");
        let third_buys: Vec<_> = l0.bsp.iter().filter(|p| p.bits.buy3).collect();
        assert_eq!(third_buys.len(), 1, "一个第三类买点");
        let p = third_buys[0];
        assert_eq!(p.source_index, 20, "买卖点定位回试端点");
        assert_eq!(p.pivot_low, 210, "结构止损价 pivot_low single source");
        assert_eq!(
            p.center.and_then(|o| match o {
                signal::OwnerRef::Center(c) => Some(c.zg),
                _ => None,
            }),
            Some(200),
            "3 买止损 = ZG single source（#218 面 A 载体形态：Center 变体读出）"
        );
    }

    /// 端到端边界：有中枢但无离开/回试 ⟹ 中枢非空、bsp 空（无买卖点是诚实产出，非错误）。
    #[test]
    fn end_to_end_center_without_signal() {
        let cfg = ThetaConfig::default();
        // 三段重叠成中枢，但无后续离开线段 ⟹ 无第三类买卖点。
        let layer = ParseLayer {
            segments: Rc::new(vec![
                seg(Direction::Up, 0, 4, 100, 200),
                seg(Direction::Down, 4, 8, 200, 100),
                seg(Direction::Up, 8, 12, 100, 200),
            ]),
            ..Default::default()
        };
        let out = classify(&layer, &cfg);
        assert_eq!(out.levels[0].centers.len(), 1);
        assert!(
            out.levels[0].bsp.is_empty(),
            "无离开/回试 ⟹ 无买卖点（诚实空）"
        );
    }

    /// ★#885 S4-d（**验收测试锁**）：否则域（`T3InCGrade::Missing`）分级记录进
    /// `Classification`、按 (level, source_index) 坐标可查——此前记录体只在 env 门控的
    /// thread_local 诊断 sidecar `GRADE_SIDECAR`，生产 `Classification` 按坐标查不到，
    /// 任何涉及类一类点的命中率因此只是下界。本票只建载体/可查性，判据未动
    /// （否则域点仍零一类 bit，#607 D2 语义不变）。
    ///
    /// fixture（全管线 classlify 真跑）：两依次向下中枢（C0[300,400] → C1[180,210]，外缘
    /// C1.gg=280 < C0.dd=290 ⟹ Trend(Down)）+ C 段 s6 破 C1 核心（端点 80 < zd=180）；固定首对
    /// = (s6 Down, s7 Down) 同向 ⟹ `Missing(SameDirection)`（#606 D1 五桶之一，不后扫）。
    /// closes：A 段 [12,20] 急跌（hist 面积大）→ 回拉 → C 段 [24,28] 缓跌（面积小 ⟹ C<A 背驰）。
    #[test]
    fn otherwise_domain_records_queryable_by_coordinate_in_classification() {
        let cfg = ThetaConfig::default();
        let segments = vec![
            seg(Direction::Up, 0, 4, 300, 400),
            seg(Direction::Down, 4, 8, 400, 290),
            seg(Direction::Up, 8, 12, 290, 410), // → C0 [300,400]（[0,12]，dd=290/gg=410）
            seg(Direction::Down, 12, 16, 280, 180), // [180,280] 不触 C0 核心 ⟹ non-extension
            seg(Direction::Up, 16, 20, 180, 210),
            seg(Direction::Down, 20, 24, 210, 150), // → C1 [180,210]（[12,24]，dd=150/gg=280）
            seg(Direction::Down, 24, 28, 170, 80),  // s6 C 段：破 C1 核心（80 < zd=180）
            seg(Direction::Down, 28, 32, 80, 70), // s7 与 s6 同向 ⟹ 固定首对 Missing(SameDirection)
        ];
        let closes: Vec<i64> = vec![
            350, 350, 350, 350, 350, 350, 350, 350, 350, 350, 350,
            350, // 0..12 预热（EMA 收敛）
            340, 320, 290, 260, 230, 200, 170, 150, // 12..20 A 段急跌（hist 面积大）
            160, 180, 200, 210, // 20..24 回拉（EMA 收敛）
            205, 200, 195, 190, // 24..28 C 段缓跌（hist 面积小 ⟹ C<A）
            188, 186, 184, 182, 180, // 28..33 缓跌延续
        ];
        let layer = ParseLayer {
            segments: Rc::new(segments.clone()),
            merged_bars: Rc::new(bars_from_closes(&closes)),
            ..Default::default()
        };
        let cls = classify(&layer, &cfg);

        // 0) 结构前提：L0 两中枢 + Trend(Down)（fixture 不自证则后述断言全空转）。
        let l0 = &cls.levels[0];
        assert_eq!(l0.centers.len(), 2, "fixture 前提：L0 两中枢");
        assert_eq!(
            (l0.centers[1].zd, l0.centers[1].zg),
            (180, 210),
            "fixture 前提：C1 核心 [180,210]"
        );

        // 1) 否则域点仍零一类 bit（#607 D2 语义不变——本票只建可查性，判据未动）。
        let p28 = l0
            .bsp
            .iter()
            .find(|p| p.source_index == 28)
            .expect("否则域候选点仍在 bsp 样本（零 bit 结构候选）");
        assert!(
            !p28.bits.buy1 && !p28.bits.sell1,
            "否则域点零一类 bit（D2 语义不变）"
        );

        // 2) **验收锁**：按 (level=0, source_index=28) 坐标查到否则域记录，字段逐位锁定。
        let rec = cls
            .otherwise_domain_at(0, 28)
            .expect("否则域记录按坐标可查（#885 验收）");
        assert_eq!(rec.level, 0, "level 由装配方按真实级别盖章");
        assert_eq!(rec.side, Side::Long);
        assert_eq!(
            rec.grade,
            signal::T3InCGrade::Missing(signal::T3InCGradeReason::SameDirection),
            "固定首对 (s6 Down, s7 Down) 同向 ⟹ Missing(SameDirection)"
        );
        assert_eq!(
            (
                rec.center_start_index,
                rec.center_end_index,
                rec.center_zd,
                rec.center_zg
            ),
            (12, 24, 180, 210),
            "中枢身份 = 判定中枢 C1"
        );

        // 3) LevelState 侧查询同 record；否则域迭代器覆盖全部 Missing 记录（含 s7 若 diverged）。
        assert_eq!(
            l0.first_class_grade_at(28),
            Some(rec),
            "LevelState::first_class_grade_at 与 Classification::otherwise_domain_at 同源"
        );
        let otherwise: Vec<_> = l0.otherwise_domain_records().collect();
        // 前后对照的可测面数字（钉死 fixture 产出，防静默漂移）：本 fixture L0 两个 diverged
        // 一类候选（s6/s7，C 段 episode 未回中枢 ⟹ 同一固定首对同向桶）均落否则域
        // Missing(SameDirection)，无 Present 记录 ⟹ 全部记录恰 2 条且全是否则域。
        // 改前同一 fixture 在 Classification 上的可查否则域数恒 0（无字段，只落 env 门控
        // sidecar）；改后 = 2（下界口径不变，可测面从 0 扩到 2）。
        assert_eq!(
            l0.first_class_grades.len(),
            2,
            "fixture 钉死：L0 恰 2 条分级记录（s6/s7 两候选）"
        );
        assert_eq!(
            otherwise.len(),
            2,
            "fixture 钉死：2 条记录全是否则域 Missing(SameDirection)（无 Present）"
        );
        assert!(otherwise.iter().all(
            |r| r.grade == signal::T3InCGrade::Missing(signal::T3InCGradeReason::SameDirection)
        ));
        assert_eq!(
            otherwise.iter().map(|r| r.source_index).collect::<Vec<_>>(),
            vec![28, 32],
            "记录按 source_index 升序（提取出口 canonical 排序）"
        );

        // 4) 负坐标/越界 level/非 Missing 坐标 ⟹ None。
        assert!(
            cls.otherwise_domain_at(0, 24).is_none(),
            "无该坐标的记录 ⟹ None"
        );
        assert!(
            cls.otherwise_domain_at(9, 28).is_none(),
            "越界 level ⟹ None"
        );

        // 5) 增量路径同可查（bit-exact 含新字段）：逐段前缀重放，增量 == 全量。
        let mut cache = TowerCache::new();
        for n in 1..=segments.len() {
            let prefix = ParseLayer {
                segments: Rc::new(segments[..n].to_vec()),
                merged_bars: Rc::new(bars_from_closes(&closes)),
                ..Default::default()
            };
            let (inc_cls, _inc_tower) = classify_with_tower_incremental(&prefix, &cfg, &mut cache);
            let (full_cls, _full_tower) = classify_with_tower(&prefix, &cfg);
            assert_eq!(
                inc_cls, full_cls,
                "n={n}: 增量 Classification == 全量（含 first_class_grades，#885 新字段 bit-exact）"
            );
        }
        let (inc_cls_final, _t) =
            classify_with_tower_incremental(&layer, &cfg, &mut TowerCache::new());
        let inc_rec = inc_cls_final
            .otherwise_domain_at(0, 28)
            .expect("增量路径同样按坐标可查");
        assert_eq!(inc_rec, rec, "增量/全量同一否则域记录");
    }

    /// ★classify_with_tower (i) 段导出桥——tower 非空 + depth≥1 真嵌套存在。
    ///
    /// 9 段 L0 → 3 个 L1 走势 → L2 中枢（几何路径）：tower[1] 含 sub_moves 非空的
    /// LeveledMove（RMove::Compose，depth=1 真嵌套）。坐实：导出桥正确产出真嵌套塔。
    #[test]
    fn classify_with_tower_depth_ge1_true_nesting() {
        let cfg = ThetaConfig::default();
        // 9 段：三组 up-down-up（每组 → 一个 L1 走势），三个 L1 走势外缘重叠成 L2 中枢。
        // ★task #142 延伸语义诚实重算（同 end_to_end_second_buy_via_l1_l2_geometric 推导）：三组
        // 核心分离（组B 首段 hi=115<ZD_A=120、组C 首段 lo=115>ZG_B=114 ⟹ non-extension，PDF §5
        // Step3），外缘 O_A=[110,150]/O_B=[80,125]/O_C=[112,148] 共同相交 ⟹ L2 核心 [112,125] 非空。
        let segments = vec![
            seg(Direction::Up, 0, 4, 110, 150),
            seg(Direction::Down, 4, 8, 150, 120),
            seg(Direction::Up, 8, 12, 120, 148),
            seg(Direction::Down, 12, 16, 115, 80),
            seg(Direction::Up, 16, 20, 80, 125),
            seg(Direction::Down, 20, 24, 114, 85),
            seg(Direction::Up, 24, 28, 115, 148),
            seg(Direction::Down, 28, 32, 148, 112),
            seg(Direction::Up, 32, 36, 112, 147),
        ];
        let mut closes: Vec<i64> = Vec::new();
        for i in 0..12 {
            closes.push(100 + if i % 2 == 0 { 40 } else { -40 });
        }
        for i in 0..12 {
            closes.push(100 + if i % 2 == 0 { 5 } else { -5 });
        }
        for i in 0..16 {
            closes.push(100 + if i % 2 == 0 { 3 } else { -3 });
        }
        let layer = ParseLayer {
            segments: Rc::new(segments),
            merged_bars: Rc::new(bars_from_closes(&closes)),
            ..Default::default()
        };
        let (_, tower) = classify_with_tower(&layer, &cfg);
        assert!(!tower.is_empty(), "tower 非空（至少 L0 级被处理）");
        assert!(
            tower.len() >= 2,
            "9 段 L0 → 3 个 L1 走势 → L2 中枢 ⟹ tower 至少 2 层"
        );
        // depth≥1 真嵌套：tower[1] 含 sub_moves 非空的 LeveledMove（RMove::Compose，L1 输入塔）。
        let has_true_nesting = tower[1].iter().any(|m| !m.sub_moves.is_empty());
        assert!(
            has_true_nesting,
            "tower[1] 含真嵌套 LeveledMove（sub_moves 非空，depth≥1）"
        );
    }

    /// ★classify_with_tower Classification 与 classify 同输入 bit-identical（导出不改原分类）。
    #[test]
    fn classify_with_tower_classification_equals_classify() {
        let cfg = ThetaConfig::default();
        // ★task #142 延伸语义诚实重算（同 end_to_end_second_buy_via_l1_l2_geometric 推导）：三组
        // 核心分离（组B 首段 hi=115<ZD_A=120、组C 首段 lo=115>ZG_B=114 ⟹ non-extension，PDF §5
        // Step3），外缘 O_A=[110,150]/O_B=[80,125]/O_C=[112,148] 共同相交 ⟹ L2 核心 [112,125] 非空。
        let segments = vec![
            seg(Direction::Up, 0, 4, 110, 150),
            seg(Direction::Down, 4, 8, 150, 120),
            seg(Direction::Up, 8, 12, 120, 148),
            seg(Direction::Down, 12, 16, 115, 80),
            seg(Direction::Up, 16, 20, 80, 125),
            seg(Direction::Down, 20, 24, 114, 85),
            seg(Direction::Up, 24, 28, 115, 148),
            seg(Direction::Down, 28, 32, 148, 112),
            seg(Direction::Up, 32, 36, 112, 147),
        ];
        let mut closes: Vec<i64> = Vec::new();
        for i in 0..12 {
            closes.push(100 + if i % 2 == 0 { 40 } else { -40 });
        }
        for i in 0..12 {
            closes.push(100 + if i % 2 == 0 { 5 } else { -5 });
        }
        for i in 0..16 {
            closes.push(100 + if i % 2 == 0 { 3 } else { -3 });
        }
        let layer = ParseLayer {
            segments: Rc::new(segments),
            merged_bars: Rc::new(bars_from_closes(&closes)),
            ..Default::default()
        };
        let expected = classify(&layer, &cfg);
        let (actual, _) = classify_with_tower(&layer, &cfg);
        assert_eq!(
            actual, expected,
            "classify_with_tower Classification 与 classify bit-identical（原分类不变）"
        );
    }

    // ──────────────────────────────────────────────────────────────────────
    //  增量塔 API bit-exact（task #93：incremental == 全量，逐 bar 断言）
    // ──────────────────────────────────────────────────────────────────────

    /// 构造逐段追加的合成段序列（方向交替 + 价格震荡，产足够中枢触发多级塔）。
    fn synthetic_segments(count: usize) -> Vec<Segment> {
        (0..count)
            .map(|i| {
                let dir = if i % 2 == 0 {
                    Direction::Up
                } else {
                    Direction::Down
                };
                let base = 100i64 + (i as i64) * 3;
                let swing = if i % 2 == 0 { 50 } else { -50 };
                let sp = base;
                let ep = base + swing;
                seg(dir, i * 4, i * 4 + 4, sp, ep)
            })
            .collect()
    }

    fn candidate_rich_segments(count: usize) -> Vec<Segment> {
        let mut price = 100_i64;
        let mut state = 0x9e3779b97f4a7c15_u64;
        (0..count)
            .map(|i| {
                state = state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let swing = 20 + ((state >> 32) % 90) as i64;
                let dir = if i % 2 == 0 {
                    Direction::Up
                } else {
                    Direction::Down
                };
                let start = price;
                price += match dir {
                    Direction::Up => swing,
                    Direction::Down => -swing,
                };
                seg(dir, i * 4, i * 4 + 4, start, price)
            })
            .collect()
    }

    /// ★增量塔单次 bit-exact：对完整段序列，`classify_with_tower_incremental(.., fresh cache)`
    /// 输出 == `classify_with_tower`（Classification + tower 逐字段相等）。
    ///
    /// fresh cache（空）从 consumed=0 续扫 == 全量扫描。验证增量入口的基础正确性。
    #[test]
    fn incremental_tower_fresh_cache_equals_full() {
        let cfg = ThetaConfig::default();
        for n in [3usize, 6, 9, 12, 18] {
            let segments = synthetic_segments(n);
            let closes: Vec<i64> = (0..(n * 4 + 8) as i64).map(|i| 100 + (i % 5) * 5).collect();
            let layer = ParseLayer {
                segments: Rc::new(segments),
                merged_bars: Rc::new(bars_from_closes(&closes)),
                ..Default::default()
            };

            let (full_cls, full_tower) = classify_with_tower(&layer, &cfg);
            let mut cache = TowerCache::new();
            let (inc_cls, inc_tower) = classify_with_tower_incremental(&layer, &cfg, &mut cache);

            assert_eq!(inc_cls, full_cls, "n={n}: 增量 Classification == 全量");
            assert_eq!(
                inc_tower.len(),
                full_tower.len(),
                "n={n}: 增量 tower 层数 == 全量"
            );
            for (lvl, (il, fl)) in inc_tower.iter().zip(full_tower.iter()).enumerate() {
                assert_eq!(
                    il, fl,
                    "n={n} level {lvl}: 增量 tower 级 LeveledMove 序列 == 全量"
                );
            }
        }
    }

    /// #550 主缝：新事件通道不改既有两字段，且 fresh 增量与全量逐字段一致。
    #[test]
    fn candidate_event_stream_fresh_incremental_equals_full() {
        let cfg = ThetaConfig::default();
        let segments = candidate_rich_segments(120);
        let closes: Vec<i64> = (0..=segments.last().unwrap().end_index)
            .map(|i| 100 + if i % 8 < 4 { 35 } else { -35 } + (i as i64 / 16))
            .collect();
        let layer = ParseLayer {
            segments: Rc::new(segments),
            merged_bars: Rc::new(bars_from_closes(&closes)),
            ..Default::default()
        };
        let legacy = classify_with_tower(&layer, &cfg);
        let full = classify_with_tower_events(&layer, &cfg);
        let mut cache = TowerCache::new();
        let incremental = classify_with_tower_events_incremental(&layer, &cfg, &mut cache);
        assert_eq!(full.0, legacy.0, "Classification 逐字段不动");
        assert_eq!(full.1, legacy.1, "tower 逐字段不动");
        assert_eq!(incremental, full, "候选事件流 fresh 增量≡全量");
        assert!(
            full.2.iter().any(|stream| !stream.is_empty()),
            "主缝事件流必须非空，禁止真空绿"
        );
        let pan = full
            .2
            .iter()
            .flat_map(|stream| stream.iter())
            .find(|event| event.kind == cand_event::CandidateKind::Pan)
            .expect("双域电池必须命中 Pan");
        assert_eq!(pan.key.previous_center_start, None);
        assert_eq!(pan.center_ids, None);
    }

    /// 每 key 最新 revision（因果簿终态投影的机器口径，裁定(i)）。
    fn latest_by_key(
        streams: &cand_event::CandidateStreams,
    ) -> std::collections::BTreeMap<cand_event::CandidateKey, cand_event::CandidateEvent> {
        let mut latest = std::collections::BTreeMap::new();
        for stream in streams.iter() {
            for event in stream.iter() {
                latest.insert(event.key, event.clone());
            }
        }
        latest
    }

    fn candidate_rich_layer() -> ParseLayer {
        let segments = candidate_rich_segments(120);
        let closes: Vec<i64> = (0..=segments.last().unwrap().end_index)
            .map(|i| 100 + if i % 8 < 4 { 35 } else { -35 } + (i as i64 / 16))
            .collect();
        ParseLayer {
            segments: Rc::new(segments),
            merged_bars: Rc::new(bars_from_closes(&closes)),
            ..Default::default()
        }
    }

    /// #550 主缝②：修订富集的逐段因果重放，全历史重建与跨步增量事件簿逐字段相等。
    #[test]
    fn candidate_event_stream_per_segment_full_replay_equals_incremental() {
        let cfg = ThetaConfig::default();
        let all_segments = candidate_rich_segments(120);
        let closes: Vec<i64> = (0..=(all_segments.last().unwrap().end_index + 16))
            .map(|i| 100 + if i % 8 < 4 { 35 } else { -35 } + (i as i64 / 16))
            .collect();
        let mut steps = Vec::new();
        for n in 1..=all_segments.len() {
            let prefix = all_segments[..n].to_vec();
            steps.push(prefix.clone());
            if n >= 12 {
                let mut extended = prefix;
                let tail = extended.last_mut().unwrap();
                tail.end_index += 1;
                match tail.direction {
                    Direction::Up => tail.end_price += 7,
                    Direction::Down => tail.end_price -= 7,
                }
                steps.push(extended);
            }
        }
        let mut incremental_cache = TowerCache::new();
        let mut terminal = std::rc::Rc::new(Vec::new());

        for (step_idx, segments) in steps.iter().enumerate() {
            let end = segments.last().unwrap().end_index.min(closes.len() - 1);
            let layer = ParseLayer {
                segments: Rc::new(segments.clone()),
                merged_bars: Rc::new(bars_from_closes(&closes[..=end])),
                ..Default::default()
            };
            terminal =
                classify_with_tower_events_incremental(&layer, &cfg, &mut incremental_cache).2;

            let mut replay_cache = TowerCache::new();
            let mut replay_terminal = std::rc::Rc::new(Vec::new());
            for replay_segments in &steps[..=step_idx] {
                let replay_end = replay_segments
                    .last()
                    .unwrap()
                    .end_index
                    .min(closes.len() - 1);
                let replay_layer = ParseLayer {
                    segments: Rc::new(replay_segments.clone()),
                    merged_bars: Rc::new(bars_from_closes(&closes[..=replay_end])),
                    ..Default::default()
                };
                replay_terminal =
                    classify_with_tower_events_incremental(&replay_layer, &cfg, &mut replay_cache)
                        .2;
            }
            assert_eq!(
                terminal, replay_terminal,
                "step={step_idx}: 全历史重建≡增量事件簿"
            );
        }
        let identities: std::collections::BTreeSet<_> = terminal
            .iter()
            .flat_map(|stream| stream.iter())
            .map(|event| event.key)
            .collect();
        assert!(!identities.is_empty(), "事件身份流必须非空");

        let sample = terminal
            .iter()
            .flat_map(|stream| stream.iter())
            .next()
            .expect("非空锁");
        let mut book = cand_event::CandidateEventBook::default();
        let observation = cand_event::CandidateObservation {
            key: sample.key,
            kind: sample.kind,
            center_ids: sample.center_ids,
            candidate_group_id: sample.candidate_group_id,
            pair_id: sample.pair_id,
            structural_predicates: sample.structural_predicates,
            extreme_proof: sample.extreme_proof,
            third_class_proof: sample.third_class_proof,
            interval: sample.interval,
            state: cand_event::ObservedState::Provisional,
            first_provable_at: sample.first_provable_at,
            confirmed_at: None,
        };
        book.advance(std::slice::from_ref(&observation), sample.revision_at);
        let mut changed = observation;
        changed.pair_id ^= 1;
        assert_eq!(
            book.advance(&[changed], sample.revision_at + 1)[0].revision,
            1,
            "修订富集锁：右端不动而投影变化必须追加 revision"
        );
    }

    /// #550 FNV 锁真实 classify 产出，不锁手搓 book。
    #[test]
    fn candidate_event_stream_classify_fnv1a_golden() {
        let cfg = ThetaConfig::default();
        let segments = candidate_rich_segments(120);
        let closes: Vec<i64> = (0..=segments.last().unwrap().end_index)
            .map(|i| 100 + if i % 8 < 4 { 35 } else { -35 } + (i as i64 / 16))
            .collect();
        let layer = ParseLayer {
            segments: Rc::new(segments),
            merged_bars: Rc::new(bars_from_closes(&closes)),
            ..Default::default()
        };
        let streams = classify_with_tower_events(&layer, &cfg).2;
        assert!(streams.iter().any(|stream| !stream.is_empty()));
        let digest = format!("{streams:?}")
            .bytes()
            .fold(cand_event::FNV_OFFSET_BASIS, |hash, byte| {
                (hash ^ byte as u64).wrapping_mul(cand_event::FNV_PRIME)
            });
        // #551 诚实更新（旧值 3608191067574153658）：`CandidateKey` 增 `rule_version` 分量、
        // `first_provable_at` 由 `usize` 改 `Option<usize>`（未决期不落钟）、Trend 域四态映射上线
        // （Unresolved 生产可达）、同 episode 多腿归约为每 key 一条观察。
        assert_eq!(
            digest, 3542680779063880892,
            "真实事件流漂移须诚实更新 golden"
        );
    }

    /// [`chain_book_over_prefixes`] 的**唯一变量**：`TowerCache` 是否跨前缀复用。
    ///
    /// #676-3（尾部 LOW-4 / Fowler #2）：原先两个 18 行逐字重复的 helper
    /// （`chain_book_over_prefixes` / `chain_book_over_prefixes_fresh_cache`）只差 cache 建在
    /// 循环外还是循环内。合成一个函数 + 本枚举后，「两侧唯一差别就是 cache 复用与否」这件事
    /// 由类型自证，不再靠读者逐行对比两份代码。
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum PrefixCacheReuse {
        /// 全程共享一个 cache（**因果簿**驱动：事件流 append-only，曾出现的候选身份永存）。
        SharedAcrossPrefixes,
        /// 每个前缀新建 cache（**终态窗口投影**驱动：fresh 无记忆，只投影当步终态）。
        FreshPerPrefix,
    }

    /// #641（N3）逐段前缀推进的链簿夹具（与 #550 主缝②同一段序列口径，规模收小以夹住 O(n²)）。
    ///
    /// 每一步把该前缀喂给候选事件通道，再把**当步的事件流**喂给链簿推进一次——链簿因此走的是
    /// 真实的多 `as_of` 生命史（覆盖边重算、可扩展性翻转、证伪跨越、终态封口），不是单点快照。
    ///
    /// `reuse` 选事件流的驱动语义（见 [`PrefixCacheReuse`]）；除它之外两种驱动逐字同路：同一段
    /// 前缀序列、同一 `ParseLayer` 构造、同一 `as_of`（`end`）、同一推进次数与顺序。
    fn chain_book_over_prefixes(
        segments: &[Segment],
        closes: &[i64],
        cfg: &ThetaConfig,
        reuse: PrefixCacheReuse,
    ) -> chain_cert::ChainCertificateBook {
        let mut book = chain_cert::ChainCertificateBook::default();
        let mut cache = TowerCache::new();
        for n in 1..=segments.len() {
            // #691 LOW-4：`match` 穷尽两档而非 `==` 单项判断——扩展第三档时编译红，不会静默落入
            // `SharedAcrossPrefixes` 分支（旧 `==` 写法对新增档零抵抗，见 commit message 反事实负控）。
            match reuse {
                PrefixCacheReuse::FreshPerPrefix => {
                    // 唯一的差别就这一行：丢弃上一前缀的记忆（等价于原 `_fresh_cache` 版本在循环
                    // **内**建 cache），其余一切逐字同路。
                    cache = TowerCache::new();
                }
                PrefixCacheReuse::SharedAcrossPrefixes => {}
            }
            let end = segments[n - 1].end_index.min(closes.len() - 1);
            let layer = ParseLayer {
                segments: Rc::new(segments[..n].to_vec()),
                merged_bars: Rc::new(bars_from_closes(&closes[..=end])),
                ..Default::default()
            };
            let streams = classify_with_tower_events_incremental(&layer, cfg, &mut cache).2;
            book.advance(&streams, end);
        }
        book
    }

    fn chain_fixture(count: usize) -> (Vec<Segment>, Vec<i64>) {
        let segments = candidate_rich_segments(count);
        let closes: Vec<i64> = (0..=segments.last().expect("非空").end_index)
            .map(|i| 100 + if i % 8 < 4 { 35 } else { -35 } + (i as i64 / 16))
            .collect();
        (segments, closes)
    }

    /// ★#641 链簿真双路径：同一候选事件生命史下，跨步增量宿主 ≡ 每步从零全历史重放。
    ///
    /// 输入序列只由一个共享 `TowerCache` 生成一次；两侧唯一变量是
    /// `ChainCertificateBook` 持续推进，还是每个前缀都用新簿从第一步重放。
    ///
    /// **锁定对象照实（#667 C-2 收窄）**：本测试实锁三件——簿对同一事件序列的**重建确定性**
    /// （同一函数序列、同一初值、同一顺序与次数 ⟹ 同一簿态）、**`Clone` 保真**（逐步快照与
    /// 驻留簿逐字段相等）、**无进程级/迭代序不确定性**。「跨步隐藏状态」**不在分辨力内**：
    /// 两侧推进节拍逐字相同，依赖调用次数的隐藏状态会在两侧同样累积、不产生分叉；而链簿
    /// 生命史（三只钟 / revision）按定义依赖推进节拍，节拍不同的对照物在本模块不可构造
    /// （两驱动的真实语义差由 `causal_and_terminal_projection_drives_agree_on_common_chain_keys`
    /// 固化，见分叉归因报告）。
    ///
    /// **口径降级登记（#667 C-2；#676-5 订正）**：#641 Acceptance 1 字面「全量/增量双路径逐
    /// 字节一致」在链侧不可满足（两驱动候选身份集合互有对方没有的 key，谁都不是谁的子集——
    /// 40 段夹具上曾误判为终态投影⊆因果簿，120 段上子集关系已证伪，见 #676-5/#681），验收物
    /// 降级替换为本测试 + 共有 key 一致性固化；降级在 #641 关票评论登记。
    ///
    /// **夹具规模照实**：原 40 段夹具上四条链**全部在末步一次落簿**
    /// （`distinct_as_of == {124}`），逐步重放这一侧退化成前 39 步空簿比空簿——比对恒过但没锁
    /// 住任何东西。改用 120 段（与下方 golden 同规模）后落簿跨 8 个 `as_of`
    /// （`{124, 232, 280, 320, 396, 420, 444, 464}`、38 条 revision），重放对照才有内容。
    /// 规模是**加大**取覆盖，不是缩小避分叉：40 段上双路径比对本来就全过。
    #[test]
    fn chain_certificate_book_incremental_equals_full_replay() {
        let cfg = ThetaConfig::default();
        let (segments, closes) = chain_fixture(120);
        let mut cache = TowerCache::new();
        let mut inputs = Vec::with_capacity(segments.len());
        for n in 1..=segments.len() {
            let end = segments[n - 1].end_index.min(closes.len() - 1);
            let layer = ParseLayer {
                segments: Rc::new(segments[..n].to_vec()),
                merged_bars: Rc::new(bars_from_closes(&closes[..=end])),
                ..Default::default()
            };
            let streams = classify_with_tower_events_incremental(&layer, &cfg, &mut cache).2;
            inputs.push((streams, end));
        }

        let mut incremental = chain_cert::ChainCertificateBook::default();
        let mut snapshots = Vec::with_capacity(inputs.len());
        for (streams, end) in &inputs {
            incremental.advance(streams, *end);
            snapshots.push(incremental.clone());
        }

        for (step_index, snapshot_a) in snapshots.iter().enumerate() {
            let step = step_index + 1;
            let mut replay_b = chain_cert::ChainCertificateBook::default();
            for (streams, end) in inputs.iter().take(step) {
                replay_b.advance(streams, *end);
            }

            let certificates_a = snapshot_a.certificates();
            let certificates_b = replay_b.certificates();
            assert_eq!(
                certificates_a.len(),
                certificates_b.len(),
                "step={step}: certificate 数量分叉；side_a={} side_b={}",
                certificates_a.len(),
                certificates_b.len()
            );
            for (index, (certificate_a, certificate_b)) in
                certificates_a.iter().zip(certificates_b.iter()).enumerate()
            {
                if certificate_a != certificate_b {
                    panic!(
                        "step={step}: 第一处分叉 index={index}；\
                         side_a={certificate_a:#?} side_b={certificate_b:#?}"
                    );
                }
            }
            assert_eq!(
                snapshot_a, &replay_b,
                "step={step}: certificates 已逐条一致，ChainCertificateBook 内部状态分叉"
            );
        }

        let final_book = snapshots.last().expect("fixture 必须至少产生一个输入步骤");
        let certificates = final_book.certificates();
        let certificate_count = certificates.len();
        assert!(
            certificate_count > 0,
            "非真空锁：最终 book 的 certificates 必须非空；certificates={certificate_count}"
        );
        // 多 `as_of` 生命史的非真空锁：链必须在**两个以上不同 `as_of`** 上落过簿，否则整条
        // 双路径比对退化成「单点快照比单点快照」，逐步重放这一侧等于没走。
        //
        // 090 照实：本合成夹具上每条链**一次成型**（`revision` 全 0、候选事件 `revision` 亦
        // 全 0——链首次可见即定型，同下方 golden 对 `payload_revision` 的照实声明），故此处
        // 不断言 `revision > 0`；同 key 多修订的生命史由 `chain_cert::tests` 的
        // `resident_open_chain_becomes_extendable_and_appends_a_payload_revision` 覆盖。
        let distinct_as_of: std::collections::BTreeSet<usize> = certificates
            .iter()
            .map(|certificate| certificate.revision_at)
            .collect();
        assert!(
            distinct_as_of.len() > 1,
            "生命史锁：链簿必须跨多个 as_of 落簿；distinct_as_of={distinct_as_of:?} \
             certificates={certificate_count}"
        );
        let with_edges_count = certificates
            .iter()
            .filter(|certificate| !certificate.edges.is_empty())
            .count();
        assert!(
            with_edges_count > 0,
            "边非真空锁：至少一条 certificate 的 edges 非空；\
             with_edges={with_edges_count} certificates={certificate_count}"
        );
    }

    /// [`causal_and_terminal_projection_drives_agree_on_common_chain_keys`] 的比对口径：
    /// 逐字段相同，唯一豁免 `edges[].skipped_levels[].alive_at_level` / `.inside_parent`——
    /// 这两个数字是「该级别在**候选全集**里存活多少候选」的当场快照，候选全集本就由驱动
    /// 决定（因果簿的候选全集是 append-only 累积，终态投影每步 fresh），两驱动在此项上
    /// 天然不同不代表链身份/生命史分叉。除这两个数字外，`ChainEdge` 的其余字段
    /// （`parent`/`child`/`kind`/`skipped_levels[].level`/`crossed_nodes`/`predicate_holds`/
    /// `breach`）与证书的其余字段（`key`/`extends`/`nodes`/`status`/`revision_at` 等）逐一
    /// 参与比对，不豁免。
    /// 三层全部解构开头（不用 `..`）：新增字段落进任一层的字面量都会编译红在**本函数**，把
    /// 「不豁免」的承诺从手写等式合取链的君子协定变成编译期事实（MED-2，同 diff 的
    /// `issue550_event_battery::print_chain_readout` 已用同招）。`SkippedLevel` 的
    /// `alive_at_level` / `inside_parent` 两字段仍在解构里显式列出、显式 `_` 排除——豁免理由
    /// 见本函数上方 doc（候选全集计数由驱动决定，天然不同不代表链身份分叉）。
    fn certificates_agree_ignoring_alive_candidate_universe_counts(
        causal: &chain_cert::TowerChainCertificate,
        terminal: &chain_cert::TowerChainCertificate,
    ) -> bool {
        fn skipped_level_key(level: &chain_cert::SkippedLevel) -> u32 {
            let chain_cert::SkippedLevel {
                level,
                // 豁免：候选全集计数，见外层函数 doc。
                alive_at_level: _,
                inside_parent: _,
            } = level;
            *level
        }

        fn edges_agree(a: &chain_cert::ChainEdge, b: &chain_cert::ChainEdge) -> bool {
            let chain_cert::ChainEdge {
                parent: a_parent,
                child: a_child,
                kind: a_kind,
                skipped_levels: a_skipped_levels,
                crossed_nodes: a_crossed_nodes,
                predicate_holds: a_predicate_holds,
                breach: a_breach,
            } = a;
            let chain_cert::ChainEdge {
                parent: b_parent,
                child: b_child,
                kind: b_kind,
                skipped_levels: b_skipped_levels,
                crossed_nodes: b_crossed_nodes,
                predicate_holds: b_predicate_holds,
                breach: b_breach,
            } = b;
            a_parent == b_parent
                && a_child == b_child
                && a_kind == b_kind
                && a_crossed_nodes == b_crossed_nodes
                && a_predicate_holds == b_predicate_holds
                && a_breach == b_breach
                && a_skipped_levels.len() == b_skipped_levels.len()
                && a_skipped_levels
                    .iter()
                    .zip(b_skipped_levels.iter())
                    .all(|(sa, sb)| skipped_level_key(sa) == skipped_level_key(sb))
        }

        let chain_cert::TowerChainCertificate {
            key: causal_key,
            extends: causal_extends,
            root_level: causal_root_level,
            leaf_level: causal_leaf_level,
            nodes: causal_nodes,
            edges: causal_edges,
            extendable: causal_extendable,
            status: causal_status,
            observed_at: causal_observed_at,
            closed_at: causal_closed_at,
            invalidated_at: causal_invalidated_at,
            invalidation_cause: causal_invalidation_cause,
            revision: causal_revision,
            supersedes_revision: causal_supersedes_revision,
            revision_at: causal_revision_at,
        } = causal;
        let chain_cert::TowerChainCertificate {
            key: terminal_key,
            extends: terminal_extends,
            root_level: terminal_root_level,
            leaf_level: terminal_leaf_level,
            nodes: terminal_nodes,
            edges: terminal_edges,
            extendable: terminal_extendable,
            status: terminal_status,
            observed_at: terminal_observed_at,
            closed_at: terminal_closed_at,
            invalidated_at: terminal_invalidated_at,
            invalidation_cause: terminal_invalidation_cause,
            revision: terminal_revision,
            supersedes_revision: terminal_supersedes_revision,
            revision_at: terminal_revision_at,
        } = terminal;

        causal_edges.len() == terminal_edges.len()
            && causal_edges
                .iter()
                .zip(terminal_edges.iter())
                .all(|(a, b)| edges_agree(a, b))
            && causal_key == terminal_key
            && causal_extends == terminal_extends
            && causal_root_level == terminal_root_level
            && causal_leaf_level == terminal_leaf_level
            && causal_nodes == terminal_nodes
            && causal_extendable == terminal_extendable
            && causal_status == terminal_status
            && causal_observed_at == terminal_observed_at
            && causal_closed_at == terminal_closed_at
            && causal_invalidated_at == terminal_invalidated_at
            && causal_invalidation_cause == terminal_invalidation_cause
            && causal_revision == terminal_revision
            && causal_supersedes_revision == terminal_supersedes_revision
            && causal_revision_at == terminal_revision_at
    }

    /// 因果簿驱动与终态窗口投影驱动都是 `chain_cert` 声明支持的输入语义
    /// （见 `ChainNodeStatus::Absent` 文档）。#676-5（编排 2026-07-29 重裁）：`chain_fixture(40)`
    /// 上曾实测到「terminal_projection ⊆ causal」，把这条巧合升格成了断言
    /// （`causal_book_drive_is_a_superset_of_terminal_projection_drive`）；升到 `chain_fixture(120)`
    /// 后子集关系当场破裂——`terminal_only` 非空（因果簿反而**漏**了 11 个终态投影侧独有的
    /// key），说明超集关系不是规律，是 40 段夹具的规模偏差。
    ///
    /// 归因追查（为何终态投影会出现因果簿没有的 key）另开 #681，不在本测试分辨力内。
    ///
    /// **硬锁口径二次收窄（本轮实测新发现）**：先按「共有 key 的证书逐字段相同」起草硬锁，
    /// 在 120 段夹具上实测**当场击穿**——20 个共有 key 里 4 个证书分叉，逐字段比对后分叉
    /// 精确定位在 `edges[].skipped_levels[].alive_at_level` / `.inside_parent`（候选全集里
    /// 该级别当场存活/落入父端点区间的候选计数），其余全部字段（含 `nodes`/`status`/
    /// `revision_at`/边的 `parent`/`child`/`kind`/`predicate_holds`/`breach`）逐一相同。
    /// 这两个数字统计的是候选全集，而候选全集本就由驱动决定（因果簿 append-only 累积、
    /// 终态投影每步 fresh），两驱动在此项上天然不同——不是链簿重建被破坏，是统计口径
    /// 引用了驱动相关的外部量。硬锁因此收窄为
    /// [`certificates_agree_ignoring_alive_candidate_universe_counts`]：链身份/生命史/边拓扑
    /// 逐字段相同，唯独候选全集计数不参与比对。
    ///
    /// 双向差集（`causal_only` / `terminal_only`）不是不变量，只照实登记为本夹具上的
    /// **实测形状**：`causal_only` 方向的语义差由 #551 裁定甲管；`terminal_only` 方向
    /// （即本次发现的反常子集破裂）根因在查，见 #681 与报告锚
    /// `chanlun/review-results/issue641-chain-dualpath-divergence-20260729.md`。
    #[test]
    fn causal_and_terminal_projection_drives_agree_on_common_chain_keys() {
        let cfg = ThetaConfig::default();
        let (segments, closes) = chain_fixture(120);
        let causal_book = chain_book_over_prefixes(
            &segments,
            &closes,
            &cfg,
            PrefixCacheReuse::SharedAcrossPrefixes,
        );
        let terminal_projection_book =
            chain_book_over_prefixes(&segments, &closes, &cfg, PrefixCacheReuse::FreshPerPrefix);

        let causal_heads = causal_book.heads();
        let terminal_projection_heads = terminal_projection_book.heads();

        // 硬锁（唯一不变量）：两驱动共有 key 的证书逐字段相同，唯独候选全集计数
        // （`alive_at_level` / `inside_parent`）豁免——理由见上方函数文档。
        let common_differences: Vec<_> = terminal_projection_heads
            .iter()
            .filter_map(|terminal| {
                let causal = causal_heads
                    .iter()
                    .find(|causal| causal.key == terminal.key)?;
                (!certificates_agree_ignoring_alive_candidate_universe_counts(causal, terminal))
                    .then_some((*causal, *terminal))
            })
            .collect();
        assert!(
            common_differences.is_empty(),
            "共有 key 的最新 revision 证书在候选全集计数以外的字段分叉；\
             differences={common_differences:#?}"
        );

        // 照实登记（golden 锚，非规律断言）：`chain_fixture(120)` 上的实测差集形状。
        // 40 段时 terminal_only=0（超集关系「成立」）纯属规模巧合；120 段上子集关系不成立，
        // causal_only/terminal_only 双向计数按此固定，漂移即改证据、不悄悄放宽。
        let common_count = terminal_projection_heads
            .iter()
            .filter(|terminal| causal_heads.iter().any(|causal| causal.key == terminal.key))
            .count();
        let causal_only_count = causal_heads
            .iter()
            .filter(|causal| {
                !terminal_projection_heads
                    .iter()
                    .any(|terminal| terminal.key == causal.key)
            })
            .count();
        let terminal_only_count = terminal_projection_heads
            .iter()
            .filter(|terminal| !causal_heads.iter().any(|causal| causal.key == terminal.key))
            .count();
        assert_eq!(
            (
                causal_heads.len(),
                terminal_projection_heads.len(),
                common_count
            ),
            (38, 31, 20),
            "驱动身份总数 golden 漂移（causal, terminal_projection, common）"
        );
        assert_eq!(
            causal_only_count, 18,
            "causal_only 计数 golden 漂移（#551 甲管方向）"
        );
        assert_eq!(
            terminal_only_count, 11,
            "terminal_only 计数 golden 漂移（子集关系破裂方向，根因在查 #681）"
        );
    }

    /// ★#641 FNV golden + 非真空锁：链簿产出漂移当场变红，且覆盖探针证明真走过链路径。
    #[test]
    fn chain_certificate_book_classify_fnv1a_golden() {
        chain_cert::chain_probe::reset();
        let cfg = ThetaConfig::default();
        let (segments, closes) = chain_fixture(120);
        let book = chain_book_over_prefixes(
            &segments,
            &closes,
            &cfg,
            PrefixCacheReuse::SharedAcrossPrefixes,
        );

        let heads = book.heads();
        assert!(!heads.is_empty(), "非真空锁：链身份非空");
        assert!(
            heads
                .iter()
                .any(|certificate| !certificate.edges.is_empty()),
            "非真空锁：至少一条链带边（否则边侧全部判据未被触发）"
        );
        let probe = chain_cert::chain_probe::snapshot();
        assert!(probe.birth_open + probe.birth_closed > 0, "{probe:?}");
        assert!(
            probe.skip_edges > 0,
            "非真空锁：合成 classify 面上 skip 边真被走过（{probe:?}）"
        );
        assert_eq!(
            probe.birth_invalidated, 0,
            "构造口径锁：首次观察的链不可能一出生即 Invalidated（{probe:?}）"
        );
        // ★#641 `extends` 查簿分支在主缝上真被走到（非真空）：本合成夹具的三节点链一次成型，
        // 其结构真前缀从未作为极大路径落过簿 ⟹ 全部落「未物化」格、一条 `extends` 都不写。
        // 旧实装在这一格上照写不误（幽灵前缀）——本断言即那条修复的机器载体。
        assert!(
            probe.extends_not_materialized > 0,
            "非真空锁：extends 的查簿分支必须真被走过（{probe:?}）"
        );
        assert!(
            heads
                .iter()
                .all(|certificate| certificate.extends.is_none()),
            "本夹具上无一前缀物化 ⟹ extends 全空（{probe:?}）"
        );
        // 090 照实：本合成夹具**未覆盖**到的分支（`to_invalidated` / `fact_edges` /
        // `falsified_nodes` / `payload_revision`）由 `chain_cert::tests` 的语义锁逐条覆盖；
        // 此处不为凑覆盖率而断言它们非零（合成数据规整不是缺陷）。
        let pan_rooted = heads
            .iter()
            .filter(|certificate| certificate.key.root().kind == cand_event::CandidateKind::Pan)
            .count();

        // 摘要走库内口径 `ChainCertificateBook::digest()`（#641 修复轮从 B 侧移植的库内读数口）
        // ——golden 与诊断 bin 的读数因此不可能各算各的。
        let digest = book.digest();
        // 诚实更新（旧值 9771189513849272089）：#641 修复轮给 `TowerChainCertificate` 加了
        // `invalidation_cause`、给 `ChainEdge` 加了 `breach`，两者都进 `#[derive(Debug)]`；
        // 且 `extends` 由结构派生改为查簿命中（本夹具上原值即全是未物化前缀 ⟹ 现全部为 None）。
        // 三处都改写 Debug 字节流，摘要必然翻转。裁定锚：#641 comment-5121793896。
        assert_eq!(
            digest, 9935805022530767834,
            "链簿产出漂移须诚实更新 golden（probe={probe:?} certs={} heads={} pan_rooted={pan_rooted}）",
            book.certificates().len(),
            heads.len()
        );

        // ★地板条款监视格（#641 comment-5121572134）：合成 classify 面上 `Closed` 且零链段恒 0。
        let summary = book.summarize();
        assert_eq!(
            summary.closed_with_zero_segments, 0,
            "地板条款被绕过（summary={summary:?}）"
        );
        assert_eq!(
            summary.chains,
            heads.len(),
            "库内读数口与簿的 head 集合同源"
        );
    }

    /// #551 生命史全谱合成夹具（几何逐点标注）。
    ///
    /// 中枢1 `[1000,1200]` → 大跌 → 中枢2 `[400,600]` → 过渡走势（`398→260→100`，两段同向
    /// 使三段重叠为空 ⟹ 不成中枢，走势低点 100 压到中枢3 下沿之下）→ 中枢3 `[150,250]`
    /// （`dd=100 < zd=150`，故「破核心」与「破 b 包络极值」可分离）→ C 腿1 `140→130`（破
    /// `zd=150`、未破 `b_lo=100` ⟹ 未决）→ 同 episode 反弹 `130→138`（不回中枢）→ C 腿2
    /// `138→80`（破 `b_lo` ⟹ 可证）。
    ///
    /// 中枢2 与中枢3 的 `dd/gg` 不相交 ⟹ `classify_relation` 判 `DownContinuation` ⟹ 趋势门
    /// 对中枢3 开启，C 腿才进第一类候选域。段方向不要求严格交替（parser 允许同向相邻段）。
    fn lifecycle_rich_layer() -> ParseLayer {
        let pts: [i64; 21] = [
            1000, 1200, 1000, 1200, 350, 600, 400, 600, 390, 398, 260, 100, 250, 150, 250, 150,
            250, 140, 130, 138, 80,
        ];
        let segments: Vec<Segment> = pts
            .windows(2)
            .enumerate()
            .map(|(i, w)| {
                let dir = if w[1] > w[0] {
                    Direction::Up
                } else {
                    Direction::Down
                };
                seg(dir, i * 4, i * 4 + 4, w[0], w[1])
            })
            .collect();
        let closes: Vec<i64> = (0..=segments.last().expect("非空").end_index)
            .map(|i| 100 + (i as i64 % 7) * 3)
            .collect();
        ParseLayer {
            segments: Rc::new(segments),
            merged_bars: Rc::new(bars_from_closes(&closes)),
            ..Default::default()
        }
    }

    /// 逐段前缀推进出的因果簿（真实增量宿主，保留全部 revision）。
    fn causal_book_over_prefixes(layer: &ParseLayer, cfg: &ThetaConfig) -> TowerCache {
        let mut cache = TowerCache::new();
        for n in 1..=layer.segments.len() {
            let prefix = ParseLayer {
                segments: Rc::new(layer.segments[..n].to_vec()),
                merged_bars: Rc::clone(&layer.merged_bars),
                ..Default::default()
            };
            classify_with_tower_events_incremental(&prefix, cfg, &mut cache);
        }
        cache
    }

    /// 业务载荷投影相等 —— 口径与字段表的唯一来源是
    /// [`cand_event::CandidateProjection`]（钟与 revision 计数属生命史，不入等价比较）。
    fn payload_eq(a: &cand_event::CandidateEvent, b: &cand_event::CandidateEvent) -> bool {
        a.projection() == b.projection()
    }

    /// ★#551 状态机全谱在 classify 全链上的生产可达锁：∅→Unresolved→Provisional，
    /// 身份不变、右端生长、首证钟在转 Provisional 那一刻一次写入。
    #[test]
    fn classify_chain_walks_unresolved_to_provisional_with_growth_and_clock() {
        let cfg = ThetaConfig::default();
        let layer = lifecycle_rich_layer();
        let book = causal_book_over_prefixes(&layer, &cfg)
            .candidate_book
            .streams();
        let history: Vec<_> = book
            .iter()
            .flat_map(|stream| stream.iter())
            .filter(|event| event.kind == cand_event::CandidateKind::Trend)
            .collect();
        assert_eq!(history.len(), 2, "Trend 候选须走满两段生命史，禁真空绿");
        assert_eq!(history[0].key, history[1].key, "右端不入键 ⟹ 同一身份");

        assert_eq!(history[0].state, cand_event::CandidateState::Unresolved);
        assert!(
            !history[0].structural_predicates.extreme,
            "破核心未破包络极值"
        );
        assert_eq!(history[0].first_provable_at, None, "未决期不落首证钟");
        assert_eq!(history[0].revision, 0);

        assert_eq!(history[1].state, cand_event::CandidateState::Provisional);
        assert!(history[1].structural_predicates.extreme);
        assert_eq!(history[1].revision, 1);
        assert_eq!(history[1].supersedes_revision, Some(0));
        assert!(
            history[1].interval.1 > history[0].interval.1,
            "C 段右端生长（生产触发的生长修订）"
        );
        assert_eq!(
            history[1].first_provable_at,
            Some(history[1].interval.1),
            "首证钟 = 首次全谓词成立的结构位"
        );
        assert_eq!(
            history[1].observed_at, history[0].observed_at,
            "入簿钟不后移"
        );
    }

    /// ★#551 裁定(i) 投影等价锁 —— **甲口径定稿**（编排者 2026-07-28 裁决，非上浮态）。
    ///
    /// 裁定(i) 原文「每 key 最新 revision 逐字段相等」已按出路甲收窄，本锁是该修订的机器载体：
    /// - **无条件域 = 非终态**：因果簿中 `Provisional`/`Unresolved` 的每个 key，fresh-full 侧必
    ///   存在同 key 且 `payload_eq` 逐字段成立。零豁免。
    /// - **计量域 = 终态**：`Confirmed`/`Invalidated` 的 key 上只数三类事实
    ///   （`terminal_equal` / `terminal_differ` / `terminal_absent_in_fresh`），不作相等断言。
    ///   理由：终态载荷在终态时刻**冻结**，而其上游证书仍随 bar 变，fresh-full 无状态、按当前
    ///   结构重判 ⟹ 终态域分叉是 E2E-O 终态语义历史相关性的**定义后果**，不是缺陷。
    /// - **仍是硬断言**：「fresh 有而因果簿无该 key」。它不在终态域豁免范围内 —— 豁免的是
    ///   「因果簿判了终态、fresh 侧不再产出/载荷已变」这一个方向；反过来 fresh 侧凭空多出因果簿
    ///   从未记过的 key，说明增量宿主漏记，是实装缺陷而非语义后果。
    ///
    /// 非真空三重前提：因果簿真含多 revision、fresh 非空、且 `live_checked > 0`（非终态域断言
    /// 不得真空 —— 夹具若一个非终态 key 都没有，本锁等于没锁，必须红）。
    ///
    /// **计量域的量度归属（诚实声明）**：合成夹具规整，终态计数可能全为 0，故本锁**不断言**它们
    /// 非零（断言非零会把「合成数据规整」误报成缺陷）。真实数据的非真空量度由电池给出：
    /// BTC 100k `payload_differ=25` 且 `payload_differ_live=0`（`ISSUE551_FORK` 行）。
    #[test]
    fn causal_book_terminal_projection_equals_fresh_full_stream() {
        let cfg = ThetaConfig::default();
        let layer = lifecycle_rich_layer();
        let fresh = latest_by_key(&classify_with_tower_events(&layer, &cfg).2);
        let book = causal_book_over_prefixes(&layer, &cfg)
            .candidate_book
            .streams();

        let revisions: usize = book.iter().map(|stream| stream.len()).sum();
        let terminal = latest_by_key(&book);
        assert!(
            revisions > terminal.len(),
            "非真空前提：因果簿须真含多 revision（revisions={revisions} identities={}）",
            terminal.len()
        );
        assert!(!fresh.is_empty(), "fresh 全量流非空，禁真空绿");

        // 无条件域（非终态）逐字段相等 + 计量域（终态）只数不断。遍历因果簿（正本）。
        let mut live_checked = 0usize;
        let mut terminal_equal = 0usize;
        let mut terminal_differ = 0usize;
        let mut terminal_absent_in_fresh = 0usize;
        for (key, booked) in &terminal {
            if booked.state.is_terminal() {
                match fresh.get(key) {
                    None => terminal_absent_in_fresh += 1,
                    Some(event) if payload_eq(booked, event) => terminal_equal += 1,
                    Some(_) => terminal_differ += 1,
                }
                continue;
            }
            let event = fresh.get(key).unwrap_or_else(|| {
                panic!(
                    "非终态域无条件相等：因果簿有活候选而 fresh 缺席 {key:?}\n  因果簿 {booked:?}"
                )
            });
            assert!(
                payload_eq(booked, event),
                "非终态域无条件相等（裁定(i) 甲口径）\n  因果簿 {booked:?}\n  fresh {event:?}"
            );
            live_checked += 1;
        }
        assert!(
            live_checked > 0,
            "非真空前提：非终态域断言不得真空（terminal_identities={} fresh_identities={}）",
            terminal.len(),
            fresh.len()
        );

        // 反方向仍是硬断言：fresh 侧不得出现因果簿从未记过的身份（漏记缺陷，非终态语义后果）。
        for (key, event) in &fresh {
            assert!(
                terminal.contains_key(key),
                "fresh 有而因果簿无该 key（因果簿是正本，此方向不在终态域豁免内）：{key:?}\n  fresh {event:?}"
            );
        }

        // 计量域事实（口径：只报数，不断言非零；见函数头量度归属）。
        assert_eq!(
            terminal_equal + terminal_differ + terminal_absent_in_fresh + live_checked,
            terminal.len(),
            "计量口径分解须覆盖因果簿全部身份：live={live_checked} \
             terminal_equal={terminal_equal} terminal_differ={terminal_differ} \
             terminal_absent_in_fresh={terminal_absent_in_fresh} \
             terminal_identities={} fresh_identities={}",
            terminal.len(),
            fresh.len()
        );
    }

    /// ★#551 裁定(i) 失效分叉锁 —— **甲口径定稿**（编排者 2026-07-28 裁决，非上浮态）。
    ///
    /// 与 `causal_book_terminal_projection_equals_fresh_full_stream` 同一口径的失效侧：
    /// - **无条件域 = 非终态禁向**：不存在这样的 key —— 因果簿中为 `Provisional`/`Unresolved`
    ///   而 fresh 侧缺席或载荷不等。**本夹具上该分支真空**（诚实声明）：L0 塌空后因果簿里的活
    ///   候选被当场判 `Invalidated`，故非终态集合为空，该断言在此缝上走不到。非真空由锁一的
    ///   `live_checked > 0` 承担。真空不是删掉它的理由 —— 口径分支必须在两枚锁上同构存在，
    ///   否则夹具一变（塌空前提被替换）就没有任何东西守住这个方向。
    /// - **计量域 = 终态**：原「禁止方向（fresh 在产而因果簿已判终态 = 复活）必须为空」的**禁令
    ///   已按裁决取消**，改为计量 `revived_terminal`。复活型分叉是 E2E-O 终态语义历史相关性的
    ///   定义后果，不再是违规。本夹具（塌空后**未回长**）上它恒 0；回长场景的非零量度由
    ///   `escalated_upstream_regrowth_after_collapse_revival_fork_metered` 单独计量。
    ///   同域的 `fork_causal_only_terminal`（因果簿终态在案而 fresh 无该流）一并报数。
    ///
    /// 非真空前提保留：塌空须真产生 `Invalidated`、塌空后 fresh 须为空 —— 二者保证本锁走的是
    /// 真实的中途消失，而非空跑。
    #[test]
    fn invalidation_fork_only_points_from_causal_book_to_absent_fresh_stream() {
        let cfg = ThetaConfig::default();
        let layer = lifecycle_rich_layer();
        let mut cache = causal_book_over_prefixes(&layer, &cfg);
        let collapsed = ParseLayer {
            segments: Rc::new(Vec::new()),
            merged_bars: Rc::clone(&layer.merged_bars),
            ..Default::default()
        };
        classify_with_tower_events_incremental(&collapsed, &cfg, &mut cache);
        let terminal = latest_by_key(&cache.candidate_book.streams());
        let fresh = latest_by_key(&classify_with_tower_events(&collapsed, &cfg).2);

        let dead: Vec<_> = terminal
            .values()
            .filter(|event| event.state == cand_event::CandidateState::Invalidated)
            .collect();
        assert!(!dead.is_empty(), "非真空前提：塌空须真产生 Invalidated");
        assert!(fresh.is_empty(), "塌空后 fresh-full 无任何候选流");

        // 无条件域：非终态禁向。本夹具上 live_scanned 恒 0（见函数头真空声明）。
        let mut live_scanned = 0usize;
        for (key, booked) in &terminal {
            if booked.state.is_terminal() {
                continue;
            }
            live_scanned += 1;
            let event = fresh.get(key).unwrap_or_else(|| {
                panic!("非终态禁向：因果簿有活候选而 fresh 缺席 {key:?}\n  因果簿 {booked:?}")
            });
            assert!(
                payload_eq(booked, event),
                "非终态禁向：活候选载荷须逐字段相等\n  因果簿 {booked:?}\n  fresh {event:?}"
            );
        }

        // 计量域（**非禁令**）：终态 key 在 fresh 侧的两种去向。
        let revived_terminal = fresh
            .keys()
            .filter(|key| {
                terminal
                    .get(*key)
                    .is_some_and(|event| event.state.is_terminal())
            })
            .count();
        let fork_causal_only_terminal = terminal
            .iter()
            .filter(|(key, event)| event.state.is_terminal() && !fresh.contains_key(*key))
            .count();
        // 计量口径分解（覆盖因果簿全部身份，报出实值）；本夹具未回长 ⟹ revived_terminal=0，
        // 但该 0 是**观测结果**，不是禁令 —— 回长场景下它为 1 且被上面点名的那枚锁计量。
        assert_eq!(
            live_scanned + revived_terminal + fork_causal_only_terminal,
            terminal.len(),
            "计量口径分解：live_scanned={live_scanned} revived_terminal={revived_terminal} \
             fork_causal_only_terminal={fork_causal_only_terminal} \
             terminal_identities={} fresh_identities={}",
            terminal.len(),
            fresh.len()
        );
    }

    /// ★#551：终态候选的载荷在确认时刻冻结 ⟹ 与 fresh-full 的当前重判必然分叉。
    ///
    /// 本锁**不掩盖**该分叉，而是把它的边界机器化：差异只允许落在终态候选上（`Confirmed`/
    /// `Invalidated`），非终态候选必须逐字段相等。Pan 域一入簿即 `Confirmed`（#550 决策），其
    /// C 段区间此后仍随 bar 推进而变——这曾是 E2E-O「终态不改写」与裁定(i)「全 key 等价」在
    /// 「终态候选上游载荷仍会变」下的不可弥合张力，**已裁决：甲**（编排者 2026-07-28，见
    /// `chanlun/review-results/issue551-t2-impl-20260728.md` §五）。裁决把裁定(i) 收窄到非终态
    /// 域，本锁的断言形状恰是甲口径本身 ⟹ 断言逐字未动，只是身份从「矛盾边界锁」变为
    /// **甲口径的机器载体**。
    ///
    /// **口径边界（诚实声明）**：本 lib 锁只固定不变式的**方向**（`differ_live == 0`）。合成
    /// 夹具规整、终态候选的上游证书不再变动，`differ_terminal` 恒为 0，故该分支在本缝上真空。
    /// 非真空量度由真实数据电池给出：BTC 100k 实测 `payload_differ=25` 且 `payload_differ_live=0`
    /// （`ISSUE551_FORK` 行），即 25 个差异 100% 落在终态候选上。
    #[test]
    fn projection_divergence_is_confined_to_terminal_candidates() {
        let cfg = ThetaConfig::default();
        let layer = candidate_rich_layer();
        let fresh = latest_by_key(&classify_with_tower_events(&layer, &cfg).2);
        let terminal = latest_by_key(
            &causal_book_over_prefixes(&layer, &cfg)
                .candidate_book
                .streams(),
        );

        let mut differ_terminal = 0usize;
        let mut differ_live = 0usize;
        for (key, event) in &fresh {
            let Some(booked) = terminal.get(key) else {
                continue;
            };
            if payload_eq(booked, event) {
                continue;
            }
            if booked.state.is_terminal() {
                differ_terminal += 1;
            } else {
                differ_live += 1;
            }
        }
        assert_eq!(
            differ_live, 0,
            "非终态候选的投影必须逐字段相等（裁定(i) 在活假设域上无条件成立）"
        );
        // `differ_terminal` 在合成夹具上恒 0（见函数头口径边界），此处只报数不断言非零——
        // 断言它 > 0 会把「合成数据规整」误报成缺陷。真实数据的非零量度在电池 bin。
        assert!(
            differ_terminal < fresh.len(),
            "终态分叉不应吞掉全部身份（合成夹具期望 0，真实数据期望少数）"
        );
    }

    /// ★#551 复活型分叉的**计量锁**（甲口径定稿）。
    ///
    /// **命名已按甲口径订正**（编排者 2026-07-28 裁决）：原上浮期名
    /// `escalated_upstream_regrowth_after_collapse_forks_in_forbidden_direction` →
    /// 现名 `…_revival_fork_metered`。该方向已不再是「禁止方向」，旧名与代码实际锁住的事实
    /// 名实不符 = 声明膨胀（090 严格性），故改名，而非靠 doc 反向纠正。
    /// 名册差集对账不靠「不改名」偿付，改由**显式声明 rename 非删增**偿付：改名 commit 的正文
    /// 记录「删旧名 1 / 增新名 1、二者为同一测试的改名两端、测试体与断言消息逐字未动」，
    /// 对账时按此把这一删一增抵消，不计入测试增删。
    ///
    /// **改名与断言消息的不对称（诚实声明）**：函数名改了，下方断言消息中的「禁止方向」
    /// 「矛盾已上浮」**逐字未改**。二者性质不同 —— 函数名是本测试对外的身份标识，不进入任何
    /// 断言比较，改它只改称谓；断言消息是断言失败时打印的文本，属于本锁固定下来的现场记录，
    /// 改动它会改变本锁锁住的事实边界（裁决要求断言逐字不动）。故：称谓按裁决后的甲口径订正，
    /// 断言文本保持上浮期原貌，其定性一律以本 doc 为准 —— 消息里的「禁止方向」读作
    /// 「原判为禁止、现判为计量的那个方向」。
    ///
    /// 上游塌空后再回长到**逐字段相同**的结构时：因果簿按 E2E-O 判该 key 终态（`Invalidated`
    /// 不复活），而 fresh-full 无状态、只看当下，会重新产出同一个 key —— 复活型分叉在机制上可达。
    ///
    /// 根因不是实装缺陷，是两条规则的语义差：E2E-O 的终态语义**历史相关**（一旦终态永远终态），
    /// fresh-full 的语义**历史无关**（只反映当前结构）。**已裁决：甲**（编排者 2026-07-28，报告
    /// §五）——裁定(i) 收窄为非终态域逐字段相等；终态域（含本处的复活型分叉）是该语义差的
    /// **定义后果**，接受并计量，不再作禁止断言。乙（改 Pan 状态映射）与丙（key 补上游代次）
    /// 未被采纳，代价见报告 §五 对照表。
    ///
    /// 于是本锁从「矛盾边界锁」转为**计量锁**：锁住的是该分叉恰落在终态域、数量恰为 1、且两侧
    /// 状态如实可解释（因果簿 `Invalidated` / fresh `Provisional`）。断言逐字未动 —— 它锁的事实
    /// 没变，变的只是这些事实的定性。
    ///
    /// 生产可达性（保留）：BTC 100k 实测 `invalidations=0`，塌空—回长从未发生 ⟹ 该分叉当前
    /// **生产不可达**。
    #[test]
    fn escalated_upstream_regrowth_after_collapse_revival_fork_metered() {
        let cfg = ThetaConfig::default();
        let layer = lifecycle_rich_layer();
        let mut cache = causal_book_over_prefixes(&layer, &cfg);
        let collapsed = ParseLayer {
            segments: Rc::new(Vec::new()),
            merged_bars: Rc::clone(&layer.merged_bars),
            ..Default::default()
        };
        classify_with_tower_events_incremental(&collapsed, &cfg, &mut cache);
        classify_with_tower_events_incremental(&layer, &cfg, &mut cache);

        let terminal = latest_by_key(&cache.candidate_book.streams());
        let fresh = latest_by_key(&classify_with_tower_events(&layer, &cfg).2);
        let revived: Vec<_> = fresh
            .keys()
            .filter(|key| {
                terminal.get(key).map(|event| event.state)
                    == Some(cand_event::CandidateState::Invalidated)
            })
            .collect();
        assert_eq!(
            revived.len(),
            1,
            "锁定当前语义：塌空—回长恰产生 1 个禁止方向的分叉（矛盾已上浮，见函数头）"
        );
        assert_eq!(
            terminal[revived[0]].state,
            cand_event::CandidateState::Invalidated,
            "因果簿侧：终态不复活（E2E-O 成立）"
        );
        assert_eq!(
            fresh[revived[0]].state,
            cand_event::CandidateState::Provisional,
            "fresh 侧：历史无关，重新产出同一 key（裁定(i) 的禁止方向）"
        );
    }

    /// ★#551：L0 塌空的 bar 必须**当场**把在案活候选判 `Invalidated`，不推迟到下一非空 bar。
    #[test]
    fn empty_l0_bar_invalidates_live_candidates_without_delay() {
        let cfg = ThetaConfig::default();
        let layer = lifecycle_rich_layer();
        let mut cache = TowerCache::new();
        let live = classify_with_tower_events_incremental(&layer, &cfg, &mut cache).2;
        // 只有**非终态**候选会因观察缺席而失效；`Confirmed`/`Invalidated` 是终态，按 E2E-O
        // 不复活也不再改写。
        let live_keys: Vec<_> = latest_by_key(&live)
            .into_iter()
            .filter(|(_, event)| !event.state.is_terminal())
            .map(|(key, _)| key)
            .collect();
        assert!(!live_keys.is_empty(), "非真空前提：塌空前须有非终态活候选");

        cand_event::event_probe::reset();
        let collapsed = ParseLayer {
            segments: Rc::new(Vec::new()),
            merged_bars: Rc::clone(&layer.merged_bars),
            ..Default::default()
        };
        let after =
            latest_by_key(&classify_with_tower_events_incremental(&collapsed, &cfg, &mut cache).2);
        for key in &live_keys {
            assert_eq!(
                after.get(key).map(|event| event.state),
                Some(cand_event::CandidateState::Invalidated),
                "L0 塌空 ⟹ 候选身份消失 ⟹ 当场判终态（禁延迟到下一非空 bar）{key:?}"
            );
        }
        assert_eq!(
            cand_event::event_probe::snapshot().invalidated_absent as usize,
            live_keys.len(),
            "走的是既有「观察缺席 ⟹ Invalidated」单一路径"
        );
    }

    /// ★增量塔逐段追加 bit-exact（#93 核心铁律）：模拟 per-bar substrate 逐段追加，
    /// 每步断言 `classify_with_tower_incremental(layer[..=i], cache)` ==
    /// `classify_with_tower(layer[..=i])`（Classification + tower 逐字段相等）。
    ///
    /// 这是增量塔的真实使用场景——段账本单调增长，cache 跨步复用前级 confirmed 前缀。
    /// 任何 resume bit-exact 破裂、跨级传播错误、裁决漂移都会在此捕获。
    #[test]
    fn incremental_tower_per_segment_append_matches_full() {
        let cfg = ThetaConfig::default();
        let all_segments = synthetic_segments(21);
        let closes: Vec<i64> = (0..100).map(|i| 100 + (i % 7) * 4).collect();

        let mut cache = TowerCache::new();
        for n in 1..=all_segments.len() {
            let segments = all_segments[..n].to_vec();
            let layer = ParseLayer {
                segments: Rc::new(segments),
                merged_bars: Rc::new(bars_from_closes(&closes)),
                ..Default::default()
            };

            // 全量基准。
            let (full_cls, full_tower) = classify_with_tower(&layer, &cfg);
            // 增量（cache 跨步复用）。
            let (inc_cls, inc_tower) = classify_with_tower_incremental(&layer, &cfg, &mut cache);

            assert_eq!(
                inc_cls, full_cls,
                "n={n}: 增量 Classification != 全量（bit-exact 破裂）"
            );
            assert_eq!(
                inc_tower.len(),
                full_tower.len(),
                "n={n}: 增量 tower 层数 != 全量"
            );
            for (lvl, (il, fl)) in inc_tower.iter().zip(full_tower.iter()).enumerate() {
                assert_eq!(
                    il, fl,
                    "n={n} level {lvl}: 增量 tower 级 LeveledMove != 全量（真 subs 嵌套破裂）"
                );
            }
        }
    }

    /// ★段账本回缩退化 bit-exact：模拟 parser 回撤最后一段（非单调追加），
    /// `cache` 自动检测回缩 ⟹ 清空 + 全量重扫 ⟹ 仍 bit-exact（退化不破坏正确性）。
    #[test]
    fn incremental_tower_shrink_falls_back_to_full() {
        let cfg = ThetaConfig::default();
        let all_segments = synthetic_segments(12);
        let closes: Vec<i64> = (0..80).map(|i| 100 + (i % 6) * 4).collect();

        let mut cache = TowerCache::new();
        // 先追加到 12 段。
        let layer_full = ParseLayer {
            segments: Rc::new(all_segments.clone()),
            merged_bars: Rc::new(bars_from_closes(&closes)),
            ..Default::default()
        };
        let _ = classify_with_tower_incremental(&layer_full, &cfg, &mut cache);
        // 回缩到 8 段（parser 回撤）。
        let layer_shrink = ParseLayer {
            segments: Rc::new(all_segments[..8].to_vec()),
            merged_bars: Rc::new(bars_from_closes(&closes)),
            ..Default::default()
        };
        let (full_cls, full_tower) = classify_with_tower(&layer_shrink, &cfg);
        let (inc_cls, inc_tower) = classify_with_tower_incremental(&layer_shrink, &cfg, &mut cache);
        assert_eq!(inc_cls, full_cls, "回缩退化：增量 Classification == 全量");
        assert_eq!(inc_tower, full_tower, "回缩退化：增量 tower == 全量");
    }

    /// ★B2 真产出 bit-exact：增量塔在产 B2 的真实结构（9 段三组 up-down-up）下，
    /// `classify_with_tower_incremental` 产出的 B2 与全量 `classify_with_tower` bit-identical——
    /// 验证增量 compose 的真 Fugue 547（subs 真 Compose，B2 真可产，禁级别差伪造）。
    #[test]
    fn incremental_tower_preserves_b2_second_buy() {
        let cfg = ThetaConfig::default();
        // ★task #142 延伸语义诚实重算（同 end_to_end_second_buy_via_l1_l2_geometric 推导）：三组
        // 核心分离（组B 首段 hi=115<ZD_A=120、组C 首段 lo=115>ZG_B=114 ⟹ non-extension，PDF §5
        // Step3），外缘 O_A=[110,150]/O_B=[80,125]/O_C=[112,148] 共同相交 ⟹ L2 核心 [112,125] 非空。
        let segments = vec![
            seg(Direction::Up, 0, 4, 110, 150),
            seg(Direction::Down, 4, 8, 150, 120),
            seg(Direction::Up, 8, 12, 120, 148),
            seg(Direction::Down, 12, 16, 115, 80),
            seg(Direction::Up, 16, 20, 80, 125),
            seg(Direction::Down, 20, 24, 114, 85),
            seg(Direction::Up, 24, 28, 115, 148),
            seg(Direction::Down, 28, 32, 148, 112),
            seg(Direction::Up, 32, 36, 112, 147),
        ];
        let mut closes: Vec<i64> = Vec::new();
        for i in 0..12 {
            closes.push(100 + if i % 2 == 0 { 40 } else { -40 });
        }
        for i in 0..12 {
            closes.push(100 + if i % 2 == 0 { 5 } else { -5 });
        }
        for i in 0..16 {
            closes.push(100 + if i % 2 == 0 { 3 } else { -3 });
        }
        let layer = ParseLayer {
            segments: Rc::new(segments),
            merged_bars: Rc::new(bars_from_closes(&closes)),
            ..Default::default()
        };

        let (full_cls, _) = classify_with_tower(&layer, &cfg);
        let mut cache = TowerCache::new();
        let (inc_cls, _) = classify_with_tower_incremental(&layer, &cfg, &mut cache);

        // 全量产 1 个 B2（见 end_to_end_second_buy_via_l1_l2_geometric），增量须 bit-identical。
        let full_b2: Vec<_> = full_cls.levels[1]
            .bsp
            .iter()
            .filter(|p| p.bits.buy2)
            .collect();
        let inc_b2: Vec<_> = inc_cls.levels[1]
            .bsp
            .iter()
            .filter(|p| p.bits.buy2)
            .collect();
        assert_eq!(
            inc_b2.len(),
            full_b2.len(),
            "增量塔 B2 数量 == 全量（真 Fugue 547 保留）"
        );
        assert_eq!(
            inc_b2.len(),
            1,
            "增量塔仍真产 B2（subs 真 Compose，非级别差伪造）"
        );
        assert_eq!(
            inc_b2[0].source_index, full_b2[0].source_index,
            "B2 source_index bit-exact"
        );
        assert_eq!(
            inc_cls, full_cls,
            "增量塔完整 Classification == 全量（含 B2 BSP）"
        );
    }

    /// ★codex 反例（cascade reset 完备性，L1 构造）：L0 frontier 末段**内点改写**——
    /// 上级投影 `UnitRange(lo,hi)` bit-identical 但底层 `sub_moves` 变。
    ///
    /// 场景：9 段三组（task #142 核心分离 fixture），末段 seg[8] `Up[32,36]` end_price 从 147 改写
    /// 为 140。140 是组 C 外缘内点（组 C max-hi=148(seg[6]/seg[7]) / min-lo=112(seg[7]/seg[8].sp)
    /// 不变）⟹ L0 该窗口中枢 gg/dd 不变 ⟹ L1 输入投影 `project_to_units` bit-identical。但 seg[8]
    /// 的 lo/hi 从 [112,147] 变 [112,140] ⟹ L0 upper_moves[2].sub_moves[2] 深嵌套坐标变。
    ///
    /// 旧守卫（仅比对本级 `project_to_units` 投影）：L0 reset 正确，但 L1 frontier_mutated=false
    /// 漏 reset ⟹ `cache.levels[1].upper_moves` 深嵌套 sub_moves 陈旧（仍 [112,147]）+ BSP memo
    /// （key 仅三长度）复用陈旧 BSP ⟹ 与全量发散。
    /// cascade reset 修复：L0 变异 → 强制 reset L1+（无条件跟随下级），深嵌套 sub_moves 重建为 [112,140]。
    ///
    /// **L1**（合成构造，验证管线完备性，非真实数据假设——formalization-validity-domain 231号）。
    #[test]
    fn cascade_reset_on_frontier_interior_rewrite() {
        let cfg = ThetaConfig::default();
        // ★task #142 延伸语义诚实重算：三组核心分离 fixture（同 end_to_end_second_buy_via_l1_l2_geometric
        // 推导——组B 首段 hi=115<ZD_A=120、组C 首段 lo=115>ZG_B=114 ⟹ non-extension，PDF §5 Step3）。
        let base = vec![
            seg(Direction::Up, 0, 4, 110, 150),
            seg(Direction::Down, 4, 8, 150, 120),
            seg(Direction::Up, 8, 12, 120, 148),
            seg(Direction::Down, 12, 16, 115, 80),
            seg(Direction::Up, 16, 20, 80, 125),
            seg(Direction::Down, 20, 24, 114, 85),
            seg(Direction::Up, 24, 28, 115, 148),
            seg(Direction::Down, 28, 32, 148, 112),
            seg(Direction::Up, 32, 36, 112, 147), // v1 末段
        ];
        let mut closes: Vec<i64> = Vec::new();
        for i in 0..12 {
            closes.push(100 + if i % 2 == 0 { 40 } else { -40 });
        }
        for i in 0..12 {
            closes.push(100 + if i % 2 == 0 { 5 } else { -5 });
        }
        for i in 0..16 {
            closes.push(100 + if i % 2 == 0 { 3 } else { -3 });
        }
        let merged = Rc::new(bars_from_closes(&closes));

        // v1: 末段 end_price=147。
        let layer_v1 = ParseLayer {
            segments: Rc::new(base.clone()),
            merged_bars: merged.clone(),
            ..Default::default()
        };
        // v2: 仅末段 end_price 改写 147→140（组 C 外缘内点，L1 投影不变，L0 sub_moves 变）。
        let mut v2_segs = base.clone();
        v2_segs[8].end_price = 140;
        let layer_v2 = ParseLayer {
            segments: Rc::new(v2_segs),
            merged_bars: merged.clone(),
            ..Default::default()
        };

        // 前提自检（codex 反例成立的必要条件）：L1 投影输入 v1==v2 bit-identical（守卫看不到变异），
        // 但 L0 末段 sub_moves 已变（147→140）。若此前提不成立，本测试不构成反例。
        let mk_l1_units = |segs: &Rc<Vec<Segment>>| {
            let l0_units: Vec<UnitRange> = segs.iter().map(segment_to_unit).collect();
            let moves_l0: Vec<LeveledMove> = l0_units
                .iter()
                .enumerate()
                .map(|(i, u)| {
                    LeveledMove::from_unit(
                        u,
                        recursive_tower::ElementId {
                            level: 0,
                            ordinal: i as u64,
                        },
                    )
                })
                .collect();
            let (c, upper, _) = recursive_tower::compose_level(&l0_units, &moves_l0, true, 1);
            recursive_tower::project_to_units(&upper, &decompose::decompose(&c))
        };
        assert_eq!(
            mk_l1_units(&layer_v1.segments),
            mk_l1_units(&layer_v2.segments),
            "前提：L1 投影输入 v1==v2（守卫的本级投影比对看不到此变异）"
        );

        // 共享 cache：先喂 v1（缓存 L0/L1），再喂 v2（frontier 内点改写）——模拟 per-bar 末段重划。
        let mut cache = TowerCache::new();
        let _ = classify_with_tower_incremental(&layer_v1, &cfg, &mut cache);
        let (inc_v2, inc_tower_v2) = classify_with_tower_incremental(&layer_v2, &cfg, &mut cache);
        let (full_v2, full_tower_v2) = classify_with_tower(&layer_v2, &cfg);

        // ★核心断言（latent 陈旧检测，非仅返回值）：cache 内 L1 深嵌套 sub_moves 末段坐标必须 == v2
        // 的 [112,140]。返回的 Classification/tower 不消费 cache 内 L1 upper_moves 的深 subs（tower 用
        // 新鲜 moves_tower 快照），故陈旧在返回值里 latent——但它喂 BSP（extract_second_for_level）+
        // 下一 bar 的 L2 投影。直接断言 cache 深 subs，捕获 latent 陈旧（640：不靠返回值碰巧相等）。
        // 推导（task #142 fixture）：v2 seg[8] = Up 112→140 ⟹ 区间 [112,140]（v1 为 [112,147]）。
        let l0_seg8_full = classify_with_tower(&layer_v2, &cfg).1[0]
            .last()
            .unwrap()
            .rmove
            .clone();
        assert_eq!(
            l0_seg8_full,
            descend::RMove::Segment {
                direction: Direction::Up,
                lo: 112,
                hi: 140
            },
            "前提：v2 全量 L0 末段 == [112,140]"
        );
        // cache.L1.upper_moves[0].sub_moves[2](groupC).sub_moves[2](seg[8]) 应 == [112,140]。
        let l1_deep = &cache.levels[1].upper_moves[0].sub_moves[2].sub_moves[2].rmove;
        assert_eq!(*l1_deep, descend::RMove::Segment { direction: Direction::Up, lo: 112, hi: 140 },
            "cascade: cache L1 深嵌套 seg[8] == v2 [112,140]（陈旧则 [112,147]——L1 漏 cascade reset）");

        // 返回值也须 bit-exact（cascade 后 L1 重建，tower/Classification 全对齐）。
        assert_eq!(inc_v2, full_v2, "cascade: v2 增量 Classification == 全量");
        assert_eq!(
            inc_tower_v2.len(),
            full_tower_v2.len(),
            "cascade: tower 层数 == 全量"
        );
        for (lvl, (il, fl)) in inc_tower_v2.iter().zip(full_tower_v2.iter()).enumerate() {
            assert_eq!(il, fl, "cascade: level {lvl} LeveledMove == 全量");
        }
    }

    /// ★on2w2 G7（epoch 递增覆盖性）：L0 尾段重划场景（= A12 bar3020 假命中场景）**必须** bump
    /// forest_epoch。复用 `cascade_reset_on_frontier_interior_rewrite` 的 v1→v2 fixture——v2 仅末段
    /// end_price 147→140（组 C 外缘内点，L1 投影 bit-identical，L0 sub_moves 变）。这是 gen 快路当年
    /// 假命中返陈旧森林的精确形态（TowerCache::generation 靠 l0_is_root blunt 兜底才没漏）。
    ///
    /// 断言：喂 v1 后喂 v2，forest_epoch **严格递增**（若 epoch 漏 bump ⟹ 下游 TreeCache 假命中返
    /// v1 陈旧森林 ⟹ bit-exact 破裂）。E1 逐值判据在此场景 [reuse..] 尾段值变（147→140）⟹ dirty。
    #[test]
    fn forest_epoch_bumps_on_l0_tail_redivision() {
        let cfg = ThetaConfig::default();
        let base = vec![
            seg(Direction::Up, 0, 4, 110, 150),
            seg(Direction::Down, 4, 8, 150, 120),
            seg(Direction::Up, 8, 12, 120, 148),
            seg(Direction::Down, 12, 16, 115, 80),
            seg(Direction::Up, 16, 20, 80, 125),
            seg(Direction::Down, 20, 24, 114, 85),
            seg(Direction::Up, 24, 28, 115, 148),
            seg(Direction::Down, 28, 32, 148, 112),
            seg(Direction::Up, 32, 36, 112, 147), // v1 末段
        ];
        let mut closes: Vec<i64> = Vec::new();
        for i in 0..12 {
            closes.push(100 + if i % 2 == 0 { 40 } else { -40 });
        }
        for i in 0..12 {
            closes.push(100 + if i % 2 == 0 { 5 } else { -5 });
        }
        for i in 0..16 {
            closes.push(100 + if i % 2 == 0 { 3 } else { -3 });
        }
        let merged = Rc::new(bars_from_closes(&closes));
        let layer_v1 = ParseLayer {
            segments: Rc::new(base.clone()),
            merged_bars: merged.clone(),
            ..Default::default()
        };
        let mut v2_segs = base.clone();
        v2_segs[8].end_price = 140; // L0 尾段重划（内点改写）——A12 bar3020 假命中场景。
        let layer_v2 = ParseLayer {
            segments: Rc::new(v2_segs),
            merged_bars: merged.clone(),
            ..Default::default()
        };

        let mut cache = TowerCache::new();
        let _ = classify_with_tower_incremental(&layer_v1, &cfg, &mut cache);
        let epoch_after_v1 = cache.forest_epoch();
        let _ = classify_with_tower_incremental(&layer_v2, &cfg, &mut cache);
        let epoch_after_v2 = cache.forest_epoch();
        assert!(
            epoch_after_v2 > epoch_after_v1,
            "L0 尾段重划（147→140）必须 bump forest_epoch（漏 bump ⟹ TreeCache 假命中返陈旧森林）：\
             v1_epoch={epoch_after_v1} v2_epoch={epoch_after_v2}"
        );
    }

    /// ★on2w2：无字节变更 bar（inclusion-only，L0 尾段值不变）**不** bump forest_epoch——O(n²) 消除
    /// 的机制（bump 率贴近 forest 真变率而非每 bar）。喂完全相同的 layer 两次，第二次 epoch 不应变。
    #[test]
    fn forest_epoch_stable_on_no_change() {
        let cfg = ThetaConfig::default();
        let base = vec![
            seg(Direction::Up, 0, 4, 110, 150),
            seg(Direction::Down, 4, 8, 150, 120),
            seg(Direction::Up, 8, 12, 120, 148),
            seg(Direction::Down, 12, 16, 115, 80),
            seg(Direction::Up, 16, 20, 80, 125),
            seg(Direction::Down, 20, 24, 114, 85),
            seg(Direction::Up, 24, 28, 115, 148),
        ];
        let mut closes: Vec<i64> = Vec::new();
        for i in 0..28 {
            closes.push(100 + if i % 2 == 0 { 30 } else { -30 });
        }
        let merged = Rc::new(bars_from_closes(&closes));
        let layer = ParseLayer {
            segments: Rc::new(base.clone()),
            merged_bars: merged.clone(),
            ..Default::default()
        };

        let mut cache = TowerCache::new();
        let _ = classify_with_tower_incremental(&layer, &cfg, &mut cache);
        let e1 = cache.forest_epoch();
        // 完全相同输入再喂一次——无任何塔字节变更 ⟹ epoch 不应 bump。
        let _ = classify_with_tower_incremental(&layer, &cfg, &mut cache);
        let e2 = cache.forest_epoch();
        assert_eq!(e1, e2, "无变更 bar 不应 bump forest_epoch（否则退化每 bar bump = O(n²) 未消除）：e1={e1} e2={e2}");
    }

    // ──────────────────────────────────────────────────────────────────────
    //  标度验证（task #93：per-bar 累积成本，增量 vs 全量）
    // ──────────────────────────────────────────────────────────────────────

    /// ★合成标度：per-bar 段追加累积成本，增量 exp 显著 < 全量 exp。
    ///
    /// 全量 `classify_with_tower` 每步从 0 重扫塔 ⟹ 累积 O(Σ i) ≈ O(N²)，exp≈2。
    /// 增量 `classify_with_tower_incremental` 每步续扫 tail ⟹ 累积 O(Σ tail) ≈ O(N)，exp≈1。
    /// 合成段序列单调追加（增量有效域）；此测试 always-run（无需真实数据）。
    #[test]
    fn incremental_tower_scaling_dominates_full_synthetic() {
        let cfg = ThetaConfig::default();
        let sizes = [100usize, 200, 400];
        let mut full_times = Vec::new();
        let mut inc_times = Vec::new();

        for &n in &sizes {
            let all_segments = synthetic_segments(n);
            let closes: Vec<i64> = (0..(n * 4 + 8) as i64).map(|i| 100 + (i % 7) * 4).collect();

            // 全量 per-bar 累积。
            let t0 = std::time::Instant::now();
            for k in 1..=n {
                let layer = ParseLayer {
                    segments: Rc::new(all_segments[..k].to_vec()),
                    merged_bars: Rc::new(bars_from_closes(&closes)),
                    ..Default::default()
                };
                let _ = classify_with_tower(&layer, &cfg);
            }
            full_times.push(t0.elapsed().as_secs_f64());

            // 增量 per-bar 累积（cache 跨步复用）。
            let t0 = std::time::Instant::now();
            let mut cache = TowerCache::new();
            for k in 1..=n {
                let layer = ParseLayer {
                    segments: Rc::new(all_segments[..k].to_vec()),
                    merged_bars: Rc::new(bars_from_closes(&closes)),
                    ..Default::default()
                };
                let _ = classify_with_tower_incremental(&layer, &cfg, &mut cache);
            }
            inc_times.push(t0.elapsed().as_secs_f64());
        }

        // exp 估计（log-log 斜率，sizes 翻倍）。
        let full_exp =
            (full_times[2] / full_times[0]).ln() / (sizes[2] as f64 / sizes[0] as f64).ln();
        let inc_exp = (inc_times[2] / inc_times[0]).ln() / (sizes[2] as f64 / sizes[0] as f64).ln();

        eprintln!(
            "\n===== 增量塔标度（合成 per-bar 累积）=====\n  \
             sizes={sizes:?}\n  full_times={full_times:?} (exp≈{full_exp:.2})\n  \
             inc_times={inc_times:?} (exp≈{inc_exp:.2})\n  \
             增量/全量比 @n={}: {:.2}x（越小增量越优）",
            sizes[2],
            inc_times[2] / full_times[2].max(1e-12)
        );

        // 增量须显著快于全量（MACD 增量 + 塔构造增量 + 走势分解增量 综合加速）。
        // ★判据：最大规模下增量/全量时间比 < 0.7（即增量至少 ~1.43x 加速）为稳健下界。
        // exp 差距在小规模 debug 噪声大（两者均 O(n²) 受限于 LevelState/tower_snapshots clone
        // 的 API 所需 O(k)/iter，故此合成尺度只能验证常数因子优势，asymptotic 分离须看
        // profile_incremental_tower_real_scaling 的真实大规模 #[ignore]）。此处验证常数因子：
        // 增量消除 MACD 全量重算 + 塔构造全量扫描。
        // 标度重标定（B4 / task#2，commit 254 改调 extract_signals_with_hist）：MACD 消重后
        // 全量只做 1×MACD（原 2×），增量相对优势从 >2x 收窄到 ~1.8x（ratio 实测集群
        // 0.543/0.548/0.559/0.55 across runs）。原阈值 0.5 按 full=2×MACD 标定，1×MACD 后需
        // 重标；取 0.7 为稳健下界（观测集群 ~0.55，留 ~0.14 机器噪声余量，仍断言真常数因子优势——
        // 若增量退化到无优势 ratio→1.0 则捕获）。这是因果重标定非「为绿改阈值」（no-patch 合规）。
        let ratio_at_max = inc_times[2] / full_times[2].max(1e-12);
        assert!(
            ratio_at_max < 0.7,
            "增量/全量比 @n={} = {ratio_at_max:.3} 须 < 0.7（增量至少 ~1.43x 加速；MACD+塔+分解增量）\n\
             full_exp≈{full_exp:.2}, inc_exp≈{inc_exp:.2}",
            sizes[2]
        );
    }
}

#[cfg(test)]
mod incremental_profile {
    //! 增量塔真实数据标度 profile（#[ignore]，需真实数据 + release）。
    use super::super::backtest::data;
    use super::super::parser;
    use super::*;

    /// ★真实数据 per-bar 标度：CL 真实段账本逐段追加，增量 vs 全量累积成本 + exp。
    ///
    /// 真实段账本单调追加（parser 前缀稳定语义）⟹ 增量有效域命中。验证真实数据下增量 exp≈1
    /// 而全量 exp≈2（塔构造超线性消除）。L2 经验标度（formalization-validity-domain 231号）。
    #[test]
    #[ignore = "真实数据标度 profile：需 CL 数据；--release（per-bar 双跑对照）"]
    fn profile_incremental_tower_real_scaling() {
        let cfg = ThetaConfig::default();
        let ds = match data::load_by_symbol("CL", &cfg) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("DATA BLOCKER: {e}");
                panic!("需真实数据");
            }
        };
        let oos = ds.slice_date_window("2023-01-01", "2025-06-30");
        // ponytail: 大规模标度验证 acceptance[4]——全引擎 per-bar exp≈1.0 @16K。
        // a7dfec46: n<5000 不可靠；此处用 [5K, 10K, 16K] 真实大规模。
        let sizes = [5_000usize, 10_000, 16_000];
        let mut full_times = Vec::new();
        let mut inc_times = Vec::new();
        let mut used_sizes = Vec::new();

        for &n in &sizes {
            if n > oos.bars.len() {
                eprintln!(
                    "DATA LIMIT: n={n} > oos.bars.len()={}，跳过",
                    oos.bars.len()
                );
                break;
            }
            let bars = &oos.bars[..n];

            // 全量 per-bar 累积。
            let t0 = std::time::Instant::now();
            for i in 50..n {
                let l0 = parser::parse_layer(&bars[..i], &cfg);
                let _ = classify_with_tower(&l0, &cfg);
            }
            full_times.push(t0.elapsed().as_secs_f64());

            // 增量 per-bar 累积。
            let t0 = std::time::Instant::now();
            let mut cache = TowerCache::new();
            for i in 50..n {
                let l0 = parser::parse_layer(&bars[..i], &cfg);
                let _ = classify_with_tower_incremental(&l0, &cfg, &mut cache);
            }
            inc_times.push(t0.elapsed().as_secs_f64());
            used_sizes.push(n);
            eprintln!(
                "n={n} done: full={:.2}s inc={:.2}s",
                *full_times.last().unwrap(),
                *inc_times.last().unwrap()
            );
        }

        // 逐相邻对算 exp（log-log 斜率），大规模验证 acceptance[4]。
        eprintln!("\n===== 增量塔真实标度（CL per-bar 累积，大规模）=====");
        for w in used_sizes.windows(2) {
            let (n0, n1) = (w[0], w[1]);
            let i0 = used_sizes.iter().position(|&s| s == n0).unwrap();
            let i1 = i0 + 1;
            let full_exp =
                (full_times[i1] / full_times[i0].max(1e-12)).ln() / (n1 as f64 / n0 as f64).ln();
            let inc_exp =
                (inc_times[i1] / inc_times[i0].max(1e-12)).ln() / (n1 as f64 / n0 as f64).ln();
            eprintln!(
                "  [{n0}→{n1}] full exp≈{full_exp:.2}  inc exp≈{inc_exp:.2}  \
                 增量/全量比 @{n1}: {:.2}x",
                inc_times[i1] / full_times[i1].max(1e-12)
            );
        }
    }
}
