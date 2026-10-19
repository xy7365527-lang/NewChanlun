//! #1404：期望来自执行前冻结的 raw 账簿，不从段输出生成。
use super::super::types::Direction;
use super::*;

fn chain(prices: &[i64]) -> Vec<Stroke> {
    prices
        .windows(2)
        .enumerate()
        .map(|(i, p)| Stroke {
            direction: if p[1] > p[0] {
                super::super::types::Direction::Up
            } else {
                super::super::types::Direction::Down
            },
            start_index: 4 * i + 1,
            end_index: 4 * i + 5,
            start_price: p[0],
            end_price: p[1],
        })
        .collect()
}

#[test]
fn tb02c_directional_inclusion_and_no_fractal_witnesses() {
    for sign in [1, -1] {
        let ss = chain(&[0, 10, 4, 9, 5].map(|p| p * sign));
        let (_, _, facts) = segment::divide_segments_with_facts(&ss, &ThetaConfig::default().parse);
        let c = &facts.candidates[0];
        assert_eq!(c.first.standard.len(), 1);
        assert_eq!(c.first.standard[0].stroke_indices, vec![1, 3]);
        assert_eq!(c.first.steps[1].action, "merge");
        assert_eq!(c.first.steps[1].scope, "SAME_SEQUENCE");
        assert_eq!(
            c.first.steps[1].merge_direction,
            Some(if sign == 1 { "UP" } else { "DOWN" })
        );
        assert_eq!(
            (c.first.standard[0].low, c.first.standard[0].high),
            if sign == 1 { (5, 10) } else { (-10, -5) }
        );
        let ss = chain(&[0, 10, 4, 12, 6, 14, 8].map(|p| p * sign));
        let (_, _, facts) = segment::divide_segments_with_facts(&ss, &ThetaConfig::default().parse);
        // #1493：末元素 [8,14] 是无缺口峰候选、尚无第三元素 ⟹ 候选态 OPEN_AT_PREFIX_END（§7.1 第 1 条）；
        // 没有第三笔落在 b 范围内，结果分类不变。
        assert_eq!(facts.candidates[0].result, "NO_FIRST");
        assert_eq!(
            facts.candidates[0].turn_hypothesis_state,
            Some("OPEN_AT_PREFIX_END")
        );
        assert_eq!(facts.candidates[0].first.checks.last().unwrap().gap, None);
        assert!(
            !facts.candidates[0]
                .first
                .checks
                .last()
                .unwrap()
                .first_fractal
        );
    }
}

#[test]
fn tb02c_turn_window_stays_open() {
    // #1493 义务 7：c=[7,18] ⊂ b=[5,20] 且 b 是峰候选 ⟹ c 是 b 的窗成员，b 区间不动、不发布段。
    let (segments, _, facts) = segment::divide_segments_with_facts(
        &chain(&[0, 10, 4, 20, 5, 18, 7]),
        &ThetaConfig::default().parse,
    );
    assert!(segments.is_empty());
    let c = &facts.candidates[0];
    assert_eq!(c.result, "PENDING_CASE_ONE");
    assert_eq!(c.turn_hypothesis_state, Some("OPEN_AT_PREFIX_END"));
    assert_eq!(c.window_members, vec![5]);
    assert_eq!(c.first.standard.len(), 2);
    assert_eq!(c.first.standard[1].stroke_idx, 3);
    assert_eq!((c.first.standard[1].low, c.first.standard[1].high), (5, 20));
    assert_eq!(c.first.standard[1].window_members, vec![5]);
    let last = c.first.steps.last().unwrap();
    assert_eq!(last.action, "window_member");
    assert_eq!(last.scope, "TURN_HYPOTHESIS");
    assert_eq!(last.merge_direction, None);
    let open = c.first.open_turn.as_ref().unwrap();
    assert_eq!(open.turn_hypothesis_pair, [1, 3]);
    assert_eq!(open.turn_hypothesis_state, "OPEN_AT_PREFIX_END");
    assert!(c.waiting_reasons.contains(&"first_kind_third_stroke_within_first"));
    assert!(!c.first.checks.iter().any(|k| k.first_fractal));
    assert!(!c.first.steps.iter().any(|s| s.action == "merge"));
    assert!(facts.parse_invalid_from.is_none());
}

