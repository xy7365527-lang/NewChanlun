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
mod buysellpoint;
mod divergence;
mod fractal;
mod level;
mod macd;
mod moves;
mod orchestrator;
mod ph;
mod segment;
mod stroke;
mod zhongshu;

use pyo3::prelude::*;
use stroke::{Direction, Stroke};

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

    buysellpoint::buysellpoints_from_level(&segs, &zss, &zs_break, &mvs, &divs, level_id)
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
    ))]
    fn new(
        max_levels: i64,
        stroke_mode: &str,
        min_strict_sep: i64,
        reset_dir_on_fractal: bool,
        new_raw_gap_min: i64,
        enable_macd_divergence: bool,
    ) -> Self {
        PyRecursiveOrchestrator {
            inner: orchestrator::RecursiveOrchestrator::new(
                max_levels,
                stroke_mode,
                min_strict_sep,
                reset_dir_on_fractal,
                new_raw_gap_min,
                enable_macd_divergence,
            ),
        }
    }

    /// 处理一根新 K 线（仅传 OHLC；结构化状态不依赖时间戳）。
    fn process_bar(&mut self, open: f64, high: f64, low: f64, close: f64) {
        self.inner.process_bar(open, high, low, close);
    }

    fn reset(&mut self) {
        self.inner.reset();
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
}

/// Python 模块定义。
#[pymodule]
fn newchan_rust(m: &Bound<'_, PyModule>) -> PyResult<()> {
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
