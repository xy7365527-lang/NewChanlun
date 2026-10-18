//! #1420 TB-02-W：第二种候选三态语义终点（#1344 裁定，xianduan §5.1）的 parser 核心锁。
//!
//! 期待值逐笔手算自裁定与原文（`078-第78课.md:52` 失效；`067-第67课.md:38` 分型确认；
//! `065-第65课.md:28` 只对已走出的笔下结论），不调被测函数生成。夹具约定：s0..s2 为向上段 A
//! 的种子，s3 是 FIRST 顶分型中心 b（P = 40），a/b 缺口 [20,25] 未被 c 封闭 ⟹ 第二种候选；
//! `sign = -1` 是逐价取负的镜像（向下段，P = -40）。
use super::super::types::Direction;
use super::feature_seq::{
    second_sequence, ScanCoverage, SecondTerminal, SECOND_CANDIDATE_RULE_REVISION,
    SECOND_SEQ_SCAN_BUDGET_POLICY,
};
use super::second_fold::{
    FoldResumeError, SecondCandidateId, SecondFoldCursor, SecondFoldStore,
    SECOND_FOLD_CURSOR_SCHEMA,
};
use super::*;

fn chain(prices: &[i64], sign: i64) -> Vec<Stroke> {
    prices
        .windows(2)
        .enumerate()
        .map(|(i, p)| Stroke {
            direction: if (p[1] - p[0]) * sign > 0 {
                Direction::Up
            } else {
                Direction::Down
            },
            start_index: i * 4 + 1,
            end_index: i * 4 + 5,
            start_price: p[0] * sign,
            end_price: p[1] * sign,
        })
        .collect()
}

fn dir(sign: i64) -> Direction {
    if sign > 0 {
        Direction::Up
    } else {
        Direction::Down
    }
}

/// s4 (35,25)、s6 (30,22)、s8 (33,23)：s8 补成底分型 ⟹ CONFIRMED。
const CONFIRMED: &[i64] = &[0, 20, 10, 40, 25, 35, 22, 30, 23, 33];
/// s6 22→45 先严格越过 P=40 ⟹ KILLED；其后 s8 (38,30)、s10 (34,26) 在旧「只认分型」下会成顶分型复活。
const KILLED: &[i64] = &[0, 20, 10, 40, 25, 35, 22, 45, 30, 38, 26, 34];
/// 已知前缀内既无分型也无越界 ⟹ OPEN_AT_PREFIX_END。
const OPEN: &[i64] = &[0, 20, 10, 40, 25, 35, 22, 30];
/// s8 23→45 同笔：起点 23 先补成底分型，随后高点 45 越界 ⟹ 分型在先，CONFIRMED。
const SAME_STROKE: &[i64] = &[0, 20, 10, 40, 25, 35, 22, 30, 23, 45];
/// s6 22→40 恰等于 P：越界取严格不等 ⟹ 不失效。
const TOUCH: &[i64] = &[0, 20, 10, 40, 25, 35, 22, 40];
/// s6 24→41 被 (35,25) 包含并入（不是独立第三元素），其新高 41 仍越过 P ⟹ KILLED。
const MERGED_KILL: &[i64] = &[0, 20, 10, 40, 25, 35, 24, 41];
/// KILLED 之后：A 由 s7 无缺口顶分型终结于 s6，B（s7..s11）由 s12 底分型终结，C 自 s12 起。
const KILLED_THEN_B: &[i64] = &[
    0, 20, 10, 40, 25, 35, 22, 45, 30, 38, 26, 34, 15, 28, 20, 50, 30, 60,
];

fn second(prices: &[i64], sign: i64, window: u32) -> feature_seq::SecondSequenceFacts {
    second_sequence(&chain(prices, sign), dir(sign), 3, 40 * sign, window)
}

#[test]
fn tb02w_three_terminals_confirmed_killed_open() {
    for sign in [1, -1] {
        let c = second(CONFIRMED, sign, 0);
        assert_eq!(
            c.semantic_state,
            Some(SecondTerminal::Confirmed),
            "sign={sign}"
        );
        assert_eq!(c.coverage, ScanCoverage::Complete);
        assert_eq!(c.killed_by, None);
        assert_eq!(c.matched.as_ref().unwrap().through_stroke, 8);
        assert_eq!(c.breach_watermark, Some(35 * sign));

        let k = second(KILLED, sign, 0);
        assert_eq!(
            k.semantic_state,
            Some(SecondTerminal::KilledByNewExtreme),
            "sign={sign}"
        );
        assert_eq!(k.coverage, ScanCoverage::Complete);
        assert!(k.matched.is_none() && !k.resource_truncated);
        let kill = k.killed_by.as_ref().unwrap();
        assert_eq!(
            (kill.stroke_idx, kill.price, kill.candidate_extreme),
            (6, 45 * sign, 40 * sign)
        );
        assert_eq!(kill.comparison, if sign > 0 { ">" } else { "<" });
        assert_eq!(
            k.through_stroke, 6,
            "失效即停，不再扫描其后可能「复活」的分型"
        );
        assert_eq!(k.breach_watermark, Some(45 * sign));

        let o = second(OPEN, sign, 0);
        assert_eq!(o.semantic_state, Some(SecondTerminal::OpenAtPrefixEnd));
        assert_eq!(o.coverage, ScanCoverage::Complete);
        assert!(o.matched.is_none() && o.killed_by.is_none() && !o.resource_truncated);
        assert_eq!(o.through_stroke, 6);
        assert_eq!(o.breach_watermark, Some(35 * sign));
    }
}

