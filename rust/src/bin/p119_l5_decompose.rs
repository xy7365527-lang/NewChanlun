//! task #119（U1 裁定 §6-4 分解探针）：L5 零覆盖归因层「未结」的逐对分解（只读 bin）。
//!
//! ▍裁定依据（先读）
//! - U1 裁定 `chanlun/escalate/nest-leftover-rulings-u1-u3-20260718.md` U1 项：L5 零覆盖
//!   归因层裁「未结」；签署 p109 §6-4 分解探针（只读、不进证书门）；输出按预裁二叉
//!   处置——(a) D2 pair 缺失 ⟹ provider 装配缺口（立实装修复任务）；(b) Extreme 未过／
//!   结构不成立 ⟹ 市场事实终审；(c) 残余第三因如实落档，占比显著时另立裁定。
//! - 证据锚：`p109-chain157-gate-attrition-20260717.md:93-95`（链侧 8 个 L5 strict pair、
//!   events=0、命中率 0%）、同文 `:206`（§6-4：D2 pair 缺失 vs Extreme 未过待分解）；
//!   `p107-level-funnel-audit-20260717.md:85`（provider 漏斗 L5 全 0）、`:97-98`
//!   （「市场事实」读法已降级为「现状读数，待分解」）。
//!
//! ▍090 声明（声明必须与实际能力一致）
//! 1. 只读探针：不改账本、不进证书门、零 git mutation、零生产源码改动；全部经 lib
//!    `pub` API 调用（theta_v0 生产源码只读）。主仓只读；输入数据只读。
//! 2. **本 bin 未经编译验证**——任务纪律：在跑 p116 重放期间禁跑任何 cargo 命令
//!    （含 check/build/run）。首次 `cargo build` 可能需小修（import 路径/类型签名
//!    以 lib 实际 pub 面为准）；本头声明与代码均经 2026-07-18 直读源码逐条核对，
//!    但未编译 ≠ 已验证，如实声明。
//! 3. 复制非调用（pub 面不可达原语的**逐字复制**，已逐字核对；生产侧演进须同步，
//!    复制处注明来源行号）：`leg_as_segment`（level_view.rs:436-448，私有）；
//!    `move_range_envelope`（divergence.rs:864-876，pub(crate)）；
//!    `structural_pair_span`（level_view.rs:504-517，私有——其谓词在 decompose_pair
//!    内联复评，不切出函数）。
//! 4. 外部复评非插桩：D2-pair-缺失的子门定位 = 在 bin 侧用同一批 pub 原语
//!    （`locate_departure_move_a`/`departure_move_c_start`/`self_anchors`，
//!    divergence.rs:767/841/804）按 `provide_divergence_pairs`（level_view.rs:810-869）
//!    的同序同参复评；provider 内部零插桩。provider 逻辑未来变更时本复评可能漂移，
//!    一切以 level_view.rs 当时行为为准。
//! 5. 不可分解面（如实声明，不装能分解）：pan 通道门 `locate_pan_div_structure` /
//!    `pan_div_structure_extreme`（signal.rs:637/736，pub(crate)）bin 不可达——盘整
//!    leave 对的 pan 侧只报事件有/无（turn_source ∈ 对 span 的**近似归属**），不分解
//!    门因；`trend_confirm_time`（level_view.rs:535，私有）不可达——
//!    divergence_confirmed 真值读自 provider 事件字段，不复算。
//! 6. 对账锚（不预设结论）：L5 strict_pairs 期望复现 p109:93 的 8；实测值如实上报，
//!    与 p109 不符时以本 run 实测为准并如实披露（探针只测量，不下裁定——判定留
//!    U1 复议）。hist/dif/close_src 取自 `TowerCache.causal_series()/macd_dif()`
//!    （mod.rs:942-950，文档锁 bit-exact 等价全量 compute_macd，与 p92/p116 生产同源；
//!    p109 bin:891-893 的全量 compute_macd 路径逐位等价）。
//!
//! ▍输入/输出契约
//! 输入：argv[1] = 全量 BTC 1m JSON（只读）；env `P119_MAX_BARS`（截断重放，冒烟用）。
//! 输出（stdout）：
//!   `P119_INPUT bars=<n> replay_bars=<n> first_date=<..> last_date=<..>`
//!   `P119_RULE <口径行>`
//!   `P119_L5_LEVELSTATE levels_len=<n> l5_moves=<n> l5_completed=<n> l5_centers=<n>`
//!     ——塔 LevelState 视角（classification.levels[5]，mod.rs:163；MoveBlock.status
//!     语义 decompose.rs:34-39）；塔级 <6 时输出 note=塔顶无结构(塔级<6)。
//!   `P119_L5_STRUCTURE tower_levels=<n> tower_windows=<n> runs=<n> invalid_windows=<n>
//!     moves=<n> completed=<n> strict_pairs=<n> provider_events=<n>`
//!     ——C2 view 视角，p109 `P109_LEVEL_V3` 行的对账镜像。
//!   `P119_CASE pair=<i> span=[s,e] side=<Long|Short|NA> d2_event=<bool>
//!     kill_gate=<none|extreme|comparable|dir|anchor|tau|other:描述> detail=<关键坐标>`
//!   `P119_L5 total=<n> d2_events=<n> extreme_killed=<n> other_killed=<n>`
//!     （total=0 时附 note=塔顶无结构(无完成结构对)）
//!   `P119_VERDICT branch_a_d2_pair_missing=<n> branch_b_extreme_killed=<n>
//!     branch_b_structural_tau=<n> branch_c_other=<n> events=<n>
//!     note=实测计数_不自动下裁定_判定留U1复议`
//!
//! ▍kill_gate 词汇表（与任务书枚举对齐；门序 = 生产管线序）
//! - `tau`：leave 块非趋势 τ（kind=Consolidation ⟹ dir=None ⟹ D2 零 pair，
//!   level_view.rs:798-799/811-813；signal.rs τ 门控同义）。归二叉 (b)「结构不成立」侧。
//! - `dir`：leave=Trend 但 dir=None（decompose 不变量下理论不可达，防御性保留）。归 (c)。
//! - `comparable`：`locate_departure_move_a`=None（无可比较前段 A；cand_predicate 条件2）。
//!   归二叉 (a)「D2 pair 缺失」。
//! - `anchor`：`departure_move_c_start`=None（C 锚定失败）。归 (a)。
//! - `extreme`：D2 pair 存在但 c 包络未破 b 包络（level_view.rs:677-683 Extreme 预滤；
//!   detail 给 seg_a/seg_c 包络坐标）。归二叉 (b)。
//! - `other:no_c_terminal` / `other:no_c_end`：全离开段 C 不存在
//!   （level_view.rs:826-832/847-858）。归 (a)（C 未长出 ⟹ pair 未产，装配语义同缺失）。
//! - `other:retest_not_completed`：view 双 Completed 但 decompose 块 status≠Completed
//!   （structural_pair_span 缝——view Completed(TerminalDivergence) 与块 status 的对账缝）。归 (c)。
//! - `other:envelope_unmappable`：包络无整支落入段（037:20 不可验，诚实判负）。归 (c)。
//! - `other:block_span_invalid` / `other:pair_logic_mismatch` / `other:provider_mismatch`：
//!   不变量/复评交叉核对失败（不可能即 bug，如实落档）。归 (c)。
//! - `none`：d2_event=true（detail 带 kind/turn_source/divergence_confirmed/judge_at）。
//!
//! ▍复用关系
//! - 数据加载（`load_bars`/`BarsJson`/`date_to_timestamp`/`LoadedBars`）与
//!   `run_terminal_pass`/`TerminalState` 骨架逐字复制自
//!   p116_turnpoint_anchor_existence.rs:241-246,542-569,1134-1211（进度标签 P116_→P119_；
//!   l0/classification 字段为骨架一致性保留，classification 仅用于 P119_L5_LEVELSTATE 行）。
//! - L5 strict pair 口径逐字复刻 p109 bin `measure_level` V3 分支
//!   （p109_chain157_gate_attrition.rs:213-289：runs 切分→carried_only 投影→decompose
//!   →assemble_level_view→view.moves.windows(2) 双 Completed；events=
//!   provide_nest_candidate_events）。PairRec 字段语义 :44-53。
//! - 与 p117 bin 无代码复用（p117 是 S1a 级别复核探针，门链不同；仅共享 p117 T1 裁定
//!   后的时代口径背景，本探针不查终端背书）。
//!
//! ▍注册备查（纪律：禁改 Cargo.toml——src/bin/*.rs 由 cargo 自动发现；若未来需显式注册）：
//! ```toml
//! [[bin]]
//! name = "p119_l5_decompose"
//! path = "src/bin/p119_l5_decompose.rs"
//! ```
//!
//! 用法（p116 重放完结后由他人执行；冒烟 `P119_MAX_BARS=250000 ...`）：
//! `cargo run --release --bin p119_l5_decompose -- /Users/silencehan/Projects/NewChanlun/analysis/data_cache/btc_1m_full.json`

