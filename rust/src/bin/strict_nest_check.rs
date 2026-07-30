//! strict_nest_check：严格区间套 P1/P2 校验 bin。
//!
//! 规格冻结（只读，主仓）：
//! - `chanlun/review-results/strict-nesting-divergence-plan-20260708.md`（P1/P2 规格与验收门）
//! - `chanlun/escalate/strict-nesting-rulings-20260708.md`（三裁决：Cand^δ≔背驰段谓词、
//!   盘整背驰不入链、确认时点=完成时）
//!
//! ## P1 硬门（本 bin 主判）
//! 默认（全量档）：因果重放下，每 STRICT_NEST_P1_CHECKPOINT（默认 1000）bar 对当前快照
//! 做一次每级全比对——`cand_delta_tower`（P1 谓词层）cand_delta=true 事件
//! (confirm_src, side) 多重集 ≟ `Classification.levels[ℓ].bsp` buy1/sell1 背驰确认支
//! (source_index, side) 多重集**逐 bit 一致**——末 bar 终态必判。任何 checkpoint / 终态
//! 不一致 ⟹ FAIL + 差异样例，停线（判据不可调）。另有逐 bar 因果诊断（非门）：确认支
//! 累积集 vs 终态集——「确认撤回」= 上游结构修订时两侧锁步撤回（实测存在，250k 前缀
//! 8 处；同前缀逐 bar 档 0 mismatch 证明两侧每 bar 仍 bit-exact），由 checkpoint 比对覆盖。
//! `STRICT_NEST_P1_PERBAR=1`：每 bar 每级全比对档（谓词层逐 bar 重算 ⟹ O(n²)，实测分段
//! 耗时线性增长、全量外推 20h+；只配 STRICT_NEST_MAX_BARS 做前缀逐 bar 证据——已留
//! 250k DIAG_CANDCACHE 对拍 + 600k 前缀 0 mismatch 证据）。两档判据同源
//! （`cand_delta_tower_cached`，零分叉）。
//!
//! ## 基线 sanity（每次必带）
//! 发射跟踪五元组 = P7-RESULT 头部（raw seen 29088 / conf 键 27152 / type3 键 15165 /
//! entry==close 40001/40001 / 零长度 2955）。
//!
//! ## E1 三元组复算（P2 验收比对表输入）
//! t1 首见键合计 942（ℓ0=533/ℓ1=37/ℓ2=95/ℓ3=142/ℓ4=135）；1278 笔名单（k=2 已匹配）内
//! n_B=1、n_C=0（口径 = /tmp/codex-work-p7/P7-RESULT.md + e1_tri_anchor.rs，锚判据逐行复刻，
//! 只保留「是否匹配」布尔——bps 统计列非 P2 门，不复算）。
//!
//! 骨架逐行复制自 `/tmp/codex-work-p7/rust/src/bin/e1_tri_anchor.rs`（因果重放模式）。
//! 不改任何已有库判据；P1 层为纯增量代码（见 recursive_tower.rs P1 段头铁律）。

use newchan_rust::theta_v0::classifier;
use newchan_rust::theta_v0::classifier::nest::{
    assemble_certificates_terminal, is_sub, NestInterval,
};
use newchan_rust::theta_v0::classifier::recursive_tower::{
    cp_terminal_certificate, find_move_by_end_index, CandDeltaEvent, CpScanOwnership, ElementId,
    LeveledMove,
};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser;
use newchan_rust::theta_v0::types::{quantize, Bar, BspBits, Side, Timestamp};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Instant;

// ═══════════════════════ 因果重放骨架（e1_tri_anchor.rs 逐行复制） ═══════════════════════

struct IncrementalClassifier<'a> {
    bars: &'a [Bar],
    config: &'a ThetaConfig,
    parser_incr: parser::ParseLayerIncr<'a>,
    tower_cache: classifier::TowerCache,
}

impl<'a> IncrementalClassifier<'a> {
    fn new(bars: &'a [Bar], config: &'a ThetaConfig) -> Self {
        Self {
            bars,
            config,
            parser_incr: parser::ParseLayerIncr::new(config),
            tower_cache: classifier::TowerCache::new(),
        }
    }

    /// 与 e1 的 classify_at 同构，但把 l0_i 一并返回（P1 谓词层需同一因果快照的 ParseLayer）。
    fn classify_at(
        &mut self,
        i: usize,
    ) -> (
        parser::ParseLayer,
        classifier::Classification,
        Vec<Rc<Vec<LeveledMove>>>,
    ) {
        let l0_i = classifier::stage_profile::time("zz_parser_append", || {
            self.parser_incr.append(self.bars[i])
        });
        let (c, t) = classifier::stage_profile::time("zz_classify_call", || {
            classifier::classify_with_tower_incremental(&l0_i, self.config, &mut self.tower_cache)
        });
        (l0_i, c, t)
    }
}

#[derive(Debug)]
struct LoadedBars {
    bars: Vec<Bar>,
    first_date: String,
    last_date: String,
}

// ═══════════════════════ 发射跟踪（e1 逐行复制：P1 emission 口径 + t1 区间快照） ═══════════════════════

#[derive(Debug, Clone, Copy)]
struct Emission {
    src: usize,
    first_bar: usize,
    level: u8,
}

#[derive(Debug, Clone, Copy)]
struct T1Emission {
    src: usize,
    first_bar: usize,
    level: u8,
    iv_start: usize,
    iv_end: usize,
    iv_kind: u8,
}

#[derive(Debug, Clone, Copy)]
struct T1Rec {
    bar: usize,
    iv_start: usize,
    iv_end: usize,
    iv_kind: u8,
}

fn lookup_interval(tower: &[Rc<Vec<LeveledMove>>], lvl: usize, src: usize) -> (usize, usize, u8) {
    if let Some(moves) = tower.get(lvl) {
        let moves: &[LeveledMove] = moves;
        if let Some(i) = find_move_by_end_index(moves, src) {
            let m = &moves[i];
            return (m.start_index, m.end_index, 0);
        }
        let j = moves.partition_point(|m| m.end_index < src);
        if j < moves.len() && moves[j].start_index <= src {
            let m = &moves[j];
            return (m.start_index, m.end_index, 1);
        }
    }
    (src, src, 2)
}

struct EmissionTracker {
    mirrors: Vec<Vec<(usize, u8)>>,
    seen: HashSet<(usize, usize, u8)>,
    first_conf: HashMap<(usize, usize, i8), usize>,
    first_t3: HashMap<(usize, usize, i8), usize>,
    first_t1: HashMap<(usize, usize, i8), T1Rec>,
    raw_events: usize,
}

impl EmissionTracker {
    fn new() -> Self {
        Self {
            mirrors: Vec::new(),
            seen: HashSet::new(),
            first_conf: HashMap::new(),
            first_t3: HashMap::new(),
            first_t1: HashMap::new(),
            raw_events: 0,
        }
    }

    fn note_t1(
        &mut self,
        bar: usize,
        lvl: usize,
        src: usize,
        side: i8,
        tower: &[Rc<Vec<LeveledMove>>],
    ) {
        self.first_t1.entry((lvl, src, side)).or_insert_with(|| {
            let (iv_start, iv_end, iv_kind) = lookup_interval(tower, lvl, src);
            T1Rec {
                bar,
                iv_start,
                iv_end,
                iv_kind,
            }
        });
    }

    fn scan(
        &mut self,
        bar: usize,
        classification: &classifier::Classification,
        tower: &[Rc<Vec<LeveledMove>>],
    ) {
        let levels = &classification.levels;
        if self.mirrors.len() < levels.len() {
            self.mirrors.resize(levels.len(), Vec::new());
        }
        for (lvl, ls) in levels.iter().enumerate() {
            let cur: &[classifier::bsp::BspPoint] = &ls.bsp;
            let mir = &mut self.mirrors[lvl];
            let common = mir.len().min(cur.len());
            let mut j = 0usize;
            while j < common {
                let p = &cur[j];
                let key = (p.source_index, p.bits.class_index());
                if mir[j] != key {
                    break;
                }
                j += 1;
            }
            if j == cur.len() && j == mir.len() {
                continue;
            }
            self.mirrors[lvl].truncate(j);
            let tail: Vec<(usize, u8, bool, bool, bool, bool, bool, bool)> = cur[j..]
                .iter()
                .map(|p| {
                    (
                        p.source_index,
                        p.bits.class_index(),
                        p.bits.conf_plus(),
                        p.bits.conf_minus(),
                        p.bits.buy3,
                        p.bits.sell3,
                        p.bits.buy1,
                        p.bits.sell1,
                    )
                })
                .collect();
            for (src, ci, cplus, cminus, b3, s3, b1, s1) in tail {
                self.mirrors[lvl].push((src, ci));
                if self.seen.insert((lvl, src, ci)) {
                    self.raw_events += 1;
                    if cplus {
                        self.first_conf.entry((lvl, src, 1)).or_insert(bar);
                    }
                    if cminus {
                        self.first_conf.entry((lvl, src, -1)).or_insert(bar);
                    }
                    if b3 {
                        self.first_t3.entry((lvl, src, 1)).or_insert(bar);
                    }
                    if s3 {
                        self.first_t3.entry((lvl, src, -1)).or_insert(bar);
                    }
                    if b1 {
                        self.note_t1(bar, lvl, src, 1, tower);
                    }
                    if s1 {
                        self.note_t1(bar, lvl, src, -1, tower);
                    }
                }
            }
        }
    }
}

struct EventIndex {
    conf: [Vec<Emission>; 2],
    t1: [Vec<T1Emission>; 2],
}

fn side_idx(side: i8) -> usize {
    if side > 0 {
        0
    } else {
        1
    }
}

fn build_index(tracker: &EmissionTracker) -> EventIndex {
    let mut conf: [Vec<Emission>; 2] = [Vec::new(), Vec::new()];
    let mut t1: [Vec<T1Emission>; 2] = [Vec::new(), Vec::new()];
    for (&(lvl, src, side), &bar) in &tracker.first_conf {
        conf[side_idx(side)].push(Emission {
            src,
            first_bar: bar,
            level: lvl as u8,
        });
    }
    for (&(lvl, src, side), rec) in &tracker.first_t1 {
        t1[side_idx(side)].push(T1Emission {
            src,
            first_bar: rec.bar,
            level: lvl as u8,
            iv_start: rec.iv_start,
            iv_end: rec.iv_end,
            iv_kind: rec.iv_kind,
        });
    }
    for v in conf.iter_mut() {
        v.sort_by_key(|e| (e.src, e.first_bar, e.level));
    }
    for v in t1.iter_mut() {
        v.sort_by_key(|e| (e.src, e.first_bar, e.level));
    }
    EventIndex { conf, t1 }
}

fn range_events<'a>(sorted: &'a [Emission], w0: usize, e: usize) -> &'a [Emission] {
    let lo = sorted.partition_point(|ev| ev.src < w0);
    let hi = sorted.partition_point(|ev| ev.src < e);
    &sorted[lo..hi]
}

fn range_events_t1<'a>(sorted: &'a [T1Emission], w0: usize, e: usize) -> &'a [T1Emission] {
    let lo = sorted.partition_point(|ev| ev.src < w0);
    let hi = sorted.partition_point(|ev| ev.src < e);
    &sorted[lo..hi]
}

// ═══════════════════════ trades.jsonl 解析（e1 逐行复制） ═══════════════════════

#[derive(Debug, Clone, Deserialize)]
struct Trade {
    entry_bar: usize,
    entry: i64,
    dir: i8,
    exit_bar: usize,
    seg_start_index: usize,
}

fn load_trades(path: &Path) -> Result<Vec<Trade>, String> {
    let text =
        std::fs::read_to_string(path).map_err(|e| format!("读取 {} 失败: {e}", path.display()))?;
    let mut out = Vec::new();
    for (ln, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let t: Trade =
            serde_json::from_str(line).map_err(|e| format!("行 {}: JSON 解析失败: {e}", ln + 1))?;
        if t.seg_start_index >= t.entry_bar {
            return Err(format!(
                "行 {}: seg_start_index >= entry_bar（W 空）",
                ln + 1
            ));
        }
        if t.dir != 1 && t.dir != -1 {
            return Err(format!("行 {}: dir={} 非 ±1", ln + 1, t.dir));
        }
        out.push(t);
    }
    Ok(out)
}

