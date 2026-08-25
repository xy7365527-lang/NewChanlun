//! #1080 S2 Lean 镜像验收的只读捕获 seam。
//!
//! 默认恒关闭。验收 bin 只在 checkpoint 那一根 bar 前调用 [`begin_capture`]，生产
//! `merged_scan_resume` 返回时经 [`record_scan`] 复制同一次调用的 raw 输入与 3a 四产口，
//! 随后 [`take_capture`] 取走。此模块不重判、不筛选、不改写任何生产对象。

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::theta_v0::classifier::cand_event::CandidateObservation;
use crate::theta_v0::classifier::center::UnitRange;
use crate::theta_v0::classifier::decompose::MoveBlock;
use crate::theta_v0::classifier::descend::RMove;
use crate::theta_v0::classifier::divergence::DivergenceGauge;
use crate::theta_v0::classifier::recursive_tower::{
    advance_cp_lifecycles, CpLifecycleStatus, CpScanOwnership, CpStructureIdentity, ElementId,
    FullTrendCQualified, FullTrendQualificationEvidence, LeveledMove, ThirdClassInCp,
};
use crate::theta_v0::classifier::signal::{
    extract_second_signals, judge_first_from_gates, FirstClassGradeRecord, FirstStructuralGates,
    PanDivCert, T3InCScan,
};
use crate::theta_v0::classifier::BspPoint;
use crate::theta_v0::types::{Center, Direction, Segment, Side, Stroke, Tick};

#[derive(Debug, Clone, PartialEq)]
pub struct ScanCapture {
    pub level: u32,
    pub centers: Vec<Center>,
    pub segments: Vec<Segment>,
    pub anchors: Vec<Option<Direction>>,
    pub departure_ends: Option<Vec<usize>>,
    pub blocks: Vec<MoveBlock>,
    pub points: Vec<BspPoint>,
    pub pan_divs: Vec<PanDivCert>,
    pub grades: Vec<FirstClassGradeRecord>,
    pub observations: Vec<CandidateObservation>,
}

/// `judge_third_cert` 的一次真实生产调用。成功与失败都记录，Lean 现场从同一几何输入重判；
/// `output=None` 是合法的空产口，不得被提取层过滤掉。
#[derive(Debug, Clone, PartialEq)]
pub struct ThirdAssemblyCapture {
    pub level: u32,
    pub center: Center,
    pub leave: Segment,
    pub leave_anchor: Option<Direction>,
    pub retest: Segment,
    pub output: Option<BspPoint>,
}

/// P-10 一段扫描产生的 raw emission。`candidate_legs` 是归约前素材；`observations` 只在
/// `final_output` 使用，避免把最终观察冒充 frontier 缓存内容。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ScanEmissionCapture {
    pub points: Vec<BspPoint>,
    pub pan_divs: Vec<PanDivCert>,
    pub grades: Vec<FirstClassGradeRecord>,
    pub candidate_legs: Vec<CandidateObservation>,
    pub observations: Vec<CandidateObservation>,
}

/// P-10 confirmed prefix 的四件 Rc 语义镜面（这里复制其所指列表值，不参与 Rc 身份/判定）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FourCacheCapture {
    pub points: Vec<BspPoint>,
    pub pan_divs: Vec<PanDivCert>,
    pub grades: Vec<FirstClassGradeRecord>,
    pub candidate_legs: Vec<CandidateObservation>,
    pub cached_count: usize,
}

/// 同一 checkpoint 各 level 共享的 `merged_scan_resume` 市场序列输入。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSeriesCapture {
    pub hist_bits: Vec<u64>,
    pub dif_bits: Vec<u64>,
    pub closes_ticks: Vec<i64>,
    pub close_src: Vec<usize>,
    pub gauge: DivergenceGauge,
    pub strokes: Vec<Stroke>,
}

/// P-10 单次真实前沿推进的五阶段输入/输出。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FrontierCapture {
    pub level: u32,
    pub prefix_count: usize,
    pub dirty_e: usize,
    pub freeze_boundary_src: usize,
    pub stable_seg: usize,
    pub segment_count: usize,
    /// `freeze_boundary_src` / `stableSegmentCount` 的原始输入。Runner 必须从这里重算
    /// `e_src` 与 `stable_seg`，不得把 Rust 已裁出的边界当输入。
    pub centers: Vec<Center>,
    pub segments: Vec<Segment>,
    pub anchors: Vec<Option<Direction>>,
    pub anchor_dirs: Option<Vec<Option<Direction>>>,
    pub departure_ends: Option<Vec<usize>>,
    pub blocks: Vec<MoveBlock>,
    /// 进入 `cached_count > stable_seg` 回缩守卫之前的真实四缓存。
    pub cache_before: FourCacheCapture,
    /// 回缩守卫之后、仅由 `[cached_count..stable_seg)` 新算出的追加值。
    pub confirmed_append: ScanEmissionCapture,
    /// confirmed 推进完成后的真实四缓存。
    pub cache_after: FourCacheCapture,
    /// `[stable_seg..segments.len())` 每 bar 重判的 raw tail。
    pub tail: ScanEmissionCapture,
    /// 排序、腿归约与 Pan 投影后的最终四产口。
    pub final_output: ScanEmissionCapture,
}

/// pipeline 单键控制四件 Rc 的真实命中/刷新见证。数组顺序固定为
/// `[bsp, pan_div, grades, candidates]`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FourRcMemoCapture {
    pub level: u32,
    pub key: (usize, usize, usize),
    pub cached_key_before: Option<(usize, usize, usize)>,
    pub hit: bool,
    /// 返回 Rc 是否逐件复用进入分支前的缓存 Rc。
    pub prior_reused: [bool; 4],
    /// 分支结束后缓存 Rc 是否逐件与返回 Rc 同指针。
    pub cache_after_matches_return: [bool; 4],
    pub before_lengths: [usize; 4],
    pub lengths: [usize; 4],
    /// pipeline 最终 BSP（含 07b 二类）与 3a 直接点分口导出。
    pub pipeline_points: Rc<Vec<BspPoint>>,
    /// 仅 memo miss 时由 07a scan 当场保存；hit 时为空，runner 按单键历史取回。
    pub direct_3a_points: Vec<BspPoint>,
    /// 仅 memo miss 时由 `extract_second_resume` 当场保存；不得从最终 BSP 反筛。
    pub second_07b_points: Option<Vec<BspPoint>>,
    pub pan_divs: Rc<Vec<PanDivCert>>,
    pub grades: Rc<Vec<FirstClassGradeRecord>>,
    pub candidates: Rc<Vec<CandidateObservation>>,
}

