use super::{model::*, store};
use rusqlite::Connection;

pub(crate) fn model() -> Model {
    Model {
        model_id: MODEL_ID.into(),
        revision: "oracle-1".into(),
        symbol: "TEST".into(),
        quantity_unit: "TEST-UNIT".into(),
        currency: "TEST-CASH".into(),
        tick_value: 1,
        rd11_policy: "controlled-fixture-rd11".into(),
        rd12_policy: "per-leg-source-attribution/1".into(),
        projection_kind: "per_leg".into(),
        seed: "none".into(),
        fee_profile: FeeProfile {
            reference: "controlled-fee/1".into(),
            role_source: "controlled-role-map/1".into(),
            limit_role: FeeRole::Maker,
            market_role: FeeRole::Taker,
            maker: Rate {
                numerator: 1,
                denominator: 100,
                rounding: Rounding::Ceiling,
            },
            taker: Rate {
                numerator: 2,
                denominator: 100,
                rounding: Rounding::Ceiling,
            },
            market_slippage_ticks: 1,
            market_slippage_source: "controlled-oracle/1".into(),
        },
    }
}
pub(crate) fn order(id: &str, side: Side, quantity: i64, kind: OrderKind) -> Order {
    Order {
        command_id: format!("cmd-{id}"),
        family_id: id.into(),
        chong_id: id.into(),
        revision: 1,
        side,
        quantity,
        kind,
        eligibility_ref: format!("controlled-execute-{id}"),
        projection_manifest: format!("controlled-projection-{id}"),
    }
}
pub(crate) fn batch(index: i64, changes: Vec<Change>) -> Batch {
    Batch {
        request_id: format!("b{index}"),
        known_frontier_cut: format!("frontier{index}"),
        bar: Bar {
            index,
            open: 100,
            low: 90,
            high: 110,
            close: 105,
        },
        capacity: Capacity::None,
        close_changes: changes,
        scenario_busts: vec![],
    }
}
fn seed(orders: Vec<Order>) -> State {
    advance(
        &model(),
        &State::default(),
        &batch(
            0,
            orders
                .into_iter()
                .map(|order| Change::Submit { order })
                .collect(),
        ),
    )
    .unwrap()
    .next_state
}
#[test]
fn mixed_opposite_orders_use_independent_ohlc_and_named_close_slot() {
    let s = seed(vec![
        order("buy", Side::Buy, 3, OrderKind::Limit { price: 102 }),
        order("sell", Side::Sell, 2, OrderKind::Market { exec_index: 1 }),
    ]);
    let r = advance(&model(), &s, &batch(1, vec![])).unwrap();
    assert_eq!(r.fills.len(), 2);
    assert_eq!(
        (
            r.fills[0].quantity,
            r.fills[0].price,
            r.fills[0].amount,
            r.fills[0].fee
        ),
        (3, 100, 300, 3)
    );
    assert_eq!(
        (
            r.fills[1].quantity,
            r.fills[1].price,
            r.fills[1].amount,
            r.fills[1].fee
        ),
        (2, 104, 208, 5)
    );
    assert_eq!(r.fills[0].slot, "limit_interval");
    assert_eq!(r.fills[1].slot, "close");
    assert_eq!(r.intra_batch_order, "Unknown");
    assert_eq!(r.l0_fill_count, 2);
}
#[test]
fn touch_never_fills_and_open_improvement_is_symmetric() {
    let s = seed(vec![
        order("buy", Side::Buy, 3, OrderKind::Limit { price: 90 }),
        order("sell", Side::Sell, 4, OrderKind::Limit { price: 110 }),
    ]);
    let r = advance(&model(), &s, &batch(1, vec![])).unwrap();
    assert!(r.fills.is_empty());
    let mut b = batch(2, vec![]);
    b.bar.low = 89;
    b.bar.high = 111;
    let r = advance(&model(), &r.next_state, &b).unwrap();
    assert_eq!(
        r.fills.iter().map(|f| f.price).collect::<Vec<_>>(),
        vec![90, 110]
    );
    let s = seed(vec![
        order("buy", Side::Buy, 1, OrderKind::Limit { price: 102 }),
        order("sell", Side::Sell, 1, OrderKind::Limit { price: 98 }),
    ]);
    let r = advance(&model(), &s, &batch(1, vec![])).unwrap();
    assert_eq!(
        r.fills.iter().map(|f| f.price).collect::<Vec<_>>(),
        vec![100, 100]
    );
}
#[test]
fn close_cancel_does_not_erase_current_fill() {
    let s = seed(vec![order(
        "buy",
        Side::Buy,
        100,
        OrderKind::Limit { price: 100 },
    )]);
    let mut b = batch(
        1,
        vec![Change::Cancel {
            family_id: "buy".into(),
            command_id: "cancel".into(),
            eligibility_ref: "execute-cancel".into(),
        }],
    );
    b.capacity = Capacity::Scenario {
        reference: "cap-40".into(),
        buy: 40,
        sell: 0,
    };
    let r = advance(&model(), &s, &b).unwrap();
    assert_eq!(r.fills[0].quantity, 40);
    assert!(!r.next_state.families["buy"].active);
    assert!(advance(&model(), &r.next_state, &batch(2, vec![]))
        .unwrap()
        .fills
        .is_empty());
}
#[test]
fn replace_keeps_first_quantity_and_subtracts_current_cumulative_fill() {
    let original = order("buy", Side::Buy, 100, OrderKind::Limit { price: 100 });
    let s = seed(vec![original.clone()]);
    let mut replacement = original;
    replacement.command_id = "replacement".into();
    replacement.revision = 2;
    replacement.kind = OrderKind::Limit { price: 95 };
    let mut b = batch(1, vec![Change::Replace { order: replacement }]);
    b.capacity = Capacity::Scenario {
        reference: "cap-40".into(),
        buy: 40,
        sell: 0,
    };
    let r = advance(&model(), &s, &b).unwrap();
    assert_eq!(r.fills[0].price, 100);
    assert_eq!(r.fills[0].quantity, 40);
    let r = advance(&model(), &r.next_state, &batch(2, vec![])).unwrap();
    assert_eq!(r.fills[0].price, 95);
    assert_eq!(r.fills[0].quantity, 60);
    assert_eq!(r.next_state.families["buy"].filled, 100);
}
#[test]
fn shared_scenario_capacity_uses_existing_quota_policy() {
    let s = seed(vec![
        order("a", Side::Buy, 3, OrderKind::Limit { price: 100 }),
        order("b", Side::Buy, 2, OrderKind::Limit { price: 100 }),
    ]);
    let mut b = batch(1, vec![]);
    b.capacity = Capacity::Input {
        reference: "capacity-input".into(),
        buy: 3,
        sell: 0,
    };
    let r = advance(&model(), &s, &b).unwrap();
    assert_eq!(
        r.fills.iter().map(|f| f.quantity).collect::<Vec<_>>(),
        vec![2, 1]
    );
    assert_eq!(r.l0_fill_count, 0);
}
#[test]
fn failed_batch_does_not_commit_fills_or_frontier() {
    let mut c = Connection::open_in_memory().unwrap();
    store::initialize(&mut c, &model(), "x", "epoch1").unwrap();
    let first = batch(
        0,
        vec![Change::Submit {
            order: order("a", Side::Buy, 3, OrderKind::Limit { price: 100 }),
        }],
    );
    store::commit_batch(&mut c, &first).unwrap();
    let bad = batch(
        1,
        vec![Change::Cancel {
            family_id: "missing".into(),
            command_id: "c".into(),
            eligibility_ref: "e".into(),
        }],
    );
    assert!(store::commit_batch(&mut c, &bad).is_err());
    assert_eq!(
        c.query_row("SELECT count(*) FROM x_sim_originals", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    let result = store::commit_batch(&mut c, &batch(1, vec![])).unwrap();
    assert_eq!(result["originals"].as_array().unwrap().len(), 2);
}
#[test]
fn replay_is_idempotent_and_fee_has_its_own_causal_original() {
    let run = || {
        let mut c = Connection::open_in_memory().unwrap();
        store::initialize(&mut c, &model(), "x", "epoch1").unwrap();
        let first = batch(
            0,
            vec![Change::Submit {
                order: order("a", Side::Buy, 3, OrderKind::Limit { price: 100 }),
            }],
        );
        store::commit_batch(&mut c, &first).unwrap();
        let b = batch(1, vec![]);
        let r = store::commit_batch(&mut c, &b).unwrap();
        assert_eq!(r, store::commit_batch(&mut c, &b).unwrap());
        let mut changed = b;
        changed.bar.close += 1;
        assert!(store::commit_batch(&mut c, &changed)
            .unwrap_err()
            .contains("IdentityConflict"));
        assert!(store::initialize(&mut c, &model(), "venue", "epoch1")
            .unwrap_err()
            .contains("IdentityConflict"));
        r
    };
    let a = run();
    assert_eq!(a, run());
    assert_eq!(
        a["originals"][1]["event"]["cause"]["exec_id"],
        a["originals"][0]["event"]["source"]["external_fact_id"]
    );
    assert_ne!(
        a["originals"][1]["event"]["source"]["external_fact_id"],
        a["originals"][0]["event"]["source"]["external_fact_id"]
    );
}
#[test]
fn domain_errors_are_not_silent_fallbacks() {
    let mut m = model();
    m.projection_kind = "netted".into();
    assert!(m.validate().unwrap_err().contains("Unsupported"));
    let mut m = model();
    m.fee_profile.market_slippage_source.clear();
    assert!(m.validate().unwrap_err().contains("AwaitingPolicy"));
    let s = seed(vec![]);
    assert!(advance(&model(), &s, &batch(2, vec![]))
        .unwrap_err()
        .contains("连续"));
    let mut b = batch(1, vec![]);
    b.bar.low = 106;
    assert!(advance(&model(), &s, &b).is_err());
}

#[test]
fn partial_market_residual_is_explicit_not_reexecuted_in_next_bar() {
    let s = seed(vec![order(
        "market",
        Side::Buy,
        10,
        OrderKind::Market { exec_index: 1 },
    )]);
    let mut b = batch(1, vec![]);
    b.capacity = Capacity::Scenario {
        reference: "partial".into(),
        buy: 4,
        sell: 0,
    };
    let r = advance(&model(), &s, &b).unwrap();
    assert_eq!(r.fills[0].quantity, 4);
    assert_eq!(r.unfilled_market["market"], 6);
    assert!(advance(&model(), &r.next_state, &batch(2, vec![]))
        .unwrap()
        .fills
        .is_empty());
}
#[test]
fn subprocess_fixture() {
    let Ok(dir) = std::env::var("ISSUE1381_CONTROLLED_PROCESS_FIXTURE") else {
        return;
    };
    let path = std::path::Path::new(&dir);
    let db = path.join("x.sqlite");
    let mut c = Connection::open(&db).unwrap();
    store::initialize(&mut c, &model(), "controlled-x", "one").unwrap();
    let first = batch(
        0,
        vec![
            Change::Submit {
                order: order("a", Side::Buy, 3, OrderKind::Limit { price: 100 }),
            },
            Change::Submit {
                order: order("b", Side::Sell, 2, OrderKind::Market { exec_index: 1 }),
            },
        ],
    );
    store::commit_batch(&mut c, &first).unwrap();
    let mut second = batch(1, vec![]);
    second.capacity = Capacity::Scenario {
        reference: "kill-capacity-boundary".into(),
        buy: 2,
        sell: 1,
    };
    let response = store::commit_batch(&mut c, &second).unwrap();
    drop(c);
    let mut c = Connection::open(&db).unwrap();
    store::initialize(&mut c, &model(), "controlled-x", "one").unwrap();
    assert_eq!(response, store::commit_batch(&mut c, &second).unwrap());
    assert_eq!(
        c.query_row("SELECT count(*) FROM x_sim_originals", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        4
    );
    std::fs::write(
        path.join("originals.json"),
        crate::session_protocol::canonical_json(&response),
    )
    .unwrap();
}
#[test]
fn two_fresh_processes_and_restart_preserve_exact_originals() {
    let base = std::env::temp_dir().join(format!(
        "issue1381-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut results = Vec::new();
    for suffix in ["a", "b"] {
        let dir = base.join(suffix);
        std::fs::create_dir_all(&dir).unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "history_simulation::tests::subprocess_fixture",
                "--nocapture",
            ])
            .env("ISSUE1381_CONTROLLED_PROCESS_FIXTURE", &dir)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        results.push(std::fs::read(dir.join("originals.json")).unwrap());
    }
    assert_eq!(results[0], results[1]);
    std::fs::remove_dir_all(base).unwrap();
}

#[test]
fn scenario_bust_is_immutable_causal_and_never_reissues_order() {
    let mut c = Connection::open_in_memory().unwrap();
    store::initialize(&mut c, &model(), "x", "1").unwrap();
    store::commit_batch(
        &mut c,
        &batch(
            0,
            vec![Change::Submit {
                order: order("a", Side::Buy, 3, OrderKind::Limit { price: 100 }),
            }],
        ),
    )
    .unwrap();
    let filled = store::commit_batch(&mut c, &batch(1, vec![])).unwrap();
    let exec = filled["originals"][0]["event"]["source"]["external_fact_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut b = batch(2, vec![]);
    b.capacity = Capacity::Scenario {
        reference: "scenario-bust".into(),
        buy: 0,
        sell: 0,
    };
    b.scenario_busts.push(Bust {
        exec_id: exec.clone(),
        quantity: 2,
        scenario_ref: "scenario-bust".into(),
    });
    let busted = store::commit_batch(&mut c, &b).unwrap();
    assert_eq!(busted["originals"][0]["event"]["cause"]["exec_id"], exec);
    assert_eq!(
        store::read_original(&c, &exec).unwrap(),
        filled["originals"][0]
    );
    assert_eq!(busted, store::commit_batch(&mut c, &b).unwrap());
    b.request_id = "b3".into();
    b.bar.index = 3;
    assert!(store::commit_batch(&mut c, &b)
        .unwrap_err()
        .contains("bust 超出"));
    assert!(
        store::commit_batch(&mut c, &batch(3, vec![])).unwrap()["originals"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
#[test]
fn killed_transaction_recovers_without_duplicate_originals() {
    let dir = std::env::temp_dir().join(format!("issue1381-kill-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let invoke = |crash| {
        let mut c = std::process::Command::new(std::env::current_exe().unwrap());
        c.args([
            "--exact",
            "history_simulation::tests::subprocess_fixture",
            "--nocapture",
        ])
        .env("ISSUE1381_CONTROLLED_PROCESS_FIXTURE", &dir);
        if crash {
            c.env("ISSUE1381_TEST_CRASH_BEFORE_COMMIT", "1");
        }
        c.output().unwrap()
    };
    assert_eq!(invoke(true).status.code(), Some(42));
    let c = Connection::open(dir.join("x.sqlite")).unwrap();
    assert_eq!(
        c.query_row("SELECT count(*) FROM x_sim_originals", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    let raw: String = c
        .query_row("SELECT v FROM x_sim_meta WHERE k='state'", [], |r| r.get(0))
        .unwrap();
    let before: State = serde_json::from_str(&raw).unwrap();
    assert!(before.capacity_state.epochs.is_empty());
    drop(c);
    assert!(invoke(false).status.success());
    let c = Connection::open(dir.join("x.sqlite")).unwrap();
    assert_eq!(
        c.query_row("SELECT count(*) FROM x_sim_originals", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        4
    );
    let raw: String = c
        .query_row("SELECT v FROM x_sim_meta WHERE k='state'", [], |r| r.get(0))
        .unwrap();
    let after: State = serde_json::from_str(&raw).unwrap();
    assert_eq!(after.capacity_state.epochs.len(), 2);
    drop(c);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn fee_role_is_the_explicit_frozen_contract_not_order_type() {
    let state = seed(vec![
        order("buy", Side::Buy, 3, OrderKind::Limit { price: 102 }),
        order("sell", Side::Sell, 2, OrderKind::Market { exec_index: 1 }),
    ]);
    let mut m = model();
    m.fee_profile.limit_role = FeeRole::Taker;
    m.fee_profile.market_role = FeeRole::Maker;
    let r = advance(&m, &state, &batch(1, vec![])).unwrap();
    assert_eq!((r.fills[0].role.as_str(), r.fills[0].fee), ("taker", 6));
    assert_eq!((r.fills[1].role.as_str(), r.fills[1].fee), ("maker", 3));
    m.rd12_policy = "withdrawn-policy/1".into();
    assert!(advance(&m, &state, &batch(1, vec![]))
        .unwrap_err()
        .starts_with("Unsupported"));
}

fn limited(index: i64, changes: Vec<Change>) -> Batch {
    let mut b = batch(index, changes);
    b.capacity = Capacity::Scenario {
        reference: format!("capacity-source-{index}"),
        buy: 1,
        sell: 0,
    };
    b
}
fn family(result: &ResultBatch) -> &str {
    &result.fills[0].family_id
}
#[test]
fn capacity_epoch_uses_frozen_cumulative_difference() {
    let s = seed(vec![
        order("a", Side::Buy, 1, OrderKind::Limit { price: 100 }),
        order("b", Side::Buy, 2, OrderKind::Limit { price: 100 }),
    ]);
    let one = advance(&model(), &s, &limited(1, vec![])).unwrap();
    assert_eq!(family(&one), "b");
    let two = advance(&model(), &one.next_state, &limited(2, vec![])).unwrap();
    assert_eq!(family(&two), "b");
    assert_eq!(one.fills[0].capacity_epoch, two.fills[0].capacity_epoch);
    let three = advance(&model(), &two.next_state, &limited(3, vec![])).unwrap();
    assert_eq!(family(&three), "a");
    assert!(three.next_state.capacity_state.active.is_empty());
}
#[test]
fn capacity_epoch_switches_for_full_bar_eligibility_not_ref_or_remaining() {
    let s = seed(vec![
        order("a", Side::Buy, 1, OrderKind::Limit { price: 100 }),
        order("b", Side::Buy, 2, OrderKind::Limit { price: 95 }),
    ]);
    let one = advance(&model(), &s, &limited(1, vec![])).unwrap();
    assert_eq!(family(&one), "b");
    let mut b = limited(2, vec![]);
    b.bar.low = 96;
    let two = advance(&model(), &one.next_state, &b).unwrap();
    assert_eq!(family(&two), "a");
    assert_ne!(one.fills[0].capacity_epoch, two.fills[0].capacity_epoch);
    let three = advance(&model(), &two.next_state, &limited(3, vec![])).unwrap();
    assert_eq!((family(&three), three.fills[0].quantity), ("b", 1));
}
#[test]
fn completed_members_stay_in_frozen_vector_and_ignore_price_change() {
    let s = seed(vec![
        order("a", Side::Buy, 1, OrderKind::Limit { price: 100 }),
        order("b", Side::Buy, 2, OrderKind::Limit { price: 100 }),
        order("c", Side::Buy, 3, OrderKind::Limit { price: 100 }),
    ]);
    let mut state = s;
    let mut sequence = Vec::new();
    let mut epochs = Vec::new();
    for index in 1..=5 {
        let changes = if index == 4 {
            let mut o = state.families["b"].order.clone();
            o.command_id = "reprice-b".into();
            o.revision = 2;
            o.kind = OrderKind::Limit { price: 90 };
            vec![Change::Replace { order: o }]
        } else {
            vec![]
        };
        let mut b = limited(index, changes);
        if index == 5 {
            b.bar.low = 95;
        }
        let r = advance(&model(), &state, &b).unwrap();
        sequence.push(family(&r).to_owned());
        epochs.push(r.fills[0].capacity_epoch.clone());
        state = r.next_state;
    }
    assert_eq!(sequence, vec!["c", "b", "c", "b", "c"]);
    assert!(epochs.iter().all(|id| id == &epochs[0]));
}
#[test]
fn added_and_cancelled_members_get_explicit_next_bar_epochs() {
    let s = seed(vec![
        order("a", Side::Buy, 1, OrderKind::Limit { price: 100 }),
        order("b", Side::Buy, 2, OrderKind::Limit { price: 100 }),
    ]);
    let one = advance(
        &model(),
        &s,
        &limited(
            1,
            vec![Change::Submit {
                order: order("c", Side::Buy, 1, OrderKind::Limit { price: 100 }),
            }],
        ),
    )
    .unwrap();
    assert_eq!(family(&one), "b");
    let two = advance(&model(), &one.next_state, &limited(2, vec![])).unwrap();
    assert_eq!(family(&two), "a");
    let manifest = &two.capacity_evidence[0]["manifest"];
    assert_eq!(manifest["participants"].as_array().unwrap().len(), 3);
    assert!(manifest["participants"]
        .as_array()
        .unwrap()
        .iter()
        .all(|p| p["requested_q"] == 1));
    let s = seed(vec![
        order("a", Side::Buy, 3, OrderKind::Limit { price: 100 }),
        order("b", Side::Buy, 2, OrderKind::Limit { price: 100 }),
    ]);
    let cancel = |id: &str| Change::Cancel {
        family_id: id.into(),
        command_id: format!("cancel-{id}"),
        eligibility_ref: format!("eligible-{id}"),
    };
    let one = advance(&model(), &s, &limited(1, vec![cancel("b")])).unwrap();
    assert_eq!(family(&one), "a");
    let two = advance(&model(), &one.next_state, &limited(2, vec![cancel("a")])).unwrap();
    assert_eq!(family(&two), "a");
    let empty = advance(&model(), &two.next_state, &limited(3, vec![])).unwrap();
    assert!(empty.fills.is_empty());
    assert!(empty.next_state.capacity_state.active.is_empty());
    assert_eq!(
        empty.capacity_transitions[0]["successor"],
        serde_json::Value::Null
    );
}
#[test]
fn pure_reprice_and_close_arrival_permutations_do_not_reset_or_reorder() {
    let a = order("a", Side::Buy, 1, OrderKind::Limit { price: 100 });
    let b = order("b", Side::Buy, 2, OrderKind::Limit { price: 100 });
    let x = batch(
        0,
        vec![
            Change::Submit { order: a.clone() },
            Change::Submit { order: b.clone() },
        ],
    );
    let mut y = x.clone();
    y.close_changes.reverse();
    let sa = advance(&model(), &State::default(), &x).unwrap().next_state;
    let sb = advance(&model(), &State::default(), &y).unwrap().next_state;
    assert_eq!(sa, sb);
    let mut replaced = b;
    replaced.command_id = "reprice".into();
    replaced.revision = 2;
    replaced.kind = OrderKind::Limit { price: 99 };
    let one = advance(
        &model(),
        &sa,
        &limited(1, vec![Change::Replace { order: replaced }]),
    )
    .unwrap();
    let two = advance(&model(), &one.next_state, &limited(2, vec![])).unwrap();
    assert_eq!(family(&two), "b");
    assert_eq!(one.fills[0].capacity_epoch, two.fills[0].capacity_epoch);
    assert_eq!(two.fills[0].command_id, "reprice");
}
#[test]
fn capacity_empty_frontier_reopens_and_corrupt_cumulative_is_refused() {
    let s = seed(vec![
        order("a", Side::Buy, 1, OrderKind::Limit { price: 100 }),
        order("b", Side::Buy, 2, OrderKind::Limit { price: 100 }),
    ]);
    let one = advance(&model(), &s, &limited(1, vec![])).unwrap();
    let mut corrupt = one.next_state.clone();
    corrupt
        .capacity_state
        .epochs
        .values_mut()
        .next()
        .unwrap()
        .cumulative = 0;
    assert!(advance(&model(), &corrupt, &limited(2, vec![]))
        .unwrap_err()
        .contains("累计"));
    let mut empty = limited(2, vec![]);
    empty.bar.low = 100;
    let nofill = advance(&model(), &one.next_state, &empty).unwrap();
    assert!(nofill.fills.is_empty());
    assert!(nofill.next_state.capacity_state.active.is_empty());
    let reopened = advance(&model(), &nofill.next_state, &limited(3, vec![])).unwrap();
    assert_eq!(family(&reopened), "a");
    assert_ne!(
        one.fills[0].capacity_epoch,
        reopened.fills[0].capacity_epoch
    );
}

#[test]
fn capacity_budget_is_shared_by_ordered_slots_and_independent_directions() {
    let orders = vec![
        order("bl", Side::Buy, 1, OrderKind::Limit { price: 100 }),
        order("bm", Side::Buy, 2, OrderKind::Market { exec_index: 1 }),
        order("sl", Side::Sell, 2, OrderKind::Limit { price: 100 }),
        order("sm", Side::Sell, 2, OrderKind::Market { exec_index: 1 }),
    ];
    let state = seed(orders.clone());
    let mut reversed = orders;
    reversed.reverse();
    let reversed = seed(reversed);
    let mut b = limited(1, vec![]);
    b.capacity = Capacity::Input {
        reference: "one-directional-budget".into(),
        buy: 2,
        sell: 3,
    };
    let r = advance(&model(), &state, &b).unwrap();
    assert_eq!(r, advance(&model(), &reversed, &b).unwrap());
    let fills: std::collections::BTreeMap<_, _> = r
        .fills
        .iter()
        .map(|f| (f.family_id.as_str(), f.quantity))
        .collect();
    assert_eq!(fills["bl"], 1);
    assert_eq!(fills["bm"], 1);
    assert_eq!(fills["sl"], 2);
    assert_eq!(fills["sm"], 1);
    assert_eq!(
        r.fills
            .iter()
            .filter(|f| f.side == Side::Buy)
            .map(|f| f.quantity)
            .sum::<i64>(),
        2
    );
    assert_eq!(
        r.fills
            .iter()
            .filter(|f| f.side == Side::Sell)
            .map(|f| f.quantity)
            .sum::<i64>(),
        3
    );
}

#[test]
fn cross_epoch_bust_and_fee_keep_original_projection_and_capacity_without_reopening() {
    let mut c = Connection::open_in_memory().unwrap();
    store::initialize(&mut c, &model(), "x", "1").unwrap();
    store::commit_batch(
        &mut c,
        &batch(
            0,
            vec![
                Change::Submit {
                    order: order("a", Side::Buy, 1, OrderKind::Limit { price: 100 }),
                },
                Change::Submit {
                    order: order("b", Side::Buy, 2, OrderKind::Limit { price: 100 }),
                },
            ],
        ),
    )
    .unwrap();
    let first = store::commit_batch(
        &mut c,
        &limited(
            1,
            vec![Change::Submit {
                order: order("c", Side::Buy, 1, OrderKind::Limit { price: 100 }),
            }],
        ),
    )
    .unwrap();
    let second = store::commit_batch(&mut c, &limited(2, vec![])).unwrap();
    let fill = &first["originals"][0];
    let fee = &first["originals"][1];
    let exec = fill["event"]["fill"]["exec_id"].as_str().unwrap();
    assert_ne!(
        fill["event"]["fill"]["capacity_epoch"],
        second["originals"][0]["event"]["fill"]["capacity_epoch"]
    );
    let mut b = limited(3, vec![]);
    b.capacity = Capacity::Scenario {
        reference: "late-bust".into(),
        buy: 0,
        sell: 0,
    };
    b.scenario_busts.push(Bust {
        exec_id: exec.into(),
        quantity: 1,
        scenario_ref: "late-bust".into(),
    });
    let result = store::commit_batch(&mut c, &b).unwrap();
    let bust = &result["originals"][0]["event"];
    assert_eq!(
        bust["capacity_epoch"],
        fill["event"]["fill"]["capacity_epoch"]
    );
    assert_eq!(
        bust["projection_manifest"],
        fill["event"]["fill"]["projection_manifest"]
    );
    assert_eq!(store::read_original(&c, exec).unwrap(), *fill);
    assert_eq!(
        store::read_original(
            &c,
            fee["event"]["source"]["external_fact_id"].as_str().unwrap()
        )
        .unwrap(),
        *fee
    );
    let state: String = c
        .query_row("SELECT v FROM x_sim_meta WHERE k='state'", [], |r| r.get(0))
        .unwrap();
    let state: State = serde_json::from_str(&state).unwrap();
    assert_eq!(state.families["b"].filled, 1);
}

#[test]
fn persisted_missing_capacity_frontier_refuses_without_consuming_next_bar() {
    let mut c = Connection::open_in_memory().unwrap();
    store::initialize(&mut c, &model(), "x", "1").unwrap();
    store::commit_batch(
        &mut c,
        &batch(
            0,
            vec![
                Change::Submit {
                    order: order("a", Side::Buy, 1, OrderKind::Limit { price: 100 }),
                },
                Change::Submit {
                    order: order("b", Side::Buy, 2, OrderKind::Limit { price: 100 }),
                },
            ],
        ),
    )
    .unwrap();
    store::commit_batch(&mut c, &limited(1, vec![])).unwrap();
    let raw: String = c
        .query_row("SELECT v FROM x_sim_meta WHERE k='state'", [], |r| r.get(0))
        .unwrap();
    let mut damaged: State = serde_json::from_str(&raw).unwrap();
    damaged.capacity_state = Default::default();
    c.execute(
        "UPDATE x_sim_meta SET v=?1 WHERE k='state'",
        [serde_json::to_string(&damaged).unwrap()],
    )
    .unwrap();
    assert!(store::audit_state(&c).unwrap_err().contains("前沿"));
    assert!(store::commit_batch(&mut c, &limited(2, vec![]))
        .unwrap_err()
        .contains("前沿"));
    assert_eq!(
        c.query_row(
            "SELECT count(*) FROM x_sim_batches WHERE id='b2'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    c.execute("UPDATE x_sim_meta SET v=?1 WHERE k='state'", [raw])
        .unwrap();
    let legal = store::commit_batch(&mut c, &limited(2, vec![])).unwrap();
    assert_eq!(legal["originals"][0]["event"]["fill"]["family_id"], "b");
    let mut subset = serde_json::to_value(limited(3, vec![])).unwrap();
    subset["capacity_participants"] = serde_json::json!(["a"]);
    assert!(serde_json::from_value::<Batch>(subset).is_err());
}

#[test]
fn runtime_process_fixture() {
    let Ok(db) = std::env::var("ISSUE1381_FIXTURE_DB") else {
        return;
    };
    let profile =
        std::fs::read_to_string(std::env::var("ISSUE1381_FIXTURE_PROFILE").unwrap()).unwrap();
    let profile: serde_json::Value = serde_json::from_str(&profile).unwrap();
    let p: super::runtime::Profile =
        serde_json::from_value(profile["history_simulation"].clone()).unwrap();
    let mut c = Connection::open(db).unwrap();
    c.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")
        .unwrap();
    let epoch: i64 = std::env::var("ISSUE1381_FIXTURE_EPOCH")
        .unwrap_or_else(|_| "1".into())
        .parse()
        .unwrap();
    super::runtime::validate_existing(&c, &p, epoch).unwrap();
    super::runtime::serve(&mut c, &p, epoch).unwrap();
}

#[test]
fn runtime_pending_claim_crash_and_authority_reply_loss_recover_without_reauthorization() {
    let dir = std::env::temp_dir().join(format!("issue1381-runtime-{}", std::process::id()));
    let output = std::process::Command::new("python3")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/history_simulation_entry.py"
        ))
        .arg(std::env::current_exe().unwrap())
        .arg(&dir)
        .arg("runtime-fixture")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stdout={}\nstderr={}\nevidence={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        dir.display()
    );
    println!("{}", String::from_utf8_lossy(&output.stdout));
    std::fs::remove_dir_all(dir).unwrap();
}