#[test]
fn tb02w_killed_candidate_does_not_end_segment() {
    let cfg = ThetaConfig::default();
    for sign in [1, -1] {
        let ss = chain(KILLED, sign);
        let (segs, pending, facts) = segment::divide_segments_with_facts(&ss, &cfg.parse);
        assert_eq!(segs.len(), 1, "sign={sign}");
        assert_eq!(
            segs[0].end_index, ss[6].end_index,
            "078:52：A 延续到越界笔 s6，A、B、C 只算一段"
        );
        assert_eq!(pending, Some(7));
        let a = &facts.candidates[0];
        assert_eq!((a.result, a.end_stroke), ("CASE_ONE", Some(6)));

        // 失效候选仍是当前活动候选时：结果须明示失效，不得写成「待第二序列分型」。
        let short = chain(&KILLED[..8], sign);
        let (segs, _, facts) = segment::divide_segments_with_facts(&short, &cfg.parse);
        assert!(segs.is_empty());
        let a = &facts.candidates[0];
        assert_eq!(a.result, "KILLED_CASE_TWO", "sign={sign}");
        assert_eq!(
            a.waiting_reasons,
            vec!["second_candidate_killed_by_new_extreme"]
        );
        assert_eq!(a.fact_vector.segment_broken, Some(false));
        assert!(a.fact_vector.scan_complete);
    }
}

#[test]
fn tb02w_same_stroke_fractal_precedes_strict_breach() {
    for sign in [1, -1] {
        let s = second(SAME_STROKE, sign, 0);
        assert_eq!(
            s.semantic_state,
            Some(SecondTerminal::Confirmed),
            "sign={sign}"
        );
        assert!(s.killed_by.is_none());
        let m = s.matched.as_ref().unwrap();
        assert_eq!(m.through_stroke, 8);
        assert_eq!(m.bottom_fractal, sign > 0);

        let t = second(TOUCH, sign, 0);
        assert_eq!(
            t.semantic_state,
            Some(SecondTerminal::OpenAtPrefixEnd),
            "sign={sign}: 恰等于 P 不算越界"
        );
        assert!(t.killed_by.is_none());
        assert_eq!(t.breach_watermark, Some(40 * sign));
    }
}

#[test]
fn tb02w_merged_stroke_new_extreme_still_kills() {
    for sign in [1, -1] {
        let m = second(MERGED_KILL, sign, 0);
        assert_eq!(
            m.semantic_state,
            Some(SecondTerminal::KilledByNewExtreme),
            "sign={sign}"
        );
        assert_eq!(
            m.killed_by.as_ref().map(|k| (k.stroke_idx, k.price)),
            Some((6, 41 * sign))
        );
        assert_eq!(
            m.standard.len(),
            1,
            "s6 被包含并入前一元素，不是独立第三元素"
        );
        assert_eq!(m.steps[1].action, "merge");
    }
}

#[test]
fn tb02w_killed_candidate_never_revives_incremental_and_from_full() {
    let cfg = ThetaConfig::default();
    for sign in [1, -1] {
        let ss = chain(KILLED_THEN_B, sign);
        // 旧「只认分型」会在 s10 让 s3 复活，把 A 截在 s2。
        let revived_end = ss[2].end_index;
        let mut inc = segment::IncrSegments::default();
        for n in 0..=ss.len() {
            inc = inc.append(&ss[..n], &cfg.parse);
            let batch = segment::divide_segments_with_tail(&ss[..n], &cfg.parse);
            assert_eq!(inc.to_result_vec(), batch, "sign={sign} prefix={n}");
            assert!(
                batch.0.iter().all(|s| s.end_index != revived_end),
                "sign={sign} prefix={n}: 失效候选复活"
            );
            if n == 7 {
                // 失效笔 s6 仍是末笔：末笔可改写，失效尚可撤销 ⟹ 保持未封闭。
                assert_eq!(inc.earliest_unsealed_from(), Some(0), "sign={sign}");
            }
            if n == 8 {
                // s6 不再是末笔 ⟹ 失效终局，检查点安全边界前移。
                assert_eq!(inc.earliest_unsealed_from(), None, "sign={sign}");
            }
        }
        let full = segment::divide_segments_with_tail(&ss, &cfg.parse);
        assert!(
            full.0.len() >= 2,
            "夹具须含 A、B 两段，from_full 才保留前缀段"
        );
        let mut probed = 0;
        for p in 0..=ss.len() {
            let probe = segment::IncrSegments::default().append(&ss[..p], &cfg.parse);
            if probe.earliest_unsealed_from().is_some() {
                continue; // from_full 已声明的前提：恢复前缀无未封闭候选
            }
            let (segs, pending) = segment::divide_segments_with_tail(&ss[..p], &cfg.parse);
            let resumed =
                segment::IncrSegments::from_full(&segs, pending, &ss[..p]).append(&ss, &cfg.parse);
            assert_eq!(resumed.to_result_vec(), full, "sign={sign} from_full@{p}");
            probed += 1;
        }
        assert!(probed >= 8, "sign={sign}: from_full 覆盖不足（{probed}）");
    }
}

/// A-ST-048-01 夹具：s4 起 25 根同向笔高低同降（互不包含、无分型）且都低于 P=1000；
/// 第 26 根（s54，自 s4 起第 51 项）抬高补成底分型。
fn evidence_at_item_51() -> Vec<i64> {
    let mut prices = vec![0, 20, 10, 1000, 25];
    for j in 0..=24i64 {
        prices.push(900 - 10 * j);
        if j < 24 {
            prices.push(23 - j);
        }
    }
    prices.extend([1, 665]);
    prices
}

/// 跳过诱饵：同上第 51 项证据，但 s10 22→950（<P）先被并入第二序列、在 FIRST 留下无缺口顶分型
/// s11 (950,30)。若把资源受限候选当否定跳过，就会在 s11 判出另一个终结（与窗口 0 矛盾）。
fn skip_bait() -> Vec<i64> {
    let mut prices = vec![0, 20, 10, 1000, 25, 900, 23, 890, 22, 880, 21, 950, 30, 870];
    for j in 5..=24i64 {
        prices.push(20 - (j - 5));
        prices.push(860 - 10 * (j - 5));
    }
    prices.extend([2, 675]);
    prices
}

