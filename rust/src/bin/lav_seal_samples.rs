//! #1167 S1：封存固定历史样本集（30 样本 × bundle + Comparison Set Manifest + Lead 基线）。
//!
//! 用法（仓库根）：
//!
//! ```text
//! cargo run --features lav_seal --bin lav_seal_samples -- [repo-root] [out-dir]
//! ```
//!
//! - `repo-root`：仓库根目录（默认 `.`），读取 `chanlun/review-results/*.md` 历史评审产物；
//! - `out-dir`：封存输出目录（默认 `target/lav-seal-out`）。
//!
//! 输出：每样本 2 份 bundle manifest（JCS 规范化 JSON）+ 1 份 Comparison Set Manifest +
//! 1 份 Lead 基线（先封存）；`summary.json` 汇总全部 bundle_id / comparison_set_id /
//! baseline_digest。摘要即「封存承诺」，随后才允许揭示基线明文。

use std::path::{Path, PathBuf};

use newchan_rust::lav_seal::samples::{seal_all, validate_spec, SAMPLES};

fn main() {
    let mut args = std::env::args().skip(1);
    let repo_root: PathBuf = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let out_dir: PathBuf = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/lav-seal-out"));

    match run(&repo_root, &out_dir) {
        Ok(summary) => {
            println!("{}", serde_json::to_string_pretty(&summary).unwrap());
            println!("OK: 30 样本封存完成，输出目录 {}", out_dir.display());
        }
        Err(e) => {
            eprintln!("FAIL: {e}");
            std::process::exit(1);
        }
    }
}

fn run(repo_root: &Path, out_dir: &Path) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    if let Err(e) = validate_spec(SAMPLES) {
        return Err(format!("sample spec invalid: {e}").into());
    }
    let repo_root = std::fs::canonicalize(repo_root)?;
    let sealed = seal_all(&repo_root)?;

    std::fs::create_dir_all(out_dir)?;
    let mut summary = serde_json::json!({
        "schema": "lav-seal-summary-v1",
        "ticket": "#1167",
        "sample_count": sealed.len(),
        "samples": [],
    });

    let mut entries = Vec::new();
    for s in &sealed {
        let sample_dir = out_dir.join(s.sample_id);
        std::fs::create_dir_all(&sample_dir)?;
        for bundle in &s.bundles {
            let path = sample_dir.join(format!("bundle-{}.json", bundle.manifest.candidate_id));
            write_canonical(&path, &bundle.manifest)?;
        }
        write_canonical(&sample_dir.join("comparison-set.json"), &s.set.manifest)?;
        write_canonical(
            &sample_dir.join("lead-baseline-sealed.json"),
            &s.baseline.baseline,
        )?;
        entries.push(serde_json::json!({
            "sample_id": s.sample_id,
            "class": s.class.as_str(),
            "bundle_ids": s.bundles.iter().map(|b| b.bundle_id.clone()).collect::<Vec<_>>(),
            "comparison_set_id": s.set.comparison_set_id,
            "baseline_digest": s.baseline.baseline_digest,
            "held_out": s.baseline.baseline.selected_candidate_id,
        }));
    }
    summary["samples"] = serde_json::Value::Array(entries);
    write_canonical(&out_dir.join("summary.json"), &summary)?;
    Ok(summary)
}

/// 把 manifest 按 JCS 规范化字节写到磁盘（写临时 → rename 原子发布）。
fn write_canonical<T: serde::Serialize>(
    path: &Path,
    value: &T,
) -> Result<(), Box<dyn std::error::Error>> {
    let json_value = serde_json::to_value(value)?;
    let canonical = newchan_rust::lav_seal::jcs::canonicalize(&json_value);
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, canonical.as_bytes())?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}
