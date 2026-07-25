//! task #117（主线阶段1关② T1 验收·H4）：S1a 37 案级别移位核验探针（只读，不改任何生产源码/账本）。
//!
//! 用途（T1 验收必备项，`bsp-terminal-endorsement-ruling-20260718.md` T1 验收线第 1 条）：
//! 对 p112 归因的 37 个 S1a 案（τ 门上下文，`bsp_gate=S1a_tau_none`），在 T1 级别移位后的
//! 新查法下逐案复验。可证伪预测（S1a 施工图 `p117-bsp-repair-design-s1a-20260717.md` §0-6/
//! §3.5/附录 B，任一环节实测不符即回票施工图）：
//!   1. 32 立即案 + 5 延迟案在 `levels[ℓ-1]` 账本 `[c_start, t*]` 窗口（C-b）内全部命中
//!      confirm_side 点（rescued=37/37）；C-a（精确坐标）敏感性对照预测命中 32（5 延迟案 miss）；
//!   2. 门链四布尔（门开/破核/A 配/面积）37/37 全过；
//!   3. C-b 窗口最早点的判决中枢 = D2 趋势的 B（身份保证；裁定复议触发条款 = 反例出现）。
//!
//! ══════════════ 090 声明（声明必须与实际能力一致）══════════════
//! - **只读核验探针**：stdout 唯一输出（无 dump 侧信道）；不改账本/塔/判据/事件结构；
//!   主仓零写入；零 git mutation；`rust/Cargo.toml` 零改动。
//! - **判定单一来源**：hit/verdict 以 lib 生产函数 `nest::terminal_bits_at_event`
//!   （+ `nest::event_bsp_book_level`，口径 `TerminalMatch::CWindow` = T1 裁定2 生产口径）
//!   为准，禁 fork；窗口 `[c_start, t*]` = `event.interval_b.0..=event.turn_source`。
//!   坐标回读（hit=Some(<bar>) 的 bar）为报告用扫描，与 `terminal_bits_in_book` CWindow 臂
//!   同窗口同谓词（min_by_key(source_index)）；与生产函数 is_some 不一致即 fork_mismatch
//!   告警（预期 0）。C-a（`TerminalMatch::Exact`）仅作敏感性对照（裁定 T1.2 保留常数）。
//! - **门链四布尔 = 诊断性几何复刻，非生产判定本身**：`judge_segment` 第一类路径在 D2 趋势
//!   对象（B = pair.last，prev = pair.prev）上的叙事对照——`center_trend_gate`（levels[0]
//!   .moves，与 D2 同链）/ 破 B 核（zg/zd 严格不等式，口径同 signal.rs:309-313）/
//!   `locate_departure_move_a` / `departure_move_c_start` / `AbcDivergence::diverges`
//!   （I(A) vs I(C)=[λ_C, 破核腿终点]，面积坐标映射 `map_src_to_close_idx` 同 p112）。
//!   生产全部门链（含 037:20 破极值合取、anchor provenance、归属中枢frontier迁移等）的
//!   综合结果由 hit（生产函数）如实呈现；门链布尔只解释「为何有/无点」。
//! - **锚（S2）不设字段**：L0 路径 anchors ≡ anchors_self 恒全 Some（T2 裁定证据：L0 恒等），
//!   构造性成立；A/C 配对与 λ_C 即以上述自锚数组求值（与 D2 侧 `provide_divergence_pairs`
//!   的 self_anchors 同构）。
//! - **本轮零 cargo（p116 全量重放 PID 10260 在途纪律）**：**本 bin 未经编译/运行验证**。
//!   全部 API 签名于 2026-07-18 直读 worktree 当日源码核对（nest.rs:462-539、
//!   divergence.rs:739-851、decompose.rs:159-188、signal.rs:285-361、level_view.rs:480-871，
//!   数据加载/塔重放/事件收集骨架逐字复制 p116_turnpoint_anchor_existence.rs）。
//!   `cargo run` 须待 p116 重放完成后执行，首跑即 T1 验收线 H4 实测。
//!
//! ══════════════ 坐标口径注记（R1 迁移）══════════════
//! 37 案坐标为 p112 的 #105 口径（turn = 首腿终点 = c_terminal.end，seg_a/seg_c 引
//! `/tmp/p112_full.txt` P112_CASE 原行；附录 B 已逐项核入 turn/side/seg_c/chains；案例全集
//! 出处 = `/tmp/p109_full2.txt` P109_CHAIN_DETAIL 段，754 链档案，只读，本探针运行时不消费）。
//! 现行 R1 事件：`turn_source = t*`（全合取确认时点）、`interval_b = [c_start, t*]`。
//! 配对 = #105 turn 重建（`provide_divergence_pairs` 的 c_terminal 正向 find 同表达式，R1 未改）
//! → 事件键 (kind=Trend, side, seg_a, interval_b.0=c_start, divergence_confirmed)。
//! R1 全合取下事件未确认（t* 不存在）⟹ event_missing + event_unconfirmed_r1 告警
//! （口径迁移，预期 0；p116 重放在量 R1 后分布）。塔重放 = p116 同款增量
//! `classify_with_tower_incremental`（终态与 batch `classify_with_tower` bit-equal，
//! p116 P116_BIT_EXACT 锁定）；事件收集 = p116 同款 `collect_snapshot_candidates`
//! （provider 终态快照，带 ProviderAudit 健康计数）。
//!
//! ══════════════ bin 注册备查（纪律：不改 rust/Cargo.toml）══════════════
//! 本 bin 走 autobins 自动发现（`src/bin/*.rs`），与 p92/p109/p112/p116 同款——Cargo.toml
//! 内 7 个显式 [[bin]] 均为 `required-features=["backtest_bin"]` 的特例，探针不在其列。
//! 若需显式注册（备查，切勿写入）：
//! ```text
//! [[bin]]
//! name = "p117_s1a_level_recheck"
//! path = "src/bin/p117_s1a_level_recheck.rs"
//! ```
//!
//! 用法（p116 重放完成后）：
//! `cargo run --release --bin p117_s1a_level_recheck -- <btc_1m_full.json>`
//! env：`P117_MAX_BARS`（截断重放长度；硬锚 37 案配对仅全量有效）。
//!
//! 输出（stdout）：逐案一行（任务书格式）
//!   `RECHECK case=<i> turn=<bar> level=<ℓ> book_level=<ℓ-1> window=[c_start,t*]
//!    hit=Some(<bar>)|None gate_open=<bool> broke_core=<bool> a_paired=<bool> area_ok=<bool>
//!    verdict=RESCUED|STILL_SILENT|GATE_FAIL:<tau|broke_core|a_pair|area|event_missing|pair_missing>`
//! 汇总行（任务书格式）：
//!   `P117_RECHECK rescued=<n>/37 still_silent=<n> gate_fail=<n>`
//! 辅助行（诊断/预测对照，不属任务书格式）：P117_INPUT / P117_RULE / P117_PREDICT /
//!   P117_PROVIDER / P117_PROVIDER_EVENTS / P117_PAIR_AUDIT / P117_RECHECK_WARN（逐案告警，
//!   kind ∈ pair_missing|pair_side_mismatch|pair_seg_a_mismatch|pair_c_start_mismatch|
//!   event_missing|event_dup|event_unconfirmed_r1|event_absent|b_center_missing|b_idx_zero|
//!   a_span_mismatch|lambda_c_mismatch|lambda_c_none_broke|fork_mismatch|owner_unknown）/
//!   P117_GATE_PASS / P117_RECHECK_CA / P117_RECHECK_IDENTITY。
//! 事件缺失时 level/book_level 打印案定义值 1/0（37 案全为 D2@1），window 右端打印 NA，
//! gate_open/a_paired 在 pair 在场时仍为实测值，broke_core/area_ok 无 t* 不评估（打印 false
//! 并由 event_missing 告警标注，不计入「实测不符」判读——以 P117_GATE_PASS 行注记为准）。

