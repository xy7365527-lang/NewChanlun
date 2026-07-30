//! S_Θ `Order`（`StrictAction`）↔ Nautilus `OrderSide` + `order_factory` 适配（骨架）。
//!
//! ## 职责（设计文档 §4.3 第 3-4 行 + §4.4）
//!
//! S_Θ `plan_orders` 产 `Vec<Order>`（`action: StrictAction`, `qty: i64 lot`, `exec_index: usize`）。
//! 本适配器把每个 S_Θ Order 映射到一次 Nautilus 下单意图（`OrderSide` + reduce_only + 价格腿）：
//!
//! | StrictAction | Nautilus OrderSide | reduce_only | 注记 |
//! |--------------|-------------------|-------------|------|
//! | Buy / Add    | Buy               | false       | 开多/加多 |
//! | Sell         | Sell              | false       | 建空/翻空（v0 单声部 Long 根时不产；Short 根产） |
//! | Reduce       | (持仓反向)         | true        | 减仓 |
//! | Close        | (持仓反向)         | true        | 平仓（全平） |
//! | Hold / Wait  | —                 | —           | 不下单 |
//!
//! ## ★骨架（nautilus 依赖未加，不编译）：真实接口锚以注释 + TODO 标注。
//!
//! 真实 Nautilus 下单（context7 `orders/limit.md`，待依赖后兑现）：
//! ```ignore
//! let order = self.core.order_factory().limit(
//!     instrument_id, side, Quantity::from(qty), Price::new(px, precision),
//!     Some(TimeInForce::Gtc), None, post_only, Some(reduce_only), ..);
//! self.submit_order(order, None, None)?;
//! ```

use crate::theta_v0::types::{Order, StrictAction};

/// 下单方向（Nautilus `OrderSide` 的骨架镜像，避免依赖 `nautilus_model::enums`）。
///
/// ★诚实：占位 enum，依赖加入后**删除**，直接用 `nautilus_model::enums::OrderSide`（TODO）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderSideLike {
    Buy,
    Sell,
}

/// 持仓方向上下文（Close/Reduce 的反向 side 需要它——平多=Sell，平空=Buy）。
///
/// 由 [`super::account_adapter`] 从 Nautilus portfolio `net_position` 推出（is_net_long/short）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PositionDir {
    Long,
    Short,
    Flat,
}

/// 一次 Nautilus 下单意图（骨架中间表示，order_adapter 产出 → strategy 提交）。
///
/// ★诚实：这不是 Nautilus `Order`——是骨架的**意图载荷**，strategy.rs 用它调真实
/// `order_factory().limit/market`（TODO）。`price_tick=None` ⟹ 市价单（紧急出场）；
/// `Some` ⟹ 限价单（LMT only 策略主路径，旧调研 §4.5）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderIntent {
    pub side: OrderSideLike,
    pub qty: i64,
    pub reduce_only: bool,
    /// 限价（S_Θ 整数 tick；strategy.rs 经 bar_adapter::tick_to_price_f64 → Nautilus Price）。
    /// `None` ⟹ 市价（紧急/止损触发出场）。
    pub price_tick: Option<crate::theta_v0::types::Tick>,
    /// S_Θ 的执行延迟成交 bar（exec_index）——Nautilus 中由 venue 撮合实现，此字段仅供
    /// 诊断/对账（生产路径不自己模拟延迟，见设计文档 §4.4）。
    pub exec_index: usize,
}

