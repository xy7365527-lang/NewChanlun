//! # `p856_ed_dominance` —— ED 命题的数据检验（wayfinder 票 #856，map #854），只读探针
//!
//! ## 被测命题（ED，map #854 第二轮推导 (ED) 式）
//!
//! > `Ext^str_q(T)  ⟹  F_{q,c}(k) ≥ F_{q,b}(k)  ∀k`
//!
//! 即：同一趋势中，若段 `c` 之后**继续**产生同级别同向的新中枢（延伸确认），
//! 则 `c` 的「中枢停留级别分布」一阶随机占优于前一连接段 `b`
//! （占优方向 = 更多质量落在「无中枢或更低级别中枢」，见 #854 第一轮 (1) 式）。
//!
//! **只读**：本探针不改任何生产判据，只调用生产入口 `parse_layer` + `classify_with_tower`
//! 读终态塔，然后在塔上做纯统计。
//!
//! ## 口径选择（票面硬纪律：每个选择必须明写）
//!
//! ### 选择 1：原子 = 原始 1 分钟 K 线（bar）
//!
//! `Ω_s` = 段 `s` 覆盖的 bar 集合，`μ` = 计数测度，`μ̄_s` = 段内归一化。
//! **为什么取 bar**：
//! - (a) ED 要比较的是**跨时段**两个段的分布，两段的原子必须落在同一个类型空间上。
//!   bar 是全塔唯一在所有级别、所有段上都同质存在的原子（#854 第一轮 §1 明确说 μ 未被
//!   原文指定，计数测度只是候选——本探针明写它是**本仓的选择**，不是缠师的口径）。
//! - (b) 「停留」在中文语义上是**时间上待在里面**，bar 是本仓最细的时间刻度。取次级别走势单元
//!   作原子会把「一根横盘很久的单元」和「一根瞬时单元」计成等权，与「停留」相悖。
//! - (c) 次级别单元作原子的口径本探针**同时**跑一条独立对照臂（`atom=unit`，λ 定义同构、
//!   原子换成 L0 线段），不作兜底、两臂各自独立出数（AGENTS.md #799：禁「严格版失败换宽松版」）。
//!
//! ### 选择 2：λ（相对停留级别）= 覆盖该原子的、**级别严格低于本趋势级别**的最高中枢级别；无 ⟹ ⊥
//!
//! 本仓塔中，`tower[j]`（j≥1）的每个 `LeveledMove` **就是一个中枢**（`compose_level` 一窗一中枢，
//! `centers.len() == upper_moves.len()`），其 `start_index/end_index` 是该中枢（seed 三段 + 延伸段）
//! 在原始 K 序上的跨度。对**中枢级别为 k 的趋势**（即 #854 记号里的 q = k+1），bar `t` 的相对停留级别
//!   `λ_k(t) = max { j-1 : 1 ≤ j ≤ k, ∃ m ∈ tower[j], m.start_index ≤ t ≤ m.end_index }`，无命中 ⟹ `⊥`。
//! - **必须按趋势级别截断（这是本探针第一版的实测错误，已订正并留痕）**：#854 第一轮明写
//!   `λ_{q,s}: Ω_s → {⊥,0,…,q-1}`——分布是**相对**本级别的。若不截断而取全塔最高级别，整条趋势
//!   被同一个更高级别中枢罩住，b 与 c 的分布双双退化成单点、约 82% 的边判成 `EQUAL`，命题被口径架空。
//! - **取 max 而非 min**：嵌套时「停留在更大的那个中枢里」才是力度弱的那件事
//!   （`018:52`「相连走势类型的级别越低，力度越大」）。取 min 会让每个进过 L0 中枢的原子都记 0。
//! - **⊥ 排在 0 之前**（`⊥ < 0 < 1 < …`，#854 第一轮约定）：不在任何中枢里 = 纯离开段 = 最强。
//! - **中枢跨度取窗口外缘（含延伸段），不取核心 [ZD,ZG] 的价格穿越**：「停留」是结构上属于这个
//!   中枢的时间，本仓中枢的结构跨度就是它的窗口。**代价照实**：这会把「已离开核心但仍在延伸窗口内」
//!   的原子记为停留。
//! - **k=0（L0 中枢构成的趋势）没有可用级别**：`{⊥,…,q-1} = {⊥}`，分布恒为单点 ⟹ **测不出来**，
//!   本探针把它单列为 `DEGENERATE`，既不并入占优也不并入反例（090 纪律）。
//! - **绝对级别即可**：一阶随机占优在级别标签的任何严格递增重标下不变，且 b、c 同属一个趋势、
//!   q 相同 ⟹ 直接用绝对级别，不需要减基准。
//!
//! ### 选择 3：趋势与「确认的延伸边」
//!
//! - 级别 k 的中枢序列 = `tower[k+1]`（按 K 序）。
//! - **趋势** = 极大的连续同向不重叠中枢串 `C_1..C_m`（m≥2）：上涨要求 `ZD_{i+1} > ZG_i`，
//!   下跌要求 `ZG_{i+1} < ZD_i`（**核心口径 ZD/ZG，非外缘 DD/GG**）。
//! - **连接段** `s_t` = 中枢 `C_t` 与 `C_{t+1}` 之间的那段走势。
//! - **确认的延伸边** `(b,c) = (s_t, s_{t+1})`：`c` 走完之后确实又出现同级别同向的新中枢 `C_{t+2}`
//!   ⟹ `Ext^str` 成立。故延伸边总数 = Σ_趋势 max(m-2, 0)。
//! - **对照组（非延伸边）** = 每条趋势**最后**一对相邻连接段 `(s_{m-2}, s_{m-1})`，其后趋势断裂
//!   （下一个中枢与之重叠或反向）⟹ `Ext^str` 不成立，ED 对它无预言。
//!
//! ### 选择 4：连接段的边界 —— 两条独立臂
//!
//! - **臂 A（strict，主口径）**：`s_t` = bar 开区间 `(C_t.end_index, C_{t+1}.start_index)`，
//!   即严格夹在两个中枢窗口之间的 bar。忠实于「连接段不属于任何一个中枢」。
//!   **已知代价**：本仓 canonical 扫描在非延伸单元处**从该单元本身**重新 seed，故该单元被划进
//!   下一个中枢的窗口 ⟹ 开区间可能为空。为空即**测不出来**（`undefined`），单列不并入两侧。
//! - **臂 B（closed，含端点）**：`s_t` = 闭区间 `[C_t.end_index, C_{t+1}.start_index]`。
//!   依据 = 缠论中离开段与中枢共享端点（离开段起于中枢内最后一个次级别走势的终点）。恒非空。
//! - 两臂**并列独立出数**，不互为兜底。
//!
//! ## 判据
//!
//! `dominates(c over b)` ⟺ 对每个阈值 `ℓ ∈ {⊥,0,…,q-1}`，`F_c(ℓ) ≥ F_b(ℓ)`。
//! 同时记录反向：两向同时成立 ⟺ 两分布相等（`equal`，属 ED 的弱等号侧，计入「占优」）；
//! `c` 不占优 `b` ⟺ `violates` ⟺ **ED 的反例**。
//!
//! ## 用法
//! ```text
//! cargo run --release --features backtest_bin --bin p856_ed_dominance -- [SYMBOL]
//! ```