use newchan_rust::theta_v0::classifier;
use newchan_rust::theta_v0::classifier::decompose::{self, center_trend_gate};
use newchan_rust::theta_v0::classifier::divergence::{
    departure_move_c_start, locate_departure_move_a, AbcDivergence,
};
use newchan_rust::theta_v0::classifier::level_view::{
    assemble_level_view, lower_legs_from, project_extended_windows_carried_only,
    provide_nest_candidate_events, C2LevelViewConfig, C2VersionTuple, CoordinateWindow,
    LevelViewMaterial, LevelViewQuery, LowerLeg, NestCandidateEvent, NestDivergenceKind,
    ProjectionError, ProjectionMaterial,
};
use newchan_rust::theta_v0::classifier::nest::{
    event_bsp_book_level, terminal_bits_at_event, TerminalMatch,
};
use newchan_rust::theta_v0::classifier::recursive_tower::{map_src_to_close_idx, LeveledMove};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::ParseLayerIncr;
use newchan_rust::theta_v0::types::{
    quantize, Bar, Center, Direction, Segment, Side, Timestamp,
};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;
use std::rc::Rc;
use std::time::Instant;

// ══════════════ 37 案硬锚坐标（S1a 施工图附录 B + `/tmp/p112_full.txt` P112_CASE 原行）══════════════
//
// 字段：turn（#105 口径 = seg_c.1 = 首腿终点）、side、seg_a、seg_c（seg_c.0 = c_start）。
// 附录 B 已逐项核对 turn/side/seg_c/chains；seg_a 引 P112_CASE 原行。延迟案（三买 leave 终值
// > turn）= #11/#18/#22/#24/#32；#8（turn=1812973）/#34（turn=4200339）为旧账本 turn 上
// 反向位点 2 案（p112:127-129）；#3（turn=1154662）兼 T4-ratio10 仅败 3 案之一（p112:142）。
#[derive(Debug, Clone, Copy)]
struct CaseCoord {
    turn: usize,
    side: Side,
    seg_a: (usize, usize),
    seg_c: (usize, usize),
}

