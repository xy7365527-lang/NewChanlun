// Conditional projection into unchanged kernel; no semantic input certification.
use serde_json::{json,Value};
mod theta {
    pub mod types { pub use newchan_rust::theta_v0::types::{Center,Direction,Tick}; }
    pub mod classifier {
        #[path="/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/rust/src/theta_v0/classifier/center.rs"]
        pub mod center;
    }
}
use theta::classifier::center::{UnitRange,center_from_segments};
use theta::types::Direction;
fn main(){
 let p=std::env::args().nth(1).unwrap();
 let v:Value=serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap();
 let xs=v["initial_construction"]["objects"].as_array().unwrap();
 assert_eq!(xs.len(),3);
 let us:Vec<UnitRange>=xs.iter().map(|x|UnitRange{start_index:x["start"].as_u64().unwrap() as usize,end_index:x["end"].as_u64().unwrap() as usize,direction:if x["technical_direction"]=="Up"{Direction::Up}else{Direction::Down},lo:x["whole_range"][0].as_i64().unwrap(),hi:x["whole_range"][1].as_i64().unwrap()}).collect();
 let c=center_from_segments(&us[0],&us[1],&us[2]).unwrap();
 assert_eq!((c.zd,c.zg,c.dd,c.gg),(40000,80000,38000,82000));
 assert_eq!((c.start_index,c.end_index),(1,541));
 println!("{}",json!({"mode":"conditional kernel projection only","function":"center_from_segments","uses":"three complete-candidate whole ranges and technical Up/Down/Up","center":{"zd":c.zd,"zg":c.zg,"dd":c.dd,"gg":c.gg,"start_index":c.start_index,"end_index":c.end_index},"kernel_returns_some":true,"semantic_F2_input_certified":false,"actual_admitted_F2_inputs":0,"parent_completed":false}));
}
