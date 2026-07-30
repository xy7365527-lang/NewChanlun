//! task #125（关② 验收枢轴背书类型构成 + 关③ 前置数据 877 池复测）：只读探针。
//!
//! ══════════════ 任务来源与裁定语义 ══════════════
//! - v3 全量重放（修复后口径 T1+T2+T3）已完成：`/tmp/p92_replay_v3.log` 门行
//!   `P92_YIELD ... terminal_confirmed=1213`；dump `/tmp/p92_ckpt_dump_v3.txt`。
//! - T1 复议裁定（`chanlun/escalate/bsp-terminal-endorsement-t1-review-20260718.md`）裁决 3：
//!   C-b 窗口命中的二/三类点不是「破 B 一类点」语义的充分背书；实装 `confirm_side` 谓词宽于
//!   精确化语义的缝隙已登记（收紧实装属新裁定）；H4 实测 6/6 hit 全是二/三类点。
//!   裁决 3-5：缝隙存续期间，凡引用终端门实测读数的报告须携带身份注记。本探针的全部
//!   类型构成输出即该注记所需证据。
//! - 877 池：修复前 877 个 pan 基例链 0/877 有 confirm_side BSP（p114 报告
//!   `chanlun/review-results/p114-relaxation-ceiling-20260717.md:29-30` 引 p109 数据:37）。
//!
//! ══════════════ 090 声明（声明必须与实际能力一致）══════════════
//! - **只读探针**：stdout 唯一输出；不改账本/塔/判据/事件结构；主仓零写入；零 git mutation；
//!   `rust/Cargo.toml` 零改动（autobins 自动发现，与 p92/p117 同款）。
//! - **判定单一来源**：hit/bits 一律取 lib 生产函数 `nest::terminal_bits_at_event`（事件域）与
//!   `nest::terminal_bits_in_book`（单账查法核），口径 `TerminalMatch::CWindow` = T1 裁定2 生产
//!   口径 `[c_start, t*]` = `interval_b.0..=turn_source`，账本 = `levels[ℓ-1].bsp`。**禁 fork**：
//!   本探针无任何谓词复刻，bit 向量即生产函数返回值原样回读。
//! - **类型分类语义（探针侧纯计数，非判据）**：type1 = bits.buy1|sell1；type2 = 非 type1 且
//!   bits.buy2|sell2；type3 = 非 type1/2 且 bits.buy3|sell3；other = 其余（confirm_side 前置
//!   下构造性为 0，仍如实计数）。2/3 共存（`no_exclusive_trichotomy`）在 P125_ENDO_BITS 原始
//!   六 bit 行与 mixed23 列暴露，不被优先级分类吞没。
//! - **未测不声明**：owner=B 回读（裁定 3-4 精确化语义的第二支）属探针几何复刻（p117 归因
//!   §5/§7 已证其仪器脆弱性），本探针**不做**；类型构成以 bit 向量为限，owner 注记由裁定
//!   文书自身携带。C-a（Exact）仅作敏感性对照行（裁定 T1.2 保留常数）。
//! - **877 池窗口坐标**：p109 数据基例条目 `interval_b=(ib0, ib1)` 且 ib1==turn（R1 形态一致），
//!   窗口取 `[ib0, turn]`、账本 `levels[0].bsp`（terminal@1 基例 = L1 事件 ⟹ ℓ-1=0）。
//!   v3 事件精确匹配（side∧interval_b∧turn_source 全等）作坐标漂移对照行（P125_877_V3EVENT）；
//!    headline 判定不依赖事件存活——p109 坐标直接查生产单账核，坐标漂移由对照行如实呈现。
//! - 冒烟口径：P125_MAX_BARS 截断时计数与全量（1213/877 对账数）不等属预期，P125_XCHECK
//!   行如实标注。
//!
//! 用法：`cargo run --release --bin p125_endorsement_audit -- <btc_1m_full.json> <p125_877_cases.tsv>`
//!   env：`P125_MAX_BARS`（截断重放长度；全量对账 1213 仅在不截断时有效）。
//!   877 坐标文件生成：`python3 /tmp/p125_extract_877.py`（复刻 /tmp/p114_ceiling.py 解析，
//!   从 /tmp/p109_full2.txt 提取 terminal@1 全 pan 池 877 链 1166 基例条目）。
//!
//! 输出（stdout）：
//!   P125_INPUT / P125_RULE / P125_PROVIDER / P125_EVENTS / P125_XCHECK
//!   P125_ENDO level=<ℓ>|ALL div_confirmed=<n> terminal_confirmed=<n> type1 type2 type3 other
//!   P125_ENDO_BITS level=<ℓ>|ALL buy1 buy2 buy3 sell1 sell2 sell3 mixed23
//!   P125_ENDO_KIND kind=<Trend|Consolidation> ...
//!   P125_ENDO_CA（C-a 对照）/ P125_ENDO_EVENT（逐背书事件证据行：完整 bit 向量）
//!   P125_877_INPUT / P125_877（headline）/ P125_877_TYPE / P125_877_EVENTS / P125_877_BASES /
//!   P125_877_V3EVENT / P125_877_HIT（逐命中证据行）/ P125_877_NOTE

