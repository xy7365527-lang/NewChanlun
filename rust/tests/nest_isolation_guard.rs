//! 公理级守卫：判据层禁止引用 nest 产物（依赖方向单向锁定）。
//!
//! 依据：chanlun/escalate/cert-bsp-binding-ruling-DRAFT-20260717.md（v2）
//!   - 条款 9：区间套必要条件不得由外挂观测器（nest 管线/打标产物）代实现；
//!   - 待裁 2（主人 2026-07-17 批准）：判据 crate 对 nest 产物的 import 禁令
//!     升为编译期/CI 约束——理由为公理级（递归封闭性），非工程洁癖。
//!
//! 允许的依赖方向：nest.rs → 塔构件（level_view / recursive_tower / types）。
//! 禁止的依赖方向：判据生产代码（classifier/ 除 nest.rs 外）→ nest 类型或打标产物。
//!
//! 豁免：
//!   - nest.rs 自身（独立对照实现）；
//!   - `#[cfg(test)]` 之后的测试代码（仓库约定：测试模块位于文件尾部）；
//!   - 行注释；mod.rs 的 `pub mod nest;` 模块挂载。
//!
//! 违规即测试红——修法只有两条：把逻辑挪进 nest 侧，或来找主人改裁定。

use std::fs;
use std::path::Path;

/// 禁入模式：nest 证书类型 + 打标产物标识。
const FORBIDDEN: &[&str] = &[
    "NestCertificate",
    "NestRung",
    "NestInterval",
    "::nest::",
    "use super::nest",
    "use super::super::nest",
    "p101_cert_bsp_tag",
    "cert_bsp_tag",
];

#[test]
fn classifier_production_code_never_imports_nest_artifacts() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/theta_v0/classifier");
    let mut violations: Vec<String> = Vec::new();
    let mut scanned = 0usize;

    for entry in fs::read_dir(&dir).expect("读 classifier 目录失败") {
        let path = entry.expect("目录项").path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let fname = path
            .file_name()
            .and_then(|n| n.to_str())
            .expect("文件名")
            .to_string();
        if fname == "nest.rs" {
            continue; // 独立对照实现，豁免
        }
        scanned += 1;
        let src = fs::read_to_string(&path).expect("读源文件失败");
        // 仓库约定：测试模块在文件尾部；#[cfg(test)] 起豁免。
        let prod = match src.find("#[cfg(test)]") {
            Some(pos) => &src[..pos],
            None => src.as_str(),
        };
        for (i, line) in prod.lines().enumerate() {
            let t = line.trim_start();
            if t.starts_with("//") {
                continue; // 注释豁免（含 //!  文档注释）
            }
            if t.starts_with("pub mod nest") || t.starts_with("mod nest") {
                continue; // mod.rs 模块挂载豁免
            }
            for pat in FORBIDDEN {
                if line.contains(pat) {
                    violations.push(format!("{}:{}: [{}] {}", fname, i + 1, pat, line.trim()));
                }
            }
        }
    }

    assert!(
        scanned > 10,
        "扫描文件数异常（{scanned} ≤ 10）——目录结构变了？守卫本身需要检修"
    );
    assert!(
        violations.is_empty(),
        "判据层引用了 nest 产物（公理违规，见 cert-bsp-binding-ruling-DRAFT-20260717.md v2 条款9/待裁2）：\n{}",
        violations.join("\n")
    );
}
