//! #550 真实数据事件通道电池。
//!
//! 用法：
//! `cargo run --release --bin issue550_event_battery -- <btc_1m_full.json> [max_bars]`

use newchan_rust::theta_v0::classifier::cand_event::{
    CandidateEvent, CandidateKey, CandidateKind, CandidateState,
};
use newchan_rust::theta_v0::classifier::cand_sub;
use newchan_rust::theta_v0::classifier::chain_cert;
use newchan_rust::theta_v0::classifier::streaming::OwnedIncrementalClassifier;
use newchan_rust::theta_v0::classifier::{self, TowerCache};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::ParseLayerIncr;
use newchan_rust::theta_v0::types::{quantize, Bar};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;
use std::rc::Rc;

#[derive(Deserialize)]
struct RawBars {
    opens: Vec<Option<f64>>,
    highs: Vec<Option<f64>>,
    lows: Vec<Option<f64>>,
    closes: Vec<Option<f64>>,
    #[serde(default)]
    volumes: Vec<Option<f64>>,
    dates: Vec<String>,
}

/// 从日期串提取前 14 位数字作为时间戳。解析失败是输入损坏，不是可默认化的情况——
/// 一律返回 `Err` 并带上原始 date 串，绝不用哨兵值（如 0）冒充有效时间戳。
fn timestamp(date: &str) -> Result<i64, String> {
    let digits: String = date
        .chars()
        .take_while(|c| *c != '+')
        .filter(char::is_ascii_digit)
        .take(14)
        .collect();
    digits
        .parse()
        .map_err(|error| format!("日期 {date:?} 解析时间戳失败（提取数字串 {digits:?}）: {error}"))
}

fn load(path: &Path, tick_size: f64, limit: usize) -> Result<Vec<Bar>, String> {
    let raw = load_raw(path)?;
    raw_to_bars(raw, tick_size, limit)
}

fn load_raw(path: &Path) -> Result<RawBars, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|error| format!("读取 {} 失败: {error}", path.display()))?
        .replace("-Infinity", "null")
        .replace("Infinity", "null")
        .replace("NaN", "null");
    serde_json::from_str(&text).map_err(|error| format!("解析 {} 失败: {error}", path.display()))
}

fn raw_to_bars(raw: RawBars, tick_size: f64, limit: usize) -> Result<Vec<Bar>, String> {
    let n = raw
        .closes
        .len()
        .min(raw.opens.len())
        .min(raw.highs.len())
        .min(raw.lows.len())
        .min(raw.dates.len())
        .min(limit);
    let mut bars = Vec::with_capacity(n);
    for i in 0..n {
        let prior = bars.last().map_or(0, |bar: &Bar| bar.close);
        bars.push(raw_bar(&raw, i, prior, tick_size)?);
    }
    Ok(bars)
}

fn raw_bar(raw: &RawBars, i: usize, prior: i64, tick_size: f64) -> Result<Bar, String> {
    let volume = raw.volumes.get(i).and_then(|value| *value).unwrap_or(0.0);
    let values = match (raw.opens[i], raw.highs[i], raw.lows[i], raw.closes[i]) {
        (Some(open), Some(high), Some(low), Some(close)) => {
            let invalid = high < open.max(close).max(low)
                || low > open.min(close).min(high)
                || [open, high, low, close].iter().any(|price| *price <= 0.0);
            (
                quantize(open, tick_size),
                quantize(high, tick_size),
                quantize(low, tick_size),
                quantize(close, tick_size),
                invalid,
            )
        }
        _ => (prior, prior, prior, prior, true),
    };
    Ok(Bar {
        source_index: i,
        timestamp: timestamp(&raw.dates[i])?,
        open: values.0,
        high: values.1,
        low: values.2,
        close: values.3,
        volume: volume as i64,
        untradable: values.4 || volume <= 0.0,
    })
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let path = args
        .next()
        .ok_or("用法: issue550_event_battery <btc_1m_full.json> [max_bars]")?;
    let max_bars = args
        .next()
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|error| format!("max_bars 非法: {error}"))?
        .unwrap_or(100_000);
    // #641（N3）链簿推进节拍：每 N 根 bar 推进一次（末根必推）。链的覆盖边计算是 O(n²)，
    // 逐 bar 推进在 10 万级窗口上不可行；节拍是**显式声明**的口径，随读数一并印出，不是静默采样。
    let chain_every = args
        .next()
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|error| format!("chain_every 非法: {error}"))?
        .unwrap_or(5_000)
        .max(1);
    let config = ThetaConfig::default();
    let bars = load(Path::new(&path), config.tick.tick_size, max_bars)?;
    if bars.is_empty() {
        return Err("输入窗口为空".to_string());
    }

    let (terminal_streams, chain_run) = compare_streams(&bars, &config, chain_every)?;
    print_summary(&bars, &terminal_streams);
    print_lifecycle_summary(&terminal_streams);
    print_containment_summary(&terminal_streams);
    print_projection_fork(&bars, &config, &terminal_streams)?;
    print_chain_summary(&bars, chain_run);
    Ok(())
}

