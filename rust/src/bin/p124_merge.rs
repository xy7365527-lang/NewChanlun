//! task #124（方法 B）：进程级 run 分片并行重放 —— 归并方。
//!
//! 读 S 片 dump（p124_shard_replay 产出，含 bar-loop 行 + LEDGER_* 账本导出），按归并规则
//! 合并，并重放 terminal pass 一次（~730s）重建终态 classification 后单线程重产终态区
//! （FALLBACK/CERT/TURN_CLASS/兜底 probe DIV·TERM——judge_at 用合并 book 回填，首证钟主键
//! = CERT judge_at 向量），输出合并 dump（p92/p116 逐字行，元数据尾一律剥除）与合并门行
//! （前缀 P124M_）。语义逐字复刻现行（p92 终态装配零改动）。
//!
//! ## 归并规则（逐字段）
//!
//! - candidates/divergences：按 key 取 min（分片键=(level, provider_window.0) 互斥 ⟹ 每键
//!   恰属一片，min=first-wins；dup 计数如实上报）。
//! - bar-loop 行序：section（TURN=0 < trigger(DIV/TERM)=1 < CKPT*=2）内按
//!   (at, level, run, shard, seq) 全序——同 (at,level,run) 必同片、seq 保片内发射序
//!   ⟹ 精确还原顺序插入序（trigger 行迭代键 = 终态 run 键的 BTreeSet 升序）。
//! - CKPT/TURN 行：不依赖 targets/pending，跨片完全相同 → 取 shard 0，逐片逐行断言相等
//!   （ckpt_xdiff/turn_xdiff）。
//! - unresolved_targets / prefix views：求和（Σ=顺序值的构造依据见 p124_shard_replay 模块头）。
//! - ProviderAudit terminal-pass 派生计数（too_short/invalid_seed/missing_carried_center/
//!   other/terminal_views）：跨片相同 → 取单片；归并方自有 terminal pass 复算对账（meta_drift）。
//! - 终态 CERT 装配 + judge_at 回填：归并方用合并 book（divergences→candidates→max_bars-1）
//!   重放 terminal pass 后单线程执行——CERT/TURN_CLASS 行由此重新产生，天然有序。
//! - 分片完备性：终态 targets 全集（归并方自产）必须 = Σ片 targets，且每键恰属一片
//!   （partition_missing/partition_extra/dup_owner/hash_drift 全 0）。
//!
//! ## 全量双跑协议（S=8，等编排者指令）
//!
//! 全量 S=8 归并 vs v3 全量 dump（p92 顺序）：门行全套一致（**views 求和须=148,043** 的
//! 现行值 = terminal_views + Σprefix_views）+ **dump 全行含行序 diff=0**（P124_PROBES=0
//! = p92 行集）；首证钟主键 = CERT judge_at 向量逐格一致。
//!
//! ## 用法
//!
//! ```text
//! env P124M_INPUTS=<片dump 逗号列表> P124M_DUMP=<合并dump 路径> \
//!     P124_MAX_BARS=<与片相同> P124_PROBES=<与片相同> \
//!     p124_merge <btc_1m_full.json>
//! ```

use newchan_rust::theta_v0::classifier;
use newchan_rust::theta_v0::classifier::decompose;
use newchan_rust::theta_v0::classifier::level_view::{
    assemble_level_view, lower_legs_from, project_extended_windows_carried_only,
    provide_nest_candidate_events, C2LevelViewConfig, C2VersionTuple, CoordinateWindow,
    LevelViewMaterial, LevelViewQuery, NestCandidateEvent, NestDivergenceKind, ProjectionError,
    ProjectionMaterial,
};
use newchan_rust::theta_v0::classifier::nest::{
    assemble_certificates_snapshot, assemble_typed_certificates, event_bsp_book_level,
    terminal_bits_at_event, terminal_bits_in_book, NestIntervalCaliber, TerminalMatch,
    TypedNestCertificate,
};
use newchan_rust::theta_v0::classifier::recursive_tower::LeveledMove;
use newchan_rust::theta_v0::classifier::turn_class::{
    classify_certificate_turn, is_defer_orphan_event, NestTurnClass,
};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::{ParseLayer, ParseLayerIncr};
use newchan_rust::theta_v0::types::{quantize, Bar, BspBits, Side, Timestamp};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::rc::Rc;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

/// 合并 dump 输出：`P124M_DUMP=<path>`。逐字行（无元数据尾）。
static DUMP: OnceLock<Option<Mutex<BufWriter<File>>>> = OnceLock::new();

fn dump_line(args: std::fmt::Arguments<'_>) {
    let sink = DUMP.get_or_init(|| {
        std::env::var("P124M_DUMP")
            .ok()
            .and_then(|path| File::create(path).ok())
            .map(|file| Mutex::new(BufWriter::new(file)))
    });
    if let Some(sink) = sink {
        if let Ok(mut writer) = sink.lock() {
            let _ = writer
                .write_fmt(args)
                .and_then(|()| writer.write_all(b"\n"));
        }
    }
}

fn dump_flush() {
    if let Some(Some(sink)) = DUMP.get() {
        if let Ok(mut writer) = sink.lock() {
            let _ = writer.flush();
        }
    }
}

/// 分片哈希（写死；与 p124_shard_replay 逐字一致）：
/// FNV-1a 64 混合 (level, run_source)，末态 xor-fold 后取模 S。
fn run_shard(level: u32, run_source: usize, shards: usize) -> usize {
    let mut h = 0xcbf2_9ce4_8422_2325u64; // FNV-1a 64 offset basis
    h = (h ^ u64::from(level)).wrapping_mul(0x0000_0100_0000_01b3); // FNV prime
    h = (h ^ run_source as u64).wrapping_mul(0x0000_0100_0000_01b3);
    h ^= h >> 32;
    (h % shards as u64) as usize
}

