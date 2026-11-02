//! #1374：受控已发生经济事实的 E/B 独立持久 owner（SPEC #1340 C09）。
//! 无交易决策、资金分配策略或外效；未知本金与阶段不构造默认值。
mod allocation;
mod book;
mod content_store;
// #1768：B/E 域事件外壳与 snapshot/delta 读口为 crate 内只读接口；wire op 接线待主控裁定，
// 生产构建暂无调用方，故只在非测试构建豁免 dead_code。
#[cfg_attr(not(test), allow(dead_code))]
mod domain_event;
mod evidence;
mod finance;
mod finance_transport;
mod financial_reader_resources;
mod phase_trace;
mod quantity_calibration;
mod service;
mod sr_expansion;
mod store;
mod store_read;
mod tb03c_allocation;
mod tb03c_book;
mod tb03c_corrections;
mod tb03c_e;
mod tb03c_funding;
mod tb03c_per_leg;
mod tb03c_transfer;
mod voice;
pub use service::cli;

#[cfg(test)]
mod domain_event_tests;
#[cfg(test)]
mod quantity_calibration_tests;
#[cfg(test)]
mod tb03c_allocation_tests;
#[cfg(test)]
mod tb03c_book_tests;
#[cfg(test)]
mod tb03c_e_tests;
#[cfg(test)]
mod tb03c_funding_tests;
#[cfg(test)]
mod voice_tests;

mod financial_content;

#[cfg(all(test, target_os = "macos"))]
mod tb03c_fin2_tests;

#[cfg(test)]
mod tb03c_c5_frozen_tests;
#[cfg(test)]
mod tb03c_c5_read_tests;
#[cfg(test)]
mod tb03c_c5_tests;
#[cfg(test)]
mod tb03c_chain_tests;
#[cfg(test)]
mod tb03c_kill_tests;
#[cfg(test)]
mod tb03c_rf01_tests;
#[cfg(test)]
mod tb03c_v2_kill_tests;
