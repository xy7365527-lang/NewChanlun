//! #944 买卖点层失配分布探针（**只读测量**，不改任何生产判据/默认配置/既有测试）。
//!
//! 用法：
//! ```text
//! cargo run --release --bin p944_bsp_layer_mismatch -- \
//!     ../.chanlun/review-results/data/issue940_inputs.json \
//!     ../.chanlun/review-results/issue944-bsp-metrics-20260808.json
//! ```
//!
//! ## 与 #940 的关系
//!
//! 输入 JSON **就是 #940 的 `issue940_inputs.json`**（同一个 `prep_issue940.py` 产物，
//! 同窗口、同标的、同 A/B/C 定义），笔档位也同样跑 `new`（#813 生产默认）与 `wide` 两档。
//! 结构层（笔/线段/中枢/走势）的组装**逐字复用 `p940_struct_input_abc.rs::run_arm`**，
//! 本 bin 只把管线**往下游多接两层**：`divergence` → `buysellpoint`。
//!
//! ## 为什么用 `#[path]` 把引擎模块直接编进本 bin
//!
//! `rust/src/lib.rs:49-79` 里 `bi_engine`（`:49`）/`buysellpoint`（`:51`）/`divergence`（`:54`）/
//! `fractal`（`:55`）/`macd`（`:60`）/`moves`（`:61`）/`ph`（`:63`）/`segment`（`:67`）/
//! `stroke`（`:72`）/`zhongshu`（`:79`）**全是私有 `mod`**（该区间内只有 `fugue_v3`（`:58`）/
//! `recursive_t`（`:66`）/`spiral`（`:71`）/`theta_v0`（`:77`）是 `pub mod`），
//! bin 无法经 `newchan_rust::` 触达；
//! 而票面纪律禁止改既有文件，不能把它们改成 `pub`。`#[path]` 把**同一份源文件**编进本 bin
//! —— 跑的是逐字节同一份实现，**没有造第二套平行实现**（同 #940 探针的做法）。
//!
//! ## 买卖点两层的组装口径（逐行打开确认，非凭记忆）
//!
//! 生产里有**两条**买卖点链，本 bin 两条都量：
//!
//! ### 层 A：线段中枢买卖点（`orchestrator.rs::compute_bsps`，`:1013-1042`）
//!
//! | 入参 | 本 bin | 生产落点 |
//! |---|---|---|
//! | `segs` | `seg_views(全部 Segment)`，`settled = s.confirmed` | `orchestrator.rs:1014` `let segs = seg_views(&self.prev_segments);`；`seg_views` 定义 `:398`，其 `settled: s.confirmed` 在 `:407` |
//! | `zss` / `zs_break` | `zhongshu_from_segments(...)` 的输出 | `:1015` `zs_views(self.inc_seg_zs.zhongshus())` / `:1016` `zs_break(...)`（定义 `:412` / `:424`）；`inc_seg_zs` 的 bit-exact 契约 = `zhongshu_from_segments`（`segment_layers.rs:35`） |
//! | `moves` | `attach_persistence(moves_from_zhongshus(zs, Some(n_seg)), pv)` | `:1017` `move_views(&self.prev_moves)`（定义 `:430`），`prev_moves` 由 `:887` `moves_from_zhongshus(self.inc_seg_zs.zhongshus(), Some(n_seg))` + `:889` `attach_persistence` 生成 |
//! | `macd_ctx` | `None` | `enable_macd_divergence` 生产默认 = `false`（`lib.rs:965` pyo3 signature） |
//! | `level_id` | `1` | `orchestrator.rs:1033` 传 `self.level_id`；`RecursiveOrchestrator::new` 里 `level_id: 1`（`:760`） |
//! | `require_settled` | `false` | `:1041` 传 `self.require_settled_subseg`，pyo3 默认 `false`（`lib.rs:967`） |
//!
//! 生产实际走的是增量器 `IncrementalSegBsp`（`use_inc_bsp = enable_bsp && !enable_macd_divergence`
//! ⟹ 默认 `true`，`orchestrator.rs:776`），其**逐位等价于 `compute_bsps`（无 macd 分支）**——契约
//! 逐字写在 `orchestrator.rs:922`（「逐位等价于 `compute_bsps`（无 macd 分支）——由真实数据端到端
//! 差分守卫」）。故本 bin 走批量 `compute_bsps` 口径不引入偏差。
//!
//! ### 层 B：笔中枢买卖点（`bi_zhongshu_bsp.rs::IncrementalBiZhongshuBsp`）
//!
//! 该引擎的 bit-exact 契约逐字写在 `bi_zhongshu_bsp.rs:6-8`：
//! 「等价于逐前缀全量 `confirmed 笔 → zhongshu_from_strokes → moves_from_zhongshus →
//! divergences_from_moves_v1(None) → buysellpoints_from_level`」。本 bin 按该式批量重算：
//!
//! - `segs` = confirmed 笔前缀的 `SegView`，**`settled` 恒 `true`**（`bi_zhongshu_bsp.rs:104` 注
//!   「笔此处全 confirmed ⟹ settled=true（settle 门无效）」，赋值在 `:112`）
//! - `num_segments` 传给 `moves_from_zhongshus` 的是 **confirmed 笔数**（`:116`
//!   `let num_segments = self.segs.len();`）
//! - **不过 `attach_persistence`**（笔链里没有这一步，见 `push_confirmed_stroke` `:96-190` 全文）
//! - `require_settled = false`、`level_id = 1`（`bi_zhongshu_bsp.rs:78`
//!   `bsp: IncrementalBsp::new(level_id, false)`；`level_id` 的 pyo3 默认 = 1，见 `lib.rs:1178`
//!   `#[pyo3(signature = (level_id = 1))]`）
//! - confirmed 前缀口径 = `lib.rs:1560-1573` `sync_inc`：`:1564` 末笔未 confirmed 则取 `n-1`，否则 `n`
//!
//! 为什么两层都量：线段中枢层在本窗口（209 天 / 1h）**样本极小**（#940：10 标的合计 18 个线段中枢），
//! 只报它会得到一个不可解读的个位数；笔中枢层同属生产路径（`lib.rs` 的 `inc_bz`）且样本大一个量级。
//! **两层分开报，不合并、不互相代表。**
//!
//! ## 买卖点的对齐判据
//!
//! 买卖点的挂钟落点 = `bsp.bar_idx`（= 所在段/笔的 `i1`，`buysellpoint.rs:218/284/379/777/846/905`
//! 六处赋值全部为 `seg.i1`）经 `BiEngine::merged_to_raw()` 换成 raw bar 区间 `[ts[r0], ts[r1]+1h)`。
//! 与 #940 端点层同一套 `gap ≤ tol` 判据、同一套 `PRIMARY_TOL = 2h`、同一套贪心一对一。
//! **严配**：`kind`（type1/2/3）**与** `side`（buy/sell）都相同；**松配**：只要 `side` 相同
//! （用来把「这个点消失了」和「这个点还在但被改判了类型」分开）。

