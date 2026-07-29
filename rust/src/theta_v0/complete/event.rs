//! 外部事件 `e_{t+1}`（8 元组）的 Rust 实装——契约锚 `Origin.CompleteStateEvent.ExternalEvent`
//! （FULL 结果包 §20 line 1416-1429 boxed）。
//!
//! ## 工位定位（cov-rust-impl D7 事件仅价格笔 补全）
//!
//! cov-rust-impl 报告「事件仅价格/笔」的根源：闭环引擎的事件字母表是 [`super::super::closed_loop::
//! state::MicroEvent`]（仅 `NewBar`/`NewStroke` 两类——纯价格/笔的在线增量），**未承载** §20 的成交/
//! 拒单/手续费/资金费/保证金/借券/公司行为 7 类外部事件。本模块把 §20 的完整 8 元组**逐分量显式化**
//! （零遗漏），与 Lean `Origin.CompleteStateEvent.ExternalEvent` 逐字段对齐。
//!
//! ## 与 `MicroEvent` 的关系（诚实声明，非补丁）
//!
//! `MicroEvent`（NewBar/NewStroke）**不被替换或删除**——它是闭环引擎的**内部在线增量**字母表（驱动
//! Origin `parse` 的前缀窗口推进），是 §20 `bar` 分量（`y_{t+1}`）经 parser 导出的**结构识别事件**。
//! 本 [`ExternalEvent`] 是 §20 **完整外部输入** schema：`bar` 是市场行情（`MicroEvent::NewBar` 由它
//! 导出），其余 7 元组（Fill/Reject/Fee/Funding/MarginUpdate/BorrowUpdate/CorpAction）是 `MicroEvent`
//! **从未承载**的经纪/会计/公司行为外部输入。关系 = 外部事件（本模块）→ 微事件（MicroEvent）是一个
//! 投影（取 `bar` 分量经 parser 产出微事件），反之不能（微事件丢失了 7 类经纪事件）。
//!
//! ## 认识论等级（formalization-validity-domain 231号，强制标注）
//!
//! - 本模块 = **L0**（纯结构 schema 镜像：8 元组结构与 Lean `ExternalEvent` 逐字段对齐 = FULL §20 文字
//!   ↦ Rust 类型，零信息增量）。`cargo build` 绿 = 类型自洽，**不**是事件处理逻辑的有效声明。

use super::super::types::Bar;

/// 成交回报 `Fill`（契约锚 `Origin.CompleteStateEvent.Fill`，FULL §20 line 1422）。
///
/// `Option<Fill>`（见 [`ExternalEvent::fill`]）：`None`=本步无成交；`Some`=成交价×量（整数域）。
/// ★§20 line 1447：「成交作为外部事件输入以后，下一状态才唯一」——`Fill` 是状态唯一性的依赖输入
/// （策略只唯一决定订单 `O`，不决定成交）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fill {
    pub price: i64,
    pub qty: i64,
}

/// 拒单 `Reject`（契约锚 `Origin.CompleteStateEvent.Reject`，FULL §20 line 1423）。
/// `rejected=true` 表示挂单被拒（`order_ref` 为被拒订单标识）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reject {
    pub rejected: bool,
    pub order_ref: i64,
}

/// 手续费 `Fee`（契约锚 `Origin.CompleteStateEvent.Fee`，FULL §20 line 1424）。
/// 本步产生的手续费（整数域，进账本 Π 的成本侧）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fee {
    pub amount: i64,
}

/// 资金费 `Funding`（契约锚 `Origin.CompleteStateEvent.Funding`，FULL §20 line 1425）。
/// 持仓资金费（**永续合约** funding，正负皆可）——契约锚语义不动。
/// ★venue 适用性（#303 裁定 2026-07-26）：本仓 BTC 数据窗 = Binance **现货**（无资金费）。
/// 本类型当前**无生产构造点**（唯二构造：[`ExternalEvent::price_only`] 的 `amount: 0`，与
/// `complete::mod` 的模块测试 `amount: -1`），
/// 回测侧持有成本走另一条路径 `strategy::risk::CostModel`（现货口径 = 资金占用机会成本 + 现货
/// 杠杆借币 + 强平罚金）。真永续接入是另票（#62 datum + datum 版本管理）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Funding {
    pub amount: i64,
}

