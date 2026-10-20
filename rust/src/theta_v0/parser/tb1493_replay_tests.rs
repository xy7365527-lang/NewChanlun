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
    /// fail-closed 事件数（`parse_invalid_from` 由 None 变 Some 或起点改变的次数）。#1493 裁定 1 残余
    /// 形态（早先第二种候选作废后再判的段端仍越过段起点）只能以门失败出现，故即其计数。
    invalid_events: usize,
    /// 前缀不回退违例：(bar, 首个改写段下标, 是否「末笔可改写」类)。#1493 裁定 1：只容许末笔可改写
    /// ——上一前缀的末笔在本 bar 被改写或撤回，且被改写的是上一前缀的末个已确认段。
    rewrites: Vec<(usize, usize, bool)>,
}

/// 逐 bar 增量回放；`ladder` 中的前缀处对拍全量（segments/strokes/tail + fail-closed 起点）。
///
/// 不跨 bar 持有上一层的 `Rc`：否则 `IncrSegments::append` 的 `Rc::make_mut` 每 bar 整表复制
/// （生产调用方逐 bar 丢弃旧层）。前缀不回退只比上一层末 `TAIL` 段的拷贝。
fn replay(bars: &[Bar], ladder: &[usize]) -> (Replay, ParseLayer) {
    const TAIL: usize = 64;
    let theta = ThetaConfig::default();
    let mut inc = ParseLayerIncr::new(&theta);
    let mut r = Replay::default();
    // (上一层段数, 拷贝起点, 拷贝, 上一层笔数, 上一层末笔)
    let mut prev: Option<(usize, usize, Vec<Segment>, usize, Option<Stroke>)> = None;
    let mut was_invalid: Option<usize> = None;
    let mut deep_unverified = 0usize;
    let n_all = bars.len();
    for (i, bar) in bars.iter().enumerate() {
        let layer = inc.append(*bar);
        let invalid = inc.segments_parse_invalid_from();
        if invalid.is_some() {
            r.invalid_bars += 1;
            if invalid != was_invalid {
                r.invalid_events += 1;
            }
        }
        was_invalid = invalid;
        if let Some((plen, off, old, pstrokes, plast)) = &prev {
            // 末笔可改写：上一前缀的末笔在本层不再原样存在（被延伸、改写或撤回）。
            let last_rewritten = match plast {
                Some(s) => *pstrokes == 0 || layer.strokes.get(pstrokes - 1) != Some(s),
                None => false,
            };
            // 已发射段（除末段可随末笔改写）须是后一前缀的前缀；只比保留前缀之后的区间。
            let end = plen.saturating_sub(1);
            let k = layer.segments_confirmed_len.min(end);
            if k < *off {
                deep_unverified += 1;
            }
            let from = k.max(*off);
            for j in from..end {
                if layer.segments.get(j) != Some(&old[j - off]) {
                    r.rewrites.push((i, j, last_rewritten && j + 2 >= *plen));
                    break;
                }
            }
            if layer.segments.len() > *plen {
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
                "TB1493 LADDER n={n} strokes={} segments={} failed_geometry={} parse_invalid={:?}",
                full.strokes.len(),
                full.segments.len(),
                sf.candidates
                    .iter()
                    .filter(|c| c.result == "FAILED_GEOMETRY")
                    .count(),
                sf.parse_invalid_from.as_ref().map(|p| p.seg_start)
            );
        }
        if n == n_all {
            eprintln!("TB1493 REPLAY deep_unverified={deep_unverified}");
            return (r, layer);
        }
        let len = layer.segments.len();
        let off = len.saturating_sub(TAIL);
        prev = Some((
            len,
            off,
            layer.segments[off..].to_vec(),
            layer.strokes.len(),
            layer.strokes.last().copied(),
        ));
        drop(layer);
    }
    (r, ParseLayer::default())
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
    let mut gaps: Vec<(usize, usize)> = r.emits.windows(2).map(|w| (w[1] - w[0], w[0])).collect();
    gaps.push((
        n - r.emits.last().copied().unwrap_or(0),
        r.emits.last().copied().unwrap_or(0),
    ));
    gaps.sort_unstable_by(|a, b| b.cmp(a));
    eprintln!(
        "TB1493 {tag} bars={n} emits={} buckets({bucket})={buckets:?} max_zero_run={max_zero_run} top_gaps={:?} invalid_bars={} invalid_events={} rewrites={} rewrites_not_last_stroke={}",
        r.emits.len(),
        &gaps[..gaps.len().min(5)],
        r.invalid_bars,
        r.invalid_events,
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
    let (_, _, sf) =
        segment::divide_segments_with_facts(&layer.strokes, &ThetaConfig::default().parse);
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
    assert!(
        sf.parse_invalid_from.is_none(),
        "{tag}：parse_invalid_from 须为 None"
    );
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
    assert_eq!(
        r.rewrites.iter().filter(|x| !x.2).count(),
        0,
        "前缀确认回退（非末笔可改写）"
    );
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
    assert_eq!(
        r.rewrites.iter().filter(|x| !x.2).count(),
        0,
        "前缀确认回退（非末笔可改写）"
    );
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
        let attempts: usize = sf
            .candidates
            .iter()
            .map(|c| c.termination_attempts.len())
            .sum();
        eprintln!(
            "TB1493 MEM n={n} rss_mb={} strokes={} cands={} first_checks={first_checks} first_steps={first_steps} idx_first={idx_first} second_steps={second} idx_second={idx_second} attempts={attempts} seeds={}",
            vm_hwm_mb(),
            layer.strokes.len(),
            sf.candidates.len(),
            sf.seeds.len()
        );
    }
}