#[path = "../bi_engine.rs"]
mod bi_engine;
#[path = "../buysellpoint.rs"]
mod buysellpoint;
#[path = "../divergence.rs"]
mod divergence;
#[path = "../fractal.rs"]
mod fractal;
#[path = "../macd.rs"]
mod macd;
#[path = "../moves.rs"]
mod moves;
#[path = "../ph.rs"]
mod ph;
#[path = "../segment.rs"]
mod segment;
#[path = "../stroke.rs"]
mod stroke;
#[path = "../zhongshu.rs"]
mod zhongshu;

use bi_engine::BiEngine;
use buysellpoint::{buysellpoints_from_level, BspKind, BuySellPoint, Side};
use divergence::{divergences_from_moves_v1, MoveView, SegView, ZsView};
use moves::{moves_from_zhongshus, Move};
use ph::{attach_persistence, ZsPriceView};
use segment::{SegKind, Segment};
use serde::Deserialize;
use stroke::{Direction, Stroke};
use zhongshu::{zhongshu_from_segments, zhongshu_from_strokes, Zhongshu};

/// 两个笔档位同跑（同 #940；主读数 = `new`，#813 裁定的生产默认）。
const STROKE_MODES: [&str; 2] = ["new", "wide"];
const BAR_SECS: i64 = 3600;