use newchan_rust::theta_v0::backtest::data::load_by_symbol;
use newchan_rust::theta_v0::classifier::descend::RMove;
use newchan_rust::theta_v0::classifier::recursive_tower::LeveledMove;
use newchan_rust::theta_v0::classifier::{classify_with_tower, Classification};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::parse_layer;
use newchan_rust::theta_v0::types::{Center, Direction};
use std::collections::BTreeMap;
use std::rc::Rc;

/// ⊥ 的数值编码（`⊥ < 0 < 1 < …`）。
const BOT: i32 = -1;

/// 段内 λ 直方图（key = λ 编码，value = 原子数）。
type Hist = BTreeMap<i32, u64>;

fn hist_of(lambda: &[i32], lo: usize, hi: usize) -> Hist {
    let mut h: Hist = BTreeMap::new();
    if lo > hi || lo >= lambda.len() {
        return h;
    }
    let hi = hi.min(lambda.len() - 1);
    for v in &lambda[lo..=hi] {
        *h.entry(*v).or_default() += 1;
    }
    h
}

fn total(h: &Hist) -> u64 {
    h.values().sum()
}

/// 累计分布 `F(ℓ) = ν({⊥,…,ℓ})`，阈值枚举到 `lambda_max`（= q-1）。
fn cdf(h: &Hist, lambda_max: i32) -> Vec<f64> {
    let n = total(h) as f64;
    let mut out = Vec::new();
    let mut acc = 0u64;
    let mut l = BOT;
    while l <= lambda_max {
        acc += h.get(&l).copied().unwrap_or(0);
        out.push(acc as f64 / n);
        l += 1;
    }
    out
}

