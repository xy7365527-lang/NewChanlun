//! #951 D1 出口：`theta_v0` 的 PyO3 出口（建新件）。
//!
//! Python 类名 `ThetaStream`（Rust 内部 `PyThetaStream`），seam =
//! `push_bar(o, h, l, c, p_t, nav) -> f64`（返回目标净敞口 `p_star`，有符号手数，lot 对齐）。
//!
//! ## 状态裁定（设计稿 §1.3）
//!
//! 无状态持仓口径：`p_t`（调用方真实净持仓，手数）每 bar 显式传入；成交回执不回填 Rust。
//! 订单由 Python 侧照 `rec_t_strategy.py:_rebalance` 的老办法算 `delta = target − portfolio.net_position`。
//! `push_bar_selfstate` 是研究对拍便利重载（内部用上一次 `p_star` 当 `p_t`），**非生产语义**，
//! docstring 明写。
//!
//! ## ElementId 投影（设计稿 §5.1 未决项，本文件定案）
//!
//! 打开 [`ElementId`](crate::theta_v0::classifier::recursive_tower::ElementId) 后确认其字段构成
//! = `{ level: u32, ordinal: u64 }` ⟹ 投影为 `(u32, u64)` 元组（可哈希、可比较、跨 bar 一致）。

use pyo3::prelude::*;
use pyo3::types::PyDict;

use super::config::ThetaConfig;
use super::stream::ThetaPiStream;
use super::types::{quantize, Bar, StrictAction};

fn action_str(a: StrictAction) -> &'static str {
    match a {
        StrictAction::Buy => "buy",
        StrictAction::Sell => "sell",
        StrictAction::Add => "add",
        StrictAction::Reduce => "reduce",
        StrictAction::Hold => "hold",
        StrictAction::Close => "close",
        StrictAction::Wait => "wait",
    }
}

fn side_str(s: super::strategy::voice::VoiceSide) -> &'static str {
    match s {
        super::strategy::voice::VoiceSide::Long => "long",
        super::strategy::voice::VoiceSide::Short => "short",
        super::strategy::voice::VoiceSide::Flat => "flat",
    }
}

fn vertical_str(v: super::strategy::coverage::Vertical) -> &'static str {
    match v {
        super::strategy::coverage::Vertical::Ambient => "ambient",
        super::strategy::coverage::Vertical::FollowParent => "follow_parent",
        super::strategy::coverage::Vertical::ReverseOpen => "reverse_open",
    }
}

fn parent_proj(p: Option<super::classifier::recursive_tower::ElementId>) -> Option<(u32, u64)> {
    p.map(|id| (id.level, id.ordinal))
}

// `unsendable`：`ThetaPiStream` 内含 `OwnedIncrementalClassifier` 的 `Rc<..>`（增量分类器缓存），
// 不满足 `Send`。Python 侧单线程经 GIL 访问 ⟹ `unsendable` 合法（与 pyo3 0.23 语义一致）。
#[pyclass(name = "ThetaStream", unsendable)]
pub struct PyThetaStream {
    core: ThetaPiStream,
    tick_size: f64,
    initial_nav: f64,
}