/// O-01/07b 生产适配器的一条次级别走势 raw 输入。`diverged` 与 `source_index`
/// 分别是生产 `extract_second_signals` 两个上游 oracle 的冻结输入，不从输出反推。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecondLegRawCapture {
    pub direction: Direction,
    pub lo: Tick,
    pub hi: Tick,
    pub diverged: bool,
    pub source_index: usize,
}

/// 显式诊断调用的 raw + 生产输出。该值不挂生产热路径；Runner 必须只读 `center/side/
/// level/legs` 独立重算，再与 `output` 逐叶比较。
#[derive(Debug, Clone, PartialEq)]
pub struct SecondProductionCapture {
    pub center: Center,
    pub side: Side,
    pub level: u32,
    pub legs: Vec<SecondLegRawCapture>,
    pub output: Vec<BspPoint>,
}

/// 默认关闭的 O-01/07b 只读诊断适配器：仅由验收执行器显式调用，内部真实经过现役
/// [`extract_second_signals`]。它不写 capture slot，也不参与任何生产准入或缓存。
pub fn run_second_production_diagnostic(
    center: Center,
    side: Side,
    level: u32,
    legs: Vec<SecondLegRawCapture>,
) -> SecondProductionCapture {
    assert!(
        !capture_enabled(),
        "second production diagnostic 只能在默认关闭的 capture seam 外显式运行"
    );
    let subs = Rc::new(
        legs.iter()
            .map(|leg| RMove::Segment {
                direction: leg.direction,
                lo: leg.lo,
                hi: leg.hi,
            })
            .collect::<Vec<_>>(),
    );
    let parent = RMove::Compose {
        subs: Rc::clone(&subs),
        centers: vec![center],
        level,
    };
    let raw_index = |m: &RMove| {
        subs.iter()
            .position(|candidate| std::ptr::eq(candidate, m))
            .expect("extract_second_signals 返回的走势必须来自 diagnostic raw subs")
    };
    let output = extract_second_signals(
        &parent,
        side,
        &center,
        |m| legs[raw_index(m)].diverged,
        |m| legs[raw_index(m)].source_index,
    );
    SecondProductionCapture {
        center,
        side,
        level,
        legs,
        output,
    }
}

/// 默认关闭的 c_p 完整证书诊断适配器。输入形状移植自生产测试
/// `completed_decomposition_waits_for_successor_then_closed_object_is_reviewed_once`：第一次
/// `advance_cp_lifecycles` 由真实第三类几何把 Pending 关成 Closed，第二次由真实后继走势补齐
/// completed decomposition 与 `full_trend_c_qualified`。返回值是生产 hook 捕获的 raw before /
/// centers / visible moves / after，不手工构造 full-qualified 输出。
#[doc(hidden)]
pub fn run_cp_full_qualified_review_diagnostic() -> CpReviewCapture {
    assert!(
        !capture_enabled(),
        "c_p production diagnostic 只能在默认关闭的 capture seam 外显式运行"
    );
    let id = |level, ordinal| ElementId { level, ordinal };
    let unit = |start_index, end_index, direction, lo, hi| UnitRange {
        start_index,
        end_index,
        direction,
        lo,
        hi,
    };
    let composed = |center: Center, ordinal: u64, start_index, end_index| {
        let sub = LeveledMove::from_unit(
            &unit(
                start_index,
                end_index,
                Direction::Down,
                center.dd,
                center.gg,
            ),
            id(0, ordinal),
        );
        LeveledMove::compose(&[sub], &[center], 1, id(1, ordinal))
    };

    let a = Center {
        zd: 300,
        zg: 400,
        dd: 290,
        gg: 410,
        start_index: 0,
        end_index: 5,
    };
    let b = Center {
        zd: 100,
        zg: 200,
        dd: 90,
        gg: 210,
        start_index: 6,
        end_index: 8,
    };
    let first = Center {
        zd: 100,
        zg: 120,
        dd: 80,
        gg: 150,
        start_index: 9,
        end_index: 11,
    };
    let second = Center {
        zd: 50,
        zg: 60,
        dd: 40,
        gg: 70,
        start_index: 11,
        end_index: 13,
    };
    let successor = Center {
        zd: 55,
        zg: 65,
        dd: 50,
        gg: 90,
        start_index: 13,
        end_index: 15,
    };
    let centers = vec![a, b];
    let movements = vec![
        composed(first, 0, 9, 11),
        composed(second, 1, 11, 13),
        composed(successor, 2, 13, 15),
    ];
    let units = vec![
        unit(9, 11, Direction::Down, first.dd, first.gg),
        unit(11, 13, Direction::Up, second.dd, second.gg),
        unit(13, 15, Direction::Down, successor.dd, successor.gg),
    ];
    let pending = |b_center_index, b_center_id, b_center, departure_move_id, departure_interval| {
        CpScanOwnership {
            b_center_index,
            b_center_id,
            b_center,
            departure_move_id,
            departure_interval,
            lifecycle: CpLifecycleStatus::Pending,
            cp_certificate_confirm_src: None,
            c_structure: None,
            third_class_in_c: None,
            full_trend_evidence: None,
            full_trend_c_qualified: None,
        }
    };
    // `advance_cp_lifecycles` 按 nearest-center index 路由，ownership 列必须与 centers 1:1。
    let mut objects = vec![
        pending(0, id(2, 0), a, None, None),
        pending(1, id(2, 1), b, Some(id(1, 0)), Some((9, 11))),
    ];

    begin_cp_probe();
    advance_cp_lifecycles(
        &mut objects,
        &centers,
        &units[..2],
        &movements[..2],
        None,
        1,
    );
    assert_eq!(objects[1].lifecycle, CpLifecycleStatus::Closed);
    assert!(objects[1].full_trend_evidence.is_some());
    assert!(objects[1].full_trend_c_qualified.is_none());
    advance_cp_lifecycles(&mut objects, &centers, &units, &movements, None, 2);
    assert!(objects[1].full_trend_c_qualified.is_some());
    let mut batch = take_capture();
    assert_eq!(batch.cp_reviews.len(), 1, "fixture 必须恰好产一次 review");
    batch.cp_reviews.remove(0)
}

