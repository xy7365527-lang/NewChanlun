//! 回测指标层——权益曲线 + 交易列表 → 协议 §3 全套指标。
//!
//! ## 认识论等级：**L1**（确定性变换，零信息增量）
//!
//! 给定权益曲线/交易序列，算 CAGR/Sharpe/... 是确定性数值变换——验证指标算法正确，
//! **不验证 Θ 有效**（formalization-validity-domain 231号）。指标的 **L2** 价值只在喂入
//! **真实历史数据驱动的引擎产出**的权益曲线时才产生（届时指标可否证 Θ）。
//!
//! ## 协议锚点（docs/backtest-protocol-v0.md §3，全部预注册，全部报告）
//!
//! 不允许只报有利指标。本模块计算 §3 列出的**全部**指标，调用方一次性输出。
//!
//! ## 实现诚实声明
//!
//! - **统计显著性**（§3.4 Sharpe 标准误 Lo 2002 调整 / block bootstrap p 值）和**随机
//!   入场对照**（§4，seed=20260625）是**独立的统计检验步骤**，依赖完整交易序列与
//!   per-trade 收益分布。本模块提供基础指标（CAGR/Sharpe/...）；bootstrap/随机对照在
//!   runner 层组织（需要引擎产出真实订单流后才有意义，属**等引擎阻塞点**）。
//! - 年化因子按"每年交易日"近似——日聚合 returns 的年化用 √252（标准约定）。各品种实际
//!   交易日数在 runner 层按数据日历精确化（§3.1"年化因子按各品种实际交易日"）。

/// 协议 §3 全套指标（单个检验窗）。每个字段对应 §3 一行。
#[derive(Debug, Clone, PartialEq)]
pub struct Metrics {
    // ── §3.1 收益与风险调整 ──
    /// 几何年化收益（扣成本后）。
    pub cagr: f64,
    /// 年化 Sharpe（rf=0，日 returns 年化）。
    pub sharpe: f64,
    /// 年化 Sortino（下行标准差）。
    pub sortino: f64,
    /// 最大回撤（峰谷，正数，0..1）。
    pub max_drawdown: f64,
    /// Calmar = CAGR / |MaxDD|。
    pub calmar: f64,
    /// CVaR(5%)：5% 尾部条件期望亏损（日 returns，负数表亏损）。
    pub cvar_5: f64,

    // ── §3.2 交易行为 ──
    /// 完整开平仓次数。
    pub n_trades: usize,
    /// 胜率 = 盈利交易 / 总交易。
    pub win_rate: f64,
    /// 盈亏比 = 平均盈利 / |平均亏损|。
    pub profit_factor: f64,
    /// top-5 交易盈亏和 / 总盈亏（§5.2 失败判据：>50% 收益过度集中）。
    pub top5_pnl_ratio: f64,

    // ── 基线对照（§4 Buy&Hold；随机/方向打乱在 runner 层）──
    /// 策略总收益率（期末权益/初始-1）。
    pub strat_return: f64,
    /// buy&hold 总收益率（末 close/首 close-1）。
    pub bh_return: f64,
}

