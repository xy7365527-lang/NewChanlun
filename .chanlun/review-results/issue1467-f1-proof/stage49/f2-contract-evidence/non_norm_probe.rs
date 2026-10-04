//! 独立小驱动，直接链接本 HEAD 的原始 Rust 文件；不复制中枢公式。
#![allow(dead_code)]
mod theta_v0 {
    #[path = "/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/rust/src/theta_v0/types.rs"]
    pub mod types;
    pub mod classifier {
        #[path = "/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/rust/src/theta_v0/classifier/center.rs"]
        pub mod center;
    }
}
use theta_v0::classifier::center::{center_from_segments, center_from_window, UnitRange};
use theta_v0::types::Direction;

fn unit(path: &[i64], start: usize, direction: Direction) -> UnitRange {
    UnitRange {
        start_index: start,
        end_index: start + path.len() - 1,
        direction,
        lo: *path.iter().min().unwrap(),
        hi: *path.iter().max().unwrap(),
    }
}

fn endpoints(path: &[i64], start: usize, direction: Direction) -> UnitRange {
    let mut out = unit(path, start, direction);
    out.lo = path[0].min(*path.last().unwrap());
    out.hi = path[0].max(*path.last().unwrap());
    out
}

fn main() {
    let a = [10, 10, 10, 10, 30, 20, 20, 20];
    let b = [20, 25, 5];
    let c = [5, 28, 10];
    assert_eq!(a.last(), b.first());
    assert_eq!(b.last(), c.first());
    let full = [unit(&a, 0, Direction::Up), unit(&b, 7, Direction::Down), unit(&c, 9, Direction::Up)];
    let ends = [endpoints(&a, 0, Direction::Up), endpoints(&b, 7, Direction::Down), endpoints(&c, 9, Direction::Up)];
    assert_eq!((full[0].lo, full[0].hi), (10, 30));
    assert_eq!((full[1].lo, full[1].hi), (5, 25));
    assert_eq!((full[2].lo, full[2].hi), (5, 28));
    for i in 0..3 { assert_ne!((full[i].lo, full[i].hi), (ends[i].lo, ends[i].hi)); }
    let actual = center_from_segments(&full[0], &full[1], &full[2]).unwrap();
    assert_eq!((actual.zd, actual.zg, actual.dd, actual.gg), (10, 25, 5, 30));
    assert_eq!((actual.start_index, actual.end_index), (0, 11));
    assert_eq!(center_from_window(&full[0], &full[1], &full[2]), Some(actual));
    assert!(center_from_segments(&ends[0], &ends[1], &ends[2]).is_none());
    assert!(center_from_window(&ends[0], &ends[1], &ends[2]).is_none());
    let mut wrong_dir = full;
    wrong_dir[1].direction = Direction::Up;
    assert!(center_from_segments(&wrong_dir[0], &wrong_dir[1], &wrong_dir[2]).is_none());
    assert_eq!(center_from_window(&wrong_dir[0], &wrong_dir[1], &wrong_dir[2]), Some(actual));
    let mut disconnected = full;
    disconnected[1].start_index = 100;
    assert_eq!(center_from_segments(&disconnected[0], &disconnected[1], &disconnected[2]), Some(actual));
    println!("full_ranges=[(10,30),(5,25),(5,28)]; all_non_norm=true");
    println!("actual_boundaries_connect=true; directions=Up/Down/Up");
    println!("original_center_from_segments={actual:?}");
    println!("original_center_from_window=Some(same center)");
    println!("endpoint_ranges=[(10,20),(5,20),(5,10)]; strict_core=[10,10]; rejected=true");
    println!("wrong_direction: segments=None; window=Some(same center)");
    println!("broken_index_connection: segments=Some(same center), caller obligation remains");
    println!("semantic_completion=NOT_PROVIDED; level=NOT_PROVIDED; full_F2=NOT_CLAIMED");
}
