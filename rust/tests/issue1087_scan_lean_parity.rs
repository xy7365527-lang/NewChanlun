//! #1087 真实 EQUS.MINI 窗口签收：Rust 生产 merged 是被检侧；Lean 从原始
//! rows/centers/blocks 重算 prelude，从逐段 sink 重算四输出，并重算 CandDelta 装配与 c_p
//! stable-revision 推进。三标的各 200k bars，L0/L1/L2 逐字段硬门；无 legacy oracle。
//!
//! fixture 渲染与 Lean 复核共享面 = `classifier::lean_mirror`（3b 起与 strict_nest_check 镜像
//! 对拍执行器共用单一渲染口径，#1060/ADR 0026；#799 收敛通则——不另存第二份渲染器）。

use newchan_rust::theta_v0::{
    classifier::{issue1087_parity, lean_mirror, TowerCache},
    parser,
};
use std::{path::Path, process::Command};

const BAR_COUNT: usize = 200_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WindowSpec {
    symbol: &'static str,
    start: usize,
    end: usize,
}

const WINDOWS: [WindowSpec; 3] = [
    WindowSpec {
        symbol: "AAPL",
        start: 0,
        end: BAR_COUNT,
    },
    WindowSpec {
        symbol: "MSFT",
        start: 0,
        end: BAR_COUNT,
    },
    WindowSpec {
        symbol: "NVDA",
        start: 0,
        end: BAR_COUNT,
    },
];

/// 完整验收入口没有 symbol 过滤器；它机械固定为 AAPL/MSFT/NVDA 三窗。
fn complete_acceptance_windows() -> [WindowSpec; 3] {
    WINDOWS
}

fn mirror_window(window: WindowSpec) -> lean_mirror::MirrorWindow {
    lean_mirror::MirrorWindow {
        symbol: window.symbol.to_string(),
        start: window.start,
        end: window.end,
    }
}

fn windows_are_pairwise_disjoint(windows: &[WindowSpec]) -> bool {
    windows.iter().enumerate().all(|(i, left)| {
        left.start < left.end
            && windows.iter().skip(i + 1).all(|right| {
                left.symbol != right.symbol || left.end <= right.start || right.end <= left.start
            })
    })
}

#[test]
fn complete_acceptance_plan_is_exactly_aapl_msft_nvda() {
    assert_eq!(complete_acceptance_windows(), WINDOWS);
}

#[test]
fn acceptance_stage_without_l2_is_rejected() {
    let err = lean_mirror::assert_complete_level_ladder(
        &mirror_window(WindowSpec {
            symbol: "NEGATIVE",
            start: 0,
            end: BAR_COUNT,
        }),
        "negative",
        &[0, 1],
        true,
    )
    .expect_err("必须拒绝未达 L2 的级别阶梯");
    assert!(err.contains("至少达到 L2"), "错误信息必须点名 L2 门：{err}");
}

#[test]
fn three_acceptance_windows_are_three_symbols_and_pairwise_disjoint() {
    assert!(windows_are_pairwise_disjoint(&WINDOWS));
    assert!(WINDOWS.iter().enumerate().all(|(index, left)| WINDOWS
        .iter()
        .skip(index + 1)
        .all(|right| left.symbol != right.symbol)));
    assert!(WINDOWS
        .iter()
        .all(|window| window.start == 0 && window.end - window.start == BAR_COUNT));
}

#[test]
fn databento_nanodollar_price_uses_cents_tick_roundtrip() {
    lean_mirror::databento_nanodollars_price_uses_cents_tick_roundtrip_check();
}