use newchan_rust::theta_v0::classifier;
use newchan_rust::theta_v0::classifier::decompose::{self, MoveBlock, MoveStatus};
use newchan_rust::theta_v0::classifier::divergence::{
    departure_move_c_start, locate_departure_move_a, self_anchors,
};
use newchan_rust::theta_v0::classifier::level_view::{
    assemble_level_view, lower_legs_from, project_extended_windows_carried_only,
    provide_nest_candidate_events, C2LevelViewConfig, C2VersionTuple, CompletionStatus,
    CoordinateWindow, ExactThreeProjection, LevelAsOfView, LevelViewMaterial, LevelViewQuery,
    LowerLeg, NestCandidateEvent, NestDivergenceKind, ProjectionMaterial,
};
use newchan_rust::theta_v0::classifier::recursive_tower::LeveledMove;
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::{ParseLayer, ParseLayerIncr};
use newchan_rust::theta_v0::types::{
    quantize, Bar, Direction, MoveKind, Segment, Side, Tick, Timestamp,
};
use serde::Deserialize;
use std::path::Path;
use std::rc::Rc;
use std::time::Instant;

/// 目标级别：L5 = 塔顶（config.rs:82 `l_max: 6` ⟹ 级别 0..=5，tower.len()=6 时 levels[5] 存在）。
const L5: usize = 5;