/// `force_features` 的一段原始闭区间输入。浮点逐个保存 IEEE-754 bits，避免 JSON/Lean
/// 十进制往返改变比较；切片已归一化为从 0 开始的局部坐标。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForceSegmentRawCapture {
    pub hist_bits: Vec<u64>,
    pub dif_bits: Vec<u64>,
    pub closes: Vec<i64>,
    pub direction: Direction,
}

/// `judge_first_from_gates` 的一次真实生产调用。早退 `None` 也必须记录；力度从独立 raw
/// hist/dif/close 与 ForceL 输入由 Lean 现场构造，不允许从 `output.force` 回填期望值。
#[derive(Debug, Clone, PartialEq)]
pub struct FirstAssemblyCapture {
    pub level: u32,
    pub center: Center,
    pub trend_dir: Direction,
    pub segment: Segment,
    pub departure_end: Option<usize>,
    pub side: crate::theta_v0::types::Side,
    pub seg_a: (usize, usize),
    pub lambda_c: usize,
    pub extreme: bool,
    pub comparable: bool,
    pub gauge: DivergenceGauge,
    pub force_l_a: Option<f64>,
    pub force_l_c: Option<f64>,
    pub force_seg_a_raw: Option<ForceSegmentRawCapture>,
    pub force_seg_c_raw: Option<ForceSegmentRawCapture>,
    pub t3_leave: Option<Segment>,
    pub t3_retest: Option<Segment>,
    /// `grade_sink` 在这次生产调用里真实追加的 O-03 记录；`rust_grade` 只是内禀
    /// T3-in-c 判定值，二者不得混用。
    pub grade_output: Option<FirstClassGradeRecord>,
    pub rust_grade: Option<T3InCScan>,
    pub output: Option<BspPoint>,
}

/// O-01/O-03 的最小真实生产适配器。两条见证都调用生产
/// [`judge_first_from_gates`]：一条全窗后扫为 Present（c 内含第三类买卖点对），一条为
/// Missing（c 内无第三类买卖点对）；Rust actual、力度代理与分级 sink 都由该生产调用写出，
/// 适配器只提供确定性的 raw 输入。
#[doc(hidden)]
pub fn run_first_production_diagnostics() -> Vec<FirstAssemblyCapture> {
    assert!(!capture_enabled(), "fixture capture 不得嵌套现有捕获会话");
    let center = Center {
        start_index: 1,
        end_index: 3,
        zd: 100,
        zg: 120,
        dd: 90,
        gg: 130,
    };
    // #1229 裁定 a：全窗后扫口径下，一类点的 breaking 段须落在第三类买卖点对（leave+retest）
    // 之后——c 全窗 [λ_C, seg.end] 内含对 B 的三卖（leave Down 破 zd、retest Up 不重回）才算
    // Present。leave 与 retest 共享端点（retest.start == leave.end），随后 breaking 段创出新低。
    let leave = Segment {
        direction: Direction::Down,
        start_index: 4,
        end_index: 6,
        start_price: 125,
        end_price: 80,
    };
    let retest = Segment {
        direction: Direction::Up,
        start_index: 6,
        end_index: 8,
        start_price: 80,
        end_price: 90,
    };
    let segment = Segment {
        direction: Direction::Down,
        start_index: 8,
        end_index: 10,
        start_price: 90,
        end_price: 70,
    };
    let gates = FirstStructuralGates {
        side: Side::Long,
        seg_a: (1, 3),
        lambda_c: 4,
        a_idx: Some((0, 2)),
        c_idx: Some((3, 4)),
        extreme: true,
    };
    let hist = [-1.0, -2.0, -1.0, -0.5, -0.5];
    let dif = [-2.0, -3.0, -2.0, -1.5, -1.0];
    let closes = [120, 110, 100, 90, 80];
    // L(A)=15（A span [1,3]：末笔速度 20 − 首笔 5）；L(C)=0（C span [4,10]：末笔速度 5 − 首笔
    // 5），0 < 15 ⟹ 真实 ForceL 判据严格成立；各段至少含两笔，避免以单笔零差值偶然命中。
    let strokes = [
        Stroke {
            direction: Direction::Up,
            start_index: 1,
            end_index: 1,
            start_price: 0,
            end_price: 5,
        },
        Stroke {
            direction: Direction::Up,
            start_index: 2,
            end_index: 3,
            start_price: 0,
            end_price: 40,
        },
        Stroke {
            direction: Direction::Up,
            start_index: 4,
            end_index: 4,
            start_price: 0,
            end_price: 5,
        },
        Stroke {
            direction: Direction::Up,
            start_index: 5,
            end_index: 6,
            start_price: 0,
            end_price: 20,
        },
        Stroke {
            direction: Direction::Up,
            start_index: 7,
            end_index: 8,
            start_price: 0,
            end_price: 5,
        },
        Stroke {
            direction: Direction::Up,
            start_index: 9,
            end_index: 10,
            start_price: 0,
            end_price: 10,
        },
    ];

    let run = |sorted: &[Segment]| {
        begin_capture();
        let mut grades = Vec::new();
        let output = judge_first_from_gates(
            gates,
            &center,
            Direction::Down,
            &segment,
            None,
            &hist,
            &dif,
            &closes,
            DivergenceGauge::ForceL,
            &strokes,
            sorted,
            Some(0),
            &mut grades,
        );
        let mut batch = take_capture();
        assert!(output.is_some(), "fixture 必须真实产出 O-01 点");
        assert_eq!(grades.len(), 1, "fixture 必须真实产出 O-03 分级");
        assert_eq!(
            batch.first_assemblies.len(),
            1,
            "fixture 必须恰好捕获一次 O-01"
        );
        let capture = batch.first_assemblies.remove(0);
        assert!(capture
            .output
            .as_ref()
            .is_some_and(|point| point.force.is_some() && point.struct_break_dir.is_some()));
        capture
    };

    let present = run(&[leave, retest, segment]);
    let missing = run(&[leave, segment]);
    assert!(matches!(
        present.rust_grade,
        Some(T3InCScan::Present { .. })
    ));
    assert!(matches!(missing.rust_grade, Some(T3InCScan::Missing)));
    assert!(matches!(
        present.grade_output.map(|record| record.grade),
        Some(T3InCScan::Present { .. })
    ));
    assert!(matches!(
        missing.grade_output.map(|record| record.grade),
        Some(T3InCScan::Missing)
    ));
    vec![present, missing]
}

