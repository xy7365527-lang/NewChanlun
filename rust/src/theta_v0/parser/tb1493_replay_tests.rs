//! #1493 验收（重型验证窗口，ignored）：BTC 1m 全量与尾窗回放下的段层产出、全量 ≡ 逐 bar 增量、
//! 前缀确认不回退、fail-closed 计数与 #1487 六处现场。数据不入仓：
//! `analysis/data_cache/btc_1m_full.json`（sha256 `16ea13d5…707b`，先校验）。
//! 运行：`cargo test --release --lib tb1493_replay -- --ignored --nocapture --test-threads=1`。
use super::*;
use crate::theta_v0::backtest::data::{data_dir, load_symbol};

const FULL_SHA256: &str = "16ea13d55f2ae7edcfc503f604a14961fd1a378227afd37c63f23386e894707b";

fn load_full() -> Vec<Bar> {
    use sha2::{Digest, Sha256};
    let path = data_dir().join("btc_1m_full.json");
    let bytes = std::fs::read(&path).expect("缺 analysis/data_cache/btc_1m_full.json");
    let sha: String = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(sha, FULL_SHA256, "数据不是 #1487 诊断口径的全量文件");
    drop(bytes);
    load_symbol(&path, "BTCUSDT-1m", &ThetaConfig::default(), 60)
        .unwrap()
        .bars
}

#[derive(Default, Debug)]
struct Replay {
    /// 每根 bar 末的段数（含末段）。
    seg_counts: Vec<usize>,
    /// 段数增加发生的 bar 下标。
    emits: Vec<usize>,
    invalid_bars: usize,
    /// 前缀不回退违例：(bar, 首个改写段下标, 改写前一步是否有未封闭第二种候选)。
    rewrites: Vec<(usize, usize, bool)>,
}

/// 逐 bar 增量回放；`ladder` 中的前缀处对拍全量（segments/strokes/tail + fail-closed 起点）。
fn replay(bars: &[Bar], ladder: &[usize]) -> (Replay, ParseLayer) {
    let theta = ThetaConfig::default();
    let mut inc = ParseLayerIncr::new(&theta);
    let mut r = Replay::default();
    let mut prev: Option<ParseLayer> = None;
    let mut last = ParseLayer::default();
    for (i, bar) in bars.iter().enumerate() {
        let layer = inc.append(*bar);
        if inc.segments_parse_invalid_from().is_some() {
            r.invalid_bars += 1;
        }
        if let Some(p) = &prev {
            // 已发射段（除末段可随末笔改写）须是后一前缀的前缀；只比保留前缀之后的区间。
            let k = layer.segments_confirmed_len.min(p.segments.len());
            let old = &p.segments[k..p.segments.len().saturating_sub(1)];
            let new = layer.segments.get(k..k + old.len());
            if new != Some(old) {
                let j = (0..old.len())
                    .find(|&j| layer.segments.get(k + j) != Some(&old[j]))
                    .unwrap_or(old.len());
                r.rewrites
                    .push((i, k + j, p.segments_earliest_unsealed.is_some()));
            }
            if layer.segments.len() > p.segments.len() {
                r.emits.push(i);
            }
        }
        r.seg_counts.push(layer.segments.len());
        let n = i + 1;
        if ladder.contains(&n) {
            // 全量走生产入口；段事实在同一批笔上取（inclusion 事实路径随前缀二次增长，见内存探针）。
            let full = parse_layer(&bars[..n], &theta);
            assert_eq!(layer.strokes, full.strokes, "前缀 {n}：笔");
            assert_eq!(layer.segments, full.segments, "前缀 {n}：段");
            assert_eq!(layer.tail, full.tail, "前缀 {n}：尾部");
            let (fsegs, _, sf) = segment::divide_segments_with_facts(&full.strokes, &theta.parse);
            assert_eq!(&fsegs, &*full.segments, "前缀 {n}：观察路径同一判定");
            assert_eq!(
                inc.segments_parse_invalid_from(),
                sf.parse_invalid_from.as_ref().map(|p| p.seg_start),
                "前缀 {n}：fail-closed 起点"
            );
            eprintln!(
                "TB1493 LADDER n={n} strokes={} segments={} pending={:?} failed_geometry={} parse_invalid={:?}",
                full.strokes.len(),
                full.segments.len(),
                full.tail.iter().find_map(|t| match t {
                    crate::theta_v0::types::PendingTail::PendingSegment { start_index, .. } => {
                        Some(*start_index)
                    }
                    _ => None,
                }),
                sf.candidates.iter().filter(|c| c.result == "FAILED_GEOMETRY").count(),
                sf.parse_invalid_from.as_ref().map(|p| p.seg_start)
            );
        }
        prev = Some(layer.clone());
        last = layer;
    }
    (r, last)
}

