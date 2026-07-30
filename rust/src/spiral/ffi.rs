//! PyO3 导出：`SpiralStream`（流式，对标 `UnnStream`）+ `run_spiral`（批量）。
//!
//! GUARD-ROLE: legacy-generation-loadbearing-for-fugue-v3——名分：现役（详见
//! `spiral/mod.rs` 头部 GUARD-ROLE 块，#762 C7-E3 核定）。
//!
//! 设计来源：架构 §9.2/§9.3。批量 + 流式**共享 `SpiralEngineCore::step/finish`** ⇒
//! 逐 bar 累积的 `finish()` 与批量逐位等价（bit-exact 由构造保证，非对齐努力）。
//!
//! ## 数据流（与 NautilusTrader on_bar 同构，架构 §9.1）
//! ```text
//! NT on_bar → RecursiveOrchestrator.process_bar → StreamingSignalReader
//!   → SpiralStream.push_bar(close, masks, bsp_rows, div_rows, flip_rows)
//!   → list[trade11]（本 bar 新增）→ ChanlunBridge → NT 下单
//! ```
//!
//! ## trade11 契约（固定，破坏即破 bit-exact 验证管线，架构 §9.3）
//! `(ladder, entry_bar, entry_price, exit_bar, exit_price, shares, weight_at_entry,
//!   deferred_bars, partial, exit_reason, polarity)`

use pyo3::prelude::*;
use pyo3::types::PyDict;

use super::engine::SpiralEngineCore;
use super::result::SpiralResult;
use crate::buysellpoint::{BspKind, Side};
use crate::divergence::DivKind;
use crate::stroke::Direction;
use crate::trading::tape::BarSig;
use crate::trading::types::{BspClass, BspEvent, DivEvent, LadderMask, MAX_LADDER};

type BspRow = (
    u8,
    String,
    String,
    i64,
    bool,
    Option<i64>,
    Option<f64>,
    Option<f64>,
    f64,
);
type DivRow = (u8, String, String, i64, f64, f64, f64);
type FlipRow = (u8, String);
type Trade11 = (
    u8,
    i64,
    f64,
    i64,
    f64,
    f64,
    f64,
    i64,
    bool,
    &'static str,
    &'static str,
);
type BarTuple = (
    f64,
    u16,
    u16,
    u16,
    u16,
    u16,
    u8,
    bool,
    Vec<BspRow>,
    Vec<DivRow>,
    Vec<FlipRow>,
);

fn parse_bsp_kind(s: &str) -> PyResult<BspKind> {
    match s {
        "type1" => Ok(BspKind::Type1),
        "type2" => Ok(BspKind::Type2),
        "type3" => Ok(BspKind::Type3),
        _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "非法 BSP kind: {s:?}"
        ))),
    }
}

fn parse_side(s: &str) -> PyResult<Side> {
    match s {
        "buy" => Ok(Side::Buy),
        "sell" => Ok(Side::Sell),
        _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "非法 side: {s:?}"
        ))),
    }
}

fn parse_direction(s: &str) -> PyResult<Direction> {
    match s {
        "up" => Ok(Direction::Up),
        "down" => Ok(Direction::Down),
        _ => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "非法方向: {s:?}"
        ))),
    }
}

/// 从 Python 行构造 BarSig + flip_edge（push_bar / run_spiral 共享，DRY）。
#[allow(clippy::too_many_arguments)]
fn build_bar_sig(
    close: f64,
    buy1: u16,
    sell1: u16,
    sell_any: u16,
    buy_any: u16,
    up_settled: u16,
    max_ladder: u8,
    type2_buy: bool,
    bsp_rows: Vec<BspRow>,
    div_rows: Vec<DivRow>,
    flip_rows: Vec<FlipRow>,
) -> PyResult<(BarSig, [Option<Direction>; MAX_LADDER])> {
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
                "bsp 事件 cs 存在而 zd/zg 缺失（ladder {lad}）"
            )));
        }
        let class = BspClass::from_parts(parse_bsp_kind(&kind)?, parse_side(&side)?);
        sig.bsp_events
            .get_or_insert_with(|| Box::new(<[Vec<BspEvent>; MAX_LADDER]>::default()))[lad_us]
            .push(BspEvent {
                class,
                seg_idx,
                confirmed,
                cs,
                zd,
                zg,
                price,
            });
    }
    for (lad, kind, direction, seg_idx, force_a, force_c, price) in div_rows {
        let lad_us = lad as usize;
        if lad_us >= MAX_LADDER {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "div 事件 ladder 越界: {lad}"
            )));
        }
        let dkind = match kind.as_str() {
            "trend" => DivKind::Trend,
            "consolidation" => DivKind::Consolidation,
            _ => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "非法 div kind: {kind:?}"
                )))
            }
        };
        let dir = parse_direction(&direction)?;
        sig.div_events
            .get_or_insert_with(|| Box::new(<[Vec<DivEvent>; MAX_LADDER]>::default()))[lad_us]
            .push(DivEvent {
                kind: dkind,
                direction: dir,
                seg_idx,
                force_a,
                force_c,
                price,
            });
    }
    let mut flip_edge: [Option<Direction>; MAX_LADDER] = [None; MAX_LADDER];
    for (lad, dir) in flip_rows {
        let lad_us = lad as usize;
        if lad_us >= MAX_LADDER {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "flip 行 ladder 越界: {lad}"
            )));
        }
        flip_edge[lad_us] = Some(parse_direction(&dir)?);
    }
    Ok((sig, flip_edge))
}

