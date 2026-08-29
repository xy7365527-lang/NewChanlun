//! #1308 第二批：生产路径历史回放验收报告器（一键验收命令）。
//!
//! `theta_replay`（#1305 ①件，确定性回放）＋ `theta_pi_diff`（#1306 ②件，三面对拍）组装成
//! 一条命令：N 个月生产 runner 路径回放 + 信号/状态/账本三面对拍 + PASS/FAIL 报告（含预期差
//! 计数）。对接 #1279 检查单 B 组「生产路径历史回放验收」条目：
//!
//! > 用生产 runner 路径（非分析脚本路径）完整回放 ≥N 个月历史，与回测路径逐位对拍
//! > （信号/状态迁移/账本事件三面）——一致 = 管道认证；不一致 = #948/#378 族问题现形。
//!
//! ## 两面判定
//!
//! 1. **回放侧**（[`replay_dump::run_replay_double`]）：同输入双跑逐位一致 = 确定性自证。
//! 2. **对拍侧**（[`theta_pi_diff`]）：信号面铁锁零预期差 + 状态/账本面预期差机械清单计数；
//!    预期外差异 = 回归告警（FAIL）。
//!
//! 两面都过 ⟹ 该品种 PASS。含不可交易 bar 的窗上，对拍属 #1306 v1 已知 seam（stream 逐 bar
//! append vs 批量只对可交易 bar classify_at，v1 不展开）——对拍计数照打但标注 inconclusive，
//! 不作为回归判据（回放确定性仍硬判）。
//!
//! ## 用法
//! ```text
//! cargo run --release --features backtest_bin --bin theta_accept -- [SYMBOL...] [选项]
//!   SYMBOL...           品种代码（BTC/ES/CL/GC/BRN/DX/QQQ/OKLO）；省略 = 全部 8 品种
//!   --window START END  ISO 日期窗闭区间（如 2024-01-01 2024-12-31）
//!   --months N          从数据集末往前 N 个月（与 --window 二选一；省略 = 全量）
//!   --parallel          多品种并行（默认顺序；报告顺序恒为 SYMBOLS 表序）
//!   --out DIR           报告写盘（每品种一份 <SYMBOL>.md + summary.md；默认只打 stdout）
//! ```
//!
//! ## 认识论等级（formalization-validity-domain 231号）
//!
//! **L1**（管线正确性 + 确定性 + 对拍一致性）：本命令验证「历史 feed → 生产 runner 路径」
//! 串通、确定性与三面对拍，不验证 Θ 在市场有效。

use std::fmt::Write as _;
use std::time::Instant;

use newchan_rust::theta_v0::backtest::data::{load_by_symbol, Dataset, SYMBOLS};
use newchan_rust::theta_v0::backtest::replay_dump::{
    initial_nav_for, run_replay_double, ReplayOutcome,
};
use newchan_rust::theta_v0::backtest::theta_pi_diff::{diff, run_batch, run_stream, DiffReport};
use newchan_rust::theta_v0::config::ThetaConfig;

/// 单品种验收结果。
struct SymbolReport {
    symbol: String,
    window: Option<(String, String)>,
    months: f64,
    replay: ReplayOutcome,
    diff: DiffReport,
    /// 对拍是否权威（窗内无不可交易 bar ⟹ true；含则 #1306 v1 seam ⟹ advisory）。
    diff_authoritative: bool,
    elapsed: std::time::Duration,
}

impl SymbolReport {
    /// 回放确定性硬判。
    fn replay_pass(&self) -> bool {
        self.replay.identical
    }

    /// 对拍判定：权威窗 = 无预期外差异；advisory 窗 = 不判回归（恒 true）。
    fn diff_pass(&self) -> bool {
        if !self.diff_authoritative {
            return true;
        }
        self.diff.n_unexpected() == 0
    }

    fn pass(&self) -> bool {
        self.replay_pass() && self.diff_pass()
    }

    fn verdict(&self) -> &'static str {
        match (
            self.replay_pass(),
            self.diff_authoritative,
            self.diff.n_unexpected(),
        ) {
            (true, true, 0) => "PASS",
            (true, false, _) => "PASS（对拍 inconclusive：含不可交易 bar）",
            (true, true, _) => "FAIL（预期外差异 = 回归）",
            (false, _, _) => "FAIL（确定性自检）",
        }
    }
}

