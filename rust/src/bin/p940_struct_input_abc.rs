//! #940 结构输入 A/B/C 三套的笔/线段/中枢差异探针（**只读测量**，不改任何生产判据）。
//!
//! 用法：
//! ```text
//! cargo run --release --bin p940_struct_input_abc -- \
//!     ../.chanlun/review-results/data/issue940_inputs.json \
//!     ../.chanlun/review-results/issue940-metrics-20260807.json
//! ```
//!
//! ## 为什么用 `#[path]` 把引擎模块直接编进本 bin
//!
//! `rust/src/lib.rs` 里 `bi_engine` / `stroke` / `segment` / `zhongshu` / `moves` / `level` / `ph`
//! **全是私有 mod**（`lib.rs:49-79`，只有 `fugue_v3` / `recursive_t` / `spiral` / `theta_v0` 是
//! `pub mod`），故 bin 无法经 `newchan_rust::` 触达它们；而票面纪律是「不改仓内任何既有文件」，
//! 不能把它们改成 `pub`。`#[path]` 直接把**同一份源文件**编进本 bin：跑的是逐字节同一份实现，
//! 没有第二套平行实现（本仓在案病灶正是「两条平行实现互不复用」，见 `parser/mod.rs:append_incr_layer`
//! 的同款说明）。这些模块之间只用 `crate::{fractal,stroke,zhongshu,moves}`，在 bin crate 根下
//! 逐一声明后路径全部解析成功，无 pyo3 依赖。
//!
//! ## 组装口径（逐条对着 `orchestrator.rs::process_bar` 抄，不自创）
//!
//! | 层 | 本 bin | 生产路径（orchestrator.rs 行号） |
//! |---|---|---|
//! | 笔 | `BiEngine::new(mode, 5, false, 3)` 逐 bar `process_bar` | `:754` `BiEngine::new(...)`、`:837` `self.bi.process_bar` |
//! | 线段 | `segments_from_strokes_v1(strokes, 3, true)` | `:860`（全量分支）/ `:857`（resume 分支），参数同为 `3, true` |
//! | 线段中枢 | `zhongshu_from_segments(confirmed ∧ Settled)` | `:873` `inc_seg_zs.update`，其 bit-exact 契约 = `zhongshu_from_segments`（`segment_layers.rs:35`） |
//! | 笔中枢 | `zhongshu_from_strokes(confirmed 笔)` | `zhongshu.rs:320`（笔中枢层入口） |
//! | L1 走势 | `moves_from_zhongshus(zs, Some(n_seg))` + `attach_persistence` | `:887-889` |
//! | L≥2 递归 | `zhongshu_from_components` + `moves_from_level_zhongshus` + `attach_persistence`，`should_stop_recursion` 停机 | `LevelEngine::process`（`:553` 起）+ `RecursiveStack::process`（`:636` 起） |
//!
//! **本 bin 不算买卖点**（`level_buysellpoints` 需 `divergence`/`buysellpoint` 两个模块，与本票
//! 要量的五项无关），故递归层缺 `buysellpoints` 字段——**中枢/走势/停机判据逐字段与生产一致**。
//!
//! ## 笔的档位
//!
//! 由命令行之外的常量 [`STROKE_MODES`] 控制，**同时跑 `new` 与 `wide` 两档**并分别报数——
//! `.chanlun/definitions/bi.md` v2.0（#813）裁定「生产默认切新笔」，而 `lib.rs:961`（pyo3 signature）
//! `RecursiveOrchestrator` 的 pyo3 默认值至今仍是 `"wide"`（**实装未跟裁定**）。三套输入的
//! 对比在**同一档**内进行，跨档数字不混用。

#[path = "../fractal.rs"]
mod fractal;
#[path = "../stroke.rs"]
mod stroke;
#[path = "../bi_engine.rs"]
mod bi_engine;
#[path = "../segment.rs"]
mod segment;
#[path = "../zhongshu.rs"]
mod zhongshu;
#[path = "../moves.rs"]
mod moves;
#[path = "../ph.rs"]
mod ph;
#[path = "../level.rs"]
mod level;

use bi_engine::BiEngine;
use level::{moves_from_level_zhongshus, zhongshu_from_components, CompView};
use moves::{moves_from_zhongshus, Move};
use ph::{attach_persistence, should_stop_recursion, ZsPriceView};
use segment::{SegKind, Segment};
use serde::Deserialize;
use std::collections::BTreeMap;
use stroke::{Direction, Stroke};
use zhongshu::{zhongshu_from_segments, zhongshu_from_strokes, Zhongshu};