/// #641（N3）：真实数据上的链证书簿读数（照实登记，不预设任何数值）。
///
/// 与 `ISSUE552_CONTAIN` 同款纪律——印出的每个数都是生产判据本身的判定结果
/// （[`chain_cert::ChainCertificateBook::advance`] 内部逐对调 `candidate_is_sub`），
/// bin 侧只做**分桶计数**，不重写任何判据。
///
/// 链簿挂在 #550 既有的逐 bar 对拍循环上（[`ChainRun`]），**不另跑一遍全量分类**：
/// 500k 窗口上多一遍逐 bar 因果重放会把电池整体推过 10 分钟（实测中止在案，见实施报告）。
/// 挂载点只**读**该循环已经产出的事件流，不改其输入、判定与输出——既有行逐字节不动。
fn print_chain_summary(bars: &[Bar], run: ChainRun) {
    let ChainRun {
        mut book,
        every,
        advances,
        last_streams,
        last_as_of,
    } = run;
    // 分桶计数走**库内**读数口 `ChainCertificateBook::summarize()`（#641 修复轮从 B 侧移植）——
    // bin 侧此前自己重写一遍循环，与 p123 的 dump 行各算各的、且不可单测。现两处同源。
    let summary = book.summarize();
    // `digest` 与 `summarize` 必须取自同一簿状态点（幂等重跑`advance`之前）——否则幂等一旦破，
    // `digest` 会静默包含重跑追加的 revision 而同行打印的 `summary` 不含（#653 影子评审 LOW-1）。
    let digest = book.digest();
    let by_status = BTreeMap::from([
        ("Open", summary.open),
        ("Closed", summary.closed),
        ("Invalidated", summary.invalidated),
    ]);

    // 幂等：同一 as_of、同一事件流重跑必须零 Delta。非零即为红（此处照实印出，不吞）。
    let replay = book.advance(&last_streams, last_as_of).len();

    println!(
        "ISSUE641_CHAIN bars={} chains={} revisions={} advance_every={every} advances={advances} \
         statuses={by_status:?} path_lens={:?} root_levels={:?} \
         extends_some={} idempotent_replay_delta={replay}",
        bars.len(),
        summary.chains,
        summary.revisions,
        summary.by_path_len,
        summary.by_root_level,
        summary.extends_some,
    );
    println!(
        "ISSUE641_CHAIN_EDGE edges={} adjacent={} skip={} fact_edges={} \
         skip_ratio={:.4} crossed_nodes={}",
        summary.edges,
        summary.adjacent_edges,
        summary.skip_edges,
        summary.fact_edges,
        if summary.edges == 0 {
            0.0
        } else {
            summary.skip_edges as f64 / summary.edges as f64
        },
        summary.crossed_nodes,
    );
    println!(
        "ISSUE641_CHAIN_TRACE nodes_alive={} nodes_falsified={} \
         nodes_absent={} skipped_level_missing={} \
         skipped_level_broken_outside={} \
         skipped_level_broken_inside={}",
        summary.nodes_alive,
        summary.nodes_falsified,
        summary.nodes_absent,
        summary.skipped_level_missing,
        summary.skipped_level_broken_outside,
        summary.skipped_level_broken_inside,
    );
    // #641 修复轮新增读数：地板条款监视格（应恒 0）+ 判死成因两档 + 簿摘要。
    println!(
        "ISSUE641_CHAIN_FLOOR closed_with_zero_segments={} segments={} \
         invalidated_head={} invalidated_predicate={} digest={}",
        summary.closed_with_zero_segments,
        summary.segments,
        summary.invalidated_head,
        summary.invalidated_predicate,
        digest,
    );
}

