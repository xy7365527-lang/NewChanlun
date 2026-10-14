//! #1374：受控已发生经济事实的 E/B 独立持久 owner（SPEC #1340 C09）。
//! 无交易决策、资金分配策略或外效；未知本金与阶段不构造默认值。
mod allocation;
mod book;
mod content_store;
mod evidence;
mod finance;
mod finance_transport;
mod financial_reader_resources;
mod service;
mod store;
mod voice;
pub use service::cli;

#[cfg(test)]
mod tb03c_book_tests;
#[cfg(test)]
mod voice_tests;

mod financial_content;
