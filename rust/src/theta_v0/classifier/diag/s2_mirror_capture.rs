//! #1080 S2 Lean 镜像验收的只读捕获 seam。
//!
//! 默认恒关闭。验收 bin 只在 checkpoint 那一根 bar 前调用 [`begin_capture`]，生产
//! `merged_scan_resume` 返回时经 [`record_scan`] 复制同一次调用的 raw 输入与 3a 四产口，
//! 随后 [`take_capture`] 取走。此模块不重判、不筛选、不改写任何生产对象。

use std::cell::RefCell;

use crate::theta_v0::classifier::cand_event::CandidateObservation;
use crate::theta_v0::classifier::decompose::MoveBlock;
use crate::theta_v0::classifier::divergence::DivergenceGauge;
use crate::theta_v0::classifier::signal::{
    FirstClassGradeRecord, FirstStructuralGates, PanDivCert, T3InCGrade,
};
use crate::theta_v0::classifier::BspPoint;
use crate::theta_v0::types::{Center, Direction, Segment, Stroke};

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

/// `judge_first_from_gates` 的一次真实生产调用（仅记录已穿过 extreme/坐标门并产点者）。
/// 力度保存 A/C 的原始 `segment_force_l` 数值，由 Lean 现场比较；不注入 diverged Bool。
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
    pub t3_leave: Option<Segment>,
    pub t3_retest: Option<Segment>,
    pub rust_grade: T3InCGrade,
    pub output: BspPoint,
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
    pub scans: Vec<ScanCapture>,
    pub first_assemblies: Vec<FirstAssemblyCapture>,
    pub trend_assemblies: Vec<TrendAssemblyCapture>,
    pub reductions: Vec<ReductionCapture>,
}

thread_local! {
    static CAPTURE: RefCell<Option<CaptureBatch>> = const { RefCell::new(None) };
}

pub fn begin_capture() {
    CAPTURE.with(|slot| *slot.borrow_mut() = Some(CaptureBatch::default()));
}

pub fn take_capture() -> CaptureBatch {
    CAPTURE.with(|slot| slot.borrow_mut().take().unwrap_or_default())
}

pub(crate) fn capture_enabled() -> bool {
    CAPTURE.with(|slot| slot.borrow().is_some())
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
    strokes: &[Stroke],
    sorted: &[Segment],
    rust_grade: T3InCGrade,
    output: &BspPoint,
) {
    CAPTURE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(batch) = slot.as_mut() else {
            return;
        };
        let anchor = sorted.partition_point(|s| s.start_index < center.end_index);
        batch.first_assemblies.push(FirstAssemblyCapture {
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
            t3_leave: sorted.get(anchor).copied(),
            t3_retest: sorted.get(anchor + 1).copied(),
            rust_grade,
            output: *output,
        });
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
    CAPTURE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let Some(batch) = slot.as_mut() else {
            return;
        };
        batch.trend_assemblies.push(TrendAssemblyCapture {
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
        });
    });
}

pub(crate) fn record_reduction(
    level: u32,
    legs: Vec<CandidateObservation>,
    output: &[CandidateObservation],
) {
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