/// #552（N2）：真实事件流上的相邻级 `C⊆C` 包含只读探针 + 覆盖计数（防真空绿）。
///
/// 计数**不是** bin 侧的独立重算——本节直接调用生产谓词的扫描入口
/// [`cand_sub::scan_adjacent_containment`]（其内部逐对调 `candidate_is_sub`），
/// 故印出的每个数都是谓词本身在真实数据上的判定结果，与单测同源。
///
/// `pairs=0` 是**真空**读数（该窗口没有相邻级对可判），必须与「判过但全不成立」区分——
/// 因此逐级印 child_events/parent_events/pairs，不只印成立数。
fn print_containment_summary(streams: &classifier::cand_event::CandidateStreams) {
    let scan = cand_sub::scan_adjacent_containment(streams);
    for entry in &scan.levels {
        println!(
            "ISSUE552_CONTAIN_LEVEL child_level={} parent_level={} child_events={} \
             parent_events={} pairs={} contained={} touching={} strict={} disjoint={} \
             degenerate={} reverse_blocked_by_level={}",
            entry.child_level,
            entry.parent_level,
            entry.child_events,
            entry.parent_events,
            entry.pairs,
            entry.contained,
            entry.touching,
            entry.strict,
            entry.disjoint,
            entry.degenerate,
            entry.reverse_blocked_by_level,
        );
    }
    println!(
        "ISSUE552_CONTAIN adjacent_level_pairs_scanned={} total_pairs={} total_contained={} \
         total_touching={} same_level_pairs={} same_level_interval_ok={} same_level_blocked={} \
         same_level_reflexive_blocked={} same_level_non_reflexive_blocked={}",
        scan.levels.len(),
        scan.total_pairs(),
        scan.total_contained(),
        scan.total_touching(),
        scan.same_level.pairs,
        scan.same_level.interval_ok,
        scan.same_level.blocked,
        scan.same_level.reflexive_blocked,
        scan.same_level.non_reflexive_blocked(),
    );
}

/// #551：因果簿生命史分解——四态构成、修订类别、首证钟落位。
///
/// 与 `print_summary` 的「每 key 最新 revision」口径互补：本节读**全部** revision，使
/// 「转移/修订/钟路径是否在真实数据上真被触发」机器可见（防真空绿）。
fn print_lifecycle_summary(streams: &classifier::cand_event::CandidateStreams) {
    let mut latest = BTreeMap::<CandidateKey, &CandidateEvent>::new();
    let mut history = BTreeMap::<CandidateKey, Vec<&CandidateEvent>>::new();
    for stream in streams.iter() {
        for event in stream.iter() {
            latest.insert(event.key, event);
            history.entry(event.key).or_default().push(event);
        }
    }

    let mut by_state = BTreeMap::<&'static str, usize>::new();
    let mut trend_by_state = BTreeMap::<&'static str, usize>::new();
    let mut with_clock = 0usize;
    for event in latest.values() {
        *by_state.entry(state_name(event.state)).or_default() += 1;
        if event.kind == CandidateKind::Trend {
            *trend_by_state.entry(state_name(event.state)).or_default() += 1;
        }
        if event.first_provable_at.is_some() {
            with_clock += 1;
        }
    }

    let (mut growth, mut payload, mut transition, mut invalidation) = (0usize, 0usize, 0usize, 0);
    for revisions in history.values() {
        for pair in revisions.windows(2) {
            let (prior, next) = (pair[0], pair[1]);
            if next.state == CandidateState::Invalidated && prior.state != next.state {
                invalidation += 1;
            } else if prior.state != next.state {
                transition += 1;
            } else if next.interval.1 > prior.interval.1 {
                growth += 1;
            } else {
                payload += 1;
            }
        }
    }
    if std::env::var_os("ISSUE551_DIAG").is_some() {
        for event in latest.values() {
            if event.state == CandidateState::Unresolved || event.kind == CandidateKind::Trend {
                println!(
                    "ISSUE551_DIAG trend kind={:?} state={:?} preds={:?} interval={:?} first_provable={:?} revision={} level={}",
                    event.kind, event.state, event.structural_predicates, event.interval,
                    event.first_provable_at, event.revision, event.key.level
                );
            }
        }
    }
    println!(
        "ISSUE551_LIFECYCLE states={by_state:?} trend_states={trend_by_state:?} \
         with_first_provable={with_clock} growth_revisions={growth} payload_revisions={payload} \
         transitions={transition} invalidations={invalidation}"
    );
}

