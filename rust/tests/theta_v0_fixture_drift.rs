//! fixture 漂移 gate（`#[ignore]` 集成测试，GitHub issue #263，#245 裁定）。
//!
//! ## 工位定位
//!
//! 把 `scripts/check_fixture_drift.py` 包成 cargo 集成测试：它进 `cargo test` 全量编译但
//! **默认不跑**（`#[ignore]`，不影响现有绿测试），用 `-- --ignored` 单独触发。脚本 regen 两个
//! Lean 机器导出器（`formal/Origin/ParityFixtureExport.lean`、`formal/Origin/CenterConstruct.lean`
//! 的 `#eval`）到临时文件（绝不覆盖仓内 fixture），与 `rust/tests/fixtures/` 下
//! `theta_v0_parity.json` / `theta_v0_center_parity.json` 做字段级 diff——Lean 改了值但 fixture
//! 未重落盘 = 漂移，本测试红。
//!
//! ## 失败可辨识（#263：与真漂移严格区分，不同退出路径 + 不同报错文案）
//!
//! - 脚本 exit=1 ⟹ **真漂移**：panic 前缀 `FIXTURE DRIFT`，脚本已打出字段级差异
//!   （key path + 仓内值/regen 值）。
//! - 脚本 exit=3/4/5 ⟹ **环境失败（非漂移）**：panic 前缀 `FIXTURE-GATE ENVIRONMENT`
//!   （3=需要 Lean/Lake 工具链，lake 不在 PATH；4=`lake build` 失败；5=导出器运行失败）。
//!   无 lake 工具链或未构建环境走这条路，明确报「需要 lean 工具链，非漂移」。
//! - 其他退出码 ⟹ 脚本自身故障。
//!
//! ## 认识论（formalization-validity-domain 231号）
//!
//! L0：管线一致性 gate（Lean `#eval` 机器导出 ↔ 仓内 fixture 一致性），非 L2 行情有效断言。
//!
//! ## 跑法
//!
//! ```sh
//! cargo test --manifest-path rust/Cargo.toml --test theta_v0_fixture_drift -- --ignored --nocapture
//! ```
//!
//! 环境要求与实测耗时见 `scripts/check_fixture_drift.py` docstring（热路径 ≈3–7s；
//! `formal/.lake/build` 冷时脚本先 `lake build` 全量构建，2026-07-26 实测 12.2s）。python3 解释器可用
//! `FIXTURE_DRIFT_PYTHON` 环境变量覆盖（缺省 `python3`）。

use std::path::PathBuf;
use std::process::Command;

#[test]
#[ignore = "漂移 gate：需 Lean/Lake 工具链 + formal 可构建；-- --ignored 单独触发"]
fn fixture_drift_gate() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("rust/ 父目录 = 项目根")
        .to_path_buf();
    let script = root.join("scripts/check_fixture_drift.py");
    assert!(script.is_file(), "漂移脚本缺失: {}", script.display());

    let python = std::env::var("FIXTURE_DRIFT_PYTHON").unwrap_or_else(|_| "python3".to_string());
    let out = Command::new(&python)
        .arg(&script)
        .current_dir(&root)
        .output()
        .unwrap_or_else(|e| panic!("FIXTURE-GATE ENVIRONMENT（非漂移）：无法启动 {python}: {e}"));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    // 透传脚本输出（字段级 diff / 环境报错），--nocapture 下可见。
    print!("{stdout}");
    eprint!("{stderr}");

    match out.status.code() {
        Some(0) => {}
        Some(1) => panic!(
            "FIXTURE DRIFT（真漂移）：Lean 导出与仓内 fixture 不一致——见上文字段级 diff；\
             regen 重落 fixture 或回退 Lean 改动"
        ),
        Some(c @ (3 | 4 | 5)) => panic!(
            "FIXTURE-GATE ENVIRONMENT（非漂移，exit={c}）：需要 lean 工具链且 formal 可构建\
             （3=lake 不在 PATH / 4=lake build 失败 / 5=导出器运行失败）——修好环境后重跑，\
             勿当漂移处理"
        ),
        other => {
            panic!("FIXTURE-GATE 内部错误（exit={other:?}）：查 scripts/check_fixture_drift.py")
        }
    }
}