/// 对齐容差扫描（秒）。取值与理由**逐字沿用 #940 端点层**，否则跨层不可比。
const TOLERANCES: [i64; 5] = [0, 3600, 7200, 21600, 86400];
const PRIMARY_TOL: i64 = 7200;

/// 零假设对照的环形平移量（天）。同 #940。
const NULL_SHIFT_DAYS: [i64; 5] = [29, 57, 89, 113, 151];

/// 价格噪声对照臂 A′ 的噪声半宽 + 种子。同 #940（`median|ε| = 0.051%` = #934 实测 perp-vs-cash
/// 1h 收盘价偏差中位数）。
const NOISE_HALFWIDTH: f64 = 0.00102;
const NOISE_SEEDS: [u64; 3] = [0x9E3779B97F4A7C15, 0xBF58476D1CE4E5B9, 0x94D049BB133111EB];

// ════════════════════════════════════════════════════════════
// 输入（与 #940 的 issue940_inputs.json 同 schema）
// ════════════════════════════════════════════════════════════

#[derive(Deserialize)]
struct Arm {
    ts: Vec<i64>,
    o: Vec<f64>,
    h: Vec<f64>,
    l: Vec<f64>,
    c: Vec<f64>,
    #[allow(dead_code)]
    cls: Vec<String>,
}

#[derive(Deserialize)]
struct SymbolInput {
    symbol: String,
    window_start_utc: i64,
    window_end_utc: i64,
    n_sessions_in_window: usize,
    #[serde(rename = "A")]
    a: Arm,
    #[serde(rename = "B")]
    b: Arm,
    #[serde(rename = "C")]
    c: Arm,
}

#[derive(Deserialize)]
struct Inputs {
    symbols: Vec<SymbolInput>,
}

// ════════════════════════════════════════════════════════════
// 产物
// ════════════════════════════════════════════════════════════

/// 一个买卖点在挂钟时间上的落点 + 分类。
#[derive(Debug, Clone, Copy)]
struct BspPt {
    kind: BspKind,
    side: Side,
    /// bar_idx 所在 merged bar 覆盖的挂钟区间 [t0, t1)。
    t0: i64,
    t1: i64,
    price: f64,
    confirmed: bool,
    settled: bool,
}

fn kind_str(k: BspKind) -> &'static str {
    k.as_str()
}
fn side_str(s: Side) -> &'static str {
    s.as_str()
}

struct ArmResult {
    n_bars: usize,
    n_strokes: usize,
    n_strokes_confirmed: usize,
    n_segments: usize,
    n_bi_centers: usize,
    n_seg_centers: usize,
    n_moves_l1: usize,
    /// 端点序列（（is_top, t0, t1））——用来在同一次跑里复现 #940 的端点层读数作交叉校验。
    endpoints: Vec<(bool, i64, i64)>,
    /// 线段中枢买卖点层（生产 `compute_bsps` 口径）。
    seg_bsps: Vec<BspPt>,
    /// 笔中枢买卖点层（生产 `IncrementalBiZhongshuBsp` 口径）。
    bi_bsps: Vec<BspPt>,
}

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// 噪声对照臂 A′：逐 bar 同一个 `1+ε` 缩放（同 #940，OHLC 大小序恒保持）。
fn noised(arm: &Arm, seed: u64) -> Arm {
    let mut st = seed;
    let n = arm.ts.len();
    let (mut o, mut h, mut l, mut c) = (
        Vec::with_capacity(n),
        Vec::with_capacity(n),
        Vec::with_capacity(n),
        Vec::with_capacity(n),
    );
    for i in 0..n {
        let u = (splitmix64(&mut st) >> 11) as f64 / ((1u64 << 53) as f64);
        let f = 1.0 + (2.0 * u - 1.0) * NOISE_HALFWIDTH;
        o.push(arm.o[i] * f);
        h.push(arm.h[i] * f);
        l.push(arm.l[i] * f);
        c.push(arm.c[i] * f);
    }
    Arm {
        ts: arm.ts.clone(),
        o,
        h,
        l,
        c,
        cls: arm.cls.clone(),
    }
}

