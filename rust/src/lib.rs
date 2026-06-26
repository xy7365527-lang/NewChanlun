//! newchan_rust — 缠论核心引擎的 Rust 重写，通过 PyO3 暴露 Python 接口。
//!
//! Phase 2 范围：bi 引擎（笔）。strokes 列表逐位等价于 Python `BiEngine`。
//! Phase 3 范围：segment 引擎（线段，特征序列法）+ zhongshu 引擎（中枢，三段重叠法）+
//! move 引擎（走势类型）。批量等价于 Python `segments_from_strokes_v1` /
//! `zhongshu_from_segments` / `zhongshu_from_strokes` / `moves_from_zhongshus`。
//! Phase 4 范围：背驰（divergence）+ 买卖点（buysellpoint，type1/2/3 + 2B3B 重合）。
//! 批量等价于 Python `divergences_from_moves_v1` / `buysellpoints_from_level`。
//! Phase 7 范围：MACD 力度层（macd）——EMA → DIF/DEA/hist → area/peak。逐位等价于
//! Python `a_macd.compute_macd` / `OnlineMacdState` / `macd_area_for_range` 及
//! `a_divergence_v1` 的 `dif_peak_for_range` / `histogram_peak_for_range` /
//! `_b_segment_crosses_zero`。背驰 MACD 三维度路径（T2/T6/T7 + T4）随之接通。

mod bi_engine;
mod bi_zhongshu_bsp;
mod buysellpoint;
#[cfg(test)]
mod c_segment_verify;
mod divergence;
mod fractal;
mod level;
mod macd;
mod moves;
mod orchestrator;
mod ph;
mod segment;
mod segment_layers;
/// 螺旋引擎 v2（D∞ 群结构第一性原理）。`pub` 导出使其成为 crate 公开 API 表面，
/// Step 0 尚未接入 PyO3/orchestrator 时避免 dead_code 误报（架构 §3）。
pub mod spiral;
/// 赋格引擎 v3（递归嵌套多重赋格 = 操作必然结构）。层结构四步循环操作引擎，复用
/// spiral 信号层，核心仓 H⁰(2/3) ⊕ 机动仓 H¹(1/3 四步 1-cycle 穿 ε=−1)。`pub` 导出。
pub mod fugue_v3;
/// 统一递归算子 T（缠师第65课 `aₙ=f(aₙ₋₁)` 形式化）。`pub` 导出避免 dead_code 误报——
/// 设计阶段脚手架，尚未接入 PyO3（文档 docs/unified_recursive_operator_T.md）。
pub mod recursive_t;
/// Reference Θ v0 — bit-exact 缠论可执行系统引擎（Phase 2，task #38）。对齐
/// `formal/Strict/*.lean` 的 L0 spec（非 Python-等价旧 ladder）。parser→classifier→
/// strategy 三子模块，所有 Θ 参数显式 config（Phase 6 扫描入口）。`pub` 导出避免
/// 骨架阶段（未接入 PyO3）的 dead_code 误报。文档 docs/reference-theta-v0.md。
pub mod theta_v0;
mod stroke;
mod trading;
mod zhongshu;

use std::collections::HashSet;

use pyo3::prelude::*;
use stroke::{Direction, Stroke};

use buysellpoint::{BspKind, Side};

/// 线段批量入口的输出元组（嵌套以规避 PyO3 元组 ≤12 元素限制）：
///   ((s0, s1, i0, i1, dir, high, low, confirmed, kind),
///    (ep0_i, ep0_price, ep0_type, ep1_i, ep1_price, ep1_type, p0, p1),
///    Option<(trigger_stroke_k, (a, b, c), gap_type)>)
type SegmentTuple = (
    (usize, usize, usize, usize, &'static str, f64, f64, bool, &'static str),
    (usize, f64, &'static str, usize, f64, &'static str, f64, f64),
    Option<(usize, (usize, usize, usize), &'static str)>,
);

/// 解析 Python 笔方向字符串。非法值 panic（输入契约由调用方保证）。
fn parse_direction(s: &str) -> Direction {
    match s {
        "up" => Direction::Up,
        "down" => Direction::Down,
        other => panic!("invalid stroke direction: {other:?}"),
    }
}

/// 笔事件引擎的 Python 绑定。
///
/// 用法（Python）：
///
/// ```text
/// from newchan_rust import BiEngine
/// eng = BiEngine(stroke_mode="new")
/// for o, h, l, c in bars:
///     eng.process_bar(o, h, l, c)
/// strokes = eng.current_strokes()  # list[(i0,i1,dir,high,low,p0,p1,confirmed)]
/// ```
#[pyclass(name = "BiEngine")]
struct PyBiEngine {
    inner: bi_engine::BiEngine,
}

#[pymethods]
impl PyBiEngine {
    #[new]
    #[pyo3(signature = (
        stroke_mode = "new",
        min_strict_sep = 5,
        reset_dir_on_fractal = false,
        new_raw_gap_min = 3,
    ))]
    fn new(
        stroke_mode: &str,
        min_strict_sep: i64,
        reset_dir_on_fractal: bool,
        new_raw_gap_min: i64,
    ) -> Self {
        PyBiEngine {
            inner: bi_engine::BiEngine::new(
                stroke_mode,
                min_strict_sep,
                reset_dir_on_fractal,
                new_raw_gap_min,
            ),
        }
    }

    /// 处理一根新 K 线（仅传 OHLC；strokes 不依赖时间戳）。
    fn process_bar(&mut self, open: f64, high: f64, low: f64, close: f64) {
        self.inner.process_bar(open, high, low, close);
    }

    /// 当前笔快照。返回 list of tuples，字段顺序与 Python `Stroke` 一致：
    /// (i0, i1, direction, high, low, p0, p1, confirmed)。
    fn current_strokes(&self) -> Vec<(usize, usize, &'static str, f64, f64, f64, f64, bool)> {
        self.inner
            .current_strokes()
            .iter()
            .map(|s| {
                (
                    s.i0,
                    s.i1,
                    s.direction.as_str(),
                    s.high,
                    s.low,
                    s.p0,
                    s.p1,
                    s.confirmed,
                )
            })
            .collect()
    }

    /// 已处理的 bar 总数。
    #[getter]
    fn bar_count(&self) -> i64 {
        self.inner.bar_count()
    }
}

/// 线段 v1 批量构造（特征序列法）。逐位等价于 Python
/// `segments_from_strokes_v1(strokes, min_seg_strokes, extend_mode)`。
///
/// 用法（Python）：
///
/// ```text
/// from newchan_rust import segments_from_strokes_v1
/// # strokes: list[(i0, i1, direction, high, low, p0, p1, confirmed)]
/// segs = segments_from_strokes_v1(strokes, 3, "strict")
/// ```
///
/// 返回 list[SegmentTuple]（嵌套元组，见 `SegmentTuple` 文档）。
#[pyfunction]
#[pyo3(signature = (strokes, min_seg_strokes = 3, extend_mode = "strict"))]
fn segments_from_strokes_v1(
    strokes: Vec<(usize, usize, String, f64, f64, f64, f64, bool)>,
    min_seg_strokes: usize,
    extend_mode: &str,
) -> Vec<SegmentTuple> {
    let strokes: Vec<Stroke> = strokes
        .into_iter()
        .map(|(i0, i1, dir, high, low, p0, p1, confirmed)| Stroke {
            i0,
            i1,
            direction: parse_direction(&dir),
            high,
            low,
            p0,
            p1,
            confirmed,
        })
        .collect();

    let extend_strict = extend_mode == "strict";
    let segs = segment::segments_from_strokes_v1(&strokes, min_seg_strokes, extend_strict);

    segs.iter()
        .map(|s| {
            let be = s.break_evidence.map(|b| {
                (b.trigger_stroke_k, b.fractal_abc, b.gap_type.as_str())
            });
            (
                (
                    s.s0,
                    s.s1,
                    s.i0,
                    s.i1,
                    s.direction.as_str(),
                    s.high,
                    s.low,
                    s.confirmed,
                    s.kind.as_str(),
                ),
                (
                    s.ep0_i,
                    s.ep0_price,
                    s.ep0_type.as_str(),
                    s.ep1_i,
                    s.ep1_price,
                    s.ep1_type.as_str(),
                    s.p0,
                    s.p1,
                ),
                be,
            )
        })
        .collect()
}

/// 中枢输出元组（12 元素，PyO3 上限）：
///   (zd, zg, seg_start, seg_end, seg_count, settled, break_seg,
///    break_direction, first_seg_s0, last_seg_s1, gg, dd)
type ZhongshuTuple = (
    f64,
    f64,
    usize,
    usize,
    usize,
    bool,
    i64,
    &'static str,
    usize,
    usize,
    f64,
    f64,
);

fn zhongshu_to_tuple(z: &zhongshu::Zhongshu) -> ZhongshuTuple {
    (
        z.zd,
        z.zg,
        z.seg_start,
        z.seg_end,
        z.seg_count,
        z.settled,
        z.break_seg,
        z.break_direction.as_str(),
        z.first_seg_s0,
        z.last_seg_s1,
        z.gg,
        z.dd,
    )
}

/// 线段中枢。逐位等价于 Python `zhongshu_from_segments`。
///
/// `segments`: list[(s0, s1, high, low, confirmed, kind_is_settled)]，过滤在 Rust 内做。
#[pyfunction]
fn zhongshu_from_segments(
    segments: Vec<(usize, usize, f64, f64, bool, bool)>,
) -> Vec<ZhongshuTuple> {
    zhongshu::zhongshu_from_segments(&segments)
        .iter()
        .map(zhongshu_to_tuple)
        .collect()
}

/// 笔中枢。逐位等价于 Python `zhongshu_from_strokes`。
///
/// `strokes`: list[(i0, i1, high, low, confirmed)]，过滤在 Rust 内做。
#[pyfunction]
fn zhongshu_from_strokes(
    strokes: Vec<(usize, usize, f64, f64, bool)>,
) -> Vec<ZhongshuTuple> {
    zhongshu::zhongshu_from_strokes(&strokes)
        .iter()
        .map(zhongshu_to_tuple)
        .collect()
}

/// 解析中枢突破方向字符串。
fn parse_break_dir(s: &str) -> zhongshu::BreakDir {
    match s {
        "" => zhongshu::BreakDir::None,
        "up" => zhongshu::BreakDir::Up,
        "down" => zhongshu::BreakDir::Down,
        other => panic!("invalid break_direction: {other:?}"),
    }
}

/// 中枢输入元组（= zhongshu_from_* 的输出格式，12 元素）。
type ZhongshuInput = (
    f64,
    f64,
    usize,
    usize,
    usize,
    bool,
    i64,
    String,
    usize,
    usize,
    f64,
    f64,
);

/// 走势输出元组（嵌套，15 字段规避 PyO3 ≤12 限制）：
///   ((kind, direction, seg_start, seg_end, zs_start, zs_end, zs_count, settled),
///    (high, low, first_seg_s0, last_seg_s1, zg_max, zd_min, persistence))
type MoveTuple = (
    (&'static str, &'static str, i64, i64, usize, usize, usize, bool),
    (f64, f64, usize, usize, f64, f64, f64),
);

/// 走势输入元组（= MoveTuple 同结构，但 kind/direction 用 String 接收 Python 输入）：
///   ((kind, direction, seg_start, seg_end, zs_start, zs_end, zs_count, settled),
///    (high, low, first_seg_s0, last_seg_s1, zg_max, zd_min, persistence))
type MoveTupleIn = (
    (String, String, i64, i64, usize, usize, usize, bool),
    (f64, f64, usize, usize, f64, f64, f64),
);

/// Rust Move → 输出元组（嵌套 15 字段）。
fn move_to_tuple(m: &moves::Move) -> MoveTuple {
    (
        (
            m.kind.as_str(),
            m.direction.as_str(),
            m.seg_start,
            m.seg_end,
            m.zs_start,
            m.zs_end,
            m.zs_count,
            m.settled,
        ),
        (
            m.high,
            m.low,
            m.first_seg_s0,
            m.last_seg_s1,
            m.zg_max,
            m.zd_min,
            m.persistence,
        ),
    )
}

/// 输入元组 → Rust Move（PH 层往返用；kind/direction 字符串解析）。
fn move_from_tuple(t: &MoveTupleIn) -> moves::Move {
    let (head, tail) = t;
    moves::Move {
        kind: parse_move_kind(&head.0),
        direction: parse_direction(&head.1),
        seg_start: head.2,
        seg_end: head.3,
        zs_start: head.4,
        zs_end: head.5,
        zs_count: head.6,
        settled: head.7,
        high: tail.0,
        low: tail.1,
        first_seg_s0: tail.2,
        last_seg_s1: tail.3,
        zg_max: tail.4,
        zd_min: tail.5,
        persistence: tail.6,
    }
}