#[test]
fn tb02w_ast_048_01_resource_bound_candidate_is_not_skipped() {
    for sign in [1, -1] {
        let ss = chain(&skip_bait(), sign);
        assert_eq!(ss.len(), 55);
        let full = second_sequence(&ss, dir(sign), 3, 1000 * sign, 0);
        assert_eq!(full.matched.as_ref().map(|m| m.through_stroke), Some(54));
        let w50 = second_sequence(&ss, dir(sign), 3, 1000 * sign, 50);
        assert_eq!(w50.coverage, ScanCoverage::IncompleteResourceBound);
        let (s0, _) = segment::divide_segments_with_tail(&ss, &ThetaConfig::default().parse);
        assert_eq!(
            s0[0].end_index, ss[2].end_index,
            "窗口 0：s3 候选确认，A 止于 s2"
        );
        let mut cfg = ThetaConfig::default();
        cfg.parse.second_seq_scan_window = 50;
        let (sw, pending) = segment::divide_segments_with_tail(&ss, &cfg.parse);
        assert!(
            sw.is_empty(),
            "sign={sign}: 窗口 50 越过资源受限候选判出 {:?}",
            sw.first().map(|s| s.end_index)
        );
        assert_eq!(pending, Some(0));
    }
}

#[test]
fn tb02w_ast_048_01_evidence_at_item_51_is_not_negation() {
    for sign in [1, -1] {
        let ss = chain(&evidence_at_item_51(), sign);
        assert_eq!(ss.len(), 55);
        let p = 1000 * sign;
        let w50 = second_sequence(&ss, dir(sign), 3, p, 50);
        assert_eq!(w50.coverage, ScanCoverage::IncompleteResourceBound);
        assert_eq!(
            w50.semantic_state, None,
            "sign={sign}: 预算耗尽不产语义结论"
        );
        assert!(w50.resource_truncated && w50.matched.is_none() && w50.killed_by.is_none());
        assert_eq!(w50.through_stroke, 53);
        for window in [0, 200] {
            let full = second_sequence(&ss, dir(sign), 3, p, window);
            assert_eq!(full.semantic_state, Some(SecondTerminal::Confirmed));
            assert_eq!(full.matched.as_ref().unwrap().through_stroke, 54);
        }
        let mut cfg = ThetaConfig::default();
        cfg.parse.second_seq_scan_window = 50;
        let (segs, _, facts) = segment::divide_segments_with_facts(&ss, &cfg.parse);
        assert!(
            segs.is_empty(),
            "sign={sign}: 不得越过资源受限候选判后续终结"
        );
        let a = &facts.candidates[0];
        assert_eq!(a.result, "PENDING_CASE_TWO");
        assert!(a
            .waiting_reasons
            .contains(&"second_sequence_resource_bound"));
        assert_eq!(a.fact_vector.segment_broken, None);
        assert!(!a.fact_vector.scan_complete);
    }
}

/// 交替随机游走笔链（确定性 LCG），用于 A-ST-048-02 与三路对拍的广域覆盖。
pub(super) fn random_prices(seed: u64, strokes: usize) -> Vec<i64> {
    let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let mut next = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (state >> 33) as i64
    };
    let mut price = 1_000i64;
    let mut prices = vec![price];
    for i in 0..strokes {
        let step = 1 + next() % 40;
        price += if i % 2 == 0 { step } else { -step };
        prices.push(price);
    }
    prices
}

fn assert_windows_never_change_decided(ss: &[Stroke], label: &str) -> bool {
    let base = ThetaConfig::default();
    let (s0, p0, f0) = segment::divide_segments_with_facts(ss, &base.parse);
    let mut saw_bound = false;
    for window in [50u32, 200] {
        let mut cfg = ThetaConfig::default();
        cfg.parse.second_seq_scan_window = window;
        let (sw, pw, fw) = segment::divide_segments_with_facts(ss, &cfg.parse);
        let bound = fw.candidates.iter().any(|c| {
            c.second
                .as_ref()
                .is_some_and(|s| s.coverage == ScanCoverage::IncompleteResourceBound)
        });
        if bound {
            saw_bound = true;
            assert!(
                sw.len() <= s0.len() && sw[..] == s0[..sw.len()],
                "{label} window={window}: 已判定线段须是窗口 0 结果的前缀"
            );
        } else {
            assert_eq!(
                (&sw, pw, &fw),
                (&s0, p0, &f0),
                "{label} window={window}: 覆盖完整时语义结果须逐位相同"
            );
        }
    }
    saw_bound
}

#[test]
fn tb02w_ast_048_02_windows_0_50_200_never_change_decided_semantics() {
    let named: [&[i64]; 7] = [
        CONFIRMED,
        KILLED,
        OPEN,
        SAME_STROKE,
        TOUCH,
        MERGED_KILL,
        KILLED_THEN_B,
    ];
    let item51 = evidence_at_item_51();
    let bait = skip_bait();
    let mut saw_bound = false;
    for sign in [1, -1] {
        for (i, prices) in named.iter().enumerate() {
            assert_windows_never_change_decided(&chain(prices, sign), &format!("named{i}/{sign}"));
        }
        saw_bound |=
            assert_windows_never_change_decided(&chain(&item51, sign), &format!("item51/{sign}"));
        saw_bound |=
            assert_windows_never_change_decided(&chain(&bait, sign), &format!("bait/{sign}"));
        for seed in 1..=40u64 {
            let prices = random_prices(seed, 60 + (seed as usize % 5) * 20);
            saw_bound |= assert_windows_never_change_decided(
                &chain(&prices, sign),
                &format!("random{seed}/{sign}"),
            );
        }
    }
    assert!(saw_bound, "覆盖为零：语料从未触发资源受限分支");
}

// ─── K1 每候选折叠游标（#1344 第四问，xianduan §5.1）─────────────────────────────────────

fn cand(sign: i64, b: usize, p: i64) -> SecondCandidateId {
    SecondCandidateId {
        seg_dir: dir(sign),
        b_stroke: b,
        candidate_extreme: p * sign,
    }
}

/// 末笔改写：同向延伸前的暂态（末笔终点回缩到半程，至少 1 tick）。
fn wobble_tail(ss: &[Stroke]) -> Vec<Stroke> {
    let mut w = ss.to_vec();
    if let Some(last) = w.last_mut() {
        let back = ((last.end_price - last.start_price).abs() / 2).max(1);
        last.end_price += match last.direction {
            Direction::Up => -back,
            Direction::Down => back,
        };
    }
    w
}

