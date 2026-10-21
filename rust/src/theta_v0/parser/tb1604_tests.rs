//! #1604（#1586 C-04 / A-12）：尾窗与相切边界并入具名记录后的行为锁。
//!
//! - C-04：尾窗 `tail_window` 读 `second-seq-scan-budget` 记录。窗口 ≥ 3 时与不设上限（0）逐位
//!   相同（只有标准化末元素可变，窗口 (i-1,i,i+1) 在 `len ≥ i+3` 时已定稿）；唯一差异是诊断
//!   轨迹 `checks`——全扫会对已判过的峰重复求值并多记几条重复检查，判定结果不变。窗口 1 会挤掉
//!   候选，此时输出须带 `INCOMPLETE_RESOURCE_BOUND` 标记，不得静默改写结果；增量标记与全量一致。
//! - A-12：相切 `tangency` 读 `segment-second` 记录。两档在相切语料上产出不同段划分——谓词若
//!   回退为常量（任一档），`open` 研究覆写就与 `closed` 相同，本文件即红。
use super::super::config::{ParseConfig, Tangency, ThetaConfig};
use super::super::types::{Direction, Segment, Stroke};
use super::segment::{self, IncrSegments, SegmentFacts};

/// 逐价构造交替笔链（价差符号定方向）。
fn chain(prices: &[i64]) -> Vec<Stroke> {
    prices
        .windows(2)
        .enumerate()
        .map(|(i, p)| Stroke {
            direction: if p[1] > p[0] {
                Direction::Up
            } else {
                Direction::Down
            },
            start_index: i * 4 + 1,
            end_index: i * 4 + 5,
            start_price: p[0],
            end_price: p[1],
        })
        .collect()
}

/// 确定性伪随机锯齿（LCG，无外部依赖）：`amp` 小则相切、等价高低点频繁。
fn zigzag(seed: u64, n: usize, amp: u64) -> Vec<i64> {
    let mut x = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let mut p = 100i64;
    let mut prices = vec![p];
    for i in 0..n {
        x = x
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let step = 1 + ((x >> 33) % amp) as i64;
        p += if i % 2 == 0 { step } else { -step };
        prices.push(p);
    }
    prices
}

fn corpus() -> Vec<Vec<Stroke>> {
    let mut out = Vec::new();
    for seed in 0..120u64 {
        for amp in [3u64, 6, 15] {
            out.push(chain(&zigzag(seed, 60, amp)));
        }
    }
    out
}

fn with_tail(tw: u32) -> ParseConfig {
    let mut pc = ThetaConfig::default().parse;
    pc.research_override_tail_window(tw);
    pc
}

/// 去掉记录戳后的语义结果（研究覆写按设计戳 `…/research-override`）。
fn semantic(mut f: SegmentFacts) -> SegmentFacts {
    f.policy = None;
    f
}

/// 两份事实的 JSON 差异路径（失败信息只列差异处，不整份打印），按「是否落在诊断轨迹 `checks`
/// 内」分成两组：`(语义差异, 轨迹差异条数)`。
fn json_diff(a: &SegmentFacts, b: &SegmentFacts) -> (Vec<String>, usize) {
    fn walk(a: &serde_json::Value, b: &serde_json::Value, path: String, out: &mut Vec<String>) {
        use serde_json::Value::{Array, Object};
        match (a, b) {
            (Object(x), Object(y)) => {
                for k in x.keys().chain(y.keys().filter(|k| !x.contains_key(*k))) {
                    let null = serde_json::Value::Null;
                    let (va, vb) = (x.get(k).unwrap_or(&null), y.get(k).unwrap_or(&null));
                    walk(va, vb, format!("{path}.{k}"), out);
                }
            }
            (Array(x), Array(y)) if x.len() == y.len() => {
                for (i, (va, vb)) in x.iter().zip(y).enumerate() {
                    walk(va, vb, format!("{path}[{i}]"), out);
                }
            }
            _ if a != b => out.push(format!("{path}: {a} ≠ {b}")),
            _ => {}
        }
    }
    let mut out = Vec::new();
    let (a, b) = (
        serde_json::to_value(a).unwrap(),
        serde_json::to_value(b).unwrap(),
    );
    walk(&a, &b, String::new(), &mut out);
    let (trace, mut semantic): (Vec<_>, Vec<_>) =
        out.into_iter().partition(|p| p.contains(".checks"));
    semantic.truncate(8);
    (semantic, trace.len())
}

/// 增量逐笔追加到末尾后的段结果与尾窗标记。
fn incremental(ss: &[Stroke], pc: &ParseConfig) -> (Vec<Segment>, Option<usize>) {
    let mut st = IncrSegments::empty();
    for k in 1..=ss.len() {
        st = st.append(&ss[..k], pc);
    }
    let (segs, _) = st.to_result_rc();
    (segs.to_vec(), st.resource_bound_from())
}