/// L5 strict pair（p109 `PairRec` 同口径 + view.moves 下标，供 decompose 块回查）。
/// view.moves 与 decompose blocks 1:1 同序（level_view.rs:992-1084 顺序 push）。
#[derive(Debug, Clone, Copy)]
struct StrictPair {
    /// leave 块在 view.moves / blocks 中的下标；retest = leave_index + 1。
    leave_index: usize,
    /// p109 PairRec.start = leave.start_index（source bar 坐标系）。
    start: usize,
    /// p109 PairRec.end = retest.end_index（source bar 坐标系）。
    end: usize,
    leave_kind: MoveKind,
    leave_dir: Option<Direction>,
    #[allow(dead_code)] // 保留 p109 PairRec 字段完整性；retest_kind 不进归因键。
    retest_kind: MoveKind,
}

/// 击杀门（词汇表见模块头；tag() 输出与任务书枚举逐字对齐）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KillGate {
    None,
    Tau,
    Dir,
    Comparable,
    Anchor,
    Extreme,
    OtherNoCTerminal,
    OtherNoCEnd,
    OtherRetestNotCompleted,
    OtherEnvelopeUnmappable,
    OtherBlockSpanInvalid,
    OtherPairLogicMismatch,
    OtherProviderMismatch,
}

impl KillGate {
    fn tag(self) -> &'static str {
        match self {
            KillGate::None => "none",
            KillGate::Tau => "tau",
            KillGate::Dir => "dir",
            KillGate::Comparable => "comparable",
            KillGate::Anchor => "anchor",
            KillGate::Extreme => "extreme",
            KillGate::OtherNoCTerminal => "other:no_c_terminal",
            KillGate::OtherNoCEnd => "other:no_c_end",
            KillGate::OtherRetestNotCompleted => "other:retest_not_completed",
            KillGate::OtherEnvelopeUnmappable => "other:envelope_unmappable",
            KillGate::OtherBlockSpanInvalid => "other:block_span_invalid",
            KillGate::OtherPairLogicMismatch => "other:pair_logic_mismatch",
            KillGate::OtherProviderMismatch => "other:provider_mismatch",
        }
    }
}

/// 单对分解结论（detail 为单 token：内部只用 `;` `,` `=` 分隔，不含空格）。
#[derive(Debug)]
struct CaseOutcome {
    d2_event: bool,
    gate: KillGate,
    side: Option<Side>,
    detail: String,
}

/// 逐字复制自 `level_view.rs:436-448`（私有，bin 不可达）——复制非调用，见模块头 090 声明 3。
fn leg_as_segment(value: &LowerLeg) -> Segment {
    let (start_price, end_price) = match value.direction {
        Direction::Up => (value.lo, value.hi),
        Direction::Down => (value.hi, value.lo),
    };
    Segment {
        direction: value.direction,
        start_index: value.start_index,
        end_index: value.end_index,
        start_price,
        end_price,
    }
}

