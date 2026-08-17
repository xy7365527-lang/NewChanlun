//! 成立档门户类型（裁定八；S1 只做这一档）：中枢死亡证明 + 三类点成立证据包。
//!
//! 两型本体原居 [`super::book`]，票 #765（`book.rs` 破 800 顶格瘦身）纯移动迁入本模块，
//! 一个 bit 不改；开具点仍留账本本体（[`super::book::RetraceLedger::established`] /
//! [`super::book::RetraceLedger::established_pack`] / [`super::book::RetraceLedger::death_certificate`]）。

use super::{CenterFrame, RetraceKey, RetracePoint, RetraceSide};

/// 中枢死亡证明（裁定一：Success 事件即死亡证明，发中枢账登记 Broken）。
///
/// **`side` 字段（票 #664，闭合 #637 尾部方向盲区）**：证明本身携带杀路径方向——
/// 三买（`RetraceSide::Buy`，向上离开）/ 三卖（`RetraceSide::Sell`，向下离开）；
/// 取自落锤拍证据 [`super::RetraceEvidence::side`]（[`super::RetraceEntry::terminal_evidence`]），
/// 与 [`ThirdPointPack::side`] 同源同值。消费方（`CenterBook::consume_death_certificate`）
/// 据此登记 `dead_down` / `frozen` / 补发 `CenterEvent::Terminated`，不再对方向恒读 false。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CenterDeathCertificate {
    /// 中枢身份 = 快照转正后的四条边（临时右边成为永恒右边）。
    pub center: CenterFrame,
    /// 杀路径方向：向上离开（三买终结）/ 向下离开（三卖终结）。
    pub side: RetraceSide,
    /// **开具时间 = 落锤知情时**；与「中枢右边缘 = 快照右边」是两个时间，不混（裁定一）。
    pub issued_as_of: usize,
}

/// 成立档证据包（裁定八②：交易层「三类点成立」+ 全套证据）。
///
/// **两拍口径**（影子评审 #621 MEDIUM-2 如实声明）：`frame`/`identity` 取自**注册拍**（frame 是
/// 身份一部分），`side`/`leave_end`/`retest_end` 取自**落锤拍**（终态证据载荷）——同一证据包
/// 内部横跨两拍。真实引擎两拍同值（leave 笔已走完才有 departure，既成事实）；provider 两拍
/// 改口的一致性守卫（裁定二 fail-loud 同族）上浮 S2 待裁，S1 无守卫如实声明。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThirdPointPack {
    pub identity: RetraceKey,
    pub side: RetraceSide,
    /// 四条边快照（转正）。
    pub frame: CenterFrame,
    /// 「离开的边」= leave 笔终点。
    pub leave_end: RetracePoint,
    /// 三类点位置 = 回抽笔终点。
    pub retest_end: RetracePoint,
    /// 出生知情时。
    pub registered_as_of: usize,
    /// 落锤知情时。
    pub confirmed_as_of: usize,
    /// 死亡证明（不问迟到——054:60 延迟合法、#583 在案，照开）。
    pub death_certificate: CenterDeathCertificate,
}
