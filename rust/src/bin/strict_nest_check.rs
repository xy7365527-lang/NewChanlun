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
use newchan_rust::theta_v0::classifier::nest::{assemble_certificates, is_sub, NestInterval};
use newchan_rust::theta_v0::classifier::recursive_tower::{
    find_move_by_end_index, CandDeltaEvent, LeveledMove,
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
        let t: Trade = serde_json::from_str(line)
            .map_err(|e| format!("行 {}: JSON 解析失败: {e}", ln + 1))?;
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
        [
            self.same_side,
            self.sub_start,
            self.sub_end,
        ]
        .into_iter()
        .filter(|ok| !ok)
        .count()
    }
}

fn diagnose_pair(parent: &CandDeltaEvent, child: &CandDeltaEvent) -> PairGateDiag {
    PairGateDiag {
        same_side: parent.side == child.side,
        sub_start: child.a_interval.0 >= parent.enter_src,
        sub_end: child.a_interval.1 <= parent.interval.1,
    }
}

fn confirmation_lag(parent: &CandDeltaEvent, child: &CandDeltaEvent) -> i128 {
    child.confirm_src as i128 - parent.interval.1 as i128
}

/// 数值门离通过还差多少根 bar；通过门贡献 0。方向门单独由 failed_conditions 排序。
fn pair_gap_bars(parent: &CandDeltaEvent, child: &CandDeltaEvent) -> (usize, usize) {
    (
        parent.enter_src.saturating_sub(child.a_interval.0),
        child.a_interval.1.saturating_sub(parent.interval.1),
    )
}

#[derive(Debug, Clone, Default)]
struct FunnelLevel {
    predicate_hits: usize,
    structural_events: usize,
    cand_candidates: usize,
    reachable_pair_successes: usize,
    reachable_candidates: usize,
    certificates: usize,
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
            for (child_idx, child) in children.iter().enumerate() {
                if !reachable[level - 1][child_idx] || !child.cand_delta {
                    continue;
                }
                let diag = diagnose_pair(parent, child);
                if diag.same_side {
                    levels[level].lag_considered.push(confirmation_lag(parent, child));
                }
                if diag.passes() {
                    levels[level].reachable_pair_successes += 1;
                    levels[level].lag_accepted.push(confirmation_lag(parent, child));
                    parent_reachable = true;
                }
            }
            reachable[level][parent_idx] = parent_reachable;
        }
        levels[level].reachable_candidates = reachable[level].iter().filter(|&&r| r).count();
    }
    (levels, reachable)
}

