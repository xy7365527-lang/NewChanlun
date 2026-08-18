//! 分类管线（classify pipeline）：Θ_level 递归级别构造主管线 + 公共数据载体
//! （[`LevelState`]/[`Classification`]）+ `classify*` 入口族。
//!
//! ★名分（#648 T2，2026-08-16）：本文件是 `classifier/mod.rs` **内联管线的纯移动抽取**
//! （#786 Q3 裁定改写稿 T2，勘察 `chanlun/review-results/issue786-q3-classifier-arch-review-20260816.md`），
//! **非 C6（#745）所删旧孤儿 `pipeline.rs` 的复活**——那一份是 kimi 线格局的零引用死壳
//! （416 行，从未被 mod 声明纳入编译树），与本文件内容不同源。零行为变更：
//! `mod.rs` 以 `pub use pipeline::*;` 保持 `classifier::Classification` 等公共路径不变。

use super::super::config::ThetaConfig;
use super::super::parser::ParseLayer;
use super::super::types::Side;
#[cfg(test)]
use super::super::types::Stroke;
use super::super::types::{Center, Direction, Segment, Tick};
use super::bsp::BspPoint;
use super::center::{self, UnitRange};
#[cfg(test)]
use super::decompose::decompose;
use super::decompose::{self, decompose_resume, MoveBlock};
use super::divergence;
#[cfg(test)]
use super::oracle_probe;
use super::recursive_tower::{
    self, descend_leveled, index_of_in, map_src_to_close_idx, CpScanOwnership, ElementId,
    LeveledMove,
};
use super::recursive_tower::{compose_level_resume, WindowScanCursor};
use super::tower_cache::{
    compute_macd_hist_incremental, update_closes_cache, AreaCache, LevelCache,
};
use super::TowerCache;
use super::{
    cand_event, cand_predicate, cp_replay_diagnostics, descend, operation, projection, rebase_txn,
    signal, stage_profile,
};
use std::borrow::Cow;
use std::cell::RefCell;
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
// ★#1053：全量循环已并入增量循环，本 helper 只剩测试消费。
#[cfg(test)]
pub(crate) fn extract_first_third_for_level(
    centers: &[Center],
    units: &[UnitRange],
    provenance_anchors: &[Option<Direction>],
    hist: &[f64],
    dif: &[f64],
    closes_tick: &[Tick],
    close_src: &[usize],
    gauge: divergence::DivergenceGauge,
    // ★#990 I-2：L0 笔序列（source_index 域，跨级同坐标系）供 ForceL 教义判据。
    strokes: &[Stroke],
    // ★#885 S4-d：分级记录生产 sink（原样透传 `_anchored`；记录 `level` 为占位 0，由
    // 装配方按 level_idx 盖章）。
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
        strokes,
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
// ★#1053：全量循环已并入增量循环，本 helper 只剩测试消费。
#[cfg(test)]
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
// ★#1053：全量循环已并入增量循环，本 helper 只剩测试消费。
#[cfg(test)]
fn detect_centers_geometric(units: &[UnitRange]) -> Vec<Center> {
    detect_centers_with(units, center::center_from_window)
}

/// canonical 中枢扫描（seed + 延伸吸收 + non-extension 终止）——**单一来源委托**
/// `recursive_tower::detect_centers_windowed_resume`（全量 = `start_i=0`；增量塔走同一函数的
/// resume 路径 ⟹ T^inc == T^full 定义性成立，非对拍性成立）。
// ★#1053：全量循环已并入增量循环，本 helper 只剩测试消费。
#[cfg(test)]
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
// ★#1053：全量循环已并入增量循环，本 helper 只剩测试消费。
#[cfg(test)]
pub(crate) fn classify_level(units: &[UnitRange], is_l0: bool) -> (Vec<Center>, Vec<MoveBlock>) {
    let centers = if is_l0 {
        detect_centers_complete(units)
    } else {
        detect_centers_geometric(units)
    };
    let blocks = decompose(&centers);
    (centers, blocks)
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
    classify_with_tower_incremental_inner(l0, config, &mut TowerCache::new(), &[]).0
}

/// ★#881 S3 操作级别挂载点入口（ADR 0011 裁定一/二/六/七）：分类 + 横向旁路读法。
///
/// 塔在构造时被告知 `operating_levels`（哪些层是操作级别，可多个同时存在——多重赋格的
/// 挂载面）；返回 `(Classification, Vec<OperationSequence>)`：
/// - `Classification`：与 [`classify`] **逐位相同**（旁路不回流主干的测试锁即比对两者全等）；
/// - `Vec<OperationSequence>`：每个挂载级别一份操作序列（口径 S 中枢 + 不延伸折叠块，含
///   并列盘整），按 `level` 升序（挂载声明先排序去重，重复声明不重复产）。
///
/// 挂载级别 ≥ 塔自然终止层时该挂载静默无产出（该层元素不存在，无可重折——同 `classify`
/// 的自然终止纪律，不报错不造假序列）。
pub fn classify_with_operations(
    l0: &ParseLayer,
    config: &ThetaConfig,
    operating_levels: &[u32],
) -> (Classification, Vec<operation::OperationSequence>) {
    let (classification, _, operations) =
        classify_with_tower_incremental_inner(l0, config, &mut TowerCache::new(), operating_levels);
    (classification, operations)
}

/// 分类 + 逐级塔导出入口（(i) 段导出桥，MEMORY coverage-engine-needs-tower-export-bridge）。
///
/// 返回 `(Classification, Vec<Vec<LeveledMove>>)`：
/// - `Classification`：与 `classify` bit-identical（全量入口 = 唯一循环 + 空 TowerCache，原行为不变）。
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
    let (classification, tower, _) =
        classify_with_tower_incremental_inner(l0, config, &mut TowerCache::new(), &[]);
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
    let mut cache = TowerCache::new();
    let (classification, tower, _) =
        classify_with_tower_incremental_inner(l0, config, &mut cache, &[]);
    (classification, tower, cache.candidate_book.streams())
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
    let (cls, tower, _) = classify_with_tower_incremental_inner(l0, config, cache, &[]);
    (cls, tower)
}