#[test]
fn tb02w_k1_cursor_fields_seal_everything_but_the_tail() {
    for sign in [1, -1] {
        let killed = chain(KILLED, sign);
        let mut cur = SecondFoldCursor::new(cand(sign, 3, 40), 0);
        assert_eq!(
            (cur.schema, cur.rule_revision, cur.budget_policy),
            (
                SECOND_FOLD_CURSOR_SCHEMA,
                SECOND_CANDIDATE_RULE_REVISION,
                SECOND_SEQ_SCAN_BUDGET_POLICY
            )
        );
        assert_eq!(cur.budget_granted, None);
        assert_eq!(cur.state.facts.from_stroke, 4);
        assert_eq!(
            cur.state.dir_state,
            if sign > 0 {
                Some(Direction::Down)
            } else {
                None
            }
        );
        assert_eq!(cur.resume(&killed).unwrap(), second(KILLED, sign, 0));
        // 失效笔 s6 不是输入末笔 ⟹ 终点封存：s4、s5、s6 三笔。
        assert_eq!(cur.state.consumed_len, 3, "sign={sign}");
        assert_eq!(
            cur.state.facts.semantic_state,
            Some(SecondTerminal::KilledByNewExtreme)
        );
        assert_eq!(cur.state.facts.through_stroke, 6);
        assert_eq!(cur.state.facts.breach_watermark, Some(45 * sign));
        assert_eq!(cur.input_stroke_count, killed.len());

        // OPEN：末笔 s6 只暂算，封存态停在 s5。
        let open = chain(OPEN, sign);
        let mut cur = SecondFoldCursor::new(cand(sign, 3, 40), 0);
        assert_eq!(cur.resume(&open).unwrap(), second(OPEN, sign, 0));
        assert_eq!(cur.state.consumed_len, 2, "sign={sign}");
        assert_eq!(cur.state.facts.semantic_state, None);
        let members: Vec<_> = cur
            .state
            .facts
            .standard
            .iter()
            .map(|e| e.stroke_indices.clone())
            .collect();
        assert_eq!(members, vec![vec![4]]);

        // 末笔 s6 22→30 延伸为 22→45：游标未封存 s6，续扫即得失效。
        let mut extended = open.clone();
        extended[6].end_price = 45 * sign;
        let got = cur.resume(&extended).unwrap();
        assert_eq!(got, second_sequence(&extended, dir(sign), 3, 40 * sign, 0));
        assert_eq!(got.semantic_state, Some(SecondTerminal::KilledByNewExtreme));

        // 输入回撤一笔、新末笔 s5 改写：封存的 s5 回退一步，改为暂算。
        let mut cur = SecondFoldCursor::new(cand(sign, 3, 40), 0);
        cur.resume(&open).unwrap();
        let mut retracted = open[..6].to_vec();
        retracted[5].end_price = 20 * sign;
        let got = cur.resume(&retracted).unwrap();
        assert_eq!(got, second_sequence(&retracted, dir(sign), 3, 40 * sign, 0));
        assert_eq!(cur.state.consumed_len, 1);

        // 已封存笔被改写超过一步 ⟹ 拒绝续扫；重折叠与无中断折叠相同。
        let mut cur = SecondFoldCursor::new(cand(sign, 3, 40), 0);
        cur.resume(&killed).unwrap();
        let mut tampered = killed.clone();
        tampered[4].end_price = 36 * sign;
        assert_eq!(cur.resume(&tampered), Err(FoldResumeError::InputChanged));
        assert_eq!(
            cur.refold(&tampered),
            second_sequence(&tampered, dir(sign), 3, 40 * sign, 0)
        );
    }
}

#[test]
fn tb02w_k1_budget_grant_resumes_to_full_budget_semantics() {
    for sign in [1, -1] {
        let ss = chain(&evidence_at_item_51(), sign);
        let p = 1000 * sign;
        let mut cur = SecondFoldCursor::new(cand(sign, 3, 1000), 50);
        assert_eq!(cur.budget_granted, Some(50));
        let w50 = cur.resume(&ss).unwrap();
        assert_eq!(w50, second_sequence(&ss, dir(sign), 3, p, 50));
        assert_eq!(w50.coverage, ScanCoverage::IncompleteResourceBound);
        assert_eq!(w50.semantic_state, None, "sign={sign}: 预算耗尽不是否定");
        assert_eq!(
            cur.state.consumed_len, 50,
            "预算内 50 项都不是输入末笔，全部封存"
        );
        cur.grant(50);
        assert_eq!(cur.budget_granted, Some(100));
        let w100 = cur.resume(&ss).unwrap();
        assert_eq!(w100, second_sequence(&ss, dir(sign), 3, p, 100));
        assert_eq!(
            w100,
            second_sequence(&ss, dir(sign), 3, p, 0),
            "sign={sign}: 完备输入上语义与预算无关"
        );
        assert_eq!(w100.semantic_state, Some(SecondTerminal::Confirmed));
        cur.grant(0);
        assert_eq!(cur.budget_granted, None);
    }
}