/// c_p 初始化的真实参数与产物。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpInitCapture {
    pub b_center_id: ElementId,
    pub center: Center,
    pub departure_move_id: Option<ElementId>,
    pub departure_interval: Option<(usize, usize)>,
    pub after: CpScanOwnership,
}

/// dirty invalidate 的整列 before/参数/after 与扫描起点回执。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpDirtyCapture {
    pub dirty_from: usize,
    pub before: Vec<CpScanOwnership>,
    pub after: Vec<CpScanOwnership>,
    pub scan_from: usize,
    pub pending_fallbacks: u64,
    pub certificate_clear_recomputes: u64,
}

/// frontier 同身份重建的真实 before/参数/after。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpReinheritCapture {
    pub dirty_from: usize,
    pub rebuilt_before: CpScanOwnership,
    pub prior: Option<CpScanOwnership>,
    pub prior_is_stable: bool,
    pub after: CpScanOwnership,
}

/// Pending→Closed 时在写回前形成的独立 closure wire。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpClosureCapture {
    pub confirm_src: usize,
    pub structure: CpStructureIdentity,
    pub third: ThirdClassInCp,
    pub evidence: Option<FullTrendQualificationEvidence>,
    pub qualification: Option<FullTrendCQualified>,
}

/// lifecycle advance 的真实 before + 第三类 raw 几何 + closure + after。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpAdvanceCapture {
    pub before: CpScanOwnership,
    pub leave: Segment,
    pub leave_anchor: Option<Direction>,
    pub retest: Segment,
    pub leave_move_id: ElementId,
    pub retest_move_id: ElementId,
    pub centers: Vec<Center>,
    pub visible_moves: Vec<LeveledMove>,
    pub after: CpScanOwnership,
}

/// Closed 对象遇到 terminal 后继时的 raw 复核窗口。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpReviewCapture {
    pub before: CpScanOwnership,
    pub centers: Vec<Center>,
    pub visible_moves: Vec<LeveledMove>,
    pub after: CpScanOwnership,
}

/// c_p 生命周期生产 writer 的真实发生顺序。分栏 Vec 仅保留给局部诊断；跨 writer 的
/// 语义对拍必须消费本序列，不能把 init/dirty/reinherit/advance/review 分组后重排。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CpLifecycleEvent {
    Init(CpInitCapture),
    Dirty(CpDirtyCapture),
    Reinherit(CpReinheritCapture),
    Advance(CpAdvanceCapture),
    Review(CpReviewCapture),
}

/// `make_trend_observation` 的 raw 输入与真实 Rust 输出。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrendAssemblyCapture {
    pub level: u32,
    pub previous: Center,
    pub parent: Center,
    pub segment: Segment,
    pub side: crate::theta_v0::types::Side,
    pub seg_a: (usize, usize),
    pub lambda_c: usize,
    pub a_idx_present: bool,
    pub c_idx_present: bool,
    pub extreme: bool,
    pub output: CandidateObservation,
}

/// 一次真实 `reduce_structural_legs` 调用的输入腿与归约结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReductionCapture {
    pub level: u32,
    pub legs: Vec<CandidateObservation>,
    pub output: Vec<CandidateObservation>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CaptureBatch {
    pub raw_series: Option<RawSeriesCapture>,
    pub scans: Vec<ScanCapture>,
    pub first_assemblies: Vec<FirstAssemblyCapture>,
    pub third_assemblies: Vec<ThirdAssemblyCapture>,
    pub trend_assemblies: Vec<TrendAssemblyCapture>,
    pub reductions: Vec<ReductionCapture>,
    pub frontiers: Vec<FrontierCapture>,
    pub four_rc_memos: Vec<FourRcMemoCapture>,
    pub cp_inits: Vec<CpInitCapture>,
    pub cp_dirty: Vec<CpDirtyCapture>,
    pub cp_reinherits: Vec<CpReinheritCapture>,
    pub cp_advances: Vec<CpAdvanceCapture>,
    pub cp_reviews: Vec<CpReviewCapture>,
    pub cp_lifecycle_events: Vec<CpLifecycleEvent>,
}

