//! #412 回归测试：`pi_bsp_timing` CLI 参数解析（`parse_cli_args`）。
//!
//! 拆到独立文件（`#[path]` 引入，见 `pi_bsp_timing.rs` 尾部）是为了不把主文件推过
//! coding-style.md 的 800 行硬顶——测试逻辑本身仍完全属于 `pi_bsp_timing` 这一个 bin，
//! 不违反「只动 `pi_bsp_timing.rs`（及其测试）」的改动范围。
use super::{parse_cli_args, CliArgs};

fn args(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// #412 回归核心：传窗口参数（START END）之后再带 THETA，窗口切片依然必须生效。
/// 旧实现用 `args.len()==4` 判断是否切片——THETA 一出现 `args.len()` 变 5，
/// 这条判据就假了，`dataset` 落到 `else` 分支跑全量。本测试锁死
/// 「窗口参数出现 ⟹ `window.is_some()`」这一条不变量，不再依赖参数总个数。
#[test]
fn window_plus_theta_still_slices() {
    let parsed =
        parse_cli_args(&args(&["pi_bsp_timing", "OKLO", "2026-06-01", "2026-06-24", "0.0"]))
            .expect("5 个参数应解析成功");
    assert_eq!(
        parsed.window,
        Some(("2026-06-01".to_string(), "2026-06-24".to_string())),
        "带 THETA 时窗口切片被静默丢弃——#412 复发"
    );
    assert_eq!(parsed.theta, 0.0);
}

/// 同上，THETA 之后再带 Z_ALPHA（6 个参数）——窗口切片仍必须生效。
#[test]
fn window_plus_theta_plus_z_alpha_still_slices() {
    let parsed = parse_cli_args(&args(&[
        "pi_bsp_timing",
        "OKLO",
        "2026-06-01",
        "2026-06-24",
        "0.05",
        "0.1",
    ]))
    .expect("6 个参数应解析成功");
    assert_eq!(parsed.window, Some(("2026-06-01".to_string(), "2026-06-24".to_string())));
    assert_eq!(parsed.theta, 0.05);
    assert_eq!(parsed.z_alpha, 0.1);
}

#[test]
fn symbol_only_has_no_window() {
    let parsed = parse_cli_args(&args(&["pi_bsp_timing", "OKLO"])).expect("2 个参数应解析成功");
    assert_eq!(
        parsed,
        CliArgs { symbol: "OKLO".to_string(), window: None, theta: 0.0, z_alpha: 0.0 }
    );
}

#[test]
fn window_without_theta_still_slices() {
    let parsed = parse_cli_args(&args(&["pi_bsp_timing", "OKLO", "2026-06-01", "2026-06-24"]))
        .expect("4 个参数应解析成功");
    assert!(parsed.window.is_some());
    assert_eq!(parsed.theta, 0.0);
}

/// 静默必须变响亮：参数数量不匹配（缺 SYMBOL）一律报错，不悄悄用默认值垫上。
#[test]
fn missing_symbol_errors() {
    assert!(parse_cli_args(&args(&["pi_bsp_timing"])).is_err());
}

/// 静默必须变响亮：只给 START 不给 END（3 个参数）一律报错，不悄悄退化成不切片。
#[test]
fn incomplete_window_errors() {
    assert!(parse_cli_args(&args(&["pi_bsp_timing", "OKLO", "2026-06-01"])).is_err());
}

/// 静默必须变响亮：多余的第 7 个参数一律报错，不悄悄忽略。
#[test]
fn too_many_args_errors() {
    assert!(parse_cli_args(&args(&[
        "pi_bsp_timing", "OKLO", "2026-06-01", "2026-06-24", "0.0", "0.1", "extra"
    ]))
    .is_err());
}

/// 静默必须变响亮：THETA 解析失败（非法浮点数）报错，不悄悄 `unwrap_or(0.0)` 吞掉。
#[test]
fn malformed_theta_errors() {
    let err = parse_cli_args(&args(&[
        "pi_bsp_timing",
        "OKLO",
        "2026-06-01",
        "2026-06-24",
        "not_a_number",
    ]))
    .unwrap_err();
    assert!(err.contains("THETA"), "错误信息应指明是 THETA 解析失败: {err}");
}

/// 静默必须变响亮：Z_ALPHA 解析失败（非法浮点数）报错，不悄悄吞掉。
#[test]
fn malformed_z_alpha_errors() {
    let err = parse_cli_args(&args(&[
        "pi_bsp_timing",
        "OKLO",
        "2026-06-01",
        "2026-06-24",
        "0.0",
        "not_a_number",
    ]))
    .unwrap_err();
    assert!(err.contains("Z_ALPHA"), "错误信息应指明是 Z_ALPHA 解析失败: {err}");
}
