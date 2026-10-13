//! #1405（TB-02-V）：#1343 同价裁定的行为锁。
//!
//! oracle = `.chanlun/definitions/bi.md` §7.1 + #1343 `WITNESS-WALK.md`（六见证经生产入口的输出，
//! 夹具 `rust/tests/fixtures/tb02v_same_price/`）。证明义务的载体一律是测试锁（#804）：
//! P1 确定性（双新进程哈希）、P2 前缀不回退、P3 选择无关、P4 full ≡ 增量。
use super::tests::{check_incremental_prefixes, hl_bars, without_expanded_sources};
use super::*;
use crate::theta_v0::backtest::data::load_symbol;
use crate::theta_v0::config::ThetaConfig;
use crate::theta_v0::parser::inclusion::IncrFactsState;
use crate::theta_v0::parser::{parse_layer, parse_layer_with_inclusion_facts};
use std::path::Path;

const WITNESS_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/tb02v_same_price"
);

/// 见证切片按 `load_symbol` 同一量化/坏 bar 规则读入；`stride` 只稀疏化源坐标（raw 序位不变）。
fn witness(name: &str, stride: usize) -> Vec<Bar> {
    let path = Path::new(WITNESS_DIR).join(format!("{name}.json"));
    let mut bars = load_symbol(&path, name, &ThetaConfig::default(), 60)
        .unwrap_or_else(|e| panic!("见证夹具 {name}: {e}"))
        .bars;
    for bar in &mut bars {
        bar.source_index *= stride;
    }
    bars
}

/// `WITNESS-WALK.md` 的事件码：同型 replace / keep / keep_eq，异型 formed / hetero_not_formed。
fn event_of(p: &PairFact) -> &'static str {
    match (p.selection, p.comparison) {
        (Some("REPLACE"), _) => "replace",
        (Some("KEEP"), Some("EQ")) => "keep_eq",
        (Some("KEEP"), _) => "keep",
        (Some(other), _) => panic!("同型取舍只有 KEEP/REPLACE，见到 {other}"),
        (None, _) if p.conditions.as_ref().is_some_and(Conditions::formed) => "formed",
        (None, _) => "hetero_not_formed",
    }
}

type Cond = (usize, usize, [bool; 3]);
struct Walk {
    name: &'static str,
    merged: usize,
    fractals: usize,
    /// (kind, 组锚, 极值根集)；raw_first/raw_last = 根集首末（见证自 raw 0 起，序位 = 坐标）。
    endpoints: &'static [(&'static str, usize, &'static [usize])],
    /// 每个非首端点一次转移：(事件码, 新端点组锚, 异型条件 (merged_gap, raw_between, 向量))。
    events: &'static [(&'static str, usize, Option<Cond>)],
    /// (起点组锚, 终点组锚, 终点极值根集, merged_gap, raw_between)。
    strokes: &'static [(usize, usize, &'static [usize], usize, usize)],
}

const TTT: [bool; 3] = [true, true, true];
const FFT: [bool; 3] = [false, false, true];
const TFT: [bool; 3] = [true, false, true];
const FTT: [bool; 3] = [false, true, true];