/// S_Θ `Order` → Nautilus 下单意图 [`OrderIntent`]（设计文档 §4.3 映射表）。
///
/// `pos_dir`：当前持仓方向（Close/Reduce 的反向 side 依赖它）。`entry_tick`：限价腿价格
/// （从 `VoiceDecision.entry` 取，strategy.rs 传入；市价腿传 `None`）。
///
/// 返回 `None` 当 action ∈ {Hold, Wait}（不下单）/ qty<=0（spec:47 不交易）/ Close|Reduce 但
/// 当前 Flat（无仓可平，诚实跳过非错误）。
pub fn to_order_intent(
    order: &Order,
    pos_dir: PositionDir,
    entry_tick: Option<crate::theta_v0::types::Tick>,
) -> Option<OrderIntent> {
    if order.qty <= 0 {
        return None; // spec:47 qty<=0 不交易
    }
    match order.action {
        StrictAction::Buy | StrictAction::Add => Some(OrderIntent {
            side: OrderSideLike::Buy,
            qty: order.qty,
            reduce_only: false,
            price_tick: entry_tick,
            exec_index: order.exec_index,
        }),
        StrictAction::Sell => Some(OrderIntent {
            // 建空/翻空（开仓侧 Short 根）。注：S_Θ Sell 在 apply_order 中也兼作「平多」语义
            // （runner.rs:630-633），但生产路径平仓走 Close（reduce_only），Sell 专指建空开仓侧。
            side: OrderSideLike::Sell,
            qty: order.qty,
            reduce_only: false,
            price_tick: entry_tick,
            exec_index: order.exec_index,
        }),
        StrictAction::Close | StrictAction::Reduce => {
            // 平仓/减仓：方向 = 持仓反向（平多=Sell，平空=Buy）。Flat ⟹ 无仓可平（None）。
            let side = match pos_dir {
                PositionDir::Long => OrderSideLike::Sell,
                PositionDir::Short => OrderSideLike::Buy,
                PositionDir::Flat => return None, // 无仓可平（诚实跳过）
            };
            Some(OrderIntent {
                side,
                qty: order.qty,
                reduce_only: true,
                // 平仓骨架先用市价（None）——结构止损价的限价平仓待与 strategy 退出生成器对齐（TODO）。
                price_tick: None,
                exec_index: order.exec_index,
            })
        }
        StrictAction::Hold | StrictAction::Wait => None, // 不下单
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_order(action: StrictAction, qty: i64) -> Order {
        Order {
            action,
            qty,
            exec_index: 1,
        }
    }

    /// Buy → OrderSide::Buy，非 reduce，带限价。
    #[test]
    fn buy_maps_to_buy_side_limit() {
        let oi = to_order_intent(
            &mk_order(StrictAction::Buy, 10),
            PositionDir::Flat,
            Some(100),
        )
        .unwrap();
        assert_eq!(oi.side, OrderSideLike::Buy);
        assert_eq!(oi.qty, 10);
        assert!(!oi.reduce_only);
        assert_eq!(oi.price_tick, Some(100));
    }

    /// Close 持多 → Sell + reduce_only。
    #[test]
    fn close_long_maps_to_sell_reduce_only() {
        let oi =
            to_order_intent(&mk_order(StrictAction::Close, 5), PositionDir::Long, None).unwrap();
        assert_eq!(oi.side, OrderSideLike::Sell);
        assert!(oi.reduce_only);
    }

    /// Close 持空 → Buy + reduce_only。
    #[test]
    fn close_short_maps_to_buy_reduce_only() {
        let oi =
            to_order_intent(&mk_order(StrictAction::Close, 5), PositionDir::Short, None).unwrap();
        assert_eq!(oi.side, OrderSideLike::Buy);
        assert!(oi.reduce_only);
    }

    /// Close 但 Flat ⟹ None（无仓可平）。
    #[test]
    fn close_flat_yields_none() {
        assert!(
            to_order_intent(&mk_order(StrictAction::Close, 5), PositionDir::Flat, None).is_none()
        );
    }

    /// Hold/Wait ⟹ 不下单。
    #[test]
    fn hold_wait_yield_none() {
        assert!(
            to_order_intent(&mk_order(StrictAction::Hold, 5), PositionDir::Long, None).is_none()
        );
        assert!(
            to_order_intent(&mk_order(StrictAction::Wait, 5), PositionDir::Flat, None).is_none()
        );
    }

    /// qty<=0 ⟹ 不交易（spec:47）。
    #[test]
    fn nonpos_qty_yields_none() {
        assert!(to_order_intent(
            &mk_order(StrictAction::Buy, 0),
            PositionDir::Flat,
            Some(100)
        )
        .is_none());
    }
}