fn seg_views_of(segs: &[Segment]) -> Vec<SegView> {
    // 逐字段照抄 orchestrator.rs:399-413 `seg_views`。
    segs.iter()
        .map(|s| SegView {
            direction: s.direction,
            high: s.high,
            low: s.low,
            i0: s.i0,
            i1: s.i1,
            settled: s.confirmed,
        })
        .collect()
}

fn zs_views_of(zss: &[Zhongshu]) -> Vec<ZsView> {
    // orchestrator.rs:415-425 `zs_views`。
    zss.iter()
        .map(|z| ZsView {
            zd: z.zd,
            zg: z.zg,
            seg_start: z.seg_start,
            seg_end: z.seg_end,
            settled: z.settled,
        })
        .collect()
}

fn zs_break_of(zss: &[Zhongshu]) -> Vec<(bool, zhongshu::BreakDir, i64)> {
    // orchestrator.rs:427-432 `zs_break`。
    zss.iter()
        .map(|z| (z.settled, z.break_direction, z.break_seg))
        .collect()
}

fn move_views_of(mvs: &[Move]) -> Vec<MoveView> {
    // orchestrator.rs:434-449 `move_views`。
    mvs.iter()
        .map(|m| MoveView {
            kind: m.kind,
            direction: m.direction,
            seg_start: m.seg_start,
            seg_end: m.seg_end,
            zs_start: m.zs_start,
            zs_end: m.zs_end,
            zs_count: m.zs_count,
            settled: m.settled,
        })
        .collect()
}

/// merged 下标 → 挂钟区间。越界返回 None（防御；正常路径不触发）。
fn span_of(m2r: &[(usize, usize)], ts: &[i64], m: usize) -> Option<(i64, i64)> {
    let &(r0, r1) = m2r.get(m)?;
    Some((*ts.get(r0)?, *ts.get(r1)? + BAR_SECS))
}

fn to_pts(bsps: &[BuySellPoint], m2r: &[(usize, usize)], ts: &[i64]) -> Vec<BspPt> {
    let mut v: Vec<BspPt> = bsps
        .iter()
        .filter_map(|b| {
            let m = if b.bar_idx < 0 {
                return None;
            } else {
                b.bar_idx as usize
            };
            let (t0, t1) = span_of(m2r, ts, m)?;
            Some(BspPt {
                kind: b.kind,
                side: b.side,
                t0,
                t1,
                price: b.price,
                confirmed: b.confirmed,
                settled: b.settled,
            })
        })
        .collect();
    v.sort_by_key(|p| (p.t0, p.t1));
    v
}