/// 两个笔档位同跑（理由见模块头「笔的档位」）。
const STROKE_MODES: [&str; 2] = ["new", "wide"];
/// 递归塔上限，取 `analysis/*.py` 全仓惯用值 6（如 `analysis/k4_1min_lib.py:262`）。
const MAX_LEVELS: i64 = 6;
/// 1h bar 的秒数——本票所有输入都是 1 小时线。
const BAR_SECS: i64 = 3600;

/// 端点对齐容差扫描（秒）。主读数 = [`PRIMARY_TOL`]，其余用于敏感性对照。
///
/// **不是随手取的数**（票面明确要求给理由）：
/// - `0`  ：两个端点所在的 merged bar 的**挂钟区间直接相交**（最严，零容差）；
/// - `3600`（1 根 bar）：A 的 bar 边界是 09:30 对齐、B 是整点对齐，两者天然错半小时，
///   1 根 bar 是「同一个转折点因为网格错位落到相邻格」的最小容差；
/// - `7200`（2 根 bar，**主读数**）：在上一条之上再放一根，覆盖「包含处理把相邻两根并成一根、
///   端点因此挪一格」这一在案机制（`bi_engine.rs::incremental_merge`），仍远小于一个交易日；
/// - `21600`（6 根 = 约一个美股交易日的长度）、`86400`（1 天）：粗档，用来看「是不是只是
///   时间刻度不同、结构其实同一个」——若放到 1 天才对得上，那已经不是同一个转折点了。
const TOLERANCES: [i64; 5] = [0, 3600, 7200, 21600, 86400];
const PRIMARY_TOL: i64 = 7200;

/// **零假设对照组的时间平移量（秒）**。B 的端点密度是 A 的约 5 倍，任何「A→B 命中率」
/// 都被这个密度抬高——不给对照就没法判断 72% 是「结构对上了」还是「B 太密，随便撞都撞得到」。
/// 对照做法：把 A 的整条端点序列在时间轴上平移 δ（同一段行情、同一批端点、只是错开），
/// 再跑同一个对齐器。**平移后仍能对上的部分 = 纯粹由密度贡献的偶然命中**。
/// 取 5 个互不相关的偏移（29/57/89/113/151 天，都不是 7 的倍数以免与周节律共振），报均值。
const NULL_SHIFT_DAYS: [i64; 5] = [29, 57, 89, 113, 151];

/// **价格噪声对照臂 A′ 的噪声半宽**。C 与 A 的残差里混着两样东西：①永续价 vs 真实股价的
/// basis，②bar 网格错位（A 的 bar 从 09:30 起、C 从整点起）。本对照只回答①能解释多少：
/// 给 A **自己**的每根 bar 乘一个 `1+ε`（同根 bar 的 OHLC 用同一个 ε，故 `h≥max(o,c)≥min(o,c)≥l`
/// 恒保持），`ε ~ U(-K, K)`。取 `K = 0.00102` ⟹ `median|ε| = K/2 = 0.00051 = 0.051%`，
/// 正好等于 [#934](https://github.com/xy7365527-lang/NewChanlun/issues/934) 实测的 perp-vs-cash
/// 1h 收盘价偏差中位数。**A′ 与 A 是同一根 bar、同一条时间网格**，故 A↔A′ 的差异纯由①贡献。
const NOISE_HALFWIDTH: f64 = 0.00102;
/// 噪声对照的 LCG 种子（三次独立重复取均值；确定性，可复现）。
const NOISE_SEEDS: [u64; 3] = [0x9E3779B97F4A7C15, 0xBF58476D1CE4E5B9, 0x94D049BB133111EB];

// ════════════════════════════════════════════════════════════
// 输入
// ════════════════════════════════════════════════════════════