/// 走势类型构造。逐位等价于 Python `moves_from_zhongshus`。
///
/// `zhongshus`: list[zhongshu 12 元组]（zhongshu_from_* 的输出）。
#[pyfunction]
#[pyo3(signature = (zhongshus, num_segments = None))]
fn moves_from_zhongshus(
    zhongshus: Vec<ZhongshuInput>,
    num_segments: Option<usize>,
) -> Vec<MoveTuple> {
    let zs: Vec<zhongshu::Zhongshu> = zhongshus
        .into_iter()
        .map(|t| zhongshu::Zhongshu {
            zd: t.0,
            zg: t.1,
            seg_start: t.2,
            seg_end: t.3,
            seg_count: t.4,
            settled: t.5,
            break_seg: t.6,
            break_direction: parse_break_dir(&t.7),
            first_seg_s0: t.8,
            last_seg_s1: t.9,
            gg: t.10,
            dd: t.11,
        })
        .collect();

    moves::moves_from_zhongshus(&zs, num_segments)
        .iter()
        .map(move_to_tuple)
        .collect()
}

/// 从中枢 12 元组提取 PH 层所需的 (dd, gg) 价格视图。
fn zs_price_views(zhongshus: &[ZhongshuInput]) -> Vec<ph::ZsPriceView> {
    zhongshus
        .iter()
        .map(|t| ph::ZsPriceView { dd: t.11, gg: t.10 })
        .collect()
}

/// 单个 Move 的 H0 persistence（闭式 max-min）。逐位等价于
/// `ph_layer.compute_move_persistence`。
///
/// `move_tuple`: MoveTuple 同结构输入（仅 zs_start/zs_end/high/low 参与计算）。
/// `zhongshus`: list[zhongshu 12 元组]（仅 dd/gg 参与）。
#[pyfunction]
fn compute_move_persistence(move_tuple: MoveTupleIn, zhongshus: Vec<ZhongshuInput>) -> f64 {
    let m = move_from_tuple(&move_tuple);
    let zss = zs_price_views(&zhongshus);
    ph::compute_move_persistence(&m, &zss)
}

/// 为 Move 列表附着 persistence，返回新列表。逐位等价于
/// `ph_layer.attach_persistence`。
///
/// `moves`: list[MoveTuple 同结构]（moves_from_zhongshus 的输出）。
/// `zhongshus`: list[zhongshu 12 元组]。返回 list[MoveTuple]（persistence 已填）。
#[pyfunction]
fn attach_persistence(moves: Vec<MoveTupleIn>, zhongshus: Vec<ZhongshuInput>) -> Vec<MoveTuple> {
    let ms: Vec<moves::Move> = moves.iter().map(move_from_tuple).collect();
    let zss = zs_price_views(&zhongshus);
    ph::attach_persistence(&ms, &zss)
        .iter()
        .map(move_to_tuple)
        .collect()
}

/// 递归终止判据。逐位等价于 `ph_layer.should_stop_recursion`。
///
/// `curr_moves` / `prev_moves`: list[MoveTuple 同结构]（仅 settled/persistence 参与）。
#[pyfunction]
fn should_stop_recursion(curr_moves: Vec<MoveTupleIn>, prev_moves: Vec<MoveTupleIn>) -> bool {
    let curr: Vec<moves::Move> = curr_moves.iter().map(move_from_tuple).collect();
    let prev: Vec<moves::Move> = prev_moves.iter().map(move_from_tuple).collect();
    ph::should_stop_recursion(&curr, &prev)
}

/// 解析走势类型字符串。
fn parse_move_kind(s: &str) -> moves::MoveKind {
    match s {
        "consolidation" => moves::MoveKind::Consolidation,
        "trend" => moves::MoveKind::Trend,
        other => panic!("invalid move kind: {other:?}"),
    }
}

/// 解析背驰类型字符串。
fn parse_div_kind(s: &str) -> divergence::DivKind {
    match s {
        "trend" => divergence::DivKind::Trend,
        "consolidation" => divergence::DivKind::Consolidation,
        other => panic!("invalid divergence kind: {other:?}"),
    }
}

/// 解析背驰方向字符串。
fn parse_div_dir(s: &str) -> divergence::DivDir {
    match s {
        "top" => divergence::DivDir::Top,
        "bottom" => divergence::DivDir::Bottom,
        other => panic!("invalid divergence direction: {other:?}"),
    }
}

/// 段输入元组 (direction, high, low, i0, i1) → SegView。
type SegInput = (String, f64, f64, usize, usize);

fn to_seg_views(segments: &[SegInput]) -> Vec<divergence::SegView> {
    segments
        .iter()
        .map(|(dir, high, low, i0, i1)| divergence::SegView {
            direction: parse_direction(dir),
            high: *high,
            low: *low,
            i0: *i0,
            i1: *i1,
            // 此 Python 批量入口的段元组不携带 confirmed ⟹ 不支持 require_settled
            // （下方调用恒传 false）；settled=true 使合取门即便误开也为 no-op。
            settled: true,
        })
        .collect()
}

/// 走势输入元组 (kind, direction, seg_start, seg_end, zs_start, zs_end, zs_count, settled) → MoveView。
type MoveInput = (String, String, i64, i64, usize, usize, usize, bool);

fn to_move_views(moves: &[MoveInput]) -> Vec<divergence::MoveView> {
    moves
        .iter()
        .map(|(kind, dir, seg_start, seg_end, zs_start, zs_end, zs_count, settled)| {
            divergence::MoveView {
                kind: parse_move_kind(kind),
                direction: parse_direction(dir),
                seg_start: *seg_start,
                seg_end: *seg_end,
                zs_start: *zs_start,
                zs_end: *zs_end,
                zs_count: *zs_count,
                settled: *settled,
            }
        })
        .collect()
}

/// 背驰输出元组（嵌套，15 字段规避 PyO3 ≤12 限制）：
///   ((kind, direction, level_id, seg_a_start, seg_a_end, seg_c_start, seg_c_end, center_idx),
///    (force_a, force_c, confirmed, dif_peak_a, dif_peak_c, hist_peak_a, hist_peak_c))
type DivergenceTuple = (
    (&'static str, &'static str, i64, i64, i64, i64, i64, usize),
    (f64, f64, bool, f64, f64, f64, f64),
);

fn divergence_to_tuple(d: &divergence::Divergence) -> DivergenceTuple {
    (
        (
            d.kind.as_str(),
            d.direction.as_str(),
            d.level_id,
            d.seg_a_start,
            d.seg_a_end,
            d.seg_c_start,
            d.seg_c_end,
            d.center_idx,
        ),
        (
            d.force_a,
            d.force_c,
            d.confirmed,
            d.dif_peak_a,
            d.dif_peak_c,
            d.hist_peak_a,
            d.hist_peak_c,
        ),
    )
}

/// 背驰检测。逐位等价于 Python
/// `divergences_from_moves_v1(segments, zhongshus, moves, level_id, df_macd=, merged_to_raw=)`。
///
/// `zhongshus`: list[(zd, zg, seg_start, seg_end, settled)]（背驰只需这 5 字段）。
///
/// MACD 三维度路径（第七层接通）：同时传 `macd_col`/`hist_col`/`merged_to_raw`（三者均非 None）
/// 时走 MACD（⇔ Python `df_macd is not None and merged_to_raw is not None`）；任一为 None 走
/// 价格振幅 fallback。
///   - `macd_col`: df_macd['macd'] 列（list[float]）。
///   - `hist_col`: df_macd['hist'] 列（list[float]）。
///   - `merged_to_raw`: list[(raw_lo, raw_hi)]。
#[pyfunction]
#[pyo3(signature = (segments, zhongshus, moves, level_id, macd_col = None, hist_col = None, merged_to_raw = None))]
fn divergences_from_moves_v1(
    segments: Vec<SegInput>,
    zhongshus: Vec<(f64, f64, usize, usize, bool)>,
    moves: Vec<MoveInput>,
    level_id: i64,
    macd_col: Option<Vec<f64>>,
    hist_col: Option<Vec<f64>>,
    merged_to_raw: Option<Vec<(usize, usize)>>,
) -> Vec<DivergenceTuple> {
    let segs = to_seg_views(&segments);
    let zss: Vec<divergence::ZsView> = zhongshus
        .iter()
        .map(|&(zd, zg, seg_start, seg_end, settled)| divergence::ZsView {
            zd,
            zg,
            seg_start,
            seg_end,
            settled,
        })
        .collect();
    let mvs = to_move_views(&moves);

    // 三者均非 None ⇔ Python df_macd is not None and merged_to_raw is not None。
    let macd_ctx = match (&macd_col, &hist_col, &merged_to_raw) {
        (Some(m), Some(h), Some(m2r)) => Some(divergence::MacdCtx {
            macd: m.as_slice(),
            hist: h.as_slice(),
            merged_to_raw: m2r.as_slice(),
        }),
        _ => None,
    };

    divergence::divergences_from_moves_v1(&segs, &zss, &mvs, level_id, macd_ctx.as_ref())
        .iter()
        .map(divergence_to_tuple)
        .collect()
}

/// 买卖点输出元组（嵌套，14 字段规避 PyO3 ≤12 限制）：
///   ((kind, side, level_id, seg_idx, move_seg_start, confirmed, settled),
///    (center_zd, center_zg, price, bar_idx),
///    divergence_key, center_seg_start, overlaps_with)
type BspTuple = (
    (&'static str, &'static str, i64, i64, i64, bool, bool),
    (f64, f64, f64, i64),
    Option<(usize, usize, usize)>,
    Option<usize>,
    Option<&'static str>,
);

fn bsp_to_tuple(b: &buysellpoint::BuySellPoint) -> BspTuple {
    (
        (
            b.kind.as_str(),
            b.side.as_str(),
            b.level_id,
            b.seg_idx,
            b.move_seg_start,
            b.confirmed,
            b.settled,
        ),
        (b.center_zd, b.center_zg, b.price, b.bar_idx),
        b.divergence_key,
        b.center_seg_start,
        b.overlaps_with.as_str(),
    )
}

/// 买卖点检测。逐位等价于 Python
/// `buysellpoints_from_level(segments, zhongshus, moves, divergences, level_id)`。
///
/// `zhongshus`: list[(zd, zg, seg_start, seg_end, settled, break_direction, break_seg)]。
/// `divergences`: list[(kind, direction, center_idx, seg_c_start, seg_c_end, force_a, force_c)]
///   （BSP 仅消费这 7 个字段；其余背驰字段不参与买卖点构造）。
#[pyfunction]
fn buysellpoints_from_level(
    segments: Vec<SegInput>,
    zhongshus: Vec<(f64, f64, usize, usize, bool, String, i64)>,
    moves: Vec<MoveInput>,
    divergences: Vec<(String, String, usize, i64, i64, f64, f64)>,
    level_id: i64,
) -> Vec<BspTuple> {
    let segs = to_seg_views(&segments);
    let zss: Vec<divergence::ZsView> = zhongshus
        .iter()
        .map(|&(zd, zg, seg_start, seg_end, settled, _, _)| divergence::ZsView {
            zd,
            zg,
            seg_start,
            seg_end,
            settled,
        })
        .collect();
    let zs_break: Vec<(bool, zhongshu::BreakDir, i64)> = zhongshus
        .iter()
        .map(|(_, _, _, _, settled, bd, bs)| (*settled, parse_break_dir(bd), *bs))
        .collect();
    let mvs = to_move_views(&moves);
    let divs: Vec<divergence::Divergence> = divergences
        .iter()
        .map(
            |(kind, dir, center_idx, seg_c_start, seg_c_end, force_a, force_c)| {
                divergence::Divergence {
                    kind: parse_div_kind(kind),
                    direction: parse_div_dir(dir),
                    level_id,
                    seg_a_start: 0,
                    seg_a_end: 0,
                    seg_c_start: *seg_c_start,
                    seg_c_end: *seg_c_end,
                    center_idx: *center_idx,
                    force_a: *force_a,
                    force_c: *force_c,
                    confirmed: true,
                    dif_peak_a: 0.0,
                    dif_peak_c: 0.0,
                    hist_peak_a: 0.0,
                    hist_peak_c: 0.0,
                }
            },
        )
        .collect();

    buysellpoint::buysellpoints_from_level(&segs, &zss, &zs_break, &mvs, &divs, level_id, false)
        .iter()
        .map(bsp_to_tuple)
        .collect()
}

// ── MACD 力度层（第七层）─────────────────────────────────

/// 批量 MACD。逐位等价于 Python `compute_macd(df_raw, fast, slow, signal)`。
///
/// `close`: list[float]。返回 (macd, signal, hist) 三个 list[float]，长度均 = len(close)。
/// EMA 走 pandas ewm(adjust=False) 形式 `prev + α·(x−prev)`。
#[pyfunction]
#[pyo3(signature = (close, fast = 12, slow = 26, signal = 9))]
fn compute_macd(
    close: Vec<f64>,
    fast: i64,
    slow: i64,
    signal: i64,
) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let s = macd::compute_macd_batch(&close, fast, slow, signal);
    (s.macd, s.signal, s.hist)
}

/// 指定 raw bar 范围的 MACD 面积。逐位等价于 Python
/// `macd_area_for_range(df_macd, raw_i0, raw_i1)`。
///
/// `hist`: list[float]（df_macd['hist'] 列）。`raw_i0`/`raw_i1`: i64（可为负，闭区间）。
/// 返回 (area_total, area_pos, area_neg, n_bars)，前三个已 round(·, 6)。
/// area 累加用 numpy pairwise summation（bit-exact）。
#[pyfunction]
fn macd_area_for_range(hist: Vec<f64>, raw_i0: i64, raw_i1: i64) -> (f64, f64, f64, usize) {
    let a = macd::macd_area_for_range(&hist, raw_i0, raw_i1);
    (a.area_total, a.area_pos, a.area_neg, a.n_bars)
}

