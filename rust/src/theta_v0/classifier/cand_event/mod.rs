//! #550 塔内原生背驰段候选事件。
//!
//! 本模块只产出、存储候选生命史，不接 BSP/admission/订单消费点。候选事实在
//! `classify_impl` 的逐级循环内，由与 BSP 共用的 `signal::first_structural_gates` 结构门产出；
//! 力度只影响 BSP bit，不进入候选身份、状态或任一事件字段（#551 起本模块在类型层够不到 MACD 序列）。
//!
//! ## 按域拆分（#634；来源 = 拆分前的单文件 `classifier/cand_event.rs`，1417 行）
//!
//! 三域各一文件、各带 `mod tests`；全部 pub 项经本文件原样 re-export ⟹ 外部
//! `cand_event::X` 路径逐字不变，下游（`classifier/mod.rs`、`chain_cert/`、`cand_sub`、
//! 校验 bin）零改动：
//!
//! - [`key`]：身份键、状态、结构谓词、事件形状、闭区间几何判据、载荷投影（纯类型与判据，
//!   无状态无外部调用）；
//! - [`book`]：每级 append-only 修订簿 [`CandidateEventBook`] + 生命史覆盖探针 [`event_probe`]
//!   （修订协议、三只钟、终态不复活、幂等跳过）；
//! - [`observe`]：单次塔扫描 → [`CandidateObservation`] 的产出侧（结构域结构门投影 + Pan 域
//!   证书投影 + 同 episode 归约）。
//!
//! `event_probe` 的 hook 调用点是 `#[cfg(test)]`（生产零开销），故拆分**必须留在本 crate 内**
//! ——迁到 `rust/tests/` 独立 crate 会让这些断言静默失效（#551 小修批实测，#576 评论在案）。
//!
//! 三域测试共用的夹具集中在 [`fixtures`]（`#[cfg(test)]`，不带 `#[test]` ⟹ 不进测试名清单）。

mod book;
mod key;
mod observe;

#[cfg(test)]
mod fixtures;

pub use book::{event_probe, CandidateEventBook};
pub use key::{
    interval_is_degenerate, interval_is_sub, intervals_are_disjoint, intervals_touch,
    CandidateEvent, CandidateKey, CandidateKind, CandidateProjection, CandidateState,
    CandidateStreams, ObservedState, ParentFingerprint, StructuralPredicates,
    CANDIDATE_RULE_VERSION, FNV_OFFSET_BASIS, FNV_PRIME,
};
pub(crate) use observe::observations_for_level;
pub use observe::CandidateObservation;
// `structural_observations_for_level` / `pan_observations_for_level` 在拆分前是
// `cand_event::<名>` 的 crate 内可见路径（实际消费者只有同文件 `mod tests`；#634 实测
// `cand_event/` 外零引用）。拆分后这两个 tests 迁到 `book`/`observe` 的域内 `mod tests`，
// 走域内路径消费 ⟹ 本层 re-export 无消费者。**仍保留**是为守住「pub 项原样导出、可见面
// 逐字不变」这条验收条款（#359 四件套②）——缩面属行为变化，不在本票范围内。
#[allow(unused_imports)]
pub(crate) use observe::{pan_observations_for_level, structural_observations_for_level};