fn run_arm(arm: &Arm, mode: &str) -> ArmResult {
    // ── 笔（生产参数：min_strict_sep=5 / reset_dir_on_fractal=false / new_raw_gap_min=3，
    //    与 lib.rs:137-140 pyo3 默认签名一致；同 #940 探针）──
    let mut eng = BiEngine::new(mode, 5, false, 3);
    for i in 0..arm.ts.len() {
        eng.process_bar(arm.o[i], arm.h[i], arm.l[i], arm.c[i]);
    }
    let strokes: Vec<Stroke> = eng.current_strokes().to_vec();
    let m2r: Vec<(usize, usize)> = eng.merged_to_raw().to_vec();

    // ── 线段（生产参数 3, true）──
    let segments: Vec<Segment> = segment::segments_from_strokes_v1(&strokes, 3, true);
    let n_seg = segments.len();

    // ── 笔中枢 / 线段中枢 ──
    let stroke_tuples: Vec<(usize, usize, f64, f64, bool)> = strokes
        .iter()
        .map(|s| (s.i0, s.i1, s.high, s.low, s.confirmed))
        .collect();
    let bi_centers = zhongshu_from_strokes(&stroke_tuples);

    let seg_tuples: Vec<(usize, usize, f64, f64, bool, bool)> = segments
        .iter()
        .map(|s| {
            (
                s.s0,
                s.s1,
                s.high,
                s.low,
                s.confirmed,
                s.kind == SegKind::Settled,
            )
        })
        .collect();
    let seg_centers: Vec<Zhongshu> = zhongshu_from_segments(&seg_tuples);

    // ── L1 走势（orchestrator.rs:891-895）──
    let mut l1_moves: Vec<Move> = moves_from_zhongshus(&seg_centers, Some(n_seg));
    let pv: Vec<ZsPriceView> = seg_centers
        .iter()
        .map(|z| ZsPriceView { dd: z.dd, gg: z.gg })
        .collect();
    l1_moves = attach_persistence(&l1_moves, &pv);

    // ── 层 A：线段中枢买卖点（compute_bsps，macd_ctx = None，level_id = 1，require_settled = false）──
    let segs_v = seg_views_of(&segments);
    let zss_v = zs_views_of(&seg_centers);
    let zsb_v = zs_break_of(&seg_centers);
    let mvs_v = move_views_of(&l1_moves);
    let seg_divs = divergences_from_moves_v1(&segs_v, &zss_v, &mvs_v, 1, None);
    let seg_bsp_raw =
        buysellpoints_from_level(&segs_v, &zss_v, &zsb_v, &mvs_v, &seg_divs, 1, false);

    // ── 层 B：笔中枢买卖点（bi_zhongshu_bsp.rs 的 bit-exact 批量式）──
    // confirmed 前缀口径 = lib.rs:1565-1571 sync_inc（末笔未 confirmed 则 n-1）。
    let n_all = strokes.len();
    let confirmed_count = if n_all > 0 && !strokes[n_all - 1].confirmed {
        n_all - 1
    } else {
        n_all
    };
    let bi_segs_v: Vec<SegView> = strokes[..confirmed_count]
        .iter()
        .map(|s| SegView {
            direction: s.direction,
            high: s.high,
            low: s.low,
            i0: s.i0,
            i1: s.i1,
            settled: true, // bi_zhongshu_bsp.rs:110「笔此处全 confirmed ⟹ settled=true」
        })
        .collect();
    let bi_zs_src: Vec<(usize, usize, f64, f64, bool)> = strokes[..confirmed_count]
        .iter()
        .map(|s| (s.i0, s.i1, s.high, s.low, s.confirmed))
        .collect();
    let bi_zss = zhongshu_from_strokes(&bi_zs_src);
    let bi_zss_v = zs_views_of(&bi_zss);
    let bi_zsb_v = zs_break_of(&bi_zss);
    // num_segments = confirmed 笔数（bi_zhongshu_bsp.rs:117 self.segs.len()）；**无 attach_persistence**。
    let bi_moves = moves_from_zhongshus(&bi_zss, Some(confirmed_count));
    let bi_mvs_v = move_views_of(&bi_moves);
    let bi_divs = divergences_from_moves_v1(&bi_segs_v, &bi_zss_v, &bi_mvs_v, 1, None);
    let bi_bsp_raw = buysellpoints_from_level(
        &bi_segs_v, &bi_zss_v, &bi_zsb_v, &bi_mvs_v, &bi_divs, 1, false,
    );

    // ── 端点序列（#940 口径，用于交叉校验本 bin 与 #940 的结构层是否同一读数）──
    let mut endpoints: Vec<(bool, i64, i64)> = Vec::new();
    if let Some(first) = strokes.first() {
        if let Some((t0, t1)) = span_of(&m2r, &arm.ts, first.i0) {
            endpoints.push((first.direction == Direction::Down, t0, t1));
        }
    }
    for s in &strokes {
        if let Some((t0, t1)) = span_of(&m2r, &arm.ts, s.i1) {
            endpoints.push((s.direction == Direction::Up, t0, t1));
        }
    }

    ArmResult {
        n_bars: arm.ts.len(),
        n_strokes: n_all,
        n_strokes_confirmed: confirmed_count,
        n_segments: n_seg,
        n_bi_centers: bi_centers.len(),
        n_seg_centers: seg_centers.len(),
        n_moves_l1: l1_moves.len(),
        endpoints,
        seg_bsps: to_pts(&seg_bsp_raw, &m2r, &arm.ts),
        bi_bsps: to_pts(&bi_bsp_raw, &m2r, &arm.ts),
    }
}

// ════════════════════════════════════════════════════════════
// 对齐
// ════════════════════════════════════════════════════════════