#[derive(Deserialize)]
struct Arm {
    ts: Vec<i64>,
    o: Vec<f64>,
    h: Vec<f64>,
    l: Vec<f64>,
    c: Vec<f64>,
    /// 每根 bar 的时段名分：session / overnight / weekend / edge（A、C 恒 session）。
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
// 一次跑批的结构产物
// ════════════════════════════════════════════════════════════

/// 一个端点（= 笔的一个端分型）在挂钟时间上的落点。
#[derive(Debug, Clone, Copy)]
struct Endpoint {
    /// true = 顶分型，false = 底分型。
    is_top: bool,
    /// 该端点所在 merged bar 覆盖的挂钟区间 [t0, t1)（t1 = 末根 raw bar 的收盘时刻）。
    t0: i64,
    t1: i64,
    price: f64,
    /// merged bar 覆盖的 raw bar 的时段名分归并（见 `class_of_span`）。
    cls: &'static str,
}

/// 中枢在挂钟时间上的落点 + 价格区间。
#[derive(Debug, Clone, Copy)]
struct CenterSpan {
    t0: i64,
    t1: i64,
    zd: f64,
    zg: f64,
    settled: bool,
}

struct ArmResult {
    n_bars: usize,
    n_merged: usize,
    n_strokes: usize,
    n_strokes_confirmed: usize,
    n_segments: usize,
    n_segments_settled: usize,
    n_bi_centers: usize,
    n_seg_centers: usize,
    n_seg_centers_settled: usize,
    n_moves_l1: usize,
    /// level_id → (中枢数, 走势数)，level_id ≥ 2。
    tower: BTreeMap<i64, (usize, usize)>,
    endpoints: Vec<Endpoint>,
    seg_center_spans: Vec<CenterSpan>,
}

/// merged bar 覆盖 [r0, r1] 的时段归并：碰到开市 bar 记 `session`（这根 merged bar
/// 至少有一部分在开市内），否则整根都在休市——再按 weekend 优先于 overnight 归类。
fn class_of_span(cls: &[String], r0: usize, r1: usize) -> &'static str {
    let mut has_weekend = false;
    for c in cls.iter().take(r1 + 1).skip(r0) {
        match c.as_str() {
            "session" => return "session",
            "weekend" => has_weekend = true,
            _ => {}
        }
    }
    if has_weekend {
        "weekend"
    } else {
        "overnight"
    }
}

