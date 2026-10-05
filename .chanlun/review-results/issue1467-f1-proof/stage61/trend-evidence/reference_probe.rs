// #1467 Stage61: compact, exact readback of unchanged R_W-v1 on every prefix.
#[path = "../../strict_point_reference.rs"]
mod strict_point_reference;
use newchan_rust::theta_v0::types::{Bar, Direction, Stroke};
use serde_json::{json, Value};
fn sj(s: &Stroke) -> Value {
    json!({"up":s.direction==Direction::Up,"start":s.start_index,"end":s.end_index,
      "start_price":s.start_price,"end_price":s.end_price})
}
fn main() {
    let file=std::env::args().nth(1).expect("input JSON");
    let input:Value=serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
    let bars:Vec<Bar>=input["observations"].as_array().unwrap().iter().enumerate().map(|(i,p)|{
        let price=p["price"].as_i64().unwrap();
        Bar {source_index:i,timestamp:p["time"].as_i64().unwrap(),open:price,high:price,
            low:price,close:price,volume:0.0,untradable:true}
    }).collect();
    let whole=strict_point_reference::build(&bars);
    let full:Vec<Value>=whole.strokes.iter().map(sj).collect();
    let mut prefixes=Vec::new();let mut certificates=Vec::new();
    for cut in 0..bars.len() {
        let r=strict_point_reference::build(&bars[..=cut]);
        let n=r.strokes.len().saturating_sub(1);
        let stable:Vec<Value>=r.strokes[..n].iter().map(sj).collect();
        assert_eq!(stable,full[..n],"stable source mismatch at cut {cut}");
        prefixes.push(json!({"cut":cut,"stable_count":n,"stable_equals_final_prefix":true,
            "active":r.strokes.last().map(sj)}));
        if cut>=38 && (cut-38)%20==0 {
            certificates.push(json!({"cut":cut,"stable":stable}));
        }
    }
    println!("{}",serde_json::to_string(&json!({"groups":whole.groups.len(),
        "strokes":full,"prefixes":prefixes,"certificate_prefixes":certificates})).unwrap());
}