/// ★#902：增量塔 + 操作级别旁路（#881 S3 的增量对应物）——挂载级别的口径 S 分解随
/// TowerCache resume/frontier 同生命周期维护（`operation_decompose_resume`，`LevelCache.
/// operation_state`，cascade 同批失效），返回各挂载级别的 `OperationSequence`。
/// 空挂载 = 与 `classify_with_tower_incremental` 逐字节同行为。
pub fn classify_with_tower_incremental_operations(
    l0: &ParseLayer,
    config: &ThetaConfig,
    cache: &mut TowerCache,
    operating_levels: &[u32],
) -> (
    Classification,
    Vec<Rc<Vec<LeveledMove>>>,
    Vec<operation::OperationSequence>,
) {
    classify_with_tower_incremental_inner(l0, config, cache, operating_levels)
}

fn classify_with_tower_incremental_inner(
    l0: &ParseLayer,
    config: &ThetaConfig,
    cache: &mut TowerCache,
    operating_levels: &[u32],
) -> (
    Classification,
    Vec<Rc<Vec<LeveledMove>>>,
    Vec<operation::OperationSequence>,
) {
    let mounts: std::collections::BTreeSet<u32> = operating_levels.iter().copied().collect();
    let mut operations: Vec<operation::OperationSequence> = Vec::new();
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
    if std::env::var(super::super::env_registry::DIAG_L0UNITS).is_ok() {
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
        return (Classification::default(), Vec::new(), Vec::new());
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
        .get_or_init(|| std::env::var(super::super::env_registry::THETA_CASCADE_EPROBE).is_ok());
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
        // 自然终止：单元数 < min_parts ⟹ 停止。
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
            if *CASCADE_FULLCLEAR.get_or_init(|| {
                std::env::var(super::super::env_registry::THETA_CASCADE_FULLCLEAR).is_ok()
            }) {
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
                lc.operation_state = Default::default(); // #902：同批失效（同 key 守卫）。
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
                lc.operation_state = Default::default(); // #902：同批失效（同 key 守卫）。
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
        // ★#902 操作级别挂载点（增量版，ADR 0011 裁定一/六/七同约束）：本级声明为操作级别
        // ⟹ 口径 S 分解随 resume/frontier 增量维护（旁路只读 `units`、不回流主干，与 #881
        // 全量钩同位）。`dirty_from` = 本级 units 不可变前缀（§2.4 证书），frontier 弹窗/
        // 回卷由 `operation_decompose_resume` 内部按同一证书处理。
        if mounts.contains(&(level_idx as u32)) {
            let op_build: fn(&UnitRange, &UnitRange, &UnitRange) -> Option<Center> = if is_l0 {
                center::center_from_segments
            } else {
                center::center_from_window
            };
            operations.push(operation::operation_decompose_resume(
                &units,
                op_build,
                level_idx as u32,
                dirty_from,
                &mut lc.operation_state,
            ));
        }

        let (tail_centers, tail_upper, mut tail_cp, tail_metas, new_cursor) =
            stage_profile::time("05_compose_resume", || {
                compose_level_resume(
                    &units,
                    &moves_tower[..],
                    is_l0,
                    level_idx as u32 + 1,
                    resume_start,
                    prefix_count,
                    // #897：前缀中枢（pop 后保留段，与 prefix_count 1:1）供 tail run 计算。
                    &lc.centers,
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

        // BSP 提取（L0 线段层 + 递归组装层）。
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
                        &l0.strokes,
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
                        &l0.strokes,
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
    (Classification { levels }, tower_snapshots, operations)
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
/// 序列内识别第二类走势结构（第一类离开 + 回拉段，§10.2 买卖点定律一；回拉**不问**新不新低——
/// #816 B-2②，破一类极值由 `retrace_breaks_type1` 重合标注承载）。产出的 B2/S2
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
    // 次级别中枢（RMove::Compose.centers 末位 = 本窗真派生 B 口径核心区间；#897 后载荷为
    // 走势类型块中枢序列，本窗中枢在末位）。
    let c1 = match &parent.rmove {
        descend::RMove::Compose { centers, .. } => match centers.last() {
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
    let prev_area = cached_segment_area(
        area_cache, hist, stable_len, prev_seg.0, prev_seg.1, curr_dir,
    );
    let curr_area = cached_segment_area(
        area_cache, hist, stable_len, curr_seg.0, curr_seg.1, curr_dir,
    );
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
    dir: Direction,
) -> f64 {
    if end < stable_len {
        if let Some(&area) = cache.borrow().get(&(start, end)) {
            return area;
        }
        let area = divergence::segment_macd_area(hist, start, end, dir);
        cache.borrow_mut().insert((start, end), area);
        area
    } else {
        divergence::segment_macd_area(hist, start, end, dir)
    }
}
