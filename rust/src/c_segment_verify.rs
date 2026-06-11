//! C 段边界修复（修复A + B2）的 OKLO 447K 真实数据验证（L2 认识论等级）。
//!
//! 验证对象：engine_bsp_gap_diagnosis 的两项修复
//!   - 修复A（level.rs）：递归层 move seg_end 与 level-1 扩展语义对齐
//!     （非末组 = 下一组首中枢 comp_start − 1，末组 = num_components − 1）。
//!   - 修复B2（divergence.rs）：趋势背驰 C 段不被 mv.seg_end 截断，
//!     搜索窗口延伸到下一 settled 中枢 seg_end，C 段终点 = 窗口内趋势极值段。
//!
//! 期望（修复前 OKLO 447K 实证基线，见 .chanlun 谱系 project_bsp_gap_root_cause）。
//! 层级口径 = ladder（fugue_version_i）：ladder2 = 笔中枢级（构造标签 level_id=1）、
//! ladder3 = 走势级（engine level 1）、ladder4+ = 递归层（engine level ≥2）：
//!   - 递归层 type1 从结构性恒空（16/16 c_empty）变为非空；
//!   - ladder2 type1 ≫ 1（修复前 1 个）、type2 > 0（修复前 0）；
//!   - ladder4（递归 L2）出现 type1（修复前 0，全 type3）。
//!
//! 运行（长测试，默认 ignore）：
//!   cargo test --release c_segment_oklo -- --ignored --nocapture
//! 数据：analysis/data_cache/oklo_447k_ohlc.f64（<4d LE per bar，由
//!   oklo_1m_databento.json 一次性转换）。

#![cfg(test)]

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::buysellpoint::{buysellpoints_from_level, BspKind, BuySellPoint};
use crate::divergence::{divergences_from_moves_v1, MoveView, SegView, ZsView};
use crate::moves::Move;
use crate::orchestrator::RecursiveOrchestrator;

fn load_ohlc() -> Vec<[f64; 4]> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../analysis/data_cache/oklo_447k_ohlc.f64"
    );
    let raw = fs::read(path).expect("缺少 oklo_447k_ohlc.f64（由 oklo_1m_databento.json 转换）");
    assert_eq!(raw.len() % 32, 0, "文件长度必须是 32 字节/bar 的整数倍");
    raw.chunks_exact(32)
        .map(|c| {
            let f = |i: usize| f64::from_le_bytes(c[i * 8..i * 8 + 8].try_into().unwrap());
            [f(0), f(1), f(2), f(3)]
        })
        .collect()
}

/// per_level_bsp.py 适配器的 Rust 复刻：递归层 N≥2 的 BSP。
///
/// segments = 前一级别 moves（i0=first_seg_s0, i1=last_seg_s1, high/low/direction 透传），
/// zhongshus = 本级别 LevelZhongshu（seg_start=comp_start, seg_end=comp_end,
/// break_seg=break_comp），moves = 本级别 Move，df_macd=None（拓扑代理力度，521号边界）。
fn level_bsps(
    prev_moves: &[Move],
    zss: &[crate::level::LevelZhongshu],
    mvs: &[Move],
    level_id: i64,
) -> Vec<BuySellPoint> {
    let segs: Vec<SegView> = prev_moves
        .iter()
        .map(|m| SegView {
            direction: m.direction,
            high: m.high,
            low: m.low,
            i0: m.first_seg_s0,
            i1: m.last_seg_s1,
        })
        .collect();
    let zs_views: Vec<ZsView> = zss
        .iter()
        .map(|z| ZsView {
            zd: z.zd,
            zg: z.zg,
            seg_start: z.comp_start,
            seg_end: z.comp_end,
            settled: z.settled,
        })
        .collect();
    let zs_break: Vec<(bool, crate::zhongshu::BreakDir, i64)> = zss
        .iter()
        .map(|z| (z.settled, z.break_direction, z.break_comp))
        .collect();
    let move_views: Vec<MoveView> = mvs
        .iter()
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
        .collect();
    let divs = divergences_from_moves_v1(&segs, &zs_views, &move_views, level_id, None);
    buysellpoints_from_level(&segs, &zs_views, &zs_break, &move_views, &divs, level_id)
}