#[test]
fn three_real_windows_recompute_in_lean_and_match_every_field() {
    let root = lean_mirror::equs_mini_root(
        std::env::var_os("EQUS_MINI_DIR")
            .as_deref()
            .map(std::path::Path::new),
    );
    assert!(
        root.is_dir(),
        "真实 EQUS.MINI 目录不存在：{}；可设置 EQUS_MINI_DIR",
        root.display()
    );
    assert!(windows_are_pairwise_disjoint(&WINDOWS));
    let formal = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("formal");
    let build = Command::new("lake")
        .current_dir(&formal)
        .args([
            "build",
            "Origin.ScanAssemblyMirror",
            "Origin.ScanAssemblyBridge",
        ])
        .output()
        .expect("无法执行 lake build");
    assert!(
        build.status.success(),
        "Lean 构建失败：\n{}\n{}",
        String::from_utf8_lossy(&build.stdout),
        String::from_utf8_lossy(&build.stderr)
    );

    let config = lean_mirror::issue1087_config();
    let mut executed_windows = Vec::with_capacity(WINDOWS.len());
    for window in complete_acceptance_windows() {
        let all_bars = lean_mirror::load_equs_bars(&root, window.symbol, window.end);
        let bars = &all_bars[window.start..window.end];
        assert_eq!(bars.len(), BAR_COUNT);
        assert_eq!(bars[0].source_index, window.start);
        assert_eq!(bars[BAR_COUNT - 1].source_index, window.end - 1);
        assert!(bars
            .windows(2)
            .all(|pair| pair[0].timestamp < pair[1].timestamp));

        // checkpoint → 末根 → 回缩 → 末根恢复。四次都复用同一 TowerCache，真实穿过
        // cache/frontier resume 与 shrink invalidation；每次 Rust 快照都由 fresh raw sink 喂 Lean。
        let stages = [
            ("checkpoint", BAR_COUNT / 2),
            ("terminal", BAR_COUNT),
            ("shrink", BAR_COUNT / 2),
            ("terminal_resume", BAR_COUNT),
        ];
        let mut cache = TowerCache::new();
        let mut saw_cand = false;
        let mut saw_rejected = false;
        let mut saw_cp_close = false;
        for (stage, prefix) in stages {
            issue1087_parity::begin();
            let layer = parser::parse_layer(&bars[..prefix], &config);
            let _ = newchan_rust::theta_v0::classifier::classify_incremental(
                &layer,
                &config,
                &mut cache,
                &[],
            );
            let records = issue1087_parity::finish();
            lean_mirror::verify_records_in_lean(
                &formal,
                mirror_window(window),
                stage,
                &records,
                true,
                true,
            )
            .unwrap_or_else(|error| panic!("{error}"));
            let (accepted_raw, production_events) =
                lean_mirror::stage_event_domain_counts(&records);

            saw_cand |= records
                .iter()
                .any(|record| !record.cand_delta_cases.is_empty());
            saw_rejected |= records
                .iter()
                .flat_map(|record| &record.cand_delta_cases)
                .any(|case| !lean_mirror::cand_case_is_accepted(case));
            saw_cp_close |= records
                .iter()
                .flat_map(|record| &record.cp_transitions)
                .any(|transition| {
                    transition.before.lifecycle
                        == newchan_rust::theta_v0::classifier::recursive_tower::CpLifecycleStatus::Pending
                        && transition.after.lifecycle
                            == newchan_rust::theta_v0::classifier::recursive_tower::CpLifecycleStatus::Closed
                });
            eprintln!(
                "#1087 {} stage={} prefix={} levels={} cand_cases={} accepted_raw={} production_events={} rejected={} cp_closed={} PASS",
                window.symbol,
                stage,
                prefix,
                records.len(),
                records
                    .iter()
                    .map(|record| record.cand_delta_cases.len())
                    .sum::<usize>(),
                accepted_raw,
                production_events,
                records
                    .iter()
                    .flat_map(|record| &record.cand_delta_cases)
                    .filter(|case| !lean_mirror::cand_case_is_accepted(case))
                    .count(),
                records
                    .iter()
                    .flat_map(|record| &record.cp_transitions)
                    .filter(|transition| {
                        transition.before.lifecycle
                            == newchan_rust::theta_v0::classifier::recursive_tower::CpLifecycleStatus::Pending
                            && transition.after.lifecycle
                                == newchan_rust::theta_v0::classifier::recursive_tower::CpLifecycleStatus::Closed
                    })
                    .count(),
            );
        }
        assert!(saw_cand, "{window:?} 必须有 event 前 raw CandDelta case");
        assert!(saw_rejected, "{window:?} 必须覆盖至少一个 Lean 判拒样本");
        assert!(
            saw_cp_close,
            "{window:?} 必须覆盖 raw closure 判定的 Pending→Closed"
        );
        executed_windows.push(window);
    }
    assert_eq!(
        executed_windows.as_slice(),
        WINDOWS.as_slice(),
        "完整验收必须恰好执行 AAPL/MSFT/NVDA 三窗"
    );
}