#[test]
fn tb1604_signed_record_reaches_parse_config_and_facts() {
    let pc = ThetaConfig::default().parse;
    assert_eq!(pc.tail_window, 7);
    assert_eq!(pc.scan_budget.label(), "second-seq-scan-budget/2");
    assert_eq!(pc.construction_rule.label(), segment::CONSTRUCTION_RULE);
    let ss = chain(&zigzag(7, 40, 9));
    let (_, _, facts) = segment::divide_segments_with_facts(&ss, &pc);
    let policy = facts.policy.expect("每次划分输出须带所读记录");
    assert_eq!(policy.scan_budget, "second-seq-scan-budget/2");
    assert_eq!(policy.construction_rule, segment::CONSTRUCTION_RULE);
    assert_eq!(policy.tail_window, pc.tail_window);
    assert_eq!(policy.tangency, pc.tangency.as_str());
    assert!(facts.tail_window_bound.is_none(), "签署尾窗 7 不应截断");
}

/// C-04 7 vs ∞：窗口 3 / 7 与 0 的段、尾部、段事实（除诊断轨迹 `checks`）逐位相同且无标记；
/// 窗口 1 凡与 0 不同处必带标记，且至少一例实测截断（证明标记路径活着）；增量标记 == 全量标记。
#[test]
fn tb1604_tail_window_ge3_equals_unbounded_and_tw1_is_marked() {
    let unbounded = with_tail(0);
    let mut marked = 0usize;
    let mut differed = 0usize;
    let mut trace_only = [0usize; 2];
    for (i, ss) in corpus().iter().enumerate() {
        let (s0, p0, f0) = segment::divide_segments_with_facts(ss, &unbounded);
        assert!(
            f0.tail_window_bound.is_none(),
            "#{i}: 不设上限时不得有尾窗标记"
        );
        let f0 = semantic(f0);
        for (k, tw) in [3u32, 7].into_iter().enumerate() {
            let (s, p, f) = segment::divide_segments_with_facts(ss, &with_tail(tw));
            assert_eq!((&s, p), (&s0, p0), "#{i}: tw={tw} 段与尾部须与 ∞ 逐位相同");
            let (d, trace) = json_diff(&semantic(f), &f0);
            assert!(d.is_empty(), "#{i}: tw={tw} 段事实与 ∞ 不同：{d:?}");
            trace_only[k] += usize::from(trace > 0);
        }
        let pc1 = with_tail(1);
        let (s1, p1, f1) = segment::divide_segments_with_facts(ss, &pc1);
        let bound = f1.tail_window_bound.as_ref().map(|b| b.seg_start);
        if let Some(b) = f1.tail_window_bound.as_ref() {
            marked += 1;
            assert_eq!(b.policy, "second-seq-scan-budget/research-override");
        }
        if (&s1, p1) != (&s0, p0) {
            differed += 1;
            assert!(
                bound.is_some(),
                "#{i}: tw=1 改变了结果却未标 INCOMPLETE_RESOURCE_BOUND"
            );
        }
        let (si, bi) = incremental(ss, &pc1);
        assert_eq!(bi, bound, "#{i}: 增量尾窗标记须 == 全量");
        assert_eq!(si, s1, "#{i}: tw=1 增量段须 == 全量");
    }
    assert!(marked > 0, "语料未触发尾窗截断，标记路径未被覆盖");
    eprintln!(
        "tb1604 语料 {} 例：tw=3/7 仅诊断轨迹 checks 与 ∞ 不同 {}/{} 例；tw=1：{marked} 例带标记，其中 {differed} 例段/尾部与 ∞ 不同",
        corpus().len(),
        trace_only[0],
        trace_only[1]
    );
}

/// A-12：`open` 研究覆写在相切语料上改变段划分（常量回退变异体必红）；`closed` 覆写与签署记录
/// 逐位相同（等值对拍）；输出戳随覆写变化。
#[test]
fn tb1604_tangency_open_changes_output_closed_reproduces_signed() {
    let signed = ThetaConfig::default().parse;
    let mut closed = signed;
    closed.research_override_tangency(Tangency::Closed);
    let mut open = signed;
    open.research_override_tangency(Tangency::Open);
    let mut flipped = 0usize;
    for (i, ss) in corpus().iter().enumerate() {
        let (s0, p0, f0) = segment::divide_segments_with_facts(ss, &signed);
        let (sc, pc, fc) = segment::divide_segments_with_facts(ss, &closed);
        assert_eq!(
            fc.policy.as_ref().map(|p| p.construction_rule.as_str()),
            Some("segment-second/research-override")
        );
        assert_eq!((&sc, pc, &semantic(fc)), (&s0, p0, &semantic(f0)), "#{i}");
        let (so, po, fo) = segment::divide_segments_with_facts(ss, &open);
        assert_eq!(fo.policy.as_ref().map(|p| p.tangency), Some("open"));
        if (&so, po) != (&s0, p0) {
            flipped += 1;
        }
        let (si, _) = incremental(ss, &open);
        assert_eq!(si, so, "#{i}: open 增量段须 == 全量");
    }
    assert!(
        flipped > 0,
        "open 与 closed 在相切语料上无差异——相切谓词疑似回退为常量"
    );
}

