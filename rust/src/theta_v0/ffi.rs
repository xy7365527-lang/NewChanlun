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
use crate::funding_record::FundingRecord;

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
    funding_record: String,
    records: Vec<String>,
    admissible: bool,
}

/// 诊断夹具预设名（#1602）：场所 profile 用 `venue-profile-diagnostic-fixture`，不参与准入。
pub const DIAGNOSTIC_FIXTURE_PRESET: &str = "diagnostic-fixture";

/// `push_bar` 的 nav 准入（#1601 C-03）：nav 必须是正有限数；≤0 或非有限 ⟹ 报错，不换成任何备用值。
fn checked_nav(nav: f64) -> PyResult<f64> {
    if nav.is_finite() && nav > 0.0 {
        Ok(nav)
    } else {
        Err(pyo3::exceptions::PyValueError::new_err(format!(
            "nav={nav} 非正：账户净值缺失或非法，停止（不回退任何备用值，#1601 / ADR 0041）"
        )))
    }
}

#[pymethods]
impl PyThetaStream {
    /// 构造（#1602 记录口径）：
    ///
    /// - `venue_profile="P0"` 等 ⟹ 生产场所 profile；profile 缺 tick 或成本项 ⟹ `ValueError`
    ///   （`AwaitingPolicy{venue_profile|cost_inputs}`），不回退任何旧常数；
    /// - `preset="diagnostic-fixture"` ⟹ 诊断夹具场所 profile，[`admissible`](Self::admissible) 为 `False`，
    ///   不参与准入；
    /// - 两者都不给 ⟹ 没有场所 profile，同样 `ValueError`。两者同时给或给未知预设 ⟹ `ValueError`。
    ///
    /// `funding_profile` 必须指定有效生产资金记录；诊断场所仍不参与准入。
    #[new]
    #[pyo3(signature = (preset=None, funding_profile=None, venue_profile=None))]
    fn new(
        preset: Option<String>,
        funding_profile: Option<&str>,
        venue_profile: Option<String>,
    ) -> PyResult<Self> {
        let err = |m: String| pyo3::exceptions::PyValueError::new_err(m);
        let config = match (preset.as_deref(), venue_profile.as_deref()) {
            (Some(DIAGNOSTIC_FIXTURE_PRESET), None) => ThetaConfig::diagnostic_fixture(),
            (Some(name), None) => {
                return Err(err(format!(
                    "preset {name:?} 未实装：只支持 preset={DIAGNOSTIC_FIXTURE_PRESET:?}"
                )))
            }
            (Some(_), Some(_)) => return Err(err("preset 与 venue_profile 只能给一个".into())),
            (None, Some(profile)) => ThetaConfig::for_production_profile(profile),
            (None, None) => Err(super::run_records::no_venue_profile()),
        }
        .map_err(|e| err(e.to_string()))?;
        let funding = FundingRecord::production(funding_profile).map_err(|e| err(e.to_string()))?;
        let tick_size = config.tick.tick_size;
        let records = config.record_refs().iter().map(|r| r.label()).collect();
        let admissible = config.admissible();
        Ok(Self {
            core: ThetaPiStream::new(config),
            tick_size,
            initial_nav: funding.q_total_f64(),
            funding_record: funding.label(),
            records,
            admissible,
        })
    }

    /// 所用记录的 `id/版本` 列表（#1602；塔预算、MACD、场所 profile、Θ_risk、数据 profile）。
    fn records(&self) -> Vec<String> {
        self.records.clone()
    }

    /// 本流的输出能否参与准入（诊断夹具 ⟹ `False`）。
    fn admissible(&self) -> bool {
        self.admissible
    }

    /// 推一根 bar。`p_t` = 调用方真实净持仓（手数，有符号）；`nav` = 账户净值（须 >0 且有限，
    /// 否则报错、本 bar 不推进）。返回**目标净持仓 p\***（有符号手数，lot 对齐）——与
    /// `RecTStream.push_bar` 的 `lu−su` 同范畴同量纲。
    fn push_bar(&mut self, o: f64, h: f64, l: f64, c: f64, p_t: f64, nav: f64) -> PyResult<f64> {
        let nav = checked_nav(nav)?;
        let bar = self.make_bar(o, h, l, c);
        Ok(self.core.push_bar(bar, p_t, nav))
    }

    /// 构造期读到的 `Q_total`（funding-record `cap`）。
    #[getter]
    fn q_total(&self) -> f64 {
        self.initial_nav
    }

    /// 本实例读的 funding-record id 与版本。
    #[getter]
    fn funding_record(&self) -> String {
        self.funding_record.clone()
    }

