//! master 出场判定 — C1 类型隔离（532号谱系的编译期形态）。
//!
//! 532号结算（L3，3/3 标的满分母）：master 出场判定与 voice 反向腿触发是
//! **两个概念**——前者保持 sell1，后者保留段终结三触发。master 消费段终结
//! 三触发（M-a 之源：−562.5/−39.0/−263.6 pp，暴露比 0.50）在 v2 中：
//!   (a) 类型上不可表示：`MasterExitSignal` 字段私有，唯一构造入口
//!       `from_sell1_row` 只接受 confirmed Sell1 布尔行——没有接受事件流的
//!       构造路径，"给 master 喂三触发"必须改类型签名 = 强制过谱系；
//!   (b) 模块依赖图上不可表达：本模块不 import `DivEvent` / `CenterEvent` /
//!       `BspEvent`——双重屏障。
//!
//! 数据源声明：Python oracle 的 master 触发 = `sig.sell1[entry_ladder]`
//! （BarSignalI 布尔行，语义 = "该 ladder 本 bar 新 confirmed type1 卖点"，
//! 见 `_scan_bsp_events` docstring 的 bit-exact 证明）。v2 设计 §C1 的
//! `from_events` 形态与此同义（confirmed Sell1 事件 ⇔ sell1 行置位）；
//! 本实现取布尔行形态以保 V0≡P5 逐位等价。

/// master 出场信号。本模块外不可构造。
pub struct MasterExitSignal {
    _priv: (),
}

impl MasterExitSignal {
    /// 唯一构造入口：policy 指定层（entry 或 earning 升级后的 exit_ladder）的
    /// confirmed Sell1 布尔行。层归属由调用方 runner 保证。
    pub fn from_sell1_row(sell1_at_policy_ladder: bool) -> Option<Self> {
        sell1_at_policy_ladder.then_some(MasterExitSignal { _priv: () })
    }

    /// sc_master_exit（次级别确认完整递归，2026-06-11）：sell1 行 ∧ 次级别
    /// 卖侧确认。触发类型仍是 sell1 单一来源——确认是 27课区间套对出场
    /// **时机**的细化，不是新触发源：532号类型隔离不被破坏（输入仍是布尔行，
    /// 没有事件流构造路径，"给 master 喂三触发"依旧不可表示）。
    pub fn from_sell1_row_sub_confirmed(sell1: bool, sub_confirm: bool) -> Option<Self> {
        Self::from_sell1_row(sell1 && sub_confirm)
    }
}

/// earning 反作用下 master 自身的相位（45课持股持币；K5 v1 §5.6 语义零改动）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MasterState {
    Ride,
    Rev,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_sell1_row_constructs_signal() {
        assert!(MasterExitSignal::from_sell1_row(true).is_some());
        assert!(MasterExitSignal::from_sell1_row(false).is_none());
    }
}
