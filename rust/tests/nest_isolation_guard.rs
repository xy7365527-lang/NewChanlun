//! 公理级守卫：判据层禁止引用 nest 产物（依赖方向单向锁定）。
//!
//! 依据：chanlun/escalate/cert-bsp-binding-ruling-DRAFT-20260717.md（v2）
//!   - 条款 9：区间套必要条件不得由外挂观测器（nest 管线/打标产物）代实现；
//!   - 待裁 2（主人 2026-07-17 批准）：判据 crate 对 nest 产物的 import 禁令
//!     升为编译期/CI 约束——理由为公理级（递归封闭性），非工程洁癖。
//!   - #449 §2 裁定（2026-07-27）：根子 = 禁令说的是「层」，旧守卫查的是「目录」，
//!     而 nest 三件本体（nest.rs/nest_index.rs/turn_class.rs）住在判据目录 `classifier/`
//!     里，目录 ≠ 层；旧版按文件名白名单打补丁盖住这个错配。#451 按本裁定把豁免机制
//!     从「文件名」改成「模块自述角色」。
//!
//! 允许的依赖方向：声明 `GUARD-ROLE: nest-pipeline` 的模块 → 塔构件（level_view /
//! recursive_tower / types）。
//! 禁止的依赖方向：未声明该角色的判据生产代码（如 `interval_necessity.rs`）→ nest
//! 类型或打标产物。
//!
//! ## 角色声明怎么读（本轮改造核心）
//!
//! 守卫扫描每个 `.rs` 文件的**内容**，找形如
//! ```text
//! //! GUARD-ROLE: nest-pipeline
//! ```
//! 的一整行（必须逐字匹配，不含额外文字）。命中 ⟹ 该文件视为 nest 管线本体，豁免扫描。
//! 未命中（含完全不声明、或声明为别的值如 `judge`）⟹ 照常扫描，禁入模式命中即红。
//!
//! 当前声明 `nest-pipeline` 的三件：`nest.rs`（独立对照实现本体）、`nest_index.rs`
//! （生产装配+查表，不持判定）、`turn_class.rs`（对账本只读派生）——均为 #449 §2 点名的
//! nest 管线部件。`interval_necessity.rs` 显式声明 `judge`（自述记录，不产生豁免效果，
//! 见该文件模块头），继续被拦，这是 #451 的硬性要求。
//!
//! ## 已知弱点（显式登记，不掩盖——#449 §2 原文）
//!
//! **自述可骗**：豁免判据是模块自己写的一行注释，不是任何独立于模块本身的事实来源。
//! 任何模块只要把这行 `GUARD-ROLE: nest-pipeline` 抄进自己的文档注释，就能让判据代码
//! 逃出本守卫的扫描——守卫不核实自述是否属实，只认字面存在。这不是本轮修复的疏漏，是
//! 编排者裁定书原文就承认的代价：
//!
//! > 物理搬家（把 nest 三件移出 `classifier/`，让目录 = 层）是唯一自执行的做法，
//! > 因代价未估被否于成本而非原理；若日后出现第二例自称逃逸，此案应重开。
//!
//! 换句话说：本守卫比旧版（文件名白名单）更贴合「角色」语义，但**不比旧版更难绕过**——
//! 旧版靠改文件名逃逸，新版靠改一行注释逃逸，两者都是「守卫信任扫描对象的自我描述」这
//! 一类弱点的具体形式。唯一根治法是把 nest 三件搬出判据目录，本轮未做，成本未估。
//!
//! 豁免（与角色声明正交，独立生效）：
//!   - `#[cfg(test)]` 之后的测试代码（仓库约定：测试模块位于文件尾部）；
//!   - 行注释；mod.rs 的 `pub mod nest;` 模块挂载。
//!
//! 违规即测试红——修法只有三条：把逻辑挪进已声明 `nest-pipeline` 的模块、去掉违规引用，
//! 或来找主人改裁定（不是自己在目标文件里加一行 GUARD-ROLE 了事——那正是上面登记的弱点，
//! 出现即视为需要重开 #449 该案，不是本守卫的正常使用方式）。

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

/// 角色声明必须逐字匹配的整行标记（前后 trim 空白后比较）。
const NEST_PIPELINE_ROLE_MARKER: &str = "//! GUARD-ROLE: nest-pipeline";

/// 读模块自述角色：整行逐字匹配 `NEST_PIPELINE_ROLE_MARKER` 才算 nest-pipeline。
/// 不做子串匹配、不认文件名——这是 #451 要求的「守卫读声明，不读白名单」。
fn declares_nest_pipeline_role(src: &str) -> bool {
    src.lines().any(|line| line.trim() == NEST_PIPELINE_ROLE_MARKER)
}

#[test]
fn classifier_production_code_never_imports_nest_artifacts() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/theta_v0/classifier");
    let mut violations: Vec<String> = Vec::new();
    let mut scanned = 0usize;
    let mut exempted_as_nest_pipeline: Vec<String> = Vec::new();

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
        let src = fs::read_to_string(&path).expect("读源文件失败");

        if declares_nest_pipeline_role(&src) {
            exempted_as_nest_pipeline.push(fname);
            continue; // 模块自述 nest 管线本体，豁免（弱点见文件头注释）
        }

        scanned += 1;
        // 仓库约定：测试模块在文件尾部；#[cfg(test)] 起豁免。
        let prod = match src.find("#[cfg(test)]") {
            Some(pos) => &src[..pos],
            None => src.as_str(),
        };
        for (i, line) in prod.lines().enumerate() {
            let t = line.trim_start();
            if t.starts_with("//") {
                continue; // 注释豁免（含 //!  文档注释，含本文件自己写的 GUARD-ROLE 行）
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
        "扫描文件数异常（{scanned} ≤ 10，豁免掉 {} 个：{:?}）——目录结构变了？\
         或者角色声明被滥用来批量逃逸？守卫本身需要检修",
        exempted_as_nest_pipeline.len(),
        exempted_as_nest_pipeline
    );
    assert!(
        violations.is_empty(),
        "判据层引用了 nest 产物（公理违规，见 cert-bsp-binding-ruling-DRAFT-20260717.md v2 条款9/\
         待裁2 + #449 §2 裁定）：\n{}",
        violations.join("\n")
    );
}