#[pymethods]
impl PyThetaStream {
    /// symbol/config 预设名。本票只支持 `None`（`ThetaConfig::default()`）；传值 ⟹ 报错（诚实拒绝，
    /// 不静默吞掉未知预设）。`initial_nav` 仅在 `push_bar_selfstate` 与 `push_bar` 的 `nav<=0` 时作兜底。
    #[new]
    #[pyo3(signature = (preset=None, initial_nav=1.0e6))]
    fn new(preset: Option<String>, initial_nav: f64) -> PyResult<Self> {
        if let Some(name) = preset {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "preset {name:?} 未实装：本票只支持 preset=None（ThetaConfig::default()）"
            )));
        }
        let config = ThetaConfig::default();
        let tick_size = config.tick.tick_size;
        Ok(Self {
            core: ThetaPiStream::new(config),
            tick_size,
            initial_nav: if initial_nav > 0.0 {
                initial_nav
            } else {
                1.0e6
            },
        })
    }

    /// 推一根 bar。`p_t` = 调用方真实净持仓（手数，有符号）；`nav` = 账户净值（<=0 时用
    /// 构造期 `initial_nav` 兜底）。返回**目标净持仓 p\***（有符号手数，lot 对齐）——与
    /// `RecTStream.push_bar` 的 `lu−su` 同范畴同量纲。
    fn push_bar(&mut self, o: f64, h: f64, l: f64, c: f64, p_t: f64, nav: f64) -> f64 {
        let bar = self.make_bar(o, h, l, c);
        let nav = if nav > 0.0 { nav } else { self.initial_nav };
        self.core.push_bar(bar, p_t, nav)
    }

    /// 便利重载（**仅供研究脚本对拍**，非生产语义）：内部用上一次 `p_star` 当 `p_t`，模拟
    /// 「全成交无滑点」。生产路径必须用 [`push_bar`](Self::push_bar) 显式传 `p_t`。
    #[pyo3(signature = (o, h, l, c))]
    fn push_bar_selfstate(&mut self, o: f64, h: f64, l: f64, c: f64) -> f64 {
        let p_t = self.core.target_net_units();
        let nav = self.initial_nav;
        self.push_bar(o, h, l, c, p_t, nav)
    }

    /// 上一步的 p\*（不推进 bar）。对位 `RecTStream.target_net_units()`。
    fn target_net_units(&self) -> f64 {
        self.core.target_net_units()
    }

    /// 上一步 Schedule_Θ 的订单形态（诊断用，**不是**下单指令——生产下单由 Python 侧 delta
    /// 提单）。返回 `(action: str, qty: int, exec_index: int)`；action ∈
    /// `{"buy","sell","add","reduce","hold","close","wait"}`。
    fn last_order(&self) -> (String, i64, usize) {
        let o = self.core.last_order();
        (action_str(o.action).to_string(), o.qty, o.exec_index)
    }

    /// 快照：(bar_count, p_star, n_active_voices, n_active_levels)。
    fn snapshot(&self) -> (i64, f64, usize, usize) {
        let (bar, p_star, n_active, n_levels) = self.core.snapshot();
        (bar as i64, p_star, n_active, n_levels)
    }

    /// 逐声部目标腿快照（本 bar 的 P^sep_{t+1}；只读）。
    /// 每项 = `(level, ordinal, side, q_units, role_v, parent_id)`：
    /// `side ∈ {"long","short"}`；`role_v ∈ {"ambient","follow_parent","reverse_open"}`；
    /// `parent_id = None` 或 `(level, ordinal)`（边界胚元 ∂ 根声部 ⟹ None）。
    fn sep_legs(&self, py: Python<'_>) -> PyResult<PyObject> {
        let list = pyo3::types::PyList::empty(py);
        for leg in self.core.sep_legs() {
            list.append((
                leg.id.level,
                leg.id.ordinal,
                side_str(leg.side),
                leg.q_units,
                vertical_str(leg.role_v),
                parent_proj(leg.parent_id),
            ))?;
        }
        Ok(list.into())
    }

    /// 收尾 → 完整 per-leg 账本 dict（对位 `RecTStream.finish_full`，先窗口终点强平再读）。
    /// - `"active_voices"`: [(level, ordinal, side, q, role_v, parent_id, entry_bar, entry_px, pnl_v)]
    /// - `"closed_voices"`: [(level, ordinal, side, role_v, parent_id, entry_bar, exit_bar,
    ///    entry_px, exit_px, pnl_v)]（ClosedVoice 无 q 字段，见 overlay_state.rs:83）
    /// - `"account_price_pnl"`: f64（账户级 Σ_t N_t·ΔP_t）
    /// - `"total_voice_pnl"`: f64（Σ_v pnl_v）
    /// - `"reconcile_residual"`: f64（|前两者之差|，PDF §11 对账，应 < eps）
    /// - `"level_nets"`: [(level, net_ℓ)]（LevelLedgerMirror::nets）
    /// - `"lee_net_witness"`: (n_observations, max_abs_residual, max_abs_net, identity_witnessed)
    /// - `"n_orders"`: usize（ΔN 非零步数）
    fn finish_full(&mut self, py: Python<'_>) -> PyResult<PyObject> {
        self.core.finish();
        let d = PyDict::new(py);
        let active: Vec<(
            u32,
            u64,
            String,
            i64,
            String,
            Option<(u32, u64)>,
            usize,
            f64,
            f64,
        )> = self
            .core
            .overlay()
            .active_voices()
            .map(|v| {
                (
                    v.id.level,
                    v.id.ordinal,
                    side_str(v.side).to_string(),
                    v.q,
                    vertical_str(v.role_v).to_string(),
                    parent_proj(v.parent_id),
                    v.entry_bar,
                    v.entry_px,
                    v.pnl_v,
                )
            })
            .collect();
        d.set_item("active_voices", active)?;
        // ClosedVoice 无 q 字段（overlay_state.rs:83 实测：id/side/role_v/parent_id/entry_bar/
        // exit_bar/entry_px/exit_px/pnl_v）——设计稿 §2.2 的 closed_voices 形状写了 q，与实结构不符；
        // 本文件按实结构投影（10 元组，无 q）。q 只存在于活动声部 VoiceBook（active_voices 保留）。
        let closed: Vec<(
            u32,
            u64,
            String,
            String,
            Option<(u32, u64)>,
            usize,
            usize,
            f64,
            f64,
            f64,
        )> = self
            .core
            .overlay()
            .closed_voices()
            .iter()
            .map(|c| {
                (
                    c.id.level,
                    c.id.ordinal,
                    side_str(c.side).to_string(),
                    vertical_str(c.role_v).to_string(),
                    parent_proj(c.parent_id),
                    c.entry_bar,
                    c.exit_bar,
                    c.entry_px,
                    c.exit_px,
                    c.pnl_v,
                )
            })
            .collect();
        d.set_item("closed_voices", closed)?;
        let account_price_pnl = self.core.overlay().account_price_pnl();
        let total_voice_pnl = self.core.overlay().total_voice_pnl();
        d.set_item("account_price_pnl", account_price_pnl)?;
        d.set_item("total_voice_pnl", total_voice_pnl)?;
        d.set_item(
            "reconcile_residual",
            (account_price_pnl - total_voice_pnl).abs(),
        )?;
        let level_nets: Vec<(u32, i64)> = self
            .core
            .level_ledger()
            .nets()
            .iter()
            .map(|(l, n)| (*l, *n))
            .collect();
        d.set_item("level_nets", level_nets)?;
        let w = self.core.level_ledger().lee_net_witness();
        d.set_item(
            "lee_net_witness",
            (
                w.n_observations,
                w.max_abs_residual,
                w.max_abs_net,
                w.identity_witnessed(),
            ),
        )?;
        d.set_item("n_orders", self.core.n_orders())?;
        Ok(d.into())
    }
}

impl PyThetaStream {
    fn make_bar(&self, o: f64, h: f64, l: f64, c: f64) -> Bar {
        let bad_range = h < o.max(c).max(l) || l > o.min(c).min(h);
        let bad_price = o <= 0.0 || h <= 0.0 || l <= 0.0 || c <= 0.0;
        let bad_value = !o.is_finite() || !h.is_finite() || !l.is_finite() || !c.is_finite();
        let untradable = bad_range || bad_price || bad_value;
        let idx = self.core.bar_count();
        Bar {
            source_index: idx,
            timestamp: idx as i64,
            open: quantize(o, self.tick_size),
            high: quantize(h, self.tick_size),
            low: quantize(l, self.tick_size),
            close: quantize(c, self.tick_size),
            // 无 volume 入参 ⟹ 置 1 哨兵（非零 = 不因缺 volume 被标 untradable；数据卫生归 Python 侧）。
            volume: 1,
            untradable,
        }
    }
}