fn state_name(state: CandidateState) -> &'static str {
    match state {
        CandidateState::Provisional => "Provisional",
        CandidateState::Unresolved => "Unresolved",
        CandidateState::Confirmed => "Confirmed",
        CandidateState::Invalidated => "Invalidated",
    }
}

/// #551 裁定(i) 的真实数据版：末态 fresh 全量流 vs 因果簿终态投影。
///
/// 只跑**一次** fresh 全量（O(n)，非逐前缀 O(n²)）。两侧按 key 对齐后分类报数。
///
/// 口径以裁定(i) 的**甲口径定稿**为准（编排者 2026-07-28 裁决，见
/// `chanlun/review-results/issue551-t2-impl-20260728.md` §五）：
/// - `fork_causal_only`：因果簿有而 fresh 无——终态语义的定义后果（中途消失 ⟹ Invalidated
///   在案），计量项；
/// - `revived`：因果簿已判终态却仍被 fresh 产出（复活型分叉）——**禁令已随裁决取消**，现为
///   终态域计量项，非零不是红；
/// - `fork_fresh_only`：fresh 有而因果簿无——**仍是禁止方向**，非零即为红。它不在终态域豁免内：
///   因果簿是正本，fresh 侧凭空多出从未入簿的身份 = 增量宿主漏记，是实装缺陷而非语义后果；
/// - `payload_differ_live`：非终态候选的载荷差异——甲口径的**无条件域**，非零即为红。
fn print_projection_fork(
    bars: &[Bar],
    config: &ThetaConfig,
    causal: &classifier::cand_event::CandidateStreams,
) -> Result<(), String> {
    let mut parser = ParseLayerIncr::new(config);
    let mut l0 = parser.append(bars[0]);
    for bar in bars.iter().copied().skip(1) {
        l0 = parser.append(bar);
    }
    let fresh = classifier::classify_with_tower_events(&l0, config).2;

    let mut terminal = BTreeMap::<CandidateKey, &CandidateEvent>::new();
    for stream in causal.iter() {
        for event in stream.iter() {
            terminal.insert(event.key, event);
        }
    }
    let mut fresh_latest = BTreeMap::<CandidateKey, &CandidateEvent>::new();
    for stream in fresh.iter() {
        for event in stream.iter() {
            fresh_latest.insert(event.key, event);
        }
    }

    let mut payload_equal = 0usize;
    let mut payload_differ = 0usize;
    let mut payload_differ_live = 0usize;
    let mut revived = 0usize;
    let mut fork_fresh_only = 0usize;
    for (key, event) in &fresh_latest {
        match terminal.get(key) {
            None => fork_fresh_only += 1,
            Some(booked) if booked.state == CandidateState::Invalidated => revived += 1,
            Some(booked) => {
                if booked.projection() == event.projection() {
                    payload_equal += 1;
                } else {
                    payload_differ += 1;
                    if !booked.state.is_terminal() {
                        payload_differ_live += 1;
                    }
                }
            }
        }
    }
    let causal_only: Vec<_> = terminal
        .iter()
        .filter(|(key, _)| !fresh_latest.contains_key(key))
        .collect();
    let fork_causal_only = causal_only.len();
    let fork_causal_only_terminal = causal_only
        .iter()
        .filter(|(_, event)| event.state.is_terminal())
        .count();
    if std::env::var_os("ISSUE551_DIAG").is_some() {
        for (key, event) in causal_only.iter().take(4) {
            println!("ISSUE551_DIAG causal_only key={key:?} state={:?} interval={:?} observed_at={} revision={}",
                event.state, event.interval, event.observed_at, event.revision);
        }
        for (key, event) in &fresh_latest {
            if let Some(booked) = terminal.get(key) {
                if booked.projection() != event.projection() {
                    println!(
                        "ISSUE551_DIAG payload_differ key={key:?}\n  causal  state={:?} interval={:?} preds={:?} revision={}\n  fresh   state={:?} interval={:?} preds={:?}",
                        booked.state, booked.interval, booked.structural_predicates, booked.revision,
                        event.state, event.interval, event.structural_predicates
                    );
                }
            }
        }
    }
    println!(
        "ISSUE551_FORK fresh_identities={} causal_identities={} payload_equal={payload_equal} \
         payload_differ={payload_differ} payload_differ_live={payload_differ_live} \
         fork_causal_only={fork_causal_only} fork_causal_only_terminal={fork_causal_only_terminal} \
         fork_fresh_only={fork_fresh_only} revived={revived}",
        fresh_latest.len(),
        terminal.len(),
    );
    Ok(())
}

