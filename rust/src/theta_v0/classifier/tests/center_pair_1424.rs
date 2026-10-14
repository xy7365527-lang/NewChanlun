//! #1424 TB-02-F：相邻中枢五叶关系（CC-020）与延伸测试单元位置（CC-018）的工作例锁。
//!
//! 期待值取自独立 oracle `.chanlun/review-results/issue1424-tb02f-20261014/oracle_pairs.py` 的输出
//! `rust/tests/fixtures/center_pair_1424_worked_examples.json`（纯标准库手算，不导入项目代码；备料
//! `tb1424-prep-r2` 同一算法），不从被测函数推出。W0–W12 的含义见夹具 `note` 字段：
//! W3/W4 是 R26 判别例（Zn 外缘判新生、全单元外缘判扩展）；W5 外缘恰等；W6/W11 核心相切/重叠；W7 外缘包含与
//! #1478 端点条款下的连接段；W8 两单元连接段；W9 ≥9 窗口一端待定；W10 相切位置；W12 B 开放期的关系修订。

use serde_json::Value;

use super::super::super::types::{Center, Direction, Tick};
use super::super::center::{
    center_from_segments, center_pair_facts, classify_relation, PairEndpoint, PairLeaf, UnitRange,
};
use super::super::center_scan::{scan_center_facts_full, CenterScanFacts, PairOutcome};
use super::super::recursive_tower::detect_centers_windowed_resume;

const FIXTURE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/center_pair_1424_worked_examples.json"
));

fn doc() -> Value {
    serde_json::from_str(FIXTURE).expect("夹具是 JSON")
}

/// 端点相连、方向由相邻端点决定的 L0 单元序列（同 `center_scan_1419.rs`）。
fn units_from_endpoints(p: &[Tick]) -> Vec<UnitRange> {
    p.windows(2)
        .enumerate()
        .map(|(k, w)| UnitRange {
            start_index: k * 4,
            end_index: k * 4 + 4,
            direction: if w[1] > w[0] {
                Direction::Up
            } else {
                Direction::Down
            },
            lo: w[0].min(w[1]),
            hi: w[0].max(w[1]),
        })
        .collect()
}

fn endpoints(st: &Value) -> Vec<Tick> {
    st["endpoints"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_i64().unwrap())
        .collect()
}

fn ints(v: &Value) -> Vec<usize> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_u64().unwrap() as usize)
        .collect()
}

fn tick(v: &Value) -> Option<Tick> {
    v.as_i64()
}

fn ord_str(o: std::cmp::Ordering) -> &'static str {
    match o {
        std::cmp::Ordering::Less => "<",
        std::cmp::Ordering::Equal => "=",
        std::cmp::Ordering::Greater => ">",
    }
}