#[rustfmt::skip]
const CASES: [CaseCoord; 37] = [
    CaseCoord { turn: 867223,  side: Side::Short, seg_a: (866696, 867022),   seg_c: (867071, 867223) },   // #1  立即
    CaseCoord { turn: 977720,  side: Side::Long,  seg_a: (976723, 977458),   seg_c: (977629, 977720) },   // #2  立即
    CaseCoord { turn: 1154662, side: Side::Long,  seg_a: (1154302, 1154427), seg_c: (1154603, 1154662) }, // #3  立即（T4-ratio10 仅败 3 案之一）
    CaseCoord { turn: 1444502, side: Side::Long,  seg_a: (1443997, 1444195), seg_c: (1444331, 1444502) }, // #4  立即
    CaseCoord { turn: 1511155, side: Side::Short, seg_a: (1510289, 1510925), seg_c: (1511033, 1511155) }, // #5  立即
    CaseCoord { turn: 1545297, side: Side::Long,  seg_a: (1544839, 1545074), seg_c: (1545202, 1545297) }, // #6  立即
    CaseCoord { turn: 1684581, side: Side::Short, seg_a: (1683941, 1684307), seg_c: (1684405, 1684581) }, // #7  立即
    CaseCoord { turn: 1812973, side: Side::Short, seg_a: (1812461, 1812918), seg_c: (1812944, 1812973) }, // #8  立即（旧账本 turn 上反向位点）
    CaseCoord { turn: 1890249, side: Side::Short, seg_a: (1889712, 1889979), seg_c: (1890175, 1890249) }, // #9  立即
    CaseCoord { turn: 1963252, side: Side::Long,  seg_a: (1962195, 1962964), seg_c: (1963034, 1963252) }, // #10 立即
    CaseCoord { turn: 2035662, side: Side::Short, seg_a: (2035153, 2035494), seg_c: (2035524, 2035662) }, // #11 延迟 (2063914,2064134)
    CaseCoord { turn: 2043126, side: Side::Short, seg_a: (2042665, 2042926), seg_c: (2042972, 2043126) }, // #12 立即
    CaseCoord { turn: 2090467, side: Side::Short, seg_a: (2089931, 2090155), seg_c: (2090302, 2090467) }, // #13 立即
    CaseCoord { turn: 2136917, side: Side::Short, seg_a: (2136303, 2136796), seg_c: (2136843, 2136917) }, // #14 立即
    CaseCoord { turn: 2178474, side: Side::Short, seg_a: (2178045, 2178296), seg_c: (2178454, 2178474) }, // #15 立即
    CaseCoord { turn: 2195437, side: Side::Short, seg_a: (2194211, 2195150), seg_c: (2195268, 2195437) }, // #16 立即
    CaseCoord { turn: 2241463, side: Side::Long,  seg_a: (2240644, 2241205), seg_c: (2241312, 2241463) }, // #17 立即
    CaseCoord { turn: 2669193, side: Side::Long,  seg_a: (2668016, 2668543), seg_c: (2668906, 2669193) }, // #18 延迟 (2672552,2672592)
    CaseCoord { turn: 3595896, side: Side::Long,  seg_a: (3595279, 3595683), seg_c: (3595811, 3595896) }, // #19 立即
    CaseCoord { turn: 3602474, side: Side::Long,  seg_a: (3602103, 3602278), seg_c: (3602333, 3602474) }, // #20 立即
    CaseCoord { turn: 3818615, side: Side::Long,  seg_a: (3818234, 3818361), seg_c: (3818541, 3818615) }, // #21 立即
    CaseCoord { turn: 3828263, side: Side::Long,  seg_a: (3827769, 3828015), seg_c: (3828115, 3828263) }, // #22 延迟 (3858568,3858607)
    CaseCoord { turn: 3916879, side: Side::Long,  seg_a: (3916169, 3916573), seg_c: (3916719, 3916879) }, // #23 立即
    CaseCoord { turn: 3971708, side: Side::Short, seg_a: (3970897, 3971250), seg_c: (3971407, 3971708) }, // #24 延迟 (3972821,3972852)，S1a 最大链簇
    CaseCoord { turn: 3989658, side: Side::Short, seg_a: (3988504, 3989337), seg_c: (3989432, 3989658) }, // #25 立即
    CaseCoord { turn: 4015179, side: Side::Short, seg_a: (4014529, 4014952), seg_c: (4015041, 4015179) }, // #26 立即
    CaseCoord { turn: 4018257, side: Side::Long,  seg_a: (4017832, 4018070), seg_c: (4018100, 4018257) }, // #27 立即
    CaseCoord { turn: 4032359, side: Side::Short, seg_a: (4031579, 4032102), seg_c: (4032284, 4032359) }, // #28 立即
    CaseCoord { turn: 4044258, side: Side::Short, seg_a: (4043558, 4044121), seg_c: (4044196, 4044258) }, // #29 立即
    CaseCoord { turn: 4108519, side: Side::Short, seg_a: (4107943, 4108212), seg_c: (4108342, 4108519) }, // #30 立即
    CaseCoord { turn: 4172278, side: Side::Short, seg_a: (4171561, 4171885), seg_c: (4172028, 4172278) }, // #31 立即
    CaseCoord { turn: 4186102, side: Side::Short, seg_a: (4185578, 4185847), seg_c: (4185938, 4186102) }, // #32 延迟 (4189334,4189454)
    CaseCoord { turn: 4189745, side: Side::Short, seg_a: (4189197, 4189527), seg_c: (4189641, 4189745) }, // #33 立即
    CaseCoord { turn: 4200339, side: Side::Long,  seg_a: (4199942, 4200135), seg_c: (4200242, 4200339) }, // #34 立即（旧账本 turn 上反向位点）
    CaseCoord { turn: 4298765, side: Side::Short, seg_a: (4298004, 4298330), seg_c: (4298554, 4298765) }, // #35 立即
    CaseCoord { turn: 4321214, side: Side::Short, seg_a: (4320459, 4321032), seg_c: (4321166, 4321214) }, // #36 立即
    CaseCoord { turn: 4324005, side: Side::Long,  seg_a: (4323446, 4323756), seg_c: (4323850, 4324005) }, // #37 立即
];

/// trend direction → side（provide_nest_candidate_events :665-668 同映射：Up=顶背 Short 卖，
/// Down=底背 Long 买；配对交叉核验用——case.side 与 pair.direction 必须同映射成立）。
fn side_of_dir(direction: Direction) -> Side {
    match direction {
        Direction::Up => Side::Short,
        Direction::Down => Side::Long,
    }
}

/// #97 身份键（p116 同款）：事件去重键。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct EventKey {
    level: u32,
    short: bool,
    kind: NestDivergenceKind,
    seg_a: (usize, usize),
    interval_b: (usize, usize),
    interval_a: (usize, usize),
    turn_source: usize,
}

