//! #1588 memory 分片：解析入口内存/拷贝增长探针（只读诊断，不改任何解析结果）。
//!
//! 口径与 #1493 (E) 探针一致：`btc_1m_full.json`（sha256 先校验）尾 50,000 根再去掉前 5 根，
//! 取前 n 根。每次进程只跑一个 (mode, n)，RSS 高水位（VmHWM）不被前一个 n 污染。
//!
//! 模式：
//! - `facts`：`parse_layer_with_inclusion_facts` 一次全量调用；报告分配峰值、调用后留存字节、
//!   InclusionFacts 中各 `Vec<usize>` 来源坐标的元素数/容量（结构性计数，用来判定增长是否是
//!   输出 schema 本身的体积），以及 layer/facts 的 Debug 流式摘要。
//! - `replay-drop` / `replay-hold`：`ParseLayerIncr::append` 逐 bar 回放；drop = 调用前先丢上一轮
//!   ParseLayer，hold = 上一轮 ParseLayer 在 append 期间仍存活（生产 `classify_at` 修前的持有方式）。
//!   报告累计分配字节、峰值、耗时、逐 K bar 滚动摘要与末 bar 摘要。
//! - `classifier`：`IncrementalClassifier::classify_at` 逐 bar；报告同上，摘要覆盖每 K bar 的
//!   `classification` 与末 bar 的 `last_l0`，用于修前/修后同输入对拍。
//!
//! 用法：`issue1588_parse_mem_probe <btc_1m_full.json> <mode> <n> [digest_every]`
//! （digest_every=0 只摘末 bar；默认 0）。分配计数只含 Rust 全局分配器的请求 layout，不含
//! allocator 元数据；RSS 取 /proc/self/status（仅 Linux 有值）。

use std::alloc::{GlobalAlloc, Layout, System};
use std::fmt::Write as _;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::time::Instant;

use newchan_rust::theta_v0::backtest::data::load_symbol;
use newchan_rust::theta_v0::backtest::incremental::IncrementalClassifier;
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::{
    inclusion::{InclusionDirectionEvidence, InclusionFacts},
    parse_layer_with_inclusion_facts,
    stroke::Endpoint,
    ParseLayer, ParseLayerIncr,
};
use newchan_rust::theta_v0::types::Bar;
use sha2::{Digest, Sha256};

const FULL_SHA256: &str = "16ea13d55f2ae7edcfc503f604a14961fd1a378227afd37c63f23386e894707b";

static LIVE: AtomicU64 = AtomicU64::new(0);
static PEAK: AtomicU64 = AtomicU64::new(0);
static TOTAL: AtomicU64 = AtomicU64::new(0);

struct Counting;
#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn add(bytes: usize) {
    let live = LIVE.fetch_add(bytes as u64, Relaxed) + bytes as u64;
    TOTAL.fetch_add(bytes as u64, Relaxed);
    PEAK.fetch_max(live, Relaxed);
}
fn sub(bytes: usize) {
    LIVE.fetch_sub(bytes as u64, Relaxed);
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            add(layout.size());
        }
        p
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc_zeroed(layout) };
        if !p.is_null() {
            add(layout.size());
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, layout: Layout) {
        unsafe { System.dealloc(p, layout) };
        sub(layout.size());
    }
    unsafe fn realloc(&self, p: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let next = unsafe { System.realloc(p, layout, size) };
        if !next.is_null() {
            sub(layout.size());
            add(size);
        }
        next
    }
}

/// 开窗：峰值重置为当前 live，返回 (live, total) 基线。
fn window() -> (u64, u64) {
    let live = LIVE.load(Relaxed);
    PEAK.store(live, Relaxed);
    (live, TOTAL.load(Relaxed))
}

fn mb(bytes: u64) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

fn proc_status(key: &str) -> String {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with(key))
                .map(|l| l[key.len()..].trim().to_string())
        })
        .unwrap_or_else(|| "n/a".to_string())
}

/// Debug 流式进 sha256，不物化整串（facts 在 20k 时 Debug 串达 GB 级）。
struct HashWriter(Sha256);
impl std::fmt::Write for HashWriter {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0.update(s.as_bytes());
        Ok(())
    }
}
fn hex(h: Sha256) -> String {
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}
fn digest_debug<T: std::fmt::Debug>(v: &T) -> String {
    let mut w = HashWriter(Sha256::new());
    write!(w, "{v:?}").expect("hash writer 不失败");
    hex(w.0)
}