/// A-12 定点 fixture（每档一条）：三笔 [5,10]、[0,10]、[0,5] 只在 5 相切。`closed` 判起点 0 有重合，
/// `open` 判无重合。
#[test]
fn tb1604_tangency_fixture_seed_per_value() {
    let ss = chain(&[5, 10, 0, 5]);
    for (t, want) in [(Tangency::Closed, true), (Tangency::Open, false)] {
        let c = segment::construction(&ss, 0, 3, t);
        assert_eq!(c.overlap_relation, Some("TOUCH"));
        assert_eq!(c.vector.unwrap()[1], want, "{t:?}");
    }
}

/// #1604 M3：classifier 输出带段层两份记录戳（id/版本），与段事实所读一致。
#[test]
fn tb1604_classifier_output_stamps_segment_records() {
    let cfg = ThetaConfig::default();
    let bars = super::tb1581_tests::tb1581_bars(0x1604_0001, 600);
    let layer = super::parse_layer(&bars, &cfg);
    let out = super::super::classifier::classify(&layer, &cfg, &[]);
    assert_eq!(out.policy.scan_budget.label(), "second-seq-scan-budget/2");
    assert_eq!(
        out.policy.construction_rule.label(),
        segment::CONSTRUCTION_RULE
    );
    // 研究覆写改了值，戳随之标 research-override，不冒称签署版本。
    let mut over = cfg.clone();
    over.parse.research_override_tail_window(3);
    over.parse.research_override_tangency(Tangency::Closed);
    let out = super::super::classifier::classify(&layer, &over, &[]);
    assert_eq!(
        out.policy.scan_budget.label(),
        "second-seq-scan-budget/research-override"
    );
    assert_eq!(
        out.policy.construction_rule.label(),
        "segment-second/research-override"
    );
}

/// #1604 M3 负控 / 正控：L0 带尾窗截断标记 ⟹ 全量与增量 classifier 输出都显式带
/// `INCOMPLETE_RESOURCE_BOUND` 与起点、预算记录版本（不静默）；无标记 ⟹ `COMPLETE`。拒读路径
/// （段端门标记同在）也不吞掉截断标记。读取改回「恒 Complete」的变异体让负控变红。
#[test]
fn tb1604_classifier_surfaces_tail_window_bound() {
    use super::super::classifier::{self, SegmentCoverage};
    let cfg = ThetaConfig::default();
    let bars = super::tb1581_tests::tb1581_bars(0x1604_0002, 1_200);
    let mut incr = super::ParseLayerIncr::new(&cfg);
    let mut cache = classifier::TowerCache::new();
    let inject_at = 1_000;
    let mut checked = false;
    for (i, bar) in bars.iter().enumerate() {
        let layer = incr.append(*bar);
        assert_eq!(
            layer.segments_resource_bound_from, None,
            "签署尾窗 7 无截断"
        );
        let out = classifier::classify_incremental(&layer, &cfg, &mut cache, &[]);
        assert_eq!(
            out.segment_coverage,
            SegmentCoverage::Complete,
            "正控 bar {i}"
        );
        assert!(!out.is_segment_resource_bound());
        if i != inject_at {
            continue;
        }
        assert!(!layer.segments.is_empty(), "夹具须有段");
        let from = layer.segments[layer.segments.len() / 2].start_index;
        let mut marked = layer.clone();
        marked.segments_resource_bound_from = Some(from);
        let expect = SegmentCoverage::IncompleteResourceBound {
            from,
            budget: cfg.parse.scan_budget,
        };
        let full = classifier::classify(&marked, &cfg, &[]);
        assert_eq!(full.segment_coverage, expect);
        assert_eq!(full.segment_coverage.code(), "INCOMPLETE_RESOURCE_BOUND");
        assert!(full.is_segment_resource_bound());
        // 只标记不拒读：结构与无标记时同一输入逐位相同。
        let plain = classifier::classify(&layer, &cfg, &[]);
        assert_eq!(full.classification, plain.classification);
        let mut warm = cache.clone();
        let inc = classifier::classify_incremental(&marked, &cfg, &mut warm, &[]);
        assert_eq!(inc.segment_coverage, expect, "增量入口同一标记");
        let mut both = marked.clone();
        both.segments_parse_invalid_from = Some(0);
        let refused = classifier::classify(&both, &cfg, &[]);
        assert_eq!(refused.segment_coverage, expect, "拒读路径保留截断标记");
        checked = true;
    }
    assert!(checked);
}
