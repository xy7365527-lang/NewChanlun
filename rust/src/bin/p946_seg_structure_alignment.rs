//! #946 第二问 C-窄产物（任务 #1024）：L1 线段中枢层「结构对齐」探针
//! （**只读测量**，不改任何生产判据/默认配置/既有测试）。
//!
//! 用法：
//! ```text
//! cargo run --release --features backtest_bin --bin p946_seg_structure_alignment -- \
//!     ../.chanlun/review-results/data/issue946_inputs.json \
//!     ../.chanlun/review-results/issue946-seg-structure-metrics-20260817.json
//! ```
//!
//! ## 测什么 / 不测什么（#946 第二问裁定 C-窄，2026-08-17）
//!
//! - **测**：A/C 两臂 L1 线段中枢**本体**（区间 [zd,zg] + bar 跨域）对齐率 +
//!   中枢**离开结构**（破位方向 `break_direction`）对齐率。生产臂真正消费的是
//!   「线段中枢 + 结构背驰」这套结构，买卖点只是其出口 ⟹ 测结构对齐比测买卖点
//!   更贴近失配的物理源头，且样本由中枢数给（#940：10 标的 A 臂合计 18 个线段中枢，
//!   比买卖点多一个量级）。
//! - **不测**：买卖点对齐（BspKind 层不进测量面——样本天花板绕不开，第二问裁定）；
//!   L2+ 级别（窗口 209 天递归塔止步 L1）；`macd_ctx=None`（生产默认
//!   `enable_macd_divergence=false`，`lib.rs:965`）。
//!
//! ## 与 #944 的关系
//!
//! 结构层组装**逐字复用 `p944_bsp_layer_mismatch.rs::run_arm`** 的生产口径：
//! confirmed 线段 → `zhongshu_from_segments` → `moves_from_zhongshus(Some(n_seg))`
//! → `attach_persistence`（对应 `orchestrator.rs:887-895`）。本 bin 在其上**换掉测量
//! 对象**：不接 `divergence`/`buysellpoint`，改为抽取线段中枢本体做 A↔C 对齐。
//! 输入 JSON schema 与 #940/#944 相同；窗口钳制口径见
//! `.chanlun/review-results/scripts/prep_issue946_clamp.py` 头注。
//!
//! `#[path]` 直接编引擎模块的理由与 p944 相同（`lib.rs:49-79` 这些全是私有 `mod`，
//! 票面纪律禁止改既有文件；编进 bin 跑的是逐字节同一份实现，无第二套平行实现）。
//!
//! ## 中枢字段的挂钟化
//!
//! `zhongshu_from_segments`（`zhongshu.rs:303-315`）先把线段过滤为
//! `confirmed && kind==Settled` 的组件数组再扫描，故 `Zhongshu.seg_start/seg_end/break_seg`
//! **索引的是过滤后数组**——本 bin 用**同一过滤式**重建 `conf_segs` 再按下标取段，
//! 中枢 bar 跨域 = `conf_segs[seg_start].i0 .. conf_segs[seg_end].i1`（merged bar idx，
//! 经 `BiEngine::merged_to_raw()` 换挂钟区间，同 #940/#944 端点层）。
//!
//! ## 两层对齐判据（容差沿用 #944 primary_tol_s = 2h 的 gap≤tol 逻辑）
//!
//! 1. **中枢本体对齐**：贪心一对一（多候选取 gap 最小，并列取更早，同 #940/#944），
//!    相容条件 = 时间 gap ≤ tol **且**价格区间相交（`max(zd) ≤ min(zg)`，即「同区间」）。
//! 2. **离开结构对齐**：A 臂已破位中枢（`break_direction != None`）→ C 臂，相容条件 =
//!    破位方向相同；另报**条件读数** = 本体已配对的中枢对里破位方向一致的比例
//!    （把「中枢对上了但离开方向不同」与「中枢根本没对上」分开）。
//! 3. **零假设对照**：A 臂中枢环形平移 29/57/89/113/151 天后再对齐（逐字沿用 #944 方法）。

#[path = "../bi_engine.rs"]
mod bi_engine;
#[path = "../fractal.rs"]
mod fractal;
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
use moves::moves_from_zhongshus;
use ph::{attach_persistence, ZsPriceView};
use segment::{SegKind, Segment};
use serde::Deserialize;
use stroke::Direction;
use zhongshu::{zhongshu_from_segments, zhongshu_from_strokes, BreakDir, Zhongshu};

/// 两个笔档位同跑（同 #940/#944；主读数 = `new`，#813 裁定的生产默认）。
const STROKE_MODES: [&str; 2] = ["new", "wide"];
const BAR_SECS: i64 = 3600;