/// #1343 `WITNESS-WALK.md` 逐件抄录（坐标 = 切片内 raw 序号）。
const WALKS: &[Walk] = &[
    Walk {
        name: "real_B_bars25_30_minimal_eq",
        merged: 6,
        fractals: 3,
        endpoints: &[("BOTTOM", 1, &[1]), ("TOP", 2, &[2]), ("BOTTOM", 4, &[4])],
        events: &[
            ("hetero_not_formed", 2, Some((1, 0, FFT))),
            ("keep_eq", 4, None),
        ],
        strokes: &[],
    },
    Walk {
        name: "real_A_bars14_30_formed_then_popped",
        merged: 11,
        fractals: 6,
        endpoints: &[
            ("TOP", 1, &[1]),
            ("BOTTOM", 6, &[6]),
            ("TOP", 7, &[9, 10]),
            ("BOTTOM", 12, &[12]),
            ("TOP", 13, &[13]),
            ("BOTTOM", 15, &[15]),
        ],
        events: &[
            ("hetero_not_formed", 6, Some((2, 4, FTT))),
            ("keep", 7, None),
            ("formed", 12, Some((5, 10, TTT))),
            ("hetero_not_formed", 13, Some((1, 0, FFT))),
            ("keep_eq", 15, None),
        ],
        strokes: &[(1, 12, &[12], 5, 10)],
    },
    Walk {
        name: "real_C_bars14_40_multi_root_second_gate",
        merged: 19,
        fractals: 10,
        endpoints: &[
            ("TOP", 1, &[1]),
            ("BOTTOM", 6, &[6]),
            ("TOP", 7, &[9, 10]),
            ("BOTTOM", 12, &[12]),
            ("TOP", 13, &[13]),
            ("BOTTOM", 15, &[15]),
            ("TOP", 16, &[16]),
            ("BOTTOM", 17, &[17]),
            ("TOP", 21, &[21, 22]),
            ("BOTTOM", 24, &[24]),
        ],
        events: &[
            ("hetero_not_formed", 6, Some((2, 4, FTT))),
            ("keep", 7, None),
            ("formed", 12, Some((5, 10, TTT))),
            ("hetero_not_formed", 13, Some((1, 0, FFT))),
            ("keep_eq", 15, None),
            ("formed", 16, Some((4, 3, TTT))),
            ("hetero_not_formed", 17, Some((1, 0, FFT))),
            ("replace", 21, None),
            ("hetero_not_formed", 24, Some((2, 1, FFT))),
        ],
        strokes: &[(1, 12, &[12], 5, 10), (12, 21, &[21, 22], 9, 8)],
    },
    Walk {
        name: "syn1_same_kind_equal_price",
        merged: 14,
        fractals: 6,
        endpoints: &[
            ("TOP", 3, &[3]),
            ("BOTTOM", 7, &[7]),
            ("TOP", 8, &[8]),
            ("BOTTOM", 9, &[9]),
            ("TOP", 10, &[10]),
            ("BOTTOM", 11, &[11]),
        ],
        events: &[
            ("formed", 7, Some((4, 3, TTT))),
            ("hetero_not_formed", 8, Some((1, 0, FFT))),
            ("keep_eq", 9, None),
            ("hetero_not_formed", 10, Some((3, 2, TFT))),
            ("replace", 11, None),
        ],
        strokes: &[(3, 11, &[11], 8, 7)],
    },
    Walk {
        name: "syn2_multi_root_group",
        merged: 14,
        fractals: 6,
        endpoints: &[
            ("TOP", 3, &[3]),
            ("BOTTOM", 7, &[7]),
            ("TOP", 8, &[8]),
            ("BOTTOM", 9, &[9]),
            ("TOP", 10, &[10, 11]),
            ("BOTTOM", 13, &[13]),
        ],
        events: &[
            ("formed", 7, Some((4, 3, TTT))),
            ("hetero_not_formed", 8, Some((1, 0, FFT))),
            ("keep", 9, None),
            // 极值区判别见证：raw_between = pos(10) − pos(7) − 1 = 2，最晚根取法会得 3 而收。
            ("hetero_not_formed", 10, Some((3, 2, TFT))),
            ("keep", 13, None),
        ],
        strokes: &[(3, 7, &[7], 4, 3)],
    },
    Walk {
        name: "syn3_multi_root_replace",
        merged: 13,
        fractals: 5,
        endpoints: &[
            ("BOTTOM", 3, &[3]),
            ("TOP", 7, &[7]),
            ("BOTTOM", 8, &[8]),
            ("TOP", 9, &[9, 10]),
            ("BOTTOM", 12, &[12]),
        ],
        events: &[
            ("formed", 7, Some((4, 3, TTT))),
            ("hetero_not_formed", 8, Some((1, 0, FFT))),
            ("replace", 9, None),
            ("hetero_not_formed", 12, Some((2, 1, FFT))),
        ],
        strokes: &[(3, 9, &[9, 10], 6, 5)],
    },
];