/// #1493：按 §7.1 第 5 条定义流首段起点后首段门恒过；以 TestOnly 种子起点构造门失败，检验见证与
/// fail-closed（`parse_invalid_from`）。生产口径下同一输入不失败。
fn tb1493_seed_start<T>(f: impl FnOnce() -> T) -> T {
    segment::FIRST_START_AT_SEED.with(|c| c.set(true));
    let r = f();
    segment::FIRST_START_AT_SEED.with(|c| c.set(false));
    r
}

#[test]
fn tb02c_geometry_failure_keeps_witness_and_does_not_emit() {
    let ss = chain(&[5, 10, 0, 5, -1, 2, -3, 3, -2, 1, -4]);
    let (_, _, prod) = segment::divide_segments_with_facts(&ss, &ThetaConfig::default().parse);
    assert!(prod.parse_invalid_from.is_none());
    assert!(prod.candidates.iter().all(|c| c.result != "FAILED_GEOMETRY"));
    let (segments, _, facts) = tb1493_seed_start(|| {
        tb1493_full_eq_incremental(&ss);
        segment::divide_segments_with_facts(&ss, &ThetaConfig::default().parse)
    });
    assert!(segments.is_empty());
    let invalid = facts.parse_invalid_from.as_ref().expect("门失败须 fail-closed 上报");
    assert_eq!(invalid.seg_start, 0);
    assert_eq!(invalid.construction_vector, Some([true, true, false, true]));
    assert!(invalid.failed_conditions.contains(&"top_gt_bottom"));
    assert_eq!(invalid.elements[1].stroke_idx, invalid.end_stroke + 1);
    assert_eq!(facts.candidates[0].parse_invalid_from, Some(0));
    let c = &facts.candidates[0];
    assert_eq!(c.seed.overlap_relation, Some("TOUCH"));
    assert_eq!(c.result, "FAILED_GEOMETRY");
    assert_eq!(
        c.termination_attempts.last().unwrap().construction.vector,
        Some([true, true, false, true])
    );
    assert!(c.waiting_reasons.contains(&"top_gt_bottom"));
}

#[test]
fn tb02c_gap_does_not_hide_failed_geometry() {
    // FIRST 后三元素 [-16,-10],[-8,-6],[last,-7]：有 gap，
    // 分型端 -6 低于段首 0；不论 c 是否封 gap 都已知几何失败。
    // #1493：生产口径下流首段起点按定义取 s4（-20），首段 s4–s6 合法；门失败只在 TestOnly 种子起点下构造。
    for last in [-12, -9] {
        let ss = chain(&[0, 20, 8, 10, -20, -10, -16, -6, -8, -7, last]);
        let (segs, _, prod) =
            segment::divide_segments_with_facts(&ss, &ThetaConfig::default().parse);
        assert!(prod.parse_invalid_from.is_none());
        assert_eq!(prod.candidates[0].start_stroke, 4);
        assert_eq!(
            prod.candidates[0].result,
            if last == -12 { "CASE_ONE" } else { "PENDING_CASE_TWO" }
        );
        assert_eq!(segs.len(), usize::from(last == -12));
        let (_, _, facts) = tb1493_seed_start(|| {
            segment::divide_segments_with_facts(&ss, &ThetaConfig::default().parse)
        });
        let c = &facts.candidates[0];
        assert_eq!(c.first.checks.last().unwrap().gap, Some(true));
        assert_eq!(c.result, "FAILED_GEOMETRY");
        assert_eq!(c.end_stroke, None);
        assert_eq!(
            c.termination_attempts.last().unwrap().construction.vector,
            Some([true, true, false, true])
        );
        assert!(c.waiting_reasons.contains(&"top_gt_bottom"));
        assert!(!c.waiting_reasons.contains(&"second_kind_out_of_scope"));
        assert!(!c.waiting_reasons.contains(&"no_admissible_first_fractal"));
    }
}

#[test]
fn tb02c_length_axes_do_not_fill_inapplicable_conditions() {
    let ss = chain(&[0, 10, 4, 12, 6]);
    for (n, name) in [
        (0, "n=0"),
        (1, "n=1"),
        (2, "n=2"),
        (3, "odd_ge3"),
        (4, "even_ge3"),
    ] {
        let c = segment::construction(&ss, 0, n);
        assert_eq!(c.length, name);
        assert_eq!(c.vector.is_some(), n == 3);
    }
}