/// 把 ISO 日期 `YYYY-MM-DD` 往前推 `n` 个月（日钳到当月天数）。
fn subtract_months(date: &str, n: u32) -> String {
    let y: i32 = date.get(0..4).and_then(|s| s.parse().ok()).unwrap_or(1970);
    let m: i32 = date.get(5..7).and_then(|s| s.parse().ok()).unwrap_or(1);
    let d: i32 = date.get(8..10).and_then(|s| s.parse().ok()).unwrap_or(1);
    let total = y * 12 + (m - 1) - n as i32;
    let ny = total.div_euclid(12);
    let nm = total.rem_euclid(12) + 1;
    let days = match nm {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            let leap = (ny % 4 == 0 && ny % 100 != 0) || ny % 400 == 0;
            if leap {
                29
            } else {
                28
            }
        }
        _ => 30,
    };
    format!("{ny:04}-{nm:02}-{:02}", d.min(days))
}

/// 跑单品种（回放双跑 + 三面对拍）。
fn run_symbol(
    symbol: &str,
    window: Option<(&str, &str)>,
    months: Option<u32>,
) -> Result<SymbolReport, String> {
    let config = ThetaConfig::default();
    let full = load_by_symbol(symbol, &config)?;

    // ── 解析有效窗 (start, end)；None = 全量。 ──
    let slice: Dataset;
    let resolved: Option<(String, String)>;
    match window {
        Some((s, e)) => {
            if s > e {
                return Err(format!("日期窗非法: START `{s}` > END `{e}`"));
            }
            slice = full.slice_date_window(s, e);
            resolved = Some((s.to_string(), e.to_string()));
        }
        None => match months {
            Some(n) => {
                let last = full
                    .dates
                    .last()
                    .map(|d| d.get(..10).unwrap_or("").to_string())
                    .ok_or_else(|| "空数据集无法按 --months 切窗".to_string())?;
                let start = subtract_months(&last, n);
                slice = full.slice_date_window(&start, &last);
                resolved = Some((start, last));
            }
            None => {
                slice = full;
                resolved = None;
            }
        },
    }

    if slice.bars.is_empty() {
        return Err(format!("品种 {symbol} 在指定窗内无 bar（空数据集）"));
    }

    let started = Instant::now();
    let replay = run_replay_double(
        &slice,
        &config,
        resolved.as_ref().map(|(s, e)| (s.as_str(), e.as_str())),
    );
    let diff_authoritative = slice.untradable_ratio() == 0.0;
    let initial_nav = initial_nav_for(&slice, &config);
    let batch = run_batch(&slice.bars, &config, initial_nav);
    let stream = run_stream(&slice.bars, &config, &batch);
    let report = diff(&batch, &stream);
    let elapsed = started.elapsed();
    let years = slice.bars.len() as f64
        / newchan_rust::theta_v0::backtest::data::bars_per_year(slice.bar_seconds);
    let months_f = years * 12.0;

    Ok(SymbolReport {
        symbol: symbol.to_string(),
        window: resolved,
        months: months_f,
        replay,
        diff: report,
        diff_authoritative,
        elapsed,
    })
}

fn print_symbol_report(r: &SymbolReport) {
    println!();
    println!("════════════════════════════════════════════════════════════════");
    println!("  品种 {}", r.symbol);
    println!("════════════════════════════════════════════════════════════════");
    match &r.window {
        Some((s, e)) => println!("数据窗        : {s} .. {e}（≈{:.1} 个月）", r.months),
        None => println!("数据窗        : 全量（≈{:.1} 个月）", r.months),
    }
    println!("bar 数        : {}", r.replay.n_bars);
    println!("不可交易占比  : {:.2}%", r.replay.untradable_ratio * 100.0);
    println!("总耗时        : {:.2}s", r.elapsed.as_secs_f64());
    println!();
    println!("── 回放侧（theta_replay，#1305）──────────────────────────────");
    println!(
        "同输入双跑自检  : {}",
        if r.replay.identical {
            "PASS（逐位一致）".to_string()
        } else {
            format!(
                "FAIL（首个分歧偏移 = {}）",
                r.replay.first_divergence_offset.unwrap_or(0)
            )
        }
    );
    println!(
        "双跑吞吐        : {:.0} bar/s（双跑共 {} bar，{:.2}s）",
        (r.replay.n_bars as f64 * 2.0) / r.replay.elapsed.as_secs_f64().max(1e-9),
        r.replay.n_bars * 2,
        r.replay.elapsed.as_secs_f64(),
    );
    println!();
    println!("── 对拍侧（theta_pi_diff，#1306）──────────────────────────────");
    print!("{}", r.diff);
    if !r.diff_authoritative {
        println!(
            "⚠ 窗内含不可交易 bar（{:.2}%）——对拍属 #1306 v1 已知 seam，",
            r.replay.untradable_ratio * 100.0
        );
        println!("  三面 diff 计数仅作读数，不作为回归判据（回放确定性仍硬判）。");
    }
    println!();
    println!("── 验收判定（#1279 检查单 B 组「生产路径历史回放验收」）──────");
    println!(
        "回放确定性  : {}",
        if r.replay_pass() { "PASS" } else { "FAIL" }
    );
    println!(
        "三面对拍    : {}",
        if r.diff_authoritative {
            if r.diff.n_unexpected() == 0 {
                "PASS（无预期外差异）"
            } else {
                "FAIL（预期外差异 = 回归）"
            }
        } else {
            "inconclusive（含不可交易 bar）"
        }
    );
    println!("预期内差异  : {}", r.diff.n_expected());
    println!("预期外差异  : {}", r.diff.n_unexpected());
    println!("判定        : {}", r.verdict());
}