/// DIF 区间峰值。逐位等价于 Python `dif_peak_for_range(df_macd, raw_i0, raw_i1, trend_direction)`。
///
/// `macd`: list[float]（df_macd['macd'] 列）。`up`: True="up"（取 max）/ False="down"（取 abs(min)）。
#[pyfunction]
fn dif_peak_for_range(macd: Vec<f64>, raw_i0: i64, raw_i1: i64, up: bool) -> f64 {
    macd::dif_peak_for_range(&macd, raw_i0, raw_i1, up)
}

/// HIST 区间峰值。逐位等价于 Python `histogram_peak_for_range(df_macd, raw_i0, raw_i1, trend_direction)`。
#[pyfunction]
fn histogram_peak_for_range(hist: Vec<f64>, raw_i0: i64, raw_i1: i64, up: bool) -> f64 {
    macd::histogram_peak_for_range(&hist, raw_i0, raw_i1, up)
}

/// B 段黄白线穿越 0 轴检测（T4 前提）。逐位等价于 Python `_b_segment_crosses_zero`
/// 的范围判定核心（macd 列在 [raw_i0, raw_i1] 内同时有正负值）。
///
/// `macd`: list[float]（df_macd['macd'] 列）。
#[pyfunction]
fn b_segment_crosses_zero(macd: Vec<f64>, raw_i0: i64, raw_i1: i64) -> bool {
    macd::b_segment_crosses_zero(&macd, raw_i0, raw_i1)
}

/// 在线 MACD 状态机。逐位等价于 Python `OnlineMacdState`。
///
/// 递推形式 `α·x + (1−α)·prev`（**与 compute_macd 的 pandas ewm 形式不同**——见 macd.rs 模块文档）。
/// 用法（Python）：
///
/// ```text
/// from newchan_rust import OnlineMacdState
/// st = OnlineMacdState()                 # fast=12, slow=26, signal=9
/// for c in closes:
///     macd, signal, hist = st.update(c)
/// m, s, h = st.series()                  # 等价于 to_dataframe() 三列
/// ```
#[pyclass(name = "OnlineMacdState")]
struct PyOnlineMacdState {
    inner: macd::OnlineMacdState,
}

#[pymethods]
impl PyOnlineMacdState {
    #[new]
    #[pyo3(signature = (fast = 12, slow = 26, signal = 9))]
    fn new(fast: i64, slow: i64, signal: i64) -> Self {
        PyOnlineMacdState {
            inner: macd::OnlineMacdState::new(fast, slow, signal),
        }
    }

    /// 摄入一根 bar 的 close，返回 (macd, signal, hist)。O(1)。
    fn update(&mut self, close: f64) -> (f64, f64, f64) {
        self.inner.update(close)
    }

    /// 返回完整历史三列 (macd, signal, hist)（等价于 to_dataframe()）。
    fn series(&self) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let s = self.inner.series();
        (s.macd, s.signal, s.hist)
    }

    fn reset(&mut self) {
        self.inner.reset();
    }

    #[getter]
    fn n_bars(&self) -> usize {
        self.inner.n_bars()
    }
}

/// Rust Segment → SegmentTuple（与 `segments_from_strokes_v1` 输出同格式）。
fn segment_to_tuple(s: &segment::Segment) -> SegmentTuple {
    let be = s
        .break_evidence
        .map(|b| (b.trigger_stroke_k, b.fractal_abc, b.gap_type.as_str()));
    (
        (
            s.s0,
            s.s1,
            s.i0,
            s.i1,
            s.direction.as_str(),
            s.high,
            s.low,
            s.confirmed,
            s.kind.as_str(),
        ),
        (
            s.ep0_i,
            s.ep0_price,
            s.ep0_type.as_str(),
            s.ep1_i,
            s.ep1_price,
            s.ep1_type.as_str(),
            s.p0,
            s.p1,
        ),
        be,
    )
}

/// 泛化中枢（递归级别）输出元组（11 字段，PyO3 ≤12 上限内）：
///   (zd, zg, comp_start, comp_end, comp_count, settled, break_comp,
///    break_direction, gg, dd, level_id)
type LevelZhongshuTuple = (
    f64,
    f64,
    usize,
    usize,
    usize,
    bool,
    i64,
    &'static str,
    f64,
    f64,
    i64,
);

fn level_zhongshu_to_tuple(z: &level::LevelZhongshu) -> LevelZhongshuTuple {
    (
        z.zd,
        z.zg,
        z.comp_start,
        z.comp_end,
        z.comp_count,
        z.settled,
        z.break_comp,
        z.break_direction.as_str(),
        z.gg,
        z.dd,
        z.level_id,
    )
}

/// 一个递归级别快照元组：(level_id, zhongshus, moves)。
type LevelSnapshotTuple = (i64, Vec<LevelZhongshuTuple>, Vec<MoveTuple>);

/// 递归编排器的 Python 绑定。逐 bar 驱动全链，结构化快照逐位等价于 Python
/// `RecursiveOrchestrator.process_bar(bar)` 的 `RecursiveOrchestratorSnapshot`。
///
/// 用法（Python）：
///
/// ```text
/// from newchan_rust import RecursiveOrchestrator
/// orch = RecursiveOrchestrator(max_levels=6, stroke_mode="wide")
/// for o, h, l, c in bars:
///     orch.process_bar(o, h, l, c)
/// strokes = orch.current_strokes()      # 同 BiEngine.current_strokes()
/// segments = orch.current_segments()    # list[SegmentTuple]
/// zhongshus = orch.current_zhongshus()  # list[ZhongshuTuple]
/// moves = orch.current_moves()          # list[MoveTuple]
/// bsps = orch.current_buysellpoints()   # list[BspTuple]
/// rec = orch.current_recursive()        # list[(level_id, [LevelZhongshuTuple], [MoveTuple])]
/// ```
#[pyclass(name = "RecursiveOrchestrator")]
struct PyRecursiveOrchestrator {
    inner: orchestrator::RecursiveOrchestrator,
    /// 笔中枢买卖点全链增量引擎（lazy 创建；level_id 首次调用时绑定）。
    inc_bz: Option<bi_zhongshu_bsp::IncrementalBiZhongshuBsp>,
    /// 已 push 到 inc_bz 的 confirmed 笔数。
    inc_pushed: usize,

    // ── 走势级（level-1 segment）买卖点 delta 信号：消除调用层每-bar 全量
    //    current_buysellpoints() marshal + _scan_new 扫描的 B-scaling O(N²) ──
    /// confirmed 买卖点去重集（复刻 Python `trend_seen`，键 = (kind, side, seg_idx)）。
    trend_seen: HashSet<(BspKind, Side, i64)>,
    /// type2 买卖点去重集（复刻 Python `type2_seen`，键 = (seg_idx, move_seg_start)）。
    trend_type2_seen: HashSet<(i64, i64)>,
    /// 上次扫描时见到的 bsp_epoch（epoch 未变 ⟹ prev_bsps 逐位不变 ⟹ O(1) 跳过）。
    trend_last_bsp_epoch: u64,
    /// 走势级事件流去重集（复刻调用方 _scan_events_rust 的 seen；键含 confirmed——
    /// candidate/confirmed 分别入流一次。与 trend_seen 独立——两接口调用时序解耦）。
    trend_ev_seen: HashSet<(BspKind, Side, i64, bool)>,
    /// 走势级背驰事件流去重集（复刻调用方 _scan_div_events 的 seen）。
    trend_div_seen: HashSet<(divergence::DivKind, divergence::DivDir, i64)>,
}