/// (kind, confirmed) 计数。
fn count_bsps(bsps: &[BuySellPoint]) -> BTreeMap<(&'static str, bool), usize> {
    let mut c: BTreeMap<(&'static str, bool), usize> = BTreeMap::new();
    for b in bsps {
        *c.entry((b.kind.as_str(), b.confirmed)).or_insert(0) += 1;
    }
    c
}

fn kind_total(bsps: &[BuySellPoint], kind: BspKind) -> usize {
    bsps.iter().filter(|b| b.kind == kind).count()
}

/// 笔中枢级（ladder2）BSP——复刻 lib.rs `current_bi_zhongshu_buysellpoints` 全链：
/// confirmed 笔 → zhongshu_from_strokes → moves_from_zhongshus(num_segments=len)
/// → divergences_from_moves_v1(df_macd=None) → buysellpoints_from_level。
fn bi_zhongshu_bsps(orch: &RecursiveOrchestrator) -> Vec<BuySellPoint> {
    let confirmed: Vec<&crate::stroke::Stroke> =
        orch.strokes().iter().filter(|s| s.confirmed).collect();
    if confirmed.len() < 3 {
        return Vec::new();
    }
    let zs_in: Vec<(usize, usize, f64, f64, bool)> = confirmed
        .iter()
        .map(|s| (s.i0, s.i1, s.high, s.low, s.confirmed))
        .collect();
    let zhongshus = crate::zhongshu::zhongshu_from_strokes(&zs_in);
    let moves = crate::moves::moves_from_zhongshus(&zhongshus, Some(confirmed.len()));
    let segs: Vec<SegView> = confirmed
        .iter()
        .map(|s| SegView {
            direction: s.direction,
            high: s.high,
            low: s.low,
            i0: s.i0,
            i1: s.i1,
        })
        .collect();
    let zss: Vec<ZsView> = zhongshus
        .iter()
        .map(|z| ZsView {
            zd: z.zd,
            zg: z.zg,
            seg_start: z.seg_start,
            seg_end: z.seg_end,
            settled: z.settled,
        })
        .collect();
    let zsb: Vec<(bool, crate::zhongshu::BreakDir, i64)> = zhongshus
        .iter()
        .map(|z| (z.settled, z.break_direction, z.break_seg))
        .collect();
    let mvs: Vec<MoveView> = moves
        .iter()
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
        .collect();
    let divs = divergences_from_moves_v1(&segs, &zss, &mvs, 1, None);
    buysellpoints_from_level(&segs, &zss, &zsb, &mvs, &divs, 1)
}

#[test]
#[ignore = "长测试：OKLO 447K 全量逐 bar，显式运行"]
fn c_segment_oklo_447k_per_level_bsp() {
    let bars = load_ohlc();
    assert_eq!(bars.len(), 447_739, "OKLO 447K 全时段");

    // 生产同口径配置（fugue_version_i.py: max_levels=8，其余 PyO3 默认）。
    let mut orch = RecursiveOrchestrator::new(8, "wide", 5, false, 3, false, true);
    for b in &bars {
        orch.process_bar(b[0], b[1], b[2], b[3]);
    }

    let mut report = String::from("{\n");
    report.push_str(&format!("  \"n_bars\": {},\n", bars.len()));
    report.push_str("  \"levels\": {\n");

    // ── ladder2（笔中枢级，构造标签 level_id=1）──
    let mut per_level: Vec<(String, Vec<BuySellPoint>)> =
        vec![("ladder2_bi_zhongshu".into(), bi_zhongshu_bsps(&orch))];

    // ── ladder3（走势级，engine level 1）：引擎内置 BSP ──
    per_level.push(("ladder3_L1".into(), orch.buysellpoints().to_vec()));

    // ── ladder≥4（递归层 N≥2）：per_level_bsp 适配器路径 ──
    let snaps = orch.recursive().to_vec();
    let l1_moves = orch.moves().to_vec();
    for (i, snap) in snaps.iter().enumerate() {
        let prev_moves: &[Move] = if i == 0 { &l1_moves } else { &snaps[i - 1].moves };
        let bsps = level_bsps(prev_moves, &snap.zhongshus, &snap.moves, snap.level_id);
        per_level.push((format!("ladder{}_L{}", snap.level_id + 2, snap.level_id), bsps));
    }

    let n_levels = per_level.len();
    for (idx, (label, bsps)) in per_level.iter().enumerate() {
        let counts = count_bsps(bsps);
        println!("== {label}（共 {} BSP）==", bsps.len());
        report.push_str(&format!("    \"{label}\": {{\n"));
        let n_entries = counts.len();
        for (j, ((kind, confirmed), n)) in counts.iter().enumerate() {
            println!("  {kind} confirmed={confirmed}: {n}");
            let comma = if j + 1 < n_entries { "," } else { "" };
            report.push_str(&format!(
                "      \"{kind}_confirmed_{confirmed}\": {n}{comma}\n"
            ));
        }
        let comma = if idx + 1 < n_levels { "," } else { "" };
        report.push_str(&format!("    }}{comma}\n"));
    }
    report.push_str("  }\n}\n");

    let out = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../analysis/data_cache/c_segment_fix_rust_verify.json"
    ));
    fs::write(out, &report).expect("写验证结果失败");
    println!("已落盘: {}", out.display());

    // ── 判据（任务卡预注册，ladder 口径）──
    let find = |label: &str| {
        per_level
            .iter()
            .find(|(l, _)| l.starts_with(label))
            .map(|(_, b)| b)
    };
    let ladder2 = find("ladder2").expect("笔中枢级必须存在");
    let l2_t1 = kind_total(ladder2, BspKind::Type1);
    let l2_t2 = kind_total(ladder2, BspKind::Type2);
    assert!(l2_t1 > 100, "ladder2 type1 应 >100（修复前=1，实测374），实际 {l2_t1}");
    assert!(l2_t2 > 0, "ladder2 type2 应 >0（修复前=0），实际 {l2_t2}");
    let ladder4 = find("ladder4").expect("ladder4（递归 L2）必须涌现");
    let l4_t1 = kind_total(ladder4, BspKind::Type1);
    assert!(l4_t1 > 0, "ladder4 type1 应 >0（修复前=0 全 type3），实际 {l4_t1}");
}

