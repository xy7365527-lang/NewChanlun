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
pub mod signal;
pub mod six_state;
pub mod voice_eat;
pub mod cand_predicate;

use bsp::BspPoint;
use center::UnitRange;
use decompose::{decompose, decompose_resume, MoveBlock};
use recursive_tower::{
    compose_level, descend_leveled, index_of_in, map_src_to_close_idx, project_to_units,
    ElementId, LeveledMove,
};
use super::types::Side;

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
}

/// 多级别递归分类输出（L0..Lmax；某层自然终止则该层及以上为空）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Classification {
    /// 索引 = 级别 ℓ（0=L0=1分钟线段账本）。
    pub levels: Vec<LevelState>,
}

/// 把 L0 线段规约为携带方向的走势单元（契约锚 `Origin.ChanlunElements.Segment` + `CenterConstruction.segHigh/segLow`）。
///
/// L0 单元 = parser 线段（**含方向**，完整判据 `DirAlternates` 的输入）；`[lo,hi]` 对齐
/// `Origin.CenterConstruction.segHigh/segLow`（向上段 hi=端价/向下段 lo=端价已规约为区间）。
fn segment_to_unit(seg: &Segment) -> UnitRange {
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
fn unit_to_segment(u: &UnitRange) -> Segment {
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
/// `point.force` 亦算得 `Some`（4 proxy），进 selector force_state 第 8 维。二/三类 force=None。
fn extract_first_third_for_level(
    centers: &[Center],
    units: &[UnitRange],
    hist: &[f64],
    dif: &[f64],
    closes_tick: &[Tick],
    close_src: &[usize],
) -> (Vec<BspPoint>, Vec<signal::PanDivCert>) {
    let segs: Vec<Segment> = units.iter().map(unit_to_segment).collect();
    signal::extract_signals_with_hist(centers, &segs, hist, dif, closes_tick, close_src)
}

/// 从 L0 线段单元序列识别 canonical 中枢序列（**完整判据** seed + 延伸吸收，契约锚
/// `Origin.CenterComplete.CenterConfirmedComplete` + 第20课中心定理一）。
///
/// L0 线段有内在方向 ⟹ seed 用 `center::center_from_segments`（完整判据：方向交替 ∧ 全三段核心
/// 非空，口径 B——第三段贯穿已被全三段核心非空吸收，637号 codex L0 等价）。seed 成立后进入延伸
/// 吸收（中心定理一：后续段区间触及 [ZD,ZG] ⟹ 同一中枢延伸，task #142），仅 non-extension
/// （`d_j>ZG ∨ g_j<ZD`）终止；seed 不成立则前进一段继续找。算法单一来源 = `recursive_tower::
/// detect_centers_windowed_resume`（全量/增量同一扫描，bit-exact 定义性）。
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
fn classify_impl(l0: &ParseLayer, config: &ThetaConfig) -> (Classification, Vec<Rc<Vec<LeveledMove>>>) {
    let min_parts = config.level.min_parts_per_level as usize;
    let l_max = config.level.l_max as usize;

    // L0 输入单元 = parser 线段账本（reference:29 L0=1分钟线段账本）。
    let mut units: Vec<UnitRange> = l0.segments.iter().map(segment_to_unit).collect();

    // 空 L0：无可构造级别（自然终止于 L0 之前）。
    if units.is_empty() {
        return (Classification::default(), Vec::new());
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
            .map(|(i, u)| LeveledMove::from_unit(u, ElementId { level: 0, ordinal: i as u64 }))
            .collect(),
    );

    // 第一类背驰 MACD：closes/close_src 在全递归层共享（L0 唯一可达 close 序列；上级走势的
    // 次级别 close 区间由 source_index 坐标定位，见 macd 接入点）。
    let closes: Vec<f64> = l0.merged_bars.iter().map(|b| b.close as f64).collect();
    let close_src: Vec<usize> = l0.merged_bars.iter().map(|b| b.source_index).collect();
    // ★force_state 生产热路由（beta-route #115）：dif（黄白线）+ closes_tick（整数 close）供一类候选
    // A/C 段 4 proxy（DIF 峰/振幅/速度）。hist/dif 同一 compute_macd 单趟产出（无额外 O(n) 扫描）。
    let series = divergence::compute_macd(&closes, &config.macd);
    let hist = series.hist;
    let dif = series.dif;
    let closes_tick: Vec<Tick> = l0.merged_bars.iter().map(|b| b.close).collect();

    let mut levels: Vec<LevelState> = Vec::new();
    let mut tower_snapshots: Vec<Rc<Vec<LeveledMove>>> = Vec::new();

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
        let (centers_w, upper_moves) = compose_level(&units, &moves_tower[..], is_l0, level_idx as u32 + 1);
        debug_assert_eq!(centers, centers_w, "compose_level 与 classify_level 中枢序列一致");

        // BSP 信号提取（reference:34-36）。三层覆盖：
        // - **L0 线段层**（`extract_signals`）：第一类（破中枢几何 L0 ∧ MACD 背驰 L1 真算）+ 第三类
        //   （confirmed 结构几何）。L0 走势单元 = 线段（有方向），第一/三类在线段端点上 bit-exact 判定。
        // - **递归组装层**（`extract_second_signals`，#53 接入）：第二类（B2/S2）由次级别第一类构成
        //   （买卖点定律一 §10.2）。对本级**每个上级走势** `RMove::Compose`，从 descend 取回的次级别
        //   走势序列内识别第二类走势结构（第一类离开 + 回拉不创新低/新高），产 B2/S2。背驰力度由
        //   `divergence_of` 闭包用 `divergence.rs` MACD 真算（次级别走势 close 区间 → 面积比较）。
        let (mut bsp, pan_div): (Vec<BspPoint>, Vec<signal::PanDivCert>) = if is_l0 {
            // ★force_state 生产热路由（beta-route #115）：传真 dif/closes_tick ⟹ 一类候选 point.force
            // = Some（4 proxy），进 selector force_state 第 8 维。结构六 bit 不变（force 不进 class_index/
            // 分桶 key，PartialEq 排除），GOLDEN 因 Debug 含 force 诚实翻转（signal.rs digest guard）。
            signal::extract_signals_with_hist(&centers, &l0.segments, &hist, &dif, &closes_tick, &close_src)
        } else {
            // 级别-N 一/三类（codex-decide-20260703 裁定 A）：units 承担线段角色，复用 L0 判据（含 force）。
            extract_first_third_for_level(&centers, &units, &hist, &dif, &closes_tick, &close_src)
        };
        // 递归组装层 B2/S2（#53 接入）：对每个上级走势的次级别走势序列识别第二类结构。
        bsp.extend(extract_second_for_level(&upper_moves, &hist, &close_src));
        bsp.sort_by_key(|p| p.source_index);

        levels.push(LevelState {
            moves,
            centers: Rc::new(centers.clone()),
            bsp: Rc::new(bsp),
            pan_div: Rc::new(pan_div),
        });

        // L(k+1) 输入单元 = 上级走势塔的 `UnitRange` 投影（外缘区间 + 坐标 + 外缘趋势方向）。
        // 上级走势携 subs（`RMove::Compose`），投影只为下一级几何中枢检测提供 [lo,hi] 区间——
        // 真递归 subs 在 `moves_tower` 里保留（不丢弃，旧塔丢弃 subs 是 B2 不可产的根因）。
        // Q7（task #145）：方向源 = 本级中枢 ownership 块方向（刚 push 的 LevelState.moves 单一来源）。
        units = project_to_units(&upper_moves, &levels.last().expect("本级 LevelState 已 push").moves);
        moves_tower = Rc::new(upper_moves);

        // 本级无中枢 ⟹ 无上级输入单元，停止递归（自然终止）。
        if units.is_empty() {
            break;
        }
    }

    (Classification { levels }, tower_snapshots)
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
    classify_impl(l0, config)
}

// ════════════════════════════════════════════════════════════════════════════
//  增量塔 API（task #93：per-bar substrate 塔构造 O(n²) → O(n)）
// ════════════════════════════════════════════════════════════════════════════
//
// ## 超线性根因（前序 aed4d5f5 实证：classify c_exp≈2.31 主导）
//
// `classify_impl` 的塔循环 `for level_idx in 0..=l_max` 每级调 `compose_level`（→
// `detect_centers_windowed`）+ `classify_level`（→ `detect_centers_with`）——两次全量滑窗扫描。
// per-bar substrate（每 bar 追加段）下，前级 confirmed 前缀稳定却重复全量扫描 ⟹ 超线性。
//
// ## 增量策略（严格有效域声明，formalization-validity-domain 231号）
//
// **有效域**：`tower_snapshots`（Vec<Vec<LeveledMove>>）+ `LevelState.centers` 的**中枢扫描构造**
// 可增量——`detect_centers_windowed` 是确定性左折叠（见 recursive_tower.rs 增量证明），已产出
// 中枢是不可变前缀，尾部追加续扫产出 bit-exact 尾部。
//
// **不在有效域**（诚实声明，no-声明膨胀）：
// - `LevelState.moves`（走势分解）：`decompose(&centers)` 在完整 centers 序列上折块，
//   尾部追加中枢可改变整体裁决 ⟹ 须每 bar 从累积 centers 全量裁决（非增量）。但裁决是 O(centers)
//   单趟，不是超线性源。
// - `LevelState.bsp`：依赖 MACD hist（每 bar 变）+ 完整 centers ⟹ 每 bar 全量重算。
//
// 故增量塔的 LevelState.centers/moves/bsp 由**累积的完整 centers 序列**经与全量同口径的裁决/
// BSP 提取产出（bit-exact），仅**中枢扫描构造**走增量 resume。塔构造的超线性（双次全量扫描）
// 被消除 ⟹ exp≈1。
//
// **641 谱系标注（有效域声明）**：incr_total exp 0.91 @16K ≈ 1.0 → 塔构造 O(n) **已达成**
// （L2 真实数据验证，非 L0/L1）。有效域 = 中枢扫描构造（compose_level_resume）+ parse_layer
// 增量合并（process_inclusion）。**不在有效域**：全引擎 per-bar 端到端 exp 须大规模验证（moves/
// bsp 裁决每 bar 全量但 O(centers) 单趟，非超线性源；实测端到端 exp 见 benchmark）。
// **注意（ab5f5a29d）**：O(n) 达成 ≠ 身份稳定→Stale 降根。增量塔 bit-exact 跨 bar 复用，但
// held_leg_tree_index 值字段比较（level/ρ/eps/λ）非对象身份——bit-exact 不变 ⟹ Stale 不降。
// "增量塔→身份连续→Stale 降根→ΔSharpe 可非零"路径被 L2 证伪（见 runner.rs 注释 + memory
// newchanlun-deltasharpe-zero-stale-rooting-perbar-reclass）。
//
// ## 跨级增量传播（严格不变量）
//
// 每级 units = `project_to_units(&upper_moves)`。上级 upper_moves 尾部追加时下级 units 尾部变，
// 下级从自己的断点续扫。**前缀不变量链**：L0 段前缀不变 ⟹ L0 upper_moves 前缀不变（resume 已证）
// ⟹ L1 units 前缀不变（project 逐元素，前缀同序同值）⟹ L1 扫描断点有效 ⟹ ... 逐级传播。
//
// bit-exact 充要：`cache.last_units_len` 与再次进入时的前缀严格增长（只追加，不修改前缀）。

use recursive_tower::{compose_level_resume, WindowScanCursor};

/// 单级增量缓存：已确认前缀 + 续扫断点。
///
/// 不变量（跨 bar 保持）：
/// - `scan_cursor.consumed`：本级窗口扫描退出断点（上次扫到此处，`units[..consumed]` 路径确定）。
/// - `upper_moves`：本级已 compose 的上级走势序列（前缀不可变；尾部续扫追加）。
/// - `last_input_len`：上次扫描时本级 `subs_moves`（= units）长度——再次进入时前缀须不大于此。
/// - `decompose_state`：增量走势分解冻结前缀（sealed 关系折出的块，O(1)/bar 续折）。
#[derive(Debug, Clone, Default)]
struct LevelCache {
    scan_cursor: WindowScanCursor,
    /// 已 compose 的上级走势序列（前缀不可变；尾部续扫追加）。每元素 `RMove::Compose` 携真 subs。
    /// ★O(n) 重构：`Rc` 共享塔——`moves_tower`/`tower_snapshots` 经 `Rc::clone`（O(1) 引用计数）取得，
    /// 消除 per-bar 全塔深拷贝（O(n²) 热点①）。`extend` 经 `Rc::make_mut`：caller 逐 bar drop 返回的
    /// snapshot ⟹ 下 bar extend 时 strong_count==1 ⟹ 原地追加 O(tail)，不触发写时复制。
    upper_moves: Rc<Vec<LeveledMove>>,
    /// 已识别中枢序列（与 `upper_moves` 一一对应，每窗口一中枢；前缀不可变，尾部续扫追加）。
    centers: Rc<Vec<Center>>,
    last_input_len: usize,
    /// 上次扫描的本级输入单元快照（前缀变异检测）。`detect_centers_windowed_resume` 充要条件 #2
    /// 要求 `units[..consumed]` 跨 bar 不可变；但 parser frontier 末段会**原地改写**（缠论古怪线段
    /// 重划，67/78课「顶高于底」+ 特征序列再分辨——定义层正确行为，非 parser bug）。仅长度回缩
    /// 守卫漏掉「长度不变/增长但末段值改写」的 frontier 变异。本快照逐值比对扫描区，变异 ⟹ 该级
    /// 缓存全量重置（anc.pdf §16：confirmed prefix immutable 可跳，frontier mutable 必须每 bar 重算）。
    cached_units: Vec<UnitRange>,
    /// 增量走势分解状态（decompose.rs resume 单一来源；冻结不变量见其模块头——只折 sealed
    /// 关系，临时尾关系每 bar 重折 ⟹ frontier 中枢改写/一次多产无需额外失效钩子）。
    decompose_state: decompose::DecomposeState,
    /// BSP memo 缓存：上次提取的 BSP 序列（与 `cached_bsp_key` 配对）。
    cached_bsp: Rc<Vec<BspPoint>>,
    /// ★Q4（task #145）：盘整背驰证书 memo 缓存——与 `cached_bsp` 同一 extract 调用产出、同一
    /// `cached_bsp_key` 守卫（hit/miss/cascade 三态与 bsp 锁步 ⟹ 下游 Rc::ptr_eq(bsp) 蕴含 pan_div 同批）。
    cached_pan_div: Rc<Vec<signal::PanDivCert>>,
    /// BSP memo guard key = (centers.len, upper_moves.len, segments.len[L0 only])。三者不变 ⟹
    /// BSP 纯函数同输入同输出（confirmed 元素区间在稳定前缀，尾 bar 不影响）⟹ 复用缓存 bit-exact。
    cached_bsp_key: Option<(usize, usize, usize)>,
    /// ★O(n²) 真修（#106）：本级 `upper_moves` 投影缓存（= `project_to_units(&upper_moves)`，下一级输入）。
    /// `upper_moves` 前缀不变仅尾部 append（§16）⟹ 投影前缀不变（`fold_direction(prev)` 只依赖元素+前驱，
    /// 前缀稳定 ⟹ 前缀投影稳定）。每 bar 只对新 append 的 tail 投影（`projected_units.len()..`），避免
    /// per-bar 全量 `project_to_units` 递归 `rmove.lo()/hi()` 整棵子树 O(nodes)/bar=O(n²)。
    /// cascade_reset（前缀重排）⟹ 与 upper_moves 同步清空（line 850 旁）重投影。
    projected_units: Vec<UnitRange>,
    /// ★07b frontier 门控（A 泳道 resume 家族，与 A3/07c 同族）：confirmed 前缀 `upper_moves` 的第二类
    /// B2/S2 输出缓存。`extract_second_for_level` 每 memo-miss 全塔重扫 O(U)=O(n²)，但每个 parent 的 B2
    /// 只依赖该 parent（`c1`/subs）+ hist/close_src——confirmed 前缀 parent 的源区间落稳定前缀、hist 前缀
    /// append-only 稳定 ⟹ 其 B2 跨 bar 不变。缓存前缀 B2（稀疏：多数 parent 产 0 个 B2），每 bar 只重算
    /// frontier tail。cascade_reset（前缀重排）⟹ 同步清空（与 cached_bsp 一致，line 1119 旁）。
    cached_second: Vec<BspPoint>,
    /// `cached_second` 已覆盖的 `upper_moves` 前缀数（推进锚，= 上次门控的 prefix_count）。
    cached_second_count: usize,
}

/// B3 #4 area-memo（07b 残余 O(n²) 根治）：`(start,end)→segment_macd_area` 冻结缓存，跨 bar 持久
/// （挂 [`TowerCache`]，非 per-level——`hist` 全局单份，键值与 level 无关）。B3 profile 坐实：同一
/// (start,end) 被 [`sublevel_diverges`] 跨 bar 重复查询（calls=4.19M / distinct=2314 @ 300K，冗余
/// 99.94%）——07b 门控消除了「confirmed 前缀 parent 每 bar 重扫」，但 frontier parent 每 bar 仍对
/// 其固定 `prev_seg`（已稳定、远端）重新线性求和一次，O(range) 逐 bar 累积 = 残余 O(n²)。
///
/// 值一旦写入永久有效（hist 前缀 append-only 稳定，见 [`TowerCache::macd_hist`]）——**只对
/// `end < stable_len` 的查询读写缓存**（[`cached_segment_area`]），`stable_len` = 当前 bar 的
/// `TowerCache::macd_state_len`（`compute_macd_hist_incremental` 每 bar 末元素是 unstable tail，
/// 下 bar 可能被改写覆盖，见其函数头注释——绝不缓存该越界查询，防污染未来错值）。
type AreaCache = HashMap<(usize, usize), f64>;

/// 增量塔缓存（跨 bar 跨级复用）：每级 `LevelCache` + L0 段账本快照长度 + MACD 增量状态。
///
/// **使用契约**（bit-exact 充要，违反则增量破裂）：
/// 1. 每 bar 喂 `classify_with_tower_incremental(l0_i, config, &mut cache)`，`l0_i.segments` 是
///    `parse_layer(&bars[..=i])`——段账本**只允许尾部增长**（前缀段不可变，parser 前缀稳定语义）。
/// 2. 若某 bar 段账本前缀**回缩或改写**（非单调追加），须 `cache.clear()` 重置（退化为全量）。
/// 3. `config.level`（l_max/min_parts）不可变——变则 `cache.clear()`。
/// 4. `merged_bars.close` 前缀须单调追加（前缀不变）——inclusion 合并可能改写尾部，
///    故 `macd_closes_prefix` 校验 `closes[..last_merged_len]` 逐值一致；不一致则全量重建。
#[derive(Debug, Clone, Default)]
pub struct TowerCache {
    /// 逐级缓存（下标 = 级别 idx，与 `tower_snapshots` 同构）。
    levels: Vec<LevelCache>,
    /// 上次处理的 L0 段数（前缀不变量校验用）。
    last_l0_segments_len: usize,
    /// MACD 增量递推状态（None=未初始化；Some=已处理至 `macd_state_len` 末）。
    macd_state: Option<MacdState>,
    /// 已增量产出的 hist 前缀（不可变；尾部 append 续产）。bit-exact 等价于
    /// `compute_macd(closes[..macd_state_len]).hist`。
    macd_hist: Vec<f64>,
    /// 已增量产出的 dif 前缀（黄白线，与 `macd_hist` **逐 bar 锁步**——同一 `MacdState::current_point`
    /// 派生，同一 truncate/push 边界）。bit-exact 等价于 `compute_macd(closes[..macd_state_len]).dif`
    /// （dif 是 hist 的子表达式 `hist=dif-dea`，hist 增量已证 bit-exact ⟹ dif 同证）。force_state 生产
    /// 热路由（一类候选 A/C 段 `segment_dif_peak`）消费——`extract_signals_with_hist` 收 dif 才算 force。
    macd_dif: Vec<f64>,
    /// MACD state 实际消费的 close 数（state 表示 `closes[..macd_state_len]` 的累积）。
    /// #106：替代旧 `macd_closes_prefix: Vec<f64>`（每 bar O(n) 全量比较 + to_vec 克隆 = O(n²)）。
    /// 增量边界 = `macd_state_len`；前缀稳定性靠 parser `confirmed_len` 证书（codex：绑 state_len
    /// 而非 cached_len-1，修血缘断裂漏洞）。config 变更经 `clear()` 失效（ponytail: config 在 cache
    /// 生命周期固定，clear 已覆盖，无需独立 macd_epoch）。
    macd_state_len: usize,
    /// #106：L0 走势塔增量缓存（前缀稳定，仅 segments 末段可古怪线段重划改写）。用 parser
    /// `segments_confirmed_len` 证书复用前缀，只 map 新尾段——替代每 bar 全量 `from_unit` 重建
    /// （O(segs)×n = O(n²)，profile 坐实 400K=5.8s）。bit-exact：from_unit 只依赖单 seg（无相邻
    /// 依赖，codex 确认），前缀稳定 ⟹ moves_tower_l0 前缀稳定；ordinal=全局索引（reuse+i）跨 bar 稳定。
    moves_tower_l0: Rc<Vec<LeveledMove>>,
    /// #106：L0 输入单元（segment_to_unit 投影）增量缓存——同 moves_tower_l0 证书复用，消除每 bar
    /// 全量 `l0.segments.map(segment_to_unit).collect()`（O(segs)×n，profile 坐实 400K=0.6s）。
    /// segment_to_unit 只依赖单 seg ⟹ 前缀稳定 bit-exact。
    l0_units_cache: Vec<UnitRange>,
    /// merged_bars.close 增量缓存（前缀稳定，仅尾 bar 可能 inclusion 改写）。每 bar mem::take 出借
    /// 给 BSP/MACD，用毕放回——避免每 bar 全量 `.map().collect()` 重建（O(n)/bar → O(n²) 根因）。
    closes: Vec<f64>,
    /// merged_bars.source_index 增量缓存（与 `closes` 同步，坐标系映射用）。
    close_src: Vec<usize>,
    /// merged_bars.close 的 **Tick（整数）** 增量缓存（与 `closes` 同步，force 价格振幅/速度 proxy 用）。
    /// `closes_tick[i] == merged_bars[i].close`（Tick 本身，非 f64 往返）——与 `extract_signals_force`
    /// 的 `closes as f64 as Tick` 逐值一致（整值 Tick 往返 f64 精确）。热路径 mem::take 出借，用毕放回。
    closes_tick: Vec<Tick>,
    /// ★工位 4g：塔变更代次（generation）——下游 [`super::strategy::interp::TreeCache`] 用其 O(1) 判断
    /// 是否复用缓存树，**跳过每 bar O(tree) 的 `TreeKey::of(tower)` 全量重算**（exp≈2.0 真因）。
    ///
    /// **单调递增**，仅当 `extract_elements(tower)` **可观察输出可能变化**时 +1。维护点（codex 异质审
    /// soundness 全覆盖）：(a) 任一级 `cascade_reset`（frontier 改写/回缩传播至最高级重扫）；(b) 任一级
    /// `upper_moves` extend 非空 tail（含新级涌现首产 + 最高级 append）；(c) `clear()`（全量重扫）。
    ///
    /// soundness 充要（codex Q3）：generation 绑定"可观察树变更"非指针/len。低级 extend 不影响最高级
    /// 输出时**过度** +1（保守——多算一次 TreeKey，绝不假命中），sound 安全。`extract_elements` 只读最高
    /// 非空级，但 `sub_moves: Vec<LeveledMove>` 值拷贝（compose 时 `subs.to_vec()`）⟹ 低级静默变异必经
    /// cascade 重建父级才更新副本（codex Q1 确认无静默路径）。
    generation: u64,
    /// B3 #4 area-memo：[`AreaCache`]（见其文档）——`sublevel_diverges` 的 `(start,end)→area`
    /// 冻结缓存，跨 bar 持久。
    area_cache: AreaCache,
}

impl TowerCache {
    /// 构造空缓存（首次调用全量扫，之后增量复用）。
    pub fn new() -> Self {
        Self::default()
    }

    /// ★工位 4g：当前塔变更代次（下游 `TreeCache` O(1) 命中判据）。同代次 ⟹ `extract_elements`
    /// 输出逐字节不变（soundness 见 `generation` 字段文档）。
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// 当前增量产出的 MACD dif 前缀（黄白线，force_state 生产热路由输入；bit-exact 等价全量
    /// `compute_macd(closes).dif`）。与 [`Self::macd_hist_for_test`] 逐 bar 锁步、等长。
    pub fn macd_dif(&self) -> &[f64] {
        &self.macd_dif
    }

    /// 当前增量产出的 MACD hist 前缀（对拍锚：dif 增量正确性由 `dif==全量` ∧ `hist==全量` 双证）。
    pub fn macd_hist_for_test(&self) -> &[f64] {
        &self.macd_hist
    }

    /// 当前增量产出的 close(Tick) 前缀（force 价格振幅/速度 proxy 输入；`== merged_bars.close`）。
    pub fn closes_tick(&self) -> &[Tick] {
        &self.closes_tick
    }

    /// 重置缓存（退化为下次全量重扫）。
    ///
    /// 调用时机：段账本前缀非单调追加（回缩/改写）、或 config 变更、或 merged_bars
    /// 前缀改写（inclusion 合并回退改写尾部）。
    pub fn clear(&mut self) {
        self.levels.clear();
        self.last_l0_segments_len = 0;
        self.macd_state = None;
        self.macd_hist.clear();
        self.macd_dif.clear(); // 与 macd_hist 锁步（同 truncate/rebuild 边界，见 compute_macd_hist_incremental）。
        self.macd_state_len = 0;
        Rc::make_mut(&mut self.moves_tower_l0).clear();
        self.l0_units_cache.clear();
        self.closes.clear();
        self.close_src.clear();
        self.closes_tick.clear(); // 与 closes 锁步（同 update_closes_cache 前缀复用）。
        self.area_cache.clear(); // hist 全量重扫 ⟹ 旧 (start,end)→area 键值可能不再对应新 hist。
        // ★工位 4g：clear=全量重扫 ⟹ extract 输出会变 ⟹ +generation（不 reset 为 0——下游缓存旧
        // generation 可能恰为 0 ⟹ 假命中复用陈旧树）。单调递增保 sound。
        self.generation += 1;
    }
}

/// 增量 MACD hist 计算（231号纯性能，bit-exact 铁律）。
///
/// 返回完整 hist 序列（等长 `closes`），与 `divergence::compute_macd(closes, cfg).hist`
/// 逐元素 bit-identical（ac75d4b3 已证 `MacdState::append` bit-exact）。
///
/// ## per-bar substrate 语义（inclusion 合并的尾部不稳定性）
///
/// `parse_layer(&bars[..i])` 的 `merged_bars` = `merged_prefix`（confirmed 稳定前缀）+
/// `acc`（当前未定稿合并段，可能被后续 bar 吸收改写）。故跨 bar：
/// - `merged_bars[..len-1]` 稳定（confirmed 前缀不动）
/// - `merged_bars[len-1]`（= acc）可能改写
///
/// 增量 state 表示 `closes[..stable_prefix]`（stable_prefix = `closes.len() - 1`，排除
/// 不稳定的尾 bar）。尾 bar 的 hist 由 state + 尾 close O(1) 派生。每 bar 追加：
/// - 长度相同（inclusion 吸收）：stable_prefix 不变 ⟹ state 有效，尾 bar hist 重算。
/// - 长度 +1（非包含定稿）：旧尾 bar 现稳定 ⟹ state append 旧尾 close，新尾 bar hist 派生。
///
/// ## 退化路径（cache 空 / 前缀改写 / 长度回缩）
///
/// 逐 bar `append` 重建 state + hist 数组（与全量同 EMA 约简顺序，bit-exact）。O(n) 单次，
/// 非 fallback——是 inclusion 回退的合法 bit-exact 退化（同塔 cache clear 逻辑）。
///
/// ## 认识论（formalization-validity-domain 231号）
///
/// L1：bit-exact 等价于全量 `compute_macd`（管线正确性，零 alpha 信息增量）。
/// 增量只把同一浮点约简从「全量重算」改成「逐 bar 延伸」，数值不变。
/// closes/close_src 增量缓存更新（231号纯性能，bit-exact 铁律）。
///
/// 维护 `cache.closes == merged_bars.iter().map(|b| b.close as f64).collect()` 与
/// `cache.close_src == merged_bars.iter().map(|b| b.source_index).collect()`，逐元素 bit-identical。
///
/// ## per-bar substrate 语义（inclusion 尾部不稳定性，同 `compute_macd_hist_incremental`）
///
/// `merged_bars` = confirmed 稳定前缀 + 当前未定稿合并段 `acc`（尾元素，可能被后续 bar 吸收改写）。
/// 故 `merged_bars[..len-1]` 跨 bar 稳定（confirmed 前缀不动），仅 `merged_bars[len-1]`（acc）可能改写。
/// ⟹ 缓存只需：截掉尾元素（重算的边界），从稳定前缀末续推 close/source_index。
///
/// - 长度 +k（新 bar 定稿）：旧尾现稳定 ⟹ 续 push 新元素。
/// - 长度不变（inclusion 吸收）：尾元素可能改写 ⟹ 截尾重 push。
/// - 长度回缩 / 前缀改写：全量重建（bit-exact 退化，同 cache clear 逻辑）。
fn update_closes_cache(
    merged_bars: &[super::types::Bar],
    confirmed_len: usize,
    cache: &mut TowerCache,
) {
    let n = merged_bars.len();
    let cached = cache.closes.len();

    // ★#106 O(1) 证书路径（替代旧每 bar O(n) 前缀全量比较 = O(n²) 主导根因，profile 坐实
    // 200K=15.3s/67%）：parser `merged_confirmed_len` 保证 `merged_bars[..confirmed_len]` 跨 bar
    // **物理不变**（相 B append_folded 仅 Rc::make_mut pop/push 末根，前缀字节不动；codex 锚定）。
    // ⟹ `cache.closes[..confirmed_len]` 与 `merged_bars[..confirmed_len]` 逐值一致（close + source_index
    // 双稳定，codex 3a/3b 满足——同一 Bar 物理不变两字段都不变），无需 O(n) 比较。
    //
    // bit-exact 前提（codex 血缘）：cache 由 classifier 每 bar 同源维护（生产路径每 bar 调 classify_at），
    // confirmed_len 单调 ⟹ cache[..confirmed_len] 是上轮前缀。前提失守（cached < confirmed_len，
    // 跨 bar 漏调 / 血缘断裂）⟹ reuse = min(cached, confirmed_len)，余下全量重扫（保守 bit-exact）。
    // confirmed_len=0（相 A / 相 A→B fold_all 整段重写 / 全量 parse_layer 无血缘）⟹ reuse=0 = 全量重建。
    let reuse = confirmed_len
        .min(cached)
        .min(cache.close_src.len())
        .min(cache.closes_tick.len());
    cache.closes.truncate(reuse);
    cache.close_src.truncate(reuse);
    cache.closes_tick.truncate(reuse);
    cache.closes.reserve(n.saturating_sub(reuse));
    cache.close_src.reserve(n.saturating_sub(reuse));
    cache.closes_tick.reserve(n.saturating_sub(reuse));
    for b in &merged_bars[reuse..] {
        cache.closes.push(b.close as f64);
        cache.close_src.push(b.source_index);
        cache.closes_tick.push(b.close); // Tick 本身（整数），force 价格振幅/速度 proxy 用（与 closes 锁步）。
    }
}

fn compute_macd_hist_incremental(
    closes: &[f64],
    confirmed_len: usize,
    cfg: &super::config::MacdConfig,
    cache: &mut TowerCache,
) {
    // 空 closes：无 MACD 可算，清空 cache MACD 域（防御性，classify 入口已防空 layer）。
    if closes.is_empty() {
        cache.macd_state = None;
        cache.macd_hist.clear();
        cache.macd_dif.clear();
        cache.macd_state_len = 0;
        return;
    }

    // 单 bar：state = init(closes[0])，hist = [0.0]（首 bar DIF=DEA=hist=0）。state 覆盖 closes[..1]。
    if closes.len() == 1 {
        let state = MacdState::init(closes[0], cfg);
        let p = state.current_point();
        cache.macd_hist = vec![p.hist];
        cache.macd_dif = vec![p.dif]; // 首 bar DIF=0（close-close），与 hist 同点派生。
        cache.macd_state = Some(state);
        cache.macd_state_len = 1;
        return;
    }

    // 稳定前缀长度（排除不稳定的尾 bar）。state 续推目标 = closes[..stable_prefix]。
    let stable_prefix = closes.len() - 1;

    // ★#106 O(1) 证书增量（替代旧每 bar O(n) 前缀比较 + `closes.to_vec()` 全量克隆 = O(n²)，
    // profile 坐实 200K=4.7s/21%）：复用边界 = `min(macd_state_len, confirmed_len)`。
    // - `macd_state_len`：state 已消费的 close 数（state 表示 closes[..macd_state_len] 累积）。
    // - `confirmed_len`：parser 证书——closes[..confirmed_len] 跨 bar 物理不变（前缀未被 inclusion
    //   改写）。取 min ⟹ state 覆盖的部分**全在稳定前缀内** ⟹ 续推不被改写污染（codex 血缘修复：
    //   绑 state_len 非 cached_len-1）。
    // resume_from > stable_prefix 不可能（resume_from <= macd_state_len <= 上轮 stable < 本轮 stable）；
    // 但 confirmed_len 收缩（相 A→B fold_all=0）⟹ resume_from=0 ⟹ 从头全量重推（bit-exact 退化）。
    let resume_from = cache.macd_state_len.min(confirmed_len).min(stable_prefix);

    // macd_dif 与 macd_hist **逐 bar 锁步**：同一 truncate/clear/push 边界，同一 `current_point` 派生
    // （dif 是 hist 的子表达式）。任一分支对 hist 的操作都对 dif 做同样操作 ⟹ len 恒等、bit-exact。
    let mut state = if resume_from > 0 && cache.macd_state.is_some() {
        // 增量：从 resume_from 的 state 续推。需要 state 恰好表示 closes[..resume_from]——
        // 若 macd_state_len > resume_from（confirmed_len 收缩截断），state 比 resume_from 多消费了
        // 已失效的 close ⟹ 不能直接用，须从头重推。故仅 macd_state_len == resume_from 时复用。
        if cache.macd_state_len == resume_from {
            cache.macd_hist.truncate(resume_from);
            cache.macd_dif.truncate(resume_from);
            cache.macd_state.clone().expect("is_some 已判")
        } else {
            cache.macd_hist.clear();
            cache.macd_dif.clear();
            rebuild_macd_state_to(closes, resume_from, cfg, &mut cache.macd_hist, &mut cache.macd_dif)
        }
    } else {
        // 全量重建（resume_from=0 或 state 空）。
        cache.macd_hist.clear();
        cache.macd_dif.clear();
        rebuild_macd_state_to(closes, 0, cfg, &mut cache.macd_hist, &mut cache.macd_dif)
    };

    // 续推 closes[hist.len()..stable_prefix]（新稳定 bar）+ 尾 bar（不稳定）hist/dif。
    for &c in &closes[cache.macd_hist.len()..stable_prefix] {
        state = divergence::compute_macd_append(&state, c);
        let p = state.current_point();
        cache.macd_hist.push(p.hist);
        cache.macd_dif.push(p.dif);
    }
    let tail_state = divergence::compute_macd_append(&state, closes[stable_prefix]);
    let tail_p = tail_state.current_point();
    cache.macd_hist.push(tail_p.hist);
    cache.macd_dif.push(tail_p.dif);
    cache.macd_state = Some(state);
    cache.macd_state_len = stable_prefix;
}

/// MACD state 重建到 `closes[..target]`（target=0 ⟹ init(closes[0])，state_len=1）。
/// `hist`/`dif` 被 push 至 len==max(target,1)（首 bar hist=dif=0 + 续 bar），逐 bar 同点派生锁步。
/// 返回 closes[..hist.len()] 的 state。bit-exact：与全量 `compute_macd` 同 EMA 约简（逐 bar append）。
fn rebuild_macd_state_to(
    closes: &[f64],
    target: usize,
    cfg: &super::config::MacdConfig,
    hist: &mut Vec<f64>,
    dif: &mut Vec<f64>,
) -> MacdState {
    let mut state = MacdState::init(closes[0], cfg);
    let p0 = state.current_point();
    hist.push(p0.hist);
    dif.push(p0.dif);
    let end = target.max(1);
    for &c in &closes[1..end] {
        state = divergence::compute_macd_append(&state, c);
        let p = state.current_point();
        hist.push(p.hist);
        dif.push(p.dif);
    }
    state
}

// ════════════════════════════════════════════════════════════════════════════
//  阶段计时插桩（profile-only，#106 真热点定位）
// ════════════════════════════════════════════════════════════════════════════
//
// thread_local 累加器，env `THETA_PROFILE_STAGES=1` 时启用。只测时间不改逻辑（bit-exact 安全）。
pub mod stage_profile {
    use std::cell::RefCell;
    use std::time::{Duration, Instant};

    thread_local! {
        static ACC: RefCell<Vec<(&'static str, Duration)>> = const { RefCell::new(Vec::new()) };
        // 跨度累加器（(label, sum, max, count)）——区分 H-detect（跨度随 n 增长）vs H-detect-bounded
        // （跨度 O(1)）。env-gated，未启用时 record_span 直通。
        static SPANS: RefCell<Vec<(&'static str, u64, u64, u64)>> = const { RefCell::new(Vec::new()) };
        static ENABLED: bool = std::env::var("THETA_PROFILE_STAGES").is_ok();
    }

    pub fn enabled() -> bool {
        ENABLED.with(|e| *e)
    }

    /// 记录一次跨度样本（如 05 续扫的 `units.len() - start_i`）。env 未启用时零开销直通。
    pub fn record_span(label: &'static str, v: u64) {
        if !enabled() {
            return;
        }
        SPANS.with(|s| {
            let mut s = s.borrow_mut();
            if let Some(slot) = s.iter_mut().find(|(l, ..)| *l == label) {
                slot.1 += v;
                slot.2 = slot.2.max(v);
                slot.3 += 1;
            } else {
                s.push((label, v, v, 1));
            }
        });
    }

    pub struct Guard {
        label: &'static str,
        start: Instant,
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            let d = self.start.elapsed();
            let label = self.label;
            ACC.with(|a| {
                let mut a = a.borrow_mut();
                if let Some(slot) = a.iter_mut().find(|(l, _)| *l == label) {
                    slot.1 += d;
                } else {
                    a.push((label, d));
                }
            });
        }
    }

    pub fn stage(label: &'static str) -> Option<Guard> {
        if enabled() {
            Some(Guard { label, start: Instant::now() })
        } else {
            None
        }
    }

    /// 计时一个表达式（闭包包裹；env 未启用时零开销直通）。
    pub fn time<T>(label: &'static str, f: impl FnOnce() -> T) -> T {
        if !enabled() {
            return f();
        }
        let start = Instant::now();
        let r = f();
        let d = start.elapsed();
        ACC.with(|a| {
            let mut a = a.borrow_mut();
            if let Some(slot) = a.iter_mut().find(|(l, _)| *l == label) {
                slot.1 += d;
            } else {
                a.push((label, d));
            }
        });
        r
    }

    pub fn dump() {
        if !enabled() {
            return;
        }
        ACC.with(|a| {
            let a = a.borrow();
            eprintln!("=== THETA STAGE PROFILE ===");
            let mut rows: Vec<_> = a.iter().collect();
            rows.sort_by_key(|(_, d)| std::cmp::Reverse(*d));
            for (label, d) in rows {
                eprintln!("  {:<36} {:>10.3} ms", label, d.as_secs_f64() * 1000.0);
            }
            eprintln!("===========================");
        });
        SPANS.with(|s| {
            let s = s.borrow();
            if s.is_empty() {
                return;
            }
            eprintln!("=== THETA STAGE SPANS ===");
            for (label, sum, max, count) in s.iter() {
                let avg = if *count > 0 { *sum as f64 / *count as f64 } else { 0.0 };
                eprintln!(
                    "  {:<24} sum={:>14} max={:>10} count={:>10} avg={:>12.2}",
                    label, sum, max, count, avg
                );
            }
            eprintln!("=========================");
        });
    }
}

/// ★A3 证书 oracle 探针（仅 test 构建）：记录证书热路径分支命中，供 always-run oracle 断言
/// 「fixture 确实触发了 had_emitted_window pop（T==1/T>1）与两处早停缓存血缘失效」——防止
/// 「always-run 但覆盖为零」的陷阱（codex 审计第6条根修）。release/非 test 构建完全不编译。
#[cfg(test)]
pub mod oracle_probe {
    use std::cell::RefCell;

    #[derive(Default, Clone, Debug)]
    pub struct Probe {
        /// had_emitted_window pop 后重扫仅复现被 pop 窗口（`tail_upper.len() == 1`）——did_extend 证伪正向锁。
        pub t_eq1: u64,
        /// had_emitted_window pop 后重扫产出 >1 窗口（frontier 值改写，bar-1464 型）。
        pub t_gt1: u64,
        /// `units.len() < min_parts` 早停 break 触发次数（truncate(level_idx) 语句执行——含 no-op）。
        pub minparts_break: u64,
        /// `units.is_empty()` 早停 break 触发次数（truncate(level_idx+1) 语句执行——含 no-op）。
        pub empty_break: u64,
        /// `units.len() < min_parts` 早停且实际 truncate 掉已建级（§2.6 血缘失效路径1，depth 下降）。
        pub reentry_minparts: u64,
        /// `units.is_empty()` 早停且实际 truncate 掉已建级（§2.6 血缘失效路径2，depth 下降）。
        pub reentry_empty: u64,
    }

    thread_local! {
        static PROBE: RefCell<Probe> = RefCell::new(Probe::default());
    }

    pub fn reset() {
        PROBE.with(|p| *p.borrow_mut() = Probe::default());
    }
    pub fn snapshot() -> Probe {
        PROBE.with(|p| p.borrow().clone())
    }
    /// `t` = `tail_upper.len()`（本 bar 本级 compose 产出窗口数），had_emitted_window 时调用。
    pub fn on_pop_rescan(t: usize) {
        PROBE.with(|p| {
            let mut p = p.borrow_mut();
            if t == 1 {
                p.t_eq1 += 1;
            } else if t > 1 {
                p.t_gt1 += 1;
            }
        });
    }
    /// min_parts 早停：`removed` = 本次 truncate 是否实际删掉已建级（depth 下降）。
    pub fn on_minparts_break(removed: bool) {
        PROBE.with(|p| {
            let mut p = p.borrow_mut();
            p.minparts_break += 1;
            if removed {
                p.reentry_minparts += 1;
            }
        });
    }
    /// units.is_empty 早停：`removed` = 本次 truncate 是否实际删掉已建级（depth 下降）。
    pub fn on_empty_break(removed: bool) {
        PROBE.with(|p| {
            let mut p = p.borrow_mut();
            p.empty_break += 1;
            if removed {
                p.reentry_empty += 1;
            }
        });
    }
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

    // L0 输入单元 = parser 线段账本。#106 证书增量（同 moves_tower_l0，消除每 bar 全量 collect）。
    // 返回 `reuse` = L0 units 复用前缀长度 = dirty_from[0]（§2.3：L0 不可变前缀，units[..reuse]
    // 逐字段等上 bar，units[reuse..] 本 bar 新 extend）。
    let l0_dirty_from = stage_profile::time("00_l0_units_build", || {
        let reuse = l0.segments_confirmed_len.min(cache.l0_units_cache.len());
        cache.l0_units_cache.truncate(reuse);
        cache.l0_units_cache.extend(l0.segments[reuse..].iter().map(segment_to_unit));
        reuse
    });
    let l0_units: Vec<UnitRange> =
        stage_profile::time("00b_l0_units_clone", || cache.l0_units_cache.clone());

    // DIAG(frontier-bit-exact): 对拍复用版 l0_units_cache vs 全量 segment_to_unit（隔离 L0 units 前缀复用是否陈旧）。
    if std::env::var("DIAG_L0UNITS").is_ok() {
        let full: Vec<UnitRange> = l0.segments.iter().map(segment_to_unit).collect();
        if l0_units != full {
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
        cache.clear();
        return (Classification::default(), Vec::new());
    }

    // 前缀不变量校验：段账本回缩 ⟹ 清空重扫（bit-exact 退化，非增量）。
    if l0.segments.len() < cache.last_l0_segments_len {
        cache.clear();
    }
    cache.last_l0_segments_len = l0.segments.len();

    // L0 走势塔 = 携坐标的 RMove::Segment（递归底）。
    // ★#106 证书增量（替代每 bar 全量 from_unit 重建 = O(segs)×n，profile 坐实 400K=5.8s）：
    // parser `segments_confirmed_len` 保证 segments[..confirmed_len] 跨 bar bit-stable（仅末段可古怪
    // 线段重划，codex 确认）⟹ moves_tower_l0[..reuse] 复用（from_unit 只依赖单 seg，无相邻依赖）。
    // ordinal=reuse+i 全局索引（前缀 reuse<=confirmed_len 时 ordinal 不变 = 全量 enumerate，bit-exact）。
    // make_mut：caller 逐 bar drop 上轮 tower_snapshots[0] ⟹ strong_count==1 ⟹ 原地 O(tail)；
    // >1（理论 caller 跨 bar 持有）⟹ 写时复制（仍 bit-exact）。clear() 已同步清空（退化全量）。
    stage_profile::time("01_l0_tower_rebuild", || {
        let reuse = l0.segments_confirmed_len.min(cache.moves_tower_l0.len());
        let m = Rc::make_mut(&mut cache.moves_tower_l0);
        m.truncate(reuse);
        for (off, seg) in l0.segments[reuse..].iter().enumerate() {
            let i = reuse + off;
            let u = segment_to_unit(seg);
            m.push(LeveledMove::from_unit(&u, ElementId { level: 0, ordinal: i as u64 }));
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
    let mut units: Vec<UnitRange> = l0_units;
    let mut moves_tower: Rc<Vec<LeveledMove>> = moves_tower_l0;

    // ★cascade reset（codex 异质审查裁决：option 2）：任一级检出 frontier 变异 ⟹ 该级**及所有更高级**
    // 无条件 reset。根因：本级守卫只比对 `project_to_units` 投影（有损——丢弃 `sub_moves`/`rmove.subs`）。
    // 当 L0 末段内点改写使上级投影 bit-identical 但底层 sub_moves 变时，上级 frontier_mutated=false 会
    // 漏 reset ⟹ `upper_moves` 深嵌套 sub_moves 陈旧 + BSP memo 复用陈旧（codex 反例
    // `cascade_reset_on_frontier_interior_rewrite` 坐实）。cascade 消除整类"投影是否捕获深字段变化"
    // 的易错判断（no-patch）：下级变异无条件向上传播，上级不依赖投影完备性。代价：变异 bar（16K 中
    // ~120 次稀疏）该级+所有上级全量重扫，amortized 仍 O(n)。
    let mut cascade_reset = false;
    // ★工位 4g：本 bar 是否有任一级 extend 非空 tail（含新级涌现首产 + 最高级 append）——驱动
    // generation +1（与 cascade_reset 一起完整覆盖 extract 可观察树变更，codex Q3）。
    let mut did_extend = false;
    // ★A3 证书（per-level dirty_from，§2.4）：本级 units 的不可变前缀长度。L0 = l0_dirty_from
    // （§2.3）；L≥1 = 父级 prefix_count（loop 尾 `dirty_from = prefix_count`）。驱动 03（L1+ stable
    // 从 0 抬起）+ 04（cached_units truncate+extend O(tail)）。
    let mut dirty_from: usize = l0_dirty_from;

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
        let scanned = (lc.scan_cursor.consumed + 2).min(units.len()).min(lc.cached_units.len());
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
        if cascade_reset {
            lc.scan_cursor = WindowScanCursor::default();
            // make_mut：若上 bar snapshot 仍持引用则写时复制再 clear（退化 bit-exact）；否则原地清。
            Rc::make_mut(&mut lc.upper_moves).clear();
            Rc::make_mut(&mut lc.centers).clear();
            lc.decompose_state.reset();
            Rc::make_mut(&mut lc.cached_bsp).clear();
            Rc::make_mut(&mut lc.cached_pan_div).clear(); // Q4：与 cached_bsp 同批失效（同 key 守卫）。
            lc.cached_bsp_key = None;
            lc.cached_second.clear(); // 07b 门控：前缀重排 ⟹ 前缀 B2 缓存失效，重扫。
            lc.cached_second_count = 0;
            lc.projected_units.clear(); // #106：upper_moves 前缀重排 ⟹ 投影缓存失效，重投影。
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
        if had_emitted_window {
            // pop 最后一个中枢（frontier 中枢，重扫会重新产出）——前缀不变量不破（pop 的是尾部）。
            debug_assert!(
                !lc.centers.is_empty() && !lc.upper_moves.is_empty(),
                "had_emitted_window ⟹ 至少一个已产出中枢可回退"
            );
            Rc::make_mut(&mut lc.centers).pop();
            Rc::make_mut(&mut lc.upper_moves).pop();
        }
        // ★codex Q4：prefix_count = lc.upper_moves.len()（pop 后的已产出前缀数），tail ordinal 接续
        // 前缀 ⟹ 全量/增量产同 ElementId（跨 bar 稳定身份）。★A3 §2.2：prefix_count 同时是本级
        // projected_units 的不可变前缀（§2.5 truncate 锚）+ 下一级 units 的 dirty_from（§2.4 loop 尾）。
        let prefix_count = lc.upper_moves.len();
        let (tail_centers, tail_upper, new_cursor) =
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
        stage_profile::time("06_extend_centers_upper", || {
            // make_mut：strong_count==1 ⟹ 原地 extend O(tail)；>1 ⟹ 写时复制（bit-exact）。
            Rc::make_mut(&mut lc.centers).extend(tail_centers);
            Rc::make_mut(&mut lc.upper_moves).extend(tail_upper);
        });
        lc.scan_cursor = new_cursor;
        debug_assert!(
            lc.centers.len() == lc.upper_moves.len(),
            "增量塔：centers 与 upper_moves 一一对应（每窗口一中枢）"
        );

        // 本级输入塔快照（compose 前）。
        // ★O(1) 优化：moves_tower 是 Rc——move 入 snapshots（所有权转移，零拷贝）。下一级用
        // Rc::clone(&lc.upper_moves) 重置 moves_tower（line 912），故此处 move 后 moves_tower 失效合法。
        // bit-exact：snapshots 内容 == 全量版（Rc 指向的 Vec 值不变，仅所有权/引用计数变）。
        tower_snapshots.push(std::mem::take(&mut moves_tower));

        // 走势分解（增量续折：resume 单一来源 ⟹ 与全量 decompose 定义性 bit-exact）。
        let moves = decompose_resume(&lc.centers, &mut lc.decompose_state);

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
        let struct_len = if is_l0 { l0.segments.len() } else { units.len() };
        let bsp_key = (lc.centers.len(), lc.upper_moves.len(), struct_len);
        let (bsp, pan_div): (Rc<Vec<BspPoint>>, Rc<Vec<signal::PanDivCert>>) = if lc.cached_bsp_key
            == Some(bsp_key)
        {
            // 07c：memo 命中 ⟹ `Rc::clone`（引用计数 O(1)），替代全量 `cached_bsp.clone()`。
            // Q4：pan_div 同批命中（同 key 守卫 ⟹ 同一 extract 产出的两半锁步复用）。
            stage_profile::time("07c_bsp_memo_clone", || {
                (Rc::clone(&lc.cached_bsp), Rc::clone(&lc.cached_pan_div))
            })
        } else {
            let (mut b, pan): (Vec<BspPoint>, Vec<signal::PanDivCert>) = if is_l0 {
                stage_profile::time("07a_extract_signals_l0", || {
                    // ★force_state 生产热路由（beta-route #115）：传真 dif/closes_tick ⟹ 一类候选
                    // point.force=Some（进 force_state 第 8 维）。结构六 bit 不变（force 不进分桶 key）。
                    signal::extract_signals_with_hist(&lc.centers, &l0.segments, hist, dif, &closes_tick, &close_src)
                })
            } else {
                stage_profile::time("07a_extract_first_third_ln", || {
                    // 级别-N 一/三类（裁定 A）：units 承担线段角色，复用 L0 判据（含 force）。memo miss 才重算
                    // （bsp_key 含 units.len，见上）；命中走 07c Rc::clone O(1)。units_L 随级别几何衰减
                    // ⟹ 每 miss O(units_L) 全扫，struct 变化次数 ≪ bar 数 ⟹ 摊还 O(n)（同 L0 memo 特性）。
                    extract_first_third_for_level(&lc.centers, &units, hist, dif, &closes_tick, &close_src)
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
            lc.cached_bsp = Rc::clone(&rc);
            lc.cached_pan_div = Rc::clone(&rc_pan); // Q4：与 bsp 同批缓存（同 key）。
            lc.cached_bsp_key = Some(bsp_key);
            (rc, rc_pan)
        };

        // 08：`Rc::clone`（O(1)）投影增量塔 centers 到 LevelState，替代全量 `centers.clone()`。
        let level_centers = stage_profile::time("08_levels_centers_clone", || Rc::clone(&lc.centers));
        levels.push(LevelState {
            moves,
            centers: level_centers,
            bsp,
            pan_div,
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
            lc.projected_units.truncate(prefix_count);
            // Q7（task #145）：方向源 = 本级中枢 ownership 块方向（levels 尾 = 本级刚 push 的
            // LevelState.moves，与 batch 同一 decompose 单一来源）。confirmed 前缀方向冻结
            // （R(i-1,i) sealed 后标签不变），frontier 由 truncate(prefix_count) 每 bar 重投影。
            recursive_tower::project_to_units_resume(
                &lc.upper_moves[..],
                &levels.last().expect("本级 LevelState 已 push").moves,
                &mut lc.projected_units,
            );
        });
        // ★A3 投影证书护栏（debug/test）：truncate(prefix_count)+resume 必逐字段等全量 project_to_units。
        debug_assert!(
            lc.projected_units
                == recursive_tower::project_to_units(
                    &lc.upper_moves,
                    &levels.last().expect("本级 LevelState 已 push").moves
                ),
            "投影证书违反：projected_units != 全量 project_to_units（truncate(prefix_count)/resume 破裂）"
        );
        units = stage_profile::time("10_projected_units_clone", || lc.projected_units.clone());

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

    // closes/close_src 缓冲放回 cache（mem::take 取出的所有权归还，下 bar 复用，零额外分配）。
    cache.closes = closes;
    cache.close_src = close_src;
    cache.closes_tick = closes_tick; // force 价格振幅/速度 proxy 缓冲放回，下 bar 复用（同 closes 模式）。
    cache.area_cache = area_cache.into_inner(); // B3 #4 area-memo：缓冲放回，下 bar 复用（同上模式）。

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

    (Classification { levels }, tower_snapshots)
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
        second_for_parent(parent, hist, close_src, &area_cache, stable_len, &mut points);
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
/// ★门控消解的是「confirmed 前缀 parent 的重复扫描」这一 O(U²) 源。**残余 O(n²) 仍在**（profile 坐实
/// 门控后 90ms@300K→1026ms@1M 仍指数≈2.03）——根因是 frontier parent 的 `segments_diverge` MACD 面积
/// 累加：其对照走势 `prev` 的 close 区间随 n 增长，`|hist|` 逐点求和 O(range) 随窗口线性增长。这是
/// **B3 #4 area-memo（(start,end)→area 冻结缓存）的领域**，门控消除 confirmed 前缀重扫恰是 B3 判定的
/// area-memo 翻转条件；两者正交叠加才能把 07b 完全线性化（本工位只做门控，area-memo 未接入）。
/// 缓存 clone O(|前缀 B2|) 在此非瓶颈（profile 坐实与原地增量版 1022ms 同噪声，area 累加主导）。
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
/// 同向 = 走势的 direction 相同（`RMove::Segment.direction`；`Compose` 走势取外缘趋势方向占位）。
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
    let curr_dir = rmove_direction(&curr.rmove);
    // 序列序最近同向前驱走势（reference:34「末段相对前同向段」的确定配对）。
    let Some(prev) = subs[..idx].iter().rev().find(|x| rmove_direction(&x.rmove) == curr_dir) else {
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

/// 走势方向（`RMove::Segment` 直接取 direction；`Compose` 取外缘趋势方向占位——首子升=Up）。
///
/// ★诚实有效域：`Compose` 走势的方向是**外缘占位**（subs 区间聚合趋势），用于背驰「同向段」配对的
/// 序列序判定。不冒充 §6.1 意义的线段方向交替（中枢检测用几何路径，不读方向，见 center.rs）。
fn rmove_direction(m: &descend::RMove) -> Direction {
    match m {
        descend::RMove::Segment { direction, .. } => *direction,
        // Compose 走势：外缘下沿 vs 上沿——hi 偏离 lo 多者为趋势向（占位，背驰同向配对用）。
        // subs 首尾区间趋势：末子 hi >= 首子 hi ⟹ Up（外缘上移），否则 Down。
        descend::RMove::Compose { subs, .. } => {
            match (subs.first(), subs.last()) {
                (Some(f), Some(l)) if l.hi() >= f.hi() => Direction::Up,
                (Some(_), Some(_)) => Direction::Down,
                _ => Direction::Up, // 空 subs ⟹ 缺省 Up（防御性）。
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::MoveKind;
    use super::*;
    use super::super::parser::ParseLayer;
    use super::super::types::Direction;

    fn seg(dir: Direction, si: usize, ei: usize, sp: i64, ep: i64) -> Segment {
        Segment { direction: dir, start_index: si, end_index: ei, start_price: sp, end_price: ep }
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
            assert_eq!(cache.macd_dif(), full.dif.as_slice(), "bar {k}: dif 增量 ≠ 全量（bit-exact 破）");
            assert_eq!(cache.macd_hist_for_test(), full.hist.as_slice(), "bar {k}: hist 增量 ≠ 全量");
            assert_eq!(cache.macd_dif().len(), cache.macd_hist_for_test().len(), "dif/hist 锁步等长");
        }

        // ── closes_tick 增量 == merged_bars.close（整数域，force 振幅/速度 proxy 输入）──
        let bars = bars_from_closes(&vals);
        let mut cache2 = TowerCache::new();
        for k in 1..=bars.len() {
            update_closes_cache(&bars[..k], k.saturating_sub(1), &mut cache2);
            let expect: Vec<Tick> = bars[..k].iter().map(|b| b.close).collect();
            assert_eq!(cache2.closes_tick(), expect.as_slice(), "bar {k}: closes_tick ≠ merged_bars.close");
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
            seg(Direction::Up,   0,  4, 110, 150),
            seg(Direction::Down, 4,  8, 150, 120),
            seg(Direction::Up,   8, 12, 120, 148),
            // 组B（L1[1]）：down-up-down，首段 hi=115<ZD_A=120 non-ext，lo=80 深破 L2 核心下沿 112
            seg(Direction::Down,12, 16, 115,  80),
            seg(Direction::Up,  16, 20,  80, 125),
            seg(Direction::Down,20, 24, 114,  85),
            // 组C（L1[2]）：up-down-up，首段 lo=115>ZG_B=114 non-ext，回拉不创新低（lo=112 >= 80）
            seg(Direction::Up,  24, 28, 115, 148),
            seg(Direction::Down,28, 32, 148, 112),
            seg(Direction::Up,  32, 36, 112, 147),
        ];
        // closes 让 L1[1] 区间（source_index [12,24]）MACD 面积 < L1[0] 区间（[0,12]）= 背驰（真算）。
        // 前段大幅波动（面积大），后段小幅（面积小）。
        let mut closes: Vec<i64> = Vec::new();
        for i in 0..12 { closes.push(100 + if i % 2 == 0 { 40 } else { -40 }); } // L1[0] 大幅
        for i in 0..12 { closes.push(100 + if i % 2 == 0 { 5 } else { -5 }); }   // L1[1] 小幅（背驰）
        for i in 0..16 { closes.push(100 + if i % 2 == 0 { 3 } else { -3 }); }   // L1[2] 更小
        let layer = ParseLayer { segments: Rc::new(segments), merged_bars: Rc::new(bars_from_closes(&closes)), ..Default::default() };
        let out = classify(&layer, &cfg);

        // L1 级别（索引 1）含 L2 中枢 + B2（递归组装层产出）。
        assert!(out.levels.len() >= 2, "三组 L0 → L1 走势塔 → L2 中枢，至少 2 级");
        let l1 = &out.levels[1];
        assert_eq!(l1.centers.len(), 1, "3 个 L1 走势 → 1 个 L2 中枢（几何路径）");
        let second_buys: Vec<_> = l1.bsp.iter().filter(|p| p.bits.buy2).collect();
        assert_eq!(second_buys.len(), 1, "★升级后塔真产 B2（旧 UnitRange 塔产 0）");
        let b2 = second_buys[0];
        // B2 端点坐标由 source_index 侧车真映射（回拉走势 L1[2] 的 end_index=36）。
        assert_eq!(b2.source_index, 36, "B2 source_index = 回拉走势 L1[2] 的原始 K 序（坐标侧车真映射）");
        // 第二类止损 = 回拉低点（second_point = 回拉走势 m2.lo）；center=None（1/2 类用 pivot 非 center）。
        assert!(b2.pivot_low != 0, "B2 携结构止损价 pivot_low（回拉低点 single source）");
        assert!(b2.center.is_none(), "1/2 类止损用 pivot 非 center ⟹ center=None");
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
            seg(Direction::Up,   0,  4, 100, 200),
            seg(Direction::Down, 4,  8, 200, 100),
            seg(Direction::Up,   8, 12, 100, 200),
        ];
        let closes: Vec<i64> = (0..16).map(|i| 100 + (i % 4) * 10).collect();
        let layer = ParseLayer { segments: Rc::new(segments), merged_bars: Rc::new(bars_from_closes(&closes)), ..Default::default() };
        let out = classify(&layer, &cfg);
        // L0 级别 bsp 不含第二类（三段交替窗口结构上界）。
        for p in out.levels[0].bsp.iter() {
            assert!(!p.bits.buy2, "L0→L1 三段交替窗口不产 B2（still-MISSING-窗口，codex 裁决）");
            assert!(!p.bits.sell2, "L0→L1 三段交替窗口不产 S2（still-MISSING-窗口，codex 裁决）");
        }
    }

    /// ★codex-decide-20260703 裁定 A 最大实现风险点（end_price 忠实性，单测强制）：级别-N 输入单元
    /// → Segment 的端点价按 fold_direction 取 hi/lo，且与 `segment_to_unit` 互逆（L0 段 round-trip
    /// bit-exact）。此测试失败 ⟹ 级别-N「线段」端点价错位 ⟹ A/C 破中枢几何 + judge_third 判据全错。
    #[test]
    fn unit_to_segment_endpoint_faithful_and_roundtrips() {
        use super::center::UnitRange;
        // 向上单元：起点=lo、终点=hi（seg_end 取 end_price=hi=高点）。
        let up = UnitRange { start_index: 4, end_index: 8, direction: Direction::Up, lo: 90, hi: 150 };
        let s_up = unit_to_segment(&up);
        assert_eq!((s_up.start_price, s_up.end_price), (90, 150), "向上单元 end_price=hi（终点=高点）");
        assert_eq!(s_up.direction, Direction::Up);
        assert_eq!((s_up.start_index, s_up.end_index), (4, 8), "source_index 坐标保留（A/C 面积映射用）");
        // 向下单元：起点=hi、终点=lo（终点=低点）。
        let down = UnitRange { start_index: 8, end_index: 12, direction: Direction::Down, lo: 90, hi: 150 };
        let s_down = unit_to_segment(&down);
        assert_eq!((s_down.start_price, s_down.end_price), (150, 90), "向下单元 end_price=lo（终点=低点）");
        // round-trip：L0 段 → segment_to_unit → unit_to_segment == 原段（互逆 bit-exact）。
        for orig in [seg(Direction::Up, 0, 4, 100, 200), seg(Direction::Down, 4, 8, 200, 50)] {
            let back = unit_to_segment(&segment_to_unit(&orig));
            assert_eq!(
                (back.direction, back.start_index, back.end_index, back.start_price, back.end_price),
                (orig.direction, orig.start_index, orig.end_index, orig.start_price, orig.end_price),
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
        let c0 = Center { zd: 300, zg: 400, dd: 290, gg: 410, start_index: 0, end_index: 2 };
        let c1 = Center { zd: 100, zg: 200, dd: 90, gg: 210, start_index: 0, end_index: 8 };
        // Down 单元：lo=终点价、hi=起点价（unit_to_segment 还原 start=hi/end=lo）。
        let units = vec![
            UnitRange { start_index: 3, end_index: 5, direction: Direction::Down, lo: 250, hi: 350 }, // A 段（C0 离开）
            UnitRange { start_index: 5, end_index: 7, direction: Direction::Up, lo: 250, hi: 280 },   // B 段连接
            UnitRange { start_index: 9, end_index: 11, direction: Direction::Down, lo: 80, hi: 150 }, // C 段破 C1（<100）
        ];
        // A 段 bar[3,5] 急跌（hist 面积大）、C 段 bar[9,11] 缓动（面积小=背驰）——同 signal.rs fixture。
        let prices: Vec<i64> = vec![300, 300, 300, 300, 100, 250, 250, 250, 250, 248, 246, 244];
        let closes: Vec<f64> = prices.iter().map(|&v| v as f64).collect();
        let close_src: Vec<usize> = (0..prices.len()).collect();
        let hist = divergence::compute_macd(&closes, &ThetaConfig::default().macd).hist;
        // 本测试只验结构六 bit（force 旁挂不改），传空 dif/closes_tick ⟹ force=None（不影响 buy1 判据）。
        let (bsp, _pan) = extract_first_third_for_level(&[c0, c1], &units, &hist, &[], &[], &close_src);
        let buy1: Vec<_> = bsp.iter().filter(|p| p.bits.buy1).collect();
        assert_eq!(buy1.len(), 1, "级别-N 下跌趋势 C 段破最后中枢 ∧ C<A 背驰 ⟹ 一个 1 买（缺口已填，非 no-op）");
        assert_eq!(buy1[0].source_index, 11, "1 买端点 = C 段（破最后中枢单元）终止 source_index");
        assert_eq!(buy1[0].pivot_low, 80, "1 买止损源 = pivot_low（C 段破中枢端点极值）");
        assert!(buy1[0].center.is_none(), "1 类止损用 pivot 非 center ⟹ center=None");
    }

    /// ★裁定 A 三类（高级别「中枢外缘区间」边界语义，codex 风险点单独 snapshot）：级别-N 离开中枢
    /// + 回试不重入 ⟹ 3 买。`judge_third` 在级别-N units（外缘区间端点 hi/lo）vs 几何中枢 [zd,zg]
    /// 上判定——离开单元终点 > c.zg ∧ 回试单元终点 > c.zg（严格不触闭区间）。
    #[test]
    fn level_ge1_extract_first_third_produces_type3_via_units() {
        use super::center::UnitRange;
        // 单中枢 [100,200]（盘整 τ ⟹ 无一类）——三类是纯几何位置判据，不依赖趋势门控。
        let c = Center { zd: 100, zg: 200, dd: 90, gg: 210, start_index: 0, end_index: 12 };
        let units = vec![
            // 离开单元：向上，终点=hi=250 > zg=200（离开中枢上方）。
            UnitRange { start_index: 12, end_index: 16, direction: Direction::Up, lo: 150, hi: 250 },
            // 回试单元：向下，终点=lo=210 > zg=200（不重入闭区间中枢）⟹ 3 买。
            UnitRange { start_index: 16, end_index: 20, direction: Direction::Down, lo: 210, hi: 250 },
        ];
        // 三类无 MACD 依赖（纯整数几何），hist 空亦可——传空 hist/dif/closes_tick（第一类自然不产，force=None）。
        let (bsp, _pan) = extract_first_third_for_level(&[c], &units, &[], &[], &[], &(0..24).collect::<Vec<_>>());
        let buy3: Vec<_> = bsp.iter().filter(|p| p.bits.buy3).collect();
        assert_eq!(buy3.len(), 1, "级别-N 离开中枢 + 回试不重入 ⟹ 一个 3 买（外缘区间端点判据）");
        assert_eq!(buy3[0].source_index, 20, "3 买端点 = 回试单元终止 source_index");
        assert_eq!(buy3[0].pivot_low, 210, "3 买止损源 = pivot_low（回试低点）");
        assert_eq!(buy3[0].center.map(|c| c.zg), Some(200), "3 买 center=Some（止损=zg）");
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
        let full = load_by_symbol("BTC", &cfg).expect("BTC 数据加载（analysis/data_cache/btc_1m_full.json）");
        // 可选日期窗（CENSUS_WINDOW="2020-10-01,2021-04-01"）——检验 type1 的水平线依赖性：
        // 全历史中枢链全局非单调 ⟹ trend_class=Degenerate ⟹ type1=0；单向牛/熊窗内某级链可单调 ⟹ type1>0。
        let ds = match std::env::var("CENSUS_WINDOW") {
            Ok(w) => {
                let (s, e) = w.split_once(',').expect("CENSUS_WINDOW 格式 start,end");
                eprintln!("[census] window={s}..{e}");
                full.slice_date_window(s, e)
            }
            Err(_) => full,
        };
        eprintln!("[census] BTC bars={}", ds.bars.len());
        let layer = parse_layer(&ds.bars, &cfg);
        eprintln!("[census] L0 segments={} merged_bars={}", layer.segments.len(), layer.merged_bars.len());
        let out = classify(&layer, &cfg);
        eprintln!("[census] levels={}", out.levels.len());
        for (li, lv) in out.levels.iter().enumerate() {
            let trend = lv.moves.iter().filter(|m| m.kind == MoveKind::Trend).count();
            let (mut b1, mut s1, mut b2, mut s2, mut b3, mut s3) = (0, 0, 0, 0, 0, 0);
            for p in lv.bsp.iter() {
                b1 += p.bits.buy1 as usize; s1 += p.bits.sell1 as usize;
                b2 += p.bits.buy2 as usize; s2 += p.bits.sell2 as usize;
                b3 += p.bits.buy3 as usize; s3 += p.bits.sell3 as usize;
            }
            eprintln!(
                "[census] L{li}: centers={} moves={}(trend={}) bsp={} pan_div={} | buy1={b1} sell1={s1} buy2={b2} sell2={s2} buy3={b3} sell3={s3}",
                lv.centers.len(), lv.moves.len(), trend, lv.bsp.len(), lv.pan_div.len()
            );
            // 抽样：level≥1 的前 3 个一类端点（若有）+ 前 3 个三类端点（人工核结构合法性——
            // source_index + center[zd,zg] + pivot（回试端点极值）；三类不重入判据由 judge_third 保证）。
            if li >= 1 {
                let t1: Vec<_> = lv.bsp.iter().filter(|p| p.bits.buy1 || p.bits.sell1).take(3).collect();
                for (k, p) in t1.iter().enumerate() {
                    eprintln!(
                        "[census]   L{li} type1#{k}: src_idx={} buy1={} sell1={} break_dir={:?} pivot_low={} pivot_high={}",
                        p.source_index, p.bits.buy1, p.bits.sell1, p.struct_break_dir, p.pivot_low, p.pivot_high
                    );
                }
                let t3: Vec<_> = lv.bsp.iter().filter(|p| p.bits.buy3 || p.bits.sell3).take(3).collect();
                for (k, p) in t3.iter().enumerate() {
                    eprintln!(
                        "[census]   L{li} type3#{k}: src_idx={} buy3={} sell3={} center_zd={:?} center_zg={:?} pivot_low={} pivot_high={}",
                        p.source_index, p.bits.buy3, p.bits.sell3,
                        p.center.map(|c| c.zd), p.center.map(|c| c.zg), p.pivot_low, p.pivot_high
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
        let full = load_by_symbol("BTC", &cfg).expect("BTC 数据加载（analysis/data_cache/btc_1m_full.json）");
        let ds = match std::env::var("CENSUS_WINDOW") {
            Ok(w) => {
                let (s, e) = w.split_once(',').expect("CENSUS_WINDOW 格式 start,end");
                eprintln!("[funnel] window={s}..{e}");
                full.slice_date_window(s, e)
            }
            Err(_) => full,
        };
        eprintln!("[funnel] BTC bars={}", ds.bars.len());
        let layer = parse_layer(&ds.bars, &cfg);
        eprintln!("[funnel] L0 segments={} merged_bars={}", layer.segments.len(), layer.merged_bars.len());

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
                .map(|(i, u)| LeveledMove::from_unit(u, ElementId { level: 0, ordinal: i as u64 }))
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
                    hist[if w <= 3 { 0 } else if w <= 5 { 1 } else if w <= 9 { 2 } else { 3 }] += 1;
                }
                eprintln!(
                    "[funnel] L{level_idx}: 窗口段数分布 =3段:{} 4-5:{} 6-9:{} ≥10:{} max={}",
                    hist[0], hist[1], hist[2], hist[3], max_w
                );
            }

            // 中枢链相邻关系直方图 + 前缀 τ 时间线 + 反事实局部同向 run。
            let rels: Vec<CenterRelation> =
                centers.windows(2).map(|w| classify_relation(&w[0], &w[1])).collect();
            let n_up = rels.iter().filter(|r| **r == CenterRelation::UpContinuation).count();
            let n_down = rels.iter().filter(|r| **r == CenterRelation::DownContinuation).count();
            let n_exp = rels.iter().filter(|r| **r == CenterRelation::LevelExpansion).count();
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
                    let date = ds.dates.get(c_end).map(|d| d.get(..10).unwrap_or("?")).unwrap_or("?");
                    format!("中枢#{}（end_src={} {date}）", i + 1, c_end)
                }
            };

            let segs: Vec<Segment> = if is_l0 {
                layer.segments.to_vec()
            } else {
                units.iter().map(unit_to_segment).collect()
            };
            let f = signal::type1_funnel_dx(&centers, &segs, &series.hist, &series.dif, &closes_tick, &close_src);
            eprintln!(
                "[funnel] L{level_idx}: centers={} segs={} rel(up/down/exp)={}/{}/{} blocks(trend/consol)={}/{} 最长趋势块={}中枢 | 旧AllTrend锁死点={} 局部同向run≥2中枢数={} 最长run={}(={}中枢)",
                f.n_centers, f.n_segments, n_up, n_down, n_exp, f.n_trend_blocks, f.n_consol_blocks,
                f.longest_trend_run, lock_desc, runs_ge1, longest_run, longest_run + 1
            );
            eprintln!(
                "[funnel] L{level_idx}: 环0候选(有最近中枢)={} → 环1有前驱中枢={} → 环2过局部趋势门={} → 环3破最后中枢={} → 环4 A/C配对={} → 环5坐标映射={} → 环6背驰C<A={}",
                f.s_with_center, f.s_pos_ge1, f.s_gate_open, f.s_broke, f.s_a_paired, f.s_mapped, f.s_diverge
            );
            // ★task #144 验收证据：2021 顶区 sell1 在全历史因果重放中出现（先例窗反差闭合的正验证，
            // 生产 classify 输出直读——非探针另算）。窗口 = #141 外审切窗 2020-10-01..2021-04-15。
            {
                let top_sell1: Vec<&str> = out.levels[level_idx]
                    .bsp
                    .iter()
                    .filter(|p| p.bits.sell1)
                    .filter_map(|p| ds.dates.get(p.source_index).map(|d| d.get(..10).unwrap_or("?")))
                    .filter(|d| ("2020-10-01".."2021-04-15").contains(d))
                    .collect();
                let n_sell1 =
                    out.levels[level_idx].bsp.iter().filter(|p| p.bits.sell1).count();
                let n_buy1 = out.levels[level_idx].bsp.iter().filter(|p| p.bits.buy1).count();
                eprintln!(
                    "[funnel] L{level_idx}: 全历史 buy1={} sell1={} | 2021顶区(2020-10-01..2021-04-15) sell1×{}: {:?}",
                    n_buy1, n_sell1, top_sell1.len(), top_sell1
                );
            }

            let (_cw, upper_moves) = compose_level(&units, &moves_tower[..], is_l0, level_idx as u32 + 1);
            units = project_to_units(&upper_moves, &out.levels[level_idx].moves); // Q7：生产同源块
            moves_tower = Rc::new(upper_moves);
            if units.is_empty() {
                break;
            }
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
            segments: Rc::new(vec![seg(Direction::Up, 0, 4, 0, 10), seg(Direction::Down, 4, 8, 10, 5)]),
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
        assert_eq!((l0.moves[0].kind, l0.moves[0].start_center, l0.moves[0].end_center),
                   (MoveKind::Consolidation, 0, 0));
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
        assert!(out.levels[0].centers.is_empty(), "同向三段无方向交替 ⟹ 完整判据拒绝（非中枢）");
    }

    #[test]
    fn lmax_bound_respected() {
        // 构造大量重叠段，验证递归不超过 l_max+1 级（每级至少消耗中枢，最终自然终止）。
        let cfg = ThetaConfig::default();
        let mut segments = Vec::new();
        // 27 段全重叠区间 [0,100]（每三段成一中枢，逐级递归）。
        for i in 0..27 {
            let dir = if i % 2 == 0 { Direction::Up } else { Direction::Down };
            segments.push(seg(dir, i * 4, i * 4 + 4, 0, 100));
        }
        let layer = ParseLayer { segments: Rc::new(segments), ..Default::default() };
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
                seg(Direction::Up, 12, 16, 205, 250),   // 离开中枢上方（lo=205>ZG ⟹ non-extension）
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
        assert_eq!(p.center.map(|c| c.zg), Some(200), "3 买止损 = ZG single source");
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
        assert!(out.levels[0].bsp.is_empty(), "无离开/回试 ⟹ 无买卖点（诚实空）");
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
            seg(Direction::Up,   0,  4, 110, 150),
            seg(Direction::Down, 4,  8, 150, 120),
            seg(Direction::Up,   8, 12, 120, 148),
            seg(Direction::Down,12, 16, 115,  80),
            seg(Direction::Up,  16, 20,  80, 125),
            seg(Direction::Down,20, 24, 114,  85),
            seg(Direction::Up,  24, 28, 115, 148),
            seg(Direction::Down,28, 32, 148, 112),
            seg(Direction::Up,  32, 36, 112, 147),
        ];
        let mut closes: Vec<i64> = Vec::new();
        for i in 0..12 { closes.push(100 + if i % 2 == 0 { 40 } else { -40 }); }
        for i in 0..12 { closes.push(100 + if i % 2 == 0 {  5 } else {  -5 }); }
        for i in 0..16 { closes.push(100 + if i % 2 == 0 {  3 } else {  -3 }); }
        let layer = ParseLayer { segments: Rc::new(segments), merged_bars: Rc::new(bars_from_closes(&closes)), ..Default::default() };
        let (_, tower) = classify_with_tower(&layer, &cfg);
        assert!(!tower.is_empty(), "tower 非空（至少 L0 级被处理）");
        assert!(tower.len() >= 2, "9 段 L0 → 3 个 L1 走势 → L2 中枢 ⟹ tower 至少 2 层");
        // depth≥1 真嵌套：tower[1] 含 sub_moves 非空的 LeveledMove（RMove::Compose，L1 输入塔）。
        let has_true_nesting = tower[1].iter().any(|m| !m.sub_moves.is_empty());
        assert!(has_true_nesting, "tower[1] 含真嵌套 LeveledMove（sub_moves 非空，depth≥1）");
    }

    /// ★classify_with_tower Classification 与 classify 同输入 bit-identical（导出不改原分类）。
    #[test]
    fn classify_with_tower_classification_equals_classify() {
        let cfg = ThetaConfig::default();
        // ★task #142 延伸语义诚实重算（同 end_to_end_second_buy_via_l1_l2_geometric 推导）：三组
        // 核心分离（组B 首段 hi=115<ZD_A=120、组C 首段 lo=115>ZG_B=114 ⟹ non-extension，PDF §5
        // Step3），外缘 O_A=[110,150]/O_B=[80,125]/O_C=[112,148] 共同相交 ⟹ L2 核心 [112,125] 非空。
        let segments = vec![
            seg(Direction::Up,   0,  4, 110, 150),
            seg(Direction::Down, 4,  8, 150, 120),
            seg(Direction::Up,   8, 12, 120, 148),
            seg(Direction::Down,12, 16, 115,  80),
            seg(Direction::Up,  16, 20,  80, 125),
            seg(Direction::Down,20, 24, 114,  85),
            seg(Direction::Up,  24, 28, 115, 148),
            seg(Direction::Down,28, 32, 148, 112),
            seg(Direction::Up,  32, 36, 112, 147),
        ];
        let mut closes: Vec<i64> = Vec::new();
        for i in 0..12 { closes.push(100 + if i % 2 == 0 { 40 } else { -40 }); }
        for i in 0..12 { closes.push(100 + if i % 2 == 0 {  5 } else {  -5 }); }
        for i in 0..16 { closes.push(100 + if i % 2 == 0 {  3 } else {  -3 }); }
        let layer = ParseLayer { segments: Rc::new(segments), merged_bars: Rc::new(bars_from_closes(&closes)), ..Default::default() };
        let expected = classify(&layer, &cfg);
        let (actual, _) = classify_with_tower(&layer, &cfg);
        assert_eq!(actual, expected, "classify_with_tower Classification 与 classify bit-identical（原分类不变）");
    }

    // ──────────────────────────────────────────────────────────────────────
    //  增量塔 API bit-exact（task #93：incremental == 全量，逐 bar 断言）
    // ──────────────────────────────────────────────────────────────────────

    /// 构造逐段追加的合成段序列（方向交替 + 价格震荡，产足够中枢触发多级塔）。
    fn synthetic_segments(count: usize) -> Vec<Segment> {
        (0..count)
            .map(|i| {
                let dir = if i % 2 == 0 { Direction::Up } else { Direction::Down };
                let base = 100i64 + (i as i64) * 3;
                let swing = if i % 2 == 0 { 50 } else { -50 };
                let sp = base;
                let ep = base + swing;
                seg(dir, i * 4, i * 4 + 4, sp, ep)
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
            let layer =
                ParseLayer { segments: Rc::new(segments), merged_bars: Rc::new(bars_from_closes(&closes)), ..Default::default() };

            let (full_cls, full_tower) = classify_with_tower(&layer, &cfg);
            let mut cache = TowerCache::new();
            let (inc_cls, inc_tower) = classify_with_tower_incremental(&layer, &cfg, &mut cache);

            assert_eq!(inc_cls, full_cls, "n={n}: 增量 Classification == 全量");
            assert_eq!(inc_tower.len(), full_tower.len(), "n={n}: 增量 tower 层数 == 全量");
            for (lvl, (il, fl)) in inc_tower.iter().zip(full_tower.iter()).enumerate() {
                assert_eq!(il, fl, "n={n} level {lvl}: 增量 tower 级 LeveledMove 序列 == 全量");
            }
        }
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

            assert_eq!(inc_cls, full_cls, "n={n}: 增量 Classification != 全量（bit-exact 破裂）");
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
        let layer_full =
            ParseLayer { segments: Rc::new(all_segments.clone()), merged_bars: Rc::new(bars_from_closes(&closes)), ..Default::default() };
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
            seg(Direction::Up,   0,  4, 110, 150),
            seg(Direction::Down, 4,  8, 150, 120),
            seg(Direction::Up,   8, 12, 120, 148),
            seg(Direction::Down,12, 16, 115,  80),
            seg(Direction::Up,  16, 20,  80, 125),
            seg(Direction::Down,20, 24, 114,  85),
            seg(Direction::Up,  24, 28, 115, 148),
            seg(Direction::Down,28, 32, 148, 112),
            seg(Direction::Up,  32, 36, 112, 147),
        ];
        let mut closes: Vec<i64> = Vec::new();
        for i in 0..12 { closes.push(100 + if i % 2 == 0 { 40 } else { -40 }); }
        for i in 0..12 { closes.push(100 + if i % 2 == 0 {  5 } else {  -5 }); }
        for i in 0..16 { closes.push(100 + if i % 2 == 0 {  3 } else {  -3 }); }
        let layer = ParseLayer { segments: Rc::new(segments), merged_bars: Rc::new(bars_from_closes(&closes)), ..Default::default() };

        let (full_cls, _) = classify_with_tower(&layer, &cfg);
        let mut cache = TowerCache::new();
        let (inc_cls, _) = classify_with_tower_incremental(&layer, &cfg, &mut cache);

        // 全量产 1 个 B2（见 end_to_end_second_buy_via_l1_l2_geometric），增量须 bit-identical。
        let full_b2: Vec<_> = full_cls.levels[1].bsp.iter().filter(|p| p.bits.buy2).collect();
        let inc_b2: Vec<_> = inc_cls.levels[1].bsp.iter().filter(|p| p.bits.buy2).collect();
        assert_eq!(inc_b2.len(), full_b2.len(), "增量塔 B2 数量 == 全量（真 Fugue 547 保留）");
        assert_eq!(inc_b2.len(), 1, "增量塔仍真产 B2（subs 真 Compose，非级别差伪造）");
        assert_eq!(inc_b2[0].source_index, full_b2[0].source_index, "B2 source_index bit-exact");
        assert_eq!(inc_cls, full_cls, "增量塔完整 Classification == 全量（含 B2 BSP）");
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
            seg(Direction::Up,   0,  4, 110, 150),
            seg(Direction::Down, 4,  8, 150, 120),
            seg(Direction::Up,   8, 12, 120, 148),
            seg(Direction::Down,12, 16, 115,  80),
            seg(Direction::Up,  16, 20,  80, 125),
            seg(Direction::Down,20, 24, 114,  85),
            seg(Direction::Up,  24, 28, 115, 148),
            seg(Direction::Down,28, 32, 148, 112),
            seg(Direction::Up,  32, 36, 112, 147), // v1 末段
        ];
        let mut closes: Vec<i64> = Vec::new();
        for i in 0..12 { closes.push(100 + if i % 2 == 0 { 40 } else { -40 }); }
        for i in 0..12 { closes.push(100 + if i % 2 == 0 {  5 } else {  -5 }); }
        for i in 0..16 { closes.push(100 + if i % 2 == 0 {  3 } else {  -3 }); }
        let merged = Rc::new(bars_from_closes(&closes));

        // v1: 末段 end_price=147。
        let layer_v1 = ParseLayer { segments: Rc::new(base.clone()), merged_bars: merged.clone(), ..Default::default() };
        // v2: 仅末段 end_price 改写 147→140（组 C 外缘内点，L1 投影不变，L0 sub_moves 变）。
        let mut v2_segs = base.clone();
        v2_segs[8].end_price = 140;
        let layer_v2 = ParseLayer { segments: Rc::new(v2_segs), merged_bars: merged.clone(), ..Default::default() };

        // 前提自检（codex 反例成立的必要条件）：L1 投影输入 v1==v2 bit-identical（守卫看不到变异），
        // 但 L0 末段 sub_moves 已变（147→140）。若此前提不成立，本测试不构成反例。
        let mk_l1_units = |segs: &Rc<Vec<Segment>>| {
            let l0_units: Vec<UnitRange> = segs.iter().map(segment_to_unit).collect();
            let moves_l0: Vec<LeveledMove> = l0_units.iter().enumerate()
                .map(|(i,u)| LeveledMove::from_unit(u, recursive_tower::ElementId{level:0,ordinal:i as u64})).collect();
            let (c, upper) = recursive_tower::compose_level(&l0_units, &moves_l0, true, 1);
            recursive_tower::project_to_units(&upper, &decompose::decompose(&c))
        };
        assert_eq!(mk_l1_units(&layer_v1.segments), mk_l1_units(&layer_v2.segments),
            "前提：L1 投影输入 v1==v2（守卫的本级投影比对看不到此变异）");

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
        let l0_seg8_full = classify_with_tower(&layer_v2, &cfg).1[0].last().unwrap().rmove.clone();
        assert_eq!(l0_seg8_full, descend::RMove::Segment { direction: Direction::Up, lo: 112, hi: 140 },
            "前提：v2 全量 L0 末段 == [112,140]");
        // cache.L1.upper_moves[0].sub_moves[2](groupC).sub_moves[2](seg[8]) 应 == [112,140]。
        let l1_deep = &cache.levels[1].upper_moves[0].sub_moves[2].sub_moves[2].rmove;
        assert_eq!(*l1_deep, descend::RMove::Segment { direction: Direction::Up, lo: 112, hi: 140 },
            "cascade: cache L1 深嵌套 seg[8] == v2 [112,140]（陈旧则 [112,147]——L1 漏 cascade reset）");

        // 返回值也须 bit-exact（cascade 后 L1 重建，tower/Classification 全对齐）。
        assert_eq!(inc_v2, full_v2, "cascade: v2 增量 Classification == 全量");
        assert_eq!(inc_tower_v2.len(), full_tower_v2.len(), "cascade: tower 层数 == 全量");
        for (lvl, (il, fl)) in inc_tower_v2.iter().zip(full_tower_v2.iter()).enumerate() {
            assert_eq!(il, fl, "cascade: level {lvl} LeveledMove == 全量");
        }
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
        let full_exp = (full_times[2] / full_times[0]).ln() / (sizes[2] as f64 / sizes[0] as f64).ln();
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
    use super::*;
    use super::super::backtest::data;
    use super::super::parser;

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
                eprintln!("DATA LIMIT: n={n} > oos.bars.len()={}，跳过", oos.bars.len());
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
            eprintln!("n={n} done: full={:.2}s inc={:.2}s", *full_times.last().unwrap(), *inc_times.last().unwrap());
        }

        // 逐相邻对算 exp（log-log 斜率），大规模验证 acceptance[4]。
        eprintln!("\n===== 增量塔真实标度（CL per-bar 累积，大规模）=====");
        for w in used_sizes.windows(2) {
            let (n0, n1) = (w[0], w[1]);
            let i0 = used_sizes.iter().position(|&s| s == n0).unwrap();
            let i1 = i0 + 1;
            let full_exp = (full_times[i1] / full_times[i0].max(1e-12)).ln()
                / (n1 as f64 / n0 as f64).ln();
            let inc_exp = (inc_times[i1] / inc_times[i0].max(1e-12)).ln()
                / (n1 as f64 / n0 as f64).ln();
            eprintln!(
                "  [{n0}→{n1}] full exp≈{full_exp:.2}  inc exp≈{inc_exp:.2}  \
                 增量/全量比 @{n1}: {:.2}x",
                inc_times[i1] / full_times[i1].max(1e-12)
            );
        }
    }
}
