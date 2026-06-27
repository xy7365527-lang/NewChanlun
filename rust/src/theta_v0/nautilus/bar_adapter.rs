//! Nautilus `Bar` ↔ S_Θ `types::Bar` 适配（骨架，整数 tick 域往返）。
//!
//! ## 职责（设计文档 §4.3 第 1 行）
//!
//! S_Θ 在**整数 tick 域**工作（`types::quantize(price, tick_size)` 是引擎边界唯一 f64→i64 转换点，
//! 见 `types.rs:20-26`）。Nautilus `Bar` 用 `Price`（定点 f64 语义）。本适配器：
//! - **入**：Nautilus `Bar`（f64 OHLC + ts_event）→ `types::Bar`（i64 Tick + untradable 判定）。
//! - **出**：S_Θ 价格（Tick）→ Nautilus `Price`（dequantize via tick_size），供 order_adapter 用。
//!
//! ## ★骨架（nautilus 依赖未加，不编译）：真实接口锚以注释 + TODO 标注，不 `use nautilus_*`。
//!
//! 真实 Nautilus 类型（context7 `nautilus_model::data::Bar` / `types::Price`）：
//! - `Bar` 字段：`open/high/low/close: Price`、`volume: Quantity`、`ts_event: UnixNanos`。
//! - `Price::as_f64()` 取 f64；`Price::new(value, precision)` 构造。

use crate::theta_v0::types::{quantize, Bar, Timestamp};

/// Nautilus Bar 的最小载荷（占位结构体，避免依赖 `nautilus_model`）。
///
/// ★诚实：这**不是** Nautilus 的 `Bar`——它是骨架阶段的**字段镜像占位**，让 [`from_nautilus_bar`]
/// 的签名 + 逻辑可表达。依赖加入后**删除此占位**，直接吃 `&nautilus_model::data::Bar`（TODO）。
/// 字段对齐 Nautilus `Bar`（open/high/low/close: f64 via Price::as_f64；volume: f64；ts_event: i64 ns）。
#[derive(Debug, Clone, Copy)]
pub struct NautilusBarLike {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
    pub ts_event: i64,
}

/// Nautilus Bar → S_Θ `types::Bar`（quantize 到整数 tick 域）。
///
/// `source_index`：策略内累积 bar 窗口的序号（由调用方 [`super::strategy`] 维护，单调递增）——
/// S_Θ 用它做平局裁决 + exec 延迟起点（`types::Bar.tie_key` + `fill_bar_index`，spec:16/50）。
/// `tick_size`：来自 `ThetaConfig.tick.tick_size`（quantize 分母，bit-exact 第一原理）。
///
/// ## untradable 判定（spec:53，types.rs:46-47）
///
/// 缺 OHLC / `high < max(open,close,low)` / `low > min(open,close,high)` / volume=0 ⟹ untradable。
/// **halt/limit flag** 需 venue 层标记（Nautilus 不在 Bar 内携带）——骨架先判 OHLC 完整性 +
/// volume，halt/limit 待接 venue instrument 状态（TODO，标注非缺陷是有效域边界）。
///
/// 边界条件：`tick_size <= 0` 在 config 校验层已拒（types.rs:23 假设已校验 > 0）。
pub fn from_nautilus_bar(nb: &NautilusBarLike, source_index: usize, tick_size: f64) -> Bar {
    let open = quantize(nb.open, tick_size);
    let high = quantize(nb.high, tick_size);
    let low = quantize(nb.low, tick_size);
    let close = quantize(nb.close, tick_size);
    let volume = nb.volume as i64;

    // untradable：OHLC 完整性（spec:53）。halt/limit 待 venue 状态（TODO）。
    let untradable = volume <= 0
        || high < open.max(close).max(low)
        || low > open.min(close).min(high);

    Bar {
        source_index,
        timestamp: nb.ts_event as Timestamp,
        open,
        high,
        low,
        close,
        volume,
        untradable,
    }
}

/// S_Θ 价格（Tick）→ Nautilus `Price` 的 f64 值（dequantize，order_adapter 构造 `Price` 用）。
///
/// 还原：`price_f64 = tick * tick_size`（quantize 的逆）。Nautilus 侧再 `Price::new(value, precision)`
/// 按 instrument price_precision 构造（precision 由 instrument 定，骨架返回 f64 由 order_adapter 接）。
pub fn tick_to_price_f64(tick: crate::theta_v0::types::Tick, tick_size: f64) -> f64 {
    tick as f64 * tick_size
}

#[cfg(test)]
mod tests {
    use super::*;

    /// L1（待依赖后真验）：quantize 往返一致性骨架占位。
    ///
    /// ★诚实：此测试用占位 `NautilusBarLike`（非真实 Nautilus Bar），故是 **L0 占位**（验证
    /// quantize 调用形状），**不是** L1（L1 需真实 Nautilus Bar 往返）。依赖加入后升级为吃真实
    /// `nautilus_model::data::Bar` 的往返测试（TODO）。
    #[test]
    fn from_nautilus_bar_quantizes_to_tick_domain() {
        let nb = NautilusBarLike {
            open: 100.0,
            high: 110.0,
            low: 95.0,
            close: 105.0,
            volume: 1000.0,
            ts_event: 42,
        };
        let bar = from_nautilus_bar(&nb, 7, 1.0); // tick_size=1.0 ⟹ tick=美元
        assert_eq!((bar.open, bar.high, bar.low, bar.close), (100, 110, 95, 105));
        assert_eq!(bar.source_index, 7);
        assert_eq!(bar.timestamp, 42);
        assert!(!bar.untradable, "完整 OHLC + volume>0 ⟹ tradable");
    }

    /// untradable 判定：volume=0 ⟹ untradable（spec:53）。
    #[test]
    fn zero_volume_marks_untradable() {
        let nb = NautilusBarLike {
            open: 100.0, high: 110.0, low: 95.0, close: 105.0, volume: 0.0, ts_event: 1,
        };
        let bar = from_nautilus_bar(&nb, 0, 1.0);
        assert!(bar.untradable, "volume=0 ⟹ untradable");
    }

    /// dequantize 还原：tick × tick_size = 美元价。
    #[test]
    fn tick_to_price_roundtrips() {
        assert_eq!(tick_to_price_f64(105, 1.0), 105.0);
        // 1e-8 tick：100_000_000 tick × 1e-8 = 1.0 美元。
        assert!((tick_to_price_f64(100_000_000, 1e-8) - 1.0).abs() < 1e-9);
    }
}