fn gap(a0: i64, a1: i64, b0: i64, b1: i64) -> i64 {
    if a1 <= b0 {
        b0 - a1
    } else if b1 <= a0 {
        a0 - b1
    } else {
        0
    }
}

/// 严配 = kind ∧ side 都同；松配 = 只要 side 同（`strict=false`）。
fn compatible(a: &BspPt, b: &BspPt, strict: bool) -> bool {
    if a.side != b.side {
        return false;
    }
    !strict || a.kind == b.kind
}

/// 一对一贪心对齐（同 #940 端点层：多个候选取 gap 最小，并列取更早）。
/// 返回 (匹配数, a 侧每个点是否被匹配, b 侧每个点是否被认领)。
fn align(a: &[BspPt], b: &[BspPt], tol: i64, strict: bool) -> (usize, Vec<bool>, Vec<bool>) {
    let mut used = vec![false; b.len()];
    let mut hit = vec![false; a.len()];
    let mut matched = 0usize;
    for (i, ea) in a.iter().enumerate() {
        let mut best: Option<(i64, usize)> = None;
        for (j, eb) in b.iter().enumerate() {
            if used[j] || !compatible(ea, eb, strict) {
                continue;
            }
            let g = gap(ea.t0, ea.t1, eb.t0, eb.t1);
            if g > tol {
                continue;
            }
            if best.map(|(bg, _)| g < bg).unwrap_or(true) {
                best = Some((g, j));
            }
        }
        if let Some((_, j)) = best {
            used[j] = true;
            hit[i] = true;
            matched += 1;
        }
    }
    (matched, hit, used)
}

/// 端点层对齐（#940 同款，用于交叉校验）。
fn align_ep(a: &[(bool, i64, i64)], b: &[(bool, i64, i64)], tol: i64) -> usize {
    let mut used = vec![false; b.len()];
    let mut m = 0usize;
    for ea in a {
        let mut best: Option<(i64, usize)> = None;
        for (j, eb) in b.iter().enumerate() {
            if used[j] || eb.0 != ea.0 {
                continue;
            }
            let g = gap(ea.1, ea.2, eb.1, eb.2);
            if g > tol {
                continue;
            }
            if best.map(|(bg, _)| g < bg).unwrap_or(true) {
                best = Some((g, j));
            }
        }
        if let Some((_, j)) = best {
            used[j] = true;
            m += 1;
        }
    }
    m
}

/// 环形平移（零假设对照，同 #940）。
fn shift_circ(pts: &[BspPt], w0: i64, wlen: i64, days: i64) -> Vec<BspPt> {
    pts.iter()
        .map(|p| {
            let nt0 = w0 + (p.t0 - w0 + days * 86400).rem_euclid(wlen);
            BspPt {
                t0: nt0,
                t1: nt0 + (p.t1 - p.t0),
                ..*p
            }
        })
        .collect()
}

// ════════════════════════════════════════════════════════════
// 输出
// ════════════════════════════════════════════════════════════

fn pt_json(p: &BspPt, flags: &[(&str, String)]) -> String {
    let mut s = format!(
        "{{\"kind\":\"{}\",\"side\":\"{}\",\"t0\":{},\"t1\":{},\"price\":{:.6},\
         \"confirmed\":{},\"settled\":{}",
        kind_str(p.kind),
        side_str(p.side),
        p.t0,
        p.t1,
        p.price,
        p.confirmed,
        p.settled
    );
    for (k, v) in flags {
        s.push_str(&format!(",\"{}\":{}", k, v));
    }
    s.push('}');
    s
}

/// 一个买卖点层的全部对照读数。
struct LayerOut {
    json: String,
}

