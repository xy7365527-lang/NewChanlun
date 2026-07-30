//! #668（N4）桥接对象验收对拍——修复轮 1（#670 影子评审 HIGH-3 回炉，
//! `chanlun/review-results/issue668-n4-fix-round1-20260729.md`）+ 修复轮 2（#670 影子评审第 2 轮
//! FAIL 回炉两条新 HIGH，`chanlun/review-results/issue668-n4-fix-round2-20260729.md`）。
//!
//! ## 门重定（HIGH-3 处置）
//!
//! 旧版本的参照集与生产模块共享同一 v2 键公式**且逐字段同构复写**——两侧共享全部设计判断，
//! 包括错的那些，验不出判据层错误（HIGH-2 就在它眼前而它 cmp=0）。裁定第三轮 supersede ⑤
//! 明文给的替代方案 = 「跨 as_of 平价锁 + 一条会因 HIGH-2 式漏配变红的负控」：
//! - **跨 as_of 真平价锁**：`bsp_bridge::tests::bridge_book_incremental_final_state_equals_fresh_full_replay_from_empty`
//!   （对齐 N1 先例——增量推进 vs 从空簿单次全量重放两个不同驱动，单测覆盖，非本 bin 职责）。
//! - **负控**：`bsp_bridge::tests::trend_event_growth_does_not_orphan_earlier_first_class_point`
//!   （在旧右端等值判据下必然失败，锁住 HIGH-2 那条根因不再复发）。
//!
//! 本 bin 保留的职责收窄为**真独立参照集**——不再对拍 `bridge.heads()`（只含链头，同 episode
//! 多物理点会被折叠），改对拍 `bridge.edges()`（append-only 全量修订历史；单次 fresh-full
//! 窗口内，一个 episode 覆盖的全部物理点在同一次 `advance` 里折叠成一条覆盖点集合，见
//! `bsp_bridge.rs` `resolve_first_class_episode_points` 文档）——参照集独立实现「episode
//! 区间覆盖」这同一条已由 dispatch 第三轮 supersede 裁定settled 的判据（判据本身不再是本 bin
//! 的论证对象，`bsp_bridge.rs` 模块头 + 真值表已界定），但**代码路径不共享**：本 bin 用线性扫描
//! + 独立数据结构，零调用 `bsp_bridge` 内部索引/折叠函数——仍能捕获实现层错误（字段取错/
//! off-by-one/漏判类/边界开闭错），只是不再重新论证「episode 覆盖是不是对的判据」（那件事已经
//! 由三轮 supersede + 单元测试负控关闭）。
//!
//! ## 幂等 + 查询完备性回归门（修复轮 2，R2-HIGH-1/R2-HIGH-2）
//!
//! 修复轮 1 的这条对拍 bin 只 `advance` 一次，测不出「同输入重跑」与「查询入口完备性」——正是
//! 修复轮 1 遗留的两条新 HIGH 藏身之处（评审 #670 第 2 轮）。本轮新增两条硬门，写入退出码：
//! - **幂等**（`ISSUE668_IDEMPOTENCE`）：同一 `(classification, streams, as_of)` 连续 `advance`
//!   三次，第 2/3 次必须零 Delta、边数不再增长。
//! - **查询完备性**（`ISSUE668_QUERY_ENTRY`）：`edges()` 里出现的每个物理点都必须能经
//!   `edges_for_bsp_point` 查到（`query_lost` 必须为 0）。
//!
//! ## 现役拼缝跨对象族不可直接对拍（HIGH-3 ①，如实登记不可执行）
//!
//! 裁定②点名的现役拼缝 `nest::terminal_bits_at_event` 需要 `NestCandidateEvent`（`nest` 模块
//! 自有事件体系，非本票 `cand_event::CandidateEvent`）+ ~~`OwnerAnchorCtx`（owner 判同
//! oracle）~~ + ~~`event_bsp_book_level` 级别移位~~；把桥接对象接进这条拼缝需要新构造一整套
//! nest 侧事件与锚供给，这本身是一次新的消费接线（违反裁定④「p92/π runner 本票零消费接线」+
//! 「不重算既有判据」方法学）。按 dispatch 「跨对象族不可直接对拍则上报改门，不得自替代」的
//! 处置指引，此路在本修复轮判**不可执行**，改用上述两件替代验收物。
//!
//! **订正（R2-LOW-2，第五轮 supersede）**：上面三条障碍里只有 `NestCandidateEvent` 成立且是
//! 唯一承重的一条——全仓无 lib 侧构造入口，产出只在 `p92`/`p123`/`p124` bin 内的
//! `collect_target_candidates`（需 tower + MACD 供给管线），接进来确属新构造一整套供给。
//! `OwnerAnchorCtx` 不成立：`p92` 自己传的是 4 行 `never` stub（自述「本 bin 是归档研究/审计
//! 工具，未接事件锚账本」），复制它零成本；`event_bsp_book_level` 不成立：`nest.rs` 内一行
//! `pub fn`，一行调用。结论不变（此路仍判不可执行，不退回）——收窄为只留
//! `NestCandidateEvent` 这一条真实障碍，删去两条虚障碍，不冒充论证比实际更扎实。
//!
//! ## 分点类计数（第五轮 supersede，修复 #670 R2-MED-3）
//!
//! 旧版本只报总 `cmp`——三窗合计参与比对的边全部是一类（二/三类两侧同为空集），总 `cmp=0`
//! 会让读者误以为三窗对六个点类都验了东西。本轮新增 `ISSUE668_BRIDGE_BATTERY_BY_CLASS`：逐
//! 点类拆开报 reference/produced/missing/extra/cmp，两侧同为 0 时显式标注该类空域、cmp 无
//! 信息量，不与真正验过的一类混在一起数。
//!
//! 用法：`cargo run --release --bin p_issue668_bsp_bridge_battery -- <btc_1m_full.json> [max_bars]`