/// 分批续扫（逐前缀，含末笔改写与回撤一笔的暂态输入）、自任一切点复制的游标恢复，都与同一
/// 输入上的无中断折叠逐位相同。
#[test]
fn tb02w_k1_resume_and_restore_equal_uninterrupted_fold() {
    let mut corpora: Vec<Vec<i64>> = [
        CONFIRMED,
        KILLED,
        OPEN,
        SAME_STROKE,
        TOUCH,
        MERGED_KILL,
        KILLED_THEN_B,
    ]
    .iter()
    .map(|p| p.to_vec())
    .collect();
    corpora.push(skip_bait());
    for seed in 1..=10u64 {
        corpora.push(random_prices(seed, 24 + (seed as usize % 4) * 8));
    }
    let mut seen = [0usize; 4];
    for sign in [1, -1] {
        for (ci, prices) in corpora.iter().enumerate() {
            let ss = chain(prices, sign);
            for b in 0..ss.len() {
                let seg_dir = ss[b].direction.flip();
                let (hi, lo) = (
                    ss[b].start_price.max(ss[b].end_price),
                    ss[b].start_price.min(ss[b].end_price),
                );
                let c = SecondCandidateId {
                    seg_dir,
                    b_stroke: b,
                    candidate_extreme: if seg_dir == Direction::Up { hi } else { lo },
                };
                for window in [0u32, 5] {
                    let fold = |s: &[Stroke]| {
                        second_sequence(s, c.seg_dir, b, c.candidate_extreme, window)
                    };
                    let label = format!("corpus{ci} sign={sign} b={b} window={window}");
                    let mut cur = SecondFoldCursor::new(c, window);
                    let mut snapshots = Vec::new();
                    for n in 0..=ss.len() {
                        if n >= 1 {
                            let w = wobble_tail(&ss[..n]);
                            assert_eq!(cur.resume(&w).unwrap(), fold(&w), "{label} wobble@{n}");
                        }
                        if n >= 2 {
                            assert_eq!(
                                cur.resume(&ss[..n]).unwrap(),
                                fold(&ss[..n]),
                                "{label}@{n}"
                            );
                            let r = wobble_tail(&ss[..n - 1]);
                            assert_eq!(cur.resume(&r).unwrap(), fold(&r), "{label} retract@{n}");
                        }
                        let got = cur.resume(&ss[..n]).unwrap();
                        assert_eq!(got, fold(&ss[..n]), "{label}@{n}");
                        snapshots.push(cur.clone());
                    }
                    let full = fold(&ss);
                    seen[match full.semantic_state {
                        Some(SecondTerminal::Confirmed) => 0,
                        Some(SecondTerminal::KilledByNewExtreme) => 1,
                        Some(SecondTerminal::OpenAtPrefixEnd) => 2,
                        None => 3,
                    }] += 1;
                    for (p, snap) in snapshots.into_iter().enumerate() {
                        let mut restored = snap;
                        assert_eq!(restored.resume(&ss).unwrap(), full, "{label} restore@{p}");
                    }
                }
            }
        }
    }
    assert!(
        seen.iter().all(|&k| k > 0),
        "语料须覆盖确认、失效、未决与预算耗尽四类：{seen:?}"
    );
}

// ─── 三路逐位对拍：批处理 / 增量 / 恢复（票面验收二，xianduan §5.1）──────────────────────────

/// 偶发大跳的交替笔链（制造缺口候选与深回退），构造思路同 `segment.rs` 的 `gappy_strokes`。
fn gappy_prices(seed: u64, strokes: usize) -> Vec<i64> {
    let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let mut next = move |lo: i64, hi: i64| {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        lo + ((state >> 33) as i64).rem_euclid(hi - lo + 1)
    };
    let mut price = 100_000i64;
    let mut prices = vec![price];
    for i in 0..strokes {
        let amp = if next(0, 9) < 2 {
            next(400, 900)
        } else {
            next(60, 300)
        };
        price += if i % 2 == 0 { amp } else { -amp };
        prices.push(price);
    }
    prices
}

/// 同一完整输入上：批处理（无中断全量）、增量（逐笔 append，含末笔延伸、回缩与撤尾的暂态输入）、
/// 恢复（任一前缀的全量结果 + 检查点安全边界，再续到完整输入）三者线段与 pending 逐位相同。
/// 预算 0（无限）与 5（耗尽常态化）、两个方向都跑。
#[test]
fn tb02w_three_way_batch_incremental_restore_bit_exact() {
    let mut corpora: Vec<Vec<i64>> = [
        CONFIRMED,
        KILLED,
        OPEN,
        SAME_STROKE,
        TOUCH,
        MERGED_KILL,
        KILLED_THEN_B,
    ]
    .iter()
    .map(|p| p.to_vec())
    .collect();
    corpora.push(evidence_at_item_51());
    corpora.push(skip_bait());
    for seed in 1..=5u64 {
        corpora.push(random_prices(seed, 40 + (seed as usize % 3) * 15));
        corpora.push(gappy_prices(seed, 50 + (seed as usize % 3) * 10));
    }
    let mut restores = 0usize;
    for window in [0u32, 5] {
        let mut theta = ThetaConfig::default();
        theta.parse.second_seq_scan_window = window;
        let cfg = theta.parse;
        let batch = |s: &[Stroke]| segment::divide_segments_with_tail(s, &cfg);
        for sign in [1, -1] {
            for (ci, prices) in corpora.iter().enumerate() {
                let ss = chain(prices, sign);
                let label = format!("corpus{ci} sign={sign} window={window}");
                let full = batch(&ss);
                let mut inc = segment::IncrSegments::default();
                let mut checkpoints = Vec::new();
                for n in 0..=ss.len() {
                    let w = wobble_tail(&ss[..n]);
                    // 新笔先以未走完的暂态出现，再延伸到实值；随后同长度回缩再复原（仅末笔改写）。
                    for input in [&w[..], &ss[..n], &w[..], &ss[..n]] {
                        inc = inc.append(input, &cfg);
                        assert_eq!(inc.to_result_vec(), batch(input), "{label} tail@{n}");
                    }
                    if n >= 2 {
                        // 撤尾：生产（parse_layer_from_compact_facts）不复用旧段缓存，自空状态重建。
                        let r = wobble_tail(&ss[..n - 1]);
                        inc = segment::IncrSegments::empty().append(&r, &cfg);
                        assert_eq!(inc.to_result_vec(), batch(&r), "{label} retract@{n}");
                        inc = inc.append(&ss[..n], &cfg);
                    }
                    let (segs, pending) = inc.to_result_vec();
                    assert_eq!((segs.clone(), pending), batch(&ss[..n]), "{label}@{n}");
                    checkpoints.push((segs, pending, inc.earliest_unsealed_from()));
                }
                assert_eq!(inc.to_result_vec(), full, "{label} incremental");
                for (p, (segs, pending, euf)) in checkpoints.into_iter().enumerate() {
                    let restored =
                        segment::IncrSegments::from_checkpoint(&segs, pending, &ss[..p], euf)
                            .append(&ss, &cfg);
                    assert_eq!(restored.to_result_vec(), full, "{label} restore@{p}");
                    restores += 1;
                }
            }
        }
    }
    assert!(restores > 1000, "恢复切点覆盖不足：{restores}");
}

