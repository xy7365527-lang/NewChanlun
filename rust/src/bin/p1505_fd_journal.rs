//! #1505 D2 普通hot-journal组件原型；本机禁止运行。独立feature，绝不接生产admission。
#![deny(unsafe_op_in_unsafe_fn)]
#[path = "p1505_fd_journal/allocator.rs"]
mod allocator;
#[path = "p1505_fd_journal/driver.rs"]
mod driver;
#[path = "p1505_fd_journal/overlay.rs"]
mod overlay;
#[path = "p1505_fd_journal/vfs.rs"]
mod vfs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    max_source_main_bytes: u64,
    max_source_journal_bytes: u64,
    overlay_payload_capacity_bytes: u64,
    overlay_metadata_capacity_bytes: u64,
    evidence_capacity_bytes: u64,
    sqlite_heap_limit_bytes: u64,
}
impl Limits {
    fn validate(&self) -> vfs::Result<()> {
        let values = [
            self.max_source_main_bytes,
            self.max_source_journal_bytes,
            self.overlay_payload_capacity_bytes,
            self.overlay_metadata_capacity_bytes,
            self.evidence_capacity_bytes,
            self.sqlite_heap_limit_bytes,
        ];
        if values
            .iter()
            .any(|&x| x == 0 || x > i64::MAX as u64 || usize::try_from(x).is_err())
        {
            return Err("missing/zero/overflow test limit".into());
        }
        Ok(())
    }
    fn load(path: &std::path::Path) -> vfs::Result<Self> {
        let x: Self = serde_json::from_slice(&std::fs::read(path)?)?;
        x.validate()?;
        Ok(x)
    }
    fn exact_small_profile(&self) -> bool {
        self.max_source_main_bytes == 2_097_152
            && self.max_source_journal_bytes == 4_194_304
            && self.overlay_payload_capacity_bytes == 8_388_608
            && self.overlay_metadata_capacity_bytes == 1_048_576
            && self.evidence_capacity_bytes == 4_194_304
            && self.sqlite_heap_limit_bytes == 8_388_608
    }
}
fn main() {
    let args: Vec<_> = std::env::args_os().collect();
    let result=match args.get(1).and_then(|s|s.to_str()){
        Some("suite") if args.len()==4 => driver::suite(&PathBuf::from(&args[2]),&PathBuf::from(&args[3])),
        Some("writer") if args.len()==5=>driver::writer(&PathBuf::from(&args[2]),args[3].to_str().unwrap_or(""),args[4].to_str().unwrap_or("")),
        Some("oracle") if args.len()==4=>driver::oracle(&PathBuf::from(&args[2]),&PathBuf::from(&args[3])),
        Some("reader") if args.len()==6=>driver::reader(&PathBuf::from(&args[2]),&PathBuf::from(&args[3]),&PathBuf::from(&args[4]),args[5].to_str().unwrap_or("")),
        _=>Err("usage: p1505_fd_journal suite /absolute/new/evidence /absolute/explicit-limits.json (child modes are internal)".into()),
    };
    if let Err(e) = result {
        eprintln!("D2 FAILED; task fixtures retained: {e}");
        std::process::exit(1);
    }
}