/// 验收 1 + P2/P4（见证）：六见证经生产入口（`parse_layer` 与同次 facts 入口）得到
/// `WITNESS-WALK.md` 的输出，全程无等待/冻结，逐前缀增量 ≡ 全量。
#[test]
fn tb02v_six_witnesses_through_production_entry() {
    let theta = ThetaConfig::default();
    for w in WALKS {
        for stride in [1usize, 7] {
            let raw = witness(w.name, stride);
            let label = format!("{} stride {stride}", w.name);
            let layer = parse_layer(&raw, &theta);
            let (facts_layer, inclusion) = parse_layer_with_inclusion_facts(&raw, &theta);
            assert_eq!(
                layer, facts_layer,
                "{label}：紧凑生产入口与 facts 入口逐字段相同"
            );
            let facts = inclusion.stroke_facts.as_ref().unwrap();
            assert_eq!(facts.policy, SAME_PRICE_POLICY, "{label}");
            assert_eq!(layer.merged_bars.len(), w.merged, "{label} merged");
            assert_eq!(layer.fractals.len(), w.fractals, "{label} fractals");
            assert!(facts.waiting_reasons.is_empty(), "{label}：同价域无冻结");

            let endpoints: Vec<_> = facts
                .endpoints
                .iter()
                .map(|e| {
                    assert!(e.waiting_reasons.is_empty(), "{label}");
                    (
                        e.kind,
                        e.group_anchor,
                        e.extreme_roots.clone(),
                        e.raw_first,
                        e.raw_last,
                    )
                })
                .collect();
            let want: Vec<_> = w
                .endpoints
                .iter()
                .map(|(kind, anchor, roots)| {
                    (
                        *kind,
                        anchor * stride,
                        roots.iter().map(|r| r * stride).collect::<Vec<_>>(),
                        *roots.iter().min().unwrap(),
                        *roots.iter().max().unwrap(),
                    )
                })
                .collect();
            assert_eq!(endpoints, want, "{label} endpoints");

            let events: Vec<_> = facts
                .pairs
                .iter()
                .map(|p| {
                    (
                        event_of(p),
                        p.new.group_anchor,
                        p.conditions.as_ref().map(|c| {
                            (
                                c.merged_gap,
                                c.raw_between_actual_extrema,
                                c.vector.map(|v| v.expect("条件位恒确定（#1343 Q3）")),
                            )
                        }),
                    )
                })
                .collect();
            let want: Vec<_> = w
                .events
                .iter()
                .map(|(event, anchor, c)| (*event, anchor * stride, *c))
                .collect();
            assert_eq!(events, want, "{label} events");
            for p in &facts.pairs {
                assert!(p.waiting_reasons.is_empty(), "{label}");
                let keep_eq = event_of(p) == "keep_eq";
                assert_eq!(
                    p.evidence,
                    if keep_eq {
                        vec![SAME_KIND_EQUAL_PRICE_KEEP_EARLIER]
                    } else {
                        vec![]
                    },
                    "{label}"
                );
                if let Some(selection) = p.selection {
                    let kept = if selection == "REPLACE" {
                        &p.new
                    } else {
                        &p.old
                    };
                    assert_eq!(p.retained_anchors, vec![kept.group_anchor], "{label}");
                }
            }

            let strokes: Vec<_> = facts
                .strokes
                .iter()
                .map(|s| {
                    (
                        s.start.group_anchor,
                        s.end.group_anchor,
                        s.end.extreme_roots.clone(),
                        s.formation.merged_gap,
                        s.formation.raw_between_actual_extrema,
                    )
                })
                .collect();
            let want: Vec<_> = w
                .strokes
                .iter()
                .map(|(a, b, roots, gap, between)| {
                    (
                        a * stride,
                        b * stride,
                        roots.iter().map(|r| r * stride).collect::<Vec<_>>(),
                        *gap,
                        *between,
                    )
                })
                .collect();
            assert_eq!(strokes, want, "{label} strokes");
            assert_eq!(*layer.strokes, facts.production_strokes(), "{label}");
            check_incremental_prefixes(&raw, &label);
        }
    }
}

/// 确定性 xorshift64*（测试自带，不引随机依赖）。
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn shuffle(&mut self, xs: &mut [usize]) {
        for i in (1..xs.len()).rev() {
            xs.swap(i, self.below(i as u64 + 1) as usize);
        }
    }
}

