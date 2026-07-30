//! task #107 检测聋度审计：L0–L5 逐级漏斗（结构候选→背驰确认→终端确认→入链）。
//!
//! 用法：
//! `cargo run --release --bin p107_level_funnel_audit -- <btc_1m_full.json> <p92_ckpt_dump.txt>`
//!
//! 只读复核，不改任何生产源码。终态事件集与证书装配管线和 p92/p102 逐字同源
//! （644 meta-rule：探针必走生产路径，无影子分叉）：
//! - 终态分类 + 终态快照候选 = p102 探针 run_terminal_pass / collect_terminal_events 同管线；
//! - 终端门 = p92 bin `terminal_bits_new` 同构查法（nest 级 k == classifier lvl k，p105 §3）；
//! - 证书装配 = nest.rs `assemble_typed_certificates` 生产函数直调，dedup 键与 p92/p102 相同。
//!
//! 回答三问：
//! (a) #105 修复后 trend 429/521 确认率的逐级分布是否正常，pan 对照（交叉验证 P104_LEVEL）；
//! (b) exec>1 的 6 张（A/B 各 3）未递归到 L1：逐级向下枚举同侧候选，按门分类卡位；
//! (c) L0 不监听的结构性影响面：新路径结构性事实 + 旧 CandDeltaEvent 路径（含 L0）参照。
//!
//! 附带仲裁：p92 P92_YIELD terminal_confirmed=24（missed=0）vs p102 P102_NESTABLE
//! 合格基例合计 26（L1 21 + L2 4 + L3 1）——两探针同一管线同一门，数字必须相等，
//! 本探针在同一次运行内同时产出漏斗计数与证书集，给出确定答案。