/// 长缺口笔链上逐笔增量、在每个切点存检查点（全量结果 + 安全边界）后恢复续到完整输入，
/// 与无中断全量逐位相同；须覆盖「最早未封闭候选落在末个已确认段之前」的深边界切点。
/// `from_full`（无边界）与全量的差异在本语料上只计数不断言，计数为零不说明无边界恢复安全：
/// 独审 W1 在 99 笔上实测 `from_full` 发散（评审 25760 切点中 24 处），euf 边界的必要性由
/// `tb02w_w1_checkpoint_boundary_is_necessary` 锁住。
#[test]
fn tb02w_checkpoint_restore_long_gappy_bit_exact() {
    let cfg = ThetaConfig::default().parse;
    let mut restores = 0usize;
    let mut deep = 0usize;
    let mut boundary_needed = 0usize;
    for seed in [0xC0FF_EE12_3456_789Au64, 7, 11] {
        for sign in [1, -1] {
            let ss = chain(&gappy_prices(seed, 300), sign);
            let full = segment::divide_segments_with_tail(&ss, &cfg);
            let mut inc = segment::IncrSegments::default();
            for p in 0..=ss.len() {
                inc = inc.append(&ss[..p], &cfg);
                let (segs, pending) = inc.to_result_vec();
                let euf = inc.earliest_unsealed_from();
                let last_end = segs
                    .last()
                    .and_then(|g| ss.iter().position(|s| s.end_index == g.end_index));
                if euf.is_some_and(|e| last_end.is_some_and(|k| e < k)) {
                    deep += 1;
                }
                let restored =
                    segment::IncrSegments::from_checkpoint(&segs, pending, &ss[..p], euf)
                        .append(&ss, &cfg);
                assert_eq!(
                    restored.to_result_vec(),
                    full,
                    "seed={seed} sign={sign} restore@{p}"
                );
                restores += 1;
                let unbounded =
                    segment::IncrSegments::from_full(&segs, pending, &ss[..p]).append(&ss, &cfg);
                if unbounded.to_result_vec() != full {
                    boundary_needed += 1;
                }
            }
            assert_eq!(
                inc.to_result_vec(),
                full,
                "seed={seed} sign={sign} incremental"
            );
        }
    }
    eprintln!(
        "checkpoint boundary: restores={restores} deep={deep} from_full_diverged={boundary_needed}"
    );
    assert!(
        deep > 0,
        "覆盖为零：从未出现深边界切点，安全边界未被实际检验"
    );
}

// ─── K1 线形与 S 调度簿（tb02c 片：跨 cut 续扫、预算追加与恢复）──────────────────────────

fn wire_text(v: serde_json::Value) -> serde_json::Value {
    use serde_json::Value;
    match v {
        Value::Number(n) => Value::String(n.to_string()),
        Value::Array(a) => Value::Array(a.into_iter().map(wire_text).collect()),
        Value::Object(m) => Value::Object(m.into_iter().map(|(k, x)| (k, wire_text(x))).collect()),
        x => x,
    }
}

fn store_corpora() -> Vec<Vec<i64>> {
    let mut corpora: Vec<Vec<i64>> = [
        CONFIRMED,
        KILLED,
        OPEN,
        SAME_STROKE,
        MERGED_KILL,
        KILLED_THEN_B,
    ]
    .iter()
    .map(|p| p.to_vec())
    .collect();
    corpora.push(evidence_at_item_51());
    for seed in 1..=6u64 {
        corpora.push(random_prices(seed, 30 + (seed as usize % 3) * 10));
        corpora.push(gappy_prices(seed, 40));
    }
    corpora
}

/// S 线形（数字转十进制文本）存出的游标，在同一输入上自 b 笔重折叠核验通过、与原游标相等，
/// 下一 cut 续扫等于无中断折叠；线形被改则不采纳，续扫退回重折叠，结果仍相同。
#[test]
fn tb02w_k1_wire_restore_verifies_then_resumes() {
    let mut verified_nonempty = 0usize;
    for sign in [1, -1] {
        for (ci, prices) in store_corpora().iter().enumerate() {
            let ss = chain(prices, sign);
            for b in 0..ss.len() {
                let seg_dir = ss[b].direction.flip();
                let (hi, lo) = (
                    ss[b].start_price.max(ss[b].end_price),
                    ss[b].start_price.min(ss[b].end_price),
                );
                let c = SecondCandidateId {
                    seg_dir,
                    b_stroke: b,
                    candidate_extreme: if seg_dir == Direction::Up { hi } else { lo },
                };
                for window in [0u32, 5] {
                    let full = second_sequence(&ss, seg_dir, b, c.candidate_extreme, window);
                    for p in (b + 1)..ss.len() {
                        let label = format!("corpus{ci} sign={sign} b={b} w={window} p={p}");
                        let mut cur = SecondFoldCursor::new(c, window);
                        cur.resume(&ss[..p]).unwrap();
                        let wire = wire_text(cur.to_wire());
                        let (restored, verified, _) =
                            SecondFoldCursor::from_wire(&wire, &ss[..p]).unwrap();
                        assert!(verified, "{label}");
                        assert_eq!(restored, cur, "{label}");
                        verified_nonempty += usize::from(cur.state.consumed_len > 0);
                        let mut next = restored;
                        assert_eq!(next.resume(&ss).unwrap(), full, "{label} resume");
                        if cur.state.consumed_len > 0 {
                            let mut forged = wire.clone();
                            forged["consumed_digest"] = serde_json::json!("0000000000000000");
                            let (mut f, ok, _) =
                                SecondFoldCursor::from_wire(&forged, &ss[..p]).unwrap();
                            assert!(!ok, "{label}: 篡改的线形不得采纳");
                            assert_eq!(
                                f.budget_granted, cur.budget_granted,
                                "{label}: 保留累计预算"
                            );
                            assert_eq!(f.resume(&ss).unwrap(), full, "{label} forged");
                        }
                    }
                }
            }
        }
    }
    assert!(
        verified_nonempty > 1000,
        "核验覆盖不足：{verified_nonempty}"
    );
}

