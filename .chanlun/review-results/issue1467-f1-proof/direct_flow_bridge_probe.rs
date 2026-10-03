// #1467 R_D-v2：同向供需片段+触价区间，不把供需方向等同几何方向。
use newchan_rust::theta_v0::classifier::center::{center_from_segments, UnitRange};
use newchan_rust::theta_v0::types::Direction;

#[derive(Clone, Copy, Debug)]
enum Op {
    CancelAsk,
    AddBid,
    CancelBid,
    AddAsk,
}
impl Op {
    fn upward_flow(self) -> bool {
        matches!(self, Self::CancelAsk | Self::AddBid)
    }
    fn price(self) -> i64 {
        if matches!(self, Self::AddBid | Self::CancelBid) {
            100
        } else {
            102
        }
    }
}
#[derive(Debug)]
struct Block {
    upward: bool,
    events: Vec<(usize, Op)>,
    known_at: usize,
}

fn main() {
    let events: Vec<_> = (0..3)
        .flat_map(|_| [Op::CancelAsk, Op::AddBid, Op::CancelBid, Op::AddAsk])
        .collect();
    let (mut bid_qty, mut ask_qty) = (100i64, 100i64);
    let mut completed = Vec::new();
    let mut active = Vec::new();
    let mut sign = None;
    for (i, &e) in events.iter().enumerate() {
        match e {
            Op::CancelAsk => ask_qty -= 10,
            Op::AddAsk => ask_qty += 10,
            Op::AddBid => bid_qty += 10,
            Op::CancelBid => bid_qty -= 10,
        }
        assert!(bid_qty > 0 && ask_qty > 0);
        let next = e.upward_flow();
        if let Some(old) = sign {
            if old != next {
                completed.push(Block {
                    upward: old,
                    events: std::mem::take(&mut active),
                    known_at: i,
                });
            }
        }
        sign = Some(next);
        active.push((i, e));
    }
    assert_eq!((bid_qty, ask_qty), (100, 100));
    let mut valid = 0;
    let mut units = Vec::new();
    for b in &completed {
        let lo = b.events.iter().map(|(_, e)| e.price()).min().unwrap();
        let hi = b.events.iter().map(|(_, e)| e.price()).max().unwrap();
        let (start, end) = if b.upward { (lo, hi) } else { (hi, lo) };
        let witness = b.events.iter().find_map(|(i, e)| {
            if e.price() == start {
                b.events
                    .iter()
                    .find(|(j, f)| i < j && f.price() == end)
                    .map(|(j, _)| (*i, *j))
            } else {
                None
            }
        });
        valid += usize::from(witness.is_some());
        units.push(UnitRange {
            start_index: b.events.first().unwrap().0,
            end_index: b.events.last().unwrap().0,
            direction: if b.upward {
                Direction::Up
            } else {
                Direction::Down
            },
            lo,
            hi,
        });
        println!("block={b:?} touched_prices={:?} support=[{lo},{hi}] price_direction_witness={witness:?}",b.events.iter().map(|(_,e)|e.price()).collect::<Vec<_>>());
    }
    let accepted = center_from_segments(&units[0], &units[1], &units[2]);
    assert_eq!(completed.len(), 5);
    assert_eq!(valid, 0);
    assert!(accepted.is_some());
    println!("completed_flow_blocks={} geometric_endpoint_witnesses={valid} center_kernel_accepts_first_three={} first_center={accepted:?}",completed.len(),accepted.is_some());
    println!("scope=reject_copying_flow_sign_as_geometric_direction_not_all_direct_order_bases");
}