fn load_bars(path: &Path) -> Vec<Bar> {
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("读取 {path:?} 失败: {e}"));
    let sha = hex({
        let mut h = Sha256::new();
        h.update(&bytes);
        h
    });
    assert_eq!(sha, FULL_SHA256, "数据不是 #1493 (E) 口径的全量文件");
    drop(bytes);
    load_symbol(path, "BTCUSDT-1m", &ThetaConfig::default(), 60)
        .expect("load_symbol")
        .bars
}

#[derive(Default)]
struct CoordCount {
    elems: u64,
    cap: u64,
}
impl CoordCount {
    fn vec(&mut self, v: &[usize], cap: usize) {
        self.elems += v.len() as u64;
        self.cap += cap as u64;
    }
    fn ev(&mut self, e: &Option<InclusionDirectionEvidence>) {
        if let Some(e) = e {
            self.vec(&e.source_coords, e.source_coords.capacity());
        }
    }
    fn endpoint(&mut self, e: &Endpoint) {
        self.vec(&e.source_coords, e.source_coords.capacity());
        self.vec(&e.extreme_roots, e.extreme_roots.capacity());
    }
}

fn facts_counts(f: &InclusionFacts) -> [(&'static str, CoordCount); 7] {
    let mut sf_endpoints = CoordCount::default();
    let mut sf_pairs = CoordCount::default();
    let mut sf_strokes = CoordCount::default();
    if let Some(sf) = &f.stroke_facts {
        sf.endpoints.iter().for_each(|e| sf_endpoints.endpoint(e));
        for p in &sf.pairs {
            sf_pairs.endpoint(&p.old);
            sf_pairs.endpoint(&p.new);
            sf_pairs.vec(&p.retained_anchors, p.retained_anchors.capacity());
        }
        for b in &sf.strokes {
            sf_strokes.endpoint(&b.start);
            sf_strokes.endpoint(&b.end);
            if let Some(c) = &b.confirmation {
                sf_strokes.vec(&c.source_coords, c.source_coords.capacity());
            }
        }
    }
    let mut step_src = CoordCount::default();
    let mut step_ev = CoordCount::default();
    let mut group_src = CoordCount::default();
    let mut group_ev = CoordCount::default();
    for s in &f.steps {
        step_src.vec(&s.source_coords, s.source_coords.capacity());
        step_src.vec(&s.high_sources, s.high_sources.capacity());
        step_src.vec(&s.low_sources, s.low_sources.capacity());
        step_ev.ev(&s.direction_evidence);
    }
    for g in &f.groups {
        group_src.vec(&g.members, g.members.capacity());
        group_src.vec(&g.high_sources, g.high_sources.capacity());
        group_src.vec(&g.low_sources, g.low_sources.capacity());
        group_ev.ev(&g.direction_evidence);
        group_ev.ev(&g.confirmation_evidence);
    }
    [
        ("steps.source/high/low", step_src),
        ("steps.direction_evidence", step_ev),
        ("groups.members/high/low", group_src),
        ("groups.direction+confirmation_evidence", group_ev),
        ("stroke_facts.endpoints", sf_endpoints),
        ("stroke_facts.pairs(old/new/anchors)", sf_pairs),
        ("stroke_facts.strokes(start/end/confirmation)", sf_strokes),
    ]
}

fn layer_shape(l: &ParseLayer) -> String {
    format!(
        "merged={} fractals={} strokes={} segments={} tail={}",
        l.merged_bars.len(),
        l.fractals.len(),
        l.strokes.len(),
        l.segments.len(),
        l.tail.len()
    )
}

fn run_facts(bars: &[Bar], config: &ThetaConfig) {
    let (live0, _) = window();
    let t = Instant::now();
    let (layer, facts) = parse_layer_with_inclusion_facts(bars, config);
    let secs = t.elapsed().as_secs_f64();
    let peak = PEAK.load(Relaxed) - live0;
    let retained = LIVE.load(Relaxed) - live0;
    println!(
        "facts secs={secs:.2} peak_mb={:.1} retained_mb={:.1} steps={} groups={} {}",
        mb(peak),
        mb(retained),
        facts.steps.len(),
        facts.groups.len(),
        layer_shape(&layer)
    );
    let mut cap_total = 0u64;
    for (name, c) in facts_counts(&facts) {
        cap_total += c.cap;
        println!(
            "  coords {name}: elems={} cap={} cap_mb={:.1}",
            c.elems,
            c.cap,
            mb(c.cap * 8)
        );
    }
    println!(
        "  coords total cap_mb={:.1} ({:.1}% of retained)",
        mb(cap_total * 8),
        100.0 * (cap_total * 8) as f64 / retained.max(1) as f64
    );
    println!(
        "  digest layer={} facts={}",
        digest_debug(&layer),
        digest_debug(&facts)
    );
}

fn run_replay(bars: &[Bar], config: &ThetaConfig, hold: bool, every: usize) {
    let (live0, total0) = window();
    let t = Instant::now();
    let mut incr = ParseLayerIncr::new(config);
    let mut held: Option<ParseLayer> = None;
    let mut rolling = Sha256::new();
    for (i, bar) in bars.iter().enumerate() {
        if !hold {
            drop(held.take());
        }
        let layer = incr.append(*bar);
        if every > 0 && (i + 1) % every == 0 {
            rolling.update(digest_debug(&layer).as_bytes());
        }
        held = Some(layer);
    }
    let secs = t.elapsed().as_secs_f64();
    let layer = held.expect("n ≥ 1");
    println!(
        "replay-{} secs={secs:.2} alloc_total_mb={:.1} peak_mb={:.1} {}",
        if hold { "hold" } else { "drop" },
        mb(TOTAL.load(Relaxed) - total0),
        mb(PEAK.load(Relaxed) - live0),
        layer_shape(&layer)
    );
    println!(
        "  digest every={every} rolling={} last_layer={}",
        hex(rolling),
        digest_debug(&layer)
    );
}

fn run_classifier(bars: &[Bar], config: &ThetaConfig, every: usize) {
    let (live0, total0) = window();
    let t = Instant::now();
    let mut clf = IncrementalClassifier::new(bars, config);
    let mut rolling = Sha256::new();
    let mut last = None;
    for i in 0..bars.len() {
        let out = clf.classify_at(i);
        if every > 0 && (i + 1) % every == 0 {
            rolling.update(digest_debug(&out.classification).as_bytes());
        }
        last = Some(out);
    }
    let secs = t.elapsed().as_secs_f64();
    let l0 = clf.last_l0().expect("n ≥ 1");
    println!(
        "classifier secs={secs:.2} alloc_total_mb={:.1} peak_mb={:.1} {}",
        mb(TOTAL.load(Relaxed) - total0),
        mb(PEAK.load(Relaxed) - live0),
        layer_shape(l0)
    );
    println!(
        "  digest every={every} rolling={} last_out={} last_l0={}",
        hex(rolling),
        digest_debug(&last.expect("n ≥ 1")),
        digest_debug(l0)
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!(
            "用法: {} <btc_1m_full.json> <facts|replay-drop|replay-hold|classifier> <n> [digest_every]",
            args[0]
        );
        std::process::exit(2);
    }
    let mode = args[2].as_str();
    let n: usize = args[3].parse().expect("n 须为整数");
    let every: usize = args
        .get(4)
        .map_or(0, |s| s.parse().expect("digest_every 须为整数"));

    let all = load_bars(Path::new(&args[1]));
    let tail = &all[all.len() - 50_000..];
    let bars = &tail[5..];
    assert!(n <= bars.len(), "n={n} 超出窗口 {}", bars.len());
    let bars = &bars[..n];
    let config = ThetaConfig::default();
    println!("== mode={mode} n={n}");
    match mode {
        "facts" => run_facts(bars, &config),
        "replay-drop" => run_replay(bars, &config, false, every),
        "replay-hold" => run_replay(bars, &config, true, every),
        "classifier" => run_classifier(bars, &config, every),
        other => {
            eprintln!("未知 mode `{other}`");
            std::process::exit(2);
        }
    }
    println!(
        "  rss VmRSS={} VmHWM={}",
        proc_status("VmRSS:"),
        proc_status("VmHWM:")
    );
}