/// LEDGER key 编码（p124_shard_replay::enc_key 的逐字互逆）。
/// `L<level>,s<0|1>,k<t|p>,sa<a0>-<a1>,ib<b0>-<b1>,ia<a0>-<a1>,ts<turn_source>`
fn parse_enc_key(text: &str) -> Result<EventKey, String> {
    let mut level = None;
    let mut short = None;
    let mut kind = None;
    let mut seg_a = None;
    let mut interval_b = None;
    let mut interval_a = None;
    let mut turn_source = None;
    for part in text.split(',') {
        // 定长标签前缀剥离（sa/ib/ia/ts 先于 s 匹配；k 的值本身无数字）。
        let (tag, value) = ["sa", "ib", "ia", "ts", "L", "s", "k"]
            .into_iter()
            .find_map(|tag| part.strip_prefix(tag).map(|value| (tag, value)))
            .ok_or_else(|| format!("key 段未知: {part}"))?;
        let parse_pair = |v: &str| -> Result<(usize, usize), String> {
            let (a, b) = v
                .split_once('-')
                .ok_or_else(|| format!("key 区间缺 -: {v}"))?;
            Ok((
                a.parse().map_err(|_| format!("key 区间左非整数: {v}"))?,
                b.parse().map_err(|_| format!("key 区间右非整数: {v}"))?,
            ))
        };
        match tag {
            "L" => level = Some(value.parse().map_err(|_| "key level 非整数")?),
            "s" => short = Some(value == "1"),
            "k" => {
                kind = Some(match value {
                    "t" => NestDivergenceKind::Trend,
                    "p" => NestDivergenceKind::Consolidation,
                    _ => return Err(format!("key kind 未知: {value}")),
                })
            }
            "sa" => seg_a = Some(parse_pair(value)?),
            "ib" => interval_b = Some(parse_pair(value)?),
            "ia" => interval_a = Some(parse_pair(value)?),
            "ts" => turn_source = Some(value.parse().map_err(|_| "key ts 非整数")?),
            _ => return Err(format!("key 段未知: {part}")),
        }
    }
    Ok(EventKey {
        level: level.ok_or("key 缺 L")?,
        short: short.ok_or("key 缺 s")?,
        kind: kind.ok_or("key 缺 k")?,
        seg_a: seg_a.ok_or("key 缺 sa")?,
        interval_b: interval_b.ok_or("key 缺 ib")?,
        interval_a: interval_a.ok_or("key 缺 ia")?,
        turn_source: turn_source.ok_or("key 缺 ts")?,
    })
}

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

#[derive(Debug, Default, Clone, Copy)]
struct ProviderAudit {
    snapshots: usize,
    views: usize,
    projection_too_short: usize,
    projection_invalid_seed: usize,
    projection_missing_carried_center: usize,
    projection_other: usize,
    snapshot_future_violations: usize,
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

/// #97 身份键：与 [`NestEventIdentity`] 字段一一对应，用于对账链覆盖。
type IdentityKey = (u32, usize, (usize, usize));

#[derive(Debug, Default)]
struct YieldBook {
    candidates: BTreeMap<EventKey, usize>,
    divergences: BTreeMap<EventKey, usize>,
    terminal_confirmed: BTreeSet<EventKey>,
    cert_a: BTreeSet<String>,
    cert_b: BTreeSet<String>,
    cert_a_kind: BTreeMap<&'static str, usize>,
    cert_b_kind: BTreeMap<&'static str, usize>,
    d3_edges: usize,
    d3_violations: usize,
    /// #97: B 口径链实际吸收过的事件身份（任意 rung 位置）。
    covered_b: BTreeSet<IdentityKey>,
    /// #97: 走盘整块回退进料口的候选。
    intake_fallbacks: BTreeSet<EventKey>,
    /// p116 探针：TERM 行去重账本（键→首次终端确认 bar；prefix 段由 LEDGER_TERM 预置）。
    term_seen: BTreeMap<EventKey, usize>,
}

struct TerminalState {
    l0: ParseLayer,
    classification: classifier::Classification,
    tower: Vec<Rc<Vec<LeveledMove>>>,
    cache: classifier::TowerCache,
}

// ── 片 dump 解析 ──

#[derive(Debug, Default)]
struct ShardMeta {
    shard: usize,
    shards: usize,
    hash: String,
    max_bars: usize,
    ckpt: usize,
    probes: bool,
    targets: usize,
    prefix_views: usize,
    terminal_views: usize,
    too_short: usize,
    invalid_seed: usize,
    missing_carried_center: usize,
    other: usize,
    unresolved: usize,
    candidates_final: usize,
    divergences_final: usize,
    terminal_confirmed: usize,
    missed: usize,
}

#[derive(Debug)]
struct BarRow {
    text: String,
    at: usize,
    shard: usize,
    seq: usize,
    run: Option<(u32, usize)>,
    section: u8, // 0=TURN 1=trigger(DIV/TERM) 2=CKPT*
}

#[derive(Debug, Default)]
struct ShardDump {
    meta: Option<ShardMeta>,
    /// key → (at, run)
    cand: BTreeMap<EventKey, (usize, (u32, usize))>,
    div: BTreeMap<EventKey, (usize, (u32, usize))>,
    term: BTreeMap<EventKey, (usize, (u32, usize))>,
    pending: BTreeMap<EventKey, (u32, usize)>,
    rows: Vec<BarRow>,
}

#[derive(Debug, Default)]
struct MergeAudit {
    dup_owner: usize,
    dup_cand: usize,
    dup_div: usize,
    dup_term: usize,
    dup_pending: usize,
    hash_drift: usize,
    ledger_orphan: usize,
    meta_drift: usize,
    ckpt_xdiff: usize,
    turn_xdiff: usize,
    bad_rows: usize,
    partition_missing: usize,
    partition_extra: usize,
    own_terminal_drift: usize,
}

fn row_section(text: &str) -> Option<u8> {
    let head = text.split_whitespace().next()?;
    match head {
        "TURN" => Some(0),
        "DIV" | "TERM" => Some(1),
        "CKPT" | "CKPT_STATS" | "CKPT_ERR" => Some(2),
        _ => None,
    }
}

fn parse_shard_dump(path: &str, audit: &mut MergeAudit) -> Result<ShardDump, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("读取片 dump {path} 失败: {error}"))?;
    let mut out = ShardDump::default();
    for (lineno, line) in text.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("LEDGER_") {
            parse_ledger(rest, &mut out, audit, path, lineno)?;
            continue;
        }
        let (row_text, meta) = line
            .split_once(" ||")
            .ok_or_else(|| format!("{path}:{} 行缺归并元数据: {line}", lineno + 1))?;
        let mut at = None;
        let mut shard = None;
        let mut seq = None;
        let mut run = None;
        for kv in meta.split('|') {
            let (k, v) = kv
                .split_once('=')
                .ok_or_else(|| format!("{path}:{} 元数据段缺 =: {kv}", lineno + 1))?;
            match k {
                "at" => at = Some(v.parse().map_err(|_| format!("at 非整数: {v}"))?),
                "shard" => shard = Some(v.parse().map_err(|_| format!("shard 非整数: {v}"))?),
                "seq" => seq = Some(v.parse().map_err(|_| format!("seq 非整数: {v}"))?),
                "run" => {
                    let (l, s) = v.split_once(':').ok_or_else(|| format!("run 缺 :: {v}"))?;
                    run = Some((
                        l.parse().map_err(|_| format!("run level 非整数: {v}"))?,
                        s.parse().map_err(|_| format!("run src 非整数: {v}"))?,
                    ));
                }
                "key" => {} // key 仅片内自洽用；归并以 LEDGER 账本为准
                _ => return Err(format!("{path}:{} 元数据键未知: {kv}", lineno + 1)),
            }
        }
        let section = row_section(row_text).unwrap_or_else(|| {
            audit.bad_rows += 1;
            u8::MAX
        });
        out.rows.push(BarRow {
            text: row_text.to_string(),
            at: at.ok_or_else(|| format!("{path}:{} 缺 at", lineno + 1))?,
            shard: shard.ok_or_else(|| format!("{path}:{} 缺 shard", lineno + 1))?,
            seq: seq.ok_or_else(|| format!("{path}:{} 缺 seq", lineno + 1))?,
            run,
            section,
        });
    }
    Ok(out)
}