#[test]
fn tb02c_closed_seed_touch_and_gap_remain_distinct() {
    let touch = segment::construction(&chain(&[5, 10, 0, 5]), 0, 3);
    assert_eq!(touch.overlap, Some([5, 5]));
    assert_eq!(touch.overlap_relation, Some("TOUCH"));
    // 三笔相切成立，但段首末同价使顶>底失败；不把闭重叠偷换成全部条件。
    assert_eq!(touch.vector.unwrap()[1], true);
    let separated = segment::construction(&chain(&[0, 10, 20, 30]), 0, 3);
    assert_eq!(separated.overlap_relation, Some("DISJOINT"));
    assert_eq!(separated.vector.unwrap()[1], false);
    for prices in [[0, 10, 5, 20, 12, 18, 11], [0, 10, 5, 20, 12, 18, 8]] {
        let (_, _, facts) =
            segment::divide_segments_with_facts(&chain(&prices), &ThetaConfig::default().parse);
        let c = &facts.candidates[0];
        // #1414 已承接 strict：保留 raw gap，c 封闭转 CASE_ONE。
        let closed = prices[6] <= 10;
        assert_eq!(
            c.result,
            if closed {
                "CASE_ONE"
            } else {
                "PENDING_CASE_TWO"
            }
        );
        assert_eq!(c.end_stroke, closed.then_some(2));
        assert_eq!(c.first.checks.last().unwrap().gap, Some(true));
        assert_eq!(c.strict.as_ref().unwrap().gap_closed_by_c, Some(closed));
    }
}

#[test]
fn tb02c_waiting_witness_stops_at_its_scan_frontier() {
    // #1414 有原 gap、c 封闭走第一种；后续输入不能混进 cursor=5 的见证。
    let (_, _, facts) = segment::divide_segments_with_facts(
        &chain(&[0, 10, 5, 20, 12, 18, 8, 16, 9]),
        &ThetaConfig::default().parse,
    );
    let c = &facts.candidates[0];
    assert_eq!(c.result, "CASE_ONE");
    assert_eq!(c.observed_through, 5);
    assert_eq!(c.construction.stroke_indices, vec![0, 1, 2]);
    assert!(c.termination_attempts.iter().all(|a| a
        .construction
        .stroke_indices
        .iter()
        .all(|i| *i <= c.observed_through)));
}

#[test]
fn tb02c_raw_seed_first_and_incremental_agree() {
    let ledger: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../s_session/tests/fixtures/tb02c/raw-ledger.json"
    ))
    .unwrap();
    for name in ["first_up", "first_down"] {
        let config = ThetaConfig::default();
        let mut incremental = ParseLayerIncr::new(&config);
        let mut bars = vec![];
        for (i, row) in ledger["cases"][name].as_array().unwrap().iter().enumerate() {
            let bar = Bar {
                source_index: i,
                timestamp: i as i64,
                open: row["open"].as_i64().unwrap(),
                high: row["high"].as_i64().unwrap(),
                low: row["low"].as_i64().unwrap(),
                close: row["close"].as_i64().unwrap(),
                volume: 1.0,
                untradable: false,
            };
            bars.push(bar);
            let inc = incremental.append(bar);
            let (full, facts) = parse_layer_with_inclusion_facts(&bars, &config);
            assert_eq!(inc.strokes, full.strokes, "{name} raw{i}");
            assert_eq!(inc.segments, full.segments, "{name} raw{i}");
            let facts = facts.segment_facts.unwrap();
            // R3 的两方向28前缀不进入本轮修正的几何失败提示分支。
            assert!(facts
                .candidates
                .iter()
                .all(|c| c.result != "FAILED_GEOMETRY"));
            if i == 15 || i == 27 {
                let c = &facts.candidates[0];
                assert_eq!(c.seed.vector, Some([true; 4]));
                assert_eq!(
                    c.seed.overlap,
                    Some(if name == "first_up" {
                        [192, 384]
                    } else {
                        [-384, -192]
                    })
                );
                assert_eq!(
                    c.first
                        .raw_elements
                        .iter()
                        .map(|e| e.stroke_idx)
                        .collect::<Vec<_>>(),
                    if i == 15 { vec![1] } else { vec![1, 3, 5] }
                );
                assert_eq!(c.result, if i == 15 { "NO_FIRST" } else { "CASE_ONE" });
                assert_eq!(c.end_stroke, if i == 15 { None } else { Some(2) });
                if i == 27 {
                    assert_eq!(full.segments[0].end_index, 13);
                    assert_eq!(c.trigger_stroke, Some(5));
                    assert_eq!(c.first.checks.last().unwrap().gap, Some(false));
                    assert_eq!(facts.candidates[1].start_stroke, 3);
                }
            }
        }
    }
}