use std::collections::BTreeSet;
use std::path::Path;

use newchan_rust::theta_v0::classifier::bsp::OwnerRef;
use newchan_rust::theta_v0::classifier::bsp_bridge::{BspBridgeBook, BspPointClass};
use newchan_rust::theta_v0::classifier::cand_event::{CandidateEvent, CandidateKey, CandidateKind, CandidateStreams};
use newchan_rust::theta_v0::classifier::{self, Classification};
use newchan_rust::theta_v0::config::ThetaConfig;
use newchan_rust::theta_v0::parser::ParseLayerIncr;
use newchan_rust::theta_v0::types::{quantize, Bar, Side};
use serde::Deserialize;
use std::collections::BTreeMap;

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

fn timestamp(date: &str) -> i64 {
    let digits: String = date.chars().take_while(|c| *c != '+').filter(char::is_ascii_digit).take(14).collect();
    digits.parse().unwrap_or_else(|_| panic!("日期 {date:?} 解析失败"))
}

fn load(path: &Path, tick_size: f64, limit: usize) -> Vec<Bar> {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("读取 {} 失败: {e}", path.display()))
        .replace("-Infinity", "null")
        .replace("Infinity", "null")
        .replace("NaN", "null");
    let raw: RawBars = serde_json::from_str(&text).unwrap_or_else(|e| panic!("解析失败: {e}"));
    let n = raw.closes.len().min(raw.opens.len()).min(raw.highs.len()).min(raw.lows.len()).min(raw.dates.len()).min(limit);
    let mut bars = Vec::with_capacity(n);
    for i in 0..n {
        let prior = bars.last().map_or(0, |bar: &Bar| bar.close);
        let volume = raw.volumes.get(i).and_then(|value| *value).unwrap_or(0.0);
        let values = match (raw.opens[i], raw.highs[i], raw.lows[i], raw.closes[i]) {
            (Some(open), Some(high), Some(low), Some(close)) => {
                let invalid = high < open.max(close).max(low)
                    || low > open.min(close).min(high)
                    || [open, high, low, close].iter().any(|price| *price <= 0.0);
                (quantize(open, tick_size), quantize(high, tick_size), quantize(low, tick_size), quantize(close, tick_size), invalid)
            }
            _ => (prior, prior, prior, prior, true),
        };
        bars.push(Bar {
            source_index: i,
            timestamp: timestamp(&raw.dates[i]),
            open: values.0,
            high: values.1,
            low: values.2,
            close: values.3,
            volume: volume as i64,
            untradable: values.4 || volume <= 0.0,
        });
    }
    bars
}