/// 小整数价的分段趋势游走：组内多根同极值与跨组同型同价都高频出现，同时有足够长的
/// 单边段落成笔。首两根严格向上，避开初始方向未定。
fn tie_heavy(rng: &mut Rng, n: usize) -> Vec<(i64, i64)> {
    let mut out = vec![(20, 18), (22, 20)];
    let mut drift = 1i64;
    while out.len() < n {
        if rng.below(6) == 0 {
            drift = -drift;
        }
        let (h, l) = *out.last().unwrap();
        let mid =
            ((h + l) / 2 + drift * rng.below(3) as i64 + rng.below(3) as i64 - 1).clamp(6, 60);
        let half = 1 + rng.below(2) as i64;
        out.push((mid + half, mid - half));
    }
    out
}

/// 把所有端点（含 pair 两端、笔两端）的极值根集按坐标排序，只消去「到达序」这一个维度。
fn roots_as_sets(mut facts: StrokeFacts) -> StrokeFacts {
    let norm = |e: &mut Endpoint| e.extreme_roots.sort_unstable();
    facts.endpoints.iter_mut().for_each(norm);
    for p in &mut facts.pairs {
        norm(&mut p.old);
        norm(&mut p.new);
    }
    for s in &mut facts.strokes {
        norm(&mut s.start);
        norm(&mut s.end);
    }
    facts
}

/// P3 选择无关 + P2/P4（随机）：打乱并列极值根的到达序（价格、成员、组不变），条件向量与笔
/// 序列逐位不变；每个异型对的 raw 间隔等于「对两端任意指定并列根」所得间隔的最小值。
#[test]
fn tb02v_tied_root_order_and_designation_do_not_change_vectors_or_strokes() {
    let config = ParseConfig::default();
    let mut rng = Rng(0x1405_1343_0000_0001);
    let (mut multi_root_endpoints, mut keep_eq, mut strokes, mut designations) = (0, 0, 0, 0);
    for case in 0..160 {
        let stride = 1 + case % 3;
        let n = 60 + rng.below(140) as usize;
        let raw = hl_bars(&tie_heavy(&mut rng, n), stride);
        let inclusion = process_inclusion_with_facts(&raw);
        let base = build_stroke_facts(&inclusion, &config);
        let positions = positions_of_steps(&inclusion);
        multi_root_endpoints += base
            .endpoints
            .iter()
            .filter(|e| e.extreme_roots.len() > 1)
            .count();
        keep_eq += base
            .pairs
            .iter()
            .filter(|p| p.evidence.contains(&SAME_KIND_EQUAL_PRICE_KEEP_EARLIER))
            .count();
        strokes += base.strokes.len();
        for p in base.pairs.iter().filter(|p| p.conditions.is_some()) {
            let mut least = usize::MAX;
            for x in &p.old.extreme_roots {
                for y in &p.new.extreme_roots {
                    let (px, py) = (positions[x], positions[y]);
                    assert!(py > px, "case {case}：时间先后的极值区不相交");
                    least = least.min(py - px - 1);
                    designations += 1;
                }
            }
            assert_eq!(
                p.conditions.as_ref().unwrap().raw_between_actual_extrema,
                least,
                "case {case}：极值区间隔 = 对任意指定取最小"
            );
        }
        for round in 0..3 {
            let mut permuted = inclusion.clone();
            for g in &mut permuted.groups {
                rng.shuffle(&mut g.high_sources);
                rng.shuffle(&mut g.low_sources);
            }
            let shuffled = build_stroke_facts(&permuted, &config);
            assert_eq!(
                roots_as_sets(shuffled),
                roots_as_sets(base.clone()),
                "case {case} round {round}：并列根到达序不进任何判据"
            );
        }
        if case < 24 {
            check_incremental_prefixes(&raw, &format!("tie_heavy case {case}"));
        }
    }
    eprintln!(
        "tb02v property: multi_root_endpoints={multi_root_endpoints} keep_eq={keep_eq} strokes={strokes} designations={designations}"
    );
    assert!(multi_root_endpoints > 200, "生成器须真实覆盖多根组端点");
    assert!(keep_eq > 50, "生成器须真实覆盖同型同价 KEEP");
    assert!(strokes > 500, "生成器须真实成笔");
}