#[pymethods]
impl PyRecursiveOrchestrator {
    #[new]
    #[pyo3(signature = (
        max_levels = 6,
        stroke_mode = "wide",
        min_strict_sep = 5,
        reset_dir_on_fractal = false,
        new_raw_gap_min = 3,
        enable_macd_divergence = false,
        enable_bsp = true,
        require_settled_subseg = false,
    ))]
    fn new(
        max_levels: i64,
        stroke_mode: &str,
        min_strict_sep: i64,
        reset_dir_on_fractal: bool,
        new_raw_gap_min: i64,
        enable_macd_divergence: bool,
        enable_bsp: bool,
        require_settled_subseg: bool,
    ) -> Self {
        PyRecursiveOrchestrator {
            inner: orchestrator::RecursiveOrchestrator::new(
                max_levels,
                stroke_mode,
                min_strict_sep,
                reset_dir_on_fractal,
                new_raw_gap_min,
                enable_macd_divergence,
                enable_bsp,
                require_settled_subseg,
            ),
            inc_bz: None,
            inc_pushed: 0,
            trend_seen: HashSet::new(),
            trend_type2_seen: HashSet::new(),
            trend_last_bsp_epoch: 0,
            trend_ev_seen: HashSet::new(),
            trend_div_seen: HashSet::new(),
        }
    }

    /// 处理一根新 K 线（仅传 OHLC；结构化状态不依赖时间戳）。
    fn process_bar(&mut self, open: f64, high: f64, low: f64, close: f64) {
        self.inner.process_bar(open, high, low, close);
    }

    fn reset(&mut self) {
        self.inner.reset();
        self.inc_bz = None;
        self.inc_pushed = 0;
        self.trend_seen.clear();
        self.trend_type2_seen.clear();
        self.trend_last_bsp_epoch = 0;
        self.trend_ev_seen.clear();
        self.trend_div_seen.clear();
    }

    fn current_strokes(
        &self,
    ) -> Vec<(usize, usize, &'static str, f64, f64, f64, f64, bool)> {
        self.inner
            .strokes()
            .iter()
            .map(|s| {
                (
                    s.i0,
                    s.i1,
                    s.direction.as_str(),
                    s.high,
                    s.low,
                    s.p0,
                    s.p1,
                    s.confirmed,
                )
            })
            .collect()
    }

    fn current_segments(&self) -> Vec<SegmentTuple> {
        self.inner.segments().iter().map(segment_to_tuple).collect()
    }

    fn current_zhongshus(&self) -> Vec<ZhongshuTuple> {
        self.inner.zhongshus().iter().map(zhongshu_to_tuple).collect()
    }

    fn current_moves(&self) -> Vec<MoveTuple> {
        self.inner.moves().iter().map(move_to_tuple).collect()
    }

    /// **走势 settle delta 接口**——返回自上次调用以来新结算的走势（MoveTuple 列表）。
    ///
    /// 逐位等价于 E 引擎 `diff_moves(prev_moves, current_moves())` 中 MoveSettleV1 对应的
    /// settled move。消除每-epoch 全量 `current_moves()` marshal + Python `diff_moves`
    /// （O(B·n_moves) 超线性 → 端到端 O(n_seg·新结算)）。move_epoch 未变 ⟹ 返回空（O(1)）。
    fn take_move_settle_events(&mut self) -> Vec<MoveTuple> {
        self.inner.move_settle_delta().iter().map(move_to_tuple).collect()
    }

    fn current_buysellpoints(&self) -> Vec<BspTuple> {
        self.inner.buysellpoints().iter().map(bsp_to_tuple).collect()
    }

    fn current_recursive(&self) -> Vec<LevelSnapshotTuple> {
        self.inner
            .recursive()
            .iter()
            .map(|snap| {
                (
                    snap.level_id,
                    snap.zhongshus.iter().map(level_zhongshu_to_tuple).collect(),
                    snap.moves.iter().map(move_to_tuple).collect(),
                )
            })
            .collect()
    }

    /// 当前笔总数（O(1)，廉价门控用——避免每 bar 全量 marshal `current_strokes`）。
    fn stroke_count(&self) -> usize {
        self.inner.strokes().len()
    }

    /// 笔 `[k:]` 的终点价 `p1`（bi-PH 只消费新增笔的 p1，O(新增) marshal）。
    fn strokes_p1_since(&self, k: usize) -> Vec<f64> {
        let strokes = self.inner.strokes();
        if k >= strokes.len() {
            return Vec::new();
        }
        strokes[k..].iter().map(|s| s.p1).collect()
    }

    /// 笔中枢级 confirmed/candidate 买卖点——**直读内部笔，零 stroke marshal**。
    ///
    /// 逐位等价于 Python `per_level_bsp.confirmed_bsp_bi_zhongshu(strokes)`：
    ///   confirmed 笔 → `zhongshu_from_strokes` → `moves_from_zhongshus(num_segments=len)`
    ///   → `divergences_from_moves_v1(df_macd=None)` → `buysellpoints_from_level`。
    /// 合并全链为单次 Rust 调用，消除原 Python 端 5 次中间 tuple 列表构造 + 往返
    /// （profile 测得 Python marshalling 占 bi-zhongshu 总耗时 ~54%）。
    /// 返回 BspTuple 列表（调用方按 (kind,side,seg_idx) 去重检测新增 confirmed）。
    #[pyo3(signature = (level_id = 1))]
    fn current_bi_zhongshu_buysellpoints(&self, level_id: i64) -> Vec<BspTuple> {
        let strokes = self.inner.strokes();
        // 过滤 confirmed 笔（per_level_bsp: confirmed = [s for s in strokes if s.confirmed]）。
        let confirmed: Vec<&stroke::Stroke> = strokes.iter().filter(|s| s.confirmed).collect();
        if confirmed.len() < 3 {
            return Vec::new();
        }
        // zhongshu_from_strokes 输入: (i0, i1, high, low, confirmed)。
        let zs_in: Vec<(usize, usize, f64, f64, bool)> = confirmed
            .iter()
            .map(|s| (s.i0, s.i1, s.high, s.low, s.confirmed))
            .collect();
        let zhongshus = zhongshu::zhongshu_from_strokes(&zs_in);
        let moves = moves::moves_from_zhongshus(&zhongshus, Some(confirmed.len()));
        // SegView ← confirmed 笔（笔原生暴露 direction/high/low/i0/i1，525号组件来源无关性）。
        let segs: Vec<divergence::SegView> = confirmed
            .iter()
            .map(|s| divergence::SegView {
                direction: s.direction,
                high: s.high,
                low: s.low,
                i0: s.i0,
                i1: s.i1,
                settled: s.confirmed, // 笔此处全 confirmed ⟹ settle 门恒真无效（require_settled=false）
            })
            .collect();
        let zss: Vec<divergence::ZsView> = zhongshus
            .iter()
            .map(|z| divergence::ZsView {
                zd: z.zd,
                zg: z.zg,
                seg_start: z.seg_start,
                seg_end: z.seg_end,
                settled: z.settled,
            })
            .collect();
        let mvs: Vec<divergence::MoveView> = moves
            .iter()
            .map(|m| divergence::MoveView {
                kind: m.kind,
                direction: m.direction,
                seg_start: m.seg_start,
                seg_end: m.seg_end,
                zs_start: m.zs_start,
                zs_end: m.zs_end,
                zs_count: m.zs_count,
                settled: m.settled,
            })
            .collect();
        let divs = divergence::divergences_from_moves_v1(&segs, &zss, &mvs, level_id, None);
        let zs_break: Vec<(bool, zhongshu::BreakDir, i64)> = zhongshus
            .iter()
            .map(|z| (z.settled, z.break_direction, z.break_seg))
            .collect();
        buysellpoint::buysellpoints_from_level(&segs, &zss, &zs_break, &mvs, &divs, level_id, false)
            .iter()
            .map(bsp_to_tuple)
            .collect()
    }

    /// 笔中枢级 confirmed/candidate 买卖点——**全链增量引擎**（消除上一方法的 O(strokes²)）。
    ///
    /// 逐位等价于 `current_bi_zhongshu_buysellpoints(level_id)`，但摊还 O(window)/调用：
    /// confirmed 笔 append-only push 进 `IncrementalBiZhongshuBsp`（四层增量），返回缓存全量。
    /// confirmed 笔 = snapshot[..len-1]（最后一笔 unconfirmed）；只 push 新增 O(Δ)。
    ///
    /// 调用契约：仅在 stroke_count 增长时调用（方向反转 ⟹ 前序 confirmed 笔已永久固定，
    /// append-only 前提成立）。注：返回全量 marshal O(B)；如只需新信号用
    /// `bi_zhongshu_new_signals`（delta 接口，端到端 O(N)）。
    #[pyo3(signature = (level_id = 1))]
    fn current_bi_zhongshu_buysellpoints_inc(&mut self, level_id: i64) -> Vec<BspTuple> {
        self.sync_inc(level_id);
        self.inc_bz
            .as_ref()
            .map(|e| e.current().iter().map(bsp_to_tuple).collect())
            .unwrap_or_default()
    }

    /// 笔中枢级背驰流——**增量引擎 divs 层缓存直读**（O(n_div) marshal/次，零重算）。
    ///
    /// 返回 list[(kind, direction, seg_c_end, force_a, force_c, price)]：
    ///   kind ∈ {"trend","consolidation"}；direction ∈ {"top","bottom"}；
    ///   price = 背驰段端点价（top→段 high / bottom→段 low，与 type1 BSP price 同构）。
    /// 逐位等价于全量 `divergences_from_moves_v1(confirmed笔, 笔中枢, 笔级走势, level_id)`
    /// （IncrementalDivergences.current() 的等价性契约）。盘整背驰不构造 BSP，
    /// 此前对调用方完全不可见——本接口是其唯一暴露面（有机赋格 div_events）。
    /// 调用契约同 `_inc`：仅在 stroke_count 增长时调用（confirmed 笔 append-only）。
    #[pyo3(signature = (level_id = 1))]
    fn current_bi_zhongshu_divergences_inc(
        &mut self,
        level_id: i64,
    ) -> Vec<(&'static str, &'static str, i64, f64, f64, f64)> {
        self.sync_inc(level_id);
        let e = match self.inc_bz.as_ref() {
            Some(e) => e,
            None => return Vec::new(),
        };
        let segs = e.stroke_views();
        e.divergences()
            .iter()
            .map(|d| {
                let idx = d.seg_c_end as usize;
                let price = if idx < segs.len() {
                    match d.direction {
                        divergence::DivDir::Top => segs[idx].high,
                        divergence::DivDir::Bottom => segs[idx].low,
                    }
                } else {
                    0.0
                };
                (
                    d.kind.as_str(),
                    d.direction.as_str(),
                    d.seg_c_end,
                    d.force_a,
                    d.force_c,
                    price,
                )
            })
            .collect()
    }

    /// D3 dir_row[2]：笔级走势尾 move 方向（有机赋格 v2 §7 镜像读数；O(1) 只读
    /// surfacing，divergences 先例）。调用契约同 `_inc`（stroke_count 增长 bar）。
    /// 无走势 → None。
    #[pyo3(signature = (level_id = 1))]
    fn bi_zhongshu_last_move_dir(&mut self, level_id: i64) -> Option<&'static str> {
        self.sync_inc(level_id);
        self.inc_bz.as_ref().and_then(|e| e.last_move_direction()).map(|d| d.as_str())
    }

    /// D3 dir_row[3]：走势级（L1）尾 move 方向（O(1) 只读）。无走势 → None。
    fn trend_last_move_dir(&self) -> Option<&'static str> {
        self.inner.moves().last().map(|m| m.direction.as_str())
    }

    /// 趋势态行 trend_row[2]：笔级走势尾 move kind（"trend"/"consolidation"）。
    /// 38课循环 voice 的宿主趋势态读数（O(1) 只读 surfacing，dir 行同先例）。
    /// 调用契约同 `_inc`（仅 stroke_count 增长 bar）。无走势 → None。
    #[pyo3(signature = (level_id = 1))]
    fn bi_zhongshu_last_move_kind(&mut self, level_id: i64) -> Option<&'static str> {
        self.sync_inc(level_id);
        self.inc_bz.as_ref().and_then(|e| e.last_move_kind()).map(|k| k.as_str())
    }

    /// 趋势态行 trend_row[3]：走势级（L1）尾 move kind（O(1) 只读）。无走势 → None。
    fn trend_last_move_kind(&self) -> Option<&'static str> {
        self.inner.moves().last().map(|m| m.kind.as_str())
    }

    /// 走势级背驰流——inc_seg_div 增量缓存直读（同上格式；O(n_div) marshal/次）。
    ///
    /// 逐位等价于引擎内部 compute_bsps 所消费的 divergences（segments 全量 /
    /// inc 中枢 / prev_moves / macd_ctx=None）。有效域 = enable_macd_divergence=false
    /// （macd 路径下 inc_seg_div 不更新，直接 panic——契约违例非数据问题）。
    /// 缓存随 bsp_key 重算更新 → 调用方用 bsp_epoch 门控与 current_buysellpoints 同步。
    fn current_trend_divergences(
        &self,
    ) -> Vec<(&'static str, &'static str, i64, f64, f64, f64)> {
        if self.inner.macd_divergence_enabled() {
            // MACD 路径：inc_seg_div 缓存不更新，返回空 Vec
            return Vec::new();
        }
        let segs = self.inner.segments();
        self.inner
            .trend_divergences()
            .iter()
            .map(|d| {
                let idx = d.seg_c_end as usize;
                let price = if idx < segs.len() {
                    match d.direction {
                        divergence::DivDir::Top => segs[idx].high,
                        divergence::DivDir::Bottom => segs[idx].low,
                    }
                } else {
                    0.0
                };
                (
                    d.kind.as_str(),
                    d.direction.as_str(),
                    d.seg_c_end,
                    d.force_a,
                    d.force_c,
                    price,
                )
            })
            .collect()
    }

    /// **delta 信号接口**——返回新触发的 confirmed 买卖点信号 (buy1, sell1, sell_any, buy_any)。
    ///
    /// 逐位等价于调用方 `_scan_new(current_bi_zhongshu_buysellpoints_inc(level_id), seg_seen)`，
    /// 但去重 seen 下沉到 Rust + 只扫尾部窗口 → 消除 marshal O(B) + 调用方扫描 O(B)，端到端 O(N)。
    #[pyo3(signature = (level_id = 1))]
    fn bi_zhongshu_new_signals(&mut self, level_id: i64) -> (bool, bool, bool, bool) {
        self.sync_inc(level_id);
        self.inc_bz
            .as_mut()
            .map(|e| e.take_new_signals())
            .unwrap_or((false, false, false, false))
    }

    /// **事件流 delta 接口**——marshal 只含新事件（消除期货长序列 O(S×B) 主导项）。
    ///
    /// 返回 (buy1, sell1, sell_any, buy_any, list[event])，event =
    /// (kind, side, seg_idx, confirmed, center_seg_start, center_zd, center_zg, price)。
    /// 逐位等价于调用方
    /// `_scan_events_rust(current_bi_zhongshu_buysellpoints_inc(level_id), seg_seen)`
    /// （organic_signals）：seen 下沉 Rust + 尾窗扫描（等价性证明见
    /// `IncrementalBiZhongshuBsp::take_new_events`）。调用契约同 `_inc`
    /// （仅 stroke_count 增长 bar）。
    #[allow(clippy::type_complexity)]
    #[pyo3(signature = (level_id = 1))]
    fn take_bi_zhongshu_bsp_events(
        &mut self,
        level_id: i64,
    ) -> (
        bool,
        bool,
        bool,
        bool,
        Vec<(&'static str, &'static str, i64, bool, Option<usize>, f64, f64, f64)>,
    ) {
        self.sync_inc(level_id);
        self.inc_bz
            .as_mut()
            .map(|e| e.take_new_events())
            .unwrap_or_default()
    }

    /// **背驰事件流 delta 接口**——marshal 只含新背驰行（行格式与
    /// `current_bi_zhongshu_divergences_inc` 同构；去重键 (kind, direction, seg_c_end)
    /// 下沉 Rust）。调用契约同 `_inc`（仅 stroke_count 增长 bar）。
    #[pyo3(signature = (level_id = 1))]
    fn take_bi_zhongshu_div_events(
        &mut self,
        level_id: i64,
    ) -> Vec<(&'static str, &'static str, i64, f64, f64, f64)> {
        self.sync_inc(level_id);
        self.inc_bz
            .as_mut()
            .map(|e| e.take_new_div_rows())
            .unwrap_or_default()
    }

    /// **走势级（level-1 segment）delta 信号接口**——消除调用层每-bar 全量
    /// `current_buysellpoints()` marshal + `_scan_new` 扫描的 B-scaling O(N²)。
    ///
    /// 逐位等价于 Python `m1_i_rust_engine` 每-bar：
    ///   `l1_* = _scan_new(current_buysellpoints(), trend_seen)`
    ///   `type2_buy = (首现 type2 buy in current_buysellpoints(), keyed (seg_idx, move_seg_start))`
    /// 返回 `(l1_buy1, l1_sell1, l1_sell_any, l1_buy_any, type2_buy)`。
    ///
    /// ## O(1) 门控 + delta 原理
    /// `bsp_epoch` 仅在走势级买卖点层**重算**（bsp_key 变化）时自增。epoch 未变 ⟹
    /// `inner.buysellpoints()` 逐位不变 ⟹ 上次已扫，本 bar 无新信号 → 直接返回全 false
    /// （O(1)）。仅 epoch 变化 bar 扫描全量 bsp（共 O(n_seg) 次），seen-set 去重保证
    /// 每个 (kind,side,seg_idx) / (seg_idx,move_seg_start) 一生只触发一次——与 Python
    /// 跨 bar seen-set 语义逐字一致。端到端从 O(B·n_bsp) 降到 O(n_seg·n_bsp)（B≫n_seg）。
    ///
    /// ## 等价性（状态-事件二象性）
    /// Python 每 bar 全量扫 + seen 去重 ⟺ 仅在内容变化 bar 扫 + seen 去重：未变 bar 的
    /// 扫描必然全部命中 seen（无新增）→ 结果恒 false。故门控不改变任何 bar 的信号输出。
    #[pyo3(signature = (level_id = 1))]
    fn trend_new_signals(&mut self, level_id: i64) -> (bool, bool, bool, bool, bool) {
        let _ = level_id; // 走势级 bsp 已由 inner.process_bar 以 self.level_id=1 算妥；保持签名对称。
        let epoch = self.inner.bsp_epoch();
        if epoch == self.trend_last_bsp_epoch {
            return (false, false, false, false, false);
        }
        self.trend_last_bsp_epoch = epoch;

        let (mut b1, mut s1, mut sa, mut ba, mut type2_buy) =
            (false, false, false, false, false);
        for bp in self.inner.buysellpoints() {
            // ── _scan_new 部分：confirmed 必需，键 (kind, side, seg_idx) ──
            if bp.confirmed {
                let key = (bp.kind, bp.side, bp.seg_idx);
                if self.trend_seen.insert(key) {
                    match bp.side {
                        Side::Buy => {
                            ba = true;
                            if bp.kind == BspKind::Type1 {
                                b1 = true;
                            }
                        }
                        Side::Sell => {
                            sa = true;
                            if bp.kind == BspKind::Type1 {
                                s1 = true;
                            }
                        }
                    }
                }
            }
            // ── type2_buy 部分：不门控 confirmed，键 (seg_idx, move_seg_start) ──
            if bp.side == Side::Buy && bp.kind == BspKind::Type2 {
                let key2 = (bp.seg_idx, bp.move_seg_start);
                if self.trend_type2_seen.insert(key2) {
                    type2_buy = true;
                }
            }
        }
        (b1, s1, sa, ba, type2_buy)
    }

    /// **走势级事件流 delta 接口**——marshal 只含新事件（ladder3 残留 O(N²) 项：
    /// 每 bsp_epoch 变化 bar 全量 `current_buysellpoints()` marshal + Python 全量扫描，
    /// CL 2.5M profile 实测两项 ~5.1s 且随 B 二次增长）。
    ///
    /// 返回 (buy1, sell1, sell_any, buy_any, list[event])，event =
    /// (kind, side, seg_idx, confirmed, center_seg_start, center_zd, center_zg, price)。
    /// 逐位等价于调用方 `_scan_events_rust(current_buysellpoints(), trend_seen)`
    /// （organic_signals）：去重键 (kind,side,seg_idx,confirmed)，candidate/confirmed
    /// 分别入流一次；布尔仅由新 confirmed 事件置位（与 `trend_new_signals` 3 元键的
    /// 等价性：confirmed 事件 4 元键新 ⟺ 该 3 元键首次以 confirmed 出现）。
    /// Rust 侧全量扫但只在 epoch 变化 bar 被调用（共 O(n_seg) 次）——与
    /// `trend_new_signals` 同款成本结构 O(n_seg·n_bsp)，消除的是 marshal O(B) +
    /// Python 逐元素扫描。调用契约：仅 bsp_epoch 变化 bar 调用（调用层既有门控）。
    #[allow(clippy::type_complexity)]
    fn take_trend_bsp_events(
        &mut self,
    ) -> (
        bool,
        bool,
        bool,
        bool,
        Vec<(&'static str, &'static str, i64, bool, Option<usize>, f64, f64, f64)>,
    ) {
        let (mut b1, mut s1, mut sa, mut ba) = (false, false, false, false);
        let mut events = Vec::new();
        for bp in self.inner.buysellpoints() {
            let key = (bp.kind, bp.side, bp.seg_idx, bp.confirmed);
            if self.trend_ev_seen.contains(&key) {
                continue;
            }
            self.trend_ev_seen.insert(key);
            events.push((
                bp.kind.as_str(),
                bp.side.as_str(),
                bp.seg_idx,
                bp.confirmed,
                bp.center_seg_start,
                bp.center_zd,
                bp.center_zg,
                bp.price,
            ));
            if !bp.confirmed {
                continue;
            }
            match bp.side {
                Side::Buy => {
                    ba = true;
                    if bp.kind == BspKind::Type1 {
                        b1 = true;
                    }
                }
                Side::Sell => {
                    sa = true;
                    if bp.kind == BspKind::Type1 {
                        s1 = true;
                    }
                }
            }
        }
        (b1, s1, sa, ba, events)
    }

    /// **走势级背驰事件流 delta 接口**——行格式与 `current_trend_divergences` 同构
    /// （(kind, direction, seg_c_end, force_a, force_c, price)），去重键
    /// (kind, direction, seg_c_end) 下沉 Rust，marshal 只含新行。
    /// 有效域同源接口：enable_macd_divergence=false（macd 路径 inc_seg_div 不更新，
    /// 直接 panic——契约违例非数据问题）。调用契约：仅 bsp_epoch 变化 bar 调用。
    fn take_trend_div_events(
        &mut self,
    ) -> Vec<(&'static str, &'static str, i64, f64, f64, f64)> {
        if self.inner.macd_divergence_enabled() {
            // MACD 路径：inc_seg_div 缓存不更新，返回空 Vec
            // BSP 层已用 MACD 面积力竭判定背驰，surfacing 层暂空
            return Vec::new();
        }
        let segs = self.inner.segments();
        let mut out = Vec::new();
        for d in self.inner.trend_divergences() {
            let key = (d.kind, d.direction, d.seg_c_end);
            if self.trend_div_seen.contains(&key) {
                continue;
            }
            self.trend_div_seen.insert(key);
            let idx = d.seg_c_end as usize;
            let price = if idx < segs.len() {
                match d.direction {
                    divergence::DivDir::Top => segs[idx].high,
                    divergence::DivDir::Bottom => segs[idx].low,
                }
            } else {
                0.0
            };
            out.push((
                d.kind.as_str(),
                d.direction.as_str(),
                d.seg_c_end,
                d.force_a,
                d.force_c,
                price,
            ));
        }
        out
    }

    /// 走势层内容变化纪元（O(1)）。调用层缓存上次值，未变 ⟹ `current_moves()` 逐位不变
    /// ⟹ 跳过 marshal（消除 E 引擎每-bar `tuple(current_moves())` 的 B-scaling）。
    fn move_epoch(&self) -> u64 {
        self.inner.move_epoch()
    }

    /// 走势级买卖点内容变化纪元（O(1)）。`trend_new_signals` 内部门控用；亦供调用层直接消费。
    fn bsp_epoch(&self) -> u64 {
        self.inner.bsp_epoch()
    }

    /// 递归层内容变化纪元（O(1)）。未变 ⟹ `current_recursive()` 逐位不变 ⟹ 跳过整个
    /// 递归 marshal + 各层 cache_key 构造 + _scan_new（消除 I 引擎每-bar 递归块的 B-scaling）。
    fn recursive_epoch(&self) -> u64 {
        self.inner.recursive_epoch()
    }

}