/// 独立参照集（真独立：不调用 `bsp_bridge` 任何函数/索引结构，线性扫描 + 自有数据形状）：
/// 一类 = episode 区间覆盖（`c_start <= source_index <= interval.1`，同 level/side/中枢指纹）；
/// 二类 = 其一类锚坐标同样按 episode 区间覆盖反查；三类 = `leave_interval.1` 精确等值（v2 三类
/// 锚本轮未改，评审 #670 已验不空洞）。
fn reference_join(classification: &Classification, streams: &CandidateStreams) -> BTreeSet<(u32, usize, &'static str, CandidateKey)> {
    let mut latest: BTreeMap<CandidateKey, CandidateEvent> = BTreeMap::new();
    for batch in streams.iter() {
        for event in batch.iter() {
            latest.insert(event.key, event.clone());
        }
    }
    // 独立数据形状：Vec 线性表，不建 BTreeMap 索引（刻意与生产 `trend_episodes`/`find_episode`
    // 的实现路径分道——只共享「episode 区间覆盖」这条已被 dispatch 裁定settled 的判据本身）。
    let trend_events: Vec<(u32, Side, (usize, i64, i64), usize, usize, CandidateKey)> = latest
        .values()
        .filter(|event| event.kind == CandidateKind::Trend)
        .map(|event| {
            let p = (event.key.parent.center_start, event.key.parent.zd, event.key.parent.zg);
            (event.event_level, event.key.side, p, event.key.c_start, event.interval.1, event.key)
        })
        .collect();
    let trend_by_exact_end: BTreeMap<(u32, Side, (usize, i64, i64), usize), CandidateKey> = trend_events
        .iter()
        .map(|&(level, side, p, _, end, key)| ((level, side, p, end), key))
        .collect();

    let find_covering = |level: u32, side: Side, parent: (usize, i64, i64), source_index: usize| -> Option<CandidateKey> {
        let mut found: Option<CandidateKey> = None;
        for &(el, es, ep, c_start, interval_end, key) in &trend_events {
            if el == level && es == side && ep == parent && c_start <= source_index && source_index <= interval_end {
                found = Some(key);
                break; // 独立扫描不断言唯一性（生产侧的 debug_assert 已在单测覆盖），取首个。
            }
        }
        found
    };

    let mut out = BTreeSet::new();
    for (level_idx, level) in classification.levels.iter().enumerate() {
        for point in level.bsp.iter() {
            let center_fp = match point.center {
                Some(OwnerRef::Center(c)) => Some((c.start_index, c.zd, c.zg)),
                _ => None,
            };
            // 一类：episode 区间覆盖。
            for (set, side, name) in [(point.bits.buy1, Side::Long, "Buy1"), (point.bits.sell1, Side::Short, "Sell1")] {
                if !set {
                    continue;
                }
                let Some(parent) = center_fp else { continue };
                if let Some(ek) = find_covering(level_idx as u32, side, parent, point.source_index) {
                    out.insert((level_idx as u32, point.source_index, name, ek));
                }
            }
            // 三类：leave_interval.1 精确等值（未改判据）。
            for (set, side, name) in [(point.bits.buy3, Side::Long, "Buy3"), (point.bits.sell3, Side::Short, "Sell3")] {
                if !set {
                    continue;
                }
                let (Some(parent), Some(entry)) = (center_fp, point.bits.third_class_entry) else { continue };
                if let Some(&ek) = trend_by_exact_end.get(&(level_idx as u32, side, parent, entry.leave_interval.1)) {
                    out.insert((level_idx as u32, point.source_index, name, ek));
                }
            }
            // 二类：反查同级一类锚，同样按 episode 区间覆盖（HIGH-2 修复覆盖二类）。
            for (set, side, name, want_buy1) in [
                (point.bits.buy2, Side::Long, "Buy2", true),
                (point.bits.sell2, Side::Short, "Sell2", false),
            ] {
                if !set {
                    continue;
                }
                let anchor_idx = match point.center {
                    Some(OwnerRef::Type1Anchor(idx)) => idx,
                    _ => continue,
                };
                let anchor_parent = level.bsp.iter().find_map(|p| {
                    if p.source_index != anchor_idx {
                        return None;
                    }
                    let hit = if want_buy1 { p.bits.buy1 } else { p.bits.sell1 };
                    if !hit {
                        return None;
                    }
                    match p.center {
                        Some(OwnerRef::Center(c)) => Some((c.start_index, c.zd, c.zg)),
                        _ => None,
                    }
                });
                let Some(parent) = anchor_parent else { continue };
                if let Some(ek) = find_covering(level_idx as u32, side, parent, anchor_idx) {
                    out.insert((level_idx as u32, point.source_index, name, ek));
                }
            }
        }
    }
    out
}