impl From<&NestCandidateEvent> for EventKey {
    fn from(event: &NestCandidateEvent) -> Self {
        Self {
            level: event.level,
            short: event.side == Side::Short,
            kind: event.kind,
            seg_a: event.seg_a,
            interval_b: event.interval_b,
            interval_a: event.interval_a,
            turn_source: event.turn_source,
        }
    }
}

/// provider 健康计数（p116 ProviderAudit 的测量子集——本探针不消费 snapshot_future_violations，
/// 不打印未测量的计数（090：未测不声明））。
#[derive(Debug, Default, Clone, Copy)]
struct ProviderAudit {
    snapshots: usize,
    views: usize,
    projection_too_short: usize,
    projection_invalid_seed: usize,
    projection_missing_carried_center: usize,
    projection_other: usize,
}

impl ProviderAudit {
    fn observe_projection(&mut self, error: ProjectionError) {
        match error {
            ProjectionError::TooShort { .. } => self.projection_too_short += 1,
            ProjectionError::InvalidSeed { .. } => self.projection_invalid_seed += 1,
            ProjectionError::MissingCarriedCenter { .. } => {
                self.projection_missing_carried_center += 1
            }
            ProjectionError::InvalidLowerLeg { .. } => self.projection_other += 1,
        }
    }

    fn provider_complete(&self) -> bool {
        self.projection_too_short == 0
            && self.projection_invalid_seed == 0
            && self.projection_missing_carried_center == 0
            && self.projection_other == 0
    }
}

struct TerminalState {
    classification: classifier::Classification,
    tower: Vec<Rc<Vec<LeveledMove>>>,
    cache: classifier::TowerCache,
}

/// 增量塔重放（逐字复制 p116 run_terminal_pass；终态与 batch bit-equal，p116 BIT_EXACT 锁定）。
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
                "P117_TERMINAL_PROGRESS bar={index}/{} elapsed={:.1}s",
                bars.len(),
                started.elapsed().as_secs_f64()
            );
        }
        if index + 1 == bars.len() {
            terminal = Some((classification, tower));
        }
    }
    let (classification, tower) = terminal.ok_or("空 replay")?;
    Ok(TerminalState {
        classification,
        tower,
        cache,
    })
}