impl PyRecursiveOrchestrator {
    /// 同步新增 confirmed 笔到增量引擎（O(Δ)）。三个增量接口共用。
    ///
    /// confirmed 笔为 snapshot 前缀（仅末笔 unconfirmed）；append-only push 新增部分。
    fn sync_inc(&mut self, level_id: i64) {
        let to_push: Vec<(usize, usize, f64, f64, stroke::Direction)> = {
            let strokes = self.inner.strokes();
            let n = strokes.len();
            let confirmed_count = if n > 0 && !strokes[n - 1].confirmed {
                n - 1
            } else {
                n
            };
            strokes[self.inc_pushed.min(confirmed_count)..confirmed_count]
                .iter()
                .map(|s| (s.i0, s.i1, s.high, s.low, s.direction))
                .collect()
        };
        let eng = self
            .inc_bz
            .get_or_insert_with(|| bi_zhongshu_bsp::IncrementalBiZhongshuBsp::new(level_id));
        for (i0, i1, high, low, dir) in to_push {
            eng.push_confirmed_stroke(i0, i1, high, low, dir);
            self.inc_pushed += 1;
        }
    }
}

// ════════════════════════════════════════════════════════════
// 有机赋格 v2 交易层绑定（trading/，M1 磁带注入接口）
// ════════════════════════════════════════════════════════════

/// 信号磁带容器。从列式数组一次性构造（O(N) marshal，每数据集一次），
/// 之后所有变体回测共享，不再跨边界（compute-once）。
#[pyclass(name = "OrganicTape")]
struct PyOrganicTape {
    inner: trading::tape::SignalTape,
}

fn parse_bsp_kind(s: &str) -> PyResult<BspKind> {
    match s {
        "type1" => Ok(BspKind::Type1),
        "type2" => Ok(BspKind::Type2),
        "type3" => Ok(BspKind::Type3),
        _ => Err(pyo3::exceptions::PyValueError::new_err(format!("非法 BSP kind: {s:?}"))),
    }
}

fn parse_side(s: &str) -> PyResult<Side> {
    match s {
        "buy" => Ok(Side::Buy),
        "sell" => Ok(Side::Sell),
        _ => Err(pyo3::exceptions::PyValueError::new_err(format!("非法 side: {s:?}"))),
    }
}

#[pymethods]
impl PyOrganicTape {
    /// 列式构造。bsp_flat 行 =
    ///   (bar, ladder, kind, side, seg_idx, confirmed, cs, zd, zg, price)；
    /// div_flat 行 = (bar, ladder, kind, direction, seg_idx, force_a, force_c, price)
    ///   （side 不传——direction 的纯函数，v1R §2.1）。
    /// 布尔行 = 11 位掩码（Python 侧 sum(1<<k …) 打包）。
    /// D3 行（v2，稀疏）：dir_flips = [(bar, ladder, "up"/"down")]（bar 升序，
    /// 仅方向翻转 bar）；run_high = 密集 n×11 展平（当前信号层不产出 → None）。
    /// fail-fast：close 非有限、cs 存在而 zd/zg 缺失、非法枚举串、越界/乱序索引。
    #[staticmethod]
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    #[pyo3(signature = (closes, buy1, sell1, sell_any, buy_any, up_settled, max_ladder,
                        type2_buy, bsp_flat, div_flat, dir_flips = None, run_high = None,
                        trend_flips = None))]
    fn from_columns(
        closes: Vec<f64>,
        buy1: Vec<u16>,
        sell1: Vec<u16>,
        sell_any: Vec<u16>,
        buy_any: Vec<u16>,
        up_settled: Vec<u16>,
        max_ladder: Vec<u8>,
        type2_buy: Vec<bool>,
        bsp_flat: Vec<(i64, u8, String, String, i64, bool, Option<i64>, Option<f64>, Option<f64>, f64)>,
        div_flat: Vec<(i64, u8, String, String, i64, f64, f64, f64)>,
        dir_flips: Option<Vec<(i64, u8, String)>>,
        run_high: Option<Vec<f64>>,
        trend_flips: Option<Vec<(i64, u8, bool)>>,
    ) -> PyResult<Self> {
        use trading::tape::{BarSig, SignalTape};
        use trading::types::{BspClass, BspEvent as TBspEvent, DivEvent as TDivEvent, LadderMask, MAX_LADDER};
        let n = closes.len();
        for (name, len) in [
            ("buy1", buy1.len()),
            ("sell1", sell1.len()),
            ("sell_any", sell_any.len()),
            ("buy_any", buy_any.len()),
            ("up_settled", up_settled.len()),
            ("max_ladder", max_ladder.len()),
            ("type2_buy", type2_buy.len()),
        ] {
            if len != n {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "列长不一致：closes={n} vs {name}={len}"
                )));
            }
        }
        let mut bars: Vec<BarSig> = (0..n)
            .map(|i| {
                BarSig {
                    close: closes[i],
                    buy1: LadderMask(buy1[i]),
                    sell1: LadderMask(sell1[i]),
                    sell_any: LadderMask(sell_any[i]),
                    buy_any: LadderMask(buy_any[i]),
                    max_ladder: max_ladder[i],
                    type2_buy: type2_buy[i],
                    up_move_settled: LadderMask(up_settled[i]),
                    ..Default::default()
                }
            })
            .collect();
        for (i, &c) in closes.iter().enumerate() {
            if !c.is_finite() {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "close 含非有限值（bar {i}）——NaN 必须在数据清洗期删除（T7 纪律）"
                )));
            }
        }
        for (bar, lad, kind, side, seg_idx, confirmed, cs, zd, zg, price) in bsp_flat {
            let (bar_us, lad_us) = (bar as usize, lad as usize);
            if bar_us >= n || lad_us >= MAX_LADDER {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "bsp 事件越界：bar={bar} ladder={lad}"
                )));
            }
            if cs.is_some() && (zd.is_none() || zg.is_none()) {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "bsp 事件 cs 存在而 zd/zg 缺失（bar {bar}）——CenterBook 算术前提"
                )));
            }
            let class = BspClass::from_parts(parse_bsp_kind(&kind)?, parse_side(&side)?);
            bars[bar_us]
                .bsp_events
                .get_or_insert_with(|| Box::new(<[Vec<TBspEvent>; MAX_LADDER]>::default()))
                [lad_us]
                .push(TBspEvent { class, seg_idx, confirmed, cs, zd, zg, price });
        }
        for (bar, lad, kind, direction, seg_idx, force_a, force_c, price) in div_flat {
            let (bar_us, lad_us) = (bar as usize, lad as usize);
            if bar_us >= n || lad_us >= MAX_LADDER {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "div 事件越界：bar={bar} ladder={lad}"
                )));
            }
            let dkind = match kind.as_str() {
                "trend" => divergence::DivKind::Trend,
                "consolidation" => divergence::DivKind::Consolidation,
                _ => {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "非法 div kind: {kind:?}"
                    )))
                }
            };
            let dir = match direction.as_str() {
                "up" => Direction::Up,
                "down" => Direction::Down,
                _ => {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "非法 div direction: {direction:?}"
                    )))
                }
            };
            bars[bar_us]
                .div_events
                .get_or_insert_with(|| Box::new(<[Vec<TDivEvent>; MAX_LADDER]>::default()))
                [lad_us]
                .push(TDivEvent { kind: dkind, direction: dir, seg_idx, force_a, force_c, price });
        }
        let dir_flips_parsed = match dir_flips {
            None => None,
            Some(rows) => {
                let mut out: Vec<(i64, u8, Direction)> = Vec::with_capacity(rows.len());
                let mut last_bar = -1i64;
                for (bar, lad, dir) in rows {
                    if bar < last_bar || (bar as usize) >= n || (lad as usize) >= MAX_LADDER {
                        return Err(pyo3::exceptions::PyValueError::new_err(format!(
                            "dir_flips 越界或乱序：bar={bar} ladder={lad}（须 bar 升序）"
                        )));
                    }
                    last_bar = bar;
                    let d = match dir.as_str() {
                        "up" => Direction::Up,
                        "down" => Direction::Down,
                        _ => {
                            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                                "非法 dir_flips 方向: {dir:?}"
                            )))
                        }
                    };
                    out.push((bar, lad, d));
                }
                Some(out)
            }
        };
        if let Some(rh) = &run_high {
            if rh.len() != n * MAX_LADDER {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "run_high 长度须为 n×{MAX_LADDER}：{} vs {}",
                    rh.len(),
                    n * MAX_LADDER
                )));
            }
        }
        // 趋势态行（38课循环 voice）：(bar, ladder, is_trend)，bar 升序校验同
        // dir_flips（消费方 runner 单指针滚动推进的前提）。
        if let Some(rows) = &trend_flips {
            let mut last_bar = -1i64;
            for &(bar, lad, _) in rows {
                if bar < last_bar || (bar as usize) >= n || (lad as usize) >= MAX_LADDER {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "trend_flips 越界或乱序：bar={bar} ladder={lad}（须 bar 升序）"
                    )));
                }
                last_bar = bar;
            }
        }
        Ok(PyOrganicTape {
            inner: SignalTape { bars, dir_flips: dir_flips_parsed, run_high, trend_flips },
        })
    }

    fn n_bars(&self) -> usize {
        self.inner.bars.len()
    }
}