/// 保证金更新 `MarginUpdate`（契约锚 `Origin.CompleteStateEvent.MarginUpdate`，FULL §20 line 1426）。
/// 经纪侧保证金占用变化（更新 `ν.margin_used`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarginUpdate {
    pub new_margin_used: i64,
}

/// 借券更新 `BorrowUpdate`（契约锚 `Origin.CompleteStateEvent.BorrowUpdate`，FULL §20 line 1427）。
/// 借券可得性/成本变化（更新 `ν.borrowable`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BorrowUpdate {
    pub borrowable: bool,
    pub borrow_cost: i64,
}

/// 公司行为 `CorpAction`（契约锚 `Origin.CompleteStateEvent.CorpAction`，FULL §20 line 1428）。
///
/// 拆股/分红/合并等改变价格连续性与持仓数量的外部事件。逐子分量：`split_num`/`split_den`（拆股比
/// 分子/分母）、`dividend`（每单位分红）。「无行为」用 `(1,1,0)` 表示（拆股比 1:1、零分红）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CorpAction {
    pub split_num: i64,
    pub split_den: i64,
    pub dividend: i64,
}

impl CorpAction {
    /// 「无公司行为」实例（拆股比 1:1、零分红）——每步缺省值。
    pub fn none_action() -> CorpAction {
        CorpAction { split_num: 1, split_den: 1, dividend: 0 }
    }
}

/// 外部事件 `e_{t+1}`（8 元组逐分量，契约锚 `Origin.CompleteStateEvent.ExternalEvent`，FULL §20 boxed）。
///
/// 字段与 Lean `ExternalEvent` 逐字段对齐（见 Lean 模块的分量对照表）：
/// 1. `bar`（`y_{t+1}` 下一行情）/ 2. `fill`（`Option`，成交决定下一状态唯一性）/ 3. `reject` /
/// 4. `fee` / 5. `funding` / 6. `margin_update` / 7. `borrow_update` / 8. `corp_action`。
///
/// ★`fill` 用 `Option`（§20 line 1447：策略发单后成交未知）；其余每步都有值（无值时取「零」实例
/// 如 `Fee { amount: 0 }` / `CorpAction::none_action()`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExternalEvent {
    /// 1. `y_{t+1}` 下一行情（FULL line 1421）。
    pub bar: Bar,
    /// 2. `Fill_{t+1}` 成交回报（FULL line 1422；`None`=无成交）。
    pub fill: Option<Fill>,
    /// 3. `Reject_{t+1}` 拒单（FULL line 1423）。
    pub reject: Reject,
    /// 4. `Fee_{t+1}` 手续费（FULL line 1424）。
    pub fee: Fee,
    /// 5. `Funding_{t+1}` 资金费（FULL line 1425）。
    pub funding: Funding,
    /// 6. `MarginUpdate_{t+1}` 保证金变化（FULL line 1426）。
    pub margin_update: MarginUpdate,
    /// 7. `BorrowUpdate_{t+1}` 借券变化（FULL line 1427）。
    pub borrow_update: BorrowUpdate,
    /// 8. `CorpAction_{t+1}` 公司行为（FULL line 1428）。
    pub corp_action: CorpAction,
}

impl ExternalEvent {
    /// 纯行情事件（仅 `bar`，其余 7 元组取零/缺省）——对应 cov-rust-impl D7「事件仅价格/笔」的
    /// 旧覆盖范围在 8 元组中的子集嵌入：这是把旧「仅价格笔」事件提升为完整 8 元组时的退化构造
    /// （新引擎不丢失旧能力，旧价格事件 = 完整事件的 `fill=None ∧ 零费用 ∧ 无公司行为` 切片）。
    pub fn price_only(bar: Bar) -> ExternalEvent {
        ExternalEvent {
            bar,
            fill: None,
            reject: Reject { rejected: false, order_ref: -1 },
            fee: Fee { amount: 0 },
            funding: Funding { amount: 0 },
            margin_update: MarginUpdate { new_margin_used: 0 },
            borrow_update: BorrowUpdate { borrowable: true, borrow_cost: 0 },
            corp_action: CorpAction::none_action(),
        }
    }
}