use newchan_rust::theta_v0::classifier;
use newchan_rust::theta_v0::classifier::decompose;
use newchan_rust::theta_v0::classifier::level_view::{
    assemble_level_view, lower_legs_from, project_extended_windows_carried_only,
    provide_nest_candidate_events, C2LevelViewConfig, C2VersionTuple, CoordinateWindow,
    LevelViewMaterial, LevelViewQuery, NestCandidateEvent, NestDivergenceKind, ProjectionMaterial,
};
use newchan_rust::theta_v0::classifier::nest::{
    assemble_certificates_snapshot, assemble_typed_certificates, is_sub, NestInterval,
    NestIntervalCaliber, TypedNestCertificate,
};
use newchan_rust::theta_v0::classifier::recursive_tower::LeveledMove;
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::{ParseLayer, ParseLayerIncr};
use newchan_rust::theta_v0::types::{quantize, Bar, BspBits, Side, Timestamp};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::rc::Rc;
use std::time::Instant;

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("用法: p107_level_funnel_audit <btc_1m_full.json> <p92_ckpt_dump.txt>")?;
    let dump_path = args
        .next()
        .ok_or("用法: p107_level_funnel_audit <btc_1m_full.json> <p92_ckpt_dump.txt>")?;
    let config = ThetaConfig::default();
    let loaded = load_bars(Path::new(&path), config.tick.tick_size)?;
    if loaded.bars.is_empty() {
        return Err("输入 bars 为空".to_string());
    }
    let max_bars = std::env::var("P107_MAX_BARS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .map_or(loaded.bars.len(), |v| v.min(loaded.bars.len()));
    println!(
        "P107_INPUT bars={} replay_bars={} last_date={}",
        loaded.bars.len(),
        max_bars,
        loaded.last_date
    );

    let terminal = run_terminal_pass(&loaded.bars[..max_bars], &config)?;
    let (hist, close_src) = terminal.cache.causal_series();
    let dif = terminal.cache.macd_dif();
    let (events, proj_errors) =
        collect_terminal_events(&terminal.tower, max_bars - 1, hist, dif, close_src)?;
    println!(
        "P107_EVENTS levels={} total={} projection_errors={}",
        events.len(),
        events.iter().map(Vec::len).sum::<usize>(),
        proj_errors
    );

    // ── 一、逐级漏斗 F1 结构候选 / F2 背驰确认 / F3 终端确认（level×side×kind）──
    let mut total_cands = 0usize;
    let mut total_div = 0usize;
    let mut total_term = 0usize;
    let mut total_term_nodiv = 0usize;
    // F3 身份清单（供 missed 仲裁）。Side 不实现 Ord，键用 is_short。
    let mut term_ids: Vec<(usize, Side, NestDivergenceKind, usize)> = Vec::new();
    for level in 0..events.len() {
        for side in [Side::Long, Side::Short] {
            for kind in [NestDivergenceKind::Trend, NestDivergenceKind::Consolidation] {
                let pool: Vec<&NestCandidateEvent> = events[level]
                    .iter()
                    .filter(|e| e.side == side && e.kind == kind)
                    .collect();
                let cands = pool.len();
                let div = pool.iter().filter(|e| e.divergence_confirmed).count();
                let term = pool
                    .iter()
                    .filter(|e| {
                        e.divergence_confirmed
                            && terminal_bits_new(&terminal.classification, e).is_some()
                    })
                    .count();
                // p102 P102_NESTABLE（合格合计 26）vs p92 terminal_confirmed=24 仲裁用：
                // 终端有同级同向 BSP 但背驰未确认的事件（terminal-only）。
                let term_nodiv = pool
                    .iter()
                    .filter(|e| {
                        !e.divergence_confirmed
                            && terminal_bits_new(&terminal.classification, e).is_some()
                    })
                    .count();
                total_cands += cands;
                total_div += div;
                total_term += term;
                total_term_nodiv += term_nodiv;
                for e in pool.iter().filter(|e| {
                    e.divergence_confirmed
                        && terminal_bits_new(&terminal.classification, e).is_some()
                }) {
                    term_ids.push((level, side, kind, e.turn_source));
                }
                for e in pool.iter().filter(|e| {
                    !e.divergence_confirmed
                        && terminal_bits_new(&terminal.classification, e).is_some()
                }) {
                    println!(
                        "P107_TERM_NODIV_ID lvl={} side={:?} kind={:?} turn_source={} interval_b=({}, {})",
                        level, side, kind, e.turn_source, e.interval_b.0, e.interval_b.1
                    );
                }
                println!(
                    "P107_FUNNEL lvl={} side={:?} kind={:?} cands={} div={} term={} term_nodiv={}",
                    level, side, kind, cands, div, term, term_nodiv
                );
            }
        }
    }
    println!(
        "P107_FUNNEL_SUM cands={} div={} term={} term_nodiv={}",
        total_cands, total_div, total_term, total_term_nodiv
    );
    for (level, side, kind, id) in &term_ids {
        println!(
            "P107_TERM_ID lvl={} side={:?} kind={:?} turn_source={}",
            level, side, kind, id
        );
    }

    // ── 二、证书装配（双口径，与 p92 observe_snapshot / p102 同 dedup 键）──
    let mut seen_a = BTreeSet::new();
    let mut seen_b = BTreeSet::new();
    let mut certs_a: Vec<(usize, usize, TypedNestCertificate)> = Vec::new();
    let mut certs_b: Vec<(usize, usize, TypedNestCertificate)> = Vec::new();
    for exec in 1..events.len() {
        for top in exec..events.len() {
            for cert in
                assemble_typed_certificates(&events, exec, top, NestIntervalCaliber::A, |e| {
                    terminal_bits_new(&terminal.classification, e)
                })
            {
                if seen_a.insert(certificate_key(exec, top, &cert)) {
                    certs_a.push((exec, top, cert));
                }
            }
            for cert in
                assemble_typed_certificates(&events, exec, top, NestIntervalCaliber::B, |e| {
                    terminal_bits_new(&terminal.classification, e)
                })
            {
                if seen_b.insert(certificate_key(exec, top, &cert)) {
                    certs_b.push((exec, top, cert));
                }
            }
        }
    }
    println!(
        "P107_CERT caliber=A certs={} caliber=B certs={}",
        certs_a.len(),
        certs_b.len()
    );

    // ── 三、F4 入链：逐级被吸收身份数（base / rung 分列；missed 仲裁）──
    for (caliber, set) in [("A", &certs_a), ("B", &certs_b)] {
        let mut base_ids: BTreeMap<usize, BTreeSet<(usize, (usize, usize))>> = BTreeMap::new();
        let mut rung_ids: BTreeMap<usize, BTreeSet<(usize, (usize, usize))>> = BTreeMap::new();
        for (_exec, _top, cert) in set {
            let ids = cert.identities();
            for (pos, id) in ids.iter().enumerate() {
                let key = (id.turn_source, id.interval_b);
                if pos == ids.len() - 1 {
                    base_ids.entry(id.level as usize).or_default().insert(key);
                } else {
                    rung_ids.entry(id.level as usize).or_default().insert(key);
                }
            }
        }
        for level in 0..events.len() {
            let nb = base_ids.get(&level).map_or(0, |s| s.len());
            let nr = rung_ids.get(&level).map_or(0, |s| s.len());
            println!(
                "P107_ABSORB caliber={} lvl={} base={} rung={}",
                caliber, level, nb, nr
            );
        }
        // missed：F3（背驰∧终端确认）却未被本口径任何链吸收的事件（P92_MISSED 同语义）。
        let mut covered: BTreeSet<(usize, usize, (usize, usize))> = BTreeSet::new();
        for (_exec, _top, cert) in set {
            for id in cert.identities() {
                covered.insert((id.level as usize, id.turn_source, id.interval_b));
            }
        }
        let mut missed = 0usize;
        for level in 0..events.len() {
            for e in &events[level] {
                if !e.divergence_confirmed
                    || terminal_bits_new(&terminal.classification, e).is_none()
                {
                    continue;
                }
                if !covered.contains(&(level, e.turn_source, e.interval_b)) {
                    missed += 1;
                    println!(
                        "P107_MISSED caliber={} lvl={} side={:?} kind={:?} turn_source={} interval_b={:?}",
                        caliber, level, e.side, e.kind, e.turn_source, e.interval_b
                    );
                }
            }
        }
        println!("P107_MISSED_SUM caliber={} missed={}", caliber, missed);
    }

    // ── 四、终端门聋度诊断：F2 未过 F3 的事件到最近同级同向 BSP 的距离分布 ──
    // 绑定规则（生产）：BSP.source_index == event.turn_source 且 confirm_side。
    // 对 div∧¬term 事件，测其 turn_source 到同级同向 BSP source_index 的最近距离，
    // 区分『同级同向 BSP 根本不存在』（市场事实）与『近在咫尺但不重合』（绑定口径）。
    let mut side_sources: BTreeMap<(usize, bool), BTreeSet<usize>> = BTreeMap::new();
    for (lvl, ls) in terminal.classification.levels.iter().enumerate() {
        for p in ls.bsp.iter() {
            for side in [Side::Long, Side::Short] {
                if p.bits.confirm_side(side) {
                    side_sources
                        .entry((lvl, matches!(side, Side::Short)))
                        .or_default()
                        .insert(p.source_index);
                }
            }
        }
    }
    for level in 0..events.len() {
        for side in [Side::Long, Side::Short] {
            let skey = (level, matches!(side, Side::Short));
            let empty = side_sources.get(&skey).map_or(0, |s| s.len());
            let mut buckets = [0usize; 7]; // 0 / 1-10 / 11-48 / 49-240 / 241-1440 / >1440 / none
            let mut n_fail = 0usize;
            for e in events[level]
                .iter()
                .filter(|e| e.side == side && e.divergence_confirmed)
            {
                if terminal_bits_new(&terminal.classification, e).is_some() {
                    continue;
                }
                n_fail += 1;
                let Some(sources) = side_sources.get(&skey) else {
                    buckets[6] += 1;
                    continue;
                };
                if sources.is_empty() {
                    buckets[6] += 1;
                    continue;
                }
                let d = nearest_distance(sources, e.turn_source);
                match d {
                    0 => buckets[0] += 1,
                    1..=10 => buckets[1] += 1,
                    11..=48 => buckets[2] += 1,
                    49..=240 => buckets[3] += 1,
                    241..=1440 => buckets[4] += 1,
                    _ => buckets[5] += 1,
                }
            }
            println!(
                "P107_BIND_DIST lvl={} side={:?} div_not_term={} side_bsp_sources={} d0={} d1_10={} d11_48={} d49_240={} d241_1440={} d_gt1440={} no_side_bsp={}",
                level, side, n_fail, empty, buckets[0], buckets[1], buckets[2], buckets[3], buckets[4], buckets[5], buckets[6]
            );
        }
    }

    // ── 五、(b) exec>1 基例向下递归卡位：逐级同侧候选按门分类 ──
    for (caliber, set) in [("A", &certs_a), ("B", &certs_b)] {
        for (exec, top, cert) in set {
            if *exec <= 1 {
                continue;
            }
            let base_id = cert.identities().last().expect("证书必有基例");
            let Some(base) = events[*exec].iter().find(|e| {
                e.turn_source == base_id.turn_source && e.interval_b == base_id.interval_b
            }) else {
                println!(
                    "P107_DOWN_WARN caliber={} base not found: {base_id:?}",
                    caliber
                );
                continue;
            };
            let side = cert.certificate().side();
            let base_b = typed_iv(base, NestIntervalCaliber::B);
            let base_a = typed_iv(base, NestIntervalCaliber::A);
            println!(
                "P107_DOWN_BASE caliber={} exec={} top={} side={:?} kind={:?} turn_source={} interval_b=({}, {}) interval_a=({}, {})",
                caliber, exec, top, side, base.kind, base.turn_source,
                base.interval_b.0, base.interval_b.1, base.interval_a.0, base.interval_a.1
            );
            for k in 1..*exec {
                let pool: Vec<&NestCandidateEvent> =
                    events[k].iter().filter(|e| e.side == side).collect();
                let mut contained_b = 0usize;
                let mut contained_a = 0usize;
                let mut cb_div = 0usize;
                let mut cb_term = 0usize;
                let mut ca_div = 0usize;
                let mut ca_term = 0usize;
                let mut pool_div = 0usize;
                let mut pool_term = 0usize;
                // 最近未包含候选（边界擦线 vs 大位移）。
                let mut near: Vec<(i64, i64, &NestCandidateEvent)> = Vec::new();
                for e in &pool {
                    let eb = typed_iv(e, NestIntervalCaliber::B);
                    let ea = typed_iv(e, NestIntervalCaliber::A);
                    if e.divergence_confirmed {
                        pool_div += 1;
                    }
                    let term = e.divergence_confirmed
                        && terminal_bits_new(&terminal.classification, e).is_some();
                    if term {
                        pool_term += 1;
                    }
                    let ok_b = is_sub(&eb, &base_b);
                    let ok_a = is_sub(&ea, &base_a);
                    if ok_b {
                        contained_b += 1;
                        cb_div += usize::from(e.divergence_confirmed);
                        cb_term += usize::from(term);
                    }
                    if ok_a {
                        contained_a += 1;
                        ca_div += usize::from(e.divergence_confirmed);
                        ca_term += usize::from(term);
                    }
                    if !ok_b {
                        let left_gap = eb.start_time as i64 - base_b.start_time as i64;
                        let right_gap = base_b.end_time as i64 - eb.end_time as i64;
                        let miss = (left_gap.min(right_gap)).unsigned_abs() as i64;
                        near.push((miss, left_gap.min(right_gap), *e));
                    }
                }
                near.sort_by_key(|(miss, _, e)| (*miss, e.turn_source));
                println!(
                    "P107_DOWN caliber={} base={}:{} k={} pool={} pool_div={} pool_term={} contained_B={} cb_div={} cb_term={} contained_A={} ca_div={} ca_term={}",
                    caliber, exec, base.turn_source, k, pool.len(), pool_div, pool_term,
                    contained_b, cb_div, cb_term, contained_a, ca_div, ca_term
                );
                for (_, gap, e) in near.iter().take(3) {
                    println!(
                        "P107_DOWN_NEAR caliber={} base={}:{} k={} cand={}:{}-{} kind={:?} div={} term={} min_gap={} interval_b=({}, {})",
                        caliber, exec, base.turn_source, k, e.turn_source, e.interval_b.0, e.interval_b.1,
                        e.kind, e.divergence_confirmed,
                        e.divergence_confirmed && terminal_bits_new(&terminal.classification, e).is_some(),
                        gap, e.interval_b.0, e.interval_b.1
                    );
                }
            }
            // exec=3 追加：L2 中间级被包含者 → 其下 L1 同侧候选的门分类（二级嵌套可行性）。
            if *exec == 3 {
                for e2 in events[2].iter().filter(|e| e.side == side) {
                    let e2b = typed_iv(e2, NestIntervalCaliber::B);
                    if !is_sub(&e2b, &base_b) {
                        continue;
                    }
                    let mut c1 = 0usize;
                    let mut c1_div = 0usize;
                    let mut c1_term = 0usize;
                    for e1 in events[1].iter().filter(|e| e.side == side) {
                        let e1b = typed_iv(e1, NestIntervalCaliber::B);
                        if !is_sub(&e1b, &e2b) {
                            continue;
                        }
                        c1 += 1;
                        c1_div += usize::from(e1.divergence_confirmed);
                        c1_term += usize::from(
                            e1.divergence_confirmed
                                && terminal_bits_new(&terminal.classification, e1).is_some(),
                        );
                    }
                    println!(
                        "P107_DOWN2 caliber={} base=3:{} mid=2:{}-{} mid_div={} mid_term={} l1_contained={} l1_div={} l1_term={}",
                        caliber, base.turn_source, e2.interval_b.0, e2.interval_b.1,
                        e2.divergence_confirmed,
                        e2.divergence_confirmed && terminal_bits_new(&terminal.classification, e2).is_some(),
                        c1, c1_div, c1_term
                    );
                }
            }
        }
    }

    // ── 六、(c) L0 影响面参照：旧 CandDeltaEvent 路径（含 L0）逐级分解 ──
    // 注意口径边界：旧路径事件语义与终端门（confirm_src 绑定）与新路径不同，
    // 仅作『唯一现存的 L0 监听实装』量级参照，不作新路径反事实。
    let old_events = classifier::cand_delta_tower_cached(
        &terminal.l0,
        &terminal.classification,
        &terminal.tower,
        &config,
        &terminal.cache,
    );
    let mut old_total_cands = 0usize;
    let mut old_total_term = 0usize;
    for level in 0..old_events.len() {
        let cands = old_events[level].iter().filter(|e| e.cand_delta).count();
        let term = old_events[level]
            .iter()
            .filter(|e| {
                e.cand_delta
                    && terminal_bits_old(&terminal.classification, level, e.confirm_src, e.side)
                        .is_some()
            })
            .count();
        old_total_cands += cands;
        old_total_term += term;
        println!("P107_OLD_LVL lvl={} cands={} term={}", level, cands, term);
    }
    let mut old_certs = 0usize;
    let mut old_certs_by_exec: BTreeMap<usize, usize> = BTreeMap::new();
    let mut old_certs_multi = 0usize;
    for exec in 0..old_events.len() {
        for top in exec..old_events.len() {
            let n = assemble_certificates_snapshot(&old_events, exec, top, |event| {
                terminal_bits_old(
                    &terminal.classification,
                    exec,
                    event.confirm_src,
                    event.side,
                )
            })
            .len();
            old_certs += n;
            *old_certs_by_exec.entry(exec).or_default() += n;
            if top > exec {
                old_certs_multi += n;
            }
        }
    }
    println!(
        "P107_OLD_SUM cands={} term={} certs={} certs_multi_top={}",
        old_total_cands, old_total_term, old_certs, old_certs_multi
    );
    for (exec, n) in &old_certs_by_exec {
        println!("P107_OLD_CERT_EXEC exec={} certs={}", exec, n);
    }

    // ── 七、与 dump 终端 CERT 集合双向 diff（探针 = 生产路径的验证门）──
    let dump_certs = parse_certs(&dump_path)?;
    let probe_keys: BTreeSet<String> = certs_a
        .iter()
        .chain(certs_b.iter())
        .map(|(exec, top, c)| {
            format!(
                "{:?}:{}:{}:{:?}:{}",
                c.caliber(),
                exec,
                top,
                c.certificate().side(),
                ids_key(c)
            )
        })
        .collect();
    let dump_keys: BTreeSet<String> = dump_certs.into_iter().collect();
    let both = probe_keys.intersection(&dump_keys).count();
    let only_probe = probe_keys.difference(&dump_keys).count();
    let only_dump = dump_keys.difference(&probe_keys).count();
    println!(
        "P107_DUMP_DIFF probe={} dump={} both={} only_probe={} only_dump={}",
        probe_keys.len(),
        dump_keys.len(),
        both,
        only_probe,
        only_dump
    );
    for k in probe_keys.difference(&dump_keys) {
        println!("P107_DUMP_ONLY_PROBE {k}");
    }
    for k in dump_keys.difference(&probe_keys) {
        println!("P107_DUMP_ONLY_DUMP {k}");
    }
    println!("P107_DONE");
    Ok(())
}