thread_local! {
    static CAPTURE: RefCell<Option<CaptureBatch>> = const { RefCell::new(None) };
    static CAPTURE_MODE: Cell<CaptureMode> = const { Cell::new(CaptureMode::Off) };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CaptureMode {
    Off,
    DataFull,
    MissProbe,
    LifecycleOnly,
}

pub fn begin_capture() {
    CAPTURE_MODE.with(|mode| mode.set(CaptureMode::DataFull));
    CAPTURE.with(|slot| *slot.borrow_mut() = Some(CaptureBatch::default()));
}

/// 低成本 P-10 miss 探针：memo hit 路径不得复制任何 Vec；只有真实 miss 进入 merged scan 后
/// 才会产出内容。cp 生命周期 hook 只响应 full checkpoint，不响应本探针。
pub fn begin_miss_probe() {
    CAPTURE_MODE.with(|mode| mode.set(CaptureMode::MissProbe));
    CAPTURE.with(|slot| *slot.borrow_mut() = Some(CaptureBatch::default()));
}

/// 每 bar 捕获稀疏 c_p lifecycle 事件，并仅在 pipeline memo miss 时保存 07a/07b
/// 组成见证；禁止触发 scan/frontier Vec 复制，memo hit 也不复制 Vec。
pub fn begin_cp_probe() {
    CAPTURE_MODE.with(|mode| mode.set(CaptureMode::LifecycleOnly));
    CAPTURE.with(|slot| *slot.borrow_mut() = Some(CaptureBatch::default()));
}

pub fn take_capture() -> CaptureBatch {
    CAPTURE_MODE.with(|mode| mode.set(CaptureMode::Off));
    CAPTURE.with(|slot| slot.borrow_mut().take().unwrap_or_default())
}

/// Extractor-owned deterministic fixture: execute the real production resume entry from an empty
/// four-cache and return its raw boundary plus all five captured stages.  This is deliberately a
/// public diagnostic adapter because the standalone extractor binary cannot call crate-private
/// `scan::merged_scan_resume` directly.  Capture mode is restored to Off before returning.
#[doc(hidden)]
#[allow(clippy::too_many_arguments)]
pub fn capture_fixture_frontier(
    cache_before: FourCacheCapture,
    level: u32,
    centers: &[Center],
    segments: &[Segment],
    anchor_dirs: Option<&[Option<Direction>]>,
    departure_ends: Option<&[usize]>,
    blocks: &[MoveBlock],
    prefix_count: usize,
    dirty_e: usize,
    hist: &[f64],
    dif: &[f64],
    closes_tick: &[Tick],
    close_src: &[usize],
    gauge: DivergenceGauge,
    strokes: &[Stroke],
) -> (RawSeriesCapture, FrontierCapture) {
    assert!(!capture_enabled(), "fixture capture 不得嵌套现有捕获会话");
    begin_capture();
    let mut points = cache_before.points;
    let mut pan_divs = cache_before.pan_divs;
    let mut grades = cache_before.grades;
    let mut candidate_legs = cache_before.candidate_legs;
    let mut cached_count = cache_before.cached_count;
    let _ = super::super::scan::merged_scan_resume(
        &mut points,
        &mut pan_divs,
        &mut grades,
        &mut candidate_legs,
        &mut cached_count,
        level,
        centers,
        segments,
        anchor_dirs,
        departure_ends,
        blocks,
        prefix_count,
        dirty_e,
        hist,
        dif,
        closes_tick,
        close_src,
        gauge,
        strokes,
    );
    let mut batch = take_capture();
    assert_eq!(
        batch.frontiers.len(),
        1,
        "fixture 必须恰好捕获一个 frontier"
    );
    let series = batch.raw_series.take().expect("fixture 缺共享 raw series");
    (series, batch.frontiers.remove(0))
}

pub(crate) fn capture_enabled() -> bool {
    CAPTURE_MODE.with(|mode| mode.get() != CaptureMode::Off)
        && CAPTURE.with(|slot| slot.borrow().is_some())
}

pub(crate) fn full_capture_enabled() -> bool {
    capture_enabled() && CAPTURE_MODE.with(|mode| mode.get() == CaptureMode::DataFull)
}

pub(crate) fn data_capture_enabled() -> bool {
    capture_enabled()
        && CAPTURE_MODE
            .with(|mode| matches!(mode.get(), CaptureMode::DataFull | CaptureMode::MissProbe))
}

pub(crate) fn cp_only_enabled() -> bool {
    capture_enabled() && CAPTURE_MODE.with(|mode| mode.get() == CaptureMode::LifecycleOnly)
}

pub(crate) fn lifecycle_capture_enabled() -> bool {
    capture_enabled()
        && CAPTURE_MODE.with(|mode| {
            matches!(
                mode.get(),
                CaptureMode::DataFull | CaptureMode::LifecycleOnly
            )
        })
}

pub(crate) fn memo_capture_enabled() -> bool {
    capture_enabled()
}

/// CP probe 遇到真实 pipeline memo miss 时，临时打开同一次 07a raw/P10 frontier 捕获；
/// 返回值只供配对恢复，full/miss probe 调用不改变模式。
pub(crate) fn enter_memo_miss_data_capture() -> bool {
    let restore = cp_only_enabled();
    if restore {
        CAPTURE_MODE.with(|mode| mode.set(CaptureMode::DataFull));
    }
    restore
}

pub(crate) fn exit_memo_miss_data_capture(restore_cp_only: bool) {
    if restore_cp_only && capture_enabled() {
        CAPTURE_MODE.with(|mode| mode.set(CaptureMode::LifecycleOnly));
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn record_scan(
    level: u32,
    centers: &[Center],
    segments: &[Segment],
    anchors: &[Option<Direction>],
    departure_ends: Option<&[usize]>,
    blocks: &[MoveBlock],
    points: &[BspPoint],
    pan_divs: &[PanDivCert],
    grades: &[FirstClassGradeRecord],
    observations: &[CandidateObservation],
) {
    if !data_capture_enabled() {
        return;
    }
    CAPTURE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(batch) = slot.as_mut() else {
            return;
        };
        batch.scans.push(ScanCapture {
            level,
            centers: centers.to_vec(),
            segments: segments.to_vec(),
            anchors: anchors.to_vec(),
            departure_ends: departure_ends.map(<[usize]>::to_vec),
            blocks: blocks.to_vec(),
            points: points.to_vec(),
            pan_divs: pan_divs.to_vec(),
            grades: grades.to_vec(),
            observations: observations.to_vec(),
        });
    });
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn record_first_assembly(
    level: u32,
    center: &Center,
    trend_dir: Direction,
    segment: &Segment,
    departure_end: Option<usize>,
    gates: FirstStructuralGates,
    gauge: DivergenceGauge,
    hist: &[f64],
    dif: &[f64],
    closes: &[i64],
    strokes: &[Stroke],
    sorted: &[Segment],
    grade_output: Option<FirstClassGradeRecord>,
    rust_grade: Option<T3InCScan>,
    output: Option<&BspPoint>,
) {
    if !data_capture_enabled() {
        return;
    }
    let anchor = sorted.partition_point(|s| s.start_index < center.end_index);
    let raw = |range: Option<(usize, usize)>| {
        let (start, end) = range?;
        if start > end || end >= hist.len() || end >= dif.len() || end >= closes.len() {
            return None;
        }
        Some(ForceSegmentRawCapture {
            hist_bits: hist[start..=end]
                .iter()
                .map(|value| value.to_bits())
                .collect(),
            dif_bits: dif[start..=end]
                .iter()
                .map(|value| value.to_bits())
                .collect(),
            closes: closes[start..=end].to_vec(),
            direction: trend_dir,
        })
    };
    let capture = FirstAssemblyCapture {
        level,
        center: *center,
        trend_dir,
        segment: *segment,
        departure_end,
        side: gates.side,
        seg_a: gates.seg_a,
        lambda_c: gates.lambda_c,
        extreme: gates.extreme,
        comparable: gates.comparable(),
        gauge,
        force_l_a: crate::theta_v0::parser::segment::segment_force_l(
            strokes,
            gates.seg_a.0,
            gates.seg_a.1,
        ),
        force_l_c: crate::theta_v0::parser::segment::segment_force_l(
            strokes,
            gates.lambda_c,
            segment.end_index,
        ),
        force_seg_a_raw: raw(gates.a_idx),
        force_seg_c_raw: raw(gates.c_idx),
        t3_leave: sorted.get(anchor).copied(),
        t3_retest: sorted.get(anchor + 1).copied(),
        grade_output,
        rust_grade,
        output: output.copied(),
    };
    CAPTURE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(batch) = slot.as_mut() else {
            return;
        };
        batch.first_assemblies.push(capture.clone());
    });
}