#[test]
#[ignore = "诊断：全量逐 bar 增量回放的耗时曲线（每 10 万根一行）"]
fn tb1493_replay_speed_probe() {
    let all = load_full();
    let bars = &all[1..];
    let theta = ThetaConfig::default();
    let mut inc = ParseLayerIncr::new(&theta);
    let t0 = std::time::Instant::now();
    let mut lap = t0;
    for (i, bar) in bars.iter().enumerate() {
        let layer = inc.append(*bar);
        if (i + 1) % 100_000 == 0 {
            eprintln!(
                "TB1493 SPEED n={} segs={} strokes={} confirmed={} euf={:?} lap_s={:.1} total_s={:.1}",
                i + 1,
                layer.segments.len(),
                layer.strokes.len(),
                layer.segments_confirmed_len,
                layer.segments_earliest_unsealed,
                lap.elapsed().as_secs_f64(),
                t0.elapsed().as_secs_f64()
            );
            lap = std::time::Instant::now();
        }
    }
}

#[test]
#[ignore = "重型验证窗口：btc_1m_full.json 全量，只走逐 bar 增量（不做全量批处理对拍）"]
fn tb1493_replay_full_history_incremental_only() {
    let all = load_full();
    let bars = &all[1..];
    let n = bars.len();
    let (r, last) = replay(bars, &[]);
    let (max_zero_run, longest_gap) = report("FULLINC", &r, n, 250_000);
    for raw in [70_351usize, 787_209] {
        let k = r
            .emits
            .iter()
            .filter(|&&e| e + 1 >= raw && e + 1 < raw + 100_000)
            .count();
        eprintln!("TB1493 FULLINC after_raw={raw} emits_in_100k={k}");
    }
    eprintln!(
        "TB1493 FULLINC strokes={} segments={} last_segment_end={:?} last_stroke_end={:?} n={n} longest_gap={longest_gap}",
        last.strokes.len(),
        last.segments.len(),
        last.segments.last().map(|s| s.end_index),
        last.strokes.last().map(|s| s.end_index)
    );
    // 流首段起点位移：种子 vs 按定义的起点（前 2000 笔上取段事实；观察路径的第二序列事实随长度二次增长）。
    let head = &last.strokes[..last.strokes.len().min(2_000)];
    let (_, _, sf) = segment::divide_segments_with_facts(head, &ThetaConfig::default().parse);
    eprintln!(
        "TB1493 FULLINC first_seed={:?} first_start={:?} head_failed_geometry={} head_parse_invalid={:?}",
        sf.seeds.last().map(|s| s.start_stroke),
        sf.candidates.first().map(|c| c.start_stroke),
        sf.candidates.iter().filter(|c| c.result == "FAILED_GEOMETRY").count(),
        sf.parse_invalid_from.as_ref().map(|p| p.seg_start)
    );
    // #1487 现场（笔号）：其后 10 万根内出段数；覆盖该笔的段。
    for s in [3178usize, 59848] {
        let bar = last.strokes[s].start_index;
        let k = r
            .emits
            .iter()
            .filter(|&&e| bars[e].source_index >= bar && bars[e].source_index < bar + 100_000)
            .count();
        let cover = last
            .segments
            .iter()
            .find(|g| g.start_index <= bar && g.end_index >= bar);
        eprintln!(
            "TB1493 FULLINC SITE stroke={s} bar={bar} emits_in_100k={k} covering_segment={cover:?}"
        );
        assert!(k > 0, "笔 {s} 之后段层停产");
    }
    assert_eq!(r.invalid_bars, 0);
    assert_eq!(
        r.rewrites.iter().filter(|x| !x.2).count(),
        0,
        "前缀确认回退（非末笔可改写）"
    );
    assert!(max_zero_run <= 1, "每 25 万根分桶不得有连续零桶");
}