/// 每 `bucket` 根分桶的段数增量、最长连续零桶、最长空窗（相邻两次出段之间的 bar 数）。
fn report(tag: &str, r: &Replay, n: usize, bucket: usize) -> (usize, usize) {
    let buckets: Vec<usize> = (0..n.div_ceil(bucket))
        .map(|b| {
            r.emits
                .iter()
                .filter(|&&e| e >= b * bucket && e < (b + 1) * bucket)
                .count()
        })
        .collect();
    let mut run = 0;
    let mut max_zero_run = 0;
    for &c in &buckets {
        run = if c == 0 { run + 1 } else { 0 };
        max_zero_run = max_zero_run.max(run);
    }
    let mut gaps: Vec<(usize, usize)> = r
        .emits
        .windows(2)
        .map(|w| (w[1] - w[0], w[0]))
        .collect();
    gaps.push((n - r.emits.last().copied().unwrap_or(0), r.emits.last().copied().unwrap_or(0)));
    gaps.sort_unstable_by(|a, b| b.cmp(a));
    eprintln!(
        "TB1493 {tag} bars={n} emits={} buckets({bucket})={buckets:?} max_zero_run={max_zero_run} top_gaps={:?} invalid_bars={} rewrites={} rewrites_without_second_kind={}",
        r.emits.len(),
        &gaps[..gaps.len().min(5)],
        r.invalid_bars,
        r.rewrites.len(),
        r.rewrites.iter().filter(|x| !x.2).count(),
    );
    for x in r.rewrites.iter().filter(|x| !x.2).take(20) {
        eprintln!("TB1493 {tag} REWRITE bar={} seg={}", x.0, x.1);
    }
    (max_zero_run, gaps[0].0)
}

/// 全量 facts 读数：FAILED_GEOMETRY、fail-closed、流首段起点位移、指定笔所在候选。
fn facts_report(tag: &str, layer: &ParseLayer, sites: &[usize]) {
    let (_, _, sf) = segment::divide_segments_with_facts(&layer.strokes, &ThetaConfig::default().parse);
    let seed = sf.seeds.last().map(|s| s.start_stroke);
    let failed = sf
        .candidates
        .iter()
        .filter(|c| c.result == "FAILED_GEOMETRY")
        .count();
    eprintln!(
        "TB1493 {tag} FACTS strokes={} segments={} failed_geometry={failed} parse_invalid={:?} first_seed={seed:?} first_start={:?} last_segment_end_bar={:?} last_stroke_end_bar={:?}",
        layer.strokes.len(),
        layer.segments.len(),
        sf.parse_invalid_from.as_ref().map(|p| (p.seg_start, p.end_stroke)),
        sf.candidates.first().map(|c| c.start_stroke),
        layer.segments.last().map(|s| s.end_index),
        layer.strokes.last().map(|s| s.end_index)
    );
    for &s in sites {
        let c = sf.candidates.iter().rev().find(|c| c.start_stroke <= s);
        if let Some(c) = c {
            let states: Vec<_> = c
                .first
                .checks
                .iter()
                .filter(|k| k.elements[1].stroke_idx >= s.saturating_sub(4))
                .map(|k| (k.elements[1].stroke_idx, k.turn_hypothesis_state))
                .take(6)
                .collect();
            eprintln!(
                "TB1493 {tag} SITE stroke={s} cand_start={} result={} end={:?} turn={:?} checks_near={states:?}",
                c.start_stroke, c.result, c.end_stroke, c.turn_hypothesis_state
            );
        }
    }
    assert_eq!(failed, 0, "{tag}：FAILED_GEOMETRY 须为 0");
    assert!(sf.parse_invalid_from.is_none(), "{tag}：parse_invalid_from 须为 None");
}