fn nearest_distance(sources: &BTreeSet<usize>, x: usize) -> usize {
    let mut best = usize::MAX;
    for &s in sources.range(x..).take(1) {
        best = best.min(s - x);
    }
    if let Some(&s) = sources.range(..=x).next_back() {
        best = best.min(x - s);
    }
    best
}

fn typed_iv(event: &NestCandidateEvent, caliber: NestIntervalCaliber) -> NestInterval {
    let (start, end) = match caliber {
        NestIntervalCaliber::A => event.interval_a,
        NestIntervalCaliber::B => event.interval_b,
    };
    NestInterval {
        start_time: start as u64,
        end_time: end as u64,
        idx: event.turn_source as u64,
    }
}

fn ids_key(cert: &TypedNestCertificate) -> String {
    cert.identities()
        .iter()
        .map(|id| {
            format!(
                "{}:{}:{}-{}",
                id.level, id.turn_source, id.interval_b.0, id.interval_b.1
            )
        })
        .collect::<Vec<_>>()
        .join("|")
}

fn certificate_key(exec: usize, top: usize, certificate: &TypedNestCertificate) -> String {
    let cert = certificate.certificate();
    let rungs = cert
        .rungs()
        .iter()
        .map(|rung| (rung.interval(), rung.child_interval()))
        .collect::<Vec<_>>();
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

fn terminal_bits_new(
    classification: &classifier::Classification,
    event: &NestCandidateEvent,
) -> Option<BspBits> {
    classification
        .levels
        .get(event.level as usize)?
        .bsp
        .iter()
        .find(|point| {
            point.source_index == event.turn_source && point.bits.confirm_side(event.side)
        })
        .map(|point| point.bits)
}

fn terminal_bits_old(
    classification: &classifier::Classification,
    level: usize,
    source: usize,
    side: Side,
) -> Option<BspBits> {
    classification
        .levels
        .get(level)?
        .bsp
        .iter()
        .find(|point| point.source_index == source && point.bits.confirm_side(side))
        .map(|point| point.bits)
}

/// 解析 dump 的 CERT 行，产出与 probe_keys 同构的身份字符串集合。
fn parse_certs(path: &str) -> Result<BTreeSet<String>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("读 {path} 失败: {e}"))?;
    let mut out = BTreeSet::new();
    for (lineno, line) in text.lines().enumerate() {
        let Some(rest) = line.strip_prefix("CERT ") else {
            continue;
        };
        let mut caliber = String::new();
        let mut side = String::new();
        let mut exec = String::new();
        let mut top = String::new();
        let mut ids = String::new();
        for token in rest.split_whitespace() {
            let Some((key, value)) = token.split_once('=') else {
                continue;
            };
            match key {
                "caliber" => caliber = value.to_string(),
                "side" => side = value.to_string(),
                "exec" => exec = value.to_string(),
                "top" => top = value.to_string(),
                "ids" => ids = value.to_string(),
                _ => {}
            }
        }
        if caliber.is_empty() || ids.is_empty() {
            return Err(format!("dump:{} CERT 行缺字段", lineno + 1));
        }
        out.insert(format!("{caliber}:{exec}:{top}:{side}:{ids}"));
    }
    Ok(out)
}

