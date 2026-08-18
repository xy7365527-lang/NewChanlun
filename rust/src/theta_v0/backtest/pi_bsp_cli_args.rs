//! `pi_bsp_timing` CLI 参数显式解析（#412 修复核心）——从 bin 抽出为 lib 模块，
//! 回归测试锁放进**默认测试套件**（`cargo test --lib` 无 feature 即跑）。
//!
//! 成因（#412 重做要求）：首版修复把测试文件放 `src/bin/` 下——cargo 自动发现为二进制目标
//! （E0433 `super` 越级 + E0601 缺 `main`），且未加 `required-features` 门控 ⟹
//! `cargo check --all-targets` 直接转红，已从 main 回退。本模块化后测试随 lib 编译，
//! 任何破坏都会在默认 CI 跑红，不再有「验证静默没发生」。

/// 命令行参数解析结果（#412：窗口是否切片提升为显式字段，杜绝「传了 THETA 就静默丢窗口」）。
#[derive(Debug, PartialEq)]
pub struct CliArgs {
    pub symbol: String,
    pub window: Option<(String, String)>,
    pub theta: f64,
    pub z_alpha: f64,
}

pub const USAGE: &str = "<SYMBOL> [START_DATE END_DATE [THETA [Z_ALPHA]]]\n  SYMBOL: BTC/ES/CL/GC/BRN/DX/QQQ/OKLO\n  THETA: χ 阈值 θ（默认 0；负值趋近全覆盖，验收边界）\n  Z_ALPHA: LCB 置信分位（默认 0）";

/// 解析单个 `Θ_risk` 浮点参数（THETA / Z_ALPHA 共用）——静默必须变响亮：
/// 解析失败直接报错返回，不做 `unwrap_or(0.0)` 式的静默默认值退化。
pub fn parse_theta_like(field_name: &str, raw: &str) -> Result<f64, String> {
    raw.parse::<f64>()
        .map_err(|e| format!("{field_name} 解析失败（{raw:?}）: {e}"))
}