/// 无同价竞争输入的 A/B 同值：单根且无同型同价时，极值区读数与单根 raw 序位逐位相同，
/// 冻结账簿中其余 case 的向量/间隔/笔与 v1 手算账簿一致（由 hand-oracle 驱动的测试覆盖）。
#[test]
fn tb02v_single_root_without_equal_price_matches_single_position_reading() {
    let mut rng = Rng(0x1405_0000_0000_00AB);
    let config = ParseConfig::default();
    let mut checked = 0;
    for case in 0..80 {
        // 无包含锯齿：相邻根区间相隔 ≥5000、宽 900，组 = 单根；价带根序号尾数，全序列无同价。
        let n = 40 + rng.below(80) as usize;
        let mut hl = vec![];
        let mut level = 1000i64;
        let mut up = true;
        for i in 0..n {
            if i > 1 && i % (3 + rng.below(4) as usize) == 0 {
                up = !up;
            }
            level += if up {
                7 + i as i64 % 3
            } else {
                -(5 + i as i64 % 2)
            };
            hl.push((level * 1000 + 900 + i as i64, level * 1000 + i as i64));
        }
        let raw = hl_bars(&hl, 1 + case % 4);
        let inclusion = process_inclusion_with_facts(&raw);
        let facts = build_stroke_facts(&inclusion, &config);
        let positions = positions_of_steps(&inclusion);
        for e in &facts.endpoints {
            if e.extreme_roots.len() != 1 {
                continue;
            }
            let single = positions[&e.extreme_roots[0]];
            assert_eq!((e.raw_first, e.raw_last), (single, single), "case {case}");
            checked += 1;
        }
        assert!(
            facts.pairs.iter().all(|p| p.evidence.is_empty()),
            "case {case}：生成器不得含同价"
        );
        for p in facts.pairs.iter().filter(|p| p.conditions.is_some()) {
            if p.old.extreme_roots.len() == 1 && p.new.extreme_roots.len() == 1 {
                let single =
                    positions[&p.new.extreme_roots[0]] - positions[&p.old.extreme_roots[0]] - 1;
                assert_eq!(
                    p.conditions.as_ref().unwrap().raw_between_actual_extrema,
                    single
                );
            }
        }
    }
    assert!(checked > 300, "须真实覆盖单根端点");
}

