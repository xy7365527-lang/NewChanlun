//! PyO3 binding for 统一递归算子 T。
//!
//! a₀ = 线段序列（与 v3 递归层 `move = zhongshu_from_segments` 对齐，设计文档 §2），
//! 输出全塔买卖点。用于 T 输出与 v3 nf 信号 L2 对照（§6.6 阶段2）+ 三模式回测对照
//! （编排者 2026-06-18 裁决「三条路实测」，escalation `2026-06-18-1752-t-stepc-macd-
//! scope-vs-direction.md`）。
//!
//! 步骤c 走势完美判定模式由 `mode` 选取（默认纯结构，向后兼容历史单参数调用）；MACD
//! 面积由调用方经 `seg_areas` 注入（每段红/绿柱面积），T 自身不算 MACD——保持 standalone
//! 纯结构定位，close 价格不污染 T 的拓扑构造（escalation 附录·坐标系陷阱）。

use pyo3::prelude::*;
use pyo3::types::PyDict;

use super::stream::TFugueStreamCore;
use super::{iterate, Direction, PerfectionMode, Unit};
use crate::fugue_v3::layer::FugueResult;

/// 解析方向字符串。
fn parse_dir(s: &str) -> Direction {
    match s {
        "up" => Direction::Up,
        "down" => Direction::Down,
        other => panic!("invalid segment direction: {other:?}"),
    }
}

/// 解析走势完美判定模式字符串。默认（`None`）= `Structural`；兼容 "structure"/"structural"。
fn parse_mode(s: Option<&str>) -> PerfectionMode {
    match s {
        None | Some("structure") | Some("structural") => PerfectionMode::Structural,
        Some("and") => PerfectionMode::And,
        Some("or") => PerfectionMode::Or,
        Some(other) => panic!("invalid perfection mode: {other:?} (expect structure/and/or)"),
    }
}

/// 用线段序列驱动 T，返回全塔买卖点。
///
/// `segs`：每段 `(i0, i1, dir, high, low, confirmed, kind_is_settled)`——与
/// `segments_from_strokes_v1` 输出对齐。仅 `confirmed AND settled` 线段作为 a₀ 单元
/// （与 v3 `zhongshu_from_segments` 过滤口径一致，保证 T/v3 在同一 a₀ 上对照）。
///
/// `seg_areas`（可选）：每段 `(area_pos, area_neg)`，**与 `segs` 同序同长**（过滤前对齐）。
/// 调用方经 `macd::macd_area_for_range` + `merged_to_raw` 算出（均取非负，绿柱用 `.abs()`），
/// 供 `And`/`Or` 模式的 MACD 面积背驰判据。`None` → area 全 0（纯结构，与历史单参数调用
/// 逐位等价）。
///
/// `mode`（可选）：步骤c 走势完美判定模式 "structure"（默认）/ "and" / "or"。
///
/// 返回：`(kind, bar, price, level)` 列表。kind ∈ {type1_buy/sell, type2_*, type3_*}；
/// bar = 买卖点所在 a₀ 单元端点的 bar（i0/i1，**merged bar 坐标**——回测取 close 价须经
/// `merged_to_raw` 转 raw bar）；level = T 迭代深度（0 = 最低走势级别，由线段构成；
/// 向上递归到涌现上界 r*）。
#[pyfunction]
#[pyo3(signature = (segs, seg_areas=None, mode=None))]
pub fn run_recursive_t(
    segs: Vec<(usize, usize, String, f64, f64, bool, bool)>,
    seg_areas: Option<Vec<(f64, f64)>>,
    mode: Option<String>,
) -> Vec<(String, i64, f64, usize)> {
    let perfection = parse_mode(mode.as_deref());
    // area 与 segs 同序对齐（过滤前）；缺省全 0。长度不符 = 调用方错误，fail fast。
    let areas: Vec<(f64, f64)> = match seg_areas {
        Some(a) => {
            assert_eq!(
                a.len(),
                segs.len(),
                "seg_areas 长度 ({}) 须与 segs ({}) 一致",
                a.len(),
                segs.len()
            );
            a
        }
        None => vec![(0.0, 0.0); segs.len()],
    };
    let units: Vec<Unit> = segs
        .iter()
        .zip(areas.iter())
        .filter(|((_, _, _, _, _, confirmed, settled), _)| *confirmed && *settled)
        .map(|((i0, i1, dir, high, low, _, _), (area_pos, area_neg))| Unit {
            high: *high,
            low: *low,
            start_bar: *i0 as i64,
            end_bar: *i1 as i64,
            direction: parse_dir(dir),
            level: 0,
            inner_zhongshu_count: 0,
            area_pos: *area_pos,
            area_neg: *area_neg,
        })
        .collect();
    let tree = iterate(units, perfection);
    tree.all_bsps()
        .iter()
        .map(|b| (b.kind.as_str().to_string(), b.bar, b.price, b.level))
        .collect()
}

// ════════════════════════════════════════════════════════════════════════════
// T 算子流式赋格引擎（NautilusTrader on_bar 驱动；对标 FugueV3Stream）
// ════════════════════════════════════════════════════════════════════════════

