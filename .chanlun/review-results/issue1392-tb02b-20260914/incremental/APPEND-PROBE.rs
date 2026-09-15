use newchan_rust::theta_v0::{classifier::streaming::OwnedIncrementalClassifier,config::ThetaConfig,types::Bar};
use serde_json::{Value,json};
use std::{fs,time::Instant};
fn main(){
 let a:Vec<String>=std::env::args().collect();let count:usize=a[2].parse().unwrap();
 let values:Vec<Value>=serde_json::from_slice(&fs::read(&a[1]).unwrap()).unwrap();
 let bars:Vec<Bar>=values.iter().take(count).map(|v|Bar{source_index:v["source_index"].as_u64().unwrap() as usize,timestamp:v["timestamp"].as_i64().unwrap(),open:v["open"].as_i64().unwrap(),high:v["high"].as_i64().unwrap(),low:v["low"].as_i64().unwrap(),close:v["close"].as_i64().unwrap(),volume:v["volume"].as_f64().unwrap(),untradable:false}).collect();
 assert_eq!(bars.len(),count);let mut cfg=ThetaConfig::default();cfg.tick.tick_size=1.0;cfg.level.l_max=6;
 let mut state=OwnedIncrementalClassifier::new(cfg);let started=Instant::now();let mut level_count=0;
 for bar in bars {let out=state.append_bar(bar);level_count=out.classification.levels.len();std::hint::black_box(out);}
 println!("{}",json!({"bars":count,"elapsed_seconds":started.elapsed().as_secs_f64(),"final_levels":level_count,"bar_count":state.bar_count()}));
}
