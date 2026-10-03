// #1467：把DCWitness.lean中独立证明的三点见证与研究Rust实现对照。
// 单个见证不能替代DC-REFINE-v0的全域精化证明。
mod directional_change;
use directional_change::{reference, Active, Constructor, Direction, Point, Unit};

fn main() {
    let expected = vec![Unit {
        direction: Direction::Up,
        start: Point { index: 0, value: 0 },
        end: Point { index: 1, value: 3 },
        known_at: 2,
    }];
    let mut c = Constructor::new(2);
    for p in [0, 3, 0] {
        c.push(p);
    }
    assert_eq!(c.units, expected);
    assert_eq!(reference(&[0, 3, 0], 2), expected);
    assert_eq!(c.initialized_at, Some(1));
    assert_eq!(
        c.active,
        Some(Active::Down {
            start: Point { index: 1, value: 3 },
            trough: Point { index: 2, value: 0 },
        })
    );
    println!("lean_witness=[0,3,0] delta=2 initialization=1 unit=Up(0,1) known_at=2 active=Down(1,2) rust_incremental_and_reference=pass");
    println!("scope=one_formally_certified_witness_not_full_rust_refinement");
}