#[allow(clippy::too_many_arguments)]
fn layer_report(
    a: &[BspPt],
    b: &[BspPt],
    c: &[BspPt],
    noise_arms: &[Vec<BspPt>],
    w0: i64,
    wlen: i64,
) -> LayerOut {
    // 主容差下的严配 / 松配（A→C、A→B、C→A、B→A）
    let (ac_s, a_hit_c_s, c_used_s) = align(a, c, PRIMARY_TOL, true);
    let (ac_l, a_hit_c_l, c_used_l) = align(a, c, PRIMARY_TOL, false);
    let (ab_s, a_hit_b_s, b_used_s) = align(a, b, PRIMARY_TOL, true);
    let (_ab_l, _, _) = align(a, b, PRIMARY_TOL, false);

    // 容差扫描（严配）
    let mut tol_rows: Vec<String> = Vec::new();
    for tol in TOLERANCES {
        let (m_ac, _, u_c) = align(a, c, tol, true);
        let (m_ab, _, u_b) = align(a, b, tol, true);
        let (m_ac_l, _, _) = align(a, c, tol, false);
        // 零假设：A 环形平移后再对 C / B
        let (mut n_ac, mut n_ab) = (0usize, 0usize);
        for d in NULL_SHIFT_DAYS {
            let sh = shift_circ(a, w0, wlen, d);
            n_ac += align(&sh, c, tol, true).0;
            n_ab += align(&sh, b, tol, true).0;
        }
        let k = NULL_SHIFT_DAYS.len() as f64;
        tol_rows.push(format!(
            "{{\"tol_s\":{},\"ac_strict\":{},\"ac_loose\":{},\"ab_strict\":{},\
             \"c_unclaimed\":{},\"b_unclaimed\":{},\"null_ac_mean\":{:.2},\"null_ab_mean\":{:.2}}}",
            tol,
            m_ac,
            m_ac_l,
            m_ab,
            u_c.iter().filter(|x| !**x).count(),
            u_b.iter().filter(|x| !**x).count(),
            n_ac as f64 / k,
            n_ab as f64 / k
        ));
    }

    // 噪声天花板 A→A′（严配，主容差；三种子）
    let mut noise_matched = 0f64;
    let mut noise_count = 0f64;
    for na in noise_arms {
        noise_matched += align(a, na, PRIMARY_TOL, true).0 as f64;
        noise_count += na.len() as f64;
    }
    let ns = noise_arms.len().max(1) as f64;

    // A 侧逐点（带是否在 C 中有对应物）
    let a_list: Vec<String> = a
        .iter()
        .enumerate()
        .map(|(i, p)| {
            pt_json(
                p,
                &[
                    ("in_C_strict", a_hit_c_s[i].to_string()),
                    ("in_C_loose", a_hit_c_l[i].to_string()),
                    ("in_B_strict", a_hit_b_s[i].to_string()),
                ],
            )
        })
        .collect();
    // C 侧逐点（带是否被 A 认领 —— 「实盘会开、回测里不存在」的那些仓）
    let c_list: Vec<String> = c
        .iter()
        .enumerate()
        .map(|(i, p)| {
            pt_json(
                p,
                &[
                    ("claimed_by_A_strict", c_used_s[i].to_string()),
                    ("claimed_by_A_loose", c_used_l[i].to_string()),
                ],
            )
        })
        .collect();
    let b_list: Vec<String> = b
        .iter()
        .enumerate()
        .map(|(i, p)| pt_json(p, &[("claimed_by_A_strict", b_used_s[i].to_string())]))
        .collect();

    LayerOut {
        json: format!(
            "{{\"n_A\":{},\"n_B\":{},\"n_C\":{},\"ac_strict\":{},\"ac_loose\":{},\"ab_strict\":{},\
             \"tol_scan\":[{}],\"noise\":{{\"matched_mean\":{:.2},\"n_mean\":{:.2}}},\
             \"A\":[{}],\"C\":[{}],\"B\":[{}]}}",
            a.len(),
            b.len(),
            c.len(),
            ac_s,
            ac_l,
            ab_s,
            tol_rows.join(","),
            noise_matched / ns,
            noise_count / ns,
            a_list.join(","),
            c_list.join(","),
            b_list.join(",")
        ),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("用法: p944_bsp_layer_mismatch <inputs.json> <metrics_out.json>");
        std::process::exit(2);
    }
    let raw = std::fs::read_to_string(&args[1]).expect("读取输入 JSON 失败");
    let inputs: Inputs = serde_json::from_str(&raw).expect("解析输入 JSON 失败");

    let mut out = String::from("{\n  \"records\": [\n");
    let mut first = true;

    for sym in &inputs.symbols {
        for mode in STROKE_MODES {
            let ra = run_arm(&sym.a, mode);
            let rb = run_arm(&sym.b, mode);
            let rc = run_arm(&sym.c, mode);
            let noise: Vec<ArmResult> = NOISE_SEEDS
                .iter()
                .map(|&s| run_arm(&noised(&sym.a, s), mode))
                .collect();

            let w0 = sym.window_start_utc;
            let wlen = (sym.window_end_utc - sym.window_start_utc).max(1);

            let seg_layer = layer_report(
                &ra.seg_bsps,
                &rb.seg_bsps,
                &rc.seg_bsps,
                &noise.iter().map(|n| n.seg_bsps.clone()).collect::<Vec<_>>(),
                w0,
                wlen,
            );
            let bi_layer = layer_report(
                &ra.bi_bsps,
                &rb.bi_bsps,
                &rc.bi_bsps,
                &noise.iter().map(|n| n.bi_bsps.clone()).collect::<Vec<_>>(),
                w0,
                wlen,
            );

            // 端点层交叉校验（#940 同口径；主容差）
            let ep_ac = align_ep(&ra.endpoints, &rc.endpoints, PRIMARY_TOL);
            let ep_ab = align_ep(&ra.endpoints, &rb.endpoints, PRIMARY_TOL);
            let mut ep_noise = 0f64;
            for n in &noise {
                ep_noise += align_ep(&ra.endpoints, &n.endpoints, PRIMARY_TOL) as f64;
            }

            let arm_json = |r: &ArmResult| {
                format!(
                    "{{\"n_bars\":{},\"n_strokes\":{},\"n_strokes_confirmed\":{},\"n_segments\":{},\
                     \"n_bi_centers\":{},\"n_seg_centers\":{},\"n_moves_l1\":{},\"n_endpoints\":{},\
                     \"n_seg_bsps\":{},\"n_bi_bsps\":{}}}",
                    r.n_bars,
                    r.n_strokes,
                    r.n_strokes_confirmed,
                    r.n_segments,
                    r.n_bi_centers,
                    r.n_seg_centers,
                    r.n_moves_l1,
                    r.endpoints.len(),
                    r.seg_bsps.len(),
                    r.bi_bsps.len()
                )
            };

            if !first {
                out.push_str(",\n");
            }
            first = false;
            out.push_str(&format!(
                "    {{\"symbol\":\"{}\",\"stroke_mode\":\"{}\",\"window\":[{},{}],\
                 \"n_sessions\":{},\"primary_tol_s\":{},\
                 \"arms\":{{\"A\":{},\"B\":{},\"C\":{}}},\
                 \"endpoint_xcheck\":{{\"n_A\":{},\"n_C\":{},\"n_B\":{},\"ac\":{},\"ab\":{},\"noise_mean\":{:.2}}},\
                 \"seg_layer\":{},\"bi_layer\":{}}}",
                sym.symbol,
                mode,
                sym.window_start_utc,
                sym.window_end_utc,
                sym.n_sessions_in_window,
                PRIMARY_TOL,
                arm_json(&ra),
                arm_json(&rb),
                arm_json(&rc),
                ra.endpoints.len(),
                rc.endpoints.len(),
                rb.endpoints.len(),
                ep_ac,
                ep_ab,
                ep_noise / NOISE_SEEDS.len() as f64,
                seg_layer.json,
                bi_layer.json
            ));

            println!(
                "P944 {sym} mode={mode} | seg_bsp A/B/C = {}/{}/{} | bi_bsp A/B/C = {}/{}/{} \
                 | ep A→C {}/{}",
                ra.seg_bsps.len(),
                rb.seg_bsps.len(),
                rc.seg_bsps.len(),
                ra.bi_bsps.len(),
                rb.bi_bsps.len(),
                rc.bi_bsps.len(),
                ep_ac,
                ra.endpoints.len(),
                sym = sym.symbol,
                mode = mode
            );
        }
    }
    out.push_str("\n  ]\n}\n");
    std::fs::write(&args[2], out).expect("写 metrics JSON 失败");
    eprintln!("metrics -> {}", args[2]);
}
