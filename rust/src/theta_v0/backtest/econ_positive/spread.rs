//! 聚合诊断「钱去哪了」的归因载体（#1175 A01 职责块自 `econ_positive.rs` 迁出，零行为）。
//!
//! 承载 [`SpreadAttribution`]（确定性分解 Σ 精确等于 Σ trade gross）。消费面经
//! `econ_positive` 重导出保持原路径。

/// 聚合诊断「钱去哪了」（确定性分解，Σ 精确等于 Σ trade gross，非概率推断）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SpreadAttribution {
    pub n_signals: usize,
    /// Σ Ab_rev：反转交易腿给的理想总价差（664 号：post-signal 腿，非触发段）。
    pub sum_a_b: f64,
    /// Σ x_in：**signed** 入场滑移总和（664-Q3）。含有利（<0）+不利（>0），非 adverse-only。
    pub sum_x_in: f64,
    /// Σ y_out：**signed** 出场滑移总和（664-Q3）。
    pub sum_y_out: f64,
    /// Σ ηin = Σ max(0, x_in)：入场滞后吃掉（adverse-only，≥0）。
    pub sum_eta_in: f64,
    /// Σ ηout = Σ max(0, y_out)：出场滞后吃掉（adverse-only，≥0）。
    pub sum_eta_out: f64,
    /// Σ actual_spread = Σ δ(Pτout − Pτin)：**真实成交价差**总和（无成本，664-Q3，含全部滑移）。
    pub sum_actual_spread: f64,
    /// Σ Ce/qe：成本吃掉。
    pub sum_ce: f64,
    /// Σ captured：adverse-only 保守压力测试剩余 alpha（丢有利滑移，系统性偏负）。
    pub sum_captured: f64,
    /// 可捕获信号数（captured>0）——逐信号路径级正占比的分子。
    pub n_captured_positive: usize,
    /// actual_pnl>0 信号数（真实成交口径）——区别于 n_captured_positive（adverse-only）。
    pub n_actual_positive: usize,
    /// 三审计统计①：rho_rev_bar>lambda_rev_bar 计数（出场 pivot 端点在入场 pivot 端点之后；codex Q1 不变量）。
    pub n_rho_after_lambda: usize,
    /// 三审计统计②：same_bar_opposite 计数（同 bar 出现反向信号被 eb>entry_bar 排除；codex Q2 边界）。
    pub n_same_bar_opposite: usize,
    /// 三审计统计③：n_unpaired 计数（入场信号无配对出场反转信号，右删失诚实跳过；codex Q4 边界）。
    pub n_unpaired: usize,
    /// P7 正规出场统计：CloseRoot（第一类顶背驰）配对出场信号数。
    pub n_exit_close_root: usize,
    /// P7 正规出场统计：ReduceCore（第三类）配对出场信号数。
    pub n_exit_reduce_core: usize,
    /// P7 正规出场统计：Type2Missing（第二类闭环 still-MISSING，见 [`ExitDecision`] docstring）配对出场信号数（诚实标注）。
    pub n_exit_type2_missing: usize,
    /// P7 正规出场统计：Hold（无正规卖点 bit 的反向信号出场）配对出场信号数。
    pub n_exit_hold: usize,
}

impl SpreadAttribution {
    /// L2 否证判据（**adverse-only 保守口径**）：执行损耗（含成本）是否吃光结构价差。
    ///
    /// `true` ⟺ Σcaptured ≤ 0。注意：captured 用 max(0,·) 丢弃有利滑移 ⟹ 系统性低估真实 PnL，
    /// 这是**压力测试**口径，不等于真实成交。真实成交口径见 `actual_pnl_proxy`/`actual_pnl_eaten`。
    pub fn spread_eaten(&self) -> bool {
        self.sum_captured <= 0.0
    }

    /// 真实成交 PnL 代理（664-Q3 codex）：Σactual_spread − ΣCe = Σδ(Pτout−Pτin) − ΣCe。
    /// 含全部滑移（有利+不利），是确认 bar 实际成交价差减成本——比 adverse-only captured 更接近真实成交。
    pub fn actual_pnl_proxy(&self) -> f64 {
        self.sum_actual_spread - self.sum_ce
    }

    /// 真实成交口径的「执行吃光」判据：actual_pnl_proxy ≤ 0。
    ///
    /// 与 `spread_eaten`（adverse-only）的分歧即 codex Q3 关注点：若 `spread_eaten`=true 但本判据=false，
    /// 则「执行滞后吃光」是 adverse-only 伪结论（有利滑移被丢弃所致），真实成交其实可捕获。
    pub fn actual_pnl_eaten(&self) -> bool {
        self.actual_pnl_proxy() <= 0.0
    }
}