/// splitmix64——确定性伪随机（无外部依赖，与 crate 内任何生产随机源无关，仅本对照臂用）。
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// 造噪声对照臂 A′：逐 bar 同一个 `1+ε` 缩放（见 [`NOISE_HALFWIDTH`]）。
fn noised(arm: &Arm, seed: u64) -> Arm {
    let mut st = seed;
    let (mut o, mut h, mut l, mut c) = (
        Vec::with_capacity(arm.ts.len()),
        Vec::with_capacity(arm.ts.len()),
        Vec::with_capacity(arm.ts.len()),
        Vec::with_capacity(arm.ts.len()),
    );
    for i in 0..arm.ts.len() {
        let u = (splitmix64(&mut st) >> 11) as f64 / ((1u64 << 53) as f64); // [0,1)
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

fn run_arm(arm: &Arm, mode: &str) -> ArmResult {
    // ── 笔（生产参数：min_strict_sep=5 / reset_dir_on_fractal=false / new_raw_gap_min=3，
    //    与 `lib.rs:137-140` 的 pyo3 默认签名逐字段一致）──
    let mut eng = BiEngine::new(mode, 5, false, 3);
    for i in 0..arm.ts.len() {
        eng.process_bar(arm.o[i], arm.h[i], arm.l[i], arm.c[i]);
    }
    let strokes: Vec<Stroke> = eng.current_strokes().to_vec();
    let m2r: Vec<(usize, usize)> = eng.merged_to_raw().to_vec();

    // merged idx → 挂钟区间 [t0, t1)
    let span = |m: usize| -> (i64, i64, usize, usize) {
        let (r0, r1) = m2r[m];
        (arm.ts[r0], arm.ts[r1] + BAR_SECS, r0, r1)
    };

    // ── 线段（生产参数 min_seg_strokes=3 / extend_mode_strict=true，orchestrator.rs:865）──
    let segments: Vec<Segment> = segment::segments_from_strokes_v1(&strokes, 3, true);
    let n_segments_settled = segments
        .iter()
        .filter(|s| s.confirmed && s.kind == SegKind::Settled)
        .count();

    // ── 中枢：笔中枢 + 线段中枢 ──
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

    // ── L1 走势（orchestrator.rs:888-891 同口径）──
    let n_seg = segments.len();
    let mut l1_moves: Vec<Move> = moves_from_zhongshus(&seg_centers, Some(n_seg));
    let pv: Vec<ZsPriceView> = seg_centers
        .iter()
        .map(|z| ZsPriceView { dd: z.dd, gg: z.gg })
        .collect();
    l1_moves = attach_persistence(&l1_moves, &pv);

    // ── L≥2 递归塔（RecursiveStack::process + LevelEngine::process 逐句复刻，去掉 bsp）──
    let mut tower: BTreeMap<i64, (usize, usize)> = BTreeMap::new();
    let mut current_moves = l1_moves.clone();
    let mut prev_moves = l1_moves.clone();
    let mut current_level: i64 = 1;
    while current_level < MAX_LEVELS {
        let next_level = current_level + 1;
        let comps: Vec<CompView> = current_moves
            .iter()
            .filter(|m| m.settled)
            .enumerate()
            .map(|(i, m)| CompView {
                high: if m.zg_max != 0.0 { m.zg_max } else { m.high },
                low: if m.zd_min != 0.0 { m.zd_min } else { m.low },
                component_idx: i,
            })
            .collect();
        let zss = zhongshu_from_components(&comps, next_level);
        let mvs = moves_from_level_zhongshus(&zss, Some(comps.len()));
        let lpv: Vec<ZsPriceView> = zss
            .iter()
            .map(|z| ZsPriceView { dd: z.dd, gg: z.gg })
            .collect();
        let mvs = attach_persistence(&mvs, &lpv);
        tower.insert(next_level, (zss.len(), mvs.len()));

        if mvs.len() < 3 || should_stop_recursion(&mvs, &prev_moves) {
            break;
        }
        current_moves = mvs.clone();
        prev_moves = mvs;
        current_level = next_level;
    }

    // ── 端点序列（去重：相邻笔共用端点；序列 = s0.i0 ++ [s.i1]）──
    let mut endpoints: Vec<Endpoint> = Vec::new();
    if let Some(first) = strokes.first() {
        let (t0, t1, r0, r1) = span(first.i0);
        endpoints.push(Endpoint {
            is_top: first.direction == Direction::Down,
            t0,
            t1,
            price: first.p0,
            cls: class_of_span(&arm.cls, r0, r1),
        });
    }
    for s in &strokes {
        let (t0, t1, r0, r1) = span(s.i1);
        endpoints.push(Endpoint {
            is_top: s.direction == Direction::Up,
            t0,
            t1,
            price: s.p1,
            cls: class_of_span(&arm.cls, r0, r1),
        });
    }

    // ── 线段中枢的挂钟落点（first_seg_s0 / last_seg_s1 = 笔下标，见 zhongshu.rs:56 Component）──
    let mut seg_center_spans: Vec<CenterSpan> = Vec::new();
    for z in &seg_centers {
        let (a, b) = (z.first_seg_s0, z.last_seg_s1);
        if a >= strokes.len() || b >= strokes.len() {
            continue;
        }
        let (t0, _, _, _) = span(strokes[a].i0);
        let (_, t1, _, _) = span(strokes[b].i1);
        seg_center_spans.push(CenterSpan {
            t0,
            t1,
            zd: z.zd,
            zg: z.zg,
            settled: z.settled,
        });
    }

    ArmResult {
        n_bars: arm.ts.len(),
        n_merged: m2r.len(),
        n_strokes: strokes.len(),
        n_strokes_confirmed: strokes.iter().filter(|s| s.confirmed).count(),
        n_segments: n_seg,
        n_segments_settled,
        n_bi_centers: bi_centers.len(),
        n_seg_centers: seg_centers.len(),
        n_seg_centers_settled: seg_centers.iter().filter(|z| z.settled).count(),
        n_moves_l1: l1_moves.len(),
        tower,
        endpoints,
        seg_center_spans,
    }
}

// ════════════════════════════════════════════════════════════
// 对齐
// ════════════════════════════════════════════════════════════

/// 两个挂钟区间的间隙（相交 = 0）。
fn gap(a0: i64, a1: i64, b0: i64, b1: i64) -> i64 {
    if a1 <= b0 {
        b0 - a1
    } else if b1 <= a0 {
        a0 - b1
    } else {
        0
    }
}

/// 一对一贪心对齐：同 kind（顶配顶、底配底）且间隙 ≤ tol；同一个 B 端点只能被用一次，
/// 多个候选取间隙最小（并列取更早）。两个端点序列都按时间单调，故贪心即最优匹配的
/// 一个稳定实现（不做全局最优匹配——差别只会出现在密集并列上，本票读的是比率不是个例）。
fn align(a: &[Endpoint], b: &[Endpoint], tol: i64) -> (usize, Vec<bool>) {
    let mut used = vec![false; b.len()];
    let mut matched = 0usize;
    for ea in a {
        let mut best: Option<(i64, usize)> = None;
        for (j, eb) in b.iter().enumerate() {
            if used[j] || eb.is_top != ea.is_top {
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
            matched += 1;
        }
    }
    (matched, used)
}

/// 中枢的对应：挂钟区间相交 **且** 价格区间 [zd, zg] 相交。两条都要，理由：只看时间会把
/// 「同一段行情里价位完全不同的两个中枢」判成同一个；只看价格会把相隔数月的同价位中枢配上。
fn match_centers(a: &[CenterSpan], b: &[CenterSpan]) -> usize {
    let mut used = vec![false; b.len()];
    let mut m = 0usize;
    for ca in a {
        for (j, cb) in b.iter().enumerate() {
            if used[j] {
                continue;
            }
            let t_ok = ca.t0 < cb.t1 && cb.t0 < ca.t1;
            let p_ok = ca.zd <= cb.zg && cb.zd <= ca.zg;
            if t_ok && p_ok {
                used[j] = true;
                m += 1;
                break;
            }
        }
    }
    m
}

// ════════════════════════════════════════════════════════════
// main
// ════════════════════════════════════════════════════════════

fn json_num(v: usize) -> String {
    v.to_string()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("用法: p940_struct_input_abc <inputs.json> <metrics_out.json>");
        std::process::exit(2);
    }
    let raw = std::fs::read_to_string(&args[1]).expect("读取输入 JSON 失败");
    let inputs: Inputs = serde_json::from_str(&raw).expect("解析输入 JSON 失败");

    let mut out = String::from("{\n  \"records\": [\n");
    let mut first_rec = true;

    for sym in &inputs.symbols {
        for mode in STROKE_MODES {
            let ra = run_arm(&sym.a, mode);
            let rb = run_arm(&sym.b, mode);
            let rc = run_arm(&sym.c, mode);

            // 端点对齐（A↔B、A↔C），全容差档
            let mut tol_rows: Vec<String> = Vec::new();
            let mut extra_by_cls: BTreeMap<&'static str, usize> = BTreeMap::new();
            for tol in TOLERANCES {
                let (ab, used_b) = align(&ra.endpoints, &rb.endpoints, tol);
                let (ac, used_c) = align(&ra.endpoints, &rc.endpoints, tol);
                if tol == PRIMARY_TOL {
                    for (j, u) in used_b.iter().enumerate() {
                        if !u {
                            *extra_by_cls.entry(rb.endpoints[j].cls).or_insert(0) += 1;
                        }
                    }
                }
                let unmatched_b = used_b.iter().filter(|u| !**u).count();
                let unmatched_c = used_c.iter().filter(|u| !**u).count();
                // 零假设对照：把 A 端点整体平移 δ 后再对齐，取 5 个偏移的均值（四舍五入）。
                let mut null_ab = 0usize;
                let mut null_ac = 0usize;
                let w0 = sym.window_start_utc;
                let wlen = (sym.window_end_utc - sym.window_start_utc).max(1);
                for d in NULL_SHIFT_DAYS {
                    // **环形**平移（在窗口内取模）：否则平移出窗口的那部分端点必然无对手，
                    // 对照组会被系统性压低、把偶然命中率报得比真实值小。
                    let shifted: Vec<Endpoint> = ra
                        .endpoints
                        .iter()
                        .map(|e| {
                            let nt0 = w0 + (e.t0 - w0 + d * 86400).rem_euclid(wlen);
                            Endpoint {
                                t0: nt0,
                                t1: nt0 + (e.t1 - e.t0),
                                ..*e
                            }
                        })
                        .collect();
                    null_ab += align(&shifted, &rb.endpoints, tol).0;
                    null_ac += align(&shifted, &rc.endpoints, tol).0;
                }
                let n = NULL_SHIFT_DAYS.len();
                tol_rows.push(format!(
                    "{{\"tol_s\":{},\"ab_matched\":{},\"ac_matched\":{},\
                     \"b_unmatched\":{},\"c_unmatched\":{},\
                     \"null_ab_mean\":{:.2},\"null_ac_mean\":{:.2}}}",
                    tol,
                    ab,
                    ac,
                    unmatched_b,
                    unmatched_c,
                    null_ab as f64 / n as f64,
                    null_ac as f64 / n as f64
                ));
            }

            // ── 价格噪声对照臂 A′（三种子取均值；只在主容差档报数）──
            let mut noise_matched = 0f64;
            let mut noise_strokes = 0f64;
            for s in NOISE_SEEDS {
                let rn = run_arm(&noised(&sym.a, s), mode);
                noise_matched += align(&ra.endpoints, &rn.endpoints, PRIMARY_TOL).0 as f64;
                noise_strokes += rn.n_strokes as f64;
            }
            let nseeds = NOISE_SEEDS.len() as f64;

            let ab_centers = match_centers(&ra.seg_center_spans, &rb.seg_center_spans);
            let ac_centers = match_centers(&ra.seg_center_spans, &rc.seg_center_spans);

            let arm_json = |r: &ArmResult| -> String {
                let tower: Vec<String> = r
                    .tower
                    .iter()
                    .map(|(k, (z, m))| format!("\"L{}\":[{},{}]", k, z, m))
                    .collect();
                format!(
                    "{{\"n_bars\":{},\"n_merged\":{},\"n_strokes\":{},\"n_strokes_confirmed\":{},\
                     \"n_segments\":{},\"n_segments_settled\":{},\"n_bi_centers\":{},\
                     \"n_seg_centers\":{},\"n_seg_centers_settled\":{},\"n_moves_l1\":{},\
                     \"n_endpoints\":{},\"tower\":{{{}}}}}",
                    json_num(r.n_bars),
                    json_num(r.n_merged),
                    json_num(r.n_strokes),
                    json_num(r.n_strokes_confirmed),
                    json_num(r.n_segments),
                    json_num(r.n_segments_settled),
                    json_num(r.n_bi_centers),
                    json_num(r.n_seg_centers),
                    json_num(r.n_seg_centers_settled),
                    json_num(r.n_moves_l1),
                    json_num(r.endpoints.len()),
                    tower.join(",")
                )
            };

            let extra: Vec<String> = extra_by_cls
                .iter()
                .map(|(k, v)| format!("\"{}\":{}", k, v))
                .collect();

            if !first_rec {
                out.push_str(",\n");
            }
            first_rec = false;
            out.push_str(&format!(
                "    {{\"symbol\":\"{}\",\"stroke_mode\":\"{}\",\"window\":[{},{}],\
                 \"n_sessions\":{},\"A\":{},\"B\":{},\"C\":{},\"align\":[{}],\
                 \"primary_tol_s\":{},\"noise_ctrl\":{{\"matched_mean\":{:.2},\"strokes_mean\":{:.2}}},\
                 \"extra_B_endpoints_by_class\":{{{}}},\
                 \"seg_center_match\":{{\"A_total\":{},\"B_total\":{},\"C_total\":{},\
                 \"AB_matched\":{},\"AC_matched\":{}}}}}",
                sym.symbol,
                mode,
                sym.window_start_utc,
                sym.window_end_utc,
                sym.n_sessions_in_window,
                arm_json(&ra),
                arm_json(&rb),
                arm_json(&rc),
                tol_rows.join(","),
                PRIMARY_TOL,
                noise_matched / nseeds,
                noise_strokes / nseeds,
                extra.join(","),
                ra.seg_center_spans.len(),
                rb.seg_center_spans.len(),
                rc.seg_center_spans.len(),
                ab_centers,
                ac_centers
            ));

            println!(
                "P940 {sym} mode={mode} | strokes A/B/C = {}/{}/{} | segs {}/{}/{} | \
                 bi_zs {}/{}/{} | seg_zs {}/{}/{} | L1_moves {}/{}/{}",
                ra.n_strokes,
                rb.n_strokes,
                rc.n_strokes,
                ra.n_segments,
                rb.n_segments,
                rc.n_segments,
                ra.n_bi_centers,
                rb.n_bi_centers,
                rc.n_bi_centers,
                ra.n_seg_centers,
                rb.n_seg_centers,
                rc.n_seg_centers,
                ra.n_moves_l1,
                rb.n_moves_l1,
                rc.n_moves_l1,
                sym = sym.symbol,
                mode = mode,
            );
        }
    }
    out.push_str("\n  ]\n}\n");
    std::fs::write(&args[2], out).expect("写 metrics JSON 失败");
    eprintln!("metrics -> {}", args[2]);
}