fn parse_ledger(
    rest: &str,
    out: &mut ShardDump,
    audit: &mut MergeAudit,
    path: &str,
    lineno: usize,
) -> Result<(), String> {
    let (kind, fields) = rest
        .split_once(' ')
        .ok_or_else(|| format!("{path}:{} LEDGER 行缺字段", lineno + 1))?;
    let mut map = BTreeMap::new();
    for kv in fields.split_whitespace() {
        let (k, v) = kv
            .split_once('=')
            .ok_or_else(|| format!("{path}:{} LEDGER 段缺 =: {kv}", lineno + 1))?;
        map.insert(k, v);
    }
    let get_usize = |k: &str| -> Result<usize, String> {
        map.get(k)
            .ok_or_else(|| format!("{path}:{} LEDGER 缺 {k}", lineno + 1))?
            .parse()
            .map_err(|_| format!("{path}:{} LEDGER {k} 非整数", lineno + 1))
    };
    let get_run = |map: &BTreeMap<&str, &str>| -> Result<(u32, usize), String> {
        let v = map
            .get("run")
            .ok_or_else(|| format!("{path}:{} LEDGER 缺 run", lineno + 1))?;
        let (l, s) = v.split_once(':').ok_or_else(|| format!("run 缺 :: {v}"))?;
        Ok((
            l.parse().map_err(|_| format!("run level 非整数: {v}"))?,
            s.parse().map_err(|_| format!("run src 非整数: {v}"))?,
        ))
    };
    match kind {
        "CAND" | "DIV" | "TERM" => {
            let at = get_usize("at")?;
            let run = get_run(&map)?;
            let key = parse_enc_key(map.get("key").ok_or("LEDGER 缺 key")?)?;
            let book = match kind {
                "CAND" => &mut out.cand,
                "DIV" => &mut out.div,
                _ => &mut out.term,
            };
            if book.insert(key, (at, run)).is_some() {
                match kind {
                    "CAND" => audit.dup_cand += 1,
                    "DIV" => audit.dup_div += 1,
                    _ => audit.dup_term += 1,
                }
            }
        }
        "PENDING" => {
            let run = get_run(&map)?;
            let key = parse_enc_key(map.get("key").ok_or("LEDGER 缺 key")?)?;
            if out.pending.insert(key, run).is_some() {
                audit.dup_pending += 1;
            }
        }
        "META" => {
            out.meta = Some(ShardMeta {
                shard: get_usize("shard")?,
                shards: get_usize("shards")?,
                hash: map.get("hash").ok_or("LEDGER 缺 hash")?.to_string(),
                max_bars: get_usize("max_bars")?,
                ckpt: get_usize("ckpt")?,
                probes: get_usize("probes")? == 1,
                targets: get_usize("targets")?,
                prefix_views: get_usize("prefix_views")?,
                terminal_views: get_usize("terminal_views")?,
                too_short: get_usize("too_short")?,
                invalid_seed: get_usize("invalid_seed")?,
                missing_carried_center: get_usize("missing_carried_center")?,
                other: get_usize("other")?,
                unresolved: get_usize("unresolved")?,
                candidates_final: get_usize("candidates_final")?,
                divergences_final: get_usize("divergences_final")?,
                terminal_confirmed: get_usize("terminal_confirmed")?,
                missed: get_usize("missed")?,
            });
        }
        _ => return Err(format!("{path}:{} LEDGER 类未知: {kind}", lineno + 1)),
    }
    Ok(())
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args.next().ok_or("用法: p124_merge <btc_1m_full.json>")?;
    if args.next().is_some() {
        return Err("参数过多".to_string());
    }
    let inputs = std::env::var("P124M_INPUTS")
        .map_err(|_| "缺 P124M_INPUTS=<片dump 逗号列表>".to_string())?;
    let probes = std::env::var("P124_PROBES").ok().as_deref() == Some("1");

    let mut audit = MergeAudit::default();
    let mut shards: Vec<ShardDump> = Vec::new();
    for input in inputs.split(',') {
        shards.push(parse_shard_dump(input, &mut audit)?);
    }
    if shards.is_empty() {
        return Err("P124M_INPUTS 为空".to_string());
    }
    for shard in &shards {
        if shard.meta.is_none() {
            return Err("片 dump 缺 LEDGER_META".to_string());
        }
    }
    let metas: Vec<&ShardMeta> = shards
        .iter()
        .map(|s| s.meta.as_ref().expect("meta"))
        .collect();
    let shard_count = metas[0].shards;
    let shard0 = metas[0];
    // META 跨片一致性（max_bars/ckpt/probes/hash/shards 须全等；索引须唯一覆盖 0..S）。
    let mut seen_index = BTreeSet::new();
    for meta in &metas {
        if meta.shards != shard_count
            || meta.max_bars != shard0.max_bars
            || meta.ckpt != shard0.ckpt
            || meta.probes != shard0.probes
            || meta.hash != shard0.hash
            || meta.terminal_views != shard0.terminal_views
            || meta.too_short != shard0.too_short
            || meta.invalid_seed != shard0.invalid_seed
            || meta.missing_carried_center != shard0.missing_carried_center
            || meta.other != shard0.other
        {
            audit.meta_drift += 1;
        }
        if !seen_index.insert(meta.shard) {
            audit.meta_drift += 1;
        }
    }
    if metas.len() != shard_count || seen_index.len() != shard_count {
        audit.meta_drift += 1;
    }
    if shard0.probes != probes {
        return Err(format!(
            "P124_PROBES={} 与片 probes={} 不一致",
            u8::from(probes),
            u8::from(shard0.probes)
        ));
    }

    let config = ThetaConfig::default();
    let loaded = load_bars(Path::new(&path), config.tick.tick_size)?;
    if loaded.bars.is_empty() {
        return Err("输入 bars 为空".to_string());
    }
    let max_bars = std::env::var("P124_MAX_BARS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .map_or(loaded.bars.len(), |value| value.min(loaded.bars.len()));
    if max_bars != shard0.max_bars {
        audit.meta_drift += 1;
        eprintln!(
            "P124M_WARN max_bars={} 与片 META max_bars={} 不一致（如实继续）",
            max_bars, shard0.max_bars
        );
    }

    println!(
        "P124M_INPUT bars={} replay_bars={} first_date={} last_date={}",
        loaded.bars.len(),
        max_bars,
        loaded.first_date,
        loaded.last_date
    );
    println!(
        "P124M_RULE cand=dir_and_comparable_and_extreme weak=divergence_stage interval_production=B typed=trend_or_consolidation d3=sidecar terminal=BspBits_confirm_side clock=first_prefix created_at=forbidden"
    );

    // ── 分片完备性 + 账本合并（candidates/divergences 按 key 取 min = first-wins）──
    // owners：key → shard（每键恰属一片：CAND∪PENDING 跨片互斥为分片正确性硬证据）。
    let mut owners: BTreeMap<EventKey, usize> = BTreeMap::new();
    let mut book = YieldBook::default();
    let mut prefix_views_sum = 0usize;
    let mut unresolved_sum = 0usize;
    let mut shard_targets_sum = 0usize;
    let mut sum_candidates_final = 0usize;
    let mut sum_divergences_final = 0usize;
    let mut sum_terminal_confirmed = 0usize;
    let mut sum_missed = 0usize;
    for shard in &shards {
        let meta = shard.meta.as_ref().expect("meta");
        prefix_views_sum += meta.prefix_views;
        unresolved_sum += meta.unresolved;
        shard_targets_sum += meta.targets;
        sum_candidates_final += meta.candidates_final;
        sum_divergences_final += meta.divergences_final;
        sum_terminal_confirmed += meta.terminal_confirmed;
        sum_missed += meta.missed;
        for (key, &(at, run)) in &shard.cand {
            if run_shard(run.0, run.1, shard_count) != meta.shard {
                audit.hash_drift += 1;
            }
            if owners.insert(key.clone(), meta.shard).is_some() {
                audit.dup_owner += 1;
            }
            book.candidates.entry(key.clone()).or_insert(at);
        }
        for (key, run) in &shard.pending {
            if run_shard(run.0, run.1, shard_count) != meta.shard {
                audit.hash_drift += 1;
            }
            if owners.insert(key.clone(), meta.shard).is_some() {
                audit.dup_owner += 1;
            }
        }
        for (key, &(at, run)) in &shard.div {
            if !shard.cand.contains_key(key) {
                audit.ledger_orphan += 1;
            }
            if run_shard(run.0, run.1, shard_count) != meta.shard {
                audit.hash_drift += 1;
            }
            book.divergences.entry(key.clone()).or_insert(at);
        }
        for (key, &(at, _run)) in &shard.term {
            if !shard.cand.contains_key(key) {
                audit.ledger_orphan += 1;
            }
            book.term_seen.entry(key.clone()).or_insert(at);
        }
        if meta.unresolved != shard.pending.len() {
            audit.meta_drift += 1;
        }
    }

    // ── bar-loop 合并：CKPT/TURN 跨片断言相等后取 shard 0；trigger 行全序合并 ──
    let turn_x = cross_shard_rows_equal(&shards, 0);
    let ckpt_x = cross_shard_rows_equal(&shards, 2);
    audit.turn_xdiff = turn_x;
    audit.ckpt_xdiff = ckpt_x;
    let mut merged_rows: Vec<&BarRow> = Vec::new();
    for shard in &shards {
        let is_shard0 = shard.meta.as_ref().expect("meta").shard == 0;
        for row in &shard.rows {
            if row.section == u8::MAX {
                continue; // bad_rows 已计
            }
            if row.section != 1 && !is_shard0 {
                continue; // TURN/CKPT 取 shard 0（跨片相等已断言）
            }
            merged_rows.push(row);
        }
    }
    merged_rows.sort_by_key(|row| {
        (
            row.at,
            row.section,
            row.run.map_or(0, |(l, _)| l),
            row.run.map_or(0, |(_, s)| s),
            row.shard,
            row.seq,
        )
    });
    for row in &merged_rows {
        dump_line(format_args!("{}", row.text));
    }

    // ── terminal pass 重放（~730s；终态 classification 重建，终态区单线程重产）──
    let mut paudit = ProviderAudit::default();
    let terminal = run_terminal_pass(&loaded.bars[..max_bars], &config)?;
    let terminal_views_own = paudit.views;
    let (hist, close_src) = terminal.cache.causal_series();
    let dif = terminal.cache.macd_dif();
    let terminal_events = collect_snapshot_candidates(
        &terminal.tower,
        max_bars - 1,
        hist,
        dif,
        close_src,
        &mut paudit,
    )?;
    let terminal_views_own = paudit.views - terminal_views_own;
    if terminal_views_own != shard0.terminal_views
        || paudit.projection_too_short != shard0.too_short
        || paudit.projection_invalid_seed != shard0.invalid_seed
        || paudit.projection_missing_carried_center != shard0.missing_carried_center
        || paudit.projection_other != shard0.other
    {
        audit.own_terminal_drift += 1;
    }
    let mut targets = BTreeMap::new();
    for event in terminal_events.iter().flatten() {
        targets.insert(EventKey::from(event), *event);
    }
    // 分片完备性：终态 targets 全集（归并方自产）vs 片账本 owners。
    for key in targets.keys() {
        if !owners.contains_key(key) {
            audit.partition_missing += 1;
        }
    }
    for key in owners.keys() {
        if !targets.contains_key(key) {
            audit.partition_extra += 1;
        }
    }
    if targets.len() != shard_targets_sum {
        audit.meta_drift += 1;
    }

    // ── judge_at 回填（合并 book：divergences→candidates→max_bars-1，p92 公式逐字）──
    let l0 = terminal.l0;
    let classification = terminal.classification;
    let tower = terminal.tower;
    let cache = terminal.cache;
    let mut final_events = terminal_events;
    for events in &mut final_events {
        for event in events {
            let key = EventKey::from(&*event);
            event.judge_at = book
                .divergences
                .get(&key)
                .or_else(|| book.candidates.get(&key))
                .copied()
                .unwrap_or(max_bars - 1);
        }
    }
    observe_snapshot(
        &classification,
        final_events,
        max_bars - 1,
        &mut book,
        &mut paudit,
        probes,
    );
    paudit.snapshots += book.candidates.len();

    // ── BIT_EXACT（p92 逐字）──
    let (full_classification, full_tower) = classifier::classify_with_tower(&l0, &config);
    let classification_diff = usize::from(full_classification != classification);
    let tower_diff = usize::from(full_tower != tower);
    let mut old_semantic_diff = tower_diff;
    let mut lifecycle_diff = 0usize;
    let levels = full_classification
        .levels
        .len()
        .max(classification.levels.len());
    for level in 0..levels {
        match (
            full_classification.levels.get(level),
            classification.levels.get(level),
        ) {
            (Some(full), Some(incremental)) => {
                old_semantic_diff += usize::from(full.moves != incremental.moves)
                    + usize::from(full.centers != incremental.centers)
                    + usize::from(full.bsp != incremental.bsp)
                    + usize::from(full.pan_div != incremental.pan_div);
                lifecycle_diff += usize::from(full.cp_ownership != incremental.cp_ownership);
            }
            _ => old_semantic_diff += 1,
        }
    }
    if classification_diff != 0 {
        let levels = full_classification
            .levels
            .len()
            .max(classification.levels.len());
        for level in 0..levels {
            let full = full_classification.levels.get(level);
            let incremental = classification.levels.get(level);
            let summary = match (full, incremental) {
                (Some(full), Some(incremental)) => format!(
                    "moves={} centers={} ownership={} bsp={} pan={}",
                    usize::from(full.moves != incremental.moves),
                    usize::from(full.centers != incremental.centers),
                    usize::from(full.cp_ownership != incremental.cp_ownership),
                    usize::from(full.bsp != incremental.bsp),
                    usize::from(full.pan_div != incremental.pan_div),
                ),
                _ => "missing_level=1".to_string(),
            };
            println!("P124M_BIT_DIFF_LEVEL L{level} {summary}");
        }
    }
    let old_events =
        classifier::cand_delta_tower_cached(&l0, &classification, &tower, &config, &cache);
    let old_candidates = old_events
        .iter()
        .flatten()
        .filter(|event| event.cand_delta)
        .count();
    let old_terminal = old_events
        .iter()
        .enumerate()
        .map(|(level, events)| {
            events
                .iter()
                .filter(|event| {
                    event.cand_delta
                        && terminal_bits_old(
                            &classification,
                            level,
                            event.c_episode_start,
                            event.confirm_src,
                            event.side,
                            event.b_parent.map(|p| p.source_interval.0),
                        )
                        .is_some()
                })
                .count()
        })
        .sum::<usize>();
    let mut old_certificates = 0usize;
    for exec in 0..old_events.len() {
        for top in exec..old_events.len() {
            old_certificates += assemble_certificates_snapshot(&old_events, exec, top, |event| {
                terminal_bits_old(
                    &classification,
                    exec,
                    event.c_episode_start,
                    event.confirm_src,
                    event.side,
                    event.b_parent.map(|p| p.source_interval.0),
                )
            })
            .len();
        }
    }

    let trend_candidates = book
        .candidates
        .keys()
        .filter(|key| key.kind == NestDivergenceKind::Trend)
        .count();
    let pan_candidates = book.candidates.len() - trend_candidates;
    let trend_divergences = book
        .divergences
        .keys()
        .filter(|key| key.kind == NestDivergenceKind::Trend)
        .count();
    let pan_divergences = book.divergences.len() - trend_divergences;
    let d3_rate = if book.d3_edges == 0 {
        0.0
    } else {
        book.d3_violations as f64 / book.d3_edges as f64
    };

    println!(
        "P124M_MERGE shards={} shard_targets={} terminal_targets={} partition_missing={} partition_extra={} dup_owner={} dup_cand={} dup_div={} dup_term={} dup_pending={} hash_drift={} ledger_orphan={} meta_drift={} ckpt_xdiff={} turn_xdiff={} bad_rows={} own_terminal_drift={}",
        shard_count,
        shard_targets_sum,
        targets.len(),
        audit.partition_missing,
        audit.partition_extra,
        audit.dup_owner,
        audit.dup_cand,
        audit.dup_div,
        audit.dup_term,
        audit.dup_pending,
        audit.hash_drift,
        audit.ledger_orphan,
        audit.meta_drift,
        audit.ckpt_xdiff,
        audit.turn_xdiff,
        audit.bad_rows,
        audit.own_terminal_drift,
    );
    let shard_sum_equal = sum_candidates_final == book.candidates.len()
        && sum_divergences_final == book.divergences.len()
        && sum_terminal_confirmed == book.terminal_confirmed.len()
        && sum_missed
            == book
                .terminal_confirmed
                .iter()
                .filter(|key| {
                    !book
                        .covered_b
                        .contains(&(key.level, key.turn_source, key.interval_b))
                })
                .count();
    println!(
        "P124M_SHARD_SUM candidates={} divergences={} terminal={} missed={} prefix_views={} unresolved={} equal_to_merged={}",
        sum_candidates_final,
        sum_divergences_final,
        sum_terminal_confirmed,
        sum_missed,
        prefix_views_sum,
        unresolved_sum,
        shard_sum_equal,
    );
    println!(
        "P124M_YIELD candidates={} trend_candidates={} pan_candidates={} divergence_confirmed={} trend_divergence={} pan_divergence={} terminal_confirmed={}",
        book.candidates.len(),
        trend_candidates,
        pan_candidates,
        book.divergences.len(),
        trend_divergences,
        pan_divergences,
        book.terminal_confirmed.len()
    );
    println!(
        "P124M_CERT caliber_A={} A_trend={} A_pan={} A_mixed={} caliber_B={} B_trend={} B_pan={} B_mixed={}",
        book.cert_a.len(),
        book.cert_a_kind.get("trend").copied().unwrap_or(0),
        book.cert_a_kind.get("pan").copied().unwrap_or(0),
        book.cert_a_kind.get("mixed").copied().unwrap_or(0),
        book.cert_b.len(),
        book.cert_b_kind.get("trend").copied().unwrap_or(0),
        book.cert_b_kind.get("pan").copied().unwrap_or(0),
        book.cert_b_kind.get("mixed").copied().unwrap_or(0),
    );
    println!(
        "P124M_D3 edges={} violations={} rate={:.9}",
        book.d3_edges, book.d3_violations, d3_rate
    );
    println!(
        "P124M_BASELINE old_candidates={} old_terminal_confirmed={} old_certificates={} new_candidates={} new_terminal_confirmed={} new_B_certificates={}",
        old_candidates,
        old_terminal,
        old_certificates,
        book.candidates.len(),
        book.terminal_confirmed.len(),
        book.cert_b.len()
    );
    println!(
        "P124M_PROVIDER snapshots={} views={} prefix_views={} terminal_views={} too_short={} invalid_seed={} missing_carried_center={} other={} unresolved_targets={} complete={}",
        paudit.snapshots,
        paudit.views + prefix_views_sum,
        prefix_views_sum,
        paudit.views,
        paudit.projection_too_short,
        paudit.projection_invalid_seed,
        paudit.projection_missing_carried_center,
        paudit.projection_other,
        unresolved_sum,
        paudit.provider_complete() && unresolved_sum == 0
    );
    println!(
        "P124M_SNAPSHOT future_violations={} created_at_reads=0 no_forward={}",
        paudit.snapshot_future_violations,
        paudit.snapshot_future_violations == 0
    );
    println!(
        "P124M_BIT_EXACT old_path_diff={} tower_diff={} moves_centers_bsp_pan_diff={} lifecycle_cp_ownership_diff={} classification_total_diff={}",
        old_semantic_diff,
        tower_diff,
        old_semantic_diff.saturating_sub(tower_diff),
        lifecycle_diff,
        classification_diff
    );
    println!(
        "P124M_R7 provider_complete={} definition_faithful=true snapshot_no_forward={} B_zero={}",
        paudit.provider_complete() && unresolved_sum == 0,
        paudit.snapshot_future_violations == 0,
        book.cert_b.is_empty()
    );
    // #97: 遗漏对账 —— 钟位可证的候选却从未被任何 B 链吸收，逐条枚举（p92 逐字）。
    let missed: Vec<&EventKey> = book
        .terminal_confirmed
        .iter()
        .filter(|key| {
            !book
                .covered_b
                .contains(&(key.level, key.turn_source, key.interval_b))
        })
        .collect();
    println!(
        "P124M_MISSED terminal_confirmed={} covered_b={} intake_fallback_events={} missed={}",
        book.terminal_confirmed.len(),
        book.covered_b.len(),
        book.intake_fallbacks.len(),
        missed.len()
    );
    for key in &missed {
        println!(
            "P124M_MISSED_EVENT level={} kind={:?} short={} seg_a={:?} interval_b={:?} interval_a={:?} turn_source={} intake_fallback={}",
            key.level,
            key.kind,
            key.short,
            key.seg_a,
            key.interval_b,
            key.interval_a,
            key.turn_source,
            book.intake_fallbacks.contains(*key)
        );
    }
    dump_flush();
    Ok(())
}

/// CKPT/TURN 跨片相等性断言：返回与 shard 0 的 section 行序列不一致的片数。
/// 逐行逐字比较（去元数据尾后的 p92/p116 行）。
fn cross_shard_rows_equal(shards: &[ShardDump], section: u8) -> usize {
    let per_shard: Vec<Vec<&str>> = shards
        .iter()
        .map(|shard| {
            let mut rows: Vec<&BarRow> = shard
                .rows
                .iter()
                .filter(|row| row.section == section)
                .collect();
            rows.sort_by_key(|row| row.seq);
            rows.into_iter().map(|row| row.text.as_str()).collect()
        })
        .collect();
    let Some(base) = per_shard.first() else {
        return 0;
    };
    per_shard
        .iter()
        .skip(1)
        .filter(|rows| *rows != base)
        .count()
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
                "P124M_TERMINAL_PROGRESS bar={index}/{} elapsed={:.1}s",
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

/// 终态快照装配（p92 逐字 + p116 探针兜底（probes=1 时 DIV/TERM 终态兜底行））：
/// 在合并 book 上运行——candidates/divergences/term_seen 已由 LEDGER 预置，
/// 终态兜底 or_insert 语义与顺序逐字一致；CERT/TURN_CLASS 行由此重新产生，天然有序。
fn observe_snapshot(
    classification: &classifier::Classification,
    mut current: Vec<Vec<NestCandidateEvent>>,
    as_of: usize,
    book: &mut YieldBook,
    audit: &mut ProviderAudit,
    probes: bool,
) {
    for events in &mut current {
        for event in events {
            if event.interval_a.1 > as_of || event.interval_b.1 > as_of || event.turn_source > as_of
            {
                audit.snapshot_future_violations += 1;
            }
            let key = EventKey::from(&*event);
            if event.intake_fallback && book.intake_fallbacks.insert(key.clone()) {
                dump_line(format_args!(
                    "FALLBACK as_of={} level={} kind={:?} side={:?} turn_source={} seg_a={:?} interval_a={:?} interval_b={:?} divergence_confirmed={} judge_at={}",
                    as_of,
                    event.level,
                    event.kind,
                    event.side,
                    event.turn_source,
                    event.seg_a,
                    event.interval_a,
                    event.interval_b,
                    event.divergence_confirmed,
                    event.judge_at,
                ));
            }
            let candidate_at = *book.candidates.entry(key.clone()).or_insert(as_of);
            if event.interval_a.1 > candidate_at
                || event.interval_b.1 > candidate_at
                || event.turn_source > candidate_at
            {
                audit.snapshot_future_violations += 1;
            }
            if event.divergence_confirmed {
                // p116 探针：DIV 终态兜底（合并 divergences 未覆盖的首插同样落盘；or_insert 逐字）。
                if probes && !book.divergences.contains_key(&key) {
                    dump_line(format_args!(
                        "DIV level={} end={} kind={} side={:?}",
                        event.level,
                        event.turn_source,
                        nest_kind_tag(event.kind),
                        event.side,
                    ));
                }
                let first = *book.divergences.entry(key.clone()).or_insert(as_of);
                event.judge_at = first;
                if terminal_bits_new(classification, event).is_some() {
                    book.terminal_confirmed.insert(key.clone());
                    // p116 探针：TERM 终态兜底（合并 term_seen 全程去重）。
                    if probes && !book.term_seen.contains_key(&key) {
                        book.term_seen.insert(key.clone(), as_of);
                        dump_line(format_args!(
                            "TERM level={} bar={} side={:?}",
                            event.level, event.turn_source, event.side,
                        ));
                    }
                }
            } else {
                event.judge_at = *book.candidates.get(&key).expect("inserted");
            }
        }
    }
    for exec in 1..current.len() {
        for top in exec..current.len() {
            observe_certificates(
                &current,
                classification,
                exec,
                top,
                as_of,
                NestIntervalCaliber::A,
                &mut book.cert_a,
                &mut book.cert_a_kind,
                &mut book.d3_edges,
                &mut book.d3_violations,
                false,
                None,
            );
            observe_certificates(
                &current,
                classification,
                exec,
                top,
                as_of,
                NestIntervalCaliber::B,
                &mut book.cert_b,
                &mut book.cert_b_kind,
                &mut book.d3_edges,
                &mut book.d3_violations,
                true,
                Some(&mut book.covered_b),
            );
        }
    }
    // p118 关④ TURN_CLASS 侧信道：039:34 defer 孤儿发射（只写不判）——p92 逐字。
    for events in &current {
        for event in events {
            let covered =
                book.covered_b
                    .contains(&(event.level, event.turn_source, event.interval_b));
            if is_defer_orphan_event(event, covered) {
                dump_line(format_args!(
                    "TURN_CLASS ids={}:{}:{}-{} class=DeferOrphan confirmed_vec=0",
                    event.level, event.turn_source, event.interval_b.0, event.interval_b.1,
                ));
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn observe_certificates(
    events: &[Vec<NestCandidateEvent>],
    classification: &classifier::Classification,
    exec: usize,
    top: usize,
    as_of: usize,
    caliber: NestIntervalCaliber,
    seen: &mut BTreeSet<String>,
    kinds: &mut BTreeMap<&'static str, usize>,
    d3_edges: &mut usize,
    d3_violations: &mut usize,
    count_d3: bool,
    mut covered: Option<&mut BTreeSet<IdentityKey>>,
) {
    let certificates = assemble_typed_certificates(events, exec, top, caliber, |event| {
        terminal_bits_new(classification, event)
    });
    for certificate in certificates {
        if let Some(covered) = covered.as_deref_mut() {
            for identity in certificate.identities() {
                covered.insert((identity.level, identity.turn_source, identity.interval_b));
            }
        }
        let key = certificate_key(exec, top, &certificate);
        if seen.insert(key) {
            let bucket = certificate_kind(&certificate);
            *kinds.entry(bucket).or_default() += 1;
            if count_d3 {
                let (edges, violations) = certificate.d3_descent_stats();
                *d3_edges += edges;
                *d3_violations += violations;
            }
            // #98/#99 侧信道：身份向量（高→低，含基例）为 A/B 跨口径对账主键；
            // judge_at 向量供 D3 逐边离线复算。只写不判。——首证钟主键 = judge_at 向量。
            let ids = certificate
                .identities()
                .iter()
                .map(|id| {
                    format!(
                        "{}:{}:{}-{}",
                        id.level, id.turn_source, id.interval_b.0, id.interval_b.1
                    )
                })
                .collect::<Vec<_>>()
                .join("|");
            let kinds_str = certificate
                .kinds()
                .iter()
                .map(|kind| format!("{kind:?}"))
                .collect::<Vec<_>>()
                .join(",");
            let clocks = certificate
                .judge_at()
                .iter()
                .map(|clock| clock.to_string())
                .collect::<Vec<_>>()
                .join(",");
            dump_line(format_args!(
                "CERT caliber={:?} exec={} top={} as_of={} side={:?} bucket={} kinds={} judge_at={} ids={}",
                certificate.caliber(),
                exec,
                top,
                as_of,
                certificate.certificate().side(),
                bucket,
                kinds_str,
                clocks,
                ids,
            ));
            // p118 关④ TURN_CLASS 侧信道（只写不判；行契约见 turn_class.rs 模块头）：
            // 与 CERT 行同主键（ids）配对——CERT 行格式零改，本行为独立新增行。
            let turn_class = classify_certificate_turn(&certificate, classification);
            let confirmed_vec = certificate
                .confirmed()
                .iter()
                .map(|flag| if *flag { '1' } else { '0' })
                .collect::<String>();
            let (class_name, evidence_str) = turn_class_dump(&turn_class);
            dump_line(format_args!(
                "TURN_CLASS caliber={:?} exec={} top={} as_of={} ids={} class={} confirmed_vec={}{}",
                certificate.caliber(),
                exec,
                top,
                as_of,
                ids,
                class_name,
                confirmed_vec,
                evidence_str,
            ));
        }
    }
}

/// p118 关④：`NestTurnClass` → TURN_CLASS 行的 (class 名, evidence 片段) 投影（p92 逐字）。
fn turn_class_dump(class: &NestTurnClass) -> (&'static str, String) {
    match class {
        NestTurnClass::NestedConfirmed => ("NestedConfirmed", String::new()),
        NestTurnClass::ExecEvidenceOnly => ("ExecEvidenceOnly", String::new()),
        NestTurnClass::XiaozhuandaCandidate { evidence } => (
            "XiaozhuandaCandidate",
            format!(
                " evidence=c_prime=({},{},{},{});third={};second={}",
                evidence.c_prime.zg,
                evidence.c_prime.zd,
                evidence.c_prime.start_index,
                evidence.c_prime.end_index,
                evidence.third_src,
                evidence
                    .second_class
                    .map_or_else(|| "-".to_string(), |src| src.to_string()),
            ),
        ),
        // DeferOrphan 由 observe_snapshot 尾部单独发射（事件层条目，非证书行）。
        NestTurnClass::DeferOrphan { .. } => ("DeferOrphan", String::new()),
    }
}

/// p116 探针：DIV 行 kind 标签（趋势背驰=trend，盘整背驰=pan）。
fn nest_kind_tag(kind: NestDivergenceKind) -> &'static str {
    match kind {
        NestDivergenceKind::Trend => "trend",
        NestDivergenceKind::Consolidation => "pan",
    }
}

fn certificate_kind(certificate: &TypedNestCertificate) -> &'static str {
    let trend = certificate
        .kinds()
        .iter()
        .any(|kind| *kind == NestDivergenceKind::Trend);
    let pan = certificate
        .kinds()
        .iter()
        .any(|kind| *kind == NestDivergenceKind::Consolidation);
    match (trend, pan) {
        (true, false) => "trend",
        (false, true) => "pan",
        _ => "mixed",
    }
}

fn certificate_key(exec: usize, top: usize, certificate: &TypedNestCertificate) -> String {
    let cert = certificate.certificate();
    let rungs = cert
        .rungs()
        .iter()
        .map(|rung| (rung.interval(), rung.child_interval()))
        .collect::<Vec<_>>();
    // #97: 身份标签参与去重键，同结构不同 provider 身份的链不互相吞并。
    let ids = certificate
        .identities()
        .iter()
        .map(|id| (id.level, id.turn_source, id.interval_b))
        .collect::<Vec<_>>();
    format!(
        "{exec}:{top}:{:?}:{:?}:{:?}:{:?}:{:?}:{ids:?}",
        certificate.caliber(),
        cert.side(),
        cert.base_interval(),
        rungs,
        certificate.kinds()
    )
}

/// ★p117 T1 终端背书生产口径常数（bsp-terminal-endorsement-ruling-20260718 裁决2）：
/// C-b = `[c_start, t*]` 窗口最早 confirm_side 点。与 p92 同一常数同一切换点。

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

const TERMINAL_MATCH: TerminalMatch = TerminalMatch::CWindow;

/// 终端背书查法（生产）：委托 lib 单一来源 `nest::terminal_bits_at_event`——账本级别移位
/// `levels[ℓ-1].bsp` + 口径常数 [`TERMINAL_MATCH`]。与 p92 同名函数逐字同口径。
fn terminal_bits_new(
    classification: &classifier::Classification,
    event: &NestCandidateEvent,
) -> Option<BspBits> {
    // 关③ P3：lib 返回形状扩为 `TerminalEndorsement`（bits + owner start_index）——
    // 生产装配消费 bits 层；owner 两维构成归收紧后重放审计读数，非本 bin 职责。
    terminal_bits_at_event(classification, event, TERMINAL_MATCH, &bin_anchor_ctx()).map(|t| t.bits)
}

/// 旧事件路径（`CandDeltaEvent`，P1 基线对账/P124M_BASELINE 类审计）的终端查法：同一级别
/// 移位 + C-b 口径平移。委托 lib 单一来源 `nest::terminal_bits_in_book`，同口径平移并标注。
fn terminal_bits_old(
    classification: &classifier::Classification,
    level: usize,
    c_start: usize,
    source: usize,
    side: Side,
    b_center_start: Option<usize>,
) -> Option<BspBits> {
    let book_level = classification
        .levels
        .get(event_bsp_book_level(level as u32)?)?;
    let book = &book_level.bsp;
    // 关③ P3 平移：旧事件 = Cand^δ 趋势族线（pan_div_diag 为 cand_delta=false 纯诊断，
    // 结构性不入终端查询）⟹ kind=Trend；B 身份 = 事件自带 `b_parent.source_interval.0`
    //（ParentCenterIdentity 已携 B start_index 快照，单一来源，无第二查法）。
    // #218 面 B：一/三类判同的 B 带由同层 `centers` 查出（b_center_start 只当查找键）。
    terminal_bits_in_book(
        book,
        &book_level.centers,
        c_start,
        source,
        side,
        NestDivergenceKind::Trend,
        b_center_start,
        TERMINAL_MATCH,
        &bin_anchor_ctx(),
    )
    .map(|t| t.bits)
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