/// 单品种报告写盘（markdown）。
fn write_symbol_report_md(r: &SymbolReport, dir: &std::path::Path) -> std::io::Result<()> {
    let mut md = String::new();
    let _ = writeln!(md, "# 生产路径历史回放验收报告 —— {}", r.symbol);
    let _ = writeln!(md);
    let _ = writeln!(
        md,
        "- 检查单条目：#1279 B 组「生产路径历史回放验收」（#1308 一键验收命令产出）"
    );
    let _ = writeln!(md, "- 判定：**{}**", r.verdict());
    match &r.window {
        Some((s, e)) => {
            let _ = writeln!(md, "- 数据窗：{s} .. {e}（≈{:.1} 个月）", r.months);
        }
        None => {
            let _ = writeln!(md, "- 数据窗：全量（≈{:.1} 个月）", r.months);
        }
    }
    let _ = writeln!(md, "- bar 数：{}", r.replay.n_bars);
    let _ = writeln!(
        md,
        "- 不可交易占比：{:.2}%",
        r.replay.untradable_ratio * 100.0
    );
    let _ = writeln!(
        md,
        "- 回放确定性（同输入双跑逐位）：{}",
        if r.replay.identical { "PASS" } else { "FAIL" }
    );
    let _ = writeln!(
        md,
        "- 三面对拍：信号分歧 {} / 预期内 {} / 预期外 {}（{}）",
        r.diff.n_signal_diffs(),
        r.diff.n_expected(),
        r.diff.n_unexpected(),
        if r.diff_authoritative {
            "权威"
        } else {
            "inconclusive：含不可交易 bar"
        }
    );
    let _ = writeln!(md);
    let _ = writeln!(md, "```text");
    let _ = write!(md, "{}", r.diff);
    let _ = writeln!(md, "```");
    std::fs::write(dir.join(format!("{}.md", r.symbol)), md)
}