// ═══════════════════════ 三锚匹配（e1 判据逐行复刻，只保留匹配布尔——bps 列非 P2 门） ═══════════════════════

#[derive(Debug, Clone)]
struct TradeCalc {
    /// 锚A k=2（conf 链不同级别数 ≥2 达成 bar）——Some ⟺ 主名单（1278 口径）成员。
    a2: Option<usize>,
    /// 锚B（t1 buy1/sell1 首见链，不同级别数 ≥2）。
    b: Option<usize>,
    /// 锚C（锚B 事件集上相邻级别 Sub 包含 + 高→低首见递降）。
    c: Option<usize>,
}

fn depth_bar(evs: &[(usize, u8)], depth_want: usize) -> Option<usize> {
    let mut mask: u32 = 0;
    let mut depth = 0usize;
    for &(bar, lvl) in evs {
        let bit = 1u32 << lvl;
        if mask & bit == 0 {
            mask |= bit;
            depth += 1;
            if depth >= depth_want {
                return Some(bar);
            }
        }
    }
    None
}

fn compute_trade(t: &Trade, idx: &EventIndex) -> TradeCalc {
    let si = side_idx(t.dir);
    let e = t.entry_bar;
    let w0 = t.seg_start_index;

    // 锚A k=2：conf 事件 src ∈ W、first_bar < e，不同级别数 ≥2。
    let mut evs: Vec<(usize, u8)> = range_events(&idx.conf[si], w0, e)
        .iter()
        .filter(|ev| ev.first_bar < e)
        .map(|ev| (ev.first_bar, ev.level))
        .collect();
    evs.sort_unstable();
    let a2 = depth_bar(&evs, 2);

    // 锚B：t1 首见事件同口径，不同级别数 ≥2。
    let t1evs: Vec<&T1Emission> = range_events_t1(&idx.t1[si], w0, e)
        .iter()
        .filter(|ev| ev.first_bar < e)
        .collect();
    let mut evs_b: Vec<(usize, u8)> = t1evs.iter().map(|ev| (ev.first_bar, ev.level)).collect();
    evs_b.sort_unstable();
    let b = depth_bar(&evs_b, 2);

    // 锚C：相邻级别对 (ℓ+1→ℓ)、Sub 包含（nest.rs is_sub 复用）、高→低首见递降，多对取最早。
    let mut best: Option<usize> = None;
    for x in &t1evs {
        for y in &t1evs {
            if x.level != y.level + 1 || x.first_bar > y.first_bar {
                continue;
            }
            let outer = NestInterval {
                end_time: x.iv_end as u64,
                start_time: x.iv_start as u64,
                idx: 0,
            };
            let inner = NestInterval {
                end_time: y.iv_end as u64,
                start_time: y.iv_start as u64,
                idx: 0,
            };
            if !is_sub(&inner, &outer) {
                continue;
            }
            let bar = y.first_bar;
            best = Some(match best {
                None => bar,
                Some(bb) => bb.min(bar),
            });
        }
    }
    TradeCalc { a2, b, c: best }
}

// ═══════════════════════ P1 逐 bit 校验器 ═══════════════════════

/// P1 硬门：每 bar 每级，谓词层 cand_delta=true 事件 (confirm_src, side) 多重集 ≟
/// levels[ℓ].bsp 的 buy1/sell1 (source_index, side) 多重集。
struct P1Checker {
    bars_checked: usize,
    comparisons: usize,
    mismatch_bars: usize,
    per_level_mismatch: HashMap<usize, usize>,
    samples: Vec<String>,
    /// 末 bar 快照：每级 (buy1/sell1 bit 数, 事件总数, cand_delta=true 数, pan_div_diag=true 数)。
    final_snapshot: Vec<(usize, usize, usize, usize)>,
    /// 因果累积（默认档）：每级 (source_index, side) → 首见 bar。
    seen: Vec<HashMap<(usize, i8), usize>>,
    /// 每级 bsp 指纹 (Rc 指针, len, 尾元素 (source_index, buy1, sell1))——未变则 O(1) 跳过。
    fps: Vec<(usize, usize, Option<(usize, bool, bool)>)>,
    /// 每级已扫描长度（尾部重叠窗起点）。
    scanned_len: Vec<usize>,
    /// 终态审计：累积集有而终态确认支无（确认撤回）。
    retracted: usize,
    /// 终态审计：终态确认支有而累积集无（因果漏捕获）。
    uncaptured: usize,
    /// 诊断（非门）：确认支首见 bar 相对 source_index 的最大滞后。
    max_confirm_delay: usize,
    /// 默认档 checkpoint 全比对次数。
    checkpoints: usize,
}

fn side_i8(s: Side) -> i8 {
    if s == Side::Long {
        1
    } else {
        -1
    }
}

impl P1Checker {
    fn new() -> Self {
        Self {
            bars_checked: 0,
            comparisons: 0,
            mismatch_bars: 0,
            per_level_mismatch: HashMap::new(),
            samples: Vec::new(),
            final_snapshot: Vec::new(),
            seen: Vec::new(),
            fps: Vec::new(),
            scanned_len: Vec::new(),
            retracted: 0,
            uncaptured: 0,
            max_confirm_delay: 0,
            checkpoints: 0,
        }
    }

    /// 默认档逐 bar 累积（无谓词层重算）：bsp 指纹未变则 O(1) 跳过；变则从
    /// `scanned_len - OVERLAP` 起只扫尾部窗。完成时语义下前缀确认支冻结；任何违背都会在
    /// [`Self::finalize`] 的全量审计中以 FAIL 暴露，故窗口只影响首见 bar 诊断值，不影响判定。
    fn accumulate(&mut self, bar: usize, classification: &classifier::Classification) {
        const OVERLAP: usize = 64;
        self.bars_checked += 1;
        let nl = classification.levels.len();
        if self.seen.len() < nl {
            self.seen.resize_with(nl, HashMap::new);
            self.fps.resize(nl, (0, 0, None));
            self.scanned_len.resize(nl, 0);
        }
        for (lvl, ls) in classification.levels.iter().enumerate() {
            let fp = (
                Rc::as_ptr(&ls.bsp) as usize,
                ls.bsp.len(),
                ls.bsp
                    .last()
                    .map(|p| (p.source_index, p.bits.buy1, p.bits.sell1)),
            );
            if self.fps[lvl] == fp {
                continue;
            }
            self.fps[lvl] = fp;
            let from = self.scanned_len[lvl]
                .saturating_sub(OVERLAP)
                .min(ls.bsp.len());
            for p in ls.bsp[from..].iter() {
                if p.bits.buy1 {
                    self.seen[lvl].entry((p.source_index, 1)).or_insert(bar);
                }
                if p.bits.sell1 {
                    self.seen[lvl].entry((p.source_index, -1)).or_insert(bar);
                }
            }
            self.scanned_len[lvl] = ls.bsp.len();
        }
    }

    /// 终态定判：谓词层单次重算（rhs）≟ 终态确认支全量扫描（lhs）逐 bit——不一致 ⟹
    /// mismatch（P1 FAIL）。另做因果诊断（非门）：累积集 vs 终态集——撤回/漏捕获为上游
    /// 结构修订的两侧锁步行为（checkpoint 逐 bit 比对已覆盖），只计数报告不入门。
    fn finalize(
        &mut self,
        _bar: usize,
        classification: &classifier::Classification,
        cand: &[Vec<CandDeltaEvent>],
    ) {
        self.final_snapshot.clear();
        for (lvl, ls) in classification.levels.iter().enumerate() {
            let mut lhs: Vec<(usize, i8)> = Vec::new();
            for p in ls.bsp.iter() {
                if p.bits.buy1 {
                    lhs.push((p.source_index, 1));
                }
                if p.bits.sell1 {
                    lhs.push((p.source_index, -1));
                }
            }
            let evs = &cand[lvl];
            let mut rhs: Vec<(usize, i8)> = evs
                .iter()
                .filter(|e| e.cand_delta)
                .map(|e| (e.confirm_src, side_i8(e.side)))
                .collect();
            lhs.sort_unstable();
            rhs.sort_unstable();
            self.comparisons += 1;
            self.final_snapshot.push((
                lhs.len(),
                evs.len(),
                rhs.len(),
                evs.iter().filter(|e| e.pan_div_diag).count(),
            ));
            let mut level_bad = false;
            if lhs != rhs {
                level_bad = true;
                if self.samples.len() < 10 {
                    let only_l: Vec<_> = lhs.iter().filter(|k| !rhs.contains(k)).take(4).collect();
                    let only_r: Vec<_> = rhs.iter().filter(|k| !lhs.contains(k)).take(4).collect();
                    self.samples.push(format!(
                        "终态 ℓ{lvl}: bsp={} cand={}；仅 bsp 侧 {:?}，仅谓词侧 {:?}",
                        lhs.len(),
                        rhs.len(),
                        only_l,
                        only_r
                    ));
                }
            }
            // 因果审计：默认档才有累积集（perbar 档 seen 为空 ⟹ 跳过，逐 bar 比对已覆盖）。
            if let Some(seen) = self.seen.get(lvl) {
                if !seen.is_empty() || !lhs.is_empty() {
                    let fset: HashSet<(usize, i8)> = lhs.iter().copied().collect();
                    for (k, &first_bar) in seen {
                        if fset.contains(k) {
                            self.max_confirm_delay =
                                self.max_confirm_delay.max(first_bar.saturating_sub(k.0));
                        } else {
                            // 上游结构修订的锁步撤回（两侧同步，checkpoint 比对覆盖）——诊断非门。
                            self.retracted += 1;
                            if self.samples.len() < 10 {
                                self.samples.push(format!(
                                    "诊断·确认撤回 ℓ{lvl}: {k:?} 首见 bar={first_bar}，终态无"
                                ));
                            }
                        }
                    }
                    for k in &fset {
                        if !seen.contains_key(k) {
                            self.uncaptured += 1;
                            if self.samples.len() < 10 {
                                self.samples.push(format!(
                                    "诊断·因果漏捕获 ℓ{lvl}: {k:?} 终态有，重放累积无"
                                ));
                            }
                        }
                    }
                }
            }
            if level_bad {
                *self.per_level_mismatch.entry(lvl).or_insert(0) += 1;
                self.mismatch_bars += 1;
            }
        }
    }

    /// 逐 bar 档（STRICT_NEST_P1_PERBAR=1）：每 bar 全比对。
    fn compare(
        &mut self,
        bar: usize,
        classification: &classifier::Classification,
        cand: &[Vec<CandDeltaEvent>],
    ) {
        self.bars_checked += 1;
        self.compare_inner(bar, classification, cand);
    }

    /// 默认档 checkpoint：每 STRICT_NEST_P1_CHECKPOINT bar 一次全比对（bar 数由
    /// [`Self::accumulate`] 计，不重复计入）。
    fn checkpoint(
        &mut self,
        bar: usize,
        classification: &classifier::Classification,
        cand: &[Vec<CandDeltaEvent>],
    ) {
        self.checkpoints += 1;
        self.compare_inner(bar, classification, cand);
    }

    /// 单 bar 快照全比对核心（P1 硬门本体）：该 bar 下每级 bsp buy1/sell1 多重集 ≟ 谓词
    /// cand_delta 多重集，逐 bit。
    fn compare_inner(
        &mut self,
        bar: usize,
        classification: &classifier::Classification,
        cand: &[Vec<CandDeltaEvent>],
    ) {
        let mut bar_bad = false;
        self.final_snapshot.clear();
        for (lvl, ls) in classification.levels.iter().enumerate() {
            let mut lhs: Vec<(usize, i8)> = Vec::new();
            for p in ls.bsp.iter() {
                if p.bits.buy1 {
                    lhs.push((p.source_index, 1));
                }
                if p.bits.sell1 {
                    lhs.push((p.source_index, -1));
                }
            }
            let evs = &cand[lvl];
            let mut rhs: Vec<(usize, i8)> = evs
                .iter()
                .filter(|e| e.cand_delta)
                .map(|e| (e.confirm_src, side_i8(e.side)))
                .collect();
            lhs.sort_unstable();
            rhs.sort_unstable();
            self.comparisons += 1;
            self.final_snapshot.push((
                lhs.len(),
                evs.len(),
                rhs.len(),
                evs.iter().filter(|e| e.pan_div_diag).count(),
            ));
            if lhs != rhs {
                bar_bad = true;
                *self.per_level_mismatch.entry(lvl).or_insert(0) += 1;
                if self.samples.len() < 10 {
                    let only_l: Vec<_> = lhs.iter().filter(|k| !rhs.contains(k)).take(4).collect();
                    let only_r: Vec<_> = rhs.iter().filter(|k| !lhs.contains(k)).take(4).collect();
                    self.samples.push(format!(
                        "bar={bar} ℓ{lvl}: bsp={} cand={}；仅 bsp 侧 {:?}，仅谓词侧 {:?}",
                        lhs.len(),
                        rhs.len(),
                        only_l,
                        only_r
                    ));
                }
            }
        }
        if bar_bad {
            self.mismatch_bars += 1;
        }
    }

