//! 三轴接口（operation_route_exhaustion §6B：级别间关系沿三轴正交分布）。
//!
//! 用户裁决（2026-06-17）：三轴**分开实装**，每轴用自己的穷尽工具，轴间**只通过信号接口耦合**
//! （trait），不直接互相调用内部状态。
//!
//! | 轴 | 内容 | 穷尽工具 |
//! |----|------|---------|
//! | **H⁰**（形态学） | 笔/段/中枢/走势的递归构成 | 形态学公理（递归定义） |
//! | **groupoid**（observe） | 信号确认+区间套定位 | 范畴极限（逆极限） |
//! | **H¹**（operate） | 操作路线（D∞ word） | 群论 word 枚举 |
//!
//! ## 分离边界 = operate ⊥ (morphology, observe)
//! H¹ 操作层通过 trait 读 H⁰/groupoid，不直接 reach into 信号层内部。morphology↔observe 的 confirm
//! 耦合（T49 向心读 type1_hist）是 §6B.4 内在依赖，内聚在信号层，对 operate 暴露为两个独立 view。
//!
//! ## 认识论等级
//! 三轴 trait 接口 = **L1**（架构表达力，bit-exact 与单体计算等价）。

use crate::spiral::signal::PendingLocate;
use crate::stroke::Direction;
use crate::trading::types::MAX_LADDER;

use super::layer::Layer;

/// **H⁰ 形态学轴**：笔/段/中枢/走势的递归构成（形态学公理穷尽）。当下的客观结构。
pub trait MorphologyAxis {
    /// 涌现级别上界（爬升封顶 = max_ladder+1）。
    fn emergent_ceiling(&self) -> usize;
    /// 该级别本 bar 是否有 type1 买点 confirmed（buy1）。
    fn buy1(&self, ladder: usize) -> bool;
    /// 该级别本 bar 是否有 type1 卖点 confirmed（sell1）。
    fn sell1(&self, ladder: usize) -> bool;
    /// 该级别方向态（走势包含方向，§6B.5 关系3；T5 涌现读数）。
    fn direction(&self, ladder: usize) -> Option<Direction>;
    /// 该级别方向锚 bar（段锚）。
    fn anchor(&self, ladder: usize) -> i64;
    /// 次级别 sub 的势幅度 θ（中枢/走势振幅参照，成本门 N4）。None=势不可测。
    fn theta(&self, sub: usize) -> Option<f64>;

    /// 当前根方向：最高活跃级别的走势方向（默认实现，从高向低扫 direction()）。
    /// 语义：root_dir = direction(r*(t))，r*(t) 是当前已涌现最高级别。
    /// 返回 None 当且仅当所有级别均无已形成走势（通常是行情启动前几 bar）。
    fn root_direction(&self) -> Option<Direction> {
        let ceiling = self.emergent_ceiling();
        (0..ceiling).rev().find_map(|k| self.direction(k))
    }
}

/// **groupoid 观察轴**：信号确认（向心回溯 T49）+ 区间套定位（逆极限，§6B.4）。
pub trait ObserveAxis {
    /// 自层卖侧向心 confirm fire（nf = 向心回溯贯通结果）。
    fn nf_sell(&self, ladder: usize) -> Option<f64>;
    /// 自层买侧向心 confirm fire。
    fn nf_buy(&self, ladder: usize) -> Option<f64>;
    /// 卖侧区间套链顶 source（C 清仓势源）。
    fn sell_source(&self) -> Option<usize>;
    /// 买侧区间套链顶 source（F 入场势源）。
    fn buy_source(&self) -> Option<usize>;
    /// 卖侧 located 链（区间套定位全貌；prove_chain/prove_t52 守卫读）。
    fn located_sell_chain(&self) -> &[Option<PendingLocate>; MAX_LADDER];
    /// 买侧 located 链。
    fn located_buy_chain(&self) -> &[Option<PendingLocate>; MAX_LADDER];
}

/// **H¹ 操作轴**：D∞ word 处理器（h/τ 原子，仓位在级别间 1/λ 流动）。
pub trait OperateAxis {
    /// 单 bar 操作步进。从 `h0`（H⁰）读形态学、`obs`（groupoid）读确认/定位，决定 h/τ word，
    /// 产出操作 + 反馈（`StepOutcome`，供 engine 协调 observe 清链）。
    fn step(
        &mut self,
        h0: &dyn MorphologyAxis,
        obs: &dyn ObserveAxis,
        bar: i64,
        price: f64,
    ) -> StepOutcome;
    /// 各级别仓位状态（层结构只读视图）。
    fn layers(&self) -> &[Layer];
}

/// operate.step 的本 bar 反馈（engine 据此协调 observe 清链；operate 不直接写 observe）。
///
/// ε 对称性（D∞ 双向 root）：Long root 消费买链入场/卖链清仓；Short root 消费卖链入场/买链清仓。
#[derive(Debug, Clone, Copy, Default)]
pub struct StepOutcome {
    /// Long root C 清仓消费的卖侧 source（Some ⇒ engine 清 observe located_sell）。
    pub cleared_sell_source: Option<usize>,
    /// Long root F 建仓消费的买侧 source（Some ⇒ engine 清 observe located_buy）。
    pub entered_buy_source: Option<usize>,
    /// Short root C 清仓消费的买侧 source（Some ⇒ engine 清 observe located_buy）。
    pub cleared_buy_source: Option<usize>,
    /// Short root F 建仓消费的卖侧 source（Some ⇒ engine 清 observe located_sell）。
    pub entered_sell_source: Option<usize>,
}
