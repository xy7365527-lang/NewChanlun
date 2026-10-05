// #1467 Stage60: run the unchanged R_W-v1 on each concrete input prefix.
#[path = "/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/strict_point_reference.rs"]
mod strict_point_reference;
use newchan_rust::theta_v0::types::{Bar, Direction, Stroke};
use serde_json::{json, Value};
fn sj(s: &Stroke) -> Value {
    json!({"up":s.direction==Direction::Up,"start":s.start_index,"end":s.end_index,
      "start_price":s.start_price,"end_price":s.end_price})
}
fn main() {
    let path = std::env::args().nth(1).expect("input JSON");
    let input: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let bars: Vec<Bar> = input["observations"].as_array().unwrap().iter().enumerate().map(|(i,p)| {
        let price=p["price"].as_i64().unwrap();
        Bar{source_index:i,timestamp:p["time"].as_i64().unwrap(),open:price,high:price,low:price,
          close:price,volume:0.0,untradable:true}
    }).collect();
    let whole = strict_point_reference::build(&bars);
    let prefixes:Vec<Value>=(0..bars.len()).map(|cut| {
        let r=strict_point_reference::build(&bars[..=cut]);
        let count=r.strokes.len().saturating_sub(1);
        json!({"cut":cut,"stable_count":count,
          "stable":r.strokes[..count].iter().map(sj).collect::<Vec<_>>(),
          "active":r.strokes.last().map(sj),
          "anchors":r.anchors.iter().map(|x|json!({"raw":x.raw,"price":x.price,"top":x.top,"known_at":x.known_at})).collect::<Vec<_>>()})
    }).collect();
    println!("{}",serde_json::to_string_pretty(&json!({"groups":whole.groups.len(),
      "strokes":whole.strokes.iter().map(sj).collect::<Vec<_>>(),"prefixes":prefixes})).unwrap());
}