/// 终态快照候选收集（逐字复制 p116 collect_snapshot_candidates：合法 run 分区 → 投影 →
/// decompose → C2 assemble → provide_nest_candidate_events；EventKey 去重）。
fn collect_snapshot_candidates(
    tower: &[Rc<Vec<LeveledMove>>],
    as_of: usize,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
    audit: &mut ProviderAudit,
) -> Result<Vec<Vec<NestCandidateEvent>>, String> {
    audit.snapshots += 1;
    let mut by_level = vec![Vec::new(); tower.len()];
    for level in 1..tower.len() {
        let lower = lower_legs_from(&tower[level - 1])
            .map_err(|error| format!("L{level} lower legs 失败: {error:?}"))?;
        let windows = &tower[level];
        let mut run_start = None;
        for index in 0..=windows.len() {
            let valid = if index < windows.len() {
                match project_extended_windows_carried_only(std::slice::from_ref(&windows[index])) {
                    Ok(_) => true,
                    Err(error) => {
                        audit.observe_projection(error);
                        false
                    }
                }
            } else {
                false
            };
            match (run_start, valid) {
                (None, true) => run_start = Some(index),
                (Some(start), false) => {
                    let projection = project_extended_windows_carried_only(&windows[start..index])
                        .map_err(|error| format!("L{level} run projection 失败: {error:?}"))?;
                    let centers: Vec<_> = projection.seeds.iter().map(|seed| seed.center).collect();
                    let blocks = decompose::decompose(&centers);
                    let start_source = projection.seeds.first().expect("nonempty run").start_index;
                    let end_source = projection.seeds.last().expect("nonempty run").end_index;
                    let query = LevelViewQuery {
                        level: level as u32,
                        coordinate_window: CoordinateWindow {
                            start: start_source,
                            end: end_source,
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
                    .map_err(|error| format!("L{level} C2 assemble 失败: {error:?}"))?;
                    audit.views += 1;
                    by_level[level].extend(provide_nest_candidate_events(
                        level as u32,
                        &projection,
                        &blocks,
                        &lower,
                        &view,
                        hist,
                        dif,
                        close_src,
                    ));
                    run_start = None;
                }
                _ => {}
            }
        }
    }
    for events in &mut by_level {
        events.sort_by_key(|event| EventKey::from(&*event));
        events.dedup_by(|left, right| EventKey::from(&*left) == EventKey::from(&*right));
    }
    Ok(by_level)
}

/// level_view.rs:431 `leg_as_segment` 同口径复刻（p112 同款：LowerLeg → Segment）。
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

/// signal.rs:208 `nearest_confirmed_center_idx` 同口径复刻（p112 同款：前缀末下标）。
fn nearest_center_idx(centers: &[Center], seg_start: usize) -> Option<usize> {
    let hi = centers.partition_point(|c| c.end_index <= seg_start);
    if hi == 0 {
        None
    } else {
        Some(hi - 1)
    }
}

/// D2 pair 的 L0 门链诊断上下文（B/prev 中枢、趋势方向、pair 坐标交叉核验字段）。
#[derive(Debug, Clone, Copy)]
struct PairCtx {
    direction: Direction,
    prev: Center,
    last: Center,
    seg_a: (usize, usize),
    c_start: usize,
}

/// L1 run 逐对收集 D2 pair 上下文（p112 measure_level 的 level==1 分支同款 run 分区/投影/
/// C2 assemble；pair 取自 view.pairs，与事件 seg_a/interval_b 同源——p112 实测 collisions=0、
/// miss_ctx=0 证明该 join 严丝合缝）。键 = #105 turn 重建（c_terminal 正向 find，
/// provide_divergence_pairs :826-832 同表达式，R1 未改此段）。
fn collect_l1_pair_contexts(
    tower: &[Rc<Vec<LeveledMove>>],
    lower: &[LowerLeg],
    as_of: usize,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
) -> Result<(BTreeMap<usize, PairCtx>, usize), String> {
    let mut out: BTreeMap<usize, PairCtx> = BTreeMap::new();
    let mut collisions = 0usize;
    if tower.len() < 2 {
        return Ok((out, collisions));
    }
    let windows = &tower[1];
    let segments: Vec<Segment> = lower.iter().map(leg_as_segment).collect();
    let version = C2VersionTuple::auto_pairing();
    let mut run_start: Option<usize> = None;
    for index in 0..=windows.len() {
        let valid = index < windows.len()
            && project_extended_windows_carried_only(std::slice::from_ref(&windows[index])).is_ok();
        match (run_start, valid) {
            (None, true) => run_start = Some(index),
            (Some(start), false) => {
                let projection = project_extended_windows_carried_only(&windows[start..index])
                    .map_err(|error| format!("L1 run 投影失败: {error:?}"))?;
                let centers: Vec<_> = projection.seeds.iter().map(|seed| seed.center).collect();
                let blocks = decompose::decompose(&centers);
                let query = LevelViewQuery {
                    level: 1,
                    coordinate_window: CoordinateWindow {
                        start: projection.seeds.first().expect("nonempty run").start_index,
                        end: projection.seeds.last().expect("nonempty run").end_index,
                    },
                    as_of,
                    version,
                };
                let view = assemble_level_view(
                    C2LevelViewConfig { enabled: true },
                    query,
                    LevelViewMaterial {
                        projection: ProjectionMaterial::ExactThree(&projection),
                        move_blocks: &blocks,
                        lower_legs: lower,
                        hist,
                        dif,
                        close_src,
                    },
                )
                .map_err(|error| format!("L1 C2 assemble 失败: {error:?}"))?;
                for pair in &view.pairs {
                    let (Some(last_seed), Some(prev_seed)) = (
                        projection.seeds.get(pair.id.block_end_center),
                        pair.id
                            .block_end_center
                            .checked_sub(1)
                            .and_then(|p| projection.seeds.get(p)),
                    ) else {
                        continue;
                    };
                    // #105 turn 重建：离开最后中枢后的第一个同向段终点（R1 前 seg_c.1 口径）。
                    let Some(c_terminal) = segments.iter().find(|segment| {
                        segment.direction == pair.id.direction
                            && segment.start_index >= last_seed.center.end_index
                            && segment.end_index <= as_of
                    }) else {
                        continue;
                    };
                    let ctx = PairCtx {
                        direction: pair.id.direction,
                        prev: prev_seed.center,
                        last: last_seed.center,
                        seg_a: pair.seg_a,
                        c_start: pair.seg_c.0,
                    };
                    if out.insert(c_terminal.end_index, ctx).is_some() {
                        collisions += 1;
                    }
                }
                run_start = None;
            }
            _ => {}
        }
    }
    Ok((out, collisions))
}

/// 门链四布尔实测值（诊断性几何复刻，见模块头 090 声明）。
#[derive(Debug, Default, Clone, Copy)]
struct GateEval {
    gate_open: bool,
    broke_core: bool,
    a_paired: bool,
    area_ok: bool,
}

/// 在 D2 趋势对象（B=ctx.last、prev=ctx.prev）上复刻 L0 门链四环：
/// τ 门开（`center_trend_gate` @ levels[0].moves，与 D2 同链；附录 A：per-run 与全局在被消费
/// 中枢上门值恒等，故用生产全局门）→ 破 B 核（[c_start, t*] 内首个同向破核腿，严格不等式
/// 同 signal.rs:309-313）→ A 可配（`locate_departure_move_a`，自锚）→ 面积过
/// （`AbcDivergence::diverges`，I(A) vs I(C)=[λ_C, 破核腿终点]）。t_star=None（事件缺失）时
/// 破核/面积不评估（false，由 event_missing 告警标注）。warns 收集诊断细节（主循环落盘）。
#[allow(clippy::too_many_arguments)]
fn eval_l0_gate_chain(
    case: &CaseCoord,
    ctx: &PairCtx,
    t_star: Option<usize>,
    segs_l0: &[Segment],
    anchors0: &[Option<Direction>],
    centers0: &[Center],
    gate0: &[Option<Direction>],
    hist: &[f64],
    close_src: &[usize],
    warns: &mut Vec<String>,
) -> GateEval {
    let dir = ctx.direction;
    // 门开：B 在 levels[0].centers 的下标（carried ≡ detect 位格等值 #89，逐字段相等定位）。
    let gate_open = match centers0.iter().position(|c| *c == ctx.last) {
        Some(b_idx) => {
            if b_idx == 0 {
                // §1.3 遗留核验项：c_idx==0 边角——gate[0] 恒 None，必落 GATE_FAIL:tau。
                warns.push(format!(
                    "b_idx_zero b=({}, {})",
                    ctx.last.start_index, ctx.last.end_index
                ));
            }
            gate0.get(b_idx).copied().flatten() == Some(dir)
        }
        None => {
            warns.push(format!(
                "b_center_missing b=({}, {}) —— #89 carried≡detect 违例嫌疑",
                ctx.last.start_index, ctx.last.end_index
            ));
            false
        }
    };
    // 破核：[c_start, t*] 内首个同向腿端点破 B 核（预测 = t3_ext 离开腿，p112 实测 37/37）。
    let broke_leg = t_star.and_then(|t| {
        segs_l0.iter().find(|s| {
            s.direction == dir
                && s.start_index >= case.seg_c.0
                && s.end_index <= t
                && (match dir {
                    Direction::Up => s.end_price > ctx.last.zg,
                    Direction::Down => s.end_price < ctx.last.zd,
                })
        })
    });
    let broke_core = broke_leg.is_some();
    // A 可配：同 helper 同自锚（D2 侧 self_anchors 同构）；结果应逐位等于 pair.seg_a。
    let a_span = locate_departure_move_a(segs_l0, anchors0, &ctx.prev, &ctx.last, dir);
    let a_paired = a_span.is_some();
    if a_span != Some(case.seg_a) {
        warns.push(format!(
            "a_span_mismatch got={a_span:?} expected={:?}",
            case.seg_a
        ));
    }
    // 面积：I(C)=[λ_C, 破核腿终点]（Q5 全区间口径，signal.rs:280-284/:344-353 同构）。
    let area_ok = match (a_span, broke_leg) {
        (Some(a), Some(br)) => {
            match departure_move_c_start(segs_l0, anchors0, &ctx.last, dir, br.start_index) {
                Some(lambda_c) => {
                    if lambda_c != case.seg_c.0 {
                        warns.push(format!(
                            "lambda_c_mismatch got={lambda_c} expected={}",
                            case.seg_c.0
                        ));
                    }
                    let abc = AbcDivergence {
                        seg_a: a,
                        seg_c: (lambda_c, br.end_index),
                        is_trend: true,
                    };
                    match (
                        map_src_to_close_idx(close_src, a.0, a.1),
                        map_src_to_close_idx(close_src, lambda_c, br.end_index),
                    ) {
                        (Some(a_idx), Some(c_idx)) => abc.diverges(hist, a_idx, c_idx),
                        _ => false,
                    }
                }
                None => {
                    // 生产契约：broke 成立 ⟹ λ_C 必 Some（signal.rs:338-342 debug_assert）。
                    warns.push("lambda_c_none_broke —— 生产契约（broke⟹λ_C Some）违例".to_string());
                    false
                }
            }
        }
        _ => false,
    };
    GateEval {
        gate_open,
        broke_core,
        a_paired,
        area_ok,
    }
}


/// #218 面 B 研究 bin 锚供给说明（诚实，090）：owner 判同已换两族锚（一/三类核心区间
/// 带判同经账本 `centers` 全功能；二类一类点身份锚判同需 T1 oracle + 事件锚账本）。
/// 本 bin 是归档研究/审计工具，未接事件锚账本——二类判同锚不可解 = 诚实判负（与
/// 生产 gate 全接线读数有别，面 B 注册项；一/三类判同不受影响）。
fn bin_anchor_ctx() -> newchan_rust::theta_v0::classifier::nest::OwnerAnchorCtx<'static> {
    fn never(_: usize) -> Option<(newchan_rust::theta_v0::types::Tick, usize)> {
        None
    }
    newchan_rust::theta_v0::classifier::nest::OwnerAnchorCtx { anchor_at: &never, event_anchor: (None, None) }
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("用法: p117_s1a_level_recheck <btc_1m_full.json>")?;
    if args.next().is_some() {
        return Err("参数过多".to_string());
    }
    let config = ThetaConfig::default();
    let loaded = load_bars(Path::new(&path), config.tick.tick_size)?;
    if loaded.bars.is_empty() {
        return Err("输入 bars 为空".to_string());
    }
    let max_bars = std::env::var("P117_MAX_BARS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .map_or(loaded.bars.len(), |value| value.min(loaded.bars.len()));
    let as_of = max_bars - 1;
    println!(
        "P117_INPUT bars={} replay_bars={} as_of={} first_date={} last_date={}",
        loaded.bars.len(), max_bars, as_of, loaded.first_date, loaded.last_date
    );
    println!(
        "P117_RULE book=event_bsp_book_level(ℓ-1) window=CWindow[c_start,t*] hit=terminal_bits_at_event(生产单一来源) ca=Exact(敏感性对照) gates=L0叙事复刻(B=pair.last) verdict=event/pair_missing>tau>broke_core>a_pair>area>RESCUED>STILL_SILENT"
    );
    println!(
        "P117_PREDICT rescued=37/37（32立即+5延迟经C-b）gate_pass=37/37/37/37 ca_exact_hits=32 identity_owner_eq_b=全部 source=S1a图§0-6/§3.5"
    );

    // ── 塔重放（p116 同款增量）+ 终态事件收集（p116 同款）──
    let terminal = run_terminal_pass(&loaded.bars[..max_bars], &config)?;
    let (hist, close_src) = terminal.cache.causal_series();
    let dif = terminal.cache.macd_dif();
    let mut audit = ProviderAudit::default();
    let events_by_level =
        collect_snapshot_candidates(&terminal.tower, as_of, hist, dif, close_src, &mut audit)?;
    println!(
        "P117_PROVIDER snapshots={} views={} too_short={} invalid_seed={} missing_carried_center={} other={} complete={}",
        audit.snapshots,
        audit.views,
        audit.projection_too_short,
        audit.projection_invalid_seed,
        audit.projection_missing_carried_center,
        audit.projection_other,
        audit.provider_complete()
    );
    let total_events: usize = events_by_level.iter().map(Vec::len).sum();
    let total_confirmed: usize = events_by_level
        .iter()
        .flat_map(|events| events.iter())
        .filter(|event| event.divergence_confirmed)
        .count();
    let trend_confirmed: usize = events_by_level
        .iter()
        .flat_map(|events| events.iter())
        .filter(|event| event.divergence_confirmed && event.kind == NestDivergenceKind::Trend)
        .count();
    println!(
        "P117_PROVIDER_EVENTS candidates={} divergence_confirmed={} trend_confirmed={} note=R1后实测值（p112硬锚3683/1235/429为R1前口径，仅对照不断言——口径迁移见模块头）",
        total_events, total_confirmed, trend_confirmed
    );

    let classification = &terminal.classification;
    if terminal.tower.len() < 2 || classification.levels.len() < 2 {
        return Err("塔/分类级别不足（须 ≥2）".to_string());
    }
    // ── L0 门链材料（生产同源：段 = D2 侧 lower_legs_from(tower[0]) 同空间；anchors ≡ 自锚）──
    let legs0 = lower_legs_from(&terminal.tower[0])
        .map_err(|error| format!("L0 legs 失败: {error:?}"))?;
    let segs_l0: Vec<Segment> = legs0.iter().map(leg_as_segment).collect();
    let anchors0: Vec<Option<Direction>> =
        segs_l0.iter().map(|s| Some(s.direction)).collect();
    let centers0 = &classification.levels[0].centers;
    let gate0 = center_trend_gate(centers0.len(), &classification.levels[0].moves);
    let (pair_map, pair_collisions) =
        collect_l1_pair_contexts(&terminal.tower, &legs0, as_of, hist, dif, close_src)?;
    println!(
        "P117_PAIR_AUDIT pairs={} turn_collisions={}（collisions 须为 0：#105 turn 键唯一）",
        pair_map.len(),
        pair_collisions
    );
    let l1_events: &[NestCandidateEvent] = events_by_level.get(1).map_or(&[], Vec::as_slice);

    // ── 逐案复验 ──
    let mut rescued = 0usize;
    let mut still_silent = 0usize;
    let mut gate_fail = 0usize;
    let mut pass_gate_open = 0usize;
    let mut pass_broke_core = 0usize;
    let mut pass_a_paired = 0usize;
    let mut pass_area_ok = 0usize;
    let mut ca_hits = 0usize;
    let mut hits = 0usize;
    let mut owner_eq_b = 0usize;
    let mut owner_ne_b = 0usize;
    let mut owner_unknown = 0usize;
    let n_cases = CASES.len();
    for (case_idx, case) in CASES.iter().enumerate() {
        let case_no = case_idx + 1;
        let mut warns: Vec<String> = Vec::new();
        // ① pair 配对（#105 turn 重建键）+ 坐标交叉核验（side/seg_a/c_start）。
        let pair = pair_map.get(&case.turn);
        match pair {
            Some(ctx) => {
                if side_of_dir(ctx.direction) != case.side {
                    warns.push(format!(
                        "pair_side_mismatch got={:?} expected={:?}",
                        side_of_dir(ctx.direction),
                        case.side
                    ));
                }
                if ctx.seg_a != case.seg_a {
                    warns.push(format!(
                        "pair_seg_a_mismatch got={:?} expected={:?}",
                        ctx.seg_a, case.seg_a
                    ));
                }
                if ctx.c_start != case.seg_c.0 {
                    warns.push(format!(
                        "pair_c_start_mismatch got={} expected={}",
                        ctx.c_start, case.seg_c.0
                    ));
                }
            }
            None => {
                warns.push("pair_missing —— #105 turn 重建无匹配 pair（复现缝隙，预期 0）".to_string());
            }
        }
        // ② 事件配对（R1 后：confirmed Trend，键 = side ∧ seg_a ∧ interval_b.0=c_start）。
        let matches: Vec<&NestCandidateEvent> = l1_events
            .iter()
            .filter(|event| {
                event.kind == NestDivergenceKind::Trend
                    && event.side == case.side
                    && event.seg_a == case.seg_a
                    && event.interval_b.0 == case.seg_c.0
                    && event.divergence_confirmed
            })
            .collect();
        if matches.len() > 1 {
            warns.push(format!("event_dup count={}（取首个，须核）", matches.len()));
        }
        let event = matches.first().copied();
        if event.is_none() {
            let unconfirmed = l1_events.iter().any(|e| {
                e.kind == NestDivergenceKind::Trend
                    && e.side == case.side
                    && e.seg_a == case.seg_a
                    && e.interval_b.0 == case.seg_c.0
                    && !e.divergence_confirmed
            });
            if unconfirmed {
                warns.push("event_unconfirmed_r1 —— R1 全合取下事件未确认（t* 不存在；口径迁移，p116 重放在量）".to_string());
            } else {
                warns.push("event_absent —— provider 未复现该事件（复现缝隙，预期 0）".to_string());
            }
        }
        // ③ hit（生产单一来源）+ 坐标回读扫描（报告用，fork 告警守护）+ C-a 敏感性。
        let (level, book_level, window, hit, hit_idx, hit_ca) = match event {
            Some(e) => {
                let book_level = event_bsp_book_level(e.level);
                let hit = terminal_bits_at_event(classification, e, TerminalMatch::CWindow, &bin_anchor_ctx()).is_some();
                let hit_ca =
                    terminal_bits_at_event(classification, e, TerminalMatch::Exact, &bin_anchor_ctx()).is_some();
                let hit_idx = book_level.and_then(|bl| {
                    classification.levels.get(bl).map(|state| {
                        state
                            .bsp
                            .iter()
                            .filter(|p| {
                                e.interval_b.0 <= p.source_index
                                    && p.source_index <= e.turn_source
                                    && p.bits.confirm_side(e.side)
                            })
                            .min_by_key(|p| p.source_index)
                            .map(|p| p.source_index)
                    })
                }).flatten();
                if hit_idx.is_some() != hit {
                    warns.push(format!(
                        "fork_mismatch production={} scan={hit_idx:?} —— 判定以生产函数为准",
                        hit
                    ));
                }
                (e.level, book_level, Some((e.interval_b.0, e.turn_source)), hit, hit_idx, hit_ca)
            }
            None => (1, Some(0), None, false, None, false),
        };
        // ④ 门链四布尔（pair 在场才评估；事件缺失时 broke/area 无 t* 不评估）。
        let gates = match pair {
            Some(ctx) => eval_l0_gate_chain(
                case,
                ctx,
                window.map(|w| w.1),
                &segs_l0,
                &anchors0,
                centers0,
                &gate0,
                hist,
                close_src,
                &mut warns,
            ),
            None => GateEval::default(),
        };
        // ⑤ 身份保证实测：hit 点的判决中枢 = B？（裁定复议触发条款的直接测量）。
        if let Some(idx) = hit_idx {
            hits += 1;
            match segs_l0.iter().find(|s| s.end_index == idx) {
                Some(u) => match nearest_center_idx(centers0, u.start_index) {
                    Some(owner) => {
                        if let Some(ctx) = pair {
                            if centers0[owner] == ctx.last {
                                owner_eq_b += 1;
                            } else {
                                owner_ne_b += 1;
                                warns.push(format!(
                                    "identity_owner_ne_b hit={} owner=({}, {}) b=({}, {})",
                                    idx,
                                    centers0[owner].start_index,
                                    centers0[owner].end_index,
                                    ctx.last.start_index,
                                    ctx.last.end_index
                                ));
                            }
                        }
                    }
                    None => {
                        owner_unknown += 1;
                        warns.push(format!("owner_unknown hit={}（无前驱中枢）", idx));
                    }
                },
                None => {
                    owner_unknown += 1;
                    warns.push(format!("owner_unknown hit={}（无对应 L0 段端点）", idx));
                }
            }
        }
        // ⑥ verdict（优先序见 P117_RULE；hit 由生产函数判定）。
        let verdict = if event.is_none() {
            gate_fail += 1;
            "GATE_FAIL:event_missing"
        } else if pair.is_none() {
            gate_fail += 1;
            "GATE_FAIL:pair_missing"
        } else if !gates.gate_open {
            gate_fail += 1;
            "GATE_FAIL:tau"
        } else if !gates.broke_core {
            gate_fail += 1;
            "GATE_FAIL:broke_core"
        } else if !gates.a_paired {
            gate_fail += 1;
            "GATE_FAIL:a_pair"
        } else if !gates.area_ok {
            gate_fail += 1;
            "GATE_FAIL:area"
        } else if hit {
            rescued += 1;
            "RESCUED"
        } else {
            still_silent += 1;
            "STILL_SILENT"
        };
        pass_gate_open += usize::from(gates.gate_open);
        pass_broke_core += usize::from(gates.broke_core);
        pass_a_paired += usize::from(gates.a_paired);
        pass_area_ok += usize::from(gates.area_ok);
        ca_hits += usize::from(hit_ca);
        for warn in &warns {
            println!("P117_RECHECK_WARN case={case_no} {warn}");
        }
        let window_str = match window {
            Some((c0, t)) => format!("[{c0},{t}]"),
            None => format!("[{},NA]", case.seg_c.0),
        };
        let hit_str = match hit_idx {
            Some(idx) => format!("Some({idx})"),
            None => "None".to_string(),
        };
        let book_str = match book_level {
            Some(bl) => bl.to_string(),
            None => "none".to_string(),
        };
        println!(
            "RECHECK case={} turn={} level={} book_level={} window={} hit={} gate_open={} broke_core={} a_paired={} area_ok={} verdict={}",
            case_no,
            case.turn,
            level,
            book_str,
            window_str,
            hit_str,
            gates.gate_open,
            gates.broke_core,
            gates.a_paired,
            gates.area_ok,
            verdict
        );
    }

    // ── 汇总（任务书格式行 + 诊断/预测对照行）──
    println!(
        "P117_GATE_PASS gate_open={}/{} broke_core={}/{} a_paired={}/{} area_ok={}/{} predicted=37（事件缺失案 broke_core/area_ok 未评估，见模块头注记）",
        pass_gate_open, n_cases, pass_broke_core, n_cases, pass_a_paired, n_cases, pass_area_ok, n_cases
    );
    println!(
        "P117_RECHECK_CA exact_hits={}/{} predicted=32（C-a 敏感性对照，裁定 T1.2：预测差=5 延迟案）",
        ca_hits, n_cases
    );
    println!(
        "P117_RECHECK_IDENTITY hits={} owner_eq_b={} owner_ne_b={} owner_unknown={}（C-b 身份保证实测；owner_ne_b>0 ⟹ 裁定复议触发条款）",
        hits, owner_eq_b, owner_ne_b, owner_unknown
    );
    println!(
        "P117_RECHECK rescued={}/{} still_silent={} gate_fail={}",
        rescued, n_cases, still_silent, gate_fail
    );
    Ok(())
}

#[derive(Debug)]
struct LoadedBars {
    bars: Vec<Bar>,
    first_date: String,
    last_date: String,
}

/// 数据加载（逐字复制 p116 load_bars / BarsJson / date_to_timestamp）。
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