/// 一条流（或流的一个切面）的中枢、位置与对，逐项对夹具。返回核对项数。
fn check_stream(st: &Value) -> usize {
    let name = st["name"].as_str().unwrap();
    let units = units_from_endpoints(&endpoints(st));
    let f: CenterScanFacts = scan_center_facts_full(&units, center_from_segments);
    let wf = f.window_facts(&units);
    let positions = f.tested_positions(&units);
    let centers = st["centers"].as_array().unwrap();
    assert_eq!(wf.len(), centers.len(), "{name}: 中枢数");
    let mut n = 0;
    for (w, (got, want)) in wf.iter().zip(centers).enumerate() {
        let ctx = format!("{name} C{w}");
        assert_eq!(
            (got.first, got.last, got.settled, got.break_unit),
            (
                want["first"].as_u64().unwrap() as usize,
                want["last"].as_u64().unwrap() as usize,
                want["settled"].as_bool().unwrap(),
                want["break_unit"].as_u64().map(|x| x as usize)
            ),
            "{ctx}"
        );
        assert_eq!(
            got.upgrade_pending,
            want["status"] == "UPGRADE_PENDING_TB02H",
            "{ctx}"
        );
        assert_eq!(got.zn_members, ints(&want["zn_members"]), "{ctx}: Zn");
        let c = f.window_center(&units, w);
        assert_eq!(
            (Some(c.zd), Some(c.zg)),
            (tick(&want["ZD"]), tick(&want["ZG"])),
            "{ctx}: 核心"
        );
        if !got.upgrade_pending {
            assert_eq!(
                (Some(c.dd), Some(c.gg)),
                (tick(&want["DD"]), tick(&want["GG"])),
                "{ctx}: Zn 外缘"
            );
        }
        let tested = want["tested"].as_array().unwrap();
        assert_eq!(positions[w].len(), tested.len(), "{ctx}: 延伸测试单元数");
        for (p, t) in positions[w].iter().zip(tested) {
            assert_eq!(
                (p.unit, p.position.as_str(), p.role.as_str()),
                (
                    t["unit"].as_u64().unwrap() as usize,
                    t["position"].as_str().unwrap(),
                    t["role"].as_str().unwrap()
                ),
                "{ctx}: CC-018"
            );
            n += 1;
        }
        n += 1;
    }
    let pairs = f.pair_facts(&units);
    let want_pairs = st["pairs"].as_array().unwrap();
    assert_eq!(pairs.len(), want_pairs.len(), "{name}: 对数 = 中枢数 − 1");
    for (p, want) in pairs.iter().zip(want_pairs) {
        let ctx = format!("{name} C{}→C{}", p.left, p.right);
        assert_eq!(
            p.connectors.clone().collect::<Vec<_>>(),
            ints(&want["connectors"]),
            "{ctx}: 连接段"
        );
        match p.outcome {
            PairOutcome::PendingUpgrade => {
                assert_eq!(want["status"], "PENDING_UPGRADE_TB02H", "{ctx}");
                assert!(want["leaf"].is_null() && want["matrix"].is_null(), "{ctx}");
            }
            PairOutcome::Evaluated(facts) => {
                assert_eq!(want["status"], "EVALUATED", "{ctx}");
                assert_eq!(
                    facts.leaf.as_str(),
                    want["leaf"].as_str().unwrap(),
                    "{ctx}: 叶"
                );
                assert_eq!(
                    facts.leaf.direction().map(|d| match d {
                        Direction::Up => "UP",
                        Direction::Down => "DOWN",
                    }),
                    want["direction"].as_str(),
                    "{ctx}: 方向"
                );
                for x in PairEndpoint::ALL {
                    for y in PairEndpoint::ALL {
                        let key = format!("B.{}?A.{}", x.as_str(), y.as_str());
                        assert_eq!(
                            ord_str(facts.cmp(x, y)),
                            want["matrix"][&key].as_str().unwrap(),
                            "{ctx}: {key}"
                        );
                    }
                }
                let wit = &want["witnesses"];
                assert_eq!(
                    facts.core_tangent(),
                    wit["core_tangent"].as_bool().unwrap(),
                    "{ctx}"
                );
                assert_eq!(
                    facts.outer_touch(),
                    wit["outer_touch"].as_bool().unwrap(),
                    "{ctx}"
                );
                assert_eq!(
                    facts.outer_containment().as_str(),
                    wit["outer_containment"].as_str().unwrap(),
                    "{ctx}"
                );
                assert_eq!(
                    facts.core_containment().as_str(),
                    wit["core_containment"].as_str().unwrap(),
                    "{ctx}"
                );
                // 四态投影 = 塔 `classify_relation` 在同一对中枢上的读数（#804 L1）。
                let (a, b) = (
                    f.window_center(&units, p.left),
                    f.window_center(&units, p.right),
                );
                assert_eq!(
                    format!("{:?}", classify_relation(&a, &b)),
                    want["relation4"].as_str().unwrap(),
                    "{ctx}: 四态"
                );
                // R26：外缘换回全单元时的叶（判别例 W3/W4 与 Zn 口径不同）。
                let full = |w: usize| {
                    let win = &f.windows[w];
                    let m = &units[win.seed_at..win.exit];
                    let c = f.window_center(&units, w);
                    Center {
                        dd: m.iter().map(|u| u.lo).min().unwrap(),
                        gg: m.iter().map(|u| u.hi).max().unwrap(),
                        ..c
                    }
                };
                assert_eq!(
                    center_pair_facts(&full(p.left), &full(p.right))
                        .leaf
                        .as_str(),
                    want["leaf_if_full_unit_outer"].as_str().unwrap(),
                    "{ctx}: 全单元外缘对照"
                );
            }
        }
        n += 1;
    }
    n
}

/// W0–W11 全部流：中枢、CC-018 位置与角色、CC-020 对（叶、方向、16 格矩阵、证据、连接段、四态投影、全单元对照）。
#[test]
fn s1424_worked_examples_streams_match_oracle() {
    let d = doc();
    assert_eq!(d["schema"], "tb1424-worked-examples/1");
    let streams = d["streams"].as_array().unwrap();
    assert_eq!(streams.len(), 12, "W0–W11");
    let n: usize = streams.iter().map(check_stream).sum();
    assert!(n > 80, "核对项 {n}");
}

/// W12：同一流三个切面，B 开放期关系由 DOWN_NEWBIRTH 修订为 DOWN_EXPANSION（A-ST-047-05）。
#[test]
fn s1424_w12_leaf_revision_across_cuts() {
    let d = doc();
    let series = &d["series"][0];
    assert_eq!(series["name"], "W12_LEAF_REVISION");
    let mut leaves = vec![];
    for cut in series["cuts"].as_array().unwrap() {
        check_stream(cut);
        leaves.push(cut["pairs"][0]["leaf"].as_str().unwrap().to_string());
    }
    assert_eq!(leaves, ["DOWN_NEWBIRTH", "DOWN_NEWBIRTH", "DOWN_EXPANSION"]);
}