/// SpiralResult → PyDict（push_bar.finish + run_spiral 共享）。
fn result_to_dict<'py>(py: Python<'py>, res: &SpiralResult) -> PyResult<Bound<'py, PyDict>> {
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
    d.set_item(
        "n_root_entries_by_ladder",
        res.n_root_entries_by_ladder.to_vec(),
    )?;
    d.set_item("n_entries_by_ladder", res.n_entries_by_ladder.to_vec())?;
    d.set_item("n_exits_by_ladder", res.n_exits_by_ladder.to_vec())?;
    d.set_item(
        "n_root_flips_by_ladder",
        res.n_root_flips_by_ladder.to_vec(),
    )?;
    d.set_item("n_spawns_by_ladder", res.n_spawns_by_ladder.to_vec())?;
    d.set_item(
        "n_cascade_closes_by_ladder",
        res.n_cascade_closes_by_ladder.to_vec(),
    )?;
    d.set_item(
        "n_liquidations_by_ladder",
        res.n_liquidations_by_ladder.to_vec(),
    )?;
    d.set_item(
        "n_cost_rejects_by_ladder",
        res.n_cost_rejects_by_ladder.to_vec(),
    )?;
    d.set_item(
        "n_noref_rejects_by_ladder",
        res.n_noref_rejects_by_ladder.to_vec(),
    )?;
    d.set_item(
        "n_floor_stops_by_ladder",
        res.n_floor_stops_by_ladder.to_vec(),
    )?;
    d.set_item("n_arms_by_ladder", res.n_arms_by_ladder.to_vec())?;
    d.set_item("n_fire_sell_by_ladder", res.n_fire_sell_by_ladder.to_vec())?;
    d.set_item("n_fire_buy_by_ladder", res.n_fire_buy_by_ladder.to_vec())?;
    d.set_item("n_breaks_by_ladder", res.n_breaks_by_ladder.to_vec())?;
    d.set_item(
        "short_net_cash_by_ladder",
        res.short_net_cash_by_ladder.to_vec(),
    )?;
    d.set_item("earning_units", res.earning_units)?;
    d.set_item("shrink_units", res.shrink_units)?;
    d.set_item("cross_level_closures", res.cross_level_closures)?;
    d.set_item("max_children", res.max_children)?;
    d.set_item("t50_monotone_violations", res.t50_monotone_violations)?;
    d.set_item("t56_radial_coverage", res.t56_radial_coverage)?;
    d.set_item("t57_onesided_layers", res.t57_onesided_layers)?;
    d.set_item("t58_active_levels", res.t58_active_levels)?;
    d.set_item("t59_degenerate_layers", res.t59_degenerate_layers)?;
    d.set_item("phys_long_bars", res.phys_long_bars)?;
    d.set_item("phys_short_bars", res.phys_short_bars)?;
    d.set_item("held_bars_by_ladder", res.held_bars_by_ladder.to_vec())?;
    d.set_item(
        "short_held_bars_by_ladder",
        res.short_held_bars_by_ladder.to_vec(),
    )?;
    Ok(d)
}

/// 流式螺旋引擎（对标 `UnnStream`）。
#[pyclass(name = "SpiralStream")]
pub struct PySpiralStream {
    core: SpiralEngineCore,
}

#[pymethods]
impl PySpiralStream {
    #[new]
    #[pyo3(signature = (floor_ladder = 2))]
    fn new(floor_ladder: usize) -> PyResult<Self> {
        let core =
            SpiralEngineCore::new(floor_ladder).map_err(pyo3::exceptions::PyValueError::new_err)?;
        Ok(Self { core })
    }

    /// 逐 bar 推送（事件行无 bar 字段，本 bar 内）。返回本 bar **新增** trade（trade11）。
    #[allow(clippy::too_many_arguments)]
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
        bsp_rows: Vec<BspRow>,
        div_rows: Vec<DivRow>,
        flip_rows: Vec<FlipRow>,
    ) -> PyResult<Vec<Trade11>> {
        let (sig, flip_edge) = build_bar_sig(
            close, buy1, sell1, sell_any, buy_any, up_settled, max_ladder, type2_buy, bsp_rows,
            div_rows, flip_rows,
        )?;
        let before = self.core.n_trades();
        self.core.step(&sig, &flip_edge);
        let new_trades = self.core.result().trades[before..]
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

    /// 收尾（eod cascade 关根 + N4 反证 + T 观测）并返回完整结果 dict。幂等。
    fn finish(&mut self, py: Python<'_>) -> PyResult<PyObject> {
        self.core.finish();
        Ok(result_to_dict(py, self.core.result())?.into())
    }
}

/// 批量螺旋回测（与 `SpiralStream.finish` 同结构 dict）。共享 `step/finish` ⇒ 与流式
/// 逐位等价（bit-exact 构造性保证）。`bars` = 每 bar 11-tuple（与 push_bar 参数同序）。
#[pyfunction]
#[pyo3(signature = (bars, floor_ladder = 2))]
pub fn run_spiral(py: Python<'_>, bars: Vec<BarTuple>, floor_ladder: usize) -> PyResult<PyObject> {
    let mut core =
        SpiralEngineCore::new(floor_ladder).map_err(pyo3::exceptions::PyValueError::new_err)?;
    for (
        close,
        buy1,
        sell1,
        sell_any,
        buy_any,
        up_settled,
        max_ladder,
        type2_buy,
        bsp_rows,
        div_rows,
        flip_rows,
    ) in bars
    {
        let (sig, flip_edge) = build_bar_sig(
            close, buy1, sell1, sell_any, buy_any, up_settled, max_ladder, type2_buy, bsp_rows,
            div_rows, flip_rows,
        )?;
        core.step(&sig, &flip_edge);
    }
    core.finish();
    Ok(result_to_dict(py, core.result())?.into())
}