/// 对齐容差扫描（秒）。取值与理由**逐字沿用 #940/#944**，否则跨层不可比。
const TOLERANCES: [i64; 5] = [0, 3600, 7200, 21600, 86400];
const PRIMARY_TOL: i64 = 7200;

/// 零假设对照的环形平移量（天）。同 #940/#944。
const NULL_SHIFT_DAYS: [i64; 5] = [29, 57, 89, 113, 151];

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
    #[allow(dead_code)]
    b: Arm, // B 臂保留在 schema 里（与 #944 同输入），本票测量面只用 A/C 两臂
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

/// 一个 L1 线段中枢的测量投影：本体区间 + bar 跨域 + 破位结构。
#[derive(Debug, Clone, Copy)]
struct CenterPt {
    zd: f64,
    zg: f64,
    dd: f64,
    gg: f64,
    /// 中枢 bar 跨域的挂钟区间 [t0, t1)。
    t0: i64,
    t1: i64,
    settled: bool,
    break_dir: BreakDir,
    /// 破位段挂钟区间；未破位 = (0, 0)。
    break_t0: i64,
    break_t1: i64,
}

struct ArmResult {
    n_bars: usize,
    n_strokes: usize,
    n_segments: usize,
    n_bi_centers: usize,
    n_seg_centers: usize,
    n_moves_l1: usize,
    /// 端点序列（（is_top, t0, t1））——同一次跑里复现 #940/#944 端点层读数作交叉校验。
    endpoints: Vec<(bool, i64, i64)>,
    /// L1 线段中枢（生产 `compute_bsps` 同一条结构组装链的中枢本体）。
    centers: Vec<CenterPt>,
}

/// merged 下标 → 挂钟区间。越界返回 None（防御；正常路径不触发）。
fn span_of(m2r: &[(usize, usize)], ts: &[i64], m: usize) -> Option<(i64, i64)> {
    let &(r0, r1) = m2r.get(m)?;
    Some((*ts.get(r0)?, *ts.get(r1)? + BAR_SECS))
}

/// 线段中枢 → 测量投影。`conf_segs` 必须用与 `zhongshu_from_segments` 内部
/// 逐字相同的过滤式（`confirmed && kind == Settled`）重建，否则 seg_start/seg_end/
/// break_seg 的下标语义对不上。
fn to_center_pts(
    zss: &[Zhongshu],
    conf_segs: &[&Segment],
    m2r: &[(usize, usize)],
    ts: &[i64],
) -> Vec<CenterPt> {
    let mut v: Vec<CenterPt> = zss
        .iter()
        .filter_map(|z| {
            let first = conf_segs.get(z.seg_start)?;
            let last = conf_segs.get(z.seg_end)?;
            let (t0, _) = span_of(m2r, ts, first.i0)?;
            let (_, t1) = span_of(m2r, ts, last.i1)?;
            let (break_t0, break_t1) = if z.break_seg >= 0 {
                match conf_segs.get(z.break_seg as usize) {
                    Some(bs) => {
                        let (x0, _) = span_of(m2r, ts, bs.i0)?;
                        let (_, x1) = span_of(m2r, ts, bs.i1)?;
                        (x0, x1)
                    }
                    None => (0, 0),
                }
            } else {
                (0, 0)
            };
            Some(CenterPt {
                zd: z.zd,
                zg: z.zg,
                dd: z.dd,
                gg: z.gg,
                t0,
                t1,
                settled: z.settled,
                break_dir: z.break_direction,
                break_t0,
                break_t1,
            })
        })
        .collect();
    v.sort_by_key(|p| (p.t0, p.t1));
    v
}