// ════════════════════════════════════════════════════════════
// #1493（TB-02-X，#1490 裁定 xianduan.md §7.1）：第一种情况候选三态、转折点窗、先破判据、
// 流首段起点定义、段端门 fail-closed。夹具来自 #1487 诊断（tb1487-diag/runs/fixtures），
// 期望值为 #1493 票面义务 8 (a)(b) 的应得结果。
// ════════════════════════════════════════════════════════════

const TB1493_MINI: &str = include_str!("../../../tests/fixtures/tb1490/mini_points.json");
const TB1493_E1_REAL: &str = include_str!("../../../tests/fixtures/tb1490/e1_real_59845_40.json");
const TB1493_E2_REAL: &str = include_str!("../../../tests/fixtures/tb1490/e2_real_3175_40.json");

fn tb1493_mini(key: &str) -> Vec<Stroke> {
    let value: serde_json::Value = serde_json::from_str(TB1493_MINI).unwrap();
    let points: Vec<i64> = value[key]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_i64().unwrap())
        .collect();
    chain(&points)
}

fn tb1493_real(src: &str) -> Vec<Stroke> {
    let value: serde_json::Value = serde_json::from_str(src).unwrap();
    value["strokes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            let row = row.as_array().unwrap();
            Stroke {
                direction: if row[0].as_str().unwrap() == "Up" {
                    Direction::Up
                } else {
                    Direction::Down
                },
                start_index: row[1].as_u64().unwrap() as usize,
                end_index: row[2].as_u64().unwrap() as usize,
                start_price: row[3].as_i64().unwrap(),
                end_price: row[4].as_i64().unwrap(),
            }
        })
        .collect()
}

/// #804 / 义务 8(g)：逐前缀对拍全量与逐笔增量（segments、pending_start、fail-closed 起点）。
fn tb1493_full_eq_incremental(strokes: &[Stroke]) {
    let cfg = &ThetaConfig::default().parse;
    let mut inc = segment::IncrSegments::empty();
    for k in 1..=strokes.len() {
        inc = inc.append(&strokes[..k], cfg);
        let (full, pending, facts) = segment::divide_segments_with_facts(&strokes[..k], cfg);
        let (segs, tail) = inc.to_result_vec();
        assert_eq!(segs, full, "前缀 {k}：增量 segments != 全量");
        assert_eq!(tail, pending, "前缀 {k}：增量 pending_start != 全量");
        assert_eq!(
            inc.parse_invalid_from(),
            facts.parse_invalid_from.as_ref().map(|p| p.seg_start),
            "前缀 {k}：增量 parse_invalid_from != 全量"
        );
        assert_eq!(
            segment::divide_segments_with_tail(&strokes[..k], cfg),
            (full, pending),
            "前缀 {k}：观察与非观察路径须同一判定"
        );
    }
}

/// (段起笔, 段末笔, 方向, 起价, 终价)
fn tb1493_spans(strokes: &[Stroke], segs: &[Segment]) -> Vec<(usize, usize, Direction, i64, i64)> {
    segs.iter()
        .map(|s| {
            let a = strokes.iter().position(|x| x.start_index == s.start_index).unwrap();
            let b = strokes.iter().rposition(|x| x.end_index == s.end_index).unwrap();
            (a, b, s.direction, s.start_price, s.end_price)
        })
        .collect()
}