    fn pass(&self) -> bool {
        self.mismatch_bars == 0
    }
}

// ═══════════════════════ 主流程 ═══════════════════════

/// E1 三元组期望（P2 验收比对表基准，E1-RESULT/P7-RESULT 冻结数字）。
const T1_TOTAL_EXPECTED: usize = 942;
const T1_PER_LEVEL_EXPECTED: [(usize, usize); 5] = [(0, 533), (1, 37), (2, 95), (3, 142), (4, 135)];
const N_MAIN_EXPECTED: usize = 1278;
const N_B_EXPECTED: usize = 1;
const N_C_EXPECTED: usize = 0;
/// P7-RESULT 头部五元组：raw seen / conf 键 / type3 键 / entry==close / 零长度。
const P7_HDR_EXPECTED: (usize, usize, usize, usize, usize) = (29088, 27152, 15165, 40001, 2955);

// ═══════════════════════ 证书漏斗只读插桩（不改判据） ═══════════════════════

/// 相邻级父/子 Cand 事件的三个原子门诊断。
///
/// 逐字镜像 `nest.rs::extend_upward`：方向一致，以及闭区间
/// `I(A_child) ⊆ D_parent` 的左右边界。确认时点仅登记延迟，不参与门。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PairGateDiag {
    same_side: bool,
    sub_start: bool,
    sub_end: bool,
}

impl PairGateDiag {
    fn passes(self) -> bool {
        self.same_side && self.sub_start && self.sub_end
    }

