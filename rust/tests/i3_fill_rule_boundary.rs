use newchan_rust::diagnostic_fill::{compare, replay, Checkpoint, Replay, Request};
use newchan_rust::theta_v0::config::{ExecConfig, FillRule};
use serde_json::Value;
use std::process::{Command, Output};

const FIXTURE: &str = include_str!("fixtures/issue1718/boundary.json");
fn request() -> Request { serde_json::from_str(FIXTURE).unwrap() }

fn command(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_diagnostic_fill")).args(args).output().unwrap()
}

#[test]
fn cross_requires_at_least_one_tick() {
    let mut cfg = ExecConfig::default();
    cfg.fill_rule = FillRule::Cross(0);
    assert!(cfg.require_production_fill_rule().unwrap_err().contains("1 tick"));
    let mut input = request();
    input.fill_rule = FillRule::Cross(0);
    assert!(replay(input).unwrap_err().contains("1 tick"));
    cfg.fill_rule = FillRule::Cross(1);
    assert_eq!(cfg.require_production_fill_rule(), Ok(()));
}

#[test]
fn touch_diagnostic_not_acceptance_evidence() {
    let mut input = request();
    input.fill_rule = FillRule::Touch;
    let report = replay(input).unwrap();
    let rows: Vec<_> = report.fills.iter()
        .map(|f| (f.order_id.as_str(), f.bar_index, f.price_ticks, f.quantity)).collect();
    assert_eq!(rows, vec![("buy", 0, 100, 3), ("sell", 0, 110, 2)]);
    assert_eq!(report.mode, "diagnostic");
    assert_eq!(report.fill_rule, FillRule::Touch);
    assert!(report.warning.contains("过拟合"));
    let mut cfg = ExecConfig::default();
    cfg.fill_rule = FillRule::Touch;
    assert!(cfg.require_production_fill_rule().unwrap_err().contains("诊断"));
    let mut input = request();
    input.mode = "formal_acceptance".into();
    assert!(replay(input).unwrap_err().contains("DiagnosticOnly"));
}

#[test]
fn cross_failure_never_falls_back_to_touch() {
    let mut input = request();
    input.bars.truncate(1);
    let cross = replay(input.clone()).unwrap();
    assert!(cross.fills.is_empty());
    assert_eq!(cross.pending_order_ids, vec!["buy", "sell", "pending"]);
    assert_eq!(cross.fill_rule, FillRule::Cross(1));
    let comparison = compare(&input).unwrap();
    assert_eq!(comparison.cross, cross);
    assert_eq!(comparison.touch.fills.len(), 2);
    assert_eq!(replay(input).unwrap(), cross);
}

#[test]
fn one_tick_cross_emits_real_order_fills_and_open_improvement() {
    let report = replay(request()).unwrap();
    let rows: Vec<_> = report.fills.iter()
        .map(|f| (f.order_id.as_str(), f.bar_index, f.price_ticks, f.quantity)).collect();
    assert_eq!(rows, vec![("buy", 1, 99, 3), ("sell", 1, 110, 2)]);
    assert_eq!(report.pending_order_ids, vec!["pending"]);
    let comparison = compare(&request()).unwrap();
    assert_eq!(comparison.changed_order_ids, vec!["buy", "sell"]);
    let mut deeper = request();
    deeper.fill_rule = FillRule::Cross(2);
    assert!(replay(deeper.clone()).unwrap().fills.is_empty());
    assert!(compare(&deeper).unwrap().cross.fills.is_empty());
}

#[test]
fn diagnostic_replay_keeps_rule_and_version() {
    let input = request();
    let mut original = Replay::new(input.clone()).unwrap();
    original.advance_to(1).unwrap();
    let bytes = serde_json::to_vec(&original.checkpoint()).unwrap();
    let mut recovered = Replay::restore(serde_json::from_slice(&bytes).unwrap(), &input).unwrap();
    recovered.advance_to(2).unwrap();
    assert_eq!(recovered.report(), replay(input.clone()).unwrap());
    let before = recovered.report();
    recovered.advance_to(2).unwrap();
    assert_eq!(recovered.report(), before);
    assert!(recovered.advance_to(1).is_err());
    for field in ["model_id", "record_version", "market_snapshot", "fill_rule", "bars", "orders"] {
        let mut changed = serde_json::to_value(&input).unwrap();
        match field {
            "fill_rule" => changed[field] = serde_json::json!("Touch"),
            "bars" => changed[field][0]["low"] = serde_json::json!(99),
            "orders" => changed[field][0]["quantity"] = serde_json::json!(4),
            _ => changed[field] = serde_json::json!("changed/2"),
        }
        let changed: Request = serde_json::from_value(changed).unwrap();
        assert!(Replay::restore(serde_json::from_slice(&bytes).unwrap(), &changed).is_err(), "{field}");
    }
}