use newchan_rust::theta_v0::classifier;
use newchan_rust::theta_v0::classifier::decompose;
use newchan_rust::theta_v0::classifier::level_view::{
    assemble_level_view, lower_legs_from, project_extended_windows_carried_only,
    provide_nest_candidate_events, C2LevelViewConfig, C2VersionTuple, CoordinateWindow,
    LevelViewMaterial, LevelViewQuery, NestCandidateEvent, NestDivergenceKind, ProjectionError,
    ProjectionMaterial,
};
use newchan_rust::theta_v0::classifier::nest::{
    terminal_bits_at_event, terminal_bits_in_book, TerminalMatch,
};
use newchan_rust::theta_v0::classifier::recursive_tower::LeveledMove;
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::ParseLayerIncr;
use newchan_rust::theta_v0::types::{quantize, Bar, BspBits, Side, Timestamp};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;
use std::rc::Rc;
use std::time::Instant;

/// #97 身份键（p117/p92 同款）：事件去重键。
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

/// provider 健康计数（p117/p92 同款测量子集：未测不声明）。
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

/// 增量塔重放（逐字复制 p117 run_terminal_pass；终态与 batch bit-equal，p116/p92 BIT_EXACT 锁定）。
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
                "P125_TERMINAL_PROGRESS bar={index}/{} elapsed={:.1}s",
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

/// 终态快照候选收集（逐字复制 p117/p92 collect_snapshot_candidates：合法 run 分区 → 投影 →
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

// ══════════════ 类型分类（探针侧纯计数，非判据；优先级 1>2>3 仅作分桶，原始 bit 行全量暴露） ══════════════

/// bit 向量 → 类型桶名（模块头 090 声明的分类语义）。
fn bits_class(bits: &BspBits) -> &'static str {
    if bits.buy1 || bits.sell1 {
        "type1"
    } else if bits.buy2 || bits.sell2 {
        "type2"
    } else if bits.buy3 || bits.sell3 {
        "type3"
    } else {
        "other"
    }
}

/// 完整 bit 向量证据串（b1b2b3s1s2s3 六位 0/1）。
fn bits_str(bits: &BspBits) -> String {
    format!(
        "{}{}{}{}{}{}",
        u8::from(bits.buy1),
        u8::from(bits.buy2),
        u8::from(bits.buy3),
        u8::from(bits.sell1),
        u8::from(bits.sell2),
        u8::from(bits.sell3)
    )
}

#[derive(Debug, Default, Clone, Copy)]
struct TypeCounts {
    type1: usize,
    type2: usize,
    type3: usize,
    other: usize,
}

impl TypeCounts {
    fn observe(&mut self, bits: &BspBits) {
        match bits_class(bits) {
            "type1" => self.type1 += 1,
            "type2" => self.type2 += 1,
            "type3" => self.type3 += 1,
            _ => self.other += 1,
        }
    }