/// NOT_RUN：真实非空游标经借用核恢复；shape 篡改仍走原重折叠，并保留累计预算。
#[test]
fn tb02w_k1_borrowed_restore_keeps_nonempty_contract() {
    for sign in [1, -1] {
        let ss = chain(CONFIRMED, sign);
        let candidate = SecondCandidateId {
            seg_dir: dir(sign),
            b_stroke: 3,
            candidate_extreme: 40 * sign,
        };
        let mut cursor = SecondFoldCursor::new(candidate, 3);
        cursor.resume(&ss).unwrap();
        assert!(cursor.state.consumed_len > 0);
        assert!(!cursor.state.facts.standard.is_empty());
        for wire in [cursor.to_wire(), wire_text(cursor.to_wire())] {
            let (restored, verified, exhausted) = SecondFoldCursor::from_wire(&wire, &ss).unwrap();
            assert!(verified);
            assert_eq!(restored, cursor);
            assert_eq!(exhausted, cursor.exhausted());
            for mutation in 0..3 {
                let mut forged = wire.clone();
                match mutation {
                    0 => {
                        forged.as_object_mut().unwrap().remove("matched");
                    }
                    1 => {
                        forged["standard"][0]["extra"] = serde_json::Value::Null;
                    }
                    _ => {
                        forged["standard"][0]["stroke_indices"] = serde_json::json!([]);
                    }
                }
                let (mut reset, verified, _) = SecondFoldCursor::from_wire(&forged, &ss).unwrap();
                assert!(!verified);
                assert_eq!(reset.budget_granted, cursor.budget_granted);
                assert_eq!(
                    reset.resume(&ss).unwrap(),
                    second_sequence(&ss, dir(sign), 3, 40 * sign, 3)
                );
            }
        }
    }
}

fn round(
    ss: &[Stroke],
    budget: u32,
    wires: &[serde_json::Value],
) -> (
    (Vec<Segment>, Option<usize>, segment::SegmentFacts),
    Vec<serde_json::Value>,
    (usize, usize),
) {
    let mut store = SecondFoldStore::new(budget);
    for w in wires {
        store.restore(w);
    }
    let cfg = ThetaConfig::default().parse;
    let (segs, pending, facts, store) = segment::divide_segments_with_facts_store(ss, &cfg, store);
    for c in &facts.candidates {
        if let Some(s) = &c.second {
            let id = SecondCandidateId {
                seg_dir: ss[c.start_stroke].direction,
                b_stroke: s.from_stroke - 1,
                candidate_extreme: s.candidate_extreme,
            };
            assert!(store.cursor(&id).is_some(), "第二种候选必经游标簿");
        }
    }
    let next = store.checkpoint().into_iter().map(wire_text).collect();
    ((segs, pending, facts), next, store.round_counts())
}

fn has_bound(facts: &segment::SegmentFacts) -> bool {
    facts.candidates.iter().any(|c| {
        c.second
            .as_ref()
            .is_some_and(|s| s.coverage == ScanCoverage::IncompleteResourceBound)
    })
}

/// S 每 cut 一轮：预算 0 时，逐 cut 经 K1 线形带入上一轮游标的划分与无游标批处理逐位相同；
/// 预算耗尽的候选在下一轮先追加预算再续扫，同一完整输入上若干轮后与预算 0 逐位相同，不会永久停在
/// INCOMPLETE；A-ST-048-01 的第 51 项证据第二轮即确认。
#[test]
fn tb02w_store_rounds_match_batch_and_grant_exhausted_cursors() {
    let cfg = ThetaConfig::default().parse;
    let (mut verified, mut grants) = (0usize, 0usize);
    for sign in [1, -1] {
        for (ci, prices) in store_corpora().iter().enumerate() {
            let ss = chain(prices, sign);
            let mut wires = vec![];
            for n in 0..=ss.len() {
                let (got, next, (v, granted)) = round(&ss[..n], 0, &wires);
                assert_eq!(
                    got,
                    segment::divide_segments_with_facts(&ss[..n], &cfg),
                    "corpus{ci} sign={sign} cut@{n}"
                );
                assert_eq!((v, granted), (0, 0), "预算 0 无调度相关游标");
                assert!(next.is_empty(), "预算 0 的检查点为空");
                wires = next;
            }
            // 预算 5、输入逐笔增长、每 cut 一轮（同 S）：已判定线段始终是同前缀预算 0 结果的前缀，
            // 无耗尽候选时逐位相同；输入走完后继续调度，至多 len/5+2 轮与预算 0 逐位相同。
            let mut wires = vec![];
            for n in 0..=ss.len() {
                let (got, next, (v, g)) = round(&ss[..n], 5, &wires);
                let base = segment::divide_segments_with_facts(&ss[..n], &cfg);
                let label = format!("corpus{ci} sign={sign} budget5 cut@{n}");
                assert!(
                    got.0.len() <= base.0.len() && got.0[..] == base.0[..got.0.len()],
                    "{label}: 已判定线段须是预算 0 的前缀"
                );
                if !has_bound(&got.2) {
                    assert_eq!(got, base, "{label}");
                }
                verified += v;
                grants += g;
                wires = next;
            }
            let target = segment::divide_segments_with_facts(&ss, &cfg);
            let mut rounds = 0;
            loop {
                let (got, next, (v, g)) = round(&ss, 5, &wires);
                rounds += 1;
                verified += v;
                grants += g;
                if !has_bound(&got.2) {
                    assert_eq!(got, target, "corpus{ci} sign={sign}: 续扫完毕须等于预算 0");
                    break;
                }
                assert!(
                    rounds <= ss.len() / 5 + 2,
                    "corpus{ci} sign={sign}: 续扫不收敛"
                );
                wires = next;
            }
        }
        let ss = chain(&evidence_at_item_51(), sign);
        let (first, wires, _) = round(&ss, 50, &[]);
        assert!(
            first.0.is_empty() && has_bound(&first.2),
            "sign={sign}: 第一轮预算耗尽"
        );
        let (second, _, (_, granted)) = round(&ss, 50, &wires);
        assert_eq!(granted, 1, "sign={sign}: 第二轮对耗尽游标追加预算");
        assert_eq!(second, segment::divide_segments_with_facts(&ss, &cfg));
        assert_eq!(second.2.candidates[0].result, "CASE_TWO");
    }
    assert!(
        verified > 100 && grants > 20,
        "跨轮核验/追加覆盖不足：verified={verified} grants={grants}"
    );
}

// ─── 独审见证夹具（#1420 独审 tb1420-review/witnesses.json，RESULT BLOCKED_3 第③项）──────────