/// `a` 一阶随机占优于 `b`（本命题方向：质量更集中在低级别）⟺ `F_a(ℓ) ≥ F_b(ℓ) ∀ℓ`。
fn dominates(fa: &[f64], fb: &[f64]) -> bool {
    fa.iter().zip(fb.iter()).all(|(x, y)| *x >= *y - 1e-12)
}

fn bucket_name(len: usize) -> &'static str {
    match len {
        0 => "0",
        1..=9 => "1-9",
        10..=99 => "10-99",
        100..=999 => "100-999",
        1000..=9999 => "1k-9k",
        _ => "10k+",
    }
}

fn center_of(m: &LeveledMove) -> Option<Center> {
    match &m.rmove {
        RMove::Compose { centers, .. } => centers.first().copied(),
        RMove::Segment { .. } => None,
    }
}

/// 相邻两中枢的趋势方向（核心口径 ZD/ZG；重叠 ⟹ 非趋势链接）。
fn link_dir(a: &Center, b: &Center) -> Option<Direction> {
    if b.zd > a.zg {
        Some(Direction::Up)
    } else if b.zg < a.zd {
        Some(Direction::Down)
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    /// c 严格占优 b。
    Dominates,
    /// 两分布相等（互相占优）——ED 的弱等号侧，计入「占优」。
    Equal,
    /// ED 反例·成因①：**反向**——b 严格占优 c（分布真的朝相反方向走）。
    ReverseDominates,
    /// ED 反例·成因②：**不可比**——两向都不占优（一阶随机占优是偏序，交叉的 CDF 无序）。
    Incomparable,
    /// 段为空（臂 A 开区间可能为空）⟹ 测不出来，不并入任何一侧。
    Undefined,
}

struct EdgeResult {
    verdict: Verdict,
    level: usize,
    dir: Direction,
    b_span: (usize, usize),
    c_span: (usize, usize),
    b_hist: Hist,
    c_hist: Hist,
}

fn judge(
    level: usize,
    dir: Direction,
    b: Option<(usize, usize)>,
    c: Option<(usize, usize)>,
    lambda: &[i32],
    lambda_max: i32,
) -> EdgeResult {
    let (Some(bs), Some(cs)) = (b, c) else {
        return EdgeResult {
            verdict: Verdict::Undefined,
            level,
            dir,
            b_span: b.unwrap_or((0, 0)),
            c_span: c.unwrap_or((0, 0)),
            b_hist: Hist::new(),
            c_hist: Hist::new(),
        };
    };
    let bh = hist_of(lambda, bs.0, bs.1);
    let ch = hist_of(lambda, cs.0, cs.1);
    if bh.is_empty() || ch.is_empty() {
        return EdgeResult {
            verdict: Verdict::Undefined,
            level,
            dir,
            b_span: bs,
            c_span: cs,
            b_hist: bh,
            c_hist: ch,
        };
    }
    let fb = cdf(&bh, lambda_max);
    let fc = cdf(&ch, lambda_max);
    let c_dom = dominates(&fc, &fb);
    let b_dom = dominates(&fb, &fc);
    let verdict = match (c_dom, b_dom) {
        (true, true) => Verdict::Equal,
        (true, false) => Verdict::Dominates,
        (false, true) => Verdict::ReverseDominates,
        (false, false) => Verdict::Incomparable,
    };
    EdgeResult {
        verdict,
        level,
        dir,
        b_span: bs,
        c_span: cs,
        b_hist: bh,
        c_hist: ch,
    }
}

fn fmt_hist(h: &Hist) -> String {
    if h.is_empty() {
        return "{}".into();
    }
    let n = total(h);
    let parts: Vec<String> = h
        .iter()
        .map(|(k, v)| {
            let key = if *k == BOT {
                "bot".to_string()
            } else {
                k.to_string()
            };
            format!("{key}:{v}")
        })
        .collect();
    format!("n={n}[{}]", parts.join(","))
}

#[derive(Default)]
struct Tally {
    dominates: u64,
    equal: u64,
    reverse: u64,
    incomparable: u64,
    undefined: u64,
}

impl Tally {
    fn add(&mut self, v: Verdict) {
        match v {
            Verdict::Dominates => self.dominates += 1,
            Verdict::Equal => self.equal += 1,
            Verdict::ReverseDominates => self.reverse += 1,
            Verdict::Incomparable => self.incomparable += 1,
            Verdict::Undefined => self.undefined += 1,
        }
    }
    /// ED 反例总数（成因两类之和）。
    fn violates(&self) -> u64 {
        self.reverse + self.incomparable
    }
    fn measurable(&self) -> u64 {
        self.dominates + self.equal + self.violates()
    }
    fn total(&self) -> u64 {
        self.measurable() + self.undefined
    }
    fn merge(&mut self, o: &Tally) {
        self.dominates += o.dominates;
        self.equal += o.equal;
        self.reverse += o.reverse;
        self.incomparable += o.incomparable;
        self.undefined += o.undefined;
    }
}

fn main() -> std::process::ExitCode {
    let symbol = std::env::args().nth(1).unwrap_or_else(|| "BTC".to_string());
    let config = ThetaConfig::default();
    let dataset = match load_by_symbol(&symbol, &config) {
        Ok(ds) => ds,
        Err(e) => {
            eprintln!("数据加载失败: {e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    let bars = &dataset.bars;
    let n_bars = bars.len();
    eprintln!("P856 载入 {symbol} bars={n_bars}");

    let l0 = parse_layer(bars, &config);
    eprintln!("P856 parse_layer 完成 segments={}", l0.segments.len());
    let (classification, tower): (Classification, Vec<Rc<Vec<LeveledMove>>>) =
        classify_with_tower(&l0, &config);
    eprintln!("P856 塔构造完成 tower_levels={}", tower.len());
    println!("P856_INPUT symbol={symbol} bars={n_bars} l0_segments={}", l0.segments.len());
    for (j, lv) in tower.iter().enumerate() {
        println!("P856_TOWER level={j} moves={}", lv.len());
    }
    for (k, lv) in classification.levels.iter().enumerate() {
        println!("P856_CLASSIF level={k} centers={}", lv.centers.len());
    }

    // ── λ 场（选择 2）：随级别递增**逐级叠加** ⟹ 测 level=k 的趋势时，
    //    lambda_* 恰含级别 0..k-1 的中枢（相对截断，对齐 #854 的 {⊥,0,…,q-1}）。
    let units = tower[0].clone();
    let n_units = units.len();
    let mut lambda_bar: Vec<i32> = vec![BOT; n_bars];
    let mut lambda_unit: Vec<i32> = vec![BOT; n_units];

    let mut tallies: BTreeMap<(&str, &str, &str, usize), Tally> = BTreeMap::new();
    let mut violations: Vec<(&str, &str, EdgeResult)> = Vec::new();
    let mut n_trend = 0u64;
    let mut trend_len_dist: BTreeMap<(usize, usize), u64> = BTreeMap::new();
    let mut gap_bucket: BTreeMap<(usize, &str), u64> = BTreeMap::new();
    let mut degenerate: BTreeMap<(&str, usize), u64> = BTreeMap::new();

    for (j, lv) in tower.iter().enumerate() {
        if j == 0 {
            continue;
        }
        let level = j - 1; // 本轮要测的趋势的中枢级别 k
        let lambda_max = level as i32 - 1; // = q-1 的相对截断上界；level=0 ⟹ 只有 ⊥
        let cs: Vec<Center> = lv.iter().filter_map(center_of).collect();
        if cs.len() == lv.len() && cs.len() >= 2 {
            let mut i = 0usize;
            while i + 1 < cs.len() {
                let Some(d0) = link_dir(&cs[i], &cs[i + 1]) else {
                    i += 1;
                    continue;
                };
                let mut end = i + 1;
                while end + 1 < cs.len() {
                    match link_dir(&cs[end], &cs[end + 1]) {
                        Some(d) if d == d0 => end += 1,
                        _ => break,
                    }
                }
                let m = end - i + 1;
                n_trend += 1;
                *trend_len_dist.entry((level, m)).or_default() += 1;

                let seg_span = |t: usize, closed: bool| -> Option<(usize, usize)> {
                    let a_end = lv[t].end_index;
                    let b_start = lv[t + 1].start_index;
                    if closed {
                        Some((a_end.min(b_start), a_end.max(b_start)))
                    } else if b_start >= a_end + 2 {
                        Some((a_end + 1, b_start - 1))
                    } else {
                        None
                    }
                };
                let seg_units = |t: usize, closed: bool| -> Option<(usize, usize)> {
                    let (lo, hi) = seg_span(t, closed)?;
                    let ul = units.partition_point(|u| u.start_index < lo);
                    let uh = units.partition_point(|u| u.end_index <= hi);
                    if uh > ul {
                        Some((ul, uh - 1))
                    } else {
                        None
                    }
                };

                for t in i..end {
                    let len = seg_span(t, false).map(|(a, b)| b - a + 1).unwrap_or(0);
                    *gap_bucket.entry((level, bucket_name(len))).or_default() += 1;
                }

                // 待判的边：延伸边 t = i..end-2；对照边 = 该趋势最后一对
                let mut edges: Vec<(&str, usize)> = Vec::new();
                for t in i..end {
                    if t + 1 < end {
                        edges.push(("ext", t));
                    }
                }
                if end - i >= 2 {
                    edges.push(("ctrl", end - 2));
                }
                for (grp, t) in edges {
                    if lambda_max < 0 {
                        // level==0 ⟹ λ 值域 = {⊥} 单点，分布恒退化 ⟹ 测不出来（090 纪律）
                        *degenerate.entry((grp, level)).or_default() += 1;
                        continue;
                    }
                    for (bnd, closed) in [("strict", false), ("closed", true)] {
                        let rb = judge(
                            level,
                            d0,
                            seg_span(t, closed),
                            seg_span(t + 1, closed),
                            &lambda_bar,
                            lambda_max,
                        );
                        let rbv = rb.verdict;
                        if matches!(
                            rbv,
                            Verdict::ReverseDominates | Verdict::Incomparable
                        ) && grp == "ext"
                        {
                            violations.push(("bar", bnd, rb));
                        }
                        tallies
                            .entry((grp, "bar", bnd, level))
                            .or_default()
                            .add(rbv);
                        let ru = judge(
                            level,
                            d0,
                            seg_units(t, closed),
                            seg_units(t + 1, closed),
                            &lambda_unit,
                            lambda_max,
                        );
                        let ruv = ru.verdict;
                        if matches!(
                            ruv,
                            Verdict::ReverseDominates | Verdict::Incomparable
                        ) && grp == "ext"
                        {
                            violations.push(("unit", bnd, ru));
                        }
                        {
                            tallies
                                .entry((grp, "unit", bnd, level))
                                .or_default()
                                .add(ruv);
                        }
                    }
                }
                i = end;
            }
        }

        // 叠加本级中枢（级别 = level）进 λ 场，供下一轮（level+1）使用
        for m in lv.iter() {
            if n_bars > 0 {
                let lo = m.start_index.min(n_bars - 1);
                let hi = m.end_index.min(n_bars - 1);
                for t in lo..=hi {
                    if lambda_bar[t] < level as i32 {
                        lambda_bar[t] = level as i32;
                    }
                }
            }
            let ul = units.partition_point(|u| u.start_index < m.start_index);
            let uh = units.partition_point(|u| u.end_index <= m.end_index);
            for it in lambda_unit.iter_mut().take(uh).skip(ul) {
                if *it < level as i32 {
                    *it = level as i32;
                }
            }
        }
    }

    println!("P856_TRENDS n={n_trend}");
    for ((lv, m), c) in &trend_len_dist {
        println!("P856_TREND_LEN level={lv} centers={m} n={c}");
    }
    for ((lv, b), c) in &gap_bucket {
        println!("P856_GAP_BARS level={lv} strict_len_bucket={b} n={c}");
    }
    for ((grp, lv), c) in &degenerate {
        println!("P856_DEGENERATE group={grp} level={lv} edges={c} reason=lambda_space_is_singleton_bot");
    }
    for ((grp, atom, bnd, lv), t) in &tallies {
        println!(
            "P856_TALLY group={grp} atom={atom} boundary={bnd} level={lv} total={} measurable={} dominates={} equal={} violates={} v_reverse={} v_incomparable={} undefined={}",
            t.total(),
            t.measurable(),
            t.dominates,
            t.equal,
            t.violates(),
            t.reverse,
            t.incomparable,
            t.undefined
        );
    }
    // 汇总（跨级别）
    for grp in ["ext", "ctrl"] {
        for atom in ["bar", "unit"] {
            for bnd in ["strict", "closed"] {
                let mut agg = Tally::default();
                for ((g, a, b, _), t) in &tallies {
                    if *g == grp && *a == atom && *b == bnd {
                        agg.merge(t);
                    }
                }
                println!(
                    "P856_SUMMARY group={grp} atom={atom} boundary={bnd} total={} measurable={} dominates={} equal={} violates={} v_reverse={} v_incomparable={} undefined={}",
                    agg.total(),
                    agg.measurable(),
                    agg.dominates,
                    agg.equal,
                    agg.violates(),
                    agg.reverse,
                    agg.incomparable,
                    agg.undefined
                );
            }
        }
    }
    // 反例明细（最多 60 条）
    violations.sort_by_key(|(_, _, r)| (r.level, r.b_span.0));
    let date_at = |i: usize| -> &str {
        dataset
            .dates
            .get(i)
            .map(|s| s.as_str())
            .unwrap_or("?")
    };
    for (atom, bnd, r) in violations.iter().rev().take(80) {
        println!(
            "P856_VIOLATION atom={atom} boundary={bnd} level={} cause={} dir={:?} b=[{},{}] c=[{},{}] b_date={} c_date_end={} b_hist={} c_hist={}",
            r.level,
            match r.verdict {
                Verdict::ReverseDominates => "reverse",
                Verdict::Incomparable => "incomparable",
                _ => "?",
            },
            r.dir,
            r.b_span.0,
            r.b_span.1,
            r.c_span.0,
            r.c_span.1,
            date_at(r.b_span.0),
            date_at(r.c_span.1),
            fmt_hist(&r.b_hist),
            fmt_hist(&r.c_hist)
        );
    }
    println!("P856_VIOLATION_ROWS n={}", violations.len());
    std::process::ExitCode::SUCCESS
}