fn run_arm(arm: &Arm, mode: &str) -> ArmResult {
    // ── 笔（生产参数：min_strict_sep=5 / reset_dir_on_fractal=false / new_raw_gap_min=3，
    //    与 lib.rs:137-140 pyo3 默认签名一致；同 #940/#944 探针）──
    let mut eng = BiEngine::new(mode, 5, false, 3);
    for i in 0..arm.ts.len() {
        eng.process_bar(arm.o[i], arm.h[i], arm.l[i], arm.c[i]);
    }
    let strokes = eng.current_strokes().to_vec();
    let m2r: Vec<(usize, usize)> = eng.merged_to_raw().to_vec();

    // ── 线段（生产参数 3, true）──
    let segments: Vec<Segment> = segment::segments_from_strokes_v1(&strokes, 3, true);
    let n_seg = segments.len();

    // ── 笔中枢（仅作 arm 统计，与 #944 同口径，用于跨探针对拍）──
    let stroke_tuples: Vec<(usize, usize, f64, f64, bool)> = strokes
        .iter()
        .map(|s| (s.i0, s.i1, s.high, s.low, s.confirmed))
        .collect();
    let bi_centers = zhongshu_from_strokes(&stroke_tuples);

    // ── 线段中枢（逐字复用 p944 run_arm：zhongshu_from_segments 输入六元组）──
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

    // ── L1 走势（orchestrator.rs:891-895：moves_from_zhongshus + attach_persistence）──
    let mut l1_moves = moves_from_zhongshus(&seg_centers, Some(n_seg));
    let pv: Vec<ZsPriceView> = seg_centers
        .iter()
        .map(|z| ZsPriceView { dd: z.dd, gg: z.gg })
        .collect();
    l1_moves = attach_persistence(&l1_moves, &pv);

    // ── 中枢测量投影（过滤式与 zhongshu_from_segments 内部逐字相同）──
    let conf_segs: Vec<&Segment> = segments
        .iter()
        .filter(|s| s.confirmed && s.kind == SegKind::Settled)
        .collect();
    let centers = to_center_pts(&seg_centers, &conf_segs, &m2r, &arm.ts);

    // ── 端点序列（#940 口径，交叉校验用）──
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
        n_strokes: strokes.len(),
        n_segments: n_seg,
        n_bi_centers: bi_centers.len(),
        n_seg_centers: seg_centers.len(),
        n_moves_l1: l1_moves.len(),
        endpoints,
        centers,
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

/// 中枢本体相容 = 价格区间 [zd,zg] 相交（「同区间」）。时间 gap 由 tol 在 align 里判。
fn body_compat(a: &CenterPt, b: &CenterPt) -> bool {
    a.zd.max(b.zd) <= a.zg.min(b.zg)
}

/// 离开结构相容 = 破位方向相同（调用方保证 A 侧已破位；C 侧未破位=None 自然不会等）。
fn break_compat(a: &CenterPt, b: &CenterPt) -> bool {
    a.break_dir == b.break_dir
}

/// 一对一贪心对齐（同 #940/#944：多个候选取 gap 最小，并列取更早）。
/// 返回 (匹配数, A 侧逐点配对到的 C 下标, C 侧逐点是否被认领)。
fn align(
    a: &[CenterPt],
    c: &[CenterPt],
    tol: i64,
    compat: fn(&CenterPt, &CenterPt) -> bool,
) -> (usize, Vec<Option<usize>>, Vec<bool>) {
    let mut used = vec![false; c.len()];
    let mut pair: Vec<Option<usize>> = vec![None; a.len()];
    let mut matched = 0usize;
    for (i, ea) in a.iter().enumerate() {
        let mut best: Option<(i64, usize)> = None;
        for (j, eb) in c.iter().enumerate() {
            if used[j] || !compat(ea, eb) {
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
            pair[i] = Some(j);
            matched += 1;
        }
    }
    (matched, pair, used)
}

/// 端点层对齐（#940 同款，交叉校验用）。
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

/// 环形平移（零假设对照，同 #940/#944）。
fn shift_circ(pts: &[CenterPt], w0: i64, wlen: i64, days: i64) -> Vec<CenterPt> {
    pts.iter()
        .map(|p| {
            let nt0 = w0 + (p.t0 - w0 + days * 86400).rem_euclid(wlen);
            CenterPt {
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

fn center_json(p: &CenterPt, flags: &[(&str, String)]) -> String {
    let mut s = format!(
        "{{\"zd\":{:.6},\"zg\":{:.6},\"dd\":{:.6},\"gg\":{:.6},\"t0\":{},\"t1\":{},\
         \"settled\":{},\"break_dir\":\"{}\",\"break_t0\":{},\"break_t1\":{}",
        p.zd,
        p.zg,
        p.dd,
        p.gg,
        p.t0,
        p.t1,
        p.settled,
        p.break_dir.as_str(),
        p.break_t0,
        p.break_t1
    );
    for (k, v) in flags {
        s.push_str(&format!(",\"{}\":{}", k, v));
    }
    s.push('}');
    s
}

/// 两臂结构对齐的全部读数（中枢本体层 + 离开结构层，分开报）。
struct StructOut {
    json: String,
    // 供合计行用：
    n_a: usize,
    n_c: usize,
    body_matched: usize,
    n_a_broken: usize,
    break_matched: usize,
    joint_same_dir: usize,
    joint_base_broken: usize,
    body_null_mean: f64,
    break_null_mean: f64,
}

fn struct_report(a: &[CenterPt], c: &[CenterPt], w0: i64, wlen: i64) -> StructOut {
    // ── 层 1：中枢本体对齐（主容差）──
    let (body_m, body_pair, body_c_used) = align(a, c, PRIMARY_TOL, body_compat);

    // 容差扫描（本体层），每档附环形平移零假设
    let mut tol_rows: Vec<String> = Vec::new();
    for tol in TOLERANCES {
        let (m_ac, _, u_c) = align(a, c, tol, body_compat);
        let mut n_null = 0usize;
        for d in NULL_SHIFT_DAYS {
            let sh = shift_circ(a, w0, wlen, d);
            n_null += align(&sh, c, tol, body_compat).0;
        }
        tol_rows.push(format!(
            "{{\"tol_s\":{},\"body_ac\":{},\"c_unclaimed\":{},\"null_body_mean\":{:.2}}}",
            tol,
            m_ac,
            u_c.iter().filter(|x| !**x).count(),
            n_null as f64 / NULL_SHIFT_DAYS.len() as f64
        ));
    }

    // ── 层 2：离开结构对齐（主容差）──
    let a_broken: Vec<CenterPt> = a
        .iter()
        .copied()
        .filter(|p| p.break_dir != BreakDir::None)
        .collect();
    let (brk_m, _, _) = align(&a_broken, c, PRIMARY_TOL, break_compat);
    let mut n_null_brk = 0usize;
    for d in NULL_SHIFT_DAYS {
        let sh = shift_circ(&a_broken, w0, wlen, d);
        n_null_brk += align(&sh, c, PRIMARY_TOL, break_compat).0;
    }
    let break_null_mean = n_null_brk as f64 / NULL_SHIFT_DAYS.len() as f64;

    // 条件读数：本体已配对的中枢对里，破位方向一致的比例
    // （joint_base_broken 只数 A 侧已破位的对——未破位中枢没有离开结构可比对）。
    let mut joint_same_dir = 0usize;
    let mut joint_base_broken = 0usize;
    for (i, pj) in body_pair.iter().enumerate() {
        if let Some(j) = pj {
            if a[i].break_dir != BreakDir::None {
                joint_base_broken += 1;
                if a[i].break_dir == c[*j].break_dir {
                    joint_same_dir += 1;
                }
            }
        }
    }

    // 本体层零假设（主容差）
    let mut n_null_body = 0usize;
    for d in NULL_SHIFT_DAYS {
        let sh = shift_circ(a, w0, wlen, d);
        n_null_body += align(&sh, c, PRIMARY_TOL, body_compat).0;
    }
    let body_null_mean = n_null_body as f64 / NULL_SHIFT_DAYS.len() as f64;

    // 逐中枢明细（带配对标志）
    let a_list: Vec<String> = a
        .iter()
        .enumerate()
        .map(|(i, p)| center_json(p, &[("body_in_C", body_pair[i].is_some().to_string())]))
        .collect();
    let c_list: Vec<String> = c
        .iter()
        .enumerate()
        .map(|(i, p)| center_json(p, &[("claimed_by_A", body_c_used[i].to_string())]))
        .collect();

    StructOut {
        json: format!(
            "{{\"n_A\":{},\"n_C\":{},\"n_A_broken\":{},\"n_C_broken\":{},\
             \"body\":{{\"matched\":{},\"null_mean\":{:.2}}},\
             \"break\":{{\"matched\":{},\"null_mean\":{:.2}}},\
             \"joint\":{{\"same_dir\":{},\"base_broken\":{}}},\
             \"tol_scan\":[{}],\"A\":[{}],\"C\":[{}]}}",
            a.len(),
            c.len(),
            a_broken.len(),
            c.iter().filter(|p| p.break_dir != BreakDir::None).count(),
            body_m,
            body_null_mean,
            brk_m,
            break_null_mean,
            joint_same_dir,
            joint_base_broken,
            tol_rows.join(","),
            a_list.join(","),
            c_list.join(",")
        ),
        n_a: a.len(),
        n_c: c.len(),
        body_matched: body_m,
        n_a_broken: a_broken.len(),
        break_matched: brk_m,
        joint_same_dir,
        joint_base_broken,
        body_null_mean,
        break_null_mean,
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("用法: p946_seg_structure_alignment <inputs.json> <metrics_out.json>");
        std::process::exit(2);
    }
    let raw = std::fs::read_to_string(&args[1]).expect("读取输入 JSON 失败");
    let inputs: Inputs = serde_json::from_str(&raw).expect("解析输入 JSON 失败");

    let mut out = String::from("{\n  \"records\": [\n");
    let mut first = true;

    for mode in STROKE_MODES {
        // 合计行（全标的）：主读数口径下的分子分母
        let (mut t_n_a, mut t_n_c) = (0usize, 0usize);
        let (mut t_body, mut t_a_broken, mut t_brk) = (0usize, 0usize, 0usize);
        let (mut t_joint_same, mut t_joint_base) = (0usize, 0usize);
        let (mut t_body_null, mut t_brk_null) = (0f64, 0f64);
        let mut n_sym = 0usize;

        for sym in &inputs.symbols {
            let ra = run_arm(&sym.a, mode);
            let rc = run_arm(&sym.c, mode);
            let w0 = sym.window_start_utc;
            let wlen = (sym.window_end_utc - sym.window_start_utc).max(1);

            let s = struct_report(&ra.centers, &rc.centers, w0, wlen);

            // 端点层交叉校验（#940/#944 同口径；主容差）
            let ep_ac = align_ep(&ra.endpoints, &rc.endpoints, PRIMARY_TOL);

            let arm_json = |r: &ArmResult| {
                format!(
                    "{{\"n_bars\":{},\"n_strokes\":{},\"n_segments\":{},\
                     \"n_bi_centers\":{},\"n_seg_centers\":{},\"n_moves_l1\":{},\
                     \"n_endpoints\":{}}}",
                    r.n_bars,
                    r.n_strokes,
                    r.n_segments,
                    r.n_bi_centers,
                    r.n_seg_centers,
                    r.n_moves_l1,
                    r.endpoints.len()
                )
            };

            if !first {
                out.push_str(",\n");
            }
            first = false;
            out.push_str(&format!(
                "    {{\"symbol\":\"{}\",\"stroke_mode\":\"{}\",\"window\":[{},{}],\
                 \"n_sessions\":{},\"primary_tol_s\":{},\
                 \"arms\":{{\"A\":{},\"C\":{}}},\
                 \"endpoint_xcheck\":{{\"n_A\":{},\"n_C\":{},\"ac\":{}}},\
                 \"struct\":{}}}",
                sym.symbol,
                mode,
                sym.window_start_utc,
                sym.window_end_utc,
                sym.n_sessions_in_window,
                PRIMARY_TOL,
                arm_json(&ra),
                arm_json(&rc),
                ra.endpoints.len(),
                rc.endpoints.len(),
                ep_ac,
                s.json
            ));

            println!(
                "P946 {sym} mode={mode} | seg_zs A/C = {na}/{nc} (broken {nab}) \
                 | body {bm}/{na} | break {brkm}/{nab} | joint {js}/{jb} | ep A→C {ep}/{nep}",
                sym = sym.symbol,
                mode = mode,
                na = s.n_a,
                nc = s.n_c,
                nab = s.n_a_broken,
                bm = s.body_matched,
                brkm = s.break_matched,
                js = s.joint_same_dir,
                jb = s.joint_base_broken,
                ep = ep_ac,
                nep = ra.endpoints.len()
            );

            t_n_a += s.n_a;
            t_n_c += s.n_c;
            t_body += s.body_matched;
            t_a_broken += s.n_a_broken;
            t_brk += s.break_matched;
            t_joint_same += s.joint_same_dir;
            t_joint_base += s.joint_base_broken;
            t_body_null += s.body_null_mean;
            t_brk_null += s.break_null_mean;
            n_sym += 1;
        }

        // 合计行（零假设 = 逐标的零假设均值之平均，与逐标的口径分开标注）
        out.push_str(&format!(
            ",\n    {{\"symbol\":\"__TOTAL__\",\"stroke_mode\":\"{}\",\"n_symbols\":{},\
             \"primary_tol_s\":{},\"n_A\":{},\"n_C\":{},\"n_A_broken\":{},\
             \"body\":{{\"matched\":{},\"null_mean_of_means\":{:.2}}},\
             \"break\":{{\"matched\":{},\"null_mean_of_means\":{:.2}}},\
             \"joint\":{{\"same_dir\":{},\"base_broken\":{}}}}}",
            mode,
            n_sym,
            PRIMARY_TOL,
            t_n_a,
            t_n_c,
            t_a_broken,
            t_body,
            t_body_null / n_sym.max(1) as f64,
            t_brk,
            t_brk_null / n_sym.max(1) as f64,
            t_joint_same,
            t_joint_base
        ));
        println!(
            "P946 TOTAL mode={mode} | body {t_body}/{t_n_a} | break {t_brk}/{t_a_broken} \
             | joint {t_joint_same}/{t_joint_base}"
        );
    }
    out.push_str("\n  ]\n}\n");
    std::fs::write(&args[2], out).expect("写 metrics JSON 失败");
    eprintln!("metrics -> {}", args[2]);
}
