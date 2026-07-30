//! task #124（方法 B）：进程级 run 分片并行重放 —— 分片方。
//!
//! 管线 = p92_nest_replay_postruling 逐字（现行口径：p117 T1 终端背书 C-b + p118 TURN_CLASS
//! 侧信道 + p116 TURN/DIV/TERM 探针行），唯一改动 = 终态 targets 装配后按 **run 键**
//! `(level, provider_window.0)` 哈希过滤到分片 i，prefix pass 只追本片 targets。
//! 语义逐字复刻现行：判据/账本/trigger 门/装配函数零改动（禁趁机改判据）。
//!
//! ## 分片键与哈希（构造性 bit-exact 的依据）
//!
//! - **分片键 = run 键 `(level, provider_window.0)`**（provider_window.0 = 终态快照中该 run
//!   的 coordinate_window.start）。views 计数器以 run 为共享粒度（每 trigger 每 run 一次
//!   「投影→decompose→view→事件」装配计 1 view）——同 run 多目标必须同片，Σviews 才=顺序值。
//! - **哈希（写死，跨平台/跨进程确定性；与 p124_merge 逐字一致）**：FNV-1a 64 混合
//!   (level, run_source)，末态 xor-fold（`h ^= h >> 32`）后取模 S。标签 `fnv1a64.xorfold.mod`。
//! - **构造保证**：装配函数全为只读纯函数；增量塔/MACD 缓存演化只依赖 bars 不依赖 pending；
//!   trigger 门 `(forest_epoch, signal_signature)` 同理；pending 移除 per-key 自闭；账本
//!   or_insert first-wins 按 key 隔离。⟹ 片内 pending 演化 = 顺序在该片键集上的限制：
//!   trigger 处理集 = {变化且片 pending 非空}（片 pending 空 ⟺ 片键全解 ⟺ 顺序中这些 run
//!   不再被扫），每 run 扫描时集合逐格一致 ⟹ 每键 first-seen 钟/views 逐格一致。
//! - terminal pass 每片全量照常（~730s 串行不可分）；TURN/CKPT 行不依赖 pending，
//!   跨片完全相同（归并去重并断言）。
//!
//! ## 分配口径（P124_BALANCE 切换；缺省=hash，零行为变化）
//!
//! 分配函数两口径，粒度均为 run 键 `(level, provider_window.0)`——「同 run 多目标同片」
//! 不变，run 内语义与分配无关 ⟹ 上文 bit-exact 构造论证对两口径同样成立（level_rr 只是
//! 换一个 run→片 映射，仍是 run 键的划分）：
//!
//! - **hash（缺省）**：上文 FNV-1a 64 xor-fold mod S，逐位同现行。`P124_BALANCE` 未设或
//!   =`hash` 走此口径——stdout/dump 逐字同旧版（含 META `hash=fnv1a64.xorfold.mod` 标签）。
//! - **level_rr（opt-in，`P124_BALANCE=level_rr`）**：level==1 的 run 按 run_source 升序
//!   round-robin 到 0..S-1（rank i → 片 i mod S；rank 表 = 终态 targets 全集中 L1 run 键
//!   去重升序——无 target 的 run 不进 prefix 评估、与负载无关，不入表）；level≥2 维持 FNV
//!   哈希 mod S 逐位不变。动机：L1 占 view 税 ~82%（p122 §6 实测权重 + 全量分片读数），
//!   哈希可能把多个 L1 run 聚到同片；RR 保证各片 L1 run 计数差 ≤1。**注意**：现行全量
//!   实测总体退化（每级恰 1 个 target-carrying run，run_buckets Σ=5），L1 独块不可再分，
//!   level_rr 只把它从 s2 挪到 s0——最大片份额不变；收益在多 L1 run 总体才显现。
//! - **对归并的透明性**：p124_merge 零改动可消费 level_rr 片 dump——归并的结构性不变量
//!   （owners 跨片互斥 dup_owner=0、partition_missing/extra=0、candidates/divergences
//!   min-merge、bar-loop 行序全序、CKPT/TURN 跨片相等、Σprefix_views）对**任意** run 划分
//!   成立，与分配口径无关。唯一与口径耦合的读数 = `hash_drift`（归并侧硬编码 FNV 复算片
//!   归属）：level_rr 下 FNV≠RR 的 L1 行按设计计入（读数 = 被 RR 挪片的 L1
//!   CAND/PENDING/DIV 行总数），非正确性失败；level_rr 全量验收按本口径解读该读数。
//!   META `hash=` 标签在 level_rr 下改写为 `level_rr.l1_asc+fnv1a64.xorfold.mod`（跨片
//!   一致 ⟹ meta_drift=0；缺省口径标签逐字不变）。
//!
//! ## 本片不产的行（归并方重放 terminal pass 一次后单线程重产，天然有序）
//!
//! FALLBACK / CERT / TURN_CLASS / probe 兜底 DIV·TERM：judge_at 需跨片合并 book
//! （首证钟主键 = CERT judge_at 向量），本片 book 只覆盖片键，产出行会带错 judge_at，
//! 故一律 defer（见 P124_DEFER 门行）。covered_b/missed 不依赖 judge_at（nest.rs:559
//! 「judge_at 只登记 D3 违反率，绝不作硬门」），本片在全终态事件集上装 B 链求得，
//! 跨片逐格一致，Σmissed=顺序值。
//!
//! ## 归并规则（p124_merge 实装侧；此处备查）
//!
//! candidates/divergences 按 key 取 min（分片键互斥 ⟹ 每键恰属一片，min=first-wins）；
//! bar-loop 行按 (at, section, level, run, seq) 全序还原顺序插入序（section：TURN=0<
//! trigger(DIV/TERM)=1<CKPT*=2；同 (at,level,run) 必同片，seq 保片内发射序 ⟹ 精确）；
//! CKPT/TURN 行跨片完全相同→取 shard 0 并逐片断言相等；unresolved_targets/prefix views
//! 求和；ProviderAudit terminal-pass 派生计数（too_short/invalid_seed/missing_carried_center/
//! other/terminal_views）跨片相同→取单片并断言；终态 CERT 装配+judge_at 回填由归并方
//! 用合并 book 重放 terminal pass（~730s）后单线程执行——CERT/TURN_CLASS 行由此重产。
//!
//! ## 全量双跑协议（S=8，等编排者指令，不在本任务执行）
//!
//! 全量 S=8 归并 vs v3 全量 dump（p92 顺序）：门行全套一致（**views 求和须=148,043**
//! 的现行值 = terminal_views + Σprefix_views）+ **dump 全行含行序 diff=0**
//! （probes 须 OFF = p92 行集）；首证钟主键 = CERT judge_at 向量逐格一致。
//!
//! ## 用法
//!
//! ```text
//! env P124_SHARD=<i>/<S> P124_DUMP_PREFIX=<path 前缀> P124_MAX_BARS=<n> \
//!     P124_CKPT=<K> P124_PROBES=<0|1> \
//!     p124_shard_replay <btc_1m_full.json>   # dump 落 <前缀><i>，stdout 门行前缀 P124_
//! ```
//!
//! - `P124_SHARD`（必需）："i/S"，0≤i<S。S=1 退化为顺序等价（同码无过滤）。
//! - `P124_BALANCE`：分配口径，`hash`（缺省，现行 FNV 逐位）| `level_rr`（opt-in：
//!   L1 run 按 run_source 升序 round-robin，L≥2 维持 FNV）。详见模块头「分配口径」节。
//! - `P124_DUMP_PREFIX`：dump 文件前缀，实落 `{前缀}{i}`；未设则零 dump（归并不可用）。
//! - `P124_PROBES=1`：发射 p116 探针行（TURN/DIV/TERM，行尾带归并元数据）。默认 0
//!   = p92 行集（全量 S=8 diff 协议用 0；500k 自检用 1）。
//! - dump 行尾元数据 ` ||at=<bar>|shard=<i>|seq=<n>[|run=<l>:<src>|key=<enc>]` 仅供
//!   归并消费，归并输出一律剥除（恢复 p92/p116 逐字行）。
//! - `LEDGER_*` 行（CAND/DIV/TERM/PENDING/META）为片账本导出，归并消费、不进合并 dump。

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
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::{ParseLayer, ParseLayerIncr};
use newchan_rust::theta_v0::types::{quantize, Bar, BspBits, MoveKind, Side, Timestamp};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

