//! task #108 反事实探针：裁定② D_parent 口径反事实。
//!
//! 主线问题：B 口径 nest 证书低产量（全量 25 张、Long 多级 0 张）有多少是裁定②
//! 「D_parent = 背驰段、不含回试段」（`chanlun/escalate/nest-migration-ruling-20260716.md:14`）
//! 的口径产物，而非市场事实。
//!
//! 四个口径同台（子侧 I(A_child) 定义不动，只改 D_parent——隔离裁定②）：
//! - B0 基线：child=interval_b, D_parent=interval_b（=seg_c，现行生产 B 口径）；
//! - V1 变体(i)：child=interval_b, D_parent=(seg_c.start, max(retest.end, seg_c.end))
//!   ——背驰段含回试段；retest.end 取自 `interval_a.1`（`structural_pair_span` 右端，
//!   `level_view.rs:513-526`）；intake_fallback 事件 interval_a 为兜底跨度，自然无延伸；
//! - V2 变体(ii)：child=interval_b, D_parent=(seg_c.start, max(judge_at, seg_c.end))
//!   ——延伸到父级确认时点；judge_at=首次背驰确认钟（因果 prefix 重放实测，与 p92 同管线）；
//!   父级事件 terminal 未确认背驰 ⟹ 无确认时点 ⟹ 不延伸（退回基线区间）；
//! - A 参照：child=interval_a, D_parent=interval_a（生产既有诊断口径，夹逼上界+镜像验证）。
//!
//! 忠实性硬门（任一不过则非零退出，结论不得引用）：
//! 1. 生产 `assemble_typed_certificates` A/B 在终态事件集上复现 41/25（全量）并与
//!    `/tmp/p92_ckpt_dump.txt` 末态 CERT 行双向 diff=0（含 judge_at 向量逐字）；
//! 2. 本探针参数化装配器以 (ib,ib)/(ia,ia) 运行，身份集须与生产 A/B 输出恒等
//!   （证明本地 DFS 与 `nest.rs:524-583` 逐语义一致），随后才允许跑 V1/V2。
//!
//! 用法：
//! `cargo run --release --features backtest_bin --bin p108_dparent_caliber_counterfactual -- <btc_1m_full.json> [p92_ckpt_dump.txt]`
//! 冒烟：`P108_MAX_BARS=250000`（复现 dump 首检查点 A=7/B=4）。
//!
//! 纪律：只读探针；生产源码零改动；未改 Cargo.toml（bin 自动发现）；无 git mutation。

use newchan_rust::theta_v0::classifier;
use newchan_rust::theta_v0::classifier::decompose;
use newchan_rust::theta_v0::classifier::level_view::{
    assemble_level_view, lower_legs_from, project_extended_windows_carried_only,
    provide_nest_candidate_events, C2LevelViewConfig, C2VersionTuple, CoordinateWindow,
    LevelViewMaterial, LevelViewQuery, NestCandidateEvent, NestDivergenceKind, ProjectionMaterial,
};
use newchan_rust::theta_v0::classifier::nest::{
    assemble_typed_certificates, is_sub, NestEventIdentity, NestInterval, NestIntervalCaliber,
    TypedNestCertificate,
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

// ═══════════════ 事件身份与首见钟簿（p92 同构） ═══════════════

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

#[derive(Debug, Default)]
struct ClockBook {
    /// 事件首次作为候选出现的钟（bar index）。
    candidates: BTreeMap<EventKey, usize>,
    /// 事件首次 divergence_confirmed=true 的钟。
    divergences: BTreeMap<EventKey, usize>,
}

// ═══════════════ 本地反事实证书（不碰生产 NestCertificate 构造器） ═══════════════

/// 反事实链：只携带归因所需 sidecar（身份/类型/钟，高→低含基例）。
/// 不构造生产 TypedNestCertificate（其构造器为 crate 私有，090：不得旁路伪造）。
#[derive(Debug, Clone)]
struct CfCert {
    side: Side,
    exec: usize,
    top: usize,
    /// (身份, 类型, judge_at)，高→低含基例。
    ids: Vec<(NestEventIdentity, NestDivergenceKind, usize)>,
}

impl CfCert {
    fn ids_string(&self) -> String {
        self.ids
            .iter()
            .map(|(id, _, _)| {
                format!(
                    "{}:{}:{}-{}",
                    id.level, id.turn_source, id.interval_b.0, id.interval_b.1
                )
            })
            .collect::<Vec<_>>()
            .join("|")
    }
    fn chain_key(&self) -> String {
        format!("{}:{}:{:?}:{}", self.exec, self.top, self.side, self.ids_string())
    }
    fn base_key(&self) -> String {
        let (id, _, _) = self.ids.last().expect("链非空");
        format!(
            "{}:{}:{}:{}-{}",
            self.exec, self.top, id.turn_source, id.interval_b.0, id.interval_b.1
        )
    }
    fn depth(&self) -> usize {
        self.top - self.exec + 1
    }
    fn kind_bucket(&self) -> &'static str {
        let trend = self.ids.iter().any(|(_, k, _)| *k == NestDivergenceKind::Trend);
        let pan = self
            .ids
            .iter()
            .any(|(_, k, _)| *k == NestDivergenceKind::Consolidation);
        match (trend, pan) {
            (true, false) => "trend",
            (false, true) => "pan",
            _ => "mixed",
        }
    }
}