/// 单变体回测。返回 dict（trades/counters/归因表/diag）。
/// variant ∈ {V0|O0, V1f, V1r, V2, V3, V3p, V4, VS}（v2 §8.1 矩阵）。
#[pyfunction]
#[pyo3(signature = (tape, variant, floor_ladder = 2, stop_mode = "none", diag = false))]
fn run_organic_rust(
    py: Python<'_>,
    tape: &PyOrganicTape,
    variant: &str,
    floor_ladder: usize,
    stop_mode: &str,
    diag: bool,
) -> PyResult<PyObject> {
    use pyo3::types::PyDict;
    let cfg = trading::config::variant(variant).ok_or_else(|| {
        pyo3::exceptions::PyValueError::new_err(format!("未知变体: {variant:?}"))
    })?;
    let sm = trading::config::StopMode::parse(stop_mode).ok_or_else(|| {
        pyo3::exceptions::PyValueError::new_err(format!("非法 stop_mode: {stop_mode:?}"))
    })?;
    let res = trading::runner::run_organic(&tape.inner, floor_ladder, &cfg, sm, diag)
        .map_err(pyo3::exceptions::PyValueError::new_err)?;

    let out = PyDict::new(py);
    let trades: Vec<(i64, f64, i64, f64, f64, String, u32, f64)> = res
        .trades
        .iter()
        .map(|t| {
            (
                t.entry_bar,
                t.entry_price,
                t.exit_bar,
                t.exit_price,
                t.pnl_pct,
                t.exit_reason.clone(),
                t.n_short_diffs,
                t.cost_basis_at_exit,
            )
        })
        .collect();
    out.set_item("trades", trades)?;
    let counters = PyDict::new(py);
    for (k, v) in res.counters.py_items() {
        counters.set_item(k, v)?;
    }
    counters.set_item(
        "fatigue_open_bars_by_ladder",
        res.counters.fatigue_open_bars_by_ladder.to_vec(),
    )?;
    // rev_paired 按开腿类型净现金（f64——py_items 仅承载 u64）
    counters.set_item("rev_osc_net_cash", res.counters.rev_osc_cash)?;
    counters.set_item("rev_esc_net_cash", res.counters.rev_esc_cash)?;
    // 递归子腿聚合净现金（O_sub1 判据直接读数）
    counters.set_item("sub_net_cash", res.counters.sub_cash)?;
    counters.set_item("c38_net_cash", res.counters.c38_cash)?;
    // 循环短差逐腿 (reason, profit) 日志——按买点类型的 payoff 分布读数
    counters.set_item("c38_close_profits", res.counters.c38_close_profits.clone())?;
    // 41课域腿门拒开逐事件 (ladder, bar) 日志——regime 分段统计数据基础
    counters.set_item("osc_l41_reject_log", res.counters.osc_l41_reject_log.clone())?;
    // osc 操作域域外不开逐事件 (ladder, bar) 日志（osc_domain=ConsolidationOnly）
    counters.set_item(
        "osc_domain_reject_log",
        res.counters.osc_domain_reject_log.clone(),
    )?;
    // H1 candidate 冻结拒开逐事件 (ladder, bar) 日志（osc_candidate_freeze）
    counters.set_item("osc_cf_reject_log", res.counters.osc_cf_reject_log.clone())?;
    // H2 力度收敛门拒开逐事件 (ladder, bar, reason 0=新生/1=扩张) 日志
    // （osc_strength_gate——预注册判据1 时序靶的数据基础）
    counters.set_item("osc_sg_reject_log", res.counters.osc_sg_reject_log.clone())?;
    // H4 滚动振幅准入拒开逐事件 (ladder, bar, reason 0=noref/1=振幅不足) 日志
    // （osc_amp_gate——时序靶探针与 G3 双拒因分离的数据基础）
    counters.set_item(
        "osc_amp_reject_log",
        res.counters.osc_amp_reject_log.clone(),
    )?;
    // H3 级别上移开腿逐事件 (槽层 ladder, bar) 日志（osc_domain=TrendUpshift
    // ——重路由 vs 删除裁决的时序探针数据基础）
    counters.set_item(
        "osc_upshift_open_log",
        res.counters.osc_upshift_open_log.clone(),
    )?;
    out.set_item("counters", counters)?;
    out.set_item("rev_attempts_by_ladder", res.rev_attempts_by_ladder.to_vec())?;
    out.set_item("rev_opens_by_ladder", res.rev_opens_by_ladder.to_vec())?;
    out.set_item("ladder_attribution", res.ladder_attribution.to_vec())?;
    out.set_item("ladder_held_bars", res.ladder_held_bars.to_vec())?;
    out.set_item("leg_contribution", res.leg_contribution.clone())?;
    out.set_item("n_addon", res.n_addon)?;
    out.set_item("n_core_stops", res.n_core_stops)?;
    // master 递归建仓成交日志（entry_mode=Full 恒空表——在册对账面零侵入）。
    out.set_item("rec_fills", res.rec_fills.clone())?;
    // 锚中枢相对振幅调研日志（θ 自适应任务；diag=false 时为空表）。
    out.set_item("center_amp_log", res.center_amp_log.clone())?;
    match &res.diag {
        None => out.set_item("diag", py.None())?,
        Some(diags) => {
            // 头部 10 元组 + trace 行（6 元组, 7 元组）嵌套（PyO3 元组 ≤12 限制）
            #[allow(clippy::type_complexity)]
            let rows: Vec<(
                (i64, f64, i64, f64, String, usize, f64, f64, f64, bool),
                Vec<((i64, &'static str, i64, f64, i64, f64), (f64, f64, f64, bool, f64, f64, f64))>,
            )> = diags
                .iter()
                .map(|d| {
                    let header = (
                        d.entry_bar,
                        d.entry_price,
                        d.exit_bar,
                        d.exit_price,
                        d.exit_reason.clone(),
                        d.entry_ladder,
                        d.pnl_pct,
                        d.cost_basis_exit,
                        d.total_shares_exit,
                        d.reached_earning,
                    );
                    let diffs = d
                        .diffs
                        .iter()
                        .map(|r| {
                            (
                                (
                                    r.slot.py_key(),
                                    // H3 上移腿投影为 "osc_up"（回测按腿分解上移腿
                                    // 盈亏）；非 H3 变体 upshift 恒 false ⇒ 逐位不变
                                    if r.upshift { "osc_up" } else { r.slot.leg_kind() },
                                    r.sell_bar,
                                    r.sell_price,
                                    r.buy_bar,
                                    r.buy_price,
                                ),
                                (
                                    r.shares,
                                    r.diff,
                                    r.profit,
                                    r.was_earning,
                                    r.shares_delta,
                                    r.cost_basis_before,
                                    r.cost_basis_after,
                                ),
                            )
                        })
                        .collect();
                    (header, diffs)
                })
                .collect();
            out.set_item("diag", rows)?;
        }
    }
    Ok(out.into())
}

/// 递归建仓回测（RecursivePosition，2026-06-11 任务）。独立入口而非
/// OrganicConfig 字段——递归建仓与 OrganicLedger 是不同账本范畴（从零建仓
/// vs 满仓降成本），塞配置位即声明膨胀；run_organic 路径零接触（O0≡P5
/// 守卫自动满足）。weight ∈ {"exp2", "linear"}。
#[pyfunction]
#[pyo3(signature = (tape, floor_ladder = 2, base_frac = 0.1, weight = "exp2", sell_t1_only = false, with_fills = false))]
fn run_recursive_rust(
    py: Python<'_>,
    tape: &PyOrganicTape,
    floor_ladder: usize,
    base_frac: f64,
    weight: &str,
    sell_t1_only: bool,
    with_fills: bool,
) -> PyResult<PyObject> {
    use pyo3::types::PyDict;
    use trading::recursive_position::{run_recursive, WeightFn};
    let wf = WeightFn::parse(weight).ok_or_else(|| {
        pyo3::exceptions::PyValueError::new_err(format!("非法 weight: {weight:?}"))
    })?;
    let res = run_recursive(&tape.inner, floor_ladder, base_frac, wf, sell_t1_only, with_fills)
        .map_err(pyo3::exceptions::PyValueError::new_err)?;
    let out = PyDict::new(py);
    out.set_item("final_return_pct", res.final_return_pct)?;
    out.set_item("max_dd_pct", res.max_dd_pct)?;
    out.set_item("avg_invested_frac", res.avg_invested_frac)?;
    out.set_item("final_invested_frac", res.final_invested_frac)?;
    out.set_item("turnover", res.turnover)?;
    out.set_item("buy_fills", res.buy_fills.to_vec())?;
    out.set_item("buy_noops", res.buy_noops.to_vec())?;
    out.set_item("buy_starved", res.buy_starved.to_vec())?;
    out.set_item("sell_clears", res.sell_clears.to_vec())?;
    out.set_item("sell_noops", res.sell_noops.to_vec())?;
    let fills: Vec<(i64, u8, u8, f64, f64, f64, f64)> = res
        .fills
        .iter()
        .map(|f| (f.bar, f.ladder, f.side, f.shares, f.price, f.cash_after, f.equity_after))
        .collect();
    out.set_item("fills", fills)?;
    Ok(out.into())
}

/// 多级别仓位分层回测（positional fugue，2026-06-12 任务）。独立入口——
/// 与单体 master（run_organic）是不同仓位范畴（N 个独立 45课 FSM 各拿
/// 涌现配额 vs 一个 FSM 拿全仓）；run_organic 路径零接触（O0≡P5 守卫
/// 自动满足）。设计：`analysis/positional_fugue_design.md`。
/// mode ∈ {"cycle45"（v1 每层独立45课循环，否证基线）,
///         "hold26"（26课恒仓：任意卖点削减/任意买点回复）,
///         "hold26_t1"（卖点词汇收窄为 type1）,
///         "fusion"/"fusion_t"/"fusion_s"/"fusion_e"/"fusion_se"/"fusion_t{gdb}"
///         （B+C 合体家族）,
///         "fusion_u"（统一配置 U：fusion_t 基座 + 相位递归路由 osc 层，
///          2026-06-12 任务）, "fusion_uw"（U−③ 强震荡门消融臂，预注册 P5）}。
#[pyfunction]
#[pyo3(signature = (tape, floor_ladder = 2, mode = "hold26"))]
fn run_positional_rust(
    py: Python<'_>,
    tape: &PyOrganicTape,
    floor_ladder: usize,
    mode: &str,
) -> PyResult<PyObject> {
    use trading::positional::{run_positional, PolarityMode};
    let pm = PolarityMode::parse(mode).ok_or_else(|| {
        pyo3::exceptions::PyValueError::new_err(format!("非法 mode: {mode:?}"))
    })?;
    let res = run_positional(&tape.inner, floor_ladder, pm)
        .map_err(pyo3::exceptions::PyValueError::new_err)?;
    Ok(positional_result_to_dict(py, &res)?.into())
}

/// `PositionalResult` → PyDict（`run_positional_rust` + `UnnStream.finish` 复用，
/// 保证流式 finish 与批量同结构 ⇒ Python 端逐键 bit-exact 比对成立）。
fn positional_result_to_dict<'py>(
    py: Python<'py>,
    res: &trading::positional::PositionalResult,
) -> PyResult<pyo3::Bound<'py, pyo3::types::PyDict>> {
    use pyo3::types::PyDict;
    let out = PyDict::new(py);
    // trade 行 = (ladder, entry_bar, entry_price, exit_bar, exit_price,
    //             shares, weight_at_entry, deferred_bars, partial, exit_reason,
    //             polarity)。polarity ∈ {"long","short"}：Short 行现金流镜像
    //             （开空收 proceeds / 平空付买回款），NAV 重建按此分派符号。
    let trades: Vec<(u8, i64, f64, i64, f64, f64, f64, i64, bool, &'static str, &'static str)> =
        res.trades
            .iter()
            .map(|t| {
                (
                    t.ladder,
                    t.entry_bar,
                    t.entry_price,
                    t.exit_bar,
                    t.exit_price,
                    t.shares,
                    t.weight_at_entry,
                    t.deferred_bars,
                    t.partial,
                    t.exit_reason,
                    t.polarity.as_str(),
                )
            })
            .collect();
    out.set_item("trades", trades)?;
    out.set_item("equity", res.equity.clone())?;
    out.set_item("final_nav", res.final_nav)?;
    out.set_item("n_entries_by_ladder", res.n_entries_by_ladder.to_vec())?;
    out.set_item("n_exits_by_ladder", res.n_exits_by_ladder.to_vec())?;
    out.set_item("held_bars_by_ladder", res.held_bars_by_ladder.to_vec())?;
    out.set_item("n_disarms_by_ladder", res.n_disarms_by_ladder.to_vec())?;
    out.set_item(
        "n_pending_cancels_by_ladder",
        res.n_pending_cancels_by_ladder.to_vec(),
    )?;
    out.set_item("n_noref_skips_by_ladder", res.n_noref_skips_by_ladder.to_vec())?;
    out.set_item("n_partial_by_ladder", res.n_partial_by_ladder.to_vec())?;
    out.set_item("n_deferred_by_ladder", res.n_deferred_by_ladder.to_vec())?;
    out.set_item(
        "n_trend_div_exits_by_ladder",
        res.n_trend_div_exits_by_ladder.to_vec(),
    )?;
    out.set_item(
        "n_gate41_blocks_by_ladder",
        res.n_gate41_blocks_by_ladder.to_vec(),
    )?;
    // Fusion（B+C 合体）观测面（legacy 模式恒零）
    out.set_item("n_trend_holds_by_ladder", res.n_trend_holds_by_ladder.to_vec())?;
    out.set_item("n_sub_opens_by_ladder", res.n_sub_opens_by_ladder.to_vec())?;
    out.set_item("n_sub_restores_by_ladder", res.n_sub_restores_by_ladder.to_vec())?;
    out.set_item(
        "n_sub_kbuy_restores_by_ladder",
        res.n_sub_kbuy_restores_by_ladder.to_vec(),
    )?;
    out.set_item("n_sub_escalates_by_ladder", res.n_sub_escalates_by_ladder.to_vec())?;
    out.set_item(
        "n_sub_phase_closes_by_ladder",
        res.n_sub_phase_closes_by_ladder.to_vec(),
    )?;
    out.set_item(
        "n_sub_cost_rejects_by_ladder",
        res.n_sub_cost_rejects_by_ladder.to_vec(),
    )?;
    out.set_item(
        "n_sub_noref_rejects_by_ladder",
        res.n_sub_noref_rejects_by_ladder.to_vec(),
    )?;
    out.set_item("n_sub_restore_defer_bars", res.n_sub_restore_defer_bars)?;
    out.set_item(
        "n_sub_pool_topup_by_ladder",
        res.n_sub_pool_topup_by_ladder.to_vec(),
    )?;
    out.set_item("sub_net_cash_by_ladder", res.sub_net_cash_by_ladder.to_vec())?;
    // 统一配置 U（fusion_u）观测面（P3 机制可观测性；其余模式恒零）
    out.set_item("n_osc_opens_by_ladder", res.n_osc_opens_by_ladder.to_vec())?;
    out.set_item(
        "n_osc_upshift_opens_by_ladder",
        res.n_osc_upshift_opens_by_ladder.to_vec(),
    )?;
    out.set_item("n_osc_open_at_level", res.n_osc_open_at_level.to_vec())?;
    out.set_item("n_osc_out_bars_by_ladder", res.n_osc_out_bars_by_ladder.to_vec())?;
    out.set_item(
        "n_osc_up_out_bars_by_ladder",
        res.n_osc_up_out_bars_by_ladder.to_vec(),
    )?;
    out.set_item("n_route_phase_skips", res.n_route_phase_skips.to_vec())?;
    out.set_item("n_route_no_center", res.n_route_no_center.to_vec())?;
    out.set_item("n_route_amp_rejects", res.n_route_amp_rejects.to_vec())?;
    out.set_item("n_route_amp_noref", res.n_route_amp_noref.to_vec())?;
    out.set_item("n_route_weak_rejects", res.n_route_weak_rejects.to_vec())?;
    out.set_item("n_route_weak_noref", res.n_route_weak_noref.to_vec())?;
    // H1 candidate 冻结 + Sequence38 子腿观测面（全量普适组合 2026-06-12）
    out.set_item("n_route_h1_freezes", res.n_route_h1_freezes.to_vec())?;
    out.set_item("n_seq38_opens_by_ladder", res.n_seq38_opens_by_ladder.to_vec())?;
    out.set_item(
        "n_seq38_consbuy_closes_by_ladder",
        res.n_seq38_consbuy_closes_by_ladder.to_vec(),
    )?;
    out.set_item(
        "n_seq38_nobreak_closes_by_ladder",
        res.n_seq38_nobreak_closes_by_ladder.to_vec(),
    )?;
    out.set_item(
        "n_seq38_newdiv_closes_by_ladder",
        res.n_seq38_newdiv_closes_by_ladder.to_vec(),
    )?;
    out.set_item(
        "n_route_selected_by_level",
        res.n_route_selected_by_level.to_vec(),
    )?;
    out.set_item("n_route_exhausted", res.n_route_exhausted)?;
    out.set_item(
        "n_osc_zd_restores_by_ladder",
        res.n_osc_zd_restores_by_ladder.to_vec(),
    )?;
    out.set_item(
        "n_osc_death_restores_by_ladder",
        res.n_osc_death_restores_by_ladder.to_vec(),
    )?;
    out.set_item(
        "n_osc_shift_restores_by_ladder",
        res.n_osc_shift_restores_by_ladder.to_vec(),
    )?;
    out.set_item(
        "n_osc_kbuy_restores_by_ladder",
        res.n_osc_kbuy_restores_by_ladder.to_vec(),
    )?;
    out.set_item(
        "n_osc_phase_restores_by_ladder",
        res.n_osc_phase_restores_by_ladder.to_vec(),
    )?;
    out.set_item(
        "n_osc_due_restores_by_ladder",
        res.n_osc_due_restores_by_ladder.to_vec(),
    )?;
    out.set_item("n_osc_escalates_by_ladder", res.n_osc_escalates_by_ladder.to_vec())?;
    out.set_item(
        "n_osc_trend_hold_sells_by_ladder",
        res.n_osc_trend_hold_sells_by_ladder.to_vec(),
    )?;
    out.set_item(
        "n_osc_sell3_vetos_by_ladder",
        res.n_osc_sell3_vetos_by_ladder.to_vec(),
    )?;
    out.set_item("n_osc_restore_defer_bars", res.n_osc_restore_defer_bars)?;
    out.set_item("osc_net_cash_by_ladder", res.osc_net_cash_by_ladder.to_vec())?;
    out.set_item("osc_net_cash_at_level", res.osc_net_cash_at_level.to_vec())?;
    // ── P6 相位机观测面（fusion_p/fusion_pu；其余模式恒零）──
    out.set_item(
        "n_phase_up_opens_by_ladder",
        res.n_phase_up_opens_by_ladder.to_vec(),
    )?;
    out.set_item(
        "n_phase_up_settle_closes_by_ladder",
        res.n_phase_up_settle_closes_by_ladder.to_vec(),
    )?;
    out.set_item(
        "n_phase_up_div_closes_by_ladder",
        res.n_phase_up_div_closes_by_ladder.to_vec(),
    )?;
    out.set_item(
        "n_phase_up_dir_closes_by_ladder",
        res.n_phase_up_dir_closes_by_ladder.to_vec(),
    )?;
    out.set_item(
        "n_phase_dn_opens_by_ladder",
        res.n_phase_dn_opens_by_ladder.to_vec(),
    )?;
    out.set_item(
        "n_phase_dn_closes_by_ladder",
        res.n_phase_dn_closes_by_ladder.to_vec(),
    )?;
    out.set_item("phase_up_bars_by_ladder", res.phase_up_bars_by_ladder.to_vec())?;
    out.set_item("phase_dn_bars_by_ladder", res.phase_dn_bars_by_ladder.to_vec())?;
    out.set_item(
        "n_r2_pos_blocks_by_ladder",
        res.n_r2_pos_blocks_by_ladder.to_vec(),
    )?;
    // anc 祖先趋势豁免（hold26_anc/fusion_ta）观测面（slow_bull P7；其余恒零）
    out.set_item(
        "n_anc_exempt_blocks_by_ladder",
        res.n_anc_exempt_blocks_by_ladder.to_vec(),
    )?;
    out.set_item("anc_up_bars_by_ladder", res.anc_up_bars_by_ladder.to_vec())?;
    // 区间套正向定位（fusion_tn/fusion_trn）观测面（其余模式恒零）
    out.set_item("n_nest_arms_by_ladder", res.n_nest_arms_by_ladder.to_vec())?;
    out.set_item(
        "n_nest_fire_sell_by_ladder",
        res.n_nest_fire_sell_by_ladder.to_vec(),
    )?;
    out.set_item(
        "n_nest_fire_buy_by_ladder",
        res.n_nest_fire_buy_by_ladder.to_vec(),
    )?;
    out.set_item("n_nest_breaks_by_ladder", res.n_nest_breaks_by_ladder.to_vec())?;
    out.set_item("nest_lead_bars_sum", res.nest_lead_bars_sum)?;
    out.set_item("nest_lead_n", res.nest_lead_n)?;
    // 统一递归 voice FSM（fusion_v）观测面（其余模式恒零）
    out.set_item("freeze_up_bars_by_ladder", res.freeze_up_bars_by_ladder.to_vec())?;
    out.set_item("freeze_dn_bars_by_ladder", res.freeze_dn_bars_by_ladder.to_vec())?;
    out.set_item("n_t2w_arms_by_ladder", res.n_t2w_arms_by_ladder.to_vec())?;
    out.set_item("n_t2w_fires_by_ladder", res.n_t2w_fires_by_ladder.to_vec())?;
    out.set_item("n_t2w_negates_by_ladder", res.n_t2w_negates_by_ladder.to_vec())?;
    // 双书独立逐仓 voice（fusion_vd/vdn）观测面（其余模式恒零）
    out.set_item(
        "n_dual_short_opens_by_ladder",
        res.n_dual_short_opens_by_ladder.to_vec(),
    )?;
    out.set_item(
        "n_dual_short_open_blocks_by_ladder",
        res.n_dual_short_open_blocks_by_ladder.to_vec(),
    )?;
    out.set_item(
        "dual_both_held_bars_by_ladder",
        res.dual_both_held_bars_by_ladder.to_vec(),
    )?;
    // 双向条件轴 S1-S4（fusion_btr/fusion_btra）观测面（其余模式恒零）
    out.set_item("n_flip_shorts_by_ladder", res.n_flip_shorts_by_ladder.to_vec())?;
    out.set_item("n_short_covers_by_ladder", res.n_short_covers_by_ladder.to_vec())?;
    out.set_item(
        "n_short_moveup_covers_by_ladder",
        res.n_short_moveup_covers_by_ladder.to_vec(),
    )?;
    out.set_item(
        "n_short_trend_holds_by_ladder",
        res.n_short_trend_holds_by_ladder.to_vec(),
    )?;
    out.set_item(
        "n_short_r2_blocks_by_ladder",
        res.n_short_r2_blocks_by_ladder.to_vec(),
    )?;
    out.set_item(
        "n_short_anc_rejects_by_ladder",
        res.n_short_anc_rejects_by_ladder.to_vec(),
    )?;
    out.set_item(
        "n_short_liquidations_by_ladder",
        res.n_short_liquidations_by_ladder.to_vec(),
    )?;
    out.set_item(
        "short_held_bars_by_ladder",
        res.short_held_bars_by_ladder.to_vec(),
    )?;
    out.set_item(
        "short_net_cash_by_ladder",
        res.short_net_cash_by_ladder.to_vec(),
    )?;
    // 纯回复门消融臂（fusion_btrg）观测面（其余模式恒零）
    out.set_item("n_gate_enters_by_ladder", res.n_gate_enters_by_ladder.to_vec())?;
    out.set_item(
        "n_gate_restores_by_ladder",
        res.n_gate_restores_by_ladder.to_vec(),
    )?;
    out.set_item(
        "n_gate_moveup_restores_by_ladder",
        res.n_gate_moveup_restores_by_ladder.to_vec(),
    )?;
    out.set_item(
        "gate_held_bars_by_ladder",
        res.gate_held_bars_by_ladder.to_vec(),
    )?;
    // 嵌套递归赋格（nrf）观测面（其余模式恒零）
    out.set_item(
        "n_nrf_root_entries_by_ladder",
        res.n_nrf_root_entries_by_ladder.to_vec(),
    )?;
    out.set_item("n_nrf_spawns_by_ladder", res.n_nrf_spawns_by_ladder.to_vec())?;
    out.set_item(
        "n_nrf_negate_closes_by_ladder",
        res.n_nrf_negate_closes_by_ladder.to_vec(),
    )?;
    out.set_item(
        "n_nrf_cascade_closes_by_ladder",
        res.n_nrf_cascade_closes_by_ladder.to_vec(),
    )?;
    out.set_item(
        "n_nrf_cost_rejects_by_ladder",
        res.n_nrf_cost_rejects_by_ladder.to_vec(),
    )?;
    out.set_item(
        "n_nrf_noref_rejects_by_ladder",
        res.n_nrf_noref_rejects_by_ladder.to_vec(),
    )?;
    out.set_item(
        "n_nrf_floor_stops_by_ladder",
        res.n_nrf_floor_stops_by_ladder.to_vec(),
    )?;
    out.set_item("n_nrf_flips_by_ladder", res.n_nrf_flips_by_ladder.to_vec())?;
    out.set_item(
        "n_nrf_root_flips_by_ladder",
        res.n_nrf_root_flips_by_ladder.to_vec(),
    )?;
    out.set_item(
        "n_nrf_deep_fires_by_ladder",
        res.n_nrf_deep_fires_by_ladder.to_vec(),
    )?;
    out.set_item("nrf_depth_bars", res.nrf_depth_bars.to_vec())?;
    out.set_item("nrf_phys_long_bars", res.nrf_phys_long_bars)?;
    out.set_item("nrf_phys_short_bars", res.nrf_phys_short_bars)?;
    out.set_item(
        "n_nrf_earning_adds_by_ladder",
        res.n_nrf_earning_adds_by_ladder.to_vec(),
    )?;
    out.set_item("nrf_earning_units", res.nrf_earning_units)?;
    out.set_item("nrf_short_earning_hits", res.nrf_short_earning_hits)?;
    out.set_item("nrf_shrink_units", res.nrf_shrink_units)?;
    out.set_item("nrf_max_children", res.nrf_max_children)?;
    // unn 螺旋 eod ~观测（结构化输出，引擎纯函数；其余模式恒 0）——
    // formalization-validity-domain.md：~观测须可被下游 L2/L3 结构化消费而非 eprintln 丢弃。
    out.set_item("nrf_t50_monotone_violations", res.nrf_t50_monotone_violations)?;
    out.set_item("nrf_t56_radial_coverage", res.nrf_t56_radial_coverage)?;
    out.set_item("nrf_t57_onesided_layers", res.nrf_t57_onesided_layers)?;
    out.set_item("nrf_t58_active_levels", res.nrf_t58_active_levels)?;
    out.set_item("nrf_t59_degenerate_layers", res.nrf_t59_degenerate_layers)?;
    out.set_item("sig_n_ops", res.sig_n_ops)?;
    out.set_item("sig_n_s9_violations", res.sig_n_s9_violations)?;
    out.set_item("sig_n_trend_obs", res.sig_n_trend_obs)?;
    out.set_item("sig_n_trend_decel", res.sig_n_trend_decel)?;
    Ok(out)
}