pub(crate) fn record_cp_init(
    b_center_id: ElementId,
    center: Center,
    departure_move_id: Option<ElementId>,
    departure_interval: Option<(usize, usize)>,
    after: &CpScanOwnership,
) {
    if !lifecycle_capture_enabled() {
        return;
    }
    CAPTURE.with(|slot| {
        if let Some(batch) = slot.borrow_mut().as_mut() {
            let capture = CpInitCapture {
                b_center_id,
                center,
                departure_move_id,
                departure_interval,
                after: after.clone(),
            };
            batch
                .cp_lifecycle_events
                .push(CpLifecycleEvent::Init(capture.clone()));
            batch.cp_inits.push(capture);
        }
    });
}

pub(crate) fn record_cp_dirty(value: CpDirtyCapture) {
    if !lifecycle_capture_enabled() {
        return;
    }
    CAPTURE.with(|slot| {
        if let Some(batch) = slot.borrow_mut().as_mut() {
            batch
                .cp_lifecycle_events
                .push(CpLifecycleEvent::Dirty(value.clone()));
            batch.cp_dirty.push(value);
        }
    });
}

pub(crate) fn record_cp_reinherit(value: CpReinheritCapture) {
    if !lifecycle_capture_enabled() {
        return;
    }
    // 连续 lifecycle probe 的新对象必须由生产构造器的 cp_init 留证；`prior=None`
    // 不是 reinherit，不能冒充初始化 writer。prior=Some 即使 after 值未变也保留：
    // 该 writer 的稳定性判定与 rebuilt/prior 原始字段仍须由 Runner 对拍。
    if cp_only_enabled() && value.prior.is_none() {
        return;
    }
    CAPTURE.with(|slot| {
        if let Some(batch) = slot.borrow_mut().as_mut() {
            batch
                .cp_lifecycle_events
                .push(CpLifecycleEvent::Reinherit(value.clone()));
            batch.cp_reinherits.push(value);
        }
    });
}

pub(crate) fn record_cp_advance(value: CpAdvanceCapture) {
    if !lifecycle_capture_enabled() {
        return;
    }
    CAPTURE.with(|slot| {
        if let Some(batch) = slot.borrow_mut().as_mut() {
            batch
                .cp_lifecycle_events
                .push(CpLifecycleEvent::Advance(value.clone()));
            batch.cp_advances.push(value);
        }
    });
}

pub(crate) fn record_cp_review(value: CpReviewCapture) {
    if !lifecycle_capture_enabled() {
        return;
    }
    CAPTURE.with(|slot| {
        if let Some(batch) = slot.borrow_mut().as_mut() {
            batch
                .cp_lifecycle_events
                .push(CpLifecycleEvent::Review(value.clone()));
            batch.cp_reviews.push(value);
        }
    });
}

pub(crate) fn record_third_assembly(
    level: u32,
    center: &Center,
    leave: &Segment,
    leave_anchor: Option<Direction>,
    retest: &Segment,
    output: Option<&BspPoint>,
) {
    if !data_capture_enabled() {
        return;
    }
    let capture = ThirdAssemblyCapture {
        level,
        center: *center,
        leave: *leave,
        leave_anchor,
        retest: *retest,
        output: output.copied(),
    };
    CAPTURE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(batch) = slot.as_mut() else {
            return;
        };
        batch.third_assemblies.push(capture.clone());
    });
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn record_trend_assembly(
    level: u32,
    previous: &Center,
    parent: &Center,
    segment: &Segment,
    gates: &FirstStructuralGates,
    output: &CandidateObservation,
) {
    if !data_capture_enabled() {
        return;
    }
    let capture = TrendAssemblyCapture {
        level,
        previous: *previous,
        parent: *parent,
        segment: *segment,
        side: gates.side,
        seg_a: gates.seg_a,
        lambda_c: gates.lambda_c,
        a_idx_present: gates.a_idx.is_some(),
        c_idx_present: gates.c_idx.is_some(),
        extreme: gates.extreme,
        output: output.clone(),
    };
    CAPTURE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(batch) = slot.as_mut() else {
            return;
        };
        batch.trend_assemblies.push(capture.clone());
    });
}

