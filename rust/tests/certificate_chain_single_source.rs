//! #1225 证书链形式化收口：候选谓词/严档谓词单源机械锁。
//!
//! 这是语法门，不替代行为对拍：行为由 `certificate_chain_parity.json` 的 Rust↔Lean 测试锁；
//! 本门只禁止在 canonical 文件之外再定义一份同名谓词，或在 `chain_lookup` 里恢复内联严档判定。

use std::fs;
use std::path::{Path, PathBuf};

fn collect_files(root: &Path, extension: &str, out: &mut Vec<PathBuf>) {
    for entry in
        fs::read_dir(root).unwrap_or_else(|error| panic!("无法读取 {}: {error}", root.display()))
    {
        let path = entry.expect("目录项可读").path();
        if path.is_dir() {
            collect_files(&path, extension, out);
        } else if path.extension().and_then(|value| value.to_str()) == Some(extension) {
            out.push(path);
        }
    }
}

fn occurrences(files: &[PathBuf], needle: &str) -> Vec<String> {
    let mut hits = Vec::new();
    for path in files {
        let source = fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("无法读取 {}: {error}", path.display()));
        for (index, line) in source.lines().enumerate() {
            if line.contains(needle) {
                hits.push(format!("{}:{}", path.display(), index + 1));
            }
        }
    }
    hits
}

#[test]
fn candidate_and_strict_predicates_have_one_canonical_implementation() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("rust/ 父目录 = 仓库根");

    let mut lean_files = Vec::new();
    collect_files(&repo.join("formal/Origin"), "lean", &mut lean_files);
    for (needle, expected_suffix) in [
        ("def DivCand (", "formal/Origin/NestingCertificate.lean"),
        ("def Cand (", "formal/Origin/NestingCertificate.lean"),
        ("def Strict (", "formal/Origin/NestingCertificate.lean"),
    ] {
        let hits = occurrences(&lean_files, needle);
        assert_eq!(hits.len(), 1, "{needle:?} 必须恰有一份实现，实际 {hits:?}");
        assert!(
            hits[0].contains(expected_suffix),
            "{needle:?} canonical 必须在 {expected_suffix}，实际 {:?}",
            hits[0]
        );
    }

    let mut rust_files = Vec::new();
    collect_files(&repo.join("rust/src"), "rs", &mut rust_files);
    let strict_defs = occurrences(&rust_files, "fn strict_chain_verdict(");
    assert_eq!(
        strict_defs.len(),
        1,
        "Rust 严档裁决函数必须恰有一份实现，实际 {strict_defs:?}"
    );
    assert!(strict_defs[0].contains("theta_v0/backtest/admission.rs"));

    let admission = fs::read_to_string(repo.join("rust/src/theta_v0/backtest/admission.rs"))
        .expect("admission.rs 可读");
    assert_eq!(
        admission.matches("strict_chain_verdict(&levels)").count(),
        1,
        "chain_lookup 必须且只能委托严档单源一次"
    );
    assert_eq!(
        admission.matches("n_closed == levels.len()").count(),
        1,
        "全闭合比较只能留在 strict_chain_verdict 函数体内"
    );
}
