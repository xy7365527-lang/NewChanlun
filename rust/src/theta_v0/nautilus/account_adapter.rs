//! Nautilus `portfolio`（net_position / PnL）↔ S_Θ `AccountState` 适配（骨架）。
//!
//! ## 职责（设计文档 §4.3 第 6-9 行 + §4.4 状态管理）
//!
//! S_Θ sizing（`risk::size_position`）吃 `AccountState{nav, voice_qty}`（`strategy/mod.rs:104`）。
//! 接 Nautilus 后，**持仓真相源移交 Nautilus portfolio**（消除 S_Θ 回测台账 vs 实盘账本双源）：
//! - `nav` ← Nautilus 账户净值（`portfolio.account(venue).balance` 或 net_exposure 推算）。
//! - `voice_qty[0]` ← `portfolio.net_position(instrument_id)` 的手数（v0 单声部 depth=0）。
//!
//! ## ★骨架（nautilus 依赖未加，不编译）：真实接口锚以注释 + TODO 标注。
//!
//! 真实 Nautilus portfolio（context7 `concepts/strategies.md`，待依赖后兑现）：
//! - `net_position(instrument_id) -> Decimal`、`is_net_long/short/flat(instrument_id) -> bool`。
//! - `unrealized_pnl`/`realized_pnl(instrument_id) -> Money`、`account(venue) -> Account`。

use crate::theta_v0::strategy::AccountState;

use super::order_adapter::PositionDir;

/// Nautilus portfolio 快照（骨架占位，避免依赖 `nautilus_*`）。
///
/// ★诚实：占位结构，依赖加入后**删除**，直接从 `self.core.portfolio()` 读真实值（TODO）。
/// 字段对齐 context7 portfolio API：`net_position`（Decimal→f64）、`nav`（账户净值美元）、
/// `realized_pnl`/`unrealized_pnl`（Money→f64，供 ledger R=Π−A−W 对账）。
#[derive(Debug, Clone, Copy)]
pub struct PortfolioSnapshot {
    /// 账户净值（美元，sizing 基数）。实盘取真实账户余额；回测取 venue starting_balance + PnL。
    pub nav: f64,
    /// 当前标的净仓手数（Nautilus `net_position`，正=多/负=空/0=平）。
    pub net_position: f64,
    /// 已实现盈亏（对账 S_Θ ledger 的 Realize 侧）。
    pub realized_pnl: f64,
    /// 未实现盈亏（mark-to-market；对账 S_Θ TW = free + holding）。
    pub unrealized_pnl: f64,
}

/// Nautilus portfolio 快照 → S_Θ `AccountState`（设计文档 §4.3 映射）。
///
/// `max_depth`：声部树最大深度（`config.voice.max_depth`）——v0 单声部 depth=0，故 `voice_qty`
/// 长度 = max_depth，仅 `[0]` 非零（= |net_position|）。多声部时按 depth 分配持仓待嵌套树扩展（TODO）。
///
/// ★诚实有效域：v0 classifier 只产 depth=0 独立根（`recognize` 注释 strategy/mod.rs:594），故
/// `net_position` 整体归 voice_qty[0]。多独立根并存（§5）时各根都是 depth=0，net_position 是它们的
/// **净和**——这与 S_Θ「多独立根各自 voice_qty」有口径差（Nautilus NETTING 单净仓 vs S_Θ 多根分账）。
/// 单根/单方向时无差异（v0 主路径）；多根对冲时需在适配层拆分账（标注非缺陷是有效域边界，TODO）。
pub fn to_account_state(snap: &PortfolioSnapshot, max_depth: u32) -> AccountState {
    let depth = max_depth.max(1) as usize;
    let mut voice_qty = vec![0u32; depth];
    voice_qty[0] = snap.net_position.abs() as u32; // v0 单声部 depth=0
    AccountState {
        nav: if snap.nav > 0.0 { snap.nav } else { 0.0 },
        voice_qty,
    }
}

/// 从 Nautilus net_position 推出持仓方向（order_adapter 的 Close/Reduce 反向 side 用）。
pub fn position_dir(snap: &PortfolioSnapshot) -> PositionDir {
    if snap.net_position > 0.0 {
        PositionDir::Long
    } else if snap.net_position < 0.0 {
        PositionDir::Short
    } else {
        PositionDir::Flat
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(nav: f64, net: f64) -> PortfolioSnapshot {
        PortfolioSnapshot {
            nav,
            net_position: net,
            realized_pnl: 0.0,
            unrealized_pnl: 0.0,
        }
    }

    /// net_position → voice_qty[0]，nav 透传。
    #[test]
    fn portfolio_maps_to_account_state() {
        let acct = to_account_state(&snap(1_000_000.0, 300.0), 4);
        assert_eq!(acct.nav, 1_000_000.0);
        assert_eq!(acct.voice_qty.len(), 4);
        assert_eq!(
            acct.voice_qty[0], 300,
            "v0 单声部 ⟹ net_position 归 voice_qty[0]"
        );
        assert_eq!(acct.voice_qty[1], 0);
    }

    /// 空仓 net_position → voice_qty 全 0 + Flat。
    #[test]
    fn flat_position() {
        let s = snap(500_000.0, 0.0);
        let acct = to_account_state(&s, 2);
        assert_eq!(acct.voice_qty, vec![0, 0]);
        assert_eq!(position_dir(&s), PositionDir::Flat);
    }

    /// 持空 net_position<0 → Short，voice_qty 取绝对值。
    #[test]
    fn short_position_dir_and_qty() {
        let s = snap(500_000.0, -50.0);
        assert_eq!(position_dir(&s), PositionDir::Short);
        assert_eq!(to_account_state(&s, 1).voice_qty[0], 50);
    }

    /// 负/零 nav 兜底为 0（防御性，sizing 用 0 NAV ⟹ qty=0 不产单，对齐 spec）。
    #[test]
    fn nonpos_nav_floored() {
        assert_eq!(to_account_state(&snap(-1.0, 0.0), 1).nav, 0.0);
    }
}