pub(crate) fn record_reduction(
    level: u32,
    legs: Vec<CandidateObservation>,
    output: &[CandidateObservation],
) {
    if !data_capture_enabled() {
        return;
    }
    CAPTURE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(batch) = slot.as_mut() else {
            return;
        };
        batch.reductions.push(ReductionCapture {
            level,
            legs,
            output: output.to_vec(),
        });
    });
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn record_frontier(
    frontier: FrontierCapture,
    hist: &[f64],
    dif: &[f64],
    closes_ticks: &[i64],
    close_src: &[usize],
    gauge: DivergenceGauge,
    strokes: &[Stroke],
) {
    if !data_capture_enabled() {
        return;
    }
    CAPTURE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(batch) = slot.as_mut() else {
            return;
        };
        let series = RawSeriesCapture {
            hist_bits: hist.iter().map(|value| value.to_bits()).collect(),
            dif_bits: dif.iter().map(|value| value.to_bits()).collect(),
            closes_ticks: closes_ticks.to_vec(),
            close_src: close_src.to_vec(),
            gauge,
            strokes: strokes.to_vec(),
        };
        if let Some(existing) = &batch.raw_series {
            assert_eq!(
                existing, &series,
                "同 checkpoint 各 level raw series 必须逐位相同"
            );
        } else {
            batch.raw_series = Some(series);
        }
        batch.frontiers.push(frontier);
    });
}