fn iv(span: (usize, usize), turn_source: usize) -> NestInterval {
    NestInterval {
        start_time: span.0 as u64,
        end_time: span.1 as u64,
        idx: turn_source as u64,
    }
}

fn ident_of(event: &NestCandidateEvent) -> NestEventIdentity {
    NestEventIdentity {
        level: event.level,
        turn_source: event.turn_source,
        interval_b: event.interval_b,
    }
}

/// 参数化装配：镜像 `nest.rs` `assemble_typed_certificate`/`extend_typed_upward` 语义。
/// child_iv_of / parent_iv_of 分别给出子侧 I(A_child) 与父侧 D_parent；
/// 基例门（divergence_confirmed ∧ terminal.confirm_side）与 DFS 排序/回溯逐字镜像。
fn cf_sweep(
    events: &[Vec<NestCandidateEvent>],
    classification: &classifier::Classification,
    child_iv_of: &dyn Fn(&NestCandidateEvent) -> NestInterval,
    parent_iv_of: &dyn Fn(&NestCandidateEvent) -> NestInterval,
) -> Vec<CfCert> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for exec in 1..events.len() {
        for top in exec..events.len() {
            for base in &events[exec] {
                if !base.divergence_confirmed {
                    continue;
                }
                let Some(terminal) = terminal_bits_new(classification, base) else {
                    continue;
                };
                if !terminal.confirm_side(base.side) {
                    continue;
                }
                let base_iv = child_iv_of(base);
                // 收集序低→高（与生产 ids_low_to_high 同），成功后反转为高→低。
                let mut acc: Vec<(NestEventIdentity, NestDivergenceKind, usize)> =
                    vec![(ident_of(base), base.kind, base.judge_at)];
                if cf_extend(
                    events,
                    base.side,
                    exec + 1,
                    top,
                    &base_iv,
                    child_iv_of,
                    parent_iv_of,
                    &mut acc,
                ) {
                    acc.reverse();
                    let cert = CfCert {
                        side: base.side,
                        exec,
                        top,
                        ids: acc,
                    };
                    if seen.insert(cert.chain_key()) {
                        out.push(cert);
                    }
                }
            }
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn cf_extend(
    events: &[Vec<NestCandidateEvent>],
    side: Side,
    level: usize,
    top: usize,
    child: &NestInterval,
    child_iv_of: &dyn Fn(&NestCandidateEvent) -> NestInterval,
    parent_iv_of: &dyn Fn(&NestCandidateEvent) -> NestInterval,
    acc: &mut Vec<(NestEventIdentity, NestDivergenceKind, usize)>,
) -> bool {
    if level > top {
        return true;
    }
    let Some(evs) = events.get(level) else {
        return false;
    };
    // 与生产同序：D_parent 的 sel_key（end,start,idx）升序 + turn_source + 原索引。
    let mut order: Vec<usize> = (0..evs.len()).collect();
    order.sort_by_key(|&i| (parent_iv_of(&evs[i]).sel_key(), evs[i].turn_source, i));
    for i in order {
        let event = &evs[i];
        // D1 裁定：rung 级纯结构宽候选，不要求 divergence_confirmed（镜像 nest.rs:550-556）。
        if event.side != side {
            continue;
        }
        let parent = parent_iv_of(event);
        if !is_sub(child, &parent) {
            continue;
        }
        acc.push((ident_of(event), event.kind, event.judge_at));
        let next_child = child_iv_of(event);
        if cf_extend(
            events,
            side,
            level + 1,
            top,
            &next_child,
            child_iv_of,
            parent_iv_of,
            acc,
        ) {
            return true;
        }
        acc.pop();
    }
    false
}

// ═══════════════ 口径闭包 ═══════════════

fn child_ib(e: &NestCandidateEvent) -> NestInterval {
    iv(e.interval_b, e.turn_source)
}
fn child_ia(e: &NestCandidateEvent) -> NestInterval {
    iv(e.interval_a, e.turn_source)
}
/// V1：D_parent = 背驰段 ∪ 回试段 = (seg_c.start, retest.end)。max 保外延单调（不退化）。
fn parent_v1(e: &NestCandidateEvent) -> NestInterval {
    iv(
        (e.interval_b.0, e.interval_a.1.max(e.interval_b.1)),
        e.turn_source,
    )
}
/// V2：D_parent = 背驰段延伸到父级确认时点（首次背驰确认钟）；未确认则无延伸。
fn parent_v2(e: &NestCandidateEvent) -> NestInterval {
    let end = if e.divergence_confirmed {
        e.judge_at.max(e.interval_b.1)
    } else {
        e.interval_b.1
    };
    iv((e.interval_b.0, end), e.turn_source)
}

// ═══════════════ main ═══════════════

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("用法: p108_dparent_caliber_counterfactual <btc_1m_full.json> [p92_ckpt_dump.txt]")?;
    let dump_path = args.next();
    let config = ThetaConfig::default();
    let loaded = load_bars(Path::new(&path), config.tick.tick_size)?;
    if loaded.bars.is_empty() {
        return Err("输入 bars 为空".to_string());
    }
    let max_bars = std::env::var("P108_MAX_BARS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .map_or(loaded.bars.len(), |v| v.min(loaded.bars.len()));
    let as_of = max_bars - 1;
    println!(
        "P108_INPUT bars={} replay_bars={} last_date={}",
        loaded.bars.len(),
        max_bars,
        loaded.last_date
    );
    println!(
        "P108_RULE child=interval_b(frozen) B0_D=seg_c V1_D=(seg_c.start,max(retest.end,seg_c.end)) V2_D=(seg_c.start,max(first_div_confirm,seg_c.end)) A_ref=(interval_a,interval_a) base_gate=divergence_confirmed+confirm_side rung_D1=structural_only"
    );

    // ── 终态一遍（p102 同管线） ──
    let started = Instant::now();
    let terminal = run_terminal_pass(&loaded.bars[..max_bars], &config)?;
    let (hist, close_src) = terminal.cache.causal_series();
    let dif = terminal.cache.macd_dif();
    let (mut events, proj_errors) =
        collect_terminal_events(&terminal.tower, as_of, hist, dif, close_src)?;
    let total_events: usize = events.iter().map(Vec::len).sum();
    for level in 1..events.len() {
        let confirmed = events[level]
            .iter()
            .filter(|e| e.divergence_confirmed)
            .count();
        println!(
            "P108_EVENTS level={} candidates={} confirmed={}",
            level,
            events[level].len(),
            confirmed
        );
    }
    println!(
        "P108_EVENTS_TOTAL levels={} total={} projection_errors={}",
        events.len(),
        total_events,
        proj_errors
    );

    // ── 因果 prefix 二遍：实测每个终态事件的首次可证钟（p92 同管线，V2 必需） ──
    let mut targets = BTreeMap::new();
    for event in events.iter().flatten() {
        targets.insert(EventKey::from(event), *event);
    }
    let target_count = targets.len();
    let (book, views, unresolved) =
        run_targeted_prefix_pass(&loaded.bars[..max_bars], &config, &targets)?;
    println!(
        "P108_JUDGE targets={} divergence_clocks={} candidate_clocks={} views={} unresolved={}",
        target_count,
        book.divergences.len(),
        book.candidates.len(),
        views,
        unresolved
    );
    for event in events.iter_mut().flatten() {
        let key = EventKey::from(&*event);
        // p92 observe_snapshot 语义：确认事件取首确认钟，未确认取首候选钟。
        event.judge_at = if event.divergence_confirmed {
            book.divergences.get(&key).copied().unwrap_or(as_of)
        } else {
            book.candidates.get(&key).copied().unwrap_or(as_of)
        };
    }

    // ── 硬门①：生产装配复现 A/B 基线 ──
    let prod_a = production_sweep(&events, &terminal.classification, NestIntervalCaliber::A);
    let prod_b = production_sweep(&events, &terminal.classification, NestIntervalCaliber::B);
    println!(
        "P108_REF caliber=A certs={} caliber=B certs={}",
        prod_a.len(),
        prod_b.len()
    );

    // ── 硬门②：dump 双向 diff（含 judge_at 向量逐字） ──
    if let Some(dump) = dump_path.as_deref() {
        let dump_rows = parse_dump_certs(dump, as_of)?;
        // 冒烟（P108_MAX_BARS 截断）时 dump 无对应 as_of 的 CERT 行——降级为计数核对，跳过硬门。
        let dump_gate_active = !dump_rows.is_empty();
        if !dump_gate_active {
            println!(
                "P108_DUMP_DIFF skipped=1 reason=no_CERT_rows_at_as_of={as_of} (smoke: 以计数核对为准)"
            );
        }
        let diff_one = |tag: &str, mine: &[(usize, usize, TypedNestCertificate)]| {
            let my_set: BTreeSet<String> = mine
                .iter()
                .map(|(exec, top, cert)| prod_chain_key(*exec, *top, cert))
                .collect();
            let dump_set: BTreeSet<String> = dump_rows
                .iter()
                .filter(|(caliber, _, _, _, _, _)| caliber == tag)
                .map(|(_, exec, top, side, ids, _)| format!("{exec}:{top}:{side}:{ids}"))
                .collect();
            let only_mine = my_set.difference(&dump_set).count();
            let only_dump = dump_set.difference(&my_set).count();
            let judge_mismatch = mine
                .iter()
                .filter(|(exec, top, cert)| {
                    let key = prod_chain_key(*exec, *top, cert);
                    dump_rows
                        .iter()
                        .find(|(caliber, e, t, s, ids, _)| {
                            caliber == tag && format!("{e}:{t}:{s}:{ids}") == key
                        })
                        .map(|(_, _, _, _, _, judge)| {
                            judge
                                != &cert
                                    .judge_at()
                                    .iter()
                                    .map(|c| c.to_string())
                                    .collect::<Vec<_>>()
                                    .join(",")
                        })
                        .unwrap_or(false)
                })
                .count();
            println!(
                "P108_DUMP_DIFF caliber={} mine={} dump={} only_in_probe={} only_in_dump={} judge_mismatch={}",
                tag,
                my_set.len(),
                dump_set.len(),
                only_mine,
                only_dump,
                judge_mismatch
            );
            (only_mine, only_dump, judge_mismatch)
        };
        let da = diff_one("A", &prod_a);
        let db = diff_one("B", &prod_b);
        if dump_gate_active && (da != (0, 0, 0) || db != (0, 0, 0)) {
            return Err("硬门①失败：终态复现与 dump 不一致，反事实结论不得引用".to_string());
        }
    }

    // ── 硬门③：本地参数化装配器镜像验证 (ib,ib)→B、(ia,ia)→A ──
    let cf_b0 = cf_sweep(&events, &terminal.classification, &child_ib, &child_ib);
    let cf_a = cf_sweep(&events, &terminal.classification, &child_ia, &child_ia);
    let mirror_ok = |tag: &str,
                     mine: &[CfCert],
                     prod: &[(usize, usize, TypedNestCertificate)]|
     -> bool {
        let my_set: BTreeSet<String> = mine.iter().map(CfCert::chain_key).collect();
        let prod_set: BTreeSet<String> = prod
            .iter()
            .map(|(exec, top, cert)| prod_chain_key(*exec, *top, cert))
            .collect();
        let only_mine = my_set.difference(&prod_set).count();
        let only_prod = prod_set.difference(&my_set).count();
        println!(
            "P108_MIRROR caliber={} local={} prod={} only_in_local={} only_in_prod={} match={}",
            tag,
            my_set.len(),
            prod_set.len(),
            only_mine,
            only_prod,
            only_mine == 0 && only_prod == 0
        );
        only_mine == 0 && only_prod == 0
    };
    let ok_b = mirror_ok("B0", &cf_b0, &prod_b);
    let ok_a = mirror_ok("A", &cf_a, &prod_a);
    if !ok_b || !ok_a {
        return Err("硬门③失败：本地装配器与生产不恒等，禁止引用 V1/V2".to_string());
    }

    // ── 反事实两变体 ──
    let cf_v1 = cf_sweep(&events, &terminal.classification, &child_ib, &parent_v1);
    let cf_v2 = cf_sweep(&events, &terminal.classification, &child_ib, &parent_v2);

    for (tag, set) in [
        ("B0", &cf_b0),
        ("V1", &cf_v1),
        ("V2", &cf_v2),
        ("A", &cf_a),
    ] {
        report_set(tag, set);
    }

    // ── 变体宽度/覆盖诊断 ──
    for level in 2..events.len() {
        let evs = &events[level];
        if evs.is_empty() {
            continue;
        }
        let mut ratio_v1: Vec<f64> = evs
            .iter()
            .filter(|e| e.interval_b.1 > e.interval_b.0)
            .map(|e| {
                (e.interval_a.1.max(e.interval_b.1) - e.interval_b.0) as f64
                    / (e.interval_b.1 - e.interval_b.0) as f64
            })
            .collect();
        ratio_v1.sort_by(|a, b| a.partial_cmp(b).expect("f64"));
        let mut ratio_a: Vec<f64> = evs
            .iter()
            .filter(|e| e.interval_b.1 > e.interval_b.0)
            .map(|e| {
                (e.interval_a.1 - e.interval_a.0) as f64 / (e.interval_b.1 - e.interval_b.0) as f64
            })
            .collect();
        ratio_a.sort_by(|a, b| a.partial_cmp(b).expect("f64"));
        let confirmed: Vec<&NestCandidateEvent> =
            evs.iter().filter(|e| e.divergence_confirmed).collect();
        let mut ext_v2: Vec<usize> = confirmed
            .iter()
            .map(|e| e.judge_at.saturating_sub(e.interval_b.1))
            .collect();
        ext_v2.sort_unstable();
        let mut ext_v1: Vec<usize> = evs
            .iter()
            .map(|e| e.interval_a.1.max(e.interval_b.1) - e.interval_b.1)
            .collect();
        ext_v1.sort_unstable();
        let med = |v: &[f64]| if v.is_empty() { 0.0 } else { v[v.len() / 2] };
        let medu =
            |v: &[usize]| if v.is_empty() { 0 } else { v[v.len() / 2] };
        println!(
            "P108_WIDTH level={} events={} confirmed={} v1_width_ratio_p50={:.3} fullA_width_ratio_p50={:.3} v1_ext_bars_p50={} v2_ext_bars_p50={}",
            level,
            evs.len(),
            confirmed.len(),
            med(&ratio_v1),
            med(&ratio_a),
            medu(&ext_v1),
            medu(&ext_v2),
        );
    }

    // ── 差集分析：B0→V1 / B0→V2 / B0→A（救活的链与边） ──
    for (tag, var) in [("V1", &cf_v1), ("V2", &cf_v2), ("A", &cf_a)] {
        diff_report(tag, &cf_b0, var, &events);
    }

    println!(
        "P108_DONE elapsed_s={:.1}",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

// ═══════════════ 统计与差集报告 ═══════════════

fn report_set(tag: &str, certs: &[CfCert]) {
    let long = certs.iter().filter(|c| c.side == Side::Long).count();
    let long_multi = certs
        .iter()
        .filter(|c| c.side == Side::Long && c.exec == 1 && c.top > 1)
        .count();
    let short_multi = certs
        .iter()
        .filter(|c| c.side == Side::Short && c.exec == 1 && c.top > 1)
        .count();
    let bucket = |b: &str| certs.iter().filter(|c| c.kind_bucket() == b).count();
    println!(
        "P108_CERT variant={} certs={} long={} short={} long_exec1_multi={} short_exec1_multi={} trend={} pan={} mixed={}",
        tag,
        certs.len(),
        long,
        certs.len() - long,
        long_multi,
        short_multi,
        bucket("trend"),
        bucket("pan"),
        bucket("mixed"),
    );
    for depth in 1..=8 {
        let n = certs.iter().filter(|c| c.depth() == depth).count();
        if n > 0 {
            println!("P108_DEPTH variant={} depth={} n={}", tag, depth, n);
        }
    }
    for exec in 1..=8 {
        let n = certs.iter().filter(|c| c.exec == exec).count();
        if n > 0 {
            println!("P108_EXEC variant={} exec={} n={}", tag, exec, n);
        }
    }
    for side in [Side::Long, Side::Short] {
        for top in 1..=8 {
            let n = certs
                .iter()
                .filter(|c| c.side == side && c.top == top)
                .count();
            if n > 0 {
                println!(
                    "P108_SIDE_TOP variant={} side={:?} top={} n={}",
                    tag, side, top, n
                );
            }
        }
    }
    let base_set: BTreeSet<String> = certs.iter().map(CfCert::base_key).collect();
    println!(
        "P108_BASE_FEASIBLE variant={} bases={}",
        tag,
        base_set.len()
    );
    for cert in certs {
        let kinds = cert
            .ids
            .iter()
            .map(|(_, k, _)| format!("{k:?}"))
            .collect::<Vec<_>>()
            .join(",");
        let judges = cert
            .ids
            .iter()
            .map(|(_, _, j)| j.to_string())
            .collect::<Vec<_>>()
            .join(",");
        println!(
            "P108_CERTLINE variant={} exec={} top={} side={:?} bucket={} kinds={} judge_at={} ids={}",
            tag,
            cert.exec,
            cert.top,
            cert.side,
            cert.kind_bucket(),
            kinds,
            judges,
            cert.ids_string()
        );
    }
}

fn find_event<'a>(
    events: &'a [Vec<NestCandidateEvent>],
    level: usize,
    turn_source: usize,
    interval_b: (usize, usize),
) -> Option<&'a NestCandidateEvent> {
    events.get(level)?.iter().find(|e| {
        e.turn_source == turn_source && e.interval_b == interval_b
    })
}

fn diff_report(
    tag: &str,
    base: &[CfCert],
    var: &[CfCert],
    events: &[Vec<NestCandidateEvent>],
) {
    let base_ids: BTreeSet<String> = base.iter().map(CfCert::chain_key).collect();
    let var_ids: BTreeSet<String> = var.iter().map(CfCert::chain_key).collect();
    let base_bases: BTreeSet<String> = base.iter().map(CfCert::base_key).collect();
    let var_bases: BTreeSet<String> = var.iter().map(CfCert::base_key).collect();
    let rescued: Vec<&CfCert> = var
        .iter()
        .filter(|c| !base_ids.contains(&c.chain_key()))
        .collect();
    let killed: Vec<&CfCert> = base
        .iter()
        .filter(|c| !var_ids.contains(&c.chain_key()))
        .collect();
    let base_new = var_bases.difference(&base_bases).count();
    let base_lost = base_bases.difference(&var_bases).count();
    let parent_switch = rescued
        .iter()
        .filter(|c| base_bases.contains(&c.base_key()))
        .count();
    println!(
        "P108_DIFF pair=B0->{} rescued_chains={} killed_chains={} base_new={} base_lost={} rescued_via_parent_switch={}",
        tag,
        rescued.len(),
        killed.len(),
        base_new,
        base_lost,
        parent_switch
    );
    for cert in &rescued {
        let base_novel = !base_bases.contains(&cert.base_key());
        println!(
            "P108_RESCUE pair=B0->{} base_novel={} exec={} top={} side={:?} ids={}",
            tag,
            base_novel,
            cert.exec,
            cert.top,
            cert.side,
            cert.ids_string()
        );
        // 逐边复核：该边在 B0 下是否破裂（救活边定位）。
        for k in 0..cert.ids.len() - 1 {
            let (pid, _, pjudge) = cert.ids[k];
            let (cid, _, _) = cert.ids[k + 1];
            let (Some(parent), Some(child)) = (
                find_event(events, pid.level as usize, pid.turn_source, pid.interval_b),
                find_event(events, cid.level as usize, cid.turn_source, cid.interval_b),
            ) else {
                continue;
            };
            let cb = child_ib(child);
            let pb = child_ib(parent);
            if is_sub(&cb, &pb) {
                continue; // B0 下本就成立，非救活边
            }
            let left_gap = cb.start_time as i64 - pb.start_time as i64;
            let right_gap = pb.end_time as i64 - cb.end_time as i64;
            let shape = if cb.start_time > pb.end_time {
                "child_right_of_parent(retest_zone)"
            } else if cb.end_time < pb.start_time {
                "child_left_of_parent(pre_C_zone)"
            } else {
                "overlap_not_contained"
            };
            let dv1 = parent_v1(parent);
            let dv2 = parent_v2(parent);
            println!(
                "P108_RESCUE_EDGE pair=B0->{} edge=L{}->L{} side={:?} parent_kind={:?} child_b=({},{}) parent_b=({},{}) left_gap={} right_gap={} shape={} v1_parent_D=({},{}) v1_holds={} v2_parent_D=({},{}) v2_holds={} parent_div_confirmed={} parent_judge_at={}",
                tag,
                pid.level,
                cid.level,
                cert.side,
                parent.kind,
                cb.start_time,
                cb.end_time,
                pb.start_time,
                pb.end_time,
                left_gap,
                right_gap,
                shape,
                dv1.start_time,
                dv1.end_time,
                is_sub(&cb, &dv1),
                dv2.start_time,
                dv2.end_time,
                is_sub(&cb, &dv2),
                parent.divergence_confirmed,
                pjudge,
            );
        }
    }
    for cert in &killed {
        let base_alive = var_bases.contains(&cert.base_key());
        println!(
            "P108_KILLED pair=B0->{} base_still_covered={} exec={} top={} side={:?} ids={}",
            tag,
            base_alive,
            cert.exec,
            cert.top,
            cert.side,
            cert.ids_string()
        );
    }
}

// ═══════════════ 生产装配包装（硬门对照） ═══════════════

fn production_sweep(
    events: &[Vec<NestCandidateEvent>],
    classification: &classifier::Classification,
    caliber: NestIntervalCaliber,
) -> Vec<(usize, usize, TypedNestCertificate)> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for exec in 1..events.len() {
        for top in exec..events.len() {
            for cert in assemble_typed_certificates(events, exec, top, caliber, |e| {
                terminal_bits_new(classification, e)
            }) {
                let key = prod_chain_key(exec, top, &cert);
                if seen.insert(key) {
                    out.push((exec, top, cert));
                }
            }
        }
    }
    out
}

fn prod_chain_key(exec: usize, top: usize, cert: &TypedNestCertificate) -> String {
    let ids = cert
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
    format!("{exec}:{top}:{:?}:{ids}", cert.certificate().side())
}

/// 解析 p92 dump 的 CERT 行：(caliber, exec, top, side, ids, judge_at)。
fn parse_dump_certs(
    path: &str,
    as_of: usize,
) -> Result<Vec<(String, usize, usize, String, String, String)>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("读取 dump {} 失败: {error}", path))?;
    let want_asof = format!("as_of={as_of}");
    let mut rows = Vec::new();
    for line in text.lines() {
        if !line.starts_with("CERT ") || !line.contains(&want_asof) {
            continue;
        }
        let mut caliber = None;
        let mut exec = None;
        let mut top = None;
        let mut side = None;
        let mut ids = None;
        let mut judge = None;
        for tok in line.split_whitespace().skip(1) {
            if let Some((k, v)) = tok.split_once('=') {
                match k {
                    "caliber" => caliber = Some(v.to_string()),
                    "exec" => exec = v.parse::<usize>().ok(),
                    "top" => top = v.parse::<usize>().ok(),
                    "side" => side = Some(v.to_string()),
                    "ids" => ids = Some(v.to_string()),
                    "judge_at" => judge = Some(v.to_string()),
                    _ => {}
                }
            }
        }
        let (Some(c), Some(e), Some(t), Some(s), Some(i), Some(j)) =
            (caliber, exec, top, side, ids, judge)
        else {
            return Err(format!("dump CERT 行解析失败: {line}"));
        };
        rows.push((c, e, t, s, i, j));
    }
    Ok(rows)
}