/// 计算全套指标。
///
/// 输入：
/// - `equity_curve`：逐 bar 权益（归一化，初始=1.0），与回测 bar 序列同长。
/// - `trade_pnls`：每笔完整平仓交易的盈亏（归一化口径）。
/// - `bars_per_year`：年化因子分母（日 returns 数 / 年；§3.1 按各品种实际交易日）。
///   1min bar 聚合到日 returns 后，标准约定 √252——此处传日 returns 数对应的年化基数。
/// - `bh_return`：buy&hold 收益率（runner 层算，传入对照）。
///
/// **边界条件**：`equity_curve` 空或全平（无交易）⇒ 收益/风险指标为 0，盈亏比/胜率为 0
/// （NaN 防御，§5.5 由 runner 判 inconclusive）。
pub fn compute(
    equity_curve: &[f64],
    daily_returns: &[f64],
    trade_pnls: &[f64],
    years: f64,
    bh_return: f64,
) -> Metrics {
    let strat_return = if let (Some(&first), Some(&last)) =
        (equity_curve.first(), equity_curve.last())
    {
        if first > 0.0 {
            last / first - 1.0
        } else {
            0.0
        }
    } else {
        0.0
    };

    // CAGR：几何年化。years<=0 或权益<=0 ⇒ 0。
    let cagr = if years > 0.0 && strat_return > -1.0 {
        (1.0 + strat_return).powf(1.0 / years) - 1.0
    } else {
        0.0
    };

    let (sharpe, sortino) = sharpe_sortino(daily_returns);
    let max_drawdown = max_drawdown(equity_curve);
    let calmar = if max_drawdown > 1e-12 {
        cagr / max_drawdown
    } else {
        0.0
    };
    let cvar_5 = cvar(daily_returns, 0.05);

    let (win_rate, profit_factor) = win_rate_profit_factor(trade_pnls);
    let top5_pnl_ratio = top_n_pnl_ratio(trade_pnls, 5);

    Metrics {
        cagr,
        sharpe,
        sortino,
        max_drawdown,
        calmar,
        cvar_5,
        n_trades: trade_pnls.len(),
        win_rate,
        profit_factor,
        top5_pnl_ratio,
        strat_return,
        bh_return,
    }
}

/// 年化 Sharpe + Sortino（rf=0）。日 returns 序列 → 均值/标准差 → ×√252 年化。
fn sharpe_sortino(daily_returns: &[f64]) -> (f64, f64) {
    let n = daily_returns.len();
    if n < 2 {
        return (0.0, 0.0);
    }
    let mean = daily_returns.iter().sum::<f64>() / n as f64;
    // 样本标准差（n-1）。
    let var = daily_returns
        .iter()
        .map(|r| (r - mean).powi(2))
        .sum::<f64>()
        / (n - 1) as f64;
    let std = var.sqrt();
    // 下行标准差（仅负 returns，相对 0 目标）。
    let downside_var = daily_returns
        .iter()
        .filter(|&&r| r < 0.0)
        .map(|r| r.powi(2))
        .sum::<f64>()
        / (n - 1) as f64;
    let downside_std = downside_var.sqrt();

    const ANN: f64 = 252.0;
    let sharpe = if std > 1e-12 {
        mean / std * ANN.sqrt()
    } else {
        0.0
    };
    let sortino = if downside_std > 1e-12 {
        mean / downside_std * ANN.sqrt()
    } else {
        0.0
    };
    (sharpe, sortino)
}

/// 最大回撤（峰谷，正数，基于权益曲线）。
fn max_drawdown(equity: &[f64]) -> f64 {
    let mut peak = f64::MIN;
    let mut max_dd = 0.0;
    for &e in equity {
        if e > peak {
            peak = e;
        }
        if peak > 0.0 {
            let dd = (peak - e) / peak;
            if dd > max_dd {
                max_dd = dd;
            }
        }
    }
    max_dd
}

/// CVaR(α)：最差 α 分位尾部的条件期望（日 returns）。返回负数（亏损）。
fn cvar(daily_returns: &[f64], alpha: f64) -> f64 {
    let n = daily_returns.len();
    if n == 0 {
        return 0.0;
    }
    let mut sorted: Vec<f64> = daily_returns.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    // 尾部样本数（至少 1）。
    let k = ((n as f64) * alpha).ceil().max(1.0) as usize;
    let tail = &sorted[..k.min(n)];
    tail.iter().sum::<f64>() / tail.len() as f64
}

/// 胜率 + 盈亏比。
fn win_rate_profit_factor(pnls: &[f64]) -> (f64, f64) {
    if pnls.is_empty() {
        return (0.0, 0.0);
    }
    let wins: Vec<f64> = pnls.iter().copied().filter(|&p| p > 0.0).collect();
    let losses: Vec<f64> = pnls.iter().copied().filter(|&p| p < 0.0).collect();
    let win_rate = wins.len() as f64 / pnls.len() as f64;
    let avg_win = if wins.is_empty() {
        0.0
    } else {
        wins.iter().sum::<f64>() / wins.len() as f64
    };
    let avg_loss = if losses.is_empty() {
        0.0
    } else {
        losses.iter().sum::<f64>() / losses.len() as f64
    };
    let profit_factor = if avg_loss.abs() > 1e-12 {
        avg_win / avg_loss.abs()
    } else {
        0.0
    };
    (win_rate, profit_factor)
}