pub(crate) fn record_four_rc_memo(memo: FourRcMemoCapture) {
    if !memo_capture_enabled() {
        return;
    }
    CAPTURE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(batch) = slot.as_mut() else {
            return;
        };
        batch.four_rc_memos.push(memo);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pending_cp(ordinal: u64) -> CpScanOwnership {
        let id = ElementId { level: 1, ordinal };
        CpScanOwnership {
            b_center_index: ordinal as usize,
            b_center_id: id,
            b_center: Center {
                zd: 10,
                zg: 20,
                dd: 5,
                gg: 25,
                start_index: 1,
                end_index: 3,
            },
            departure_move_id: None,
            departure_interval: None,
            lifecycle: CpLifecycleStatus::Pending,
            cp_certificate_confirm_src: None,
            c_structure: None,
            third_class_in_c: None,
            full_trend_evidence: None,
            full_trend_c_qualified: None,
        }
    }

    #[test]
    fn capture_session_is_opt_in_and_drains() {
        assert_eq!(
            take_capture(),
            CaptureBatch::default(),
            "默认关闭不得残留捕获"
        );
        begin_capture();
        assert_eq!(
            take_capture(),
            CaptureBatch::default(),
            "无 scan 调用时为空"
        );
        assert_eq!(
            take_capture(),
            CaptureBatch::default(),
            "take 后须恢复默认关闭"
        );
    }

    #[test]
    fn frontier_capture_is_opt_in_and_preserves_all_five_stages() {
        let frontier = FrontierCapture {
            level: 2,
            prefix_count: 4,
            dirty_e: 99,
            freeze_boundary_src: 80,
            stable_seg: 7,
            segment_count: 9,
            centers: Vec::new(),
            segments: Vec::new(),
            anchors: Vec::new(),
            anchor_dirs: None,
            departure_ends: None,
            blocks: Vec::new(),
            cache_before: FourCacheCapture {
                cached_count: 5,
                ..FourCacheCapture::default()
            },
            confirmed_append: ScanEmissionCapture::default(),
            cache_after: FourCacheCapture {
                cached_count: 7,
                ..FourCacheCapture::default()
            },
            tail: ScanEmissionCapture::default(),
            final_output: ScanEmissionCapture::default(),
        };

        record_frontier(
            frontier.clone(),
            &[],
            &[],
            &[],
            &[],
            DivergenceGauge::default(),
            &[],
        );
        assert!(
            take_capture().frontiers.is_empty(),
            "默认关闭时不得复制前沿"
        );

        begin_capture();
        record_frontier(
            frontier.clone(),
            &[],
            &[],
            &[],
            &[],
            DivergenceGauge::default(),
            &[],
        );
        let batch = take_capture();
        assert_eq!(batch.frontiers, vec![frontier]);
        assert_eq!(batch.frontiers[0].cache_before.cached_count, 5);
        assert_eq!(batch.frontiers[0].cache_after.cached_count, 7);
    }

    #[test]
    fn four_rc_memo_capture_is_opt_in_and_keeps_one_key_verdict() {
        let memo = FourRcMemoCapture {
            level: 1,
            key: (3, 5, 8),
            cached_key_before: Some((3, 5, 8)),
            hit: true,
            prior_reused: [true; 4],
            cache_after_matches_return: [true; 4],
            before_lengths: [2, 3, 4, 5],
            lengths: [2, 3, 4, 5],
            pipeline_points: Rc::new(Vec::new()),
            direct_3a_points: Vec::new(),
            second_07b_points: Some(Vec::new()),
            pan_divs: Rc::new(Vec::new()),
            grades: Rc::new(Vec::new()),
            candidates: Rc::new(Vec::new()),
        };
        record_four_rc_memo(memo.clone());
        assert!(take_capture().four_rc_memos.is_empty());
        begin_capture();
        record_four_rc_memo(memo.clone());
        let batch = take_capture();
        assert_eq!(batch.four_rc_memos, vec![memo.clone()]);
        assert!(
            Rc::ptr_eq(
                &batch.four_rc_memos[0].pipeline_points,
                &memo.pipeline_points
            ),
            "memo hit 捕获只克隆 Rc，不克隆 Vec"
        );
        assert!(Rc::ptr_eq(&batch.four_rc_memos[0].pan_divs, &memo.pan_divs));
        assert!(Rc::ptr_eq(&batch.four_rc_memos[0].grades, &memo.grades));
        assert!(Rc::ptr_eq(
            &batch.four_rc_memos[0].candidates,
            &memo.candidates
        ));
    }

    #[test]
    fn second_diagnostic_calls_production_extractor_and_keeps_raw() {
        take_capture();
        let center = Center {
            start_index: 0,
            end_index: 0,
            zd: 0,
            zg: 4,
            dd: -2,
            gg: 6,
        };
        let legs = vec![
            SecondLegRawCapture {
                direction: Direction::Down,
                lo: -10,
                hi: -2,
                diverged: true,
                source_index: 30,
            },
            SecondLegRawCapture {
                direction: Direction::Up,
                lo: -8,
                hi: 3,
                diverged: false,
                source_index: 42,
            },
            SecondLegRawCapture {
                direction: Direction::Up,
                lo: 1,
                hi: 5,
                diverged: false,
                source_index: 50,
            },
        ];
        let capture = run_second_production_diagnostic(center, Side::Long, 1, legs.clone());
        assert_eq!(capture.legs, legs, "raw oracle 输入必须原样附着");
        assert_eq!(capture.output.len(), 1);
        assert!(capture.output[0].bits.buy2);
        assert_eq!(capture.output[0].source_index, 42);
        assert_eq!(capture.output[0].pivot_low, -8);
        assert_eq!(
            capture.output[0].center,
            Some(crate::theta_v0::classifier::bsp::OwnerRef::Type1Anchor(30))
        );
        assert_eq!(capture.output[0].retrace_breaks_type1, Some(false));
        assert!(!capture_enabled(), "adapter 不得打开 capture seam");
    }

    #[test]
    fn cp_review_diagnostic_reaches_real_full_qualified_writer() {
        let review = run_cp_full_qualified_review_diagnostic();
        assert_eq!(review.before.lifecycle, CpLifecycleStatus::Closed);
        assert!(review.before.full_trend_evidence.is_some());
        assert!(review.before.full_trend_c_qualified.is_none());
        assert!(review.after.full_trend_evidence.is_some());
        assert!(review.after.full_trend_c_qualified.is_some());
        assert_eq!(
            review
                .after
                .full_trend_c_qualified
                .as_ref()
                .map(|qualified| qualified.confirm_src),
            Some(15)
        );
    }

    #[test]
    fn first_diagnostic_uses_real_point_force_and_grade_writers() {
        let captures = run_first_production_diagnostics();
        assert_eq!(captures.len(), 2);
        assert!(captures.iter().all(|capture| {
            capture.output.as_ref().is_some_and(|point| {
                point.force.is_some() && point.struct_break_dir == Some(Side::Long)
            })
        }));
        assert!(matches!(
            captures[0].rust_grade,
            Some(T3InCScan::Present { .. })
        ));
        assert!(matches!(captures[1].rust_grade, Some(T3InCScan::Missing)));
        assert!(captures
            .iter()
            .all(|capture| capture.grade_output.is_some()));
        assert!(!capture_enabled(), "adapter 不得打开 capture seam");
    }

    #[test]
    fn miss_probe_records_frontier_but_not_full_cp_lifecycle() {
        begin_miss_probe();
        assert!(capture_enabled());
        assert!(!full_capture_enabled());
        record_frontier(
            FrontierCapture::default(),
            &[],
            &[],
            &[],
            &[],
            DivergenceGauge::default(),
            &[],
        );
        record_cp_dirty(CpDirtyCapture {
            dirty_from: 3,
            before: Vec::new(),
            after: Vec::new(),
            scan_from: 3,
            pending_fallbacks: 0,
            certificate_clear_recomputes: 0,
        });
        let batch = take_capture();
        assert_eq!(batch.frontiers.len(), 1);
        assert!(
            batch.cp_dirty.is_empty(),
            "miss probe 不得复制 c_p 生命周期列"
        );
        assert!(!capture_enabled());
    }

    #[test]
    fn capture_mode_gate_table_is_mutually_exclusive() {
        let assert_gates = |capture, data, lifecycle, cp_only, full| {
            assert_eq!(capture_enabled(), capture);
            assert_eq!(data_capture_enabled(), data);
            assert_eq!(lifecycle_capture_enabled(), lifecycle);
            assert_eq!(cp_only_enabled(), cp_only);
            assert_eq!(full_capture_enabled(), full);
        };

        take_capture();
        assert_gates(false, false, false, false, false);
        begin_capture();
        assert_gates(true, true, true, false, true);
        take_capture();
        begin_miss_probe();
        assert_gates(true, true, false, false, false);
        take_capture();
        begin_cp_probe();
        assert_gates(true, false, true, true, false);
        let restore = enter_memo_miss_data_capture();
        assert!(restore);
        assert_gates(true, true, true, false, true);
        exit_memo_miss_data_capture(restore);
        assert_gates(true, false, true, true, false);
        take_capture();
        assert_gates(false, false, false, false, false);
    }

    #[test]
    fn lifecycle_probe_keeps_real_init_and_never_uses_reinherit_none_as_new_writer() {
        let cp = pending_cp(7);
        begin_cp_probe();
        record_cp_init(cp.b_center_id, cp.b_center, None, None, &cp);
        record_cp_dirty(CpDirtyCapture {
            dirty_from: 3,
            before: vec![cp.clone()],
            after: vec![cp.clone()],
            scan_from: 3,
            pending_fallbacks: 0,
            certificate_clear_recomputes: 0,
        });
        record_cp_reinherit(CpReinheritCapture {
            dirty_from: 3,
            rebuilt_before: cp.clone(),
            prior: None,
            prior_is_stable: false,
            after: cp,
        });

        let batch = take_capture();
        assert_eq!(
            batch.cp_inits.len(),
            1,
            "LifecycleOnly 必须保留生产构造器 init"
        );
        assert_eq!(
            batch.cp_dirty.len(),
            1,
            "dirty writer 不得被 lifecycle probe 吞掉"
        );
        assert!(
            batch.cp_reinherits.is_empty(),
            "reinherit(None) 不是生命周期 writer，不得冒充 cp_init"
        );
        assert!(matches!(
            batch.cp_lifecycle_events.as_slice(),
            [CpLifecycleEvent::Init(_), CpLifecycleEvent::Dirty(_)]
        ));
    }
}