#[test]
#[ignore = "重型验证窗口：btc_1m_full.json 尾 5 万根"]
fn tb1493_replay_tail_window_ladder() {
    let all = load_full();
    let tail = &all[all.len() - 50_000..];
    let bars = &tail[5..]; // start_offset=5（#1419 L3 / #1405 口径）
    let (r, last) = replay(bars, &[5_000, 10_000, 20_000, 30_000, 40_000, bars.len()]);
    let ladder: Vec<_> = [5_000, 10_000, 20_000, 30_000, 40_000, bars.len()]
        .iter()
        .map(|&n| r.seg_counts[n - 1])
        .collect();
    eprintln!("TB1493 TAIL ladder_segments={ladder:?}");
    // #1487 现场：尾窗 5k–36k（笔 370）、38k–43k（笔 3040）、x1420 尾窗 7k–29k（笔 508）期间须出段。
    for (lo, hi) in [(5_000, 36_000), (38_000, 43_000), (7_000, 29_000)] {
        let k = r.emits.iter().filter(|&&e| e >= lo && e < hi).count();
        eprintln!("TB1493 TAIL window {lo}-{hi} emits={k}");
        assert!(k > 0, "尾窗 {lo}–{hi} 段层停产");
    }
    report("TAIL", &r, bars.len(), 5_000);
    facts_report("TAIL", &last, &[370, 508, 3040]);
    assert_eq!(r.invalid_bars, 0);
}

#[test]
#[ignore = "重型验证窗口：btc_1m_full.json 全量"]
fn tb1493_replay_full_history() {
    let all = load_full();
    // 从 raw 1 起（票面口径）：raw 0/1 两根同价互含，首对包含使初始方向永久未定（#1393 范围）。
    let bars = &all[1..];
    let n = bars.len();
    let (r, last) = replay(bars, &[n]);
    let (max_zero_run, longest_gap) = report("FULL", &r, n, 250_000);
    // #1487 现场：v1405/x1420 末段现场（笔 59848）、v1405 第一次长停（笔 3178）、x1420 第一次长停（raw 70,351）。
    for raw in [70_351usize, 787_209] {
        let k = r
            .emits
            .iter()
            .filter(|&&e| e + 1 >= raw && e + 1 < raw + 100_000)
            .count();
        eprintln!("TB1493 FULL after_raw={raw} emits_in_100k={k}");
    }
    facts_report("FULL", &last, &[3178, 59848]);
    eprintln!(
        "TB1493 FULL last_segment_end={:?} n={n} longest_gap={longest_gap}",
        last.segments.last().map(|s| s.end_index)
    );
    assert_eq!(r.invalid_bars, 0);
    assert!(max_zero_run <= 1, "每 25 万根分桶不得有连续零桶");
}

fn vm_hwm_mb() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmRSS:"))
                .and_then(|l| l.split_whitespace().nth(1)?.parse::<u64>().ok())
        })
        .unwrap_or(0)
        / 1024
}

#[test]
#[ignore = "诊断：facts 路径内存随前缀增长"]
fn tb1493_facts_memory_probe() {
    let all = load_full();
    let tail = &all[all.len() - 50_000..];
    let bars = &tail[5..];
    for n in [2_000usize, 5_000, 10_000, 20_000] {
        let (layer, facts) = parse_layer_with_inclusion_facts(&bars[..n], &ThetaConfig::default());
        let sf = facts.segment_facts.as_ref().unwrap();
        let first_checks: usize = sf.candidates.iter().map(|c| c.first.checks.len()).sum();
        let first_steps: usize = sf.candidates.iter().map(|c| c.first.steps.len()).sum();
        let idx_first: usize = sf
            .candidates
            .iter()
            .flat_map(|c| c.first.checks.iter().flat_map(|k| k.elements.iter()))
            .map(|e| e.stroke_indices.len())
            .sum();
        let second: usize = sf
            .candidates
            .iter()
            .filter_map(|c| c.second.as_ref())
            .map(|s| s.steps.len())
            .sum();
        let idx_second: usize = sf
            .candidates
            .iter()
            .filter_map(|c| c.second.as_ref())
            .flat_map(|s| s.steps.iter())
            .map(|st| st.after.stroke_indices.len())
            .sum();
        let attempts: usize = sf.candidates.iter().map(|c| c.termination_attempts.len()).sum();
        eprintln!(
            "TB1493 MEM n={n} rss_mb={} strokes={} cands={} first_checks={first_checks} first_steps={first_steps} idx_first={idx_first} second_steps={second} idx_second={idx_second} attempts={attempts} seeds={}",
            vm_hwm_mb(),
            layer.strokes.len(),
            sf.candidates.len(),
            sf.seeds.len()
        );
    }
}