#[test]
#[ignore = "诊断：全量增量回放中段端门 fail-closed 的现场"]
fn tb1493_replay_invalid_sites() {
    let all = load_full();
    let bars = &all[1..];
    let theta = ThetaConfig::default();
    let mut inc = ParseLayerIncr::new(&theta);
    let mut was: Option<usize> = None;
    let mut shown = 0;
    for (i, bar) in bars.iter().enumerate() {
        let layer = inc.append(*bar);
        let now = inc.segments_parse_invalid_from();
        if now.is_some() && now != was && shown < 8 {
            shown += 1;
            let p = now.unwrap();
            let ss: Vec<Stroke> = layer.strokes[p..].to_vec();
            let prev_seg = layer
                .segments
                .iter()
                .rev()
                .find(|g| g.end_index <= ss[0].start_index)
                .copied();
            segment::FIRST_START_AT_SEED.with(|c| c.set(true));
            let (_, _, sf) = segment::divide_segments_with_facts(&ss, &theta.parse);
            segment::FIRST_START_AT_SEED.with(|c| c.set(false));
            let fmt = |s: &Stroke| {
                format!(
                    "{:?}{}->{}@{}/{}",
                    s.direction, s.start_index, s.end_index, s.start_price, s.end_price
                )
            };
            eprintln!(
                "TB1493 INVALID bar_i={i} raw={} p={p} strokes_total={} prev_seg={prev_seg:?} last_stroke={} local_invalid={:?}",
                bar.source_index,
                layer.strokes.len(),
                fmt(layer.strokes.last().unwrap()),
                sf.parse_invalid_from
            );
            for (j, s) in ss.iter().enumerate().take(24) {
                eprintln!("TB1493 INVALID   s{}={}", p + j, fmt(s));
            }
        }
        was = now;
    }
}

/// #1493 裁定 2 三方对照（真实数据）：段数、两两差异段数、段端非本段极值的段数、段端门失败。
fn ruling2_arm(strokes: &[Stroke], arm: u8) -> (Vec<Segment>, usize, Option<usize>) {
    let cfg = &ThetaConfig::default().parse;
    super::feature_seq::GAP_INCLUSION_ARM.with(|a| a.set(arm));
    let (segs, _) = segment::divide_segments_with_tail(strokes, cfg);
    let invalid = segment::IncrSegments::empty()
        .append(strokes, cfg)
        .parse_invalid_from();
    super::feature_seq::GAP_INCLUSION_ARM.with(|a| a.set(0));
    let mut off = 0usize;
    for s in &segs {
        let a = strokes
            .iter()
            .position(|x| x.start_index == s.start_index)
            .unwrap();
        let b = strokes
            .iter()
            .rposition(|x| x.end_index == s.end_index)
            .unwrap();
        let (hi, lo) = strokes[a..=b]
            .iter()
            .fold((i64::MIN, i64::MAX), |(h, l), x| {
                (
                    h.max(x.start_price.max(x.end_price)),
                    l.min(x.start_price.min(x.end_price)),
                )
            });
        // 段端点 = 段末笔终点价（`Segment.end_price` 是第 78 课标准化后的段内极值）。
        let endpoint = strokes[b].end_price;
        let ok = match s.direction {
            super::super::types::Direction::Up => endpoint == hi,
            super::super::types::Direction::Down => endpoint == lo,
        };
        if !ok {
            off += 1;
        }
    }
    (segs, off, invalid)
}

#[test]
#[ignore = "重型验证窗口：#1493 裁定 2 三方对照（全量与尾窗真实数据）"]
fn tb1493_ruling2_three_way_real() {
    let all = load_full();
    let theta = ThetaConfig::default();
    let tail = &all[all.len() - 50_000..];
    for (tag, bars) in [("TAIL", &tail[5..]), ("FULL", &all[1..])] {
        let layer = parse_layer(bars, &theta);
        let r: Vec<_> = (0..3u8)
            .map(|arm| ruling2_arm(&layer.strokes, arm))
            .collect();
        let diff = |x: usize, y: usize| {
            let (p, q) = (&r[x].0, &r[y].0);
            (0..p.len().max(q.len()))
                .filter(|&j| p.get(j) != q.get(j))
                .count()
        };
        eprintln!(
            "TB1493 RULING2 REAL {tag} strokes={} segs[ruling,window,oldG2]=[{},{},{}] diff_segs[r~w,r~g,w~g]=[{},{},{}] off_extreme=[{},{},{}] parse_invalid=[{:?},{:?},{:?}]",
            layer.strokes.len(),
            r[0].0.len(),
            r[1].0.len(),
            r[2].0.len(),
            diff(0, 1),
            diff(0, 2),
            diff(1, 2),
            r[0].1,
            r[1].1,
            r[2].1,
            r[0].2,
            r[1].2,
            r[2].2
        );
        assert!(r[0].2.is_none(), "{tag}：本裁定段端门失败");
    }
}