/// 分片环境：(shard_index, shards, probes)。main 起始处解析 P124_SHARD/P124_PROBES 后写入。
static SHARD_ENV: OnceLock<(usize, usize, bool)> = OnceLock::new();

fn shard_env() -> (usize, usize, bool) {
    SHARD_ENV.get().copied().unwrap_or((0, 1, false))
}

/// bar-loop dump 行全局发射序号（片内单调；归并 (at,section,run,seq) 全序的 tiebreak）。
static DUMP_SEQ: AtomicUsize = AtomicUsize::new(0);

/// 侧信道：`P124_DUMP_PREFIX=<前缀>` 时落 `{前缀}{i}` 并逐条写明细行。
/// 只写不判——不参与任何账本、真值或裁定；未设 env 时零行为差异。
static DUMP: OnceLock<Option<Mutex<BufWriter<File>>>> = OnceLock::new();

fn dump_line(args: std::fmt::Arguments<'_>) {
    let sink = DUMP.get_or_init(|| {
        std::env::var("P124_DUMP_PREFIX")
            .ok()
            .map(|prefix| format!("{}{}", prefix, shard_env().0))
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

/// bar-loop 行：p92/p116 逐字行 + 归并元数据尾（归并输出剥除）。
/// run/key 仅 trigger 行（DIV/TERM）携带：run = 终态 target 的 (level, provider_window.0)
/// = 分片键（collect_target_candidates 的 BTreeSet 迭代键，顺序发射序的同构全序分量）。
fn dump_row(at: usize, run: Option<(u32, usize)>, key: Option<&EventKey>, text: String) {
    let seq = DUMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let (shard, _, _) = shard_env();
    let mut line = format!("{text} ||at={at}|shard={shard}|seq={seq}");
    if let Some((level, src)) = run {
        line.push_str(&format!("|run={level}:{src}"));
    }
    if let Some(key) = key {
        line.push_str(&format!("|key={}", enc_key(key)));
    }
    dump_line(format_args!("{line}"));
}

fn dump_flush() {
    if let Some(Some(sink)) = DUMP.get() {
        if let Ok(mut writer) = sink.lock() {
            let _ = writer.flush();
        }
    }
}

/// 分片哈希（写死；与 p124_merge 逐字一致，改动=归并 break）：
/// FNV-1a 64（offset basis / prime 同 FNV 规格）混合 (level, run_source)，
/// 末态 xor-fold 后取模 S。输入 = run 键 (level, provider_window.0)——同 run 多目标同片。
fn run_shard(level: u32, run_source: usize, shards: usize) -> usize {
    let mut h = 0xcbf2_9ce4_8422_2325u64; // FNV-1a 64 offset basis
    h = (h ^ u64::from(level)).wrapping_mul(0x0000_0100_0000_01b3); // FNV prime
    h = (h ^ run_source as u64).wrapping_mul(0x0000_0100_0000_01b3);
    h ^= h >> 32;
    (h % shards as u64) as usize
}

/// 分配口径（模块头「分配口径」节；`P124_BALANCE` 切换，缺省 Hash = 现行 FNV 逐位）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShardBalance {
    /// 缺省：[`run_shard`] 的 FNV-1a xor-fold mod S（stdout/dump 逐字同旧版，零行为变化）。
    Hash,
    /// opt-in（`P124_BALANCE=level_rr`）：L1 run 按 run_source 升序 round-robin；
    /// L≥2 维持 FNV 哈希逐位不变。
    LevelRr,
}

/// `P124_BALANCE` 解析：缺省/`hash` → Hash；`level_rr` → LevelRr；
/// 其他值报错（090 禁模糊地带——拼错的口径不得静默回落成另一个口径）。
fn balance_from_spec(spec: Option<&str>) -> Result<ShardBalance, String> {
    match spec {
        None | Some("hash") => Ok(ShardBalance::Hash),
        Some("level_rr") => Ok(ShardBalance::LevelRr),
        Some(other) => Err(format!("P124_BALANCE={other} 未知（hash|level_rr）")),
    }
}

/// 分配口径标签（stdout P124_SHARD 行与 LEDGER_META `hash=` 字段共用同一字符串）：
/// 缺省逐字 = 现行标签（dump 逐位不变）；level_rr 如实改标（跨片一致 ⟹ meta_drift=0）。
fn balance_label(balance: ShardBalance) -> &'static str {
    match balance {
        ShardBalance::Hash => "fnv1a64.xorfold.mod",
        ShardBalance::LevelRr => "level_rr.l1_asc+fnv1a64.xorfold.mod",
    }
}

/// level_rr 的 L1 名次表：run 键集（(level, run_source)，BTreeSet 升序迭代）中 level==1
/// 的 run_source 去重升序 → 名次（0 起）。键集 = 终态 targets 的 run 键——无 target 的
/// run 不进 prefix 评估、与负载无关，不入表。BTreeSet/BTreeMap 迭代序确定 ⟹ 同输入同输出。
fn l1_rr_rank(runs: &BTreeSet<(u32, usize)>) -> BTreeMap<usize, usize> {
    runs.iter()
        .filter(|(level, _)| *level == 1)
        .map(|(_, source)| *source)
        .enumerate()
        .map(|(rank, source)| (source, rank))
        .collect()
}

/// level_rr 分配：level==1 → 名次 mod S（run_source 升序 round-robin 到 0..S-1）；
/// level≥2 → FNV 哈希（与缺省口径逐位一致）。run_source 不在名次表（生产构造上不可能：
/// 名次表与过滤同源；仅为保持全函数确定性）时回落 FNV 哈希。
fn run_shard_level_rr(
    level: u32,
    run_source: usize,
    shards: usize,
    l1_rank: &BTreeMap<usize, usize>,
) -> usize {
    if level == 1 {
        if let Some(rank) = l1_rank.get(&run_source) {
            return rank % shards;
        }
    }
    run_shard(level, run_source, shards)
}

/// LEDGER key 编码（无空格；与 p124_merge 的 parse_enc_key 逐字互逆）。
/// `L<level>,s<0|1>,k<t|p>,sa<a0>-<a1>,ib<b0>-<b1>,ia<a0>-<a1>,ts<turn_source>`
fn enc_key(key: &EventKey) -> String {
    format!(
        "L{},s{},k{},sa{}-{},ib{}-{},ia{}-{},ts{}",
        key.level,
        u8::from(key.short),
        if key.kind == NestDivergenceKind::Trend {
            "t"
        } else {
            "p"
        },
        key.seg_a.0,
        key.seg_a.1,
        key.interval_b.0,
        key.interval_b.1,
        key.interval_a.0,
        key.interval_a.1,
        key.turn_source,
    )
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
    /// #97: B 口径链实际吸收过的事件身份（任意 rung 位置）。
    covered_b: BTreeSet<IdentityKey>,
    /// #97: 走盘整块回退进料口的候选。
    intake_fallbacks: BTreeSet<EventKey>,
    /// p116 探针（P124_PROBES=1）：TERM 行去重账本（键→首次终端确认 bar；
    /// p116 为 BTreeSet，此处带钟值供 LEDGER_TERM 导出；去重语义逐字一致）。
    term_seen: BTreeMap<EventKey, usize>,
}

/// p116 探针（P124_PROBES=1）：TURN 游标（每级已落盘稳定完成块数 + 身份去重集）。
#[derive(Debug, Default)]
struct TurnBook {
    done: BTreeMap<usize, usize>,
    seen: BTreeMap<usize, BTreeSet<(usize, usize, &'static str)>>,
}

impl TurnBook {
    /// 每 bar 调用：把每级「稳定完成」（≥2 后继块，见 p116 模块头稳定规则）的 typed 结构
    /// 终点落盘为 TURN 行。语义逐字同 p116；仅输出口换成带归并元数据的 dump_row。
    fn observe(&mut self, classification: &classifier::Classification, at: usize) {
        for (level, state) in classification.levels.iter().enumerate() {
            let m = state.moves.len();
            // 链尾两块（Active 尾 + 最近 Completed 块）仍可能触 frontier 重折，不落盘。
            let ready = m.saturating_sub(2);
            let done = self.done.entry(level).or_insert(0);
            if ready < *done {
                // 段账本回缩 / 全量重折护栏：游标归零重扫（seen 挡掉同身份重复行）。
                *done = 0;
            }
            let from = *done;
            *done = ready;
            if from >= ready {
                continue;
            }
            let seen = self.seen.entry(level).or_default();
            for block in &state.moves[from..ready] {
                let key = (
                    block.start_center,
                    block.end_center,
                    move_kind_tag(block.kind),
                );
                if !seen.insert(key) {
                    continue;
                }
                let start = state
                    .centers
                    .get(block.start_center)
                    .expect("MoveBlock.start_center 越界（decompose 不变量破裂）");
                let end = state
                    .centers
                    .get(block.end_center)
                    .expect("MoveBlock.end_center 越界（decompose 不变量破裂）");
                dump_row(
                    at,
                    None,
                    None,
                    format!(
                        "TURN level={} end={} start={} kind={}",
                        level,
                        end.end_index,
                        start.start_index,
                        move_kind_tag(block.kind),
                    ),
                );
            }
        }
    }
}

fn move_kind_tag(kind: MoveKind) -> &'static str {
    match kind {
        MoveKind::Trend => "Trend",
        MoveKind::Consolidation => "Consolidation",
    }
}

/// p116 探针：DIV 行 kind 标签（趋势背驰=trend，盘整背驰=pan）。
fn nest_kind_tag(kind: NestDivergenceKind) -> &'static str {
    match kind {
        NestDivergenceKind::Trend => "trend",
        NestDivergenceKind::Consolidation => "pan",
    }
}