    fn failed_conditions(self) -> usize {
        [self.same_side, self.sub_start, self.sub_end]
            .into_iter()
            .filter(|ok| !ok)
            .count()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParentWindowMode {
    /// 2026-07-10 冻结基线，仅用于同输入复核；不得进入正式装配。
    EpisodeBaseline,
    /// #43 现行口径：只接受已闭合的完整 c_p 证书。
    FullCp,
}

fn parent_interval(parent: &CandDeltaEvent, mode: ParentWindowMode) -> Option<(usize, usize)> {
    match mode {
        ParentWindowMode::EpisodeBaseline => Some(parent.c_episode_interval),
        ParentWindowMode::FullCp => parent.c_interval_full,
    }
}

fn diagnose_pair(
    parent: &CandDeltaEvent,
    child: &CandDeltaEvent,
    mode: ParentWindowMode,
) -> Option<PairGateDiag> {
    let (parent_start, parent_end) = parent_interval(parent, mode)?;
    Some(PairGateDiag {
        same_side: parent.side == child.side,
        sub_start: child.a_interval.0 >= parent_start,
        sub_end: child.a_interval.1 <= parent_end,
    })
}

fn confirmation_lag(
    parent: &CandDeltaEvent,
    child: &CandDeltaEvent,
    mode: ParentWindowMode,
) -> Option<i128> {
    let (_, parent_end) = parent_interval(parent, mode)?;
    Some(child.confirm_src as i128 - parent_end as i128)
}

/// 数值门离通过还差多少根 bar；通过门贡献 0。方向门单独由 failed_conditions 排序。
fn pair_gap_bars(
    parent: &CandDeltaEvent,
    child: &CandDeltaEvent,
    mode: ParentWindowMode,
) -> Option<(usize, usize)> {
    let (parent_start, parent_end) = parent_interval(parent, mode)?;
    Some((
        parent_start.saturating_sub(child.a_interval.0),
        child.a_interval.1.saturating_sub(parent_end),
    ))
}

#[derive(Debug, Clone, Default)]
struct FunnelLevel {
    predicate_hits: usize,
    structural_events: usize,
    cand_candidates: usize,
    reachable_pair_successes: usize,
    reachable_candidates: usize,
    certificates: usize,
    full_parent_events: usize,
    incomplete_parent_events: usize,
    incomplete_pair_rejections: usize,
    lag_considered: Vec<i128>,
    lag_accepted: Vec<i128>,
}

/// F-07：首个归零段的确定原因（漏斗阶段：structural → Cand → terminal/base reachable →
/// pair edges → reachable candidate → certificate）。此前检测强制要求本级
/// `cand_candidates > 0`，L0 terminal 全查无、高级别候选空集、终门归零都只能落入
/// 模糊的“需检查证书终门或更高层候选空集”。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FirstZeroCause {
    /// L0 有 `Cand^δ` 基例但 terminal 全部查无（base reachable 归零）。
    L0TerminalAllMissing,
    /// 该级 `Cand^δ` 候选为空（上一级仍有可达 Cand）。
    CandEmpty(usize),
    /// 上下两级均有候选但可达递降链配对为 0（原有情形）。
    PairZero(usize),
    /// 配对成功但可达本级 Cand 归零（防御性分支，正常不应出现）。
    ReachableZero(usize),
    /// 可达 Cand 存在但证书终门归零。
    CertificateZero(usize),
}

/// 查找“上一阶段非零、当前阶段为零”的首个断点；全程非零（或结构层本身为空）返回 None。
fn first_zero_cause(funnel_levels: &[FunnelLevel]) -> Option<FirstZeroCause> {
    let l0 = funnel_levels.first()?;
    if l0.cand_candidates > 0 && l0.reachable_candidates == 0 {
        return Some(FirstZeroCause::L0TerminalAllMissing);
    }
    for level in 1..funnel_levels.len() {
        if funnel_levels[level - 1].reachable_candidates == 0 {
            break; // 上游已空（如 L0 即无候选），无“非零→零”断点可归因
        }
        let cur = &funnel_levels[level];
        if cur.cand_candidates == 0 {
            return Some(FirstZeroCause::CandEmpty(level));
        }
        if cur.reachable_pair_successes == 0 {
            return Some(FirstZeroCause::PairZero(level));
        }
        if cur.reachable_candidates == 0 {
            return Some(FirstZeroCause::ReachableZero(level));
        }
        if cur.certificates == 0 {
            return Some(FirstZeroCause::CertificateZero(level));
        }
    }
    None
}

#[derive(Debug, Clone, Copy)]
struct NearMiss {
    parent_level: usize,
    parent_idx: usize,
    child_idx: usize,
    diag: PairGateDiag,
    start_gap: usize,
    end_gap: usize,
    confirm_lag: i128,
}

impl NearMiss {
    fn rank_key(self) -> (usize, usize, usize, usize, usize) {
        (
            self.diag.failed_conditions(),
            self.start_gap + self.end_gap,
            self.start_gap,
            self.end_gap,
            self.parent_idx,
        )
    }
}

/// 计算从 L0 基例向上的可达漏斗。
///
/// `reachable_pair_successes[ℓ]` 只计能把一条已从 L0 到达 ℓ-1 的 partial chain 延长一级的
/// 相邻父子边；因此第一个 0 正是完整递降链第一次断裂处，而不是与 L0 无关的孤立高层配对。
fn build_cert_funnel(
    classification: &classifier::Classification,
    events_by_level: &[Vec<CandDeltaEvent>],
    terminal_by_key: &HashMap<(usize, i8), BspBits>,
    mode: ParentWindowMode,
) -> (Vec<FunnelLevel>, Vec<Vec<bool>>) {
    let level_count = events_by_level.len().max(classification.levels.len());
    let mut levels = vec![FunnelLevel::default(); level_count];
    for (level, state) in classification.levels.iter().enumerate() {
        levels[level].predicate_hits = state
            .bsp
            .iter()
            .map(|p| usize::from(p.bits.buy1) + usize::from(p.bits.sell1))
            .sum();
    }
    for (level, events) in events_by_level.iter().enumerate() {
        levels[level].structural_events = events.len();
        levels[level].cand_candidates = events.iter().filter(|e| e.cand_delta).count();
        if mode == ParentWindowMode::FullCp && level > 0 {
            levels[level].full_parent_events = events
                .iter()
                .filter(|e| e.cand_delta && e.c_interval_full.is_some())
                .count();
            levels[level].incomplete_parent_events = events
                .iter()
                .filter(|e| e.cand_delta && e.c_interval_full.is_none())
                .count();
        }
    }

    let mut reachable: Vec<Vec<bool>> = events_by_level
        .iter()
        .map(|events| vec![false; events.len()])
        .collect();
    if let Some(base_events) = events_by_level.first() {
        for (idx, base) in base_events.iter().enumerate() {
            reachable[0][idx] = base.cand_delta
                && terminal_by_key.contains_key(&(base.confirm_src, side_i8(base.side)));
        }
        levels[0].reachable_candidates = reachable[0].iter().filter(|&&r| r).count();
    }

    for level in 1..events_by_level.len() {
        let (lower, upper) = events_by_level.split_at(level);
        let children = &lower[level - 1];
        let parents = &upper[0];
        for (parent_idx, parent) in parents.iter().enumerate().filter(|(_, e)| e.cand_delta) {
            let mut parent_reachable = false;
            if parent_interval(parent, mode).is_none() {
                levels[level].incomplete_pair_rejections += children
                    .iter()
                    .enumerate()
                    .filter(|(child_idx, child)| {
                        reachable[level - 1][*child_idx]
                            && child.cand_delta
                            && child.side == parent.side
                    })
                    .count();
                continue;
            }
            for (child_idx, child) in children.iter().enumerate() {
                if !reachable[level - 1][child_idx] || !child.cand_delta {
                    continue;
                }
                let diag =
                    diagnose_pair(parent, child, mode).expect("父完整区间已在循环前验证为 Some");
                if diag.same_side {
                    levels[level]
                        .lag_considered
                        .push(confirmation_lag(parent, child, mode).expect("父区间已闭合"));
                }
                if diag.passes() {
                    levels[level].reachable_pair_successes += 1;
                    levels[level]
                        .lag_accepted
                        .push(confirmation_lag(parent, child, mode).expect("父区间已闭合"));
                    parent_reachable = true;
                }
            }
            reachable[level][parent_idx] = parent_reachable;
        }
        levels[level].reachable_candidates = reachable[level].iter().filter(|&&r| r).count();
    }
    (levels, reachable)
}

/// 重放专用的双口径证书计数。正式生产装配只走 `nest::assemble_certificates` 的 FullCp；
/// EpisodeBaseline 仅用于 §7.B 在同一终态快照上复核 2026-07-10 基线。
fn can_extend_for_mode(
    events_by_level: &[Vec<CandDeltaEvent>],
    side: Side,
    level: usize,
    top_level: usize,
    child_interval: (usize, usize),
    mode: ParentWindowMode,
) -> bool {
    if level > top_level {
        return true;
    }
    let Some(parents) = events_by_level.get(level) else {
        return false;
    };
    let mut order: Vec<usize> = (0..parents.len()).collect();
    order.sort_by_key(|&i| {
        (
            parent_interval(&parents[i], mode).unwrap_or((usize::MAX, usize::MAX)),
            parents[i].a_interval,
            i,
        )
    });
    order.into_iter().any(|i| {
        let parent = &parents[i];
        if !parent.cand_delta || parent.side != side {
            return false;
        }
        let Some(parent_iv) = parent_interval(parent, mode) else {
            return false;
        };
        child_interval.0 >= parent_iv.0
            && child_interval.1 <= parent_iv.1
            && can_extend_for_mode(
                events_by_level,
                side,
                level + 1,
                top_level,
                parent.a_interval,
                mode,
            )
    })
}

fn count_certificates_for_mode(
    events_by_level: &[Vec<CandDeltaEvent>],
    terminal_by_key: &HashMap<(usize, i8), BspBits>,
    top_level: usize,
    mode: ParentWindowMode,
) -> usize {
    events_by_level
        .first()
        .into_iter()
        .flatten()
        .filter(|base| base.cand_delta)
        .filter(|base| {
            terminal_by_key.contains_key(&(base.confirm_src, side_i8(base.side)))
                && can_extend_for_mode(
                    events_by_level,
                    base.side,
                    1,
                    top_level,
                    base.a_interval,
                    mode,
                )
        })
        .count()
}

/// 终态研究投影：原事件保持不变；每个克隆只沿稳定边装入终态证书，供显式 terminal 诊断。
/// 没有终态证书的事件清空快照证书字段，绝不回退 episode 或确认点。
fn terminal_event_projection(
    classification: &classifier::Classification,
    event_time: &[Vec<CandDeltaEvent>],
) -> Vec<Vec<CandDeltaEvent>> {
    event_time
        .iter()
        .enumerate()
        .map(|(level, events)| {
            let objects = classification
                .levels
                .get(level)
                .map(|state| state.cp_ownership.as_slice())
                .unwrap_or(&[]);
            events
                .iter()
                .map(|event| {
                    let mut terminal = event.clone();
                    match cp_terminal_certificate(event, objects) {
                        Some(certificate) => {
                            terminal.cp_certificate_confirm_src =
                                Some(certificate.cp_certificate_confirm_src);
                            terminal.c_structure = Some(certificate.c_structure);
                            terminal.third_class_in_c = Some(certificate.third_class_in_c);
                            terminal.c_interval_full = Some(certificate.c_interval_full);
                            terminal.full_trend_evidence = certificate.full_trend_evidence;
                            terminal.full_trend_c_qualified = certificate.full_trend_c_qualified;
                        }
                        None => {
                            terminal.cp_certificate_confirm_src = None;
                            terminal.c_structure = terminal.c_structure.map(|mut structure| {
                                structure.terminal_move_id = None;
                                structure.source_end = None;
                                structure
                            });
                            terminal.third_class_in_c = None;
                            terminal.c_interval_full = None;
                            terminal.full_trend_evidence = None;
                            terminal.full_trend_c_qualified = None;
                        }
                    }
                    terminal
                })
                .collect()
        })
        .collect()
}

#[derive(Debug, Clone, Copy)]
struct RecursiveOwnershipProof {
    parent_first: ElementId,
    parent_last: ElementId,
    child_first: ElementId,
    child_last: ElementId,
    child_span: (usize, usize),
}

/// 为“child.a_interval 属于 parent.c_p 内部”生成独立递归结构证据：先按父 `c_p` 的同级
/// ElementId 首尾取连续组件，再从这些组件的 `sub_moves` 侧车中定位子级 I(A) 的精确首尾。
/// 数值闭包含仍由原 Sub 门判断；本函数不参与产量门，只用于逐接受边证据验收。
fn prove_child_structural_ownership(
    parent: &CandDeltaEvent,
    child: &CandDeltaEvent,
    tower: &[Rc<Vec<LeveledMove>>],
) -> Option<RecursiveOwnershipProof> {
    if parent.level != child.level + 1 {
        return None;
    }
    let edge = parent.cp_ownership?;
    let c = parent.c_structure?;
    let terminal_move_id = c.terminal_move_id?;
    let parent_moves = tower.get(parent.level as usize)?;
    let components: Vec<&LeveledMove> = parent_moves
        .iter()
        .filter(|m| {
            m.id.level == edge.cp_departure_move_id.level
                && edge.cp_departure_move_id.ordinal <= m.id.ordinal
                && m.id.ordinal <= terminal_move_id.ordinal
        })
        .collect();
    let (first_parent, last_parent) = (components.first()?, components.last()?);
    if first_parent.id != edge.cp_departure_move_id
        || last_parent.id != terminal_move_id
        || first_parent.start_index != edge.cp_source_start
        || last_parent.end_index != c.source_end?
        || !components
            .windows(2)
            .all(|w| w[0].id.ordinal + 1 == w[1].id.ordinal)
    {
        return None;
    }

    let descendants: Vec<&LeveledMove> = components
        .iter()
        .flat_map(|m| m.sub_moves.iter())
        .filter(|m| {
            m.id.level == child.level
                && m.end_index >= child.a_interval.0
                && m.start_index <= child.a_interval.1
        })
        .collect();
    let (first_child, last_child) = (descendants.first()?, descendants.last()?);
    if first_child.start_index != child.a_interval.0
        || last_child.end_index != child.a_interval.1
        || !descendants
            .windows(2)
            .all(|w| w[0].id.ordinal + 1 == w[1].id.ordinal)
    {
        return None;
    }
    Some(RecursiveOwnershipProof {
        parent_first: first_parent.id,
        parent_last: last_parent.id,
        child_first: first_child.id,
        child_last: last_child.id,
        child_span: (first_child.start_index, last_child.end_index),
    })
}

fn full_parent_evidence_ok(event: &CandDeltaEvent) -> bool {
    let Some((start, end)) = event.c_interval_full else {
        return true; // 未闭合由消费者显式拒绝，不构成伪证书。
    };
    let (Some(b), Some(c), Some(third), Some(edge)) = (
        event.b_parent,
        event.c_structure,
        event.third_class_in_c,
        event.cp_ownership,
    ) else {
        return false;
    };
    c.source_start == start
        && c.source_end == Some(end)
        && c.b_center_id == b.center_id
        && edge.b_center_id == b.center_id
        && edge.cp_departure_move_id == c.departure_move_id
        && edge.cp_source_start == start
        && event.cp_certificate_confirm_src == Some(third.point_source_index)
        && third.b_center_id == b.center_id
        && third.cp_departure_move_id == c.departure_move_id
        && start >= b.source_interval.1
        && start > event.a_interval.0
        && start <= third.departure_interval.0
        && third.retest_interval.1 <= end
}

fn c_start_mapping_ok(event: &CandDeltaEvent) -> bool {
    match (event.b_parent, event.c_structure) {
        (Some(b), Some(c)) => {
            c.b_center_id == b.center_id
                && c.source_start >= b.source_interval.1
                && c.source_start > event.a_interval.0
        }
        // 逐事件部分函数允许尚无离开单元；此类事件必须保持 None 并由消费端拒绝。
        (_, None) => event.c_interval_full.is_none() && event.cp_ownership.is_none(),
        (None, Some(_)) => false,
    }
}

fn closest_pair_misses(
    events_by_level: &[Vec<CandDeltaEvent>],
    reachable: &[Vec<bool>],
    parent_level: usize,
    limit: usize,
    mode: ParentWindowMode,
) -> Vec<NearMiss> {
    if parent_level == 0 || parent_level >= events_by_level.len() {
        return Vec::new();
    }
    let mut misses = Vec::new();
    for (parent_idx, parent) in events_by_level[parent_level]
        .iter()
        .enumerate()
        .filter(|(_, e)| e.cand_delta)
    {
        for (child_idx, child) in events_by_level[parent_level - 1].iter().enumerate() {
            if !reachable[parent_level - 1][child_idx] || !child.cand_delta {
                continue;
            }
            let Some(diag) = diagnose_pair(parent, child, mode) else {
                continue;
            };
            if diag.passes() {
                continue;
            }
            let (start_gap, end_gap) =
                pair_gap_bars(parent, child, mode).expect("diagnose_pair Some 蕴含父区间 Some");
            misses.push(NearMiss {
                parent_level,
                parent_idx,
                child_idx,
                diag,
                start_gap,
                end_gap,
                confirm_lag: confirmation_lag(parent, child, mode)
                    .expect("diagnose_pair Some 蕴含父区间 Some"),
            });
        }
    }
    misses.sort_by_key(|m| m.rank_key());
    misses.truncate(limit);
    misses
}

fn failed_condition_text(miss: NearMiss) -> String {
    let mut failed = Vec::new();
    if !miss.diag.same_side {
        failed.push("方向 δ 不一致".to_string());
    }
    if !miss.diag.sub_start {
        failed.push(format!("Sub 左界失败（子起点早 {} bar）", miss.start_gap));
    }
    if !miss.diag.sub_end {
        failed.push(format!("Sub 右界失败（子终点晚 {} bar）", miss.end_gap));
    }
    failed.join("；")
}

fn percentile(sorted: &[i128], pct: usize) -> i128 {
    sorted[(sorted.len() - 1) * pct / 100]
}

fn lag_distribution(values: &[i128]) -> String {
    if values.is_empty() {
        return "n=0".to_string();
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let negative = sorted.iter().filter(|&&v| v < 0).count();
    let zero = sorted.iter().filter(|&&v| v == 0).count();
    let positive = sorted.len() - negative - zero;
    format!(
        "n={}；负/零/正={}/{}/{}；min/p25/p50/p75/p90/p95/max={}/{}/{}/{}/{}/{}/{}",
        sorted.len(),
        negative,
        zero,
        positive,
        sorted[0],
        percentile(&sorted, 25),
        percentile(&sorted, 50),
        percentile(&sorted, 75),
        percentile(&sorted, 90),
        percentile(&sorted, 95),
        sorted[sorted.len() - 1]
    )
}

fn dparent_enter_mismatches(events_by_level: &[Vec<CandDeltaEvent>]) -> usize {
    events_by_level
        .iter()
        .flatten()
        .filter(|event| event.cand_delta && event.enter_src != event.interval.0)
        .count()
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(pass) => {
            if pass {
                std::process::ExitCode::SUCCESS
            } else {
                std::process::ExitCode::FAILURE
            }
        }
        Err(e) => {
            eprintln!("strict_nest_check 失败: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[allow(clippy::too_many_lines)]
fn run() -> Result<bool, String> {
    let config = ThetaConfig::default();
    let out_root = match std::env::var("STRICT_NEST_REPORT_ROOT") {
        Ok(root) => PathBuf::from(root),
        Err(_) => PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .ok_or("无法定位 worktree 根目录")?
            .to_path_buf(),
    };
    // 数据只在 /tmp/codex-work-p7（只读引用，worktree 无数据缓存）；env 可覆盖。
    let data_root =
        std::env::var("STRICT_NEST_DATA_ROOT").unwrap_or_else(|_| "/tmp/codex-work-p7".to_string());
    let data_path = PathBuf::from(&data_root).join("analysis/data_cache/btc_1m_full.json");
    let trades_path = PathBuf::from(&data_root).join("p7_inputs/trades.jsonl");

    let t_load = Instant::now();
    let loaded = load_btc_bars(&data_path, config.tick.tick_size)?;
    let total_bars = loaded.bars.len();
    if total_bars == 0 {
        return Err("BTC 数据为空".to_string());
    }
    let trades = load_trades(&trades_path)?;
    for t in &trades {
        if t.entry_bar >= total_bars || t.exit_bar >= total_bars {
            return Err("trade bar 越界".to_string());
        }
    }
    eprintln!(
        "数据加载 {:.2}s：{} bar（{} .. {}），{} 笔交易",
        t_load.elapsed().as_secs_f64(),
        total_bars,
        loaded.first_date,
        loaded.last_date,
        trades.len()
    );

    // ── 全量因果重放：发射跟踪 + P1 逐 bit 校验（同一快照内） ──
    let mut classifier_incr = IncrementalClassifier::new(&loaded.bars, &config);
    let mut tracker = EmissionTracker::new();
    let mut p1 = P1Checker::new();
    let t_replay = Instant::now();
    // 诊断用重放上限（DIAG_CANDCACHE 对拍短跑）；正式判定必须全量（未设 = total_bars）。
    let replay_bars: usize = std::env::var("STRICT_NEST_MAX_BARS")
        .ok()
        .and_then(|s| s.parse().ok())
        .map_or(total_bars, |m: usize| m.min(total_bars));
    if replay_bars < total_bars {
        eprintln!("★诊断截断：只重放前 {replay_bars}/{total_bars} bar（判定不作数）");
    }
    // P1 档位：默认 = 因果累积 + checkpoint 全比对 + 终态定判（摊还 O(n) + O(n/K) 次谓词
    // 重算）；STRICT_NEST_P1_PERBAR=1 = 每 bar 每级全比对（谓词层逐 bar 重算
    // level_cand_delta，实测分段耗时线性增长 ⟹ O(n²)，全量外推 20h+——只配
    // STRICT_NEST_MAX_BARS 做前缀逐 bar 证据）。判据同源零分叉。
    let perbar = std::env::var("STRICT_NEST_P1_PERBAR").is_ok();
    let ckpt_every: usize = std::env::var("STRICT_NEST_P1_CHECKPOINT")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|&k| k > 0)
        .unwrap_or(1000);
    if perbar {
        eprintln!("★P1 逐 bar 档（O(n²)）：仅限前缀诊断");
    } else {
        eprintln!("P1 checkpoint 间隔 = {ckpt_every} bar");
    }
    // 注意：ParseLayer（l0_i）不得跨 bar 持有——parser append 的 Rc::make_mut 折回
    // （parser/mod.rs:215）在强引用 >1 时退化为全量深拷贝（O(n)/bar ⟹ O(n²) 总量，
    // 450k 前缀实测 append 157.4s/165s）。故 l0 仅末 bar 保留；checkpoint 用当轮 l0_i。
    let mut last_state = None;
    let mut last_l0 = None;
    // 分项计时（诊断用，STRICT_NEST_PROFILE=1 时随 200k 心跳打印；不改判据路径）。
    let profile = std::env::var("STRICT_NEST_PROFILE").is_ok();
    let (mut t_cls, mut t_scan, mut t_p1) = (
        std::time::Duration::ZERO,
        std::time::Duration::ZERO,
        std::time::Duration::ZERO,
    );
    for i in 0..replay_bars {
        let t0 = std::time::Instant::now();
        let (l0_i, classification, tower) = classifier_incr.classify_at(i);
        if profile {
            t_cls += t0.elapsed();
        }
        let t0 = std::time::Instant::now();
        tracker.scan(i, &classification, &tower);
        if profile {
            t_scan += t0.elapsed();
        }
        let t0 = std::time::Instant::now();
        if perbar {
            // 缓存序列变体（判据零分叉，见 mod.rs cand_delta_tower_cached 文档）。
            let cand = classifier::cand_delta_tower_cached(
                &l0_i,
                &classification,
                &tower,
                &config,
                &classifier_incr.tower_cache,
            );
            p1.compare(i, &classification, &cand);
        } else {
            p1.accumulate(i, &classification);
            if i % ckpt_every == 0 && i > 0 {
                let cand = classifier::cand_delta_tower_cached(
                    &l0_i,
                    &classification,
                    &tower,
                    &config,
                    &classifier_incr.tower_cache,
                );
                p1.checkpoint(i, &classification, &cand);
            }
        }
        if profile {
            t_p1 += t0.elapsed();
        }
        if i % 200_000 == 0 && i > 0 {
            eprintln!(
                "  replay {}/{}（{:.1}s，t1_keys={}，P1 mismatch bars={}）",
                i,
                total_bars,
                t_replay.elapsed().as_secs_f64(),
                tracker.first_t1.len(),
                p1.mismatch_bars
            );
            if profile {
                eprintln!(
                    "    profile：classify_at={:.1}s scan={:.1}s p1={:.1}s",
                    t_cls.as_secs_f64(),
                    t_scan.as_secs_f64(),
                    t_p1.as_secs_f64()
                );
            }
            // THETA_PROFILE_STAGES=1 时打印 classify 内部阶段累计（未启用零开销直通）。
            classifier::stage_profile::dump();
        }
        if i + 1 == replay_bars {
            last_l0 = Some(l0_i); // 仅末 bar 保留（此后不再 append，Rc 共享无害）
        }
        last_state = Some((classification, tower));
    }
    // 终态谓词层单次重算 + P1 定判（默认档主判；perbar 档下为幂等复核）。
    let (cls_f, tower_f) = last_state.ok_or("重放为空")?;
    let l0_f = last_l0.ok_or("重放为空")?;
    let event_time_cand_f = classifier::cand_delta_tower_cached(
        &l0_f,
        &cls_f,
        &tower_f,
        &config,
        &classifier_incr.tower_cache,
    );
    // GATED-3：后续结构/区间套诊断显式选择 terminal；P1 finalize 仍只读 event-time 快照。
    let cand_f = terminal_event_projection(&cls_f, &event_time_cand_f);
    let objects_by_level: Vec<&[CpScanOwnership]> = cls_f
        .levels
        .iter()
        .map(|state| state.cp_ownership.as_slice())
        .collect();
    // 冻结诊断不变量：release 复跑也必须实际检查，不能只依赖 debug_assert。
    let dparent_enter_mismatch = dparent_enter_mismatches(&cand_f);
    p1.finalize(replay_bars - 1, &cls_f, &event_time_cand_f);
    let replay_sec = t_replay.elapsed().as_secs_f64();
    eprintln!("重放完成 {replay_sec:.1}s（含终态谓词定判）");

    // ── sanity 五元组 ──
    let entry_eq_close = trades
        .iter()
        .filter(|t| loaded.bars[t.entry_bar].close == t.entry)
        .count();
    let zero_len = trades.iter().filter(|t| t.entry_bar == t.exit_bar).count();
    let hdr_got = (
        tracker.raw_events,
        tracker.first_conf.len(),
        tracker.first_t3.len(),
        entry_eq_close,
        zero_len,
    );
    let hdr_ok = hdr_got == P7_HDR_EXPECTED;

    // ── E1 三元组复算 ──
    let idx = build_index(&tracker);
    let calcs: Vec<TradeCalc> = trades.iter().map(|t| compute_trade(t, &idx)).collect();
    let main_list: Vec<&TradeCalc> = calcs.iter().filter(|c| c.a2.is_some()).collect();
    let n_main = main_list.len();
    let n_b = main_list.iter().filter(|c| c.b.is_some()).count();
    let n_c = main_list.iter().filter(|c| c.c.is_some()).count();

    // ── #100 侧信道：BSP 发射时间线导出（BSP_DUMP=<path>，只写不改任何判定） ──
    if let Ok(dump_path) = std::env::var("BSP_DUMP") {
        use std::io::Write as _;
        match std::fs::File::create(&dump_path) {
            Ok(file) => {
                let mut w = std::io::BufWriter::new(file);
                for (&(lvl, src, side), &bar) in &tracker.first_conf {
                    let _ = writeln!(
                        w,
                        "BSP_CONF lvl={lvl} src={src} side={side} first_bar={bar}"
                    );
                }
                for (&(lvl, src, side), rec) in &tracker.first_t1 {
                    let _ = writeln!(
                        w,
                        "BSP_T1 lvl={lvl} src={src} side={side} first_bar={} iv={}-{} iv_kind={}",
                        rec.bar, rec.iv_start, rec.iv_end, rec.iv_kind
                    );
                }
                for (&(lvl, src, side), &bar) in &tracker.first_t3 {
                    let _ = writeln!(w, "BSP_T3 lvl={lvl} src={src} side={side} first_bar={bar}");
                }
                for (t, c) in trades.iter().zip(&calcs) {
                    let _ = writeln!(
                        w,
                        "BSP_TRADE entry_bar={} dir={} w0={} exit_bar={} a2={:?} b={:?} c={:?}",
                        t.entry_bar, t.dir, t.seg_start_index, t.exit_bar, c.a2, c.b, c.c
                    );
                }
                let _ = w.flush();
                eprintln!("BSP_DUMP 已写出: {dump_path}");
            }
            Err(error) => eprintln!("BSP_DUMP 创建 {dump_path} 失败（跳过导出）: {error}"),
        }
    }

    let mut per_level_t1: HashMap<usize, usize> = HashMap::new();
    for &(lvl, _src, _side) in tracker.first_t1.keys() {
        *per_level_t1.entry(lvl).or_insert(0) += 1;
    }
    let t1_total = tracker.first_t1.len();
    let t1_levels_ok = T1_PER_LEVEL_EXPECTED
        .iter()
        .all(|&(l, n)| per_level_t1.get(&l).copied().unwrap_or(0) == n)
        && per_level_t1.len() == T1_PER_LEVEL_EXPECTED.len();
    let triple_ok = t1_total == T1_TOTAL_EXPECTED
        && t1_levels_ok
        && n_main == N_MAIN_EXPECTED
        && n_b == N_B_EXPECTED
        && n_c == N_C_EXPECTED;

    // ── P2：证书生产装配（nest.rs 装配层；终态谓词事件 + 终态分类 bits，全局口径）──
    // terminal 查找：ℓ0 终态 bsp 的 (source_index, side) → BspBits（buy1→Long / sell1→Short，
    // 不重判置位——P1 逐 bit 一致 ⟹ 每个 cand_delta=true 基例必有对应 bit）。
    let mut terminal_by_key: HashMap<(usize, i8), newchan_rust::theta_v0::types::BspBits> =
        HashMap::new();
    if let Some(l0lv) = cls_f.levels.first() {
        for p in l0lv.bsp.iter() {
            if p.bits.buy1 {
                terminal_by_key.entry((p.source_index, 1)).or_insert(p.bits);
            }
            if p.bits.sell1 {
                terminal_by_key
                    .entry((p.source_index, -1))
                    .or_insert(p.bits);
            }
        }
    }
    let n_base = cand_f
        .first()
        .map(|evs| evs.iter().filter(|e| e.cand_delta).count())
        .unwrap_or(0);
    // F-06：terminal 查无以“唯一 L0 基例”计数——此前在每个 top 的装配回调里累加，
    // 同一缺失基例会按可用 top 数重复计入（当前为 0 未显现，首现即膨胀）。
    let cert_missing_terminal = cand_f
        .first()
        .map(|evs| {
            evs.iter()
                .filter(|e| e.cand_delta)
                .filter(|e| !terminal_by_key.contains_key(&(e.confirm_src, side_i8(e.side))))
                .count()
        })
        .unwrap_or(0);
    let (mut old_funnel, old_reachable) = build_cert_funnel(
        &cls_f,
        &cand_f,
        &terminal_by_key,
        ParentWindowMode::EpisodeBaseline,
    );
    let old_baseline_ok = old_funnel
        .get(1)
        .is_some_and(|l| l.reachable_candidates == 3)
        && old_funnel
            .get(2)
            .is_some_and(|l| l.cand_candidates == 13 && l.lag_considered.len() == 22);
    let (mut funnel_levels, reachable) =
        build_cert_funnel(&cls_f, &cand_f, &terminal_by_key, ParentWindowMode::FullCp);
    let mut cert_per_top: Vec<(usize, usize)> = Vec::new(); // 现行 FullCp 目标级证书数
    let mut old_cert_per_top: Vec<(usize, usize)> = Vec::new();
    let mut cert_samples: Vec<String> = Vec::new();
    let mut cert_total = 0usize;
    let mut new_assembler_matches = true;
    for top in 1..cand_f.len() {
        let old_count = count_certificates_for_mode(
            &cand_f,
            &terminal_by_key,
            top,
            ParentWindowMode::EpisodeBaseline,
        );
        old_funnel[top].certificates = old_count;
        old_cert_per_top.push((top, old_count));
        let certs =
            assemble_certificates_terminal(&event_time_cand_f, &objects_by_level, 0, top, |b| {
                terminal_by_key
                    .get(&(b.confirm_src, side_i8(b.side)))
                    .copied()
            });
        let replay_count =
            count_certificates_for_mode(&cand_f, &terminal_by_key, top, ParentWindowMode::FullCp);
        new_assembler_matches &= replay_count == certs.len();
        cert_total += certs.len();
        for c in &certs {
            if cert_samples.len() < 10 {
                let rungs: Vec<String> = c
                    .rungs()
                    .iter()
                    .map(|r| {
                        format!(
                            "confirm={} I(A_child)=[{},{}]⊆D_parent=[{},{}]",
                            r.confirm_src().expect("strict 装配 rung 必携 confirm_src"),
                            r.child_interval()
                                .expect("strict 装配 rung 必携 I(A_child)")
                                .start_time,
                            r.child_interval()
                                .expect("strict 装配 rung 必携 I(A_child)")
                                .end_time,
                            r.interval().start_time,
                            r.interval().end_time
                        )
                    })
                    .collect();
                cert_samples.push(format!(
                    "ℓ={top} side={:?} base(confirm={} I(A)=[{},{}]) rungs(高→低)={}",
                    c.side(),
                    c.base_confirm_src()
                        .expect("strict 装配基例必携 confirm_src"),
                    c.base_interval().start_time,
                    c.base_interval().end_time,
                    rungs.join("⊇")
                ));
            }
        }
        cert_per_top.push((top, certs.len()));
        funnel_levels[top].certificates = certs.len();
    }
    // 裁决：产量不作定义闸门；硬门只检查 terminal 与正式装配/重放镜像一致。
    let l01_certificates = cert_per_top
        .iter()
        .find(|&&(top, _)| top == 1)
        .map_or(0, |&(_, n)| n);
    let l01_regression_expected = l01_certificates >= 3;
    let p2_pass = cert_missing_terminal == 0 && new_assembler_matches;

    // ── 报告 ──
    let mut out = String::new();
    let w = &mut out;
    let _ = writeln!(
        w,
        "# STRICT-NEST-CHECK（P1 逐 bit 校验 + 基线 sanity + E1 三元组复算 + P2 证书装配）"
    );
    let _ = writeln!(w);
    let _ = writeln!(
        w,
        "数据：`{}`，{} bar（{} .. {}）；交易：`{}`，{} 笔。全量因果重放 {:.1}s（P1 确认支逐 bar 因果累积，谓词层终态单次重算定判）。",
        data_path.display(),
        total_bars,
        loaded.first_date,
        loaded.last_date,
        trades_path.display(),
        trades.len(),
        replay_sec
    );
    let _ = writeln!(
        w,
        "参数：`ThetaConfig::default()`（l_max={}, min_parts_per_level={}，未调参）。",
        config.level.l_max, config.level.min_parts_per_level
    );
    let _ = writeln!(w);
    let _ = writeln!(w, "## 基线 sanity（每次必带）");
    let _ = writeln!(w);
    let _ = writeln!(
        w,
        "- D_parent 左端诊断：cand_delta=true 事件中 `enter_src != interval.0` = **{}**（仅诊断）。",
        dparent_enter_mismatch
    );
    let _ = writeln!(w);
    let _ = writeln!(
        w,
        "- raw seen = {}（期望 {}），conf 键 = {}（期望 {}），type3 键 = {}（期望 {}），entry==close = {}/{}（期望 {}/{}），零长度 = {}（期望 {}）→ **{}**",
        hdr_got.0, P7_HDR_EXPECTED.0, hdr_got.1, P7_HDR_EXPECTED.1, hdr_got.2, P7_HDR_EXPECTED.2,
        hdr_got.3, trades.len(), P7_HDR_EXPECTED.3, P7_HDR_EXPECTED.3,
        hdr_got.4, P7_HDR_EXPECTED.4,
        if hdr_ok { "一致" } else { "**不一致（骨架漂移，判读作废）**" }
    );
    let _ = writeln!(w);
    let _ = writeln!(w, "## P1 硬门：谓词输出 ≟ extract_signals buy1/sell1 背驰确认支（checkpoint 逐 bit 全比对 + 终态定判）");
    let _ = writeln!(w);
    let _ = writeln!(
        w,
        "- 校验 bar 数 = {}；checkpoint 全比对 = {} 次；(bar/终态,级) 比较次数 = {}；不一致数 = **{}**。",
        p1.bars_checked, p1.checkpoints, p1.comparisons, p1.mismatch_bars
    );
    let _ = writeln!(
        w,
        "- 因果诊断（非门）：确认撤回 = {}，因果漏捕获 = {}（上游结构修订的两侧锁步行为，由 checkpoint 逐 bit 比对覆盖）；确认支首见滞后 max = {} bar。",
        p1.retracted, p1.uncaptured, p1.max_confirm_delay
    );
    if p1.per_level_mismatch.is_empty() {
        let _ = writeln!(w, "- 每级不一致：无。");
    } else {
        let mut ks: Vec<_> = p1.per_level_mismatch.iter().collect();
        ks.sort();
        for (lvl, n) in ks {
            let _ = writeln!(w, "- ℓ{lvl}：{n} 个 bar 不一致。");
        }
        let _ = writeln!(w, "- 差异样例（前 {}）：", p1.samples.len());
        for s in &p1.samples {
            let _ = writeln!(w, "  - {s}");
        }
    }
    let _ = writeln!(w);
    let _ = writeln!(w, "末 bar 快照（每级：buy1/sell1 bit 数 | 谓词事件数 | cand_delta=true | pan_div_diag=true）：");
    let _ = writeln!(w);
    let _ = writeln!(
        w,
        "| 级别 ℓ | buy1/sell1 bits | Cand 事件 | cand_delta=true | pan_div_diag |"
    );
    let _ = writeln!(w, "|---:|---:|---:|---:|---:|");
    for (lvl, (bits, evs, cd, pd)) in p1.final_snapshot.iter().enumerate() {
        let _ = writeln!(w, "| {lvl} | {bits} | {evs} | {cd} | {pd} |");
    }
    let _ = writeln!(w);
    let p1_pass = p1.pass();
    let _ = writeln!(
        w,
        "**P1 硬门：{}**",
        if p1_pass {
            "PASS（逐 bit 一致）"
        } else {
            "**FAIL（停线：报告差异样例，判据不可调）**"
        }
    );
    let _ = writeln!(w);
    let _ = writeln!(w, "## E1 三元组复算（P2 验收比对表输入）");
    let _ = writeln!(w);
    let _ = writeln!(w, "| 级别 ℓ | t1 首见键（buy1+sell1） | 期望 |");
    let _ = writeln!(w, "|---:|---:|---:|");
    let mut lvls: Vec<usize> = per_level_t1.keys().copied().collect();
    lvls.sort_unstable();
    for lvl in &lvls {
        let exp = T1_PER_LEVEL_EXPECTED
            .iter()
            .find(|&&(l, _)| l == *lvl)
            .map(|&(_, n)| n.to_string())
            .unwrap_or_else(|| "—".to_string());
        let _ = writeln!(w, "| {} | {} | {} |", lvl, per_level_t1[lvl], exp);
    }
    let _ = writeln!(w);
    let _ = writeln!(
        w,
        "- t1 首见键合计 = {}（期望 {}）；主名单 n = {}（期望 {}）；n_B = {}（期望 {}）；n_C = {}（期望 {}）→ **{}**",
        t1_total,
        T1_TOTAL_EXPECTED,
        n_main,
        N_MAIN_EXPECTED,
        n_b,
        N_B_EXPECTED,
        n_c,
        N_C_EXPECTED,
        if triple_ok { "逐项一致" } else { "**不一致**" }
    );
    let _ = writeln!(w);
    let _ = writeln!(
        w,
        "## P2：D_parent 证书生产装配（N^δ_{{ℓ↓0}}，终态 cand_delta=true）"
    );
    let _ = writeln!(w);
    let _ = writeln!(w, "| 目标级 ℓ | 证书数（ℓ↓0 完整链） |");
    let _ = writeln!(w, "|---:|---:|");
    for (top, n) in &cert_per_top {
        let _ = writeln!(w, "| {top} | {n} |");
    }
    let _ = writeln!(w);
    let _ = writeln!(
        w,
        "- 基例（ℓ0 终态 cand_delta=true）= {}；terminal 查无（唯一基例数）= {}（须 0，P1 一致性推论）；证书合计 = **{}**。",
        n_base, cert_missing_terminal, cert_total
    );
    let _ = writeln!(
        w,
        "- 历史 E1 名单 n_C = {}（冻结期望 {}，仅 sanity）；全局 D_parent 证书产量 = {}。L0→L1 证书 = {}，回归预期 ≥3 → **{}（非硬闸门）**。",
        n_c, N_C_EXPECTED, cert_total, l01_certificates,
        if l01_regression_expected { "达到" } else { "未达到" }
    );
    if cert_samples.is_empty() {
        let _ = writeln!(w, "- 证书样例：无（产量 0）。");
    } else {
        let _ = writeln!(w, "- 证书样例（前 {}）：", cert_samples.len());
        for s in &cert_samples {
            let _ = writeln!(w, "  - {s}");
        }
    }
    let _ = writeln!(w);
    let _ = writeln!(
        w,
        "**P2 硬门：{}**",
        if p2_pass {
            "PASS（terminal 对账；产量预期不作硬闸门）"
        } else {
            "**FAIL（terminal 查无）**"
        }
    );
    let _ = writeln!(w);
    let overall = hdr_ok && p1_pass && triple_ok && p2_pass;
    let _ = writeln!(
        w,
        "## 总判：**{}**（sanity {} / P1 {} / E1 三元组 {} / P2 证书 {}）",
        if overall { "PASS" } else { "FAIL" },
        if hdr_ok { "✓" } else { "✗" },
        if p1_pass { "✓" } else { "✗" },
        if triple_ok { "✓" } else { "✗" },
        if p2_pass { "✓" } else { "✗" }
    );

    // ── #43 v2 正式隔离重放报告：只写新产物，不覆盖 2026-07-10 冻结报告。──
    let parent_events: Vec<&CandDeltaEvent> = cand_f
        .iter()
        .enumerate()
        .skip(1)
        .flat_map(|(_, events)| events.iter().filter(|e| e.cand_delta))
        .collect();
    let mut rel_eq = 0usize;
    let mut rel_lt = 0usize;
    let mut rel_gt = 0usize;
    let mut rel_missing = 0usize;
    for event in &parent_events {
        match event.c_structure.map(|c| c.source_start) {
            Some(start) if start == event.c_episode_start => rel_eq += 1,
            Some(start) if start < event.c_episode_start => rel_lt += 1,
            Some(_) => rel_gt += 1,
            None => rel_missing += 1,
        }
    }
    let mapping_ok = parent_events.iter().all(|e| c_start_mapping_ok(e))
        && parent_events.iter().all(|e| full_parent_evidence_ok(e));
    let complete_parent_events = parent_events
        .iter()
        .filter(|e| e.c_interval_full.is_some())
        .count();
    let incomplete_parent_events = parent_events.len() - complete_parent_events;
    let capability_visible = parent_events
        .iter()
        .any(|e| e.b_parent.is_some() || e.c_structure.is_some());

    let mut new_l12_edges: Vec<(usize, usize, PairGateDiag)> = Vec::new();
    if cand_f.len() > 2 && reachable.len() > 1 && old_reachable.len() > 1 {
        for (parent_idx, parent) in cand_f[2].iter().enumerate().filter(|(_, e)| e.cand_delta) {
            for (child_idx, child) in cand_f[1].iter().enumerate() {
                if !reachable[1][child_idx] || !child.cand_delta {
                    continue;
                }
                let Some(new_diag) = diagnose_pair(parent, child, ParentWindowMode::FullCp) else {
                    continue;
                };
                let old_pass = old_reachable[1][child_idx]
                    && diagnose_pair(parent, child, ParentWindowMode::EpisodeBaseline)
                        .is_some_and(PairGateDiag::passes);
                if new_diag.passes() && !old_pass {
                    new_l12_edges.push((parent_idx, child_idx, new_diag));
                }
            }
        }
    }
    let accepted_edges_evidence_ok = new_l12_edges.iter().all(|&(pi, ci, _)| {
        prove_child_structural_ownership(&cand_f[2][pi], &cand_f[1][ci], &tower_f).is_some()
    });

    let prereg = [
        ((352_003, 354_036), 354_036, (340_499, 341_236), 345_518),
        (
            (2_180_264, 2_182_560),
            2_182_560,
            (2_160_411, 2_161_133),
            2_168_349,
        ),
        (
            (2_194_856, 2_197_213),
            2_197_213,
            (2_160_411, 2_161_133),
            2_168_349,
        ),
    ];
    let mut prereg_rows = Vec::new();
    let mut prereg_located = true;
    let mut prediction_falsified = false;
    for (rank, (old_parent, parent_confirm, child_a, child_confirm)) in
        prereg.iter().copied().enumerate()
    {
        let parents: Vec<&CandDeltaEvent> = cand_f
            .get(2)
            .into_iter()
            .flatten()
            .filter(|p| {
                p.cand_delta
                    && p.c_episode_interval == old_parent
                    && p.confirm_src == parent_confirm
            })
            .collect();
        let children: Vec<&CandDeltaEvent> = cand_f
            .get(1)
            .into_iter()
            .flatten()
            .filter(|c| c.cand_delta && c.a_interval == child_a && c.confirm_src == child_confirm)
            .collect();
        if parents.len() != 1 || children.len() != 1 || parents[0].side != children[0].side {
            prereg_located = false;
            prereg_rows.push(format!(
                "| {} | `{:?}` | `{:?}` | **身份重定位失败**（parent={} / child={}） | — | — | — |",
                rank + 1,
                old_parent,
                child_a,
                parents.len(),
                children.len()
            ));
            continue;
        }
        let parent = parents[0];
        let child = children[0];
        let start = parent.c_structure.map(|c| c.source_start);
        let signed = start.map(|s| child.a_interval.0 as i128 - s as i128);
        let diag = diagnose_pair(parent, child, ParentWindowMode::FullCp);
        let passed = diag.is_some_and(PairGateDiag::passes);
        prediction_falsified |= passed;
        let start_text = match (start, parent.c_interval_full) {
            (Some(s), Some(_)) => s.to_string(),
            (Some(s), None) => format!("{s}（右端未闭合，完整父证书拒绝）"),
            (None, _) => "证书未闭合（c_start_full=None）".to_string(),
        };
        let (left, right) = diag
            .map(|d| (d.sub_start.to_string(), d.sub_end.to_string()))
            .unwrap_or_else(|| {
                (
                    "未进入（完整证书拒绝）".to_string(),
                    "未进入（完整证书拒绝）".to_string(),
                )
            });
        let cp = parent.c_structure.map_or_else(
            || "None".to_string(),
            |c| {
                format!(
                    "B={:?}; c=L{}#{}..{}; third={:?}",
                    parent.b_parent.map(|b| b.center_id),
                    c.departure_move_id.level,
                    c.departure_move_id.ordinal,
                    c.terminal_move_id
                        .map_or_else(|| "None".to_string(), |id| id.ordinal.to_string()),
                    parent
                        .third_class_in_c
                        .map(|t| (t.departure_move_id, t.retest_move_id))
                )
            },
        );
        prereg_rows.push(format!(
            "| {} | `{:?}` | `{:?}` | {} | {} | {} / {} | **{}**；{} |",
            rank + 1,
            old_parent,
            child_a,
            start_text,
            signed.map_or_else(|| "—".to_string(), |d| format!("{d:+}")),
            left,
            right,
            if passed { "通过" } else { "不通过" },
            cp
        ));
    }
    let prediction_status = if prediction_falsified {
        "FALSIFIED"
    } else if prereg_located {
        "SUPPORTED-ON-THIS-REPLAY"
    } else {
        "UNRESOLVED-EVIDENCE"
    };

    let core_ok = overall && old_baseline_ok;
    let final_verdict = if !capability_visible {
        "BLOCKED-CAPABILITY"
    } else if !mapping_ok {
        "FAIL-DEFINITION-MAPPING"
    } else if !core_ok || !prereg_located || !accepted_edges_evidence_ok {
        "FAIL-EVIDENCE"
    } else {
        "PASS"
    };

    let mut replay_out = String::new();
    let rw = &mut replay_out;
    let _ = writeln!(
        rw,
        "# P43 正式隔离重放 v2：D_parent = c_interval_full（2026-07-12）"
    );
    let _ = writeln!(rw);
    let _ = writeln!(rw, "- 最终判定：**{final_verdict}**");
    let _ = writeln!(rw, "- 权威：`dparent-leftend-p0-review-20260711.md` §7；能力基线：`p45-cp-capability-20260712.md`。");
    let _ = writeln!(rw, "- 输入：`{}`，{} bar（{} .. {}），{} trades；`ThetaConfig::default()`（l_max={} / min_parts_per_level={}）；全量因果重放 {:.1}s。", data_path.display(), total_bars, loaded.first_date, loaded.last_date, trades.len(), config.level.l_max, config.level.min_parts_per_level, replay_sec);
    let _ = writeln!(rw, "- 唯一语义变量：`D_parent` 由消费者显式选择 `d_parent_interval_snapshot` 或 `d_parent_interval_terminal`；本报告正式装配使用 terminal 对象证书。`D_child := child.a_interval` 保持 **provisional / pending separate ruling**。方向、`cand_delta`、右端证明规则、`confirm_src/lag_conf/epsilon_conf` 均未改，后三者仅诊断。");
    let _ = writeln!(rw, "- 写入边界：本次只落盘本报告；未回写 2026-07-10 漏斗、冻结或裁决文档，未修改 `departure_move_c_start`。");

    let _ = writeln!(rw, "\n## 1. 重放与旧基线门");
    let _ = writeln!(
        rw,
        "- sanity / P1 / E1 / terminal / 装配镜像：{} / {} / {} / {} / {}。",
        hdr_ok,
        p1_pass,
        triple_ok,
        cert_missing_terminal == 0,
        new_assembler_matches
    );
    let _ = writeln!(rw, "- 旧基线复核：L1 partial chain={}（期望 3），L2 cand_delta=true={}（期望 13），L1→L2 同向可达候选对={}（期望 22）→ **{}**。", old_funnel.get(1).map_or(0, |l| l.reachable_candidates), old_funnel.get(2).map_or(0, |l| l.cand_candidates), old_funnel.get(2).map_or(0, |l| l.lag_considered.len()), if old_baseline_ok { "无漂移，允许横比" } else { "漂移，停止横比" });

    let _ = writeln!(rw, "\n## 2. 每个父事件的完整 c / episode 三元组与结构证据");
    let _ = writeln!(rw, "`关系` 比较 `c_start_full` 与 `c_episode_start`。`c_end_full=None` 的事件被正式装配拒绝并计数；绝不回填 episode。");
    let _ = writeln!(rw, "\n| L | side / confirm | B_p（ID / source） | c_start_full | c_episode_start | c_end_full | 关系 | c_p 首尾递归 ID | 第三类（leave / retest / source） | A_left / 未扩张检查 |");
    let _ = writeln!(rw, "|---:|---|---|---:|---:|---|:---:|---|---|---|");
    for event in &parent_events {
        let b = event.b_parent.map_or_else(
            || "None".to_string(),
            |b| {
                format!(
                    "L{}#{} / {:?}",
                    b.center_id.level, b.center_id.ordinal, b.source_interval
                )
            },
        );
        let start = event.c_structure.map(|c| c.source_start);
        let end = event.c_interval_full.map(|iv| iv.1);
        let relation = start.map_or("?", |s| {
            if s == event.c_episode_start {
                "=="
            } else if s < event.c_episode_start {
                "<"
            } else {
                ">"
            }
        });
        let cp = event.c_structure.map_or_else(
            || "None".to_string(),
            |c| {
                format!(
                    "L{}#{} .. {}",
                    c.departure_move_id.level,
                    c.departure_move_id.ordinal,
                    c.terminal_move_id.map_or_else(
                        || "None".to_string(),
                        |id| format!("L{}#{}", id.level, id.ordinal)
                    )
                )
            },
        );
        let third = event.third_class_in_c.map_or_else(
            || "None".to_string(),
            |t| {
                format!(
                    "L{}#{} / L{}#{} / {:?}+{:?}",
                    t.departure_move_id.level,
                    t.departure_move_id.ordinal,
                    t.retest_move_id.level,
                    t.retest_move_id.ordinal,
                    t.departure_interval,
                    t.retest_interval
                )
            },
        );
        let _ = writeln!(
            rw,
            "| {} | `{:?} / {}` | `{}` | {} | {} | {} | {} | `{}` | `{}` | {} / {} |",
            event.level,
            event.side,
            event.confirm_src,
            b,
            start.map_or_else(|| "None".to_string(), |v| v.to_string()),
            event.c_episode_start,
            end.map_or_else(|| "None（拒绝）".to_string(), |v| v.to_string()),
            relation,
            cp,
            third,
            event.a_interval.0,
            c_start_mapping_ok(event)
        );
    }
    let _ = writeln!(rw, "\n分布 `== / < / > / c_start 缺失` = **{rel_eq} / {rel_lt} / {rel_gt} / {rel_missing}**。若 `>` 非零，其表中 `B_p -> departure_move_id` 说明被排除前缀归属于 B 而非完整 c；本次不以 episode 值替代。所有可表达左端均来自扫描侧车的 non-extension 离开单元；`mapping_ok={mapping_ok}`，证明未读取父 A 起点或整趋势起点。");
    let _ = writeln!(
        rw,
        "父事件总数 = **{}**；完整 `c_interval_full` = **{}**；未闭合并被拒绝 = **{}**。",
        parent_events.len(),
        complete_parent_events,
        incomplete_parent_events
    );

    let _ = writeln!(rw, "\n## 3. §7.B 新旧漏斗");
    if old_baseline_ok {
        let _ = writeln!(rw, "| L | Cand 候选旧→新 | 相邻边旧→新（差） | 可达 Cand 旧→新（差） | 完整父事件 / 未闭合拒绝 | 完整链证书旧→新（差） |");
        let _ = writeln!(rw, "|---:|---:|---:|---:|---:|---:|");
        for level in 0..funnel_levels.len() {
            let old = &old_funnel[level];
            let new = &funnel_levels[level];
            let old_cert = if level == 0 {
                old.reachable_candidates
            } else {
                old.certificates
            };
            let new_cert = if level == 0 {
                new.reachable_candidates
            } else {
                new.certificates
            };
            let _ = writeln!(rw, "| {level} | {}→{} | {}→{} ({:+}) | {}→{} ({:+}) | {} / {}（pair拒绝={}） | {}→{} ({:+}) |", old.cand_candidates, new.cand_candidates, old.reachable_pair_successes, new.reachable_pair_successes, new.reachable_pair_successes as i128 - old.reachable_pair_successes as i128, old.reachable_candidates, new.reachable_candidates, new.reachable_candidates as i128 - old.reachable_candidates as i128, new.full_parent_events, new.incomplete_parent_events, new.incomplete_pair_rejections, old_cert, new_cert, new_cert as i128 - old_cert as i128);
        }
    } else {
        let _ = writeln!(
            rw,
            "旧基线漂移，按裁决停止新旧横比；仅保留上节事件级原始证据。"
        );
    }
    let _ = writeln!(rw, "\n确认延迟分布仍只诊断：");
    for level in 1..funnel_levels.len() {
        let _ = writeln!(
            rw,
            "- L{}→L{} considered `{}`；accepted `{}`。",
            level - 1,
            level,
            lag_distribution(&funnel_levels[level].lag_considered),
            lag_distribution(&funnel_levels[level].lag_accepted)
        );
    }

    let _ = writeln!(rw, "\n### 所有新增 L1→L2 边");
    if new_l12_edges.is_empty() {
        let _ = writeln!(rw, "无新增边。");
    } else {
        let _ = writeln!(rw, "| parent B | c_interval_full | c_episode_interval | child.a_interval | 方向 | Sub 左 / 右 | 递归结构归属证明 |");
        let _ = writeln!(rw, "|---|---|---|---|---|---|---|");
        for &(pi, ci, diag) in &new_l12_edges {
            let parent = &cand_f[2][pi];
            let child = &cand_f[1][ci];
            let proof = prove_child_structural_ownership(parent, child, &tower_f);
            let b = parent.b_parent.map_or_else(
                || "None".to_string(),
                |b| {
                    format!(
                        "L{}#{} {:?}",
                        b.center_id.level, b.center_id.ordinal, b.source_interval
                    )
                },
            );
            let proof_text = proof.map_or_else(|| "**MISSING**".to_string(), |p| format!("parent L{}#{}..L{}#{} 的 sub_moves 包含 child L{}#{}..L{}#{}，结构 span={:?}", p.parent_first.level, p.parent_first.ordinal, p.parent_last.level, p.parent_last.ordinal, p.child_first.level, p.child_first.ordinal, p.child_last.level, p.child_last.ordinal, p.child_span));
            let _ = writeln!(
                rw,
                "| `{}` | `{:?}` | `{:?}` | `{:?}` | `{:?} / {:?}` | {} / {} | {} |",
                b,
                parent.c_interval_full,
                parent.c_episode_interval,
                child.a_interval,
                parent.side,
                child.side,
                diag.sub_start,
                diag.sub_end,
                proof_text
            );
        }
    }

    let _ = writeln!(rw, "\n## 4. §7.C 三个预注册近失样本（按身份重定位）");
    let _ = writeln!(rw, "| # | 旧父窗 | child.a_interval（provisional） | 新 c_start_full / 闭合状态 | child.left - c_start_full | Sub 左 / 右 | 最终结果与 c_p 分解 |");
    let _ = writeln!(rw, "|---:|---|---|---|---:|---|---|");
    for row in prereg_rows {
        let _ = writeln!(rw, "{row}");
    }
    let _ = writeln!(
        rw,
        "\n`P0-43-L1L2-CFULL`：**{prediction_status}**。{}",
        if prediction_falsified {
            "至少一例合法通过；预测被证伪不自动推翻 Q1。"
        } else if prereg_located {
            "三例在本次同数据同参数重放均未通过；不外推到其他数据、参数或品种。"
        } else {
            "至少一个预注册事件身份未能唯一重定位；预测保持未裁定。"
        }
    );

    let _ = writeln!(rw, "\n## 5. 裁决 §7 清单逐项勾选");
    let mark = |ok: bool| if ok { "x" } else { " " };
    let _ = writeln!(rw, "\n### A. 定义 / 字段");
    let _ = writeln!(rw, "- [{}] 唯一变量为 D_parent.left 切到 c_start_full；D_child/方向/cand_delta/确认诊断门不变。", mark(mapping_ok));
    let _ = writeln!(
        rw,
        "- [x] 每个父事件均输出 c_start_full / c_episode_start / c_end_full（None 明示拒绝）。"
    );
    let _ = writeln!(
        rw,
        "- [{}] 每个完整事件附 B_p、c_p 首尾及第三类；完整证书内部一致。",
        mark(mapping_ok)
    );
    let _ = writeln!(
        rw,
        "- [x] 已统计 == / < / > / 缺失；> 由结构归属解释且未静默接受。"
    );
    let _ = writeln!(
        rw,
        "- [{}] D_parent 未扩到父 A 或整趋势起点。",
        mark(mapping_ok)
    );
    let _ = writeln!(rw, "- [x] confirm_src / lag_conf / epsilon_conf 仅诊断。");
    let _ = writeln!(rw, "\n### B. 漏斗");
    let _ = writeln!(
        rw,
        "- [{}] 旧基线 3 / 13 / 22 复核；漂移时停止横比。",
        mark(old_baseline_ok)
    );
    let _ = writeln!(
        rw,
        "- [{}] 已输出各级新旧候选、边、完整父事件、完整链证书及差分。",
        mark(old_baseline_ok)
    );
    let _ = writeln!(
        rw,
        "- [x] 所有新增 L1→L2 边均逐条打印要求字段（空集亦明示）。"
    );
    let _ = writeln!(
        rw,
        "- [{}] 每条新增边均有独立递归 sub_moves 归属证明。",
        mark(accepted_edges_evidence_ok)
    );
    let _ = writeln!(rw, "\n### C. 预注册样本");
    let _ = writeln!(
        rw,
        "- [{}] 三例按原 parent confirm/episode 与 child confirm/a_interval 身份唯一重定位。",
        mark(prereg_located)
    );
    let _ = writeln!(
        rw,
        "- [{}] 已报告新左端或未闭合、有符号差、Sub 左右界。",
        mark(prereg_located)
    );
    let _ = writeln!(
        rw,
        "- [{}] 任一通过则标 FALSIFIED 并附 c_p 分解。",
        mark(!prediction_falsified || prereg_located)
    );
    let _ = writeln!(
        rw,
        "- [{}] 三例全不通过则仅标 SUPPORTED-ON-THIS-REPLAY。",
        mark(prediction_falsified || prediction_status == "SUPPORTED-ON-THIS-REPLAY")
    );
    let _ = writeln!(rw, "\n### D. 最终判定");
    for verdict in [
        "PASS",
        "FAIL-DEFINITION-MAPPING",
        "FAIL-EVIDENCE",
        "BLOCKED-CAPABILITY",
    ] {
        let _ = writeln!(rw, "- [{}] `{verdict}`", mark(final_verdict == verdict));
    }

    let replay_path = out_root.join("chanlun/review-results/p43-replay-cfull-v2-20260712.md");
    std::fs::write(&replay_path, &replay_out)
        .map_err(|e| format!("写入 {} 失败: {e}", replay_path.display()))?;
    println!("{out}");
    eprintln!("#43 v2 报告：{}", replay_path.display());
    Ok(final_verdict == "PASS")
}

#[cfg(test)]
mod funnel_tests {
    use super::*;

    fn event(
        level: u32,
        side: Side,
        confirm_src: usize,
        interval: (usize, usize),
    ) -> CandDeltaEvent {
        CandDeltaEvent {
            level,
            side,
            divergence_confirm_src: confirm_src,
            confirm_src,
            interval,
            a_interval: interval,
            c_episode_start: interval.0,
            c_episode_interval: interval,
            c_interval_full: None,
            b_parent: None,
            c_structure: None,
            third_class_in_c: None,
            cp_certificate_confirm_src: None,
            full_trend_c_qualified: None,
            full_trend_evidence: None,
            cp_ownership: None,
            enter_src: interval.0,
            cand_delta: true,
            pan_div_diag: false,
        }
    }

    #[test]
    fn pair_diag_is_exact_three_gate_conjunction() {
        let child = event(0, Side::Long, 90, (40, 80));
        let parent = event(1, Side::Long, 70, (20, 100));
        let diag = diagnose_pair(&parent, &child, ParentWindowMode::EpisodeBaseline).unwrap();
        assert!(diag.passes());
        assert_eq!(diag.failed_conditions(), 0);
        assert_eq!(
            pair_gap_bars(&parent, &child, ParentWindowMode::EpisodeBaseline),
            Some((0, 0))
        );
    }

    #[test]
    fn pair_diag_reports_each_numeric_gap_without_relaxing_boundary() {
        let child = event(0, Side::Long, 80, (10, 120));
        let parent = event(1, Side::Long, 90, (20, 100));
        let diag = diagnose_pair(&parent, &child, ParentWindowMode::EpisodeBaseline).unwrap();
        assert!(diag.same_side);
        assert!(!diag.sub_start);
        assert!(!diag.sub_end);
        assert_eq!(
            pair_gap_bars(&parent, &child, ParentWindowMode::EpisodeBaseline),
            Some((10, 20))
        );
    }

    #[test]
    fn pair_diag_keeps_closed_sub_boundaries_inclusive() {
        let child = event(0, Side::Short, 100, (20, 100));
        let parent = event(1, Side::Short, 100, (20, 100));
        assert!(
            diagnose_pair(&parent, &child, ParentWindowMode::EpisodeBaseline)
                .unwrap()
                .passes()
        );
    }

    #[test]
    fn dparent_pair_diag_uses_child_a_and_never_gates_confirm_lag() {
        let mut child = event(0, Side::Long, 200, (0, 300));
        child.a_interval = (40, 60);
        let parent = event(1, Side::Long, 250, (20, 80));
        let diag = diagnose_pair(&parent, &child, ParentWindowMode::EpisodeBaseline).unwrap();
        assert!(diag.passes(), "I(A_child)⊆D_parent；完成时顺序不得进入门");
        assert_eq!(
            confirmation_lag(&parent, &child, ParentWindowMode::EpisodeBaseline),
            Some(120)
        );
    }

    #[test]
    fn full_cp_pair_diag_rejects_unclosed_parent_without_episode_fallback() {
        let child = event(0, Side::Long, 90, (40, 80));
        let parent = event(1, Side::Long, 70, (20, 100));
        assert!(diagnose_pair(&parent, &child, ParentWindowMode::FullCp).is_none());
    }

    #[test]
    fn lag_distribution_preserves_signed_values() {
        assert_eq!(
            lag_distribution(&[-2, 0, 5, 9]),
            "n=4；负/零/正=1/1/2；min/p25/p50/p75/p90/p95/max=-2/-2/0/5/5/5/9"
        );
    }

    #[test]
    fn dparent_enter_invariant_is_checked_in_release_code() {
        let good = event(0, Side::Long, 80, (20, 80));
        let mut bad = event(1, Side::Long, 100, (30, 100));
        bad.enter_src = 29;
        assert_eq!(dparent_enter_mismatches(&[vec![good], vec![bad]]), 1);
    }
    /// F-07：归零漏斗须覆盖 L0 terminal 全查无、候选空集、配对归零、终门归零与全通。
    #[test]
    fn f07_first_zero_cause_covers_all_funnel_stages() {
        fn fl(cand: usize, pair: usize, reach: usize, cert: usize) -> FunnelLevel {
            FunnelLevel {
                cand_candidates: cand,
                reachable_pair_successes: pair,
                reachable_candidates: reach,
                certificates: cert,
                ..Default::default()
            }
        }
        // L0 有基例但 terminal 全查无
        assert_eq!(
            first_zero_cause(&[fl(3, 0, 0, 0), fl(2, 0, 0, 0)]),
            Some(FirstZeroCause::L0TerminalAllMissing)
        );
        // 高级别候选空集（旧实现返回 None，只给模糊提示）
        assert_eq!(
            first_zero_cause(&[fl(3, 0, 3, 0), fl(0, 0, 0, 0)]),
            Some(FirstZeroCause::CandEmpty(1))
        );
        // 原有配对归零情形保持不变
        assert_eq!(
            first_zero_cause(&[fl(3, 0, 3, 0), fl(2, 0, 0, 0)]),
            Some(FirstZeroCause::PairZero(1))
        );
        // 配对成功但终门归零（旧实现返回 None）
        assert_eq!(
            first_zero_cause(&[fl(3, 0, 3, 0), fl(2, 1, 1, 0)]),
            Some(FirstZeroCause::CertificateZero(1))
        );
        // 全通 / 结构层为空均无断点
        assert_eq!(first_zero_cause(&[fl(3, 0, 3, 0), fl(2, 1, 1, 1)]), None);
        assert_eq!(first_zero_cause(&[fl(0, 0, 0, 0), fl(0, 0, 0, 0)]), None);
    }

    /// B/cert F-05：JSON 数组必须遵守逗号语法，空白或双逗号不能被当成合法分隔符吞掉。
    #[test]
    fn f05_rejects_missing_and_duplicate_array_commas() {
        for (name, malformed) in [("missing", "[1 2]"), ("duplicate", "[1,,2]")] {
            let path = std::env::temp_dir().join(format!(
                "strict_nest_f05_{name}_{}.json",
                std::process::id()
            ));
            let json = format!(
                "{{\"opens\":{malformed},\"highs\":[2,2],\"lows\":[1,1],\"closes\":[1,1],\"volumes\":[1,1],\"dates\":[\"2026-01-01\",\"2026-01-02\"]}}"
            );
            std::fs::write(&path, json).unwrap();
            assert!(
                load_btc_bars(&path, 1.0).is_err(),
                "非法数组 {malformed} 必须拒绝"
            );
            let _ = std::fs::remove_file(path);
        }
    }
}

// ═══════════════════════ 数据加载（e1 逐行复制） ═══════════════════════

fn load_btc_bars(path: &Path, tick_size: f64) -> Result<LoadedBars, String> {
    let text =
        std::fs::read_to_string(path).map_err(|e| format!("读取 {} 失败: {e}", path.display()))?;
    let raw: BarsJson = serde_json::from_str(&text)
        .map_err(|e| format!("{}: JSON 解析失败: {e}", path.display()))?;
    let BarsJson {
        opens,
        highs,
        lows,
        closes,
        volumes,
        dates,
    } = raw;
    let timestamps: Vec<Timestamp> = dates.iter().map(|date| date_to_timestamp(date)).collect();
    let first_date = dates.first().cloned().unwrap_or_default();
    let last_date = dates.last().cloned().unwrap_or_default();

    let n = closes.len();
    for (name, len) in [
        ("opens", opens.len()),
        ("highs", highs.len()),
        ("lows", lows.len()),
        ("volumes", volumes.len()),
        ("dates", timestamps.len()),
    ] {
        if len != n {
            return Err(format!("列长度不一致：{name}={len} vs closes={n}"));
        }
    }

    let mut bars = Vec::with_capacity(n);
    for i in 0..n {
        let o = opens[i];
        let h = highs[i];
        let l = lows[i];
        let c = closes[i];
        let v = volumes[i];
        let bad_range = h < o.max(c).max(l) || l > o.min(c).min(h);
        let bad_price = o <= 0.0 || h <= 0.0 || l <= 0.0 || c <= 0.0;
        bars.push(Bar {
            source_index: i,
            timestamp: timestamps[i],
            open: quantize(o, tick_size),
            high: quantize(h, tick_size),
            low: quantize(l, tick_size),
            close: quantize(c, tick_size),
            volume: v as i64,
            untradable: bad_range || bad_price || v <= 0.0,
        });
    }

    Ok(LoadedBars {
        bars,
        first_date,
        last_date,
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
    for ch in date.chars().take_while(|c| *c != '+') {
        if ch.is_ascii_digit() {
            digits.push(ch);
            if digits.len() == 14 {
                break;
            }
        }
    }
    digits.parse::<i64>().unwrap_or(0)
}