fn replay_digest() -> String {
    use sha2::{Digest, Sha256};
    let theta = ThetaConfig::default();
    let mut h = Sha256::new();
    for w in WALKS {
        let (_, inclusion) = parse_layer_with_inclusion_facts(&witness(w.name, 1), &theta);
        h.update(serde_json::to_vec(inclusion.stroke_facts.as_ref().unwrap()).unwrap());
    }
    let mut rng = Rng(0x1405_A714);
    let raw = hl_bars(&tie_heavy(&mut rng, 400), 3);
    h.update(
        serde_json::to_vec(&build_stroke_facts(
            &process_inclusion_with_facts(&raw),
            &theta.parse,
        ))
        .unwrap(),
    );
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// P1 确定性（AT-14）：同输入在两个全新进程里重放，`StrokeFacts` 序列化哈希逐位相等，
/// 且等于本进程的读数（facts 是输入的纯函数，无全局态）。
#[test]
fn tb02v_replay_hash_equal_across_fresh_processes() {
    use crate::theta_v0::env_registry::TB02V_REPLAY_CHILD as CHILD;
    if std::env::var_os(CHILD).is_some() {
        println!("TB02V_REPLAY_DIGEST={}", replay_digest());
        return;
    }
    let exe = std::env::current_exe().unwrap();
    let name =
        "theta_v0::parser::stroke::tb02v_tests::tb02v_replay_hash_equal_across_fresh_processes";
    let run = || {
        let out = std::process::Command::new(&exe)
            .args(["--exact", name, "--nocapture", "--test-threads=1"])
            .env(CHILD, "1")
            .output()
            .expect("新进程重放");
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let stdout = String::from_utf8(out.stdout).unwrap();
        // libtest 把子进程打印接在 `test … ` 同一行之后：按标记切出其后 64 位十六进制。
        stdout
            .split("TB02V_REPLAY_DIGEST=")
            .nth(1)
            .and_then(|rest| rest.get(..64))
            .unwrap_or_else(|| panic!("子进程未打印读数：{stdout}"))
            .to_string()
    };
    let (a, b) = (run(), run());
    eprintln!("tb02v replay digest a={a} b={b}");
    assert_eq!(a, b, "两个新进程同输入重放哈希不等");
    assert_eq!(a, replay_digest(), "新进程与本进程读数不等");
}

/// 紧凑全量重建（生产 `parse_layer` 同一路径）：`IncrFactsState` 折叠 + 唯一 transition。
fn compact_full(bars: &[Bar], config: &ParseConfig) -> StrokeFacts {
    let mut state = IncrFactsState::empty();
    for bar in bars {
        state.append(*bar);
    }
    build_stroke_facts_with_positions(
        state.merged_slice(),
        state.groups_slice(),
        state.unsettled(),
        state.positions(),
        config,
    )
}

/// 验收 2 + 4（真实 BTC 1m 尾 5 万根，外部数据）：读数锁与前缀不回退阶梯。
///
/// 数据不入仓（3.5 MB）：`TB02V_BTC_TAIL50K=<btc_tail50k.json>` 指向
/// `analysis/data_cache/btc_1m_full.json` 尾 50,000 根（sha256 `4c005de4…601f`，先校验）。
/// 不得改断言修绿：不符先核 Python 移植（诊断 `port.py` + #1343 `ruling_replay.py`）再核实装。
#[test]
#[ignore = "外部数据：TB02V_BTC_TAIL50K=<btc_tail50k.json>；收据见 .chanlun/review-results/issue1405-*"]
fn tb02v_btc_tail50k_reading_and_prefix_ladder() {
    use crate::theta_v0::env_registry::TB02V_BTC_TAIL50K;
    use sha2::{Digest, Sha256};
    let path = std::env::var(TB02V_BTC_TAIL50K).expect("TB02V_BTC_TAIL50K=<btc_tail50k.json>");
    let bytes = std::fs::read(&path).unwrap();
    let sha: String = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(
        sha, "4c005de496d282fc680d5c583e0d1b82b194e5112913ef473ec5328890e3601f",
        "数据不是诊断口径的尾 5 万根"
    );
    let theta = ThetaConfig::default();
    let config = theta.parse;
    let all = load_symbol(Path::new(&path), "BTCUSDT-1m", &theta, 60)
        .unwrap()
        .bars;
    assert_eq!(all.len(), 50_000);
    let bars = &all[5..]; // start_offset=5：首两根互不包含
    let layer = parse_layer(bars, &theta);
    let facts = compact_full(bars, &config);
    assert_eq!(*layer.strokes, facts.production_strokes());
    let mut fp = Sha256::new();
    for s in &facts.strokes {
        fp.update(
            format!(
                "{}:{}->{}@{}/{};",
                s.start.kind, s.start.group_anchor, s.end.group_anchor, s.start.price, s.end.price
            )
            .as_bytes(),
        );
    }
    let fingerprint: String = fp.finalize()[..8]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let mut events = BTreeMap::<&str, usize>::new();
    for p in &facts.pairs {
        *events.entry(event_of(p)).or_default() += 1;
    }
    let head: Vec<_> = facts
        .strokes
        .iter()
        .take(3)
        .map(|s| (s.start.kind, s.start.group_anchor, s.end.group_anchor))
        .collect();
    let last = facts.strokes.last().unwrap();
    let mut choice_free_pairs = 0;
    let positions = {
        let mut state = IncrFactsState::empty();
        for bar in bars {
            state.append(*bar);
        }
        state.positions().clone()
    };
    for p in facts.pairs.iter().filter(|p| p.conditions.is_some()) {
        let least = p
            .old
            .extreme_roots
            .iter()
            .flat_map(|x| p.new.extreme_roots.iter().map(move |y| (x, y)))
            .map(|(x, y)| positions[y] - positions[x] - 1)
            .min()
            .unwrap();
        assert_eq!(
            p.conditions.as_ref().unwrap().raw_between_actual_extrema,
            least
        );
        choice_free_pairs += 1;
    }
    eprintln!(
        "TB02V_TAIL50K bars={} merged={} fractals={} strokes={} segments={} fingerprint={fingerprint} events={events:?} head={head:?} last=({}, {}, {}) choice_free_pairs={choice_free_pairs} waiting={:?} policy={}",
        bars.len(),
        layer.merged_bars.len(),
        layer.fractals.len(),
        layer.strokes.len(),
        layer.segments.len(),
        last.start.kind,
        last.start.group_anchor,
        last.end.group_anchor,
        facts.waiting_reasons,
        facts.policy
    );
    assert_eq!(layer.merged_bars.len(), 30_863);
    assert_eq!(layer.fractals.len(), 14_017);
    assert_eq!(layer.strokes.len(), 4_016);
    assert!(!layer.segments.is_empty(), "段 > 0");
    assert_eq!(fingerprint, "4fd87a7e429962ac");
    assert_eq!(events["keep_eq"], 53);
    assert_eq!(events["formed"], 4_016);
    assert_eq!(events["replace"], 2_649);
    assert_eq!(events["keep"], 2_298);
    assert_eq!(events["hetero_not_formed"], 5_000);
    assert_eq!(
        head,
        vec![("TOP", 15, 26), ("BOTTOM", 26, 42), ("TOP", 42, 55)]
    );
    assert_eq!(
        (
            last.start.kind,
            last.start.group_anchor,
            last.end.group_anchor
        ),
        ("BOTTOM", 49_979, 49_996)
    );
    assert!(facts.waiting_reasons.is_empty());
    assert_eq!(
        choice_free_pairs,
        14_016 - events["keep"] - events["keep_eq"] - events["replace"]
    );

    // 前缀不回退阶梯 + full ≡ 增量：一次逐 bar 增量扫描，到每个切点与全量重建逐位对拍。
    let cuts = [
        600, 1_000, 2_000, 5_000, 10_000, 20_000, 30_000, 40_000, 49_995,
    ];
    let mut incr = IncrStrokes::empty();
    let mut previous: Option<(usize, Vec<Stroke>)> = None;
    let mut cut = cuts.iter().peekable();
    for (i, bar) in bars.iter().enumerate() {
        incr.append_bar_incremental(*bar);
        incr.update_strokes_incremental(&config);
        if cut.peek() != Some(&&(i + 1)) {
            continue;
        }
        let n = *cut.next().unwrap();
        let full = compact_full(&bars[..n], &config);
        assert_eq!(
            without_expanded_sources(incr.scan.out.clone()),
            without_expanded_sources(full.clone()),
            "切点 {n}：增量与全量同一 transition 逐位相同"
        );
        let now = parse_layer(&bars[..n], &theta).strokes.to_vec();
        assert_eq!(now, full.production_strokes(), "切点 {n}");
        assert_eq!(incr.to_result(), &now[..], "切点 {n}");
        if let Some((m, before)) = &previous {
            let k = before.len();
            assert!(now.len() >= k, "{m}→{n}：从不 pop");
            assert_eq!(
                now[..k - 1],
                before[..k - 1],
                "{m}→{n}：已确认前缀逐字段相同"
            );
            let (was, is) = (before[k - 1], now[k - 1]);
            assert_eq!(
                (was.direction, was.start_index, was.start_price),
                (is.direction, is.start_index, is.start_price),
                "{m}→{n}：末笔起点相同"
            );
            assert!(is.end_index >= was.end_index, "{m}→{n}：末笔终点只可延伸");
            if is.end_index == was.end_index {
                assert_eq!(is.end_price, was.end_price);
            } else if is.direction == Direction::Up {
                assert!(is.end_price > was.end_price, "{m}→{n}：延伸须严格更强");
            } else {
                assert!(is.end_price < was.end_price, "{m}→{n}：延伸须严格更强");
            }
            eprintln!(
                "TB02V_LADDER {m}->{n} strokes {k}->{} last_end {}->{}",
                now.len(),
                was.end_index,
                is.end_index
            );
        }
        previous = Some((n, now));
    }
    assert!(cut.next().is_none(), "九个切点全部走到");
}