/// tranche 重验（任务卡【tranche递归建仓重验】）：
/// 旧结论（`_rev_tranche_path2_section.md`）——tranche 目标区间
/// `(home, entry_ladder−1]`，entry_ladder = 进场 bar 最高 **confirmed type1 BUY**
/// 的 ladder（= buy1 掩码最高位）。旧 buy1 ladder 集合恒为 {2,3} ⟹ entry≤3 ⟹
/// 区间 (2,2]=∅，是 buy1≤3 的定理。本测捕获 **(kind, side, confirmed) 三元计数**
/// （上一测仅 (kind,confirmed) 折叠了 side），直接判定 ladder≥4 是否存在
/// confirmed type1 buy（= buy1 掩码进位到 ≥4 ⟹ entry 上限 3→4 ⟹ 区间 (2,3] 非空）。
///
/// 与生产信号层 `organic_signals.compute_organic_signals` 同口径：buy1[ladder]
/// 由 `_scan_events_rust` 在该 ladder 出现 **confirmed type1 buy** BSP 时置位
/// （`_level_bsps_with_divs`→`buysellpoints_from_level`，与本测 `level_bsps` 同链）。
/// 纯 Rust 闭环（无 Python/PyO3）。
#[test]
#[ignore = "长测试：OKLO 447K 全量逐 bar，显式运行"]
fn c_segment_oklo_447k_tranche_side_split() {
    let bars = load_ohlc();
    assert_eq!(bars.len(), 447_739, "OKLO 447K 全时段");

    let mut orch = RecursiveOrchestrator::new(8, "wide", 5, false, 3, false, true);
    for b in &bars {
        orch.process_bar(b[0], b[1], b[2], b[3]);
    }

    // ladder → BSP 集合（与上一测同口径构造）。
    let mut per_level: Vec<(usize, Vec<BuySellPoint>)> =
        vec![(2, bi_zhongshu_bsps(&orch)), (3, orch.buysellpoints().to_vec())];
    let snaps = orch.recursive().to_vec();
    let l1_moves = orch.moves().to_vec();
    for (i, snap) in snaps.iter().enumerate() {
        let prev_moves: &[Move] = if i == 0 { &l1_moves } else { &snaps[i - 1].moves };
        let bsps = level_bsps(prev_moves, &snap.zhongshus, &snap.moves, snap.level_id);
        per_level.push(((snap.level_id + 2) as usize, bsps));
    }

    // confirmed type1 BUY 计数（= buy1 掩码源），逐 ladder。
    let mut buy1_ladders: Vec<usize> = Vec::new();
    let mut report = String::from("{\n");
    report.push_str(&format!("  \"n_bars\": {},\n", bars.len()));
    report.push_str("  \"per_ladder\": {\n");
    let n = per_level.len();
    for (idx, (ladder, bsps)) in per_level.iter().enumerate() {
        let cnt = |k: BspKind, s: crate::buysellpoint::Side, conf: bool| {
            bsps.iter()
                .filter(|b| b.kind == k && b.side == s && b.confirmed == conf)
                .count()
        };
        let t1_buy_conf = cnt(BspKind::Type1, crate::buysellpoint::Side::Buy, true);
        let t1_sell_conf = cnt(BspKind::Type1, crate::buysellpoint::Side::Sell, true);
        let t2_buy_conf = cnt(BspKind::Type2, crate::buysellpoint::Side::Buy, true);
        if t1_buy_conf > 0 {
            buy1_ladders.push(*ladder);
        }
        println!(
            "ladder{ladder}: type1_buy_confirmed={t1_buy_conf} \
             type1_sell_confirmed={t1_sell_conf} type2_buy_confirmed={t2_buy_conf}"
        );
        let comma = if idx + 1 < n { "," } else { "" };
        report.push_str(&format!(
            "    \"ladder{ladder}\": {{\"type1_buy_confirmed\": {t1_buy_conf}, \
             \"type1_sell_confirmed\": {t1_sell_conf}, \
             \"type2_buy_confirmed\": {t2_buy_conf}}}{comma}\n"
        ));
    }
    report.push_str("  },\n");
    let entry_cap = buy1_ladders.iter().max().copied().unwrap_or(0);
    let tranche_lo = 2usize; // home = floor = LADDER_SEG（旧报告 rev home k=2）
    let tranche_nonempty = entry_cap > tranche_lo + 1; // 区间 (lo, entry−1] 非空 ⟺ entry−1 > lo
    let tranche_layers = entry_cap.saturating_sub(tranche_lo + 1);
    report.push_str(&format!("  \"buy1_ladders\": {buy1_ladders:?},\n"));
    report.push_str(&format!("  \"entry_cap\": {entry_cap},\n"));
    report.push_str(&format!("  \"tranche_interval\": \"({tranche_lo}, {}]\",\n",
        entry_cap.saturating_sub(1)));
    report.push_str(&format!("  \"tranche_nonempty\": {tranche_nonempty},\n"));
    report.push_str(&format!("  \"tranche_layers\": {tranche_layers}\n"));
    report.push_str("}\n");
    println!("buy1_ladders={buy1_ladders:?} entry_cap={entry_cap} \
              tranche_nonempty={tranche_nonempty} layers={tranche_layers}");

    let out = Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../analysis/data_cache/tranche_recheck_side_split.json"
    ));
    fs::write(out, &report).expect("写 tranche 重验结果失败");
    println!("已落盘: {}", out.display());

    // 判据：旧定理 buy1⊆{2,3} 是否被打破（entry 上限是否 3→4）。
    assert!(
        buy1_ladders.contains(&4),
        "tranche 解锁需 ladder4 出现 confirmed type1 buy（旧恒空），实际 buy1_ladders={buy1_ladders:?}"
    );
}