#[test]
fn checkpoint_preserves_prior_fills_without_duplicate_consumption() {
    let mut input = request();
    input.fill_rule = FillRule::Touch;
    let mut original = Replay::new(input.clone()).unwrap();
    original.advance_to(1).unwrap();
    assert_eq!(original.report().fills.len(), 2);
    let mut recovered = Replay::restore(original.checkpoint(), &input).unwrap();
    recovered.advance_to(2).unwrap();
    assert_eq!(recovered.report(), replay(input).unwrap());
    assert_eq!(recovered.report().fills.len(), 2);
}

#[test]
fn known_frontier_and_order_submission_are_causal() {
    let mut future = request();
    future.as_of = 1;
    assert!(replay(future).is_err());
    let mut late_order = request();
    late_order.orders[0].submitted_at = 2;
    let result = replay(late_order).unwrap();
    assert!(result.fills.iter().all(|f| f.order_id != "buy"));
    let mut invalid = request();
    invalid.bars[0].open = 111;
    assert!(replay(invalid).is_err());
    let mut unordered = request();
    unordered.bars.swap(0, 1);
    assert!(replay(unordered).is_err());
}

#[test]
fn integer_tick_boundaries_do_not_overflow() {
    let mut input = request();
    input.orders.truncate(1);
    input.orders[0].limit_ticks = i64::MAX;
    input.bars.truncate(1);
    input.bars[0].low = i64::MIN;
    input.bars[0].high = i64::MAX;
    assert_eq!(replay(input).unwrap().fills[0].price_ticks, 105);
}

#[test]
fn fill_rule_serialization_round_trip() {
    for rule in [FillRule::Cross(2), FillRule::Touch] {
        assert_eq!(serde_json::from_str::<FillRule>(&serde_json::to_string(&rule).unwrap()).unwrap(), rule);
    }
}

#[test]
fn diagnostic_cli_consumes_fixture_and_recovers_in_new_process() {
    let dir = std::env::temp_dir().join(format!("issue1718-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let input = dir.join("input.json");
    let checkpoint = dir.join("checkpoint.json");
    std::fs::write(&input, FIXTURE).unwrap();
    let input = input.to_str().unwrap();
    let checkpoint_path = checkpoint.to_str().unwrap();
    let direct = command(&["run", input]);
    assert!(direct.status.success(), "{}", String::from_utf8_lossy(&direct.stderr));
    let compare = command(&["compare", input]);
    assert!(compare.status.success());
    let diff: Value = serde_json::from_slice(&compare.stdout).unwrap();
    assert_eq!(diff["changed_order_ids"], serde_json::json!(["buy", "sell"]));
    assert_eq!(diff["cross"]["fills"][0]["bar_index"], 1);
    assert_eq!(diff["touch"]["fills"][0]["bar_index"], 0);
    let saved = command(&["checkpoint", input, "1"]);
    assert!(saved.status.success());
    let _: Checkpoint = serde_json::from_slice(&saved.stdout).unwrap();
    std::fs::write(&checkpoint, saved.stdout).unwrap();
    let recovered = command(&["resume", input, checkpoint_path]);
    assert!(recovered.status.success(), "{}", String::from_utf8_lossy(&recovered.stderr));
    assert_eq!(recovered.stdout, direct.stdout);
    let mut changed = request();
    changed.record_version = "synthetic-boundary/2".into();
    std::fs::write(input, serde_json::to_vec(&changed).unwrap()).unwrap();
    let refused = command(&["resume", input, checkpoint_path]);
    assert!(!refused.status.success());
    assert!(refused.stdout.is_empty());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("VersionMismatch"));
    std::fs::remove_dir_all(dir).unwrap();
}