/// #641 链簿在 #550 对拍循环上的挂载态（只读该循环的事件流产出）。
struct ChainRun {
    book: chain_cert::ChainCertificateBook,
    every: usize,
    advances: usize,
    last_streams: classifier::cand_event::CandidateStreams,
    last_as_of: usize,
}

fn compare_streams(
    bars: &[Bar],
    config: &ThetaConfig,
    chain_every: usize,
) -> Result<(classifier::cand_event::CandidateStreams, ChainRun), String> {
    let mut parser = ParseLayerIncr::new(config);
    let mut cache = TowerCache::new();
    let mut owned = OwnedIncrementalClassifier::new(config.clone());
    let mut terminal_streams = Rc::new(Vec::new());
    let mut chain = ChainRun {
        book: chain_cert::ChainCertificateBook::default(),
        every: chain_every,
        advances: 0,
        last_streams: Rc::new(Vec::new()),
        last_as_of: 0,
    };
    for (i, bar) in bars.iter().copied().enumerate() {
        let l0 = parser.append(bar);
        let direct = classifier::classify_with_tower_events_incremental(&l0, config, &mut cache);
        let streamed = owned.append_bar_events(bar);
        if direct != streamed {
            return Err(format!("逐 bar 三元通道不等: bar={i}"));
        }
        terminal_streams = streamed.2;
        if (i + 1) % chain_every == 0 || i + 1 == bars.len() {
            chain.book.advance(&terminal_streams, i);
            chain.advances += 1;
            chain.last_streams = Rc::clone(&terminal_streams);
            chain.last_as_of = i;
        }
    }
    Ok((terminal_streams, chain))
}

fn print_summary(bars: &[Bar], terminal_streams: &classifier::cand_event::CandidateStreams) {
    let mut latest = BTreeMap::<CandidateKey, &CandidateEvent>::new();
    let mut revisions = 0usize;
    for stream in terminal_streams.iter() {
        for event in stream.iter() {
            revisions += 1;
            latest.insert(event.key, event);
        }
    }
    let identities = latest.len();
    let mut active_by_level = BTreeMap::<u32, usize>::new();
    let mut active = 0usize;
    let mut pan_identities = 0usize;
    let mut pan_active = 0usize;
    for event in latest.values() {
        if event.kind == CandidateKind::Pan {
            pan_identities += 1;
        }
        if event.state != CandidateState::Invalidated {
            active += 1;
            *active_by_level.entry(event.key.level).or_default() += 1;
            if event.kind == CandidateKind::Pan {
                pan_active += 1;
            }
        }
    }
    println!(
        "ISSUE550_BATTERY bars={} compared={} stream_levels={} identities={} revisions={} active={} levels={:?} pan_identities={} pan_active={}",
        bars.len(),
        bars.len(),
        terminal_streams.len(),
        identities,
        revisions,
        active,
        active_by_level,
        pan_identities,
        pan_active,
    );
    if std::env::var_os("ISSUE550_VERBOSE").is_some() {
        println!("ISSUE550_LATEST {latest:#?}");
    }
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ISSUE550_BATTERY_ERROR {error}");
            std::process::ExitCode::from(1)
        }
    }
}