/// 流式统一必然性引擎（mode="unn"；535号边界B：push_bar 收 BarSig）。
///
/// 与批量 `run_positional_rust(mode="unn")` **共享 `UnnStreamCore::step/finish`** ⇒
/// 逐 bar 累积的 `finish()` 结果与批量逐位等价（bit-exact 由构造保证，非对齐努力）。
/// 用法（NautilusTrader on_bar 逐 bar 驱动）：
///   s = UnnStream(floor_ladder=2)
///   for bar: signals = s.push_bar(close, buy1, ..., bsp_rows, div_rows, flip_rows)
///   result = s.finish()   # 与 run_positional_rust 同结构 dict
#[pyclass(name = "UnnStream")]
struct PyUnnStream {
    core: trading::unified_necessity::UnnStreamCore,
}

#[pymethods]
impl PyUnnStream {
    #[new]
    #[pyo3(signature = (floor_ladder = 2))]
    fn new(floor_ladder: usize) -> PyResult<Self> {
        let core = trading::unified_necessity::UnnStreamCore::new(floor_ladder)
            .map_err(pyo3::exceptions::PyValueError::new_err)?;
        Ok(Self { core })
    }

    /// 逐 bar 推送。事件行无 bar 字段（本 bar 内）：
    ///   bsp_rows 行 = (ladder, kind, side, seg_idx, confirmed, cs, zd, zg, price)；
    ///   div_rows 行 = (ladder, kind, direction, seg_idx, force_a, force_c, price)；
    ///   flip_rows 行 = (ladder, "up"/"down")（本 bar 方向翻转沿，a0 端证据）。
    /// 布尔 = 11 位掩码（Python `sum(1<<k …)`）。返回本 bar **新增** trade 行
    /// （与 `run_positional_rust` trades 同 11 元组格式；entry 时空、exit/翻转时配对）。
    /// fail-fast 与 `from_columns` 同口径（close 非有限 / cs 缺 zd-zg / 非法枚举 / 越界）。
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    #[pyo3(signature = (close, buy1, sell1, sell_any, buy_any, up_settled, max_ladder,
                        type2_buy, bsp_rows, div_rows, flip_rows))]
    fn push_bar(
        &mut self,
        close: f64,
        buy1: u16,
        sell1: u16,
        sell_any: u16,
        buy_any: u16,
        up_settled: u16,
        max_ladder: u8,
        type2_buy: bool,
        bsp_rows: Vec<(u8, String, String, i64, bool, Option<i64>, Option<f64>, Option<f64>, f64)>,
        div_rows: Vec<(u8, String, String, i64, f64, f64, f64)>,
        flip_rows: Vec<(u8, String)>,
    ) -> PyResult<Vec<(u8, i64, f64, i64, f64, f64, f64, i64, bool, &'static str, &'static str)>> {
        use trading::tape::BarSig;
        use trading::types::{BspClass, BspEvent as TBspEvent, DivEvent as TDivEvent, LadderMask, MAX_LADDER};
        if !close.is_finite() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "close 含非有限值——NaN 必须在数据清洗期删除（T7 纪律）",
            ));
        }
        let mut sig = BarSig {
            close,
            buy1: LadderMask(buy1),
            sell1: LadderMask(sell1),
            sell_any: LadderMask(sell_any),
            buy_any: LadderMask(buy_any),
            max_ladder,
            type2_buy,
            up_move_settled: LadderMask(up_settled),
            ..Default::default()
        };
        for (lad, kind, side, seg_idx, confirmed, cs, zd, zg, price) in bsp_rows {
            let lad_us = lad as usize;
            if lad_us >= MAX_LADDER {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "bsp 事件 ladder 越界: {lad}"
                )));
            }
            if cs.is_some() && (zd.is_none() || zg.is_none()) {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "bsp 事件 cs 存在而 zd/zg 缺失（ladder {lad}）——CenterBook 算术前提"
                )));
            }
            let class = BspClass::from_parts(parse_bsp_kind(&kind)?, parse_side(&side)?);
            sig.bsp_events
                .get_or_insert_with(|| Box::new(<[Vec<TBspEvent>; MAX_LADDER]>::default()))
                [lad_us]
                .push(TBspEvent { class, seg_idx, confirmed, cs, zd, zg, price });
        }
        for (lad, kind, direction, seg_idx, force_a, force_c, price) in div_rows {
            let lad_us = lad as usize;
            if lad_us >= MAX_LADDER {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "div 事件 ladder 越界: {lad}"
                )));
            }
            let dkind = match kind.as_str() {
                "trend" => divergence::DivKind::Trend,
                "consolidation" => divergence::DivKind::Consolidation,
                _ => {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "非法 div kind: {kind:?}"
                    )))
                }
            };
            let dir = match direction.as_str() {
                "up" => Direction::Up,
                "down" => Direction::Down,
                _ => {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "非法 div direction: {direction:?}"
                    )))
                }
            };
            sig.div_events
                .get_or_insert_with(|| Box::new(<[Vec<TDivEvent>; MAX_LADDER]>::default()))
                [lad_us]
                .push(TDivEvent { kind: dkind, direction: dir, seg_idx, force_a, force_c, price });
        }
        // flip_edge（本 bar）：从 flip_rows 构造（批量从 flips 数组按 bar 切，语义同）。
        let mut flip_edge: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];
        for (lad, dir) in flip_rows {
            let lad_us = lad as usize;
            if lad_us >= MAX_LADDER {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "flip 行 ladder 越界: {lad}"
                )));
            }
            let d = match dir.as_str() {
                "up" => Direction::Up,
                "down" => Direction::Down,
                _ => {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "非法 flip 方向: {dir:?}"
                    )))
                }
            };
            flip_edge[lad_us] = Some(d);
        }
        let before = self.core.n_trades();
        self.core.step(&sig, &flip_edge);
        let new_trades = self.core.trades()[before..]
            .iter()
            .map(|t| {
                (
                    t.ladder,
                    t.entry_bar,
                    t.entry_price,
                    t.exit_bar,
                    t.exit_price,
                    t.shares,
                    t.weight_at_entry,
                    t.deferred_bars,
                    t.partial,
                    t.exit_reason,
                    t.polarity.as_str(),
                )
            })
            .collect();
        Ok(new_trades)
    }

    /// 状态快照: (cur_bar, nav, long_units, short_units, n_active_voices)。
    fn snapshot(&self) -> (i64, f64, f64, f64, usize) {
        self.core.snapshot()
    }

    /// 收尾（eod cascade 关根 + N4 反证）并返回完整结果 dict（与
    /// `run_positional_rust(mode="unn")` 同结构）。幂等。
    fn finish(&mut self, py: Python<'_>) -> PyResult<PyObject> {
        self.core.finish();
        Ok(positional_result_to_dict(py, self.core.result())?.into())
    }
}