// ═══════════════ 终态/prefix 管线（p92/p102 同构） ═══════════════

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

struct TerminalState {
    #[allow(dead_code)]
    l0: ParseLayer,
    classification: classifier::Classification,
    tower: Vec<Rc<Vec<LeveledMove>>>,
    cache: classifier::TowerCache,
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
                "P108_TERMINAL_PROGRESS bar={index}/{} elapsed={:.1}s",
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

/// 与 p92 `collect_snapshot_candidates` 同管线：终态 tower → 逐 run 投影 → C2 视图 → 候选事件。
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
                && match project_extended_windows_carried_only(std::slice::from_ref(&windows[index]))
                {
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

/// p92 目标化 prefix 重放：只对终态目标事件路由视图，实测首次候选/确认钟。
fn run_targeted_prefix_pass(
    bars: &[Bar],
    config: &ThetaConfig,
    targets: &BTreeMap<EventKey, NestCandidateEvent>,
) -> Result<(ClockBook, usize, usize), String> {
    let mut parser = ParseLayerIncr::new(config);
    let mut cache = classifier::TowerCache::new();
    let mut book = ClockBook::default();
    let mut pending: BTreeSet<EventKey> = targets.keys().cloned().collect();
    let mut views = 0usize;
    let mut last_trigger = None;
    let started = Instant::now();
    for (index, bar) in bars.iter().copied().enumerate() {
        let l0 = parser.append(bar);
        let (classification, tower) =
            classifier::classify_with_tower_incremental(&l0, config, &mut cache);
        let trigger = (cache.forest_epoch(), signal_signature(&classification));
        if last_trigger.as_ref() != Some(&trigger) && !pending.is_empty() {
            let (hist, close_src) = cache.causal_series();
            let dif = cache.macd_dif();
            let (found, used_views) =
                collect_target_candidates(&tower, index, hist, dif, close_src, targets, &pending)?;
            views += used_views;
            for event in found {
                let key = EventKey::from(&event);
                if !pending.contains(&key) {
                    continue;
                }
                book.candidates.entry(key.clone()).or_insert(index);
                let target = targets.get(&key).expect("pending target");
                if event.divergence_confirmed {
                    book.divergences.entry(key.clone()).or_insert(index);
                }
                if event.divergence_confirmed == target.divergence_confirmed {
                    pending.remove(&key);
                }
            }
            last_trigger = Some(trigger);
        }
        if index > 0 && index % 500_000 == 0 {
            eprintln!(
                "P108_PREFIX_PROGRESS bar={index}/{} elapsed={:.1}s pending={}/{} views={views}",
                bars.len(),
                started.elapsed().as_secs_f64(),
                pending.len(),
                targets.len()
            );
        }
    }
    Ok((book, views, pending.len()))
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
        // snapshot 无前视：turn_source 未到的对象不可能成为 Cand（p92 同剪枝）。
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

// ═══════════════ 输入装载（p102 同构） ═══════════════

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