/// 显式按位置解析（#412 修复核心）：`window` 只取决于 `START_DATE END_DATE` 是否被传入，
/// 与是否额外传了 THETA/Z_ALPHA **无关**——不再用 `args.len()==4` 这种「参数数量决定语义」
/// 的魔数判据（旧判据在 THETA 存在时 `args.len()==5`，导致窗口切片无声跳过，是 #412 根因）。
/// 不认识的参数个数一律报错返回 `Err`（响亮失败），不做静默降级。
pub fn parse_cli_args(args: &[String]) -> Result<CliArgs, String> {
    if args.len() < 2 {
        return Err(format!("用法: pi_bsp_timing {USAGE}"));
    }
    let symbol = args[1].clone();
    let window = |args: &[String]| Some((args[2].clone(), args[3].clone()));
    match args.len() {
        2 => Ok(CliArgs { symbol, window: None, theta: 0.0, z_alpha: 0.0 }),
        3 => Err(format!(
            "参数数量不匹配：给了 START_DATE 但缺 END_DATE。用法: pi_bsp_timing {USAGE}"
        )),
        4 => Ok(CliArgs { symbol, window: window(args), theta: 0.0, z_alpha: 0.0 }),
        5 => {
            let theta = parse_theta_like("THETA", &args[4])?;
            Ok(CliArgs { symbol, window: window(args), theta, z_alpha: 0.0 })
        }
        6 => {
            let theta = parse_theta_like("THETA", &args[4])?;
            let z_alpha = parse_theta_like("Z_ALPHA", &args[5])?;
            Ok(CliArgs { symbol, window: window(args), theta, z_alpha })
        }
        n => Err(format!(
            "参数数量不匹配（收到 {} 个，SYMBOL 之外只接受 0/2/3/4 个）。用法: pi_bsp_timing {USAGE}",
            n - 1
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造完整 argv（含程序名 argv[0]，与 bin 内 `std::env::args()` 形态一致）。
    fn argv(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn symbol_only_means_full_dataset() {
        let cli = parse_cli_args(&argv(&["pi_bsp_timing", "OKLO"])).unwrap();
        assert_eq!(cli.symbol, "OKLO");
        assert_eq!(cli.window, None);
        assert_eq!(cli.theta, 0.0);
        assert_eq!(cli.z_alpha, 0.0);
    }

    #[test]
    fn window_without_theta_slices() {
        let cli = parse_cli_args(&argv(&[
            "pi_bsp_timing",
            "OKLO",
            "2026-06-01",
            "2026-06-24",
        ]))
        .unwrap();
        assert_eq!(
            cli.window,
            Some(("2026-06-01".to_string(), "2026-06-24".to_string()))
        );
    }

    /// ★#412 原 bug 现象的镜像复现锁：带 THETA 时窗口切片仍生效（旧实现 `args.len()==4`
    /// 判据在 THETA 存在时 len==5 ⟹ 静默跳过切片、退化成跑全量）。
    #[test]
    fn window_plus_theta_still_slices() {
        let cli = parse_cli_args(&argv(&[
            "pi_bsp_timing",
            "OKLO",
            "2026-06-01",
            "2026-06-24",
            "0.0",
        ]))
        .unwrap();
        assert!(
            cli.window.is_some(),
            "带 THETA 时窗口切片不得被静默跳过（#412 根因）"
        );
        assert_eq!(cli.theta, 0.0);
    }

    /// 副产品修复锁：原 `args.len()!=2 && !=4 && !=5` 校验把 len==6（含 Z_ALPHA）直接拒绝
    /// ——Z_ALPHA 参数此前是死代码；显式解析后首次可达。
    #[test]
    fn window_plus_theta_plus_z_alpha_still_slices() {
        let cli = parse_cli_args(&argv(&[
            "pi_bsp_timing",
            "OKLO",
            "2026-06-01",
            "2026-06-24",
            "0.0",
            "1.96",
        ]))
        .unwrap();
        assert!(
            cli.window.is_some(),
            "带 THETA+Z_ALPHA 时窗口切片不得被静默跳过"
        );
        assert_eq!(cli.theta, 0.0);
        assert_eq!(cli.z_alpha, 1.96);
    }

    /// 响亮失败路径：参数缺失。
    #[test]
    fn missing_symbol_errors() {
        assert!(parse_cli_args(&argv(&["pi_bsp_timing"])).is_err());
        assert!(parse_cli_args(&[]).is_err());
    }

    /// 响亮失败路径：给了 START_DATE 但缺 END_DATE。
    #[test]
    fn start_without_end_errors() {
        let err = parse_cli_args(&argv(&["pi_bsp_timing", "OKLO", "2026-06-01"])).unwrap_err();
        assert!(err.contains("缺 END_DATE"), "err={err:?}");
    }

    /// 响亮失败路径：THETA 非法浮点（旧实现 `.unwrap_or(0.0)` 静默吞错）。
    #[test]
    fn invalid_theta_errors_loudly() {
        let err = parse_cli_args(&argv(&[
            "pi_bsp_timing",
            "OKLO",
            "2026-06-01",
            "2026-06-24",
            "not-a-float",
        ]))
        .unwrap_err();
        assert!(err.contains("THETA 解析失败"), "err={err:?}");
    }

    /// 响亮失败路径：Z_ALPHA 非法浮点。
    #[test]
    fn invalid_z_alpha_errors_loudly() {
        let err = parse_cli_args(&argv(&[
            "pi_bsp_timing",
            "OKLO",
            "2026-06-01",
            "2026-06-24",
            "0.0",
            "not-a-float",
        ]))
        .unwrap_err();
        assert!(err.contains("Z_ALPHA 解析失败"), "err={err:?}");
    }

    /// 响亮失败路径：参数过多。
    #[test]
    fn too_many_args_errors() {
        let err = parse_cli_args(&argv(&[
            "pi_bsp_timing",
            "OKLO",
            "2026-06-01",
            "2026-06-24",
            "0.0",
            "1.96",
            "extra",
        ]))
        .unwrap_err();
        assert!(err.contains("参数数量不匹配"), "err={err:?}");
    }
}