fn tb1493_check(
    strokes: &[Stroke],
    expected: &[(usize, usize, Direction, i64, i64)],
    pending: Option<usize>,
) -> segment::SegmentFacts {
    tb1493_full_eq_incremental(strokes);
    let (segs, tail, facts) =
        segment::divide_segments_with_facts(strokes, &ThetaConfig::default().parse);
    assert_eq!(tb1493_spans(strokes, &segs), expected, "段");
    assert_eq!(tail, pending, "未确认尾段起点");
    assert!(facts.parse_invalid_from.is_none(), "段端门不得失败");
    assert!(facts.candidates.iter().all(|c| c.result != "FAILED_GEOMETRY"));
    facts
}

#[test]
fn tb1493_e1_min_old_segment_continues() {
    let ss = tb1493_mini("e1_min");
    let facts = tb1493_check(&ss, &[], Some(0));
    assert_eq!(ss[0].direction, Direction::Up);
    // E1：c ⊂ b 不再过早确认；向上段延续。
    assert!(facts.candidates[0]
        .first
        .checks
        .iter()
        .all(|c| c.turn_hypothesis_state != "CONFIRMED"));
}

#[test]
fn tb1493_e1_release_old_segment_continues_to_80() {
    let ss = tb1493_mini("e1_release");
    tb1493_check(&ss, &[], Some(0));
    // 向上段延续到 80（第 13 笔终点），其后无向下段确认。
    assert_eq!(ss[12].end_price, 80);
    let (_, _, facts) = segment::divide_segments_with_facts(&ss, &ThetaConfig::default().parse);
    assert_eq!(facts.candidates.len(), 1);
}

#[test]
fn tb1493_e2_min_confirms_by_first_break() {
    let ss = tb1493_mini("e2_min");
    tb1493_check(
        &ss,
        &[(0, 2, Direction::Up, 10, 50), (3, 5, Direction::Down, 50, 25)],
        Some(6),
    );
    assert_eq!(ss[6].direction, Direction::Up);
}

#[test]
fn tb1493_e1_real_59845_old_segment_continues() {
    let ss = tb1493_real(TB1493_E1_REAL);
    tb1493_check(&ss, &[], Some(0));
    assert_eq!(ss[0].direction, Direction::Up);
}

#[test]
fn tb1493_e2_real_3175_six_segments() {
    let ss = tb1493_real(TB1493_E2_REAL);
    tb1493_full_eq_incremental(&ss);
    let (segs, _, facts) = segment::divide_segments_with_facts(&ss, &ThetaConfig::default().parse);
    assert!(facts.parse_invalid_from.is_none());
    let spans = tb1493_spans(&ss, &segs);
    assert_eq!(spans.len(), 6, "{spans:?}");
    assert_eq!(spans[0], (0, 2, Direction::Up, 376_644_000_000, 380_753_000_000));
    assert_eq!(spans[1], (3, 5, Direction::Down, 380_753_000_000, 377_000_000_000));
}

#[test]
fn tb1493_window_member_does_not_forge_gap() {
    // 义务 8(c)：向下段 b=[8190,8249]、成员 [8215,8221]、下一候选 [8163,8201]。按 G-2 把成员折进 b
    // 会把 b.low 抬到 8215，伪造 8215 > 8201 的缺口；窗成员保持 b 区间不动 ⟹ 下一候选无缺口。
    let ss = chain(&[8300, 8200, 8260, 8190, 8249, 8215, 8221, 8163, 8201, 8170, 8230]);
    tb1493_full_eq_incremental(&ss);
    let (segs, _, facts) = segment::divide_segments_with_facts(&ss, &ThetaConfig::default().parse);
    let first = &facts.candidates[0].first;
    let b = first.standard.iter().find(|e| e.stroke_idx == 3).unwrap();
    assert_eq!((b.low, b.high), (8190, 8249));
    assert_eq!(b.window_members, vec![5]);
    let kill = first.checks.iter().find(|c| c.elements[1].stroke_idx == 3).unwrap();
    assert_eq!(kill.turn_hypothesis_state, "KILLED_BY_NEW_EXTREME");
    let next = first.checks.iter().find(|c| c.elements[1].stroke_idx == 7).unwrap();
    assert_eq!(next.gap, Some(false));
    assert_eq!(next.turn_hypothesis_state, "CONFIRMED");
    assert_eq!(
        tb1493_spans(&ss, &segs),
        vec![(0, 6, Direction::Down, 8300, 8163)]
    );
}