struct TerminalState {
    l0: ParseLayer,
    classification: classifier::Classification,
    tower: Vec<Rc<Vec<LeveledMove>>>,
    cache: classifier::TowerCache,
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("用法: p124_shard_replay <btc_1m_full.json>")?;
    if args.next().is_some() {
        return Err("参数过多".to_string());
    }
    // P124_SHARD="i/S"（必需）：分片键=(level, provider_window.0)，哈希见 run_shard。
    let shard_spec = std::env::var("P124_SHARD")
        .map_err(|_| "缺 P124_SHARD=<i>/<S>（S=1 退化为顺序等价）".to_string())?;
    let (shard_i, shards) = parse_shard_spec(&shard_spec)?;
    let probes = std::env::var("P124_PROBES").ok().as_deref() == Some("1");
    // P124_BALANCE（opt-in）：缺省/hash = 现行 FNV 逐位；level_rr = L1 RR + L≥2 FNV。
    let balance = balance_from_spec(std::env::var("P124_BALANCE").ok().as_deref())?;
    SHARD_ENV
        .set((shard_i, shards, probes))
        .map_err(|_| "SHARD_ENV 重复初始化".to_string())?;

    let config = ThetaConfig::default();
    let loaded = load_bars(Path::new(&path), config.tick.tick_size)?;
    if loaded.bars.is_empty() {
        return Err("输入 bars 为空".to_string());
    }
    let max_bars = std::env::var("P124_MAX_BARS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .map_or(loaded.bars.len(), |value| value.min(loaded.bars.len()));
    let ckpt_every: usize = std::env::var("P124_CKPT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);

    println!(
        "P124_INPUT bars={} replay_bars={} first_date={} last_date={}",
        loaded.bars.len(),
        max_bars,
        loaded.first_date,
        loaded.last_date
    );
    println!(
        "P124_RULE cand=dir_and_comparable_and_extreme weak=divergence_stage interval_production=B typed=trend_or_consolidation d3=sidecar terminal=BspBits_confirm_side clock=first_prefix created_at=forbidden"
    );

    let mut audit = ProviderAudit::default();
    let terminal = run_terminal_pass(&loaded.bars[..max_bars], &config)?;
    let terminal_views = audit.views;
    let (hist, close_src) = terminal.cache.causal_series();
    let dif = terminal.cache.macd_dif();
    let terminal_events = collect_snapshot_candidates(
        &terminal.tower,
        max_bars - 1,
        hist,
        dif,
        close_src,
        &mut audit,
    )?;
    let terminal_views = audit.views - terminal_views;
    let mut all_targets = BTreeMap::new();
    for event in terminal_events.iter().flatten() {
        all_targets.insert(EventKey::from(event), *event);
    }
    // ★分片过滤：按 run 键 (level, provider_window.0) 分配到分片 i——同 run 多目标同片。
    // 分配口径 = balance（缺省 hash = run_shard 逐位；level_rr 时 L1 改走名次表 RR，
    // L≥2 逐位不变——见模块头「分配口径」节）。
    let l1_rank = match balance {
        ShardBalance::LevelRr => {
            let runs: BTreeSet<(u32, usize)> = all_targets
                .values()
                .map(|event| (event.level, event.provider_window.0))
                .collect();
            l1_rr_rank(&runs)
        }
        ShardBalance::Hash => BTreeMap::new(),
    };
    let mut targets: BTreeMap<EventKey, NestCandidateEvent> = BTreeMap::new();
    let mut run_buckets = BTreeSet::new();
    for (key, event) in &all_targets {
        let assigned = match balance {
            ShardBalance::Hash => run_shard(event.level, event.provider_window.0, shards),
            ShardBalance::LevelRr => {
                run_shard_level_rr(event.level, event.provider_window.0, shards, &l1_rank)
            }
        };
        if assigned == shard_i {
            run_buckets.insert((event.level, event.provider_window.0));
            targets.insert(key.clone(), *event);
        }
    }
    println!(
        "P124_SHARD shard={}/{} hash={} key=(level,provider_window.0) probes={} targets={}/{} run_buckets={} ckpt={}",
        shard_i,
        shards,
        balance_label(balance),
        u8::from(probes),
        targets.len(),
        all_targets.len(),
        run_buckets.len(),
        ckpt_every,
    );
    let (mut book, prefix_views, prefix_pending) = run_targeted_prefix_pass(
        &loaded.bars[..max_bars],
        &config,
        &targets,
        ckpt_every,
        probes,
    )?;
    let unresolved_targets = prefix_pending.len();
    audit.views += prefix_views;
    audit.snapshots += book.candidates.len();

    // ── LEDGER 账本导出（prefix 末态；归并消费，不进合并 dump）──
    // candidates/divergences 为 prefix 末账本（终态兜底由归并方重产）；PENDING = 未解 targets。
    for (key, at) in &book.candidates {
        let target = targets.get(key).expect("candidate 必为 target");
        dump_line(format_args!(
            "LEDGER_CAND shard={} at={} run={}:{} key={}",
            shard_i,
            at,
            target.level,
            target.provider_window.0,
            enc_key(key),
        ));
    }
    for (key, at) in &book.divergences {
        let target = targets.get(key).expect("divergence 必为 target");
        dump_line(format_args!(
            "LEDGER_DIV shard={} at={} run={}:{} key={}",
            shard_i,
            at,
            target.level,
            target.provider_window.0,
            enc_key(key),
        ));
    }
    for (key, at) in &book.term_seen {
        let target = targets.get(key).expect("term 必为 target");
        dump_line(format_args!(
            "LEDGER_TERM shard={} at={} run={}:{} key={}",
            shard_i,
            at,
            target.level,
            target.provider_window.0,
            enc_key(key),
        ));
    }
    for key in &prefix_pending {
        let target = targets.get(key).expect("pending 必为 target");
        dump_line(format_args!(
            "LEDGER_PENDING shard={} run={}:{} key={}",
            shard_i,
            target.level,
            target.provider_window.0,
            enc_key(key),
        ));
    }

    let l0 = terminal.l0;
    let classification = terminal.classification;
    let tower = terminal.tower;
    let cache = terminal.cache;

    // ── 终态轻量装配（分片可信口径；不产 FALLBACK/CERT/TURN_CLASS/兜底 probe 行——
    //    judge_at 跨片不完备，归并方重放 terminal pass 后用合并 book 单线程重产）──
    // candidates/divergences 终态兜底入账（仅片键；逐键 or_insert 独立 ⟹ Σ=顺序值）。
    // terminal_confirmed（逐键独立 ⟹ Σ=顺序值）；covered_b/intake_fallbacks 在全终态
    // 事件集上求得（跨片链完整、不依赖 judge_at ⟹ 与顺序逐格一致）。
    let as_of = max_bars - 1;
    let mut final_events = terminal_events;
    for events in &mut final_events {
        for event in events {
            let key = EventKey::from(&*event);
            if !targets.contains_key(&key) {
                continue;
            }
            book.candidates.entry(key.clone()).or_insert(as_of);
            if event.divergence_confirmed {
                book.divergences.entry(key.clone()).or_insert(as_of);
                if terminal_bits_new(&classification, event).is_some() {
                    book.terminal_confirmed.insert(key.clone());
                }
            }
        }
    }
    for exec in 1..final_events.len() {
        for top in exec..final_events.len() {
            let certificates = assemble_typed_certificates(
                &final_events,
                exec,
                top,
                NestIntervalCaliber::B,
                |event| terminal_bits_new(&classification, event),
            );
            for certificate in &certificates {
                for identity in certificate.identities() {
                    book.covered_b.insert((
                        identity.level,
                        identity.turn_source,
                        identity.interval_b,
                    ));
                }
            }
        }
    }
    for event in final_events.iter().flatten() {
        if event.intake_fallback {
            book.intake_fallbacks.insert(EventKey::from(event));
        }
    }

    // ── LEDGER_META（归并自洽断言的全部输入）──
    let missed: Vec<&EventKey> = book
        .terminal_confirmed
        .iter()
        .filter(|key| {
            !book
                .covered_b
                .contains(&(key.level, key.turn_source, key.interval_b))
        })
        .collect();
    dump_line(format_args!(
        "LEDGER_META shard={} shards={} v=1 hash={} max_bars={} ckpt={} probes={} targets={} run_buckets={} prefix_views={} terminal_views={} too_short={} invalid_seed={} missing_carried_center={} other={} unresolved={} candidates_final={} divergences_final={} terminal_confirmed={} missed={}",
        shard_i,
        shards,
        balance_label(balance),
        max_bars,
        ckpt_every,
        u8::from(probes),
        targets.len(),
        run_buckets.len(),
        prefix_views,
        terminal_views,
        audit.projection_too_short,
        audit.projection_invalid_seed,
        audit.projection_missing_carried_center,
        audit.projection_other,
        unresolved_targets,
        book.candidates.len(),
        book.divergences.len(),
        book.terminal_confirmed.len(),
        missed.len(),
    ));

    // ── BIT_EXACT（增量 vs 全量；各片相同，证明片内增量塔=全量塔）──
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
            println!("P124_BIT_DIFF_LEVEL L{level} {summary}");
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

    println!(
        "P124_YIELD candidates={} trend_candidates={} pan_candidates={} divergence_confirmed={} trend_divergence={} pan_divergence={} terminal_confirmed={}",
        book.candidates.len(),
        trend_candidates,
        pan_candidates,
        book.divergences.len(),
        trend_divergences,
        pan_divergences,
        book.terminal_confirmed.len()
    );
    println!(
        "P124_BASELINE old_candidates={} old_terminal_confirmed={} old_certificates={} new_candidates={} new_terminal_confirmed={} new_B_certificates=deferred_to_merger",
        old_candidates,
        old_terminal,
        old_certificates,
        book.candidates.len(),
        book.terminal_confirmed.len(),
    );
    println!(
        "P124_PROVIDER snapshots={} views={} prefix_views={} terminal_views={} too_short={} invalid_seed={} missing_carried_center={} other={} unresolved_targets={} complete={}",
        audit.snapshots,
        audit.views,
        prefix_views,
        terminal_views,
        audit.projection_too_short,
        audit.projection_invalid_seed,
        audit.projection_missing_carried_center,
        audit.projection_other,
        unresolved_targets,
        audit.provider_complete() && unresolved_targets == 0
    );
    println!(
        "P124_BIT_EXACT old_path_diff={} tower_diff={} moves_centers_bsp_pan_diff={} lifecycle_cp_ownership_diff={} classification_total_diff={}",
        old_semantic_diff,
        tower_diff,
        old_semantic_diff.saturating_sub(tower_diff),
        lifecycle_diff,
        classification_diff
    );
    println!(
        "P124_DEFER final_snapshot_rows=merger cert=merger d3=merger snapshot=merger r7=merger reason=cross_shard_judge_at"
    );
    println!(
        "P124_MISSED terminal_confirmed={} covered_b={} intake_fallback_events={} missed={}",
        book.terminal_confirmed.len(),
        book.covered_b.len(),
        book.intake_fallbacks.len(),
        missed.len()
    );
    for key in &missed {
        println!(
            "P124_MISSED_EVENT level={} kind={:?} short={} seg_a={:?} interval_b={:?} interval_a={:?} turn_source={} intake_fallback={}",
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

/// P124_SHARD="i/S" 解析（0≤i<S，S≥1）。
fn parse_shard_spec(spec: &str) -> Result<(usize, usize), String> {
    let (i, s) = spec
        .split_once('/')
        .ok_or_else(|| format!("P124_SHARD={spec} 形如 <i>/<S>"))?;
    let i: usize = i
        .parse()
        .map_err(|_| format!("P124_SHARD i 非整数: {spec}"))?;
    let s: usize = s
        .parse()
        .map_err(|_| format!("P124_SHARD S 非整数: {spec}"))?;
    if s == 0 || i >= s {
        return Err(format!("P124_SHARD 需 0<=i<S: {spec}"));
    }
    Ok((i, s))
}

fn signal_signature(
    classification: &classifier::Classification,
) -> Vec<(usize, usize, usize, usize)> {
    classification
        .levels
        .iter()
        .map(|level| {
            (
                level.bsp.len(),
                level
                    .bsp
                    .last()
                    .map_or(usize::MAX, |point| point.source_index),
                level.pan_div.len(),
                level
                    .pan_div
                    .last()
                    .map_or(usize::MAX, |cert| cert.source_index),
            )
        })
        .collect()
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
                "P124_TERMINAL_PROGRESS shard={} bar={index}/{} elapsed={:.1}s",
                shard_env().0,
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

/// prefix pass（p92 逐字 + p116 探针（P124_PROBES=1）+ 归并元数据）：
/// trigger 门/pending 语义/账本 or_insert/views 计数与 p92·p116 逐字一致——
/// 分片只缩 targets 全集，不改任何判据。返回 (book, prefix_views, prefix 末 pending 键集)。
fn run_targeted_prefix_pass(
    bars: &[Bar],
    config: &ThetaConfig,
    targets: &BTreeMap<EventKey, NestCandidateEvent>,
    ckpt_every: usize,
    probes: bool,
) -> Result<(YieldBook, usize, BTreeSet<EventKey>), String> {
    let mut parser = ParseLayerIncr::new(config);
    let mut cache = classifier::TowerCache::new();
    let mut book = YieldBook::default();
    let mut turns = TurnBook::default();
    let mut pending: BTreeSet<EventKey> = targets.keys().cloned().collect();
    let mut views = 0usize;
    let mut last_trigger = None;
    let started = Instant::now();
    for (index, bar) in bars.iter().copied().enumerate() {
        let l0 = parser.append(bar);
        let (classification, tower) =
            classifier::classify_with_tower_incremental(&l0, config, &mut cache);
        // p116 探针：TURN 逐 bar 观察（不经 trigger 门——pending 空后仍落盘；不依赖 targets，
        // 跨片完全相同）。probes=0 时整段跳过（p92 行集，零热环差异）。
        if probes {
            turns.observe(&classification, index);
        }
        let trigger = (cache.forest_epoch(), signal_signature(&classification));
        if last_trigger.as_ref() != Some(&trigger) && !pending.is_empty() {
            let (hist, close_src) = cache.causal_series();
            let dif = cache.macd_dif();
            let (events, used_views) =
                collect_target_candidates(&tower, index, hist, dif, close_src, targets, &pending)?;
            views += used_views;
            for event in events {
                let key = EventKey::from(&event);
                if !pending.contains(&key) {
                    continue;
                }
                book.candidates.entry(key.clone()).or_insert(index);
                let target = targets.get(&key).expect("pending target");
                if event.divergence_confirmed {
                    // p116 探针：DIV = divergences 账本首次插入瞬间（or_insert 语义逐字同 p116）。
                    if probes && !book.divergences.contains_key(&key) {
                        dump_row(
                            index,
                            Some((event.level, target.provider_window.0)),
                            Some(&key),
                            format!(
                                "DIV level={} end={} kind={} side={:?}",
                                event.level,
                                event.turn_source,
                                nest_kind_tag(event.kind),
                                event.side,
                            ),
                        );
                    }
                    book.divergences.entry(key.clone()).or_insert(index);
                }
                // p116 探针：TERM = 事件键首次终端 bits 确认（term_seen 全程去重，不动账本）。
                if probes
                    && !book.term_seen.contains_key(&key)
                    && terminal_bits_new(&classification, &event).is_some()
                {
                    book.term_seen.insert(key.clone(), index);
                    dump_row(
                        index,
                        Some((event.level, target.provider_window.0)),
                        Some(&key),
                        format!(
                            "TERM level={} bar={} side={:?}",
                            event.level, event.turn_source, event.side,
                        ),
                    );
                }
                if event.divergence_confirmed == target.divergence_confirmed {
                    pending.remove(&key);
                }
            }
            last_trigger = Some(trigger);
        }
        // #103 侧信道（逐字同 p92）：每 ckpt_every bars 全量快照装配并 dump（只写不判；
        // 不依赖 targets/pending，跨片完全相同，归并取 shard 0 并逐片断言）。
        if ckpt_every > 0 && index > 0 && index % ckpt_every == 0 {
            let (hist, close_src) = cache.causal_series();
            let dif = cache.macd_dif();
            let mut ckpt_audit = ProviderAudit::default();
            match collect_snapshot_candidates(&tower, index, hist, dif, close_src, &mut ckpt_audit)
            {
                Ok(by_level) => checkpoint_certificates(&by_level, &classification, index),
                Err(error) => {
                    dump_row(
                        index,
                        None,
                        None,
                        format!("CKPT_ERR as_of={index} err={error}"),
                    );
                }
            }
        }
        if index > 0 && index % 500_000 == 0 {
            eprintln!(
                "P124_PREFIX_PROGRESS shard={} bar={index}/{} elapsed={:.1}s pending={}/{} views={views}",
                shard_env().0,
                bars.len(),
                started.elapsed().as_secs_f64(),
                pending.len(),
                targets.len()
            );
        }
    }
    Ok((book, views, pending))
}

fn collect_target_candidates(
    tower: &[Rc<Vec<LeveledMove>>],
    as_of: usize,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
    targets: &BTreeMap<EventKey, NestCandidateEvent>,
    pending: &BTreeSet<EventKey>,
) -> Result<(Vec<NestCandidateEvent>, usize), String> {
    let mut runs_by_level: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    for key in pending {
        let Some(event) = targets.get(key) else {
            continue;
        };
        // snapshot 无前视的结构下，turn_source 尚未到达时该对象不可能成为 Cand。
        // 提前投影这些终态目标只增加扫描量，不可能改变首次可证钟。
        if event.turn_source <= as_of {
            runs_by_level
                .entry(event.level as usize)
                .or_default()
                .insert(event.provider_window.0);
        }
    }
    let mut out = Vec::new();
    let mut views = 0usize;
    for (level, run_sources) in runs_by_level {
        if level == 0 || level >= tower.len() {
            continue;
        }
        let windows = &tower[level];
        // 同一触发快照内，每级 lower legs 与合法 run 分区都只构造一次。
        let lower = lower_legs_from(&tower[level - 1])
            .map_err(|error| format!("L{level} targeted lower legs 失败: {error:?}"))?;
        let mut run_ranges = BTreeMap::new();
        let mut run_start = None;
        let mut run_source = None;
        for index in 0..=windows.len() {
            let seed_start = (index < windows.len())
                .then(|| {
                    project_extended_windows_carried_only(std::slice::from_ref(&windows[index]))
                        .ok()
                })
                .flatten()
                .and_then(|projection| projection.seeds.first().map(|seed| seed.start_index));
            match (run_start, seed_start) {
                (None, Some(source)) => {
                    run_start = Some(index);
                    run_source = Some(source);
                }
                (Some(start), None) => {
                    run_ranges.insert(run_source.expect("合法 run 有 source"), (start, index));
                    run_start = None;
                    run_source = None;
                }
                _ => {}
            }
        }
        for run_source_start in run_sources {
            let Some(&(start, end)) = run_ranges.get(&run_source_start) else {
                continue;
            };
            let projection = project_extended_windows_carried_only(&windows[start..end])
                .map_err(|error| format!("L{level} targeted projection 失败: {error:?}"))?;
            let centers: Vec<_> = projection.seeds.iter().map(|seed| seed.center).collect();
            let blocks = decompose::decompose(&centers);
            let query = LevelViewQuery {
                level: level as u32,
                coordinate_window: CoordinateWindow {
                    start: projection.seeds.first().expect("nonempty").start_index,
                    end: projection.seeds.last().expect("nonempty").end_index,
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
            .map_err(|error| format!("L{level} targeted C2 assemble 失败: {error:?}"))?;
            views += 1;
            out.extend(
                provide_nest_candidate_events(
                    level as u32,
                    &projection,
                    &blocks,
                    &lower,
                    &view,
                    hist,
                    dif,
                    close_src,
                )
                .into_iter()
                .filter(|event| pending.contains(&EventKey::from(event))),
            );
        }
    }
    Ok((out, views))
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

/// #103 侧信道：检查点全量证书导出（只写不判，不触碰 book/seen 等主路径状态）。
/// 与 p92 逐字同语义；行尾带归并元数据（归并剥除后 = p92 CKPT 行逐字）。
fn checkpoint_certificates(
    events: &[Vec<NestCandidateEvent>],
    classification: &classifier::Classification,
    as_of: usize,
) {
    for caliber in [NestIntervalCaliber::A, NestIntervalCaliber::B] {
        let mut seen = BTreeSet::new();
        let mut total = 0usize;
        for exec in 1..events.len() {
            for top in exec..events.len() {
                let certificates =
                    assemble_typed_certificates(events, exec, top, caliber, |event| {
                        terminal_bits_new(classification, event)
                    });
                for certificate in certificates {
                    let key = certificate_key(exec, top, &certificate);
                    if !seen.insert(key) {
                        continue;
                    }
                    total += 1;
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
                    dump_row(
                        as_of,
                        None,
                        None,
                        format!(
                            "CKPT caliber={:?} as_of={} exec={} top={} side={:?} bucket={} ids={}",
                            certificate.caliber(),
                            as_of,
                            exec,
                            top,
                            certificate.certificate().side(),
                            certificate_kind(&certificate),
                            ids,
                        ),
                    );
                }
            }
        }
        dump_row(
            as_of,
            None,
            None,
            format!("CKPT_STATS caliber={caliber:?} as_of={as_of} certs={total}"),
        );
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
/// C-b = `[c_start, t*]` 窗口最早 confirm_side 点。与 p92/p116 同一常数同一切换点。

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

/// 旧事件路径（`CandDeltaEvent`，P1 基线对账/P124_BASELINE 类审计）的终端查法：同一级别
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

#[cfg(test)]
mod shard_balance_tests {
    use super::*;

    /// 缺省=哈希回归锁：env 缺省/`hash` → Hash；`level_rr` → LevelRr；未知值报错。
    /// FNV 片号向量 = 现行全量 S=8 生产实测（L1→s2、L2→s5、L3→s4、L4→s7、L5→s6）——
    /// 逐位锁定：改动 run_shard 即破此测试（=归并 hash_drift 全崩）。
    #[test]
    fn default_is_hash_and_fnv_locked() {
        assert_eq!(balance_from_spec(None).unwrap(), ShardBalance::Hash);
        assert_eq!(balance_from_spec(Some("hash")).unwrap(), ShardBalance::Hash);
        assert_eq!(
            balance_from_spec(Some("level_rr")).unwrap(),
            ShardBalance::LevelRr
        );
        assert!(balance_from_spec(Some("level-rr")).is_err());
        assert!(balance_from_spec(Some("")).is_err());
        assert_eq!(run_shard(1, 35, 8), 2);
        assert_eq!(run_shard(2, 35, 8), 5);
        assert_eq!(run_shard(3, 35, 8), 4);
        assert_eq!(run_shard(4, 35, 8), 7);
        assert_eq!(run_shard(5, 160_931, 8), 6);
    }

    /// 两口径确定性：同输入同输出（哈希重复求值逐位一致；level_rr 名次表构造可重现）。
    #[test]
    fn assignment_deterministic_both_calibers() {
        let runs: BTreeSet<(u32, usize)> =
            (0..40u32).map(|s| (1u32, (s * 7 + 3) as usize)).collect();
        let rank_a = l1_rr_rank(&runs);
        let rank_b = l1_rr_rank(&runs);
        assert_eq!(rank_a, rank_b);
        for level in 1..=5u32 {
            for source in [0usize, 3, 35, 159, 160_931] {
                assert_eq!(run_shard(level, source, 8), run_shard(level, source, 8));
                assert_eq!(
                    run_shard_level_rr(level, source, 8, &rank_a),
                    run_shard_level_rr(level, source, 8, &rank_b)
                );
            }
        }
    }

    /// level_rr 的 L1 均衡性：S=8 下各片 L1 run 计数差 ≤1（含非整除总体），且全覆盖。
    #[test]
    fn level_rr_l1_spread_within_one() {
        for population in [8usize, 16, 19, 40] {
            let runs: BTreeSet<(u32, usize)> = (0..population).map(|s| (1u32, s)).collect();
            let rank = l1_rr_rank(&runs);
            let mut counts = [0usize; 8];
            for &(_, source) in &runs {
                counts[run_shard_level_rr(1, source, 8, &rank)] += 1;
            }
            let min = *counts.iter().min().unwrap();
            let max = *counts.iter().max().unwrap();
            assert!(max - min <= 1, "population={population} counts={counts:?}");
            assert_eq!(counts.iter().sum::<usize>(), population);
        }
    }

    /// level_rr 对 L≥2 与缺省口径逐位一致；现行全量实测总体（每级恰 1 个
    /// target-carrying run，Σrun_buckets=5）下 L1 独块从 s2 挪到 s0，其余片不动。
    #[test]
    fn level_rr_non_l1_identical_to_hash() {
        let runs: BTreeSet<(u32, usize)> =
            [(1u32, 35usize), (2, 35), (3, 35), (4, 35), (5, 160_931)]
                .into_iter()
                .collect();
        let rank = l1_rr_rank(&runs);
        assert_eq!(rank.get(&35), Some(&0));
        for &(level, source) in &runs {
            let expected = if level == 1 {
                0 // L1 单 run：rank 0 → 片 0
            } else {
                run_shard(level, source, 8)
            };
            assert_eq!(run_shard_level_rr(level, source, 8, &rank), expected);
        }
        // 缺省口径不构造名次表；空表 + 非 L1 逐位 = FNV（回落路径同一函数）。
        let empty = BTreeMap::new();
        assert_eq!(run_shard_level_rr(2, 35, 8, &empty), run_shard(2, 35, 8));
    }
}