    /// 便利重载（**仅供研究脚本对拍**，非生产语义）：内部用上一次 `p_star` 当 `p_t`，模拟
    /// 「全成交无滑点」。生产路径必须用 [`push_bar`](Self::push_bar) 显式传 `p_t`。
    #[pyo3(signature = (o, h, l, c))]
    fn push_bar_selfstate(&mut self, o: f64, h: f64, l: f64, c: f64) -> PyResult<f64> {
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
        d.set_item("funding_record", &self.funding_record)?;
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
            volume: 1.0,
            untradable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #1601 C-03 负控：nav ≤ 0 / 非有限 ⟹ 报错，不换成 1e6 或任何备用值；正值原样通过。
    #[test]
    fn non_positive_nav_is_rejected_not_replaced() {
        for bad in [0.0, -1.0, -1.0e6, f64::NAN, f64::INFINITY] {
            assert!(checked_nav(bad).is_err(), "nav={bad} 须报错");
        }
        assert_eq!(checked_nav(123.5).unwrap(), 123.5);
    }

    /// #1601：缺 profile ⟹ 构造报错；evidence_only ⟹ 拒收；P0 ⟹ nav 基数 = Q_total 100,000，
    /// 并写出记录 id 与版本（改回常数的变异体在此变红）。push_bar 在 nav≤0 时报错且不推进 bar。
    #[test]
    fn funding_and_venue_profiles_are_independent_and_both_required() {
        pyo3::prepare_freethreaded_python();
        assert!(PyThetaStream::new(None, Some("P0"), None).is_err());
        let venue_error = PyThetaStream::new(None, Some("P0"), Some("P0".into()))
            .err()
            .expect("unsigned venue cannot borrow funding authorization");
        assert!(
            venue_error
                .to_string()
                .contains("AwaitingPolicy{cost_inputs}"),
            "{venue_error}"
        );
        // #1774 B2：显式给 synthetic id 的通用调用被 usage 门拒绝（InvalidPolicy，不是缺资金）。
        let synthetic_error = PyThetaStream::new(
            None,
            Some("P0"),
            Some(super::run_records::SYNTHETIC_TB02A_PROFILE_ID.into()),
        )
        .err()
        .expect("synthetic 只供 S 专用入口");
        let text = synthetic_error.to_string();
        assert!(text.contains("InvalidPolicy{venue_profile}"), "{text}");
        assert!(text.contains("usage 不符"), "{text}");
        assert!(PyThetaStream::new(
            Some(DIAGNOSTIC_FIXTURE_PRESET.into()),
            Some("P0"),
            Some("P0".into())
        )
        .is_err());
        let stream = PyThetaStream::new(
            Some(DIAGNOSTIC_FIXTURE_PRESET.into()),
            Some("P1-hl-testnet"),
            None,
        )
        .unwrap();
        assert_eq!(stream.q_total(), 500.0);
        assert!(!stream.admissible());
        assert!(stream
            .records()
            .contains(&"venue-profile-diagnostic-fixture/1".into()));
    }

    #[test]
    fn stream_reads_funding_record_and_rejects_bad_nav() {
        assert!(PyThetaStream::new(Some(DIAGNOSTIC_FIXTURE_PRESET.into()), None, None).is_err());
        assert!(PyThetaStream::new(
            Some(DIAGNOSTIC_FIXTURE_PRESET.into()),
            Some("P0-acceptance-5m"),
            None
        )
        .is_err());
        let mut s = PyThetaStream::new(Some(DIAGNOSTIC_FIXTURE_PRESET.into()), Some("P0"), None)
            .expect("P0 生效记录");
        assert_eq!(s.q_total(), 100_000.0);
        assert_eq!(s.funding_record(), "#1557/funding-authorization/v1@P0/v1");
        let p1 = PyThetaStream::new(
            Some(DIAGNOSTIC_FIXTURE_PRESET.into()),
            Some("P1-hl-testnet"),
            None,
        )
        .expect("P1 生效记录");
        assert_eq!(p1.q_total(), 500.0);
        assert!(s.push_bar(100.0, 100.1, 99.9, 100.0, 0.0, 0.0).is_err());
        assert!(s.push_bar(100.0, 100.1, 99.9, 100.0, 0.0, -5.0).is_err());
        assert_eq!(s.core.bar_count(), 0, "nav 非法的 bar 不得推进");
        assert!(s.push_bar(100.0, 100.1, 99.9, 100.0, 0.0, 1_000.0).is_ok());
        assert_eq!(s.core.bar_count(), 1);
    }
}