#[test]
fn tb1493_tie_without_gap_confirmed_by_first_break() {
    // 义务 8(d) 之一：b=[5,20]、c=[3,20]：P 侧相等不是越过，c 严格破 b 终点 ⟹ 先破判据确认。
    let ss = chain(&[0, 10, 4, 20, 5, 20, 3]);
    tb1493_full_eq_incremental(&ss);
    let (segs, _, facts) = segment::divide_segments_with_facts(&ss, &ThetaConfig::default().parse);
    let c = &facts.candidates[0];
    let tie = c.first.steps.iter().find(|s| s.action == "tie_pair").unwrap();
    assert_eq!(tie.tie_with, Some(3));
    assert_eq!(tie.scope, "TURN_HYPOTHESIS");
    assert_eq!(c.result, "CASE_ONE");
    assert_eq!(c.turn_hypothesis_state, Some("CONFIRMED"));
    assert_eq!(tb1493_spans(&ss, &segs), vec![(0, 2, Direction::Up, 0, 20)]);
}

#[test]
fn tb1493_tie_with_gap_is_window_member_then_case_two() {
    // 义务 8(d) 之二：a=[4,10]、b=[12,20] 有缺口；c=[8,20] 并列 ⟹ b 的窗成员（第二种情况走第二序列）。
    let ss = chain(&[0, 10, 4, 20, 12, 20, 8]);
    tb1493_full_eq_incremental(&ss);
    let (segs, _, facts) = segment::divide_segments_with_facts(&ss, &ThetaConfig::default().parse);
    assert!(segs.is_empty());
    let c = &facts.candidates[0];
    assert_eq!(c.first.standard.len(), 2);
    assert_eq!(c.first.standard[1].window_members, vec![5]);
    assert_eq!(c.first.steps.last().unwrap().action, "window_member");
    assert!(c.first.open_turn.is_none());
    // 后续 d=[11,15] 自下方出窗：067 严格分型成立、缺口未被 d 封闭 ⟹ 第二种情况。
    let ss = chain(&[0, 10, 4, 20, 12, 20, 8, 15, 11]);
    tb1493_full_eq_incremental(&ss);
    let (_, _, facts) = segment::divide_segments_with_facts(&ss, &ThetaConfig::default().parse);
    let c = &facts.candidates[0];
    let check = c.first.checks.last().unwrap();
    assert!(check.first_fractal);
    assert_eq!(check.gap, Some(true));
    assert_eq!(check.turn_hypothesis_state, "NOT_CASE_ONE");
    assert_eq!(check.elements[1].window_members, vec![5]);
    assert_eq!(check.elements[2].stroke_idx, 7);
    assert_eq!(c.result, "PENDING_CASE_TWO");
}

#[test]
fn tb1493_stream_first_segment_starts_at_extreme() {
    // 义务 8(f)：种子 s0 起价 10；确认前成员笔中向上笔起点最低者是 s2（5）⟹ 流首段 = s2–s4。
    let ss = chain(&[10, 30, 5, 25, 15, 40, 20, 35, 10]);
    tb1493_full_eq_incremental(&ss);
    let (segs, pending, facts) =
        segment::divide_segments_with_facts(&ss, &ThetaConfig::default().parse);
    assert!(facts.parse_invalid_from.is_none());
    assert_eq!(
        tb1493_spans(&ss, &segs),
        vec![(2, 4, Direction::Up, 5, 40)]
    );
    assert_eq!(facts.candidates[0].start_stroke, 2);
    // 非流首段不适用：第二段从分型中心 b=s5 起。
    assert_eq!(pending, Some(5));
    // 流首段确认前，pending 起点同样按定义随前缀变化（不发布）。
    let (none, p, _) = segment::divide_segments_with_facts(&ss[..5], &ThetaConfig::default().parse);
    assert!(none.is_empty());
    assert_eq!(p, Some(2));
}