/// Python 模块定义。
#[pymodule]
fn newchan_rust(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyOrganicTape>()?;
    m.add_function(wrap_pyfunction!(run_organic_rust, m)?)?;
    m.add_function(wrap_pyfunction!(run_recursive_rust, m)?)?;
    m.add_function(wrap_pyfunction!(run_positional_rust, m)?)?;
    m.add_class::<PyUnnStream>()?;
    // 螺旋引擎 v2（D∞ 群结构第一性原理）：流式 SpiralStream + 批量 run_spiral。
    m.add_class::<spiral::ffi::PySpiralStream>()?;
    m.add_function(wrap_pyfunction!(spiral::ffi::run_spiral, m)?)?;
    // 赋格引擎 v3（递归嵌套多重赋格）：流式 FugueV3Stream + 批量 run_fugue_v3。
    m.add_class::<fugue_v3::ffi::PyFugueV3Stream>()?;
    m.add_function(wrap_pyfunction!(fugue_v3::ffi::run_fugue_v3, m)?)?;
    m.add_function(wrap_pyfunction!(recursive_t::ffi::run_recursive_t, m)?)?;
    // 统一递归算子 T 流式赋格引擎（全 Rust 内聚：orchestrator + T 重跑 + 完整仓位）：
    // 流式 TFugueStream（NT on_bar 驱动）+ 批量 run_t_fugue（共享 core，bit-exact）。
    m.add_class::<recursive_t::ffi::PyTFugueStream>()?;
    m.add_function(wrap_pyfunction!(recursive_t::ffi::run_t_fugue, m)?)?;
    // 递归 T 引擎流式（NT on_bar 驱动，输出目标净敞口，NT 1:1 镜像执行）。
    m.add_class::<recursive_t::ffi::PyRecStream>()?;
    m.add_class::<PyBiEngine>()?;
    m.add_class::<PyOnlineMacdState>()?;
    m.add_class::<PyRecursiveOrchestrator>()?;
    m.add_function(wrap_pyfunction!(segments_from_strokes_v1, m)?)?;
    m.add_function(wrap_pyfunction!(zhongshu_from_segments, m)?)?;
    m.add_function(wrap_pyfunction!(zhongshu_from_strokes, m)?)?;
    m.add_function(wrap_pyfunction!(moves_from_zhongshus, m)?)?;
    m.add_function(wrap_pyfunction!(compute_move_persistence, m)?)?;
    m.add_function(wrap_pyfunction!(attach_persistence, m)?)?;
    m.add_function(wrap_pyfunction!(should_stop_recursion, m)?)?;
    m.add_function(wrap_pyfunction!(divergences_from_moves_v1, m)?)?;
    m.add_function(wrap_pyfunction!(buysellpoints_from_level, m)?)?;
    m.add_function(wrap_pyfunction!(compute_macd, m)?)?;
    m.add_function(wrap_pyfunction!(macd_area_for_range, m)?)?;
    m.add_function(wrap_pyfunction!(dif_peak_for_range, m)?)?;
    m.add_function(wrap_pyfunction!(histogram_peak_for_range, m)?)?;
    m.add_function(wrap_pyfunction!(b_segment_crosses_zero, m)?)?;
    Ok(())
}