    fn total(&self) -> usize {
        self.type1 + self.type2 + self.type3 + self.other
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct RawBits {
    buy1: usize,
    buy2: usize,
    buy3: usize,
    sell1: usize,
    sell2: usize,
    sell3: usize,
    mixed23: usize,
}

impl RawBits {
    fn observe(&mut self, bits: &BspBits) {
        self.buy1 += usize::from(bits.buy1);
        self.buy2 += usize::from(bits.buy2);
        self.buy3 += usize::from(bits.buy3);
        self.sell1 += usize::from(bits.sell1);
        self.sell2 += usize::from(bits.sell2);
        self.sell3 += usize::from(bits.sell3);
        self.mixed23 += usize::from((bits.buy2 || bits.sell2) && (bits.buy3 || bits.sell3));
    }
}

// ══════════════ 877 池坐标（p109 数据，经 /tmp/p125_extract_877.py 提取） ══════════════

#[derive(Debug, Clone, Copy)]
struct PanCase {
    chain: usize,
    side: Side,
    turn: usize,
    ib0: usize,
    ib1: usize,
}

fn parse_pan_cases(path: &str) -> Result<Vec<PanCase>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("读取 877 坐标 {path} 失败: {error}"))?;
    let mut out = Vec::new();
    for (lineno, line) in text.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 5 {
            return Err(format!("{path}:{} 字段不足: {line}", lineno + 1));
        }
        let side = match fields[1] {
            "Long" => Side::Long,
            "Short" => Side::Short,
            other => return Err(format!("{path}:{} 未知 side: {other}", lineno + 1)),
        };
        let parse_num = |index: usize| {
            fields[index]
                .parse::<usize>()
                .map_err(|error| format!("{path}:{} 字段{} 解析失败: {error}", lineno + 1, index))
        };
        out.push(PanCase {
            chain: parse_num(0)?,
            side,
            turn: parse_num(2)?,
            ib0: parse_num(3)?,
            ib1: parse_num(4)?,
        });
    }
    Ok(out)
}