/// 见证的笔链约定（独审 `mk_chain`）：第 i 笔 = prices[i-1]→prices[i]，start_index = 3i，
/// end_index = 3i + 3。与本文件 `chain` 的下标约定不同；见证读数（段终点下标）依此约定。
fn witness_chain(prices: &[i64]) -> Vec<Stroke> {
    (1..prices.len())
        .map(|i| {
            let (a, b) = (prices[i - 1], prices[i]);
            Stroke {
                direction: if b > a {
                    Direction::Up
                } else {
                    Direction::Down
                },
                start_index: i * 3,
                end_index: i * 3 + 3,
                start_price: a,
                end_price: b,
            }
        })
        .collect()
}

/// W1 价格（独审生成器 `regime_prices(5004, 160)` 前 100 个，向上）：99 笔。
const W1_PRICES: [i64; 100] = [
    100000, 100049, 99954, 100046, 99925, 100075, 99881, 100033, 99915, 99987, 99935, 99981, 99941,
    99991, 99966, 99982, 99968, 99970, 99968, 99988, 99968, 99984, 99962, 99964, 99955, 99976,
    99974, 99989, 99891, 99977, 99915, 99967, 99919, 99960, 99945, 99969, 99951, 99953, 99941,
    99949, 99947, 99949, 99947, 99951, 99932, 99934, 99919, 99988, 99922, 101146, 99892, 99961,
    99889, 99964, 98854, 98916, 98846, 98919, 98849, 98915, 98868, 98907, 98836, 98900, 98851,
    98901, 98847, 98909, 98779, 98932, 98760, 98990, 98721, 99074, 98646, 99197, 98519, 99347,
    98316, 99615, 99448, 99600, 99463, 99546, 99457, 99525, 99464, 99504, 99460, 99490, 99465,
    99484, 99458, 99466, 99464, 99466, 99463, 99467, 99446, 99448,
];

/// 独审 W1：逐笔增量到 p = 97 存检查点（全量结果 + 安全边界 euf = 62），续扫到 q = 98、99。
/// 带边界的 `from_checkpoint` 每步与无中断全量逐位相同；无边界的 `from_full` 在 q = 99 与全量
/// 发散。这一条锁住 euf：`from_checkpoint` 丢弃边界（退化为 `from_full`）即红。
#[test]
fn tb02w_w1_checkpoint_boundary_is_necessary() {
    let cfg = ThetaConfig::default().parse;
    assert_eq!(cfg.second_seq_scan_window, 0);
    let ss = witness_chain(&W1_PRICES);
    assert_eq!(ss.len(), 99);
    let p = 97;
    let mut inc = segment::IncrSegments::default();
    for n in 0..=p {
        inc = inc.append(&ss[..n], &cfg);
    }
    let (segs, pending) = inc.to_result_vec();
    let euf = inc.earliest_unsealed_from();
    assert_eq!(euf, Some(62), "切点 p=97 须有未封闭第二种候选（深边界）");
    let mut diverged = Vec::new();
    for q in p..=ss.len() {
        let batch = segment::divide_segments_with_tail(&ss[..q], &cfg);
        let bounded = segment::IncrSegments::from_checkpoint(&segs, pending, &ss[..p], euf)
            .append(&ss[..q], &cfg);
        assert_eq!(bounded.to_result_vec(), batch, "from_checkpoint@q={q}");
        let unbounded =
            segment::IncrSegments::from_full(&segs, pending, &ss[..p]).append(&ss[..q], &cfg);
        if unbounded.to_result_vec() != batch {
            diverged.push(q);
        }
    }
    assert_eq!(
        diverged,
        [99],
        "from_full（无边界）须恰在 q=99 发散，见证 euf 边界必要"
    );
}

/// W2 价格（独审生成器 `spiky_prices(21, 40)` 前 25 个，向上）：24 笔。
const W2_PRICES: [i64; 25] = [
    50000, 50070, 50024, 50068, 50033, 50049, 50038, 50133, 50074, 50233, 49515, 49523, 49472,
    49541, 49488, 49630, 49436, 49484, 49384, 49493, 49428, 49466, 49418, 49616, 49553,
];

/// 独审 W2：窗口 1 下第二种候选的试探中途预算耗尽。须报 INCOMPLETE_RESOURCE_BOUND、留待定尾，
/// 且已判段是窗口 0 结果的前缀（耗尽不产否定、不提前封段）。窗口 0 段终点下标为 [30, 57]；
/// 回退「试探中耗尽也阻塞封段」的修正后，窗口 1 不报耗尽并静默把第二段终点改判为 63（即红）。
#[test]
fn tb02w_w2_provisional_budget_exhaustion_blocks_sealing() {
    let ss = witness_chain(&W2_PRICES);
    assert_eq!(ss.len(), 24);
    let mut w0 = ThetaConfig::default();
    w0.parse.second_seq_scan_window = 0;
    let mut w1 = ThetaConfig::default();
    w1.parse.second_seq_scan_window = 1;
    let (s0, p0, f0) = segment::divide_segments_with_facts(&ss, &w0.parse);
    let (s1, p1, f1) = segment::divide_segments_with_facts(&ss, &w1.parse);
    let ends = |segs: &[Segment]| segs.iter().map(|g| g.end_index).collect::<Vec<_>>();
    assert_eq!(ends(&s0), [30, 57], "窗口 0 基准读数");
    assert!(
        !f0.candidates.iter().any(|c| c
            .second
            .as_ref()
            .is_some_and(|s| s.coverage == ScanCoverage::IncompleteResourceBound)),
        "窗口 0 不得报耗尽"
    );
    let _ = p0;
    assert!(
        f1.candidates.iter().any(|c| c
            .second
            .as_ref()
            .is_some_and(|s| s.coverage == ScanCoverage::IncompleteResourceBound)),
        "窗口 1 须报 INCOMPLETE_RESOURCE_BOUND"
    );
    assert!(p1.is_some(), "预算耗尽须留待定尾");
    assert!(
        s1.len() <= s0.len() && s1[..] == s0[..s1.len()],
        "窗口 1 已判段须为窗口 0 前缀：w1={:?} w0={:?}",
        ends(&s1),
        ends(&s0)
    );
}