fn print_summary(reports: &[SymbolReport]) {
    println!();
    println!("════════════════════════════════════════════════════════════════");
    println!("  验收汇总（{} 品种）", reports.len());
    println!("════════════════════════════════════════════════════════════════");
    for r in reports {
        println!(
            "{:<6} {:<38} 信号分歧 {}  预期内 {}  预期外 {}",
            r.symbol,
            r.verdict(),
            r.diff.n_signal_diffs(),
            r.diff.n_expected(),
            r.diff.n_unexpected(),
        );
    }
    let n_pass = reports.iter().filter(|r| r.pass()).count();
    println!("合计：PASS {n_pass} / FAIL {}", reports.len() - n_pass);
}

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().collect();

    let mut symbols: Vec<String> = Vec::new();
    let mut window: Option<(&str, &str)> = None;
    let mut months: Option<u32> = None;
    let mut parallel = false;
    let mut out_dir: Option<&str> = None;

    let mut rest = args.iter().skip(1).peekable();
    while let Some(a) = rest.next() {
        match a.as_str() {
            "--window" => match (rest.next(), rest.next()) {
                (Some(s), Some(e)) => window = Some((s, e)),
                _ => {
                    eprintln!("--window 需要 START END 两个参数");
                    return std::process::ExitCode::from(2);
                }
            },
            "--months" => match rest.next() {
                Some(n) => match n.parse::<u32>() {
                    Ok(v) if v > 0 => months = Some(v),
                    _ => {
                        eprintln!("--months 需要正整数");
                        return std::process::ExitCode::from(2);
                    }
                },
                None => {
                    eprintln!("--months 需要参数");
                    return std::process::ExitCode::from(2);
                }
            },
            "--parallel" => parallel = true,
            "--out" => match rest.next() {
                Some(p) => out_dir = Some(p),
                None => {
                    eprintln!("--out 需要路径参数");
                    return std::process::ExitCode::from(2);
                }
            },
            other if other.starts_with("--") => {
                eprintln!("未知选项: {other}");
                return std::process::ExitCode::from(2);
            }
            sym => symbols.push(sym.to_string()),
        }
    }

    if symbols.is_empty() {
        symbols = SYMBOLS.iter().map(|(s, _, _)| s.to_string()).collect();
    }

    // 品种名归一（大小写不敏感），未知品种 fail-loud。
    for sym in &symbols {
        if !SYMBOLS.iter().any(|(s, _, _)| s.eq_ignore_ascii_case(sym)) {
            eprintln!("未知品种 `{sym}`（不在 SYMBOLS 表：BTC/ES/CL/GC/BRN/DX/QQQ/OKLO）");
            return std::process::ExitCode::from(2);
        }
    }

    println!("=== 生产路径历史回放验收报告器（#1308，#1279 检查单 B 组）===");
    println!("检查单条目  : B「生产路径历史回放验收」——生产 runner 路径回放 + 三面对拍");
    match (window, months) {
        (Some((s, e)), _) => println!("数据窗      : {s} .. {e}"),
        (None, Some(n)) => println!("数据窗      : 末 {n} 个月"),
        (None, None) => println!("数据窗      : 全量"),
    }
    println!("品种        : {}", symbols.join(", "));
    println!("模式        : {}", if parallel { "并行" } else { "顺序" });

    // ── 逐品种执行。并行用 thread::scope：diff_capture/signal_capture 均为 thread_local，
    //    各线程隔离；报告仍按 SYMBOLS 表序（输入序）汇总，确定性输出。 ──
    let run_one = |sym: &str| run_symbol(sym, window, months);

    let reports: Vec<Result<SymbolReport, String>> = if parallel {
        std::thread::scope(|scope| {
            let handles: Vec<_> = symbols
                .iter()
                .map(|sym| scope.spawn(move || run_one(sym)))
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        })
    } else {
        symbols.iter().map(|sym| run_one(sym)).collect()
    };

    // 失败品种收集（load 失败/空窗）不与成功品种混序：先打成功报告，再打失败。
    let mut ok_reports: Vec<SymbolReport> = Vec::new();
    let mut failures: Vec<(String, String)> = Vec::new();
    for (sym, r) in symbols.iter().zip(reports) {
        match r {
            Ok(rep) => ok_reports.push(rep),
            Err(e) => failures.push((sym.clone(), e)),
        }
    }

    for r in &ok_reports {
        print_symbol_report(r);
    }
    if !failures.is_empty() {
        println!();
        println!("── 失败/空窗（不参与判定）──────────────────────────────────");
        for (sym, e) in &failures {
            println!("  {sym:<6} {e}");
        }
    }

    if ok_reports.is_empty() {
        eprintln!("无品种可验收（全部加载失败或空窗）。");
        return std::process::ExitCode::FAILURE;
    }

    print_summary(&ok_reports);

    // ── 报告写盘（可选）。 ──
    if let Some(dir) = out_dir {
        let dir = std::path::Path::new(dir);
        if let Err(e) = std::fs::create_dir_all(dir) {
            eprintln!("创建输出目录 {dir:?} 失败: {e}");
            return std::process::ExitCode::FAILURE;
        }
        for r in &ok_reports {
            if let Err(e) = write_symbol_report_md(r, dir) {
                eprintln!("写报告 {}.md 失败: {e}", r.symbol);
                return std::process::ExitCode::FAILURE;
            }
        }
        let mut summary = String::new();
        summary.push_str("# 生产路径历史回放验收汇总（#1308 / #1279 检查单 B 组）\n\n");
        for r in &ok_reports {
            let _ = writeln!(
                summary,
                "- {:<6} {}（信号分歧 {} / 预期内 {} / 预期外 {}）",
                r.symbol,
                r.verdict(),
                r.diff.n_signal_diffs(),
                r.diff.n_expected(),
                r.diff.n_unexpected(),
            );
        }
        if let Err(e) = std::fs::write(dir.join("summary.md"), summary) {
            eprintln!("写 summary.md 失败: {e}");
            return std::process::ExitCode::FAILURE;
        }
        println!("\n报告已写入: {dir:?}");
    }

    if ok_reports.iter().all(|r| r.pass()) {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::subtract_months;

    /// `subtract_months` 跨年/跨月/闰年二月日钳制（#1308 --months 窗切分的单调键前提）。
    #[test]
    fn subtract_months_crosses_year_and_clamps_day() {
        assert_eq!(subtract_months("2024-01-31", 1), "2023-12-31");
        // 2024 闰年二月 29 天。
        assert_eq!(subtract_months("2024-03-31", 1), "2024-02-29");
        // 2023 平年二月 28 天。
        assert_eq!(subtract_months("2023-03-31", 1), "2023-02-28");
        assert_eq!(subtract_months("2024-01-15", 12), "2023-01-15");
        assert_eq!(subtract_months("2024-01-15", 0), "2024-01-15");
        // 12 个月以上跨多年。
        assert_eq!(subtract_months("2024-01-15", 13), "2022-12-15");
    }
}
