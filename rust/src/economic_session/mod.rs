//! #1374：受控已发生经济事实的 E/B 独立持久 owner（SPEC #1340 C09）。
//! 无交易决策、资金分配策略或外效；未知本金与阶段不构造默认值。
mod book;
mod evidence;
mod voice;
mod service;
mod store;
pub use service::cli;

#[cfg(test)]
mod voice_tests;