#[test]
fn tb1493_gate_failure_is_fail_closed() {
    // 门失败不回退、不改起点重判：跳过该触发、段继续，并在 facts 与增量快照上置同一 parse_invalid_from。
    let ss = chain(&[0, 20, 8, 10, -20, -10, -16, -6, -8, -7, -12, -2, -9, 5, -5]);
    tb1493_seed_start(|| {
        tb1493_full_eq_incremental(&ss);
        let (segs, _, facts) =
            segment::divide_segments_with_facts(&ss, &ThetaConfig::default().parse);
        let p = facts.parse_invalid_from.as_ref().expect("须 fail-closed 上报");
        assert_eq!((p.seg_start, p.end_stroke), (0, 6));
        assert!(p.construction_vector.is_some_and(|v| v != [true; 4]));
        // 不改起点重判：此后不得出现以其他起点重开的流首段。
        assert!(segs.iter().all(|s| s.start_index == ss[0].start_index));
    });
}

fn tb1493_splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// 交替折线；`distinct` ⟹ 端点价两两互异（低位编号），否则小整数域高频并列。
fn tb1493_polyline(seed: u64, n: usize, distinct: bool) -> Vec<i64> {
    let mut st = seed;
    let mut p = 10_000i64;
    let mut out = vec![p];
    let mut up = tb1493_splitmix(&mut st) % 2 == 0;
    for i in 0..n {
        let step = if distinct {
            1 + (tb1493_splitmix(&mut st) % 400) as i64
        } else {
            1 + (tb1493_splitmix(&mut st) % 6) as i64
        };
        p += if up { step } else { -step };
        out.push(p);
        up = !up;
        let _ = i;
    }
    if distinct {
        out.iter().enumerate().map(|(i, v)| v * 1000 + i as i64).collect()
    } else {
        out
    }
}

#[test]
fn tb1493_first_break_equals_067_on_distinct_prices() {
    // 义务 8(e)：端点价互异 ⟹ 先破判据与 067 严格谓词的终结序列逐位相同；含并列 ⟹ 差异全部落在
    // 「c 的 P 侧极值 = P」的候选上。
    let cfg = &ThetaConfig::default().parse;
    let run = |ss: &[Stroke], oracle: bool| {
        feature_seq::CASE_ONE_ORACLE_067.with(|o| o.set(oracle));
        let r = segment::divide_segments_with_facts(ss, cfg);
        feature_seq::CASE_ONE_ORACLE_067.with(|o| o.set(false));
        r
    };
    let mut tie_diffs = 0usize;
    for g in 0..400u64 {
        for distinct in [true, false] {
            let ss = chain(&tb1493_polyline(0x1493_0000 + g, 300, distinct));
            let (a, _, fa) = run(&ss, false);
            let (b, _, _) = run(&ss, true);
            if distinct {
                assert_eq!(a, b, "组 {g}：端点价互异时两判据须逐位相同");
                continue;
            }
            let Some(j) = (0..a.len().max(b.len())).find(|&j| a.get(j) != b.get(j)) else {
                continue;
            };
            tie_diffs += 1;
            let w = fa.candidates[j]
                .strict
                .as_ref()
                .unwrap_or_else(|| panic!("组 {g}：首个差异段 {j} 须是先破判据侧的第一种终结"));
            let up = ss[fa.candidates[j].start_stroke].direction == Direction::Up;
            let (bp, cp) = if up {
                (w.elements[1].high, w.elements[2].high)
            } else {
                (w.elements[1].low, w.elements[2].low)
            };
            assert_eq!(bp, cp, "组 {g}：差异须落在 c 的 P 侧极值 = P 的候选上");
        }
    }
    eprintln!("tb1493 性质锁：并列输入差异组数 {tie_diffs}/400");
}

#[test]
fn tb1493_random_full_eq_incremental_and_no_gate_failure_after_first() {
    // 义务 8(g)：随机折线上全量 ≡ 逐笔增量。附读数：首段之外门失败次数（#1490 原型为 0）。
    let mut later_failures = 0usize;
    for g in 0..60u64 {
        for distinct in [true, false] {
            let ss = chain(&tb1493_polyline(0x1493_8000 + g, 120, distinct));
            tb1493_full_eq_incremental(&ss);
            let (segs, _, facts) =
                segment::divide_segments_with_facts(&ss, &ThetaConfig::default().parse);
            if facts.parse_invalid_from.is_some() && !segs.is_empty() {
                later_failures += 1;
            }
        }
    }
    eprintln!("tb1493 随机折线：首段之后门失败组数 {later_failures}/120");
}