/// #218 面 B 研究 bin 锚供给说明（诚实，090）：owner 判同已换两族锚（一/三类核心区间
/// 带判同经账本 `centers` 全功能；二类一类点身份锚判同需 T1 oracle + 事件锚账本）。
/// 本 bin 是归档研究/审计工具，未接事件锚账本——二类判同锚不可解 = 诚实判负（与
/// 生产 gate 全接线读数有别，面 B 注册项；一/三类判同不受影响）。
fn bin_anchor_ctx() -> newchan_rust::theta_v0::classifier::nest::OwnerAnchorCtx<'static> {
    fn never(_: usize) -> Option<(newchan_rust::theta_v0::types::Tick, usize)> {
        None
    }
    newchan_rust::theta_v0::classifier::nest::OwnerAnchorCtx {
        anchor_at: &never,
        event_anchor: (None, None),
    }
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("用法: p125_endorsement_audit <btc_1m_full.json> <p125_877_cases.tsv>")?;
    let cases_path = args
        .next()
        .ok_or("用法: p125_endorsement_audit <btc_1m_full.json> <p125_877_cases.tsv>")?;
    if args.next().is_some() {
        return Err("参数过多".to_string());
    }
    let config = ThetaConfig::default();
    let loaded = load_bars(Path::new(&path), config.tick.tick_size)?;
    if loaded.bars.is_empty() {
        return Err("输入 bars 为空".to_string());
    }
    let max_bars = std::env::var("P125_MAX_BARS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .map_or(loaded.bars.len(), |value| value.min(loaded.bars.len()));
    let as_of = max_bars - 1;
    let truncated = max_bars < loaded.bars.len();
    println!(
        "P125_INPUT bars={} replay_bars={} as_of={} first_date={} last_date={} truncated={}",
        loaded.bars.len(),
        max_bars,
        as_of,
        loaded.first_date,
        loaded.last_date,
        truncated
    );
    println!(
        "P125_RULE book=levels[ℓ-1].bsp window=CWindow[c_start,t*] judge=terminal_bits_at_event/terminal_bits_in_book(生产单一来源,nest.rs,禁fork) 关③P3收紧=Trend:一类∧owner=B(start_index判同,事件自带b_center_start,禁全字段等式) Pan:同向一/二/三类(confirm_side,谓词零改动) type1=buy1|sell1 type2=非1∧(buy2|sell2) type3=非12∧(buy3|sell3) other=其余 ca=Exact(敏感性对照,不收紧) owner回读=start_index判同(关③P3)"
    );

    // ── 塔重放（p117 同款增量）+ 终态事件收集（p117/p92 同款）──
    let terminal = run_terminal_pass(&loaded.bars[..max_bars], &config)?;
    let (hist, close_src) = terminal.cache.causal_series();
    let dif = terminal.cache.macd_dif();
    let mut audit = ProviderAudit::default();
    let events_by_level =
        collect_snapshot_candidates(&terminal.tower, as_of, hist, dif, close_src, &mut audit)?;
    println!(
        "P125_PROVIDER snapshots={} views={} too_short={} invalid_seed={} missing_carried_center={} other={} complete={}",
        audit.snapshots,
        audit.views,
        audit.projection_too_short,
        audit.projection_invalid_seed,
        audit.projection_missing_carried_center,
        audit.projection_other,
        audit.provider_complete()
    );
    let classification = &terminal.classification;

    // ══════════════ 关②：1213 终端确认事件的背书类型构成 ══════════════
    let n_levels = events_by_level.len();
    let mut candidates = 0usize;
    let mut div_confirmed = 0usize;
    let mut trend_confirmed = 0usize;
    let mut pan_confirmed = 0usize;
    let mut per_level_div = vec![0usize; n_levels];
    let mut per_level_types: Vec<TypeCounts> = vec![TypeCounts::default(); n_levels];
    let mut per_level_raw: Vec<RawBits> = vec![RawBits::default(); n_levels];
    let mut kind_types: BTreeMap<&'static str, TypeCounts> = BTreeMap::new();
    let mut ca_hits = 0usize;
    let mut evidence: Vec<String> = Vec::new();
    for (level, events) in events_by_level.iter().enumerate() {
        candidates += events.len();
        for event in events {
            if !event.divergence_confirmed {
                continue;
            }
            div_confirmed += 1;
            per_level_div[level] += 1;
            match event.kind {
                NestDivergenceKind::Trend => trend_confirmed += 1,
                _ => pan_confirmed += 1,
            }
            // 生产单一来源 hit + 完整 bit 向量回读（关③ P3：返回形状扩为判定结构，取 bits 层）。
            if let Some(bits) = terminal_bits_at_event(
                classification,
                event,
                TerminalMatch::CWindow,
                &bin_anchor_ctx(),
            )
            .map(|t| t.bits)
            {
                per_level_types[level].observe(&bits);
                per_level_raw[level].observe(&bits);
                let kind_name = match event.kind {
                    NestDivergenceKind::Trend => "Trend",
                    NestDivergenceKind::Consolidation => "Consolidation",
                };
                kind_types.entry(kind_name).or_default().observe(&bits);
                evidence.push(format!(
                    "P125_ENDO_EVENT level={} kind={} side={:?} turn_source={} window=[{},{}] bits={} class={}",
                    event.level,
                    kind_name,
                    event.side,
                    event.turn_source,
                    event.interval_b.0,
                    event.turn_source,
                    bits_str(&bits),
                    bits_class(&bits)
                ));
            }
            // C-a 敏感性对照（裁定 T1.2 保留常数，不作生产默认）。
            if terminal_bits_at_event(
                classification,
                event,
                TerminalMatch::Exact,
                &bin_anchor_ctx(),
            )
            .is_some()
            {
                ca_hits += 1;
            }
        }
    }
    let total_types: TypeCounts =
        per_level_types
            .iter()
            .fold(TypeCounts::default(), |mut acc, t| {
                acc.type1 += t.type1;
                acc.type2 += t.type2;
                acc.type3 += t.type3;
                acc.other += t.other;
                acc
            });
    let total_raw: RawBits = per_level_raw.iter().fold(RawBits::default(), |mut acc, r| {
        acc.buy1 += r.buy1;
        acc.buy2 += r.buy2;
        acc.buy3 += r.buy3;
        acc.sell1 += r.sell1;
        acc.sell2 += r.sell2;
        acc.sell3 += r.sell3;
        acc.mixed23 += r.mixed23;
        acc
    });
    let terminal_confirmed = total_types.total();
    println!(
        "P125_EVENTS candidates={} divergence_confirmed={} trend_confirmed={} pan_confirmed={} terminal_confirmed={}",
        candidates, div_confirmed, trend_confirmed, pan_confirmed, terminal_confirmed
    );
    println!(
        "P125_XCHECK terminal_confirmed={} v3_full_log=1213 match={} note={}",
        terminal_confirmed,
        terminal_confirmed == 1213,
        if truncated {
            "截断冒烟：不等 1213 属预期（P125_MAX_BARS 在量）"
        } else {
            "全量重放：不等 1213 即实装/管线异常，立查"
        }
    );
    for level in 1..n_levels {
        let t = per_level_types[level];
        println!(
            "P125_ENDO level={} div_confirmed={} terminal_confirmed={} type1={} type2={} type3={} other={}",
            level,
            per_level_div[level],
            t.total(),
            t.type1,
            t.type2,
            t.type3,
            t.other
        );
    }
    println!(
        "P125_ENDO level=ALL div_confirmed={} terminal_confirmed={} type1={} type2={} type3={} other={}",
        div_confirmed,
        terminal_confirmed,
        total_types.type1,
        total_types.type2,
        total_types.type3,
        total_types.other
    );
    for level in 1..n_levels {
        let r = per_level_raw[level];
        println!(
            "P125_ENDO_BITS level={} buy1={} buy2={} buy3={} sell1={} sell2={} sell3={} mixed23={}",
            level, r.buy1, r.buy2, r.buy3, r.sell1, r.sell2, r.sell3, r.mixed23
        );
    }
    println!(
        "P125_ENDO_BITS level=ALL buy1={} buy2={} buy3={} sell1={} sell2={} sell3={} mixed23={}",
        total_raw.buy1,
        total_raw.buy2,
        total_raw.buy3,
        total_raw.sell1,
        total_raw.sell2,
        total_raw.sell3,
        total_raw.mixed23
    );
    for (kind, t) in &kind_types {
        println!(
            "P125_ENDO_KIND kind={} terminal_confirmed={} type1={} type2={} type3={} other={}",
            kind,
            t.total(),
            t.type1,
            t.type2,
            t.type3,
            t.other
        );
    }
    println!(
        "P125_ENDO_CA exact_hits={} of={}（C-a 敏感性对照，裁定 T1.2：验收报告须落 C-a/C-b 对照读数）",
        ca_hits, div_confirmed
    );

    // ══════════════ 关③：877 全 pan 池 v3 口径复测 ══════════════
    let cases = parse_pan_cases(&cases_path)?;
    let n_chains = cases
        .iter()
        .map(|case| case.chain)
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let distinct_bases = cases
        .iter()
        .map(|case| (case.side == Side::Short, case.turn, case.ib0, case.ib1))
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    println!(
        "P125_877_INPUT chains={} base_events={} distinct_bases={} file={}",
        n_chains,
        cases.len(),
        distinct_bases,
        cases_path
    );
    // v3 L1 Consolidation 事件索引（坐标漂移对照）：(short, ib0, ib1, turn_source) 精确键 +
    // (short, ib0) 次级键（coord_shift 检测）。重复键如实计数（预期 0）。
    let mut exact_index: BTreeMap<(bool, usize, usize, usize), usize> = BTreeMap::new();
    let mut c_start_index: BTreeMap<(bool, usize), usize> = BTreeMap::new();
    for event in events_by_level.get(1).map_or(&[][..], Vec::as_slice) {
        if event.kind != NestDivergenceKind::Consolidation {
            continue;
        }
        let short = event.side == Side::Short;
        *exact_index
            .entry((
                short,
                event.interval_b.0,
                event.interval_b.1,
                event.turn_source,
            ))
            .or_default() += 1;
        *c_start_index
            .entry((short, event.interval_b.0))
            .or_default() += 1;
    }
    let exact_dup: usize = exact_index.values().filter(|count| **count > 1).count();
    // book = levels[0].bsp（terminal@1 基例 = L1 事件 ⟹ ℓ-1=0；级别移位同 T1 裁定1）。
    let book0 = classification
        .levels
        .first()
        .map(|state| state.bsp.as_slice());
    let mut chain_endorsed: BTreeMap<usize, bool> = BTreeMap::new();
    let mut base_endorsed = 0usize;
    let mut base_types = TypeCounts::default();
    let mut base_raw = RawBits::default();
    let mut distinct_endorsed: std::collections::BTreeSet<(bool, usize, usize, usize)> =
        std::collections::BTreeSet::new();
    let mut match_exact = 0usize;
    let mut match_coord_shift = 0usize;
    let mut match_none = 0usize;
    let mut hit_lines: Vec<String> = Vec::new();
    for case in &cases {
        // headline：p109 坐标窗 [ib0, turn]，生产单账查法核（单一来源，禁 fork）。
        // 关③ P2/P3：877 池 = 全 pan 基例 ⟹ kind=Consolidation（同向一/二/三类全合法，
        // owner 不入判 ⟹ b_center_start=None）；收紧后复测预期不变（谓词零改动域）。
        let bits = book0.and_then(|book| {
            // #218 面 B：Pan 域 owner 不入判（锚参照包不消费）；centers 传同级账本层
            // （一/三类判同查找键域，Pan 用不到但签名需要——与生产同一核，禁 fork）。
            let centers0 = classification
                .levels
                .first()
                .map(|state| state.centers.as_slice());
            terminal_bits_in_book(
                book,
                centers0.unwrap_or(&[]),
                case.ib0,
                case.turn,
                case.side,
                NestDivergenceKind::Consolidation,
                None,
                TerminalMatch::CWindow,
                &bin_anchor_ctx(),
            )
            .map(|t| t.bits)
        });
        let endorsed = bits.is_some();
        *chain_endorsed.entry(case.chain).or_insert(false) |= endorsed;
        if let Some(bits) = bits {
            base_endorsed += 1;
            base_types.observe(&bits);
            base_raw.observe(&bits);
            distinct_endorsed.insert((case.side == Side::Short, case.turn, case.ib0, case.ib1));
            hit_lines.push(format!(
                "P125_877_HIT chain={} side={:?} turn={} window=[{},{}] bits={} class={}",
                case.chain,
                case.side,
                case.turn,
                case.ib0,
                case.turn,
                bits_str(&bits),
                bits_class(&bits)
            ));
        }
        // v3 事件坐标漂移对照（只计数，不改 headline 判定）。
        let short = case.side == Side::Short;
        if exact_index.contains_key(&(short, case.ib0, case.ib1, case.turn)) {
            match_exact += 1;
        } else if c_start_index.contains_key(&(short, case.ib0)) {
            match_coord_shift += 1;
        } else {
            match_none += 1;
        }
    }
    let chains_with = chain_endorsed
        .values()
        .filter(|endorsed| **endorsed)
        .count();
    let chains_total = chain_endorsed.len();
    println!(
        "P125_877 total={} with_endorsement={} still_silent={}",
        chains_total,
        chains_with,
        chains_total - chains_with
    );
    println!(
        "P125_877_TYPE endorsed_base_events={} type1={} type2={} type3={} other={}（分类型构成：基例事件粒度，headline 同口径）",
        base_endorsed, base_types.type1, base_types.type2, base_types.type3, base_types.other
    );
    println!(
        "P125_877_BITS buy1={} buy2={} buy3={} sell1={} sell2={} sell3={} mixed23={}",
        base_raw.buy1,
        base_raw.buy2,
        base_raw.buy3,
        base_raw.sell1,
        base_raw.sell2,
        base_raw.sell3,
        base_raw.mixed23
    );
    println!(
        "P125_877_EVENTS base_events={} with_endorsement={} still_silent={}",
        cases.len(),
        base_endorsed,
        cases.len() - base_endorsed
    );
    println!(
        "P125_877_BASES distinct_bases={} endorsed={} still_silent={}",
        distinct_bases,
        distinct_endorsed.len(),
        distinct_bases - distinct_endorsed.len()
    );
    println!(
        "P125_877_V3EVENT exact={} coord_shift={} no_event={} exact_dup_keys={}（v3 L1 pan 事件坐标对照：headline 判定不依赖本行；exact<{} 即坐标漂移在量）",
        match_exact,
        match_coord_shift,
        match_none,
        exact_dup,
        cases.len()
    );
    println!(
        "P125_877_NOTE window=[ib0,turn]取自p109基例条目(ib1==turn,R1形态一致) book=levels[0].bsp judge=terminal_bits_in_book(生产单一来源) 修复前口径=turn位格等式0/877(p109:37) 本行=C-b窗口v3复测"
    );

    // ── 证据行（逐背书事件/逐命中基例：完整 bit 向量原样回读）──
    for line in &evidence {
        println!("{line}");
    }
    for line in &hit_lines {
        println!("{line}");
    }
    Ok(())
}

#[derive(Debug)]
struct LoadedBars {
    bars: Vec<Bar>,
    first_date: String,
    last_date: String,
}

/// 数据加载（逐字复制 p117/p92 load_bars / BarsJson / date_to_timestamp）。
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
