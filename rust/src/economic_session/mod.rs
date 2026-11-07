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

/// 测试专用：写出桩脚本并设为可执行。内容先在本进程写到暂存文件，再由 `/bin/cp` 子进程复制成
/// 真正被执行的文件：本进程从不持有该文件的写句柄，并行测试线程 fork 出的子进程就不会继承它，
/// 执行时也就不会因 ETXTBSY（文本文件忙）失败。
/// 不要改成 `std::fs::copy` 或直接 `std::fs::write`：两者都在本进程打开目标文件写入，竞态会原样回来。
#[cfg(test)]
pub(crate) fn write_test_executable(path: &std::path::Path, body: &str, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    let staged = path.with_file_name(format!(
        "{}.staged",
        path.file_name().unwrap().to_str().unwrap()
    ));
    std::fs::write(&staged, body).unwrap();
    let copied = std::process::Command::new("/bin/cp")
        .arg(&staged)
        .arg(path)
        .status()
        .unwrap();
    std::fs::remove_file(&staged).unwrap();
    assert!(copied.success(), "复制桩脚本失败：{copied}");
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}