/// top-N 交易盈亏和 / 总盈亏（§5.2 收益集中度）。总盈亏≤0 ⇒ 0（防御）。
fn top_n_pnl_ratio(pnls: &[f64], n: usize) -> f64 {
    let total: f64 = pnls.iter().sum();
    if total <= 1e-12 {
        return 0.0;
    }
    let mut sorted: Vec<f64> = pnls.to_vec();
    sorted.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal)); // 降序
    let top_sum: f64 = sorted.iter().take(n).sum();
    top_sum / total
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 空权益/无交易 → 全 0（不 NaN，§5.5 inconclusive 防御）。
    #[test]
    fn empty_inputs_yield_zero_metrics() {
        let m = compute(&[], &[], &[], 1.0, 0.0);
        assert_eq!(m.cagr, 0.0);
        assert_eq!(m.sharpe, 0.0);
        assert_eq!(m.n_trades, 0);
        assert_eq!(m.win_rate, 0.0);
        assert_eq!(m.profit_factor, 0.0);
        assert_eq!(m.top5_pnl_ratio, 0.0);
    }

    /// max_drawdown 峰谷正确（已知曲线）。
    #[test]
    fn max_drawdown_known_curve() {
        // 1 → 2 → 1（峰 2，谷 1，回撤 50%）→ 1.5。
        let dd = max_drawdown(&[1.0, 2.0, 1.0, 1.5]);
        assert!((dd - 0.5).abs() < 1e-9, "峰 2 谷 1 = 50% 回撤");
    }

    /// CVaR(5%) 取最差尾部均值（负数）。
    #[test]
    fn cvar_picks_worst_tail() {
        // 20 个 returns，最差 1 个（5%）= -0.10。
        let mut rets = vec![0.01; 19];
        rets.push(-0.10);
        let c = cvar(&rets, 0.05);
        assert!((c - (-0.10)).abs() < 1e-9, "5% 尾部 = 最差 1 个 = -0.10");
    }

    /// 胜率 + 盈亏比已知值。
    #[test]
    fn win_rate_and_profit_factor_known() {
        // 3 笔：+2, +4, -3 ⇒ 胜率 2/3；avg_win=3, avg_loss=3 ⇒ pf=1.0。
        let (wr, pf) = win_rate_profit_factor(&[2.0, 4.0, -3.0]);
        assert!((wr - 2.0 / 3.0).abs() < 1e-9);
        assert!((pf - 1.0).abs() < 1e-9, "avg_win 3 / avg_loss 3 = 1.0");
    }

    /// top-5 占比（§5.2）：少于 5 笔时取全部。
    #[test]
    fn top5_ratio_concentration() {
        // 总 = 10；top-1 = 8 ⇒ top5（不足5取全）= 10/10 = 1.0。
        let r = top_n_pnl_ratio(&[8.0, 1.0, 1.0], 5);
        assert!((r - 1.0).abs() < 1e-9);
        // 总 = 10；top-2 = 8+1=9 ⇒ 0.9。
        let r2 = top_n_pnl_ratio(&[8.0, 1.0, 0.5, 0.5], 2);
        assert!((r2 - 0.9).abs() < 1e-9);
    }

    /// Sharpe 符号：正均值 → 正 Sharpe；零波动 → 0（防 NaN）。
    #[test]
    fn sharpe_sign_and_zero_vol() {
        let (s, _) = sharpe_sortino(&[0.01, 0.02, 0.01, 0.015]);
        assert!(s > 0.0, "正均值 returns → 正 Sharpe");
        let (s0, so0) = sharpe_sortino(&[0.01, 0.01, 0.01]);
        assert_eq!(s0, 0.0, "零波动 → Sharpe 0（防 NaN）");
        assert_eq!(so0, 0.0, "无下行 → Sortino 0");
    }
}