/// R26 判别例（#1478 回填评论）：同一输入按 Zn 外缘判新生、按全单元外缘判扩展；外缘换回全单元的变异体必红。
#[test]
fn s1424_r26_zn_outer_discriminates_newbirth_from_expansion() {
    let d = doc();
    for (name, zn, full) in [
        (
            "W3_DISC_DOWN_NEWBIRTH",
            PairLeaf::DownNewbirth,
            "DOWN_EXPANSION",
        ),
        ("W4_DISC_UP_NEWBIRTH", PairLeaf::UpNewbirth, "UP_EXPANSION"),
    ] {
        let st = d["streams"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["name"] == name)
            .unwrap();
        let units = units_from_endpoints(&endpoints(st));
        let f = scan_center_facts_full(&units, center_from_segments);
        match f.pair_facts(&units)[0].outcome {
            PairOutcome::Evaluated(facts) => assert_eq!(facts.leaf, zn, "{name}"),
            PairOutcome::PendingUpgrade => panic!("{name}: 不应待定"),
        }
        assert_eq!(st["pairs"][0]["leaf_if_full_unit_outer"], full, "{name}");
    }
}

/// #804 L2：塔（`detect_centers_windowed_resume`，NoopSink）相邻中枢的四态 = 夹具；两端都不是 ≥9 子中枢的对，
/// 塔的四态 = S 对的叶投影。W9：S 待定，塔在该处给出过渡态子中枢关系（§9.5），S 不照抄。
#[test]
fn s1424_tower_pairs_same_judgment_and_ge9_not_copied() {
    let d = doc();
    let mut compared = 0;
    for st in d["streams"].as_array().unwrap() {
        let name = st["name"].as_str().unwrap();
        let units = units_from_endpoints(&endpoints(st));
        let (tower, _, _) = detect_centers_windowed_resume(&units, center_from_segments, 0);
        let want = st["tower_pairs"].as_array().unwrap();
        assert_eq!(tower.len(), want.len() + 1, "{name}: 塔中枢数");
        for (pair, w) in tower.windows(2).zip(want) {
            let ((a, aw), (b, bw)) = (&pair[0], &pair[1]);
            assert_eq!(
                (vec![aw.0, aw.1], vec![bw.0, bw.1]),
                (ints(&w["a_win"]), ints(&w["b_win"])),
                "{name}"
            );
            assert_eq!(
                format!("{:?}", classify_relation(a, b)),
                w["relation4"].as_str().unwrap(),
                "{name}: 塔四态"
            );
            assert_eq!(
                center_pair_facts(a, b).leaf.as_str(),
                w["leaf"].as_str().unwrap(),
                "{name}: 塔五叶"
            );
        }
        let f = scan_center_facts_full(&units, center_from_segments);
        for p in f.pair_facts(&units) {
            let (wa, wb) = (&f.windows[p.left], &f.windows[p.right]);
            if wa.upgraded || wb.upgraded {
                assert_eq!(p.outcome, PairOutcome::PendingUpgrade, "{name}");
                continue;
            }
            let ta = tower
                .iter()
                .find(|(_, w)| w.0 == wa.seed_at)
                .expect("塔有同窗口中枢")
                .0;
            let tb = tower.iter().find(|(_, w)| w.0 == wb.seed_at).unwrap().0;
            match p.outcome {
                PairOutcome::Evaluated(facts) => {
                    assert_eq!(facts.leaf.relation(), classify_relation(&ta, &tb), "{name}");
                    assert_eq!(facts, center_pair_facts(&ta, &tb), "{name}: 同输入同输出");
                    compared += 1;
                }
                PairOutcome::PendingUpgrade => panic!("{name}: <9 不应待定"),
            }
        }
    }
    assert_eq!(compared, 11, "W0–W11 中可判对数");
    // W9：塔末子中枢 (u9..u11) → B 给出 DOWN_EXPANSION（子窗全单元外缘 DD=5），而整窗按 Zn 取外缘（DD=10）会是
    // DOWN_NEWBIRTH——三种读数不同，S 记待定。
    let w9 = d["streams"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["name"] == "W9_UPGRADE_THEN_B")
        .unwrap();
    assert_eq!(w9["pairs"][0]["status"], "PENDING_UPGRADE_TB02H");
    assert_eq!(w9["tower_pairs"][3]["leaf"], "DOWN_EXPANSION");
    let whole_zn_a = Center {
        zd: 20,
        zg: 35,
        dd: 10,
        gg: 40,
        start_index: 0,
        end_index: 0,
    };
    let b = Center {
        zd: 3,
        zg: 8,
        dd: 2,
        gg: 9,
        start_index: 0,
        end_index: 0,
    };
    assert_eq!(
        center_pair_facts(&whole_zn_a, &b).leaf,
        PairLeaf::DownNewbirth
    );
}