/// 逐字复制自 `divergence.rs:864-876`（pub(crate)，bin 不可达）——复制非调用，见模块头 090 声明 3。
/// 段须整支落入 span；包络 = 各段 [min,max] 的 (min,max) 折叠；无段整支落入 ⟹ None。
fn move_range_envelope(segments: &[Segment], span: (usize, usize)) -> Option<(Tick, Tick)> {
    segments
        .iter()
        .filter(|segment| segment.start_index >= span.0 && segment.end_index <= span.1)
        .fold(None, |acc, segment| {
            let lo = segment.start_price.min(segment.end_price);
            let hi = segment.start_price.max(segment.end_price);
            Some(match acc {
                None => (lo, hi),
                Some((old_lo, old_hi)) => (old_lo.min(lo), old_hi.max(hi)),
            })
        })
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("用法: p119_l5_decompose <btc_1m_full.json>")?;
    if args.next().is_some() {
        return Err("参数过多".to_string());
    }
    let config = ThetaConfig::default();
    let loaded = load_bars(Path::new(&path), config.tick.tick_size)?;
    if loaded.bars.is_empty() {
        return Err("输入 bars 为空".to_string());
    }
    let max_bars = std::env::var("P119_MAX_BARS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .map_or(loaded.bars.len(), |value| value.min(loaded.bars.len()));

    println!(
        "P119_INPUT bars={} replay_bars={} first_date={} last_date={}",
        loaded.bars.len(),
        max_bars,
        loaded.first_date,
        loaded.last_date
    );
    println!(
        "P119_RULE pair=p109_strict(view.moves.windows(2)_both_completed,V3_carried_only,auto_pairing) d2_gate_chain=tau/dir/comparable/anchor/extreme(外部复评,level_view.rs:810-869+677-683) events=provide_nest_candidate_events as_of=terminal(replay_bars-1) pan_gate=pub(crate)_不可外部分解 ruling=none_判定留U1复议"
    );

    let terminal = run_terminal_pass(&loaded.bars[..max_bars], &config)?;
    let as_of = max_bars - 1;
    let (hist, close_src) = terminal.cache.causal_series();
    let dif = terminal.cache.macd_dif();
    let tower = &terminal.tower;

    // ── 塔 LevelState 视角（classification.levels[5]；对象类型 LevelState，mod.rs:163）──
    match terminal.classification.levels.get(L5) {
        Some(state) => {
            let completed = state
                .moves
                .iter()
                .filter(|block| block.status == MoveStatus::Completed)
                .count();
            println!(
                "P119_L5_LEVELSTATE levels_len={} l5_moves={} l5_completed={} l5_centers={}",
                terminal.classification.levels.len(),
                state.moves.len(),
                completed,
                state.centers.len()
            );
        }
        None => println!(
            "P119_L5_LEVELSTATE levels_len={} note=塔顶无结构(塔级<6,levels[5]不存在)",
            terminal.classification.levels.len()
        ),
    }

    // ── 塔级护栏：l_max=6（config.rs:82）⟹ 正常 tower.len()=6；不足则如实报「塔顶无结构」──
    if tower.len() <= L5 {
        println!(
            "P119_L5_STRUCTURE tower_levels={} note=塔顶无结构(塔级<6,无L5窗口)",
            tower.len()
        );
        println!("P119_L5 total=0 d2_events=0 extreme_killed=0 other_killed=0 note=塔顶无结构");
        println!(
            "P119_VERDICT branch_a_d2_pair_missing=0 branch_b_extreme_killed=0 branch_b_structural_tau=0 branch_c_other=0 events=0 note=塔顶无结构_不自动下裁定_判定留U1复议"
        );
        return Ok(());
    }

    let lower = lower_legs_from(&tower[L5 - 1])
        .map_err(|error| format!("L5 lower legs 失败: {error:?}"))?;
    let windows = &tower[L5];
    let segments: Vec<Segment> = lower.iter().map(leg_as_segment).collect();
    let anchors = self_anchors(&segments);

    let mut runs = 0usize;
    let mut invalid_windows = 0usize;
    let mut total_moves = 0usize;
    let mut total_completed = 0usize;
    let mut total_events = 0usize;
    let mut pair_index = 0usize;
    let mut d2_events = 0usize;
    let mut extreme_killed = 0usize;
    let mut other_killed = 0usize;
    let mut branch_a = 0usize;
    let mut branch_b_tau = 0usize;
    let mut branch_c = 0usize;

    // ── runs 切分 → 投影 → decompose → C2 view → strict pairs + provider 事件 ──
    // （p109 measure_level V3 分支逐字复刻，p109_chain157_gate_attrition.rs:213-289）
    let mut run_start: Option<usize> = None;
    for index in 0..=windows.len() {
        let valid = index < windows.len()
            && match project_extended_windows_carried_only(std::slice::from_ref(&windows[index]))
            {
                Ok(_) => true,
                Err(_) => {
                    invalid_windows += 1;
                    false
                }
            };
        match (run_start, valid) {
            (None, true) => run_start = Some(index),
            (Some(start), false) => {
                let projection = project_extended_windows_carried_only(&windows[start..index])
                    .map_err(|error| format!("L5 run 投影失败: {error:?}"))?;
                let centers: Vec<_> = projection.seeds.iter().map(|seed| seed.center).collect();
                let blocks = decompose::decompose(&centers);
                let query = LevelViewQuery {
                    level: L5 as u32,
                    coordinate_window: CoordinateWindow {
                        start: projection.seeds.first().expect("nonempty run").start_index,
                        end: projection.seeds.last().expect("nonempty run").end_index,
                    },
                    as_of,
                    version: C2VersionTuple::auto_pairing(),
                };
                let view = assemble_level_view(
                    C2LevelViewConfig { enabled: true },
                    query,
                    LevelViewMaterial {
                        projection: ProjectionMaterial::ExactThree(&projection),
                        move_blocks: &blocks,
                        lower_legs: &lower,
                        hist,
                        dif,
                        close_src,
                    },
                )
                .map_err(|error| format!("L5 C2 assemble 失败: {error:?}"))?;
                let events = provide_nest_candidate_events(
                    L5 as u32,
                    &projection,
                    &blocks,
                    &lower,
                    &view,
                    hist,
                    dif,
                    close_src,
                );
                runs += 1;
                total_moves += view.moves.len();
                total_completed += view
                    .moves
                    .iter()
                    .filter(|m| matches!(m.completion, CompletionStatus::Completed { .. }))
                    .count();
                total_events += events.len();
                // strict pair = 相邻双 Completed view moves（p109:259-273 同口径）。
                for (leave_index, pair) in view.moves.windows(2).enumerate() {
                    let (leave, retest) = (&pair[0], &pair[1]);
                    if !matches!(leave.completion, CompletionStatus::Completed { .. })
                        || !matches!(retest.completion, CompletionStatus::Completed { .. })
                    {
                        continue;
                    }
                    let strict = StrictPair {
                        leave_index,
                        start: leave.start_index,
                        end: retest.end_index,
                        leave_kind: leave.kind,
                        leave_dir: leave.direction,
                        retest_kind: retest.kind,
                    };
                    let outcome = decompose_pair(
                        &strict, &blocks, &projection, &view, &events, &segments, &anchors,
                        as_of,
                    );
                    let side_tag = match outcome.side {
                        Some(Side::Long) => "Long",
                        Some(Side::Short) => "Short",
                        None => "NA",
                    };
                    println!(
                        "P119_CASE pair={} span=[{},{}] side={} d2_event={} kill_gate={} detail={}",
                        pair_index,
                        strict.start,
                        strict.end,
                        side_tag,
                        outcome.d2_event,
                        outcome.gate.tag(),
                        outcome.detail
                    );
                    pair_index += 1;
                    if outcome.d2_event {
                        d2_events += 1;
                    }
                    match outcome.gate {
                        KillGate::Extreme => extreme_killed += 1,
                        KillGate::Comparable
                        | KillGate::Anchor
                        | KillGate::OtherNoCTerminal
                        | KillGate::OtherNoCEnd => {
                            branch_a += 1;
                            other_killed += 1;
                        }
                        KillGate::Tau => {
                            branch_b_tau += 1;
                            other_killed += 1;
                        }
                        KillGate::None => {}
                        _ => {
                            branch_c += 1;
                            other_killed += 1;
                        }
                    }
                }
                run_start = None;
            }
            _ => {}
        }
    }

    println!(
        "P119_L5_STRUCTURE tower_levels={} tower_windows={} runs={} invalid_windows={} moves={} completed={} strict_pairs={} provider_events={}",
        tower.len(),
        windows.len(),
        runs,
        invalid_windows,
        total_moves,
        total_completed,
        pair_index,
        total_events
    );
    if pair_index == 0 {
        println!(
            "P119_L5 total=0 d2_events=0 extreme_killed=0 other_killed=0 note=塔顶无结构(无完成结构对,runs={} moves={})",
            runs, total_moves
        );
    } else {
        println!(
            "P119_L5 total={} d2_events={} extreme_killed={} other_killed={}",
            pair_index, d2_events, extreme_killed, other_killed
        );
    }
    println!(
        "P119_VERDICT branch_a_d2_pair_missing={} branch_b_extreme_killed={} branch_b_structural_tau={} branch_c_other={} events={} note=实测计数_不自动下裁定_判定留U1复议",
        branch_a, extreme_killed, branch_b_tau, branch_c, d2_events
    );
    Ok(())
}

/// 单对分解：先 τ 门（非趋势 leave 走 pan 通道近似归属），再 D2 pair 存在性
/// （缺失时按 provide_divergence_pairs 同序同参外部复评子门），再 structural_pair_span
/// 缝，最后 Extreme 预滤；全通则交叉核对 provider 输出。
#[allow(clippy::too_many_arguments)]
fn decompose_pair(
    pair: &StrictPair,
    blocks: &[MoveBlock],
    projection: &ExactThreeProjection,
    view: &LevelAsOfView,
    events: &[NestCandidateEvent],
    segments: &[Segment],
    anchors: &[Option<Direction>],
    as_of: usize,
) -> CaseOutcome {
    let side = pair.leave_dir.map(|direction| match direction {
        Direction::Down => Side::Long,
        Direction::Up => Side::Short,
    });

    // ── τ 门（level_view.rs:798-799/811-813）：非趋势 leave ⟹ dir=None ⟹ 趋势 D2 零 pair。
    //    pan 通道只报事件有/无（近似归属：turn_source ∈ 对 span；locate/extreme 门
    //    pub(crate) 不可外部分解，模块头 090 声明 5）。
    if pair.leave_kind != MoveKind::Trend {
        if let Some(event) = events.iter().find(|event| {
            event.kind == NestDivergenceKind::Consolidation
                && event.turn_source >= pair.start
                && event.turn_source <= pair.end
        }) {
            return CaseOutcome {
                d2_event: true,
                gate: KillGate::None,
                side: Some(event.side),
                detail: format!(
                    "kind=pan;turn_source={};divergence_confirmed={};judge_at={};归属=turn_source∈span(近似)",
                    event.turn_source, event.divergence_confirmed, event.judge_at
                ),
            };
        }
        return CaseOutcome {
            d2_event: false,
            gate: KillGate::Tau,
            side,
            detail: "leave=Consolidation;趋势D2不适用(dir=None零pair);pan_event=none;pan门(locate/extreme,signal.rs_pub(crate))不可外部分解"
                .to_string(),
        };
    }

    // ── dir 门（防御：Trend 块必有 dir，decompose 不变量；不可达分支如实保留）──
    let Some(direction) = pair.leave_dir else {
        return CaseOutcome {
            d2_event: false,
            gate: KillGate::Dir,
            side: None,
            detail: "leave=Trend但dir=None(decompose不变量破裂?)".to_string(),
        };
    };
    let side = side.expect("Trend leave 必有 side（direction 已 Some）");
    let leave = &blocks[pair.leave_index];
    let retest = &blocks[pair.leave_index + 1];

    // ── D2 pair 存在性：view.pairs 键 = (block_start_center, block_end_center, direction)
    //    （DivergencePairId，level_view.rs:451-456/859-869）。
    let Some(d2) = view.pairs.iter().find(|candidate| {
        candidate.id.block_start_center == leave.start_center
            && candidate.id.block_end_center == leave.end_center
            && candidate.id.direction == direction
    }) else {
        // D2 pair 缺失 ⟹ 外部复评 provide_divergence_pairs 门序
        // （level_view.rs:810-869 同序同参；模块头 090 声明 4）。
        if leave.end_center <= leave.start_center || leave.end_center >= projection.seeds.len() {
            return CaseOutcome {
                d2_event: false,
                gate: KillGate::OtherBlockSpanInvalid,
                side: Some(side),
                detail: format!(
                    "block_span=({},{});seeds_len={}(level_view.rs:814)",
                    leave.start_center,
                    leave.end_center,
                    projection.seeds.len()
                ),
            };
        }
        let prev = projection.seeds[leave.end_center - 1].center;
        let last = projection.seeds[leave.end_center].center;
        if locate_departure_move_a(segments, anchors, &prev, &last, direction).is_none() {
            return CaseOutcome {
                d2_event: false,
                gate: KillGate::Comparable,
                side: Some(side),
                detail: format!(
                    "locate_departure_move_a=None;无可比较前段A;prev_center=({},{});last_center=({},{})",
                    prev.start_index, prev.end_index, last.start_index, last.end_index
                ),
            };
        }
        let Some(c_terminal) = segments.iter().find(|segment| {
            segment.direction == direction
                && segment.start_index >= last.end_index
                && segment.end_index <= as_of
        }) else {
            return CaseOutcome {
                d2_event: false,
                gate: KillGate::OtherNoCTerminal,
                side: Some(side),
                detail: format!(
                    "无C终段(同向段start>={}且end<=as_of={}不存在;level_view.rs:826-832)",
                    last.end_index, as_of
                ),
            };
        };
        let Some(c_start) =
            departure_move_c_start(segments, anchors, &last, direction, c_terminal.start_index)
        else {
            return CaseOutcome {
                d2_event: false,
                gate: KillGate::Anchor,
                side: Some(side),
                detail: format!(
                    "departure_move_c_start=None;C锚定失败;c_terminal_start={};last_center_end={}",
                    c_terminal.start_index, last.end_index
                ),
            };
        };
        let None = segments
            .iter()
            .rev()
            .find(|segment| {
                segment.direction == direction
                    && segment.start_index >= c_start
                    && segment.end_index <= as_of
            })
            .map(|segment| segment.end_index)
        else {
            return CaseOutcome {
                d2_event: false,
                gate: KillGate::OtherPairLogicMismatch,
                side: Some(side),
                detail: "外部复评全通但view.pairs无此pair(复评与provider不一致,不可能即bug)"
                    .to_string(),
            };
        };
        return CaseOutcome {
            d2_event: false,
            gate: KillGate::OtherNoCEnd,
            side: Some(side),
            detail: format!(
                "全离开段c_end不存在(c_start={}起end<=as_of={}的同向段不存在;level_view.rs:847-858)",
                c_start, as_of
            ),
        };
    };

    // ── structural_pair_span 缝（level_view.rs:662/504-517 谓词内联复评）：
    //    leave+retest 双 MoveStatus::Completed 才产 interval_a；view Completed
    //    （TerminalDivergence 证据）不蕴含块 status=Completed。
    if leave.status != MoveStatus::Completed || retest.status != MoveStatus::Completed {
        return CaseOutcome {
            d2_event: false,
            gate: KillGate::OtherRetestNotCompleted,
            side: Some(side),
            detail: format!(
                "view双Completed但块status未Completed;leave_status={:?};retest_status={:?}",
                leave.status, retest.status
            ),
        };
    }

    // ── Extreme 预滤（level_view.rs:669-683 同口径外部复评；037:20 c 包络破 b 包络）──
    let (Some(a), Some(c)) = (
        move_range_envelope(segments, d2.seg_a),
        move_range_envelope(segments, d2.seg_c),
    ) else {
        return CaseOutcome {
            d2_event: false,
            gate: KillGate::OtherEnvelopeUnmappable,
            side: Some(side),
            detail: format!(
                "包络无整支落入段(037:20不可验诚实判负);seg_a=({},{});seg_c=({},{})",
                d2.seg_a.0, d2.seg_a.1, d2.seg_c.0, d2.seg_c.1
            ),
        };
    };
    let extreme = match side {
        Side::Long => c.0 < a.0,
        Side::Short => c.1 > a.1,
    };
    if !extreme {
        return CaseOutcome {
            d2_event: false,
            gate: KillGate::Extreme,
            side: Some(side),
            detail: format!(
                "c未破极值;seg_a=({},{}):[{},{}];seg_c=({},{}):[{},{}]",
                d2.seg_a.0, d2.seg_a.1, a.0, a.1, d2.seg_c.0, d2.seg_c.1, c.0, c.1
            ),
        };
    }

    // ── 全通 ⟹ provider 应已产事件（level_view.rs:705-717）；交叉核对 ──
    match events.iter().find(|event| {
        event.kind == NestDivergenceKind::Trend && event.side == side && event.seg_a == d2.seg_a
    }) {
        Some(event) => CaseOutcome {
            d2_event: true,
            gate: KillGate::None,
            side: Some(side),
            detail: format!(
                "kind=trend;turn_source={};divergence_confirmed={};judge_at={};seg_c=({},{})",
                event.turn_source, event.divergence_confirmed, event.judge_at, d2.seg_c.0,
                d2.seg_c.1
            ),
        },
        None => CaseOutcome {
            d2_event: false,
            gate: KillGate::OtherProviderMismatch,
            side: Some(side),
            detail: "外部复评全通但provider输出无对应事件(不可能即bug,如实报)".to_string(),
        },
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 以下为 p116 骨架逐字复制（数据加载 + 终态重放；进度标签 P116_→P119_）。
// 来源：p116_turnpoint_anchor_existence.rs:241-246（TerminalState）、
//       :542-569（run_terminal_pass）、:1134-1211（LoadedBars/load_bars/
//       BarsJson/date_to_timestamp）。复制非引用（bin 间无共享模块），与 p116 同源维护。
// ─────────────────────────────────────────────────────────────────────────────

struct TerminalState {
    #[allow(dead_code)] // 骨架一致性保留（p116 原字段）；本探针不消费 l0。
    l0: ParseLayer,
    classification: classifier::Classification,
    tower: Vec<Rc<Vec<LeveledMove>>>,
    cache: classifier::TowerCache,
}

fn run_terminal_pass(bars: &[Bar], config: &ThetaConfig) -> Result<TerminalState, String> {
    let mut parser = ParseLayerIncr::new(config);
    let mut cache = classifier::TowerCache::new();
    let mut terminal = None;
    let started = Instant::now();
    for (index, bar) in bars.iter().copied().enumerate() {
        let l0 = parser.append(bar);
        let (classification, tower) =
            classifier::classify_with_tower_incremental(&l0, config, &mut cache);
        if index > 0 && index % 500_000 == 0 {
            eprintln!(
                "P119_TERMINAL_PROGRESS bar={index}/{} elapsed={:.1}s",
                bars.len(),
                started.elapsed().as_secs_f64()
            );
        }
        if index + 1 == bars.len() {
            terminal = Some((l0, classification, tower));
        }
    }
    let (l0, classification, tower) = terminal.ok_or("空 replay")?;
    Ok(TerminalState {
        l0,
        classification,
        tower,
        cache,
    })
}

#[derive(Debug)]
struct LoadedBars {
    bars: Vec<Bar>,
    first_date: String,
    last_date: String,
}

fn load_bars(path: &Path, tick_size: f64) -> Result<LoadedBars, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("读取 {} 失败: {error}", path.display()))?;
    let raw: BarsJson = serde_json::from_str(&text)
        .map_err(|error| format!("{} JSON 解析失败: {error}", path.display()))?;
    let n = raw.closes.len();
    for (name, len) in [
        ("opens", raw.opens.len()),
        ("highs", raw.highs.len()),
        ("lows", raw.lows.len()),
        ("volumes", raw.volumes.len()),
        ("dates", raw.dates.len()),
    ] {
        if len != n {
            return Err(format!("列长度不一致: {name}={len}, closes={n}"));
        }
    }
    let bars = (0..n)
        .map(|index| {
            let open = raw.opens[index];
            let high = raw.highs[index];
            let low = raw.lows[index];
            let close = raw.closes[index];
            let volume = raw.volumes[index];
            Bar {
                source_index: index,
                timestamp: date_to_timestamp(&raw.dates[index]),
                open: quantize(open, tick_size),
                high: quantize(high, tick_size),
                low: quantize(low, tick_size),
                close: quantize(close, tick_size),
                volume: volume as i64,
                untradable: high < open.max(close).max(low)
                    || low > open.min(close).min(high)
                    || open <= 0.0
                    || high <= 0.0
                    || low <= 0.0
                    || close <= 0.0
                    || volume <= 0.0,
            }
        })
        .collect();
    Ok(LoadedBars {
        bars,
        first_date: raw.dates.first().cloned().unwrap_or_default(),
        last_date: raw.dates.last().cloned().unwrap_or_default(),
    })
}

#[derive(Debug, Deserialize)]
struct BarsJson {
    opens: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    closes: Vec<f64>,
    volumes: Vec<f64>,
    dates: Vec<String>,
}

fn date_to_timestamp(date: &str) -> Timestamp {
    let mut digits = String::with_capacity(14);
    for ch in date.chars().take_while(|value| *value != '+') {
        if ch.is_ascii_digit() {
            digits.push(ch);
            if digits.len() == 14 {
                break;
            }
        }
    }
    digits.parse().unwrap_or(0)
}