/// trade11 契约（与 FugueV3Stream 逐字一致 ⇒ Python 分析层复用）：
/// `(ladder, entry_bar, entry_price, exit_bar, exit_price, shares, weight_at_entry,
///   deferred_bars, partial, exit_reason, polarity)`。
type Trade11 = (u8, i64, f64, i64, f64, f64, f64, i64, bool, &'static str, &'static str);

/// FugueResult → PyDict（PyTFugueStream.finish + run_t_fugue 共享）。
///
/// 只暴露 T 引擎**实际产出**的字段（no-patch-mentality 声明=能力）：trade11 + 守恒/操作计数。
/// **不**含 n_arms/n_fire/n_breaks——那是 v3 spiral 信号层 nest 窗口计数，T standalone 不产
/// （T 的信号是全塔 BSP diff，无向心 confirm 武装/破窗概念）。
fn t_result_to_dict<'py>(py: Python<'py>, res: &FugueResult) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new(py);
    let trades: Vec<Trade11> = res
        .trades
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
    d.set_item("trades", trades)?;
    d.set_item("equity", res.equity.clone())?;
    d.set_item("final_nav", res.final_nav)?;
    d.set_item("n_entries_by_ladder", res.n_entries_by_ladder.to_vec())?;
    d.set_item("n_core_clears_by_ladder", res.n_core_clears_by_ladder.to_vec())?;
    d.set_item("n_cycle_opens_by_ladder", res.n_cycle_opens_by_ladder.to_vec())?;
    d.set_item("n_cycle_closes_by_ladder", res.n_cycle_closes_by_ladder.to_vec())?;
    d.set_item("n_liquidations_by_ladder", res.n_liquidations_by_ladder.to_vec())?;
    d.set_item("n_cost_rejects_by_ladder", res.n_cost_rejects_by_ladder.to_vec())?;
    d.set_item("n_noref_rejects_by_ladder", res.n_noref_rejects_by_ladder.to_vec())?;
    d.set_item("mobile_realized_pnl_by_ladder", res.mobile_realized_pnl_by_ladder.to_vec())?;
    d.set_item("cross_level_closures", res.cross_level_closures)?;
    d.set_item("max_concurrent_voices", res.max_concurrent_voices)?;
    d.set_item("max_chiral_same_dir", res.max_chiral_same_dir)?;
    d.set_item("phys_long_bars", res.phys_long_bars)?;
    d.set_item("phys_short_bars", res.phys_short_bars)?;
    d.set_item("short_held_bars_by_ladder", res.short_held_bars_by_ladder.to_vec())?;
    Ok(d)
}

/// T 算子流式赋格引擎（对标 `FugueV3Stream`）。
///
/// 与 FugueV3Stream 的范畴差：T standalone ⇒ 信号层 + 仓位层全 Rust 内聚，`push_bar` 只传 OHLC
/// 4 个 float（FugueV3Stream 信号层在 Python，push_bar 收 11 个拆解字段）。`mode`：步骤c 走势
/// 完美判定 Structural/And/Or（受控实验唯一变量）。
#[pyclass(name = "TFugueStream")]
pub struct PyTFugueStream {
    core: TFugueStreamCore,
}

#[pymethods]
impl PyTFugueStream {
    #[new]
    #[pyo3(signature = (mode=None))]
    fn new(mode: Option<String>) -> Self {
        let perfection = parse_mode(mode.as_deref());
        PyTFugueStream { core: TFugueStreamCore::new(perfection) }
    }

    /// 逐 bar 推送 OHLC（NautilusTrader on_bar）。返回本 bar **新增** trade（trade11）。
    fn push_bar(&mut self, o: f64, h: f64, l: f64, c: f64) -> Vec<Trade11> {
        let n_new = self.core.push_bar(o, h, l, c);
        let trades = &self.core.result().trades;
        let start = trades.len().saturating_sub(n_new);
        trades[start..]
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
            .collect()
    }

    /// 状态快照: (cur_bar, nav, long_units, short_units, n_active_voices)。
    fn snapshot(&self) -> (i64, f64, f64, f64, usize) {
        self.core.snapshot()
    }

    /// 收尾（eod 清仓 + final_nav）并返回完整结果 dict。幂等。
    fn finish(&mut self, py: Python<'_>) -> PyResult<PyObject> {
        self.core.finish();
        let d = t_result_to_dict(py, self.core.result())?;
        Ok(d.into())
    }
}

/// 批量 T 流式赋格回测（与 `TFugueStream.finish` 同结构 dict）。共享 `TFugueStreamCore` ⇒
/// 逐 bar 累积的 finish 与批量逐位等价（bit-exact 由构造保证）。
///
/// `bars`：`(open, high, low, close)` 序列（须已清洗，NaN/≤0 在数据层删除）。
#[pyfunction]
#[pyo3(signature = (bars, mode=None))]
pub fn run_t_fugue(py: Python<'_>, bars: Vec<(f64, f64, f64, f64)>, mode: Option<String>) -> PyResult<PyObject> {
    let perfection = parse_mode(mode.as_deref());
    let mut core = TFugueStreamCore::new(perfection);
    for (o, h, l, c) in bars {
        core.push_bar(o, h, l, c);
    }
    core.finish();
    let d = t_result_to_dict(py, core.result())?;
    Ok(d.into())
}