fn class_name(class: BspPointClass) -> &'static str {
    match class {
        BspPointClass::Buy1 => "Buy1",
        BspPointClass::Buy2 => "Buy2",
        BspPointClass::Buy3 => "Buy3",
        BspPointClass::Sell1 => "Sell1",
        BspPointClass::Sell2 => "Sell2",
        BspPointClass::Sell3 => "Sell3",
    }
}

fn main() -> std::process::ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("用法: p_issue668_bsp_bridge_battery <btc_1m_full.json> [max_bars]");
        return std::process::ExitCode::FAILURE;
    };
    let max_bars = args.next().map(|v| v.parse::<usize>().expect("max_bars 非法")).unwrap_or(100_000);
    let config = ThetaConfig::default();
    let bars = load(Path::new(&path), config.tick.tick_size, max_bars);
    if bars.is_empty() {
        eprintln!("空窗口");
        return std::process::ExitCode::FAILURE;
    }

    let mut parser = ParseLayerIncr::new(&config);
    let mut l0 = parser.append(bars[0]);
    for bar in bars.iter().copied().skip(1) {
        l0 = parser.append(bar);
    }
    let (classification, _tower, streams) = classifier::classify_with_tower_events(&l0, &config);
    let as_of = bars.last().map_or(0, |b| b.source_index);

    let mut trend = 0usize;
    let mut pan = 0usize;
    for batch in streams.iter() {
        for event in batch.iter() {
            match event.kind {
                CandidateKind::Trend => trend += 1,
                CandidateKind::Pan => pan += 1,
            }
        }
    }
    let mut b1 = 0usize;
    let mut b2 = 0usize;
    let mut b3 = 0usize;
    let mut b1_by_level: BTreeMap<usize, usize> = BTreeMap::new();
    for (level_idx, level) in classification.levels.iter().enumerate() {
        for p in level.bsp.iter() {
            if p.bits.buy1 || p.bits.sell1 {
                b1 += 1;
                *b1_by_level.entry(level_idx).or_default() += 1;
            }
            if p.bits.buy2 || p.bits.sell2 {
                b2 += 1;
            }
            if p.bits.buy3 || p.bits.sell3 {
                b3 += 1;
            }
        }
    }

    let reference = reference_join(&classification, &streams);
    let mut hit_by_class: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut hit_by_level_b1: BTreeMap<usize, usize> = BTreeMap::new();
    for (level, _, name, _) in &reference {
        *hit_by_class.entry(*name).or_default() += 1;
        if matches!(*name, "Buy1" | "Sell1") {
            *hit_by_level_b1.entry(*level as usize).or_default() += 1;
        }
    }
    println!(
        "ISSUE668_BRIDGE_COVERAGE bars={} trend_events={trend} pan_events={pan} \
         b1_points={b1} b2_points={b2} b3_points={b3} hit_by_class={hit_by_class:?} \
         b1_by_level={b1_by_level:?} b1_hit_by_level={hit_by_level_b1:?}",
        bars.len(),
    );

    let mut bridge = BspBridgeBook::default();
    let delta1 = bridge.advance(&classification, &streams, as_of).len();
    let edges1 = bridge.edges().len();
    // 第四轮 supersede 幂等回归门（R2-HIGH-1）：同一 (classification, streams, as_of) 重跑必须
    // 零 Delta——修复轮 1 在此处每次重跑追加 len(covered) 条 churn revision（评审 #670 复核探针
    // 300k 窗实测每次 +12，边数 29→41→53 无上界增长）。
    let delta2 = bridge.advance(&classification, &streams, as_of).len();
    let edges2 = bridge.edges().len();
    let delta3 = bridge.advance(&classification, &streams, as_of).len();
    let edges3 = bridge.edges().len();
    println!(
        "ISSUE668_IDEMPOTENCE bars={} delta1={delta1} edges1={edges1} delta2={delta2} edges2={edges2} \
         delta3={delta3} edges3={edges3}",
        bars.len(),
    );

    // 对拍全量修订历史（非 heads()）：同 episode 多物理点在单次 fresh-full advance 里折叠为一条
    // revision，覆盖点集合（`bsp_source_indices`，第四轮 supersede）里的每个物理点都应在参照集
    // 里留痕（HIGH-3 处置：真独立参照集对全量历史，不是只对链头）。
    let produced: BTreeSet<(u32, usize, &'static str, CandidateKey)> = bridge
        .edges()
        .iter()
        .flat_map(|edge| {
            edge.bsp_source_indices
                .iter()
                .map(move |source_index| (edge.bsp_level, *source_index, class_name(edge.key.bsp.class), edge.key.event))
        })
        .collect();

    let missing_in_bridge: Vec<_> = reference.difference(&produced).collect();
    let extra_in_bridge: Vec<_> = produced.difference(&reference).collect();
    let cmp = missing_in_bridge.len() + extra_in_bridge.len();

    println!(
        "ISSUE668_BRIDGE_BATTERY bars={} reference_edges={} produced_edges={} heads={} missing_in_bridge={} extra_in_bridge={} cmp={}",
        bars.len(),
        reference.len(),
        produced.len(),
        bridge.heads().len(),
        missing_in_bridge.len(),
        extra_in_bridge.len(),
        cmp,
    );
    for item in missing_in_bridge.iter().take(5) {
        println!("ISSUE668_BRIDGE_MISSING {item:?}");
    }
    for item in extra_in_bridge.iter().take(5) {
        println!("ISSUE668_BRIDGE_EXTRA {item:?}");
    }

    // 分点类计数行（R2-MED-3：三窗合计参与比对的 32 条边全为一类，二/三类两侧同为空集，总
    // cmp=0 掩盖了这一点——读者会误以为三窗都验了六个点类）。逐点类拆开报 reference/produced/
    // missing/extra/cmp；两侧同为 0 时明确标注该类空域、cmp 无信息量，不冒充「验过了」。
    for class in ["Buy1", "Buy2", "Buy3", "Sell1", "Sell2", "Sell3"] {
        let ref_n = reference.iter().filter(|item| item.2 == class).count();
        let prod_n = produced.iter().filter(|item| item.2 == class).count();
        let miss_n = missing_in_bridge.iter().filter(|item| item.2 == class).count();
        let extra_n = extra_in_bridge.iter().filter(|item| item.2 == class).count();
        let class_cmp = miss_n + extra_n;
        let note = if ref_n == 0 && prod_n == 0 { " note=此类空域，cmp无信息量" } else { "" };
        println!(
            "ISSUE668_BRIDGE_BATTERY_BY_CLASS bars={} class={class} reference={ref_n} produced={prod_n} \
             missing={miss_n} extra={extra_n} cmp={class_cmp}{note}",
            bars.len(),
        );
    }

    // 查询入口完备性回归门（R2-HIGH-2）：edges() 里出现的每个物理点都必须能经
    // edges_for_bsp_point 查到——修复轮 1 在此处 300k 窗实测 7/29 点静默查无（失效后链头回退
    // 到遍历序首个物理点，其余点从 heads() 过滤后的查询入口消失）。
    let distinct_points: BTreeSet<(u32, usize)> =
        bridge.edges().iter().flat_map(|edge| edge.bsp_source_indices.iter().map(move |src| (edge.bsp_level, *src))).collect();
    let query_lost = distinct_points
        .iter()
        .filter(|&&(level, source_index)| bridge.edges_for_bsp_point(level, source_index).is_empty())
        .count();
    println!(
        "ISSUE668_QUERY_ENTRY bars={} distinct_points_with_edges={} query_reachable={} query_lost={}",
        bars.len(),
        distinct_points.len(),
        distinct_points.len() - query_lost,
        query_lost,
    );

    let idempotence_ok = delta2 == 0 && delta3 == 0 && edges1 == edges2 && edges2 == edges3;
    if cmp == 0 && idempotence_ok && query_lost == 0 {
        std::process::ExitCode::SUCCESS
    } else {
        std::process::ExitCode::FAILURE
    }
}