struct TerminalState {
    l0: ParseLayer,
    classification: classifier::Classification,
    tower: Vec<Rc<Vec<LeveledMove>>>,
    cache: classifier::TowerCache,
}

/// 与 p92/p102 同管线：增量因果塔终态分类一遍。
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
                "P107_PROGRESS bar={index}/{} elapsed={:.1}s",
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

/// 与 p92 `collect_snapshot_candidates` / p102 `collect_terminal_events` 同管线：
/// 终态 tower → 逐 run 投影 → C2 视图 → 候选事件（L0 结构性跳过，生产同构）。
fn collect_terminal_events(
    tower: &[Rc<Vec<LeveledMove>>],
    as_of: usize,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
) -> Result<(Vec<Vec<NestCandidateEvent>>, usize), String> {
    let mut errors = 0usize;
    let mut by_level = vec![Vec::new(); tower.len()];
    for level in 1..tower.len() {
        let lower = lower_legs_from(&tower[level - 1])
            .map_err(|error| format!("L{level} lower legs 失败: {error:?}"))?;
        let windows = &tower[level];
        let mut run_start = None;
        for index in 0..=windows.len() {
            let valid = index < windows.len()
                && match project_extended_windows_carried_only(std::slice::from_ref(
                    &windows[index],
                )) {
                    Ok(_) => true,
                    Err(_) => {
                        errors += 1;
                        false
                    }
                };
            match (run_start, valid) {
                (None, true) => run_start = Some(index),
                (Some(start), false) => {
                    let projection = project_extended_windows_carried_only(&windows[start..index])
                        .map_err(|error| format!("L{level} run projection 失败: {error:?}"))?;
                    let centers: Vec<_> = projection.seeds.iter().map(|seed| seed.center).collect();
                    let blocks = decompose::decompose(&centers);
                    let query = LevelViewQuery {
                        level: level as u32,
                        coordinate_window: CoordinateWindow {
                            start: projection.seeds.first().expect("nonempty run").start_index,
                            end: projection.seeds.last().expect("nonempty run").end_index,
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
        events.sort_by_key(|event| {
            (
                event.level,
                matches!(event.side, Side::Short),
                event.kind,
                event.seg_a,
                event.interval_b,
                event.interval_a,
                event.turn_source,
            )
        });
        events.dedup_by(|left, right| {
            (
                left.level,
                left.side,
                left.kind,
                left.seg_a,
                left.interval_b,
                left.interval_a,
                left.turn_source,
            ) == (
                right.level,
                right.side,
                right.kind,
                right.seg_a,
                right.interval_b,
                right.interval_a,
                right.turn_source,
            )
        });
    }
    Ok((by_level, errors))
}

#[derive(Debug)]
struct LoadedBars {
    bars: Vec<Bar>,
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