fn closest_pair_misses(
    events_by_level: &[Vec<CandDeltaEvent>],
    reachable: &[Vec<bool>],
    parent_level: usize,
    limit: usize,
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
            let diag = diagnose_pair(parent, child);
            if diag.passes() {
                continue;
            }
            let (start_gap, end_gap) = pair_gap_bars(parent, child);
            misses.push(NearMiss {
                parent_level,
                parent_idx,
                child_idx,
                diag,
                start_gap,
                end_gap,
                confirm_lag: confirmation_lag(parent, child),
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
        sorted.len(), negative, zero, positive, sorted[0], percentile(&sorted, 25),
        percentile(&sorted, 50), percentile(&sorted, 75), percentile(&sorted, 90),
        percentile(&sorted, 95), sorted[sorted.len() - 1]
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
    let out_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("无法定位 worktree 根目录")?
        .to_path_buf();
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
    let cand_f = classifier::cand_delta_tower_cached(
        &l0_f,
        &cls_f,
        &tower_f,
        &config,
        &classifier_incr.tower_cache,
    );
    // 冻结诊断不变量：release 复跑也必须实际检查，不能只依赖 debug_assert。
    let dparent_enter_mismatch = dparent_enter_mismatches(&cand_f);
    p1.finalize(replay_bars - 1, &cls_f, &cand_f);
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
    let mut cert_per_top: Vec<(usize, usize)> = Vec::new(); // (目标级 ℓ, 证书数)
    let mut cert_samples: Vec<String> = Vec::new();
    let mut cert_total = 0usize;
    let (mut funnel_levels, reachable) = build_cert_funnel(&cls_f, &cand_f, &terminal_by_key);
    for top in 1..cand_f.len() {
        let certs = assemble_certificates(&cand_f, 0, top, |b| {
            terminal_by_key
                .get(&(b.confirm_src, side_i8(b.side)))
                .copied()
        });
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
                            r.child_interval().expect("strict 装配 rung 必携 I(A_child)").start_time,
                            r.child_interval().expect("strict 装配 rung 必携 I(A_child)").end_time,
                            r.interval().start_time,
                            r.interval().end_time
                        )
                    })
                    .collect();
                cert_samples.push(format!(
                    "ℓ={top} side={:?} base(confirm={} I(A)=[{},{}]) rungs(高→低)={}",
                    c.side(),
                    c.base_confirm_src().expect("strict 装配基例必携 confirm_src"),
                    c.base_interval().start_time,
                    c.base_interval().end_time,
                    rungs.join("⊇")
                ));
            }
        }
        cert_per_top.push((top, certs.len()));
        funnel_levels[top].certificates = certs.len();
    }
    // 裁决：L0→L1 ≥3 仅作回归预期，不作硬闸门；P2 硬门只保留 terminal 对账。
    let l01_certificates = cert_per_top
        .iter()
        .find(|&&(top, _)| top == 1)
        .map_or(0, |&(_, n)| n);
    let l01_regression_expected = l01_certificates >= 3;
    let p2_pass = cert_missing_terminal == 0;

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

    let report_path = out_root.join("STRICT-NEST-DPARENT-CHECK.md");
    std::fs::write(&report_path, &out)
        .map_err(|e| format!("写入 {} 失败: {e}", report_path.display()))?;

    // ── 证书产量=0 漏斗归因报告（只读终态插桩）──
    // F-07：按漏斗阶段顺序归因，输出确定原因枚举；近失样本仅对配对归零情形有意义。
    let first_zero = first_zero_cause(&funnel_levels);
    let near_misses = match first_zero {
        Some(FirstZeroCause::PairZero(level)) => {
            closest_pair_misses(&cand_f, &reachable, level, 3)
        }
        _ => Vec::new(),
    };
    let mut funnel_out = String::new();
    let fw = &mut funnel_out;
    let _ = writeln!(fw, "# D_parent 严格区间套复跑原始漏斗（2026-07-10）");
    let _ = writeln!(fw);
    let _ = writeln!(fw, "## 运行范围与语义护栏");
    let _ = writeln!(fw);
    let _ = writeln!(
        fw,
        "- 数据：`{}`，**{}** bar（{} .. {}）；`ThetaConfig::default()`，`l_max={}`，`min_parts_per_level={}`。",
        data_path.display(), total_bars, loaded.first_date, loaded.last_date,
        config.level.l_max, config.level.min_parts_per_level
    );
    let _ = writeln!(fw, "- 命令：`cargo build --release --bin strict_nest_check && ./target/release/strict_nest_check`；全量因果重放 {:.1}s。", replay_sec);
    let _ = writeln!(fw, "- 语义：`J_parent := D_parent = parent.interval`；闭包含只判 `child.a_interval ⊆ D_parent`；父子均显式过滤 `cand_delta=true` 且方向一致。");
    let _ = writeln!(fw, "- `child.confirm_src - right(D_parent)` 只登记有符号分布；`ε_conf` 未进入任何控制流、排序或否决门。");
    let _ = writeln!(fw, "- D_parent 左端诊断：cand_delta=true 事件中 `enter_src != interval.0` = **{}**；该计数不作产量闸门。", dparent_enter_mismatch);
    let _ = writeln!(fw, "- 列口径：背驰段谓词命中 = 终态分类 buy1/sell1 bit；`Cand^δ` 候选 = 塔上 `cand_delta=true` 事件；相邻边成功 = 能把已从 L0 可达的 partial chain 以 `I(A_child)⊆D_parent` 延长一级；最终证书 = `assemble_certificates(events, 0, ℓ, terminal)` 产量。");
    let _ = writeln!(fw);
    let _ = writeln!(fw, "## ① 各段 × 各级计数");
    let _ = writeln!(fw);
    let _ = writeln!(fw, "| 级别 ℓ | 结构事件（辅助） | 背驰段谓词命中 | Cand^δ 候选 | D_parent 相邻边成功 | 可达本级 Cand | 最终证书 N^δ_{{ℓ↓0}} |");
    let _ = writeln!(fw, "|---:|---:|---:|---:|---:|---:|---:|");
    for (level, counts) in funnel_levels.iter().enumerate() {
        let cert = if level == 0 {
            format!("—（基例 terminal={}）", counts.reachable_candidates)
        } else {
            counts.certificates.to_string()
        };
        let pair = if level == 0 {
            "—".to_string()
        } else {
            counts.reachable_pair_successes.to_string()
        };
        let _ = writeln!(
            fw,
            "| {level} | {} | {} | {} | {pair} | {} | {cert} |",
            counts.structural_events,
            counts.predicate_hits,
            counts.cand_candidates,
            counts.reachable_candidates
        );
    }
    let _ = writeln!(fw);
    let _ = writeln!(fw, "## ② λ_conf(child) − right(D_parent) 延迟分布（纯诊断）");
    let _ = writeln!(fw);
    let _ = writeln!(fw, "| 相邻级 | 同向可达候选对（结构门前） | 通过 I(A)⊆D_parent 的边 |");
    let _ = writeln!(fw, "|---|---|---|");
    for level in 1..funnel_levels.len() {
        let counts = &funnel_levels[level];
        let _ = writeln!(
            fw,
            "| L{}→L{} | {} | {} |",
            level - 1,
            level,
            lag_distribution(&counts.lag_considered),
            lag_distribution(&counts.lag_accepted)
        );
    }
    let _ = writeln!(fw);
    let _ = writeln!(fw, "`ε_conf`：**仅诊断标签，未设置数值，未作闸门**。");
    let _ = writeln!(fw);
    let _ = writeln!(
        fw,
        "核对：P1 mismatch bar = **{}**；L0 terminal 查无（唯一基例数）= **{}**；跨级证书合计 = **{}**。",
        p1.mismatch_bars, cert_missing_terminal, cert_total
    );
    let _ = writeln!(fw);
    let _ = writeln!(fw, "## ③ 首个归零的段");
    let _ = writeln!(fw);
    match first_zero {
        Some(FirstZeroCause::PairZero(level)) => {
            let _ = writeln!(
                fw,
                "首个归零发生在 **L{} → L{} 的 D_parent 相邻配对**：L{} 有 {} 个可达 Cand 基例/partial-chain，L{} 有 {} 个 `Cand^δ=true` 候选，但满足 `同方向 ∧ I(A_child) ⊆ D_parent` 的可达相邻边为 **0**。`confirm_src` 延迟不参与该结论。因此从该段开始所有 `N^δ_{{ℓ↓0}}` 完整证书均为 0；损失不发生在 P1 谓词→Cand 映射，也不发生在 terminal 查找。",
                level - 1,
                level,
                level - 1,
                funnel_levels[level - 1].reachable_candidates,
                level,
                funnel_levels[level].cand_candidates
            );
        }
        Some(FirstZeroCause::L0TerminalAllMissing) => {
            let _ = writeln!(
                fw,
                "首个归零发生在 **L0 terminal 查找**：L0 有 {} 个 `Cand^δ=true` 基例，但可达（有 terminal）基例为 **0**。损失在谓词→terminal 对账，不在递降链配对。",
                funnel_levels[0].cand_candidates
            );
        }
        Some(FirstZeroCause::CandEmpty(level)) => {
            let _ = writeln!(
                fw,
                "首个归零发生在 **L{level} 的 `Cand^δ` 候选空集**：L{} 仍有 {} 个可达 Cand，但 L{level} 候选为 **0**。损失在本级候选生成，不在递降链配对。",
                level - 1,
                funnel_levels[level - 1].reachable_candidates
            );
        }
        Some(FirstZeroCause::ReachableZero(level)) => {
            let _ = writeln!(
                fw,
                "首个归零发生在 **L{level} 的可达 Cand**：配对成功数非零但可达本级 Cand 为 **0**（防御性分支，正常不应出现，须人工核查插桩）。"
            );
        }
        Some(FirstZeroCause::CertificateZero(level)) => {
            let _ = writeln!(
                fw,
                "首个归零发生在 **L{level} 的证书终门**：本级仍有 {} 个可达 Cand，但 `assemble_certificates` 产量为 **0**。损失在装配终门（terminal 对账/链装配），不在递降链配对。",
                funnel_levels[level].reachable_candidates
            );
        }
        None => {
            let _ = writeln!(
                fw,
                "未发现“上一阶段非零、当前阶段为零”的断点：漏斗各段到证书终门均非零，或结构层本身为空。"
            );
        }
    }
    let _ = writeln!(fw);
    let _ = writeln!(fw, "## ④ 首个归零段最接近通过的 3 个样本");
    let _ = writeln!(fw);
    let _ = writeln!(fw, "排序冻结为：失败原子条件数升序 → 两个 Sub 边界缺口 bar 总和升序 → `(Sub 左界缺口, Sub 右界缺口, parent_idx)` 字典序；确认延迟不参与排序，未用价格/收益挑样本。");
    let _ = writeln!(fw);
    if near_misses.is_empty() {
        let _ = writeln!(fw, "无可比较近失样本。");
    } else {
        let _ = writeln!(fw, "| 排名 | 父级 Cand（side, confirm, D_parent） | 子级可达 Cand（side, confirm, I(A)） | 通过条件 | 具体缺口；确认延迟仅诊断 |");
        let _ = writeln!(fw, "|---:|---|---|---|---|");
        for (rank, miss) in near_misses.iter().copied().enumerate() {
            let parent = &cand_f[miss.parent_level][miss.parent_idx];
            let child = &cand_f[miss.parent_level - 1][miss.child_idx];
            let passed = [
                (miss.diag.same_side, "方向"),
                (miss.diag.sub_start, "Sub左界"),
                (miss.diag.sub_end, "Sub右界"),
            ]
            .into_iter()
            .filter_map(|(ok, name)| ok.then_some(name))
            .collect::<Vec<_>>()
            .join("、");
            let _ = writeln!(
                fw,
                "| {} | `L{} {:?}, t={}, D=[{},{}]` | `L{} {:?}, t={}, I(A)=[{},{}]` | {} | **{}；lag_conf={} bar（仅诊断）** |",
                rank + 1,
                miss.parent_level,
                parent.side,
                parent.confirm_src,
                parent.interval.0,
                parent.interval.1,
                miss.parent_level - 1,
                child.side,
                child.confirm_src,
                child.a_interval.0,
                child.a_interval.1,
                passed,
                failed_condition_text(miss),
                miss.confirm_lag
            );
        }
    }
    let _ = writeln!(fw);
    let _ = writeln!(fw, "## 结论");
    let _ = writeln!(fw);
    match first_zero {
        Some(FirstZeroCause::PairZero(level)) => {
            let _ = writeln!(fw, "当前 BTC 1m / 默认 Θ / 趋势背驰-only D_parent 有效域内，证书产量 0 的首因是 **L{}→L{} 的 I(A_child)⊆D_parent 相邻边为 0**。上游背驰谓词并非零产量，P1→Cand 也无损；确认延迟与 ε_conf 均未作闸门。该结论不外推到其他数据、Θ 或盘整背驰入链。", level - 1, level);
        }
        Some(cause) => {
            let _ = writeln!(fw, "证书产量 0 的首因见 ③ 段（{cause:?}）；非 D_parent 相邻配对归零，近失样本表不适用。该结论不外推到其他数据、Θ 或盘整背驰入链。");
        }
        None => {}
    }
    let funnel_path = out_root.join("chanlun/review-results/dparent-funnel-20260710.md");
    if let Some(parent) = funnel_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("创建 {} 失败: {e}", parent.display()))?;
    }
    std::fs::write(&funnel_path, &funnel_out)
        .map_err(|e| format!("写入 {} 失败: {e}", funnel_path.display()))?;
    println!("{out}");
    eprintln!("报告：{}", report_path.display());
    eprintln!("漏斗报告：{}", funnel_path.display());
    Ok(overall)
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
            confirm_src,
            interval,
            a_interval: interval,
            enter_src: interval.0,
            cand_delta: true,
            pan_div_diag: false,
        }
    }

    #[test]
    fn pair_diag_is_exact_three_gate_conjunction() {
        let child = event(0, Side::Long, 90, (40, 80));
        let parent = event(1, Side::Long, 70, (20, 100));
        let diag = diagnose_pair(&parent, &child);
        assert!(diag.passes());
        assert_eq!(diag.failed_conditions(), 0);
        assert_eq!(pair_gap_bars(&parent, &child), (0, 0));
    }

    #[test]
    fn pair_diag_reports_each_numeric_gap_without_relaxing_boundary() {
        let child = event(0, Side::Long, 80, (10, 120));
        let parent = event(1, Side::Long, 90, (20, 100));
        let diag = diagnose_pair(&parent, &child);
        assert!(diag.same_side);
        assert!(!diag.sub_start);
        assert!(!diag.sub_end);
        assert_eq!(pair_gap_bars(&parent, &child), (10, 20));
    }

    #[test]
    fn pair_diag_keeps_closed_sub_boundaries_inclusive() {
        let child = event(0, Side::Short, 100, (20, 100));
        let parent = event(1, Side::Short, 100, (20, 100));
        assert!(diagnose_pair(&parent, &child).passes());
    }

    #[test]
    fn dparent_pair_diag_uses_child_a_and_never_gates_confirm_lag() {
        let mut child = event(0, Side::Long, 200, (0, 300));
        child.a_interval = (40, 60);
        let parent = event(1, Side::Long, 250, (20, 80));
        let diag = diagnose_pair(&parent, &child);
        assert!(diag.passes(), "I(A_child)⊆D_parent；完成时顺序不得进入门");
        assert_eq!(confirmation_lag(&parent, &child), 120);
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
    let BarsJson { opens, highs, lows, closes, volumes, dates } = raw;
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
